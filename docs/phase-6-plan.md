# Phase 6 Plan — 1.0 Release

> Status: **approved** by the user (2026-11-26). Derived from
> `nova-spec/00-MASTER-SPEC.md` §3 (Phase 6, `:935-942`), §1.9
> (`:99-102`), §5.3 (`:996`) and §7 (`:1126-1134`),
> `nova-spec/20-STDLIB.md` §1, `docs/releasing.md`, and ADRs 0047, 0048
> and 0061, with the research of 2026-11-26 on `main` at `4fe11d55`
> (Phase 5 closed by ADR 0061; `v0.5.0` released). The user's decisions
> of 2026-11-26 are in §1. Each sub-phase below gets its own spec, plan
> and review before it is built.

## 1. Goal (from the spec)

Phase 6 = "1.0 Release", weeks 105 to 120: 16 weeks, the last of the
master spec's 30 months (`nova-spec/README.md:93`). Unlike Phases 0 to
5, it has no goal line and no gate (`nova-spec/00-MASTER-SPEC.md:935-943`);
this plan proposes the gate below. The master spec lists seven items
(`:936-942`):

1. Security audit (external)
2. Stability freeze — RFC for any breaking change
3. Documentation 100% (every public API has rustdoc-equivalent)
4. Tutorial site at `novalang.dev`
5. Interactive playground (compile in browser via WASM-compiled Nova compiler)
6. Submit to TechEmpower benchmarks
7. Launch posts: HN, Lobsters, Reddit r/programming, dev.to, Twitter

Around them: §1.9's "Pre-1.0: breaking changes allowed in minor
versions, RFC required", "Post-1.0: semver strict, edition system
(`edition = "2026"`) for opt-in breaking" and "Deprecation: minimum 2
minor versions before removal" (`:100-102`); §5.3's "Link to docs" in
every user-facing error (`:996`); and §7's "Tag a milestone release:
`v0.{phase}.0`" (`:1134`), which for Phase 6 reads `v0.6.0`.

Names, as in Phase 5's plan: **S0** is the Rust `nova`; **N** is the
Nova compiler in `selfhost/`.

**The gate (proposed; the master spec gives none):** a part for each
item in order (items 6 and 7 together), then the release; on `main`
after 6.10, with reproducible commands, in CI wherever it can run:
1. **The audit, as in-repo hardening:** `SECURITY.md`, a threat model
   and the `unsafe` and dependency inventories merged, the inventories
   held by tests; fuzz targets for the lexer, the parser, JSON, regex
   and the HTTP request parser pass in CI on fixed seeds; a dependency
   check (advisories, licences, sources) passes in CI; the external
   audit recorded as done, with its findings' disposition, or not done.
2. **The freeze:** the stability, RFC and MSRV policies merged; std's
   public signatures and each command's `--help` pinned by snapshot
   tests (recommended, §3 decision 4); `@deprecated` warning at a use
   site and shown on std's site; the two S0 bugs ADR 0061 keeps fixed in
   S0 and N with cases, and issue #3 closed by a test; `std/regex`, and
   the HTTP client with handlers that await, documented and tested on
   their targets, the client against local servers only.
3. **Documentation 100%:** std's documentation tests cover every public
   module, `std/dom` included; `missing_docs` denied on every library
   crate; every code a diagnostic can carry has an error-index entry,
   and each rendered diagnostic its link, both held by tests; the
   reference's and the book's code blocks run as doc tests.
4. **The site:** one command builds it (the book, the reference, std's
   documentation with `std/dom`, the error index, the playground) in CI
   with no broken internal link; a workflow deploys it to GitHub Pages
   from the release's tag.
5. **The playground:** CI builds the compiler for
   `wasm32-unknown-unknown`; Node and Chromium tests compile and run
   hello world and a fixed list of programs through it against their
   expected output, and show a failing program's diagnostics; its size
   and hello world's compile time recorded, within 6.0's limits or as
   ruled.
6. **TechEmpower and the launch:** an app in the repository answers the
   JSON and plaintext test types as the rules require, held by a CI
   test, with how to submit; the announcement and draft posts in the
   repository; the TechEmpower pull request and the posts recorded as
   the user's, done or not.
7. **The release:** `1.0.0` published by `docs/releasing.md`, each
   [stop] on the user's word, the Release gate green on three systems;
   every item of §7's Definition of Done met, as the boundary ADR
   records.

**The user's decisions, 2026-11-26:**
- **P1, scope: all seven items.** The in-repo work is gated. The
  external items (an external audit firm, the TechEmpower pull request,
  the launch posts, the domain and DNS, switching GitHub Pages on, any
  account or money) are the user's, recorded, and do not block 1.0.
- **P2, the playground: option A for 1.0, after a spike.** S0's front
  end and WASM backend built for `wasm32-unknown-unknown`, running in
  the browser (in-memory sources through the `Sources` trait,
  `build_program_wasm`, the JS layer's `init({ wasm })` with a print
  hook). The spike needs the user's word to add the `wasm32` rustup
  target, locally and in CI. An ADR records that the playground is S0
  in the browser, not N built for WASM, as ADRs 0048 and 0061 had it;
  option B, N built for WASM, stays on the backlog.
- **P3, the version:** `1.0.0` is the phase's release, an ADR recording
  it in place of §7's `v0.{phase}.0`; strict semver after it (§1.9);
  release candidates only if needed.
- **P4, the site:** no `novalang.dev` yet. GitHub Pages
  (`sakeerin.github.io/nova`) hosts the tutorial, the documentation and
  the playground; the user switches Pages on; a custom domain later, if
  bought.
- **P5, documentation 100%: all of it.** std, `std/dom` included, on the
  site; a `missing_docs` lint for the Rust crates; a language reference;
  a book or tutorial; an error-code index, errors linking to it (§5.3);
  the measurable parts held by tests.
