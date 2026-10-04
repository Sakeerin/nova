//! The process-wide shutdown flag behind `std/http`'s graceful
//! `Server::listen`.
//!
//! One `AtomicBool`, set by the first termination request the process
//! receives: SIGTERM or SIGINT on Unix, CTRL_BREAK or CTRL_C on Windows. A
//! request that arrives once the flag is already set takes the signal's
//! default action instead, so a stalled shutdown can always be ended.
//!
//! **Opt-in, and permanent once in.** Nothing is installed until the first
//! call to [`nova_rt_shutdown_requested`], which only `std/http`'s `Server`
//! makes: the builtin `shutdown_requested` is `STD_ONLY`. A program that never
//! calls it keeps every signal's default action. Once installed, the handler
//! stays for the life of the process, and the flag is never cleared.
//!
//! **The handler touches nothing but [`SHUTDOWN`].** On Unix it runs in signal
//! context, where only async-signal-safe calls are allowed. On Windows the OS
//! runs it on a thread of its own, where this crate's `thread_local!` state
//! (the executor, the socket table, the heap) is a different, empty copy. So
//! it never wakes the executor: `poll.rs` retries an interrupted wait, and Nova
//! code sees the flag at its next deadline. `Server::listen` provides one
//! every 100 ms. `docs/adr/0022-process-shutdown-signals.md` records the
//! decision and the alternatives it rejected.
//!
//! **An inherited ignore stays ignored.** A process started with SIGINT
//! ignored (a job backgrounded by a non-interactive shell) keeps ignoring it.
//! Windows' inherited Ctrl-C ignore flag keeps CTRL_C from reaching any
//! handler at all. SIGTERM and CTRL_BREAK are not affected.
//!
//! `pub`, unlike [`crate::time`], so the integration tests under
//! `crates/nova-runtime/tests/` can drive it. Each process-global scenario
//! there is its own test binary, because neither the install nor the flag can
//! be undone within one process.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Once;

/// Whether the process has been asked to stop.
static SHUTDOWN: AtomicBool = AtomicBool::new(false);

/// What a handler does with one termination request.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Action {
    /// The first request: set [`SHUTDOWN`] and carry on.
    SetFlag,
    /// A later one: take the signal's default action.
    Default,
}

/// The first request sets the flag; any later one takes the default action.
fn action(already_set: bool) -> Action {
    if already_set {
        Action::Default
    } else {
        Action::SetFlag
    }
}

/// Windows console control events, numbered as `windows-sys` numbers them
/// (checked against it at compile time below). Plain constants, so the
/// decision logic's unit tests run on every operating system.
#[cfg(any(windows, test))]
const CTRL_C: u32 = 0;
#[cfg(any(windows, test))]
const CTRL_BREAK: u32 = 1;

#[cfg(windows)]
const _: () = {
    use windows_sys::Win32::System::Console::{CTRL_BREAK_EVENT, CTRL_C_EVENT};
    assert!(CTRL_C == CTRL_C_EVENT && CTRL_BREAK == CTRL_BREAK_EVENT);
};

/// Whether a console control event asks for a graceful shutdown. Close,
/// logoff and shutdown events keep the default action.
#[cfg(any(windows, test))]
fn is_termination_event(event: u32) -> bool {
    event == CTRL_C || event == CTRL_BREAK
}

/// `shutdown_requested() -> Bool`: installs the handler on the first call,
/// then reports whether the flag is set, as `1` or `0`.
#[no_mangle]
pub extern "C" fn nova_rt_shutdown_requested() -> i8 {
    install();
    i8::from(SHUTDOWN.load(Ordering::SeqCst))
}

/// Installs the handler, once per process.
fn install() {
    static INSTALL: Once = Once::new();
    INSTALL.call_once(install_handler);
}

#[cfg(unix)]
fn install_handler() {
    // SAFETY: an all-zero `sigaction` is a valid value (no flags, empty mask,
    // `SIG_DFL`). Every field set below is a plain integer, or the address of
    // `unix_handler`, whose signature is the one `sa_sigaction` expects when
    // `SA_SIGINFO` is not set. `sigaction` reads `sa` and writes `old` only
    // for the duration of each call.
    unsafe {
        let mut sa: libc::sigaction = std::mem::zeroed();
        sa.sa_sigaction = unix_handler as extern "C" fn(libc::c_int) as libc::sighandler_t;
        sa.sa_flags = libc::SA_RESTART;
        libc::sigemptyset(&mut sa.sa_mask);
        libc::sigaction(libc::SIGTERM, &sa, std::ptr::null_mut());
        let mut old: libc::sigaction = std::mem::zeroed();
        libc::sigaction(libc::SIGINT, std::ptr::null(), &mut old);
        if old.sa_sigaction != libc::SIG_IGN {
            libc::sigaction(libc::SIGINT, &sa, std::ptr::null_mut());
        }
    }
}

/// The SIGTERM and SIGINT handler. Only async-signal-safe work: an atomic
/// swap and, on a second signal, `signal` and `raise`, both on POSIX's list.
#[cfg(unix)]
extern "C" fn unix_handler(sig: libc::c_int) {
    if action(SHUTDOWN.swap(true, Ordering::SeqCst)) == Action::Default {
        // SAFETY: restores `sig`'s default disposition and raises it again.
        // `sig` is blocked while this handler runs, so the raised signal is
        // delivered as the handler returns, with the default action.
        unsafe {
            libc::signal(sig, libc::SIG_DFL);
            libc::raise(sig);
        }
    }
}

#[cfg(windows)]
fn install_handler() {
    use windows_sys::Win32::System::Console::SetConsoleCtrlHandler;
    // SAFETY: `console_handler` has the `PHANDLER_ROUTINE` signature, and as a
    // plain function it outlives the registration.
    let _ = unsafe {
        SetConsoleCtrlHandler(
            Some(console_handler as unsafe extern "system" fn(u32) -> windows_sys::core::BOOL),
            1,
        )
    };
}

/// The console control routine [`install`] registers on Windows. It returns
/// TRUE (handled) for the first CTRL_C or CTRL_BREAK and FALSE for anything
/// else, so the next routine runs instead: the default one, which ends the
/// process with `0xC000013A`.
///
/// The OS calls it on a thread of its own. It touches only [`SHUTDOWN`], an
/// atomic, so it is safe on any thread. `pub` so `tests/signal_console.rs` can
/// call it directly.
#[cfg(windows)]
pub extern "system" fn console_handler(event: u32) -> windows_sys::core::BOOL {
    if !is_termination_event(event) {
        return 0;
    }
    i32::from(action(SHUTDOWN.swap(true, Ordering::SeqCst)) == Action::SetFlag)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_first_request_sets_the_flag_and_a_later_one_takes_the_default_action() {
        assert_eq!(action(false), Action::SetFlag);
        assert_eq!(action(true), Action::Default);
    }

    #[test]
    fn only_ctrl_c_and_ctrl_break_ask_for_a_graceful_shutdown() {
        // CTRL_CLOSE_EVENT, CTRL_LOGOFF_EVENT and CTRL_SHUTDOWN_EVENT are 2, 5
        // and 6 in `windows-sys`.
        for (event, expected) in [
            (CTRL_C, true),
            (CTRL_BREAK, true),
            (2, false),
            (5, false),
            (6, false),
        ] {
            assert_eq!(is_termination_event(event), expected, "event {event}");
        }
    }
}
