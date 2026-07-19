# Changelog

All notable changes to Nova are documented here.

Format follows [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).
Nova uses [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

---

## [Unreleased]

### Added (Phase 1 — MVP compiler, in progress)
- `nova run <file>`: compile and execute Nova programs natively via the
  Cranelift JIT; `nova check <file>`: type-check only
- `nova-resolver`: item-level name resolution (functions, sum types,
  variants), builtin prelude (`println`, `print`), E0002 duplicates
- `nova-typeck` + `nova-hir`: Hindley-Milner inference with occurs check,
  explicit generics at function boundaries, sum types with minimal
  exhaustiveness checking (E0020), typed & desugared HIR output
- `nova-mir`: monomorphization reachable from `main`, CFG lowering,
  match compilation to switches, short-circuit lowering
- `nova-runtime`: C-ABI strings, console output, sum allocation
  (leaking allocator pending GC — ADR 0002)
- `nova-codegen-cranelift`: MIR → native code via cranelift-jit
- Gate programs verified end-to-end: hello-world, fibonacci,
  match-on-enum, generic functions (with e2e stdout tests)

### Fixed
- Lexer: leading whitespace in a string segment directly after `${expr}`
  was skipped when resuming string mode

### Remaining for Phase 1 gate completion
- `nova build` (object emission + linking), LLVM release backend,
  records, traits, closures, `for` loops, full Maranget exhaustiveness

## [0.0.0] - 2026-05-10

Phase 0 (Foundation) milestone. Gate verified: `examples/01-hello-world` and
`examples/02-fibonacci` parse to AST with zero errors.

### Added
- Initial workspace setup (Phase 0)
- `nova-diagnostics`: error reporting infrastructure with codespan-reporting
- `nova-lexer`: full token set for Nova source files (logos-based)
- `nova-ast`: AST node type definitions
- `nova-parser`: recursive descent + Pratt parser (chumsky-based)
- `nova-cli`: `nova parse <file>` command for parser testing
- CI workflow (cargo test, fmt, clippy)
- Snapshot testing harness via `insta`
- Example files: hello-world, fibonacci