- **P6, stability:** the promise covers the language, std, the CLI,
  `nova.toml`, `nova.lock` and the index format; not the hidden flags,
  the Rust crates (internal), N, or diagnostic wording. A written
  policy, an RFC process and `SECURITY.md`.
- **P7, the audit:** in-repo hardening is the gate (a threat model, the
  `unsafe` and dependency inventories, fuzz targets, `cargo-deny` or
  `cargo-audit` or an equivalent that needs no new crate in the
  workspace, `SECURITY.md`); the external audit is the user's, recorded
  as not yet done unless booked.
- **P8, TechEmpower:** the test types that need no database (JSON,
  plaintext), as an app in the repository with how to submit; a
  database driver on the backlog; the pull request is the user's.
- **P9, before the freeze:** what would break later (an `@deprecated`
  attribute; fixes to the two S0 bugs ADR 0061 keeps, std's bodies
  seeing a program's impls and float `%` failing `build --release`;
  issue #3, E0902 on `sqrt`), and `std/regex` and the HTTP client, with
  handlers that can await. UDP and Unix sockets, AEAD, `Queue`,
  `RwLock` and atomics stay post-1.0.
- **P10:** none of ADR 0061's backlog is in Phase 6 (the bootstrap on
  macOS and Windows, N on a large-stack thread and E0093, compiler speed
  among it), but for the two kept bugs P9 names.
- **P11, MSRV:** keep 1.78, with a written policy; a raise only on the
  user's word (an advisory may force one).
- **P12, the order:** the playground spike first; the rest ordered by
  this plan, for the user's approval.

**Not asked; recommended here:** the launch materials in the repository
(an announcement, the `[1.0.0]` changelog entry, draft posts, and a
README for users in place of the "Specification Bundle" opening,
`README.md:1-4`), the posts themselves the user's; the site built by
`nova-doc`, extended to render a book (§3 decision 8); and the gate
above, the research's question 12 made testable item by item.

## 2. Reality check — what 1.0 needs that the repo lacks

Measured on `main` at `4fe11d55`, 2026-11-26, with `nova` 0.5.0.
"Heuristic" counts come from searches, not tests.

1. **The audit (item 1): nothing an auditor is handed exists.**
   - No `SECURITY.md`, `deny.toml`, `cargo audit` or `cargo deny` step,
     or `fuzz/`; `CODE_OF_CONDUCT.md:7` says "Report issues to the
     maintainers", with no contact.
   - `Cargo.lock` holds 254 packages, 231 from crates.io. Two pins are
     held back by MSRV 1.78: `wasm-encoder` and `wasmparser` at
     `=0.243.0` (`Cargo.toml:69-73`), `ureq` at `=3.2.1`
     (`crates/nova-index/Cargo.toml:33-37`). Advisories were not checked
     (no network).
   - `unsafe` occurs 715 times in `crates/nova-runtime/src` (tests
     included), beside 119 `#[no_mangle]` exports and `gc_stack.c`;
     outside it, `crates/nova-cli/src/cmd/run.rs:146` and the JIT
     entry's `transmute` (`crates/nova-codegen-cranelift/src/lib.rs:62`).
     No crate forbids `unsafe_code`. Fuzzing is the self-hosting
     differential (`crates/nova-cli/tests/selfhost/fuzz.rs`) and
     `proptest` in the lexer's and parser's tests.
   - Defended already: a package checked against the lock's SHA-256,
     bounded and confined (ADR 0031 `:48-53`); install scripts checking
     `SHA256SUMS` from the same release
     (`tools/install/install.sh:78-94`): integrity, not authenticity.
     `ci.yml` and `benchmarks.yml` set no `permissions`; no branch on
     GitHub is protected, `main` included.
   - An audit's surfaces: `std/http`'s parsing, `std/json`'s depth,
     `std/crypto` on `ring`, the blog's sessions (ADR 0046), the
     collector and executor, `nova dev`, the WASM shim (ADR 0041), the
     index and downloads, and this phase's playground and client.
     External: the firm and its cost; GitHub's settings.
2. **The freeze (item 2): no policy, no process, nothing pins std's
   API.**
   - `docs/rfcs/` holds 0000 (informational) and 0001 (the component
     syntax), with no README, template or states; RFCs "propose new
     features that the spec doesn't yet cover"
     (`nova-spec/README.md:117-119`). `CONTRIBUTING.md` is 32 lines,
     with no governance.
   - The golden suite (176 programs), `tests/wasm/`, `tests/dom/`, 16
     insta snapshots and the CLI's tests pin behaviour; std's
     documentation test checks documentation, not signatures
     (`crates/nova-doc/tests/std_docs.rs:8-27`).
   - Versioned: `nova.lock`'s `version = 1`
     (`nova-spec/40-TOOLING.md:424-429`), the index line's `v` (ADR 0031
     `:20-23`); one edition, `"2026"`
     (`crates/nova-pm/src/manifest.rs:13-14`), edition rules unbuilt
     (`40-TOOLING.md:436-439`). Outside any promise already: the 20
     published crates (`Cargo.toml:20-26`) and the hidden `--dump-*`
     flags (`CHANGELOG.md:219`). No deprecation: `test` is the only
     attribute, any other E0082 (`crates/nova-resolver/src/lib.rs:3336`).
   - What a freeze would lock in: std's bodies see a program's impls,
     worked around by a fallback (ADR 0056 `:1220-1221`); float `%`
     type-checks and fails in `build --release` (ADR 0058 `:885-886`);
     issue #3, `nova run` failing to resolve `sqrt` through the JIT on
     Linux (E0902), open since 2026-08-12.
