# Phase 2 gate examples 03 and 04: what blocks them, measured

**Date:** 2026-10-04. **Measured against:** `main` at `3f4fdfb`, with the release
`nova.exe` current with it (a no-op `cargo build --release -p nova-cli -p
nova-runtime` confirmed it). **Host:** this development host, Windows 11.

This is a record, not a design. It inventories what stands between today's Nova
and the two Phase 2 gate examples that do not exist yet:
`nova-spec/60-EXAMPLES.md` §3, `03-http-server`, and §4, `04-todo-cli`. Nothing
in the repository changed to produce it. No probe program is kept.

---

## 1. Method, and how far to trust each column

- **The verbatim listings first.** Both listings were extracted byte for byte from
  the spec, lines 65-77 for §3 and 97-155 for §4, and run through `nova check`.
  Both fail on their first line (§2).
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
  - every diagnostic code in the two tables below, from 41 one-construct programs.
    Each program is valid except for the construct named, so each code belongs to
    that construct alone;
  - the §3 router (§3.2);
  - the §4 cycle (§4.2);
  - the stdout defect (§5).

The probe sources are not tracked. A table row's code can be reproduced by
writing the construct into an otherwise valid `fn main() {}` and running
`nova check`.

---

## 2. The headline

- **Neither listing compiles as written. Each fails on its first line:**
  `import std/http` and `import std/fs` are `P0001`, `expected item ...,
  found /`. That is the finding the 2026-09-03 amendment under §5 made about
  §5's listing. These two are written in the same Nova that does not exist.
- **§3's first gate clause is reachable today with no compiler or runtime
  change.**
  - A `Server` router written in Nova served `Hello from Nova!` on
    `0.0.0.0:3000`.
  - `std/http` can gain that type as Nova source.
- **§3's second clause, "exits cleanly on SIGTERM", is undefined.** The runtime
  observes no signal at all.
- **§4 has a route in today's Nova for everything except one gap: there is no
  way to get the program's arguments.**
  - The add → list → done → list cycle ran over four separate processes, with
    each command read from a file instead of argv.
  - `args()` has no portable route.
  - `exit(code)` has a working FFI route, but not a std one.
  - `nova run` cannot pass arguments through to a program.

---

## 3. `03-http-server`

### 3.1 Every construct

The code column comes from one program per construct (§1).

| §3 reaches for | Present? | Diagnostic | Route in today's Nova | Fix locus | Findings |
|---|---|---|---|---|---|
| `import std/http`, `import std/log` | no | `P0001` found `/`. `import std::http` is `E0900`: qualified import paths not supported | delete the line: every std module is glob-imported implicitly | spec drift: `11-PARSER.md:67,171` paths are `::`, and std modules have no importable name | A-1, A-4 |
| `log.init()`, `log.info(...)` | no | `E0001` cannot find `log` | `Log::init()`, `Log::info(...)`; ran, writes a timestamped line to stderr | spec drift: `std/log` ships `Log::` associated fns on purpose | A-5, A-6 |
| `http.Server.new()` | no | `Server::new()` is `E0900` "module-qualified paths", because with no `Server` type the path reads as a module path. `Response.text(..)`, a `.` call on a type, is `E0001` | a user-level router: §3.2 | std (`std/http/lib.nova`, writable in Nova); spec drift for `http.` and for `.` on a type | B-1, B-2, B-3 |
| `.get(...)` continuation lines starting with `.` | **yes** | — | — | — | B-4 |
| `|_| ...` handler closures | **yes** | ok | — | — | B-7 |
| a handler that *uses* its request, `|req| req.path` (not in §3, but any real route) | no | `E0014` cannot access field `path` on `?0` | annotate `|req: Request|`, or pass `req` straight to a typed fn | typechecker: `check_closure` (`check.rs:4582`) checks the body with no expected type | B-8, F1-3, G-R1 |
| `Response.text("Hello from Nova!")`, one argument | no | `E0016` takes 2 arguments | `Response::text(200, "...")` | std or spec: three meanings disagree, see §5 | B-9 |
| `Response.json(...)` | no | `E0001` no variant `json` on type `Response` | an `impl Response { pub fn json(v: JsonValue) -> Response }` in user code; ran | std (`std/http/lib.nova`); conflicts with `20-STDLIB.md:487` | B-10, B-R7 |
| `{ "status": "ok" }` | no | `P0001` | `Map::new()`, `insert`, then `Object(m)` | parser and typechecker; no spec chapter defines a map literal | B-11 |
| `app.listen(addr).await.unwrap()` | **yes** (shape) | — | an `async fn listen(self, ...)` in an impl, awaited and unwrapped | — | B-12, B-15 |

