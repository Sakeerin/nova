# `05-json-api` — measured throughput

**Date of first content:** 2026-09-10. **Corrected 2026-09-11.**
**Host:** MINGW64_NT-10.0-26100, 12 cores.

`nova-spec/60-EXAMPLES.md` §5 names this file as the destination for this
example's numbers, and it has been unsatisfiable until the example existed.

---

## AMENDMENT 2026-09-11: the figures first published here were WRONG

**Every claim derived from them inverts.**

**What this file said on 2026-09-10 was 455.5 req/sec at a ten-user
collection. That run measured a binary built by a debug `nova`**, so
`find_runtime_lib` linked the **debug** runtime staticlib. This file's own
"What was measured" section asserted the opposite — "The example was built
with `./target/release/nova`, so `find_runtime_lib` resolved the release
runtime staticlib sitting beside it" — and that sentence was a claim about a
build nobody had checked. `docs/benchmarks/README.md` already carried a
verdict table marking `debug` + Cranelift as "measurable, misleading — a
debug runtime depresses everything". **The hazard was documented by name and
the procedure had no step that checks it.**

**How the wrong binary got measured.** `nova build -o NAME` writes exactly
`NAME`, with no platform executable suffix — a subtlety
`docs/benchmarks/README.md` documents. An earlier build had left a
`NAME.exe` beside it. The measurement asked for `NAME.exe` and got a binary
that was real, ran, and served the right routes; it was simply a different
build. An explicit path that does not exist fails loudly. A stale file at
the path you asked for succeeds and answers every sanity check.

**Identified by measuring the binaries, not by inferring.** The stale binary
was still on disk and was re-measured beside a correct one, back to back in
one script at identical parameters:

| binary | bytes | runtime profile | source | ten-user req/sec |
|---|---|---|---|---|
| **the one published** | 965,632 | **debug** | pre-routing-fix | **258.9, 386.7** |
| same source, release `nova` | 690,176 | release | pre-routing-fix | 2364.8 |

And the profile attribution rests on a grid rather than on one matching
cell, because a size match alone cannot tell a profile difference from a
source difference:

| `nova` profile | source revision | binary bytes |
|---|---|---|
| release | pre-routing-fix | 690,176 |
| release | current | 690,176 |
| debug | pre-routing-fix | **965,632** |
| debug | current | **965,632** |

Across the two source revisions tested, binary size here moves with the
`nova` profile and not with the revision. The stale binary was 965,632 bytes and answered
`/foo/1` with `200`, so it was a debug build of the pre-routing-fix source.
**The routing fix itself costs nothing measurable** — release builds of both
revisions land in the same throughput range.

**So this file now records the binary's byte size beside every figure.** The
size makes the runtime profile a checkable fact rather than a claim, and it
is what would have caught this from the record alone.

**Superseded figures, kept visible rather than deleted:** 455.5 req/sec at
ten users, 3100.6 empty, 232.1 at twenty users, a 3.81–4.07 µs-per-byte
whole-server marginal, a 167 µs response side at **7.6%** of per-request
cost, an unmeasured **92%**, a **13×** amplification, and the guidance that
a reader optimising `users_json` addresses under a tenth of the cost. **All
of these are withdrawn.** The corrected values are below, and the last two
invert: `users_json` is the largest single identified cost.

**What could not have caught this.** The golden test
(`json_api_example_serves_its_routes`) drives the example with `nova run` on
the source and never builds a binary. CI does run the load generator, in
`bench_http_server_and_generator_agree`, but that test's own comment says its
assertions are "the target run's, minus any throughput number" -- it checks
that at least one request completed and no error did, and it too drives its
server with `nova run`. So no test in this workspace compares a throughput
figure against anything, and none measures a built binary.

---

**Phase 2's gate is NOT met by this measurement, and nothing here claims
otherwise.** `nova-spec/00-MASTER-SPEC.md` §3 asks for 10k+ req/sec. The
corrected ten-user figure is a **range of 1875.2 to 3108.5 req/sec** across
the six runs taken in a fresh process — short of the gate by roughly
**3× to 5×**, where the withdrawn figure implied 22×. §5's other criterion, a ratio
against Bun, is **not measured here**; Bun 1.3.0 is installed on this host,
so that half is measurable rather than blocked.

## What was measured, and with what

Every parameter below belongs to the figure. A req/sec number for a list
endpoint without its payload size, its build axes and its route is right for
some formula and unlabelled as to which.

- **Binary:** `690,176` bytes. **This is the identity check, not a
  description.** A debug-runtime build of the same program is 965,632 bytes,
  so the size distinguishes the profile from the record alone.
- **Route:** `/users`, the endpoint §5's own methodology benchmarks. Passed
  as `--path /users`. **The `RESULT` line does not carry the path** — a
  deliberate choice to keep recorded lines format-comparable with
  `docs/benchmarks/http-fixed-response.md` — so the route is recorded here.
