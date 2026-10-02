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

**RESOLVED 2026-09-12 (branch `remeasure-after-fast-path`): the range this
amendment corrects to, 1875.2 to 3108.5 req/sec, is re-measured in the
further amendment below at 2868.2 to 3392.4 req/sec at ten users — six
fresh-process readings against this range's own six — with a binary built
from `stringify`'s scalar fast path. The two ranges OVERLAP; the later one
does not cleanly replace this one, and the amendment below says why.**

---

## AMENDMENT 2026-09-12: `stringify`-affected figures below measure a superseded build

**`std/json`'s `stringify` gained a scalar fast path on this branch.** Every
figure below that exercises `users_json` on a NON-EMPTY user collection --
which calls that top-level `String` path twice per user -- was measured
through the `stringify` that existed before this branch. Two figures in
this file are NOT reached by this note: the empty-store rows (**8940.2**
and **9180.4** req/sec, below), because an empty collection means
`users_json` loops zero times and never calls `stringify`; and the
**11940.0** req/sec figure in "Comparison, and one that does not hold",
which is `docs/benchmarks/http-fixed-response.md`'s own program, not this
one, and was never measured through this file's `stringify` at all.

**What this note covers is the server figures below AND the compiled
per-call decomposition in "Where the cost is" further down -- not only the
former.** The absolute-criterion range (**1875.2 to 3108.5 req/sec**) and
the Bun-ratio range (**0.116 to 0.231**), whose Nova-side cells come from
that same binary, are both below. So is "Where the cost is"' compiled
decomposition: `users_json` at 158,399 ns/call with its 94%/5%/1% shares,
the "about 32% to 52%" response-side share, and the 2.0x-3.2x
amplification. All of it comes from a binary built before this branch.
**What changed between that build and this one is HOW a top-level scalar
reaches `stringify`'s output, not WHAT it outputs** -- the fast path is
argued byte-identical to the general path it shadows, in this increment's
own design doc, rather than asserted; the build is superseded as a
measurement of cost, not as a record of behaviour. "Measured 2026-09-12"
below shows the quantity at the center of that decomposition moved: this
same harness's `users_json`, measured in isolation, is faster after this
branch than the 158,399 ns/call (and the 80,092–594,432 ns/call range)
"Where the cost is" records, so the percentages, the 32%-52% framing and
the 2.0x-3.2x amplification derived from that number are stale in the same
way -- this amendment does not restate any of them as a new figure.

**What makes that checkable rather than only asserted.** The req/sec ranges
are already recorded beside the example binary's byte size, `690,176` --
see "What was measured, and with what" below and "Identity of each side" in
the Bun-ratio section. That recorded size is what a rebuild's own byte size
has to be compared against; this amendment does not perform that rebuild
and does not state what its byte size would be.

**Re-measurement is deferred, not silently owed, and for a stated reason.**
Neither the absolute-criterion range nor the Bun ratio came from one run:
the absolute criterion came from a fresh-process series with replicates,
and the Bun ratio came from a four-cell matrix (pinned and unpinned, both
sides) with replicates, alternating sides, gated by the equivalence check
(`docs/benchmarks/bun-equivalence.js`). The compiled decomposition came from
two thousand iterations at each of four collection sizes, in a compiled
binary rather than under `nova run`. Re-running any one of these without
the others would not reproduce its own methodology. **Owed against this
file:** the fresh-process absolute-criterion series, the four-cell
Bun-ratio matrix with replicates, and a re-run of the compiled
decomposition, all against a binary built from the current `stringify`,
with the equivalence check re-run alongside them -- before any figure named
in this amendment is read as current. No new throughput or per-call
figure, measured or predicted, is stated here.

**RESOLVED 2026-09-12 (branch `remeasure-after-fast-path`): all three items
owed above are done, against a binary built from the current `stringify`,
with the equivalence check re-run alongside them and passing.** See the
further amendment immediately below for the series, and the honesty
caveats that come with them.

---

## FURTHER AMENDMENT 2026-09-12 (branch `remeasure-after-fast-path`): the debt above is discharged, on both criteria, without closing the gate

All three items the amendment above named as owed — the fresh-process
absolute-criterion series, the four-cell Bun-ratio matrix with replicates,
and a re-run of the compiled decomposition — are measured here against a
binary built from the current `stringify`, with the equivalence check
re-run alongside them first. `json-api.exe` is **690,688 bytes** (690,176
before this branch — the identity check working the same way it worked for
the debug/release distinction in the first amendment above). The
equivalence gate (`docs/benchmarks/bun-equivalence.js`) passed: all nine
exchanges match on status and body bytes. Host: MINGW64_NT-10.0-26100, 12
cores, Windows — the host this file's header names.

### The finding: a matched comparison neither confirms nor refutes the amplification

The amendment above records a **2.0x to 3.2x** amplification between
`users_json`'s isolated cost and the whole server's marginal cost per byte,
arrived at by comparing two separately-measured quantities and explained by
nothing. This re-measurement is a controlled change of a known isolated
size, so it can test that band rather than re-infer it.

**It does not settle it either way, and saying so is the whole finding.**
Compared median against median on matched populations the amplification is
about **1.6x**. But propagating the ranges instead of the medians, the
per-request saving runs from **-27.0 to +238.5 microseconds** — the best
before reading is faster than the worst after one — which puts the
amplification anywhere from about **-0.4x to 3.8x**. That interval contains
the 2.0x-3.2x band entirely. **So this measurement locates the amplification
no better than the inference it was meant to test**, and a reader should take
1.6x as where the middles sit rather than as where the quantity is.

The comparison has to be drawn against the same population on both sides,
and "The runs" below enumerates six fresh-process ten-user readings from
before this branch — 1875.2, 1906.7, 2291.6, 2364.8, 2622.1 and 3108.5,
spread 1.66x, median 2328.2. Six fresh-process readings were taken after
it, across the two series here: 2868.2, 3024.9, 3036.5, 3056.5, 3078.4 and
3392.4, spread 1.18x, median 3046.5.

| quantity | before | after |
|---|---|---|
| six fresh-process readings, ten users | 1875.2 – 3108.5, median 2328.2 | 2868.2 – 3392.4, median 3046.5 |
| per request at the medians | 429.5 microseconds | 328.2 microseconds |

So the server saves about **101 microseconds** per request at the medians,
against an isolated `users_json` saving of **63.1 to 63.8 microseconds**
(158,399 ns/call before, 94,562 to 95,251 after) — an amplification of
**1.59x to 1.60x**.

**Three things keep even the median figure loose, and all are stated rather
than left for a reader to find.** The before and after ranges OVERLAP: the
highest before reading, 3108.5, exceeds the lowest after reading, 2868.2.
The before six were taken across different scripts and sessions while the
after six all come from one contiguous session, so between-session variance
sits in one population and not the other. And the two populations are not
composed alike: the after six include two readings from pinned cells, while
the before six include none — folding in the four available pinned and
unpinned before-side counterparts moves the median only from 2328.2 to
2331.7 and leaves the figure at 1.58x-1.60x, so the number is robust to that,
but the composition is disclosed rather than assumed away. The median shift
of 1.31x should be held loosely for all three reasons.

**An earlier draft of this section reported 3.06x to 3.30x**, and it was
wrong in a way worth recording rather than quietly fixing: it compared the
after readings against 1875.2 and 1906.7, which are the two LOWEST of the
six the same file already enumerated a few paragraphs below. Picking a
subsample rather than the recorded population roughly doubled the apparent
effect. The figure was never untraceable — every number traced — and that
is the point: traceability is not the same property as a matched
population.

The mechanism remains unestablished either way: GC pressure from fewer
allocations, cache behaviour, and allocator contention across 200 live
connections are all candidates, and this measures none of them.
**Measured, not diagnosed.**

### Absolute criterion, re-measured — one fresh process per point

200 connections, 15s measurement, 5s warmup, unpinned:

| collection | body | before | after | change |
|---|---|---|---|---|
| empty (CONTROL) | 2 B | 8940.2, 9180.4 | 9501.0, 8688.1 | 0.95x – 1.06x |
| ten users | 534 B | 1906.7, 1875.2 | **3078.4, 3036.5** | see below |
| twenty users | 1094 B | 945.1, 1146.1 | **2203.3, 2107.0** | see below |

**The change column is deliberately absent from the two seeded rows.** Each
pairs two readings against two, and for the ten-user row the two "before"
values are the lowest of six this file records; dividing them would restate
the error the section above withdraws. The matched six-against-six
comparison is there instead, and the twenty-user row has only two readings
per side, so it cannot carry the same weight as the ten-user one and no
ratio is offered for it here.

**The empty-store cell is a control, not a data point.** An empty
collection means `users_json` loops zero times and never calls
`stringify`, so this cell should not move under this change — and it does
not; its before and after ranges overlap. That is what makes the ten- and
twenty-user cells attributable to the fast path rather than to machine
drift between sessions.

Even taken on its own, the control shows this file's response-side work
cannot be the whole story for the 10k+ ask, and that is a claim with
standing history rather than one minted here: `CHANGELOG.md`'s
`[0.2.0-alpha.4]` entry and
`docs/superpowers/specs/2026-09-11-bun-ratio-design.md` §2 both record
that the absolute criterion is not reachable by response-path work alone,
because the gate allows 100 microseconds per request and the empty-store
control already exceeds it. **This session's control reads 105.3 to 115.1
microseconds per request against that 100-microsecond line — the fastest
reading is within about 5% of it, while the control's own two readings
differ from each other by 1.09x, a wider swing than that margin.** The
claim holds on every reading taken so far, but it is not a settled
impossibility: it rests on a quantity that moves by more than the margin
it has left, so a later reader should re-derive it rather than quote it
forward.

### Bun ratio, re-measured — four cells, two replicates each, alternating sides

200 connections, 30s measurement, 5s warmup:

| cell | before | after |
|---|---|---|
| A Nova pinned | 2234.7, 2500.9 | 2868.2, 3392.4 |
| B Nova unpinned | 2314.3, 2349.0 | 3024.9, 3056.5 |
| C Bun pinned | 12245.0, 19220.6 | 12470.8, 12480.6 |
| D Bun unpinned | 10165.1, 12498.7 | 12647.3, 12934.0 |

| ratio | before | after |
|---|---|---|
| pinned | 0.116 – 0.204 | **0.230 – 0.272** |
| unpinned | 0.185 – 0.231 | **0.234 – 0.242** |

**Part of the pinned ratio's apparent gain is Bun's own variance, not
Nova's.** The withdrawn 0.116 lower bound was computed against Bun's
19220.6, a reading far above Bun's other three pinned-cell readings across
both sessions; this session's two pinned Bun cells agree to within 10
req/sec of each other. The Nova-side change is the trustworthy half of this
comparison, and the absolute series above measures it more cleanly than
this ratio does.

### Compiled decomposition, re-run — ten users, same harness source as before, 2000 iterations per cell, two runs each its own process

Harness binary: **521,728 bytes**.

| step | before | after |
|---|---|---|
| `users_json` | 158,399 ns | **94,562 – 95,251 ns** |
| `to_bytes` | 8,663 ns | 9,325 – 9,437 ns |
| `json_response` | 1,612 ns | 1,759 – 2,000 ns |
| response side total | 168,674 ns | **105,999 – 106,335 ns** |

Shares after: `users_json` 89–90%, `to_bytes` 9%, `json_response` 2%.

**`to_bytes` and `json_response` read slightly slower than before, and
neither was changed by this branch.** Recorded rather than smoothed;
nothing here establishes whether the difference is measurement noise, code
layout, or something else, and both remain small absolute numbers beside
`users_json`.

`users_json` in isolation, after, across the same four collection sizes as
before: 5 users 46,265 and 46,614 ns; 10 users 95,251 and 94,562 ns; 20
users 198,027 and 196,703 ns; 40 users 358,962 and 374,218 ns.

### The gate, re-measured on both criteria, still not met

| criterion | asked | now | short by |
|---|---|---|---|
| absolute (`00-MASTER-SPEC.md` §3) | 10k+ req/sec | 2868.2 – 3392.4 req/sec at ten users, six fresh-process readings | ~2.9x to 3.5x |
| ratio (`60-EXAMPLES.md` §5) | at least 1.0 | 0.230 – 0.272 | ~3.7x to 4.4x |

Previously ~3.2x to 5.3x and ~4.3x to 8.6x respectively, each
from its own population. **Both criteria
moved in the same direction and both remain unmet.** Nothing here suggests
either is close: at ten users the absolute criterion is still short by
more than three times, and the control cell above shows response-side work
alone has no path to 10k+ regardless.

**Amended 2026-10-02 (gate-remeasure):** this table and the paragraph
above describe `991fdfc`. On `main` at `5efcc2e`, the same method gives:
- **Absolute, pooled the way the table pooled them:** 8424.0–10382.4
  req/sec, straddling 10k+.
- **Absolute, split by pinning:** unpinned 10250.0–10382.4, both readings
  above 10k by 2.5–3.8%, which is a thin margin on two readings; pinned
  8424.0–8520.1, below.
- **Ratio:** 0.70–0.79 pinned and 0.79–0.81 unpinned, not met.

See "AMENDMENT 2026-10-02 (gate-remeasure)".

**Figures superseded by this amendment, kept visible rather than
deleted.** The 2026-09-11 amendment's ten-user range of 1875.2 to 3108.5
req/sec is now measured at 2868.2 to 3392.4 over six fresh-process readings
against that range's own six — and note the two RANGES OVERLAP rather than
one replacing the other cleanly. Its Bun ratio of 0.116 to 0.231 is now
0.230 to 0.272, with the caveat above that part of the pinned movement is
Bun's own variance. "Where the cost is"' compiled decomposition — 158,399
ns/call for `users_json` with 94%/5%/1% shares — is now 94,562 to 95,251
ns/call with 89-90%/9%/2% shares.

**Its 2.0x-3.2x amplification is neither superseded nor confirmed.** The
matched comparison above puts the medians at about 1.6x while the ranges
admit anything from -0.4x to 3.8x, an interval containing that band; an
earlier draft of this amendment reported 3.06x-3.30x as a confirmation and
is withdrawn there, and a second draft reported 1.6x as a refutation and is
withdrawn with it. None of the superseded figures is edited at its own
site.

---

**Phase 2's gate is NOT met by this measurement, and nothing here claims
otherwise.** `nova-spec/00-MASTER-SPEC.md` §3 asks for 10k+ req/sec. The
corrected ten-user figure is a **range of 1875.2 to 3108.5 req/sec** across
the six runs taken in a fresh process — short of the gate by roughly
**3× to 5×**, where the withdrawn figure implied 22×. §5's other criterion, a ratio
against Bun, is **not measured here**; Bun 1.3.0 is installed on this host,
so that half is measurable rather than blocked.

**Both figures in this paragraph are further superseded — see the
"FURTHER AMENDMENT 2026-09-12" section above for the fresh-process series
and the four-cell Bun-ratio matrix that supersede them.**

## AMENDMENT 2026-09-29: two mechanisms of this file's own residual are measured, and its empty-store control moved

"Where the cost is" below states that nothing had measured
`read_request`'s parse, the socket write, the scheduler or the collector
separately, and calls the leftover unattributed to any named mechanism.
**Two mechanisms inside that remainder now carry a measurement**: eager
header materialisation and body accumulation.

**The figures live in `docs/benchmarks/README.md` under "Differential
decomposition, 2026-09-29" and are deliberately not restated here.** A
comparison whose size depends on which readings each side draws from
should have one home; this file learned that when a single amplification
sentence was copied into seven satellite records and was wrong in all
seven at once.

**None of the four mechanisms named above is measured individually.** The
new series measures header materialisation, which is part of what
`read_request` does rather than the whole of it, and body accumulation.
The socket write, the scheduler and the collector remain unmeasured, and
so does the intrinsic parse except as a subtracted baseline.
**[2026-09-30: the collector is now measured, so one of the four is —
see "AMENDMENT 2026-09-30" below. The socket write, the scheduler and the
intrinsic parse remain unmeasured.]**

### The empty-store control was re-derived, and it moved across the line

This file's own instruction was followed rather than its figure quoted.
The "Absolute criterion, re-measured" section above says of the
105.3–115.1 microsecond control that "it rests on a quantity that moves
by more than the margin it has left, so a later reader should re-derive it
rather than quote it forward."

Re-derived on 2026-09-29, same route, same parameters, on a binary of the
**same 690,688 bytes** this file records for the post-fast-path build:
**10074.1, 10400.4, 10230.4 and 10245.8 req/sec** across four
fresh-process readings — **96.15 to 99.26 microseconds per request. All
four clear 10k req/sec and all four sit below the 100-microsecond line.**

The two readings above, 8688.1 and 9501.0, do not. **Pooled, the control
spans 8688.1 to 10400.4 req/sec, a 1.20x spread that straddles the
criterion**, while the two sessions' ranges do NOT OVERLAP -- 8688.1 to 9501.0
against 10074.1 to 10400.4 -- so the movement is between sessions
rather than within one. (Within-session spreads are 1.09x for the
earlier pair and 1.03x for this session's four. An earlier draft cited
only the 1.03x and called it "within-session spread", generalising one
session's figure to both -- contradicted by the 1.09x these same records
already state.)

**The consequence for the claim this control carries.** This file, and
`CHANGELOG.md`'s `[0.2.0-alpha.4]` entry, and
`docs/superpowers/specs/2026-09-11-bun-ratio-design.md` section 2, all
record that the absolute criterion is not reachable by response-path work
alone, because the empty-store control already exceeds the gate's whole
100-microsecond budget. **On this session's readings it does not exceed
it.** The claim is therefore **neither supported nor refuted by this
control any longer** — which is not the same as the claim being refuted,
and is exactly the outcome the sentence quoted above anticipated.

**None of this is a gate figure.** `examples/05-json-api` serving an EMPTY
store is not the gate's workload; the gate's own measured figure remains
the seeded ten-user series above, unchanged by this amendment. Nothing
here claims the gate is met, reachable, or unreachable.

## AMENDMENT 2026-09-30: the collector is measured, and removing it does not reach the gate

The 2026-09-29 amendment above lists the collector among the mechanisms
that "remain unmeasured". **It now carries a measurement, taken on this
example at the gate's own workload**: ten seeded users, a 604-byte `/users`
body, 200 connections, one fresh server process per reading.

