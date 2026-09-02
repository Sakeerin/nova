# ADR 0019 — The offset-table intrinsic boundary for `std/http`, and why hyper does not drive the server

**Numbering:** confirmed against `docs/adr/`'s actual contents rather than
trusted from the plan — `0001` through `0018` all exist with no gap, so
`0019` is next.

## Status

Accepted (2026-09-01). The `std/http` request-parsing increment, branch
`std-http-parsing`
(`docs/superpowers/specs/2026-09-01-std-http-request-parsing-design.md`).

## Context

`std/http` is Phase 2 position 10 of the thirteen listed in
`nova-spec/00-MASTER-SPEC.md` §3, and going into this increment it was one of
two module groups with nothing built — no `std/http/`, no HTTP-shaped
dependency in `Cargo.lock`. The other, `std/crypto` at position 12, is
untouched here and nothing in the Phase 2 gate depends on it. Position 11,
`std/json`, already shipped ahead of position 10
(`docs/adr/0018-std-json-scope-and-build-order.md`); this increment does not
reopen that ordering, it fills the gap ADR 0018 left recorded as "unstarted
and unblocked."

The gate itself is `examples/05-json-api` serving 10k+ req/sec
(`nova-spec/00-MASTER-SPEC.md:245`). Nothing in that chain exists yet — the
example isn't written and `docs/benchmarks/` doesn't exist — because the
piece everything else waits on, an HTTP/1.1 server that can parse a request
and write a response, did not exist either. This increment supplies that
piece: request-head parsing, backed by one runtime intrinsic, and response
serialisation, backed by none.

Two design questions had to be answered before any Nova code could be
written, and both are architectural rather than local to `std/http`: how a
parsed request head crosses the Rust/Nova boundary, and whether the
project's own stated plan for that boundary — hyper — is actually available
on this runtime. Neither answer is obvious from the spec text alone; both
were measured against this tree.

## Decision

### 1. One intrinsic, and the table it returns

```
http_parse_request(buf: Bytes) -> [Int]
```

is the only new intrinsic this increment adds. It parses an HTTP/1.1 request
head out of a caller-owned buffer and returns a flat table of **byte offsets
into that same buffer** — no bytes copied, nothing allocated on the Rust
side beyond the returned array, and no state held between calls. The
encoding, stated exactly (`crates/nova-runtime/src/http.rs`,
`nova_rt_http_parse_request`'s own doc comment; pinned from the Rust side by
`parses_a_request_with_two_headers` and from the Nova side by
`tests/runtime/http_offsets.nova`):

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

`status` is `0` for a complete head, `1` for a partial one (the caller reads
more bytes and calls again), and negative for an error — the value is the
negation of an error kind, so the set of kinds can grow without changing the
table's shape. **On any non-zero status the array has length 1** and carries
nothing else. That is deliberate: a caller who forgets to check `status`
before reading an offset indexes out of bounds and fails loudly, rather than
reading whatever the last field of a differently-shaped table happens to
hold and treating it as a plausible offset. All offsets are byte offsets
from the start of `buf`; `body_start` points just past the terminating
CRLF CRLF, and the body's *length* is deliberately not in the table — it
comes from `Content-Length`, a header the caller has to read and validate
against its own limits regardless, so the table does not duplicate that
work.

This is the first intrinsic in the project to hand back a structured table
rather than a scalar or a single value, and the properties above are what a
future one in the same shape should copy: offsets into the caller's own
buffer rather than copies, so nothing has to be freed; no state held on the
Rust side between calls, so there is nothing to leak; a status word first,
checked before anything else in the table is read; and a length-1 array on
any non-zero status, so a caller that skips the check fails immediately
rather than reading garbage that happens to look like an offset. Whether
this is the *only* other intrinsic shaped this way is left for a reader to
check against the current `Builtin` roster rather than asserted here as a
closed count; the allocation choice in §3 below matches one specific
sibling, `nova_rt_bytes_to_ints`, which is a narrower and checkable claim.

### 2. Why one intrinsic, and not a handle table over a `static` map

