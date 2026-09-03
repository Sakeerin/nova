# Phase 2 Gate Benchmark Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Produce a reproducible methodology and one honest throughput number for `std/http`'s read-and-parse path, without writing the gate's examples and without claiming the gate is passed.

**Architecture:** A dependency-free Rust load generator in the workspace drives keep-alive HTTP against a minimal Nova server. The generator carries a `--self-test` mode that measures its own ceiling against a server inside the same binary, so the Nova figure is always reported beside the harness figure that bounds it. A one-second smoke test keeps both halves working in CI; the full load run is taken by hand.

**Tech Stack:** Rust (`std::net`, `std::thread` only — no new dependency), Nova (`std/http` server half, `std/net`), the existing `assert_cmd` test harness.

**Spec:** `docs/superpowers/specs/2026-09-02-phase-2-gate-benchmark-design.md`

## Global Constraints

Every task's requirements implicitly include this section.

- `cargo build --locked --workspace` **before** `cargo test`. A test run after a failed build proves nothing.
- `cargo test --locked --workspace --no-fail-fast`. `--no-fail-fast` is mandatory.
- **Never pipe cargo output through `head`/`tail` before summing.** There are 44 test targets; sum **every** `test result:` line. Baseline: **1099 passed / 0 failed / 8 ignored**.
- No `reason = "..."` in any lint attribute. MSRV is 1.78 and `reason` postdates it.
- `cargo clippy --locked --workspace --all-targets -- -D warnings` must pass on **both** ubuntu and windows.
- `cargo fmt --all -- --check`. If it fails, run `cargo fmt --all`, then `git diff --numstat` and confirm the change count matches what you meant — `cargo fmt` writes LF into this CRLF working copy.
- The ignored ADR-0010 GC tests stay ignored and untouched.
- The poll ABI is frozen and no panic may cross a generated poll boundary. **Inert in this increment** — nothing here adds a runtime intrinsic or touches an async boundary in Rust. Say so rather than implying care satisfied it.
- Every fixture path unique per process.
- Commit messages written to a UTF-8 file and applied with `git commit -F`, **never a heredoc**. Every body ends exactly with the line `Co-Authored-By: Claude Opus 5 <noreply@anthropic.com>`.
- **Cite no SHA that is not already an ancestor of `main`.** `5be3a4f` is. Every commit on this branch is branch-local; refer to those changes by what they did, and derive the roster with `git log --format=%h main..HEAD` rather than typing one.
- **Byte-scan every file you write:** valid UTF-8; no byte below 0x20 outside tab, CR and LF; no 0x7f; and **zero occurrences of backslash-u followed by four hex digits** in tracked markdown — write code points as U+XXXX. Build that pattern from `chr(92)` in Python and assert it against a planted positive first: in a POSIX ERE `\u` degrades to a literal `u`, so a `grep -E` for it matches the word "succeeds". **Scan staged content via `git show :<path>`, not working-tree files** — `core.autocrlf` smudges the working tree, so a working-tree scan answers a different question than "what will git store".
- **Do not author Nova string escapes or markdown backslashes through a heredoc.** A quoted bash heredoc ate a backslash level twice on the previous branch, once putting four real CR bytes into a tracked file. Use the Write tool, or a Python rewrite that asserts a match count.
- **Sentence-shape discipline**, binding on every comment and record: prefer a roster with no count; a corrected number is usually the wrong fix; no ordinals or closed worlds over `std`, the runtime, the workspace or the record set; never claim a test is "the only" thing that catches something; **and never write that a fixture pins something without checking that a fixture actually executes it** — three findings on the previous branch were comments asserting coverage no test performed, and counting call sites caught all three. grep is line-oriented, so a miss is **not** evidence of absence: sweep prose with whitespace-tolerant patterns that also normalise `//`, `///` and `>` gutters.

### Known flake, and how to handle it

Roughly one run in four, an async/threading test fails on Windows. It has historically carried `0xc0000005`, but **not always** — a 2026-08-29 instance carried no crash code at all, just an async child exiting non-zero with empty stdout. The cause is not established and every shared-path hypothesis has been eliminated. **Task 2's smoke test is this increment's exposure.** If you hit it: re-run, say so in your report, attribute no cause, and fix nothing. **Do not grep for `0xc0000005` as the test of whether it fired** — it will tell you it did not.

---

## THE DECISIVE CONSTRAINT: which build you measure

Read this before Task 3, and do not let the naturally-reached-for commands decide it for you.

`find_runtime_lib()` (`crates/nova-driver/src/link.rs`) resolves the runtime static library **next to the `nova` executable**, with a `NOVA_RUNTIME_LIB` environment override taking precedence. So a `nova` from `target/debug/` links the **debug** runtime, and every figure taken that way is depressed by it.

`nova build` has two backends: the default **Cranelift**, and `--release`, described by its own help text as *"Optimizing build via the LLVM backend (emits LLVM IR and compiles it with a discovered `clang`/`llc`)"*.

| `nova` binary | code backend | verdict |
|---|---|---|
| debug | Cranelift | measurable, **misleading** — a debug runtime depresses everything |
| **release** | **Cranelift** | **measurable here, and what this increment reports** |
| debug | LLVM | unmeasurable on this host |
| release | LLVM | unmeasurable on this host |

**Measured on this host: `clang`, `llc`, `clang-17` and `llc-17` are all absent**, so the LLVM path cannot run and the optimised-codegen figure stays unmeasured. That bears directly on reading the result against the gate's 10k, which presumably assumed an optimising compiler.

Task 3's procedure therefore begins with `cargo build --release`, and the recorded observation names **both** axes. A req/sec figure without its backend and its runtime profile is close to meaningless.

**The smoke test is the exception and may use `nova run`.** It measures correctness, not throughput, so JIT warmup is irrelevant there and a separate build step would only add moving parts.

---

## File Structure

