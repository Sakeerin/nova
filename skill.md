# Nova — Skills Reference

> Knowledge areas and technical skills required to work on different parts of the Nova compiler and toolchain.
> Use this as a map: find the area you're working on, check what you need to know.

---

## 1. Core Language: Rust

Every crate in this repo is Rust. Minimum competency for any contribution:

| Skill | Where it shows up |
|---|---|
| Ownership, borrowing, lifetimes | Everywhere — especially AST nodes and IR types |
| Traits and generics | Type system crates, codegen traits |
| Error handling (`Result`, `?`, `thiserror`) | All crates — `unwrap()` is banned outside tests |
| Pattern matching | Parser, type-checker, codegen |
| Iterators and closures | Transformations in all pipeline stages |
| `cargo` workspace layout | Multi-crate project structure |
| `#[cfg(test)]` and `insta` | Snapshot testing in every crate |
| `tracing` / structured logging | Diagnostics and CLI output |

---

## 2. Lexer (`nova-lexer`)

**Spec:** `nova-spec/10-LEXER.md`

| Skill | Notes |
|---|---|
| `logos` crate | Token definition via derive macros |
| Regex fundamentals | Token pattern matching |
| Unicode handling | Identifiers, string content |
| Span tracking | Every token carries a `Span` (byte offset range) |
| Error recovery | Emit an `Unknown` token; do not panic |

Key types: `Token`, `Spanned<T>`, `LexError`.

---

## 3. Parser (`nova-parser`)

**Spec:** `nova-spec/11-PARSER.md`

| Skill | Notes |
|---|---|
| `chumsky` 0.10 | Parser combinator library — read its guide first |
| Pratt parsing | Expression precedence (used for binary ops) |
| EBNF grammars | The spec provides an EBNF grammar to implement |
| Recursive descent | Top-down parsing strategy |
| Error recovery | `chumsky` supports recovery combinators — use them |
| AST design | Immutable node types in `nova-ast` |

Key types: all AST nodes in `nova-ast` (e.g., `Expr`, `Stmt`, `Item`, `TypeExpr`).

---

## 4. Type System (`nova-typeck`)

**Spec:** `nova-spec/12-TYPESYSTEM.md`

| Skill | Notes |
|---|---|
| Hindley-Milner type inference | Algorithm W / constraint solving |
| Unification | Solving type variable constraints |
| Traits (Rust-style) | Trait bounds, impl resolution |
| Monomorphization | Specializing generics at compile time |
| Sum types / ADTs | `enum` with payloads |
| Exhaustiveness checking | Match arm coverage for ADTs |
| `Option<T>` / `Result<T,E>` | No `null`, no exceptions |

---

## 5. IR Design (`nova-hir`, `nova-mir`)

| Skill | Notes |
|---|---|
| High-level IR concepts | Desugared, typed representation of the AST |
| 3-address code / SSA | MIR is a 3-address-style IR |
| Control flow graphs | Basic blocks and edges |
| Lowering passes | Transforming one IR level to the next |
| `salsa` incremental computation | Used for query-based compilation |
| `indexmap`, `rustc-hash` | Efficient maps for IR nodes |

---

## 6. Code Generation

### 6a. Cranelift (`nova-codegen-cranelift`) — debug backend
**Spec:** `nova-spec/14-CODEGEN.md`

| Skill | Notes |
|---|---|
| `cranelift` + `cranelift-module` | IR builder API |
| `cranelift-object` | Emit object files |
| Calling conventions | ABI for function calls |
| Cranelift IR types | `i32`, `i64`, `f64`, `ptr`, etc. |

### 6b. LLVM (`nova-codegen-llvm`) — release backend

| Skill | Notes |
|---|---|
| `inkwell` crate | Rust LLVM bindings |
| LLVM IR concepts | `Function`, `BasicBlock`, `Value`, `Builder` |
| Optimization passes | `PassManager` setup |
| LLVM types | `IntType`, `PointerType`, struct layout |

### 6c. WASM (`nova-codegen-wasm`) — browser backend

| Skill | Notes |
|---|---|
| `wasm-encoder` | Low-level WASM binary encoding |
| `walrus` | Higher-level WASM IR and transforms |
| WebAssembly spec | Modules, types, memory model, tables |
| JS interop | ABI for calling JS from WASM and vice versa |

---

## 7. Runtime (`nova-runtime`)

**Spec:** `nova-spec/13-RUNTIME.md`

| Skill | Notes |
|---|---|
| Garbage collection concepts | Mark-and-sweep, generational GC |
| `mmtk` crate (or `bdwgc` bindings) | GC implementation |
| `tokio` async runtime | Task scheduling, `async`/`await` |
| `unsafe` Rust | Allocator, GC roots, FFI |
| Panic handling | Unwinding vs. abort strategies |
| FFI (`extern "C"`) | Exposing runtime functions to compiled code |

---

## 8. Standard Library (`std/`)

**Spec:** `nova-spec/20-STDLIB.md`

Written in Nova (Phase 2+). Skills:

