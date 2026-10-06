# Nova — Master Specification

> **Audience:** Claude Code (or any AI coding agent / engineer) executing the build.
> **Mode:** All decisions are FINAL. No questions back to user. Execute in order.
> **Last updated:** 2026

---

## 0. Project Identity (FINAL)

| Field | Value |
|---|---|
| Language name | **Nova** |
| File extension | `.nova` |
| CLI binary | `nova` |
| Package manager | `nova` (subcommand) |
| Registry domain | `registry.novalang.dev` (placeholder, can change) |
| GitHub org | `novalang` (placeholder) |
| Primary repo | `novalang/nova` |
| License | **MIT OR Apache-2.0** dual license |
| Bootstrap language | **Rust** (edition 2021, MSRV 1.78) |
| Self-hosting target | Phase 5 (~month 42) |

If "Nova" is taken when you check the registry, fall back to: `Nyx`, `Lumen`, `Vela`, `Astra` — in that order.

---

## 1. Locked Technical Decisions

These are NOT up for debate. If a tradeoff appears mid-implementation, prefer the choice listed here.

### 1.1 Compilation
- **Backend (server):** Native AOT via LLVM (release) + Cranelift (debug, fast iteration)
- **Frontend (browser):** WebAssembly (WASM) + auto-generated JS shim
- **No JIT.** No interpreter beyond REPL eval.
- **No Virtual Machine.** No bytecode distribution format.

### 1.2 Memory
- **Default:** Tracing GC (mark-and-sweep, generational later)
- **GC implementation v0:** wrap MMTk (modular GC framework in Rust). Fall back to bdwgc if MMTk integration too heavy in Phase 1.
- **Future:** opt-in ownership annotations in v2.0 (`@own`, `@borrow`) — NOT in v1.0
- **No raw pointers in safe code.** `unsafe` block required.

### 1.3 Type System
- **Static, sound, with type inference** (Hindley-Milner + extensions)
- **Generics:** monomorphization (like Rust/C++)
- **Sum types (algebraic data types):** first-class
- **Traits:** Rust-style, no inheritance, no implicit conversions
- **Null safety:** `Option<T>`. There is no `null` keyword.
- **Error handling:** `Result<T, E>` + `?` operator. **No exceptions.** `panic!` for unrecoverable only.

### 1.4 Concurrency
- **async/await** as the default concurrency model
- **Lightweight tasks** scheduled on a work-stealing thread pool (Tokio model)
- **Channels** for message passing (`std/sync/channel`)
- **Structured concurrency**: every spawned task has a parent, cancellation propagates

### 1.5 Syntax Family
- **TypeScript / Swift inspired** — curly braces, expression-oriented
- **Significant whitespace: NO** (curly braces win every time)
- **Semicolons: optional** (newline terminates statement; semicolons allowed for one-liners)
- **String interpolation:** `"Hello, ${name}"` (TS-style)
- **Comments:** `//` line, `/* */` block, `///` doc

### 1.6 Module System
- **File path == module path** (Go/Rust hybrid)
- **`pub` keyword** for visibility
- **No circular imports** (compiler-enforced)
- **One package per `nova.toml`**

### 1.7 Tooling
- **Single binary `nova`** dispatches all subcommands (no plugins in v1)
- **Formatter is opinionated** (no config, like gofmt)
- **LSP built-in:** `nova lsp`
- **No third-party build tools.** `nova build` is the only path.

### 1.8 Frontend
- **Reactivity model:** signals (SolidJS-style), NOT virtual DOM
- **SSR/SSG:** built-in flags on `nova build`
- **Bundler:** built-in (`nova bundle`)
- **HMR:** built-in (`nova dev`)

### 1.9 Versioning & Stability
- **Pre-1.0:** breaking changes allowed in minor versions, RFC required
- **Post-1.0:** semver strict, edition system (`edition = "2026"`) for opt-in breaking
- **Deprecation:** minimum 2 minor versions before removal

---

## 2. Folder Structure (FINAL)

Create exactly this layout:

```
nova/
├── README.md
├── LICENSE-MIT
├── LICENSE-APACHE
├── CONTRIBUTING.md
├── CODE_OF_CONDUCT.md
├── ARCHITECTURE.md
├── Cargo.toml                    # workspace root
├── rust-toolchain.toml           # pin Rust version
├── .github/
│   ├── workflows/
│   │   ├── ci.yml
│   │   ├── release.yml
│   │   └── benchmarks.yml
│   ├── ISSUE_TEMPLATE/
│   └── PULL_REQUEST_TEMPLATE.md
├── crates/
│   ├── nova-cli/                 # `nova` binary entry point
│   ├── nova-driver/              # orchestrates compile pipeline
│   ├── nova-lexer/
│   ├── nova-parser/
│   ├── nova-ast/
│   ├── nova-resolver/            # name resolution
│   ├── nova-typeck/              # type checker
│   ├── nova-hir/                 # high-level IR
│   ├── nova-mir/                 # mid-level IR
│   ├── nova-codegen-llvm/
│   ├── nova-codegen-cranelift/
│   ├── nova-codegen-wasm/
│   ├── nova-runtime/             # GC + async runtime (Rust, linked into binaries)
│   ├── nova-fmt/                 # formatter
│   ├── nova-lsp/                 # LSP server
│   ├── nova-test/                # test runner
│   ├── nova-pm/                  # package manager
│   ├── nova-bundler/             # frontend bundler
│   ├── nova-doc/                 # doc generator
│   └── nova-diagnostics/         # error reporting (shared)
├── std/                          # stdlib written in Nova
│   ├── core/
│   ├── io/
│   ├── fs/
│   ├── net/
│   ├── http/
│   ├── json/
│   ├── crypto/
│   ├── time/
│   ├── log/
│   ├── test/
│   ├── fmt/
│   ├── collections/
│   ├── strings/
│   ├── regex/
│   ├── process/
│   ├── sync/
│   ├── task/
│   └── ui/                       # frontend (Phase 4)
├── examples/
│   ├── 01-hello-world/
│   ├── 02-fibonacci/
│   ├── 03-http-server/
│   ├── 04-todo-cli/
│   ├── 05-json-api/
│   ├── 06-counter-spa/           # frontend example
│   └── 07-fullstack-blog/
├── tests/
│   ├── compile-pass/             # must compile
│   ├── compile-fail/             # must error with snapshot
│   ├── runtime/                  # must run with expected output
│   ├── ui/                       # frontend WASM tests
│   └── benchmarks/
├── docs/
│   ├── spec/                     # formal language specification
│   │   ├── grammar.bnf
│   │   ├── semantics.md
│   │   └── stdlib-reference.md
│   ├── book/                     # The Nova Book (mdBook)
│   │   ├── book.toml
│   │   └── src/
│   ├── adr/                      # architecture decision records
│   │   └── 0001-native-aot.md
│   └── rfcs/
│       └── 0000-language-overview.md
└── tools/
    ├── vscode-nova/              # VSCode extension
    ├── zed-nova/
    └── nvim-nova/
```

