# ADR 0025 — Phase 2's boundary

## Status

Accepted (2026-10-06). Branch `phase-2-closeout`
(`docs/superpowers/specs/2026-10-06-phase-2-closeout-design.md`).

## Context

`nova-spec/00-MASTER-SPEC.md` §7 calls a phase done when:
1. its crates compile and pass CI;
2. its gate criteria are met with reproducible commands;
3. its new public surface is documented;
4. `CHANGELOG.md` is updated;
5. an ADR records each decision that deviates from the spec;
6. a `v0.{phase}.0` milestone is tagged.

Phase 2, "Standard Library Core", makes its promises in three places:
- **the master spec's §3:** the Phase 2 list of thirteen numbered entries,
  cited below as "position N", and the Phase 2 gate;
- **`docs/phase-2-plan.md`:** sub-phases 2.0 to 2.5, with a gate for each of
  2.0 to 2.4; its §4, on how a sub-phase closes; and its §5's cross-cutting
  items;
- **`nova-spec/60-EXAMPLES.md`:** its §3, §4 and §5, which label
  `03-http-server`, `04-todo-cli` and `05-json-api` as Phase 2 gates.

`nova-spec/20-STDLIB.md` §1 names more, but it is headed "Module Index
(v1.0)": it lists v1.0's standard library, of which Phase 2 builds a part.
The inventory below includes the index's items that none of the three names,
marked "index", so that what `v0.2.0` leaves out is explicit too.

Earlier ADRs already narrowed Phase 2:
- 0003: the module model, which defers `import … as`, qualified `m::name`
  paths, nested module directories and re-exports;
- 0009: `std/task` runs on Nova's own single-threaded executor, not Tokio;
- 0014: the standard library's build order;
- 0015: `std/fmt`'s scope;
- 0016 and 0017: `std/sync`;
- 0018 and 0019: 2.4 split into three increments, and `std/http` built over
  `httparse` rather than hyper.

An assessment on 2026-10-05 found Definition of Done items 1, 2 and 4 met:
- item 3 lacked 20-STDLIB sections for `std/strings` and `std/bytes`;
- item 5 lacked records for the deviations below;
- item 6, the tag, was not done.

It also found that the collector had never run off Windows, which ADR 0024
fixed for glibc Linux and macOS.

On 2026-10-06 the user decided:
- **Record only.** Closing Phase 2 builds nothing. Each gap is covered by an
  existing record or recorded here, and `v0.2.0` ships what exists.
- **An unscheduled backlog.** The deferred standard-library pieces get no
  phase and no date; each is built when a program needs it. Tooling goes
  where it fits: `salsa` to Phase 3, which builds the language server, and
  fuzzing to Phase 6, which holds the security audit.
- **One ADR,** this one, built around an inventory table.

## Decision

### The inventory

"Position N" is the master spec's Phase 2 list, and "index" is 20-STDLIB §1.
"2.N" is a sub-phase of `docs/phase-2-plan.md`, and "plan §N" is a section
of it. "60-EXAMPLES §N" is a section of `nova-spec/60-EXAMPLES.md`. A bare
"§N" is a section of `nova-spec/20-STDLIB.md`.

