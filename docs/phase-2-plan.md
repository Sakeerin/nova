# Phase 2 Plan — Standard Library Core

> Status: **draft** (2026-07-23). Derived from `nova-spec/00-MASTER-SPEC.md` §Phase 2
> and `nova-spec/20-STDLIB.md` / `13-RUNTIME.md`. Supersedes nothing yet.

## 1. Goal (from the spec)

Phase 2 = "Standard Library Core" (spec weeks 25–40): **server-side apps work**,
benchmarked against Bun. The stdlib is **written in Nova with FFI down to the
`nova-runtime` Rust crate** (which wraps Tokio, hyper, ring, etc. — we do not
re-implement them). Spec gate: `examples/05-json-api` serves **10k+ req/sec**.

The spec lists 13+ std modules to build in dependency order: `core`, `fmt`,
`io`, `collections`, `strings`, `fs`, `time`, `log`, `task` (async), `sync`,
`net`, `http`, `json`, `crypto`, `test`.

## 2. Reality check — language prerequisites Phase 1 deferred

The stdlib source (`module std.core`, `impl<T,E> Result<T,E>`, `map<U>(...)`,
`extern`…) exercises features Phase 1 explicitly rejects today. **These are hard
prerequisites and must land before any std module compiles:**

| Prerequisite | Current state | Needed for |
|---|---|---|
| **Module system** — multi-file, `module a.b`, `import`, cross-module resolution, `pub` visibility | single-file only; `module-qualified type paths` → E0900 | every `std/*` module |
| **`extern` / FFI** — `extern "C" { fn … }`, link config | extern blocks → E0900 (resolver) | binding `nova_rt_*` / Tokio / hyper |
| **Method-level generics** — `fn map<U>(self, …)` | `generic methods` → E0900 | `Option::map`, `Result::and_then`, iterators |
| **`where` clauses** | → E0900 | bounded generic std APIs |
| **Async / await** — `Future<T>` state machines, `.await` | `async functions` → E0900 | `std/task`, `std/net`, `std/http` |
| **Dynamic collections** — growable memory (`Vec`) | arrays are fixed-length heap blocks | `std/collections` |
| **Prelude** — auto-import `Option`/`Result`/core traits | none | ergonomic std |

There is also known Phase-1 drift to reconcile: **chumsky 0.9 → 0.10**, add
**`salsa`** for incremental compilation, write **`fuzz/`** targets, and add
**precise GC stack bounds** for non-Windows.

## 3. Key decisions (recommend now, confirm before building)

These are genuine forks that shape the whole phase — like the LLVM-backend
decision in Phase 1. Recommendations in **bold**.

1. **Async execution model.** The spec says "compile to a state machine like
   Rust." That is a very large compiler feature (async lowering, self-referential
   state, pinning). Options:
   - (a) **Full state-machine lowering** (spec-faithful, portable, no OS threads per task).
   - (b) **Stackful coroutines / thread-per-task over Tokio** (much simpler to
     ship; `await` blocks a runtime worker). **Recommended for Phase 2.0**, revisit
     (a) in Phase 2.x once collections/net are proven. This is a spec deviation →
     needs an ADR.
2. **Stdlib distribution / compile model.** How does `std/` reach a user program?
   Options: compile std sources alongside the user program every build; or a
   precompiled std artifact; or inject a prelude + link std objects. **Recommended:**
   compile-with-program initially (simplest, monomorphization already whole-program),
   optimize later.
3. **FFI scope for Phase 2.** Full C-FFI with `nova.toml` link config, or a
   **curated `nova_rt_*` intrinsic surface** that std binds to (runtime does the
   real work in Rust). **Recommended:** curated intrinsics first (enough for all
   std modules); general C-FFI + `nova.toml` linking deferred to Phase 3 (tooling).
4. **Benchmark gate realism.** 10k req/sec HTTP is an ambitious end gate. **Recommended:**
   keep it as the *phase* gate but add earlier, cheaper gates per sub-phase (below)
   so progress is verifiable long before the server exists. Document methodology in
   `docs/benchmarks/`.

## 4. Sub-phases (each independently gated, reviewed, and tagged)

Ordered so every step is verifiable and unblocks the next. Each ends with the
established loop: implement → tests → clippy/fmt → commit → adversarial-review
workflow → fix findings.

