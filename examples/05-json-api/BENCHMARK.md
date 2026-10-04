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

See "AMENDMENT 2026-10-02 (gate-remeasure)". A later run on `68d0b94`
repeated only the unpinned cells and found Nova and Bun level, at 0.89–1.13
taken the same way, partly because Bun read lower than it did there. It is
not a gate measurement, and this status stands until the full method is run
again. See
"AMENDMENT 2026-10-02 (fixed-vs-json)".

**Amended 2026-10-02 (gate-remeasure-2):** the full method, with three
replicates per cell, on `main` at `cdaea7e`. Its figures supersede
"(gate-remeasure)"'s as the gate's recorded status:
- **Absolute:** met in all six Nova readings, pinned 10986.5–12403.2 req/sec
  and unpinned 10768.5–12126.4, so pooled 10768.5–12403.2.
- **Ratio:** not met unpinned, at 0.854–0.976 with the ranges disjoint;
  0.715–1.334 pinned, straddling 1.0.

See "AMENDMENT 2026-10-02 (gate-remeasure-2)".

**Amended 2026-10-02 (gate-remeasure-3):** the same method on `main` at
`012ca55`, after "(fast-join)". Its figures supersede
"(gate-remeasure-2)"'s as the gate's recorded status:
- **Absolute:** met in all six Nova readings, pinned 17682.5–18528.1
  req/sec and unpinned 17400.4–20014.4.
- **Ratio:** not met pinned, at 0.886–0.993 with the ranges disjoint;
  0.891–1.076 unpinned, straddling 1.0.

Every cell, Bun's included, ran much faster than in earlier runs, for
reasons not measured, so these absolute figures do not compare with them.
See "AMENDMENT 2026-10-02 (gate-remeasure-3)".

**Amended 2026-10-03 (gate-remeasure-4):** the same method on `main` at
`7f2b85e`, after "(byte-search)" and "(json-quote)", with six rounds per
cell instead of three. Its figures supersede "(gate-remeasure-3)"'s as the
gate's recorded status:
- **Absolute:** met in all twelve Nova readings, pinned 12010.9–14557.6
  req/sec and unpinned 13200.4–15506.7.
- **Ratio:** not established under either condition. Pinned it is
  0.708–1.334 and unpinned 0.822–1.279; both straddle 1.0.

Every cell, Bun's included, ran slower than in "(gate-remeasure-3)", for
reasons not measured, so these absolute figures do not compare with it.
See "AMENDMENT 2026-10-03 (gate-remeasure-4)".

**Amended 2026-10-03 (gate-ratio-criterion):** the ratio against Bun is
now judged by twelve paired pinned rounds, per
`docs/adr/0021-gate-ratio-paired-rounds.md`. A round counts for Nova only
if Nova's req/sec over Bun's is at least a margin for Bun's `Date` header:
Bun's response bytes over Nova's, never below 1.0, re-measured each run,
and about 1.055 on the sizes last measured. The ratio is met if Nova clears
it in 10 or more of the 12, not met if it falls short in 10 or more, and
inconclusive otherwise. The status above is not re-judged: none of the runs
above took twelve rounds.

**Amended 2026-10-03 (gate-remeasure-5):** the first run judged under
ADR 0021, on `main` at `1972b37`. Its figures supersede
"(gate-remeasure-4)"'s as the gate's recorded status:
- **Absolute:** met in all 24 Nova readings, pinned 13086.8–25031.1
  req/sec and unpinned 13463.0–24419.0.
- **Ratio: inconclusive.** The margin was 713/676, about 1.0547, and Nova
  cleared it in 8 of 12 pinned rounds, where 10 are needed. Nova was faster
  in plain req/sec in all 12, at 1.025–1.161 round by round. Inconclusive
  leaves §5 not met.

The upper ends are from rounds 9 and 10, when every reading, Bun's
included, ran faster, for reasons not measured. So these absolute figures
do not compare with earlier runs'.
See "AMENDMENT 2026-10-03 (gate-remeasure-5)".

**Amended 2026-10-03 (gate-remeasure-6):** the second run judged under
ADR 0021, on `main` at `b24379e`, after "(gc-direct-strings)". Its figures
supersede "(gate-remeasure-5)"'s as the gate's recorded status:
- **Absolute:** met in all 24 Nova readings, pinned 14455.1–22200.2
  req/sec and unpinned 14589.1–22089.4.
- **Ratio: inconclusive.** The margin was again 713/676, about 1.0547.
  Nova cleared it in 7 of 12 pinned rounds, where 10 are needed, and was
  faster in plain req/sec in 8. Inconclusive leaves §5 not met.
- **Unpinned, which does not decide,** Nova cleared the margin in 12 of 12.

The host's speed fell during the run, not steadily, and the pinned round
ratios scattered from 0.907 to 1.517. See
"AMENDMENT 2026-10-03 (gate-remeasure-6)".

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

## AMENDMENT 2026-10-02 (fixed-vs-json): each server measured beside a fixed-response variant

This run asks where a json-api request's time goes, and how that compares
with Bun. Is it in the path that reads a request and sends a response, or
in building the response? It measures each server beside variants that
send the same ten-user response, prepared once at startup.

**Results:**
- **Nova and Bun were level on this run.**
  - Unpinned, Nova's json-api ran at 11392.3–13400.0 req/sec and Bun's twin
    at 11834.1–12844.0. The ranges overlap.
  - Taken round by round, Nova over Bun was 0.963, 1.043 and 0.943.
  - Taken the way "(gate-remeasure)" took its 0.79–0.81 unpinned, from the
    two ranges' extremes, it is 0.89–1.13.
  - **Both servers moved since "(gate-remeasure)", in opposite
    directions.** Nova's readings sit wholly above its 10250.0–10382.4
    there. Bun's sit wholly below its 12846.9–12956.6, by 2.9 req/sec. So
    part of the move to level is Bun reading lower on this run.
  - **This is not a gate measurement.** It has no pinned cells and three
    readings per cell, and it ran on `68d0b94` rather than `5efcc2e`. The
    gate's recorded status stays "(gate-remeasure)"'s until the full method
    is run again.
- **Within Nova, the fixed path is most of a request.**
  - Nova's fixed variant took 52.1–57.5 us per request, against 74.6–87.8
    for the json-api. That fixed path still reads and parses each request
    and builds its `Request`.
  - Response building is the json-api minus the fixed variant: 30.2, 22.1
    and 32.3 us, round by round. It covers the route match, `users_json`,
    `json_response`, `to_bytes`, and the collection their allocations
    trigger.
- **Against Bun, only the totals compare like for like.**
  - Bun's static route is the closest analogue to Nova's fixed variant,
    since both send a response prepared once.
  - But building the request object falls on opposite sides of that
    split. Both runtimes read and parse each request on both sides. Nova's
    fixed variant also builds a `Request`. Bun's handler call, and the
    JavaScript `Request` it receives, fall on the json-api side, unless the
    static route also pays for them, which this run did not check.
  - Split there anyway, Nova's fixed path was slower than Bun's by 19.5, 6.4
    and 4.9 us. Its response building was cheaper than all of Bun's work
    beyond the static route, by 16.2, 9.6 and 0.1 us. Both differences may
    be inflated by the same unmeasured amount: what building the request
    object costs on the side where it falls.
  - Nova's json-api minus Bun's came to 3.3, −3.2 and 4.8 us.
- **Nova's fixed variant, at 52.1–57.5 us per request, beat Bun's
  fetch-handler variant at 61.3–64.6 us.** The ranges are disjoint. That Bun
  variant builds a JavaScript `Request` and a `Response` per request, while
  Nova's builds a `Request` and writes prebuilt bytes. So this is not like
  for like either.
- **Bun's json-api minus its fixed variant is 19.9, 16.5 and 17.9 us**, round
  by round. That covers the json-api handler's `async` Promise, `new URL`,
  the method and path match, and `usersJson`.

### How it was measured

- **The five cells**, each built or run from `68d0b94`:
  - **Nova json-api:** the example, 700,416 bytes, SHA-256
    `c081248a8cb573ca…`.
  - **Nova fixed:** the example with four edits and nothing else, 659,968
    bytes, SHA-256 `01e3d2d845b36438…`. `serve` takes the response's wire
    bytes instead of the store and writes them for every request. `main`
    creates the same ten users through `Store.create`, then builds the wire
    bytes once with `json_response(200, users_json(store)).to_bytes()`.
  - **Bun json-api:** `docs/benchmarks/bun-server.js`.
  - **Bun fixed:** that file's helpers, with the same ten users created at
    startup and `usersJson()` computed once. Its fetch handler is a plain
    function, where the json-api's is `async`, and returns
    `json(200, BODY)` for every request.
  - **Bun static:** the same, but answered from a `routes` entry holding a
    prebuilt `Response`. Whether Bun runs any JavaScript per request on
    that route is not checked here.
- **Provenance, from this session rather than a file:** both Nova binaries
  were built by the release `nova` from `68d0b94`, and Bun is 1.3.0.
  - The three variants are scratch files, never committed, each generated
    by a script.
  - The Nova one asserts each of its four edits matches exactly once.
  - The Bun one asserts its one anchor, `const server = Bun.serve({`, and
    replaces it and everything after it.
- **Method:** "(gate-remeasure)"'s unpinned cells.
  - Ten users, 200 connections, `--warmup 5 --duration 30`.
  - One fresh process per reading, each process's affinity mask read back
    as 4095 (all twelve logical processors).
  - Five cells alternated in the order above, three rounds.
  - Every reading reported `errors=0`.
  - `bun docs/benchmarks/bun-equivalence.js` passed first, with all 9
    exchanges matching. Its log names no binary; that it ran against the
    Nova json-api binary is from this session.
- **Generator ceiling:** the self-test ran at 109246.7 req/sec, against at
  most 26282.9 in any cell, so the generator was not the limit. Its
  connections ranged from 861 to 36430 requests each.
- **Same body, different framing.**
  - All fifteen bodies were 604 bytes with SHA-256 `3ff5004bf26139cc…`.
  - Every Nova head was 72 bytes and every Bun fetch head 109. Bun's own
    `Date` header accounts for the 37-byte difference.
  - The static route's head was 135 bytes, because it also sends a 26-byte
    `etag` line.
  - Nova's two headers came out in either order, varying by process: in
    round 3 both Nova processes sent `content-type` first. Every head was
    read once per reading, with `curl -D`, and compared with its `Date`
    line removed.
- **Per request means wall time at saturation, `1e6 / rps`.** It is not CPU
  time: a Bun process may also run work on other threads, which this run
  did not check.
- **3-second smoke readings, taken first to check framing, pointed the
  other way.** They had Nova's fixed variant at 26235.6 and Bun's json-api
  above its fixed variant. They are not used.

These predictions were written before any variant existed:

| prediction | measured | verdict |
|---|---|---|
| Nova json-api 10,000–11,900 | 11392.3–13400.0 | one of three above |
| Nova fixed 14,000–19,000 | 17376.6–19210.0 | two of three just above |
| Bun json-api 12,000–13,500 | 11834.1–12844.0 | one of three below |
| Bun fixed 13,500–15,500 | 15484.2–16306.8 | two of three above |
| Bun static 18,000–25,000 | 21190.0–26282.9 | one of three above |
| Nova response building 30–40 us | 30.2, 22.1, 32.3 by round | two of three in range |
| Bun `usersJson` 3–8 us | 19.9, 16.5, 17.9 by round, which also covers the `async` Promise, `new URL` and the method and path match | wrong: far above |
| Nova fixed above Bun static by 10–25 us | 19.5, 6.4, 4.9 by round | two of three below |
| most of the gap is response building; but Nova's fixed path is also slower than Bun's static path | little gap to place; Nova's fixed variant was slower than Bun's static route in all three rounds | first half wrong; second right, though that comparison may count building the request object on opposite sides |

### The readings, req/sec, in run order

| cell | round 1 | round 2 | round 3 | us per request |
|---|---|---|---|---|
| Nova json-api | 11392.3 | 13400.0 | 11861.3 | 74.6–87.8 |
| Nova fixed | 17376.6 | 19040.8 | 19210.0 | 52.1–57.5 |
| Bun json-api | 11834.1 | 12844.0 | 12577.4 | 77.9–84.5 |
| Bun fixed | 15484.2 | 16306.8 | 16226.6 | 61.3–64.6 |
| Bun static | 26282.9 | 21670.9 | 21190.0 | 38.0–47.2 |

### Derived, us per request, by round

| difference | round 1 | round 2 | round 3 |
|---|---|---|---|
| Nova json-api − Nova fixed | 30.2 | 22.1 | 32.3 |
| Bun json-api − Bun fixed (`async` Promise, `new URL`, method and path match, `usersJson`) | 19.9 | 16.5 | 17.9 |
| Bun json-api − Bun static | 46.5 | 31.7 | 32.3 |
| Nova fixed − Bun static | 19.5 | 6.4 | 4.9 |
| Nova json-api − Bun json-api | 3.3 | −3.2 | 4.8 |

A round's five readings were taken minutes apart. Readings moved between
rounds, and not all in one direction: from round 1 to round 2, Bun static
fell 17.5% while the other four cells rose 5.3–17.6%. So each per-round
figure carries that variance. Taken from the ranges' extremes instead, Nova
json-api minus Nova fixed is 17.1–35.7 us, and Nova json-api minus Bun
json-api is −9.9 to 9.9.

### What this does not settle

- **The gate.** Its full method has pinned cells too. Apart from this file
  and `CHANGELOG.md`, this partial run amends none of the eight other
  tracked files that cite "(gate-remeasure)":
  `nova-spec/00-MASTER-SPEC.md`, `nova-spec/13-RUNTIME.md`,
  `nova-spec/20-STDLIB.md`, `nova-spec/60-EXAMPLES.md`,
  `docs/phase-2-plan.md`, `docs/adr/0018-std-json-scope-and-build-order.md`,
  `docs/adr/0019-offset-table-intrinsic-boundary.md` and
  `docs/benchmarks/README.md`.
- **What building the request object costs, on each side.** That is what
  keeps the half-by-half comparison with Bun from being like for like.
  Whether Bun's static route also builds one is not checked.
- **What Nova's fixed path spends its 52.1–57.5 us on.**
  "(remainder-split)" put the one socket write at 35.5–38.6 us per request,
  but that was measured in the json-api, on an earlier build. It was not
  measured here.
- **Whether the two would be level on another day.** This is one session,
  one host, Windows, three rounds.

## AMENDMENT 2026-10-02 (gate-remeasure-2): the Phase 2 gate on `main` at `cdaea7e`

This reruns both of the gate's criteria with "(gate-remeasure)"'s method:
ten users, 200 connections, 30 s after a 5 s warmup, four cells
alternated. It takes three replicates per cell, where that run took two.
Since `5efcc2e`, the only functional code change is "(str-chars-direct)"'s;
the other commits changed only documentation and comments.

**The absolute criterion is met in every reading.**
- All six Nova readings clear `nova-spec/00-MASTER-SPEC.md` §3's 10,000
  req/sec: pinned to one core at 10986.5–12403.2, unpinned at
  10768.5–12126.4. That is 7.7–24.0% above the line.
- This is the first matrix whose pinned cells clear it too. So the pooled
  and split readings now agree.

**The ratio against Bun is not met.**
- **Unpinned:** B/D is 0.854–0.976, taken from the ranges' extremes as
  "(gate-remeasure)" did, against the 1.0 `nova-spec/60-EXAMPLES.md` §5
  asks for.
  - The ranges are disjoint. Nova's fastest unpinned reading, 12126.4, is
    below Bun's slowest, 12427.5.
  - Round by round it is 0.962, 0.904 and 0.867.
- **Pinned:** A/C is 0.715–1.334, which straddles 1.0.
  - Bun's pinned readings spread 1.65x, from 9300.1 to 15374.9.
  - Round by round it is 0.774, 1.181 and 0.807. The one round above 1.0 is
    the round with Bun's slowest reading.

**The gate's two statements still disagree.** §3's absolute criterion is
now met under either pinning condition. §5's ratio is not met unpinned and
not established pinned. Under any reading that counts §5, the gate is not
met.

### The cells, in run order

| reading | cell | side | pinned | mask read back | req/sec |
|---|---|---|---|---|---|
| 1 | A | Nova | core 0 | 1 | 11394.6 |
| 2 | C | Bun | core 0 | 1 | 14723.4 |
| 3 | B | Nova | no | 4095 | 12126.4 |
| 4 | D | Bun | no | 4095 | 12605.6 |
| 5 | A | Nova | core 0 | 1 | 10986.5 |
| 6 | C | Bun | core 0 | 1 | 9300.1 |
| 7 | B | Nova | no | 4095 | 11390.0 |
| 8 | D | Bun | no | 4095 | 12599.9 |
| 9 | A | Nova | core 0 | 1 | 12403.2 |
| 10 | C | Bun | core 0 | 1 | 15374.9 |
| 11 | B | Nova | no | 4095 | 10768.5 |
| 12 | D | Bun | no | 4095 | 12427.5 |

- **Every reading was `errors=0`.**
- **Each server was a fresh process.** After it printed its listening line
  and before seeding, it was pinned or left alone, and its affinity mask
  was read back and logged.

| cell | range | ratio |
|---|---|---|
| A, Nova pinned | 10986.5–12403.2 | |
| B, Nova unpinned | 10768.5–12126.4 | |
| C, Bun pinned | 9300.1–15374.9 | A/C 0.715–1.334 |
| D, Bun unpinned | 12427.5–12605.6 | B/D 0.854–0.976 |

**The load generator was not the limit.** Its self-test ceiling, taken
after the equivalence check and before the matrix, is 99203.3 req/sec.
Bun's fastest reading is 15.5% of that.
- It was invoked as `nova-bench-http --self-test --connections 200
  --duration 30 --warmup 5`. That command is from this session.
- The self-test recorded `errors=2`, and at least one connection completed
  no requests (`conn_min=0`, `conn_max=66374`). That makes it weaker
  calibration than "(gate-remeasure)"'s. At 15.5% of it, the conclusion
  does not depend on it.
- The generator is `target/release/nova-bench-http.exe`: 256,512 bytes,
  SHA-256 `d17062335e988c18…`, built 2026-09-29, the same file
  "(gate-remeasure)" used.

**These predictions were written before any throughput reading.** The
files' modification times put the predictions at 16:47:59, the
equivalence log at 16:48:33 and the self-test log at 16:49:13. That the
matrix followed the self-test, in the same command, is from this session.

| prediction | measured | verdict |
|---|---|---|
| A: 8,500–10,500 | 10986.5–12403.2 | wrong: above |
| C: 10,000–12,500 | 9300.1–15374.9 | one reading below, two above |
| B: 10,500–13,500 | 10768.5–12126.4 | within |
| D: 11,500–13,500 | 12427.5–12605.6 | within |
| A/C: 0.70–0.95 | 0.715–1.334 | wrong: upper end above |
| B/D: 0.85–1.10 | 0.854–0.976 | within |
| unpinned above 10k in all three readings | all three | right |
| pinned below 10k in at least two | none below | wrong |
| ratio not met under any reading that counts the pinned cells | not met unpinned; straddles pinned | partly: pinned it is not established rather than not met |

### Identity of each side, and the payload

- **Nova:** `examples/05-json-api`, built by the release `nova` from
  `cdaea7e`; that is from this session. `json-api.exe` is 700,416 bytes,
  SHA-256 `923bef11794d9409…`, and the same binary ran in every Nova
  reading.
- **Bun:** `docs/benchmarks/bun-server.js`, 4,415 bytes in this checkout,
  SHA-256 `f95426e14e22034c…`, run by bun 1.3.0. The version is from
  `bun --version` in this session.
- **The payload.** Ten users were seeded by the same `curl` POSTs on each
  side. Every reading on both sides served the same 604-byte body, SHA-256
  `3ff5004bf26139cc…`.
- **Equivalence was run first**, against the `json-api.exe` built for this
  run: `EQUIVALENCE OK: all 9 exchanges match on status and body bytes`.
  The log does not record the binary's path; that is from this session.
- **Framing was not re-measured in this matrix.** "(fixed-vs-json)"
  measured it on `68d0b94`, whose code is the same: Nova's head is 72 bytes
  and Bun's 109, the difference being Bun's `Date` header.

