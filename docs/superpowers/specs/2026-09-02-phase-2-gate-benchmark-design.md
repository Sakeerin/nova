# Phase 2 gate: a measured HTTP throughput number — design

**Status:** approved 2026-09-02. Implements the measurement half of `nova-spec/00-MASTER-SPEC.md` §3's Phase 2 gate, narrowed; see §1 and §2.

**Increment:** the first step toward Phase 2's gate. It produces a reproducible
methodology and one honest number. It does **not** write the gate's examples and
does **not** claim the gate is passed.

---

## 1. What this closes, and the four things it does not

Phase 2's gate reads: *"`examples/05-json-api` serves 10k+ req/sec on benchmark
hardware. Document benchmark methodology in `docs/benchmarks/`."* Neither half
exists. `docs/benchmarks/` is absent, and `examples/` holds `01-hello-world`,
`02-fibonacci` and `03-producer-consumer` — none of the three the gate chain
names.

`std/http`'s server half landed before this increment, so a Nova HTTP server can
now be written. **This increment builds the smallest one that answers a
question**: what throughput does the HTTP path sustain, and how does that compare
to what the load generator itself can push?

Deliberately not here:

- **`examples/05-json-api` as specified.** `nova-spec/60-EXAMPLES.md` §5's listing
  requires a router (`http.Server.new().get(...)`, whose `Handler` type is
  `P0001`), `@derive(ToJson, FromJson, Clone)`, the `?` operator, turbofish
  (`body_json::<User>()`, `parse::<Int>()`), pointer dereference with compound
  assignment (`*next += 1`), struct-update syntax with field shorthand
  (`User { id, ..user }`), `String::parse`, `Map::values()`, and
  `Response::json`/`.status(201)` builders. Measured on this tree: `@derive` does
  not exist, `Map` has `keys()` and no `values()`, and there is no
  `String`-to-number conversion in the language at all. That listing is a language
  project, not an example.
- **`examples/03-http-server` as specified**, which is milder but still
  unwritable: `import std/http` is not how `std` works — ADR 0004 glob-imports it
  implicitly — and it too calls `http.Server.new()`.
- **A verdict on the gate.** See §10: the number is the deliverable, and a figure
  below 10k is a finding rather than a failure of this increment.
- **Optimisation.** If the number is low, where to optimise is the next
  conversation, informed by this one.

---

## 2. Where the gate is specified inconsistently, recorded rather than resolved

Each of the following was measured against the tree, and this increment records it
rather than silently picking a winner. The roster is what was found, not a claim
that nothing else is inconsistent.

**The criterion is specified twice, and the two are not equivalent.**
`00-MASTER-SPEC.md` §3 wants 10k+ req/sec absolute. `60-EXAMPLES.md` §5 wants a
ratio: *"Benchmark vs Bun on same hardware shows ≥ 1.0x req/sec ratio."* Those can
disagree in either direction — an absolute 10k could pass while the ratio fails,
and a ratio above 1.0 could hold well below 10k if Bun is slower on the same box.
**This increment claims neither.** The absolute figure is measurable with what is
present; the ratio needs Bun installed plus a second methodology, and is
unmeasured.

**The destination is specified twice.** §3 says `docs/benchmarks/`; §5 says
`examples/05-json-api/BENCHMARK.md`. The latter is unsatisfiable because that
directory does not exist and this increment does not create it, so
`docs/benchmarks/` is where the number goes.

**The tool cannot run on this project's own dev platform.** §5's methodology names
`wrk -t8 -c200 -d30s`. Measured: of `wrk`, `oha`, `bombardier`, `hey`, `ab`, `k6`
and `vegeta`, none is installed on this host — `curl` is the only HTTP client
present. And `wrk` is POSIX-only, so it does not run natively on this Windows host
at all. Hence the generator in §4.

**The consequence that is easiest to miss, and §7 must state it:** figures from a
generator written here are **not directly comparable** to published `wrk`-based
numbers. Different generator, different connection handling, different measurement
window. Comparing our number against a blog post's Bun figure would draw a
conclusion the data does not support.

---

## 3. Architecture

The table below is the roster and its `status` column says which are new. Only the harness test is a modification; everything else is added.

| path | status | responsibility |
|---|---|---|
| `crates/nova-bench-http/` | new | the load generator, a Rust bin |
| `docs/benchmarks/README.md` | new | the procedure: how to reproduce |
| `docs/benchmarks/http-fixed-response.md` | new | one dated observation: what a machine gave |
| `docs/benchmarks/server.nova` | new | the Nova server under test |
| `crates/nova-cli/tests/run_tests.rs` | modified | one smoke test |

`Cargo.toml`'s `members = ["crates/*"]` is a glob, so the new crate joins the
workspace with no manifest edit.

**The new crate must carry `[[bin]] bench = false`.** The existing crates carry
`bench = false` on their lib and bin targets for one reason: without it,
`cargo bench --workspace -- --output-format bencher` hands a libtest harness a flag
only criterion accepts. A new bin that omits it breaks the Benchmarks workflow,
which is green today.

### Why the server lives under `docs/benchmarks/` and is not called `03-http-server`