| Skill | Notes |
|---|---|
| Nova language itself | Eating our own dog food |
| HTTP / TCP / UDP | `std/http`, `std/net` — backed by `hyper` in runtime |
| JSON | Custom parser + trait-based codec |
| Cryptography | Wrap `ring` at the runtime layer |
| Async patterns | All I/O is async |

---

## 9. Tooling

### Formatter (`nova-fmt`)
**Spec:** `nova-spec/40-TOOLING.md`

| Skill | Notes |
|---|---|
| Pretty-printing algorithms | Wadler-Lindig or simple greedy |
| AST traversal | Walk `nova-ast` nodes |
| No config | Opinionated output only; `--check` mode for CI |

### LSP (`nova-lsp`)
**Spec:** `nova-spec/40-TOOLING.md`

| Skill | Notes |
|---|---|
| Language Server Protocol spec | Requests, notifications, capabilities |
| `tower-lsp` crate | Async LSP server framework |
| Incremental parsing | Re-parse only changed regions |
| `salsa` | Query-based incremental compilation |

### Package Manager (`nova-pm`)
**Spec:** `nova-spec/40-TOOLING.md`

| Skill | Notes |
|---|---|
| `nova.toml` format | TOML parsing via `toml` crate |
| Semver | Version resolution algorithm |
| Lock files | Reproducible builds |
| Registry API | REST + S3 for package storage |

---

## 10. Frontend / WASM (Phase 4)

**Spec:** `nova-spec/30-FRONTEND.md`

| Skill | Notes |
|---|---|
| Signals / fine-grained reactivity | SolidJS model — no virtual DOM |
| DOM bindings | `web-sys`-style descriptor bindgen |
| SSR / SSG | Render-to-string on native, hydrate on WASM |
| Bundler / tree-shaking | Module graph, dead-code elimination |
| HMR (hot module replacement) | Dev server WebSocket protocol |
| `wasm-bindgen` concepts | ABI between WASM and JS |

---

## 11. Testing

**Spec:** `nova-spec/50-TESTING.md`

| Skill | Notes |
|---|---|
| `insta` snapshot testing | Parser, type-checker, formatter output |
| `proptest` property testing | Lexer, parser, JSON |
| `assert_cmd` integration tests | Run `nova` binary end-to-end |
| `criterion` benchmarks | Performance regression tracking |
| Fuzz testing (`cargo-fuzz`) | Targets for lexer, parser, JSON, regex |

---

## 12. CI / Infrastructure

| Skill | Notes |
|---|---|
| GitHub Actions | `.github/workflows/ci.yml`, `release.yml`, `benchmarks.yml` |
| `cargo` caching | Speed up CI with `sccache` or `actions/cache` |
| Release automation | `cargo-release` or `release.yml` |
| Cross-compilation | Targets: `x86_64`, `aarch64`, `wasm32` |

---

## 13. Compiler Theory (Background Reading)

You don't need a PhD, but these concepts appear throughout the codebase:

| Concept | Where it matters |
|---|---|
| Hindley-Milner (Algorithm W) | `nova-typeck` |
| Pratt parsing | `nova-parser` (expression precedence) |
| SSA form | `nova-mir` |
| Register allocation | `nova-codegen-cranelift` / LLVM handles it |
| Dataflow analysis | Future optimization passes |
| DWARF debug info | Emitted by both Cranelift and LLVM backends |
| WebAssembly binary format | `nova-codegen-wasm` |
| Structured concurrency | `nova-runtime` task model |

---

## 14. Skill → Crate Matrix

| Crate | Rust | Parsing | Types | IR | Codegen | Async |
|---|---|---|---|---|---|---|
| nova-diagnostics | ✓✓ | — | — | — | — | — |
| nova-lexer | ✓✓ | logos | — | — | — | — |
| nova-ast | ✓ | — | — | — | — | — |
| nova-parser | ✓✓ | chumsky | — | — | — | — |
| nova-resolver | ✓✓ | — | ✓ | — | — | — |
| nova-typeck | ✓✓ | — | ✓✓✓ | ✓ | — | — |
| nova-hir | ✓✓ | — | ✓✓ | ✓✓ | — | — |
| nova-mir | ✓✓ | — | ✓ | ✓✓✓ | — | — |
| nova-codegen-cranelift | ✓✓ | — | — | ✓ | ✓✓✓ | — |
| nova-codegen-llvm | ✓✓ | — | — | ✓ | ✓✓✓ | — |
| nova-codegen-wasm | ✓✓ | — | — | ✓ | ✓✓✓ | — |
| nova-runtime | ✓✓✓ | — | — | — | ✓ | ✓✓✓ |
| nova-driver | ✓✓ | — | — | ✓ | ✓ | ✓ |
| nova-cli | ✓ | — | — | — | — | ✓ |
| nova-fmt | ✓✓ | ✓ | — | — | — | — |
| nova-lsp | ✓✓ | ✓ | ✓ | — | — | ✓✓ |
| nova-pm | ✓✓ | — | — | — | — | ✓ |

Legend: ✓ basic familiarity needed · ✓✓ solid working knowledge · ✓✓✓ deep expertise needed