**The headline: disabling collection raises throughput from 2981.1–3410.3
to 4892.4–5561.1 req/sec. The ranges do not overlap.** Serial per-request
cost falls from 293.2–335.4 to 179.8–204.4 microseconds, a differential of
**88.8 to 155.6 microseconds per request** and a throughput share of
**30% to 46%**. **Even with no collection at all, 179.8–204.4 microseconds is
still 1.8x–2.0x over the 100-microsecond budget, so the collector alone does
not close the gate.**

### How it was measured

The runtime was built with two additions that are **not on `main`**, so these
figures cannot be reproduced from a checkout without re-applying them (the
patch is below): a per-cycle wall-clock timer printed under
`NOVA_GC_DEBUG`, and a `NOVA_GC_THRESHOLD` override of the 1 MiB initial
collection threshold. A threshold of 10^11 bytes disables collection for any
run that fits in memory. Three things support "disabled", and none is
conclusive alone: the arithmetic (no run here allocated anywhere near
10^11 bytes); one such run with `NOVA_GC_DEBUG` set logging **no**
collection lines (weak on its own, because that flag prints only inside a
collection, so a run without it logs nothing either); and a working set of
about 2.5 GB against about 16 MB with collection on.

Seeding is ten `POST /users`; the load is
`nova-bench-http --path /users --connections 200`, `--duration 5 --warmup 1`
for the short series and `--duration 15 --warmup 5` for the long one. Every
reading below had `errors=0` and a 604-byte body. Nine readings' result
lines were read from the terminal and not saved to a log: the fourth
disabled reading, the six in the instrumentation control, and the two
warmup runs below.

| binary | bytes | what it is |
|---|---|---|
| instrumented | 693,760 | this amendment's arms |
| `main` at `d029d71` | 690,688 | control, same size this file records for the post-fast-path build |

**No effect of the instrumentation shows on the default arm.** Alternated
in one script, 5 s each: `main` 2828.8, 3193.6, 3310.2; instrumented
3089.8, 3201.2, 3146.9. The ranges overlap. Three readings each can show
that no large effect exists, not that none does.

### Collection disabled against default — 5 s

| arm | readings (req/sec) |
|---|---|
| default, 1 MiB threshold | 2981.1, 3410.3, 3300.8 |
| collection disabled | 5064.7, 5414.8, 5561.1, 4892.4 |

The first three readings of each arm were alternated. The fourth disabled
reading ran about 20 s after them, outside the alternation, and carried
`NOVA_GC_DEBUG`. Across all four disabled readings the working set reached
**2,361,708–2,650,084 K** as `tasklist` reports it, in about six seconds,
against 15,644–16,096 K for the default arm. This is why the series is
short: a 20-second disabled run would not fit in this host's free memory.

**What the differential is a differential of.** The disabled arm skips
marking, sweeping *and* every `free()`, and pays first-touch page faults on
about 2.4–2.7 GB of fresh memory instead. So 88.8–155.6 microseconds prices "the
collector plus freeing" against "never reusing memory". It is not the mark
phase alone.

### The cost scales mostly with bytes allocated, not with how often collection runs

Long series, alternated, 15 s each:

| arm | req/sec | collections | per collection | total collector time |
|---|---|---|---|---|
| default, 1 MiB | 3289.0, 3227.9, 3127.3 | — | — | — |
| default, timer on | 3260.5, 3258.6, 3074.5 | 818–850 | 11.8–12.4 ms mean | 10.02–10.21 s |
| 256 MiB threshold, timer on | 3289.8, 3235.3, 3259.5 | 9–10 | 874–917 ms mean | 7.87–9.17 s |

Cutting the collection count 82- to 94-fold cuts total collector time by
only 8.5% to 23%, and throughput does not move. The two total-time ranges do
not overlap, so a part of the cost does depend on collection count. Per
byte freed, the collector cost 3.89–4.14 ns at the 1 MiB threshold and
3.26–3.42 ns at 256 MiB. **Most of the cost follows the garbage, and the
garbage is set by what the program allocates. So raising the threshold is
neither a fix nor a control.** One timed run freed 2.55 GB over its life.

The totals cover the whole process life, including seeding, the warmup and
the measured window, about 20 s of load in all. **The timer's ~10 s is
therefore roughly half the process's time**, which is more than the 30–46%
the differential implies. The two are different quantities, and three
candidates for the gap are known and untested: the timer includes the
collector's own logging; the disabled arm pays for page faults; and the
timer's share comes from 20-second runs, the differential from 6-second
runs whose live set was not recorded, and each collection costs more once
the live set takes its second step (below). The gap is recorded and not explained.