Every existing example is `examples/<name>/src/main.nova` and is pinned by a
`nova run` test against a golden. A load-test server has no golden: its output is
a number that varies by machine.

Naming it `03-http-server` would also half-resolve drift that two spec files
currently record as open — `00-MASTER-SPEC.md` §2's tree and `60-EXAMPLES.md` §3
both name `03-http-server` while `examples/` holds `03-producer-consumer`.
Shipping something the spec does not describe under the name the spec reserves is
worse than leaving the drift recorded.

**This puts Nova source in a docs directory, which is unusual for this tree and is
a deliberate trade.** The alternative is a fourth top-level directory, which
`00-MASTER-SPEC.md` §2's folder structure — marked FINAL — does not have.
Co-locating the server with the methodology that explains it is the lesser
oddity, but it is an oddity.

### The procedure/observation split

`README.md` is *how to reproduce*. `http-fixed-response.md` is *one dated
observation from one machine*. Re-running on new hardware appends an observation
rather than editing the procedure — otherwise the procedure drifts toward
describing whichever machine ran last.

---

## 4. The load generator

`crates/nova-bench-http`, a Rust bin, with **no new dependency**.

`std::net::TcpStream` and `std::thread` are sufficient: one OS thread per
connection, each looping — write a request, read the response, count it — with
keep-alive, so many requests per connection. That is what `-c200` means in §5's
own methodology. Blocked threads cost little, and the host has 12 cores.

No tokio, no reqwest, no hyper. This matters here: `httparse` had to be justified
as the single new dependency of the previous increment, and test tooling is a poor
place to spend that argument again. Nova has no HTTP client of its own —
`std/http` shipped the server half only — so the generator must be Rust
regardless.

**Flags:** `--addr`, `--connections`, `--duration`, `--warmup`.

**Report:** total requests, elapsed, requests per second, error count, and
per-connection minimum and maximum so imbalance is visible rather than averaged
away.

### Calibration is a mode, not a manual step

`--self-test` spawns a trivial fixed-response server on a `std::net::TcpListener`
**inside the same binary** and drives load against it. One command yields the
generator's own ceiling on the host, with nothing external installed.

**Without this the Nova figure is unfalsifiable.** A reading of 5k could be
Nova's limit or the harness's, and no amount of care in the prose distinguishes
them. §7 requires both numbers and their ratio; a Nova figure presented without
its harness ceiling is not a measurement.

---

## 5. The Nova server

`docs/benchmarks/server.nova`, written in the language that exists.

`bind` in `main`, then a loop over `accept().await`, `spawn`ing one task per
connection. Each connection task loops `read_request(conn, Limits::default())`
and writes the response, leaving the loop on `Err`.

**That shape is forced, not chosen.** Staging two socket waits in a single poll
aborts the process (`stage_park` in `crates/nova-runtime/src/task.rs`), so the
task parked in `accept` cannot also read a connection. There is no `select` or
`race`.

The server binds `127.0.0.1:0` and prints the kernel's chosen port via
`local_port()`. No fixed port, no collision, no port file — and the same line
serves both the human following the procedure and the smoke test parsing stdout.

### One decision inside it that changes what the number means

**The response's wire bytes are built once, outside the loop, and reused.**
`Response::to_bytes` allocates; hoisting it isolates the read-and-parse path,
which is where the ~18 microseconds of header materialisation that `std/http`'s
own design spec discloses actually lives.

**The consequence, which §7 must state rather than imply: the recorded number
excludes response serialisation.** It is a ceiling for a real server rather than a
simulation of one. Measuring the delta by not hoisting is a natural follow-up and
is not in scope.

### Two properties the procedure must accommodate

**The server never exits.** `block_on` cannot return while a task is parked in
`accept`, so the procedure kills the process rather than expecting a clean
shutdown.

**There is no read timeout.** `read_request` parks with no deadline, so any
connection the generator abandons holds its task until the process dies. Bounded
runs are fine; a service would not be. `std/net::read_timeout` exists and
`std/http` does not use it, which that module's own header records.

**The server logs nothing per request.** Per-request logging at these rates would
become the bottleneck we accidentally measured.

---

## 6. The build configuration, which is the decisive constraint

`find_runtime_lib()` in `crates/nova-driver/src/link.rs` resolves the runtime
static library **next to the `nova` executable**, with a `NOVA_RUNTIME_LIB`
environment override taking precedence. So a `nova` from `target/debug/` links the
**debug** runtime and a release-built one links the release static library.

`nova build` has two backends: the default **Cranelift**, and `--release`, which
is *"Optimizing build via the LLVM backend (emits LLVM IR and compiles it with a
discovered `clang`/`llc`)"*.

That gives four combinations:

| `nova` binary | code backend | verdict |
|---|---|---|
| debug | Cranelift | measurable, **misleading** — a debug runtime depresses everything |
| **release** | **Cranelift** | **measurable here, and what this increment reports** |
| debug | LLVM | unmeasurable on this host |
| release | LLVM | unmeasurable on this host |

**Measured: `clang`, `llc`, `clang-17` and `llc-17` are all absent**, so the LLVM
path cannot run here and the optimised-codegen number stays unmeasured. That bears
directly on reading the result against the gate's 10k, which presumably assumed an
optimising compiler.

