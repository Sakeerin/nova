//! Open TCP connections, keyed by descriptor, and the two-phase, non-blocking
//! `connect` that populates the table.
//!
//! # The table is `file.rs`'s model, not a second one
//!
//! Same shape as `file.rs`'s open-file table, copied rather than reinvented:
//! a `thread_local!` `RefCell<HashMap<i64, _>>`, a `Cell<i64>` next-fd
//! starting at 1 and never reused, `try_borrow_mut` with an `abort_with`
//! backstop, and **absence from the table is closedness** -- a closed fd, a
//! stale fd, and an fd a Nova program forged by hand (`TcpStream { fd: 9999
//! }`, since record fields are not privacy-enforced) all miss the lookup and
//! become one `IoError`. `close` is idempotent for the identical reason
//! `file.rs`'s is: `std/net`'s `close` cannot consume its receiver, because
//! Nova has no move checking, so a caller can always reach a second call.
//!
//! # Why `connect` cannot be `std::net::TcpStream::connect`
//!
//! `std::net::TcpStream::connect` (and `connect_timeout`) block until the
//! connection settles; there is no `std`-only way to start one and ask about
//! it later. **This module builds the socket, sets it non-blocking, and
//! issues the `connect` syscall itself** (`platform_connect`, below),
//! wrapping the raw handle in a `std::net::TcpStream` immediately so `Drop`
//! closes it on every early-return path, and everything past that point --
//! reading `SO_ERROR` on the second poll -- goes back through `std::net`'s own
//! `TcpStream::take_error`, which already does exactly that `getsockopt` call.
//!
//! `connect`'s poll function joins the family `task.rs` already contains
//! (`poll_yield_once`, `poll_sleep`, `poll_join`): a constructor intrinsic
//! calls `task::build_future`, and the poll function reads a resume tag,
//! staging a park and returning `POLL_PENDING` on the first call, writing the
//! output and returning `POLL_READY` on the second.
//!
//! # `CONNECTION_REFUSED` gets its first producer
//!
//! `fs.rs`'s status table already declares `CONNECTION_REFUSED` and maps
//! `std::io::ErrorKind::ConnectionRefused` to it, but no filesystem operation
//! can produce that kind, so it has never been exercised. A refused `connect`
//! is its first producer -- reached either synchronously (loopback refusals
//! often arrive as an immediate `ECONNREFUSED`/`WSAECONNREFUSED` from the
//! `connect` syscall itself, before this socket is ever parked, since the RST
//! for a closed loopback port needs no real network round trip) or via the
//! second poll's `SO_ERROR` check, for a refusal that arrives after parking.
//! Both paths route through the identical `fs::fail`, so no second mapping
//! exists to drift from the first.
//!
//! # Reasoned, not measured
//!
//! This module's Unix arm (`platform_connect`'s `#[cfg(unix)]` half) was
//! written and read against `connect(2)`/`libc` semantics, not built or run:
//! no Unix or macOS toolchain was reachable from this task's environment,
//! the identical status `poll.rs`'s own Unix arm already records. The BSD
//! (`sockaddr_in`/`sockaddr_in6`) layouts carry a leading `sin_len`/`sin6_len`
//! field this module's socket-address literals do not set explicitly --
//! struct-update syntax pulls it from a zeroed base instead, which `connect`
//! is widely documented to tolerate, but that tolerance is asserted from
//! documentation here, not from a passing test on that platform. The Windows
//! arm was built and exercised for real against real loopback sockets, on
//! this task's own Windows host, the same way `poll.rs`'s Windows arm was.

use crate::fs::{fail, stash, Slot, OK};
use crate::poll::{set_nonblocking, Interest, RawSocket};
use crate::task::{
    build_future, stage_io_park, PollFn, POLL_PENDING, POLL_READY, STATE_MIN_SIZE,
    STATE_SLOT_OUTPUT, STATE_SLOT_TAG, STATE_SLOT_TEMPS,
};
use crate::NovaStr;
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::net::{SocketAddr, TcpStream, ToSocketAddrs};

thread_local! {
    /// Open TCP connections by descriptor. `thread_local!` for the reason
    /// `task.rs`'s module doc gives for `TASKS` and `file.rs`'s module doc
    /// gives for `FILES`: the GC's roots are per-thread, so a second thread
    /// running Nova code would free objects the first holds.
    static SOCKETS: RefCell<HashMap<i64, TcpStream>> = RefCell::new(HashMap::new());
    /// Never reused, so a stale fd stays stale rather than aliasing a
    /// different connection later. Starts at 1 so 0 is available as an
    /// obviously invalid value in diagnostics.
    static NEXT_FD: Cell<i64> = const { Cell::new(1) };
}

/// Run `f` against the socket behind `fd`, or report a closed-socket error.
///
/// `try_borrow_mut` rather than `borrow_mut`, for the identical reason
/// `file.rs`'s `with_fd` gives: a `RefCell` panic here would cross a
/// generated poll boundary. The `None` arm is the closed/stale/forged case
/// and is an ordinary error, not an abort.
fn with_fd<R>(fd: i64, f: impl FnOnce(&mut TcpStream) -> R) -> Option<R> {
    SOCKETS.with(|sockets| {
        let Ok(mut sockets) = sockets.try_borrow_mut() else {
            crate::task::abort_with("nova_rt_net: handle table is already borrowed")
        };
        sockets.get_mut(&fd).map(f)
    })
}

