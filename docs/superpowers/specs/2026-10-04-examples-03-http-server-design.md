# `examples/03-http-server` — design

**Status:** proposed, 2026-10-04, branch `examples-03-http-server`.

## 1. What this is, and what it is not

`nova-spec/60-EXAMPLES.md` §3 is a Phase 2 gate that has never existed. It has
two clauses:

- **Clause 1:** `curl http://localhost:3000/` returns `Hello from Nova!`.
- **Clause 2:** the process exits cleanly on SIGTERM.

This increment writes the example **in the Nova that exists** and meets both
clauses on all three CI operating systems.

**"Cleanly" is defined here, by the user's decision of 2026-10-04, as graceful.**
On SIGTERM the server:
1. stops accepting connections;
2. finishes requests already in flight;
3. exits 0.

Nothing in the spec defined the word before. Three things decided it:
- `docs/superpowers/specs/2026-10-04-examples-03-04-inventory.md` §3.3 laid out
  the two readings.
- The user chose the graceful one.
- The user chose the design below over two alternatives (§4.4).

**It adds three things:**
- **A runtime signal source.** A process-wide flag set by a SIGTERM or Ctrl+Break
  handler. It is opt-in, and only `Server.listen` turns it on.
- **A router in `std/http`, written in Nova.** `Server` with `new`, `get`,
  `dispatch` and `listen`, plus a `Response::json` constructor.
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
programs, and routes that ran. See inventory §3.1 and §3.2.

| §3's listing writes | The example writes | Why |
|---|---|---|
| `import std/http`, `import std/log` | nothing | std modules are glob-imported implicitly. The `/` path is `P0001`, and `import std::http` is `E0900` |
| `log.init()`, `log.info(..)` | `Log::init()`, `Log::info(..)` | `log.` is `E0001`. std/log ships `Log::` associated fns on purpose |
| `http.Server.new()` | `Server::new()` | `http.` is `E0001`, and `.` on a type is `E0001`. `Server` is new in this increment (§5) |
| `.get(path, \|_\| ...)` chain over lines starting with `.` | the same | builder chains, leading-dot continuation lines and `\|_\|` closures all work |
| `http.Response.text("Hello from Nova!")` | `Response::text(200, "Hello from Nova!")` | `text` takes 2 arguments (`E0016`). A 1-argument `text` would collide with std's 2-argument one (no overloading) |
| `http.Response.json({ "status": "ok" })` | `Response::json(status_ok())` | the map literal is `P0001`. `status_ok()` builds `Object(m)` from a `Map`. `Response::json` is new (§5) |
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

Each of these was read in the code or measured. The understand pass of
2026-10-04 recorded them, with file:line.
- **The executor and all runtime state are per-thread** (`task.rs`, `net.rs`,
  `gc.rs`; ADR 0009). A Unix signal handler runs in async-signal context, and a
  Windows console handler runs on an OS-created thread. **So a handler may only
  store into a process-global atomic.**
- **A built executable has no Rust `main`.** The C `main` shims call `nova_main()`
  and return 0. **So the handler is installed by a runtime intrinsic.**
- **Installing a flag-only handler swallows the signal.** If every program
  installed it, 05, the benchmark server and every async program would stop
  responding to SIGTERM. **So installation is opt-in.**
- **One task can wait on only one socket per poll.** A second wait aborts the
  process (`task.rs` `try_stage`). So the accept loop can notice a stop only
  through a merged deadline, which is what `std/time`'s `timeout` provides.
- **Closing a listener while a task waits in `accept` on it hangs on Windows.**
  Measured: a 10 ms `WSAENOTSOCK` retry loop, or a block forever. **So only the
  accepting task closes the listener, and only after its accept has returned.**
- **`block_on` returns only when no task is ready and none is parked**
  (`task.rs` run_to_completion), and there is no cancellation. **So every
  connection's waits must be bounded,** or exit 0 never comes.
- **The stop check must sit on the wait for a request's first bytes.** Wrapping
  `read_request` in `timeout` loses the bytes already read: a head split across a
  tick was misparsed in 3 of 3 runs. **So the server owns its read buffer.**
- **Unix `select` retries on EINTR, and a console handler cannot wake WSAPoll.**
  So a flag set by a handler is seen at the next deadline. The ticks above
  provide one.

### 4.2 Runtime: the signal source

A new module `crates/nova-runtime/src/signal.rs` holds:
- one process-wide `static SHUTDOWN: AtomicBool`;
- one `static INSTALL: Once`.

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
  is `SIG_IGN`**: a background job or `nohup` stays deaf to Ctrl+C.
- The handler does `if SHUTDOWN.swap(true) { sigaction(sig, SIG_DFL); raise(sig) }`.
  Both calls are async-signal-safe under POSIX.
- So the first TERM or INT sets the flag, and any later one takes the default
  action and kills the process. A stalled drain can always be ended.

**Windows** (`windows-sys`):
- **Dependency.** `Win32_System_Console` is declared explicitly in
  `crates/nova-runtime/Cargo.toml`. Today it is only reachable by accident,
  through another dependency's feature unification.
