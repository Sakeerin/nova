# `std/crypto` Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Ship SHA-256, SHA-512, HMAC-SHA-256, a constant-time HMAC tag check, random bytes and a bounded random integer as `std/crypto`, backed by `ring` over three runtime intrinsics.

**Architecture:** Three `unsafe extern "C"` intrinsics in a new `crates/nova-runtime/src/crypto.rs`, using the established status-plus-slot convention: each returns a status `i64` and stashes any payload in `Slot::Buffer`, which Nova collects with `fs_take_bytes()` or `decode_count(fs_take_bytes())`. The unbiased reduction for `random_int` is a pure Rust function taking raw bytes, so its rejection branch is testable. `std/crypto/lib.nova` wraps the three behind the surface in section 4 of the spec.

**Tech Stack:** Rust (`ring = "0.17"`), Nova (`std/crypto/lib.nova`), the ADR 0018 intrinsic seam across `nova-mir`, `nova-resolver` and `nova-typeck`.

**Spec:** `docs/superpowers/specs/2026-09-03-std-crypto-design.md`

## Global Constraints

Every task's requirements implicitly include this section.

- `cargo build --locked --workspace` **before** `cargo test`. A test run after a failed build proves nothing.
- `cargo test --locked --workspace --no-fail-fast`. `--no-fail-fast` is mandatory.
- **Never pipe cargo output through `head`/`tail` before summing.** Sum **every** `test result:` line. Baseline: **1110 passed / 0 failed / 8 ignored across 45 targets**, verified locally on Windows.
- No `reason = "..."` in any lint attribute. MSRV is 1.78 and `reason` postdates it.
- `cargo clippy --locked --workspace --all-targets -- -D warnings` must pass on **both** ubuntu and windows.
- `cargo fmt --all -- --check`. If it fails, run `cargo fmt --all`, then `git diff --numstat` and confirm the change count matches what you meant — `cargo fmt` writes LF into this CRLF working copy.
- The ignored ADR-0010 GC tests stay ignored and untouched.
- **The poll ABI is frozen and no panic may cross a generated poll boundary.** This is LIVE in this increment, unlike the last one: Task 1 adds runtime intrinsics reachable from compiled Nova frames.
- Every fixture path unique per process.
- Commit messages written to a UTF-8 file and applied with `git commit -F`, **never a heredoc**. Every body ends exactly with the line `Co-Authored-By: Claude Opus 5 <noreply@anthropic.com>`.
- **Cite no SHA that is not already an ancestor of `main`.** `e31c997` is. Every commit on this branch is branch-local; refer to those changes by what they did.
- **Byte-scan every file you write, from STAGED content via `git show :<path>`** — `core.autocrlf` smudges the working tree. Valid UTF-8; no byte below 0x20 outside tab, CR and LF; no 0x7f; zero occurrences of a backslash followed by `u` and four hex digits in tracked markdown, written as U+XXXX instead. Build that pattern with `re.escape(chr(92))` and assert it against a planted positive first: typed as a single backslash it becomes a regex escape that fails to compile, and a POSIX ERE degrades it to a literal `u` and matches the word "succeeds".
- **Do not author Nova string escapes or markdown backslashes through a heredoc.** A quoted heredoc ate a backslash level twice on an earlier branch, once putting four real CR bytes into a tracked file. Use the Write tool.
- **Sentence-shape discipline**, binding on every comment, doc and record: prefer a roster with no count; a corrected number is usually the wrong fix; no ordinals or closed worlds over `std`, the runtime, the workspace or the record set; never claim a test is "the only" thing that catches something; never write that a fixture pins something without checking a fixture executes it.
- **A grep hit is a pointer, not a fact.** Read a document's status before quoting its body. This plan's spec justified a decision from ADR 0002's title while that ADR's Status line said Superseded.
- **grep is line-oriented and this repo's prose wraps.** A miss is not evidence of absence; flatten before claiming anything is absent.

### Known flake, and how to handle it

Roughly one run in four, an async or threading test fails on Windows. It has historically carried `0xc0000005` but **not always** — an instance carried no crash code at all, just an async child exiting non-zero with empty stdout. The cause is not established. If you hit it: re-run, say so in your report, attribute no cause, and fix nothing. **Do not grep for `0xc0000005` as the test of whether it fired** — it will tell you it did not.

---

## File Structure

| file | responsibility |
|---|---|
| `crates/nova-runtime/src/crypto.rs` | **new.** The three intrinsics, the error constants, the pure reduction, the guard test and the Rust-side unit tests. |
| `crates/nova-runtime/src/lib.rs` | `mod crypto;` plus three `symbols()` entries. |
| `crates/nova-runtime/Cargo.toml` | the `ring` dependency. |
| `crates/nova-mir/src/lib.rs` | `RtFunc` variant, symbol-name arm, MIR-signature arm — per intrinsic. |
| `crates/nova-mir/src/lower.rs` | `Builtin` to `Lowering::Runtime` arm — per intrinsic. |
| `crates/nova-resolver/src/lib.rs` | `Builtin` variant, Nova-name arm, `STD_ONLY` element — per intrinsic; and the `STD_MODULES` entry. |
| `crates/nova-typeck/src/check.rs` | the classification arm, the signature arm, and the `#[cfg(test)]` description table — per intrinsic. |
| `std/crypto/lib.nova` | **new.** The user-facing surface. |
| `tests/runtime/crypto_*.nova` and `.stdout` | **new.** End-to-end fixtures with goldens. |
| `crates/nova-mir/tests/lower_tests.rs` | one lowering assertion per intrinsic. |

