# Garbage collection on Linux and macOS: design

Branch `gc-unix-stack-bounds`, from `main` at `a4eb388`. The design was approved
section by section in conversation on 2026-10-05; this file is the written spec.

## 1. What this is, and what it is not

**The goal.** Nova's collector has never run off Windows. `gc.rs`'s
`stack_base()` returns `None` on every other platform, `collect()` then returns
before marking anything, and every allocation lives until the process exits.
Phase 2's goal is "server-side apps work"; on Linux and macOS a server's memory
grows with every request it serves. The 2026-10-05 Phase 2 assessment named this
the material gap. After this change the collector runs on glibc Linux, macOS and
Windows: the three platforms CI tests.

**The user's decisions, 2026-10-05:**
- Fix the GC first, then close out Phase 2. The close-out (ADRs for the deferred
  modules, `20-STDLIB` sections for `std/strings` and `std/bytes`, the `v0.2.0`
  tag) is separate work.
- Run a throwaway CI spike before any spec. It ran as draft PR #98, closed
  unmerged (§2).
- "Done" means the spike's checks kept permanently, on all three OSes, **plus** a
  CI test that a long server workload keeps a bounded live set.
- That workload is the `examples/03-http-server` server.
- **Approach A** (§3.6): ask the calling thread for its own stack bounds.
- Linux runs happen locally in Docker (the official `rust:1-slim` image), so each
  Unix test can be watched failing before the fix. macOS runs only in CI.
- Design sections 1 to 4 were approved as presented: the runtime (§3), the tests
  and the CI change (§5, apart from §5.3), the leak test (§5.3), and the records
  (§6). §4 and §7 to §9 are written from those sections.

**Not in scope:**
- Platforms other than the three CI tests: musl, the BSDs, others. They keep
  skipping collection, now visibly (§3.4).
- A precise or moving collector. Scanning stays conservative.
- The MSVC path of `gc_stack.c`, which stays byte for byte as it is.
- ADR 0012's decision about file descriptors, whose second reason this change
  removes (§6). The decision is not revisited here.
- `extern_ffi_run`'s Linux failure (issue #3).
- The advisory GC scan tests' over-retention on macOS (§2.3), which ADR 0010
  already accepts.
- The LLVM backend.

## 2. Evidence: the spike and the measurements

### 2.1 The spike, PR #98

Branch `spike-ci-gc-unix`, commit `9aad29f`, CI run 37269987533. The PR was read
once and closed unmerged; the branch is kept so its code stays readable. It
added the §3.2 bounds and the §3.3 register spill, lifted the Windows-only gates
named in §5.2, and added a stack-bounds sanity test.

Blocking `cargo test` step, passed / failed / ignored, against PR #97's run:

| OS | PR #97 (no collection off Windows) | PR #98 (collection on) |
|---|---|---|
| ubuntu | 1211 / 0 / 1 | 1213 / 0 / 9 |
| macOS | 1212 / 0 / 0 | 1214 / 0 / 8 |
| windows | 1218 / 0 / 8 | 1219 / 0 / 8 |

- The changes are exactly what the spike added. The +2 passed are
  `gc_reclaims_garbage` and the sanity test. The +8 ignored are the GC scan
  tests, which stay `#[ignore]`d.
- All 17 tests that set `NOVA_GC_STRESS` (all in `run_tests.rs`) passed on all
  three OSes. On Linux and macOS this was the first time they collected on every
  allocation: `stress()` calls `collect()` each time, and `collect()` skips only
  when the stack base is unknown. No program crashed or printed wrong output.
- `gc_reclaims_garbage` passed on Linux and macOS: at least one collection ran,
  and fewer than 1,000 objects stayed live.
- `nova run` runs the program on the process's main thread: none of `nova-cli`,
  `nova-driver` or `nova-codegen-cranelift` spawns a thread. So these runs used
  glibc's main-thread path, which reads `/proc/self/maps`.
- Blocking-step time: ubuntu 80 → 84 s, macOS 46 → 49 s.

### 2.2 Clippy

