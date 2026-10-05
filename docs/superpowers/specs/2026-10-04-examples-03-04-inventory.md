# Phase 2 gate examples 03 and 04: what blocks them, measured

**Date:** 2026-10-04. **Measured against:** `main` at `3f4fdfb`, with the release
`nova.exe` current with it (a no-op `cargo build --release -p nova-cli -p
nova-runtime` confirmed it). **Host:** this development host, Windows 11.

This is a record, not a design. It inventories what stands between today's Nova
and the two Phase 2 gate examples that do not exist yet:
`nova-spec/60-EXAMPLES.md` §3, `03-http-server`, and §4, `04-todo-cli`. Nothing
in the repository changed to produce it. No probe program is kept.

**Recorded 2026-10-04 (branch `examples-03-http-server`): parts of this record
are now history.**
- §2 and §3.3: the runtime now observes SIGTERM and SIGINT (Ctrl+Break and
  Ctrl+C on Windows), through an opt-in flag
  (`docs/adr/0022-process-shutdown-signals.md`), and `std/http`'s `Server`
  turns it into a graceful exit.
- §6: decision (i) is made, "exits cleanly" means graceful, and so are (ii),
  beside `03-producer-consumer`, and (iii), the listing kept as aspiration.
  Step 1 of the recommended order is done: `examples/03-http-server` exists.
- §3.2 and §7: the router compiles inside `std/http`, so "whether it compiles
  inside std is inferred" is settled.
- Line numbers this record cites in `nova-spec/20-STDLIB.md` hold for this
  record's own branch, as §1 says. The branch named above added a note under
  that file's §6, which moves every later line again, so read those citations
  by the text they quote. The same holds for §1's `nova-spec/60-EXAMPLES.md`
  line range for §4's listing: that branch also added a note under §3, which
  moves that listing again.

**Recorded 2026-10-05 (branch `examples-04-todo-cli`): the argument findings in
§2, §4.1 and §4.3 are history too.** `std/process` gives `args()` and `exit(code)`, `nova run
[FILE] -- ARGS` passes arguments through, and a built executable's runtime reads
the OS argv (`docs/adr/0023-program-arguments.md`). §6's steps 2 and 3 are done:
`examples/04-todo-cli` exists, and `02-fibonacci`'s gate is met. The five std
additions §6 step 3 lists were written around, not added.

Bare section numbers (§2, §3.1, ...) are this record's own. The spec's sections
are always written with the file name, as in `60-EXAMPLES.md` §3.

---

## 1. Method, and how far to trust each column

- **The verbatim listings first.** Both listings were extracted byte for byte from
  `60-EXAMPLES.md` as it stood at `3f4fdfb`: lines 65-77 for its §3 and 97-155 for
  its §4. The note this branch adds under `60-EXAMPLES.md` §3 moves that file's §4
  listing to 107-165. Both were run through `nova check`, and both fail on their
  first line (§2).
- **Line numbers in `20-STDLIB.md` are cited as they stand on this branch.** The
  note it adds under that file's §6 amendment moved every line after 465 down by
  8. So 495-496, 509, 519, 548 and 1715-1716 here were 487-488, 501, 511, 540 and
  1707-1708 at `3f4fdfb`; 232-234 did not move.
- **Then a fan-out of probes.** Nine probe groups, one per family of constructs,
  each wrote and ran its own programs:
  - imports;
  - the HTTP builder;
  - the SIGTERM lifecycle;
  - declarations and JSON;
  - the process surface;
  - Option and String;
  - iteration and mutation;
  - the filesystem cycle;
  - the harness and naming.

  Each group had to try a route for every gap and *run* it. Probes lived outside
  the repository.
- **Every finding was then challenged.** One adversarial verifier per group tried
  to refute each finding.
- **A completeness critic mapped every non-blank line of both listings to a
  finding.** It found five uncovered constructs. A gap round probed those five,
  and its own verifier checked the results.
- **Denominators.**
  - 149 primary findings got 149 verdicts: 106 upheld, 43 partly upheld, 0 refuted.
  - The verifiers added 78 more findings of their own. Each of those rests on that
    one verifier's evidence; nothing checked them a second time.
  - The nine groups and the gap round compiled or ran 233 probe programs. The
    verifiers' own probes are not in that count.
