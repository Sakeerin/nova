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

/// The byte at `i`, as an `Int` in `0..=255`.
///
/// Aborts on an out-of-range index. Nova's `Bytes::byte_at` bounds-checks first
/// and returns `Option`, so this abort guards a bug in `std/bytes` rather than
/// user error — and it aborts rather than panicking because a panic here could
/// unwind through a generated poll frame.
///
/// # Safety
/// `b` must point to a live `NovaStr`.
#[no_mangle]
pub unsafe extern "C" fn nova_rt_bytes_at(b: *const NovaStr, i: i64) -> i64 {
    // SAFETY: forwarding this function's own contract.
    let bytes = unsafe { as_bytes(b) };
    let Ok(idx) = usize::try_from(i) else {
        crate::task::abort_with("nova_rt_bytes_at: negative index");
    };
    match bytes.get(idx) {
        Some(byte) => i64::from(*byte),
        None => crate::task::abort_with("nova_rt_bytes_at: index out of range"),
    }
}

/// The bytes in `start..end`, clamped to the buffer and to `start <= end`.
///
/// **Deliberately diverges from `String::slice`, which panics on an
/// out-of-range bound (`std/strings/lib.nova`) rather than clamping.** A
/// `Bytes` length often comes from disk (`fs::read`'s result, say), not from
/// a caller who is expected to already know it the way a codepoint index
/// is; aborting the whole process on a bound derived from external input is
/// a worse failure mode than the same defect in a string index a caller
/// chose by hand. The clamp below is two saturating `.max`/`.min` integer
/// comparisons, which is what keeps this from ever aborting, unlike a bounds
/// check that panics. This is a recorded, accepted divergence, not an
/// oversight — see the design spec's dated note — and it does not imply
/// `String::slice` will ever clamp too.
///
/// # Safety
/// `b` must point to a live `NovaStr`.
#[no_mangle]
pub unsafe extern "C" fn nova_rt_bytes_slice(
    b: *const NovaStr,
    start: i64,
    end: i64,
) -> *mut NovaStr {
    // SAFETY: forwarding this function's own contract.
    let bytes = unsafe { as_bytes(b) };
    let lo = start.max(0).min(bytes.len() as i64) as usize;
    let hi = end.max(0).min(bytes.len() as i64) as usize;
    if hi <= lo {
        return gc_bytes(&[]);
    }
    gc_bytes(&bytes[lo..hi])
}

/// `a`'s bytes followed by `b`'s, in a freshly allocated buffer.
///
/// # Safety
/// `a` and `b` must point to live `NovaStr`s.
#[no_mangle]
pub unsafe extern "C" fn nova_rt_bytes_concat(
    a: *const NovaStr,
    b: *const NovaStr,
) -> *mut NovaStr {
    // SAFETY: forwarding this function's own contract.
    let (a, b) = unsafe { (as_bytes(a), as_bytes(b)) };
    let mut out = Vec::with_capacity(a.len() + b.len());
    out.extend_from_slice(a);
    out.extend_from_slice(b);
    gc_bytes(&out)
}

/// Decompose `b` into a Nova `[Int]`, one element per byte in `0..=255`.
///
/// The result must match **exactly** what codegen emits for an array: one
/// block holding `{ len: i64, elem0, elem1, … }`, element `i` at byte offset
/// `8 + 8*i`, allocated *scanned* — the same layout `nova_rt_str_chars`
/// (`nova-runtime`'s crate root) builds for `[Char]`, and pinned here the same
/// way that one is pinned, in this module's own `mod tests`
/// (`to_ints_writes_the_array_layout_codegen_expects`, below).
///
/// # Safety
/// `b` must point to a live `NovaStr`.
#[no_mangle]
pub unsafe extern "C" fn nova_rt_bytes_to_ints(b: *const NovaStr) -> *mut u8 {
    // SAFETY: forwarding this function's own contract.
    let bytes = unsafe { as_bytes(b) };
    let n = bytes.len();
    // `8` for the length header plus `8` per element, exactly as
    // `nova_rt_str_chars` sizes its own `[Char]` block.
    let block = crate::gc::alloc(8 + 8 * n, true);
    let words = block as *mut i64;
    // SAFETY: `block` is a fresh allocation of `8 + 8*n` bytes, so words
    // `0..=n` are in bounds.
    unsafe {
        *words = n as i64;
        for (i, byte) in bytes.iter().enumerate() {
            *words.add(1 + i) = i64::from(*byte);
        }
    }
    block
}