Clippy failed on ubuntu and windows over one lint, in the spike's own new test:
`assertions_on_constants` on `None => assert!(!cfg!(any(...)))`. Clippy had not
been run locally before pushing. §5.1's tests are gated with `#[cfg]` instead.
Rustfmt and MSRV passed.

### 2.3 The advisory `--ignored` step

- **macOS:** 5 of the 8 GC scan tests passed. The 4 that check a registered root
  is kept all passed. 3 of the 4 that check an unreachable object is freed
  failed, every one because the object survived. Each panic line below is in the
  spike commit `9aad29f`; the function's line on this branch follows it:
  - `an_unregistered_parent_and_child_are_swept`: panicked at `gc.rs:1451`;
    the function is at `gc.rs:1366`;
  - `remove_root_actually_unroots`: panicked at `gc.rs:1304`; the function is at
    `gc.rs:1208`;
  - `an_unspawned_tasks_state_is_swept`: panicked at `task.rs:5492`; the
    function is at `task.rs:5481`.

  Keeping too much is the safe failure, and it is the reason ADR 0010 gives for
  ignoring these tests.
- **ubuntu:** the step never reached them. `extern_ffi_run`, ignored on Linux
  only ("E0902 at JIT time on Linux; see issue #3"), runs in this step and fails.
  The step has no `--no-fail-fast`, so cargo stopped after `nova-cli`'s
  `run_tests`, the 5th of about 50 test binaries, before `nova-runtime`. PR #97's
  ubuntu run stops at the same test, so the gap predates the spike. `continue-on-error`
  keeps the job green whatever happens, so only the count of `test result:` lines
  shows it: 5 on ubuntu, 24 on macOS, 50 on windows.
- **windows:** 8 of 8 passed, as before.

### 2.4 The 03 server's live set, measured

The measurement uses a throwaway client, not committed: it starts
`nova run examples/03-http-server/src/main.nova` with `NOVA_GC_DEBUG=1`, sends
`GET / HTTP/1.1\r\nhost: x\r\n\r\n` requests, reads each response by its
`content-length`, closes the connection, kills the server, and reads each line
`nova-gc: collection C freed F bytes, L objects live (B bytes)`. Both binaries
are debug builds, as the tests use.

| Run | Source | Collections | Live objects, min–max | Highest per block, in order |
|---|---|---|---|---|
| Windows, 3,000 connections × 1 request | `main` `a4eb388`, `nova.exe` 15,363,584 bytes | 22 | 38–104 | 99, 104, 99, 104, 99 |
| Windows, 300 connections × 10 requests | same | 17 | 68–109 | 109, 104, 109, 108, 90 |
| Linux (Docker), 3,000 × 1 | spike `9aad29f`, `nova` 98,170,408 bytes | 22 | 38–104 | 96, 104, 104, 99, 79 |
| Linux (Docker), 300 × 10 | same | 17 | 47–109 | 109, 109, 109, 109, 95 |

A block is 5 consecutive collections in the 22-collection runs and 4 in the
17-collection runs; the last block is the remainder. The live set is flat on
both OSes, and the same on both: the program allocates deterministically. 3,000 single-request connections took 1.7 s on Windows and
0.7 s on Linux.

## 3. Architecture

### 3.1 Constraints the design is built around

- The collector is conservative, non-moving mark-sweep. Its roots are every word
  between the collector's own frame and the thread's stack top, plus the
  registered roots in `PINNED`. A wrong stack top fails in one of two ways. Too
  low, and the scan misses caller frames: live objects are freed and the program
  later reads freed memory. Too high, and the scan reads unmapped memory and the
  process crashes.
- A root held only in a callee-saved register when the collector runs must be
  written to the stack before the scan. Caller-saved registers need nothing: by
  the C ABI the compiled code has already spilled any live value out of them
  before calling the allocator.
- `HEAP` is `thread_local!`, and the stack top is cached in it per thread
  (`HEAP.base`; `0` means not captured yet, `usize::MAX` means none).
- CI runs ubuntu-latest (x86-64, glibc), macos-latest (arm64) and windows-latest
  (MSVC). The MSRV is 1.78.
