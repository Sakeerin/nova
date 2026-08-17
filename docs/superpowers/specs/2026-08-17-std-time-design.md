# `std/time` — design

**Status:** approved 2026-08-17. Base: `main` == `origin/main` == `25c9b43`, tagged `v0.2.0-alpha.1`, 437 commits, 0 merge commits, 973 tests (8 deliberately ignored), clean tree.

**Goal.** Give Nova a clock and a duration type: `nova-spec/20-STDLIB.md` §9 minus its two async combinators, plus the relocation of `sleep` off `std/task` and onto a real `Duration`.

**Approach in one line.** `Instant` and `Duration` are ordinary Nova records over an `Int` of nanoseconds; the runtime contributes exactly one new intrinsic; every arithmetic operation is Nova code.

---

## 1. Scope

### In

- A new embedded module `std/time` with `Instant`, `Duration`, and `sleep(d: Duration)`.
- One new runtime intrinsic: `time_now_nanos() -> Int`.
- Retyping the sleep parker from milliseconds to nanoseconds, and renaming it so the change cannot be silent.
- A timeout-conversion invariant in `poll.rs` that sub-unit durations must not become zero timeouts, made testable.
- A structural test accessor for the staged deadline, which does not exist today.

### Out, deliberately

- **`timeout<T>(d, fut) -> Result<T, TimeoutError>`** and `TimeoutError`. §9 lists them; they are their own increment. Measured reasons: `try_stage` (`crates/nova-runtime/src/task.rs:495`) treats a second deadline as a collision and a collision calls `abort_with`, so `timeout(d, sleep(..))` (Deadline+Deadline) and `timeout(d, handle.join())` (Task+Deadline) would abort the process; `poll_sleep` is edge-triggered where `poll_join` is level-triggered, so a merged earlier wake would make a sleep report completion it has not earned; and nothing defines what happens to an abandoned inner future's socket registration or GC root. Those are executor-semantics questions and belong with the widening that answers them.
- **Wall-clock time.** §9 specifies a monotonic `Instant` only. `std/log` will eventually want a timestamp, which is a wall clock; adding one now is speculation, so it waits for the increment that needs it.
- **`as_micros`, `as_nanos`, `from_nanos`, `Duration` arithmetic operators, `Instant` ordering.** Not in §9. YAGNI.

---

## 2. Representation

```nova
pub record Instant { nanos: Int }
pub record Duration { nanos: Int }
```

Both fields are **private** — no `pub` — which is what makes them opaque as §9 requires, and it matches `std/net`'s `TcpStream { fd: Int }` exactly. `std/time`'s own `impl` blocks reach the field because they are in the same module.

`Instant.nanos` is nanoseconds since a single process epoch, so every value is non-negative and two `Instant`s are comparable by subtraction. `Duration.nanos` is a nanosecond count.

**Why nanoseconds.** All three §9 constructors (`from_secs`, `from_millis`, `from_micros`) are exactly representable with no precision loss, and `elapsed()` is a measurement whose precision would be thrown away by a coarser unit. i64 nanoseconds spans ~292 years, which bounds nothing a program will build deliberately; §8 covers what happens when it does.

**Why records rather than builtin types.** `hir::Ty::Record { .. }` maps to `MirTy::Ptr` (`crates/nova-mir/src/lib.rs:811`), so each `Duration` heap-allocates. That cost is accepted: it is true of every record in Nova today, and nothing here is on a hot path. The alternative — `Instant`/`Duration` as `hir::Ty` variants over `MirTy::I64`, unboxed, joining `RESERVED_TYPE_NAMES` 7→9 — is a compiler-wide change and a second breaking rename, which `Bytes` earned only because it had a real representation (a scanned `{len,ptr}` header over a GC leaf buffer). An integer does not need one. `RESERVED_TYPE_NAMES` therefore stays at **7**.

---

## 3. Nova surface

`std/time/lib.nova`, complete:

```nova
module std.time

pub record Instant { nanos: Int }
pub record Duration { nanos: Int }

impl Instant {
    pub fn now() -> Instant
    pub fn elapsed(self) -> Duration
    pub fn duration_since(self, earlier: Instant) -> Duration
}

impl Duration {
    pub fn from_secs(s: Int) -> Duration
    pub fn from_millis(ms: Int) -> Duration
    pub fn from_micros(us: Int) -> Duration
    pub fn as_secs(self) -> Int
    pub fn as_millis(self) -> Int
}

pub async fn sleep(d: Duration)
```