| path | status | responsibility |
|---|---|---|
| `crates/nova-bench-http/Cargo.toml` | create | manifest, carrying `[[bin]] bench = false` |
| `crates/nova-bench-http/src/main.rs` | create | argument parsing, the worker loop, the response reader, `--self-test`, the report |
| `docs/benchmarks/server.nova` | create | the Nova server under test |
| `docs/benchmarks/README.md` | create | the procedure: how to reproduce |
| `docs/benchmarks/http-fixed-response.md` | create | one dated observation from one machine |
| `crates/nova-cli/tests/run_tests.rs` | modify | one smoke test, appended |
| `nova-spec/60-EXAMPLES.md` | modify | dated amendment to §5 |
| `nova-spec/00-MASTER-SPEC.md` | modify | dated amendment to §3 |
| `CHANGELOG.md` | modify | entry under `[Unreleased]` |

`Cargo.toml`'s `members = ["crates/*"]` is a glob, so the new crate joins the workspace with no manifest edit.

**Two placement decisions already settled by the spec — do not revisit them.** The server lives under `docs/benchmarks/` rather than `examples/`, because every existing example is pinned by a `nova run` test against a golden and a load-test server has no golden. And it deliberately does **not** take the name `03-http-server`: that numbering drift is recorded as open in two spec files, and shipping something the spec does not describe under the name the spec reserves would half-resolve it, which is worse than leaving it recorded.

---

## Task 1: The load generator

**Files:**
- Create: `crates/nova-bench-http/Cargo.toml`
- Create: `crates/nova-bench-http/src/main.rs`

**Interfaces:**
- Consumes: nothing. `std::net` and `std::thread` only.
- Produces: a `nova-bench-http` binary accepting `--addr`, `--connections`, `--duration`, `--warmup`, `--self-test`, and printing a final machine-readable `RESULT` line. Task 2's smoke test parses that line; Task 3's procedure reads it by eye.

### The one correctness trap in this task

**On a keep-alive connection you must consume exactly one response before sending the next request.** Read too little and the next response's bytes are mistaken for this one's; read too much and you eat the next one. Either way the stream desyncs and every count after it is meaningless — while still *looking* like a successful run. So the reader finds the end of the head (`\r\n\r\n`), parses `content-length`, and reads until it holds exactly head-plus-body. That is the reason for the parser below; it is not incidental.

- [ ] **Step 1: Write the manifest**

`crates/nova-bench-http/Cargo.toml`:

```toml
[package]
name = "nova-bench-http"
version = "0.1.0"
edition.workspace = true
rust-version.workspace = true
license.workspace = true
repository.workspace = true
description = "Keep-alive HTTP load generator for benchmarking Nova's std/http (no dependencies)"

[[bin]]
name = "nova-bench-http"
path = "src/main.rs"
# Not a benchmark target: `cargo bench --workspace -- --output-format bencher`
# would otherwise build this under libtest's harness, which rejects criterion's
# `--output-format`. The workspace's only benchmark is `nova-lexer/benches/lex.rs`.
bench = false

[dependencies]
# Deliberately empty. `std::net` and `std::thread` are sufficient, and the
# previous increment had to justify `httparse` as its single new dependency --
# test tooling is a poor place to spend that argument again.
```

- [ ] **Step 2: Write the failing tests**

Create `crates/nova-bench-http/src/main.rs` containing **only** the test module below, so it fails to compile against functions that do not exist yet.

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn head_end_is_found_only_on_a_complete_terminator() {
        assert_eq!(head_end(b"HTTP/1.1 200 OK\r\n\r\nbody"), Some(19));
        assert_eq!(head_end(b"HTTP/1.1 200 OK\r\n\r"), None, "a split terminator is not an end");
        assert_eq!(head_end(b""), None);
    }

    #[test]
    fn content_length_is_parsed_case_insensitively() {
        let head = b"HTTP/1.1 200 OK\r\nContent-Length: 5\r\n\r\n";
        assert_eq!(content_length(head), Some(5));
        let lower = b"HTTP/1.1 200 OK\r\ncontent-length: 12\r\n\r\n";
        assert_eq!(content_length(lower), Some(12));
    }

    #[test]
    fn a_head_without_content_length_reports_none() {
        assert_eq!(content_length(b"HTTP/1.1 200 OK\r\nHost: x\r\n\r\n"), None);
    }

    #[test]
    fn a_non_numeric_content_length_reports_none_rather_than_zero() {
        // Zero would be indistinguishable from a legitimately empty body, and
        // the reader would then desync on the next keep-alive response.
        assert_eq!(content_length(b"HTTP/1.1 200 OK\r\nContent-Length: abc\r\n\r\n"), None);
    }

    #[test]
    fn a_response_is_complete_only_when_head_and_body_are_both_present() {
        let full = b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\n\r\nhi";
        assert_eq!(response_len(full), Some(full.len()));
        assert_eq!(response_len(&full[..full.len() - 1]), None, "one byte short is not complete");
    }

    #[test]
    fn two_pipelined_responses_report_only_the_first_length() {
        let one = b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\n\r\nhi";
        let mut two = one.to_vec();
        two.extend_from_slice(one);
        assert_eq!(
            response_len(&two),
            Some(one.len()),
            "reading past the first response would desync a keep-alive stream"
        );
    }

    #[test]
    fn args_reject_a_missing_value_rather_than_defaulting_it() {
        assert!(Config::from_args(["--connections"].iter().map(|s| s.to_string())).is_err());
    }

    #[test]
    fn args_reject_zero_connections() {
        let a = ["--addr", "127.0.0.1:1", "--connections", "0"];
        assert!(Config::from_args(a.iter().map(|s| s.to_string())).is_err());
    }

    #[test]
    fn self_test_needs_no_addr_and_plain_mode_does() {
        assert!(Config::from_args(["--self-test"].iter().map(|s| s.to_string())).is_ok());
        assert!(Config::from_args(std::iter::empty()).is_err());
    }
}
```

- [ ] **Step 3: Run the tests to verify they fail**

```bash
cargo test --locked --package nova-bench-http --no-fail-fast
```

Expected: a compile error naming `head_end`, `content_length`, `response_len` and `Config`. A compile failure is the expected failing state; there is no partial implementation to get a clean assertion failure from.

- [ ] **Step 4: Write the implementation**

Prepend to `crates/nova-bench-http/src/main.rs`, above the test module:

> **Corrected 2026-09-03, after Task 1 ran.** Two things about the block
> below, both found by executing it rather than by reading it.
>
> **`content_length` was wrong here, and is fixed below.** It read
> `let (name, value) = line.split_once(':')?;` inside the loop. `?` on an
> `Option` returns from the whole function, and the first line scanned is
> always the status line, which carries no colon -- so `content_length`
> returned `None` before reaching any header, and `response_len`'s
> `.unwrap_or(0)` then treated every body as empty. That is the desync this
> task exists to prevent, moved from reading the wrong count to always
> computing the wrong one. The tests in Step 2 caught it: the
> case-insensitive positive match failed, and so did both `response_len`
> tests, the pipelining guard among them.
>
> **`worker` below is INCOMPLETE, and deliberately not re-copied here.**
> Task 1's review found that its `stream.read` has no timeout, so a peer
> that accepts and then stalls -- rather than closing -- hangs the run past
> `--duration` with no diagnostic and no `RESULT` line, because a blocked
> worker never reaches its `stop` check and `run_load` then joins it. What
> shipped arms a read and a write timeout on the connection and threads the
> value through `worker` and `run_load` so a test can pass a short one.
> **Read `crates/nova-bench-http/src/main.rs` for the current shape rather
> than transcribing the block below**, which is kept as the plan it was.

```rust
//! Keep-alive HTTP load generator for Nova's `std/http`.
//!
//! # Why this exists rather than `wrk`
//!
//! `nova-spec/60-EXAMPLES.md` §5's methodology names `wrk -t8 -c200 -d30s`.
//! Measured on this project's Windows dev host: of `wrk`, `oha`, `bombardier`,
//! `hey`, `ab`, `k6` and `vegeta`, none is installed, `curl` is the only HTTP
//! client present, and `wrk` is POSIX-only so it does not run here natively at
//! all. A generator in the workspace is reproducible by anyone who can already
//! build this repo, and the methodology can cite exact code rather than a tool
//! version.
//!
//! **Consequence, and the design spec states it too: figures from here are not
//! directly comparable to published `wrk` numbers.** Different generator,
//! different connection handling, different measurement window.
//!
//! # No dependencies
//!
//! `std::net` and `std::thread` are sufficient: one OS thread per connection,
//! each blocked on I/O most of the time. Nova has no HTTP client of its own --
//! `std/http` shipped the server half only -- so this must be Rust regardless.

