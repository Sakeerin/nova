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

The generator sends one request per round trip, built by `request_bytes`
(`crates/nova-bench-http/src/main.rs`): the request line
`GET <path> HTTP/1.1`, one header `Host: nova-bench`, and the blank line that
ends the head. No body.

**The path comes from `--path`, and it defaults to `/`.** So a generator
invoked exactly as the commands below invoke it sends exactly the bytes it
sent before the flag existed: the 36-byte request. That default is the reason
adding the flag left every observation `http-fixed-response.md` already held
describing the same run it had always described.
`docs/benchmarks/server.nova` answers every path with the same fixed
response, so against it the path changes nothing. **Against a target that
routes, the path decides which handler gets measured.**
`examples/05-json-api` answers `GET /` with a 404 while `60-EXAMPLES.md`
section 5's own methodology drives `/users`, so a run against a router at the
default path measures its not-found handler. Set `--path` to the route being
measured.

**On Git Bash, protect that value from MSYS's own path conversion.** A
plain `--path /users` there is rewritten before the generator ever sees it
and is then refused with `--path must begin with /` -- an exit-2 failure,
not a wrong number, on a value that visibly starts with `/`. Set
`MSYS_NO_PATHCONV=1` before the invocation; see the Git Bash note after the
command block below for the full command.

**`errors=` counts a non-2xx answer, so a run aimed at a route the target
does not serve no longer reports zero.** Alongside `--path`, a completed
response whose status line reads anything other than 2xx -- or that will not
parse as a status line at all -- began incrementing the same `errors` counter
a failed write already incremented. Round trips that got such an answer are
still counted in `requests`, so `rps` remains a round-trip rate and a wrong
route shows up as an error count near the request count rather than as a
throughput figure that drops to zero.

**What that widening catches is narrow**: a route the target answers with a
4xx, and a target answering 5xx under load. It says nothing about whether the
response body was the one expected, and it is not a general warrant that a
figure printed beside `errors=0` measured what its surrounding prose claims.
Neither is `errors=` a roster of one thing: `worker`
(`crates/nova-bench-http/src/main.rs`) hands back an error tally, and
every path that feeds it lands in that same figure -- the early
`return (0, 1)` exits before its loop begins, and the `errors += 1`
sites within it. **The population is whatever those two patterns match
inside `worker` today**, which is the check; a count written here is
not. That is not a hypothetical: this paragraph named a count for each,
and the `errors += 1` count was one lower earlier on the same branch
that wrote it, before the response-status check above added a site.
Among the members are a connection that could not be established, a
socket option that could not be set, a request write that failed, a
read that timed out, a connection closed mid-response, and an answer
outside 2xx -- so a non-zero count is a reason to look at the code
rather than a diagnosis in itself.

Back to that request's shape. One header and no body leave three costs
`std/http` documents in its own source exercised at or near their minimum,
and each narrows what a figure from here covers:

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
  **Measured 2026-09-29 by a different benchmark -- see "Differential
  decomposition" at the end of this file.** The bullet above still
  describes the run it was written about; it is not rewritten.
- **The request-body read loop is never entered at all.** The request declares
  no `Content-Length`, so `read_request`'s `want` is 0 and its
  `while body.len() < want` loop (`std/http/lib.nova`) runs zero iterations.
  **This remains true of the run this bullet describes. A body-carrying
  series was taken 2026-09-29 -- see "Differential decomposition" at the
  end of this file -- and it enters that loop at its larger sizes.**
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

**Amended 2026-10-07 (branch `phase-3-0-foundations`):** a release-profile
`nova` now carries its own release runtime, and links it before the one
beside it (ADR 0027), so the profile conclusion above stands. But after
editing the runtime, rebuild `nova-cli` too, or set `NOVA_RUNTIME_LIB`.
Otherwise the server links the copy embedded at nova-cli's last build.

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

# 2. Remove BOTH names before building. `-o bench-server` writes exactly
#    `bench-server`; a stale `bench-server.exe` from an earlier build would
#    survive and is what step 3 would then run. See "Check, do not trust"
#    below -- this is not hypothetical.
rm -f bench-server bench-server.exe

# 3. Build the server to a native binary. Cranelift backend; --release needs
#    clang/llc, which may be absent.
./target/release/nova build docs/benchmarks/server.nova -o bench-server

# 4. Assert exactly ONE file, written just now, and RECORD ITS BYTE SIZE.
#    The size is the identity check: a debug-runtime build of the same
#    program differs from a release one by hundreds of kilobytes, so the
#    size recorded beside a figure is what makes its runtime profile
#    auditable afterwards.
ls -l bench-server*

# 5. Start it and read the port it prints.
./bench-server

# 6. In another shell: the harness's own ceiling. MANDATORY.
./target/release/nova-bench-http --self-test --connections 200 --duration 30 --warmup 5