- **Re-run by the controller, not taken on report:**
  - every code in the two tables below, and the codes in §3.1's list and §3.2,
    from 48 one-construct programs. Each program is valid except for the construct
    named, so each code belongs to that construct alone;
  - the router in §3.2;
  - the cycle in §4.2;
  - the stdout defect in §5;
  - the two async-handler programs in §3.1.
- **Finding ids are labels, not links.** The Findings column and the ids in
  parentheses name findings in the fan-out's output, which is not tracked. The
  prefix before the hyphen is the group:
  - A imports, B the HTTP builder, C the SIGTERM lifecycle;
  - D declarations and JSON, E the process surface;
  - F1 Option and String, F2 iteration and mutation;
  - G the filesystem cycle, H the harness and naming;
  - GAP the gap round.

  An `-R` in an id marks a finding a verifier added.

The probe sources are not tracked. To reproduce a table row's code, write the
construct into an otherwise valid program and run `nova check`:
- items (imports, `const`, attributes, aliases, `impl`) at top level;
- expressions in `fn main`, or in an `async fn main` where the construct awaits;
- any record or typed helper fn the construct needs, declared beside it.

A probe declares its own receivers and names its own variables, so a message can
name `t`, `v`, `Vec<Int>` or `?3` where the listing would name something else.
Where the construct itself was probed in another spelling, the Diagnostic cell
says so.

---

## 2. The headline

- **Neither listing compiles as written. Each fails on its first line:**
  `import std/http` and `import std/fs` are `P0001`, `expected item ...,
  found /`. That is the finding the 2026-09-03 amendment under `60-EXAMPLES.md`
  §5 made about that section's listing. These two are written in the same Nova
  that does not exist.
- **The first gate clause of `60-EXAMPLES.md` §3 is reachable today without
  changing the compiler's or the runtime's source.**
  - A `Server` router written in the example's own source served
    `Hello from Nova!` on `0.0.0.0:3000` on this host.
  - `std/http` can gain the same type as Nova source. That means rebuilding
    `nova.exe`, and that it compiles inside std is inferred (§7).
- **The second clause, "exits cleanly on SIGTERM", is undefined.** The runtime
  observes no signal at all.
- **`60-EXAMPLES.md` §4 has a route in today's Nova for everything except the
  program's arguments, which have no portable route.**
  - The add → list → done → list cycle ran with one process per command, each
    command read from a file instead of argv.
  - A built exe on this Windows host can read its arguments through FFI (§4.3).
    Linux and macOS were not probed. Under `nova run` a program cannot be given
    arguments at all.
  - `exit(code)` has a working FFI route, but not a std one.

---

## 3. `03-http-server`

### 3.1 Every construct

The code column comes from one program per construct (§1).

