# `examples/04-todo-cli` and 02-fibonacci's Argument Gate Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Give Nova programs their command-line arguments (`std/process`'s `args()` and `exit()`, and `nova run [FILE] -- ARGS`), then ship `examples/04-todo-cli` (the last Phase 2 gate example, `nova-spec/60-EXAMPLES.md` §4) and meet `02-fibonacci`'s never-met gate (`nova run -- 20` prints `fib(20) = 6765`).

**Architecture:**
- **Runtime.** `crates/nova-runtime/src/process.rs` holds one process-wide argument list, set at most once. `nova run` sets it before `main`. If nothing set it, the first read takes the OS's argv through `std::env::args_os()`, which is how a built executable gets its arguments. Neither code generator changes.
- **`std/process`.** A new std module, written in Nova over three `STD_ONLY` builtins: `args() -> Vec<String>` and `exit(code: Int)`.
- **The examples.** Both are written in today's Nova. The spec listings stay as aspiration.

**Tech Stack:** Rust (`nova-runtime`, `nova-resolver`, `nova-typeck`, `nova-mir`, `nova-codegen-cranelift`, `nova-driver`, `nova-cli` with clap 4), Nova (`std/process/lib.nova`, the two examples), libtest end-to-end tests in `crates/nova-cli/tests/run_tests.rs`.

**Spec:** `docs/superpowers/specs/2026-10-05-examples-04-todo-cli-design.md`, approved by the user on 2026-10-05. Read it before Task 1. It is the authority this plan argues from.

## Global Constraints

- **Approach A** (spec §4): the runtime holds the list; `nova run` sets it; a built executable's runtime reads the OS's argv through `std::env::args_os()`. The generated C `main` does not change, in either backend.
- **`args()[0]` is the program:** FILE as written under `nova run` (`src/main.nova` when defaulted), whatever the OS reports in a built executable. User arguments start at index 1 (spec §4.4).
- **`nova run [FILE] [-- ARGS...]`.** Without `--`, `args()` is exactly `[FILE]` (spec §4.4).
- **Non-UTF-8 arguments** are converted with `to_string_lossy`, never panicked on (spec §4.2).
- **`exit(code)`** calls `std::process::exit`: it flushes stdout, does not join running tasks, and is typed `(Int) -> ()` (spec §4.3).
- **Counts:** three new `STD_ONLY` builtins, 78 → 81; `STD_MODULES` 15 → 16; `lib.nova` files 16 → 17 with `STD_TEST_MODULE` (spec §4.3).
- **The examples** are written in the Nova that exists. Both spec listings stay unchanged (spec §1-3).
- **04's behaviour** (spec §5.2, §6): `todos.json` in the working directory, compact JSON. A file that exists but cannot be read, parsed or decoded is refused: an error on stderr, exit 1, the file untouched. `add`'s title is every argument after `add`, joined with single spaces; `untitled` if none. Anything other than `add`, `list` and `done` prints the usage line on stderr and exits 1.
- **02's behaviour** (spec §5.1): the first argument through `parse` and `Int::from_json`, default 10; prints `fib(n) = ...`.
- **04 is tested both under `nova run` and as a built executable** (spec §7.4).
- **Records state facts only.** No record declares Phase 2 complete (spec §8).
- **Dependencies.** None added. `Cargo.lock` must not change: check it with `git diff --exit-code Cargo.lock`.
- **Toolchain and CI.** Edition 2021, MSRV 1.78. CI runs `cargo test --locked --workspace --all-features --no-fail-fast` and `cargo clippy --locked --all-targets --all-features -- -D warnings` on ubuntu, windows and macOS. Both must pass.
- **Dated notes** carry the date they are written. Every note below says `2026-10-05`. If you execute on a later day, change that date in every note you write, and leave the dates that refer to past decisions as they are.

## Review Focus

The five failure modes most likely to bite a user, each with the test that pins it:

1. **Non-ASCII arguments.** A Thai title must survive the arguments, `todos.json` and `list`, both under `nova run` and in a built executable, whose argv on Windows would be garbled by a C `main` → `todo_cli_keeps_a_thai_title_under_nova_run`, `todo_cli_keeps_a_thai_title_as_a_built_executable` (Task 5).
2. **An argument with spaces, and an empty argument.** Each arrives as one argument, unchanged → `process_args_run` (Task 3); `add buy milk` against `add "walk dog"` in `todo_cycle` (Task 5).
3. **A corrupt `todos.json` is never overwritten.** Every command refuses it, `list` included, and the bytes on disk are unchanged → `todo_cli_refuses_a_corrupt_todos_json_and_leaves_it_alone` (Task 5).
4. **`exit()` flushes a partial line and returns the exact code,** under `nova run` and in a built executable → `process_exit_run`, `process_exit_build_standalone` (Task 3).
5. **`nova run` without `--` behaves exactly as before:** `args()` is `[FILE]` → `process_args_without_a_separator_is_just_the_file` (Task 3); every existing `*_run` test.

## File Structure

| File | Task | Responsibility |
|---|---|---|
| `crates/nova-runtime/src/process.rs` (new) | 1 | The list, `set_args`, `args`, the OS fallback, the three builtins, and unit tests |
| `crates/nova-runtime/src/lib.rs` | 1 | `pub mod process;` and three `symbols()` entries |
| `crates/nova-runtime/tests/process_args_set.rs`, `process_args_os.rs` (new) | 1 | One process-global scenario per test binary |
| `docs/adr/0023-program-arguments.md` (new) | 1 | The decision record |
| `crates/nova-resolver/src/lib.rs`, `crates/nova-typeck/src/check.rs`, `crates/nova-mir/src/lib.rs`, `crates/nova-mir/src/lower.rs` | 2 | The three builtins' registration |
| `std/process/lib.nova` (new); `crates/nova-resolver/src/lib.rs` | 3 | `args()` and `exit()`; the `STD_MODULES` entry |
| `crates/nova-codegen-cranelift/src/lib.rs`, `crates/nova-driver/src/lib.rs`, `crates/nova-cli/src/cmd/run.rs` | 3 | `CompiledProgram::run_with_args`; `run_file`'s `args`; `RunCmd`'s `-- ARGS` |
| `tests/runtime/process_args.nova` and `.stdout`, `tests/runtime/process_exit.nova` (new) | 3 | `std/process` fixtures |
| `examples/02-fibonacci/src/main.nova` | 4 | Reads its argument |
| `examples/04-todo-cli/src/main.nova`, `README.md` (new); `.gitignore` | 5 | The example, its §9 README, and its ignored data file |
| `crates/nova-cli/tests/run_tests.rs` | 3, 4, 5 | The fixtures' tests, 02's tests, 04's end-to-end tests |
| Records (Task 6 lists them) | 6 | Dated notes, the new `std/process` section, the CHANGELOG, the sweep |

## Conventions for every task

- **Working directory.** `D:\Projects\nona\nova`, which is Git Bash `/d/Projects/nona/nova`. The Bash tool resets its directory after each call, so write `cd /d/Projects/nona/nova && …` in one command.
- **Line endings.** The working tree is CRLF (`core.autocrlf=true`). Any edit tool is fine. A script that matches multi-line text must convert its anchors to the file's own newline. The script in Task 6 does that, and aborts before writing anything if an anchor does not match exactly once.
- **Write scripts with the Write tool, never a Bash heredoc.** The Bash tool here turns `\\` into `\`.
  - Put them in `C:/Users/SAKEER~1/AppData/Local/Temp/gcm/_once/`.
  - Run them with `python -X utf8 <path>`.
  - Rename each to `<name>.applied` once it has run.
- **Stale runtime library.** `nova build` in tests links `target/debug/nova_runtime.lib`. Run `cargo build -p nova-runtime` before any `nova-cli` test run that follows a runtime change.
- **Chain a commit and what follows it with `&&`, never `;`.** A failed format check must stop the commit and every step after it.
- **Long output** goes to a file in the scratchpad. Read its tail.

---

### Task 1: The runtime's argument list (`process.rs`) and ADR 0023

**Files:**
- Create: `crates/nova-runtime/src/process.rs`
- Modify: `crates/nova-runtime/src/lib.rs` (after `pub mod signal;`, and the `symbols()` entry after `nova_rt_shutdown_requested`)
- Create: `crates/nova-runtime/tests/process_args_set.rs`, `crates/nova-runtime/tests/process_args_os.rs`
- Create: `docs/adr/0023-program-arguments.md`

**Interfaces:**
- Consumes: `crate::gc_str(s: &str) -> *mut NovaStr` (`crates/nova-runtime/src/lib.rs`), `crate::NovaStr`.
- Produces:
  - `nova_runtime::process::set_args(args: Vec<String>) -> bool`;
  - `nova_runtime::process::args() -> &'static [String]`;
  - `#[no_mangle] extern "C"`: `nova_rt_process_arg_count() -> i64`, `nova_rt_process_arg(i: i64) -> *mut NovaStr`, `nova_rt_process_exit(code: i64) -> !`;
  - their three `symbols()` entries.

- [ ] **Step 1: Write the failing integration tests**

Create `crates/nova-runtime/tests/process_args_set.rs`:

```rust
//! One scenario, one process: the list `set_args` stores is the list the
//! runtime reports, and a second `set_args` changes nothing. Its own test
//! binary because the list is set at most once per process.

use nova_runtime::process::{args, nova_rt_process_arg_count, set_args};

#[test]
fn set_args_fixes_the_list_once() {
    let first = vec![
        "prog.nova".to_string(),
        "one".to_string(),
        "two words".to_string(),
        String::new(),
    ];
    assert!(set_args(first.clone()), "the first set_args takes");
    assert_eq!(args(), first.as_slice());
    assert_eq!(nova_rt_process_arg_count(), 4);
    assert!(
        !set_args(vec!["other".to_string()]),
        "a second set_args is refused"
    );
    assert_eq!(args(), first.as_slice(), "and changes nothing");
}
```