# 7. The measurement, same shape, against the Nova server.
./target/release/nova-bench-http --addr 127.0.0.1:<port> --connections 200 --duration 30 --warmup 5

# 8. Kill the server. It has no shutdown path and that is deliberate.
```

### Check, do not trust

**2026-09-11.** Everything above about the runtime profile was already in
this document on 2026-09-10, including the verdict table calling a debug
`nova` "measurable, misleading", and a gate figure was published anyway from
a binary built by a debug `nova`. The record beside it asserted "release
runtime profile" as a stated parameter. **A warning without a check is what
failed**, and the mechanism was the `-o` suffix rule this document explains
two paragraphs down: `-o NAME` writes `NAME`, an earlier build had left
`NAME.exe`, and the measurement asked for `NAME.exe`. An explicit path that
does not exist fails loudly; a stale file at the path you asked for succeeds
and answers every sanity check.

So steps 2 and 4 above are not tidiness. Three further habits follow, and
`examples/05-json-api/BENCHMARK.md` applies all of them:

- **Record the binary's byte size beside every figure.** It converts the
  runtime profile from a claim into something a later reader can check
  against a rebuild.
- **One fresh server process per data point.** Serving a few hundred
  thousand requests measurably slows a process here, so a table that seeds
  progressively inside one server varies heap age alongside whatever it
  meant to vary.
- **State a range, or say it was one run.** Repeated runs of one workload on
  this host have spanned 1.66x. Every figure recorded in this directory is a
  single run, and none has been re-verified against the identity check
  above.

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

**That same Git Bash also mangles `--path`'s own value, a separate problem
from the one above.** MSYS's automatic path conversion rewrites a
POSIX-looking argument such as `/users` into a Windows path before
`nova-bench-http` ever reads it, so `--path /users` typed exactly as shown
is refused with `--path must begin with /`, exit code 2 -- on a value that
visibly starts with `/`. This reproduces on the first attempt, not a
contrived one, and it fails loudly rather than reporting a wrong number: the
run does not start and no `RESULT` line prints. Prefix the invocation with
`MSYS_NO_PATHCONV=1` to pass the value through unchanged:

```bash
MSYS_NO_PATHCONV=1 ./target/release/nova-bench-http --addr 127.0.0.1:<port> --path /users --connections 200 --duration 30 --warmup 5
```

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
`errors`, `elapsed_ms`, `rps`, `conn_min` and `conn_max`. Of the settings
that shape a run, only `--connections` appears there; `--duration`,
`--warmup` and `--path` leave no trace in the line at all -- and a run's path
decides which of a routing target's handlers was measured, so it belongs in
the record. `errors=` does not stand in for it: a non-zero count catches a
path the target refuses, while a path the target does serve but that is not
the one the surrounding prose claims leaves no trace in the line at all. A
pasted `RESULT` line is therefore not self-describing, and whoever appends an
observation must write the invoking command's flags alongside it, as
`http-fixed-response.md` does.

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
figure; the Bun ratio is unmeasured. (Amended 2026-10-02: the Bun ratio
has been measured since 2026-09-11, by the procedure under "Comparing
against Bun" below; this paragraph's last clause describes only
this procedure's own scope.)

**The destination is given twice, and the second is unsatisfiable on this
tree.** `60-EXAMPLES.md` names `examples/05-json-api/BENCHMARK.md`, but
`examples/` holds `01-hello-world`, `02-fibonacci` and `03-producer-consumer`
-- no `05-json-api` directory exists to hold that file. `docs/benchmarks/` is
where the number goes instead.

**AMENDED 2026-09-10 (branch `examples-05-json-api`): both halves
of this paragraph are false now.** `examples/05-json-api` exists,
so `examples/` no longer holds only the three folders named above;
and `examples/05-json-api/BENCHMARK.md` exists and carries the gate's
own figure (recorded 2026-09-10 as 455.5 req/sec against `/users`, and
CORRECTED 2026-09-11 to a 1875-3109 req/sec range after that run was found
to have measured a debug-runtime build -- see the amendment at the head of
that file, and the procedure fix below), so the
destination `60-EXAMPLES.md` names is satisfied rather than unsatisfiable,
and the gate's number no longer defaults to `docs/benchmarks/` the way this
paragraph concluded. `docs/benchmarks/` keeps its own figure regardless:
the read-and-parse-path ceiling this document is about is a different
subject from the gate's number, not a stand-in for it, and remains the
destination `00-MASTER-SPEC.md` section 3 asks for. The branch that added
the example is what falsified this paragraph, not an edit to this file. The
same branch's most recent commit amended the identical stale claim in
`nova-spec/60-EXAMPLES.md` and other spec files but left this one untouched:
owning a file is not the same as fixing everything another task falsified
in it, and this file's amendment fell to whichever task next touched it.

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
  (`std/http/lib.nova:461` discloses this directly, alongside why: the
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

## Comparing against Bun

`nova-spec/60-EXAMPLES.md` section 5 asks for a throughput ratio against
Bun, not just the absolute figure this file's own procedure produces. This
section is that comparison's procedure; `examples/05-json-api/BENCHMARK.md`
is where a run's numbers get recorded, the same division of labour this
file already keeps with `http-fixed-response.md`.

### Build both sides

Steps 1–5 above still build and start the Nova side, with one change: name
the output with an explicit `.exe` extension rather than the bare stem
`-o` alone would leave. **This is observed behaviour, not a diagnosed
mechanism.** Spawning an extensionless Nova binary from Bun's `Bun.spawn`
failed here with `uv_spawn ENOENT`; naming a single `<stem>.exe` — that
stem, no extensionless sibling — spawned successfully, and no account on
record explains why. Nor is "extensionless paths never spawn" the rule
either: an extensionless copy of a *different* executable, with no `.exe`
sibling present, launched successfully in the same setup. What follows is
load-bearing regardless of mechanism: **the binary must remain one file
with that stem.** Two files sharing a stem, one of them stale, is the
arrangement behind this project's withdrawn benchmark figure (see the
amendment at the top of `examples/05-json-api/BENCHMARK.md`), independent
of which of the two carries the extension.

Start `docs/benchmarks/bun-server.js` with `bun docs/benchmarks/bun-server.js`
and read its port from the `listening on 127.0.0.1:<port>` line it prints —
the same shape step 5's Nova server prints.

### Equivalence gates the measurement, not the build

Before any throughput number is taken, run the equivalence check against
the Nova binary:

```bash
bun docs/benchmarks/bun-equivalence.js /path/to/json-api.exe
```

Run from the repository root: `bun-equivalence.js` spawns
`docs/benchmarks/bun-server.js` by that same relative path, so it only
finds it when launched from there.

It drives the nine exchanges `crates/nova-cli/tests/run_tests.rs` already
pins, against a fresh instance of each server, and compares status code and
response body bytes. Exit 0 and `EQUIVALENCE OK: all 9 exchanges match on
status and body bytes` is what licenses reading a throughput ratio between
the two servers as a ratio of the same workload rather than of two
different ones; it says nothing about whether either server builds, only
about whether, having built, the two answer alike. A failing equivalence
run means the throughput numbers below are not comparable, not that either
binary is broken.

**No automated test runs this.** CI's runners have no Bun installed, and an
`#[ignore]`d test that shells out to it would fail on every push rather
than occasionally — CI's Test job has an advisory step that runs exactly
the ignored tests with `continue-on-error: true`, so a deterministically
failing one would sit there unread rather than catching a real regression.
The equivalence check is run by hand, alongside the throughput comparison
it gates.

