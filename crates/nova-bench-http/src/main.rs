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