---

## 3. Build Order (Strict)

Execute phases sequentially. Each phase has gating criteria — do not advance until met.

**AMENDED 2026-09-03 (branch `phase-2-gate-benchmark`): Phase 2's gate below
is specified twice, with two criteria that are not equivalent.** This
section's own Phase 2 gate, below, reads "`examples/05-json-api` serves
10k+ req/sec on benchmark hardware"; `nova-spec/60-EXAMPLES.md` §5's own
gate for the same example reads "Benchmark vs Bun on same hardware shows
≥ 1.0x req/sec ratio." An absolute 10k and a ratio against Bun can disagree
in either direction — 10k could be reached while the ratio fails, or the
ratio could clear 1.0 well under 10k if Bun itself is slower on the same
machine. This increment measures only the absolute figure, on one host, one
run: `docs/benchmarks/README.md` documents the procedure and
`docs/benchmarks/http-fixed-response.md` records 11,940.0 req/sec against
`std/http`'s read-and-parse path, excluding response serialisation, on the
Cranelift backend rather than the optimising LLVM one, which numerically
clears the criterion stated below. The ratio against Bun that §5 also asks
for is entirely unmeasured, and `examples/05-json-api` itself still does not
exist (`nova-spec/60-EXAMPLES.md` §5 carries its own dated amendment on what
that example would need). No claim is made here that Phase 2's gate, below,
is passed.

**ALSO AMENDED 2026-09-03 (branch `phase-2-gate-benchmark`): these amendments
inserted lines at the top of section 3, so every line below them in this file
moved down, and line-number citations into this section written before this
date now point above what they meant.** No line count is given for the shift,
and that is deliberate rather than lazy: a stated offset is falsified by the
next edit to this section, including edits to this note. Two attempts at a
figure here were each true only until the paragraph containing them was
written or revised. What follows instead is a translation from each stale
anchor to something that does not move — a list position and the module it
names — which is itself the reason the durable form of a citation here is not
a number. The
Phase 2 gate that was at line 245 is the `**Gate:**` line under Phase 2 below;
the Phase 2 build-order entries that were at lines 238, 240 and 241 are
positions 8, 10 and 11 of the numbered list under the same heading —
`std/sync`, `std/http` and `std/json` respectively. So the compound form
`:240-241` names positions 10 and 11 *together*, which is how the documents
using it read it: they cite the pair to contrast `std/http` with `std/json`,
and losing `std/http` from that pair loses the contrast they were drawing.
Citations of `00-MASTER-SPEC.md:238`, `:240`, `:241`, `:240-241` and `:245`
appear in
`CHANGELOG.md`, `nova-spec/20-STDLIB.md`,
`docs/adr/0016-std-sync-partial-close.md`,
`docs/adr/0017-std-sync-channel-shape.md`,
`docs/adr/0018-std-json-scope-and-build-order.md`,
`docs/adr/0019-offset-table-intrinsic-boundary.md`, and earlier specs and
plans under `docs/superpowers/`. Each of those sits in a dated record this
project amends rather than rewrites, so every one is left exactly as written
and this note is what makes the shift discoverable — at the place a reader
following a stale citation arrives. **Cite this section by heading, not by
line number.** A line number in a file that gets amended is the same class of
fragile pointer as a branch-local commit hash: it dangles while still reading
as precise, and replacing one stale number with a fresh one only restarts the
countdown. That is why the citations this increment wrote for itself name the
section rather than a line.

**AMENDED 2026-09-03 (branch `std-crypto-hashes-hmac-random`, a different
increment from the `phase-2-gate-benchmark` one dated the same day): Phase 2
position 12 is now partially built — and the finding worth recording is that
NO claim in this section is falsified by that.** A reader arriving here
expecting a correction should stop looking for one. Position 12's entry under
Phase 2 below reads "`std/crypto` (wrap `ring` at runtime)": that was a
pending directive, and it is now carried out as written. `ring = "0.17"` is
declared in `crates/nova-runtime/Cargo.toml` and `ring` 0.17.14 is in the
tracked `Cargo.lock`, with `untrusted` 0.9.0, `wasi` 0.11.1 and `getrandom`
0.2.17 behind it; `std/crypto/lib.nova` reaches it through three runtime
intrinsics, which is the runtime layer the parenthetical specifies. Under §6
below, the `ring = "0.17"` line is the version this increment pinned, so that
line came true rather than going stale.

