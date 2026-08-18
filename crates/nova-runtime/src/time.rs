//! Monotonic clock readings for `std/time`.
//!
//! One origin and one intrinsic for this module's own readings, deliberately
//! -- `net.rs` keeps a second, separate origin of its own, for unrelated
//! reasons (see [`epoch`]'s doc comment). `Instant` and `Duration`
//! arithmetic is Nova code in `std/time/lib.nova` -- a subtraction does not
//! need to cross into Rust. This module exists only because a clock reading
//! is the one thing in §9 that Nova cannot express at all.

use std::sync::OnceLock;
use std::time::Instant;

/// The process-monotonic origin every reading taken *through this module* is
/// relative to, initialized on first read so no reading is ever negative.
///
/// **Shared with `poll.rs`'s log rate limiter, not with the whole process.**
/// `poll.rs` used to own a second `OnceLock<Instant>` of its own (`log_epoch`)
/// and now reads this one instead -- two independent origins that merely
/// happened to agree was worse than one named for what it is. That fold does
/// not make this the process's only origin, though: `net.rs`'s own
/// `deadline_epoch` is a second, separate `OnceLock<Instant>` that survives,
/// kept for `read_timeout`'s deadline arithmetic by a deliberate choice
/// recorded there. The process has two monotonic origins by design; this doc
/// comment speaks only for this one.
pub(crate) fn epoch() -> Instant {
    static EPOCH: OnceLock<Instant> = OnceLock::new();
    *EPOCH.get_or_init(Instant::now)
}

/// Nanoseconds since [`epoch`], saturating at `i64::MAX`.
///
/// The one place a clock reading becomes an `i64`. `task.rs` encodes deadlines
/// against this so a deadline fits an `i64` state slot, which a
/// `std::time::Instant` cannot: it has no documented byte layout.
///
/// **Saturates rather than wrapping.** `as_nanos` is `u128`, and a bare
/// `as i64` would wrap after roughly 292 years of process uptime. A wrapped
/// reading is negative, `std/time` computes `Duration`s from it by
/// subtraction, and `sleep` treats a non-positive duration as an immediate
/// wake -- so a wrap would turn a very long sleep into no sleep at all. A
/// clamp keeps a useless answer from becoming a wrong one.
pub(crate) fn now_nanos() -> i64 {
    i64::try_from(epoch().elapsed().as_nanos()).unwrap_or(i64::MAX)
}

#[no_mangle]
pub extern "C-unwind" fn nova_rt_time_now_nanos() -> i64 {
    now_nanos()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// One origin, not a fresh reading per call. If `epoch` returned
    /// `Instant::now()` directly, every reading would be ~0 and `elapsed`
    /// would always be zero -- which no other test here would notice.
    #[test]
    fn the_epoch_is_a_single_origin() {
        assert_eq!(epoch(), epoch());
    }

    /// A reading must advance by at least as much wall time as really passed.
    ///
    /// This is the assertion the earlier version of this test lacked: a
    /// non-negativity check cannot fail, because `as_nanos` is `u128` and the
    /// `unwrap_or(i64::MAX)` fallback is positive, so it held for any
    /// implementation at all -- including one returning a constant. Sleeping a
    /// real interval and requiring the delta to cover it kills a frozen clock,
    /// a constant, and a reading taken from the wrong origin, while still not
    /// being able to flake: the clock is monotonic and `sleep` guarantees a
    /// lower bound, so the delta can only be larger than the interval.
    #[test]
    fn a_reading_advances_by_at_least_the_time_that_passed() {
        let slept = std::time::Duration::from_millis(2);
        let first = nova_rt_time_now_nanos();
        std::thread::sleep(slept);
        let second = nova_rt_time_now_nanos();
        let delta = second - first;
        assert!(
            delta >= i64::try_from(slept.as_nanos()).expect("2ms fits in i64"),
            "a {slept:?} sleep advanced the clock by only {delta}ns"
        );
    }
}