- No new dependency: `libc` is already a `cfg(unix)` dependency of
  `nova-runtime`, so `Cargo.lock` does not change.

### 3.2 `stack_base()` per platform (`crates/nova-runtime/src/gc.rs`)

Each version returns the calling thread's stack top, meaning its highest address,
or `None`.

```rust
#[cfg(windows)]
fn stack_base() -> Option<usize> { /* unchanged: GetCurrentThreadStackLimits */ }

/// The calling thread's stack top on glibc Linux, from `pthread_getattr_np`:
/// its lowest address plus its size.
#[cfg(all(target_os = "linux", target_env = "gnu"))]
fn stack_base() -> Option<usize> {
    // SAFETY: `pthread_getattr_np` fills `attr` for the calling thread, and
    // `pthread_attr_getstack` reads the stack's lowest address and size from
    // it. `attr` is destroyed before returning.
    unsafe {
        let mut attr: libc::pthread_attr_t = std::mem::zeroed();
        if libc::pthread_getattr_np(libc::pthread_self(), &mut attr) != 0 {
            return None;
        }
        let mut addr: *mut libc::c_void = std::ptr::null_mut();
        let mut size: libc::size_t = 0;
        let rc = libc::pthread_attr_getstack(&attr, &mut addr, &mut size);
        libc::pthread_attr_destroy(&mut attr);
        if rc != 0 || addr.is_null() || size == 0 {
            return None;
        }
        Some(addr as usize + size)
    }
}

/// The calling thread's stack top on macOS, which `pthread_get_stackaddr_np`
/// reports directly.
#[cfg(target_os = "macos")]
fn stack_base() -> Option<usize> {
    // SAFETY: reads the calling thread's own stack top.
    let top = unsafe { libc::pthread_get_stackaddr_np(libc::pthread_self()) };
    (!top.is_null()).then_some(top as usize)
}

#[cfg(not(any(
    windows,
    all(target_os = "linux", target_env = "gnu"),
    target_os = "macos"
)))]
fn stack_base() -> Option<usize> {
    None
}
```

- On glibc's main thread, `pthread_getattr_np` finds the stack by reading
  `/proc/self/maps`. The top it reports is the end of the `[stack]` mapping,
  above `argv` and the environment. Scanning those words adds false roots only.
- On other glibc threads, the top is the end of the thread's whole stack block,
  which includes glibc's thread descriptor and static TLS above the stack itself.
  Those are mapped memory too.

### 3.3 The register spill (`crates/nova-runtime/src/gc_stack.c`)

glibc's x86-64 `setjmp` scrambles the saved `rbp`, `rsp` and return address, and
Apple's arm64 `setjmp` scrambles the saved frame pointer, link register and stack
pointer. A root held only in one of those would not look like an address in the
saved copy. On GCC and Clang the shim becomes:

```c
static __attribute__((noinline)) void nova_gc_scan_from_below(void *stack_base) {
    void *volatile marker = 0;
    nova_gc_scan_range((void *)&marker, stack_base);
}

void nova_gc_collect_roots(void *stack_base) {
    jmp_buf regs;
    __builtin_unwind_init();
    setjmp(regs);
    nova_gc_scan_from_below(stack_base);
    __asm__ __volatile__("" : : "r"(&regs) : "memory");
}
```

- `__builtin_unwind_init()` makes `nova_gc_collect_roots` save every callee-saved
  register in its own frame.
- The scan starts in a separate non-inlined function, so it begins below that
  frame and covers all of it.
- The empty volatile `asm` reads `&regs` after the call, so the call cannot
  become a tail call that pops this frame before the scan.
- `setjmp` stays as a second copy. That is the exact shape #98 ran, and it can't
  be the only spill on these two platforms.
- The `#else` branch, used by MSVC, is today's code unchanged.
- The header comment is rewritten to describe both paths.

### 3.4 Platforms without stack bounds

`collect()` captures the stack top once per thread. When `stack_base()` returns
`None`, it records `usize::MAX` as today, and also prints this once if
`NOVA_GC_DEBUG` is set:

```text
nova-gc: no stack bounds on this platform; collection is disabled and every allocation leaks
```