### Pin both sides to one core, and read the mask back

Nova's executor is single-threaded by ADR 0009
(`docs/adr/0009-async-execution-model.md`), so the fairer single-core
comparison pins Bun to one core too rather than leaving it free to use the
host's twelve. Spawn first, then set the process's affinity, then read the
property back before trusting it — a short-lived process can exit before
the set lands, and a pin that silently failed would look exactly like a
slow Bun. Recorded in `examples/05-json-api/BENCHMARK.md`: mask **1** when
a pin was applied, mask **4095** when it was not — an assertion that read
back 1 either way would prove nothing, and this one distinguishes the two.

Both sides were measured pinned and unpinned rather than pinning being
assumed fair: each side's pinned and unpinned ranges overlap, so the
fairness decision does not move which side of 1.0 the ratio lands on.
Pinned is reported as the headline because it is the single-core
comparison that matches Nova's own executor, not because it was the only
one measured.

**Amended 2026-10-02 (gate-remeasure):** on `main` at `5efcc2e`, the pinned
and unpinned ranges no longer overlap on either side:
- Nova: 8424.0–8520.1 pinned against 10250.0–10382.4 unpinned;
- Bun: 10774.3–12068.8 pinned against 12846.9–12956.6 unpinned.

The conclusion still holds, since both ratios, 0.70–0.79 and 0.79–0.81,
fall short of 1.0. But the premise above no longer does, and on the
absolute criterion the pinning decision now does matter: unpinned clears
10k, pinned does not. See `examples/05-json-api/BENCHMARK.md`, "AMENDMENT
2026-10-02 (gate-remeasure)".

**Amended 2026-10-02 (gate-remeasure-2):** on `main` at `cdaea7e`, the
pinned and unpinned ranges overlap again on both sides:
- Nova: 10986.5–12403.2 pinned against 10768.5–12126.4 unpinned;
- Bun: 9300.1–15374.9 pinned, which contains 12427.5–12605.6 unpinned.

