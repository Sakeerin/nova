//! The three standard streams' boundary.
//!
//! Each intrinsic acquires the process-global stream, acts, and drops it inside
//! the one call -- the same shape `nova_rt_print`/`nova_rt_eprint` have used
//! since phase 1. Nothing here holds an OS handle between calls, which is why
//! these three types need no lifetime management and `File` (increment 3c) will.
//!
//! Payloads travel in the per-task slot table `fs` owns. No panic may cross a
//! generated poll boundary, so nothing here unwraps, expects, indexes a slice,
//! or formats fallibly.
//!
//! # Two boundary decisions, recorded here as well as in the task report that
//! # made them
//!
//! **How a write's byte count reaches Nova.** The status word already carries
//! the `IoErrorKind` on failure, so it cannot also carry a count on success
//! without a sentinel this boundary does not have. It travels instead as a
//! `Bytes` payload in the existing [`Slot::Buffer`], not a new `Slot` variant
//! -- see [`stash_count`]'s own doc comment for why, and for the encoding a
//! Nova wrapper must decode.
//!
//! **`write`, not `write_all`.** [`nova_rt_io_stdout_write`] and
//! [`nova_rt_io_stderr_write`] each make one `Write::write` call and report
//! whatever count it returns, rather than looping until the whole buffer is
//! sent. See [`nova_rt_io_stdout_write`]'s own doc comment for why.

use crate::fs::{fail, stash, Slot, OK};
use crate::NovaStr;
use std::io::Write as _;

/// Read from `reader` into `buf`, truncate to what was actually read, and
/// stash the result as a `Bytes` payload.
///
/// Factored out of [`nova_rt_io_stdin_read`] so the truncate-on-a-short-read
/// behaviour is exercised in this module's own tests against a reader fully
/// under the test's control (`std::io::Cursor`), rather than against the real
/// process stdin. A unit test cannot safely drive real stdin: a blocking read
/// with no piped input left waiting would hang the test binary rather than
/// fail one test, on a developer's machine even though not in this
/// environment's own non-interactive shell. `nova_rt_io_stdin_read` itself,
/// reading real, harness-fed input, is exercised at the Nova fixture level
/// instead (`tests/runtime`, increment 3b's later tasks).
fn read_and_stash(mut reader: impl std::io::Read, buf: &mut Vec<u8>) -> i64 {
    match reader.read(buf) {
        Ok(n) => {
            buf.truncate(n);
            stash(Slot::Buffer, crate::bytes::gc_bytes(buf));
            OK
        }
        Err(e) => fail(&e),
    }
}

/// Read up to `max` bytes from stdin. An **empty** payload means end of stream.
///
/// A short read is not EOF: a terminal or pipe returns what is available. The
/// Nova-level contract is stated in `std/io`'s `Read::read`.
///
/// # Safety
/// No pointer argument, so no dereference precondition; marked `unsafe extern
/// "C"` for uniformity with this crate's other JIT-registered symbols.
#[no_mangle]
pub unsafe extern "C" fn nova_rt_io_stdin_read(max: i64) -> i64 {
    let Ok(cap) = usize::try_from(max) else {
        crate::task::abort_with("nova_rt_io_stdin_read: negative maximum")
    };
    let mut buf = vec![0u8; cap];
    let stdin = std::io::stdin();
    read_and_stash(stdin.lock(), &mut buf)
}

/// Stash a byte count as an 8-byte little-endian `Bytes` payload in
/// [`Slot::Buffer`].
///
/// **Decision 1 of 2: the count travels as a `Bytes` payload, not a new slot
/// kind.** The alternative was a fourth `Slot` variant holding a raw `usize`
/// rather than a GC pointer. `Slot::Buffer` already exists, already crosses
/// into Nova as `Bytes` through the existing `nova_rt_fs_take_bytes`, and
/// already serves more than one logical payload -- `fs.rs`'s own `Slot` doc
/// comment makes the identical point for `String` versus `Bytes` -- so reusing
/// it needs no new `Slot` variant and no new taker intrinsic. `fs.rs` itself
/// only gains `pub(crate)` visibility on `stash`/`fail` (`take` stays
/// private -- its one cross-module consumer is `crate::io`'s own tests,
/// reached through the test-only `take_for_test` instead of widening `take`
/// itself, final review M4) and doc-comment notes (including on the two
/// existing takers) recording the reuse -- no behavioural change anywhere
/// in it.
///
/// **Fixed-width 8 bytes, little-endian -- not the single byte a first sketch
/// of the Nova wrapper might reach for.** A write can exceed 255 bytes, so one
/// byte cannot hold every count an `i64` can; eight bytes hold any
/// non-negative count `write` could ever report. **The Nova-level wrapper
/// must decode all eight bytes back into an `Int`, not read only the first**
/// -- this module's own `stash_count_round_trips_a_count_above_one_byte` test
/// is exactly the case a single-byte encoding would get wrong.
fn stash_count(n: usize) {
    stash(
        Slot::Buffer,
        crate::bytes::gc_bytes(&(n as i64).to_le_bytes()),
    );
}