/// Allocate a fresh, never-reused fd for `stream` and insert it into the
/// table. Mirrors `file.rs`'s `register_new_file` exactly, including the
/// fallible-borrow reasoning: a `RefCell` panic here would cross a generated
/// poll boundary, and there is no missing-key case to report since insertion
/// always creates the entry.
fn register_new_socket(stream: TcpStream) -> i64 {
    let fd = NEXT_FD.with(|next| {
        let fd = next.get();
        next.set(fd + 1);
        fd
    });
    SOCKETS.with(|sockets| {
        let Ok(mut sockets) = sockets.try_borrow_mut() else {
            crate::task::abort_with("nova_rt_net: handle table is already borrowed")
        };
        sockets.insert(fd, stream);
    });
    fd
}

/// Remove `fd` from the table, dropping the underlying `TcpStream` and
/// closing its OS handle. Silent (not an error) if `fd` is already absent --
/// this is the shared body [`nova_rt_net_close`] exposes directly and
/// [`finish_connect`] uses to release a socket whose connection was refused.
fn remove_socket(fd: i64) {
    SOCKETS.with(|sockets| {
        let Ok(mut sockets) = sockets.try_borrow_mut() else {
            crate::task::abort_with("nova_rt_net: handle table is already borrowed")
        };
        sockets.remove(&fd);
    });
}

/// Stash `n` as an 8-byte little-endian `Bytes` payload in `Slot::Buffer`.
///
/// The identical encoding `file.rs`'s own `stash_i64` uses for `open`'s new
/// fd, reproduced here rather than shared across modules for the same reason
/// that one gives: a second, self-contained implementation of one small
/// encoding costs less than reaching past the four names this module imports
/// from `fs`.
fn stash_i64(n: i64) {
    stash(Slot::Buffer, crate::bytes::gc_bytes(&n.to_le_bytes()));
}

/// Report the closed/stale/forged-fd case as `IoError { kind: Other }`.
///
/// Mirrors `file.rs`'s `closed_fd_error` exactly: there is no real
/// `std::io::Error` to draw a status and message from, since the table
/// lookup missed before any syscall ran, so this fabricates one of
/// `ErrorKind::Other` and routes it through the same [`fail`] every real
/// failure in this module already uses, landing on `fs.rs`'s catch-all
/// status rather than a new constant.
fn closed_fd_error() -> i64 {
    fail(&std::io::Error::other("socket is not open"))
}

/// Close `fd`, dropping the underlying connection and releasing its OS
/// handle. Idempotent, for the identical reason `file.rs`'s
/// `nova_rt_file_close` is: `std/net`'s `close` cannot consume its receiver,
/// because Nova has no move checking, so a caller can always reach a second
/// call, and it must find nothing and still succeed.
///
/// # Safety
/// No pointer argument, so no dereference precondition; marked `unsafe
/// extern "C"` for uniformity with this crate's other JIT-registered symbols.
#[no_mangle]
pub unsafe extern "C" fn nova_rt_net_close(fd: i64) -> i64 {
    remove_socket(fd);
    OK
}

/// Resolve `addr` ("host:port") to one socket address.
///
/// Uses `std`'s own `ToSocketAddrs` -- the identical resolution
/// `std::net::TcpStream::connect` performs -- taking the first result, the
/// same choice `TcpStream::connect` itself makes for a multi-address name.
/// Every caller in this project's own fixtures passes a numeric loopback
/// address, for which this never reaches real DNS; a hostname would, and
/// this module does not attempt to make that lookup itself non-blocking --
/// out of scope for this task, which specifies the `connect` *syscall* as
/// the non-blocking half.
fn resolve_addr(addr: &str) -> std::io::Result<SocketAddr> {
    addr.to_socket_addrs()?
        .next()
        .ok_or_else(|| std::io::Error::other("no addresses resolved for connect"))
}

/// The raw OS handle underneath `stream`, widened to the `RawSocket` this
/// crate's poller and executor share.
#[cfg(unix)]
fn raw_socket_of(stream: &TcpStream) -> RawSocket {
    use std::os::unix::io::AsRawFd;
    RawSocket(i64::from(stream.as_raw_fd()))
}

#[cfg(windows)]
fn raw_socket_of(stream: &TcpStream) -> RawSocket {
    use std::os::windows::io::AsRawSocket;
    RawSocket(stream.as_raw_socket() as i64)
}

