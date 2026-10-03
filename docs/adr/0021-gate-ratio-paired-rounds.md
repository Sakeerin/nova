# ADR 0021 — Judging the Phase 2 gate's ratio against Bun by paired rounds

## Status

Accepted (2026-10-03).

## Context

`nova-spec/60-EXAMPLES.md` §5 states the gate as "Benchmark vs Bun on same
hardware shows ≥ 1.0x req/sec ratio." It does not say how a set of readings
shows that.

**The rule in use until now.** Every ratio measurement recorded in
`examples/05-json-api/BENCHMARK.md` judged the ratio by the ranges'
extremes, one pinning condition at a time.
- It would have been met only if Nova's slowest reading was above Bun's
  fastest. No run was ever judged met.
- No tracked file chose that rule for the gate. The same rule appears in
  `docs/superpowers/plans/2026-10-01-gc-page-heap.md` for before/after gain
  claims, with "take three more of each" when ranges overlap.
  "(gate-remeasure-4)" borrowed both.

**Nor did pinned alone decide.** "(gate-remeasure-2)" recorded the ratio as
not met on its unpinned cells while its pinned cells straddled 1.0.
"(gate-remeasure-4)"'s predictions framed the verdict under both
conditions.

**On this development host, the extremes rule often cannot settle a ratio
near 1.0, and more readings can undo a result it does settle.**
- **Added readings only widen ranges.** Once Nova's range and Bun's
  overlap, no further readings from the same run can separate them.
- **Bun's pinned readings have spread wider than the gap being judged.**
  They spread 1.65x within "(gate-remeasure-2)" and 1.56x within
  "(gate-remeasure-4)".
- **"(gate-remeasure-4)" shows both effects.**
  - Its first three rounds settled the pinned ratio at 1.010–1.269, wholly
    above 1.0; six rounds gave 0.708–1.334.
  - Over the six, Nova was faster in four pinned rounds and five unpinned.
- **It did settle one ratio near 1.0:** "(gate-remeasure-3)"'s pinned
  0.886–0.993, wholly below.

**The readings are already taken in pairs.** Every run has measured each
condition's Nova and Bun readings back to back. A paired comparison uses
that; a range comparison throws it away.

## Decision

- **Pairs and rounds.**
  - **A pair** is one Nova reading and one Bun reading under the same
    pinning condition, run back to back, each a fresh process. A pair goes
    to Nova if Nova's req/sec is at least the margin below times Bun's.
  - **A round** is two pairs: pinned, then unpinned. Below, a round's
    pinned pair is called its pinned round.
- **The margin corrects for wire framing.**
  - Bun writes a `Date` header that Nova does not. In "(gate-remeasure)"
    on `5efcc2e`, and again in "(fixed-vs-json)" on `68d0b94`, both on
    2026-10-02, that made Bun's `/users` response 713 bytes against Nova's
    676 (`examples/05-json-api/BENCHMARK.md`). Neither size has been
    measured since.
  - `docs/superpowers/specs/2026-09-11-bun-ratio-design.md` already ruled
    that this bias favours Nova, and that a ratio within a few percent of
    1.0 must not be reported as a pass because of it.
  - So the margin is Bun's response bytes over Nova's, and never less than
    1.0. On those sizes it is 713/676, about 1.055. A pair goes to Nova
    only if Nova moved at least as many wire bytes per second as Bun.
  - Each run measures both responses' bytes again, before its first
    reading, and uses its own margin.
  - The real throughput cost of those 37 bytes is not measured. Charging
    them at their full share of the response is expected to err against
    Nova. "(native-floor)" found that a loopback send costs tens of
    microseconds on this host even from native code, which suggests much
    of a request's cost is per send rather than per byte. How a write's
    cost depends on its size has not been measured. Erring against Nova is
    the direction the gate can afford.
- **Twelve rounds, fixed before the run.**
  - A run is never extended. A run that ends inconclusive is reported as
    inconclusive, and a later run takes its own twelve.
  - A run is not repeated on unchanged code to replace its verdict. Each
    run's verdict stands for the code it measured.
- **The order alternates.**
  - Odd rounds: Nova pinned, Bun pinned, Nova unpinned, Bun unpinned.
  - Even rounds: Bun pinned, Nova pinned, Bun unpinned, Nova unpinned.
- **Pinned decides. This is a change:** earlier records counted either
  condition.
  - Both sides are pinned to core 0 and the mask is read back, as
    `docs/benchmarks/README.md` describes.
  - That file already reports pinned as the headline, because it is the
    single-core comparison that matches Nova's executor
    (`docs/adr/0009-async-execution-model.md`).
  - Unpinned pairs are run and reported beside it; they do not decide.
