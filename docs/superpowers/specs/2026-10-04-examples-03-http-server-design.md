# `examples/03-http-server` — design

**Status:** proposed, 2026-10-04, branch `examples-03-http-server`.

## 1. What this is, and what it is not

`nova-spec/60-EXAMPLES.md` §3 is a Phase 2 gate that has never existed. It has
two clauses:

- **Clause 1:** `curl http://localhost:3000/` returns `Hello from Nova!`.
- **Clause 2:** the process exits cleanly on SIGTERM.

This increment writes the example **in the Nova that exists**. It covers each
clause as follows:

- **Clause 1's routes** are verified on all three CI operating systems, over
  `127.0.0.1:3000`. The literal `curl http://localhost:3000/` form is checked on
  this development host.
- **Clause 2** is verified on all three CI operating systems.

**"Cleanly" is defined here, by the user's decision of 2026-10-04, as graceful.**
On the termination signal the server:
1. stops accepting connections;
2. finishes requests already in flight;
3. exits 0.

Nothing in the spec defined the word before. Three things decided it:
- `docs/superpowers/specs/2026-10-04-examples-03-04-inventory.md` §3.3 laid out
  the two readings.
- The user chose the graceful one.
- The user chose the design below over two alternatives (§4.4).

**It adds three things:**
- **A runtime signal source.** A process-wide flag, set by the first SIGTERM or
  SIGINT on Unix, or CTRL_BREAK or CTRL_C on Windows, unless that input was
  inherited as ignored (§4.2). It is opt-in, and only `Server.listen` turns it on.
- **A router in `std/http`, written in Nova.** `Server` with `new`, `get`,
  `dispatch` and `listen`, plus a `Response::json` constructor. `Server`
  answers pipelined requests (§4.3).
- **The example itself.** Its README, its tests, and dated notes in every record
  this makes stale.

**What it does not do:**
- **No executor change.** `task.rs`, `poll.rs` and `net.rs` keep their behaviour.
- **No public signal API.** User code cannot read the flag.
- **No language feature.** Every construct §3's listing reaches for and the
  language lacks keeps the route the inventory measured.
- **Nothing from 04 or std/process.** No `args()`, no `exit()`.

## 2. Why the spec's listing is not the deliverable

§3's listing is kept **as the aspiration it was**, the precedent
`examples/05-json-api` set under §5. The deliverable is a working example that
serves the same two routes, plus this record of how it differs and why. A dated
amendment under §3 points here. The listing itself is not edited.

The listing cannot compile as written. Its first line, `import std/http`, is
`P0001` (inventory §2). §3 below gives every substitution.

## 3. Every substitution, and the evidence for it

Every row was measured in the inventory: `nova check` codes from one-construct
programs, and routes that ran. See inventory §3.1 and §3.2. The listing's
`async fn main() {` and closing `}` are unchanged and have no row.

| §3's listing writes | The example writes | Why |
|---|---|---|
| `import std/http`, `import std/log` | nothing | std modules are glob-imported implicitly. The `/` path is `P0001`, and `import std::http` is `E0900` |
| `log.init()`, `log.info(..)` | `Log::init()`, `Log::info(..)` | `log.` is `E0001`. std/log ships `Log::` associated fns on purpose |
| `http.Server.new()` | `Server::new()` | `http.` is `E0001`, and `.` on a type is `E0001`. `Server` is new in this increment (§4.3) |
| `.get(path, \|_\| ...)` chain over lines starting with `.` | the same | builder chains, leading-dot continuation lines and `\|_\|` closures all work |
| `http.Response.text("Hello from Nova!")` | `Response::text(200, "Hello from Nova!")` | `text` takes 2 arguments (`E0016`). A 1-argument `text` would collide with std's 2-argument one (no overloading: `E0074`) |
| `http.Response.json({ "status": "ok" })` | `Response::json(status_ok())` | the map literal is `P0001`. `status_ok()` builds `Object(m)` from a `Map`. `Response::json` is new (§4.3) |
| `app.listen("0.0.0.0:3000").await.unwrap()` | the same | an `async fn` method returning `Result`, awaited and unwrapped, works |

