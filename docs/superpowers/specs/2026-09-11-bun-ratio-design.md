# Measuring the Bun ratio — design

**Date:** 2026-09-11. **Base:** `main` at `66b3e1e`, 700 commits, 0 merge
commits ever, 1130 passed / 0 failed / 8 ignored across 45 targets.

## 1. What this is, and what it is not

`nova-spec/60-EXAMPLES.md` §5 states a gate criterion that has never been
measured:

> **Gate:** Benchmark vs Bun on same hardware shows >= 1.0x req/sec ratio.
> Document numbers in `examples/05-json-api/BENCHMARK.md`.

This increment measures it. **It optimises nothing, and it claims nothing
about whether the gate should be met.** The deliverable is a ratio with every
parameter that belongs to it, an equivalence check that makes the ratio mean
something, and the procedure written down so a later increment can repeat it.

**It is not** the absolute 10k criterion in `nova-spec/00-MASTER-SPEC.md` §3,
which is recorded as non-equivalent to this one and able to disagree in either
direction. It is not a performance increment. It does not touch
`examples/05-json-api/src/main.nova`.

## 2. Why this criterion and not the absolute one

Two reasons, both measured rather than argued.

**The absolute criterion names a hardware target the spec never defines.**
`git grep -i 'benchmark hardware'` finds the phrase only inside restatements
of the gate itself. Every figure recorded against it so far came from one
12-core Windows development host. §5's criterion is **self-normalising** —
both sides run on the same machine — so it is well-posed on hardware the
spec never pinned.

**A same-session ratio cancels the variance that has twice produced a wrong
record here.** Twelve release-runtime runs of one workload spanned 1.76x, and
six fresh-process runs of it spanned 1.66x. An absolute figure inherits all
of that. A ratio whose numerator and denominator are taken minutes apart on
one machine does not, because the machine's state is common to both.

**And the absolute criterion is not reachable by response-path work**, which
is worth recording here because it is why this increment is not that one.
The gate allows 100 microseconds per request; the runtime is single-threaded
by ADR 0009 as a correctness requirement, so 1/rps is a serial per-request
budget rather than an average over workers. With a **2-byte** body the
example already costs 108.9 to 111.9 microseconds per request. Driving the
response body's cost to zero therefore leaves it near 9,180 req/sec, still
short. That is a finding this increment records only as context; acting on it
belongs to a later one.

## 3. Architecture

Four artifacts, three of them records.

**`docs/benchmarks/bun-server.js`** — the comparison arm. It lives in
`docs/benchmarks/` and not beside the example, because `examples/` holds Nova
programs and a `.js` file there would read as part of the example. This
follows the placement already set by `docs/benchmarks/server.nova`.

**`docs/benchmarks/README.md`** — gains the comparison procedure: the
equivalence check, the pinning mechanism, the matrix, and what each figure
may be quoted for.

**`examples/05-json-api/BENCHMARK.md`** — gains the ratio. §5 names this file
as the destination for this example's numbers.

**`CHANGELOG.md`** — an `[Unreleased]` entry.

**No new crate, no new dependency, and no change to any Nova source.**

### 3.1 The Bun server uses `Bun.serve` and no router library

Nova's side is a raw `std/http` accept loop, because `std/http` has no
router — `pub type Handler = async fn(Request) -> Response` is `P0001`.
Putting a routing framework on Bun's side would measure Nova against that
framework. `Bun.serve` with a hand-written path match is the like-for-like
shape.

Same reasoning for state: an in-memory object with ids dense from 1, matching
`record Store { users: Map<Int, User>, next_id: Int }`. No database, no
persistence, no clustering.

## 4. Equivalence is checked, not asserted

This is the part that makes the ratio mean anything, and it gets the same
treatment the runtime profile now gets.

**Response cost here is linear in body bytes** — the whole-server marginal is
0.60 to 0.95 microseconds per byte, and `users_json` is flat at about 298
nanoseconds per byte across a factor of eight in collection size. A Bun
server emitting JSON with different spacing would move the ratio by more than
the quantity being measured. So bodies are compared **byte for byte** before
any figure is taken.

**The roster is the nine exchanges the example's golden test already drives**,
reused rather than invented, because that roster has been through review:

| # | request | expected |
|---|---|---|
| 1 | `GET /users` on an empty store | 200, `[]` |
| 2 | `POST /users` | 201, the created user |
| 3 | `GET /users/1` | 200, that user |
| 4 | `GET /users/zz` | 400, id must be an integer |
| 5 | `GET /users/99` | 404, no such user |
| 6 | `GET /nope` | 404, not found |
| 7 | `GET /nope/1` | 404, not found |
| 8 | `POST /users` with a name containing a quote and a backslash | 201, escaped |
| 9 | `GET /users` on the populated store | 200, the list |

For each, the check compares **status code and response body bytes** between
the two servers and fails loudly on any difference. Exchange 8 is
load-bearing: it is what catches a Bun server using a different JSON escaping
convention, which would otherwise show up only as a byte-count difference in
the ratio.

**Bodies are compared; wire framing is not, and the difference is measured.**
Checked on this host rather than assumed: for a 2-byte body the Nova example
writes a **70-byte** response head — `HTTP/1.1 200 OK`, `content-type`,
`content-length` — and `Bun.serve` writes **107 bytes**, adding exactly one
header, `Date`, at 37 bytes. Bun adds no `Connection` header.

**That 37-byte difference biases the ratio in Nova's favour, which is the
direction that matters most here.** Bun writes about 6% more wire bytes per
response at this payload, so Bun pays for framing Nova does not, and the
quotient Nova/Bun comes out higher than like-for-like. On a criterion Nova
has to clear at 1.0, a bias inflating Nova's side is the one that could
manufacture a pass. So it is **stated beside the ratio with its byte count**,
and a ratio within a few percent of 1.0 must be read against it rather than
reported as a pass.

Stripping `Date` from Bun's output is deliberately **not** attempted: it is
what `Bun.serve` does, and making Bun unrepresentative to flatter the
comparison would be a worse distortion than a disclosed 37 bytes.

## 5. Pinning, and why both sides get measured both ways

ADR 0009 makes single-threading a **correctness** requirement for Nova — the
collector's heap is thread-local — so Nova's throughput is one core's worth
by construction. Bun's is not. "Same hardware" therefore does not settle the
comparison, and the choice changes the number several-fold.

**The headline ratio pins both processes to core 0.** That isolates language
and runtime cost from thread count, which is what a runtime comparison is
for. **Bun's unpinned figure is recorded beside it** as what Bun actually
does on this box, so neither figure can be quoted without the other.

**Nova is measured pinned as well as unpinned**, rather than reusing the
existing unpinned figures as the numerator. Pinning may move Nova's own
throughput — its mask reads 4095, so the OS is free to move its single thread
across all twelve cores — and assuming it does not is precisely the class of
unchecked assumption that produced the withdrawn 455.5 figure. If pinned and
unpinned Nova differ, the pinned ratio takes the pinned numerator.

Mechanism, verified on this host: spawn, then set `ProcessorAffinity` from
PowerShell and **read it back**. Measured on a live server process, the mask
went from 4095 (all twelve cores) to 1. The procedure records the mask it
read back, not the one it set.

Process affinity sets the default for every thread in the process, so Bun's
worker and GC threads inherit it — but a thread can override its own
affinity and nothing here establishes that Bun does not. So the procedure
**also observes Bun's CPU usage during the pinned run** and records whether
it is consistent with one core. A pinning that silently did not take would
otherwise look exactly like a Bun that is slow.

## 6. The measurement matrix

| | pinned to core 0 | unpinned |
|---|---|---|
| Nova `05-json-api` | headline numerator | recorded |
| Bun | headline denominator | recorded |

Common to every cell:

- The same generator, `crates/nova-bench-http`, driving both sides. Using one
  generator for both is the strongest fairness property available here.
- `--path /users`, 200 connections, `--warmup 5`, `--duration 30`.
- A **ten-user** collection, seeded by `curl` POSTs, matching the collection
  the existing record uses.
- **One fresh server process per data point.** Process age costs Nova 1.29x
  to 1.38x after roughly 280,000 requests.
- **At least two replicates per cell**, reported as a **range**, with the
  record stating how many were taken. A single run cannot carry a point
  value here: twelve runs of one workload spanned 1.76x.

