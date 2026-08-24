# Changelog

All notable changes to Nova are documented here.

Format follows [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).
Nova uses [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

---

## [Unreleased]

### Added
- **`std/time`**, a ninth `STD_MODULES` entry (`"$std.time"`, `STD_MODULES`
  8 → 9): `Instant { nanos: Int }` and `Duration { nanos: Int }`, both a
  nanosecond count — `Instant`'s since a single process-monotonic origin
  (`crates/nova-runtime/src/time.rs`'s `epoch()`, a `OnceLock<Instant>`),
  `Duration`'s a plain span. Eight methods, each one line of Nova arithmetic
  over the field: `Instant::now()`, `Instant::elapsed(self)`,
  `Instant::duration_since(self, earlier)` (saturating at zero rather than
  going negative), and `Duration::from_secs`/`from_millis`/`from_micros`/
  `as_secs`/`as_millis`. The three constructors **saturate** at the largest
  representable value instead of wrapping Nova's `Int` past `i64::MAX`, so a
  duration built from an overflowing count clamps rather than silently
  going negative and making `sleep` wake instantly. `net.rs`'s own
  `deadline_epoch` is a second, separate origin that predates this change
  and remains, by choice, for `read_timeout`'s deadline arithmetic — not
  folded into `epoch()`. One new runtime intrinsic, `time_now_nanos() ->
  Int` (`Builtin::STD_ONLY` 58 → 59), narrows the clock reading to `i64`,
  itself saturating at `i64::MAX` rather than wrapping.
  `RESERVED_TYPE_NAMES` stays at 7 — `Instant`/`Duration` are ordinary,
  glob-imported, shadowable `std/time` records, not builtin types, the
  same standing `TcpStream` and `File` already have.
- `pub async fn sleep(d: Duration)`, in `std/time` — see Changed, below, for
  the breaking half of moving it out of `std/task`.
- The sleep parker is retyped from milliseconds to nanoseconds and renamed
  in the same change, so the unit change cannot pass silently: `Builtin::
  TaskSleepFuture` → `Builtin::TaskSleepFutureNanos`, `"task_sleep_future"`
  → `"task_sleep_future_nanos"`, `RtFunc::TaskSleepFuture` → `RtFunc::
  TaskSleepFutureNanos`, `nova_rt_task_sleep_future` → `nova_rt_task_sleep_
  future_nanos`, `deadline_from_ms` → `deadline_from_nanos`, `SLEEP_SLOT_MS`
  → `SLEEP_SLOT_NANOS`. Every compiler-seam call site — resolver, typeck,
  MIR, and the runtime symbol table — was updated mechanically to match;
  the typed and MIR signatures are unchanged (`(Int) -> Future<unit>` /
  `(vec![MirTy::I64], MirTy::Ptr)`) — only the integer's meaning and every
  name carrying "ms" changed.
- The poller's zero-timeout invariant now holds down to sub-millisecond
  deadlines: a remaining duration greater than zero converts to **at least
  one** platform timeout unit — 1µs on Unix (`select_timeout`), 1ms on
  Windows (`wsapoll_timeout_ms`), both in `crates/nova-runtime/src/poll.rs`
  — never truncating to a zero timeout that would turn a park into a busy
  spin. A remaining duration of exactly zero still converts to zero, since
  the deadline has already passed. Both arms are pinned by their own unit
  tests, one per platform.
- A structural test accessor, `staged_deadline_for_test()` (mirroring the
  existing `staged_io_for_test()`), closes a gap this design found rather
  than assumed: the only existing sleep coverage
  (`tests/runtime/task_sleep_order.nova`) is scale-invariant, so a
  millisecond-to-nanosecond conversion off by a factor of a million would
  still pass unnoticed. The new test asserts a 50ms sleep stages a deadline
  whose **magnitude**, not merely presence, is right — measured against a
  `before = Instant::now()` taken ahead of the poll, so the lower bound
  holds by monotonicity regardless of scheduling jitter.
- **`timeout<T>(d: Duration, fut: Future<T>) -> Result<T, TimeoutError>`**,
  in `std/time` (`Builtin::STD_ONLY` 59 → 60; `STD_MODULES` stays 9;
  `RESERVED_TYPE_NAMES` stays 7 — `TimeoutError {}` is an ordinary record,
  the one record in `std/time` with no field to disclose because it has none;
  `std/io`'s `Stdin`/`Stdout`/`Stderr` are fieldless too, so this is not a
  first anywhere wider than its own library). Built
  over one new builtin, `task_timeout_future`, and one hand-written
  `PollFn`, `poll_timeout`, which polls the inner future *before* checking
  the deadline — so work that already completed is never reported as timed
  out, and `timeout(Duration::from_secs(0), ready_future)` returns `Ok`, the
  least surprising answer. **The contract is abandonment, not
  cancellation**: on timeout, `timeout` returns `Err(TimeoutError {})` and
  simply stops polling the inner future — nothing tells it that it lost the
  race, and the poll ABI has no cancellation hook to tell it with. Measured
  against what each inner future owns mid-park, this is free for `sleep`
  (its state is GC-reclaimed), `join` (the joined task runs on
  independently), and `read`/`write` (the caller still holds the
  `TcpStream`). **`connect` is the one leak**: `start_connect` registers a
  socket in the poller's table and only `finish_connect` removes it, so a
  timed-out `connect` leaves a socket entry nothing can reach or close — it
  leaks until process exit, the same standing
  `docs/adr/0012-file-descriptor-lifecycle.md` already accepts for any
  unclosed descriptor. Documented at `timeout`'s own doc comment and at
  `std/net::connect`. **Abandoning the inner future also discards the park it
  staged**: `poll_timeout` snapshots the executor's `PENDING_PARK` slot before
  polling the inner and restores that snapshot on both of its completing
  exits. `poll_one` does take the slot unconditionally, but once per *task
  poll* rather than once per future, so without this an abandoned inner's
  `Wait::Task` or `Wait::Io` was still staged when the next suspension in that
  same task poll staged its own — a collision, and a `std::process::abort()`
  reachable from `timeout(d, h.join())` followed by any other parking
  suspension, under a diagnostic that misattributed it to an inner future's
  `POLL_PENDING` not propagating. The snapshot is *restored* rather than
  cleared, which makes the guarantee local ("the slot is handed back exactly
  as it was found") and so composes with nested timeouts and with an earlier
  abandonment in the same poll. Seven new fixtures (`timeout_ok`,
  `timeout_elapsed`, `timeout_value`, `timeout_join_ok`,
  `timeout_join_elapsed`, `timeout_elapsed_then_join`,
  `timeout_elapsed_twice`) cover both branches, the `task_output(fut)` read
  that must come from the inner future's own slot, both directions of the
  `timeout`-over-`join` pairing that used to abort the process, and both
  shapes of the abandoned-park collision. A third test accessor,
  `staged_task_for_test()`, joins `staged_io_for_test()` and
  `staged_deadline_for_test()`: with only two of the three fields readable no
  test could assert the staged slot is *empty*, which is exactly the
  postcondition the abandonment path has to establish.
- **`SystemTime`, a wall-clock addition to `std/time`** (`pub record
  SystemTime { nanos: Int }`, nanoseconds since the Unix epoch) —
  deliberately a **separate type** from `Instant`, not a method on it:
  `Instant`'s whole contract is monotonicity within one process, and a wall
  clock has none — it can jump backwards when NTP corrects it. Two methods:
  `SystemTime::now()`, reading a new runtime intrinsic,
  `time_now_epoch_nanos() -> Int` (`Builtin::STD_ONLY` 60 → 61) that shares
  nothing with the existing `now_nanos()`/`epoch()` — those answer "how
  long has this process been running," this answers "what time is it," and
  the two must not be confused despite sharing a unit and a width, the same
  register `SLEEP_SLOT_MS` → `SLEEP_SLOT_NANOS` → `SLEEP_SLOT_DEADLINE_
  NANOS` was renamed twice to avoid; and `SystemTime::to_iso8601() ->
  String`, rendering fixed-width ISO-8601 UTC to milliseconds
  (`2026-08-19T02:40:13.123Z`) computed entirely in Nova via Hinnant's
  civil-from-days algorithm, behind private `pad2`/`pad3`/`civil_from_days`
  helpers. **UTC only, and permanently so**: `00-MASTER-SPEC.md` §6's Rust
  crate list is FINAL and carries no date/time crate, so there is no
  timezone database to consult and a local-time offset would be a guess
  that is wrong twice a year in every DST zone. The intrinsic
  (`crates/nova-runtime/src/time.rs`'s `nova_rt_time_now_epoch_nanos`)
  saturates at `i64::MAX` — the rendered timestamp silently stops advancing
  past the year 2262, recorded rather than guarded — and returns `0` for a
  clock set before 1970 rather than propagating a negative value into the
  calendar math. `RESERVED_TYPE_NAMES` stays at 7 — `SystemTime` is an
  ordinary, glob-imported `std/time` record, the same standing `Instant`
  and `Duration` already have. This discharges the wall-clock deferral
  `std/time`'s own design recorded: "`std/log` will eventually want a
  timestamp, which is a wall clock; adding one now is speculation, so it
  waits for the increment that needs it" — this is that increment.
- **`std/log`, a tenth `STD_MODULES` entry** (`"$std.log"`, `STD_MODULES` 9
  → 10): `Log`, `LogLevel`, `LogFormat`, `LogOutput`, `LogConfig`, and
  `init`/`init_with`. The five level functions — `trace`/`debug`/`info`/
  `warn`/`error` — ship as **associated functions on an empty record,
  `Log`** (`Log::info("...")`), not the top-level `pub fn`s `nova-spec`
  §10 originally showed: Nova has no import statements and no qualified
  paths, so every std module's public names are glob-imported into every
  other module, and `import_std_module` resolves a name collision silently
  in the *user's* favour (`crates/nova-resolver/src/lib.rs:1305-1311`) — a
  top-level `error` would have made `std/log`'s own `error` unreachable
  with no diagnostic. `std/strings`'s `join` (`std/strings/lib.nova:
  248-252`) set this precedent first, for the identical reason. Filtering
  compares `LogLevel::to_int()` (`Trace` 0 through `Error` 4) against a
  configured threshold with `<`, inclusive at the threshold, because Nova
  has no `==` on sum types. Two outputs ship, `Stderr` and `Stdout`;
  `LogFormat` has one variant today, `Human`. Configuration lives in the
  runtime (`crates/nova-runtime/src/log.rs`), a thread-local
  `Cell<Option<Config>>` whose getters resolve `None` to `Config { level: 2
  /* Info */, to_stderr: true }` — the entire auto-initialize rule: a
  program that never calls `Log::init()` still logs, and
  `init()`/`init_with(config)` are an explicit override rather than a
  prerequisite, with last-writer-wins semantics. Three new intrinsics,
  `log_config_level`, `log_config_to_stderr` and `log_set_config`
  (`Builtin::STD_ONLY` 61 → 64) — **two separate getters, not one packed
  integer**, deliberately: packing would reintroduce the exact hazard this
  project has now hit twice, an `i64` whose meaning changes while its type
  does not. Log calls return nothing and cannot fail — a logger has
  nowhere left to report a stderr write failure. Deferred to a named later
  increment, not left as unnoticed gaps: `LogFormat::Json`,
  `LogOutput::File(String)`, and the TTY detection needed to choose
  between `Human` and `Json` automatically. `RESERVED_TYPE_NAMES` stays at
  7 — `Log` and `LogConfig` are ordinary, glob-imported `std/log` records
  and sum types, not builtin types.
- **`std/fmt`, an eleventh `STD_MODULES` entry** (`"$std.fmt"`, `STD_MODULES`
  10 → 11, placed immediately after `$std.strings`): `Int::pad(width: Int)
  -> String`, `String::pad_left(width: Int) -> String`,
  `String::pad_right(width: Int) -> String`, and `Float::fixed(places: Int)
  -> String`. All four ship as **methods on the primitive types they
  format, not top-level `pub fn`s**, because Nova has no imports or
  qualified paths — every std module's public names glob-import into every
  other module, and a top-level name would take `pad`/`pad_left`/
  `pad_right`/`fixed` away from any program defining its own, the same
  trade `std/strings`'s `join` and `std/log`'s five level functions already
  declined. `Int::pad` zero-pads with the sign counted toward the
  requested width (`(-5).pad(3)` is `"-05"`); `String::pad_left`/
  `pad_right` space-pad on the left/right. All three return the value
  **unpadded, never truncated**, when it is already at or beyond `width`
  — the same early return doubles as the negative-width clamp, since
  `String::repeat` panics on a negative count. `Float::fixed` renders a
  fixed number of decimal places (`(100.0 / 3.0).fixed(2)` is `"33.33"`)
  over one new runtime intrinsic, `float_fixed` (`Builtin::STD_ONLY` 64 →
  65; `nova_rt_float_fixed`, `extern "C"`), clamping `places` to `0..=17`
  — Nova has no `Float`→`Int` conversion at all, so fixed-place decimal
  rendering was inexpressible in the language before this.
  `RESERVED_TYPE_NAMES` stays at 7 — nothing here is a new type. See
  `nova-spec/20-STDLIB.md` §3's amendment and
  `docs/adr/0015-std-fmt-scope.md` for what §3 specifies that this does
  not ship, and why.
- **`std/sync`, a twelfth `STD_MODULES` entry** (`"$std.sync"`, `STD_MODULES`
  11 → 12, placed immediately after `$std.task`): an *async* `Mutex<T> {
  locked: Bool, value: T }` with `new(value: T) -> Mutex<T>`, `try_lock(mut
  self) -> Option<MutexGuard<T>>`, and `async fn lock(mut self) ->
  MutexGuard<T>`, plus `MutexGuard<T> { owner: Mutex<T>, released: Bool }`
  with `get(self) -> T`, `set(mut self, v: T)` and an idempotent
  `release(mut self)`. `released` is what makes that idempotence true and
  not merely stated — without it `release` was an unconditional
  `self.owner.locked = false`, so a second `release` on a guard whose mutex
  had since been reacquired freed a lock another task was holding — and
  `set` is the write half of the pair `Vec` already ships
  (`std/collections/lib.nova:49`, `:53`), because `get` returns the value
  and a `T` with no assignable interior (`Int`, `Bool`, `Float`, `Char`,
  `String`) is therefore read-only through the guard. `set` is a no-op on a
  released guard, for the same reason `release` is. Contention is handled
  by yielding and retrying (`while !self.take() { yield_now().await }`),
  not by parking, so neither the executor nor the poll ABI changes; the
  cost is that a waiter stays *runnable*, so `report_deadlock` cannot see
  it and a never-released lock spins instead of being diagnosed. Release
  is explicit, not RAII,
  for one reason and one only: `Drop` is described in three spec files
  (`12-TYPESYSTEM.md:192`, `13-RUNTIME.md:96`, `14-CODEGEN.md:24`) and
  implemented in none of them, so a guard's release
  (`self.owner.locked = false`) is pure Nova with no language mechanism to
  hook a scope exit to. This closes position 8 of `00-MASTER-SPEC.md` §3's
  build order **partially** — `std/sync` also specifies `channel<T>`, and
  `20-STDLIB.md` §13's neighbouring `std/task` entry (position 7) specifies
  `spawn_blocking` and `JoinHandle::cancel`; none of the three ship this
  increment. `00-MASTER-SPEC.md:238` names a third position-8 item,
  `atomic`, which this increment does not touch either. Zero new runtime
  intrinsics — `Builtin::STD_ONLY` stays at 65 — and
  `RESERVED_TYPE_NAMES` stays at 7; `Mutex`/`MutexGuard` are
  ordinary, glob-imported `std/sync` records. See
  `nova-spec/20-STDLIB.md` §13's amendment and
  `docs/adr/0016-std-sync-partial-close.md` for what §13 specifies that
  this does not ship, why, and why release being explicit is not itself a
  shortcut.
- **A bounded async channel in `std/sync`**, pure Nova: `Channel<T>` over a
  private fixed-capacity ring, plus `Sender<T>` and `Receiver<T>` as two
  views onto it. `channel<T>(buffer: Int) -> Channel<T>` constructs one and
  `ch.sender()`/`ch.receiver()` reach the pair; `Sender` has
  `try_send(mut self, v) -> Bool`, `async send(mut self, v) -> Bool` and an
  idempotent `close(mut self)`, `Receiver` has
  `try_recv(mut self) -> Option<T>` and
  `async recv(mut self) -> Option<T>`. Those `mut` receivers are
  load-bearing at the call site: per ADR 0005 `mut` is a permission on the
  binding, so handles must be bound as `let mut tx = ch.sender()` —
  ``error[E0060]: `Sender_T.try_send` mutates its receiver, but `tx` is
  immutable`` otherwise. The channel itself needs no `mut`.
  **The signature deviates from `20-STDLIB.md` §13**, which writes
  `channel<T>(buffer: Int) -> (Sender<T>, Receiver<T>)`: that return type
  is a tuple and Nova has none — `error[E0900]: tuple types are not
  supported yet` — so `channel` returns `Channel<T>` instead, the record
  §13's original code block declares immediately above the function and
  never returns. The deviation is confined to the return type and the type
  it moved to is one that block already declares; all three of the channel
  type names it uses are now built. (Scoped to that block because §13 now
  also carries this increment's amendment, which does return `Channel<T>`.)
  The blocker was not lifted but routed around: Nova still has no tuples,
  and §13's literal signature still produces `E0900` today. The two
  `try_`/`async` pairs differ on purpose: `try_send` returns `false` when
  the channel is **full or closed** while `send` returns
  `false` **only** when closed, and `try_recv` returns `None` when
  **empty** while `recv` returns `None` **only when closed and drained** —
  every iteration dequeues before reading `closed`, so buffered values
  drain before a close is reported and a consumer loop has a termination
  condition at all. `buffer < 1` **clamps to 1** rather than panicking,
  following `Int::pad`'s early-return precedent; a zero-capacity
  rendezvous channel would need a second wait state. **Call sites must
  annotate** — `let ch: Channel<Int> = channel(2)` is the only available
  form, because `T` appears only in the return type and Nova has no
  turbofish (`channel<Int>(2)` parses as two chained comparisons, not a
  call). Contention is handled by yielding and retrying, as `Mutex`
  already does, so neither the executor nor the poll ABI changes: no
  fourth `Wait` variant and no arm added to any of `task.rs`'s three
  non-exhaustive `retain` matches. The cost is the same one `Mutex`
  carries — a waiter stays *runnable*, so `report_deadlock` cannot see a
  channel nobody drains, which spins forever instead of being diagnosed.
  That cost was **observed**, not just predicted: a mutation making `recv`
  answer `None` on an empty-but-open channel truncated its fixture's
  output and then hung indefinitely. The field-privacy hazards are
  likewise documented and not prevented, and **ranked by reach**: forging
  a `Sender { ch: c }` grants *nothing*, because the literal needs a
  `Channel<T>` to name and `sender()`/`receiver()` are `pub`; what does
  escalate is `ch.closed = false`, which reopens a closed channel from any
  legitimately-held handle and destroys the terminal-`None` property a
  consumer loop terminates on, and writes through the ring
  (`ch.ring.head`, `ch.ring.len`), which break the invariant the
  truncating `%` depends on. Unenforceable rather than unenforced: a
  field's `vis` is dropped at AST→HIR lowering
  (`crates/nova-hir/src/lib.rs:918-923`), so `check_field_set` has no
  visibility left to check. An earlier version of this entry named forgery
  alone — the same understatement corrected for `MutexGuard` above.
  Zero new runtime
  intrinsics — `Builtin::STD_ONLY` stays at 65 — `RESERVED_TYPE_NAMES`
  stays at 7, and `STD_MODULES` stays at 12; `std/sync/lib.nova` grows
  from 166 to 329 lines and the only *Rust* change is the registration of
  the new `channel_*` fixtures in `crates/nova-cli/tests/run_tests.rs` —
  no runtime, compiler or codegen crate is touched. Three of those
  fixtures exist because a record or a claim was checked against the code
  rather than because the code was written:
  `channel_clamps_buffer_below_one` pins the `buffer < 1` clamp, which no
  test had exercised; `channel_send_refuses_when_closed` pins the async
  `send`'s refusal on a closed channel, the one point where `send` and
  `try_send` are specified to differ and which no test had called; and
  `channel_enqueue_wraps_after_dequeue` pins the ring's *enqueue* modulo,
  `(head + len) % cap`, which no test executed in a state where it wraps
  and which no record had named as either covered or missing. **This
  supersedes the `std/sync` entry above where it says `channel<T>` does
  not ship** — true of that increment, no longer true of the tree — but it
  does **not** close position 8, which stays partial: §13 specifies
  `Mutex` and `Channel` and both now ship, while `RwLock` (named at
  `20-STDLIB.md:27` only) and `atomic` (named there and at
  `00-MASTER-SPEC.md:238`) have no signature or section anywhere in
  `nova-spec/`, and the two position-7 `std/task` items, `spawn_blocking`
  and `JoinHandle::cancel`, are untouched. See
  `docs/adr/0017-std-sync-channel-shape.md` and `20-STDLIB.md` §13's
  2026-08-21 amendment for the shape decision, the two rejected inference
  escapes, and why the livelock is untestable by construction.
- **`std/json`, a thirteenth `STD_MODULES` entry** (`"$std.json"`,
  `STD_MODULES` 12 → 13), built at `00-MASTER-SPEC.md` §3's position **11
  ahead of position 10**: position 10 `std/http` is specified as "use hyper
  internals at runtime layer" — a Rust dependency and runtime surface this
  workspace does not have — while position 11 is a "custom parser" that
  `20-STDLIB.md` §7 writes entirely in Nova. **Position 10 stays unstarted
  and unblocked**; nothing here makes it harder. `20-STDLIB.md` §7's
  `pub type JsonValue = | Null | Bool(Bool) | Number(Float) |
  String(String) | Array([JsonValue]) | Object(Map<String, JsonValue>)`
  compiles **verbatim**, shadowing variant names included — measured, not
  assumed. Shipped with it: `pub record JsonError { msg: String, at: Int }`
  (which §7 names in the trait signatures but never declares),
  `stringify`, total **over values** (no `JsonValue` shape it cannot render,
  no error channel) but **not over nesting depth**, on which see the two
  unbounded costs below; `parse`, accepting all six `JsonValue` forms, one
  value per document with trailing content rejected, RFC 8259 section 7's
  nine escape forms (its eight one-character escapes, and `u` followed by
  four hexadecimal digits — both counted after the backslash, where the RFC
  counts the backslash too and calls them two-character and six-character
  sequences), a high surrogate combined with a following low surrogate
  (though *requiring* that low surrogate is this parser's decision rather
  than conformance: RFC 8259 section 8.2 notes its own ABNF admits a lone
  unpaired surrogate and calls the behaviour of software that receives one
  unpredictable), and JSON's number grammar, whose six rejections belong to
  two mechanisms — `parse_value`'s dispatch refuses a leading `+` and a
  bare leading `.` as `expected a value` before the number scanner is
  reached, while the scanner itself rejects a leading zero before another
  digit, a `-` with no digit after it (wider than "a lone `-`" — `-a`,
  `-.5`, `-+1` and `-]` all land there, since only a digit after the sign
  gets past it), a fraction point with no digit after it and an exponent
  with no digits — a list of what is built and pinned by
  `tests/runtime/json_parse_values.nova`, `json_parse_strings.nova` and
  `json_parse_numbers.nova`, not a conformance claim against the whole RFC;
  and `ToJson`/`FromJson` with impls for `Int`, `Float`, `Bool` and `String`
  and **nothing else** — no container impl and no blanket impl, each
  rejected as untested public API rather than overlooked.
  `stringify_pretty` and `@derive` do **not** ship, so §7 is not closed.
  One new intrinsic, `str_to_float(s: String) -> Float`
  (`Builtin::STD_ONLY` 65 → 66), the inverse of `float_fixed` and a builtin
  for the same reason, which is **correctly rounded decimal-to-binary
  conversion being research-grade and nothing else**: a `digits × 10^exp`
  accumulation double-rounds, and a codec whose numbers do not round-trip
  is a bad property to choose deliberately. Nova's missing `Int`-to-`Float`
  conversion (no builtin, `as` casts `E0900`) makes a Nova parser awkward
  rather than impossible — an if/else chain from `Int` to a `Float` literal
  spans any fixed digit range, exactly as `std/json`'s own `hex_digit` does
  — so it is a cost and not a wall; the rounding ground alone carries the
  refusal. `RESERVED_TYPE_NAMES` stays at 7. Adding one intrinsic touches
  **12 sites**, of which **7 are compiler-forced** (measured by adding only
  the two enum variants and letting the compiler name the rest); reaching 7
  needs `--all-targets`, because one forced site is a description table
  under `#[cfg(test)]` and a plain `cargo check --workspace` finds **6 and
  reports success**. The 12 follows a stated counting rule (ADR 0018 §3) —
  one declaration, `match` arm, array or function body per site, so a
  variant's doc comment counts with its variant and an array's length with
  its array — because a seam count with no rule behind it is not
  reproducible, which is how an earlier draft of that section published an
  unreachable 13. Three deliberate data-integrity rules, each documented
  at the code: a non-finite `Number` renders as `null`, since JSON has no
  `NaN` and no `Infinity` — deliberately lossy, and a live path rather than
  a theoretical one, because Nova can produce both (`0.0 / 0.0`,
  `1.0 / 0.0`, measured); `Int::to_json` rounds **silently** beyond ±2^53,
  because the trait returns `JsonValue` and has no error channel; and
  `Int::from_json` **rejects** a fractional `Number` and a magnitude
  outside `i64::MIN..=i64::MAX` rather than truncating or wrapping either.
  That decode path first used `Float`'s `Display`, which is
  shortest-round-trip rather than exact, so `i64::MIN` rendered as
  `-9223372036854776000` — off by 192 and out of range — and it now takes
  its digits from `float_fixed(n, 0)`; `Display` is still read, but only
  for the fraction check, since `float_fixed` rounds a fraction away. The
  range check is exact rather than merely plausible because `i64::MAX` is
  **not** representable as an `f64` and its nearest `f64` is 2^63, the very
  bound the check rejects. `\uXXXX` needed no second intrinsic: there is no
  `Int`-to-`Char` conversion anywhere, so a code point becomes UTF-8 bytes
  encoded in Nova, then `bytes_from_ints`, then `Bytes::to_string() ->
  Option<String>`, **whose `Option` is the validation** — where an
  `Int`-to-`Char` primitive would have been strictly worse: it would have
  wrapped the runtime's own `nova_rt_char_to_str`, whose body silently
  substitutes U+FFFD for exactly the inputs that must be rejected —
  `char::from_u32(v as u32).unwrap_or(char::REPLACEMENT_CHARACTER)`. **No
  fixture reaches that `None` arm**, though: the surrogate checks ahead of
  it leave every code point that gets to the encoder a scalar value, so it
  cannot fire as written — a second line of defence, recorded at the code
  and unexercised by construction. One-character escapes and raw characters
  never pass through that `Option` at all. **Two unbounded costs, both
  measured and both deliberate, recorded because the Phase 2 gate puts this
  module in front of a socket.** (1) **Nesting depth is unbounded in both
  directions and exceeding it kills the process** — `parse` and `stringify`
  each recurse one native frame per level, and exhausting the stack is a
  hard abort, not a `JsonError`, since `parse` cannot return from a frame
  that no longer exists and `stringify` has no error channel. Measured,
  debug build: `parse` of `[` × 5000 `1` `]` × 5000 succeeds and 6000 dies;
  `stringify` of a value 8192 levels deep renders and 20000 dies — so
  **roughly 12 KB of input text aborts the process.** RFC 8259 section 9
  permits a depth cap; none is imposed, deliberately, a stack-size
  threshold being no portable budget to derive one from. **A caller putting
  `std/json` on a socket must cap depth above it.** (2) **Four string
  accumulators are quadratic, and the dominant pair is `stringify`'s** —
  `quote` and `scan_str` are quadratic in one string literal, while
  `stringify`'s `Array` and `Object` arms are quadratic in the whole
  rendered document, so every document pays. Measured with the flat ~180 ms
  compile baseline subtracted: 4 000 / 8 000 / 16 000 one-character numbers
  parse in 279 / 282 / 344 ms, effectively flat, and render in 236 / 1 309
  / **11 622** ms, so a ~32 KB document takes about twelve seconds to
  render; one string literal costs 182 ms at 8 000 characters and 15 624 at
  64 000. Absolutes are debug-build numbers, asymptotics are not, and
  neither is fixable without a growable string buffer the language does not
  have. Nine new tests, 1048 → 1057: seven `json_*` `nova run` fixtures,
  one for `Map::keys`, and one Rust unit test on the intrinsic. See
  `docs/adr/0018-std-json-scope-and-build-order.md` and `20-STDLIB.md` §7's
  2026-08-23 amendment. **This does not close Phase 2**: position 10
  `std/http` and position 12 `std/crypto` are both unstarted, and the Phase
  2 gate still needs `examples/05-json-api` and `docs/benchmarks/`, neither
  of which exists. **It also splits `docs/phase-2-plan.md` §2.4**, which
  bundles `std/net` + `std/http` + `std/json` as one increment: `std/net`
  shipped alone with the I/O poller, `std/json` ships alone here and ahead
  of `std/http`, and nothing is left of the bundle as a unit. That plan
  still describes it as one step; recorded here because a commit that
  changes a decision must sweep every document asserting it, and this
  branch's sweep reached `20-STDLIB.md`, `13-RUNTIME.md`, ADR 0017, ADR
  0018 and this file but not that one — the same way the ADR-0009
  divergence from `docs/phase-2-plan.md` decision 1 is recorded below.
- `Map::keys(self) -> [K]`, in `std/collections` — position 3's public API,
  changed from a position-11 increment, because `Object(Map<String,
  JsonValue>)` cannot be serialised without enumerating its keys and `Map`
  had every operation except the one that visits what is there. Taken as a
  `std/collections` change rather than a private `std/json` helper because
  a map that cannot be enumerated is arguably incomplete regardless of
  json. Returns **table order, not insertion order**, documented at the
  method as explicitly not a guarantee. The precise statement is that
  **`keys()` order is not a function of the key set alone**: hash order
  alone reverses insertion order with **no growth at all** (measured —
  inserting `"a"`, `"c"`, `"e"` returns `"e"`, `"c"`, `"a"`, three entries
  in a cap-8 table that never reaches the 3/4 threshold), and linear
  probing makes slot order depend on arrival sequence, with a `grow`
  reinserting every entry as a further reordering rather than the only one.
  Naming only `grow` would suggest a map that never grows preserves
  insertion order; it does not.

- **A server side for `std/net`** — `TcpListener`, `bind`, `local_port` and
  `accept` — so that Phase 2 position 10 (`std/http`) has a transport to
  build a server on. Position 9's own section had assigned all three to "a
  future increment"; this is that increment, so it closes a position in order
  rather than deviating from the build order, and adds no ADR.
  `pub record TcpListener { fd: Int }` plus
  `bind(addr: String) -> Result<TcpListener, IoError>`,
  `local_port(self) -> Result<Int, IoError>`,
  `async accept(self) -> Result<TcpStream, IoError>` and an
  `async close(self) -> Result<(), IoError>` matching `TcpStream`'s. The
  `async` markers belong in this list rather than being flattened out of
  it: `bind` and `local_port` cannot suspend and so are plain `fn`, while
  `close` is `async` anyway to match `TcpStream::close` — the one
  deliberate exception to that rule, and the distinction §16 spends a
  paragraph on. Three intrinsics (`STD_ONLY` 66 → 69): `net_listen` and
  `net_local_port` cannot suspend and so take `net_close`'s plain
  status-word shape with no poll function, while `net_accept` does suspend
  and has one hand-written `extern "C-unwind"` poll function, written
  level-triggered and tag-free because the frozen ABI carries no waker and
  records no reason for a wake. `close` needed no fourth intrinsic: the
  runtime removes a table entry by key regardless of kind, so a listener and
  a stream share one. Listeners live in the existing socket table as a
  two-variant enum, which keeps one descriptor space, one closedness
  invariant and one `close` — and makes a kind mismatch an ordinary error
  rather than a lookup that succeeds oddly.
  `accept` returns a `TcpStream` indistinguishable from a connected one, so
  every existing `Read`, `Write`, `read_timeout` and `close` works on it.
  The poller's `Interest` and its wait logic did not change:
  accept-readiness is read-readiness on both `select` and `WSAPoll`, so
  `Interest` gained no variant. This said "nothing in the poller changed",
  which was too broad: `poll.rs`'s module doc comment changed, and so did
  `Interest`'s own doc comment — no executable line in that file did. The
  first correction here said "nothing else in the file did", which the very
  commit that wrote it falsified by editing `Interest`'s doc two findings
  later.
  **Two failures collapse to `IoError { kind: Other }`, deliberately**:
  `IoErrorKind` has eight variants and `AddrInUse` is not one, so an
  address already in use is separable only by the OS message; wrong-kind
  access collapses the same way, because one kind-checked table reports
  absent and wrong-kind through the same `None`. Adding a variant is a
  wire-contract change across two crates and was not made.
  **What this does not close.** Concurrency is not proven *at scale*.
  This said "no fixture parks two sockets at once"; the fixture added by
  this increment falsifies that, parking two tasks on two sockets on every
  run, which is what shows the park path works at all. What stays unproven
  is *many* concurrent connections: there is no `select`/`race`/`join_all`,
  and one socket wait per task is enforced by process abort, so a server
  needs two tasks minimum. `accept` also makes a many-descriptor process
  the first *realistic* shape for this module. It does not make the
  poller's `FD_SETSIZE` rejection path reachable — that rejection is on a
  descriptor's number, one socket at a time, so one socket has always
  sufficed, and `std/fs`'s long-lived descriptors could already push a
  socket's number past the ceiling with no listener involved. On Unix a
  descriptor at or above the ceiling is skipped, never re-watched, and its
  task is never woken and never errors; no `IoError` reaches the Nova
  program, though the process does emit a rate-limited warning line. That
  path has still never executed on any platform; see the dated amendment
  in `docs/adr/0013-io-poller.md`. The non-blocking assertions are
  platform-conditional, decided by a kernel-calibrated probe, and where it
  declines the coverage is *unknown* rather than claimed. UDP and Unix
  sockets stay unbuilt. **Phase 2 is not complete**: positions 10 and 12,
  `examples/05-json-api` and `docs/benchmarks/` are all still absent, and
  the tag stays `v0.2.0-alpha.1` because §7 of `00-MASTER-SPEC.md` makes
  `v0.{phase}.0` assert that a phase is done.

### Changed

Filed here as well as under Added, because this changes the meaning of code
that already compiled.

- **`std/task::sleep(ms: Int)` is removed; `sleep` now lives in `std/time`,
  over a `Duration` instead of a bare `Int`** (full detail in the Added
  entry above). This breaks every program calling `sleep` with a raw
  integer: `sleep(200)` becomes `sleep(Duration::from_millis(200))`.
  Same-name coexistence across the two modules was not available to soften
  the move — `import_std_module` seeds each std module's exports
  first-write-wins with no diagnostic (`scope.values.entry(n).or_insert(*r)`,
  `crates/nova-resolver/src/lib.rs`), and `$std.task` is registered ahead of
  `$std.time`, so a lingering `std/task::sleep` would simply have made
  `std/time::sleep` invisible in every importing module instead of
  coexisting with it. Accepted rather than fixed a different way, in the
  same register `Bytes` joining `RESERVED_TYPE_NAMES` was: a trap silently
  resolving to whichever definition loads first is worse than one migration
  line.
- **A deadline may now accompany any wait, not only `Wait::Deadline` and
  `Wait::Io`.** `try_stage` no longer treats a second staged deadline as an
  automatic collision: two deadlines merge to the earlier by `min`, and
  `Wait::Task` grows a field to carry one — `Wait::Task(i64)` becomes
  `Wait::Task { id: i64, deadline: Option<Instant> }`. Every other pairing
  (Task+Task, Io+Io) still collides and aborts, unchanged.
  `earliest_deadline` and `deadlock_report` are exhaustive matches the
  compiler forced onto the new shape; `wake_due` is not — its `retain` ends
  in `_ => true` — so it needed an explicit added arm, pinned by a test that
  fails if that arm is missing (omitting it does not fail to compile, panic,
  or diagnose anything; it just hangs).
- **`sleep` is level-triggered, not edge-triggered.** `poll_sleep` now
  re-checks `now >= deadline` on every poll instead of completing
  unconditionally on its second one, becoming structurally identical to
  `poll_join` — forced by the widening above, since a task can now be woken
  for a deadline that is not its own. It also stores a **deadline** in
  epoch-nanoseconds rather than a duration: the slot is renamed again,
  `SLEEP_SLOT_NANOS` → `SLEEP_SLOT_DEADLINE_NANOS`, and `deadline_from_
  nanos` is replaced by two helpers, `deadline_nanos_from_now` (clamps a
  raw duration argument to non-negative, once, at construction) and
  `instant_from_deadline_nanos` (converts the stored absolute deadline back
  to an `Instant` for staging, replacing a panicking `Instant + Duration`
  with `checked_add` plus a halving-loop fallback). The fallback's own
  guarantee is stated carefully rather than absolutely: it is **not**
  structurally impossible for it to hand back an instant the executor treats
  as already due — the loop's lower bound is `base + headroom/2`, which the
  arithmetic alone does not put ahead of `now`. What holds is empirical and
  holds twice over: `Instant` headroom from a process-start `base` vastly
  exceeds process uptime on every supported backend, so the loop lands
  astronomically far out; and the fallback is unreachable through this
  crate's API at all, because `deadline_nanos_from_now` saturates at
  `i64::MAX` nanoseconds and `checked_add` of that always succeeds on the
  first attempt. Clamping the result forward to `now + 1ns` to make the
  property structural is recorded at the function as a **rejected**
  alternative: it would reintroduce exactly the livelock the fallback
  replaced, because `poll_sleep` re-checks the stored *integer* deadline and
  would re-stage on every 1ns wake. Consequence worth naming: a sleep or
  timeout's deadline now runs from **construction**, not first poll — invisible for
  `sleep(d).await` written inline, observable for a future built and held
  before being awaited.
- **`timeout<T>`, previously recorded here as deliberately not delivered,
  shipped in this increment** — see Added, above, for the abandonment
  contract. Its three blockers are each resolved by a change above: the
  staging collision, by the widened `try_stage`; `poll_sleep`'s
  edge-triggering, by making it level-triggered; and the undefined
  abandonment semantics, by choosing abandonment and documenting its one
  leak (`connect`). Also recorded in `nova-spec/20-STDLIB.md` §9's own
  amendment and
  `docs/superpowers/specs/2026-08-18-timeout-combinator-design.md`.
- **`std/time`'s private `pad2`/`pad3` helpers are gone, replaced by
  `.pad(2)`/`.pad(3)` calls on the values themselves**, inside
  `SystemTime::to_iso8601`'s single interpolation line. **Internal only,
  with no surface effect** — `pad2`/`pad3` were never `pub`, so no caller
  outside `std/time` could reach them, and the six ISO-8601 golden outputs
  (`1970-01-01T00:00:00.000Z`, `2000-02-29T00:00:00.000Z`,
  `2024-02-29T00:00:00.000Z`, `2100-03-01T00:00:00.000Z`,
  `2025-12-31T23:59:59.999Z`, `2025-08-31T00:01:03.007Z`) are unchanged
  byte-for-byte — a reader diffing this branch should not go looking for a
  breaking change here. Verified beyond the goldens: a review compared the
  deleted hand-rolled padding against `Int::pad` across every integer
  0–1000, with zero mismatches.

- **`nova-spec/13-RUNTIME.md` section 4 corrected**, and it was false in every
  subsection. It specified a Tokio-backed, work-stealing, multi-threaded
  executor; a `trait NovaFuture` with a `Context` and a `Poll<NovaValue>`;
  `task.spawn(async { ... })` module-path syntax with an awaitable handle;
  structured cancellation via drop-cancels-child and `task.cancel()`; and a
  channel destructured from a tuple, backed by Tokio's `mpsc::channel`.
  Measured against the tree: the workspace depends on **no Tokio at all**
  (zero hits across every `Cargo.toml` and `Cargo.lock`), `task.rs` opens
  "A single-threaded cooperative executor", the real poll ABI is a frozen
  `unsafe extern "C-unwind" fn(*mut u8, *mut u8) -> i64` whose `task_ctx` is
  always null, `spawn`/`block_on`/`yield_now` are free functions with
  `handle.join().await` rather than `handle.await`, cancellation is filed by
  ADR 0009 as an open residual gap, and the channel is pure Nova over a
  private ring. Section 4.4 is relabelled **NOT BUILT** rather than deleted,
  since cancellation remains intent; its two blockers are now named (`Drop`
  unimplemented, and no interrupt hook in a frozen ABI). The correction also
  reaches outside section 4, because fixing the wording alone would have left
  the file contradicting itself: section 1's premise no longer cites Tokio as
  the thing being leveraged, its architecture diagram no longer lists a
  "Tokio executor" component, section 8's "No Tokio" for WASM no longer
  implies Tokio elsewhere, and section 9's `--minimal` no longer excludes a
  dependency that does not exist. Section 9's comparison to a ~3 MB
  Rust+Tokio hello world stays — that is a claim about Rust, not about Nova.
  This is the first dated amendment in `13-RUNTIME.md`; `20-STDLIB.md` had
  been the only file under `nova-spec/` carrying the convention. Routed here
  by `docs/adr/0017-std-sync-channel-shape.md`.
- **`nova-spec/13-RUNTIME.md` sections 2.1 and 3 corrected in the same pass.**
  Section 3 named bdwgc then MMTk; the collector is hand-written, and the
  workspace depends on no third-party collector at all. Its declared
  compiler-facing interface — `nova_gc_alloc(size, type_id)`,
  `nova_gc_register_root(slot)`, `nova_gc_safepoint()`, `nova_gc_init()` —
  exists in **no** form; the real API is `alloc(size, scan: bool)` plus
  `add_root`/`remove_root` taking an object pointer rather than a slot, with
  `nova_gc_scan_range` and a `nova_gc_collect_roots` shim in `gc_stack.c`.
  Three differences are substantive, not cosmetic: `scan: bool` replaces
  `type_id` because a conservative collector has no use for type identity;
  roots pin by address, not by slot; and **there is no safepoint** — the
  claim that the compiler emits `nova_gc_safepoint()` at loop back-edges was
  false, and collection triggers from `alloc` on a growth threshold, so a
  program that allocates nothing never collects. 3.2 (MMTk) is relabelled an
  aspiration, with the note that its blocker is the same fact that forces 3.1
  to be conservative: neither codegen backend emits stack maps. 3.4's
  `Drop`-based finalizer is relabelled NOT BUILT, and the per-object hook
  that *does* exist is named — the sweep calls `task::forget_freed_state(addr)`
  for every freed object, receiving the address and nothing else, which is
  precisely why ADRs 0012 and 0017 both chose an explicit `close`.
  Section 2.1 followed because it could not be left standing: it specified an
  in-band `ObjectHeader { type_id, flags }` before every object's fields, and
  there is **no header** — metadata lives in a side table
  (`struct Obj { addr, size, scan, marked }`), which is the same fact that
  makes 3.3's `type_id` wrong. Section 1's diagram and section 8's WASM notes
  were updated for both, since "No bdwgc" implied bdwgc elsewhere exactly as
  "No Tokio" did.

- **`nova-spec/13-RUNTIME.md` sections 5 through 10 audited, completing the
  file.** Section 5 contradicted itself: it said a panic aborts the current
  task "not whole process by default" and propagates to an `await` site as
  `Err`, three lines above a block calling `std::process::abort()`. A panic
  ends the process; there is no unwinding and no per-task recovery, the entry
  point is `nova_rt_panic_str(s: *const NovaStr) -> !` rather than
  `nova_panic(msg, len)`, and the `Err` claim is precluded by 4.2 rather than
  merely unimplemented — a generated poll frame has no landing pads. No
  `--panic=abort` flag exists and none is needed. Section 7 was worse, because
  it is what a contributor reads to add a runtime hook: it described reaching
  the runtime via `extern "nova-rt"` declarations, which exist in no `.nova`
  file and which `std/core/lib.nova` records as **deliberately rejected** —
  `nova_`-prefixed extern symbols are compiler-reserved, "so a user-visible
  `extern` was not an option either." Rewritten to describe the real route (a
  compiler-known `Builtin`, a `nova_rt_*` function, and a line in `symbols()`)
  and to name the one seam the compiler cannot enforce, `symbols()`, where an
  omission compiles clean and fails at JIT link time — with
  `every_rt_func_symbol_is_registered_with_the_jit` as its guard. It gives no
  count of forced seams on purpose: that number has moved as the compiler grew.
  Four of section 7's six example functions did not exist at all. Sections 6
  and 10 get labels rather than rewrites, since they are unmarked intent rather
  than a misdescribed mechanism: 6.1/6.2 need a `nova.toml`, `@c_import`,
  `unsafe` blocks and a `--crate-type` flag that do not exist, and section 10's
  stress and benchmark bullets have no harness behind them.

## [0.2.0-alpha.1] - 2026-08-16

Standard-library-core progress milestone, and a **pre-release on purpose**.
Phase 2 is not done: of the 13 module groups in `nova-spec/00-MASTER-SPEC.md`
§3, six are complete, two are partial, and five are unstarted — and the Phase 2
gate (`examples/05-json-api` at 10k+ req/sec, with methodology in
`docs/benchmarks/`) cannot yet be assessed, because neither artifact exists.
§7 reserves `v0.{phase}.0` for a phase that is DONE, so this ships as
`v0.2.0-alpha.1` and leaves `v0.2.0` for the gate.

What it does deliver, on top of v0.1.0: a module system, an async runtime with
a real single-threaded executor and three wake sources, a `Bytes` type,
`std/io` with `Read`/`Write` and stdio, `std/fs` on both Strings and bytes with
`File`/`open`/`OpenOptions`, `std/collections`, `std/strings`, `std/test` with
`nova test`, and an I/O poller with a `std/net` TCP client that demonstrably
suspends rather than spins.

Verified at the tag: `cargo build --locked --workspace` then 973 passed, 0
failed, 8 ignored across 44 targets; `clippy --locked --all-targets
--all-features -D warnings` and `cargo fmt --all --check` clean; seven CI
checks green on ubuntu, windows and macos, including an MSRV 1.78 leg that
asserts the compiler it runs.

### Added (Phase 2 — standard library core, in progress)
- Module system (Phase 2.0): multi-file programs with `import`. One file is one
  module (its file stem); `import m` brings all of module `m`'s `pub` items into
  scope, `import m::{a, b}` brings the named ones, and only `pub` items are
  importable — private items stay module-local (ADR 0003). The driver loads the
  entry plus every transitively imported `<name>.nova` beside it; the resolver
  builds per-module namespaces, enforces visibility, and merges all items into
  one program so whole-program monomorphization is unchanged. Cross-module
  records, functions, and traits (including generic bounds over an imported
  trait) work under both backends. Dangling imports and private-item imports
  report `E0001`. (`import as` aliases and qualified `m::name` paths are later
  increments.)
- Method-level generics (Phase 2.0): an inherent method may introduce its own
  generic parameters on top of the impl's — e.g. `impl<T> Box<T> { fn map<U>(self,
  f: fn(T) -> U) -> Box<U> { … } }`. The method's parameters are inferred from
  the call arguments (the impl's from the receiver), inline bounds (`<U: Show>`)
  are enforced at monomorphization, and each concrete instantiation is
  monomorphized separately. Implemented via a flat generic layout — impl
  parameters first, method parameters after — so substitution, bound checking,
  and monomorphization stay uniform. Generic parameters that shadow an impl
  parameter report `E0403`.
- Generic trait methods (Phase 2.0): a trait method may now declare its own
  generic parameters — `trait Mapper { fn remap<U>(self, f: fn(Int) -> U) -> U }`
  — in the trait declaration (required or default-bodied) and in its impls. The
  method's type arguments are inferred per call site (`Self` at `Param(0)`, the
  method's generics after) and threaded into monomorphization so each concrete
  instantiation is compiled separately; inline bounds are enforced at
  monomorphization (`E0013`). An impl method whose generic arity *or* bounds
  disagree with the trait's is rejected (`E0072`), so the trait method signature
  the caller programs against is the contract every impl honors — an impl may
  neither drop nor add a method-generic bound. Dispatch works both on concrete
  receivers and through a `T: Trait` bound in a generic function. `async` on a
  trait-method declaration is rejected (`E0900`) like every other method site.
- Duplicate generic parameter names are now rejected (`E0403`) at every site a
  generic list can be written — free functions, records, sum types, trait
  methods, and `impl` blocks — not only impl methods. A silent duplicate had
  kept just the last binding, leaving the earlier parameter a phantom the
  program could never name.
- `where` clauses (Phase 2.0): an out-of-line spelling of generic bounds on
  functions, impl blocks, and inherent methods — `fn label<T>(x: T) -> String
  where T: Show { … }` is equivalent to the inline `<T: Show>`, and
  `impl<T> Box<T> where T: Show { … }` is the conditional impl `impl<T: Show>`.
  Bounds accumulate on top of any inline bounds and are enforced at
  monomorphization. A `where` clause may only constrain one of the item's own
  type parameters; constraints on concrete/compound types (`where Box<T>:
  Trait`) and on trait methods are rejected with `E0900`.
- Prelude (Phase 2.0): `Option<T> = | Some(T) | None` and `Result<T, E> =
  | Ok(T) | Err(E)` are now built in — available in every module with no import
  or definition. They are compiled as an implicit module and glob-imported into
  every user module, so they use the ordinary generic sum-type and
  monomorphization machinery (and cost nothing when unused). A program may still
  define its own `Option`/`Result`, which shadows the prelude.
- extern / FFI (Phase 2.0): `extern "C" { fn sqrt(x: Float) -> Float }` declares
  external C functions callable like ordinary functions. Symbols resolve against
  the C runtime with no extra configuration — the Cranelift JIT's dlsym fallback
  under `nova run`, and the system linker under `nova build`. The emitted symbol
  is the raw (unmangled) declared name, imported into both backends. Supported:
  the C ABI (`"C"` or omitted) and FFI-safe scalar types — `Int`↔`int64_t`,
  `Float`↔`double`, `Bool`↔`_Bool`, and a unit (`void`) return. Non-scalar types
  (String, records, arrays — GC heap values), other ABIs, and generic/async/
  `where` extern declarations are rejected with `E0900`; symbols that collide
  with the compiler's own (`nova_*`, `main`) are reserved. Note: because `Int`
  is 64-bit, C functions that use narrower integers (32-bit `int`, e.g. `abs`,
  `getchar`) cannot be declared correctly yet and will truncate — declare only
  `int64_t`/`long long`/`double` C functions for now. (Pointers, strings,
  variadics, `link_name`, and fixed-width C integers are later increments.)
- `panic(msg: String)` builtin (Phase 2.1): aborts the process — prints
  `nova: panic: <msg>` to stderr and calls `std::process::abort()` (no
  unwinding) — via a new runtime function, `nova_rt_panic_str`. Typed
  `Never`, so a `panic(...)` call unifies with whatever type its context
  expects and can stand as a match arm's or `if`-branch's tail expression,
  e.g. the `None`/`Err` arm of `std/core`'s `unwrap()`. Declared in both
  codegen backends' runtime-declaration lists (Cranelift's `ALL_RT`, LLVM's
  `DECLS`). Like `println`/`print`, `panic` is seeded into *every* module's
  scope (`Builtin::GLOBAL`), so it is now a reserved word: a user
  `fn panic(...)` reports `E0002: duplicate definition of 'panic'` with the
  note "`panic` is a compiler builtin". This is deliberate — `panic` is
  user-visible language surface — and is the opposite of `str_cmp` below,
  which is scoped to `std/core` alone precisely so it does *not* reserve a
  name in user code.
- `nova_rt_str_cmp` (Phase 2.1): a runtime function comparing two strings
  byte-lexicographically and returning `-1`/`0`/`1`. Needed because Nova has
  no built-in string ordering to write one *in* Nova source — `String` has
  neither length nor indexing, and `String < String` is `E0013` by design —
  and `std/core`'s `Ord for String` needs one (below).
- Associated-function call syntax, `Type::f(args)` (Phase 2.1): a self-less
  method — one declared with no `self` receiver, in an inherent impl or a
  trait impl — is now callable as `Type::f(...)`, e.g. `P::new()` for
  `impl P { fn new() -> P { ... } }`, or `Int::zero()` for
  `trait Zero { fn zero() -> Self }` + `impl Zero for Int { ... }`. Also
  dispatches through a generic bound inside a generic function
  (`fn make<T: Zero>() -> T { T::zero() }` resolves `T::zero()` to whichever
  concrete impl the call site's `T` turns out to be). `Type` may now also be
  a primitive (`Int`/`Float`/`Bool`/`Char`/`String`) for both inherent and
  trait associated functions — primitive type names previously had no entry
  in the resolver's type namespace at all, so every `Primitive::f()` call
  fell through to a misleading `E0900: module-qualified paths are not
  supported yet`. `std/core`'s `Int::default()` (and the equivalent for
  every other primitive) depends on this.
- Supertraits, `trait Ord: Eq` (Phase 2.1): a trait declaration may name one
  or more supertraits; an impl of the subtrait for a type `R` must be paired
  with an impl of each direct supertrait for that same `R`, or `E0072` names
  the specific supertrait that is missing. A bound `T: Subtrait` (and a
  subtrait's own default-method bodies, reaching the supertrait through
  `Self`) has the supertrait's bounds folded in too, so `std/core`'s
  `Ord`-bounded code can call `Eq`'s `eq`/`ne` without a function separately
  requiring `T: Eq`. Diamond and cyclic supertrait graphs are deduplicated
  and the expansion always terminates. A trait's own `where` clause is now
  parsed and rejected as `E0900` (previously parsed and silently discarded
  with no effect at all — `trait B where Self: A` enforced nothing).
- `std/core`, Nova's first standard-library module (Phase 2.1): real Nova
  source at `std/core/lib.nova`, embedded into the compiler binary
  (`include_str!`) and compiled as one more implicit module — appended last
  and glob-imported into every user module at the lowest priority, so its
  names need no `import` and a user definition of the same name silently
  shadows it (`docs/adr/0004-stdlib-compile-model.md` records this compile
  model and why it is an embed rather than a disk search path or a
  precompiled artifact). Silent shadowing covers the *item* namespaces only:
  a user `impl<T> Option<T>` (or `impl<T, E> Result<T, E>`) that redefines a
  method `std/core` already provides — `map`, `unwrap`, `is_some`, … — is a
  normal overlapping-inherent-impl conflict and reports `E0074: method 'x' is
  defined by multiple overlapping inherent impls`; `std/core`'s impls get no
  immunity from coherence. The method names the six traits claim on the five
  primitives are likewise not shadowable (see ADR 0004's Consequences).
  Contents: `Option<T>`/`Result<T, E>` (previously a
  hardcoded two-line prelude string, now real source checked and diagnosed
  like any other module) gain full method sets — `Option`: `is_some`,
  `is_none`, `map`, `and_then`, `unwrap`, `unwrap_or`, `ok_or`; `Result`:
  `is_ok`, `is_err`, `map`, `map_err`, `and_then`, `unwrap`, `unwrap_or` —
  plus six core traits, each implemented for all five primitive types
  (`Int`, `Float`, `Bool`, `Char`, `String`): `Display` and `Debug` (a
  direct `.fmt()`/`.dbg()` call and a generic `T: Display`/`T: Debug` bound
  now work uniformly across primitives and user types alike; `Debug` quotes
  where `Display` does not — `String` as `"…"`, `Char` as `'…'`, with `Char`
  escaping the backslash, the delimiting quote, and the control escapes the
  lexer accepts, so its output round-trips as a Nova char literal. Known
  limitation: `Debug for String` cannot escape its content, so a string
  containing `"` or `\` debugs to something that is not a valid literal —
  Nova has no way to inspect a string's contents from Nova source, so closing
  this needs a new `std/core`-scoped builtin); `Eq` (`eq`, plus a defaulted `ne`);
  `Ord: Eq` (`cmp(self, other: Self) -> Ordering`; `Bool` orders via
  `if`/`else` and `String` via the new `str_cmp` builtin above — seeded only
  into `std/core`'s own module scope, not a globally reserved word, since
  `String` fails FFI-safety and a `nova_`-prefixed `extern` symbol is
  reserved, ruling out the two ways a library-level string comparison would
  normally reach the runtime); `Clone`; and `Default` (including
  `Default for Char`, `'\0'`).
