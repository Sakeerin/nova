//! Filesystem intrinsics for `std/fs`.
//!
//! # The boundary this module implements
//!
//! Nova has no out-parameters, so an intrinsic returns exactly one word — but
//! `read_to_string` must convey a `String` *and* a two-field `IoError`. Rather
//! than build a Nova sum and record here, which would put Nova value layout in
//! Rust, **the status code returned by each operation *is* the error kind**, and
//! payloads travel in the slots below. `std/fs`'s Nova wrappers map a status to
//! an `IoErrorKind` and construct every `Result` and `IoError` themselves.
//!
//! # Why these slots are `Cell<usize>` and not `RefCell<Option<String>>`
//!
//! **Every function here is called from inside an `async fn`'s generated
//! `$poll`, which has no landing pads** (see `nova_mir::async_lower`'s module
//! doc comment), so nothing here may panic. A `RefCell` borrow can panic; a
//! `Cell` of a `Copy` value cannot. The slots therefore hold a raw
//! already-allocated pointer as a `usize`, with `0` meaning empty.
//!
//! # Why the pointer is GC-rooted while it sits in a slot
//!
//! A slot is written by one intrinsic and read by the next, with Nova code in
//! between. A conservative stack scan cannot see a pointer that lives only in a
//! thread-local, so the object is registered with `gc::add_root` while stashed
//! and released on take — the same balance `task::spawn_internal` and
//! `take_output_internal` keep. Without it the payload could be collected
//! between the two calls, which is why this does not merely rely on "no
//! allocation happens in between".

use crate::{gc, NovaStr};
use std::cell::Cell;
use std::thread::LocalKey;

/// Status codes. `0` is success; every other value is an `IoErrorKind`.
///
/// **This numbering is one half of a wire contract.** The other half is
/// `io_error_kind_of` in `std/io/lib.nova`. The two are independent copies, so
/// they are pinned together by a fixture per kind, not by this comment.
pub const OK: i64 = 0;
pub const NOT_FOUND: i64 = 1;
pub const PERMISSION_DENIED: i64 = 2;
pub const ALREADY_EXISTS: i64 = 3;
pub const INVALID_DATA: i64 = 4;
pub const INTERRUPTED: i64 = 5;
pub const TIMED_OUT: i64 = 6;
pub const CONNECTION_REFUSED: i64 = 7;
pub const OTHER: i64 = 8;

thread_local! {
    /// A GC-rooted `*mut NovaStr` awaiting `nova_rt_fs_take_string`, or 0.
    static STRING_SLOT: Cell<usize> = const { Cell::new(0) };
    /// A GC-rooted array block awaiting `nova_rt_fs_take_string_array`, or 0.
    ///
    /// Unused until Task 3/4 adds the operation that fills it. Declared here
    /// alongside its two siblings because all three slots are one boundary
    /// design introduced together, not three unrelated additions; `#[allow]`
    /// rather than leaving it out because adding it back under time pressure
    /// later is exactly how the drift this module exists to avoid happens.
    #[allow(dead_code)]
    static ARRAY_SLOT: Cell<usize> = const { Cell::new(0) };
    /// A GC-rooted `*mut NovaStr` awaiting `nova_rt_fs_last_error_message`, or 0.
    static MESSAGE_SLOT: Cell<usize> = const { Cell::new(0) };
}

/// Allocate a GC-managed string for a slot payload.
///
/// A thin name for [`crate::gc_str`], reused rather than reproducing
/// `NovaStr { len, ptr }`'s layout here a second time — precisely the drift
/// class this module's boundary design exists to avoid.
fn gc_message(s: &str) -> *mut NovaStr {
    crate::gc_str(s)
}

/// Register `ptr` as a GC root, then publish it in `slot`.
///
/// Rooting before publishing means a scan running between these two
/// statements still finds the object through the root table — there is no
/// instant where it is reachable by neither the slot nor a root.
fn stash(slot: &'static LocalKey<Cell<usize>>, ptr: *mut NovaStr) {
    gc::add_root(ptr as *mut u8);
    slot.set(ptr as usize);
}

