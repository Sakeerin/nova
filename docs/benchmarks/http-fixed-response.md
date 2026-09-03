# Observation: fixed-response throughput, 2026-09-03

One dated run, taken by following `docs/benchmarks/README.md` exactly as
written. See that file for what these numbers do and do not mean; this file
only records what one machine gave.

## Machine

- **CPU:** AMD Ryzen 5 5600H with Radeon Graphics -- 6 physical cores, 12
  logical processors. (`nproc` reports 12; recorded as 12 logical processors
  rather than "12 cores" because the hardware has half that many physical
  cores.)
- **OS:** Windows 11, build 26100 (`MINGW64_NT-10.0-26100`, x86_64).
- **Toolchain:** `rustc 1.95.0 (59807616e 2026-04-14)`.

## Tree measured

Base commit `5be3a4f`, which is an ancestor of `main` and already carries
`std/http`'s server-half implementation -- that work merged before this
branch existed, and nothing in this benchmark changes it. On top of that
base, this branch adds exactly the two pieces this measurement needs: the
`nova-bench-http` load generator (`crates/nova-bench-http`) and the Nova
server under test (`docs/benchmarks/server.nova`).

This branch's own commits are deliberately not cited by hash. They are
branch-local, and this project's rebase-merge rewrites branch-local commits
when a branch lands -- a hash recorded here would dangle for any reader who
opens this file afterward, which is worse than recording no hash at all,
since a dangling hash still reads as precise while pointing nowhere.
Identifying the tree by what it contains, layered on a base commit that is
already an ancestor of `main`, survives that rewrite; a bare branch-local
hash would not.

## Build

- **Code backend:** Cranelift (`nova build docs/benchmarks/server.nova -o
  bench-server`, no `--release`). `clang`, `llc`, `clang-17`, `llc-17`,
  `clang++` and `lld` were all confirmed absent from this host's `PATH`
  before this run, so the LLVM backend (`nova build --release`) could not
  run here at all -- that path's own figure stays unmeasured on this
  machine.
- **Runtime profile:** release. `cargo build --release --locked --workspace`
  ran first, and `target/release/nova.exe` links the release-profile
  `nova_runtime.lib` that sits beside it, per `find_runtime_lib()`
  (`crates/nova-driver/src/link.rs`).

## The two runs

Both used identical generator settings so they can be compared at all:
`--connections 200 --duration 30 --warmup 5`.

**Self-test ceiling** (the generator against its own in-process
fixed-response server -- mandatory calibration, not optional context):

```
RESULT mode=self-test addr=127.0.0.1:61389 connections=200 requests=3080737 errors=0 elapsed_ms=33153 rps=92923.6 conn_min=850 conn_max=31234
```

**Nova figure** (the same generator, same settings, against
`docs/benchmarks/server.nova`):

```
RESULT mode=target addr=127.0.0.1:59903 connections=200 requests=358727 errors=0 elapsed_ms=30044 rps=11940.0 conn_min=1760 conn_max=1862
```

Both runs reported `errors=0`.

## Ratio, and what it means here

11940.0 / 92923.6 ≈ 0.128 -- the Nova figure landed at roughly 12.8% of the
self-test ceiling, which is a statement about how much headroom the load
generator had and not a comparison of the two servers; or put the other way,
the ceiling was roughly 7.8 times the Nova figure. The qualification is
attached to the number rather than left to the paragraphs below because this
figure is the one most likely to be quoted on its own, and quoted bare it
says something this run cannot support.

That is not close to 1.0, so per `README.md`'s "Reading the ratio" section,
this is the case where the generator had headroom to spare: it was not the
binding constraint on this run, and the Nova figure above reflects
`std/http`'s own read-and-parse path rather than a lower bound depressed by
the harness. This reading is, if anything, understated rather than
overstated: the self-test ceiling itself is a conservative estimate of the
generator's real capacity, because that calibration run has the generator's
own threads contending with the in-process self-test server's threads on
the same cores, while the real run above has the generator's threads
sharing cores with only a single separate thread (`docs/benchmarks/server.nova`
is single-threaded by ADR 0009's correctness requirement). A less-contended
ceiling would sit at or above 92923.6, which would only push this ratio
lower still.

The per-connection spread is worth reading alongside the ratio rather than
past it: the self-test run's `conn_min=850`/`conn_max=31234` spans a wide
range consistent with independent OS threads competing for scheduling
across the host's cores, while the Nova run's `conn_min=1760`/`conn_max=1862`
is tight, consistent with 200 tasks taking turns cooperatively on the one
thread `docs/benchmarks/server.nova` runs on.

**What this ratio does not license:** a claim that Nova is some multiple
slower than a real server, or any reading of the shape "Nova reached 12.8%
of the harness ceiling" taken as a verdict on Nova's own performance. The
two servers being compared do not share a concurrency model -- see
`README.md` for why that comparison is not a fair one to draw from this
number.

## What this figure is, and is not

This run excludes response serialization: `docs/benchmarks/server.nova`
builds its response's wire bytes once, outside the accept loop, and reuses
them for every request, so `Response::to_bytes`'s allocation cost never ran
inside the timed window. 11,940.0 req/sec is therefore a ceiling for a real
server that serializes a response per request, not a simulation of one that
does.

11,940.0 numerically clears the 10k figure in `nova-spec/00-MASTER-SPEC.md`'s
Phase 2 gate. **This file makes no claim that the gate is passed.** That
gate is specified twice with non-equivalent criteria (`README.md`'s "Where
the gate is specified inconsistently"), and this run settles only the
absolute-figure half of it, on one machine, from one run, on the Cranelift
backend rather than the optimizing LLVM one, excluding response
serialization. The ratio against Bun that `nova-spec/60-EXAMPLES.md` also
asks for remains entirely unmeasured. Where this number lands is the
finding this increment set out to produce, not a verdict on the gate as a
whole.
