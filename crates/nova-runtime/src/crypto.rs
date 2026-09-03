//! `std/crypto`'s intrinsics: SHA-256, SHA-512, HMAC-SHA-256 and its
//! constant-time check, plus random bytes and a bounded random integer.
//!
//! # The boundary
//!
//! Each intrinsic returns a status `i64` and stashes any payload in
//! `Slot::Buffer`, the convention `std/fs` and `std/net` already use: Nova
//! collects bytes with `fs_take_bytes()` and an integer with
//! `decode_count(fs_take_bytes())`. `nova_rt_http_parse_request` instead
//! returns a GC-allocated array directly; ADR 0019 argues that shape on
//! FFI-crossing count and leak-freedom, neither of which applies here, so
//! this module follows the older and simpler convention.
//!
//! # What is inherited rather than implemented
//!
//! The constant-time property of the tag check comes from
//! `ring::hmac::verify`. **No test here observes timing**, so that property
//! is inherited and is described as such wherever it appears. The obvious
//! alternative a caller might reach for on the Nova side, `Bytes::eq`,
//! bottoms out in Rust slice equality: it returns early on a length
//! mismatch, and because `u8` is `BytewiseEq` it then calls `memcmp`, which
//! carries no constant-time guarantee.

use crate::bytes::{as_bytes, gc_bytes};
use crate::fs::{stash, Slot};
use crate::NovaStr;
use ring::rand::SecureRandom;
use ring::{digest, hmac, rand};

/// Which operation `nova_rt_crypto_hash` performs.
///
/// These four values are duplicated in `std/crypto/lib.nova` as Nova
/// constants. Nothing in the compiler ties the two sides together, so a
/// fixture asserts a known vector per operation: swapping two of these
/// constants on either side must fail that fixture, which is what pins them.
pub(crate) const OP_SHA256: i64 = 0;
pub(crate) const OP_SHA512: i64 = 1;
pub(crate) const OP_HMAC_SHA256: i64 = 2;
pub(crate) const OP_HMAC_SHA256_VERIFY: i64 = 3;

/// Error kinds, returned negated in a status word.
///
/// `nova_rt_crypto_hash` never returns any of these: none of its operations
/// can fail. They belong to the two random intrinsics.
const ERR_ENTROPY_UNAVAILABLE: i64 = 1;
const ERR_INVALID_LENGTH: i64 = 2;
const ERR_REQUEST_TOO_LARGE: i64 = 3;
const ERR_INVALID_RANGE: i64 = 4;

/// The largest `random_bytes` request served.
///
/// Keys, nonces and tokens run to tens of bytes, so this is generous by
/// orders of magnitude for every intended use, and it is checked before
/// anything is allocated. What it guards is platform-dependent: ADR 0002 is
/// superseded by a mark-and-sweep collector that reclaims, but precise stack
/// bounds are implemented on Windows only, and the other platforms still
/// fall back to leak-until-exit until their stack-bounds query lands.
const MAX_RANDOM_BYTES: usize = 64 * 1024;

/// Status for the tag-check operation when the tag does not match.
///
/// Not an error: a mismatch is the answer the caller asked for.
const TAG_MISMATCH: i64 = 1;

/// Digest or HMAC over `data`, keyed by `key` when the operation is keyed.
///
/// Returns `None` for an unknown operation, which the callers treat as an
/// invariant violation rather than an error kind.
fn digest_bytes(op: i64, key: &[u8], data: &[u8]) -> Option<Vec<u8>> {
    match op {
        OP_SHA256 => Some(digest::digest(&digest::SHA256, data).as_ref().to_vec()),
        OP_SHA512 => Some(digest::digest(&digest::SHA512, data).as_ref().to_vec()),
        OP_HMAC_SHA256 => {
            let k = hmac::Key::new(hmac::HMAC_SHA256, key);
            Some(hmac::sign(&k, data).as_ref().to_vec())
        }
        _ => None,
    }
}

/// Constant-time check of `tag` against the HMAC of `data` under `key`.
///
/// The constant-time property is `ring::hmac::verify`'s, inherited here and
/// not demonstrated by any test in this crate.
fn verify_tag(key: &[u8], data: &[u8], tag: &[u8]) -> bool {
    let k = hmac::Key::new(hmac::HMAC_SHA256, key);
    hmac::verify(&k, data, tag).is_ok()
}