```rust
if h.base == 0 {
    h.base = stack_base().unwrap_or_else(|| {
        if debug() {
            eprintln!(
                "nova-gc: no stack bounds on this platform; \
                 collection is disabled and every allocation leaks"
            );
        }
        usize::MAX
    });
}
```

The line is printed once per thread because later collections find `h.base`
already set. `eprintln!` allocates only through Rust's system allocator, never
the GC heap, so it cannot re-enter the collector. No CI platform reaches this
branch, so no test executes it (§8).

### 3.5 `collect_for_test`, and the Windows-only gates

- `gc.rs`'s `collect_for_test` loses `#[cfg(windows)]`. It keeps
  `#[cfg(test)]` and `#[inline(always)]`.
- The `mod registry` tests in `gc.rs` and the `mod root_registration` tests in
  `task.rs` lose `#[cfg(windows)]`.
- `run_tests.rs`'s `gc_reclaims_garbage` loses `#[cfg(windows)]`.
- Every doc comment that justified one of those gates is rewritten.

### 3.6 Alternatives considered

- **B: record the stack top where a program starts.** A Rust entry wrapper would
  store the address of one of its own locals, then call the Nova program. It
  needs no platform API, so collection would run on every platform. Declined:
  - every way a program starts would need the wrapper: a built executable, the
    JIT under `nova run`, and every runtime unit test on its libtest thread;
  - the base must be recorded in a caller frame, because a frame that records
    its own base can have locals the compiler placed above the marker;
  - nothing has measured it, while CI ran A.
- **A crate for stack bounds.** The one known here, `stacker`, exposes only the
  space remaining down to the low end of the stack, which is what stack-overflow
  checks need. The scan needs the high end. No other crate was surveyed.
- **Dropping `setjmp` on the GCC/Clang path.** `__builtin_unwind_init` is enough
  on its own. Declined only because it would ship a shape no CI run has tested.

## 4. Error handling

