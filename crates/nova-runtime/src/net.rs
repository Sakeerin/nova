//! Open TCP connections, keyed by descriptor; the two-phase, non-blocking
//! `connect` that populates the table; and, since Task 4, the `read`,
//! `write`, and `read_timeout` futures that act on it.
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
use std::sync::OnceLock;
use std::time::{Duration, Instant};

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
/// The identical encoding `file.rs`'s own `stash_i64` (and `io.rs`'s
/// `stash_count`) use, reproduced here rather than shared across modules for
/// the same reason those give: a second, self-contained implementation of one
/// small encoding costs less than reaching past the four names this module
/// imports from `fs`. Used here for two different payloads across this
/// module's life -- `connect`'s new fd ([`finish_connect`]) and, since Task 4,
/// `write`'s reported byte count ([`try_write`]) -- the same dual use
/// `file.rs`'s own copy of this function already has for `open`'s fd and
/// `write`'s count.
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

// ---------------------------------------------------------------------------
// `read`, `write`, and `read_timeout` -- Task 4.
//
// Each of these parks on `Interest::Read`/`Interest::Write` exactly the way
// `connect` parks on `Interest::Write` above, through the same
// `stage_io_park` seam. But unlike `connect` -- whose first poll (issue the
// syscall) and second (check `SO_ERROR`) are two genuinely different
// operations -- the retried operation here (a non-blocking `Read::read` or
// `Write::write` call) is the *same* call on every poll. Neither `read` nor
// `write` needs `connect`'s `STATE_SLOT_TAG` phase switch for that reason:
// repeating the same attempt is exactly correct whether it is the first poll
// or the fifth, and trying it before ever parking is what gives both of them
// `poll_join`'s own optimisation for free -- data (or room) already waiting
// completes on the very first poll, with no park staged at all.
//
// `read_timeout` still keeps a resume tag, but not to pick a different
// *operation* the way `connect` does -- only to compute its absolute deadline
// exactly once, the first time it is ever polled. See `poll_read_timeout`'s
// own doc comment for why distinguishing "ready" from "timed out" cannot ride
// the poll call itself and has to be re-derived instead.
// ---------------------------------------------------------------------------

/// What one non-blocking `Read::read` attempt against a socket produced.
enum ReadStep {
    /// The attempt settled: success (bytes stashed via `Slot::Buffer`,
    /// truncated to what was actually read -- an **empty** result is EOF, a
    /// **short** one is not) or a real I/O error (`fail`'s status) or the fd
    /// was not open (`closed_fd_error`). Either way there is nothing left to
    /// do but report this status.
    Done(i64),
    /// The fd is open but has nothing to read right now. Carries the raw
    /// socket a caller should park on for `Interest::Read`.
    WouldBlock(RawSocket),
}

/// One non-blocking `Read::read` attempt against `fd`, truncating and
/// stashing the result via `Slot::Buffer` on success.
///
/// Shared by [`poll_read`] and [`poll_read_timeout`] -- the two futures this
/// module builds that read differ only in what they do with a
/// [`ReadStep::WouldBlock`] (park with no deadline, or park with one and
/// separately watch for it passing), never in how the read itself works.
///
/// **An empty result is EOF, and a short read is not** -- the `truncate`
/// below keeps exactly what `Read::read` reported, never padding with the
/// rest of `buf`'s zeroed capacity. The Nova-level contract this mirrors is
/// `io.rs`'s own `read_and_stash` and `file.rs`'s `nova_rt_file_read`: the
/// test is `len() == 0`, never `len() < max`. Getting this backwards --
/// checking `n < max` instead of stashing exactly `n` bytes -- truncates
/// silently rather than hanging, measured twice on the `byte-type` branch
/// (this task's own brief carries the same warning).
///
/// Guards `max` the way `nova_rt_io_stdin_read` and `nova_rt_file_read` do: a
/// negative value aborts rather than wrapping into an enormous allocation
/// request. A large `max` still allocates the whole capacity eagerly before
/// any read happens, the identical known asymmetry those two already carry.
fn try_read(fd: i64, max: i64) -> ReadStep {
    let Ok(cap) = usize::try_from(max) else {
        crate::task::abort_with("nova_rt_net: read: negative maximum")
    };
    let mut buf = vec![0u8; cap];
    let outcome = with_fd(fd, |stream| match std::io::Read::read(stream, &mut buf) {
        Ok(n) => {
            buf.truncate(n);
            stash(Slot::Buffer, crate::bytes::gc_bytes(&buf));
            ReadStep::Done(OK)
        }
        Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
            ReadStep::WouldBlock(raw_socket_of(stream))
        }
        Err(e) => ReadStep::Done(fail(&e)),
    });
    match outcome {
        Some(step) => step,
        None => ReadStep::Done(closed_fd_error()),
    }
}

/// What one non-blocking `Write::write` attempt against a socket produced.
/// Mirrors [`ReadStep`] exactly, against the opposite direction.
enum WriteStep {
    Done(i64),
    WouldBlock(RawSocket),
}

/// One non-blocking `Write::write` attempt against `fd`, stashing the byte
/// count actually written via [`stash_i64`] on success.
///
/// **May write fewer bytes than given.** One `Write::write` call, not a
/// `write_all` loop -- deliberately unlike `std/fs`'s `write`, which promises
/// no partial write. See `io.rs`'s own `write_and_stash` doc comment for the
/// full contract this mirrors; a caller of this boundary that wants a
/// guaranteed full write must loop on the returned count itself.
fn try_write(fd: i64, bytes: &[u8]) -> WriteStep {
    let outcome = with_fd(fd, |stream| match std::io::Write::write(stream, bytes) {
        Ok(n) => {
            stash_i64(n as i64);
            WriteStep::Done(OK)
        }
        Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
            WriteStep::WouldBlock(raw_socket_of(stream))
        }
        Err(e) => WriteStep::Done(fail(&e)),
    });
    match outcome {
        Some(step) => step,
        None => WriteStep::Done(closed_fd_error()),
    }
}

/// Where a `read` future keeps its `fd` and `max` between polls.
const READ_SLOT_FD: usize = STATE_SLOT_TEMPS;
const READ_SLOT_MAX: usize = STATE_SLOT_TEMPS + 1;

/// State size for a read future: the ABI minimum plus the two temp slots
/// holding `fd` and `max`.
const READ_STATE_SIZE: usize = STATE_MIN_SIZE + 16;

const _: () = assert!(READ_STATE_SIZE >= (READ_SLOT_MAX + 1) * 8);

/// The read future's poll function.
///
/// **No resume tag, unlike `connect`'s `poll_connect`.** See this section's
/// own header comment for why: attempting the read is the same operation on
/// every poll, so repeating it is always correct, and doing so before ever
/// parking is what gives this future `poll_join`'s own optimisation for free
/// -- if the socket already has data (or is at EOF), [`try_read`] returns
/// [`ReadStep::Done`] on the very first call and this never parks at all.
///
/// Must not unwind, as every other `PollFn` in this crate must not: every
/// fallible step -- the borrow inside [`try_read`]/`with_fd`, and the I/O call
/// itself -- already returns a value this function maps into a status rather
/// than unwrapping.
unsafe extern "C-unwind" fn poll_read(state: *mut u8, _task_ctx: *mut u8) -> i64 {
    let slots = state as *mut i64;
    // SAFETY: `state` is the object `nova_rt_net_read_future` built, at least
    // `READ_STATE_SIZE` bytes, so both slots below are in bounds.
    let fd = unsafe { slots.add(READ_SLOT_FD).read() };
    let max = unsafe { slots.add(READ_SLOT_MAX).read() };
    match try_read(fd, max) {
        ReadStep::Done(status) => {
            // SAFETY: same object, output slot.
            unsafe { slots.add(STATE_SLOT_OUTPUT).write(status) };
            POLL_READY
        }
        ReadStep::WouldBlock(socket) => {
            stage_io_park(socket, Interest::Read, None);
            POLL_PENDING
        }
    }
}

