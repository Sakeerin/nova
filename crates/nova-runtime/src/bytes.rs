//! Byte-buffer intrinsics for `std/bytes`.
//!
//! # Representation, and why there is no `NovaBytes` struct
//!
//! A `Bytes` value is a [`crate::NovaStr`] — a scanned `{len, ptr}` header over
//! a GC **leaf** buffer. That is not an approximation: `String` and `Bytes` have
//! the *same* layout and differ only in that `String` carries a UTF-8
//! guarantee, which lives in the type system rather than the representation.
//!
//! **A second struct with the same fields would be a second copy of a layout,
//! which is the drift class this project has already shipped a miscompile
//! from.** So there is one struct and one set of allocation helpers, and the
//! distinction is enforced by `hir::Ty`, not by Rust.
//!
//! # Panic-freedom
//!
//! Some of these are reachable from inside an `async fn`'s generated `$poll`,
//! which has no landing pads, so nothing here may panic. Where a caller error
//! must be rejected, it aborts — `abort_with` does not unwind.

use crate::NovaStr;

/// Store `bytes` as a GC-managed `Bytes` value: the payload in a **leaf**
/// buffer, the header a **scanned** object that keeps the buffer alive.
///
/// The sibling of `crate::gc_str`, differing only in taking `&[u8]` rather than
/// `&str`. Both produce the identical layout, deliberately.
pub(crate) fn gc_bytes(bytes: &[u8]) -> *mut NovaStr {
    let len = bytes.len();
    // A non-traced buffer: bytes are never pointers, so tracing them would
    // retain arbitrary heap objects that merely look like addresses.
    let buf = crate::gc::alloc(len.max(1), false);
    // SAFETY: `buf` has `len.max(1)` writable bytes.
    unsafe { std::ptr::copy_nonoverlapping(bytes.as_ptr(), buf, len) };
    let node = crate::gc::alloc(std::mem::size_of::<NovaStr>(), true) as *mut NovaStr;
    // SAFETY: `node` points to a fresh `NovaStr`-sized allocation.
    unsafe {
        (*node).len = len as u64;
        (*node).ptr = buf;
    }
    node
}

/// The bytes of `b` as a slice.
///
/// # Safety
/// `b` must point to a live `NovaStr`.
pub(crate) unsafe fn as_bytes<'a>(b: *const NovaStr) -> &'a [u8] {
    // SAFETY: forwarding this function's own contract; `{len, ptr}` describes a
    // valid buffer for any `NovaStr` this module or `gc_str` produced.
    unsafe { std::slice::from_raw_parts((*b).ptr, (*b).len as usize) }
}

/// The byte length of `b`. Not a character count — `Bytes` has no encoding.
///
/// # Safety
/// `b` must point to a live `NovaStr`.
#[no_mangle]
pub unsafe extern "C" fn nova_rt_bytes_len(b: *const NovaStr) -> i64 {
    // SAFETY: forwarding this function's own contract.
    unsafe { (*b).len as i64 }
}

/// A `Bytes` holding `s`'s UTF-8 bytes.
///
/// # Safety
/// `s` must point to a live `NovaStr`.
#[no_mangle]
pub unsafe extern "C" fn nova_rt_bytes_from_string(s: *const NovaStr) -> *mut NovaStr {
    // SAFETY: forwarding this function's own contract.
    gc_bytes(unsafe { as_bytes(s) })
}

/// Whether `b`'s bytes are valid UTF-8.
///
/// Paired with [`nova_rt_bytes_to_string_unchecked`] so `std/bytes`'s
/// `to_string` can be a Nova-level `if`, rather than needing a status-code
/// protocol for what is one boolean.
///
/// # Safety
/// `b` must point to a live `NovaStr`.
#[no_mangle]
pub unsafe extern "C" fn nova_rt_bytes_is_utf8(b: *const NovaStr) -> i8 {
    // SAFETY: forwarding this function's own contract.
    i8::from(std::str::from_utf8(unsafe { as_bytes(b) }).is_ok())
}