| Promise | Source | Status | Record |
|---|---|---|---|
| `std/core` | position 1 | shipped | §2 |
| `std/fmt`, `std/io` | position 2 | shipped | §3, §4; ADR 0015 |
| `std/collections`: `Vec`, `Map`, `Set`, iterators | position 3; 2.2 | shipped | §2 (`Iterator`), §12 |
| `std/collections`: `Queue`, `Deque`, `Vec::with_capacity` | §12's code block; `Queue` also index and 2.2 | not built | §12; backlog |
| `std/strings` | position 4 | shipped | §18 |
| `std/fs` | position 5 | shipped | §5; ADR 0012 |
| `std/time`, `std/log` | position 6 | shipped | §9, §10 |
| `std/task` | position 7, "wrap Tokio" | shipped, on Nova's own single-threaded executor | ADR 0009 |
| `std/task`: `spawn_blocking`, `JoinHandle::cancel` | §13's code block | not built | §13; backlog |
| `std/sync`: `Mutex`, a bounded `channel` | position 8 | shipped | ADRs 0016, 0017 |
| `std/sync`: atomics, `RwLock` | position 8 (atomics); index; 2.3 | not built | §13; ADR 0016; backlog |
| `std/sync`: a oneshot channel | 2.3 | not built | backlog |
| `std/net`: TCP, client and server | position 9 | shipped | §16; ADR 0013 |
| `std/net`: UDP, Unix sockets | position 9 (UDP); index | not built | §16; backlog |
| `std/http`: the server, over `httparse` | position 10 | shipped | §6; ADR 0019 |
| `std/http`: the client | position 10; index | not built | §6; backlog |
| `std/http`: a router beyond exact paths and `GET` | §6's code block; 2.4's notes | not built | §6; backlog |
| `std/http`: HTTPS, HTTP/2, chunked transfer-encoding | §6 | not in v1, by §6's own statement | §6 |
| `std/json` | position 11 | shipped | §7; ADR 0018 |
| `std/crypto`: SHA-256, SHA-512, HMAC-SHA-256, randomness | position 12 | shipped | §8 |
| `std/crypto`: AEAD | index; §8's code block | not built | §8; ADR 0018; backlog |
| `std/crypto`: BLAKE3 | §8's code block | refused by the `ring` backing | §8; backlog, with another backing |
| `std/test` and `nova test` | position 13 | shipped | §11 |
| `std/bytes` | index | shipped | §19 |
| `std/process`: `args`, `exit` | index; 2.5, optional | shipped | §17; ADR 0023 |
| `std/process`: `spawn`, `env` | index; 2.5, optional | not built | §17; backlog |
| `std/regex` | index; 2.5, optional | not built | backlog |
| 2.0: the module system: `import` with glob and `{…}` lists, `pub`, multi-file programs | 2.0 | shipped | ADR 0003 |
| 2.0: `import … as` | 2.0 | not built | ADR 0003, which also defers qualified `m::name` paths, nested module directories and re-exports; backlog |
| 2.0: `extern` blocks and FFI intrinsics | 2.0 | shipped; under `nova run` on Linux an `extern` program fails with `E0902`, and that test is ignored (issue #3) | — |
| 2.0: method-level generics, `where` clauses | 2.0 | shipped | — |
| 2.0: the prelude | 2.0 | shipped | ADRs 0003, 0004 |
| 2.1: the `?` operator, "if in scope" | 2.1 | not built: it parses, and type-checking rejects it as unsupported | backlog |
| 2.2: growable memory through a runtime realloc-style intrinsic | 2.2 | not built as planned: collections grow by allocating a new array and copying | the 2.2a design |
| Gate: `examples/05-json-api` serves 10k+ req/sec on benchmark hardware, with the methodology in `docs/benchmarks/` | the master spec's Phase 2 gate; 60-EXAMPLES §5 | met, on this development host | ADR 0021; "benchmark hardware" decided here |
| Gate: `examples/03-http-server` | 60-EXAMPLES §3 | met: its end-to-end tests run on all three CI operating systems | ADR 0022 |
| Gate: `examples/04-todo-cli` | 60-EXAMPLES §4 | met: its end-to-end tests run on all three CI operating systems | ADR 0023 |
| 2.0's gate | 2.0 | met in parts, under Cranelift only | LLVM parity, below; backlog |
| 2.1's gate | 2.1 | met as the 2.1 design set it, under Cranelift | the 2.1 design's §1 |
| 2.2's gate | 2.2 | GC stress met; no collections benchmark | backlog |
| 2.3's gate | 2.3 | met by `examples/03-producer-consumer` | — |
| 2.4's gate | 2.4 | met by `examples/05-json-api` | ADR 0021 |
| 2.5: `std/test` and `nova test` | 2.5 | shipped | §11 |
| 2.5: migrating the e2e fixtures to `nova test` | 2.5 | not done | backlog |
| 2.5: chumsky 0.10 | 2.5; the master spec's Phase 0 position 6, and the master spec's §6 | not adopted | decided here |
| 2.5: `salsa` scaffolding | 2.5; the master spec's §6 | not built | Phase 3 |
| 2.5: `fuzz/` targets | 2.5; the master spec's §5.2; `nova-spec/50-TESTING.md` §1.7 | not built | Phase 6 |
| 2.5: GC stack bounds off Windows | 2.5 | done for glibc Linux and macOS | ADR 0024 |
| Every module's programs under both backends and `NOVA_GC_STRESS` | plan §5 | none under LLVM; per-module stress coverage not assessed | LLVM parity, below; backlog |
| Both backends kept in lockstep | plan §5 | parity unverified | LLVM parity, below; backlog |
| Sub-phases "independently gated, reviewed, and tagged" | plan §4 | 2.0 to 2.4 gated; 2.5 had no gate; none tagged | decided here |

**Status words:**
- *Shipped:* a `v0.2.0` program can rely on it, within the limits its record
  states.
- *Not built:* nothing implements it.
- *Backlog*, *Phase 3* and *Phase 6:* where a deferred item goes.
- *Decided here:* a section below makes the decision.

### Deviations no earlier record decides

Each section says what was promised and where, and what exists. Every
deferred item waits for the backlog's one reason: it is built when a program
needs it. Where a passage already says the item is unbuilt, the section cites
it. Three sections make a decision.

#### UDP and Unix sockets

Position 9 names UDP, and the index names "TCP/UDP/Unix sockets". `std/net`
ships TCP, both client and server (§16; ADR 0013). §16 records that UDP and
Unix sockets remain unbuilt. Backlog.

#### The HTTP client

Position 10 says "server first, then client", and the index says "HTTP
client + server". §6's code block sketches the client: `get`, `post`, and a
`Response::json` that decodes. The server ships (ADR 0019), with `Server`
built on it. §6 records two things:
- the client does not ship;
- `Response::json` became a constructor, and one type cannot carry both
  meanings of that name, so building the client means renaming one of them.

Backlog.

#### A router beyond `Server`'s exact paths and `GET`

§6's code block specifies more than ships:
- `post`, `put`, `delete`, `route` and `use_middleware`;
- path parameters;
- the `Handler` and `Middleware` aliases.

2.4's notes track the router. `Server` ships with `new`, `get`, `dispatch`
and `listen`: `GET` only, exact paths, and synchronous handlers. An
`async fn` type does not parse yet (`P0001`), so neither alias can be written
(§6's 2026-10-04 note). Backlog.

#### Atomics and `RwLock`

Position 8 names atomics, and the index and 2.3 name both. `std/sync` ships a
`Mutex` and a bounded `channel` (ADRs 0016 and 0017). §13 records that
neither atomics nor `RwLock` has "a signature, a semantic, or a section
anywhere in `nova-spec/`", so building either starts with a design. Backlog.

#### `Queue`, `Deque` and `Vec::with_capacity`

§12's code block declares all three, and the index and 2.2 name `Queue`. None
is implemented, as §12 records. `std/sync`'s channel carries its own ring
buffer because `std/collections` "has no `Queue` to borrow". Backlog.

#### AEAD

The index promises "hashing, AEAD, random", and §8's code block sketches an
`Aead` API. `std/crypto` ships SHA-256, SHA-512, HMAC-SHA-256 and
randomness. §8 records AEAD as unstarted, as ADR 0018 does. Backlog.

#### BLAKE3

§8's code block declares `blake3`. §8 records it as refused by the backing §8
itself names: `ring` does not implement BLAKE3. Backlog, with a backing other
than `ring`.

#### LLVM parity

Plan §5 says both backends "must stay in lockstep", and that every module's
programs run under both. 2.0's gate asks for a program that "compiles and
runs under both backends". What exists:
- **The LLVM backend,** `crates/nova-codegen-llvm`, emits LLVM IR as text.
  `nova-driver`'s `link.rs` passes it to `clang`, or to `llc` when `clang` is
  missing. The backend's own tests check the IR text.
- **One release test runs anything.**
  `release_builds_and_runs_when_clang_available` builds and runs hello world.
  When `clang` is not on `PATH`, it returns early and passes, having run
  nothing. The other two `--release` tests hide the toolchain on purpose and
  check the IR it leaves behind.
- **Where a sub-phase record says a program runs "under both backends", it
  means two Cranelift paths.** Those records name `nova run` and
  `nova build`, and both are Cranelift; only `nova build --release` reaches
  LLVM. The module-system commit `8c37c79` and the 2.1 design's gate both say
  so. Elsewhere the phrase can mean the two codegen crates: the 2.2a plan
  emits `ArrayAlloc` in both.
- **2.0's features are tested separately.** `tests/runtime/modules/` is
  multi-file, with `import` and a generic function. `method_generics.nova`,
  `where_clauses.nova` and `extern_ffi.nova` test the other three features,
  each in its own program. No test runs any of them under LLVM.

So whether the release backend compiles Phase 2's programs correctly is
unverified, and the one test that would show it can pass without running.
Backlog: parity, including a release test that cannot pass vacuously.

#### The collections benchmark

2.2's gate ends with "benchmark basic ops". `collections_under_gc_stress`
meets its first half; nothing benchmarks collection operations. Backlog.

#### The fixture migration

2.5 says to "migrate the compiler's e2e fixtures to `nova test` where
sensible". `nova test` ships (§11), and the fixtures still run from
`crates/nova-cli/tests/run_tests.rs`. Backlog.

#### `salsa`

2.5 asks for "`salsa` scaffolding", and the master spec's §6 lists the crate.
No crate depends on it. Phase 3: the language server is where incremental
queries pay off (`nova-spec/40-TOOLING.md`).

#### Fuzz targets

2.5 asks for `fuzz/` targets for the lexer and parser, and the master spec's
§5.2 and `nova-spec/50-TESTING.md` §1.7 list more. There is no `fuzz/`
directory. Phase 6, beside its security audit.

#### chumsky: decided, the hand-written parser stays

2.5 asks for "chumsky 0.10", and the master spec's Phase 0 position 6 says to
use it. The master spec's §6 and `nova-spec/11-PARSER.md` §1 name it too.

The parser has been hand-written recursive descent since Phase 0
(`crates/nova-parser/src/grammar.rs`). Operator precedence comes from
explicit layering, and after an error the parser skips to the next item or
statement boundary. No crate has ever depended on chumsky. The workspace
`Cargo.toml` still declares `chumsky = "0.9"`, unused, a different version
from the 0.10 the specs name.

**Decision:** the hand-written parser stays, and chumsky is not adopted.
Adopting it would rewrite a working parser with no defect to fix. The unused
declaration stays until a manifest change has its own reason to happen; this
close-out changes no manifest.

#### Per-sub-phase tags: decided, the four alpha tags stand in

Plan §4's heading says each sub-phase is "independently gated, reviewed, and
tagged". 2.0 to 2.4 each had a gate, which the inventory reports on, and 2.5
had none; no sub-phase was tagged. Four pre-release tags mark Phase 2's
progress instead, `v0.2.0-alpha.1` to `v0.2.0-alpha.4`.

**Decision:** those four stand in for per-sub-phase tags, and none is added
after the fact.

#### "Benchmark hardware": decided, this development host

The master spec's Phase 2 gate reads "`examples/05-json-api` serves 10k+
req/sec on benchmark hardware", and the spec never defines benchmark
hardware. The gate was met under ADR 0021's procedure on this development
host, which runs Windows with the load generator on the same machine. The
methodology is in `docs/benchmarks/README.md`, and the figures are in
`examples/05-json-api/BENCHMARK.md`.

**Decision:** for Phase 2, benchmark hardware means that host. A later phase
that needs a stronger claim defines its hardware before it measures.

### Rows whose record already explains them

These rows have no section above:
- **`spawn_blocking` and `JoinHandle::cancel`:** §13 gives the reasons.
- **HTTPS, HTTP/2 and chunked transfer-encoding:** §6 puts them "Not in v1".
- **`std/process`'s `spawn` and `env`, and `std/regex`:** 2.5 lists those
  modules as optional, "as the server example demands", and §17 records that
  `spawn` and `env` do not exist.
- **The oneshot channel:** 2.3 names it beside `mpsc`, and its row is its
  record.
- **`import … as`:** ADR 0003 deferred it, with qualified `m::name` paths,
  nested module directories and re-exports.
- **The `?` operator:** 2.1 promised it "if in scope", and the 2.1 design left
  it out; its row is its record.
- **Growable memory:** the 2.2a design records why no realloc-style intrinsic
  was needed: a collection grows by allocating a bigger array and copying
  into it.
- **`extern` under the JIT on Linux:** issue #3 tracks the `E0902`; under
  `nova build`, `extern` programs run on every operating system.
- **2.1's gate.** The 2.1 design narrowed the sub-phase (its §2) and set its
  own gate (its §1): a program that round-trips `Option` and `Result` and
  prints a custom `Display`, under `nova run` and `nova build`. That gate is
  met. The plan's "rewrite the Phase-1 examples" is not part of it, and
  nothing records it as done. This ADR records the drop, and the design's
  gate stands.

### The backlog

The backlog is unscheduled. Each item is built when a program needs it, and
the list promises no date, no phase and no order.
- The language: `import … as`, with ADR 0003's other deferred module
  features; the `?` operator.
- `std/net`: UDP; Unix sockets.
- `std/http`: the client; a router beyond exact paths and `GET`.
- `std/sync`: atomics; `RwLock`; a oneshot channel.
- `std/collections`: `Queue`; `Deque`; `Vec::with_capacity`.
- `std/task`: `spawn_blocking`; `JoinHandle::cancel`.
- `std/crypto`: AEAD; BLAKE3, with a backing other than `ring`.
- `std/process`: `spawn`; `env`.
- `std/regex`.
- LLVM parity with Cranelift, and a release test that cannot pass without
  running.
- A benchmark of collection operations.
- Migrating the compiler's e2e fixtures to `nova test`.

Mapped to later phases:
- Phase 3 (Tooling): `salsa`.
- Phase 6 (1.0 Release, with its security audit): fuzz targets.

### Found outside this boundary

Met while closing Phase 2; recorded, not decided:
- ADR 0001 and the master spec's §1.1 say there is no JIT, while `nova run`
  JIT-compiles with Cranelift.
- Phase 1's LLVM backend emits textual IR rather than using `inkwell`, which
  the master spec's Phase 1 position 9 and `nova-spec/14-CODEGEN.md` §2.2
  name.

## Consequences

- With 20-STDLIB §18 and §19 (Definition of Done item 3) and this ADR
  (item 5), every item of the master spec's §7 Definition of Done is met but
  the tag. Item 2's gate criteria are the master spec's Phase 2 gate and
  60-EXAMPLES's three Phase 2 gates, and all of them are met. The plan's
  sub-phase gates are its own, and the inventory reports each one, 2.0's as
  met only in parts. Tagging `v0.2.0` completes Phase 2 within this boundary.
- What a `v0.2.0` program can rely on is the "shipped" rows, within the
  limits their records state.
- The backlog promises nothing.
- Dated notes point here from:
  - the master spec's §3, Phase 0 position 6, §5.2 and §6;
  - `nova-spec/10-LEXER.md`, `11-PARSER.md`, `13-RUNTIME.md`,
    `14-CODEGEN.md` §2.2 and `50-TESTING.md` §1.7;
  - `nova-spec/20-STDLIB.md`'s §1 and its gate notes in §7;
  - `docs/phase-2-plan.md`.
- The Phase 0 guides (`ARCHITECTURE.md`, `agent.md` and `skill.md`) and the
  parser's rustdoc are corrected in place instead, because they describe the
  code as it is now.
- The workspace `Cargo.toml` still declares `chumsky = "0.9"`,
  `rustyline = "14"` and `serde_json = "1"`, which no crate uses.

## References

- Spec: `docs/superpowers/specs/2026-10-06-phase-2-closeout-design.md`
- `nova-spec/00-MASTER-SPEC.md` §3 and §7; `docs/phase-2-plan.md`;
  `nova-spec/60-EXAMPLES.md` §3 to §5
- `nova-spec/20-STDLIB.md` §1 to §13 and §16 to §19
- `docs/superpowers/specs/2026-07-25-phase-2-1-std-core-design.md` and
  `docs/superpowers/specs/2026-07-26-phase-2-2a-collections-design.md`
- ADRs 0001, 0003, 0004, 0009, 0012, 0013, 0014, 0015, 0016, 0017, 0018,
  0019, 0021, 0022, 0023 and 0024
