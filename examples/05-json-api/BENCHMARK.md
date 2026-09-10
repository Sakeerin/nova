# `05-json-api` — measured throughput

**Date:** 2026-09-10. **Host:** MINGW64_NT-10.0-26100, 12 cores.

`nova-spec/60-EXAMPLES.md` §5 names this file as the destination for this
example's numbers, and it has been unsatisfiable until the example existed.
This is its first content.

**Phase 2's gate is NOT met by this measurement, and nothing here claims
otherwise.** `nova-spec/00-MASTER-SPEC.md` §3 asks for 10k+ req/sec. The
figure for the endpoint the gate names is **455.5 req/sec** at a ten-user
collection — short by a factor of roughly twenty-two. §5's other criterion, a
ratio against Bun, is **not measured here**; Bun 1.3.0 is installed on this
host, so that half is measurable rather than blocked, and a later increment
can take it deliberately.

## What was measured, and with what

Every parameter below belongs to the figure. A req/sec number for a list
endpoint without its payload size, its build axes and its route is right for
some formula and unlabelled as to which.

- **Route:** `/users`, the endpoint §5's own methodology benchmarks. Passed as
  `--path /users`. **The `RESULT` line does not carry the path** — that was a
  deliberate choice to keep recorded lines format-comparable with
  `docs/benchmarks/http-fixed-response.md` — so the route is recorded here
  instead.
- **Backend:** Cranelift. The LLVM path cannot run on this host; neither
  `clang` nor `llc` is present.
- **Runtime profile:** release. The example was built with
  `./target/release/nova`, so `find_runtime_lib` resolved the **release**
  runtime staticlib sitting beside it. Nothing pins the profile, so a debug
  `nova` would have linked a debug runtime and depressed every figure here.
- **Generator:** `crates/nova-bench-http`, dependency-free, 200 connections,
  30s measurement, 5s warmup, keep-alive.
- **Seeding:** by `curl` POSTs before each run, verified with `curl /users`
  before measuring. The store is in-memory and shared across connections.
- **Shell note:** Git Bash rewrites `--path /users` through its MSYS path
  layer and the generator then refuses the mangled value, so every run used
  `MSYS_NO_PATHCONV=1`.

## The runs

Verbatim `RESULT` lines, in the order taken. The empty-store run came first
because the store cannot be un-seeded without restarting the server.

```
RESULT mode=target addr=127.0.0.1:65312 connections=200 requests=93175 errors=0 elapsed_ms=30050 rps=3100.6 conn_min=459 conn_max=491
RESULT mode=target addr=127.0.0.1:65312 connections=200 requests=13900 errors=0 elapsed_ms=30513 rps=455.5 conn_min=69 conn_max=73
RESULT mode=target addr=127.0.0.1:65312 connections=200 requests=7118 errors=0 elapsed_ms=30671 rps=232.1 conn_min=34 conn_max=38
RESULT mode=self-test addr=127.0.0.1:55113 connections=200 requests=3785298 errors=75 elapsed_ms=30994 rps=122129.0 conn_min=0 conn_max=84034
```

| collection | response body | req/sec | errors | ms per request |
|---|---|---|---|---|
| empty | 2 B (`[]`) | 3100.6 | 0 | 0.32 |
| ten users | 494 B | **455.5** | 0 | 2.20 |
| twenty users | 1014 B | 232.1 | 0 | 4.31 |
| harness ceiling | fixed | 122129.0 | 75 | — |

**`errors=0` on all three target runs is meaningful here and was not before.**
The generator gained a response-status check in this same increment;
previously it counted any completed round trip as a success, so a run against
the wrong route reported `errors=0` while measuring the not-found fallback.
Zero now means the route answered 2xx. It does **not** mean the body was
correct — the check does not inspect bodies — which is why each run was
preceded by a `curl /users` that confirmed the expected user count.

## What the three collection sizes establish

The empty-store run exists to separate response *framing* from element
*serialisation*, and the twenty-user run to characterise how the cost grows.

**Cost is linear in response bytes, at roughly 4 µs per byte.** Marginal:
1.87 ms over the 492 bytes from empty to ten users (3.81 µs/B), and 2.11 ms
over the 520 bytes from ten to twenty (4.07 µs/B).

**A quadratic-accumulation hypothesis was tested and refuted.**
`users_json` builds its output with `out = "${out}..."`, which looks like the
same shape as the quadratic `Bytes::concat` body accumulation already recorded
against `std/http` — so doubling the collection should have quartered
throughput. It halved it: 455.5 to 232.1, a ratio of 1.96 against a payload
ratio of 2.05. Whatever dominates is proportional to output size, not to the
square of the element count. **Naming the actual mechanism needs profiling and
is not attempted here.**

## Comparison, and one that does not hold

`docs/benchmarks/http-fixed-response.md` records `rps=11940.0` for
`std/http`'s read-and-parse path. **That figure and these are not
comparable**, and the difference is not a regression: that server builds its
response bytes **once, outside the accept loop**, and reuses them, which its
own header states. This example cannot — every response is freshly built from
current state. The empty-store run here is the closest thing to a like
comparison and still pays full response construction.

The harness ceiling is recorded for the same reason it always is: to show the
generator was not the constraint. At 122129.0 req/sec against a target of
455.5, it was not.

**The ceiling run itself was not clean, and that is disclosed rather than
dropped:** `errors=75` out of 3,785,298, with `conn_min=0` beside
`conn_max=84034`. One connection completed nothing while another completed
tens of thousands. That is a property of the in-process self-test server under
200 connections, not of the example, and it does not affect the target
figures — but a ceiling with a non-zero error count and a starved connection
is weaker evidence than one without, so it is written down.

## What this does not settle

- **Why the per-byte cost is what it is.** Measured, not diagnosed.
- **The two costs already recorded against `std/http`** — eager header
  materialisation, and quadratic body accumulation — are neither confirmed nor
  refuted here. The generator sends one header and no body, so this
  measurement does not reach either.
- **The Bun ratio**, §5's own criterion. Unmeasured, and now known to be
  measurable on this host.
- **Anything about other hosts.** One host, one run per configuration.