use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

/// The request every worker sends. HTTP/1.1 keeps the connection open by
/// default, which is the point: `--connections N` means N connections carrying
/// many requests each, not N requests.
const REQUEST: &[u8] = b"GET / HTTP/1.1\r\nHost: nova-bench\r\n\r\n";

/// Byte offset just past a complete `CRLF CRLF`, or `None`.
///
/// A split terminator is not an end: returning an offset for `"...\r\n\r"`
/// would have the caller treat one byte of the terminator as body.
fn head_end(buf: &[u8]) -> Option<usize> {
    buf.windows(4).position(|w| w == b"\r\n\r\n").map(|i| i + 4)
}

/// `Content-Length` from a complete head, or `None` when it is absent or not a
/// plain decimal run.
///
/// `None` for a malformed value rather than `0`: zero is indistinguishable from
/// a legitimately empty body, and the reader would then desync on the next
/// keep-alive response instead of reporting an error.
fn content_length(head: &[u8]) -> Option<usize> {
    let text = std::str::from_utf8(head).ok()?;
    for line in text.split("\r\n") {
        // A line with no `:` (the status line, or a trailing blank line) is
        // simply not a header -- skip it rather than treating its absence as
        // "no Content-Length anywhere", which would stop the scan before it
        // ever reaches the real header.
        let Some((name, value)) = line.split_once(':') else {
            continue;
        };
        if name.trim().eq_ignore_ascii_case("content-length") {
            let v = value.trim();
            if v.is_empty() || !v.bytes().all(|b| b.is_ascii_digit()) {
                return None;
            }
            return v.parse().ok();
        }
    }
    None
}

/// The length of the FIRST complete response in `buf`, or `None` if it is not
/// yet complete.
///
/// Returning the first response's length rather than the buffer's is what keeps
/// a keep-alive stream in sync when two responses arrive in one read.
fn response_len(buf: &[u8]) -> Option<usize> {
    let end = head_end(buf)?;
    let body = content_length(&buf[..end]).unwrap_or(0);
    let total = end + body;
    if buf.len() >= total {
        Some(total)
    } else {
        None
    }
}

struct Config {
    addr: String,
    connections: usize,
    duration: Duration,
    warmup: Duration,
    self_test: bool,
}

impl Config {
    fn from_args<I: Iterator<Item = String>>(args: I) -> Result<Config, String> {
        let mut addr: Option<String> = None;
        let mut connections = 1usize;
        let mut duration = 10u64;
        let mut warmup = 1u64;
        let mut self_test = false;
        let mut it = args;
        while let Some(a) = it.next() {
            let mut take = |name: &str| -> Result<String, String> {
                it.next().ok_or_else(|| format!("{name} needs a value"))
            };
            match a.as_str() {
                "--self-test" => self_test = true,
                "--addr" => addr = Some(take("--addr")?),
                "--connections" => {
                    connections = take("--connections")?
                        .parse()
                        .map_err(|_| "--connections must be a positive integer".to_string())?;
                }
                "--duration" => {
                    duration = take("--duration")?
                        .parse()
                        .map_err(|_| "--duration must be seconds as an integer".to_string())?;
                }
                "--warmup" => {
                    warmup = take("--warmup")?
                        .parse()
                        .map_err(|_| "--warmup must be seconds as an integer".to_string())?;
                }
                other => return Err(format!("unknown argument: {other}")),
            }
        }
        if connections == 0 {
            return Err("--connections must be at least 1".to_string());
        }
        if !self_test && addr.is_none() {
            return Err("--addr is required unless --self-test is given".to_string());
        }
        Ok(Config {
            addr: addr.unwrap_or_default(),
            connections,
            duration: Duration::from_secs(duration),
            warmup: Duration::from_secs(warmup),
            self_test,
        })
    }
}