### Against "(gate-remeasure)", on `5efcc2e`

| cell | (gate-remeasure) | now |
|---|---|---|
| A, Nova pinned | 8520.1, 8424.0 | 11394.6, 10986.5, 12403.2 |
| B, Nova unpinned | 10250.0, 10382.4 | 12126.4, 11390.0, 10768.5 |
| C, Bun pinned | 10774.3, 12068.8 | 14723.4, 9300.1, 15374.9 |
| D, Bun unpinned | 12956.6, 12846.9 | 12605.6, 12599.9, 12427.5 |

- **Nova rose in both cells, disjoint.** Pinned rose by 1.29x–1.47x, and
  unpinned by 1.04x–1.18x.
- **Bun's code did not change, and its cells moved too.** Unpinned fell
  1.9–4.1%, disjoint. Pinned now spans a range that contains its old one.
- **The rise is not attributed.** Nova's one code change,
  "(str-chars-direct)", claimed no server gain: its six pairs overlapped,
  with a 6.0% higher mean. How much of Nova's rise is that change and how
  much is the session is not separated.
- **Pinned and unpinned now overlap on both sides.** In "(gate-remeasure)"
  they were disjoint on both.

### What this does not settle

- **Which statement of the gate governs.** As of `cdaea7e`, no tracked
  file settles it.
- **Whether the unpinned ratio holds below 1.0 on another session.** About
  an hour earlier, "(fixed-vs-json)" ran the same unpinned cells on the same code
  and found Nova and Bun level: Nova 11392.3–13400.0 against Bun
  11834.1–12844.0. That run's unpinned ratio straddled 1.0, at 0.89–1.13;
  this one's falls below it.
- **Why Bun's pinned readings spread 1.65x.** It was not measured.
- **Any host but this one.** Every figure here is from this development
  host, Windows, with the load generator on the same machine.

## AMENDMENT 2026-10-02 (native-floor): what a request's fixed path costs on this host

"(fixed-vs-json)" found that Nova's fixed path, at 52.1–57.5 us, is most of
each json-api request. "(remainder-split)" had put the one socket write at
35.5–38.6 us of a request. This run asks whether that cost is Nova's or this
host's. It measures Nova's fixed variant beside a native Rust server built
in the same shape, and beside Bun's static route.

**Results:**
- **One `send` costs about 34 us or more on this host, in plain Rust too.**
  - The native server times each call. Averaged over windows of 50,000
    responses after the warmup, a send took 33.5–60.3 us. In round 1, whose
    windows varied least, it took 33.9–39.1 us.
  - A recv that returned data took 3.2–5.2 us, and one that would block
    1.4–2.1 us. `WSAPoll` came to 0.85–1.5 us per response.
  - So the send is most of the native server's 42.2–51.9 us per request.
  - "(remainder-split)"'s Nova figures fall inside these native ranges:
    35.5–38.6 us per write, and 1.55–1.73 us per read that would block.
    They come from the json-api, in a profiling build at 7904.6–8531.3
    req/sec. Nova fixed's own system calls were not timed in this run.
- **In this session Bun's static route ran close to the native floor.**
  It was within 2.3 us of native in two of three rounds, and 17.7 us slower
  in the first.
- **Nova's fixed path sat 3.2–16.8 us above that floor.**
  - Nova fixed minus native was 12.9, 16.8 and 3.2 us per request, round by
    round, and 2.1–26.4 us from the ranges' extremes.
  - If Nova's system calls cost what native's do, as "(remainder-split)"'s
    figures suggest, the excess is Nova's own work. It includes parsing the
    request, building its `Request`, scheduling the task, and the
    collection those allocations trigger. It also includes the work
    `try_read` and `try_write` do around the system call, which
    "(remainder-split)" put at 1.65–1.78 and 0.66–0.76 us per request. It
    was not attributed.
  - It is about the size of the gate's remaining unpinned gap. In
    "(gate-remeasure-2)", unpinned Nova took 3.1, 8.4 and 12.4 us more per
    request than unpinned Bun, round by round, and 2.0–13.5 us from the
    ranges' extremes.
- **No effect of `TCP_NODELAY` was resolved.**
  - The native server ran at 20828.7–23022.4 req/sec with it and
    19278.0–23670.8 without. The ranges overlap.
  - Round by round, the two differed in both directions, by −3.8%, +8.0%
    and +16.9%. Native's own readings spread 23%.

### How it was measured

- **The four cells:**
  - **Nova fixed:** "(fixed-vs-json)"'s `nf.exe`, 659,968 bytes, SHA-256
    `01e3d2d845b36438…`, built from `68d0b94`. Through `c12a06c`, `main`
    has changed only documentation since.
  - **Native:** a scratch Rust program, never committed, 187,904 bytes,
    SHA-256 `e0aed1ef05b5b3d2…`.
    - It was built by `cargo build --release` with rustc 1.95.0, against
      `windows-sys` 0.61.2, the version `Cargo.lock` resolves for
      `nova-runtime` at `c12a06c`.
    - It takes the shape of Nova's runtime. One thread waits in `WSAPoll`,
      with no timeout, over the listener and every open connection. Its
      sockets are std non-blocking sockets. It reads each ready connection
      until a read would block. It answers each complete request with one
      std `write` of the whole response.
    - A complete request is a head plus any `content-length` body.
    - The response is a prebuilt 676 bytes: Nova's 72-byte head and the
      604-byte body, read from a file captured from Nova's fixed variant.
  - **Native with `TCP_NODELAY`:** the same binary, setting `TCP_NODELAY` on
    each accepted socket.
  - **Bun static:** "(fixed-vs-json)"'s `bun-static.js`, 2,872 bytes,
    SHA-256 `9f2237da56065aea…`, run by bun 1.3.0.
- **Provenance:** the bun version is from this session rather than a file.
  The rustc version is recorded in the scratch build's
  `target/.rustc_info.json`.
- **Differences from Nova's runtime that do not matter here.**
  - The native server drops a connection on a short write, where Nova
    writes the remainder, parking only if the socket is not writable. No
    short write is evident: a dropped connection would have shown as an
    error, and every reading had `errors=0`.
  - It polls every open connection, where Nova polls only the parked
    ones. When every connection is waiting for its next request, those
    are the same set.
  - Its clock reads add to its own figures. So if anything, Nova's excess
    above it is understated.
- **The native server's own timing.** It reads the clock before and after
  each poll, recv and send, and the per-request figures include those
  reads. It prints mean nanoseconds per call for each window of 50,000
  responses. Each reading's first three windows are excluded. Together they
  span 150,000 responses, more than the seeding and the 5 s warmup came to
  at the measured rates, at most about 118,000.
- **Method:** "(fixed-vs-json)"'s unpinned cells.
  - Ten users POSTed, 200 connections, `--warmup 5 --duration 30`.
  - One fresh process per reading, its affinity mask read back as 4095.
  - Four cells alternated in the order Nova fixed, native, native with
    `TCP_NODELAY`, Bun static, for three rounds.
  - Every reading reported `errors=0`.
- **Same body, same framing.**
  - All twelve bodies were 604 bytes with SHA-256 `3ff5004bf26139cc…`.
  - Every native head matched Nova's byte for byte, in one of the two
    orders Nova's headers come in. Two of the three Nova readings sent the
    other order. Bun static's head was 135 bytes, as before.
- **Ordering.** The predictions' modification time is 17:29:08. The native
  source's is 17:29:32, and its build's 17:29:46.
- **3-second smoke readings again disagreed with the 30-second ones.**
  Three of the four fell below their cell's range; Nova fixed read 9116.3.
  They are not used.

These predictions were written before the native server existed:

| prediction | measured | verdict |
|---|---|---|
| Nova fixed 17,000–19,500 | 14563.7–18541.7 | one of three below |
| native 18,000–30,000 | 19278.0–23670.8 | within |
| native with `TCP_NODELAY` within 5% of native | −3.8%, +8.0%, +16.9% by round | wrong: two of three outside, both faster with `TCP_NODELAY` |
| Bun static 20,000–26,500 | 16669.2–20171.7 | two of three below |
| native send 15–35 us | 33.5–60.3 per window | wrong: 52 of 71 windows above |
| native recv with data 3–10 us | 3.2–5.2 | within |
| native recv that would block 1–3 us | 1.4–2.1 | within |
| native within about 15 us of Bun static | −17.7, 2.3, −0.8 by round | two of three |
| Nova fixed above native by 10–25 us | 12.9, 16.8, 3.2 by round | two of three; one below |

### The readings, req/sec, in run order

| cell | round 1 | round 2 | round 3 | us per request |
|---|---|---|---|---|
| Nova fixed | 18136.3 | 14563.7 | 18541.7 | 53.9–68.7 |
| native | 23670.8 | 19278.0 | 19699.2 | 42.2–51.9 |
| native, `TCP_NODELAY` | 22783.0 | 20828.7 | 23022.4 | 43.4–48.0 |
| Bun static | 16669.2 | 20171.7 | 19389.7 | 49.6–60.0 |

### Derived, us per request, by round

| difference | round 1 | round 2 | round 3 |
|---|---|---|---|
| Nova fixed − native | 12.9 | 16.8 | 3.2 |
| native − Bun static | −17.7 | 2.3 | −0.8 |
| Nova fixed − Bun static | −4.9 | 19.1 | 2.4 |
| native with `TCP_NODELAY` − native | 1.6 | −3.9 | −7.3 |

### Native per-call means, us, over each reading's windows after its third

| reading | windows | send | recv with data | recv that would block |
|---|---|---|---|---|
| native, round 1 | 13 | 33.9–36.8 | 3.4–3.7 | 1.5–1.6 |
| native, round 2 | 10 | 38.5–47.3 | 3.7–4.6 | 1.6–1.9 |
| native, round 3 | 11 | 36.4–60.3 | 3.6–5.2 | 1.5–2.1 |
| `TCP_NODELAY`, round 1 | 13 | 34.6–39.1 | 3.4–4.0 | 1.5–1.6 |
| `TCP_NODELAY`, round 2 | 11 | 34.1–54.1 | 3.2–5.2 | 1.5–1.9 |
| `TCP_NODELAY`, round 3 | 13 | 33.5–47.4 | 3.2–4.5 | 1.4–1.7 |

### What this does not settle

- **What Nova's 3.2–16.8 us above the floor is spent on.** A sampled
  profile of the fixed variant would split it.
- **Why a loopback send costs this much here.** The time is spent inside
  the system call, which nothing here looks into. Whether another I/O model, such as
  overlapped sends through an I/O completion port, would send more cheaply
  is not tested.
- **How far readings drift within one session.** Bun static ran at
  16669.2–20171.7 here, against 21190.0–26282.9 in "(fixed-vs-json)" about
  two hours earlier, from the same file. Nova fixed ran at 14563.7–18541.7
  here, against 17376.6–19210.0 there.
- **Any host but this one.** It is one host, Windows, three rounds.

## AMENDMENT 2026-10-02 (fixed-profile): where Nova's fixed path spends its time

"(native-floor)" left open what Nova's fixed path spends its time on beyond
the native floor. This run samples the fixed variant's server thread with
"(sampled-profile)"'s scratch sampler.

**Results:**
- **Socket system calls are 84.3–87.0% of the fixed variant's server
  thread.** That is the self time inside `ntdll!ZwDeviceIoControlFile`.
  - The send path, inclusive, is 68.8–70.4%: 25.5–32.7 us per request.
  - The receive path, both reads, inclusive, is 14.8–16.4%: 6.0–6.9 us per
    request.
  - Those two are inclusive figures, so they also count the little time
    each spends outside the system call. Their self time inside it is
    68.6–70.3% and 14.3–16.0%.
- **Everything outside those system calls is 13.0–15.7% of the thread,
  4.7–6.8 us per request in these runs.** On a single-threaded runtime that
  holds all of Nova's own work on this path: its runtime, its compiled code
  and its allocator. It is wall time, so it also counts any time the thread
  was descheduled outside a system call. It also holds the socket
  libraries' own user-mode code, 1.7–2.0% of the thread or 0.6–0.8 us per
  request. Without that, Nova's share is 4.1–6.0 us per request. Its
  largest named parts overlap:
  - `read_request`'s own work beyond its receive calls: 2.7–4.0 us per
    request. That includes `parse_request_head` at 1.6–2.3 us, which in
    turn holds 88–92% of `Bytes.slice`'s 0.8–0.9 us.
  - `gc::alloc`, collection included: 5.7–6.8% of the thread, 2.1–3.0 us.
    64–67% of it is called from inside `read_request`.
  - The system heap: 0.8–1.2% of the thread.
  - `WSAPoll`, the readiness wait: 2.1–2.4% of the thread, 0.9–1.0 us.
    Only 37–39% of its samples are inside the system call. Most of the rest
    are in the socket libraries' user-mode code. The part inside the
    13.0–15.7% is 1.3–1.5% of the thread, 0.55–0.61 us.
- **So this run puts 4.7–6.8 us per request outside the system calls.**
  That is about the size of the gate's remaining unpinned gap, 3.1–12.4 us
  round by round in "(gate-remeasure-2)".
- **Response building holds two to five times as much, comparing across
  runs.** It took 22.1–32.3 us per request, round by round, in
  "(fixed-vs-json)". This run's 13.0–15.7% share, applied to that run's
  fixed variant at 52.1–57.5 us, gives 6.8–9.0 us outside the system
  calls there, which makes response building 2.5–4.8 times as large; from
  that run's range extremes, the low end is 1.9 times. Either way it has
  more of Nova's time to cut. That run found it no larger than Bun's
  per-request work beyond its static route, but that comparison split
  building the request object differently on the two sides.

### How it was measured

- **The binary.** The fixed variant from "(fixed-vs-json)", built by the
  release `nova` from `cec2221` with the scratch sampler patch applied. It
  is 691,712 bytes, SHA-256 `b740cba719076a22…`, linked with a symbol map.
  - That the patch was unchanged from "(reprofile)"'s is from this
    session. The patch was reverted afterwards.
  - A json-api built by the rebuilt release `nova` then came out at
    700,416 bytes with no map, the plain build's size. Unlike the profiling
    binary, it does not contain the string `NOVA_PROF_SAMPLE`.
  - `68d0b94..cec2221` changed only documentation, so apart from the
    sampler patch, this binary's code is that of the fixed variant
    measured unsampled before.
- **The sampler** suspends the server thread, walks its stack, then asks
  to sleep 1 ms, as "(sampled-profile)" describes. Samples came 1.57 ms
  apart on the median, as they did there, so it took 8890–9845 samples per
  15 s load window. 99.8–99.9% of the walks ended cleanly.
- **The load:** 200 connections for 15 s, no warmup, one fresh process per
  run, three runs. Every run reported `errors=0` and served the 604-byte
  body.
- **The analysis** is "(reprofile)"'s scripts, unchanged. They counted the
  samples in the 15 s from each run's load start. "(reprofile)" counted the
  14 s starting half a second in; recounting this run that way moved no
  share by more than 0.33 points.
  - Per-request figures are a share times that run's `1e6 / rps`.
  - One new scratch script found where `WSAPoll`'s samples end and counted
    the socket libraries' self time.
- **Throughput with the sampler** was 21522.2–27524.8 req/sec. That is
  above the unsampled fixed variant's 14563.7–19210.0 in "(fixed-vs-json)"
  and "(native-floor)", which used 30 s after a 5 s warmup. The send cost
  here, 25.5–32.7 us, is also below "(native-floor)"'s native 33.5–60.3.
  - The candidates are three: the host was faster during this run, the
    shorter method reads higher, or the sampler itself changes the
    timing, whose cost "(sampled-profile)" left unresolved.
  - Which one is not separated. The shares are this run's result.
- **Ordering.** The predictions' modification time is 20:35:33, the
  profiling binary's 20:36:00, and the end of the first run's samples
  20:36:30.

These predictions were written before the profiling build existed:

| prediction | measured | verdict |
|---|---|---|
| send path 55–68% | 68.8–70.4% | wrong: above |
| receive path 8–14% | 14.8–16.4% | wrong: above |
| `read_request` incl. its receive 15–28%; `parse_request_head` 2–6% | 23.5–24.8%; 4.4–5.2% | within; within |
| `gc::alloc` incl. collection 4–12% | 5.7–6.8% | within |
| system heap 2–7% | 0.8–1.2% | wrong: below |
| `poll::wait` 1–4% | 2.15–2.45% | within |
| throughput with the sampler 13,000–19,000 | 21522.2–27524.8 | wrong: above |
| Nova's excess is mostly `read_request`'s own work plus allocation and collection; the scheduler and poller are small | those are its largest named parts; everything outside the system calls is 4.7–6.8 us | right in kind; its size was not predicted |

### The runs

| | run 1 | run 2 | run 3 |
|---|---|---|---|
| req/sec | 27524.8 | 21522.2 | 24562.5 |
| us per request | 36.3 | 46.5 | 40.7 |
| samples in the load window | 9845 | 8890 | 9757 |
| socket system calls, self | 87.04% | 85.41% | 84.34% |
| send path, inclusive | 70.29% | 70.44% | 68.80% |
| receive path, inclusive | 16.43% | 14.81% | 15.30% |
| `read_request`, inclusive | 23.84% | 23.48% | 24.84% |
| `parse_request_head`, inclusive | 4.37% | 4.98% | 5.22% |
| `gc::alloc`, collection included | 5.67% | 6.41% | 6.82% |
| `WSAPoll`, inclusive | 2.39% | 2.10% | 2.36% |
| of those, inside the system call | 37% | 37% | 39% |
| socket libraries, self | 1.73% | 1.78% | 1.97% |
| `Bytes.slice`, inclusive | 2.09% | 1.87% | 2.27% |
| system heap, inclusive | 0.83% | 1.08% | 1.19% |

### What this does not settle

- **Whether the unsampled build splits the same way.** These are a
  sampled build's shares, at a higher throughput than the unsampled
  readings.
- **"(native-floor)"'s larger per-round differences.** Nova fixed ran
  12.9 and 16.8 us above native in two of its rounds. That is more than all
  of Nova's work outside the system calls here, 4.7–6.8 us. So in those
  rounds Nova's system calls cost more than native's, or the readings
  drifted between cells, or this run's split does not hold there. Which
  one is not separated.
- **Why the send's cost differed between these runs and "(native-floor)"'s.**
  It was 25.5–32.7 us here, against 33.5–60.3 us there.
- **One host, Windows, three runs.**

## AMENDMENT 2026-10-02 (fast-join): `String.join` copies bytes, and `users_json` joins once

"(fixed-profile)" pointed back at response building, and "(reprofile)" had
found `users_json` growing its output two pieces at a time, with each step
copying everything built so far.
- **The language:** a new std-only builtin, `str_join(sep, parts)`, now backs
  `std/strings`' `String.join`. Its runtime function copies every part and
  separator into one buffer sized up front. The Nova-level `join` it
  replaces walked every part character by character into a `[Char]`.
- **The example:** `users_json` now fills a `[String]` sized by
  `s.users.len()` and joins it once with `","`. Bun's twin also joins once
  with `","`, though it pushes onto an empty array rather than sizing one
  by the user count.

**Results:**
- **Per call, both are disjoint.**
  - `users_json` at ten users went from 10166–10337 to 8205–8491 ns, an
    18–20% cut per pair.
  - `",".join` of the ten user objects went from 4987–6463 to 160–181 ns,
    29–39 times faster per pair.
- **The ten-user server is disjoint after three pairs, so a gain is
  claimed.**
  - Before ran at 10734.5–11382.1 req/sec and after at 11893.7–12269.5.
  - After was 6.6–10.8% higher per pair, 5.4–9.1 us less per request.
- **The server saved more per request than the per-call cut.** `users_json`
  got 1.8–2.0 us cheaper per call in the harness, pair by pair. Why the
  server gained more is not measured.

### How it was measured

- **Per call:** a scratch Nova harness, never committed, times 100,000
  calls each of `users_json` and of `",".join` over the ten user objects.
  - It holds copies of the example's `User`, `Store` and `user_json`, and
    both forms of `users_json`. It panics if the two give different
    output.
  - `before` is 526,336 bytes, SHA-256 `ea4f49385b9e0de4…`.
  - `after` is 525,824 bytes, SHA-256 `9be3effdf31a1b97…`.
  - The "before" figure is the old `users_json` under `before`, and the
    "after" figure the new one under `after`. The two were alternated,
    three runs each, one fresh process per run.
