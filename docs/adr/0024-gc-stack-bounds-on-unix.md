# ADR 0024 — The collector finds its stack on glibc Linux and macOS too

## Status

Accepted (2026-10-05). Branch `gc-unix-stack-bounds`
(`docs/superpowers/specs/2026-10-05-gc-unix-stack-bounds-design.md`).

## Context

`crates/nova-runtime/src/gc.rs` is a conservative mark-and-sweep collector.
Its roots include every word between its own frame and the calling thread's
stack top. Until this decision only Windows could find that top
(`GetCurrentThreadStackLimits`). Everywhere else `stack_base()` returned `None`,
`collect()` returned before marking anything, and every allocation lived until
the process exited. A server on Linux or macOS grew with every request it
served, against Phase 2's goal that server-side apps work.

A throwaway CI spike, draft PR #98 (closed unmerged, run 37269987533), turned
collection on for Linux and macOS. The whole blocking suite passed on ubuntu
and macOS, including all 17 `NOVA_GC_STRESS` tests. Those collect on every
allocation, so a missed root would have shown up as a crash or as wrong output.

Two facts constrain any design:

- **A root held only in a callee-saved register is invisible to a stack scan**
  unless the register is written to the stack first.
- **glibc's and Apple's `setjmp` scramble some of the registers they save.**
  glibc's x86-64 `setjmp` stores the saved `rbp`, `rsp` and return address
  mangled; Apple's arm64 `setjmp`, the frame pointer, link register and stack
  pointer. A `setjmp` buffer alone is not a reliable register spill there.

## Decision

1. **Ask the calling thread for its own stack top.** `stack_base()` is still
   cached per thread, in `HEAP.base`:
   - glibc Linux: `pthread_getattr_np` on `pthread_self()`, then
     `pthread_attr_getstack`. The top is the lowest address plus the size.
   - macOS: `pthread_get_stackaddr_np`, which reports the top directly.
   - Windows: `GetCurrentThreadStackLimits`, unchanged.
   - Every other platform: `None`, as before.
2. **On GCC and Clang, spill registers with `__builtin_unwind_init()`.**
   `gc_stack.c`'s `nova_gc_collect_roots` scans from a separate non-inlined
   frame below its own. An empty `asm` that reads the `setjmp` buffer keeps the
   call from becoming a tail call. `setjmp` stays as a second copy, which is the
   shape the spike ran. MSVC's path is unchanged.
3. **A platform without stack bounds says so.** Under `NOVA_GC_DEBUG`, the first
   collection on each such thread prints `nova-gc: no stack bounds on this
   platform; collection is disabled and every allocation leaks`.

## Alternatives considered

- **Record the stack top where a program starts.** A Rust entry wrapper would
  store the address of one of its own locals, then call the Nova program. It
  needs no platform API, so collection would run everywhere. Declined:
  - every way a program starts would need the wrapper: a built executable, the
    JIT under `nova run`, and every runtime unit test on its libtest thread;
  - the base must be recorded in a caller frame, because a frame that records
    its own base can have locals placed above the marker;
  - nothing has measured it, while CI ran this decision.
- **A crate for stack bounds.** The one known here, `stacker`, exposes only the
  space remaining down to the stack's low end. The scan needs the high end.
- **Dropping `setjmp` on the GCC/Clang path.** `__builtin_unwind_init` is enough
  on its own. Declined only because it would ship a shape no CI run has tested.

## Consequences

- **Collection runs on all three CI platforms.** Measured during the design on
  `examples/03-http-server`: 3,000 single-request connections leave 38–109 live
  objects on Windows and on Linux alike, flat. Two tests keep that bounded:
  `http_server_example_keeps_a_bounded_live_set`, and a variant with eight
  concurrent clients.
- **The register spill has no discriminating test.** No test can force a
  pointer to live only in a callee-saved register, so removing
  `__builtin_unwind_init` would probably fail no test. The evidence is the
  stress suite passing with collection live on Linux and macOS, in these
  builds:
  - **Debug**, the only profile CI builds: on Linux and macOS in the spike, and
    on Linux in a local Docker container for this branch.
  - **Release**, on Linux only, in Docker on 2026-10-06 (x86-64, the release
    `nova` at 8,153,144 bytes). The 17 stress tests, `gc_reclaims_garbage`,
    both leak tests and the two stack tests all passed. Optimized runtime code
    is where a root can sit in a callee-saved register such as `rbp`, which
    glibc's `setjmp` scrambles, so this is the profile the spill exists for.
  - **macOS release builds have never collected under any test.**
- **Any other platform still leaks**, as does glibc's main thread where `/proc`
  is unavailable, since glibc reads `/proc/self/maps` for it. The no-bounds line
  names the condition. No CI runner reaches it.
- **ADR 0010's eight GC scan tests are no longer `#[cfg(windows)]`.** They stay
  `#[ignore]`d, and CI's advisory step now runs them on every OS. That step
  gained `--no-fail-fast`, because on ubuntu it had stopped at the first failing
  test binary, before `nova-runtime`. In the spike, on macOS, three of the four
  `is_none()` tests failed, each by over-retention, the direction ADR 0010
  describes.
- **CI's clippy gains a macOS leg.** `gc.rs` now has `#[cfg(target_os =
  "macos")]` code, which no other leg compiles.
- **ADR 0012's second reason no longer holds**: collection runs off Windows.
  Its decision stands on its first reason.

## References

- Spec: `docs/superpowers/specs/2026-10-05-gc-unix-stack-bounds-design.md`
- Spike: https://github.com/Sakeerin/nova/pull/98 (closed unmerged; branch
  `spike-ci-gc-unix`, commit `9aad29f`)
- `crates/nova-runtime/src/gc.rs`: `stack_base`, `collect`, `nova_gc_scan_range`
- `crates/nova-runtime/src/gc_stack.c`: the register spill
- `docs/adr/0002-phase1-leaking-allocator.md`,
  `docs/adr/0010-conservative-scan-root-test-gating.md`,
  `docs/adr/0012-file-descriptor-lifecycle.md`