struct Report {
    requests: u64,
    errors: u64,
    elapsed: Duration,
    per_conn_min: u64,
    per_conn_max: u64,
}

/// Drive one connection until `stop`, returning `(requests, errors)`.
///
/// A connection that cannot be established at all is one error and no requests,
/// rather than a panic: `main` decides whether the whole run is a failure, and
/// it needs the count to decide.
fn worker(addr: &str, stop: &AtomicBool) -> (u64, u64) {
    let mut stream = match TcpStream::connect(addr) {
        Ok(s) => s,
        Err(_) => return (0, 1),
    };
    if stream.set_nodelay(true).is_err() {
        return (0, 1);
    }
    let mut requests = 0u64;
    let mut errors = 0u64;
    let mut buf: Vec<u8> = Vec::with_capacity(4096);
    let mut chunk = [0u8; 4096];
    while !stop.load(Ordering::Relaxed) {
        if stream.write_all(REQUEST).is_err() {
            errors += 1;
            break;
        }
        // Consume exactly one response. Reading less desyncs the stream; reading
        // more eats the next response. See `response_len`.
        loop {
            if let Some(n) = response_len(&buf) {
                buf.drain(..n);
                requests += 1;
                break;
            }
            match stream.read(&mut chunk) {
                Ok(0) => {
                    errors += 1;
                    return (requests, errors);
                }
                Ok(n) => buf.extend_from_slice(&chunk[..n]),
                Err(_) => {
                    errors += 1;
                    return (requests, errors);
                }
            }
        }
    }
    (requests, errors)
}

fn run_load(addr: &str, connections: usize, duration: Duration) -> Report {
    let stop = Arc::new(AtomicBool::new(false));
    let start = Instant::now();
    let handles: Vec<_> = (0..connections)
        .map(|_| {
            let stop = Arc::clone(&stop);
            let addr = addr.to_string();
            std::thread::spawn(move || worker(&addr, &stop))
        })
        .collect();
    std::thread::sleep(duration);
    stop.store(true, Ordering::Relaxed);
    let mut requests = 0u64;
    let mut errors = 0u64;
    let mut per = Vec::with_capacity(connections);
    for h in handles {
        let (r, e) = h.join().unwrap_or((0, 1));
        requests += r;
        errors += e;
        per.push(r);
    }
    Report {
        requests,
        errors,
        elapsed: start.elapsed(),
        per_conn_min: per.iter().copied().min().unwrap_or(0),
        per_conn_max: per.iter().copied().max().unwrap_or(0),
    }
}

/// A fixed-response server inside this binary, for `--self-test`.
///
/// This is the harness's own ceiling: without it a Nova reading of, say, 5k
/// could be Nova's limit or this generator's, and no care in the prose
/// distinguishes them.
fn spawn_self_test_server() -> Result<String, String> {
    let listener =
        TcpListener::bind("127.0.0.1:0").map_err(|e| format!("self-test bind: {e}"))?;
    let addr = listener
        .local_addr()
        .map_err(|e| format!("self-test local_addr: {e}"))?
        .to_string();
    std::thread::spawn(move || {
        for conn in listener.incoming() {
            let Ok(mut conn) = conn else { continue };
            let _ = conn.set_nodelay(true);
            std::thread::spawn(move || {
                let body = b"{\"ok\":true}";
                let head = format!(
                    "HTTP/1.1 200 OK\r\ncontent-length: {}\r\ncontent-type: application/json\r\n\r\n",
                    body.len()
                );
                let mut buf: Vec<u8> = Vec::with_capacity(4096);
                let mut chunk = [0u8; 4096];
                loop {
                    // Consume one request head before answering, so this server
                    // stays in sync with a keep-alive client for the same reason
                    // the worker does.
                    match head_end(&buf) {
                        Some(n) => {
                            buf.drain(..n);
                            if conn.write_all(head.as_bytes()).is_err()
                                || conn.write_all(body).is_err()
                            {
                                return;
                            }
                        }
                        None => match conn.read(&mut chunk) {
                            Ok(0) | Err(_) => return,
                            Ok(n) => buf.extend_from_slice(&chunk[..n]),
                        },
                    }
                }
            });
        }
    });
    Ok(addr)
}