/// Read and clear `slot`, releasing its root, and return what was there (`0`
/// if the slot was empty).
///
/// **Ordered exactly as `task::take_output_internal` orders its matching
/// `gc::remove_root`:** the pointer is read out of the slot and the slot is
/// cleared *before* the root is released, so the add/remove stay balanced and
/// the returned pointer is live throughout — rooted by the slot until the
/// instant before this returns, then rooted by the caller's own frame like
/// any other runtime return value.
fn take(slot: &'static LocalKey<Cell<usize>>) -> usize {
    let ptr = slot.get();
    slot.set(0);
    if ptr != 0 {
        gc::remove_root(ptr as *mut u8);
    }
    ptr
}

/// Map an `std::io::Error` to its status code, and stash its message.
///
/// Called on every failure path, so the message is always available to the
/// wrapper that is about to build an `IoError`.
fn fail(e: &std::io::Error) -> i64 {
    use std::io::ErrorKind;
    stash(&MESSAGE_SLOT, gc_message(&e.to_string()));
    match e.kind() {
        ErrorKind::NotFound => NOT_FOUND,
        ErrorKind::PermissionDenied => PERMISSION_DENIED,
        ErrorKind::AlreadyExists => ALREADY_EXISTS,
        ErrorKind::InvalidData => INVALID_DATA,
        ErrorKind::Interrupted => INTERRUPTED,
        ErrorKind::TimedOut => TIMED_OUT,
        ErrorKind::ConnectionRefused => CONNECTION_REFUSED,
        _ => OTHER,
    }
}

/// Read `path` as UTF-8. Returns a status; on `OK` the contents are waiting in
/// `nova_rt_fs_take_string`.
///
/// Non-UTF-8 contents are `INVALID_DATA`: `std::fs::read_to_string` reports
/// `ErrorKind::InvalidData` for them, which is exactly the case that motivated
/// adding that kind (see docs/adr/0011-io-error-kinds.md).
///
/// # Safety
/// `path` must point to a live `NovaStr`.
#[no_mangle]
pub unsafe extern "C" fn nova_rt_fs_read_to_string(path: *const NovaStr) -> i64 {
    // SAFETY: forwarding this function's own contract.
    let p = unsafe { crate::as_str(path) };
    match std::fs::read_to_string(p) {
        Ok(s) => {
            stash(&STRING_SLOT, gc_message(&s));
            OK
        }
        Err(e) => fail(&e),
    }
}

/// Write `content` to `path`, truncating an existing file.
///
/// # Safety
/// Both arguments must point to live `NovaStr`s.
#[no_mangle]
pub unsafe extern "C" fn nova_rt_fs_write_string(
    path: *const NovaStr,
    content: *const NovaStr,
) -> i64 {
    // SAFETY: forwarding this function's own contract.
    let (p, c) = unsafe { (crate::as_str(path), crate::as_str(content)) };
    match std::fs::write(p, c) {
        Ok(()) => OK,
        Err(e) => fail(&e),
    }
}

/// Take the pending payload string. Returns an empty string if nothing is
/// pending, which a correct wrapper never asks for.
#[no_mangle]
pub extern "C" fn nova_rt_fs_take_string() -> *mut NovaStr {
    match take(&STRING_SLOT) {
        0 => gc_message(""),
        p => p as *mut NovaStr,
    }
}

/// Take the pending error message.
#[no_mangle]
pub extern "C" fn nova_rt_fs_last_error_message() -> *mut NovaStr {
    match take(&MESSAGE_SLOT) {
        0 => gc_message(""),
        p => p as *mut NovaStr,
    }
}