| `60-EXAMPLES.md` §3 reaches for | Present? | Diagnostic | Route in today's Nova | Fix locus | Findings |
|---|---|---|---|---|---|
| `import std/http`, `import std/log` | no | `P0001` found `/`. `import std::http` is `E0900`: qualified import paths not supported | delete the line: every std module is glob-imported implicitly | spec drift: `11-PARSER.md:67,171` paths are `::`, and std modules have no importable name | A-1, A-4 |
| `log.init()`, `log.info(...)` | no | `E0001` cannot find `log` | `Log::init()`, `Log::info(...)`; ran, writes a timestamped line to stderr | spec drift: `std/log` ships `Log::` associated fns on purpose | A-5, A-6 |
| `http.Server.new()` | no | `Server::new()` is `E0900` "module-qualified paths", because with no `Server` type the path reads as a module path. `Response.text(..)`, a `.` call on a type, is `E0001` cannot find `Response` | a user-level router: §3.2 | std (`std/http/lib.nova`, writable in Nova); spec drift for `http.` and for `.` on a type | B-1, B-2, B-3 |
| `.get(...)` continuation lines starting with `.` | **yes** | — | — | — | B-4 |
| `\|_\| ...` handler closures | **yes** | ok | — | — | B-7 |
| a handler that reads a field of its unannotated request before anything in its body has fixed that type, `\|req\| req.path` (not in `60-EXAMPLES.md` §3, but any real route) | no | `E0014` cannot access field `path` on `?0` | annotate `\|req: Request\|`, pass `req` straight to a typed fn, or rebind it with a typed `let` | typechecker: `check_closure` (`check.rs:4582`) checks the body with no expected type | B-8, F1-3, G-R1 |
| `Response.text("Hello from Nova!")`, one argument | no | probed as `Response::text("...")`: `E0016` takes 2 arguments. The `.` spelling is `E0001`, as in the `http.Server.new()` row | `Response::text(200, "...")` | std or spec: three meanings disagree, see §5 | B-9 |
| `Response.json(...)` | no | probed as `Response::json(..)`: `E0001` no variant `json` on type `Response`. The `.` spelling is `E0001` cannot find `Response`, as in the `http.Server.new()` row | an `impl Response { pub fn json(v: JsonValue) -> Response }` in user code; ran | std (`std/http/lib.nova`); conflicts with `20-STDLIB.md:495` | B-10, B-R7 |
| `{ "status": "ok" }` | no | `P0001` | `Map::new()`, `insert`, then `Object(m)` | parser and typechecker; no spec chapter defines a map literal | B-11 |
| `app.listen(addr).await.unwrap()` | **yes** (shape) | — | an `async fn listen(self, ...)` in an impl, awaited and unwrapped | — | B-12, B-15 |

Not in `60-EXAMPLES.md` §3, but in the API it assumes: `20-STDLIB.md:519`'s
`pub type Handler = async fn(Request) -> Response`.
- The async alias is `P0001`, and a sync alias is `E0900`: type aliases are not
  supported yet.
- The working spelling for a synchronous handler is `fn(Request) -> Response`,
  written inline, and `60-EXAMPLES.md` §3's handlers are all synchronous (B-16).
- **An async handler needs no compiler change either.** It is stored as
  `fn(Request) -> Future<Response>`, then called and awaited. The controller
  re-ran two programs, neither served over HTTP:
  - a named `async fn` that awaits a `sleep`, held in a record field of that
    type, bound to a local, called and awaited, returned `200 users at /users`
    (B-16's route, B-R8);
  - a router whose `async fn dispatch` walks a `Vec<Route>`, matches the path and
    awaits the stored handler printed `200 users at /users`, and `404` for an
    unknown path.
- Async closures do not exist: `async |n: Int| n + 1` is `P0001`, found `async`
  (B-16).

### 3.2 First gate clause, `curl http://localhost:3000/` returns `Hello from Nova!`: reached by a substitute

The router is one 105-line user file, comments and `main` included:
- `record Route { path: String, handler: fn(Request) -> Response }`;
- `record Server { routes: Vec<Route> }`;
- `get(self, path, handler) -> Server`;
- a dispatch on `req.method` and `req.path`;
- an `async fn listen(self, addr)` that binds, accepts, and spawns one task per
  connection, as `examples/05-json-api` does.

Its `main` stays close to the listing: `Server::new().get("/", |_|
Response::text(200, "Hello from Nova!")).get("/health", ...)`.

Re-run by the controller with a fresh build, 664,064 bytes, on this host:
- `GET /` returns `200`, `content-length: 16`, and body `Hello from Nova!`;
- `/health` returns `200`, `application/json`, and `{"status":"ok"}`;
- an unknown path returns `404`.

**`localhost` took 0.213 s against 0.0018 s for `127.0.0.1`**, one curl
`time_total` reading each. curl tries `::1` first, and a `0.0.0.0` bind is IPv4
only. curl's happy-eyeballs timer starts the IPv4 attempt about 200 ms later,
while the `::1` attempt is still pending; that attempt is refused only after
about 2 s (GAP-3). Binding `[::]` as well removes the delay.

Three things the router needed beyond the listing:
- **The `get` builder takes `self`, not `mut self`.** A `mut self` call on the
  temporary `Server::new()` returns is `E0060`.
- **`get` returns `Server`, not the `-> Self` that `20-STDLIB.md:509` gives.**
  `Self` as a type is `E0001`, cannot find type `Self`, in an inherent impl and
  in a trait impl alike; the controller re-ran both. It is bound only inside trait
  declarations (B-6).
- **A fn-typed field is bound to a local before it is called.** `r.handler(req)`
  is `E0014` (B-14).

Moving the router into `std/http` means a compiler rebuild, because std is
compiled into `nova.exe` (D-R5).

### 3.3 Second gate clause, "Process exits cleanly on SIGTERM": undefined, and not observable

- **The runtime installs no signal or console-control handler.** The searches
  for `sigaction`, `SIGTERM`, `SetConsoleCtrlHandler`, `ctrlc` and
  `tokio::signal`, over `crates/nova-runtime` and `Cargo.lock`, find none (C-1).
- **On this host, with no handler installed, no termination reached any Nova
  code: each either ended the process outright or did not end it at all** (C-8
  to C-12, C-R3):
  - Ended it: Git Bash `kill -TERM` gave 143, ending the process from outside,
    so no handler could see it (C-10). `taskkill /F` gave 1 (C-9). `CTRL_BREAK`
    gave `0xC000013A` (C-11). `CTRL_C` gave `0xC000013A`, but only once the
    Ctrl-C ignore flag that children inherit here had been reset (C-12, C-R3).
  - Did not end it: a plain `taskkill` was refused as "can only be terminated
    forcefully", and the server kept serving (C-8). `CTRL_C` with the flag
    inherited left the process alive (C-12, C-R3), and so did Git Bash
    `kill -INT`, still alive after 3 s (C-10).
  - Unix was not measured. With no handler, the default disposition applies,
    which is reasoning, not a measurement.
- **`main` returning does not end a server.** `block_on` "implicitly joins
  everything" (`docs/adr/0009-async-execution-model.md:143-145`), so one idle
  connection keeps the process alive (C-2).
- **A clean exit 0 is reachable today without a signal.** Two shapes ran:
  - an accept loop that stops after a count, then closes its listener;
  - an accept polled under `std/time`'s `timeout` with a stop condition, with
    every connection's reads bounded the same way (C-3, C-15, and the correction
    to H-2).

  What is missing is only the signal as an input.
