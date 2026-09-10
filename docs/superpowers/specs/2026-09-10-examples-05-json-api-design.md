# `examples/05-json-api` — design

**Status:** proposed, 2026-09-10, branch `examples-05-json-api`.

## 1. What this is, and what it is not

Phase 2's gate names one artifact that has never existed:
`00-MASTER-SPEC.md` §3 says "`examples/05-json-api` serves 10k+ req/sec on
benchmark hardware. Document benchmark methodology in `docs/benchmarks/`."
The methodology and one measured figure landed in an earlier increment. The
example did not.

This increment writes it **in the Nova that exists**, measures it with the
harness that already exists, and records the number honestly whatever it is.

**It does not claim the gate.** The stance is the one the gate-benchmark
increment took and recorded: measure, and decline to claim. A figure below
10k is a finding about `std/http`'s cost, not a failure of this increment.

**It changes no compiler code and no `std` module.** Every construct
`60-EXAMPLES.md` §5's listing reaches for and the language lacks has a
public-API route, and section 3 gives each one with the measurement that
established it.

## 2. Why the spec's listing is not the deliverable

`60-EXAMPLES.md` §5 carries a dated 2026-09-03 amendment stating that its
listing "is written in a Nova that does not exist", and ending: "The listing
below stays as written, as the aspiration it always was." That decision
stands and this increment does not reopen it.

So the deliverable is a **working example that serves the same three routes**,
plus a record of how it differs and why. `std/http` set this precedent when it
shipped without the router its own spec section specifies, recorded the
`P0001` that blocks the `Handler` alias, and argued the loss was smaller than
it sounds.

**One claim in that amendment is refuted by this increment**, and section 8
carries the correction rather than burying it: the amendment says "the
language has no String-to-number conversion reachable from user code". The
`str_to_float`-is-`STD_ONLY` half is true. The conclusion is false.

## 3. Every blocker, and the route around it — each measured

Measured by writing the construct and running `nova check`, exit code
captured. `nova check` exits 1 on error and 0 on ok.

| §5 reaches for | Present? | Route taken here | Evidence |
|---|---|---|---|
| `parse::<Int>()` | no | `parse(s)` then `Int::from_json(v)` | **ran**, printed `42` |
| `@derive(ToJson, FromJson, Clone)` | no | hand-written `impl FromJson for User` | **ran**, decoded a posted body |
| `?` | no | explicit `match` | `E0900` on `f()?` |
| turbofish | no | not needed; `Int::from_json(v)` infers | `P0001` on `Map::<K,V>::new()` |
| `Server.get(path, handler)` | no | `match` on `req.method` / `req.path` | `P0001` on the `async fn` alias |
| `users.values()` | no | iterate `keys()`, `get()` each | `E0014: no method values` |
| `User { id, ..user }` | **yes** | used directly | **ran** — this blocker was stale |
| `*next += 1` | `+=` **yes** | `+=` on a `mut` binding | **ran** |

The last two rows matter beyond this increment: `60-EXAMPLES.md` §5 and
`nova-next-increment` both listed struct update syntax among the missing
features. It works.

## 4. Architecture

Modelled on `docs/benchmarks/server.nova`, which is the only working Nova HTTP
server in the tree.

**Task shape is forced, not chosen.** One task accepts; one task serves each
connection. Staging two socket waits in a single poll aborts the process
(`stage_park` in `crates/nova-runtime/src/task.rs`), so the task parked in
`accept` cannot also read a connection.

**Routing** is a `match` over `req.method` and `req.path`. `Request` exposes
`pub method: Method`, `pub path: String`, `pub headers: Map<String, String>`
and `pub body: Bytes`, so nothing about the routing needs a type the language
cannot express. `/users/:id` is a path split, and the trailing segment becomes
an `Int` by the route in section 3.

**State is a plain record, and `Mutex` is not used.** ADR 0009 makes
single-threading a *correctness* requirement — the collector's heap is
thread-local — so the `Mutex<Map<Int, User>>` in §5's listing has nothing to
protect. Records are heap objects with reference semantics under ADR 0005, and
a `mut self` method stores through the receiver pointer so the write is
visible to every alias. That is not inferred: `tests/runtime/field_assign.nova`
pins it, including the alias case, and its own comments state the rule.

**The server never exits.** `block_on` cannot return while a task is parked,
and the accept loop parks forever. The benchmark procedure kills the process;
`docs/benchmarks/server.nova` documents the same property. There is no
shutdown path and its absence is not an oversight.

## 5. Responses are built as strings, and that is a deliberate deviation

§5's listing returns `http.Response.json(...)` over a `Map`-backed
`JsonValue`. This example builds response JSON by interpolation instead. Two
reasons, and the first is a correctness requirement rather than a preference.

**`Map` iteration order is not stable across processes.** `std/collections`'
`Map` is seeded per process — the HashDoS work — so object key order varies
run to run. Measured: the same three-line program emitting one two-field
object produced `{"name":"ada","id":7}` and `{"id":7,"name":"ada"}` across
eight runs, both orders appearing. A golden test over JSON text would
therefore flake intermittently, and intermittently is the worst rate.
Interpolated output has a fixed key order and is safe to pin.

