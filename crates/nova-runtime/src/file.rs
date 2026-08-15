//! Open files, keyed by descriptor.
//!
//! **This is the first thing in this runtime that holds an OS resource across
//! more than one intrinsic call.** `fs`'s functions open, act and close inside
//! a single call, and `io`'s streams are process-global and never closed — so
//! neither needed a table. A `File` does.
//!
//! Nova has no destructors, so `std/fs`'s `close` is the only release
//! mechanism and a forgotten `File` leaks its descriptor until the process
//! exits. The design spec records why close-on-collect is foreclosed rather
//! than merely unbuilt.
//!
//! Absence from this table *is* closedness: a `read` on a closed fd, a stale
//! fd, or an fd a Nova program forged by hand all miss the lookup and become
//! one `IoError`. That is deliberate — record fields are not privacy-enforced,
//! so a forged `File { fd: 99 }` is constructible and must be safe.

use crate::fs::{fail, stash, Slot, OK};
use crate::NovaStr;
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::io::{Read as _, Write as _};

thread_local! {
    /// Open files by descriptor. `thread_local!` for the reason `task.rs`'s
    /// module doc gives for `TASKS`: the GC's roots are per-thread, so a
    /// second thread running Nova code would free objects the first holds.
    static FILES: RefCell<HashMap<i64, std::fs::File>> = RefCell::new(HashMap::new());
    /// Never reused, so a stale fd stays stale rather than aliasing a
    /// different file later. Starts at 1 so 0 is available as an obviously
    /// invalid value in diagnostics.
    static NEXT_FD: Cell<i64> = const { Cell::new(1) };
}

/// Run `f` against the file behind `fd`, or report a closed-file error.
///
/// `try_borrow_mut` rather than `borrow_mut`: a `RefCell` panic here would
/// cross a generated poll boundary. The `None` arm is the closed/stale/forged
/// case and is an ordinary error, not an abort — `std/fs`'s `close` cannot
/// consume its receiver, because Nova has no move checking, so use-after-close
/// is a mistake the language invites rather than an exotic one.
fn with_fd<R>(fd: i64, f: impl FnOnce(&mut std::fs::File) -> R) -> Option<R> {
    FILES.with(|files| {
        let Ok(mut files) = files.try_borrow_mut() else {
            crate::task::abort_with("nova_rt_file: handle table is already borrowed")
        };
        files.get_mut(&fd).map(f)
    })
}

/// Allocate a fresh, never-reused fd for `file` and insert it into the table.
///
/// Fallible-borrow the same way [`with_fd`] is, and for the identical reason:
/// a `RefCell` panic here would cross a generated poll boundary. Unlike
/// [`with_fd`] there is no missing-key case to report — insertion always
/// creates the entry — so the only failure mode is the contended borrow,
/// which aborts.
fn register_new_file(file: std::fs::File) -> i64 {
    let fd = NEXT_FD.with(|next| {
        let fd = next.get();
        next.set(fd + 1);
        fd
    });
    FILES.with(|files| {
        let Ok(mut files) = files.try_borrow_mut() else {
            crate::task::abort_with("nova_rt_file: handle table is already borrowed")
        };
        files.insert(fd, file);
    });
    fd
}

/// Stash `n` as an 8-byte little-endian `Bytes` payload in `Slot::Buffer`.
///
/// Matches the encoding `crate::io`'s `stash_count` uses for a write's byte
/// count (see `nova_rt_io_stdout_write`'s own doc comment) rather than
/// inventing a second one: both are an `i64` that must ride in `Slot::Buffer`
/// because the status word already carries the `IoErrorKind`. Not a call into
/// that function itself, which is private to its own module and outside the
/// four names this module imports from `fs` — this is a second
/// implementation of the identical encoding, used here for both `open`'s new
/// fd and `write`'s byte count.
fn stash_i64(n: i64) {
    stash(Slot::Buffer, crate::bytes::gc_bytes(&n.to_le_bytes()));
}