Create `crates/nova-runtime/tests/process_args_os.rs`:

```rust
//! One scenario, one process: with nothing set, the runtime reports the OS's
//! own argv, the route a built executable takes. Its own test binary because
//! the first read fixes the list for the rest of the process.

use nova_runtime::process::{args, nova_rt_process_arg_count};

#[test]
fn with_nothing_set_the_list_is_the_os_argv() {
    let list = args();
    let os: Vec<String> = std::env::args_os()
        .map(|a| a.to_string_lossy().into_owned())
        .collect();
    assert_eq!(list, os.as_slice());
    assert_eq!(nova_rt_process_arg_count(), list.len() as i64);
    let exe = std::env::current_exe().expect("current_exe");
    let stem = exe
        .file_stem()
        .expect("an executable has a file name")
        .to_string_lossy()
        .into_owned();
    assert!(
        list[0].contains(&stem),
        "argv[0] {:?} should name this test binary, {stem:?}",
        list[0]
    );
}
```

These read the list through the Rust `args()`, not through `nova_rt_process_arg`. That builtin allocates a GC string, which needs no Nova program here but is better exercised by one: Task 3's `process_args_run` drives it end to end, and Step 3's unit tests pin its index logic.

- [ ] **Step 2: Run them to see them fail**

Run: `cd /d/Projects/nona/nova && cargo test -p nova-runtime --test process_args_set --test process_args_os 2>&1 | grep -E "^error|unresolved" | head -5`
Expected: FAIL to compile, `error[E0432]: unresolved import` naming `nova_runtime::process`.

- [ ] **Step 3: Write `process.rs` and wire it in**

Create `crates/nova-runtime/src/process.rs`:

```rust
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
```

In `crates/nova-runtime/src/lib.rs`, after the three-line doc comment and `pub mod signal;`, add:

```rust
/// The program's arguments and `exit`, behind `std/process` (see its module
/// doc and `docs/adr/0023-program-arguments.md`). `pub` so the driver's JIT
/// path can set the list, and so `tests/process_args_*.rs` can drive it.
pub mod process;
```

In `symbols()`, after the `nova_rt_shutdown_requested` entry, add:

```rust
        ("nova_rt_process_arg_count", process::nova_rt_process_arg_count as *const u8),
        ("nova_rt_process_arg", process::nova_rt_process_arg as *const u8),
        ("nova_rt_process_exit", process::nova_rt_process_exit as *const u8),
```

- [ ] **Step 4: Run every new test**

Run: `cd /d/Projects/nona/nova && cargo test -p nova-runtime --lib process:: 2>&1 | grep -E "^test |test result" && cargo test -p nova-runtime --test process_args_set --test process_args_os 2>&1 | grep -E "^test |test result"`
Expected: the two unit tests and both integration tests pass.

- [ ] **Step 5: Write ADR 0023**

Create `docs/adr/0023-program-arguments.md`:

```markdown
# ADR 0023 — Program arguments: the runtime holds the list; `nova run` sets it, a built executable reads the OS

## Status

Accepted (2026-10-05). Branch `examples-04-todo-cli`
(`docs/superpowers/specs/2026-10-05-examples-04-todo-cli-design.md`).

## Context

`nova-spec/60-EXAMPLES.md` §4's `04-todo-cli` and §2's `02-fibonacci` both
read their arguments with `args()`. Until now a Nova program could not see its
arguments (`docs/superpowers/specs/2026-10-04-examples-03-04-inventory.md`
§4.3). Four facts constrain any route:

- **`nova run` runs the program inside `nova.exe`,** through
  `CompiledProgram::run`, so the process's argv is nova's own.
- **A built executable's entry is a generated C `main` that drops `argc` and
  `argv`:** Cranelift's `emit_c_main`, and the LLVM backend's. It calls
  `nova_main()` and returns 0.
- **On Windows, a C `main`'s `argv` is in the ANSI code page** (CP874 on the
  development host), so non-ASCII arguments arrive garbled. The UTF-16 command
  line is lossless.
- **Rust's `std::env::args_os()` does not need Rust's `main`.** glibc passes
  `argc` and `argv` to std's `.init_array` function, macOS answers
  `_NSGetArgc` and `_NSGetArgv`, and Windows parses `GetCommandLineW`.

## Decision

1. **The runtime holds one process-wide list,** in
   `crates/nova-runtime/src/process.rs`, set at most once.
2. **`nova run [FILE] -- ARGS` sets it** to `[FILE, ARGS...]` through
   `CompiledProgram::run_with_args`, before `main`. FILE is as written, or
   `src/main.nova` when defaulted.
3. **Otherwise the first read takes the OS's argv** through
   `std::env::args_os()`, converted with `to_string_lossy`. A built
   executable takes this path, and so does `nova test`'s binary, which gets no
   arguments.
4. **Three `STD_ONLY` builtins expose it:** `process_arg_count`, `process_arg`
   and `process_exit`. `std/process` builds `args() -> Vec<String>` from the
   first two. `exit(code)` calls `std::process::exit`, which flushes stdout
   and does not join running tasks.
5. **The generated C `main` does not change,** in either backend.

## Alternatives considered

- **Forward `argc` and `argv` from the generated C `main`** to a runtime init
  call. Rejected: both code generators change, the LLVM one cannot be built on
  the development host, and Windows' `argv` is in the ANSI code page unless
  the shim moves to `wmain`.
- **An environment variable** set by `nova run`. Rejected: quoting, and the
  variable leaks into every child process.
- **`fn main(args: Vec<String>)`.** Rejected: it changes the language's `main`
  contract, and the spec listings call `args()`.

## Consequences

- **`std/process` exists,** with `args` and `exit` (`nova-spec/20-STDLIB.md`
  §17). `STD_ONLY` grows from 78 to 81, and `STD_MODULES` from 15 to 16.
- **Decision 3 is reasoned from std's implementation for Linux and macOS, not
  measured on the development host.** Its measurement is the built-executable
  end-to-end tests in `crates/nova-cli/tests/run_tests.rs`
  (`todo_cli_cycle_as_a_built_executable` and its Thai-title sibling), which
  run on all three CI operating systems. A libc that does not pass `argc` and
  `argv` to `.init_array` functions, musl for one, would give an empty list.
- **`exit` under `nova run` ends `nova.exe` itself,** so the shell sees the
  program's code.
- **Not covered:**
  - `std/process`'s `spawn` and `env`;
  - E-11: a built executable drops a trailing partial line when `main`
    returns, because the C `main` returns without std's flush. `exit` flushes;
  - the LLVM backend, which this decision does not touch.

## References

- Design: `docs/superpowers/specs/2026-10-05-examples-04-todo-cli-design.md` §4
- Inventory: `docs/superpowers/specs/2026-10-04-examples-03-04-inventory.md`
  §4.3
- Code: `crates/nova-runtime/src/process.rs`, `std/process/lib.nova`,
  `crates/nova-cli/src/cmd/run.rs`
- Tests: `crates/nova-runtime/tests/process_args_set.rs` and
  `process_args_os.rs`; the `process_*` and `todo_cli_*` tests in
  `crates/nova-cli/tests/run_tests.rs`
```

- [ ] **Step 6: Refresh the runtime library, then commit**

```bash
cd /d/Projects/nona/nova && cargo fmt -p nova-runtime -- --check && cargo clippy -p nova-runtime --all-targets -- -D warnings 2>&1 | tail -1 && cargo build -p nova-runtime 2>&1 | tail -1 && git add crates/nova-runtime/src/process.rs crates/nova-runtime/src/lib.rs crates/nova-runtime/tests/process_args_set.rs crates/nova-runtime/tests/process_args_os.rs docs/adr/0023-program-arguments.md && git commit -m "feat(runtime): the program's argument list and exit (ADR 0023)

process.rs holds one process-wide list, set at most once: nova run sets
it before main, and otherwise the first read takes the OS argv through
std::env::args_os, the route a built executable takes. Three builtins
expose the count, one argument, and std::process::exit. One integration
test binary per process-global scenario.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

If `cargo fmt` reports a diff, run `cargo fmt -p nova-runtime`, re-run Step 4, then commit.

---

### Task 2: Register the three `std/process` builtins

**Files:**
- Modify: `crates/nova-resolver/src/lib.rs`: the `builtins!` list (after `ShutdownRequested,`), `Builtin::name` (after `Builtin::ShutdownRequested => "shutdown_requested",`), and `STD_ONLY` (`[Builtin; 78]` becomes `81`; append after `Builtin::ShutdownRequested,`)
- Modify: `crates/nova-typeck/src/check.rs`: the hint arm (`| Builtin::ShutdownRequested => "",`), `builtin_signature` (after `Builtin::ShutdownRequested => (vec![], Ty::Bool),`), the test table `expected()` (after its `Builtin::ShutdownRequested` entry), and a new test
- Modify: `crates/nova-mir/src/lib.rs`: the `rt_funcs!` list (after `ShutdownRequested,`), `symbol()` and `signature()`
- Modify: `crates/nova-mir/src/lower.rs`: the lowering table (after `Builtin::ShutdownRequested => Lowering::Runtime(RtFunc::ShutdownRequested),`)

**Interfaces:**
- Consumes: Task 1's three runtime symbols and their `symbols()` entries.
- Produces: a std module may call `process_arg_count() -> Int`, `process_arg(i: Int) -> String` and `process_exit(code: Int) -> ()`, lowered to `RtFunc::ProcessArgCount` (`() -> i64`), `RtFunc::ProcessArg` (`(i64) -> ptr`) and `RtFunc::ProcessExit` (`(i64) -> unit`).

- [ ] **Step 1: Write the failing test**

In `crates/nova-typeck/src/check.rs`'s test module, after `shutdown_requested_is_std_only_and_returns_bool`, add:

```rust
    /// The three `std/process` builtins are `STD_ONLY` and typed as
    /// `std/process/lib.nova` calls them (docs/adr/0023).
    #[test]
    fn the_process_builtins_are_std_only_with_their_signatures() {
        for (name, sig) in [
            ("process_arg_count", (vec![], Ty::Int)),
            ("process_arg", (vec![Ty::Int], Ty::String)),
            ("process_exit", (vec![Ty::Int], Ty::Unit)),
        ] {
            let b = nova_resolver::Builtin::STD_ONLY
                .iter()
                .copied()
                .find(|b| b.name() == name)
                .unwrap_or_else(|| panic!("{name} must be an STD_ONLY builtin"));
            assert_eq!(builtin_signature(b), sig, "{name}");
        }
    }