The established shape for handing a Rust-side resource to Nova already
exists: `File`'s "call, then take" idiom
(`docs/adr/0012-file-descriptor-lifecycle.md`) — one call opens a resource
and returns a status, a second call retrieves a payload, and Nova assembles
the record from what comes back. Applied to a request head, that shape
needs roughly seven intrinsics — open, an accessor each for the method, the
path and the version, a header-count accessor, a per-header accessor, and a
release — and **two FFI crossings per header** rather than the single
crossing this design spends on the whole request. It also inherits `File`'s
defining hazard: **Nova has no destructors**, so a value that goes out of
scope without an explicit release leaves its Rust-side entry exactly where
it was. For a `File` that is an accepted, bounded cost — a program that
forgets to close a handle leaks one entry, and `docs/adr/0012` already
prices that in. A per-request table entry is not bounded the same way: it
would leak at the request rate, on the exact throughput path
`examples/05-json-api`'s 10k+ req/sec gate measures, so a caller who forgets
one release does not accumulate a fixed cost, they accumulate a growing one
under load — the shape of hazard the gate exists to catch, not the shape of
hazard `File`'s bound already tolerates.

The offset table sidesteps this rather than mitigating it: it holds no
Rust-side state at all, so there is nothing to release and therefore nothing
to leak. This is not a general answer to "how does a builtin return a
record" — it is available here specifically because every piece of data the
caller wants is already sitting in a buffer the caller already owns, and the
intrinsic only has to say where.

### 3. The offset block is allocated scanned, matching `nova_rt_bytes_to_ints`

Both allocations in `http.rs` — `status_only`'s one-word error block and the
full table — call `crate::gc::alloc(_, true)`, the same `true` ("scanned")
flag `nova_rt_bytes_to_ints` (`crates/nova-runtime/src/bytes.rs`) uses for
its own `[Int]` block. Looked at in isolation, an offset table holds no
pointers — every element is a plain integer — so an unscanned allocation
would be a cheaper, still-correct choice for this one intrinsic. It was not
made. Every other `[Int]`-shaped array the runtime allocates is scanned, and
picking a different flag for this one would be a decision about the
collector's tagging convention for arrays in general, not a decision about
HTTP parsing — a change worth making on its own measurement and its own
review if it is worth making at all, not one to smuggle in through a single
intrinsic that happens not to need it.

### 4. Why hyper does not drive the server

`nova-spec/00-MASTER-SPEC.md`'s Phase 2 list named this position "server
first, then client; use hyper internals at runtime layer"
(`:240`, narrowed by this increment's own record amendment — see
References). Hyper driving the server — accepting connections, holding the
request/response lifecycle, calling back into a Nova handler — is blocked by
three independent, separately measured properties of this runtime:

1. **The executor cannot be re-entered.** `crates/nova-runtime/src/task.rs`
   guards a thread-local `IN_BLOCK_ON` and calls `abort_with` if it is
   already set when a block is entered; `run_aborts_when_an_async_fn_calls_
   block_on` pins that behaviour with a running test. If hyper owned the
   accept loop and drove connections itself, a Nova handler that `.await`s
   would have to suspend inside one of hyper's own futures — re-entering
   this executor from inside hyper's poll, which aborts the process rather
   than suspending.
2. **There are no wakers.** `task.rs`'s own module documentation names a
   deadline and another task's completion as "the executor's only two wake
   sources," both "scheduled by the executor itself, not registered as an
   arbitrary callback the awaited resource invokes." Hyper's connection
   driver is built around registering a waker with whatever it is polling
   and being woken back up by that resource; there is no mechanism here to
   register one with.
3. **Hyper cannot have its own thread.** `docs/adr/0009-async-execution-model.md`
   makes single-threading a correctness requirement, not merely a
   simplification: the collector's heap is a `thread_local!`, so an object
   allocated on one thread is invisible to any other thread's collector and
   can be freed out from under a thread that still holds it live. A second
   thread running hyper's own runtime is exactly the case that breaks.

Any one of these is separately fixable. Together, making hyper drive the
server means rebuilding the executor around wakers and touching the frozen
poll ABI those wake sources are defined against — a larger increment than
this one, and a different one.

**What survives is the master spec's own wording.** It names hyper's
*internals*, and hyper's HTTP/1 internals are `httparse`: a standalone,
allocation-free parser with no async machinery and no runtime of its own.
This increment takes exactly that — `httparse` 1.10.1, with no dependencies
of its own (`Cargo.lock`'s new package block carries no `dependencies`
list) — and leaves hyper's own executor, connection driver and thread
model untouched, because none of the three is usable here.

### 5. The costs, recorded as risks rather than smoothed over

