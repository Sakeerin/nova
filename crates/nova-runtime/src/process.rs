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
//!
//! # Other programs, and the environment
//!
//! The runtime half of `std/process`'s `run`, `status`, `env` and
//! `current_exe` (5.0 spec §5.7) is on `std/fs`'s wire contract
//! (`crate::fs`'s module doc): each fallible builtin returns a status code,
//! `0` or an `IoErrorKind`'s, and its payload waits to be taken.
//! [`nova_rt_process_run`] starts a program through `std::process::Command`,
//! which searches `PATH` for a bare name, `.exe` included on Windows, and
//! passes each argument as it is, with no shell; it blocks the thread until
//! the child ends. The child's exit status, and its output when captured,
//! wait in this thread's [`STATUS`], [`STDOUT`] and [`STDERR`] for the three
//! `nova_rt_process_take_*` builtins. They are Rust's own bytes, not GC
//! objects, so nothing is rooted while they wait: the `Bytes` is allocated
//! when it is taken. A failure goes through `crate::fs::fail`, which stashes
//! its message for `nova_rt_fs_last_error_message`, as `std/fs`'s does.
//! [`nova_rt_process_env`] and [`nova_rt_process_current_exe`] stash their
//! text for `nova_rt_fs_take_string`.
//!
//! A thread's cells, not a task's slots: `run` and `status` are plain `fn`s
//! whose wrappers take the payload straight after the run, with no `.await`
//! between, so no other task runs in between on this thread. Nothing here
//! panics, since a builtin can be called from an `async fn`'s poll: each
//! cell is a `Cell` that is only swapped.

use std::cell::Cell;
use std::sync::OnceLock;

use crate::fs::{fail, stash, Slot, OK};
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

thread_local! {
    /// The exit status of this thread's last run, until taken.
    static STATUS: Cell<i64> = const { Cell::new(0) };
    /// The standard output of this thread's last captured run, until taken.
    static STDOUT: Cell<Vec<u8>> = const { Cell::new(Vec::new()) };
    /// The standard error of this thread's last captured run, until taken.
    static STDERR: Cell<Vec<u8>> = const { Cell::new(Vec::new()) };
}

/// What a finished child's status is to Nova: its exit code, or, for a
/// death by signal on Unix, `128 + signal`, as shells report it (5.0 spec
/// §5.7, §12 item 22). Windows always has a code, read as Rust's `i32`.
/// `-1` stands for a status that is neither, which no system reports.
fn exit_code(status: std::process::ExitStatus) -> i64 {
    if let Some(code) = status.code() {
        return i64::from(code);
    }
    #[cfg(unix)]
    {
        use std::os::unix::process::ExitStatusExt;
        if let Some(signal) = status.signal() {
            return 128 + i64::from(signal);
        }
    }
    -1
}

/// Runs `program` with `args` and waits for it. With `capture`, its
/// standard input is empty and its standard output and error are read
/// whole, both at once, so a child that fills one pipe while the other is
/// read cannot stall; without, it inherits all three of this process's
/// streams, after this process's standard output is flushed, so what was
/// printed before comes first. Returns the exit status and the two outputs,
/// empty when not captured.
fn run_command(
    program: &str,
    args: &[&str],
    capture: bool,
) -> std::io::Result<(i64, Vec<u8>, Vec<u8>)> {
    use std::io::Write;
    use std::process::{Command, Stdio};
    let _ = std::io::stdout().flush();
    let mut command = Command::new(program);
    command.args(args);
    if capture {
        let out = command.stdin(Stdio::null()).output()?;
        Ok((exit_code(out.status), out.stdout, out.stderr))
    } else {
        let status = command.status()?;
        Ok((exit_code(status), Vec::new(), Vec::new()))
    }
}

/// The value of the environment variable `name`, or `None` when it is
/// unset or not UTF-8. A `name` that is empty or holds `=` or NUL is
/// `None` without asking the system, which may panic on one (5.0 spec
/// §5.7).
fn env_value(name: &str) -> Option<String> {
    if name.is_empty() || name.contains('=') || name.contains('\0') {
        return None;
    }
    std::env::var(name).ok()
}

/// `process_run(program: String, args: [String], capture: Bool) -> Int`:
/// runs `program`, as [`run_command`] does. Returns `OK`, the exit status
/// and the output then waiting for the `nova_rt_process_take_*` builtins,
/// or an `IoErrorKind`'s status code, through `crate::fs::fail`: a missing
/// program is `NOT_FOUND`, one that cannot be run `PERMISSION_DENIED`.
///
/// # Safety
/// `program` must point to a live `NovaStr`, and `args` to a live Nova
/// `[String]`: its length in word 0, a `NovaStr` pointer in each word after.
#[no_mangle]
pub unsafe extern "C" fn nova_rt_process_run(
    program: *const NovaStr,
    args: *const u8,
    capture: i8,
) -> i64 {
    let words = args as *const i64;
    // SAFETY: forwarding this function's own contract.
    let n = unsafe { *words }.max(0) as usize;
    let list: Vec<&str> = (0..n)
        // SAFETY: word `1 + i` is element `i` of the array, a `NovaStr`.
        .map(|i| unsafe { crate::as_str(*words.add(1 + i) as *const NovaStr) })
        .collect();
    // SAFETY: forwarding this function's own contract.
    let program = unsafe { crate::as_str(program) };
    match run_command(program, &list, capture != 0) {
        Ok((status, out, err)) => {
            STATUS.with(|c| c.set(status));
            STDOUT.with(|c| c.set(out));
            STDERR.with(|c| c.set(err));
            OK
        }
        Err(e) => fail(&e),
    }
}