```

- [ ] **Step 2: Run it to see it fail**

Run: `cd /d/Projects/nona/nova && cargo test -p nova-typeck the_process_builtins_are_std_only 2>&1 | grep -E "panicked|must be|test result" | head -4`
Expected: FAIL with `process_arg_count must be an STD_ONLY builtin`.

- [ ] **Step 3: Add the builtins at every site**

`crates/nova-resolver/src/lib.rs`, in the `builtins!` list, after `ShutdownRequested,`:

```rust
    /// `process_arg_count() -> Int` — how many arguments the program has,
    /// counting the program itself at index 0. Runtime symbol
    /// `nova_rt_process_arg_count` (`crates/nova-runtime/src/process.rs`,
    /// docs/adr/0023). Std-only.
    ProcessArgCount,
    /// `process_arg(i: Int) -> String` — argument `i`, or the empty string
    /// for an index out of range. Runtime symbol `nova_rt_process_arg`.
    /// Std-only.
    ProcessArg,
    /// `process_exit(code: Int) -> unit` — ends the process with `code`
    /// after flushing stdout, and never returns. Runtime symbol
    /// `nova_rt_process_exit`. Std-only.
    ProcessExit,
```

In `Builtin::name`, after `Builtin::ShutdownRequested => "shutdown_requested",`:

```rust
            Builtin::ProcessArgCount => "process_arg_count",
            Builtin::ProcessArg => "process_arg",
            Builtin::ProcessExit => "process_exit",
