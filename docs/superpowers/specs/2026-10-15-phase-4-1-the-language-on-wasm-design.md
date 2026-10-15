# Phase 4.1, "The language on WASM": design

- **Date:** 2026-10-15.
- **Status:** for the user's review.
- **The user's decisions, 2026-10-15:**
  - the async executor is written in Nova, in `std/rt`, and JS only sets
    timers and starts turns;
  - the WASM target gains the std modules `task`, `time`, `log` and
    `sync`;
  - the collector runs only between turns;
  - the gate is every `tests/runtime/` program that uses only WASM's std,
    and `examples/03-producer-consumer`;
  - `block_on` works on WASM: it runs the executor within the call, and
    waits for a timer by watching the clock.
- **Branch:** `claude/loving-archimedes-frtf5q`.
- **Inputs:**
  - `docs/phase-4-plan.md` §3 (decisions 3, 4 and 5) and §4's 4.1 entry;
  - the 4.0 spec, `docs/superpowers/specs/2026-10-14-phase-4-0-foundations-design.md`;
  - ADR 0039, the WASM backend and its runtime;
  - `crates/nova-runtime/src/task.rs`, `gc.rs` and `gc/pages.rs`, which
    this sub-phase ports.

## 1. What 4.1 delivers

After 4.1, a WASM build runs async code, sleeps and timeouts, logs, and
uses `Mutex` and channels, with output identical to the native build's.
Memory that a program no longer reaches is reclaimed between turns.

1. **WASM's std** gains `task`, `sync`, `time` and `log` (§3). A program
   that names a native-only module gets an error that names the target,
   E0092, instead of today's E0001 or the misleading E0900.
2. **The executor**, ported from `nova-runtime` to Nova in `std/rt` (§4).
   It keeps the native one's order: spawn order, round-robin, timers in
   the order they were parked, the deadlock report.
3. **The event loop.** `async fn main` returns to JS once nothing more can
   run without a timer. JS sets a timer for the earliest deadline, and
   each timer starts a turn (§5). `block_on` in synchronous code runs the
   executor within the call, and waits for a deadline by watching the
   clock (decision 5).
4. **The allocator and the collector** (§6). The bump allocator becomes a
   page allocator with size classes, ported from `gc/pages.rs`. A
   conservative mark-sweep collector runs at the end of a turn, when the
   WASM stack holds nothing, with a stress mode for the tests.
5. **The shim** gains the two clocks, the turn loop and the collector's
   switches (§7).
6. **ADR 0040** records the collector, the executor and `block_on` on
   WASM (§10).

**The gate:**
- every `tests/runtime/` program that uses only WASM's std, built with
  `--target wasm` and run under Node, behaves as its native build does, on
  all three operating systems: 94 of the 134 programs (§9.2);
- so does `examples/03-producer-consumer`;
- the 40 programs left out are listed in the tests, each with its reason.

## 2. The starting point (read on 2026-10-15, at `93dcc5a`)

