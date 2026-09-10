# `examples/05-json-api` Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Write Phase 2's gate example in the Nova that exists, measure it honestly with the harness that exists, and claim nothing the measurement does not support.

**Architecture:** One task accepts, one serves each connection — forced by `stage_park`, not chosen. Routing is a `match` over `req.method` and a `split` of `req.path`. State is a plain record with a `mut self` method; no `Mutex`, because ADR 0009 makes single-threading a correctness requirement. Responses are interpolated strings, not `Map`-backed `JsonValue`.

**Tech Stack:** Nova over `std/http`, `std/json`, `std/net`, `std/collections`, `std/strings`. Measurement by `crates/nova-bench-http`, which already exists and has no dependencies.

**Spec:** `docs/superpowers/specs/2026-09-10-examples-05-json-api-design.md`

## Global Constraints

- `cargo build --locked --workspace` **before** `cargo test`. Always.
- `--no-fail-fast` on every test run. **Never pipe cargo output through `head` or `tail` before summing** — sum every `test result:` line across all 45 targets. Baseline: **1124 passed / 0 failed / 8 ignored**.
- `cargo clippy --locked --workspace --all-targets -- -D warnings` clean. CI's own gate additionally passes `--all-features`, on both ubuntu and windows.
- `cargo fmt --all -- --check` clean.
- No `reason = "..."` in any lint attribute — MSRV is 1.78.
- The 8 ignored ADR-0010 GC tests stay ignored and untouched. The poll ABI is **frozen**; no panic may cross a generated poll boundary.
- Every fixture path unique per process.
- **Never `git add -A` or `git add .`** Stage by name. Do not write scratch files into the repository.
- Commit messages written to a UTF-8 file and applied with `git commit -F`. **Never a heredoc.** Body ends exactly with `Co-Authored-By: Claude Opus 5 <noreply@anthropic.com>`.
- **Cite no branch-local SHA** in any tracked file or commit body. Derive the roster with `git log --format=%h main..HEAD`.
- **Byte-scan every file written, on the STAGED content** via `git show :<path>`: valid UTF-8; no byte below 0x20 outside tab/CR/LF; no 0x7f; zero backslash-`u`-four-hex in tracked markdown (write U+XXXX).
- **Do not author Nova escapes or markdown backslashes through a heredoc.** A quoted heredoc has eaten a backslash level **six times** on this project. Use the Write tool, or a Python rewrite that asserts a match count. The working tree is **CRLF**, so a pattern written with `\n` matches nothing.
- A known Windows async flake fires roughly one run in four — historically `0xc0000005`, but one instance was just an async child exiting non-zero with empty stdout. Re-run, say so, **attribute no cause, fix nothing**, and do not grep for that code as the test of whether it fired.
- Sentence shapes: prefer a roster with **no count**. No bare counts, ordinals or closed worlds over `std`, the runtime, the workspace or the record set. **Never write that a fixture pins something without checking a fixture executes it.**

## Measured facts this plan rests on

Every one was established by writing the construct and running `nova check` / `nova run`, exit code captured. `nova check` exits 1 on error, 0 on ok.

- `parse(s)` then `Int::from_json(v)` converts a `String` to an `Int` — **ran**, printed `42`. This is the route around the absent `parse::<Int>()`.
- `impl FromJson for User` in user code compiles and decodes — **ran**. `JsonError` is `{ msg: String, at: Int }`.
- `stringify(String(s))` returns a quoted, **escaped** JSON string — `"a\"b\\c"`. Interpolating it yields valid JSON.
- `Map<Int, User>` with a `mut self` method mutating through the receiver works — **ran**, `insert`/`get`/`keys`/`len` all behaved.
- `String == String` works; `path.split("/")` yields `[String]`.
- **`Map` iteration order is seeded per process.** The same program emitted `{"name":…,"id":…}` and `{"id":…,"name":…}` across runs. Both `keys()` order and `JsonValue::Object` emission are affected.
- The dense-id walk below produced **one distinct output across six runs**.
- `Response` is `{ status: Int, headers: Map<String,String>, body: Bytes }` with public fields. `Response::text` hardcodes `text/plain`, so this example constructs `Response` directly to set `application/json`.
- `Request` is `{ method: Method, path: String, headers: Map<String,String>, body: Bytes }`. `Method` is `Get | Post | Put | Delete | Patch | Head | Options | Unknown(String)`.
- `Bytes::to_string(self) -> Option<String>`.