fn main() {
    let cfg = match Config::from_args(std::env::args().skip(1)) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("nova-bench-http: {e}");
            eprintln!(
                "usage: nova-bench-http (--addr HOST:PORT | --self-test) \
                 [--connections N] [--duration SECS] [--warmup SECS]"
            );
            std::process::exit(2);
        }
    };

    let (addr, label) = if cfg.self_test {
        match spawn_self_test_server() {
            Ok(a) => (a, "self-test"),
            Err(e) => {
                eprintln!("nova-bench-http: {e}");
                std::process::exit(1);
            }
        }
    } else {
        (cfg.addr.clone(), "target")
    };

    if !cfg.warmup.is_zero() {
        let w = run_load(&addr, cfg.connections, cfg.warmup);
        if w.requests == 0 {
            // A warmup that completed no request means nothing is answering.
            // Reporting 0 req/sec here would read as a valid measurement.
            eprintln!(
                "nova-bench-http: warmup completed no requests against {addr} \
                 ({} errors) -- is the server running?",
                w.errors
            );
            std::process::exit(1);
        }
    }

    let r = run_load(&addr, cfg.connections, cfg.duration);
    if r.requests == 0 {
        eprintln!("nova-bench-http: measurement completed no requests against {addr}");
        std::process::exit(1);
    }
    let secs = r.elapsed.as_secs_f64();
    let rps = if secs > 0.0 {
        r.requests as f64 / secs
    } else {
        0.0
    };
    println!(
        "RESULT mode={label} addr={addr} connections={} requests={} errors={} \
         elapsed_ms={} rps={rps:.1} conn_min={} conn_max={}",
        cfg.connections,
        r.requests,
        r.errors,
        r.elapsed.as_millis(),
        r.per_conn_min,
        r.per_conn_max
    );
}
```

- [ ] **Step 5: Run the tests to verify they pass**

```bash
cargo build --locked --workspace && cargo test --locked --package nova-bench-http --no-fail-fast
```

Expected: every test from Step 2 passes.

- [ ] **Step 6: Verify `--self-test` produces a ceiling**

```bash
cargo run --release --package nova-bench-http -- --self-test --connections 4 --duration 2 --warmup 1
```

Expected: one `RESULT mode=self-test ...` line with `errors=0`, `requests` well above zero, and `conn_min`/`conn_max` within the same order of magnitude of each other. **Record the number in your report** — Task 3 needs it as the harness ceiling, and this is the first time anyone has seen it.

- [ ] **Step 7: Mutation 3 — point the generator at a closed port**

```bash
cargo run --release --package nova-bench-http -- --addr 127.0.0.1:9 --connections 2 --duration 1 --warmup 1
```

Expected: a non-zero exit and a message naming the address, **not** a `RESULT` line with `rps=0.0`. A zero-rps `RESULT` would read as a valid measurement, which is the failure this guards. **Report what actually happened**, including the exit code and the exact message.

- [ ] **Step 8: Run the full suite, lint, format, byte-scan, commit**

```bash
cargo build --locked --workspace && cargo test --locked --workspace --no-fail-fast
cargo clippy --locked --workspace --all-targets -- -D warnings
cargo fmt --all -- --check
```

Sum **every** `test result:` line across all 44 targets. Expected: 1099 plus the tests added here, 0 failed, 8 ignored. Byte-scan the staged files, then:

```bash
git add crates/nova-bench-http
git commit -F <path-to-message-file>
```

Subject: `feat(bench): add a dependency-free keep-alive HTTP load generator`. The body records why this exists rather than `wrk` (naming the generators checked and that `wrk` is POSIX-only), that it adds no dependency, why the response reader must consume exactly one response, the `--self-test` ceiling you measured, and what mutation 3 actually did.

---

## Task 2: The Nova server and the smoke test

**Files:**
- Create: `docs/benchmarks/server.nova`
- Modify: `crates/nova-cli/tests/run_tests.rs`

**Interfaces:**
- Consumes: `nova-bench-http` from Task 1, including its `RESULT` line. From `std/http`: `read_request(conn: TcpStream, limits: Limits) -> Result<Request, HttpError>`, `write_response(conn: TcpStream, resp: Response) -> Result<Int, IoError>`, `Response::text(status: Int, s: String) -> Response`, `Response::to_bytes(self) -> Bytes`, `Limits::default()`. From `std/net`: `bind(addr: String) -> Result<TcpListener, IoError>` (a plain `fn`), `TcpListener::local_port(self) -> Result<Int, IoError>` (plain `fn`), `TcpListener::accept(self)` (async), `TcpStream::write(self, buf: Bytes)` (async), `TcpStream::close(self)` (async). From `std/task`: `spawn`.
- Produces: `docs/benchmarks/server.nova`, which prints exactly one line `listening on 127.0.0.1:<port>` before serving. Task 3's procedure reads that line by eye.

### Three properties of this runtime that shape the server

**One socket wait per task, enforced by process abort.** Staging two I/O waits in a single poll ends the process (`stage_park` in `crates/nova-runtime/src/task.rs`), so a task parked in `accept` cannot also read a connection. A server needs one task accepting and one per connection, and this language has no `select` or `race`. Read `std/net`'s own `accept` doc comment before writing the loop.

**The server never exits.** `block_on` cannot return while any task is parked, and the accept loop parks forever. The procedure kills the process; do not add a shutdown path and do not treat the absence of one as a defect.

**There is no read timeout.** `read_request` parks with no deadline, so a connection the generator abandons holds its task until the process dies. Fine for a bounded run; `std/http`'s own header records that it is not fine for a service.

- [ ] **Step 1: Write the server**

Create `docs/benchmarks/server.nova` with the **Write tool** — it contains `\r\n` escapes that a heredoc would eat.

```nova
// The Nova server under benchmark. Not an example, and deliberately not named
// `03-http-server`: that number and name have drifted from what `examples/`
// holds, the drift is recorded as open in `nova-spec/00-MASTER-SPEC.md` and
// `nova-spec/60-EXAMPLES.md`, and taking the reserved name for something the
// spec does not describe would half-resolve it. See
// `docs/superpowers/specs/2026-09-02-phase-2-gate-benchmark-design.md` section 3.
//
// **The response bytes are built ONCE, outside the accept loop, and reused.**
// `Response::to_bytes` allocates, and hoisting it isolates the read-and-parse
// path -- which is where the roughly 18 microseconds per request of eager
// header materialisation that `std/http`'s design spec discloses actually
// lives. So any number taken against this server EXCLUDES response
// serialisation: it is a ceiling for a real server rather than a simulation of
// one. `docs/benchmarks/README.md` states that beside the figure.
//
// **This server never exits.** `block_on` cannot return while a task is parked,
// and the accept loop parks forever, so the benchmark procedure kills the
// process. There is no shutdown path and its absence is not an oversight.
//
// **It logs nothing per request.** Per-request output at these rates would
// become the bottleneck we accidentally measured.
//
// One task accepts and one task serves each connection, which is forced rather
// than chosen: staging two socket waits in a single poll aborts the process
// (`stage_park` in `crates/nova-runtime/src/task.rs`), so the task parked in
// `accept` cannot also read a connection.