But the pinning decision now moves the ratio. Unpinned it is 0.854–0.976,
below 1.0. Pinned it is 0.715–1.334 and straddles 1.0, because Bun's pinned
readings spread 1.65x. So under the pinned headline above, §5's ratio is
not established, and the verdict that it is not met rests on the unpinned
cells. On the absolute criterion the decision no longer matters: both clear
10k. See `examples/05-json-api/BENCHMARK.md`,
"AMENDMENT 2026-10-02 (gate-remeasure-2)".

**Amended 2026-10-02 (gate-remeasure-3):** on `main` at `012ca55`, the
pinned and unpinned ranges overlap on both sides again:
- Nova: 17682.5–18528.1 pinned against 17400.4–20014.4 unpinned;
- Bun: 18664.9–19963.2 pinned against 18604.7–19533.0 unpinned.

The pinning decision still moves the ratio, now the other way. Pinned it is
0.886–0.993, below 1.0 with the ranges disjoint. Unpinned it is 0.891–1.076
and straddles 1.0. So under the pinned headline above, §5's ratio is not
met. See `examples/05-json-api/BENCHMARK.md`, "AMENDMENT 2026-10-02 (gate-remeasure-3)".

**Amended 2026-10-03 (gate-remeasure-4):** on `main` at `7f2b85e`, over six
rounds per cell, the pinned and unpinned ranges overlap on both sides:
- Nova: 12010.9–14557.6 pinned against 13200.4–15506.7 unpinned;
- Bun: 10910.6–16971.5 pinned, which contains 12121.8–16064.0 unpinned.

Both ratios straddle 1.0: pinned 0.708–1.334, unpinned 0.822–1.279. Over
the first three rounds alone the pinned ratio was 1.010–1.269, wholly above
1.0; rounds 4–6 undid that. So under the pinned headline above, §5's ratio
is not established. See `examples/05-json-api/BENCHMARK.md`,
"AMENDMENT 2026-10-03 (gate-remeasure-4)".

**Amended 2026-10-03 (gate-remeasure-5):** on `main` at `1972b37`, the
first run judged under `docs/adr/0021-gate-ratio-paired-rounds.md`, pinned
decides and its verdict is inconclusive: Nova cleared the 1.0547 margin in
8 of 12 pinned rounds. Unpinned, it cleared it in 9 of 12. The whole-run
ranges overlap on both sides, and the host's speed-up in rounds 8–10
widened them:
- Nova: 13086.8–25031.1 pinned against 13463.0–24419.0 unpinned;
- Bun: 11434.4–22949.7 pinned, which contains 12482.5–22670.2 unpinned.

See `examples/05-json-api/BENCHMARK.md`, "AMENDMENT 2026-10-03 (gate-remeasure-5)".

**Amended 2026-10-03 (gate-remeasure-6):** on `main` at `b24379e`, the
two conditions disagreed. Pinned, which decides under `docs/adr/0021-gate-ratio-paired-rounds.md`,
Nova cleared the 1.0547 margin in 7 of 12 rounds, an inconclusive verdict;
its pinned round ratios scattered from 0.907 to 1.517. Unpinned it cleared
it in 12 of 12, at 1.067–1.468. In "(gate-remeasure-5)" the pinned ratios
were the tighter of the two, so pinning to core 0, which the load generator
may share, does not by itself account for it; neither run tests that. See
`examples/05-json-api/BENCHMARK.md`, "AMENDMENT 2026-10-03 (gate-remeasure-6)".

**Amended 2026-10-04 (gate-remeasure-7):** on `main` at `6fda78b`, the
two conditions agreed. Pinned, which decides under `docs/adr/0021-gate-ratio-paired-rounds.md`,
Nova cleared the 1.0547 margin in all 12 rounds, a met verdict; its
pinned round ratios ran from 1.061 to 1.261, a standard deviation of the
log ratio of 0.054 against "(gate-remeasure-6)"'s 0.146. Unpinned it
cleared it in 12 of 12, at 1.139–1.197. Why this run scattered so much
less is not measured. See `examples/05-json-api/BENCHMARK.md`, "AMENDMENT 2026-10-04 (gate-remeasure-7)".

### Judging the ratio: twelve paired rounds

**Added 2026-10-03.** `docs/adr/0021-gate-ratio-paired-rounds.md` decides how a
run's readings show §5's ratio:
- **Twelve rounds, fixed before the run.** A run is never extended.
- **Each round** takes one Nova and one Bun reading per pinning condition,
  back to back, each a fresh process.
- **The order alternates.** Odd rounds run Nova pinned, Bun pinned, Nova
  unpinned, Bun unpinned. Even rounds run Bun first in each pair.
- **A margin for wire framing.** A round counts for Nova only if Nova's
  req/sec is at least *m* times Bun's, where *m* is Bun's response bytes
  over Nova's, never below 1.0. That charges the 37-byte `Date` difference
  under "What each figure may be quoted for" below at its full share of
  the response. Each run measures both sizes again.