3. **Documentation (item 3): std is at 100% by test; the rest does not
   exist.**
   - `std_is_documented` and `every_std_module_has_an_example` cover
     the 21 modules of `nova_std::ALL` (`std_docs.rs:8-47`); heuristic:
     621 public items in all 25 std sources, none without `///`. `ALL`
     leaves out `RT`, `DOM` and `std/ui`'s backend (`std/lib.rs:76-78`);
     `std/dom`'s 67 items are off the site by ADR 0047's decision
     (`:170-185`).
   - Rust: no `missing_docs` lint; heuristic: 113 of 832 item-level
     `pub` declarations lack `///` (44 in `nova-cli`, a binary), fields
     and variants not counted.
   - Missing: a language reference (§2's `docs/spec/`, `:182-185`), a
     book (`docs/book/`, `:186-188`), an error-code index, CLI and
     `nova.toml` references. §5.3's link is built and recorded nowhere.
     About 75 codes are in use (55 E, 17 M, 2 P, 1 W; heuristic).
   - The README opens as "Nova Language — Specification Bundle".
4. **The site (item 4): nothing exists, and `novalang.dev` "does not
   exist"** (`40-TOOLING.md:450`, `:626`; `50-TESTING.md:243`). No
   site sources, Pages workflow or `gh-pages` branch; a Pages deploy
   needs `pages: write` and `id-token: write`, which no workflow grants.
   At hand: `nova-doc` renders Markdown with `pulldown-cmark`, with
   search and themes, and runs code blocks as doc tests (ADRs 0034,
   0035); `release.yml`'s `std-docs` job builds std's site
   (`.github/workflows/release.yml:264-291`). mdBook, which §2 names, is
   not in the workspace. External: the Pages switch; a domain and DNS.
5. **The playground (item 5): option A's path exists in pieces.**
   - S0's WASM backend needs no other toolchain (ADR 0039 `:38-41`);
     its runtime is `std/rt`, in Nova; one memory of at most 1 GB
     (`:70`); V8's stack, about 1 MB, bounds a program's recursion
     (`:105-107`). WASM's std is 16 modules, without `io`, `fs`, `net`,
     `http`, `crypto`, `process`, `test` and `ui.ssr`
     (`crates/nova-resolver/src/lib.rs:2892-2909`).
   - In memory: `build_program_wasm` lowers, compiles and makes the shim
     before its writes (`crates/nova-driver/src/lib.rs:865-892`); the
     front end reads through `Sources`
     (`crates/nova-driver/src/analyze.rs:25-42`); the shim's `init`
     takes `options.wasm` (`crates/nova-codegen-wasm/src/shim/run.js:2-10`)
     and prints to `console.log` (`shim/core.js:34-45`).
   - The probe, without a build: the front-end crates and
     `nova-codegen-wasm` use no OS API outside tests but one thread with
     a 256 MB stack (`crates/nova-codegen-wasm/src/lib.rs:63`,
     `:198-208`). `nova-driver` runs `clang` and depends, through
     Cranelift, on `nova-runtime` (`ring`, `libc`, C), so it cannot be
     the browser's entry. The chain is 55,749 Rust lines with tests, and
     std's sources 742,977 bytes; whether it builds for `wasm32`, its
     size and its speed are unknown until built.
   - Option B: N's MIR is native only (ADR 0057 `:521-534`), and its
     `async fn main` would never collect mid-compile (ADR 0040 `:42-43`).
6. **TechEmpower (item 6): a server, no database driver, no client.**
   `std/http` serves HTTP/1.1 with a router, pipelined requests in
   order; a handler "cannot await anything" (`std/http/lib.nova:8-9`);
   "Nova gets concurrency, not multi-core execution" (ADR 0009 `:57`).
   Measured once: 11,940.0 req/s on the read-and-parse path
   (`docs/benchmarks/http-fixed-response.md`). The rules were not read
   (no network). External: the pull request and its review.
7. **The launch (item 7): the user's.** The repository can give
   `[1.0.0]`'s notes, which `release.yml` takes from `CHANGELOG.md`
   (`docs/releasing.md:148-154`), an announcement, and drafts.
8. **The release: the runbook needs no change for `1.0.0`** (from
   `v0.3.0` on, `docs/releasing.md:3`; a prerelease only "if the version
   has a `-`", `:226-231`). CI's MSRV job checks 1.78 (`ci.yml:399-476`);
   no policy says when it may rise. The next ADR is 0062.
9. **std's v1.0 index** (`20-STDLIB.md:8-29`): not built, `Queue`,
   atomics and `RwLock`, UDP and Unix sockets, the HTTP client, AEAD,
   and `std/regex`, "PCRE-compatible" (`:25`). On WASM, `std/dom` has a
   `fetch` (`README.md:111-114`).
10. **Two compilers.** CI holds N equal to S0 on every pull request: the
    dump differential on three systems, S1 == S2 == S3 on Ubuntu (ADR
    0060). A change to a ported part changes both in one pull request
    (ADR 0056 `:1205-1211`). The Selfhost job's lines are 80, 100 and
    110 s (ADR 0061 `:205`).
11. **Tools here:** Node 22; Rust 1.97 with the 1.78 and 1.100
    toolchains; no `wasm32-unknown-unknown` target. CI's browser job
    installs Chromium, Lighthouse and binaryen (`ci.yml:281-356`).

## 3. Key decisions (recommended now, confirmed before building)

Each is confirmed in its sub-phase's spec, and recorded in an ADR where
it departs from the master spec or an earlier ADR.

1. **Scope, and what is external (P1, P9, P10).** ADR 0062 records the
   seven items, the in-repo part of each gated and the user's parts
   named, and what Phase 6 leaves out, to the backlog:
   - N built for WASM (option B), amending ADR 0048 (`:44`) and ADR
     0061's mapping (`:394-395`); a database driver; UDP and Unix
     sockets, AEAD, `Queue`, `RwLock` and atomics; ADR 0061's backlog,
     but for its two kept bugs;
   - what other specs give Phase 6 that the seven items do not name:
     multiple toolchains (`40-TOOLING.md:646`; ADR 0037 `:93`),
     `nova-embed` (`13-RUNTIME.md:480`), the `setup-nova` action
     (`40-TOOLING.md:583-598`), examples 08 to 13
     (`60-EXAMPLES.md:801-812`), and edition rules.

   A sub-phase's spec may pull one back in, with a ruling that names
   ADR 0062. **Pre-1.0 RFCs:** §1.9 asks one for a breaking change
   before 1.0 (`:100`); as in Phases 1 to 5, a sub-phase's approved
   spec and its ADR stand as that RFC for 6.3 to 6.5, and the RFC
   process (6.2) governs every change after `1.0.0`.
2. **The playground: S0 in the browser, after a spike (P2).** 6.0's
   spike builds the front-end chain and `nova-codegen-wasm` for
   `wasm32-unknown-unknown` with a throwaway entry, in scratch space, as
   Phase 5's lexer spike was, and measures the compiler's size, hello
   world's compile time, memory, and the deepest input that survives.
   ADR 0062 records the approach with those figures; it amends ADRs
   0048 and 0061, not the master spec, whose "WASM-compiled Nova
   compiler" (`:940`) names no stage. The page compiles one source text
   with std embedded, returns diagnostics or `app.wasm` and `app.js`,
   and runs the program in a sandbox away from the site's origin (§6
   risk 1), through hand-written JS glue (no `wasm-bindgen`). **If the
   spike misses its limits,** the user rules before 6.1: make it smaller
   (std's checking kept across compiles, `wasm-opt`), compile on a
   server (an external service, P1), or leave the playground for after
   1.0; an ADR records the choice.
3. **The version: `1.0.0` (P3).** ADR 0062 records it in place of §7's
   `v0.{phase}.0` (`:1134`). The runbook runs unchanged with
   `version=1.0.0`. A `1.0.0-rc.N` comes only if the audit, the freeze
   or the Release gate asks for one, on the user's word, since each
   release repeats the user's publishing steps. Strict semver after.
4. **The stability promise (P6), written in 6.2, in force from
   `1.0.0`.** **Covered:** the language at edition `"2026"`; std's
   public API on both targets; the CLI's commands, documented flags and
   exit codes; `nova.toml`; `nova.lock` (`version = 1`); the index line
   (`v`). **Not covered:** hidden flags; the Rust crates, published so
   that `cargo install` works, their API internal as their README says;
   N; diagnostics' wording; the generated JS and WASM's internals; the
   playground; performance. **Rules:** semver; a breaking language
   change only in a new edition; deprecation at least two minor versions
   before a removal, through `@deprecated`; soundness and security fixes
   as recorded exceptions. **The RFC process:** `docs/rfcs/README.md`
   and a template (0000 is the overview's): when an RFC is needed, its
   states, who decides (the maintainer). **Held by tests (recommended,
   not asked):** snapshots of std's public signatures, from `nova-doc`'s
   item tables, and of each command's `--help`, which a pull request
   changing either updates in review.
5. **Before the freeze (P9).** What would break, or come harder, after
   `1.0.0` lands before hardening: `@deprecated` and the three fixes
   (6.3), `std/regex` (6.4), the client and handlers that await (6.5).
   The handler change breaks today's `fn(Request) -> Response`
   handlers; `ui::ssr`, the examples and the blog move with it.
6. **Documentation 100% (P5), each part held:** std with `std/dom`
   (amending ADR 0047, `:180-185`; `rt` and the ui backends internal),
   by the extended documentation tests; Rust, by `missing_docs`, denied
   through clippy's `-D warnings`; the error index, by tests that every
   emitted code has an entry whose example gives it, and that each
   rendered diagnostic and the language server's `codeDescription` link
   to it; the references, by doc tests and tests that every command,
   documented flag and manifest key appears; the book, by doc tests.
7. **Hardening, the audit's in-repo gate (P7),** as 6.6, with
   `SECURITY.md` from 6.2 and fuzz targets for §5.2's lexer, parser,
   JSON and regex (`:985`) and the HTTP request parser. If the user
   books a firm in time, `1.0.0` may wait for its findings, on the
   user's word.
8. **The site: GitHub Pages, built by `nova-doc` (P4; recommended).**
   A project site under `/nova/`, so every link is relative to that
   base; the user turns Pages on (source: GitHub Actions). `nova-doc`,
   extended to render a book, over mdBook, which §2 names: no new tool
   in CI, one look and one search across std's documentation and the
   book, and its code blocks tested by `nova`'s own doc tests. The book
   keeps mdBook's layout (`docs/book/src/SUMMARY.md`), so mdBook could
   render it later; the site's command is hidden, outside the promise.
   It deploys from the release's tag, the playground built from the
   same commit.
9. **TechEmpower: JSON and plaintext (P8),** as 6.9; the rules read in
   its spec, by the user or with the user's leave.
10. **MSRV: 1.78, with a policy (P11),** written in 6.2: a raise is the
    user's decision, in a minor release, with an ADR; an advisory in a
    pin held back for 1.78 may force one, and the dependency check finds
    it.
11. **Two compilers in step.** A change to a ported part (6.3) changes
    S0 and N in one pull request (ADR 0056 `:1205-1211`); std grown in
    Nova is type-checked by both; the playground, being S0, needs no
    port.
12. **New crates, rustup targets and CI tools.** None is added before
    the user's word, after its spec shows the change:

    | Sub-phase | What may be asked | For |
    |---|---|---|
    | 6.0 | the rustup target `wasm32-unknown-unknown`, locally (stable and 1.78) | the spike, built offline from the crates `Cargo.lock` already holds |
    | 6.1 | the same target in CI; a new unpublished workspace crate for the browser's entry, or a feature split of `nova-driver` (either changes `Cargo.lock`) | the playground |
    | 6.4 | nothing, if `std/regex` is in Nova; `regex` 1.13.1 (in `Cargo.lock` through `criterion`) as a test-only oracle, or as the engine if the spec chooses the runtime | the engine and its check |
    | 6.5 | `rustls` 0.23.45 and `webpki-roots` 1.0.9 for `nova-runtime` (in `Cargo.lock` through `ureq`; a lock change); test certificates checked in | HTTPS |
    | 6.6 | `cargo-deny` or `cargo-audit`, installed by its CI job at a pinned version, with the advisory database that job fetches; optionally `cargo-fuzz`, `libfuzzer-sys` and a nightly toolchain, outside the workspace; optionally attestations in `release.yml` | the checks |
    | 6.8 | `actions/configure-pages`, `actions/upload-pages-artifact`, `actions/deploy-pages`, in a job with `pages: write` and `id-token: write` | the deploy |
    | 6.9 | Docker, for the user's local check only | TechEmpower |

    A JS library for the playground's editor, or any npm package, is
    asked the same way; the default is none.

## 4. Sub-phases (each gated, reviewed, and merged on the user's word)

Each sub-phase runs the established loop: research where needed, the
user's decisions, a spec, a prototype, a draft pull request measuring
CI on the user's word and closed unmerged, a plan, the implementation,
one fresh review and one fix pass, a pull request, and a merge on the
user's word.

**The order.** Each sub-phase needs the one before it, but where this
paragraph says otherwise. The spike comes first (P12), after the user
approves this plan, and the playground follows while its figures are
fresh. The policies (6.2) move ahead of the changes that affect the
freeze, so that the promise decides what must change first and
`@deprecated` (6.3) builds the rule they write; 6.1 and 6.2 may run
side by side. `std/regex` (6.4) and the client (6.5) precede hardening
(6.6), whose fuzz targets and threat model cover them. Documentation
(6.7) follows the last change to the language, std and the codes, so
the index and the reference are written once; the site (6.8) needs it
and the playground. TechEmpower (6.9) needs only 6.5 and may run beside
6.7 or 6.8. The release (6.10) needs 6.8 and 6.9; the records (6.11)
follow its publishes, as 5.6's did.

**Sizes, in Phase 5's terms.** Phase 5's sub-phase pull requests each
added about 1,100 (5.5b) to 4,100 lines (5.0b) outside `docs/`,
`nova-spec/`, the CHANGELOG and the test directories, beside a spec and
a plan; 5.6's release and records added almost none.

### 6.0 — The scope ADR and the playground spike
- **Goal:** know whether option A is viable, and record the scope.
- **Scope:** the spike, in scratch space, offline; only its figures are
  merged. The chain and `nova-codegen-wasm` (the thread cut by a `cfg`)
  built for `wasm32-unknown-unknown` on stable and 1.78, with a
  throwaway entry; measured: the size raw, gzipped and after
  `wasm-opt`; hello world's and two examples' compile times under Node,
  and Chromium where it can run; peak memory; the deepest of 5.1b's
  depth cases that survives V8's stack; hello world's output.
- **Deliverables:** ADR 0062 (§3 decisions 1 to 3, with the spike's
  figures); this plan, approved; a dated note in the master spec.
- **Gate:** the figures are within the spec's limits, or the user has
  ruled; ADR 0062 merged.
- **The user's part:** the word for the local target; the limits.
- **Size:** records only, as 5.6c; the spike's code stays in scratch.
- **Questions for its spec:** the limits (suggested: under 5 MB
  gzipped, hello world in under 2 s in Chromium on CI, the depth limit
  of 128 survived); keeping std's checked items across compiles;
  Chromium locally or in a draft pull request.

**Amended 2026-11-27 (6.0, branch `claude/loving-archimedes-frtf5q-60`,
ADR 0062; spec
`docs/superpowers/specs/2026-11-26-phase-6-0-the-plan-and-adr-0062-design.md`):**
the spike ran on 2026-11-26, its code not committed. Option A is within
its spec's limits, measured in Node and a local Chromium: 519,967 to
830,149 bytes gzipped; hello world compiled in 197 ms the first time,
about 34 ms after; every depth case at 128 survived, in a worker too. It
built on stable alone; the build on 1.78, the size after `wasm-opt` and
Chromium in CI are 6.1's. A long function body overflows codegen's
recursion in the browser: 6.1 catches it and says so, by the user's
decision of 2026-11-26, and an iterative structurer is on the backlog.
This plan's `path:line`s stay `4fe11d55`'s.

### 6.1 — The playground
- **Goal:** compile and run Nova in the browser, with S0's own parts.
- **Scope:** the browser's entry, apart from `nova-driver`'s native
  backends, over in-memory `Sources` (one file, std embedded); the
  codegen thread behind `cfg(not(target_arch = "wasm32"))`, the stack
  set at link time; a print hook in the shim; a page (an editor, Run,
  the output, examples, diagnostics), the compiler in a worker, the
  program in a sandboxed iframe with a time limit and Stop; CI builds
  the compiler, shrinks it with `wasm-opt` and prints its size.
- **Deliverables:** the code; §1's gate item 5's Node and Chromium
  tests, over a fixed list of programs that fit WASM's std; an ADR; the
  page as a build output, deployed by 6.8.
- **Gate:** both tests pass in CI; size and compile time recorded,
  within 6.0's limits or as ruled; the WASM suite and the Selfhost job
  unchanged.
- **The user's part:** the word for the CI target and the entry.
- **Size:** about 800 to 1,500 lines of Rust, JS and HTML with CI; one
  pull request, two if the entry's split goes first; about 5.5a's.
- **Questions for its spec:** a crate or a feature; the fixed list;
  `std/dom` programs in the iframe, or console ones only; share links
  (recommended: none that run on open); the editor (a textarea, or a
  library, asked); the MSRV job and `wasm32`; a release asset or not.

### 6.2 — The stability policy, the RFC process, the MSRV policy and `SECURITY.md`
- **Goal:** what `1.0.0` promises, and how it changes, written first.
- **Deliverables:** `docs/stability.md` (name per spec: coverage,
  semver, editions, deprecation, exceptions, the MSRV policy);
  `docs/rfcs/README.md` and a template, the issue template pointing at
  them; `SECURITY.md` (supported versions, the reporting channel, what
  a reporter can expect, disclosure); `CONTRIBUTING.md` (who decides,
  the RFC process, the MSRV, the two-compilers rule); the snapshot
  tests (recommended, §3 decision 4); an ADR.
- **Gate:** the documents merged and linked from the README; the
  snapshot tests pass, and fail on a changed signature or flag.
- **The user's part:** the policy's text; the reporting channel and
  its GitHub setting (private vulnerability reporting).
- **Size:** mostly prose, 600 to 1,000 lines, with 200 to 400 of tests;
  one pull request.
- **Questions for its spec:** whether a code keeps its meaning; the
  support window; what needs an RFC after 1.0 (breaking changes only, or
  new features and std modules too); the snapshot's format; the conduct
  contact `CODE_OF_CONDUCT.md:7` lacks.

### 6.3 — Before the freeze: `@deprecated`, the two kept bugs and issue #3
- **Goal:** a way to deprecate, and fixes to what 1.0 would promise.
- **Scope:** `@deprecated` with a note on public items, warning at each
  use outside the item's module on every backend, shown by `nova doc`
  and tagged by the language server, in S0 and N (the resolver's
  attribute list, the type checker's warning) with differential cases;
  std's bodies no longer seeing a program's impls, in both type
  checkers, the fallback gone; float `%` under `--release`, in both
  `.ll` emitters; the JIT resolving `sqrt` and its kin on Linux.
- **Deliverables:** the code, cases and golden programs; an ADR; issue
  #3 closed by the pull request.
- **Gate:** the cases pass under `run`, `build` and `--release`; the
  differential on three systems and the bootstrap hold; a test runs
  `sqrt` under `nova run` on Linux.
- **The user's part:** rulings where a fix changes a program.
- **Size:** about 1,000 to 2,000 lines across both compilers; one pull
  request; about 5.0a's.
- **Questions for its spec:** the attribute's form and targets; the
  warning's code; whether std deprecates anything now; what the impls
  fix changes; which libm symbols the JIT lacks.

### 6.4 — `std/regex`
- **Goal:** the std module P9 adds beside the client.
- **Scope (recommended):** written in Nova, so it runs on both targets
  and in the playground with no new crate; a linear-time engine (a Pike
  VM), so no pattern makes a match exponential; a documented syntax, a
  subset recorded as a departure from "PCRE-compatible", without
  backreferences or lookaround; imported (`import regex`), not
  glob-imported, so a program that does not use it pays nothing.
- **Deliverables:** the module, documented, with an example; tests on
  both targets; a check against an oracle (§3 decision 12) or left to
  6.6; a section in `20-STDLIB.md`; an ADR.
- **Gate:** the tests pass natively and on WASM; std's documentation
  tests cover it; its matches equal the oracle's on a fixed-seed corpus.
- **The user's part:** the oracle's crate, if chosen.
- **Size:** about 1,500 to 3,000 lines of Nova with tests; one pull
  request; about 5.0c's.
- **Questions for its spec:** Nova, or the runtime over the `regex`
  crate (native only); the syntax; Unicode classes; the API
  (`Regex::new`, `is_match`, `find`, `captures`, `replace`, `split`).

### 6.5 — The HTTP client, and handlers that await
May split into 6.5a (the client over plain HTTP, and the handlers) and
6.5b (HTTPS), as its spec decides.
- **Goal:** the client half of `std/http`, and handlers that await.
- **Scope:** the client, native only (WASM keeps `std/dom`'s `fetch`):
  `get`, `post` and a request with a method, headers and a body;
  responses framed by `Content-Length` or chunked; redirects, timeouts
  and size limits; HTTPS through `rustls` on `ring` with
  `webpki-roots`, on the user's word, over the runtime's poller;
  handlers returning a future that the server awaits, with `ui::ssr`,
  the examples, the blog and their tests moved to them.
- **Deliverables:** the code; tests against servers the tests start,
  plain and with a checked-in test certificate; `20-STDLIB.md` §6 up to
  date; an ADR.
- **Gate:** the tests pass on three systems with no internet; in a
  test, a handler that awaits the client serves a request; the golden
  suite, the WASM suite and the browser job still pass.
- **The user's part:** the word for `rustls` and `webpki-roots`, or
  HTTPS left for after 1.0.
- **Size:** about 1,500 to 3,000 lines of Nova and Rust; one or two
  pull requests; about 5.0b's in all.
- **Questions for its spec:** the API, and a name for the client's JSON
  decoding, `Response::json` being the server's constructor; `async fn`
  handlers only, or both forms; whether a closure returning a future
  serves, there being no `async` closures; how much `rustls` grows each
  native program; bundled roots or the system's.

### 6.6 — Hardening
- **Goal:** what an auditor is handed, and checks that keep it true.
- **Deliverables:** a threat model (`docs/security/`, name per spec):
  §2 item 1's surfaces with the playground and the client, each
  defence and what is left; the `unsafe` inventory, held by a test of
  each file's count, and `#![forbid(unsafe_code)]` where there is none;
  the dependency inventory; fuzz targets in `cargo test` on fixed
  seeds: the lexer and the parser directly, and JSON, regex and the
  HTTP request parser through small Nova programs fed random input, as
  the selfhost fuzzer drives N; the dependency check in CI (licences,
  sources and bans on every pull request; advisories on a schedule, on
  a change to `Cargo.lock` and before a release, so an upstream
  advisory fails no unrelated pull request); `permissions:` in every
  workflow; an ADR.
- **Gate:** the documents merged; the inventory tests and fuzz targets
  pass in CI; the dependency check passes, each exception reasoned.
- **The user's part:** the word for the check's tool and its install,
  and any optional tool (§3 decision 12); private vulnerability
  reporting, Dependabot alerts and branch protection on `main`,
  recommended before the launch; whether a firm is booked.
- **Size:** about 1,000 to 2,000 lines of tests and CI, with the
  documents; one pull request; about 5.1a's.
- **Questions for its spec:** `cargo-deny` or `cargo-audit`;
  `cargo-fuzz` too, or not; the fuzz targets' time in CI; attestations
  for the release's assets; actions pinned by commit.

### 6.7 — Documentation 100%
Two pull requests, in order (§3 decision 6).
- **6.7a, the measurable parts:** `missing_docs` on every library crate,
  its gaps filled; `std/dom` in the documentation tests and on std's
  site, its examples built for WASM, not run; the error index
  (`docs/errors/`, name per spec), an entry per code with its title,
  explanation, example and fix; the link in each rendered diagnostic
  and the language server's, from one base address; the tests of §3
  decision 6.
- **6.7b, the references:** the language reference (`docs/reference/`,
  recommended over §2's `docs/spec/`): lexical structure, items, types
  and inference, traits and generics, patterns, modules and packages,
  async, attributes, the WASM target's differences, editions, the
  grammar; the CLI and `nova.toml` references; their doc tests and
  coverage tests.
- **Gate:** §1's gate item 3, but for the book.
- **The user's part:** approval only.
- **Size:** 6.7a about 1,000 to 2,000 lines of code and comments, with
  about 75 index entries; 6.7b thousands of lines of Markdown, little
  code.
- **Questions for its spec:** the link's form; `nova explain`
  (recommended: not in 1.0, as it would join the promised CLI); the link
  added by the renderer from the code, not as a note in each diagnostic,
  so the dumps N is compared on stay as they are; `nova-cli` and the
  lint.

### 6.8 — The book, and the site on GitHub Pages
- **Goal:** the tutorial site, hosting the docs and the playground.
- **Deliverables:** `nova-doc` renders a book from
  `docs/book/src/SUMMARY.md` and builds the site (§1's gate item 4,
  with the CLI and manifest references, a home page and one search);
  the book (installing, the basics, errors and `?`, collections, traits
  and generics, async, packages, testing, a web server, a front end in
  WASM, shipping), each code block a doc test; a workflow deploying the
  site to Pages from a `v*` tag or by hand; the root README for users
  (what Nova is, install, hello world, the site, the playground);
  an ADR.
- **Gate:** the site builds in CI with no broken internal link under the
  `/nova/` base; the book's doc tests pass; the deploy job runs green
  once, by hand on the user's word, and the user checks the hosted page.
- **The user's part:** turn Pages on; the word for the Pages actions;
  check the page.
- **Size:** about 800 to 1,500 lines of Rust and workflow, with 3,000 to
  5,000 lines of book; one or two pull requests.
- **Questions for its spec:** a hidden subcommand or flag; the deploy's
  trigger (the tag only, or `main` as a preview); whether std's site
  stays a release asset; the README's two relative links in
  "Self-hosting", which the rewrite replaces.

### 6.9 — The TechEmpower app
- **Goal:** what a submission needs, in the repository (P8).
- **Deliverables:** the app for the JSON and plaintext test types, with
  the files TechEmpower's layout asks for; a CI test building it with
  the checkout's `nova` and checking each response's status, headers and
  body; a `Dockerfile` installing the released `nova`, its version in
  the release's bump and in `release_manifests.rs`; how to submit and to
  run it locally.
- **Gate:** the test passes in CI; beside it, the rules each response
  meets, as read.
- **The user's part:** reading the rules, or leave for the session to
  read them; the pull request to TechEmpower, after `1.0.0`.
- **Size:** about 200 to 500 lines; one pull request; smaller than 5.6a.
- **Questions for its spec:** the rules (headers such as `Date` and
  `Server`, plaintext's pipelining); a process per core, the executor
  using one (`std/net` has no `SO_REUSEPORT`); the app's directory.

### 6.10 — The release `1.0.0`, and the launch materials
Two pull requests, in order, then the publishes.
- **6.10a, the launch materials:** an announcement, on the site and in
  `docs/launch/` (name per spec); drafts of the posts for each site item
  7 names, for the user to edit and post; `[Unreleased]` read through as
  `1.0.0`'s notes.
- **6.10b, the release:** the bump by `docs/releasing.md` with
  `version=1.0.0` and `CHANGELOG.md`'s `[1.0.0]`; then, each on the
  user's word, the tag, the GitHub release, the 20 crates, the
  extension in both stores, the Release gate on three systems, the
  user's try, and the site's deploy from the tag.
- **Gate:** §1's gate item 7; the hosted site and playground at
  `1.0.0`.
- **The user's part:** every [stop] of the runbook; the posts, when the
  user chooses.
- **Size:** prose for 6.10a; about 5.6b's for 6.10b.
- **Questions for its spec:** a release candidate first, or not; the
  runbook's new steps (the deploy, the app's version); whether the
  user's try replaces step 6.2 again, as for `v0.5.0`.

### 6.11 — The records, and the boundary ADR
- **Deliverables:** the boundary ADR, as ADR 0061 closed Phase 5: the
  inventory of the seven items and of this plan's decisions; the gate's
  record with its runs; each external item, done or not (the audit, the
  TechEmpower pull request, the posts, the domain); the backlog (§3
  decision 1's list, ADR 0061's carried); dated notes in the master
  spec's Phase 6, `20-STDLIB.md`, `40-TOOLING.md`, `50-TESTING.md`,
  `docs/releasing.md` and this plan; `[1.0.0]`'s note. Phase 6 is the
  master spec's last, so the ADR says later work goes through the RFC
  process.
- **Gate:** every item of §7's Definition of Done met, as recorded.
- **The user's part:** the external items' status; the merge.
- **Size:** records only, as 5.6c.

## 5. Cross-cutting

- **The workflow** of §4, in every sub-phase. Nothing is pushed,
  merged, tagged or published without the user's word; no auto-merge;
  merges by rebase.
- **Testing:** tests never touch the internet (servers and certificates
  are the test's own); tests run with a fresh `NOVA_HOME`; fuzz targets
  run on fixed seeds in CI and longer locally; the playground is tested
  in Node and in the browser job's Chromium.
- **Dependencies and tools** (§3 decision 12): no new crate, no change
  to `Cargo.lock`, no rustup target, CI tool or third-party action
  without the user's word, after the spec shows it. Rust builds on
  1.78, which CI's MSRV job checks.
- **CI time:** every program type-checks the prelude, so std's growth in
  6.5 (and 6.4, if regex joins the prelude) slows every run, N's
  differential among them, against the Selfhost job's lines of 80, 100
  and 110 s (ADR 0061 `:205`); a line crossed is the user's ruling, as
  in Phase 5. The playground's build and tests, the fuzz targets and the
  dependency check add their own time; each spec measures it.
- **The release runbook for `1.0.0`:** `docs/releasing.md` as for
  `v0.5.0`, with two steps added, the site's deploy from the tag (a
  [stop]) and the app's `Dockerfile` in the bump; `v0.5.0`'s lessons
  stand (ADR 0061 `:340-353`).
- **Records:** the master spec's Phase 6 and the companion specs get
  dated notes as each sub-phase lands, each departure an ADR, and this
  plan dated amendments, as Phase 5's did. ADR 0061 found notes missing
  in Phase 5 (`:430-436`); 6.11 checks every one.
- **Pull requests** carry no local path and no personal detail.
- **GitHub's settings** are the user's: Pages, private vulnerability
  reporting, Dependabot alerts, branch protection, environments.

## 6. Top risks

1. **The playground in a browser** (§2 item 5).
   - Size and speed: 55,749 lines of Rust and 743 KB of std in one
     module, which loads slowly on a phone, and each Run type-checks the
     whole prelude std (6.0 asks whether std's checked items can be
     kept).
   - Stack and dependencies: natively, the backend compiles on a 256 MB
     thread (`crates/nova-codegen-wasm/src/lib.rs:63`); in V8 a deep
     input may overflow, and the page must say so. The chain's `wasm32`
     build is unverified; a dependency that fails needs a `cfg` or a
     replacement, asked.
   - The origin: Pages serves an account's project sites from one
     origin, so a program run there could read what the account's other
     sites keep; programs run in a sandboxed iframe with an opaque
     origin, and no share link runs on open.
   - The remedy: the spike first, with limits and the user's ruling.
2. **Regex's size.** "PCRE-compatible" is open-ended; a backtracking
   engine is exponential on some patterns, a denial of service for a
   server matching untrusted input; Unicode classes need tables; Nova
   code runs about 5× slower than Rust (`docs/phase-5-plan.md:172-174`).
   The remedy: a subset, a linear engine, an import, an oracle.
3. **The client's TLS.** HTTPS puts `rustls` in the runtime, over the
   poller, on three systems; roots go stale; test certificates expire;
   every native program's runtime archive may grow. The remedy: HTTPS
   last (6.5b); if the user declines the crates, HTTP only in 1.0.
4. **The audit may not happen.** 1.0 then ships with in-repo hardening
   alone (P7), as the boundary ADR and `SECURITY.md` say plainly; the
   threat model and inventories make a later audit cheaper, and a
   finding after 1.0 is a patch release.
5. **The phase's length.** Twelve sub-phases, about sixteen pull
   requests, much of it prose (the error index, the reference, the
   book), against 16 weeks. The remedy: the measurable parts first; a
   prose sub-phase may split; nothing external blocks 1.0 (P1).
6. **Two compilers, and CI time.** 6.3 costs twice; std's growth slows
   N's differential toward the Selfhost job's lines; a change missed in
   N fails the differential, which is the guard.
7. **The freeze locks in what exists:** the prelude's names, the
   handler type, `nova.toml`'s keys; the language's gaps (ADR 0061
   `:377-386`) are additive. The policy's exclusions, the snapshots and
   `@deprecated`'s two minor versions' notice are the remedy.
8. **A pin held back for 1.78 gets an advisory.** `ureq` 3.2.1 and
   `wasm-encoder` 0.243.0 cannot move without a newer Rust; the
   dependency check finds it, and the raise is the user's (P11).
9. **TechEmpower's showing.** One executor uses one core (ADR 0009
   `:57`), so a many-core run shows one core's throughput unless the
   app runs a process per core; submitting is the user's decision.
10. **Links that move.** Diagnostics link to the Pages site, which a
    later domain moves; one base address, wording outside the promise,
    and Pages' redirect of the old address (6.8's spec) contain it.

## 7. Suggested first step

6.0: ask the user's word for the local `wasm32-unknown-unknown` target,
run the spike in scratch space, and write 6.0's spec with its figures
and limits, then ADR 0062. Nothing comes before it (P12).