- **"Cleanly" is not defined (C-16).** It reads one of two ways:
  - **(a) The process terminates on the signal's default disposition.** No work
    needed on Unix, by the reasoning above (not measured): a parent should see
    signal 15, not exit 0. Windows has no SIGTERM.
  - **(b) Graceful: stop accepting, finish in-flight requests, exit 0.** This
    needs a runtime signal source that Nova can observe, such as a pollable flag,
    or an accept that returns `Err` on shutdown. On Windows the equivalent is a
    console control event.

  A Windows-only FFI route to (b) exists in a built exe (C-R2), but it is not a
  route for an example CI runs on three operating systems.

---

## 4. `04-todo-cli`

### 4.1 Every construct

| `60-EXAMPLES.md` §4 reaches for | Present? | Diagnostic | Route in today's Nova | Fix locus | Findings |
|---|---|---|---|---|---|
| the four `import std/...` lines | no | `P0001` found `/` | delete them: `println` and `eprintln` are builtins, std/fs and std/json are glob-imported, and std/process does not exist (see the `args()` and `exit(1)` rows) | spec drift | A-1, A-3, E-1, E-2 |
| `@derive(ToJson, FromJson, Clone)` | no | `E0082` unknown attribute; the known set is `test` | hand-written `impl FromJson for Todo` and an encoder; `Clone` is never called | resolver, plus an impl-synthesis step that does not exist | D-1, D-2 |
| record fields separated by newlines | **yes** | — | — | — | D-3 |
| `const DB_PATH = "todos.json"` | no | `P0001` expected `:` | `const DB_PATH: String = ...` | spec drift: `11-PARSER.md:65` requires the type | D-4, G-6 |
| `-> [Todo]` with `return []` | **yes** | — | — | — | D-5 |
| `[Todo]` used as a growable list | no | `push` and `iter` on an array are `E0014`. A `for` over an array, or over a `Vec`, is `E0900` (try `.iter()`) | `Vec<Todo>` throughout, with `.iter()` | spec drift: arrays are fixed-length (`docs/phase-2-plan.md:30`), though the parser spec's fixture assumes otherwise (§5) | D-9, F2-1, F2-4, F2-6 |
| `!fs.exists(DB_PATH).await` | `!x.await` **yes**; `fs.` no | `E0001` cannot find `fs` | `!exists(DB_PATH).await` | spec drift | GAP-1, G-1 |
| `fs.read(..).await.unwrap_or([])` | no | for `unwrap_or([])`, probed as `read(..)` in an `async fn main`: `E0010`, `[?0]` given where `Bytes` was expected. `fs.` is `E0001`, as in the row above | `unwrap_or(bytes_from_string(""))`, or `read_to_string` | spec drift: `fs.read` returns `Bytes` | F1-11, G-2 |
| `String::from_utf8(bytes)` | no | `E0001` no associated function `from_utf8` | `bytes.to_string()`, which returns `Option<String>` | spec drift (renamed), or std | F1-10, G-3 |
| `json.parse(s).and_then(\|v\| ...)` | **yes**, without `json.` | — | `parse(s).and_then(...)` | — | D-12 |
| `Vec::<Todo>::from_json(v)` | no | the turbofish is `P0001` (`Vec::<Int>::new()`: chained comparison). There is no `FromJson for Vec` | the expected type drives inference with no turbofish, plus a decoder for the array | parser (turbofish); std (`impl<T: FromJson> FromJson for Vec<T>`, ran as user code) | D-7, D-8 |
| `todos.to_json()` | no | `E0014` no method `to_json` on `Vec<Int>` (on `[Int]` too) | interpolate each record, escaping through `stringify(String(..))`, as 05 does; or a generic `array_to_json<T: ToJson>` or `impl<T: ToJson> ToJson for Vec<T>`, both ran as user code | std, for `Vec<T>`. An impl on `[T]` is refused: `E0010` impl blocks are only supported on named types | D-6, D-13, G-12 |
| `json.stringify_pretty(v, 2)` | no | probed as `stringify_pretty(..)`: `E0001` cannot find function. The `json.` spelling is `E0001` cannot find `json` | write it: about 30-45 lines of Nova, ran, and `parse` reads its output back | std (`std/json/lib.nova`). The signature is already declared at `20-STDLIB.md:548` | A-8, D-10, GAP-4 |
| `fs.write_string(..).await.unwrap()` | **yes**, without `fs.` | — | — | — | G-4 |
| `args()` | no | `E0001` cannot find function `args` | **none portable**: see §4.3 | runtime builtin, std/process, cli, codegen | A-2, E-3, E-8, G-8 |
| `argv.get(1)` | on an array, no | `E0014` no method `get` on `[String]` | have `args()` return `Vec<String>`, whose `get` returns `Option` | std (the return type) | F1-1 |
| `.map(\|s\| s.as_str())` | no | `E0011` cannot infer the receiver's type (the closure parameter). Annotated `\|s: String\|`, it is `E0014` no method `as_str` | drop it: Nova has one `String` type | spec drift for `as_str`; typechecker (`check_closure`) for the inference | F1-3, F1-4 |
| `Some("add") => ...` | no | `E0900` nested patterns inside variants | `match argv.get(1).unwrap_or("") { "add" => .. "list" => .. }`; top-level string patterns work | typechecker, HIR and MIR match lowering | F1-5, F2-12, G-R3 |
| `"untitled".to_string()` | no | `E0014` no method `to_string` on `String` | `"untitled"` | spec drift | F1-6 |
| `.map(\|t\| t.id)` | no | `E0014` cannot access field `id` on `?3` | `\|t: Todo\| t.id` | typechecker (`check_closure`) | F2-3, G-R1 |
| `.max()` | no | probed as `v.iter().max()`: `E0014` no method `max` on `VecIter<Int>` | `fold`, or a bounded `impl<I: Iterator, U: Ord>` on `MapIter`, which ran | std (`std/core/lib.nova`) | F2-2, F2-R1 |
| `Todo { id, title, done: false }` | **yes** | ok | — | — | D-16, F2-5 |
| `if todo.done { "[x]" } else { "[ ]" }`; `"${todo.id}"` | **yes** | — | — | — | F2-7, F2-8 |
| `s.parse::<Int>().ok()` | no | `parse::<Int>` is `P0001`; `"1".parse()` is `E0014`; `.ok()` is `E0014` | `parse(s)` from `std/json`, then `Int::from_json`, as 05's `path_id` does. Not `parse::<Int>()`'s semantics: the text goes through a JSON `Float`, so it accepts ` 7 `, `7.0` and `1e2`, rejects `007` and `9223372036854775807`, and rounds above 2^53. Harmless for todo ids | std (`parse<T: FromStr>` and `Result::ok`, both ran as user code); parser for the turbofish | F1-7, F1-8, F1-9, G-R9 |
| `for todo in &mut todos` | no | `E0900` reference operators are not supported yet | `for t in todos.iter() { let mut u = t; ... }` | spec drift: `20-STDLIB.md:232` puts references off the roadmap | F2-9, G-14 |
| `todo.done = true` on the loop variable | no | `E0060` cannot assign to a field of immutable `t` | rebind it `mut`. The write is visible through the collection because records alias. Aliasing through a `let mut` rebinding is pinned by `tests/runtime/field_assign.nova`; no test pins it for an element reached through a `Vec` | spec drift: a `for` loop variable is immutable by design (`check.rs:4318`) | F2-10, D-R2 |
| `println`, `eprintln` | **yes** | — | — | — | E-5, E-6 |
| `exit(1)` | no | `E0001` cannot find function `exit` | `extern "C" { fn exit(code: Int) }`, exit code 1 under both `run` and `build` | runtime builtin (`std::process::exit`) plus std/process | E-4, C-R1, E-10 |

