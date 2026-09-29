# Differential decomposition of per-request cost — design

**Date:** 2026-09-29
**Branch:** `request-cost-differential`
**Base:** `main` == `origin/main` == `991fdfc`, 713 commits, 1130 passed /
0 failed / 8 ignored across 45 targets on merged `main`.

---

## 1. What this is, in one paragraph

Phase 2's gate asks `examples/05-json-api` to serve 10k+ req/sec. It is
measured at 2868.2 – 3392.4 req/sec at ten users and is short by roughly
2.9x to 3.5x. The response side of that cost has been decomposed twice and
optimised once. **The rest of it has never been decomposed at all.** This
increment measures two named mechanisms inside that undecomposed remainder
— eager header materialisation and body accumulation — by varying one
input at a time against a server built to hold everything else constant,
and by timing the same mechanisms in isolation in a compiled harness. It
produces figures and a reproducible procedure. **It changes no
`std/http` code, optimises nothing, and does not claim the gate is
reachable or unreachable.**

---

## 2. What is known, what is inferred, and what is neither

This section exists because the increment's whole value is replacing a
computed number with a measured one, and that is only meaningful if the
two are told apart first.

### 2.1 Measured, on this host, recorded with ranges

From `examples/05-json-api/BENCHMARK.md`, the "FURTHER AMENDMENT
2026-09-12" section:

- The absolute criterion is **2868.2 – 3392.4 req/sec** at ten users over
  six fresh-process readings.
- The Bun ratio is **0.230 – 0.272**.
- The empty-store control — a 2-byte body, where `users_json` loops zero
  times and `stringify` is never called — reads **9501.0 and 8688.1
  req/sec**, which is **105.3 – 115.1 microseconds per request**.
- The response side after the scalar fast path totals **105,999 – 106,335
  ns** per request at ten users.

The runtime is single-threaded by ADR 0009, and `BENCHMARK.md` uses that
to establish that `1/rps` is a **serial per-request budget** rather than an
average over parallel workers. Every differential figure in this increment
depends on that framing, and the spec restates it rather than assuming it.

### 2.2 Inferred — a product of two numbers, never measured together

`docs/adr/0019-offset-table-intrinsic-boundary.md` section 5,
`docs/superpowers/specs/2026-09-01-std-http-request-parsing-design.md`
section 7, and `CHANGELOG.md` all carry the same figure: eager header
materialisation costs **about 18 microseconds per request**, computed as
roughly 20 GC allocations for a ten-header request at this project's
measured ~900 ns per allocation.

**Neither factor was measured in this path.** `docs/benchmarks/README.md`
records that the benchmark nominated to confirm or refute the figure
"does neither", because the generator sends one header where the figure
assumes ten.

**And the allocation count looks low by inspection.** Reading
`parse_request_head` in `std/http/lib.nova`, each header costs
`text_at(...)` returning a `String`, then `.to_lower()` returning another,
then `text_at_utf8(...)` returning a third, then a `Map::insert`. That is
three-plus allocations per header against the two the arithmetic assumes.
**This is an observation about source, not a measurement**, and it is
written here as a prediction the experiment can falsify rather than as a
correction to the figure.

### 2.3 Neither measured nor inferred — the residual

`BENCHMARK.md`'s "Where the cost is" section states plainly: *"nothing
here measured `read_request`'s parse, the socket write, the scheduler or
the collector separately"*, and calls the leftover *"unattributed to any
named mechanism"*. The body accumulation loop in `read_request` has never
been entered by any measurement taken, because no run has ever sent a
`Content-Length`.

### 2.4 The claim this increment must not lean on

`BENCHMARK.md` says the empty-store control shows response-side work
cannot reach 10k+ alone — and then guards it: the fastest control reading
is within about 5% of the 100-microsecond line while the control's own two
readings differ by 1.09x, *a wider swing than that margin*, so a later
reader is told to re-derive rather than quote it forward.

**This increment re-derives it rather than quoting it.** Its own baseline
readings are taken fresh, in the same sessions as its sweeps.

---

## 3. Scope

### 3.1 In scope

1. `crates/nova-bench-http` gains `--header` (repeatable) and
   `--body-bytes`, each validated, each tested.
