//! The running program's arguments, and a way to end it with a status:
//! the runtime half of `std/process`'s `args()` and `exit()`.
//!
//! **One process-wide list, set at most once.** `nova run` runs a program
//! inside `nova.exe`, whose own argv is nova's, so the driver calls
//! [`set_args`] with `[FILE, ARGS...]` before `main`
//! (`CompiledProgram::run_with_args`). If nothing set it, the first read takes
//! the OS's argv through `std::env::args_os()`. A built executable takes that
//! path: its generated C `main` drops `argc` and `argv`, but std's argv does
//! not depend on them. glibc passes them to std's `.init_array` function,
//! macOS answers `_NSGetArgc`/`_NSGetArgv`, and Windows parses the UTF-16
//! `GetCommandLineW`, which keeps non-ASCII arguments a C `main`'s ANSI
//! `argv` would garble. `nova test`'s binary takes it too, and gets no
//! arguments. `docs/adr/0023-program-arguments.md` records the decision.
//!
//! An argument that is not valid Unicode is converted with
//! `to_string_lossy`: U+FFFD stands in for its bad bytes, and nothing panics.
//!
//! `pub`, so `crates/nova-runtime/tests/process_args_*.rs` can drive it, and
//! so `nova-codegen-cranelift` can call [`set_args`].

use std::sync::OnceLock;

use crate::NovaStr;

/// The program's arguments, once set or read.
static ARGS: OnceLock<Vec<String>> = OnceLock::new();

/// Fixes the program's arguments to `args`. Returns `true` if this call set
/// them, and `false`, changing nothing, if they were already fixed by an
/// earlier call or an earlier read.
pub fn set_args(args: Vec<String>) -> bool {
    ARGS.set(args).is_ok()
}

/// The program's arguments: the list [`set_args`] stored, or else the OS's.
pub fn args() -> &'static [String] {
    ARGS.get_or_init(|| {
        std::env::args_os()
            .map(|a| a.to_string_lossy().into_owned())
            .collect()
    })
}

/// Argument `i` of `list`, or the empty string for an index out of range.
fn arg_at(list: &[String], i: i64) -> &str {
    usize::try_from(i)
        .ok()
        .and_then(|i| list.get(i))
        .map_or("", String::as_str)
}

/// `process_arg_count() -> Int`: how many arguments the program has,
/// counting the program itself at index 0.
#[no_mangle]
pub extern "C" fn nova_rt_process_arg_count() -> i64 {
    args().len() as i64
}

/// `process_arg(i: Int) -> String`: argument `i`, or the empty string for an
/// index out of range. `std/process`'s `args()` never asks for one.
#[no_mangle]
pub extern "C" fn nova_rt_process_arg(i: i64) -> *mut NovaStr {
    crate::gc_str(arg_at(args(), i))
}

/// `process_exit(code: Int)`: ends the process with `code`. `std::process::exit`
/// runs std's cleanup, which flushes stdout, and does not join running tasks.
/// The OS truncates `code`: Unix keeps its low 8 bits.
#[no_mangle]
pub extern "C" fn nova_rt_process_exit(code: i64) -> ! {
    std::process::exit(code as i32)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_index_in_range_is_that_argument() {
        let list = vec!["a".to_string(), "b c".to_string(), String::new()];
        assert_eq!(arg_at(&list, 0), "a");
        assert_eq!(arg_at(&list, 1), "b c");
        assert_eq!(arg_at(&list, 2), "");
    }

    #[test]
    fn an_index_out_of_range_is_the_empty_string() {
        let list = vec!["a".to_string()];
        assert_eq!(arg_at(&list, 1), "");
        assert_eq!(arg_at(&list, -1), "");
        assert_eq!(arg_at(&list, i64::MAX), "");
    }
}