### 4.2 Gate, "Full CLI cycle works (add → list → done → list)": reached by a substitute, minus argv

A program today's Nova accepts, using no FFI, ran the full cycle. It reads each
command from a `cmd.txt` in the working directory, and in place of the listing's
constructs it uses:
- a hand-written JSON codec;
- `Vec<Todo>`;
- an `if` chain for dispatch.

Re-run by the controller, built, 636,928 bytes, one process per command, on this
host:
- `add buy milk` printed `added: 1`, and `add walk dog` printed `added: 2`.
- `list` printed `[ ] 1: buy milk` and `[ ] 2: walk dog`.
- `done 1` printed nothing.
- `list` printed `[x] 1: buy milk` and `[ ] 2: walk dog`.
- The final file was
  `[{"id":1,"title":"buy milk","done":true},{"id":2,"title":"walk dog","done":false}]`.
- The same cycle under `nova run` gave the same results.
- An unknown command printed the usage line and **exited 0**: the missing `exit`.

### 4.3 What still separates that from the gate

- **`args()`.**
  - No builtin and no std module gives a program its arguments. None offers a
    general environment-variable read either.
  - Both C `main` shims drop `argc` and `argv`. Cranelift's `emit_c_main`,
    `crates/nova-codegen-cranelift/src/lib.rs:288-323`, and the LLVM backend's,
    `crates/nova-codegen-llvm/src/lib.rs:137-139`, both call `nova_main()` and
    return 0.
  - Under `nova run` the program runs inside `nova.exe`, so the process's own
    argv is nova's.
  - A built exe on Windows can still read the CRT's argv through
    `__p___argc`/`__p___argv` FFI (E-R1).
  - An FFI `getenv` reads an environment variable on this host under both
    `nova run` and a built exe (E-R2). That is an input channel, not argv.
  - **Why no argv route is portable.** Every argv route that ran uses symbols
    only Windows has: `__p___argc`/`__p___argv`, or `GetCommandLineA` for the raw
    command line. Nova has no conditional compilation (the known attribute set is
    `test`), so one example source cannot carry a per-OS route. Linux and macOS
    were not probed.