2. A compiled profiling harness lives in the repository under
   `docs/benchmarks/`, timing `parse_offsets` and `parse_request_head`
   directly.
3. A **header sweep** and a **body sweep**, each run against
   `docs/benchmarks/server.nova`, each with its own fresh-process
   baseline taken in the same session.
4. The record: a new dated section in `docs/benchmarks/README.md` carrying
   the procedure, and the figures written where the claims they bear on
   already live.

### 3.2 Explicitly out of scope, with reasons

- **Any change to `std/http`.** Lazy header materialisation is ADR 0019's
  named escape hatch and may well be the next increment. Implementing it
  here would mean optimising against the number this increment exists to
  measure.
- **Scheduler and collector accounting.** They cannot be reached by
  varying a client-side input, so they need in-runtime instrumentation.
  That is a separate increment, and it should be scoped by what this one
  finds: if materialisation turns out to be a third of the baseline, the
  next move is the escape hatch, not scheduler timers.
- **A `--method` flag.** The sweeps run against a server that ignores the
  method, and `read_request` enters its body loop on `Content-Length`
  alone regardless of method. Adding POST support would be speculative
  here. A later increment measuring `examples/05-json-api`'s POST route
  end-to-end should add it then.
- **Any claim about the gate's reachability.** The increment reports where
  time goes in two mechanisms. It does not total them against 100
  microseconds and pronounce.

---

## 4. The instrument, and why it is this server

Both sweeps run against `docs/benchmarks/server.nova`, not against
`examples/05-json-api`.

That server's own header records why it is shaped as it is: the response
wire bytes are built once outside the accept loop, *"so any number taken
against this server"* measures the read-and-parse path. It calls
`read_request(conn, Limits::default())` and then writes a constant,
pre-built response, ignoring the path entirely.

The consequence is the property a differential experiment needs:

| varied input | what changes in the server's work | what stays constant |
|---|---|---|
| header count | `parse_request_head`'s materialisation loop runs more iterations | response bytes, route logic, write size |
| `Content-Length` and body bytes | `read_request`'s `while body.len() < want` concat loop is entered | response bytes, route logic, write size |

**`examples/05-json-api` cannot do this job.** Its response size depends on
the store, its handler branches on method and path, and its `POST /users`
route requires the body to be valid JSON that parses into a record — so a
body sweep against it would vary parse cost, handler cost and response
cost together.

**The two servers are not interchangeable and the record must not equate
them.** `server.nova` writes a fixed 11-byte JSON body; the gate example's
empty-store control writes 2 bytes. Their baselines are expected to be
close and are not assumed equal. Every delta this increment reports is
computed **within `server.nova`'s own series**, and the gate example
appears only as a separately-labelled anchor.

---

## 5. Generator changes

### 5.1 `--header NAME:VALUE`, repeatable

Appends one header line to every request of the run. Given more than once,
every value is appended in the order given.

Validation, each with its own error message:

- The value must contain a `:`. Without one it is not a header line.
- The name (before the first `:`) must be non-empty and must contain no
  space, no CR and no LF.
- The value (after the first `:`) must contain no CR and no LF.

The CR/LF rule mirrors `--path`'s existing check, and for the same reason
its comment gives: a CR or LF would turn the rest of the value into forged
header lines on every request of the run, so the run would measure a
request shape other than the one its own record names, with nothing in the
`RESULT` line to show it.

`--header` does not deduplicate and does not lower-case. The server does
the lower-casing; the generator sends what it was given.

**Head-size ceiling is the operator's to respect, and the tool says so.**
`Limits::default()` in `std/http` sets `max_head_bytes` to 8192 and
`max_header_count` to 100. A sweep that crosses either gets a 4xx from the
server, and the generator counts every non-2xx as an error, so the failure
is loud rather than silent. The procedure records both numbers beside the
sweep so a reader knows what bounds the series.

### 5.2 `--body-bytes N`

Sends a body of exactly `N` bytes with a matching `Content-Length` header,
where `N` is a non-negative integer. `0` means no body and no
`Content-Length`, which is the current behaviour and the default.

The body is `N` bytes of a single repeated ASCII character. Its content is
irrelevant to what is being measured — `read_request` accumulates bytes
without inspecting them — and a constant byte keeps the request bytes
identical across every connection and every request of a run.