- Deferred from `std/core` (Phase 2.1), each needing its own design before
  it can be added rather than being an incidental extension of what's here:
  `std/fmt` (richer formatting beyond `Display`/`Debug`), `std/io`,
  `Iterator` (needs a laziness / `for`-loop-desugaring story), `Hash` (best
  designed alongside the collection types it would serve), and `Copy`
  (implicit-copy value semantics, tied to an ownership/move model Nova does
  not have yet).
- Record field assignment, `rec.f = v` (Phase 2.2a): records were immutable
  after construction, which blocked every collection and most future std work.
  Mutability reuses the existing `place_root` chain walk that array element
  assignment already used, so `rec.inner.f = v` and `make().f = v` are rejected
  at the root with `E0060` exactly as `arr[i] = v` is. The store mirrors the
  field *read*'s `8 * index` offset in both backends — the index/type lookup is
  now one shared function — so reads and writes cannot disagree about layout.
  Records are heap objects, so **assignment is alias-visible**: two bindings to
  the same record see each other's writes (`let mut alias = c` then
  `alias.n = 99` changes `c.n`), and the same holds through a `mut self` method
  because the receiver is passed as the same pointer, not copied. That is
  deliberate reference semantics, not an oversight, and it is executed under
  both backends by `tests/runtime/field_assign.nova`. The `E0900` fallback for
  an assignment target that is none of the assignable forms now names all three
  ("a local variable, array element, or record field") instead of only the two
  that predated this change.