---

## Task 1: The runtime intrinsics

**Files:**
- Create: `crates/nova-runtime/src/crypto.rs`
- Modify: `crates/nova-runtime/Cargo.toml`, `crates/nova-runtime/src/lib.rs`

**Interfaces:**
- Consumes: `crate::bytes::{as_bytes, gc_bytes}`, `crate::fs::{Slot, stash}`, and `ring`.
- Produces: `nova_rt_crypto_hash(op: i64, key: *const NovaStr, data: *const NovaStr, tag: *const NovaStr) -> i64`; `nova_rt_crypto_random_bytes(n: i64) -> i64`; `nova_rt_crypto_random_int(min: i64, max: i64) -> i64`. Op constants `OP_SHA256 = 0`, `OP_SHA512 = 1`, `OP_HMAC_SHA256 = 2`, `OP_HMAC_SHA256_VERIFY = 3`. Error constants `ERR_ENTROPY_UNAVAILABLE = 1`, `ERR_INVALID_LENGTH = 2`, `ERR_REQUEST_TOO_LARGE = 3`, `ERR_INVALID_RANGE = 4`, returned negated.

- [ ] **Step 1: Add the dependency**

In `crates/nova-runtime/Cargo.toml`, under `[dependencies]`:

```toml
# The Phase 2 build order names `ring` for `std/crypto`
# (`nova-spec/00-MASTER-SPEC.md` section 3). Chosen over the RustCrypto
# family because it is what the spec names, needs one dependency argument
# rather than four, and adds fewer new lockfile entries: `ring`, `untrusted`,
# `wasi` and `getrandom` 0.2.x, where the alternative adds roughly a dozen.
# Its own MSRV is 1.66, under this workspace's 1.78 floor.
ring = "0.17"
```

Then run `cargo build --locked --workspace`. **If it fails because the lockfile is stale**, that is expected: run `cargo build --workspace` once to update `Cargo.lock`, then confirm with `git diff --stat Cargo.lock` that the added entries are the four named above and nothing else. Report the actual set — if it differs, say so rather than adjusting the comment to match.

- [ ] **Step 2: Write the failing unit tests**

Create `crates/nova-runtime/src/crypto.rs` containing only the test module for now:

```rust
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
        assert_eq!(reduce(1, 3), Some(1 % 3));
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
        assert_eq!(place(0, -10, 10), -10);
        assert_eq!(place(20, -10, 10), 10);
        assert_eq!(place(0, i64::MIN, i64::MAX), i64::MIN);
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
        assert!(!verify_tag(&[0x0cu8; 20], b"Hi There", &tag), "different key");
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
```

**Note the `hex` helper lives inside `mod tests`** so its `unwrap_or` does not trip the guard test, which only scans production code. `char::from_digit` cannot fail for a nibble, and `unwrap_or('?')` avoids a panic path regardless.

- [ ] **Step 3: Run the tests to verify they fail**

```bash
cargo test --locked --package nova-runtime crypto:: --no-fail-fast
```

Expected: **compile failure**, not test failures — `reduce`, `width_of`, `place`, `digest_bytes`, `verify_tag` and the `OP_*` constants do not exist yet. That is the intended shape: the tests name the interface before it exists. Report the actual first error.

- [ ] **Step 4: Write the implementation**

Prepend to `crates/nova-runtime/src/crypto.rs`, above the test module:

```rust
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
use ring::{digest, hmac, rand};
use ring::rand::SecureRandom;

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

/// Place a reduced draw inside the caller's interval.
///
/// `wrapping_add` is correct rather than merely convenient: `offset` is at
/// most `max - min`, so the sum is in `min..=max` and fits, but the
/// intermediate cast of a large `u64` to `i64` is negative and two's
/// complement addition still lands on the right value.
fn place(offset: u64, min: i64, max: i64) -> i64 {
    let _ = max;
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
        place(u64::from_le_bytes(raw), min, max)
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
            Some(v) => place(v, min, max),
            None => return -ERR_ENTROPY_UNAVAILABLE,
        }
    };

    stash(Slot::Buffer, gc_bytes(&value.to_le_bytes()));
    0
}
```

**AMENDED 2026-09-08 (branch `std-crypto-hashes-hmac-random`): three doc comments quoted in this task's blocks say more than is true, and the shipped file says less.** They are left as written above and superseded by this marker rather than edited.

