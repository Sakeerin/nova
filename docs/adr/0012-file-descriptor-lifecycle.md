# ADR 0012 — `File` descriptor lifecycle: explicit `close` only, close-on-collect foreclosed

**Numbering:** confirmed against `docs/adr/`'s actual contents rather than
trusted from the plan — `0001` through `0011` all exist with no gap, so
`0012` is next.

## Status

Accepted (2026-08-14). Increment 3c of the `std/fs`-on-Strings decomposition
(`docs/superpowers/specs/2026-08-14-file-open-and-openoptions-design.md` §3,
§6), branch `file-open-openoptions` — the increment that shipped `File`,
`open` and `OpenOptions`, closing `docs/adr/0011-io-error-kinds.md` decision
2's deviation.

## Context

`File` is Nova's first value holding an OS resource across more than one
intrinsic call. Every `std/fs` function before it opens, acts and closes
inside one intrinsic; increment 3b's three standard streams
(`Stdin`/`Stdout`/`Stderr`) are process-global and never closed. Neither
needed a table of live handles. `File` does: `crates/nova-runtime/src/file.rs`
holds a thread-local `HashMap<i64, std::fs::File>`, and `File { fd: Int }`'s
`fd` is a key into it, not an OS file descriptor number.

**Nova has no destructors and no finalizers.** The collector can free the
`File` record naming an open handle while that handle stays open at the OS
level — nothing runs when a GC object dies, by design. So the question this
decision answers is: what closes the handle when a Nova program forgets to?

The answer looks, at first read, like it might already exist. **The
collector has a per-object notification hook.** `gc.rs`'s sweep calls
`crate::task::forget_freed_state(o.addr)` unconditionally for every object it
frees, not only for task states — `forget_freed_state` itself simply misses
for an address that was never a task's state (`task.rs`'s own doc comment:
"an address that never was a task's state simply misses: this map is keyed
on state objects, and a sweep frees every kind of heap object"). The call
site's own comment states the constraint such a hook must satisfy: it runs
with `HEAP` borrowed, so it must touch nothing the collector owns, allocate
nothing through `alloc`, and not re-enter the collector. Closing a file is a
syscall with no allocation, so it fits that shape. **A reader who finds this
hook will reasonably ask why `File` does not register with it.**

## Decision

**Explicit `close` is the only release mechanism. Forgetting it leaks the
descriptor until the process exits — on every platform, deliberately.**
`std/fs/lib.nova`'s `File::close` is `async fn close(self) -> Result<(),
IoError>`, idempotent (a second or third call also returns `Ok(())`, since
Nova has no move checking and `self` by value cannot stop a second call); any
other operation on a closed, stale, or forged handle misses the same table
lookup and returns the identical `IoError { kind: Other }`, because absence
from the table *is* closedness (`crates/nova-runtime/src/file.rs`'s own
module doc).

This is the same shape `docs/adr/0009-async-execution-model.md` §1 already
documents and accepts for a different resource: **"a spawned task whose
output is never taken leaks its state object"** — inherited here, not
invented. Neither leak is fixed by this decision; both are accepted as the
deliberate half of a trade, for reasons specific to what would have to change
to close them.

**Close-on-collect is foreclosed for two measured reasons, not merely left
unbuilt:**

1. **`fd: Int` makes it impossible, not merely unused.** The collector's
   sweep walks GC-tracked *objects* — records, arrays, strings, closures,
   sums — and notifies on their addresses dying. It never sees an `Int`
   *inside* a record: a `File`'s `fd` field is an ordinary scanned word, not
   itself a heap object with an address the sweep can report on. When a
   `File` record is freed, `forget_freed_state` is called with *that
   record's own address*, which names nothing in `file.rs`'s handle table —
   the table is keyed by `fd`, an arbitrary small integer bearing no
   relation to any GC address. Keeping the option open would require `File`
   to hold a GC-managed cookie whose *address* keys the table instead of an
   `Int` doing so — `JoinHandle<T>`'s pointer-identity pattern
   (`docs/adr/0009-async-execution-model.md` §1, branch `task-identity`),
   which a prior increment deliberately migrated *toward* for exactly this
   kind of lookup. That was considered for `File` too and declined; see
   Alternatives below.
2. **Collection does not run at all off Windows.** `gc.rs`'s `stack_base()`
   returns `None` under `#[cfg(not(windows))]`, with the comment "Precise
   stack bounds for non-Windows platforms are a follow-up; until then
   collection is skipped there." So even a `File` redesigned to be
   collect-friendly would be closed automatically on Windows and leak
   unboundedly on Linux and macOS. **A platform asymmetry in resource
   correctness is worse to document than a uniform leak** — it would encode
   a guarantee two of the three supported platforms cannot honor, which
   invites exactly the kind of "works on my machine" bug report this
   project's own CI (ubuntu, macos, windows) exists to catch before it
   ships.

