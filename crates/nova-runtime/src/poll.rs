//! The executor's third wake source: socket readiness.
//!
//! Before this module existed, `task.rs`'s executor had exactly two wake
//! sources -- a deadline passing and a task completing -- and "nothing left
//! to run" meant sleeping the whole thread on the earliest deadline
//! (`std::thread::sleep`, called directly from `task.rs`). This module gives
//! `task.rs` a single place to ask "wake me when a deadline passes, a socket
//! becomes ready, or whichever comes first", so the executor no longer knows
//! how to wait -- it only knows how to ask [`wait`] to do it.
//!
//! **This module now ships a real poller** (`select` on Unix, `WSAPoll` on
//! Windows, behind one `#[cfg]` seam) -- added after the module's first
//! version shipped only [`RawSocket`] and [`Interest`] so `task.rs`'s
//! `Wait::Io` variant had something to name a socket and an interest with,
//! and made [`wait`]'s non-empty-set branch behave exactly like the old
//! `thread::sleep`-based timer path (return nothing ready, always). Nothing
//! in `task.rs` had to change to pick up the real poller, because `task.rs`
//! already only calls [`wait`] and never sleeps itself.
//!
//! **Still true today:** no caller in this crate ever builds a non-empty
//! `sockets` slice yet -- `net.rs` (Task 3) is this module's first caller
//! that will. Until then the non-empty path is exercised only by this
//! module's own tests, directly, against real loopback sockets.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::OnceLock;
use std::time::{Duration, Instant};

/// A socket as the poller sees it: the OS handle, widened to `i64` so
/// `task.rs` can hold one in `Wait` without depending on this module's
/// platform types, and so a future `net.rs` (Task 3) and `task.rs` agree on
/// one socket representation without either depending on the other's.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[repr(transparent)]
pub struct RawSocket(pub i64);

/// What a task is waiting for a socket to become ready for.
///
/// `#[allow(dead_code)]`: `wait` below only *matches* on these variants, and
/// a match is not a construction, so the non-test build still never builds
/// one -- `net.rs` (Task 3) is this enum's first production caller, staging a
/// `Wait::Io` with a real interest. The seam is added now, not deleted and
/// re-added later, so `Wait::Io` (`task.rs`) has a real type to name an
/// interest with today.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[allow(dead_code)]
pub enum Interest {
    Read,
    Write,
}

/// Wait until one of `sockets` becomes ready for its paired [`Interest`], or
/// `deadline` passes, whichever happens first. Returns every socket found
/// ready; empty if the deadline fired instead, or if `sockets` is empty.
///
/// **No deadline and a non-empty `sockets` blocks indefinitely** until a
/// socket is ready -- there is nothing else for this function to wait on, and
/// a task parked on I/O with no deadline is not this module's problem (see
/// `task.rs`'s `deadlock_report`, which does not treat that as a deadlock).
///
/// This function must not panic: it runs from `task.rs`'s drive loop, between
/// polls, where nothing catches an unwind. No `unwrap`/`expect`/indexing/
/// fallible `format!` here or in anything it calls.
///
/// **A known, reported limitation, not an oversight:** a persistent
/// (non-`EINTR`) failure from the platform poll call, or a socket the
/// platform's primitive cannot represent at all (see the Unix arm's
/// `FD_SETSIZE` note), makes this function report "nothing ready" and back
/// off rather than spin -- but `Vec<RawSocket>` has no channel to say *which*
/// socket is broken, so a caller cannot evict it from its own park set.
/// Closing that needs either a richer return type (e.g. separating "ready"
/// from "errored" sockets) or an out-of-band failure report, and no caller
/// exists yet (`net.rs`, Task 3) to say which shape it actually needs.
/// `tracing::warn!` logs the condition in the meantime, so it is at least
/// diagnosable rather than silent.
pub fn wait(sockets: &[(RawSocket, Interest)], deadline: Option<Instant>) -> Vec<RawSocket> {
    if sockets.is_empty() {
        if let Some(at) = deadline {
            let now = Instant::now();
            if at > now {
                std::thread::sleep(at - now);
            }
        }
        return Vec::new();
    }
    platform_wait(sockets, deadline)
}