async fn serve(conn: TcpStream, wire: Bytes) {
    // Keep-alive: one connection, requests until the peer stops or errs.
    while true {
        match read_request(conn, Limits::default()).await {
            Ok(req) => {
                match conn.write(wire).await {
                    Ok(n) => n
                    Err(e) => break
                }
            }
            Err(e) => break
        }
    }
    let _ = conn.close().await
}

async fn main() {
    let l = match bind("127.0.0.1:0") {
        Ok(l) => l
        Err(e) => panic("bind: ${e.message}")
    }
    let port = match l.local_port() {
        Ok(p) => p
        Err(e) => panic("local_port: ${e.message}")
    }
    // The one line this server prints. The benchmark procedure reads it by eye
    // and the smoke test parses it from stdout, so its shape is load-bearing.
    println("listening on 127.0.0.1:${port}")

    let wire = Response::text(200, "{\"ok\":true}").to_bytes()

    while true {
        match l.accept().await {
            Ok(conn) => {
                let h = spawn(serve(conn, wire))
            }
            Err(e) => break
        }
    }
}
```

**Verify two things about this source before moving on**, because both are the kind of assumption this project has been burned by: that `spawn`'s handle can be bound and dropped without joining (the accept loop must not wait on a connection task), and that a `Bytes` value captured by an `async fn` parameter survives being passed to many tasks. If either does not hold, report it rather than working around it.

- [ ] **Step 2: Write the failing smoke test**

Append to `crates/nova-cli/tests/run_tests.rs`. The spawn-and-pipe shape follows the existing `run_with_a_broken_pipe` helper in that file — read it first (`std::process::Command::new(assert_cmd::cargo::cargo_bin("nova"))` with piped stdio) and match its style.

```rust
/// The benchmark's two halves still work together: the Nova server starts,
/// prints its port, and `nova-bench-http` drives keep-alive requests against
/// it with no errors.
///
/// **This asserts no throughput number**, so it cannot flake on timing. One
/// connection for one second is enough to prove the halves connect; the real
/// measurement is taken by hand, per `docs/benchmarks/README.md`.
///
/// It is a normal test rather than `#[ignore]`d on purpose: CI's Test job has
/// an advisory step that runs exactly the ignored tests, so a load test placed
/// there would run on every push inside a step whose failures are tolerated and
/// therefore unread.
///
/// Uses `nova run` rather than a built binary because this checks correctness,
/// not throughput -- JIT warmup is irrelevant here, and a separate build step
/// would only add moving parts. Task 3's procedure builds a native binary
/// instead, and `docs/benchmarks/README.md` says why.
#[test]
fn bench_http_server_and_generator_agree() {
    use std::io::{BufRead, BufReader};
    use std::process::Stdio;

    let mut server = std::process::Command::new(assert_cmd::cargo::cargo_bin("nova"))
        .arg("run")
        .arg(repo_root().join("docs/benchmarks/server.nova"))
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn the benchmark server");

    // The port line is the first thing the server prints, so this read blocks
    // only until it arrives -- and if the program fails to compile, the stream
    // closes and `next()` yields `None`, which fails with a clear message
    // rather than hanging.
    let stdout = server.stdout.take().expect("stdout was piped");
    let first = BufReader::new(stdout)
        .lines()
        .next()
        .and_then(|l| l.ok())
        .unwrap_or_default();
    let port = first
        .rsplit(':')
        .next()
        .and_then(|p| p.trim().parse::<u16>().ok());

    let port = match port {
        Some(p) => p,
        None => {
            let _ = server.kill();
            panic!("server did not print a parseable port line; got {first:?}");
        }
    };

    let out = std::process::Command::new(assert_cmd::cargo::cargo_bin("nova-bench-http"))
        .arg("--addr")
        .arg(format!("127.0.0.1:{port}"))
        .args(["--connections", "1", "--duration", "1", "--warmup", "0"])
        .output()
        .expect("run nova-bench-http");

    let _ = server.kill();
    let _ = server.wait();

    let report = String::from_utf8_lossy(&out.stdout).to_string();
    assert!(
        out.status.success(),
        "generator failed: {}\nstderr: {}",
        report,
        String::from_utf8_lossy(&out.stderr)
    );
    let result = report
        .lines()
        .find(|l| l.starts_with("RESULT "))
        .unwrap_or_else(|| panic!("no RESULT line in generator output: {report:?}"));
    assert!(
        result.contains(" errors=0 "),
        "generator reported errors: {result}"
    );
    let requests: u64 = result
        .split_whitespace()
        .find_map(|f| f.strip_prefix("requests="))
        .and_then(|v| v.parse().ok())
        .unwrap_or(0);
    assert!(requests >= 1, "no request completed: {result}");
}
```

- [ ] **Step 3: Run the smoke test to verify it fails**

```bash
cargo test --locked --package nova-cli bench_http_server_and_generator_agree --no-fail-fast
```

Expected: FAIL. Before Step 1's file exists it fails on the missing path; if you wrote Step 1 first it may fail on a Nova diagnostic instead. **Report which**, and if it is a Nova diagnostic, report the code and message — that is information about the language, not just about this task.

- [ ] **Step 4: Run it to verify it passes**

```bash
cargo build --locked --workspace && cargo test --locked --package nova-cli bench_http_server_and_generator_agree --no-fail-fast
```

Expected: PASS. **Run it at least five times** and report how many passed — this is the increment's exposure to the known Windows flake, and a single green run is not evidence of stability.

- [ ] **Step 5: Mutation 1 — `--self-test` reports a ceiling without driving load**

In `crates/nova-bench-http/src/main.rs`, make `run_load` return a `Report` with a plausible `requests` count without spawning any worker. Run:

```bash
cargo test --locked --package nova-cli bench_http_server_and_generator_agree --no-fail-fast
```

The smoke test **must** fail. If it passes, the smoke test is not exercising the generator and must be strengthened before you revert. **Report what actually happened**, then revert and confirm green.

- [ ] **Step 6: Mutation 2 — the server answers without reading the request**

In `docs/benchmarks/server.nova`, remove the `read_request` call from `serve`'s loop and write the response unconditionally. Run the same test. It **must** fail on a request count or an error count — with no request consumed, the keep-alive stream desyncs and the generator's reader will not find a clean response boundary. **Report the observed failure mode rather than the predicted one**, then revert and confirm green.

- [ ] **Step 7: Full suite, lint, format, byte-scan, commit**

```bash
cargo build --locked --workspace && cargo test --locked --workspace --no-fail-fast
cargo clippy --locked --workspace --all-targets -- -D warnings
cargo fmt --all -- --check
```

Sum every `test result:` line. Expected: Task 1's total plus one, 0 failed, 8 ignored. Byte-scan the staged files — `server.nova` must contain **zero real CR bytes** and its CRLFs must be in-Nova `\r\n` escapes. Then commit.

Subject: `feat(bench): serve a fixed response from Nova and smoke-test both halves`. The body records the three runtime properties that shaped the server, that the response bytes are hoisted and therefore excluded from any figure, why the smoke test asserts no throughput number and is not `#[ignore]`d, how many times you ran it, and what mutations 1 and 2 actually did.

