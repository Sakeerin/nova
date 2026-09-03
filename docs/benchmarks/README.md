# Benchmarking `std/http`'s read-and-parse path

This directory holds the procedure for taking one throughput number against
Nova's HTTP server, and the dated observations that procedure has produced.
Three pieces cooperate:

- `crates/nova-bench-http` -- a dependency-free Rust load generator (`std::net`
  and `std::thread` only). It drives keep-alive HTTP/1.1 against a target and
  prints a machine-readable `RESULT` line. Its `--self-test` mode drives the
  same load against a trivial fixed-response server built into the same
  binary, which is how this procedure gets the generator's own ceiling with
  nothing external installed.
- `docs/benchmarks/server.nova` -- the Nova server under test. It answers
  every request with the same fixed JSON body.
- `docs/benchmarks/http-fixed-response.md` -- one dated observation per
  machine that ran this procedure. This file is the procedure; that one is
  the record. Re-running on new hardware appends an observation there rather
  than editing anything here, so this document does not quietly drift into
  describing whichever machine happened to run it last.

## What is measured, and what is not

This measures `std/http`'s read-and-parse path: `read_request` parsing an
HTTP/1.1 request off the wire, repeated for as many keep-alive requests as
fit in the run's window, on a server that does real accept/read/write I/O
over real loopback sockets.

It does **not** measure response serialization. `docs/benchmarks/server.nova`
builds the response's wire bytes once, outside its accept loop, with
`Response::text(200, "{\"ok\":true}").to_bytes()`, and reuses that same
`Bytes` value on every write, for every request, on every connection --
`Response::to_bytes`'s allocation cost never runs inside the timed window.

So the number this procedure produces is a **ceiling for a real server**
that serializes a response per request, not a simulation of one that does.
Measuring the delta a real per-request serialization step would cost is a
natural follow-up; nobody has done it as part of this increment.

### The request side, and three costs it leaves at their floor

The generator sends one fixed request, 36 bytes (`REQUEST` in
`crates/nova-bench-http/src/main.rs`): the request line `GET / HTTP/1.1`, one
header `Host: nova-bench`, and the blank line that ends the head. No body.
Three costs `std/http` documents in its own source are consequently exercised
at or near their minimum, and each narrows what a figure from here covers:

- **Header materialisation runs at its one-header minimum.** `std/http`'s
  design spec
  (`docs/superpowers/specs/2026-09-01-std-http-request-parsing-design.md`
  section 7) puts eager materialisation at about 18 microseconds per request
  -- a figure its own sentence scopes to a request **with ten headers**, and
  which is per-header arithmetic (20 GC allocations for ten headers' strings,
  at that spec's measured ~900 ns each), so it scales with the header count.
  At one header this run pays roughly a tenth of it, so the per-request
  header cost here is near its floor rather
  than representative. `docs/benchmarks/server.nova`'s own header records the
  consequence: that spec nominated the first benchmark as what would confirm
  or refute its 18 microsecond figure, and this benchmark does neither.
- **The request-body read loop is never entered at all.** The request declares
  no `Content-Length`, so `read_request`'s `want` is 0 and its
  `while body.len() < want` loop (`std/http/lib.nova`) runs zero iterations.
  That loop accumulates with `Bytes::concat`, and its own comment records the
  bytes copied as triangular in the read count -- around a 128x amplification
  to assemble a 1 MiB body 4096 bytes at a time, stated there as a floor
  rather than a ceiling, and as "a lever attacker-controlled input can still
  pull on the worst-case path". The behaviour `std/http` flags as
  attacker-controlled therefore sits entirely outside this measurement.
- **Head accumulation runs at its best case.** `read_request` accumulates the
  head with the same per-call-O(n) `concat`, which its doc comment records as
  O(n*k) in the read count `k`. A 36-byte request arrives in one segment, so
  `k` is 1: one concatenation, the cheapest `k` this path has.

So **"this measures `std/http`'s read-and-parse path" is broader than what
actually ran.** What ran is the single-read, one-header, empty-body corner of
that path. The example the gate names does carry bodies --
`nova-spec/60-EXAMPLES.md` section 5's listing has a
`.post("/users", ...)` handler whose first line is
`req.body_json::<User>()?`, even though the `wrk` line in the same section
drives `GET /users`. So a body-carrying measurement is the obvious next one,
and nobody has taken it.
None of that makes the number soft -- it is a real measurement of a real path,
taken with `errors=0`. The point is to say precisely which path.

## Build configuration

Two independent choices decide what a throughput figure actually measures:
which `nova` binary compiles the server, and which backend that binary uses
to do it.

`find_runtime_lib()` (`crates/nova-driver/src/link.rs`) resolves the Nova
runtime's static library next to the running `nova` executable (a
`NOVA_RUNTIME_LIB` environment variable overrides this; the procedure below
does not set it). So the `nova` binary you invoke as the compiler decides
which runtime profile the compiled server links against and calls into at
run time -- a `nova` out of `target/debug/` links the debug-profile runtime,
and one out of `target/release/` links the release-profile one.