**Spec signatures that cannot ship as written** (`20-STDLIB.md` §6):
- `get` returns `-> Self` there. `Self` as a type is `E0001` in any impl block,
  so `get` returns `-> Server`.
- The `Handler` alias is `P0001` (async) or `E0900` (sync). Handlers are typed
  `fn(Request) -> Response` inline.
- `post`, `put`, `delete`, `route`, `use_middleware` and path params are not
  shipped (§10).

## 4. Architecture

### 4.1 Constraints the design is built around

Each of these was read in the code or measured, except where a clause is marked
as taken from POSIX or Win32 documentation, or inferred. The understand pass of
2026-10-04 recorded them, with file:line for what was read.
- **The executor and all runtime state are per-thread** (`task.rs`, `net.rs`,
  `gc.rs`; ADR 0009). A Unix signal handler runs in async-signal context (POSIX).
  A Windows console handler runs on an OS-created thread (Win32; not measured
  here). **So a handler may only store into a process-global atomic.**
- **A built executable has no Rust `main`.** The C `main` shims call `nova_main()`
  and return 0. **So the handler is installed by a runtime intrinsic.**
- **Installing a flag-only handler swallows the signal.** On Windows this was
  measured: a handler returning TRUE survived CTRL_BREAK. The Unix half, and the
  consequence that 05, the benchmark server and every async program would stop
  responding to SIGTERM if every program installed it, are inferred. **So
  installation is opt-in.**
- **One task can wait on only one socket per poll.** A second wait aborts the
  process (`task.rs` `try_stage`). So the accept loop can notice a stop only
  through a merged deadline, which is what `std/time`'s `timeout` provides.
- **Closing a listener while a task waits in `accept` on it hangs on Windows.**
  Measured: a 10 ms `WSAENOTSOCK` retry loop, or a block forever. **So only the
  accepting task closes the listener, and only after its accept has returned.**
- **`block_on` returns only when no task is ready and none is parked**
  (`task.rs` run_to_completion), and there is no cancellation. **So every
  connection's waits must be bounded, reads and writes alike,** or exit 0 never
  comes.
- **The stop check must sit on the wait for a request's first bytes.** Wrapping
  `read_request` in `timeout` loses the bytes already read: a head split across a
  tick was misparsed in 3 of 3 runs. **So the server owns its read buffer.**
- **Unix `select` retries on EINTR, and a console handler cannot wake WSAPoll.**
  So a flag set by a handler is seen at the next deadline. The ticks above
  provide one.

### 4.2 Runtime: the signal source

A new module `crates/nova-runtime/src/signal.rs` holds:
- one process-wide `static SHUTDOWN: AtomicBool`;
- one `static INSTALL: Once`;
- the decision logic as a pure function, so it can be unit-tested on every OS.

**The builtin.** One new `STD_ONLY` builtin, `shutdown_requested() -> Bool`.
- Its first call installs the handler, through the `Once`.
- Every call returns `SHUTDOWN`.
- It is registered at the twelve sites every runtime builtin uses:
  - the resolver's variant, name and `STD_ONLY` entry (77 becomes 78);
  - the typechecker's hint arm, signature and test table;
  - MIR's `RtFunc` variant, `symbol()`, `signature()` and lowering;
  - the runtime's `extern "C"` function and its `symbols()` entry.
- `STD_ONLY` keeps the name out of user programs.

**Unix** (libc, already a dependency):
- `sigaction` installs one handler for SIGTERM.
- It installs the same handler for SIGINT, **unless SIGINT's current disposition
  is `SIG_IGN`**. A job backgrounded by a non-interactive shell (`cmd &` in a
  script), whose SIGINT is ignored, stays deaf to Ctrl+C.
