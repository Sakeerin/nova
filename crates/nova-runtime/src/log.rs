//! The logger's configuration, and nothing else.
//!
//! Nova has no mutable global state — top-level bindings are `const` — so
//! the logger's level and destination live here, in the shape `file.rs`'s
//! open-file table and `task.rs`'s `CURRENT` already establish.
//!
//! **Thread-local, not global**, because the executor is single-threaded
//! and every other piece of runtime state here already is. If Nova grows
//! real threads, a per-thread logger configuration is the wrong answer and
//! changing it is ADR-worthy — recorded now so that it is a decision then
//! rather than a discovery.

use std::cell::Cell;

#[derive(Clone, Copy)]
struct Config {
    level: i64,
    to_stderr: bool,
}

/// `None` means "never configured", which the getters resolve to the
/// default. That resolution is the entire auto-initialize rule: a program
/// that never calls `Log::init()` still logs, because the first *read*
/// installs the default.
const DEFAULT: Config = Config {
    level: 2, // Info
    to_stderr: true,
};

thread_local! {
    static CONFIG: Cell<Option<Config>> = const { Cell::new(None) };
}

fn get() -> Config {
    CONFIG.with(|c| match c.get() {
        Some(cfg) => cfg,
        None => {
            c.set(Some(DEFAULT));
            DEFAULT
        }
    })
}

/// The threshold, as `LogLevel::to_int` numbers it.
#[no_mangle]
pub extern "C-unwind" fn nova_rt_log_config_level() -> i64 {
    get().level
}

/// `1` for stderr, `0` for stdout. An `i64` rather than a Rust `bool`
/// because every other intrinsic in this crate crosses the boundary as one.
#[no_mangle]
pub extern "C-unwind" fn nova_rt_log_config_to_stderr() -> i64 {
    if get().to_stderr {
        1
    } else {
        0
    }
}

/// Install a configuration, overwriting any previous one.
///
/// Two separate getters above rather than one packed integer
/// (`level * 2 + to_stderr`), deliberately: packing would save one builtin
/// and reintroduce an `i64` whose *meaning* the compiler cannot check,
/// which this project has already had to rename its way out of twice.
#[no_mangle]
pub extern "C-unwind" fn nova_rt_log_set_config(level: i64, to_stderr: i64) {
    CONFIG.with(|c| {
        c.set(Some(Config {
            level,
            to_stderr: to_stderr != 0,
        }))
    });
}

/// Clear the cell so a test can observe the unset behaviour. `#[cfg(test)]`
/// only — nothing in a running program un-configures a logger.
#[cfg(test)]
fn reset_for_test() {
    CONFIG.with(|c| c.set(None));
}

#[cfg(test)]
mod tests {
    use super::*;

    /// An unset cell resolves to the spec's default rather than to zero:
    /// level `Info` (2), output stderr. This *is* the auto-initialize rule
    /// — there is no separate init path, the getter's `None` arm is it.
    #[test]
    fn an_unset_config_reads_as_the_default() {
        reset_for_test();
        assert_eq!(nova_rt_log_config_level(), 2);
        assert_eq!(nova_rt_log_config_to_stderr(), 1);
    }

    #[test]
    fn set_then_get_round_trips() {
        reset_for_test();
        nova_rt_log_set_config(4, 0);
        assert_eq!(nova_rt_log_config_level(), 4);
        assert_eq!(nova_rt_log_config_to_stderr(), 0);
    }

    /// Last writer wins, which is what makes `init_with` after a log call
    /// reconfigure rather than being ignored.
    #[test]
    fn the_second_set_wins() {
        reset_for_test();
        nova_rt_log_set_config(0, 1);
        nova_rt_log_set_config(3, 0);
        assert_eq!(nova_rt_log_config_level(), 3);
        assert_eq!(nova_rt_log_config_to_stderr(), 0);
    }
}