**Eager header materialisation.** Every header is turned into a `String`
pair on every request, immediately, whether or not the handler ever looks
at it. At this project's own measured ~900 ns per GC allocation, ten headers
cost roughly 20 allocations (a name and a value each) — about 18 µs per
request, which is roughly 18% of one core at 10k req/sec, **before**
parsing, I/O or the handler run at all. The design spec
(§7) records this and names the escape hatch, and it is worth restating
because it bears on this ADR's own §1: the escape hatch does not touch the
intrinsic. If eager materialisation turns out to dominate, `Request` can
keep the offset table instead of a `Map` and materialise a header only when
one is looked up — the intrinsic, its encoding and the wire behaviour are
unchanged; only `Request`'s internals move. No claim is made that the
10k req/sec gate is reached by this increment. It makes the number
**measurable**, which it was not before.

**Body accumulation is quadratic, and the design spec does not mention it —
recorded here so the cost picture it gives does not read as complete when it
is not.** `read_request`'s body loop (`std/http/lib.nova`) accumulates
what it has read with `Bytes::concat`, which copies everything accumulated
so far plus the new chunk on every call. At the 1 MiB `max_body_bytes`
default, reading in 4096-byte chunks takes up to 256 concatenations and
copies roughly 134,742,016 bytes to assemble 1,048,576 — about a 128x
amplification. `Read::read` may legally hand back fewer bytes than asked
for, so a peer that sends the same body in smaller pieces makes this worse,
not better: at 1024-byte reads, up to 1024 concatenations copy roughly
537,395,200 bytes, about 512x. This is bounded, since `max_body_bytes`
bounds it, but it is real work amplification on attacker-controlled input,
sitting on the same request path the throughput gate measures. No loop
restructuring, no different read size and no intrinsic is part of this
change; it is recorded, in full, at `read_request`'s own doc comment, and
recorded here as well because a reader who reads only the design spec's
cost section would otherwise conclude eager header materialisation is the
whole of the cost picture.

### 6. `Map<String, String>` inherits the per-process seeded string hash

`Request.headers` and `Response.headers` are `Map<String, String>`. Header
parsing is the canonical HashDoS vector — an attacker who controls the
header names controls what a naive hash table buckets them by — and the
mitigation is not new work this increment owed. `impl Hash for String` is
seeded once per process and finalised
(`docs/adr/0005-mutable-receivers-and-one-shot-hash.md` and its later
amendments), so a `Map` keyed by attacker-supplied header names resists a
precomputed collision set built by an attacker who has not observed this
process's own seed. That property is cited here, not re-derived: this
increment adds nothing to the mechanism and takes no new position on the
exclusions ADR 0005 already states — not claimed against an adversary who
can observe timing and adapt, and not claimed as cryptographic.

### 7. Three rulings, each with its reason

- **`Limits`' two head-related fields may only tighten the runtime's
  compiled-in walls (8 KiB, 100 headers), never loosen them.** The intrinsic
  takes exactly one argument, the buffer, so there is nowhere for a caller's
  own limit to reach across the FFI boundary — the runtime's ceiling is
  checked unconditionally before anything is allocated, and `std/http`'s
  `Limits` can only add a stricter check on the Nova side afterward.
  `max_body_bytes` has no runtime counterpart at all: the intrinsic never
  looks at the body, so that field is Nova-side alone.
- **`Method`'s catch-all is `Unknown(String)`, not the design spec's
  `Other`.** `std/io` already exports an `Other` variant
  (`IoErrorKind::Other`), every `STD_MODULES` entry is glob-imported into
  every other module, and whether two std modules may export one variant
  name under that scheme is not established anywhere on this tree. This
  increment is not the one that finds out; it names the arm `Unknown`
  instead, and keeps the raw method token as that arm's payload rather than
  discarding it, since a request may carry any token a peer invents and the
  parser must not abort on one it does not recognise.
- **The offset block is allocated scanned, matching `nova_rt_bytes_to_ints`**
  — the reasoning is §3 above; named again here because it is a ruling of
  the same kind as the other two, made once and meant to hold rather than
  be re-litigated per intrinsic.

## Consequences

