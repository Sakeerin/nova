//! One scenario, one process: the Windows console handler sets the shutdown
//! flag on the first CTRL_C, and lets every later event fall through to the
//! default action. Its own test binary for the reason `signal_sigterm.rs`
//! gives. The routine is called directly: delivering a real console event to
//! this process would also reach cargo.
#![cfg(windows)]

use nova_runtime::signal::{console_handler, nova_rt_shutdown_requested};

#[test]
fn ctrl_c_sets_the_flag_and_a_later_event_falls_through() {
    assert_eq!(
        nova_rt_shutdown_requested(),
        0,
        "the first read installs the handler and finds the flag clear"
    );
    assert_eq!(
        console_handler(0),
        1,
        "CTRL_C_EVENT is handled the first time"
    );
    assert_eq!(nova_rt_shutdown_requested(), 1, "and it set the flag");
    assert_eq!(
        console_handler(1),
        0,
        "CTRL_BREAK_EVENT with the flag already set falls through to the default action"
    );
    assert_eq!(
        console_handler(2),
        0,
        "CTRL_CLOSE_EVENT always falls through"
    );
}