/// `max - min` as an unsigned width, or `None` for an inverted range.
///
/// Computed in `i128` because `max - min` overflows `i64` for a wide range:
/// `i64::MIN` to `i64::MAX` is `u64::MAX`, and the span it implies is one
/// larger still and fits no `u64` at all. That case is handled by its
/// callers rather than represented here.
fn width_of(min: i64, max: i64) -> Option<u64> {
    if min > max {
        return None;
    }
    Some((max as i128 - min as i128) as u64)
}

/// A uniform value in `0..span` from one 64-bit draw, or `None` if the draw
/// falls in the biased tail and must be rejected.
///
/// Rejecting the low `2^64 mod span` values leaves a count divisible by
/// `span`, so the modulo above the tail is uniform. `2^64 mod span` is
/// computed as `(u64::MAX - span + 1) % span`, since `u64::MAX - span + 1`
/// is `2^64 - span` and that is congruent to `2^64` modulo `span`. The
/// rejection probability is therefore `(2^64 mod span) / 2^64`, which the
/// caller's range controls: a span of `2^62 + 1` rejects a quarter of draws
/// and `2^63 + 1` a half, while a span of ten rejects about three draws in
/// every `10^19`.
fn reduce(draw: u64, span: u64) -> Option<u64> {
    if span <= 1 {
        return Some(0);
    }
    let threshold = (u64::MAX - span + 1) % span;
    if draw < threshold {
        return None;
    }
    Some(draw % span)
}

/// Place a reduced draw at `min + offset`.
///
/// The caller has already reduced `offset` into `0..=(max - min)` for
/// whatever interval it is placing into, so the sum lands inside that
/// interval and fits; this function itself sees only `min` and the offset,
/// never the interval's upper end. `wrapping_add` is correct rather than
/// merely convenient: the intermediate cast of a large `u64` to `i64` is
/// negative and two's complement addition still lands on the right value
/// regardless.
fn place(offset: u64, min: i64) -> i64 {
    min.wrapping_add(offset as i64)
}

/// Fill `out` from the OS entropy source.
fn fill(out: &mut [u8]) -> Result<(), ()> {
    let sr = rand::SystemRandom::new();
    match sr.fill(out) {
        Ok(()) => Ok(()),
        Err(_) => Err(()),
    }
}

/// SHA-256, SHA-512, HMAC-SHA-256 or its constant-time check.
///
/// Returns `0` with the digest in `Slot::Buffer` for the three producing
/// operations, `TAG_MISMATCH` for a failed check, and `0` for a passing one.
/// **No negative status**: none of the four operations can fail, so this
/// intrinsic has no error range at all.
///
/// # Safety
/// `key`, `data` and `tag` must each be a valid `NovaStr` pointer or null,
/// which is what the compiler emits for a `Bytes` argument.
#[no_mangle]
pub unsafe extern "C" fn nova_rt_crypto_hash(
    op: i64,
    key: *const NovaStr,
    data: *const NovaStr,
    tag: *const NovaStr,
) -> i64 {
    // SAFETY: forwarding this function's own contract.
    let k = unsafe { as_bytes(key) };
    let d = unsafe { as_bytes(data) };
    let t = unsafe { as_bytes(tag) };

    if op == OP_HMAC_SHA256_VERIFY {
        return if verify_tag(k, d, t) { 0 } else { TAG_MISMATCH };
    }

    match digest_bytes(op, k, d) {
        Some(out) => {
            stash(Slot::Buffer, gc_bytes(&out));
            0
        }
        // An unknown op means the Nova side and this file disagree, which no
        // caller can cause and no error kind describes.
        None => crate::task::abort_with("nova_rt_crypto_hash: unknown operation"),
    }
}