- The mutable-receiver rule (Phase 2.2a, `docs/adr/0005-mutable-receivers-and-one-shot-hash.md`
  §1): **calling a method that declares `mut self` now requires a mutable
  receiver place**, reported as `E0060` with the same ``declare it as `let mut
  …` `` note the two assignment forms carry — except when the immutable root is
  a method's own receiver, where the note says to declare it as `mut self`,
  since `let mut self` is not Nova syntax and the advice would be
  unfollowable. All three forms (`arr[i] = v`, `rec.f = v`, and a `mut self`
  call) now share one `require_mutable_place` helper, so the classification, the
  code and the note exist once. Previously `let v = Vec::new()`
  followed by `v.push(1)` was accepted while the equivalent `v.len = v.len + 1`
  was `E0060` — the same effect got two different answers depending on whether
  it was spelled as a field assignment or wrapped in a one-line method, which
  reduced `mut` to gating a syntax rather than an effect. The receiver may be
  any place, not just a bare local (`self.map.insert(k, v)` from inside a
  `mut self` method resolves through the `self` root — that is how `Set` is
  built on `Map`), a temporary receiver (`make().bump()`) is rejected as not a
  place, and the check is a no-op for plain `self` readers, so only the `mut`
  keyword demands anything of callers. Consequently every mutating std API
  declares `mut self` and every caller needs `let mut`. **Known gap, documented
  rather than closed:** trait-method calls are *not* covered — for a generic
  receiver there is no single impl to consult and `hir::TraitMethod` has no
  receiver-mutability field, so `impl Tr for P { fn m(mut self) { … } }` called
  as `p.m()` on an immutable `p` is still accepted. The collections use
  inherent impls only; ADR 0005 §1 records the three-step migration path and
  why closing it first needs a conformance rule for an impl whose receiver
  mutability disagrees with its trait's.
- Repeat-array literal, `[init; n]` (Phase 2.2a): arrays could only come from
  element-by-element literals, so there was no way to allocate one of *runtime*
  length — exactly what a growable collection needs. `init` is a
  **caller-supplied** value rather than a zero or null fill, which is what
  keeps a fresh array from ever holding uninitialized memory and is why no
  `Default` bound is needed anywhere in `std/collections`: `Vec::push` fills
  with the element being pushed, and `Map` fills its key/value arrays with the
  pair being inserted (`state`'s `0` filler happens to be exactly the "empty"
  tag, so a fresh table is empty by construction). `init` is evaluated
  **once**, and that one value is stored into every slot — these are *not* `n`
  copies, so for a heap element type all `n` slots are the same object:
  `[Cell { n: 0 }; 3]` is one `Cell` seen three times and `a[0].n = 42` shows
  through `a[1]` and `a[2]`, and `[Vec::new(); rows]` is one `Vec`, not `rows`
  of them. That is the same deliberate reference semantics as field assignment
  above (Nova has no `Copy` and so no per-slot clone to insert), and
  `tests/runtime/array_repeat.nova` executes the record case under both
  backends. The fill loop is emitted in MIR with the existing block machinery,
  so both backends need only the new `ArrayAlloc` statement. **Both ends of the
  length range abort** rather than being clamped — `[x; -1]` and
  `[x; 1 << 60]` both call the same `nova_rt_panic_str` path, with "array
  length must not be negative" and "array length is too large" — following
  `check_bounds`' abort-on-bad-input precedent. Both bounds are memory safety,
  because the backends compute the allocation size as `8 * len + 8` with
  *wrapping* arithmetic: a large negative length overflows the multiplication,
  and a length above `(i64::MAX - 8) / 8` wraps the size back to negative,
  which `gc::alloc`'s `size.max(8)` clamps to an 8-byte block that the
  deliberately unchecked fill loop then runs off the end of. A clamp instead of
  an abort would also let a clamped-to-zero capacity silently spin a growable
  collection.