/// Create a socket for `addr`'s family, set it non-blocking, and issue the
/// `connect` syscall against it.
///
/// Returns the connected-or-connecting `TcpStream` on anything short of a
/// hard failure -- both a genuine `EINPROGRESS`/`WSAEWOULDBLOCK` (the
/// ordinary case) and the rare synchronous success some platforms can return
/// directly from `connect` are folded together, since both mean "park on
/// write readiness and let `SO_ERROR` settle it on the next poll" is the
/// right next step either way (an already-established connection is
/// trivially write-ready, so parking on one costs one extra, immediately-won
/// turn rather than a wrong answer). A synchronous refusal
/// (`ECONNREFUSED`/`WSAECONNREFUSED`) -- routine for a loopback RST, which
/// needs no network round trip to arrive -- and every other creation or
/// `connect` failure return `Err`.
///
/// The raw handle is wrapped in a `TcpStream` immediately after creation, so
/// every `?` after that point closes it via `Drop` rather than leaking it.
#[cfg(unix)]
fn platform_connect(addr: SocketAddr) -> std::io::Result<TcpStream> {
    use std::os::unix::io::{AsRawFd, FromRawFd};

    let domain = match addr {
        SocketAddr::V4(_) => libc::AF_INET,
        SocketAddr::V6(_) => libc::AF_INET6,
    };
    // SAFETY: `domain` and `libc::SOCK_STREAM` are plain integers; this
    // creates a new, unconnected socket and returns its descriptor, or -1.
    let fd = unsafe { libc::socket(domain, libc::SOCK_STREAM, 0) };
    if fd < 0 {
        return Err(std::io::Error::last_os_error());
    }
    // SAFETY: `fd` is the fresh, uniquely-owned descriptor `socket` just
    // returned above.
    let stream = unsafe { TcpStream::from_raw_fd(fd) };
    set_nonblocking(raw_socket_of(&stream))?;

    let rc = match addr {
        SocketAddr::V4(a) => {
            // `..unsafe { zeroed() }` rather than naming every field: BSD's
            // `sockaddr_in` carries a leading `sin_len` this literal does not
            // set (see this module's own "reasoned, not measured" note).
            let sin = libc::sockaddr_in {
                sin_family: libc::AF_INET as libc::sa_family_t,
                sin_port: a.port().to_be(),
                sin_addr: libc::in_addr {
                    s_addr: u32::from(*a.ip()).to_be(),
                },
                sin_zero: [0; 8],
                ..unsafe { std::mem::zeroed() }
            };
            // SAFETY: `fd` is the socket just created above; `sin` is a
            // live, correctly sized `sockaddr_in` for the duration of this
            // call.
            unsafe {
                libc::connect(
                    fd,
                    std::ptr::addr_of!(sin).cast::<libc::sockaddr>(),
                    std::mem::size_of::<libc::sockaddr_in>() as libc::socklen_t,
                )
            }
        }
        SocketAddr::V6(a) => {
            let sin6 = libc::sockaddr_in6 {
                sin6_family: libc::AF_INET6 as libc::sa_family_t,
                sin6_port: a.port().to_be(),
                sin6_flowinfo: a.flowinfo(),
                sin6_addr: libc::in6_addr {
                    s6_addr: a.ip().octets(),
                },
                sin6_scope_id: a.scope_id(),
                ..unsafe { std::mem::zeroed() }
            };
            // SAFETY: same as the `V4` arm above, for `sockaddr_in6`.
            unsafe {
                libc::connect(
                    fd,
                    std::ptr::addr_of!(sin6).cast::<libc::sockaddr>(),
                    std::mem::size_of::<libc::sockaddr_in6>() as libc::socklen_t,
                )
            }
        }
    };

    if rc == 0 {
        return Ok(stream);
    }
    let err = std::io::Error::last_os_error();
    if err.raw_os_error() == Some(libc::EINPROGRESS) {
        return Ok(stream);
    }
    Err(err)
}

/// Ensure Winsock has been initialized on this process, exactly once.
///
/// Nothing guarantees `std::net` has already been touched elsewhere in this
/// process by the time a Nova program's first `connect` call reaches this
/// module's raw `socket`/`connect` calls, so this cannot rely on `std`'s own
/// internal `WSAStartup` having already run. Calling `WSAStartup` a second
/// time (alongside whatever `std::net` itself may separately call) is
/// explicitly supported by the Winsock spec, which reference-counts it; this
/// never calls the matching `WSACleanup`, the same choice `std` itself
/// makes, relying on process exit to release the resource.
#[cfg(windows)]
fn ensure_wsa_started() {
    use std::sync::Once;
    use windows_sys::Win32::Networking::WinSock::{WSAStartup, WSADATA};

    static INIT: Once = Once::new();
    INIT.call_once(|| {
        let mut data = WSADATA::default();
        // SAFETY: `data` is a valid, writable `WSADATA` for the duration of
        // this call; `0x0202` requests Winsock 2.2, the only version this
        // module's socket calls assume.
        let _ = unsafe { WSAStartup(0x0202, &mut data) };
    });
}

/// Windows counterpart to the Unix `platform_connect` above -- same
/// contract, same three-outcome shape (synchronous success, in-progress,
/// hard failure), read side by side with it per this module's own "compare
/// the two arms" discipline. Built and exercised for real against loopback
/// sockets on this task's own Windows host (see this module's own doc
/// comment).
#[cfg(windows)]
fn platform_connect(addr: SocketAddr) -> std::io::Result<TcpStream> {
    use std::os::windows::io::FromRawSocket;
    use windows_sys::Win32::Networking::WinSock::{
        connect, socket, WSAGetLastError, AF_INET, AF_INET6, IN6_ADDR, IN6_ADDR_0, INVALID_SOCKET,
        IN_ADDR, IN_ADDR_0, IPPROTO_TCP, SOCKADDR, SOCKADDR_IN, SOCKADDR_IN6, SOCKADDR_IN6_0,
        SOCK_STREAM, WSAEWOULDBLOCK,
    };

    ensure_wsa_started();

    let domain = match addr {
        SocketAddr::V4(_) => i32::from(AF_INET),
        SocketAddr::V6(_) => i32::from(AF_INET6),
    };
    // SAFETY: `domain`/`SOCK_STREAM`/`IPPROTO_TCP` are plain integers; this
    // creates a new, unconnected socket.
    let sock = unsafe { socket(domain, SOCK_STREAM, IPPROTO_TCP) };
    if sock == INVALID_SOCKET {
        // SAFETY: reads thread-local Winsock error state only.
        return Err(std::io::Error::from_raw_os_error(unsafe {
            WSAGetLastError()
        }));
    }
    // SAFETY: `sock` is the fresh, uniquely-owned handle `socket` just
    // returned above.
    let stream = unsafe { TcpStream::from_raw_socket(sock as u64) };
    set_nonblocking(raw_socket_of(&stream))?;

    let rc = match addr {
        SocketAddr::V4(a) => {
            let sin = SOCKADDR_IN {
                sin_family: AF_INET,
                sin_port: a.port().to_be(),
                sin_addr: IN_ADDR {
                    S_un: IN_ADDR_0 {
                        S_addr: u32::from(*a.ip()).to_be(),
                    },
                },
                sin_zero: [0; 8],
            };
            // SAFETY: `sock` is the socket just created above; `sin` is a
            // live, correctly sized `SOCKADDR_IN` for the duration of this
            // call.
            unsafe {
                connect(
                    sock,
                    std::ptr::addr_of!(sin).cast::<SOCKADDR>(),
                    std::mem::size_of::<SOCKADDR_IN>() as i32,
                )
            }
        }
        SocketAddr::V6(a) => {
            let sin6 = SOCKADDR_IN6 {
                sin6_family: AF_INET6,
                sin6_port: a.port().to_be(),
                sin6_flowinfo: a.flowinfo(),
                sin6_addr: IN6_ADDR {
                    u: IN6_ADDR_0 {
                        Byte: a.ip().octets(),
                    },
                },
                Anonymous: SOCKADDR_IN6_0 {
                    sin6_scope_id: a.scope_id(),
                },
            };
            // SAFETY: same as the `V4` arm above, for `SOCKADDR_IN6`.
            unsafe {
                connect(
                    sock,
                    std::ptr::addr_of!(sin6).cast::<SOCKADDR>(),
                    std::mem::size_of::<SOCKADDR_IN6>() as i32,
                )
            }
        }
    };

    if rc == 0 {
        return Ok(stream);
    }
    // SAFETY: reads thread-local Winsock error state only.
    let err = unsafe { WSAGetLastError() };
    if err == WSAEWOULDBLOCK {
        return Ok(stream);
    }
    Err(std::io::Error::from_raw_os_error(err))
}