/// A fresh `Future<Int>` (a status, per this module's boundary design) that
/// reads up to `max` bytes from `fd`, non-blockingly. On success, the bytes
/// actually read -- truncated to what was available; empty means EOF, a short
/// read is not -- are stashed via `Slot::Buffer`, exactly as `file.rs`'s
/// `nova_rt_file_read` stashes its own.
///
/// **The state object is fresh on every call**, for the same reason
/// `nova_rt_net_connect_future`'s own doc comment gives: the whole value
/// carried across a suspension is this state object's own `fd`/`max`, so two
/// reads in flight at once would otherwise corrupt each other.
///
/// # Safety
/// No pointer argument, so no dereference precondition beyond `build_future`'s
/// own; marked `unsafe extern "C-unwind"` for uniformity with this module's
/// other future constructors.
#[no_mangle]
pub unsafe extern "C-unwind" fn nova_rt_net_read_future(fd: i64, max: i64) -> *mut u8 {
    let poll: PollFn = poll_read;
    build_future(poll, READ_STATE_SIZE, |slots| {
        // SAFETY: `slots` addresses a live `READ_STATE_SIZE` block, and both
        // slots are in bounds by the assertion above.
        unsafe {
            slots.add(READ_SLOT_FD).write(fd);
            slots.add(READ_SLOT_MAX).write(max);
        }
    })
}

/// Where a `write` future keeps its `fd` and its `bytes` pointer between
/// polls. Unlike `CONNECT_SLOT_SOCK`, this slot is never overwritten across
/// the future's life -- there is only ever one phase.
const WRITE_SLOT_FD: usize = STATE_SLOT_TEMPS;
const WRITE_SLOT_BYTES: usize = STATE_SLOT_TEMPS + 1;

/// State size for a write future: the ABI minimum plus the two temp slots
/// holding `fd` and the `bytes` pointer.
const WRITE_STATE_SIZE: usize = STATE_MIN_SIZE + 16;

const _: () = assert!(WRITE_STATE_SIZE >= (WRITE_SLOT_BYTES + 1) * 8);

/// The write future's poll function. Mirrors [`poll_read`] exactly, against
/// the opposite direction and interest -- see that function's own doc comment
/// for why neither needs a resume tag.
unsafe extern "C-unwind" fn poll_write(state: *mut u8, _task_ctx: *mut u8) -> i64 {
    let slots = state as *mut i64;
    // SAFETY: `state` is the object `nova_rt_net_write_future` built, at least
    // `WRITE_STATE_SIZE` bytes, so both slots below are in bounds.
    let fd = unsafe { slots.add(WRITE_SLOT_FD).read() };
    let ptr = unsafe { slots.add(WRITE_SLOT_BYTES).read() } as *const NovaStr;
    // SAFETY: `ptr` is the `NovaStr` `nova_rt_net_write_future` was given,
    // kept alive by this state object's own GC root and scan -- this slot is
    // never overwritten across this future's whole life, so it is still the
    // original pointer on every poll.
    let bytes = unsafe { crate::bytes::as_bytes(ptr) };
    match try_write(fd, bytes) {
        WriteStep::Done(status) => {
            // SAFETY: same object, output slot.
            unsafe { slots.add(STATE_SLOT_OUTPUT).write(status) };
            POLL_READY
        }
        WriteStep::WouldBlock(socket) => {
            stage_io_park(socket, Interest::Write, None);
            POLL_PENDING
        }
    }
}

/// A fresh `Future<Int>` that writes `bytes` to `fd`, non-blockingly, possibly
/// fewer of them than given (see [`try_write`]'s own doc comment). On
/// success, the byte count actually written is stashed via `Slot::Buffer`,
/// the identical 8-byte little-endian encoding [`stash_i64`] already uses for
/// `connect`'s own fd.
///
/// # Safety
/// `bytes` must point to a live `NovaStr`.
#[no_mangle]
pub unsafe extern "C-unwind" fn nova_rt_net_write_future(
    fd: i64,
    bytes: *const NovaStr,
) -> *mut u8 {
    let poll: PollFn = poll_write;
    build_future(poll, WRITE_STATE_SIZE, |slots| {
        // SAFETY: `slots` addresses a live `WRITE_STATE_SIZE` block, and both
        // slots are in bounds by the assertion above.
        unsafe {
            slots.add(WRITE_SLOT_FD).write(fd);
            slots.add(WRITE_SLOT_BYTES).write(bytes as i64);
        }
    })
}

/// A fixed point in time `read_timeout`'s deadline arithmetic measures
/// against, lazily fixed on first use.
///
/// The same technique `poll.rs`'s own (private) `log_epoch` uses, reproduced
/// here rather than shared across modules -- only relative elapsed time is
/// ever compared against it, so an arbitrary origin is fine, and this
/// module's reason to want one is different from that one's (rate-limiting a
/// log line there; encoding a deadline as a plain, scannable `i64` here). A
/// `std::time::Instant` has no documented byte layout this module could
/// safely write into one of its own state slots directly the way it writes a
/// plain fd or count there -- so `read_timeout` stores milliseconds-since-
/// this-epoch instead, the same spirit as `CONNECT_SLOT_SOCK` storing a plain
/// fd rather than a `TcpStream`.
fn deadline_epoch() -> Instant {
    static EPOCH: OnceLock<Instant> = OnceLock::new();
    *EPOCH.get_or_init(Instant::now)
}

/// Milliseconds elapsed since [`deadline_epoch`], as a plain `i64` a
/// `read_timeout` future's state object can hold in a scanned slot.
///
/// `i64::MAX` on the (astronomically distant) overflow case rather than
/// unwrapping: this runs inside a generated poll boundary with no landing
/// pads, the same reason every other fallible conversion in this module falls
/// back to a value instead of panicking.
fn now_epoch_ms() -> i64 {
    i64::try_from(deadline_epoch().elapsed().as_millis()).unwrap_or(i64::MAX)
}

/// An `Instant` `remaining_ms` milliseconds from now, clamping a non-positive
/// value to "now" -- the identical clamp `task.rs`'s own (private)
/// `deadline_from_ms` applies, for the identical reason: nothing stops a
/// remaining duration computed from a stored deadline from having already
/// reached zero, or gone negative, by the time this runs.
fn instant_from_remaining_ms(remaining_ms: i64) -> Instant {
    let ms = u64::try_from(remaining_ms).unwrap_or(0);
    Instant::now() + Duration::from_millis(ms)
}