Semantics, each one line of Nova over the field:

| Function | Definition |
|---|---|
| `Instant::now()` | `Instant { nanos: time_now_nanos() }` |
| `Instant::elapsed(self)` | `Instant::now().duration_since(self)` |
| `Instant::duration_since(self, earlier)` | `self.nanos - earlier.nanos`, **saturated at 0** |
| `Duration::from_secs(s)` | `s * 1_000_000_000`, saturating (§8) |
| `Duration::from_millis(ms)` | `ms * 1_000_000`, saturating |
| `Duration::from_micros(us)` | `us * 1_000`, saturating |
| `Duration::as_secs(self)` | `self.nanos / 1_000_000_000` |
| `Duration::as_millis(self)` | `self.nanos / 1_000_000` |
| `sleep(d)` | `task_sleep_future_nanos(d.nanos).await` |

Both spellings are **measured, not assumed**: no-receiver associated functions already work in this stdlib (`Vec::new()`, `OpenOptions::reading()`), and chaining a by-value `self` method on a temporary type-checks — a probe of `W::make().get()` returned `ok`, so `Instant::now().elapsed()` is legal. The `E0060` restriction that blocked `OpenOptions`' chainable builder applies only to **receiver-mutating** methods. Underscored numeric literals and integer division are also confirmed to work.

---

## 4. Runtime

A new file `crates/nova-runtime/src/time.rs`, following the one-responsibility-per-module shape of `net.rs`, `file.rs`, `io.rs` and `poll.rs`.

```rust
/// The single process-monotonic origin every clock reading is relative to.
pub(crate) fn epoch() -> std::time::Instant

#[no_mangle]
pub extern "C-unwind" fn nova_rt_time_now_nanos() -> i64
```

`epoch()` is a `OnceLock<Instant>` initialized on first read, so every reading is non-negative. `nova_rt_time_now_nanos` returns `epoch().elapsed().as_nanos()` narrowed to `i64`, **saturating at `i64::MAX`** rather than wrapping.

**This replaces an existing epoch rather than adding a second one.** `poll.rs` already has `fn log_epoch() -> Instant` (`crates/nova-runtime/src/poll.rs:136`), a `OnceLock<Instant>` with exactly one caller — `LogGate::allow` at `poll.rs:182`. `log_epoch` is deleted and that caller reads `crate::time::epoch()`. Two independent origins that happen to agree is worse than one that is named for what it is.

---

## 5. The sleep parker: retyped and renamed

`std/task::sleep(ms: Int)` is **deleted**. `std/time::sleep(d: Duration)` replaces it. Same-name coexistence is not available: `import_std_module` seeds each std module's exports with `scope.values.entry(n).or_insert(*r)` (`crates/nova-resolver/src/lib.rs:1284`), so the first writer wins **silently with no diagnostic**, and `$std.task` precedes any appended `$std.time` — `std.time::sleep` would simply be invisible in every user module.

The parker itself moves from milliseconds to nanoseconds. Because its type is `(Int) -> Future<unit>` either way, **retyping alone is invisible to the compiler** — no call site would fail. So it is renamed in the same change, which turns a silent unit change into a compile error at every stale site:

| Was | Becomes |
|---|---|
| `Builtin::TaskSleepFuture` | `Builtin::TaskSleepFutureNanos` |
| `"task_sleep_future"` | `"task_sleep_future_nanos"` |
| `RtFunc::TaskSleepFuture` | `RtFunc::TaskSleepFutureNanos` |
| `nova_rt_task_sleep_future` | `nova_rt_task_sleep_future_nanos` |
| `deadline_from_ms(ms)` | `deadline_from_nanos(nanos)` |
| `SLEEP_SLOT_MS` | `SLEEP_SLOT_NANOS` |

Twelve sites across four files, all mechanical: `crates/nova-mir/src/lib.rs` (5), `crates/nova-mir/src/lower.rs` (1), `crates/nova-resolver/src/lib.rs` (3), `crates/nova-typeck/src/check.rs` (3). The typed signature stays `(vec![Ty::Int], Ty::Future(Ty::Unit))` and the MIR signature stays `(vec![MirTy::I64], MirTy::Ptr)`; only the integer's meaning and the name change.

`deadline_from_nanos` keeps the existing clamp: a non-positive argument becomes "now", so a negative or zero duration wakes immediately rather than inventing a failure mode.

### Call sites to update