/// The OS temporary-directory path, per `std::env::temp_dir()`.
///
/// `to_string_lossy`, not a fallible conversion: a Nova `String` is UTF-8
/// while a Windows path is UTF-16, so a path containing an unpaired
/// surrogate cannot round-trip exactly. There is no `IoError` to report that
/// through here -- nothing failed at the OS level, the path is simply
/// unrepresentable exactly -- and this path is exotic enough in practice
/// that a usable (if occasionally imprecise) string beats an operation with
/// no way to signal the problem.
#[no_mangle]
pub extern "C" fn nova_rt_fs_temp_dir() -> *mut NovaStr {
    gc_message(&std::env::temp_dir().to_string_lossy())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Pins the spec's "these `async fn`s never suspend" property.
    ///
    /// **Decided before execution.** The spec asks for a test that `PARKED` is
    /// empty after a `std/fs` call, but `PARKED` is private to `crate::task` and
    /// no accessor exists, so the property is pinned at its source instead:
    /// nothing in this module may register a park. That is strictly what
    /// matters — a `std/fs` intrinsic can only reach the park set by calling
    /// `stage_park`.
    ///
    /// Self-referential, like `every_rt_func_is_declared_with_its_real_signature`
    /// elsewhere in this workspace, and carrying the same weakness: it checks
    /// the text of this file, not the behaviour of a running program. It would
    /// miss parking reached by some route other than a literal call. Accepted
    /// because the alternative is test-only surface in a module this branch
    /// does not otherwise touch.
    #[test]
    fn no_filesystem_intrinsic_registers_a_park() {
        let source = include_str!("fs.rs");
        // Skip the whole `#[cfg(test)] mod tests` block, not just this
        // function's body: this test's *own* doc comment above names
        // `stage_park` in prose, and splitting on the function name alone
        // would leave that doc comment in the scanned half, failing even
        // when no real intrinsic parks. Splitting at the test module's own
        // marker instead excludes every test (and its comments) while still
        // scanning all of this file's actual production code.
        let code = source.split("#[cfg(test)]").next().unwrap_or(source);
        assert!(
            !code.contains("stage_park"),
            "a std/fs intrinsic must not park: these async fns run synchronously \
             inside the poll, and parking here without the poller work would \
             suspend a task nothing can wake"
        );
    }

    /// `stash` registers exactly one root for its pointer, and `take` releases
    /// exactly that root before returning it — the pairing this module's
    /// whole safety claim rests on, since a collection between the two is
    /// what the payload must survive.
    ///
    /// **Decided during verification, deviating from the plan.** The plan's
    /// version of this test allocated 2000 "churn" strings after stashing and
    /// then asserted the payload's bytes survived, relying on a real,
    /// conservative-scan-triggered collection to actually run. Applying the
    /// mutation this test exists to catch (deleting the `gc::add_root` call
    /// in `stash`) and re-running proved that version does not discriminate:
    /// it kept passing even under `NOVA_GC_STRESS=1` (a collection on every
    /// allocation) and even with the stash moved into its own
    /// `#[inline(never)]` frame. That is this workspace's own documented
    /// failure mode — `docs/adr/0010-conservative-scan-root-test-gating.md`
    /// records the conservative stack scanner intermittently retaining an
    /// object via a stale word in an already-returned frame, standing in for
    /// the registration a test like this means to check — which is exactly
    /// why the 8 tests that ran into it are `#[ignore]`d rather than fixed by
    /// raising a loop count further. `gc::root_count` sidesteps the scanner
    /// entirely by reading the root registry directly, deterministically and
    /// on every platform; `task.rs`'s
    /// `a_completed_tasks_state_stays_rooted_until_its_output_is_taken` pins
    /// the identical kind of add_root/remove_root pairing the same way, for
    /// the same reason.
    #[test]
    fn a_stashed_string_is_rooted_until_it_is_taken() {
        let p = gc_message("payload");
        let addr = p as usize;
        stash(&STRING_SLOT, p);
        assert_eq!(
            gc::root_count(addr),
            1,
            "stash must register exactly one root for its pointer, or a \
             collection between stash and take could free the payload"
        );

        let taken = nova_rt_fs_take_string();
        assert_eq!(taken as usize, addr, "take must return the stashed pointer");
        assert_eq!(
            gc::root_count(addr),
            0,
            "take must release exactly the one root stash registered, or \
             add_root/remove_root drift out of balance"
        );
        // SAFETY: `taken`'s root was only just released, one statement
        // above, and nothing has allocated since -- so despite no longer
        // being explicitly rooted, it has not yet been swept.
        assert_eq!(unsafe { crate::as_str(taken) }, "payload");
    }
}