- **`nova run` cannot pass arguments through.** `RunCmd`
  (`crates/nova-cli/src/cmd/run.rs:11-15`) has only `file`.
  - `nova run src/main.nova add foo` exits 2 on an unexpected argument.
  - `nova run -- 20` takes `20` as the FILE.
- **`02-fibonacci`'s own gate is unmet too, and could not be met today for the
  same reasons.**
  - `60-EXAMPLES.md:55` asks for `nova run -- 20` to print `fib(20) = 6765`,
    and that has never been met.
  - The example has hard-coded `let n = 10` since the Phase 0 skeleton, and it
    prints `fibonacci(10) = 55`.
  - What it met is `00-MASTER-SPEC.md`'s Phase 1 gate, which asks only that it
    run (E-17).
- **An exit status.**
  - The FFI `exit` above works.
  - libc `exit` drops an unflushed partial line, which `std::process::exit`
    would flush (E-10).
  - `panic` aborts with `0xC0000409` on Windows, not 1.
  - `fn main() -> Int` is accepted and its value is silently ignored. The
    `main` check at `crates/nova-mir/src/mono.rs:26-31` looks at generics and
    parameters only (C-6, G-R5).

---

## 5. Across both

- **The spec contradicts itself in at least five places these listings touch:**
  - **Import paths.** The `11-PARSER.md:67,171` grammar has `import path` with
    `::` paths, and `20-STDLIB.md:1715-1716` says Nova has no import statements
    and no qualified paths. Both `60-EXAMPLES.md` §3 and §4 write
    `import std/...`.
  - **Const types.** The `11-PARSER.md:65` grammar requires a const's type;
    `60-EXAMPLES.md` §4 omits it.
  - **Arrays as lists.** The `11-PARSER.md:370-374` fixture pushes onto and
    iterates a `[T]`, the same assumption `60-EXAMPLES.md` §4 makes, and it fails
    in the same way (F2-R3).
  - **`Response::json` and `Response::text` have opposite meanings.**
    `20-STDLIB.md:495-496` declares them as client-side decoders,
    `60-EXAMPLES.md` §3 uses them as constructors, and `std/http/lib.nova:346`
    has `text(status, s)` (B-9, B-10).
  - **References.** `20-STDLIB.md:232-234` puts references "off this roadmap
    permanently", yet `60-EXAMPLES.md` §4 writes `&mut` (F2-9).