- A second embedded std module (Phase 2.2a): `std/core` was loaded through a
  seam that assumed exactly one implicit module. It is now a list, so
  `std/collections` lives in its own file (`std/collections/lib.nova`) with the
  same compile model as `std/core` (ADR 0004 — embedded with `include_str!`,
  appended last, glob-imported at lowest priority, silently shadowable). The
  driver registers a `FileId` per std module so diagnostics still name a real
  file, and the std-only builtin gating now asks whether a module is *a* std
  module rather than *the* std module.
- `Hash` (Phase 2.2a, ADR 0005 §2): `pub trait Hash { fn hash(self) -> Int }`
  in `std/core`, with impls for `Int`, `Bool`, `Char` and `String` and the
  contract that `a.eq(b)` implies `a.hash() == b.hash()`. It is **one-shot**
  rather than `nova-spec`'s streaming `Hasher` protocol, which Nova cannot
  express: a hasher must accumulate into a field, needing `mut` on a parameter
  plus a `mut self` *trait* method — precisely the gap §1 leaves open — and the
  whole mechanism would then rest on alias visibility rather than on anything
  the type says. ADR 0005 §2 records that this is a commitment, not a stopgap:
  `hash` is the trait's only method, so switching shapes would break every impl
  and call site. Backed by `mix64`, the splitmix64 finalizer (module-private, so
  it enters no user namespace) for `Int`/`Bool`/`Char`, and two std-only
  builtins: `str_hash` (over the runtime's new FNV-1a `nova_rt_str_hash`,
  because `String` has no length, indexing or iteration and is not FFI-safe, so
  Nova cannot walk its bytes) and `char_to_int` (Nova has no `as` casts and no
  other `Char` → `Int` conversion; it is the first builtin with no runtime
  function at all, since `Char` and `Int` are both `MirTy::I64` and `nova-mir`
  lowers it to a register move). Being std-scoped rather than global, neither
  becomes a reserved word. **Mask a hash; never shift one and never read its
  high bits** — `hash & (cap - 1)` over a power-of-two capacity is the only
  supported way to get a bucket index, because a hash spans the full `Int`
  range including negatives (so `hash % cap` can be a negative index), the high
  bits are not an independent second hash, and `mix64`'s guarantees are stated
  over its whole 64-bit result. **There is deliberately no `Hash for Float`**, a
  documented deviation from `20-STDLIB.md`: NaN never equals itself, so a NaN
  key would be inserted and then unfindable even by the expression that
  produced it, and `0.0 == -0.0` while their bit patterns differ, so any
  bitwise hash would break the `eq` ⇒ equal-hash contract. That needs a NaN
  decision belonging with the `Ord for Float` caveat, and `float_has_no_hash_impl`
  pins the absence so re-adding it is a deliberate act.
- `std/collections`, Nova's second standard-library module — `Vec`, `Map` and
  `Set`, written **in Nova** (Phase 2.2a):
  - `Vec<T>`: `new`, `len`, `is_empty`, `push`, `pop`, `get`, `set`, `clear`.
    Growth doubles from 4 by allocating `[x; newcap]` with the pushed element
    as the filler and copying the existing elements back; the record object's
    address never changes and the array's only referent is that field, so the
    conservative non-moving collector needs no special handling. `get` returns
    `Option<T>` *by value* rather than the spec's `Option<&T>` — Nova has no
    references, and for heap types the value is the pointer, so it still
    behaves referentially. `set` out of range panics.
  - `Map<K, V>` for `K: Hash + Eq`: `new`, `len`, `is_empty`, `insert`, `get`,
    `contains_key`, `remove`. Open addressing with linear probing over a
    power-of-two capacity, so a bucket is `hash & (cap - 1)`. Removal leaves a
    **tombstone**, which is what keeps probe chains intact across a deletion —
    including chains that wrap past the end of the table — and tombstones count
    toward the 3/4 load threshold, so a remove-heavy workload cannot degrade
    into an all-tombstone scan. `insert` probes *past* a tombstone to either the
    key itself or an empty slot before storing back into the first tombstone it
    passed, so a replacement can never leave a second, permanently shadowed
    copy behind the hole. Growth doubles and reinserts only the live entries,
    which is also what clears the tombstones. `insert` returns the previous
    value; `remove` returns the removed one.
  - `Set<T>` for `T: Hash + Eq`: `new`, `len`, `is_empty`, `insert`,
    `contains`, `remove`, backed by a `Map<T, Bool>` so the probing, tombstone
    and growth logic lives in exactly one place. `insert` and `remove` report
    whether the set changed.
  - The bound sits on each `impl`, not on the record's generic parameters: a
    bound on a record's generic parameter is rejected with `E0900`, not
    silently accepted (see "Fixed" below). On the impl it is real, and a
    non-`Hash` key is `E0013` at monomorphization. Reachability pruning rooted
    at `main` keeps a program that touches no collection from paying for any
    of it.
    *(Superseded in Phase 2.2d for **records**: a bound on a record's type
    parameter is now accepted, as a resolution scope for projections in field
    types, and is not enforced at construction. The **sum-type** form is
    unchanged and still `E0900`. `Map` and `Set` still carry their bounds on the
    impl, so nothing about this entry's collections changed. See the Phase 2.2d
    entry below and `docs/adr/0007-record-parameter-bounds.md` §1.)*
  - The whole module is exercised end-to-end by `tests/runtime/collections.nova`
    under `nova run`, `nova build` **and `NOVA_GC_STRESS=1`** (collect on every
    allocation): `Vec` across three growths, `Map` through two rehashes with the
    load-factor arithmetic visible, mid-chain and wrapping-chain removals with
    lookups past the hole, tombstone reuse, replacement-behind-tombstones with
    no shadowed duplicate, a user record as a key/element with its own `Hash`
    and `Eq`, `Map<String, Int>` through the runtime hash, negative `Int` keys,
    and `Set` dedup.
- Deferred from `std/collections` (Phase 2.2a), each blocked on a language
  feature rather than on effort:
  - **Iteration on any collection** — `iter()` and `for x in coll`. Needs
    `Iterator` *plus* associated types (`type Item`), which Nova does not have;
    `for` currently works only over integer ranges. Iterating a `Map`'s pairs
    additionally needs tuples, which Nova also lacks, so even
    `for (k, v) in m` has no expressible element type. This is the single
    biggest gap: today a collection can only be read back through the keys or
    indices the caller already holds.
    *(Superseded in two steps, and only in part. Phase 2.2c shipped
    `Iterator`, `VecIter<T>` and `Vec::iter()`; Phase 2.2d added `for x in it`
    and six default methods; the position-11 `std/json` increment added
    `Map::keys(self) -> [K]`. So a `Vec` no longer needs its indices held,
    and a `Map`'s keys can be enumerated without them. What still holds of
    the sentence above: a `Map`'s **values** and **pairs** have no API at
    all, `Set` cannot be enumerated even though it is a `Map` underneath,
    and no container is itself iterable — there is no `IntoIterator`, so it
    is `for x in c.iter()` and never `for x in c`. See the Phase 2.2c and
    2.2d entries below and the `std/json` entry above.)*
  - `Queue` / `Deque` — a ring buffer is expressible, but its `pop_front` would
    want the same iteration story to be useful, and `20-STDLIB.md`'s shape is
    not settled.
  - `Vec::with_capacity` — it would need a `T` to fill the reserved slots with,
    and Nova cannot express reserved-but-uninitialized capacity at all (which is
    the same reason `[init; n]` takes a caller-supplied filler).
  - `Hash for Float` — see the `Hash` entry above; it needs a NaN decision.
  - `std/strings` — string operations beyond `Eq`/`Ord`/`Display`. `String` has
    no length, indexing or iteration from Nova source, so every operation needs
    a new std-scoped builtin plus a runtime function; that is a module-sized
    design, not an increment on this one.
- `std/strings`, Nova's third standard-library module — five new runtime-backed
  intrinsics plus 18 inherent `String` methods, written in Nova (Phase 2.2b):
  - Five new std-only builtins (`Builtin::STD_ONLY` grows from `[Builtin; 3]` to
    `[Builtin; 8]`, so none becomes a reserved word in user code): `str_len_chars`,
    `str_chars` (`String -> [Char]`), `str_from_chars` (`[Char] -> String`),
    `str_to_upper` and `str_to_lower`, each backed by its own `nova_rt_str_*`
    runtime function. `str_chars` is the first intrinsic to construct a Nova
    array from the runtime (`{ len, elem0, elem1, … }`, scanned, matching
    codegen's own array layout byte-for-byte) — a layout mistake there would be
    a silent miscompile, not a crash, so it is pinned by a Nova-level test that
    reads `.len()` back and indexes elements, not by inspecting the Rust code.
  - `std/strings/lib.nova`, the third embedded std module (same compile model as
    `std/core` and `std/collections`: `include_str!`, appended last, glob-imported
    at lowest priority, silently shadowable — ADR 0004), holding the language's
    first inherent `impl String` block, with 18 methods: `len`, `is_empty`,
    `chars`, `char_at`, `slice`, `contains`, `starts_with`, `ends_with`,
    `index_of`, `split`, `trim`, `trim_start`, `trim_end`, `to_upper`, `to_lower`,
    `repeat`, `reverse`, `join`. **Every index and length is in codepoints
    (Unicode scalar values), never bytes** — `"café".len()` is 4 though its UTF-8
    is 5 bytes, and `"日本語".len()` is 3. Consequently these 18 names are now
    reserved on `String`, but by *shadowing* rather than by conflict: an
    inherent method wins by priority over a same-named trait method, so a user
    trait implementing e.g. `trim` for `String` still compiles and `s.trim()`
    silently resolves to the std method instead — gentler than the `E0015`
    ambiguity a second *trait* impl would cause, but still a permanent
    commitment.
  - Error-handling shape follows the `std/collections` precedent: `char_at`
    returns `Option<Char>` (`None` for an out-of-range *or* negative index,
    matching `Vec::get`); `slice(start, end)` panics on an invalid range
    (`start < 0`, `end > len`, or `start > end` — `start == end` is valid and
    yields `""`), matching `Vec::set`; `index_of` returns `Option<Int>` rather
    than encoding absence as `-1`. `split`'s pinned semantics: a missing
    separator yields a one-element array holding the whole string, never an
    empty one; adjacent/leading/trailing separators produce empty pieces with
    no collapsing; an empty separator splits into single codepoints (the
    JavaScript behaviour — Rust adds boundary empties, Python raises, so there
    is no consensus to inherit) and `"".split("")` is `[]`. `join` hangs off the
    separator (`",".join(parts)`, Python-style) rather than being a free
    function, so it does not take the name `join` away from every module via
    glob import. Case mapping (`to_upper`/`to_lower`) is whole-string, not
    `Char -> Char`, because it is not always 1-to-1: `"ß".to_upper()` is `"SS"`
    (2 codepoints, longer than the input) and `"İ".to_lower()` is `"i"` plus a
    combining dot-above (2 codepoints).
  - Deliberate limitations, accepted for this increment rather than overlooked:
    the `trim` family's whitespace test is an explicit list (space, `\t`, `\n`,
    `\r`, and four common Unicode spaces), not Unicode's full `White_Space`
    property; every method that decodes the string at all — `char_at` (the one
    the module's own header flags as the quadratic hazard when called in a
    loop), `slice`, `starts_with`, `ends_with` (which decodes twice, once per
    operand), `contains`, `index_of`, `split`, the `trim` family, `reverse`,
    `repeat`, `join`, and `std/core`'s `Debug for String` — decodes the whole
    string to a `[Char]` first, so each call is O(n) allocation — a 1 MB
    haystack allocates roughly 8 MB — accepted because the Nova-level API is
    unchanged if a `str_find` fast path is ever added underneath it; and there
    is no `replace`, no `pad_start`/`pad_end`, no `split_once`, and no
    `String -> Int`/`Float` parsing.
  - The whole module is exercised end-to-end by `tests/runtime/strings.nova`
    under `nova run`, `nova build` **and `NOVA_GC_STRESS=1`**: byte-vs-codepoint
    length, `chars()`'s array read back from Nova, both `char_at` boundaries,
    `slice`'s half-open boundary plus a nonzero-`start` offset with a
    multi-byte prefix, a round-trip through `slice`+`join` for ASCII/accented/
    CJK/emoji input, every pinned `split` row including a self-overlapping
    separator, search boundaries (an anchored vs. merely-occurring-somewhere
    needle, an odd-index mismatch inside the shared `chars_match_at`
    primitive that backs `starts_with`/`ends_with`/`index_of`/`contains`/
    `split`, empty needle, same-length haystack/needle), the trim family's
    own all-whitespace fallback, an odd-length whitespace run, non-ASCII
    whitespace and `\r`, `repeat`, `reverse`, and whole-string case mapping
    including both directions on `""`.
- Deferred from `std/strings` (Phase 2.2b), each blocked on a language feature
  or a scope decision rather than on effort: `replace`, `pad_start`/`pad_end`,
  `split_once`; `String -> Int`/`Float` parsing (needed by `std/json` later, but
  it raises its own questions — radix, overflow, leading `+`, surrounding
  whitespace — that would widen this increment); grapheme-cluster segmentation
  (Nova's `Char` is a Unicode scalar value, not a grapheme); an exact
  `char::is_whitespace` intrinsic (the approximate list above stands in for
  it); and `nova_rt_str_find`/other fast paths for the O(n)-per-call cost
  noted above.
- **Associated types (Phase 2.2c)** — `trait Iterator { type Item }` and
  projections written `Self::Item` / `I::Item`. A trait may declare associated
  types; an impl binds each one (`type Item = T`), and conformance checks the
  set in both directions — `E0070` for a binding the trait requires and the
  impl omits, `E0071` for one the trait never declared.
  - **Syntax is `::`, deviating from `nova-spec/20-STDLIB.md:95`, which wrote
    `Self.Item` with a dot** (ADR 0006, and the spec is corrected). `::` reuses
    a path form the parser already produced — `A::B` in type position always
    parsed and was rejected by *typeck*, so the projection syntax cost zero
    parser work — and it is already Nova's reach-into-a-type operator
    (`P::new()`, `T::default()`). `Self.Item` does not parse at all.
  - **Represented as `Ty::Assoc { on, assoc }`**, where `assoc` is the
    associated type's own `DefId` under a new `DefKind::AssocType`. Resolved by
    **normalization at seams, not by deferred obligations**: the unifier is a
    210-line Robinson engine whose entire state is `vars: Vec<Option<Ty>>`, with
    no impl table and no constraint queue, and giving it one would have been the
    larger change. The shared core is `hir::normalize_ty(&Ty, &[ImplInfo])` in
    `nova-hir` — a free function over a slice, which is the one signature both
    the type checker (`&self.impls`) and monomorphization (`&module.impls`) can
    satisfy. It is never called from `unify`.
  - Normalization runs **wherever a checked type is consumed** — considerably
    more places in the type checker than the design's three predicted seams —
    plus impl signature checking and, after `subst`, monomorphization. (No count
    is given deliberately: three readers counting the call sites during review
    got three different answers depending on whether they counted logical seams
    or `self.normalize(` calls, and a number here would go stale on the next
    task. `grep` is the authority.) Impl signature checking needed a **separate
    pass after the impl table is complete**: `collect_impls` calls conformance
    ten lines before pushing the `ImplInfo`, so normalizing in place cannot see
    the impl being checked, and hoisting the push instead would make resolution
    depend on declaration order — which Nova deliberately does not have for
    impls.
  - **`Self` is no longer a legal generic-parameter name (`E0076`).** It had
    been accepted, so `impl<Self: It> W<Self>` type-checked with `Self` meaning
    an ordinary parameter rather than the impl's self type — two meanings for
    one token in one scope.
  - Cyclic bindings are rejected (`E0077`, `type Item = Self::Item` and mutual
    chains), while the legitimate chain `type A = Self::B` / `type B = Int`
    still resolves. Normalization is bounded by **two** independent allowances
    and reports rather than diverging: a depth limit (`E0078`, reachable from a
    chain longer than 64) and a total-work allowance (`E0078`, for a *branching*
    chain, which is exponential in depth). Both are load-bearing and measured:
    dropping the work allowance makes a 58-line accepted program take longer
    than 60 seconds; dropping the depth limit overflows the stack.
  - A projection that somehow survives to monomorphization is `E0079` rather
    than reaching code generation. That path is not reachable from source today
    — every probe of it hit an earlier diagnostic first — and is pinned by a
    `nova-mir` unit test, which is the honest way to test a backstop.
  - **The mutable-receiver rule now covers trait methods** (`E0060`), closing
    ADR 0005 §1's documented gap, which `Iterator::next(mut self)` required.
    The check sits at the single point where a trait call's receiver is emitted,
    because the gap turned out to have **five** routes, not one: a direct call,
    a generic bound, a supertrait bound, a trait default body delegating to a
    mutator, and string interpolation reaching a `fmt(mut self)` through a path
    that bypasses ordinary method dispatch entirely.

    **This is a behaviour change, not only an addition.** Code that compiled
    before may now report `E0060`: calling a `mut self` trait method on an
    immutable binding was silently accepted and did mutate. The fix at a call
    site is `let mut x = …`; at a function parameter it is `mut x: T` in the
    signature — note the `E0060` message currently suggests `let mut` even for
    a parameter, which is the wrong advice for that case and is queued. Nothing
    in `std` relied on the permissive behaviour — `VecIter::next` is std's only
    `mut self` in a trait impl and nothing in `std` calls it; every other
    `mut self` method there is in an inherent impl and takes the pre-existing
    route — and no gate fixture output moved.
  - Deliberately out of scope: `Map::iter()` yielding key/value pairs, which
    needs tuples; generic associated types (`I::Item<Int>` is `E0012`); and
    bounds on an associated type (`type Item: Display` is `E0900`).
  - **Known limitation — a projection parameter must not precede the parameter
    that determines it.** `fn f<I: It>(y: I::Item, x: I)` *declares* fine, but
    every call reports `argument to 'f' has type 'Int' but '?0::Item' was
    expected`: `I` is not yet pinned when the first argument is checked, and
    normalization has nothing to resolve. **There is no workaround** —
    annotating the argument does not help — so put the determining parameter
    first (`fn f<I: It>(x: I, y: I::Item)`), which works. This is the price of
    resolving projections at seams instead of deferring them to a constraint
    queue, and the `?0` in that message is an internal inference variable that
    should not be user-visible; both are recorded in the design doc §4.2.
  - **`Iterator` in `std/core`, `VecIter<T>` and `Vec::iter()` in
    `std/collections`** — the consumer the whole increment exists for.
    `pub trait Iterator { type Item  fn next(mut self) -> Option<Self::Item> }`,
    with `pub record VecIter<T> { v: Vec<T>, i: Int }` and
    `impl<T> Iterator for VecIter<T> { type Item = T }`. `iter()` went into
    `std/collections`' **existing** `impl<T> Vec<T>` block rather than a second
    inherent impl on the same type, which nothing in std or the test suite
    exercises. `Item` is bound to the impl's own parameter, not to a primitive,
    so every projection through it goes through `subst` — a monomorphic
    `type Item = Char` would have left that path untested while appearing to
    pass. The impl writes `-> Option<T>` rather than echoing
    `-> Option<Self::Item>`; both are accepted, and the equivalence was checked
    on the shipped impl, not only on a test-local trait.
    - **An iterator must be held in a `mut` binding or arrive as a `mut`
      parameter**, or `next()` is `E0060`. That is the visible face of the
      `mut self` trait-method rule above, and `mut` on a *parameter* is what
      carries it when the iterator arrives as an argument — there is no `let mut`
      to reach for there. (The `E0060` note still advises `let mut` on that
      route, which is wrong advice for a parameter; queued.)
    - `mut self` on `next` is load-bearing, not stylistic: with plain `self` on
      both the trait and the impl, `VecIter::next`'s own body does not compile
      (`E0060`, cannot assign to a field of immutable self). Measured, which is
      why the trait could not be declared before ADR 0005 §1's gap closed.
    - **`Iterator` is implemented for no primitive**, unlike the six traits
      above it in `std/core`, so `next` is *not* taken away from user code on
      `Int`/`Float`/`Bool`/`Char`/`String` the way `fmt`/`eq`/`cmp`/`clone`/
      `default` are (ADR 0004, "method names are not soft-reserved").
    - `VecIter` holds the `Vec` by pointer, so it **aliases** the caller's
      storage rather than copying it: a `push` during iteration is visible to
      the iterator, and an element appended after `next` has already answered
      `None` is still yielded by the following call. Documented and pinned by a
      test rather than prevented — preventing it needs borrow tracking the
      language does not have. Record field visibility is also parsed and never
      enforced, so `VecIter`'s cursor (like `Vec`'s `len`) is writable from any
      user program; pre-existing, and bounded — breaking the invariant produces
      a bounds-check abort, not memory unsafety.
    - Iterating today means a hand-written `while` plus a `match` on the
      `Option`. Still absent, all deliberate: **no `for x in it`** desugar; **no
      default methods**, so no `map`/`filter`/`collect`/`fold`; **no `Set` or
      `String` iterator** (`chars()` already returns an indexable `[Char]`, so
      nothing regresses); **no `IntoIterator`**; and no backwards inference —
      unifying a projection with a concrete type never deduces its `Self`.
  - **The gate:** `tests/runtime/assoc_types.{nova,stdout}`, run three ways
    (`assoc_types_run`, `assoc_types_build_standalone`,
    `assoc_types_under_gc_stress`) — a fourth committed fixture beside
    `collections`, `std_core` and `strings`, and the first coverage of
    associated-type code through the object-file backend and under
    `NOVA_GC_STRESS=1` at all.
    - One measured fact from building it is worth recording, because it makes an
      obvious-looking test toothless: **`mir_ty` maps `Int` *and* `Char` to
      `MirTy::I64`, and `String`/`Record`/`Sum`/`Array` to `MirTy::Ptr`, which is
      `pointer_type()` — `types::I64` on x86-64.** So at the level a backend can
      see, `Int`, `Char`, `String` and every heap type are one type. A generic
      function naming a projection resolved to the *wrong* one of them
      miscompiles silently: `fn first_or<I: Iterator>(mut it: I, dflt: I::Item)
      -> I::Item` at `Vec<Int>` + `Vec<String>` survives a mutation that makes
      monomorphization's normalization cache its first answer, byte-identically,
      and so does `Vec<Int>` + `Vec<Char>`. Only `Bool` (`I8`) and `Float`
      (`F64`) have distinguishable machine classes. Every generic block in the
      fixture therefore instantiates at `Bool` and at `Float`, and each one
      independently kills that mutation.
  - **Three constructs that compiled before now do not.** All three were
    silently accepted, and each was found by review or probing rather than
    predicted:
    - **A projection in an impl's self type** — `impl<T: It> Tr for W<T::Item>`
      — is now `E0900`. It type-checked, but impl selection recovers an impl's
      arguments by structural matching and cannot invert a projection, so such an
      impl could never be selected; worse, it was invisible to overlap checking,
      so it coexisted with `impl Tr for W<Int>` without the `E0074` that pair
      would otherwise get. Dead code that also punched a hole in coherence.
    - **A trait declaring the same associated type twice**, and — the same
      defect wearing different clothes — **a trait declaring the same method
      name twice** — are now `E0403`, the existing duplicate-name code. Both
      previously kept one binding silently.
  - **Fixed, all pre-existing and none introduced by this increment:**
    - `Ty::Error` no longer reaches a user-facing `E0072` as the literal
      `{error}`. A poisoned or unresolvable associated-type binding produced its
      real diagnostic *plus* a meaningless follow-on comparing against
      `{error}`. The cause is worth recording: `Ty` derives `PartialEq` with no
      `Error` absorption, so at the impl signature comparison an `Error` on one
      side **forces** a mismatch — the exact opposite of its behaviour at
      `unify`, where it absorbs. The guard is transitive, because
      `Option<{error}>` is a `Sum`, not a `Ty::Error`.
    - **An impl may now echo a supertrait's associated type.** A trait method
      could name `Self::Elem` inherited from a supertrait, but its impl writing
      the same signature reported `E0001` — the trait side resolved against the
      expanded supertrait bounds and the impl side did not.
    - **Parser recovery no longer escapes an impl body.** One bad token inside
      an `impl` consumed every following top-level item, because the item-boundary
      sync did not treat `}` as a stop — so a following `record` was reported as
      an illegal impl member and `fn main` was parsed *into* the impl and
      discarded with it. Fixing it needed a checked-progress guard rather than
      the obvious "advance first": `parse_file`'s own loop syncs *without*
      advancing, and `}` is the first stop it has no arm for, so the obvious fix
      made `nova check` hang on a two-line file — measured, and caught only
      because the plan required verifying the invariant it also asserted.

