//! One scenario, one process: a SIGINT that was not ignored sets the shutdown
//! flag without ending the process. Its own test binary for the reason
//! `signal_sigterm.rs` gives.
#![cfg(unix)]

use nova_runtime::signal::nova_rt_shutdown_requested;

#[test]
fn sigint_sets_the_flag_when_it_was_not_ignored() {
    // SAFETY: setting a disposition to `SIG_DFL` touches no memory.
    unsafe { libc::signal(libc::SIGINT, libc::SIG_DFL) };
    assert_eq!(
        nova_rt_shutdown_requested(),
        0,
        "the first read installs the handler and finds the flag clear"
    );

    // SAFETY: SIGINT now has a handler that only stores an atomic.
    unsafe { libc::raise(libc::SIGINT) };
    assert_eq!(
        nova_rt_shutdown_requested(),
        1,
        "SIGINT sets the flag, and the process survives it"
    );
}