Not in §3, but in the API it assumes: `20-STDLIB.md:511`'s
`pub type Handler = async fn(Request) -> Response`.
- The async alias is `P0001`, and a sync alias is `E0900`: type aliases are not
  supported yet.
- The working spelling is `fn(Request) -> Response` written inline, which §3's
  synchronous handlers are content with (B-16).

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

Re-run by the controller with a fresh build, 664,064 bytes:
- `GET /` returns `200`, `content-length: 16`, and body `Hello from Nova!`;
- `/health` returns `200`, `application/json`, and `{"status":"ok"}`;
- an unknown path returns `404`.

**`localhost` paid 0.213 s against 0.0018 s for `127.0.0.1`.** curl tries `::1`
first, and a `0.0.0.0` bind is IPv4 only, so the first attempt is refused before
the IPv4 fallback (GAP-3). Binding `[::]` as well removes the delay.

Three things the router needed beyond the listing:
- **The `get` builder takes `self`, not `mut self`.** A `mut self` call on the
  temporary `Server::new()` returns is `E0060`.
- **A fn-typed field is bound to a local before it is called.** `r.handler(req)`
  is `E0014` (B-14).
- **`std/http` is compiled into `nova.exe`.** Moving the router there means a
  compiler rebuild (D-R5).

### 3.3 Second gate clause, "Process exits cleanly on SIGTERM": undefined, and not observable

- **The runtime installs no signal or console-control handler.** The searches
  for `sigaction`, `SIGTERM`, `SetConsoleCtrlHandler`, `ctrlc` and
  `tokio::signal`, over `crates/nova-runtime` and `Cargo.lock`, find none (C-1).
- **On this host every termination tried ended the process outright** (C-8 to
  C-12):
  - Git Bash `kill -TERM` gave 143.
  - `CTRL_BREAK` and `CTRL_C` gave `0xC000013A`.
  - `taskkill /F` gave 1.
  - A plain `taskkill` was refused as "can only be terminated forcefully", and
    the server kept serving.
  - Unix was not measured. With no handler, the default disposition applies,
    which is reasoning, not a measurement.
- **`main` returning does not end a server.** `block_on` "implicitly joins
  everything" (`docs/adr/0009-async-execution-model.md:143-145`), so one idle
  connection keeps the process alive (C-2).
- **A clean exit 0 is reachable today without a signal.** Two shapes ran:
  - an accept loop that stops after a count, then closes its listener;
  - an accept polled under `std/time`'s `timeout` with a stop condition, and
    reads bounded the same way (C-3, C-15, and the correction to H-2).

  What is missing is only the signal as an input.
- **"Cleanly" is not defined (C-16).** It reads one of two ways:
  - **(a) The process terminates on the signal's default disposition.** No work
    needed on Unix, where a parent sees signal 15, not exit 0. Windows has no
    SIGTERM.
  - **(b) Graceful: stop accepting, finish in-flight requests, exit 0.** This
    needs a runtime signal source that Nova can observe, such as a pollable flag,
    or an accept that returns `Err` on shutdown. On Windows the equivalent is a
    console control event.

  A Windows-only FFI route to (b) exists in a built exe (C-R2), but it is not a
  route for an example CI runs on three operating systems.