- The `ERR_*` block's "none of its operations can fail" and `nova_rt_crypto_hash`'s "**No negative status**: none of the four operations can fail, so this intrinsic has no error range at all" are both **unconditional, and the claim needs a scope.** Three of the four operations route through a `ring` entry point that ends in an `.unwrap()` of an `InputTooLongError` — `ring::digest::digest`, `ring::hmac::Key::new` and `ring::hmac::sign`. That error needs an input near 2^61 bytes, so no input a Nova program can construct reaches it and the plan's conclusions all survive; what does not survive is the unqualified sentence. `ring::hmac::verify`, which the tag check uses, returns a `Result` and unwraps nothing — worth stating, because this module's constant-time argument rests on that function. The shipped comments say "cannot fail for any input a Nova program can construct" and name the bound as `ring`'s.
- `no_crypto_intrinsic_can_panic`'s doc, in Step 2's block above, discloses only that "indexing and arithmetic can still panic without matching any needle here". **It omits the larger blind spot**: `include_str!("crypto.rs")` scans this file and never the dependency these intrinsics call into, which is where the `.unwrap()`s actually are. The shipped doc discloses both directions, because this guard is the project's named mechanism for "no panic crosses a generated poll boundary" and an overstated scope is what that invariant cannot afford.
- `nova_rt_crypto_hash`'s `# Safety` block says `key`, `data` and `tag` "must each be a valid `NovaStr` pointer **or null**". **No such tolerance exists.** `as_bytes` (`crates/nova-runtime/src/bytes.rs`) requires a live `NovaStr` and dereferences `(*b).ptr` with no null check, and no caller of it checks either. Nothing passes null today — Nova's `no_bytes()` is `bytes_from_ints([])`, a non-null empty `Bytes` — so this was a contract advertising a capability rather than a live bug. The shipped comment drops "or null" and says why.

Then register the module and its symbols. In `crates/nova-runtime/src/lib.rs`, add `mod crypto;` beside the other module declarations, and three entries in `symbols()` alongside the existing `nova_rt_http_parse_request` pair:

```rust
            "nova_rt_crypto_hash",
            crypto::nova_rt_crypto_hash as *const u8,
            "nova_rt_crypto_random_bytes",
            crypto::nova_rt_crypto_random_bytes as *const u8,
            "nova_rt_crypto_random_int",
            crypto::nova_rt_crypto_random_int as *const u8,
```

Match the surrounding entries' exact shape — read them first rather than assuming this snippet's formatting fits.

**One comment elsewhere goes stale.** `stash`'s doc comment in `crates/nova-runtime/src/fs.rs` says it is `pub(crate)` because "`crate::io` is a second consumer". `crate::crypto` makes that an enumeration of some but not all consumers. Amend that sentence to name the consumers without counting them, or to say `std/fs` is not the only consumer without asserting how many there are.

- [ ] **Step 5: Run the tests to verify they pass**

```bash
cargo build --locked --workspace && cargo test --locked --package nova-runtime crypto:: --no-fail-fast
```

Expected: PASS, all of them.

- [ ] **Step 6: Run the reduction mutation**

Change `reduce` so the rejection branch never fires — replace `if draw < threshold { return None; }` with nothing — and run:

```bash
cargo test --locked --package nova-runtime crypto::tests::reduce_rejects --no-fail-fast
```

The test **must** fail. **Report what actually happened**, then revert and confirm green. If it passes, the test is not reaching the branch and must be strengthened before you revert.

- [ ] **Step 7: Full suite, lint, format, byte-scan, commit**

```bash
cargo build --locked --workspace && cargo test --locked --workspace --no-fail-fast
cargo clippy --locked --workspace --all-targets -- -D warnings
cargo fmt --all -- --check
```

Sum every `test result:` line. Expected: 1110 plus the new unit tests, 0 failed, 8 ignored. Byte-scan the staged files, then commit.

Subject: `feat(std/crypto): add the hash, HMAC and random intrinsics`. The body records the boundary convention and why ADR 0019's shape was not followed, that the constant-time property is inherited from `ring` and untested, the four new lockfile entries as actually observed, the bounded retry and why it is bounded, and the mutation's actual outcome.

---

## Task 2: The compiler seam

**Files:**
- Modify: `crates/nova-mir/src/lib.rs`, `crates/nova-mir/src/lower.rs`, `crates/nova-resolver/src/lib.rs`, `crates/nova-typeck/src/check.rs`
- Test: `crates/nova-mir/tests/lower_tests.rs`

**Interfaces:**
- Consumes: the three C symbols from Task 1.
- Produces: Nova-callable builtins `crypto_hash(op: Int, key: Bytes, data: Bytes, tag: Bytes) -> Int`, `crypto_random_bytes(n: Int) -> Int`, `crypto_random_int(min: Int, max: Int) -> Int`. All three are `STD_ONLY`.

- [ ] **Step 1: Read the ADR, then find the sites by measurement**

Read `docs/adr/0018-std-json-scope-and-build-order.md` section 3 for the counting rule. Then, rather than working from a list, add **only** the two enum variants for the first intrinsic — `RtFunc::CryptoHash` in `crates/nova-mir/src/lib.rs` and `Builtin::CryptoHash` in `crates/nova-resolver/src/lib.rs` — and run:

```bash
cargo check --locked --workspace --all-targets
```

Every remaining forced site comes back as `error[E0004]: non-exhaustive patterns`. **`--all-targets` is mandatory**: one forced site is a description table inside `nova-typeck`'s `#[cfg(test)]` module, so a plain `cargo check --workspace` finds one fewer and reports success.

For reference, the equivalent sites for the last intrinsic added were:

| file | site |
|---|---|
| `nova-mir/src/lib.rs` | `RtFunc` variant |
| `nova-mir/src/lib.rs` | `RtFunc` to C-symbol-name arm |
| `nova-mir/src/lib.rs` | `RtFunc` to MIR signature arm |
| `nova-mir/src/lower.rs` | `Builtin` to `Lowering::Runtime` arm |
| `nova-resolver/src/lib.rs` | `Builtin` variant |
| `nova-resolver/src/lib.rs` | `Builtin` to Nova-name arm |
| `nova-resolver/src/lib.rs` | the `STD_ONLY` array element |
| `nova-typeck/src/check.rs` | the classification arm |
| `nova-typeck/src/check.rs` | the `Builtin` to signature arm |
| `nova-typeck/src/check.rs` | the `#[cfg(test)]` description table |

plus the `extern "C"` definition and the `symbols()` entry, which Task 1 did.

**Do not plan against three times that count.** Some of those sites are arrays and match blocks that take three entries in a single edit; how much recurs per additional intrinsic in one increment is not established anywhere. **Re-derive the real number with the grep ADR 0018 prescribes and report what it actually was** — that measurement is a deliverable of this task.

- [ ] **Step 2: Write the failing lowering test**

Append to `crates/nova-mir/tests/lower_tests.rs`, matching the shape of the existing `HttpParseRequest` assertion — read it first:

```rust
#[test]
fn crypto_builtins_reach_their_runtime_functions() {
    assert_lowers_to(
        "fn main() { let _ = crypto_hash(0, bytes_from_ints([]), bytes_from_ints([]), bytes_from_ints([])) }",
        vec![RtFunc::CryptoHash],
        "a Nova call to `crypto_hash` reaches `RtFunc::CryptoHash` exactly",
    );
    assert_lowers_to(
        "fn main() { let _ = crypto_random_bytes(8) }",
        vec![RtFunc::CryptoRandomBytes],
        "a Nova call to `crypto_random_bytes` reaches `RtFunc::CryptoRandomBytes` exactly",
    );
    assert_lowers_to(
        "fn main() { let _ = crypto_random_int(0, 9) }",
        vec![RtFunc::CryptoRandomInt],
        "a Nova call to `crypto_random_int` reaches `RtFunc::CryptoRandomInt` exactly",
    );
}
```

**The helper's real name and signature may differ** — the existing assertion is the authority. Adapt to it rather than introducing a second helper.

- [ ] **Step 3: Run it to verify it fails**

```bash
cargo test --locked --package nova-mir crypto_builtins --no-fail-fast
```

Expected: compile failure on the unknown `RtFunc` variants, or a failed assertion. Report which.

- [ ] **Step 4: Fill in every site**

Work through the errors `--all-targets` reports. The values each site needs:

- Nova names: `crypto_hash`, `crypto_random_bytes`, `crypto_random_int`.
- C symbols: `nova_rt_crypto_hash`, `nova_rt_crypto_random_bytes`, `nova_rt_crypto_random_int`.
- MIR signatures: `CryptoHash` is `(vec![MirTy::I64, MirTy::Ptr, MirTy::Ptr, MirTy::Ptr], MirTy::I64)`; `CryptoRandomBytes` is `(vec![MirTy::I64], MirTy::I64)`; `CryptoRandomInt` is `(vec![MirTy::I64, MirTy::I64], MirTy::I64)`. **Check the `MirTy` variant names against the file** — `I64` is a guess from `MirTy::Ptr` being real, and the enum is the authority.
- Typeck signatures: `CryptoHash` is `(vec![Ty::Int, Ty::Bytes, Ty::Bytes, Ty::Bytes], Ty::Int)`; `CryptoRandomBytes` is `(vec![Ty::Int], Ty::Int)`; `CryptoRandomInt` is `(vec![Ty::Int, Ty::Int], Ty::Int)`.
- All three go in `STD_ONLY`: they are not user-visible language surface, only `std/crypto`'s.

- [ ] **Step 5: Run the test to verify it passes**

```bash
cargo build --locked --workspace && cargo test --locked --package nova-mir crypto_builtins --no-fail-fast
```

Expected: PASS.

- [ ] **Step 6: Confirm `--all-targets` was necessary, and report the count**

```bash
cargo check --locked --workspace
cargo check --locked --workspace --all-targets
grep -rn 'CryptoHash' crates/ --include=*.rs
```

Report the site count the grep gives against the arithmetic ADR 0018 predicts, and whether the plain `check` would have reported success with a site still missing. If it would not have — if this increment's shape makes all sites visible without `--all-targets` — that is a finding about the ADR's claim worth recording, not a reason to skip the flag.

- [ ] **Step 7: Full suite, lint, format, commit**

As Task 1's Step 7. Subject: `feat(std/crypto): wire three intrinsics through the compiler seam`. The body records the measured site count against the predicted one, that `--all-targets` was or was not load-bearing here, and that all three builtins are `STD_ONLY`.

---

## Task 3: The Nova module and its fixtures

**Files:**
- Create: `std/crypto/lib.nova`, `tests/runtime/crypto_hashes.nova`, `tests/runtime/crypto_hashes.stdout`, `tests/runtime/crypto_random.nova`, `tests/runtime/crypto_random.stdout`
- Modify: `crates/nova-resolver/src/lib.rs` (the `STD_MODULES` entry), `crates/nova-cli/tests/run_tests.rs`