/// Where the connect future keeps its socket between polls. One past the
/// last slot the ABI reserves, so the state object is one word larger than
/// `STATE_MIN_SIZE` -- the same arrangement `SLEEP_SLOT_MS` uses.
///
/// Reused for two different values across the future's short life: before
/// the first poll it holds the address argument's `NovaStr` pointer (kept
/// alive by this state object's own GC root/scan, exactly as
/// `SLEEP_SLOT_MS` needs no rooting of its own for a plain `Int`); the first
/// poll reads that out and overwrites this same slot with the socket's table
/// fd for the second poll to look back up.
const CONNECT_SLOT_SOCK: usize = STATE_SLOT_TEMPS;

/// State size for a connect future: the ABI minimum plus the one temp slot
/// holding the address pointer, and later the socket.
const CONNECT_STATE_SIZE: usize = STATE_MIN_SIZE + 8;

const _: () = assert!(CONNECT_STATE_SIZE >= (CONNECT_SLOT_SOCK + 1) * 8);

/// What starting a connect attempt produced.
enum Started {
    /// The `connect` syscall is in progress (or, rarely, already
    /// succeeded) -- park on write readiness and let the next poll's
    /// `SO_ERROR` check settle it. Carries the raw socket to park on.
    WouldBlock(RawSocket),
    /// The `connect` syscall failed synchronously, most commonly
    /// `ECONNREFUSED`/`WSAECONNREFUSED` for a loopback RST (see this
    /// module's own doc comment on why that can arrive before any park is
    /// ever staged). Carries the status [`fail`] already produced.
    Failed(i64),
}

/// The first poll's work: read the address out of `slots`, resolve it,
/// create a non-blocking socket, and issue `connect`.
///
/// On success, registers the new `TcpStream` in [`SOCKETS`] and overwrites
/// `slots[CONNECT_SLOT_SOCK]` with its table fd -- the address pointer that
/// slot held until this point is never read again after this call.
fn start_connect(slots: *mut i64) -> Started {
    // SAFETY: `slots` is the connect future's own state object, at least
    // `CONNECT_STATE_SIZE` bytes; `nova_rt_net_connect_future` wrote the
    // address pointer into `CONNECT_SLOT_SOCK` at construction, and this is
    // the first poll, so nothing has overwritten it yet.
    let ptr = unsafe { slots.add(CONNECT_SLOT_SOCK).read() } as *const NovaStr;
    // SAFETY: `ptr` is the address `nova_rt_net_connect_future` was given,
    // kept alive by this state object's own GC root and scan for as long as
    // this slot has not yet been overwritten -- true up to this read.
    let addr_str = unsafe { crate::as_str(ptr) };

    let addr = match resolve_addr(addr_str) {
        Ok(addr) => addr,
        Err(e) => return Started::Failed(fail(&e)),
    };

    match platform_connect(addr) {
        Ok(stream) => {
            let raw = raw_socket_of(&stream);
            let table_fd = register_new_socket(stream);
            // SAFETY: same object; the address has been read out above, so
            // overwriting this slot with the table fd loses nothing
            // `finish_connect` still needs.
            unsafe { slots.add(CONNECT_SLOT_SOCK).write(table_fd) };
            Started::WouldBlock(raw)
        }
        Err(e) => Started::Failed(fail(&e)),
    }
}

/// The second poll's work: look up the socket `start_connect` registered and
/// check whether it finished connecting or was refused.
///
/// `TcpStream::take_error` is `std`'s own `SO_ERROR` check -- reused rather
/// than a second, raw `getsockopt` call, since it already does exactly what
/// this needs. `Ok(None)` is success; anything else -- a pending socket
/// error, or the table lookup itself missing -- removes the entry (nothing
/// further can be done with a failed connection) and reports the failure
/// through the same [`fail`] every other status in this module already uses.
fn finish_connect(slots: *mut i64) -> i64 {
    // SAFETY: same object; `start_connect` overwrote this slot with the
    // table fd before parking, and this is the second poll.
    let table_fd = unsafe { slots.add(CONNECT_SLOT_SOCK).read() };
    match with_fd(table_fd, |stream| stream.take_error()) {
        Some(Ok(None)) => {
            stash_i64(table_fd);
            OK
        }
        Some(Ok(Some(e))) => {
            remove_socket(table_fd);
            fail(&e)
        }
        Some(Err(e)) => {
            remove_socket(table_fd);
            fail(&e)
        }
        None => closed_fd_error(),
    }
}

