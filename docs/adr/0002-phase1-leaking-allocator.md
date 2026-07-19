# ADR 0002 — Phase 1 uses a leaking allocator instead of bdwgc

## Status

Accepted (2026-07-19). Revisit before the Phase 1 gate is declared for
long-running programs, at the latest when `std/collections` lands (Phase 2).

## Context

`nova-spec/13-RUNTIME.md` §3.1 specifies bdwgc (Boehm) as the Phase 1 MVP
garbage collector, with MMTk in Phase 2+. Integrating bdwgc requires
bundling and building the C library (via `bdwgc-sys` style bindings) on all
three host platforms, and the primary development host for this repo is
Windows/MSVC where that build is the most fragile.

The Phase 1 gate programs (hello-world, fibonacci, match-on-enum, generic
functions) allocate only short-lived strings and small sum values, and
every gate program runs for milliseconds.

## Decision

`nova-runtime` Phase 1 allocates with the system allocator and never frees
(`Box::leak` semantics). The allocation entry points (`nova_rt_alloc_sum`,
string constructors) keep the same shape as the future GC interface
(`nova_gc_alloc`) so codegen does not change when the GC lands.

## Consequences

- Gate programs behave identically to a GC build (they exit before memory
  pressure matters).
- Long-running programs would leak; this is acceptable only inside Phase 1
  and is tracked as a TODO for the bdwgc integration step.
- No `Drop`/finalizer semantics yet, matching the spec's "minimal runtime"
  Phase 1 scope.