- **Backend:** Cranelift. The LLVM path cannot run on this host; neither
  `clang` nor `llc` is present.
- **Runtime profile:** release, established by the byte size above rather
  than asserted.
- **Generator:** `crates/nova-bench-http`, dependency-free, 200 connections,
  15s measurement, 5s warmup, keep-alive.
- **One fresh server process per data point.** The 2026-09-10 runs took
  empty, then ten, then twenty inside a single process, which varied heap
  age alongside payload size. See "Two confounds" below.
- **Seeding:** by `curl` POSTs into a freshly started server, verified with
  `curl /users` before measuring, which is also where the body size in each
  row comes from.
- **Payload note.** These runs serve **534 bytes** at ten users and 1094 at
  twenty, against 494 and 1014 for the withdrawn runs, because the seeded
  names and e-mail addresses differ in length. Rows here are comparable with
  each other; they are not directly comparable with the withdrawn ones, and
  that is a second reason not to read the two sets as one series.
- **Shell note:** Git Bash rewrites `--path /users` through its MSYS path
  layer and the generator then refuses the mangled value, so every run used
  `MSYS_NO_PATHCONV=1`.

## The runs

Verbatim `RESULT` lines, two replicates, each point in its own process. The
`addr` port differs on every line because every point started its own
server, which is the methodology fix and not an artifact of transcription.

```
RESULT mode=target addr=127.0.0.1:53085 connections=200 requests=134358 errors=0 elapsed_ms=15028 rps=8940.2 conn_min=662 conn_max=696
RESULT mode=target addr=127.0.0.1:53493 connections=200 requests=28846 errors=0 elapsed_ms=15129 rps=1906.7 conn_min=136 conn_max=157
RESULT mode=target addr=127.0.0.1:61035 connections=200 requests=14396 errors=0 elapsed_ms=15232 rps=945.1 conn_min=65 conn_max=101
RESULT mode=target addr=127.0.0.1:59065 connections=200 requests=138099 errors=0 elapsed_ms=15042 rps=9180.4 conn_min=661 conn_max=760
RESULT mode=target addr=127.0.0.1:59470 connections=200 requests=28364 errors=0 elapsed_ms=15125 rps=1875.2 conn_min=133 conn_max=166
RESULT mode=target addr=127.0.0.1:59887 connections=200 requests=17405 errors=0 elapsed_ms=15186 rps=1146.1 conn_min=85 conn_max=95
```

| collection | response body | req/sec, two replicates | µs per request |
|---|---|---|---|
| empty | 2 B (`[]`) | 8940.2, 9180.4 | 112, 109 |
| ten users | 534 B | **1906.7, 1875.2** | 525, 533 |
| twenty users | 1094 B | 945.1, 1146.1 | 1058, 873 |

**A single run cannot carry a point value here.** Twelve ten-user runs on
the release runtime were taken across the scripts of this increment, at 15s
and 30s. Split by the condition that turns out to matter:

| process state | runs | req/sec | spread | median |
|---|---|---|---|---|
| fresh | 6 | 1875.2, 1906.7, 2291.6, 2364.8, 2622.1, 3108.5 | 1.66× | 2328.2 |
| aged | 3 | 1769.6, 1902.3, 2136.1 | 1.21× | 1902.3 |
| not recorded | 3 | 1874.6, 2007.2, 2114.5 | 1.13× | 2007.2 |

All twelve span 1769.6 to 3108.5, a 1.76× spread. **The fresh-process row is
the one this file's figures come from**, because that is the methodology the
procedure now prescribes; the third row is three 15s runs taken before
process age was known to matter, so their process state was not recorded and
they are reported separately rather than folded in. Every figure in the
withdrawn record was a single run, and so is every figure in
`docs/benchmarks/`.

**`errors=0` on every run is meaningful here and was not before this
increment.** The generator gained a response-status check when this example
landed; previously it counted any completed round trip as a success, so a
run against the wrong route reported `errors=0` while measuring the
not-found fallback. Zero now means the route answered 2xx. It does **not**
mean the body was correct — the check does not inspect bodies — which is why
each run was preceded by a `curl /users` that confirmed the expected user
count and recorded its byte length.

## Two confounds in the withdrawn methodology, both measured

**Process age costs 1.29× to 1.38×.** A server that has already served
roughly 280,000 requests measures slower on the same workload than a freshly
started one. Alternating fresh and aged arms so machine drift cannot
masquerade as the effect, two replicates:

| replicate | fresh | aged, after a 30s empty-store run in the same process | ratio |
|---|---|---|---|
| 1 | 2622.1 | 1902.3 | 1.38× |
| 2 | 2291.6 | 1769.6 | 1.29× |

ADR 0002 describes the collector as a leaking allocator in Phase 1 terms,
which is a plausible mechanism and **is not established by these runs**.
Against a run-to-run spread of up to ~1.4× at fixed everything, the effect
is directionally consistent in both replicates but not cleanly separated
from noise. **What follows for the procedure is unambiguous either way:**
take one fresh process per data point, because the withdrawn table varied
payload size and heap age together and attributed the whole difference to
payload.