/// The time remaining until `deadline`, clamped so it is never negative.
///
/// Nova's `Int` is signed and a timed wait built from it can carry a deadline
/// that is already in the past (mirrors `task.rs`'s `deadline_from_ms`, which
/// clamps the same way for the timer-only path). `None` means "no deadline":
/// block indefinitely rather than returning a zero timeout, which would make
/// an I/O wait busy-loop instead of actually waiting.
fn remaining(deadline: Option<Instant>) -> Option<Duration> {
    deadline.map(|at| {
        let now = Instant::now();
        if at > now {
            at - now
        } else {
            Duration::ZERO
        }
    })
}

/// How long a platform arm sleeps, when it has no deadline to defer to,
/// before reporting "nothing ready" for a reason retrying cannot fix on its
/// own (every socket unwatchable, or a persistent non-`EINTR` failure). A
/// task genuinely stuck this way makes no more progress no matter how often
/// `wait` is called again; this constant bounds the *cost* of being stuck --
/// CPU spent busy-looping `task.rs`'s drive loop -- not the stuck condition
/// itself, which only a caller with a channel to evict the bad socket (see
/// `wait`'s own doc comment) could actually resolve.
const ERROR_RETRY_BACKOFF: Duration = Duration::from_millis(10);

/// A fixed point in time this module measures log rate-limiting against,
/// lazily fixed on first use. Only relative elapsed time matters for
/// [`LogGate`], so an arbitrary origin is fine.
fn log_epoch() -> Instant {
    static EPOCH: OnceLock<Instant> = OnceLock::new();
    *EPOCH.get_or_init(Instant::now)
}

/// How often a single `tracing::warn!` call site in this module may log, at
/// most. `wait` can be re-called every [`ERROR_RETRY_BACKOFF`] for a
/// condition that persists indefinitely (see `wait`'s own doc comment), and
/// one call site (the Unix arm's `FD_SETSIZE` check) is reached once per
/// out-of-range socket *inside* a single call besides -- unthrottled, either
/// shape logs without bound for as long as the underlying condition lasts.
const LOG_RATE_LIMIT: Duration = Duration::from_secs(1);

/// Rate-limits one `tracing::warn!` call site to at most one line per
/// [`LOG_RATE_LIMIT`], across however many times or however many sockets
/// reach it within that window -- a cap per call site, not per socket, so
/// this costs a fixed handful of bytes rather than growing with however many
/// distinct sockets happen to trip it over the process's lifetime.
struct LogGate(AtomicU64);

impl LogGate {
    const fn new() -> Self {
        Self(AtomicU64::new(0))
    }

    /// `true` at most once per [`LOG_RATE_LIMIT`]; lock-free, so a
    /// concurrent tie can rarely let two callers both through at once -- an
    /// occasional duplicate line, not a correctness problem for a
    /// diagnostic that must not itself panic or block.
    ///
    /// **Stores the last-allowed timestamp offset by one, so `0` means
    /// "never logged yet" rather than colliding with a real elapsed time of
    /// `0`ms.** Without the offset, a fresh gate's `0` sentinel is
    /// indistinguishable from an actual first call landing at `elapsed() ==
    /// 0` (as one always does: [`log_epoch`] is fixed to `Instant::now()` by
    /// whichever gate touches it first, so *that* gate's own first `elapsed()`
    /// reads back ~0) -- `0.saturating_sub(0) < LOG_RATE_LIMIT` is `true`,
    /// denying the very first call at every site, forever losing the first
    /// (and often most diagnostically valuable) occurrence of whatever this
    /// gates.
    fn allow(&self) -> bool {
        let now_ms = log_epoch().elapsed().as_millis() as u64;
        let prev_raw = self.0.load(Ordering::Relaxed);
        if prev_raw != 0 {
            let prev_ms = prev_raw - 1;
            if now_ms.saturating_sub(prev_ms) < LOG_RATE_LIMIT.as_millis() as u64 {
                return false;
            }
        }
        self.0.store(now_ms.saturating_add(1), Ordering::Relaxed);
        true
    }
}