- **Iteration (Phase 2.2d)** — `for x in it` over any `Iterator`, plus six
  default methods, so iterating no longer means a hand-written `while` and a
  `match` on the `Option`.
  - **`for x in it` desugars to the loop you would have written**:
    `let mut __it = it` and then `while true { match __it.next() { Some(x) =>
    body, None => break } }`. The hidden iterator is bound `mut` regardless of
    the source expression's mutability, so `for x in <an immutable local>`
    advances that local; the loop variable itself stays immutable, exactly as in
    the range form. The integer-range form (`for i in 0..n`) is unchanged and
    still takes its own counter-driven path.
  - **`for x in v` is NOT supported — write `for x in v.iter()`.** There is no
    `IntoIterator`, deliberately, so a container is not an iterator. `for x in 5`
    and every other non-iterator reports `E0900` naming both accepted forms. The
    desugar keys on `next`'s *shape* rather than on std's `Iterator`'s identity —
    the same duck-typing string interpolation uses for `fmt` — so a user trait
    declaring `next` is as good an iterator as std's.
  - **Six default methods**: `map`, `filter`, `fold`, `count`, `any`, `collect`.
    `map` and `filter` are **lazy** — each returns an adapter record
    (`MapIter<Self, U>`, `FilterIter<Self>`) that pulls one element per `next`,
    so nothing is consumed until a consumer runs, and `f` is never called for
    elements nobody asks for. `collect` returns `Vec<Self::Item>`; `any`
    short-circuits (which is why it is not written over `fold`). Adapters chain
    on adapters, so `v.iter().filter(f).map(g).collect()` works as one
    expression.
  - **`collect` makes `std/core` depend on `std/collections` for the first
    time**, because it returns a `Vec`. Accepted deliberately: one method and one
    type, and the whole-program merge means there is no layering *mechanism* to
    violate, only a convention. The alternative, `Vec::from_iter(it)` in
    `std/collections`, keeps `std/core` free of collections and reads worse at
    every call site.
  - **A bound on a record's type parameter now resolves projections in field
    types, and is NOT enforced at construction** (`docs/adr/0007-record-parameter-bounds.md`
    §1). `record MapIter<I: Iterator, U> { it: I, f: fn(I::Item) -> U }` used to
    be `E0900`; it is now accepted, and the bound is a **resolution scope** —
    it exists so the field type may name `I::Item` at all. It is deliberately
    *not* a constraint, because `MakeRecord` carries no type arguments and MIR
    erases records to `Ptr`, so there is nowhere for the check to live that would
    fire reliably. This is the one thing in this increment a reader could
    reasonably be surprised by, so: what happens instead is **three different
    answers**, not one. Where a field type names the projection (std's two
    adapters), a wrong instantiation is `E0079` **at construction**, even
    undriven — earlier and stricter than the bound would have been. Where it does
    not, but a bounded impl method is instantiated, it is `E0013`. Where neither
    holds, it **compiles, runs and prints, with no diagnostic** — a residual hole,
    pinned by a test as accepted so it cannot be rediscovered as a bug or closed
    silently. A bound on a *sum type*'s parameter is still `E0900`.
  - **`Iterator`'s four consumers take plain `self`, not `mut self`** (ADR 0007
    §2, which amends ADR 0005 §1). `mut self` rejects a temporary receiver, so
    `v.iter().filter(f).map(g).collect()` — the form this increment exists for —
    was `E0060`. The consumers now open with `let mut it = self` and drive `it`.
    `next` remains the only `mut self` method, so driving an iterator by hand
    still needs a mutable binding. **The cost is real and is not the goal:** the
    same relaxation also lets a consumer accept an *immutable* local and advance
    it silently (`let it = …; it.count()` twice gives `3` then `0`, no
    diagnostic). ADR 0005's promise that "every std API that mutates must declare
    `mut self`" no longer holds, and ADR 0005 has been amended in place to say so.
  - **Adapters share their source by pointer, so mutating a source mid-iteration
    is observable.** A `MapIter` holds its inner iterator the way `VecIter` holds
    its `Vec` — records are heap objects, and there is no copy anywhere — so
    advancing an adapter advances its source, and a `push` to the underlying
    vector partway through a chain is visible to it. The same alias visibility
    `VecIter` already documented; preventing it needs borrow tracking Nova lacks.
    It is also what makes `let mut it = self` inside a consumer correct rather
    than merely convenient.
  - **The gate:** `tests/runtime/iterator.{nova,stdout}`, run three ways
    (`iterator_run`, `iterator_build_standalone`, `iterator_under_gc_stress`) —
    joining `collections`, `std_core`, `strings` and `assoc_types` among the
    fixtures driven through all three of those configurations. Every generic
    block in it is instantiated at `Bool` **and** at
    `Float`, which is not decoration: `mir_ty` collapses `Int` and `Char` to
    `MirTy::I64` and every heap type to `MirTy::Ptr`, so those are one machine
    class and a wrong `Item` hides in them. Measured on this fixture — an
    `Int`-only reduction of it survives a mutation that makes monomorphization's
    normalization cache its first answer **byte-identically**, and of the two
    distinguishable classes only the `Float` lines catch that one while the `Bool`
    lines pass. `Float` is the strictly stronger of the two — `Bool` is `MirTy::I8`
    and its only values 0 and 1 survive an `I64` confusion intact in the low byte
    of a register, whereas `Float` is `MirTy::F64` and crosses register banks — so
    the fixture carries both but the `Float` half is the one that must not be
    trimmed. (`assoc_types`' header records its own kill on the `Bool` half of the
    *same* mutation, so what flips which class catches it is the usage shape, not
    the mutation.)

- `@name(args)` attributes on items (Phase 2.2e): a leading `@name` or
  `@name(arg, ...)` immediately before a function, record, type declaration,
  trait, impl block, const, `import`, `module`, or `extern` block — every
  item kind that can carry one is validated the same way. Arguments are bare
  identifiers, not arbitrary expressions or `key = value` pairs. The
  recognized vocabulary is closed: an attribute name outside it is always
  `E0082`, never silently accepted. This matters most for `@test` itself — a
  mistyped `@tset` that compiled would define a function that looks like a
  test and never runs as one, the least visible member of this project's
  "parses, then enforces nothing" family (a record-parameter bound that
  bounds nothing when unused, an impl-level `const` parsed and discarded,
  `pub` on a method accepted and ignored, record field visibility parsed and
  never enforced). Adding a new attribute is therefore always a compiler
  change, not something a library can register on its own
  (`docs/adr/0008-attributes-and-test-isolation.md` §1).
- `@test` and `@test(should_panic)` (Phase 2.2e): mark a function as a test,
  collected in source order. `@test` on anything but a function is `E0083`;
  a `@test` function declaring parameters, generics, or an explicit
  non-`Unit` return type is `E0084`; an unknown `@test(...)` argument is
  `E0085`.
- `nova test [filter]` (Phase 2.2e): compiles the program once to a
  standalone test binary, then runs each collected `@test` function in its
  own process, one at a time — never in parallel and never sharing a process
  with another test, because a panic aborts with no unwinding anywhere in
  this runtime, so a shared-process runner could not survive one failing
  test to report on the rest. A finished test process is reported `ok`,
  `FAILED` (with the panic message printed beneath it), or `TRAPPED` (an
  exit code and nothing else — an illegal instruction, a segfault, whatever
  the runtime did not choose to do). The discriminator is whether stderr
  contains a `nova: panic:` line, never the raw exit code — the exit code is
  mediated by the platform and the shell and carries no message of its own,
  so on its own it cannot distinguish a checked panic from a hard trap
  (`docs/adr/0008-attributes-and-test-isolation.md` §2).
  **A hard trap is reported distinctly from a panic and does not satisfy
  `should_panic`** — integer division by zero is the example that matters,
  since it traps directly rather than reaching a checked panic path.
  `@test(should_panic)` inverts only the `Panicked` outcome; a trapped test
  still reads `TRAPPED` and still fails the run, and so does a
  `should_panic` test that neither panics nor traps. `[filter]` selects
  tests whose name contains the given substring anywhere, not only as a
  prefix; a filter matching no test still exits 0. `nova test` itself exits
  nonzero if anything failed or trapped.
  **A program's own `fn main` is shadowed under `nova test`, not treated as
  the entry point** — every pre-existing `main` is renamed before the test
  dispatcher is installed, so a program with both `@test` functions and an
  ordinary entry point (the common case: `nova build`/`nova run` both
  require a `main`, so a real program having one already is unremarkable)
  compiles its `main` but `nova test` never executes it. Not part of the
  original design — a Critical defect found and fixed during implementation
  (`docs/adr/0008-attributes-and-test-isolation.md` §3).
- `std/test` (Phase 2.2e): `assert`, `assert_eq` and `assert_ne`, each
  failing via `panic` with a message naming what was compared
  (`assert_eq`/`assert_ne` require `Eq + Debug`). Seeded only when compiling
  under `nova test`, not into every program: always embedding them would
  take `assert`, `assert_eq` and `assert_ne` away from every module of every
  ordinary program via glob import, the same hazard `join` hanging off the
  separator already avoided (`std/strings`, Phase 2.2b). Outside
  `nova test` those names are simply unresolved (`E0001`).
- Deferred, each for a stated reason rather than left unmentioned
  (`nova-spec/20-STDLIB.md` §11,
  `docs/adr/0008-attributes-and-test-isolation.md` §2): `assert_throws`
  (this runtime has no unwinding, so a
  panic cannot be caught, inspected, and recovered from —
  `@test(should_panic)` is the only supported way to assert that something
  panics, and there is no supported way to assert *what* it panics with);
  `@bench` (unrecognized by the resolver today — writing it reports
  `E0082`, the same as any other unknown attribute); parallel test
  execution and a per-test timeout (deferred together — a hanging test
  currently blocks `nova test` indefinitely, indistinguishable from slow
  work); and `nova test --doc` (`nova-spec/20-STDLIB.md` §15).
- Recorded, not implemented here: `nova-spec/50-TESTING.md` specifies a
  `tests/compile-pass` / `tests/compile-fail` harness (§§1.1-1.2, with the
  Rust integration-test shape — `insta`-snapshotted compiler diagnostics —
  in §2.1) and, separately, a `tests/ui` harness (§1.5: WASM tests via
  `wasm-bindgen-test`, headless-browser tests via Playwright); neither
  exists yet. `nova test` runs `@test` functions written in Nova; it is a
  different mechanism aimed at a different layer, and its existence
  narrows neither gap.