- `std/task/lib.nova:123` — the declaration, deleted. Its doc comment currently reads *"`ms` is whole milliseconds; Nova has no duration type"*, a claim this increment falsifies; deleting the function retires it.
- `crates/nova-resolver/src/lib.rs:199` — the builtin's doc comment states the `ms` signature and must be rewritten.
- `tests/runtime/task_sleep_order.nova:2,7` — two `sleep(200).await` / `sleep(20).await` sites become `Duration::from_millis`.
- `tests/runtime/task_deadlock.nova:21` — a comment naming `sleep(10).await`.

---

## 6. The zero-timeout invariant

Once sub-millisecond deadlines can reach the poller, both arms of `poll::wait` can convert a real remaining duration into a **zero** timeout, which returns immediately and turns a park into a busy spin until the deadline passes:

- Windows truncates: `i32::try_from(d.as_millis()).unwrap_or(i32::MAX)` (`crates/nova-runtime/src/poll.rs:487`) maps 500µs to `0`, and `WSAPoll(.., 0)` returns at once.
- Unix has the same shape one unit finer: `libc::timeval.tv_usec` truncates a sub-microsecond remainder to zero.

**The rule, stated once for both arms:** a remaining duration that is greater than zero converts to **at least one** of the platform's smallest timeout units — 1µs on Unix, 1ms on Windows. A remaining duration of exactly zero still converts to zero, because the deadline has passed and returning immediately is correct.

This is not a platform workaround, it is the only behaviour consistent with the contract already documented: sleep suspends for **at least** the requested time. Rounding up honours that; truncating down violates it.

To make the invariant checkable rather than reviewed, each arm's conversion is extracted from its inline expression into a named pure function — `fn select_timeout(d: Duration) -> libc::timeval` and `fn wsapoll_timeout_ms(d: Duration) -> i32` — whose whole purpose is to be unit-testable.

---

## 7. Compiler and module seams

| Constant | Was | Becomes |
|---|---|---|
| `STD_MODULES` (`crates/nova-resolver/src/lib.rs:1212`) | 8 | **9** — `("$std.time", include_str!("../../../std/time/lib.nova"))` inserted immediately after `$std.task` |
| `Builtin::STD_ONLY` (`crates/nova-resolver/src/lib.rs:647`) | 58 | **59** — `Builtin::TimeNowNanos`, spelled `time_now_nanos`, typed `() -> Int` |
| `RESERVED_TYPE_NAMES` | 7 | **7**, unchanged |

`$std.core` stays first, as its own doc comment requires. A directory under `std/` is not automatically a module — `std/test` is the standing proof — so the `STD_MODULES` entry is what makes `std/time` exist.

The new builtin threads the same seams every intrinsic does: the `Builtin` enum and its spelling, `STD_ONLY`, the typeck signature table, `Lowering::Runtime` in `nova-mir/src/lower.rs`, the `RtFunc` enum with its symbol name and MIR signature, and the codegen mapping.

---

## 8. Errors and edge cases

**There is no error surface.** No `Result` appears anywhere in §3, no operation touches the OS beyond reading a clock, and `Instant::now()` is infallible. So unlike `std/fs` and `std/net`, `std/time` has **no status-code boundary** and ADR 0011's "the status code is the error kind" pattern does not appear here at all. Stated explicitly so no reviewer goes looking for it.

The only failure mode is arithmetic, and it is silent: **Nova's `Int` wraps.** Measured — `9223372036854775807 + 1` prints `-9223372036854775808`, with no trap and no diagnostic.

Left alone, that produces a genuinely surprising result. `from_secs(10_000_000_000)` overflows i64 nanoseconds; the wrapped value can be negative; and `deadline_from_nanos` clamps a non-positive argument to "now" — so an *overflowing* sleep would return **instantly** instead of sleeping a very long time.

So the three constructors **saturate** at the largest representable value instead of wrapping:

| Constructor | Saturates above |
|---|---|
| `from_secs` | `9_223_372_036` seconds |
| `from_millis` | `9_223_372_036_854` milliseconds |
| `from_micros` | `9_223_372_036_854_775` microseconds |

Each is a comparison and a clamp in Nova. This is the no-panic analogue of `std::time::Duration::from_secs`, which panics on overflow: a clamped answer instead of a silently wrong one.

Remaining edges, each with a defined answer:

- **Negative arguments.** Nova's `Int` is signed and nothing stops `from_millis(-1)`. Constructors accept them as ordinary arithmetic; `sleep` treats any non-positive duration as an immediate wake.
- **`duration_since` with a later argument** saturates at zero, so an elapsed time can never come back negative.
- **`from_micros(1).as_millis()`** is `0`. Truncating integer division, intended, and asserted in §9 so it cannot drift.
- **Monotonicity** comes from Rust's `Instant`, so readings are non-decreasing and `Instant.nanos` is never negative.
- **`i64` exhaustion** of the clock itself needs ~292 years of process uptime; `nova_rt_time_now_nanos` saturates rather than wrapping.

---

## 9. Testing

Four layers. The fourth exists because of a gap this design found rather than assumed.

**1. The intrinsic (Rust unit tests, `time.rs`).** `nova_rt_time_now_nanos` is non-negative, and two successive readings are non-decreasing. `epoch()` returns the same origin across calls — a second `OnceLock` would silently break the shared-origin claim in §4.

**2. The zero-timeout invariant (Rust unit tests, `poll.rs`).** Against the extracted functions from §6: a duration greater than zero but below one platform unit converts to a **non-zero** timeout, and a zero duration converts to zero. Per arm, so the Windows rule is covered on the Windows leg and the Unix rule on the other two — the clippy-on-both-OSes reasoning applies to tests too.

**3. Nova arithmetic (fixtures under `tests/runtime/`).** The conversion table in §3 both directions: `from_secs(1).as_millis() == 1000`, `from_millis(1500).as_secs() == 1`, `from_micros(1).as_millis() == 0`, the saturating boundaries from §8, and `duration_since` saturating at zero when the argument is later.

**4. A structural assertion on sleep's park — new machinery.** There is currently **no** way to observe a staged deadline: `task.rs:290` exposes `staged_io_for_test()` for I/O, and the comment above it notes that a bare `Wait::Deadline` has no equivalent. The only coverage of sleeping is `tests/runtime/task_sleep_order.nova`, which sleeps 200 then 20 and pins the order — and **that fixture is scale-invariant**. A millisecond-to-nanosecond conversion wrong by a factor of a million preserves the ordering, so the single riskiest change in this increment would pass unnoticed.

So this increment adds `#[cfg(test)] pub(crate) fn staged_deadline_for_test() -> Option<Instant>` mirroring `staged_io_for_test`, and asserts that a 50ms sleep stages a deadline whose **magnitude** is right — not merely that some deadline is present. Assert identity, not existence.

The bound is exact, and so is the point it is measured from. Capture `before = Instant::now()` **before** polling, then assert

```
staged_deadline_for_test().unwrap() - before  ∈  [40ms, 500ms]
```

Measuring from before the poll is what makes the lower bound safe: the staged deadline is computed at or after `before`, so by monotonicity the difference is at least the requested 50ms however long the thread is descheduled — a deadline captured *after* the poll could drift below any lower bound and flake. The upper bound holds because the difference exceeds 50ms only by the cost of the poll itself.

That window is deliberately loose against jitter and tight against unit errors: a factor-of-a-million overshoot stages ~50,000 seconds and fails the upper bound, a factor-of-a-million undershoot stages ~50 nanoseconds and fails the lower one, and a factor of a thousand fails in either direction.

An end-to-end test cannot substitute here. A test that merely drives the executor cannot distinguish a park from a busy spin, because both complete — established four independent times during the poller increment.

---

## 10. Records to update

- **`CHANGELOG.md`** — `[Unreleased]`, `Added` for `std/time`, and **`Changed` as breaking** for `std/task::sleep`'s removal, in the register `Bytes` joining `RESERVED_TYPE_NAMES` used.
- **`nova-spec/20-STDLIB.md` §9** — amended to record the nanosecond representation, that `timeout` is not yet delivered, and the platform granularity asymmetry from §6, following the §16-plus-§4-note precedent set by the poller increment.
- **No new ADR.** Nothing here deviates from the spec: §9 is implemented as written apart from the deferral, `std.task` never listed `sleep` in §13 so removing it moves *toward* the spec, and the representation §9 leaves opaque. Should the implementation force a deviation, that is the signal to write one.

---

## 11. Deferred, recorded

- `timeout<T>` and `TimeoutError`, with the three measured blockers in §1 — the next increment.
- The two `connect` unit tests that pre-poll and then hand the same future to `block_on` never exercise park-and-wake; their `OK` proves only that nothing had failed yet. Untouched here, still recorded.
- Wall-clock time, whenever `std/log` needs a timestamp.
