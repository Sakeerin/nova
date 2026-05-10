# Nova — Agent Guide

> Practical instructions for AI coding agents (Claude Code, Cursor, etc.) working on the Nova compiler.
> This complements `nova-spec/00-MASTER-SPEC.md`, which defines *what* to build. This file defines *how to work* in the repo.

---

## 1. Orientation

| Item | Location |
|---|---|
| Language spec & build order | `nova-spec/00-MASTER-SPEC.md` |
| Architecture overview | `ARCHITECTURE.md` |
| All crates | `crates/` |
| Standard library (Nova source) | `std/` (Phase 2+) |
| Example programs (gate criteria) | `examples/` |
| Integration tests | `tests/` |
| Architecture decisions | `docs/adr/` |
| RFCs (proposed features) | `docs/rfcs/` |

Read `nova-spec/00-MASTER-SPEC.md` end-to-end before touching any code. Section 3 (Build Order) and Section 5 (Conventions) are binding.

---

## 2. Where to Start Each Phase

### Phase 0 (current)
```
1. nova-diagnostics  → error reporting shared infra
2. nova-lexer        → source → tokens (see nova-spec/10-LEXER.md)
3. nova-ast          → AST node types
4. nova-parser       → tokens → AST (see nova-spec/11-PARSER.md)
5. nova-cli          → wire `nova parse <file>`
```
Gate: `cargo run -p nova-cli -- parse examples/01-hello-world/src/main.nova` produces an AST without panicking.

### Phase 1+
See `nova-spec/00-MASTER-SPEC.md` §3 for subsequent phase order.

---

## 3. Running the Project

```powershell
# Build everything
cargo build --workspace

# Run all tests
cargo test --workspace

# Lint (must pass — CI enforces this)
cargo clippy --all-targets --all-features -- -D warnings

# Format check
cargo fmt --check

# Parse a Nova file (Phase 0 gate)
cargo run -p nova-cli -- parse examples/01-hello-world/src/main.nova

# Run a Nova program (Phase 1 gate)
cargo run -p nova-cli -- run examples/01-hello-world/src/main.nova
```

---

## 4. Non-Negotiable Rules

These are from `nova-spec/00-MASTER-SPEC.md §5`. Violating them will cause CI failure or spec drift.

### Code Quality
- `cargo fmt` + `cargo clippy -D warnings` must pass on every commit.
- No `unwrap()` outside `#[cfg(test)]` — use `expect("descriptive reason")` or `?`.
- All public Rust items must have rustdoc comments.
- Errors must implement `std::error::Error` via `thiserror`.

### Testing
- Every module ships with `#[cfg(test)] mod tests { ... }`.
- Parser, type-checker, and formatter output → snapshot tests via `insta`.
- Integration tests in `tests/` use `assert_cmd` to invoke the `nova` binary.

### Commits
- Conventional commits: `feat:`, `fix:`, `chore:`, `docs:`, `refactor:`, `test:`.
- One logical change per commit; body explains *why*.

### Dependencies
- All workspace dependencies declared in root `Cargo.toml` under `[workspace.dependencies]`.
- Do not add new crates without noting justification in the commit message.
- Approved crate list is in `nova-spec/00-MASTER-SPEC.md §6`.

---

## 5. Crate Responsibilities (Quick Reference)

| Crate | Role | Key dependency |
|---|---|---|
| `nova-diagnostics` | Shared error types + rendering | `codespan-reporting` |
| `nova-lexer` | `&str` → `Vec<Spanned<Token>>` | `logos` |
| `nova-ast` | Pure data types — no logic | — |
| `nova-parser` | Tokens → AST | `chumsky` |
| `nova-resolver` | Name resolution, module graph | `nova-ast`, `nova-diagnostics` |
| `nova-typeck` | HM type inference + checking | `nova-hir`, `nova-diagnostics` |
| `nova-hir` | Desugared, typed AST | `nova-ast` |
| `nova-mir` | 3-address IR | `nova-hir` |
| `nova-codegen-cranelift` | Debug-mode object files | `cranelift` |
| `nova-codegen-llvm` | Release-mode object files | `inkwell` |
| `nova-codegen-wasm` | WASM modules | `wasm-encoder`, `walrus` |
| `nova-runtime` | GC + async runtime (Rust, linked) | `tokio` |
| `nova-driver` | Pipeline orchestration | all crates above |
| `nova-cli` | `nova` binary, CLI arg parsing | `nova-driver`, `clap` |
| `nova-fmt` | Opinionated formatter | `nova-ast` |
| `nova-lsp` | LSP server | `tower-lsp` |
| `nova-pm` | Package manager | `nova-driver`, `toml` |
| `nova-bundler` | Frontend bundler | `nova-codegen-wasm` |
| `nova-doc` | Doc generator | `nova-ast` |
| `nova-test` | Test runner | `nova-driver` |