**It is also the cheaper path**, which matters here specifically. The
benchmark's subject is throughput, and unlike `docs/benchmarks/server.nova`
this example cannot hoist its response bytes out of the loop — every response
is freshly built. Avoiding a `Map` insert and a `JsonValue` tree per response
removes allocation from exactly the path the measurement times.

**`ToJson for User` is therefore not implemented at all.** An earlier draft of
this section kept it "because the codec is part of what the example
demonstrates" — but with every response interpolated, nothing would call it,
and an impl no test executes described as exercised is the exact defect this
project keeps finding. The decode direction is different: `FromJson for User`
is genuinely required, because `POST /users` has to turn a request body into a
record, and section 9's golden test drives that path. Measured: a hand-written
`impl FromJson for User` in user code compiles and decodes at runtime, using
`JsonError { msg: String, at: Int }`.

## 6. Error handling

Every fallible step is an explicit `match`, since `?` does not exist. The
routes answer:

- unparseable path id → `400`
- unparseable or non-object request body → `400`
- unknown id on `GET /users/:id` → `404`, as §5 specifies
- unknown method or path → `404`

`std/http`'s `HttpErrorKind` has ten arms and `read_request` returns
`Result<Request, HttpError>`; a read error closes the connection rather than
answering, matching `server.nova`.

**No panic may cross a generated poll boundary.** The poll ABI is frozen. The
example must reach no partial function on any path a request can drive.

## 7. Measurement

Reuse `crates/nova-bench-http`, the dependency-free load generator that
already exists. `docs/benchmarks/README.md` is the procedure.

**Both axes get recorded**, because a req/sec figure without them is close to
meaningless: the backend (Cranelift — the LLVM path cannot run, no `clang` or
`llc` on this host) and the runtime profile. `find_runtime_lib` resolves the
runtime staticlib *next to the `nova` executable* and nothing pins the
profile, so a debug `nova` links a debug runtime and depresses every figure.
The procedure requires `cargo build --release` first.

**Bun 1.3.0 IS installed on this host.** That is worth stating plainly because
§5's own gate is a ratio — "Benchmark vs Bun on same hardware shows ≥ 1.0x
req/sec ratio" — and the gate has two non-equivalent statements: §3's absolute
10k and §5's Bun ratio, which can disagree in either direction. Writing an
equivalent Bun server is real work and this spec does **not** put it in scope;
it records that the ratio is now *measurable* rather than blocked, so a later
increment can take it deliberately instead of inheriting "unmeasured" as if it
were "impossible".

`BENCHMARK.md` states the number, the two axes, the date, and whether it
clears 10k. It claims nothing about the ratio.

## 8. Records to amend

- **`60-EXAMPLES.md` §5** — a dated amendment recording that the example now
  exists, that it is written differently from the listing and why, and
  **correcting the String-to-number claim**: `str_to_float` is indeed
  `STD_ONLY`, and `char_to_int` is too, but `std/json`'s public
  `FromJson for Int` reaches the same result from user code. The amendment
  must not credit this increment with adding that route — it existed already
  and nobody had noticed.
- **`60-EXAMPLES.md` §5, again** — struct update syntax works, and that
  amendment lists it among the missing features. Note this is the *tracked*
  record; a stale note in the controller's own memory is not a repository
  concern and is not this increment's work.
- **`00-MASTER-SPEC.md` §3** — the gate's status after this measurement,
  cited by heading rather than line number, since that section's numbering
  has shifted twice.
- **`CHANGELOG.md`** under `[Unreleased]`.
- **`60-EXAMPLES.md` §9's README template** — this example follows it, and no
  existing example does. The amendment should say that rather than implying
  the others were brought into line.

## 9. Testing

One golden test in `crates/nova-cli/tests/run_tests.rs`, a normal test and not
`#[ignore]`d: CI's Test job runs the ignored tests in an advisory step whose
failures are tolerated and unread, so a test placed there would run unwatched.

It pins **correctness, not throughput**, and asserts no duration and no rate:
start the server, drive the three routes, assert the status codes and the
response bodies. The bodies are pinnable only because of section 5's
interpolation decision. Asserting no duration is not the same as having no
timing dependency, though: the test installs a ten-second read and write
timeout on each socket, so a stalled peer fails it rather than parking the
suite, and a red there reads as a stall rather than as a slow machine.

**The full load run is never executed by CI.**

## 10. What is not covered

- No compiler feature. The four language gaps stay open.
- No `std` change. `Map::values()` stays absent; this example walks ids
  ascending from 1 and looks each one up instead, which is what makes its list
  output deterministic — `keys()` is seeded per process and is not used here.
- No optimisation of `std/http`'s two recorded costs — eager header
  materialisation and quadratic `Bytes::concat` body accumulation. If the
  measured number lands under 10k, those are the named suspects and
  investigating them is a separate increment.
- No Bun server and no ratio.
- The numbering drift between `03-http-server` in the spec tree and
  `03-producer-consumer` on disk stays recorded and untouched.

## 11. Success criteria

1. `examples/05-json-api/src/main.nova` compiles and serves the three routes.
2. A golden test pins all three, and passes on ubuntu, windows and macOS.
3. `BENCHMARK.md` carries a measured figure with both axes and a date.
4. Every record in section 8 is amended, including the two corrections.
5. The suite stays green and no existing test changes meaning.
6. Whatever the number is, no record claims the gate is passed unless the
   measurement actually supports it — and if it does, the §5 ratio half is
   still explicitly unmeasured.