---

## Task 3: The methodology, and the measurement

**Files:**
- Create: `docs/benchmarks/README.md`
- Create: `docs/benchmarks/http-fixed-response.md`

**Interfaces:**
- Consumes: the `nova-bench-http` binary and its `RESULT` line from Task 1; `docs/benchmarks/server.nova` from Task 2.
- Produces: nothing later tasks consume. Task 4 cites both files.

**Read "THE DECISIVE CONSTRAINT" at the top of this plan before starting.** The naturally-reached-for commands measure a debug runtime and produce a misleading figure.

- [ ] **Step 1: Write the procedure**

Create `docs/benchmarks/README.md` with the Write tool. It must contain:

**What is measured, and what is not.** The read-and-parse path of `std/http`, excluding response serialisation because `server.nova` hoists the response bytes out of its loop. Say plainly that the figure is a ceiling for a real server rather than a simulation of one, and that measuring the delta by not hoisting is a follow-up nobody has done.

**The build configuration, with the table from this plan's "DECISIVE CONSTRAINT" section reproduced.** State that `clang` and `llc` were absent on the host that took the recorded run, so the LLVM figure is unmeasured, and that this bears on reading the result against the gate's 10k.

**The commands, in order:**

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

**What must be recorded beside any number:** CPU model and core count, OS, `rustc --version`, the commit SHA, the code backend, the runtime profile, the self-test ceiling, the Nova figure, and the ratio between them.

**That calibration is mandatory, not advisory.** A Nova figure without its harness ceiling is not a measurement: a reading of 5k could be Nova's limit or the generator's, and no care in the prose distinguishes them. This document does not present one without the other.

**Where the gate is specified inconsistently**, from spec §2 — whose own roster is what was found rather than a claim that nothing else is: the criterion appears twice and the two are not equivalent (10k absolute versus a ratio of at least 1.0 against Bun, which can disagree in either direction); the destination appears twice and `examples/05-json-api/BENCHMARK.md` is unsatisfiable because that directory does not exist; and the named tool `wrk` does not run on this project's Windows host. **State that figures from `nova-bench-http` are not directly comparable to published `wrk` numbers.**

**The known properties:** single-core throughput, because ADR 0009 makes single-threading a correctness requirement rather than a current limitation; one Nova task per connection, so 200 connections is 200 tasks on one thread; no read timeout; the server does not exit.

**Cap connections below roughly 1000 and say why:** the poller's `FD_SETSIZE` rejection path is documented in `crates/nova-runtime/src/poll.rs` as *"still reasoned, not measured"*, so above that the run enters untested territory. Windows uses `WSAPoll` and is not bound this way.

- [ ] **Step 2: Take the measurement**

Follow your own procedure exactly as written. If a step does not work, **fix the procedure and start over** — a procedure that its own author had to deviate from is not reproducible.

Record everything the README says to record. Use `--connections 200 --duration 30 --warmup 5` for both runs so the ceiling and the figure are directly comparable.

- [ ] **Step 3: Write the observation**

Create `docs/benchmarks/http-fixed-response.md`: one dated run, the hardware, the OS, the rustc version, the commit SHA, the backend, the runtime profile, the self-test ceiling, the Nova figure, the ratio, and the raw `RESULT` lines from both runs.

**Report the number you got, whatever it is.** A figure below 10k satisfies this increment's success criteria — the deliverable is a reproducible methodology and an honest number, and where it lands is the finding. Do not tune anything to improve it; optimisation is explicitly out of scope.

If the Nova figure is close to the self-test ceiling, say so: that would mean the generator is the binding constraint and the Nova figure is a lower bound rather than a measurement of Nova.

- [ ] **Step 4: Byte-scan and commit**

Byte-scan both files from staged content. Then commit.

Subject: `docs(benchmarks): record the procedure and the first measured number`. The body carries the figure, the ceiling, the ratio, the backend and runtime profile, that the number excludes response serialisation, and that a figure below 10k is the finding rather than a failure.

---

## Task 4: The records

**Files:**
- Modify: `nova-spec/60-EXAMPLES.md`
- Modify: `nova-spec/00-MASTER-SPEC.md`
- Modify: `CHANGELOG.md`

**Interfaces:**
- Consumes: everything Tasks 1 through 3 shipped, including the measured figure.
- Produces: nothing.

- [ ] **Step 1: Amend `nova-spec/60-EXAMPLES.md` §5**

Insert a dated amendment below the `## 5.` heading and above its code block, in the format that file already uses (`**AMENDED 2026-09-02 (branch \`phase-2-gate-benchmark\`):** ...`). **Amend; do not rewrite the body** — it stays as the aspiration it always was.