- **`nova test` cannot test an example the way `60-EXAMPLES.md` §10 asks.**
  - In any example directory it collects only `@test` functions reachable from
    `src/main.nova`, so it reports 0 tests and exits 0.
  - A `tests/` folder is invisible to it (H-3, H-4).
  - No example meets `60-EXAMPLES.md` §10 (H-5).
- **The 03 slot is taken.** `examples/03-producer-consumer` holds it, with one code
  reference at `crates/nova-cli/tests/run_tests.rs:1322` (H-8, H-9).
- **Records this inventory narrows.**
  - `docs/superpowers/specs/2026-09-01-std-http-request-parsing-design.md` §1
    says `std/http` being absent is why 03, 04 and 05 "cannot be written".
    `60-EXAMPLES.md` §4's listing uses no `std/http` (H-17).
  - Several records give the unparseable `Handler` alias as the reason no
    `Server.get` can exist, though §3.1 and §3.2 show a router needs no alias.
  - This branch amends these docs:
    - `examples/05-json-api/README.md`, whose sentence is rewritten in place;
    - a paragraph among `60-EXAMPLES.md` §5's amendments, with a dated note;
    - `20-STDLIB.md` §6's amendment, with a dated note;
    - the 2026-09-01 design above, with dated notes in its §1 and §3;
    - `docs/superpowers/specs/2026-08-23-std-net-listener-design.md`, which says
      there is "no graceful-shutdown path", with a dated note.
  - Other records repeat these claims and are not amended here. That covers
    design specs, plans and a CHANGELOG entry from before this branch, found by
    `git grep` for `Handler` near `P0001` or "does not parse". It is not a
    claim that no other record says it. Two std source comments are among them,
    left alone because std is compiled into `nova.exe`:
    - `std/http/lib.nova:10-14` (the alias);
    - `std/net/lib.nova:278-282` (no graceful-shutdown path).
