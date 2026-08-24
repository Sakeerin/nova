//! Open TCP handles, keyed by descriptor; the two-phase, non-blocking
//! `connect` that populates the table; the `read`, `write`, and
//! `read_timeout` futures that act on it; the `listen` that puts the table's
//! other kind of entry -- a listening socket -- into it; the `local_port`
//! that reads back which port the kernel gave one; and the `accept` that
//! turns an incoming connection into an ordinary stream entry.
//!
//! # Two kinds of handle, one table
//!
//! An entry is a [`Sock`]: a connected `TcpStream` or a listening
//! `TcpListener`. One table rather than two, so there is one `NEXT_FD` space,
//! one closedness invariant, and one `close` -- see [`Sock`]'s own doc comment
//! for why that is the better trade, and [`remove_socket`]'s for why it is
//! what lets `close` serve both kinds with no second intrinsic. The cost is
//! that every accessor has to say which kind it wants ([`with_stream`],
//! [`with_listener`]), and a caller asking for the wrong one gets the same
//! `None` an absent fd gives.
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
//! no Unix or macOS toolchain was reachable from any environment this
//! increment was implemented or reviewed in, the identical status `poll.rs`'s
//! own Unix arm recorded. **CI has since measured the IPv4 half of it on both
//! platforms**: every test and fixture in this project connects to a
//! `127.0.0.1` loopback address, so `Test (ubuntu-latest)` and
//! `Test (macos-latest)` drive the `SocketAddr::V4` path for real. That
//! answers the BSD-layout question for `sockaddr_in`: these layouts carry a
//! leading `sin_len` this module's socket-address construction does not set
//! explicitly -- it zeroes the whole struct and assigns only the fields every
//! Unix has, so the length byte stays zero -- and a Darwin `connect` demonstrably
//! accepts that, rather than the claim resting on documentation alone.
//!
//! **The `SocketAddr::V6` path is measured too now, on all three platforms.**
//! It was not for this module's whole life before that: nothing in this project
//! passed an IPv6 address to `connect`, so neither `sockaddr_in6` arm -- Unix
//! or Windows -- had ever run, and `sin6_len`-left-zero carried exactly the
//! documentation-only status `sin_len` used to.
//! `an_ipv6_connect_parks_on_its_first_poll_and_then_establishes` closed that:
//! it binds a `::1` loopback listener and dials it through the real
//! `nova_rt_net_connect_future`, so both arms execute on every leg of CI, and
//! a Darwin `connect` demonstrably accepts a `sockaddr_in6` whose leading
//! `sin6_len` this module leaves zero -- the same claim the V4 path's
//! `sin_len` had to wait for CI to settle, settled the same way.
//!
//! Two fields inside that arm remain unmeasured, and cannot reasonably be
//! otherwise: `sin6_flowinfo` and `sin6_scope_id` are both zero for `::1`, and
//! `std`'s own `SocketAddr` parser accepts no textual syntax for either, so no
//! address string that can reach [`resolve_addr`] from `std/net`'s
//! `connect(addr: String)` produces a non-zero value for them at all. That
//! bounds the exposure rather than leaving it open: the two assignments are
//! unreachable as anything but zero from the only surface a Nova program has.
//! See that test's own doc comment for the rest of what it does and does not
//! pin down.
//!
//! The Windows arm's IPv4 path was built and exercised for real against real
//! loopback sockets on this task's own Windows host, the same way `poll.rs`'s
//! Windows arm was, and `Test (windows-latest)` re-runs it on every push.

use crate::fs::{fail, stash, Slot, OK};
use crate::poll::{set_nonblocking, Interest, RawSocket};
use crate::task::{
    build_future, stage_io_park, PollFn, POLL_PENDING, POLL_READY, STATE_MIN_SIZE,
    STATE_SLOT_OUTPUT, STATE_SLOT_TAG, STATE_SLOT_TEMPS,
};
use crate::NovaStr;
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::net::{SocketAddr, TcpListener, TcpStream, ToSocketAddrs};
use std::sync::OnceLock;
use std::time::{Duration, Instant};

/// A registered handle: either a connected stream or a listening socket.
///
/// **One table rather than two, and the whole payoff is in what stays
/// single**: one `NEXT_FD` space, so no fd can ever name two different
/// handles; one closedness invariant, so "absence from the table is
/// closedness" holds for both kinds at once rather than being restated per
/// table; and one `close` -- [`remove_socket`] removes an entry by key
/// regardless of which variant it holds, so [`nova_rt_net_close`] serves a
/// listener unchanged and this increment adds no fourth intrinsic.
///
/// **A kind mismatch is *not* separately reportable, and that is a deliberate
/// consequence rather than a second payoff.** Every accessor has to say which
/// kind it wants ([`with_stream`], [`with_listener`]), and asking for the
/// wrong one collapses into the same `None` an absent fd gives -- so a read
/// against a listener fd produces exactly what a closed, stale or forged fd
/// produces: [`closed_fd_error`]'s status `OTHER`, with the message "socket is
/// not open". Bit for bit the same answer, at the Nova boundary and here. The
/// collapse is not an oversight: `fs.rs`'s `IoErrorKind` table has no
/// wrong-kind variant, and adding one would be a wire-contract change across
/// `fs.rs` and `std/io` together, out of scope for this increment. A second
/// table would not have bought a distinction either -- it would have made the
/// wrong-kind lookup miss for the identical reason. See [`closed_fd_error`]
/// for the same statement where the status is actually produced, and
/// `reading_a_listener_fd_is_an_error_not_a_stream_read` for the test that
/// pins it.
enum Sock {
    Stream(TcpStream),
    Listener(TcpListener),
}

thread_local! {
    /// Open TCP handles by descriptor -- connected streams and listening
    /// sockets in one table, per [`Sock`]. `thread_local!` for the reason
    /// `task.rs`'s module doc gives for `TASKS` and `file.rs`'s module doc
    /// gives for `FILES`: the GC's roots are per-thread, so a second thread
    /// running Nova code would free objects the first holds.
    static SOCKETS: RefCell<HashMap<i64, Sock>> = RefCell::new(HashMap::new());
    /// Never reused, so a stale fd stays stale rather than aliasing a
    /// different connection later. Starts at 1 so 0 is available as an
    /// obviously invalid value in diagnostics.
    static NEXT_FD: Cell<i64> = const { Cell::new(1) };
}

/// Run `f` against the stream behind `fd`. `None` covers the cases every
/// caller reports identically: absent from the table (closed, stale or
/// forged) and present under the wrong kind.
///
/// `try_borrow_mut` rather than `borrow_mut`, for the identical reason
/// `file.rs`'s `with_fd` gives: a `RefCell` failure here would cross a
/// generated poll boundary. The `None` arm is an ordinary error, not an
/// abort.
fn with_stream<R>(fd: i64, f: impl FnOnce(&mut TcpStream) -> R) -> Option<R> {
    SOCKETS.with(|sockets| {
        let Ok(mut sockets) = sockets.try_borrow_mut() else {
            crate::task::abort_with("nova_rt_net: handle table is already borrowed")
        };
        match sockets.get_mut(&fd) {
            Some(Sock::Stream(s)) => Some(f(s)),
            _ => None,
        }
    })
}

/// Run `f` against the listener behind `fd`. Same collapse of absent-and
/// wrong-kind into one `None` as [`with_stream`].
///
/// [`nova_rt_net_local_port`] and [`try_accept`] are its production callers.
/// It carried `#[allow(dead_code)]` for exactly as long as this increment's
/// Task 1 was the whole of it -- nothing in production read a listener then,
/// and the only caller was `is_listener_for_test` below -- and the attribute
/// left with the arrival of the first real caller rather than being kept on to
/// mask a genuinely dead accessor later. That is the same lifecycle `poll.rs`'s
/// `Interest` records for its own former one.
fn with_listener<R>(fd: i64, f: impl FnOnce(&mut TcpListener) -> R) -> Option<R> {
    SOCKETS.with(|sockets| {
        let Ok(mut sockets) = sockets.try_borrow_mut() else {
            crate::task::abort_with("nova_rt_net: handle table is already borrowed")
        };
        match sockets.get_mut(&fd) {
            Some(Sock::Listener(l)) => Some(f(l)),
            _ => None,
        }
    })
}

/// Allocate a fresh, never-reused fd for `sock` and insert it into the
/// table. Mirrors `file.rs`'s `register_new_file` exactly, including the
/// fallible-borrow reasoning: a `RefCell` failure here would cross a generated
/// poll boundary, and there is no missing-key case to report since insertion
/// always creates the entry.
///
/// One function taking a [`Sock`] rather than a sibling per variant, so the
/// single `NEXT_FD` space is visibly single: a second registrar would be a
/// second place to get the never-reuse rule right.
fn register_new_socket(sock: Sock) -> i64 {
    let fd = NEXT_FD.with(|next| {
        let fd = next.get();
        next.set(fd + 1);
        fd
    });
    SOCKETS.with(|sockets| {
        let Ok(mut sockets) = sockets.try_borrow_mut() else {
            crate::task::abort_with("nova_rt_net: handle table is already borrowed")
        };
        sockets.insert(fd, sock);
    });
    fd
}

