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
//! # Why `SLOTS` is a `RefCell` guarded with `try_borrow_mut`, not `borrow_mut`
//!
//! **Every function here is called from inside an `async fn`'s generated
//! `$poll`, which has no landing pads** (see `nova_mir::async_lower`'s module
//! doc comment), so nothing here may panic. `RefCell::borrow_mut` panics on an
//! already-outstanding borrow; `try_borrow_mut` cannot, so `with_slot` uses
//! that instead and falls back to `abort_with` -- which terminates without
//! unwinding -- on the contended case. Each slot still holds nothing more
//! than a raw already-allocated pointer as a `usize`, with `0` meaning empty;
//! the per-task table just adds one layer of indirection to reach it, keyed
//! by `slot_index`.
//!
//! `stash` itself is `pub(crate)`: `crates/nova-runtime/src/io.rs` is a
//! second consumer, stashing the three standard streams' payloads and write
//! byte counts through this identical slot table rather than a second one.
//! `take` stays private (final review, M4) -- `io.rs`'s own production code
//! only ever stashes, and its tests reach `take` through the test-only
//! `take_for_test` instead of `take` itself being widened for their sake.
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
use std::cell::RefCell;

/// Status codes. `0` is success; every other value is an `IoErrorKind`.
///
/// **This numbering is one half of a wire contract.** The other half is
/// `io_error_kind_of` in `std/io/lib.nova`. The two are independent copies.
///
/// **MEASURED, not "pinned together by a fixture per kind" for all eight —
/// that claim was false and this replaces it (final review, I1).** The Nova
/// half (`io_error_kind_of`'s own arms) is pinned per-code by
/// `fs_io_types.nova`. This Rust half, the one that produces a code from a
/// real `std::io::ErrorKind` below, is pinned by a fixture that provokes the
/// real OS condition for only four of the eight: `NotFound`, `AlreadyExists`,
/// `InvalidData`, and `PermissionDenied` on Windows only.
///
/// **Corrected 2026-08-14 (branch `file-open-openoptions`): naming one
/// fixture per kind, as this comment used to, went stale for three of those
/// four the moment `file.rs`'s `open` began producing this same numbering
/// from a second Rust source.** Per kind, now:
/// - `NotFound`: `fs_not_found.nova` (`read_to_string` on a missing path),
///   and independently `file_errors.nova` (`open` on a path under a missing
///   parent directory).
/// - `AlreadyExists`: `fs_already_exists.nova` (`create_dir` on an existing
///   path, twice), and independently `file_errors.nova` (`open` with
///   `create_new` on an existing path).
/// - `InvalidData`: `fs_invalid_data.nova` only — `File::read` hands back
///   `Bytes` and never decodes UTF-8, so nothing about `File` can provoke
///   this one.
/// - `PermissionDenied`, Windows only: `fs_permission_denied.nova`
///   (`read_to_string` on a directory, `#[cfg(windows)]`), and independently
///   `file_open_dir.nova` (`open` on a directory, `#[cfg(windows)]`).
///
/// `Interrupted`, `TimedOut` and `ConnectionRefused` are not reachable from
/// `std/fs`'s `async fn`s at all, portably or otherwise, so no fixture can pin
/// them. **Reasoned, not measured, for `read`/`write` (`byte-type` branch):**
/// both route through this same `fail` function exactly as
/// `read_to_string`/`write_string` do, over `std::fs::read`/`std::fs::write`
/// rather than `std::fs::read_to_string`/`std::fs::write` — reading or
/// writing raw bytes instead of UTF-8 text opens no path to a network- or
/// signal-flavoured `ErrorKind` that plain-text local I/O did not already
/// have, so the same unreachability is expected to hold, not separately
/// provoked. The `_ => OTHER` fallback *is* reachable (e.g. `read_dir` on a
/// plain file returns it, still unexercised).
///
/// **Corrected 2026-08-14: it is no longer true that nothing exercises this
/// fallback.** `file.rs`'s closed/stale/forged-fd case (`closed_fd_error`)
/// fabricates a `std::io::Error` of `ErrorKind::Other` on every table miss,
/// unconditionally, which falls to this identical `_` arm — and
/// `file_lifetime.nova`'s "read after close" check asserts on the exact
/// message that arm stashes. See `docs/adr/0011-io-error-kinds.md`, which
/// states this split correctly as of the same correction.
///
/// **Corrected 2026-08-16 (branch `io-poller-std-net`): the "not reachable ...
/// so no fixture can pin them" paragraph above is now stale for two of its
/// three kinds, and the per-kind breakdown two paragraphs up goes four
/// fixture-pinned kinds to six.** `TIMED_OUT` and `CONNECTION_REFUSED` have
/// carried this numbering with no producer since increment 1, because no
/// filesystem operation can produce either — `crates/nova-runtime/src/net.rs`'s
/// two-phase `connect` and its `read_timeout` future are their first. A
/// `connect` to a loopback port nothing is listening on is refused either
/// synchronously (an immediate `ECONNREFUSED`/`WSAECONNREFUSED`, routine for
/// a closed loopback port) or via the second poll's `SO_ERROR` check, both
/// paths through this identical `fail` (`net.rs`'s own module doc comment,
/// "`CONNECTION_REFUSED` gets its first producer"), pinned end to end by
/// `tests/runtime/net_refused.nova` (`net_refused_run`,
/// `crates/nova-cli/tests/run_tests.rs`). `read_timeout` reports `TIMED_OUT`
/// when its deadline passes with a read still would-blocking, pinned by
/// `tests/runtime/net_timeout.nova` (`net_timeout_run`), which also pins the
/// opposite direction in the same fixture — a deadline comfortably longer
/// than the peer's own delay must still succeed with the real data, not fire
/// merely because a deadline is in play at all. Six of the eight status
/// codes are therefore fixture-pinned now: the original four two paragraphs
/// up, plus these two. Only `INTERRUPTED` remains genuinely unreachable from
/// either module's `async fn`s, portably or otherwise — the "reasoned, not
/// measured" argument above, made for all three together, now supports
/// `INTERRUPTED` alone.
pub const OK: i64 = 0;
pub const NOT_FOUND: i64 = 1;
pub const PERMISSION_DENIED: i64 = 2;
pub const ALREADY_EXISTS: i64 = 3;
pub const INVALID_DATA: i64 = 4;
pub const INTERRUPTED: i64 = 5;
pub const TIMED_OUT: i64 = 6;
pub const CONNECTION_REFUSED: i64 = 7;
pub const OTHER: i64 = 8;