- **Install.** `SetConsoleCtrlHandler(handler, TRUE)`.
- **The handler.** For `CTRL_C_EVENT` or `CTRL_BREAK_EVENT` it returns FALSE if
  `SHUTDOWN.swap(true)` was already true, so the default action ends the
  process, and TRUE otherwise. Every other event returns FALSE: close, logoff
  and shutdown keep the default action.
- **Inherited Ctrl-C ignore.** An inherited ignore flag keeps Ctrl+C from
  reaching any handler, so it is respected without extra code. Ctrl+Break is
  unaffected.

**Under `nova run`** the program runs inside `nova.exe`, so the handler is
installed in that process. Once `main` returns, the CLI exits 0 (measured).

**ADR 0022** records:
- the opt-in install;
- the signal sets on each OS, the second-signal rule and the inherited-ignore
  rule;
- why the handler touches only an atomic. It never signals the executor, so ADR
  0013's rejected "second thread signals the executor" stays rejected;
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
    pub fn json(v: JsonValue) -> Response   // 200, application/json, stringify(v)
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
     - It awaits `timeout(Duration::from_millis(TICK), l.accept())`.
     - `Ok(Ok(conn))` spawns `serve(self, conn)`.
     - `Ok(Err(e))` closes the listener and returns `Err(e)`.
     - A timeout goes round again.
  4. Once stopped, it closes the listener from this same task and returns `Ok(())`.
- **`serve`**, one task per connection, private, loops over requests with a buffer
  it owns.
  - **Idle (buffer empty).** It awaits `conn.read_timeout(4096, TICK)`.
    - Zero bytes means EOF: close.
    - Bytes start a request, with a deadline 10 s after the first byte.
    - A timeout closes if `shutdown_requested()`, and otherwise keeps waiting.
  - **A request started.** It reads until `parse_request_head` and the
    `Content-Length` body are complete, each read bounded by the time left to the
    deadline.
    - Passing the deadline closes the connection.
    - A malformed request closes without a response, as 05 does.
    - Bytes beyond the request stay in the buffer for the next one.
  - **Answered.** It writes `dispatch(req)` with `write_response`.
    - A write error closes.
    - With `shutdown_requested()` set it then closes; otherwise it loops, keeping
      the connection alive.
- **`TICK` is 100 ms, and not configurable.** A probe of exactly this shape
  returned from `listen` 6-31 ms after the stop. It answered a request whose head
  straddled the stop and exited 0, in 3 of 3 runs.
- **`read_request` is unchanged.** 05 and the fixtures keep using it. `serve` is
  built from `parse_request_head`, `content_length_of` and `Limits::default()`.

**Exit sequence after the first signal:**
1. `listen` sees the flag within one tick, closes the listener and returns
   `Ok(())`.
2. `main`'s `.await.unwrap()` returns, and `main` returns.
3. `block_on` waits for the serve tasks. An idle connection closes at its next
   tick. A started request finishes within its 10 s deadline.
4. The process exits 0, from a built executable or under `nova run`.

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
the substitutions and the shutdown behaviour.

**`curl http://localhost:3000/` pays about 0.2 s on Windows.** curl tries `::1`
first, and a `0.0.0.0` bind is IPv4 only (inventory §3.2). That doesn't affect
correctness, and the listing's bind is kept. Dual-binding `[::]` is not in scope.

## 6. Error handling

- **`bind` fails** (for example, port 3000 in use): `listen` returns `Err`, and
  the example's `.unwrap()` panics with a nonzero exit. The error is the
  `IoError`, which the README names.
- **`accept` fails** (not a timeout): `listen` closes the listener and returns
  `Err`. In-flight connections still drain.
- **Per connection,** a read error, EOF, malformed request, passed deadline or
  write error closes that connection only.
- **A second signal while draining** takes the default action at once: signal
  death on Unix, `0xC000013A` on Windows.

## 7. Testing

Tests come first throughout, red then green.

### 7.1 Runtime

A new integration test binary in `crates/nova-runtime/tests/`. It needs its own
process, because installing the handler and setting the flag are process-global.
- **The decision logic is a pure function,** so it can be tested on every OS: the
  first event sets the flag, a later one takes the default action, and other
  Windows events are refused.
- **Unix:**
  - With the handler installed, one `raise(SIGTERM)` sets the flag and the process
    lives.
  - With SIGINT set to `SIG_IGN` before install, it stays ignored.
- **Windows:** calling the console handler routine directly with
  `CTRL_BREAK_EVENT` returns TRUE and sets the flag. A second call returns FALSE.
- **The STD_ONLY guard:** the resolver's existing loop test that every `STD_ONLY`
  builtin is unreachable from user code covers the new entry, and gains one
  explicit case.

### 7.2 `std/http`, without sockets

A golden fixture, `tests/runtime/http_server_dispatch.nova` with its `.stdout`,
exercises:
- `Server::dispatch` for `/` and `/health`;
- an unknown path, which gets 404;
- a non-GET method, which gets 404;
- `Response::json`'s status, `content-type` and body bytes.