/// `n` random bytes into `Slot::Buffer`.
///
/// # Safety
/// No pointer argument, so no dereference precondition; marked `unsafe
/// extern "C"` for uniformity with this crate's other JIT-registered
/// symbols.
#[no_mangle]
pub unsafe extern "C" fn nova_rt_crypto_random_bytes(n: i64) -> i64 {
    let Ok(want) = usize::try_from(n) else {
        return -ERR_INVALID_LENGTH;
    };
    // Checked before anything is allocated, following `MAX_HEAD_BYTES`.
    if want > MAX_RANDOM_BYTES {
        return -ERR_REQUEST_TOO_LARGE;
    }
    let mut out = vec![0u8; want];
    if fill(&mut out).is_err() {
        return -ERR_ENTROPY_UNAVAILABLE;
    }
    stash(Slot::Buffer, gc_bytes(&out));
    0
}

/// A uniform integer in `min..=max`, encoded into `Slot::Buffer`.
///
/// # Safety
/// No pointer argument, so no dereference precondition; marked `unsafe
/// extern "C"` for uniformity with this crate's other JIT-registered
/// symbols.
#[no_mangle]
pub unsafe extern "C" fn nova_rt_crypto_random_int(min: i64, max: i64) -> i64 {
    let Some(width) = width_of(min, max) else {
        return -ERR_INVALID_RANGE;
    };

    let value = if width == u64::MAX {
        // A span of 2^64: every draw is in range, so there is nothing to
        // reject and no span to reduce by.
        let mut raw = [0u8; 8];
        if fill(&mut raw).is_err() {
            return -ERR_ENTROPY_UNAVAILABLE;
        }
        place(u64::from_le_bytes(raw), min)
    } else {
        let span = width + 1;
        let mut offset = None;
        // Bounded rather than unbounded: the rejection probability is below
        // one half for every span, so this exits with probability at least
        // `1 - 2^-64`. An unbounded loop here would be a hang under a broken
        // entropy source, which this project has recorded as the failure
        // mode that hides rather than fails.
        for _ in 0..64 {
            let mut raw = [0u8; 8];
            if fill(&mut raw).is_err() {
                return -ERR_ENTROPY_UNAVAILABLE;
            }
            if let Some(v) = reduce(u64::from_le_bytes(raw), span) {
                offset = Some(v);
                break;
            }
        }
        match offset {
            Some(v) => place(v, min),
            None => return -ERR_ENTROPY_UNAVAILABLE,
        }
    };

    stash(Slot::Buffer, gc_bytes(&value.to_le_bytes()));
    0
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The reduction is pure and takes its randomness as an argument, so the
    /// rejection branch can be reached on demand. Against a live entropy
    /// source it cannot: `SystemRandom` cannot be asked for a particular
    /// draw.
    #[test]
    fn reduce_rejects_the_low_tail_and_is_uniform_above_it() {
        // span 3 over a 64-bit draw: 2^64 mod 3 == 1, so exactly one value
        // (0) must be rejected and the rest map uniformly.
        assert_eq!(reduce(0, 3), None, "the one biased draw must be rejected");
        // A draw of 1 is already below the span, so `draw % span` (the same
        // relationship the next line's `u64::MAX % 3` checks) is just 1.
        assert_eq!(reduce(1, 3), Some(1));
        assert_eq!(reduce(u64::MAX, 3), Some(u64::MAX % 3));
    }

    #[test]
    fn reduce_accepts_every_draw_when_the_span_divides_the_range() {
        // 2^64 mod 2 == 0, so nothing is rejected.
        assert_eq!(reduce(0, 2), Some(0));
        assert_eq!(reduce(u64::MAX, 2), Some(1));
    }

    #[test]
    fn reduce_is_identity_for_a_span_of_one() {
        assert_eq!(reduce(0, 1), Some(0));
        assert_eq!(reduce(u64::MAX, 1), Some(0));
    }

    /// `min == i64::MIN` with `max == i64::MAX` is a span of 2^64, which does
    /// not fit a `u64`. It is the case a naive `max - min + 1` gets wrong.
    #[test]
    fn the_full_signed_range_is_a_span_no_u64_can_hold() {
        assert_eq!(width_of(i64::MIN, i64::MAX), Some(u64::MAX));
        assert_eq!(width_of(0, 0), Some(0));
        assert_eq!(width_of(5, 4), None, "an inverted range has no width");
    }

    /// Placing a draw at both ends of the requested interval, which is what
    /// a caller sees.
    #[test]
    fn a_reduced_draw_lands_inside_the_requested_interval() {
        assert_eq!(place(0, -10), -10);
        assert_eq!(place(20, -10), 10);
        assert_eq!(place(0, i64::MIN), i64::MIN);
    }

    /// The rejection branch is reachable from a LIVE source when the span is
    /// wide, which the pure test above cannot show: this one exercises the
    /// entropy source and the reduction together. At a span of `2^63 + 1`
    /// the rejection probability is one half, so forty draws all landing
    /// above the tail has probability around `2^-40`.
    #[test]
    fn a_wide_span_reaches_the_rejection_branch_from_the_live_source() {
        let span = (1u64 << 63) + 1;
        let mut rejected = 0;
        let mut accepted = 0;
        for _ in 0..40 {
            let mut raw = [0u8; 8];
            assert!(fill(&mut raw).is_ok(), "the OS entropy source must work");
            match reduce(u64::from_le_bytes(raw), span) {
                None => rejected += 1,
                Some(v) => {
                    accepted += 1;
                    assert!(v < span, "a reduced draw must be below the span");
                }
            }
        }
        assert!(
            rejected > 0,
            "a span of 2^63 + 1 rejects half of all draws, so forty should              include at least one rejection"
        );
        assert!(accepted > 0, "and at least one acceptance");
    }

    /// The guard `std/http`'s intrinsic ships. It is a source-text check, not
    /// a proof of panic-freedom: indexing and arithmetic can still panic
    /// without matching any needle here.
    #[test]
    fn no_crypto_intrinsic_can_panic() {
        let source = include_str!("crypto.rs");
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
                "a std/crypto intrinsic must not panic: `{needle}` found in \
                 this file's production code, which is reachable from a \
                 compiled Nova frame with no landing pad to unwind through"
            );
        }
    }

    #[test]
    fn sha256_matches_a_cross_checked_vector() {
        // Cross-checked against Python's hashlib. Widely published as FIPS
        // 180-4's worked example; this tree has verified the former, not the
        // latter.
        let got = digest_bytes(OP_SHA256, &[], b"abc").expect("sha256 cannot fail");
        assert_eq!(
            hex(&got),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }

    #[test]
    fn sha512_matches_a_cross_checked_vector() {
        let got = digest_bytes(OP_SHA512, &[], b"abc").expect("sha512 cannot fail");
        assert_eq!(got.len(), 64);
        assert!(hex(&got).starts_with("ddaf35a193617aba"));
    }

    #[test]
    fn hmac_sha256_matches_a_cross_checked_vector() {
        // Key of twenty 0x0b bytes over "Hi There"; cross-checked against
        // Python's hmac. Widely published as RFC 4231's first case.
        let key = [0x0bu8; 20];
        let got = digest_bytes(OP_HMAC_SHA256, &key, b"Hi There").expect("hmac cannot fail");
        assert_eq!(
            hex(&got),
            "b0344c61d8db38535ca8afceaf0bf12b881dc200c9833da726e9376c2e32cff7"
        );
    }

    #[test]
    fn hmac_verify_accepts_the_right_tag_and_rejects_others() {
        let key = [0x0bu8; 20];
        let tag = digest_bytes(OP_HMAC_SHA256, &key, b"Hi There").expect("hmac cannot fail");
        assert!(verify_tag(&key, b"Hi There", &tag));
        assert!(!verify_tag(&key, b"Hi there", &tag), "different data");
        assert!(
            !verify_tag(&[0x0cu8; 20], b"Hi There", &tag),
            "different key"
        );
        assert!(!verify_tag(&key, b"Hi There", &tag[..31]), "truncated tag");
        assert!(!verify_tag(&key, b"Hi There", &[]), "empty tag");
    }

    fn hex(v: &[u8]) -> String {
        let mut s = String::with_capacity(v.len() * 2);
        for b in v {
            s.push(char::from_digit((b >> 4) as u32, 16).unwrap_or('?'));
            s.push(char::from_digit((b & 0x0f) as u32, 16).unwrap_or('?'));
        }
        s
    }
}
