# 05-json-api

A JSON API over `std/http`: list users, create one, fetch one by id.

## What this demonstrates

- Routing without a router type: a `match` over `req.method` and a
  `req.path.split("/")`. `nova-spec/20-STDLIB.md`'s own
  `pub type Handler = async fn(Request) -> Response` does not parse today, so
  there is nothing here for `Server.get`/`.post` to be built on.
- Decoding a request body into a record by hand -- `impl FromJson for User`,
  written out rather than derived. `@derive` is not implemented: an unknown
  attribute is `E0082`, and the message lists what the resolver's
  `KNOWN_ATTRIBUTES` actually holds, which is `test`.
- Turning a path segment into an `Int` through `std/json`: `parse(seg)` and
  then `Int::from_json(v)`. There is no `parse::<Int>()` and no turbofish, and
  `str_to_float` is `STD_ONLY`, so this is the route through `std`. A program
  can also walk `String::chars()` and fold digits by hand -- `std/json`'s own
  `hex_digit` is that shape in the other direction -- which is more code and
  has to repeat the range check `Int::from_json` already makes.
- Shared mutable state with no lock: a plain `Store` record and a `mut self`
  method. ADR 0009 makes single-threading a correctness requirement, so there
  is nothing for a `Mutex` to protect, and ADR 0005 gives records reference
  semantics, so the insert is visible through every alias -- including the
  next connection's `serve` task.
- Escaping through `stringify(String(s))` rather than by hand, so a name
  carrying a quote or a backslash cannot break the response document.
- One task accepting and one task per connection, which `stage_park` forces
  rather than the design choosing: staging two socket waits in a single poll
  aborts the process, so the task parked in `accept` cannot also read a
  connection.

## Run it

```bash
cd examples/05-json-api
nova run
```

## Expected output

The server prints one line and then serves until it is killed. The port is
whatever the OS hands out, so it differs from run to run:

```
listening on 127.0.0.1:50620
```

Driven against that port -- transcript trimmed to each status line and body:

```
$ curl -s -i localhost:PORT/users
HTTP/1.1 200 OK
[]

$ curl -s -i -X POST localhost:PORT/users -d '{"name":"ada","email":"a@example.com"}'
HTTP/1.1 201 Created
{"id":1,"name":"ada","email":"a@example.com"}

$ curl -s -i localhost:PORT/users/1
HTTP/1.1 200 OK
{"id":1,"name":"ada","email":"a@example.com"}

$ curl -s -i localhost:PORT/users/zz
HTTP/1.1 400 Bad Request
{"error":"id must be an integer"}

$ curl -s -i localhost:PORT/users/99
HTTP/1.1 404 Not Found
{"error":"no such user"}

$ curl -s -i localhost:PORT/nope
HTTP/1.1 404 Not Found
{"error":"not found"}
```

## Notes

**This README follows the per-example template in `nova-spec/60-EXAMPLES.md`
section 9.** Writing it changed nothing under any other example folder, so
whatever section 9's dated record says about the folders it names still says
it.

**This example is not section 5's listing.** That listing carries a dated
amendment ruling it "written in a Nova that does not exist" and leaving it
"as the aspiration it always was". This serves the same routes in the
language that exists. Each substitution is recorded, with the diagnostic that
established it, in
`docs/superpowers/specs/2026-09-10-examples-05-json-api-design.md` section 3.

**Response bodies are interpolated strings rather than `Map`-backed
`JsonValue`s, and that is a correctness requirement before it is an
optimisation.** `std/collections`' `Map` is seeded per process, so the key
order of an object built through it varies between runs -- measured, the same
two-field object emitting both orders. A test over JSON text would flake
intermittently. Interpolation fixes the order; `stringify` supplies the
escaping.

**The response HEADERS are still `Map`-ordered, and their order is not
fixed.** `json_response` inserts `content-length` before `content-type`, and a
hand run emitted them the other way round. Anything reading this server's
responses should look headers up by name rather than by position.

**This server never exits.** `block_on` cannot return while a task is parked
and the accept loop parks forever, so there is no shutdown path; a caller
kills the process. Its absence is not an oversight -- `docs/benchmarks/server.nova`
records the same constraint.

**What is checked automatically.**
`json_api_example_serves_its_routes` in `crates/nova-cli/tests/run_tests.rs`
spawns this example, parses the port out of that first line, and asserts a
status code and a whole response body for each exchange in the transcript
above, plus a `POST` whose name carries a quote and a backslash and then a
second `GET /users`, which comes back as
`[{"id":1,...},{"id":2,...}]` -- the only exchange that drives `users_json`'s
loop body, since the `[]` above it comes out of a loop that never enters.

It asserts no duration and no rate. That is not the same as being immune to
timing: it puts a ten-second read and write timeout on each socket, so a
stalled peer fails the test instead of parking the suite. A red there means a
stall, not a slow machine.

The short-write retry loop in `serve` is not driven by it, because a response
this small does not get a short write on loopback -- that loop is written for
correctness rather than pinned by a fixture. It is not the only arm the test
leaves undriven, and this paragraph is not the list of them; deleting a line
and re-running the test is what settles any particular one.