- `async fn`, `.await` and `Future<T>` (Phase 2.3a): an `async fn` returns a
  `Future<T>` of its declared return type, and `.await` inside another
  `async fn` suspends until that future produces its value. `Future<T>` is a
  compiler-known type rather than a std record — the compiler knows no
  library type by name, so there is no other place it could live — and it **is**
  nameable in source: `fn take(f: Future<Int>) -> Int { block_on(f) }`
  type-checks and runs. What a program cannot do is *construct* one without
  calling an `async fn`: there is no future literal and no constructor, and
  `Future` is not FFI-safe, so an `extern` signature cannot mention one either.
  Both backends compile the same
  transform, because it runs on the monomorphized MIR module that Cranelift
  and LLVM both consume. `.await` outside an `async fn` is `E0086`;
  `.await` on a non-future is `E0087` and names the type. `async` on a
  *trait* method is still refused, but by two different checks: in a trait
  declaration and in a default body it is `E0900`, while in an **impl** it is
  `E0072` — an `async fn` returns `Future<T>` and the trait declared `T`, so
  trait conformance rejects it as a return-type mismatch. `async` on an
  `extern` function is `E0900`. Async **inherent** methods are supported. Async
  closures are not in the grammar at all. A `@test` function may not be
  `async` (`E0084`): a test's result is discarded rather than awaited, so an
  `async` one's body would never run. Drive the future from a non-`async` test
  instead, as the diagnostic's note spells out — `@test fn t() {
  assert_eq(block_on(f()), 42) }`. The `assert_eq` is not decoration: a `@test`
  body must have type `()`, so a bare `block_on(f())` returning a value is
  itself `E0010`.
- **The execution model is single-threaded cooperative state machines, which
  reverses `docs/phase-2-plan.md` decision 1** (`docs/adr/0009-async-execution-model.md`
  §1). That plan recommended thread-per-task over Tokio as the pragmatic
  choice and deferred state-machine lowering; the recommendation was measured
  false *for this codebase*. Nova's GC heap is a `thread_local!`, so an object
  allocated on one task's thread is invisible to every other task's collector
  and is freed by its own thread's next collection while another thread still
  holds a pointer to it — a use-after-free reachable from ordinary source with
  no diagnostic. Making thread-per-task sound needs a global locked heap,
  stop-the-world coordination, all-thread stack scanning and safepoints:
  larger and subtler than the lowering it was meant to avoid, and it moves the
  risk out of the compiler and into the collector. Single-threaded state
  machines leave the thread-local invariant untouched. `nova-spec/13-RUNTIME.md`
  §4.2 (state machines) is honoured; §4.1 (Tokio, work-stealing thread pool)
  and §4.4 (cancellation) are deviations recorded in the ADR. **What that
  costs: real parallelism, and `spawn_blocking`** (`nova-spec/20-STDLIB.md`
  §13), which is not provided rather than provided as a synonym for `spawn`
  that would silently block the executor.
- `std/task` (Phase 2.3a) — the fourth embedded std module, glob-imported like
  the others: `spawn(fut) -> JoinHandle<T>`, `JoinHandle::join`, `yield_now()`
  and `block_on(fut)`, plus `async fn main`, which is driven by `block_on`.
  **`spawn` starts nothing on its own**: the task is queued, and the queue is
  drained only while a `block_on` call is running, so a `spawn` with no
  `block_on` above it never runs — silently. `join` **caches** rather than
  consumes, deliberately: Nova has no move checking, so a `fn join(self)`
  cannot prevent a second call, and the value is re-read from the future's own
  output slot instead. **`block_on` is not callable from inside an `async fn`**
  — a nested call would run a second executor loop inside the first one's
  frame, and the runtime ends the process with a diagnostic rather than
  corrupting the shared queue. It aborts rather than panicking for the reason
  the whole transform rests on: a generated poll function's frame has no
  landing pads and no unwind description, so no unwind may cross one.
- Known gaps in async, each disclosed during implementation rather than found
  afterwards, and each recorded in full in ADR 0009 §1: **no parking and no
  waking** — a task that reports pending is re-queued round-robin and
  re-polled, so waiting spins at one poll per turn rather than sleeping
  *(true of `yield_now` only as of 2026-08-10 — `sleep` and `join` now park
  instead of spinning; see the park set entry below and ADR 0009 §1's dated
  amendment)*;
  **`block_on` drains the whole queue**, so it implicitly joins everything
  spawned on the thread (unlike tokio's, which returns as soon as its own
  future resolves) **and does not terminate if any queued task never becomes
  ready** *(narrowed, not retracted, as of 2026-08-10: a genuine deadlock —
  every remaining task itself parked and no deadline left for any of them
  to wake on, e.g. two tasks joining each other — now aborts with a
  diagnostic instead of hanging; a task that never parks
  at all still leaves this un-terminating, since it alone keeps the queue
  non-empty forever; see below and ADR 0009 §1's dated amendment)* — no
  ordinary async program reaches this, since every suspension
  resumes on the next turn *(true only of Phase 2.3a, before this same
  2026-08-10 park set — `sleep` and `join` now suspend without resuming on
  the next turn, each waking on its own deadline or completion instead; a
  second, unforged route to an undiagnosed hang also exists today, since
  joining a task that loops on `yield_now` forever is a livelock rather
  than a deadlock and is deliberately not reported — see below)*, but it
  **was reachable** by forging a
  `JoinHandle`: the record's fields were public and task ids were `0, 1, 2, …`
  in spawn order with `block_on`'s own root spawned first, so
  `JoinHandle { id: 0, fut: … }` — **`id` is no longer a field of
  `JoinHandle<T>`; this no longer compiles, kept here as the record of the
  hazard** — inside that root made `join` wait for the task that was doing the
  waiting, so `nova check` reported `ok` while `nova run` hung with empty
  output. **Closed on branch `task-identity`**: `JoinHandle<T>` now holds only
  its future, and the two Nova-facing lookups resolve a task by the future's
  state address instead of a forgeable `Int`, so a hand-built handle can no
  longer name a task other than its own; the one remaining case, a handle on a
  future that was never spawned, now aborts instead of hanging. **What makes
  the first half of that true is that the executor's state-address index is
  pruned when the collector frees a state object**, not merely populated at
  spawn: an address is an identity only while the object at it lives (`gc.rs`'s
  module doc comment), so an entry left behind let a never-spawned future whose
  fresh state landed on a recycled address resolve to the old, released task —
  reported done, so `join` skipped its wait loop, the release no-opped, and the
  value read back was the caller's own never-polled output slot, `0` for `Int`
  and a null pointer for every heap class. Pruning costs `join`'s idempotence
  nothing rather than trading against it: a state object a handle can still
  reach is marked through that handle's own future, so it is never swept.
  Pinned by `tests/runtime/recycled_task_state.nova` under `NOVA_GC_STRESS=1`
  and, deterministically and on every platform, by
  `a_swept_states_key_is_dropped_so_a_recycled_address_cannot_misresolve`
  (ADR 0009 §1 has the full mechanism, the footgun the `spawn`-liveness check
  accepts, and that check itself); the park set and the deadlock diagnostic
  *(shipped 2026-08-10 — see the Added and Changed entries below, and ADR
  0009 §1's dated amendment)* are no longer owed; what remains owed is a
  primitive that parks on a genuine external event, `std/io`'s job, which
  this park set was built ahead of; **no cancellation** of any kind, so
  dropping a `JoinHandle` does nothing;
  **every temp is spilled into the state object**, not only those live across
  a suspend, which is what buys the transform out of liveness analysis at the
  cost of over-retaining (a later liveness pass narrows the state record and
  changes no semantics); **awaiting the same future twice re-polls a completed
  future**, re-running the body after its last suspend, because there is no
  move checking to forbid it; **a spawned task whose output is never taken
  leaks its state object**, the deliberate half of a trade whose alternative
  frees a heap-valued output while the task still names it; **each
  `yield_now()` costs four allocations**; and **the executor's
  out-of-range-poll-status check is still a `panic!`**, the one place an unwind
  can still cross a generated frame — kept because its precondition is a
  broken compiler and its observability is what a test asserts on.
- Gate: `tests/runtime/async_tasks.{nova,stdout}` — two tasks spawned, joined,
  yielding inside each so the executor interleaves them, and a `Float` result —
  driven through `nova run`, `nova build` and `NOVA_GC_STRESS=1`, joining
  `std_core`, `collections`, `strings`, `assoc_types`, `iterator` and
  `nova_test` among the fixtures registered in all three configurations. The
  GC-stress configuration carries the most weight of the three here: a
  suspended task's state object is reachable from no stack and no register, so
  its only root is the executor's entry in the collector's root registry, and
  collecting on every allocation is what distinguishes "the registry is
  populated" from "the registry is honoured" on real generated code. The
  interleaving rather than the totals is what the fixture exists to pin — a
  run-to-completion scheduler would print the same total with all of one
  task's lines before all of the other's — and the `Float` line is not
  decoration, since `Int` and every pointer-like type share one machine class
  and only `F64` crosses register banks.
- Recorded, not implemented in this slice: channels and `Mutex` (2.3b),
  `Instant`/`Duration`/`sleep`/`timeout` (2.3c) *(`sleep` and real parking
  and waking shipped 2026-08-10, below — see the park set entry under this
  same heading and the Changed entry for `join`/`block_on`; `Instant`,
  `Duration` and `timeout` are still absent from Nova source, and parking on
  an external I/O event is still `std/io`'s to add)*, `std/log` (2.3d),
  `spawn_blocking`, cancellation, async trait methods,
  async closures, liveness-based state minimization, and parallelism. A
  recursive `async fn` is accepted and recurses without bound, the same
  undiagnosed class as `fn f() { f() }`.
- **Park set (2026-08-10, a Phase 2.3a follow-up)** — the executor gained a
  park set: a way for a task to register *why* it is suspended (a deadline,
  or another task's id) and be pulled out of the ready queue until that
  deadline passes or that task finishes, instead of being re-queued and
  re-polled every turn regardless of what it is waiting for.
  `sleep(ms: Int)`, in `std/task`, is the first primitive to use it — it
  registers a deadline and is not polled again until that deadline passes,
  so a sleeping task costs no polls at all (`ms` is whole milliseconds; Nova
  has no duration type, and a negative value wakes immediately rather than
  being rejected). `join` now parks the same way, described under Changed
  below. `yield_now` stages no wait and keeps re-queueing itself every turn,
  deliberately — it is the primitive that means "let others run", not a
  wait on anything. When the ready queue drains with tasks still parked and
  no deadline left for any of them to wake on, the executor reports a
  deadlock — naming every parked task and what it is waiting for — and
  aborts, e.g. two tasks each `join`ing the other, or a task joining itself.
  This is not a general wake-on-I/O mechanism: parking on a genuine external
  event (a socket or pipe with nothing ready) is still `std/io`'s to add, and
  this park set exists ahead of it for exactly that reason
  (`docs/superpowers/specs/2026-08-10-park-set-design.md` §1). Two shapes of
  hang are deliberately still undiagnosed, both recorded as accepted
  limitations rather than defects: a task that never parks (loops on
  `yield_now`) keeps the ready queue non-empty forever and, by itself,
  suppresses the deadlock check even for two *other* tasks in a genuine
  cycle; and joining specifically that kind of task is a livelock, not a
  deadlock, and separating "will never finish" from "has not finished yet"
  is the halting problem, not an oversight
  (`docs/adr/0009-async-execution-model.md` §1's 2026-08-10 amendment).
- The six built-in type names — `Int`, `Float`, `Bool`, `Char`, `String`, and
  `Future` — are now reserved: declaring a `record` or a sum `type` under one
  of them is rejected (`E0089`) instead of compiling. `convert_ty` resolves
  each of these names to its built-in type, or to a shadowing generic
  parameter, wherever it runs — never to a user declaration — so such a
  declaration could never be referred to in a type annotation.
  **This is a breaking change, not a no-op**: construction and pattern
  matching resolve through a different path (a record literal through
  `resolve_type` directly; a sum type's variants through the value
  namespace) and worked, before this change, for a type declared under one
  of these names. An `impl` header's self type is a `convert_ty` site too,
  so an `impl` written for one of these names attaches to the built-in,
  never to the declaration — already true before this change. See the
  Changed entry below for exactly what breaks. Generic parameters and trait
  names are separate namespaces and are unaffected:
  `fn f<Int>(x: Int) -> Int { x }` and `trait Int { fn m(self) -> Int }`
  both remain legal.

- `std/io`'s error surface, eight `std/fs` functions plus `DirEntry`, and
  `eprint`/`eprintln` (increment 1 of the `std/fs`-on-Strings decomposition,
  `docs/superpowers/specs/2026-08-11-std-fs-strings-design.md`): Nova's first
  file I/O.
  - **`std/io`** gains `IoError { kind: IoErrorKind, message: String }` and
    `IoErrorKind`'s eight variants — `NotFound`, `PermissionDenied`,
    `AlreadyExists`, `InvalidData`, `Interrupted`, `TimedOut`,
    `ConnectionRefused`, `Other` — plus `io_error_kind_of(code: Int) ->
    IoErrorKind`, the Nova-side half of the status-code boundary described
    below. `AlreadyExists` and `InvalidData` are additions to
    `nova-spec/20-STDLIB.md` §4's original six, amended in place; see
    `docs/adr/0011-io-error-kinds.md`.
  - **`std/fs`** gains eight `async fn`s matching `nova-spec/20-STDLIB.md` §5's
    signatures exactly — `read_to_string`, `write_string`, `exists`,
    `create_dir`, `create_dir_all`, `remove_file`, `remove_dir_all`,
    `read_dir` — plus `record DirEntry { name, path, is_file, is_dir }` and
    `temp_dir() -> String`. `temp_dir` is a plain `fn`, not `async`: it only
    queries the environment and touches no filesystem, and it is not in the
    spec at all (`docs/adr/0011-io-error-kinds.md`). This is the
    `String`-and-`Bool` subset of §5 — `read`, `write`, `open` and `File` need
    a byte type Nova does not have yet, deferred to a later increment.
  - **`eprint`/`eprintln`** join `println`/`print`/`panic` in
    `Builtin::GLOBAL` (`[Builtin; 3]` to `[Builtin; 5]`), mirroring
    `println`/`print` at every seam. Like `panic` before them (Phase 2.1
    above), this makes both names reserved words: a user `fn eprint(...)` or
    `fn eprintln(...)` now reports `E0002: duplicate definition of 'eprint'`
    (or `'eprintln'`) with the note "is a compiler builtin", the same
    treatment `println`/`print`/`panic` already had — not the silent,
    lowest-priority shadowing `std/core`'s glob-imported items get under ADR
    0004, which is a different mechanism for a different `Builtin` category
    (`STD_ONLY`, not `GLOBAL`).
  - **The boundary: an intrinsic returns one word, and the status code is the
    error kind.** Nova has no out-parameters and building a Nova aggregate
    from Rust would duplicate its layout there — precisely the drift this
    project has already shipped a miscompile from once — so every `std/fs`
    intrinsic that can fail returns a status `Int` (`0` success, otherwise an
    `IoErrorKind` code) and, on success, leaves its payload (a string, a
    `[String]`, or an error message) in a thread-local, GC-rooted
    `Cell<usize>` slot for a follow-up intrinsic to take. `std/fs`'s Nova
    wrappers alone build every `Result`, `IoError` and `DirEntry`; no Nova
    aggregate layout enters Rust. The status numbering is one wire contract
    with independent copies in `crates/nova-runtime/src/fs.rs` and
    `std/io/lib.nova`'s `io_error_kind_of`, rather than a "keep in sync"
    comment — but a fixture pins only four of the eight kinds for real
    (`NotFound`, `AlreadyExists`, `InvalidData`, and `PermissionDenied` on
    Windows); the other four are either unreachable from `std/fs` or reachable
    but not yet exercised. See `docs/adr/0011-io-error-kinds.md`. Thirteen new
    `Builtin::STD_ONLY` intrinsics land this way (`[Builtin; 17]` to
    `[Builtin; 30]`), and `STD_MODULES` grows from 4 to 6 (`$std.io`,
    `$std.fs`).
  - **These eight `async fn`s never suspend**: each runs its filesystem
    operation synchronously inside the first poll, because there is no I/O
    poller yet (pinned by `no_filesystem_intrinsic_registers_a_park`, which
    asserts `fs.rs`'s own source contains no `stage_park` call). The
    signature is what matters for forward compatibility — it does not change
    when a poller lands — but the cost is real: a call blocks the whole
    executor for its duration, so a sibling task makes no progress during a
    large read. This is a new instance of the starvation
    `docs/adr/0009-async-execution-model.md` §1 already named, and that
    ADR's "the fix is always an `.await`" consequence is amended in place to
    say so (`docs/adr/0011-io-error-kinds.md`).
  - **`exists` returns `Bool`, not `Result`, per the spec** — so a path that
    exists but cannot be examined is indistinguishable from one that is
    absent. This is the spec's own choice and is left alone.
  - All three deviations from `nova-spec/20-STDLIB.md` and both limitations
    above are recorded in full, with the reasons behind each, in
    `docs/adr/0011-io-error-kinds.md`.

- `Bytes`, `std/bytes`, and byte-based `fs::read`/`fs::write` (increment 2 of
  the `std/fs`-on-Strings decomposition,
  `docs/superpowers/specs/2026-08-12-byte-type-design.md`): Nova's first byte
  type, closing the byte-type gap increment 1 (above) deferred `read`/`write`
  on.
  - **`Bytes` is a new nullary `hir::Ty` variant mapping to `MirTy::Ptr`**,
    represented exactly as `String` is: a scanned `{len, ptr}` header over a
    GC **leaf** buffer, reusing `crate::NovaStr` rather than a second Rust
    struct with the identical layout. `Bytes` and `String` are therefore
    structurally identical and semantically distinct — same representation,
    but `String` carries a UTF-8 guarantee and `Bytes` does not, and nothing
    converts between them implicitly. `Bytes` reaches codegen as the same
    `MirTy::Ptr` every other heap pointer already lowers to, so **neither
    codegen backend changes** — not because every `Bytes` operation is an
    intrinsic (`index_of`/`contains` are pure Nova control flow over
    `to_ints`, no intrinsic of their own), but because codegen dispatches on
    MIR types, and `Ptr` needed no new arm.
  - **`std/bytes`**, a seventh `STD_MODULES` entry, gives `Bytes` eight
    methods — `len`, `byte_at` (`Option<Int>`, matching `String::char_at`),
    `slice`, `concat`, `to_ints`, `to_string` (`Option<String>`, `None` on
    invalid UTF-8), `index_of` (`Option<Int>`), `contains` — plus two
    constructors, `bytes_from_string(s: String) -> Bytes` and
    `bytes_from_ints(ints: [Int]) -> Bytes` (aborts on an element outside
    `0..=255`), and `impl Eq for Bytes`. `index_of`/`contains` are Nova-level
    over `to_ints`, the same shape `std/strings` already builds over
    `str_chars` — but not every bounds check is: `slice`'s clamp, `bytes_at`'s
    intrinsic-level range guard and `from_ints`'s `0..=255` guard are
    Rust-level, and `byte_at`'s own Nova-level check tests `self.len()`
    directly rather than paying for a `to_ints()` conversion just to
    bounds-check.
  - **`fs::read(path: String) -> Result<Bytes, IoError>` and
    `fs::write(path: String, content: Bytes) -> Result<(), IoError>`** join
    `std/fs`, over three more intrinsics on increment 1's existing
    status-boundary. **No new thread-local slot**: `String` and `Bytes` share
    one payload slot (renamed `BUFFER_SLOT`), since both have the identical
    `{len, ptr}` representation and the distinction is carried entirely by
    which builtin stashed into it and which one reads it back.
  - **Thirteen new `Builtin::STD_ONLY` intrinsics land this way** (ten byte
    intrinsics plus the three above), taking `STD_ONLY` from `[Builtin; 30]`
    to `[Builtin; 43]`; `STD_MODULES` grows from 6 to 7.
  - **`Bytes` joins the six reserved built-in type names above via the
    identical `E0089` mechanism** (`RESERVED_TYPE_NAMES`: six names to
    seven). See the Changed entry below for exactly what breaks.
  - Deferred, per `docs/adr/0011-io-error-kinds.md`'s narrowed deviation:
    `open`, `File`, the `Read`/`Write` traits, and `stdin`/`stdout`/`stderr`.
    **References are removed from the roadmap entirely, not merely
    unimplemented** — Nova's byte I/O is buffer-returning, so
    `nova-spec/20-STDLIB.md` §4's `&mut [u8]`/`&[u8]` parameters are amended
    in place (dated note) rather than eventually built against.

- `std/io`'s `Read`/`Write` traits and the three standard streams
  (`docs/superpowers/specs/2026-08-13-read-write-and-stdio-design.md`):
  closes the deferral the `Bytes` entry above left open for the `Read`/
  `Write` traits and `stdin`/`stdout`/`stderr` — only `open`/`File` remain
  deferred now (`docs/adr/0011-io-error-kinds.md`, narrowed in place).
  - **`trait Read { fn read(self, max: Int) -> Future<Result<Bytes,
    IoError>>; fn read_to_end(self) -> Future<Result<Bytes, IoError>> {
    .. } }` and `trait Write { fn write(self, buf: Bytes) ->
    Future<Result<Int, IoError>>; fn flush(self) -> Future<Result<(),
    IoError>> }`**, plus fieldless records `Stdin`/`Stdout`/`Stderr`
    (built by lowercase `stdin()`/`stdout()`/`stderr()`) implementing them
    against the process's three standard streams.
  - **Both traits are spelled `fn … -> Future<T>`, not `async fn … -> T`.**
    `async fn` in a trait *declaration* is `E0900`, so neither trait can be
    declared the second way at all. Calling an `async fn` without `.await`
    produces its `Future` without running it, so a plain, non-`async` `fn`
    can still return one, unawaited — every method on both traits,
    including `read_to_end`'s default body, follows this shape.
    `nova-spec/20-STDLIB.md` §4 is amended in place (dated note) to record
    this, since the spec text there still shows the `async fn` spelling.
  - **`stdin`/`stdout`/`stderr` return the concrete records, not `impl
    Read`/`impl Write`**, because `impl Trait` in return position does not
    parse at all (`P0001`) — the same amendment covers this.
  - **EOF is an empty result from `read`; a short read is not EOF.** A pipe
    or a terminal may hand back far fewer bytes than asked for and still
    have more to give later, so `read_to_end`'s default loop keeps reading
    on anything non-empty and stops only on a genuinely empty result.
    Stopping as soon as a result looks "short" instead truncates output
    against the common case, not an edge case — a real pipe or terminal
    rarely hands back a chunk of exactly the size asked for.
  - **`Write::write` may write fewer bytes than `buf` holds, and reports
    exactly how many** — deliberately unlike `std/fs::write`'s write-all,
    count-less `Result<(), IoError>`. A caller that needs a guaranteed full
    write must loop on the returned count itself.
  - Five new `Builtin::STD_ONLY` intrinsics (`io_stdin_read`,
    `io_stdout_write`, `io_stderr_write`, `io_stdout_flush`,
    `io_stderr_flush`) land this way, taking `STD_ONLY` from `[Builtin; 43]`
    to `[Builtin; 48]`. `STD_MODULES` and `RESERVED_TYPE_NAMES` are both
    unchanged at 7 — nothing new is reserved, and no existing program
    breaks.
  - `docs/adr/0009-async-execution-model.md` §1 gains a second instance of
    its cooperative-scheduling hazard, worse in degree than `std/fs`'s:
    with no I/O poller yet, `Stdin::read` blocks the whole executor for as
    long as the OS read takes, so a program that spawns tasks and then
    reads stdin on an interactive terminal with nothing typed stalls every
    other task on the thread until the user sends EOF — a filesystem read
    finishes on its own; a terminal read waits on a human. Amended in place
    (dated note).
  - `open` and `File` remain deferred — the same two items
    `docs/adr/0011-io-error-kinds.md` tracks: building either needs a
    handle with real lifetime management, which `Stdin`/`Stdout`/`Stderr`
    (process-global, always open, never closed) do not. `OpenOptions`,
    `open`'s parameter type, does not exist yet either, but it is not a
    separately deferred item in its own right — just part of the
    signature `open` has not shipped.
  - **`print`/`println`/`eprint`/`eprintln` are deliberately unchanged.**
    They are synchronous language-level output primitives seeded into
    `Builtin::GLOBAL` (Phase 2.1 above), calling `nova_rt_print`/
    `nova_rt_println`/`nova_rt_eprint`/`nova_rt_eprintln` directly — not
    `std/io` trait methods, and not routed through `Stdout`/`Stderr` or the
    `Write` trait. Nothing about this increment touches them, gives them a
    `Future`-returning signature, or makes them fallible; they stay exactly
    the fire-and-forget calls they already were.

- `File`, `open`, `OpenOptions`, and `impl Read`/`impl Write for File`
  (increment 3c of the `std/fs`-on-Strings decomposition, the last of it —
  `docs/superpowers/specs/2026-08-14-file-open-and-openoptions-design.md`):
  `open`/`File` were `docs/adr/0011-io-error-kinds.md`'s last remaining
  deviation from `nova-spec/20-STDLIB.md`, now **closed rather than narrowed
  further** — every item that ADR ever named as deferred is shipped.
  - **`File` is Nova's first value holding an OS resource across more than
    one intrinsic call.** Every `std/fs` function before it opens, acts and
    closes inside a single intrinsic, and increment 3b's three standard
    streams are process-global and never closed, so nothing shipped so far
    needed real handle lifetime management. `File` does:
    `File { fd: Int }`, where `fd` keys a new thread-local table of open
    `std::fs::File`s (`crates/nova-runtime/src/file.rs`), not an OS file
    descriptor number.
  - **`OpenOptions`** is a record of six `Bool` flags — `read`, `write`,
    `append`, `truncate`, `create`, `create_new` — with `impl Default`
    (every flag false) and three named constructors, `reading()`,
    `writing()` (write + create + truncate) and `appending()` (append +
    create), each forwarding one for one onto `std::fs::OpenOptions`'s own
    method of the same name. No chainable builder: a receiver-mutating
    method cannot be called on a temporary (`E0060`, measured), so
    `OpenOptions::reading().with_write()` does not compile in this
    language — an exotic combination starts from `OpenOptions::default()`
    plus field assignment on a `let mut` local instead.
  - **`pub async fn open(path: String, options: OpenOptions) -> Result<File,
    IoError>`**, plus an inherent `pub async fn close(self) -> Result<(),
    IoError>` on `File` — plain `async fn`, not the `Future<T>` spelling
    increment 3b's *trait* methods needed, because that spelling is forced
    only where `async fn` is illegal (a trait declaration) and `close` is
    not one — and `impl Read for File` / `impl Write for File` over the same
    two traits increment 3b shipped.
  - **`close` is idempotent, and any other operation on a closed `File` is
    an ordinary `IoError { kind: Other }`, never a panic, an abort, or a
    read of freed memory.** Nova has no move checking, so `self` by value
    cannot stop a second call to `close`, and no `IoErrorKind` variant
    exists for "closed" — the message names the condition instead. A forged
    handle (`File { fd: 9999 }`, constructible because Nova has no field
    privacy) gets identical treatment, because **absence from the handle
    table is what closedness is**: a closed fd, a stale fd and a forged fd
    all miss the same lookup and become the same error.
  - **Explicit `close` is the only release mechanism, and forgetting it
    leaks the descriptor until the process exits — on every platform,
    deliberately.** The collector already has a per-object notification
    hook it could in principle use (`gc.rs`'s sweep calls
    `task::forget_freed_state` on every freed address), but `File`'s
    `fd: Int` representation makes using it impossible rather than merely
    unbuilt — the collector never sees an integer field inside a record —
    and collection does not run at all off Windows, so a backstop reachable
    only there would encode a guarantee two of three supported platforms
    cannot honor. Recorded in a new ADR, rather than fixed:
    `docs/adr/0012-file-descriptor-lifecycle.md`.
  - Five new `Builtin::STD_ONLY` intrinsics
    (`file_open`, `file_close`, `file_read`, `file_write`, `file_flush`),
    taking `STD_ONLY` from `[Builtin; 48]` to `[Builtin; 53]`. **`STD_MODULES` and
    `RESERVED_TYPE_NAMES` are both unchanged at 7** — `File` and
    `OpenOptions` are ordinary `std/fs` definitions, glob-imported and
    shadowable like any other, so no name is reserved and no existing
    program breaks.
  - `std/io`'s `decode_count` — the function decoding a write's byte count
    back out of the 8 little-endian bytes it crosses the runtime boundary
    as — is now `pub`, so `std/fs`'s `File` wrappers can call it directly
    for `open`'s new fd and `Write::write`'s byte count, rather than
    duplicating it. Forced, not chosen: Nova's `Visibility` has exactly two
    levels, `Pub` and `Private` — there is no `pub(crate)` tier — and only a
    `Pub` item enters a module's exports, so `pub` was the only mechanism
    available at all. Widens `std/io`'s public surface as a side effect.
  - `nova-spec/20-STDLIB.md` §5 is amended in place (dated notes) to add
    `OpenOptions`'s `pub record` declaration, with no `pub` on any field —
    Nova has no field privacy to enforce either way — together with its
    `impl Default` and the `impl OpenOptions` block holding `reading()`,
    `writing()` and `appending()`, bodies elided in that section's own
    style. The section had named `OpenOptions` as `open`'s parameter since
    before this increment existed but never declared it as a type, and its
    own prose described the `Default` impl and the three constructors while
    the code fence showed neither. Amended also to record that `File`
    carries an `Int` descriptor with explicit `close`, not the opaque shape
    the code sample there still shows. Two one-sentence gaps increment 3b
    left in §4
    are also closed: a short write is legal (`Write::write` may report
    fewer bytes than `buf`
    holds — already true of increment 3b's shipped `write`, just not
    previously stated), and `nova_rt_io_stdin_read` charges a generous `max`
    in full, allocating its whole capacity eagerly before any read happens
    (`File::read` shares the identical shape).
  - Four new fixtures: `file_roundtrip.nova` (open for writing, write,
    close, reopen for reading, `read_to_end`, close — exercising both trait
    impls and the `Read` default increment 3b shipped), `file_lifetime.nova`
    (idempotent close; a closed handle and a forged handle both fail as an
    ordinary error rather than a panic or a hang), `file_errors.nova` (two
    portable failures — `create_new` on an existing path, `open` under a
    missing parent directory — ungated, on every platform), and
    `file_open_dir.nova` (opening a directory for reading,
    `#[cfg(windows)]`, split out from `file_errors.nova` so its two portable
    checks run everywhere rather than only where the directory check needs).

