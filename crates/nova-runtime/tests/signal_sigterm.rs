//! One scenario, one process: SIGTERM sets the shutdown flag without ending
//! the process, and a SIGINT the process already ignored stays ignored.
//!
//! Its own test binary because the handler's install and the flag are
//! process-global and permanent (see `nova_runtime::signal`), so no other
//! scenario may share this process.
#![cfg(unix)]

use nova_runtime::signal::nova_rt_shutdown_requested;

#[test]
fn sigterm_sets_the_flag_and_an_ignored_sigint_stays_ignored() {
    // SAFETY: setting a disposition to `SIG_IGN` touches no memory.
    unsafe { libc::signal(libc::SIGINT, libc::SIG_IGN) };
    assert_eq!(
        nova_rt_shutdown_requested(),
        0,
        "the first read installs the handler and finds the flag clear"
    );

    // SAFETY: `old` is a valid, writable `sigaction` for the call's duration.
    let mut old: libc::sigaction = unsafe { std::mem::zeroed() };
    unsafe { libc::sigaction(libc::SIGINT, std::ptr::null(), &mut old) };
    assert_eq!(
        old.sa_sigaction,
        libc::SIG_IGN,
        "an inherited SIGINT ignore must survive the install"
    );

    // SAFETY: SIGTERM now has a handler that only stores an atomic.
    unsafe { libc::raise(libc::SIGTERM) };
    assert_eq!(
        nova_rt_shutdown_requested(),
        1,
        "SIGTERM sets the flag, and the process survives it"
    );
}
