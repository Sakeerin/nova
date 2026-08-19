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
/// The conversion `task.rs` encodes its sleep/timeout deadlines against, so a
/// deadline fits an `i64` state slot, which a `std::time::Instant` cannot: it
/// has no documented byte layout.
///
/// **Not the only clock-reading-to-`i64` conversion in this crate**, scoped
/// the same way [`epoch`]'s own doc comment scopes itself: `net.rs`'s
/// `now_epoch_ms` does the identical kind of conversion, in milliseconds
/// rather than nanoseconds, against its own separate origin
/// (`net.rs`'s `deadline_epoch`) for `read_timeout`'s unrelated deadline
/// arithmetic. This doc comment speaks only for readings taken through
/// [`epoch`].
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

/// Nanoseconds since the Unix epoch, saturating at `i64::MAX`.
///
/// **Deliberately does not reuse [`now_nanos`] or [`epoch`].** Those read
/// from a process-start `OnceLock` and answer "how long has this process
/// been running"; this answers "what time is it". Same unit, same width,
/// different quantity — which is exactly the shape of mistake that forced
/// the `SLEEP_SLOT_MS` → `SLEEP_SLOT_NANOS` → `SLEEP_SLOT_DEADLINE_NANOS`
/// renames, twice, because the type never changed while the meaning did.
///
/// A clock set before 1970 makes `duration_since` fail; that returns `0`
/// rather than a negative value, so the calendar math downstream never has
/// to interpret one.
#[no_mangle]
pub extern "C-unwind" fn nova_rt_time_now_epoch_nanos() -> i64 {
    match std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH) {
        Ok(d) => i64::try_from(d.as_nanos()).unwrap_or(i64::MAX),
        Err(_) => 0,
    }
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

    /// The wall clock reads from the Unix epoch, not from this module's
    /// process epoch. A reading taken now must sit after 2026-01-01 and
    /// before 2100-01-01 — a window wide enough to never be flaky and narrow
    /// enough to fail if the reading is actually a process-relative value,
    /// which would be a handful of milliseconds.
    #[test]
    fn the_wall_clock_reads_from_the_unix_epoch_not_the_process_epoch() {
        let n = nova_rt_time_now_epoch_nanos();
        assert!(
            n > 1_767_225_600_000_000_000,
            "wall clock returned {n}, which is before 2026-01-01; a process-relative reading looks like this"
        );
        assert!(
            n < 4_102_444_800_000_000_000,
            "wall clock returned {n}, after 2100-01-01"
        );
    }
}