- The handler does `if SHUTDOWN.swap(true) { sigaction(sig, SIG_DFL); raise(sig) }`.
  Both calls are async-signal-safe under POSIX. The raised signal is blocked
  while its own handler runs, so it is delivered on return, with the default
  action.
- So the first TERM or INT sets the flag, and any later one takes the default
  action and kills the process. A stalled drain can always be ended.

**Windows** (`windows-sys`):
- **Dependency.** `Win32_System_Console` is declared explicitly in
  `crates/nova-runtime/Cargo.toml`. Today it is only reachable by accident,
  through another dependency's feature unification.
- **Install.** `SetConsoleCtrlHandler(handler, TRUE)`.
- **The handler.** For `CTRL_C_EVENT` or `CTRL_BREAK_EVENT` it returns FALSE if
  `SHUTDOWN.swap(true)` was already true, so the default action ends the
  process with `0xC000013A`, and TRUE otherwise. Every other event returns
  FALSE: close, logoff and shutdown keep the default action.
- **Inherited Ctrl-C ignore.** An inherited ignore flag keeps CTRL_C from
  reaching any handler, so it is respected without extra code. CTRL_BREAK is
  unaffected.

**Under `nova run`** the program runs inside `nova.exe`, so the handler is
installed in that process. `nova-cli` spawns no threads. Once `main` returns,
the CLI exits 0 (measured).

**The install is permanent for the process.** The handler is never removed and
`SHUTDOWN` is never cleared. Two cases matter:
- **`listen` returns `Err`** (bind or accept failure) with the flag still clear.
  A program that handles that `Err` and keeps running swallows its first later
  signal, and only a second one ends it.
- **`listen` returns `Ok`.** That happens only after the flag is set, so the next
  signal already takes the default action. A later `listen` call returns `Ok` at
  once, which matches a process-wide stop.

**ADR 0022** records:
- the opt-in install, and that it is permanent;
- the signal sets on each OS, the second-signal rule and the inherited-ignore
  rule;
- why the handler touches only an atomic. It never signals the executor, so the
  alternative ADR 0013 rejected, "A poller thread signalling the executor",
  stays rejected;
- the two alternatives in §4.4, and why they lost.

### 4.3 `std/http`: the `Server`

```nova
pub record Server { routes: Vec<Route> }
record Route { path: String, handler: fn(Request) -> Response }

impl Server {
    pub fn new() -> Server
    pub fn get(self, path: String, handler: fn(Request) -> Response) -> Server
    pub fn dispatch(self, req: Request) -> Response
    pub async fn listen(self, addr: String) -> Result<(), IoError>
}
impl Response {
    pub fn json(v: JsonValue) -> Response   // 200, content-type application/json,
                                           // content-length, body stringify(v)
}
```

- **`get`** appends to the route table and returns the new `Server`. It takes
  `self`, because a `mut self` call on the temporary `Server::new()` returns is
  `E0060`.
- **`dispatch`.** On GET, an exact `req.path` match calls that route's handler.
  The handler is bound to a local first, because calling a fn-typed field directly
  is `E0014`. Anything else gets `Response::not_found()`.
- **`listen`:**
  1. Calls `shutdown_requested()` once **before** `bind`. The handler is in place
     before the port opens, so a client that can connect can also stop the server
     gracefully.
  2. `bind(addr)`. An error returns `Err`.
  3. Loops, checking `shutdown_requested()` **every** iteration. Checking only on
     the timeout arm starves the check under steady traffic.
     - It awaits `timeout(Duration::from_millis(TICK), l.accept())`. The result
       is read with nested `match` expressions: `Ok(Ok(..))` as one pattern is
       `E0900`.
     - A connection spawns `serve(self, conn)`.
     - An accept error closes the listener and returns `Err(e)`.
     - A timeout goes round again.
  4. Once stopped, it closes the listener from this same task and returns `Ok(())`.

  **The return contract.** `listen` returns as soon as accepting stops; it does
  not wait for connections. Their serve tasks keep running alongside any code
  after `.await`, and the process exits only once `block_on`'s implicit join has
  finished them (ADR 0009). The `listen` doc comment and the `20-STDLIB.md` note
  both say so.