**Bytes allocated per request are not measured.** The generator does not
report the warmup's request count, so the denominator is unknown. The
working-set figures put it in the tens of kilobytes per request, and that is
an inference. **[Later on 2026-09-30: measured, with no warmup so the
denominator is known -- about 38.3 KB in 963 objects per ten-user request
on the collector's heap, on the post-fix `6240a05` build rather than the
build this paragraph describes. See "AMENDMENT 2026-09-30
(alloc-per-request)" below.]**

### The live set's second step follows the generator's warmup boundary

In the timed default runs the live set grows in two steps, not steadily.
It climbs from about 2,080 objects to about 10,700 by the eighth
collection and holds there. Then, within three collections (collections
349–364, depending on the run), it jumps to about 22,300 (1.94 MB) and
holds there for the rest of the run. The per-collection time follows it:
the first three collections of each run took 2.3–3.7 ms, the last three
12.2–19.7 ms.

`nova-bench-http` runs the warmup and the measurement as two separate
`run_load` calls. So when the warmup ends, 200 connections close and 200
new ones open. **Moving that boundary moves the step**, in one timed run
per arm, all 200 connections and 20 s of load except where noted:

| `--warmup` | second step, by bytes freed so far | final live objects |
|---|---|---|
| 0 (15 s of load) | none | 10,713 |
| 5 | 0.67 of 2.55 GB, 26% | 22,296 |
| 10 | 1.27 of 2.44 GB, 52% | 22,329 |

The warmup boundaries sit at 25% and 50% of the load's duration, so the
step lands at the boundary if the allocation rate is steady, which is
assumed rather than measured.

**So the objects added at the boundary stay live for the rest of the run,
and no collection reclaims them.** About 11,600 objects for 200 closed
connections is about 58 per connection. This locates the growth; it does
not name what holds those objects. Retained per-connection or per-task
state and false roots from the conservative stack scan are both
candidates. **[Later on 2026-09-30: identified as retained per-task state
-- see "FURTHER AMENDMENT 2026-09-30" below.]** **It bears on the gate's
own figures**: every procedure these records give for a ten-user
reading carries `--warmup 5`, and the
generator's default is 1 s. So if those runs behaved as these did, their
measured windows sat on the upper plateau. Whether that depresses them is
not measured here. The one no-warmup reading, 3099.3 req/sec, sits
inside the 3074.5–3260.5 range of the timed 5 s-warmup readings, which
cannot show a small effect. **This
bears on "Whether process age is the collector", below, and does not
settle it.**

### What this does not settle

- **How much of the cost is marking, sweeping, or `free()`.** The timer
  covers the whole cycle.
- **Whether allocating less would recover the differential.** It is the
  obvious reading of the scaling result, and it is untested.
- **The socket write, the scheduler and `read_request`'s intrinsic parse**,
  which remain unmeasured.
- **Anything about other hosts.** One host, Windows, three or four readings
  per arm.

### The instrumentation, for reproduction

Applied to `crates/nova-runtime/src/gc.rs` at `d029d71`:

```diff
+fn thresh() -> usize {
+    static T: OnceLock<usize> = OnceLock::new();
+    *T.get_or_init(|| {
+        std::env::var("NOVA_GC_THRESHOLD")
+            .ok()
+            .and_then(|v| v.parse().ok())
+            .unwrap_or(INITIAL_THRESHOLD)
+    })
+}
+
@@ fn maybe_collect(incoming: usize) {
-        h.alloc_since_gc + incoming >= h.next_gc
+        h.alloc_since_gc + incoming >= h.next_gc.max(thresh())
@@
 fn collect() {
+    let t0 = std::time::Instant::now();
+    collect_inner();
+    if debug() {
+        let ns = t0.elapsed().as_nanos();
+        let (n, live) = HEAP.with(|h| {
+            let h = h.borrow();
+            (h.objects.len(), h.live_bytes)
+        });
+        eprintln!("nova-gc-time: ns={ns} objects={n} live={live}");
+    }
+}
+
+fn collect_inner() {
@@ fn collect_with_roots(roots: &[usize]) {
-        h.next_gc = std::cmp::max(INITIAL_THRESHOLD, h.live_bytes.saturating_mul(2));
+        h.next_gc = std::cmp::max(thresh(), h.live_bytes.saturating_mul(2));
```

## FURTHER AMENDMENT 2026-09-30: what holds the closed connections' objects

The amendment above located the live set's second step at the load
generator's warmup boundary and left its holder unnamed. **The holder is
the executor's own root on each finished connection task.** Nothing ever
releases that root, because nothing ever joins the task.

### The mechanism, from the source

- `main` accepts each connection with `let h = spawn(serve(conn, store))`
  and never uses `h` again.
- `spawn_internal` in `crates/nova-runtime/src/task.rs` registers the
  task's state object with `gc::add_root`. The only matching
  `gc::remove_root` calls are in `take_output_internal` and
  `release_internal`. **Neither runs when a task completes.**
- **For a task created by `spawn`, the only Nova code that reaches either
  is `JoinHandle::join`** in `std/task/lib.nova`, which calls
  `task_release`. The one other route is `block_on`, which takes the output
  of the root task it spawns itself (`run_to_completion` in `task.rs`), and
  never of any other task. `std/task` offers no way to detach a task.
- So a task whose handle is dropped keeps its state object rooted, with
  everything that state reaches, for the rest of the process.
  `take_output_internal`'s own doc comment names this cost: "That is a
  leak, not unsoundness, and it is the deliberate trade". The trade is to
  release the root at take and accept that leak. The alternative it
  rejects, unrooting at completion, would free a heap-valued output while
  `Task::output` still names it.

### The measurement

Two scratch additions, neither on `main` (the patch is below). The first
prints the size of the collector's registered-root registry on each
`nova-gc-time` line. That registry also holds the runtime's other
registered roots, and none were present at these collections: the counts
below are fully accounted for by tasks. The second is a switch,
`NOVA_EXPERIMENT_RELEASE_AT_DONE`, that releases a task's root when the
task completes. **It is an experiment, not a fix. It provably breaks
`block_on`:** `take_output_internal` asserts that the root has not already
been released, so a `block_on` root task that completes would panic.
This server's `main` never completes, so it never trips that. Whether
releasing at completion is otherwise sound is not settled here. `join`
reads the output through the handle's own future rather than from the
executor, which suggests it is, and nothing here tests it.

One run per arm, `--warmup 5 --duration 15`, `NOVA_GC_DEBUG` on, binary
694,272 bytes. The result lines, `errors=0` in each, and the choice of
binary were read from the terminal and not saved. The logs corroborate the
binary: only this build prints `pinned=`.

| arm | registered roots, before → after the warmup boundary | live objects, before → after | req/sec |
|---|---|---|---|
| as shipped, 200 connections | 212 → **412** | 10,705 → 22,300 | 2937.0 |
| as shipped, 50 connections | 62 → **112** | 3,350 → 6,255 | 3307.2 |
| released at completion, 200 connections | 201 → 201 | 9,887 → 9,887 | 3238.9 |

- **One root per closed connection.** The root count rises by the number
  of connections the generator opens per phase, 200 and 50. That the old
  set closed there is taken from the generator: its warmup joins all of its
  connection threads before the measurement opens fresh ones.
- **About 58 objects per retained task, at both connection counts.**
  11,595 objects over 200 is 58.0, and 2,905 over 50 is 58.1.
- **The starting counts fit the same account.** 212 is the `main` task, the
  200 open connections, and the 11 connections the seeding step opens (ten
  `POST`s and one `GET`). Those 11 had already finished and were still
  registered at the first collection. 62 is 1 + 50 + 11. With the switch
  on, the count is 201, the `main` task plus the 200 open connections.
- **Releasing at completion removes the second step, and the old tasks'
  state is actually freed.** With the switch on, the live set falls at the
  boundary from its 9,887 plateau to 1,889 objects by collection #360, as
  the finished tasks' state is collected. It then rebuilds as the new
  connections open. For collections #360–#418 only 130–134 of the new
  connections were registered (131–135 roots). From #419 all 200 were, and
  by #425 the live set was back at about 9,900 objects. It held there until
  the last three collections, where the measured window's own connections
  close. The as-shipped 200-connection run shows the same two-stage
  opening, 130 connections and then 70. Why the generator's second set of
  connections opens that way is not examined.

**The object counts do not show two further tables that grow.** Both are
Rust-side and outside the collector's heap:

- `TASKS` is a `Vec` indexed by task id. It gains an entry on every spawn,
  joined or not, and no code in `task.rs` removes one.
- `BY_STATE` drops an entry only when the collector frees that state
  object, so it keeps one entry per task whose root is never released.

### What this does not settle

- **Whether the retention costs throughput.** **[Later on 2026-09-30: measured
  -- it does; see the amendment below.]** The req/sec column is one
  reading per arm. This file records a 1.66x spread across runs of one
  workload, so the 2937.0 against 3238.9 difference establishes nothing.
- **Whether it explains "Process age costs 1.29× to 1.38×", below.** An
  aged process has accepted more connections, so it holds more retained
  roots. That is consistent with the effect and does not establish it.
- **What a fix looks like.** Releasing the root the moment a handle is
  dropped or detached is unsound, because a parked task's state would then
  be swept. A detach has to mark the task, so that the executor releases
  it at completion, or at once if the task is already done. That is a
  design question and is not taken up here. **[Later on 2026-09-30: fixed
  on branch `release-spawned-task-roots`, without a detach -- see the
  amendment below.]**

### The patch, for reproduction

Applied on top of the "AMENDMENT 2026-09-30" instrumentation above. In
`crates/nova-runtime/src/gc.rs`, the `nova-gc-time` line gains the
registered-root count:

```diff
+        let pinned = PINNED.with(|p| p.borrow().len());
-        eprintln!("nova-gc-time: ns={ns} objects={n} live={live}");
+        eprintln!("nova-gc-time: ns={ns} objects={n} live={live} pinned={pinned}");
```

In `crates/nova-runtime/src/task.rs`, at the end of `poll_one`:

```diff
     wake_tasks_waiting_on(id);
+    // SCRATCH EXPERIMENT ONLY -- breaks `block_on`'s take of its root task.
+    if std::env::var_os("NOVA_EXPERIMENT_RELEASE_AT_DONE").is_some() {
+        release_internal(id);
+    }
 }
```

## AMENDMENT 2026-09-30 (release-spawned-task-roots): the retained roots are released

The FURTHER AMENDMENT above found each finished connection task's GC root
retained for the life of the process, because nothing joins the task.
Branch `release-spawned-task-roots` makes the executor release a spawned
task's root when the task completes. Only `block_on`'s own root keeps
release-at-take. **Measured before against after: the retention is gone,
and ten-user throughput rose by 11% to 18%, with the ranges disjoint. The
gate is still not met.**

### What was measured

Four release binaries, all built fresh for this series. Sibling names were
deleted before each build, and each binary's contents were checked for the
strings its build must and must not carry.

| binary | bytes | built from | carries |
|---|---|---|---|
| `before` | 690,688 | `a598d18` | — |
| `before-i` | 693,760 | `a598d18` + the instrumentation | timer, `NOVA_GC_THRESHOLD`, `pinned=` |
| `after` | 691,712 | this branch | the fix |
| `after-i` | 694,784 | this branch + the instrumentation | the fix, timer, `NOVA_GC_THRESHOLD`, `pinned=` |

The instrumentation is the `gc.rs` part of the "FURTHER AMENDMENT
2026-09-30" patch above. It is not on `main`, and none of the four
binaries contains the experiment switch.

Ten seeded users, a 604-byte `/users` body, 200 connections,
`--warmup 5 --duration 15`, one fresh server process per reading. The
twelve readings ran in one script, in the order uninstrumented before,
instrumented before, uninstrumented after, instrumented after, repeated
three times. Every reading had
`errors=0` and a 604-byte body, and every result line and server log was
saved.

### Throughput

| arm | before | after |
|---|---|---|
| uninstrumented | 3394.0, 3479.0, 3487.3 | 3886.4, 3918.7, 4008.8 |
| instrumented | 3454.3, 3550.8, 3453.8 | 3751.7, 3842.4, 3816.2 |

**The uninstrumented ranges do not overlap**, and neither do the
instrumented ones. Serial per-request cost falls from 286.8–294.6 to
249.5–257.3 microseconds, which is 29.4 to 45.2 microseconds per request,
or 1.11x to 1.18x the throughput.

**The gate is still not met.** 3886.4–4008.8 req/sec is 2.5x–2.6x short
of 10k. **No comparison with earlier series is made.** The
"FURTHER AMENDMENT 2026-09-12" series served a 534-byte body, not 604, and
this file's other 604-byte series from 2026-09-30 used different durations
or instrumented builds. The comparison that counts is the alternated one
within this session.

### The retention

From the instrumented runs, reading the collector's log at its 100th
collection (inside the warmup) and at 75% of the run (well after the
boundary):

| | before | after |
|---|---|---|
| registered roots, warmup | 212 (all three) | 201 (all three) |
| registered roots, after the boundary | 412 (all three) | 201 (all three) |
| live objects, warmup | 10,700–10,703 | 9,887–9,899 |
| live objects, after the boundary | 22,301–22,305 | 9,887–9,899 |
| collections | 880–931 | 1,588–1,611 |
| total collector time | 9.88–9.93 s | 9.20–9.27 s |

**The spec's three success criteria, each met in all three runs:**

- **Met:** after the boundary, the root count is 201 (the `main` task plus
  the 200 open connections), against 412 before.
- **Met:** the live set after the boundary holds at 9,887–9,899 objects,
  against 22,301–22,305.
- **Met:** the 11 seeding connections no longer appear. The count during
  warmup is 201, not 212.

Every root-count value each run logged, with how many collections showed
it:

| run | values (count × collections) |
|---|---|
| before 1 | 212 × 364, 269 × 1, 401 × 18, 412 × 497 |
| before 2 | 212 × 393, 412 × 530 |
| before 3 | 212 × 413, 238 × 1, 367 × 19, 412 × 498 |
| after 1 | 200 × 3, 201 × 1,600 |
| after 2 | 135 × 64, 198 × 1, 199 × 1, 200 × 2, 201 × 1,543 |
| after 3 | 148 × 51, 198 × 3, 199 × 2, 200 × 2, 201 × 1,530 |

The after runs' 135 and 148 are the boundary itself: the old connections'
roots are gone and not all of the new ones are open yet, the generator's
two-stage reopening this file records above. The 198–200 readings come as
the measured window's own connections close at the end.

**The smaller live set means more collections, not fewer.** The next
threshold is twice the live bytes, so a smaller live set crosses it
sooner. Each collection is cheaper, and the total collector time falls by
6% to 7%, disjoint.

**In the timed arm, the collector accounts for the whole gain.** That arm
rose 5.7% to 11.3% (3751.7 over 3550.8, and 3842.4 over 3453.8). Splitting
each timed run's log at its first root-count change, which falls at the
warmup boundary, and dividing by the measured window's request count:

| | before (3 runs) | after (2 runs) |
|---|---|---|
| collector time after the boundary | 7.475–7.566 s | 6.821–6.870 s |
| collector time per request | 139.7–145.3 µs | 117.8–119.5 µs |
| everything else per request, (15 s − collector) ÷ requests | 140.6–143.3 µs | 141.2–141.5 µs |

The third after run is excluded: its root count never changes at the
boundary, so the split cannot be placed. **The cost outside the collector
does not move; the collector's per-request cost falls by 14.5% to 18.9%.**
Two approximations sit in this split: the first root-count change only
approximates the start of the 15-second window, and the whole-run figures
(9.88–9.93 s against 9.20–9.27 s) cover warmup too.

The untimed arm's larger gain, 11.4% to 18.1%, is not accounted for here.
One difference between the arms is known: the instrumented after-build
logs two lines per collection on 1.71 to 1.83 times as many collections, so
logging costs the timed arm more after the fix than before. Whether that
explains the gap is not measured.

### What this does not settle

- **One host, Windows, three readings per arm.**
- **Why the untimed arm gained more than the timed one.**
- **`TASKS` growth.** The executor's `Vec` still gains one 32-byte entry
  per spawn, joined or not, and the payload-slot table grows the same way.
  Neither is on the GC heap, so neither appears in these counts.
- **Anything about the gate beyond this workload.** The gate is measured,
  and still not met.

## AMENDMENT 2026-09-30 (alloc-per-request): what one request allocates on the GC heap

The amendment above found the collector still costing 117.8–119.5
microseconds per request after the retained roots were fixed, and this
file has found its cost following bytes allocated. **This measures the
allocation itself, on the collector's heap: one ten-user request allocates
about 38.3 KB in 963 objects through `gc::alloc`, and three quarters of
those objects are 16 bytes or smaller.** Rust-side allocations are not
counted, for example `try_read`'s read buffer in `net.rs` and the
executor's `TASKS` entries. Sizes are as `gc::alloc` records them, after its
8-byte floor.

### How it was measured

Scratch instrumentation, not on `main`: cumulative counters in `gc::alloc`
of bytes and objects allocated, plus a five-bucket size histogram, printed
on each collection's `nova-gc-time` line. Its patch is below. The binary is
695,296 bytes, built from `main` at `6240a05` plus the instrumentation. The
size is consistent with a release build; no build log was kept.

- **Load:** 200 connections, `--duration 15 --warmup 0`, one fresh server
  process per reading, every reading `errors=0`.
- **Why no warmup:** all load then falls in the generator's counted window,
  so the denominator is the count the generator reports.
- **Per request:** the last collection's cumulative totals divided by that
  count.

**The division is not exact:**
- The denominator leaves out the seeding requests: ten `POST`s and one
  `GET` in the seeded arms, one `GET` for the empty store. The generator
  counts the request in progress when it stops, because it checks its stop
  flag only between round trips, and every reading had `errors=0`.
- The numerator leaves out whatever was allocated after the last
  collection, at most one collection threshold: 1.4–2.2 MB here, against at
  least 0.65 GB, so 0.21% or less.
- The numerator includes startup, seeding, and setting up and tearing down
  the 200 load connections. No bound on those is derived. Empirically they
  are too small to see: two nine-header readings with 24,320 and 43,259
  requests agree to 0.007% in objects per request, so no fixed per-run
  offset shows.
- Within one script, the three readings of each arm agree to within 0.03%
  in objects per request, and to about 0.9% in bytes. The widest byte
  spread is the empty store's 5,900–5,954, which is one object per request
  landing in a different size class in one reading.

### Three arms, alternated, three readings each

| arm | body | bytes per request | objects per request |
|---|---|---|---|
| empty store | 2 B | 5,900–5,954 | 173.8 (all three) |
| ten users | 604 B | 38,330–38,372 | 963.0–963.1 |
| ten users + nine extra request headers | 604 B | 41,815–41,828 | 1,114.8–1,115.1 |

The extra headers are `x-pad-N: 0123456789abcdef`, the form the 2026-09-29
header sweep in `docs/benchmarks/README.md` uses, on top of the one header
the generator always sends.

Throughput varied from run to run (1605.8–2933.4 req/sec in the two
ten-user arms), while objects per request stayed within 0.03%. **Those
ranges understate the variation between separate scripts.** The
value-length runs below repeat two of these configurations and land just
outside them: 962.45 objects against 963.00–963.15 for ten users, and
1,115.50 against 1,114.81–1,115.12 with nine headers. The differences are
0.03% to 0.07% in objects, and 0.08% to 0.19% in bytes.

### The differences

- **Serving ten users rather than none adds 32,376–32,472 bytes and
  789.2–789.4 objects per request.** That is about 79 objects and 3.2 KB
  per user. Per byte of response body, it is about 1.3 objects and 54
  bytes. The two arms share the route and differ only in the store, so
  this is what the whole handler path allocates for ten users: reading the
  store, building the body, and writing it.
- **Each extra header adds 16.85–16.90 objects and 382.5–388.8 bytes.**
  Differences are paired at the widest extremes of each range.

**The per-header object count barely depends on the value's length.**
One reading each, ten users, varying only the nine extra headers' value:

| header value | objects per request | bytes per request |
|---|---|---|
| none (no extra headers) | 962.4 | 38,300 |
| 4 characters | 1,114.8 | 41,455 |
| 16 characters | 1,115.5 | 41,883 |
| 32 characters | 1,116.1 | 42,483 |

Measured against the no-extra-header row from the same script, each
header adds 16.93 objects at 4 characters, 17.01 at 16 and 17.08 at 32: up
about 0.15 per header over 28 characters. One object per character would
add 252 objects per request over that span; the logs show about 1.3.
**So the roughly 17 objects are close to a fixed per-header cost, not one
per character.** The small rise is about twice the gap between scripts,
from one reading per length, and is not examined further. Bytes grow by
about 4.1 per extra value character per header. Header *name* length was
not varied.

**This does not settle the 2026-09-29 allocation-count question.** That
series timed ten-header materialisation in isolation, in
`docs/benchmarks/profile-http-head.nova`, at 21.3–22.1 microseconds. That
time fell between two arithmetic predictions, 18 microseconds at two
allocations per header and 27 or more at three or more, and the series
recorded that neither count was established. The 17 here counts every
`gc::alloc` object the whole server makes per header, which is a wider
scope than materialisation. The two figures describe different things.

### Where the objects are, by size

Per request, from the first reading of each arm. The other two readings
agree to within about one object per bucket; the largest gap is 1.015
objects.

| arm | ≤16 B | 17–64 B | 65–256 B | 257–4096 B | >4096 B |
|---|---|---|---|---|---|
| empty store | 132.8 obj, 1,818 B | 17.0 obj, 597 B | 23.0 obj, 2,217 B | 1.0 obj, 1,321 B | 0 |
| ten users | 739.1 obj, 11,195 B | 92.0 obj, 3,666 B | 107.0 obj, 12,808 B | 25.0 obj, 10,661 B | 0 |
| ten users + nine headers | 888.3 obj, 13,247 B | 89.9 obj, 3,580 B | 107.9 obj, 12,996 B | 29.0 obj, 12,002 B | 0 |

**Objects of 16 bytes or smaller are 77% of a ten-user request's objects
but 29% of its bytes.** The 25 objects between 257 and 4,096 bytes carry
another 28% of the bytes. `gc::alloc` rounds every request up to at least
8 bytes, so the smallest bucket is 8–16 bytes. Which code allocates the
small objects is not measured here: the counters see sizes, not call sites.

### What this does not settle

- **Which code makes the objects.** The obvious next measurement is
  attribution by call site. Nothing here names a mechanism for the 79
  objects per user or the 17 per header. **[Later on 2026-09-30:
  attributed by function -- `stringify` of a string at about ten objects
  plus one per character (the per-character one is `Vec::get`'s `Some`),
  and all of the per-header objects inside `parse_request_head`. See "AMENDMENT 2026-09-30 (alloc-attribution)"
  below.]**
- **Whether fewer, larger objects would cost the collector less than the
  same bytes in many small ones.** This file has found collector cost
  following bytes. Whether object count matters separately is not
  measured.
- **One host, Windows, three readings per arm** (one per value-length
  variant).

### The instrumentation, for reproduction

Applied on top of the `gc.rs` part of the "FURTHER AMENDMENT 2026-09-30"
patch above, which is itself a delta on the first "AMENDMENT 2026-09-30"
patch (the timer and `NOVA_GC_THRESHOLD`): four fields on `Heap` (`total_bytes`, `total_objs`, and
five-slot `bucket_objs` and `bucket_bytes` arrays, all zero-initialised).
In `gc::alloc`, after `h.live_bytes += size;`:

```rust
        h.total_bytes += size as u64;
        h.total_objs += 1;
        let b = match size {
            0..=16 => 0,
            17..=64 => 1,
            65..=256 => 2,
            257..=4096 => 3,
            _ => 4,
        };
        h.bucket_objs[b] += 1;
        h.bucket_bytes[b] += size as u64;
```

The `nova-gc-time` line then also prints `total_bytes`, `total_objs`,
`bobjs` and `bbytes`. The harness seeds the store, or leaves it empty,
then runs `nova-bench-http --path /users --connections 200 --duration 15
--warmup 0`, with `--header "x-pad-N: <value>"` repeated for the header
arms.

## AMENDMENT 2026-09-30 (alloc-attribution): which functions make the objects

The amendment above counted about 963 GC-heap objects per ten-user
request, about 79 per user and about 17 per extra header. It left open
which code makes them. **Counted in isolation, one function at a time,
the answer has three parts:**

- **`std/json`'s `stringify` of a string** allocates about ten objects
  plus one per character. It accounts for about 55 of the 79 objects per
  user.
- **The per-character object is `Vec::get`'s `Some`.** One 16-byte object
  is allocated per call, and `stringify`'s string path calls it once per
  output character.
- **All of the ~17 objects per extra header, and about 80% of its bytes,
  are inside `std/http`'s `parse_request_head`.**

### How it was measured

The same scratch counters as the amendment above, which are not on `main`.
Fourteen small compiled Nova programs were used, plus four `stringify`
length variants. Each one:

- calls one function in a loop;
- folds a value derived from every result into a printed accumulator, so
  a skipped loop would show;
- where it needs the example's records and response-building functions,
  copies them verbatim from `src/main.nova`.

Objects per call is the last collection's cumulative count divided by the
loop's iterations. Each program ran once, with `NOVA_GC_DEBUG=1` and
`NOVA_GC_THRESHOLD=65536`, on the runtime source at `6f7d941` plus the
counters. The binaries are 470,016–526,336 bytes, consistent with release
builds. No build log was kept.

**How exact the counts are.** Each figure is (setup plus loop) divided by
the iterations, less an uncounted tail:

- **The tail:** allocations after a program's last collection go
  uncounted. The small threshold keeps that to one collection window, 64
  KiB or less, against at least 22 MB allocated by each program in the
  first table. That is 0.3% or less of the bytes, and it makes a figure
  read up to 0.26% low. The three probes further down allocate less, as
  little as 6.4 MB for `get`, so their figures can read up to about 1%
  low. `get` reads 0.993.
- **Setup:** it is counted too, and it can push a figure slightly high.
  Setup is at most about 1,250 objects, in the 604-byte `json_response`
  program, whose setup builds a ten-user body. That is 0.03 objects per
  call or less. `parse_offsets` at ten headers reads 1.00012 for this
  reason.
- **Integers:** if every iteration allocates the same number of objects,
  these bounds force the true per-call counts to the nearest integers
  (25, 73, 4, 793, 93, 93, 1, 1, 43 and 196 in the first table). That premise
  is an assumption.

### Per call

| function | iterations | objects per call | bytes per call |
|---|---|---|---|
| loop with no call (baseline) | 400,000 | under 0.33 (no collection ran) | under 2.7 |
| `stringify(String(s))`, `s` = `"User Number 5"` (13 characters) | 400,000 | 25.000 | 799.0 |
| `user_json(u)`, one user | 100,000 | 72.986 | 2,386.5 |
| `users_json(s)`, empty store | 400,000 | 3.990 | 55.9 |
| `users_json(s)`, ten users | 10,000 | 792.979 | 31,217.6 |
| `json_response(200, body).to_bytes()`, 2-byte body | 40,000 | 92.979 | 2,638.4 |
| `json_response(200, body).to_bytes()`, 604-byte body | 40,000 | 92.990 | 3,878.4 |
| `parse_offsets(head)`, 1 header | 400,000 | 0.999 | 103.8 |
| `parse_offsets(head)`, 10 headers | 400,000 | 1.000 | 392.0 |
| `parse_request_head(head)`, 1 header | 100,000 | 42.971 | 875.4 |
| `parse_request_head(head)`, 10 headers | 40,000 | 195.976 | 3,635.7 |

The baseline's bound follows from its never collecting: the first
collection comes at 1 MiB, so it allocated less than that in total.

**The heads are byte-identical to what the load generator sends** with
`--path /users`: `GET /users HTTP/1.1`, then `Host: nova-bench`, then for
the ten-header rows nine `x-pad-N: 0123456789abcdef` headers.

### The isolated figures reproduce the server's

- **Per user.** `users_json` at ten users minus the empty store is 78.9
  objects per user, against the server's 78.9. In bytes it is 3,116 per
  user. The server's difference also includes the larger response body:
  `json_response` at 604 bytes minus 2 bytes is 1,240 bytes, or 124 per
  user. Adding that gives 3,240, inside the server's 3,237.6–3,247.2.
- **Per header, objects.** `parse_request_head` at ten headers minus one
  is 17.0 objects per header, against the server's 16.85–16.90.
- **Per header, bytes.** It is 306.7 bytes per header, against the
  server's 382.5–388.8. **About 20% of the per-header bytes are outside
  `parse_request_head`.** The server reaches it through `read_request`,
  which grows its buffer with `concat` and calls `parse_offsets` a second
  time. `parse_offsets`'s array grows 32 bytes per header (103.8 to
  392.0). Neither of those was measured separately.
- **The rest of the request path.** Head parsing, `users_json` and the
  response sum to about 929 objects at ten users and about 140 for the
  empty store. The server measured 963 and 174, so **about 34 objects per
  request fall outside these three pieces in both arms.** They belong to
  reading, routing and writing, including `read_request`'s own work. That
  split is not measured.

### `stringify` of a string: about ten objects plus one per character

One run each, varying only the string:

| characters | objects per call | bytes per call |
|---|---|---|
| 1 | 10.991 | 199.8 |
| 13 | 25.000 | 799.0 |
| 17 | 29.999 | 1,195.0 |
| 26 | 38.996 | 1,491.9 |
| 52 | 65.997 | 2,869.9 |

The slope is 1.00 objects per character from 17 to 26 and 1.04 from 26 to
52. Below 17 the relation is not a straight line.

**Per user, then:** the name (13 characters, 25 objects) and the email (17
characters, 30 objects) make about 55 of `user_json`'s 73 objects. By
elimination, the other 18 go to the interpolation that assembles the
user's JSON. About 6 per user go to `users_json`'s own loop, the `Map::get`
and the growing output's concatenations. These per-user figures are
averages over the seeded store. Its tenth user's name and email are one
character longer, and ten users carry only nine commas.

**The per-character object is `Vec::get`'s `Some`, measured directly.**
`stringify`'s string path, `quote` in `std/json/lib.nova`, builds its
output as follows:

- it walks `s.chars()`;
- it pushes each character, plus the two quotes, into a `Vec<Char>`;
- `vec_chars_to_string`, a private function in the same file, reads the
  vector back with `b.get(i)` once per output character.

`Vec::get` returns `Some(self.data[i])` (`std/collections/lib.nova`), and
an enum value is a heap allocation. Three probes, each counted like the
table above:

| probe | iterations | objects per call | bytes per call |
|---|---|---|---|
| `v.get(i)` on a 13-element `Vec<Char>` | 400,000 | 0.993 | 15.9 |
| building a 13-element `Vec<Char>` with `push` | 100,000 | 4.996 | 271.8 |
| `"User Number 5".chars()` | 400,000 | 0.999 | 111.9 |

So each `get` is one 16-byte object, and `quote` makes one per output
character. That is the per-character slope, and it lands in the 16-byte
bucket that holds most of a request's objects. Building the vector by
`push` adds a few objects: its growth steps (capacity 4, then doubling). A
formula of 9 + n + the number of capacity steps needed for n + 2 elements
reproduces all five measured counts. That formula is fitted, not measured
term by term.

### Also measured

- **`json_response` plus `to_bytes` is about 93 objects at both body sizes
  measured** (2 and 604 bytes). The two differ by 0.01 objects and 1,240
  bytes, about 2.1 bytes allocated per body byte.
- **`parse_offsets` allocates one array at both header counts measured**
  (1 and 10). Its bytes, 103.8 and 392.0, fit 8 + 8 × 12 and 8 + 8 × 48. So
  every per-header object comes from what `parse_request_head` adds over
  the intrinsic.

### What this does not settle

- **Whether allocating less would move the gate.** The collector's cost
  has followed bytes allocated in this file's earlier series. Nothing here
  changes code or measures throughput. **[Later on 2026-09-30: one lever
  measured -- removing the per-character `Some` cut about 342 objects (14%
  of the bytes) per request and raised ten-user throughput 1.15x-1.36x,
  disjoint. The gate is still not met. See "AMENDMENT 2026-09-30
  (json-drain-no-option)" below.]**
- **The ~34 objects per request outside the three pieces**, and the ~20%
  of per-header bytes outside `parse_request_head`.
- **One run per program.** The earlier amendment found object counts
  repeating to within 0.03% within one script, and to within 0.07% between
  scripts.

### Reproduction

The programs that call the example's functions copy the example's `User`,
`Store`, `Store::create`, `user_json`, `users_json` and `json_response`
verbatim from `src/main.nova`. Two helpers go with them. `seeded(n)`
creates `n` users named `User Number i` with email `useri@example.com`.
`head_with(extra)` builds the head described above. Then:

```
fn main() {
    <setup>
    let mut acc = 0
    let mut i = 0
    while i < <iterations> {
        <body>
        i = i + 1
    }
    println("phase=<name> iters=<iterations> acc=${acc}")
}
```

The `<body>` of each program is:

- **`stringify`:** `acc = acc + stringify(String(v)).len()`
- **`user_json`:** `acc = acc + user_json(u).len()`
- **`users_json`:** `acc = acc + users_json(s).len()`
- **`json_response`:** `acc = acc + json_response(200, body).to_bytes().len()`
- **`parse_offsets`:** `acc = acc + parse_offsets(buf).len()`
- **`parse_request_head`:** the two nested matches from
  `docs/benchmarks/profile-http-head.nova`, folding `r.headers.len()`
- **`get`:** a `match v.get(i % 13)` that folds `+1` for `Some`
- **`push`:** a fresh `Vec<Char>`, pushed from `v0.chars()`, folding its
  `len()`
- **`chars`:** `acc = acc + s.chars().len()`

## AMENDMENT 2026-09-30 (json-drain-no-option): the per-character `Some` removed

The amendment above traced `stringify`'s per-character allocation to
`Vec::get`'s `Some`, allocated once per output character by `std/json`'s
`vec_chars_to_string`. This branch makes that function, and its sibling
`vec_to_array`, index the vector's backing array directly. The loop only
runs below `len`, so `get` could never return `None` there. **A ten-user
request now allocates about 342 fewer objects, and ten-user throughput rose
from 3098.1–3290.1 to 3798.1–4217.1 req/sec, with the ranges disjoint.
The gate is still not met.**

### Allocation, before against after

The same scratch counters as the amendments above, which are not on
`main`. Every figure is objects or bytes per call, or per request, counted
as described there.

**`stringify(String(s))`**, one run each, `NOVA_GC_THRESHOLD=65536`:

| characters | objects before → after | bytes before → after |
|---|---|---|
| 1 | 10.991 → 7.999 | 199.8 → 152.0 |
| 13 | 25.000 → 9.998 | 799.0 → 558.9 |
| 52 | 65.997 → 11.999 | 2,869.9 → 2,005.9 |

The drop is 3, 15 and 54 objects: exactly the output length, the
characters plus the two quotes. Bytes fall by 48, 240 and 864, which is 16
per object. The accumulated output lengths are identical before and after.

**The server**, ten users, 200 connections, `--warmup 0`, three fresh
processes each, alternated:

| | before | after |
|---|---|---|
| objects per request | 962.4–963.2 | 620.8–621.0 |
| bytes per request | 38,323–38,345 | 32,838–32,891 |

That is 341.5–342.4 fewer objects and 5,432–5,508 fewer bytes per request.
The prediction was 342. Per user, the name costs 13 + 2 and the email 17 +
2, which is 340 across ten users. The seeded store's tenth user has a name
and an email one character longer each, which adds 2.

The "before" counter binary is the one the "(alloc-per-request)"
amendment used, built from `6240a05`, 695,296 bytes. The runtime and
`std/json` there are identical to this branch's base; only documentation
has changed since. The "after" counter binary is 694,784 bytes.

### Throughput, before against after

Uninstrumented binaries: `main` at `6b076f8` (691,712 bytes) against this
branch (691,200 bytes). Ten users, a 604-byte body, 200 connections,
`--warmup 5 --duration 15`, one fresh process per reading, alternated,
three readings each. Every reading had `errors=0` and a 604-byte body.

| before | after |
|---|---|
| 3290.1, 3198.0, 3098.1 | 3798.1, 3960.1, 4217.1 |

**The ranges do not overlap.** Serial per-request cost falls from
303.9–322.8 to 237.1–263.3 microseconds. That is 40.7 to 85.6
microseconds per request, or 1.15x to 1.36x the throughput.

**The gate is still not met.** 3798.1–4217.1 req/sec is 2.4x–2.6x short of
10k.

**No comparison with earlier series is made.** This session's before
range, 3098.1–3290.1, sits below the post-fix 3886.4–4008.8 recorded
earlier today on a binary of the same 691,712 bytes. That is consistent
with the between-session movement this file has recorded several times,
and nothing here measures it. The comparison that counts is the
alternated one here.

### The collector, in the counter runs

The allocation runs above used the instrumented binaries, whose collector
log carries a per-collection timer. The throughput runs did not. From the
counter runs, ten users, `--warmup 0`:

| | before | after |
|---|---|---|
| req/sec | 3361.2, 2862.7, 3198.5 | 3806.7, 3881.4, 3805.7 |
| collector time per request | 136.6, 170.6, 148.9 µs | 113.8, 112.3, 112.8 µs |
| collections per request | 0.0209 | 0.0179 |
| time per collection | 6.54, 8.15, 7.13 ms | 6.35, 6.27, 6.30 ms |

- **Collections per request fell 14.4%**, in line with bytes allocated.
- **Time per collection fell too**, so collector time per request fell
  more than bytes did: 16.7%, 34.2% and 24.3% in the three alternated
  pairs.
- **In these runs the collector's drop is 64% to 72% of each pair's
  per-request saving.**

These runs are instrumented and include the collector's own logging. The
before arm is noisy: its second reading ran at 2862.7 req/sec, with the
longest collections. Why the time per collection fell, whether from fewer
objects to sweep or something else, is not measured.

### Correctness

**Behaviour is unchanged, by argument.** Old and new read the same slot of
the backing array under the same bounds check, and `get`'s `None` arm was
unreachable because the loop stays below `len`. The tests show only that
nothing detectable changed. All 15 `nova-cli` JSON tests pass. Two
mutations were run and restored:

- **`vec_chars_to_string` writing a space instead of the character** fails
  8 of those 15.
- **`vec_to_array` writing `Null` instead of the element** fails 2,
  `json_parse_values_run` and `json_round_trip_run`. That is the whole of
  the guard on `vec_to_array`.

Neither mutation changes an output's length, so the length checks in this
file's harnesses cannot tell the fix from mutation 1. The JSON tests,
which compare content, are what distinguish them.

**The full suite, once, showed one failure that did not reproduce.**
`repeat_array_negative_length_aborts` failed once: its program exited
non-zero with empty stderr, where the test expects "array length must
not be negative". The test uses no JSON. It passed 5 of 5 runs alone and
283 of 283 in two reruns of the whole `nova-cli` target. Its cause is not
known. A second full run of the workspace was clean: 1141 passed, 0 failed
and 8 ignored across 45 targets. The `nova-cli` target then passed 283 of
283 again on the final files.

### What this does not settle

- **The rest of the throughput gain.** About 28% to 36% of each counter
  pair's per-request saving is outside the collector, and it may include
  the removed call and `match` per character as well as allocation. That
  split is not measured.
- **The rest of the per-request objects.** About 621 remain. **[2026-10-01: 105
  of them removed by joining interpolations once -- see "AMENDMENT
  2026-10-01 (nary-interpolation)" below.]** The earlier
  attribution put about 93 in `json_response` plus `to_bytes`, 43 in head
  parsing, and the rest in `users_json` and the ~34 outside the pieces
  measured.
- **The parser.** `vec_to_array`, and the parser's string scanning, which
  also drains through `vec_chars_to_string`, run on request bodies. The GET
  path measured here does neither, so the effect on `POST` is not
  measured.
- **One host, Windows, three readings per arm.**

## AMENDMENT 2026-10-01 (nary-interpolation): interpolation joined once, not pairwise

The previous amendment left about 621 GC objects per ten-user request.
Probing what remained showed how strings are paid for:

- every runtime string is two objects, a 16-byte header plus a separate
  byte buffer;
- evaluating a string literal allocates one header;
- an interpolation of *n* parts made *n* − 1 pairwise concatenations, each
  a new two-object string copying everything built so far.

Branch `nary-interpolation` lowers every interpolation of three or more
parts to one heap array of the parts and one `nova_rt_str_concat_n`, which
copies each part once and builds one string. **The per-request saving was
predicted from the code before the change was measured, and the measurement
matches it: about 105 fewer objects per request.** **Ten-user throughput
rose from 3929.3–4071.1 to 4257.0–4770.4 req/sec,
with the ranges disjoint, and the gate is still not met.**

### The prediction, written before measuring

An interpolation of *n* ≥ 3 parts saves 2(*n* − 1) − 3 = 2*n* − 5 objects
per execution. That is *n* − 1 two-object concatenations replaced by one
array and one two-object result. Literal headers and conversions are
unchanged.

| site | parts | runs per request | saving |
|---|---|---|---|
| `src/main.nova` `user_json` | 7 (`{"id":`, id, `,"name":`, name, `,"email":`, email, `}`) | 10 | 90 |
| `std/http` `Response::to_bytes`, status line | 5 (`HTTP/1.1 `, status, a space, reason, CRLF) | 1 | 5 |
| `std/http` `Response::to_bytes`, header line | 5 (head, name, `: `, value, CRLF) | 2 (`content-length`, `content-type`) | 10 |
| every 1- and 2-part interpolation on the path | 1 or 2 | — | 0 |

**Predicted: 105 fewer objects per ten-user request, 621 → 516.** Head
parsing makes no interpolation and was predicted unchanged.

### Allocation, before against after

The same scratch counters as the amendments above, which are not on
`main`.

**Per call**, one run each, with `NOVA_GC_THRESHOLD=65536`. That makes a
collection run about every 64 KiB after the first, which always comes at
the fixed 1 MiB initial threshold, and so keeps the uncounted tail small:

| probe | objects before → after | bytes before → after | predicted objects |
|---|---|---|---|
| `user_json`'s 7-part interpolation alone | 17.990 → 8.989 | 392.8 → 226.7 | 18 → 9 |
| `user_json(u)` | 38.988 → 29.999 | 1,842.4 → 1,677.0 | 39 → 30 |
| `users_json(s)`, ten users | 450.969 → 361.002 | 25,744.5 → 24,078.2 | 451 → 361 |
| `json_response(200, body).to_bytes()`, 604-byte body | 92.984 → 77.995 | 3,878.4 → 3,524.9 | 93 → 78 |
| `stringify(String(s))`, 13 characters (control) | 9.998 → 9.998 | 558.9 → 558.9 | unchanged |

The accumulated output lengths are identical before and after for every
probe.

**Per request**, ten users, 200 connections, `--warmup 0`, three fresh
processes per build, alternated:

| | before | after |
|---|---|---|
| objects per request | 620.6–620.9 | 515.9–516.0 |
| bytes per request | 32,828–32,888 | 30,862–30,871 |

That is 104.8–105.0 fewer objects per request within each alternated
pair, or 104.6–105.1 across the ranges' extremes, against the predicted
105.

### Throughput, before against after

Uninstrumented binaries: `main` at `0e00190` (691,200 bytes) against this
branch (691,712 bytes). Ten users, a 604-byte body, 200 connections,
`--warmup 5 --duration 15`, one fresh process per reading, alternated,
three readings each. Every reading had `errors=0` and a 604-byte body.

| before | after |
|---|---|
| 4049.7, 3929.3, 4071.1 | 4770.4, 4583.9, 4257.0 |

**The ranges do not overlap, though the gap between them is narrow:**
4071.1 against 4257.0. Serial per-request cost falls from 245.6–254.5 to
209.6–234.9 microseconds. That is 10.7 to 44.9 microseconds per request,
or 1.05x to 1.21x the throughput. The gate is still not met: 4257.0–4770.4
req/sec is 2.1x–2.3x short of 10k.

### The collector, in the counter runs

| | before | after |
|---|---|---|
| collector time per request | 116.5, 117.5, 108.6 µs | 93.3, 93.2, 96.2 µs |
| collections per request | 0.0179 | 0.0168 |
| time per collection | 6.51, 6.57, 6.06 ms | 5.56, 5.54, 5.73 ms |

Collections per request fell 6.1%, in line with the 6.0% drop in bytes
allocated, and each collection got shorter too. In the three alternated
pairs, the collector's drop is 52% to 61% of the per-request saving
measured in those same counter runs. These runs are instrumented and
include the collector's own logging.

### Binaries

| binary | bytes |
|---|---|
| before | 691,200 |
| after | 691,712 |
| before, counters | 694,784 |
| after, counters | 695,808 |
| probes | 470,528–527,360 |

Before and after differ for every pair, checked with `cmp`.

### Correctness

Output is byte-identical by construction: the parts are lowered as before,
in the same order, and both runtime paths copy their bytes in part order.
It is checked byte for byte by a new fixture,
`tests/runtime/interpolation_nary.nova`, which covers interpolations of 3,
4, 5, 7 and 8 parts across every part type, an empty string, non-ASCII
text, a user `Display` type, parts with side effects, a loop, and an
`.await` inside a part. It runs normally and under `NOVA_GC_STRESS=1`, and
its output was predicted by hand and matched on the unchanged compiler
before the change. Existing tests also compare exact interpolation output:
`std/test`'s four-part `assert_eq` messages, and `user_json`'s seven-part
interpolation in `json_api_example_serves_its_routes`. The probes above
compare only lengths.

Three mutations were run and restored:

- dropping the last part fails the runtime test and the fixture;
- reversing the order fails both;
- applying the n-ary path to two parts fails the two-part MIR guard.

**Full workspace runs on this branch.** Four were run:

- at the lowering change: 1146 tests, 1141 plus 5 new, of which 1145
  passed, 1 failed and 8 were ignored;
- on the final code: 1146 passed, 0 failed, 8 ignored;
- twice after the whole-branch review added one more MIR test: once 1146
  passed, 1 failed, 8 ignored, and once 1147 passed, 0 failed, 8 ignored.

Both failures were a crash of a compiled `nova test` child: an access
violation, with empty stdout. **That crash family predates this change: base
`0e00190` shows it too.** From repeated runs of the `nova-cli` target:

| suite | runs | runs with any compiled-child failure | of which an access violation |
|---|---|---|---|
| base `0e00190` | 30 | 5 | 2 |
| this branch | 15 | 7 | 6 |
| this branch, skipping its two new tests | 15 | 3 | 3 |

The other failures are children that exited non-zero with empty stderr,
where an abort message was expected. Of three further branch runs outside
the table, one failed, with three tests crashing, all access violations.
The full run above is a fourth.

The harness binary of one crashing fixture (`tests/runtime/nova_test.nova`),
built by each compiler, ran 800 times under 8-way concurrency with no
access violation and identical exit codes per test. A further 400 runs
alone are not kept in a log. This checks one fixture's binary, not the
others that crashed in the suite.

**The branch's higher rate is consistent with its two new tests adding
load to the parallel suite**: with them skipped, the branch's rate fell to
the base's. That comparison ran in a separate series, and 7 of 15 against 3
of 15 is not statistically strong, so this is not established. If it is
the cause, it is a real cost of this change, which makes an existing flake
appear more often. The flake's own cause is not established either. It is
the anomaly `docs/adr/0008-attributes-and-test-isolation.md` §4 has recorded
as open since Phase 2.2e: a freshly linked binary producing no output at
all, with `0xC0000005`. That section now carries this branch's recurrence
tally.

### What this does not settle

- **The literal headers.** Each evaluated string literal still allocates
  one header. In the probe interpolation that is 4 of the 9 objects left.
- **Two-part concatenations**, such as `users_json`'s growing `out`, which
  still copy the whole prefix each time.
- **The access-violation flake's cause.**
- **One host, Windows, three readings per arm.**

## AMENDMENT 2026-10-01 (noncollector-cost): where a request's time goes

After the n-ary interpolation change, the collector costs about 93–96
microseconds of a ten-user request's roughly 220–226 in the counter runs.
So even a collector that cost nothing would leave about 126–130
microseconds, still over the 100-microsecond budget. This measures that
remainder. **Collection plus allocation take 118.3–136.1 microseconds per
request, over the whole budget on their own. Socket calls take 42.8–47.4.
Everything else, which is compiled Nova code, runtime helpers, the
scheduler and the profiling's own printing, takes 49.2–53.1. The server
is almost never idle.**

### How it was measured

Scratch timers in the runtime, never committed. Each adds nanoseconds to a
per-thread total, and the totals print on the collector's `nova-gc-time`
line:

- **Socket calls:** around the `std::io::Read::read` call in `net.rs`'s
  `try_read`, and the `std::io::Write::write` call in `try_write`. In
  effect that is the OS call. Reads and writes go into one counter.
- **Idle wait:** around the whole of `poll::wait`, the executor's
  readiness wait.
- **Allocation outside collection:** around one `gc::alloc` call in 16,
  skipping any sampled call during which a collection ran, then scaled by
  the allocation count.
- **The collector:** the per-collection timer from the "AMENDMENT
  2026-09-30" scratch patch, which accumulates only with `NOVA_GC_DEBUG`
  set.
- **Everything else:** wall time minus those four.

**The wait is counted inside the measured window only.** The timers run
for the whole process, but wall time is the load generator's 15-second
window, which starts after the store is seeded. By the first collection,
the wait total already held 0.98–1.17 s over 32–33 calls, which is idle
time while the seeding requests trickled in. That is subtracted. The other
categories do almost nothing during seeding, eleven requests' worth, and
are left whole.

**The allocation sample was calibrated.**
- A pair of `Instant::now` and `elapsed` calls costs 73.0–73.4 ns on this
  host, and the interval it reports around nothing averages 35.8 ns
  (35.7–35.8 per run). That bias is subtracted from each sample: the raw
  101.4–113.2 ns per allocation becomes 65.6–77.4 ns.
- The correction removes only the clock's own time. The bracket also
  contains the scratch counters and two extra reads of the collection
  count, so the corrected figure still overstates a plain build's
  allocation by a few ns.
- This clock ticks every 100 ns, which is longer than the operation being
  timed. Each sample therefore reads 0, 100 or 200 ns, and the average is
  meaningful only because samples land at random against the ticks.
- The socket timer carries the same 36 ns bias on about 3 calls per
  request, about 0.1 µs, which is left in.

Ten users, a 604-byte body, 200 connections, `--warmup 0 --duration 15`,
three fresh processes, every reading `errors=0`, `NOVA_GC_DEBUG` set. The
profiling server is 696,832 bytes. It is `main` at `c59d2eb` plus the
timers and the "(alloc-per-request)" counters, which carry an inactive
`NOVA_GC_THRESHOLD` override.

### Per request

| category | run 1 | run 2 | run 3 | share |
|---|---|---|---|---|
| wall time | 237.7 µs | 211.2 µs | 215.3 µs | 100% |
| collector | 96.2 | 84.5 | 86.1 | 40% |
| everything else | 53.1 | 49.2 | 50.0 | 22–23% |
| socket read and write calls | 47.4 | 42.8 | 43.7 | 20% |
| allocation outside collection | 39.9 | 33.8 | 34.6 | 16–17% |
| idle in the readiness wait | 1.06 | 0.98 | 1.00 | under 0.5% |

- **Socket calls:** 2.99 per request, at 14.3–15.9 µs each. The counter
  does not separate reads from writes, and each call's time includes any
  time the OS spent running other threads. The 200 load-generator threads
  share this host.
- **Allocation:** 515.8–516.1 objects per request, at 65.6–77.4 ns each.
- **Idle wait:** about 0.005 calls per request inside the window, averaging
  0.20–0.21 ms each. The server is idle less than 0.5% of the window.

**What the budget arithmetic says.** The gate allows 100 µs per request.
- Collection plus allocation is 118.3–136.1 µs. That is over the budget on
  its own, so it has to fall.
- With both removed, the rest sums to 92.9–101.6 µs, about the whole
  budget. So unless collection and allocation fall almost to nothing,
  socket calls or everything else have to fall as well.
- Idle time is not a lever.

**The profiling overhead is not resolved.** Plain `main` (691,712 bytes)
and the profiling build were alternated, three readings each, at the
standard `--warmup 5 --duration 15`:

| plain | profiling |
|---|---|
| 4834.7, 4971.7, 4641.0 | 4360.8, 4763.4, 4778.2 |

The ranges overlap. The per-pair ratios are 0.90, 0.96 and 1.03, and the
profiling mean is 3.8% lower. That cannot rule out an overhead of around
10%. The arithmetic predicts about 3 µs per request, mostly the sampled
timer pairs. **This comparison ran with `NOVA_GC_DEBUG` unset, while the
profile above ran with it set.** The collector's per-collection
`nova-gc-time` print sits outside every timer, so the profile's printing
lands in "everything else", and this check did not compare it.

### What this does not settle

- **What "everything else" contains**, beyond compiled code, runtime
  helpers, the scheduler, the profiling's printing, and time the OS spent
  on other threads.
- **Why a socket call costs about 15 µs**, and how the three calls per
  request split between reads and writes.
- **The plain build's exact allocation cost**, which is a few ns under the
  figure above.
- **One host, Windows, three readings.**

## AMENDMENT 2026-10-01 (gc-phase-cost): where one collection's time goes

The "(noncollector-cost)" amendment put the collector at 84.5–96.2
microseconds of a ten-user request, about 40%. This splits the collector's
time across its own phases. **About 70% of collection time is work over
every object on the heap, three quarters of which are about to be freed.
Sweeping takes 42–46% of collection time. Clearing the marks and
rebuilding the sorted index take 24–27%, most of that the sort.
Marking the survivors takes 27–28%. In the runs that kept their request
counts, the collector cost 93.8–101.6 microseconds per request, which is
about the whole 100-microsecond budget. Its largest phase, the sweep, was
42.9–46.2 of that, so no single phase is half the collector.**

### How it was measured

Scratch timers in `gc.rs`, never committed. Each adds nanoseconds or a
count to a per-thread total, and the totals print on a `nova-gc-phase`
line after every collection when `NOVA_GC_DEBUG` is set. The figures below
come from each run's last line, so they cover the whole process. Every
share is therefore a share of a run's total collection time; no single
collection's split was recorded. The totals include the eleven seeding
requests, under 0.02% of the requests in runs 4–6.

- **Whole collection:** around the entire body of `collect`.
- **Root scan:** copying the pinned registry, plus
  `nova_gc_collect_roots`'s scan of the stack and registers.
- **Clear marks**, **build the index** (the `(start, end, index)` vector),
  **sort the index** (`sort_unstable_by_key`), the **mark** loop including
  every binary search, and the **sweep** loop. Each is timed as a whole.
- **Inside the sweep**, every 16th freed object gets three `Instant::now`
  reads, bracketing `dealloc` and `task::forget_freed_state`. Each
  bracket's average is corrected by the 35.8 ns clock bias calibrated in
  "(noncollector-cost)", then multiplied by the freed count. This clock
  ticks every 100 ns, longer than either operation, so the averages are
  meaningful only because samples land at random against the ticks. One
  in 16 in sweep order is a systematic sample, not a random one.
- **Counts:** objects on the heap when the collection starts, objects
  freed, root words, and scanned words. Scanned words are those the mark
  loop reads from surviving objects flagged for scanning.

Ten seeded users, 200 connections, `--warmup 0 --duration 15`, one fresh
process per run, `NOVA_GC_DEBUG` set. Both sets ran the same harness
command with this binary. Only runs 4–6 kept the load-generator output,
which records the seed and the 604-byte body. The profiling server is
693,760 bytes. It is `main` at `c34420e`, whose runtime is unchanged from
`c59d2eb`, plus these timers and nothing else. The plain build of that
runtime is 691,712 bytes, as recorded in "(noncollector-cost)".

**There are two sets of three runs, from the same binary.** The first
set's load-generator output was discarded, so it has per-collection
figures but no request counts. The second set reran the same binary with
the output kept, and every one of its readings is `errors=0`. These
predictions were written before the second set:

| prediction | measured in runs 4–6 |
|---|---|
| shares within about 3 points of the first set | not throughout: sort is 1.6–3.7 points below the first set's runs, sweep 0.3–3.7 above |
| sweep 42–45% | 45.5–45.7%, slightly above |
| mark 27–28% | 27.4–27.7% |
| sort 23–24% | 20.5–21.3%, below |
| build about 2–3%, clear under 1%, root scan about 0.1% | 2.0–2.4%, 0.8–0.9%, 0.1–0.2% |
| 4.8–5.6 ms per collection | 5.59–6.05, two runs above |
| 1100–1300 collections per run | 1071–1150, one run below |
| collector 84–96 µs per request | 93.8–101.6, one run above |
| 4200–4800 req/sec | 4231.1–4548.4 |

### Per collection, all six runs

| | run 1 | run 2 | run 3 | run 4 | run 5 | run 6 |
|---|---|---|---|---|---|---|
| collections | 1135 | 1275 | 1252 | 1071 | 1146 | 1150 |
| ms per collection | 5.60 | 4.82 | 5.03 | 6.05 | 5.69 | 5.59 |
| sweep | 45.2% | 42.0% | 43.7% | 45.5% | 45.5% | 45.7% |
| mark | 26.7% | 27.5% | 27.1% | 27.5% | 27.7% | 27.4% |
| sort the index | 22.9% | 24.2% | 23.4% | 20.9% | 20.5% | 21.3% |
| build the index | 2.0% | 2.6% | 2.4% | 2.4% | 2.4% | 2.0% |
| clear marks | 0.6% | 0.7% | 0.6% | 0.8% | 0.9% | 0.8% |
| root scan | 0.1% | 0.1% | 0.1% | 0.1% | 0.2% | 0.1% |
| outside every phase | 2.4% | 2.8% | 2.6% | 2.8% | 2.9% | 2.6% |

"Outside every phase" is everything inside the whole-collection timer that
no phase timer covers. That includes the collector's own `nova-gc:` debug
print, the bookkeeping after the sweep, freeing the index and the work
list, and the phase timers' own reads and counter updates.

Averaged per collection, the heap looks the same in all six runs:
- 40,553–40,585 objects when it starts.
- 30,687–30,710 of them freed, which is 75.7%, leaving about 9,870.
- 81,289–81,370 words scanned, about 650 KB.
- 507–646 root words.

### Per request, runs 4–6

| | run 4 | run 5 | run 6 |
|---|---|---|---|
| requests | 63,720 | 68,195 | 68,467 |
| req/sec | 4231.1 | 4529.3 | 4548.4 |
| wall time per request | 236.3 µs | 220.8 µs | 219.9 µs |
| **collector** | **101.6** | **95.6** | **93.8** |
| sweep | 46.2 | 43.5 | 42.9 |
| — `dealloc` | 25.0 | 22.1 | 21.6 |
| — `forget_freed_state` | 8.8 | 6.8 | 7.2 |
| — rest of the sweep loop | 12.4 | 14.6 | 14.1 |
| mark | 28.0 | 26.5 | 25.8 |
| sort the index | 21.2 | 19.6 | 20.0 |
| build the index | 2.46 | 2.33 | 1.87 |
| clear marks | 0.78 | 0.83 | 0.74 |
| root scan | 0.15 | 0.14 | 0.12 |
| outside every phase | 2.83 | 2.75 | 2.48 |

Each run made 59.5 requests per collection and freed 515.8 objects per
request. That is close to the 515.8–516.1 allocated per request in
"(noncollector-cost)". Allocations were not counted in these runs.

**What each unit costs.** Runs 4–6 are given first, runs 1–3 in brackets.
- **Sorting the index:** 28.8–31.1 ns per object (28.8–31.6).
- **Marking:** 18.8–20.4 ns per scanned word (16.3–18.4). Each nonzero
  word costs a binary search of the whole 40.5k-entry index, three
  quarters of which are objects about to be freed.
- **`dealloc`:** 41.9–48.5 ns per call after the correction, 77.7–84.3
  raw (26.0–35.2 corrected, 61.8–71.0 raw). It is one call into the
  system allocator per freed object.
- **`forget_freed_state`:** 13.1–17.0 ns per call after the correction,
  48.9–52.8 raw (12.2–22.1 corrected, 48.0–57.9 raw). It is one removal
  from a thread-local `HashMap` per freed object.
- **The rest of the sweep loop:** 18.2–21.4 ns per object visited, live
  ones included (19.0–21.5).

**The sampling's own cost is estimated, not measured.** Counting only the
clock reads, each sample makes three at about 36.5 ns each, half the
calibrated 73 ns pair. At 1,918 samples per collection, that is about
0.21 ms per collection, or about 3.5 µs per request. Correcting the
brackets moves it out of the `dealloc` and `forget_freed_state` figures.
It stays in the sweep total, all of it in "rest of the sweep loop", where
it is about 5 ns of the 18–21 per object. **That is a lower bound.** The
sweep also makes three thread-local counter updates per sample, and a
modulo test, three branches and a counter increment per freed object.
This build's collector range, 93.8–101.6, overlaps
"(noncollector-cost)"'s 84.5–96.2; only run 4 lies above it.

**The two sets disagree.** The second set is slower, with disjoint ranges,
in five measures:

| | runs 1–3 | runs 4–6 |
|---|---|---|
| marking, per scanned word | 16.3–18.4 ns | 18.8–20.4 ns |
| `dealloc`, per call | 26.0–35.2 ns | 41.9–48.5 ns |
| clearing marks, per object | 0.78–0.84 ns | 1.09–1.22 ns |
| root scan, per root word | 10.3–13.2 ns | 14.1–16.7 ns |
| outside every phase, per collection | 132–137 µs | 147–168 µs |

They overlap on the other four per-unit measures: sorting, building the
index, `forget_freed_state` and the rest of the sweep loop. Sorting is
nearly identical, 28.8–31.6 ns per object in the first set and 28.8–31.1
in the second. That is consistent with the sort's share falling from
22.9–24.2% to 20.5–21.3%. What changed between the sets is not known. The
shares in the headline span both sets.

### What the phases bound

- **Per-object work is 69.3–70.7% of collection time in all six runs.**
  That is the sweep, clearing marks, and building and sorting the index.
  Each of them runs over every object on the heap at collection time.
  About three quarters of those objects were allocated since the last
  collection; the rest are the previous collection's survivors. In runs
  4–6 this work costs 127–137 ns per freed object. Each object also costs
  65.6–77.4 ns when it is allocated ("(noncollector-cost)").
- **What that predicts, untested on this build.** Collecting less often
  would leave most of this work in place, because every new object is
  still cleared, indexed, sorted and swept at least once. What would fall
  is the work repeated on survivors, at every collection they live
  through. And the sort's cost per object would rise with the heap's size,
  since sorting is O(n log n). No threshold was varied here. The first
  "AMENDMENT 2026-09-30" finding agrees in direction: an 82- to 94-fold
  cut in collections cut collector time by only 8.5% to 23%. It does not
  confirm this split, because that build allocated differently, and
  marking alone is 27–28% here.
- **Marking is paid per collection, not per new object.** It reads the
  survivors, about 9,870 objects and 650 KB. Each nonzero word, root words
  included, searches the whole index, so its cost depends on the heap's
  total size as well. The survivor count did not vary across these runs,
  so how marking scales was not measured.
- **The remainders, runs 4–6.** Without clearing marks or the index
  rebuild, the collector would still take 71.3–77.2 µs per request.
  Without the sweep, it would still take 50.9–55.4. Neither fits the
  100-microsecond budget beside the 92.9–101.6 µs that
  "(noncollector-cost)" measured outside both collection and allocation.

### What this does not settle

- **Why the two sets differ.** Five measures are slower in the second set,
  ranges disjoint. The sort per object did not change.
- **What the ~81.4k scanned words per collection are**, meaning which
  surviving objects carry them.
- **The sampling's own cost.** The figure above is an estimate and a
  lower bound, not a measurement.
- **Whether `dealloc`'s cost depends on object size.** The sample is one
  in 16 in sweep order, not random.
- **One host, Windows, six readings in two sets.**

## AMENDMENT 2026-10-01 (gc-page-heap): small objects in size-class pages

The "(gc-phase-cost)" amendment found about 70% of collection time going
to work over every object on the heap. This change, ADR 0020, moves every
object of 2048 bytes or less into 64 KiB pages of fixed-size slots, marked
and swept by bitmap. **Ten-user throughput rose from 3983.0–4411.0 to
7559.7–9129.2 req/sec, ranges disjoint: 1.71x to 2.29x. That is
109.5–132.3 microseconds per request against the gate's 100, so the
absolute criterion is now short by 1.10x to 1.32x.** In the profiling
build the collector costs 11.6–12.9 microseconds per request, against
93.8–101.6 in "(gc-phase-cost)"'s profiling build. An allocation costs
29.8–34.9 ns, against 65.6–77.4 in "(noncollector-cost)". That earlier
bracket also held extra counter work, so not all of the drop is the
change. Peak working set fell from 12.0–12.2 MB to 9.3–10.0 MB.

### How it was measured

- **Binaries:**
  - `before`: 691,712 bytes, `main` at `331dec5`, built from its own
    worktree. That is the plain build's byte size recorded in
    "(noncollector-cost)".
  - `after`: 700,928 bytes, branch `gc-page-heap` at `2225e6c`.
  - The build commands deleted every sibling name before each
    `nova build -o`. They were run by hand and are not saved as a script.
- **Configuration:** ten users, a 604-byte body, 200 connections,
  `--warmup 5 --duration 15`, one fresh process per reading. Before and
  after were alternated, and every reading was `errors=0`.
- **Peak working set** is the process's `PeakWorkingSet64`, read through
  PowerShell just before the process is killed.

The predictions were written before anything was built:

| prediction | measured | verdict |
|---|---|---|
| before 4200–4800 req/sec | 3983.0–4411.0 | one reading below |
| after 5500–8000 req/sec | 7559.7–9129.2 | two readings above |
| collector 15–35 µs per request | 11.64–12.94 | below: smaller than predicted |
| allocation 15–40 ns per call | 29.8–34.9 | within |
| large path under 5 objects per request | 0 | within |
| peak working set after 0.8x–1.5x before | 0.775–0.815 per pair | two of three pairs below |

### Throughput and footprint

In run order:

| reading | build | req/sec | peak working set |
|---|---|---|---|
| 1 | before | 3983.0 | 12,230,656 B |
| 2 | after | 7559.7 | 9,969,664 B |
| 3 | before | 4411.0 | 12,017,664 B |
| 4 | after | 8658.7 | 9,318,400 B |
| 5 | before | 4308.5 | 11,988,992 B |
| 6 | after | 9129.2 | 9,314,304 B |

The after readings span 1.21x, wider than the before readings' 1.11x.
The three after readings rose in run order. With three readings, that
is not evidence of a trend, and its cause is not known.

### Where a request's time goes now

These figures come from a profiling build of 702,464 bytes: the `after`
runtime plus scratch timers, never committed. It ran
three fresh processes with `--warmup 0` and `NOVA_GC_DEBUG` set, and every
RESULT line was kept. The timers cover:
- the root scan;
- the large-object sort;
- marking;
- the page sweep;
- the large-object sweep;
- the state-map prune;
- the whole collection;
- one `alloc` call in 16, skipping any call during which a collection ran.

The allocation sample is corrected by the 35.8 ns clock bias calibrated in
"(noncollector-cost)".

| | run 1 | run 2 | run 3 | "(gc-phase-cost)", runs 4–6 |
|---|---|---|---|---|
| requests | 119,866 | 130,798 | 129,571 | |
| req/sec | 7972.0 | 8695.3 | 8618.4 | 4231.1–4548.4 |
| wall time per request | 125.4 µs | 115.0 µs | 116.0 µs | 219.9–236.3 µs |
| **collector** | **12.94** | **11.67** | **11.64** | **93.8–101.6** |
| — mark | 10.89 | 10.02 | 10.02 | 25.8–28.0 |
| — outside every phase, including the `nova-gc:` debug print | 1.81 | 1.39 | 1.37 | 2.48–2.83 |
| — page sweep | 0.099 | 0.121 | 0.104 | sweep 42.9–46.2 |
| — state-map prune | 0.083 | 0.078 | 0.077 | `forget_freed_state` 6.8–8.8, inside the sweep |
| — root scan | 0.062 | 0.060 | 0.064 | 0.12–0.15 |
| — large sort and large sweep together | 0.0017 | 0.0016 | 0.0016 | clearing, building and sorting the index 22.6–24.5 |
| allocation outside collection | 18.0 | 15.4 | 15.5 | 33.8–39.9 ("(noncollector-cost)") |

**Per collection:**
- A collection takes 705.6–784.0 µs, against 5.59–6.05 ms.
- It comes every 60.6 requests.
- Marking is 84.1–86.1% of it, at 6.61–7.18 ns per scanned word against
  18.8–20.4 ns.
- It scans 91,817–91,836 words, against 81,289–81,370, a rise of about
  12.9%. Marking now scans each slot's full size rather than the requested
  size, which adds rounding-tail words. Whether that accounts for the whole
  rise, or the survivors changed as well, is not known: this run counted
  neither the tail words nor the survivors.

**Per allocation:** 515.8–516.1 calls per request, at 65.6–70.7 ns raw
and 29.8–34.9 ns corrected. The bracket differs from "(noncollector-cost)"'s,
which also held two collection-count reads and that build's allocation
counters. That amendment said its figure overstates allocation by a few
ns, so part of the drop from 65.6–77.4 is the measurement, not the
change.

**The large path took no objects at all** in any of the three runs: zero
objects and zero bytes.

**What is left of the budget.** Collection plus allocation is now
27.1–31.0 µs of a 115.0–125.4 µs request in the profiling build.
**Subtracting them leaves 87.9–94.5 µs, which is close to the whole
100-microsecond budget on its own.** That figure is derived, not measured,
and it includes the profiling's own overhead:
- the per-collection `nova-gc-pg:` print, which sits outside the
  collection timer;
- the counter work on every allocation;
- the clock reads around the sampled one call in 16. By the calibrated
  pair cost that is about 2.4 µs per request, an estimate.

"(noncollector-cost)" split its own remainder, 92.9–101.6 µs on an earlier
build, into three parts:
- socket calls, 42.8–47.4;
- everything else, 49.2–53.1;
- idle wait, 0.98–1.06.

This amendment did not re-measure that split.

### What this does not settle

- **Why the peak working set fell.** Three things the old heap had are
  candidates. None was measured.
  - The 24-byte `Obj` record that each of about 40k objects per collection
    carried.
  - The 24-byte-per-object sort index each collection built, about 1 MB.
  - The system allocator's own overhead per object.
- **Why the after readings vary as much as they do** (7559.7–9129.2).
- **The profiling timers' own cost.** The profiling runs used
  `--warmup 0` with `NOVA_GC_DEBUG` set, and were not alternated with the
  plain runs. So their 7972.0–8695.3 req/sec cannot rule out an overhead
  of around 10%, inside a plain after range that is itself 1.21x wide.
- **What the remaining 87.9–94.5 µs is made of** on this build.
- **One host, Windows, three readings per arm.**

## AMENDMENT 2026-10-01 (remainder-split): what the rest of a request is made of

"(gc-page-heap)" put collection plus allocation at 27.1–31.0
microseconds of a ten-user request, and derived the rest, 87.9–94.5, by
subtraction. Here collection plus allocation is 25.9–28.7, and the rest
91.3–97.8. This measures 41.6–45.3 µs of that rest directly: the socket
calls and the readiness wait. The other 49.7–52.5 is still a subtraction.

**The response write is the costliest single item measured: one write per
request at 35.5–38.6 µs, 30.3–30.5% of the request.** Reads are cheap:
- a read that returns data (an `Ok` read) takes 3.6–4.0 µs in the OS call;
- a read that finds nothing waiting happens 0.98 times per request and
  takes 1.5–1.7 µs per request in the OS call, 1.3%. The park and re-poll
  it leads to are not measured separately.

**Everything else is 49.7–52.5 µs, 41.5–42.4%.** That is compiled Nova
code, runtime helpers, the scheduler, the profiling's own cost, and any OS
time spent on other threads.

### How it was measured

Scratch timers in the runtime, never committed. They add to the
"(gc-page-heap)" profiling build:

- **Socket calls:** around the `std::io::Read::read` call in `net.rs`'s
  `try_read`, and around the `std::io::Write::write` call in `try_write`.
  Each call's time and count is kept by outcome:
  - a read returning `Ok`, a read that would block, any other read. An
    `Ok` read of zero bytes, which is end of file, counts as `Ok`;
  - a write that completes, a partial write, any other write.
- **`try_read` and `try_write` as wholes**, and the allocation and zeroing
  of `try_read`'s 4096-byte buffer.
- **Readiness wait:** around the whole of `poll::wait`, with its call count,
  the number of sockets it was given and the number it returned.
- **The collector, and one `alloc` call in 16:** exactly as in
  "(gc-page-heap)". The allocation sample is corrected by the 35.8 ns
  clock bias. The socket and wait timers carry the same bias, about 0.1 µs
  per request across three calls, and it is left in.
- **Everything else** is wall time minus the socket OS calls, the readiness
  wait, the collector and allocation. The `try_read`, `try_write` and
  buffer timers overlap those and are not subtracted.

**Readiness wait during seeding is excluded.** The counters cover the whole
process, but by the first collection the wait total already held
0.939–0.959 s over 31–33 calls. That is mostly idle time while the eleven
seeding requests trickled in, before the load generator connected, so the
wait figure below subtracts the value at the first collection. The other
counters are left whole; seeding is under 0.01% of their requests.

Ten users, a 604-byte body, 200 connections, `--warmup 0 --duration 15`,
three fresh processes, every reading `errors=0`, `NOVA_GC_DEBUG` set. The
profiling server is 704,512 bytes: `main` at `00ccb48` plus the timers. The
plain build of that commit is 700,928 bytes.

These predictions were written before anything was built:

| prediction | measured | verdict |
|---|---|---|
| about 3 socket calls per request: 2 reads, 1 write | 1.000 `Ok` reads, 0.983–0.985 reads that find nothing, 1.000 writes | within |
| about 1 read per request finds nothing waiting | 0.983–0.985 | within |
| 10–16 µs per socket call | reads 1.6–4.0, writes 35.5–38.6 | wrong: the average fits, but the calls are nothing alike |
| socket total 30–48 µs per request | 40.6–44.3 | within |
| readiness wait under 2 µs per request | 0.94–1.03 | within |
| collector 11–13 µs per request | 11.60–11.98 | within |
| allocation 15–18 µs per request | 14.28–16.76 | one run below |
| everything else 40–60 µs per request | 49.7–52.5 | within |
| `try_read`'s 4096-byte buffer under 1 µs per request | 0.67–0.71 | within |
| wall time 110–130 µs per request | 117.2–126.5 | within |

### Per request

| | run 1 | run 2 | run 3 | share |
|---|---|---|---|---|
| requests | 118,855 | 128,252 | 124,926 | |
| req/sec | 7904.6 | 8531.3 | 8310.2 | |
| wall time | 126.5 µs | 117.2 µs | 120.3 µs | 100% |
| everything else | 52.50 | 49.73 | 50.33 | 41.5–42.4% |
| socket write, 1.000 per request | 38.61 | 35.47 | 36.46 | 30.3–30.5% |
| allocation, 516 calls | 16.76 | 14.28 | 15.31 | 12.2–13.2% |
| collector | 11.93 | 11.60 | 11.98 | 9.4–10.0% |
| socket read returning `Ok`, 1.000 per request | 3.97 | 3.63 | 3.72 | 3.1% |
| socket read finding nothing, 0.983–0.985 per request | 1.70 | 1.53 | 1.57 | 1.3% |
| readiness wait, inside the load window | 1.03 | 0.98 | 0.94 | 0.8% |

- **An `Ok` read carries 41.0 bytes on average**, the whole request head.
- **No read failed.** The counters do not separate end of file from data.
  Between the first and last collection, the bytes read are exact
  multiples of the 41-byte head: 118,819, 128,228 and 124,886 heads over
  118,821, 128,230 and 124,887 `Ok` reads. That leaves 2, 2 and 1 reads
  that carried no head, consistent with end of file or with a head split
  across two reads.
- **Every write completed in one call.** No write was partial or failed.
- **The readiness wait is rare and wide.** It ran 0.0049 times per request
  inside the window. Each wait was given about 200–201 sockets and returned
  about 199–200 ready.
- **`try_read`'s work beyond the OS call** is 1.65–1.78 µs per request.
  - Of that, 0.67–0.71 is allocating and zeroing its 4096-byte buffer.
  - The other 0.98–1.07 is not broken down. It includes copying the data
    into a GC byte buffer and making its `NovaStr` node, two allocations
    also counted under allocation. It also includes freeing the buffer,
    the handle-table lookup and the timers. An allocation in it can also
    run a collection, which is then counted under the collector as well.

  It overlaps "everything else" and allocation rather than sitting beside
  them.
- **`try_write`'s work beyond the OS call** is 0.66–0.76 µs per request.

**The profiling overhead is not resolved.** The plain build (700,928 bytes)
and the profiling build were alternated, three readings each, at the
standard `--warmup 5 --duration 15` with `NOVA_GC_DEBUG` unset. Which
binary each reading ran is not recorded beside the readings: `run_pk.sh`
takes it from `$BIN` and does not log it.

| plain | profiling |
|---|---|
| 7769.8, 8895.7, 8771.6 | 8118.9, 8087.2, 7912.0 |

The ranges overlap. The per-pair ratios are 1.04, 0.91 and 0.90, and the
profiling mean is 5.2% lower. That cannot rule out an overhead of around
10%. **This check ran with `NOVA_GC_DEBUG` unset, while the profile ran
with it set** and printed two lines per collection, so the printing's
cost is not in this comparison.

### What the figures bound

- **The write alone costs more than the gate's remaining gap.** The plain
  build here runs at 112.4–128.7 µs per request, 12.4–28.7 over the 100 µs
  budget, and the write is 35.5–38.6. Taking the whole write away would
  leave 81.7–87.9 µs of the profiling build's requests.
- **One write costs about ten times one data read:** 35.5–38.6 µs against
  3.6–4.0. By the code, the write carries 676 bytes, the 72-byte response
  head plus the 604-byte body, against the read's 41, about 16 times as
  many. How a write's cost depends on its size is not measured.
- **Everything else, 49.7–52.5 µs, is now the largest category.** It is
  still not broken down.

### What this does not settle

- **Why a write costs 35.5–38.6 µs.** The load generator's 200 threads share
  this host, and on loopback a send may do part of the receiver's work, or
  wait while the OS runs another thread. Nothing here separates those from
  the server's own cost. How the cost depends on the payload size is not
  measured either.
- **What "everything else" contains.** Beyond compiled code, runtime
  helpers and the scheduler, it holds the profiling's own cost:
  - the `nova-gc-pg:` print on every collection, outside the collection
    timer;
  - the counter work on every allocation;
  - the clock reads around the sampled allocations, about 2.4 µs per
    request by "(gc-page-heap)"'s estimate;
  - the outer timers around `try_read`, its buffer and `try_write`.

  It also holds any OS time spent on other threads outside the timed
  regions.
- **The profiling's own cost**, as above.
- **One host, Windows, three readings.**

## AMENDMENT 2026-10-02 (sampled-profile): where the server thread's time goes, by function

"(remainder-split)" left 49.7–52.5 µs of a ten-user request as "everything
else", derived by subtraction. This samples the server thread directly, so
every sample lands in a named function. **`users_json` is 38.9–39.4% of the
server thread's time. Within it, `std/json`'s `quote` alone is 25.0–28.2%:
it rebuilds each string one character at a time, through `String.chars`,
`Vec.push` and `vec_chars_to_string`. The socket send path is 36.7–37.7%.
The system heap, which the runtime's string helpers use, is 10.0–11.0%.
No earlier counter isolated it: the string helpers' share sat inside the
subtracted "everything else". Request-head parsing is 1.8–2.0%.**

### How it was measured

A scratch sampler in the runtime, never committed (`NOVA_PROF_SAMPLE=<path>`):
- **Sampling.** A sampler thread asks to sleep 1 ms between samples. In
  practice samples came 1.57 ms apart on the median, about 650 a second.
  Each time, it suspends the thread that called `nova_rt_task_block_on`.
  It reads that thread's registers, walks its stack, resumes it, and only
  then writes the sample. It allocates nothing while the thread is
  suspended.
- **Stack walking.** Frames with Windows unwind info go through
  `RtlVirtualUnwind`.
  - **Nova's generated functions have no Windows unwind info.** In a smoke
    build made before the fallback below, all 3,500 walks stopped early,
    3,493 of them at an address inside Nova's code, and none passed a Nova
    frame. Those addresses resolve with the final build's map: 3,474 of
    them are exact addresses that also appear in the final build's stacks.
  - Whether Cranelift or Nova's object emission is responsible is not
    settled. `task.rs` already notes that generated frames have no unwind
    table.
  - So inside the image the sampler follows the frame-pointer chain,
    bounds-checked against the thread's stack.
  - 98.9–99.2% of walks then ended cleanly. The rest stopped early.
- **Naming.** A scratch `/MAP` flag on the link step maps addresses inside
  the executable to Nova functions, runtime functions and Rust std.
  Addresses in system DLLs are named from those DLLs' own export tables.
- **Each run counts only its load window**: the 14 s starting half a second
  after the load generator connected.

A sample is wall-clock time on the server thread, including time the OS
spent running other threads while the server thread was descheduled.
"Self" counts the function a sample was in. "Inclusive" counts every
function on its stack, once per sample.

Ten users, a 604-byte body, 200 connections, `--warmup 0 --duration 15`,
three fresh processes, every reading `errors=0`. The sampled server is
737,280 bytes, built from `main` at `5ef7b68` with the scratch sampler and
link map applied. The patch file was saved after the build. That the tree
was not edited in between is from this session, not from a file. It ran
at 9382.6–9684.2 req/sec and took 9,071–9,153 samples per window.

**The sampler's own cost is not resolved.** These builds were alternated,
three 10 s readings each:

| plain (700,928 B) | sampler build, sampling off | sampler build, sampling on |
|---|---|---|
| 7717.8, 8974.4, 8937.6 | 8972.4, 9109.0, 8927.6 | 8572.8, 8535.0, 8933.9 |

All three ranges overlap.

**One earlier observation was a mistake, not a finding.** Two smoke runs of
the sampler reached 12,766 and 12,905 req/sec, which looked like the
sampler making the server faster. Those runs never seeded the ten users, so
they measured an empty store. `users_json` appears in 6 of one smoke run's
3,442 samples, against about 39% when seeded.

The plain build was then rerun on a seeded store, unsampled. Back-to-back
5 s loads on one process gave 8329.6, 7294.9 and 8065.5 req/sec, and a
15 s load gave 8487.7, none near the empty-store figures. These readings
and the smoke figures come from this session's output and were not saved
to a file. They show no steady decline: the second load is 12.4% below
the first and the third recovers. They cannot show decay within one load.

These predictions were written before the first sampled run:

| prediction | measured | verdict |
|---|---|---|
| OS DLLs 30–40% of self time | 56.0–57.7% with kernel32, 55.3–57.1% for the four predicted | wrong: above; the socket system call alone is 44.2–45.1%, and the system heap adds about 10 points more |
| collector 8–12% | 10.2–10.5% with the named collector functions' own self time, inferred (see below) | within |
| allocation 8–14% | 8.0–8.2% self in `gc::alloc` and `Pages::alloc_slot`; 12.1–12.7% with the allocation closure and slot zeroing | within |
| Nova compiled code 20–35% self | 9.0–9.3% | wrong: below |
| other runtime helpers 10–20% self | 21.2–22.1%; 9.5–10.0% once both thread-local closures (collector and allocation) are set apart | wrong: slightly below in all three runs, once set apart |
| `users_json` 25–40% inclusive | 38.9–39.4% | within |
| `parse_request_head` 5–15% inclusive | 1.8–2.0% | wrong: below |
| throughput within 10% of the plain build | no sampling-on reading is more than 4.9% below any plain reading | within on the slowdown side; the sampler's cost itself is not resolved |
| most walks unwind cleanly through Nova frames | none did on unwind info alone; 98.9–99.2% did with the frame-pointer walk | wrong as stated |

### Inclusive, % of the server thread's samples

| | run 1 | run 2 | run 3 |
|---|---|---|---|
| `handle` (the route) | 40.05 | 39.77 | 40.39 |
| `users_json` | 39.11 | 38.92 | 39.40 |
| — `user_json` | 30.44 | 30.27 | 33.02 |
| — `stringify` | 26.49 | 25.54 | 28.59 |
| — `quote` | 26.14 | 25.05 | 28.20 |
| socket send path (`ws2_32!send`) | 37.67 | 36.69 | 37.72 |
| `gc::alloc`, collection inside it included | 23.70 | 24.28 | 23.21 |
| `read_request` | 11.58 | 11.67 | 12.57 |
| — socket receive path (`ws2_32!recv`) | 7.39 | 7.53 | 7.55 |
| — `parse_request_head` | 1.96 | 1.78 | 1.83 |
| system heap (`RtlAllocateHeap`, `RtlFreeHeap`, `RtlReAllocateHeap`) | 10.99 | 9.99 | 10.72 |
| `Vec.push` | 10.61 | 10.74 | 9.57 |
| `vec_chars_to_string` | 7.99 | 7.11 | 11.07 |
| `nova_rt_str_chars` | 7.35 | 7.20 | 7.93 |
| `nova_rt_str_concat` | 7.81 | 7.60 | 5.66 |
| `Response.to_bytes` | 4.12 | 5.10 | 5.58 |
| `nova_rt_check_bounds` | 2.85 | 2.80 | 2.79 |
| `memset` | 2.51 | 2.70 | 2.43 |

These rows nest and overlap, so they do not sum to 100%.

### Self, % of the server thread's samples

| | run 1 | run 2 | run 3 |
|---|---|---|---|
| `ntdll!ZwDeviceIoControlFile`, the socket system call | 45.03 | 44.17 | 45.13 |
| system heap, self | 10.62 | 9.46 | 10.21 |
| one `LocalKey::with` instance entered from `gc::alloc` (inferred collector) | 10.01 | 10.30 | 10.01 |
| Nova compiled code | 9.05 | 9.26 | 9.18 |
| `gc::alloc` and `Pages::alloc_slot` | 8.11 | 8.19 | 8.01 |
| `vcruntime140` (`memset` and friends) | 3.67 | 4.13 | 3.64 |

**The collector is identified by inference, and the evidence is strong.**
`collect_with_roots` runs its mark and sweep inside a `HEAP.with` closure,
reached from `gc::alloc` through the inlined `maybe_collect` and `collect`.
- **The collector instance.** One `LocalKey::with` instance,
  `…ce5e268d0d359845`, holds 10.0–10.3% of self time. Every one of its
  2,761 self samples across the three runs is entered from `gc::alloc`. Its
  callees are only collector-path code, including a frame named
  `collect_with_roots::{{closure}}::{{closure}}`. With those callees it is 10.25–10.61%.
- **The allocation instance.** Allocation runs in a different instance,
  `…d03a43526a1fdc50`. That one is the parent of every `Pages::alloc_slot`
  sample and every `memset` sample from slot zeroing.
- **Against the timers.** The collector's 9.4–10.0% in the timer-based
  "(remainder-split)" run is consistent with this, on a different build and
  load.

**What drives the system heap.** These are the samples inside a heap call,
charged to the runtime function above it:
- `nova_rt_str_chars`: 4.3–4.6%;
- `nova_rt_str_from_chars`: 2.3–2.7%;
- `nova_rt_str_concat`: 1.4–1.7%;
- `nova_rt_int_to_str`: 0.5–0.6%;
- `try_read`'s 4096-byte buffer: 0.6%;
- `nova_rt_str_concat_n`: 0.4–0.6%.

### What the figures bound

- **`quote` is 25.0–28.2% inclusive.** It is the deepest Nova function
  holding that much time: `user_json` and `stringify` contain it, and
  `write_stream.240$poll` (37.0–38.0%) is the socket path.
  - Collections that happened to trigger inside `quote`'s allocations
    account for 3.4%, 3.5% and 6.5% of all samples. Without them `quote` is
    21.5–22.8%, and run 3's higher figure comes entirely from them.
  - What `quote` does: it turns every string into a `Vec<Char>`, pushes
    each character into a second vector that grows through `nova_rt_alloc`,
    and rebuilds a string from that. For a string with nothing to escape,
    all of that produces the string wrapped in quotation marks.
  - How much a faster `quote` would save is not measured.
- **The socket send path is 36.7–37.7% here**, against the timed write's
  30.3–30.5% in "(remainder-split)". Both cover the same layers: here,
  std's `TcpStream::write` inclusive equals `ws2_32!send` inclusive to
  within one sample. The gap is between methods, builds and loads, and is
  not explained: 9382.6–9684.2 req/sec here, 7904.6–8531.3 there.
- **Request-head parsing is small:** `parse_request_head` is 1.8–2.0%
  inclusive. `read_request`'s other parsing (`parse_offsets`,
  `content_length_of`) adds about 0.3–0.4%. Any collections its
  allocations trigger are charged wherever the threshold is crossed.

### What this does not settle

- **Samples that stopped early.** 0.8–1.1% of walks did not end cleanly,
  most of them at `kernel32!GetProcessHeap` on the allocation path. So the
  heap and string-helper inclusive rows are slightly low.
- **Frames a frameless Nova leaf would hide.** The frame-pointer walk
  assumes every Nova function keeps a frame. A Nova leaf function without
  one would charge its caller's samples to the caller's caller, which
  slightly shifts the inclusive figures for Nova functions.
- **The sampler's own cost**, as above.
- **What a Nova function without unwind info means elsewhere.** Anything
  that unwinds the stack on Windows, a debugger or a crash handler, cannot
  pass through Nova frames on unwind info alone. That was not investigated
  here.
- **One host, Windows, three readings.**

## AMENDMENT 2026-10-02 (quote-fast-path): a string with nothing to escape is not rebuilt

"(sampled-profile)" found `std/json`'s `quote` at 25.0–28.2% of the server
thread, rebuilding every string one character at a time. `quote` now scans
first for `"`, `\` or a control character below `0x20`. A string with none
is returned in quotation marks by one interpolation; any other string takes
the old loop unchanged.

**Results:**
- **Per call, on strings with nothing to escape,** `stringify` went from
  747–824 to 402–459 ns for a 13-character name, and from 986–1024 to
  408–431 ns for a 17-character email.
- **The escaped control string** went from 1029–1080 to 1066–1140 ns. Each
  pair was slower, by +3.6%, +10.1% and +3.3%, but the ranges overlap and
  the cause was not isolated.
- **Ten-user throughput** rose from 8603.0–9340.5 to 9954.7–10806.2
  req/sec. The ranges are disjoint, a gain of 1.07x to 1.26x.
- **Two of the three after readings exceed 10,000 req/sec**, under this
  file's 15 s method. Neither of the gate's criteria was measured here:
  not the absolute 10k of `nova-spec/00-MASTER-SPEC.md` §3 under the 30 s
  after a 5 s warmup that §5's 2026-09-11 amendment used, and not
  `nova-spec/60-EXAMPLES.md` §5's ratio against Bun.

### How it was measured

- **Per call:** a scratch Nova harness, never committed, times 500,000
  calls of `stringify(String(s))` per string with `std/time`'s `Instant`.
  It used three strings: `User Number 7`, `user7@example.com`, and
  `say "hi" back\slash`, which needs escaping and serves as the control.
  - `before` is 517,120 bytes, built from `main` at `942506c`'s `std/json`.
  - `after` is 517,632 bytes, built from `ccd8cea`.
  - The two were alternated, three runs each, one fresh process per run.
- **Server:** ten users, a 604-byte body, 200 connections,
  `--warmup 5 --duration 15`, three fresh processes each, alternated. Every
  reading was `errors=0`.
  - **Both server binaries are 700,928 bytes.** The linker pads each PE
    section to the 512-byte FileAlignment, and the server's `.text` grew
    384 bytes inside its existing padding. So each reading logged its
    binary's SHA-256 as well: `1222411ee0f0296d…` before,
    `b6fd6c650a1a2ed6…` after.
  - **All four binaries were built by the release `nova`**, so they link the
    release runtime. 700,928 bytes is also the plain release build's size
    recorded in "(gc-page-heap)".
  - **The response body was byte-identical in all six readings**, SHA-256
    `3ff5004bf26139cc…`.

These predictions were written before the fast path existed. Their per-call
"before" figures were copied from a smoke run of the `before` harness. That
run's output is from this session and was not saved to a file. So only the
"after" figures, the control's and the server's are predictions:

| prediction | measured | verdict |
|---|---|---|
| name: before about 760 ns, after 250–450 | before 747–824, after 402–459 | one after reading above |
| email: before about 990 ns, after 300–550 | before 986–1024, after 408–431 | within |
| escaped control: within +0% to +15% of before | +3.6%, +10.1% and +3.3% per pair | within |
| server: before 7700–9100, after 9500–11500, disjoint | before 8603.0–9340.5, after 9954.7–10806.2, disjoint | one before reading above; after within |

### Per call, ns, in run order

| run | name before | name after | email before | email after | escaped before | escaped after |
|---|---|---|---|---|---|---|
| 1 | 747 | 402 | 986 | 408 | 1029 | 1066 |
| 2 | 824 | 459 | 993 | 417 | 1035 | 1140 |
| 3 | 790 | 411 | 1024 | 431 | 1080 | 1116 |

### Server, in run order

| reading | build | req/sec |
|---|---|---|
| 1 | before | 8603.0 |
| 2 | after | 10676.8 |
| 3 | before | 9061.9 |
| 4 | after | 10806.2 |
| 5 | before | 9340.5 |
| 6 | after | 9954.7 |

### Correctness

- **The escapes fixture gained boundary cases:** `0x1f`, the highest
  character below `0x20`, still escapes; space and `~` pass through; and an escapable
  character first, or last behind a clean run, still sends the whole string
  through the loop.
- **Six mutants were run against every `json_` fixture:**
  - dropping the `"` check: fails the two string fixtures;
  - dropping the `\` check: fails them too;
  - moving the control boundary down to `< 31`: fails them too;
  - stopping the scan one character early: fails them too;
  - returning the string without its quotation marks: fails seven tests;
  - moving the boundary up to `< 33`: survives, as expected. The only
    strings it additionally sends through the slow loop are clean strings
    containing a space, and the loop's output for them is the same, so no
    output test can see it.
- **The gates pass:** the whole suite at 1170 passed, 0 failed, 8 ignored,
  including all 12 `*_under_gc_stress` tests. Clippy with `-D warnings` and
  rustfmt are clean on the code commit.

### What this does not settle

- **The gate.** Its absolute criterion was not measured at the 30 s after a
  5 s warmup that §5's 2026-09-11 amendment used, and the after range here
  straddles 10,000. Its ratio against Bun was not measured either.
- **What `quote` still costs.** It still materialises `s.chars()` to scan.
  The sampled profile was not rerun on this build.
- **One host, Windows, three readings per arm.**

## AMENDMENT 2026-10-02 (gate-remeasure): the Phase 2 gate on `main` at `5efcc2e`

This reruns both of the gate's criteria with the method the 2026-09-11 Bun
ratio section uses: ten users, 200 connections, 30 s after a 5 s warmup,
four cells alternated, two replicates each.

**The absolute criterion is not cleanly met.**
- **Pooled.** This file's earlier status table pooled Nova's pinned and
  unpinned readings. Pooled the same way, the four Nova readings span
  8424.0–10382.4 req/sec and straddle `nova-spec/00-MASTER-SPEC.md` §3's
  10,000.
- **Unpinned.** Nova clears 10,000 in both readings, at 10250.0–10382.4,
  2.5–3.8% above it.
- **Pinned to one core.** Nova misses, at 8424.0–8520.1.

§3 names no pinning condition.

**The ratio against Bun is not met.** Pinned Nova over pinned Bun is
0.70–0.79, and unpinned over unpinned is 0.79–0.81, against the 1.0
`nova-spec/60-EXAMPLES.md` §5 asks for: short by 1.24x to 1.43x.

**The gate is specified twice, and the two statements now disagree.** That
is the case §3's 2026-09-03 amendment anticipated: "10k could be reached
while the ratio fails". Under any reading that counts §5, the gate is not
met. The unpinned absolute pass is also thin. It rests on two readings, and
this same code at 15 s ran from 9954.7 to 10806.2 req/sec in
"(quote-fast-path)". Another session could land below 10,000.

### The cells, in run order

| reading | cell | side | pinned | mask read back | req/sec |
|---|---|---|---|---|---|
| 1 | A | Nova | core 0 | 1 | 8520.1 |
| 2 | C | Bun | core 0 | 1 | 10774.3 |
| 3 | B | Nova | no | 4095 | 10250.0 |
| 4 | D | Bun | no | 4095 | 12956.6 |
| 5 | A | Nova | core 0 | 1 | 8424.0 |
| 6 | C | Bun | core 0 | 1 | 12068.8 |
| 7 | B | Nova | no | 4095 | 10382.4 |
| 8 | D | Bun | no | 4095 | 12846.9 |

- **Every reading was `errors=0`.**
- **Each server was a fresh process.** After it printed its listening line
  and before seeding, it was pinned or left alone, and its affinity mask
  was read back and logged.
- **One reading is not in the table.** A smoke reading of cell C, run after
  the predictions and before the matrix, gave 11741.5 req/sec with mask 1.
  It falls inside C's range.

| cell | range | ratio |
|---|---|---|
| A, Nova pinned | 8424.0–8520.1 | |
| B, Nova unpinned | 10250.0–10382.4 | |
| C, Bun pinned | 10774.3–12068.8 | A/C 0.698–0.791 |
| D, Bun unpinned | 12846.9–12956.6 | B/D 0.791–0.808 |

**The load generator was not the limit.** Its self-test ceiling, taken
after the matrix in the same session, is 93680.1 req/sec. Bun's fastest
reading is under 14% of that.
- It was invoked as `nova-bench-http --self-test --connections 200
  --duration 30 --warmup 5`. That command is from this session; the RESULT
  line itself records only the connection count.
- One connection was starved (`conn_min=672`, `conn_max=50255`), which this
  file elsewhere treats as weaker evidence for a ceiling. At 13.8% of it,
  the conclusion does not depend on that. The generator is
`target/release/nova-bench-http.exe`: 256,512 bytes, SHA-256
`d17062335e988c18…`, built 2026-09-29.

**These predictions were written before any throughput reading.** The same
command then ran the equivalence check. That ordering is from this
session, and no file records it. The files' mtimes show only that the
predictions precede the check's end.

| prediction | measured | verdict |
|---|---|---|
| equivalence: all 9 exchanges match | all 9 match | within |
| A: 9000–11000 | 8424.0–8520.1 | wrong: below |
| B: 9000–11000 | 10250.0–10382.4 | within |
| C: 11000–19000 | 10774.3–12068.8 | one reading below |
| D: 10000–13000 | 12846.9–12956.6 | within |
| A/C: 0.5–0.9 | 0.698–0.791 | within |
| B/D: 0.7–1.05 | 0.791–0.808 | within |
| Nova's ranges straddle 10,000 | A entirely below, B entirely above | partly: the cells split rather than straddle, though pooled they straddle |

### Identity of each side, and the payload

- **Nova:** `examples/05-json-api` built by the release `nova` from
  `5efcc2e`. `json-api.exe` is 700,928 bytes, SHA-256
  `99f6e5f6b4b45206…`, and the same binary ran in every Nova reading.
- **Bun:** `docs/benchmarks/bun-server.js`, run by bun 1.3.0. The version
  is from `bun --version` in this session and was not logged.
  - The script's code is unchanged since 2026-09-11; only its comments
    changed.
  - It is 4,297 bytes in git, against 4,179 then, and 4,415 bytes in this
    checkout because of Windows line endings.
- **The payload.**
  - Ten users were seeded by the same `curl` POSTs on each side.
  - Before each load the body was fetched. Its size and hash were compared
    afterwards, in analysis.
  - Every reading on both sides served the same 604-byte body, SHA-256
    `3ff5004bf26139cc…`.
- **Equivalence was run first**, by hand, against the `json-api.exe` built
  for this run. The log does not record the binary's path. Its result:
  `EQUIVALENCE OK: all 9 exchanges match on status and body bytes`.

**The wire framing still differs, and still in Nova's favour.** Measured
again on this build for the `/users` response, Nova's head is 72 bytes and
Bun's is 109. Bun adds a `Date` header, 37 bytes, the same difference
recorded on 2026-09-11, so Bun sends 713 bytes per response against Nova's
676.

### Against the last matrix, 2026-09-12

That matrix is the one behind the ratio row of this file's status table, on
a 534-byte body. This one uses a 604-byte body.

| cell | 2026-09-12 | now |
|---|---|---|
| A, Nova pinned | 2868.2, 3392.4 | 8424.0, 8520.1 |
| B, Nova unpinned | 3024.9, 3056.5 | 10250.0, 10382.4 |
| C, Bun pinned | 12470.8, 12480.6 | 10774.3, 12068.8 |
| D, Bun unpinned | 12647.3, 12934.0 | 12846.9, 12956.6 |

- **The pinned ratio** rose from 0.230–0.272 to 0.70–0.79.
- **Part of that rise is Bun.** Bun pinned is lower now, at 0.863–0.968 of
  its 2026-09-12 readings, ranges disjoint, on a 13% heavier body.
- **Bun unpinned overlaps** its 2026-09-12 readings.

### What this does not settle

- **Why the pinned cells are slower than the unpinned ones.** This session
  that holds on both sides, disjoint:
  - Nova: 8424.0–8520.1 against 10250.0–10382.4;
  - Bun: 10774.3–12068.8 against 12846.9–12956.6.

  On 2026-09-11 and 2026-09-12, Nova's two cells overlapped. Bun's were
  already narrowly disjoint on 2026-09-12. One candidate is that core 0 is
  shared with the load generator's threads. It was not measured.
- **Whether the unpinned absolute pass holds across sessions.** It rests on
  two readings 2.5–3.8% above the line.
- **Which statement of the gate governs.** No tracked file settles it.
- **Any host but this one.** Every figure here is from this development
  host, Windows, with the load generator on the same machine. The spec does
  not define §3's "benchmark hardware".

## AMENDMENT 2026-10-02 (reprofile): the server thread after `quote`'s fast path

This reruns "(sampled-profile)"'s scratch sampler, unchanged, on `main` at
`82c9c3a`, so the next lever is chosen from current data.

**The socket send path is now the largest item: 38.4–39.3% of the server
thread.** `users_json` fell from 38.9–39.4% to 33.0–33.4%, and `quote`
within it from 25.0–28.2% to 15.5–16.4%. The per-character rebuild is gone:
`vec_chars_to_string`, `Vec.push` and `nova_rt_str_from_chars` no longer
appear. **Half of what `quote` still costs is `s.chars()`**, 7.7–8.2%,
which builds a character array just to scan it. The scan itself,
`needs_escape`, is 1.1–1.6%. `gc::alloc`, collection included, is
unchanged at 23.0–23.8%, and the system heap is 11.4–12.2%.

### How it was measured

- **Method.** It is "(sampled-profile)"'s method and scratch patch; that
  the patch was unchanged is from this session, not from a file.
  - `analyze.py`, `heap_callers.py` and `lk.py` are unchanged.
  - `keys.py` is renumbered for this build's Nova symbols and adds
    `needs_escape`.
  - A new script, `idx_callers.py`, charges a function's samples to the
    Nova frames above it. The caller attributions below come from it.
  - Another new script, `heap_split.py`, makes the system-heap split by
    caller.
- **The sampled server** is 737,280 bytes, SHA-256 `81833226e6961d71…`.
  - It was built by the release `nova` from `82c9c3a` with the patch
    applied, and the patch was reverted afterwards. Both facts are from
    this session, not from a file.
  - The map does show a build after the fast path, since it carries
    `needs_escape`.
- **Runs.** Ten seeded users, a 604-byte body, 200 connections,
  `--warmup 0 --duration 15`, three fresh processes, every reading
  `errors=0`. Each run counts only the 14 s starting half a second after
  load began: 9,089–9,181 samples. 98.7–99.1% of walks ended cleanly.

**Throughput varied 1.41x across the three runs** of this one binary, at
9275.4, 13078.5 and 10831.8 req/sec, while every share in the inclusive
table below stayed within 1.5 points.
- **Not one part of the thread.** A cause confined to one part would have
  moved that part's share far more, so the variation was not in any one
  part.
- **Possibly less CPU.** Samples are wall-clock time and include time the
  thread was descheduled, with the load generator on the same host. So it
  is as consistent with the thread getting less CPU as with all of it
  running slower.
- **The cause is not known.**

These predictions were written before any sampled run:

| prediction | measured | verdict |
|---|---|---|
| send path 40–46% | 38.4–39.3% | wrong: below |
| `users_json` 22–30% | 33.0–33.4% | wrong: above |
| `quote` 8–14%; within it, `nova_rt_str_chars` the largest part | 15.5–16.4%; `String.chars` is 7.7–8.2% of it, its largest callee | wrong: above; the part named is right |
| `gc::alloc` 18–24% | 23.0–23.8% | within |
| system heap 7–10% | 11.4–12.2% | wrong: above |
| `read_request` 12–15%, `parse_request_head` 2–3% | 14.5–15.2%; 2.1–2.8% | two `read_request` readings just above |
| 9500–11500 req/sec with the sampler | 9275.4, 13078.5, 10831.8 | one below, one above |

### Inclusive, % of the server thread's samples

| | run 1 | run 2 | run 3 | "(sampled-profile)" |
|---|---|---|---|---|
| socket send path (`ws2_32!send`) | 38.81 | 39.34 | 38.41 | 36.69–37.72 |
| `handle` (the route) | 35.22 | 34.48 | 35.16 | 39.77–40.39 |
| `users_json` | 33.35 | 33.00 | 33.38 | 38.92–39.40 |
| — `user_json` | 23.09 | 22.25 | 23.73 | 30.27–33.02 |
| — `quote` | 15.79 | 15.50 | 16.36 | 25.05–28.20 |
| — — `String.chars` | 8.20 | 7.73 | 7.95 | 5.15–5.41 |
| — — `needs_escape` | 1.07 | 1.55 | 1.52 | (new) |
| `gc::alloc`, collection included | 23.80 | 23.04 | 22.98 | 23.21–24.28 |
| `read_request` | 15.04 | 15.22 | 14.46 | 11.58–12.57 |
| — socket receive path (`ws2_32!recv`) | 7.90 | 8.99 | 7.87 | 7.39–7.55 |
| — `parse_request_head` | 2.81 | 2.11 | 2.47 | 1.78–1.96 |
| system heap | 11.43 | 12.10 | 12.15 | 9.99–10.99 |
| `nova_rt_str_chars` | 11.20 | 10.60 | 11.13 | 7.20–7.93 |
| `nova_rt_str_concat` | 9.03 | 9.53 | 8.46 | 5.66–7.81 |
| `Response.to_bytes` | 7.17 | 7.42 | 7.71 | 4.12–5.58 |
| `nova_rt_str_concat_n` | 6.59 | 5.90 | 6.75 | 1.79–2.32 |

The rows nest and overlap, so they do not sum to 100%. A share can rise
while its absolute cost stays flat, because others shrank around it.

### Where the remaining Nova-side work is

- **`quote`'s `s.chars()`: 7.7–8.2%.** All but 2–4 `String.chars` samples
  per run came from `quote`, and the exceptions lack only `quote`'s frame.
  - `nova_rt_str_chars` builds a Rust `Vec<char>` on the system heap, then
    copies it into a GC array, only for `needs_escape` to read it once.
  - It is the system heap's largest caller, at 6.1–6.8% of all samples:
    4.5–4.8% through `quote`, and the rest, 1.6–2.0%, through the header
    check below.
- **`is_crlf_free` in `Response.to_bytes`: 5.1–5.4% inclusive.** It is the
  header check, run on every header name and value, once for `"\r"` and
  once for `"\n"`.
  - Each check goes through `String.contains` to `String.index_of`, which
    builds two fresh character arrays first: one for the string and one
    for the needle. That is `nova_rt_str_chars`, at 2.9–3.2% of the thread.
  - The character-by-character walk itself, `chars_match_at`, is only
    0.7–0.8%.
  - Every `String.index_of` sample came from this check: 4.7–5.0% of the
    thread.
- **`nova_rt_str_concat`: 8.5–9.5%.** About 98% of it, 8.3–9.3 points of
  the thread, is `users_json`, which grows its output string two pieces at
  a time, so each step copies everything built so far.
- **The collector:** 8.1–9.2%, the same inferred `LocalKey::with` instance
  as before.

### What this does not settle

- **Why throughput varied 1.41x** across three runs of one binary.
- **What a cheaper `s.chars()`, header check or concatenation would
  save.** The shares above bound each one; none of them was measured.
- **The send path.** At 38.4–39.3% it is the largest single item, and why
  a write costs what it does is still unmeasured.
- **One host, Windows, three readings.**

## AMENDMENT 2026-10-02 (str-chars-direct): a string's characters go straight into GC memory

"(reprofile)" found `nova_rt_str_chars` charging 6.1–6.8% of the server
thread to the system heap. It collected a string's characters into a Rust
`Vec<char>`, then copied that into the GC array. It now counts the
characters, allocates the GC array once and fills it directly. Its output,
size and scan flag are unchanged.

**Results:**
- **Per call,** `chars()` went from 160–201 to 33–38 ns on a 13-character
  name, and from 164–226 to 35–43 ns on a 17-character email. `stringify` on
  the clean name went from 348–368 to 215–258 ns. All three are disjoint.
- **The ten-user server's ranges overlap**, so no gain is claimed for it.
  Over six alternated pairs, before ran at 9597.7–11933.1 req/sec and after
  at 10308.3–11824.1. After was faster in 5 of the 6 pairs, and its mean was
  6.0% higher. The before readings alone ranged from 9597.7 to 11933.1
  (1.24x) across the session, which is more than that difference.

### How it was measured

- **Per call:** a scratch Nova harness, never committed, times 500,000
  calls each of `"User Number 7".chars()`, `"user7@example.com".chars()` and
  `stringify(String("User Number 7"))`.
  - `before` is 518,144 bytes, SHA-256 `85529fb73fa7f6d4…`.
  - `after` is 517,120 bytes, SHA-256 `4a36eb0e6807f107…`.
  - Both were built by the release `nova`, `before` from `00915c3` and
    `after` from `7d04191`. That is from this session, not from a file.
  - The two were alternated, three runs each, one fresh process per run.
- **Server:**
  - `before` is 700,928 bytes, SHA-256 `4c566ecc09053175…`; `after` is
    700,416 bytes, SHA-256 `47058d5f49b2a10b…`. They were built the same
    way, from the same two commits; that too is from this session.
  - Ten users, a 604-byte body, 200 connections, `--warmup 5 --duration 15`.
  - Alternated, one fresh process per reading, every reading `errors=0`.
  - The response body was byte-identical in all twelve readings, SHA-256
    `3ff5004bf26139cc…`.
  - **Six pairs, not three.** The first three overlapped, so three more
    pairs were taken. That follows the rule written for the page heap in
    `docs/superpowers/plans/2026-10-01-gc-page-heap.md`: "If the ranges
    overlap, take three more of each and report all readings." They
    overlapped too.

These predictions were written before the change existed. Their per-call
"before" figures were copied from a smoke run of the `before` harness, so
only the "after" figures and the server's are predictions:

| prediction | measured | verdict |
|---|---|---|
| `chars()` name: after 100–170 ns | 33–38 | wrong: far below |
| `chars()` email: after 100–170 ns | 35–43 | wrong: far below |
| `stringify` name: after 300–360 ns | 215–258 | wrong: below |
| server: after within 0–8% of before, ranges likely overlap | ranges overlap; after's mean 6.0% higher | ranges overlap, as predicted; the 0–8% part is not settled |

The per-call savings ran past the predictions. A 13-character `chars()`
went from 160–201 ns to 33–38, a cut of 76–83% per pair. So the work
removed was most of each call's cost: the `Vec`'s allocation, its two
regrowths and its free, and the copy. The two regrowths are read from the
source of std's `Vec` collect in Rust 1.95.0, not measured: 4 to 8 to 16
slots for the name, 5 to 10 to 20 for the email. The net cut is a lower
bound on the work removed, since the change also added a counting pass.

### Per call, ns, in run order

| run | `chars()` name before | after | `chars()` email before | after | `stringify` name before | after |
|---|---|---|---|---|---|---|
| 1 | 160 | 33 | 172 | 35 | 368 | 258 |
| 2 | 161 | 38 | 164 | 43 | 348 | 215 |
| 3 | 201 | 34 | 226 | 37 | 365 | 216 |

### Server, in run order

| pair | before | after | after / before |
|---|---|---|---|
| 1 | 9597.7 | 10308.3 | 1.074 |
| 2 | 10718.0 | 11360.5 | 1.060 |
| 3 | 9795.0 | 11529.9 | 1.177 |
| 4 | 10592.7 | 11016.9 | 1.040 |
| 5 | 11933.1 | 11824.1 | 0.991 |
| 6 | 10845.6 | 11271.1 | 1.039 |

### Correctness

- **A new unit test,
  `str_chars_sizes_and_fills_by_character_on_a_long_mixed_string`,** runs
  on 100 repetitions of `aé🦀`: 300 characters in 700 bytes. It checks the
  length word, every element, and the size and scan flag `gc::alloc`
  recorded, `(2408, true)`. The existing layout test still covers a mixed
  ASCII and multi-byte string, `a→🦀`, and the empty string.
- **Four mutants each fail named tests:**
  - counting bytes instead of characters: 3 tests;
  - shifting the fill onto the length word: 3;
  - allocating the array as a leaf: 2;
  - stopping the fill one character short: 3.
- **The source is read after the allocation.** The old code copied it out
  first. Its bytes stay alive because this function's pointer to them is a
  root under the collector's stack scan, as `gc.rs`'s module doc states. A
  comment at the call says so.
  - All 12 `*_under_gc_stress` tests, which collect on every allocation,
    pass.
  - They are weak evidence for this claim. A literal's bytes are static
    and never freed. A string a caller still holds keeps its bytes alive
    through its header, which `gc_str` allocates as scanned. In both cases
    the tests pass whether or not this frame's pointer is scanned. The
    claim rests on the argument.
- **The gates pass on `b117631`.** The branch's one later code commit
  changes only a comment. A scratch log records that commit's hash and
  each command:
  - `cargo test --locked --workspace`: 1171 passed, 0 failed, 8 ignored;
  - `cargo clippy --locked --workspace --all-targets --all-features -- -D
    warnings`: exit 0;
  - `cargo fmt --all --check`: exit 0.

### What this does not settle

- **The server-level effect.** Its ranges overlap. Separating a shift of
  the size the means suggest from this session's drift needs more pairs or
  a quieter host.
- **What `chars()` still costs inside `quote` and the header check** on this
  build. The sampled profile was not rerun.
- **One host, Windows.**

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

**[2026-09-30: "AMENDMENT 2026-09-30" above records the collector's
per-cycle time growing with the live set over a run. That bears on this
mechanism and does not establish it.]**

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

**Superseded by AMENDMENT 2026-09-12, above.** Every `users_json` figure in
this section -- the decomposition, the isolation table, the 32%-52%
framing and the 2.0x-3.2x amplification -- was measured through the
`stringify` this branch replaced, for the same reason and to the same
degree that amendment states, and is not restated as a new figure here.
**This decomposition is re-run in the "FURTHER AMENDMENT 2026-09-12"
section near the top of this file**, against a binary built from the
current `stringify`.

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
separately. **[2026-09-30: the collector now is — see "AMENDMENT
2026-09-30" above.]**

## Measured 2026-09-12: `stringify`'s scalar fast path

`std/json`'s `stringify` gained a fast path that matches a top-level scalar
(`Null`, `Bool`, `Number` or `String`) before the work-list loop above
allocates anything, and returns finished text directly for each of those
four cases; `Array`, `Object`, and a scalar reached through either, still
fall through to the unchanged loop above. **Only the `String` arm's saving
was measured below -- `Null`, `Bool` and `Number` got the same match arm
with no separate measurement, and no figure in this section covers them.**

Measured with a dedicated harness (`/tmp/bench_stringify.nova`), separate
from the compiled per-request decomposition above and not merged into it
here: three separate process invocations per cell, each reported as a
range, beside the harness binary's byte size for that side of the
comparison.

| cell | before -- 511488-byte binary | after -- 512000-byte binary |
|---|---|---|
| `stringify` on a `String`, per call | 4330–4491 ns | 2161–2314 ns |
| `users_json`, 5 users, 266 B | 80227–82254 ns | 46627–49438 ns |
| `users_json`, 10 users, 532 B | 163015–165525 ns | 94620–106971 ns |
| `users_json`, 20 users, 1092 B | 342292–346948 ns | 198884–221441 ns |
| `users_json`, 40 users, 2212 B | 644027–656796 ns | 428785–456048 ns |

`stringify` on a `String` is roughly **1.9x–2.1x faster**. `user_json(u)`
builds each user's JSON by hand, interpolating `stringify(String(u.name))`
and `stringify(String(u.email))` directly rather than calling `stringify`
once on an assembled `JsonValue::Object` -- so `users_json` exercises that
same top-level `String` path twice per user, and its speedup, roughly
**1.4x–1.8x** across the four collection sizes above, sits close to
`stringify`'s own rather than diluted by tree-walking work this fast path
does not touch.

This harness's own `users_json` figures are a different measurement from
the compiled per-request decomposition above (`users_json` at 158,399
ns/call, ten users, 2000 iterations) -- a different harness, not folded into
one series with it here. Full figures, raw per-run output and the mutation
that checked the `String` arm are in
`.superpowers/sdd/2026-09-12-stringify-scalar-fast-path/task-1-results.md`
(gitignored, not part of this file).

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
  is a plausible cause and is not established here. **[2026-09-30: still
  not established; see "AMENDMENT 2026-09-30" above for the growing live
  set.]**
- **The two costs already recorded against `std/http`** — eager header
  materialisation, and quadratic body accumulation — are neither confirmed
  nor refuted. The generator sends one header and no body, so this
  measurement does not reach either.
- **The Bun ratio**, §5's own criterion. Unmeasured, and measurable on this
  host.
- **Anything about other hosts.** One host, two replicates per point,
  Windows.

## Measured 2026-09-11: the Bun ratio, §5's own criterion

**Its four cells are superseded by the "FURTHER AMENDMENT 2026-09-12"
section near the top of this file, which re-runs this same matrix against
a binary built from `stringify`'s scalar fast path.**

Four cells, two replicates each, taken alternating sides (A, C, B, D, then
A, C, B, D) so drift over the session could not align with one side of the
ratio. `errors=0` on every run. Generator: `crates/nova-bench-http`, the
same binary driving both sides, `--connections 200 --duration 30 --warmup 5
--path /users`, keep-alive, against a ten-user collection.

| cell | side | pinned | affinity read back | range across 2 replicates |
|---|---|---|---|---|
| A | Nova | core 0 | **1** | 2234.7 – 2500.9 |
| B | Nova | no | **4095** | 2314.3 – 2349.0 |
| C | Bun | core 0 | **1** | 12245.0 – 19220.6 |
| D | Bun | no | **4095** | 10165.1 – 12498.7 |

**Headline: cell A over cell C, pinned Nova over pinned Bun — 0.116 –
0.204.** Cell B over cell D, unpinned Nova over unpinned Bun, is 0.185 –
0.231. Combined, the measured ratio is 0.116 to 0.231. `docs/benchmarks/
README.md` records the pinning mechanism, why pinned is reported as the
headline, and that both arms were measured rather than one chosen — which
is how it is known that the fairness decision does not move the verdict.

`nova-spec/60-EXAMPLES.md` section 5 asks for at least 1.0. **The measured
ratio falls short by roughly 4.3x to 8.6x.**

### Identity of each side, and the seeded payload

| side | artifact | identity |
|---|---|---|
| Nova | `examples/05-json-api` built by the release `nova` | `json-api.exe`, **690,176 bytes** |
| Bun | `docs/benchmarks/bun-server.js` | **4,179 bytes**, run by **bun 1.3.0** |

Ten users seeded by `curl` POSTs into a freshly started server; every cell
above reported `seeded users=10 body_bytes=534` — the same 534-byte body on
both sides, confirmed by `curl /users` before every run.

### Equivalence, which is what the ratio is a ratio *of*

Task 1's check (`docs/benchmarks/bun-equivalence.js`) drove the nine
exchanges `crates/nova-cli/tests/run_tests.rs` already pins, against a
fresh instance of each server, comparing status code and response body
bytes: `EQUIVALENCE OK: all 9 exchanges match on status and body bytes`.
That result is load-bearing rather than vacuously green: mutating the Bun
side's body formatting failed 4 of the 9 exchanges — 2, 3, 8 and 9, the
ones whose response body calls `userJson` — and mutating its routing to
drop the `/users` segment check failed the one
exchange that depends on it (`GET /nope/1`); a revert reconfirmed 9 of 9.
The equivalence check gates the measurement above, not the build — see
`docs/benchmarks/README.md`.

### The wire framing is not identical, and the difference favours Nova

Nova's response head is **70 bytes** for a 2-byte body; Bun's is **107
bytes** for the same body — `Bun.serve` adds one `Date` header the example
does not, measured at **37 bytes** with `curl -s -D - -o /dev/null`
against each server for the same 2-byte body, not assumed. Bun pays for
framing this measurement does not charge Nova for, which biases the
ratio **in Nova's favour**. At
0.116 – 0.231 that bias cannot rescue the result; a ratio landing within a
few percent of 1.0 would need to be read against it rather than reported as
a pass.
