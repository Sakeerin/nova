# ADR 0010 — The conservative stack scan's root tests are gated, not fixed

The eight tests that drive `nova-runtime`'s real, stack-scanning `gc::collect()`
are `#[ignore]`d unconditionally. This records why, what the underlying
mechanism actually is, which two remedies were tried and measured to make it
worse, and what a future attempt must not repeat.

**Numbering:** `docs/adr/0009-async-execution-model.md` is reserved by
`docs/superpowers/plans/2026-08-07-phase-2-3a-async-core.md` (Task 8) and does
not exist yet; this document deliberately takes 0010 rather than claiming it.

## Status

Accepted (2026-08-08). Phase 2.3a, branch `async-core`. Supersedes the
release-only `#[cfg_attr(not(debug_assertions), ignore = …)]` gating that Task 3
of the same phase originally applied to six of these eight tests.

The gate is expected to be revisited, not permanent. It is recorded as an ADR
rather than a code comment because the case for it rests on measurements, and
this branch's standing rule is **invariant in the comment, measurement in the
document** — eight doc-accuracy defects on this branch came from comments that
narrated a number which later drifted.

## Context

`crates/nova-runtime/src/gc.rs` is a conservative, non-moving mark-and-sweep
collector. Neither codegen backend emits stack maps, so any machine word whose
value falls inside a live allocation keeps that allocation alive. Roots come
from the stack, from callee-saved registers (flushed by the `setjmp` shim in
`gc_stack.c`), from the `PINNED` registry (`gc::add_root` / `gc::remove_root`),
and transitively from scanned heap objects.

Eight tests assert on the *outcome* of a real collection, and so are the only
coverage of the `PINNED` registry actually being seeded into the mark set:

| File | Module | Tests |
|---|---|---|
| `crates/nova-runtime/src/gc.rs` | `#[cfg(windows)] mod registry` | 6 |
| `crates/nova-runtime/src/task.rs` | `#[cfg(windows)] mod root_registration` | 2 |

Four of the eight assert an object **was swept** (`object_info(addr).is_none()`)
and four assert an object **survived** (`is_some()`). Each `is_some()` test is
paired with an `is_none()` control, because an `is_some()` assertion alone also
passes against a collector that frees nothing at all — the exact vacuous-green
shape this project has shipped before.

All eight are `#[cfg(windows)]`: `gc.rs`'s `stack_base()` only implements
precise stack bounds on Windows, and elsewhere `collect()` returns before
marking anything, which would make every `is_some()` assertion pass vacuously
and every `is_none()` assertion fail outright.

## The mechanism, as measured

**The four `is_none()` tests intermittently fail** — in debug as well as
release, at the default test parallelism CI runs at. The failure is always
**over-retention**: an object that should have been swept survives. Never a
false free. That is the direction the collector's own contract already calls
acceptable, so it produces spurious *test* failures, never data corruption.

Run-level failure rates for `cargo test -p nova-runtime --lib` at default
parallelism (12 logical CPUs), 20 runs per sample: **5/20, 7/20 and 5/20**
across three independently taken samples — roughly 25-35%, evidently a
stochastic process rather than a fixed rate.

An instrumented collection (a parallel recording of every `(stack address,
word value)` pair the scan pushed into `ROOTS`, read after the mark/sweep
decision was already final, then reverted) located the retaining word:

| Capture | Scan range | Retaining word found at, as bytes above `lo` | As % of the way from `lo` to `hi` |
|---|---|---|---|
| A | 3696 bytes | 2776, 2952, 2984 | 75%, 80%, 81% |
| B | 3952 bytes | 2104, 2776, 2984 | 53%, 70%, 75% |

**It is not the register-flush buffer.** `&regs` in `gc_stack.c` sits at `lo`,
the very start of the scanned range; every candidate is 53-81% of the way from
`lo` toward `hi`. Register retention is ruled out.

**It is a stale stack word in an already-returned frame.** `lo` is the shim's
own frame — the deepest, most recent point in the call chain at scan time. A
word 75-81% of the way toward `hi` (the thread's base) sits far above anything
`collect()`'s own call chain can still be occupying, in territory whose frames
had already returned before the collection started.

A further observation, held with less confidence: the offsets *from `lo`*
repeat almost exactly across independently-ASLR'd runs (2776 and 2984 appear in
both captures, at completely different absolute addresses). That is more
consistent with a deterministic position in the test's own always-identical
call chain — most plausibly `gc::alloc`'s own now-popped frame, which is
invoked from a fixed point in every affected test and has enough locals in an
unoptimized build — than with content inherited from a different, previously
exited thread. It does not distinguish those two sub-explanations conclusively;
both are "a stale word in an already-returned frame".