`nova build` separately has two code-generation backends for the *server
program itself*: the default, Cranelift, and `--release`, which `nova build
--help` describes in its own words as "Optimizing build via the LLVM backend
(emits LLVM IR and compiles it with a discovered `clang`/`llc`); the default
is the fast Cranelift backend."

| `nova` binary | code backend | verdict |
|---|---|---|
| debug | Cranelift | measurable, misleading -- a debug runtime depresses everything |
| **release** | **Cranelift** | **measurable here, and what this procedure reports** |
| debug | LLVM | unmeasurable here -- no `clang`/`llc` this procedure can discover |
| release | LLVM | unmeasurable here -- no `clang`/`llc` this procedure can discover |

`clang`, `llc`, `clang-17`, `llc-17`, `clang++` and `lld` are all absent from
the PATH on the host that took the recorded run, **and this procedure sets
neither `NOVA_CLANG` nor `NOVA_LLC`** -- the two environment variables
`compile_ir_to_object` (`crates/nova-driver/src/link.rs`) consults for an
off-PATH `clang` or `llc` before it bails with "no LLVM toolchain found for
`--release`". So the LLVM path is unmeasurable *under this procedure on that
host*, for want of a toolchain it can discover, rather than unmeasurable in
principle: pointing either variable at an installed LLVM is what would make
that path measurable, and nobody has done it. That leaves release-`nova` plus
Cranelift as the only cell in the table both measurable and honest to report
there, and it is what the commands below build.

This bears directly on reading a figure from this procedure against the
gate's 10k (`nova-spec/00-MASTER-SPEC.md`): that figure presumably assumed an
optimizing compiler, and every number this procedure can currently produce
comes from Cranelift instead, with the LLVM path's own figure entirely
unmeasured.

## The commands, in order

```bash
# 1. A release nova, so a release runtime is what gets linked.
cargo build --release --locked --workspace

# 2. Build the server to a native binary. Cranelift backend; --release needs
#    clang/llc, which may be absent.
./target/release/nova build docs/benchmarks/server.nova -o bench-server

# 3. Start it and read the port it prints.
./bench-server

# 4. In another shell: the harness's own ceiling. MANDATORY.
./target/release/nova-bench-http --self-test --connections 200 --duration 30 --warmup 5

# 5. The measurement, same shape, against the Nova server.
./target/release/nova-bench-http --addr 127.0.0.1:<port> --connections 200 --duration 30 --warmup 5

# 6. Kill the server. It has no shutdown path and that is deliberate.
```