**What that entry does and does not commit to.** It names no primitive, so
what ships — SHA-256, SHA-512, HMAC-SHA-256, a constant-time HMAC tag check,
random bytes and a bounded random integer — and what does not — AEAD, and
BLAKE3, which the `ring` backing does not implement at all — leaves this
section untouched either way. That exposure belongs to
`nova-spec/20-STDLIB.md` §8, whose own declared surface names both and which
carries its own dated amendment; audit that section, not this one. Cited by
heading, per the instruction the note above gives.

**Two things this increment did not do, said here so a later reader does not
credit it with them.** It added no new deviation from the order this section
specifies: `std/crypto` shipped after `std/net`, `std/http` and `std/json`,
which is where position 12 sits relative to those three. **It did not ship
before `std/test`, and a build-order audit needs that pair the other way
round.** Position 13's `std/test/lib.nova` reached disk on 2026-08-07 (commit
`6aa4296`, an ancestor of `main`), a month ahead of `std/crypto` and ahead of
every lower-numbered position whose module reached disk after that date. That
is an older deviation than this increment, which neither created nor closes
it, and no record captures it:
`docs/adr/0014-stdlib-build-order-deviations.md`, the ADR *for* build-order
deviations, records position 2's skips and names neither `std/test` nor
position 13. The durable check is `git log --diff-filter=A --format=%ad --
std/<name>/lib.nova` for each entry in the Phase 2 list below, not this
sentence. And it put `ring` in a member crate's manifest rather than the
workspace root the line introducing §6's block asks for — continuing the
placement `httparse` had already established
before `std/crypto` existed, so the departure from that instruction is older
and broader than this increment and is not amended here.

**Phase 2's gate is still not reached.** `examples/05-json-api` still does not
exist, and no measurement in this increment bears on the throughput criterion
or on the ratio `nova-spec/60-EXAMPLES.md` §5 asks for. Nothing here narrows
the first amendment above.

**AMENDED 2026-09-10 (branch `examples-05-json-api`): `examples/05-json-api`
now exists and has been measured. Phase 2's gate, below, is still NOT met, and
the reason it is not met has changed.** Cited by heading, per the instruction
two amendments above. The example serves `GET /users`, `POST /users` and
`GET /users/:id` over `std/http`, written in the language that exists rather
than as `nova-spec/60-EXAMPLES.md` section 5's listing spells it — that
section carries its own dated amendment on the substitutions and on two
corrections to what it had recorded as absent. So the clause in the first
amendment above reading "`examples/05-json-api` itself still does not exist" no
longer holds, and neither does the same clause in the paragraph directly above
this one; both are left as written and superseded here.

**The figure, and what it is a figure for.** Measured 2026-09-10 against
`/users`, the endpoint section 5's own methodology names, at a ten-user
collection: **455.5 req/sec** over a 494-byte body, no errors, on the Cranelift
backend with the release runtime profile, 200 connections, 30s after a 5s
warmup. This section's Phase 2 gate asks for 10k+, so the endpoint the gate
names is short by a factor of roughly twenty-two. **Nothing here claims the
gate is passed, and no record should read this figure as passing it.** Every
axis above belongs to the number; `examples/05-json-api/BENCHMARK.md` holds the
run lines, two further collection sizes, and what the measurement does and does
not settle.

**AMENDED 2026-09-11: the 455.5 req/sec figure above is WITHDRAWN.** That run
measured a binary built by a debug `nova`, so the debug runtime was linked --
the hazard `docs/benchmarks/README.md` names in its own verdict table, in a
procedure that had no step checking it. The stale binary was still on disk
and was re-measured at 258.9 and 386.7 req/sec, beside 2364.8 for the same
source built by the release `nova`. Binary size turned out to be a pure
function of the `nova` profile, and a 2x2 over profile and source revision is
what established that, rather than a single matching size.
**Corrected, with one fresh server process per data point: 1875.2 to 3108.5
req/sec at ten users over six fresh-process runs, median 2328.2 -- twelve
release-runtime runs in all, fresh and aged, span 1769.6 to 3108.5.** So the
shortfall
against 10k+ is roughly **3x to 5x**, not twenty-two. The derived claims
invert too: response construction is about a third of per-request cost rather
than 7.6%, the amplification over the isolated body-building cost is 2x to 3x
rather than 13x, and the unattributed residual is on the order of a fifth
rather than 92%. **The gate is still NOT met and nothing here claims
otherwise.** `examples/05-json-api/BENCHMARK.md` carries the amendment, both
identification tables, the corrected runs, and two confounds found in the
withdrawn methodology.

**Only one of the gate's two statements has been measured, and they are still
not equivalent.** The first amendment above records that Phase 2's gate is
specified twice with criteria that can disagree in either direction: this
section's absolute 10k, and `nova-spec/60-EXAMPLES.md` section 5's ratio of at
least 1.0 against Bun. The absolute one is what 455.5 req/sec answers, and it
answers it in the negative. **The Bun ratio remains unmeasured** — but Bun
1.3.0 is installed on this project's development host, so that half is now
measurable rather than blocked, which is a different thing to inherit than it
was when the first amendment was written. A later increment can take it
deliberately.

**What this increment did not do, said here so a later reader does not credit
it.** It changed no build-order position and closed no deviation, including the
`std/test` ordering the amendment above records as older and open. It shipped no
language feature: the router's `Handler` type alias still does not parse,
`@derive` is not implemented, and there is no `?` operator and no turbofish.
`Map` still has `keys()` and no `values()`. The example routes around each of
those rather than removing any of them.