- **A defect found while probing, which predates this work.**
  - A built executable drops a trailing partial line written with
    `stdout().write(...)` when `main` returns. `nova run` keeps it, so the JIT and
    the built (Cranelift object) executable disagree on output.
  - Re-run by the controller: built, `full line\n`; `nova run`,
    `full line\npartial-no-newline`.
  - The cause is probably that the C `main` shims return without the flush
    Rust's own runtime would do (E-11).
  - The LLVM backend's `main` also returns with no flush; whether it drops the
    line too is not measured.
  - It is not fixed here; it is recorded as separate work.

---

## 6. Recommended order, and the decisions that are the user's

**Precedents for listing drift.** Three examples on disk diverge from their spec
listings:
- `01-hello-world` silently dropped its `import std/fmt { println }`, the same
  first-line failure §2 reports.
- `02-fibonacci` never read its arguments (§4.3).
- `05-json-api` kept its listing "as the aspiration it always was" and wrote the
  example in the language that exists. The substitutions are recorded in the
  spec section's own dated amendment and in a design-spec table.

The fourth, `03-producer-consumer`, has no listing.

None of the language features inventoried in §3 and §4 blocks either gate:
- `@derive`;
- nested literal patterns;
- closure parameter inference;
- the turbofish;
- map literals;
- `&mut`;
- qualified std access;
- type aliases.

Each has a route that ran. What blocks a gate is narrower:

1. **`03-http-server` first.** No change to the compiler's or the runtime's
   source is needed for clause 1:
   - `std/http` gains a `Server` (route table, `get`, `listen`) and a
     `Response::json`, written in Nova. That means a rebuild (§3.2), and that it
     compiles inside std is inferred (§7).
   - The example is written in today's spellings, under the 05 precedent.
   - A README follows `60-EXAMPLES.md` §9's template.
   - An end-to-end test follows the 05 pattern. It binds port 0 and prints the
     port, because a fixed `:3000` collides on a shared runner.

   Clause 2 waits on decision (i) below.
2. **`std/process` next.** It needs:
   - `args()` and `exit(code)` as runtime builtins;
   - `exit` via `std::process::exit`, so output is flushed;
   - `nova run` passing arguments through.

   This touches:
   - the CLI, the driver, the runtime, the resolver and the typechecker;
   - MIR: the `RtFunc` table in `crates/nova-mir/src/lib.rs`, and the exhaustive
     builtin match in `lower.rs`;
   - the codegen shims, if argv is forwarded rather than read by the runtime
     itself.

   It is also what `02-fibonacci`'s `nova run -- 20` gate needs, but it does not
   clear that gate on its own. That example's `main` and the test that pins
   `fibonacci(10) = 55` (`crates/nova-cli/tests/run_tests.rs:30-37`) would both
   have to be rewritten.
3. **`04-todo-cli` last.** It needs std additions written in Nova:
   - `stringify_pretty`;
   - `Vec` codecs;
   - `Result::ok`;
   - a String-to-Int parse;
   - an iterator `max`.

   Then the example, on top of 2.

**Decisions this record cannot make:**
- **(i) What "exits cleanly on SIGTERM" means:** meaning (a) or (b) in §3.3. Under
  (a) the runtime needs no work. Under (b) it needs a signal source.
- **(ii) Where 03 goes:** beside `03-producer-consumer`, or with a renumber.
- **(iii) Whether the listings are kept as aspiration,** as 05's was, or rewritten
  into the language that exists.

---

## 7. What is not covered

- **Only this Windows host.** Linux and macOS were not measured: SIGTERM's
  default disposition, the panic exit code and argv under glibc are all reasoned,
  not measured.
- **The LLVM backend.**
  - `nova build --release` emits LLVM IR and then exits 1, because the host has
    no `clang` or `llc` (G-R7, E-R6). So no LLVM-built executable was linked or
    run.
  - Its generated `main` drops `argc` and `argv`, calls `nova_main()` and returns
    0, as Cranelift's does (§4.3).
  - Its argv, exit-status and stdout-flush behaviour at run time is not measured.
- **No std edit was compiled into `nova.exe`.** That would have meant changing
  the repository. Every std-located route ran as user code, so whether it
  compiles inside std is inferred.
- **The 78 findings the verifiers added** have one verifier's evidence each and
  no second check.