- **`serve`**, one task per connection, private, keeps one buffer for the
  connection's whole life.
  - **A request is in flight once any of its bytes are in the buffer.** Its
    deadline is 10 s after serve takes it up: when its first byte arrives, or,
    for a request already buffered, when the response before it has been written.
  - **Idle (buffer empty).** It awaits `conn.read_timeout(4096, TICK)`.
    - Zero bytes means EOF: close.
    - Bytes take up a new request.
    - A timeout closes if `shutdown_requested()`, and otherwise keeps waiting.
  - **A request taken up.** It parses what is buffered **before** reading more.
    - The head comes from `parse_request_head`.
    - The body starts at `parse_offsets`' last entry.
    - Its length is `content_length_of`, then checked against
      `max_body_bytes` as `read_request` does.
    - While incomplete, it reads with `read_timeout(4096, ms left to the
      deadline)`.
    - Passing the deadline, EOF, a malformed head or an oversized body closes
      the connection without a response, as 05 does.
  - **Answering.** It writes `dispatch(req)` through
    `timeout(ms left to the deadline, write_response(conn, resp))`, so the write
    is bounded too. Expiry or a write error closes.
  - **After an answer** the request's bytes leave the buffer.
    - **Bytes remain:** the next request is taken up and answered, flag or not.
      Pipelined requests are in flight.
    - **Empty and the flag set:** close.
    - **Empty and the flag clear:** go idle, keeping the connection alive.

    [Amended 2026-10-05, the branch's final review: "flag or not" left the
    drain unbounded. A client that keeps pipelining never lets the buffer
    empty, and each new head was read to completion, so the connection never
    closed. A request taken up after the stop now gets no further reads: it is
    answered only if it has already arrived whole, and otherwise the
    connection closes. Requests taken up before the stop are unchanged.
    Pinned by `http_server_example_bounds_the_drain_for_a_pipelining_client`.]
- **`TICK` is 100 ms, and not configurable.**
  - **The probe behind it** had the same tick-polled accept loop and first-byte
    tick, with a sibling task setting the stop. Its `listen` returned within one
    tick of the stop: 6-31 ms after a stop at 400 ms, and 41-93 ms after one at
    450-470 ms, depending on where the stop fell within the tick. It answered a
    request whose head straddled the stop, and exited 0, in 3 of 3 runs.
  - **Not covered by that probe:**
    - an OS signal source, and the install before bind;
    - the 10 s deadline (the probe gave up after 50 empty reads);
    - body reads and pipelined leftovers (its buffer lasted one request);
    - bounded writes;
    - closing the listener on an accept error;
    - 404 for non-GET (it returned 405).

  A real signal lands anywhere within a tick, so expect up to 100 ms.
- **`read_request` is unchanged.** 05 and the fixtures keep using it, and its
  documented limits, including the pipelining deadlock, still describe it.

**Exit sequence after the first signal:**
1. `listen` sees the flag within one tick, closes the listener and returns
   `Ok(())`.
2. `main`'s `.await.unwrap()` returns, and `main` returns.
3. `block_on` waits for the serve tasks:
   - an idle connection closes at its next tick;
   - every request already in flight, pipelined ones included, is read and
     answered within its own 10 s deadline, or its connection is closed at the
     deadline.
4. The process exits 0. Under `nova run` this is tested (§7.3). A
   Cranelift-built executable exits 0 by construction, because `emit_c_main`
   returns 0 once `nova_main` returns. That path was measured only on Windows,
   through an FFI-handler stand-in (3 of 3, rc=0). No test builds 03, and an
   LLVM-built executable has never been run.

### 4.4 Alternatives considered