- **Phase 2 position 10 is no longer wholly unstarted.** The server half —
  parsing a request head and serialising a response over `std/net` — ships.
  Still absent, all recorded in `nova-spec/20-STDLIB.md` §6's own dated
  amendment: the router, the client, HTTPS, HTTP/2, chunked
  transfer-encoding, and request pipelining (a pipelined second request is
  silently consumed and the connection deadlocks, which is a sharper claim
  than "unsupported").
- **Phase 2 is not complete, and this increment does not close it.**
  `examples/05-json-api` and `docs/benchmarks/` still do not exist, and
  position 12 `std/crypto` is the one Phase 2 module group this tree still
  has not started. Nothing here claims the 10k+ req/sec gate is reached;
  §5 above records two open costs against it instead.
- **Counts.** `STD_MODULES` **13 → 14** (`$std.http`, the array's last
  entry); `Builtin::STD_ONLY` **70 → 71** (`http_parse_request`);
  `RESERVED_TYPE_NAMES` unchanged at **7** — `Method`, `Request`,
  `Response`, `Limits` and `HttpError` are ordinary glob-imported
  `std/http` items, shadowable, the same standing `TcpStream` and `File`
  already have. One new Cargo dependency, `httparse` 1.10.1, with no
  dependencies of its own; `Cargo.lock` changes by exactly two hunks —
  `httparse`'s own package block, and `nova-runtime`'s dependency list
  gaining one line.
- **This ADR's own rulings reach forward.** Any later intrinsic returning a
  structured table has this one's four properties (§1) to match or to
  depart from with its own stated reason; any later `Limits`-shaped API
  crossing a single-argument intrinsic boundary has the same "tighten only"
  constraint for the same reason (§7); and any later std module adding a
  variant name already claimed by another std module's export meets the
  open question §7 declined to resolve, not a precedent that resolved it.
- **Records swept for staleness this ADR's own existence creates.** Several
  documents asserted, accurately at the time, that `std/http` was unstarted
  or that hyper was the plan for it; this increment's own commit corrects
  the ones the sweep found — `nova-spec/13-RUNTIME.md`,
  `docs/adr/0018-std-json-scope-and-build-order.md`,
  `docs/phase-2-plan.md`, `CHANGELOG.md` and one Rust test's doc comment —
  following the convention each of those documents already uses for a
  correction: a dated note added forward, not the original sentence
  rewritten.

## References

- Design: `docs/superpowers/specs/2026-09-01-std-http-request-parsing-design.md`
  (§2 the three blockers, measured; §4 the encoding and the handle-table
  rejection; §7 the eager-materialisation cost)
- Plan: `docs/superpowers/plans/2026-09-01-std-http-request-parsing.md`
- `nova-spec/00-MASTER-SPEC.md:240`: Phase 2 position 10, narrowed to
  parsing internals by this increment's record amendment; `:245`: the Phase
  2 gate; `:416`: the dependency table, recording `httparse` in place of
  the `hyper` line
- `nova-spec/20-STDLIB.md` §6: `std/http`'s specification, and this
  increment's dated amendment recording what shipped against it
- `docs/adr/0005-mutable-receivers-and-one-shot-hash.md`: the seeded,
  finalised string hash and its later amendments, cited rather than
  re-derived in §6 above
- `docs/adr/0009-async-execution-model.md`: single-threading as a
  correctness requirement, the reason hyper cannot have its own thread
- `docs/adr/0012-file-descriptor-lifecycle.md`: the `File` handle-table
  pattern, its accepted per-descriptor leak, and the alternative this ADR
  declines for a per-request-frequency resource
- `docs/adr/0018-std-json-scope-and-build-order.md`: position 11 shipped
  ahead of position 10, and the "unstarted and unblocked" record this
  increment closes the first half of
- `crates/nova-runtime/src/http.rs`: `nova_rt_http_parse_request`, the
  `ERR_*` kinds, `status_only`, `offset_within`, and the panic-freedom
  guard `no_http_intrinsic_can_panic`
- `crates/nova-runtime/src/task.rs`: `IN_BLOCK_ON`,
  `run_aborts_when_an_async_fn_calls_block_on`, and the module
  documentation naming the executor's two wake sources
- `crates/nova-runtime/src/bytes.rs`: `nova_rt_bytes_to_ints`, the
  scanned-allocation precedent §3 above matches
- `std/http/lib.nova`: `parse_offsets`, `parse_request_head`,
  `http_error_kind_of`, and `read_request`'s own doc comment, which carries
  the body-accumulation cost in full