**AMENDED 2026-09-11: the gate's two criteria, recorded above as able to
disagree in either direction, now both have a measured figure — and they
agree.** The absolute figure, above, is 1875.2 to 3108.5 req/sec at ten
users against the 10k+ this section asks for, short by roughly 3x to 5x.
`nova-spec/60-EXAMPLES.md` section 5's own ratio against Bun is now
measured too, at the same ten-user `/users` collection this section's own
figure uses: 0.116 to 0.204 pinned (the headline), 0.185 to 0.231
unpinned — combined, 0.116 to 0.231 — against the ≥ 1.0 that section asks
for, short by roughly 4.3x to 8.6x. **Both criteria say the gate is not
met.** That they agree here is not guaranteed by their definitions — the
amendment above is explicit that an absolute figure and a ratio against a
second, independently-variable server can disagree in either direction —
it is what this increment found on measuring both. Full account, including
the fairness check on pinning and the wire-framing bias in Nova's favour,
in `examples/05-json-api/BENCHMARK.md` and `docs/benchmarks/README.md`.

**AMENDED 2026-09-12 (branch `remeasure-after-fast-path`): both of the
gate's criteria are re-measured against a binary built from `stringify`'s
scalar fast path, and both remain unmet.** That fast path shipped on a
separate, earlier branch; the amendment immediately above already named
every `stringify`-affected figure in this section as measuring the build
before it. Re-measured with one fresh process per point, across the pinned and
unpinned cells alike, against
`json-api.exe` at **690,688 bytes** (690,176 before — the identity check
working), the equivalence check re-run first and all nine exchanges still
matching on status and body bytes: this section's absolute criterion is
**2868.2 to 3392.4 req/sec** at ten users, short of the 10k+ asked for by
roughly **2.9x to 3.5x** rather than 3x to 5x. `nova-spec/60-EXAMPLES.md`
§5's ratio against Bun, re-measured as a four-cell matrix with replicates,
is **0.230 to 0.272**, short of its ≥ 1.0 by roughly **3.7x to 4.4x**
rather than 4.3x to 8.6x. **Both criteria moved and both say the gate is
still not met.** Nor does this bring it within reach through further work
on the response path — a claim with standing history rather than one
minted here: `CHANGELOG.md`'s `[0.2.0-alpha.4]` entry and
`docs/superpowers/specs/2026-09-11-bun-ratio-design.md` §2 both record
that the absolute criterion is not reachable by response-path work alone,
because the gate allows 100 microseconds per request and the empty-store
control already exceeds it. **This session's control, which never calls
`stringify`, reads 105.3 to 115.1 microseconds per request against that
100-microsecond line — the fastest reading is within about 5% of it,
while the control's own two readings differ from each other by 1.09x, a
wider swing than that margin.** The claim holds on every reading taken so
far, but it is not a settled impossibility: it rests on a quantity that
moves by more than the margin it has left, so a later reader should
re-derive it rather than quote it forward. What that change did to the
server's own per-request cost, and how it compares with the saving measured
inside `users_json`, is reported in `examples/05-json-api/BENCHMARK.md` and
deliberately not restated here: it is a comparison whose size depends on
which readings each side is drawn from, and repeating it in every record is
how it would go stale in every record. Full account in
`examples/05-json-api/BENCHMARK.md`.

**RE-DERIVED 2026-09-29, as the paragraph above instructs, and it moved.**
Four fresh-process readings of that same empty-store control, same route,
same parameters, on a binary of the same byte size, give **96.15 to 99.26
microseconds per request — all four clearing 10k req/sec and all four
below the 100-microsecond line.** The two readings above do not. Pooled,
the control spans a **1.20x range that straddles the criterion**, while the two sessions' ranges do NOT OVERLAP -- 8688.1 to 9501.0
against 10074.1 to 10400.4 -- so the movement is between sessions
rather than within one. (Within-session spreads are 1.09x for the
earlier pair and 1.03x for this session's four. An earlier draft cited
only the 1.03x and called it "within-session spread", generalising one
session's figure to both -- contradicted by the 1.09x these same records
already state.) **The claim that the absolute criterion is
unreachable by response-path work alone is therefore no longer supported by
this control — and it is not refuted either.** That is the outcome the
"not a settled impossibility" sentence above anticipated, and the figure
above is left standing as what was measured then. An empty store is not
the gate's workload, and the gate's own figure is unchanged: measured, and
not met. Readings and method in `docs/benchmarks/README.md` under
"Differential decomposition, 2026-09-29".

**Amended 2026-10-02 (gate-remeasure):** remeasured on `main` at `5efcc2e`
with the 2026-09-11 method (ten users, 200 connections, 30 s after a 5 s
warmup).
- **This section's absolute criterion, split by pinning:** met unpinned at
  10250.0–10382.4 req/sec, both readings 2.5–3.8% above 10k; not met pinned
  to one core, at 8424.0–8520.1.
- **Pooled the way the 2026-09-12 figure pooled them:** 8424.0–10382.4,
  straddling 10k.
- **`60-EXAMPLES.md` §5's ratio against Bun is still not met:** 0.70–0.79
  pinned, 0.79–0.81 unpinned.

The gate's two statements now disagree, the case the 2026-09-03 amendment
above anticipated. Under any reading that counts §5, the gate is not met.
The unpinned absolute pass rests on two readings with a thin margin. Full
account in `examples/05-json-api/BENCHMARK.md`, "AMENDMENT 2026-10-02
(gate-remeasure)".

**Amended 2026-10-02 (gate-remeasure-2):** remeasured on `main` at `cdaea7e`
with the same method, three replicates per cell.
- **This section's absolute criterion is met in all six readings:** pinned
  to one core at 10986.5–12403.2 req/sec, unpinned at 10768.5–12126.4.