- **B: a watcher task on a wake socket.** A self-pipe on Unix or a loopback pair
  on Windows. On wake, the watcher connects to the server's own port to unblock
  `accept`.
  - The only gain is lower accept latency.
  - If `listen` ends any other way, the watcher's untimed wait keeps the process
    alive, turning an error exit into a hang.
  - Reads still need the ticks.
- **C: a wake built into the executor.** A runtime-owned socket in the poll set,
  and `accept` returning `Interrupted`.
  - It changes the scheduler every program runs on.
  - It cannot tell an idle keep-alive read from a mid-request one, so std/http
    still needs the ticks.
  - It carries the largest ADR and record cost.

The user chose A on 2026-10-04.

## 5. The example

`examples/03-http-server/src/main.nova` sits beside `examples/03-producer-consumer`.
That is the user's decision; nothing is renumbered.

```nova
fn status_ok() -> JsonValue {
    let mut m: Map<String, JsonValue> = Map::new()
    m.insert("status", String("ok"))
    Object(m)
}

async fn main() {
    Log::init()

    let app = Server::new()
        .get("/", |_| Response::text(200, "Hello from Nova!"))
        .get("/health", |_| Response::json(status_ok()))

    Log::info("listening on :3000")
    app.listen("0.0.0.0:3000").await.unwrap()
}
```

`examples/03-http-server/README.md` follows `60-EXAMPLES.md` §9's template: what
it demonstrates, how to run it, the expected output, and notes. The notes cover
the substitutions, the shutdown behaviour, and three facts a reader needs:
- **The `listening on :3000` line is printed before `listen` runs.** It comes
  before the port is bound (about 2.2 ms earlier under `nova run`, measured) and
  before the handler is installed. **Readiness, for a request or a graceful
  signal, means the port accepts a connection.**
- **An idle connection costs a wake every 100 ms.**
- **A bind failure prints only the generic unwrap message** (§6).

**`curl http://localhost:3000/` pays about 0.2 s on Windows.** curl tries `::1`
first, and a `0.0.0.0` bind is IPv4 only (inventory §3.2). That doesn't affect
correctness, and the listing's bind is kept. Dual-binding `[::]` is not in scope.

## 6. Error handling

- **`bind` fails** (for example, port 3000 in use): `listen` returns `Err`, and
  the example's `.unwrap()` panics with a nonzero exit.
  - stderr shows only `nova: panic: called `unwrap` on an `Err` value`, because
    `unwrap` discards the `IoError`.
  - The README, not the terminal, tells the user the cause is usually the port.
- **`accept` fails** (not a timeout): `listen` closes the listener and returns
  `Err`. It does not touch the serve tasks, and it does not set the flag.
  - **In the example,** `.unwrap()` then panics. A Nova panic aborts the process
    at once (`std::process::abort`: `0xC0000409` on Windows, SIGABRT on Unix).
    Requests in flight are cut off, with the same kind of nonzero exit as a bind
    failure.
  - **A caller that handles the `Err`** lets started requests finish. But the
    flag is unset, so connections stay open and keep being served. Idle
    connections have no timeout outside shutdown (§10), so the process exits only
    when clients close them or a signal arrives.
- **Per connection,** a read error, EOF, malformed request, oversized body,
  passed deadline or write error closes that connection only.
- **A second signal while draining** takes the default action at once: signal
  death on Unix, `0xC000013A` on Windows.

## 7. Testing

Tests come first throughout, red then green.

### 7.1 Runtime

**The decision logic.** A unit test in `signal.rs` runs on every OS and touches
no process state. It is a table over (event, flag already set):
- CTRL_C and CTRL_BREAK give TRUE with the flag clear, and FALSE with it set.
- CLOSE, LOGOFF and SHUTDOWN always give FALSE.
- On Unix the decision ignores the signal number: first sets, any later one
  takes the default action.