## File Structure

- **Create** `examples/05-json-api/src/main.nova` — the server. Sole responsibility: serve the three routes.
- **Create** `examples/05-json-api/README.md` — per `60-EXAMPLES.md` §9's template.
- **Create** `examples/05-json-api/BENCHMARK.md` — the measured figure. §5 names this destination.
- **Modify** `crates/nova-cli/tests/run_tests.rs` — one golden test.
- **Modify** `nova-spec/60-EXAMPLES.md`, `nova-spec/00-MASTER-SPEC.md`, `CHANGELOG.md` — records.

---

### Task 1: The example and its golden test

**Files:**
- Create: `examples/05-json-api/src/main.nova`, `examples/05-json-api/README.md`
- Modify: `crates/nova-cli/tests/run_tests.rs`

**Interfaces:**
- Consumes: `std/http`'s `read_request`, `Limits::default()`, `Request`, `Response`; `std/json`'s `parse`, `stringify`, `JsonValue`, `JsonError`, `FromJson`; `std/net`'s `bind`, `accept`, `local_port`, `write`, `close`; `std/collections`' `Map`; `std/strings`' `split`.
- Produces: an executable example, and a printed line `listening on 127.0.0.1:${port}` whose shape the golden test parses.

- [ ] **Step 1: Write the example**

Create `examples/05-json-api/src/main.nova`. **Use the Write tool** — this file contains Nova string escapes and a heredoc will eat a level.