/// Remove `fd` from the table, dropping the underlying [`Sock`] and closing
/// its OS handle. Silent (not an error) if `fd` is already absent -- this is
/// the shared body [`nova_rt_net_close`] exposes directly and
/// [`finish_connect`] uses to release a socket whose connection was refused.
///
/// **Removes by key regardless of which variant the entry holds**, which is
/// the whole reason [`nova_rt_net_close`] serves a listener as well as a
/// stream and this module needs no separate close-a-listener intrinsic. The
/// kind-checked accessors exist to stop a *read* from treating a listener as
/// a stream; releasing a handle needs no such distinction, since dropping
/// either closes exactly one OS handle.
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
/// imports from `fs`.
///
/// **Every payload this module encodes as an *integer* goes through it**:
/// `connect`'s new fd ([`finish_connect`]), `write`'s reported byte count
/// ([`try_write`]), `listen`'s new listener fd ([`nova_rt_net_listen`]), a
/// listener's bound port ([`nova_rt_net_local_port`]) and an accepted
/// connection's new fd ([`try_accept`]).
///
/// **That is not every `Slot::Buffer` payload, and the exception is this
/// module's most-used one.** [`try_read`] writes the slot directly, with the
/// raw bytes it read: those are already a byte payload, so there is no integer
/// to encode and nothing for this function to do. A reader must therefore not
/// conclude that everything reaching `Slot::Buffer` from this module is eight
/// little-endian bytes -- on the read path it is the payload itself, of
/// whatever length the read returned.
///
/// Deliberately a roster with no quantifier over call sites. The wording
/// before this one said "two different payloads", which the very next function
/// added to this file falsified; the wording after that said "every payload
/// this module puts in `Slot::Buffer`", which was not merely stale but false,
/// for the [`try_read`] reason above. **Replacing a count with a universal
/// trades a claim that goes stale for one that is wrong** -- so this names its
/// members and quantifies only over the thing it actually covers. `file.rs`'s
/// own copy carries the same multiple use, for `open`'s fd and `write`'s
/// count.
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
///
/// **The wrong-kind case lands here too**, since [`Sock`] arrived: reading a
/// listener fd, or asking a stream fd for a listener-only property, takes
/// this same path rather than a distinct status. That collapse is deliberate,
/// not an omission -- `fs.rs`'s `IoErrorKind` table has no wrong-kind variant,
/// and adding one is a wire-contract change across `fs.rs` and `std/io`
/// together. A Nova program can still tell the two apart only by the
/// `message`, which this fabricates as "socket is not open" for both.
fn closed_fd_error() -> i64 {
    fail(&std::io::Error::other("socket is not open"))
}

/// Close `fd`, dropping the underlying handle -- a connection or a listener
/// alike -- and releasing its OS handle. Idempotent, for the identical reason
/// `file.rs`'s `nova_rt_file_close` is: `std/net`'s `close` cannot consume its
/// receiver, because Nova has no move checking, so a caller can always reach a
/// second call, and it must find nothing and still succeed.
///
/// **Serves both [`Sock`] variants, which is why this increment's listener
/// needs no second close intrinsic**: see [`remove_socket`]'s own doc comment
/// for why removing by key regardless of variant is right rather than merely
/// convenient.
///
/// # Safety
/// No pointer argument, so no dereference precondition; marked `unsafe
/// extern "C"` for uniformity with this crate's other JIT-registered symbols.
#[no_mangle]
pub unsafe extern "C" fn nova_rt_net_close(fd: i64) -> i64 {
    remove_socket(fd);
    OK
}

/// Bind and listen on `addr` ("host:port"), registering a non-blocking
/// listener in the same table connected streams live in.
///
/// **Non-suspending, so this is an ordinary status word like
/// [`nova_rt_net_close`] rather than a future constructor and gets no poll
/// function.** The `bind` and `listen` *syscalls* are immediate kernel
/// bookkeeping -- neither waits on a peer -- and the waiting a server does
/// happens in [`nova_rt_net_accept_future`], which is a separate intrinsic and
/// *is* a future constructor. On success the new fd
/// is stashed via `Slot::Buffer` exactly as `connect`'s is, so the Nova side
/// decodes it with the same `decode_count`.
///
/// **Name resolution is the one path that can still block this thread, and the
/// sentence above deliberately does not cover it.** `addr` reaches
/// `TcpListener::bind` as a `&str`, so `std` resolves it through
/// `ToSocketAddrs`: a numeric address parses in userspace and never blocks,
/// but `std/net`'s `bind(addr: String)` is reachable from user Nova code with
/// a hostname, and that lookup blocks with no future to park on. This is
/// exactly the caveat [`resolve_addr`] already records for `connect`, with the
/// same status -- out of scope for this increment rather than handled -- so it
/// is not restated in full here. "Cannot suspend" is therefore a claim about
/// the syscalls, not about every string a caller can pass.
///
/// **Resolution also differs from this module's `connect` in a second way,
/// written down nowhere until now:** `connect` goes through [`resolve_addr`],
/// which takes only the *first* address a multi-address name yields; this
/// hands the whole string to `std`, whose `TcpListener::bind` attempts each
/// resolved address in turn and reports a failure only once every one of them
/// has failed. So a multi-address name gets a try-each policy here and a
/// first-only policy there. Neither is wrong, but they are not the same
/// policy, and a reader assuming the two intrinsics resolve identically would
/// be wrong.
///
/// `set_nonblocking(true)` is load-bearing rather than hygiene: an `accept`
/// against a blocking listener would block this whole executor thread instead
/// of parking the one task, defeating the point of the poller for every
/// sibling task on it.
///
/// **Now measured, and the answer is a hang rather than a failure.** Deleting
/// that `set_nonblocking` call used to fail nothing at all: before
/// [`nova_rt_net_accept_future`] existed no test reached an `accept`, so the
/// whole suite stayed green with a blocking listener sitting in the table.
/// With `accept` in place the deletion was run again, and
/// `accept_parks_until_a_client_connects_then_yields_a_stream_fd` **hangs**:
/// [`try_accept`]'s `accept` call blocks this thread inside the very first
/// poll instead of reporting would-block, so the future never returns
/// `POLL_PENDING`, the executor is blocked rather than the task parked, and
/// the test binary never exits. Every other test in the workspace still
/// passes, so that one test is the whole of the coverage -- counted, not
/// assumed, across all 44 targets.
///
/// **A hang is weaker than an assertion and is recorded as such.** Under
/// `cargo test` it surfaces as a CI timeout with no message naming this line,
/// which is a build failure but a poor diagnostic. Making it an assertion
/// instead would need a portable "is this handle non-blocking" query, and
/// there is none: Windows offers no way to read a socket's blocking mode back,
/// so the only observation available is that an operation which should have
/// returned immediately did not. The accepted stream's own `set_nonblocking`
/// in [`try_accept`] is pinned the same way and for the same reason.
///
/// Uses `std::net::TcpListener::bind` directly, unlike `connect`, which had to
/// issue its own syscall (see this module's own header): there is no
/// two-phase problem here to work around, because binding never has to be
/// started now and settled later.
///
/// # Safety
/// `addr` must point to a live `NovaStr`.
#[no_mangle]
pub unsafe extern "C" fn nova_rt_net_listen(addr: *const NovaStr) -> i64 {
    // SAFETY: `addr` points to a live `NovaStr` by this function's own
    // precondition, and the borrow does not outlive this call.
    let addr = unsafe { crate::as_str(addr) };
    let listener = match TcpListener::bind(addr) {
        Ok(l) => l,
        Err(e) => return fail(&e),
    };
    if let Err(e) = listener.set_nonblocking(true) {
        return fail(&e);
    }
    let fd = register_new_socket(Sock::Listener(listener));
    stash_i64(fd);
    OK
}

/// Report the port `fd`'s listener is bound to, which is how a caller learns
/// the kernel's choice after binding port 0.
///
/// **Non-suspending, so this is an ordinary status word like
/// [`nova_rt_net_close`] and [`nova_rt_net_listen`] rather than a future
/// constructor, and it gets no poll function.** `TcpListener::local_addr` is a
/// `getsockname` call over bookkeeping the kernel already holds: nothing waits
/// on a peer, and -- unlike [`nova_rt_net_listen`] -- there is no address
/// argument at all, so none of that function's name-resolution caveat applies
/// here. This one really cannot block, on any argument.
///
/// **The port travels in `Slot::Buffer` rather than in the return value,
/// because the return value is already spoken for: it *is* the error kind.** A
/// port there would make `0` ambiguous -- success, or a port of zero -- and
/// every non-zero port indistinguishable from a failure status. So the answer
/// rides the same 8-byte little-endian channel [`nova_rt_net_listen`]'s fd
/// rides, decoded on the Nova side by the same `decode_count`.
///
/// **A stream fd is an error here, not a port, and the kind check is what
/// refuses it rather than the syscall.** A connected socket genuinely has a
/// local port, so `local_addr` would have answered; [`with_listener`] never
/// reaches it, and the miss collapses into [`closed_fd_error`] exactly as an
/// absent, closed, stale or forged fd does. See [`Sock`]'s own doc comment for
/// why that collapse is deliberate rather than an omission, and
/// `net_local_port_on_a_stream_fd_is_an_error` for the test that pins it.
///
/// `i64::from(addr.port())` rather than a cast: `port()` is a `u16`, so the
/// widening is infallible and needs no `as`.
///
/// # Safety
/// No pointer argument, so no dereference precondition; marked `unsafe
/// extern "C"` for uniformity with this crate's other JIT-registered symbols.
#[no_mangle]
pub unsafe extern "C" fn nova_rt_net_local_port(fd: i64) -> i64 {
    match with_listener(fd, |l| l.local_addr()) {
        Some(Ok(addr)) => {
            stash_i64(i64::from(addr.port()));
            OK
        }
        Some(Err(e)) => fail(&e),
        None => closed_fd_error(),
    }
}

/// Resolve `addr` ("host:port") to one socket address.
///
/// Uses `std`'s own `ToSocketAddrs` -- the identical resolution
/// `std::net::TcpStream::connect` performs -- and takes the first result.
///
/// **That first-only choice is *not* what `std` does, and the sentence here
/// used to claim it was.** `std::net::TcpStream::connect` attempts each
/// resolved address in turn and reports a failure only once every one of them
/// has failed; this attempts exactly one. The behaviour is deliberate and
/// unchanged -- the two-phase, non-blocking `connect` this module builds has
/// no place to hold "the addresses not tried yet" across a suspension, and
/// [`nova_rt_net_listen`]'s own doc comment already records that the two
/// intrinsics therefore resolve by different policies -- but the *reason*
/// given was false, so it is gone. A multi-address name that resolves to a
/// dead address first fails here where `std` would have moved on.
///
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

/// [`raw_socket_of`]'s sibling across [`Sock`], for a listening socket.
///
/// A separate function rather than one generic over `AsRawFd`/`AsRawSocket`:
/// the trait that supplies the handle is itself platform-specific, so a
/// generic version would still need both `#[cfg]` arms and would additionally
/// have to name a different bound in each -- two arms either way, with a type
/// parameter added for nothing. `TcpListener` and `TcpStream` are the only two
/// types this module ever asks for a handle, and neither is generic.
#[cfg(unix)]
fn raw_socket_of_listener(listener: &TcpListener) -> RawSocket {
    use std::os::unix::io::AsRawFd;
    RawSocket(i64::from(listener.as_raw_fd()))
}