`--body-bytes` and an explicit `--header content-length:...` together are
rejected, because the two would disagree and the run would measure
framing confusion rather than the body loop.

### 5.3 What does not change

`request_bytes` remains one function building one byte vector reused by
every worker, built once before the workers start. No per-request
formatting is introduced; that would put allocation inside the measurement
loop on the generator's side.

---

## 6. The profiling harness

### 6.1 Why it exists at all

`BENCHMARK.md`'s compiled decomposition — the per-call `users_json`,
`to_bytes` and `json_response` figures — was produced by a harness that
lived outside the repository and no longer exists. **Those figures cannot
be reproduced from a checkout.** This increment does not recreate that
harness, but it does not repeat the mistake: its own harness is tracked.

### 6.2 What it measures

`docs/benchmarks/profile-http-head.nova`, built with `nova build` and run
as a native binary. For each header count in the swept series it times, in
separate loops over a fixed iteration count:

- `parse_offsets(buf)` — the thin `pub` wrapper over the
  `http_parse_request` intrinsic. The intrinsic copies nothing and holds
  no Rust-side state, but the wrapper's return type is `[Int]`, **so a
  call does allocate one Nova array.** It is not a zero-allocation
  baseline and the record must not describe it as one.
- `parse_request_head(buf, Limits::default())` — `parse_offsets` once,
  plus method and path extraction, plus the materialisation loop and its
  `Map`.

**The difference between the two is everything `parse_request_head` adds
over the intrinsic call, which is dominated by materialisation.** Both
sides pay exactly one `parse_offsets` call, so the array allocation
cancels in the subtraction rather than landing in it. That difference is
the quantity the ~18 microsecond figure names, measured directly for the
first time.

**And one thing the harness can settle that no record currently
remarks on: `read_request` calls `parse_offsets` more than once per
request.** Once inside `parse_request_head`, and again afterwards to
re-derive `body_start` — that second call's own comment argues it "costs
less than threading it out of `parse_request_head`", a comparison made
against no measurement. A head arriving across several reads pays another
call per read. Timing one call prices that argument, and the record states
what it found without recommending a change; changing it is not in this
increment's scope.

It reports ns per call for each, per header count, so a reader can compute
a per-header slope rather than trusting a single subtraction.

### 6.3 Timing

`std/time`'s `Instant` holds `nanos` from the runtime's monotonic
`time_now_nanos`, and `Instant`'s `nanos` field is readable — `std/time`'s
own comment records that no record field in this language is
privacy-enforced. So the harness reads `Instant::now().nanos` directly and
needs no new surface.

The clock is read once before a loop and once after, never per iteration,
so a clock read is amortised across the whole loop rather than added to
each call.

### 6.4 The harness's own honesty check

A loop whose result is discarded can in principle be removed by the
compiler, and a harness that measures nothing reports an impressively
small number. The harness accumulates something derived from each call's
result — a field of the returned table, summed — and prints the
accumulator with the timings. A reader who sees the accumulator can tell
the loop ran. Mutation 3 in section 8 is what establishes the check can
fail.

---

## 7. The series

Every run is **one fresh process per data point**, the discipline
`BENCHMARK.md` adopted after process age was measured to cost 1.29x to
1.38x after roughly 280k requests.

Every figure is recorded as a **range over replicates**, never a point.
Nine runs of one workload have spanned 1.66x on this host.

Every server binary's **byte size** is recorded beside its figures, the
identity check that caught the debug-runtime error.

### 7.1 Header sweep

Against `docs/benchmarks/server.nova`. Header counts spanning from the
current one-header shape up to a count that stays inside `max_head_bytes`
and `max_header_count`, with at least four points so a slope has more than
two readings to rest on, and at least two replicates per point.

The quantity of interest is the **slope**: microseconds of serial
per-request cost added per header. Multiplied by ten, that is directly
comparable to the ~18 microsecond figure, which is scoped to a ten-header
request.

### 7.2 Body sweep

Against the same server. Body sizes spanning from 0 up toward
`max_body_bytes`, chosen so that the `Bytes::concat` accumulation's
predicted shape — triangular in the read count — would be visible if it
is there, and so that at least one point sits where a single read can
deliver the whole body and at least one sits well above the 4096-byte read
size.