- **`60-EXAMPLES.md` §5's ratio against Bun is still not met:** 0.854–0.976
  unpinned, with the ranges disjoint; 0.715–1.334 pinned, straddling 1.0.

The gate's two statements still disagree. This section's criterion is now
met under either pinning condition, and §5's is not. Under any reading that
counts §5, the gate is not met. Full account in
`examples/05-json-api/BENCHMARK.md`, "AMENDMENT 2026-10-02 (gate-remeasure-2)".

**Amended 2026-10-02 (gate-remeasure-3):** remeasured on `main` at `012ca55`
with the same method, after a byte-level `String.join`.
- **This section's absolute criterion is met in all six readings:** pinned
  at 17682.5–18528.1 req/sec, unpinned at 17400.4–20014.4. Every cell,
  Bun's included, ran much faster than in earlier runs, for reasons not
  measured, so these figures do not compare with earlier ones.
- **`60-EXAMPLES.md` §5's ratio against Bun is still not met:** 0.886–0.993
  pinned, with the ranges disjoint; 0.891–1.076 unpinned, straddling 1.0.

The gate's two statements still disagree. Under any reading that counts §5,
the gate is not met. Full account in `examples/05-json-api/BENCHMARK.md`,
"AMENDMENT 2026-10-02 (gate-remeasure-3)".

**Amended 2026-10-03 (gate-remeasure-4):** remeasured on `main` at `7f2b85e`
with the same method, six rounds per cell, after byte-level string search
and a runtime builtin behind `std/json`'s `quote`.
- **This section's absolute criterion is met in all twelve Nova readings:**
  pinned at 12010.9–14557.6 req/sec, unpinned at 13200.4–15506.7. Every
  cell, Bun's included, ran slower than in "(gate-remeasure-3)", for
  reasons not measured, so these figures do not compare with earlier ones.
- **`60-EXAMPLES.md` §5's ratio against Bun is not established:**
  0.708–1.334 pinned and 0.822–1.279 unpinned, both straddling 1.0.

The gate's two statements still disagree. Under any reading that counts §5,
the gate is not met. Full account in `examples/05-json-api/BENCHMARK.md`,
"AMENDMENT 2026-10-03 (gate-remeasure-4)".

**Amended 2026-10-03 (gate-ratio-criterion):** `60-EXAMPLES.md` §5's ratio
is now judged by twelve paired pinned rounds, with a margin for Bun's
`Date` header, per `docs/adr/0021-gate-ratio-paired-rounds.md`. This
section's absolute criterion is unchanged. Which of the two statements
governs is still not settled.

**Amended 2026-10-03 (gate-remeasure-5):** remeasured on `main` at
`1972b37`, the first run judged under `docs/adr/0021-gate-ratio-paired-rounds.md`.
- **This section's absolute criterion is met in all 24 Nova readings:**
  pinned at 13086.8–25031.1 req/sec, unpinned at 13463.0–24419.0. The
  upper ends are from rounds 9 and 10, when every reading, Bun's included,
  ran faster, for reasons not measured. So these figures do not compare
  with earlier runs'.
- **`60-EXAMPLES.md` §5's ratio against Bun is inconclusive:** Nova
  cleared the 1.0547 margin in 8 of 12 pinned rounds, where 10 are needed,
  though it was faster in plain req/sec in all 12.

The gate's two statements still disagree. Under any reading that counts §5,
the gate is not met. Full account in `examples/05-json-api/BENCHMARK.md`,
"AMENDMENT 2026-10-03 (gate-remeasure-5)".

**Amended 2026-10-03 (gate-remeasure-6):** remeasured on `main` at
`b24379e`, the second run judged under `docs/adr/0021-gate-ratio-paired-rounds.md`.
- **This section's absolute criterion is met in all 24 Nova readings:**
  pinned at 14455.1–22200.2 req/sec, unpinned at 14589.1–22089.4.
- **`60-EXAMPLES.md` §5's ratio against Bun is inconclusive:** Nova
  cleared the 1.0547 margin in 7 of 12 pinned rounds, where 10 are needed.
  Unpinned, which does not decide, it cleared it in all 12.

The gate's two statements still disagree. Under any reading that counts §5,
the gate is not met. Full account in `examples/05-json-api/BENCHMARK.md`,
"AMENDMENT 2026-10-03 (gate-remeasure-6)".

**Amended 2026-10-04 (gate-remeasure-7):** remeasured on `main` at
`6fda78b`, the third run judged under `docs/adr/0021-gate-ratio-paired-rounds.md`.
- **This section's absolute criterion is met in all 24 Nova readings:**
  pinned at 14198.0–16277.5 req/sec, unpinned at 15128.3–16301.7.
- **`60-EXAMPLES.md` §5's ratio against Bun is met:** Nova cleared the
  1.0547 margin in all 12 pinned rounds, where 10 are needed. Unpinned,
  which does not decide, it cleared it in all 12 too.

Both of the gate's statements are now met, so the `examples/05-json-api`
benchmark gate under Phase 2 below is met, on this development host,
Windows, with the load generator on the same machine. That gate line asks
for "benchmark hardware", which the spec does not define; these figures
are from this host. `60-EXAMPLES.md` §3 (`03-http-server`) and §4
(`04-todo-cli`) are also labelled Phase 2 gates; neither exists under
`examples/` and nothing here judges them, so this does not say Phase 2 is
complete. Full account in `examples/05-json-api/BENCHMARK.md`,
"AMENDMENT 2026-10-04 (gate-remeasure-7)".