**Interfaces:**
- Consumes: the three builtins from Task 2; `decode_count` from `std/io`; `fs_take_bytes` from the runtime; `bytes_from_ints`, `Bytes::len`, `Bytes::slice`, `Bytes::eq` from `std/bytes`.
- Produces: the surface in the spec's section 4.

- [ ] **Step 1: Write the module**

Create `std/crypto/lib.nova` with the **Write tool** — it contains `\r\n`-free but escape-bearing string literals and a heredoc would eat a backslash level.

```nova
// `std/crypto`: hashes, HMAC and random, over three runtime intrinsics.
//
// **Digest lengths are documented, not typed.** `nova-spec/20-STDLIB.md`
// section 8 declares these as `[u8; 32]` and `[u8; 64]`; neither is
// writable, because `u8` does not name a type and a fixed-length array type
// does not parse. So a digest is a `Bytes` whose length is stated in a
// comment, and nothing can force a caller to hold exactly 32 bytes.
//
// **What this module does not provide**, each recorded in section 8's dated
// amendment: BLAKE3, which the `ring` backing the same section names does
// not implement; AEAD; and any streaming or incremental hasher, so a caller
// with more data than fits in one `Bytes` has no route here.

pub type CryptoErrorKind =
    | EntropyUnavailable
    | InvalidLength
    | RequestTooLarge
    | InvalidRange

pub record CryptoError {
    pub kind: CryptoErrorKind
    pub message: String
}

// These four values are duplicated in `crates/nova-runtime/src/crypto.rs`.
// Nothing ties the two sides together, so the fixtures assert a known vector
// per operation: swapping two of these must fail a fixture.
const OP_SHA256: Int = 0
const OP_SHA512: Int = 1
const OP_HMAC_SHA256: Int = 2
const OP_HMAC_SHA256_VERIFY: Int = 3

fn crypto_error_kind_of(status: Int) -> CryptoErrorKind {
    match 0 - status {
        1 => EntropyUnavailable
        2 => InvalidLength
        3 => RequestTooLarge
        _ => InvalidRange
    }
}

fn no_bytes() -> Bytes { bytes_from_ints([]) }

// SHA-256 of `data`, 32 bytes.
//
// Infallible: the intrinsic's status is always 0 for this operation, because
// no input this surface can produce makes a digest fail.
pub fn sha256(data: Bytes) -> Bytes {
    let _ = crypto_hash(OP_SHA256, no_bytes(), data, no_bytes())
    fs_take_bytes()
}

// SHA-512 of `data`, 64 bytes. Infallible, for the reason `sha256` gives.
pub fn sha512(data: Bytes) -> Bytes {
    let _ = crypto_hash(OP_SHA512, no_bytes(), data, no_bytes())
    fs_take_bytes()
}

// HMAC-SHA-256 of `data` under `key`, 32 bytes. Infallible.
pub fn hmac_sha256(key: Bytes, data: Bytes) -> Bytes {
    let _ = crypto_hash(OP_HMAC_SHA256, key, data, no_bytes())
    fs_take_bytes()
}

// Whether `tag` is the HMAC-SHA-256 of `data` under `key`.
//
// **Constant-time, and that property is INHERITED from `ring::hmac::verify`
// rather than demonstrated here.** No test in this project observes timing.
// Prefer this over comparing tags yourself: `Bytes::eq` is available and
// exact, but it is not constant-time -- it returns early on a length
// mismatch and then calls `memcmp` -- so a hand-written check leaks.
pub fn hmac_sha256_verify(key: Bytes, data: Bytes, tag: Bytes) -> Bool {
    crypto_hash(OP_HMAC_SHA256_VERIFY, key, data, tag) == 0
}

// `n` cryptographically random bytes.
//
// Fallible where section 8 declared it infallible: the OS entropy source can
// fail, `n` can be negative or above the runtime's cap, and no panic may
// cross a generated poll boundary, so the alternative to a `Result` is
// ending the process.
pub fn random_bytes(n: Int) -> Result<Bytes, CryptoError> {
    let status = crypto_random_bytes(n)
    if status == 0 {
        return Ok(fs_take_bytes())
    }
    Err(CryptoError {
        kind: crypto_error_kind_of(status),
        message: "random_bytes failed",
    })
}

// A uniform integer in `min ..= max`.
//
// `min == max` is a valid degenerate range and returns `min`; a range with
// `min` above `max` is `InvalidRange`.
pub fn random_int(min: Int, max: Int) -> Result<Int, CryptoError> {
    let status = crypto_random_int(min, max)
    if status == 0 {
        return Ok(decode_count(fs_take_bytes()))
    }
    Err(CryptoError {
        kind: crypto_error_kind_of(status),
        message: "random_int failed",
    })
}
```

**Two things to verify before moving on**, because neither is measured on this tree: that `crypto_hash(...) == 0` type-checks as a `Bool` (an `Int` comparison, which should be fine, but `==` on `Bytes` is `E0013` so operator support is uneven), and that `decode_count` is reachable from `std/crypto` — it lives in `std/io` and ADR 0004 glob-imports std, but the `STD_MODULES` ordering may matter. Report any diagnostic rather than working around it.

- [ ] **Step 2: Register the module**