/// The connect future's poll function: non-blocking `connect`, parked on
/// write readiness, resolved by checking the socket error on the second
/// poll.
///
/// Must not unwind, as `poll_sleep` and `poll_join` must not (see `task.rs`'s
/// `PollFn` doc comment): every fallible step here -- the borrow in
/// `with_fd`/`register_new_socket`/`remove_socket`, and every I/O call --
/// already returns a `Result` this function maps into a status rather than
/// unwrapping.
unsafe extern "C-unwind" fn poll_connect(state: *mut u8, _task_ctx: *mut u8) -> i64 {
    let slots = state as *mut i64;
    // SAFETY: `state` is the object `nova_rt_net_connect_future` built, at
    // least `CONNECT_STATE_SIZE` bytes, so every slot below is in bounds.
    let tag = unsafe { slots.add(STATE_SLOT_TAG).read() };
    if tag == 0 {
        unsafe { slots.add(STATE_SLOT_TAG).write(1) };
        // Non-blocking connect: expect would-block, park on WRITE readiness.
        // A blocking connect would pass every loopback test in this project
        // while defeating this whole increment -- see this module's own doc
        // comment.
        match start_connect(slots) {
            Started::WouldBlock(sock) => {
                stage_io_park(sock, Interest::Write, None);
                return POLL_PENDING;
            }
            Started::Failed(status) => {
                // SAFETY: same object, output slot.
                unsafe { slots.add(STATE_SLOT_OUTPUT).write(status) };
                return POLL_READY;
            }
        }
    }
    // Second poll: the socket is write-ready (or its deadline is irrelevant
    // here -- `connect` parks with none); `SO_ERROR` says whether the
    // connection actually established or was refused.
    let status = finish_connect(slots);
    // SAFETY: same object, output slot.
    unsafe { slots.add(STATE_SLOT_OUTPUT).write(status) };
    POLL_READY
}