**Process-global behaviour.** Installing the handler and setting the flag are
permanent for a process. libtest runs one binary's tests on parallel threads in
one process. **So each scenario gets its own file under `crates/nova-runtime/tests/`,**
which is its own binary and process. Each file holds one `#[test]` that runs its
steps in order and fires the `Once` once:
- **`signal_sigterm.rs` (Unix):**
  1. Set SIGINT to `SIG_IGN`.
  2. Install, through the first flag read.
  3. Assert SIGINT is still `SIG_IGN`.
  4. `raise(SIGTERM)`.
  5. Assert the flag is set and the process lives.
- **`signal_sigint.rs` (Unix):**
  1. With SIGINT at its default, install.
  2. `raise(SIGINT)`.
  3. Assert the flag is set and the process lives.
- **`signal_console.rs` (Windows):**
  1. Install.
  2. Call the console handler routine with `CTRL_C_EVENT`: expect TRUE, and the
     flag set.
  3. Call it with `CTRL_BREAK_EVENT`: expect FALSE.
  4. Call it with `CTRL_CLOSE_EVENT`: expect FALSE.

**The STD_ONLY guards.** Two existing loops cover the new entry with no new code:
- the typechecker's `no_std_only_builtin_is_callable_from_user_code`, which
  makes it unreachable from user code;
- the resolver's `no_std_only_builtin_is_a_reserved_word`, which keeps its name
  free for user definitions.

### 7.2 `std/http`, without sockets

A golden fixture, `tests/runtime/http_server_dispatch.nova` with its `.stdout`,
is registered as `http_server_dispatch_run` in `crates/nova-cli/tests/run_tests.rs`,
following `http_serialise_run`. It exercises:
- `Server::dispatch` for `/` and `/health`;
- an unknown path, which gets 404;
- a non-GET method, which gets 404;
- `Response::json`'s status, `content-type`, `content-length` and body bytes.

### 7.3 End to end, the gate itself (`crates/nova-cli/tests/run_tests.rs`)

**Port 3000.** Both tests use the example on its fixed port 3000, so they take
one `static PORT_3000: Mutex<()>`, tolerating poison, and never overlap. Each
first checks the port is free:
- binding `0.0.0.0:3000` and dropping it must succeed;
- a connect to `127.0.0.1:3000` must be refused.

Otherwise it fails with a clear message.

**Draining.** Each test spawns `nova run examples/03-http-server/src/main.nova`,
with `CREATE_NEW_PROCESS_GROUP` on Windows. It drains stdout and stderr on
threads for the whole run:
- a closed stdout pipe aborts a later `println` (measured);
- an undrained full pipe can block.

**`http_server_example_serves_and_exits_cleanly`:**
1. **Ready.** Waits until `127.0.0.1:3000` accepts a connection.
2. **Routes, on connection K.**
   - `GET /`: status 200 and body `Hello from Nova!`.
   - `GET /health`: status 200, `content-type: application/json`,
     `content-length` (headers checked by name, never as raw bytes), and body
     `{"status":"ok"}`.

   K then stays open and idle.
3. **Pipelining.** On connection P, one write carries `GET /` and
   `GET /health`. Both responses arrive, in order.
4. **In flight.** This step proves a request that straddles the stop is
   finished:
   - Open connection F and write half of a `GET /` head.
   - Pause 300 ms, at least one tick. The server has then accepted F and read the
     half head before the flag can exist.
   - Send the signal.
   - Wait until a fresh connect to `127.0.0.1:3000` is refused, which proves
     `listen` saw the flag. Any connect timeout must exceed about 2.1 s, because a
     refusal takes about 2 s on Windows (measured).
   - Only then write the rest of the head.
   - Assert the complete `Hello from Nova!` response, and that the server then
     closes F.
5. **Idle close.** The server has closed K: a read returns 0.
6. **Exit.** Exit code 0 within 10 s of the signal. On failure it prints the
   status in decimal and hex, the signal-delivery result, and both streams.

