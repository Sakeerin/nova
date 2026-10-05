# ADR 0020 — Size-class page heap

## Status

Accepted (2026-10-01). Design:
`docs/superpowers/specs/2026-10-01-gc-page-heap-design.md`.

## Context

The collector (`crates/nova-runtime/src/gc.rs`) gave every object its own
system allocation and an `Obj` record in one vector. Each collection rebuilt
and sorted an index of every object so marking could binary-search it, then
freed each dead object with its own `dealloc` and its own
`task::forget_freed_state` call. `examples/05-json-api/BENCHMARK.md`'s
"(gc-phase-cost)" amendment measured that per-object work (sweep, clearing
marks, building and sorting the index) at 69.3–70.7% of collection time,
and the collector at 93.8–101.6 µs of a ten-user request. The gate allows
100 µs for the whole request.

## Decision

- **Pages of one size class.** Every object of 2048 bytes or less
  (`pages::SMALL_MAX`) lives in a 65,536-byte page of one of 24 size
  classes, from 16 to 2048 bytes.
- **Bitmaps, kept out of band.** Each page's descriptor holds allocated,
  marked and scan bitmaps, outside the page, so `alloc` still returns a
  bare pointer with nothing in front of it.
- **Marking** finds a slot by binary-searching a directory of page bases.
- **The sweep** frees unmarked slots by bitmap arithmetic, 64 at a time.
- **Allocation** takes the lowest free bit and zeroes that slot.
- **Large objects keep the old path.** Anything over 2048 bytes keeps its
  own system allocation and an `Obj` record, in a short vector sorted at
  each collection.
- **Empty pages are bounded.** A sweep keeps up to 16 empty pages
  (`pages::RESERVE_PAGES`, 1 MiB) in a reserve and returns the rest to the
  system.
- **The executor's state map** is pruned once per collection by
  `task::prune_freed_states`, against a predicate that holds for exactly
  the start addresses of the survivors. There is therefore no longer a
  per-object notification of any kind.

The collector stays conservative, non-moving and thread-local, and still
skips collection off Windows.

**Amended 2026-10-05 (branch `gc-unix-stack-bounds`):** collection runs on
glibc Linux and macOS as well now (`docs/adr/0024-gc-stack-bounds-on-unix.md`); other platforms still
skip it.

## Alternatives rejected

- **Mixed-size blocks with an object-start bitmap (Immix-style).** No
  rounding waste, but it needs hole-finding and fragmentation handling. It
  also costs more per object to look up and sweep than a fixed slot
  division, for a heap whose objects were 77% 16 bytes or smaller
  ("(alloc-per-request)").
- **Pages whose free slots are threaded into a list through their first
  word.** Allocation is a pop, but the sweep has to write to every freed
  slot, which is the per-object cost being removed. Freed slots would also
  hold pointer-shaped words.
- **Local fixes on the old heap** (a radix sort, pruning the state map once)
  and **recycling freed blocks per size**. Each leaves the per-collection
  index, and none is sized like the gap to the gate.

## Consequences

- **Rounding waste.** Up to 128 bytes, a request rounds up to the next
  multiple of 16, at most 15 bytes. Above that it adds less than 25% of the
  request. How much of this is new against the system allocator's own
  granularity is not measured.
- **Partly filled pages** hold memory that the system allocator would have
  had back. The measurement amendment for this change reports the peak
  working set.
- **The per-object hook is gone.** ADRs 0012, 0016 and 0017,
  `nova-spec/13-RUNTIME.md` §3.4 and `nova-spec/20-STDLIB.md` describe that
  hook as existing, and carry dated amendments. Their decisions stand.
- **A thread that exits** leaks its pages, as it leaked its objects before.
  Its minimum is one page per size class it used.