/// `process_take_status() -> Int`: the last run's exit status.
#[no_mangle]
pub extern "C" fn nova_rt_process_take_status() -> i64 {
    STATUS.with(|c| c.replace(0))
}

/// `process_take_stdout() -> Bytes`: the last captured run's standard
/// output, which is then empty.
#[no_mangle]
pub extern "C" fn nova_rt_process_take_stdout() -> *mut NovaStr {
    crate::bytes::gc_bytes(&STDOUT.with(Cell::take))
}

/// `process_take_stderr() -> Bytes`: the last captured run's standard
/// error, which is then empty.
#[no_mangle]
pub extern "C" fn nova_rt_process_take_stderr() -> *mut NovaStr {
    crate::bytes::gc_bytes(&STDERR.with(Cell::take))
}

/// `process_env(name: String) -> Bool`: whether `name` is set to UTF-8
/// text, as [`env_value`] says; its value then waits for
/// `nova_rt_fs_take_string`.
///
/// # Safety
/// `name` must point to a live `NovaStr`.
#[no_mangle]
pub unsafe extern "C" fn nova_rt_process_env(name: *const NovaStr) -> i8 {
    // SAFETY: forwarding this function's own contract.
    match env_value(unsafe { crate::as_str(name) }) {
        Some(value) => {
            stash(Slot::Buffer, crate::gc_str(&value));
            1
        }
        None => 0,
    }
}

/// `process_current_exe() -> Int`: `OK`, with the running executable's
/// path waiting for `nova_rt_fs_take_string`, or an `IoErrorKind`'s status
/// code. Under `nova run` the executable is `nova` itself. A path that is
/// not valid Unicode is `INVALID_DATA`, as a file that is not UTF-8 is to
/// `read_to_string`: a path with its bad bytes replaced would name another
/// file.
#[no_mangle]
pub extern "C" fn nova_rt_process_current_exe() -> i64 {
    match std::env::current_exe() {
        Ok(path) => match path.to_str() {
            Some(text) => {
                stash(Slot::Buffer, crate::gc_str(text));
                OK
            }
            None => fail(&std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "the executable's path is not valid Unicode",
            )),
        },
        Err(e) => fail(&e),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A program found by its bare name on `PATH`, `.exe` included on
    /// Windows, runs with its arguments as they are, and its status, and on
    /// Unix both outputs, come back (5.0 spec §5.7). On Windows `cmd`'s own
    /// reading of its command line is not Rust's, so only the status is
    /// checked there; `tests/runtime/process_run.nova` checks the output
    /// of a child that reads its arguments as Rust does, on all three
    /// systems.
    #[test]
    fn a_program_on_path_runs_and_its_output_is_captured() {
        #[cfg(unix)]
        {
            let script = "printf out; printf err >&2; exit 7";
            let (status, out, err) = run_command("sh", &["-c", script], true).expect("it runs");
            assert_eq!(status, 7);
            assert_eq!(out, b"out");
            assert_eq!(err, b"err");
        }
        #[cfg(windows)]
        {
            let (status, _, _) = run_command("cmd", &["/C", "exit 7"], true).expect("it runs");
            assert_eq!(status, 7);
        }
    }

    /// More output than a pipe holds, on both streams, does not stall the
    /// run: both are read at once.
    #[cfg(unix)]
    #[test]
    fn output_larger_than_a_pipe_is_read_whole() {
        let script = "i=0; while [ $i -lt 20000 ]; do echo 0123456789; echo 0123456789 >&2; \
                      i=$((i+1)); done";
        let (status, out, err) = run_command("sh", &["-c", script], true).expect("it runs");
        assert_eq!(status, 0);
        assert_eq!(out.len(), 220_000);
        assert_eq!(err.len(), 220_000);
    }

    /// A death by signal is `128 + signal`, as shells report it (5.0 spec
    /// §12 item 22): `kill -9` gives 137.
    #[cfg(unix)]
    #[test]
    fn a_death_by_signal_is_128_plus_the_signal() {
        let (status, _, _) = run_command("sh", &["-c", "kill -9 $$"], true).expect("it runs");
        assert_eq!(status, 137);
    }

    /// A missing program is `NotFound`, the kind `fail` maps to
    /// `NOT_FOUND`.
    #[test]
    fn a_missing_program_is_not_found() {
        let e = run_command("nova-no-such-program-5-0c", &[], true).expect_err("no program");
        assert_eq!(e.kind(), std::io::ErrorKind::NotFound);
        let e = run_command("nova-no-such-program-5-0c", &[], false).expect_err("no program");
        assert_eq!(e.kind(), std::io::ErrorKind::NotFound);
    }

    /// A file that is not executable cannot be run: `PermissionDenied`.
    #[cfg(unix)]
    #[test]
    fn a_file_that_cannot_be_run_is_permission_denied() {
        let dir = std::env::temp_dir().join("nova-runtime-process-denied");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("a directory");
        let file = dir.join("not-executable");
        std::fs::write(&file, "echo hi\n").expect("a file");
        let e = run_command(&file.to_string_lossy(), &[], true).expect_err("not runnable");
        assert_eq!(e.kind(), std::io::ErrorKind::PermissionDenied);
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// An empty name, or one with `=` or NUL, is `None` without asking the
    /// system; an unset name is `None`; a set one is its value (5.0 spec
    /// §5.7). `PATH` is set on every system the tests run on.
    #[test]
    fn env_refuses_names_the_system_cannot_hold() {
        for name in ["", "A=B", "=", "A\0B", "\0"] {
            assert_eq!(env_value(name), None, "{name:?}");
        }
        assert_eq!(env_value("NOVA_RUNTIME_SURELY_UNSET_5_0C"), None);
        assert!(env_value("PATH").is_some());
    }

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