- **Pinned decides.** The ratio is met if Nova clears the margin in 10 or
  more of the 12 pinned rounds, not met if it falls short in 10 or more,
  and inconclusive otherwise.
- **Unpinned** is run and reported, and does not decide.

The range-extremes reading the amendments above use is no longer the
criterion. Under it, added readings only widen ranges, so within one run
an overlap can never resolve. Ranges and extremes are still recorded, as
description.

### The CPU-usage observation

Process affinity sets the default for every thread, but a thread can
override its own, so a silently-failed pin and a genuinely slow Bun would
look identical from throughput alone. CPU time consumed across a
35-second window (30s measurement plus 5s warmup, where one fully
saturated core is about 35 cpu-seconds) was checked directly with one
further run on each arm: pinned, **33.07** cpu-seconds (about 0.95 of a
core); unpinned, **34.98** cpu-seconds (about 1.0 core). So the pin took,
and separately, Bun was not exploiting the other eleven cores on this
workload even when free to. Read as an observation about Bun on this
workload, not a claim about Bun in general.

### The warmup finding

Checked once, not assumed: quadrupling the generator's warmup from 5s to
20s, one further pinned Bun run — mask read back as **1**, same as cell
C — at 20s warmup measured **12207.4** against
**12245.0** for the first 5s-warmup replicate of that same cell — well
inside Bun's own 1.57x spread between its two pinned replicates. The
standard 5s warmup stands for both sides. Recorded either way, because
"checked and it did not matter" and "not checked" are different things for
a later reader to inherit.

### What each figure may be quoted for

The pinned/pinned ratio may be quoted for "Nova against Bun, single core
against single core, on this host, this route, this payload." The
unpinned/unpinned ratio may be quoted for the same comparison with the OS
scheduler left free on both sides. Neither may be quoted as a general
multiple — a different host, route or payload shape is a different
measurement — and neither answers `00-MASTER-SPEC.md` section 3's separate
10k+ criterion, which this file's own figure above bears on instead. The
37-byte wire-framing difference, measured on this host rather than
assumed and recorded in `examples/05-json-api/BENCHMARK.md`, biases every
ratio here in Nova's favour, so none of them should be quoted as a
ceiling on how far short Nova
falls.

## Differential decomposition, 2026-09-29

`examples/05-json-api/BENCHMARK.md` records that nothing had measured
`read_request`'s parse, the socket write, the scheduler or the collector
separately, and calls the leftover unattributed to any named mechanism.
This series measures two of the mechanisms inside that remainder -- eager
header materialisation and body accumulation -- by varying one client-side
input at a time and reading serial throughput, and by timing the same head
parsing in isolation in a compiled harness.

**This is the home for these figures. Other records point here rather than
restating them.** A comparison whose size depends on which readings each
side draws from should have one home.

### Why a differential reads as serial cost here

ADR 0009 makes the runtime single-threaded, and `BENCHMARK.md` uses that to
establish that `1/rps` is a **serial per-request budget** rather than an
average over parallel workers. So varying one input and reading throughput
attributes real serial cost.

`docs/benchmarks/server.nova` is the instrument because its response wire
bytes are built once outside the accept loop -- its own header says this is
so a run against it times the read-and-parse path. Header count therefore
varies only `parse_request_head`'s materialisation loop, and
`Content-Length` varies only `read_request`'s body loop.

**`examples/05-json-api` cannot do this job and is not equated with it.**
Its response size depends on the store, its handler branches on method and
path, and its `POST /users` requires the body to parse into a record. It
appears below only as a separately labelled anchor.

### The commands

```bash
# A release nova, so a release runtime is what gets linked.
cargo build --release --locked --workspace

# Remove BOTH names before building, then assert exactly one file.
rm -f benchsrv benchsrv.exe && ls benchsrv*
./target/release/nova build docs/benchmarks/server.nova -o benchsrv
ls -l benchsrv*

rm -f profhead profhead.exe && ls profhead*
./target/release/nova build docs/benchmarks/profile-http-head.nova -o profhead
ls -l profhead*

# Header sweep: one fresh server process per point.
./benchsrv                      # prints its port
./target/release/nova-bench-http --addr 127.0.0.1:<port> \
  --connections 200 --duration 15 --warmup 5 \
  --header "x-pad-0: 0123456789abcdef" --header "x-pad-1: 0123456789abcdef"

# Body sweep: same shape.
./target/release/nova-bench-http --addr 127.0.0.1:<port> \
  --connections 200 --duration 15 --warmup 5 --body-bytes 16384

# The isolated side.
./profhead
```