- **Server:** ten users, a 604-byte body, 200 connections,
  `--warmup 5 --duration 15`.
  - `before` is 700,416 bytes, SHA-256 `360e5ca596671ec8…`; `after` is
    701,440 bytes, SHA-256 `cfea00f7615e6089…`.
  - Alternated, one fresh process per reading, every reading `errors=0`.
  - Every reading served the same body, SHA-256 `3ff5004bf26139cc…`.
  - `bun docs/benchmarks/bun-equivalence.js` passed against `after`: all 9
    exchanges match.
- **Provenance, from this session rather than a file:**
  - All four binaries were built by the release `nova`, `before` from
    `2556880` and `after` from `bf0dc38`.
  - The equivalence log does not name the binary it ran against.
- **Ordering.** The predictions' modification time is 21:30:56. The
  `before` harness's is 21:31:05, and the code commit is dated 21:36:38.

These predictions were written before any of the change existed:

| prediction | measured | verdict |
|---|---|---|
| `users_json` before 10,000–20,000 ns | 10166–10337 | within |
| `users_json` after 20–40% lower, disjoint | 17.9–19.6% lower per pair, disjoint | wrong: just below |
| `join` before 3,000–8,000 ns | 4987–6463 | within |
| `join` after 200–600 ns | 160–181 | wrong: below |
| server after 2–8% above before, ranges likely overlap | 6.6%, 8.8%, 10.8% per pair, ranges disjoint | wrong: one of three within, two above, and disjoint |
| every existing join test, the json-api golden test and the equivalence check pass unchanged | all did | right |
| the four named mutants each fail at least one named test | all four did | right; a fifth, added later, also failed |

### Per call, ns, in run order

| run | `users_json` before | after | `join` before | after |
|---|---|---|---|---|
| 1 | 10166 | 8297 | 5224 | 181 |
| 2 | 10337 | 8491 | 6463 | 165 |
| 3 | 10205 | 8205 | 4987 | 160 |

The harness also timed the cross terms.
- The old `users_json` under `after` took 10497–10670 ns. It calls no
  `join`, yet it ran 2.9–4.5% slower per pair than under `before`, 1.5–5.0%
  from the ranges' extremes, with the ranges disjoint.
  - Why is not measured. `after` always ran second in each pair, so an
    order effect cannot be told apart from a property of the binary.
  - Either way the 18–20% cut may slightly understate the change. Within
    the `after` binary alone, the new form was 20.4–21.9% faster than the
    old, pair by pair.
- The new `users_json` under `before` took 13353–13784 ns. Joining once
  only paid off once the join itself was cheap.

### Server, in run order

| pair | before | after | after / before |
|---|---|---|---|
| 1 | 11273.7 | 12269.5 | 1.088 |
| 2 | 11382.1 | 12129.7 | 1.066 |
| 3 | 10734.5 | 11893.7 | 1.108 |

### Correctness

- **A new runtime test,
  `join_puts_the_separator_between_every_pair_of_parts`,** checks
  `nova_rt_str_join` against Rust's own `join`. It covers five part shapes,
  from none to multi-byte, under four separators, from empty to
  multi-byte.
- **The existing tests pass unchanged.** They are `String.join`'s two
  split-and-join tests, `strings_run`, `strings_under_gc_stress` (which
  collects on every allocation), `strings_build_standalone`, and
  `json_api_example_serves_its_routes`.
  - `strings_build_standalone` is weaker evidence than the others. It
    links `target/debug/nova_runtime.lib`, which `cargo test -p nova-cli`
    does not refresh, so it passed under all three runtime mutants.
  - In the full-suite run it linked a runtime that has the new function:
    `strings.nova` calls `join`, so the link would fail without
    `nova_rt_str_join`.
- **Five mutants each fail named tests:**
  - the runtime join dropping its separator: 6 tests;
  - putting it after every part: 6;
  - skipping the last part: 6;
  - `users_json` sizing its array one slot too large: 1, the json-api
    route test;
  - `users_json` writing one user short: 1, the same test.
- **The `users_json` change rests on an invariant its comment states.**
  `Store.create` is the only insert and there is no delete route, so every
  id below `next_id` is present and the walk fills every slot.
- **The gates pass.** A scratch log records the base commit, the working
  tree's files and each command:
  - `cargo test --locked --workspace`: 1172 passed, 0 failed, 8 ignored;
  - `cargo clippy --locked --workspace --all-targets --all-features -- -D
    warnings`: exit 0.
  - Both ran before `cargo fmt` reflowed one line of the new type-checker
    test entry. After the reflow, `cargo fmt --all --check` exited 0; the
    log records that. That the signature test then passed again is from
    this session.
  - A later commit changes only comments and one test message string.

### What this does not settle

- **Why the server gained more than the per-call cut.**
  - By arithmetic, not measurement, the old `users_json` made roughly 6 KB
    of intermediate strings per ten-user request.
  - One candidate is that those cost more to collect in the server's heap
    than in the harness's. It was not measured.
- **The gate.** Neither criterion was rerun. "(gate-remeasure-2)"'s
  unpinned gap was 3.1–12.4 us per request, round by round. This change
  saved 5.4–9.1 us per request on the 15 s method, a different method in
  a different session.
- **One host, Windows.**

## AMENDMENT 2026-10-02 (gate-remeasure-3): the Phase 2 gate on `main` at `012ca55`

This reruns both of the gate's criteria after "(fast-join)", with
"(gate-remeasure-2)"'s method: ten users, 200 connections, 30 s after a 5 s
warmup, four cells alternated, three replicates per cell. Since `cdaea7e`,
the only functional code change is "(fast-join)"'s.

**The absolute criterion is met in every reading.** All six Nova readings
clear `nova-spec/00-MASTER-SPEC.md` §3's 10,000 req/sec. Pinned to one core
they ran at 17682.5–18528.1, and unpinned at 17400.4–20014.4. That is
74.0–100.1% above the line.

**The ratio against Bun is not met.**
- **Pinned:** A/C is 0.886–0.993 from the ranges' extremes, wholly below
  the 1.0 `nova-spec/60-EXAMPLES.md` §5 asks for.
  - The ranges are disjoint. Nova's fastest pinned reading, 18528.1, is
    below Bun's slowest pinned reading, 18664.9.
  - Round by round it is 0.929, 0.928 and 0.947: Nova took 3.0–3.9 us more
    per request.
- **Unpinned:** B/D is 0.891–1.076, which straddles 1.0.
  - Round by round it is 1.049, 0.979 and 0.935. Round 1 is above 1.0.

**The two pinning conditions traded places since "(gate-remeasure-2)".**
There, pinned straddled 1.0 and unpinned fell below it. Here, pinned falls
below and unpinned straddles.
- The trade cannot be separated from "(fast-join)", which by its own
  record saved 5.4–9.1 us per request, more than the pinned gap here.
- "(gate-remeasure-2)"'s pinned straddle also rested on one Bun reading,
  9300.1.

§3's absolute criterion is met under either condition. §5's ratio is not met
pinned and not established unpinned, so the gate is still not met.

**Every cell ran much faster than in "(gate-remeasure-2)", Bun's too,
whose code did not change.** From the ranges' extremes:
- Bun unpinned rose 1.48x–1.57x and Bun pinned 1.21x–2.15x;
- Nova pinned rose 1.43x–1.69x and Nova unpinned 1.43x–1.86x.

The change came within about 36 minutes.
- "(fast-join)"'s `after` server, functionally the same code as
  `012ca55`, ran at 11893.7–12269.5 req/sec between 21:39 and 21:41, on
  the 15 s method. This matrix ran from 22:17 to 22:25.
- The generator's self-test ceiling rose only 1.06x, from 99203.3 to
  104971.2, while the servers rose 1.2x–2.2x.
- Why is not measured.

So absolute figures from different runs are not comparable here. The
ratios, taken from alternated readings within one run, are less exposed to
it. Nothing measured whether such a change leaves them alone either.

### The cells, in run order

| reading | cell | side | pinned | mask read back | req/sec |
|---|---|---|---|---|---|
| 1 | A | Nova | core 0 | 1 | 18277.9 |
| 2 | C | Bun | core 0 | 1 | 19670.7 |
| 3 | B | Nova | no | 4095 | 20014.4 |
| 4 | D | Bun | no | 4095 | 19079.2 |
| 5 | A | Nova | core 0 | 1 | 18528.1 |
| 6 | C | Bun | core 0 | 1 | 19963.2 |
| 7 | B | Nova | no | 4095 | 19117.4 |
| 8 | D | Bun | no | 4095 | 19533.0 |
| 9 | A | Nova | core 0 | 1 | 17682.5 |
| 10 | C | Bun | core 0 | 1 | 18664.9 |
| 11 | B | Nova | no | 4095 | 17400.4 |
| 12 | D | Bun | no | 4095 | 18604.7 |

- **Every reading was `errors=0`.**
- **Each server was a fresh process.** After it printed its listening line
  and before seeding, it was pinned or left alone, and its affinity mask
  was read back and logged.

| cell | range | ratio |
|---|---|---|
| A, Nova pinned | 17682.5–18528.1 | |
| B, Nova unpinned | 17400.4–20014.4 | |
| C, Bun pinned | 18664.9–19963.2 | A/C 0.886–0.993 |
| D, Bun unpinned | 18604.7–19533.0 | B/D 0.891–1.076 |

**The load generator was not the limit.** Its self-test ceiling, taken
after the equivalence check and before the matrix, is 104971.2 req/sec.
The fastest reading, Nova's 20014.4, is 19.1% of that.
- It was invoked as `nova-bench-http --self-test --connections 200
  --duration 30 --warmup 5`. That command is from this session.
- The self-test recorded `errors=72`, and at least one connection completed
  no requests (`conn_min=0`, `conn_max=110458`). That is weaker calibration
  than "(gate-remeasure)"'s or "(gate-remeasure-2)"'s. At 19.1% of it, the
  conclusion does not depend on it.
- The generator is `target/release/nova-bench-http.exe`, SHA-256
  `d17062335e988c18…`, the same file "(gate-remeasure)" and
  "(gate-remeasure-2)" used.

**These predictions were written before any throughput reading.** The
files' modification times are 22:16:02 for the predictions, 22:16:32 for
the gate binary, 22:16:42 for the equivalence log and 22:17:19 for the
self-test log. That the matrix followed the self-test, in the same command,
is from this session.

| prediction | measured | verdict |
|---|---|---|
| A: 11,000–14,000 | 17682.5–18528.1 | wrong: above |
| C: 9,000–16,000 | 18664.9–19963.2 | wrong: above |
| B: 11,000–13,500 | 17400.4–20014.4 | wrong: above |
| D: 11,500–13,500 | 18604.7–19533.0 | wrong: above |
| B/D: 0.85–1.15, straddling 1.0 | 0.891–1.076 | within |
| A/C: straddles 1.0 | 0.886–0.993 | wrong: wholly below |
| unpinned, round by round: at least one of three at or above 1.0 | 1.049 in round 1 | right |
| all six Nova readings above 10k | all six | right |
| the ratio is not established as met unpinned | the ranges overlap | right |

All four throughput predictions missed above, on both sides, Bun's
unchanged code included; why is not measured. The ratio predictions held
except the pinned one.

### Identity of each side, and the payload

- **Nova:** `examples/05-json-api`, built by the release `nova` from
  `012ca55`; that is from this session. `json-api.exe` is 701,440 bytes,
  SHA-256 `23437c576dc14112…`, and the same binary ran in every Nova
  reading.
- **Bun:** `docs/benchmarks/bun-server.js`, SHA-256 `f95426e14e22034c…`,
  the same file as before, run by bun 1.3.0. The version is from this
  session.
- **The payload.** Ten users were seeded by the same `curl` POSTs on each
  side. Every reading on both sides served the same 604-byte body, SHA-256
  `3ff5004bf26139cc…`.
- **Equivalence was run first**, against the `json-api.exe` built for this
  run: `EQUIVALENCE OK: all 9 exchanges match on status and body bytes`.
  The log does not record the binary's path; that is from this session.
- **Framing was not re-measured.** The fast join changed only how the body
  is built, not the head.

### What this does not settle

- **Which statement of the gate governs.** As of `012ca55`, no tracked
  file settles it.
- **Whether either ratio holds its side of 1.0 on another run.** The
  pinned and unpinned verdicts have traded places once already, between
  two runs that also differ by "(fast-join)". How much of the trade is the
  host is not separated.
- **Why every cell ran so much faster than about 36 minutes earlier.** It
  was not measured.
- **Any host but this one.** Every figure here is from this development
  host, Windows, with the load generator on the same machine.

## AMENDMENT 2026-10-02 (byte-search): `String.index_of` and `String.contains` search bytes

`Response.to_bytes` checks every header name and value with `std/http`'s
`is_crlf_free`, which calls `String.contains` twice. Each `contains` went
through `String.index_of`, which built a `[Char]` for both strings before
comparing them character by character. For the json-api's two headers
that is eight searches and sixteen character arrays per response.
- **The change:** a new std-only builtin, `str_index_of(haystack, needle)`,
  now backs both methods.
- **How it searches:** its runtime function finds the first match with a
  byte search, then counts the characters before it.
- **Why the result is the same:** both strings are valid UTF-8, and no
  character starts with a continuation byte, so a byte match can only begin
  on a character boundary. It is therefore the same first match the character
  search found.
- **`contains`** now compares the builtin's result with 0 itself, so it no
  longer allocates an `Option`.

**Results:**
- **Per call, all four are disjoint.**
  - `"application/json".contains("\r")`, a miss, went from 168–390 to
    23–26 ns.
  - `"604".contains("\n")`, a miss, went from 82–146 to 13 ns.
  - `"hello wörld, hello wörld".index_of("wörld")`, whose needle holds a
    multi-byte character, went from 144–148 to 46–49 ns.
  - `Response.to_bytes`, for a response with the json-api's two headers
    and a 604-byte body, went from 2265–2278 to 1228–1331 ns. That is a
    41.6–45.8% cut per pair, about 1 us.
- **The ten-user server's ranges overlap over six pairs, so no gain is
  claimed.**
  - Before ran at 12178.4–12928.2 req/sec and after at 12603.0–13035.8.
  - After was faster in 4 of the 6 pairs, and its mean was 1.7% higher.
  - The roughly 1 us `to_bytes` saves is about 1% of a request here,
    smaller than the spread of either side's readings.

### How it was measured

- **Per call:** a scratch Nova harness, never committed. It times 200,000
  calls each of the two `contains` misses, the `index_of`, and `to_bytes`.
  - Its response comes from a copy of the json-api's `json_response`, on a
    604-character body.
  - `before` is 495,616 bytes, SHA-256 `3791e301c5d75c91…`.
  - `after` is 497,152 bytes, SHA-256 `3184eb7e0b8f836c…`.
  - The two were alternated, three runs each, one fresh process per run.
- **Server:** ten users, a 604-byte body, 200 connections,
  `--warmup 5 --duration 15`.
  - `before` is 701,440 bytes, SHA-256 `ba5de69c09cb4358…`; `after` is
    702,464 bytes, SHA-256 `bd67f5424af36c69…`.
  - Alternated, one fresh process per reading, every reading `errors=0`.
  - Every reading served the same body, SHA-256 `3ff5004bf26139cc…`.
  - **Six pairs, not three.** The first three overlapped, so three more were
    taken, under the rule written for the page heap in
    `docs/superpowers/plans/2026-10-01-gc-page-heap.md`: "If the ranges
    overlap, take three more of each and report all readings."
  - `bun docs/benchmarks/bun-equivalence.js` passed against `after`: all 9
    exchanges match.
- **Provenance, from this session rather than a file:**
  - All four binaries were built by the release `nova`, `before` from
    `3ef7031` and `after` from `26b729a`.
  - The equivalence log does not name the binary it ran against.
- **Ordering.** The predictions' modification time is 22:39:32. The
  `before` harness binary's is 22:39:49, and the code commit is dated
  22:42:51.

These predictions were written before any of the change existed:

| prediction | measured | verdict |
|---|---|---|
| `contains` on `"application/json"`: before 150–400 ns, after 20–60 | 168–390; 23–26 | within; within |
| `contains` on `"604"`: before 60–200 ns, after 15–50 | 82–146; 13 | within; wrong: below |
| `index_of` multi-byte: before 200–600 ns, after 40–150 | 144–148; 46–49 | wrong: below; within |
| `to_bytes`: before 1,500–4,000 ns, after 20–50% lower | 2265–2278; 41.6–45.8% lower per pair | within; within |
| server after 1–5% above before, ranges likely overlap | ranges overlap over six pairs; −2.5% to +6.1% per pair, and after's mean req/sec 1.7% above before's | ranges overlap, as predicted; the 1–5% part is not settled |
| the three named mutants each fail at least one named test | all three did | right; two more, added later, also failed |

### Per call, ns, in run order

| run | `contains` ctype before | after | `contains` short before | after | `index_of` before | after | `to_bytes` before | after |
|---|---|---|---|---|---|---|---|---|
| 1 | 390 | 25 | 146 | 13 | 148 | 47 | 2278 | 1331 |
| 2 | 168 | 23 | 86 | 13 | 146 | 46 | 2275 | 1244 |
| 3 | 168 | 26 | 82 | 13 | 144 | 49 | 2265 | 1228 |

The first `before` run's two `contains` readings are above its others:
2.3 times for the `"application/json"` one and 1.7–1.8 times for the
`"604"` one. Why is not measured. The two later runs are disjoint from
every `after` run too.

### Server, in run order

| pair | before | after | after / before |
|---|---|---|---|
| 1 | 12178.4 | 12918.0 | 1.061 |
| 2 | 12928.2 | 12603.0 | 0.975 |
| 3 | 12654.0 | 13035.8 | 1.030 |
| 4 | 12507.5 | 12723.1 | 1.017 |
| 5 | 12731.6 | 12604.2 | 0.990 |
| 6 | 12471.5 | 12877.9 | 1.033 |

### Correctness

- **A new runtime test,
  `index_of_reports_the_first_match_as_a_character_index`,** checks
  `nova_rt_str_index_of` against a character-level reference over twelve
  haystack and needle pairs. They include matches after multi-byte text,
  repeated and overlapping needles, misses, an empty needle and an empty
  haystack.
- **The existing tests pass unchanged.** `tests/runtime/strings.nova`'s
  `index_of` and `contains` lines run in `strings_run`,
  `strings_under_gc_stress` and `strings_build_standalone`.
  `http_serialise_run` pins the header check's refusal of an embedded CRLF.
- **Five mutants each fail named tests:**
  - the runtime reporting a byte index instead of a character index: 3
    tests;
  - the runtime reporting an empty needle missing: 3;
  - the runtime never finding a match: 4, `http_serialise_run` among
    them;
  - `contains` testing `> 0` instead of `>= 0`: 3;
  - `index_of` treating a match at 0 as a miss: 3.
- **`strings_build_standalone` caught only the two `std/strings`
  mutants.** It links `target/debug/nova_runtime.lib`, which
  `cargo test -p nova-cli` does not refresh, as "(fast-join)" found.
- **The gates pass.** A scratch log records the base commit, the working
  tree's files and each command:
  - `cargo test --locked --workspace`: 1173 passed, 0 failed, 8 ignored;
  - `cargo clippy --locked --workspace --all-targets --all-features -- -D
    warnings`: exit 0;
  - `cargo fmt --all --check`: exit 0.
  - A later commit changes only comments.

### What this does not settle

- **The server-level effect.** Its ranges overlap. A 1% shift is smaller
  than this host's spread between readings.
- **The gate.** Neither criterion was rerun, so "(gate-remeasure-3)", on
  `012ca55`, stays the recorded status.
- **`starts_with`, `ends_with` and `split`.** They still build character
  arrays. None of them runs on the measured `GET /users` path; the example
  splits the request path only for its other routes.
- **One host, Windows.**

## AMENDMENT 2026-10-03 (reprofile-2): the json-api's server thread after three string changes

"(reprofile)" sampled the json-api on `82c9c3a`. Since then
"(str-chars-direct)", "(fast-join)" and "(byte-search)" have changed how
strings are built and searched. This run samples it again on `369a82c`,
with the same sampler, "(reprofile)"'s scripts and three new ones, to
re-rank what is left.