#[cfg(windows)]
fn raw_socket_of_listener(listener: &TcpListener) -> RawSocket {
    use std::os::windows::io::AsRawSocket;
    RawSocket(listener.as_raw_socket() as i64)
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
    use std::os::unix::io::FromRawFd;

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
            // Zero the whole struct, then assign field by field -- not a
            // struct literal, because `sockaddr_in`'s *field set* differs
            // across Unix. BSD and Darwin carry a leading `sin_len` that Linux
            // does not have at all, so a literal naming only the portable
            // fields needs a `..zeroed()` base on Darwin and has nothing left
            // to take from one on Linux, where `clippy::needless_update`
            // rejects it as an error under this project's `-D warnings`. This
            // form names no field that does not exist on every platform, and
            // leaves every field it does not name -- `sin_len` included, where
            // it exists, and `sin_zero` everywhere -- zeroed on all of them,
            // exactly what the struct-update base achieved.
            //
            // SAFETY: `sockaddr_in` is a plain C struct of integers and byte
            // arrays with no niche or validity invariant, so all-zero is a
            // valid value for it on every platform this arm compiles for.
            let mut sin: libc::sockaddr_in = unsafe { std::mem::zeroed() };
            sin.sin_family = libc::AF_INET as libc::sa_family_t;
            sin.sin_port = a.port().to_be();
            sin.sin_addr = libc::in_addr {
                s_addr: u32::from(*a.ip()).to_be(),
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
            // Zeroed struct then per-field assignment, for the identical
            // reason the `V4` arm above documents at length: BSD and Darwin
            // carry a leading `sin6_len` that Linux does not have.
            //
            // SAFETY: as for `sockaddr_in` above -- a plain C struct with no
            // validity invariant, for which all-zero is a valid value.
            let mut sin6: libc::sockaddr_in6 = unsafe { std::mem::zeroed() };
            sin6.sin6_family = libc::AF_INET6 as libc::sa_family_t;
            sin6.sin6_port = a.port().to_be();
            sin6.sin6_flowinfo = a.flowinfo();
            sin6.sin6_addr = libc::in6_addr {
                s6_addr: a.ip().octets(),
            };
            sin6.sin6_scope_id = a.scope_id();
            // SAFETY: same as the `V4` arm's `connect` above: `fd` is the
            // socket just created, and `sin6` is a live, correctly sized
            // `sockaddr_in6` for the duration of this call.
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
/// `STATE_MIN_SIZE` -- the same arrangement `SLEEP_SLOT_DEADLINE_NANOS` uses.
///
/// Reused for two different values across the future's short life: before
/// the first poll it holds the address argument's `NovaStr` pointer (kept
/// alive by this state object's own GC root/scan, exactly as
/// `SLEEP_SLOT_DEADLINE_NANOS` needs no rooting of its own for a plain
/// `Int`); the first poll reads that out and overwrites this same slot with
/// the socket's table fd for the second poll to look back up.
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
            let table_fd = register_new_socket(Sock::Stream(stream));
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
    match with_stream(table_fd, |stream| stream.take_error()) {
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
/// `with_stream`/`register_new_socket`/`remove_socket`, and every I/O call --
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
/// reason `nova_rt_task_sleep_future_nanos`'s doc comment gives: the whole value
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
    let outcome = with_stream(fd, |stream| match std::io::Read::read(stream, &mut buf) {
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
    let outcome = with_stream(fd, |stream| match std::io::Write::write(stream, bytes) {
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
/// fallible step -- the borrow inside [`try_read`]/`with_stream`, and the I/O call
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
/// **A separate origin from `crate::time::epoch`, on purpose.** Only relative
/// elapsed time is ever compared against this one, so an arbitrary origin is
/// fine -- but what this module needs from a clock reading is a different job
/// from what `crate::time::epoch` is for: encoding a deadline as a plain,
/// scannable `i64` a state slot can hold, not handing back a
/// `std::time::Instant` itself. A `std::time::Instant` has no documented byte
/// layout this module could safely write into one of its own state slots
/// directly the way it writes a plain fd or count there -- so `read_timeout`
/// stores milliseconds-since-this-epoch instead, the same spirit as
/// `CONNECT_SLOT_SOCK` storing a plain fd rather than a `TcpStream`.
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
/// value to "now": nothing stops a remaining duration, recomputed from a
/// stored deadline on every poll, from having already reached zero or gone
/// negative by the time this runs.
///
/// Not the same shape as `task.rs`'s own clamp anymore. Its `deadline_from_
/// nanos` used to recompute a remaining duration from "now" the identical
/// way this function does; the `timeout-combinator` increment eliminated
/// that shape entirely -- `task.rs` now computes a sleep's or a timeout's
/// deadline exactly once, at construction, via `deadline_nanos_from_now`,
/// whose `nanos.max(0)` clamp guards a fresh duration *argument* rather than
/// a value re-derived at every poll, and nothing time-relative is recomputed
/// on that side again. `read_timeout` still recomputes here on every poll,
/// because nothing in that increment touched `net.rs`.
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
/// construction matches `task.rs`'s own `nova_rt_task_sleep_future_nanos`: a
/// future built but not immediately polled should time out `ms` after it
/// *starts running*, not after it was merely constructed.
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
/// plain `write` and `accept` each stage `None`.
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

// ---------------------------------------------------------------------------
// `accept` -- the listener operation that waits on a peer.
//
// Parks on `Interest::Read` through the same `stage_io_park` seam `read` uses,
// and needs no resume tag for `poll_read`'s own reason: re-attempting the
// accept is the same operation on every poll, so repeating it is correct
// whether this is the first poll or the fifth, and trying it before ever
// parking means a connection already sitting in the backlog completes on the
// very first poll with no park staged at all.
//
// **Accept-readiness is read-readiness, so `poll.rs` needs no new
// `Interest`.** `select` reports a listening socket with a pending connection
// in its read set, and `WSAPoll` reports it as `POLLRDNORM` -- the same
// signals a readable stream produces, which is why a listener can share the
// poller's existing `Interest::Read` rather than needing a third variant to be
// threaded through `task.rs`'s `Wait` and both of `poll.rs`'s platform arms.
// ---------------------------------------------------------------------------

/// What one non-blocking `TcpListener::accept` attempt produced. Mirrors
/// [`ReadStep`] and [`WriteStep`] exactly, against a listener rather than a
/// stream.
enum AcceptStep {
    /// The attempt settled: a connection was accepted (registered as a fresh
    /// [`Sock::Stream`] with its new fd stashed via [`stash_i64`]) or a real
    /// I/O error ([`fail`]'s status) or `fd` was not a live listener
    /// ([`closed_fd_error`]). Either way there is nothing left to do but
    /// report this status.
    Done(i64),
    /// The listener is open but no connection is pending right now. Carries
    /// the raw socket a caller should park on for `Interest::Read`.
    WouldBlock(RawSocket),
}

/// One non-blocking `TcpListener::accept` attempt against `fd`, registering
/// the accepted connection and stashing its new fd via [`stash_i64`] on
/// success.
///
/// **The accepted stream is registered *outside* [`with_listener`]'s closure,
/// and that is a correctness requirement rather than a style choice.**
/// [`register_new_socket`] takes the very `SOCKETS` borrow [`with_listener`]
/// is still holding, and a nested fallible re-borrow fails rather than
/// waiting -- which this module's accessors turn into an `abort_with`, ending
/// the process. So the closure hands the accepted `TcpStream` straight back
/// out and everything that touches the table again runs after the borrow is
/// released. [`try_read`] can stash from inside its own closure because
/// `fs.rs`'s slot table is a different `RefCell`; this cannot, because the
/// table is the same one.
///
/// **The listener's raw socket is taken on every attempt, including the ones
/// that succeed and never use it.** It is one field read off a handle already
/// in hand, and taking it here rather than in a second [`with_listener`] call
/// on the would-block path keeps this to exactly one table lookup per poll --
/// and removes a second `None` arm that could only ever mean "the table
/// changed between two lookups inside one poll", a case nothing can produce
/// and no reader could act on.
///
/// **The accepted stream is set non-blocking explicitly, not left to
/// inherit.** `accept` propagates the listening socket's non-blocking mode on
/// some platforms and not others -- Linux's `accept(2)` documents that the new
/// socket does *not* inherit `O_NONBLOCK` -- so without this a Nova server's
/// first `read` on an accepted connection would block the whole executor
/// thread on one of this project's three CI platforms and park correctly on
/// the other two. The listener's own `set_nonblocking` in
/// [`nova_rt_net_listen`] governs the `accept` call; this one governs every
/// operation on what it returns.
fn try_accept(fd: i64) -> AcceptStep {
    let attempt = with_listener(fd, |listener| {
        (raw_socket_of_listener(listener), listener.accept())
    });
    match attempt {
        Some((_, Ok((stream, _peer)))) => {
            if let Err(e) = stream.set_nonblocking(true) {
                return AcceptStep::Done(fail(&e));
            }
            let accepted = register_new_socket(Sock::Stream(stream));
            stash_i64(accepted);
            AcceptStep::Done(OK)
        }
        Some((socket, Err(e))) if e.kind() == std::io::ErrorKind::WouldBlock => {
            AcceptStep::WouldBlock(socket)
        }
        Some((_, Err(e))) => AcceptStep::Done(fail(&e)),
        None => AcceptStep::Done(closed_fd_error()),
    }
}

/// Where an `accept` future keeps its listener fd between polls. One slot and
/// no resume tag, unlike `connect`'s and `read_timeout`'s state: there is
/// nothing to carry across a suspension but the fd, and nothing to compute
/// once.
const ACCEPT_SLOT_FD: usize = STATE_SLOT_TEMPS;

/// State size for an `accept` future: the ABI minimum plus the one temp slot
/// holding the listener fd.
const ACCEPT_STATE_SIZE: usize = STATE_MIN_SIZE + 8;

const _: () = assert!(ACCEPT_STATE_SIZE >= (ACCEPT_SLOT_FD + 1) * 8);

/// The accept future's poll function: one non-blocking `accept` attempt,
/// parking on the listener's read-readiness when no connection is pending.
///
/// **Level-triggered and tag-free, which is mandatory here rather than
/// stylistic.** The poll ABI is frozen and `task_ctx` is always null (see
/// `task.rs`'s own `PollFn` doc comment), and `task.rs`'s wake paths just move
/// a parked task back onto the ready queue without recording *why* -- so this
/// never asks what woke it and instead re-derives its whole answer from the
/// listener on every call, exactly as [`poll_read`] and [`poll_read_timeout`]
/// re-derive theirs. Re-attempting the accept is also what makes a spurious
/// wake harmless: it finds `WouldBlock` again, stages the same wait again, and
/// nothing is left permanently parked with no one to wake it.
///
/// Must not unwind, as every other `PollFn` in this crate must not: every
/// fallible step -- the fallible table borrows inside [`try_accept`],
/// `set_nonblocking` on the accepted stream, and the `accept` call itself --
/// already returns a value [`try_accept`] maps into a status rather than
/// unwrapping it.
unsafe extern "C-unwind" fn poll_accept(state: *mut u8, _task_ctx: *mut u8) -> i64 {
    let slots = state as *mut i64;
    // SAFETY: `state` is the object `nova_rt_net_accept_future` built, at
    // least `ACCEPT_STATE_SIZE` bytes, so the slot below is in bounds.
    let fd = unsafe { slots.add(ACCEPT_SLOT_FD).read() };
    match try_accept(fd) {
        AcceptStep::Done(status) => {
            // SAFETY: same object, output slot.
            unsafe { slots.add(STATE_SLOT_OUTPUT).write(status) };
            POLL_READY
        }
        AcceptStep::WouldBlock(socket) => {
            stage_io_park(socket, Interest::Read, None);
            POLL_PENDING
        }
    }
}

/// A fresh `Future<Int>` (a status, per this module's boundary design) that
/// waits for the next incoming connection on the listener behind `fd`. On
/// success the accepted connection's new fd is stashed via `Slot::Buffer`,
/// the identical 8-byte little-endian channel [`nova_rt_net_listen`]'s own fd
/// rides -- so the Nova side decodes it with the same `decode_count`.
///
/// **Waiting on a peer is what makes this a future**, and it is the only
/// thing that does: [`nova_rt_net_listen`] and [`nova_rt_net_local_port`] are
/// ordinary status words because their syscalls cannot wait on one. Stated as
/// a property rather than as a position or a count, deliberately -- every
/// numbered claim about this module's future/status split written so far has
/// gone stale within an increment.
///
/// **The state object is fresh on every call**, for the same reason
/// [`nova_rt_net_connect_future`]'s own doc comment gives: the whole value
/// carried across a suspension is this state object's own listener fd, so two
/// accepts in flight at once would otherwise corrupt each other. Two accepts
/// in flight in the *same task* still cannot happen, for a different reason --
/// staging two I/O waits in one poll ends the process (`task.rs`'s
/// `stage_park`), so a server needs one task per concurrent wait.
///
/// # Safety
/// No pointer argument, so no dereference precondition beyond `build_future`'s
/// own; marked `unsafe extern "C-unwind"` for uniformity with this module's
/// other future constructors.
#[no_mangle]
pub unsafe extern "C-unwind" fn nova_rt_net_accept_future(fd: i64) -> *mut u8 {
    let poll: PollFn = poll_accept;
    build_future(poll, ACCEPT_STATE_SIZE, |slots| {
        // SAFETY: `slots` addresses a live `ACCEPT_STATE_SIZE` block, and the
        // slot is in bounds by the assertion above.
        unsafe { slots.add(ACCEPT_SLOT_FD).write(fd) };
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fs::{CONNECTION_REFUSED, OTHER, TIMED_OUT};

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
    ///
    /// **Named for the payload it was written against, not the only one it
    /// reads.** [`stash_i64`]'s encoding is the same whatever the number means,
    /// so this also decodes a write's byte count and, since
    /// [`nova_rt_net_local_port`], a listener's bound port. The name is kept to
    /// stay aligned with `file.rs`'s own helper rather than renamed for each
    /// new payload.
    fn take_fd() -> i64 {
        let ptr = crate::fs::take_for_test(Slot::Buffer) as *const NovaStr;
        // Checked before the dereference, not after: an empty slot hands back
        // a null pointer, and reading a `NovaStr` through it is a crash rather
        // than any assertion failure a reader could act on.
        // [`poll_write_until_it_parks_for_test`] is the caller that can reach
        // this, if a would-block arm ever reports a bare success without a
        // count behind it.
        assert!(
            !ptr.is_null(),
            "nothing at all was pending in Slot::Buffer -- whatever was \
             expected to stash a count did not"
        );
        // SAFETY: test-only, and non-null by the check above. Whatever last
        // stashed into `Slot::Buffer` did so earlier in this same test, with
        // nothing allocated since, so the payload has not been swept.
        let bytes = unsafe { crate::bytes::as_bytes(ptr) };
        let Ok(arr) = <[u8; 8]>::try_from(bytes) else {
            panic!(
                "stash_i64 stashes an 8-byte payload; got {} bytes instead -- \
                 the encoding changed without updating this test helper to \
                 match (an empty slot is caught above, not here)",
                bytes.len()
            );
        };
        i64::from_le_bytes(arr)
    }

    /// Test-only: run `f` with `CURRENT` set to `task_id`, restoring
    /// whatever it held before -- the same technique `fs.rs`'s
    /// `stash_for_test` uses. Needed wherever a test drives code that touches
    /// task-keyed state with no real task around it: staging a park aborts
    /// outside a task context, and `fs::Slot` is keyed on `current_task`. The
    /// id only has to agree between the stash and the read back, which is why
    /// most call sites below pass `0`; the few that pass a real `id` are
    /// reading what an actually-spawned task stashed -- see
    /// [`a_successful_connect_stashes_its_fd_via_slot_buffer`] for the case
    /// that motivated this helper.
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

    /// Test-only: whether `fd` is currently a live **stream** entry in the
    /// socket table -- the direct check for "absence from the table is
    /// closedness," without needing a read to distinguish "closed" from
    /// "open but idle" (a `WouldBlock` on a still-open, still-idle socket
    /// maps to the same non-`OK` status a closed one does -- both fall to
    /// `fail`'s `_ => OTHER` arm).
    ///
    /// **Narrower than its name since [`Sock`] arrived**: it went from "is
    /// registered" to "is registered as a stream" when its body became
    /// [`with_stream`]. Most callers pass an fd `connect` produced and cannot
    /// tell the difference -- but **one caller deliberately can, and its whole
    /// assertion rests on it**:
    /// `net_listen_binds_an_ephemeral_port_and_registers_a_listener` checks
    /// `!is_open_for_test(fd)` against an fd **`net_listen`** produced, which
    /// is the narrowing being observed rather than merely tolerated. That is
    /// what makes this the right partner for [`is_listener_for_test`] -- the
    /// two together say which variant an fd holds, not merely that it holds
    /// one.
    ///
    /// The sentence this replaced claimed no caller could tell the difference.
    /// That was false the moment it was written: the assertion above shipped in
    /// the very same commit, two screens further down this file.
    fn is_open_for_test(fd: i64) -> bool {
        with_stream(fd, |_| ()).is_some()
    }

    /// Test-only: whether `fd` is currently a live **listener** entry in the
    /// socket table -- [`is_open_for_test`]'s mirror image across [`Sock`],
    /// with the identical body against [`with_listener`].
    fn is_listener_for_test(fd: i64) -> bool {
        with_listener(fd, |_| ()).is_some()
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

    /// Test-only: block until `expected` bytes the peer has written are
    /// genuinely sitting in `fd`'s receive buffer, so a test that asserts what
    /// one specific poll returns can establish that precondition instead of
    /// assuming it.
    ///
    /// **Why any test needs this.** `Write::write_all` on the peer socket
    /// returns once the bytes are in the *sender's* send buffer. Moving them
    /// into this socket's receive buffer is a separate, kernel-internal step,
    /// and nothing in `write_all`'s contract says it has already happened by
    /// the time the call returns. On Linux and Windows loopback delivery
    /// completes inside the writing syscall in practice, so a non-blocking read
    /// issued immediately afterwards finds the data and a test that assumed so
    /// passes; on macOS it does not, and
    /// [`a_read_with_data_already_waiting_completes_on_the_first_poll_without_parking`]
    /// failed on `macos-latest` CI for exactly that reason while every other
    /// test in this module passed there -- including every test whose read
    /// goes through the real executor's park/wake loop, which absorbs the
    /// delay. [`an_eof_read_stashes_an_empty_payload_not_an_error`] already
    /// records the same asynchrony for the FIN a peer's `close` delivers, and
    /// takes that same executor-driven way out; the two tests calling this
    /// helper cannot, because what they assert *is* the return value of one
    /// particular poll.
    ///
    /// **Observes the arrival; never waits a fixed time for it.** `peek`
    /// reports what is in the receive buffer without consuming it, so this can
    /// be called as often as it likes and the read under test still sees every
    /// byte. What it blocks in between attempts is [`crate::poll::wait`] on
    /// real read-readiness -- this crate's own poller, the same primitive the
    /// executor parks tasks on -- not a sleep. So a caller proceeds the instant
    /// the precondition it assumes actually holds, and fails with this
    /// helper's own message rather than a bare `POLL_PENDING` mismatch if it
    /// never does. The deadline is a bound on the failure, not a wait anyone
    /// pays for on the passing path: where delivery is synchronous the first
    /// `peek` already succeeds and `wait` is never reached at all.
    ///
    /// A `peek` reporting *fewer* than `expected` bytes loops the same way a
    /// `WouldBlock` does. That case leaves the socket readable, so `wait`
    /// returns at once and this spins rather than blocking -- bounded by the
    /// same deadline, and transient by construction, since the peer these
    /// tests use has already written the whole payload before this is called.
    fn await_peer_bytes_for_test(fd: i64, expected: usize) {
        let socket = with_stream(fd, |stream| raw_socket_of(stream)).expect("fd must be open");
        let deadline = Instant::now() + Duration::from_secs(10);
        let mut buf = vec![0u8; expected];
        loop {
            let peeked = with_stream(fd, |stream| stream.peek(&mut buf)).expect("fd must be open");
            match peeked {
                Ok(n) if n >= expected => return,
                Ok(_) => {}
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {}
                Err(e) => panic!("test setup: peeking for the peer's bytes failed: {e}"),
            }
            assert!(
                Instant::now() < deadline,
                "test setup: the {expected} bytes the peer wrote never reached \
                 this socket's receive buffer"
            );
            crate::poll::wait(&[(socket, Interest::Read)], Some(deadline));
        }
    }

    /// Test-only: block until `fd`'s peer address is actually reportable, and
    /// hand it back -- so a test can assert *which* address a connect landed on
    /// without assuming that answer is available the instant this module calls
    /// the connection established.
    ///
    /// **Why a single `peer_addr` call is not enough, and why the reason is
    /// the calling test's own shape rather than the platform's.** Its one
    /// caller, [`an_ipv6_connect_parks_on_its_first_poll_and_then_establishes`],
    /// polls the connect future **by hand** and then hands *that same future*
    /// to `block_on`. The manual poll sets `STATE_SLOT_TAG` to 1 and stages its
    /// park into a `CURRENT` borrowed by `with_test_current`, which is restored
    /// on the way out -- so no task ever exists to commit that park to
    /// `PARKED`. `block_on` then spawns the future fresh, its first poll of it
    /// reads `tag == 1`, and [`poll_connect`] drops straight into
    /// [`finish_connect`]. **`crate::poll::wait` never runs for this socket at
    /// all**, so `SO_ERROR` is clear here because nothing has *failed* yet --
    /// microseconds after `connect` was issued -- not because the connection
    /// completed. On loopback it usually has anyway; on Darwin, sometimes it
    /// has not, and `getpeername` says `NotConnected`/`ENOTCONN` (errno 57).
    ///
    /// **What that retires.** An earlier version of this comment read the same
    /// failure as Darwin reporting a socket write-ready before it was usable,
    /// and left a question standing about whether [`finish_connect`]'s success
    /// condition -- write-readiness plus a clear `SO_ERROR`, the POSIX
    /// completion test for a non-blocking connect -- could tell "not finished
    /// yet" from "finished successfully" on any platform. A throwaway probe
    /// settled it (`macos-latest` CI, PR #15, closed unmerged): of **522**
    /// second polls sampled for write-readiness, `SO_ERROR` and `peer_addr`
    /// together, the **520** that were reached through a real
    /// `crate::poll::wait` readiness report had a usable connection every
    /// time, and the **2** that reported `ENOTCONN` were *not* write-ready and
    /// had been woken by nothing at all. Darwin never claimed readiness it did
    /// not have. `finish_connect` needs no stronger check, and its
    /// precondition is simply not satisfied by a pre-polled future.
    ///
    /// Those 2 samples came one from this helper's caller and one from
    /// [`connect_parks_on_its_first_poll_rather_than_completing_synchronously`],
    /// which is built the same way over V4 -- so the shape is the cause, not
    /// anything specific to IPv6. That test needs no helper only because it
    /// never asks its socket about itself, so the same in-flight connect is
    /// invisible there rather than absent.
    ///
    /// So `block_on` returning `OK` to a caller that pre-polled proves only
    /// that nothing had failed by then. That is worth knowing before reading
    /// either test's `OK` as evidence the connection established: what proves
    /// *that* is the peer address arriving, below. Restructuring both tests to
    /// drive the park for real -- and so make their `OK` load-bearing -- would
    /// remove the need for this helper; it is deliberately not done here,
    /// because a park honoured end to end is already covered by the
    /// `net_*.nova` fixtures and by every other `connect` test in this module.
    ///
    /// The handling is the same as [`await_peer_bytes_for_test`] and
    /// [`fill_send_buffer_until_would_block_for_test`] use for genuine
    /// loopback asynchrony, even though the cause here is different: observe
    /// the state actually arriving, never sleep a fixed time for it, and fail
    /// with a message of this helper's own if it never does.
    ///
    /// Tolerates exactly `NotConnected`, the one transient that was measured
    /// (2 of 522 samples, `macos-latest` only). Any other error fails
    /// immediately rather than being retried into a timeout, and so does an
    /// address that never arrives.
    fn await_peer_addr_for_test(fd: i64) -> SocketAddr {
        let socket = with_stream(fd, |stream| raw_socket_of(stream)).expect("fd must be open");
        let deadline = Instant::now() + Duration::from_secs(10);
        loop {
            match with_stream(fd, |stream| stream.peer_addr()).expect("fd must be open") {
                Ok(addr) => return addr,
                Err(e) if e.kind() == std::io::ErrorKind::NotConnected => {}
                Err(e) => panic!("test setup: reading the peer address failed: {e}"),
            }
            assert!(
                Instant::now() < deadline,
                "the connection this module reported established never became \
                 reportable by getpeername at all"
            );
            crate::poll::wait(&[(socket, Interest::Write)], Some(deadline));
        }
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
    ///
    /// **The fullness this establishes is instantaneous, not durable.** All a
    /// returned `WouldBlock` says is that the buffer was full at the instant
    /// of that one syscall. TCP frees a sender's buffered bytes as the peer
    /// ACKs them, and the peer ACKs whatever fits in its own receive buffer,
    /// so room can reappear at any moment until that receive buffer is full
    /// as well -- and on macOS's loopback the transfer that leads to those
    /// ACKs keeps happening after the writing syscall has returned, the same
    /// asynchrony [`await_peer_bytes_for_test`] exists for, seen from the
    /// sending end. A caller that asserts on the return value of one
    /// particular poll must therefore re-establish fullness and re-poll
    /// rather than assume this call's result still holds when that poll
    /// finally runs; [`poll_write_until_it_parks_for_test`] is that loop, and
    /// both callers asserting a park go through it.
    /// [`try_write_reports_would_block_once_the_send_buffer_fills`] is the
    /// one caller that does not need it: it asserts nothing at all after this
    /// returns, so it has no precondition left to lose.
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

    /// Test-only: poll `fut` -- a write future over `fd` -- until one poll
    /// runs against a genuinely full send buffer, and assert that *that* poll
    /// both returned `POLL_PENDING` and staged a park on `socket` for
    /// `Interest::Write` with no deadline. Re-fills with `chunk` and polls
    /// again whenever a poll instead finds room and completes. `context`
    /// names the poll in every failure message.
    ///
    /// **Why one fill-then-poll is not enough.**
    /// [`fill_send_buffer_until_would_block_for_test`] establishes fullness
    /// from one syscall's result, and it is only true at that instant (see
    /// that helper's own doc comment for the TCP mechanism). Every caller
    /// then has a gap to cross before the poll under test runs -- building
    /// the payload, building the future, reaching into its fat pointer -- and
    /// on macOS's loopback the kernel keeps moving bytes to the peer across
    /// that gap, the peer ACKs them, room reappears, and the write completes
    /// instead of parking.
    /// `a_write_against_a_full_send_buffer_parks_rather_than_spinning` failed
    /// on `macos-latest` CI exactly that way -- `POLL_READY` where
    /// `POLL_PENDING` was asserted -- on a pull request whose entire diff was
    /// comment text, while the ubuntu and windows legs of the same run passed
    /// and the same test had passed on macOS one commit earlier. A
    /// behavioural failure a comment cannot cause is a flake by elimination.
    ///
    /// **Bounded, and not a sleep.** Nothing here waits for a fixed time:
    /// each attempt re-establishes the precondition with real writes and
    /// re-polls, so the passing path on a platform that never drains pays one
    /// iteration. The loop terminates because the drain is finite -- nothing
    /// ever reads the peer these tests use, so the total it can ever ACK is
    /// bounded by its own receive buffer, each attempt's completing write
    /// takes back all the room that appeared (`Write::write` writes as much
    /// as fits) and the re-fill above it takes the rest, and once the peer's
    /// receive buffer is full it advertises a zero window and no further room
    /// can appear at all. That fixpoint, not a timeout, is what makes the
    /// bound reachable.
    ///
    /// **Tolerating a drain does not weaken what the two callers exist for.**
    /// The two properties they are built around both still hold on exactly
    /// one poll's result. A `poll_write` that stripped its own
    /// `stage_io_park` call would still return `POLL_PENDING` from the
    /// attempt that finds the buffer full, so the staged-park assertion --
    /// not the status -- is still what catches it. And `POLL_READY` is
    /// tolerated only as a *successful write of a positive count*: a
    /// would-block arm that reported a failure status instead of parking
    /// fails the output-slot assertion on the first attempt, and one that
    /// reported a zero-byte success -- room that cannot have appeared --
    /// fails the count assertion just as immediately. Neither is quietly
    /// retried away.
    fn poll_write_until_it_parks_for_test(
        fd: i64,
        chunk: &[u8],
        fut: *mut u8,
        socket: RawSocket,
        context: &str,
    ) {
        let (poll, state) = poll_fn_and_state(fut);
        for _ in 0..64 {
            fill_send_buffer_until_would_block_for_test(fd, chunk);
            // SAFETY: `poll`/`state` are the pair inside `fut`, a well-formed
            // future this module built; `task_ctx` is null, as `poll_one`'s
            // own call passes.
            let status = with_test_current(0, || unsafe { poll(state, std::ptr::null_mut()) });
            if status == POLL_PENDING {
                assert_eq!(
                    crate::task::staged_io_for_test(),
                    Some((socket, Interest::Write, None)),
                    "{context} must genuinely stage a park on Interest::Write \
                     for this exact socket, with no deadline -- not merely \
                     return POLL_PENDING while parking nothing"
                );
                return;
            }
            assert_eq!(
                status, POLL_READY,
                "{context} returned neither of the two statuses the ABI \
                 defines"
            );
            // Checked before the count below, not after: a failure status
            // stashes a message rather than a byte count, so reading the
            // count first would decode an empty `Slot::Buffer`.
            assert_eq!(
                state_slot_of(fut, STATE_SLOT_OUTPUT),
                OK,
                "{context} did not park, so the only tolerable reason is that \
                 room reappeared and this write took it -- a failure status \
                 is the wrong status, not a drained buffer"
            );
            let written = with_test_current(0, take_fd);
            assert!(
                written > 0,
                "{context} reported a successful write of {written} bytes, so \
                 no room can have reappeared -- a would-block reporting a \
                 zero-byte write instead of parking"
            );
        }
        panic!(
            "{context}: 64 attempts, each re-filling the send buffer with \
             {}-byte writes until one reported WouldBlock, and not one of \
             them ever parked -- nothing reads the peer, so the room a drain \
             can free is bounded by its receive buffer and has to run out",
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
        // this closed fd never has data waiting, so its `with_stream` `None` arm
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
    /// `with_stream`'s `None` arm as a closed one -- the `net.rs` shape of the
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

    /// `net_listen` on port 0 must bind, listen, and land a **listener** in
    /// the shared table -- not a stream, which is the variant every other
    /// entry this module creates holds.
    ///
    /// No [`with_test_current`] wrapper, unlike the connect tests that read
    /// `Slot::Buffer`: nothing here is spawned, so `nova_rt_net_listen`'s own
    /// `stash_i64` and this test's `take_fd` run under whatever `CURRENT`
    /// already held, which is the same value for both since nothing in
    /// between changes it. Those tests need the wrapper because the id they
    /// read under belongs to a task the executor has already finished.
    #[test]
    fn net_listen_binds_an_ephemeral_port_and_registers_a_listener() {
        let addr = crate::gc_str("127.0.0.1:0");
        // SAFETY: `addr` is the live `NovaStr` allocated immediately above.
        let status = unsafe { nova_rt_net_listen(addr) };
        assert_eq!(status, OK, "binding loopback port 0 must succeed");
        let fd = take_fd();
        assert!(fd > 0, "fd must be a real handle, got {fd}");
        assert!(
            is_listener_for_test(fd),
            "fd must be registered as a listener"
        );
        assert!(
            !is_open_for_test(fd),
            "and must NOT answer as a stream -- one table, two variants, and \
             a listener is not readable"
        );
        assert_eq!(unsafe { nova_rt_net_close(fd) }, OK);
    }

    /// An address no local interface owns cannot be bound, and that failure
    /// has to arrive as a status rather than as a handle nothing listens on.
    /// `192.0.2.0/24` is TEST-NET-1 (RFC 5737), reserved for documentation and
    /// never routed or assigned, so no host has it -- chosen over a
    /// privileged-port-on-loopback failure, which would depend on whether the
    /// test runs as root.
    ///
    /// **Measured on Windows only so far**, where the status is
    /// `WSAEADDRNOTAVAIL` (`os error 10049`); the Unix arms are *reasoned, not
    /// measured*, this module's own convention for a path read rather than run
    /// (see `poll.rs`'s "Still reasoned, not measured" note for the
    /// precedent). `EADDRNOTAVAIL` is the documented answer there and CI will
    /// settle it on `ubuntu-latest` and `macos-latest` the first time this runs
    /// on them. The assertion is deliberately `!= OK` rather than a specific
    /// status for that reason: what this test pins is that an unbindable
    /// address is an error and not a handle, which holds under any of them.
    #[test]
    fn net_listen_reports_an_error_for_an_unbindable_address() {
        let addr = crate::gc_str("192.0.2.1:1");
        // SAFETY: `addr` is the live `NovaStr` allocated immediately above.
        let status = unsafe { nova_rt_net_listen(addr) };
        assert_ne!(status, OK, "binding an unroutable address must fail");
    }

    /// `net_local_port` on a listener bound to port 0 must report the port the
    /// kernel actually chose. That is the whole reason the intrinsic exists:
    /// `bind("127.0.0.1:0")` otherwise leaves a caller holding a listening
    /// socket with no way to tell anyone where it is.
    ///
    /// **The range check alone does not catch a wrong port**, so the assertion
    /// that carries the weight is the equality against the listener's own
    /// `local_addr`: it pins that the number reaching `Slot::Buffer` is this
    /// listener's port *unaltered*, which is the whole claim. It shares
    /// [`with_listener`] and `local_addr` with the code under test and is not
    /// an independent oracle, deliberately -- what is under test here is the
    /// encode-and-stash step between them, and a mutation anywhere in it dies.
    ///
    /// **The `std` connect below is a reachability check, not a second
    /// discriminator, and this is measured rather than assumed.** Mutating the
    /// intrinsic to stash `port + 1` left the dial *passing*: `cargo test` runs
    /// this binary's tests in parallel threads, several of which hold their own
    /// ephemeral loopback listeners, so a neighbouring port is routinely
    /// occupied by a sibling test and answers the connect. The dial still earns
    /// its place -- it proves the reported port is genuinely listening and
    /// accepting, which a port number read out of a struct cannot show -- but a
    /// reader must not mistake it for the assertion that pins the value. The
    /// peer socket is never accepted -- `nova_rt_net_accept_future` now exists
    /// and this test still does not use it -- and does not need to be: the
    /// handshake completes in the kernel on a listening socket's behalf whether
    /// or not the application ever picks the connection up. That is exactly
    /// what keeps this test about `local_port` alone;
    /// [`accept_parks_until_a_client_connects_then_yields_a_stream_fd`] is
    /// where picking the connection up is what is under test.
    ///
    /// **The port arrives in `Slot::Buffer`, not in the status word**, so this
    /// reads it with [`take_fd`] exactly as
    /// [`net_listen_binds_an_ephemeral_port_and_registers_a_listener`] reads
    /// its fd -- and with no [`with_test_current`] wrapper, for the reason that
    /// test's own doc comment records: nothing here is spawned, so the stash
    /// and the read back run under the same `CURRENT`.
    #[test]
    fn net_local_port_reports_the_kernel_assigned_port() {
        let addr = crate::gc_str("127.0.0.1:0");
        // SAFETY: `addr` is the live `NovaStr` allocated immediately above.
        assert_eq!(unsafe { nova_rt_net_listen(addr) }, OK);
        let fd = take_fd();

        let status = unsafe { nova_rt_net_local_port(fd) };
        assert_eq!(status, OK, "a bound listener must report its port");
        let port = take_fd();
        assert!(
            port > 0 && port < 65536,
            "port must be a real, in-range port the kernel assigned, got {port}"
        );

        let actual = with_listener(fd, |l| l.local_addr())
            .expect("the fd must still be registered as a listener")
            .expect("a bound listener must have a local address");
        assert_eq!(
            port,
            i64::from(actual.port()),
            "the stashed port must be this listener's own port, unaltered -- \
             the encode-and-stash step between `local_addr` and `Slot::Buffer` \
             is what this pins"
        );

        let dialed = std::net::TcpStream::connect(("127.0.0.1", port as u16));
        assert!(
            dialed.is_ok(),
            "nothing accepted a connection on port {port}. The assertion above \
             has already established that this is the listener's own port, so \
             the port number is not the problem -- this fd is registered and \
             bound but not actually listening, which is what `net_listen` \
             failing to reach its `listen` step would look like: {dialed:?}"
        );
        drop(dialed);

        assert_eq!(unsafe { nova_rt_net_close(fd) }, OK);
    }

    /// A connected stream fd must be an error here, not a port -- **and the
    /// syscall is not what refuses it**. A connected socket genuinely has a
    /// local port, so `local_addr` would have answered happily; what fails is
    /// [`with_listener`]'s kind check, before any syscall runs.
    ///
    /// **The status is `OTHER`, the identical one an absent fd produces**, and
    /// this pins that rather than merely `!= OK`, exactly as
    /// [`reading_a_listener_fd_is_an_error_not_a_stream_read`] pins the mirror
    /// case. The collapse is deliberate and recorded on [`closed_fd_error`];
    /// a reader looking for a distinguishable wrong-kind status will not find
    /// one to rely on.
    ///
    /// The far end is held alive for the whole test. Dropping it would tear the
    /// connection down underneath the assertion and leave a passing test that
    /// proved something else.
    #[test]
    fn net_local_port_on_a_stream_fd_is_an_error() {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind");
        let addr = listener.local_addr().expect("addr").to_string();
        let fd = connect_blocking_for_test(&addr);
        let (_server, _) = listener.accept().expect("accept");

        let status = unsafe { nova_rt_net_local_port(fd) };
        assert_eq!(
            status, OTHER,
            "a stream is the wrong kind for local_port, and must report the \
             same catch-all error an absent fd does -- not succeed with the \
             stream's own local port, and not a distinct wrong-kind code"
        );

        assert_eq!(unsafe { nova_rt_net_close(fd) }, OK);
    }

    /// An fd this module never issued takes the same path a wrong-kind one
    /// does -- [`with_listener`]'s `None` arm, reported by
    /// [`closed_fd_error`]. The `net_local_port` sibling of
    /// `closing_a_never_issued_fd_is_still_ok`, except that a *read* of an
    /// absent fd is an error rather than the no-op a close is.
    #[test]
    fn net_local_port_on_an_absent_fd_is_an_error() {
        let status = unsafe { nova_rt_net_local_port(999_999) };
        assert_eq!(
            status, OTHER,
            "an unregistered fd must not report a port, and reports the same \
             catch-all error a wrong-kind fd does"
        );
    }

    /// A read against a listener fd must fail rather than treat the entry as a
    /// stream. Driven through the real production `read` future, as
    /// `a_connected_socket_closes_once_and_then_reports_not_open` is: the
    /// `with_stream` `None` arm resolves on the very first poll, so no park is
    /// involved.
    ///
    /// **The status is `OTHER`, the identical one a closed, stale or forged fd
    /// produces**, and this pins that rather than merely `!= OK`. The collapse
    /// is deliberate, recorded on `closed_fd_error` itself: `fs.rs`'s
    /// `IoErrorKind` table has no wrong-kind variant, and adding one is a
    /// wire-contract change across `fs.rs` and `std/io` together. So this test
    /// asserts what the boundary actually promises -- an error, of the
    /// catch-all kind -- and a reader looking for a distinguishable wrong-kind
    /// status will not find one to rely on.
    #[test]
    fn reading_a_listener_fd_is_an_error_not_a_stream_read() {
        let addr = crate::gc_str("127.0.0.1:0");
        // SAFETY: `addr` is the live `NovaStr` allocated immediately above.
        assert_eq!(unsafe { nova_rt_net_listen(addr) }, OK);
        let fd = take_fd();
        let status = unsafe { crate::task::nova_rt_task_block_on(nova_rt_net_read_future(fd, 16)) };
        assert_eq!(
            status, OTHER,
            "a read on a listener must report the same catch-all error an \
             absent fd does, not succeed and not a distinct wrong-kind code"
        );
        assert_eq!(unsafe { nova_rt_net_close(fd) }, OK);
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

    /// The one test that reaches `platform_connect`'s `SocketAddr::V6` arm at
    /// all, on any platform. Every other test in this file and every
    /// `tests/runtime/net_*.nova` fixture connects to `127.0.0.1`, so until
    /// this test both `sockaddr_in6` arms -- Unix and Windows -- were shipped
    /// code with zero coverage, and `sin6_len`-left-zero on Darwin carried
    /// exactly the documentation-only status `sin_len` held before CI first
    /// drove the V4 path for real (this module's own doc comment records both).
    ///
    /// **The bracketed literal is the Nova-visible spelling, and this pins
    /// that it resolves.** `std/net`'s `connect` takes a `String` and hands it
    /// to this module unchanged, so whether a Nova caller can reach the V6 arm
    /// at all is entirely a question about [`resolve_addr`] -- which runs
    /// `std`'s own `ToSocketAddrs`, and that parses a `[::1]:0`-style bracketed
    /// literal as a `SocketAddr` before it ever considers a name lookup. Hence
    /// the address below is `TcpListener::local_addr().to_string()`, already
    /// bracketed exactly as a caller would have to write it, and hence the
    /// `resolve_addr` assertion: with a V4 address coming back out of it
    /// nothing that follows would touch the arm this test exists for, and the
    /// test would pass while covering the V4 arm one more redundant time.
    ///
    /// **Not gated, and deliberately not a skip.** All three CI runners are
    /// expected to have `::1` on their loopback interface, independently of
    /// having no IPv6 route off the machine. A platform where that turned out
    /// false fails this test's `bind` loudly, which is the intended outcome:
    /// this project's one platform-divergent fixture
    /// (`tests/runtime/file_open_dir.nova`) is `#[cfg(windows)]`-gated with its
    /// reason written down rather than silently skipped, and a swallowed
    /// `bind` failure turning into a pass would be precisely the silent skip
    /// that convention exists to rule out. Should a runner ever lack `::1`, the
    /// remedy is a gate with its reason recorded here, not a tolerated error.
    ///
    /// **Asserts on one poll, not only on the outcome.** Follows
    /// [`connect_parks_on_its_first_poll_rather_than_completing_synchronously`]
    /// for the reason that test's own doc comment gives -- a blocking V6
    /// connect would return `POLL_READY` here and still establish the
    /// connection, and an end-to-end test that merely drives the executor
    /// cannot tell a park from a busy spin, since both eventually complete. It
    /// additionally reads back *what was staged*, which that older test does
    /// not: a `poll_connect` returning `POLL_PENDING` while staging nothing
    /// would be busy-re-queued through `QUEUE` and still complete, the same
    /// hole the `read`/`write` tests below already close for themselves.
    ///
    /// **What this cannot distinguish: `sin6_flowinfo` and `sin6_scope_id`.**
    /// Both are zero for `::1`, and both are zero in a `sockaddr_in6` whose
    /// construction dropped them, so an implementation that never assigned
    /// either passes this test. That limit is real and not reasonably
    /// closable, but it is also bounded: `std`'s `SocketAddr` parser accepts no
    /// textual syntax for either field, so no `String` a Nova caller can write
    /// yields a non-zero value for them through [`resolve_addr`] at all. The
    /// two assignments are unreachable from the Nova surface as anything but
    /// zero, which makes dropping them unobservable rather than
    /// untested-and-load-bearing.
    ///
    /// **`sin6_addr` is pinned, and that was measured rather than assumed.** A
    /// construction that left the address zeroed dials `[::]`, the unspecified
    /// address, and the standing worry was that a platform substitutes loopback
    /// for it the way every platform here does for IPv4's `0.0.0.0` -- which
    /// would land that mutation on this test's own listener and survive it,
    /// `peer_addr` check included, since the address the connection settles on
    /// would *be* the substituted one. Measured on Windows: it does not, and
    /// the connect fails outright. Reversed octets fail there too. Whether
    /// Linux and Darwin refuse `[::]` the same way is reasoned, not measured
    /// -- the Unix arms of this file are exercised only by CI, and the
    /// pre-existing V4 tests carry the identical open question about `0.0.0.0`
    /// for the identical reason, so this is a property of loopback-only
    /// testing rather than something this test gave up.
    #[test]
    fn an_ipv6_connect_parks_on_its_first_poll_and_then_establishes() {
        let listener = std::net::TcpListener::bind("[::1]:0").expect(
            "bind an IPv6 loopback listener -- see this test's own doc comment \
             on why a runner without ::1 must fail here rather than skip",
        );
        let addr = listener.local_addr().expect("addr").to_string();
        assert!(
            matches!(resolve_addr(&addr), Ok(SocketAddr::V6(_))),
            "test setup: {addr} must resolve to a V6 socket address, or \
             nothing below reaches platform_connect's V6 arm at all"
        );

        let addr_ptr = crate::gc_str(&addr);
        let fut = unsafe { nova_rt_net_connect_future(addr_ptr) };
        let (poll, state) = poll_fn_and_state(fut);
        // SAFETY: `poll`/`state` are the pair inside `fut`, a well-formed
        // future this module just built; `task_ctx` is null, matching every
        // other call site in this crate. `stage_io_park` aborts outside a task
        // context, so one is borrowed for this one manual poll and restored
        // afterward, the same technique the V4 park test above uses.
        let first = with_test_current(0, || unsafe { poll(state, std::ptr::null_mut()) });
        assert_eq!(
            first, POLL_PENDING,
            "a non-blocking V6 connect must park on its first poll rather \
             than complete synchronously"
        );

        let fd = state_slot_of(fut, CONNECT_SLOT_SOCK);
        let expected_socket = with_stream(fd, |stream| raw_socket_of(stream))
            .expect("the first poll must have registered its connecting socket");
        assert_eq!(
            crate::task::staged_io_for_test(),
            Some((expected_socket, Interest::Write, None)),
            "a V6 connect must genuinely stage a park on Interest::Write for \
             this exact socket, with no deadline -- not merely return \
             POLL_PENDING while parking nothing"
        );

        // The other half: it really connects. `block_on` drives the same
        // future the rest of the way, and `finish_connect`'s `OK` is
        // `SO_ERROR` reporting no error. Read that `OK` for no more than it is
        // worth here: because the poll above already set the tag, this poll
        // runs without any `poll::wait` in between, so `OK` says nothing had
        // failed yet -- see [`await_peer_addr_for_test`]'s own doc comment for
        // the measurement behind that, and for what it retires.
        //
        // A sockaddr this arm built wrong is therefore caught at one of the
        // two ends rather than in the middle: a family the socket does not
        // have is rejected by `connect` itself, so it never parks and the
        // assertion above fails; a port that skipped `to_be` names some other
        // address, which the assertion below never sees arrive.
        assert_eq!(
            unsafe { crate::task::nova_rt_task_block_on(fut) },
            OK,
            "a V6 connect to a live ::1 listener must establish"
        );
        assert_eq!(
            await_peer_addr_for_test(fd),
            listener.local_addr().expect("addr"),
            "connect must have landed on this test's own ::1 listener, over \
             IPv6 -- not on whatever other address a mis-built sockaddr_in6 \
             happened to name"
        );
        let (_server, _) = listener.accept().expect("accept");
        assert_eq!(unsafe { nova_rt_net_close(fd) }, OK);
    }

    /// The one test that proves `finish_connect`'s successful path really
    /// does stash the new fd via `fs::Slot::Buffer` -- the side channel
    /// Task 5's Nova-facing wrapper will read from, and the piece every other
    /// ***connect*** test in this file deliberately bypasses (see
    /// [`state_slot_of`]'s own doc comment for why they do, and why that is
    /// not itself a hole in coverage without this test to fill it).
    ///
    /// **Scoped to the connect tests deliberately.** This once read "every
    /// other test in this file", which was false: the `read`/`read_timeout`
    /// tests have always read `Slot::Buffer` through `take_bytes_for_test`, and
    /// `net_listen`'s and `net_local_port`'s tests read it through `take_fd`.
    /// What is special here is not touching the slot at all -- it is being the
    /// only test that reads *`finish_connect`'s* fd from it rather than out of
    /// the state object, which is the ordering hazard [`state_slot_of`]
    /// documents.
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
        let expected_socket =
            with_stream(fd, |stream| raw_socket_of(stream)).expect("fd must be open");

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
        // The write returning does not mean the bytes have reached *this*
        // socket yet, and this second poll is the only one this test gets --
        // see [`await_peer_bytes_for_test`] for the platform this distinction
        // was caught on and why a park/wake-driven test does not need it.
        await_peer_bytes_for_test(fd, b"hello".len());

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
        let expected_socket =
            with_stream(fd, |stream| raw_socket_of(stream)).expect("fd must be open");

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

    /// **"Already waiting" is established, not assumed.** This test asserts
    /// what the *first* poll of a fresh read future returns, so the data has to
    /// actually be in this socket's receive buffer before that poll runs --
    /// which the peer's `write_all` returning does not by itself mean.
    /// [`await_peer_bytes_for_test`] observes the arrival; its doc comment
    /// carries the platform difference that caught this and why a fixed sleep
    /// would be the wrong fix.
    #[test]
    fn a_read_with_data_already_waiting_completes_on_the_first_poll_without_parking() {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind");
        let addr = listener.local_addr().expect("addr").to_string();
        let fd = connect_blocking_for_test(&addr);
        let (mut server, _) = listener.accept().expect("accept");
        std::io::Write::write_all(&mut server, b"hi").expect("write");
        await_peer_bytes_for_test(fd, b"hi".len());

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

    /// The production write future, against a full send buffer, must park
    /// rather than spin or silently fail -- this project's own busy-spin
    /// hazard, on the write side this time. Matches
    /// `connect_parks_on_its_first_poll_rather_than_completing_synchronously`'s
    /// own technique: only the *return value of one poll* tells a real park
    /// apart from a mutation that would still eventually succeed via
    /// `block_on` alone -- **and even that return value is not enough on its
    /// own**: a future that strips its own `stage_io_park` call still returns
    /// `POLL_PENDING` here and would still eventually succeed through the
    /// real executor's busy re-queue, since nothing would ever move it into
    /// `PARKED` at all. So the staged park is read back too, not only the
    /// poll's return value.
    ///
    /// Both of those checks, and the reason the full buffer has to be
    /// re-established around each attempt rather than assumed to survive the
    /// gap between the fill and the poll, live in
    /// [`poll_write_until_it_parks_for_test`].
    ///
    /// The fill goes through repeated direct `try_write` calls, never
    /// `block_on`, which would hang this test's own thread forever the moment
    /// a write genuinely needs to park with nothing ever draining the peer
    /// (see [`fill_send_buffer_until_would_block_for_test`]).
    #[test]
    fn a_write_against_a_full_send_buffer_parks_rather_than_spinning() {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind");
        let addr = listener.local_addr().expect("addr").to_string();
        let fd = connect_blocking_for_test(&addr);
        let (_server, _) = listener.accept().expect("accept");
        let expected_socket =
            with_stream(fd, |stream| raw_socket_of(stream)).expect("fd must be open");

        let chunk_bytes = vec![0xABu8; 1024 * 1024];
        let chunk = crate::gc_bytes_for_test(&chunk_bytes);
        let fut = unsafe { nova_rt_net_write_future(fd, chunk) };
        poll_write_until_it_parks_for_test(
            fd,
            &chunk_bytes,
            fut,
            expected_socket,
            "a write against a full send buffer",
        );

        drain_stray_pending_park_for_test();
        assert_eq!(unsafe { nova_rt_net_close(fd) }, OK);
    }

    /// The write-side counterpart of
    /// [`a_read_still_blocked_on_its_second_poll_stages_a_park_again`] -- see
    /// that test's own doc comment for the gap this closes and why draining
    /// between the two polls is required.
    ///
    /// **Why the second round re-fills too, and what "second poll" means
    /// here.** The read counterpart establishes its precondition by *not*
    /// writing on the peer, which nothing can undo, so its two polls can sit
    /// back to back. The write side's precondition is a full send buffer,
    /// which the kernel can undo between any two polls
    /// ([`poll_write_until_it_parks_for_test`] carries the mechanism), and the
    /// `drain_stray_pending_park_for_test` call between the two rounds is
    /// itself such a gap -- it runs a real `block_on`. So each round
    /// re-establishes fullness and retries until a poll genuinely blocks,
    /// which means the second round's parking poll is the future's second
    /// *or later*. That is exactly the property at issue: what the "stage once
    /// ever" shape this test exists to kill gets wrong is every would-block
    /// poll after its first, whichever number it lands on. With the drained
    /// park taken in between, the second round's staged park can only have
    /// come from a fresh `stage_io_park` call.
    #[test]
    fn a_write_still_blocked_on_its_second_poll_stages_a_park_again() {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind");
        let addr = listener.local_addr().expect("addr").to_string();
        let fd = connect_blocking_for_test(&addr);
        let (_server, _) = listener.accept().expect("accept");
        let expected_socket =
            with_stream(fd, |stream| raw_socket_of(stream)).expect("fd must be open");

        let chunk_bytes = vec![0xABu8; 1024 * 1024];
        let chunk = crate::gc_bytes_for_test(&chunk_bytes);
        let fut = unsafe { nova_rt_net_write_future(fd, chunk) };

        poll_write_until_it_parks_for_test(
            fd,
            &chunk_bytes,
            fut,
            expected_socket,
            "test setup: the first poll of a write against a full send buffer",
        );

        // Nothing drained the peer deliberately in between: the send buffer
        // is filled right back up below, simulating a wake with no room
        // actually having opened up.
        drain_stray_pending_park_for_test();

        poll_write_until_it_parks_for_test(
            fd,
            &chunk_bytes,
            fut,
            expected_socket,
            "a still-full write on a later poll -- a future that stages once \
             and never again leaves a spuriously-woken task parked forever \
             with nothing left to wake it",
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

    /// Using `read`, `write` or `read_timeout` against a closed fd is an
    /// ordinary error, not a panic -- the async-future shape of `file.rs`'s
    /// `using_a_file_after_close_is_an_error_not_a_panic`. Checks all three
    /// individually, since a mutation could plausibly land in just one of
    /// their [`with_stream`] `None` arms without touching the other two.
    ///
    /// **Named rather than counted, and `accept` is deliberately outside it.**
    /// This used to read "the three futures this task adds", which was a count
    /// over a population that has since grown -- [`nova_rt_net_accept_future`]
    /// is a fourth future in this file. It is not folded in here because its
    /// miss is a *different* lookup: these three go through [`with_stream`] and
    /// `accept` goes through [`with_listener`], so covering it would not be one
    /// more iteration of the same shape. **That arm is genuinely unpinned**:
    /// nothing exercises [`try_accept`]'s `None` case, so a closed, stale,
    /// forged or wrong-kind fd reaching `accept` reports
    /// [`closed_fd_error`]'s status by construction rather than by
    /// measurement.
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
        let expected_socket =
            with_stream(fd, |stream| raw_socket_of(stream)).expect("fd must be open");

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
        let expected_socket =
            with_stream(fd, |stream| raw_socket_of(stream)).expect("fd must be open");

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

    // -----------------------------------------------------------------------
    // `accept` -- the listener operation that waits on a peer.
    // -----------------------------------------------------------------------

    /// `accept` must park while nothing has connected and settle with a
    /// **stream** fd once something has -- and the park has to be a real one,
    /// staged on the listener's own socket for `Interest::Read` with no
    /// deadline.
    ///
    /// **Why the return value of the first poll is the load-bearing assertion,
    /// and why it is still not enough on its own.** The same argument
    /// [`a_read_with_no_data_parks_and_completes_when_data_arrives`] makes at
    /// length applies here unchanged: an `accept` that spun on `WouldBlock`
    /// instead of parking, or that treated `WouldBlock` as a settled error,
    /// would still hand back a usable connection once the client below
    /// connects, so an outcome-only test cannot tell a park from a busy spin --
    /// and a future that returns `POLL_PENDING` while staging *nothing* looks
    /// identical from the return value alone. So this reads back what was
    /// actually staged as well, via `crate::task::staged_io_for_test`.
    ///
    /// **Accept-readiness is read-readiness**, which is why the staged
    /// interest asserted below is `Interest::Read` and why this increment adds
    /// no `Interest` variant: `select` reports a pending connection in its
    /// read set and `WSAPoll` reports it as `POLLRDNORM`.
    ///
    /// **The accepted stream must be non-blocking, and the probe at the end is
    /// what pins that.** A fresh connection with nothing written to it yet has
    /// nothing to read, so [`try_read`] against it must report
    /// [`ReadStep::WouldBlock`]; against a *blocking* accepted socket the same
    /// call would block this test's thread instead, so that mutation shows up
    /// as a hang rather than as a failed assertion. `accept` does not propagate
    /// the listener's non-blocking mode on every platform -- Linux's
    /// `accept(2)` explicitly does not -- so without the explicit
    /// `set_nonblocking` this would hang on `ubuntu-latest` alone while passing
    /// here.
    ///
    /// **One known way this can flake, shared with `dead_addr`.** The first
    /// poll asserts that *nothing* has connected yet, and this listener holds
    /// an ephemeral port a sibling test in this same binary could in principle
    /// dial: [`dead_addr`] binds `127.0.0.1:0` and then drops the listener, so
    /// the port it returns is free for this test to be handed and for
    /// `connecting_to_a_closed_port_is_connection_refused` to then dial. That
    /// window is the pre-existing flake [`dead_addr`]'s own comment records,
    /// not a new one, and this test keeps its listener alive for the listener's
    /// whole life rather than reproducing the shape.
    #[test]
    fn accept_parks_until_a_client_connects_then_yields_a_stream_fd() {
        let addr = crate::gc_str("127.0.0.1:0");
        // SAFETY: `addr` is the live `NovaStr` allocated immediately above.
        assert_eq!(unsafe { nova_rt_net_listen(addr) }, OK);
        let listener_fd = take_fd();
        assert_eq!(unsafe { nova_rt_net_local_port(listener_fd) }, OK);
        let port = take_fd();
        let expected_socket = with_listener(listener_fd, |l| raw_socket_of_listener(l))
            .expect("test setup: the fd must be registered as a listener");

        let fut = unsafe { nova_rt_net_accept_future(listener_fd) };
        let (poll, state) = poll_fn_and_state(fut);

        let first = with_test_current(0, || unsafe { poll(state, std::ptr::null_mut()) });
        assert_eq!(
            first, POLL_PENDING,
            "an accept with no client connected must park rather than settle"
        );
        assert_eq!(
            crate::task::staged_io_for_test(),
            Some((expected_socket, Interest::Read, None)),
            "an accept with no client connected must genuinely stage a park on \
             Interest::Read for this listener's own socket, with no deadline -- \
             not merely return POLL_PENDING while parking nothing"
        );
        // The staged park has to go before the next poll can stage its own: a
        // same-kind pair aborts the process (`try_stage`'s own doc comment),
        // which is what makes every later park below observable at all.
        drain_stray_pending_park_for_test();

        let client = std::net::TcpStream::connect(("127.0.0.1", port as u16))
            .expect("test setup: a loopback connect to this listener must succeed");

        // The connect returning does not mean the connection has reached this
        // listener's backlog yet, so this polls until it has, blocking on real
        // read-readiness in between rather than sleeping -- the same shape
        // `await_peer_bytes_for_test` uses for the same kind of asynchrony.
        let deadline = Instant::now() + Duration::from_secs(10);
        let ready = loop {
            let outcome = with_test_current(0, || unsafe { poll(state, std::ptr::null_mut()) });
            if outcome == POLL_READY {
                break outcome;
            }
            assert!(
                Instant::now() < deadline,
                "a client connected on port {port} and this accept never \
                 settled"
            );
            drain_stray_pending_park_for_test();
            crate::poll::wait(&[(expected_socket, Interest::Read)], Some(deadline));
        };
        assert_eq!(ready, POLL_READY);
        assert_eq!(
            unsafe { (state as *mut i64).add(STATE_SLOT_OUTPUT).read() },
            OK,
            "a settled accept must report success, not a status"
        );

        // Read before anything else allocates: `Slot::Buffer`'s payload is
        // rooted by nothing, exactly as `take_fd`'s own comment records.
        let accepted_fd = with_test_current(0, take_fd);
        assert!(
            accepted_fd > 0 && accepted_fd != listener_fd,
            "the accepted connection must get a fresh fd of its own, never the \
             listener's: got {accepted_fd} against listener {listener_fd}"
        );
        assert!(
            is_open_for_test(accepted_fd),
            "the accepted fd must be registered as a stream, so every read, \
             write and read_timeout works on it unchanged"
        );
        assert!(
            !is_listener_for_test(accepted_fd),
            "and must not answer as a listener -- one table, two variants"
        );
        assert!(
            matches!(try_read(accepted_fd, 1), ReadStep::WouldBlock(_)),
            "the accepted stream must be non-blocking: nothing has been \
             written to it, so a read must report would-block rather than \
             waiting for a byte"
        );

        drop(client);
        assert_eq!(unsafe { nova_rt_net_close(accepted_fd) }, OK);
        assert_eq!(unsafe { nova_rt_net_close(listener_fd) }, OK);
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