**`MSYS_NO_PATHCONV=1` and `taskkill` interact, and getting it wrong hangs
the run with no diagnostic.** Git Bash needs `MSYS_NO_PATHCONV=1` so a
`--header` value's colon and a `--path` value survive intact. That same
setting stops MSYS rewriting `taskkill`'s flags, so with it exported the
form is `taskkill /F /IM benchsrv` and **without** it the form is
`taskkill //F //IM benchsrv`. Both were hit here: the doubled form under
the export is refused with `Invalid argument/option - '//F'`, and the
single form without it is silently path-converted and does nothing -- the
server then survives, a `wait` blocks on a process that never exits, and
the driver hangs forever. Any `nova build` must run **outside** the export,
because it needs the path conversion the export disables.

### Binary byte sizes, the identity check

| binary | bytes |
|---|---|
| `benchsrv` (`docs/benchmarks/server.nova`) | 608,256 |
| `profhead` (`docs/benchmarks/profile-http-head.nova`) | 510,464 |
| `jsonapi` (`examples/05-json-api/src/main.nova`) | 690,688 |

`jsonapi` at 690,688 matches what `BENCHMARK.md` records for the
post-fast-path build, so the anchor below measures the build that record
describes.

**What the byte size does and does not discriminate, with a counterexample
from this session.** It separates a release-runtime build from a
debug-runtime one, which is what it was introduced for. It does not track
source content: gutting the harness's FIRST timing loop left its binary at
510,464 bytes, identical to the unmutated build, while gutting its second
moved it to 482,304. So a matching byte size establishes the runtime
profile, not that the source is unchanged.

The harness's own `bytes=` output and the sweep's request lengths agree at
every point -- 36, 144, 279, 558, 1398 -- so the isolated and serial
instruments measure the same request shape rather than two shapes that
happen to share a header count.

### Bounds on each series

`Limits::default()` sets `max_head_bytes` 8192, `max_header_count` 100 and
`max_body_bytes` 1048576. Every point below is inside all three. Crossing
one is not silent: the server answers outside 2xx and the generator counts
every non-2xx as an error.

### A pass-state finding, and it is NOT the warmup finding above

**The first sweep pass taken after a build reads high, by up to 1.58x, and
more than its first point is affected.** Four of its five points are
elevated; the fifth, at 50 headers, reads 3009.1 against a settled range of
2883.7 to 3050.9 and is not. The first
pass here read 22896.6 req/sec at one header. Six consecutive repeats of
that same point, taken later with nothing varied, read **14604.4 --
14886.1, a spread of 1.019x**, and two further full passes agreed with
those six.

This is a different mechanism from "The warmup finding" above, which
concerns the generator's own `--warmup` flag. **Every run here already
carried the standard 5s warmup, and it did not cover this.** The remedy is
a discarded pass, not a longer `--warmup`.

It was caught by running a control that varies nothing. Without that
control, the first pass and the second would have been recorded as two
replicates whose disagreement was noise -- and the gap between them shrank
as header count rose, which would have biased any slope drawn through them.

**The discarded pass, recorded rather than deleted:** 22896.6, 17415.9,
13150.8, 7643.3, 3009.1 req/sec at 1, 5, 10, 20, 50 headers.

### Header sweep -- three settled passes

`errors=0` on every run below; `elapsed_ms` between 15019 and 15099. The
`addr`, `conn_min` and `conn_max` fields of each `RESULT` line are omitted
as incidental to the figure.

| headers | head bytes | pass A | pass B | pass C | microseconds per request |
|---|---|---|---|---|---|
| 1 | 36 | 14518.0 | 14844.0 | 14556.9 | **67.37 - 68.88** |
| 5 | 144 | 11434.5 | 11906.3 | 11575.1 | **83.99 - 87.45** |
| 10 | 279 | 8870.0 | 9239.4 | 8589.7 | **108.23 - 116.42** |
| 20 | 558 | 6049.0 | 6426.8 | 6248.1 | **155.60 - 165.32** |
| 50 | 1398 | 2973.9 | 3050.9 | 2883.7 | **327.77 - 346.78** |

Pass-to-pass agreement is 1.02x to 1.08x, comparable to the 1.019x the
control showed with nothing varied.

**Control, nothing varied** (one header, six fresh processes): 14886.1,
14832.1, 14747.7, 14639.2, 14604.4, 14723.5 req/sec -- 67.18 to 68.47
microseconds.

### Isolated materialisation -- the harness, two runs

`head_ns` minus `offsets_ns`. Both sides pay exactly one `parse_offsets`
call, so that call's array allocation cancels rather than landing in the
difference. **`parse_offsets` is not a zero-allocation baseline** -- the
intrinsic copies nothing and holds no Rust-side state, but the wrapper
returns a Nova array, so a call allocates one.

