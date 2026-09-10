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
fn request_bytes(path: &str) -> Vec<u8> {
    format!("GET {path} HTTP/1.1\r\nHost: nova-bench\r\n\r\n").into_bytes()
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
    connections: usize,
    duration: Duration,
    warmup: Duration,
    self_test: bool,
}

impl Config {
    fn from_args<I: Iterator<Item = String>>(args: I) -> Result<Config, String> {
        let mut addr: Option<String> = None;
        // `/` rather than a required flag: it is the request every
        // observation `docs/benchmarks/http-fixed-response.md` already held
        // was taken with, so defaulting to it left those figures describing
        // the same run they always did.
        let mut path = "/".to_string();
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
        Ok(Config {
            addr: addr.unwrap_or_default(),
            path,
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
    let request = request_bytes(&cfg.path);

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
        assert_eq!(
            request_bytes("/users"),
            b"GET /users HTTP/1.1\r\nHost: nova-bench\r\n\r\n".to_vec()
        );
        assert_eq!(
            request_bytes("/"),
            b"GET / HTTP/1.1\r\nHost: nova-bench\r\n\r\n".to_vec(),
            "the default path's request is the 36 bytes docs/benchmarks/README.md describes"
        );
    }

    #[test]
    fn the_path_defaults_to_root_and_must_begin_with_a_slash() {
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
        worker(&addr, &request_bytes("/"), &stop, Duration::from_secs(5))
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
        let (requests, errors) = worker(
            &addr,
            &request_bytes("/"),
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
