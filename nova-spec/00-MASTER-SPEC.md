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
section's own Phase 2 gate (line 245) reads "`examples/05-json-api` serves
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

### Phase 0 — Foundation (week 1–4)
**Goal:** Repo skeleton + lexer + parser for a minimal subset.

Files to create in order:
1. Workspace `Cargo.toml`, `rust-toolchain.toml`, root README, LICENSE files
2. CI: `.github/workflows/ci.yml` (cargo test, fmt, clippy on PR)
3. `crates/nova-diagnostics/` — error reporting infrastructure (use `codespan-reporting`)
4. `crates/nova-lexer/` — see [10-LEXER.md]
5. `crates/nova-ast/` — AST node definitions
6. `crates/nova-parser/` — see [11-PARSER.md], use **chumsky** (Pratt-style for expressions)
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
