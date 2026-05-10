# ADR 0001: Native AOT Compilation Only

**Status:** Accepted  
**Date:** 2026

## Context

Nova needed to choose a compilation and execution model.

## Decision

Nova compiles to native code via AOT (Ahead of Time) compilation using LLVM (release) and
Cranelift (debug). There is no interpreter, no JIT, and no bytecode distribution format.

For the browser, Nova compiles to WebAssembly via a dedicated WASM backend.

## Rationale

- Predictable performance without JIT warm-up
- Smaller runtime (no interpreter VM)
- Better startup time for CLI tools
- LLVM gives access to state-of-the-art optimizations for release builds
- Cranelift is faster to compile to, giving better developer iteration speed in debug

## Consequences

- No REPL in the traditional sense (Phase 3 uses AOT-then-dlopen)
- Longer initial compile times vs interpreters
- Self-hosting requires bootstrapping (Phase 5)
