# Nova Architecture

## Overview

Nova is a statically-typed, full-stack programming language built in Rust. This document describes the high-level architecture of the compiler and toolchain.

## Compilation Pipeline

```
Source (.nova)
    │
    ▼
nova-lexer          → Vec<Spanned<Token>>
    │
    ▼
nova-parser         → AST (nova-ast)
    │
    ▼
nova-resolver       → Name-resolved AST
    │
    ▼
nova-typeck         → Typed HIR (nova-hir)
    │
    ▼
nova-mir            → Mid-level IR
    │
    ├─► nova-codegen-cranelift  → Object file (debug)
    ├─► nova-codegen-llvm       → Object file (release)
    └─► nova-codegen-wasm       → WASM module (browser)
```

## Crates

| Crate | Purpose |
|---|---|
| `nova-cli` | Binary entry point, CLI argument parsing |
| `nova-driver` | Orchestrates the compilation pipeline |
| `nova-diagnostics` | Shared error reporting infrastructure |
| `nova-lexer` | Source → tokens (uses `logos`) |
| `nova-ast` | AST node type definitions |
| `nova-parser` | Tokens → AST (hand-written recursive descent) |
| `nova-resolver` | Name resolution, module graph |
| `nova-typeck` | Type inference and checking (HM + extensions) |
| `nova-hir` | High-level IR, desugared AST |
| `nova-mir` | Mid-level IR, 3-address style |
| `nova-codegen-cranelift` | Fast debug backend |
| `nova-codegen-llvm` | Optimized release backend |
| `nova-codegen-wasm` | WebAssembly backend |
| `nova-runtime` | GC, async runtime, panic handling |
| `nova-std` (in `std/`) | The standard library's `.nova` sources, embedded for the compiler |
| `nova-fmt` | The formatter: prints the AST in one fixed layout, keeps every comment, and refuses output that would change the program (ADR 0028) |
| `nova-lsp` | The language server, `nova lsp`: diagnostics, completion and formatting over `nova_driver::analyze` (ADR 0029) |
| `nova-test` | Test runner |
| `nova-pm` | Package manager: `nova.toml` parsing and project discovery so far |
| `nova-bundler` | Frontend bundler |
| `nova-doc` | Documentation generator |
| `nova-bench-http` | Keep-alive HTTP load generator for benchmarking `std/http` |

## Key Design Decisions

See `docs/adr/` for Architecture Decision Records.

- AOT compilation only (no JIT, no VM) — see `docs/adr/0001-native-aot.md`
- Tracing GC via MMTk
- Hindley-Milner type inference with trait extensions
- Signals-based frontend reactivity (SolidJS model)