/// Report the closed/stale/forged-fd case as `IoError { kind: Other }`.
///
/// **The decision this task asked to be made and reported.** Every other
/// failure path in this module has a real `std::io::Error` to draw both a
/// status and a message from, via [`fail`]. This path does not: the table
/// lookup missed before any syscall ran, so there is no OS error to describe.
/// Rather than reach past the four names this module imports from `fs` for a
/// message-only stash helper, this fabricates a `std::io::Error` of
/// `ErrorKind::Other` carrying a fixed message and routes it through the same
/// [`fail`] every real failure already uses. `fail`'s own match has no arm
/// for `ErrorKind::Other`, so it falls to that match's catch-all and returns
/// the identical status a real `Other`-kind failure would, while `fail`
/// stashes this message through the same `Slot::Message` path it always
/// does. Net effect: no new constant, no second status-mapping table, and a
/// Nova program reads back this fixed string through `fs_last_error_message()`
/// in place of whatever text a real OS failure would have produced — there
/// being no real failure here to describe.
fn closed_fd_error() -> i64 {
    fail(&std::io::Error::other("file descriptor is not open"))
}

/// Open `path`, forwarding all six flags to `std::fs::OpenOptions` one for
/// one. On success, stashes the new fd as an 8-byte little-endian `Bytes`
/// payload in `Slot::Buffer` via [`stash_i64`] — the fd cannot travel in the
/// status word because the status word already carries the `IoErrorKind`.
///
/// # Safety
/// `path` must point to a live `NovaStr`.
#[no_mangle]
pub unsafe extern "C" fn nova_rt_file_open(
    path: *const NovaStr,
    read: i8,
    write: i8,
    append: i8,
    truncate: i8,
    create: i8,
    create_new: i8,
) -> i64 {
    // SAFETY: forwarding this function's own contract.
    let p = unsafe { crate::as_str(path) };
    match std::fs::OpenOptions::new()
        .read(read != 0)
        .write(write != 0)
        .append(append != 0)
        .truncate(truncate != 0)
        .create(create != 0)
        .create_new(create_new != 0)
        .open(p)
    {
        Ok(file) => {
            stash_i64(register_new_file(file));
            OK
        }
        Err(e) => fail(&e),
    }
}

/// Close `fd`, dropping the underlying `std::fs::File` and releasing its OS
/// handle. Idempotent: removing an already-closed, stale, or forged fd finds
/// nothing in the table and still returns `OK`, so a second or third `close`
/// on the same value succeeds exactly like the first — `std/fs`'s `close`
/// cannot consume its receiver, because Nova has no move checking, so a
/// caller can always reach a second call.
///
/// # Safety
/// No pointer argument, so no dereference precondition; marked `unsafe
/// extern "C"` for uniformity with this crate's other JIT-registered symbols.
#[no_mangle]
pub unsafe extern "C" fn nova_rt_file_close(fd: i64) -> i64 {
    FILES.with(|files| {
        let Ok(mut files) = files.try_borrow_mut() else {
            crate::task::abort_with("nova_rt_file: handle table is already borrowed")
        };
        files.remove(&fd);
    });
    OK
}

/// Read up to `max` bytes from `fd` into a truncated buffer, stashed via
/// `Slot::Buffer`. An **empty** result means end of stream; a short read does
/// not, mirroring `std/io`'s `Read::read` contract.
///
/// Guards `max` the way `nova_rt_io_stdin_read` does: a negative value
/// aborts. A large `max` still allocates the whole capacity eagerly before
/// any read happens — a known asymmetry the design spec records, not fixed
/// here.
///
/// # Safety
/// No pointer argument, so no dereference precondition; marked `unsafe
/// extern "C"` for uniformity with this crate's other JIT-registered symbols.
#[no_mangle]
pub unsafe extern "C" fn nova_rt_file_read(fd: i64, max: i64) -> i64 {
    let Ok(cap) = usize::try_from(max) else {
        crate::task::abort_with("nova_rt_file_read: negative maximum")
    };
    let mut buf = vec![0u8; cap];
    match with_fd(fd, |file| file.read(&mut buf)) {
        Some(Ok(n)) => {
            buf.truncate(n);
            stash(Slot::Buffer, crate::bytes::gc_bytes(&buf));
            OK
        }
        Some(Err(e)) => fail(&e),
        None => closed_fd_error(),
    }
}