### 2.0 — Language completeness (compiler, no std yet)
The foundation. No stdlib until this is solid.
- **Module system**: `module`, `import` (+ `as`, `{…}` lists), a module graph in
  `nova-resolver`, cross-module name/def resolution, `pub` visibility enforcement,
  multi-file driver input.
- **`extern` blocks + curated FFI intrinsics**: typeck/codegen for `extern "C"`
  declarations; both backends already emit C-ABI calls, so this is mostly
  front-end + a runtime symbol registry.
- **Method-level generics** and **`where` clauses** in typeck/mono (extends the
  existing generic machinery — bounds already verified at monomorphization).
- **Prelude** injection (auto-visible `Option`, `Result`, core traits).
- **Gate:** a multi-file Nova program using `import`, a generic method, a `where`
  bound, and an `extern` runtime call compiles and runs under both backends.

### 2.1 — `std/core` + `std/fmt` + `std/io`
- `Option<T>`, `Result<T,E>` with full method sets; core traits (`Eq`, `Ord`,
  `Display`, `Debug`, `Clone`, `Iterator`, `Hash`), `?`-style error propagation
  (if in scope), `panic`/`assert`.
- `Display`/`Debug` + `write`, `println`/`eprintln`/formatting; `io` handles,
  file/stdout/stderr abstractions.
- **Gate:** rewrite the Phase-1 examples to use `std/core` + `std/io`; a program
  round-trips `Option`/`Result` and custom `Display`.

### 2.2 — `std/collections` + `std/strings`
- Growable memory support (runtime `realloc`-style intrinsic; GC must track
  moved/resized blocks). `Vec`, `Map` (hash), `Set`, `Queue`. Iterators.
- Unicode-aware string ops (`std/strings`), building on the existing `NovaStr`.
- **Gate:** a program building/mutating `Vec`/`Map` under GC stress
  (`NOVA_GC_STRESS`) with correct output; benchmark basic ops.

### 2.3 — `std/task` (async) + `std/sync` + `std/time` + `std/log`
- Async model per decision (1). `spawn`, `.await`, `block_on` in `main`;
  `mpsc`/oneshot channels; `Mutex`/`RwLock`/atomics over the Rust runtime.
- `Instant`/`Duration`/`sleep`; structured logging.
- **Gate:** a concurrent producer/consumer example with channels and timers
  produces deterministic output.

### 2.4 — `std/net` + `std/http` + `std/json`
- TCP/UDP over the Tokio wrapper; HTTP server (hyper internals) then client;
  a codec-based, type-safe JSON parser/serializer in Nova.
- **Gate:** `examples/05-json-api` responds correctly to requests (functional
  first), then the **10k+ req/sec** benchmark → `docs/benchmarks/`.

### 2.5 — `std/test` (+ `nova test`) and hardening
- Test runner; migrate the compiler's e2e fixtures to `nova test` where sensible.
- Fold in the drift cleanup: chumsky 0.10, `salsa` scaffolding, `fuzz/` targets
  for lexer/parser, non-Windows GC stack bounds.
- Optional: `std/crypto` (ring), `std/fs`, `std/process`, `std/regex` as the
  server example demands.

## 5. Cross-cutting

- **Testing:** every module gets Nova programs run under both backends and under
  `NOVA_GC_STRESS`; adversarial-review workflow after each substantial feature
  (it found real bugs in every Phase-1 feature except the GC).
- **Backends:** both Cranelift (debug) and LLVM-IR (release) must stay in lockstep;
  new MIR constructs (async state machines, FFI calls) need both.
- **GC:** dynamic collections and async state machines add new heap shapes and
  long-lived roots — re-validate the conservative collector as those land, and
  finish non-Windows stack bounds so CI on Linux exercises real collection.

## 6. Top risks

1. **Async** is the single largest feature; the model choice (decision 1)
   dominates the phase's timeline. De-risk with the pragmatic model first.
2. **The module system** touches resolver, typeck, driver, and monomorphization
   (which is currently whole-program from a single file) — sequence it first and
   review hard.
3. **The benchmark gate** depends on codegen quality + runtime efficiency; treat
   the functional server as the real milestone and the throughput number as a
   stretch to iterate toward.

## 7. Suggested first step

Start **2.0 → module system**: design the module graph + `import` resolution in
`nova-resolver`, since everything else (std source, `pub`, prelude) hangs off it.
Recommend an ADR for the module/compile model (decision 2) and one for the async
model (decision 1) before coding those.
