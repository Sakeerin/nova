# Nova Language — Specification Bundle

> Execution-ready specification for building Nova, a modern full-stack programming language.
> All architectural decisions are locked. This bundle is designed to be handed to Claude Code (or any AI coding agent) to execute end-to-end.

---

## How to use this bundle

### If you're using Claude Code:

1. Place this entire `nova-spec/` folder at the root of your repo (or alongside it)
2. Open Claude Code in the project root
3. Tell Claude Code:
   > Read `nova-spec/00-MASTER-SPEC.md` and execute Phase 0. Follow the build order strictly. Do not ask me for approvals between steps. Commit frequently. Read additional spec files as referenced.

4. Claude Code will read the master spec, set up the workspace, and start implementing the lexer.

### If you're using another agent or working manually:

Read `00-MASTER-SPEC.md` first. It tells you what order to read the rest of the files and what to build when.

---

## File index

| File | What it specs |
|---|---|
| `00-MASTER-SPEC.md` | **Start here.** Project identity, locked decisions, folder structure, build order, conventions |
| `10-LEXER.md` | Tokens, lexer rules, error handling — Phase 0 |
| `11-PARSER.md` | EBNF grammar, AST nodes, parser approach — Phase 0 |
| `12-TYPESYSTEM.md` | Type rules, inference algorithm, traits, exhaustiveness — Phase 1 |
| `13-RUNTIME.md` | GC, async runtime, FFI, panic handling — Phase 1–2 |
| `14-CODEGEN.md` | LLVM/Cranelift/WASM backends, lowering rules — Phase 1, 4 |
| `20-STDLIB.md` | All stdlib module APIs (core, http, json, fs, etc.) — Phase 2 |
| `30-FRONTEND.md` | Signals, components, DOM bindings, bundler, SSR — Phase 4 |
| `40-TOOLING.md` | CLI, formatter, LSP, package manager, registry — Phase 3 |
| `50-TESTING.md` | Test strategy, fixtures, CI pipeline — All phases |
| `60-EXAMPLES.md` | Reference example programs (gate criteria) — All phases |

---

## What's locked vs. what's flexible

**LOCKED (do not change without an ADR):**
- Language name, syntax family, file extension
- Bootstrap language (Rust)
- Compilation strategy (native AOT + WASM, no JIT)
- Memory model (GC, no ownership in v1.0)
- Type system (HM + traits, monomorphization)
- Concurrency model (async/await + Tokio)
- Frontend reactivity (signals)
- License, registry, tooling architecture

**FLEXIBLE (judgment calls during implementation):**
- Specific error message wording
- Internal API shapes between crates
- Optimization choices in codegen
- Stdlib internal implementation
- CLI flag names (within reason)

---

## Quick-start commands for Claude Code

After Claude Code finishes Phase 0, these should work:

```bash
# In the repo root
cargo build --workspace
cargo test --workspace
./target/debug/nova parse examples/01-hello-world/src/main.nova
```

After Phase 1:
```bash
./target/debug/nova run examples/01-hello-world/src/main.nova
# Output: Hello, World!
```

---

## Estimated timeline

| Phase | Duration | Cumulative |
|---|---|---|
| Phase 0 (Foundation) | 4 weeks | 1 month |
| Phase 1 (MVP Compiler) | 20 weeks | 6 months |
| Phase 2 (Stdlib Core) | 16 weeks | 10 months |
| Phase 3 (Tooling) | 16 weeks | 14 months |
| Phase 4 (Frontend/WASM) | 24 weeks | 20 months |
| Phase 5 (Self-hosting) | 24 weeks | 26 months |
| Phase 6 (1.0 Release) | 16 weeks | 30 months |

These are best-case estimates assuming consistent work. Real projects of this size historically take 1.5–2x longer (Rust took 10 years, Bun took 4 years to reach buzz).

---

## Realistic expectations

This bundle assumes you (the human) have:
- A clear motivation for building this
- Time horizon measured in years, not months
- Some kind of sustainable workflow (full-time, weekend project, sponsored)
- Access to Claude Code or equivalent agentic coding tool
- Willingness to debug compiler internals when things break

If any of these aren't true, consider:
- A smaller scope (e.g. just a DSL for one domain)
- Contributing to an existing language project
- Building on top of an existing platform (Rust crate, Deno extension)

---

## Working with the spec

- The spec is **versioned**. If you change it, bump a version comment in `00-MASTER-SPEC.md` and note the change in `CHANGELOG-SPEC.md` (you'll create this if needed).
- ADRs (`docs/adr/`) capture decisions that deviate from the spec.
- RFCs (`docs/rfcs/`) propose new features that the spec doesn't yet cover.

---

## License

This specification is released under CC-BY-4.0. The reference implementation (when built) will be MIT OR Apache-2.0.