| headers | offsets_ns | head_ns | materialisation, ns |
|---|---|---|---|
| 1 | 199, 202 | 4639, 4840 | **4440 - 4638** |
| 5 | 457, 457 | 11577, 12118 | **11120 - 11661** |
| 10 | 486, 512 | 21797, 22615 | **21311 - 22103** |
| 20 | 663, 726 | 41110, 42342 | **40447 - 41616** |
| 50 | 1148, 1201 | 105850, 105220 | **104019 - 104702** |

Accumulators exact and identical across every run -- `acc_offsets` 240000,
560000, 960000, 1760000, 4160000 and `acc_head` 20000, 100000, 200000,
400000, 1000000 -- which is what shows each timing loop ran.

**One `parse_offsets` call costs 199 to 1201 nanoseconds** across this
range, which prices an argument no measurement had priced. `read_request`
makes more than one such call per request -- once inside
`parse_request_head` and again afterwards to re-derive `body_start`, plus
another per read when a head arrives across several -- and that second
call's own comment argues it "costs less than threading it out of
`parse_request_head`". At a one-header request that second call is about
200 ns; at fifty headers about 1.2 microseconds, against that request's
roughly 105 microseconds of head parsing. **Nothing here recommends
changing it**; the argument simply now has a number.

### The ~18 microsecond figure, measured

`docs/superpowers/specs/2026-09-01-std-http-request-parsing-design.md`
section 7 puts eager header materialisation at about 18 microseconds for a
**ten-header** request, computed as roughly 20 GC allocations at this
project's measured ~900 ns each;
`docs/adr/0019-offset-table-intrinsic-boundary.md` section 5 restates it
and names the escape hatch. (An earlier draft of this paragraph attributed
the figure to the ADR's own section 7. That ADR's section 7 is a different
subject -- three rulings and their reasons -- while section 5 is the one
carrying this figure, and the section 7 it cites is the design spec's. (A
first correction of this said "the ADR has no section 7", which is itself
false, inside the very paragraph warning about citing the wrong section of
the right file.) Corrected
before this file was committed, and recorded because a citation that names
the wrong section of the right file is the shape of error this project
keeps finding.)

**Measured isolated, ten headers: 21.3 to 22.1 microseconds.**

Two arithmetics bracket that rather than either matching it. The two
allocations per header the original figure assumes give 18. Reading
`parse_request_head`'s source suggested three or more per header -- a
`text_at` String, a `to_lower` String, a `text_at_utf8` String, and a
`Map::insert` -- which at ~900 ns gives 27 or more. **The measurement sits
between them, so neither allocation count is established by it.** A cost
consistent with ~900 ns per allocation is consistent with other mechanisms
of similar size regardless, so nothing here establishes allocation as the
cause.

**Amended 2026-10-04:** the source reading above counts each `String` as
one allocation. Until 2026-10-04 a runtime `String` was two, a 16-byte
header and a separate byte buffer, so that reading undercounted. Since one
allocation per string
(`docs/superpowers/specs/2026-10-04-one-allocation-strings-design.md`) it
is one. The figure's restatements (`CHANGELOG.md`, ADR 0019,
`examples/05-json-api/BENCHMARK.md`, this file's own "20 GC allocations
for ten headers' strings", `docs/benchmarks/server.nova`) are left as they
are; ADR 0019 already points readers here.

### Amplification, on matched populations

Nine extra headers, from the one-header shape to the ten-header shape, in
one session against one build.

| quantity | value |
|---|---|
| serial, whole-server | **39.35 - 49.05 microseconds** |
| isolated, harness | **16.67 - 17.66 microseconds** |
| amplification | **2.23x - 2.94x** |

Propagated at both endpoint pairings, not summarised by a middle.

That interval falls inside the 2.0x to 3.2x band `BENCHMARK.md` records.
**It does not confirm that band's own claim.** The band was inferred for
RESPONSE-side work on `examples/05-json-api`; this is header
materialisation on `docs/benchmarks/server.nova`. Two different mechanisms
on two different servers agreeing in magnitude is worth recording, and is
not the same as one verifying the other.

### Anchor -- `examples/05-json-api` empty store, re-derived

Four fresh-process readings, `GET /users`, store never seeded, `errors=0`
throughout: **10074.1, 10400.4, 10230.4, 10245.8 req/sec** -- **96.15 to
99.26 microseconds per request**.

**All four clear 10k req/sec and all four sit below the 100-microsecond
line.**

`BENCHMARK.md` records that same control, on a binary of the same byte
size, at 8688.1 and 9501.0 req/sec -- 105.3 to 115.1 microseconds -- and
that reading is what supports the standing claim that response-path work
alone cannot reach the gate, the control already exceeding the whole
budget. That record tells a later reader to re-derive the figure rather
than quote it forward, because it rests on a quantity moving by more than
its remaining margin. Re-derived, it moved across the line.

