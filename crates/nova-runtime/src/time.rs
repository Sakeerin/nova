//! Monotonic clock readings for `std/time`.
//!
//! One origin and one intrinsic, deliberately. `Instant` and `Duration`
//! arithmetic is Nova code in `std/time/lib.nova` -- a subtraction does not
//! need to cross into Rust. This module exists only because a clock reading
//! is the one thing in §9 that Nova cannot express at all.

use std::sync::OnceLock;
use std::time::Instant;

/// The single process-monotonic origin every reading is relative to,
/// initialized on first read so no reading is ever negative.
///
/// **Shared on purpose.** `poll.rs`'s log rate limiter reads this same origin;
/// it used to own a second `OnceLock<Instant>` of its own (`log_epoch`). Two
/// independent origins that merely happen to agree is worse than one named for
/// what it is.
pub(crate) fn epoch() -> Instant {
    static EPOCH: OnceLock<Instant> = OnceLock::new();
    *EPOCH.get_or_init(Instant::now)
}

/// Nanoseconds since [`epoch`], saturating at `i64::MAX`.
///
/// **Saturates rather than wrapping.** `as_nanos` is `u128`, and a bare
/// `as i64` would wrap after roughly 292 years of process uptime. A wrapped
/// reading is negative, `std/time` computes `Duration`s from it by
/// subtraction, and `sleep` treats a non-positive duration as an immediate
/// wake -- so a wrap would turn a very long sleep into no sleep at all. A
/// clamp keeps a useless answer from becoming a wrong one.
#[no_mangle]
pub extern "C-unwind" fn nova_rt_time_now_nanos() -> i64 {
    i64::try_from(epoch().elapsed().as_nanos()).unwrap_or(i64::MAX)
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

    #[test]
    fn readings_are_non_negative_and_non_decreasing() {
        let first = nova_rt_time_now_nanos();
        let second = nova_rt_time_now_nanos();
        assert!(
            first >= 0,
            "a reading relative to the process epoch cannot be negative: {first}"
        );
        assert!(
            second >= first,
            "monotonic clock went backwards: {second} < {first}"
        );
    }
}