/// Write `bytes` to `writer` with one `Write::write` call, and stash however
/// many bytes it reports via [`stash_count`].
///
/// **Decision 2 of 2: one `write` call, not a `write_all` loop.** The
/// original (pre-`Bytes`-amendment) `nova-spec/20-STDLIB.md` §4 already typed
/// this as `Result<Int, IoError>`, mirroring Rust's own `Write::write` (a
/// single, possibly-partial call reporting a real count) rather than
/// `Write::write_all` (`Result<(), IoError>`, no count, because it either
/// sends everything or fails). `std/fs`'s `write` makes the opposite,
/// deliberate choice for the opposite reason: it calls `std::fs::write`
/// (write-all semantics under the hood) and returns `Result<(), IoError>`
/// with no count, precisely because it promises no partial write. A caller of
/// *this* boundary that wants a guaranteed full write must loop on the
/// returned count itself -- symmetric to how `Read::read_to_end`'s default
/// body loops over `read` -- rather than this intrinsic silently hiding a
/// short write.
fn write_and_stash(mut writer: impl std::io::Write, bytes: &[u8]) -> i64 {
    match writer.write(bytes) {
        Ok(n) => {
            stash_count(n);
            OK
        }
        Err(e) => fail(&e),
    }
}

/// Write `buf`'s bytes to stdout. See [`write_and_stash`] for why this is one
/// `write` call rather than a `write_all` loop, and [`stash_count`] for how
/// the reported count crosses back into Nova.
///
/// # Safety
/// `buf` must point to a live `NovaStr`.
#[no_mangle]
pub unsafe extern "C" fn nova_rt_io_stdout_write(buf: *const NovaStr) -> i64 {
    // SAFETY: forwarding this function's own contract.
    let bytes = unsafe { crate::bytes::as_bytes(buf) };
    let stdout = std::io::stdout();
    write_and_stash(stdout.lock(), bytes)
}

/// Write `buf`'s bytes to stderr. Mirrors [`nova_rt_io_stdout_write`] exactly,
/// against the other stream.
///
/// # Safety
/// `buf` must point to a live `NovaStr`.
#[no_mangle]
pub unsafe extern "C" fn nova_rt_io_stderr_write(buf: *const NovaStr) -> i64 {
    // SAFETY: forwarding this function's own contract.
    let bytes = unsafe { crate::bytes::as_bytes(buf) };
    let stderr = std::io::stderr();
    write_and_stash(stderr.lock(), bytes)
}

/// Flush stdout.
///
/// # Safety
/// No pointer argument, so no dereference precondition; marked `unsafe extern
/// "C"` for uniformity with this crate's other JIT-registered symbols.
#[no_mangle]
pub unsafe extern "C" fn nova_rt_io_stdout_flush() -> i64 {
    let stdout = std::io::stdout();
    let mut lock = stdout.lock();
    match lock.flush() {
        Ok(()) => OK,
        Err(e) => fail(&e),
    }
}