**Results:**
- **Allocation is the largest Nova-side cost in two of three runs, and
  most of it is not the collector.**
  - `gc::alloc`, collection included, is 22.7–23.6% of the server thread,
    12.0–13.3 us per request.
  - In the third run `users_json` carried a collection and came to 27.86%.
    Without the collector's samples it is 19.5%, so `gc::alloc` is larger
    in all three on that comparison.
  - The collector, its callees included, is 8.1–8.6% of the thread,
    4.4–4.9 us. Its self time alone is 7.9–8.5%.
  - It is the `LocalKey::with` instance `ce5e268d0d359845`, the one
    "(sampled-profile)" identified by inference. The inference is strong
    here: the instance's only callees are the collector's sweep and mark
    functions, and almost every sample of it in a run lands at one call
    site, as a rare, threshold-triggered collection would.
  - Every one of the collector's samples sits under `gc::alloc`. So the
    rest of `gc::alloc`, 14.4–15.0% or 7.6–8.5 us, is allocation proper.
  - Charging each sample of allocation proper to its leaf, the largest parts
    are:
    - finding a slot (`Pages::alloc_slot`, 4.9–5.1%);
    - `gc::alloc`'s own code (3.8–4.2%);
    - zeroing (`memset`, 2.1–2.6%);
    - a second `LocalKey::with` instance (2.1–2.2%).
- **Where a collection lands moves from run to run.** The collector's
  total stays at 8.1–8.6%, but which allocation triggers it varies.
  - In runs 1 and 2, `Bytes.concat` inside `Response.to_bytes` carried
    8.8–9.3% of the thread. 95–97% of `Bytes.concat`'s samples sat under
    `gc::alloc`, mostly inside the collector.
  - In run 3 it carried 0.62%. There, `users_json`'s interpolation carried
    the collection instead: `nova_rt_str_concat_n` was 15.36% against
    7.04–7.12%.
  - So the shares of `to_bytes`, `users_json` and `handle` swing with it,
    and the stable comparisons are the ones that exclude it.
- **The stable items, per request:**
  - socket system calls, self: 55.8–56.6% of the thread, 29.9–32.1 us; the
    send path alone is 44.4–45.3%, 23.4–25.8 us;
  - `user_json`: 16.6–16.7%, 8.8–9.5 us, of which `quote` is 9.2–9.6%,
    5.1–5.4 us, across its twenty calls;
  - `read_request`, its receive calls included: 17.3–17.8%, 9.4–10.1 us;
  - the system heap: 4.8–5.2%, 2.6–3.0 us.

### How it was measured

- **The binary.** The json-api, built by the release `nova` from
  `369a82c` with "(sampled-profile)"'s scratch sampler patch applied. It is
  739,328 bytes, SHA-256 `cfa80f804df92ec9…`, linked with a symbol map.
  - That the patch was unchanged is from this session. The patch was
    reverted afterwards.
  - A json-api built by the rebuilt release `nova` then came out at 702,464
    bytes, the plain build's size, with no map. Unlike the profiling
    binary, it does not contain the string `NOVA_PROF_SAMPLE`.
- **The load:** ten users seeded, 200 connections for 15 s, no warmup, one
  fresh process per run, three runs. Every run reported `errors=0` and
  served the 604-byte body. With the sampler they ran at 17554.4–18924.7
  req/sec.
  - That is 1.35–1.50 times the unsampled 12603.0–13035.8 that
    "(byte-search)"'s `after` server reached the evening before. Its code
    is functionally the same, measured on the same 15 s method.
  - The host's speed swung between those runs, as "(gate-remeasure-3)"
    found, so the sampler's own cost is not measured here.
- **The sampler** took 9741–9792 samples per 15 s load window.
- **The analysis:**
  - "(reprofile)"'s `analyze.py`, unchanged, over the 15 s from each run's
    load start. "(reprofile)" counted the 14 s starting half a second in.
  - Three new scratch scripts. One counts inclusive shares by frame
    name. One charges each `gc::alloc` sample to its leaf's full symbol,
    which tells the collector's instance from the others. One breaks down
    what sits beneath a named frame.
  - Per-request figures are a share times that run's `1e6 / rps`.
- **Ordering.** The predictions' modification time is 07:23:21, the
  profiling binary's 07:23:49, and the end of the first run's samples
  07:24:14.

These predictions were written before the profiling build existed:

| prediction | measured | verdict |
|---|---|---|
| send path 30–42% | 44.4–45.3% | wrong: above |
| receive path 6–11% | 11.0–12.2% | wrong: above; run 1 at the edge, 11.01 |
| `users_json` 15–25% | 19.3–27.9% | two within; the third carried a collection |
| `quote` 6–12% | 9.2–9.6% | within |
| `gc::alloc`, collection included, 15–25% | 22.7–23.6% | within |
| `read_request` 10–15%; `parse_request_head` 2–4% | 17.3–17.8%; 2.3–3.0% | wrong: above; within |
| `Response.to_bytes` 2–5% | 4.3–12.7% | one within; two carried a collection |
| `json_response` 2–5% | 1.1–1.4% | wrong: below |
| system heap 3–8% | 4.8–5.2% | within |
| throughput with the sampler 10,000–20,000 | 17554.4–18924.7 | within |
| allocation plus collection is the largest Nova-side item, larger than `users_json` | `gc::alloc` 22.7–23.6% against `users_json` 19.3–27.9% | right in two runs; in the third `users_json` carried a collection |

### The runs, inclusive % of the server thread

| | run 1 | run 2 | run 3 |
|---|---|---|---|
| req/sec with the sampler | 17702.9 | 17554.4 | 18924.7 |
| us per request | 56.5 | 57.0 | 52.8 |
| socket system calls, self | 55.82 | 56.38 | 56.64 |
| send path (`ws2_32!send`) | 44.69 | 45.27 | 44.36 |
| receive path (`ws2_32!recv`) | 11.01 | 11.20 | 12.24 |
| `gc::alloc`, collection included | 23.60 | 22.70 | 22.73 |
| — the collector's instance | 8.63 | 8.14 | 8.32 |
| `handle` | 20.57 | 20.72 | 29.31 |
| `users_json` | 19.31 | 19.38 | 27.86 |
| `user_json` | 16.61 | 16.73 | 16.69 |
| — `quote` | 9.62 | 9.22 | 9.58 |
| — — `String.chars` | 1.92 | 1.93 | 1.95 |
| — — `needs_escape` | 1.70 | 1.56 | 1.74 |
| `nova_rt_str_concat_n` | 7.04 | 7.12 | 15.36 |
| `String.join` | 0.70 | 0.82 | 0.82 |
| `Response.to_bytes` | 12.70 | 12.10 | 4.25 |
| — `Bytes.concat` | 9.29 | 8.91 | 0.62 |
| `json_response` | 1.14 | 1.18 | 1.37 |
| `read_request` | 17.35 | 17.79 | 17.77 |
| — `parse_request_head` | 3.00 | 2.95 | 2.33 |
| `poll::wait` | 1.77 | 1.51 | 1.53 |
| system heap | 4.84 | 5.24 | 4.85 |

### What this does not settle

- **What allocation proper costs per object.** The object count per
  request was last measured in "(nary-interpolation)", at 515.9–516.0.
  "(quote-fast-path)", "(fast-join)" and "(byte-search)" have each changed
  it since, so a per-object figure is not derived here.
- **Whether the unsampled build splits the same way.** These are a
  sampled build's shares.
- **One host, Windows, three runs.**

## AMENDMENT 2026-10-03 (alloc-fast-path): a trimmed allocation path with no established cut in allocation's cost

"(reprofile-2)" put allocation proper at 7.6–8.5 us per ten-user request.
`docs/superpowers/specs/2026-10-03-alloc-fast-path-design.md` designed two
trims of `gc::alloc` that keep what it promises:
- **One `HEAP` borrow per allocation instead of two.** The threshold test
  and the stress flag move inside the borrow that takes the slot. No
  `Layout` check is made for a small request.
- **Constant-size zeroing for the eight smallest slot classes,** in place
  of a `memset` call.

Both were implemented, tested and measured. **Together they did not cut
allocation's cost measurably, so the code did not land.** Only the two
together were profiled and timed per allocation; each alone ran only on
the server, which could not resolve 2% at the time. What landed is the four
characterization tests written first, the spec and plan with outcome
notes, and this record.

**Results:**
- **A same-session profile establishes no reduction.** The base and the
  branch were profiled alternately, three sampled runs each.
  - Allocation proper ran at 13.9–15.1% of the server thread on the base
    and 13.7–14.0% on the branch. The ranges overlap, by 0.1 point: all
    three branch readings sit at the bottom of the base's range.
  - The zeroing moved rather than vanished: `memset` fell from 1.9–2.2% to
    0.7%, while `Pages::alloc_slot`, now holding the inline stores, rose
    from 4.3–4.7% to 5.8–6.3%.
  - `gc::alloc`'s own code fell from 3.7–4.7% to 2.1–2.4%, but a new
    `Heap::allocate` frame took 1.1–1.2%.
- **Inlining that frame did not help either.** A third variant marked
  `Heap::allocate` `#[inline(always)]`, and its frame left the map.
  Profiled alternately with the base again, allocation proper was
  13.8–14.7% against 14.0–15.0%. `gc::alloc`'s own code rose back to
  3.0–3.2%. The variant was not kept.
  - In those alternated sampled runs the inlined variant was also disjointly
    slower than the base: 13279.6–13727.8 against 13902.7–14758.6 req/sec.
    That is three runs each, with the sampler running, so it is reported,
    not relied on.
- **Per allocation, nothing is disjoint.** Over five alternated runs of a
  picosecond harness:
  - a two-field record took 17.7–20.7 ns before and 16.0–18.9 after;
  - `"${i}"` took 77.6–87.6 ns before and 73.6–86.0 after. It was faster
    in all five pairs, by 1.8–6.1%, but the ranges overlap.
- **The server's first answer was not reproduced.**
  - Six alternated pairs, `before` always first, had `after` slower in all
    six: 12742.2–12950.0 against 13016.0–13336.8 req/sec. Noise was low
    during those pairs, so `before` always running first stays a candidate
    cause.
  - Five builds then ran alternately over three rounds. They were the
    original `before`, the base rebuilt, the single borrow alone, the
    zeroing alone, and both. Their ranges all overlapped, `after` at
    12599.2–12989.6 inside `before`'s 12560.7–13495.8.
  - Three more pairs with `after` first swung from 11745.1 to 18746.5
    req/sec across six consecutive runs; the first run of a pair was faster
    in 2 of 3.
  - So the host's noise at the time exceeded a 2% effect, and no
    code-level cause for the first answer was found.

### How it was measured

- **Builds.** Release toolchains from `cb4a80a` (the base) and from the
  branch's commits, with the sibling `NAME` and `NAME.exe` deleted before
  every `nova build`. The plain binaries' sizes and SHA-256s are in the
  scratch logs, and the sampled builds' sizes in each run log. Every plain
  server binary was 702,464 bytes, each with a different hash; the sampled
  builds were 739,328 bytes.
- **Per allocation:** a scratch Nova harness, never committed, times
  5,000,000 iterations each of `P { a: i, b: i + 1 }` and `"${i}"` and
  prints picoseconds per iteration. Five alternated runs per binary, one
  fresh process each, with the checks `5000000` and `chars=` the same in
  every run. A first version printed whole nanoseconds: records 17, 17, 17
  before against 16, 16, 18 after, and `"${i}"` 74–75 against 72–73.
- **Server:** ten users, 200 connections, `--warmup 5 --duration 15`, one
  fresh process per reading, every reading `errors=0`, every body the same
  604 bytes. `bun docs/benchmarks/bun-equivalence.js` passed against the
  branch's server first.
- **Profiles:** "(reprofile-2)"'s sampler and method, base and branch built
  with their own symbol maps and run alternately.
  - Allocation proper is `gc::alloc`'s inclusive share minus the
    collector's instance. In each build that instance was confirmed as the
    `LocalKey::with` that is the parent of the sweep and mark samples.
  - The base's samples from the first comparison were overwritten when the
    base was rerun for the second. Their figures here come from this
    session's printed analysis, not from a file.
  - The sampler patch was reverted after each build.
  - A third run set of the branch, sampled before the alternated
    comparison and from the same binary, read 13.94–14.36%. It is left out
    because it was not alternated with the base.

These predictions were written before any fast-path measurement binary was
built. They name the first, whole-nanosecond harness of 2,000,000
iterations and three runs, so the rows use its figures:

| prediction | measured | verdict |
|---|---|---|
| a record: before 20–60 ns, after 10–30% lower, disjoint | 17, 17, 17 ns before; 16, 16, 18 after | wrong: below, and not disjoint |
| `"${i}"`: before 80–200 ns, after 5–20% lower | 74–75 ns before; 1.4–4.0% lower per pair | wrong: below on both |
| server after 2–6% above before, ranges likely overlap | first six pairs after slower in all six; not reproduced | wrong |
| `memset` under `alloc_slot` from 2.1–2.6% to under 1% | 0.7% | right; the cost moved into `alloc_slot` |
| the second `LocalKey::with` instance gone or under 1% | 2.3–2.5% | wrong |
| `gc::alloc`, collection included, 17–21% | 20.2–20.8% | within; the base read 20.0–22.2% in the same session |

### Per allocation, ps, in run order

| run | record before | after | `"${i}"` before | after |
|---|---|---|---|---|
| 1 | 19424 | 15971 | 80845 | 76387 |
| 2 | 17684 | 18910 | 87597 | 85978 |
| 3 | 20695 | 17199 | 79681 | 76379 |
| 4 | 17822 | 17864 | 78596 | 73820 |
| 5 | 17990 | 16217 | 77578 | 73587 |

### The profiles, % of the server thread, in run order

| | base, first | branch | base, second | inlined |
|---|---|---|---|---|
| allocation proper | 14.35, 15.13, 13.93 | 13.74, 14.03, 13.98 | 13.98, 14.65, 14.96 | 14.72, 13.80, 14.20 |
| `memset` | 2.17, 1.94, 2.19 | 0.68, 0.73, 0.73 | 2.09, 2.05, 2.19 | 0.65, 0.58, 0.68 |
| `Pages::alloc_slot` | 4.70, 4.30, 4.57 | 5.83, 6.28, 5.92 | 4.54, 5.01, 4.94 | 6.26, 6.05, 6.26 |
| `gc::alloc`, its own code | 3.99, 4.69, 3.67 | 2.35, 2.14, 2.21 | 3.59, 4.17, 3.78 | 3.12, 3.00, 3.22 |
| `Heap::allocate` | — | 1.14, 1.19, 1.20 | — | — |

### What landed

- **`every_class_hands_out_a_zeroed_slot_over_stale_bytes`** checks all 24
  classes, where an existing test covered only the 64-byte one. On the base
  it fails when a slot is zeroed only 8 bytes deep.
- **`the_crossing_allocation_collects_before_taking_its_slot`** pins that
  the allocation reaching `next_gc` collects before taking its slot. On the
  base it fails with `>` in place of `>=`, and when collection never runs.
- **`small_sizes_are_always_describable`** pins that `heap_layout` accepts
  `SMALL_MAX`. A `heap_layout` rejecting 2,048 bytes fails it when run
  alone; in a full run that mutant aborts the test process through
  `alloc`'s abort path first.
- **`small_max_takes_a_slot_and_one_byte_more_takes_the_large_path`** pins
  the boundary. `<` for `<=` in `alloc` fails it, and the existing
  `sizes_over_small_max_take_the_large_path` too.
- **The every-class test adds what the old one lacked.** Zeroing at most 64
  bytes fails it and passes the existing 64-byte test.
- **The gates pass** on the tests-only branch: `cargo test --locked
  --workspace`, clippy with `-D warnings`, and rustfmt.

### What this does not settle

- **Where allocation's cost really goes.** The trims moved cost between
  frames. On the base the slot search with its zeroing is 6.2–7.1% of the
  thread; that and the thread-local access are left. Changing them needs a structural
  design: handing out runs of free slots, or skipping zeroing for leaves
  the runtime fills at once. The spec set both aside.
- **The server-level effect of any of it.** At the time, readings of
  identical binaries varied by more than a 2% effect.
- **One host, Windows.**

## AMENDMENT 2026-10-03 (json-quote): a string is quoted in one runtime pass

"(reprofile-2)" put `quote` at 5.1–5.4 us of each ten-user request, across
its twenty calls from `user_json`. It built a `[Char]` of the string,
scanned it in Nova with `needs_escape`, and wrapped a clean string in
quotation marks with an interpolation. A string that needed escaping it
rebuilt one character at a time.
- **The change:** a new std-only builtin, `json_quote(s)`, now backs
  `quote`. Its runtime function copies the string's bytes into one buffer,
  escaping as it goes.