It must record: that the listing is written in a Nova that does not exist, naming what it needs and what is measurably absent (`@derive` does not exist; `Map` has `keys()` and no `values()`; the language has no `String`-to-number conversion; the `Handler` type is `P0001`; there is no `?` operator and no turbofish); that its `BENCHMARK.md` destination is unsatisfiable until that example exists, so the number lives in `docs/benchmarks/`; and that its `wrk -t8 -c200 -d30s` methodology does not run on this project's Windows host, naming the generators checked and that `wrk` is POSIX-only.

- [ ] **Step 2: Amend `nova-spec/00-MASTER-SPEC.md` §3**

A dated amendment recording that the gate is specified twice with non-equivalent criteria — 10k+ req/sec absolute here, a ratio of at least 1.0 against Bun in `60-EXAMPLES.md` §5 — and that those can disagree in either direction. Record that this increment measures the absolute figure on one host and leaves the Bun ratio unmeasured, and point at `docs/benchmarks/` for both the procedure and the number.

- [ ] **Step 3: Add the CHANGELOG entry**

Under `[Unreleased]`, in the style of the entries already there — dense, naming measured numbers, and saying what was deliberately not done. It must cover: the new `nova-bench-http` crate with no dependency; `docs/benchmarks/` and what it contains; the measured figure, its ceiling, and their ratio; that the number excludes response serialisation and why; that only release-`nova` plus Cranelift is measurable on this host and the LLVM figure is unmeasured; that a figure below 10k is the finding rather than a failure; and that **no claim is made that the Phase 2 gate is passed**.

- [ ] **Step 4: Sweep for claims this increment made stale**

grep is line-oriented, so **a miss is not evidence of absence** — sweep with whitespace-tolerant patterns that also normalise `//`, `///` and `>` gutters. A plain line-oriented grep returned zero hits on a phrase that existed, twice, on the previous branch.

Sweep `nova-spec/`, `docs/`, `std/`, `crates/` and `CHANGELOG.md` for: any sentence saying `docs/benchmarks/` does not exist or that the gate's methodology is undocumented; any claim that no benchmark of `std/http` exists; any bare count of workspace crates or benchmark targets; and any roster of `examples/` that this increment's records now contradict.

**Fix every artifact the sweep names, not only the one you happen to be editing** — the recurring failure on the previous branch was fixing the artifact in hand and leaving the tracked ones repeating it. Report what you searched for, with the patterns, what you found, and what you changed. If a category returns nothing, **say which pattern you used** so a reader can judge whether the miss is meaningful.

- [ ] **Step 5: Byte-scan and commit**

Byte-scan every file written, from staged content, including the planted-positive assertion for the backslash-u pattern. Confirm no branch-local SHA appears in any tracked file, deriving the roster with `git log --format=%h main..HEAD`. Then commit.

Subject: `docs(benchmarks): amend the gate's records for what was measured`. The body says which two spec sections were amended and how, what the sweep found, and that the example-numbering drift stays recorded and untouched.

---

## Self-Review

Run against the spec after the plan was complete.

**1. Spec coverage.** §1 (scope, and the four exclusions): the plan's File Structure note and Task 4 Step 1. §2 (the three inconsistencies): Task 3 Step 1 and Task 4 Steps 1-2. §3 (architecture, placement, the procedure/observation split): File Structure and Tasks 1-3. §4 (the generator, its flags, its report, calibration as a mode): Task 1. §5 (the server, the hoisting decision, the three runtime properties): Task 2. §6 (the build configuration): its own section at the top of the plan, referenced from Task 3. §7 (the methodology's contents): Task 3 Step 1, item by item. §8 (testing and the three mutations): Task 2 Steps 2-6 and Task 1 Step 7. §9 (failure modes): the Global Constraints flake note, Task 2's runtime properties, Task 3 Step 1's `FD_SETSIZE` cap. §10 (success criteria): criterion 1 is Task 1 Step 1; 2 is Task 1 Step 6 and Task 2 Step 5; 3 is Task 2 Step 4; 4 is Task 3 Step 1; 5 is Task 3 Step 3; 6 is every task's suite step. §11 (records): Task 4.

**No gaps found**, with one thing flagged to the executor rather than hidden: spec §4 lists a `--warmup` flag and spec §7's command sequence uses it, but the smoke test passes `--warmup 0` deliberately, because a warmup in a one-second correctness check would consume the whole run. That is a deliberate asymmetry, not a contradiction, and Task 2's test carries it explicitly.

**2. Placeholder scan.** One value is deliberately unknown at plan time: the measured figure in Task 3, which the plan instructs the executor to record rather than predict. That is an instruction to measure, not a placeholder — the alternative would be inventing a number for the executor to bend results toward, which is the failure the previous branch's "measure, then fill in" goldens exist to prevent. Every other step carries its actual content.

**3. Type consistency.** `head_end`, `content_length`, `response_len`, `Config::from_args`, `Report`, `worker`, `run_load` and `spawn_self_test_server` are named identically in Task 1's tests and its implementation. The `RESULT` line's fields — `mode`, `addr`, `connections`, `requests`, `errors`, `elapsed_ms`, `rps`, `conn_min`, `conn_max` — are produced by Task 1's `main` and parsed by Task 2's smoke test, which reads `errors=` and `requests=` with the exact spellings emitted. `docs/benchmarks/server.nova`'s printed line is `listening on 127.0.0.1:<port>`, and Task 2's test parses the port by splitting on the last `:`, which that shape satisfies.

---

## Execution Handoff

Plan complete and saved to `docs/superpowers/plans/2026-09-02-phase-2-gate-benchmark.md`. Two execution options:

**1. Subagent-Driven (recommended)** — a fresh subagent per task, with a spec-compliance and code-quality review between tasks, and a broad whole-branch review at the end.

**2. Inline Execution** — execute the tasks in this session using `superpowers:executing-plans`, batching with checkpoints for review.

Which approach?