---

## 4. `04-todo-cli`

### 4.1 Every construct

| §4 reaches for | Present? | Diagnostic | Route in today's Nova | Fix locus | Findings |
|---|---|---|---|---|---|
| the four `import std/...` lines | no | `P0001` found `/` | delete them: `println` and `eprintln` are builtins, and the rest are glob-imported | spec drift | A-1, A-3, E-1, E-2 |
| `@derive(ToJson, FromJson, Clone)` | no | `E0082` unknown attribute; the known set is `test` | hand-written `impl FromJson for Todo` and an encoder; `Clone` is never called | resolver, plus an impl-synthesis step that does not exist | D-1, D-2 |
| record fields separated by newlines | **yes** | — | — | — | D-3 |
| `const DB_PATH = "todos.json"` | no | `P0001` expected `:` | `const DB_PATH: String = ...` | spec drift: `11-PARSER.md:65` requires the type | D-4, G-6 |
| `-> [Todo]` with `return []` | **yes** | — | — | — | D-5 |
| `[Todo]` used as a growable list | no | `push` and `iter` on an array are `E0014`. A `for` over an array, or over a `Vec`, is `E0900` (try `.iter()`) | `Vec<Todo>` throughout, with `.iter()` | spec drift: arrays are fixed-length (`docs/phase-2-plan.md:30`), though the parser spec's fixture assumes otherwise (§5) | D-9, F2-1, F2-4, F2-6 |
| `!fs.exists(DB_PATH).await` | `!x.await` **yes**; `fs.` no | `E0001` cannot find `fs` | `!exists(DB_PATH).await` | spec drift | GAP-1, G-1 |
| `fs.read(..).await.unwrap_or([])` | no | `E0010`: `[?0]` given where `Bytes` was expected | `unwrap_or(bytes_from_string(""))`, or `read_to_string` | spec drift: `fs.read` returns `Bytes` | F1-11, G-2 |
| `String::from_utf8(bytes)` | no | `E0001` no associated function `from_utf8` | `bytes.to_string()`, which returns `Option<String>` | spec drift (renamed), or std | F1-10, G-3 |
| `json.parse(s).and_then(\|v\| ...)` | **yes**, without `json.` | — | `parse(s).and_then(...)` | — | D-12 |
| `Vec::<Todo>::from_json(v)` | no | the turbofish is `P0001` (`Vec::<Int>::new()`: chained comparison). There is no `FromJson for Vec` | the expected type drives inference with no turbofish, plus a decoder for the array | parser (turbofish); std (`impl<T: FromJson> FromJson for Vec<T>`, ran as user code) | D-7, D-8 |
| `todos.to_json()` | no | `E0014` no method `to_json` on `Vec<Int>` (on `[Int]` too) | interpolate each record, escaping through `stringify(String(..))`, as 05 does | std, for `Vec<T>`. An impl on `[T]` is refused (`E0010`) | D-6 |
| `json.stringify_pretty(v, 2)` | no | `E0001` cannot find function | write it: about 30-45 lines of Nova, ran, and `parse` reads its output back | std (`std/json/lib.nova`). The signature is already declared at `20-STDLIB.md:540` | A-8, D-10, GAP-4 |
| `fs.write_string(..).await.unwrap()` | **yes**, without `fs.` | — | — | — | G-4 |
| `args()` | no | `E0001` cannot find function `args` | **none portable**: see §4.3 | runtime builtin, std/process, cli, codegen | A-2, E-3, E-8, G-8 |
| `argv.get(1)` | on an array, no | `E0014` no method `get` on `[String]` | have `args()` return `Vec<String>`, whose `get` returns `Option` | std (the return type) | F1-1 |
| `.map(\|s\| s.as_str())` | no | `E0014` no method `as_str` | drop it: Nova has one `String` type | spec drift | F1-4 |
| `Some("add") => ...` | no | `E0900` nested patterns inside variants | `match argv.get(1).unwrap_or("") { "add" => .. "list" => .. }`; top-level string patterns work | typechecker, HIR and MIR match lowering | F1-5, F2-12, G-R3 |
| `"untitled".to_string()` | no | `E0014` no method `to_string` on `String` | `"untitled"` | spec drift | F1-6 |
| `.map(\|t\| t.id)` | no | `E0014` cannot access field `id` on `?3` | `\|t: Todo\| t.id` | typechecker (`check_closure`) | F2-3, G-R1 |
| `.max()` | no | `E0014` no method `max` on `VecIter<Int>` | `fold`, or a bounded `impl<I: Iterator, U: Ord>` on `MapIter`, which ran | std (`std/core/lib.nova`) | F2-2, F2-R1 |
| `Todo { id, title, done: false }` | **yes** | ok | — | — | D-16, F2-5 |
| `if todo.done { "[x]" } else { "[ ]" }`; `"${todo.id}"` | **yes** | — | — | — | F2-7, F2-8 |
| `s.parse::<Int>().ok()` | no | `parse::<Int>` is `P0001`; `"1".parse()` is `E0014`; `.ok()` is `E0014` | `parse(s)` from `std/json`, then `Int::from_json`, as 05's `path_id` does | std (`parse<T: FromStr>` and `Result::ok`, both ran as user code); parser for the turbofish | F1-7, F1-8, F1-9 |
| `for todo in &mut todos` | no | `E0900` reference operators are not supported yet | `for t in todos.iter() { let mut u = t; ... }` | spec drift: `20-STDLIB.md:232` puts references off the roadmap | F2-9, G-14 |
| `todo.done = true` on the loop variable | no | `E0060` cannot assign to a field of immutable `t` | rebind it `mut`. The write is visible through the collection, because records alias, though no test pins that | — | F2-10, D-R2 |
| `println`, `eprintln` | **yes** | — | — | — | E-5, E-6 |
| `exit(1)` | no | `E0001` cannot find function `exit` | `extern "C" { fn exit(code: Int) }`, exit code 1 under both `run` and `build` | runtime builtin (`std::process::exit`) plus std/process | E-4, C-R1, E-10 |

