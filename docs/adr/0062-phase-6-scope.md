# ADR 0062 — Phase 6's scope, the playground and 1.0.0

## Status

Accepted (2026-11-27). Branch `claude/loving-archimedes-frtf5q-60`
(`docs/superpowers/specs/2026-11-26-phase-6-0-the-plan-and-adr-0062-design.md`
§3.2), with `docs/phase-6-plan.md`, which the user approved on
2026-11-26. The scope is the user's decisions of 2026-11-26, P1 to P12
(the plan's §1), with the `wasm32-unknown-unknown` target allowed
locally for the spike and, after it, the long-function limit caught on
the page (Q1). This ADR confirms the plan's §3 decisions 1 to 3; the
others are confirmed in their sub-phases' specs. It names `main`'s
commits only.

## Context

`nova-spec/00-MASTER-SPEC.md` §3 makes Phase 6 "1.0 Release", weeks 105
to 120, the last of its phases, with seven items (`:936-942`): an
external security audit; a stability freeze, "RFC for any breaking
change"; documentation 100%; a tutorial site at `novalang.dev`; an
interactive playground, to "compile in browser via WASM-compiled Nova
compiler" (`:940`); a TechEmpower submission; and launch posts. Unlike
Phases 0 to 5, it has no goal and no gate (`:935-943`). Around it, §1.9
(`:99-102`) allows breaking changes before 1.0 with an RFC, then asks
strict semver, editions for opt-in breaks and two minor versions' notice
before a removal; §5.3 asks every user-facing error for a "Link to
docs"; and §7's Definition of Done tags each phase `v0.{phase}.0`, which
reads `v0.6.0` for this one.

ADR 0048 left N built for WASM out of Phase 5 as what "Phase 6's
playground needs" (`:44`), and ADR 0061 kept that mapping: R5 (`:93`),
its inventory's row (`:148`) and its backlog (`:394-395`). The master
spec's `:940` names no stage, but its own note under Phase 5, added with
ADR 0061, also maps "building the compiler for WASM" to Phase 6's
playground (`:929-930`); the dated note under Phase 6's list, added with
this ADR, answers it.

The research of 2026-11-26 on `main` at `4fe11d55`
(`docs/phase-6-plan.md` §2), a line for each item:
- **the audit:** no `SECURITY.md`, threat model, fuzz targets or
  dependency check; 254 packages in `Cargo.lock`, two pins held back by
  MSRV 1.78;
- **the freeze:** no stability policy or RFC process, no test of std's
  signatures, no way to deprecate; two S0 bugs ADR 0061 keeps; issue #3;
- **documentation:** std at 100% by test, but for `std/dom`, which ADR
  0047 left off the site; no `missing_docs`, reference, book or error
  index;
- **the site:** none, no Pages workflow, and no `novalang.dev`;
- **the playground:** S0's WASM backend needs no other toolchain (ADR
  0039); the front end reads through `Sources`
  (`crates/nova-driver/src/analyze.rs:25-42`); `build_program_wasm`
  builds in memory before it writes (`lib.rs:865-892`); the front-end
  crates and `nova-codegen-wasm` use no OS API but codegen's 256 MB
  thread; `nova-driver`, which runs `clang` and depends on the native
  runtime, cannot be the browser's entry;
- **TechEmpower:** a server, with no database driver and no client;
- **the launch:** the user's; the repository can give notes and drafts;
- **the release:** `docs/releasing.md` runs unchanged for `1.0.0`;
- **std's v1.0 index:** unbuilt, `Queue`, atomics and `RwLock`, UDP and
  Unix sockets, the HTTP client, AEAD and `std/regex`;
- **two compilers:** CI holds N equal to S0, so a change to a ported
  part changes both;
- **the tools:** Node 22, Rust 1.97 with the 1.78 toolchain, and no
  `wasm32-unknown-unknown` target, which was then installed locally for
  the spike.

N built for WASM is further off: its MIR is native only (ADR 0057
`:521-534`), and under its `async fn main` a compile, one long turn,
would never collect (ADR 0040 `:42-43`). Names, as in Phase 5's plan:
**S0** is the Rust `nova`; **N** is the Nova compiler in `selfhost/`.

The user's decisions of 2026-11-26, labelled as in the plan's §1:
**P1,** all seven items, the external parts the user's; **P2,** the
playground S0 in the browser after a spike, N for WASM to the backlog;
**P3,** `1.0.0` as the phase's release; **P4,** GitHub Pages until there
is a domain; **P5,** documentation, all of it; **P6,** what the
stability promise covers; **P7,** in-repository hardening as the audit's
gate; **P8,** TechEmpower's test types that need no database; **P9,**
`@deprecated`, the two kept bugs, issue #3, `std/regex` and the HTTP
client before the freeze; **P10,** none of ADR 0061's backlog; **P11,**
MSRV 1.78, with a policy; **P12,** the spike first. The spike, on the
target the user allowed locally, measured S0's front end and WASM
backend built for `wasm32-unknown-unknown`: their size, speed, memory
and output, and the depth cases at the parser's limit of 128 levels (ADR
0052), as below. After it, **Q1:** a long function's overflow is caught
on the page.

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
     toolchains (`nova-spec/40-TOOLING.md:646`; ADR 0037 `:93`) and
     `nova-embed` (`nova-spec/13-RUNTIME.md:480`);
   - what the specs and earlier ADRs put elsewhere, not in Phase 6, left
     there: the `setup-nova` action (`40-TOOLING.md:583-598`), on Phase
     3's backlog (ADR 0026 `:50`; ADR 0037 `:100`); examples 08 to 15
     (`nova-spec/60-EXAMPLES.md:801-803`), "post-Phase 6", after the 1.0
     release; and edition rules (`40-TOOLING.md:436-439`), on Phase 3's
     backlog (ADR 0037 `:97`).
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
     case at the parser's limit, 128, survived with no trap. 6.1's gate
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
     hook in the shim; the target in CI. No new dependency.
4. **The version: `1.0.0`** (P3) is Phase 6's release, in place of the
   `v0.{phase}.0` of the master spec's §7, item 6, which reads `v0.6.0`
   for this phase. `docs/releasing.md` runs unchanged with
   `version=1.0.0`. A `1.0.0-rc.N` comes only if the audit, the freeze
   or the Release gate asks for one, on the user's word. Semver is strict
   after it (the master spec's §1.9, `:101`).
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

## References

- Spec: `docs/superpowers/specs/2026-11-26-phase-6-0-the-plan-and-adr-0062-design.md`,
  and its plan,
  `docs/superpowers/plans/2026-11-27-phase-6-0-the-plan-and-adr-0062.md`
- `nova-spec/00-MASTER-SPEC.md` §1.9, §3, §5.3 and §7
- `nova-spec/13-RUNTIME.md`, `nova-spec/20-STDLIB.md` §1,
  `nova-spec/40-TOOLING.md` and `nova-spec/60-EXAMPLES.md`
- `docs/phase-6-plan.md` §1, §3, and §4's 6.0 and 6.1
- `docs/phase-5-plan.md` §2 item 9, Phase 5's lexer spike
- `docs/releasing.md`
- ADRs 0026, 0037, 0038, 0039, 0040, 0047, 0048, 0052, 0057 and 0061
- `crates/nova-codegen-wasm/src/lib.rs:61-63` and `:198-208`;
  `crates/nova-driver/src/analyze.rs:25-42` and `lib.rs:865-892`
- `.github/workflows/ci.yml`'s browser job