- **The verdict:**
  - **met** if Nova takes at least 10 of the 12 pinned rounds;
  - **not met** if Nova falls short of the margin in at least 10 of the 12;
  - **inconclusive** otherwise. Inconclusive leaves §5 not met.
- **Why 10.** Suppose each pinned round went to either side independently,
  with equal chance.
  - A given side would then take 10 or more of 12 with probability
    79/4096, about 1.9%. That is a one-sided sign test for each verdict.
  - Either decisive verdict would come out with probability 158/4096,
    about 3.9%.
  - Nine or more would have probability 299/4096, about 7.3%. So 10 is
    the smallest count that keeps one side's chance under 5%.
- **The rest of the method is unchanged:**
  - ten users seeded, 200 connections, 30 s after a 5 s warmup;
  - the generator `nova-bench-http`;
  - the equivalence check and the generator's self-test before the first
    reading;
  - predictions written before any reading.
- **Recorded as description, not as the criterion:**
  - each pair's plain req/sec ratio, the ranges and their extremes, and
    the median pinned-round ratio;
  - each run's two response sizes and its margin;
  - for every reading, its affinity mask, `errors`, binary size and
    SHA-256, and body hash, as before.

## Alternatives rejected

- **Keep the extremes rule.** Within one run, more readings cannot settle
  it once the ranges overlap. On this host Bun's own spread has been larger
  than the gap being judged.
- **The median round ratio with a bootstrap interval.** This uses each
  round's size as well as its sign. But twelve rounds give a coarse
  interval, and the method has more to fix in advance, such as the
  resampling count and the interval level.
- **Count rounds at a plain 1.0, with the framing bias stated beside the
  verdict.** That would report as a pass what the 2026-09-11 design says
  must not be, a ratio within a few percent of 1.0.
- **Measure the 37 bytes' throughput cost first, and set the margin from
  it.** That would be more accurate, but it adds a measurement before any
  run can be judged. A later ADR can lower the margin on such a
  measurement.
- **Require both pinning conditions to pass.** That is stricter. But
  `docs/benchmarks/README.md` already reports pinned as the headline, and
  for a reason tied to Nova's executor.
- **More rounds, such as 15 of 20.** That resolves more often. But a run
  would take about 53 minutes on this host, against about 32 for twelve. A
  later ADR can raise the count.

## Consequences

- **No earlier run is re-judged.** Each took fewer than twelve rounds, and
  each fixed its method before this rule existed.
- **This rule was not chosen to pass the latest run.** In
  "(gate-remeasure-4)", Nova was faster in four of six pinned rounds, but
  cleared a 1.055 margin in only three: 1.101, 1.174 and 1.066. Three of
  every six over twelve rounds is six, inside the inconclusive band.
- **Rounds are not fully independent, and the sign test assumes they
  are.**
  - Alternating the order balances a steady drift in the host's speed
    across the two readings of a pair.
  - It does not remove shifts in the host's speed that last several
    rounds. The records show such shifts.
    - In "(gate-remeasure-3)", every cell, Bun's included, ran 1.2x–2.2x
      faster than in "(gate-remeasure-2)". Nova's server ran about
      1.4x–1.7x faster than 36 minutes earlier.
    - In "(gate-remeasure-4)", Bun's pinned readings rose in rounds 4–6.
  - Such shifts correlate rounds, which makes a false decisive verdict
    likelier than 1.9%. So a met verdict says that one run, on this host,
    favoured Nova. It is not a claim about other runs or other hosts.
- **The margin can make "not met" the verdict for a faster Nova.** Nova
  could be faster in plain req/sec in every pinned round and still fall
  short of the margin in 10 or more of them.
- **The deciding condition has an unmeasured confound.** Pinning puts each
  server on core 0, which the load generator's threads may also use.
  "(gate-remeasure)" named that as a candidate and did not measure it.
- **Which statement of the gate governs is still open.** This ADR says how
  §5's ratio is judged. It does not settle whether §5 or
  `nova-spec/00-MASTER-SPEC.md` §3's absolute 10k criterion governs. See
  `docs/benchmarks/README.md`, "Where the gate is specified
  inconsistently".
- **A run costs 48 readings.** On this host that is about 31 minutes of
  readings, and about 32 with the equivalence check and the self-test.
- **The first run judged this way** is the next gate remeasurement.