/// Set `socket` non-blocking, for `net.rs` (Task 3) to call right after a
/// socket is created and before it is ever handed to [`wait`] or connected --
/// the property a non-blocking `connect` depends on.
///
/// `#[allow(dead_code)]`: this task ships no `net.rs`, so nothing in
/// production calls this yet -- `net.rs` (Task 3) is this function's first
/// caller. This module's own tests exercise it in the meantime.
#[allow(dead_code)]
pub fn set_nonblocking(socket: RawSocket) -> std::io::Result<()> {
    platform_set_nonblocking(socket)
}

// ---------------------------------------------------------------------------
// Unix: `select`.
//
// **Not built or run in this task.** No Unix or macOS toolchain was reachable
// from this task's environment (a WSL2 Ubuntu attempt is recorded in the
// task report), so this arm is reasoned from `select(2)`/`libc` semantics,
// not measured -- the same status the design spec already recorded for
// socket pollability off Windows. Using `libc` rather than hand-rolling
// `fd_set` narrows what "reasoned" has to cover: `fd_set`'s bit-mask element
// width differs between glibc (64-bit words) and Darwin (32-bit words), and
// `libc` supplies the per-target layout rather than this module guessing it,
// so the open risk is "does `select` on a loopback socket behave as
// documented on Linux and macOS", not also "did this module's own `fd_set`
// arithmetic match either platform's memory layout".
// ---------------------------------------------------------------------------