- **The 134 programs in `tests/runtime/`, built for WASM today:**
  - 51 run as natively: the 46 with a `.stdout` file that 4.0's build
    already prints (41 in 4.0's gate, and 5 more), and the 5 panicking
    programs of 4.0's gate;
  - 83 do not build. Of these:
    - 43 need only `task`, `time`, `log` or `sync` beyond 4.0's six
      modules (§9.2);
    - 37 need `fs`, `io`, `net`, `http`, `process` or `crypto`;
    - 3 never can: `extern_ffi` (E0090), `bytes_reserved` (a program
      whose E0089 is the point) and `nova_test` (a test file, run by
      `nova test`).
- **How they fail:**
  - a qualified call into a module WASM lacks (`Instant::now()`,
    `Mutex::new`) is E0900, "module-qualified paths are not supported
    yet" (`crates/nova-typeck/src/check.rs`, `check_path`);
  - a free function or a type is E0001;
  - a runtime function the build lacks is E0091.
- **Memory.** Of the 46 passing programs with a `.stdout` file, 45 stay
  within the first 16 MiB of memory with the allocator that never frees.
  `json_render_guard_under` reaches 24.6 MiB.
- **The executor** (`crates/nova-runtime/src/task.rs`) is about 1,700
  lines of code and 77 tests:
  - its state is thread-local: a task table, a FIFO run queue, a list of
    parked tasks, and a map from state address to task id;
  - its leaf futures (yield, sleep, join, timeout) are poll functions in
    Rust;
  - `block_on` drains every task, and blocks the thread to wait for a
    deadline or for I/O.
- **The collector** (`crates/nova-runtime/src/gc.rs`, `gc/pages.rs`; ADR
  0020):
  - it is conservative, non-moving mark-sweep, with no object header;
  - small objects (2,048 bytes or less) live in 64 KiB pages of one size
    class each, with three bitmaps per page;
  - large objects are allocated alone;
  - it finds roots by scanning the native stack, plus a registry of pinned
    states;
  - `NOVA_GC_STRESS` collects at every allocation, and `NOVA_GC_DEBUG`
    prints a line per collection.
- **WASM objects** carry no size or type (4.0 spec §4.4):
  - every pointer slot is 8-aligned;
  - the only interior pointer stored in memory is a runtime string's own
    `ptr` word, at `+16`.
- **`std/rt`'s state words:** 8, 16 and 24 are used, and 32 to 56 are free.

## 3. std per target

### 3.1 The modules

WASM's std becomes core, bytes, collections, strings, fmt, **task, sync,
time**, json, **log**, and `$std.rt`.
- The order is native's. Std modules see each other's names by a glob
  import of the lowest priority, so the same order gives the same winners.
- `log` needs `time` (`SystemTime::now()` in its line format), and `sync`
  needs `task` (`yield_now`): a module set without them would fail to
  check in every build.

Native-only: `fs`, `io`, `net`, `http`, `process` and `crypto`.

### 3.2 E0092, a name in a module the target lacks

- **The resolver** collects, for each std module the target does not
  compile, the names it exports: values, sum variants, types and traits.
  It stores them in `Definitions`, mapped to their module.
- **The type checker**, at each place where a name does not resolve,
  consults that map first. If the name is there, the error is E0092,
  labelled on the name:

  > E0092: `Instant` is in `std/time`, which target wasm does not have

  The message uses `CompileTarget::name()`. Otherwise E0001 or E0900
  stands as today.
- A user's own definition of the name shadows the map, as any definition
  does.
- None of the native-only modules adds an `impl` to a built-in type, so
  method calls need no map.

### 3.3 Runtime functions by kind

`RtFunc::on_wasm()` changes for these:

| Runtime function | Kind on WASM |
|---|---|
| `TaskSpawn`, `TaskBlockOn`, `TaskIsDone`, `TaskTakeOutput`, `TaskRelease`, `TaskSleepFutureNanos`, `TaskJoinFuture`, `TaskTimeoutFuture` | `std/rt` (§4) |
| `TaskYieldFuture` | built in by the backend (§4.3) |
| `TimeNowNanos`, `TimeNowEpochNanos` | host (§7.1) |
| `LogConfigLevel`, `LogConfigToStderr`, `LogSetConfig` | `std/rt` (§4.6) |

`ShutdownRequested` and the I/O functions stay unavailable: only
`std/http`'s server loop calls the former.

## 4. The executor, in `std/rt`

### 4.1 What it keeps from the native one

The port reproduces `task.rs`'s behaviour exactly where a program can see
it. Each item below cites the native code it follows.
- **Tasks.**
  - A task id is its index in the task table, from 0, across every
    `block_on`.
  - Tasks are never removed.
- **Spawning.**
  - `spawn` queues the task at the back, without running it.
  - Spawning a future that is already a live task aborts with the native
    message. Spawning one whose task was released gives a new task.
- **The run queue is FIFO.**
  - A child spawned during a poll is queued before its parent is
    re-queued or parked.
  - A task that yields goes to the back.
- **Parking.**
  - A leaf future stages a wait (a deadline, a task, or a task with a
    deadline) before it returns pending.
  - A task that returns pending with a staged wait is parked; without one,
    it is re-queued.
  - Deadlines merge to the earliest. Two waits on tasks in one poll abort
    with the native message.
  - A wait staged by a poll that then returns ready is discarded.
- **Waking.**
  - A task's completion queues every task waiting on it, in the order they
    were parked.
  - Deadlines that are due wake in the order they were parked, not by
    deadline. They are checked after every poll while anything is parked,
    so a task that spins on `yield_now` cannot starve a sleeper.
- **Sleep and timeout.**
  - A deadline is fixed at the leaf future's construction: the first poll
    of `sleep`'s or `timeout`'s body. Its value is `now` plus the duration,
    with negatives counted as 0, saturating.
  - A sleep that is already due completes without suspending.
  - A timeout polls its inner future first, then checks its deadline. It
    abandons the inner future without cancelling it.
- **Join.** It resolves its target once, at construction. An unspawned
  future aborts with the native message, "nova_rt_task_join_future: this
  future was never spawned…".
- **Release** is idempotent. **The root's output** is taken once.
- **`block_on`.**
  - It aborts if called while a `block_on` runs, with the native message.
  - It queues the root behind earlier spawns, drains every task, and
    returns the root's output.
- **Deadlock.** When tasks are parked and none can wake, it writes the
  native report to stderr, then aborts with status 134:

  ```
  nova: deadlock: N task(s) ... parked and none can wake
    task {id} is waiting for task {t} to finish[, with a deadline]
  ```

### 4.2 Its state

- Nova has no globals. The executor's tables live in the heap, and a state
  word points at them (§6.1). They are:
  - the task table: each task's poll code, state, done flag, output and
    released flag;
  - the run queue;
  - the parked list;
  - the current task;
  - the staged wait.
- The tables are leaf objects (§6.3): the collector does not trace them.
  Its roots include each unreleased task's state instead, as native pins
  them (§6.4).
- The map from state address to task id is a scan of the task table. Its
  ids are pruned after a collection, as native's are, so a reused address
  cannot name an old task.

### 4.3 Leaf futures, in Nova

A future is `{poll code, state}`, and a poll function on WASM has the type
`(i32 state, i32 ctx) -> i64` (ADR 0039).
- **`yield`, built in by the backend.**
  - `TaskYieldFuture` becomes an intrinsic: the backend emits one small
    poll function. Its first poll sets the tag and returns pending, with
    no wait staged; its second writes the output and returns ready.
  - The backend allocates its state and its future through `rt_alloc`.
- **Polling a future from `std/rt`.**
  - A new WASM-only builtin, `future_poll(future) -> Int`, polls a future
    once. It lowers to `call_indirect` with a null context.
  - Only `std/rt` can name it, as with the memory builtins.
- **`sleep`, `join` and `timeout`** are `async fn`s in `std/rt`. Each loops
  on a check, stages its wait, and awaits a fresh yield future. For
  example:

  ```
  async fn rt_sleep_until(deadline: Int) -> Int {
      while time_now_nanos() < deadline {
          rt_stage_deadline(deadline)
          task_yield_future().await
      }
      0
  }
  ```

  - The yield future's pending return carries the staged wait, so the task
    parks. When the deadline wakes it, the loop checks again.
  - The timeout polls its inner future with `future_poll` on each turn of
    its loop. It snapshots the staged wait first, and restores it when it
    completes.
  - This is the native poll functions' logic, written with one primitive.

  The plan's prototype confirms it on the gate's programs.
- **The runtime functions** `rt_task_sleep_future_nanos`,
  `rt_task_join_future` and `rt_task_timeout_future` compute the deadline
  or resolve the target, then return the `async fn`'s future.

### 4.4 `block_on` and `async fn main`

- **`block_on`** runs the executor loop within the call:
  - drain the run queue;
  - if parked tasks remain and none has a deadline, report the deadlock;
  - otherwise wait for the earliest deadline by reading the monotonic clock
    in a loop, then wake what is due and drain again.

  The waiting loop holds the thread, as native `thread::sleep` does, but it
  uses the CPU: a page that sleeps inside `block_on` does not respond until
  it returns. `async fn main` is the event loop's way.
- **`async fn main`.**
  - On WASM, the main shim does not call `block_on`. It spawns the root and
    runs the executor until nothing can run without a timer, then returns
    to JS.
  - The function that does this is a root of the build, so the second
    routing pass after the async transform cannot drop it.
  - Turns then continue from JS (§5) until no task remains.

### 4.5 Panics and aborts

- A Nova panic in any turn writes `nova: panic: <msg>` and exits 134, as
  in 4.0. The executor's aborts are panics with the native messages.
- The deadlock report needs an abort without the panic prefix. A new host
  function, `abort`, throws in the shim and exits 134 with nothing more
  written.

### 4.6 The log configuration

- `rt_log_config_level`, `rt_log_config_to_stderr` and `rt_log_set_config`
  keep the configuration in state words.
- Zeroed memory reads as native's default, level 2 (`Info`) to stderr. The
  setter stores the level as given and the flag as 0 or 1.
- Log lines go through `println` and `eprintln`, which are host functions
  since 4.0.

## 5. The event loop

- **A turn** is one call from JS into the module that runs Nova code:
  - `main`;
  - or the exported turn function, `nova_turn`, which a timer calls.
- **`nova_turn`:**
  1. wakes what is due;
  2. drains the run queue;
  3. collects if the threshold is met (§6.5);
  4. returns the delay until the earliest deadline, in milliseconds and
     rounded up; or -1 when no task is parked.
- **The shim's loop:**
  - After `main` returns, if the module exports `nova_turn`, the shim
    calls it with a timer for the delay it returned, until it returns -1.
    The delay is clamped to 2^31-1 ms, the most `setTimeout` takes.
  - A timer can fire before the deadline, so `nova_turn` re-arms when
    nothing is due yet.
  - When the loop ends, the shim flushes output and Node exits with status
    0.
  - A panic in any turn ends the program as in 4.0.
- **Only programs that use the executor** export `nova_turn`. A program
  without async code keeps 4.0's shape: `main`, then exit.
- **One clock.** Every deadline and every check uses `time_now_nanos`, the
  same clock as `Instant`. So `elapsed >= d` after `sleep(d)` holds, as
  `time_elapsed` requires.

## 6. The allocator and the collector

### 6.1 State words

- `DATA_BASE` moves from 64 to 128. The state words become 8 to 128, with
  room for:
  - the allocator's and collector's metadata;
  - the executor's tables (§4.2);
  - the log configuration (§4.6);
  - the collector's switches (§6.6).
- The plan assigns each word. Literal addresses shift by 64, and the tests
  that pin them change.

### 6.2 Pages and size classes, ported from `gc/pages.rs`

- **Small objects** (2,048 bytes or less) take a slot in a 64 KiB page of
  one size class: 24 classes, from 16 to 2,048 bytes. The class table and
  rounding are native's.
  - Each page has three bitmaps, one bit per slot: allocated, marked,
    scanned.
  - A page table, indexed by `(address - heap base) >> 16`, finds a page's
    descriptor.
  - A page is WASM's page size, so `memory.grow` adds pages directly.
- **Large objects** take a run of whole pages, recorded in the page table.
- **Every slot is zeroed when handed out.** Code relies on zeroed memory:
  - the shim's strings write only the low half of a pointer word;
  - an `I8` is stored as one byte;
  - unit fields are never written.
- **Empty pages** go to a reserve of up to 16, as native's do. WASM memory
  cannot shrink, so pages beyond the reserve stay with the allocator, free
  for any class.
- An allocation over 1 GB is refused with "out of memory" before any page
  arithmetic.

### 6.3 Leaf objects

- Strings and `Bytes` hold no pointers but their own `ptr` word, so their
  slots are not traced.
  - `rt_str_alloc` allocates them as leaves.
  - The shim's strings use a second export, `nova_alloc_leaf`.
- The executor's tables are leaves too (§4.2).

### 6.4 Marking and sweeping

- **A word is a pointer** if it falls inside an allocated slot or large
  object, interior pointers included: native's range test.
- **The mark phase** uses an explicit worklist in linear memory, not
  recursion, since V8's stack is about 1 MB. It traces every word of each
  non-leaf object it marks.
- **The roots:**
  - each unreleased task's state;
  - the executor's tables;
  - whatever later sub-phases register: 4.2's table of Nova values that JS
    holds will be one.

  The WASM stack holds nothing at the end of a turn, so there is no stack
  to scan. The data segment holds no heap pointer.
- **The sweep** keeps what is marked and frees the rest by bitmap, as
  native's does. It then prunes the executor's map of states (§4.2).

### 6.5 When it runs

- **At the end of a turn:** after `main` returns, and at the end of each
  `nova_turn`. The collector never runs inside an allocation: the shim
  allocates from within host functions, while Nova frames are live.
- **The threshold is native's:**
  - a collection runs once the bytes allocated since the last one reach
    `next_gc`;
  - `next_gc` starts at 1 MiB, then becomes `max(1 MiB, 2 × live bytes)`.
- **A long computation within one turn does not collect.** The memory's
  limit is 1 GB, and a program that allocates past it within one turn
  stops with "out of memory". 4.1's programs stay far below it (§2).

### 6.6 Stress and debug

- WASM has no environment. Under Node, the shim reads `NOVA_GC_STRESS` and
  `NOVA_GC_DEBUG` and passes them in through an export, before `main`. In
  a browser, `init(options)` takes `gcStress` and `gcDebug`.
- **Stress** collects at the end of every turn.
- **Debug** writes native's line to stderr after each collection:

  ```
  nova-gc: collection C freed F bytes, N objects live (B bytes)
  ```

## 7. The shim

### 7.1 Host functions

- **`time_now_nanos`** returns monotonic nanoseconds since its first call,
  as a `BigInt`. It is computed as
  `BigInt(Math.floor((performance.now() - origin) * 1e6))`. Every step is
  non-decreasing.
- **`time_now_epoch_nanos`** returns `Date.now()` times 10^6 as a `BigInt`.
  It returns 0 before 1970, and saturates at `i64::MAX`. Its resolution is
  1 ms, which `to_iso8601`'s milliseconds do not show.
- **`abort`** ends the program with status 134 and writes nothing (§4.5).

Each comes into `app.js` only when the module imports it, as in 4.0.

### 7.2 The turn loop and the switches

- After `main`, the shim runs §5's loop when the module exports
  `nova_turn`.
- Before `main`, it passes the collector's switches in (§6.6).
- `init` resolves when the loop ends, so a caller can await the whole
  program.

## 8. Crates

- No new crate; `Cargo.lock` does not change.
- `nova-codegen-wasm` gains:
  - the yield intrinsic;
  - the `future_poll` lowering;
  - the exports `nova_turn`, `nova_alloc_leaf` and the collector's switch.
- `nova-mir`, `nova-resolver`, `nova-typeck` and `nova-driver` change as
  §3 and §4 say.

## 9. Testing

### 9.1 Unchanged rules

- WASM tests run under Node, and fail without it under
  `NOVA_REQUIRE_NODE=1` (4.0 spec §9.1).
- Tests never touch the internet.
- Test directories have fixed names under the system's temporary
  directory.

### 9.2 The gate

`crates/nova-cli/tests/wasm_tests.rs`'s `mod gate` gains the 43 programs,
one test each:
- **with a `.stdout` file**, held to it as in 4.0:
  - async_tasks, interpolation_nary, spawned_output_survives_release,
    task_sleep_order;
  - time_conversions, time_elapsed, time_saturation, system_time_iso8601,
    system_time_now;
  - timeout_ok, timeout_elapsed, timeout_value, timeout_join_ok,
    timeout_join_elapsed, timeout_elapsed_then_join, timeout_elapsed_twice;
  - channel_clamps_buffer_below_one, channel_close_refuses_send,
    channel_close_then_drain, channel_enqueue_wraps_after_dequeue,
    channel_fifo_order, channel_full_refuses, channel_uncontended;
  - channel_recv_suspends_only_to_retry, channel_send_suspends_only_to_retry,
    channel_send_refuses_when_closed, channel_two_tasks_blocking;
  - sync_mutex_uncontended, sync_mutex_try_lock_fails_when_held,
    sync_mutex_release_is_idempotent, sync_mutex_two_tasks_serialise,
    sync_mutex_int_set_serialises, sync_mutex_stale_guard_cannot_steal,
    sync_mutex_stale_guard_cannot_write.
- **The log programs** (log_default_level, log_init_with_threshold,
  log_reconfigure_after_logging, log_init_resets_to_default) are held to
  their `.stdout` file and to their stderr lines, with each timestamp
  stripped, as `run_tests.rs` checks them natively. So are log_level_labels
  and log_stdout_output, which have no `.stdout` file.
- **The aborting programs**, checked as `run_tests.rs` checks them:
  - forged_join_handle: the join abort, with no stdout;
  - recycled_task_state: the same abort, run in stress mode;
  - task_deadlock: the deadlock report, status 134.
- **`examples/03-producer-consumer`:** `got 1` to `got 5`, then `done`.
- **The 40 programs left out** are listed in a constant, each with its
  reason. A test checks that the list and the gate together cover every
  program in `tests/runtime/`. A new program must therefore be added to
  one or the other.
- **A time limit.** Two of the channel programs hang rather than fail on a
  regression, so each gate run is killed after 60 seconds.

### 9.3 The collector

- **Stress mode** runs the gate's async programs, and the 8 programs that
  native runs under `NOVA_GC_STRESS`, with output unchanged.
- **Reclaiming.** A program sleeps 1 ms 300 times, allocating 1,000 records
  each time, under `NOVA_GC_DEBUG`. Every collection must report fewer than
  1,000 objects live, as native's `gc_reclaims_garbage` does.
- **A record reached only from a task survives.** A task holds a record
  across many turns while garbage is collected around it, and prints the
  record at the end.
- **Unit tests in `nova-codegen-wasm`** cover the yield intrinsic and
  `future_poll`, each validated and run under Node.

### 9.4 E0092 and the clocks

- E0092 for a function, a type, a qualified call and a record literal from
  a native-only module.
- Each case is accepted natively.
- `wasm_tests.rs`'s test of `args()` expects E0092 now.
- The shim's two clocks, run under Node:
  - `time_now_nanos` never decreases over a busy loop;
  - `time_now_epoch_nanos` lies within a second of `Date.now()`.

## 10. Records

- **ADR 0040, the WASM collector and executor:**
  - mark-sweep between turns, and why it departs from `13-RUNTIME.md` §8's
    reference counting;
  - the page allocator;
  - the executor in Nova, and its one built-in future;
  - `block_on` on WASM, which differs from `docs/phase-4-plan.md` §3
    decision 4.
- **Dated notes:**
  - `nova-spec/13-RUNTIME.md` §8: the collector and the executor on WASM;
  - `nova-spec/14-CODEGEN.md` §8: the intrinsic, `future_poll`, the
    exports and the new `DATA_BASE`;
  - `nova-spec/30-FRONTEND.md`: async on the event loop;
  - `docs/phase-4-plan.md`'s 4.1 entry.
- **ADR 0039** gains a note: its state words and `DATA_BASE` changed.
- **`CHANGELOG.md`**, under `[Unreleased]`.
- **`ARCHITECTURE.md`**, the `nova-std` row.

## 11. Risks

1. **A missed root frees a live object.** The roots are narrow (§6.4).
   Stress mode, and the record kept alive across turns, are the checks.
   A shadow stack is not needed while collection happens only between
   turns.
2. **Busy-waiting `block_on` freezes a page.** It is recorded in ADR 0040
   and in `block_on`'s documentation, which points to `async fn main`.
3. **The port's order drifts from native.** The gate's programs pin FIFO,
   park order and deadline order. The plan's prototype runs them before
   the spec's figures are fixed.
4. **The allocator grows the module.** Hello world's size check stays at
   30 KB gzipped. The executor and its turn export come only with a
   program that uses them.
5. **Timers under Node fire early.** `nova_turn` re-arms until the clock
   says the deadline is due (§5).

## 12. Not in 4.1

- `fs`, `io`, `net`, `http`, `process` and `crypto` on WASM; I/O waits.
- Collecting within a turn, and a shadow stack.
- `std/dom` and JS-held values (4.2).
- The 37 programs that need a native-only module, and `extern_ffi`,
  `bytes_reserved` and `nova_test`.

## 13. Decisions made while writing this spec

1. **E0092** is a new error code: E0090 and E0091 are WASM's already, and
   the message names the module and the target.
2. **The executor's leaf futures** are `async fn`s over one built-in yield
   future and a `future_poll` builtin (§4.3). The alternative was a poll
   function per leaf, emitted by the backend, which would move the
   executor's logic out of Nova.
3. **`DATA_BASE` moves to 128** for state words, rather than keeping one
   word that points at a block in the heap. Fixed words need no root, and
   the collector's own metadata must not be in the heap it collects.
4. **The page allocator is native's design**, rather than a header per
   object with an object-start bitmap. It fits WASM's 64 KiB pages, needs
   no header a string's bytes could forge, and is proven by native's tests.
5. **`async fn main` returns to JS** instead of calling `block_on`, so a
   page that awaits stays responsive. `block_on` in synchronous code
   busy-waits, by the user's decision.
6. **The deadlock report** uses a new `abort` host function, so its stderr
   matches native's exactly.
7. **The left-out list is a test**, so a new program in `tests/runtime/`
   must be added to the gate or to the list with its reason.
