# `std/http` request parsing — design

**Status:** approved 2026-09-01. Implements the server half of `nova-spec/20-STDLIB.md` §6, narrowed; see §1 and §10.

**Increment:** the first half of `std/http`, which is module group 10 of the
thirteen in `nova-spec/00-MASTER-SPEC.md` §3's Phase 2 list and one of the two
still missing. The other, `std/crypto`, is untouched here and nothing in the
Phase 2 gate depends on it.

---

## 1. What this closes, and the three things it does not

Phase 2's gate is `examples/05-json-api` serving 10k+ req/sec with methodology
in `docs/benchmarks/`. Nothing in that chain exists: `std/http` is absent, so
`03-http-server`, `04-todo-cli` and `05-json-api` cannot be written, so the gate
cannot be measured. This increment supplies the part everything else waits on —
**an HTTP/1.1 server that parses a request and writes a response.**

Deliberately not here:

- **The router.** `nova-spec/20-STDLIB.md` §6 specifies
  `Server::new().get(path, handler)` with `pub type Handler = async fn(Request) -> Response`.
  That type alias does not parse — measured, `P0001: expected type (in type alias), found async`.
  Deferred until the language can name an async function type. §3 explains why
  the loop-based shape loses nothing in the meantime.
- **The client.** `get`, `post`, `Response::json`, `HttpError` in that same
  section. A separate problem with no gate dependency.
- **HTTPS, HTTP/2, chunked transfer-encoding, and request pipelining.**
  Keep-alive *is* in scope: the gate is a throughput number and reconnecting per
  request would dominate it.

---

## 2. Why the spec's stated strategy is unavailable, measured rather than argued

`nova-spec/00-MASTER-SPEC.md` §3 says of this module group: "server first, then
client; **use hyper internals at runtime layer**". Hyper driving the server is
blocked by three independent properties of this runtime, each checked against
the tree rather than recalled:

1. **The executor cannot be re-entered.** `crates/nova-runtime/src/task.rs`
   guards `IN_BLOCK_ON` and calls `abort_with`, and
   `run_aborts_when_an_async_fn_calls_block_on` pins it. If hyper drove the
   server and called back into a Nova handler, a handler that awaits would have
   to suspend inside a hyper future — re-entering the executor, which aborts.
2. **There are no wakers.** `task.rs`'s module docs state that a deadline and
   another task's completion are "the executor's only two wake sources", and
   that they are scheduled by the executor "not registered as an arbitrary
   callback the awaited resource invokes". Hyper's connection driver expects to
   register a waker. There is nothing to register with.
3. **Hyper cannot have its own thread.** ADR 0009 makes single-threading a
   correctness requirement, not a simplification: the collector's heap lives in
   a `thread_local!`, so a second thread running Nova code frees objects the
   first still holds.

Any one of these is separately fixable. Together they mean "hyper drives the
server" requires rebuilding the executor around wakers and would touch the
frozen poll ABI — a larger increment than this one, and a different one.

**What survives is the wording's own distinction.** The spec says hyper's
*internals*, and hyper's HTTP/1 parsing is `httparse`: a standalone,
allocation-free parser with no async and no runtime. This increment takes that
and leaves hyper's runtime alone.

---

## 3. The design

**The server is a Nova-side accept loop over `std/net`, and parsing is one
intrinsic.**

`std/net` already establishes the shape — `bind` is synchronous,
`TcpListener::accept` is async, and there are no callbacks anywhere. A server in
that style needs no `Handler` type, which is what makes the router's absence
cost nothing today: **handler code sits inside the accept loop, which is already
an async context, so it can `await` freely.** The router would have *added* a
constraint, not removed one.

Verified against the compiler before relying on any of it:

| construct | status |
|---|---|
| `fn(Int) -> Int` as a parameter type, called | compiles **and runs** |
| closures, including capture of an enclosing binding | compile **and run** |
| `Result<(), E>` with `Ok(())` | compiles |
| `pub type Method = \| Get \| Post` sum type | compiles |
| `[u8]` | `E0001: cannot find type u8` — the byte type is `Bytes` |
| `pub type H = async fn(Request) -> Response` | `P0001` |

The first two are stronger than expected and are what make a future router
possible without a compiler change, provided handlers are synchronous. They are
recorded here because the spec's §6 assumes the opposite.

---

## 4. The parse intrinsic, and why it is one rather than seven

```nova
http_parse_request(buf: Bytes) -> [Int]
```

Returns a flat table of **offsets into the caller's own buffer**. It copies
nothing, allocates nothing on the Rust side, and holds no state between calls.

The obvious alternative was the `File` pattern — `File { fd: Int }` over a
`static FILES: RefCell<HashMap<i64, std::fs::File>>` — reached through the
established "call, then take" idiom that `File::open` uses (`file_open` returns
a status, `fs_take_bytes()` retrieves the payload, Nova assembles the record).
Applied here that needs roughly seven intrinsics and **two FFI crossings per
header**, and it inherits `File`'s defining hazard: Nova has no destructors, so
a forgotten release leaks. A file handle leaking is a bug; **a per-request table
entry leaking is unbounded growth under exactly the load the gate measures.**

