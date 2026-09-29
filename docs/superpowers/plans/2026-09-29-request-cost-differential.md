# Differential decomposition of per-request cost — implementation plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use
> superpowers:subagent-driven-development (recommended) or
> superpowers:executing-plans to implement this plan task-by-task. Steps
> use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Measure what eager header materialisation and body accumulation
actually cost per request, by varying one client-side input at a time
against a server that holds everything else constant, and by timing the
same code in isolation in a compiled harness.

**Architecture:** Two new generator flags let a run send extra headers and
a body. Both sweeps target `docs/benchmarks/server.nova`, whose response
bytes are hoisted outside the accept loop, so header count varies only
`parse_request_head`'s materialisation loop and `Content-Length` varies
only `read_request`'s body loop. A tracked Nova harness times
`parse_offsets` against `parse_request_head` directly; their difference is
materialisation in isolation. Records are amended, never silently
rewritten, and the figure lives in one home with pointers from the rest.

**Tech Stack:** Rust (`crates/nova-bench-http`, no new dependency), Nova
(`docs/benchmarks/`), Markdown records.

**Spec:** `docs/superpowers/specs/2026-09-29-request-cost-differential-design.md`

## Global Constraints

Every task's requirements implicitly include this section.

- `cargo build --locked --workspace` **before** `cargo test`. Always.
- `cargo test --workspace --no-fail-fast`. Never pipe cargo output through
  `head` or `tail` before summing — there are 45 targets and you must sum
  **every** `test result:` line. Baseline: **1130 passed / 0 failed / 8
  ignored**.
- `cargo fmt --all -- --check` must pass. Note `cargo fmt --all` writes LF
  into this CRLF working copy; check `git diff --numstat` after running it.
- `cargo clippy --locked --all-targets --all-features -- -D warnings` must
  pass. That is CI's actual gate.
- No `reason = "..."` in any lint attribute. MSRV is 1.78.
- The ignored ADR-0010 GC tests stay ignored and untouched.
- The poll ABI is frozen; no panic may cross a generated poll boundary.
- Every fixture path must be unique per process.
- `crates/nova-bench-http`'s `[[bin]]` keeps `bench = false`, or the
  Benchmarks workflow breaks.
- Commit messages are written to a UTF-8 file and applied with
  `git commit -F`. **Never a heredoc.** Each body ends exactly
  `Co-Authored-By: Claude Opus 5 <noreply@anthropic.com>`.
- Cite no SHA that is not already an ancestor of `main`. `991fdfc` is; the
  spec commit on this branch is **branch-local and must not appear in any
  tracked file**.
- Byte-scan every file written, via `git show :<path>` — **staged content,
  not the working tree**, because `core.autocrlf` smudges it. Require:
  valid UTF-8; no byte below 0x20 outside tab, CR and LF; no 0x7f; zero
  backslash-u-four-hex sequences in tracked markdown (write code points as
  U+XXXX). Run planted positives first so the scanner is trusted only
  after it has been seen to fire.
- Never `git add -A` or `git add .`. Stage named paths.
- Do not write scratch files into the repository.
- **Do not author Nova string escapes or markdown backslashes through a
  shell heredoc.** A quoted heredoc has eaten a backslash level repeatedly
  on this project, once putting real CR bytes into a tracked file. This
  plan is full of CRLF literals, so the hazard is live in nearly every
  task. Use the editor tool, or a script that asserts its match count.

### Sentence-shape discipline, binding on every comment, doc and record

- Prefer a roster with no count. A corrected number is usually the wrong
  fix.
- No ordinals or closed worlds over std, the runtime, the workspace or the
  record set.
- Never claim a test is "the only" thing that catches something.
- Never write that a fixture pins something without checking that a
  fixture actually executes it.
- grep is line-oriented, so a miss is not evidence of absence. Sweep prose
  with whitespace-tolerant patterns that also normalise `//`, `///` and
  `>` gutters, and build any file roster as a **set difference over the
  union of every affected token**:
  `comm -23 <(git grep -l TOKEN | sort) <(git diff --name-only main..HEAD | sort)`.

### What results may and may not support

Binding on every record this plan produces.

- **Ranges are propagated, never summarised.** Compute every comparison at
  both endpoint pairings as well as the middles. If those straddle a
  claim, the finding is that the data does not settle it. Reporting a
  median as a location when the spread admits an interval containing the
  rival claim is the error PR #53 withdrew a draft for.
- A slope consistent with roughly 900 ns per allocation is consistent with
  **other mechanisms of similar size**, so a slope does not establish
  allocation as the cause.
- Two mechanisms of an undecomposed remainder do not make a decomposition.
  **Nothing totals them against the 100-microsecond budget and pronounces
  on the gate.**
- The ~18 microsecond figure being confirmed, refuted, **or reported as
  not settled by these data** are all three results.

## Review Focus

Input classes the spec implies that no task's happy path exercises, most
likely to bite first. Each one's test is added to the task that owns the
code.

1. **`--body-bytes` with an enormous value** allocates that many bytes in
   the generator before any server sees it. Bounded at `max_body_bytes`
   and rejected above it — Task 2.
2. **`--header` with a space in the name** produces a malformed header
   line on every request of the run, which the target answers outside 2xx,
   so the run measures a rejection rather than the shape its record names.
   Rejected — Task 1.
3. **`--header` with an empty name** (a leading colon) is not a header
   line at all. Rejected — Task 1.
4. **`--header` whose value contains further colons** is ordinary HTTP and
   must be accepted, split at the first colon only — Task 1.
5. **`--header` with an empty value** is valid HTTP and must be accepted;
   rejecting it would be stricter than the protocol — Task 1.

