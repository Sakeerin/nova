# ADR 0040 — The WASM collector and executor

## Status

Accepted, 2026-10-15 (Phase 4.1, branch `claude/loving-archimedes-frtf5q`;
spec `docs/superpowers/specs/2026-10-15-phase-4-1-the-language-on-wasm-design.md`).
The executor's language, the collector's timing and `block_on` on WASM are
the user's decisions of 2026-10-15.

## Context

ADR 0039 gave a WASM build its runtime, `std/rt`, written in Nova, with an
allocator that only grew, and no async: `async fn main` was E0091.
- `nova-spec/13-RUNTIME.md` §8 says a WASM build starts with reference
  counting. `docs/phase-4-plan.md` §3 decision 3 chose mark-sweep between
  event turns instead: the graph of signals and effects is full of cycles.
- The same plan's decision 4 put async on the browser's event loop, with
  no `block_on` on WASM.
- `nova-runtime`'s executor (`task.rs`) and collector (`gc.rs`,
  `gc/pages.rs`) cannot run in a browser: the collector scans the native
  stack, and the executor blocks the thread to wait.

## Decision

1. **The collector is conservative, non-moving mark-sweep, run only
   between turns.**
   - A turn is one call from JS that runs Nova code: `main`, or
     `nova_turn`. When it returns, the WASM stack holds nothing the
     collector must see, so there is no stack to scan, and no shadow stack.
   - Its roots are the executor's block and tables, and each unreleased
     task's state, as `nova-runtime` pins it. 4.2's table of values JS
     holds will be one more.
   - A word is a pointer if it falls inside an allocated object. Strings,
     `Bytes` and the executor's tables are leaves, kept but not traced.
     Marking uses a worklist in memory: V8's stack is about 1 MB.
   - The threshold is native's: a collection once the bytes allocated since
     the last reach 1 MiB, then twice the live bytes. `NOVA_GC_STRESS`
     collects at the end of every turn, and `NOVA_GC_DEBUG` writes native's
     line. JS passes them in, from Node's environment or `init`'s options.
   - Only a program with an `async fn main` has turns after `main`, so only
     it collects, and only its module exports the switches.
   - A long computation within one turn does not collect: it stops with
     "out of memory" at the memory's 1 GB.
2. **The allocator is `gc/pages.rs`'s design.** Objects of 2,048 bytes or
   less take a slot of one of 24 size classes in a 64 KiB page, which is
   WASM's page; larger ones a run of pages. Three bitmaps per page record
   which slots are allocated, marked and to be traced, and a page table
   finds a page's kind. A large object's size and flags are in front of it
   in its first page, where native keeps them out of band: WASM has no
   system allocator to hold them. The page table, not the object, finds
   them, so nothing in a string's bytes can pass for them. Slots are zeroed
   when handed out. Memory cannot shrink, so an empty page is free for any
   later use.
3. **The executor is written in Nova, in `std/rt`,** a port of `task.rs`
   with its order: spawn order, a FIFO queue, timers woken in the order
   they were parked, the deadlock report.
   - The backend builds one future itself, the yield future, pending once
     with no wait staged. `sleep`, `join` and `timeout` are `async fn`s
     over it, which stage their wait and await a fresh one; each fixes its
     deadline or target at its first poll, which comes as std builds it.
   - A WASM-only builtin, `future_poll`, polls a future once; the executor
     drives every task through it. Another, `host_abort`, ends the program
     after the deadlock report, which has no `nova: panic:` prefix.
   - Its state is in leaf objects that a state word points at. A state's
     task is found through a hash index, pruned after each collection, so a
     reused address names no old task.
4. **`async fn main` runs on the event loop.** Its shim spawns the root and
   runs what can run, then returns to JS. Each `nova_turn` wakes what is
   due, runs the queue, collects if due, and returns the milliseconds to
   the next deadline, or -1 when no task waits; JS sets a timer for it. A
   module exports `nova_turn` only when it has an `async fn main`.
5. **`block_on` works on WASM,** unlike the phase plan's decision 4, by
   the user's decision: it runs the executor within the call, and waits for
   a deadline by reading the clock in a loop. A page that sleeps inside
   `block_on` does not respond until it returns; `async fn main` is the
   event loop's way.
6. **`std/rt`'s state words grow to `8..128`,** and the data segment starts
   at 128: the allocator's metadata, the executor's block, the log
   configuration and the collector's words have fixed addresses, which
   need no root.

## Consequences

- **Two executors and two collectors exist,** Rust's and Nova's. The
  gate's 94 programs, run both ways and under stress, keep their order and
  their results equal.
- **WASM's std gains task, sync, time and log.** A name from a module it
  lacks is E0092, naming the module; E0091 is unreachable through WASM's
  std.
- **Async code that waits in `block_on` blocks a page,** as it blocks a
  native thread.
- **4.2's handles and closures held by JS** must be roots of this
  collector, and a call from JS into Nova a turn.

## References

- `docs/phase-4-plan.md` §3, decisions 3 to 5.
- The spec named above, §3 to §6 and §13.
- ADR 0020, the size-class page heap; ADR 0009, the async execution
  model; ADR 0039, the WASM backend.
- `nova-spec/13-RUNTIME.md` §8; `nova-spec/14-CODEGEN.md` §8;
  `nova-spec/30-FRONTEND.md` §7.
