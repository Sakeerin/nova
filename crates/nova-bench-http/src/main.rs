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

/// The bytes every worker sends, for the configured `path`. HTTP/1.1 keeps the
/// connection open by default, which is the point: `--connections N` means N
/// connections carrying many requests each, not N requests.
///
/// The path is a flag rather than a constant because a target that routes
/// answers different paths differently, so a hardcoded path decides which of
/// its handlers gets measured. `docs/benchmarks/server.nova` answers every
/// path with one fixed response, which is why `/` was enough for it and is
/// still the default; `examples/05-json-api` answers `GET /` with a 404, so a
/// run against it at the default path measures its not-found handler rather
/// than the route the gate's methodology names.
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
    if cfg.body_bytes > 0 {
        s.push_str(&format!("Content-Length: {}\r\n", cfg.body_bytes));
    }
    s.push_str("\r\n");
    let mut v = s.into_bytes();
    v.resize(v.len() + cfg.body_bytes, b'x');
    v
}

/// Per-read and per-write socket timeout for every connection.
///
/// Ten seconds cannot fire against a healthy server: measured against the
/// in-process `--self-test` server, one request-response round trip runs in
/// tens of microseconds -- roughly 80 at four connections -- which leaves
/// about five orders of magnitude of margin. Read the `RESULT` line's `rps`
/// as the AGGREGATE across connections, not a per-connection rate: taking it
/// for one connection's throughput overstates that by the connection count,
/// and an earlier draft of this comment did exactly that.
/// What it guards against is a target that accepts a connection and then
/// stalls mid-response rather than closing it -- without this, `stream.read`
/// blocks forever, the worker never reaches its `stop` check, and `run_load`
/// hangs past `--duration` with no diagnostic and no `RESULT` line. Ten
/// seconds converts that unbounded hang into one counted error.
const IO_TIMEOUT: Duration = Duration::from_secs(10);

/// Upper bound on `--body-bytes`, matching `std/http`'s
/// `Limits::default().max_body_bytes`. Above it the target answers
/// `BodyTooLarge` and every response counts as an error, so nothing
/// measurable lies up there; bounding here also keeps an operator's typo
/// from being allocated before any server sees it. Inclusive, so a sweep
/// can reach the bound itself.
const MAX_BODY_BYTES: usize = 1_048_576;

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

/// The status code off the front of a response, or `None` when those bytes are
/// not `HTTP/<version> <three digits>`.
///
/// This is deliberately NOT part of `response_len`. That function answers "how
/// many bytes is this response", and a length is not a verdict on the response
/// -- folding a status into it would make one function return two unrelated
/// facts, and its `None` would then mean either "not complete yet" or "not
/// successful", which the read loop must tell apart. Both read the same bytes
/// the caller already holds, so keeping them separate costs no extra read.
fn status_code(response: &[u8]) -> Option<u16> {
    const PREFIX: &[u8] = b"HTTP/";
    if !response.starts_with(PREFIX) {
        return None;
    }
    let after_prefix = &response[PREFIX.len()..];
    // The version runs to the first space and the code is the three bytes
    // after it. `get` reports a short line as `None` rather than panicking on
    // the slice, which matters because this runs on whatever a peer sent.
    let space = after_prefix.iter().position(|&b| b == b' ')?;
    let digits = after_prefix.get(space + 1..space + 4)?;
    if !digits.iter().all(u8::is_ascii_digit) {
        return None;
    }
    std::str::from_utf8(digits).ok()?.parse().ok()
}

/// Whether a response's status line reports 2xx.
///
/// A status line this cannot read is not a success: a peer answering something
/// unparseable is not a peer whose answers should be counted as good.
fn is_success(response: &[u8]) -> bool {
    matches!(status_code(response), Some(code) if (200..300).contains(&code))
}

struct Config {
    addr: String,
    path: String,
    /// Extra header lines, each emitted verbatim after `Host`. Empty by
    /// default, which is what keeps the default request byte-identical to
    /// the one every already-recorded figure was taken with.
    headers: Vec<String>,
    /// Body length in bytes; `0` means no body and no `Content-Length`.
    body_bytes: usize,
    connections: usize,
    duration: Duration,
    warmup: Duration,
    self_test: bool,
}

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