```nova
// Phase 2's gate example: a JSON API over `std/http`.
//
// **This is not `nova-spec/60-EXAMPLES.md` section 5's listing, and that is
// deliberate.** That listing carries a dated amendment ruling it "written in
// a Nova that does not exist" and leaving it "as the aspiration it always
// was". This serves the same three routes in the language that exists. Every
// substitution is recorded in
// `docs/superpowers/specs/2026-09-10-examples-05-json-api-design.md` section
// 3, with the diagnostic that established it.
//
// **Responses are interpolated strings rather than `Map`-backed
// `JsonValue`s, and that is a correctness requirement before it is an
// optimisation.** `std/collections`' `Map` is seeded per process, so object
// key order varies between runs -- measured, the same two-field object
// emitting both key orders across runs. A golden test over JSON text would
// flake intermittently. Interpolation fixes the order; `stringify` supplies
// the escaping, so a name containing a quote cannot break the document.
//
// **No `Mutex`.** ADR 0009 makes single-threading a correctness requirement,
// so there is nothing for one to protect. Records are heap objects with
// reference semantics under ADR 0005, and `tests/runtime/field_assign.nova`
// pins that a `mut self` write is visible through every alias.
//
// **This server never exits**, for the reason `docs/benchmarks/server.nova`
// gives: `block_on` cannot return while a task is parked, and the accept
// loop parks forever. The benchmark procedure kills it.

record User { id: Int, name: String, email: String }

record Store { users: Map<Int, User>, next_id: Int }

impl Store {
    // `mut self` stores through the receiver pointer, so the write is visible
    // to every alias -- which is how the handlers share one store without a
    // lock.
    fn create(mut self, name: String, email: String) -> User {
        let id = self.next_id
        self.next_id = id + 1
        let u = User { id: id, name: name, email: email }
        self.users.insert(id, u)
        u
    }
}

// One string field out of a decoded object, or a `JsonError` naming it.
fn str_field(m: Map<String, JsonValue>, k: String) -> Result<String, JsonError> {
    match m.get(k) {
        Some(v) => String::from_json(v)
        None => Err(JsonError { msg: "missing field ${k}", at: 0 })
    }
}

// Hand-written because `@derive` does not exist: `E0082` reports `test` as
// the only attribute the compiler knows. The decode direction is the one the
// example needs -- `POST /users` turns a body into a record. There is no
// `impl ToJson for User`, because with responses interpolated nothing would
// call it.
impl FromJson for User {
    fn from_json(v: JsonValue) -> Result<User, JsonError> {
        match v {
            Object(m) => {
                match str_field(m, "name") {
                    Ok(n) => {
                        match str_field(m, "email") {
                            Ok(e) => Ok(User { id: 0, name: n, email: e })
                            Err(er) => Err(er)
                        }
                    }
                    Err(er) => Err(er)
                }
            }
            _ => Err(JsonError { msg: "expected an object", at: 0 })
        }
    }
}

// `stringify(String(s))` quotes AND escapes, so a name carrying a quote or a
// backslash cannot break the document. The key order is fixed by this
// sentence rather than by `Map` iteration.
fn user_json(u: User) -> String {
    "{\"id\":${u.id},\"name\":${stringify(String(u.name))},\"email\":${stringify(String(u.email))}}"
}

// Ids are dense from 1, so walking them ascending is DETERMINISTIC -- which
// `keys()` is not, and which is why this does not use `keys()` and does not
// need the `values()` that `std/collections` lacks. It costs a lookup per id
// ever issued rather than per user held; at this example's scale that is not
// the measured path.
fn users_json(s: Store) -> String {
    let mut out = "["
    let mut id = 1
    let mut first = true
    while id < s.next_id {
        match s.users.get(id) {
            Some(u) => {
                if !first { out = "${out}," }
                out = "${out}${user_json(u)}"
                first = false
            }
            None => {}
        }
        id = id + 1
    }
    "${out}]"
}

// `Response::text` hardcodes `text/plain`, so this builds the record.
fn json_response(status: Int, body: String) -> Response {
    let b = bytes_from_string(body)
    let mut h: Map<String, String> = Map::new()
    h.insert("content-length", "${b.len()}")
    h.insert("content-type", "application/json")
    Response { status: status, headers: h, body: b }
}

fn error_json(msg: String) -> String {
    "{\"error\":${stringify(String(msg))}}"
}

// The absent `parse::<Int>()`, via `std/json`'s public `FromJson for Int`.
fn path_id(seg: String) -> Option<Int> {
    match parse(seg) {
        Ok(v) => {
            match Int::from_json(v) {
                Ok(n) => Some(n)
                Err(e) => None
            }
        }
        Err(e) => None
    }
}

fn handle(req: Request, store: Store) -> Response {
    match req.method {
        Get => {
            if req.path == "/users" {
                json_response(200, users_json(store))
            } else {
                let parts = req.path.split("/")
                if parts.len() == 3 {
                    match path_id(parts[2]) {
                        Some(id) => {
                            match store.users.get(id) {
                                Some(u) => json_response(200, user_json(u))
                                None => json_response(404, error_json("no such user"))
                            }
                        }
                        None => json_response(400, error_json("id must be an integer"))
                    }
                } else {
                    json_response(404, error_json("not found"))
                }
            }
        }
        Post => {
            if req.path == "/users" {
                match req.body.to_string() {
                    Some(s) => {
                        match parse(s) {
                            Ok(v) => {
                                match User::from_json(v) {
                                    Ok(u) => {
                                        let created = store.create(u.name, u.email)
                                        json_response(201, user_json(created))
                                    }
                                    Err(e) => json_response(400, error_json(e.msg))
                                }
                            }
                            Err(e) => json_response(400, error_json(e.msg))
                        }
                    }
                    None => json_response(400, error_json("body is not UTF-8"))
                }
            } else {
                json_response(404, error_json("not found"))
            }
        }
        _ => json_response(404, error_json("not found"))
    }
}

async fn serve(conn: TcpStream, store: Store) {
    while true {
        match read_request(conn, Limits::default()).await {
            Ok(req) => {
                let wire = handle(req, store).to_bytes()
                let total = wire.len()
                match conn.write(wire).await {
                    Ok(n) => {
                        // `std/net`'s `write` is one non-blocking attempt, not
                        // a `write_all` loop, so a short write needs a retry.
                        // Writing `wire` whole first keeps the common case
                        // free of slicing. Unexercised by the suite: a
                        // response this small does not get a short write on
                        // loopback.
                        if n < total {
                            let mut sent = n
                            let mut write_ok = true
                            while sent < total {
                                match conn.write(wire.slice(sent, total)).await {
                                    Ok(m) => {
                                        // No progress must not retry forever
                                        // -- that hangs rather than fails.
                                        if m <= 0 {
                                            write_ok = false
                                            break
                                        }
                                        sent = sent + m
                                    }
                                    Err(e) => {
                                        write_ok = false
                                        break
                                    }
                                }
                            }
                            if !write_ok { break }
                        }
                    }
                    Err(e) => break
                }
            }
            Err(e) => break
        }
    }
    let _ = conn.close().await
}

async fn main() {
    let l = match bind("127.0.0.1:0") {
        Ok(l) => l
        Err(e) => panic("bind: ${e.message}")
    }
    let port = match l.local_port() {
        Ok(p) => p
        Err(e) => panic("local_port: ${e.message}")
    }
    // The one line this server prints. The golden test parses it from stdout,
    // so its shape is load-bearing.
    println("listening on 127.0.0.1:${port}")

    let store = Store { users: Map::new(), next_id: 1 }

    while true {
        match l.accept().await {
            Ok(conn) => {
                let h = spawn(serve(conn, store))
            }
            Err(e) => break
        }
    }
}
```

