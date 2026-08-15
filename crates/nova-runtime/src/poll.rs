//! The executor's third wake source: socket readiness.
//!
//! Before this module existed, `task.rs`'s executor had exactly two wake
//! sources -- a deadline passing and a task completing -- and "nothing left
//! to run" meant sleeping the whole thread on the earliest deadline
//! (`std::thread::sleep`, called directly from `task.rs`). This module gives
//! `task.rs` a single place to ask "wake me when a deadline passes, a socket
//! becomes ready, or whichever comes first", so the executor no longer knows
//! how to wait -- it only knows how to ask [`wait`] to do it.
//!
//! **This module ships no I/O.** [`RawSocket`] and [`Interest`] exist so
//! `task.rs`'s `Wait::Io` variant has something to name a socket and an
//! interest with, and [`wait`] behaves exactly like the old
//! `thread::sleep`-based timer path for the one socket set every caller in
//! this task passes it: the empty one. Task 2 gives this module a real
//! poller and makes the non-empty branch below do something; nothing in
//! `task.rs` has to change again when it does, because `task.rs` already
//! only calls [`wait`] and never sleeps itself.

use std::time::Instant;

/// A socket as the poller sees it: the OS handle, widened to `i64` so
/// `task.rs` can hold one in `Wait` without depending on this module's
/// platform types, and so a future `net.rs` (Task 3) and `task.rs` agree on
/// one socket representation without either depending on the other's.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[repr(transparent)]
pub struct RawSocket(pub i64);

/// What a task is waiting for a socket to become ready for.
///
/// `#[allow(dead_code)]`: this task ships no I/O, so nothing outside this
/// module's own tests constructs either variant yet -- `net.rs` (Task 3) is
/// this enum's first production caller. The seam is added now, not deleted
/// and re-added later, so `Wait::Io` (`task.rs`) has a real type to name an
/// interest with today.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[allow(dead_code)]
pub enum Interest {
    Read,
    Write,
}

/// Wait until one of `sockets` becomes ready for its paired [`Interest`], or
/// `deadline` passes, whichever happens first. Returns every socket found
/// ready; empty if the deadline fired instead, or if `sockets` is empty.
///
/// **The empty-set case is the only one this task implements**, and it is
/// exactly the old timer path: sleep until `deadline` if one was given (or
/// return immediately if it is already due or absent), then hand back no
/// ready sockets, since there were none to begin with. `task.rs`'s drive loop
/// reaches this branch both for "nothing is parked on I/O at all" (the
/// ordinary `sleep`/`join` case) and for a deadline-only park set -- see its
/// own doc comment for the `(Some(at), true)` and `(None, true)` cases.
///
/// The non-empty branch is Task 2's: nothing in this task ever builds a
/// non-empty `sockets` slice (this task ships no I/O), so it is exercised by
/// this module's own direct tests only, never by `task.rs`.
pub fn wait(sockets: &[(RawSocket, Interest)], deadline: Option<Instant>) -> Vec<RawSocket> {
    if sockets.is_empty() {
        if let Some(at) = deadline {
            let now = Instant::now();
            if at > now {
                std::thread::sleep(at - now);
            }
        }
        return Vec::new();
    }
    // Task 2 replaces this with a real readiness wait.
    Vec::new()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_empty_socket_set_with_no_deadline_returns_immediately_with_nothing_ready() {
        let ready = wait(&[], None);
        assert!(
            ready.is_empty(),
            "no sockets and no deadline must not block or report anything ready"
        );
    }

    #[test]
    fn an_empty_socket_set_with_a_past_deadline_returns_immediately_with_nothing_ready() {
        let past = Instant::now();
        let ready = wait(&[], Some(past));
        assert!(
            ready.is_empty(),
            "a due deadline must not sleep, and an empty socket set has \
             nothing to report ready regardless"
        );
    }

    #[test]
    fn a_non_empty_socket_set_reports_nothing_ready_this_task() {
        let ready = wait(&[(RawSocket(3), Interest::Read)], None);
        assert!(
            ready.is_empty(),
            "this task ships no poller: a non-empty socket set must still \
             report nothing ready rather than fabricate readiness"
        );
    }
}
