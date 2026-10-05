# `examples/04-todo-cli`, and `02-fibonacci`'s gate: design

Branch `examples-04-todo-cli`, from `main` at `f4b2bb4`. The design was approved
section by section in conversation on 2026-10-05; this file is the written spec.

## 1. What this is, and what it is not

**The goal.** `nova-spec/60-EXAMPLES.md` §4's `04-todo-cli` is the last Phase 2
gate example that does not exist. Its gate: "Full CLI cycle works (add → list →
done → list)". The same section's §2 gate for `02-fibonacci`, "`nova run -- 20`
outputs `fib(20) = 6765`", has never been met. One thing blocks both: a Nova
program cannot see its arguments (`docs/superpowers/specs/2026-10-04-examples-03-04-inventory.md`
§4.3).

**The user's decisions, 2026-10-05:**
- Close 02's gate in this branch too.
- Write both examples around the five conveniences std lacks: JSON codecs for
  `Vec<T>`, `stringify_pretty`, an iterator `max`, `Result::ok`, and a
  String-to-Int parse. The only new std surface is `std/process`'s `args()` and
  `exit()`.
- **Approach A** (§4): the runtime holds the argument list. `nova run` sets it;
  a built executable's runtime reads the OS's own argv.
- The end-to-end tests run 04's cycle both under `nova run` and as a built
  executable.
- A `todos.json` that exists but does not parse is refused: an error on stderr,
  exit 1, the file untouched. The listing would read it as empty and overwrite
  it.
- `add`'s title is every argument after `add`, joined with single spaces. The
  listing takes only the one argument after `add`.
- The records state facts only: every example `60-EXAMPLES.md` labels a Phase 2
  gate exists and passes. Declaring Phase 2 complete is a separate step.

**Kept from the 03 and 05 precedents:** the listings stay as written, as the
aspiration, with dated notes; the examples are written in the Nova that exists;
the end-to-end tests live in `crates/nova-cli/tests/run_tests.rs`.

**Not in this branch:** `std/process`'s `spawn` and `env`; the five
conveniences; E-11 (§10); the LLVM backend; a declaration that Phase 2 is
complete.

## 2. Why the listings are not the deliverable

Both listings fail on their first line, where `import std/...` is `P0001`. §4's
listing then reaches for about fifteen constructs today's Nova does not have,
each inventoried with a route that ran (inventory §4.1). §2's listing also
needs the turbofish (`parse::<Int>()`), `String::parse` and `Result::ok`.
`examples/05-json-api` set the precedent: the listing stays as written, and the
example uses today's spellings. §3 lists every substitution.

## 3. Every substitution, and the evidence for it