- [ ] **Step 2: Check it compiles, and run it by hand**

Run: `cargo build --locked --workspace` then `./target/debug/nova check examples/05-json-api/src/main.nova`
Expected: `ok:` and exit 0.

Then start it and drive the three routes with `curl`, which is the only HTTP client on this host:

```bash
./target/debug/nova run examples/05-json-api/src/main.nova
```

Against the printed port: `curl -s localhost:PORT/users`, `curl -s -X POST localhost:PORT/users -d '{"name":"ada","email":"a@example.com"}'`, `curl -s localhost:PORT/users/1`, `curl -s localhost:PORT/users/zz`, `curl -s localhost:PORT/users/99`.

**Record what each returned in the report.** If any differs from Step 3's expectations, the expectations are wrong and must be corrected against observed behaviour rather than the other way round.

- [ ] **Step 3: Write the golden test**

Append to `crates/nova-cli/tests/run_tests.rs`, modelled on the existing `crypto_random_run` and on the benchmark smoke test that parses a port from stdout. A **normal test, not `#[ignore]`d**: CI's Test job runs the ignored tests in an advisory step whose failures are tolerated and unread.

It must assert **status codes and bodies**, never a duration or a rate, so it cannot flake on timing. Drive, in order: `GET /users` on an empty store, `POST /users`, `GET /users/1`, `GET /users/zz`, `GET /users/99`, and `GET /nope`.

Expected bodies, given the interpolation order fixed in Step 1:

- empty list → `[]`
- create → `{"id":1,"name":"ada","email":"a@example.com"}` with status 201
- fetch → the same object with status 200
- bad id → status 400
- missing id → status 404
- unknown path → status 404

- [ ] **Step 4: Run the test and watch it fail first**

Run: `cargo test -p nova-cli --test run_tests --no-fail-fast json_api`
Expected on a deliberately wrong golden: FAIL, showing the diff. **Observe the red before the green** — this project has shipped a fixture whose red phase was never seen, and the review said so.

- [ ] **Step 5: Correct the golden and run to green**

Run the same command. Expected: PASS.

- [ ] **Step 6: Write the README**

`examples/05-json-api/README.md`, following `60-EXAMPLES.md` §9's per-example template. **No existing example satisfies that template** — state that this one follows it without implying the others were brought into line.

- [ ] **Step 7: Full suite, lint, format, byte-scan, commit**

`cargo build --locked --workspace`, then `cargo test --locked --workspace --no-fail-fast` summing every `test result:` line — expect **1125 passed / 0 failed / 8 ignored** (baseline plus one). Then clippy, fmt, and the staged byte scan. Commit with `git commit -F`.

---

### Task 2: The measurement

**Controller-run, not dispatched.** A benchmark needs a release build and a long run; agents that build in this environment are killed by their watchdog.

**Files:** Create `examples/05-json-api/BENCHMARK.md`

- [ ] **Step 1: Build release, both halves**

`cargo build --release --locked --workspace`. `find_runtime_lib` resolves the runtime staticlib **next to the `nova` executable** and nothing pins the profile, so a debug `nova` links a debug runtime and depresses every figure. Use `./target/release/nova`.

- [ ] **Step 2: Build the example standalone and run the generator against it**

`nova build` the example, start it, read the printed port, and drive `crates/nova-bench-http` at it per `docs/benchmarks/README.md`. Take the self-test ceiling in the same session, so the figure has its harness bound beside it.

- [ ] **Step 3: Write `BENCHMARK.md`**