**Recorded 2026-10-04 (branch `examples-03-http-server`):** `03-http-server`
now exists under `examples/`, and end-to-end tests of both of its gate
clauses run on all three CI operating systems; see `nova-spec/60-EXAMPLES.md`
§3. `04-todo-cli` still does not exist, so Phase 2 is still not complete.

**Recorded 2026-10-05 (branch `examples-04-todo-cli`):** `04-todo-cli` now
exists too, and end-to-end tests of its gate run on all three CI operating
systems; see `nova-spec/60-EXAMPLES.md` §4. Every example that file labels a
Phase 2 gate (§3, §4 and §5) now exists and passes. This note does not assess
whether Phase 2 is complete.

**Recorded 2026-10-06 (branch `phase-2-closeout`): Phase 2's boundary is
`docs/adr/0025-phase-2-boundary.md`.** Its inventory table lists every
promise the master spec's §3 and `docs/phase-2-plan.md` make for Phase 2,
whether it shipped, and where it is recorded; what is not built goes to an
unscheduled backlog. With that ADR and `nova-spec/20-STDLIB.md` §18 and §19,
every item of the master spec's §7 Definition of Done is met but the last,
the `v0.2.0` tag.

### Phase 0 — Foundation (week 1–4)
**Goal:** Repo skeleton + lexer + parser for a minimal subset.

Files to create in order:
1. Workspace `Cargo.toml`, `rust-toolchain.toml`, root README, LICENSE files
2. CI: `.github/workflows/ci.yml` (cargo test, fmt, clippy on PR)
3. `crates/nova-diagnostics/` — error reporting infrastructure (use `codespan-reporting`)
4. `crates/nova-lexer/` — see [10-LEXER.md]
5. `crates/nova-ast/` — AST node definitions
6. `crates/nova-parser/` — see [11-PARSER.md], use **chumsky** (Pratt-style for expressions)
   **Amended 2026-10-06 (branch `phase-2-closeout`):** the parser is
   hand-written recursive descent, with no chumsky and no Pratt combinator;
   `docs/adr/0025-phase-2-boundary.md` decides it stays.
7. `crates/nova-cli/` — wires `nova parse <file>` for testing parser
8. Snapshot testing harness (use `insta` crate)

**Gate:** parse all code in `examples/01-hello-world/` and `examples/02-fibonacci/` to AST.

---

### Phase 1 — MVP Compiler (week 5–24)
**Goal:** Compile Nova source → native binary that prints output.

1. `crates/nova-resolver/` — name resolution, module graph
2. `crates/nova-typeck/` — see [12-TYPESYSTEM.md], implement HM with extensions
3. `crates/nova-hir/` — desugar AST → HIR
4. `crates/nova-mir/` — lower HIR → MIR (3-address-style)
5. `crates/nova-codegen-cranelift/` — fast debug backend FIRST (faster iteration than LLVM)
6. `crates/nova-runtime/` — minimal: panic, allocator, basic types
7. `crates/nova-driver/` — pipeline orchestration
8. `nova-cli`: implement `nova run` and `nova build`
9. `crates/nova-codegen-llvm/` — release backend (use `inkwell`)

**Gate:** all 4 of these run via `nova run`:
- `examples/01-hello-world` (println)
- `examples/02-fibonacci` (recursion + arithmetic)
- A "match on enum" example
- A "generic function" example

---

### Phase 2 — Standard Library Core (week 25–40)
**Goal:** Server-side apps work. Benchmark vs Bun.

Implement std modules in order (each module is a doc in [20-STDLIB.md]):
1. `std/core` (primitives, Option, Result, traits)
2. `std/fmt`, `std/io` (println, eprintln, file handles)
3. `std/collections` (Vec, Map, Set)
4. `std/strings`
5. `std/fs`
6. `std/time`, `std/log`
7. `std/task` (async runtime — wrap Tokio in Rust runtime crate, expose Nova API)
8. `std/sync` (Mutex, channel, atomic)
9. `std/net` (TCP, UDP)
10. `std/http` (server first, then client; use hyper's HTTP/1 **parsing** internals — `httparse` — at the runtime layer; hyper's own executor and connection driver are unavailable on this runtime for three measured reasons, see `docs/adr/0019-offset-table-intrinsic-boundary.md`)
11. `std/json` (custom parser, type-safe codec via traits)
12. `std/crypto` (wrap `ring` at runtime)
13. `std/test` (test runner — `nova test`)

**Gate:** `examples/05-json-api` serves 10k+ req/sec on benchmark hardware. Document benchmark methodology in `docs/benchmarks/`.

---

### Phase 3 — Tooling (week 41–56)
**Goal:** DX matches or exceeds Rust/Go.

1. `crates/nova-fmt` — formatter (no options, only `--check`)
2. `crates/nova-pm` — package manager + `nova.toml` parsing + lock file
3. Registry server (separate repo `novalang/registry`) — Rust + Postgres + S3
4. `crates/nova-lsp` — LSP server (use `tower-lsp`)
5. `tools/vscode-nova` — VSCode extension (TypeScript)
6. `crates/nova-doc` — doc gen (output static HTML)
7. REPL: `nova repl` (use `rustyline`, evaluate via JIT… actually skip, use AOT-then-load via `dlopen`)
8. Debugger: emit DWARF debug info from LLVM/Cranelift; ensure VSCode debugger works via DAP

**Gate:** External user can `cargo install nova-cli`, init a project, write code with autocomplete, format, and publish a package.

---

### Phase 4 — Frontend / WASM (week 57–80)
**Goal:** Build SPA + SSR app end-to-end.