One intrinsic costs the twelve-site checklist in ADR 0018 §3 once. For
scale, `std/net` spent eight builtins — `NetAccept`, `NetClose`, `NetConnect`,
`NetListen`, `NetLocalPort`, `NetRead`, `NetReadTimeout`, `NetWrite` — and this
spends one. (An earlier draft of this paragraph said twenty-four, from a grep
that counted occurrences rather than builtins: each variant appears three times
in the resolver, which is three of its twelve sites.)

**Response serialisation adds no intrinsic at all.** A response is
`"HTTP/1.1 200 OK\r\n" + headers + "\r\n\r\n" + body`, which Nova builds with
`String` interpolation, `bytes_from_string` and `Bytes::concat`.

`[Int]` is a proven return type for an intrinsic: `bytes_to_ints` already
returns one.

### 4.1 The encoding, stated exactly

The returned array is:

```
[0]  status
[1]  method_start   [2]  method_len
[3]  path_start     [4]  path_len
[5]  minor_version                     (0 or 1, for HTTP/1.0 vs 1.1)
[6]  header_count   = n
[7 .. 7+4n)         four Ints per header, in wire order:
                      name_start, name_len, value_start, value_len
[7+4n]              body_start
```

`status` is `0` for a complete head, `1` for a partial one (the caller must read
more bytes and call again), and **negative for an error**, with the value being
the negation of an error kind so the set can grow without changing the shape.
When `status` is not `0`, **the array has length 1** and carries nothing else —
so a caller that forgets to check `status` indexes out of bounds rather than
reading a plausible-looking offset. That is deliberate.

All offsets are byte offsets from the start of `buf`. `body_start` is the offset
just past the terminating CRLF CRLF; the body's *length* is not in the table,
because it comes from `Content-Length`, which is a header the caller must read
and validate anyway.

**This is a small binary protocol between Rust and Nova, and it is the part most
likely to fail silently.** A disagreement about field order yields plausible
garbage, not an error. §8 requires a fixture that pins the exact array for a
known request.

---

## 5. The Nova API

```nova
pub type Method = | Get | Post | Put | Delete | Patch | Head | Options | Other

pub record Request {
    pub method: Method
    pub path: String
    pub headers: Map<String, String>
    pub body: Bytes
}

pub record Response {
    pub status: Int
    pub headers: Map<String, String>
    pub body: Bytes
}

pub record Limits {
    pub max_head_bytes: Int
    pub max_header_count: Int
    pub max_body_bytes: Int
}

impl Default for Limits { /* the defaults tabulated in section 6 */ }

impl Response {
    pub fn ok(body: Bytes) -> Response
    pub fn text(status: Int, s: String) -> Response
    pub fn not_found() -> Response
    pub fn to_bytes(self) -> Bytes
}

// Read one request from a connection, honouring keep-alive: the caller loops.
pub async fn read_request(conn: TcpStream, limits: Limits) -> Result<Request, HttpError>
pub async fn write_response(conn: TcpStream, resp: Response) -> Result<(), IoError>
```

`Method` gains an `Other` arm the spec does not have, because a request may
carry any token and the parser must not abort on one it does not know.

`headers` is `Map<String, String>`, and **that inherits a security property this
project acquired days ago**: `impl Hash for String` is seeded per process, so a
`Map` keyed on attacker-supplied header names resists a precomputed collision
set. Header parsing is the canonical HashDoS vector, and the mitigation is
already in place rather than owed. Header names are lower-cased on insert, since
HTTP field names are case-insensitive; the original casing is not preserved,
which §10 records as a limitation.

Usage is an ordinary loop, and the handler is just its body:

```nova
async fn serve_one(conn: TcpStream) {
    // Keep-alive: one connection, requests until the peer stops sending.
    while true {
        match read_request(conn, Limits::default()).await {
            Ok(req) => {
                // The handler is just this block, and it is already async,
                // so it may await anything.
                let resp = Response::text(200, "hello ${req.path}")
                if write_response(conn, resp).await.is_err() { return }
            }
            Err(e) => { let _ = write_response(conn, Response::text(400, "bad request")).await
                        return }
        }
    }
}

fn main() {
    block_on(async {
        let listener = bind("127.0.0.1:8080").unwrap()
        while true {
            let conn = listener.accept().await.unwrap()
            spawn(serve_one(conn))
        }
    })
}
```

The sketch is illustrative rather than checked: it is written against the API
this document proposes, which does not exist yet, so its syntax has not been
compiled. The constructs it leans on — `while`, `match`, `spawn`, `await`,
string interpolation — are each in use elsewhere in `std`.

---