/// Flush stderr. Mirrors [`nova_rt_io_stdout_flush`] exactly, against the
/// other stream.
///
/// # Safety
/// No pointer argument, so no dereference precondition; marked `unsafe extern
/// "C"` for uniformity with this crate's other JIT-registered symbols.
#[no_mangle]
pub unsafe extern "C" fn nova_rt_io_stderr_flush() -> i64 {
    let stderr = std::io::stderr();
    let mut lock = stderr.lock();
    match lock.flush() {
        Ok(()) => OK,
        Err(e) => fail(&e),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Test-only: decode the byte count [`stash_count`] stashed -- the mirror
    /// image of that function's own 8-byte little-endian encoding.
    ///
    /// Not `pub(crate)`: every caller is one of this module's own tests.
    fn take_count() -> i64 {
        let ptr = crate::fs::take_for_test(Slot::Buffer) as *const NovaStr;
        // SAFETY: test-only. The slot was just populated, in the same test,
        // by either `stash_count` or a real write intrinsic's call into it;
        // nothing has allocated since, so the payload has not been swept.
        let bytes = unsafe { crate::bytes::as_bytes(ptr) };
        let Ok(arr) = <[u8; 8]>::try_from(bytes) else {
            panic!(
                "stash_count always stashes exactly 8 bytes; got {} -- either \
                 nothing was pending or the encoding changed without updating \
                 this test helper to match",
                bytes.len()
            );
        };
        i64::from_le_bytes(arr)
    }

    /// A successful write reports the byte count through the per-task slot.
    ///
    /// The count travels as a payload rather than in the status word because the
    /// status word carries the `IoErrorKind`, and a byte count and an error kind
    /// cannot share one `i64` without a sentinel this boundary does not have.
    #[test]
    fn a_successful_stdout_write_reports_its_byte_count() {
        let payload = crate::gc_bytes_for_test(b"hi");
        let status = unsafe { nova_rt_io_stdout_write(payload) };
        assert_eq!(status, crate::fs::OK, "writing to stdout must succeed");
        assert_eq!(
            take_count(),
            2,
            "the byte count must be the two bytes written, read back from the slot"
        );
    }

    /// Mirrors the stdout test above, against the other stream: two separate
    /// intrinsics that must each independently wire `write` through to
    /// `stash_count`, not only the one the plan's own example happened to show.
    #[test]
    fn a_successful_stderr_write_reports_its_byte_count() {
        let payload = crate::gc_bytes_for_test(b"bye");
        let status = unsafe { nova_rt_io_stderr_write(payload) };
        assert_eq!(status, crate::fs::OK, "writing to stderr must succeed");
        assert_eq!(
            take_count(),
            3,
            "the byte count must be the three bytes written, read back from the slot"
        );
    }

    /// Writing an empty buffer is not an error: `Write::write(&[])` reports
    /// `Ok(0)`, and that zero must still reach Nova as a successful status
    /// with a `0` count, not be mistaken for a failure.
    #[test]
    fn writing_an_empty_buffer_succeeds_with_a_zero_count() {
        let payload = crate::gc_bytes_for_test(b"");
        let status = unsafe { nova_rt_io_stdout_write(payload) };
        assert_eq!(
            status,
            crate::fs::OK,
            "writing zero bytes must still succeed"
        );
        assert_eq!(take_count(), 0);
    }

    /// The count-encoding round-trips for a count above one byte, not only the
    /// two- and three-byte cases the write tests above happen to use.
    ///
    /// A single-byte encoding -- the shape a first sketch of the Nova wrapper
    /// might reach for, reading only `fs_take_bytes().byte_at(0)` -- would
    /// silently truncate any count at or above 256. This is the case that
    /// would catch that, if a future edit narrowed `stash_count`'s encoding
    /// back down to one byte.
    #[test]
    fn stash_count_round_trips_a_count_above_one_byte() {
        stash_count(300);
        assert_eq!(
            take_count(),
            300,
            "a byte count above 255 must round-trip exactly, not truncate to one byte"
        );
    }

    /// A short read must truncate the stashed payload to exactly what was
    /// read, not leave the rest of the pre-allocated buffer as zero padding.
    ///
    /// Exercises [`read_and_stash`] directly against a [`std::io::Cursor`]
    /// that deliberately holds fewer bytes than the caller asked for, rather
    /// than against real stdin -- see that function's own doc comment for
    /// why a test cannot safely do that instead. **Kills the mutation of
    /// deleting `buf.truncate(n)`**: measured by applying that exact
    /// deletion, which turns this test's stashed payload into ten zero bytes
    /// (`max`) instead of the two real bytes read (this task's report records
    /// the result).
    #[test]
    fn a_short_read_truncates_to_what_was_actually_read() {
        let mut buf = vec![0u8; 10]; // far more than the source below has
        let status = read_and_stash(std::io::Cursor::new(b"hi".to_vec()), &mut buf);
        assert_eq!(status, OK);
        let ptr = crate::fs::take_for_test(Slot::Buffer) as *const NovaStr;
        // SAFETY: just stashed by the call above; nothing has allocated since.
        let bytes = unsafe { crate::bytes::as_bytes(ptr) };
        assert_eq!(
            bytes, b"hi",
            "a short read must stash exactly what was read, not `max` zero-padded bytes"
        );
    }

    /// An empty result means end of stream, and the stashed payload must be a
    /// genuine zero-length `Bytes` -- the same EOF convention
    /// `nova_rt_fs_take_bytes`'s own doc comment states for "nothing pending".
    #[test]
    fn an_empty_read_stashes_a_zero_length_payload() {
        let mut buf = vec![0u8; 10];
        let status = read_and_stash(std::io::Cursor::new(Vec::new()), &mut buf);
        assert_eq!(status, OK);
        let ptr = crate::fs::take_for_test(Slot::Buffer) as *const NovaStr;
        // SAFETY: just stashed by the call above; nothing has allocated since.
        let bytes = unsafe { crate::bytes::as_bytes(ptr) };
        assert_eq!(
            bytes, b"" as &[u8],
            "an empty read must stash a zero-length payload -- the EOF signal \
             `Read::read`'s Nova-level contract documents"
        );
    }

    #[test]
    fn stdout_flush_reports_ok() {
        let status = unsafe { nova_rt_io_stdout_flush() };
        assert_eq!(
            status,
            crate::fs::OK,
            "flushing stdout must succeed under normal test conditions"
        );
    }

    #[test]
    fn stderr_flush_reports_ok() {
        let status = unsafe { nova_rt_io_stderr_flush() };
        assert_eq!(
            status,
            crate::fs::OK,
            "flushing stderr must succeed under normal test conditions"
        );
    }

    /// This module's own panic-freedom claim (module doc comment above),
    /// mechanically pinned the same way `fs::tests::no_slot_access_can_panic_
    /// on_a_borrow` and `bytes::tests::no_bytes_intrinsic_can_panic` pin
    /// theirs: nothing in this file's production code may panic, because
    /// every intrinsic here is reachable from a generated poll boundary with
    /// no landing pad to unwind through.
    ///
    /// Scans only the part of this file before its own `mod tests` block, for
    /// the same reason those two guards do, and with the same ceiling they
    /// both point to: what keeps a split like this sound is that the
    /// *first* occurrence of the split literal `mod tests {` is the real
    /// boundary, not that the literal is unique in the file. It is not
    /// unique here either -- the same text recurs elsewhere in this file,
    /// including in this doc comment's own quoting of it above and in this
    /// test's own `source.split("mod tests {")` call below -- and every such
    /// occurrence is harmless only because it comes after the real
    /// declaration, however many there turn out to be. **Do not replace this
    /// sentence with a count**: a fixed number here goes stale the moment
    /// anyone edits this comment again, which is why it states the property
    /// instead. An *earlier* occurrence, such as a doc comment quoting that
    /// exact text before the real declaration, would silently truncate the
    /// scan right there instead, and this class of guard fails open: it
    /// would pass while covering nothing, with no test failing to say so.
    /// Indexing (`[i]`) is also outside what a substring scan can tell safe
    /// from dangerous. See
    /// `fs::tests::no_filesystem_intrinsic_registers_a_park`'s doc comment
    /// (`fs.rs`) for the full statement of all three, including a real case
    /// where an earlier occurrence did exactly this.
    #[test]
    fn no_stream_intrinsic_can_panic() {
        let source = include_str!("io.rs");
        let production = source.split("mod tests {").next().unwrap_or(source);
        for needle in ["unwrap()", ".expect(", "panic!", "format!", "RefCell"] {
            assert!(
                !production.contains(needle),
                "a std/io stream intrinsic must not panic: `{needle}` found in \
                 this file's production code, which is reachable from a \
                 generated poll boundary with no landing pad to unwind through"
            );
        }
    }
}