1. `crates/nova-codegen-wasm/` — WASM backend (wasm-encoder + walrus)
2. `crates/nova-bundler/` — bundle, tree-shake, code-split
3. `std/dom/` — low-level DOM bindings (auto-bindgen from web-sys descriptors)
4. `std/ui/` — signals, effects, memos, components
5. `std/ui/html/` — element builders
6. `std/ui/router/` — client router
7. `nova dev` — dev server with HMR
8. SSR: render to string in native runtime, hydrate in WASM
9. SSG: compile-time route enumeration

**Gate:** `examples/06-counter-spa` and `examples/07-fullstack-blog` work in browser, pass Lighthouse 95+.

---

### Phase 5 — Self-hosting (week 81–104)
**Goal:** Compiler written in Nova.

1. Port lexer to Nova
2. Port parser to Nova
3. Port AST + HIR + MIR to Nova
4. Port type checker to Nova
5. Backend bindings: bind LLVM via `unsafe` extern from Nova
6. Bootstrap: stage0 (Rust) → stage1 (Nova compiled by stage0) → stage2 (Nova compiled by stage1) → assert stage1 == stage2

**Gate:** CI builds stage2 == stage1 byte-for-byte (or close — may differ in non-deterministic codegen, document tolerances).

---

### Phase 6 — 1.0 Release (week 105–120)
1. Security audit (external)
2. Stability freeze — RFC for any breaking change
3. Documentation 100% (every public API has rustdoc-equivalent)
4. Tutorial site at `novalang.dev`
5. Interactive playground (compile in browser via WASM-compiled Nova compiler)
6. Submit to TechEmpower benchmarks
7. Launch posts: HN, Lobsters, Reddit r/programming, dev.to, Twitter

---

## 4. Files in This Spec Bundle

This master file references these companions. Read them in order during implementation:

| File | Purpose | When |
|---|---|---|
| `00-MASTER-SPEC.md` | This file | Always |
| `10-LEXER.md` | Token spec, lexer rules | Phase 0 |
| `11-PARSER.md` | Grammar, parser approach | Phase 0 |
| `12-TYPESYSTEM.md` | Type rules, inference algorithm | Phase 1 |
| `13-RUNTIME.md` | GC, async, FFI | Phase 1–2 |
| `14-CODEGEN.md` | LLVM/Cranelift/WASM backends | Phase 1, 4 |
| `20-STDLIB.md` | Stdlib API per module | Phase 2 |
| `30-FRONTEND.md` | UI / signals / WASM specifics | Phase 4 |
| `40-TOOLING.md` | CLI, formatter, LSP, package mgr | Phase 3 |
| `50-TESTING.md` | Test strategy, fixtures, CI | All |
| `60-EXAMPLES.md` | Reference example programs | All |

---

## 5. Conventions Claude Code Must Follow

### 5.1 Code style
- Rust: `cargo fmt`, `cargo clippy --all-targets --all-features -- -D warnings` must pass
- All public Rust items have rustdoc
- Errors implement `std::error::Error` + `thiserror` derive
- No `unwrap()` outside tests; use `expect("reason")` or proper error propagation
- All async fn return concrete `Future` types (no `async-trait` unless necessary)

### 5.2 Testing
- Every new module ships with unit tests in `#[cfg(test)] mod tests`
- Snapshot tests via `insta` for parser, type errors, formatter output
- Integration tests in `tests/` use `assert_cmd` to run `nova` binary
- Property tests via `proptest` for parser, lexer, JSON
- Fuzz targets in `fuzz/` for parser, lexer, JSON, regex
  **Amended 2026-10-06 (branch `phase-2-closeout`):** there is no `fuzz/`
  directory; `docs/adr/0025-phase-2-boundary.md` maps fuzz targets to Phase 6.

### 5.3 Errors (user-facing)
- Style: Elm/Rust quality. Every error has:
  - Code (e.g. `E0042`)
  - Title (one line)
  - Source span with caret pointer
  - Explanation paragraph
  - Suggestion / fix-it (if applicable)
  - Link to docs
- Implement via `nova-diagnostics` crate using `codespan-reporting` + custom renderer

### 5.4 Commits
- Conventional commits: `feat:`, `fix:`, `chore:`, `docs:`, `refactor:`, `test:`
- One logical change per commit
- Commit body explains WHY

### 5.5 Documentation order in code
For every module/file:
1. Module-level rustdoc explaining purpose
2. Public types
3. Public functions
4. Private impl
5. Tests at bottom

### 5.6 Don't do these
- Don't add dependencies without justification in commit message
- Don't write benchmarks before correctness tests
- Don't optimize before profiling
- Don't add features outside the current phase's gate criteria
- Don't change locked decisions in Section 1 — open an ADR instead

---

## 6. Bootstrap Dependencies (Rust crates, FINAL list)

Add these to workspace `Cargo.toml`. Versions current as of 2026 — verify and pin.

```toml
[workspace]
members = ["crates/*"]
resolver = "2"

[workspace.package]
edition = "2021"
rust-version = "1.78"
license = "MIT OR Apache-2.0"
repository = "https://github.com/novalang/nova"

[workspace.dependencies]
# Parsing & errors
chumsky = "0.10"
ariadne = "0.4"
codespan-reporting = "0.11"
logos = "0.14"           # alternative lexer if chumsky lex too slow
thiserror = "1.0"
anyhow = "1.0"

# Compiler infra
salsa = "0.18"           # incremental computation
indexmap = "2"
smol_str = "0.2"
rustc-hash = "1.1"

# Codegen
inkwell = { version = "0.4", features = ["llvm17-0"] }
cranelift = "0.105"
cranelift-module = "0.105"
cranelift-jit = "0.105"
cranelift-object = "0.105"
wasm-encoder = "0.215"
walrus = "0.22"

# Runtime
tokio = { version = "1", features = ["full"] }
httparse = "1.10"        # std/http parsing; hyper's own runtime is unavailable here, see docs/adr/0019
ring = "0.17"

# Tooling
tower-lsp = "0.20"
clap = { version = "4", features = ["derive"] }
rustyline = "14"

# Testing
insta = "1"
proptest = "1"
assert_cmd = "2"
criterion = "0.5"

# Misc
serde = { version = "1", features = ["derive"] }
serde_json = "1"
toml = "0.8"
tracing = "0.1"
tracing-subscriber = "0.3"
```