**A note for readers on a different shell.** Step 2's `-o bench-server` is
taken literally: `nova build --help` documents the platform executable
suffix as added only "(default: `<file stem>` in the current directory, with
the platform executable suffix)" when `-o` is omitted, so an explicit
`-o bench-server` produces a file named exactly `bench-server`, with no
`.exe`. Git Bash's `./bench-server` runs that file directly, which is why
the commands above work as written from Git Bash. They do not from
PowerShell or cmd.exe, and both fail differently rather than merely
declining: PowerShell treats an extensionless file as a document rather
than a program (`& .\bench-server` fails with "Cannot run a document in the
middle of a pipeline"), and cmd.exe cannot resolve the bare name at all
(`bench-server` reports "is not recognized as an internal or external
command, operable program or batch file", the same message cmd.exe gives
for a name that does not exist anywhere on `PATH`). Both were checked with a
throwaway build on this project's Windows host rather than assumed. Passing
`-o bench-server.exe` in step 2 sidesteps both by giving either shell the
extension it needs.

## What must be recorded beside any number

A throughput figure on its own is nearly meaningless. Every entry in
`http-fixed-response.md` records, beside its numbers: CPU model and core
count, OS, `rustc --version`, what tree was measured, the code backend, the
runtime profile, the self-test ceiling, the Nova figure, and the ratio
between them.

**What tree was measured is identified by content, not by a branch-local
commit hash.** A commit made on a feature branch before it merges is
rewritten by this project's rebase-merge, so a hash recorded from mid-branch
would dangle for any later reader -- which, for a benchmark record meant to
outlive the branch, is worse than recording no hash at all, since a dangling
hash still reads as precise while pointing nowhere. Recording the base commit
that *is* already an ancestor of `main`, plus which named pieces sit on top
of it, survives that rewrite; a bare hash from the branch does not.

**The `RESULT` line does not carry the settings that define the run, so
record them beside it.** It prints `mode`, `addr`, `connections`, `requests`,
`errors`, `elapsed_ms`, `rps`, `conn_min` and `conn_max`. Of the three
settings that shape a run, only `--connections` appears there; `--duration`
and `--warmup` leave no trace in the line at all. A pasted `RESULT` line is
therefore not self-describing, and whoever appends an observation must write
the invoking command's flags alongside it, as `http-fixed-response.md` does.

**That calibration is mandatory, not advisory.** A Nova figure without its
self-test ceiling beside it is not a measurement: a reading of, say, 5,000
requests per second could be Nova's limit or the generator's own limit on
this host, and no amount of care in the surrounding prose distinguishes
those two cases after the fact. This procedure does not present one number
without the other, and neither should any observation appended here.

## Reading the ratio between the two figures

Both runs above use identical generator settings
(`--connections 200 --duration 30 --warmup 5`) specifically so the two
numbers can be divided against each other at all. That division is real, but
it licenses **exactly one** inference, and reading it as anything else is a
mistake this document exists to head off.

**What the ratio does license:** whether the generator itself was the
binding constraint on the measured run. If the Nova figure sits close to the
self-test ceiling, the generator -- not `std/http` -- was probably the
limit, and the Nova figure should be read as a lower bound on what `std/http`
can do rather than a measurement of it. If the Nova figure sits well below
the ceiling, the generator had headroom to spare and the figure reflects
`std/http`'s own path.

**What it does not license:** a claim that Nova is some multiple slower than
a real server. The two servers being compared do not share a concurrency
model. `--self-test`'s in-process server is one OS thread per connection,
free to run across every core the host has. `docs/benchmarks/server.nova` is
200 Nova tasks cooperatively scheduled on **one** thread, and that is a
correctness requirement rather than a current limitation of this server:
Nova's garbage collector keeps its entire heap in a thread-local
(`docs/adr/0009-async-execution-model.md`, section 1 -- "The GC heap is
thread-local" -- explains why thread-per-task would need a global heap
behind a lock instead, and rejects it on those grounds), so single-threaded
execution is what keeps that collector sound, not an oversight the ratio is
entitled to penalize. A phrase like "Nova reached N% of the harness ceiling"
reads as exactly that penalizing comparison even when no one intends it, so
this document states outright that it is not a supported reading of the
number: **the ratio says only whether the generator was the bottleneck, and
says nothing about how Nova compares to a multi-threaded server.**

**The self-test ceiling this procedure produces is a conservative estimate
of the generator's real capacity, and that cuts in one direction only.**
During the self-test run, the generator's own threads and the in-process
self-test server's threads contend for the same cores, inside the same
process. During the real measurement, the generator's threads instead share
cores with a separate, single-threaded `bench-server` process. The
configuration that actually matters -- the real measurement -- is the one
the self-test run does *not* reproduce, and the contention the self-test
run *does* have pulls its own ceiling down below what the generator can
really push. That understates the ceiling, which in turn understates how
much headroom the generator had -- so this asymmetry can only ever make "the
generator was the bottleneck" look more plausible than it really was, never
less. Read any ratio close to 1.0 with that thumb on the scale already
pressing in its favor; it is not a reason to distrust a ratio that comes out
small.

**`rps` divides by a window that includes thread spawn and join, and the two
recorded runs differ in how much of that they carry.** `run_load` takes its
`start` before spawning the workers and reads `elapsed` after joining them,
so `elapsed_ms` is spawn plus `--duration` plus join, not `--duration`
itself. In the recorded runs that gap is 44 ms for the Nova figure and about
3.15 seconds for the self-test ceiling (`elapsed_ms=30044` against
`elapsed_ms=33153`, both taken at `--duration 30`): starting and stopping 200
generator threads costs more when a 200-thread server in the same process is
contending for the same cores. Both reported figures are therefore slightly
understated, the ceiling considerably more so than the Nova number, and
re-normalising both to their `--duration` window lowers the ratio rather than
raising it -- the same direction the contention asymmetry above already
pushes, so nothing above changes. This procedure reports the one ratio
computed from the `RESULT` lines as printed; a reader who divides `requests`
by `--duration` instead arrives at a slightly smaller figure, and this
paragraph is why.

## Where the gate is specified inconsistently

Three things below were found while designing this measurement and are
recorded rather than silently resolved. This is the roster that was found,
not a claim that the gate's wording holds no other inconsistency.

**The criterion is given twice, and the two do not agree.**
`nova-spec/00-MASTER-SPEC.md` (its Phase 2 gate) reads: "`examples/05-json-api`
serves 10k+ req/sec on benchmark hardware. Document benchmark methodology in
`docs/benchmarks/`." `nova-spec/60-EXAMPLES.md` section 5's own gate reads:
"Benchmark vs Bun on same hardware shows ≥ 1.0x req/sec ratio. Document
numbers in `examples/05-json-api/BENCHMARK.md`." An absolute 10k and a ratio
against Bun can disagree in either direction -- 10k could be reached while
the ratio fails, or the ratio could clear 1.0 well under 10k if Bun itself is
slower on the same machine. This procedure measures only the absolute
figure; the Bun ratio is unmeasured.

**The destination is given twice, and the second is unsatisfiable on this
tree.** `60-EXAMPLES.md` names `examples/05-json-api/BENCHMARK.md`, but
`examples/` holds `01-hello-world`, `02-fibonacci` and `03-producer-consumer`
-- no `05-json-api` directory exists to hold that file. `docs/benchmarks/` is
where the number goes instead.

**The named tool does not run on this project's own Windows development
host.** `60-EXAMPLES.md`'s own methodology names `wrk -t8 -c200 -d30s
http://localhost:3000/users`. Checked on that host: of `wrk`, `oha`,
`bombardier`, `hey`, `ab`, `k6` and `vegeta`, none is installed, and `wrk`
itself is POSIX-only, so it would not run there natively regardless.
`crates/nova-bench-http` exists because of this gap. **Figures from
`nova-bench-http` are consequently not directly comparable to published
`wrk` numbers**: different generator, different connection handling,
different measurement window.

## Known properties of this measurement

- **Single-core throughput.** `docs/adr/0009-async-execution-model.md`
  establishes that Nova's async executor is single-threaded because the
  collector's heap is thread-local, not because a multi-threaded executor
  was merely deferred. Every figure this procedure produces is therefore a
  single core's throughput on hardware that has more than one.
- **One Nova task per connection.** Keep-alive holds each connection open
  across many requests, and `docs/benchmarks/server.nova` spawns one task
  per accepted connection, so `--connections 200` means 200 tasks
  cooperatively scheduled on that one thread, not 200 independent workers.
- **No read timeout.** `read_request` parks with no deadline
  (`std/http/lib.nova:450` discloses this directly, alongside why: the
  server's byte-valued limits never impose a temporal one). `std/net`'s own
  `TcpStream::read_timeout` exists and is not used here. What that costs, in
  that source's own terms: a peer that connects and sends nothing, or sends a
  partial head and then goes silent, holds one task and one socket parked
  indefinitely for the price of a single `connect`. **This generator is not
  such a peer**, and no run recorded here stranded a task: every exit path in
  `worker` (`crates/nova-bench-http/src/main.rs`) returns and so drops that
  connection's `TcpStream`, and `run_load` joins every worker before the
  `RESULT` line prints, so by the time a run reports, all of its connections
  have closed. A parked read then wakes on the FIN with a zero-length chunk,
  which `read_request` turns into `Err` ("connection closed mid-head") and
  `serve`'s own `Err(e) => break` arm closes on. An earlier
  draft of this bullet illustrated the property with a connection the
  `--duration` window abandons mid-request, which is not something this
  generator can produce. The property itself stands: fine for a bounded run
  taken by hand, not fine for a service left running.
- **The server does not exit.** `block_on` cannot return while a task is
  parked, and the accept loop parks forever waiting on the next connection.
  Step 6 kills the process; there is no shutdown path to wait for instead,
  and its absence is not an oversight.

## Connection cap

Keep `--connections` below roughly 1,000 on either platform. On Unix, the
poller's `FD_SETSIZE` rejection path is documented in its own source
(`crates/nova-runtime/src/poll.rs`) as "still reasoned, not measured" --
no test reaches it, since doing so needs a socket descriptor numbered above
`FD_SETSIZE`, so a run that pushes a connection count into that territory is
exercising a path nothing has exercised before. Windows' poller uses
`WSAPoll` instead (same file) and is not bound by `FD_SETSIZE` at all, but
the 200-connection shape used throughout this procedure stays well under the
cap on both, which is deliberate: reproducing the same command on either
platform should not itself be the source of a difference in the result.