/// `b`'s bytes as a `String`, **without** checking UTF-8 validity.
///
/// The caller must have established validity with
/// [`nova_rt_bytes_is_utf8`] first; `std/bytes`'s `to_string` is the only
/// caller and does exactly that. This copies `b`'s bytes into a fresh buffer
/// via `gc_bytes` — an invalid `String` would be unsound, which is why the
/// check is not optional.
///
/// # Safety
/// `b` must point to a live `NovaStr` whose bytes are valid UTF-8.
#[no_mangle]
pub unsafe extern "C" fn nova_rt_bytes_to_string_unchecked(b: *const NovaStr) -> *mut NovaStr {
    // SAFETY: forwarding this function's own contract. `gc_bytes` allocates a
    // fresh header and buffer and copies `b`'s bytes into it, rather than
    // reusing `b`'s own storage.
    gc_bytes(unsafe { as_bytes(b) })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_bytes_value_has_a_scanned_header_over_a_leaf_buffer() {
        let b = gc_bytes(&[1, 2, 3]);
        assert_eq!(
            crate::gc::object_info(b as usize),
            Some((std::mem::size_of::<NovaStr>(), true)),
            "the header must be SCANNED, or the collector frees the buffer under it"
        );
        // SAFETY: `b` is the live header `gc_bytes` just built.
        let buf = unsafe { (*b).ptr };
        assert_eq!(
            crate::gc::object_info(buf as usize),
            // `gc::alloc` floors every request to at least 8 bytes
            // (`let size = size.max(8);`, `gc.rs`) regardless of the `scan`
            // flag or caller, so a 3-byte payload is tracked at size 8, not a
            // literal 3. `Bytes::len` is unaffected -- it reads the *header's*
            // own `len` field (set to `3` below by `gc_bytes`), never this
            // allocator bookkeeping. **Measured, not copied from the plan
            // as-is**: the plan's literal `Some((3, false))` was run first and
            // failed with `left: Some((8, false))`; see the Task 2 report.
            Some((8, false)),
            "the buffer must be a LEAF: bytes are not pointers, and tracing them \
             would retain arbitrary objects that merely look like addresses"
        );
    }

    /// **Decided before execution.** An earlier draft of this plan left
    /// `bytes_is_utf8` with no test that could catch it until Task 3's Nova
    /// fixture, and said so — a deliberate but avoidable gap, since `mod
    /// tests` can reach the intrinsic directly with no Nova surface at all.
    #[test]
    fn is_utf8_distinguishes_valid_bytes_from_invalid() {
        // Both directions, because a mutation to either constant is plausible and
        // each is caught only by the case it gets wrong.
        // SAFETY: both arguments are live headers from `gc_bytes`.
        unsafe {
            assert_eq!(nova_rt_bytes_is_utf8(gc_bytes(b"hi")), 1, "valid UTF-8");
            assert_eq!(
                nova_rt_bytes_is_utf8(gc_bytes(&[0xFF])),
                0,
                "0xFF is not a valid UTF-8 sequence on its own"
            );
        }
    }

    /// **Review finding I1.** Every payload above (and the `bytes_basics`
    /// Nova fixture's) is 1-3 bytes, entirely under `gc::alloc`'s 8-byte
    /// floor (`let size = size.max(8);`, `gc.rs`), so the buffer's tracked
    /// size reads 8 whether the request was correct or wrong by up to seven
    /// bytes — no assertion using only those payloads can tell the two
    /// apart. Proven, not assumed: hardcoding `gc_bytes`'s buffer allocation
    /// to `crate::gc::alloc(1, false)`, ignoring `len` entirely, left the
    /// whole suite (including `a_bytes_value_has_a_scanned_header_over_a_
    /// leaf_buffer` above) green — and for any payload actually over 8 bytes
    /// that mutation is a real heap-buffer overflow, since the
    /// `copy_nonoverlapping` right after it still copies the full `len`
    /// bytes into an 8-byte allocation. A payload above the floor is
    /// required so the buffer's tracked size has a value other than 8 to be
    /// wrong about.
    #[test]
    fn a_bytes_buffer_above_the_gc_floor_is_tracked_at_its_exact_size() {
        let payload = [7u8; 32];
        let b = gc_bytes(&payload);
        // SAFETY: `b` is the live header `gc_bytes` just built.
        let buf = unsafe { (*b).ptr };
        assert_eq!(
            crate::gc::object_info(buf as usize),
            Some((32, false)),
            "a payload above the allocator's 8-byte floor must be tracked at \
             its own exact size, not merely at the floor"
        );
    }

    /// **Review finding M2.** Before this, the only exercise of
    /// `nova_rt_bytes_len` was the `bytes_basics` fixture's single 2-byte
    /// string — one data point, which cannot distinguish "returns the real
    /// length" from a hardcoded constant. Proven, not assumed: hardcoding
    /// the return to `2` passed the whole suite, fixture included (review
    /// also checked the brief's other suggested probe, returning
    /// `size_of::<NovaStr>()`, and that one *does* already die against the
    /// existing fixture, since 16 != 2 — so the hardcoded-constant gap was
    /// the only one left open). Two different real lengths rule out a
    /// constant; the second clears `gc::alloc`'s 8-byte floor so this
    /// doubles as a second, independent probe of the same floor
    /// `a_bytes_buffer_above_the_gc_floor_is_tracked_at_its_exact_size`
    /// exercises, this time through the header's `len` field rather than
    /// `object_info`.
    #[test]
    fn bytes_len_reports_the_real_length_for_more_than_one_size() {
        unsafe {
            assert_eq!(nova_rt_bytes_len(gc_bytes(&[1, 2, 3])), 3);
            assert_eq!(nova_rt_bytes_len(gc_bytes(&[7u8; 32])), 32);
        }
    }
}