---

## File Structure

| file | responsibility | task |
|---|---|---|
| `crates/nova-bench-http/src/main.rs` | `Config` gains `headers` and `body_bytes`; `request_bytes` becomes a function of the whole config; validation and its unit tests | 1, 2 |
| `docs/benchmarks/profile-http-head.nova` | **new** — times `parse_offsets` against `parse_request_head` across a header sweep, in a compiled binary | 3 |
| `docs/benchmarks/README.md` | the procedure for both sweeps, and a pointer from the existing one-header-minimum bullet | 6 |
| `docs/adr/0019-offset-table-intrinsic-boundary.md` | a dated pointer beside the inferred figure, which is **not** edited at its own site | 6 |
| `examples/05-json-api/BENCHMARK.md` | a dated section on what the sweeps found about the residual, and which named mechanisms remain unmeasured | 6 |
| `CHANGELOG.md` | `[Unreleased]` entry | 6 |

Unchanged on purpose: `std/http/lib.nova` (no optimisation in this
increment), `docs/benchmarks/server.nova` (it is the instrument), and
`crates/nova-cli/tests/run_tests.rs`'s
`bench_http_server_and_generator_agree`, which stays a normal
non-ignored test asserting no throughput number so it cannot flake on
timing.

---

## Task 1: `--header NAME:VALUE`, repeatable

**Files:**
- Modify: `crates/nova-bench-http/src/main.rs`

**Interfaces:**
- Consumes: nothing from earlier tasks.
- Produces: `Config { addr: String, path: String, headers: Vec<String>,
  connections: usize, duration: Duration, warmup: Duration, self_test:
  bool }` and `fn request_bytes(cfg: &Config) -> Vec<u8>`. Task 2 adds one
  field to that struct and one block to that function; it changes neither
  signature.

**Design notes an implementer needs:**

The header line is emitted **verbatim**, exactly as the operator typed it.
The colon is located only to validate the name. This avoids inventing a
normalisation rule: if the operator writes `x-a:1` the wire carries
`x-a:1`, and if they write `x-a: 1` the wire carries `x-a: 1`. The server
lower-cases names; the generator sends what it was given.

`request_bytes` changes from taking `&str` to taking `&Config`. That is
deliberate: the wire-bytes tests then run the whole path from flags to
bytes, which is what makes a flag's effect observable. Asserting a parsed
`Config` field would not.

`from_args` builds its `take` closure inside the loop body, borrowing the
argument iterator mutably. Pushing to a separate `Vec<String>` from a
match arm is fine; do not hold `take` across the push.

- [ ] **Step 1: Write the failing tests**

Add to the `#[cfg(test)]` module in `crates/nova-bench-http/src/main.rs`:

```rust
    #[test]
    fn headers_are_validated_and_sent_verbatim() {
        let args = |a: &[&str]| Config::from_args(a.iter().map(|s| s.to_string()));

        let none = args(&["--self-test"]).expect("--self-test alone is valid");
        assert!(
            none.headers.is_empty(),
            "no --header means no extra header, which is what keeps every \
             already-recorded figure describing the run it was taken from"
        );

        let one = args(&["--self-test", "--header", "x-a:1"]).expect("one header is valid");
        assert_eq!(one.headers, vec!["x-a:1".to_string()]);

        let two = args(&["--self-test", "--header", "x-a:1", "--header", "x-b:2"])
            .expect("two headers are valid");
        assert_eq!(
            two.headers,
            vec!["x-a:1".to_string(), "x-b:2".to_string()],
            "repetition accumulates in the order given, because a sweep's \
             record names the request it sent"
        );

        assert!(
            args(&["--self-test", "--header", "x-a"]).is_err(),
            "without a colon it is not a header line"
        );
        assert!(
            args(&["--self-test", "--header", ":1"]).is_err(),
            "an empty name is not a header line"
        );
        assert!(
            args(&["--self-test", "--header", "x a:1"]).is_err(),
            "a space in the name is a malformed header line on every request \
             of the run, which the target answers outside 2xx"
        );
        // Both bytes, both halves, all separately: one check covering the
        // whole string would satisfy a test of the pair alone, and that test
        // could not tell which half is doing the work.
        assert!(
            args(&["--self-test", "--header", "x\ra:1"]).is_err(),
            "a CR in the name would forge header lines into every request"
        );
        assert!(
            args(&["--self-test", "--header", "x\na:1"]).is_err(),
            "an LF in the name would forge header lines into every request"
        );
        assert!(
            args(&["--self-test", "--header", "x-a:1\r2"]).is_err(),
            "a CR in the value would forge header lines into every request"
        );
        assert!(
            args(&["--self-test", "--header", "x-a:1\n2"]).is_err(),
            "an LF in the value would forge header lines into every request"
        );

        assert_eq!(
            args(&["--self-test", "--header", "x-a:b:c"])
                .expect("further colons belong to the value")
                .headers,
            vec!["x-a:b:c".to_string()],
            "the split is at the FIRST colon; a value carrying colons is \
             ordinary HTTP"
        );
        assert_eq!(
            args(&["--self-test", "--header", "x-a:"])
                .expect("an empty value is valid HTTP")
                .headers,
            vec!["x-a:".to_string()],
            "rejecting an empty field value would be stricter than the protocol"
        );

        assert!(args(&["--self-test", "--header"]).is_err());
    }

    #[test]
    fn the_default_request_is_unchanged_and_headers_reach_the_wire() {
        let args = |a: &[&str]| Config::from_args(a.iter().map(|s| s.to_string()));

        let dflt = args(&["--self-test"]).expect("--self-test alone is valid");
        assert_eq!(
            request_bytes(&dflt),
            b"GET / HTTP/1.1\r\nHost: nova-bench\r\n\r\n".to_vec(),
            "the default is still the 36 bytes docs/benchmarks/README.md \
             describes, so every figure recorded before these flags existed \
             still describes the request it was taken with"
        );
        assert_eq!(request_bytes(&dflt).len(), 36);

        let with = args(&["--self-test", "--header", "x-a:1", "--header", "x-b:2"])
            .expect("two headers are valid");
        assert_eq!(
            request_bytes(&with),
            b"GET / HTTP/1.1\r\nHost: nova-bench\r\nx-a:1\r\nx-b:2\r\n\r\n".to_vec(),
            "asserting the WIRE bytes, not the parsed config: a flag stored \
             but never written would pass a config assertion"
        );
    }
```