```

In `STD_ONLY`, change `pub const STD_ONLY: [Builtin; 78] = [` to `pub const STD_ONLY: [Builtin; 81] = [`, and after `Builtin::ShutdownRequested,` add:

```rust
        Builtin::ProcessArgCount,
        Builtin::ProcessArg,
        Builtin::ProcessExit,
```

`crates/nova-typeck/src/check.rs`, in the hint arm, change:

```rust
            | Builtin::ShutdownRequested => "",
```

to:

```rust
            | Builtin::ShutdownRequested
            | Builtin::ProcessArgCount
            | Builtin::ProcessArg
            | Builtin::ProcessExit => "",
```

In `builtin_signature`, after `Builtin::ShutdownRequested => (vec![], Ty::Bool),`:

```rust
        Builtin::ProcessArgCount => (vec![], Ty::Int),
        Builtin::ProcessArg => (vec![Ty::Int], Ty::String),
        Builtin::ProcessExit => (vec![Ty::Int], Ty::Unit),
```

In the test table `expected()`, after its `Builtin::ShutdownRequested => ( … ),` entry:

```rust
                Builtin::ProcessArgCount => (
                    (vec![], Ty::Int),
                    "`process_arg_count()` in `std/process`'s `args`",
                ),
                Builtin::ProcessArg => (
                    (vec![Ty::Int], Ty::String),
                    "`process_arg(i)` in `std/process`'s `args`",
                ),
                Builtin::ProcessExit => (
                    (vec![Ty::Int], Ty::Unit),
                    "`process_exit(code)` in `std/process`'s `exit`",
                ),
```

`crates/nova-mir/src/lib.rs`, in the `rt_funcs!` list, after `ShutdownRequested,`:

```rust
    /// `() -> i64` — the program's argument count
    /// (`crates/nova-runtime/src/process.rs`).
    ProcessArgCount,
    /// `(i64 i) -> ptr` — argument `i` as a `String`, or the empty string.
    ProcessArg,
    /// `(i64 code) -> unit` — `std::process::exit`; never returns.
    ProcessExit,
```

In `symbol()`, after `RtFunc::ShutdownRequested => "nova_rt_shutdown_requested",`:

```rust
            RtFunc::ProcessArgCount => "nova_rt_process_arg_count",
            RtFunc::ProcessArg => "nova_rt_process_arg",
            RtFunc::ProcessExit => "nova_rt_process_exit",
```

In `signature()`, after `RtFunc::ShutdownRequested => (vec![], MirTy::I8),`:

```rust
            RtFunc::ProcessArgCount => (vec![], MirTy::I64),
            RtFunc::ProcessArg => (vec![MirTy::I64], MirTy::Ptr),
            RtFunc::ProcessExit => (vec![MirTy::I64], MirTy::Unit),
```

`crates/nova-mir/src/lower.rs`, after `Builtin::ShutdownRequested => Lowering::Runtime(RtFunc::ShutdownRequested),`:

```rust
                    Builtin::ProcessArgCount => Lowering::Runtime(RtFunc::ProcessArgCount),
                    Builtin::ProcessArg => Lowering::Runtime(RtFunc::ProcessArg),
                    Builtin::ProcessExit => Lowering::Runtime(RtFunc::ProcessExit),
```

- [ ] **Step 4: Run the new test and the guards that cover every builtin**

Run: `cd /d/Projects/nona/nova && cargo check --workspace --all-targets 2>&1 | tail -1 && cargo test -p nova-typeck -- the_process_builtins_are_std_only builtin_signatures_are_what_the_std_call_sites_use no_std_only_builtin_is_callable_from_user_code 2>&1 | grep -E "^test |test result" && cargo test -p nova-resolver no_std_only_builtin_is_a_reserved_word 2>&1 | grep -E "^test |test result" && cargo test -p nova-codegen-cranelift every_rt_func_symbol_is_registered_with_the_jit 2>&1 | grep -E "^test |test result"`
Expected:
- `cargo check` finishes clean. `--all-targets` is what reaches the typechecker's test-table site.
- Every named test passes: the new one, the signature table, the two `STD_ONLY` loops, and the JIT symbol check, which finds Task 1's `symbols()` entries.

- [ ] **Step 5: Commit**

```bash
cd /d/Projects/nona/nova && cargo fmt --all -- --check && git add crates/nova-resolver/src/lib.rs crates/nova-typeck/src/check.rs crates/nova-mir/src/lib.rs crates/nova-mir/src/lower.rs && git commit -m "feat(builtins): process_arg_count, process_arg, process_exit; STD_ONLY 78 -> 81

Typed () -> Int, (Int) -> String and (Int) -> (), lowered to the runtime's
nova_rt_process_* symbols. Std-only: std/process's args() and exit() are
the public surface.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

If `cargo fmt --all -- --check` reports a diff, run `cargo fmt --all`, re-run Step 4's `cargo check`, then commit.

---

### Task 3: `std/process`, and `nova run [FILE] -- ARGS`

**Files:**
- Create: `std/process/lib.nova`
- Modify: `crates/nova-resolver/src/lib.rs`: `STD_MODULES` (`[(&str, &str); 15]` becomes `16`; append after the `$std.crypto` entry)
- Modify: `crates/nova-codegen-cranelift/src/lib.rs`: `impl CompiledProgram` (after `run`)
- Modify: `crates/nova-driver/src/lib.rs`: `run_file`
- Modify: `crates/nova-cli/src/cmd/run.rs`: `RunCmd` and `run`
- Create: `tests/runtime/process_args.nova`, `tests/runtime/process_args.stdout`, `tests/runtime/process_exit.nova`
- Modify: `crates/nova-cli/tests/run_tests.rs`: four tests, after `http_server_dispatch_run`

**Interfaces:**
- Consumes: Task 1's `nova_runtime::process::set_args`; Task 2's three builtins.
- Produces:
  - `pub fn args() -> Vec<String>` and `pub fn exit(code: Int)`, glob-imported into every user module;
  - `CompiledProgram::run_with_args(&self, args: Vec<String>)`;
  - `nova_driver::run_file(path: &Path, args: Vec<String>) -> Result<Outcome<()>>`;
  - `nova run [FILE] [-- ARGS...]`.

- [ ] **Step 1: Write the failing fixtures and tests**

Create `tests/runtime/process_args.nova`:

```nova
// `std/process`'s `args()` under `nova run`: index 0 is FILE as written on
// the command line, and every argument after `--` arrives as given, one with
// spaces and an empty one included. Run from the repository root by
// `process_args_run` in `crates/nova-cli/tests/run_tests.rs`, as
// `nova run tests/runtime/process_args.nova -- one "two words" ""`.

fn main() {
    let argv = args()
    println("count: ${argv.len()}")
    let mut i = 0
    while i < argv.len() {
        match argv.get(i) {
            Some(a) => println("[${a}]")
            None => {}
        }
        i = i + 1
    }
}
```

Create `tests/runtime/process_args.stdout`. Every line ends with a newline, the last included:

```
count: 4
[tests/runtime/process_args.nova]
[one]
[two words]
[]
```

Create `tests/runtime/process_exit.nova`:

```nova
// `exit(3)` ends the process at once with status 3, after flushing stdout.
// `partial` has no newline, so only that flush can deliver it, and the line
// after `exit` must never run. Run by `process_exit_run` and
// `process_exit_build_standalone` in `crates/nova-cli/tests/run_tests.rs`.

fn main() {
    print("partial")
    exit(3)
    println(" never printed")
}
```

In `crates/nova-cli/tests/run_tests.rs`, after `http_server_dispatch_run`, add:

```rust

/// `std/process`'s `args()` under `nova run FILE -- ARGS`: index 0 is FILE as
/// written, and an argument with spaces and an empty one each arrive whole.
/// Run from the repository root so FILE is a fixed relative path. Design:
/// docs/superpowers/specs/2026-10-05-examples-04-todo-cli-design.md §7.3.
#[test]
fn process_args_run() {
    let expected = std::fs::read_to_string(repo_root().join("tests/runtime/process_args.stdout"))
        .expect("expected-output fixture exists")
        .replace("\r\n", "\n");
    nova()
        .current_dir(repo_root())
        .arg("run")
        .arg("tests/runtime/process_args.nova")
        .arg("--")
        .args(["one", "two words", ""])
        .assert()
        .success()
        .stdout(expected);
}

/// Without `--`, `nova run` behaves as it always has, and `args()` is just
/// FILE.
#[test]
fn process_args_without_a_separator_is_just_the_file() {
    nova()
        .current_dir(repo_root())
        .arg("run")
        .arg("tests/runtime/process_args.nova")
        .assert()
        .success()
        .stdout("count: 1\n[tests/runtime/process_args.nova]\n");
}

/// `exit(3)` under `nova run`: the `nova` process ends with status 3, and the
/// partial line written before it was flushed.
#[test]
fn process_exit_run() {
    nova()
        .arg("run")
        .arg(repo_root().join("tests/runtime/process_exit.nova"))
        .assert()
        .code(3)
        .stdout("partial");
}

/// `exit(3)` in a built executable, where Rust's own `main` never ran: the
/// status is 3 and the partial line is still flushed.
#[test]
fn process_exit_build_standalone() {
    let dir = std::env::temp_dir().join(format!("nova-build-tests-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("temp dir");
    let exe = dir.join(format!("process_exit{}", std::env::consts::EXE_SUFFIX));
    nova()
        .arg("build")
        .arg(repo_root().join("tests/runtime/process_exit.nova"))
        .arg("-o")
        .arg(&exe)
        .assert()
        .success();
    Command::new(&exe).assert().code(3).stdout("partial");
    let _ = std::fs::remove_file(&exe);
}
```

- [ ] **Step 2: Run them to see them fail**

Run: `cd /d/Projects/nona/nova && cargo build -p nova-runtime 2>&1 | tail -1 && cargo test -p nova-cli --test run_tests process_ 2>&1 | grep -E "^test |test result|E0001|unexpected argument" | head -12`
Expected: all four FAIL. The `--` runs exit 2 on `unexpected argument`; the others fail to compile the fixture with `E0001` naming `args` or `exit`.

- [ ] **Step 3: Write `std/process`**

Create `std/process/lib.nova`:

```nova
// Nova standard library -- the running process: its arguments, and a way to
// end it with a status.
//
// Compiled as an implicit module and glob-imported into every user module, so
// `args()` and `exit(1)` need no `import` (docs/adr/0004-stdlib-compile-model.md).
// A program's own `args` or `exit` shadows these.
//
// Of `nova-spec/20-STDLIB.md`'s module-index entry, "spawn, env, args", only
// `args` ships, with `exit`; `spawn` and `env` do not exist. See that file's
// section 17, and docs/adr/0023-program-arguments.md.

// The program's arguments. Index 0 is the program itself: under
// `nova run [FILE] -- ARGS` it is FILE as written (`src/main.nova` when
// defaulted), and in a built executable it is whatever the OS reports. The
// arguments after `--`, or a built executable's own, follow from index 1. One
// that is not valid Unicode has U+FFFD in place of its bad bytes.
pub fn args() -> Vec<String> {
    let mut v: Vec<String> = Vec::new()
    let n = process_arg_count()
    let mut i = 0
    while i < n {
        v.push(process_arg(i))
        i = i + 1
    }
    v
}

// Ends the process now with `code`, after flushing stdout. Tasks still running
// are not joined. The OS truncates `code`: Unix keeps its low 8 bits, so
// `exit(256)` is 0 and `exit(-1)` is 255; Windows keeps 32. Under `nova run` it
// ends the `nova` process itself.
//
// Typed `-> ()` because Nova has no never type: code after `exit(..)`
// type-checks, and never runs.
pub fn exit(code: Int) {
    process_exit(code)
}
```

In `crates/nova-resolver/src/lib.rs`, change `pub const STD_MODULES: [(&str, &str); 15] = [` to `pub const STD_MODULES: [(&str, &str); 16] = [`, and after `("$std.crypto", include_str!("../../../std/crypto/lib.nova")),` add:

```rust
    ("$std.process", include_str!("../../../std/process/lib.nova")),
```

- [ ] **Step 4: Pass the arguments from `nova run` to the runtime**

In `crates/nova-codegen-cranelift/src/lib.rs`, in `impl CompiledProgram`, after `run`, add:

```rust

    /// Execute `main` with `args` as the program's arguments: the list
    /// `std/process`'s `args()` returns. The runtime keeps the first list it
    /// is given for the rest of the process (`nova_runtime::process`).
    pub fn run_with_args(&self, args: Vec<String>) {
        let _ = nova_runtime::process::set_args(args);
        self.run();
    }
```

In `crates/nova-driver/src/lib.rs`, replace `run_file` with:

```rust
/// Compile and immediately execute a file (`nova run`), with `args` as the
/// program's arguments: `std/process`'s `args()` returns exactly this list.
pub fn run_file(path: &Path, args: Vec<String>) -> Result<Outcome<()>> {
    match compile_file(path)? {
        Outcome::Ok(program) => {
            program.run_with_args(args);
            Ok(Outcome::Ok(()))
        }
        Outcome::Failed { errors } => Ok(Outcome::Failed { errors }),
    }
}
```

In `crates/nova-cli/src/cmd/run.rs`, replace `RunCmd` with:

```rust
#[derive(Args)]
pub struct RunCmd {
    /// Path to the Nova source file to run (default: src/main.nova).
    file: Option<PathBuf>,

    /// Arguments for the program, after `--`: `nova run [FILE] -- ARGS...`.
    /// The program's `args()` is FILE followed by these.
    #[arg(last = true)]
    args: Vec<std::ffi::OsString>,
}
```

and replace the start of `run` through its `match`:

```rust
pub fn run(cmd: RunCmd) -> Result<()> {
    let file = default_file(cmd.file);
    let mut args = vec![file.to_string_lossy().into_owned()];
    args.extend(cmd.args.iter().map(|a| a.to_string_lossy().into_owned()));
    match nova_driver::run_file(&file, args)? {
```

leaving the `match`'s two arms as they are.

`args` is `Vec<OsString>`, where spec §4.4 writes `Vec<String>`. clap rejects a `String` argument that is not valid UTF-8, and spec §4.2 requires such an argument to be converted with `to_string_lossy` instead. Taking `OsString` is what satisfies §4.2 under `nova run`.

- [ ] **Step 5: Run the fixtures, then the suites a new std module could disturb**

Run: `cd /d/Projects/nona/nova && cargo build -p nova-runtime 2>&1 | tail -1 && cargo test -p nova-cli --test run_tests process_ 2>&1 | grep -E "^test |test result" && cargo test -p nova-resolver -p nova-typeck -p nova-driver 2>&1 | grep -E "test result|FAILED|panicked" | sort | uniq -c`
Expected:
- The four `process_*` tests pass.
- Every `test result` line of the resolver, typechecker and driver suites reads `ok`.

If a fixture fails on a `nova` diagnostic, read it and fix `std/process/lib.nova`. Never change a fixture to match a wrong output.

- [ ] **Step 6: Commit**

```bash
cd /d/Projects/nona/nova && cargo fmt --all -- --check && git add std/process/lib.nova crates/nova-resolver/src/lib.rs crates/nova-codegen-cranelift/src/lib.rs crates/nova-driver/src/lib.rs crates/nova-cli/src/cmd/run.rs tests/runtime/process_args.nova tests/runtime/process_args.stdout tests/runtime/process_exit.nova crates/nova-cli/tests/run_tests.rs && git commit -m "feat(std/process): args() and exit(), and nova run [FILE] -- ARGS

std/process is a new std module, written in Nova over the three
process builtins. nova run passes [FILE, ARGS...] to the driver, which
hands them to the runtime through CompiledProgram::run_with_args before
main. Fixtures pin index 0, an argument with spaces, an empty argument,
the no-separator case, and exit's code and flush under nova run and in
a built executable.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

If `cargo fmt --all -- --check` reports a diff, run `cargo fmt --all`, re-run Step 5, then commit.

---

### Task 4: `02-fibonacci` reads its argument

**Files:**
- Modify: `examples/02-fibonacci/src/main.nova`
- Modify: `crates/nova-cli/tests/run_tests.rs`: `gate_2_fibonacci_runs`, and a new test after it

**Interfaces:**
- Consumes: `args()` (Task 3), `nova run [FILE] -- ARGS` (Task 3), `std/json`'s `parse` and `Int::from_json`.
- Produces: the §2 gate.

- [ ] **Step 1: Write the failing tests**

In `crates/nova-cli/tests/run_tests.rs`, in `gate_2_fibonacci_runs`, change:

```rust
        .stdout("fibonacci(10) = 55\n");
```

to:

```rust
        .stdout("fib(10) = 55\n");
```

and after `gate_2_fibonacci_runs`, add:

```rust

/// `60-EXAMPLES.md` §2's gate, as written: `nova run -- 20` in the example's
/// own directory prints `fib(20) = 6765`.
#[test]
fn gate_2_fibonacci_reads_its_argument() {
    nova()
        .current_dir(repo_root().join("examples/02-fibonacci"))
        .arg("run")
        .arg("--")
        .arg("20")
        .assert()
        .success()
        .stdout("fib(20) = 6765\n");
}
```

- [ ] **Step 2: Run them to see them fail**

Run: `cd /d/Projects/nona/nova && cargo test -p nova-cli --test run_tests gate_2_fibonacci 2>&1 | grep -E "^test |test result"`
Expected: both FAIL; the example still prints `fibonacci(10) = 55`.

- [ ] **Step 3: Read the argument**

Replace `examples/02-fibonacci/src/main.nova` with:

```nova
fn fibonacci(n: Int) -> Int {
    if n <= 1 {
        n
    } else {
        fibonacci(n - 1) + fibonacci(n - 2)
    }
}

// The first argument as an Int, or 10 when it is absent or not a number. It
// goes through `std/json`'s `parse`, since there is no `parse::<Int>()`, so
// ` 20 ` and `2e1` also mean 20.
fn arg_n() -> Int {
    let argv = args()
    match argv.get(1) {
        Some(s) => {
            match parse(s) {
                Ok(v) => {
                    match Int::from_json(v) {
                        Ok(n) => n
                        Err(_) => 10
                    }
                }
                Err(_) => 10
            }
        }
        None => 10
    }
}

fn main() {
    let n = arg_n()
    println("fib(${n}) = ${fibonacci(n)}")
}
```

- [ ] **Step 4: Run them to see them pass**

Run: `cd /d/Projects/nona/nova && cargo test -p nova-cli --test run_tests gate_2_fibonacci 2>&1 | grep -E "^test |test result"`
Expected: both pass.

- [ ] **Step 5: Check the literal gate command on this host**

Run: `cd /d/Projects/nona/nova/examples/02-fibonacci && ../../target/debug/nova.exe run -- 20; echo "exit=$?"; ../../target/debug/nova.exe run -- abc`
Expected: `fib(20) = 6765`, `exit=0`, then `fib(10) = 55`.

- [ ] **Step 6: Commit**

```bash
cd /d/Projects/nona/nova && cargo fmt --all -- --check && git add examples/02-fibonacci/src/main.nova crates/nova-cli/tests/run_tests.rs && git commit -m "feat(examples): 02-fibonacci reads its argument, meeting its gate

nova run -- 20 now prints fib(20) = 6765, 60-EXAMPLES section 2's gate,
never met before. With no argument, or one that is not a number, n is
10, and the output follows the listing's fib(n) format.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 5: `examples/04-todo-cli`, its README and its end-to-end tests

**Files:**
- Create: `examples/04-todo-cli/src/main.nova`, `examples/04-todo-cli/README.md`
- Modify: `.gitignore`
- Modify: `crates/nova-cli/tests/run_tests.rs`: append helpers and seven tests

**Interfaces:**
- Consumes: `args()`, `exit()` and `nova run [FILE] -- ARGS` (Task 3); `std/fs`'s `exists`, `read_to_string`, `write_string`; `std/json`'s `parse`, `stringify`, `FromJson`, `JsonValue`, `JsonError`.
- Produces: the §4 gate artifact. Tests:
  - `todo_cli_cycle_under_nova_run`
  - `todo_cli_cycle_as_a_built_executable`
  - `todo_cli_keeps_a_thai_title_under_nova_run`
  - `todo_cli_keeps_a_thai_title_as_a_built_executable`
  - `todo_cli_without_a_command_prints_usage_and_exits_1`
  - `todo_cli_refuses_a_corrupt_todos_json_and_leaves_it_alone`
  - `todo_cli_done_with_an_unknown_id_changes_nothing`

- [ ] **Step 1: Write the end-to-end tests**

Append to `crates/nova-cli/tests/run_tests.rs`:

```rust

// ---------------------------------------------------------------------------
// `examples/04-todo-cli`: the Phase 2 gate in `nova-spec/60-EXAMPLES.md` §4.
// Design: docs/superpowers/specs/2026-10-05-examples-04-todo-cli-design.md
// §7.4, and the plan's Review Focus.

/// A fresh, empty directory for one 04 test. `todos.json` lives in the
/// working directory, so no two tests may share one. Each tag is unique to
/// its test, so removing a stale one races with no sibling.
fn todo_workdir(tag: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("nova-todo-{}-{tag}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("todo work dir");
    dir
}

/// `nova run examples/04-todo-cli/src/main.nova -- ARGS`, in `dir`.
fn todo_run(dir: &std::path::Path, args: &[&str]) -> std::process::Output {
    nova()
        .current_dir(dir)
        .arg("run")
        .arg(repo_root().join("examples/04-todo-cli/src/main.nova"))
        .arg("--")
        .args(args)
        .output()
        .expect("run nova")
}

/// `examples/04-todo-cli` built into `bin`.
fn todo_build(bin: &std::path::Path) -> std::path::PathBuf {
    let exe = bin.join(format!("todo{}", std::env::consts::EXE_SUFFIX));
    nova()
        .arg("build")
        .arg(repo_root().join("examples/04-todo-cli/src/main.nova"))
        .arg("-o")
        .arg(&exe)
        .assert()
        .success();
    exe
}

/// The built `exe`, run in `dir` with real argv.
fn todo_exe(exe: &std::path::Path, dir: &std::path::Path, args: &[&str]) -> std::process::Output {
    std::process::Command::new(exe)
        .current_dir(dir)
        .args(args)
        .output()
        .expect("run the built todo")
}

fn stdout_of(out: &std::process::Output) -> String {
    String::from_utf8_lossy(&out.stdout).replace("\r\n", "\n")
}

fn stderr_of(out: &std::process::Output) -> String {
    String::from_utf8_lossy(&out.stderr).replace("\r\n", "\n")
}

/// The gate's cycle, add → list → done → list, through `run`: every step's
/// output, then the file. `add buy milk` is two arguments and `add "walk
/// dog"` is one; both must store a two-word title.
fn todo_cycle(dir: &std::path::Path, run: &dyn Fn(&[&str]) -> std::process::Output) {
    let step = |args: &[&str], want: &str| {
        let out = run(args);
        assert!(
            out.status.success(),
            "{args:?}: {:?}; stderr={}",
            out.status,
            stderr_of(&out)
        );
        assert_eq!(stdout_of(&out), want, "{args:?}");
    };
    step(&["add", "buy", "milk"], "added: 1\n");
    step(&["add", "walk dog"], "added: 2\n");
    step(&["list"], "[ ] 1: buy milk\n[ ] 2: walk dog\n");
    step(&["done", "1"], "");
    step(&["list"], "[x] 1: buy milk\n[ ] 2: walk dog\n");
    let file = std::fs::read_to_string(dir.join("todos.json")).expect("todos.json written");
    assert_eq!(
        file,
        r#"[{"id":1,"title":"buy milk","done":true},{"id":2,"title":"walk dog","done":false}]"#
    );
}

/// The gate under `nova run`.
#[test]
fn todo_cli_cycle_under_nova_run() {
    let dir = todo_workdir("run-cycle");
    todo_cycle(&dir, &|args| todo_run(&dir, args));
}

/// The gate as a built executable, whose arguments come from the OS argv:
/// the measurement of docs/adr/0023's decision 3 on each CI operating system.
#[test]
fn todo_cli_cycle_as_a_built_executable() {
    let bin = todo_workdir("built-bin");
    let exe = todo_build(&bin);
    let dir = todo_workdir("built-cycle");
    todo_cycle(&dir, &|args| todo_exe(&exe, &dir, args));
}

/// A Thai title survives the arguments, `todos.json` and `list` (Review
/// Focus 1).
fn todo_thai(dir: &std::path::Path, run: &dyn Fn(&[&str]) -> std::process::Output) {
    let title = "ซื้อนม";
    let added = run(&["add", title]);
    assert!(added.status.success(), "add: {}", stderr_of(&added));
    assert_eq!(stdout_of(&run(&["list"])), format!("[ ] 1: {title}\n"));
    let file = std::fs::read_to_string(dir.join("todos.json")).expect("todos.json written");
    assert_eq!(
        file,
        format!("[{{\"id\":1,\"title\":\"{title}\",\"done\":false}}]")
    );
}

#[test]
fn todo_cli_keeps_a_thai_title_under_nova_run() {
    let dir = todo_workdir("thai-run");
    todo_thai(&dir, &|args| todo_run(&dir, args));
}

#[test]
fn todo_cli_keeps_a_thai_title_as_a_built_executable() {
    let bin = todo_workdir("thai-bin");
    let exe = todo_build(&bin);
    let dir = todo_workdir("thai-built");
    todo_thai(&dir, &|args| todo_exe(&exe, &dir, args));
}

/// No command, or an unknown one: the usage line on stderr, exit 1, and no
/// file written.
#[test]
fn todo_cli_without_a_command_prints_usage_and_exits_1() {
    let dir = todo_workdir("usage");
    for args in [&[][..], &["frobnicate"][..]] {
        let out = todo_run(&dir, args);
        assert_eq!(out.status.code(), Some(1), "{args:?}");
        assert_eq!(
            stderr_of(&out),
            "usage: todo {add <title> | list | done <id>}\n",
            "{args:?}"
        );
        assert!(out.stdout.is_empty(), "{args:?}");
    }
    assert!(!dir.join("todos.json").exists());
}

/// A `todos.json` that is not JSON, or not a list of todos, is refused by
/// every command and never written (Review Focus 3).
#[test]
fn todo_cli_refuses_a_corrupt_todos_json_and_leaves_it_alone() {
    let dir = todo_workdir("corrupt");
    let cases: [(&str, &[&str]); 4] = [
        ("{not json", &["list"]),
        ("{not json", &["add", "x"]),
        ("{not json", &["done", "1"]),
        (r#"[{"id":1}]"#, &["list"]),
    ];
    for (content, args) in cases {
        std::fs::write(dir.join("todos.json"), content).expect("write the corrupt file");
        let out = todo_run(&dir, args);
        assert_eq!(out.status.code(), Some(1), "{content:?} {args:?}");
        let err = stderr_of(&out);
        assert!(
            err.starts_with("todo: cannot use todos.json: "),
            "{content:?} {args:?}: {err:?}"
        );
        assert_eq!(
            std::fs::read_to_string(dir.join("todos.json")).expect("still there"),
            content,
            "{content:?} {args:?} must leave the file alone"
        );
    }
}

/// `done` with an id no todo has, or one that is not a number, changes
/// nothing and prints nothing.
#[test]
fn todo_cli_done_with_an_unknown_id_changes_nothing() {
    let dir = todo_workdir("unknown-id");
    assert!(todo_run(&dir, &["add", "a"]).status.success());
    let before = std::fs::read(dir.join("todos.json")).expect("written");
    for id in ["99", "x", ""] {
        let out = todo_run(&dir, &["done", id]);
        assert!(out.status.success(), "{id:?}: {}", stderr_of(&out));
        assert!(out.stdout.is_empty(), "{id:?}");
        assert_eq!(
            std::fs::read(dir.join("todos.json")).expect("still there"),
            before,
            "{id:?}"
        );
    }
}
```

- [ ] **Step 2: Run them to see them fail**

Run: `cd /d/Projects/nona/nova && cargo build -p nova-runtime 2>&1 | tail -1 && cargo test -p nova-cli --test run_tests todo_cli 2>&1 | grep -E "^test |panicked|test result" | head -20`
Expected: all seven FAIL, because `examples/04-todo-cli/src/main.nova` does not exist: `nova` reports `failed to open`, and `nova build` fails.

- [ ] **Step 3: Write the example**

Create `examples/04-todo-cli/src/main.nova`:

```nova
// Phase 2's gate example in `nova-spec/60-EXAMPLES.md` section 4: a todo list
// kept in `todos.json`, driven by the command line.
//
// **This is not that section's listing, deliberately.** The listing is kept
// as the aspiration it was, the precedent `examples/05-json-api` set, and this
// runs the same cycle in the Nova that exists. Every substitution, and the
// evidence for it, is in
// `docs/superpowers/specs/2026-10-05-examples-04-todo-cli-design.md` section 3.
//
// Two behaviours differ from the listing by the user's decision of
// 2026-10-05: a `todos.json` that does not parse is refused rather than
// overwritten, and `add`'s title is every argument after it.

record Todo { id: Int, title: String, done: Bool }

const DB_PATH: String = "todos.json"

const USAGE: String = "usage: todo {add <title> | list | done <id>}"

// ---------------------------------------------------------------------------
// JSON, by hand: there is no `@derive`.

fn field(m: Map<String, JsonValue>, k: String) -> Result<JsonValue, JsonError> {
    match m.get(k) {
        Some(v) => Ok(v)
        None => Err(JsonError { msg: "missing field ${k}", at: 0 })
    }
}

fn todo_of(m: Map<String, JsonValue>) -> Result<Todo, JsonError> {
    let id = match field(m, "id") {
        Ok(v) => {
            match Int::from_json(v) {
                Ok(n) => n
                Err(e) => return Err(e)
            }
        }
        Err(e) => return Err(e)
    }
    let title = match field(m, "title") {
        Ok(v) => {
            match String::from_json(v) {
                Ok(s) => s
                Err(e) => return Err(e)
            }
        }
        Err(e) => return Err(e)
    }
    let done = match field(m, "done") {
        Ok(v) => {
            match Bool::from_json(v) {
                Ok(b) => b
                Err(e) => return Err(e)
            }
        }
        Err(e) => return Err(e)
    }
    Ok(Todo { id: id, title: title, done: done })
}

impl FromJson for Todo {
    fn from_json(v: JsonValue) -> Result<Todo, JsonError> {
        match v {
            Object(m) => todo_of(m)
            _ => Err(JsonError { msg: "expected an object", at: 0 })
        }
    }
}

fn todos_of(v: JsonValue) -> Result<Vec<Todo>, JsonError> {
    match v {
        Array(items) => {
            let mut out: Vec<Todo> = Vec::new()
            let mut i = 0
            while i < items.len() {
                match Todo::from_json(items[i]) {
                    Ok(t) => out.push(t)
                    Err(e) => return Err(e)
                }
                i = i + 1
            }
            Ok(out)
        }
        _ => Err(JsonError { msg: "expected a list of todos", at: 0 })
    }
}

fn todo_json(t: Todo) -> String {
    let done = if t.done { "true" } else { "false" }
    "{\"id\":${t.id},\"title\":${stringify(String(t.title))},\"done\":${done}}"
}

fn todos_json(todos: Vec<Todo>) -> String {
    let mut parts = [""; todos.len()]
    let mut i = 0
    for t in todos.iter() {
        parts[i] = todo_json(t)
        i = i + 1
    }
    "[${",".join(parts)}]"
}

// ---------------------------------------------------------------------------
// The file.

// Reports `todos.json` as unusable and ends the program with status 1,
// leaving the file as it is. Typed to return a list so it can stand in a
// `match` arm; it never returns.
fn bad_file(reason: String) -> Vec<Todo> {
    eprintln("todo: cannot use ${DB_PATH}: ${reason}")
    exit(1)
    Vec::new()
}

// The stored list, or an empty one when there is no file. A file that cannot
// be read, parsed or decoded is never treated as empty: that would let the
// next save overwrite it.
async fn load_todos() -> Vec<Todo> {
    if !exists(DB_PATH).await {
        return Vec::new()
    }
    match read_to_string(DB_PATH).await {
        Ok(text) => {
            match parse(text) {
                Ok(v) => {
                    match todos_of(v) {
                        Ok(todos) => todos
                        Err(e) => bad_file(e.msg)
                    }
                }
                Err(e) => bad_file(e.msg)
            }
        }
        Err(e) => bad_file(e.message)
    }
}

async fn save_todos(todos: Vec<Todo>) {
    match write_string(DB_PATH, todos_json(todos)).await {
        Ok(_) => {}
        Err(e) => {
            eprintln("todo: cannot write ${DB_PATH}: ${e.message}")
            exit(1)
        }
    }
}

// ---------------------------------------------------------------------------
// The commands.

// Every argument after `add`, joined with single spaces, so `add buy milk` and
// `add "buy milk"` store the same title; `untitled` if there are none.
fn title_of(argv: Vec<String>) -> String {
    let n = argv.len() - 2
    if n <= 0 {
        return "untitled"
    }
    let mut words = [""; n]
    let mut i = 0
    while i < n {
        match argv.get(i + 2) {
            Some(w) => {
                words[i] = w
            }
            None => {}
        }
        i = i + 1
    }
    " ".join(words)
}

fn next_id(todos: Vec<Todo>) -> Int {
    let mut top = 0
    for t in todos.iter() {
        if t.id > top {
            top = t.id
        }
    }
    top + 1
}

// `s` as an Int, or 0, which no todo has. It goes through `std/json`'s
// `parse`, as `examples/05-json-api`'s `path_id` does, so ` 7 ` and `7.0`
// count as 7 too.
fn id_of(s: String) -> Int {
    match parse(s) {
        Ok(v) => {
            match Int::from_json(v) {
                Ok(n) => n
                Err(_) => 0
            }
        }
        Err(_) => 0
    }
}

async fn cmd_add(argv: Vec<String>) {
    let title = title_of(argv)
    let mut todos = load_todos().await
    let id = next_id(todos)
    todos.push(Todo { id: id, title: title, done: false })
    save_todos(todos).await
    println("added: ${id}")
}

async fn cmd_list() {
    let todos = load_todos().await
    for t in todos.iter() {
        let mark = if t.done { "[x]" } else { "[ ]" }
        println("${mark} ${t.id}: ${t.title}")
    }
}

// Marks todo `id` done. Records alias, so setting `done` on the loop's
// rebound copy sets it on the todo inside the `Vec`. An unknown id changes
// nothing, as in the listing.
async fn cmd_done(argv: Vec<String>) {
    let id = match argv.get(2) {
        Some(s) => id_of(s)
        None => 0
    }
    let todos = load_todos().await
    for t in todos.iter() {
        if t.id == id {
            let mut u = t
            u.done = true
        }
    }
    save_todos(todos).await
}

async fn main() {
    let argv = args()
    let cmd = match argv.get(1) {
        Some(c) => c
        None => ""
    }
    match cmd {
        "add" => cmd_add(argv).await
        "list" => cmd_list().await
        "done" => cmd_done(argv).await
        _ => {
            eprintln(USAGE)
            exit(1)
        }
    }
}
```

This code, and Task 4's 02 code, were compiled and run on 2026-10-05 against stand-in `args()` and `exit()` definitions, before either existed. The cycle printed `added: 1`, `added: 2`, both lists and nothing for `done`, and wrote exactly the file Step 1 expects. A corrupt file was reported as `todo: cannot use todos.json: expected an object key` and left untouched. 02 printed `fib(20) = 6765`, and `fib(10) = 55` for no argument and for `abc`.

Create `examples/04-todo-cli/README.md`:

````markdown
# 04-todo-cli

A todo list kept in `todos.json`, driven by the command line.

## What this demonstrates

- **`std/process`:** `args()` reads the command line, and `exit(1)` reports a
  usage error or a bad file.
- **`std/fs`:** `exists`, `read_to_string` and `write_string`, all `async`.
- **`std/json`:** `parse`, a hand-written `FromJson` for the `Todo` record, and
  JSON text built with `stringify`.
- **Records, `Vec`, and a `match` on a string.**

## Run it

```bash
cd examples/04-todo-cli
nova run -- add buy milk
nova run -- add walk dog
nova run -- list
nova run -- done 1
nova run -- list
```

Or build it once, and pass the same arguments to the executable. `nova build`
names it `main` (`main.exe` on Windows) in the current directory:

```bash
nova build
./main list
```

## Expected output

```
added: 1
added: 2
[ ] 1: buy milk
[ ] 2: walk dog
[x] 1: buy milk
[ ] 2: walk dog
```

`done 1` prints nothing. `todos.json` then holds:

```
[{"id":1,"title":"buy milk","done":true},{"id":2,"title":"walk dog","done":false}]
```

## Notes

- **This is not `nova-spec/60-EXAMPLES.md` §4's listing, on purpose.** That
  listing is written in a Nova that does not exist, starting with its first
  line, `import std/fs`, which does not parse. It is kept as the aspiration, as
  §5's is. Every substitution this example makes, and the evidence for it, is
  in `docs/superpowers/specs/2026-10-05-examples-04-todo-cli-design.md` §3.
- **`todos.json` lives in the working directory,** as compact JSON. The
  repository's `.gitignore` ignores it in this folder.
- **A `todos.json` that does not parse is refused, not overwritten:** every
  command prints `todo: cannot use todos.json: <reason>` on stderr and exits 1.
  Move the file aside to start over.
- **A title is every word after `add`,** so `add buy milk` and
  `add "buy milk"` store the same title.
- **`done` with an unknown id changes nothing,** silently, as the listing does.
  Ids are read through `std/json`'s `parse`, so `7.0` also means 7.
- **The tests** are the `todo_cli_*` functions in
  `crates/nova-cli/tests/run_tests.rs`, not a `tests/` folder here. They run the
  cycle under `nova run` and as a built executable.
````

In `.gitignore`, after the line `.env.local`, add:

```
/examples/04-todo-cli/todos.json
```

- [ ] **Step 4: Run the tests to see them pass**

Run: `cd /d/Projects/nona/nova && cargo build -p nova-runtime 2>&1 | tail -1 && cargo test -p nova-cli --test run_tests todo_cli 2>&1 | grep -E "^test |panicked|test result"`
Expected: all seven pass.

If one fails, read its message; every assertion prints the arguments and stderr.
- A `nova` diagnostic means the example does not compile. The example follows shapes that already compile in `examples/05-json-api`: nested `match`, no line starting with `(` or `[`, and `return` inside a `let`-bound `match`. Fix the example, never the test.
- A built-executable test that fails while its `nova run` sibling passes is decision 3 of ADR 0023 failing. Stop and report it: it is the design's one reasoned assumption.

- [ ] **Step 5: Check the literal gate cycle on this host, in a scratch directory**

Run: `S=/c/Users/SAKEER~1/AppData/Local/Temp/gcm/todo-gate; rm -rf $S; mkdir -p $S; cd $S && N=/d/Projects/nona/nova/target/debug/nova.exe; F=/d/Projects/nona/nova/examples/04-todo-cli/src/main.nova; $N run $F -- add buy milk; $N run $F -- add walk dog; $N run $F -- list; $N run $F -- done 1; $N run $F -- list; cat todos.json; echo; $N run $F -- oops; echo "exit=$?"`
Expected: `added: 1`, `added: 2`, two unchecked lines, then `[x] 1: buy milk` and `[ ] 2: walk dog`, the compact JSON, the usage line, and `exit=1`. Nothing runs in the background, so nothing outlives the command.

- [ ] **Step 6: Commit**

```bash
cd /d/Projects/nona/nova && cargo fmt --all -- --check && git add examples/04-todo-cli .gitignore crates/nova-cli/tests/run_tests.rs && git commit -m "feat(examples): 04-todo-cli, the 60-EXAMPLES section 4 gate

add, list and done over todos.json in the working directory, through
std/process's args() and exit(). The end-to-end tests drive:
- the add -> list -> done -> list cycle under nova run and as a built
  executable;
- a Thai title, both ways;
- the usage error (exit 1);
- a corrupt todos.json, refused and left untouched;
- done with an unknown id.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

If `cargo fmt --all -- --check` reports a diff, run `cargo fmt --all`, re-run Step 4, then commit.

---

### Task 6: Records: dated notes, `std/process`'s section, the CHANGELOG, and the sweep

**Files:** (all modify)
- `nova-spec/60-EXAMPLES.md` (§2, §4, §9, §10)
- `nova-spec/20-STDLIB.md` (the module-index note, the `lib.nova` count chain, the gate-remeasure-7 paragraph, and a new §17)
- `nova-spec/13-RUNTIME.md` (§7, and the gate-remeasure-7 paragraph)
- `nova-spec/00-MASTER-SPEC.md` (the gate-remeasure-7 paragraph, and §9's example notes)
- `docs/phase-2-plan.md` (the gate-remeasure-7 paragraph)
- `docs/superpowers/specs/2026-10-04-examples-03-04-inventory.md`
- `CHANGELOG.md` (`[Unreleased]` Added, and the inventory bullet's marker)

**Interfaces:**
- Consumes: the names and behaviour from Tasks 1-5.
- Produces: no code.

- [ ] **Step 1: Apply every note with one script**

Write this script with the Write tool to `C:/Users/SAKEER~1/AppData/Local/Temp/gcm/_once/t6_records.py`, run it with `python -X utf8`, then rename it to `t6_records.py.applied`. Every anchor below was checked on 2026-10-05 to match exactly once. If one has moved, the script aborts before writing anything: re-read that file, fix the anchor, and re-run.

```python
import sys
ROOT = "D:/Projects/nona/nova/"

def run(edits):
    state = {}
    for f, anchor, text, mode in edits:
        if f not in state:
            raw = open(ROOT + f, encoding="utf-8", newline="").read()
            state[f] = [raw, "\r\n" if "\r\n" in raw else "\n"]
        raw, nl = state[f]
        a = anchor.replace("\n", nl)
        t = text.replace("\n", nl)
        n = raw.count(a)
        if n != 1:
            sys.exit("ABORT before any write: %s: anchor matched %d times: %r" % (f, n, anchor[:70]))
        if mode == "after":
            state[f][0] = raw.replace(a, a + t)
        elif mode == "before":
            state[f][0] = raw.replace(a, t + a)
        else:
            state[f][0] = raw.replace(a, t)
    for f, (raw, nl) in state.items():
        open(ROOT + f, "w", encoding="utf-8", newline="").write(raw)
        print("wrote", f)

GATE7_04 = '`04-todo-cli` still does not exist, so Phase 2 is still not complete.\n'
GATE7_04_NOTE = """
**Recorded 2026-10-05 (branch `examples-04-todo-cli`):** `04-todo-cli` now
exists too, and end-to-end tests of its gate run on all three CI operating
systems; see `nova-spec/60-EXAMPLES.md` §4. Every example that file labels a
Phase 2 gate (§3, §4 and §5) now exists and passes. This note does not assess
whether Phase 2 is complete.
"""

EDITS = []

EDITS.append(("nova-spec/60-EXAMPLES.md",
"**Gate:** `nova run -- 20` outputs `fib(20) = 6765`\n",
"""
**Recorded 2026-10-05 (branch `examples-04-todo-cli`): this gate is met, by an
example written in today's Nova.** `nova run -- 20` in `examples/02-fibonacci`
prints `fib(20) = 6765`; with no argument it prints `fib(10) = 55`. It needed
`std/process`'s `args()` and `nova run`'s `--` pass-through, both added on that
branch (`docs/adr/0023-program-arguments.md`). The listing is unchanged; its
substitutions are in
`docs/superpowers/specs/2026-10-05-examples-04-todo-cli-design.md` §3.
""", "after"))

EDITS.append(("nova-spec/60-EXAMPLES.md",
"§4. The listing above is unchanged.\n",
"""
**Recorded 2026-10-05 (branch `examples-04-todo-cli`): the example now exists,
and its gate passes.** `examples/04-todo-cli` runs the add → list → done → list
cycle under `nova run -- ...` and as a built executable, with `todos.json` in
the working directory. What the note above found missing now exists:
`std/process` gives `args()` and `exit(code)`, and `nova run [FILE] -- ARGS`
passes arguments through (`docs/adr/0023-program-arguments.md`). Two behaviours
differ from the listing by the user's decision of 2026-10-05: a `todos.json`
that does not parse is refused rather than overwritten, and `add`'s title is
every argument after it. Every substitution is in
`docs/superpowers/specs/2026-10-05-examples-04-todo-cli-design.md` §3. The
listing above is unchanged.
""", "after"))

EDITS.append(("nova-spec/60-EXAMPLES.md",
"`ls examples/*/README.md` against `ls -d examples/*/`.\n",
"""
**Recorded 2026-10-05 (branch `examples-04-todo-cli`): a third example follows
this template,** `examples/04-todo-cli/README.md`. The durable check is
unchanged.
""", "after"))

EDITS.append(("nova-spec/60-EXAMPLES.md",
"`cargo test` step on all three operating systems.\n",
"""
**Recorded 2026-10-05 (branch `examples-04-todo-cli`):** `04-todo-cli`'s tests
live in `crates/nova-cli/tests/run_tests.rs` too, for the first of the two
reasons above. Whether any example meets this section is a check, not this
note: `ls -d examples/*/tests/` against the CI job above.
""", "after"))

EDITS.append(("nova-spec/20-STDLIB.md",
"`std/process` still have no dedicated numbered section below.\n",
"""
**AMENDED 2026-10-05 (branch `examples-04-todo-cli`): `std/process` now has one,
§17,** appended after §16 for the reason §16's opening note gives. Only `args`
ships, with `exit`; `spawn` and `env` do not exist.
""", "after"))

EDITS.append(("nova-spec/20-STDLIB.md",
"`00-MASTER-SPEC.md` §3's numbered list,\nnot this sentence.\n",
"""
**AMENDED 2026-10-05 (branch `examples-04-todo-cli`): one link further.**
`$std.process` makes it **15 → 16** `STD_MODULES` entries and **16 → 17** files
on disk with `STD_TEST_MODULE`, measured with `find std -name lib.nova`. See
§17.
""", "after"))

EDITS.append(("nova-spec/20-STDLIB.md", GATE7_04, GATE7_04_NOTE, "after"))
EDITS.append(("nova-spec/13-RUNTIME.md", GATE7_04, GATE7_04_NOTE, "after"))
EDITS.append(("nova-spec/00-MASTER-SPEC.md", GATE7_04, GATE7_04_NOTE, "after"))
EDITS.append(("docs/phase-2-plan.md", GATE7_04, GATE7_04_NOTE, "after"))

EDITS.append(("nova-spec/20-STDLIB.md",
"is *unknown* rather than claimed. UDP and Unix sockets stay unbuilt, and\n`IoErrorKind` gains no variant.\n",
"""
---

## 17. `std/process`

**Added 2026-10-05 (branch `examples-04-todo-cli`), numbered out of the
module-index order** for the reason §16's opening note gives. Of that index's
"spawn, env, args", only `args` ships, with an `exit`; `spawn` and `env` do not
exist.

```nova
// The program's arguments. Index 0 is the program itself: under
// `nova run [FILE] -- ARGS` it is FILE as written (`src/main.nova` when
// defaulted); in a built executable, whatever the OS reports.
pub fn args() -> Vec<String>

// Ends the process now with `code`, after flushing stdout. Running tasks are
// not joined. The OS truncates `code`: Unix keeps its low 8 bits.
pub fn exit(code: Int)
```

- `nova run [FILE] -- ARGS` passes ARGS to the program. Without `--`, `args()`
  is `[FILE]`.
- An argument that is not valid Unicode arrives with U+FFFD in place of its bad
  bytes.
- Both are glob-imported like every std module, and a program's own `args` or
  `exit` shadows them.
- Three `STD_ONLY` builtins back them, `process_arg_count`, `process_arg` and
  `process_exit`, so `STD_ONLY` grows from 78 to 81.
- See `docs/adr/0023-program-arguments.md`.
""", "after"))

EDITS.append(("nova-spec/13-RUNTIME.md",
"## 8. WASM Runtime (Phase 4)\n",
"""**AMENDED 2026-10-05 (branch `examples-04-todo-cli`): two more runtime hooks,
for a program's arguments and its exit.** `crates/nova-runtime/src/process.rs`
holds the argument list `std/process`'s `args()` reads. `nova run` sets it from
what follows `--`. In a built executable nothing sets it, and the runtime reads
the OS's own argv through `std::env::args_os()`, which works without Rust's
`main` on glibc, macOS and Windows. `exit(code)` calls `std::process::exit`,
which flushes stdout and does not join running tasks. See
`docs/adr/0023-program-arguments.md`.

""", "before"))

EDITS.append(("nova-spec/00-MASTER-SPEC.md",
"`ls -d examples/*/` and `ls examples/*/README.md`, not this note.]\n",
"""
[Amended 2026-10-05, branch `examples-04-todo-cli`: `examples/04-todo-cli/` now
exists, so Section 2's `04-todo-cli/` entry is no longer ahead of the disk
either. A third example now has the §9 README, `examples/04-todo-cli/README.md`.
The durable checks are unchanged.]
""", "after"))

EDITS.append(("docs/superpowers/specs/2026-10-04-examples-03-04-inventory.md",
"  moves that listing again.\n",
"""
**Recorded 2026-10-05 (branch `examples-04-todo-cli`): §4.3's argument findings
are history too.** `std/process` gives `args()` and `exit(code)`, `nova run
[FILE] -- ARGS` passes arguments through, and a built executable's runtime reads
the OS argv (`docs/adr/0023-program-arguments.md`). §6's steps 2 and 3 are done:
`examples/04-todo-cli` exists, and `02-fibonacci`'s gate is met. The five std
additions §6 step 3 lists were written around, not added.
""", "after"))

EDITS.append(("CHANGELOG.md",
"\n### Measured\n",
"""- **`examples/04-todo-cli`, the Phase 2 gate in `nova-spec/60-EXAMPLES.md`
  §4.** It keeps a todo list in `todos.json` in the working directory, with
  `add <title>`, `list` and `done <id>`. End-to-end tests on all three CI
  operating systems run the add → list → done → list cycle under `nova run --`
  and as a built executable. They also cover a Thai title, a corrupt
  `todos.json` (refused with exit 1, the file untouched) and the usage error
  (exit 1). The listing is kept as aspiration; the substitutions are in
  `docs/superpowers/specs/2026-10-05-examples-04-todo-cli-design.md` §3.
- **`examples/02-fibonacci` meets its gate for the first time:** `nova run --
  20` prints `fib(20) = 6765`. With no argument it prints `fib(10) = 55`, where
  it used to print `fibonacci(10) = 55`.
- **`std/process`: `args()` and `exit(code)`, over three new `STD_ONLY`
  builtins (`STD_ONLY` 78 → 81).** `args()[0]` is the program: the source file
  under `nova run`, the executable when built. `exit` flushes stdout and ends
  the process with `code`, without joining running tasks. See
  `docs/adr/0023-program-arguments.md`.
- **`nova run [FILE] -- ARGS` passes arguments to the program.** Without `--`,
  `args()` is `[FILE]`.
""", "before"))

EDITS.append(("CHANGELOG.md",
"    arguments through. Separately, `exit(code)` exists only through FFI.\n",
"""    [Amended 2026-10-05, branch `examples-04-todo-cli`: `04-todo-cli` now
    exists, `std/process` gives `args()` and `exit(code)`, and `nova run`
    passes arguments through; see the bullets under Added.]
""", "after"))

run(EDITS)
```

Expected: a `wrote …` line for each of the 7 files, and no `ABORT`.

- [ ] **Step 2: Check the edits are clean**

Run: `cd /d/Projects/nona/nova && git diff --check && git diff --stat && git ls-files --eol nova-spec/60-EXAMPLES.md nova-spec/20-STDLIB.md CHANGELOG.md`
Expected:
- no whitespace errors;
- 7 files changed, insertions only;
- the files still `i/lf w/crlf`.

- [ ] **Step 3: Run the set-difference sweep**

The touched set must include this task's uncommitted edits as well as the branch's commits. Run:

```bash
cd /d/Projects/nona/nova && S=/c/Users/SAKEER~1/AppData/Local/Temp/gcm; for t in "args()" "exit(" "04-todo-cli" "02-fibonacci" "fibonacci(10) = 55" "std/process" "pass arguments" "argv" "STD_ONLY" "STD_MODULES" "lib.nova" "neither exists under"; do git grep -l -F "$t"; done | sort -u > $S/sweep_all.txt && { git diff --name-only main...HEAD; git diff --name-only; } | sort -u > $S/sweep_touched.txt && comm -23 $S/sweep_all.txt $S/sweep_touched.txt
```

Expected: a list of tracked files that name one of those tokens and that this branch did not touch.

For each one:
- Read the matching lines in context, with wrapped prose flattened: a line-oriented `grep` misses a phrase split across lines, so a miss is not absence.
- Decide whether it is now **false**. Records of their own date's state are not false: historical plans, old CHANGELOG releases, and specs that describe what their increment did.
- Give every file now false a dated note in its own amendment style, and list the files you judged still true in your report.

Then re-read the existing dated notes in every touched file for a paragraph this branch made stale that the list above missed. In particular, a count of `STD_ONLY` builtins (78), `STD_MODULES` entries (15) or `lib.nova` files (16) stated as current is now one step behind.

- [ ] **Step 4: Commit**

```bash
cd /d/Projects/nona/nova && git add -A nova-spec docs CHANGELOG.md && git status --short && git commit -m "docs: record std/process, 04-todo-cli and 02's gate

Dated notes in every record this makes stale:
- 60-EXAMPLES sections 2, 4, 9 and 10;
- 20-STDLIB's module-index note and lib.nova count chain, and a new
  section 17 for std/process;
- 13-RUNTIME section 7;
- the gate-remeasure-7 notes in four files;
- MASTER-SPEC section 9's example notes;
- the 03/04 inventory.

The CHANGELOG gets four Added bullets and a marker on the inventory's
own bullet. Facts only: no record declares Phase 2 complete.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 7: Final verification

**Files:** none changed, unless a check fails. If one does, fix it in the task that owns the code and re-run this task.

- [ ] **Step 1: Full build, then the whole suite**

Run: `cd /d/Projects/nona/nova && cargo build -p nova-runtime 2>&1 | tail -1 && cargo build --locked --workspace 2>&1 | tail -1 && cargo test --locked --workspace --all-features --no-fail-fast > /c/Users/SAKEER~1/AppData/Local/Temp/gcm/t7_test.log 2>&1; grep -E "^test result|FAILED|panicked" /c/Users/SAKEER~1/AppData/Local/Temp/gcm/t7_test.log | sort | uniq -c | tail -30`
Expected:
- every `test result` line reads `ok`, with no `FAILED`;
- the new tests appear among the passes: two runtime unit tests, two runtime integration tests, one typechecker test, four `process_*` tests, one new 02 test, and seven `todo_cli_*` tests. That is 17 more than `main`'s 1201 on this host;
- the known `#[ignore]`d GC tests stay ignored.

If an `0xC0000005` crash appears in a `*_build_standalone` or `nova test` product, read `docs/adr/0008-attributes-and-test-isolation.md` §4 before re-running: it is a known flake family, and a re-run of that one test is the recorded response.

- [ ] **Step 2: Lint and format as CI does**

Run: `cd /d/Projects/nona/nova && cargo clippy --locked --all-targets --all-features -- -D warnings 2>&1 | tail -3 && cargo fmt --all -- --check && echo FMT-OK && git diff --exit-code Cargo.lock && echo LOCK-UNCHANGED`
Expected: clippy finishes with no warnings, then `FMT-OK`, then `LOCK-UNCHANGED`.

- [ ] **Step 3: Nothing stray**

Run: `cd /d/Projects/nona/nova && git status --short && ls examples/ && ls std/ && git ls-files 'std/*/lib.nova' | wc -l`
Expected:
- an empty status;
- `examples/` lists `01-hello-world`, `02-fibonacci`, `03-http-server`, `03-producer-consumer`, `04-todo-cli` and `05-json-api`;
- `std/` includes `process`, and there are 17 `lib.nova` files.

- [ ] **Step 4: Hand off to the whole-branch review**

The execution skill's final review takes over from here, followed by `superpowers:finishing-a-development-branch`.

The branch is not done until CI's three operating systems pass. That CI run is the first measurement of ADR 0023's decision 3 on Linux and macOS: a built executable reading the OS argv without Rust's `main`.