## Two remedies, both measured worse

1. **A deliberate stack clobber.** An `#[inline(never)] fn clobber_stack()`
   writing a 4 KiB non-zero buffer between the setup function's return and the
   collection, on the theory that overwriting more stack raises the odds of
   covering the risky bytes. **Measured: 4/20 failures with it, 1/20 without**,
   isolated test, default parallelism. Reverted, never committed.
2. **A serializing mutex** across all eight tests (`gc::lock_scan_test`,
   committed as `1bed9a2` and reverted as `3faca50`). **Measured: 7/20
   run-level failures before, 20/20 after**, identical command and sample size;
   at full-workspace granularity 5/5 runs failed, always the same pair of
   tests. The lock was independently verified to actually exclude concurrent
   holders (a holder counter plus a 50 ms artificial hold showed serial
   timing and never a second holder), so "the mutex silently didn't work" is
   ruled out — it worked, and the flakiness got worse.

A third, unintended data point: instrumenting only two of the six `gc.rs` tests
measurably **shifted which test failed** onto the four un-instrumented ones.
The system is sensitive to very small perturbations of code shape, which is why
both remedies above plausibly backfired for the same reason — forcing tighter
temporal and spatial proximity between scan tests, or changing frame layout,
changes which stale bytes are still readable.

## Decision

1. **All eight tests carry an unconditional `#[ignore]`**, in debug and release
   alike, rather than only the four that flake. Gating only the flaky half
   would strip the negative control from the surviving `is_some()` half and
   leave four green assertions that also pass against a collector that frees
   nothing.
2. **The mechanism is recorded, not fixed.** No further remedy is attempted
   inside a task whose scope is something else.
3. **CI runs them as an advisory, `continue-on-error` step**
   (`.github/workflows/ci.yml`), separate from `cargo test --workspace
   --all-features`. Without it, nothing CI executes checks that `PINNED` is
   *honoured* by a real collection: the registry could be populated correctly
   and ignored entirely by the marker, and no other test would notice. An
   advisory step that is sometimes red is strictly better than no coverage of
   the seeding at all.

   This is deliberately narrower than "the only coverage of `gc::add_root`".
   That the executor registers its root, and releases it exactly once, is
   asserted directly on the registry by `nova-runtime`'s
   `a_completed_tasks_state_stays_rooted_until_its_output_is_taken`, which
   needs no collection and so is neither `#[ignore]`d nor `#[cfg(windows)]` —
   it runs in the ordinary test step on all three legs. Deleting
   `gc::add_root` from `spawn` fails *that* test, not these eight.
4. **That step must not use `--test-threads=1`.** Serialization is remedy 2
   above in a different costume, and was measured to make this flakiness two
   orders of magnitude worse.

## Consequences

- `cargo test --workspace` is stable: 44 targets, 0 failed, 8 ignored.
  Before the gate, the same command failed roughly 40% of the time at default
  parallelism.
- The eight tests are reachable with `cargo test -- --ignored`, and remain
  the only coverage of `PINNED` being seeded into the mark set. Expect 1-2 of
  them to fail in roughly 1 run in 10 there; that is this document, not a
  regression.
- Anyone touching `gc.rs`'s scan or either test module should assume any
  change in frame shape can move the failure around, and should not read a
  moved failure as evidence about their change.

## What a future fix might look like

Both sketches below are unverified guesses consistent with the evidence, and
neither should be implemented on the strength of this document alone.

- **In the tests.** If the source really is `gc::alloc`'s own returned frame,
  a test could force that frame's memory to be overwritten before scanning.
  Note that this is the class remedy 1 belongs to, and it backfired.
- **In the collector.** Narrow what the scan treats as in range — bound it to
  the caller's own frame plus a deliberately generous margin, rather than the
  whole remaining stack up to `stack_base()`. That would exclude libtest's
  ancestor scaffolding from ever being scanned. It needs a real design
  conversation first about whether a legitimate root can ever live that far up
  the stack in compiled Nova code, which is a larger question than a test fix.

## References

- `crates/nova-runtime/src/gc.rs` — the collector, and `mod registry`.
- `crates/nova-runtime/src/task.rs` — `mod root_registration`.
- `.github/workflows/ci.yml` — the advisory `--ignored` step.
- `docs/adr/0002-phase1-leaking-allocator.md` — why the collector is
  conservative in the first place.