**Amended 2026-10-06 (branch `phase-2-closeout`): eight crates in this list
are not in `Cargo.lock`, and no crate depends on them.**
- `chumsky`: the parser is hand-written;
  `docs/adr/0025-phase-2-boundary.md` decides it stays.
- `salsa`: ADR 0025 maps it to Phase 3.
- `inkwell`: the LLVM backend emits textual IR and calls `clang` or `llc`.
- `tokio`: `std/task` runs on Nova's own single-threaded executor
  (`docs/adr/0009-async-execution-model.md`).
- `wasm-encoder` and `walrus`: for Phase 4's WASM backend, position 1.
- `tower-lsp` and `rustyline`: for Phase 3's language server and REPL,
  positions 4 and 7.

The workspace `Cargo.toml` also declares `chumsky = "0.9"` and
`rustyline = "14"`, which no crate uses.

---

## 7. Definition of Done (per phase)

A phase is DONE when:
1. All listed crates compile + pass CI
2. All gate criteria met with reproducible commands
3. Documentation written for new public surface
4. CHANGELOG.md updated
5. ADR written for any decision deviating from this spec
6. Tag a milestone release: `v0.{phase}.0`

---

## 8. What Claude Code Should Do First

When starting fresh:

```
1. Read 00-MASTER-SPEC.md (this file) end to end
2. Read 10-LEXER.md
3. Create the folder structure in Section 2
4. Create root files (Cargo.toml, rust-toolchain.toml, README, LICENSEs, .gitignore)
5. Create empty crate skeletons for all crates listed (lib.rs with module-level doc only)
6. Set up CI workflow (.github/workflows/ci.yml)
7. Implement nova-diagnostics first (everything else depends on it)
8. Implement nova-lexer following 10-LEXER.md
9. Run tests, commit
10. Move to nova-parser following 11-PARSER.md
```

Do not ask the user for approval between steps. Commit frequently. If blocked, write a SCRATCHPAD.md note and continue with the next independent task.

---

## 9. Recorded Drift Against `examples/` (added 2026-09-01, branch `std-http-parsing`)

Not a locked decision and not a rewrite of Section 2 above, which stays as
originally written — two facts about the tree noticed while building
`std/http`, measured against `examples/` rather than recalled, and recorded
here because neither is this increment's to fix.

**The third example's number and name have drifted from Section 2's tree.**
Section 2 above names `examples/03-http-server/`, and
`nova-spec/60-EXAMPLES.md` §3 names it too — the drift is in **two** spec
files, not one. `examples/` on disk holds `03-producer-consumer` instead.
Neither file is corrected by this note; a reader who needs the current
mapping should read `examples/` itself rather than either spec's tree.

**No example on disk has the README `60-EXAMPLES.md` §9 requires.** That
section's per-example template applies to every entry, and none of
`examples/01-hello-world`, `examples/02-fibonacci` or
`examples/03-producer-consumer` has a `README.md` at all — checked directly
(`ls examples/*/README.md` matches nothing), not assumed from one example
and generalised to the rest.

[Amended 2026-09-10, branch `examples-05-json-api`: one example on disk now has
that README. `examples/05-json-api/README.md` follows
`60-EXAMPLES.md` section 9's template. **The population changed, not the
check** — the paragraph above was measured against every entry under
`examples/` at its own date and was right about each of them; a fourth entry
was added, and `01-hello-world`, `02-fibonacci` and `03-producer-consumer`
still have no `README.md`. Nothing here brought them into line. The durable
check is `ls examples/*/README.md` against `ls -d examples/*/`, not this note.
Also worth knowing before sweeping for this claim: the version of it in
`60-EXAMPLES.md` section 9 wraps across a line break, so a line-oriented `grep`
for the phrase missed it there entirely — until that section's own 2026-09-10
amendment quoted the phrase on one line and thereby changed what such a `grep`
finds. Flatten before concluding a wrapped claim is absent, and re-measure
rather than trusting a count written into the file it counts.]

[Amended 2026-09-10, branch `examples-05-json-api`: the tree drift recorded
above is not narrowed by that addition. Section 2's tree above and
`60-EXAMPLES.md` section 3 both still name `examples/03-http-server/` while
`examples/` holds `03-producer-consumer`, and adding `05-json-api` touched
neither. Section 2's tree does name `05-json-api/`, so that entry of it is no
longer ahead of the disk.]

[Amended 2026-10-04, branch `examples-03-http-server`: `examples/03-http-server/`
now exists, beside `examples/03-producer-consumer/`, so Section 2's
`03-http-server/` entry is no longer ahead of the disk either. Slot 03 now
holds two entries; nothing was renumbered, by the user's decision of
2026-10-04. A second example now has the §9 README,
`examples/03-http-server/README.md`; `01-hello-world`, `02-fibonacci` and
`03-producer-consumer` still have none. The durable checks are still
`ls -d examples/*/` and `ls examples/*/README.md`, not this note.]

[Amended 2026-10-05, branch `examples-04-todo-cli`: `examples/04-todo-cli/` now
exists, so Section 2's `04-todo-cli/` entry is no longer ahead of the disk
either. A third example now has the §9 README, `examples/04-todo-cli/README.md`.
The durable checks are unchanged.]