**This loop has never been entered by any measurement.** The sweep's first
job is to report what it costs; establishing or refuting the triangular
shape is a second, and the spec does not promise the chosen sizes will
settle it.

### 7.3 Anchors, so the sweeps can be placed against what is already recorded

In the same session as each sweep, and labelled as separate populations:

- `server.nova` at the sweep's own baseline point (one header, no body).
- `examples/05-json-api`'s empty-store control, which re-derives the
  105.3 – 115.1 microsecond figure section 2.4 declines to quote forward.

---

## 8. Testing

### 8.1 Unit tests, in `crates/nova-bench-http`

Argument parsing gains tests mirroring the shape of the existing `--path`
tests: a valid header accepted; a header with no colon rejected; an empty
name rejected; CR and LF rejected in name and in value; repetition
accumulating in order; `--body-bytes` accepting `0` and a positive
integer; a negative or non-numeric value rejected; `--body-bytes` together
with an explicit content-length header rejected.

A test asserts the **bytes `request_bytes` builds**, not merely that the
flags parsed: given a header and a body size, the produced request must
carry that header line, a matching `Content-Length`, and a body of that
length. Asserting the wire bytes is what makes the flags' effect
observable; asserting the parsed config would not.

### 8.2 The existing smoke test

`bench_http_server_and_generator_agree` in
`crates/nova-cli/tests/run_tests.rs` stays a normal, non-ignored test and
keeps asserting no throughput number, so it cannot flake on timing. It
gains no new assertions about the new flags; section 8.1's wire-bytes test
covers them without starting a server.

### 8.3 Mutations to run and report — not to predict

1. **`--header` parsed and stored but never written into
   `request_bytes`.** Section 8.1's wire-bytes test must fail. If it
   passes, that test is not observing the wire.
2. **`--body-bytes` writes the body but omits `Content-Length`.** The
   wire-bytes test must fail. This one matters beyond the test: without
   `Content-Length`, `read_request`'s `want` is 0 and the body loop runs
   zero iterations, so the body sweep would be flat and would read as
   "body accumulation is free" rather than as "the body was never sent".
3. **The harness's timing loop made to skip its calls.** The accumulator
   printed alongside the timings must change. If it does not, the
   accumulator is not observing the loop and section 6.4's check is
   decorative.

Each mutation's actual result is recorded, including any that does not
behave as this section expects.

---

## 9. Records

- **`docs/benchmarks/README.md`** — a new dated section carrying the
  procedure for both sweeps: the exact commands, the build requirement
  (`cargo build --release` first, because `find_runtime_lib` resolves the
  runtime staticlib next to the `nova` executable), the binary byte sizes,
  and the `max_head_bytes` / `max_header_count` bounds. The existing
  "Header materialisation runs at its one-header minimum" bullet gets a
  pointer to the new section rather than a rewrite.