- **`pthread_getattr_np` fails** (glibc's main thread without `/proc`), or
  `pthread_get_stackaddr_np` returns null: `stack_base()` returns `None`. The
  thread never collects, as on any unsupported platform, and §3.4's line says why.
- **A garbage top that is still non-null** (above the mapping, or below the
  frame) would crash or free live objects (§3.1). §5.1's two tests check that
  the top is above the caller's frame and within a plausible distance of it, on
  the test thread and on a spawned thread.
- The top is computed once per thread and never recomputed.

## 5. Testing

### 5.1 Runtime unit tests (`gc.rs`'s `mod tests`)

Both tests are gated with
`#[cfg(any(windows, all(target_os = "linux", target_env = "gnu"), target_os = "macos"))]`,
not with an `assert!` on a `cfg!`. That `assert!` is the lint §2.2 hit.

```rust
/// On every platform that implements stack bounds, the top lies above this
/// frame, and within a plausible stack size of it.
#[test]
fn stack_base_lies_above_the_current_frame() {
    let local = 0u8;
    let here = std::ptr::addr_of!(local) as usize;
    let base = stack_base().expect("this platform implements stack bounds");
    assert!(base > here, "top {base:#x} must lie above this frame {here:#x}");
    assert!(base - here < 1 << 30, "top {base:#x} is implausibly far above {here:#x}");
}

/// The top is the calling thread's own: on a thread spawned with a 256 KiB
/// stack, it lies less than 1 MiB above a local in that thread.
#[test]
fn stack_base_is_the_calling_threads_own() {
    const STACK: usize = 256 * 1024;
    let (base, here) = std::thread::Builder::new()
        .stack_size(STACK)
        .spawn(|| {
            let local = 0u8;
            (stack_base(), std::ptr::addr_of!(local) as usize)
        })
        .expect("spawn a thread")
        .join()
        .expect("join it");
    let base = base.expect("this platform implements stack bounds");
    assert!(base > here, "top {base:#x} must lie above {here:#x}");
    assert!(base - here < 4 * STACK, "top {base:#x} is not this thread's: {here:#x}");
}
```

A version that returned the main thread's stack, or another thread's cached top,
fails the second test: thread stacks are separate mappings, far more than 1 MiB
apart.

### 5.2 Tests no longer Windows-only

- `gc_reclaims_garbage` runs on all three OSes.
- `gc.rs`'s `mod registry` (6 tests) and `task.rs`'s `mod root_registration`
  (2 tests) stay `#[ignore]`d under ADR 0010, so they run only in the advisory
  step, now on every OS.
- The 17 `NOVA_GC_STRESS` tests need no change. On Unix they now really collect
  on every allocation.
- The parser for `nova-gc: collection` lines moves out of `gc_reclaims_garbage`
  into a helper that it and §5.3's test share:
  `fn gc_live_object_counts(stderr: &str) -> Vec<u64>`.

### 5.3 The 03 server leak test (`crates/nova-cli/tests/run_tests.rs`)

`http_server_example_keeps_a_bounded_live_set`, next to the other 03 end-to-end
tests. Like them, it runs under `nova run` only.

- **Harness:** `Http03Server` gains `spawn_with_env(env: &[(&str, &str)])`, and
  `spawn()` becomes `spawn_with_env(&[])`.
- **Setup:** the test takes `lock_port_3000()`, calls `assert_port_3000_free()`,
  starts the server with `NOVA_GC_DEBUG=1`, and waits until it accepts
  connections.
- **Workload:** 3,000 requests, each `GET / HTTP/1.1\r\nhost: x\r\n\r\n` on a new
  `Http03Conn` that the client drops after reading the response through
  `http03_response`. That creates and tears down 3,000 connection tasks, which is
  where a leaked task root would build up.
  - 3,000 client ports in TIME_WAIT fit easily in every OS's ephemeral range:
    16,384 on Windows and macOS, about 28,000 on Linux.
- **Shutdown:** `send_termination_request(pid)`, then `wait_for_exit(10 s)`. The
  test then reads stderr through `streams()`.
- **Assertions:**
  1. Every response's status is 200.
  2. At least 10 collections ran. The measured run logs 22 (§2.4), so the bound
     below is checked across the whole run, not only at its start.
  3. Every collection reports fewer than 1,000 live objects. That is about 10×
     the measured maximum of 104–109, which leaves room for macOS's extra
     retention, measured nowhere yet. Leaking even one object per connection
     would pass 1,000 after about 1,000 connections, so 3,000 gives a 3× margin.
- **Stated limit:** a leak smaller than about one object per three connections
  could pass.

### 5.4 Proving the tests can fail

- **On Linux, before the fix** (Docker, on `main`'s runtime): §5.1's two tests
  fail at `expect` with `None`. `gc_reclaims_garbage` fails because it finds no
  `nova-gc: collection` line. §5.3's test fails assertion 2 with 0 collections.
  Each failure is watched, and its message read.
- **macOS failing first is not observed.** macOS runs only in CI. It runs the
  same pthread-plus-shim design, and its passing is checked on the PR's CI.
- **On Windows** the tests pass before the fix, since Windows already collects.
  Two mutants, neither committed, show §5.3's test discriminates:
  - (a) Windows' `stack_base()` returns `None`: the test must fail assertion 2.
  - (b) `gc::remove_root(state)` is skipped in `release_internal`
    (`task.rs:901`), so finished tasks stay rooted: the test must fail
    assertion 3.
    - That is the path the 03 server's connection tasks take. They are spawns
      (`let _ = spawn(serve(self, conn))`, `std/http/lib.nova:881`), and
      `poll_one` releases every finished task's root through `release_internal`
      (`task.rs:824`).
    - The one exception is `block_on`'s root task. It keeps its root until
      `take_output_internal` (`task.rs:969`) takes its output (`task.rs:155–161`,
      `:932–937`).
  - Each mutant must fail the assertion named for it, read from the panic
    message. A mutant that aborts the server, or fails somewhere else, proves
    nothing about that assertion.
- **The register spill (§3.3) has no discriminating test.** No test can force a
  pointer to live only in a callee-saved register, so removing
  `__builtin_unwind_init` would probably fail none. The records say so (§6)
  rather than claim coverage.

### 5.5 CI (`.github/workflows/ci.yml`)

- Add `--no-fail-fast` to the advisory step:
  `cargo test --locked --workspace --all-features --no-fail-fast -- --ignored`.
  A failing test binary then no longer stops the step before `nova-runtime`.
  Today that binary is `run_tests`, through `extern_ffi_run` on Linux.
- Rewrite the step's comment (`ci.yml:42–69`). It says "All eight are
  `#[cfg(windows)]`, so this filters to zero tests on the ubuntu and macos legs",
  and that once `gc::stack_base` grew a non-Windows version the step would cover
  them everywhere with no CI change. §2.3 showed the second claim false on
  ubuntu.
- **Add `macos-latest` to the clippy matrix.** This was added after the written
  spec's verification, not in the approved sections. `ci.yml:91–92` justifies
  having no macOS leg because "the tree has no `target_os = "macos"` or
  `target_vendor = "apple"` cfg at all, so macos would lint exactly what ubuntu
  does". §3.2's `#[cfg(target_os = "macos")]` makes that false: without the leg,
  no CI job would lint the macOS `stack_base`. The comment is rewritten to
  match. CI then reports 8 checks instead of 7.

Before pushing:
- `cargo test`, `cargo clippy --locked --all-targets --all-features -- -D warnings`
  and `cargo fmt --all -- --check`, on Windows and in the Linux container.
- The macOS `stack_base` is type-checked and clippy'd through a scratch crate for
  `aarch64-apple-darwin`, as in the spike. `gc_stack.c` cannot be cross-compiled
  for macOS on this host. CI's new macOS clippy leg is the real check.

### 5.6 Predicted CI counts

Blocking `cargo test` step, against PR #97 (passed / failed / ignored), to be
checked against the PR's run:

| OS | PR #97 | Predicted | Change |
|---|---|---|---|
| ubuntu | 1211 / 0 / 1 | 1215 / 0 / 9 | +4 passed (`gc_reclaims_garbage`, 2 stack tests, the leak test), +8 ignored |
| macOS | 1212 / 0 / 0 | 1216 / 0 / 8 | as ubuntu |
| windows | 1218 / 0 / 8 | 1221 / 0 / 8 | +3 passed (2 stack tests, the leak test) |

The advisory step should then show about 50 `test result:` lines on every OS. CI
reports 8 checks instead of 7, with the new `Clippy (macos-latest)`.

**Amended 2026-10-05 (plan `docs/superpowers/plans/2026-10-05-gc-unix-stack-bounds.md`):**
the plan adds a second leak test, for §5.7's item 3:
`http_server_example_keeps_a_bounded_live_set_under_concurrent_clients`, with
8 client threads of 375 connections each, and every collection under 2,000
live objects. It was measured on Linux while the plan was written (spike
`9aad29f`, debug `nova`): 61–544 live objects across 22 collections, flat.
1, 4 and 16 clients gave 39–104, 88–294 and 77–1,052. Each predicted count
above gains one passed test: ubuntu 1216 / 0 / 9, macOS 1217 / 0 / 8, windows
1222 / 0 / 8.

### 5.7 Review Focus

The five conditions most likely to bite a person using this, which no test above
exercises:
1. **A pointer held only in a callee-saved register when the collector runs**
   must keep its object alive. Covered by reasoning (§3.3), not by any test.
2. **glibc's main thread without `/proc`** (some containers and chroots) finds no
   bounds and leaks. §3.4's line names it; no CI runner lacks `/proc`.
3. **Concurrent connections.** §5.3 sends its requests one at a time. A server
   under parallel load keeps more tasks alive at once. The 05 benchmark measured
   10 concurrent users on Windows only.
4. **macOS's over-retention** (§2.3) is expected to stay bounded rather than grow.
   Only §5.3's test running in CI checks that.
5. **Threads with unusual stacks**, such as signal handlers on an alternate
   stack. Nova's signal handlers touch one atomic and never allocate (ADR 0022),
   so no collection can start on such a stack today.

## 6. Records to amend

- **New `docs/adr/0024-gc-stack-bounds-on-unix.md`.** It records:
  - the decision (§3) and the alternatives (§3.6);
  - the consequences:
    - other platforms still skip collection, visibly (§3.4);
    - the register spill can't be tested;
    - on macOS, 3 advisory tests fail because objects survive;
    - the advisory step gains `--no-fail-fast`;
  - the evidence (§2, §5.3).
- **Specs and plan, corrected in place:**
  - `nova-spec/13-RUNTIME.md`: its GC section (§3, `:144–253`) gains which
    platforms collect and how each finds its stack top. It says nothing about
    platforms today. `:165`'s "flushed onto the stack by a `setjmp` shim" is
    rewritten for §3.3.
  - `nova-spec/20-STDLIB.md:766`: "off Windows the collector is a no-op".
  - `docs/phase-2-plan.md:35`, `:335` and `:348`: the "non-Windows GC stack
    bounds" item, marked done.
- **ADRs, dated notes, bodies unchanged:**
  - `0002:9–13`, `0013:119` and `:255–256`, `0018:591` and `0020:42–43`.
  - `0009:439–440`: "the platform gap that would make such a backstop dishonest"
    is ADR 0012's reason 2, which no longer holds. ADR 0009 never names Windows,
    so a phrase search misses this one.
  - `0010:29`, `:38–39` (the table naming `#[cfg(windows)] mod registry` and
    `#[cfg(windows)] mod root_registration`) and `:47–50`: the eight tests now
    run on every OS in the advisory step, with the macOS result.
  - `0012:75` ("Close-on-collect is foreclosed for two measured reasons"),
    `:99–109` (reason 2, "Collection does not run at all off Windows"), `:136`,
    `:145–147`, `:163–165` and `:179`. Reason 2 no longer holds. The decision
    stands on its one remaining reason, reason 1 (`fd: Int`, `:78`), and isn't
    revisited here.
- **Code and test comments**, rewritten by the tasks that touch these files. The
  first list makes the claim outright; the second describes the `setjmp`-only
  spill that §3.3 replaces, or a gate §3.5 removes.
  - **The claim:**
    - `gc.rs:51–53`, `:321–326` ("where a `collect()`-based assertion is
      neither"), `:359–365`, `:378–386`, `:604–609`, `:711–713`, `:917–922`,
      `:975–1009`, `:1022` and `:1341`. `:975–992`'s own line references
      (`:419`, `:432`, `:264`, `:273-275`) are stale already; they point at
      `:591`, `:604`, `:408` and `:417–420`.
    - `task.rs:281–283`, `:2895–2897` and `:5342–5350`.
    - `fs.rs:384–389`, which contrasts its helper with `gc::collect_for_test`,
      "whose *only* callers genuinely are `#[cfg(windows)]`".
    - `crypto.rs:79–82`: "precise stack bounds are implemented on Windows only".
    - `run_tests.rs:1231–1232` and `:8678–8679` ("It discriminates only where the
      collector frees memory, which is Windows."), `:1265–1267` and `:1628–1630`.
    - `ci.yml:42–69` and `:91–92` (§5.5).
    - `std/json/lib.nova:204–205` and `tests/runtime/recycled_task_state.nova:37–41`.
  - **The `setjmp`-only spill:**
    - `gc_stack.c:1–13`, its header;
    - `build.rs:1–4`, which says "`setjmp` is portable";
    - `gc.rs:15–16`, `:566–568`, `:1002–1009` (the scan's low end is "`&regs`"),
      `:1039–1044` and `:1113–1118`;
    - `std/collections/lib.nova:419`.
- **`CHANGELOG.md`:** one entry under Unreleased. Five older statements sit in
  released sections (0.1.0, alpha.1, alpha.2) and stay as history: `:2950`,
  `:4260`, `:4721–4722`, `:4806–4807` and `:4910–4911`.
- **Left alone:**
  - the dated design specs and plans under `docs/superpowers/`, which are records
    of their time;
  - `examples/05-json-api/BENCHMARK.md`, whose "One host, Windows" lines label
    measurements and claim nothing about other platforms.
- **How this list was built.** It is the union of a line-based phrase grep, a
  read-only check of every claim by a fresh agent, and a proximity sweep: every
  line naming a platform within three lines of a GC word, plus every `setjmp`
  mention. Each method found entries the others missed. For example,
  `crypto.rs:79–82` splits "stack bounds" across two lines, and `ci.yml:91–92`
  has no GC word near it.
- **Sweep at the end of the records task:**
  1. Re-run all three searches: the phrases below, the proximity sweep, and
     `git grep -n 'target_os = "macos"\|target_vendor'` for claims that no macOS
     `cfg` exists.
  2. Take the set difference: every file still matched, minus the files the
     branch touched.
  3. Every leftover file, and every remaining match inside a touched file, is
     either fixed or explained in the PR.

  The phrases: `off Windows`, `non-Windows`, `Windows-only`, `Windows only`,
  `only on Windows`, `Windows alone`, `which is Windows`, `collection is skipped`,
  `skips collection`, `no-op off`, `collector is a no-op`, `stack bounds`,
  `stack-bounds`, `precise bounds`, `stack_base`, `returns before marking`,
  `Windows gate`, `setjmp`.

## 7. Risks

- **The register spill is covered by reasoning, not measurement** (§5.4). The
  17 stress tests passing on Linux and macOS is evidence that no root went
  missing in those programs, not proof for every program.
- **The leak test's bound is unmeasured on macOS.** It has about 10× headroom
  over the Windows and Linux maximum, and macOS over-retains more (§2.3).
- **TIME_WAIT ports.** The test leaves up to 3,000 client ports in TIME_WAIT,
  for minutes on Windows (its `TcpTimedWaitDelay`). A rerun inside that window
  holds 6,000 of Windows' 16,384.
- **Clippy drift.** The container's Rust 1.99.0 matches CI's stable today. CI's
  stable can move ahead of it.
- **Downloads.** Building in the Linux container fetches crates from crates.io,
  as CI does. The image itself (`rust:1-slim`, 330 MB compressed) was pulled with
  the user's approval on 2026-10-05.

## 8. What is not covered

- §3.4's line: no CI platform lacks stack bounds.
- Collection on macOS before the fix (§5.4).
- The register spill (§5.4).
- Concurrent server load on Linux or macOS (§5.7, item 3).
- A built executable's server under load. §5.3 runs under `nova run`, as 03's
  other end-to-end tests do. The 26 `*_build_standalone` tests run built
  executables, and none of their names mentions http, server, net, listen,
  socket or tcp.

**Amended 2026-10-06 (final review):** every run above used debug builds, the
only profile CI builds, so release builds were uncovered too. That matters
because the register spill (§3.3) exists for optimized code. A release-profile
run on Linux in Docker (x86-64) then passed:
- the 17 stress tests, `gc_reclaims_garbage`, both leak tests and the two stack
  tests;
- `nova_test_under_gc_stress` once the release runtime library was built
  (`cargo build --release -p nova-runtime`). `nova test` links that library,
  and `cargo test --release` alone does not build it.

One release-only failure is not this branch's:
`task::tests::the_yield_futures_layout_is_the_one_the_abi_declares` fails on
`main` (`a4eb388`) the same way, with the same 153,040-byte gap between the two
function addresses it compares. macOS release builds remain uncovered.

## 9. Success criteria

1. On ubuntu and macOS CI, the blocking `cargo test` step passes with collection
   live, matching §5.6's prediction:
   - `gc_reclaims_garbage`, §5.1's two tests and §5.3's test pass on all three
     OSes;
   - the 17 `NOVA_GC_STRESS` tests pass on all three.
2. §5.1's two tests, `gc_reclaims_garbage` and §5.3's test were each watched
   failing on Linux before the fix (§5.4). Mutants (a) and (b) each failed §5.3's
   test on Windows.
3. Clippy with `-D warnings` passes on all three OSes, macOS included through
   its new leg, and `cargo fmt --check` passes.
4. The advisory step runs every test binary on every OS (about 50 `test result:`
   lines each).
5. The records in §6 are amended, ADR 0024 exists, and the sweep leaves no
   unexplained file.