Update the existing `the_request_line_carries_the_configured_path` test,
whose two calls now need a `Config`:

```rust
    #[test]
    fn the_request_line_carries_the_configured_path() {
        let args = |a: &[&str]| Config::from_args(a.iter().map(|s| s.to_string()));
        let users = args(&["--self-test", "--path", "/users"]).expect("/users is valid");
        assert_eq!(
            request_bytes(&users),
            b"GET /users HTTP/1.1\r\nHost: nova-bench\r\n\r\n".to_vec()
        );
        let root = args(&["--self-test"]).expect("--self-test alone is valid");
        assert_eq!(
            request_bytes(&root),
            b"GET / HTTP/1.1\r\nHost: nova-bench\r\n\r\n".to_vec(),
            "the default path's request is the 36 bytes docs/benchmarks/README.md describes"
        );
    }
```

- [ ] **Step 2: Run the tests to verify they fail**

```bash
cargo build --locked --workspace && cargo test -p nova-bench-http --no-fail-fast
```

Expected: FAIL — `Config` has no field `headers`, and `request_bytes`
takes `&str` rather than `&Config`.

- [ ] **Step 3: Add the field, the validator, the parsing and the wire change**

Add the field to `Config` (`crates/nova-bench-http/src/main.rs:145`):

```rust
struct Config {
    addr: String,
    path: String,
    /// Extra header lines, each emitted verbatim after `Host`. Empty by
    /// default, which is what keeps the default request byte-identical to
    /// the one every already-recorded figure was taken with.
    headers: Vec<String>,
    connections: usize,
    duration: Duration,
    warmup: Duration,
    self_test: bool,
}
```

Add the validator above `impl Config`:

```rust
/// Check one `--header` value, which is emitted verbatim if it passes.
///
/// The colon is located only to validate the name; nothing is rewritten.
/// A CR or LF anywhere would terminate the header line and turn the rest
/// of the value into forged header lines, on every request of the run --
/// so the run would measure a request shape other than the one its own
/// record names, with nothing in the `RESULT` line to show it. Same
/// reason `--path` carries the same check.
fn validate_header(h: &str) -> Result<(), String> {
    if h.contains('\r') {
        return Err("--header must contain no CR".to_string());
    }
    if h.contains('\n') {
        return Err("--header must contain no LF".to_string());
    }
    let colon = match h.find(':') {
        Some(i) => i,
        None => return Err("--header must be NAME:VALUE".to_string()),
    };
    let name = &h[..colon];
    if name.is_empty() {
        return Err("--header name must not be empty".to_string());
    }
    if name.contains(' ') {
        return Err("--header name must contain no space".to_string());
    }
    Ok(())
}
```

In `from_args`, declare the accumulator beside the other defaults:

```rust
        let mut headers: Vec<String> = Vec::new();
```

Add the match arm beside `"--path"`:

```rust
                "--header" => {
                    let h = take("--header")?;
                    validate_header(&h)?;
                    headers.push(h);
                }
```

Add `headers,` to the `Ok(Config { ... })` literal.

Replace `request_bytes` (`crates/nova-bench-http/src/main.rs:40`), keeping
its existing doc comment and appending the new paragraph:

```rust
/// ... (keep the existing doc comment verbatim) ...
///
/// Extra headers are appended after `Host`, each verbatim. With no
/// `--header` the bytes are unchanged, which is deliberate: every figure
/// recorded before this flag existed describes the 36-byte request, and a
/// default that altered it would silently redescribe all of them.
fn request_bytes(cfg: &Config) -> Vec<u8> {
    let mut s = format!("GET {} HTTP/1.1\r\nHost: nova-bench\r\n", cfg.path);
    for h in &cfg.headers {
        s.push_str(h);
        s.push_str("\r\n");
    }
    s.push_str("\r\n");
    s.into_bytes()
}
```

Update the one call site (`crates/nova-bench-http/src/main.rs:439`):

```rust
    let request = request_bytes(&cfg);
```

- [ ] **Step 4: Run the tests to verify they pass**

```bash
cargo build --locked --workspace && cargo test -p nova-bench-http --no-fail-fast
```

Expected: PASS.

- [ ] **Step 5: Run mutation 1 and record the actual result**

Temporarily make the match arm accept and store the header without
`request_bytes` ever emitting it — comment out the `for h in &cfg.headers`
loop body. Run:

```bash
cargo test -p nova-bench-http --no-fail-fast
```

`the_default_request_is_unchanged_and_headers_reach_the_wire` **must
fail**. If it passes, that test is not observing the wire and must be
fixed before proceeding. Restore the loop and re-run to confirm green.
**Record what actually happened, including a result this step did not
expect.**

- [ ] **Step 6: Full gate**

```bash
cargo build --locked --workspace && cargo test --workspace --no-fail-fast
```