**The procedure therefore requires `cargo build --release` first**, and §7 records
both axes explicitly. "Nova served N req/sec" without naming the backend *and* the
runtime profile is close to meaningless — and the naturally-reached-for sequence,
`cargo build` then `nova build`, silently produces the misleading row.

---

## 7. The methodology document

`docs/benchmarks/README.md` contains:

- **What is measured, and what is deliberately not** — the read-and-parse path,
  excluding response serialisation, with §5's reason.
- **The commands in order**: `cargo build --release`; `nova build` the server;
  start it and read the printed port; `--self-test` for the ceiling; measure;
  kill the process.
- **What must be recorded beside any number**: CPU model and core count, OS,
  rustc version, the commit SHA, the code backend, the runtime profile, the
  self-test ceiling, the Nova figure, and the ratio between them.
- **That calibration is mandatory, not advisory.** The document refuses to present
  a Nova number without its harness ceiling.
- **The three §2 conflicts**, including that these figures are not comparable to
  published `wrk` numbers.
- **The known properties**: single-core throughput by ADR 0009's correctness
  requirement, one Nova task per connection, no read timeout, the server does not
  exit.

`docs/benchmarks/http-fixed-response.md` holds one dated run with the hardware it
came from.

---

## 8. Testing

**One smoke test** in `crates/nova-cli/tests/run_tests.rs`. It builds nothing new:
it starts the server, parses the printed port from stdout, and runs the generator
for one second at one connection. It asserts that the server printed a port, that
the generator exited successfully, that the error count is zero, and that at least
one request completed.

**It asserts no throughput number**, so it cannot flake on timing.

**It is a normal test, not `#[ignore]`d.** CI's Test job has an advisory step that
runs exactly the ignored tests, so a load test placed there would run on every
push inside a step whose failures are already tolerated and therefore unread —
both slow and invisible. A one-second single-connection check is cheap enough to
be an ordinary test, and the existing `std/net` fixtures already use real sockets
in CI.

**The full load run is never executed by CI.** `cargo build --locked --workspace`
and `cargo clippy --all-targets` keep the generator from rotting; the smoke test
keeps it honest; the number is taken by hand.

Mutations that must fail, to be run and reported rather than predicted:

1. Make `--self-test` report the ceiling without actually driving load — the
   smoke test must fail. If it passes, the smoke test is not exercising the
   generator.
2. Make the server write a response without reading the request — the smoke test
   must fail on a request count or an error count.
3. Point the generator at a closed port — it must report a clear failure rather
   than zero requests per second, which would read as a valid measurement.

---

## 9. Failure modes, named up front

- **The known Windows async flake**, roughly one run in four, historically
  `0xc0000005` but not always — a 2026-08-29 instance carried no crash code at
  all. Cause not established. The smoke test is the exposure: re-run, say so,
  attribute nothing, and do not grep for that code as the test of whether it
  fired.
- **`FD_SETSIZE` on Unix.** The poller's rejection path above the limit is
  documented in `crates/nova-runtime/src/poll.rs` as *"still reasoned, not
  measured"*. The procedure caps connections below roughly 1000 and says why.
  Windows uses `WSAPoll` and is not bound this way.
- **One Nova task per connection**, because keep-alive holds the connection open.
  At 200 connections that is 200 tasks on one thread, which is why the number is
  read as single-core throughput.
- **Generator and server share a host.** The server uses one core and the host has
  twelve, so this is acceptable — and it is recorded rather than assumed harmless.

---

## 10. Success criteria

1. `crates/nova-bench-http` builds, carries `[[bin]] bench = false`, and adds no
   dependency.
2. `--self-test` reports a ceiling, and the smoke test fails under mutation 1.
3. The Nova server serves the generator over keep-alive with zero errors.
4. `docs/benchmarks/README.md` records the procedure, and every item §7 lists is
   present.
5. `docs/benchmarks/http-fixed-response.md` records one run with its hardware, its
   backend, its runtime profile, the harness ceiling, the Nova figure, and the
   ratio.
6. The suite is green on all three platforms, counts summed per platform and split
   by step.

**A number below 10k satisfies these criteria.** The deliverable is a reproducible
methodology and an honest figure; where it lands is the finding. Nothing here
claims the Phase 2 gate is passed, and §11's amendments say so in the records.

---

## 11. Records to amend

- **`nova-spec/60-EXAMPLES.md` §5** — a dated amendment recording that its listing
  is written in a Nova that does not exist (§1's roster), that its
  `BENCHMARK.md` destination is unsatisfiable until that example exists, and that
  its `wrk` methodology does not run on this project's Windows host.
- **`nova-spec/00-MASTER-SPEC.md` §3** — a dated amendment recording that the gate
  is specified twice with non-equivalent criteria, and that this increment
  measures the absolute figure while leaving the Bun ratio unmeasured.
- **`CHANGELOG.md`** under `[Unreleased]`.
- **The example-numbering drift** stays recorded and unresolved; this increment
  does not touch it, and §3 explains why the server declines the reserved name.
