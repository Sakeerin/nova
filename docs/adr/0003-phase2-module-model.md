# ADR 0003 — Phase 2 module model

## Status

Accepted (2026-07-23). First increment of Phase 2.0 (language completeness).

## Context

Phase 1 compiled a single `.nova` file. Phase 2's standard library is written
in Nova across many files (`std/core`, `std/io`, …) with `import` between them,
so the compiler needs a real module system: multi-file input, cross-module name
resolution, and encapsulation.

Constraints from the existing pipeline:

- `nova-resolver` keys every definition by an `item_index` into the one parsed
  `File`; `nova-typeck` reads `self.file.items[item_index]`.
- `nova-mir` monomorphizes **whole-program** from `main`.
- Trait/impl coherence is already global (impls collected across the program).

## Decision

- **One file = one module.** A module's name is its file stem. `import foo`
  resolves to `foo.nova` in the same directory as the entry file (nested
  `a.b` → `a/b.nova` paths are a later increment). The entry file is itself a
  module.
- **Whole-program merge.** The driver loads the entry plus all transitively
  imported files and the resolver concatenates their items into a single merged
  `File`; `item_index` becomes the global index into that merged list. This
  keeps `nova-typeck` and whole-program monomorphization essentially unchanged.
- **Per-module scopes with `pub` visibility.** Each module has its own value /
  type / trait namespace: its own items (public and private) plus the prelude
  plus names it imports. Only `pub` items are importable by other modules.
  Name resolution in `nova-typeck` is **module-relative** — a name is resolved
  in the scope of the module that owns the item currently being checked.
- **Imports.** `import m` brings all of `m`'s `pub` names into scope (glob);
  `import m::{a, b}` brings the named ones. A name bound twice conflicts
  (`E0002`). `import m as x` and qualified `m::name` paths are a later
  increment.
- **Global coherence.** `impl` blocks are still collected program-wide and
  apply globally for trait dispatch, regardless of the module they appear in
  (as in Rust). Only *named* references (fn / type / trait / const / variant)
  are module-scoped.

## Consequences

- Minimal churn to the back half of the pipeline: MIR/codegen are unchanged; the
  merge preserves single-`File` assumptions.
- `nova-resolver` gains per-module scopes and an `item_index → module` map;
  `nova-typeck` threads the current module through its ~13 resolution sites.
- A private-item access across modules is a resolution error, giving real
  encapsulation.
- Deferred to later increments: `import as` aliases, qualified `m::name` paths,
  nested module directories, and re-exports.
