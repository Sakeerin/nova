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
  objects per user or the 17 per header.
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