**The harness ceiling figure is noisy, not wrong.** `--self-test` spawns its
server inside the generator binary, so the target binary is irrelevant to
it, and the withdrawn `rps=122129.0` is **not invalidated** by the
debug-runtime error. A second sample of the same quantity gave 63455.4 — a
1.9× spread — so it bounds the harness loosely rather than precisely. In
both samples the generator had ample headroom over any target figure here
and was not the constraint. The 122129.0 run carried `errors=75` out of
3,785,298 with `conn_min=0` beside `conn_max=84034`: one connection
completed nothing while another completed tens of thousands. That is a
property of the in-process self-test server under 200 connections, and a
ceiling with a starved connection is weaker evidence than a clean one.

## Where the cost is

**Re-profiled in a compiled binary, because the withdrawn profiling used
`nova run`** while the measured server is compiled — a comparison between
execution modes that was never sound. The JIT figures turn out to transfer:
158,399 ns per `users_json` call compiled against 151,656 under the JIT,
within about 4% on the dominant term. **So the numerator was never the
problem. The denominator was**, being a 2,195 µs per-request budget derived
from the debug build's 455.5 req/sec.

Decomposition at ten users, compiled, 2000 iterations each:

| step | ns per call | share of response side |
|---|---|---|
| `users_json` | 158,399 | 94% |
| `to_bytes` | 8,663 | 5% |
| `json_response` | 1,612 | 1% |
| **response side total** | **168,674** | — |

`users_json` in isolation, compiled, at four collection sizes:

| users | bytes | ns/call | ns/byte |
|---|---|---|---|
| 5 | 266 | 80,092 | 301.1 |
| 10 | 532 | 158,399 | 297.7 |
| 20 | 1092 | 322,236 | 295.1 |
| 40 | 2212 | 594,432 | 268.7 |

Flat per byte across a factor of eight in collection size. **The
quadratic-accumulation hypothesis is refuted from two directions** — the
server halves rather than quarters its throughput when the collection
doubles, and the isolated function's per-byte cost does not climb.

**The response side is about 32% of per-request cost, and the code it
indicts is where the time is.** 168.7 µs against the 525 and 533 µs per
request the two replicated ten-user runs imply. At the fastest ten-user run
observed (3108.5 req/sec, 322 µs) the same 168.7 µs is about 52%. The
runtime is single-threaded by ADR 0009, so 1/rps is a serial per-request
budget rather than an average over parallel workers.

**The whole-server marginal is 0.60 to 0.95 µs per byte**, computed within
each replicate: 0.776 and 0.798 from empty to ten users, 0.950 and 0.604
from ten to twenty. Against the isolated 0.298 µs per byte that is an
amplification of **2.0× to 3.2×** — where the withdrawn record claimed 13×
and concluded from it that the body-building code was the wrong place to
look.

**And the budget nearly closes.** The empty-store baseline is 109–112 µs per
request, covering read, parse, routing, write and scheduling with a 2-byte
body. Adding the response side's growth to that predicts roughly 260–280 µs
at ten users. Observed: 322 µs at the fastest run, 525–533 µs in the
replicated pair. **The residual is on the order of 20% at the fast end, not
92%**, and it is unattributed to any named mechanism. Figures that nearly
add up are still not an explanation, and nothing here measured
`read_request`'s parse, the socket write, the scheduler or the collector
separately.

## Comparison, and one that does not hold

`docs/benchmarks/http-fixed-response.md` records `rps=11940.0` for
`std/http`'s read-and-parse path. **That figure and these are not
comparable**, and the difference is not a regression: that server builds its
response bytes **once, outside the accept loop**, and reuses them, which its
own header states. This example cannot — every response is freshly built
from current state. The empty-store rows here are the closest thing to a
like comparison at 8940.2 and 9180.4, and still pay full response
construction; the gap from 11940.0 to those is the 2-byte body's own
`json_response` and `to_bytes`, plus one extra header.

That recorded 11,940 is also a single run, and it predates the identity
check this file now applies to itself. Nothing has re-verified which binary
produced it.

## What this does not settle

- **Why the per-byte cost is what it is.** `users_json` costs about 298 ns
  per byte of output and no mechanism is named for that figure.
- **The remaining fifth of the per-request budget**, above. Narrowed, not
  attributed.
- **Whether process age is the collector.** Measured as an effect; ADR 0002
  is a plausible cause and is not established here.
- **The two costs already recorded against `std/http`** — eager header
  materialisation, and quadratic body accumulation — are neither confirmed
  nor refuted. The generator sends one header and no body, so this
  measurement does not reach either.
- **The Bun ratio**, §5's own criterion. Unmeasured, and measurable on this
  host.
- **Anything about other hosts.** One host, two replicates per point,
  Windows.