In `crates/nova-resolver/src/lib.rs`, append to `STD_MODULES` after `$std.http` and bump the array's length annotation from 14 to 15:

```rust
    ("$std.crypto", include_str!("../../../std/crypto/lib.nova")),
```

Append rather than insert: every module so far was added at the end, and the array's order is the seeding order.

- [ ] **Step 3: Write the failing fixtures**

Create `tests/runtime/crypto_hashes.nova` with the Write tool:

```nova
// Known-vector fixture for `std/crypto`'s digests.
//
// **Provenance:** every expected value here was cross-checked against
// Python's `hashlib`/`hmac` before being written down. The SHA-256 of "abc"
// and the HMAC case are also widely published, as FIPS 180-4's worked
// example and RFC 4231's first case; **this project has verified the
// cross-check, not the documents**, and states which claim it is making so a
// later reader can upgrade it.
//
// This fixture is what pins the operation constants: `OP_SHA256` and
// `OP_SHA512` are duplicated between `std/crypto/lib.nova` and
// `crates/nova-runtime/src/crypto.rs`, and swapping either pair makes the
// digests below wrong.

fn hex(b: Bytes) -> String {
    let digits = "0123456789abcdef"
    let ints = b.to_ints()
    let mut out = ""
    let mut i = 0
    while i < ints.len() {
        let v = ints[i]
        out = "${out}${digits.char_at(v / 16)}${digits.char_at(v % 16)}"
        i = i + 1
    }
    out
}

fn main() {
    println("sha256 empty ${hex(sha256(bytes_from_string("")))}")
    println("sha256 abc ${hex(sha256(bytes_from_string("abc")))}")
    println("sha512 abc len ${sha512(bytes_from_string("abc")).len()}")
    println("sha512 abc head ${hex(sha512(bytes_from_string("abc")).slice(0, 8))}")

    let key = bytes_from_ints([11, 11, 11, 11, 11, 11, 11, 11, 11, 11, 11, 11, 11, 11, 11, 11, 11, 11, 11, 11])
    let data = bytes_from_string("Hi There")
    let tag = hmac_sha256(key, data)
    println("hmac ${hex(tag)}")
    println("verify good ${hmac_sha256_verify(key, data, tag)}")
    println("verify wrong data ${hmac_sha256_verify(key, bytes_from_string("Hi there"), tag)}")
    println("verify truncated ${hmac_sha256_verify(key, data, tag.slice(0, 31))}")
    println("verify empty tag ${hmac_sha256_verify(key, data, bytes_from_ints([]))}")
}
```

**`digits.char_at(...)` is a guess** — check `std/strings` for the real accessor and adapt. If none exists, index a `[String]` built with `bytes_from_ints`, or emit the digest as its `to_ints()` list instead of hex; a golden over integers pins the vector just as well and is less machinery. Report which route you took.

Its golden, `tests/runtime/crypto_hashes.stdout`:

```
sha256 empty e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855
sha256 abc ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad
sha512 abc len 64
sha512 abc head ddaf35a193617aba
hmac b0344c61d8db38535ca8afceaf0bf12b881dc200c9833da726e9376c2e32cff7
verify good true
verify wrong data false
verify truncated false
verify empty tag false
```

Create `tests/runtime/crypto_random.nova`:

```nova
// `std/crypto`'s entropy surface. **No expected value can be asserted**, so
// this asserts the properties that hold whatever the draw is.
//
// The differing-draws check is probabilistic: at 32 bytes a repeat has
// probability around 2 to the negative 256, so it is astronomically safe,
// and it is what catches an entropy source stuck at a constant.

fn kind_name(k: CryptoErrorKind) -> String {
    match k {
        EntropyUnavailable => "EntropyUnavailable"
        InvalidLength => "InvalidLength"
        RequestTooLarge => "RequestTooLarge"
        InvalidRange => "InvalidRange"
    }
}

fn main() {
    match random_bytes(32) {
        Ok(a) => println("len ${a.len()}")
        Err(e) => println("unexpected ${kind_name(e.kind)}")
    }
    match random_bytes(0) {
        Ok(a) => println("zero len ${a.len()}")
        Err(e) => println("unexpected ${kind_name(e.kind)}")
    }
    match random_bytes(0 - 1) {
        Ok(a) => println("negative unexpectedly ok ${a.len()}")
        Err(e) => println("negative ${kind_name(e.kind)}")
    }
    match random_bytes(65537) {
        Ok(a) => println("over cap unexpectedly ok ${a.len()}")
        Err(e) => println("over cap ${kind_name(e.kind)}")
    }

    // Two draws must differ. Compared with `Bytes::eq`, which is exact and
    // is fine here: this is not a tag check, so constant time is irrelevant.
    let x = match random_bytes(32) {
        Ok(v) => v
        Err(e) => bytes_from_ints([])
    }
    let y = match random_bytes(32) {
        Ok(v) => v
        Err(e) => bytes_from_ints([])
    }
    println("draws differ ${!x.eq(y)}")

    match random_int(5, 5) {
        Ok(v) => println("degenerate ${v}")
        Err(e) => println("unexpected ${kind_name(e.kind)}")
    }
    match random_int(10, 1) {
        Ok(v) => println("inverted unexpectedly ok ${v}")
        Err(e) => println("inverted ${kind_name(e.kind)}")
    }

    let mut i = 0
    let mut in_range = true
    while i < 200 {
        match random_int(0 - 3, 3) {
            Ok(v) => {
                if v < (0 - 3) {
                    in_range = false
                }
                if v > 3 {
                    in_range = false
                }
            }
            Err(e) => in_range = false
        }
        i = i + 1
    }
    println("bounded ${in_range}")
}
```