It states: the date; **both axes** — Cranelift backend (the LLVM path cannot run, no `clang`/`llc` on this host) and the release runtime profile; connections, duration, and `errors=` for both runs; the req/sec figure; and the harness ceiling.

It states plainly **whether the figure clears 10k**, and it claims nothing about §5's Bun ratio. If it falls short, name the two recorded suspects — eager header materialisation and quadratic `Bytes::concat` body accumulation — as suspects, not as diagnosis.

Unlike `docs/benchmarks/server.nova`, this example cannot hoist response serialisation, so its figure and that one are **not comparable**; say so.

---

### Task 3: The records

**Files:** Modify `nova-spec/60-EXAMPLES.md`, `nova-spec/00-MASTER-SPEC.md`, `CHANGELOG.md`

- [ ] **Step 1: Amend `60-EXAMPLES.md` §5**

A dated amendment recording that the example now exists, written differently from the listing, and **carrying two corrections to that section's own 2026-09-03 amendment**:

1. **Struct update syntax works.** That amendment does not list it, but `nova-next-increment`-class notes did; verify against the amendment's actual text before claiming it said so, and correct only what it actually says.
2. **A String-to-number conversion IS reachable from user code.** The amendment says it is not. Its evidence is sound — `str_to_float` and `char_to_int` are both inside `Builtin::STD_ONLY`, verified against the array's real bounds — but the conclusion is false: `std/json`'s public `FromJson for Int` reaches the same result, measured. **Do not credit this increment with adding that route.** It existed and went unnoticed.

- [ ] **Step 2: Amend `00-MASTER-SPEC.md` §3**

The gate's status after Task 2's measurement. **Cite by heading, not line number** — that section's numbering has shifted twice and it carries a note saying why a line number is not durable there.

- [ ] **Step 3: `CHANGELOG.md` under `[Unreleased]`**

The example, the golden test, the measured figure, the two corrections, and what is still absent: AEAD-style completeness is not the issue here — name instead the four language features still missing and the `Map::values()` this example routed around.

- [ ] **Step 4: Sweep, byte-scan, commit**

Sweep tracked files for any claim this increment falsifies — that `examples/05-json-api` does not exist, that `BENCHMARK.md` is unsatisfiable, that no example follows §9's template. **Flatten before concluding absence**: strip `///`, `//!`, `//` and `>` gutters, collapse whitespace, then search. Report the patterns used, including for categories that returned nothing.

---

## Self-Review

**1. Spec coverage.** §1's stance (measure, claim nothing) → Task 2 Step 3. §3's blocker table → Task 1 Step 1, every substitution commented at its site. §4's architecture → Task 1 Step 1. §5's interpolation decision → Task 1 Step 1's `user_json`/`users_json` and their comments. §6's error handling → `handle`'s four error paths. §7's measurement → Task 2. §8's records → Task 3. §9's testing → Task 1 Steps 3-5. §10's exclusions → nothing implements them, which is correct. §11's criteria → Tasks 1-3 in order.

**One gap found and closed while writing this:** the spec's §5 justified interpolation by object key order alone, but `keys()` order is unstable too, so a `GET /users` **array** would also vary. The plan closes it with the dense-id walk in `users_json`, which was measured deterministic across six runs where the `Map`-based form was not. The spec is not wrong, but it is narrower than the problem; Task 3 should not restate §5 as if it had covered the array case.

**2. Placeholder scan.** No "TBD", no "handle errors appropriately". Task 1 Step 3's expected bodies are concrete. Task 2's figures are deliberately absent because they are the measurement's output, and Step 3 says exactly what must appear.

**3. Type consistency.** `User { id: Int, name: String, email: String }` and `Store { users: Map<Int, User>, next_id: Int }` are spelled identically in every step. `json_response(status: Int, body: String) -> Response`, `user_json(u: User) -> String`, `users_json(s: Store) -> String`, `path_id(seg: String) -> Option<Int>`, `str_field(m, k) -> Result<String, JsonError>` are used as declared. `JsonError` is constructed as `{ msg, at }`, matching `std/json`.

**One thing this plan does NOT know**, stated rather than guessed: whether `handle` may be a plain `fn` while `serve` is `async fn`. The code above makes it plain, since nothing in it awaits. If the compiler objects, make it `async fn` and `.await` the call — and report which was needed, because the answer belongs in the record.