### 7.3 End to end, the gate itself (`crates/nova-cli/tests/run_tests.rs`)

**`http_server_example_serves_and_exits_cleanly`:**
1. Checks port 3000 is free (bind, then drop), and fails with a clear message
   if not.
2. Spawns `nova run examples/03-http-server/src/main.nova`, with
   `CREATE_NEW_PROCESS_GROUP` on Windows. It drains stdout and stderr on threads
   for the whole run, because an undrained pipe aborts a later `println` (measured).
3. Waits until `127.0.0.1:3000` accepts a connection.
4. Asserts `GET /`: status 200 and body `Hello from Nova!`.
5. Asserts `GET /health`: status 200, `content-type: application/json` (headers
   checked by name, never as raw bytes), and body `{"status":"ok"}`.
6. **In flight:** writes half of a `GET /` head, sends the signal, writes the
   rest, and asserts the complete `Hello from Nova!` response.
7. Asserts exit code 0 within 10 s. On failure it prints the status in decimal
   and hex, the signal-delivery result, and both streams.

**`http_server_example_second_signal_forces_exit`:**
1. Stalls a request mid-head.
2. Signals once, then again.
3. Asserts the default action: signal 15 on Unix, `0xC000013A` on Windows, well
   before the 10 s request deadline.

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
- **`nova-spec/60-EXAMPLES.md` §3:** the example exists, "cleanly" now means
  graceful, and this spec carries the substitution table.
- **`nova-spec/60-EXAMPLES.md` §5:** the sentence saying nothing exists for
  `Server.get` to be built on is now false.
- **`nova-spec/60-EXAMPLES.md` §9 and §10:** their population notes, since a
  second example has a README.
- **`nova-spec/20-STDLIB.md` §6:**
  - `Server`, `get`, `dispatch` and `listen` ship, with the deviations in §3.
  - `Response::json` ships as a constructor and takes the name the spec gives the
    client decoder.
  - What is not shipped.
  - The internal signal flag.
- **`nova-spec/13-RUNTIME.md`:** the signal source as a new runtime component, and
  graceful drain as distinct from cancellation.
- **`nova-spec/00-MASTER-SPEC.md`:** the paragraph saying neither 03 nor 04 exists
  is now false for 03.

**ADRs:**
- **New ADR 0022.**
- **ADR 0013 and ADR 0009:** forward pointers to ADR 0022.
- **ADR 0019:** its "still absent: the router" consequence.

**Earlier design records:**
- `docs/superpowers/specs/2026-10-04-examples-03-04-inventory.md` §2, §3.3, §6
  and §7.
- `docs/superpowers/specs/2026-09-01-std-http-request-parsing-design.md` §1, §3
  and §10.
- `docs/superpowers/specs/2026-08-23-std-net-listener-design.md` §7 item 3.

**Sources and other files, edited in place:**
- the `std/http/lib.nova` header comment, which says no router ships;
- its "never opens a socket" comments;
- the `std/net/lib.nova:278-282` "no graceful-shutdown path" comment;
- `examples/05-json-api/README.md`'s "ships no router type";
- `docs/benchmarks/bun-server.js:4-6`;
- `CHANGELOG.md` `[Unreleased]`, plus a forward marker on `[0.2.0-alpha.2]`'s
  "with no router".

The implementation plan's last task runs a set-difference sweep: every tracked
file naming `Server`, `router`, `Handler`, `SIGTERM`, `graceful` or
`03-http-server`, minus the files this branch touched. That catches repeats this
list misses.

## 9. Risks

- **Unix SIGTERM handling has never run on a Unix host.** The spike measured
  delivery, not a handler. The first measurement of the Unix handler is this
  branch's CI. The runtime test and the end-to-end test are both blocking on
  ubuntu and macOS.
- **An idle connection wakes every 100 ms.** That costs CPU per idle keep-alive
  connection. It is accepted for v1 and recorded in the README.
- **A synchronous handler that loops forever blocks the single thread,** tick
  included. A second signal still ends the process.
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
- **No 400 response for a malformed request,** and no dual-stack `[::]` bind.
- **No executor wake.** Shutdown latency is up to one tick.
- **The other examples.** 04 and the `60-EXAMPLES.md` §10 `tests/` folder and
  `nova test` CI job are untouched; Nova code cannot send a signal, so a
  `nova test` could not test clause 2 anyway.

## 11. Success criteria

1. `examples/03-http-server` serves both routes. `curl http://localhost:3000/`
   returns `Hello from Nova!` on this host.
2. On SIGTERM (Unix) or Ctrl+Break (Windows), the server stops accepting,
   answers the request in flight, and exits 0. A second signal ends it at once.
3. The tests in §7 exist, failed before the code they cover, and pass on ubuntu,
   windows and macOS in blocking CI steps.
4. No existing test changes meaning. Programs that never call `Server.listen` keep
   their signal behaviour.
5. Every record in §8 is amended, and the set-difference sweep finds nothing left
   stale.