### 4.2 Gate, "Full CLI cycle works (add → list → done → list)": reached by a substitute, minus argv

The closest program today's Nova accepts reads each command from a `cmd.txt` in
the working directory. In place of the listing's constructs it uses:
- a hand-written JSON codec;
- `Vec<Todo>`;
- an `if` chain for dispatch.

Re-run by the controller, built, 636,928 bytes, one process per step:
- `add buy milk` printed `added: 1`, and `add walk dog` printed `added: 2`.
- `list` printed `[ ] 1: buy milk` and `[ ] 2: walk dog`.
- `done 1` printed nothing.
- `list` printed `[x] 1: buy milk` and `[ ] 2: walk dog`.
- The final file was
  `[{"id":1,"title":"buy milk","done":true},{"id":2,"title":"walk dog","done":false}]`.
- The same cycle under `nova run` gave the same results.
- An unknown command printed the usage line and **exited 0**: the missing `exit`.

### 4.3 What still separates that from the gate

- **`args()`.** No builtin, no std module and no environment access exist.
  - Both C `main` shims drop `argc` and `argv`. Cranelift's `emit_c_main`,
    `crates/nova-codegen-cranelift/src/lib.rs:288-313`, and the LLVM backend's,
    `crates/nova-codegen-llvm/src/lib.rs:137-139`, both call `nova_main()` and
    return 0.
  - Under `nova run` the program runs inside `nova.exe`, so the process's own
    argv is nova's.
  - A built exe on Windows can still read the CRT's argv through
    `__p___argc`/`__p___argv` FFI (E-R1). That is Windows-only, so it is not a
    route for a CI-tested example.