impl Config {
    fn from_args<I: Iterator<Item = String>>(args: I) -> Result<Config, String> {
        let mut addr: Option<String> = None;
        // `/` rather than a required flag: it is the request every
        // observation `docs/benchmarks/http-fixed-response.md` already held
        // was taken with, so defaulting to it left those figures describing
        // the same run they always did.
        let mut path = "/".to_string();
        let mut headers: Vec<String> = Vec::new();
        let mut body_bytes = 0usize;
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
                // The `other` arm below rejects anything unrecognised, so a
                // flag that is not listed here is refused rather than ignored.
                "--path" => path = take("--path")?,
                "--header" => {
                    let h = take("--header")?;
                    validate_header(&h)?;
                    headers.push(h);
                }
                "--body-bytes" => {
                    body_bytes = take("--body-bytes")?
                        .parse()
                        .map_err(|_| "--body-bytes must be a non-negative integer".to_string())?;
                }
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
        if !path.starts_with('/') {
            return Err("--path must begin with /".to_string());
        }
        // A CR or LF here would terminate the request line `request_bytes`
        // builds and turn the rest of the value into forged header lines, on
        // every request of the run -- so the run would measure a request
        // shape other than the one its own `--path` record names, with
        // nothing in the `RESULT` line to show it. The value is
        // operator-supplied rather than attacker-supplied, so this is
        // hygiene: it costs one scan and closes the one way a flag can
        // silently change what was measured. It is deliberately NOT a full
        // request-target check -- a path carrying a space still produces a
        // malformed request line, and that one fails loudly, because the
        // target answers outside 2xx and `errors` counts it.
        if path.contains('\r') || path.contains('\n') {
            return Err("--path must contain no CR and no LF".to_string());
        }
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
        Ok(Config {
            addr: addr.unwrap_or_default(),
            path,
            headers,
            body_bytes,
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
fn worker(addr: &str, request: &[u8], stop: &AtomicBool, timeout: Duration) -> (u64, u64) {
    let mut stream = match TcpStream::connect(addr) {
        Ok(s) => s,
        Err(_) => return (0, 1),
    };
    if stream.set_nodelay(true).is_err() {
        return (0, 1);
    }
    // A peer that accepts and then stalls -- rather than closing -- must
    // surface as a counted error, not an unbounded block. The `Err(_)` arm
    // already in the read loop below counts and returns on a timeout, so the
    // loop itself needs no change; only arming the timeout does.
    if stream.set_read_timeout(Some(timeout)).is_err() {
        return (0, 1);
    }
    // **This timeout is reachable, and NO test in this project's suite
    // exercises it.** "One request outstanding at a time" is not a property
    // this worker can guarantee on its own: it depends on the peer *reading*
    // what it is sent. A peer that writes responses without ever reading the
    // requests leaves them piling up in its own receive buffer, and once that
    // buffer is full, TCP backpressure reaches this `write_all` and it
    // blocks. That peer is the design spec's mutation 2 -- "make the server
    // write a response without reading the request" -- and running that
    // mutation during this increment's Task 2 drove exactly this call: the
    // smoke test failed on `errors=1` with `elapsed_ms=10071` for a
    // `--duration 1` run, which is this ten-second timeout firing and turning
    // an unbounded block into one counted error, the same job the read
    // timeout above does for a stalled read. Removing this call would turn
    // that back into a hang. It also covers a change that writes more per
    // turn, pipelining being the obvious one.
    if stream.set_write_timeout(Some(timeout)).is_err() {
        return (0, 1);
    }
    let mut requests = 0u64;
    let mut errors = 0u64;
    let mut buf: Vec<u8> = Vec::with_capacity(4096);
    let mut chunk = [0u8; 4096];
    while !stop.load(Ordering::Relaxed) {
        if stream.write_all(request).is_err() {
            errors += 1;
            break;
        }
        // Consume exactly one response. Reading less desyncs the stream; reading
        // more eats the next response. See `response_len`.
        loop {
            if let Some(n) = response_len(&buf) {
                // The status verdict is recorded HERE, and not in
                // `response_len` or `main`, because this is the one place that
                // holds both a complete response and the `errors` counter. A
                // non-2xx increments the same counter a failed write does, so
                // a single `errors=` figure covers a run aimed at a route the
                // target does not serve and a target answering 5xx under load,
                // rather than needing a second column nobody would read.
                //
                // The round trip DID complete, so `requests` counts it too:
                // `rps` stays a round-trip rate, and a wholly wrong route
                // shows up as an error count near the request count rather
                // than as a throughput figure that quietly drops to zero.
                if !is_success(&buf[..n]) {
                    errors += 1;
                }
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

fn run_load(
    addr: &str,
    request: &[u8],
    connections: usize,
    duration: Duration,
    timeout: Duration,
) -> Report {
    let stop = Arc::new(AtomicBool::new(false));
    // One buffer for the whole run: every worker writes the same bytes and
    // none of them mutates it, so the request is shared with the workers
    // rather than copied per connection.
    let request = Arc::new(request.to_vec());
    let start = Instant::now();
    let handles: Vec<_> = (0..connections)
        .map(|_| {
            let stop = Arc::clone(&stop);
            let request = Arc::clone(&request);
            let addr = addr.to_string();
            std::thread::spawn(move || worker(&addr, &request, &stop, timeout))
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
    let listener = TcpListener::bind("127.0.0.1:0").map_err(|e| format!("self-test bind: {e}"))?;
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
                 [--path PATH] [--connections N] [--duration SECS] [--warmup SECS]"
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

    // Built once, before the warmup, so the warmup and the measurement send
    // byte-identical requests.
    let request = request_bytes(&cfg);

    if !cfg.warmup.is_zero() {
        let w = run_load(&addr, &request, cfg.connections, cfg.warmup, IO_TIMEOUT);
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

    let r = run_load(&addr, &request, cfg.connections, cfg.duration, IO_TIMEOUT);
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn head_end_is_found_only_on_a_complete_terminator() {
        assert_eq!(head_end(b"HTTP/1.1 200 OK\r\n\r\nbody"), Some(19));
        assert_eq!(
            head_end(b"HTTP/1.1 200 OK\r\n\r"),
            None,
            "a split terminator is not an end"
        );
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
        assert_eq!(
            content_length(b"HTTP/1.1 200 OK\r\nContent-Length: abc\r\n\r\n"),
            None
        );
    }

    #[test]
    fn a_response_is_complete_only_when_head_and_body_are_both_present() {
        let full = b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\n\r\nhi";
        assert_eq!(response_len(full), Some(full.len()));
        assert_eq!(
            response_len(&full[..full.len() - 1]),
            None,
            "one byte short is not complete"
        );
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

    #[test]
    fn body_bytes_is_validated_and_framed() {
        let args = |a: &[&str]| Config::from_args(a.iter().map(|s| s.to_string()));

        assert_eq!(
            args(&["--self-test"]).expect("valid").body_bytes,
            0,
            "no body by default, which keeps the default request the 36 bytes \
             every already-recorded figure was taken with"
        );
        assert_eq!(
            args(&["--self-test", "--body-bytes", "0"])
                .expect("0 is valid")
                .body_bytes,
            0
        );
        assert_eq!(
            args(&["--self-test", "--body-bytes", "7"])
                .expect("7 is valid")
                .body_bytes,
            7
        );

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
            args(&[
                "--self-test",
                "--body-bytes",
                "4",
                "--header",
                "content-length:4"
            ])
            .is_err(),
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

    #[test]
    fn the_path_defaults_to_root_and_is_validated() {
        let args = |a: &[&str]| Config::from_args(a.iter().map(|s| s.to_string()));
        let dflt = args(&["--self-test"]).expect("--self-test alone is valid");
        assert_eq!(
            dflt.path, "/",
            "the default is what keeps every already-recorded figure valid"
        );
        let given = args(&["--self-test", "--path", "/users"]).expect("--path /users is valid");
        assert_eq!(given.path, "/users");
        assert!(
            args(&["--self-test", "--path", "users"]).is_err(),
            "a path without a leading slash would send a malformed request line"
        );
        assert!(args(&["--self-test", "--path"]).is_err());
        // Both bytes, both separately: a CRLF pair would be rejected by
        // either check alone, so testing only the pair cannot tell which
        // half is doing the work.
        assert!(
            args(&["--self-test", "--path", "/a\rb"]).is_err(),
            "a CR would forge header lines into every request of the run"
        );
        assert!(
            args(&["--self-test", "--path", "/a\nb"]).is_err(),
            "an LF would forge header lines into every request of the run"
        );
        assert!(
            args(&["--self-test", "--path", "/a b"]).is_ok(),
            "the check is CR and LF, not a full request-target grammar: a \
             space is still accepted here and fails loudly at the target"
        );
    }

    #[test]
    fn a_status_code_is_read_off_the_front_of_a_response() {
        assert_eq!(status_code(b"HTTP/1.1 200 OK\r\n\r\n"), Some(200));
        assert_eq!(status_code(b"HTTP/1.1 404 Not Found\r\n\r\n"), Some(404));
        assert_eq!(status_code(b"HTTP/1.0 204 No Content\r\n\r\n"), Some(204));
        assert_eq!(
            status_code(b"HTTP/1.1 20 OK\r\n\r\n"),
            None,
            "a two-digit code is not a status line"
        );
        assert_eq!(status_code(b"200 OK\r\n\r\n"), None);
        assert_eq!(
            status_code(b"HTTP/1.1"),
            None,
            "a line that stops short must report None rather than panic on the slice"
        );
        assert_eq!(status_code(b""), None);
    }

    #[test]
    fn only_a_2xx_status_is_a_success() {
        assert!(is_success(b"HTTP/1.1 200 OK\r\n\r\n"));
        assert!(is_success(b"HTTP/1.1 201 Created\r\n\r\n"));
        assert!(is_success(b"HTTP/1.1 299 Whatever\r\n\r\n"));
        assert!(!is_success(b"HTTP/1.1 199 Early\r\n\r\n"));
        assert!(!is_success(b"HTTP/1.1 300 Multiple Choices\r\n\r\n"));
        assert!(!is_success(b"HTTP/1.1 404 Not Found\r\n\r\n"));
        assert!(!is_success(b"HTTP/1.1 500 Internal Server Error\r\n\r\n"));
        assert!(
            !is_success(b"nonsense\r\n\r\n"),
            "a status line this cannot read is not a success"
        );
    }

    /// Drive one worker against a server that answers `status_line` once and
    /// then closes, as `(requests, errors)`.
    ///
    /// The close is what ends the worker without needing a timer to set
    /// `stop`: the next write or read fails, which the worker counts and
    /// returns on. It is a GRACEFUL close because the request head is read in
    /// full first -- closing over unread bytes can reset the connection and
    /// take the response with it.
    fn one_answer_from(status_line: &str) -> (u64, u64) {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind an ephemeral port");
        let addr = listener
            .local_addr()
            .expect("read the bound port")
            .to_string();
        let status_line = status_line.to_string();
        std::thread::spawn(move || {
            let Ok((mut conn, _)) = listener.accept() else {
                return;
            };
            let body = b"{}";
            let mut buf: Vec<u8> = Vec::with_capacity(4096);
            let mut chunk = [0u8; 4096];
            while head_end(&buf).is_none() {
                match conn.read(&mut chunk) {
                    Ok(0) | Err(_) => return,
                    Ok(n) => buf.extend_from_slice(&chunk[..n]),
                }
            }
            let head = format!("{status_line}\r\ncontent-length: {}\r\n\r\n", body.len());
            let _ = conn.write_all(head.as_bytes());
            let _ = conn.write_all(body);
        });
        let stop = AtomicBool::new(false);
        let cfg = Config::from_args(["--self-test"].iter().map(|s| s.to_string()))
            .expect("--self-test alone is valid");
        worker(&addr, &request_bytes(&cfg), &stop, Duration::from_secs(5))
    }

    #[test]
    fn a_non_2xx_answer_is_one_more_error_than_the_same_answer_at_2xx() {
        // The two runs differ in nothing but the status line, so the gap
        // between their error counts is the status check and nothing else.
        // Both also count the closed connection as an error, which is why this
        // asserts the difference rather than either figure alone.
        let (ok_requests, ok_errors) = one_answer_from("HTTP/1.1 200 OK");
        let (nf_requests, nf_errors) = one_answer_from("HTTP/1.1 404 Not Found");
        assert_eq!(ok_requests, 1);
        assert_eq!(nf_requests, 1, "a 404 still completes a round trip");
        assert_eq!(
            nf_errors,
            ok_errors + 1,
            "a non-2xx answer must be counted; got {ok_errors} for the 200 and \
             {nf_errors} for the 404"
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

    #[test]
    fn a_stalled_connection_times_out_rather_than_hanging_forever() {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind an ephemeral port");
        let addr = listener
            .local_addr()
            .expect("read the bound port")
            .to_string();
        std::thread::spawn(move || {
            let Ok((_conn, _)) = listener.accept() else {
                return;
            };
            // Hold the connection open without writing a response or
            // dropping it: dropping would close the socket, and the worker
            // would then see `Ok(0)` (a clean close) rather than a timeout.
            // Parking forever keeps `_conn` alive for as long as this thread
            // runs; nothing joins this thread, so it is detached exactly
            // like `spawn_self_test_server`'s per-connection threads, and
            // the process exits at the end of the test binary regardless of
            // how long the park lasts.
            loop {
                std::thread::park();
            }
        });
        let stop = AtomicBool::new(false);
        let start = Instant::now();
        let cfg = Config::from_args(["--self-test"].iter().map(|s| s.to_string()))
            .expect("--self-test alone is valid");
        let (requests, errors) = worker(
            &addr,
            &request_bytes(&cfg),
            &stop,
            Duration::from_millis(200),
        );
        let elapsed = start.elapsed();
        assert_eq!(requests, 0);
        assert!(
            errors > 0,
            "a stalled connection must count as an error, not hang silently"
        );
        assert!(
            elapsed < Duration::from_secs(3),
            "the read timeout should fire in well under 3s; took {elapsed:?} instead"
        );
    }
}
