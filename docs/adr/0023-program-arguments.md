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