/// Which payload a `stash`/`take` pair is addressing.
///
/// Replaces the three separate `thread_local!` slots this module used before
/// per-task storage. `Buffer` serves `String` *and* `Bytes` payloads, not one
/// variant each: the two have the identical `{len, ptr}` representation (see
/// `crate::bytes`'s module doc comment), and which one a payload is gets
/// carried entirely by which builtin stashed it and which one reads it back
/// (`nova_rt_fs_read_to_string`/`nova_rt_fs_take_string` treat it as UTF-8;
/// `nova_rt_fs_read`/`nova_rt_fs_take_bytes` do not interpret it at all). A
/// dedicated `Bytes` slot would be a fourth copy of storage for a distinction
/// already carried by which builtin stashed it — exactly the kind of
/// avoidable debt this module's boundary design exists to prevent.
///
/// `pub(crate)`, not private: the [`release_task_slots`] test that seeds all
/// three kinds for one task id lives in `task.rs`, alongside the two
/// Buffer-only ones it completes, and needs to name a variant to call
/// [`stash_for_test`] with (final review, I2). A placement choice, not a
/// forced one -- the same coverage could instead have been written directly
/// in this module's own tests, which already call [`stash`] without needing
/// `Slot` visible outside it at all.
#[derive(Copy, Clone)]
pub(crate) enum Slot {
    Buffer,
    Array,
    Message,
}

/// One task's three payload slots. Each field is a GC-rooted pointer, or 0.
#[derive(Clone, Copy, Default)]
struct Slots {
    buffer: usize,
    array: usize,
    message: usize,
}

thread_local! {
    /// Payload slots, per task, indexed by `slot_index`.
    ///
    /// `thread_local!` for the reason `task.rs`'s module doc gives for `TASKS`
    /// and `QUEUE`: the GC's root table is per-thread, so a second thread
    /// running Nova code would free objects the first still holds.
    static SLOTS: RefCell<Vec<Slots>> = const { RefCell::new(Vec::new()) };
}

/// Allocate a GC-managed string for a slot payload.
///
/// A thin name for [`crate::gc_str`], reused rather than reproducing
/// `NovaStr { len, ptr }`'s layout here a second time — precisely the drift
/// class this module's boundary design exists to avoid.
fn gc_message(s: &str) -> *mut NovaStr {
    crate::gc_str(s)
}

/// Convert a task id into its `SLOTS` index. Shared by [`slot_index`] and
/// [`release_task_slots`] so the `+ 1` offset is written in exactly one place
/// (final review, M3 — it was previously duplicated between them).
///
/// `id + 1`, so index **0 stays the reserved no-task key** and is never a
/// real task's own index. Both of today's callers already validate `id`
/// against `TASKS` before reaching here (`task.rs`'s `release_internal`
/// aborts and `take_output_internal` panics on an id that was never
/// registered), so a negative `id` cannot reach this function today. Guarded
/// anyway, for a future caller that might: `abort_with`, not a check that
/// could panic, since this can run inside a generated poll boundary with no
/// landing pads. Without the guard, `id as usize + 1` on a negative `id`
/// is an overflow -- a debug-only panic, and a silent wrap onto the
/// no-task index in release -- rather than a clean rejection.
fn slot_index_for(id: i64) -> usize {
    let Ok(id) = usize::try_from(id) else {
        crate::task::abort_with("nova_rt_fs: task id must not be negative")
    };
    id + 1
}

/// The `SLOTS` index for the current task.
///
/// `id + 1`, so index **0 is the reserved no-task key**. Task ids are dense
/// from zero (`poll_one` reads `tasks.get(id as usize)`), so a `Vec` gives O(1)
/// access with no hashing and no per-call allocation.
fn slot_index() -> usize {
    crate::task::current_task().map_or(0, slot_index_for)
}

/// Run `f` against one field of the current task's slots, growing the table if
/// this task has not been seen before.
///
/// **Panic-free by construction, because this runs inside a generated poll
/// boundary with no landing pads.** `try_borrow_mut` rather than `borrow_mut`,
/// and `get_mut` rather than `[i]`; both fall back to `abort_with`, which
/// terminates without unwinding. The `get_mut` arm is unreachable — the resize
/// immediately above guarantees `i < len` — and exists so the impossible case
/// still cannot unwind.
fn with_slot<R>(slot: Slot, f: impl FnOnce(&mut usize) -> R) -> R {
    SLOTS.with(|cell| {
        let Ok(mut slots) = cell.try_borrow_mut() else {
            crate::task::abort_with("nova_rt_fs: payload slot table is already borrowed")
        };
        let i = slot_index();
        if slots.len() <= i {
            slots.resize(i + 1, Slots::default());
        }
        let Some(entry) = slots.get_mut(i) else {
            crate::task::abort_with("nova_rt_fs: payload slot index out of range after resize")
        };
        let field = match slot {
            Slot::Buffer => &mut entry.buffer,
            Slot::Array => &mut entry.array,
            Slot::Message => &mut entry.message,
        };
        f(field)
    })
}