Its golden, `tests/runtime/crypto_random.stdout`:

```
len 32
zero len 0
negative InvalidLength
over cap RequestTooLarge
draws differ true
degenerate 5
inverted InvalidRange
bounded true
```

Then register both in `crates/nova-cli/tests/run_tests.rs`, matching the shape of the existing `http_*` fixture tests — read one first.

- [ ] **Step 4: Run the fixtures to verify they fail**

```bash
cargo test --locked --package nova-cli crypto_ --no-fail-fast
```

Expected: FAIL. Before the module exists it fails on unknown names; if you wrote the module first it may fail on a Nova diagnostic instead. **Report which, and if it is a diagnostic, report its code and message** — that is information about the language, not just about this task.

- [ ] **Step 5: Run them to verify they pass**

```bash
cargo build --locked --workspace && cargo test --locked --package nova-cli crypto_ --no-fail-fast
```

Expected: PASS. **Run the random fixture at least five times** and report how many passed: it is the increment's exposure to the known Windows flake, and a single green run is not evidence of stability.

- [ ] **Step 6: Full suite, lint, format, byte-scan, commit**

As Task 1's Step 7. Byte-scan the staged fixtures and the module: `std/crypto/lib.nova` and both `.nova` fixtures must contain **zero real CR bytes**.

Subject: `feat(std/crypto): ship hashes, HMAC and random as a Nova module`. The body records the vectors' provenance and its limit, that digest lengths are documented rather than typed and why, that the constant-time property is inherited, how many fixture runs passed, and the `STD_MODULES` length going to 15.

---

## Task 4: The mutations and the records

**Files:**
- Modify: `nova-spec/20-STDLIB.md`, `nova-spec/00-MASTER-SPEC.md`, `docs/adr/0018-std-json-scope-and-build-order.md`, `docs/adr/0019-offset-table-intrinsic-boundary.md`, `CHANGELOG.md`

**Interfaces:**
- Consumes: everything Tasks 1 through 3 shipped, including the measured intrinsic site count and the fixture-run count.
- Produces: nothing.

- [ ] **Step 1: Run the four remaining mutations, and report what each actually did**

Task 1 already ran the reduction mutation. Run these four, reverting after each and confirming green:

1. **Swap `OP_SHA256` and `OP_SHA512` in `std/crypto/lib.nova`.** The hashes fixture must fail. This is what pins the constants; that the functions run does not.
2. **Swap them in `crates/nova-runtime/src/crypto.rs` instead.** The same fixture must fail, from the other side.
3. **Make `sha256` return SHA-512's digest** by changing its `digest::SHA256` to `digest::SHA512`. The fixture must fail on the digest and on the length.
4. **Replace `ring::hmac::verify` with `tag == expected`-style comparison** — compute the HMAC and compare the slices directly. **The correctness tests are expected to keep passing**, because they assert only which tags are accepted and both implementations accept exactly the same tags. Run it, and report what actually happened. If something does fail, that is more interesting than the prediction.

Mutation 4's expected survival is the honest measure of what this suite covers, and the reason the constant-time property is described as inherited rather than tested. **Report it as surviving; do not omit it.**

A mutation whose failure mode is a hang rather than a failed assertion must be **observed** hanging and killed, not recorded as a clean failure.

- [ ] **Step 2: Amend `nova-spec/20-STDLIB.md`'s entropy paragraph**

**This is the record most easily amended wrongly. Read the whole passage first** — its headline and its later lines disagree by design.

Its headline claim is "no runtime function exposes entropy to Nova". **That was already false before this increment**, and the same passage says so further down: "There IS a new route from Nova to entropy, and it is stated here rather than denied ... `str_hash` is the route", because `("").hash()` recovers the per-process seed through an invertible finalizer. So `std/crypto` is **not** the first breach, and a marker crediting it with that would be a new false claim.

What this increment does falsify is that passage's present-tense inventory: "`random_bytes` and `random_int` in §8 below are unstarted declarations, with no `ring` in `Cargo.lock` and no `std/crypto/` directory" — three clauses, all three of which change. What it adds beyond the existing route is a **deliberate, per-call entropy surface**, as distinct from a seed recoverable as a side effect of hashing. That distinction is the amendment's content.

The same passage carries a separate stale clause saying `std/collections` "has no seed to hand a `Hasher`", which the seeded-mix64 work already falsified. Note it as separately stale **without** claiming this increment falsified it.

Use the file's own convention: `**AMENDED 2026-09-03 (branch \`std-crypto-hashes-hmac-random\`): <what was wrong>.**` then the correction as prose. **Amend; never rewrite.** ADR 0018 states that convention in its own words: wording is "left as written and superseded by this marker rather than edited".

- [ ] **Step 3: Amend the rest of the records**