/// Where a `read_timeout` future keeps its state between polls.
///
/// [`RT_SLOT_DEADLINE`] is reused across the future's life exactly the way
/// `CONNECT_SLOT_SOCK` reuses its own slot: before the first poll it holds
/// the raw `ms` argument; the first poll reads that out and overwrites the
/// same slot with the *absolute* deadline (epoch-relative milliseconds,
/// [`now_epoch_ms`] plus `ms`), for every later poll to compare against.
/// Computing the absolute deadline at first-poll time rather than at
/// construction matches `task.rs`'s own `nova_rt_task_sleep_future`: a future
/// built but not immediately polled should time out `ms` after it *starts
/// running*, not after it was merely constructed.
const RT_SLOT_FD: usize = STATE_SLOT_TEMPS;
const RT_SLOT_MAX: usize = STATE_SLOT_TEMPS + 1;
const RT_SLOT_DEADLINE: usize = STATE_SLOT_TEMPS + 2;

/// State size for a `read_timeout` future: the ABI minimum plus the three
/// temp slots holding `fd`, `max`, and the reused ms-then-deadline slot.
const RT_STATE_SIZE: usize = STATE_MIN_SIZE + 24;

const _: () = assert!(RT_STATE_SIZE >= (RT_SLOT_DEADLINE + 1) * 8);

/// The `read_timeout` future's poll function.
///
/// **How this distinguishes "ready" from "timed out" with no help from the
/// call itself.** The poll ABI is frozen and `task_ctx` is always null (see
/// `task.rs`'s own `PollFn` doc comment), and `task.rs`'s wake paths
/// (`wake_ready`/`wake_due`) just move a parked task back onto the ready
/// queue -- neither records *why* for this function to read back on its next
/// call. So this never asks; it re-derives the answer directly, the same way
/// `finish_connect` re-derives a refusal from `SO_ERROR` rather than being
/// told one occurred. Every poll retries [`try_read`] first -- if data (or
/// EOF) is there, that settles it regardless of what woke this task. Only a
/// [`ReadStep::WouldBlock`] needs the second check: has [`RT_SLOT_DEADLINE`]
/// (an absolute deadline by now) already passed? If so, this reports
/// `TIMED_OUT` (via `fail`, so a friendly message is stashed exactly like
/// every other non-`OK` status here) -- the status constant `fs.rs` has
/// carried since increment 1 with no producer until now. If not, this is a
/// spurious wake (this crate's own poller should never produce one for a
/// deadline that has not passed, but nothing here assumes that): re-stage the
/// same wait, with whatever time is actually left, and try again next poll.
unsafe extern "C-unwind" fn poll_read_timeout(state: *mut u8, _task_ctx: *mut u8) -> i64 {
    let slots = state as *mut i64;
    // SAFETY: `state` is the object `nova_rt_net_read_timeout_future` built,
    // at least `RT_STATE_SIZE` bytes, so every slot below is in bounds.
    let fd = unsafe { slots.add(RT_SLOT_FD).read() };
    let max = unsafe { slots.add(RT_SLOT_MAX).read() };
    let tag = unsafe { slots.add(STATE_SLOT_TAG).read() };
    let deadline_ms = if tag == 0 {
        // SAFETY: same object; this slot has held the raw `ms` argument since
        // construction, and this is the first poll, so nothing has
        // overwritten it yet.
        let ms = unsafe { slots.add(RT_SLOT_DEADLINE).read() };
        let deadline = now_epoch_ms().saturating_add(ms.max(0));
        // SAFETY: same object; the address argument has already been read (it
        // never lived here in the first place -- `fd`/`max` are separate
        // slots), so overwriting this slot with the absolute deadline loses
        // nothing a later poll still needs.
        unsafe {
            slots.add(RT_SLOT_DEADLINE).write(deadline);
            slots.add(STATE_SLOT_TAG).write(1);
        }
        deadline
    } else {
        // SAFETY: same object; an earlier poll already overwrote this slot
        // with the absolute deadline.
        unsafe { slots.add(RT_SLOT_DEADLINE).read() }
    };

    match try_read(fd, max) {
        ReadStep::Done(status) => {
            // SAFETY: same object, output slot.
            unsafe { slots.add(STATE_SLOT_OUTPUT).write(status) };
            POLL_READY
        }
        ReadStep::WouldBlock(socket) => {
            let remaining_ms = deadline_ms.saturating_sub(now_epoch_ms());
            if remaining_ms <= 0 {
                let status = fail(&std::io::Error::new(
                    std::io::ErrorKind::TimedOut,
                    "read_timeout: the deadline passed before any data was available",
                ));
                // SAFETY: same object, output slot.
                unsafe { slots.add(STATE_SLOT_OUTPUT).write(status) };
                return POLL_READY;
            }
            stage_io_park(
                socket,
                Interest::Read,
                Some(instant_from_remaining_ms(remaining_ms)),
            );
            POLL_PENDING
        }
    }
}