/// Register `ptr` as a GC root, then publish it in `slot`.
///
/// Rooting before publishing means a scan running between these two
/// statements still finds the object through the root table — there is no
/// instant where it is reachable by neither the slot nor a root. This now
/// holds for the overwrite case too: releasing whatever `slot` already held
/// happens first, via the same `take` a normal read uses, so a displaced
/// pointer's root is never left dangling.
///
/// **Releases the slot's current occupant first (final review, I2).**
/// Without this, stashing into an already-occupied slot would `add_root` the
/// new pointer while leaving the displaced one's root registered forever —
/// `take` is the only place that releases a *slot's* root (`stash_array`'s own
/// `remove_root` call is a separate, self-contained build-time balance,
/// finished before its block ever reaches a slot), and `take` can only see
/// whichever pointer is *currently* published, so the one just overwritten
/// would never be found and released. Not reachable through today's
/// `std/fs` surface (every wrapper drains a slot before the next `stash`
/// targeting it — see `a_stashed_string_is_rooted_until_it_is_taken`'s doc
/// comment for why that is not itself enough to catch this), so this was
/// latent rather than observed in any fixture; measured with a probe
/// identical in shape to
/// `stash_overwriting_an_occupied_slot_does_not_leak_the_displaced_root`
/// below, minus this fix. `take`'s own contract (return `0` for an empty
/// slot, otherwise the released pointer) makes discarding its result safe
/// here: there is nothing to free by hand either way.
///
/// `pub(crate)`, not private: `crate::io` is a second consumer, stashing the
/// standard streams' read payloads and write byte counts through this same
/// function rather than reproducing its root-balancing logic a second time.
pub(crate) fn stash(slot: Slot, ptr: *mut NovaStr) {
    take(slot);
    gc::add_root(ptr as *mut u8);
    with_slot(slot, |field| *field = ptr as usize);
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
///
/// **The `gc::remove_root` deliberately happens after the borrow is dropped**,
/// not inside `with_slot`'s closure. Holding a `RefCell` borrow across a call
/// into the collector would be a re-entrancy hazard for no benefit; the
/// pointer is already out of the table and owned by this frame by then.
///
/// Private, not `pub(crate)` (final review, M4): every production caller is
/// in this module -- `stash` below and the `nova_rt_fs_take_*` intrinsics --
/// and `crate::io`'s production code only ever stashes (its payloads are
/// meant for a future Nova caller to take via `nova_rt_fs_take_bytes`, not
/// for `io.rs` itself to take back out). `crate::io`'s own *tests* are the
/// one cross-module consumer, verifying what a stash left behind the same
/// way this function's own tests here do; they reach this through
/// [`take_for_test`] below instead of this function being widened just for
/// them. `take` carries GC-root semantics (`remove_root`), so a
/// broader-than-needed path to it is worth the extra indirection.
fn take(slot: Slot) -> usize {
    let ptr = with_slot(slot, |field| {
        let ptr = *field;
        *field = 0;
        ptr
    });
    if ptr != 0 {
        gc::remove_root(ptr as *mut u8);
    }
    ptr
}

/// Test-only cross-module accessor for [`take`], for `crate::io`'s own tests
/// -- the same reason [`stash_for_test`] exists for [`stash`], and a
/// narrower pattern than widening `take` itself would have been: every
/// *production* call to [`take`] is inside this module, so it never needed
/// more than module-private visibility.
#[cfg(test)]
pub(crate) fn take_for_test(slot: Slot) -> usize {
    take(slot)
}

/// Release every payload root task `id` still holds, and clear its slots.
///
/// Called from `crate::task::release_internal` and `crate::task::take_output_internal` —
/// the two places that release a task's *state* root. Keyed on `id` rather
/// than [`slot_index`] precisely because those call sites do not run with
/// `CURRENT` set to the task being released.
///
/// Idempotent, and silent for a task that never stashed anything: a task id
/// past the end of the table simply has no slots.
///
/// Roots are released after the borrow is dropped, for the same reason
/// [`take`] does it that way.
pub(crate) fn release_task_slots(id: i64) {
    let held = SLOTS.with(|cell| {
        let Ok(mut slots) = cell.try_borrow_mut() else {
            crate::task::abort_with("nova_rt_fs: payload slot table is already borrowed")
        };
        let index = slot_index_for(id);
        match slots.get_mut(index) {
            Some(entry) => {
                let held = [entry.buffer, entry.array, entry.message];
                *entry = Slots::default();
                held
            }
            None => [0, 0, 0],
        }
    });
    for ptr in held {
        if ptr != 0 {
            gc::remove_root(ptr as *mut u8);
        }
    }
}

/// Test-only: stash `ptr` into task `id`'s `slot`, without requiring
/// `CURRENT` to already equal `id`.
///
/// `task.rs`'s own test module cannot reach `SLOTS` directly -- it is
/// private to this module -- and even now that [`stash`] itself is
/// `pub(crate)`, calling it directly still only ever targets *the current
/// task's* slot (through `slot_index`), never an arbitrary one. This
/// function needs a way to seed a payload for a task it registers through
/// its own internals (`spawn_internal`/`nova_rt_task_spawn`) without
/// driving a real poll, so it can call
/// `release_internal`/`take_output_internal` directly and check whether
/// either one actually drops the payload's root. Delegates
/// to [`stash`] under a temporary [`crate::task::set_current_for_test`]
/// override, restoring whatever `CURRENT` held beforehand rather than
/// assuming it was `None`.
///
/// **Takes `slot` rather than hardcoding [`Slot::Buffer`] (final review,
/// I2).** `release_task_slots` has three fields to release, and until this
/// change every caller of this function seeded only `Buffer` -- widened
/// only as far as pinning all three needed, not into a general-purpose
/// helper.
///
/// `#[cfg(test)]` only, not also `#[cfg(windows)]`: every caller of this
/// function is an ordinary, cross-platform `task.rs` test, the same
/// reasoning `set_current_for_test`'s own doc comment gives for itself --
/// contrast `gc::collect_for_test`, whose *only* callers genuinely are
/// `#[cfg(windows)]`, which is why that one carries the platform gate and
/// this one must not.
#[cfg(test)]
pub(crate) fn stash_for_test(id: i64, slot: Slot, ptr: *mut NovaStr) {
    let previous = crate::task::current_task();
    crate::task::set_current_for_test(Some(id));
    stash(slot, ptr);
    crate::task::set_current_for_test(previous);
}

/// Map an `std::io::Error` to its status code, and stash its message.
///
/// Called on every failure path, so the message is always available to the
/// wrapper that is about to build an `IoError`.
///
/// `pub(crate)`, not private: `crate::io` is a second consumer, mapping the
/// standard streams' I/O errors through this same function rather than a
/// second copy of the kind-to-status table.
pub(crate) fn fail(e: &std::io::Error) -> i64 {
    use std::io::ErrorKind;
    stash(Slot::Message, gc_message(&e.to_string()));
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

/// Build a Nova `[String]` from `names` and stash it, GC-rooted.
///
/// **Reproduces the layout codegen emits for an array**: word 0 is the element
/// count, elements follow at `8 + 8*i`, allocated scanned so the collector
/// traces the `NovaStr` pointers inside. Getting this wrong is a silent
/// miscompile rather than a failure, which is why `mod tests` asserts the
/// tracked `(size, scan)` through `gc::object_info` and not merely the values
/// read back — the same discipline `nova_rt_str_chars` carries.
///
/// Each element is rooted while it is being built, because the allocation of a
/// later element can collect and an earlier one is then named only by a block
/// that is itself not yet reachable from anywhere the collector scans.
///
/// **This is rooting the block, not each element — verified against `gc.rs`'s
/// actual contract rather than assumed.** `block` is allocated with
/// `scan = true` (the same flag `nova_rt_str_chars`'s array block uses), and
/// `collect_with_roots`'s mark phase does not stop at a rooted object: every
/// object it marks — reached directly as a root or transitively — is then
/// traced if `scan` is true, meaning its words are read back and each one is
/// itself passed through the identical range-based `mark_word` check a root
/// gets. So once `block` is a root, every `NovaStr` pointer already written
/// into its element words is found by that trace and kept alive too, the same
/// way a rooted record keeps its fields alive without each field needing its
/// own root. If `add_root`ing a block did NOT cause its contents to be
/// traced, this function would need to root every element individually
/// instead; it does not, because the property above holds.
fn stash_array(names: &[String]) {
    let n = names.len();
    let block = gc::alloc(8 + 8 * n, true);
    gc::add_root(block);
    let words = block as *mut i64;
    // SAFETY: `block` has `8 + 8*n` writable bytes, so word 0 and words
    // `1..=n` are all in bounds.
    unsafe { *words = n as i64 };
    for (i, name) in names.iter().enumerate() {
        let s = gc_message(name);
        // SAFETY: same block; `i < n`.
        unsafe { *words.add(1 + i) = s as i64 };
    }
    // The block is already rooted by `stash` below, so drop the build-time root
    // to keep `add_root`/`remove_root` balanced.
    gc::remove_root(block);
    stash(Slot::Array, block as *mut NovaStr);
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
            stash(Slot::Buffer, gc_message(&s));
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
    match take(Slot::Buffer) {
        0 => gc_message(""),
        p => p as *mut NovaStr,
    }
}

/// Read `path`'s bytes. Returns a status; on `OK` the contents are waiting in
/// `nova_rt_fs_take_bytes`.
///
/// Unlike `nova_rt_fs_read_to_string` this cannot produce `INVALID_DATA`: there
/// is no encoding to violate. Any status other than `OK` is a real I/O failure.
///
/// # Safety
/// `path` must point to a live `NovaStr`.
#[no_mangle]
pub unsafe extern "C" fn nova_rt_fs_read(path: *const NovaStr) -> i64 {
    // SAFETY: forwarding this function's own contract.
    let p = unsafe { crate::as_str(path) };
    match std::fs::read(p) {
        Ok(bytes) => {
            stash(Slot::Buffer, crate::bytes::gc_bytes(&bytes));
            OK
        }
        Err(e) => fail(&e),
    }
}

/// Write `content`'s bytes to `path`, truncating an existing file.
///
/// # Safety
/// Both arguments must point to live `NovaStr`s.
#[no_mangle]
pub unsafe extern "C" fn nova_rt_fs_write(path: *const NovaStr, content: *const NovaStr) -> i64 {
    // SAFETY: forwarding this function's own contract.
    let (p, c) = unsafe { (crate::as_str(path), crate::bytes::as_bytes(content)) };
    match std::fs::write(p, c) {
        Ok(()) => OK,
        Err(e) => fail(&e),
    }
}

/// Take the pending payload as `Bytes`. Returns an empty `Bytes` if nothing is
/// pending, which a correct wrapper never asks for.
///
/// Mirrors [`nova_rt_fs_take_string`], reading the same
/// [`Slot::Buffer`] -- the payload left there by a successful
/// [`nova_rt_fs_read`], never interpreted as UTF-8.
///
/// **The `fs_` prefix is now historical.** Since increment 3a this reads a
/// general per-task slot, not an fs-specific one, and `crate::io`'s stream
/// intrinsics stash exactly this shape of payload here too -- both a read
/// buffer and, for a write, its byte count encoded as bytes.
/// `std/io/lib.nova`'s `read_stdin`, `write_stdout` and `write_stderr` are
/// that outside-`std/fs` caller now, collecting through this same taker
/// rather than a second one.
#[no_mangle]
pub extern "C" fn nova_rt_fs_take_bytes() -> *mut NovaStr {
    match take(Slot::Buffer) {
        0 => crate::bytes::gc_bytes(&[]),
        p => p as *mut NovaStr,
    }
}

/// Take the pending error message.
///
/// **The `fs_` prefix is now historical**, for the same reason
/// [`nova_rt_fs_take_bytes`]'s doc comment gives: [`fail`] stashes into the
/// same per-task [`Slot::Message`] regardless of which module's intrinsic
/// called it. Every fallible wrapper in `std/io/lib.nova` --
/// `read_stdin`, `write_stdout`, `write_stderr`, `flush_stdout` and
/// `flush_stderr` -- reads its own error message back through this same
/// taker now, alongside `std/fs`'s own wrappers.
#[no_mangle]
pub extern "C" fn nova_rt_fs_last_error_message() -> *mut NovaStr {
    match take(Slot::Message) {
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

/// Whether `path` exists. Cannot distinguish absent from unreadable, because
/// `nova-spec/20-STDLIB.md` §5 gives `exists` a `Bool` return rather than a
/// `Result`; that is the spec's choice and is left alone.
///
/// # Safety
/// `path` must point to a live `NovaStr`.
#[no_mangle]
pub unsafe extern "C" fn nova_rt_fs_exists(path: *const NovaStr) -> i8 {
    // SAFETY: forwarding this function's own contract.
    let p = unsafe { crate::as_str(path) };
    i8::from(std::path::Path::new(p).exists())
}

/// Create the directory `path`. Its parent must exist; an existing `path` is
/// `ALREADY_EXISTS`.
///
/// # Safety
/// `path` must point to a live `NovaStr`.
#[no_mangle]
pub unsafe extern "C" fn nova_rt_fs_create_dir(path: *const NovaStr) -> i64 {
    // SAFETY: forwarding this function's own contract.
    let p = unsafe { crate::as_str(path) };
    match std::fs::create_dir(p) {
        Ok(()) => OK,
        Err(e) => fail(&e),
    }
}

/// Create the directory `path`, and any missing parent directories along the
/// way. Unlike `create_dir`, an existing directory at `path` is success, not
/// `ALREADY_EXISTS` -- `std::fs::create_dir_all`'s own behaviour, carried
/// through unchanged rather than papered over.
///
/// # Safety
/// `path` must point to a live `NovaStr`.
#[no_mangle]
pub unsafe extern "C" fn nova_rt_fs_create_dir_all(path: *const NovaStr) -> i64 {
    // SAFETY: forwarding this function's own contract.
    let p = unsafe { crate::as_str(path) };
    match std::fs::create_dir_all(p) {
        Ok(()) => OK,
        Err(e) => fail(&e),
    }
}

/// Remove the file `path`.
///
/// # Safety
/// `path` must point to a live `NovaStr`.
#[no_mangle]
pub unsafe extern "C" fn nova_rt_fs_remove_file(path: *const NovaStr) -> i64 {
    // SAFETY: forwarding this function's own contract.
    let p = unsafe { crate::as_str(path) };
    match std::fs::remove_file(p) {
        Ok(()) => OK,
        Err(e) => fail(&e),
    }
}

/// Remove the directory `path` and everything under it.
///
/// # Safety
/// `path` must point to a live `NovaStr`.
#[no_mangle]
pub unsafe extern "C" fn nova_rt_fs_remove_dir_all(path: *const NovaStr) -> i64 {
    // SAFETY: forwarding this function's own contract.
    let p = unsafe { crate::as_str(path) };
    match std::fs::remove_dir_all(p) {
        Ok(()) => OK,
        Err(e) => fail(&e),
    }
}

/// List `path`'s entry names, sorted. On `OK` the names are waiting in
/// `nova_rt_fs_take_string_array`.
///
/// **Sorted in the runtime deliberately.** Directory order is unspecified by
/// every OS, so unsorted output would make each fixture platform-dependent.
///
/// # Safety
/// `path` must point to a live `NovaStr`.
#[no_mangle]
pub unsafe extern "C" fn nova_rt_fs_read_dir(path: *const NovaStr) -> i64 {
    // SAFETY: forwarding this function's own contract.
    let p = unsafe { crate::as_str(path) };
    let iter = match std::fs::read_dir(p) {
        Ok(it) => it,
        Err(e) => return fail(&e),
    };
    let mut names: Vec<String> = Vec::new();
    for entry in iter {
        match entry {
            Ok(e) => names.push(e.file_name().to_string_lossy().into_owned()),
            Err(e) => return fail(&e),
        }
    }
    names.sort();
    stash_array(&names);
    OK
}

/// Take the array payload staged by a successful `nova_rt_fs_read_dir`.
///
/// Mirrors [`nova_rt_fs_take_string`], with the one difference forced by there
/// being no such thing as a null Nova array: an empty slot (`0`) yields a
/// fresh, well-formed zero-length array here -- the same `{ len: 0 }` shape
/// `nova_rt_str_chars` produces for an empty string -- rather than a null
/// pointer, which a correct wrapper would then have to null-check for no
/// reason (a correct wrapper never asks for this when nothing is pending,
/// exactly as `nova_rt_fs_take_string`'s own doc comment notes).
#[no_mangle]
pub extern "C" fn nova_rt_fs_take_string_array() -> *mut u8 {
    match take(Slot::Array) {
        0 => {
            let block = gc::alloc(8, true);
            let words = block as *mut i64;
            // SAFETY: `block` is 8 bytes, so word 0 is in bounds.
            unsafe { *words = 0 };
            block
        }
        p => p as *mut u8,
    }
}

/// What `path` is: 0 = metadata unavailable (absent, or unreadable), 1 file,
/// 2 directory.
///
/// One call rather than separate `is_file`/`is_dir` intrinsics, so a `DirEntry`
/// costs one syscall instead of two and the two answers cannot disagree.
///
/// **The convention for an entry that is neither, stated here because the code
/// below does not spell it out: it is classified `1` (file).** The check is
/// `is_dir()`, with everything else that exists falling to the `else` arm, so
/// a FIFO, Unix domain socket, or device node -- a filesystem entry that is
/// genuinely neither a plain file nor a directory -- is `1`, not some third
/// code. This is a stated convention, not a measurement: nothing in this
/// codebase constructs such an entry to observe it against, and on Windows
/// the class is structurally unreachable through this function's `metadata`
/// call. `tests/runtime/fs_read_dir.nova`'s fixture, the one test that drives
/// this function, contains only a plain file and a subdirectory -- it pins
/// the two-way split, and says nothing about the third case, which is why
/// this paragraph exists instead of a test.
///
/// # Safety
/// `path` must point to a live `NovaStr`.
#[no_mangle]
pub unsafe extern "C" fn nova_rt_fs_kind(path: *const NovaStr) -> i64 {
    // SAFETY: forwarding this function's own contract.
    let p = unsafe { crate::as_str(path) };
    let meta = match std::fs::metadata(p) {
        Ok(m) => m,
        Err(_) => return 0,
    };
    if meta.is_dir() {
        2
    } else {
        1
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// **Measured, not assumed -- on Windows only.** The design spec's note
    /// on `remove_dir_all` flags Windows-versus-POSIX divergence for a
    /// read-only entry as an expectation, not something anyone had actually
    /// run. Measured here, against `nova_rt_fs_remove_dir_all` itself rather
    /// than bare `std::fs`: on Windows, a read-only file nested inside the
    /// tree does not stop the removal -- the status is `OK` and the
    /// directory is gone afterward, the same as for an ordinary file.
    /// (Windows' own `DeleteFile` denies access to a read-only attribute;
    /// whatever compensates -- clearing the attribute first, or a different
    /// removal path entirely -- lives in the standard library this function
    /// calls straight into, not in this crate, so it is not this crate's to
    /// document further than "observed".)
    ///
    /// **`#[cfg(windows)]` because Windows is the only platform this was
    /// actually run on.** `.github/workflows/ci.yml` runs ubuntu, windows and
    /// macos, so leaving this ungated would assert a POSIX result nobody
    /// measured. There is a plausible reason POSIX would behave the same --
    /// unlinking a directory entry is governed by the *directory's* write
    /// permission, not the target file's own mode bits, so a read-only file
    /// should not by itself block removal -- but that is exactly a plausible
    /// reason, not a measurement, and this project keeps getting burned by
    /// treating the two as equivalent. Recorded here as the *expectation*
    /// for POSIX, explicitly labelled as unmeasured -- the same "observed,
    /// not explained" treatment the Windows mechanism gets above. See the
    /// task report this landed with for the transcript this was first
    /// observed in.
    #[cfg(windows)]
    #[test]
    fn remove_dir_all_is_not_stopped_by_a_read_only_entry() {
        let dir =
            std::env::temp_dir().join(format!("nova-rt-fs-readonly-probe-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("probe dir");
        let file = dir.join("locked.txt");
        std::fs::write(&file, "x").expect("probe file");
        let mut perms = std::fs::metadata(&file)
            .expect("probe metadata")
            .permissions();
        perms.set_readonly(true);
        std::fs::set_permissions(&file, perms).expect("set readonly");

        let path_str = gc_message(&dir.to_string_lossy());
        // SAFETY: `path_str` was just allocated above and is still live.
        let status = unsafe { nova_rt_fs_remove_dir_all(path_str) };
        let dir_gone_after_removal = !dir.exists();

        // Best-effort cleanup in case the removal above did *not* clear the
        // read-only file (i.e. the assertions below are about to fail): rely
        // on the exact same call this test measures, not a hand-rolled
        // `set_readonly(false)` -- which clippy's
        // `permissions_set_readonly_false` rightly flags as unsafe to do
        // generically, since on Unix it makes the file world-writable rather
        // than merely owner-writable. If plain `remove_dir_all` cannot clear
        // it either, there is nothing more this cleanup should attempt.
        let _ = std::fs::remove_dir_all(&dir);

        assert_eq!(
            status, OK,
            "measured on this platform: a read-only entry did not stop removal"
        );
        assert!(
            dir_gone_after_removal,
            "measured on this platform: the directory was fully removed"
        );
    }

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
    ///
    /// **What actually keeps a split-on-a-literal guard sound, stated once
    /// here for this guard, its sibling below, and `bytes.rs`'s equivalent
    /// (final review, I1, correcting an overclaim this comment used to make):
    /// the property is that the *first* occurrence of the split literal is
    /// the real boundary -- not that the literal is unique in the file.** It
    /// is not unique: this explanation, the sibling guard below, and each
    /// guard's own split call all contain the same text, and that is
    /// harmless only because every occurrence of it comes *after* the real
    /// one, however many there turn out to be. What would not be harmless is
    /// an *earlier* one -- a production doc comment quoting the exact split
    /// text before the real module declaration would silently truncate the
    /// scan right there, and **this class of guard fails open**: it passes
    /// when it covers nothing, with no test failing to say so. The ceiling
    /// this stays honest about, for the same reason: a substring scan cannot
    /// see past a rewrite that reaches equivalent code a different way --
    /// fully-qualified syntax (`RefCell::borrow_mut(&*cell)` instead of
    /// `cell.borrow_mut()`), an aliased import, or indexing spelled other
    /// than literal `[i]`. Not a plausible thing for anyone here to write,
    /// but it is the honest limit of what this class of guard can promise.
    #[test]
    fn no_filesystem_intrinsic_registers_a_park() {
        let source = include_str!("fs.rs");
        // Skip the whole `mod tests` block, not just this function's body:
        // this test's *own* doc comment above names `stage_park` in prose,
        // and splitting on the function name alone would leave that doc
        // comment in the scanned half, failing even when no real intrinsic
        // parks. Splits on the module declaration's own text, `mod tests {`,
        // rather than the bare `#[cfg(test)]` attribute one line above it --
        // `stash_for_test`'s doc comment above quotes that exact attribute
        // text in prose ("`#[cfg(test)]` only, not also..."), an *earlier*
        // occurrence of the same substring than the real attribute, which
        // silently cut this scan short right there instead of at the real
        // module boundary, for as long as that comment has existed (found
        // while writing `no_slot_access_can_panic_on_a_borrow` below, which
        // would have inherited the identical defect from the same pattern).
        // See this function's own doc comment above for the property that
        // actually keeps this sound, now that "unique" has been corrected.
        let code = source.split("mod tests {").next().unwrap_or(source);
        assert!(
            !code.contains("stage_park"),
            "a std/fs intrinsic must not park: these async fns run synchronously \
             inside the poll, and parking here without the poller work would \
             suspend a task nothing can wake"
        );
    }

    /// Every `SLOTS` access in production code is fallible-borrow, never
    /// `borrow_mut`, and this file's production half is also free of the
    /// other panic sources this workspace watches for.
    ///
    /// A `RefCell` borrow panic in this module would cross a generated poll
    /// boundary, where there are no landing pads — the one hazard the
    /// per-task table introduced that the three `Cell`s it replaced could
    /// not have. Pinned at its source rather than by a fixture, so it
    /// covers every future access without a recount.
    ///
    /// **Needle list widened toward `bytes.rs`'s, with one deliberate
    /// omission (final review, M1).** `bytes.rs`'s sibling guard
    /// (`no_bytes_intrinsic_can_panic`) also needles the bare word
    /// `RefCell`, sound there because that module never declares one. This
    /// module's design *is* a `RefCell` -- `SLOTS`, declared a few dozen
    /// lines above, correctly and unavoidably -- so that needle was checked
    /// and would fail immediately here, not merely pass with less room to
    /// regress; it is deliberately left out rather than forgotten.
    /// `unwrap()`, `.expect(`, `panic!` and `format!` were checked the same
    /// way and are genuinely absent from this file's production half today,
    /// so adding them costs nothing now and covers more later. **Indexing
    /// (`[i]`) stays outside what this guard can see** -- a substring scan
    /// cannot distinguish a safe, already-bounds-checked `[i]` from a
    /// dangerous one -- so that half of panic-freedom is held by review, not
    /// mechanically; `CHANGELOG.md`'s entry for this module says so now
    /// instead of overclaiming it.
    ///
    /// Scans only the part of this file before its own `mod tests` block,
    /// for the same reason `no_filesystem_intrinsic_registers_a_park` does
    /// and, since Task 3, in the same corrected way -- see that test's own
    /// doc comment for why the split is on the literal `mod tests {` rather
    /// than the bare `#[cfg(test)]` attribute above it, for the property
    /// that actually keeps the split sound, and for the ceiling on what a
    /// guard shaped like this one can ever promise.
    #[test]
    fn no_slot_access_can_panic_on_a_borrow() {
        let source = include_str!("fs.rs");
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
                "a std/fs intrinsic must not panic: `{needle}` found in this file's \
                 production code, which is reachable from a generated poll boundary \
                 with no landing pad to unwind through"
            );
        }
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
    ///
    /// **Also pins that `take` clears the field, not only that it releases
    /// the root (final review, I3).** Nothing previously read a slot twice or
    /// asserted a drained one reads back empty. Left undone, a second
    /// `stash` into the same slot -- or a `release_task_slots` for the same
    /// task -- would issue a second `gc::remove_root` for an address whose
    /// first registration is already gone, which can cancel a different,
    /// live registration once the collector reuses that address.
    #[test]
    fn a_stashed_string_is_rooted_until_it_is_taken() {
        let p = gc_message("payload");
        let addr = p as usize;
        stash(Slot::Buffer, p);
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
        assert_eq!(take(Slot::Buffer), 0, "take must clear the slot");
    }

    /// `stash` must not leak the root of whatever a slot already held when a
    /// second pointer overwrites it (final review, I2). Fails without the
    /// `take(slot)` line at the top of `stash`: measured by temporarily
    /// deleting it, which reproduces the review's own probe exactly --
    /// `a_root` stayed at `1` after `b` was stashed over it, instead of
    /// dropping to `0`.
    ///
    /// Not reachable through today's `std/fs` wrappers -- every one drains a
    /// slot before the next `stash` targeting it -- so this is a property of
    /// `stash` in isolation, exercised directly rather than through a Nova
    /// fixture, the same way `a_stashed_string_is_rooted_until_it_is_taken`
    /// is.
    #[test]
    fn stash_overwriting_an_occupied_slot_does_not_leak_the_displaced_root() {
        let a = gc_message("first");
        let b = gc_message("second");
        let (a_addr, b_addr) = (a as usize, b as usize);

        stash(Slot::Buffer, a);
        assert_eq!(
            gc::root_count(a_addr),
            1,
            "the first stash roots its pointer"
        );

        stash(Slot::Buffer, b);
        assert_eq!(
            gc::root_count(a_addr),
            0,
            "stashing a second pointer into an occupied slot must release the \
             first one's root -- otherwise it is pinned for the thread's \
             lifetime, since `take` is the only place that releases a slot's \
             root and can only ever see the slot's *current* occupant"
        );
        assert_eq!(
            gc::root_count(b_addr),
            1,
            "the second stash roots its pointer"
        );

        let taken = take(Slot::Buffer);
        assert_eq!(
            taken, b_addr,
            "take returns the most recently stashed pointer"
        );
        assert_eq!(
            gc::root_count(b_addr),
            0,
            "take releases the surviving root"
        );
    }

    /// **Reproduces the exact array layout codegen emits**, the same
    /// discipline `nova_rt_str_chars`'s own layout test (`nova-runtime`'s
    /// `lib.rs`) uses: reading back the words `stash_array` wrote cannot by
    /// itself distinguish a correctly sized block from one that merely has
    /// enough allocator slop past a *wrong* declared size for these three
    /// words to still land in live memory, and it cannot observe the `scan`
    /// flag at all -- that only changes GC behaviour, not what is readable.
    /// Asserting `gc::object_info` alongside the values closes both holes.
    #[test]
    fn a_stashed_array_has_the_layout_the_abi_declares() {
        let names = vec!["a".to_string(), "bb".to_string()];
        stash_array(&names);
        let block = take(Slot::Array);
        assert_eq!(
            gc::object_info(block),
            Some((8 + 8 * 2, true)),
            "an array block is a length word plus one word per element, scanned"
        );
        let words = block as *mut i64;
        // SAFETY: the block is `8 + 8*2` bytes, so these three words are in
        // bounds, and reading it here is safe for the same reason
        // `a_stashed_string_is_rooted_until_it_is_taken` reads its own `taken`
        // after release: nothing has allocated since `take` ran one statement
        // above, so nothing has swept it yet.
        unsafe {
            assert_eq!(*words, 2);
            assert_eq!(crate::as_str(*words.add(1) as *const NovaStr), "a");
            assert_eq!(crate::as_str(*words.add(2) as *const NovaStr), "bb");
        }
    }

    /// Two tasks stashing between one task's stash and its take must not collide.
    ///
    /// This is the defect the per-task table exists to prevent, and it cannot be
    /// reached from Nova today: `std/fs`'s wrappers are straight-line, so nothing
    /// runs between a stash and its take. Increment 4's poller inserts exactly
    /// that gap, which is why the interleaving is built here by hand instead.
    ///
    /// **Fails against the pre-change thread-local slots**, where task B's stash
    /// releases task A's root and overwrites the one shared slot, so A's take
    /// returns B's pointer. That is what earns this test.
    #[test]
    fn a_stash_is_private_to_the_task_that_made_it() {
        let a = crate::gc_str("task-a-payload");
        let b = crate::gc_str("task-b-payload");

        crate::task::set_current_for_test(Some(0));
        stash(Slot::Buffer, a);

        crate::task::set_current_for_test(Some(1));
        stash(Slot::Buffer, b);

        crate::task::set_current_for_test(Some(0));
        let got = take(Slot::Buffer);
        assert_eq!(
            got, a as usize,
            "task 0 must read back its own payload, not task 1's"
        );

        crate::task::set_current_for_test(Some(1));
        assert_eq!(
            take(Slot::Buffer),
            b as usize,
            "task 1's payload must still be there, undisturbed"
        );
        crate::task::set_current_for_test(None);
    }

    /// "No task" is a key, not a special case, and it does not collide with task 0.
    ///
    /// Kills the plausible mistake of indexing by `id as usize` instead of
    /// `id as usize + 1`, which would map task 0 onto the no-task slot. `fs.rs`'s
    /// other unit tests all run with `CURRENT == None`, so this is the path they
    /// take.
    #[test]
    fn the_no_task_key_does_not_collide_with_task_zero() {
        let none_payload = crate::gc_str("no-task");
        let zero_payload = crate::gc_str("task-zero");

        crate::task::set_current_for_test(None);
        stash(Slot::Buffer, none_payload);

        crate::task::set_current_for_test(Some(0));
        stash(Slot::Buffer, zero_payload);

        crate::task::set_current_for_test(None);
        assert_eq!(
            take(Slot::Buffer),
            none_payload as usize,
            "the no-task slot must not be task 0's slot"
        );

        crate::task::set_current_for_test(Some(0));
        assert_eq!(take(Slot::Buffer), zero_payload as usize);
        crate::task::set_current_for_test(None);
    }

    /// A task's unread payload is released when its state root is.
    ///
    /// `task.rs` releases a task's state root in `release_internal` and
    /// `take_output_internal` — deliberately not at completion, because a spawned
    /// task's output has to outlive it so a later `join` can take it (see
    /// `poll_one`'s and `take_output_internal`'s own doc comments in
    /// `task.rs`). Payload release hangs off the same two points so payload
    /// lifetime follows the policy `task.rs` already owns rather than a
    /// second one.
    ///
    /// Uses `gc::root_count` rather than asserting the object is collected: per
    /// ADR 0010, a churn-loop test asserting survival cannot discriminate on this
    /// platform. This proves bookkeeping, not survival, and claims only that.
    #[test]
    fn releasing_a_tasks_slots_drops_the_roots_it_held() {
        let payload = crate::gc_str("never-read");
        let addr = payload as usize;

        crate::task::set_current_for_test(Some(7));
        stash(Slot::Buffer, payload);
        assert_eq!(gc::root_count(addr), 1, "the stash roots its pointer");

        crate::task::set_current_for_test(None);
        release_task_slots(7);
        assert_eq!(
            gc::root_count(addr),
            0,
            "releasing task 7's slots must release the root it held"
        );

        // Idempotent: a second release must not double-remove or abort.
        release_task_slots(7);
        assert_eq!(gc::root_count(addr), 0);
    }

    /// Releasing one task's slots leaves another task's alone.
    #[test]
    fn releasing_one_tasks_slots_leaves_another_tasks_intact() {
        let keep = crate::gc_str("keep-me");
        let keep_addr = keep as usize;

        crate::task::set_current_for_test(Some(3));
        stash(Slot::Message, keep);
        release_task_slots(4);
        assert_eq!(
            gc::root_count(keep_addr),
            1,
            "task 4's release must not touch task 3's slots"
        );

        crate::task::set_current_for_test(Some(3));
        assert_eq!(take(Slot::Message), keep as usize);
        crate::task::set_current_for_test(None);
    }
}