- **The I/O poller and `std/net`** (`docs/superpowers/specs/2026-08-15-io-poller-and-std-net-design.md`,
  `docs/adr/0013-io-poller.md`): the executor's third wake source, and
  Nova's first standard-library networking — a program can now connect to a
  loopback server, write, read with a timeout, and close, with a sibling
  task demonstrably running while it waits.
  - **A new module, `crates/nova-runtime/src/poll.rs`, gives the executor a
    third wake source: socket readiness**, alongside the existing deadline
    (`sleep`) and task-completion (`join`) sources. `Wait` gains a third
    variant, `Wait::Io { socket, interest, deadline: Option<Instant> }` —
    the deadline rides inside the variant rather than becoming a second
    `PARKED` entry — and `stage_park`'s staging area widens from one `Wait`
    slot to a `deadline`/`io`/`task` triple, so a `read_timeout` can stage a
    deadline and an I/O wait together in one poll, the one new legal
    combination. The poller itself is `select` on Unix and `WSAPoll` on
    Windows behind one `#[cfg]` seam, both reached through one
    platform-independent `poll::wait(sockets, deadline)`.
  - **The wait happens only where `run_to_completion`'s ready queue drains,
    not once per task turn.** The existing per-poll `wake_due` check (a
    cheap `Vec` scan) still runs after every single poll so a self-requeuing
    task cannot starve a due deadline or a due I/O timeout, but the real
    wait — now covering both a deadline and a socket set at once — is
    reached only once nothing is left to run, exactly where
    `std::thread::sleep` used to be called directly. That call is now
    deleted from `task.rs` entirely: an empty socket set with a timeout *is*
    a sleep, and `poll::wait`'s own empty-set branch does it, so there is no
    second timing path to keep in sync.
  - **A task parked on I/O is never reported as a deadlock.** The
    drained-queue branch now matches both the earliest deadline and the
    live socket set at once, and reaches `report_deadlock()` only when
    *both* are empty — a park set holding even one `Wait::Io` instead blocks
    in `poll::wait`, however long that takes. `docs/adr/0009-async-execution-model.md`
    §1 gains two footguns of this same shape in a 2026-08-16 amendment: a
    permanently-runnable task starves I/O the identical way it already
    starves a deadline (joining that existing footgun rather than opening a
    new family), and a program waiting on a peer that never sends hangs
    with no diagnostic at all, following this project's own livelock
    precedent — telling "will eventually answer" from "never will" is the
    halting problem.
  - **`std/net`**, an eighth `STD_MODULES` entry (`"$std.net"`,
    `STD_MODULES` 7 → 8): `TcpStream { fd: Int }` — an `Int` key into a
    runtime-owned socket table, not an OS handle, the same shape `File`
    already established — with `connect(addr: String) -> Result<TcpStream,
    IoError>`, an inherent `close`, an inherent `read_timeout(max, ms)`
    (no `std/fs` analogue takes a deadline, so this is a second method
    beside `close` rather than folded into `Read`), and `impl Read`/`impl
    Write for TcpStream` reusing `std/io`'s existing traits. `close` is
    idempotent and any operation on a closed, stale, or forged handle
    (`TcpStream { fd: 9999 }` compiles — Nova has no field privacy) is an
    ordinary `IoError { kind: Other }`, never a panic, identically to
    `File`. Five new `Builtin::STD_ONLY` intrinsics (`net_connect`,
    `net_close`, `net_read`, `net_write`, `net_read_timeout`) land this way,
    taking `STD_ONLY` from `[Builtin; 53]` to `[Builtin; 58]`.
    `RESERVED_TYPE_NAMES` stays at 7 — `TcpStream` is an ordinary `std/net`
    definition, glob-imported and shadowable like any other, so no name is
    reserved and no existing program breaks.
  - **`connect`, `read`, `write`, and `read_timeout` genuinely suspend the
    calling task when they cannot complete immediately** — unlike every
    `std/fs` operation shipped so far, and unlike `std/net`'s own `close`
    and `flush`, neither of which suspends: `close` calls a plain
    status-returning intrinsic with no `.await` at all, the identical shape
    `std/fs`'s `File::close` already uses, and `flush` is a hardcoded
    `Ok(())` with no runtime call whatsoever, since one `write` here is
    already one unbuffered syscall with no userspace buffer to push out.
    The four that do suspend are Rust-built futures with their own poll
    functions (`crates/nova-runtime/src/net.rs`), parked through the new
    poller exactly as `sleep` parks on a deadline, joining the family of
    hand-written poll functions `task.rs` already contains
    (`poll_yield_once`, `poll_sleep`) as the first such functions outside
    `task.rs` itself. `connect` is a two-phase parker: a non-blocking
    socket and `connect` syscall on the first poll, parked on *write*
    readiness, then a `SO_ERROR` check (via `std::net::TcpStream`'s own
    `take_error`) on the second poll to tell a genuine connection from a
    refusal — a blocking `connect` would pass every loopback test in this
    project while defeating the whole increment, so the non-blocking
    two-phase shape is specified out, not merely chosen. `read`/`write`
    have no such phase split: the same non-blocking attempt is retried on
    every poll, which is also what gives them `poll_join`'s own
    would-already-be-ready optimisation for free. `read_timeout` computes
    its absolute deadline once, on its first poll, then re-derives "ready
    vs. timed out" on every later poll by retrying the read first and
    consulting the stored deadline only on a would-block — the same pattern
    `finish_connect` already uses to re-derive a refusal from `SO_ERROR`
    rather than being told one occurred, not a wake-reason channel (an
    earlier design draft's per-task-slot-table description of this was
    factually wrong and is corrected in place,
    `docs/superpowers/specs/2026-08-15-io-poller-and-std-net-design.md` §3.5).
  - **`ConnectionRefused` and `TimedOut` gain their first producers.** Both
    have carried their status-code numbering since increment 1
    (`docs/adr/0011-io-error-kinds.md`) with nothing able to produce either,
    because no filesystem operation can: `connect` against a loopback port
    nothing is listening on is `std/net`'s first `ConnectionRefused`, and
    `read_timeout` against a peer that never answers before the deadline is
    its first `TimedOut`. `crates/nova-runtime/src/fs.rs`'s pinned-kinds
    status comment goes four fixture-pinned kinds to six accordingly,
    dated in place rather than silently rewritten — only `Interrupted`
    remains unreachable from either module's `async fn`s now.
  - **Five new fixtures**, each run under `nova run` against a real
    loopback `TcpListener` a small test-only echo server binds per fixture
    (`crates/nova-cli/tests/run_tests.rs`'s `EchoServer`): `net_roundtrip.nova`
    (connect, write, read, close — the correctness baseline); `net_interleave.nova`
    (the fixture that decides the increment — asserts a spawned counter
    task's own output lands *between* a socket task's "wrote" and "read"
    lines, which only a real, non-blocking poller can produce, since a
    merely-correct round trip cannot tell a real poller from a blocking or
    busy-spinning one underneath the identical Nova-level surface);
    `net_timeout.nova` (`read_timeout` pinned in both directions — a
    deadline shorter than the peer's own delay must report `TimedOut`, and
    one comfortably longer must still succeed with the real data);
    `net_refused.nova` (`ConnectionRefused` against a reserved-then-dropped
    loopback port); and `net_lifetime.nova` (idempotent `close`, and a
    closed, stale, or forged handle all treated as an ordinary error,
    following `file_lifetime.nova`'s own shape).
  - `nova-spec/20-STDLIB.md` gains a `std/net` section (§16, appended rather
    than inserted so no existing numbered section — several of which are
    cross-referenced by number elsewhere in this repository — needs
    renumbering) and a §4 note on the asymmetry this increment leaves
    standing: **`std/fs` suspends nowhere and `std/net` suspends
    everywhere**, two different wrapper shapes now both live in this
    stdlib — an honest consequence of regular files not being
    readiness-pollable the way sockets are, on any of this project's three
    CI platforms, not an inconsistency to reconcile.
  - **Threads and a completion-based file-I/O poller (IOCP/`io_uring`) were
    both considered and declined, for two different reasons.** A poller
    thread or a worker pool would be invisible to `PARKED`, `QUEUE`,
    `SLOTS`, `FILES`, and the collector's own thread-local roots — the
    identical use-after-free argument `docs/adr/0009-async-execution-model.md`
    §1 already makes against thread-per-task generally, applied here to a
    narrower case. IOCP is declined for a separate, scope reason instead:
    it is the only route that would make `std/fs` itself genuinely
    suspend, and building it is a subsystem of its own, out of scope for an
    increment whose surface is `std/net`. Full reasoning, including why the
    thread-local argument does not by itself decide the IOCP question, is
    in `docs/adr/0013-io-poller.md`.

### Changed (Phase 2 — behaviour changes)

Filed here as well as under Added, because each of these changes the meaning of
code that already compiled. Full detail is in the `### Added` entries above.

- **A `mut self` trait method now requires a mutable receiver** (`E0060`), closing
  ADR 0005 §1's gap. Calling one through an immutable binding was silently
  accepted and did mutate. Fix: `let mut x = …` at a call site, `mut x: T` at a
  parameter. Reachable through five routes including string interpolation, so a
  program can hit this without an obvious method call.
- **Three constructs that compiled are now rejected**: a projection in an impl's
  self type (`E0900`), a trait declaring the same associated type twice, and a
  trait declaring the same method name twice (both `E0403`). All three were
  silently accepted, and the first was invisible to overlap checking.
- **A `record` or sum `type` declared under a reserved built-in type name is
  now rejected** (`E0089`; full detail in the Added entry above), rather than
  compiling. This breaks a program that declared one and either used it only
  through construction, pattern matching, and inferred local bindings, or
  did not use it at all — naming the type in a signature, or giving it a
  method, already did not work. Accepted rather than fixed a different way,
  because a type that can be built and matched but never named in any
  annotation, or given a method, is a trap not worth leaving open.
- **`JoinHandle::join` no longer spins, and `block_on` can now abort instead
  of hanging forever (2026-08-10, a Phase 2.3a follow-up).** `join` used to
  re-poll `task_is_done` once per turn until its target finished, spinning
  at 100% CPU the whole time; it now parks on the park set below and wakes
  only on the target's completion. Consequently `block_on`'s drive loop,
  which used to hang forever if a queued task never became ready, now
  detects the one case where every remaining task is parked and none can
  wake — most simply, two tasks joining each other, or a task joining
  itself — and aborts with a diagnostic naming each one and what it is
  waiting for, instead of hanging. This does not make every hang
  diagnosable: a task that never parks (loops on `yield_now`) keeps the
  ready queue non-empty forever regardless, and joining it is a livelock the
  executor still cannot tell apart from slow progress. Full detail,
  including both limitations this does not close, is in the park set entry
  under Added above and in `docs/adr/0009-async-execution-model.md` §1's
  2026-08-10 amendment.
- **A top-level `fn` or `const` named `eprint` or `eprintln` is now rejected**
  (2026-08-11, the `std/fs`-on-Strings increment): joining `Builtin::GLOBAL`
  seeds both names into every module's scope ahead of user item collection,
  so a same-named top-level declaration is now `E0002` ("is a compiler
  builtin"; full detail in the Added entry above) instead of compiling.
  Before this increment neither name was resolved to anything, so
  `fn eprint(s: String) { ... }` was ordinary, working, callable code — a
  **stronger** break than the reserved-built-in-type-names entry above,
  whose type was already unusable in a signature or an impl before that rule
  existed. A local `let`-binding or parameter of either name is unaffected:
  the conflict is checked only where `Builtin::GLOBAL` is seeded, at
  top-level item collection, which a local binding never reaches. Accepted
  as the same trade `panic`'s Phase 2.1 entry above already made:
  `eprint`/`eprintln` are ordinary language-level output primitives, not
  `std/core`-scoped implementation details like `str_cmp`, so they take
  their names away from user code the same way panic's did, and this is a
  different mechanism from — not an instance of — `std/core`'s silently
  shadowable, glob-imported names (ADR 0004), which are seeded at the
  *lowest* priority instead of the highest.
- **`Bytes` is now a reserved built-in type name: a `record` or sum `type`
  declared under it is rejected** (`E0089`; full detail in the Added entry
  above), joining the six built-in type names already reserved above
  (`RESERVED_TYPE_NAMES`: six names to seven). This breaks a program that
  declared a type named `Bytes` and either used it only through construction,
  pattern matching, and inferred local bindings, or did not use it at all —
  naming it in a signature, or giving it a method, already did not work, for
  the identical `convert_ty`-resolves-to-the-builtin reason the six-name
  entry above gives. Accepted rather than fixed a different way, for the
  identical trap-not-worth-leaving-open reason.

### Fixed (Phase 2)
- An allocation whose size is too large to *describe* now aborts with a Nova
  diagnostic instead of a Rust panic and backtrace. `gc::alloc` built its
  `Layout` with `Layout::from_size_align(size, ALIGN).expect("valid heap
  layout")`; at `ALIGN = 16` a size that rounds up past `isize::MAX` makes that
  call fail, and the `expect` ended the process with "thread caused
  non-unwinding panic" — an `expect` on a path reachable from user input, which
  the repo convention forbids. It was reachable at the very top of the *legal*
  array-length range: `[x; MAX_ARRAY_LEN]` asks for `8 * len + 8` =
  9223372036854775800 bytes, which both length guards accept and `ALIGN` rounds
  8 bytes too far. The check now lives in `gc::alloc`, the choke point every
  allocation site in the language funnels through (records, strings, closures,
  sum construction, `Vec`/`Map`/`Set` growth), rather than in one lowering, and
  it asks `Layout::from_size_align` whether the size is legal rather than
  restating its rule. The message names both the request and the limit
  ("allocation of N bytes exceeds the maximum object size of M bytes"), so it
  is distinguishable from a genuine out-of-memory, which still reports
  "memory allocation of N bytes failed" through `handle_alloc_error` and is
  what a merely-too-big length such as `2^40` produces. The neighbouring
  behaviours are unchanged: `[x; -1]` and `[x; 1 << 60]` still abort in the
  lowering's own guards, since those exist to stop a *different* bug (the
  wrapping `8 * len + 8` size arithmetic collapsing to a tiny block).
- A trait bound on a **record** or **sum type** type parameter
  (`record Keyed<K: Hash, V>`, `type Wrap<T: Hash> = …`) is now rejected with
  `E0900` instead of being silently discarded. It parsed and then enforced
  nothing: `hir::RecordType`/`hir::SumType` carry no bounds, and monomorphization
  discharges only function and impl bounds, so `Keyed { k: NoHash { … }, v: 2 }`
  compiled and ran with a `NoHash` that had no `Hash` impl.
  *(Superseded in Phase 2.2d for **records only** — and note that the
  `Keyed { k: NoHash { … }, v: 2 }` behaviour described here is deliberately back:
  it is case 3 of `docs/adr/0007-record-parameter-bounds.md` §1, accepted so that
  a field type may name a projection on a bounded parameter, which is what lazy
  iterator adapters require. The mechanism sentence above — no bounds on
  `RecordType`, mono discharging only function and impl bounds — is still exactly
  true, and is the reason the bound is still not enforced. The **sum-type** half
  of this entry stands unchanged.)* Enforcing the bound
  instead would need a notion of "record instantiation site" that no pass has —
  a record's type arguments survive only in the enclosing expression's `Ty`,
  `ExprKind::MakeRecord` does not record them, and MIR erases them — so the
  construct is rejected loudly, following the precedent set for
  `trait B where Self: A`. Write the bound on the `impl` block instead
  (`impl<K: Hash + Eq, V> Map<K, V>`), where it *is* enforced; that is what
  `std/collections` already does, so no stdlib or test-suite program changes.
  Bounds on functions, impl blocks, generic trait methods, and `where` clauses
  are unaffected. `nova-spec/20-STDLIB.md`'s `Map`/`Set` declarations, which had
  shown the unenforced form, now show the enforced one.
- A `${…}` string-interpolation hole now ends at the `}` matching its `${`
  rather than at the first `}`, so an expression containing braces works inside
  one — most visibly a record literal (`"${f(R { v: 1 })}"`), nested to any
  depth, and a block expression (`"${if a { 1 } else { 2 }}"`). Previously these
  produced two confusing errors, the first being "expected `}` (in record
  literal), found `}`", and every affected call site had to bind the value to a
  local first. A `}` inside a nested string, char, or raw-string literal within
  the hole is text, not structure (`"${g("}")}"`). A hole left unclosed is now
  reported as "unterminated string interpolation" instead of cascading into
  parse errors. A record literal in a hole also parses where the enclosing
  string sits in an `if`/`while`/`for`/`match` scrutinee.
- Cross-module symbol collision: two modules each defining a same-named item
  (function, generic function, or same-named type's inherent method) no longer
  collapse to one symbol at monomorphization. Symbols are now mangled by their
  owning `DefId`, fixing silent wrong-dispatch and a memory-unsafe type
  confusion; qualified/nested import paths (`a::b`) are rejected with `E0900`.
- A generic sum type used as a record field or a sum-variant payload
  (e.g. `record Slot { tag: Option<Int> }`) no longer gets a spurious `E0012`
  "expects 0 type arguments": type arity is precomputed, so it no longer depends
  on collection order (this also fixes forward-referenced generic records).
- Importing a module that exports a name coinciding with a prelude name
  (`Option`/`Result`/`Some`/`None`/`Ok`/`Err`) no longer raises a spurious
  `E0002`: the prelude is glob-imported last, as the lowest-priority binding, so
  a local definition *or* an import of the same name shadows it.
- Nested generic type annotations whose closing brackets abut (`Option<Option<
  Int>>`, `Box<Box<Int>>`) now parse: a glued `>>`/`>>>` token is split when
  closing generic argument lists (the `>>` right-shift operator is unaffected).
- Calling an `extern` function whose C symbol cannot be resolved no longer
  crashes `nova run` with a Rust panic — the JIT's finalize-time panic is caught
  and reported as a clean `E0902`, mirroring the `nova build` linker error.
- Two modules declaring the same C symbol with conflicting signatures now report
  `E0075` instead of crashing codegen / emitting invalid LLVM IR.
- Self-less methods (Phase 2.1): an impl method declaring no `self` receiver
  had its parameter types silently shifted by one slot — signature
  collection unconditionally prepended the receiver's type ahead of every
  method's declared parameters, even when the method had none. Three
  independent symptoms followed from the one root cause, all now fixed: a
  silent miscompile when the shifted types happened not to conflict (a later
  parameter checked against the wrong declared type, with no diagnostic
  produced at all); a bogus `E0001: no variant 'f' on type 'Type'` when
  calling such a method by qualified syntax (a two-segment path was until
  then understood only as a sum-type variant constructor); and a Cranelift
  ICE that `nova check` had already accepted — `nova check` reported exit 0
  for a self-less method called on an instance (`p.make()`), and only `nova
  run`/`nova build` crashed, with a verifier error ("mismatched argument
  count: got 1, expected 0") surfaced as "internal codegen error (this is a
  compiler bug)". Self-less methods are now tracked explicitly, so their
  signatures are never shifted, and calling one on an instance is a clean
  `E0014`. The same family of bug existed on the trait-method dispatch path
  too — a receiver-less trait method called on an instance ICE'd the same
  way, and a trait/impl pair disagreeing about whether a method takes
  `self`, in either direction, was accepted with no conformance check at
  all — now `E0014` and `E0072` respectively, with no ICE either way.
- `Type::f()` on an inherent impl no longer dispatches by impl declaration
  order. An associated function is selected by the impl's nominal head alone
  (deliberately, so `Box::make(5)` works before the qualifier's type argument
  is known), so two *disjoint concrete* impls of one generic type —
  `impl Box<Int> { fn tag() }` and `impl Box<Bool> { fn tag() }` — were both
  candidates and the first one declared silently won. Coherence does not catch
  that pair either: their self types do not overlap, so there is no `E0074`.
  Reordering the two `impl` lines changed the program's output with no
  diagnostic at all. Now every candidate is collected and an ambiguous
  qualifier reports `E0015`, mirroring the trait-associated-function path; the
  single-candidate case is unchanged.
- Record field *assignment* diagnostics now match the field *read* path's
  wording: an unknown field on a record reports "no field `x` on record `P`"
  (it used to say "no field `x` on type `P`"), and a receiver that is not a
  record at all now gets its own "cannot access field `x` on `Int`" message
  instead of being folded into the unknown-field one — the distinction the
  read path already made. `check_field_set` also no longer drops an
  independent mistake on the right-hand side when the field name itself is
  wrong: `p.nope = undefined_fn()` now reports `undefined_fn` as unresolved
  too, rather than only the unknown-field error, matching how the array path
  (`a[i] = undefined_fn()`) already behaved. The cascade guard for a receiver
  that is already `Ty::Error` is unchanged — it still reports exactly one
  error, not two.
- `Debug for String` now escapes its contents into a valid Nova literal
  (Phase 2.2b): `("a\"b").dbg()` previously produced `"a"b"`, which is not
  valid Nova source — noted as a known limitation in the `std/core` entry
  above. The fix reuses `Debug for Char`'s existing per-character escape
  table (`\\`, `\n`, `\t`, `\r`, `\0`) through one shared private helper,
  decoding the string with the new `str_chars` builtin (see `std/strings`
  above) rather than the dedicated `nova_rt_str_escape` that `std/core`'s
  stale comment had predicted — so the fix needed no new ABI symbol.
  `String` escapes `"` where `Char` escapes `'`, and additionally escapes
  every `$` as `\u{24}`: a string literal (unlike a char literal) opens an
  interpolation hole on `$` immediately followed by `{`, so a literal `$`
  left unescaped in the output could silently reopen one when pasted back
  as source — the whole-branch review caught this gap in the initial fix,
  where a string built to contain `${` still printed it unescaped. With that
  arm in place, both `Debug for Char` and `Debug for String` round-trip back
  through the lexer to the original value.
- `std/fs`'s payload-passing slots move from three thread-global slots to one
  per-task table (2026-08-13, branch `per-task-slots`,
  `docs/superpowers/specs/2026-08-12-per-task-payload-slots-design.md`): the
  `thread_local! { Cell<usize> }` slots (`BUFFER_SLOT`, `ARRAY_SLOT`,
  `MESSAGE_SLOT`) that carry a payload from a status-returning intrinsic to
  the wrapper that reads it back are now one `RefCell<Vec<Slots>>`, indexed
  by the current task (index `0` reserved for no task). This closes a latent
  cross-task clobber that today's straight-line, never-suspending `async
  fn`s cannot reach (`docs/adr/0011-io-error-kinds.md`) but a future I/O
  poller would: once a `std/fs` call can suspend between its stash and its
  take, a second task polled in the gap would previously have stashed into
  the same thread-global slot, overwriting the first task's payload and its
  GC root. `release_task_slots(id)` is called immediately after each of the
  two points `task.rs` already releases a task's state root
  (`release_internal`, `take_output_internal`), so a task's payload is
  released in the same call that releases its state, not deferred to some
  later, separate event. Every `SLOTS` access is a fallible borrow (`try_borrow_mut`/
  `get_mut`), never `borrow_mut`, aborting instead of panicking on the
  believed-unreachable contended case — a `RefCell` panic here would cross a
  generated poll boundary with no landing pad. `no_slot_access_can_panic_on_a_borrow`
  (`fs.rs`) mechanically pins that borrow half, plus `unwrap()`/`.expect(`/`panic!`/
  `format!` (final review, M1); that no access is written as indexing (`[i]`)
  instead is held by review, not by the guard, since a substring scan cannot see it.
  **Does not fix** the pre-existing leak of a task whose output is neither
  taken nor released (`docs/adr/0009-async-execution-model.md` §1): such a
  task's last unread payload still leaks until the process exits, one per
  leaked task now instead of one per thread. No Nova-visible signature
  changed; `std/fs/lib.nova` was not touched.

### Known limitations / follow-ups (not blockers for this pre-release)
- **Phase 2 is incomplete.** Unstarted: `std/time`, `std/log`, `std/sync`,
  `std/http`, `std/json`, `std/crypto`. Partial: `std/net` is a TCP **client**
  only where the spec calls for TCP and UDP; and `std/fmt` has no module of its
  own — its `print`/`println`/`eprint`/`eprintln` surface ships as compiler
  builtins, while the `Formatter` builder in `nova-spec/20-STDLIB.md` §3 is not
  delivered. The Phase 2 gate needs `examples/05-json-api` and
  `docs/benchmarks/`, neither of which exists yet.
- Precise GC stack bounds remain Windows-only: `gc::stack_base` returns `None`
  everywhere else, so collection is skipped there (leak-until-exit). The eight
  `#[cfg(windows)]` root tests that exercise a real conservative scan stay
  `#[ignore]`d and run advisory-only in CI, per
  `docs/adr/0010-conservative-scan-root-test-gating.md`.
- A task whose output is neither taken nor released still leaks its last
  payload until the process exits (`docs/adr/0009-async-execution-model.md` §1).
- Files are deliberately blocking: no OS reports readiness for regular files,
  so `std/fs` reads and writes do not park. Only sockets, timers and joins do.
- `nova build --release` needs a discovered LLVM toolchain on the machine —
  `clang` (or `NOVA_CLANG`), falling back to `llc` (or `NOVA_LLC`).
- The MSRV leg checks `--workspace` but not `--all-targets`, so an MSRV
  violation in a test or bench still passes unnoticed; widening it is blocked
  on an `assert_cmd` floor. `Cargo.lock` is now tracked, which pins the
  dependency set CI resolves — so upstream breakage no longer surfaces unaided.

## [0.1.0] - 2026-07-23

Phase 1 (MVP compiler) milestone. Gate verified: all gate programs compile and
run via `nova run` (Cranelift JIT) and `nova build` (native executables), the
workspace test suite is green, and `clippy -D warnings` + `cargo fmt --check`
pass.

### Added (Phase 1 — MVP compiler)
- `nova run <file>`: compile and execute Nova programs natively via the
  Cranelift JIT; `nova check <file>`: type-check only
- `nova build <file> [-o out]`: compile to a standalone native executable —
  Cranelift object emission with an exported C `main` wrapper, linked
  against the `nova-runtime` static library via the platform linker
  (MSVC `link.exe` through cc-rs on Windows, `cc` elsewhere); gate
  programs produce ~130 KB executables
- `nova-resolver`: item-level name resolution (functions, sum types,
  variants), builtin prelude (`println`, `print`), E0002 duplicates
- `nova-typeck` + `nova-hir`: Hindley-Milner inference with occurs check,
  explicit generics at function boundaries, sum types with minimal
  exhaustiveness checking (E0020), typed & desugared HIR output
- `nova-mir`: monomorphization reachable from `main`, CFG lowering,
  match compilation to switches, short-circuit lowering
- `nova-runtime`: C-ABI strings, console output, sum allocation
  (leaking allocator pending GC — ADR 0002)
- `nova-codegen-cranelift`: MIR → native code via cranelift-jit
- Records: declarations, literals (explicit fields, shorthand, `..base`
  spread), field access, generic records; boxed as tagless heap structs
- Traits: inherent methods, trait declarations with required and default
  methods, trait impls, method-call resolution by receiver type
  (E0015 on ambiguity), generic trait bounds verified at monomorphization
  (E0013), impl conformance (E0070/E0071), static dispatch; string
  interpolation bridges to a user `Display` (`fmt(self) -> String`)
- For loops over integer ranges (`for i in a..b` / `a..=b`), desugared to
  a counter-driven `while`
- Closures (`|x| body`) with by-value capture, and bare functions used as
  values: both compile to fat pointers `{ code, env }` with an env-first
  ABI; lifted to standalone functions and monomorphized like generics
- `break` and `continue` in `while`/`for` loops (E0080 outside a loop);
  `continue` in a `for` still advances the counter
- Top-level `const NAME: T = value` (compiled as a zero-arg function,
  referenced by call); constants may reference other constants; a cyclic
  constant is reported as E0081
- Arrays `[T]`: literals `[a, b, c]`, indexing `arr[i]`, element assignment
  `arr[i] = v` (mutable base required), and `arr.len()`; out-of-bounds
  access aborts with a message (heap layout `{ len, elems… }`)
- Match exhaustiveness and reachability via Maranget's usefulness algorithm:
  a `match` is non-exhaustive (`E0020`) when a wildcard row is still useful
  against the arms, and the diagnostic names witness patterns for the uncovered
  values (`Some(_)`, `false`, `_`); an arm is unreachable (`E0021`) when it is
  useless against the earlier arms. This fixes `match` on `Bool` being rejected
  when both `true` and `false` are covered, and detects redundant arms that the
  previous catch-all-only check missed
- Generic impl blocks: `impl<T> Box<T> { … }` (inherent) and
  `impl<T> Trait for Box<T> { … }` (trait), with the impl's type parameters
  usable in method signatures and bodies; a method is monomorphized per
  instance by recovering the impl's type arguments from the receiver type.
  Conditional impls `impl<T: Bound> Trait for Box<T>` are supported — the
  bound on the impl's parameter is verified at monomorphization (E0013),
  including transitively through nested generic impls (`where` clauses on
  impls remain unsupported)
- Garbage collector: `nova-runtime` now reclaims heap memory with a
  conservative, non-moving mark-and-sweep collector (`gc.rs`), replacing the
  leaking allocator (supersedes ADR 0002). All heap values — records, sums,
  arrays, closures, and strings — route through `gc::alloc`; collection is
  triggered at allocation past a growth threshold. Roots are found by scanning
  the stack plus callee-saved registers (flushed via a small `setjmp` C shim),
  and marking is range-based so interior pointers keep their object alive. It
  needs no codegen support and no external GC library. `NOVA_GC_DEBUG` logs
  collections; `NOVA_GC_STRESS` collects on every allocation (used to validate
  root scanning — the whole e2e suite passes under it). Precise stack bounds
  are implemented on Windows; other platforms retain leak-until-exit for now
- `nova build --release`: optimizing build through a new LLVM backend
  (`nova-codegen-llvm`) that emits textual LLVM IR from MIR and compiles it
  with a discovered LLVM toolchain (`clang`, or `llc`, at `-O2`; override with
  `NOVA_CLANG`/`NOVA_LLC`), then links via the same platform linker as the
  debug build. The IR mirrors the Cranelift backend's layouts and runtime ABI,
  so a program behaves identically across `nova run`, `nova build`, and
  `nova build --release`. Requires LLVM ≥ 15 (opaque pointers); with no
  toolchain found, the generated `.ll` is left in place with a clear message
- Gate programs verified end-to-end: hello-world, fibonacci,
  match-on-enum, generic functions, records, traits, for-loops, closures,
  break/continue, constants, arrays, generic impls (e2e stdout tests under
  both `nova run` and `nova build`)

### Fixed
- Lexer: leading whitespace in a string segment directly after `${expr}`
  was skipped when resuming string mode
- Lexer: position drifted into comment text because the logos wrapper
  advanced by the token length, ignoring skipped comment/whitespace
  trivia — any program with a comment failed to parse
- Lexer: a comment immediately before a string (spurious error cascade)
  or raw string (silent mis-lex to `Ident("r")` + plain string) — comments
  are now skipped before literal dispatch (adversarial review)
- Typeck: a trait impl whose method signature diverged from the trait
  declaration (arity, parameter, or return type) was accepted and
  miscompiled — a wrong parameter type was memory-unsafe; now `E0072`
  (adversarial review)
- `nova check` now runs monomorphization so it catches unsatisfied trait
  bounds (`E0013`) that previously only `nova run`/`nova build` rejected
  (adversarial review)
- Record literal field initializers now evaluate in source order (adversarial review)
- Typeck: closure return type is resolved through inference (closure bodies
  previously returned `()` and discarded their result)
- Typeck: closures now capture all referenced enclosing locals — assignment
  targets and called function values were missed, causing miscompiles or a
  compiler panic (adversarial review)
- Typeck: `for` loops use an independent hidden counter (assigning the loop
  variable is rejected, not silently corrupting the trip count), unscoped
  counter locals (no name capture), and an overflow-safe inclusive form
  (an inclusive range ending at `Int::MAX` no longer loops forever)
  (adversarial review)
- Typeck: an `if`/`match` with a diverging branch (`return`/`break`/
  `continue`) is typed by its non-diverging branch instead of `Never`, and
  the `while` lowering guards a diverging condition — a `Never`-typed
  condition (e.g. `while (if c { return x } else { b }) {}`) previously
  crashed codegen with an internal error (adversarial review)
- Typeck: `break`/`continue` in a loop's own condition now target that loop
  (were rejected as "outside a loop" or mis-scoped) (adversarial review)
- Typeck: a function-typed value that is not a local (e.g. a fn-typed
  constant `CONST(args)`, or a fn returned from a field/call) can now be
  called directly instead of erroring with E0900 (adversarial review)
- Typeck: element assignment `arr[i] = v` now checks the mutability of the
  place's root binding, walking through field and nested-index projections —
  `rec.data[0] = v` and `grid[0][1] = v` on an immutable binding, and
  `make()[0] = v` on a temporary, were silently accepted and mutated
  immutable heap storage; now `E0060` (adversarial review)
- Typeck: a restricted inherent method (`impl<T> Pair<T, T> { … }`) no longer
  shadows an applicable trait method for a receiver it does not fit (e.g.
  `Pair<Int, String>`) — method resolution now falls through to the trait impl
  instead of rejecting the call, and the string-interpolation `Display` bridge
  is likewise no longer blocked (adversarial review)
- Monomorphization: the trait-bound satisfaction check no longer has a
  recursion depth cap that could accept an unsatisfiable bound for a very
  deeply nested conditional-impl type; the recursion is well-founded on the
  finite structure of the type, so deep nests are checked exactly
  (adversarial review)
- Impl selection is now structural at every site. `resolve_method_full` (trait
  dispatch) and the monomorphization bound check selected the *first* impl
  sharing a type head, so a program with two non-overlapping impls for the same
  head (`impl Foo for Pair<Int, Bool>` + `impl Foo for Pair<Int, String>`) was
  accepted or rejected depending on declaration order; both now scan for the
  impl that structurally fits (adversarial review)
- Impl methods are mangled by their full self type, not just its head, so two
  concrete impls sharing a head (`Pair<Int, Bool>` vs `Pair<Int, Int>`) no
  longer collide to one symbol and miscompile each other's calls (adversarial
  review)
- Overlapping implementations are rejected (`E0074`): two trait impls of the
  same trait whose self types share a ground instance, or two inherent impls
  that overlap and define the same method (Phase 1 has no specialization);
  previously dispatch silently depended on declaration order (adversarial
  review)
- An impl type parameter that never appears in the self type is rejected
  (`E0073`) instead of leaking an uninferrable variable that made every method
  on the impl uncallable (adversarial review)
- Exhaustiveness: an empty `match` on a value of generic type (`match x { }`
  where `x: T`) was silently accepted and trapped at runtime; a match with no
  arms is now reported non-exhaustive (`E0020`) for any inhabited scrutinee
  (adversarial review)
- A bare identifier pattern that names a nullary variant of a *different* sum
  type is now rejected (`E0001`) rather than silently treated as a catch-all
  binding, which had masked uncovered cases and produced a spurious
  unreachable-arm warning — matching the `Path`/`TupleStruct` pattern arms
  (adversarial review)
- LLVM backend: a `match` on a `Bool` emitted `switch i64` over the `i8`
  scrutinee, producing a type-mismatched module that LLVM rejects (so
  `--release` failed for any boolean match); the switch now uses the
  discriminant's own type (adversarial review)
- `nova build`: intermediate object/IR files are now named so they can never
  alias the `-o` output path — previously `-o out.ll` (or `-o out.obj`) made an
  intermediate the output file and deleted the built binary on success
  (adversarial review)

### Known limitations / follow-ups (not Phase 1 blockers)
- Precise GC stack bounds are implemented on Windows; other platforms skip
  collection (leak-until-exit) until their stack-bounds query is added
- `nova build --release` needs an LLVM toolchain (`clang`/`llc`, ≥ 15) on the
  machine to produce the final binary from the emitted IR
- Spec drift to reconcile in Phase 2: chumsky 0.9 (spec calls for 0.10),
  `salsa` not yet integrated, `fuzz/` targets not yet written

## [0.0.0] - 2026-05-10

Phase 0 (Foundation) milestone. Gate verified: `examples/01-hello-world` and
`examples/02-fibonacci` parse to AST with zero errors.

### Added
- Initial workspace setup (Phase 0)
- `nova-diagnostics`: error reporting infrastructure with codespan-reporting
- `nova-lexer`: full token set for Nova source files (logos-based)
- `nova-ast`: AST node type definitions
- `nova-parser`: recursive descent + Pratt parser (chumsky-based)
- `nova-cli`: `nova parse <file>` command for parser testing
- CI workflow (cargo test, fmt, clippy)
- Snapshot testing harness via `insta`
- Example files: hello-world, fibonacci
