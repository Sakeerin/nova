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
            // per-socket (see `wait`'s own doc comment).
            if fd < 0 || fd as usize >= libc::FD_SETSIZE {
                tracing::warn!(
                    raw_socket = sock.0,
                    fd_setsize = libc::FD_SETSIZE,
                    "poll::wait: socket is outside select's representable range \
                     and will never be watched on this platform"
                );
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
            // Every socket was filtered out above (each already logged); there
            // is nothing for `select` to watch. Back off rather than spin:
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
            // call). There is no ready set to trust, and no channel to say
            // *which* socket broke so a caller could evict it (see `wait`'s
            // own doc comment) -- so log what's knowable and back off rather
            // than spin, exactly like the `!any_watched` case above: retrying
            // immediately cannot fix a persistent failure, only waste CPU
            // doing so.
            tracing::warn!(
                error = %err,
                "poll::wait: select failed; backing off rather than spinning"
            );
            std::thread::sleep(timeout.unwrap_or(ERROR_RETRY_BACKOFF));
            return Vec::new();
        }

        let mut ready = Vec::new();
        for (sock, interest) in sockets {
            let fd = sock.0 as RawFd;
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
            // to say which socket broke, so log what's knowable and back off
            // rather than spin.
            tracing::warn!(
                error_code = err,
                "poll::wait: WSAPoll failed; backing off rather than spinning"
            );
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