- **`docs/adr/0019-offset-table-intrinsic-boundary.md`** — its section 5
  carries the ~18 microsecond figure (an earlier draft said section 7,
  which is that ADR's rulings section). It gains a dated pointer to the
  measurement. **The inferred figure is not edited at its own site**, in
  keeping with this project's practice of amending rather than silently
  rewriting.
- **`examples/05-json-api/BENCHMARK.md`** — a dated section recording what
  the sweeps found about the residual its "Where the cost is" section
  names, and explicitly which of the four mechanisms it named remain
  unmeasured.
- **`CHANGELOG.md`** under `[Unreleased]`.

**The figure is written in one home and pointed at from the others.** PR
#53 found the same claim restated across seven satellite records and wrong
in all seven at once; the structural half of that fix is not to repeat it.

---

## 10. What the results can and cannot support

Written before the numbers exist, so it cannot be shaped by them.

**A differential slope measures serial per-request cost added by one more
header, on this host, against this server, at this header-count range.**
It does not measure the allocation count, and it does not establish that
allocation is the mechanism — a slope consistent with roughly 900 ns per
allocation is consistent with other mechanisms of similar size.

**The harness's subtraction measures materialisation in isolation.** The
relationship between an isolated per-call cost and a whole-server serial
cost is exactly the amplification question `BENCHMARK.md` reports as
neither confirmed nor refuted: medians gave 1.6x while the ranges admitted
-0.4x to 3.8x. This increment measures both sides in one session on
matched populations, which is the condition that comparison lacked. **It
may still fail to settle it, and that outcome is a result to report rather
than a shortfall to hide.**

**Ranges are propagated, not summarised.** Any comparison this increment
states is computed at both endpoint pairings as well as at the middles,
and if those straddle a claim, the finding is that the data does not
settle it. Reporting a median as a location, when the spread admits an
interval containing the rival claim, is the error PR #53 withdrew a draft
for.

**Nothing here totals the measured mechanisms against the 100-microsecond
budget and pronounces on the gate.** Two mechanisms of an undecomposed
remainder do not make a decomposition.

---

## 11. Hazards

- **The known Windows async flake**, roughly one run in four historically,
  whose shape is broader than the `0xc0000005` it is often described by —
  a 2026-08-29 instance carried no crash code at all, just an async child
  exiting non-zero with empty stdout. The smoke test is the exposure.
  Re-run, say so, attribute no cause, fix nothing, and do not grep for
  that code as the test of whether it fired.
- **`nova build -o NAME` writes exactly `NAME`, with no `.exe`.** Delete
  every sibling name before building and assert exactly one file was
  written, rather than trusting the path asked for. A stale file at the
  path you asked for succeeds and lies; that is how a debug-runtime build
  reached a published figure.
- **A release `nova` must build the server**, because `find_runtime_lib`
  resolves the runtime staticlib beside the executable and nothing pins
  the profile. Record the binary's byte size beside every figure.
- **Do not author Nova string escapes or markdown backslashes through a
  shell heredoc.** A quoted heredoc has eaten a backslash level repeatedly
  on this project, once putting real CR bytes into a tracked file. Use the
  editor tool, or a script that asserts its match count.
- **A grep miss is not evidence of absence**, and grep is line-oriented: a
  stale figure survived a sweep on this project by being line-wrapped.
  Sweep for the figures this increment bears on with a whitespace-tolerant
  pattern, and build the file roster as a **set difference** — every
  tracked file naming the token, minus every file this branch touched —
  over the **union** of every affected token, not one token's grep.

---

## 12. Success criteria

1. `--header` and `--body-bytes` exist, are validated, and their effect on
   the wire bytes is asserted by a test.
2. A tracked, compiled harness reports `parse_offsets` and
   `parse_request_head` timings across a header sweep, with an accumulator
   showing its loop ran.
3. Both sweeps are run, with fresh processes per point, replicates,
   ranges, and binary byte sizes recorded.
4. The ~18 microsecond figure is confirmed, refuted, or reported as not
   settled by these data — any of the three is a result.
5. The body accumulation loop has been entered by a measurement, and what
   it cost is recorded.
6. Records name what remains unmeasured as precisely as what was measured.
7. Suite unmoved, `cargo fmt --all -- --check` clean, and
   `cargo clippy --locked --all-targets --all-features -- -D warnings`
   clean, which is CI's actual gate.
8. **No claim that Phase 2's gate is met, reachable, or unreachable.**

---

## 13. Global constraints for implementation

- `cargo build --locked --workspace` **before** `cargo test`;
  `--no-fail-fast` on the test run; never pipe cargo output through
  `head`/`tail` before summing — sum **every** `test result:` line;
  baseline is 1130 passed / 0 failed / 8 ignored across 45 targets.
- No `reason = "..."` in any lint attribute; MSRV is 1.78.
- The ignored ADR-0010 GC tests stay ignored and untouched.
- The poll ABI is frozen and no panic may cross a generated poll boundary.
- Every fixture path unique per process.
- `crates/nova-bench-http`'s `[[bin]]` keeps `bench = false`, or the
  Benchmarks workflow breaks.
- Commit messages are written to a UTF-8 file and applied with
  `git commit -F`, never a heredoc, each body ending exactly
  `Co-Authored-By: Claude Opus 5 <noreply@anthropic.com>`.
- Cite no SHA that is not already an ancestor of `main`.
- Byte-scan every file written via `git show :<path>` — staged content,
  not the working tree, because `core.autocrlf` smudges it: valid UTF-8,
  no byte below 0x20 outside tab, CR and LF, no 0x7f, and zero
  backslash-u-four-hex sequences in tracked markdown. Write code points as
  U+XXXX.
- Never `git add -A` or `git add .`.