Sum **every** `test result:` line — 45 targets. Expect 1130 passed / 0
failed / 8 ignored; the new tests raise the passed count and the report
must state the new total rather than restating the baseline.

```bash
cargo fmt --all -- --check && cargo clippy --locked --all-targets --all-features -- -D warnings
```

- [ ] **Step 7: Byte-scan staged content and commit**

```bash
git add crates/nova-bench-http/src/main.rs
```

Scan the staged blob via `git show :crates/nova-bench-http/src/main.rs`,
with planted positives first. Then write the message to a UTF-8 file and:

```bash
git commit -F <message-file>
```

---

## Task 2: `--body-bytes N`

**Files:**
- Modify: `crates/nova-bench-http/src/main.rs`

**Interfaces:**
- Consumes: `Config` and `fn request_bytes(cfg: &Config) -> Vec<u8>` from
  Task 1.
- Produces: `Config` additionally carrying `body_bytes: usize`. No
  signature changes.

**Design notes an implementer needs:**

`--body-bytes 0` is the default and means no body **and no
`Content-Length`**, so the default request stays byte-identical.

The body is `N` bytes of one repeated ASCII character. Content is
irrelevant — `read_request` accumulates bytes without inspecting them —
and a constant byte keeps every request of a run identical.

**An explicit `content-length` header is rejected outright, whether or not
`--body-bytes` is given.** The spec names the both-given case; this is
deliberately stricter, because `--header content-length:5` *without* a
body makes the server wait for five bytes that never arrive, and the
run becomes ten-second timeouts rather than a measurement. The spec's
silence on that case is not permission for it to hang.

`--body-bytes` is bounded at `max_body_bytes`. Above it the server
answers with `BodyTooLarge` and the generator counts every non-2xx as an
error, so nothing measurable lies up there — and an unbounded value would
have the generator allocate it first.

- [ ] **Step 1: Write the failing tests**

```rust
    #[test]
    fn body_bytes_is_validated_and_framed() {
        let args = |a: &[&str]| Config::from_args(a.iter().map(|s| s.to_string()));

        assert_eq!(
            args(&["--self-test"]).expect("valid").body_bytes,
            0,
            "no body by default, which keeps the default request the 36 bytes \
             every already-recorded figure was taken with"
        );
        assert_eq!(args(&["--self-test", "--body-bytes", "0"]).expect("0 is valid").body_bytes, 0);
        assert_eq!(args(&["--self-test", "--body-bytes", "7"]).expect("7 is valid").body_bytes, 7);

        assert!(
            args(&["--self-test", "--body-bytes", "-1"]).is_err(),
            "a negative body is not a body"
        );
        assert!(args(&["--self-test", "--body-bytes", "x"]).is_err());
        assert!(args(&["--self-test", "--body-bytes"]).is_err());

        assert!(
            args(&["--self-test", "--body-bytes", "1048577"]).is_err(),
            "above std/http's max_body_bytes the server answers BodyTooLarge, \
             so nothing measurable lies up there -- and an unbounded value \
             would be allocated here first"
        );
        assert!(
            args(&["--self-test", "--body-bytes", "1048576"]).is_ok(),
            "the bound is inclusive, so a sweep can reach it"
        );

        assert!(
            args(&["--self-test", "--body-bytes", "4", "--header", "content-length:4"]).is_err(),
            "two framings disagree even when they agree, and the run would \
             measure framing confusion rather than the body loop"
        );
        assert!(
            args(&["--self-test", "--header", "Content-Length:5"]).is_err(),
            "a content-length with no body makes the server wait for bytes \
             that never arrive, so the run becomes timeouts; the name is \
             matched without regard to case"
        );
    }

    #[test]
    fn a_body_reaches_the_wire_with_matching_framing() {
        let args = |a: &[&str]| Config::from_args(a.iter().map(|s| s.to_string()));

        let none = args(&["--self-test", "--body-bytes", "0"]).expect("valid");
        assert_eq!(
            request_bytes(&none),
            b"GET / HTTP/1.1\r\nHost: nova-bench\r\n\r\n".to_vec(),
            "zero means no Content-Length at all, not a zero-valued one"
        );

        let some = args(&["--self-test", "--body-bytes", "4"]).expect("valid");
        assert_eq!(
            request_bytes(&some),
            b"GET / HTTP/1.1\r\nHost: nova-bench\r\nContent-Length: 4\r\n\r\nxxxx".to_vec(),
            "without Content-Length the server's `want` is 0, its body loop \
             runs zero iterations, and a body sweep would read flat -- as \
             'body accumulation is free' rather than 'the body was never sent'"
        );

        let both = args(&["--self-test", "--header", "x-a:1", "--body-bytes", "2"]).expect("valid");
        assert_eq!(
            request_bytes(&both),
            b"GET / HTTP/1.1\r\nHost: nova-bench\r\nx-a:1\r\nContent-Length: 2\r\n\r\nxx".to_vec()
        );
    }
```

- [ ] **Step 2: Run the tests to verify they fail**

```bash
cargo build --locked --workspace && cargo test -p nova-bench-http --no-fail-fast
```

Expected: FAIL — `Config` has no field `body_bytes`.

- [ ] **Step 3: Implement**

Add the constant near `IO_TIMEOUT`:

```rust
/// Upper bound on `--body-bytes`, matching `std/http`'s
/// `Limits::default().max_body_bytes`. Above it the target answers
/// `BodyTooLarge` and every response counts as an error, so nothing
/// measurable lies up there; bounding here also keeps an operator's typo
/// from being allocated before any server sees it. Inclusive, so a sweep
/// can reach the bound itself.
const MAX_BODY_BYTES: usize = 1_048_576;
```

Add the field to `Config`:

```rust
    /// Body length in bytes; `0` means no body and no `Content-Length`.
    body_bytes: usize,
```

Declare the default in `from_args`:

```rust
        let mut body_bytes = 0usize;
```

Add the match arm:

```rust
                "--body-bytes" => {
                    body_bytes = take("--body-bytes")?
                        .parse()
                        .map_err(|_| "--body-bytes must be a non-negative integer".to_string())?;
                }
```

Add the checks after the loop, beside the existing `--connections` and
`--addr` checks:

```rust
        if body_bytes > MAX_BODY_BYTES {
            return Err(format!("--body-bytes must be at most {MAX_BODY_BYTES}"));
        }
        // Rejected whether or not a body was asked for. With a body the two
        // framings disagree; without one the target waits for bytes that
        // never arrive and the run becomes timeouts rather than a
        // measurement.
        if headers.iter().any(|h| {
            h.split(':')
                .next()
                .is_some_and(|n| n.trim().eq_ignore_ascii_case("content-length"))
        }) {
            return Err(
                "set a body with --body-bytes; an explicit content-length header is refused"
                    .to_string(),
            );
        }
```

Add `body_bytes,` to the `Ok(Config { ... })` literal.

Extend `request_bytes`, between the header loop and the blank line:

```rust
    if cfg.body_bytes > 0 {
        s.push_str(&format!("Content-Length: {}\r\n", cfg.body_bytes));
    }
    s.push_str("\r\n");
    let mut v = s.into_bytes();
    v.resize(v.len() + cfg.body_bytes, b'x');
    v
```

- [ ] **Step 4: Run the tests to verify they pass**

```bash
cargo build --locked --workspace && cargo test -p nova-bench-http --no-fail-fast
```

Expected: PASS.

- [ ] **Step 5: Run mutation 2 and record the actual result**

Temporarily drop the `Content-Length` line while still appending the body
— comment out the `if cfg.body_bytes > 0 { ... }` block.

```bash
cargo test -p nova-bench-http --no-fail-fast
```

`a_body_reaches_the_wire_with_matching_framing` **must fail**. This
mutation matters beyond the test: without `Content-Length`,
`read_request`'s `want` is 0, the body loop runs zero iterations, and
Task 5's sweep would come back flat and read as "body accumulation is
free". Restore and re-run. **Record what actually happened.**

- [ ] **Step 6: Full gate** — same commands as Task 1 Step 6.

- [ ] **Step 7: Byte-scan staged content and commit** — same as Task 1
  Step 7.

---

## Task 3: The tracked profiling harness

**Files:**
- Create: `docs/benchmarks/profile-http-head.nova`

**Interfaces:**
- Consumes: nothing from earlier tasks.
- Produces: a compiled binary printing one line per header count:
  `headers=<n> iters=<i> offsets_ns=<a> head_ns=<b> acc_offsets=<x> acc_head=<y>`.
  Task 6 records those figures.

**Design notes an implementer needs:**

`parse_offsets` and `parse_request_head` are both `pub fn` in `std/http`
and both synchronous, so a sync `fn main()` is enough —
`examples/01-hello-world` and `examples/02-fibonacci` both use one. std is
glob-imported per ADR 0004, so the file needs no import lines; look at
`docs/benchmarks/server.nova`, which names `read_request` and
`Limits::default()` with none.

**`parse_offsets` is not a zero-allocation baseline.** The intrinsic
copies nothing and holds no Rust-side state, but the wrapper returns
`[Int]`, so a call allocates one Nova array. The subtraction is still the
right instrument because `parse_request_head` calls `parse_offsets`
exactly once (`std/http/lib.nova:201`), so that allocation cancels. Do not
describe it as zero-allocation anywhere.

**`Ok(Some(r))` is E0900** — nested variant patterns are unsupported.
Nest two matches, the same shape `read_request` itself uses at
`std/http/lib.nova:510`. A wildcard inside a variant, `Err(_)`, **is**
supported — `std/core/lib.nova:41` uses it.

The clock is read once before a loop and once after, never per iteration.
`Instant` and `Duration` both carry a public `nanos: Int`; `std/time`'s
own comment records that no record field in this language is
privacy-enforced.

- [ ] **Step 1: Write the harness**

Create `docs/benchmarks/profile-http-head.nova` with the editor tool, not
a heredoc — the file is full of CRLF escapes:

```nova
// Times `std/http`'s head parsing against header count, in a compiled
// binary, so materialisation can be priced in isolation.
//
// **Why two loops and a subtraction.** `parse_offsets` is the thin `pub`
// wrapper over the `http_parse_request` intrinsic; `parse_request_head`
// is that same call plus method and path extraction plus the
// materialisation loop and its `Map`. The difference between the two is
// what `parse_request_head` adds over the intrinsic call, which is
// dominated by materialisation.
//
// **`parse_offsets` is NOT a zero-allocation baseline.** The intrinsic
// copies nothing and holds no Rust-side state, but this wrapper returns
// `[Int]`, so a call allocates one Nova array. That allocation cancels in
// the subtraction because `parse_request_head` calls `parse_offsets`
// exactly once -- it does not vanish from either side.
//
// **The accumulators are load-bearing, not decoration.** A loop whose
// result is discarded can in principle be removed, and a harness that
// measures nothing reports an impressively small number. Each loop folds
// something derived from every call's result into a value that is
// printed, so a reader can tell the loop ran.
//
// Build with a RELEASE `nova`: `find_runtime_lib` resolves the runtime
// staticlib beside the `nova` executable, so a debug `nova` links the
// debug runtime and depresses every figure here.

const ITERS: Int = 20000

// A well-formed request head carrying `extra` headers beyond `Host`.
//
// Each padding header is a fixed width, so the head's byte length is
// linear in `extra` and a slope in nanoseconds per header is not also a
// slope in bytes per header at some other rate.
fn head_with(extra: Int) -> Bytes {
    let mut s = "GET / HTTP/1.1\r\nHost: nova-bench\r\n"
    let mut i = 0
    while i < extra {
        s = "${s}x-pad-${i}: 0123456789abcdef\r\n"
        i = i + 1
    }
    s = "${s}\r\n"
    bytes_from_string(s)
}

fn profile(extra: Int) {
    let buf = head_with(extra)
    let total = extra + 1

    let mut acc_offsets = 0
    let start_a = Instant::now()
    let mut i = 0
    while i < ITERS {
        let t = parse_offsets(buf)
        // `t.len()`, deliberately NOT `t[0]`. Element 0 is the status, and a
        // complete head makes it `0` -- so an accumulator folding it in
        // stays `0` whether this loop runs or not, which is precisely the
        // decorative accumulator this design exists to rule out. The table's
        // length is non-zero and grows with the header count, so a skipped
        // loop is visible and a mis-parsed one is too.
        acc_offsets = acc_offsets + t.len()
        i = i + 1
    }
    let span_a = start_a.elapsed()

    let mut acc_head = 0
    let start_b = Instant::now()
    i = 0
    while i < ITERS {
        // `Ok(Some(r))` is E0900 -- nested variant patterns are not
        // supported -- so this nests two matches, the same shape
        // `read_request` uses on this very function's result.
        match parse_request_head(buf, Limits::default()) {
            Ok(maybe) => {
                match maybe {
                    Some(r) => acc_head = acc_head + r.headers.len()
                    None => acc_head = acc_head - 1
                }
            }
            Err(_) => acc_head = acc_head - 1000
        }
        i = i + 1
    }
    let span_b = start_b.elapsed()

    let ns_a = span_a.nanos / ITERS
    let ns_b = span_b.nanos / ITERS
    println("headers=${total} bytes=${buf.len()} iters=${ITERS} offsets_ns=${ns_a} head_ns=${ns_b} acc_offsets=${acc_offsets} acc_head=${acc_head}")
}

fn main() {
    // Header counts spanning the one-header shape every recorded figure was
    // taken with up to a count well inside `Limits::default()`'s
    // `max_header_count` of 100, with the head staying inside its
    // `max_head_bytes` of 8192.
    profile(0)
    profile(4)
    profile(9)
    profile(19)
    profile(49)
}
```

- [ ] **Step 2: Build a release `nova`, then build the harness**

```bash
cargo build --release --locked --workspace
```

Then, from a scratch directory outside the repository, delete every
sibling name before building and assert exactly one file was written —
`nova build -o NAME` writes exactly `NAME` with no `.exe`, and a stale
file at the path you asked for succeeds and lies:

```bash
rm -f profhead profhead.exe && ls profhead* 2>/dev/null; echo "(nothing above == clean)"
```

```bash
/d/Projects/nona/nova/target/release/nova build /d/Projects/nona/nova/docs/benchmarks/profile-http-head.nova -o profhead && ls -l profhead*
```

Expected: exactly one file named `profhead`. Record its byte size.

- [ ] **Step 3: Run it and check the accumulators moved**

```bash
./profhead
```

Expected: five lines. **Both** accumulators must be positive and must
**grow with header count**:

- `acc_head` sums `r.headers.len()` over the iterations, so at one header
  it is `ITERS`, at five headers `5 * ITERS`, and so on.
- `acc_offsets` sums the offset table's length, which grows by a fixed
  number of entries per header.

An accumulator that does not scale with the header count means that loop
is not parsing what its line claims.

- [ ] **Step 4: Run mutation 3 and record the actual result, on BOTH loops**

Run it twice, once per loop, because an accumulator is only shown to work
by being made to fail:

1. Replace the first `while` loop's body with `i = i + 1` alone. Rebuild
   (deleting siblings first as in Step 2) and run: **`acc_offsets` must
   change to 0.**
2. Restore, then do the same to the second loop: **`acc_head` must change
   to 0.**

If either accumulator does not change, it is decorative and that loop
cannot show it ran — fix it before proceeding. Restore, rebuild, re-run,
and confirm the figures match Step 3. **Record what actually happened for
each, including a result this step did not expect.**

This step earned its place before the plan was finished: a draft of this
harness folded `t[0]` into `acc_offsets`, and element 0 is the status,
which is `0` for a complete head — so that accumulator would have read 0
whether the loop ran or not, and mutation 3 on the first loop would have
passed silently.

- [ ] **Step 5: Full gate**

The harness is not compiled by cargo, so the suite is unmoved — but run
it, because `git status` must be clean apart from the new file and the
totals must be stated rather than assumed:

```bash
cargo build --locked --workspace && cargo test --workspace --no-fail-fast
```

Sum every `test result:` line.

- [ ] **Step 6: Byte-scan staged content and commit**

```bash
git add docs/benchmarks/profile-http-head.nova
```

Scan the staged blob with planted positives first, then commit with
`git commit -F`.

---

## Task 4: The header sweep, with its anchors

**Files:**
- None tracked. This task produces figures, written to a scratch results
  file **outside the repository**; Task 6 writes them into the records.

**Interfaces:**
- Consumes: `--header` from Task 1; the harness binary from Task 3.
- Produces: a scratch results file holding, for every point, the raw
  `RESULT` lines, the binary byte sizes, and the harness output.

**Design notes an implementer needs:**

One fresh process per data point. Process age costs 1.29x to 1.38x after
roughly 280k requests, so a second reading taken from a still-running
server is not a replicate of the first.