- **`nova-spec/20-STDLIB.md` section 8** — that its listing is written in types this language does not have, naming `u8` as `E0001` and the fixed-length array type as `P0001`; that `blake3` cannot come from the `ring` backing the same section names; that `CryptoError` is used without declaration and is now declared; that it declared a tag producer with no consumer, and a verifier ships anyway because the available comparison is not constant-time; and that AEAD remains unstarted.
- **`nova-spec/00-MASTER-SPEC.md` section 3** — that position 12 is now partially built, naming what ships and what does not. **Cite it by heading, not by line number**: that section's numbering shifted twice in the preceding week, and it carries a note explaining why a line number is not a durable citation there.
- **`nova-spec/20-STDLIB.md`'s Phase 2 status notes** — several say position 12 is unstarted with no `ring` in `Cargo.lock` and no `std/crypto/` directory. All three clauses change.
- **`docs/adr/0018-...` and `docs/adr/0019-...`** — both carry present-tense claims that `std/crypto` is the one unstarted Phase 2 module group.
- **`CHANGELOG.md`** under `[Unreleased]` — the surface; the dependency and the lockfile entries as actually observed; the measured intrinsic site count against ADR 0018's predicted arithmetic; the vectors' provenance and its limit; that the constant-time property is inherited and mutation 4 survived; that digest lengths are documented rather than typed; and that AEAD and BLAKE3 are unstarted.

- [ ] **Step 4: Sweep for claims this increment made stale**

grep is line-oriented, so **a miss is not evidence of absence** — sweep with whitespace-tolerant patterns that also normalise `//`, `///` and `>` gutters. A plain grep returned zero hits on a phrase that existed, wrapped, twice on earlier branches.

Sweep `nova-spec/`, `docs/`, `std/`, `crates/` and `CHANGELOG.md` for: any sentence saying `std/crypto` is unstarted or that no `std/crypto/` directory exists; any claim that Nova cannot obtain a random value or that no runtime function exposes entropy; any claim that `ring` is absent from `Cargo.lock`; any bare count of `STD_MODULES` entries or of workspace crates; and any roster of std modules this increment now contradicts.

**Fix every artifact the sweep names, not only the one you happen to be editing.** Report what you searched for with the patterns, what you found, and what you changed. If a category returns nothing, **say which pattern you used** so a reader can judge whether the miss is meaningful.

- [ ] **Step 5: Byte-scan and commit**

Byte-scan every file written, from staged content, including the planted-positive assertion for the backslash-u pattern. Confirm no branch-local SHA appears in any tracked file, deriving the roster with `git log --format=%h main..HEAD`. Then commit.

Subject: `docs(std/crypto): amend the records for what position 12 now ships`. The body says which spec sections were amended and how, that the entropy paragraph's headline was already false before this increment and what was actually corrected, what the sweep found, and that mutation 4 survived as expected.

---

## Self-Review

Run against the spec after the plan was complete.

**1. Spec coverage.** Section 1 (what ships and what is declined): Task 3's module header and Task 4's section 8 amendment. Section 2 (unwritable types): Task 3's module header and Task 4. Section 3 (the backing, the contradiction, the lockfile delta): Task 1 Step 1 and Task 4's CHANGELOG entry. Section 4 (the surface): Task 3 Step 1. Section 5 (the boundary, the op constants, the cap, panic discipline): Task 1 Steps 4 and 2, and Task 2. Section 6 (error kinds): Task 1's constants and Task 3's `crypto_error_kind_of`. Section 7 (vectors and provenance, the pure reduction, the live-span test, what is not tested, the five mutations): Task 1 Steps 2 and 6, Task 3 Step 3, Task 4 Step 1. Section 8 (records): Task 4 Steps 2 and 3. Section 9 (success criteria): every task's final step, with criteria 4 and 8 explicitly assigned to Task 1 Step 6 and Task 2 Step 6. Section 10 (out of scope): nothing implements it, which is correct.

**One gap found and closed:** the spec's section 7 requires a live-source test at a wide span alongside the crafted-input one, and the first pass of this plan had only the latter. Task 1's Step 2 now carries `a_wide_span_reaches_the_rejection_branch_from_the_live_source`, which draws forty times at a span of `2^63 + 1` — where the rejection probability is one half — and asserts both a rejection and an acceptance occurred. Written into the plan rather than described, because a step that says what to add without showing it is a plan failure by this skill's own rule.

**2. Placeholder scan.** No "TBD" or "handle errors appropriately" survives. Three places deliberately say "this is a guess, check the file": `MirTy::I64`'s name, the `assert_lowers_to` helper's signature, and `digits.char_at`. Each names the authority to check against and what to do instead, which is different from a placeholder — the alternative is asserting a name I have not read.

**3. Type consistency.** `crypto_hash`, `crypto_random_bytes` and `crypto_random_int` are spelled identically in Task 2's lowering test, Task 2's site table and Task 3's module. `OP_SHA256` through `OP_HMAC_SHA256_VERIFY` are spelled identically in Task 1's Rust and Task 3's Nova, with the same values. `CryptoErrorKind`'s four variants match between Task 1's `ERR_*` numbering (1 through 4) and Task 3's `crypto_error_kind_of` mapping. `fs_take_bytes` and `decode_count` are used as Task 3's interface block declares them.