**Consequence to state plainly rather than bury:** a long-running program
that opens files in a loop and forgets to close them will exhaust file
descriptors, on every platform, and no collection will save it. That is the
accepted cost of this decision's simplicity, not a gap expected to close on
its own.

## Consequences

- **No new `IoErrorKind` variant for "closed".** `Other` covers the
  closed/stale/forged case; a kind no filesystem condition ever produces on
  its own is not worth widening the `IoErrorKind` wire contract for (the
  numbering `docs/adr/0011-io-error-kinds.md` decision 1 already calls a
  shape that has produced a miscompile once in this project).
- **A forged handle is exactly as safe as a real, closed one.**
  `File { fd: 9999 }` is ordinary, legal Nova source — record fields carry no
  privacy in this language — and it is safe by construction: a fd this
  module never issued misses the table lookup the identical way a closed one
  does.
- **The leak is invisible to this increment's own tests, deliberately.**
  Asserting that a descriptor stays open after `open` with no matching
  `close` is neither portable (the OS's own descriptor accounting differs by
  platform) nor meaningful (a test proving a handle is *not* yet closed
  proves nothing about whether it eventually would be) — this document is
  where the leak is recorded, not a fixture.
- **Revisiting this needs the collector, not `std/fs`.** Either measured
  reason above — the `Int` representation, or the Windows-only stack-bounds
  gap — would have to change first; neither is this increment's to fix, and
  neither is `File`'s alone (the second blocks any collect-time GC hook,
  system-wide, not only this one).

## Alternatives considered

- **A GC-managed cookie in `File`, table keyed on its address.** Matches
  `JoinHandle`'s pointer-identity pattern and would keep close-on-collect
  available the day non-Windows stack bounds land. Declined for simplicity:
  it costs an allocation per `open` that buys nothing today (collection still
  would not run off Windows), and Nova has no opaque-cookie type — the only
  available carrier is `Bytes`, a dishonest field type for something that is
  not byte data. **This is the alternative to revisit first** if descriptor
  leaks turn out to bite in practice.
- **A first-class opaque `Ty` variant for `File`**, the route `Bytes` took
  when it needed one. The most honest representation, and the collector
  could see it directly. Declined on cost — roughly ten compiler seams,
  measured against `Bytes`'s own integration — and because it would
  **reserve the name `File`**, a breaking change this increment otherwise
  does not have (`RESERVED_TYPE_NAMES` stays at 7 throughout).
- **A scope-based `with_file(path, options, body)`** that closes on the way
  out, making the common case leak-proof by construction with no destructor.
  Genuinely safer, and worth building later. Declined here because it is
  surface `nova-spec/20-STDLIB.md` never asked for, error threading through a
  closure needs its own design, and it would widen an increment whose job was
  settling the resource model, not extending the API beyond it.
- **A Windows-only close-on-collect backstop.** Rejected for the platform-
  asymmetry reason the Decision section states: closing automatically on one
  of three supported platforms and leaking unboundedly on the other two is a
  worse thing to ship, silently, than a uniform documented leak.

## References

- Design: `docs/superpowers/specs/2026-08-14-file-open-and-openoptions-design.md`
  §3 ("The resource model"), §6 (the four alternatives above, in the design's
  own words)
- `crates/nova-runtime/src/file.rs`: the handle table (`FILES`, `NEXT_FD`),
  `with_fd`, `nova_rt_file_open`/`close`/`read`/`write`/`flush`, and this
  module's own doc comment (the closedness-is-absence property)
- `crates/nova-runtime/src/gc.rs`: the sweep loop and its
  `task::forget_freed_state` call (the notification hook this decision
  declines to use), `stack_base` (the Windows-only precise-bounds gap)
- `crates/nova-runtime/src/task.rs`: `forget_freed_state`, and (for the
  pointer-identity pattern `File` declined) the `task-identity` branch's
  redesign of `JoinHandle<T>`, recorded in
  `docs/adr/0009-async-execution-model.md` §1
- `std/fs/lib.nova`: `File`, `OpenOptions`, `open`, `close`, and the module's
  own header comment on why `File` is unlike everything else in this module
- Related: `docs/adr/0009-async-execution-model.md` §1 (the identically-shaped
  leak this decision inherits from, for a spawned task's un-taken output —
  argues for treating this as an amendment there; the counter-argument, that
  a foreclosed collector integration for a *second* resource kind is a
  decision in its own right rather than another instance of the same one, is
  why this is a separate document instead), `docs/adr/0011-io-error-kinds.md`
  decision 2 (the deviation `open`/`File` were the last item of, closed by
  the same increment this document belongs to)