- **What it escapes:** a backslash goes before `"` and `\`. The five
  control characters with a short escape become `\n`, `\r`, `\t`, `\b` and
  `\f`, and every other byte below `0x20` becomes `\u00XX` in lowercase
  hex.
- **Why bytes are exact:** every byte of a multi-byte UTF-8 character is
  `0x80` or above, so none can be mistaken for one of these.
- **What went:** `needs_escape`, `escape_control` and `hex_digit` had no
  other caller, and are gone.

**Results:**
- **Per call, all four are disjoint.**
  - `stringify` of the clean name `"User Number 7"` went from 207.3–220.2
    to 102.6–118.0 ns, a 44.6–53.2% cut per pair.
  - `stringify` of the clean email `"user7@example.com"` went from
    219.0–236.4 to 104.5–125.7 ns, 46.2–55.2%.
  - `stringify` of a string holding a quote, a backslash and a newline
    went from 517.2–538.1 to 158.2–181.1 ns, 65.0–70.4%.
  - `users_json` at ten users went from 7688.8–7921.8 to 5331.9–5775.6 ns,
    27.1–31.2%.
- **The ten-user server is disjoint, in both orders, so a gain is
  claimed.**
  - Over six alternated pairs, before ran at 12322.9–12706.9 req/sec and
    after at 13506.0–13988.8.
  - After was 8.6–11.6% faster per pair, 6.3–8.4 us less per request.
  - Three pairs ran `before` first and three `after` first, since
    "(alloc-fast-path)" left run order a candidate confounder. Both orders
    gave disjoint gains: 8.6–11.6% and 9.4–10.2%.

### How it was measured

- **Per call:** a scratch Nova harness, never committed, prints picoseconds
  per call.
  - It times 1,000,000 calls each of the three `stringify`s, and 200,000
    calls of `users_json` over ten users.
  - It holds copies of the example's `User`, `Store`, `user_json` and
    `users_json` at `b0b606a`.
  - `before` is 525,312 bytes, SHA-256 `6b7748ee651f020b…`; `after` is
    525,312 bytes, SHA-256 `4d5e6261116c72bf…`.
  - They were alternated, five runs each, one fresh process per run. Every
    run printed the same output lengths and the same escaped sample,
    `"a\"b\\c\nd"`.
- **Server:** ten users, a 604-byte body, 200 connections,
  `--warmup 5 --duration 15`.
  - `before` is 702,464 bytes, SHA-256 `c535a1e13aa39c53…`; `after` is
    702,464 bytes, SHA-256 `4cd4671bc3c95dce…`.
  - One fresh process per reading, every reading `errors=0`, every body the
    same, SHA-256 `3ff5004bf26139cc…`.
  - `bun docs/benchmarks/bun-equivalence.js` passed against `after` first:
    all 9 exchanges match, one of them an escaped string.
- **Provenance, from this session rather than a file:** all four binaries
  were built by the release `nova`, `before` from `b0b606a` and `after`
  from `487eda1`. The equivalence log does not name its binary.
- **Ordering.** The predictions' modification time is 11:43:36. The
  `before` harness binary's is 11:44:12, and the code commit is dated
  11:48:11.

These predictions were written before any of the change existed:

| prediction | measured | verdict |
|---|---|---|
| clean name: before 150–350 ns, after 40–60% lower, disjoint | 207.3–220.2; 44.6–53.2% lower, disjoint | within |
| clean email: before 150–350 ns, after 40–60% lower, disjoint | 219.0–236.4; 46.2–55.2% lower, disjoint | within |
| escaped string: before 600–2,000 ns, after 70–90% lower | 517.2–538.1; 65.0–70.4% lower | wrong on `before`, below the band; the cut mostly below it, two of five pairs at 70.1% and 70.4%, three at 65.0–69.5% |
| `users_json`: before 6,000–10,000 ns, after 25–45% lower, disjoint | 7688.8–7921.8; 27.1–31.2% lower, disjoint | within |
| server after 2–6% above before, ranges likely overlap | 8.6–11.6% per pair, disjoint in both orders | wrong: above, and disjoint |
| five named mutants each fail at least one named test | all five did | right |

A sixth mutant was added after verification and was not predicted; see
"Correctness".

### Per call, ps, in run order

| run | name before | after | email before | after | escaped before | after | `users_json` before | after |
|---|---|---|---|---|---|---|---|---|
| 1 | 218539 | 107891 | 236426 | 112685 | 526368 | 171038 | 7921838 | 5775642 |
| 2 | 220169 | 102961 | 235464 | 105431 | 538067 | 159304 | 7745016 | 5331909 |
| 3 | 213146 | 118048 | 233649 | 125682 | 517159 | 181117 | 7896571 | 5556175 |
| 4 | 207287 | 102617 | 218950 | 104489 | 528618 | 158213 | 7688781 | 5432010 |
| 5 | 213028 | 109351 | 229583 | 111166 | 521546 | 158849 | 7710537 | 5549863 |

### Server, in run order

| pair | order | before | after | after / before |
|---|---|---|---|---|
| 1 | before first | 12536.0 | 13619.0 | 1.086 |
| 2 | before first | 12706.9 | 13829.0 | 1.088 |
| 3 | before first | 12405.1 | 13838.7 | 1.116 |
| 4 | after first | 12322.9 | 13506.0 | 1.096 |
| 5 | after first | 12492.1 | 13664.0 | 1.094 |
| 6 | after first | 12698.9 | 13988.8 | 1.102 |

### Correctness

- **A new runtime test,
  `json_quote_matches_the_character_level_escaper`,** checks
  `nova_rt_json_quote` against a character-level port of the Nova `quote`
  it replaces. It covers the empty string, plain ASCII, every control
  character below `0x20`, `0x7F`, multi-byte text, strings mixing
  quotes, backslashes and controls, and multi-byte text directly before
  an escape (`"é\"🦀\n"` and `"日本\u{1}語"`).
- **The existing JSON tests pass with their code and expected output
  unchanged; only comments moved:** `json_stringify_run`,
  `json_stringify_escapes_run`, `json_parse_strings_run`,
  `json_round_trip_run`, `json_traits_run` and the json-api route test.
- **Six mutants each fail named tests.** Each was run under two filtered
  commands only: `cargo test -p nova-runtime --lib json_quote` and
  `cargo test -p nova-cli --test run_tests -- json`. The counts are
  failures within those filters, not across the workspace. Every run
  printed a test result line, so none is a crash read as a pass.
  - the backslash escape dropped: 5 tests;
  - uppercase hex: 3;
  - `0x1F` not treated as a control character: 3;
  - the closing quotation mark missing: 8;
  - `0x7F` escaped: 1, the new runtime test. `json_parse_strings.nova`
    sends `0x7F` through `quote`, but it compares two `stringify`
    results that would change together, so it does not pin the boundary;
  - a clean run copied by counting characters instead of bytes: 1, the
    new runtime test, and only through the two inputs added after
    verification. Before them it passed the runtime test, and with them
    it still passes all 15 fixture tests the `json` filter selects.
- **The gates pass.** A scratch log records the base commit, the working
  tree's files and each command:
  - `cargo test --locked --workspace`: 1178 passed, 0 failed, 8 ignored;
  - `cargo clippy --locked --workspace --all-targets --all-features -- -D
    warnings`: exit 0;
  - `cargo fmt --all --check`: exit 0.
- **Notes elsewhere.** These tracked passages described the Nova `quote`
  and now say what replaced it:
  - ADR 0018 named `hex_digit` as a worked example; it carries a dated
    note that the argument stands without it. A second dated note in its
    accumulator roster says `quote`'s accumulator is gone.
  - `nova-spec/20-STDLIB.md`'s accumulator roster carries the same dated
    note.
  - The roster comment in `std/json/lib.nova` no longer counts `quote`
    among the functions appending into a `Vec<Char>`.
  - The example's `README.md` cited `hex_digit` as a shape; it now says
    that function is gone.
  - `tests/runtime/json_stringify_escapes.nova`'s header names
    `nova_rt_json_quote` as where the five short escapes are listed, and
    no longer describes a fast path that no longer exists.

### What this does not settle

- **The gate.** Neither criterion was rerun.
  - "(gate-remeasure-3)" left Nova 3.0–3.9 us per request behind Bun
    pinned, round by round, on its own method and in another session.
    There the host's speed rose 1.2x–2.2x within about 36 minutes, Bun's
    too, so microseconds do not carry from that session to this one.
  - As shares of Nova's own time per request, that pinned gap was
    5.3–7.2%, and this change cut 8.0–10.4% per pair. Those are still two
    methods in two sessions, and Bun was not run here.
  - The last lever of similar size, "(fast-join)", was 6.6–10.8% faster
    per pair on this same 15 s method. "(gate-remeasure-3)" ran after it
    and still found the pinned ratio below 1.0.
- **Whether the server gained more than the per-call cuts predict.** In
  microseconds it looks so: `users_json`'s cut was 2.1–2.4 us per call,
  per pair, and the server saved 6.3–8.4 us per request. But the harness
  runs one function in a loop and the server does not.
  - In shares it is not established. "(reprofile-2)", in another session,
    put `quote` at 9.2–9.6% of the server thread. Per call, `stringify`
    of a string lost 44.6–70.4% of its time, and `quote` is only part of
    that call, so `quote`'s own cut is at least that. That predicts a drop
    of at least 4.1% of the thread, and at most 9.6% if `quote`'s cost
    vanished entirely. The server's time per request fell 8.0–10.4%,
    which overlaps that range.
  - If there is an excess, one candidate is that fewer, smaller
    allocations cost less to collect in the server's heap. It was not
    measured.
- **One host, Windows.**

## AMENDMENT 2026-10-03 (gate-remeasure-4): the Phase 2 gate on `main` at `7f2b85e`

This reruns both of the gate's criteria after "(json-quote)", with
"(gate-remeasure-3)"'s method: ten users, 200 connections, 30 s after a 5 s
warmup, four cells alternated. Since `012ca55`, the functional code changes
are "(byte-search)"'s and "(json-quote)"'s; "(alloc-fast-path)" landed tests
only.

**Six rounds per cell, not three.**
- **Why:** after three rounds, the unpinned ranges overlapped, and the
  pinned ones cleared each other by 1.0% at the extremes. Three more rounds
  of all four cells followed. The rule is borrowed from
  `docs/superpowers/plans/2026-10-01-gc-page-heap.md`, where it governs
  before/after gain claims: "If the ranges overlap, take three more of
  each and report all readings."
- **What the extension could do:** under the extremes rule, added readings
  only widen ranges. So rounds 4–6 could break the pinned result, but could
  never make the unpinned one disjoint.
- **Decided in advance:** before rounds 4–6 ran, the six-round result was
  written down as the headline, with the three-round result reported beside
  it as the one that matches "(gate-remeasure-3)"'s method.

**The absolute criterion is met in every reading.** All twelve Nova readings
clear `nova-spec/00-MASTER-SPEC.md` §3's 10,000 req/sec. Pinned to one core
they ran at 12010.9–14557.6, and unpinned at 13200.4–15506.7. That is
20.1–55.1% above the line.

**The ratio against Bun is not established under either pinning
condition.**
- **Pinned**, the headline condition in `docs/benchmarks/README.md`: A/C
  is 0.708–1.334 from the ranges' extremes, which straddles the 1.0
  `nova-spec/60-EXAMPLES.md` §5 asks for.
  - Round by round it is 1.101, 1.174, 1.066, 0.858, 1.033 and 0.934, so
    Nova was ahead in four rounds of six.
- **Unpinned:** B/D is 0.822–1.279, which also straddles 1.0.
  - Round by round it is 1.039, 0.965, 1.058, 1.112, 1.062 and 1.065, so
    Nova was ahead in five rounds of six.
- **Over the first three rounds alone**, pinned was 1.010–1.269, wholly
  above 1.0. Nova's slowest pinned reading, 12010.9, was above Bun's
  fastest, 11887.2. All three of Bun's pinned readings in rounds 4–6,
  13155.8–16971.5, are above that Nova reading. Unpinned was 0.822–1.221
  over three rounds, straddling.
- **Bun's readings spread more than Nova's.**
  - Pinned: 1.56x against Nova's 1.21x.
  - Unpinned: 1.33x against Nova's 1.17x.

§3's absolute criterion is met under either condition. §5's ratio is not
established under either, so the gate is still not met.

**Against "(gate-remeasure-3)", the pinned rounds sit higher, but the
ranges overlap, so no move is established.**
- **There:** the three pinned rounds were 0.928–0.947, median 0.929, and
  Nova was behind in every one. The extremes were 0.886–0.993.
- **Here:** the six pinned rounds were 0.858–1.174, median 1.049. The
  extremes, 0.708–1.334, contain gate3's.
- **The two overlap on every view.** Round 6's 0.934 sits inside gate3's
  rounds, and round 4's 0.858 below them. The rule borrowed above says not
  to claim a gain on overlapping ranges.
- **Unpinned**, the six rounds were 0.965–1.112, median 1.060, against
  gate3's 0.935–1.049, median 0.979. These overlap too.
- Medians are not the criterion; they only describe where the rounds fell.

**Every cell ran slower than in "(gate-remeasure-3)", Bun's too, whose code
did not change.** From the ranges' extremes:
- Bun pinned fell to 0.55x–0.91x of its readings there, and Bun unpinned
  to 0.62x–0.86x.
- Nova pinned fell to 0.65x–0.82x, and Nova unpinned to 0.66x–0.89x.
- The generator's self-test ceiling fell only to 0.945x, from 104971.2 to
  99242.4.
- Why is not measured.

So, as "(gate-remeasure-3)" found in the other direction, absolute figures
from different runs are not comparable here.

### The cells, in run order

| reading | round | cell | side | pinned | mask read back | req/sec |
|---|---|---|---|---|---|---|
| 1 | 1 | A | Nova | core 0 | 1 | 12010.9 |
| 2 | 1 | C | Bun | core 0 | 1 | 10910.6 |
| 3 | 1 | B | Nova | no | 4095 | 13200.4 |
| 4 | 1 | D | Bun | no | 4095 | 12703.0 |
| 5 | 2 | A | Nova | core 0 | 1 | 13849.0 |
| 6 | 2 | C | Bun | core 0 | 1 | 11801.0 |
| 7 | 2 | B | Nova | no | 4095 | 15506.7 |
| 8 | 2 | D | Bun | no | 4095 | 16064.0 |
| 9 | 3 | A | Nova | core 0 | 1 | 12672.5 |
| 10 | 3 | C | Bun | core 0 | 1 | 11887.2 |
| 11 | 3 | B | Nova | no | 4095 | 13695.7 |
| 12 | 3 | D | Bun | no | 4095 | 12945.7 |
| 13 | 4 | A | Nova | core 0 | 1 | 14557.6 |
| 14 | 4 | C | Bun | core 0 | 1 | 16971.5 |
| 15 | 4 | B | Nova | no | 4095 | 13485.0 |
| 16 | 4 | D | Bun | no | 4095 | 12121.8 |
| 17 | 5 | A | Nova | core 0 | 1 | 13586.6 |
| 18 | 5 | C | Bun | core 0 | 1 | 13155.8 |
| 19 | 5 | B | Nova | no | 4095 | 13303.6 |
| 20 | 5 | D | Bun | no | 4095 | 12522.4 |
| 21 | 6 | A | Nova | core 0 | 1 | 12556.8 |
| 22 | 6 | C | Bun | core 0 | 1 | 13438.3 |
| 23 | 6 | B | Nova | no | 4095 | 13522.4 |
| 24 | 6 | D | Bun | no | 4095 | 12699.1 |

- **Every reading was `errors=0`.**
- **Each server was a fresh process.** After it printed its listening line
  and before seeding, it was pinned or left alone. Its affinity mask was
  then read back and logged.

| cell | six rounds | ratio | rounds 1–3 | ratio |
|---|---|---|---|---|
| A, Nova pinned | 12010.9–14557.6 | | 12010.9–13849.0 | |
| B, Nova unpinned | 13200.4–15506.7 | | 13200.4–15506.7 | |
| C, Bun pinned | 10910.6–16971.5 | A/C 0.708–1.334 | 10910.6–11887.2 | A/C 1.010–1.269 |
| D, Bun unpinned | 12121.8–16064.0 | B/D 0.822–1.279 | 12703.0–16064.0 | B/D 0.822–1.221 |

**The load generator was not the limit.**
- **Ceiling:** the self-test, taken after the equivalence check and before
  the matrix, reached 99242.4 req/sec. The fastest reading, Bun's 16971.5,
  is 17.1% of that.
- **Command:** `nova-bench-http --self-test --connections 200 --duration
  30 --warmup 5`.
- **Calibration:** the self-test recorded `errors=0`, and every connection
  completed requests (`conn_min=5183`, `conn_max=31178`). That is cleaner
  than "(gate-remeasure-3)"'s, which had `errors=72` and `conn_min=0`.
  - Its `elapsed_ms` is 34693, while every cell's is 30023–30042. Why is
    not known.
- **Binary:** `target/release/nova-bench-http.exe`, SHA-256
  `d17062335e988c18…`, the same file "(gate-remeasure)",
  "(gate-remeasure-2)" and "(gate-remeasure-3)" used.

**These predictions were written before any throughput reading.**
- **The predictions file was created at 14:00:36**, the time its first
  `date` line also records. The addendum moved its modification time to
  14:10:57. So that the original predictions were left unedited then is
  from this session.
- **The other files' modification times:**
  - 14:01:05 for the release `nova`, rebuilt from `7f2b85e`;
  - 14:01:13 for the gate binary;
  - 14:01:42 for the equivalence log;
  - 14:02:25 for the self-test log.

The first run's log has rounds 1–3 ending at 14:10:16. The predictions
file's addendum is dated 14:10:57, and the second run's log has rounds 4–6
starting at 14:11:02.

| prediction | measured | verdict |
|---|---|---|
| A: 11,000–22,000 | six rounds 12010.9–14557.6; rounds 1–3 12010.9–13849.0 | within |
| C: 10,000–22,000 | six rounds 10910.6–16971.5; rounds 1–3 10910.6–11887.2 | within |
| B: 11,000–23,000 | six rounds and rounds 1–3 both 13200.4–15506.7 | within |
| D: 10,000–22,000 | six rounds 12121.8–16064.0; rounds 1–3 12703.0–16064.0 | within |
| rounds 1–3, pinned round by round: 0.98–1.08, at least two of three at or above 1.0 | 1.101, 1.174, 1.066 | wrong on the band, two of three above it; right that at least two were at or above 1.0 |
| rounds 1–3, pinned extremes straddle 1.0 | 1.010–1.269 | wrong: wholly above |
| rounds 1–3, unpinned round by round: 0.98–1.18, at least two of three at or above 1.0 | 1.039, 0.965, 1.058 | one of three below the band; right that two were at or above 1.0 |
| rounds 1–3, unpinned extremes straddle 1.0 or lie above it | 0.822–1.221 | right: straddles |
| rounds 1–3: all six Nova readings above 10k | all six | right |
| rounds 1–3: "the ratio criterion is NOT established as met under BOTH conditions (at least one condition's ranges overlap Bun's)" | unpinned overlaps | right |
| addendum, six rounds: pinned does not stay disjoint above 1.0 | 0.708–1.334 | right |
| addendum, six rounds: unpinned still straddles 1.0 | 0.822–1.279 | right |
| addendum: all twelve Nova readings above 10k | all twelve | right |

The pinned three-round prediction was wrong, and the addendum's six-round
one was right. The addendum was written after seeing the first three
rounds.

### Identity of each side, and the payload

- **Nova:** `examples/05-json-api`, built by the release `nova` from
  `7f2b85e`; that is from this session.
  - `json-api.exe` is 702,464 bytes, SHA-256 `3316d6665e2b2197…`. The same
    binary ran in every Nova reading.
  - **A SHA names a file, not a source tree.** A second build of the same
    source, seconds later, came out at a different SHA-256,
    `62277e6e16fe9f44…`, first differing at byte 273. That build was
    deleted, so this is from this session. It means this binary's SHA
    differing from "(json-quote)"'s `after` server's does not by itself
    show a different program.
- **Bun:** `docs/benchmarks/bun-server.js`, SHA-256 `f95426e14e22034c…`,
  the same file as before, run by bun 1.3.0. The version is from this
  session.
- **The payload.** Ten users were seeded by the same `curl` POSTs on each
  side. Every reading on both sides served the same 604-byte body, SHA-256
  `3ff5004bf26139cc…`.
- **Equivalence was run first**, against the `json-api.exe` built for this
  run: `EQUIVALENCE OK: all 9 exchanges match on status and body bytes`.
  The log does not record the binary's path. The scratch script that ran
  it, `run.sh`, passes the `json-api.exe` in this run's directory.

### What this does not settle

- **Which statement of the gate governs.** As of `7f2b85e`, no tracked
  file settles it.
- **How the ratio could be settled on this host.** Under the extremes
  rule, added readings only widen ranges, so more rounds of one run cannot
  turn an overlap into a disjoint result. That needs a separate run, a
  quieter host, or a different criterion. Nothing here measured what makes
  Bun's readings spread 1.56x pinned.
- **Whether the pinned ratio moved since "(gate-remeasure-3)", and if so
  how much of it is "(json-quote)".** The runs also differ by
  "(byte-search)" and by the host's speed.
- **Why every cell ran slower than in "(gate-remeasure-3)".** It was not
  measured.
- **Any host but this one.** Every figure here is from this development
  host, Windows, with the load generator on the same machine.

## AMENDMENT 2026-10-03 (gate-remeasure-5): the Phase 2 gate on `main` at `1972b37`, the first run judged under ADR 0021

This is the first gate run judged by `docs/adr/0021-gate-ratio-paired-rounds.md`.
- **Rounds:** twelve, fixed in advance, each a pinned pair and then an
  unpinned pair. Odd rounds ran Nova first and even rounds Bun first.
- **Pinned decides,** with a margin for Bun's `Date` header.
- **Method otherwise:** ten users, 200 connections, 30 s after a 5 s
  warmup.
- **Code:** since "(gate-remeasure-4)"'s `7f2b85e`, only docs have
  changed.

**§5's ratio is inconclusive.**
- **The margin:** measured before the first reading, Nova's `/users`
  response was 676 bytes and Bun's 713, so the margin is 713/676, about
  1.0547.
- **Nova cleared it in 8 of the 12 pinned rounds.** The ADR needs 10 for
  met, and 10 short of it for not met.
- **Nova was faster in plain req/sec in all 12 pinned rounds,** at
  1.025–1.161 round by round, median 1.079.
  - The four rounds that did not clear the margin were 1.025, 1.044,
    1.047 and 1.048.
  - The ADR counts a round for Nova only at or above the margin. It
    records, under "Consequences", that the margin can deny a Nova that is
    faster in plain req/sec.
- **Unpinned, which does not decide,** Nova cleared the margin in 9 of 12
  rounds and was faster in 10.

**§3's absolute criterion is met in every reading.** All 24 Nova readings
clear `nova-spec/00-MASTER-SPEC.md` §3's 10,000 req/sec: pinned at
13086.8–25031.1, unpinned at 13463.0–24419.0.

So the gate is still not met: the ratio is inconclusive, and the ADR says
inconclusive leaves §5 not met.

**The host's speed shifted mid-run.**
- Rounds 1–7 ran at 11434.4–16377.6 on both sides. Round 8's unpinned
  pair ran at 16470.3 and 19664.0, rounds 9 and 10 at 19238.2–25031.1, and
  rounds 11 and 12 fell back to 12488.6–13949.9.
- Over rounds 9 and 10, the pinned round ratios were 1.044 and 1.121,
  inside the run's own 1.025–1.161.
- Three of the four pinned rounds that fell short, 9, 11 and 12, came
  during or after the speed-up.
  - Rounds 1–8 cleared the margin in 7 of 8, median 1.085.
  - Rounds 9–12 cleared it in 1 of 4, median 1.048.
  - Whether the speed change moved the ratio is not established.
- Two unpinned pairs straddled a change, with Nova second in both. Round
  8's caught the speed-up and round 10's the slow-down.
- Whole-run ranges are therefore wide. Nova pinned spread 1.91x and Bun
  pinned 2.01x, and the extremes ratio is 0.570–2.189. That is what the
  extremes rule would have had to judge.
- Why the host sped up is not measured.

### The rounds, in run order

| round | order | Nova pinned | Bun pinned | pinned ratio | clears 1.0547 | Nova unpinned | Bun unpinned | unpinned ratio |
|---|---|---|---|---|---|---|---|---|
| 1 | Nova first | 13691.0 | 12752.1 | 1.074 | yes | 13731.7 | 16377.6 | 0.838 |
| 2 | Bun first | 13777.0 | 12697.8 | 1.085 | yes | 13724.5 | 12964.9 | 1.059 |
| 3 | Nova first | 14949.2 | 12879.6 | 1.161 | yes | 15477.8 | 12482.5 | 1.240 |
| 4 | Bun first | 13162.3 | 11434.4 | 1.151 | yes | 13463.0 | 12851.8 | 1.048 |
| 5 | Nova first | 13905.9 | 12992.3 | 1.070 | yes | 14019.9 | 13239.2 | 1.059 |
| 6 | Bun first | 13484.6 | 13161.3 | 1.025 | no | 13970.8 | 13105.7 | 1.066 |
| 7 | Nova first | 13495.8 | 12442.2 | 1.085 | yes | 13970.8 | 13005.3 | 1.074 |
| 8 | Bun first | 13897.8 | 12448.2 | 1.116 | yes | 19664.0 | 16470.3 | 1.194 |
| 9 | Nova first | 23969.8 | 22949.7 | 1.044 | no | 24419.0 | 22575.5 | 1.082 |
| 10 | Bun first | 25031.1 | 22322.1 | 1.121 | yes | 19238.2 | 22670.2 | 0.849 |
| 11 | Nova first | 13311.5 | 12715.1 | 1.047 | no | 13518.7 | 12789.9 | 1.057 |
| 12 | Bun first | 13086.8 | 12488.6 | 1.048 | no | 13949.9 | 12977.2 | 1.075 |

- **Pinned:** clears in 8 of 12, 4 of the 6 Nova-first rounds and 4 of the
  6 Bun-first. Nova was faster in 12 of 12.
- **Unpinned:** clears in 9 of 12. Nova was faster in 10.
  - Readings out of line with the rest of their round sit in rounds Nova
    lost and won alike. In the two it lost, they are Bun's 16377.6 in
    round 1 and Nova's 19238.2 in round 10. Round 8's pair, 16470.3 and
    19664.0, is out of line the same way, and Nova won it at 1.194.
- **Every reading** was `errors=0`, every pinned reading's mask read back
  as 1, and every unpinned one as 4095.
- **The order** in the log matches the ADR's, reading for reading.
- **Each server was a fresh process,** pinned or left alone after it
  printed its listening line and before seeding.

**The load generator was not the limit.**
- **Ceiling:** the self-test, taken after the byte measurement and before
  the first reading, reached 97784.1 req/sec. The fastest reading, Nova's
  25031.1, is 25.6% of that.
- **Calibration:** `errors=0`, `conn_min=607`, `conn_max=54208`.
  - Its `elapsed_ms` is 36424, while every cell's is 30018–30056. Why is
    not known; "(gate-remeasure-4)"'s was 34693.
- **Binary:** `target/release/nova-bench-http.exe`, SHA-256
  `d17062335e988c18…`, the same file "(gate-remeasure)" through
  "(gate-remeasure-4)" used.

**These predictions were written before the gate binary was built, and
before any byte measurement or reading.** The files' modification times:
- 16:51:19 for the predictions;
- 16:51:36 for the gate binary;
- 16:51:58 for the equivalence log;
- 16:52:04 for the byte measurement's log;
- 16:52:46 for the self-test log.

The run's log has the first round ending at 16:55:23 and the last at
17:23:53. The predictions file's modification time, 16:51:19, matches the
`date` line it ends with, so it was not edited after it was written.

| prediction | measured | verdict |
|---|---|---|
| response bytes: Nova 676, Bun 713; m = 1.0547 | 676 and 713; 1.0547 | right |
| pinned rounds clearing m: 4–8 of 12 | 8 | within, at the top |
| verdict: inconclusive (3–9 clears) | 8 clears, inconclusive | right |
| pinned rounds with Nova faster at all: 6–10 of 12 | 12 | wrong: above |
| unpinned rounds clearing m: 5–9 of 12 | 9 | within, at the top |
| all 24 Nova readings above 10k | all 24 | right |
| Nova pinned 10,000–22,000 | 13086.8–25031.1 | wrong: two readings above, in rounds 9 and 10 |
| Bun pinned 9,000–22,000 | 11434.4–22949.7 | wrong: two readings above, in rounds 9 and 10 |
| Nova unpinned 10,000–23,000 | 13463.0–24419.0 | wrong: one reading above, in round 9 |
| Bun unpinned 9,000–22,000 | 12482.5–22670.2 | wrong: two readings above, in rounds 9 and 10 |

The verdict and the margin were predicted right. Each of the three
round-count predictions landed at or above the top of its band. This run's
pinned rounds favoured Nova more than "(gate-remeasure-4)"'s did, on the
same Nova code. The four absolute bands missed only on rounds 9 and 10,
when the host sped up.

### Identity of each side, and the payload

- **Nova:** `examples/05-json-api`, built by the release `nova` at
  `target/release/nova.exe`, SHA-256 `2bb3fef2a47b00b2…`.
  - That file was built from `7f2b85e` for "(gate-remeasure-4)". Its
    modification time, 14:01:05, predates `1972b37`'s commit at 16:47:32.
    `cargo build --release` on `1972b37` left it in place, since only docs
    changed between them; that it was run is from this session.
  - `json-api.exe` is 702,464 bytes, SHA-256 `c458804970c1cdc2…`, and the
    same binary ran in every Nova reading.
  - Its SHA differs from "(gate-remeasure-4)"'s `3316d6665e2b2197…` at the
    same size. The two files differ in four bytes, at offsets 273–274 and
    654389–654390. The first pair is where "(gate-remeasure-4)" found two
    builds of one source differ.
- **Bun:** `docs/benchmarks/bun-server.js`, SHA-256 `f95426e14e22034c…`,
  the same file as before, run by bun 1.3.0. The version is from earlier
  this session.
- **The payload.** Ten users were seeded by the same `curl` POSTs on each
  side. Every reading on both sides served the same 604-byte body, SHA-256
  `3ff5004bf26139cc…`.
- **The byte measurement.** For each side, a fresh server was seeded the
  same way, and one `GET /users` was saved with `curl -s -D`.
  - Nova's head was 72 bytes: status line, `content-type` and
    `content-length`.
  - Bun's was 109: the same three, capitalised, plus `Date`.
  - Both bodies were the 604 bytes above.
- **Equivalence was run first**, against this run's `json-api.exe`:
  `EQUIVALENCE OK: all 9 exchanges match on status and body bytes`. The
  log does not record the binary's path. The scratch script that ran it
  passes the `json-api.exe` in this run's directory.

### What this does not settle

- **Whether the margin is the right size.**
  - The verdict turned on it. Nova was faster in every pinned round, but
    four rounds fell between 1.0 and 1.0547.
  - ADR 0021 set the margin to err against Nova on purpose. The four
    short rounds, 1.025–1.048, are within the few percent of 1.0 that the
    2026-09-11 design says must not be reported as a pass.
  - The margin charges the 37-byte `Date` header at its full share of
    the response. Its real throughput cost is not measured, and ADR 0021
    says a later ADR can lower the margin on such a measurement.
  - ADR 0021 does not say whether a later margin would re-judge this run.
    It says a run is not repeated on unchanged code to replace its verdict.
- **Why the host sped up in rounds 8–10,** and slowed again.
- **Which statement of the gate governs.** As of `1972b37`, no tracked
  file settles it.
- **Any host but this one.** Every figure here is from this development
  host, Windows, with the load generator on the same machine.

## AMENDMENT 2026-10-03 (reprofile-3): the json-api's server thread after `json_quote`

"(reprofile-2)" sampled the json-api on `369a82c`. Since then
"(json-quote)" has made `std/json`'s `quote` a runtime builtin, and
"(gate-remeasure-5)" found the ratio against Bun inconclusive on a 1.0547
margin. This run samples the json-api again on `ef43fdf`, with the same
sampler and "(reprofile-2)"'s scripts, to pick the next lever.

**Results:**
- **`quote` fell to 4.1–4.4% of the server thread, 2.0–2.1 us per
  request,** from 9.2–9.6%. Nearly all of it is now `nova_rt_json_quote`.
- **Inside `nova_rt_json_quote`, its own code is the smaller part.**
  Splitting its samples:
  - its own code, the escaping loop plus `gc_str`'s inlined copy into GC
    memory, is the leaf in 23.4–24.8%;
  - the system heap, `RtlAllocateHeap` and `RtlFreeHeap`, is the leaf in
    32.7–35.9%. That is the Rust `String` it builds and then copies into
    the GC heap;
  - 35–39% sit under `gc::alloc`, for the GC string it returns.
- **The system heap is 4.7–4.9% of the thread, 2.3–2.4 us per request,
  and runtime builtins that build a Rust buffer first account for most of
  it.** Charging each heap sample to its nearest runtime caller:
  - `nova_rt_json_quote` 1.33–1.49%;
  - `nova_rt_str_concat_n` 1.14–1.23%;
  - `nova_rt_int_to_str` 0.87–1.01%;
  - `net::try_read` 0.35–0.52%;
  - `nova_rt_str_join` 0.20–0.33%;
  - `nova_rt_bytes_concat` 0.15–0.25%;
  - `str::to_lowercase` 0.07–0.17%.
- **Allocation, collection included, outweighs every function named
  here except those that call into the rest.**
  - `gc::alloc` is 18.2–18.6% of the thread, 8.7–9.0 us per request.
  - That is more than `std/json`'s `stringify` at 4.3–4.6%, `std/http`'s
    `read_request` at 13.9–15.5% and `Response.to_bytes` at 4.9–5.6%, and
    the example's own `users_json` at 14.2–17.3%.
  - Frames that call into the rest are larger inclusive. The example's
    `handle` is 18.5–21.2%, and 14.9–15.7% without the collector's
    samples.
  - The collector's instance, `LocalKey::with` `ce5e268d0d359845` as in
    "(reprofile-2)", is 6.8–7.3%, or 3.3–3.5 us. All of it sits under
    `gc::alloc`, as in "(reprofile-2)".
  - The rest of `gc::alloc`, allocation proper, is 10.9–11.3%, 5.2–5.4 us.
    Charged by leaf, its parts are:
    - finding a slot, 3.9–4.2%;
    - `gc::alloc`'s own code, 2.7–3.0%;
    - zeroing, 1.6–2.0%;
    - the second `LocalKey::with` instance, `d03a43526a1fdc50`, 1.4–1.6%;
    - `FnOnce::call_once`, 0.7–1.0%.
- **The stable items, per request:**
  - socket system calls, self: 65.4–66.5% of the thread, 31.5–32.0 us. The
    send path alone is 55.6–56.0%, 26.7–27.0 us;
  - `read_request`, its receive calls included: 13.9–15.5%, 6.7–7.4 us;
  - `user_json`, without the collector's samples: 11.2–11.5%, 5.4–5.5 us.
    Run 1's 13.54% carried 2.32 points of a collection;
  - `Response.to_bytes`, without the collector's samples: 3.7–3.8%,
    1.8 us. It carried 1.12–1.80 points of a collection in every run.
- **Where a collection lands still moves from run to run.** The
  collector's samples, in points of the thread, under each frame:

  | frame | run 1 | runs 2 and 3 |
  |---|---|---|
  | `json_response` | 0.00 | 5.01, 5.51 |
  | `Bytes.concat` | 0.00 | 1.79, 1.76 |
  | `nova_rt_str_concat_n` | 1.12 | 0.02, 0.00 |
  | `user_json` | 2.32 | 0.01, 0.00 |

  So `json_response` was 1.16% in run 1 and 6.23–6.91% in runs 2 and 3.

**What this suggests, not measured:** a builtin that wrote its result
straight into the GC string, instead of building a Rust buffer and copying
it, would skip one system-heap allocation, one free and one copy per call.
- The three largest such callers hold 3.5–3.6% of the thread between them,
  about 1.7 us per request.
- For scale, the shortest of "(gate-remeasure-5)"'s pinned rounds, 1.025,
  needed about 2.9% more to reach the 1.0547 margin. These are shares of a
  sampled build in another run, so they do not say whether removing the
  buffers would close that.

### How it was measured

- **The binary.** The json-api, built by the release `nova` from
  `ef43fdf` with "(sampled-profile)"'s scratch sampler patch applied. It is
  738,816 bytes, SHA-256 `b8b7ac238b1b04c3…`, linked with a symbol map.
  - The patch file is the one "(reprofile-2)" used; that is from this
    session. It was reverted afterwards.
  - A json-api built by the rebuilt release `nova` then came out at 702,464
    bytes, the plain build's size, with no map. Unlike the profiling
    binary, it does not contain the string `NOVA_PROF_SAMPLE`.
- **The load:** ten users seeded, 200 connections for 15 s, no warmup, one
  fresh process per run, three runs. Every run reported `errors=0` and
  served the 604-byte body. With the sampler they ran at 20662.7–20800.5
  req/sec. The sampler's own cost is not measured here.
- **The sampler** took 9962–9980 samples per 15 s load window.
- **The analysis:**
  - "(reprofile-2)"'s scripts, over the 15 s from each run's load start.
    The inclusive-share script's frame names were updated to this build's
    symbols, such as `quote.288` and `Response.to_bytes.354`.
  - The collector's instance was identified again, by counting, for each
    `LocalKey::with` instance, the samples whose leaf is a collection
    phase. `ce5e268d0d359845` had 15–20 a run, out of 681–727 samples. One
    other, `b136050d1cde226a`, had 3–8, but it appears in only 3–8 samples
    a run.
  - The heap callers came from "(reprofile)"'s caller script, unchanged.
  - Per-request figures are a share times that run's `1e6 / rps`.
- **Ordering.** The predictions' modification time is 17:53:33, the
  profiling binary's 17:54:05, and the end of the first run's samples
  17:54:28.

These predictions were written before the profiling build existed:

| prediction | measured | verdict |
|---|---|---|
| socket system calls, self, 57–64% | 65.4–66.5% | wrong: above |
| send path 45–53% | 55.6–56.0% | wrong: above |
| `read_request`, receives included, 17–22% | 13.9–15.5% | wrong: below |
| `user_json` 9–14% | 11.3–13.5% | within |
| `quote`, `json_quote` included, 2–5% | 4.1–4.4% | within |
| `gc::alloc`, collection included, 17–23% | 18.2–18.6% | within |
| system heap 4–7% | 4.7–4.9% | within |
| throughput with the sampler 12,000–25,000 | 20662.7–20800.5 | within |
| `quote` stops being a lever | 2.0–2.1 us, a third of it the system heap | wrong, by judgment: its buffer is still a candidate |
| the send path stays the largest item | 55.6–56.0% | right |
| on the Nova side, allocation (`gc::alloc` and the second `LocalKey` instance) is the largest remaining item, ahead of any single `std/json` or `std/http` function | `gc::alloc` 18.2–18.6%, against `stringify` 4.3–4.6%, `read_request` 13.9–15.5% and `Response.to_bytes` 4.9–5.6%; the union with the second instance was not evaluated | partly: ahead of those functions, but `handle`, which calls them, is larger inclusive |

### The runs, inclusive % of the server thread

| | run 1 | run 2 | run 3 |
|---|---|---|---|
| req/sec with the sampler | 20800.5 | 20799.5 | 20662.7 |
| us per request | 48.1 | 48.1 | 48.4 |
| socket system calls, self | 66.51 | 65.45 | 65.71 |
| send path (`ws2_32!send`) | 56.03 | 55.63 | 55.70 |
| receive path (`ws2_32!recv`) | 10.12 | 9.70 | 9.65 |
| `gc::alloc`, collection included | 18.16 | 18.16 | 18.55 |
| — the collector's instance | 7.27 | 6.84 | 7.29 |
| `handle` | 18.51 | 20.67 | 21.20 |
| `users_json` | 17.29 | 14.31 | 14.18 |
| `user_json` | 13.54 | 11.53 | 11.28 |
| — `quote` | 4.09 | 4.39 | 4.19 |
| — — `nova_rt_json_quote` | 4.08 | 4.35 | 4.16 |
| `nova_rt_str_concat_n` | 4.55 | 3.23 | 3.69 |
| `String.join` | 1.93 | 0.81 | 0.77 |
| `Response.to_bytes` | 4.86 | 5.57 | 5.43 |
| — `Bytes.concat` | 0.62 | 2.39 | 2.53 |
| `json_response` | 1.16 | 6.23 | 6.91 |
| `read_request` | 15.46 | 14.00 | 13.94 |
| — `parse_request_head` | 3.03 | 2.03 | 1.91 |
| `poll::wait` | 1.76 | 1.76 | 1.96 |
| system heap | 4.70 | 4.78 | 4.94 |

### What this does not settle

- **What writing straight into the GC string would save.** The 1.7 us
  above is the heap's share under the three largest callers, from a
  sampled build. Removing the buffer would also remove a copy, and might
  move other costs. It is a candidate, not a measured gain.
- **Whether the unsampled build splits the same way.** These are a
  sampled build's shares.
- **One host, Windows, three runs.**

## AMENDMENT 2026-10-03 (gc-direct-strings): builtins write their string results straight into GC memory

"(reprofile-3)" found that `nova_rt_json_quote`, `nova_rt_str_concat_n`
and `nova_rt_int_to_str` spent 3.5–3.6% of the server thread in the system
heap, building a Rust `String` that `gc_str` then copied into a GC buffer.
- **The change:** a new runtime helper, `gc_str_filled`, allocates the GC
  byte buffer at its exact final length and lets the caller write into it.
  Five builtins now use it: `json_quote`, `str_concat_n`, `str_join`,
  `str_concat` and `int_to_str`.
- **What each call saves:** one system-heap allocation, one free and one
  copy.
  - `json_quote` now counts its escaped length before it writes.
  - `int_to_str` formats its digits on the stack.
- **What changed for the collector:** the sources are now read after the
  result's first allocation, not before. They stay alive because a pointer
  held in the caller's frame or a callee-saved register is a root, the
  argument `nova_rt_str_chars` already relies on.
- **Output is unchanged.**

**Results:**
- **Per call, all four are disjoint.**
  - `stringify` of the clean name went from 103.5–111.6 to 77.2–80.1 ns, a
    22.6–30.1% cut per pair.
  - `stringify` of the clean email went from 104.5–118.8 to 81.3–87.2 ns,
    21.1–29.9%.
  - `stringify` of the escaped string went from 161.6–168.0 to 87.3–90.6
    ns, 43.9–48.0%.
  - `users_json` at ten users went from 5307.7–5647.7 to 4054.6–4372.1 ns,
    17.6–28.2%.
- **The ten-user server is disjoint, in both orders, so a gain is
  claimed.**
  - Over six alternated pairs, before ran at 14421.1–14720.4 req/sec and
    after at 15137.2–15481.7.
  - After was 4.2–6.7% faster per pair, 2.8–4.3 us less per request.
  - Three pairs ran `before` first and three `after` first. Each order on
    its own gave disjoint ranges: 4.2–5.0% faster per pair before-first,
    and 5.0–6.7% after-first.

### How it was measured

- **Per call:** "(json-quote)"'s scratch Nova harness, never committed,
  which prints picoseconds per call.
  - It times 1,000,000 calls each of the three `stringify`s, and 200,000
    calls of `users_json` over ten users.
  - Its copies of the example's `User`, `Store`, `user_json` and
    `users_json` are as of `b0b606a`. The example's source has not changed
    since.
  - `before` is 525,312 bytes, SHA-256 `bee430f576092a43…`; `after` is
    524,288 bytes, SHA-256 `b96a1ddf2c896d33…`.
  - They were alternated, five runs each, one fresh process per run, with
    `before` first in runs 1, 3 and 5. Every run printed the same output
    lengths and the same escaped sample.
- **Server:** ten users, a 604-byte body, 200 connections,
  `--warmup 5 --duration 15`.
  - `before` is 702,464 bytes, SHA-256 `828bc6742c351de0…`; `after` is
    701,440 bytes, SHA-256 `faa4e26a9a6acf2d…`.
  - One fresh process per reading. Every reading was `errors=0`, and every
    body the same, SHA-256 `3ff5004bf26139cc…`.
  - `bun docs/benchmarks/bun-equivalence.js` passed against `after` first:
    all 9 exchanges match.
- **Provenance.** The scratch binaries log records the release `nova`'s
  SHA-256 beside the commit it was built from, `02fd6ee` for `before` and
  `d32c5aa` for `after`. That all four binaries were built by it is from
  this session. The equivalence log does not name its binary.
- **Ordering.** The predictions' modification time is 18:58:25, and the
  `before` harness binary's is 18:58:47. The code commit is dated
  19:06:19.

These predictions were written before any of the change existed:

| prediction | measured | verdict |
|---|---|---|
| clean name: before 90–140 ns, after 20–40% lower, disjoint | 103.5–111.6; 22.6–30.1% lower, disjoint | within |
| clean email: before 90–140 ns, after 20–40% lower, disjoint | 104.5–118.8; 21.1–29.9% lower, disjoint | within |
| escaped string: before 140–200 ns, after 15–30% lower, disjoint | 161.6–168.0; 43.9–48.0% lower, disjoint | before within; the cut wrong: above |
| `users_json`: before 4,500–7,000 ns, after 20–35% lower, disjoint | 5307.7–5647.7; 17.6–28.2% lower, disjoint | before within; the cut: four of five pairs within, one below at 17.6% |
| server after 1–5% above before per pair; the ranges may overlap | 4.2–6.7% per pair, disjoint in both orders | four of six pairs within, two above at 5.2% and 6.7% |
| three named mutants each fail at least one named test, with exit codes checked | all three did | right |

### Per call, ps, in run order

| run | first | name before | after | email before | after | escaped before | after | `users_json` before | after |
|---|---|---|---|---|---|---|---|---|---|
| 1 | before | 109175 | 79426 | 118844 | 83283 | 167964 | 87309 | 5641663 | 4128341 |
| 2 | after | 111610 | 78026 | 110399 | 87150 | 161561 | 90574 | 5647702 | 4054602 |
| 3 | before | 106584 | 77196 | 104458 | 81261 | 163149 | 88458 | 5513361 | 4320442 |
| 4 | after | 103521 | 80077 | 109724 | 82565 | 167052 | 89392 | 5558060 | 4131211 |
| 5 | before | 105585 | 79259 | 106931 | 81933 | 162020 | 90143 | 5307721 | 4372064 |

### Server, in run order

| pair | order | before | after | after / before |
|---|---|---|---|---|
| 1 | before first | 14708.0 | 15436.4 | 1.050 |
| 2 | before first | 14660.4 | 15280.5 | 1.042 |
| 3 | before first | 14650.5 | 15348.4 | 1.048 |
| 4 | after first | 14451.3 | 15417.7 | 1.067 |
| 5 | after first | 14421.1 | 15137.2 | 1.050 |
| 6 | after first | 14720.4 | 15481.7 | 1.052 |

### Correctness

- **New tests:**
  - `int_to_str_matches_rust_at_every_digit_count_and_both_ends`: every
    digit count from one to nineteen, both signs, and `i64::MIN` and
    `i64::MAX`, against Rust's own formatting;
  - `concat_keeps_both_sides_whole`: either side empty, and multi-byte
    text on both;
  - `json_round_trip_under_gc_stress`,
    `json_stringify_escapes_under_gc_stress` and
    `json_parse_strings_under_gc_stress`: three `std/json` fixtures run
    with `NOVA_GC_STRESS=1`, a collection on every allocation, so every
    read after the result's allocation follows a collection. Between them
    they reach all seven short escapes and the `\u00XX` form. The third was
    added after the measurement, with comment fixes; that later commit
    changes no compiled code.
- **Existing tests pass unchanged:**
  - the runtime's `json_quote`, `concat_n`, `join` and `int_to_str` tests;
  - the JSON, strings and interpolation fixtures, including
    `strings_under_gc_stress` and `interpolation_nary_under_gc_stress`.
- **Three named mutants each fail.** Each was run under two filtered
  commands only:
  - `cargo test --locked -p nova-runtime --lib -- int_to_str concat
    json_quote join_puts`;
  - `cargo test --locked -p nova-cli --test run_tests -- json strings
    interpolation`.

  Exit codes and result lines were checked. The first mutant's text anchor
  missed twice, after `cargo fmt` had reindented its line, and it ran on
  the third attempt.
  - **`json_quote` counting `\u00XX` as five bytes:** the runtime test
    binary aborts when `Fill::put`'s bounds check panics (exit
    `0xc0000409`), and 5 fixture tests fail.
  - **`str_concat_n` dropping its last part:** the runtime test binary
    aborts on the length check (exit `0xc0000409`), and 14 fixture tests
    fail.
  - **`int_to_str` losing the sign:** 2 runtime tests and 3 fixture tests
    fail.
- **The gates pass** on the final tree. A scratch log records the base
  commit, the working tree's files and each step's exit code. The commands
  and their flags are from this session:
  - `cargo test --locked --workspace`: 1183 passed, 0 failed, 8 ignored;
  - `cargo clippy --locked --workspace --all-targets --all-features -- -D
    warnings`: exit 0;
  - `cargo fmt --all --check`: exit 0.

### What this does not settle

- **The gate.** Neither criterion was rerun. "(gate-remeasure-5)"'s
  shortest pinned round needed about 2.9% more to clear ADR 0021's 1.0547
  margin. This change is 4.2–6.7% faster per pair on the 15 s method, a
  different method in a different run. Only a run under ADR 0021 can say.
- **Why the escaped string gained most.** From the code, not measured:
  the old `json_quote` reserved the input's length plus two, so any escape
  outgrew the buffer and made it grow on the system heap. The new one
  counts first and never grows.
- **One host, Windows.**

## AMENDMENT 2026-10-03 (gate-remeasure-6): the Phase 2 gate on `main` at `b24379e`, after "(gc-direct-strings)"

The second gate run judged by `docs/adr/0021-gate-ratio-paired-rounds.md`,
with "(gate-remeasure-5)"'s method and scripts.
- **Code:** since `1972b37`, "(gc-direct-strings)" changed five string
  builtins. Everything else that landed was docs and tests.
- **Rounds:** twelve, fixed in advance, each a pinned pair and then an
  unpinned pair. Odd rounds ran Nova first and even rounds Bun first.
- **Pinned decides,** with a margin for Bun's `Date` header.

**§5's ratio is inconclusive.**
- **The margin:** measured before the first reading, Nova's `/users`
  response was again 676 bytes and Bun's 713, so the margin is 713/676,
  about 1.0547.
- **Nova cleared it in 7 of the 12 pinned rounds.** The ADR needs 10.
- **Nova was faster in plain req/sec in 8 of the 12.**
- **The pinned round ratios were 0.907–1.517, median 1.112.** That is far
  wider than "(gate-remeasure-5)"'s 1.025–1.161.
- **The five short rounds were all in rounds 5–10:** 1.043, 0.971, 0.988,
  0.907 and 0.925. Rounds 1–4 cleared in 4 of 4, rounds 5–8 in 0 of 4, and
  rounds 9–12 in 3 of 4.
- **Unpinned, which does not decide, Nova cleared the margin in all 12
  rounds,** at 1.067–1.468, median 1.150.
  - ADR 0021 names pinned as the deciding condition, so this does not
    change the verdict.
  - It says only that the two conditions disagreed in this run.

**§3's absolute criterion is met in every reading.** All 24 Nova readings
clear `nova-spec/00-MASTER-SPEC.md` §3's 10,000 req/sec: pinned at
14455.1–22200.2, unpinned at 14589.1–22089.4.

So the gate is still not met. The ratio is inconclusive, and the ADR says
inconclusive leaves §5 not met. It also says a run is not repeated on
unchanged code to replace its verdict.

**The host's speed fell during the run, though not steadily.**
- The median reading fell from 20013.7 in rounds 1–6 to 16794.2 in rounds
  7–12.
- 14 of the 24 readings in rounds 7–12 are below 17477.0, the slowest
  reading in rounds 1–6.
- Some later readings stayed high: round 7's unpinned Nova at 21733.4,
  round 9's pinned Nova at 21255.3, and round 12's unpinned Nova at
  20595.8.
- **Within single pinned pairs the two readings moved apart.** In round 9
  Nova pinned ran at 21255.3 and Bun pinned at 14015.9, a ratio of 1.517.
  In round 8, Bun pinned ran first at 20063.2 and Nova pinned then ran at
  18206.2, a ratio of 0.907.
- **The pinned round ratios scattered more than the unpinned ones:**
  0.907–1.517 against 1.067–1.468, a standard deviation of the log ratio
  of 0.146 against 0.085.
- **Whole-run ranges, as description:**
  - Nova pinned spread 1.54x and Bun pinned 1.59x.
  - The pinned extremes ratio is 0.714–1.742.
- Why the host slowed is not measured, and neither is why the pinned pairs
  scattered more.

### The rounds, in run order

| round | order | Nova pinned | Bun pinned | pinned ratio | clears 1.0547 | Nova unpinned | Bun unpinned | unpinned ratio |
|---|---|---|---|---|---|---|---|---|
| 1 | Nova first | 20063.7 | 17477.0 | 1.148 | yes | 21566.1 | 17871.4 | 1.207 |
| 2 | Bun first | 20530.2 | 17581.9 | 1.168 | yes | 21847.7 | 18923.7 | 1.155 |
| 3 | Nova first | 21652.2 | 19218.1 | 1.127 | yes | 21301.6 | 19963.6 | 1.067 |
| 4 | Bun first | 22200.2 | 20232.7 | 1.097 | yes | 20891.8 | 18369.9 | 1.137 |
| 5 | Nova first | 20482.2 | 19646.6 | 1.043 | no | 21802.5 | 18976.6 | 1.149 |
| 6 | Bun first | 18978.3 | 19549.6 | 0.971 | no | 22089.4 | 19185.0 | 1.151 |
| 7 | Nova first | 16634.6 | 16831.6 | 0.988 | no | 21733.4 | 19815.1 | 1.097 |
| 8 | Bun first | 18206.2 | 20063.2 | 0.907 | no | 20709.9 | 16187.6 | 1.279 |
| 9 | Nova first | 21255.3 | 14015.9 | 1.517 | yes | 19373.0 | 17372.8 | 1.115 |
| 10 | Bun first | 14665.1 | 15851.5 | 0.925 | no | 18771.1 | 16756.7 | 1.120 |
| 11 | Nova first | 19055.6 | 14620.9 | 1.303 | yes | 14589.1 | 12622.8 | 1.156 |
| 12 | Bun first | 14455.1 | 12745.0 | 1.134 | yes | 20595.8 | 14027.2 | 1.468 |

- **Pinned:** clears in 7 of 12, 4 of the 6 Nova-first rounds and 3 of the
  6 Bun-first. Nova was faster in 8 of 12.
- **Unpinned:** clears in 12 of 12, 6 of 6 in each order.
- **Every reading** was `errors=0`, every pinned reading's mask read back
  as 1, and every unpinned one as 4095.
- **The order** in the log matches the ADR's, reading for reading.
- **Each server was a fresh process,** pinned or left alone after it
  printed its listening line and before seeding.

**The load generator was not the limit.**
- **Ceiling:** the self-test, taken after the byte measurement and before
  the first reading, reached 108018.0 req/sec. The fastest reading, Nova's
  22200.2, is 20.6% of that.
- **Calibration:** the self-test recorded `errors=99`, and at least one
  connection completed no requests (`conn_min=0`, `conn_max=74266`). That
  is weaker calibration than "(gate-remeasure-5)"'s; at 20.6% of the
  ceiling, the conclusion does not depend on it.
- **Elapsed:** its `elapsed_ms` is 30032, in line with the cells'
  30017–30035, unlike the 34693 and 36424 the two runs before recorded.
- **Binary:** `target/release/nova-bench-http.exe`, SHA-256
  `d17062335e988c18…`, the same file "(gate-remeasure)" through
  "(gate-remeasure-5)" used.

**These predictions were written before any build, byte measurement or
reading.**
- **The predictions file's modification time is 20:05:49**, matching the
  `date` line it ends with, so it was not edited afterwards.
- **The other files' modification times:**
  - 20:06:06 for the release runtime library and 20:06:19 for the release
    `nova`, both rebuilt from `b24379e`;
  - 20:06:24 for the gate binary;
  - 20:06:32 for the equivalence log;
  - 20:06:38 for the byte measurement's log;
  - 20:07:19 for the self-test log.
- **The run's log** has it starting at 20:06:31 and ending at 20:38:33.

| prediction | measured | verdict |
|---|---|---|
| response bytes: Nova 676, Bun 713; m = 1.0547 | 676 and 713; 1.0547 | right |
| pinned round ratios roughly 1.06–1.25, median 1.10–1.16 | 0.907–1.517, median 1.112 | median within; the range wrong on both sides |
| pinned rounds clearing m: 9–12 of 12 | 7 | wrong: below |
| verdict: met (10 or more clears) | 7 clears, inconclusive | wrong |
| unpinned rounds clearing m: 8–12 of 12 | 12 | within |
| all 24 Nova readings above 10k | all 24 | right |
| Nova pinned 10,000–28,000 | 14455.1–22200.2 | within |
| Bun pinned 9,000–26,000 | 12745.0–20232.7 | within |
| Nova unpinned 10,000–28,000 | 14589.1–22089.4 | within |
| Bun unpinned 9,000–26,000 | 12622.8–19963.6 | within |

The deciding prediction, met, was wrong. The pinned rounds scattered more
than "(gate-remeasure-5)" led the predictions to expect.

### Identity of each side, and the payload

- **Nova:** `examples/05-json-api`, built by the release `nova` from
  `b24379e`, SHA-256 `cd88564854c343e5…`; that it was built from that
  commit is from this session.
  - `json-api.exe` is 701,440 bytes, SHA-256 `d02fcc62b20c44ea…`. The same
    binary ran in every Nova reading.
- **Bun:** `docs/benchmarks/bun-server.js`, SHA-256 `f95426e14e22034c…`,
  the same file as before, run by bun 1.3.0. The version is from earlier
  this session.
- **The payload.** Ten users were seeded by the same `curl` POSTs on each
  side. Every reading on both sides served the same 604-byte body, SHA-256
  `3ff5004bf26139cc…`.
- **The byte measurement** was "(gate-remeasure-5)"'s: a fresh server per
  side, seeded the same way, and one `GET /users` saved with `curl -s -D`.
  Nova's head was 72 bytes and Bun's 109; both bodies were 604.
- **Equivalence was run first**, against this run's `json-api.exe`:
  `EQUIVALENCE OK: all 9 exchanges match on status and body bytes`. The
  scratch script that ran it passes the `json-api.exe` in this run's
  directory.

### What this does not settle

- **Why the pinned condition scattered.**
  - Pinning puts each server on core 0, which the load generator's
    threads may also use. "(gate-remeasure)" named that as a candidate,
    and ADR 0021 lists it as an unmeasured confound.
  - In this run the pinned round ratios scattered more than the unpinned
    ones. But in "(gate-remeasure-5)", pinned the same way, the pinned
    ratios, 1.025–1.161, were tighter than the unpinned, 0.838–1.240. So
    pinning to core 0 does not by itself account for it, and neither run
    tests the candidate.
- **Whether "(gc-direct-strings)" moved the pinned ratio.**
  - The median pinned round ratio was 1.112 here and 1.079 in
    "(gate-remeasure-5)". But Nova was faster in 8 of 12 pinned rounds here
    against 12 of 12 there, so the two summaries move in opposite
    directions. The two runs' pinned rounds overlap, and the host's speed
    differed.
  - The ADR judges each run alone.
- **Which statement of the gate governs.** As of `b24379e`, no tracked
  file settles it.
- **Any host but this one.** Every figure here is from this development
  host, Windows, with the load generator on the same machine.

## AMENDMENT 2026-10-03 (aa-noise): how far the pinned setup moves a ratio on its own

"(gate-remeasure-6)" left the deciding pinned condition's scatter
unexplained. Its Nova/Bun pinned round ratios ran 0.907–1.517. ADR 0021
lists pinning to core 0, which the load generator may share, as an
unmeasured confound. This diagnostic measures two things:
- how much two pinned readings of the *same* server, back to back, differ;
- whether keeping the generator off core 0 changes that.

It is not a gate run and judges nothing.

**Results:**
- **In this run, two pinned readings of the same server scattered about as
  much as Nova against Bun did in "(gate-remeasure-6)".**
  - With the generator as the gate runs it, the 12 A/A pairs' ratios have
    a standard deviation of the log ratio of 0.139. "(gate-remeasure-6)"'s
    Nova/Bun pinned pairs had 0.146.
  - One pair carries most of it: without Bun's round 4, at 1.527, the
    figure is 0.074.
- **9 of the 24 A/A pairs moved by more than ADR 0021's margin,** with the
  second reading above 1.0547 times the first or below its reciprocal.
  That is 5 of 12 with the default generator and 4 of 12 with core 0
  excluded.
- **The readings fall into separate speed levels, and every large move is
  between them.**
  - Bun's readings form two clusters, 12453.3–13546.3 and 17829.0–22803.5,
    with nothing between.
  - Nova's form a slow cluster, 14121.5–14895.2, and a fast one,
    20557.2–23407.8, with four readings between at 15258.7–16700.1.
  - Each of the five pairs whose log ratio is above 0.1 in size has its two
    readings in different places: Bun's two cross between its clusters, and
    each of Nova's three has one reading in the band between.
  - Rounds 1, 4, 5 and 6 each hold readings at both speeds, so the level
    changed within rounds as well as between them.
- **Keeping the generator off core 0 did not reduce the standard deviation
  of the log ratio, and did not remove the large jumps.**
  - The figure is 0.164 with core 0 excluded, against 0.139 without.
  - The median absolute log ratio was lower with core 0 excluded, 0.013
    against 0.040 pooled, so the small pair-to-pair differences shrank.
- **Throughput was lower with the generator off core 0, and that is
  tangled with the speed levels.**
  - Median against median, the excluded variant ran at 0.908 of the
    default for Nova and 0.701 for Bun.
  - Bun's excluded readings sat at the slow level in 11 of 12, against 5
    of 12 for its default ones. In rounds 2 and 3, where every reading sat
    at the slow level, Bun's excluded/default ratio was 0.999 and 1.005.
  - Variant and level are not separated here.

**What this means for ADR 0021's criterion, as description:**
- **Pairs of one unchanged server moved by more than the margin in 9 of
  24.** So for 10 of 12 rounds to land on one side, a true ratio must sit
  well clear of the margin.
- **Nothing here changes any recorded verdict.**

### How it was measured

- **The design:** six rounds, each of four A/A pairs.
  - Every reading was a fresh server pinned to core 0, with its mask read
    back as 1. Ten users were seeded, then 200 connections ran for 30 s
    after a 5 s warmup.
  - In each round: Nova/Nova and Bun/Bun with the generator as the gate
    runs it, plus a read of its affinity mask; and Nova/Nova and Bun/Bun
    with the generator's affinity set to 4094, all cores but core 0.
  - Odd rounds ran the default variant first and even rounds the excluded
    one. An A/A ratio is the second reading over the first.
- **The generator's affinity** read back as 4095, all twelve cores, in
  every default reading, and as 4094 in every excluded one.
  - It was set by the Windows process ID taken right after the generator
    was launched. The record does not show that this ID was the
    generator's own process rather than a launcher's.
  - The setting was made during the generator's 5 s warmup. That it
    landed before the measured window is inferred from PowerShell's
    start-up time; the time it landed was not logged.
- **Identity.**
  - Nova is "(gate-remeasure-6)"'s `json-api.exe`, 701,440 bytes, SHA-256
    `d02fcc62b20c44ea…`, built from `b24379e`. `main` at `62c3149` has the
    same code.
  - Bun is `docs/benchmarks/bun-server.js`, SHA-256 `f95426e14e22034c…`.
  - The generator is `target/release/nova-bench-http.exe`, SHA-256
    `d17062335e988c18…`, unchanged since 2026-09-29.
  - Every reading was `errors=0` and served the 604-byte body, SHA-256
    `3ff5004bf26139cc…`. The order in the log matches the design, reading
    for reading.
- **A smoke reading,** one excluded-variant Nova cell, ran before the six
  rounds to check the affinity mechanism. It is not in the data; its log
  is kept apart.
- **Ordering.** The predictions' modification time is 20:59:41, the smoke
  reading's 21:00:43, and the run's log has it starting at 21:00:48 and
  ending at 21:31:52.

These predictions were written before any A/A reading:

| prediction | measured | verdict |
|---|---|---|
| the generator's default affinity reads back as 4095 | 4095 in every default reading | right |
| default generator: A/A standard deviation of the log ratio 0.04–0.15 | 0.139 pooled; Nova 0.100, Bun 0.167 | within, pooled; Bun alone above |
| excluding core 0 cuts that by at least 30% | 0.164 against 0.139, higher | wrong |
| excluding core 0 raises pinned throughput 0–15%, median against median | Nova 0.908 and Bun 0.701 of default | wrong: lower |
| if the cut fails, core 0 is not the main source, and host-level drift is the remaining candidate | the cut failed | the condition held; the conclusion was not tested, and the median absolute log ratio did fall with core 0 excluded |

### The A/A ratios, second reading over first

| round | first variant | Nova default | Bun default | Nova core 0 excluded | Bun core 0 excluded |
|---|---|---|---|---|---|
| 1 | default | 0.968 | 1.049 | 0.751 | 1.015 |
| 2 | excluded | 0.856 | 0.993 | 0.991 | 0.997 |
| 3 | default | 0.998 | 1.001 | 1.004 | 0.993 |
| 4 | excluded | 1.059 | 1.527 | 0.998 | 1.012 |
| 5 | default | 0.990 | 1.056 | 1.023 | 0.603 |
| 6 | excluded | 1.159 | 0.999 | 1.055 | 0.942 |

### What this does not settle

- **What switches the host between speed levels.**
  - The power plan and background load were not recorded.
  - Host-level drift was named in the predictions but not tested.
- **Whether the generator's placement moves which level a reading lands
  on.** Bun's excluded readings sat at the slow level more often, but
  variant and level are not separated.
- **Whether shorter or more tightly interleaved readings would scatter
  less.**
- **Whether unpinned pairs scatter less.** No unpinned A/A pairs were run.
- **One host, Windows, six rounds.**

## AMENDMENT 2026-10-03 (aa-interleaved): an interleaved-window method fails its validation

"(aa-noise)" found that two pinned readings of the same server, run back to
back as ADR 0021's rounds run them, moved by more than the 1.0547 margin in
5 of 12 pairs, or 9 of 24 counting its variant with the generator kept off
core 0. A method change was proposed to the user and approved, to be
adopted only if it passed an A/A validation with pass marks fixed in
advance:
- **Both servers alive at once.** Each round starts two fresh servers,
  pins both to core 0, and seeds both.
- **Short interleaved windows.** The load then goes to one server at a
  time, in four windows of `--warmup 2 --duration 8`, in ABBA order.
- **The round's ratio** is one server's mean over its two windows divided
  by the other's.
- **The user also ruled** that, if the method passed, one gate run under it
  could proceed on the current code.

**The method failed both pass marks, so it is not adopted.** ADR 0021
stands unchanged, and no ADR 0022 was written.
- **Standard deviation of the log A/A ratio over the 24 rounds:** 0.094,
  against a pass mark of at most 0.07. "(aa-noise)"'s figure with the
  generator as the gate runs it was 0.139.
- **A/A ratios at or beyond the margin either way:** 12 of 24, against a
  pass mark of at most 2.
  - "(aa-noise)" had 5 of 12 with the generator as the gate runs it.
  - That is 9 of 24 with its core-0-excluded variant included, the figure
    the pass mark quoted.
- **The median absolute log ratio was 0.054.** Under the current method,
  with the generator as the gate runs it, "(aa-noise)"'s figure was 0.040:
  Nova 0.045, Bun 0.028.

**One server's throughput changed within about 10 s, and interleaving at
this granularity did not cancel it in this run.**
- **Within a round, two windows of the same server moved by more than the
  margin in 25 of 48 pairs.**
  - The inner instance's two windows are adjacent and start about 10 s
    apart. They moved beyond the margin in 10 of 24.
  - The outer instance's two windows start about 30 s apart. They moved
    beyond it in 15 of 24.
  - Those spacings are inferred from the window settings; start times were
    not logged.
  - The standard deviation of the log ratio is 0.152 over all 48: 0.157
    for the adjacent pairs and 0.150 for the outer ones. The median absolute
    log ratio is 0.030 adjacent and 0.079 outer.
- **19 of the 24 rounds** had a fastest and slowest window more than 1.0547
  apart.
- **Nova round 5's four windows,** on two servers running the same binary,
  ran at 14089, 16042, 25310 and 20105 req/sec.
- **The windows ranged over 11289–26366 req/sec in all.**

**What this means, as description:**
- Under neither ADR 0021's pairs nor this interleaving did single-round A/A
  ratios stay within the margin. 5 of 12 default-generator pairs and 12 of
  24 interleaved rounds moved beyond it.
- Nothing here changes any recorded verdict.

### How it was measured

- **The design:** 24 pinned A/A rounds, alternating Nova/Nova and Bun/Bun,
  12 each.
  - **Each round:**
    - started two fresh instances of the same server and pinned both to
      core 0, with both masks read back as 1;
    - seeded ten users into each;
    - ran four load windows of 200 connections, `--warmup 2 --duration 8`,
      one instance at a time.
  - **Order:** instance 1, 2, 2, 1 in that side's odd rounds, and 2, 1, 1, 2
    in its even rounds.
  - **The A/A ratio** is instance 2's mean over its two windows divided by
    instance 1's.
  - **The generator** was restarted for every window, four times a round,
    with no affinity setting. Its affinity was not read back in this run.
- **Identity.**
  - Nova is "(gate-remeasure-6)"'s `json-api.exe`, SHA-256
    `d02fcc62b20c44ea…`. `main` at `a68aa09` has the same code.
  - Bun is `docs/benchmarks/bun-server.js`.
  - The generator is `target/release/nova-bench-http.exe`, SHA-256
    `d17062335e988c18…`.
- **Every window** was `errors=0` and timed 8014–8036 ms. Every instance
  served the 604-byte body, SHA-256 `3ff5004bf26139cc…`, and every round
  ran four windows.
- **A smoke round,** Nova against Bun, ran before the 24 rounds to check the
  mechanics. It is not in the data, and its log is kept apart.
- **Ordering.**
  - The predictions and pass marks were written at 21:55:01.
  - The round script was last written at 21:55:21, and the smoke round's
    first server started at 21:55:25. Its log was written at 21:56:12.
  - The run started at 21:56:18 and ended at 22:15:05.

These predictions and pass marks were written before any reading under the
method:

| prediction | measured | verdict |
|---|---|---|
| pass mark (a): standard deviation of the log A/A ratio at most 0.07 | 0.094 | failed |
| pass mark (b): at most 2 of 24 A/A ratios at or beyond the margin | 12 of 24 | failed |
| standard deviation of the log ratio 0.03–0.09 | 0.094 | wrong: above |
| rounds beyond the margin 1–4 of 24 | 12 | wrong: above |
| the pass is uncertain | it failed on both marks | the uncertainty was too optimistic |
| every reading `errors=0` | every window | right |

### The A/A ratios, instance 2 over instance 1

| round | Nova/Nova | Bun/Bun |
|---|---|---|
| 1 | 1.014 | 0.984 |
| 2 | 1.016 | 1.144 |
| 3 | 1.022 | 0.780 |
| 4 | 0.982 | 1.084 |
| 5 | 1.209 | 1.057 |
| 6 | 0.947 | 1.006 |
| 7 | 0.922 | 1.022 |
| 8 | 0.985 | 0.990 |
| 9 | 1.056 | 0.974 |
| 10 | 0.923 | 1.071 |
| 11 | 0.811 | 0.936 |
| 12 | 1.044 | 0.949 |

Rounds alternated: Nova round 1, Bun round 1, Nova round 2, and so on.

### What this does not settle

- **What moves a server's throughput within about 10 s.** The power plan,
  frequency scaling and background load were not recorded.
- **Whether the idle second instance added noise of its own.** Both
  instances were pinned to core 0, so the loaded server shared its core
  with a live, idle one. ADR 0021 runs one server at a time. The two were
  not separated here.
- **Whether a quieter host would bring either structure's A/A scatter under
  these pass marks.**
- **Whether much shorter windows, or many more of them, would average the
  noise down.** Nothing here measured that.
- **One host, Windows, 24 rounds.**

## AMENDMENT 2026-10-04 (reprofile-4): the json-api's server thread after the GC-direct string builtins

"(reprofile-3)" sampled the json-api on `ef43fdf`. Since then,
"(gc-direct-strings)" has made five string builtins write straight into
GC memory. This run samples a sampled build of it three times on
`a465dfc`, with the same sampler and scripts, to pick the next lever. The
code it measures is the same as "(gate-remeasure-6)"'s.

**Results:**
- **The system heap fell to 1.50–1.55% of the server thread,** from
  4.7–4.9%.
  - The three largest of its nearest runtime callers are `net::try_read` at
    0.57–0.81%, `nova_rt_bytes_concat` at 0.34–0.48%, and
    `str::to_lowercase` at 0.20–0.27%. The other 0.18–0.21% is spread over
    five to seven callers, none above 0.07%.
  - `nova_rt_json_quote`, `nova_rt_str_concat_n` and `nova_rt_int_to_str`
    no longer appear among its callers.
- **Allocation, collection included, is the largest Nova-side item among
  the frames compared here, in every run, once each frame's own
  `gc::alloc` samples are set aside.**
  - `gc::alloc` is 22.6–24.7% of the thread, 10.4–10.9 us per request.
  - `read_request` has two frames. Over both, it is 28.3–29.8% inclusive:
    13.5–14.8 points are socket receive calls, and nearly all of the
    collector's samples sit under its plain frame (below). Without its
    `gc::alloc` samples it is 17.9–19.8%, below `gc::alloc` in every run.
    Its `$poll` frame alone, the table's row, is 20.8–22.9%.
  - The other frames compared are `stringify` at 4.4–6.3%,
    `Response.to_bytes` at 3.9–4.7%, `users_json` at 14.7–16.5% and
    `user_json` at 11.5–13.5%. `handle`, which calls them, is 16.3–18.0%.
  - Two frames are not compared. `serve` calls into everything, at
    97.3–97.5%. `write_stream`, at 45.2–46.5%, is the send path's
    44.6–46.0 points plus about 0.6.
- **The collector and allocation proper:**
  - The collector's instance, `LocalKey::with` `ce5e268d0d359845` as
    before, is 6.7–7.6% of the thread, or 3.0–3.3 us. All of it sits under
    `gc::alloc`.
  - The rest, allocation proper, is 15.9–17.4%, 7.4–7.5 us. Charged by
    leaf, its parts are:
    - finding a slot, 5.8–7.1%;
    - `gc::alloc`'s own code, 4.2–5.0%;
    - zeroing (`memset`), 2.3–2.7%;
    - the second `LocalKey::with` instance, `d03a43526a1fdc50`, 2.0–2.5%;
    - `FnOnce::call_once`, 1.1–1.3%.
- **Collections now land in the same place every run.** Nearly all of the
  collector's samples, 6.7–7.6 points of the thread, sit under the plain
  `read_request.357` frame. That is outside the `read_request.357$poll`
  frame the table's `read_request` row measures. Apart from `gc::alloc`,
  which they all sit under, no other frame compared here carries more than
  0.01 points of them.
- **Inside the string builtins, allocation through `gc::alloc` is now just
  under half of `nova_rt_json_quote` and most of the other two.**
  - **`nova_rt_json_quote`:** 46.5–47.9% of its samples sit under
    `gc::alloc`, and no system-heap leaf remains.
    - 50.5–52.2% land in its own symbol. The map has no separate symbol for
      `gc_str_filled`, `Fill::put` or the fill closure, so they are inlined
      there. Reading the source, that symbol holds the counting pass, the
      escaping pass and the writes' bookkeeping.
    - 1.2–2.4% land in a `vcruntime140` leaf the resolver names
      `__NLG_Return2`, not under `gc::alloc`.
  - **`nova_rt_str_concat_n`:** 61.0–68.6% sit under `gc::alloc`.
  - **`nova_rt_int_to_str`:** 83.2–89.0% sit under `gc::alloc`.
- **Items with no collection samples, per request.** Except where marked,
  none of these carried more than 0.01 points of the collector's samples.
  - socket system calls, self: 58.7–61.0% of the thread, 25.1–28.2 us. The
    send path alone is 44.6–46.0%, 19.3–21.3 us, and the receive path
    13.5–14.8%, 5.7–6.9 us;
  - `user_json`: 11.5–13.5%, 5.3–5.9 us;
  - `nova_rt_json_quote`: 4.1–6.0%, 1.9–2.5 us;
  - `Response.to_bytes`, without its one collector sample in runs 2 and 3:
    3.9–4.7%, 1.8–2.0 us;
  - `parse_request_head`: 3.6–3.9%, 1.6–1.8 us.
- **The send and receive paths' shares moved against "(reprofile-3)"'s
  run, and the host's state was not recorded.**
  - The send path was 44.6–46.0% against 55.6–56.0%, and the receive path
    13.5–14.8% against 9.6–10.1%.
  - The code that changed does not touch the send path, yet its
    per-request cost fell from 26.7–27.0 us to 19.3–21.3 us. So
    per-request microseconds are not compared across the two runs.
  - Shares across the runs carry the same confound: the send path's fall
    alone raises every non-send share 1.22–1.26 times. `gc::alloc` rose
    1.22–1.36 times, close to that. Allocation proper rose more, from
    10.9–11.3% to 15.9–17.4%, 1.41–1.60 times. Why is not measured.

**What this suggests, not measured:** with the system heap mostly gone,
allocation is the lever left that every string-producing builtin and every
Nova interpolation shares.
- "(alloc-fast-path)", a single borrow per allocation with inline
  zeroing, established no reduction.
- Two structural candidates it named remain untried: handing out free
  slots in runs rather than searching for each, and skipping the zeroing
  of leaf buffers that their caller fills completely.

### How it was measured

- **The binary.** The json-api, built by the release `nova` from `a465dfc`
  with "(sampled-profile)"'s scratch sampler patch applied. It is 738,304
  bytes, SHA-256 `75154c0ab9a7548b…`, linked with a symbol map.
  - The patch file is the one "(reprofile-3)" used; that is from this
    session. It was reverted afterwards.
  - A json-api built by the rebuilt release `nova` then came out at 701,440
    bytes, the plain build's size, with no map and without the string
    `NOVA_PROF_SAMPLE`.
- **The load:** ten users seeded, 200 connections for 15 s, no warmup, one
  fresh process per run, three runs. Every run reported `errors=0` and
  served the 604-byte body. With the sampler they ran at 21597.8–23521.7
  req/sec. The sampler's own cost is not measured.
- **The sampler** took 9693–9766 samples per 15 s load window.
- **The analysis:** "(reprofile-3)"'s scripts, unchanged, and one new
  read-only script for the decompositions above.
  - This build's symbol names match that run's, such as `quote.288`, so
    the inclusive-share script needed no edit.
  - The collector's instance was identified as before. `ce5e268d0d359845`
    had 15–24 collection-leaf samples a run, out of 651–736. One other,
    `b136050d1cde226a`, had 3–7, which were all of its samples, and is not
    counted as the collector.
  - Per-request figures are a share times that run's `1e6 / rps`.
- **Ordering.** The predictions' modification time is 07:17:01, the
  profiling binary's 07:17:31, and the end of the first run's samples
  07:18:09.

These predictions were written before the profiling build existed:

| prediction | measured | verdict |
|---|---|---|
| socket system calls, self, 66–71% | 58.7–61.0% | wrong: below |
| send path 55–60% | 44.6–46.0% | wrong: below |
| system heap 1.0–3.0% | 1.50–1.55% | within |
| `nova_rt_json_quote` 2.0–3.5% | 4.1–6.0% | wrong: above |
| `nova_rt_str_concat_n` 2.0–3.5% | 2.6–3.0% | within |
| `gc::alloc`, collection included, 16–22% | 22.6–24.7% | wrong: above |
| `user_json` without the collector's samples 8–11% | 11.5–13.5% | wrong: above |
| `read_request`, receives included, 13–17% | 20.8–22.9% | wrong: above |
| throughput with the sampler 15,000–26,000 | 21597.8–23521.7 | within |
| the system heap stops being a lever, under 3% | 1.50–1.55% | right |
| allocation, collection included, stays the largest Nova-side item that is not a caller of the rest | `gc::alloc` 22.6–24.7%; `read_request`, which calls into `gc::alloc`, 28.3–29.8% over both its frames inclusive and 17.9–19.8% without its `gc::alloc` samples | right on that basis; inclusive, `read_request`'s frames, which carry the collections, are larger in every run |
| the next candidates are allocation itself, `read_request`'s head parsing, and `Response.to_bytes` | `gc::alloc` 22.6–24.7%; `parse_request_head` 3.6–3.9%; `Response.to_bytes` 3.9–4.7%; `nova_rt_json_quote`, unnamed, 4.1–6.0% | partly: allocation leads, but `nova_rt_json_quote` is above both other candidates in every run |

The send path's share fell by about ten points against "(reprofile-3)",
and the receive path's rose by about four. Of this table's other share
rows, `gc::alloc` and `read_request` rose clear of their earlier ranges.
Throughput with the sampler rose clear of its earlier range too, from
20662.7–20800.5 to 21597.8–23521.7 req/sec.
`nova_rt_json_quote`, and `user_json` without the collector's samples,
rose in runs 2 and 3 only. Socket
system calls, the system heap and `nova_rt_str_concat_n` fell.

### The runs, inclusive % of the server thread

| | run 1 | run 2 | run 3 |
|---|---|---|---|
| req/sec with the sampler | 21597.8 | 22686.5 | 23521.7 |
| us per request | 46.3 | 44.1 | 42.5 |
| socket system calls, self | 60.98 | 58.68 | 59.01 |
| send path (`ws2_32!send`) | 45.97 | 44.64 | 45.32 |
| receive path (`ws2_32!recv`) | 14.81 | 13.73 | 13.50 |
| `gc::alloc`, collection included | 22.58 | 24.70 | 24.37 |
| — the collector's instance | 6.67 | 7.59 | 6.94 |
| `handle` | 16.28 | 17.97 | 17.82 |
| `users_json` | 14.65 | 16.46 | 16.32 |
| `user_json` | 11.49 | 13.28 | 13.46 |
| — `nova_rt_json_quote` | 4.14 | 5.43 | 5.97 |
| `nova_rt_str_concat_n` | 2.77 | 2.96 | 2.63 |
| `Response.to_bytes` | 3.90 | 4.65 | 4.73 |
| `json_response` | 1.34 | 1.37 | 1.35 |
| `read_request` | 22.93 | 20.83 | 21.24 |
| — `parse_request_head` | 3.94 | 3.61 | 3.95 |
| `poll::wait` | 1.86 | 1.98 | 1.80 |
| system heap | 1.55 | 1.52 | 1.50 |

### What this does not settle

- **Why the send and receive paths' shares moved.** The host's state was
  not recorded, and the code also changed between the two runs.
- **Why allocation proper's share rose more than the rest of the non-send
  thread did.**
- **Whether the unsampled build splits the same way.** These are a
  sampled build's shares.
- **What either structural allocation candidate would save.** Neither was
  tried.
- **One host, Windows, three runs.**

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