Every figure is a range over replicates, never a point — nine runs of one
workload have spanned 1.66x on this host.

`docs/benchmarks/server.nova` has no shutdown path; the procedure kills
it. On Windows an extensionless binary appears to `tasklist` under its
bare name and `taskkill //F //IM <name>` is the reliable stop.

Do not write `AFFINITY=$(powershell ...)`: the spawned server inherits
stdout and the command substitution never returns. Write status to a file
instead.

- [ ] **Step 1: Build a release `nova` and the server, siblings deleted first**

```bash
cargo build --release --locked --workspace
```

```bash
rm -f benchsrv benchsrv.exe && ls benchsrv* 2>/dev/null; echo "(nothing above == clean)"
```

```bash
/d/Projects/nona/nova/target/release/nova build /d/Projects/nona/nova/docs/benchmarks/server.nova -o benchsrv && ls -l benchsrv*
```

Record `benchsrv`'s byte size. It goes beside every figure this task
produces — that byte size is the identity check that caught a published
figure measuring a debug-runtime build.

- [ ] **Step 2: Take the sweep**

For each header count in 1, 5, 10, 20, 50 — where 1 is the `Host`-only
shape every recorded figure was taken with, and the rest add that many
minus one `--header` flags — and for two replicates each:

1. Start `benchsrv` fresh, capturing the port it prints.
2. Run the generator against it for a fixed duration at a fixed
   connection count, with the same values at every point.
3. Record the whole `RESULT` line verbatim.
4. Kill the server.

Every point gets its own server process. Use padding headers of a fixed
width, matching the harness's `x-pad-<i>: 0123456789abcdef`, so the head's
byte length is linear in the count.

**`errors` must be 0 on every run.** A non-zero count means the target
answered outside 2xx — most likely the head crossed `max_head_bytes`
(8192) or `max_header_count` (100) — and that point is a rejection
measurement, not a materialisation measurement. Record it and re-take it
inside the bounds rather than reporting it.

- [ ] **Step 3: Take the two anchors, in this same session**

Labelled as separate populations, never merged into the sweep:

1. `server.nova` at its own one-header no-body baseline — this is the
   sweep's own first point and may be reused, stated as such.
2. `examples/05-json-api`'s empty-store control, built and run the same
   way, which **re-derives** the 105.3 to 115.1 microsecond figure rather
   than quoting it forward. `BENCHMARK.md` explicitly tells a later reader
   to do this, because that control's own two readings differ by 1.09x
   while its margin to the 100-microsecond line is about 5%.

Record `examples/05-json-api`'s binary byte size too. The two servers are
**not** equated anywhere.

- [ ] **Step 4: Run the harness for the isolated side**

```bash
./profhead
```

Twice, each its own process. Record both outputs whole.

- [ ] **Step 5: Compute the slope, both ways**

From the sweep, convert each `rps` to microseconds per request
(`1e6 / rps`) and compute the slope in microseconds per header across the
range — **at both endpoint pairings as well as the middles**. Multiply by
ten for comparison with the ~18 microsecond figure, which is scoped to a
ten-header request.

From the harness, compute `head_ns - offsets_ns` per header count and its
slope per header.

**If the endpoint pairings straddle the ~18 microsecond figure, the
finding is that these data do not settle it.** That is a result. Do not
report the middle as a location.

- [ ] **Step 6: Write the scratch results file**

Outside the repository. It holds every raw `RESULT` line, both harness
outputs, all three binary byte sizes, and the computed slopes with their
endpoint pairings shown. Task 6 reads from this file, not from memory.

---

## Task 5: The body sweep

**Files:**
- None tracked. Appends to the same scratch results file as Task 4.

**Interfaces:**
- Consumes: `--body-bytes` from Task 2; the `benchsrv` binary from Task 4
  Step 1.
- Produces: body-sweep figures appended to the scratch results file.

**Design notes an implementer needs:**

**This loop has never been entered by any measurement taken on this
project.** No run has ever sent a `Content-Length`, so `read_request`'s
`want` has always been 0 and its `while body.len() < want` loop has always
run zero iterations. The sweep's first job is to report what it costs.

`read_request` asks for exactly `want - body.len()` bytes per read, so a
peer that can deliver the whole remainder in one read costs one
concatenation regardless of `want`. Sizes below and above the 4096-byte
read size therefore probe different regimes, which is why the series
spans both.

The comment at `std/http/lib.nova` records the accumulation's bytes copied
as triangular in the read count. **Establishing or refuting that shape is
not promised by this sweep** — the sizes chosen may not settle it, and
saying so is better than implying coverage.

- [ ] **Step 1: Take the sweep**

For each body size in 0, 1024, 4096, 16384, 65536, 262144 — spanning from
no body, through sizes a single read can deliver whole, to sizes well
above the 4096-byte read size, all inside `max_body_bytes` (1048576) — and
for two replicates each:

1. Start `benchsrv` fresh, capturing its printed port.
2. Run the generator with `--body-bytes <size>` at the same duration and
   connection count used in Task 4.
3. Record the whole `RESULT` line verbatim.
4. Kill the server.

One fresh server process per point. `errors` must be 0 on every run; a
non-zero count at the top of the range means the body crossed
`max_body_bytes` and that point is a rejection measurement.

- [ ] **Step 2: Check the sweep is not flat for the wrong reason**

If every point reads within noise of the 0-byte point, **before**
concluding that body accumulation is cheap, confirm the body was actually
sent: re-run one non-zero point and verify the request bytes carry
`Content-Length`. Task 2's mutation 2 is exactly this failure mode, and a
flat sweep is what it produces.

Record which of the two explanations the check supports.