#[cfg(unix)]
fn platform_wait(sockets: &[(RawSocket, Interest)], deadline: Option<Instant>) -> Vec<RawSocket> {
    use std::os::unix::io::RawFd;

    // Rate-limit gates for this function's two `tracing::warn!` sites --
    // `static` inside the function body rather than at module scope, since
    // nothing outside this arm needs them and each persists across calls
    // exactly the same way either way. See `LogGate`'s own doc comment.
    static FD_SETSIZE_LOG_GATE: LogGate = LogGate::new();
    static SELECT_ERROR_LOG_GATE: LogGate = LogGate::new();

    loop {
        // SAFETY: `fd_set` is a plain-old-data bit-mask type; zero-initializing
        // it is exactly what `FD_ZERO` does, and both sets are fully
        // initialized by the two `FD_ZERO` calls immediately below before
        // anything reads them.
        let mut read_set: libc::fd_set = unsafe { std::mem::zeroed() };
        let mut write_set: libc::fd_set = unsafe { std::mem::zeroed() };
        // SAFETY: both sets are valid, live `fd_set` values.
        unsafe {
            libc::FD_ZERO(&mut read_set);
            libc::FD_ZERO(&mut write_set);
        }

        let mut max_fd: RawFd = -1;
        let mut any_watched = false;
        for (sock, interest) in sockets {
            let fd = sock.0 as RawFd;
            // A negative fd or one `select` cannot represent in an `fd_set`
            // (`FD_SETSIZE` wide) is skipped rather than handed to `FD_SET`,
            // which is documented UB for an out-of-range fd. `select`'s
            // ceiling is inherent, not this module's to lift -- but a task
            // parked on a socket this skips is never watched by any future
            // call either, so it starves until its own deadline (if it has
            // one) with nothing else in this crate ever explaining why.
            // `tracing::warn!` rather than silence: `wait` must not panic,
            // and there is no channel back to the caller to report this
            // per-socket (see `wait`'s own doc comment). Gated by
            // `FD_SETSIZE_LOG_GATE` because this loop runs once per socket
            // per call: without a gate, N out-of-range sockets would log N
            // lines every single call `wait` makes for as long as they stay
            // parked, which for a caller with no deadline may be forever.
            if fd < 0 || fd as usize >= libc::FD_SETSIZE {
                if FD_SETSIZE_LOG_GATE.allow() {
                    tracing::warn!(
                        raw_socket = sock.0,
                        fd_setsize = libc::FD_SETSIZE,
                        "poll::wait: at least one socket is outside select's \
                         representable range and will never be watched on this \
                         platform (rate-limited: at most one line per second)"
                    );
                }
                continue;
            }
            any_watched = true;
            match interest {
                // SAFETY: `fd` is non-negative and below `FD_SETSIZE`, and
                // `read_set`/`write_set` are live `fd_set` values.
                Interest::Read => unsafe { libc::FD_SET(fd, &mut read_set) },
                Interest::Write => unsafe { libc::FD_SET(fd, &mut write_set) },
            }
            if fd > max_fd {
                max_fd = fd;
            }
        }

        let timeout = remaining(deadline);
        if !any_watched {
            // Every socket was filtered out above (each already checked
            // against `FD_SETSIZE`, and logged if this call's rate-limit
            // gate allowed it -- see `LogGate`); there is nothing for
            // `select` to watch. Back off rather than spin:
            // retrying immediately cannot change the outcome, so bound the
            // cost to `ERROR_RETRY_BACKOFF` when there is no deadline to defer
            // to instead of returning instantly forever.
            std::thread::sleep(timeout.unwrap_or(ERROR_RETRY_BACKOFF));
            return Vec::new();
        }

        let mut tv = libc::timeval {
            tv_sec: 0,
            tv_usec: 0,
        };
        let tv_ptr = match timeout {
            // `None` (no deadline): a null timeout tells `select` to block
            // indefinitely.
            None => std::ptr::null_mut(),
            Some(d) => {
                tv.tv_sec = d.as_secs() as libc::time_t;
                tv.tv_usec = libc::suseconds_t::from(d.subsec_micros() as i32);
                std::ptr::addr_of_mut!(tv)
            }
        };

        // SAFETY: `max_fd + 1` bounds both sets; every fd set into either was
        // checked above to be non-negative and below `FD_SETSIZE`; `tv_ptr`
        // is either null or points at `tv`, which outlives this call.
        let rc = unsafe {
            libc::select(
                max_fd + 1,
                &mut read_set,
                &mut write_set,
                std::ptr::null_mut(),
                tv_ptr,
            )
        };

        if rc < 0 {
            let err = std::io::Error::last_os_error();
            if err.kind() == std::io::ErrorKind::Interrupted {
                // EINTR: a signal interrupted the wait before any socket
                // became ready or the timeout elapsed. Retry with the
                // deadline recomputed above rather than reporting readiness
                // that was never observed.
                continue;
            }
            // Some other failure (e.g. a socket closed out from under this
            // call, detected mid-wait). There is no ready set to trust, and
            // no channel to say *which* socket broke so a caller could evict
            // it (see `wait`'s own doc comment) -- so log what's knowable
            // (rate-limited, for the same reason as the `FD_SETSIZE` site
            // above) and back off rather than spin, exactly like the
            // `!any_watched` case above: retrying immediately cannot fix a
            // persistent failure, only waste CPU doing so.
            //
            // **Recomputed here, not reused from `timeout` above:** `timeout`
            // was bound *before* the blocking `select` call; sleeping that
            // stale value again after `select` already blocked for some
            // (possibly nearly the whole) remaining duration would sleep
            // twice against the same deadline, nearly doubling it in the
            // worst case. `remaining(deadline)` recomputed right here, after
            // the call, is what the deadline actually has left *now*.
            if SELECT_ERROR_LOG_GATE.allow() {
                tracing::warn!(
                    error = %err,
                    "poll::wait: select failed; backing off rather than \
                     spinning (rate-limited: at most one line per second)"
                );
            }
            std::thread::sleep(remaining(deadline).unwrap_or(ERROR_RETRY_BACKOFF));
            return Vec::new();
        }

        let mut ready = Vec::new();
        for (sock, interest) in sockets {
            let fd = sock.0 as RawFd;
            // Same predicate as the watch-set-building loop above, over the
            // same immutable `sockets` slice, within this one call: anything
            // skipped here was already *checked* there moments earlier in
            // this same call -- and logged too, if this call happened to be
            // the one in every `LOG_RATE_LIMIT` window that `LogGate` let
            // through, but not otherwise. Silent here is redundancy with an
            // already-checked condition, not a second, unreported one; it is
            // not a claim that a log line necessarily exists for it.
            if fd < 0 || fd as usize >= libc::FD_SETSIZE {
                continue;
            }
            // SAFETY: same bound as the `FD_SET` calls above.
            let is_ready = match interest {
                Interest::Read => unsafe { libc::FD_ISSET(fd, &read_set) },
                Interest::Write => unsafe { libc::FD_ISSET(fd, &write_set) },
            };
            if is_ready {
                ready.push(*sock);
            }
        }
        return ready;
    }
}