/// A fresh `Future<Int>` (a status, per this module's boundary design -- see
/// `fs.rs`'s own module doc comment) that connects to `addr` ("host:port"),
/// non-blockingly.
///
/// **The state object is fresh on every call**, not shared, for the same
/// reason `nova_rt_task_sleep_future`'s doc comment gives: the whole value
/// carried across a suspension is this state object's own resume tag and
/// socket slot, so two connects in flight at once would otherwise corrupt
/// each other.
///
/// On success, the new fd is stashed via `Slot::Buffer` exactly as
/// `file.rs`'s `nova_rt_file_open` stashes its own new fd -- the status word
/// already carries the `IoErrorKind`, so the fd cannot travel there too.
///
/// # Safety
/// `addr` must point to a live `NovaStr`.
#[no_mangle]
pub unsafe extern "C-unwind" fn nova_rt_net_connect_future(addr: *const NovaStr) -> *mut u8 {
    let poll: PollFn = poll_connect;
    build_future(poll, CONNECT_STATE_SIZE, |slots| {
        // SAFETY: `slots` addresses a live `CONNECT_STATE_SIZE` block, and
        // `CONNECT_SLOT_SOCK` is in bounds by the assertion above. Stores the
        // address pointer itself, not a copy -- kept alive by this state
        // object's own GC root and scan until `start_connect` reads it back
        // on the first poll and overwrites this slot with the socket.
        unsafe { slots.add(CONNECT_SLOT_SOCK).write(addr as i64) };
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fs::CONNECTION_REFUSED;

    /// Test-only: a `Future<unit>` that completes on its very first poll --
    /// used only to "pump" this thread's executor queue in
    /// [`a_successful_connect_stashes_its_fd_via_slot_buffer`], the same
    /// shape as `task.rs`'s own `poll_ready_now` test fixture.
    unsafe extern "C-unwind" fn poll_ready_immediately_for_test(
        state: *mut u8,
        _task_ctx: *mut u8,
    ) -> i64 {
        // SAFETY: `state` is at least `STATE_MIN_SIZE`, built by this test's
        // own `build_future` call.
        unsafe { (state as *mut i64).add(STATE_SLOT_OUTPUT).write(0) };
        POLL_READY
    }

    /// Test-only: decode an fd [`stash_i64`] stashed -- the mirror image of
    /// that function's own 8-byte little-endian encoding, matching
    /// `file.rs`'s own `take_fd` test helper.
    fn take_fd() -> i64 {
        let ptr = crate::fs::take_for_test(Slot::Buffer) as *const NovaStr;
        // SAFETY: test-only. Whatever last stashed into `Slot::Buffer` did so
        // earlier in this same test, with nothing allocated since, so the
        // payload has not been swept.
        let bytes = unsafe { crate::bytes::as_bytes(ptr) };
        let Ok(arr) = <[u8; 8]>::try_from(bytes) else {
            panic!(
                "stash_i64 stashes an 8-byte payload; got {} bytes instead -- \
                 either nothing was pending or the encoding changed without \
                 updating this test helper to match",
                bytes.len()
            );
        };
        i64::from_le_bytes(arr)
    }

    /// Test-only: run `f` with `CURRENT` set to `task_id`, restoring
    /// whatever it held before -- the same technique `fs.rs`'s
    /// `stash_for_test` uses. Only [`a_successful_connect_stashes_its_fd_via_slot_buffer`]
    /// needs this, to read `fs::Slot::Buffer` back under the connect task's
    /// own identity, per that test's own doc comment on why.
    fn with_test_current<R>(task_id: i64, f: impl FnOnce() -> R) -> R {
        let previous = crate::task::current_task();
        crate::task::set_current_for_test(Some(task_id));
        let result = f();
        crate::task::set_current_for_test(previous);
        result
    }

    /// Test-only: the `{ poll_code, state }` state pointer inside a future
    /// fat pointer, and a raw slot read from it -- shared by every helper
    /// below that reads a connect future's own state directly rather than
    /// through `fs::Slot::Buffer`.
    ///
    /// **Why read the state directly at all, rather than always going
    /// through `Slot::Buffer` and `take_fd`.** `finish_connect`'s success
    /// path stashes the new fd via `fs::Slot::Buffer`, keyed per-task by
    /// `crate::task::current_task()` (`fs.rs`'s module doc comment explains
    /// why `SLOTS` is per-task). In a compiled Nova program `connect` is
    /// always polled *inline*, as part of the awaiting task's own generated
    /// poll function, so that task reads the stash immediately after the
    /// await resolves -- long before it, in turn, ever completes and has
    /// its own `fs::Slot` storage released. This module's tests instead
    /// drive the *bare* connect future directly through
    /// `nova_rt_task_block_on`, the same way `task.rs`'s own tests drive
    /// `sleep`/`join` directly -- but completing a task through `block_on`
    /// releases that task's `fs::Slot` storage
    /// (`take_output_internal`'s `release_task_slots`) before control ever
    /// returns to the caller, so a plain `take_fd()` call after `block_on`
    /// returns always finds it already gone. `finish_connect` never
    /// overwrites `CONNECT_SLOT_SOCK` after reading it, and `poll_connect`
    /// writes the final status to `STATE_SLOT_OUTPUT` the same way every
    /// other future in this crate does -- both slots are still sitting in
    /// the state object itself, unrooted but not yet swept (nothing
    /// allocates in between), the same reasoning `take_fd`'s own comment
    /// already relies on for `Slot::Buffer`. Reading them directly sidesteps
    /// the release-slots ordering entirely, for the ordinary test helpers
    /// below that only need a working fd, not proof that the `Slot::Buffer`
    /// side channel itself works --
    /// [`a_successful_connect_stashes_its_fd_via_slot_buffer`] is the one
    /// test that exists to prove that separately.
    fn state_slot_of(fut: *mut u8, slot: usize) -> i64 {
        // SAFETY: `fut` is a well-formed `{ poll_code, state }` fat pointer
        // this module built.
        let words = fut as *mut usize;
        let state = unsafe { words.add(crate::task::FUTURE_SLOT_STATE).read() } as *mut i64;
        unsafe { state.add(slot).read() }
    }

    /// Test-only: connect to `addr` and return the resulting fd, driven
    /// through the real two-phase future and the real executor -- not a
    /// shortcut, since this project's whole point is that a blocking
    /// implementation would look identical from here. "blocking" in this
    /// helper's name (matching the task brief's own name for it) describes
    /// that it blocks *this test* until the connect settles, via
    /// `nova_rt_task_block_on`, not that `connect` itself blocks.
    ///
    /// Reads the fd out of the state object directly rather than through
    /// `fs::Slot::Buffer` -- see [`state_slot_of`]'s own doc comment for why.
    fn connect_blocking_for_test(addr: &str) -> i64 {
        let addr_ptr = crate::gc_str(addr);
        let fut = unsafe { nova_rt_net_connect_future(addr_ptr) };
        let status = unsafe { crate::task::nova_rt_task_block_on(fut) };
        assert_eq!(status, OK, "test setup: connect must succeed");
        state_slot_of(fut, CONNECT_SLOT_SOCK)
    }

    /// Test-only: connect to `addr` and return the resulting status, without
    /// assuming success -- the helper the refused-connection test uses.
    fn connect_status_for_test(addr: &str) -> i64 {
        let addr_ptr = crate::gc_str(addr);
        let fut = unsafe { nova_rt_net_connect_future(addr_ptr) };
        unsafe { crate::task::nova_rt_task_block_on(fut) }
    }

    /// Test-only: attempt a one-byte, non-blocking read on `fd` and map the
    /// outcome through the same [`fail`]/[`closed_fd_error`] paths
    /// production code uses. This module has no production read yet (Task 4
    /// adds `nova_rt_net_read_future`), so this exists only to let this
    /// module's own tests exercise `with_fd`'s `None` arm through something
    /// shaped like a real operation, the way `file.rs`'s tests exercise
    /// `nova_rt_file_read` directly.
    ///
    /// A `WouldBlock` on a still-open, still-idle socket maps to the same
    /// non-`OK` status `closed_fd_error` does (both fall to `fail`'s
    /// `_ => OTHER` arm) -- so this cannot, by itself, distinguish "no data
    /// yet" from "closed". [`is_open_for_test`] is this module's check for
    /// that distinction instead.
    fn read_status_for_test(fd: i64) -> i64 {
        let mut buf = [0u8; 1];
        match with_fd(fd, |stream| std::io::Read::read(stream, &mut buf)) {
            Some(Ok(_)) => OK,
            Some(Err(e)) => fail(&e),
            None => closed_fd_error(),
        }
    }

    /// Test-only: whether `fd` is currently a live entry in the socket
    /// table -- the direct check for "absence from the table is
    /// closedness," without needing a read to distinguish "closed" from
    /// "open but idle" the way [`read_status_for_test`] cannot.
    fn is_open_for_test(fd: i64) -> bool {
        with_fd(fd, |_| ()).is_some()
    }

    /// Test-only: a loopback address nothing is listening on -- bind, read
    /// the port, then drop the listener so the port is closed again. The
    /// kernel refuses a connection to a closed port rather than hanging, on
    /// all three CI platforms.
    fn dead_addr() -> String {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind");
        let addr = listener.local_addr().expect("addr").to_string();
        drop(listener);
        addr
    }

    #[test]
    fn a_connected_socket_closes_once_and_then_reports_not_open() {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind");
        let addr = listener.local_addr().expect("addr").to_string();
        let fd = connect_blocking_for_test(&addr);
        assert_eq!(unsafe { nova_rt_net_close(fd) }, OK);
        // Idempotent: a second close finds nothing and still succeeds.
        assert_eq!(unsafe { nova_rt_net_close(fd) }, OK);
        // Absence from the table IS closedness.
        assert_ne!(
            read_status_for_test(fd),
            OK,
            "a read on a closed fd must fail"
        );
    }

    #[test]
    fn connecting_to_a_closed_port_is_connection_refused() {
        assert_eq!(connect_status_for_test(&dead_addr()), CONNECTION_REFUSED);
    }

    /// Two live connections must not collide: connecting a second time while
    /// the first is still open must not overwrite the first's table entry
    /// or alias its handle -- the `net.rs` analogue of `file.rs`'s
    /// `two_open_files_do_not_collide`.
    #[test]
    fn two_connected_sockets_do_not_collide() {
        let listener_a = std::net::TcpListener::bind("127.0.0.1:0").expect("bind a");
        let listener_b = std::net::TcpListener::bind("127.0.0.1:0").expect("bind b");
        let addr_a = listener_a.local_addr().expect("addr a").to_string();
        let addr_b = listener_b.local_addr().expect("addr b").to_string();

        let fd_a = connect_blocking_for_test(&addr_a);
        let fd_b = connect_blocking_for_test(&addr_b);
        assert_ne!(
            fd_a, fd_b,
            "two live connections must not share a descriptor"
        );

        assert!(is_open_for_test(fd_a));
        assert!(is_open_for_test(fd_b));
        assert_eq!(unsafe { nova_rt_net_close(fd_a) }, OK);
        assert!(
            is_open_for_test(fd_b),
            "closing one socket must not close the other"
        );
        assert_eq!(unsafe { nova_rt_net_close(fd_b) }, OK);
    }

    /// `close` stays a no-op on repetition, not only correct the first time
    /// -- matching `file.rs`'s `close_is_idempotent_across_repeated_calls`.
    #[test]
    fn close_is_idempotent_across_repeated_calls() {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind");
        let addr = listener.local_addr().expect("addr").to_string();
        let fd = connect_blocking_for_test(&addr);
        assert_eq!(unsafe { nova_rt_net_close(fd) }, OK, "the first close");
        assert_eq!(unsafe { nova_rt_net_close(fd) }, OK, "a second close");
        assert_eq!(unsafe { nova_rt_net_close(fd) }, OK, "a third close");
    }

    /// An fd this module never issued takes the identical path through
    /// `with_fd`'s `None` arm as a closed one -- the `net.rs` shape of the
    /// design spec's forged `TcpStream` case, matching `file.rs`'s
    /// `a_never_issued_fd_is_an_error_not_a_panic`.
    #[test]
    fn closing_a_never_issued_fd_is_still_ok() {
        assert_eq!(
            unsafe { nova_rt_net_close(999_999) },
            OK,
            "closing an fd that was never open is still OK -- close is \
             idempotent over absence, not only over a fd it previously held"
        );
    }

    /// The hazard this project calls out by name: a blocking `connect` would
    /// return `POLL_READY` on this very first call and pass every other test
    /// in this file, since a loopback handshake settles within microseconds
    /// either way -- this project's own "passes for the wrong reason"
    /// hazard. Only the *return value of one poll*, not the eventual answer,
    /// tells the two apart.
    ///
    /// `stage_io_park` aborts the whole process if called outside a task
    /// context, so this borrows one for the duration of this one manual
    /// poll via `set_current_for_test`, the same technique `fs.rs`'s
    /// `stash_for_test` uses, and restores whatever `CURRENT` held before.
    /// The stray `PENDING_PARK` entry this manual poll leaves behind (never
    /// drained, since nothing here goes through `poll_one`'s own cleanup) is
    /// harmless: the very next real poll -- `nova_rt_task_block_on` below,
    /// on this same thread, before anything else runs here -- drains it via
    /// `Cell::take` regardless of what it finds, and this test never shares
    /// that leftover with another task in between.
    ///
    /// Reads the fd back via [`state_slot_of`] rather than `take_fd`, for
    /// the reason that helper's own doc comment gives.
    #[test]
    fn connect_parks_on_its_first_poll_rather_than_completing_synchronously() {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind");
        let addr = listener.local_addr().expect("addr").to_string();
        let addr_ptr = crate::gc_str(&addr);
        let fut = unsafe { nova_rt_net_connect_future(addr_ptr) };

        // SAFETY: `fut` is a well-formed `{ poll_code, state }` fat pointer
        // `nova_rt_net_connect_future` just built.
        let words = fut as *mut usize;
        let poll_code = unsafe { words.add(crate::task::FUTURE_SLOT_POLL).read() };
        let state = unsafe { words.add(crate::task::FUTURE_SLOT_STATE).read() } as *mut u8;
        // SAFETY: `poll_code` is a `PollFn` bit pattern by `fut`'s own
        // construction; a function pointer and a `usize` are both
        // pointer-width.
        let poll: PollFn = unsafe { std::mem::transmute(poll_code) };
        // SAFETY: `state` is the live state object above; `task_ctx` is
        // always null, matching every other call site in this crate. Any
        // non-`None` `CURRENT` satisfies `stage_io_park`'s check; the exact
        // value borrowed here does not have to match anything read back
        // later, since the fd is read via `state_slot_of` below, not
        // `fs::Slot`.
        let first = with_test_current(0, || unsafe { poll(state, std::ptr::null_mut()) });

        assert_eq!(
            first, POLL_PENDING,
            "a non-blocking connect must park on its first poll rather than \
             complete synchronously"
        );

        // Drive it the rest of the way through the real executor so this
        // test does not leak a parked task or an open socket into whatever
        // else shares this thread's tables afterward.
        assert_eq!(unsafe { crate::task::nova_rt_task_block_on(fut) }, OK);
        let fd = state_slot_of(fut, CONNECT_SLOT_SOCK);
        assert_eq!(unsafe { nova_rt_net_close(fd) }, OK);
    }

    /// The one test that proves `finish_connect`'s successful path really
    /// does stash the new fd via `fs::Slot::Buffer` -- the side channel
    /// Task 5's Nova-facing wrapper will read from, and the piece every
    /// other test in this file deliberately bypasses (see
    /// [`state_slot_of`]'s own doc comment for why they do, and why that is
    /// not itself a hole in coverage without this test to fill it).
    ///
    /// Spawns the connect future directly (`nova_rt_task_spawn`, not
    /// `nova_rt_task_block_on`) so this test -- not `block_on` -- controls
    /// when its task's `fs::Slot` storage is released. A second, unrelated,
    /// immediately-ready future is then driven through `block_on`, which
    /// "implicitly joins everything queued on this thread" (`block_on`'s own
    /// doc comment) -- draining the connect task to completion too, as a
    /// side effect, without this test ever calling
    /// `nova_rt_task_take_output`/`_release` *on it*. That is what leaves its
    /// `Slot::Buffer` entry intact for [`with_test_current`] to read back
    /// afterward, under the connect task's own id (returned directly by
    /// `nova_rt_task_spawn`, not guessed).
    #[test]
    fn a_successful_connect_stashes_its_fd_via_slot_buffer() {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind");
        let addr = listener.local_addr().expect("addr").to_string();
        let addr_ptr = crate::gc_str(&addr);
        let fut = unsafe { nova_rt_net_connect_future(addr_ptr) };
        // SAFETY: `fut` is the well-formed future built above.
        let id = unsafe { crate::task::nova_rt_task_spawn(fut) };

        let pump: PollFn = poll_ready_immediately_for_test;
        let pump_fut = build_future(pump, STATE_MIN_SIZE, |_| {});
        // SAFETY: `pump_fut` is a well-formed future built above.
        assert_eq!(unsafe { crate::task::nova_rt_task_block_on(pump_fut) }, 0);

        // SAFETY: `fut` was spawned above on this same thread.
        assert_ne!(
            unsafe { crate::task::nova_rt_task_is_done(fut) },
            0,
            "test setup: the connect task must have been driven to completion \
             as a side effect of draining this thread's whole queue"
        );
        assert_eq!(
            state_slot_of(fut, STATE_SLOT_OUTPUT),
            OK,
            "test setup: connect must succeed"
        );

        let fd = with_test_current(id, take_fd);
        assert!(fd > 0, "the stashed fd must be a real, positive descriptor");
        assert!(is_open_for_test(fd));
        assert_eq!(unsafe { nova_rt_net_close(fd) }, OK);
    }

    /// The exact layout `nova_rt_net_connect_future` builds, read back from
    /// the collector's own records -- the same discipline
    /// `the_sleep_futures_layout_is_the_one_the_abi_declares` and
    /// `the_join_futures_layout_is_the_one_the_abi_declares` in `task.rs`
    /// use, and for the identical reason: a `build_future` call site one
    /// word short of what its `init` closure writes is invisible to reading
    /// the words back (a too-small allocation with slop after it still reads
    /// back whatever was written), and only the collector's own recorded
    /// size distinguishes the two.
    ///
    /// Built but never polled, so this never touches a real socket -- the
    /// address argument is a placeholder never dialed.
    #[test]
    fn the_connect_futures_layout_is_the_one_the_abi_declares() {
        let addr_ptr = crate::gc_str("127.0.0.1:1");
        let fut = unsafe { nova_rt_net_connect_future(addr_ptr) };
        assert_eq!(
            crate::gc::object_info(fut as usize),
            Some((crate::task::FUTURE_SIZE, true)),
            "the fat pointer must be exactly the two-word future, scanned"
        );
        // SAFETY: `fut` is this call's own `FUTURE_SIZE`-byte block.
        let state = unsafe {
            (fut as *mut usize)
                .add(crate::task::FUTURE_SLOT_STATE)
                .read()
        };
        assert_eq!(
            crate::gc::object_info(state),
            Some((CONNECT_STATE_SIZE, true)),
            "the state object must be the ABI minimum plus the one temp slot \
             holding the address pointer and later the socket, scanned"
        );
        // SAFETY: same block.
        let poll = unsafe {
            (fut as *mut usize)
                .add(crate::task::FUTURE_SLOT_POLL)
                .read()
        };
        let expected: PollFn = poll_connect;
        assert_eq!(
            poll, expected as usize,
            "word 0 must be the poll function's address, not the state's"
        );
    }

    /// This module's own panic-freedom claim, pinned the same mechanical way
    /// `file.rs`'s `no_file_intrinsic_can_panic` and this crate's other
    /// sibling guards pin theirs: nothing in this file's production code may
    /// panic, since every intrinsic here is reachable from a generated poll
    /// boundary with no landing pad to unwind through.
    ///
    /// Scans only the part of this file before its own `mod tests` block, for
    /// the same reason and with the same ceiling `file.rs`'s identical guard
    /// documents: the *first* occurrence of the split literal is the real
    /// boundary, and this fails open rather than distinguishing a safe `[i]`
    /// from a dangerous one.
    #[test]
    fn no_net_intrinsic_can_panic() {
        let source = include_str!("net.rs");
        let production = source.split("mod tests {").next().unwrap_or(source);
        for needle in [
            ".borrow_mut()",
            ".borrow()",
            "unwrap()",
            ".expect(",
            "panic!",
            "format!",
        ] {
            assert!(
                !production.contains(needle),
                "a std/net intrinsic must not panic: `{needle}` found in this \
                 file's production code, which is reachable from a generated \
                 poll boundary with no landing pad to unwind through"
            );
        }
    }
}