**Identity recorded for each side**, because a figure's parameters must be
checkable rather than described: Nova's binary **byte size** (release-runtime
builds of this program are 690,176 bytes and debug ones 965,632, so the size
distinguishes the profile), and for Bun the output of `bun --version` and the
script's byte size.

## 7. Bun's warmup is an open variable, and is measured rather than assumed

The 5-second warmup was chosen for an ahead-of-time compiled Nova. Bun is
JIT-compiled, so its steady state may arrive later. The procedure takes one
Bun run at the standard 5-second warmup and one at a longer warmup, compares
them, and **records what it found**. If they differ materially the longer
warmup becomes part of the recorded methodology for Bun's side and the
difference is stated; if they do not, that is recorded too, because "we
checked and it did not matter" and "we did not check" are different things to
inherit.

## 8. Testing, and why there is none

**No automated test, deliberately, and the records say so** — otherwise
"where is the test" is a reasonable review finding with no answer in the
tree.

CI runners have no Bun, so a normal test would fail everywhere. **An
`#[ignore]`d test would be worse than none**: CI's Test job carries an
advisory step that runs exactly the ignored tests, so a Bun test placed there
would fail on every push inside a step whose failures are tolerated and
unread. That step currently passes cleanly on ubuntu, windows and macOS, and
spending it on a test that cannot run anywhere is a poor trade.

What stands in for a test is the **equivalence check of §4**, which is part
of the procedure, runs before any figure is taken, and whose result is
recorded beside the figure. It is a gate on the measurement rather than a
gate on the build, and the distinction is stated in the record.

## 9. Records to amend

- `examples/05-json-api/BENCHMARK.md` — the ratio, the matrix, the
  equivalence result, the wire-framing byte difference, and each side's
  identity. §5 names this file.
- `docs/benchmarks/README.md` — the comparison procedure, the pinning
  mechanism with its read-back, and the warmup finding.
- `nova-spec/60-EXAMPLES.md` §5 — a dated amendment recording that its own
  criterion is now measured, and what it says. Its existing text calls the
  ratio "still unmeasured".
- `nova-spec/00-MASTER-SPEC.md` §3 — a dated amendment recording that the
  gate's two criteria now have one measured figure each, and whether they
  agree.
- `CHANGELOG.md` — an `[Unreleased]` entry.

Amendments are appended and the original wording is left as written, which is
the convention every prior correction in these files follows.

## 10. What this does not cover

- **Optimising anything.** No Nova source changes.
- **The absolute 10k criterion.** §2's arithmetic is recorded as context and
  acted on by nobody here.
- **Other hardware.** One host, Windows. Bun on Windows is younger than Bun
  on Linux and that is recorded as a caveat rather than corrected for.
- **Latency percentiles.** §5's methodology names p50, p95 and p99;
  `nova-bench-http` reports total requests, elapsed, req/sec, errors and
  per-connection min/max, and gains no percentile support here. The gap is
  recorded rather than closed, because the criterion this increment measures
  is a req/sec ratio.
- **`wrk`.** §5's methodology names it; it is POSIX-only and does not run on
  this host, which is already recorded in that section's own amendment.
- **Whether process age is the collector.** Measured as an effect, unproven
  as a mechanism.

## 11. Success criteria

1. `docs/benchmarks/bun-server.js` exists and serves the nine exchanges with
   **byte-identical bodies and identical status codes** to
   `examples/05-json-api`, demonstrated by a check whose output is recorded.
2. All four cells of §6's matrix are measured with at least two replicates
   each, reported as ranges rather than point values, and the record states
   the replicate count.
3. The ratio is stated with its parameters: which cell over which cell, the
   payload byte count, both identities, the pinning mask read back, and the
   wire-framing byte difference.
4. Bun's warmup sensitivity is measured and recorded either way.
5. Every record in §9 is amended.
6. The suite is unmoved at 1130 / 0 / 8 across 45 targets, `cargo fmt --all
   -- --check` passes, and `cargo clippy --locked --all-targets
   --all-features -- -D warnings` passes on the platforms CI gates.
7. **No claim that the gate is met or unmet beyond what the measured ratio
   supports**, and no figure stated as a point value where a range was taken.