// Dead-code note: see `set_nonblocking`'s own doc comment above -- this is
// reached only through that function today.
#[cfg(unix)]
#[allow(dead_code)]
fn platform_set_nonblocking(socket: RawSocket) -> std::io::Result<()> {
    let fd = socket.0 as std::os::unix::io::RawFd;
    // SAFETY: `fcntl(F_GETFL)` reads flags for `fd`; no buffer is written.
    let flags = unsafe { libc::fcntl(fd, libc::F_GETFL) };
    if flags < 0 {
        return Err(std::io::Error::last_os_error());
    }
    // SAFETY: `fd` and `flags` are both plain integers; this sets `fd`'s
    // flags to `flags` with `O_NONBLOCK` added.
    let rc = unsafe { libc::fcntl(fd, libc::F_SETFL, flags | libc::O_NONBLOCK) };
    if rc < 0 {
        return Err(std::io::Error::last_os_error());
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Windows: `WSAPoll`.
//
// **Verified**: built with `cargo build -p nova-runtime` and exercised for
// real by this module's own tests (`wait_reports_a_socket_with_data_waiting`,
// `wait_reports_a_socket_ready_for_write`,
// `wait_returns_empty_when_the_deadline_passes_first`, and
// `set_nonblocking_makes_a_read_return_would_block_instead_of_blocking`)
// against real loopback `TcpStream`s on this task's own Windows host. Both
// `Interest::Read` and `Interest::Write` are exercised, so both the
// `POLLRDNORM` and `POLLWRNORM` branches ran for real, not just `Read`. This
// is the arm the design spec called out as *reasoned, not measured* for
// socket pollability -- on Windows, it is now measured. The non-`EINTR`
// error/backoff branch below is not exercised by any test (there is no way
// to force `WSAPoll` to fail without a real socket-level fault) and remains
// reasoned rather than measured.
// ---------------------------------------------------------------------------

#[cfg(windows)]
fn platform_wait(sockets: &[(RawSocket, Interest)], deadline: Option<Instant>) -> Vec<RawSocket> {
    use windows_sys::Win32::Networking::WinSock::{
        WSAGetLastError, WSAPoll, POLLRDNORM, POLLWRNORM, SOCKET_ERROR, WSAEINTR, WSAPOLLFD,
    };

    // See the Unix arm's identically-purposed gate for why this exists: a
    // persistent, non-`WSAEINTR` failure can recur on every call `wait`
    // makes for as long as a task stays parked with no deadline, and without
    // this gate that logs without bound.
    static WSAPOLL_ERROR_LOG_GATE: LogGate = LogGate::new();

    loop {
        let mut fds: Vec<WSAPOLLFD> = sockets
            .iter()
            .map(|(sock, interest)| WSAPOLLFD {
                fd: sock.0 as usize,
                events: match interest {
                    Interest::Read => POLLRDNORM,
                    Interest::Write => POLLWRNORM,
                },
                revents: 0,
            })
            .collect();

        let timeout_ms: i32 = match remaining(deadline) {
            // `None` (no deadline): `WSAPoll` blocks indefinitely on a
            // negative timeout.
            None => -1,
            Some(d) => i32::try_from(d.as_millis()).unwrap_or(i32::MAX),
        };

        // SAFETY: `fds` is a live, uniquely-owned buffer of `fds.len()`
        // `WSAPOLLFD` entries for the duration of this call; `WSAPoll` writes
        // only `revents` back into each entry.
        let rc = unsafe { WSAPoll(fds.as_mut_ptr(), fds.len() as u32, timeout_ms) };

        if rc == SOCKET_ERROR {
            // SAFETY: `WSAGetLastError` takes no arguments and just reads
            // thread-local error state.
            let err = unsafe { WSAGetLastError() };
            if err == WSAEINTR {
                // EINTR: retry with the deadline recomputed above rather than
                // reporting readiness that was never observed.
                continue;
            }
            // Same reasoning as the Unix arm's non-EINTR branch: no channel
            // to say which socket broke, so log what's knowable (rate-limited,
            // same reason as the Unix arm's gates) and back off rather than
            // spin. `remaining(deadline)` is recomputed fresh right here,
            // after `WSAPoll` already returned -- not reused from
            // `timeout_ms` above, which was bound *before* the blocking call
            // and would already be stale by this point for the same reason
            // the Unix arm's `rc < 0` branch had to stop reusing its own
            // pre-call value.
            //
            // `WSAGetLastError`'s numeric code is wrapped in an `io::Error`
            // (valid here: on Windows, `WSAGetLastError` is directly mapped
            // to `GetLastError`, which is exactly what `io::Error` formats)
            // so this logs the same shape (`error = %…`) as the Unix arm,
            // rather than a raw integer nothing else in this module prints.
            if WSAPOLL_ERROR_LOG_GATE.allow() {
                let io_err = std::io::Error::from_raw_os_error(err);
                tracing::warn!(
                    error = %io_err,
                    "poll::wait: WSAPoll failed; backing off rather than \
                     spinning (rate-limited: at most one line per second)"
                );
            }
            std::thread::sleep(remaining(deadline).unwrap_or(ERROR_RETRY_BACKOFF));
            return Vec::new();
        }

        let mut ready = Vec::new();
        for (i, (sock, _)) in sockets.iter().enumerate() {
            // `revents` carries error/hangup bits (`POLLERR`/`POLLHUP`/
            // `POLLNVAL`) even when they were not requested in `events` --
            // exactly like `select` folding a hung-up peer into the
            // readable set -- so any non-zero `revents` means this socket
            // needs attention, not only the bit that was asked for.
            if let Some(entry) = fds.get(i) {
                if entry.revents != 0 {
                    ready.push(*sock);
                }
            }
        }
        return ready;
    }
}

// Dead-code note: see `set_nonblocking`'s own doc comment above -- this is
// reached only through that function today.
#[cfg(windows)]
#[allow(dead_code)]
fn platform_set_nonblocking(socket: RawSocket) -> std::io::Result<()> {
    use windows_sys::Win32::Networking::WinSock::{ioctlsocket, FIONBIO, SOCKET_ERROR};

    let sock = socket.0 as usize;
    let mut mode: u32 = 1; // non-zero enables non-blocking mode.
                           // SAFETY: `sock` names a socket handle this call does not take ownership
                           // of; `mode` is a valid, live `u32` for the duration of the call.
    let rc = unsafe { ioctlsocket(sock, FIONBIO, &mut mode) };
    if rc == SOCKET_ERROR {
        return Err(std::io::Error::last_os_error());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_fresh_log_gate_allows_its_first_call() {
        // The bug this pins: `allow()`'s internal timestamp starts at the
        // sentinel `0`, and a fresh gate's very first call can itself land
        // at `elapsed() == 0` (see `allow`'s own doc comment on why) --
        // treating `0` as an ordinary elapsed time rather than "never
        // logged yet" denies this call, silently losing the first
        // occurrence of whatever the gate protects, forever.
        let gate = LogGate::new();
        assert!(
            gate.allow(),
            "a fresh gate's first call must not be swallowed"
        );
    }

    #[test]
    fn a_log_gate_denies_a_second_call_inside_the_rate_limit_window() {
        let gate = LogGate::new();
        assert!(gate.allow(), "first call must be allowed");
        assert!(
            !gate.allow(),
            "a second call inside the rate-limit window must be denied, \
             or the gate is not rate-limiting anything"
        );
    }

    #[test]
    fn a_log_gate_allows_again_once_the_rate_limit_window_elapses() {
        let gate = LogGate::new();
        assert!(gate.allow(), "first call must be allowed");
        std::thread::sleep(LOG_RATE_LIMIT + Duration::from_millis(50));
        assert!(
            gate.allow(),
            "a call after the rate-limit window elapses must be allowed \
             again, not stuck denied forever"
        );
    }

    #[test]
    fn an_empty_socket_set_with_no_deadline_returns_immediately_with_nothing_ready() {
        let ready = wait(&[], None);
        assert!(
            ready.is_empty(),
            "no sockets and no deadline must not block or report anything ready"
        );
    }

    #[test]
    fn an_empty_socket_set_with_a_past_deadline_returns_immediately_with_nothing_ready() {
        let past = Instant::now();
        let ready = wait(&[], Some(past));
        assert!(
            ready.is_empty(),
            "a due deadline must not sleep, and an empty socket set has \
             nothing to report ready regardless"
        );
    }

    #[cfg(unix)]
    fn raw_of(s: &std::net::TcpStream) -> i64 {
        use std::os::unix::io::AsRawFd;
        i64::from(s.as_raw_fd())
    }

    #[cfg(windows)]
    fn raw_of(s: &std::net::TcpStream) -> i64 {
        use std::os::windows::io::AsRawSocket;
        s.as_raw_socket() as i64
    }

    #[test]
    fn wait_reports_a_socket_with_data_waiting() {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind loopback");
        let addr = listener.local_addr().expect("local addr");
        let mut client = std::net::TcpStream::connect(addr).expect("connect");
        let (mut server, _) = listener.accept().expect("accept");
        std::io::Write::write_all(&mut server, b"hi").expect("write");

        let sock = RawSocket(raw_of(&client));
        let ready = wait(&[(sock, Interest::Read)], None);
        assert_eq!(
            ready,
            vec![sock],
            "the client socket has data and must be ready"
        );
        let _ = &mut client;
    }

    #[test]
    fn wait_reports_a_socket_ready_for_write() {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind loopback");
        let addr = listener.local_addr().expect("local addr");
        let client = std::net::TcpStream::connect(addr).expect("connect");
        let (_server, _) = listener.accept().expect("accept");

        let sock = RawSocket(raw_of(&client));
        let ready = wait(&[(sock, Interest::Write)], None);
        assert_eq!(
            ready,
            vec![sock],
            "a freshly connected socket's send buffer is empty and must be write-ready"
        );
    }

    #[test]
    fn wait_returns_empty_when_the_deadline_passes_first() {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind loopback");
        let addr = listener.local_addr().expect("local addr");
        let client = std::net::TcpStream::connect(addr).expect("connect");
        let (_server, _) = listener.accept().expect("accept");

        let sock = RawSocket(raw_of(&client));
        let deadline = Instant::now() + Duration::from_millis(50);
        let ready = wait(&[(sock, Interest::Read)], Some(deadline));
        assert!(
            ready.is_empty(),
            "no data was ever written to the connected socket; the deadline must fire first"
        );
    }

    #[test]
    fn wait_with_no_sockets_and_a_deadline_sleeps_until_it() {
        let interval = Duration::from_millis(40);
        let start = Instant::now();
        let ready = wait(&[], Some(start + interval));
        assert!(
            ready.is_empty(),
            "an empty socket set has nothing to report ready"
        );
        assert!(
            start.elapsed() >= interval,
            "the timer-only path must actually sleep out the deadline, not return early"
        );
    }

    #[test]
    fn set_nonblocking_makes_a_read_return_would_block_instead_of_blocking() {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind loopback");
        let addr = listener.local_addr().expect("local addr");
        let mut client = std::net::TcpStream::connect(addr).expect("connect");
        let _server = listener.accept().expect("accept");

        set_nonblocking(RawSocket(raw_of(&client))).expect("set nonblocking");

        let mut buf = [0u8; 8];
        let err = std::io::Read::read(&mut client, &mut buf)
            .expect_err("no data is waiting; a non-blocking read must not succeed");
        assert_eq!(
            err.kind(),
            std::io::ErrorKind::WouldBlock,
            "a non-blocking socket with nothing to read must fail with WouldBlock, not block"
        );
    }
}