/// Write `buf`'s bytes to `fd` with one `Write::write` call, possibly
/// partial, and stash however many bytes it reports via [`stash_i64`]. See
/// `crate::io`'s `nova_rt_io_stdout_write` for why this is one call rather
/// than a `write_all` loop — the identical contract applies here.
///
/// # Safety
/// `buf` must point to a live `NovaStr`.
#[no_mangle]
pub unsafe extern "C" fn nova_rt_file_write(fd: i64, buf: *const NovaStr) -> i64 {
    // SAFETY: forwarding this function's own contract.
    let bytes = unsafe { crate::bytes::as_bytes(buf) };
    match with_fd(fd, |file| file.write(bytes)) {
        Some(Ok(n)) => {
            stash_i64(n as i64);
            OK
        }
        Some(Err(e)) => fail(&e),
        None => closed_fd_error(),
    }
}

/// Flush `fd`.
///
/// # Safety
/// No pointer argument, so no dereference precondition; marked `unsafe
/// extern "C"` for uniformity with this crate's other JIT-registered symbols.
#[no_mangle]
pub unsafe extern "C" fn nova_rt_file_flush(fd: i64) -> i64 {
    match with_fd(fd, |file| file.flush()) {
        Some(Ok(())) => OK,
        Some(Err(e)) => fail(&e),
        None => closed_fd_error(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A path under the OS temp directory combining this process's id with
    /// `label`, mirroring `crates/nova-cli/tests/run_tests.rs`'s
    /// `unique_temp_dir` helper one level down: a file here, a directory
    /// there. Two calls land on different paths if they pass different
    /// labels, which is how the tests below use this; a leftover file from
    /// a prior run could only be revisited if the OS later recycled that
    /// run's exact process id before the leftover was cleaned up.
    fn unique_temp_path(label: &str) -> String {
        std::env::temp_dir()
            .join(format!("nova-rt-file-{label}-{}", std::process::id()))
            .to_string_lossy()
            .into_owned()
    }

    /// Test-only: decode an fd or byte count [`stash_i64`] stashed — the
    /// mirror image of that function's own 8-byte little-endian encoding.
    /// Serves both: an `open`'s fd and a `write`'s count ride the identical
    /// encoding, so one decoder reads back either.
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

    /// Test-only: take the pending payload as owned bytes, for comparing a
    /// read result against an expected slice.
    fn take_bytes() -> Vec<u8> {
        let ptr = crate::fs::take_for_test(Slot::Buffer) as *const NovaStr;
        // SAFETY: test-only, for the reason `take_fd`'s comment gives: this
        // same test stashed the payload earlier, with nothing allocated
        // since.
        unsafe { crate::bytes::as_bytes(ptr) }.to_vec()
    }

    /// A file opened for writing, written, closed, reopened and read gives back
    /// what was written.
    ///
    /// The whole point of this module is that a handle survives between intrinsic
    /// calls, which nothing else in this runtime does — so the first test drives
    /// the full open/write/close/open/read/close sequence rather than one call.
    #[test]
    fn a_file_round_trips_through_the_handle_table() {
        let path = unique_temp_path("round-trip");
        let p = crate::gc_str(&path);

        let status = unsafe { nova_rt_file_open(p, 0, 1, 0, 1, 1, 0) };
        assert_eq!(status, OK, "open for writing must succeed");
        let fd = take_fd();

        let payload = crate::bytes::gc_bytes(b"handle-table");
        assert_eq!(unsafe { nova_rt_file_write(fd, payload) }, OK);
        assert_eq!(unsafe { nova_rt_file_close(fd) }, OK);

        let status = unsafe { nova_rt_file_open(p, 1, 0, 0, 0, 0, 0) };
        assert_eq!(status, OK, "open for reading must succeed");
        let fd = take_fd();
        assert_eq!(unsafe { nova_rt_file_read(fd, 64) }, OK);
        assert_eq!(
            take_bytes(),
            b"handle-table",
            "the bytes must survive the round trip"
        );
        assert_eq!(unsafe { nova_rt_file_close(fd) }, OK);

        let _ = std::fs::remove_file(&path);
    }

    /// The design's central claim: an operation on a closed fd is an
    /// ordinary error, not a panic, an abort, or a silent success reading
    /// stale memory. Checks read, write and flush individually, since a
    /// mutation could plausibly land in any one of their `None` arms without
    /// touching the other two.
    ///
    /// Opened with **both** `read` and `write` (unlike this module's other
    /// write-only fixtures) so that, under a broken `close` that leaves the
    /// table entry in place, all three checks below independently succeed
    /// rather than fail for an unrelated reason — measured by applying
    /// exactly that mutation to `nova_rt_file_close`: with a write-only
    /// handle, the read check still failed, but for a real OS permission
    /// error rather than the table defect this test exists to catch,
    /// leaving the write check to catch it alone. Read-write closes that
    /// gap, and re-running the same mutation confirmed the read check then
    /// fails for the intended reason too.
    #[test]
    fn using_a_file_after_close_is_an_error_not_a_panic() {
        let path = unique_temp_path("use-after-close");
        let p = crate::gc_str(&path);
        assert_eq!(unsafe { nova_rt_file_open(p, 1, 1, 0, 1, 1, 0) }, OK);
        let fd = take_fd();
        assert_eq!(unsafe { nova_rt_file_close(fd) }, OK);

        let read_status = unsafe { nova_rt_file_read(fd, 16) };
        assert_ne!(
            read_status, OK,
            "reading a closed fd must fail, not silently succeed"
        );

        let payload = crate::bytes::gc_bytes(b"x");
        let write_status = unsafe { nova_rt_file_write(fd, payload) };
        assert_ne!(
            write_status, OK,
            "writing a closed fd must fail, not silently succeed"
        );

        let flush_status = unsafe { nova_rt_file_flush(fd) };
        assert_ne!(
            flush_status, OK,
            "flushing a closed fd must fail, not silently succeed"
        );

        let _ = std::fs::remove_file(&path);
    }

    /// A fd this module never issued — as opposed to one it issued and then
    /// closed — takes the identical path through `with_fd`'s `None` arm. `0`
    /// is never issued: `NEXT_FD` starts at 1 by this module's own stated
    /// convention. This is the Rust-level shape of the design spec's forged
    /// `File` case: a Nova program can construct `File { fd: 9999 }`
    /// directly, since record fields are not privacy-enforced.
    #[test]
    fn a_never_issued_fd_is_an_error_not_a_panic() {
        assert_ne!(unsafe { nova_rt_file_read(0, 16) }, OK);
        assert_ne!(unsafe { nova_rt_file_flush(0) }, OK);
        assert_eq!(
            unsafe { nova_rt_file_close(0) },
            OK,
            "closing an fd that was never open is still OK -- close is idempotent \
             over absence, not only over a fd it previously held"
        );
    }

    /// Two files open at the same time must not collide: opening a second
    /// file while the first is still open must not overwrite the first's
    /// table entry or alias its handle. A counter that fails to advance
    /// would still pass every test above, none of which holds more than one
    /// fd open at once.
    #[test]
    fn two_open_files_do_not_collide() {
        let path_a = unique_temp_path("two-files-a");
        let path_b = unique_temp_path("two-files-b");
        let pa = crate::gc_str(&path_a);
        let pb = crate::gc_str(&path_b);

        assert_eq!(unsafe { nova_rt_file_open(pa, 0, 1, 0, 1, 1, 0) }, OK);
        let fd_a = take_fd();
        assert_eq!(unsafe { nova_rt_file_open(pb, 0, 1, 0, 1, 1, 0) }, OK);
        let fd_b = take_fd();
        assert_ne!(
            fd_a, fd_b,
            "two opens live at the same time must not be handed the same descriptor"
        );

        assert_eq!(
            unsafe { nova_rt_file_write(fd_a, crate::bytes::gc_bytes(b"file-a-payload")) },
            OK
        );
        assert_eq!(
            unsafe { nova_rt_file_write(fd_b, crate::bytes::gc_bytes(b"file-b-payload")) },
            OK
        );
        assert_eq!(unsafe { nova_rt_file_close(fd_a) }, OK);
        assert_eq!(unsafe { nova_rt_file_close(fd_b) }, OK);

        let got_a = std::fs::read(&path_a).expect("file a must be readable after close");
        let got_b = std::fs::read(&path_b).expect("file b must be readable after close");
        assert_eq!(got_a, b"file-a-payload", "file a must hold its own payload");
        assert_eq!(got_b, b"file-b-payload", "file b must hold its own payload");

        let _ = std::fs::remove_file(&path_a);
        let _ = std::fs::remove_file(&path_b);
    }

    /// `close` stays a no-op on repetition, not only correct the first time.
    #[test]
    fn close_is_idempotent_across_repeated_calls() {
        let path = unique_temp_path("idempotent-close");
        let p = crate::gc_str(&path);
        assert_eq!(unsafe { nova_rt_file_open(p, 0, 1, 0, 1, 1, 0) }, OK);
        let fd = take_fd();
        assert_eq!(unsafe { nova_rt_file_close(fd) }, OK, "the first close");
        assert_eq!(unsafe { nova_rt_file_close(fd) }, OK, "a second close");
        assert_eq!(unsafe { nova_rt_file_close(fd) }, OK, "a third close");
        let _ = std::fs::remove_file(&path);
    }

    /// A portable, real failure: opening a path that does not exist, with no
    /// `create` flag, must fail rather than report success. Guards against
    /// `open`'s status check being deleted or hardcoded — the design spec
    /// notes that exact shape of gap has shipped before for other wrappers.
    #[test]
    fn opening_a_missing_file_for_reading_fails() {
        let path = unique_temp_path("does-not-exist");
        let p = crate::gc_str(&path);
        let status = unsafe { nova_rt_file_open(p, 1, 0, 0, 0, 0, 0) };
        assert_ne!(
            status, OK,
            "opening a nonexistent path for reading must fail"
        );
    }

    /// `create_new` must reach the OS, not be silently ignored: on an
    /// already-existing path it must fail rather than truncate the file
    /// that is already there.
    #[test]
    fn create_new_on_an_existing_path_fails() {
        let path = unique_temp_path("create-new-collision");
        let p = crate::gc_str(&path);
        assert_eq!(unsafe { nova_rt_file_open(p, 0, 1, 0, 0, 1, 0) }, OK);
        let fd = take_fd();
        assert_eq!(unsafe { nova_rt_file_close(fd) }, OK);

        let status = unsafe { nova_rt_file_open(p, 0, 1, 0, 0, 0, 1) };
        assert_ne!(status, OK, "create_new on an existing path must fail");

        let _ = std::fs::remove_file(&path);
    }

    /// The flag that distinguishes each of `std/fs`'s three `OpenOptions`
    /// constructors must reach the OS: `writing()` truncates, `appending()`
    /// does not, and `reading()` refuses a write.
    ///
    /// Written because the `truncate`/`create`/`append` trio was unpinned
    /// where the `read`/`write` pair was not. `append` was `0` at every
    /// `nova_rt_file_open` call site in this file, and no test reopened a
    /// path that already held content.
    ///
    /// The mutation that shipped green was the one at the **`std/fs` call
    /// site** — transposing `options.truncate` and `options.create` where
    /// `open` forwards them — and it is `tests/runtime/file_roundtrip.nova`
    /// that kills it. Transposing *this function's own* `truncate` and
    /// `create` parameters was never a survivor:
    /// `create_new_on_an_existing_path_fails` already caught it. Both
    /// measured, not supposed.
    ///
    /// What this test adds is the other half — that each constructor's tuple,
    /// once forwarded, reaches the OS with the effect its name claims.
    /// `writing()`'s tuple sets both flags and so cannot see a
    /// truncate/create swap at all; `appending()`'s sets only `create`, so
    /// under one this test's own open asks for append and truncate together,
    /// which `std::fs::OpenOptions` rejects outright. It therefore fails at
    /// "opening for appending must succeed" rather than by observing a
    /// truncation.
    ///
    /// The `reading()` leg is the design spec's §5 clause "`reading()` then
    /// attempting a write fails", which was named as delivered and never
    /// written.
    ///
    /// The tuples are read off `std/fs/lib.nova`'s three constructors in
    /// this function's own parameter order — read, write, append, truncate,
    /// create, create_new. What this level cannot see is a change to those
    /// constructors themselves, which do not exist here;
    /// `tests/runtime/file_roundtrip.nova` pins them from the Nova side.
    #[test]
    fn each_constructors_flags_reach_the_os() {
        let path = unique_temp_path("constructor-flags");
        let p = crate::gc_str(&path);
        let _ = std::fs::remove_file(&path);

        // `OpenOptions::writing()` — write + truncate + create.
        assert_eq!(unsafe { nova_rt_file_open(p, 0, 1, 0, 1, 1, 0) }, OK);
        let fd = take_fd();
        assert_eq!(
            unsafe { nova_rt_file_write(fd, crate::bytes::gc_bytes(b"first")) },
            OK
        );
        assert_eq!(unsafe { nova_rt_file_close(fd) }, OK);

        // `OpenOptions::appending()` — append + create, and no truncate.
        assert_eq!(
            unsafe { nova_rt_file_open(p, 0, 0, 1, 0, 1, 0) },
            OK,
            "opening for appending must succeed"
        );
        let fd = take_fd();
        assert_eq!(
            unsafe { nova_rt_file_write(fd, crate::bytes::gc_bytes(b"-second")) },
            OK
        );
        assert_eq!(unsafe { nova_rt_file_close(fd) }, OK);
        assert_eq!(
            std::fs::read(&path).expect("the appended-to file must be readable"),
            b"first-second",
            "appending() must leave content the file already had in place"
        );

        // `OpenOptions::writing()` again, this time over a file that has
        // content: only the newest payload may survive.
        assert_eq!(unsafe { nova_rt_file_open(p, 0, 1, 0, 1, 1, 0) }, OK);
        let fd = take_fd();
        assert_eq!(
            unsafe { nova_rt_file_write(fd, crate::bytes::gc_bytes(b"third")) },
            OK
        );
        assert_eq!(unsafe { nova_rt_file_close(fd) }, OK);
        assert_eq!(
            std::fs::read(&path).expect("the rewritten file must be readable"),
            b"third",
            "writing() must discard content the file already had"
        );

        // `OpenOptions::reading()` — read only, so a write must fail rather
        // than report a byte count for bytes that never landed.
        assert_eq!(unsafe { nova_rt_file_open(p, 1, 0, 0, 0, 0, 0) }, OK);
        let fd = take_fd();
        assert_ne!(
            unsafe { nova_rt_file_write(fd, crate::bytes::gc_bytes(b"nope")) },
            OK,
            "writing a read-only handle must fail, not silently succeed"
        );
        assert_eq!(unsafe { nova_rt_file_close(fd) }, OK);
        assert_eq!(
            std::fs::read(&path).expect("the read-only-opened file must be readable"),
            b"third",
            "a write the OS rejected must not have reached the file"
        );

        let _ = std::fs::remove_file(&path);
    }

    /// This module's own panic-freedom claim, pinned the same mechanical way
    /// `fs::tests::no_slot_access_can_panic_on_a_borrow`,
    /// `bytes::tests::no_bytes_intrinsic_can_panic`, and
    /// `io::tests::no_stream_intrinsic_can_panic` each pin theirs: nothing in
    /// this file's production code may panic, since every intrinsic here is
    /// reachable from a generated poll boundary with no landing pad to
    /// unwind through.
    ///
    /// Scans only the part of this file before its own `mod tests` block.
    /// What keeps a split like this sound is that the *first* occurrence of
    /// the split literal is the real boundary, not that the literal is
    /// unique in the file — it is not unique here either, and an occurrence
    /// after the real declaration is harmless for exactly that reason; see
    /// `fs::tests::no_filesystem_intrinsic_registers_a_park`'s doc comment
    /// for the full statement of that property, the class of guard this
    /// belongs to, and the ceiling on what it can see (it fails open, and
    /// cannot distinguish a safe `[i]` from a dangerous one).
    ///
    /// `RefCell` is deliberately left out of the needle list, the same
    /// deliberate omission `fs.rs`'s own sibling guard makes and for the
    /// identical reason: this module's design *is* a `RefCell` (`FILES`,
    /// declared near the top of this file), so that needle would fail
    /// immediately here rather than guard against a regression.
    #[test]
    fn no_file_intrinsic_can_panic() {
        let source = include_str!("file.rs");
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
                "a std/fs File intrinsic must not panic: `{needle}` found in this \
                 file's production code, which is reachable from a generated poll \
                 boundary with no landing pad to unwind through"
            );
        }
    }
}