- [ ] **Step 3: Compute and record**

Convert each `rps` to microseconds per request and compute the cost per
byte and per read at each size, **propagating ranges at both endpoint
pairings**. Append every raw `RESULT` line, the binary byte size, and the
computed figures to the scratch results file.

State plainly whether the data show a shape that grows faster than
linearly in body size, or do not settle it.

---

## Task 6: The records

**Files:**
- Modify: `docs/benchmarks/README.md`
- Modify: `docs/adr/0019-offset-table-intrinsic-boundary.md`
- Modify: `examples/05-json-api/BENCHMARK.md`
- Modify: `CHANGELOG.md`

**Interfaces:**
- Consumes: the scratch results file from Tasks 4 and 5; the mutation
  results from Tasks 1, 2 and 3.
- Produces: the tracked record.

**Design notes an implementer needs:**

**The figure is written in one home and pointed at from the others.** PR
#53 found the same claim restated across seven satellite records and wrong
in all seven at once; the structural half of that fix is not to repeat it.
`docs/benchmarks/README.md` is the home for the procedure and the figures;
everything else points at it.

**Nothing is edited at its own site.** ADR 0019's inferred ~18 microsecond
figure stays exactly as written, with a dated pointer beside it. That is
this project's practice: amend, never silently rewrite.

- [ ] **Step 1: `docs/benchmarks/README.md` — the home**

Add a dated section carrying: the exact commands for both sweeps; the
requirement that a **release** `nova` builds the server and the harness,
with the `find_runtime_lib` reason; the sibling-deletion step and the
assertion that exactly one file was written; every binary's byte size;
`max_head_bytes` 8192 and `max_header_count` 100 and `max_body_bytes`
1048576 as the bounds on each series; every raw `RESULT` line; the harness
output; and the computed slopes **with their endpoint pairings shown**.

State which of the four mechanisms `BENCHMARK.md` names — `read_request`'s
parse, the socket write, the scheduler, the collector — this increment
measured and which it did not.

Give the existing "Header materialisation runs at its one-header minimum"
bullet a **pointer** to the new section. Do not rewrite it.

- [ ] **Step 2: `docs/adr/0019-offset-table-intrinsic-boundary.md`**

Beside section 7's ~18 microsecond figure, add a dated line pointing at
the README section. **Do not edit the figure.** If the measurement differs
from it, the pointer says so; the original stays legible as what was
believed when it was written.

- [ ] **Step 3: `examples/05-json-api/BENCHMARK.md`**

A dated section recording what the sweeps found about the residual its
"Where the cost is" section names, pointing at the README for the figures
rather than restating them, and naming explicitly which of its four
mechanisms remain unmeasured.

Record that the sweeps ran against `docs/benchmarks/server.nova` and that
the two servers are not equated — the gate example appears only as a
separately-labelled anchor whose empty-store control was **re-derived**
in the same session.

- [ ] **Step 4: `CHANGELOG.md`**

An `[Unreleased]` entry naming the two flags, the tracked harness, what
was measured, and — in the same entry — the caveat that a slope of this
size is consistent with mechanisms other than allocation. The caveat
travels with the figure, because `CHANGELOG.md` is the file most likely to
be read alone.

- [ ] **Step 5: Sweep for claims this increment bears on**

Build the roster as a **set difference over the union** of every affected
token — the inferred figure, the mechanism's name, the body loop, the
residual — never from one token's grep:

```bash
comm -23 <(git grep -l TOKEN | sort) <(git diff --name-only main..HEAD | sort)
```

Run it once per token and take the union of the results. Use
whitespace-tolerant patterns that normalise `//`, `///` and `>` gutters,
because grep is line-oriented and a stale figure has survived a sweep on
this project by being line-wrapped.

Every file the sweep surfaces either gets a pointer or is recorded as
deliberately left alone.

- [ ] **Step 6: Full gate**

```bash
cargo build --locked --workspace && cargo test --workspace --no-fail-fast
```

Sum every `test result:` line across all 45 targets and state the total.

```bash
cargo fmt --all -- --check && cargo clippy --locked --all-targets --all-features -- -D warnings
```

- [ ] **Step 7: Byte-scan every staged record and commit**

Stage each path by name. Scan every staged blob via `git show :<path>`
with planted positives first — valid UTF-8, no byte below 0x20 outside
tab, CR and LF, no 0x7f, zero backslash-u-four-hex in tracked markdown.
Then `git commit -F`.

---

## Verification before the branch is offered

- [ ] `cargo build --locked --workspace` then
      `cargo test --workspace --no-fail-fast`, summing **every**
      `test result:` line across 45 targets, with the new total stated
      rather than the baseline restated.
- [ ] `cargo fmt --all -- --check` exits 0 and `git diff --numstat` is
      empty afterwards.
- [ ] `cargo clippy --locked --all-targets --all-features -- -D warnings`
      exits 0.
- [ ] Every mutation in Tasks 1, 2 and 3 was run and its **actual** result
      recorded, including any that did not behave as the plan expected.
- [ ] Every figure in every record carries a range and a binary byte size.
- [ ] No record totals the measured mechanisms against the
      100-microsecond budget, and none claims the gate is met, reachable
      or unreachable.
- [ ] The branch-local spec commit's SHA appears in no tracked file.

**If the smoke test `bench_http_server_and_generator_agree` fails:** that
is the known Windows async flake, roughly one run in four, whose shape is
broader than the `0xc0000005` it is often described by — a 2026-08-29
instance carried no crash code at all, just an async child exiting
non-zero with empty stdout. Re-run, say that you re-ran, attribute no
cause, and fix nothing. **Do not grep for that code as the test of whether
it fired.**