/// A `Bytes` holding each element of `ints` as one byte.
///
/// Aborts if any element is outside `0..=255`: that is a caller error (`std/
/// bytes`'s `bytes_from_ints` is the only path to this intrinsic, and it
/// documents the same contract), and a `Result` here would infect every
/// construction path with a status protocol for what is a programmer mistake.
///
/// # Safety
/// `ints` must point to a Nova array of `Int`: `{ len: i64, elems… }` with
/// element `i` at byte offset `8 + 8*i` — the layout `nova_rt_str_from_chars`
/// (`nova-runtime`'s crate root) reads for `[Char]`.
#[no_mangle]
pub unsafe extern "C" fn nova_rt_bytes_from_ints(ints: *const u8) -> *mut NovaStr {
    let words = ints as *const i64;
    // SAFETY: forwarding this function's own contract.
    let n = unsafe { *words }.max(0) as usize;
    let mut out = Vec::with_capacity(n);
    for i in 0..n {
        // SAFETY: `i < n`, and `n` is the array's own length, so this word is
        // in bounds for the block `ints` describes.
        let v = unsafe { *words.add(1 + i) };
        let Ok(byte) = u8::try_from(v) else {
            crate::task::abort_with("nova_rt_bytes_from_ints: element out of range 0..=255");
        };
        out.push(byte);
    }
    gc_bytes(&out)
}

/// Byte-for-byte equality of `a` and `b`.
///
/// # Safety
/// `a` and `b` must point to live `NovaStr`s.
#[no_mangle]
pub unsafe extern "C" fn nova_rt_bytes_eq(a: *const NovaStr, b: *const NovaStr) -> i8 {
    // SAFETY: forwarding this function's own contract.
    i8::from(unsafe { as_bytes(a) == as_bytes(b) })
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

    /// Pins `nova_rt_bytes_to_ints`'s array layout directly, the same way
    /// `nova-runtime`'s own `str_chars_writes_the_array_layout_codegen_expects`
    /// pins `nova_rt_str_chars`'s. Two elements: `8 + 8*2 = 24` bytes, well
    /// above `gc::alloc`'s 8-byte floor (`gc.rs`, `size.max(8)`), so the
    /// tracked size can actually discriminate a correct allocation from a
    /// wrong one -- a payload at or under the floor could not (see this
    /// module's own `a_bytes_buffer_above_the_gc_floor_is_tracked_at_its_exact_size`
    /// for the same reasoning applied to `gc_bytes`'s buffer).
    #[test]
    fn to_ints_writes_the_array_layout_codegen_expects() {
        let b = gc_bytes(&[7, 8]);
        // SAFETY: `b` is live; this is the same call Nova makes.
        let block = unsafe { nova_rt_bytes_to_ints(b) };
        assert_eq!(
            crate::gc::object_info(block as usize),
            Some((8 + 8 * 2, true)),
            "an array block is a length word plus one word per element, SCANNED"
        );
        let words = block as *mut i64;
        // SAFETY: the block is `8 + 8*2` bytes, so these three words are in bounds.
        unsafe {
            assert_eq!(*words, 2);
            assert_eq!(*words.add(1), 7);
            assert_eq!(*words.add(2), 8);
        }
    }

    /// Pins this module's own panic-freedom contract (module doc comment
    /// above: "nothing here may panic") mechanically, the way
    /// `fs::tests::no_filesystem_intrinsic_registers_a_park` pins `std/fs`'s
    /// "never parks" contract -- final review's recommendation (D11), added
    /// after this exact module shipped a real native, non-unwinding panic
    /// caught only by mutation testing, in code reachable from a generated
    /// poll boundary with no landing pad to unwind through.
    ///
    /// **Follows `fs.rs`'s structure exactly, including why the split point
    /// is the test module marker and not this function's own body.** The
    /// needle array below names `unwrap()`, `.expect(`, `panic!`,
    /// `format!` and `RefCell`, so splitting on this function alone
    /// would leave those words in the scanned half and the test could never
    /// pass. Splitting at `#[cfg(test)]` instead excludes every test (and its
    /// comments) while still scanning all of this file's actual production
    /// code -- which is why this test, like its sibling, must live inside
    /// this module and not be hoisted out of it.
    ///
    /// All five needles are currently absent from the production half, so
    /// this passes today; it auto-covers every future intrinsic added above
    /// without needing a recount, the same durability `fs.rs`'s version has
    /// for `stage_park`.
    #[test]
    fn no_bytes_intrinsic_can_panic() {
        let source = include_str!("bytes.rs");
        let code = source.split("#[cfg(test)]").next().unwrap_or(source);
        for needle in ["unwrap()", ".expect(", "panic!", "format!", "RefCell"] {
            assert!(
                !code.contains(needle),
                "a std/bytes intrinsic must not panic: `{needle}` found in this \
                 file's production code, which is reachable from a generated poll \
                 boundary with no landing pad to unwind through"
            );
        }
    }
}