**`http_server_example_second_signal_forces_exit`:**
1. **Ready.** Waits until the port accepts.
2. **Stall.** Opens a connection, writes half of a head, and pauses 300 ms.
3. **First signal.** Signals once, then waits until a fresh connect is refused.
   Unix merges a second SIGTERM sent while the first is still pending into one
   delivery, so this wait proves the first was handled.
4. **Second signal.** Signals again.
5. **Assert** the default action within 2 s, well before the stalled request's
   10 s deadline: signal 15 on Unix, `0xC000013A` on Windows.

**How the signal is delivered.** The helpers are the ones the spike measured,
hand-declared so `Cargo.lock` doesn't change:
- Unix: `kill(pid, 15)`.
- Windows: `GenerateConsoleCtrlEvent(CTRL_BREAK_EVENT, pid)` to the child's own
  process group, never group 0.

**The spike (2026-10-04, draft PR #95, closed unmerged)** ran that delivery
against `docs/benchmarks/server.nova`, which has no handler, on all three CI
runners. It asserted the default disposition, and passed on all three:
- signal 15 on ubuntu-latest and macos-latest;
- `0xC000013A` on windows-latest.

So delivery works on every leg. With the handler in place, the same delivery
must now produce exit 0.

### 7.4 Unchanged

05's `json_api_example_serves_its_routes` and every existing test must pass
unchanged. Programs that never call `Server.listen` install no handler, and
nothing in their behaviour changes.

## 8. Records to amend

Dated notes, following each file's own amendment style. Std sources and the 05
README are edited in place.

**Spec:**
- **`nova-spec/60-EXAMPLES.md`:**
  - **§3:** the example exists, "cleanly" now means graceful, and this spec
    carries the substitution table. The note answers §3's 2026-09-01 drift note
    and records that slot 03 now holds two entries.
  - **§5:** the sentence saying nothing exists for `Server.get` to be built on
    is now false. The two `20-STDLIB.md` line citations at §5's 2026-09-03
    amendment (`:548` and `:504`) were already stale and are corrected.
  - **§9:** its README-population note.
  - **§10:** a new note that 03's tests live in `run_tests.rs`, not in
    `examples/03-http-server/tests`.
- **`nova-spec/20-STDLIB.md` §6:**
  - `Server`, `get`, `dispatch` and `listen` ship, with the deviations in §3.
  - The `listen` return contract and the permanent install.
  - `Response::json` ships as a constructor and takes the name the spec gives the
    client decoder.
  - `Server` answers pipelined requests, though `read_request` still does not.
    This narrows `:448`.
  - What is not shipped.
  - The internal signal flag.
- **`nova-spec/13-RUNTIME.md`:** the signal source as a new runtime component, and
  graceful drain as distinct from cancellation.
- **`nova-spec/00-MASTER-SPEC.md`:** the `:527-529` paragraph saying neither 03 nor
  04 exists, and three paragraphs in §9 "Recorded Drift Against `examples/`":
  - the 2026-09-01 tree-drift paragraph;
  - the 2026-09-10 README-population amendment;
  - the 2026-09-10 tree-drift amendment. Section 2's `03-http-server/` entry is no
    longer ahead of the disk, but slot 03 holds two entries.

**ADRs:**
- **New ADR 0022.**
- **ADR 0013 and ADR 0009:** forward pointers to ADR 0022.
- **ADR 0019:**
  - its "still absent: the router" consequence;
  - its pipelining sentence (`:437`), narrowed to `read_request`.

**Earlier design records:**
- `docs/superpowers/specs/2026-10-04-examples-03-04-inventory.md` §2, §3.3, §6 and
  §7. The note also says this branch's `20-STDLIB.md` §6 note shifts every later
  line, so the inventory's §1 baseline applies.
- `docs/superpowers/specs/2026-09-01-std-http-request-parsing-design.md`:
  - §1, §3 and §10;
  - the pipelining entries at `:43` and `:401`, narrowed to `read_request`.
- `docs/superpowers/specs/2026-08-23-std-net-listener-design.md` §7 item 3.

**Sources and other files, edited in place:**
- the `std/http/lib.nova` header comment, which says no router ships;
- its "never opens a socket" comments;
- its pipelining note at `:37-40`, narrowed to `read_request`;
- the `listen` doc comment (return contract, permanent install);
- the `std/net/lib.nova:278-282` "no graceful-shutdown path" comment;
- `examples/05-json-api/README.md`'s "ships no router type";
- `docs/benchmarks/bun-server.js:4-6`;
- `CHANGELOG.md` `[Unreleased]`, plus a forward marker on `[0.2.0-alpha.2]`'s
  "with no router".

**The sweep.** The implementation plan's last task runs it in two parts:
- **A set difference.** Every tracked file naming any of these tokens, minus the
  files this branch touched: `Server`, `router`, `Handler`, `SIGTERM`, `graceful`,
  `pipelin`, `03-http-server`, `03-producer-consumer`.
- **A re-read of every touched file's existing dated notes,** with wrapped lines
  flattened first. The set difference cannot see stale paragraphs inside files
  the branch already edits.

## 9. Risks

- **Unix signal handling has never run on a Unix host.** The spike measured
  delivery, not a handler. The first measurement of the Unix handler is this
  branch's CI. The runtime tests and the end-to-end tests are blocking on ubuntu
  and macOS.
- **No test builds 03.** The built-exe path holds by construction, and was
  measured only on Windows through an FFI-handler stand-in. The LLVM backend has
  never been run on this host.
- **An idle connection wakes every 100 ms.** That costs CPU per idle keep-alive
  connection. It is accepted for v1 and recorded in the README.
- **A synchronous handler that loops forever blocks the single thread,** tick
  included. A second signal still ends the process.
- **A program that recovers from `listen`'s `Err` swallows its next signal**
  (§4.2), because the install is permanent.
- **A fixed port 3000 can collide on a developer machine.** The test fails with a
  clear message rather than hanging.
- **Std is compiled into `nova.exe`.** The std/http change needs a rebuild before
  any measurement. The trap where `*_build_standalone` tests link a stale debug
  runtime library applies.

## 10. What is not covered

- **No public signal API, no `std/process`, no `args()` or `exit()`.** That is
  step 2 of the inventory's order.
- **No other `20-STDLIB.md` §6 routes or features:**
  - `post`, `put`, `delete` and `route`;
  - middleware;
  - path params;
  - async handler fields. Their `fn(Request) -> Future<Response>` spelling works
    (inventory §3.1) but is not shipped.
- **No idle keep-alive timeout outside shutdown,** and no `Connection: close`
  handling.
- **No 400 response for a malformed request, no retry of a failed accept,** and
  no dual-stack `[::]` bind.
- **No executor wake.** Shutdown latency is up to one tick.
- **The other examples.** 04 and the `60-EXAMPLES.md` §10 `tests/` folder and
  `nova test` CI job are untouched; Nova code cannot send a signal, so a
  `nova test` could not test clause 2 anyway.

## 11. Success criteria

1. **Clause 1.**
   - On all three CI operating systems, `127.0.0.1:3000` serves both routes,
     including pipelined requests.
   - On this host, `curl http://localhost:3000/` returns `Hello from Nova!`.
2. **Clause 2.** On SIGTERM (Unix) or Ctrl+Break (Windows), the server:
   - stops accepting;
   - answers the request in flight;
   - closes idle connections;
   - exits 0.

   A second signal ends it at once.
3. **The other two inputs.** The runtime also accepts SIGINT and Ctrl+C, unless
   inherited as ignored. §7.1 pins this.
4. **The tests.** The tests in §7 exist, failed before the code they cover, and
   pass on ubuntu, windows and macOS in blocking CI steps.
5. **No regressions.** No existing test changes meaning. Programs that never call
   `Server.listen` keep their signal behaviour.
6. **The records.** Every record in §8 is amended, and the sweep finds nothing
   left stale.