- **`nova run` cannot pass arguments through.** `RunCmd`
  (`crates/nova-cli/src/cmd/run.rs:11-15`) has only `file`.
  - `nova run src/main.nova add foo` exits 2 on an unexpected argument.
  - `nova run -- 20` takes `20` as the FILE.

  This is also why `02-fibonacci`'s gate, `nova run -- 20` (`60-EXAMPLES.md:55`),
  was met with a hard-coded `let n = 10` (E-17).
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

- **The spec contradicts itself in four places these listings touch:**
  - The `11-PARSER.md:65` grammar requires a const's type; §4 omits it.
  - The `11-PARSER.md:370-374` fixture pushes onto and iterates a `[T]`, the same
    assumption §4 makes, and it fails in the same way (F2-R3).
  - `Response::json` and `Response::text` have opposite meanings. `20-STDLIB.md:487-488`
    declares them as client-side decoders, §3 uses them as constructors, and
    `std/http/lib.nova:346` has `text(status, s)` (B-9, B-10).
  - `20-STDLIB.md:232-234` puts references "off this roadmap permanently", yet §4
    writes `&mut` (F2-9).
- **`nova test` cannot test an example the way §10 asks.**
  - In any example directory it collects only `@test` functions reachable from
    `src/main.nova`, so it reports 0 tests and exits 0.
  - A `tests/` folder is invisible to it (H-3, H-4).
  - No example meets §10 (H-5).
- **The 03 slot is taken.** `examples/03-producer-consumer` holds it, with one code
  reference at `crates/nova-cli/tests/run_tests.rs:1322` (H-8, H-9).
- **A record about 04 is mis-scoped.**
  `docs/superpowers/specs/2026-09-01-std-http-request-parsing-design.md` §1
  says `std/http` being absent is why 03, 04 and 05 "cannot be written". §4's
  listing uses no `std/http` (H-17).
- **A defect found while probing, which predates this work.**
  - A built exe drops a trailing partial line written with `stdout().write(...)`
    when `main` returns. `nova run` keeps it, so the two backends disagree on
    output.
  - Re-run by the controller: built, `full line\n`; `nova run`,
    `full line\npartial-no-newline`.
  - The cause is probably that the C `main` shims return without the flush
    Rust's own runtime would do (E-11).
  - It is not fixed here; it is recorded as separate work.

---

## 6. Recommended order, and the decisions that are the user's

**Precedents for listing drift.** Two examples so far have diverged from their
spec listings:
- `05-json-api` kept its listing "as the aspiration it always was" and wrote the
  example in the language that exists. The substitutions are recorded in the
  spec section's own dated amendment and in a design-spec table.
- `02-fibonacci` dropped its arguments silently.

None of the language features §3 and §4 assume blocks either gate:
- `@derive`;
- nested literal patterns;
- closure parameter inference;
- the turbofish;
- map literals;
- `&mut`;
- qualified std access;
- type aliases.

Each has a route that ran. What blocks a gate is narrower:

1. **`03-http-server` first.** No compiler or runtime work is needed for
   clause 1:
   - `std/http` gains a `Server` (route table, `get`, `listen`) and a
     `Response::json`, written in Nova;
   - the example is written in today's spellings, under the 05 precedent;
   - a README follows §9's template;
   - an end-to-end test follows the 05 pattern. It binds port 0 and prints the
     port, because a fixed `:3000` collides on a shared runner.

   Clause 2 waits on decision (i) below.
2. **`std/process` next.** It needs:
   - `args()` and `exit(code)` as runtime builtins;
   - `exit` via `std::process::exit`, so output is flushed;
   - `nova run` passing arguments through, which also clears `02-fibonacci`'s
     `nova run -- 20`.

   This touches the CLI, the driver, the runtime, the resolver and the
   typechecker.
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
- **The LLVM backend.** `nova build --release` was not run, because the host has
  no `clang` or `llc`.
- **No std edit was compiled into `nova.exe`.** That would have meant changing
  the repository. Every std-located route ran as user code, so whether it
  compiles inside std is inferred.
- **The 78 findings the verifiers added** have one verifier's evidence each and
  no second check.