/// A fresh `Future<Int>` that reads up to `max` bytes from `fd`,
/// non-blockingly, reporting `TIMED_OUT` if `ms` milliseconds pass with
/// nothing to read first. Otherwise identical to [`nova_rt_net_read_future`],
/// including EOF/short-read semantics and where the bytes land on success.
///
/// Stages `Wait::Io { socket, interest: Interest::Read, deadline: Some(_) }`
/// through `task.rs`'s `stage_io_park` seam -- the only operation in this
/// module that ever passes a deadline through it; `connect`, plain `read`,
/// and plain `write` each stage `None`.
///
/// # Safety
/// No pointer argument, so no dereference precondition beyond `build_future`'s
/// own.
#[no_mangle]
pub unsafe extern "C-unwind" fn nova_rt_net_read_timeout_future(
    fd: i64,
    max: i64,
    ms: i64,
) -> *mut u8 {
    let poll: PollFn = poll_read_timeout;
    build_future(poll, RT_STATE_SIZE, |slots| {
        // SAFETY: `slots` addresses a live `RT_STATE_SIZE` block, and every
        // slot here is in bounds by the assertion above.
        unsafe {
            slots.add(RT_SLOT_FD).write(fd);
            slots.add(RT_SLOT_MAX).write(max);
            slots.add(RT_SLOT_DEADLINE).write(ms);
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fs::{CONNECTION_REFUSED, TIMED_OUT};

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

    /// Test-only: whether `fd` is currently a live entry in the socket
    /// table -- the direct check for "absence from the table is
    /// closedness," without needing a read to distinguish "closed" from
    /// "open but idle" (a `WouldBlock` on a still-open, still-idle socket
    /// maps to the same non-`OK` status a closed one does -- both fall to
    /// `fail`'s `_ => OTHER` arm).
    fn is_open_for_test(fd: i64) -> bool {
        with_fd(fd, |_| ()).is_some()
    }

    /// Test-only: the `(PollFn, state)` pair inside a future's `{ poll_code,
    /// state }` fat pointer, so a test can invoke a future's own poll
    /// function directly -- the same extraction
    /// `connect_parks_on_its_first_poll_rather_than_completing_synchronously`
    /// inlines for itself, factored out here since several of this module's
    /// `read`/`write`/`read_timeout` tests need it too.
    fn poll_fn_and_state(fut: *mut u8) -> (PollFn, *mut u8) {
        // SAFETY: `fut` is a well-formed `{ poll_code, state }` fat pointer
        // this module built.
        let words = fut as *mut usize;
        let poll_code = unsafe { words.add(crate::task::FUTURE_SLOT_POLL).read() };
        let state = unsafe { words.add(crate::task::FUTURE_SLOT_STATE).read() } as *mut u8;
        // SAFETY: `poll_code` is a `PollFn` bit pattern by `fut`'s own
        // construction; a function pointer and a `usize` are both
        // pointer-width.
        let poll: PollFn = unsafe { std::mem::transmute(poll_code) };
        (poll, state)
    }

    /// Test-only: drain whatever this thread's `task`-module park staging
    /// currently holds, by driving one real, never-parking poll through the
    /// real executor.
    ///
    /// A test that calls a future's poll function directly (via
    /// [`poll_fn_and_state`]) rather than through `nova_rt_task_block_on`
    /// bypasses `poll_one`'s own cleanup -- see
    /// `connect_parks_on_its_first_poll_rather_than_completing_synchronously`'s
    /// doc comment for the same observation about its own manual poll. That
    /// test immediately follows up with a real `block_on` call anyway, which
    /// drains the leftover as a side effect; a test here that does *not* go
    /// on to drive the same future through the real executor needs this
    /// instead, so the stray entry cannot collide with whatever a *later*
    /// test -- potentially sharing this OS thread, since `cargo test`'s
    /// worker threads are reused across test functions -- stages next.
    /// `poll_one` takes the staged park unconditionally after every poll it
    /// runs, regardless of what it finds, which is what makes one harmless
    /// pump call enough.
    fn drain_stray_pending_park_for_test() {
        let pump: PollFn = poll_ready_immediately_for_test;
        let pump_fut = build_future(pump, STATE_MIN_SIZE, |_| {});
        assert_eq!(
            unsafe { crate::task::nova_rt_task_block_on(pump_fut) },
            0,
            "test cleanup: the pump future must complete immediately"
        );
    }

    /// Test-only: take the pending `Slot::Buffer` payload as owned bytes --
    /// the `Bytes` counterpart to [`take_fd`], for comparing a read's result
    /// against an expected slice. Matches `file.rs`'s own `take_bytes` test
    /// helper.
    fn take_bytes_for_test() -> Vec<u8> {
        let ptr = crate::fs::take_for_test(Slot::Buffer) as *const NovaStr;
        // SAFETY: test-only, for the reason `take_fd`'s comment gives: this
        // same test stashed the payload earlier, with nothing allocated
        // since.
        unsafe { crate::bytes::as_bytes(ptr) }.to_vec()
    }

    /// Test-only: drive `fut` to completion via `nova_rt_task_spawn`, then
    /// pump this thread's queue with an unrelated, immediately-ready future
    /// through `nova_rt_task_block_on` -- the identical spawn-then-pump
    /// technique
    /// `a_successful_connect_stashes_its_fd_via_slot_buffer` uses (see that
    /// test's own doc comment for why `block_on` cannot be called on `fut`
    /// directly when a caller still needs to read its `fs::Slot::Buffer`
    /// payload afterward: `block_on` releases it as part of returning).
    /// Returns `fut`'s own task id (for reading its `Slot::Buffer` back via
    /// [`with_test_current`]) and its final status.
    fn spawn_and_pump_for_test(fut: *mut u8) -> (i64, i64) {
        let id = unsafe { crate::task::nova_rt_task_spawn(fut) };
        let pump: PollFn = poll_ready_immediately_for_test;
        let pump_fut = build_future(pump, STATE_MIN_SIZE, |_| {});
        assert_eq!(unsafe { crate::task::nova_rt_task_block_on(pump_fut) }, 0);
        assert_ne!(
            unsafe { crate::task::nova_rt_task_is_done(fut) },
            0,
            "test setup: the task must have been driven to completion as a \
             side effect of draining this thread's whole queue"
        );
        (id, state_slot_of(fut, STATE_SLOT_OUTPUT))
    }

    /// Test-only: write `chunk` to `fd` repeatedly, through the shared
    /// [`try_write`] helper directly (never `block_on`, which would hang this
    /// test's own thread forever the moment a write genuinely needs to park
    /// and nothing ever drains the peer), until one call reports
    /// `WouldBlock`. Returns nothing -- callers only need the buffer to have
    /// reached that state, not any particular count.
    ///
    /// **Why a loop of real writes, not a shrunk `SO_SNDBUF`.** A `setsockopt`
    /// shrinking the send buffer to a few hundred bytes was tried first and
    /// measured to have no effect on this task's own Windows host: a single
    /// 256 KiB write against a socket with `SO_SNDBUF` set to 1024 still
    /// reported the whole count accepted, and separately, repeated writes
    /// against an un-drained peer were measured to be all-or-nothing on this
    /// platform's loopback -- every call before the ceiling reports its full
    /// requested count, and the first call at the ceiling reports
    /// `WouldBlock` outright, never a partial count (this task's own report
    /// records the probe). A generous iteration cap keeps this fast in
    /// practice: reaching the ceiling took 2-8 calls in every measurement
    /// that produced this comment.
    fn fill_send_buffer_until_would_block_for_test(fd: i64, chunk: &[u8]) {
        for _ in 0..256 {
            match try_write(fd, chunk) {
                WriteStep::Done(status) => {
                    assert_eq!(status, OK, "a write that is not WouldBlock must succeed")
                }
                WriteStep::WouldBlock(_) => return,
            }
        }
        panic!(
            "test setup: the send buffer never filled within 256 writes of \
             {} bytes each -- nothing drained the peer, so it must eventually",
            chunk.len()
        );
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
        // Absence from the table IS closedness. Driven through the real
        // production `read` future (Task 4), not a test-only bespoke read --
        // this closed fd never has data waiting, so its `with_fd` `None` arm
        // resolves on the very first poll, with no park involved.
        let read_status =
            unsafe { crate::task::nova_rt_task_block_on(nova_rt_net_read_future(fd, 1)) };
        assert_ne!(read_status, OK, "a read on a closed fd must fail");
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

    // -----------------------------------------------------------------------
    // `read`, `write`, and `read_timeout` -- Task 4.
    // -----------------------------------------------------------------------

    #[test]
    fn a_read_with_no_data_parks_and_completes_when_data_arrives() {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind");
        let addr = listener.local_addr().expect("addr").to_string();
        let fd = connect_blocking_for_test(&addr);
        let (mut server, _) = listener.accept().expect("accept");
        let expected_socket = with_fd(fd, |stream| raw_socket_of(stream)).expect("fd must be open");

        let fut = unsafe { nova_rt_net_read_future(fd, 64) };
        let (poll, state) = poll_fn_and_state(fut);

        // First poll: nothing to read, so the future must park rather than
        // spin -- this project's own busy-spin hazard (this task's own
        // brief names it by name). A mutation that treats "would block" as
        // EOF, or that just re-attempts the read forever without ever
        // staging a park, would both still eventually see the data written
        // below and report the right bytes; only the *return value of this
        // one poll* tells a real park apart from either -- **and only
        // checking that return value is not enough either**: a future that
        // strips its own `stage_io_park` call (returning `POLL_PENDING` with
        // nothing staged at all) still returns `POLL_PENDING` here and still
        // completes with the right bytes once driven through the real
        // executor's busy re-queue, since `QUEUE` alone (not `PARKED`) would
        // keep re-polling it. So this also reads back what was actually
        // staged, not only the poll's return value.
        let first = with_test_current(0, || unsafe { poll(state, std::ptr::null_mut()) });
        assert_eq!(
            first, POLL_PENDING,
            "a read with no data waiting must park rather than complete \
             synchronously"
        );
        assert_eq!(
            crate::task::staged_io_for_test(),
            Some((expected_socket, Interest::Read, None)),
            "a read with no data waiting must genuinely stage a park on \
             Interest::Read for this exact socket, with no deadline -- not \
             merely return POLL_PENDING while parking nothing"
        );

        std::io::Write::write_all(&mut server, b"hello").expect("write");

        let second = with_test_current(0, || unsafe { poll(state, std::ptr::null_mut()) });
        assert_eq!(second, POLL_READY);
        assert_eq!(
            unsafe { (state as *mut i64).add(STATE_SLOT_OUTPUT).read() },
            OK
        );
        let bytes = with_test_current(0, take_bytes_for_test);
        assert_eq!(bytes, b"hello");

        drain_stray_pending_park_for_test();
        assert_eq!(unsafe { nova_rt_net_close(fd) }, OK);
    }

    /// The gap the single-poll check above cannot see: nothing yet drove a
    /// future through a *second* would-blocking poll while still parked, so
    /// a `poll_read` that stages correctly the first time and silently skips
    /// staging on every later `WouldBlock` -- "stage once ever" -- would
    /// still pass every other test in this file. In production that shape is
    /// a real hang: a task woken spuriously (this module's own comments on
    /// `poll_read_timeout` already anticipate one) with no data actually
    /// having arrived would re-poll, find `WouldBlock` again, and never
    /// re-register interest -- nothing is left to wake it a second time.
    ///
    /// Drains between the two polls (`drain_stray_pending_park_for_test`):
    /// `Staged` aborts on a same-kind pair (`try_stage`'s own doc comment),
    /// which is exactly what makes the second `stage_io_park` call
    /// observable at all rather than an immediate process abort.
    #[test]
    fn a_read_still_blocked_on_its_second_poll_stages_a_park_again() {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind");
        let addr = listener.local_addr().expect("addr").to_string();
        let fd = connect_blocking_for_test(&addr);
        let (_server, _) = listener.accept().expect("accept");
        let expected_socket = with_fd(fd, |stream| raw_socket_of(stream)).expect("fd must be open");

        let fut = unsafe { nova_rt_net_read_future(fd, 64) };
        let (poll, state) = poll_fn_and_state(fut);

        let first = with_test_current(0, || unsafe { poll(state, std::ptr::null_mut()) });
        assert_eq!(first, POLL_PENDING, "test setup: the first poll must park");
        assert_eq!(
            crate::task::staged_io_for_test(),
            Some((expected_socket, Interest::Read, None)),
            "test setup: the first poll must stage a park"
        );

        // No data was written on the peer in between: simulate a wake with
        // nothing actually having arrived (a spurious wake, or the socket
        // looking briefly ready and then not).
        drain_stray_pending_park_for_test();

        let second = with_test_current(0, || unsafe { poll(state, std::ptr::null_mut()) });
        assert_eq!(
            second, POLL_PENDING,
            "still no data on the second poll -- must still park, not \
             complete"
        );
        assert_eq!(
            crate::task::staged_io_for_test(),
            Some((expected_socket, Interest::Read, None)),
            "a still-blocked read must stage a park again on its second \
             poll, not only its first -- a future that stages once and \
             never again leaves a spuriously-woken task parked forever with \
             nothing left to wake it"
        );

        drain_stray_pending_park_for_test();
        assert_eq!(unsafe { nova_rt_net_close(fd) }, OK);
    }

    #[test]
    fn a_read_with_data_already_waiting_completes_on_the_first_poll_without_parking() {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind");
        let addr = listener.local_addr().expect("addr").to_string();
        let fd = connect_blocking_for_test(&addr);
        let (mut server, _) = listener.accept().expect("accept");
        std::io::Write::write_all(&mut server, b"hi").expect("write");

        let fut = unsafe { nova_rt_net_read_future(fd, 64) };
        let (poll, state) = poll_fn_and_state(fut);
        let status = with_test_current(0, || unsafe { poll(state, std::ptr::null_mut()) });
        assert_eq!(
            status, POLL_READY,
            "data already waiting must complete on the very first poll, not \
             park -- poll_join's own optimisation, applied here to a read"
        );
        assert_eq!(
            unsafe { (state as *mut i64).add(STATE_SLOT_OUTPUT).read() },
            OK
        );
        let bytes = with_test_current(0, take_bytes_for_test);
        assert_eq!(bytes, b"hi");

        assert_eq!(unsafe { nova_rt_net_close(fd) }, OK);
    }

    /// EOF is an **empty** result, and a successful one -- never mistaken for
    /// a failure status. Driven through `read_timeout` rather than a single
    /// manual poll of plain `read`: closing the peer delivers EOF
    /// asynchronously (a FIN this process's own non-blocking read may not
    /// observe for a brief moment), and a generous timeout lets the real
    /// executor's park/wake loop absorb that instead of this test racing it
    /// with one bare poll.
    #[test]
    fn an_eof_read_stashes_an_empty_payload_not_an_error() {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind");
        let addr = listener.local_addr().expect("addr").to_string();
        let fd = connect_blocking_for_test(&addr);
        let (server, _) = listener.accept().expect("accept");
        drop(server);

        let fut = unsafe { nova_rt_net_read_timeout_future(fd, 64, 2_000) };
        let (id, status) = spawn_and_pump_for_test(fut);
        assert_eq!(status, OK, "EOF is a successful read, not a failure status");
        let bytes = with_test_current(id, take_bytes_for_test);
        assert_eq!(
            bytes, b"" as &[u8],
            "an empty read must stash a zero-length payload -- the EOF signal"
        );

        assert_eq!(unsafe { nova_rt_net_close(fd) }, OK);
    }

    /// [`try_write`] directly, decoupled from the futures/executor machinery:
    /// writing repeatedly with nothing ever draining the peer must
    /// eventually report `WouldBlock`, not succeed forever. This is the
    /// unit-level half of the busy-spin hazard this task's own brief names;
    /// [`a_write_against_a_full_send_buffer_parks_rather_than_spinning`]
    /// below is the future/executor-level half.
    ///
    /// **What this does not (and, measured on this task's own Windows host,
    /// cannot reliably) show:** that a single write ever reports *fewer*
    /// bytes than given. `try_write`'s own doc comment states that contract
    /// and it is real -- `Write::write`, not `write_all`, exactly mirroring
    /// `io.rs`'s identical choice -- but every write below either succeeds
    /// with its whole requested count or reports `WouldBlock` outright with
    /// none; a probe built to force a partial count (shrinking `SO_SNDBUF`,
    /// then varying write sizes across the boundary it should create) never
    /// produced one on this host, this task's own report records the
    /// numbers.
    #[test]
    fn try_write_reports_would_block_once_the_send_buffer_fills() {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind");
        let addr = listener.local_addr().expect("addr").to_string();
        let fd = connect_blocking_for_test(&addr);
        let (_server, _) = listener.accept().expect("accept");

        let chunk = vec![0xABu8; 1024 * 1024];
        fill_send_buffer_until_would_block_for_test(fd, &chunk);

        assert_eq!(unsafe { nova_rt_net_close(fd) }, OK);
    }

    #[test]
    fn a_write_against_a_full_send_buffer_parks_rather_than_spinning() {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind");
        let addr = listener.local_addr().expect("addr").to_string();
        let fd = connect_blocking_for_test(&addr);
        let (_server, _) = listener.accept().expect("accept");
        let expected_socket = with_fd(fd, |stream| raw_socket_of(stream)).expect("fd must be open");

        let chunk_bytes = vec![0xABu8; 1024 * 1024];
        // Fill the send buffer via repeated direct `try_write` calls first --
        // not `block_on`, which would hang this test's own thread forever
        // once a write genuinely needs to park and nothing ever drains the
        // peer (see `fill_send_buffer_until_would_block_for_test`'s own doc
        // comment).
        fill_send_buffer_until_would_block_for_test(fd, &chunk_bytes);

        // Now the production write future, against the same still-full
        // buffer, must park rather than spin or silently fail -- this
        // project's own busy-spin hazard, on the write side this time.
        // Matches
        // `connect_parks_on_its_first_poll_rather_than_completing_synchronously`'s
        // own technique: only the *return value of one poll* tells a real
        // park apart from a mutation that would still eventually succeed via
        // `block_on` alone -- **and even that return value is not enough on
        // its own**: a future that strips its own `stage_io_park` call still
        // returns `POLL_PENDING` here and would still eventually succeed
        // through the real executor's busy re-queue, since nothing would
        // ever move it into `PARKED` at all. This also reads back what was
        // actually staged.
        let chunk = crate::gc_bytes_for_test(&chunk_bytes);
        let fut = unsafe { nova_rt_net_write_future(fd, chunk) };
        let (poll, state) = poll_fn_and_state(fut);
        let status = with_test_current(0, || unsafe { poll(state, std::ptr::null_mut()) });
        assert_eq!(
            status, POLL_PENDING,
            "a write against a full send buffer must park rather than spin \
             or report a wrong status"
        );
        assert_eq!(
            crate::task::staged_io_for_test(),
            Some((expected_socket, Interest::Write, None)),
            "a write against a full send buffer must genuinely stage a park \
             on Interest::Write for this exact socket, with no deadline -- \
             not merely return POLL_PENDING while parking nothing"
        );

        drain_stray_pending_park_for_test();
        assert_eq!(unsafe { nova_rt_net_close(fd) }, OK);
    }

    /// The write-side counterpart of
    /// [`a_read_still_blocked_on_its_second_poll_stages_a_park_again`] -- see
    /// that test's own doc comment for the gap this closes and why draining
    /// between the two polls is required.
    #[test]
    fn a_write_still_blocked_on_its_second_poll_stages_a_park_again() {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind");
        let addr = listener.local_addr().expect("addr").to_string();
        let fd = connect_blocking_for_test(&addr);
        let (_server, _) = listener.accept().expect("accept");
        let expected_socket = with_fd(fd, |stream| raw_socket_of(stream)).expect("fd must be open");

        let chunk_bytes = vec![0xABu8; 1024 * 1024];
        fill_send_buffer_until_would_block_for_test(fd, &chunk_bytes);

        let chunk = crate::gc_bytes_for_test(&chunk_bytes);
        let fut = unsafe { nova_rt_net_write_future(fd, chunk) };
        let (poll, state) = poll_fn_and_state(fut);

        let first = with_test_current(0, || unsafe { poll(state, std::ptr::null_mut()) });
        assert_eq!(first, POLL_PENDING, "test setup: the first poll must park");
        assert_eq!(
            crate::task::staged_io_for_test(),
            Some((expected_socket, Interest::Write, None)),
            "test setup: the first poll must stage a park"
        );

        // Nothing drained the peer in between: the send buffer is still
        // full, simulating a wake with no room actually having opened up.
        drain_stray_pending_park_for_test();

        let second = with_test_current(0, || unsafe { poll(state, std::ptr::null_mut()) });
        assert_eq!(
            second, POLL_PENDING,
            "the send buffer is still full on the second poll -- must still \
             park, not complete"
        );
        assert_eq!(
            crate::task::staged_io_for_test(),
            Some((expected_socket, Interest::Write, None)),
            "a still-full write must stage a park again on its second poll, \
             not only its first -- a future that stages once and never \
             again leaves a spuriously-woken task parked forever with \
             nothing left to wake it"
        );

        drain_stray_pending_park_for_test();
        assert_eq!(unsafe { nova_rt_net_close(fd) }, OK);
    }

    #[test]
    fn a_ready_write_stashes_its_byte_count_via_slot_buffer() {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind");
        let addr = listener.local_addr().expect("addr").to_string();
        let fd = connect_blocking_for_test(&addr);
        let (_server, _) = listener.accept().expect("accept");

        let payload = crate::gc_bytes_for_test(b"hello");
        let fut = unsafe { nova_rt_net_write_future(fd, payload) };
        let (id, status) = spawn_and_pump_for_test(fut);
        assert_eq!(status, OK);
        let count = with_test_current(id, take_fd);
        assert_eq!(
            count, 5,
            "a freshly connected socket's send buffer is empty (per \
             `poll.rs`'s own `wait_reports_a_socket_ready_for_write`), so a \
             small write must report its whole length, not a short count"
        );

        assert_eq!(unsafe { nova_rt_net_close(fd) }, OK);
    }

    /// Using any of the three futures this task adds against a closed fd is
    /// an ordinary error, not a panic -- the async-future shape of
    /// `file.rs`'s `using_a_file_after_close_is_an_error_not_a_panic`. Checks
    /// all three individually, since a mutation could plausibly land in just
    /// one of their `with_fd` `None` arms without touching the other two.
    #[test]
    fn using_a_closed_socket_is_an_error_not_a_panic_for_read_write_and_read_timeout() {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind");
        let addr = listener.local_addr().expect("addr").to_string();
        let fd = connect_blocking_for_test(&addr);
        assert_eq!(unsafe { nova_rt_net_close(fd) }, OK);

        let read_status =
            unsafe { crate::task::nova_rt_task_block_on(nova_rt_net_read_future(fd, 16)) };
        assert_ne!(read_status, OK, "reading a closed socket must fail");

        let payload = crate::gc_bytes_for_test(b"x");
        let write_status =
            unsafe { crate::task::nova_rt_task_block_on(nova_rt_net_write_future(fd, payload)) };
        assert_ne!(write_status, OK, "writing a closed socket must fail");

        let read_timeout_status = unsafe {
            crate::task::nova_rt_task_block_on(nova_rt_net_read_timeout_future(fd, 16, 50))
        };
        assert_ne!(
            read_timeout_status, OK,
            "read_timeout on a closed socket must fail"
        );
    }

    /// A connected socket the far end never writes to: `read_timeout` must
    /// report `TIMED_OUT`, not hang forever and not misreport success.
    #[test]
    fn a_read_timeout_against_a_silent_peer_reports_timed_out() {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind");
        let addr = listener.local_addr().expect("addr").to_string();
        let fd = connect_blocking_for_test(&addr);
        // Kept alive but never written to.
        let (_server, _) = listener.accept().expect("accept");

        let fut = unsafe { nova_rt_net_read_timeout_future(fd, 64, 50) };
        let status = unsafe { crate::task::nova_rt_task_block_on(fut) };
        assert_eq!(status, TIMED_OUT);

        assert_eq!(unsafe { nova_rt_net_close(fd) }, OK);
    }

    /// The direct counterpart, for `read_timeout`, of the staged-park checks
    /// [`a_read_with_no_data_parks_and_completes_when_data_arrives`] and
    /// [`a_write_against_a_full_send_buffer_parks_rather_than_spinning`] make
    /// for plain `read`/`write`.
    ///
    /// **Why this needs its own test rather than trusting
    /// [`a_read_timeout_against_a_silent_peer_reports_timed_out`] above to
    /// cover it.** That test drives `read_timeout` through the real executor
    /// via `block_on` and only checks the *final* status. Measured: with
    /// `poll_read_timeout`'s own `stage_io_park` call deleted, that test
    /// still passes -- a future that returns `POLL_PENDING` with nothing
    /// staged is simply re-queued onto `QUEUE` and busy-repolled every turn,
    /// and this module's own deadline check is wall-clock based (via
    /// `now_epoch_ms`), so ~50ms of CPU-spinning reaches the same `TIMED_OUT`
    /// conclusion a genuine park-then-wake would. Only reading back what was
    /// actually staged, via a manual single poll before anything drains it,
    /// tells the two apart.
    ///
    /// Also the one place in this crate a deadline and an I/O wait
    /// legitimately co-stage (`Staged`'s own doc comment) -- worth pinning
    /// that the deadline is genuinely present, not just the socket/interest.
    #[test]
    fn a_read_timeout_with_no_data_stages_an_io_park_carrying_its_deadline() {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind");
        let addr = listener.local_addr().expect("addr").to_string();
        let fd = connect_blocking_for_test(&addr);
        // Kept alive but never written to.
        let (_server, _) = listener.accept().expect("accept");
        let expected_socket = with_fd(fd, |stream| raw_socket_of(stream)).expect("fd must be open");

        let fut = unsafe { nova_rt_net_read_timeout_future(fd, 64, 5_000) };
        let (poll, state) = poll_fn_and_state(fut);
        let status = with_test_current(0, || unsafe { poll(state, std::ptr::null_mut()) });
        assert_eq!(
            status, POLL_PENDING,
            "read_timeout with no data waiting must park rather than \
             complete synchronously"
        );

        match crate::task::staged_io_for_test() {
            Some((socket, interest, deadline)) => {
                assert_eq!(
                    socket, expected_socket,
                    "must park on this exact socket, not an unrelated one"
                );
                assert_eq!(interest, Interest::Read);
                assert!(
                    deadline.is_some(),
                    "read_timeout's park must carry its own deadline -- \
                     without it, a passed deadline could never wake this \
                     task independently of the socket becoming ready"
                );
            }
            None => panic!(
                "read_timeout with no data waiting must genuinely stage a \
                 park, not merely return POLL_PENDING while parking nothing"
            ),
        }

        drain_stray_pending_park_for_test();
        assert_eq!(unsafe { nova_rt_net_close(fd) }, OK);
    }

    /// The `read_timeout` counterpart of
    /// [`a_read_still_blocked_on_its_second_poll_stages_a_park_again`] -- see
    /// that test's own doc comment for the gap this closes and why draining
    /// between the two polls is required. A generous `ms` keeps the deadline
    /// nowhere near passing across the small, real time this test's own
    /// drain step takes.
    #[test]
    fn a_read_timeout_still_blocked_on_its_second_poll_stages_a_park_again() {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind");
        let addr = listener.local_addr().expect("addr").to_string();
        let fd = connect_blocking_for_test(&addr);
        // Kept alive but never written to.
        let (_server, _) = listener.accept().expect("accept");
        let expected_socket = with_fd(fd, |stream| raw_socket_of(stream)).expect("fd must be open");

        let fut = unsafe { nova_rt_net_read_timeout_future(fd, 64, 5_000) };
        let (poll, state) = poll_fn_and_state(fut);

        let first = with_test_current(0, || unsafe { poll(state, std::ptr::null_mut()) });
        assert_eq!(first, POLL_PENDING, "test setup: the first poll must park");
        match crate::task::staged_io_for_test() {
            Some((socket, interest, deadline)) => {
                assert_eq!(socket, expected_socket, "test setup");
                assert_eq!(interest, Interest::Read, "test setup");
                assert!(deadline.is_some(), "test setup");
            }
            None => panic!("test setup: the first poll must stage a park"),
        }

        // No data was written on the peer in between: simulate a wake with
        // nothing actually having arrived and the deadline nowhere near due.
        drain_stray_pending_park_for_test();

        let second = with_test_current(0, || unsafe { poll(state, std::ptr::null_mut()) });
        assert_eq!(
            second, POLL_PENDING,
            "still no data and the deadline is nowhere near passing -- must \
             still park, not complete"
        );
        match crate::task::staged_io_for_test() {
            Some((socket, interest, deadline)) => {
                assert_eq!(
                    socket, expected_socket,
                    "must park on this exact socket again"
                );
                assert_eq!(interest, Interest::Read);
                assert!(
                    deadline.is_some(),
                    "a still-blocked read_timeout must stage a park again \
                     on its second poll, carrying its deadline again -- a \
                     future that stages once and never again leaves a \
                     spuriously-woken task parked forever with nothing left \
                     to wake it"
                );
            }
            None => panic!(
                "a still-blocked read_timeout must stage a park again on \
                 its second poll, not only its first"
            ),
        }

        drain_stray_pending_park_for_test();
        assert_eq!(unsafe { nova_rt_net_close(fd) }, OK);
    }

    /// The gap the test above cannot see: it asserts a deadline is *present*
    /// on the second poll (`deadline.is_some()`), never that it is the
    /// *same* deadline. A `poll_read_timeout` that recomputes
    /// `now_epoch_ms() + ms` fresh on every poll -- instead of reusing the
    /// absolute deadline it committed to on its first -- would pass that
    /// test, and the entire rest of the suite, while never actually
    /// converging on a real deadline: a `read_timeout` against a silent peer
    /// would extend its own deadline forever and never fire.
    ///
    /// **A deliberate, real 30ms sleep separates the two polls.** A
    /// sub-millisecond gap (the natural cost of
    /// `drain_stray_pending_park_for_test` alone, with no sleep) would not
    /// reliably tell the two behaviours apart: both `now_epoch_ms()` calls
    /// could land in the same truncated-millisecond bucket regardless of
    /// which is present, making the bug invisible to a fast round trip.
    ///
    /// **Not exact equality.** `instant_from_remaining_ms` calls
    /// `Instant::now()` fresh on every poll, so even a correct
    /// implementation's staged `Instant` is never bit-identical across two
    /// calls separated in real time -- only the absolute target it
    /// approximates is meant to stay fixed. A drift well under the 30ms gap
    /// is that fixed target reasserting itself through ordinary
    /// millisecond-rounding; a drift approaching the full 30ms is the
    /// deadline sliding forward with the clock instead.
    #[test]
    fn a_read_timeout_still_blocked_on_its_second_poll_preserves_its_deadline() {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind");
        let addr = listener.local_addr().expect("addr").to_string();
        let fd = connect_blocking_for_test(&addr);
        // Kept alive but never written to.
        let (_server, _) = listener.accept().expect("accept");

        let fut = unsafe { nova_rt_net_read_timeout_future(fd, 64, 5_000) };
        let (poll, state) = poll_fn_and_state(fut);

        let first = with_test_current(0, || unsafe { poll(state, std::ptr::null_mut()) });
        assert_eq!(first, POLL_PENDING, "test setup: the first poll must park");
        let first_deadline = match crate::task::staged_io_for_test() {
            Some((_, _, Some(deadline))) => deadline,
            other => panic!(
                "test setup: the first poll must stage a park carrying a \
                 deadline, got {other:?}"
            ),
        };

        drain_stray_pending_park_for_test();
        std::thread::sleep(std::time::Duration::from_millis(30));

        let second = with_test_current(0, || unsafe { poll(state, std::ptr::null_mut()) });
        assert_eq!(
            second, POLL_PENDING,
            "still no data after the sleep -- must still park, not complete"
        );
        let second_deadline = match crate::task::staged_io_for_test() {
            Some((_, _, Some(deadline))) => deadline,
            other => panic!(
                "the second poll must stage a park carrying a deadline, got \
                 {other:?}"
            ),
        };

        let drift = if second_deadline >= first_deadline {
            second_deadline - first_deadline
        } else {
            first_deadline - second_deadline
        };
        assert!(
            drift < std::time::Duration::from_millis(10),
            "read_timeout's deadline must be preserved across polls, not \
             recomputed fresh each time -- expected the second poll's \
             staged deadline to stay within a few milliseconds of the first \
             (ordinary rounding jitter), got a drift of {drift:?} after a \
             deliberate 30ms gap between polls; a drift approaching the \
             full gap means the deadline is sliding forward with the clock \
             instead of staying fixed, which would let a read_timeout \
             against a silent peer extend its own deadline forever and \
             never actually fire"
        );

        drain_stray_pending_park_for_test();
        assert_eq!(unsafe { nova_rt_net_close(fd) }, OK);
    }

    /// The other half of `read_timeout`'s own branch: data that arrives well
    /// before the deadline completes the read normally, with the real bytes
    /// -- not every path through `poll_read_timeout` reports `TIMED_OUT`.
    #[test]
    fn a_read_timeout_completes_with_data_when_it_arrives_before_the_deadline() {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind");
        let addr = listener.local_addr().expect("addr").to_string();
        let fd = connect_blocking_for_test(&addr);
        let (mut server, _) = listener.accept().expect("accept");
        std::io::Write::write_all(&mut server, b"hi").expect("write");

        let fut = unsafe { nova_rt_net_read_timeout_future(fd, 64, 5_000) };
        let (id, status) = spawn_and_pump_for_test(fut);
        assert_eq!(status, OK);
        let bytes = with_test_current(id, take_bytes_for_test);
        assert_eq!(bytes, b"hi");

        assert_eq!(unsafe { nova_rt_net_close(fd) }, OK);
    }

    /// The exact layout `nova_rt_net_read_future` builds -- the same
    /// discipline `the_connect_futures_layout_is_the_one_the_abi_declares`
    /// documents and for the identical reason.
    ///
    /// Built but never polled, so this never touches a real socket -- the fd
    /// argument is a placeholder never looked up.
    #[test]
    fn the_read_futures_layout_is_the_one_the_abi_declares() {
        let fut = unsafe { nova_rt_net_read_future(999_999, 64) };
        assert_eq!(
            crate::gc::object_info(fut as usize),
            Some((crate::task::FUTURE_SIZE, true)),
            "the fat pointer must be exactly the two-word future, scanned"
        );
        let state = unsafe {
            (fut as *mut usize)
                .add(crate::task::FUTURE_SLOT_STATE)
                .read()
        };
        assert_eq!(
            crate::gc::object_info(state),
            Some((READ_STATE_SIZE, true)),
            "the state object must be the ABI minimum plus the two temp \
             slots holding fd and max, scanned"
        );
        let poll = unsafe {
            (fut as *mut usize)
                .add(crate::task::FUTURE_SLOT_POLL)
                .read()
        };
        let expected: PollFn = poll_read;
        assert_eq!(
            poll, expected as usize,
            "word 0 must be the poll function's address, not the state's"
        );
    }

    /// The exact layout `nova_rt_net_write_future` builds. Mirrors
    /// `the_read_futures_layout_is_the_one_the_abi_declares` exactly.
    ///
    /// Built but never polled, so this never touches a real socket -- the fd
    /// argument is a placeholder never looked up, and the payload is never
    /// sent.
    #[test]
    fn the_write_futures_layout_is_the_one_the_abi_declares() {
        let payload = crate::gc_bytes_for_test(b"unsent");
        let fut = unsafe { nova_rt_net_write_future(999_999, payload) };
        assert_eq!(
            crate::gc::object_info(fut as usize),
            Some((crate::task::FUTURE_SIZE, true)),
            "the fat pointer must be exactly the two-word future, scanned"
        );
        let state = unsafe {
            (fut as *mut usize)
                .add(crate::task::FUTURE_SLOT_STATE)
                .read()
        };
        assert_eq!(
            crate::gc::object_info(state),
            Some((WRITE_STATE_SIZE, true)),
            "the state object must be the ABI minimum plus the two temp \
             slots holding fd and the bytes pointer, scanned"
        );
        let poll = unsafe {
            (fut as *mut usize)
                .add(crate::task::FUTURE_SLOT_POLL)
                .read()
        };
        let expected: PollFn = poll_write;
        assert_eq!(
            poll, expected as usize,
            "word 0 must be the poll function's address, not the state's"
        );
    }

    /// The exact layout `nova_rt_net_read_timeout_future` builds. Mirrors the
    /// two above, with the one additional temp slot the ms-then-deadline
    /// reuse needs.
    ///
    /// Built but never polled, so this never touches a real socket -- the fd
    /// argument is a placeholder never looked up.
    #[test]
    fn the_read_timeout_futures_layout_is_the_one_the_abi_declares() {
        let fut = unsafe { nova_rt_net_read_timeout_future(999_999, 64, 50) };
        assert_eq!(
            crate::gc::object_info(fut as usize),
            Some((crate::task::FUTURE_SIZE, true)),
            "the fat pointer must be exactly the two-word future, scanned"
        );
        let state = unsafe {
            (fut as *mut usize)
                .add(crate::task::FUTURE_SLOT_STATE)
                .read()
        };
        assert_eq!(
            crate::gc::object_info(state),
            Some((RT_STATE_SIZE, true)),
            "the state object must be the ABI minimum plus the three temp \
             slots holding fd, max, and the reused ms-then-deadline slot, \
             scanned"
        );
        let poll = unsafe {
            (fut as *mut usize)
                .add(crate::task::FUTURE_SLOT_POLL)
                .read()
        };
        let expected: PollFn = poll_read_timeout;
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