**Pooled across both sessions the control spans 8688.1 to 10400.4 req/sec,
a 1.20x spread straddling the criterion**, while the two sessions' ranges do NOT OVERLAP -- 8688.1 to 9501.0 against 10074.1 to 10400.4 -- so the movement is between sessions rather than within one. (Within-session spreads are 1.09x for the earlier pair and 1.03x for this session's four. An earlier draft of this paragraph cited only the 1.03x and called it "within-session spread", which generalised one session's figure to both and was contradicted by the 1.09x these same records already state.) **This
control can therefore neither support nor refute the claim built on it.**
That is not the same as the claim being refuted. And it is not a gate
figure: `05-json-api` serving an EMPTY store is not the gate's workload.

### Body sweep -- a loop no measurement had entered

No run on this project had ever sent a `Content-Length`, so
`read_request`'s `while body.len() < want` loop had always run zero times.
`errors=0` on every run; a discarded warm-up point read 14926.0 at body=0.

| body bytes | rep 1 | rep 2 | microseconds per request | over the 0-byte point | marginal ns/byte |
|---|---|---|---|---|---|
| 0 | 14783.7 | 15036.9 | **66.50 - 67.64** | -- | -- |
| 1024 | 12646.2 | 12578.9 | **79.08 - 79.50** | 11.44 - 13.00 | 11.2 - 12.7 |
| 4096 | 10774.2 | 10669.3 | **92.81 - 93.73** | 25.17 - 27.23 | 4.0 - 5.1 |
| 16384 | 8948.4 | 9334.3 | **107.13 - 111.75** | 39.49 - 45.25 | 1.0 - 1.6 |
| 65536 | 6216.3 | 6221.3 | **160.74 - 160.87** | 93.10 - 94.37 | 1.0 - 1.1 |
| 262144 | 2427.3 | 2387.1 | **411.98 - 418.92** | 344.34 - 352.42 | 1.3 - 1.3 |

"Marginal" is the cost of the bytes added since the previous row, divided
by that increment, propagated at both endpoint pairings.

**The sweep is decisively not flat**, which rules out the failure mode
worth naming: a missing `Content-Length` would leave `want` at 0, run the
loop zero times, and read as "body accumulation is free". It would also
desync keep-alive, since the body bytes would be parsed as the next
request, and `errors=0` rules that out independently.

**The shape is SUBLINEAR, not triangular.** Marginal cost falls from about
4-5 ns per byte to about 1, then sits between 1.0 and 1.3. The last
segment is slightly above the two before it, which is a mild upward trend
and not an establishment of a superlinear term.

**Two limits on what this series covers, stated rather than left to be
inferred:**

- **The triangular worst case is not exercised.** `read_request` asks for
  exactly `want - body.len()` bytes per read, so a peer delivering the
  remainder promptly costs one concatenation regardless of `want` -- and
  this generator writes the whole request with a single `write_all`, so it
  is that prompt peer. The triangular figure in `std/http/lib.nova`'s own
  comment describes a dribbling peer and is neither confirmed nor refuted
  here.
- **The small points may not enter the loop at all.** A 1024-byte body
  makes the whole request about 1060 bytes, which the first 4096-byte read
  can take entire, leaving `body.len()` already equal to `want`. That
  point's 11.4 to 13.0 microseconds is the fixed price of having a body --
  the `Content-Length` lookup, the `buf.slice`, the larger read -- not the
  loop's price. The points at 16384 and above cannot arrive in one
  4096-byte read.

### What this series does not measure

Of the mechanisms `BENCHMARK.md` names as never separately measured --
`read_request`'s parse, the socket write, the scheduler and the collector
-- **this series measures none of them individually.** It measures two
mechanisms that sit inside the same remainder. Header materialisation is
part of what `read_request` does, not the whole of it; the intrinsic parse
is priced here only as the `offsets_ns` baseline the subtraction removes.

**Nothing here totals the measured mechanisms against the 100-microsecond
budget or pronounces on the gate.** Two mechanisms of an undecomposed
remainder do not make a decomposition.

## The language server's latency (Phase 3.2)

The budget is 200 ms for the median and the maximum of 20 edits, each sent
while no check runs, to their diagnostics, and of 20 completions after
`self.users.` (ADR 0029). It is measured on
`examples/05-json-api/src/main.nova` with
`cargo test --release -p nova-cli --test lsp -- --ignored --nocapture latency`.

| Date | Run | Edit to diagnostics, median / max | Completion, median / max | Binary |
|---|---|---|---|---|
| 2026-10-08 | 1 | 17 / 18 ms | 13 / 14 ms | `target/release/nova.exe` on Windows 11, 14,534,144 bytes, built 2026-10-08 05:28 UTC |
| 2026-10-08 | 2 | 17 / 18 ms | 13 / 21 ms | the same binary |
| 2026-10-08 | 3 | 18 / 24 ms | 13 / 15 ms | the same binary |

Each figure includes the round trip through the protocol on one machine.
CI asserts 2 s for each, with the debug binary, on all three systems.