## 6. Limits, and what a malformed request does

Every limit is checked in Rust during parsing, before any Nova allocation, and
each has a distinct error kind rather than an abort:

| limit | default | why |
|---|---|---|
| max request-head bytes | 8 KiB | bounds the buffer a client can force before the head completes |
| max header count | 100 | bounds the offset table and the `Map` |
| max body bytes | 1 MiB | bounds one allocation |

**No limit may be enforced by panicking.** The parse intrinsic runs on a
compiled Nova frame's call stack, and `crate::task::PollFn`'s doc reserves the
`"C-unwind"` permission for the runtime's Rust-side entry points rather than for
compiled Nova frames. Following the precedent set by `nova_rt_int_hash_seed`,
this intrinsic is **plain `extern "C"`**, so any panic escalates to an abort
rather than unwinding through a frame with no landing pads — and the design's
job is to ensure there is no panic to escalate. Every fallible step returns a
status.

---

## 7. The cost, honestly

Measured inputs: an FFI crossing is ~15 ns, a GC allocation ~900 ns.

Per request with ten headers, eager materialisation costs roughly **20 GC
allocations for the header strings alone, about 18 µs** — so at 10k req/sec,
**about 18% of one core before parsing, I/O, or the handler.** Slicing is minor
by comparison: ~40 crossings ≈ 600 ns.

That is a real risk to the gate and is stated as one. Two things make eager
materialisation the right start anyway:

- The figure is measured rather than assumed, so the first benchmark will
  confirm or refute it directly.
- **The escape hatch does not disturb the intrinsic.** If headers dominate, Nova
  keeps the offset table and materialises a header only when looked up. The
  parse intrinsic, its encoding, and the wire behaviour are unchanged; only
  `Request`'s internals move.

No claim is made here that 10k req/sec is reached. This increment makes it
**measurable**, which it currently is not.

---

## 8. Testing, and the mutations that must fail

- A fixture pinning the **exact offset array** for a known request, field by
  field. Without it a reordering of the encoding is invisible.
- Round-trip: parse a request, serialise a response, compare bytes to a golden.
- Partial input: a head split across two reads must return `status = 1` on the
  first call and `0` on the second, with identical offsets to the unsplit case.
- Malformed input for each error kind, asserting the negative status and that
  the array has length 1.
- Each limit at its boundary and one past it.
- Keep-alive: two requests on one connection, both parsed.

Mutations that must fail, run and reported rather than predicted:

1. Swap `name_start` and `value_start` in the encoding — the offset fixture must
   fail. If only the round-trip test fails, the fixture is not pinning what it
   claims.
2. Drop the header-count limit — the limit test must fail.
3. Return `status = 0` on partial input — the split-read test must fail.
4. Skip lower-casing a header name — the case-insensitivity test must fail.

---

## 9. Records to amend

- **`nova-spec/20-STDLIB.md` §6** is written in a Nova that does not exist:
  `[u8]` is `E0001` and the `Handler` alias is `P0001`. It needs a dated
  amendment recording what v1 ships, not a rewrite of the body.
- **`nova-spec/00-MASTER-SPEC.md` §3**, whose Phase 2 list says "use hyper
  internals at runtime layer" — narrow it to the parsing internals, with §2's
  three blockers as the reason.
- **`docs/adr/`** — a new ADR for the offset-table boundary. It is the first
  intrinsic in this project to return a structured table rather than a scalar or
  a single value, and the next such intrinsic should find the reasoning.
- **`CHANGELOG.md`** under `[Unreleased]`.
- **The example-numbering drift**, which is not this increment's to fix but is
  its to record: `00-MASTER-SPEC.md` §3's tree names `03-http-server`, while
  `examples/` holds `03-producer-consumer`. `60-EXAMPLES.md` §9 also specifies a
  per-example README that `03-producer-consumer` does not have.

---

## 10. Out of scope, and known limitations

- The router, the client, HTTPS, HTTP/2, chunked encoding, pipelining — §1.
- **Original header casing is not preserved.** Names are lower-cased on insert.
- **`Content-Length` only.** A request without one is treated as bodiless.
- **No `Expect: 100-continue` handling.**
- `std/crypto`, the other missing Phase 2 module group.

---

## 11. Success criteria

1. `http_parse_request` ships as one intrinsic with all twelve ADR 0018 sites,
   verified by `cargo check --all-targets` rather than by a plain build — one
   forced site lives in `nova-typeck`'s `#[cfg(test)]` module, so a plain
   workspace check finds six of seven and reports success.
2. The offset-array fixture passes and mutation 1 fails it.
3. A Nova program accepts a connection, parses a request, writes a response, and
   serves a second request on the same connection.
4. Every limit has a test at its boundary and one past it.
5. The suite is green on all three platforms, with the gate step's counts summed
   per platform and split by step.
6. `httparse` is the only new dependency, and `Cargo.lock` changes only by its
   addition.