`04-todo-cli` (evidence IDs are the inventory's, §4.1):

| `60-EXAMPLES.md` §4 writes | The example does | Why |
|---|---|---|
| the four `import std/...` lines | nothing: `println` and `eprintln` are builtins, and std modules are glob-imported | `P0001` (A-1, A-3, E-1, E-2) |
| `@derive(ToJson, FromJson, Clone)` | a hand-written `impl FromJson for Todo` and an encoder function, as 05's `User` has | `E0082` unknown attribute (D-1, D-2) |
| `const DB_PATH = "todos.json"` | `const DB_PATH: String = "todos.json"` | `P0001`: a const needs its type (D-4) |
| `[Todo]` as a growable list | `Vec<Todo>`, iterated with `.iter()` | `E0014`, `E0900` (D-9, F2-1) |
| `fs.exists`, `fs.read`, `String::from_utf8`, `fs.write_string` | `exists`, `read_to_string`, `write_string` | `E0001` on `fs.` and `from_utf8` (GAP-1, F1-10, G-4) |
| `json.parse(s).and_then(\|v\| Vec::<Todo>::from_json(v))` | `parse(s)`, then a hand-written array decoder | turbofish `P0001`; no `FromJson for Vec` (D-7, D-8) |
| `json.stringify_pretty(todos.to_json(), 2)` | compact JSON, built per field through `stringify`, as 05's `user_json` does | `E0001`, `E0014` (A-8, D-6) |
| `args()` | `args()`, from the new `std/process` | this branch |
| `match argv.get(1).map(\|s\| s.as_str()) { Some("add") => ..` | a top-level `match` on the command string, `""` when absent | `E0900` nested patterns; `as_str` `E0014` (F1-3, F1-5) |
| `argv.get(2).unwrap_or("untitled".to_string())` | every argument after `add`, joined with single spaces; `untitled` if there are none | the user's decision |
| `todos.iter().map(\|t\| t.id).max().unwrap_or(0) + 1` | a loop that tracks the largest id | no `max` (F2-2) |
| `s.parse::<Int>().ok()` | `parse(s)` and `Int::from_json`, as 05's `path_id` does | `P0001`, `E0014` (F1-7, F1-8) |
| `for todo in &mut todos { if todo.id == id { todo.done = true } }` | `for t in todos.iter() { let mut u = t; ... }`; records alias, so the write lands in the `Vec` | `E0900`, `E0060` (F2-9, F2-10) |
| `unwrap_or([])` on a file that does not parse | an error on stderr and exit 1, file untouched | the user's decision |
| `exit(1)` | `exit(1)`, from the new `std/process` | this branch |

`02-fibonacci`:

| `60-EXAMPLES.md` §2 writes | The example does | Why |
|---|---|---|
| `import std/fmt { println }`, `import std/process { args }` | nothing | `P0001` |
| `match n { 0 => 0, 1 => 1, n => fib(n - 1) + fib(n - 2) }` | the example's existing `if` recursion, kept | the gate checks output only |
| `argv.get(1).and_then(\|s\| s.parse::<Int>().ok()).unwrap_or(10)` | `args()`, then `parse` and `Int::from_json`; 10 when absent or not a number | `P0001`, `E0014` |
| `println("fib(${n}) = ${fib(n)}")` | the same output; the function keeps its name, `fibonacci` | — |

**`parse` plus `Int::from_json` is not `parse::<Int>()`.** The text goes
through a JSON number, so it also accepts ` 20 ` and `2e1`, and rounds above
2^53 (inventory G-R9). That is harmless for a todo id or a Fibonacci index.

## 4. Architecture

### 4.1 Constraints the design is built around

- **`nova run` runs the program inside `nova.exe`.** The JIT calls `main`
  through `CompiledProgram::run` (`crates/nova-codegen-cranelift/src/lib.rs:59`),
  so the process's own argv is nova's. `RunCmd`
  (`crates/nova-cli/src/cmd/run.rs`) has only `file`.
- **A built executable's entry is a generated C `main` that drops `argc` and
  `argv`.** It calls `nova_main()` and returns 0: Cranelift's `emit_c_main`
  (`crates/nova-codegen-cranelift/src/lib.rs:288-323`), and the LLVM backend's
  (inventory §4.3).
- **On Windows, a C `main`'s `argv` is in the process's ANSI code page** (CP874
  on the development host), so non-ASCII arguments arrive garbled. The UTF-16
  command line, `GetCommandLineW`, is lossless.
- **Rust's `std::env::args_os()` does not need Rust's own `main`.** On glibc,
  std registers an `.init_array` function, which receives `argc` and `argv`; on
  macOS it calls `_NSGetArgc` and `_NSGetArgv`; on Windows it parses
  `GetCommandLineW`. So a built executable's runtime can read its arguments
  itself. This is reasoned from std's implementation, not measured; §7.4's
  built-executable test on all three CI operating systems is its measurement.
- **Builtins mostly return primitives, and Nova composes them,** through the
  `STD_ONLY` pattern and its twelve registration sites (ADR 0018). One
  exception: `fs_take_string_array` returns a runtime-built `[String]`.
- **Every std module is glob-imported into every user module, and a user's own
  item shadows a std one** with no `E0002` clash (`import_std_module`,
  `crates/nova-resolver/src/lib.rs:1644-1650`). No `.nova` file in the tree
  defines `args` or `exit` (checked 2026-10-05).
- **The runtime writes stdout through Rust's `std::io::stdout()`**
  (`crates/nova-runtime/src/io.rs:139,163`). `std::process::exit` runs std's
  cleanup, which flushes it, before the process ends. This is reasoned from
  std's source; §7.3's exit fixture measures it.
- **`nova test` runs its built test binary as a subprocess with no arguments,**
  choosing the test through `NOVA_TEST_INDEX`
  (`crates/nova-cli/src/cmd/test.rs:179,275`).

### 4.2 Runtime: `crates/nova-runtime/src/process.rs`

- **One process-wide list,** a `OnceLock<Vec<String>>`.
- **`pub fn set_args(args: Vec<String>) -> bool`,** a Rust function the driver
  calls before `main` under `nova run`. The list is set once: a later call
  changes nothing and returns `false`.
- **If nothing set it,** the first read fills it from `std::env::args_os()`,
  converting each argument with `to_string_lossy`. That is the path a built
  executable takes, and `nova test`'s binary. An argument that is not valid
  Unicode gets U+FFFD in place of its bad bytes; nothing panics.
- **Three builtins:**
  - `nova_rt_process_arg_count() -> i64`;
  - `nova_rt_process_arg(i: i64) -> String`, the empty string for an index out
    of range (std never asks for one);
  - `nova_rt_process_exit(code: i64)`, which calls
    `std::process::exit(code as i32)` and never returns.

### 4.3 `std/process`

A new `std/process/lib.nova`, and a new `STD_MODULES` entry, `$std.process`:
15 entries become 16, and the `lib.nova` files 16 become 17 with
`STD_TEST_MODULE`.

```nova
// The program's arguments. Index 0 is the program itself: the source file
// under `nova run`, the executable when built.
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

// Ends the process now with `code`, after flushing stdout. Tasks still
// running are not joined.
pub fn exit(code: Int) {
    process_exit(code)
}
```

- **Three `STD_ONLY` builtins:** `process_arg_count() -> Int`,
  `process_arg(i: Int) -> String` and `process_exit(code: Int) -> ()`.
  `STD_ONLY` grows from 78 to 81.
- **`exit` is typed `(Int) -> ()`.** Nova has no never type, so code after
  `exit(..)` type-checks and never runs. Under `nova run`, `exit` ends the
  `nova` process itself, so the shell sees the program's code.
- **The OS truncates the code:** Unix keeps its low 8 bits (`exit(256)` is 0,
  `exit(-1)` is 255); Windows keeps 32.

### 4.4 CLI and driver

- **`nova run [FILE] [-- ARGS...]`.** `RunCmd` gains
  `#[arg(last = true)] args: Vec<OsString>`. Everything after `--` goes to the
  program. `OsString`, not `String`: clap rejects a `String` argument that is
  not valid UTF-8, and §4.2 converts such an argument lossily instead.
- **The list is `[FILE, ARGS...]`,** with FILE as written, or `src/main.nova`
  when defaulted. `nova run -- 20` gives `["src/main.nova", "20"]`. Without
  `--`, `args()` has exactly one element.
- **The driver sets the list** through `set_args` before
  `CompiledProgram::run`.
- `nova build`, `nova check` and `nova test` do not change.

### 4.5 Alternatives considered

- **B: the generated C `main` forwards `argc` and `argv`** to a runtime init
  call. Rejected: it changes both code generators, and the LLVM one cannot be
  built on the development host. Windows' `argv` would also arrive in the ANSI
  code page unless the shim moved to `wmain`.
- **C: `nova run` passes the arguments in an environment variable.** Rejected:
  quoting, and the variable leaks into every child process.
- **`fn main(args: Vec<String>)`.** Rejected: it changes the language's `main`
  contract, and the listings call `args()`.
- **A builtin returning the whole list.** Rejected, though there is a
  precedent: `fs_take_string_array` returns a runtime-built `[String]`. A count
  and an index keep the runtime side to an `Int` and one `String` per call, and
  `args()` builds its `Vec` in Nova either way.

## 5. The examples

### 5.1 `examples/02-fibonacci/src/main.nova`

`fibonacci` keeps its body. `main` reads the first argument:

```nova
// The first argument as an Int, or 10 when it is absent or not a number.
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

### 5.2 `examples/04-todo-cli/src/main.nova`

- **Data:** `record Todo { id: Int, title: String, done: Bool }`, stored as a
  JSON array in `todos.json` in the working directory, written compact.
- **Codec:** an `impl FromJson for Todo`, a decoder for the array, and an
  encoder for the whole list, in the shapes of 05's `User` codec and
  `user_json`. Titles are escaped through `stringify(String(..))`.
- **Load:** no file is an empty list. A file that exists but cannot be read,
  parsed or decoded is §6's error.
- **Commands,** dispatched by a `match` on `args()`'s index 1 (`""` when
  absent):
  - `add <title words>` appends `Todo { id: max + 1, title, done: false }`
    (id 1 for an empty list) and prints `added: <id>`. The title is every
    argument after `add`, joined with single spaces; `untitled` if there are
    none.
  - `list` prints `[ ] <id>: <title>` or `[x] <id>: <title>` for each todo, in
    stored order. With no file it prints nothing.
  - `done <id>` marks that todo done and saves. An unknown or non-numeric id
    changes nothing and prints nothing, as in the listing.
  - Anything else, including no command, prints
    `usage: todo {add <title> | list | done <id>}` on stderr and exits 1.
- **README:** `examples/04-todo-cli/README.md`, following `60-EXAMPLES.md` §9's
  template, with the cycle under `nova run` and as a built executable.

## 6. Error handling

- **A missing `todos.json`** is an empty list, not an error.
- **A `todos.json` that cannot be read, is not UTF-8, is not JSON, or is not an
  array of todos:** an error naming the file and the reason on stderr, exit 1,
  and the file is not written. Every command checks this, `list` included.
- **A failed write:** an error on stderr, exit 1. The listing's `.unwrap()`
  would panic instead.
- **Exit codes:** 0 on success, 1 for usage and every error above.

## 7. Testing

### 7.1 Runtime

The list is set at most once per process, so each scenario is its own
integration-test binary, as `signal_*.rs` are:
- `crates/nova-runtime/tests/process_args_set.rs`: after `set_args`, the two
  builtins return exactly that list, and a second `set_args` returns `false`
  and changes nothing. An index out of range gives the empty string.
- `crates/nova-runtime/tests/process_args_os.rs`: with nothing set, the count
  is at least 1, and index 0 names this test binary.

### 7.2 Builtins

In `crates/nova-typeck/src/check.rs`'s tests: the three builtins are
`STD_ONLY`, typed `() -> Int`, `(Int) -> String` and `(Int) -> ()`. The
existing guards over every builtin cover the other sites.

### 7.3 `std/process`, through `nova run`

- `tests/runtime/process_args.nova`, run as
  `nova run FILE -- one "two words" ""`, prints the count, then each argument in
  brackets. Its `.stdout` fixture proves index 0 is FILE, and that spaces and
  the empty argument survive.
- `tests/runtime/process_exit.nova` writes `partial` with `std/io`'s
  `stdout().write`, which only stages it in the line buffer, and no newline,
  then calls `exit(3)`. The process must exit with status 3, and stdout must be
  exactly `partial`: only `exit`'s flush can deliver it. (`print` flushes on
  every call, so it would pass even with an `exit` that never flushed.)

### 7.4 End to end (`crates/nova-cli/tests/run_tests.rs`)

Each 04 test runs in a fresh temporary directory, because `todos.json` lives in
the working directory.
- **02:** `nova run -- 20` in `examples/02-fibonacci` prints `fib(20) = 6765`.
  The existing test, with no argument, now expects `fib(10) = 55`.
- **04 under `nova run <main.nova> -- ...`:** `add buy milk`, `add walk dog`,
  `list`, `done 1`, `list`. Exact stdout for each, and the final `todos.json`
  bytes.
- **04 as a built executable:** one `nova build`, then the same cycle with real
  argv. This is the measurement of the OS-argv route on each CI operating
  system. Locally it links the debug runtime library, which must be rebuilt
  first (the known stale-library trap).
- **04 edge cases:**
  - a Thai title, `ซื้อนม`, round-trips through the arguments, `todos.json` and
    `list`, under `nova run` and as a built executable;
  - no command: the usage line on stderr, exit 1;
  - a `todos.json` that does not parse: an error, exit 1, the file
    byte-identical afterwards;
  - `done 99` when there is no todo 99: no change.

### 7.5 Review Focus

The failure modes most likely to bite, each pinned above:
1. **Non-ASCII arguments,** under `nova run` and in a built executable.
2. **An argument with spaces, and an empty argument.**
3. **A corrupt `todos.json` is never overwritten.**
4. **`exit()` flushes a partial line and returns the exact code.**
5. **`nova run` without `--` behaves exactly as before:** `args()` is `[FILE]`.

## 8. Records to amend

Dated notes, then the set-difference sweep 03 used:
- `nova-spec/60-EXAMPLES.md`: §2 (02's gate is met), §4 (the example exists;
  its substitutions are §3 here), and §9's count of examples with a README.
- `nova-spec/20-STDLIB.md`: a new numbered section for `std/process`, giving
  `args` and `exit` and saying `spawn` and `env` are still absent; the
  module-index notes and the `lib.nova` count chain.
- `nova-spec/13-RUNTIME.md`: where the runtime gets a program's arguments, and
  what `exit` does.
- **A new ADR 0023, "Program arguments":** approach A, why B and C were
  rejected, and the built-executable test that measures A's one assumption.
- The inventory's §4.3 and §6, `00-MASTER-SPEC.md` §9's example notes, the four
  gate-remeasure-7 notes that say `04-todo-cli` does not exist, and
  `CHANGELOG.md`'s `[Unreleased]`.
- The sweep then finds anything else now false: "`args()` does not exist", "no
  `exit`", "`nova run` cannot pass arguments through", "04 does not exist".
- **Facts only about Phase 2:** every example `60-EXAMPLES.md` labels a Phase 2
  gate (03, 04, 05) exists and passes, and `00-MASTER-SPEC.md` §3's own gate was
  already met. No record declares Phase 2 complete.

## 9. Risks

- **Rust std's argv capture without a Rust `main`.** Reasoned for glibc, macOS
  and Windows, measured only by §7.4's built-executable test on CI. A libc that
  does not pass `argc` and `argv` to `.init_array` functions (musl, for one)
  would give an empty list. No CI target uses one.
- **`exit` under `nova run` ends `nova.exe` directly,** skipping the driver's
  return path. Nothing the driver does after `main` is lost today.
- **The debug runtime library** that `nova build` links in local test runs can
  be stale. The plan rebuilds it before those tests.

## 10. What is not covered

- **E-11:** a built executable drops a trailing partial line when `main`
  returns, because the C `main` returns without std's flush. A separate task
  exists. `exit()` flushes, and both examples print only whole lines, so
  neither gate depends on it.
- **`std/process`'s `spawn` and `env`.**
- **The five conveniences** §1 lists.
- **The LLVM backend:** approach A does not touch it, and it cannot be built on
  the development host.
- **A libc other than glibc on Unix.**
- **Whether Phase 2 is complete.**

## 11. Success criteria

1. `nova run -- 20` in `examples/02-fibonacci` prints `fib(20) = 6765`.
2. 04's add → list → done → list cycle passes under `nova run` and as a built
   executable, on all three CI operating systems.
3. Every test in §7 passes, including the Review Focus cases.
4. The full suite passes, clippy and fmt are clean, and `Cargo.lock` does not
   change.
5. The §8 records are amended, and the sweep finds nothing else now false.