---

## 6. Compilation Pipeline (Data Flow)

```
.nova source
    │
    ▼  nova-lexer
Vec<Spanned<Token>>
    │
    ▼  nova-parser
AST (nova-ast)
    │
    ▼  nova-resolver
Name-resolved AST
    │
    ▼  nova-typeck
Typed HIR (nova-hir)
    │
    ▼  nova-mir lowering
MIR
    │
    ├──▶ nova-codegen-cranelift  →  object (debug)
    ├──▶ nova-codegen-llvm       →  object (release)
    └──▶ nova-codegen-wasm       →  .wasm (browser)
```

Each stage is a separate crate. Pass data as typed structs — avoid stringly-typed interfaces.

---

## 7. Locked Decisions (Do Not Override)

These require an ADR in `docs/adr/` before any deviation:

- **Bootstrap language:** Rust (edition 2021, MSRV 1.78)
- **Backends:** Cranelift (debug) + LLVM (release) + WASM (browser)
- **No JIT, no bytecode VM**
- **Memory:** tracing GC (MMTk, falling back to bdwgc) — no ownership in v1.0
- **Type system:** Hindley-Milner + trait extensions, monomorphization
- **Concurrency:** async/await on Tokio
- **Reactivity:** signals (SolidJS model), no virtual DOM
- **Formatter:** opinionated, no config
- **`null` keyword:** does not exist — use `Option<T>`
- **Exceptions:** do not exist — use `Result<T, E>` + `?`

---

## 8. Error Handling Conventions

User-facing errors must follow Elm/Rust quality standards:
- Unique error code (`E0001`, `E0002`, …)
- One-line title
- Source span with caret pointer (via `nova-diagnostics` + `codespan-reporting`)
- Explanation paragraph
- Suggestion or fix-it hint
- Link placeholder: `https://docs.novalang.dev/errors/E{code}`

Emit diagnostics through `nova-diagnostics::DiagnosticEngine` — never via `eprintln!` in library crates.

---

## 9. Snapshot Testing (insta)

For parser, type-checker, and formatter tests, use `insta`:

```rust
#[test]
fn test_parse_hello_world() {
    let src = include_str!("../tests/fixtures/hello.nova");
    let ast = parse(src).unwrap();
    insta::assert_debug_snapshot!(ast);
}
```

Update snapshots with:
```powershell
cargo insta review
```

Committed snapshots live in `crates/<crate>/src/snapshots/`.

---

## 10. When Blocked

1. Check the relevant spec file (`nova-spec/10-LEXER.md`, `11-PARSER.md`, etc.).
2. Check `docs/adr/` for prior decisions.
3. If genuinely ambiguous, write a `SCRATCHPAD.md` note and proceed with the next independent task — do not halt.
4. Do not deviate from locked decisions without opening an ADR.

---

## 11. Phase Gating

Never merge work that doesn't satisfy the current phase's gate. Gate commands are in `nova-spec/00-MASTER-SPEC.md §3`. Tag each completed phase as `v0.{phase}.0`.

---

## 12. File Naming Conventions

| Thing | Convention |
|---|---|
| Rust source | `snake_case.rs` |
| Nova source | `snake_case.nova` |
| Test fixtures | `tests/fixtures/<name>.nova` |
| Snapshots | `src/snapshots/<module>__<test_name>.snap` |
| ADRs | `docs/adr/NNNN-short-title.md` |
| RFCs | `docs/rfcs/NNNN-short-title.md` |
