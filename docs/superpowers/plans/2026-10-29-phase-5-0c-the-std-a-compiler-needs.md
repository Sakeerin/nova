# Phase 5.0c, "The std a compiler needs", Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use
superpowers:subagent-driven-development (recommended) or
superpowers:executing-plans to implement this plan task-by-task. Steps use
checkbox (`- [ ]`) syntax for tracking.

**Goal:** The last of Phase 5.0's three parts: the std a compiler written in
Nova reads and writes with. Methods on arrays, `iter` and `slice`, and
`Vec`'s `at`, `last`, `to_array` and `from_array`; `Char.to_int`,
`char_from_int`, `string_from_chars`, and a `String`'s bytes in constant
time, `byte_len`, `byte_at` and `byte_slice`; `StringBuilder`, linear in the
length it builds; `Float.to_bits`; `parse_int`, `parse_int_radix` and
`parse_float`, Rust's grammar on both targets; `OrderedMap`, whose order is
the same on every run; and `std/process`'s `run`, `status`, `env` and
`current_exe`, blocking and native only. No syntax changes; what a program
may write changes in two ways: an inherent impl on arrays,
`impl<T> [T] { … }`, is accepted where every impl on an array was E0010 (a
trait impl on one stays E0010), and std's new inherent methods take a call
before a program's trait method of the same name, which a new warning,
E0105, marks; a program's own inherent method of such a name is E0074. E0074
between two inherent impls labels both, a std-only builtin named by a
program gets its public form as a note, and an array method's name shows as
`[T]::slice`. A test program that runs itself is guarded against a child
that runs it again, and every process test runs under a process limit. ADR
0050 records the departures.

**Architecture:**
- **std:** `std/collections/lib.nova` (`impl<T> [T]`, `ArrayIter`, `Vec`'s
  helpers, `OrderedMap`, `OrderedMapIter`), `std/strings/lib.nova` (byte
  access, parsing, `string_from_chars`, `StringBuilder`),
  `std/core/lib.nova` (`Char.to_int`, `char_from_int`, `Float.to_bits`),
  `std/process/lib.nova` (`ProcessOutput`, `run`, `status`, `env`,
  `current_exe`).
- **`crates/nova-hir/src/lib.rs`:** `TyHead::Array`.
  **`crates/nova-typeck/src/check.rs`:** an array's methods after `len`, its
  members, E0074's two labels, E0010 for a trait impl on an array, E0013 for
  an interpolated one, E0105, `shown_method` and `shown_method_name`, the
  ten builtins' signatures; **`check/fixes.rs`:** `len` in E0014's fix on an
  array, `STD_ONLY_PUBLIC_FORMS`. **`crates/nova-doc/src/model.rs`:**
  `Array`.
- **The ten std-only builtins:** `str_as_bytes` and `int_to_char`, register
  moves; `float_to_bits` and `str_is_float`, runtime calls and shim entries;
  `process_run`, `process_take_status`, `process_take_stdout`,
  `process_take_stderr`, `process_env` and `process_current_exe`, native
  runtime calls. Each in `crates/nova-resolver/src/lib.rs`, the checker,
  and, but for the moves, `crates/nova-mir/src/{lib,lower}.rs`'s `RtFunc`
  and `crates/nova-runtime/src/{lib,process}.rs`;
  `crates/nova-codegen-wasm/src/shim.rs` and `shim/float.js` for the two
  shim entries and the `-nan` sign.
- **Tests:** `check/std_tests.rs` (new), the checker's signature and members
  tests, `lower_tests.rs`, `ir_tests.rs`, the runtime's and the shim's, the
  language server's and `nova-doc`'s, and ten runtime programs:
  `array_helpers`, `hidden_trait_methods`, `char_and_bytes`,
  `string_builder`, `string_builder_million`, `parse_numbers`, `float_bits`,
  `ordered_map`, `process_run` and `process_env`, under `run`, `build`, GC
  stress, WASM (the eight that can) and the golden suite, with each std
  panic natively and on WASM.
- **Records:** ADR 0050; `CHANGELOG.md`; dated notes in four files of
  `nova-spec/`, and E0105 in `12-TYPESYSTEM.md`'s catalog.

**Tech Stack:**
- Rust: MSRV 1.78, edition 2021.
- **No new crate; `Cargo.lock` does not change** (spec §7.1).
- `clang`: Ubuntu's `clang-18`, 18.1.3, as 5.0a pinned it, for the golden
  suite; Node 22 for the WASM gate and the shim's tests.

**Spec:**
`docs/superpowers/specs/2026-10-26-phase-5-0-the-language-and-std-a-compiler-needs-design.md`,
approved by the user on 2026-10-26.
- Read its §5, §6, §7, §10 to §13 before Task 1: it is the authority this
  plan argues from.
- Its header holds the user's decisions; decisions 1, 6 and 8 are 5.0c's
  (decisions 1 to 3 here). Its §12 holds the decisions made while writing
  it, and its §13 what this plan settles (decisions 11 to 16).

**Checked on 2026-10-30, before execution:**
- **A prototype built 5.0c whole** on a local branch from `e13630f0` (`main`
  after 5.0b), in this tree, in two parts: part A, `2acaca56` and `778d9827`
  (the builder, the conversions and byte access, parsing, `to_bits`, arrays
  and `Vec`), and part B, `1fcd5682` (`OrderedMap`), `ab5492bd`
  (`std/process`), `4ff115c7` (ADR 0050 and the records), `46175945` (the
  guard against a runaway child) and `c8d7fe8b` (the ADR's note on it); then
  `94f37ae8`, step 0 (below), `a35fb150`, the mutants' gaps (below),
  `6b3a9037`, the first fact-check's fixes (below), and `8f1d4153`, the
  second fact-check's fixes (below). It settled spec §13's questions
  (decisions 11 to 16), and broke each check and saw its tests fail. Its
  full suite gave `2743 1 10` after part A and `2765 1 10` after part B, and
  its golden suite `146 passed, 9 excluded`.
- **The runaway child.** Part B's mutant that dropped a child's arguments
  made `process_run` run itself without end and restarted the machine three
  times; `46175945` added the guard of decision 22. Every process test and
  mutant since ran under the process limit of the conventions: as root,
  `ulimit -u` alone binds nothing, which a probe showed (80 `sleep`s started
  under `ulimit -u 40`), and a pids control group capped at 600 tasks does;
  the whole replay ran under both.
- **Step 0, before the split,** `94f37ae8 5.0c prototype: before the split`:
  - `env`'s refusal of `=` pinned on Linux (decision 25);
  - every new API's edges probed (decision 35), and two diagnostics fixed:
    E0074 labelling the program's impl (decision 18), and the public form of
    a std-only builtin in place of a wrong guess (decision 19), each with a
    test that fails without it; and three tests added;
  - the records: ADR 0050 records each departure and settled point, the
    restart hazard and its guard among them; the CHANGELOG states the E0074
    case, the labels, the note and `STD_ONLY` 90 to 100 plainly;
  - the mixed tests cut by task, two programs' printing rewritten with `+`
    (decisions 30 and 31), and the patches cleaned of trailing whitespace
    and escapes (decision 36).
- **The mutants' gaps,** `a35fb150 5.0c prototype: close the mutants' gaps`
  (decision 37): three tests, one for a gap the first run found and two for
  gaps seen while writing the mutants.
- **The first fact-check's fixes,**
  `6b3a9037 5.0c prototype: the first fact-check's fixes`, with the user's
  two rulings (decisions 17 and 40 to 44):
  - std's inherent methods take a call before a program's trait methods of
    their names, and the new warning E0105 marks each hidden one, with how
    to reach it; a program's own inherent method of such a name is E0074;
  - a trait impl on an array stays E0010, an inherent one being 5.0c's one
    newly accepted form; decision 17 was wrong to say the base accepted any
    impl on arrays;
  - an array method shows as `[T]::slice` in E0016, not `Arr_T.slice`;
  - `ordered_map`'s first-key update takes a value it did not hold; the
    runtime's `current_exe_is_a_file` is dropped, the run test covering
    5.0c's `current_exe`;
  - the records say what `to_bits` keeps of a NaN, which inherent impls on
    arrays are new, and E0105; the process limit's conventions cover cgroup
    v2 and other users;
  - each new test fails with its fix removed; the split, the replay, the
    mutants and this plan were redone on the new head.
- **The second fact-check's fixes,**
  `8f1d4153 5.0c prototype: the second fact-check's fixes` (decisions 18,
  34, 40 to 42 and 45). It found E0105's advice working on all three
  backends, and:
  - E0105 paired names alone, so a trait method beside std's selfless
    `Vec::from_array` or `Vec::new` warned falsely; it now warns only where
    both take `self` or neither does, a pair of static functions at the path
    `Vec::from_array(…)`;
  - E0105 is Task 1's now, with its `Vec` rows, so no task commit hides a
    trait method silently; Tasks 2, 4 and 5 add their rows;
  - `shown_method_name` sat inside `builtin_signature`'s doc comment, and
    moved above it; the records' lists of std's new inherent methods lacked
    `Vec::from_array`, fourteen in all;
  - decision 42 named its four sites wrongly, and is marked (differs), with
    tests for E0060's and `Vec::at`'s names;
  - the process limit's setup now joins the group and checks membership in
    every shell, and says `pids.peak` counts from the group's creation and
    that 62 was on 4 CPUs;
  - the nits: `Vec<T>` not `Vec<T0>`, `x.parse_int_radix(…)`, `Vec::at`,
    E0074's program-first label, E0013 for arrays, older and private std
    methods recorded, and `hidden_trait_methods`, a runtime test of E0105's
    advice on every backend;
  - each new test failed with its fix removed; the split, the replay, the
    mutants and this plan were redone on the new head.
- **A replay split the prototype into this plan's tasks,** on the local
  branch `split/5-0c` from `e13630f0`, by a script, and ran each task's
  steps as written:
  - fifteen commits: a tests commit and a code commit for each of Tasks 1 to
    7, then Task 8, each the patch of its task below. The last commit's tree
    is the prototype head's (`git diff --stat 8f1d4153 split/5-0c` prints
    nothing). Each patch applies with plain `git apply`, in order, on
    `e13630f0`, with no warning;
  - the split follows the items' dependencies rather than the spec's order
    (decision 29), and cuts tables by rows (decision 31);
  - each tests commit failed as its step says before its code, some by not
    compiling (decision 32), and passed after it; the whole suite, but the
    IPv6 test, `cargo fmt`, clippy with `-D warnings` and `nova fmt --check`
    passed after every task;
  - every "Expected:" below is the replay's, from a fresh `NOVA_HOME`, but
    Task 0's suite figures, which are 5.0b's last full run on `e13630f0`.
- **The final checks ran on the split's last commit:**
  - the full suite gave `2788 1 10` (6 min 12 s): the base's `2714 1 10` and
    74 tests more, which a comparison by binary finds as 74 added and 0 gone
    (Task 9, Step 1); the one failure is the IPv6 test;
  - CI's Linux leg, the whole suite under
    `NOVA_REQUIRE_CLANG=1 NOVA_CLANG=clang-18 NOVA_REQUIRE_NODE=1 … --all-features`,
    run once as CI runs it: `2788 1 10`, the golden suite among them (Task
    9, Step 2);
  - the golden suite, in 36 s of wall time:
    `release golden: 147 passed, 9 excluded, Ubuntu clang version 18.1.3 (1ubuntu1)`;
  - the AST of every `.nova` file `e13630f0` tracks:
    `ast: 285 files, 0 outside std differ, 4 std files differ; base exits nonzero on 2, the tree on 2`;
  - `nova check` over the same files:
    `check: 251 checks, 0 outside std differ, 8 std files differ` (Task 9,
    Step 5);
  - E0102 to E0105:
    `fires: 261 checks, E0102 53, E0103 0, E0104 0, E0105 3`, the three
    E0105s `hidden_trait_methods`' own;
  - every runtime program at each task:
    `progcmp: 156 programs; base: 146 same, 10 refused, 0 differ; t1: 148 same, 8 refused, 0 differ; t2: 149 same, 7 refused, 0 differ; t3: 151 same, 5 refused, 0 differ; t4: 151 same, 5 refused, 0 differ; t5: 153 same, 3 refused, 0 differ; t6: 154 same, 2 refused, 0 differ; t7: 156 same, 0 refused, 0 differ; t8: 156 same, 0 refused, 0 differ;`;
  - Rust 1.78's check was clean, and `Cargo.lock` did not change.
- **The mutants.** 75 breakages of what 5.0c adds, each a text replaced in
  one file, run one at a time at the split's last commit, under the process
  limit (Task 9, Step 9): arrays and `Vec`, their lookup, fix, members,
  E0074's labels, the docs' name, the trait impl's E0010, the names
  `[T]::slice` and `Vec::at`, E0013 on an array and E0105, one of them run
  at Task 1's own commit (29); the conversions, byte access, the view and
  the public-form note (8); the builder (4); `to_bits` natively, in the shim
  and in its wiring (5); parsing natively, in the shim, the `-nan` sign and
  the wiring (12); `OrderedMap` (5); and `std/process`, the guard and the
  runaway child among them, `env`'s refusals, the signal, the flush, the
  takes, E0092's export and the walk's variables (12). The first run, at the
  prototype's head, caught 54, left one passing, `OrderedMap` treating an
  update of the key in slot 0 as a new key, and broke the build on four
  whose edits were written wrong, each rewritten; the gap and two seen while
  writing the mutants were closed by `a35fb150` (decision 37). This run, on
  the split of `8f1d4153`, caught 75 of 75, in 34 min 56 s; none passed and
  none was left unrun, the sixteen added since the first run, for E0105, the
  trait impl on arrays, the shown names, E0074's primary label and E0013,
  among them; mutant 65, the arguments dropped, is caught by the guard, each
  child stopping at the marker.
- **What the checks left standing,** each recorded or handed on:
  - a program's `impl<T> [T]` that repeats `slice` with other arguments
    draws errors inside std after its E0074, as `impl String` does at the
    base (decision 17);
  - E0105 names a way to reach the hidden method, not a call syntax, since
    Nova has no `Trait::m(x)` (decision 40); the two E0014 sites that name
    an inherent method cannot be shown an array (decision 42);
  - `env`'s refusals of an empty name and of NUL change no result on any
    system CI runs, and no test can see them go (decision 25);
  - the process tests' Windows branches are reasoned, not run; a `.bat`
    program's arguments are refused by Rust where `cmd.exe` would misread
    them (decision 27);
  - `byte_slice` copies twice; an `OrderedMap` that only shrinks keeps its
    slots; a key inserted while an iterator runs can make it skip or repeat
    entries, as its docs say (decisions 11 and 16);
  - `-nan` interpolates as `NaN`, its sign seen only through `to_bits`;
  - Rust 1.78 checks the libraries and binaries, not the tests: `assert_cmd`
    2.2.1's manifest does not parse on 1.78 (Task 9, Step 7).

## Global Constraints

- **The workspace's minimum Rust stays 1.78,** edition 2021. CI's MSRV job
  runs `cargo check --locked --workspace` with `RUSTFLAGS=-D warnings`. No
  `std::sync::LazyLock` or `LazyCell` (1.80), no `Option::is_none_or`
  (1.82), no `std::path::absolute` (1.79). 5.0c's Rust uses `Cell::take`
  (1.17), `const` initializers in `thread_local!` (1.59) and
  `i8::from(bool)` (1.28), nothing newer.
- **No new crate, and `Cargo.lock` does not change** (spec §7.1). Any change
  to the lock stops the task: ask the user.
- **No `unwrap()` outside `#[cfg(test)]` and test files:** `expect("why")`,
  `unwrap_or…` or `?`. 5.0c's `unwrap()`s are only in test files and test
  modules: `run_tests.rs`, `wasm_tests.rs`, `lower_tests.rs`,
  `check/std_tests.rs`, and the `#[cfg(test)]` modules of
  `crates/nova-lsp/src/completion.rs` and `crates/nova-doc/src/model.rs`.
- **Tests never touch the internet.** `clang` is installed by CI's job,
  never by a test or by `nova` (spec §7.1). No test uses a port but those
  the existing tests bind. The process tests run only `sh`, `cmd` on
  Windows, a missing program, `nova`, and the programs they build.
- **Temporary names are fixed** (spec §7.1): under `std::env::temp_dir()`,
  `nova-std-panics`, `nova-process-run`, `nova-process-signal`,
  `nova-process-env`, `nova-process-current-exe`,
  `nova-runtime-process-denied` and `nova-codegen-wasm-float-bits`; the
  language server's `nova-lsp-complete-std-50c` and
  `nova-lsp-complete-process-50c`, as overlays that are never written; and
  the fixed names `wasm_tests.rs`' and `nova-doc`'s `fresh` helpers make,
  `std-panic-<n>`, `missing-process-<name>` and `array-impl`. The golden
  suite builds into `nova-release-golden/`, as in 5.0a, and clears each
  program's temporary directory before it runs. A test that needs
  `$NOVA_HOME` sets a fresh one.
- **Never clean `/tmp/nova-*`:** `nova test` keeps built binaries in
  `/tmp/nova-test-bin`, a test whose binary goes fails, and the golden
  suite's directory is one of them.
- **A test that needs Node** passes without running when `node` is not on
  `PATH`, and fails instead under `NOVA_REQUIRE_NODE=1`, which CI sets. Run
  the WASM tests here with `NOVA_REQUIRE_NODE=1`: Node 22 is installed.
  Eight of 5.0c's ten programs join the WASM gate; `process_run` and
  `process_env` are on its left-out list, as `std/process` is native only.
- **`release_golden_suite` runs only under `NOVA_REQUIRE_CLANG=1`,** which
  CI's Linux leg sets with `NOVA_CLANG=clang-18`; elsewhere it prints
  `release golden: not run (set NOVA_REQUIRE_CLANG=1 to run it)` and passes.
  Run it here with both set: this machine's `clang-18` is Ubuntu's 18.1.3,
  CI's pin. It walks every `tests/runtime/*.nova` with a `.stdout`, so each
  of 5.0c's programs runs through LLVM too, `process_env` with the variables
  its `RELEASE_GOLDEN_ENV` entry sets.
- **`nova-cli`'s tests need two files beside `target/debug/nova`:**
  `libnova_runtime.a` and the `nova-bench-http` binary. A build of the
  workspace writes both, and nothing else does: `cargo test` builds the
  `nova` it runs but not the runtime library beside it, so a `run_tests` run
  after a change to `nova-runtime` without the build links against the old
  library ("undefined reference to `nova_rt_process_run`"). Every run step
  starts with `cargo build --locked --workspace` (Conventions).
- **The runaway child.** `tests/runtime/process_run.nova` runs itself as its
  own child. A child that lost its arguments would run the parent's part
  again, and start a child of its own, without end: the prototype's mutant
  that dropped the arguments did that, and filled the machine three times.
  The program's guard, a marker in its temporary directory that a second
  parent finds and stops at, comes with the program in Task 7's tests patch
  and stays in every task after; `process_run_runs` gives the program a
  fresh temporary directory and checks the guard. **Every process test and
  every process mutant runs under a process limit** (Conventions), so a
  child that runs its parent again fails to fork rather than filling the
  machine. After any restart, restore the tree first: `git status --short`
  must print nothing before anything else runs.
- **Work in one tree.** Two worktrees of this repository sharing one
  `target/` share artifact names, so cargo can take one tree's build as
  fresh for the other (4.0's ledger). Never build in another worktree.
- **Keep `df -h /` above 6G.** `target/debug/incremental` and older
  duplicate test executables in `target/debug/deps` may be removed, between
  commands, never while one runs: cargo may be about to run the older one,
  and then fails with "No such file or directory". Every `cargo` command
  here selects the whole workspace (Conventions), so cargo keeps one copy of
  each test executable. Task 9's Rust 1.78 check uses its own target
  directory, in `/dev/shm`, removed after.
- **Never use the Write or Edit tool for text that holds a backslash-u
  escape,** a backslash and `u` followed by four hex digits or by a code in
  braces: they decode the escape into the character. None of 5.0c's patches
  holds one (`PROCESS_TEST_VALUE` writes its `é` and `✓` as themselves);
  apply them with `patch_block`, from the committed plan, and write anything
  that holds one with a script.
- **Nothing writes into the repository but the patches.** The golden suite
  and the process tests build under the system's temporary directory; Task
  9's AST and `check` comparisons write their dumps under `$P`.
- **Merge by rebase, on the user's word only.** Never enable auto-merge.
  Nothing is published and no tag is made in 5.0c.
- **This machine is Linux.** macOS and Windows results come from CI, where
  `release_golden_suite` skips; the process tests' Windows branches are read
  from the code, not run here.

## Conventions

- **Scratch directory:** `$P` is the session's scratch directory, outside
  the repository. Set it in every shell before a command that names `$P`.
- **The ledger:** `$P/ledger-50c.md`. Every ruling made while executing goes
  there, with its cost if wrong; so do each task's figures.
- **Commits.** Each task's message is written to `$P/msg-50c-<task>.txt`:
  the subject the task names, a blank line, and the session's attribution
  lines. Then `git add -A`; `git status --short` lists only the task's
  files; and `git commit -F "$P/msg-50c-<task>.txt"`. A task with a tests
  patch and a code patch makes one commit, after both.
- **The process limit.** Every shell that runs a test or a mutant first caps
  the tasks it may start at 600, threads as well as processes, in one of
  these ways, and checks that the cap holds (Task 0, Step 2).
  - **As root, a pids control group.** Linux never applies `ulimit -u` to
    root (the kernel exempts the root user in the first user namespace), so
    the shell joins a group with that cap. On cgroup v1, as the replay ran:

    ```sh
    CG=/sys/fs/cgroup/pids/nova-limit
    mkdir -p "$CG" && echo 600 > "$CG/pids.max" && echo $$ > "$CG/cgroup.procs"
    grep nova-limit /proc/self/cgroup     # N:pids:/nova-limit
    ```

    On cgroup v2 (`stat -fc %T /sys/fs/cgroup` prints `cgroup2fs`), the
    root's `cgroup.subtree_control` must hold `pids` first, and the group
    is a child of the root:

    ```sh
    grep -qw pids /sys/fs/cgroup/cgroup.subtree_control || echo +pids > /sys/fs/cgroup/cgroup.subtree_control
    CG=/sys/fs/cgroup/nova-limit
    mkdir -p "$CG" && echo 600 > "$CG/pids.max" && echo $$ > "$CG/cgroup.procs"
    cat /proc/self/cgroup                 # 0::/nova-limit
    ```

  - **As any other user,** a systemd scope with the cap,
    `systemd-run --user --scope -p TasksMax=600 bash`, whose shell then runs
    everything
    (`cat "/sys/fs/cgroup$(sed -n 's/^0:://p' /proc/self/cgroup)/pids.max"`
    prints `600`); or, without systemd, `ulimit -Su 600`, the soft limit
    alone, so the shell can raise it again. RLIMIT_NPROC counts all the
    user's tasks, not only this shell's: where the user already runs many,
    the cap is reached sooner, and a test may fail to start a thread for
    that reason alone.

  600 tasks was ample here: the replay's whole suites, builds and golden
  suite ran under it on 4 CPUs, and peaked at 62 (decision 39); the
  runaway child stops at it with "Resource temporarily unavailable". Test
  threads scale with the cores, so a machine with many more may need a
  higher cap: if a test fails to start a thread, read the group's
  `pids.peak` and raise `pids.max`, and record it. `pids.peak` counts from
  the group's creation, so to read one step's peak, make the group afresh
  first (`rmdir "$CG"` once no process is in it, then the lines above), as
  the replay did for each step (decision 34). Where no way works, record
  it in the ledger, and run no process mutant.
- **Runs.** The replay ran every command in the group and with these
  variables set. Each shell joins the group and sets them too, with seven
  shorthands, and runs every command from the repository's root; a shell
  that is not in the group stops at once:

  ```sh
  # The process limit (above), cgroup v1 as root; on v2, its v2 lines, and
  # `grep -qx '0::/nova-limit' /proc/self/cgroup || exit 1` in place of the
  # check; as another user, the shell systemd-run started, or ulimit -Su.
  CG=/sys/fs/cgroup/pids/nova-limit
  mkdir -p "$CG" && echo 600 > "$CG/pids.max" && echo $$ > "$CG/cgroup.procs"
  grep -q nova-limit /proc/self/cgroup || exit 1
  export CARGO_INCREMENTAL=0 NO_COLOR=1 RUST_BACKTRACE=0 NOVA_HOME="$P/home-50c"
  unset NOVA_RUNTIME_LIB NOVA_REQUIRE_NODE NOVA_REQUIRE_CLANG NOVA_CLANG
  mkdir -p "$NOVA_HOME"
  W() { NOVA_REQUIRE_NODE=1 cargo test --locked --workspace --test wasm_tests -- "$@"; }
  R() { cargo test --locked --workspace --test run_tests -- "$@"; }
  L() { cargo test --locked --workspace --test lower_tests -- "$@"; }
  X() { cargo test --locked --workspace --test index_complete "$@"; }
  T() { NOVA_REQUIRE_NODE=1 cargo test --locked --workspace --no-fail-fast --lib -- "$@"; }
  S() { NOVA_REQUIRE_NODE=1 cargo test --locked --workspace --no-fail-fast "$@"; }
  G() { NOVA_REQUIRE_CLANG=1 NOVA_CLANG=clang-18 cargo test --locked --workspace --test run_tests -- release_golden_suite --nocapture "$@"; }
  ```

  So `W gate::ordered_map` runs that WASM gate test, `R ordered_map_runs`
  that test of `run_tests` (under `run`, `build` and, where its program
  asks, GC stress), `L byte_len` the MIR tests whose names hold `byte_len`,
  `X` the corpus test, `T check::std_tests` the tests of every crate's
  library whose path holds that filter, here the checker's tests of 5.0c's
  std, `S` the whole suite, and `G` the golden suite through LLVM as CI's
  Linux leg runs it. `T` and `S` set `NOVA_REQUIRE_NODE=1`, so the shim's
  tests that run its code under Node run rather than pass unrun.
  `NOVA_HOME` keeps a run by hand off `~/.nova`; every test that needs one
  sets its own.
- **Every `cargo` command selects the whole workspace,** `--workspace` with
  `--test NAME` or `--lib` to narrow it, never `-p`. Cargo unifies features
  over the packages a command selects, so each `-p` selection builds and
  links its own copy of every test executable it needs, about 1.3 GB a
  selection here, which the disk does not have (Global Constraints); a
  `--workspace` command reuses one copy. So `T check::std_tests` runs every
  library's tests binary, each its own `test result` line, most with 0
  passed; such a command takes `--no-fail-fast`, as `T` does, so that a
  binary that fails does not stop the ones after it. `S` gives each code
  step the whole suite, about four and a half minutes; its one failure is
  the IPv6 test (Task 0).
- **The build.** Each "Run" step below starts with

  ```sh
  cargo build --locked --workspace
  ```

  which ends with `Finished`, in about 10 seconds after a change to std or
  the front end, so that the `nova` the steps run by hand, and the runtime
  library beside it, are the tree's (Global Constraints). The replay began
  every step with it.
- **The patches.** Each task's code is in patch blocks below, each marked
  `<!-- patch NAME -->` and fenced with four backticks. They are the
  replay's commits, so apply them; do not retype them. From the repository's
  root:

  ```sh
  PLAN=docs/superpowers/plans/2026-10-29-phase-5-0c-the-std-a-compiler-needs.md   # this file, committed
  block() {
    awk -v name="$1" '
      $0 == "<!-- patch " name " -->" { grab = 1; next }
      grab && /^````diff$/ { inside = 1; next }
      inside && /^````$/ { exit }
      inside { print }
    ' "$PLAN" > "$P/$1.patch"
  }
  patch_block() {
    block "$1" && git apply --check "$P/$1.patch" && git apply "$P/$1.patch"
  }
  ```

  `patch_block t1-tests` applies the block named `t1-tests`, and prints
  nothing: no 5.0c patch adds a line with trailing whitespace, a CR or a
  backslash-u escape, so `git apply` warns for none. The non-ASCII
  characters the patches add are `§`, `…` and `—` in prose, and the test
  programs' own text: `é`, `ñ`, `ö`, `ü`, `€`, `𝄞`, `🚀`, `🎉`, `✓`, `世界`,
  `日本語` and `١٢`. A block that does not apply means the tree differs from
  the plan's: stop, and record why in the ledger.
- **Expected outputs are written once, from a run, and read.** Each
  `.stdout` under `tests/runtime/` that 5.0c adds is in its task's tests
  patch: the prototype wrote them from the program's native run, and the
  replay ran each under `nova run` (but `process_run`, which runs itself and
  so is built), `nova build`, `--release` through `clang-18`, and, for the
  eight in the gate, WASM; four under GC stress.
- **Each task** but Task 8 is test first: its tests' patch, their run and
  the expected failure, then its code's patch, its run and the expected
  pass. Task 8's patch is the records. Each task then runs its checks:
  - `cargo fmt --all -- --check`, which prints nothing;
  - the workspace's clippy,
    `cargo clippy --locked --all-targets --all-features -- -D warnings`, as
    CI runs it, which ends with `Finished` and no warning;
  - `target/debug/nova fmt --check std examples tests/wasm tests/dom tests/browser/router-app tests/browser/ssr-app tests/ssr`,
    with the `nova` the step's build made, which prints nothing: it covers
    std, which every task but Task 8 changes. 5.0c's programs are under
    `tests/runtime`, which is not among them, as no program there is; the
    fmt corpus test formats every one of them.

  It ends with one commit. A difference from an `Expected:` is fixed in
  the smallest way and recorded in the ledger as a ruling, with its cost if
  wrong.

## Review Focus

The inputs most likely to bite a user, most likely first. Each has its check
in the task that owns the code:

1. **`std/process`, and the runaway child**
   (`crates/nova-runtime/src/process.rs`, `std/process/lib.nova`,
   `tests/runtime/process_run.nova`): `run`'s standard input empty and both
   outputs read at once, so neither pipe stalls the child; `status`
   inheriting all three streams after a flush of standard output; arguments
   passed as they are, with no shell, quoting on Windows left to Rust's
   `Command` and read back by Rust's argument parser in a Nova child; the
   exit code, or `128 + signal` on Unix; a missing program `NotFound`, one
   that cannot be run `PermissionDenied`, through `std/fs`'s `fail`; the
   payload taken straight after the run, from the thread's cells; `env`
   refusing an empty name, `=` and NUL before it asks the system;
   `current_exe` `nova` under `nova run`, the program when built,
   `InvalidData` for a path that is not Unicode; and the guard that stops a
   child that lost its arguments from running the parent again (decisions 2,
   8, 14, 21, 22 and 25) → `process.rs`'s six unit tests,
   `process_run_runs`, `process_env_runs`,
   `current_exe_is_nova_under_run_and_the_program_when_built` and
   `a_child_killed_by_a_signal_is_128_plus_the_signal` (Task 7) (mutants 64
   to 75). A child that runs its parent again is a fork bomb: the reviewer
   reads `run_command` and the guard, and runs nothing of Task 7's without
   the process limit.
2. **Parsing numbers** (`parse_digits`, `digit_value`, `str_is_float`, the
   shim's `FLOAT_TEXT` and `floatFrom`): the value gathered negatively, so
   `Int`'s least value parses, each step checked before it multiplies or
   subtracts, since `*` and `-` wrap; `+` and `-`, leading zeros, no space,
   `_` or prefix; radix 2 to 36, a panic otherwise, letters of either case,
   a digit beyond the radix refused; `parse_float` Rust's grammar and value
   on both targets, `inf`, `infinity` and `nan` in any case, a `-nan` with
   its sign bit on WASM too (decisions 6, 13 and 20) → `parse_numbers`'
   table, natively, built, through LLVM and on WASM, the runtime's
   `str_is_float_is_rusts_grammar`, the shim's `float_from_is_rusts_parse`
   (Task 5) (mutants 47 to 58).
3. **Byte access and the conversions** (`byte_at`, `byte_slice`,
   `continues_char`, `char_from_int`, `int_to_char`, `str_as_bytes`): the
   view copies nothing, a register move as `char_to_int` is; `byte_at` and
   `byte_slice` panic, naming their method, on every bad bound, and
   `byte_slice` on a bound inside a character, so its copy is always valid
   UTF-8; `char_from_int` the only maker of a `Char` from an `Int`, `None`
   below 0, above `0x10FFFF` and for the surrogates (decisions 5, 7 and 11)
   → `char_and_bytes`, the panic tables natively and on WASM, the MIR tests
   of the moves (Task 2) (mutants 30 to 37). A `Char` outside the scalar
   values breaks every string made from it downstream: the reviewer reads
   `char_from_int`.
4. **`OrderedMap`** (`insert`, `remove`, `compact`, `OrderedMapIter`): the
   order of first insertion, an update in place, a reinsertion last;
   compaction only in `insert`, before a new key's slot, when the gaps are
   at least half the slots, each key's position moved in the index; a
   removal never moving an entry, so removing while an iterator runs skips
   nothing else; the `Map` read for lookups only, so no seed reaches the
   order (decisions 3 and 16) → `ordered_map` under four backends and GC
   stress, its run-twice test, the `slots:` line that pins the threshold
   (Task 6) (mutants 59 to 63).
5. **Arrays and `Vec`** (`TyHead::Array`, the array arm of
   `check_method_call`, `members`, `method_fix`, `impl<T> [T]`, `ArrayIter`,
   `Vec`'s helpers): an array's built-in `len` first, then its impls'
   methods; E0014 for anything else, its fix offering `len` with them; a
   program's inherent impl on arrays as on `String`, E0074 where it repeats
   `iter` or `slice`, labelled at the program's impl too, and a trait impl
   on an array E0010 as at the base; an array method named `[T]::slice` in a
   diagnostic; `slice`, `at` and `last` panicking, `at` and `last` against
   `len`, not the storage; `to_array` and `from_array` copying (decisions
   10, 15, 17, 18, 23, 24 and 42) → `std_tests.rs`' array tests,
   `array_helpers`, the panic tables (Task 1) (mutants 1 to 16).
6. **`Float.to_bits`** (`nova_rt_float_to_bits`, the shim's `floatBits`):
   Rust's `f64::to_bits` read as a signed `Int`, a NaN's sign included: the
   bits of the value it is given, on every backend, while a NaN made by
   arithmetic may itself differ by backend (decision 44); the shim's
   `BigInt` signed, which WASM's own conversion would wrap silently, so only
   the shim's test sees an unsigned read (decisions 9 and 13) → the
   runtime's and the shim's tests (Task 4), `float_bits` and the IR test of
   its hex (Task 5) (mutants 42 to 46).
7. **`StringBuilder`:** `push` a `Vec` push and a byte count, `byte_len`
   bytes, not characters, `is_empty` by bytes, `clear` resetting both,
   `to_string` one join that leaves the builder as it was, `Display` for
   interpolation (decisions 4 and 12) → `string_builder` and
   `string_builder_million` (Task 3) (mutants 38 to 41).
8. **The diagnostics:** E0092 for each of `std/process`'s new names on WASM;
   E0074 between two inherent impls labelling both; E0001 at a std-only
   builtin's name with the note naming its public form, in place of a guess
   that names another operation; E0105 at each trait method one of std's
   inherent methods hides where the two can meet, both taking `self` or
   neither, and at no other, with how to reach it; a method's name as the
   source writes its type (decisions 18, 19, 2, 40 to 42 and 45) →
   `std_process_is_e0092_on_wasm` (Task 7),
   `e0074_against_stds_array_impl_labels_the_programs_impl` (Task 1),
   `a_std_only_builtin_named_by_a_program_gets_its_public_form` (Tasks 2, 4
   and 5), E0105's three tests and `hidden_trait_methods` (Task 1), its rows
   in Tasks 2, 4 and 5, and E0105 over the repository (Task 9, Step 6);
   mutants 12, 13, 36, 74 and 17 to 29.
9. **The records:** ADR 0050 against the spec's §5 and the user's decisions;
   the CHANGELOG's Added and Changed; the dated notes of `00`, `12`, `14`
   and `20` → Task 8, Step 2.

## Decisions

Made while writing the spec (its header and §12), prototyping, probing,
splitting and replaying, each as built and each read against the code. Where
one departs from the spec, it says so and why, and is marked **(differs)**;
one that fills in what the spec left to the plan (its §13) says so,
unmarked. ADR 0050 records 5.0c's departures and the points it settles (spec
§5.8).

**The user's decisions, as built**

1. **5.0c is the third of three pull requests** (the user's decision 1):
   std, after 5.0a's and 5.0b's merges. No syntax changes; an inherent impl
   on arrays, refused before, is accepted (decision 17), and std's new
   inherent methods hide a program's trait methods of their names (decision
   40).
2. **`std/process`: plain functions `run` and `status`, with `env` and
   `current_exe`; blocking, and native only** (the user's decision 6, spec
   §5.7). `run` and `status` block the thread, executor included, and say so
   in their docs; `std/process` is not among WASM's modules, so naming any
   of `run`, `status`, `env`, `current_exe` or `ProcessOutput` there is
   E0092, "`run` is in `std/process`, which target wasm does not have"
   (`std_process_is_e0092_on_wasm`; mutant 74). The four names are
   glob-imported into every module, as `args` is, and a program's own
   shadows them silently (`a_programs_own_run_status_and_env_shadow_stds`):
   seven of the repository's programs define a `run` and check as before
   (Task 9, Step 5).
3. **A builder over `Vec<String>`; `OrderedMap` a new type beside `Map`;
   `Float.to_bits` with a WASM shim** (the user's decision 8, spec §5.1,
   §5.4, §5.6). `Map` does not change; its docs point to `OrderedMap`. The
   shim's `floatBits` reads the double's bits through a `DataView`, as a
   signed `BigInt`.

**The spec's decisions (its §12), as built**

4. **`StringBuilder` is in std/strings, with `byte_len` and no `len`** (spec
   §12.18): `new`, `push`, `push_char`, `byte_len`, `is_empty`, `clear` and
   `to_string`, and `Display`, so `"${b}"` interpolates it. `to_string`
   joins every part through `str_join` and leaves the builder as it was; a
   million one-byte pushes take 0.5 s built and 0.7 s under `nova run` here
   (`string_builder_million`), with no timing assertion.
5. **`byte_at` and `byte_slice` panic** (spec §12.19), as `a[i]` does, each
   message naming its method: "String::byte_at index out of range",
   "String::byte_slice start is negative", "… end is past the end of the
   string", "… start is after end", "… start is inside a character" and "…
   end is inside a character", natively and on WASM alike (the panic tables
   of `run_tests.rs` and `wasm_tests.rs`).
6. **`parse_int_radix` beside `parse_int`; `parse_float` by Rust's grammar
   on both targets** (spec §12.20). `parse_int` and `parse_int_radix` are
   Nova over the string's bytes; `parse_float` asks `str_is_float`, then
   `str_to_float`, each Rust's parser natively and the shim's `FLOAT_TEXT`
   and `floatFrom` on WASM. `parse_numbers`' table, 28 integers, 34 radix
   cases and 62 floats, each float with its bits, prints the same on both.
7. **`char_from_int` and `string_from_chars` are free functions**, named
   after `bytes_from_string` (spec §12.21); `to_int` is a method of `Char`.
8. **`ProcessOutput`, not `Output`; `run`'s standard input is empty; a
   signal is `128 + signal`** (spec §12.22): a child that panics gives 134
   on Unix (`a_child_killed_by_a_signal_is_128_plus_the_signal`), `kill -9`
   137 (`a_death_by_signal_is_128_plus_the_signal`; mutant 69).
9. **`Float.to_bits` alone, with no `from_bits`** (spec §12.23).
10. **`Vec::to_array` and `Vec::from_array` copy** (spec §12.24), so neither
    shares storage with the other (`array_helpers`; mutant 4).

**What spec §13 left to this plan, settled**

11. **Byte access goes through a view that copies nothing.** The std-only
    builtin `str_as_bytes` reads a `String` as `Bytes`, a register move,
    lowered as `char_to_int` is (`Lowering::Reinterpret`, a `Copy`), since
    the two share one layout on every target and neither changes; then
    `byte_len`, `byte_at` and `byte_slice` are `Bytes`' own builtins. No
    runtime symbol, no std/rt function and no shim entry is new for it.
    `byte_slice` copies twice, `bytes_slice` then
    `bytes_to_string_unchecked`; a `bytes_as_str` move would save one, and
    is not built (`byte_len_lowers_to_a_move_and_the_bytes_length`; mutant
    32). `int_to_char` is the same move the other way, after `char_from_int`
    checks the range in Nova.
12. **No `str_join` that takes a count.** `to_string` copies the parts'
    pointers into an array of their count, `Vec::to_array`, then joins: one
    word a part more, still linear, where a counted join would cost a
    runtime symbol, a std/rt function and a reach into `Vec`'s storage,
    which is private to std/collections.
13. **`float_to_bits` is a runtime call, not a MIR bit cast:** one `RtFunc`,
    `nova_rt_float_to_bits` natively and the shim's `float_to_bits` on WASM,
    so no backend has a new statement (`to_bits_is_one_runtime_call`).
    `parse_float` pairs `str_to_float` with a new `str_is_float`, the same
    way, since `str_to_float`'s `0.0` for a refused text cannot be told from
    a real one.
14. **The process builtins and their wire contract,** `std/fs`'s: six
    std-only builtins, not ten. `process_run(program, args, capture) -> Int`
    serves both `run` and `status`, and returns a status code, `0` or an
    `IoErrorKind`'s; then `process_take_status`, `process_take_stdout` and
    `process_take_stderr`. `process_env(name) -> Bool` and
    `process_current_exe() -> Int` hand their text over through
    `fs_take_string`, and every failure's message waits for
    `fs_last_error_message`, through `fs.rs`'s own `fail`, so the error
    kinds follow `std/fs`'s table. The child's output waits in the thread's
    cells as Rust bytes, not GC objects, so nothing is rooted while it
    waits; the wrappers are plain `fn`s with nothing between the run and its
    takes (`std_process_wrappers_call_their_runtime_builtins`; mutant 73).
    **A test sets a variable on a built program** through the process it
    starts: `Command::env` in `run_tests.rs`, and the `--release` walk's new
    `RELEASE_GOLDEN_ENV` table, beside its `RELEASE_GOLDEN_STDIN` (mutant
    75).
15. **`Vec` gains `at(i) -> T` and `last() -> T`,** which panic where `get`
    and `pop` give `None`, "Vec::at index out of range" and "Vec::last on an
    empty vector", each checked against `len`, not the storage's length,
    which is longer: a cleared vector still holds its values (the panic
    tables' `emptied()`; mutants 1 and 2).
16. **`OrderedMap`'s compaction, and where each item goes.** Only `insert`
    compacts, before it appends a new key's slot, and only when the gaps are
    at least half the slots: so the slots stay within twice the entries plus
    one after any insertion, `Vec`'s own slack, a walk is linear in the
    entries, `insert` and `remove` stay amortized constant time, and a
    removal never moves an entry, so removing while an iterator runs skips
    nothing else (`ordered_map`'s `slots: 4->5 5->6 6->4` line; mutant 59).
    A map that only shrinks keeps its slots, and its dead keys and values,
    until it next grows. The files: `StringBuilder`, `string_from_chars`,
    byte access and parsing in `std/strings`; `to_int`, `char_from_int` and
    `to_bits` in `std/core`; arrays, `Vec`'s helpers and `OrderedMap` in
    `std/collections`; the rest in `std/process`.

**Where the prototype departs from the spec, or settles what it left open**

17. **An inherent impl on arrays is accepted; a trait impl on one stays
    E0010.** **(differs)** At the base every impl on an array was E0010,
    "impl blocks are only supported on named types", arrays having no type
    head. The spec says a program's impl on arrays follows `impl String`'s
    rule; now `impl<T> [T] { … }` and `impl [Int] { … }` check, 5.0c's one
    newly accepted form (`a_program_may_write_an_impl_on_arrays`), and one
    that defines `iter` or `slice` meets std's `impl<T> [T]`, E0074. A trait
    impl on an array, `impl Display for [Int]` or `impl<T> Hash for [T]`,
    stays E0010, now "a trait cannot be implemented for an array", with a
    note: inherent impls only, or wrap the array in a record (the user's
    ruling on the first fact-check; `a_trait_impl_on_an_array_is_e0010`;
    mutant 15). So no program that compiled stops compiling here. As with
    `impl String`, std's own calls of the method then resolve to the
    program's, so a `slice` of other arguments also draws errors inside std,
    after the E0074 (Task 9, Step 5's probe). The CHANGELOG's Added and
    Changed, ADR 0050's Decision 4 and Consequences, and `12-TYPESYSTEM.md`
    §5.2 say so.
18. **E0074 between two inherent impls labels both.** **(differs)** The
    error was reported at the later impl, which for a program's impl against
    std's is std's, so before 5.0c the program's own impl appeared nowhere
    in it. Now both are labelled, and when one is std's the program's is the
    primary label, "here", and std's the second, "`iter` is also defined by
    std's impl" (the second fact-check); between two of a program's impls,
    the later is primary and the earlier "… also defined by this impl". This
    changes every inherent E0074, not only arrays'
    (`e0074_against_stds_array_impl_labels_the_programs_impl`; mutants 12
    and 13).
19. **A std-only builtin named by a program gets its public form.**
    **(differs)** Spec §6 gives 5.0c no diagnostic change. Step 0's probe
    found that `char_to_int(c)` in a program, E0001 as before, drew "help:
    change `char_to_int` to `char_from_int`", the conversion the other way,
    since 5.0c's `char_from_int` is the nearest name in scope. For the seven
    builtins std's surface now wraps, `char_to_int`, `int_to_char`,
    `str_from_chars`, `str_as_bytes`, `str_is_float`, `str_to_float` and
    `float_to_bits`, the E0001 carries a note instead, "`char_to_int` is
    internal to std; use the method `to_int`, as in `c.to_int()`", and no
    guess (`a_std_only_builtin_named_by_a_program_gets_its_public_form`;
    mutant 36).
20. **The shim's `str_to_float` gives `-nan` its sign bit,** as Rust's
    parser does natively. **(differs)** The spec does not name it; before,
    the shim dropped the sign, which no program could see until `to_bits`
    (`float_from_is_rusts_parse`, `parse_numbers`, `float_bits`; mutant 56).
21. **`status` flushes standard output first; `current_exe` of a path that
    is not Unicode is `InvalidData`; `env` refuses an empty name, `=` and
    NUL without asking the system.** A `Write` to standard output leaves a
    partial line in the buffer, which the child's output would otherwise
    overtake (`process_run`'s "before status, unflushed, then" line; mutant
    66). A path with its bad bytes replaced would name another file. The
    spec asks for the refusals; Rust's `env::var` may refuse such a name on
    its own, or not.
22. **The runaway child, and its guard.** `process_run.nova` runs itself,
    found through `current_exe()`, as a child its first argument names. The
    prototype's mutant that dropped the child's arguments made each child
    run the parent's part again, a fork bomb that restarted the machine
    three times. The program writes a marker in its temporary directory
    before it starts a child and stops, with exit status 1 and a message, if
    one is there; a child that has its arguments never looks; its tests give
    it a fresh directory, and `process_run_runs` then runs it with the
    marker in place and expects that stop (mutants 64 and 65, the latter now
    caught by the guard, under the process limit). The golden walk clears
    each program's temporary directory first. Every task from Task 7's tests
    on keeps the guard.
23. **The documentation site names an impl on arrays `Array`:** its methods
    are `Array::iter` and `Array::slice`, under std/collections, not a
    primitive's page (`an_impl_on_arrays_names_its_methods_after_array`;
    mutant 14).
24. **An array's methods are looked up after its built-in `len`,** by
    `TyHead::Array` as any type's are; E0014 for anything else, "no method
    `sclice` on array type `[Int]`", now with "change `sclice` to `slice`"
    from the impls' methods and `len`. **(differs)** As with decision 19,
    spec §6 gives 5.0c no diagnostic change, and E0014's help on an array
    changes: at the base it could offer `len` alone, "change `lne` to
    `len`", and `sclice` drew no help. The language server's members of an
    array are `len`, `iter` and `slice`. A program's `impl [Int]`'s methods
    apply to that element type alone (decision 17; mutants 8 to 11).
25. **`env`'s refusal of `=` is pinned on Linux.** Step 0 found the refusal
    caught by no test on Linux, where glibc's `getenv` finds no such name.
    With `NOVA_PROCESS_TEST_EQ=x=y` set, glibc answers `y` for the name
    `NOVA_PROCESS_TEST_EQ=x`, and macOS's the whole value, so
    `process_env`'s `None` for it is `env`'s own; the program also asks for
    `=C:`, which Windows keeps as a hidden variable. The empty and NUL
    refusals change no result on Linux, where glibc and Rust refuse those
    names themselves, and stay as the spec asks (mutant 70).
26. **`std/process`'s doc tests start no program.** A doc test's child would
    inherit `NOVA_TEST_INDEX` and run the doc test again; `run`'s and
    `status`'s examples run a program that does not exist.
27. **Windows is reasoned from the code, not run here.** `EXE_SUFFIX` on
    every build; a program found on `PATH` with `.exe`; `cmd /C exit 7` for
    the status of a program by bare name; Rust's quoting of spaces, quotes,
    a trailing backslash, an empty argument and non-ASCII, read back by the
    Nova child's `GetCommandLineW` parse, which is Rust's; a build removed
    after its children end, since Windows locks a running executable; CRLF
    normalized. A `.bat` or `.cmd` program runs through `cmd.exe`, whose
    quoting is not the C runtime's, and Rust's `Command` refuses arguments
    it cannot pass to one safely: `run` then gives an `IoError` of kind
    `Other`. ADR 0050 says so.
28. **`STD_ONLY` grows from 90 to 100,** and one diagnostic code is new,
    E0105 (decision 40): `str_as_bytes` and `int_to_char` (Task 2),
    `float_to_bits` (Task 4), `str_is_float` (Task 5) and the six process
    builtins (Task 7), each in the resolver's enum, names and `STD_ONLY`,
    the checker's signatures and their exhaustive test, and, but for the two
    moves, `RtFunc` and the runtime's `symbols()`.

**The split's decisions**

29. **The tasks follow the order each item needs the ones before it:**
    arrays and `Vec`; `Char` and `Int` and a `String`'s bytes; the builder;
    `Float.to_bits`; parsing; `OrderedMap`; `std/process`; the records. The
    natural order puts the builder first, and the builder joins through
    `Vec::to_array` and counts through `byte_len`, so it would carry both,
    and with them half of two other tasks; parsing reads bytes through the
    view; `parse_numbers` shows each float's bits; `OrderedMap` reads its
    `Vec`s through `at`; and `process_run` parses its child's status with
    `parse_int`. So each task's code stands on the tasks before it alone.
30. **The programs ride with their features,** each with its `run_tests.rs`
    test, its `gate!` entry and `index_complete.rs`'s counts in the same
    task (5.0a's lesson): `array_helpers` and `hidden_trait_methods` with
    Task 1 (167 runtime programs, 177 corpus programs and 211 files),
    `char_and_bytes` with Task 2 (168, 212), `string_builder` and
    `string_builder_million` with Task 3 (170, 214), `parse_numbers` and
    `float_bits` with Task 5 (172, 216), `ordered_map` with Task 6 (173,
    217), `process_run` and `process_env` with Task 7 (175, 219).
    **`float_bits` rides with parsing, not `to_bits`:** it makes its
    infinities and NaNs with `parse_float`, since Nova has no literal for
    them and a NaN's sign from arithmetic differs between machines; so Task
    4 has no program, and the IR test that reads `float_bits`' golden rides
    with it too. Step 0 rewrote `array_helpers`' and `char_and_bytes`'
    string building with `+`, where they used the builder only to print, so
    each rides with its own task; their output is unchanged.
31. **Step 0 cut the prototype's mixed tests by task,** so no test names an
    item a later task adds: `lower_tests.rs`' one test of four conversions
    became four, `std_tests.rs`' one test of every item's types became one a
    task, and the language server's one completion test became one for the
    records (Task 6) and one for `std/process` (Task 7). The tables that
    name several tasks' items are cut by rows: the panic tables, the test of
    the builtins kept from user code, the public forms', the checker's
    exhaustive signature test and the four builtin lists.
32. **Tests that cannot stand first, and tests that pass first:** a test
    inside a crate's source, or naming what the code adds, does not compile
    before it, and takes its crate's whole test binary with it:
    `std_tests.rs` (Task 1), the signature test's rows (Tasks 2, 4 and 5,
    and Task 7's process test), the runtime's and the shim's tests (Tasks 4,
    5 and 7), and `lower_tests.rs`' tests that name a new `RtFunc` (Tasks 4,
    5 and 7). Every new test that runs before its code fails, each where its
    step says; none passes first. A test in a binary that does not compile
    before its code, such as Task 2's public-form test or Task 4's shim
    test, runs first at the code step.
33. **Every program at every task runs as at the end, or is refused**
    (5.0b's lesson). A task's `nova` builds every runtime program of the
    last tree whose std items it has, and refuses, before running, every one
    that names an item a later task adds. Task 9, Step 10 runs every runtime
    program of the last tree with the base's `nova` and each code commit's:
    156 programs; none differs anywhere; each task refuses exactly the
    programs of the tasks after it, and runs the rest as the last tree does.
34. **The process limit** (Conventions): 600 tasks, set by a pids control
    group for root, since Linux never applies `ulimit -u` to root, on cgroup
    v1 or v2, and by a systemd scope's `TasksMax` or a soft `ulimit -Su` for
    any other user; every shell joins it in its setup block and stops if
    `/proc/self/cgroup` does not name the group, and Task 0 records which
    way. The replay ran as root on cgroup v1 and 4 CPUs, every step in the
    group `nova-limit`, which its `limit.sh` made afresh before each step
    (`rmdir` once empty, then `mkdir`), so that `pids.peak`, which counts
    from the group's creation, is the step's alone; each step's log names
    the group and its peak. The cap is ten times the peak here; test threads
    scale with the cores, so a larger machine may need more (Conventions).

**Step 0's decisions, before the split**

35. **The probes, and what they found.** Each new API's edges were run
    natively, built and on WASM: `parse_int` and `parse_int_radix` on signs,
    `+`, leading zeros, both ends of `Int`, radix 2 and 36 and digits and
    letters just past the radix; `parse_float` on exponents, `inf`,
    `infinity` and `nan` in each case and sign, `-0.0`, an exponent of
    twenty digits, and a round trip through each value's text and `to_bits`;
    byte access on one- to four-byte characters at every bound; the builder
    empty, with non-ASCII and pushing its own text; `Vec::at` and `last`
    after `pop` and `clear`; `slice` and `iter` on empty arrays and an array
    written while iterated; `OrderedMap` removing ahead of and at the
    iterator, re-inserting after a removal, emptied and reused, and at the
    threshold with two and three slots; and `std/process` on a missing
    program, an empty one, a directory, NUL in the program and in an
    argument, `false`, `exit 300`, arguments with spaces and quotes, `env`
    of an unset name and of `NOVA_PROBE_EQ=x`, and `current_exe` under
    `nova run` and built. All agreed with the docs and the spec; three
    things were fixed, decisions 18, 19 and 25, each with a test that fails
    without its fix, and three tests were added: the edges above in
    `parse_numbers` and `float_bits`, and
    `current_exe_is_nova_under_run_and_the_program_when_built`.
36. **No patch line holds trailing whitespace or a backslash-u escape.**
    `ordered_map` printed three lines ending in a space, which `git apply`
    would warn about: it joins its words with a separator before each one
    now, its output otherwise unchanged. `PROCESS_TEST_VALUE` wrote `é` and
    `✓` as escapes, and writes them as themselves.

**The mutants' gaps**

37. **Gaps the mutants found, each closed by a test that fails with the
    mutant,** in `5.0c prototype: close the mutants' gaps`: `ordered_map`
    updates its first key too, since the first run left `if at >= 0` →
    `if at > 0` passing, no test updating the key in slot 0 (mutant 63); and
    two gaps seen while the mutants were written, before the run:
    `nova-doc`'s `an_impl_on_arrays_names_its_methods_after_array`, as no
    test read the docs' name of an impl on arrays (mutant 14), and
    `process_run_runs`' run with the marker in place, as no test reached the
    guard (mutant 64).
38. **The mutants leave the tree built** (5.0b's lesson): Task 9, Step 9
    ends with `cargo build --locked --workspace`, so `target/debug` holds
    the last commit's binaries for the steps after it.

**What the replay met**

39. **One flaky test.** In the first replay's Task 6 suite,
    `http_server_example_bounds_the_drain_for_a_pipelining_client`, a test
    of the HTTP example on port 3000 that no 5.0c patch touches, failed
    once, "connect to 127.0.0.1:3000: … Connection refused". It is the
    test's first recorded failure in 19 earlier full-suite logs, and the
    process limit was not near: the group's peak was 62 tasks of 600, on 4
    CPUs. It passed alone, three times, in that replay's other runs of the
    suite, and in all nine of this replay's, each code step's whole suite
    peaking at 62 tasks too. A step that sees it fail alone runs it again
    and records it in the ledger; any other new failure stops the task.

**The first fact-check's fixes,** in
`5.0c prototype: the first fact-check's fixes`

40. **std's inherent method wins over a program's trait method of the same
    name, as in Rust, and E0105 warns.** **(differs)** The user's ruling on
    the first fact-check. 5.0c gives existing types fourteen inherent
    methods, matching a diff of std's impls between the base and the last
    tree: `String`'s `byte_len`, `byte_at`, `byte_slice`, `parse_int`,
    `parse_int_radix` and `parse_float`, `Char`'s `to_int`, `Float`'s
    `to_bits`, `Vec`'s `at`, `last`, `to_array` and its function
    `from_array`, and arrays' `iter` and `slice`. A method call takes an
    inherent method before a trait's, so a program's
    `impl Size for String { fn byte_len … }`, whose `"é".byte_len()` ran the
    trait's method at the base, runs std's now; and a path takes an inherent
    function before a trait's, so beside a program's static `Mk::from_array`
    on `Vec<Int>`, `Vec::from_array([1, 2, 3])` is std's (the probe
    `static.nova` prints 3), where at the base it was E0011, `Vec` alone not
    picking the impl, and only a generic function reached the trait's, as it
    still does. The checker's pass after coherence reports E0105, a warning,
    at each such trait method, or at the impl for a default it takes: "std's
    inherent method `byte_len` on `String` hides `Size::byte_len`", labelled
    "`x.byte_len()` on a `String` runs std's method, not this one", or for
    two static functions "std's inherent function `from_array` on `Vec<Int>`
    hides `Mk::from_array`", labelled "`Vec::from_array(…)` runs std's
    function, not this one". Nova has no `Size::byte_len(x)` call (E0900),
    so the note says to reach the trait's method through a function generic
    over `T: Size`, whose `x.byte_len()` is the trait's (`T::from_array(…)`
    for a function), or to rename it; `hidden_trait_methods` shows both work
    on every backend (decision 45). **Only a pair that can meet warns** (the
    second fact-check): where one of the two takes `self` and the other does
    not, a method call never picks the selfless inherent function and a path
    reaches the static trait function, so
    `impl Tr for Vec<Int> { fn from_array(self) … }` and
    `impl Fresh for Vec<Int> { fn new(self) … }` draw nothing (the probes
    `fp.nova` and `fp2.nova`). Any std method a program can call counts:
    older ones, `Vec`'s `iter` or `Map`'s `len`, and private ones,
    `OrderedMap`'s `compact`, since a method's privacy is not enforced at a
    call (`priv3.nova` calls it). E0105 fires for no trait method std lacks,
    none on a type std's method is not on, and none a program's own inherent
    method hides (`a_programs_trait_method_hidden_by_a_std_method_is_e0105`,
    `a_trait_method_std_does_not_have_is_not_e0105`,
    `a_static_trait_function_hidden_by_a_std_function_is_e0105`; mutants 17
    to 27), and nowhere in std or the examples (Task 9, Step 6). **It is
    Task 1's,** with its `Vec` rows, so no task commit hides a trait method
    silently: Task 2 adds the `byte_len` and `to_int` rows, Task 4
    `to_bits`'s, Task 5 the parsers' (mutant 18 runs at Task 1's commit).
    Spec §6 gives 5.0c no diagnostic code; the CHANGELOG's Changed, ADR
    0050's Decisions 9 and 10, and `12-TYPESYSTEM.md`'s catalog and §8
    record it.
41. **A program's own inherent method of one of those names is E0074 now.**
    **(differs)** `impl String { fn byte_len … }`,
    `impl Char { fn to_int … }`, `impl Float { fn to_bits … }`,
    `impl<T> Vec<T> { fn last … }` and `impl<T> Vec<T> { fn from_array … }`
    compiled at the base, the program's method the one called; each meets
    std's impl now, labelled first at the program's impl (decision 18). The
    spec names no program that stops compiling; no repository program does
    (Task 9, Step 5). The CHANGELOG's Changed and ADR 0050's Consequences
    say so, beside decision 40's change.
42. **A method's name in a diagnostic is the source's type.** **(differs)**
    As with decisions 19 and 24, spec §6 gives 5.0c no diagnostic change.
    The resolver names an impl's methods after its self type, so E0016 said
    "`Arr_T.slice` takes 2 argument(s) but 1 were supplied". Four sites name
    an inherent method: E0060 (a `mut self` method on an immutable
    receiver), E0014 for a selfless function called as a method, E0016 (the
    arity), and E0014 for a receiver the impl does not fit. Each now calls
    `Checker::shown_method`: an array's method is `[T]::slice`, `[[T]]::m`
    for nested arrays (`shown_method_name`), a generic named type's
    `Vec::at`, not `Vec_T.at` (the second fact-check), and a plain type's
    stays `P.of`, as the checker's own tests expect. E0060 and E0016 are
    tested, "`[Int]::bump` mutates its receiver, but `a` is immutable",
    "`[T]::slice` takes …" and "`Vec::at` takes …"
    (`an_arity_error_on_an_array_method_names_it_as_written`; mutants 16 and
    28). The two E0014 sites are not reached by an array: the array arm's
    lookup skips a selfless function and a non-fitting impl and reports "no
    method `mk` on array type `[Int]`" first (the probes `e14sl.nova`,
    `e14rv.nova`), so no test can show them an array. No other place showed
    the resolver's name: E0014's fixes and the language server's completion
    use the method's own name, its hover the declaration's text, and
    `nova-doc` `Array` (decision 23). `shown_method_name` sits above
    `builtin_signature`'s doc comment, in Task 1's code.
43. **Two tests changed.** `ordered_map` updates its first key, `delta`,
    from 5 to 50, a value it did not hold, so an update that kept the old
    value, or appended the key again, shows in its output (mutant 63). The
    runtime's `current_exe_is_a_file` is dropped: it asked only that Rust's
    own `current_exe` names a file, and
    `current_exe_is_nova_under_run_and_the_program_when_built` runs 5.0c's
    builtin under `nova run` and built (mutant 72), so `process.rs` adds six
    tests, not seven.
44. **What `to_bits` promises.** It returns the bits of the value it is
    given, the same on every backend for the same value; a NaN made by
    arithmetic can itself differ by backend: `0.0 / 0.0` is
    `0xFFF8000000000000` under `nova run` and on WASM, and
    `0x7FF8000000000000` built `--release`, where LLVM folds it. So
    `float_bits` makes its NaNs with `parse_float`, whose bits are fixed;
    `std/core`'s comment, the CHANGELOG and `20-STDLIB.md` §2 say so, where
    they said "the same 64 bits on every backend".
**The second fact-check's fixes,** in `5.0c prototype: the second
fact-check's fixes`, besides decisions 18, 34, 40 to 42

45. **The second fact-check's nits, each fixed.** E0105 shows the impl's own
    parameter names, `Vec<T>` for `impl<T> … for Vec<T>`, not `Vec<T0>`, and
    a call of a method that takes arguments as `x.parse_int_radix(…)`
    (mutants 26 and 27); E0016 names `Vec::at` (decision 42); E0074 against
    std's impl puts the program's first (decision 18); E0013 on an
    interpolated array no longer advises implementing `Display`, which E0010
    refuses on an array, but "interpolate its elements, or wrap it in a
    record that does"
    (`an_interpolated_array_is_e0013_without_the_display_advice`; mutant
    29); the records say E0105 marks hides by older and private std methods
    too (decision 40). **A runtime test of E0105's advice,**
    `hidden_trait_methods`, rides with Task 1: on a `Vec<Int>`, `v.last()`
    and `v.at(0)` print std's 6 and 5, and functions generic over the trait
    print the trait's -1 and 100; `Vec::from_array` gives std's 3 elements
    and a function generic over `Make` the trait's one. It runs under
    `nova run`, built and in the WASM gate, and in the golden walk, and
    draws its three E0105s on purpose.

---

### Task 0: The baseline

The branch is `claude/loving-archimedes-frtf5q-50c`, at this plan's commit,
whose parent is `e13630f0`, `main` after 5.0b. The plan's commit adds this
file alone, so the patches apply to it as to `e13630f0`.

- [ ] **Step 1: The tree.**
  - `git status --short` prints nothing, on
    `claude/loving-archimedes-frtf5q-50c`.
  - `git diff --stat e13630f0 HEAD` lists only
    `docs/superpowers/plans/2026-10-29-phase-5-0c-the-std-a-compiler-needs.md`.
  - `BASE=$(git rev-parse --short HEAD)`: the plan's commit, recorded in the
    ledger. Task 9's review reads from it.
  - `df -h /` shows at least 7G free; if not, remove
    `target/debug/incremental` and older duplicate test executables in
    `target/debug/deps` first. The replay began with 11G and removed older
    duplicate test executables whenever less than 8G was free.
- [ ] **Step 2: The tools and the process limit.**
  - `clang-18 --version | head -n 1` prints
    `Ubuntu clang version 18.1.3 (1ubuntu1)`: CI's pin, for the golden
    suite.
  - `node --version` prints `v22.22.0`, for the WASM tests.
  - `cargo +1.78 --version` prints `cargo 1.78.0`, for Task 9.
  - The process limit, as the conventions set it: `id -u` says whether a
    pids control group is needed (`0`, root). Then the shell's membership:
    on cgroup v1, `grep nova-limit /proc/self/cgroup` prints
    `N:pids:/nova-limit` and `cat /sys/fs/cgroup/pids/nova-limit/pids.max`
    prints `600`; on cgroup v2, `cat /proc/self/cgroup` prints
    `0::/nova-limit` (or the systemd scope's path), and
    `cat "/sys/fs/cgroup$(sed -n 's/^0:://p' /proc/self/cgroup)/pids.max"`
    prints `600`; with `ulimit -Su 600` alone, `ulimit -Su` prints `600`.
    Record which way, and its output, in the ledger.
- [ ] **Step 3: The suite.** In a shell set up as the conventions say:

  ```sh
  cargo build --locked --workspace
  NOVA_REQUIRE_NODE=1 cargo test --locked --workspace --no-fail-fast > "$P/base-50c.txt" 2>&1
  grep '^test result' "$P/base-50c.txt" | awk '{p += $4; f += $6; i += $8} END {print p, f, i}'
  grep ' FAILED$' "$P/base-50c.txt"
  cp target/debug/nova "$P/nova-base-50c"
  cargo test --locked --workspace -- --list > "$P/base-list-50c.txt" 2>&1
  grep -c ': test$' "$P/base-list-50c.txt"
  ```

  Expected, from 5.0b's last full run on `e13630f0` and not the replay's:
  `2714 1 10`, and one failure, `test
  net::tests::an_ipv6_connect_parks_on_its_first_poll_and_then_establishes
  ... FAILED`. It needs IPv6, which this machine lacks, as in every
  earlier phase. The replay built `e13630f0` and listed its tests, and
  found `2725` names, the run's 2714 + 1 + 10; its `nova` was byte
  for byte the copy the prototype's checks used. The copy of `nova` is the
  base's, for Task 9's AST and `check` comparisons and its programs at each
  task; the list is the base's names, for Task 9's comparison. Task 9
  expects `2788 1 10`: 5.0c's tests make 74 more, 74 added and 0
  gone.
- [ ] **Step 4: The ledger.** Start `$P/ledger-50c.md`, headed with the base
  commit, the baseline's figures and the process limit's form.

### Task 1: Arrays and `Vec`

Spec §5.5; decisions 10, 15, 17, 18, 23, 24, 29 to 31, 40, 42 and 45.

**Files:**
- Modify:
  - `crates/nova-hir/src/lib.rs` (`TyHead::Array`, `Ty::head`)
  - `crates/nova-typeck/src/check.rs` (an array's methods after `len`, its
    members, E0074's two labels, E0010 for a trait impl on an array, E0013
    for an interpolated array, E0105, `shown_method` and
    `shown_method_name`), `crates/nova-typeck/src/check/fixes.rs` (`len` in
    E0014's fix on an array)
  - `crates/nova-doc/src/model.rs` (an impl on arrays is `Array`'s)
  - `std/collections/lib.nova` (`Vec`'s `at`, `last`, `to_array` and
    `from_array`; `impl<T> [T]`'s `iter` and `slice`; `ArrayIter`)
- Create: `crates/nova-typeck/src/check/std_tests.rs`,
  `tests/runtime/array_helpers.nova`, `array_helpers.stdout`,
  `tests/runtime/hidden_trait_methods.nova`, `hidden_trait_methods.stdout`
- Test:
  - `std_tests.rs` (fourteen tests; Tasks 2 to 7 add theirs), the checker's
    `members_of_a_string_and_of_an_array`
  - `crates/nova-mir/tests/lower_tests.rs` (one), `nova-doc`'s `model.rs` (one)
  - `crates/nova-cli/tests/run_tests.rs` (`array_helpers_run`,
    `hidden_trait_methods_run`, and `each_std_panic_names_its_method` with
    its table's array and `Vec` rows), `wasm_tests.rs`
    (`gate::array_helpers`, `gate::hidden_trait_methods`, and
    `each_std_panic_names_its_method_as_natively` with the same rows),
    `crates/nova-driver/tests/index_complete.rs` (the counts)

- [ ] **Step 1: The tests.** `patch_block t1-tests`. The patch holds:
  - `std_tests.rs`, its header, its helpers and Task 1's tests: an array's
    head is `TyHead::Array` whatever its element; std's `slice`, `iter` and
    `len` type through it; an array iterator's adapters infer their item; a
    program may write `impl<T> [T]` and `impl [Int]`, the second only for
    `[Int]`, E0014 "no method `total` on array type `[String]`" otherwise; a
    program's `iter` on arrays is E0074, its primary label at the program's
    impl and its second at std's, "`iter` is also defined by std's impl"; an
    unknown method is E0014 with "change `sclice` to `slice`" and "change
    `lne` to `len`"; `Vec`'s helpers' types; a trait impl on an array,
    `impl Display for [Int]` or `impl<T> Hash for [T]`, is E0010, "a trait
    cannot be implemented for an array", while `impl [Int]` checks; an arity
    error on `[1, 2].slice(1)` names "`[T]::slice`", E0060 on a `mut self`
    method "`[Int]::bump`", and `v.at()` "`Vec::at`"; an interpolated array
    is E0013 without the advice to implement `Display`; and E0105: a
    program's `Top::last` and `Top::at` on `Vec<T>` each draw it, shown as
    `Vec<T>`, with the note "generic over `T: Top`, whose `x.last()` is the
    trait's" and `x.at(…)`; a static `Mk::from_array` on `Vec<Int>` draws
    "std's inherent function …" with `Vec::from_array(…)`; and none fires
    for a trait method std lacks, one on another type, one a program's own
    inherent method hides, or a pair of which only one takes `self`
    (decision 40). The module is declared in `check.rs`.
  - `check.rs`'s members test: an array's members are `len`, built in, then
    `iter` and `slice`, which have a declaration.
  - `lower_tests.rs`: `impl<T> [T]`'s `slice` monomorphises once an element
    type. `model.rs`: an impl on arrays names its methods
    `impl.Array.method.second`.
  - **The program** `array_helpers`: empty arrays, slices at both ends,
    adapters over `iter()`, `next` to its end, round trips that copy, and
    arrays of strings, records and sums, with its `.stdout`, a test in
    `run_tests.rs` under `run`, `build` and GC stress, an entry in
    `wasm_tests.rs`'s `gate!`, and `index_complete.rs`'s counts, 177
    programs and 211 files, with `hidden_trait_methods` beside it.
  - **The program** `hidden_trait_methods`: a trait's `last` and `at` on
    `Vec<T>` and a static `from_array` on `Vec<Int>`, which `Vec`'s inherent
    methods hide; a call on the value prints std's results, a function
    generic over the trait the trait's, under `run`, `build` and in the gate
    (decision 45).
  - **The panic tables** of `run_tests.rs` and `wasm_tests.rs`, each with
    the program that prints `before`, evaluates the expression and would
    print `after`, and Task 1's rows: `[T]::slice`'s three bounds, `Vec::at`
    past each end and on a cleared vector, `Vec::last` on a new and a
    cleared one.
- [ ] **Step 2: Run, and see the tests fail.** First the build
  (Conventions), which ends with `Finished`.
  - `T check::std_tests check::tests::members_of_a_string_and_of_an_array an_impl_on_arrays_names_its_methods_after_array`.
    Expected: it does not compile, two
    ``error[E0599]: no variant, associated function, or constant named `Array` found for enum `nova_hir::TyHead` in the current scope``
    and
    ``error: could not compile `nova-typeck` (lib test) due to 2 previous errors``
    (decision 32).
  - `L array_methods`. Expected: `0 passed; 1 failed`,
    `array_methods_monomorphise_per_element_type`, its program not checking
    (`lower_tests.rs:24:5`): no `slice` on arrays.
  - `R array_helpers_run hidden_trait_methods_run each_std_panic_names_its_method`.
    Expected: `0 passed; 3 failed`, `nova run` refusing `array_helpers` with
    E0001 "cannot find type `ArrayIter`", E0014 "no method `slice` on array
    type `[Int]`" and the E0011s after them, `hidden_trait_methods` with
    E0011 "cannot call `Vec::from_array()`: an impl provides it, but `Vec`'s
    type arguments could not be determined from the qualifier alone to
    select it", the base's answer to that path, and the first panic program
    printing nothing where `before` is expected (`left: ""`), as
    `Vec::from_array` does not exist yet.
  - `W gate::array_helpers gate::hidden_trait_methods each_std_panic_names_its_method_as_natively every_runtime_program_is_in_the_gate_or_left_out`.
    Expected: `1 passed; 3 failed`, the three by the same errors; the gate's
    own check passes, the programs being in it.
  - `X`. Expected: `0 passed; 1 failed`:
    `every_name_in_the_corpora_is_indexed`,
    "tests/runtime/array_helpers.nova does not check cleanly: ["E0001 cannot
    find type `ArrayIter`", …]".
- [ ] **Step 3: The code.** `patch_block t1-code`:
  - **`nova-hir`:** `TyHead::Array`, which `Ty::head` gives any array, so an
    impl may name `[T]`; `match_args` tells `impl<T> [T]` and `impl [Int]`
    apart by the element type.
  - **`check.rs`:** after an array's built-in `len`, a method is looked up
    by `resolve_method_on` as any type's is, and only a method no impl has
    is E0014, its fix `method_fix`'s; the members of an array are `len`,
    then its impls' methods; E0074 between two inherent impls labels the
    earlier one too; a trait impl whose type is an array is E0010, with a
    note pointing to an inherent impl or a record; `shown_method_name` turns
    the resolver's `Arr_T.slice` into `[T]::slice` in the inherent calls'
    arity errors, and `shown_method` a generic type's `Vec_T.at` into
    `Vec::at`, at the four sites that name an inherent method; E0074 against
    std's impl labels the program's first; E0013 on an array says to
    interpolate its elements; and **E0105:** the impls' collection keeps
    whether each impl is std's, its methods' name spans and its parameter
    names; after coherence, `check_hidden_trait_methods` compares each of
    the program's trait impls, its methods and the trait's defaults it
    takes, with std's inherent impls on an overlapping type, and warns at
    each shared name where both take `self` or neither does. **`fixes.rs`:**
    `method_fix` adds `len` on an array.
  - **`nova-doc`:** an impl on `[T]` is named `Array`.
  - **`std/collections`:** `Vec::at` and `Vec::last`, which panic against
    `len`; `Vec::to_array` and `Vec::from_array`, which copy; `impl<T> [T]`
    with `iter`, which shares the array, and `slice`, which copies its range
    and panics on a bad one; `ArrayIter` and its `Iterator` impl. Each with
    a doc test.
- [ ] **Step 4: Run.** First the build, which ends with `Finished`.
  - `T check::std_tests check::tests::members_of_a_string_and_of_an_array an_impl_on_arrays_names_its_methods_after_array`.
    Expected: every `test result` `ok`, 16 passing in all: `nova-typeck`'s
    15 (`std_tests.rs`' 14 and the members test) and `nova-doc`'s 1.
  - `L array_methods`. Expected: `1 passed; 0 failed`.
  - `R array_helpers_run hidden_trait_methods_run each_std_panic_names_its_method`.
    Expected: `3 passed; 0 failed`; `hidden_trait_methods` prints its three
    E0105s on standard error.
  - `W gate::array_helpers gate::hidden_trait_methods each_std_panic_names_its_method_as_natively every_runtime_program_is_in_the_gate_or_left_out`.
    Expected: `4 passed; 0 failed`.
  - `X`. Expected: `1 passed; 0 failed`, in about 25 seconds.
  - `S > "$P/t1-50c.txt" 2>&1`, then `grep '^test result' "$P/t1-50c.txt"
    | awk '{p += $4; f += $6; i += $8} END {print p, f, i}'` and `grep '
    FAILED$' "$P/t1-50c.txt"` (about 5 min 50 s). Expected: `2736 1 10`,
    Task 0's figures and Task 1's 22 tests, and the one failure Task 0's
    IPv6 test.
  - `G`. Expected:
    `release golden: 139 passed, 9 excluded, Ubuntu clang version 18.1.3 (1ubuntu1)`
    and `1 passed`, in about 30 s: `array_helpers` and
    `hidden_trait_methods` join the walk.
- [ ] **Step 5: The checks,** as the conventions list them.
- [ ] **Step 6: Commit** `typeck, std: methods on arrays, and Vec's helpers`.

**Patch `t1-tests`:**

<!-- patch t1-tests -->

````diff
diff --git a/crates/nova-cli/tests/run_tests.rs b/crates/nova-cli/tests/run_tests.rs
index 41e15689737453afa4e51913c90c7832a6f4b437..a6dbc20b1346c74db1012356c60d2988d4c390ea 100644
--- a/crates/nova-cli/tests/run_tests.rs
+++ b/crates/nova-cli/tests/run_tests.rs
@@ -13040,6 +13040,81 @@ fn for_arrays_run() {
     golden_three_ways("for_arrays", true);
 }
 
+/// `iter` and `slice` on arrays, and `Vec`'s `to_array`, `from_array`, `at`
+/// and `last` (5.0 spec §5.5), also under GC stress.
+#[test]
+fn array_helpers_run() {
+    golden_three_ways("array_helpers", true);
+}
+
+/// A program's trait methods that `Vec`'s inherent methods hide: a call on
+/// the value runs std's, a function generic over the trait the trait's, a
+/// static function's too (E0105's advice, 5.0c's second fact-check).
+#[test]
+fn hidden_trait_methods_run() {
+    golden_three_ways("hidden_trait_methods", false);
+}
+
+/// Each panic of 5.0c's std (5.0 spec §5): the expression, and the
+/// message after `nova: panic: `. `wasm_tests.rs` has the same table.
+const STD_PANICS: &[(&str, &str)] = &[
+    ("[1, 2].slice(-1, 1)", "[T]::slice start is negative"),
+    (
+        "[1, 2].slice(0, 3)",
+        "[T]::slice end is past the end of the array",
+    ),
+    ("[1, 2].slice(2, 1)", "[T]::slice start is after end"),
+    (
+        "Vec::from_array([1, 2]).at(2)",
+        "Vec::at index out of range",
+    ),
+    (
+        "Vec::from_array([1, 2]).at(-1)",
+        "Vec::at index out of range",
+    ),
+    ("fresh().last()", "Vec::last on an empty vector"),
+    ("emptied().last()", "Vec::last on an empty vector"),
+    ("emptied().at(0)", "Vec::at index out of range"),
+];
+
+/// The program that prints `before`, then evaluates `expr`, which panics.
+/// `fresh()` is an empty `Vec`, and `emptied()` one cleared, whose storage
+/// still holds its values, so `at` and `last` must check `len`.
+fn std_panic_program(expr: &str) -> String {
+    format!(
+        "fn fresh() -> Vec<Int> {{ Vec::new() }}\n\n\
+         fn emptied() -> Vec<Int> {{\n    let mut v = Vec::from_array([1, 2])\n    v.clear()\n    v\n}}\n\n\
+         fn main() {{\n    println(\"before\")\n    let _ = {expr}\n    println(\"after\")\n}}\n"
+    )
+}
+
+/// Each panic of 5.0c's std, natively: the message, an abort, and nothing
+/// after it.
+#[test]
+fn each_std_panic_names_its_method() {
+    let dir = std::env::temp_dir().join("nova-std-panics");
+    let _ = std::fs::remove_dir_all(&dir);
+    std::fs::create_dir_all(&dir).expect("temp dir");
+    for (i, (expr, message)) in STD_PANICS.iter().enumerate() {
+        let src = dir.join(format!("p{i}.nova"));
+        std::fs::write(&src, std_panic_program(expr)).expect("write the program");
+        let a = nova()
+            .env("NO_COLOR", "1")
+            .arg("run")
+            .arg(&src)
+            .assert()
+            .failure();
+        let out = a.get_output();
+        let stdout = String::from_utf8_lossy(&out.stdout).replace("\r\n", "\n");
+        let stderr = String::from_utf8_lossy(&out.stderr);
+        assert_eq!(stdout, "before\n", "{expr}: {stderr}");
+        assert!(
+            stderr.contains(&format!("nova: panic: {message}")),
+            "{expr}: {stderr}"
+        );
+    }
+}
+
 /// A guard on an or-pattern runs once, with the bindings of the first
 /// alternative that matches, and a false guard goes on to the next arm, not
 /// to a later alternative (5.0 spec §4.6; ADR 0049): `(1, 9)` binds `a` to
diff --git a/crates/nova-cli/tests/wasm_tests.rs b/crates/nova-cli/tests/wasm_tests.rs
index 82fed9e7436f0beb4d44e1969e6d1a23d06189ca..3ddcccea919049e36dd0023382c54d4b16030d99 100644
--- a/crates/nova-cli/tests/wasm_tests.rs
+++ b/crates/nova-cli/tests/wasm_tests.rs
@@ -1158,6 +1158,52 @@ fn an_index_out_of_bounds_panics_as_natively() {
     );
 }
 
+/// Each panic of 5.0c's std (5.0 spec §5), on WASM: the native message, an
+/// abort's status, and nothing after it. `run_tests.rs`'s `STD_PANICS` is
+/// the same table.
+#[test]
+fn each_std_panic_names_its_method_as_natively() {
+    let Some(_) = node() else { return };
+    let cases: &[(&str, &str)] = &[
+        ("[1, 2].slice(-1, 1)", "[T]::slice start is negative"),
+        (
+            "[1, 2].slice(0, 3)",
+            "[T]::slice end is past the end of the array",
+        ),
+        ("[1, 2].slice(2, 1)", "[T]::slice start is after end"),
+        (
+            "Vec::from_array([1, 2]).at(2)",
+            "Vec::at index out of range",
+        ),
+        (
+            "Vec::from_array([1, 2]).at(-1)",
+            "Vec::at index out of range",
+        ),
+        ("fresh().last()", "Vec::last on an empty vector"),
+        ("emptied().last()", "Vec::last on an empty vector"),
+        ("emptied().at(0)", "Vec::at index out of range"),
+    ];
+    for (i, (expr, message)) in cases.iter().enumerate() {
+        let Some(node) = node() else { return };
+        let dir = fresh(&format!("std-panic-{i}"));
+        write(
+            &dir,
+            &[(
+                "main.nova",
+                &format!(
+                    "fn fresh() -> Vec<Int> {{ Vec::new() }}\n\n\
+                     fn emptied() -> Vec<Int> {{\n    let mut v = Vec::from_array([1, 2])\n    v.clear()\n    v\n}}\n\n\
+                     fn main() {{\n    println(\"before\")\n    let _ = {expr}\n    println(\"after\")\n}}\n"
+                ),
+            )],
+        );
+        let dist = build_wasm(&dir, Path::new("main.nova"));
+        let a = run_wasm(node, &dist).code(134);
+        assert_eq!(stdout(&a), "before\n", "{expr}");
+        assert_eq!(stderr(&a), format!("nova: panic: {message}\n"), "{expr}");
+    }
+}
+
 /// The shim's `Float` text is `nova-runtime`'s, exact ties included (spec
 /// §6, §9.3).
 #[test]
@@ -1964,6 +2010,10 @@ mod gate {
         patterns_async,
         match_effects,
         for_arrays,
+        // 5.0c's std (5.0 spec §5.1 to §5.6): Nova over existing MIR, or
+        // over a builtin that is a shim entry on WASM.
+        array_helpers,
+        hidden_trait_methods,
     );
 
     /// The programs a test of their own in this module checks, by its name.
diff --git a/crates/nova-doc/src/model.rs b/crates/nova-doc/src/model.rs
index 4ffdc4826a498540fc3179dd4abfa9088d573053..1220c35aaf35c8808cab9945622993a4ca1b4a94 100644
--- a/crates/nova-doc/src/model.rs
+++ b/crates/nova-doc/src/model.rs
@@ -1531,6 +1531,33 @@ mod tests {
         assert_eq!(doubled, ["impl.Int.method.doubled"]);
     }
 
+    /// An impl on arrays has no type name to go by: its methods are
+    /// `Array`'s, as std's `impl<T> [T]` gives `Array::iter` (5.0 spec
+    /// §5.5).
+    #[test]
+    fn an_impl_on_arrays_names_its_methods_after_array() {
+        let dir = fresh("array-impl");
+        write(
+            &dir,
+            &[
+                ("nova.toml", &manifest("arrays", "")),
+                (
+                    "src/lib.nova",
+                    "impl<T> [T] {\n    /// The second element.\n    pub fn second(self) -> T { self[1] }\n}\n",
+                ),
+            ],
+        );
+        let d = documented(&dir);
+        let lib = module(&d.site, "arrays/index.html");
+        assert_eq!(lib.other_impls.len(), 1, "the impl on arrays");
+        let methods: Vec<&str> = lib.other_impls[0]
+            .methods
+            .iter()
+            .map(|m| m.anchor.as_str())
+            .collect();
+        assert_eq!(methods, ["impl.Array.method.second"]);
+    }
+
     const PAIRS: &str = "/// Two of a kind.\npub record Pair<T> {\n    pub a: T,\n    pub b: T,\n}\n\nimpl Pair<Int> {\n    /// Their sum.\n    pub fn total(self) -> Int { self.a + self.b }\n}\n\nimpl Pair<String> {\n    /// The two, joined.\n    pub fn total(self) -> String { \"${self.a}${self.b}\" }\n}\n";
 
     #[test]
diff --git a/crates/nova-driver/tests/index_complete.rs b/crates/nova-driver/tests/index_complete.rs
index 301faefeeaabca0c9d310f9c075b610525c29bd1..bdcaa39411f2897468f2a14f1d0ee9a74d9d853b 100644
--- a/crates/nova-driver/tests/index_complete.rs
+++ b/crates/nova-driver/tests/index_complete.rs
@@ -248,8 +248,9 @@ fn every_name_in_the_corpora_is_indexed() {
     // 159 runtime programs on 2026-10-26, after 5.0a added `try_operator`,
     // `try_async`, `compound_assign_places` and `string_concat`; 161 on
     // 2026-10-27, after 5.0b added `tuples` and `tuples_async`; 165 after
-    // `patterns`, `patterns_async`, `match_effects` and `for_arrays`.
-    assert_eq!(programs.len(), 175, "{programs:#?}");
+    // `patterns`, `patterns_async`, `match_effects` and `for_arrays`; 166
+    // after 5.0c added `array_helpers`; 167 after `hidden_trait_methods`.
+    assert_eq!(programs.len(), 177, "{programs:#?}");
     let mut files = 0;
     let mut failures: Vec<String> = Vec::new();
     for (k, path) in programs.iter().enumerate() {
@@ -276,11 +277,11 @@ fn every_name_in_the_corpora_is_indexed() {
             every_name_is_covered(&a, file, &mut failures);
         }
     }
-    // 9 + 165 + 3 files of user code, 05-geometry's library's 2, the 8
+    // 9 + 167 + 3 files of user code, 05-geometry's library's 2, the 8
     // modules 07-fullstack-blog's client reaches from its `main.nova` (not
     // the server's 4, which only `server.nova` and `lib.nova` reach), and
     // native std's 22, `std/ui/ssr` among them.
-    assert_eq!(files, 209);
+    assert_eq!(files, 211);
     failures.sort();
     failures.dedup();
     assert!(
diff --git a/crates/nova-mir/tests/lower_tests.rs b/crates/nova-mir/tests/lower_tests.rs
index def2254f861dec6bec9900baa69e02e091b79401..7d95c1babcd94a7802a8f0a7ff781ca0a76d0a61 100644
--- a/crates/nova-mir/tests/lower_tests.rs
+++ b/crates/nova-mir/tests/lower_tests.rs
@@ -2373,3 +2373,19 @@ fn a_flat_char_match_switches_on_its_code_points() {
         .expect("a switch");
     assert_eq!(keys, ['a' as i64, 'b' as i64]);
 }
+
+/// `impl<T> [T]`'s methods monomorphise per element type, as any generic
+/// impl's do (spec §5.5).
+#[test]
+fn array_methods_monomorphise_per_element_type() {
+    let mir = mir_for(
+        "fn main() {\n\
+             let a = [1, 2, 3].slice(0, 2)\n\
+             let b = [\"x\", \"y\"].slice(1, 2)\n\
+             println(\"${a.len()} ${b[0]}\")\n\
+         }",
+    );
+    let names = function_names(&mir);
+    let slices: Vec<&&str> = names.iter().filter(|n| n.contains("slice")).collect();
+    assert_eq!(slices.len(), 2, "one `slice` per element type: {names:?}");
+}
diff --git a/crates/nova-typeck/src/check.rs b/crates/nova-typeck/src/check.rs
index 90338374fb42dbba583a3ee8c59907e120197414..faea4ddec0b9a70461f50a6dd4c1672c9bb205de 100644
--- a/crates/nova-typeck/src/check.rs
+++ b/crates/nova-typeck/src/check.rs
@@ -26,6 +26,8 @@ mod lang_tests;
 mod pattern_tests;
 mod patterns;
 #[cfg(test)]
+mod std_tests;
+#[cfg(test)]
 mod tuple_tests;
 mod view;
 
@@ -18187,8 +18189,12 @@ mod tests {
         let (s, _) = probe_src("fn main() {\n    let n = \"a\".<|>\n}\n");
         assert!(member_names(&s).contains(&"len"), "{:?}", member_names(&s));
         let (a, _) = probe_src("fn main() {\n    let n = [1, 2].<|>\n}\n");
-        assert_eq!(member_names(&a), ["len"]);
-        assert_eq!(a.probe.members[0].decl, None);
+        // `len` is built in; `iter` and `slice` are std's `impl<T> [T]`.
+        assert_eq!(member_names(&a), ["len", "iter", "slice"]);
+        let len = a.probe.members.iter().find(|m| m.name == "len").unwrap();
+        assert_eq!(len.decl, None);
+        let iter = a.probe.members.iter().find(|m| m.name == "iter").unwrap();
+        assert!(iter.decl.is_some());
     }
 
     #[test]
diff --git a/crates/nova-typeck/src/check/std_tests.rs b/crates/nova-typeck/src/check/std_tests.rs
new file mode 100644
index 0000000000000000000000000000000000000000..cf30caaaeaf195e71ca38eae59300918f9820c2f
--- /dev/null
+++ b/crates/nova-typeck/src/check/std_tests.rs
@@ -0,0 +1,367 @@
+//! The checker's tests of 5.0c's std (spec
+//! `docs/superpowers/specs/2026-10-26-phase-5-0-the-language-and-std-a-compiler-needs-design.md`
+//! §5):
+//! - `TyHead::Array`, `[T]`'s impls and `Vec`'s helpers, and the E0010,
+//!   E0014, E0060 and E0074 an impl on arrays meets;
+//! - E0105, a program's trait method one of std's inherent methods hides;
+//! - the new items' types;
+
+use super::*;
+use nova_diagnostics::{FileId, Severity};
+use nova_lexer::lex;
+use nova_parser::parse;
+use nova_resolver::resolve;
+
+/// `src` checked, the resolver's diagnostics first.
+fn checked(src: &str) -> CheckResult {
+    let file_id = FileId::DUMMY;
+    let (tokens, lex_errors) = lex(src, file_id);
+    assert!(lex_errors.is_empty(), "lex errors: {lex_errors:?}");
+    let (ast, parse_errors) = parse(&tokens, file_id);
+    assert!(parse_errors.is_empty(), "parse errors: {parse_errors:?}");
+    let resolved = resolve(&ast.expect("an AST"));
+    let mut result = check(&resolved.file, &resolved.definitions);
+    let mut all = resolved.diagnostics;
+    all.append(&mut result.diagnostics);
+    result.diagnostics = all;
+    result
+}
+
+fn errors(src: &str) -> Vec<Diagnostic> {
+    checked(src)
+        .diagnostics
+        .into_iter()
+        .filter(|d| d.severity == Severity::Error)
+        .collect()
+}
+
+fn clean(src: &str) {
+    let all = checked(src).diagnostics;
+    assert!(all.is_empty(), "{all:#?}");
+}
+
+/// The type of the local `name` in `main`.
+fn local_ty(r: &CheckResult, name: &str) -> Ty {
+    r.module
+        .functions
+        .iter()
+        .find(|f| f.name == "main")
+        .expect("a `main`")
+        .locals
+        .iter()
+        .find(|l| l.name == name)
+        .unwrap_or_else(|| panic!("a local `{name}`"))
+        .ty
+        .clone()
+}
+
+#[test]
+fn an_arrays_head_is_array_whatever_its_element() {
+    assert_eq!(Ty::Array(Box::new(Ty::Int)).head(), Some(TyHead::Array));
+    assert_eq!(
+        Ty::Array(Box::new(Ty::Array(Box::new(Ty::String)))).head(),
+        Some(TyHead::Array)
+    );
+}
+
+#[test]
+fn std_array_methods_type_check_through_the_arrays_head() {
+    let r = checked(
+        "fn main() {\n    let a = [1, 2, 3]\n    let s = a.slice(0, 2)\n    \
+         let it = a.iter()\n    let n = a.len()\n    let w = [\"x\"].slice(0, 1)\n}\n",
+    );
+    assert!(r.diagnostics.is_empty(), "{:#?}", r.diagnostics);
+    assert_eq!(local_ty(&r, "s"), Ty::Array(Box::new(Ty::Int)));
+    assert_eq!(local_ty(&r, "w"), Ty::Array(Box::new(Ty::String)));
+    assert_eq!(local_ty(&r, "n"), Ty::Int);
+    assert!(
+        matches!(&local_ty(&r, "it"), Ty::Record { args, .. } if args.as_slice() == [Ty::Int]),
+        "{:?}",
+        local_ty(&r, "it")
+    );
+}
+
+#[test]
+fn an_array_iterators_adapters_infer_their_item() {
+    clean(
+        "fn main() {\n    let v = [1, 2, 3].iter().map(|x| x * 2).collect()\n    \
+         println(\"${v.at(0)}\")\n}\n",
+    );
+}
+
+/// A program may write an inherent impl on arrays, as on `String` (spec
+/// §5.5), generic or on one element type; the concrete one applies only to
+/// its element type.
+#[test]
+fn a_program_may_write_an_impl_on_arrays() {
+    clean(
+        "impl<T> [T] {\n    fn second(self) -> T { self[1] }\n}\n\
+         impl [Int] {\n    fn total(self) -> Int {\n        let mut t = 0\n        \
+         for x in self { t += x }\n        t\n    }\n}\n\
+         fn main() {\n    println(\"${[\"a\", \"b\"].second()} ${[1, 2].total()}\")\n}\n",
+    );
+    let e = errors(
+        "impl [Int] {\n    fn total(self) -> Int { 0 }\n}\n\
+         fn main() {\n    println(\"${[\"a\"].total()}\")\n}\n",
+    );
+    assert_eq!(e.len(), 1, "{e:#?}");
+    assert_eq!(e[0].code, "E0014");
+    assert_eq!(e[0].message, "no method `total` on array type `[String]`");
+}
+
+/// A program's own `iter` on arrays takes the name std's has, which two
+/// overlapping inherent impls may not both define.
+#[test]
+fn a_programs_array_method_that_std_has_is_e0074() {
+    let e = errors("impl<T> [T] {\n    fn iter(self) -> Int { 0 }\n}\nfn main() {}\n");
+    assert!(
+        e.iter().any(|d| d.code == "E0074"
+            && d.message == "method `iter` is defined by multiple overlapping inherent impls"),
+        "{e:#?}"
+    );
+}
+
+/// A method no impl has is still E0014, and its fix offers `len` with the
+/// impls' methods.
+#[test]
+fn an_unknown_array_method_is_e0014_with_its_methods_offered() {
+    let e = errors("fn main() {\n    let n = [1, 2].sclice(0, 1)\n}\n");
+    assert_eq!(e.len(), 1, "{e:#?}");
+    assert_eq!(e[0].code, "E0014");
+    assert_eq!(e[0].message, "no method `sclice` on array type `[Int]`");
+    assert_eq!(
+        e[0].fixes
+            .iter()
+            .map(|f| f.title.as_str())
+            .collect::<Vec<_>>(),
+        ["change `sclice` to `slice`"]
+    );
+    let e = errors("fn main() {\n    let n = [1, 2].lne()\n}\n");
+    assert_eq!(
+        e[0].fixes
+            .iter()
+            .map(|f| f.title.as_str())
+            .collect::<Vec<_>>(),
+        ["change `lne` to `len`"]
+    );
+}
+
+/// `Vec`'s helpers' types (spec §5.5).
+#[test]
+fn vecs_helpers_have_the_specs_types() {
+    let r = checked(
+        "fn main() {\n    let mut v: Vec<Int> = Vec::from_array([1])\n    \
+         let va = v.to_array()\n    let at = v.at(0)\n    let last = v.last()\n}\n",
+    );
+    assert!(r.diagnostics.is_empty(), "{:#?}", r.diagnostics);
+    assert_eq!(local_ty(&r, "va"), Ty::Array(Box::new(Ty::Int)));
+    assert_eq!(local_ty(&r, "at"), Ty::Int);
+    assert_eq!(local_ty(&r, "last"), Ty::Int);
+}
+
+/// A program's impl that defines a method std's impl on the same type has,
+/// `iter` on arrays, is E0074 labelled at the program's impl, its primary
+/// label, and at std's, which the program cannot change (5.0 spec §5.5;
+/// 5.0c's second fact-check).
+#[test]
+fn e0074_against_stds_array_impl_labels_the_programs_impl() {
+    let src = "impl<T> [T] {\n    fn iter(self) -> Int { 0 }\n}\nfn main() {}\n";
+    let e = errors(src);
+    let d = e.iter().find(|d| d.code == "E0074").expect("an E0074");
+    let at = src.find("[T]").expect("the impl's type") as u32;
+    assert!(
+        d.labels
+            .iter()
+            .any(|l| l.primary && l.span.start == at && l.span.end == at + 3),
+        "{d:#?}"
+    );
+    assert!(
+        d.labels
+            .iter()
+            .any(|l| !l.primary && l.message == "`iter` is also defined by std's impl"),
+        "{d:#?}"
+    );
+}
+
+fn warnings(src: &str) -> Vec<Diagnostic> {
+    checked(src)
+        .diagnostics
+        .into_iter()
+        .filter(|d| d.severity == Severity::Warning)
+        .collect()
+}
+
+/// The messages of `src`'s E0105s.
+fn e0105s(src: &str) -> Vec<String> {
+    warnings(src)
+        .into_iter()
+        .filter(|d| d.code == "E0105")
+        .map(|d| d.message)
+        .collect()
+}
+
+/// A program's trait method that one of std's inherent methods hides, as
+/// `Vec`'s `last` and `at` do since 5.0c, is E0105, a warning at the trait
+/// impl's method: a call takes the inherent method (the user's ruling on
+/// 5.0c's first fact-check). The type is shown with the impl's own
+/// parameter names, and a method that takes arguments as `x.at(…)`.
+#[test]
+fn a_programs_trait_method_hidden_by_a_std_method_is_e0105() {
+    let src = "trait Top {\n    fn last(self) -> Int\n    fn at(self, i: Int) -> Int\n}\n\
+               impl<T> Top for Vec<T> {\n    fn last(self) -> Int { 0 - 1 }\n    \
+               fn at(self, i: Int) -> Int { i }\n}\n\
+               fn main() {}\n";
+    let w = warnings(src);
+    let messages: Vec<&str> = w
+        .iter()
+        .filter(|d| d.code == "E0105")
+        .map(|d| d.message.as_str())
+        .collect();
+    assert_eq!(
+        messages,
+        [
+            "std's inherent method `last` on `Vec<T>` hides `Top::last`",
+            "std's inherent method `at` on `Vec<T>` hides `Top::at`",
+        ],
+        "{w:#?}"
+    );
+    let first = w.iter().find(|d| d.code == "E0105").expect("an E0105");
+    let at = src.find("last(self) -> Int {").expect("the impl's method") as u32;
+    assert!(
+        first.labels.iter().any(|l| l.primary && l.span.start == at),
+        "{first:#?}"
+    );
+    assert!(
+        first
+            .notes
+            .iter()
+            .any(|n| n.contains("generic over `T: Top`, whose `x.last()` is the trait's")),
+        "{first:#?}"
+    );
+    let second = w
+        .iter()
+        .rfind(|d| d.code == "E0105")
+        .expect("a second E0105");
+    assert!(
+        second
+            .notes
+            .iter()
+            .any(|n| n.contains("whose `x.at(…)` is the trait's")),
+        "{second:#?}"
+    );
+}
+
+/// No E0105 for a trait method std has no inherent method of, for one
+/// named as std's but on a type std's is not on, for one the program's own
+/// inherent method hides, or where only one of the two takes `self`: a
+/// method call never picks a selfless inherent function, and a path reaches
+/// a static trait function (5.0c's second fact-check).
+#[test]
+fn a_trait_method_std_does_not_have_is_not_e0105() {
+    clean(
+        "trait Size {\n    fn width(self) -> Int\n}\n\
+         impl Size for String {\n    fn width(self) -> Int { self.len() }\n}\n\
+         trait Bytes {\n    fn byte_len(self) -> Int\n}\n\
+         impl Bytes for Int {\n    fn byte_len(self) -> Int { 8 }\n}\n\
+         trait Halve {\n    fn half(self) -> Int\n}\n\
+         impl Halve for Int {\n    fn half(self) -> Int { self / 2 }\n}\n\
+         impl Int {\n    fn half(self) -> Int { self / 2 }\n}\n\
+         trait Tr {\n    fn from_array(self) -> Int\n}\n\
+         impl Tr for Vec<Int> {\n    fn from_array(self) -> Int { 42 }\n}\n\
+         trait Fresh {\n    fn new(self) -> Int\n}\n\
+         impl Fresh for Vec<Int> {\n    fn new(self) -> Int { 7 }\n}\n\
+         fn main() {\n    let n = 3\n    let v: Vec<Int> = Vec::new()\n    \
+         println(\"${\"ab\".width()} ${n.byte_len()} ${n.half()} ${v.from_array()} ${v.new()}\")\n}\n",
+    );
+}
+
+/// A static trait function that std's inherent function of the same name
+/// hides at a path, `Vec::from_array(a)`, is E0105 too, its note naming
+/// the path and the generic function's `T::from_array(…)`.
+#[test]
+fn a_static_trait_function_hidden_by_a_std_function_is_e0105() {
+    let src = "trait Mk {\n    fn from_array(a: [Int]) -> Self\n}\n\
+               impl Mk for Vec<Int> {\n    fn from_array(a: [Int]) -> Vec<Int> { Vec::new() }\n}\n\
+               fn main() {}\n";
+    assert_eq!(
+        e0105s(src),
+        ["std's inherent function `from_array` on `Vec<Int>` hides `Mk::from_array`"]
+    );
+    let w = warnings(src);
+    let d = w.iter().find(|d| d.code == "E0105").expect("an E0105");
+    assert!(
+        d.labels
+            .iter()
+            .any(|l| l.primary
+                && l.message == "`Vec::from_array(…)` runs std's function, not this one"),
+        "{d:#?}"
+    );
+    assert!(
+        d.notes
+            .iter()
+            .any(|n| n.contains("whose `T::from_array(…)` is the trait's")),
+        "{d:#?}"
+    );
+}
+
+/// An array takes inherent impls alone: a trait impl on one stays E0010, as
+/// before 5.0c, while `impl [Int] { … }` checks (the user's ruling on 5.0c's
+/// first fact-check).
+#[test]
+fn a_trait_impl_on_an_array_is_e0010() {
+    for src in [
+        "impl Display for [Int] {\n    fn fmt(self) -> String { \"a\" }\n}\nfn main() {}\n",
+        "impl<T> Hash for [T] {\n    fn hash(self) -> Int { 0 }\n}\nfn main() {}\n",
+    ] {
+        let e = errors(src);
+        assert!(
+            e.iter()
+                .any(|d| d.code == "E0010"
+                    && d.message == "a trait cannot be implemented for an array"),
+            "{e:#?}"
+        );
+    }
+    clean("impl [Int] {\n    fn total(self) -> Int { 0 }\n}\nfn main() {}\n");
+}
+
+/// An array's method is named as the source writes the type, `[T]::slice`,
+/// not the resolver's `Arr_T.slice`, in an arity error and in E0060; a
+/// generic named type's as `Vec::at`, not `Vec_T.at` (5.0c's fact-checks).
+#[test]
+fn an_arity_error_on_an_array_method_names_it_as_written() {
+    let e = errors("fn main() {\n    let x = [1, 2].slice(1)\n}\n");
+    assert!(
+        e.iter().any(|d| d.code == "E0016"
+            && d.message == "`[T]::slice` takes 2 argument(s) but 1 were supplied"),
+        "{e:#?}"
+    );
+    let e = errors(
+        "impl [Int] {\n    fn bump(mut self) { self[0] = 1 }\n}\n\
+         fn main() {\n    let a = [1]\n    a.bump()\n}\n",
+    );
+    assert!(
+        e.iter().any(|d| d.code == "E0060"
+            && d.message == "`[Int]::bump` mutates its receiver, but `a` is immutable"),
+        "{e:#?}"
+    );
+    let e = errors("fn main() {\n    let v: Vec<Int> = Vec::new()\n    let x = v.at()\n}\n");
+    assert!(
+        e.iter().any(|d| d.code == "E0016"
+            && d.message == "`Vec::at` takes 1 argument(s) but 0 were supplied"),
+        "{e:#?}"
+    );
+}
+
+/// An array in an interpolation is E0013 without the advice to implement
+/// `Display`, which E0010 refuses on an array (5.0c's second fact-check).
+#[test]
+fn an_interpolated_array_is_e0013_without_the_display_advice() {
+    let e = errors("fn main() {\n    let a = [1]\n    println(\"${a}\")\n}\n");
+    let d = e.iter().find(|d| d.code == "E0013").expect("an E0013");
+    assert_eq!(
+        d.message,
+        "`[Int]` cannot be interpolated into a string; an array cannot implement \
+         `Display`; interpolate its elements, or wrap it in a record that does",
+        "{d:#?}"
+    );
+}
diff --git a/tests/runtime/array_helpers.nova b/tests/runtime/array_helpers.nova
new file mode 100644
index 0000000000000000000000000000000000000000..6cc86568607b17a6701bb666e581cec3a6b6721e
--- /dev/null
+++ b/tests/runtime/array_helpers.nova
@@ -0,0 +1,98 @@
+// `iter` and `slice` on arrays, `Vec::to_array` and `from_array`, and
+// `Vec::at` and `last` (5.0 spec §5.5): empty arrays, slices at both ends,
+// iterator adapters over `iter()`, round trips that copy, and arrays of
+// strings, records and sums, also under GC stress, since every helper
+// allocates.
+
+record Point { x: Int, y: Int }
+
+fn join_ints(a: [Int]) -> String {
+    let mut parts = ""
+    for i in 0..a.len() {
+        if i > 0 { parts = parts + "," }
+        parts = parts + "${a[i]}"
+    }
+    "[${parts}]"
+}
+
+fn main() {
+    let a = [1, 2, 3, 4, 5]
+    let empty: [Int] = []
+
+    let mut seen = 0
+    for x in empty.iter() { seen += x }
+    println("empty iter ${seen}, empty slice ${empty.slice(0, 0).len()}")
+
+    println(
+        "slices ${join_ints(a.slice(0, 2))} ${join_ints(a.slice(3, 5))} ${join_ints(a.slice(0, 5))} ${join_ints(a.slice(2, 2))} ${join_ints(a.slice(5, 5))}",
+    )
+
+    let s = a.slice(1, 4)
+    let mut copy = a.slice(0, a.len())
+    copy[0] = 100
+    println("a slice copies: a[0]=${a[0]} copy[0]=${copy[0]} s=${join_ints(s)}")
+
+    let evens = a.iter().filter(|x| x % 2 == 0).map(|x| x * 10).collect()
+    println("adapters ${evens.len()} ${evens.at(0)} ${evens.last()}")
+
+    let mut total = 0
+    loop_all(a.iter())
+    for x in a.iter() { total += x }
+    println("sum ${total}")
+
+    let words = ["alpha", "beta", "gamma"]
+    let mut joined = ""
+    for w in words.iter() { joined = joined + w.to_upper() }
+    println("words ${joined} ${words.slice(1, 3)[0]}")
+
+    let pts = [Point { x: 1, y: 2 }, Point { x: 3, y: 4 }]
+    let ys = pts.iter().map(|p: Point| p.y).collect()
+    println("records ${ys.at(0)} ${ys.at(1)}")
+
+    let opts = [Some(1), None, Some(3)]
+    let somes = opts.iter().filter(|o: Option<Int>| o.is_some()).collect()
+    println("sums ${somes.len()} ${opts.slice(1, 2)[0].is_none()}")
+
+    let mut v: Vec<String> = Vec::new()
+    let before = v.to_array()
+    v.push("x")
+    v.push("y")
+    v.push("z")
+    let arr = v.to_array()
+    v.set(0, "changed")
+    println(
+        "to_array ${before.len()} ${arr.len()} ${arr[0]} ${arr[2]} vec ${v.at(0)} last ${v.last()}",
+    )
+
+    let mut w = Vec::from_array(a)
+    w.push(6)
+    w.set(0, 0)
+    println("from_array ${w.len()} first ${w.at(0)} last ${w.last()} a[0] ${a[0]}")
+    let mut shares = Vec::from_array(a)
+    shares.set(1, 99)
+    println("from_array copies: a[1] ${a[1]} vec ${shares.at(1)}")
+    let round = Vec::from_array(w.to_array())
+    println("round trip ${join_ints(round.to_array())}")
+    let none: Vec<Int> = Vec::from_array([])
+    println("from empty ${none.len()} ${none.is_empty()}")
+
+    let mut big: Vec<String> = Vec::new()
+    for i in 0..2000 { big.push("n${i}") }
+    let big_arr = big.to_array()
+    let tail = big_arr.slice(1990, 2000)
+    let mut count = 0
+    for x in big_arr.iter() { if x.len() == 5 { count += 1 } }
+    println("big ${big_arr.len()} ${tail[0]} ${tail.len()} five-char ${count} at ${big.at(1234)}")
+}
+
+fn loop_all(mut it: ArrayIter<Int>) {
+    let mut n = 0
+    let mut more = true
+    while more {
+        match it.next() {
+            Some(_) => n += 1
+            None => more = false
+        }
+    }
+    println("next ${n} then ${it.next().is_none()}")
+}
diff --git a/tests/runtime/array_helpers.stdout b/tests/runtime/array_helpers.stdout
new file mode 100644
index 0000000000000000000000000000000000000000..b0f8b106a2dc15a649edf772d962fb171bcf7cd0
--- /dev/null
+++ b/tests/runtime/array_helpers.stdout
@@ -0,0 +1,15 @@
+empty iter 0, empty slice 0
+slices [1,2] [4,5] [1,2,3,4,5] [] []
+a slice copies: a[0]=1 copy[0]=100 s=[2,3,4]
+adapters 2 20 40
+next 5 then true
+sum 15
+words ALPHABETAGAMMA beta
+records 2 4
+sums 2 true
+to_array 0 3 x z vec changed last z
+from_array 6 first 0 last 6 a[0] 1
+from_array copies: a[1] 2 vec 99
+round trip [0,2,3,4,5,6]
+from empty 0 true
+big 2000 n1990 10 five-char 1000 at n1234
diff --git a/tests/runtime/hidden_trait_methods.nova b/tests/runtime/hidden_trait_methods.nova
new file mode 100644
index 0000000000000000000000000000000000000000..ec69f85e0e27efb3a13f3011554eb90a072b742a
--- /dev/null
+++ b/tests/runtime/hidden_trait_methods.nova
@@ -0,0 +1,43 @@
+// A program's trait methods that std's inherent methods of the same names
+// hide (5.0 spec §5.5; 5.0c's E0105). A call on the value runs std's; a
+// function generic over the trait reaches the trait's, as E0105's note
+// says, for a method and for a static function alike.
+
+trait Top {
+    fn last(self) -> Int
+    fn at(self, i: Int) -> Int
+}
+
+impl<T> Top for Vec<T> {
+    fn last(self) -> Int { 0 - 1 }
+    fn at(self, i: Int) -> Int { 100 + i }
+}
+
+trait Make {
+    fn from_array(a: [Int]) -> Self
+}
+
+impl Make for Vec<Int> {
+    fn from_array(a: [Int]) -> Vec<Int> {
+        let mut v: Vec<Int> = Vec::new()
+        v.push(99)
+        v
+    }
+}
+
+fn top_last<T: Top>(x: T) -> Int { x.last() }
+
+fn top_at<T: Top>(x: T, i: Int) -> Int { x.at(i) }
+
+fn made<T: Make>() -> T { T::from_array([1, 2, 3]) }
+
+fn main() {
+    let mut v: Vec<Int> = Vec::new()
+    v.push(5)
+    v.push(6)
+    println("std's: last ${v.last()}, at(0) ${v.at(0)}")
+    println("the trait's: last ${top_last(v)}, at(0) ${top_at(v, 0)}")
+    let a: Vec<Int> = Vec::from_array([1, 2, 3])
+    let b: Vec<Int> = made()
+    println("std's from_array: ${a.len()}; the trait's: ${b.len()}, ${b.at(0)}")
+}
diff --git a/tests/runtime/hidden_trait_methods.stdout b/tests/runtime/hidden_trait_methods.stdout
new file mode 100644
index 0000000000000000000000000000000000000000..be26975e81a748dce2b1fa21d8b8956368ff618f
--- /dev/null
+++ b/tests/runtime/hidden_trait_methods.stdout
@@ -0,0 +1,3 @@
+std's: last 6, at(0) 5
+the trait's: last -1, at(0) 100
+std's from_array: 3; the trait's: 1, 99
````

**Patch `t1-code`:**

<!-- patch t1-code -->

````diff
diff --git a/crates/nova-doc/src/model.rs b/crates/nova-doc/src/model.rs
index 1220c35aaf35c8808cab9945622993a4ca1b4a94..a1bc8c94f125ebcf6caa4be55314fc04b3765d03 100644
--- a/crates/nova-doc/src/model.rs
+++ b/crates/nova-doc/src/model.rs
@@ -1014,7 +1014,12 @@ impl<'a> Ctx<'a> {
                 continue;
             };
             let on = type_name(&block.ty.value);
-            let on_name = on.map_or_else(|| "type".to_string(), |s| s.value.clone());
+            // An impl on arrays, `impl<T> [T]`, has no name to go by: its
+            // methods are `Array::iter` and the like (5.0 spec §5.5).
+            let on_name = match &block.ty.value {
+                Type::Array(_) => "Array".to_string(),
+                _ => on.map_or_else(|| "type".to_string(), |s| s.value.clone()),
+            };
             let ty = on.and_then(|s| self.uses.get(&s.span)).copied();
             let dest = match ty {
                 Some(Target::Def(id)) => {
diff --git a/crates/nova-hir/src/lib.rs b/crates/nova-hir/src/lib.rs
index 042d501da14bfc4f92ae4f4248de51c88b6c8431..42c76b41dd417109bd929587eb6196c9475eb6ef 100644
--- a/crates/nova-hir/src/lib.rs
+++ b/crates/nova-hir/src/lib.rs
@@ -242,7 +242,9 @@ impl Ty {
 
     /// The nominal head of this type, if it has one — used to key impl
     /// lookups. Generic arguments do not participate (Phase 1 has no
-    /// overlapping impls per head).
+    /// overlapping impls per head). An array's head is [`TyHead::Array`],
+    /// whatever its element type, so std can write `impl<T> [T]` (5.0 spec
+    /// §5.5); a tuple, a function and a future have none.
     pub fn head(&self) -> Option<TyHead> {
         match self {
             Ty::Int => Some(TyHead::Int),
@@ -251,6 +253,7 @@ impl Ty {
             Ty::Char => Some(TyHead::Char),
             Ty::String => Some(TyHead::String),
             Ty::Bytes => Some(TyHead::Bytes),
+            Ty::Array(_) => Some(TyHead::Array),
             Ty::Sum { def_id, .. } => Some(TyHead::Sum(*def_id)),
             Ty::Record { def_id, .. } => Some(TyHead::Record(*def_id)),
             _ => None,
@@ -348,6 +351,9 @@ pub enum TyHead {
     Char,
     String,
     Bytes,
+    /// `[T]`, for any `T`: an impl on arrays is `impl<T> [T]` or one on a
+    /// concrete element type, `impl [Int]`, told apart by `match_args`.
+    Array,
     Sum(DefId),
     Record(DefId),
 }
diff --git a/crates/nova-typeck/src/check.rs b/crates/nova-typeck/src/check.rs
index faea4ddec0b9a70461f50a6dd4c1672c9bb205de..932650da52b0e60efa971e2c1fb84b79a2e8757b 100644
--- a/crates/nova-typeck/src/check.rs
+++ b/crates/nova-typeck/src/check.rs
@@ -1134,6 +1134,14 @@ impl<'a> Checker<'a> {
         // Source span of each collected impl (aligned with `self.impls`), for
         // the post-collection coherence check.
         let mut impl_spans: Vec<Span> = Vec::new();
+        // Aligned with `impl_spans`: whether each impl is std's, and the span
+        // of each of its methods' names, in `ImplInfo::methods`' order, for
+        // `check_hidden_trait_methods`.
+        let mut impl_in_std: Vec<bool> = Vec::new();
+        let mut impl_method_spans: Vec<Vec<Span>> = Vec::new();
+        // The impl's own names for its generic parameters, `T` in
+        // `impl<T> Top for Vec<T>`, so E0105 shows `Vec<T>`, not `Vec<T0>`.
+        let mut impl_param_names: Vec<Vec<String>> = Vec::new();
 
         for (item_index, item) in self.file.items.iter().enumerate() {
             let ast::Item::Impl(block) = &item.value else {
@@ -1271,6 +1279,24 @@ impl<'a> Checker<'a> {
                 }
                 None => None,
             };
+            // An array takes inherent impls alone (5.0 spec §5.5; the user's
+            // ruling on 5.0c's first fact-check): `impl<T> [T] { … }` is
+            // std's and a program's way to add methods, while a trait impl on
+            // an array, `impl Display for [Int]`, stays E0010 as before 5.0c,
+            // when no impl could name an array.
+            if trait_id.is_some() && self_head == TyHead::Array {
+                self.diagnostics.push(
+                    Diagnostic::error("E0010", "a trait cannot be implemented for an array")
+                        .with_primary_label(block.ty.span, "an array type")
+                        .with_note(
+                            "an array takes inherent impls only, `impl<T> [T] { … }` or \
+                             `impl [Int] { … }`; wrap the array in a record to implement a \
+                             trait for it"
+                                .to_string(),
+                        ),
+                );
+                continue;
+            }
 
             // From here on, `Self` inside this impl means this impl's self type.
             // Recorded by item index too, so `check_method` can restore it when
@@ -1370,6 +1396,7 @@ impl<'a> Checker<'a> {
             }
             self.check_assoc_binding_cycles(&mut assoc_bindings, &assoc_spans, &self_ty);
             let mut methods = Vec::new();
+            let mut method_spans = Vec::new();
             for (mi, f) in block.functions.iter().enumerate() {
                 let Some(def_id) = impl_methods.get(&(item_index, mi)).copied() else {
                     continue;
@@ -1475,6 +1502,7 @@ impl<'a> Checker<'a> {
                     },
                 );
                 methods.push((f.name.value.clone(), def_id));
+                method_spans.push(f.name.span);
             }
 
             // Conformance: a trait impl must define exactly the trait's
@@ -1497,6 +1525,15 @@ impl<'a> Checker<'a> {
                 assoc_bindings,
             });
             impl_spans.push(block.ty.span);
+            impl_in_std.push(self.defs.is_std_module(self.cur_module));
+            impl_method_spans.push(method_spans);
+            impl_param_names.push(
+                block
+                    .generics
+                    .iter()
+                    .map(|g| g.name.value.clone())
+                    .collect(),
+            );
         }
         // No impl is in scope for `collect_signatures` / `collect_externs`,
         // which run next.
@@ -1506,10 +1543,166 @@ impl<'a> Checker<'a> {
         // `E0072` about a method signature still precedes `check_supertrait_impls`'s
         // own `E0072`, as it did when the signature check ran inside the loop.
         self.check_impl_method_signatures(&impl_spans);
-        self.check_impl_coherence(&impl_spans);
+        self.check_impl_coherence(&impl_spans, &impl_in_std);
+        self.check_hidden_trait_methods(
+            &impl_spans,
+            &impl_in_std,
+            &impl_method_spans,
+            &impl_param_names,
+        );
         self.check_supertrait_impls(&impl_spans);
     }
 
+    /// E0105, a warning: a program's trait impl defines (or takes from the
+    /// trait's default) a method that one of std's inherent impls on an
+    /// overlapping type also defines, so that std's is the one a call
+    /// reaches (the user's ruling on 5.0c's first fact-check). Two methods
+    /// that both take `self` meet at a method call, which resolves an
+    /// inherent method before a trait's, as in Rust: `"é".byte_len()` runs
+    /// std's `String::byte_len`, never the program's `Size::byte_len`. Two
+    /// that both lack `self` meet at a path, `Vec::from_array(a)`, which
+    /// also takes the inherent function. A pair of which only one takes
+    /// `self` never meets: a method call skips a selfless inherent function,
+    /// and a path reaches a static trait function (5.0c's second
+    /// fact-check). A std method a program can call counts, private or not:
+    /// a method's privacy is not enforced at a call. Nova has no
+    /// `Trait::method(x)` call, so the note names the two ways to reach the
+    /// trait's method.
+    fn check_hidden_trait_methods(
+        &mut self,
+        spans: &[Span],
+        in_std: &[bool],
+        method_spans: &[Vec<Span>],
+        param_names: &[Vec<String>],
+    ) {
+        let mut found: Vec<Diagnostic> = Vec::new();
+        for (i, a) in self.impls.iter().enumerate() {
+            let Some(tid) = a.trait_id else { continue };
+            if in_std.get(i).copied().unwrap_or(true) {
+                continue;
+            }
+            let Some(tr) = self.traits.iter().find(|t| t.def_id == tid) else {
+                continue;
+            };
+            // The impl's own methods, at their names, then the trait's
+            // defaults it does not override, at the impl's type; each with
+            // whether it takes `self`.
+            let mut names: Vec<(String, Span, bool)> = a
+                .methods
+                .iter()
+                .enumerate()
+                .map(|(k, (n, d))| {
+                    let span = method_spans.get(i).and_then(|s| s.get(k)).copied();
+                    (
+                        n.clone(),
+                        span.unwrap_or(spans[i]),
+                        !self.selfless.contains(d),
+                    )
+                })
+                .collect();
+            for m in &tr.methods {
+                if m.default_def.is_some() && !a.methods.iter().any(|(n, _)| *n == m.name) {
+                    names.push((m.name.clone(), spans[i], m.has_self));
+                }
+            }
+            for (j, b) in self.impls.iter().enumerate() {
+                if b.trait_id.is_some() || !in_std.get(j).copied().unwrap_or(false) {
+                    continue;
+                }
+                if a.self_head != b.self_head
+                    || !hir::self_types_overlap(&a.self_ty, a.generics, &b.self_ty, b.generics)
+                {
+                    continue;
+                }
+                for (name, span, has_self) in &names {
+                    let Some((_, std_def)) = b.methods.iter().find(|(n, _)| n == name) else {
+                        continue;
+                    };
+                    let std_has_self = !self.selfless.contains(std_def);
+                    if std_has_self != *has_self {
+                        continue;
+                    }
+                    let no_names = Vec::new();
+                    let ty = display_ty_named(
+                        &a.self_ty,
+                        self.defs,
+                        param_names.get(i).unwrap_or(&no_names),
+                    );
+                    // `(…)` where std's method takes arguments beyond `self`.
+                    let declared = self.sigs.get(std_def).map_or(0, |s| s.params.len());
+                    let args = if declared > usize::from(std_has_self) {
+                        "(…)"
+                    } else {
+                        "()"
+                    };
+                    let trait_name = &tr.name;
+                    let d = if *has_self {
+                        Diagnostic::warning(
+                            "E0105",
+                            format!(
+                                "std's inherent method `{name}` on `{ty}` hides \
+                                 `{trait_name}::{name}`"
+                            ),
+                        )
+                        .with_primary_label(
+                            *span,
+                            format!("`x.{name}{args}` on a `{ty}` runs std's method, not this one"),
+                        )
+                        .with_note(format!(
+                            "a method call takes an inherent method before a trait's; Nova has no \
+                             `{trait_name}::{name}(x)` call, so reach this one through a function \
+                             generic over `T: {trait_name}`, whose `x.{name}{args}` is the \
+                             trait's, or rename it"
+                        ))
+                    } else {
+                        let head = ty.split('<').next().unwrap_or(&ty).to_string();
+                        Diagnostic::warning(
+                            "E0105",
+                            format!(
+                                "std's inherent function `{name}` on `{ty}` hides \
+                                 `{trait_name}::{name}`"
+                            ),
+                        )
+                        .with_primary_label(
+                            *span,
+                            format!("`{head}::{name}{args}` runs std's function, not this one"),
+                        )
+                        .with_note(format!(
+                            "a path takes an inherent function before a trait's, so \
+                             `{head}::{name}{args}` is std's; reach this one through a function \
+                             generic over `T: {trait_name}`, whose `T::{name}{args}` is the \
+                             trait's, or rename it"
+                        ))
+                    };
+                    found.push(d);
+                }
+            }
+        }
+        self.diagnostics.extend(found);
+    }
+
+    /// An inherent method's name as a diagnostic shows it: an array's as
+    /// `[T]::slice` (`shown_method_name`), a generic named type's, which the
+    /// resolver names `Vec_T.at`, as `Vec::at` (5.0c's second fact-check),
+    /// and any other as the resolver names it, `P.of`.
+    fn shown_method(&self, def_id: DefId) -> String {
+        let raw = &self.defs.def(def_id).name;
+        if raw.starts_with("Arr_") {
+            return shown_method_name(raw);
+        }
+        let owner = self
+            .impls
+            .iter()
+            .find(|i| i.generics > 0 && i.methods.iter().any(|(_, d)| *d == def_id));
+        match (owner, raw.rsplit_once('.')) {
+            (Some(imp), Some((_, method))) => {
+                let ty = display_ty(&imp.self_ty, self.defs);
+                format!("{}::{method}", ty.split('<').next().unwrap_or(&ty))
+            }
+            _ => raw.clone(),
+        }
+    }
+
     /// Report `E0077` for an associated type this impl binds in terms of
     /// itself, and poison the offending binding to `Ty::Error`.
     ///
@@ -1646,10 +1839,11 @@ impl<'a> Checker<'a> {
     /// conflict outright, and two inherent impls that overlap conflict on any
     /// method they both define. Without this, dispatch would depend on impl
     /// declaration order.
-    fn check_impl_coherence(&mut self, spans: &[Span]) {
-        let mut conflicts: Vec<(String, Span)> = Vec::new();
+    fn check_impl_coherence(&mut self, spans: &[Span], in_std: &[bool]) {
+        let mut conflicts: Vec<Diagnostic> = Vec::new();
         for (i, a) in self.impls.iter().enumerate() {
-            for (b, &b_span) in self.impls.iter().zip(spans).skip(i + 1) {
+            let a_span = spans.get(i).copied();
+            for (j, (b, &b_span)) in self.impls.iter().zip(spans).enumerate().skip(i + 1) {
                 if a.self_head != b.self_head || a.trait_id != b.trait_id {
                     continue;
                 }
@@ -1664,13 +1858,16 @@ impl<'a> Checker<'a> {
                             .find(|t| t.def_id == tid)
                             .map(|t| t.name.clone())
                             .unwrap_or_else(|| "?".to_string());
-                        conflicts.push((
-                            format!(
-                                "conflicting implementations of trait `{tname}` for \
-                                 overlapping types"
-                            ),
-                            b_span,
-                        ));
+                        conflicts.push(
+                            Diagnostic::error(
+                                "E0074",
+                                format!(
+                                    "conflicting implementations of trait `{tname}` for \
+                                     overlapping types"
+                                ),
+                            )
+                            .with_primary_label(b_span, "here"),
+                        );
                     }
                     None => {
                         let dup = a
@@ -1679,22 +1876,40 @@ impl<'a> Checker<'a> {
                             .map(|(n, _)| n)
                             .find(|n| b.methods.iter().any(|(m, _)| m == *n))
                             .cloned();
+                        // Both impls are labelled: when one is std's, as
+                        // a program's `impl<T> [T] { fn iter … }` meets
+                        // std's `impl<T> [T]` (5.0 spec §5.5), the other is
+                        // the program's, which the reader must see, and is
+                        // the primary label (5.0c's second fact-check).
                         if let Some(m) = dup {
-                            conflicts.push((
+                            let std_b = in_std.get(j).copied().unwrap_or(false)
+                                && !in_std.get(i).copied().unwrap_or(false);
+                            let (here, other) = match a_span {
+                                Some(a_span) if std_b => (a_span, Some(b_span)),
+                                _ => (b_span, a_span),
+                            };
+                            let mut d = Diagnostic::error(
+                                "E0074",
                                 format!(
                                     "method `{m}` is defined by multiple overlapping \
                                      inherent impls"
                                 ),
-                                b_span,
-                            ));
+                            )
+                            .with_primary_label(here, "here");
+                            if let Some(other) = other {
+                                let by = if std_b { "std's impl" } else { "this impl" };
+                                d = d.with_secondary_label(
+                                    other,
+                                    format!("`{m}` is also defined by {by}"),
+                                );
+                            }
+                            conflicts.push(d);
                         }
                     }
                 }
             }
         }
-        for (msg, span) in conflicts {
-            self.error("E0074", msg, span);
-        }
+        self.diagnostics.extend(conflicts);
     }
 
     /// Render a set of trait bounds for a diagnostic, e.g. `` `Show + Ord` ``
@@ -3695,11 +3910,18 @@ impl<'a> Checker<'a> {
                             match self.try_display(fcx, value, e, &other) {
                                 Some(fmt_call) => pieces.push(fmt_call),
                                 None => {
+                                    // An array cannot implement `Display`: a
+                                    // trait impl on one is E0010 (5.0c).
+                                    let advice = if matches!(other, Ty::Array(_)) {
+                                        "an array cannot implement `Display`; interpolate \
+                                         its elements, or wrap it in a record that does"
+                                    } else {
+                                        "implement `Display` (fmt(self) -> String) for it"
+                                    };
                                     self.error(
                                         "E0013",
                                         format!(
-                                            "`{}` cannot be interpolated into a string; \
-                                             implement `Display` (fmt(self) -> String) for it",
+                                            "`{}` cannot be interpolated into a string; {advice}",
                                             self.show(&other, fcx),
                                         ),
                                         // value was moved into try_display on the
@@ -5761,7 +5983,7 @@ impl<'a> Checker<'a> {
     /// - its inherent methods that take `self`;
     /// - the methods of the traits it implements;
     /// - for a type parameter, its bounds' methods;
-    /// - for an array, `len`.
+    /// - for an array, `len`, then the methods of its impls (5.0 spec §5.5).
     ///
     /// A field or inherent method declared without `pub` is listed only
     /// inside its own module. The first of a name wins, so an inherent method
@@ -5781,15 +6003,17 @@ impl<'a> Checker<'a> {
                     }
                 }
             }
-            Ty::Array(_) => push(
-                &mut out,
-                Member {
-                    name: "len".to_string(),
-                    kind: MemberKind::Method,
-                    decl: None,
-                },
-            ),
             _ => {
+                if let Ty::Array(_) = recv_ty {
+                    push(
+                        &mut out,
+                        Member {
+                            name: "len".to_string(),
+                            kind: MemberKind::Method,
+                            decl: None,
+                        },
+                    );
+                }
                 if let Ty::Record { def_id, .. } = recv_ty {
                     for m in self.record_members(*def_id, from) {
                         push(&mut out, m);
@@ -6322,7 +6546,9 @@ impl<'a> Checker<'a> {
             );
             return error_expr(span);
         }
-        // Built-in array methods.
+        // An array's built-in `len`, then the methods of its impls, std's
+        // `impl<T> [T]` among them (5.0 spec §5.5), looked up by
+        // `TyHead::Array` as any other type's are.
         if matches!(recv_ty, Ty::Array(_)) {
             if method.value == "len" && args.is_empty() {
                 self.note_use(method.span, Target::BuiltinMethod("len"));
@@ -6334,17 +6560,23 @@ impl<'a> Checker<'a> {
                     span,
                 };
             }
-            self.error(
-                "E0014",
-                format!(
-                    "no method `{}` on array type `{}`",
-                    method.value,
-                    self.show(&recv_ty, fcx)
-                ),
-                method.span,
-            );
-            self.push_fix(suggest::change_to(method, ["len"]));
-            return error_expr(span);
+            if matches!(
+                self.resolve_method_on(&recv_ty, fcx, &method.value),
+                MethodRes::None
+            ) {
+                self.error(
+                    "E0014",
+                    format!(
+                        "no method `{}` on array type `{}`",
+                        method.value,
+                        self.show(&recv_ty, fcx)
+                    ),
+                    method.span,
+                );
+                let fix = self.method_fix(&recv_ty, fcx, method);
+                self.push_fix(fix);
+                return error_expr(span);
+            }
         }
         match self.resolve_method_on(&recv_ty, fcx, &method.value) {
             MethodRes::Inherent(def_id) => {
@@ -6409,7 +6641,7 @@ impl<'a> Checker<'a> {
         if !self.mut_self.contains(&def_id) {
             return;
         }
-        let mname = self.defs.def(def_id).name.clone();
+        let mname = self.shown_method(def_id);
         self.require_mutable_place(fcx, receiver_ast, span, MutTarget::Receiver(mname));
     }
 
@@ -6426,7 +6658,7 @@ impl<'a> Checker<'a> {
         // would both be off by one. `find_inherent_method` already skips these —
         // this guard keeps any future caller from reintroducing the codegen ICE.
         if self.selfless.contains(&def_id) {
-            let mname = self.defs.def(def_id).name.clone();
+            let mname = self.shown_method(def_id);
             self.error(
                 "E0014",
                 format!(
@@ -6446,7 +6678,7 @@ impl<'a> Checker<'a> {
             // One wording for every arity error in this family (see also
             // `emit_assoc_call`, `emit_trait_call`, and the free-function call in
             // `check_call`), so the same mistake always reads the same way.
-            let mname = self.defs.def(def_id).name.clone();
+            let mname = self.shown_method(def_id);
             self.error(
                 "E0016",
                 format!(
@@ -6467,7 +6699,7 @@ impl<'a> Checker<'a> {
         // sharing a head is not enough, and a silent mismatch would dispatch to
         // a wrongly specialized method.
         if !fcx.icx.unify(&receiver.ty, &self_param) {
-            let mname = self.defs.def(def_id).name.clone();
+            let mname = self.shown_method(def_id);
             self.error(
                 "E0014",
                 format!(
@@ -8662,6 +8894,24 @@ fn error_expr(span: Span) -> hir::Expr {
     }
 }
 
+/// An array method's name as a diagnostic shows it. The resolver names an
+/// impl's method after its self type with `_` separators; an array's,
+/// `Arr_T.slice`, is shown as the source writes the type, `[T]::slice`
+/// (5.0c's first fact-check). `Checker::shown_method` also turns a generic
+/// named type's, `Vec_T.at`, into `Vec::at`; a plain one's, `P.of`, stays.
+pub(crate) fn shown_method_name(raw: &str) -> String {
+    fn array(ty: &str) -> String {
+        match ty.strip_prefix("Arr_") {
+            Some(elem) => format!("[{}]", array(elem)),
+            None => ty.to_string(),
+        }
+    }
+    match raw.rsplit_once('.') {
+        Some((ty, method)) if ty.starts_with("Arr_") => format!("{}::{method}", array(ty)),
+        _ => raw.to_string(),
+    }
+}
+
 /// Every builtin's parameter types and result type — the whole of what
 /// `check_builtin_call` needs to know about one, so that arity and argument
 /// checking is a single shared code path rather than an arm per builtin.
diff --git a/crates/nova-typeck/src/check/fixes.rs b/crates/nova-typeck/src/check/fixes.rs
index 3a20b6c015e7998a1d301c9b8d2f1a6251ce2a2d..7b79eb36e18302e15859f59764d67d3697a4d346 100644
--- a/crates/nova-typeck/src/check/fixes.rs
+++ b/crates/nova-typeck/src/check/fixes.rs
@@ -401,6 +401,11 @@ impl<'a> Checker<'a> {
                 MethodRes::Inherent(_) | MethodRes::Trait(..)
             )
         });
+        // An array's `len` is built in, not a method of an impl.
+        if matches!(recv, Ty::Array(_)) {
+            names.push("len".to_string());
+            names.sort();
+        }
         suggest::change_to(method, names.iter().map(String::as_str))
     }
 
diff --git a/std/collections/lib.nova b/std/collections/lib.nova
index b52c00dfb5316f49cc8d4697d539a8ab245ca16a..c22ea0b2a228680e43d2c4205e6972b225b3bf8f 100644
--- a/std/collections/lib.nova
+++ b/std/collections/lib.nova
@@ -145,6 +145,82 @@ impl<T> Vec<T> {
     /// changes made while it runs: a value pushed onto the vector is yielded
     /// too, even one pushed after the iterator has returned `None`.
     pub fn iter(self) -> VecIter<T> { VecIter { v: self, i: 0 } }
+
+    // `at` and `last` read a value with no `Option`, as `a[i]` does, and panic
+    // where `get` and `pop` give `None`: a compiler's passes read their
+    // stacks and lists in hot loops, and `get` allocates a `Some` per read
+    // (5.0 spec §13). Both check against `len`, not the storage's length,
+    // which is longer.
+    /// Returns the value at index `i`, without an `Option`.
+    ///
+    /// Panics if `i` is negative or not less than `len()`, as indexing an
+    /// array does; `get` returns `None` there instead.
+    ///
+    /// ```
+    /// let mut v: Vec<String> = Vec::new()
+    /// v.push("a")
+    /// v.push("b")
+    /// assert_eq(v.at(1), "b")
+    /// ```
+    pub fn at(self, i: Int) -> T {
+        if i < 0 || i >= self.len { panic("Vec::at index out of range") }
+        self.data[i]
+    }
+
+    /// Returns the last value, without an `Option` or removing it.
+    ///
+    /// Panics if the vector is empty; `pop` returns `None` there instead.
+    ///
+    /// ```
+    /// let mut stack: Vec<Int> = Vec::new()
+    /// stack.push(1)
+    /// stack.push(2)
+    /// assert_eq(stack.last(), 2)
+    /// assert_eq(stack.len(), 2)
+    /// ```
+    pub fn last(self) -> T {
+        if self.len == 0 { panic("Vec::last on an empty vector") }
+        self.data[self.len - 1]
+    }
+
+    // A copy, not the storage: `data` is longer than `len`, and handing it out
+    // would let a write through the array change the vector (5.0 spec §12
+    // item 24). The first value fills the new array, every slot of which is
+    // then overwritten, as `push`'s growth does.
+    /// Returns a new array of the vector's values, in order.
+    ///
+    /// The array is a copy: a change to either afterwards is not seen in the
+    /// other.
+    ///
+    /// ```
+    /// let mut v: Vec<Int> = Vec::new()
+    /// v.push(3)
+    /// v.push(4)
+    /// let a = v.to_array()
+    /// assert_eq(a.len(), 2)
+    /// assert_eq(a[1], 4)
+    /// v.push(5)
+    /// assert_eq(a.len(), 2) // a copy
+    /// ```
+    pub fn to_array(self) -> [T] {
+        if self.len == 0 { return [] }
+        let mut out = [self.data[0]; self.len]
+        for i in 1..self.len { out[i] = self.data[i] }
+        out
+    }
+
+    /// Returns a new vector of the values of `a`, in order.
+    ///
+    /// The vector copies the array, so a change to either afterwards is not
+    /// seen in the other.
+    ///
+    /// ```
+    /// let mut v = Vec::from_array([1, 2, 3])
+    /// v.push(4)
+    /// assert_eq(v.len(), 4)
+    /// assert_eq(v.at(0), 1)
+    /// ```
+    pub fn from_array(a: [T]) -> Vec<T> { Vec { len: a.len(), data: a.slice(0, a.len()) } }
 }
 
 // A cursor over a `Vec<T>`: the vector, plus the next index to yield.
@@ -197,6 +273,76 @@ impl<T> Iterator for VecIter<T> {
     }
 }
 
+// Methods on every array, `[T]` (5.0 spec §5.5). An array's `len` is built
+// into the compiler; these are looked up after it, by the array's type head,
+// as a `String`'s methods are. Both copy nothing they do not return: `iter`
+// shares the array, and `slice` copies only its range.
+/// Methods on arrays: `iter` walks one in order, and `slice` copies a range
+/// of it.
+///
+/// ```
+/// let a = [10, 20, 30, 40]
+/// let doubled = a.iter().map(|x| x * 2).collect()
+/// assert_eq(doubled.at(3), 80)
+/// assert_eq(a.slice(1, 3).len(), 2)
+/// ```
+impl<T> [T] {
+    /// Returns an iterator over the array's values, in order.
+    ///
+    /// It shares the array rather than copying it, so a value written into
+    /// the array before the iterator reaches it is the one yielded.
+    ///
+    /// ```
+    /// let mut total = 0
+    /// for x in [1, 2, 3].iter() { total = total + x }
+    /// assert_eq(total, 6)
+    /// ```
+    pub fn iter(self) -> ArrayIter<T> { ArrayIter { a: self, i: 0 } }
+
+    // Panics on a bad range, as `String::slice` does: a range that must be
+    // valid is the caller's bug.
+    /// Returns a new array of the values from index `start` up to, but not
+    /// including, `end`.
+    ///
+    /// `start == end` gives an empty array. Panics if `start` is negative, if
+    /// `end` is greater than `len()`, or if `start` is greater than `end`.
+    ///
+    /// ```
+    /// let a = ['a', 'b', 'c', 'd']
+    /// let mid = a.slice(1, 3)
+    /// assert_eq(mid.len(), 2)
+    /// assert_eq(mid[0], 'b')
+    /// assert_eq(a.slice(4, 4).len(), 0)
+    /// ```
+    pub fn slice(self, start: Int, end: Int) -> [T] {
+        if start < 0 { panic("[T]::slice start is negative") }
+        if end > self.len() { panic("[T]::slice end is past the end of the array") }
+        if start > end { panic("[T]::slice start is after end") }
+        let n = end - start
+        if n == 0 { return [] }
+        let mut out = [self[start]; n]
+        for i in 1..n { out[i] = self[start + i] }
+        out
+    }
+}
+
+// A cursor over an array: the array, shared, and the next index to yield.
+/// An iterator over an array's values, in order.
+///
+/// `[T]::iter` returns one.
+pub record ArrayIter<T> { a: [T], i: Int }
+
+impl<T> Iterator for ArrayIter<T> {
+    type Item = T
+
+    fn next(mut self) -> Option<T> {
+        if self.i >= self.a.len() { return None }
+        let x = self.a[self.i]
+        self.i = self.i + 1
+        Some(x)
+    }
+}
+
 // A hash map with open addressing and linear probing.
 //
 // Capacity is a power of two, so a bucket index is `hash & (cap - 1)` — never
````

### Task 2: `Char` and `Int`, and a `String`'s bytes

Spec §5.2; decisions 5, 7, 11, 19, 28, 30, 31, 40 and 41.

**Files:**
- Modify:
  - `crates/nova-resolver/src/lib.rs` (`IntToChar`, `StrAsBytes`: the enum,
    `name()`, `STD_ONLY`, 90 to 92)
  - `crates/nova-typeck/src/check.rs` (the two in the builtin list and
    `builtin_signature`), `crates/nova-typeck/src/check/fixes.rs`
    (`STD_ONLY_PUBLIC_FORMS`, the note in `name_fixes`)
  - `crates/nova-mir/src/lower.rs` (both `Lowering::Reinterpret`)
  - `std/core/lib.nova` (`Char.to_int`, `char_from_int`),
    `std/strings/lib.nova` (`continues_char`, `byte_len`, `byte_at`,
    `byte_slice`, `string_from_chars`)
- Create: `tests/runtime/char_and_bytes.nova`, `char_and_bytes.stdout`
- Test:
  - `std_tests.rs` (four), the checker's
    `builtin_signatures_are_what_the_std_call_sites_use` (two rows)
  - `lower_tests.rs` (two, and two helpers)
  - `run_tests.rs` (`char_and_bytes_run`, and the panic table's byte rows),
    `wasm_tests.rs` (`gate::char_and_bytes`, the same rows),
    `index_complete.rs` (the counts)

- [ ] **Step 1: The tests.** `patch_block t2-tests`. The patch holds:
  - `std_tests.rs`: the builtins kept from user code, `int_to_char` and
    `str_as_bytes` here, E0001 for a program that names one (Tasks 4 and 5
    add their rows); the conversions' and byte access's types; and a
    std-only builtin named by a program, E0001 with a note naming its public
    form and no fix, for `char_to_int`, `int_to_char`, `str_from_chars` and
    `str_as_bytes` here (decision 19); and E0105's rows for Task 2's
    methods, Task 1 having the check: a program's `Size::byte_len` on
    `String` and the default `Codes::to_int` on `Char` each draw it, at the
    impl's method or the impl (decision 40).
  - The signature test's rows for `int_to_char` and `str_as_bytes`.
  - `lower_tests.rs`: `byte_len` is a move and `Bytes`' length, one runtime
    call; `char_from_int` checks the range and moves, no call.
  - **The program** `char_and_bytes`: every edge of the scalar values both
    ways, `to_int` of one- to four-byte characters, `string_from_chars`,
    each character's bytes, and slices at every boundary, with its
    `.stdout`, `run_tests.rs`'s test under `run`, `build` and GC stress, its
    `gate!` entry, and the counts, 178 programs and 212 files.
  - The panic tables' byte rows: `byte_at` past each end, and `byte_slice`'s
    five refusals.
- [ ] **Step 2: Run, and see the tests fail.** First the build, which ends
  with `Finished`.
  - `T check::std_tests check::tests::builtin_signatures_are_what_the_std_call_sites_use`.
    Expected: it does not compile,
    ``error[E0599]: no variant, associated function, or constant named `IntToChar` found for enum `nova_resolver::Builtin` in the current scope``,
    the same for `StrAsBytes`, and
    ``error: could not compile `nova-typeck` (lib test) due to 2 previous errors``
    (decision 32).
  - `L byte_len char_from_int`. Expected: `0 passed; 2 failed`,
    `byte_len_lowers_to_a_move_and_the_bytes_length` and
    `char_from_int_checks_the_range_and_moves`, their programs not checking.
  - `R char_and_bytes_run each_std_panic_names_its_method`. Expected:
    `0 passed; 2 failed`: `char_and_bytes` refused with E0001 "cannot find
    function `char_from_int` in this scope" and E0014 "no method `byte_len`
    on type `String`", "no method `to_int` on type `Char`" and the like; the
    panic test at its first byte row, `"ab".byte_at(2)`, "no method
    `byte_at` on type `String`".
  - `W gate::char_and_bytes each_std_panic_names_its_method_as_natively every_runtime_program_is_in_the_gate_or_left_out`.
    Expected: `1 passed; 2 failed`, the same.
  - `X`. Expected: `0 passed; 1 failed`, `char_and_bytes.nova` not checking
    cleanly.
- [ ] **Step 3: The code.** `patch_block t2-code`:
  - **The resolver:** `IntToChar` and `StrAsBytes`, each documented, in the
    enum, `name()` and `STD_ONLY`, now 92.
  - **`check.rs`:** both std-only builtins, `int_to_char(Int) -> Char` and
    `str_as_bytes(String) -> Bytes`. **`fixes.rs`:**
    `STD_ONLY_PUBLIC_FORMS`, and `name_fixes` giving a name in it its note
    in place of the import and "did you mean" fixes.
  - **MIR:** both lower to `Lowering::Reinterpret`, a `Copy`, as
    `char_to_int` does; the comments say why.
  - **`std/core`:** `Char.to_int` over `char_to_int`; `char_from_int`, the
    range checked in Nova, then `int_to_char`.
  - **`std/strings`:** `continues_char`; `byte_len`, `byte_at` and
    `byte_slice` over `str_as_bytes` and `Bytes`' builtins, the last two
    panicking with their method's name; `string_from_chars` over
    `str_from_chars`. Each with a doc test.
- [ ] **Step 4: Run.** First the build, which ends with `Finished`.
  - `T check::std_tests check::tests::builtin_signatures_are_what_the_std_call_sites_use`.
    Expected: every `test result` `ok`, 19 passing in all, `nova-typeck`'s:
    `std_tests.rs`' 18 and the signature test.
  - `L byte_len char_from_int`. Expected: `2 passed; 0 failed`.
  - `R char_and_bytes_run each_std_panic_names_its_method`. Expected:
    `2 passed; 0 failed`.
  - `W gate::char_and_bytes each_std_panic_names_its_method_as_natively every_runtime_program_is_in_the_gate_or_left_out`.
    Expected: `3 passed; 0 failed`.
  - `X`. Expected: `1 passed; 0 failed`.
  - `S`, as in Task 1 (about 5 min 25 s). Expected: `2744 1 10`: Task 1's
    figures and Task 2's 8 tests, the one failure the IPv6 test.
  - `G`. Expected:
    `release golden: 140 passed, 9 excluded, Ubuntu clang version 18.1.3 (1ubuntu1)`
    and `1 passed`.
- [ ] **Step 5: The checks,** as the conventions list them.
- [ ] **Step 6: Commit** `std: Char and Int conversions, and a String's bytes`.

**Patch `t2-tests`:**

<!-- patch t2-tests -->

````diff
diff --git a/crates/nova-cli/tests/run_tests.rs b/crates/nova-cli/tests/run_tests.rs
index a6dbc20b1346c74db1012356c60d2988d4c390ea..66359eba26bb8fef97708dbefa7ea732adce340a 100644
--- a/crates/nova-cli/tests/run_tests.rs
+++ b/crates/nova-cli/tests/run_tests.rs
@@ -13040,6 +13040,13 @@ fn for_arrays_run() {
     golden_three_ways("for_arrays", true);
 }
 
+/// `Char` and `Int` both ways, `string_from_chars`, and byte access on one-
+/// to four-byte characters (5.0 spec §5.2), also under GC stress.
+#[test]
+fn char_and_bytes_run() {
+    golden_three_ways("char_and_bytes", true);
+}
+
 /// `iter` and `slice` on arrays, and `Vec`'s `to_array`, `from_array`, `at`
 /// and `last` (5.0 spec §5.5), also under GC stress.
 #[test]
@@ -13058,6 +13065,28 @@ fn hidden_trait_methods_run() {
 /// Each panic of 5.0c's std (5.0 spec §5): the expression, and the
 /// message after `nova: panic: `. `wasm_tests.rs` has the same table.
 const STD_PANICS: &[(&str, &str)] = &[
+    ("\"ab\".byte_at(2)", "String::byte_at index out of range"),
+    ("\"ab\".byte_at(-1)", "String::byte_at index out of range"),
+    (
+        "\"ab\".byte_slice(-1, 1)",
+        "String::byte_slice start is negative",
+    ),
+    (
+        "\"ab\".byte_slice(0, 3)",
+        "String::byte_slice end is past the end of the string",
+    ),
+    (
+        "\"ab\".byte_slice(2, 1)",
+        "String::byte_slice start is after end",
+    ),
+    (
+        "\"é\".byte_slice(1, 2)",
+        "String::byte_slice start is inside a character",
+    ),
+    (
+        "\"é\".byte_slice(0, 1)",
+        "String::byte_slice end is inside a character",
+    ),
     ("[1, 2].slice(-1, 1)", "[T]::slice start is negative"),
     (
         "[1, 2].slice(0, 3)",
diff --git a/crates/nova-cli/tests/wasm_tests.rs b/crates/nova-cli/tests/wasm_tests.rs
index 3ddcccea919049e36dd0023382c54d4b16030d99..df439e9d08c0d4e6ecfb81749f4fd2ea63cb7986 100644
--- a/crates/nova-cli/tests/wasm_tests.rs
+++ b/crates/nova-cli/tests/wasm_tests.rs
@@ -1165,6 +1165,28 @@ fn an_index_out_of_bounds_panics_as_natively() {
 fn each_std_panic_names_its_method_as_natively() {
     let Some(_) = node() else { return };
     let cases: &[(&str, &str)] = &[
+        ("\"ab\".byte_at(2)", "String::byte_at index out of range"),
+        ("\"ab\".byte_at(-1)", "String::byte_at index out of range"),
+        (
+            "\"ab\".byte_slice(-1, 1)",
+            "String::byte_slice start is negative",
+        ),
+        (
+            "\"ab\".byte_slice(0, 3)",
+            "String::byte_slice end is past the end of the string",
+        ),
+        (
+            "\"ab\".byte_slice(2, 1)",
+            "String::byte_slice start is after end",
+        ),
+        (
+            "\"é\".byte_slice(1, 2)",
+            "String::byte_slice start is inside a character",
+        ),
+        (
+            "\"é\".byte_slice(0, 1)",
+            "String::byte_slice end is inside a character",
+        ),
         ("[1, 2].slice(-1, 1)", "[T]::slice start is negative"),
         (
             "[1, 2].slice(0, 3)",
@@ -2014,6 +2036,7 @@ mod gate {
         // over a builtin that is a shim entry on WASM.
         array_helpers,
         hidden_trait_methods,
+        char_and_bytes,
     );
 
     /// The programs a test of their own in this module checks, by its name.
diff --git a/crates/nova-driver/tests/index_complete.rs b/crates/nova-driver/tests/index_complete.rs
index bdcaa39411f2897468f2a14f1d0ee9a74d9d853b..dab2da366be0b9fdf0f6e466975018b24042418c 100644
--- a/crates/nova-driver/tests/index_complete.rs
+++ b/crates/nova-driver/tests/index_complete.rs
@@ -249,8 +249,9 @@ fn every_name_in_the_corpora_is_indexed() {
     // `try_async`, `compound_assign_places` and `string_concat`; 161 on
     // 2026-10-27, after 5.0b added `tuples` and `tuples_async`; 165 after
     // `patterns`, `patterns_async`, `match_effects` and `for_arrays`; 166
-    // after 5.0c added `array_helpers`; 167 after `hidden_trait_methods`.
-    assert_eq!(programs.len(), 177, "{programs:#?}");
+    // after 5.0c added `array_helpers`; 167 after `hidden_trait_methods`;
+    // 168 after `char_and_bytes`.
+    assert_eq!(programs.len(), 178, "{programs:#?}");
     let mut files = 0;
     let mut failures: Vec<String> = Vec::new();
     for (k, path) in programs.iter().enumerate() {
@@ -277,11 +278,11 @@ fn every_name_in_the_corpora_is_indexed() {
             every_name_is_covered(&a, file, &mut failures);
         }
     }
-    // 9 + 167 + 3 files of user code, 05-geometry's library's 2, the 8
+    // 9 + 168 + 3 files of user code, 05-geometry's library's 2, the 8
     // modules 07-fullstack-blog's client reaches from its `main.nova` (not
     // the server's 4, which only `server.nova` and `lib.nova` reach), and
     // native std's 22, `std/ui/ssr` among them.
-    assert_eq!(files, 211);
+    assert_eq!(files, 212);
     failures.sort();
     failures.dedup();
     assert!(
diff --git a/crates/nova-mir/tests/lower_tests.rs b/crates/nova-mir/tests/lower_tests.rs
index 7d95c1babcd94a7802a8f0a7ff781ca0a76d0a61..1dc8dd08ef8fcafc37c4cc9f19ebbecd7caf09d8 100644
--- a/crates/nova-mir/tests/lower_tests.rs
+++ b/crates/nova-mir/tests/lower_tests.rs
@@ -2374,6 +2374,66 @@ fn a_flat_char_match_switches_on_its_code_points() {
     assert_eq!(keys, ['a' as i64, 'b' as i64]);
 }
 
+/// The instance of a std function whose name holds `part`, in `mir`.
+fn std_fn(mir: &nova_mir::Module, part: &str) -> String {
+    let names = function_names(mir);
+    names
+        .iter()
+        .find(|n| n.contains(part))
+        .unwrap_or_else(|| panic!("no function with `{part}`: {names:?}"))
+        .to_string()
+}
+
+/// How many `CallRuntime`s and `Copy`s the function `name` makes.
+fn calls_and_copies(mir: &nova_mir::Module, name: &str) -> (usize, usize) {
+    let stmts = stmts_of(mir, name);
+    let calls = stmts
+        .iter()
+        .filter(|s| matches!(s, nova_mir::Stmt::CallRuntime { .. }))
+        .count();
+    let copies = stmts
+        .iter()
+        .filter(|s| matches!(s, nova_mir::Stmt::Copy { .. }))
+        .count();
+    (calls, copies)
+}
+
+/// 5.0c's byte view (spec §5.2): `str_as_bytes`, in `String.byte_len`, is a
+/// register move, a `Copy`, so `byte_len`'s one runtime call is `Bytes`'
+/// own length, read on the view.
+#[test]
+fn byte_len_lowers_to_a_move_and_the_bytes_length() {
+    let mir = mir_for("fn main() {\n    let n = \"abc\".byte_len()\n    println(\"${n}\")\n}");
+    let name = std_fn(&mir, "byte_len");
+    let (calls, copies) = calls_and_copies(&mir, &name);
+    assert_eq!(calls, 1, "`{name}`: {:?}", stmts_of(&mir, &name));
+    assert!(
+        copies > 0,
+        "`{name}` has no move: {:?}",
+        stmts_of(&mir, &name)
+    );
+    assert_eq!(
+        count_rt(&stmts_of(&mir, &name), nova_mir::RtFunc::BytesLen),
+        1
+    );
+}
+
+/// `int_to_char`, in `char_from_int`, is a register move too (spec §5.2):
+/// `char_from_int` checks the range and moves, and calls nothing.
+#[test]
+fn char_from_int_checks_the_range_and_moves() {
+    let mir =
+        mir_for("fn main() {\n    let c = char_from_int(65)\n    println(\"${c.is_some()}\")\n}");
+    let name = std_fn(&mir, "char_from_int");
+    let (calls, copies) = calls_and_copies(&mir, &name);
+    assert_eq!(calls, 0, "`{name}`: {:?}", stmts_of(&mir, &name));
+    assert!(
+        copies > 0,
+        "`{name}` has no move: {:?}",
+        stmts_of(&mir, &name)
+    );
+}
+
 /// `impl<T> [T]`'s methods monomorphise per element type, as any generic
 /// impl's do (spec §5.5).
 #[test]
diff --git a/crates/nova-typeck/src/check.rs b/crates/nova-typeck/src/check.rs
index 932650da52b0e60efa971e2c1fb84b79a2e8757b..a52e43bba85a345f006eb07d5dcb237cb8e23352 100644
--- a/crates/nova-typeck/src/check.rs
+++ b/crates/nova-typeck/src/check.rs
@@ -17092,6 +17092,14 @@ mod tests {
                     (vec![Ty::String], Ty::Float),
                     "`str_to_float(span)` in `std/json`'s `scan_number`",
                 ),
+                Builtin::IntToChar => (
+                    (vec![Ty::Int], Ty::Char),
+                    "`int_to_char(n)` in `char_from_int`",
+                ),
+                Builtin::StrAsBytes => (
+                    (vec![Ty::String], Ty::Bytes),
+                    "`str_as_bytes(self)` in `String::byte_at`",
+                ),
                 Builtin::TestSelector => (
                     (vec![], Ty::Int),
                     "the synthesized `main`'s dispatch, reading `NOVA_TEST_INDEX`",
diff --git a/crates/nova-typeck/src/check/std_tests.rs b/crates/nova-typeck/src/check/std_tests.rs
index cf30caaaeaf195e71ca38eae59300918f9820c2f..130cb4c1242bb2b0fe5a560dfe8aeba32ddc0af8 100644
--- a/crates/nova-typeck/src/check/std_tests.rs
+++ b/crates/nova-typeck/src/check/std_tests.rs
@@ -4,6 +4,8 @@
 //! - `TyHead::Array`, `[T]`'s impls and `Vec`'s helpers, and the E0010,
 //!   E0014, E0060 and E0074 an impl on arrays meets;
 //! - E0105, a program's trait method one of std's inherent methods hides;
+//! - the std-only builtins kept from user code, and the public form a
+//!   program that names one is pointed to;
 //! - the new items' types;
 
 use super::*;
@@ -146,6 +148,28 @@ fn an_unknown_array_method_is_e0014_with_its_methods_offered() {
     );
 }
 
+/// The std-only builtins std/core and std/strings gain are std's own: a
+/// program that names one finds nothing (spec §5.2 to §5.4).
+#[test]
+fn the_new_builtins_are_not_visible_to_a_program() {
+    for (call, name) in [
+        ("int_to_char(65)", "int_to_char"),
+        ("str_as_bytes(\"a\")", "str_as_bytes"),
+    ] {
+        let e = errors(&format!("fn main() {{\n    let x = {call}\n}}\n"));
+        assert!(
+            e.iter()
+                .any(|d| d.code == "E0001" && d.message.contains(&format!("`{name}`"))),
+            "{name}: {e:#?}"
+        );
+    }
+}
+
+/// `option(t, inner)`: whether `t` is an `Option` of `inner`.
+fn option(t: &Ty, inner: Ty) -> bool {
+    matches!(t, Ty::Sum { args, .. } if args.as_slice() == [inner])
+}
+
 /// `Vec`'s helpers' types (spec §5.5).
 #[test]
 fn vecs_helpers_have_the_specs_types() {
@@ -159,6 +183,24 @@ fn vecs_helpers_have_the_specs_types() {
     assert_eq!(local_ty(&r, "last"), Ty::Int);
 }
 
+/// The conversions' and byte access's types (spec §5.2).
+#[test]
+fn the_conversions_and_byte_access_have_the_specs_types() {
+    let r = checked(
+        "fn main() {\n    let c = char_from_int(65)\n    let i = 'a'.to_int()\n    \
+         let s = string_from_chars(['a'])\n    let ba = \"a\".byte_at(0)\n    \
+         let bs = \"ab\".byte_slice(0, 1)\n}\n",
+    );
+    assert!(r.diagnostics.is_empty(), "{:#?}", r.diagnostics);
+    assert!(option(&local_ty(&r, "c"), Ty::Char));
+    for name in ["i", "ba"] {
+        assert_eq!(local_ty(&r, name), Ty::Int, "{name}");
+    }
+    for name in ["s", "bs"] {
+        assert_eq!(local_ty(&r, name), Ty::String, "{name}");
+    }
+}
+
 /// A program's impl that defines a method std's impl on the same type has,
 /// `iter` on arrays, is E0074 labelled at the program's impl, its primary
 /// label, and at std's, which the program cannot change (5.0 spec §5.5;
@@ -183,6 +225,31 @@ fn e0074_against_stds_array_impl_labels_the_programs_impl() {
     );
 }
 
+/// A std-only builtin that std's public surface wraps, named by a program,
+/// is E0001 with a note naming that surface, and no "did you mean": its
+/// nearest name can be another operation, `char_from_int` for
+/// `char_to_int` (5.0 spec §5.2 to §5.4).
+#[test]
+fn a_std_only_builtin_named_by_a_program_gets_its_public_form() {
+    for (call, form) in [
+        ("char_to_int('a')", "`c.to_int()`"),
+        ("int_to_char(65)", "`char_from_int(n)`"),
+        ("str_from_chars(['a'])", "`string_from_chars(cs)`"),
+        ("str_as_bytes(\"a\")", "`byte_at(i)`"),
+    ] {
+        let e = errors(&format!("fn main() {{\n    let x = {call}\n}}\n"));
+        let d = e.iter().find(|d| d.code == "E0001").expect("an E0001");
+        assert!(d.fixes.is_empty(), "{call}: {:#?}", d.fixes);
+        assert!(
+            d.notes
+                .iter()
+                .any(|n| n.contains("is internal to std; use") && n.contains(form)),
+            "{call}: {:#?}",
+            d.notes
+        );
+    }
+}
+
 fn warnings(src: &str) -> Vec<Diagnostic> {
     checked(src)
         .diagnostics
@@ -365,3 +432,22 @@ fn an_interpolated_array_is_e0013_without_the_display_advice() {
         "{d:#?}"
     );
 }
+
+/// `String`'s `byte_len` and `Char`'s `to_int`, std's since Task 2, hide a
+/// program's trait methods of those names, a trait default included: E0105
+/// at the method, or at the impl for a default it takes.
+#[test]
+fn a_trait_method_hidden_by_strings_and_chars_methods_is_e0105() {
+    let src = "trait Size {\n    fn byte_len(self) -> Int\n}\n\
+               impl Size for String {\n    fn byte_len(self) -> Int { self.len() }\n}\n\
+               trait Codes {\n    fn to_int(self) -> Int { 1000 }\n}\n\
+               impl Codes for Char {}\n\
+               fn main() {}\n";
+    assert_eq!(
+        e0105s(src),
+        [
+            "std's inherent method `byte_len` on `String` hides `Size::byte_len`",
+            "std's inherent method `to_int` on `Char` hides `Codes::to_int`",
+        ]
+    );
+}
diff --git a/tests/runtime/char_and_bytes.nova b/tests/runtime/char_and_bytes.nova
new file mode 100644
index 0000000000000000000000000000000000000000..d8aba617aa7daefd44872366ae307523c61110aa
--- /dev/null
+++ b/tests/runtime/char_and_bytes.nova
@@ -0,0 +1,80 @@
+// `Char` and `Int` both ways at every edge of the scalar values,
+// `string_from_chars`, and a `String`'s bytes in constant time on one-, two-,
+// three- and four-byte characters (5.0 spec §5.2).
+
+fn show(n: Int) {
+    match char_from_int(n) {
+        Some(c) => println("${n}: Some(${c.to_int()})")
+        None => println("${n}: None")
+    }
+}
+
+fn hex(b: Int) -> String {
+    let digits = "0123456789ABCDEF".chars()
+    "${digits[b / 16]}${digits[b % 16]}"
+}
+
+fn bytes_of(s: String) -> String {
+    let mut out = ""
+    let mut i = 0
+    while i < s.byte_len() {
+        if i > 0 { out = out + " " }
+        out = out + hex(s.byte_at(i))
+        i += 1
+    }
+    out
+}
+
+fn main() {
+    for n in [
+        0 - 1,
+        0,
+        1,
+        65,
+        127,
+        128,
+        0x7FF,
+        0x800,
+        0xD7FF,
+        0xD800,
+        0xDBFF,
+        0xDC00,
+        0xDFFF,
+        0xE000,
+        0xFFFF,
+        0x10000,
+        0x10FFFF,
+        0x110000,
+        -9223372036854775807 - 1,
+        9223372036854775807,
+    ] {
+        show(n)
+    }
+    println(
+        "'a' ${'a'.to_int()} '\\0' ${'\0'.to_int()} 'é' ${'é'.to_int()} '€' ${'€'.to_int()} '𝄞' ${'𝄞'.to_int()}",
+    )
+    let back = char_from_int('€'.to_int()).unwrap()
+    println("round trip ${back}")
+
+    let built = string_from_chars(['N', 'o', 'v', 'a', ' ', char_from_int(0x1F680).unwrap()])
+    println("built ${built} chars=${built.len()} bytes=${built.byte_len()}")
+    println("empty [${string_from_chars([])}]")
+    println("chars back ${string_from_chars("añ€𝄞".chars()) == "añ€𝄞"}")
+
+    for s in ["a", "ñ", "€", "𝄞", "añ€𝄞", ""] {
+        println("${s}: len=${s.len()} byte_len=${s.byte_len()} bytes=[${bytes_of(s)}]")
+    }
+
+    let text = "añ€𝄞z"
+    let starts = [0, 1, 3, 6, 10, 11]
+    let mut k = 0
+    while k + 1 < starts.len() {
+        println(
+            "slice ${starts[k]}..${starts[k + 1]} = ${text.byte_slice(starts[k], starts[k + 1])}",
+        )
+        k += 1
+    }
+    println("whole ${text.byte_slice(0, text.byte_len())}")
+    println("empty at end [${text.byte_slice(11, 11)}] at start [${text.byte_slice(0, 0)}]")
+    println("a slice is equal ${text.byte_slice(1, 6) == "ñ€"}")
+}
diff --git a/tests/runtime/char_and_bytes.stdout b/tests/runtime/char_and_bytes.stdout
new file mode 100644
index 0000000000000000000000000000000000000000..1316bd0d55b055a79c6172c4282112e6cf703b52
--- /dev/null
+++ b/tests/runtime/char_and_bytes.stdout
@@ -0,0 +1,39 @@
+-1: None
+0: Some(0)
+1: Some(1)
+65: Some(65)
+127: Some(127)
+128: Some(128)
+2047: Some(2047)
+2048: Some(2048)
+55295: Some(55295)
+55296: None
+56319: None
+56320: None
+57343: None
+57344: Some(57344)
+65535: Some(65535)
+65536: Some(65536)
+1114111: Some(1114111)
+1114112: None
+-9223372036854775808: None
+9223372036854775807: None
+'a' 97 '\0' 0 'é' 233 '€' 8364 '𝄞' 119070
+round trip €
+built Nova 🚀 chars=6 bytes=9
+empty []
+chars back true
+a: len=1 byte_len=1 bytes=[61]
+ñ: len=1 byte_len=2 bytes=[C3 B1]
+€: len=1 byte_len=3 bytes=[E2 82 AC]
+𝄞: len=1 byte_len=4 bytes=[F0 9D 84 9E]
+añ€𝄞: len=4 byte_len=10 bytes=[61 C3 B1 E2 82 AC F0 9D 84 9E]
+: len=0 byte_len=0 bytes=[]
+slice 0..1 = a
+slice 1..3 = ñ
+slice 3..6 = €
+slice 6..10 = 𝄞
+slice 10..11 = z
+whole añ€𝄞z
+empty at end [] at start []
+a slice is equal true
````

**Patch `t2-code`:**

<!-- patch t2-code -->

````diff
diff --git a/crates/nova-mir/src/lower.rs b/crates/nova-mir/src/lower.rs
index 29236c1ae8d39549128becdf092a02f524519602..0f06bc7987c0106b9324e8dd85b37bfc279f8fe7 100644
--- a/crates/nova-mir/src/lower.rs
+++ b/crates/nova-mir/src/lower.rs
@@ -20,7 +20,9 @@ use crate::{
 enum Lowering {
     /// One `CallRuntime` to this runtime function.
     Runtime(RtFunc),
-    /// The result *is* the argument, in a different `Ty` of the same `MirTy`.
+    /// The result *is* the argument, in a different `Ty` of the same `MirTy`:
+    /// `char_to_int` and `int_to_char` (`Char` and `Int` are both `I64`), and
+    /// `str_as_bytes` (`String` and `Bytes` are both `Ptr`, one layout).
     Reinterpret,
     /// Two typed field loads reaching a finished future's output slot.
     FutureOutput,
@@ -711,6 +713,8 @@ impl<'a> Lowerer<'a> {
                     Builtin::StrToLower => Lowering::Runtime(RtFunc::StrToLower),
                     Builtin::FloatFixed => Lowering::Runtime(RtFunc::FloatFixed),
                     Builtin::StrToFloat => Lowering::Runtime(RtFunc::StrToFloat),
+                    Builtin::IntToChar => Lowering::Reinterpret,
+                    Builtin::StrAsBytes => Lowering::Reinterpret,
                     Builtin::TestSelector => Lowering::Runtime(RtFunc::TestSelector),
                     Builtin::TaskSpawn => Lowering::Runtime(RtFunc::TaskSpawn),
                     Builtin::TaskIsDone => Lowering::Runtime(RtFunc::TaskIsDone),
@@ -825,7 +829,11 @@ impl<'a> Lowerer<'a> {
                     // Unicode scalar value at runtime), so the conversion is a
                     // register move. Giving it a runtime symbol would mean
                     // adding an ABI function whose body is the identity —
-                    // permanent surface area for nothing.
+                    // permanent surface area for nothing. `int_to_char` is
+                    // the same move the other way, its range checked in Nova
+                    // by `char_from_int` first; `str_as_bytes` is one between
+                    // two `MirTy::Ptr`s, `String` and `Bytes` sharing a layout
+                    // (5.0 spec §5.2).
                     //
                     // The `Copy` is not optional and must not be skipped:
                     // `dst` was allocated above and is returned below, so
diff --git a/crates/nova-resolver/src/lib.rs b/crates/nova-resolver/src/lib.rs
index afdd3223f23ee6229d86746eefe65bb70d99e788..99b6500f832153d740c3847133a1123bde4febdd 100644
--- a/crates/nova-resolver/src/lib.rs
+++ b/crates/nova-resolver/src/lib.rs
@@ -195,6 +195,22 @@ builtins! {
     /// replaced it on 2026-10-03. Std-only, so
     /// `str_to_float` is not a reserved word in user code.
     StrToFloat,
+    /// `int_to_char(n: Int) -> Char` — the `Char` whose scalar value is `n`,
+    /// the inverse of [`Builtin::CharToInt`]. A register move, as that one
+    /// is, with no runtime symbol on any target. **The caller must have
+    /// checked that `n` is a Unicode scalar value**, 0 to `0x10FFFF` without
+    /// the surrogates: `std/core`'s `char_from_int` is the only caller, and
+    /// checks it in Nova first (5.0 spec §5.2). Std-only.
+    IntToChar,
+    /// `str_as_bytes(s: String) -> Bytes` — the same object, read as `Bytes`.
+    /// `String` and `Bytes` share one layout on every target
+    /// ([`nova_hir::Ty::Bytes`]'s note), and both are immutable, so the view
+    /// copies nothing: a register move, as [`Builtin::CharToInt`] is. Backs
+    /// `std/strings`' byte access, `String.byte_len`, `byte_at` and
+    /// `byte_slice`, through the `Bytes` builtins std already has, so a
+    /// lexer reads a byte in constant time, and allocates nothing for it
+    /// (5.0 spec §5.2, §13). Std-only.
+    StrAsBytes,
     /// `test_selector() -> Int` — which `@test` to run in this process, or a
     /// negative value meaning "print the inventory instead".
     ///
@@ -1036,6 +1052,8 @@ impl Builtin {
             Builtin::StrToLower => "str_to_lower",
             Builtin::FloatFixed => "float_fixed",
             Builtin::StrToFloat => "str_to_float",
+            Builtin::IntToChar => "int_to_char",
+            Builtin::StrAsBytes => "str_as_bytes",
             Builtin::TestSelector => "test_selector",
             Builtin::TaskSpawn => "task_spawn",
             Builtin::TaskIsDone => "task_is_done",
@@ -1159,7 +1177,7 @@ impl Builtin {
     /// consecutive review rounds (see the Phase 2.2b whole-branch review),
     /// because the roster is duplicated information that only this array
     /// needs to stay exact.
-    pub const STD_ONLY: [Builtin; 90] = [
+    pub const STD_ONLY: [Builtin; 92] = [
         Builtin::StrCmp,
         Builtin::StrHash,
         Builtin::CharToInt,
@@ -1173,6 +1191,8 @@ impl Builtin {
         Builtin::StrToLower,
         Builtin::FloatFixed,
         Builtin::StrToFloat,
+        Builtin::IntToChar,
+        Builtin::StrAsBytes,
         Builtin::TestSelector,
         Builtin::TaskSpawn,
         Builtin::TaskIsDone,
diff --git a/crates/nova-typeck/src/check.rs b/crates/nova-typeck/src/check.rs
index a52e43bba85a345f006eb07d5dcb237cb8e23352..b751e58ee5798847f3438f2acb3a7873534a861f 100644
--- a/crates/nova-typeck/src/check.rs
+++ b/crates/nova-typeck/src/check.rs
@@ -4504,6 +4504,8 @@ impl<'a> Checker<'a> {
             | Builtin::StrToLower
             | Builtin::FloatFixed
             | Builtin::StrToFloat
+            | Builtin::IntToChar
+            | Builtin::StrAsBytes
             | Builtin::TestSelector
             | Builtin::TaskSpawn
             | Builtin::TaskIsDone
@@ -8964,6 +8966,8 @@ pub(crate) fn builtin_signature(builtin: Builtin) -> (Vec<Ty>, Ty) {
         Builtin::StrToUpper | Builtin::StrToLower => (vec![Ty::String], Ty::String),
         Builtin::FloatFixed => (vec![Ty::Float, Ty::Int], Ty::String),
         Builtin::StrToFloat => (vec![Ty::String], Ty::Float),
+        Builtin::IntToChar => (vec![Ty::Int], Ty::Char),
+        Builtin::StrAsBytes => (vec![Ty::String], Ty::Bytes),
         Builtin::TestSelector => (vec![], Ty::Int),
         Builtin::TaskSpawn => (vec![future_of_param0()], Ty::Int),
         Builtin::TaskIsDone => (vec![future_of_param0()], Ty::Bool),
diff --git a/crates/nova-typeck/src/check/fixes.rs b/crates/nova-typeck/src/check/fixes.rs
index 7b79eb36e18302e15859f59764d67d3697a4d346..dfff262193dec004eac17d3d551a04c93ac319f6 100644
--- a/crates/nova-typeck/src/check/fixes.rs
+++ b/crates/nova-typeck/src/check/fixes.rs
@@ -79,6 +79,24 @@ pub(super) fn scrutinee_sum(fcx: &FnCtx, ty: &Ty) -> Option<DefId> {
     }
 }
 
+/// The std-only builtins whose work std's public surface does since 5.0c
+/// (5.0 spec §5.2 to §5.4), with the form a program writes instead. A
+/// program that calls one by name gets E0001, and this note in place of
+/// "did you mean", whose nearest name is not always the same operation.
+const STD_ONLY_PUBLIC_FORMS: &[(&str, &str)] = &[
+    ("char_to_int", "the method `to_int`, as in `c.to_int()`"),
+    (
+        "int_to_char",
+        "`char_from_int(n)`, which is `None` when `n` is not a character",
+    ),
+    ("str_from_chars", "`string_from_chars(cs)`"),
+    (
+        "str_as_bytes",
+        "a `String`'s `byte_len()`, `byte_at(i)` and `byte_slice(a, b)`, or \
+         `bytes_from_string(s)` for a copy",
+    ),
+];
+
 impl<'a> Checker<'a> {
     /// "make `x` mutable" (spec §4.1): `mut ` before the name of a local
     /// bound by a `let` or as a parameter. `decl` is that name's span.
@@ -136,6 +154,16 @@ impl<'a> Checker<'a> {
     /// names the site could mean: the visible locals, or the type
     /// parameters in scope.
     pub(super) fn name_fixes(&mut self, name: Spanned<String>, need: Need, nearby: Vec<String>) {
+        // A std-only builtin that 5.0c's std wraps gets its public form as a
+        // note instead of a guess: the nearest name can mean something else,
+        // `char_from_int` for `char_to_int`, the conversion the other way.
+        if let Some((_, form)) = STD_ONLY_PUBLIC_FORMS.iter().find(|(b, _)| *b == name.value) {
+            if let Some(d) = self.diagnostics.last_mut() {
+                d.notes
+                    .push(format!("`{}` is internal to std; use {form}", name.value));
+            }
+            return;
+        }
         let import = self.import_fix(&name, &need);
         let change = self.did_you_mean(&name, &need, &nearby);
         self.push_fix(import);
diff --git a/std/core/lib.nova b/std/core/lib.nova
index b8e8f5a4d4ab8717d2d8f89e9569b63e52f39b01..c59e70c8ab4d2f17b2cb825104973c3a41e92e9a 100644
--- a/std/core/lib.nova
+++ b/std/core/lib.nova
@@ -962,3 +962,42 @@ impl Hash for Char {
 impl Hash for String {
     fn hash(self) -> Int { str_hash(self) }
 }
+
+// Conversions between `Char` and `Int` (5.0 spec §5.2). A method where there
+// is a receiver; `char_from_int` is a free function, named after
+// `bytes_from_string`, because `Char::from_int(n)` would need an associated
+// function on a primitive that returns an `Option` of it, and the free name
+// is what the rest of std's constructors look like.
+impl Char {
+    // A register move: `char_to_int` has no runtime symbol on any target.
+    /// Returns the character's Unicode scalar value, from 0 to `0x10FFFF`.
+    ///
+    /// ```
+    /// assert_eq('A'.to_int(), 65)
+    /// assert_eq('é'.to_int(), 233)
+    /// assert_eq(char_from_int('z'.to_int()).unwrap(), 'z')
+    /// ```
+    pub fn to_int(self) -> Int { char_to_int(self) }
+}
+
+// The range is checked here, in Nova, so `int_to_char` can be a register move
+// with no check of its own, as `char_to_int` is: a `Char` outside the scalar
+// values would break every `str_from_chars` and `Display` downstream, so this
+// function is the only place a `Char` is made from an `Int`.
+/// Returns the character whose Unicode scalar value is `n`, or `None` if `n`
+/// is not one: below 0, above `0x10FFFF`, or a surrogate, `0xD800` to
+/// `0xDFFF`.
+///
+/// ```
+/// assert_eq(char_from_int(97).unwrap(), 'a')
+/// assert_eq(char_from_int(0x10FFFF).unwrap().to_int(), 0x10FFFF)
+/// assert(char_from_int(0xD800).is_none(), "a surrogate is not a character")
+/// assert(char_from_int(0x110000).is_none(), "past the last scalar value")
+/// assert(char_from_int(-1).is_none(), "negative")
+/// ```
+pub fn char_from_int(n: Int) -> Option<Char> {
+    if n < 0 { return None }
+    if n > 0x10FFFF { return None }
+    if n >= 0xD800 && n <= 0xDFFF { return None }
+    Some(int_to_char(n))
+}
diff --git a/std/strings/lib.nova b/std/strings/lib.nova
index dc853edbb81ff3f11a96bfc8ae72d7f0cfc9496f..307547904a865fd7856466090d174535457e6ea7 100644
--- a/std/strings/lib.nova
+++ b/std/strings/lib.nova
@@ -138,6 +138,10 @@ fn trim_end_index(cs: [Char], floor: Int) -> Int {
     floor
 }
 
+// Whether byte `b` continues a UTF-8 character, so that no character starts
+// there: `0x80` to `0xBF`. `byte_slice`'s boundary check.
+fn continues_char(b: Int) -> Bool { b >= 128 && b <= 191 }
+
 impl String {
     // Not `self.len() == 0`: `len()` decodes the whole string, while `==` on
     // `String` compares byte lengths first in the runtime.
@@ -403,4 +407,81 @@ impl String {
     /// The result can have more characters than the string: `"İ".to_lower()`
     /// is `i` followed by a combining dot above, U+0307.
     pub fn to_lower(self) -> String { str_to_lower(self) }
+
+    // BYTE ACCESS, in constant time (5.0 spec §5.2). `str_as_bytes` is a
+    // register move: a `String` and a `Bytes` are one layout, so the view
+    // copies nothing, and the `Bytes` builtins read the string's own bytes.
+    // These count BYTES, unlike every other index in this module.
+    /// Returns the number of bytes in the string's UTF-8 encoding, in
+    /// constant time. `len` counts characters instead.
+    ///
+    /// ```
+    /// assert_eq("abc".byte_len(), 3)
+    /// assert_eq("é".byte_len(), 2)
+    /// assert_eq("é".len(), 1)
+    /// ```
+    pub fn byte_len(self) -> Int { bytes_len(str_as_bytes(self)) }
+
+    // Panics rather than returning an `Option`, as `a[i]` does: a lexer
+    // reads a byte per step, and an `Option` per byte is an allocation per
+    // byte (5.0 spec §12 item 19). The check is here, so the panic names
+    // this method; `bytes_at` would abort with its own message.
+    /// Returns the byte at byte index `i`, from 0 to 255, in constant time.
+    ///
+    /// Panics if `i` is negative or not less than `byte_len()`.
+    ///
+    /// ```
+    /// let s = "aé"
+    /// assert_eq(s.byte_at(0), 97)
+    /// assert_eq(s.byte_at(1), 0xC3) // the first byte of `é`
+    /// assert_eq(s.byte_at(2), 0xA9)
+    /// ```
+    pub fn byte_at(self, i: Int) -> Int {
+        let b = str_as_bytes(self)
+        if i < 0 || i >= bytes_len(b) { panic("String::byte_at index out of range") }
+        bytes_at(b, i)
+    }
+
+    // A copy of the bytes `start..end`. Both bounds must fall on a character's
+    // first byte, or at the end, so the copy is always valid UTF-8 and needs
+    // no check of its own.
+    /// Returns a copy of the bytes from byte index `start` up to, but not
+    /// including, `end`, as a string.
+    ///
+    /// Panics if `start` is negative, if `end` is greater than `byte_len()`,
+    /// if `start` is greater than `end`, or if either bound falls inside a
+    /// character's encoding.
+    ///
+    /// ```
+    /// let s = "héllo"
+    /// assert_eq(s.byte_slice(0, 1), "h")
+    /// assert_eq(s.byte_slice(1, 3), "é") // `é` is two bytes
+    /// assert_eq(s.byte_slice(3, s.byte_len()), "llo")
+    /// assert_eq(s.byte_slice(3, 3), "")
+    /// ```
+    pub fn byte_slice(self, start: Int, end: Int) -> String {
+        let b = str_as_bytes(self)
+        let n = bytes_len(b)
+        if start < 0 { panic("String::byte_slice start is negative") }
+        if end > n { panic("String::byte_slice end is past the end of the string") }
+        if start > end { panic("String::byte_slice start is after end") }
+        if start < n && continues_char(bytes_at(b, start)) {
+            panic("String::byte_slice start is inside a character")
+        }
+        if end < n && continues_char(bytes_at(b, end)) {
+            panic("String::byte_slice end is inside a character")
+        }
+        bytes_to_string_unchecked(bytes_slice(b, start, end))
+    }
 }
+
+// `string_from_chars` is a free function, named after `bytes_from_string`
+// and `bytes_from_ints` (5.0 spec §12 item 21).
+/// Returns the string of the characters in `cs`, in order.
+///
+/// ```
+/// assert_eq(string_from_chars(['n', 'o', 'v', 'a']), "nova")
+/// assert_eq(string_from_chars("héllo".chars()), "héllo")
+/// assert_eq(string_from_chars([]), "")
+/// ```
+pub fn string_from_chars(cs: [Char]) -> String { str_from_chars(cs) }
````

### Task 3: `StringBuilder`

Spec §5.1; the user's decision 8; decisions 3, 4, 12, 29 and 30.

**Files:**
- Modify: `std/strings/lib.nova` (`StringBuilder`, its `impl` and `Display`)
- Create: `tests/runtime/string_builder.nova`, `string_builder.stdout`,
  `tests/runtime/string_builder_million.nova`,
  `string_builder_million.stdout`
- Test: `std_tests.rs` (two); `run_tests.rs` (`string_builder_runs`,
  `string_builder_million_runs`), `wasm_tests.rs` (both in `gate!`),
  `index_complete.rs` (the counts)

- [ ] **Step 1: The tests.** `patch_block t3-tests`. The patch holds:
  - `std_tests.rs`: the builder's types, with `byte_len` on a `String`; and
    `push` through a binding that is not `mut`, E0060.
  - **The programs:** `string_builder`, empty, one part, an empty part,
    non-ASCII, `push_char`, `clear`, `to_string` twice, interpolation, a
    builder shared by assignment, and ten thousand pushes, under `run`,
    `build` and GC stress; `string_builder_million`, a million one-byte
    pushes and one `to_string`, read back with `byte_slice` and `byte_at`,
    with no timing assertion and without GC stress. Each with its `.stdout`,
    `gate!` entry, and the counts, 180 programs and 214 files.
- [ ] **Step 2: Run, and see the tests fail.** First the build, which ends
  with `Finished`.
  - `T check::std_tests`. Expected: `18 passed; 2 failed`:
    `the_builder_has_the_specs_types` and
    `a_builder_mutates_only_through_a_mutable_binding`, no `StringBuilder`
    (the second finding E0900 "module-qualified paths are not supported yet"
    for `StringBuilder::new`, not the E0060 it asks for).
  - `R string_builder`. Expected: `0 passed; 2 failed`,
    `string_builder_runs` and `string_builder_million_runs`, each program
    refused: E0900 "module-qualified paths are not supported yet" at
    `StringBuilder::new()`, E0001 "cannot find type `StringBuilder`" and
    E0011s.
  - `W gate::string_builder gate::string_builder_million every_runtime_program_is_in_the_gate_or_left_out`.
    Expected: `1 passed; 2 failed`, the same.
  - `X`. Expected: `0 passed; 1 failed`.
- [ ] **Step 3: The code.** `patch_block t3-code`: **`std/strings`:**
  `StringBuilder { parts: Vec<String>, bytes: Int }`, its comment on why
  `to_string` copies the parts' pointers first; `new`, `push`, a `Vec` push
  and a byte count, `push_char`, `byte_len`, `is_empty`, by bytes, `clear`,
  and `to_string`, one `str_join`; and `Display` through `to_string`. Each
  with a doc test.
- [ ] **Step 4: Run.** First the build, which ends with `Finished`.
  - `T check::std_tests`. Expected: `20 passed; 0 failed`.
  - `R string_builder`. Expected: `2 passed; 0 failed`, in about 20 s.
  - `W gate::string_builder gate::string_builder_million every_runtime_program_is_in_the_gate_or_left_out`.
    Expected: `3 passed; 0 failed`.
  - `X`. Expected: `1 passed; 0 failed`.
  - `S`, as in Task 1 (about 5 min 35 s). Expected: `2750 1 10`: Task 2's
    figures and Task 3's 6 tests, the one failure the IPv6 test.
  - `G`. Expected:
    `release golden: 142 passed, 9 excluded, Ubuntu clang version 18.1.3 (1ubuntu1)`
    and `1 passed`.
- [ ] **Step 5: The checks,** as the conventions list them.
- [ ] **Step 6: Commit** `std: StringBuilder`.

**Patch `t3-tests`:**

<!-- patch t3-tests -->

````diff
diff --git a/crates/nova-cli/tests/run_tests.rs b/crates/nova-cli/tests/run_tests.rs
index 66359eba26bb8fef97708dbefa7ea732adce340a..18eef24e1b6ae8e74ded65f8f1b12a8f978f505f 100644
--- a/crates/nova-cli/tests/run_tests.rs
+++ b/crates/nova-cli/tests/run_tests.rs
@@ -13040,6 +13040,21 @@ fn for_arrays_run() {
     golden_three_ways("for_arrays", true);
 }
 
+/// `StringBuilder` (5.0 spec §5.1), also under GC stress, since its parts
+/// live in a `Vec` that grows while the collector runs.
+#[test]
+fn string_builder_runs() {
+    golden_three_ways("string_builder", true);
+}
+
+/// A million pushes into a `StringBuilder`, which a quadratic builder could
+/// not finish in the harness's time limit; no timing assertion (5.0 spec
+/// §5.1).
+#[test]
+fn string_builder_million_runs() {
+    golden_three_ways("string_builder_million", false);
+}
+
 /// `Char` and `Int` both ways, `string_from_chars`, and byte access on one-
 /// to four-byte characters (5.0 spec §5.2), also under GC stress.
 #[test]
diff --git a/crates/nova-cli/tests/wasm_tests.rs b/crates/nova-cli/tests/wasm_tests.rs
index df439e9d08c0d4e6ecfb81749f4fd2ea63cb7986..ac00927ea194c72ea60f638e3f58b23b1195469a 100644
--- a/crates/nova-cli/tests/wasm_tests.rs
+++ b/crates/nova-cli/tests/wasm_tests.rs
@@ -2036,6 +2036,8 @@ mod gate {
         // over a builtin that is a shim entry on WASM.
         array_helpers,
         hidden_trait_methods,
+        string_builder,
+        string_builder_million,
         char_and_bytes,
     );
 
diff --git a/crates/nova-driver/tests/index_complete.rs b/crates/nova-driver/tests/index_complete.rs
index dab2da366be0b9fdf0f6e466975018b24042418c..f63883a1fb28424f27d66c6a7ad6e83b7ed7915f 100644
--- a/crates/nova-driver/tests/index_complete.rs
+++ b/crates/nova-driver/tests/index_complete.rs
@@ -250,8 +250,9 @@ fn every_name_in_the_corpora_is_indexed() {
     // 2026-10-27, after 5.0b added `tuples` and `tuples_async`; 165 after
     // `patterns`, `patterns_async`, `match_effects` and `for_arrays`; 166
     // after 5.0c added `array_helpers`; 167 after `hidden_trait_methods`;
-    // 168 after `char_and_bytes`.
-    assert_eq!(programs.len(), 178, "{programs:#?}");
+    // 168 after `char_and_bytes`; 170 after `string_builder` and
+    // `string_builder_million`.
+    assert_eq!(programs.len(), 180, "{programs:#?}");
     let mut files = 0;
     let mut failures: Vec<String> = Vec::new();
     for (k, path) in programs.iter().enumerate() {
@@ -278,11 +279,11 @@ fn every_name_in_the_corpora_is_indexed() {
             every_name_is_covered(&a, file, &mut failures);
         }
     }
-    // 9 + 168 + 3 files of user code, 05-geometry's library's 2, the 8
+    // 9 + 170 + 3 files of user code, 05-geometry's library's 2, the 8
     // modules 07-fullstack-blog's client reaches from its `main.nova` (not
     // the server's 4, which only `server.nova` and `lib.nova` reach), and
     // native std's 22, `std/ui/ssr` among them.
-    assert_eq!(files, 212);
+    assert_eq!(files, 214);
     failures.sort();
     failures.dedup();
     assert!(
diff --git a/crates/nova-typeck/src/check/std_tests.rs b/crates/nova-typeck/src/check/std_tests.rs
index 130cb4c1242bb2b0fe5a560dfe8aeba32ddc0af8..3a9ff5654401aa0160fc2b0e4d6bc9a92ea0e797 100644
--- a/crates/nova-typeck/src/check/std_tests.rs
+++ b/crates/nova-typeck/src/check/std_tests.rs
@@ -183,6 +183,24 @@ fn vecs_helpers_have_the_specs_types() {
     assert_eq!(local_ty(&r, "last"), Ty::Int);
 }
 
+/// The builder's types, and a `String`'s length in bytes (spec §5.1,
+/// §5.2).
+#[test]
+fn the_builder_has_the_specs_types() {
+    let r = checked(
+        "fn main() {\n    let bl = \"a\".byte_len()\n    \
+         let mut sb = StringBuilder::new()\n    sb.push(\"x\")\n    sb.push_char('y')\n    \
+         let sl = sb.byte_len()\n    let se = sb.is_empty()\n    let st = sb.to_string()\n    \
+         sb.clear()\n}\n",
+    );
+    assert!(r.diagnostics.is_empty(), "{:#?}", r.diagnostics);
+    for name in ["bl", "sl"] {
+        assert_eq!(local_ty(&r, name), Ty::Int, "{name}");
+    }
+    assert_eq!(local_ty(&r, "se"), Ty::Bool);
+    assert_eq!(local_ty(&r, "st"), Ty::String);
+}
+
 /// The conversions' and byte access's types (spec §5.2).
 #[test]
 fn the_conversions_and_byte_access_have_the_specs_types() {
@@ -201,6 +219,14 @@ fn the_conversions_and_byte_access_have_the_specs_types() {
     }
 }
 
+/// `push` and `clear` take `mut self`, as `Vec`'s do.
+#[test]
+fn a_builder_mutates_only_through_a_mutable_binding() {
+    let e = errors("fn main() {\n    let b = StringBuilder::new()\n    b.push(\"x\")\n}\n");
+    assert_eq!(e.len(), 1, "{e:#?}");
+    assert_eq!(e[0].code, "E0060");
+}
+
 /// A program's impl that defines a method std's impl on the same type has,
 /// `iter` on arrays, is E0074 labelled at the program's impl, its primary
 /// label, and at std's, which the program cannot change (5.0 spec §5.5;
diff --git a/tests/runtime/string_builder.nova b/tests/runtime/string_builder.nova
new file mode 100644
index 0000000000000000000000000000000000000000..d8b0064f40b21935e11c73c2ca25e0abde988d12
--- /dev/null
+++ b/tests/runtime/string_builder.nova
@@ -0,0 +1,67 @@
+// `StringBuilder` (5.0 spec §5.1): empty, one part, non-ASCII, `push_char`,
+// `clear`, `to_string` twice, interpolation through `Display`, a builder
+// shared by assignment, and ten thousand pushes, also under GC stress, since
+// its parts live in a `Vec` that grows while the collector runs.
+
+fn report(label: String, b: StringBuilder) {
+    println("${label}: [${b.to_string()}] bytes=${b.byte_len()} empty=${b.is_empty()}")
+}
+
+fn digits(n: Int) -> StringBuilder {
+    let mut b = StringBuilder::new()
+    for i in 0..n { b.push("${i % 10}") }
+    b
+}
+
+fn main() {
+    let mut b = StringBuilder::new()
+    report("new", b)
+
+    b.push("one")
+    report("one part", b)
+
+    b.push("")
+    report("empty part", b)
+
+    b.clear()
+    b.push("")
+    report("only an empty part", b)
+
+    b.clear()
+    b.push("héllo")
+    b.push(", ")
+    b.push("世界")
+    b.push_char(' ')
+    b.push_char('🎉')
+    report("non-ASCII", b)
+
+    let first = b.to_string()
+    let second = b.to_string()
+    println("twice equal: ${first == second}, still ${b.byte_len()} bytes")
+
+    b.push_char('!')
+    println("after more: ${b}")
+
+    b.clear()
+    report("cleared", b)
+    b.push("again")
+    println("interpolated: <${b}> and ${b.byte_len()}")
+
+    let mut shared = b
+    shared.push(" shared")
+    println("shared: ${b}")
+
+    let big = digits(10000)
+    let s = big.to_string()
+    println(
+        "ten thousand: bytes=${big.byte_len()} len=${s.len()} head=${s.slice(0, 12)} tail=${s.slice(9988, 10000)}",
+    )
+
+    let mut lines = StringBuilder::new()
+    for i in 0..3 {
+        lines.push("line ")
+        lines.push("${i}")
+        lines.push_char('\n')
+    }
+    print(lines.to_string())
+}
diff --git a/tests/runtime/string_builder.stdout b/tests/runtime/string_builder.stdout
new file mode 100644
index 0000000000000000000000000000000000000000..f40cd56c33966fe0eb20e1e25651d0a6fdccbd91
--- /dev/null
+++ b/tests/runtime/string_builder.stdout
@@ -0,0 +1,14 @@
+new: [] bytes=0 empty=true
+one part: [one] bytes=3 empty=false
+empty part: [one] bytes=3 empty=false
+only an empty part: [] bytes=0 empty=true
+non-ASCII: [héllo, 世界 🎉] bytes=19 empty=false
+twice equal: true, still 19 bytes
+after more: héllo, 世界 🎉!
+cleared: [] bytes=0 empty=true
+interpolated: <again> and 5
+shared: again shared
+ten thousand: bytes=10000 len=10000 head=012345678901 tail=890123456789
+line 0
+line 1
+line 2
diff --git a/tests/runtime/string_builder_million.nova b/tests/runtime/string_builder_million.nova
new file mode 100644
index 0000000000000000000000000000000000000000..b1abb975d9ba9af636f6fc8e1399ace294a3188f
--- /dev/null
+++ b/tests/runtime/string_builder_million.nova
@@ -0,0 +1,23 @@
+// A million one-byte pushes into a `StringBuilder`, then one `to_string`
+// (5.0 spec §5.1). A builder that copied the whole string at each push would
+// copy about 500 GB here and could not finish in the harness's time limit;
+// there is no timing assertion, so a slow machine only runs longer.
+
+fn main() {
+    let mut b = StringBuilder::new()
+    let mut i = 0
+    while i < 1000000 {
+        if i % 2 == 0 { b.push("a") } else { b.push_char('b') }
+        i += 1
+    }
+    let s = b.to_string()
+    println("bytes ${b.byte_len()}")
+    println("first ${s.byte_slice(0, 4)} last ${s.byte_slice(999996, 1000000)}")
+    let mut count = 0
+    let mut j = 0
+    while j < s.byte_len() {
+        if s.byte_at(j) == 97 { count += 1 }
+        j += 1
+    }
+    println("a ${count}")
+}
diff --git a/tests/runtime/string_builder_million.stdout b/tests/runtime/string_builder_million.stdout
new file mode 100644
index 0000000000000000000000000000000000000000..25263cc0b29b975644ae94275efb1dc85a71047d
--- /dev/null
+++ b/tests/runtime/string_builder_million.stdout
@@ -0,0 +1,3 @@
+bytes 1000000
+first abab last abab
+a 500000
````

**Patch `t3-code`:**

<!-- patch t3-code -->

````diff
diff --git a/std/strings/lib.nova b/std/strings/lib.nova
index 307547904a865fd7856466090d174535457e6ea7..b91849f1fb12098c572d4aa131380462659f7042 100644
--- a/std/strings/lib.nova
+++ b/std/strings/lib.nova
@@ -485,3 +485,130 @@ impl String {
 /// assert_eq(string_from_chars([]), "")
 /// ```
 pub fn string_from_chars(cs: [Char]) -> String { str_from_chars(cs) }
+
+// A string built from parts (5.0 spec §5.1): a `Vec<String>` of the parts
+// pushed, and their total length in bytes. `push` is a `Vec` push, constant
+// time amortised; `to_string` joins every part in one copy, through
+// `str_join`, so building n bytes from any number of pushes is linear, where
+// `s = s + t` or `s = "${s}${t}"` in a loop copies the whole string each time.
+//
+// `to_string` copies the parts into an array of their exact count first,
+// `Vec::to_array`, because `str_join` takes an array and a `Vec`'s storage is
+// private to std/collections, and longer than its length. That copy is of the
+// parts' pointers, one word a part, not of their bytes; a `str_join` that took
+// a count would save it, at the price of a runtime symbol and a std/rt
+// function more, and of reaching into `Vec`'s storage from here.
+/// Builds a string from parts in time linear in its length, however many
+/// parts there are.
+///
+/// `push` appends a string and `push_char` a character; `to_string` returns
+/// everything pushed so far as one string, and leaves the builder as it was,
+/// so more can be pushed after. Appending with `+` or interpolation in a loop
+/// copies the whole string at every step, which takes time in proportion to
+/// the square of its length.
+///
+/// A builder is shared, not copied, when it is assigned, as a `Vec` is.
+///
+/// ```
+/// let mut b = StringBuilder::new()
+/// for i in 0..3 {
+///     b.push("item ")
+///     b.push("${i}")
+///     b.push_char(';')
+/// }
+/// assert_eq(b.to_string(), "item 0;item 1;item 2;")
+/// assert_eq(b.byte_len(), 21)
+/// assert_eq("[${b}]", "[item 0;item 1;item 2;]") // interpolation, through `Display`
+/// b.clear()
+/// assert(b.is_empty(), "clear removes every part")
+/// ```
+pub record StringBuilder { parts: Vec<String>, bytes: Int }
+
+impl StringBuilder {
+    /// Returns an empty builder.
+    ///
+    /// ```
+    /// let b = StringBuilder::new()
+    /// assert_eq(b.to_string(), "")
+    /// ```
+    pub fn new() -> StringBuilder { StringBuilder { parts: Vec::new(), bytes: 0 } }
+
+    // An empty `s` is pushed too: it costs a slot, and skipping it would make
+    // `push` branch for nothing a caller can see.
+    /// Appends `s`, in constant time, amortised.
+    ///
+    /// ```
+    /// let mut b = StringBuilder::new()
+    /// b.push("ab")
+    /// b.push("cd")
+    /// assert_eq(b.to_string(), "abcd")
+    /// ```
+    pub fn push(mut self, s: String) {
+        self.parts.push(s)
+        self.bytes = self.bytes + s.byte_len()
+    }
+
+    /// Appends the character `c`.
+    ///
+    /// ```
+    /// let mut b = StringBuilder::new()
+    /// b.push_char('h')
+    /// b.push_char('é')
+    /// assert_eq(b.to_string(), "hé")
+    /// assert_eq(b.byte_len(), 3)
+    /// ```
+    pub fn push_char(mut self, c: Char) { self.push(str_from_chars([c])) }
+
+    // Kept as parts are pushed, so constant time. There is no `len`: a count
+    // of characters would have to walk every part.
+    /// Returns the length in bytes of the string built so far, in constant
+    /// time.
+    ///
+    /// ```
+    /// let mut b = StringBuilder::new()
+    /// b.push("añ")
+    /// assert_eq(b.byte_len(), 3)
+    /// ```
+    pub fn byte_len(self) -> Int { self.bytes }
+
+    // By bytes, not parts: a builder holding only empty parts is empty.
+    /// Returns `true` if the string built so far is empty.
+    ///
+    /// ```
+    /// let mut b = StringBuilder::new()
+    /// b.push("")
+    /// assert(b.is_empty(), "an empty part adds nothing")
+    /// ```
+    pub fn is_empty(self) -> Bool { self.bytes == 0 }
+
+    /// Removes every part, leaving the builder empty.
+    ///
+    /// ```
+    /// let mut b = StringBuilder::new()
+    /// b.push("old")
+    /// b.clear()
+    /// b.push("new")
+    /// assert_eq(b.to_string(), "new")
+    /// ```
+    pub fn clear(mut self) {
+        self.parts.clear()
+        self.bytes = 0
+    }
+
+    /// Returns the string built so far, joined in one copy. The builder is
+    /// left as it was.
+    ///
+    /// ```
+    /// let mut b = StringBuilder::new()
+    /// b.push("a")
+    /// assert_eq(b.to_string(), "a")
+    /// b.push("b")
+    /// assert_eq(b.to_string(), "ab")
+    /// ```
+    pub fn to_string(self) -> String { str_join("", self.parts.to_array()) }
+}
+
+// Interpolation, `"${b}"`, reaches the built string through `Display`.
+impl Display for StringBuilder {
+    fn fmt(self) -> String { self.to_string() }
+}
````

### Task 4: `Float.to_bits`

Spec §5.4; the user's decision 8; decisions 3, 9, 13, 19, 28 and 30.

**Files:**
- Modify:
  - `crates/nova-resolver/src/lib.rs` (`FloatToBits`; `STD_ONLY` 93)
  - `crates/nova-typeck/src/check.rs` (its signature),
    `crates/nova-typeck/src/check/fixes.rs` (its public form)
  - `crates/nova-mir/src/lib.rs` (`RtFunc::FloatToBits`: name, signature,
    `Host("float_to_bits")` on WASM), `crates/nova-mir/src/lower.rs`
  - `crates/nova-runtime/src/lib.rs` (`nova_rt_float_to_bits`, its symbol)
  - `crates/nova-codegen-wasm/src/shim.rs` (the host entry, the float
    helpers with it), `crates/nova-codegen-wasm/src/shim/float.js`
    (`floatBits`)
  - `std/core/lib.nova` (`impl Float { to_bits }`)
- Test: `std_tests.rs` (two, and a row in each of two); the signature test's
  row; `lower_tests.rs` (one); the runtime's
  `float_to_bits_is_rusts_to_bits`; the shim's `float_bits_is_rusts_to_bits`
  and two asserts in `helpers_come_only_with_their_host_functions`

- [ ] **Step 1: The tests.** `patch_block t4-tests`. The patch holds:
  `to_bits`' type; a program's trait `to_bits` on `Float`, E0105 (decision
  40); `float_to_bits` kept from user code, and its public form; its
  signature row; `to_bits` one runtime call; the runtime's function at 0,
  -0, 1, the smallest subnormal, the largest finite, both infinities and a
  NaN; the shim's `floatBits` giving back each of twelve values' bits under
  Node, a NaN's sign included; and the float helpers coming with
  `float_to_bits` alone. No program: `float_bits` makes its infinities and
  NaNs with `parse_float`, so it rides with Task 5 (decision 30).
- [ ] **Step 2: Run, and see the tests fail.** First the build, which ends
  with `Finished`.
  - `T check::std_tests check::tests::builtin_signatures_are_what_the_std_call_sites_use float_to_bits_is_rusts_to_bits float_bits_is_rusts_to_bits shim::tests::helpers_come_only_with_their_host_functions`.
    Expected: it does not compile, eight
    ``error[E0425]: cannot find function `nova_rt_float_to_bits` in this scope``
    and
    ``error: could not compile `nova-runtime` (lib test) due to 8 previous errors``
    (decision 32).
  - `L to_bits`. Expected: it does not compile,
    ``error[E0599]: no variant, associated function, or constant named `FloatToBits` found for enum `RtFunc` in the current scope``.
- [ ] **Step 3: The code.** `patch_block t4-code`:
  - **The resolver, the checker, MIR:** `FloatToBits`,
    `float_to_bits(Float) -> Int`, `RtFunc::FloatToBits`, `(f64) -> i64`,
    `nova_rt_float_to_bits` natively and the shim's `float_to_bits` on WASM;
    `STD_ONLY` 93; its public form, `x.to_bits()`.
  - **The runtime:** `nova_rt_float_to_bits`, `v.to_bits() as i64`, in
    `symbols()`.
  - **The shim:** the host entry, `floatBits(v)`, and the float helpers
    included for it; `floatBits` through a `DataView`, `getBigInt64`.
  - **`std/core`:** `impl Float { pub fn to_bits(self) -> Int }`, with a doc
    test.
- [ ] **Step 4: Run.** First the build, which ends with `Finished`.
  - `T check::std_tests check::tests::builtin_signatures_are_what_the_std_call_sites_use float_to_bits_is_rusts_to_bits float_bits_is_rusts_to_bits shim::tests::helpers_come_only_with_their_host_functions`.
    Expected: every `test result` `ok`, 26 passing in all: `nova-typeck`'s
    23 (`std_tests.rs`' 22 and the signature test), the shim's 2 and the
    runtime's 1.
  - `L to_bits`. Expected: `1 passed; 0 failed`.
  - `S`, as in Task 1 (about 5 min 50 s). Expected: `2755 1 10`: Task 3's
    figures and Task 4's 5 tests, the one failure the IPv6 test. No program
    joins the walk, so Task 3's `G` stands.
- [ ] **Step 5: The checks,** as the conventions list them.
- [ ] **Step 6: Commit** `std, runtime, shim: Float.to_bits`.

**Patch `t4-tests`:**

<!-- patch t4-tests -->

````diff
diff --git a/crates/nova-codegen-wasm/src/shim.rs b/crates/nova-codegen-wasm/src/shim.rs
index 34fdc40f66ae00ab4febec6ba2031c9313dbba0e..039f31218e192918026927dde5fb5d8fb5cd4c72 100644
--- a/crates/nova-codegen-wasm/src/shim.rs
+++ b/crates/nova-codegen-wasm/src/shim.rs
@@ -243,6 +243,58 @@ mod tests {
         assert!(floats.contains("function hashSeed"), "{floats}");
         let clocks = shim(&["time_now_epoch_nanos"]).expect("a shim");
         assert!(clocks.contains("function epochNanos"), "{clocks}");
+        // 5.0c's `float_to_bits` calls `floatBits`, in the float helpers.
+        let bits = shim(&["float_to_bits"]).expect("a shim");
+        assert!(bits.contains("function floatBits"), "{bits}");
+        assert!(bits.contains("  float_to_bits: ("), "{bits}");
+    }
+
+    /// `floatBits`, which `float_to_bits` calls, is Rust's `f64::to_bits`
+    /// read as an `i64`, a NaN's sign included (5.0 spec §5.4): each value is
+    /// made in JavaScript from its bits, and must give them back.
+    #[test]
+    fn float_bits_is_rusts_to_bits() {
+        let Some(mut node) = crate::test_node() else {
+            return;
+        };
+        let values = [
+            0.0,
+            -0.0,
+            1.0,
+            -1.0,
+            0.1,
+            f64::from_bits(1),
+            f64::MIN_POSITIVE,
+            f64::MAX,
+            f64::INFINITY,
+            f64::NEG_INFINITY,
+            f64::NAN,
+            -f64::NAN,
+        ];
+        let bits: Vec<String> = values.iter().map(|v| format!("{}n", v.to_bits())).collect();
+        let script = format!(
+            "{FLOAT}\nconst view = new DataView(new ArrayBuffer(8));\n\
+             for (const b of [{}]) {{\n  view.setBigUint64(0, b);\n  \
+             console.log(String(floatBits(view.getFloat64(0))));\n}}\n",
+            bits.join(", ")
+        );
+        let dir = std::env::temp_dir().join("nova-codegen-wasm-float-bits");
+        let _ = std::fs::remove_dir_all(&dir);
+        std::fs::create_dir_all(&dir).expect("a scratch directory");
+        let file = dir.join("float_bits.js");
+        std::fs::write(&file, script).expect("the script");
+        let out = node.arg(&file).output().expect("node runs");
+        assert!(
+            out.status.success(),
+            "{}",
+            String::from_utf8_lossy(&out.stderr)
+        );
+        let got = String::from_utf8(out.stdout).expect("UTF-8");
+        let expected: Vec<String> = values
+            .iter()
+            .map(|v| (v.to_bits() as i64).to_string())
+            .collect();
+        assert_eq!(got.lines().collect::<Vec<_>>(), expected);
     }
 
     /// The monotonic clock starts at 0 and never decreases over a busy
diff --git a/crates/nova-mir/tests/lower_tests.rs b/crates/nova-mir/tests/lower_tests.rs
index 1dc8dd08ef8fcafc37c4cc9f19ebbecd7caf09d8..dcac4dd57a421764888eb6748258bc7ebb99e97b 100644
--- a/crates/nova-mir/tests/lower_tests.rs
+++ b/crates/nova-mir/tests/lower_tests.rs
@@ -2434,6 +2434,18 @@ fn char_from_int_checks_the_range_and_moves() {
     );
 }
 
+/// `Float.to_bits` is one call of the runtime's `float_to_bits` (spec
+/// §5.4).
+#[test]
+fn to_bits_is_one_runtime_call() {
+    let mir = mir_for("fn main() {\n    let b = (1.5).to_bits()\n    println(\"${b}\")\n}");
+    let name = std_fn(&mir, "to_bits");
+    assert_eq!(
+        count_rt(&stmts_of(&mir, &name), nova_mir::RtFunc::FloatToBits),
+        1
+    );
+}
+
 /// `impl<T> [T]`'s methods monomorphise per element type, as any generic
 /// impl's do (spec §5.5).
 #[test]
diff --git a/crates/nova-runtime/src/lib.rs b/crates/nova-runtime/src/lib.rs
index 55af2cd05458c9728f473417a45f4b93a09318b0..0d27778f17d252cc753cd9504b5ce97fe72b1be1 100644
--- a/crates/nova-runtime/src/lib.rs
+++ b/crates/nova-runtime/src/lib.rs
@@ -2011,6 +2011,22 @@ mod tests {
         }
     }
 
+    /// `float_to_bits` is `f64::to_bits`, read as an `i64` (5.0 spec §5.4).
+    #[test]
+    fn float_to_bits_is_rusts_to_bits() {
+        assert_eq!(nova_rt_float_to_bits(0.0), 0);
+        assert_eq!(nova_rt_float_to_bits(-0.0), i64::MIN);
+        assert_eq!(nova_rt_float_to_bits(1.0), 0x3FF0_0000_0000_0000);
+        assert_eq!(nova_rt_float_to_bits(f64::from_bits(1)), 1);
+        assert_eq!(nova_rt_float_to_bits(f64::MAX), 0x7FEF_FFFF_FFFF_FFFF);
+        assert_eq!(nova_rt_float_to_bits(f64::INFINITY), 0x7FF0_0000_0000_0000);
+        assert_eq!(
+            nova_rt_float_to_bits(f64::NEG_INFINITY),
+            0xFFF0_0000_0000_0000_u64 as i64
+        );
+        assert_eq!(nova_rt_float_to_bits(f64::NAN), 0x7FF8_0000_0000_0000);
+    }
+
     /// A negative `places` has no meaning and must never reach `format!`'s
     /// precision argument, which would panic.
     #[test]
diff --git a/crates/nova-typeck/src/check.rs b/crates/nova-typeck/src/check.rs
index b751e58ee5798847f3438f2acb3a7873534a861f..b797f45995b81fe5602c8d70dd3e866b6f1b5933 100644
--- a/crates/nova-typeck/src/check.rs
+++ b/crates/nova-typeck/src/check.rs
@@ -17096,6 +17096,10 @@ mod tests {
                     (vec![Ty::String], Ty::Float),
                     "`str_to_float(span)` in `std/json`'s `scan_number`",
                 ),
+                Builtin::FloatToBits => (
+                    (vec![Ty::Float], Ty::Int),
+                    "`float_to_bits(self)` in `Float::to_bits`",
+                ),
                 Builtin::IntToChar => (
                     (vec![Ty::Int], Ty::Char),
                     "`int_to_char(n)` in `char_from_int`",
diff --git a/crates/nova-typeck/src/check/std_tests.rs b/crates/nova-typeck/src/check/std_tests.rs
index 3a9ff5654401aa0160fc2b0e4d6bc9a92ea0e797..2d170f1b46256ca1dcdd03b1992e950aae5a4b2d 100644
--- a/crates/nova-typeck/src/check/std_tests.rs
+++ b/crates/nova-typeck/src/check/std_tests.rs
@@ -155,6 +155,7 @@ fn the_new_builtins_are_not_visible_to_a_program() {
     for (call, name) in [
         ("int_to_char(65)", "int_to_char"),
         ("str_as_bytes(\"a\")", "str_as_bytes"),
+        ("float_to_bits(1.0)", "float_to_bits"),
     ] {
         let e = errors(&format!("fn main() {{\n    let x = {call}\n}}\n"));
         assert!(
@@ -219,6 +220,14 @@ fn the_conversions_and_byte_access_have_the_specs_types() {
     }
 }
 
+/// `Float.to_bits`'s type (spec §5.4).
+#[test]
+fn to_bits_has_the_specs_type() {
+    let r = checked("fn main() {\n    let fb = (1.0).to_bits()\n}\n");
+    assert!(r.diagnostics.is_empty(), "{:#?}", r.diagnostics);
+    assert_eq!(local_ty(&r, "fb"), Ty::Int);
+}
+
 /// `push` and `clear` take `mut self`, as `Vec`'s do.
 #[test]
 fn a_builder_mutates_only_through_a_mutable_binding() {
@@ -262,6 +271,7 @@ fn a_std_only_builtin_named_by_a_program_gets_its_public_form() {
         ("int_to_char(65)", "`char_from_int(n)`"),
         ("str_from_chars(['a'])", "`string_from_chars(cs)`"),
         ("str_as_bytes(\"a\")", "`byte_at(i)`"),
+        ("float_to_bits(1.0)", "`x.to_bits()`"),
     ] {
         let e = errors(&format!("fn main() {{\n    let x = {call}\n}}\n"));
         let d = e.iter().find(|d| d.code == "E0001").expect("an E0001");
@@ -477,3 +487,16 @@ fn a_trait_method_hidden_by_strings_and_chars_methods_is_e0105() {
         ]
     );
 }
+
+/// `Float`'s `to_bits`, std's since Task 4, hides a program's trait method
+/// of that name: E0105.
+#[test]
+fn a_trait_method_hidden_by_floats_to_bits_is_e0105() {
+    let src = "trait Bits {\n    fn to_bits(self) -> Int\n}\n\
+               impl Bits for Float {\n    fn to_bits(self) -> Int { 42 }\n}\n\
+               fn main() {}\n";
+    assert_eq!(
+        e0105s(src),
+        ["std's inherent method `to_bits` on `Float` hides `Bits::to_bits`"]
+    );
+}
````

**Patch `t4-code`:**

<!-- patch t4-code -->

````diff
diff --git a/crates/nova-codegen-wasm/src/shim.rs b/crates/nova-codegen-wasm/src/shim.rs
index 039f31218e192918026927dde5fb5d8fb5cd4c72..4a6896615f3d8ebfaeefe84e58f553174cf8e4f3 100644
--- a/crates/nova-codegen-wasm/src/shim.rs
+++ b/crates/nova-codegen-wasm/src/shim.rs
@@ -72,6 +72,7 @@ fn host(name: &str) -> Result<&'static str> {
         "float_to_str" => "  float_to_str: (v) => string(floatText(v)),\n",
         "float_fixed" => "  float_fixed: (v, places) => string(floatFixed(v, places)),\n",
         "str_to_float" => "  str_to_float: (s) => floatFrom(text(s)),\n",
+        "float_to_bits" => "  float_to_bits: (v) => floatBits(v),\n",
         "str_to_upper" => "  str_to_upper: (s) => string(text(s).toUpperCase()),\n",
         "str_to_lower" => "  str_to_lower: (s) => string(text(s).toLowerCase()),\n",
         "int_hash_seed" => "  int_hash_seed: () => hashSeed(),\n",
@@ -185,7 +186,12 @@ pub fn shim<S: AsRef<str>>(imports: &[S]) -> Result<String> {
     let imports: Vec<&str> = imports.iter().map(AsRef::as_ref).collect();
     let mut js = String::from(CORE);
     let uses = |names: &[&str]| imports.iter().any(|i| names.contains(i));
-    if uses(&["float_to_str", "float_fixed", "str_to_float"]) {
+    if uses(&[
+        "float_to_str",
+        "float_fixed",
+        "str_to_float",
+        "float_to_bits",
+    ]) {
         js.push_str(FLOAT);
     }
     if uses(&["int_hash_seed"]) {
diff --git a/crates/nova-codegen-wasm/src/shim/float.js b/crates/nova-codegen-wasm/src/shim/float.js
index e02ce3fafc10d46e9ae7beb4de23d5a17066bb9f..7951da9c255bd1edfe658561eafb96e2cbc251e2 100644
--- a/crates/nova-codegen-wasm/src/shim/float.js
+++ b/crates/nova-codegen-wasm/src/shim/float.js
@@ -95,3 +95,11 @@ function floatFrom(s) {
   if (body === "nan") return NaN;
   return Number(s);
 }
+
+// Rust's `f64::to_bits`, read as an `i64` as nova_rt_float_to_bits does:
+// the double's bits, as the BigInt an `Int` result is (5.0 spec §5.4).
+function floatBits(v) {
+  const view = new DataView(new ArrayBuffer(8));
+  view.setFloat64(0, v);
+  return view.getBigInt64(0);
+}
diff --git a/crates/nova-mir/src/lib.rs b/crates/nova-mir/src/lib.rs
index 55e1ac310c93d72764a6685c24ce272e8b30ab73..4da5a1c8f6ae347bae0f3365f207cb3ee3a17d50 100644
--- a/crates/nova-mir/src/lib.rs
+++ b/crates/nova-mir/src/lib.rs
@@ -198,6 +198,8 @@ rt_funcs! {
     FloatFixed,
     /// `(str) -> f64` — decimal text parsed as a `Float`.
     StrToFloat,
+    /// `(f64) -> i64` — the IEEE 754 bits of the value, as `f64::to_bits`.
+    FloatToBits,
     /// `(i8) -> str`
     BoolToStr,
     /// `(i64 codepoint) -> str`
@@ -574,6 +576,7 @@ impl RtFunc {
             RtFunc::FloatToStr => "nova_rt_float_to_str",
             RtFunc::FloatFixed => "nova_rt_float_fixed",
             RtFunc::StrToFloat => "nova_rt_str_to_float",
+            RtFunc::FloatToBits => "nova_rt_float_to_bits",
             RtFunc::BoolToStr => "nova_rt_bool_to_str",
             RtFunc::CharToStr => "nova_rt_char_to_str",
             RtFunc::StrEq => "nova_rt_str_eq",
@@ -682,6 +685,7 @@ impl RtFunc {
             RtFunc::FloatToStr => (vec![MirTy::F64], MirTy::Ptr),
             RtFunc::FloatFixed => (vec![MirTy::F64, MirTy::I64], MirTy::Ptr),
             RtFunc::StrToFloat => (vec![MirTy::Ptr], MirTy::F64),
+            RtFunc::FloatToBits => (vec![MirTy::F64], MirTy::I64),
             RtFunc::BoolToStr => (vec![MirTy::I8], MirTy::Ptr),
             RtFunc::CharToStr => (vec![MirTy::I64], MirTy::Ptr),
             RtFunc::StrEq => (vec![MirTy::Ptr, MirTy::Ptr], MirTy::I8),
@@ -944,6 +948,7 @@ impl RtFunc {
             RtFunc::FloatToStr => WasmRuntime::Host("float_to_str"),
             RtFunc::FloatFixed => WasmRuntime::Host("float_fixed"),
             RtFunc::StrToFloat => WasmRuntime::Host("str_to_float"),
+            RtFunc::FloatToBits => WasmRuntime::Host("float_to_bits"),
             RtFunc::BoolToStr => WasmRuntime::Nova("rt_bool_to_str"),
             RtFunc::CharToStr => WasmRuntime::Nova("rt_char_to_str"),
             RtFunc::StrEq => WasmRuntime::Nova("rt_str_eq"),
diff --git a/crates/nova-mir/src/lower.rs b/crates/nova-mir/src/lower.rs
index 0f06bc7987c0106b9324e8dd85b37bfc279f8fe7..26e91e9aae5f1e6fef55504e6c3be608390570c7 100644
--- a/crates/nova-mir/src/lower.rs
+++ b/crates/nova-mir/src/lower.rs
@@ -713,6 +713,7 @@ impl<'a> Lowerer<'a> {
                     Builtin::StrToLower => Lowering::Runtime(RtFunc::StrToLower),
                     Builtin::FloatFixed => Lowering::Runtime(RtFunc::FloatFixed),
                     Builtin::StrToFloat => Lowering::Runtime(RtFunc::StrToFloat),
+                    Builtin::FloatToBits => Lowering::Runtime(RtFunc::FloatToBits),
                     Builtin::IntToChar => Lowering::Reinterpret,
                     Builtin::StrAsBytes => Lowering::Reinterpret,
                     Builtin::TestSelector => Lowering::Runtime(RtFunc::TestSelector),
diff --git a/crates/nova-resolver/src/lib.rs b/crates/nova-resolver/src/lib.rs
index 99b6500f832153d740c3847133a1123bde4febdd..b56b41a0ac9e3e00cef7f3c56631c47e197abd0a 100644
--- a/crates/nova-resolver/src/lib.rs
+++ b/crates/nova-resolver/src/lib.rs
@@ -195,6 +195,14 @@ builtins! {
     /// replaced it on 2026-10-03. Std-only, so
     /// `str_to_float` is not a reserved word in user code.
     StrToFloat,
+    /// `float_to_bits(v: Float) -> Int` — the IEEE 754 bits of `v`, as Rust's
+    /// `f64::to_bits`, read as a signed `Int`. Backs `std/core`'s
+    /// `Float.to_bits`, which N's `.ll` emitter needs to write each float
+    /// constant as S0 does, the hex of its bits. A runtime call natively
+    /// (`nova_rt_float_to_bits`), and a shim entry on WASM, through a
+    /// `DataView` (5.0 spec §5.4, decision 8): one `RtFunc` serves both, so
+    /// no backend has a new statement. Std-only.
+    FloatToBits,
     /// `int_to_char(n: Int) -> Char` — the `Char` whose scalar value is `n`,
     /// the inverse of [`Builtin::CharToInt`]. A register move, as that one
     /// is, with no runtime symbol on any target. **The caller must have
@@ -1052,6 +1060,7 @@ impl Builtin {
             Builtin::StrToLower => "str_to_lower",
             Builtin::FloatFixed => "float_fixed",
             Builtin::StrToFloat => "str_to_float",
+            Builtin::FloatToBits => "float_to_bits",
             Builtin::IntToChar => "int_to_char",
             Builtin::StrAsBytes => "str_as_bytes",
             Builtin::TestSelector => "test_selector",
@@ -1177,7 +1186,7 @@ impl Builtin {
     /// consecutive review rounds (see the Phase 2.2b whole-branch review),
     /// because the roster is duplicated information that only this array
     /// needs to stay exact.
-    pub const STD_ONLY: [Builtin; 92] = [
+    pub const STD_ONLY: [Builtin; 93] = [
         Builtin::StrCmp,
         Builtin::StrHash,
         Builtin::CharToInt,
@@ -1191,6 +1200,7 @@ impl Builtin {
         Builtin::StrToLower,
         Builtin::FloatFixed,
         Builtin::StrToFloat,
+        Builtin::FloatToBits,
         Builtin::IntToChar,
         Builtin::StrAsBytes,
         Builtin::TestSelector,
diff --git a/crates/nova-runtime/src/lib.rs b/crates/nova-runtime/src/lib.rs
index 0d27778f17d252cc753cd9504b5ce97fe72b1be1..d795ef611136e17c8c4ad46f58592479b4a91234 100644
--- a/crates/nova-runtime/src/lib.rs
+++ b/crates/nova-runtime/src/lib.rs
@@ -870,6 +870,18 @@ pub unsafe extern "C" fn nova_rt_str_to_float(s: *const NovaStr) -> f64 {
     as_str(s).parse::<f64>().unwrap_or(0.0)
 }
 
+/// The IEEE 754 bits of `v`, as `f64::to_bits`, read as a signed `Int`.
+///
+/// Backs `std/core`'s `Float.to_bits` (5.0 spec §5.4). A runtime call rather
+/// than a bit cast in MIR, because one `RtFunc` is then a call natively and a
+/// shim entry on WASM, the user's choice there, with no statement new to any
+/// backend; a float constant's bits are asked for once per constant, so the
+/// call costs nothing that matters.
+#[no_mangle]
+pub extern "C" fn nova_rt_float_to_bits(v: f64) -> i64 {
+    v.to_bits() as i64
+}
+
 /// Format an `Int` as a string.
 ///
 /// The digits are written on the stack and copied once into GC memory,
@@ -1023,6 +1035,7 @@ pub fn symbols() -> Vec<(&'static str, *const u8)> {
         ("nova_rt_float_to_str", nova_rt_float_to_str as *const u8),
         ("nova_rt_float_fixed", nova_rt_float_fixed as *const u8),
         ("nova_rt_str_to_float", nova_rt_str_to_float as *const u8),
+        ("nova_rt_float_to_bits", nova_rt_float_to_bits as *const u8),
         ("nova_rt_bool_to_str", nova_rt_bool_to_str as *const u8),
         ("nova_rt_char_to_str", nova_rt_char_to_str as *const u8),
         ("nova_rt_alloc", nova_rt_alloc as *const u8),
diff --git a/crates/nova-typeck/src/check.rs b/crates/nova-typeck/src/check.rs
index b797f45995b81fe5602c8d70dd3e866b6f1b5933..3d040a4b113f77d9b9122e1023c0c8d029235a30 100644
--- a/crates/nova-typeck/src/check.rs
+++ b/crates/nova-typeck/src/check.rs
@@ -4504,6 +4504,7 @@ impl<'a> Checker<'a> {
             | Builtin::StrToLower
             | Builtin::FloatFixed
             | Builtin::StrToFloat
+            | Builtin::FloatToBits
             | Builtin::IntToChar
             | Builtin::StrAsBytes
             | Builtin::TestSelector
@@ -8966,6 +8967,7 @@ pub(crate) fn builtin_signature(builtin: Builtin) -> (Vec<Ty>, Ty) {
         Builtin::StrToUpper | Builtin::StrToLower => (vec![Ty::String], Ty::String),
         Builtin::FloatFixed => (vec![Ty::Float, Ty::Int], Ty::String),
         Builtin::StrToFloat => (vec![Ty::String], Ty::Float),
+        Builtin::FloatToBits => (vec![Ty::Float], Ty::Int),
         Builtin::IntToChar => (vec![Ty::Int], Ty::Char),
         Builtin::StrAsBytes => (vec![Ty::String], Ty::Bytes),
         Builtin::TestSelector => (vec![], Ty::Int),
diff --git a/crates/nova-typeck/src/check/fixes.rs b/crates/nova-typeck/src/check/fixes.rs
index dfff262193dec004eac17d3d551a04c93ac319f6..eb3e82f12a1bffee050b6ada9b4bb8c6279bff50 100644
--- a/crates/nova-typeck/src/check/fixes.rs
+++ b/crates/nova-typeck/src/check/fixes.rs
@@ -95,6 +95,7 @@ const STD_ONLY_PUBLIC_FORMS: &[(&str, &str)] = &[
         "a `String`'s `byte_len()`, `byte_at(i)` and `byte_slice(a, b)`, or \
          `bytes_from_string(s)` for a copy",
     ),
+    ("float_to_bits", "the method `to_bits`, as in `x.to_bits()`"),
 ];
 
 impl<'a> Checker<'a> {
diff --git a/std/core/lib.nova b/std/core/lib.nova
index c59e70c8ab4d2f17b2cb825104973c3a41e92e9a..b49df77e0cae54ef4cce923e300f1a6472fd58bf 100644
--- a/std/core/lib.nova
+++ b/std/core/lib.nova
@@ -1001,3 +1001,23 @@ pub fn char_from_int(n: Int) -> Option<Char> {
     if n >= 0xD800 && n <= 0xDFFF { return None }
     Some(int_to_char(n))
 }
+
+// A `Float`'s bits (5.0 spec §5.4).
+impl Float {
+    // A runtime call natively, a shim entry on WASM, each giving the bits of
+    // the value it is handed. A NaN made by arithmetic can itself differ by
+    // backend: `0.0 / 0.0` is `0xFFF8…` under `nova run` and `0x7FF8…` built
+    // `--release`, where LLVM folds it. There is no `from_bits` (5.0 spec §12
+    // item 23).
+    /// Returns the IEEE 754 bits of the number, as Rust's `f64::to_bits`
+    /// gives them, read as a signed `Int`: a negative number's sign bit makes
+    /// its bits negative.
+    ///
+    /// ```
+    /// assert_eq((1.0).to_bits(), 0x3FF0000000000000)
+    /// assert_eq((0.0).to_bits(), 0)
+    /// assert_eq((-0.0).to_bits(), -9223372036854775807 - 1) // the sign bit alone
+    /// assert_eq((5e-324).to_bits(), 1) // the smallest subnormal
+    /// ```
+    pub fn to_bits(self) -> Int { float_to_bits(self) }
+}
````

### Task 5: `parse_int`, `parse_int_radix` and `parse_float`

Spec §5.3, §5.4; decisions 6, 13, 19, 20, 28 and 30.

**Files:**
- Modify:
  - `crates/nova-resolver/src/lib.rs` (`StrIsFloat`; `STD_ONLY` 94)
  - `crates/nova-typeck/src/check.rs` (its signature),
    `crates/nova-typeck/src/check/fixes.rs` (the public forms of
    `str_is_float` and `str_to_float`)
  - `crates/nova-mir/src/lib.rs` (`RtFunc::StrIsFloat`),
    `crates/nova-mir/src/lower.rs`
  - `crates/nova-runtime/src/lib.rs` (`nova_rt_str_is_float`, its symbol)
  - `crates/nova-codegen-wasm/src/shim.rs` (the host entry, testing
    `FLOAT_TEXT`), `crates/nova-codegen-wasm/src/shim/float.js` (`-nan`'s
    sign)
  - `std/strings/lib.nova` (`digit_value`, `parse_digits`, `parse_int`,
    `parse_int_radix`, `parse_float`)
- Create: `tests/runtime/parse_numbers.nova`, `parse_numbers.stdout`,
  `tests/runtime/float_bits.nova`, `float_bits.stdout`
- Test: `std_tests.rs` (two, and rows in two); the signature test's row;
  `lower_tests.rs` (one); `ir_tests.rs` (one); the runtime's
  `str_is_float_is_rusts_grammar`; the shim's `float_from_is_rusts_parse`,
  which now compares a NaN's bits and the grammar, and two asserts in
  `helpers_come_only_with_their_host_functions`; `run_tests.rs`
  (`parse_numbers_runs`, `float_bits_runs`, the panic table's radix rows),
  `wasm_tests.rs` (both in `gate!`, the same rows), `index_complete.rs`

- [ ] **Step 1: The tests.** `patch_block t5-tests`. The patch holds:
  - `std_tests.rs`: the parsers' types; a program's trait `parse_int_radix`
    on `String`, E0105 with `x.parse_int_radix(…)` (decision 40);
    `str_is_float` kept from user code, and the public forms of
    `str_is_float` and `str_to_float`. The signature row; `parse_float` asks
    `str_is_float` once.
  - The runtime's test: fifteen texts Rust takes and fifteen it refuses. The
    shim's: `floatFrom`'s bits for every case, a NaN's included, and whether
    `FLOAT_TEXT` takes it, against Rust's `parse`.
  - **The programs:** `parse_numbers`, the tables of decision 6, each float
    shown with its bits, under `run`, `build` and GC stress; and
    `float_bits`, `to_bits` at every edge, as a signed `Int` and as hex, and
    each value's round trip through its text, without GC stress. Each with
    its `.stdout`, `gate!` entry, and the counts, 182 programs and 216
    files. `ir_tests.rs`: each literal `float_bits` writes is in its IR as
    the hex its golden prints.
  - The panic tables' radix rows: radix 1 and 37.
- [ ] **Step 2: Run, and see the tests fail.** First the build, which ends
  with `Finished`.
  - `T check::std_tests check::tests::builtin_signatures_are_what_the_std_call_sites_use str_is_float_is_rusts_grammar float_from_is_rusts_parse shim::tests::helpers_come_only_with_their_host_functions`.
    Expected: it does not compile,
    ``error[E0425]: cannot find function `nova_rt_str_is_float` in this scope``,
    twice, and
    ``error: could not compile `nova-runtime` (lib test) due to 2 previous errors``
    (decision 32).
  - `L parse_float`. Expected: it does not compile,
    ``error[E0599]: no variant, associated function, or constant named `StrIsFloat` found for enum `RtFunc` in the current scope``.
  - `cargo test --locked --workspace --test ir_tests -- float_constants`.
    Expected: `0 passed; 1 failed`,
    `float_constants_are_the_hex_that_to_bits_prints`, `float_bits` not
    checking (`ir_tests.rs:26:5`): E0014 "no method `parse_float` on type
    `String`".
  - `R parse_numbers_runs float_bits_runs each_std_panic_names_its_method`.
    Expected: `0 passed; 3 failed`: both programs refused with E0014 "no
    method `parse_int` on type `String`", "no method `parse_int_radix`…",
    "no method `parse_float`…"; the panic test at its first radix row,
    `"1".parse_int_radix(1)`.
  - `W gate::parse_numbers gate::float_bits each_std_panic_names_its_method_as_natively every_runtime_program_is_in_the_gate_or_left_out`.
    Expected: `1 passed; 3 failed`, the same.
  - `X`. Expected: `0 passed; 1 failed`.
- [ ] **Step 3: The code.** `patch_block t5-code`:
  - **The resolver, the checker, MIR:** `StrIsFloat`,
    `str_is_float(String) -> Bool`, `RtFunc::StrIsFloat`, `(str) -> i8`,
    `nova_rt_str_is_float` natively and the shim's `str_is_float` on WASM;
    `STD_ONLY` 94; the public forms, `s.parse_float()`.
  - **The runtime:** `nova_rt_str_is_float`, whether Rust's `parse` takes
    the text, in `symbols()`.
  - **The shim:** the host entry, `FLOAT_TEXT.test`, with the float helpers;
    `floatFrom("-nan")` is `-NaN`.
  - **`std/strings`:** `digit_value`; `parse_digits`, the value gathered
    negatively and checked before each step; `parse_int`; `parse_int_radix`,
    which panics outside 2 to 36; and `parse_float`, `str_is_float` then
    `str_to_float`. Each with a doc test.
- [ ] **Step 4: Run.** First the build, which ends with `Finished`.
  - `T check::std_tests check::tests::builtin_signatures_are_what_the_std_call_sites_use str_is_float_is_rusts_grammar float_from_is_rusts_parse shim::tests::helpers_come_only_with_their_host_functions`.
    Expected: every `test result` `ok`, 28 passing in all: `nova-typeck`'s
    25, the shim's 2 and the runtime's 1.
  - `L parse_float`. Expected: `1 passed; 0 failed`.
  - `cargo test --locked --workspace --test ir_tests -- float_constants`.
    Expected: `1 passed; 0 failed`.
  - `R parse_numbers_runs float_bits_runs each_std_panic_names_its_method`.
    Expected: `3 passed; 0 failed`.
  - `W gate::parse_numbers gate::float_bits each_std_panic_names_its_method_as_natively every_runtime_program_is_in_the_gate_or_left_out`.
    Expected: `4 passed; 0 failed`.
  - `X`. Expected: `1 passed; 0 failed`.
  - `S`, as in Task 1 (about 5 min 40 s). Expected: `2764 1 10`: Task 4's
    figures and Task 5's 9 tests, the one failure the IPv6 test.
  - `G`. Expected:
    `release golden: 144 passed, 9 excluded, Ubuntu clang version 18.1.3 (1ubuntu1)`
    and `1 passed`.
- [ ] **Step 5: The checks,** as the conventions list them.
- [ ] **Step 6: Commit**
  `std, runtime, shim: parse_int, parse_int_radix and parse_float`.

**Patch `t5-tests`:**

<!-- patch t5-tests -->

````diff
diff --git a/crates/nova-cli/tests/run_tests.rs b/crates/nova-cli/tests/run_tests.rs
index 18eef24e1b6ae8e74ded65f8f1b12a8f978f505f..c86b9e1771de89e804eaecc01eb22cdd4ed3068e 100644
--- a/crates/nova-cli/tests/run_tests.rs
+++ b/crates/nova-cli/tests/run_tests.rs
@@ -13062,6 +13062,19 @@ fn char_and_bytes_run() {
     golden_three_ways("char_and_bytes", true);
 }
 
+/// `parse_int`, `parse_int_radix` and `parse_float`'s table (5.0 spec §5.3),
+/// also under GC stress.
+#[test]
+fn parse_numbers_runs() {
+    golden_three_ways("parse_numbers", true);
+}
+
+/// `Float.to_bits` at every edge (5.0 spec §5.4).
+#[test]
+fn float_bits_runs() {
+    golden_three_ways("float_bits", false);
+}
+
 /// `iter` and `slice` on arrays, and `Vec`'s `to_array`, `from_array`, `at`
 /// and `last` (5.0 spec §5.5), also under GC stress.
 #[test]
@@ -13119,6 +13132,14 @@ const STD_PANICS: &[(&str, &str)] = &[
     ("fresh().last()", "Vec::last on an empty vector"),
     ("emptied().last()", "Vec::last on an empty vector"),
     ("emptied().at(0)", "Vec::at index out of range"),
+    (
+        "\"1\".parse_int_radix(1)",
+        "String::parse_int_radix radix must be from 2 to 36",
+    ),
+    (
+        "\"1\".parse_int_radix(37)",
+        "String::parse_int_radix radix must be from 2 to 36",
+    ),
 ];
 
 /// The program that prints `before`, then evaluates `expr`, which panics.
diff --git a/crates/nova-cli/tests/wasm_tests.rs b/crates/nova-cli/tests/wasm_tests.rs
index ac00927ea194c72ea60f638e3f58b23b1195469a..b0febe2a6d4e4e7ffa49e277f7d7e275c5b1cb88 100644
--- a/crates/nova-cli/tests/wasm_tests.rs
+++ b/crates/nova-cli/tests/wasm_tests.rs
@@ -1204,6 +1204,14 @@ fn each_std_panic_names_its_method_as_natively() {
         ("fresh().last()", "Vec::last on an empty vector"),
         ("emptied().last()", "Vec::last on an empty vector"),
         ("emptied().at(0)", "Vec::at index out of range"),
+        (
+            "\"1\".parse_int_radix(1)",
+            "String::parse_int_radix radix must be from 2 to 36",
+        ),
+        (
+            "\"1\".parse_int_radix(37)",
+            "String::parse_int_radix radix must be from 2 to 36",
+        ),
     ];
     for (i, (expr, message)) in cases.iter().enumerate() {
         let Some(node) = node() else { return };
@@ -2039,6 +2047,8 @@ mod gate {
         string_builder,
         string_builder_million,
         char_and_bytes,
+        parse_numbers,
+        float_bits,
     );
 
     /// The programs a test of their own in this module checks, by its name.
diff --git a/crates/nova-codegen-llvm/tests/ir_tests.rs b/crates/nova-codegen-llvm/tests/ir_tests.rs
index 28b9bf2760805c81e75f3929df7a5ea5b2714166..e9c583924051f2e1580fedc7b4a063889a79e8b4 100644
--- a/crates/nova-codegen-llvm/tests/ir_tests.rs
+++ b/crates/nova-codegen-llvm/tests/ir_tests.rs
@@ -183,6 +183,41 @@ fn floats_use_hex_constants_and_fp_ops() {
     assert_well_formed(&ir);
 }
 
+/// The `.ll` emitter writes each float constant as the hex of its bits,
+/// which is what `Float.to_bits` gives at run time (5.0 spec §5.4): every
+/// literal `tests/runtime/float_bits.nova` shows is in its IR as the hex its
+/// golden output prints, so N can write S0's text from `to_bits`.
+#[test]
+fn float_constants_are_the_hex_that_to_bits_prints() {
+    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/runtime");
+    let src = std::fs::read_to_string(root.join("float_bits.nova")).expect("the program");
+    let golden = std::fs::read_to_string(root.join("float_bits.stdout")).expect("its output");
+    let ir = ir_for(&src);
+    // The labels of the values written as literals; the others are computed.
+    let literals = [
+        "zero",
+        "one",
+        "smallest subnormal",
+        "largest subnormal",
+        "smallest normal",
+        "largest finite",
+        "tenth",
+    ];
+    for label in literals {
+        let line = golden
+            .lines()
+            .find(|l| l.starts_with(&format!("{label}: ")))
+            .unwrap_or_else(|| panic!("no `{label}` in the golden output"));
+        let hex = line.rsplit(' ').next().expect("a hex text");
+        assert!(hex.starts_with("0x") && hex.len() == 18, "{line}");
+        assert!(
+            ir.contains(&format!("double {hex}")),
+            "`{label}`: no `double {hex}` in the IR:\n{ir}"
+        );
+    }
+    assert_well_formed(&ir);
+}
+
 #[test]
 fn arrays_emit_bounds_check_and_element_addressing() {
     let ir = ir_for(
diff --git a/crates/nova-codegen-wasm/src/shim.rs b/crates/nova-codegen-wasm/src/shim.rs
index 4a6896615f3d8ebfaeefe84e58f553174cf8e4f3..2670e7ff4603cb2ebdd929378ec6424732f0438a 100644
--- a/crates/nova-codegen-wasm/src/shim.rs
+++ b/crates/nova-codegen-wasm/src/shim.rs
@@ -253,6 +253,10 @@ mod tests {
         let bits = shim(&["float_to_bits"]).expect("a shim");
         assert!(bits.contains("function floatBits"), "{bits}");
         assert!(bits.contains("  float_to_bits: ("), "{bits}");
+        // 5.0c's `str_is_float` tests `FLOAT_TEXT`, in the float helpers.
+        let is_float = shim(&["str_is_float"]).expect("a shim");
+        assert!(is_float.contains("const FLOAT_TEXT"), "{is_float}");
+        assert!(is_float.contains("  str_is_float: ("), "{is_float}");
     }
 
     /// `floatBits`, which `float_to_bits` calls, is Rust's `f64::to_bits`
@@ -396,9 +400,11 @@ mod tests {
 
     /// `floatFrom`, which `str_to_float` calls, is Rust's
     /// `str::parse::<f64>` with `unwrap_or(0.0)` on every case of its
-    /// grammar, valid and not (spec §6, §9.3). `str_to_float` is std's
-    /// alone, so no Nova program can hand it these texts: the test runs the
-    /// shim's own code under Node and compares the bits.
+    /// grammar, valid and not (spec §6, §9.3), and `FLOAT_TEXT`, which
+    /// `str_is_float` tests, is its grammar (5.0 spec §5.3). The test runs
+    /// the shim's own code under Node and compares the bits, a NaN's sign
+    /// included, which `Float.to_bits` shows (5.0 spec §5.4), and whether
+    /// each text is taken.
     #[test]
     fn float_from_is_rusts_parse() {
         let Some(mut node) = crate::test_node() else {
@@ -490,7 +496,7 @@ mod tests {
         let script = format!(
             "{FLOAT}\nconst view = new DataView(new ArrayBuffer(8));\n\
              for (const s of [{}]) {{\n  const v = floatFrom(s);\n  view.setFloat64(0, v);\n  \
-             console.log(Number.isNaN(v) ? \"NaN\" : view.getBigUint64(0).toString(16));\n}}\n",
+             console.log(view.getBigUint64(0).toString(16), FLOAT_TEXT.test(s));\n}}\n",
             list.join(", ")
         );
         let dir = std::env::temp_dir().join("nova-codegen-wasm-float-from");
@@ -508,12 +514,9 @@ mod tests {
         let got: Vec<&str> = got.lines().collect();
         assert_eq!(got.len(), cases.len(), "{got:?}");
         for (case, got) in cases.iter().zip(got) {
-            let v = case.parse::<f64>().unwrap_or(0.0);
-            let expected = if v.is_nan() {
-                "NaN".to_string()
-            } else {
-                format!("{:x}", v.to_bits())
-            };
+            let parsed = case.parse::<f64>();
+            let v = parsed.clone().unwrap_or(0.0);
+            let expected = format!("{:x} {}", v.to_bits(), parsed.is_ok());
             assert_eq!(got, expected, "floatFrom({case:?})");
         }
     }
diff --git a/crates/nova-driver/tests/index_complete.rs b/crates/nova-driver/tests/index_complete.rs
index f63883a1fb28424f27d66c6a7ad6e83b7ed7915f..2a60c0eea14629cfb0a532f4f911367d4bb66d2b 100644
--- a/crates/nova-driver/tests/index_complete.rs
+++ b/crates/nova-driver/tests/index_complete.rs
@@ -251,8 +251,8 @@ fn every_name_in_the_corpora_is_indexed() {
     // `patterns`, `patterns_async`, `match_effects` and `for_arrays`; 166
     // after 5.0c added `array_helpers`; 167 after `hidden_trait_methods`;
     // 168 after `char_and_bytes`; 170 after `string_builder` and
-    // `string_builder_million`.
-    assert_eq!(programs.len(), 180, "{programs:#?}");
+    // `string_builder_million`; 172 after `parse_numbers` and `float_bits`.
+    assert_eq!(programs.len(), 182, "{programs:#?}");
     let mut files = 0;
     let mut failures: Vec<String> = Vec::new();
     for (k, path) in programs.iter().enumerate() {
@@ -279,11 +279,11 @@ fn every_name_in_the_corpora_is_indexed() {
             every_name_is_covered(&a, file, &mut failures);
         }
     }
-    // 9 + 170 + 3 files of user code, 05-geometry's library's 2, the 8
+    // 9 + 172 + 3 files of user code, 05-geometry's library's 2, the 8
     // modules 07-fullstack-blog's client reaches from its `main.nova` (not
     // the server's 4, which only `server.nova` and `lib.nova` reach), and
     // native std's 22, `std/ui/ssr` among them.
-    assert_eq!(files, 214);
+    assert_eq!(files, 216);
     failures.sort();
     failures.dedup();
     assert!(
diff --git a/crates/nova-mir/tests/lower_tests.rs b/crates/nova-mir/tests/lower_tests.rs
index dcac4dd57a421764888eb6748258bc7ebb99e97b..c1a25a031c12f48824fadfc8ffd3021fa6ca83aa 100644
--- a/crates/nova-mir/tests/lower_tests.rs
+++ b/crates/nova-mir/tests/lower_tests.rs
@@ -2446,6 +2446,19 @@ fn to_bits_is_one_runtime_call() {
     );
 }
 
+/// `String.parse_float` asks `str_is_float` once (spec §5.3).
+#[test]
+fn parse_float_asks_str_is_float_once() {
+    let mir = mir_for(
+        "fn main() {\n    let f = \"2.5\".parse_float()\n    println(\"${f.is_some()}\")\n}",
+    );
+    let name = std_fn(&mir, "parse_float");
+    assert_eq!(
+        count_rt(&stmts_of(&mir, &name), nova_mir::RtFunc::StrIsFloat),
+        1
+    );
+}
+
 /// `impl<T> [T]`'s methods monomorphise per element type, as any generic
 /// impl's do (spec §5.5).
 #[test]
diff --git a/crates/nova-runtime/src/lib.rs b/crates/nova-runtime/src/lib.rs
index d795ef611136e17c8c4ad46f58592479b4a91234..b17301f2f9daa1de333cdcee33e86da15629cb1c 100644
--- a/crates/nova-runtime/src/lib.rs
+++ b/crates/nova-runtime/src/lib.rs
@@ -2024,6 +2024,28 @@ mod tests {
         }
     }
 
+    /// `str_is_float` says yes exactly where Rust's parser does, so
+    /// `parse_float` tells a refused text from a `0.0` (5.0 spec §5.3).
+    #[test]
+    fn str_is_float_is_rusts_grammar() {
+        let taken = [
+            "0", "-0", "+1", "1.", ".5", "1e5", "1E-5", "1e+5", "inf", "-INF", "infinity", "NaN",
+            "+nan", "1e400", "4.9e-324",
+        ];
+        let refused = [
+            "", "+", "-", ".", "e5", "1e", "1e+", "1.5.2", " 1", "1 ", "1_000", "0x1A", "infinit",
+            "nanx", "--1",
+        ];
+        unsafe {
+            for s in taken {
+                assert_eq!(nova_rt_str_is_float(make_str(s)), 1, "{s:?} is a float");
+            }
+            for s in refused {
+                assert_eq!(nova_rt_str_is_float(make_str(s)), 0, "{s:?} is not a float");
+            }
+        }
+    }
+
     /// `float_to_bits` is `f64::to_bits`, read as an `i64` (5.0 spec §5.4).
     #[test]
     fn float_to_bits_is_rusts_to_bits() {
diff --git a/crates/nova-typeck/src/check.rs b/crates/nova-typeck/src/check.rs
index 3d040a4b113f77d9b9122e1023c0c8d029235a30..16008cd6d146d8094c5ff5e3f512b1bbc739f3fc 100644
--- a/crates/nova-typeck/src/check.rs
+++ b/crates/nova-typeck/src/check.rs
@@ -17098,6 +17098,10 @@ mod tests {
                     (vec![Ty::String], Ty::Float),
                     "`str_to_float(span)` in `std/json`'s `scan_number`",
                 ),
+                Builtin::StrIsFloat => (
+                    (vec![Ty::String], Ty::Bool),
+                    "`str_is_float(self)` in `String::parse_float`",
+                ),
                 Builtin::FloatToBits => (
                     (vec![Ty::Float], Ty::Int),
                     "`float_to_bits(self)` in `Float::to_bits`",
diff --git a/crates/nova-typeck/src/check/std_tests.rs b/crates/nova-typeck/src/check/std_tests.rs
index 2d170f1b46256ca1dcdd03b1992e950aae5a4b2d..42bd48da5cd73b0f1356a7e410f7ffb06068f048 100644
--- a/crates/nova-typeck/src/check/std_tests.rs
+++ b/crates/nova-typeck/src/check/std_tests.rs
@@ -155,6 +155,7 @@ fn the_new_builtins_are_not_visible_to_a_program() {
     for (call, name) in [
         ("int_to_char(65)", "int_to_char"),
         ("str_as_bytes(\"a\")", "str_as_bytes"),
+        ("str_is_float(\"1\")", "str_is_float"),
         ("float_to_bits(1.0)", "float_to_bits"),
     ] {
         let e = errors(&format!("fn main() {{\n    let x = {call}\n}}\n"));
@@ -228,6 +229,19 @@ fn to_bits_has_the_specs_type() {
     assert_eq!(local_ty(&r, "fb"), Ty::Int);
 }
 
+/// The parsers' types (spec §5.3).
+#[test]
+fn the_parsers_have_the_specs_types() {
+    let r = checked(
+        "fn main() {\n    let pi = \"1\".parse_int()\n    \
+         let pr = \"1\".parse_int_radix(16)\n    let pf = \"1\".parse_float()\n}\n",
+    );
+    assert!(r.diagnostics.is_empty(), "{:#?}", r.diagnostics);
+    assert!(option(&local_ty(&r, "pi"), Ty::Int));
+    assert!(option(&local_ty(&r, "pr"), Ty::Int));
+    assert!(option(&local_ty(&r, "pf"), Ty::Float));
+}
+
 /// `push` and `clear` take `mut self`, as `Vec`'s do.
 #[test]
 fn a_builder_mutates_only_through_a_mutable_binding() {
@@ -271,6 +285,8 @@ fn a_std_only_builtin_named_by_a_program_gets_its_public_form() {
         ("int_to_char(65)", "`char_from_int(n)`"),
         ("str_from_chars(['a'])", "`string_from_chars(cs)`"),
         ("str_as_bytes(\"a\")", "`byte_at(i)`"),
+        ("str_is_float(\"1\")", "`s.parse_float()`"),
+        ("str_to_float(\"1\")", "`s.parse_float()`"),
         ("float_to_bits(1.0)", "`x.to_bits()`"),
     ] {
         let e = errors(&format!("fn main() {{\n    let x = {call}\n}}\n"));
@@ -500,3 +516,24 @@ fn a_trait_method_hidden_by_floats_to_bits_is_e0105() {
         ["std's inherent method `to_bits` on `Float` hides `Bits::to_bits`"]
     );
 }
+
+/// The parsers, std's since Task 5, hide a program's trait methods of their
+/// names: E0105, its call shown with arguments where std's takes them.
+#[test]
+fn a_trait_method_hidden_by_the_parsers_is_e0105() {
+    let src = "trait Parse {\n    fn parse_int_radix(self, r: Int) -> Option<Int>\n}\n\
+               impl Parse for String {\n    \
+               fn parse_int_radix(self, r: Int) -> Option<Int> { Some(r) }\n}\n\
+               fn main() {}\n";
+    let w = warnings(src);
+    let d = w.iter().find(|d| d.code == "E0105").expect("an E0105");
+    assert_eq!(
+        d.message,
+        "std's inherent method `parse_int_radix` on `String` hides `Parse::parse_int_radix`"
+    );
+    assert!(
+        d.labels.iter().any(|l| l.primary
+            && l.message == "`x.parse_int_radix(…)` on a `String` runs std's method, not this one"),
+        "{d:#?}"
+    );
+}
diff --git a/tests/runtime/float_bits.nova b/tests/runtime/float_bits.nova
new file mode 100644
index 0000000000000000000000000000000000000000..21f612eb37adfa8b5343ef0c8fe1b2c9613f1c37
--- /dev/null
+++ b/tests/runtime/float_bits.nova
@@ -0,0 +1,60 @@
+// `Float.to_bits` (5.0 spec §5.4): `0.0`, `-0.0`, `1.0`, the smallest
+// subnormal, the largest finite, both infinities and a NaN, as a signed `Int`
+// and as the hex the `.ll` emitter writes for a float constant, the same on
+// every backend, and the round trip of a value through its text and
+// `parse_float`. Its NaNs come from `parse_float`, whose bits are fixed; a
+// NaN made by arithmetic can differ by backend. `ir_tests.rs` checks the
+// emitter writes these hex texts.
+
+fn hex64(n: Int) -> String {
+    let digits = "0123456789ABCDEF".chars()
+    let mut out = [' '; 16]
+    for i in 0..16 { out[15 - i] = digits[(n >> (4 * i)) & 15] }
+    string_from_chars(out)
+}
+
+fn show(label: String, v: Float) {
+    let bits = v.to_bits()
+    println("${label}: ${bits} 0x${hex64(bits)}")
+}
+
+fn main() {
+    show("zero", 0.0)
+    show("negative zero", -0.0)
+    show("one", 1.0)
+    show("minus one", -1.0)
+    show("smallest subnormal", 5e-324)
+    show("largest subnormal", 2.2250738585072009e-308)
+    show("smallest normal", 2.2250738585072014e-308)
+    show("largest finite", 1.7976931348623157e308)
+    show("tenth", 0.1)
+    show("infinity", "inf".parse_float().unwrap())
+    show("negative infinity", "-inf".parse_float().unwrap())
+    show("NaN", "nan".parse_float().unwrap())
+    show("negative NaN", "-nan".parse_float().unwrap())
+    let x = 1.0
+    show("computed", x / 3.0)
+    show("negated", 0.0 - x)
+
+    // Each value's text, as interpolation writes it, parses back to the same
+    // bits, on every backend: `parse_float` and the text agree.
+    let values = [
+        0.0,
+        -0.0,
+        1.0,
+        0.1,
+        x / 3.0,
+        2.0 / 3.0,
+        1e300,
+        1e-300,
+        5e-324,
+        1.7976931348623157e308,
+        123456.789,
+        0.1 + 0.2,
+        "inf".parse_float().unwrap(),
+        "-inf".parse_float().unwrap(),
+    ]
+    let mut same = 0
+    for v in values { if "${v}".parse_float().unwrap().to_bits() == v.to_bits() { same += 1 } }
+    println("round trips through text: ${same} of ${values.len()}")
+}
diff --git a/tests/runtime/float_bits.stdout b/tests/runtime/float_bits.stdout
new file mode 100644
index 0000000000000000000000000000000000000000..26cfe010a29dc0df66be070fb7ae5723fef31dca
--- /dev/null
+++ b/tests/runtime/float_bits.stdout
@@ -0,0 +1,16 @@
+zero: 0 0x0000000000000000
+negative zero: -9223372036854775808 0x8000000000000000
+one: 4607182418800017408 0x3FF0000000000000
+minus one: -4616189618054758400 0xBFF0000000000000
+smallest subnormal: 1 0x0000000000000001
+largest subnormal: 4503599627370495 0x000FFFFFFFFFFFFF
+smallest normal: 4503599627370496 0x0010000000000000
+largest finite: 9218868437227405311 0x7FEFFFFFFFFFFFFF
+tenth: 4591870180066957722 0x3FB999999999999A
+infinity: 9218868437227405312 0x7FF0000000000000
+negative infinity: -4503599627370496 0xFFF0000000000000
+NaN: 9221120237041090560 0x7FF8000000000000
+negative NaN: -2251799813685248 0xFFF8000000000000
+computed: 4599676419421066581 0x3FD5555555555555
+negated: -4616189618054758400 0xBFF0000000000000
+round trips through text: 14 of 14
diff --git a/tests/runtime/parse_numbers.nova b/tests/runtime/parse_numbers.nova
new file mode 100644
index 0000000000000000000000000000000000000000..01c9533f02b250959d2b0aa8c7ad2b1cfd3b4e29
--- /dev/null
+++ b/tests/runtime/parse_numbers.nova
@@ -0,0 +1,170 @@
+// `parse_int`, `parse_int_radix` and `parse_float` (5.0 spec §5.3), one table
+// each, run natively and on WASM: signs, both ends of `Int`, overflow, `_` and
+// spaces refused, each radix's digits, exponents, `inf` and `nan`, and the
+// empty string. A float's value is shown with its bits, so the two targets
+// must agree on its rounding exactly.
+
+fn hex64(n: Int) -> String {
+    let digits = "0123456789ABCDEF".chars()
+    let mut out = [' '; 16]
+    for i in 0..16 { out[15 - i] = digits[(n >> (4 * i)) & 15] }
+    string_from_chars(out)
+}
+
+fn int_case(s: String) {
+    match s.parse_int() {
+        Some(n) => println("int [${s}] = ${n}")
+        None => println("int [${s}] = None")
+    }
+}
+
+fn radix_case(s: String, radix: Int) {
+    match s.parse_int_radix(radix) {
+        Some(n) => println("radix ${radix} [${s}] = ${n}")
+        None => println("radix ${radix} [${s}] = None")
+    }
+}
+
+fn float_case(s: String) {
+    match s.parse_float() {
+        Some(v) => println("float [${s}] = ${v} 0x${hex64(v.to_bits())}")
+        None => println("float [${s}] = None")
+    }
+}
+
+fn main() {
+    for s in [
+        "0",
+        "7",
+        "-7",
+        "+7",
+        "007",
+        "-0",
+        "+0",
+        "123456789",
+        "9223372036854775807",
+        "9223372036854775808",
+        "-9223372036854775808",
+        "-9223372036854775809",
+        "-0009223372036854775808",
+        "+0009223372036854775807",
+        "99999999999999999999",
+        "",
+        "+",
+        "-",
+        "--1",
+        "+-1",
+        " 1",
+        "1 ",
+        "1_000",
+        "1.0",
+        "1e3",
+        "0x10",
+        "١٢",
+        "12a",
+    ] {
+        int_case(s)
+    }
+
+    radix_case("1010", 2)
+    radix_case("-1010", 2)
+    radix_case("2", 2)
+    radix_case("777", 8)
+    radix_case("8", 8)
+    radix_case("ff", 16)
+    radix_case("FF", 16)
+    radix_case("fF", 16)
+    radix_case("g", 16)
+    radix_case("0x1F", 16)
+    radix_case("7fffffffffffffff", 16)
+    radix_case("8000000000000000", 16)
+    radix_case("-8000000000000000", 16)
+    radix_case("-8000000000000001", 16)
+    radix_case("zz", 36)
+    radix_case("ZZ", 36)
+    radix_case("1y2p0ij32e8e7", 36)
+    radix_case("1y2p0ij32e8e8", 36)
+    radix_case("y", 35)
+    radix_case("Z", 35)
+    radix_case("Z", 36)
+    radix_case("111111111111111111111111111111111111111111111111111111111111111", 2)
+    radix_case("1000000000000000000000000000000000000000000000000000000000000000", 2)
+    radix_case("-1000000000000000000000000000000000000000000000000000000000000000", 2)
+    radix_case("/", 10)
+    radix_case(":", 10)
+    radix_case("@", 16)
+    radix_case("[", 36)
+    radix_case("`", 36)
+    radix_case("{", 36)
+    radix_case("9", 10)
+    radix_case("a", 10)
+    radix_case("", 16)
+    radix_case("-", 16)
+
+    for s in [
+        "0",
+        "-0",
+        "+0",
+        "1",
+        "-1",
+        "2.5",
+        "1.",
+        ".5",
+        "-.5",
+        "+.5e-3",
+        "1e5",
+        "1E5",
+        "1e+5",
+        "1e-5",
+        "1.5e10",
+        "0.1",
+        "9007199254740993",
+        "1.7976931348623157e308",
+        "1.7976931348623159e308",
+        "1e400",
+        "4.9e-324",
+        "2.4703282292062327e-324",
+        "2.4703282292062328e-324",
+        "1e-400",
+        "1e0001",
+        "1e99999999999999999999",
+        "1e-99999999999999999999",
+        "-0e0",
+        "1.e5",
+        "00.5",
+        "inf",
+        "INF",
+        "-inf",
+        "+Infinity",
+        "infinity",
+        "nan",
+        "NaN",
+        "-nan",
+        "+nan",
+        "-NaN",
+        "-infinity",
+        "INFINITY",
+        "",
+        "+",
+        "-",
+        ".",
+        "e5",
+        ".e1",
+        "1e",
+        "1e+",
+        "1.5.2",
+        " 1",
+        "1 ",
+        "1_000",
+        "0x1A",
+        "infinit",
+        "nanx",
+        "infinityx",
+        "nan(1)",
+        "1e+-5",
+        "1.0f64",
+        "١",
+    ] {
+        float_case(s)
+    }
+}
diff --git a/tests/runtime/parse_numbers.stdout b/tests/runtime/parse_numbers.stdout
new file mode 100644
index 0000000000000000000000000000000000000000..20dc98d98bf2a6cdc0736baf48045e82a633b093
--- /dev/null
+++ b/tests/runtime/parse_numbers.stdout
@@ -0,0 +1,124 @@
+int [0] = 0
+int [7] = 7
+int [-7] = -7
+int [+7] = 7
+int [007] = 7
+int [-0] = 0
+int [+0] = 0
+int [123456789] = 123456789
+int [9223372036854775807] = 9223372036854775807
+int [9223372036854775808] = None
+int [-9223372036854775808] = -9223372036854775808
+int [-9223372036854775809] = None
+int [-0009223372036854775808] = -9223372036854775808
+int [+0009223372036854775807] = 9223372036854775807
+int [99999999999999999999] = None
+int [] = None
+int [+] = None
+int [-] = None
+int [--1] = None
+int [+-1] = None
+int [ 1] = None
+int [1 ] = None
+int [1_000] = None
+int [1.0] = None
+int [1e3] = None
+int [0x10] = None
+int [١٢] = None
+int [12a] = None
+radix 2 [1010] = 10
+radix 2 [-1010] = -10
+radix 2 [2] = None
+radix 8 [777] = 511
+radix 8 [8] = None
+radix 16 [ff] = 255
+radix 16 [FF] = 255
+radix 16 [fF] = 255
+radix 16 [g] = None
+radix 16 [0x1F] = None
+radix 16 [7fffffffffffffff] = 9223372036854775807
+radix 16 [8000000000000000] = None
+radix 16 [-8000000000000000] = -9223372036854775808
+radix 16 [-8000000000000001] = None
+radix 36 [zz] = 1295
+radix 36 [ZZ] = 1295
+radix 36 [1y2p0ij32e8e7] = 9223372036854775807
+radix 36 [1y2p0ij32e8e8] = None
+radix 35 [y] = 34
+radix 35 [Z] = None
+radix 36 [Z] = 35
+radix 2 [111111111111111111111111111111111111111111111111111111111111111] = 9223372036854775807
+radix 2 [1000000000000000000000000000000000000000000000000000000000000000] = None
+radix 2 [-1000000000000000000000000000000000000000000000000000000000000000] = -9223372036854775808
+radix 10 [/] = None
+radix 10 [:] = None
+radix 16 [@] = None
+radix 36 [[] = None
+radix 36 [`] = None
+radix 36 [{] = None
+radix 10 [9] = 9
+radix 10 [a] = None
+radix 16 [] = None
+radix 16 [-] = None
+float [0] = 0 0x0000000000000000
+float [-0] = -0 0x8000000000000000
+float [+0] = 0 0x0000000000000000
+float [1] = 1 0x3FF0000000000000
+float [-1] = -1 0xBFF0000000000000
+float [2.5] = 2.5 0x4004000000000000
+float [1.] = 1 0x3FF0000000000000
+float [.5] = 0.5 0x3FE0000000000000
+float [-.5] = -0.5 0xBFE0000000000000
+float [+.5e-3] = 0.0005 0x3F40624DD2F1A9FC
+float [1e5] = 100000 0x40F86A0000000000
+float [1E5] = 100000 0x40F86A0000000000
+float [1e+5] = 100000 0x40F86A0000000000
+float [1e-5] = 0.00001 0x3EE4F8B588E368F1
+float [1.5e10] = 15000000000 0x420BF08EB0000000
+float [0.1] = 0.1 0x3FB999999999999A
+float [9007199254740993] = 9007199254740992 0x4340000000000000
+float [1.7976931348623157e308] = 179769313486231570000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000 0x7FEFFFFFFFFFFFFF
+float [1.7976931348623159e308] = inf 0x7FF0000000000000
+float [1e400] = inf 0x7FF0000000000000
+float [4.9e-324] = 0.000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000005 0x0000000000000001
+float [2.4703282292062327e-324] = 0 0x0000000000000000
+float [2.4703282292062328e-324] = 0.000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000005 0x0000000000000001
+float [1e-400] = 0 0x0000000000000000
+float [1e0001] = 10 0x4024000000000000
+float [1e99999999999999999999] = inf 0x7FF0000000000000
+float [1e-99999999999999999999] = 0 0x0000000000000000
+float [-0e0] = -0 0x8000000000000000
+float [1.e5] = 100000 0x40F86A0000000000
+float [00.5] = 0.5 0x3FE0000000000000
+float [inf] = inf 0x7FF0000000000000
+float [INF] = inf 0x7FF0000000000000
+float [-inf] = -inf 0xFFF0000000000000
+float [+Infinity] = inf 0x7FF0000000000000
+float [infinity] = inf 0x7FF0000000000000
+float [nan] = NaN 0x7FF8000000000000
+float [NaN] = NaN 0x7FF8000000000000
+float [-nan] = NaN 0xFFF8000000000000
+float [+nan] = NaN 0x7FF8000000000000
+float [-NaN] = NaN 0xFFF8000000000000
+float [-infinity] = -inf 0xFFF0000000000000
+float [INFINITY] = inf 0x7FF0000000000000
+float [] = None
+float [+] = None
+float [-] = None
+float [.] = None
+float [e5] = None
+float [.e1] = None
+float [1e] = None
+float [1e+] = None
+float [1.5.2] = None
+float [ 1] = None
+float [1 ] = None
+float [1_000] = None
+float [0x1A] = None
+float [infinit] = None
+float [nanx] = None
+float [infinityx] = None
+float [nan(1)] = None
+float [1e+-5] = None
+float [1.0f64] = None
+float [١] = None
````

**Patch `t5-code`:**

<!-- patch t5-code -->

````diff
diff --git a/crates/nova-codegen-wasm/src/shim.rs b/crates/nova-codegen-wasm/src/shim.rs
index 2670e7ff4603cb2ebdd929378ec6424732f0438a..328d5c95d90fc315403f4c8a7b37aee1ce016c5a 100644
--- a/crates/nova-codegen-wasm/src/shim.rs
+++ b/crates/nova-codegen-wasm/src/shim.rs
@@ -72,6 +72,7 @@ fn host(name: &str) -> Result<&'static str> {
         "float_to_str" => "  float_to_str: (v) => string(floatText(v)),\n",
         "float_fixed" => "  float_fixed: (v, places) => string(floatFixed(v, places)),\n",
         "str_to_float" => "  str_to_float: (s) => floatFrom(text(s)),\n",
+        "str_is_float" => "  str_is_float: (s) => (FLOAT_TEXT.test(text(s)) ? 1 : 0),\n",
         "float_to_bits" => "  float_to_bits: (v) => floatBits(v),\n",
         "str_to_upper" => "  str_to_upper: (s) => string(text(s).toUpperCase()),\n",
         "str_to_lower" => "  str_to_lower: (s) => string(text(s).toLowerCase()),\n",
@@ -190,6 +191,7 @@ pub fn shim<S: AsRef<str>>(imports: &[S]) -> Result<String> {
         "float_to_str",
         "float_fixed",
         "str_to_float",
+        "str_is_float",
         "float_to_bits",
     ]) {
         js.push_str(FLOAT);
diff --git a/crates/nova-codegen-wasm/src/shim/float.js b/crates/nova-codegen-wasm/src/shim/float.js
index 7951da9c255bd1edfe658561eafb96e2cbc251e2..87e0bd1f71fa7306412b3a775110fa6da313d157 100644
--- a/crates/nova-codegen-wasm/src/shim/float.js
+++ b/crates/nova-codegen-wasm/src/shim/float.js
@@ -86,13 +86,14 @@ function floatFixed(v, places) {
 
 // Rust's `str::parse::<f64>`, or 0.0 where it fails (nova_rt_str_to_float's
 // `unwrap_or(0.0)`): its grammar exactly, then `Number`, which rounds
-// correctly as Rust does.
+// correctly as Rust does. A `-` before `nan` sets the NaN's sign bit, as
+// Rust's does, which `Float.to_bits` shows (5.0 spec §5.4).
 const FLOAT_TEXT = /^[+-]?(?:inf|infinity|nan|(?:\d+\.?\d*|\.\d+)(?:e[+-]?\d+)?)$/i;
 function floatFrom(s) {
   if (!FLOAT_TEXT.test(s)) return 0;
   const body = s.replace(/^[+-]/, "").toLowerCase();
   if (body === "inf" || body === "infinity") return s.startsWith("-") ? -Infinity : Infinity;
-  if (body === "nan") return NaN;
+  if (body === "nan") return s.startsWith("-") ? -NaN : NaN;
   return Number(s);
 }
 
diff --git a/crates/nova-mir/src/lib.rs b/crates/nova-mir/src/lib.rs
index 4da5a1c8f6ae347bae0f3365f207cb3ee3a17d50..792c43391169b9ca1fddd2a83b7ddd8b8b9e71c7 100644
--- a/crates/nova-mir/src/lib.rs
+++ b/crates/nova-mir/src/lib.rs
@@ -198,6 +198,9 @@ rt_funcs! {
     FloatFixed,
     /// `(str) -> f64` — decimal text parsed as a `Float`.
     StrToFloat,
+    /// `(str) -> i8` — whether the text is one Rust's `f64::from_str`
+    /// accepts, so `StrToFloat` gives its value rather than its `0.0`.
+    StrIsFloat,
     /// `(f64) -> i64` — the IEEE 754 bits of the value, as `f64::to_bits`.
     FloatToBits,
     /// `(i8) -> str`
@@ -576,6 +579,7 @@ impl RtFunc {
             RtFunc::FloatToStr => "nova_rt_float_to_str",
             RtFunc::FloatFixed => "nova_rt_float_fixed",
             RtFunc::StrToFloat => "nova_rt_str_to_float",
+            RtFunc::StrIsFloat => "nova_rt_str_is_float",
             RtFunc::FloatToBits => "nova_rt_float_to_bits",
             RtFunc::BoolToStr => "nova_rt_bool_to_str",
             RtFunc::CharToStr => "nova_rt_char_to_str",
@@ -685,6 +689,7 @@ impl RtFunc {
             RtFunc::FloatToStr => (vec![MirTy::F64], MirTy::Ptr),
             RtFunc::FloatFixed => (vec![MirTy::F64, MirTy::I64], MirTy::Ptr),
             RtFunc::StrToFloat => (vec![MirTy::Ptr], MirTy::F64),
+            RtFunc::StrIsFloat => (vec![MirTy::Ptr], MirTy::I8),
             RtFunc::FloatToBits => (vec![MirTy::F64], MirTy::I64),
             RtFunc::BoolToStr => (vec![MirTy::I8], MirTy::Ptr),
             RtFunc::CharToStr => (vec![MirTy::I64], MirTy::Ptr),
@@ -948,6 +953,7 @@ impl RtFunc {
             RtFunc::FloatToStr => WasmRuntime::Host("float_to_str"),
             RtFunc::FloatFixed => WasmRuntime::Host("float_fixed"),
             RtFunc::StrToFloat => WasmRuntime::Host("str_to_float"),
+            RtFunc::StrIsFloat => WasmRuntime::Host("str_is_float"),
             RtFunc::FloatToBits => WasmRuntime::Host("float_to_bits"),
             RtFunc::BoolToStr => WasmRuntime::Nova("rt_bool_to_str"),
             RtFunc::CharToStr => WasmRuntime::Nova("rt_char_to_str"),
diff --git a/crates/nova-mir/src/lower.rs b/crates/nova-mir/src/lower.rs
index 26e91e9aae5f1e6fef55504e6c3be608390570c7..fb82ee5a60dba640279501c040c3bb14ddb227f7 100644
--- a/crates/nova-mir/src/lower.rs
+++ b/crates/nova-mir/src/lower.rs
@@ -713,6 +713,7 @@ impl<'a> Lowerer<'a> {
                     Builtin::StrToLower => Lowering::Runtime(RtFunc::StrToLower),
                     Builtin::FloatFixed => Lowering::Runtime(RtFunc::FloatFixed),
                     Builtin::StrToFloat => Lowering::Runtime(RtFunc::StrToFloat),
+                    Builtin::StrIsFloat => Lowering::Runtime(RtFunc::StrIsFloat),
                     Builtin::FloatToBits => Lowering::Runtime(RtFunc::FloatToBits),
                     Builtin::IntToChar => Lowering::Reinterpret,
                     Builtin::StrAsBytes => Lowering::Reinterpret,
diff --git a/crates/nova-resolver/src/lib.rs b/crates/nova-resolver/src/lib.rs
index b56b41a0ac9e3e00cef7f3c56631c47e197abd0a..387fec6f6276ae5606fcb8ca04c27b584f3e987d 100644
--- a/crates/nova-resolver/src/lib.rs
+++ b/crates/nova-resolver/src/lib.rs
@@ -195,6 +195,15 @@ builtins! {
     /// replaced it on 2026-10-03. Std-only, so
     /// `str_to_float` is not a reserved word in user code.
     StrToFloat,
+    /// `str_is_float(s: String) -> Bool` — whether `s` is text Rust's
+    /// `f64::from_str` accepts: an optional sign, then digits with an
+    /// optional `.` and exponent, or `inf`, `infinity` or `nan` in any case.
+    /// Backs `std/strings`' `String.parse_float`, which pairs it with
+    /// [`Builtin::StrToFloat`]: that one returns `0.0` on text it refuses, so
+    /// the pair says whether a `0.0` is the value or the refusal. On WASM a
+    /// shim entry, by the same grammar the shim's `str_to_float` already
+    /// tests (5.0 spec §5.3). Std-only.
+    StrIsFloat,
     /// `float_to_bits(v: Float) -> Int` — the IEEE 754 bits of `v`, as Rust's
     /// `f64::to_bits`, read as a signed `Int`. Backs `std/core`'s
     /// `Float.to_bits`, which N's `.ll` emitter needs to write each float
@@ -1060,6 +1069,7 @@ impl Builtin {
             Builtin::StrToLower => "str_to_lower",
             Builtin::FloatFixed => "float_fixed",
             Builtin::StrToFloat => "str_to_float",
+            Builtin::StrIsFloat => "str_is_float",
             Builtin::FloatToBits => "float_to_bits",
             Builtin::IntToChar => "int_to_char",
             Builtin::StrAsBytes => "str_as_bytes",
@@ -1186,7 +1196,7 @@ impl Builtin {
     /// consecutive review rounds (see the Phase 2.2b whole-branch review),
     /// because the roster is duplicated information that only this array
     /// needs to stay exact.
-    pub const STD_ONLY: [Builtin; 93] = [
+    pub const STD_ONLY: [Builtin; 94] = [
         Builtin::StrCmp,
         Builtin::StrHash,
         Builtin::CharToInt,
@@ -1200,6 +1210,7 @@ impl Builtin {
         Builtin::StrToLower,
         Builtin::FloatFixed,
         Builtin::StrToFloat,
+        Builtin::StrIsFloat,
         Builtin::FloatToBits,
         Builtin::IntToChar,
         Builtin::StrAsBytes,
diff --git a/crates/nova-runtime/src/lib.rs b/crates/nova-runtime/src/lib.rs
index b17301f2f9daa1de333cdcee33e86da15629cb1c..12380288ff2fc3b161af16ea5648699a9b9a7c4e 100644
--- a/crates/nova-runtime/src/lib.rs
+++ b/crates/nova-runtime/src/lib.rs
@@ -870,6 +870,21 @@ pub unsafe extern "C" fn nova_rt_str_to_float(s: *const NovaStr) -> f64 {
     as_str(s).parse::<f64>().unwrap_or(0.0)
 }
 
+/// Whether `s` is text `str::parse::<f64>` accepts, `1` or `0`.
+///
+/// The other half of `std/strings`' `String.parse_float`: where this says
+/// yes, [`nova_rt_str_to_float`] gives the value, and where it says no, that
+/// `0.0` is a refusal (5.0 spec §5.3). So the grammar, and the value, are
+/// Rust's on both sides of the pair, and the shim's `str_is_float` tests the
+/// same grammar on WASM.
+///
+/// # Safety
+/// `s` must point to a valid `NovaStr`.
+#[no_mangle]
+pub unsafe extern "C" fn nova_rt_str_is_float(s: *const NovaStr) -> i8 {
+    i8::from(as_str(s).parse::<f64>().is_ok())
+}
+
 /// The IEEE 754 bits of `v`, as `f64::to_bits`, read as a signed `Int`.
 ///
 /// Backs `std/core`'s `Float.to_bits` (5.0 spec §5.4). A runtime call rather
@@ -1035,6 +1050,7 @@ pub fn symbols() -> Vec<(&'static str, *const u8)> {
         ("nova_rt_float_to_str", nova_rt_float_to_str as *const u8),
         ("nova_rt_float_fixed", nova_rt_float_fixed as *const u8),
         ("nova_rt_str_to_float", nova_rt_str_to_float as *const u8),
+        ("nova_rt_str_is_float", nova_rt_str_is_float as *const u8),
         ("nova_rt_float_to_bits", nova_rt_float_to_bits as *const u8),
         ("nova_rt_bool_to_str", nova_rt_bool_to_str as *const u8),
         ("nova_rt_char_to_str", nova_rt_char_to_str as *const u8),
diff --git a/crates/nova-typeck/src/check.rs b/crates/nova-typeck/src/check.rs
index 16008cd6d146d8094c5ff5e3f512b1bbc739f3fc..b865f62915d1e52ec81e4967d7ae3d361873a689 100644
--- a/crates/nova-typeck/src/check.rs
+++ b/crates/nova-typeck/src/check.rs
@@ -4504,6 +4504,7 @@ impl<'a> Checker<'a> {
             | Builtin::StrToLower
             | Builtin::FloatFixed
             | Builtin::StrToFloat
+            | Builtin::StrIsFloat
             | Builtin::FloatToBits
             | Builtin::IntToChar
             | Builtin::StrAsBytes
@@ -8967,6 +8968,7 @@ pub(crate) fn builtin_signature(builtin: Builtin) -> (Vec<Ty>, Ty) {
         Builtin::StrToUpper | Builtin::StrToLower => (vec![Ty::String], Ty::String),
         Builtin::FloatFixed => (vec![Ty::Float, Ty::Int], Ty::String),
         Builtin::StrToFloat => (vec![Ty::String], Ty::Float),
+        Builtin::StrIsFloat => (vec![Ty::String], Ty::Bool),
         Builtin::FloatToBits => (vec![Ty::Float], Ty::Int),
         Builtin::IntToChar => (vec![Ty::Int], Ty::Char),
         Builtin::StrAsBytes => (vec![Ty::String], Ty::Bytes),
diff --git a/crates/nova-typeck/src/check/fixes.rs b/crates/nova-typeck/src/check/fixes.rs
index eb3e82f12a1bffee050b6ada9b4bb8c6279bff50..f82623932b26c8c18430a88f9b1be81fe920bd66 100644
--- a/crates/nova-typeck/src/check/fixes.rs
+++ b/crates/nova-typeck/src/check/fixes.rs
@@ -95,6 +95,14 @@ const STD_ONLY_PUBLIC_FORMS: &[(&str, &str)] = &[
         "a `String`'s `byte_len()`, `byte_at(i)` and `byte_slice(a, b)`, or \
          `bytes_from_string(s)` for a copy",
     ),
+    (
+        "str_is_float",
+        "the method `parse_float`, as in `s.parse_float()`",
+    ),
+    (
+        "str_to_float",
+        "the method `parse_float`, as in `s.parse_float()`",
+    ),
     ("float_to_bits", "the method `to_bits`, as in `x.to_bits()`"),
 ];
 
diff --git a/std/strings/lib.nova b/std/strings/lib.nova
index b91849f1fb12098c572d4aa131380462659f7042..3e322847f378c3db88fe5e0b60174465aa95904c 100644
--- a/std/strings/lib.nova
+++ b/std/strings/lib.nova
@@ -142,6 +142,54 @@ fn trim_end_index(cs: [Char], floor: Int) -> Int {
 // there: `0x80` to `0xBF`. `byte_slice`'s boundary check.
 fn continues_char(b: Int) -> Bool { b >= 128 && b <= 191 }
 
+// The value of the ASCII digit or letter `b` in a radix up to 36, `0` to `9`
+// then `a` (or `A`) to `z` as 10 to 35, or -1 for any other byte.
+fn digit_value(b: Int) -> Int {
+    if b >= 48 && b <= 57 { return b - 48 }
+    if b >= 97 && b <= 122 { return b - 87 }
+    if b >= 65 && b <= 90 { return b - 55 }
+    0 - 1
+}
+
+// `parse_int` and `parse_int_radix`: an optional sign, then one or more
+// digits of `radix`, and nothing else. The value is gathered NEGATIVELY, so
+// `Int`'s least value, whose magnitude no positive `Int` holds, parses too;
+// each step checks before it multiplies or subtracts, since `*` and `-` wrap
+// rather than trap. `least / radix` truncates toward zero, so it is the most
+// negative value that can still be multiplied by `radix` without leaving the
+// range.
+fn parse_digits(s: String, radix: Int) -> Option<Int> {
+    let b = str_as_bytes(s)
+    let n = bytes_len(b)
+    let mut i = 0
+    let mut negative = false
+    if n > 0 {
+        let first = bytes_at(b, 0)
+        if first == 45 {
+            negative = true
+            i = 1
+        } else if first == 43 {
+            i = 1
+        }
+    }
+    if i == n { return None }
+    let least = -9223372036854775807 - 1
+    let floor = least / radix
+    let mut acc = 0
+    while i < n {
+        let d = digit_value(bytes_at(b, i))
+        if d < 0 || d >= radix { return None }
+        if acc < floor { return None }
+        acc = acc * radix
+        if acc < least + d { return None }
+        acc = acc - d
+        i = i + 1
+    }
+    if negative { return Some(acc) }
+    if acc == least { return None }
+    Some(0 - acc)
+}
+
 impl String {
     // Not `self.len() == 0`: `len()` decodes the whole string, while `==` on
     // `String` compares byte lengths first in the runtime.
@@ -473,6 +521,75 @@ impl String {
         }
         bytes_to_string_unchecked(bytes_slice(b, start, end))
     }
+
+    // PARSING NUMBERS (5.0 spec §5.3). `parse_int` and `parse_int_radix` are
+    // Nova, over the bytes; `parse_float` is the runtime's, through Rust's
+    // own parser, so N reads a float literal as S0's lexer does.
+    /// Returns the decimal integer the string holds, or `None` if it holds
+    /// anything else or a value outside `Int`'s range.
+    ///
+    /// The text is an optional `+` or `-`, then one or more ASCII digits, and
+    /// nothing else: no space, no `_`, no prefix.
+    ///
+    /// ```
+    /// assert_eq("42".parse_int().unwrap(), 42)
+    /// assert_eq("-7".parse_int().unwrap(), -7)
+    /// assert_eq("+7".parse_int().unwrap(), 7)
+    /// assert_eq("-9223372036854775808".parse_int().unwrap(), -9223372036854775807 - 1)
+    /// assert("9223372036854775808".parse_int().is_none(), "past the range")
+    /// assert(" 1".parse_int().is_none(), "no space")
+    /// assert("1_000".parse_int().is_none(), "no `_`")
+    /// assert("".parse_int().is_none(), "no digits")
+    /// ```
+    pub fn parse_int(self) -> Option<Int> { parse_digits(self, 10) }
+
+    /// Returns the integer the string holds in `radix`, 2 to 36, or `None` if
+    /// it holds anything else or a value outside `Int`'s range.
+    ///
+    /// The text is an optional `+` or `-`, then one or more digits: `0` to
+    /// `9`, then the letters `a` to `z` in either case for 10 to 35, each less
+    /// than `radix`. No prefix is taken: `"0x1F"` is `None` in radix 16.
+    /// Panics if `radix` is not from 2 to 36.
+    ///
+    /// ```
+    /// assert_eq("ff".parse_int_radix(16).unwrap(), 255)
+    /// assert_eq("FF".parse_int_radix(16).unwrap(), 255)
+    /// assert_eq("-101".parse_int_radix(2).unwrap(), -5)
+    /// assert_eq("z".parse_int_radix(36).unwrap(), 35)
+    /// assert("2".parse_int_radix(2).is_none(), "2 is not a binary digit")
+    /// assert("0x1F".parse_int_radix(16).is_none(), "no prefix")
+    /// ```
+    pub fn parse_int_radix(self, radix: Int) -> Option<Int> {
+        if radix < 2 || radix > 36 { panic("String::parse_int_radix radix must be from 2 to 36") }
+        parse_digits(self, radix)
+    }
+
+    // On WASM both builtins are the shim's, by the same grammar, and its
+    // `str_to_float` keeps a `-nan`'s sign, as Rust's does: `to_bits` shows it.
+    /// Returns the number the string holds, or `None` if it is not text that
+    /// Rust's `f64::from_str` accepts.
+    ///
+    /// The text is an optional `+` or `-`, then digits with an optional `.`
+    /// and an optional exponent, `e` or `E` with an optional sign and
+    /// digits, or `inf`, `infinity` or `nan` in any case. There must be a
+    /// digit before or after the `.`; no space or `_` is taken. The value is
+    /// the nearest `Float`, as Rust rounds it, and one too large is infinite.
+    ///
+    /// ```
+    /// assert_eq("2.5".parse_float().unwrap(), 2.5)
+    /// assert_eq("-1e3".parse_float().unwrap(), -1000.0)
+    /// assert_eq(".5".parse_float().unwrap(), 0.5)
+    /// assert_eq("5.".parse_float().unwrap(), 5.0)
+    /// assert_eq("INF".parse_float().unwrap(), "inf".parse_float().unwrap())
+    /// let nan = "NaN".parse_float().unwrap()
+    /// assert(nan != nan, "NaN is not equal to itself")
+    /// assert("1e".parse_float().is_none(), "an exponent needs digits")
+    /// assert("".parse_float().is_none(), "no digits")
+    /// assert(" 1".parse_float().is_none(), "no space")
+    /// ```
+    pub fn parse_float(self) -> Option<Float> {
+        if str_is_float(self) { Some(str_to_float(self)) } else { None }
+    }
 }
 
 // `string_from_chars` is a free function, named after `bytes_from_string`
````

### Task 6: `OrderedMap`

Spec §5.6; the user's decision 8; decisions 3, 16, 29 and 30.

**Files:**
- Modify: `std/collections/lib.nova` (the module doc; `Map`'s docs point to
  `OrderedMap`; `OrderedMap`, `OrderedMapIter`)
- Create: `tests/runtime/ordered_map.nova`, `ordered_map.stdout`
- Test: `std_tests.rs` (two); `lower_tests.rs` (one); the language server's
  `completion.rs` (one, and a helper); `run_tests.rs` (`ordered_map_runs`,
  `ordered_map_order_is_the_same_each_run`), `wasm_tests.rs` (`gate!`),
  `index_complete.rs`

- [ ] **Step 1: The tests.** `patch_block t6-tests`. The patch holds:
  - `std_tests.rs`: `OrderedMap`'s types, `iter`'s item a `(K, V)` tuple;
    and `insert` through a binding that is not `mut`, E0060.
    `lower_tests.rs`: a key type without `Hash` and `Eq` is E0013 when a
    method is instantiated. `completion.rs`: `OrderedMap`, `OrderedMapIter`,
    `StringBuilder` and `ArrayIter` complete as names, and an `OrderedMap`'s
    methods after a dot.
  - **The program** `ordered_map`: an empty map; insertion order; an update
    in place, of the first key too; a removal and a reinsertion; `keys`,
    `values` and `entries` as copies; removal while iterating; the slots at
    the threshold, through the map's own `keys` field; growth with removals;
    every key removed and the map reused; a program's own key type whose
    hashes collide. Its `.stdout`, `run_tests.rs`'s test under `run`,
    `build` and GC stress and its run-twice test, its `gate!` entry, and the
    counts, 183 programs and 217 files.
- [ ] **Step 2: Run, and see the tests fail.** First the build, which ends
  with `Finished`.
  - `T check::std_tests completion::tests::std_records_of_5_0c_complete`.
    Expected: `nova-lsp`'s `0 passed; 1 failed`,
    `std_records_of_5_0c_complete`, "OrderedMap" missing from the names
    (`completion.rs:275:13`), and `nova-typeck`'s `24 passed; 2 failed`,
    `ordered_map_has_the_specs_types` and
    `an_ordered_map_mutates_only_through_a_mutable_binding`, each meeting
    E0001 "cannot find type `OrderedMap`" and E0900 for `OrderedMap::new`.
  - `L ordered_map`. Expected: `0 passed; 1 failed`,
    `ordered_map_key_without_hash_reports_e0013`, "codes: []": lowering is
    never reached.
  - `R ordered_map`. Expected: `0 passed; 2 failed`, `ordered_map_runs` and
    `ordered_map_order_is_the_same_each_run`, the program refused with E0001
    "cannot find type `OrderedMap`", E0900 "module-qualified paths are not
    supported yet" and E0013s.
  - `W gate::ordered_map every_runtime_program_is_in_the_gate_or_left_out`.
    Expected: `1 passed; 1 failed`, the same.
  - `X`. Expected: `0 passed; 1 failed`.
- [ ] **Step 3: The code.** `patch_block t6-code`: **`std/collections`:**
  its module doc names `OrderedMap`; `Map`'s docs and `keys`' point to it;
  `OrderedMap { len, keys, vals, live, index }` and its comment on
  compaction; `new`, `len`, `is_empty`, `insert`, which compacts by decision
  16 before it appends, `get`, `contains_key`, `remove`, which marks its
  slot dead, `keys`, `values`, `entries`, `iter` and the private `compact`;
  `OrderedMapIter`, skipping dead slots. Each with a doc test.
- [ ] **Step 4: Run.** First the build, which ends with `Finished`.
  - `T check::std_tests completion::tests::std_records_of_5_0c_complete`.
    Expected: every `test result` `ok`, 27 passing in all: `nova-typeck`'s
    26 and `nova-lsp`'s 1.
  - `L ordered_map`. Expected: `1 passed; 0 failed`.
  - `R ordered_map`. Expected: `2 passed; 0 failed`.
  - `W gate::ordered_map every_runtime_program_is_in_the_gate_or_left_out`.
    Expected: `2 passed; 0 failed`.
  - `X`. Expected: `1 passed; 0 failed`.
  - `S`, as in Task 1 (about 6 min). Expected: `2771 1 10`: Task 5's figures
    and Task 6's 7 tests, the one failure the IPv6 test. The first replay's
    run here gave `2759 2 10`:
    `http_server_example_bounds_the_drain_for_a_pipelining_client`, an HTTP
    example test on port 3000 that 5.0c does not touch, failed once with
    "connect to 127.0.0.1:3000: … Connection refused"; it passed in every
    other run of the suite, this replay's here included (decision 39). A
    failure of it alone is that flake: run
    `R http_server_example_bounds_the_drain_for_a_pipelining_client`, which
    passes, and record it in the ledger.
  - `G`. Expected:
    `release golden: 145 passed, 9 excluded, Ubuntu clang version 18.1.3 (1ubuntu1)`
    and `1 passed`.
- [ ] **Step 5: The checks,** as the conventions list them.
- [ ] **Step 6: Commit** `std: OrderedMap`.

**Patch `t6-tests`:**

<!-- patch t6-tests -->

````diff
diff --git a/crates/nova-cli/tests/run_tests.rs b/crates/nova-cli/tests/run_tests.rs
index c86b9e1771de89e804eaecc01eb22cdd4ed3068e..50c7788bc9d6e3b3aa2ae26ba97ef155068ca4b2 100644
--- a/crates/nova-cli/tests/run_tests.rs
+++ b/crates/nova-cli/tests/run_tests.rs
@@ -13180,6 +13180,34 @@ fn each_std_panic_names_its_method() {
     }
 }
 
+/// `OrderedMap` (5.0 spec §5.6): order after updates, removals,
+/// reinsertion and growth, `entries` and `iter`, also under GC stress, since
+/// its entries live in `Vec`s and a `Map` that grow while the collector
+/// runs.
+#[test]
+fn ordered_map_runs() {
+    golden_three_ways("ordered_map", true);
+}
+
+/// The program prints the same order on each run (5.0 spec §5.6): its keys
+/// are `String`s and `Int`s, whose hashes take a seed drawn once per
+/// process, so a `Map`'s order could differ between the two runs; an
+/// `OrderedMap`'s cannot.
+#[test]
+fn ordered_map_order_is_the_same_each_run() {
+    let source = repo_root().join("tests/runtime/ordered_map.nova");
+    let once = || {
+        let out = nova().arg("run").arg(&source).assert().success();
+        String::from_utf8(out.get_output().stdout.clone()).expect("UTF-8")
+    };
+    let (first, second) = (once(), once());
+    assert_eq!(first, second);
+    let expected = std::fs::read_to_string(source.with_extension("stdout"))
+        .expect("the .stdout file")
+        .replace("\r\n", "\n");
+    assert_eq!(first.replace("\r\n", "\n"), expected);
+}
+
 /// A guard on an or-pattern runs once, with the bindings of the first
 /// alternative that matches, and a false guard goes on to the next arm, not
 /// to a later alternative (5.0 spec §4.6; ADR 0049): `(1, 9)` binds `a` to
diff --git a/crates/nova-cli/tests/wasm_tests.rs b/crates/nova-cli/tests/wasm_tests.rs
index b0febe2a6d4e4e7ffa49e277f7d7e275c5b1cb88..e05949b36f3dbb8414f6bcfbc44b15be1ad0740b 100644
--- a/crates/nova-cli/tests/wasm_tests.rs
+++ b/crates/nova-cli/tests/wasm_tests.rs
@@ -2049,6 +2049,7 @@ mod gate {
         char_and_bytes,
         parse_numbers,
         float_bits,
+        ordered_map,
     );
 
     /// The programs a test of their own in this module checks, by its name.
diff --git a/crates/nova-driver/tests/index_complete.rs b/crates/nova-driver/tests/index_complete.rs
index 2a60c0eea14629cfb0a532f4f911367d4bb66d2b..2062ba0020bffbae82eef64704235703ddb8e31f 100644
--- a/crates/nova-driver/tests/index_complete.rs
+++ b/crates/nova-driver/tests/index_complete.rs
@@ -251,8 +251,9 @@ fn every_name_in_the_corpora_is_indexed() {
     // `patterns`, `patterns_async`, `match_effects` and `for_arrays`; 166
     // after 5.0c added `array_helpers`; 167 after `hidden_trait_methods`;
     // 168 after `char_and_bytes`; 170 after `string_builder` and
-    // `string_builder_million`; 172 after `parse_numbers` and `float_bits`.
-    assert_eq!(programs.len(), 182, "{programs:#?}");
+    // `string_builder_million`; 172 after `parse_numbers` and `float_bits`;
+    // 173 after `ordered_map`.
+    assert_eq!(programs.len(), 183, "{programs:#?}");
     let mut files = 0;
     let mut failures: Vec<String> = Vec::new();
     for (k, path) in programs.iter().enumerate() {
@@ -279,11 +280,11 @@ fn every_name_in_the_corpora_is_indexed() {
             every_name_is_covered(&a, file, &mut failures);
         }
     }
-    // 9 + 172 + 3 files of user code, 05-geometry's library's 2, the 8
+    // 9 + 173 + 3 files of user code, 05-geometry's library's 2, the 8
     // modules 07-fullstack-blog's client reaches from its `main.nova` (not
     // the server's 4, which only `server.nova` and `lib.nova` reach), and
     // native std's 22, `std/ui/ssr` among them.
-    assert_eq!(files, 216);
+    assert_eq!(files, 217);
     failures.sort();
     failures.dedup();
     assert!(
diff --git a/crates/nova-lsp/src/completion.rs b/crates/nova-lsp/src/completion.rs
index ddee7cbd0f661c1b1208db6124b15f68715b3dba..a53a97c300937c751ea51656cd53be7712717975 100644
--- a/crates/nova-lsp/src/completion.rs
+++ b/crates/nova-lsp/src/completion.rs
@@ -252,6 +252,40 @@ mod tests {
         }
     }
 
+    /// The completion labels at `offset` in `text`, a file of its own
+    /// under the temporary directory's `dir`.
+    fn labels_in(dir: &str, text: &str, offset: usize) -> Vec<String> {
+        let path = std::env::temp_dir().join(dir).join("main.nova");
+        let overlay = Overlay::default().with(&path, text.to_string());
+        complete(&path, text, offset as u32, &overlay)
+            .into_iter()
+            .map(|i| i.label)
+            .collect()
+    }
+
+    /// 5.0c's std records are offered as names, in every module, and an
+    /// `OrderedMap`'s methods after a dot (5.0 spec §5.1, §5.5, §5.6).
+    #[test]
+    fn std_records_of_5_0c_complete() {
+        let text = "fn main() {\n    let mut m: OrderedMap<String, Int> = OrderedMap::new()\n    \
+                    \n    m.\n}\n";
+        let blank = text.find("new()\n    ").unwrap() + "new()\n    ".len();
+        let names = labels_in("nova-lsp-complete-std-50c", text, blank);
+        for want in ["OrderedMap", "OrderedMapIter", "StringBuilder", "ArrayIter"] {
+            assert!(names.contains(&want.to_string()), "{want}: {names:?}");
+        }
+        let members = labels_in(
+            "nova-lsp-complete-std-50c",
+            text,
+            text.find("m.\n").unwrap() + 2,
+        );
+        for want in [
+            "insert", "get", "remove", "keys", "values", "entries", "iter",
+        ] {
+            assert!(members.contains(&want.to_string()), "{want}: {members:?}");
+        }
+    }
+
     #[test]
     fn a_signature_stops_at_its_body_outside_parentheses() {
         assert_eq!(
diff --git a/crates/nova-mir/tests/lower_tests.rs b/crates/nova-mir/tests/lower_tests.rs
index c1a25a031c12f48824fadfc8ffd3021fa6ca83aa..ed249a1b0e9813c1b5831af48aa5d4fdff793975 100644
--- a/crates/nova-mir/tests/lower_tests.rs
+++ b/crates/nova-mir/tests/lower_tests.rs
@@ -2474,3 +2474,19 @@ fn array_methods_monomorphise_per_element_type() {
     let slices: Vec<&&str> = names.iter().filter(|n| n.contains("slice")).collect();
     assert_eq!(slices.len(), 2, "one `slice` per element type: {names:?}");
 }
+
+/// `OrderedMap`'s methods need `K: Hash + Eq`, as `Map`'s do: a key type
+/// without them is E0013 when the method is instantiated (5.0 spec §5.6).
+#[test]
+fn ordered_map_key_without_hash_reports_e0013() {
+    let codes = diagnostics_for(
+        "record Unhashable { v: Int }\n\
+         fn main() {\n\
+             let mut m = OrderedMap::new()\n\
+             let k = Unhashable { v: 1 }\n\
+             let p = m.insert(k, 1)\n\
+             println(\"${m.len()}\")\n\
+         }",
+    );
+    assert!(codes.contains(&"E0013".to_string()), "codes: {codes:?}");
+}
diff --git a/crates/nova-typeck/src/check/std_tests.rs b/crates/nova-typeck/src/check/std_tests.rs
index 42bd48da5cd73b0f1356a7e410f7ffb06068f048..c6d9433c60e0ca5bc5fd95a815e58b5189bafcf9 100644
--- a/crates/nova-typeck/src/check/std_tests.rs
+++ b/crates/nova-typeck/src/check/std_tests.rs
@@ -7,6 +7,7 @@
 //! - the std-only builtins kept from user code, and the public form a
 //!   program that names one is pointed to;
 //! - the new items' types;
+//! - `OrderedMap`'s types;
 
 use super::*;
 use nova_diagnostics::{FileId, Severity};
@@ -250,6 +251,43 @@ fn a_builder_mutates_only_through_a_mutable_binding() {
     assert_eq!(e[0].code, "E0060");
 }
 
+/// `OrderedMap`'s types (spec §5.6).
+#[test]
+fn ordered_map_has_the_specs_types() {
+    let r = checked(
+        "fn main() {\n    let mut m: OrderedMap<String, Int> = OrderedMap::new()\n    \
+         let ins = m.insert(\"a\", 1)\n    let g = m.get(\"a\")\n    \
+         let rm = m.remove(\"a\")\n    let ck = m.contains_key(\"a\")\n    \
+         let n = m.len()\n    let ie = m.is_empty()\n    let ks = m.keys()\n    \
+         let vs = m.values()\n    let es = m.entries()\n    let mut it = m.iter()\n    \
+         let nx = it.next()\n}\n",
+    );
+    assert!(r.diagnostics.is_empty(), "{:#?}", r.diagnostics);
+    for name in ["ins", "g", "rm"] {
+        assert!(option(&local_ty(&r, name), Ty::Int), "{name}");
+    }
+    let pair = Ty::Tuple(vec![Ty::String, Ty::Int]);
+    assert!(option(&local_ty(&r, "nx"), pair.clone()));
+    assert_eq!(local_ty(&r, "n"), Ty::Int);
+    for name in ["ck", "ie"] {
+        assert_eq!(local_ty(&r, name), Ty::Bool, "{name}");
+    }
+    assert_eq!(local_ty(&r, "ks"), Ty::Array(Box::new(Ty::String)));
+    assert_eq!(local_ty(&r, "vs"), Ty::Array(Box::new(Ty::Int)));
+    assert_eq!(local_ty(&r, "es"), Ty::Array(Box::new(pair)));
+}
+
+/// `insert` and `remove` take `mut self`, as `Map`'s do.
+#[test]
+fn an_ordered_map_mutates_only_through_a_mutable_binding() {
+    let e = errors(
+        "fn main() {\n    let m: OrderedMap<Int, Int> = OrderedMap::new()\n    \
+         let _ = m.insert(1, 2)\n}\n",
+    );
+    assert_eq!(e.len(), 1, "{e:#?}");
+    assert_eq!(e[0].code, "E0060");
+}
+
 /// A program's impl that defines a method std's impl on the same type has,
 /// `iter` on arrays, is E0074 labelled at the program's impl, its primary
 /// label, and at std's, which the program cannot change (5.0 spec §5.5;
diff --git a/tests/runtime/ordered_map.nova b/tests/runtime/ordered_map.nova
new file mode 100644
index 0000000000000000000000000000000000000000..0131aa97f32a4cf4f98182769515a250b2e9f997
--- /dev/null
+++ b/tests/runtime/ordered_map.nova
@@ -0,0 +1,138 @@
+// `OrderedMap` (5.0 spec §5.6): the order after updates, removals,
+// reinsertion and growth, compaction, `keys`, `values`, `entries` and
+// `iter`, removal while iterating, keys of `String`, `Int` and a program's
+// own type, also under GC stress, since every entry lives in `Vec`s and a
+// `Map` that grow while the collector runs. `String` and `Int` keys hash
+// with a seed drawn once per process, so a `Map`'s order can differ between
+// runs; this program's output cannot, which its test checks by running it
+// twice.
+
+record Key { id: Int }
+
+impl Hash for Key {
+    fn hash(self) -> Int { self.id % 4 }
+}
+
+impl Eq for Key {
+    fn eq(self, other: Key) -> Bool { self.id == other.id }
+}
+
+fn show(m: OrderedMap<String, Int>) -> String {
+    let mut b = StringBuilder::new()
+    for (k, v) in m.iter() {
+        if b.byte_len() > 0 { b.push(" ") }
+        b.push("${k}=${v}")
+    }
+    "[${b}] len=${m.len()}"
+}
+
+fn main() {
+    let mut empty: OrderedMap<String, Int> = OrderedMap::new()
+    println(
+        "empty: ${show(empty)} ${empty.is_empty()} ${empty.keys().len()} ${empty.values().len()} ${empty.entries().len()}",
+    )
+    println(
+        "missing: ${empty.get("a").is_none()} ${empty.remove("a").is_none()} ${empty.contains_key("a")}",
+    )
+
+    let mut m: OrderedMap<String, Int> = OrderedMap::new()
+    for name in ["delta", "alpha", "charlie", "bravo", "echo"] { m.insert(name, name.len()) }
+    println("inserted: ${show(m)}")
+
+    // An update keeps its place, and returns the old value.
+    let old = m.insert("alpha", 100).unwrap()
+    println("updated alpha (was ${old}): ${show(m)}")
+    // So does an update of the first key, whose slot is 0.
+    let first = m.insert("delta", 50).unwrap()
+    println("updated delta (was ${first}): ${show(m)}")
+
+    // A removal leaves the others in order; a reinsertion goes last.
+    println("removed charlie: ${m.remove("charlie").unwrap()} ${show(m)}")
+    println("removed again: ${m.remove("charlie").is_none()}")
+    m.insert("charlie", 7)
+    println("reinserted charlie: ${show(m)}")
+    println(
+        "get: ${m.get("delta").unwrap()} ${m.get("charlie").unwrap()} ${m.contains_key("bravo")} ${m.contains_key("zulu")}",
+    )
+
+    // `keys`, `values` and `entries` are copies, in the same order.
+    let ks = m.keys()
+    let vs = m.values()
+    let es = m.entries()
+    let mut line = StringBuilder::new()
+    for i in 0..ks.len() {
+        if i > 0 { line.push(" ") }
+        line.push("${ks[i]}:${vs[i]}:${es[i].0}:${es[i].1}")
+    }
+    println("arrays: ${line}")
+    m.insert("foxtrot", 6)
+    println("copies keep their length: ${ks.len()} ${vs.len()} ${es.len()}, map ${m.len()}")
+
+    // Removing every other entry while iterating skips nothing else.
+    let mut seen = StringBuilder::new()
+    let mut n = 0
+    for (k, _) in m.iter() {
+        if n > 0 { seen.push(" ") }
+        seen.push("${k}")
+        if n % 2 == 0 { let _ = m.remove(k) }
+        n += 1
+    }
+    println("walked: ${seen}")
+    println("after removing while walking: ${show(m)}")
+
+    // When gaps are compacted: only as a new key is appended, and only when
+    // they are at least half the slots. `slots` reads the map's own `keys`
+    // field, which a program can reach, as `collections.nova` reads a
+    // `Map`'s `state`; it is how many slots the map holds, gaps included.
+    let mut small: OrderedMap<String, Int> = OrderedMap::new()
+    for name in ["a", "b", "c", "d"] { small.insert(name, 0) }
+    let mut slots = StringBuilder::new()
+    for name in ["a", "b", "c"] {
+        let _ = small.remove(name)
+        slots.push("${small.keys.len()}")
+        small.insert("${name}2", 1)
+        slots.push("->${small.keys.len()} ")
+    }
+    println("slots: ${slots}${show(small)}")
+
+    // Growth, with removals: the gaps are compacted as the map grows, and
+    // the order holds through each compaction.
+    let mut big: OrderedMap<Int, Int> = OrderedMap::new()
+    for i in 0..600 { big.insert(i, i * i) }
+    for i in 0..600 { if i % 3 != 0 { let _ = big.remove(i) } }
+    println(
+        "after removals: len=${big.len()} slots=${big.keys.len()} first=${big.keys()[0]} last=${big.keys()[big.len() - 1]}",
+    )
+    big.insert(600, 600)
+    println("one more: len=${big.len()} slots=${big.keys.len()}")
+    for i in 601..1000 { big.insert(i, i) }
+    let _ = big.insert(1, -1)
+    let _ = big.insert(3, -3)
+    let keys = big.keys()
+    let mut ordered = true
+    for i in 1..keys.len() { if keys[i] <= keys[i - 1] { if keys[i] != 1 { ordered = false } } }
+    let mut sum = 0
+    for (k, v) in big.iter() { sum += k + v }
+    println(
+        "grown: len=${big.len()} slots=${big.keys.len()} keys=${keys.len()} ordered=${ordered} last=${keys[keys.len() - 1]} 3=${big.get(3).unwrap()} 999=${big.get(999).unwrap()} 5=${big.get(5).is_none()} sum=${sum}",
+    )
+
+    // Every key removed, then the map used again.
+    for k in big.keys() { let _ = big.remove(k) }
+    println("emptied: ${big.len()} ${big.is_empty()} ${big.keys().len()}")
+    big.insert(42, 1)
+    big.insert(7, 2)
+    println("reused: ${big.keys()[0]} ${big.keys()[1]} ${big.len()}")
+
+    // A program's own key type, whose hashes collide.
+    let mut own: OrderedMap<Key, String> = OrderedMap::new()
+    for id in [9, 5, 1, 13, 2] { own.insert(Key { id: id }, "k${id}") }
+    let _ = own.remove(Key { id: 1 })
+    own.insert(Key { id: 1 }, "again")
+    let mut out = StringBuilder::new()
+    for (k, v) in own.iter() {
+        if !out.is_empty() { out.push(" ") }
+        out.push("${k.id}:${v}")
+    }
+    println("own keys: ${out}")
+}
diff --git a/tests/runtime/ordered_map.stdout b/tests/runtime/ordered_map.stdout
new file mode 100644
index 0000000000000000000000000000000000000000..5e420b487406fbf0d40dc2be2d71952f96d74b0d
--- /dev/null
+++ b/tests/runtime/ordered_map.stdout
@@ -0,0 +1,20 @@
+empty: [] len=0 true 0 0 0
+missing: true true false
+inserted: [delta=5 alpha=5 charlie=7 bravo=5 echo=4] len=5
+updated alpha (was 5): [delta=5 alpha=100 charlie=7 bravo=5 echo=4] len=5
+updated delta (was 5): [delta=50 alpha=100 charlie=7 bravo=5 echo=4] len=5
+removed charlie: 7 [delta=50 alpha=100 bravo=5 echo=4] len=4
+removed again: true
+reinserted charlie: [delta=50 alpha=100 bravo=5 echo=4 charlie=7] len=5
+get: 50 7 true false
+arrays: delta:50:delta:50 alpha:100:alpha:100 bravo:5:bravo:5 echo:4:echo:4 charlie:7:charlie:7
+copies keep their length: 5 5 5, map 6
+walked: delta alpha bravo echo charlie foxtrot
+after removing while walking: [alpha=100 echo=4 foxtrot=6] len=3
+slots: 4->5 5->6 6->4 [d=0 a2=1 b2=1 c2=1] len=4
+after removals: len=200 slots=600 first=0 last=597
+one more: len=201 slots=201
+grown: len=601 slots=601 keys=601 ordered=true last=1 3=-3 999=999 5=true sum=24519588
+emptied: 0 true 0
+reused: 42 7 2
+own keys: 9:k9 5:k5 13:k13 2:k2 1:again
````

**Patch `t6-code`:**

<!-- patch t6-code -->

````diff
diff --git a/std/collections/lib.nova b/std/collections/lib.nova
index c22ea0b2a228680e43d2c4205e6972b225b3bf8f..83fb14b5c3570147269649531869774d471cfa48 100644
--- a/std/collections/lib.nova
+++ b/std/collections/lib.nova
@@ -1,12 +1,14 @@
-//! Growable vectors, hash maps and hash sets.
+//! Growable vectors, hash maps, insertion-ordered maps and hash sets.
 //!
 //! `Vec` is a list of values that grows as they are pushed, and `VecIter`
-//! walks one in order. `Map` maps keys to values, and `Set` holds values
-//! without repeats; both need a key or value type that implements `Hash` and
-//! `Eq`. Every module sees these types without an import, and a module's own
-//! definition of one of these names shadows the one here.
+//! walks one in order. `Map` maps keys to values, `OrderedMap` does too and
+//! keeps its keys in the order they were first inserted, and `Set` holds
+//! values without repeats; all three need a key or value type that
+//! implements `Hash` and `Eq`. Every module sees these types without an
+//! import, and a module's own definition of one of these names shadows the
+//! one here.
 //!
-//! The three collections are shared, not copied: assigning one to another
+//! The collections are shared, not copied: assigning one to another
 //! binding, or passing it to a function, gives a second name for the same
 //! collection, and a change made through either is seen through both. A
 //! method that changes a collection takes `mut self`, and needs a mutable
@@ -423,7 +425,8 @@ impl<T> Iterator for ArrayIter<T> {
 ///
 /// Its methods need a key type that implements `Hash` and `Eq`, and keys
 /// equal by `eq` must have the same hash. Assigning a `Map`, or passing it to
-/// a function, shares it rather than copying it.
+/// a function, shares it rather than copying it. Its keys come out in no set
+/// order; `OrderedMap` keeps them in the order they were inserted.
 pub record Map<K, V> { len: Int, used: Int, keys: [K], vals: [V], state: [Int] }
 
 impl<K: Hash + Eq, V> Map<K, V> {
@@ -648,6 +651,7 @@ impl<K: Hash + Eq, V> Map<K, V> {
     ///
     /// Their order is unspecified: it need not be the order the keys were
     /// inserted in, and it can change from one run of a program to the next.
+    /// An `OrderedMap` gives its keys in the order they were inserted.
     pub fn keys(self) -> [K] {
         if self.len == 0 { return [] }
         let cap = self.state.len()
@@ -774,3 +778,336 @@ impl<T: Hash + Eq> Set<T> {
         }
     }
 }
+
+// An insertion-ordered map (5.0 spec §5.6; docs/adr/0050): `Map` stays as it
+// is, with its unspecified order, and this is a new type beside it.
+//
+// Inside, `keys`, `vals` and `live` are three parallel `Vec`s, one slot per
+// key in the order keys were first inserted, and `index` maps each live key
+// to its slot. The `Map` is read for lookups only, never walked, so the order
+// never depends on its seed or its layout: `keys`, `values`, `entries` and
+// `iter` walk the slots. Updating a key writes its slot's value and keeps its
+// place. Removing a key takes it out of `index` and marks its slot dead,
+// leaving a gap; the dead key and value stay in their slots until the next
+// compaction.
+//
+// COMPACTION. Only `insert` compacts, when it is about to append a slot for a
+// new key and the gaps are at least half the slots: it builds the three
+// `Vec`s afresh from the live slots, in order, and moves each key's position
+// in `index`. Half, because then the slots are never more than twice the
+// entries, plus the one being added: the same slack `Vec`'s doubling allows
+// its storage, so a walk is linear in `len()` after any insertion. Each
+// compaction walks the slots, at most twice the gaps it removes, and each gap
+// came from one `remove`, so `insert` and `remove` stay amortized constant
+// time, besides the `Map`'s own. `remove` never compacts, so it never moves
+// an entry: removing entries while an iterator walks the map skips nothing.
+// A map that only shrinks keeps its slots until it next grows.
+//
+// As with `Map`, the `K: Hash + Eq` contract lives on the `impl`, not on the
+// record's parameters, where a bound would constrain nothing (ADR 0007 §1).
+/// A map from keys of type `K` to values of type `V` that keeps its keys in
+/// the order they were first inserted.
+///
+/// Updating a key's value keeps its place; removing a key, then inserting it
+/// again, puts it last. `keys`, `values`, `entries` and `iter` all give that
+/// order, the same on every run of a program, where a `Map`'s order is
+/// unspecified. Its methods need a key type that implements `Hash` and
+/// `Eq`, and keys equal by `eq` must have the same hash. Lookups take
+/// constant time on average, as a `Map`'s do. Assigning an `OrderedMap`, or
+/// passing it to a function, shares it rather than copying it.
+///
+/// ```
+/// let mut deps: OrderedMap<String, Int> = OrderedMap::new()
+/// deps.insert("lexer", 1)
+/// deps.insert("parser", 2)
+/// deps.insert("checker", 3)
+/// deps.insert("lexer", 4) // an update keeps its place
+/// let _ = deps.remove("parser")
+/// deps.insert("parser", 5) // reinserted, so it is last
+/// let order = deps.keys()
+/// assert_eq(order.len(), 3)
+/// assert_eq(order[0], "lexer")
+/// assert_eq(order[1], "checker")
+/// assert_eq(order[2], "parser")
+/// assert_eq(deps.get("lexer").unwrap(), 4)
+/// ```
+pub record OrderedMap<K, V> {
+    len: Int,
+    keys: Vec<K>,
+    vals: Vec<V>,
+    live: Vec<Bool>,
+    index: Map<K, Int>,
+}
+
+impl<K: Hash + Eq, V> OrderedMap<K, V> {
+    /// Returns an empty map.
+    ///
+    /// ```
+    /// let m: OrderedMap<String, Int> = OrderedMap::new()
+    /// assert(m.is_empty(), "a new map is empty")
+    /// assert_eq(m.keys().len(), 0)
+    /// ```
+    pub fn new() -> OrderedMap<K, V> {
+        OrderedMap {
+            len: 0,
+            keys: Vec::new(),
+            vals: Vec::new(),
+            live: Vec::new(),
+            index: Map::new(),
+        }
+    }
+
+    /// Returns the number of entries in the map.
+    ///
+    /// ```
+    /// let mut m: OrderedMap<Int, Int> = OrderedMap::new()
+    /// m.insert(1, 10)
+    /// m.insert(1, 11)
+    /// m.insert(2, 20)
+    /// assert_eq(m.len(), 2)
+    /// ```
+    pub fn len(self) -> Int { self.len }
+
+    /// Returns `true` if the map holds no entries.
+    ///
+    /// ```
+    /// let mut m: OrderedMap<Int, Int> = OrderedMap::new()
+    /// m.insert(1, 10)
+    /// let _ = m.remove(1)
+    /// assert(m.is_empty(), "its one entry was removed")
+    /// ```
+    pub fn is_empty(self) -> Bool { self.len == 0 }
+
+    /// Maps `k` to `v`, and returns the value `k` mapped to before, or `None`
+    /// if `k` was not in the map.
+    ///
+    /// A key that was in the map keeps its place and the key it had, and
+    /// takes `v`; a new key goes last.
+    ///
+    /// ```
+    /// let mut m: OrderedMap<String, Int> = OrderedMap::new()
+    /// assert(m.insert("a", 1).is_none(), "a is new")
+    /// assert_eq(m.insert("a", 2).unwrap(), 1)
+    /// assert_eq(m.len(), 1)
+    /// ```
+    pub fn insert(mut self, k: K, v: V) -> Option<V> {
+        let at = match self.index.get(k) {
+            Some(i) => i
+            None => -1
+        }
+        if at >= 0 {
+            let old = self.vals.at(at)
+            self.vals.set(at, v)
+            return Some(old)
+        }
+        let slots = self.keys.len()
+        let gaps = slots - self.len
+        if gaps > 0 && gaps * 2 >= slots { self.compact() }
+        let _ = self.index.insert(k, self.keys.len())
+        self.keys.push(k)
+        self.vals.push(v)
+        self.live.push(true)
+        self.len = self.len + 1
+        None
+    }
+
+    /// Returns the value `k` maps to, or `None` if `k` is not in the map.
+    ///
+    /// ```
+    /// let mut m: OrderedMap<Int, String> = OrderedMap::new()
+    /// m.insert(7, "seven")
+    /// assert_eq(m.get(7).unwrap(), "seven")
+    /// assert(m.get(8).is_none(), "8 was never inserted")
+    /// ```
+    pub fn get(self, k: K) -> Option<V> {
+        match self.index.get(k) {
+            Some(at) => Some(self.vals.at(at))
+            None => None
+        }
+    }
+
+    /// Returns `true` if `k` is in the map.
+    ///
+    /// ```
+    /// let mut m: OrderedMap<Char, Int> = OrderedMap::new()
+    /// m.insert('a', 1)
+    /// assert(m.contains_key('a'), "a was inserted")
+    /// assert(!m.contains_key('b'), "b was not")
+    /// ```
+    pub fn contains_key(self, k: K) -> Bool { self.index.contains_key(k) }
+
+    /// Removes the entry for `k` and returns its value, or returns `None` if
+    /// `k` was not in the map.
+    ///
+    /// The other entries keep their order. Inserting `k` again puts it last.
+    ///
+    /// ```
+    /// let mut m: OrderedMap<String, Int> = OrderedMap::new()
+    /// m.insert("a", 1)
+    /// m.insert("b", 2)
+    /// assert_eq(m.remove("a").unwrap(), 1)
+    /// assert(m.remove("a").is_none(), "already removed")
+    /// m.insert("a", 3)
+    /// assert_eq(m.keys()[1], "a")
+    /// ```
+    pub fn remove(mut self, k: K) -> Option<V> {
+        match self.index.remove(k) {
+            Some(at) => {
+                self.live.set(at, false)
+                self.len = self.len - 1
+                Some(self.vals.at(at))
+            }
+            None => None
+        }
+    }
+
+    // The three walks below fill a new array with slot 0's key or value, which
+    // is there, live or dead, whenever `len > 0`, then overwrite every
+    // element: the `[fill; n]` form needs a value in hand, as in `Map::keys`.
+    /// Returns the map's keys, in the order they were first inserted, as a
+    /// new array.
+    ///
+    /// ```
+    /// let mut m: OrderedMap<String, Bool> = OrderedMap::new()
+    /// m.insert("z", true)
+    /// m.insert("a", false)
+    /// let ks = m.keys()
+    /// assert_eq(ks[0], "z")
+    /// assert_eq(ks[1], "a")
+    /// ```
+    pub fn keys(self) -> [K] {
+        if self.len == 0 { return [] }
+        let mut out = [self.keys.at(0); self.len]
+        let mut w = 0
+        for i in 0..self.keys.len() {
+            if self.live.at(i) {
+                out[w] = self.keys.at(i)
+                w = w + 1
+            }
+        }
+        out
+    }
+
+    /// Returns the map's values, in the order of their keys, as a new
+    /// array.
+    ///
+    /// ```
+    /// let mut m: OrderedMap<String, Int> = OrderedMap::new()
+    /// m.insert("b", 2)
+    /// m.insert("a", 1)
+    /// let vs = m.values()
+    /// assert_eq(vs[0], 2)
+    /// assert_eq(vs[1], 1)
+    /// ```
+    pub fn values(self) -> [V] {
+        if self.len == 0 { return [] }
+        let mut out = [self.vals.at(0); self.len]
+        let mut w = 0
+        for i in 0..self.keys.len() {
+            if self.live.at(i) {
+                out[w] = self.vals.at(i)
+                w = w + 1
+            }
+        }
+        out
+    }
+
+    /// Returns the map's entries, each a `(key, value)` tuple, in the order
+    /// of their keys, as a new array.
+    ///
+    /// ```
+    /// let mut m: OrderedMap<String, Int> = OrderedMap::new()
+    /// m.insert("x", 1)
+    /// m.insert("y", 2)
+    /// let es = m.entries()
+    /// assert_eq(es[1].0, "y")
+    /// assert_eq(es[1].1, 2)
+    /// ```
+    pub fn entries(self) -> [(K, V)] {
+        if self.len == 0 { return [] }
+        let mut out = [(self.keys.at(0), self.vals.at(0)); self.len]
+        let mut w = 0
+        for i in 0..self.keys.len() {
+            if self.live.at(i) {
+                out[w] = (self.keys.at(i), self.vals.at(i))
+                w = w + 1
+            }
+        }
+        out
+    }
+
+    /// Returns an iterator over the map's entries, each a `(key, value)`
+    /// tuple, in the order of their keys.
+    ///
+    /// The iterator shares the map rather than copying it. Removing entries
+    /// while it runs is safe: it skips the ones removed and nothing else. A
+    /// key inserted while it runs can move the entries, and the iterator then
+    /// skips or repeats some; `entries()` takes a copy to walk instead.
+    ///
+    /// ```
+    /// let mut m: OrderedMap<String, Int> = OrderedMap::new()
+    /// m.insert("one", 1)
+    /// m.insert("two", 2)
+    /// let mut total = 0
+    /// for (name, n) in m.iter() {
+    ///     total = total + n * name.len()
+    /// }
+    /// assert_eq(total, 9)
+    /// ```
+    pub fn iter(self) -> OrderedMapIter<K, V> { OrderedMapIter { m: self, i: 0 } }
+
+    // Squeeze the gaps out: the live slots, in order, into three new `Vec`s,
+    // each key's position in `index` moved to its new slot. `insert` alone
+    // calls it, before it appends; see the record's note for when. Each
+    // `index.insert` replaces a key's value, so it never grows the `Map`.
+    fn compact(mut self) {
+        let mut keys: Vec<K> = Vec::new()
+        let mut vals: Vec<V> = Vec::new()
+        let mut live: Vec<Bool> = Vec::new()
+        for i in 0..self.keys.len() {
+            if self.live.at(i) {
+                let k = self.keys.at(i)
+                let _ = self.index.insert(k, keys.len())
+                keys.push(k)
+                vals.push(self.vals.at(i))
+                live.push(true)
+            }
+        }
+        self.keys = keys
+        self.vals = vals
+        self.live = live
+    }
+}
+
+// A cursor over an `OrderedMap`'s slots: the map, shared, and the next slot
+// to look at. Dead slots are skipped; `remove` never moves a slot, so
+// removing while it runs skips nothing else. No `Hash + Eq` bound: it reads
+// the slots, never the `Map`.
+/// An iterator over an `OrderedMap`'s entries, each a `(key, value)` tuple,
+/// in the order of their keys.
+///
+/// `OrderedMap::iter` returns one.
+///
+/// ```
+/// let mut m: OrderedMap<Int, String> = OrderedMap::new()
+/// m.insert(2, "two")
+/// m.insert(1, "one")
+/// let mut it = m.iter()
+/// assert_eq(it.next().unwrap().1, "two")
+/// assert_eq(it.next().unwrap().0, 1)
+/// assert(it.next().is_none(), "two entries")
+/// ```
+pub record OrderedMapIter<K, V> { m: OrderedMap<K, V>, i: Int }
+
+impl<K, V> Iterator for OrderedMapIter<K, V> {
+    type Item = (K, V)
+
+    fn next(mut self) -> Option<(K, V)> {
+        while self.i < self.m.keys.len() {
+            let at = self.i
+            self.i = self.i + 1
+            if self.m.live.at(at) { return Some((self.m.keys.at(at), self.m.vals.at(at))) }
+        }
+        None
+    }
+}
````

### Task 7: `std/process`: `run`, `status`, `env` and `current_exe`

Spec §5.7; the user's decision 6; decisions 2, 8, 14, 21, 22, 25 to 28, 30
and 34. **Every run of this task's tests is under the process limit**
(Conventions): Step 2's runs too, though the program fails before it starts
a child.

**Files:**
- Modify:
  - `crates/nova-resolver/src/lib.rs` (the six builtins; `STD_ONLY` 100)
  - `crates/nova-typeck/src/check.rs` (their signatures)
  - `crates/nova-mir/src/lib.rs` (six `RtFunc`s, unavailable on WASM),
    `crates/nova-mir/src/lower.rs`
  - `crates/nova-runtime/src/process.rs` (`run_command`, `exit_code`,
    `env_value`, the six `nova_rt_process_*` functions),
    `crates/nova-runtime/src/lib.rs` (their symbols)
  - `std/process/lib.nova` (`ProcessOutput`, `run`, `status`, `env`,
    `current_exe`, the module doc)
- Create: `tests/runtime/process_run.nova`, `process_run.stdout`,
  `tests/runtime/process_env.nova`, `process_env.stdout`
- Test: `std_tests.rs` (four); the checker's
  `the_process_builtins_are_std_only_with_their_signatures` (six rows);
  `lower_tests.rs` (one); `completion.rs` (one); `process.rs`' six;
  `wasm_tests.rs` (`std_process_is_e0092_on_wasm`, the left-out list);
  `run_tests.rs` (`RELEASE_GOLDEN_ENV` and the walk's use of it, four tests,
  two constants); `index_complete.rs`

- [ ] **Step 1: The tests.** `patch_block t7-tests`. The patch holds:
  - `std_tests.rs`: the six builtins kept from user code; `std/process`'s
    types; a program's own `run`, `status` and `env` shadowing std's; and
    `run`'s arguments an array, E0010 for a string. The signature test's six
    rows. `lower_tests.rs`: `run` calls `process_run` and each take once,
    `status` the run and the status alone, `env` and `current_exe` their
    builtin and `fs_take_string`. `completion.rs`: the five names complete.
  - `process.rs`' tests: a program on `PATH` runs and both outputs are
    captured (on Windows, `cmd /C exit 7`'s status); 220 KB on each pipe is
    read whole; `kill -9` is 137; a missing program is `NotFound`, with and
    without capture; a file that cannot be run is `PermissionDenied`; `env`
    refuses names the system cannot hold (decision 43: no test of Rust's own
    `current_exe`; the run test covers 5.0c's).
  - `wasm_tests.rs`: each of the five names is E0092 on WASM and checks
    natively; `process_run` and `process_env` on the left-out list.
  - **The programs:** `process_run`, which runs itself as its child, with
    the guard of decision 22, and `process_env`. Their `.stdout`s, and the
    counts, 185 programs and 219 files.
  - `run_tests.rs`: `RELEASE_GOLDEN_ENV`, and the walk setting its variables
    and clearing each program's temporary directory; `process_run_runs`,
    built, with a fresh temporary directory, then with the marker in place;
    a child that panics is 134; `process_env_runs` under `nova run` and
    built, with and without the variable; `current_exe` under `nova run` and
    built.
- [ ] **Step 2: Run, and see the tests fail.** First the build, which ends
  with `Finished`.
  - `T check::std_tests check::tests::the_process_builtins_are_std_only_with_their_signatures completion::tests::std_process_names_complete process::tests::`.
    Expected: it does not compile:
    ``error: could not compile `nova-runtime` (lib test) due to 9 previous errors``,
    ``error[E0425]: cannot find function `run_command` in this scope`` and
    `env_value`'s, and
    ``error: could not compile `nova-typeck` (lib test) due to 6 previous errors``,
    ``error[E0599]: no variant, associated function, or constant named `ProcessRun` found for enum `nova_resolver::Builtin` in the current scope``
    and the other five (decision 32).
  - `L std_process`. Expected: it does not compile,
    ``error: could not compile `nova-mir` (test "lower_tests") due to 10 previous errors``,
    no `RtFunc::ProcessRun` and the rest.
  - `R process_run_runs process_env_runs current_exe_is_nova a_child_killed_by_a_signal`.
    Expected: `0 passed; 4 failed`, each program refused before it runs,
    E0001 "cannot find function `run` in this scope", or `status`, `env`,
    `current_exe`: no child is started.
  - `W std_process_is_e0092_on_wasm every_runtime_program_is_in_the_gate_or_left_out`.
    Expected: `1 passed; 1 failed`, `std_process_is_e0092_on_wasm`, E0001
    for `run` where E0092 is asked (`wasm_tests.rs:127:5`).
  - `X`. Expected: `0 passed; 1 failed`.
- [ ] **Step 3: The code.** `patch_block t7-code`:
  - **The resolver, the checker, MIR:** `ProcessRun`, `ProcessTakeStatus`,
    `ProcessTakeStdout`, `ProcessTakeStderr`, `ProcessEnv` and
    `ProcessCurrentExe`, each documented, after `ProcessExit` in every list;
    `STD_ONLY` 100; their signatures and `RtFunc`s, each
    `WasmRuntime::Unavailable`.
  - **`process.rs`:** the module doc's section on other programs; the
    thread's `STATUS`, `STDOUT` and `STDERR` cells; `exit_code`,
    `128 + signal` on Unix; `run_command`, which flushes standard output,
    then captures both pipes with an empty standard input, or inherits all
    three; `env_value`; `nova_rt_process_run`, reading the `[String]`'s
    words, `fail` on an error; the three takes; `nova_rt_process_env`;
    `nova_rt_process_current_exe`, `InvalidData` for a path that is not
    Unicode. **`lib.rs`:** the six symbols.
  - **`std/process`:** the module doc and the comment on the wire contract;
    `ProcessOutput`; `process_error`; `run`, `status`, `env` and
    `current_exe`, each with a doc test that starts no program.
- [ ] **Step 4: Run.** First the build, which ends with `Finished`.
  - `T check::std_tests check::tests::the_process_builtins_are_std_only_with_their_signatures completion::tests::std_process_names_complete process::tests::`.
    Expected: every `test result` `ok`, 40 passing in all: `nova-typeck`'s
    31 (`std_tests.rs`' 30 and the signature test), the runtime's 8
    (`process.rs`' six new and two older) and `nova-lsp`'s 1.
  - `L std_process`. Expected: `1 passed; 0 failed`.
  - `R process_run_runs process_env_runs current_exe_is_nova a_child_killed_by_a_signal`.
    Expected: `4 passed; 0 failed`, in about 10 s.
  - `W std_process_is_e0092_on_wasm every_runtime_program_is_in_the_gate_or_left_out`.
    Expected: `2 passed; 0 failed`.
  - `X`. Expected: `1 passed; 0 failed`.
  - `S`, as in Task 1 (about 6 min 15 s). Expected: `2788 1 10`: Task 6's
    figures and Task 7's 17 tests, the one failure the IPv6 test.
  - `G`. Expected:
    `release golden: 147 passed, 9 excluded, Ubuntu clang version 18.1.3 (1ubuntu1)`
    and `1 passed`: `process_run` and `process_env` join the walk, the
    second with its two variables.
- [ ] **Step 5: The checks,** as the conventions list them.
- [ ] **Step 6: Commit** `std, runtime: run, status, env and current_exe`.

**Patch `t7-tests`:**

<!-- patch t7-tests -->

````diff
diff --git a/crates/nova-cli/tests/run_tests.rs b/crates/nova-cli/tests/run_tests.rs
index 50c7788bc9d6e3b3aa2ae26ba97ef155068ca4b2..a3cba1dfcaa5d61c158027ba83169a62bce485ad 100644
--- a/crates/nova-cli/tests/run_tests.rs
+++ b/crates/nova-cli/tests/run_tests.rs
@@ -1804,6 +1804,14 @@ const RELEASE_GOLDEN_EXCLUDED: &[(&str, &str)] = &[
 /// program's is closed.
 const RELEASE_GOLDEN_STDIN: &[(&str, &str)] = &[("io_streams", "hello-stdin")];
 
+/// The environment variables `release_golden_suite` sets on a program,
+/// beside those it inherits: `process_env` reads the two its tests set (5.0
+/// spec §5.7).
+const RELEASE_GOLDEN_ENV: &[(&str, &str, &str)] = &[
+    ("process_env", "NOVA_PROCESS_TEST_VALUE", PROCESS_TEST_VALUE),
+    ("process_env", "NOVA_PROCESS_TEST_EQ", PROCESS_TEST_EQ),
+];
+
 /// The exit status `release_golden_suite` expects of a program, where it is
 /// not 0: a program that panics after printing its `.stdout` aborts.
 const RELEASE_GOLDEN_STATUS: &[(&str, i32)] = &[];
@@ -1892,6 +1900,7 @@ fn run_release_binary(
     exe: &std::path::Path,
     tmp: &std::path::Path,
     stdin: Option<&str>,
+    vars: &[(&str, &str)],
 ) -> Result<(String, String, Option<i32>), String> {
     let mut command = std::process::Command::new(exe);
     // Each program's temporary files go in a directory of its own, so none
@@ -1900,7 +1909,8 @@ fn run_release_binary(
         .env("TMPDIR", tmp)
         .env("TMP", tmp)
         .env("TEMP", tmp)
-        .env_remove("NOVA_GC_STRESS");
+        .env_remove("NOVA_GC_STRESS")
+        .envs(vars.iter().copied());
     run_within_limit(command, stdin)
 }
 
@@ -1960,6 +1970,12 @@ fn release_golden_suite() {
             "the exclusion table names `{name}`, which has no golden program"
         );
     }
+    for (name, _, _) in RELEASE_GOLDEN_ENV {
+        assert!(
+            programs.iter().any(|p| p == name),
+            "the environment table names `{name}`, which has no golden program"
+        );
+    }
     let walk: Vec<String> = programs
         .iter()
         .filter(|p| !RELEASE_GOLDEN_EXCLUDED.iter().any(|(n, _)| n == p))
@@ -1990,6 +2006,9 @@ fn release_golden_suite() {
                 let source = runtime.join(format!("{name}.nova"));
                 let exe = dir.join(format!("{name}{}", std::env::consts::EXE_SUFFIX));
                 let tmp = dir.join(format!("{name}.tmp"));
+                // Fresh each run: `process_run` leaves a marker there if a
+                // run stops part way.
+                let _ = std::fs::remove_dir_all(&tmp);
                 std::fs::create_dir_all(&tmp).expect("a temporary directory");
                 let mut build = std::process::Command::new(&nova_exe);
                 build
@@ -2028,7 +2047,12 @@ fn release_golden_suite() {
                 let expected = std::fs::read_to_string(source.with_extension("stdout"))
                     .expect("the .stdout file")
                     .replace('\r', "");
-                let ran = run_release_binary(&exe, &tmp, stdin);
+                let vars: Vec<(&str, &str)> = RELEASE_GOLDEN_ENV
+                    .iter()
+                    .filter(|(n, _, _)| n == name)
+                    .map(|(_, k, v)| (*k, *v))
+                    .collect();
+                let ran = run_release_binary(&exe, &tmp, stdin, &vars);
                 let _ = std::fs::remove_file(&exe);
                 match ran {
                     Err(why) => failures.lock().unwrap().push(format!("{name}: {why}")),
@@ -13208,6 +13232,190 @@ fn ordered_map_order_is_the_same_each_run() {
     assert_eq!(first.replace("\r\n", "\n"), expected);
 }
 
+/// `tests/runtime/process_run.nova` built with `nova build` into `dir`, as
+/// `name`: under `nova run`, `current_exe()` is `nova`, so a program that
+/// runs itself is built first (5.0 spec §10 risk 9).
+fn build_process_program(dir: &std::path::Path, name: &str, source: &str) -> PathBuf {
+    let _ = std::fs::remove_dir_all(dir);
+    std::fs::create_dir_all(dir).expect("a directory for the build");
+    let exe = dir.join(format!("{name}{}", std::env::consts::EXE_SUFFIX));
+    nova()
+        .arg("build")
+        .arg(source)
+        .arg("-o")
+        .arg(&exe)
+        .assert()
+        .success();
+    exe
+}
+
+/// `run` and `status` (5.0 spec §5.7): the program, built, runs itself
+/// through `current_exe()` as a child that prints its arguments to standard
+/// output, a line to standard error, and exits with a chosen status. The
+/// captured output, the status, arguments with spaces, quotes, a trailing
+/// backslash and non-ASCII, more output than a pipe holds, `status`'s
+/// inherited streams in order, and a missing program as `NotFound`, on all
+/// three systems. The child's standard error under `status` is this
+/// program's own. A child that lost its arguments would start the parent
+/// again, without end; the program's marker in its temporary directory
+/// stops that, so this test gives it a fresh one, and then checks that a
+/// run that finds the marker starts nothing. The build is removed only
+/// after every child has ended, since Windows locks a running executable.
+#[test]
+fn process_run_runs() {
+    let dir = std::env::temp_dir().join("nova-process-run");
+    let exe = build_process_program(
+        &dir,
+        "process_run",
+        &repo_root()
+            .join("tests/runtime/process_run.nova")
+            .to_string_lossy(),
+    );
+    // A temporary directory of its own, fresh, for the program's marker
+    // against a child that lost its arguments.
+    let tmp = dir.join("tmp");
+    std::fs::create_dir_all(&tmp).expect("a temporary directory");
+    let out = Command::new(&exe)
+        .env("TMPDIR", &tmp)
+        .env("TMP", &tmp)
+        .env("TEMP", &tmp)
+        .assert()
+        .success();
+    let out = out.get_output();
+    let expected = std::fs::read_to_string(repo_root().join("tests/runtime/process_run.stdout"))
+        .expect("the .stdout file")
+        .replace("\r\n", "\n");
+    let stdout = String::from_utf8_lossy(&out.stdout).replace("\r\n", "\n");
+    assert_eq!(stdout, expected);
+    let stderr = String::from_utf8_lossy(&out.stderr).replace("\r\n", "\n");
+    assert_eq!(stderr, "child stderr, status 5\n");
+    // The guard: with its marker already there, as a child that lost its
+    // arguments would find it, the program starts no child and stops.
+    std::fs::write(tmp.join("nova-process-run-parent"), "").expect("the marker");
+    let out = Command::new(&exe)
+        .env("TMPDIR", &tmp)
+        .env("TMP", &tmp)
+        .env("TEMP", &tmp)
+        .assert()
+        .code(1);
+    let out = out.get_output();
+    assert_eq!(String::from_utf8_lossy(&out.stdout), "");
+    let stderr = String::from_utf8_lossy(&out.stderr);
+    assert!(
+        stderr.contains("exists: a child lost its arguments, or an earlier run stopped"),
+        "{stderr}"
+    );
+    let _ = std::fs::remove_dir_all(&dir);
+}
+
+/// A child that panics aborts, and a death by signal is `128 + signal`, as
+/// shells report it (5.0 spec §5.7, §12 item 22): `SIGABRT`, 6, gives 134.
+/// Unix only: Windows has no signals, and its abort's status is its own.
+#[cfg(unix)]
+#[test]
+fn a_child_killed_by_a_signal_is_128_plus_the_signal() {
+    let dir = std::env::temp_dir().join("nova-process-signal");
+    let _ = std::fs::remove_dir_all(&dir);
+    std::fs::create_dir_all(&dir).expect("a directory");
+    let source = dir.join("main.nova");
+    std::fs::write(
+        &source,
+        "fn main() {\n    if args().len() < 2 { panic(\"as asked\") }\n    \
+         let out = run(current_exe().unwrap(), []).unwrap()\n    \
+         println(\"status ${out.status}\")\n    \
+         println(\"${out.stderr.to_string().unwrap().contains(\"as asked\")}\")\n}\n",
+    )
+    .expect("the program");
+    let exe = build_process_program(&dir.join("bin"), "signal", &source.to_string_lossy());
+    // The parent is the run given an argument, and the child the one given
+    // none, so a child that lost its arguments still panics, and cannot
+    // start a parent again.
+    let out = Command::new(&exe).arg("parent").assert().success();
+    assert_eq!(
+        String::from_utf8_lossy(&out.get_output().stdout),
+        "status 134\ntrue\n"
+    );
+    let _ = std::fs::remove_dir_all(&dir);
+}
+
+/// The value `process_env`'s tests, and the `--release` walk, give
+/// `NOVA_PROCESS_TEST_VALUE`: `=` and non-ASCII included.
+const PROCESS_TEST_VALUE: &str = "a = b, héllo ✓";
+
+/// The value they give `NOVA_PROCESS_TEST_EQ`: with it set, glibc's
+/// `getenv("NOVA_PROCESS_TEST_EQ=x")` finds `y`, and macOS's the whole
+/// value, so the program's `None` for that name is `env`'s own refusal of a
+/// name holding `=`, seen on every system (5.0 spec §5.7).
+const PROCESS_TEST_EQ: &str = "x=y";
+
+/// `env` and `current_exe` (5.0 spec §5.7): a variable the test sets on the
+/// program, one it removes, and names no variable can have, under `nova
+/// run`, where the program sees `nova`'s environment, and built, where it
+/// sees its own.
+#[test]
+fn process_env_runs() {
+    let source = repo_root().join("tests/runtime/process_env.nova");
+    let expected = std::fs::read_to_string(source.with_extension("stdout"))
+        .expect("the .stdout file")
+        .replace("\r\n", "\n");
+    nova()
+        .env("NOVA_PROCESS_TEST_VALUE", PROCESS_TEST_VALUE)
+        .env("NOVA_PROCESS_TEST_EQ", PROCESS_TEST_EQ)
+        .env_remove("NOVA_PROCESS_TEST_UNSET")
+        .arg("run")
+        .arg(&source)
+        .assert()
+        .success()
+        .stdout(expected.clone());
+    let dir = std::env::temp_dir().join("nova-process-env");
+    let exe = build_process_program(&dir, "process_env", &source.to_string_lossy());
+    let out = Command::new(&exe)
+        .env("NOVA_PROCESS_TEST_VALUE", PROCESS_TEST_VALUE)
+        .env("NOVA_PROCESS_TEST_EQ", PROCESS_TEST_EQ)
+        .env_remove("NOVA_PROCESS_TEST_UNSET")
+        .assert()
+        .success();
+    let stdout = String::from_utf8_lossy(&out.get_output().stdout).replace("\r\n", "\n");
+    assert_eq!(stdout, expected);
+    // Without the variable, `env` gives `None`: the line is the program's
+    // reading, not a constant.
+    let out = Command::new(&exe)
+        .env_remove("NOVA_PROCESS_TEST_VALUE")
+        .assert()
+        .success();
+    let stdout = String::from_utf8_lossy(&out.get_output().stdout).replace("\r\n", "\n");
+    assert!(stdout.starts_with("set: None\n"), "{stdout}");
+    let _ = std::fs::remove_dir_all(&dir);
+}
+
+/// `current_exe()` names `nova` under `nova run`, where the program runs
+/// inside it, and the program's own executable when it is built (5.0 spec
+/// §5.7, §10 risk 9), as its docs say.
+#[test]
+fn current_exe_is_nova_under_run_and_the_program_when_built() {
+    let dir = std::env::temp_dir().join("nova-process-current-exe");
+    let _ = std::fs::remove_dir_all(&dir);
+    std::fs::create_dir_all(&dir).expect("a directory");
+    let source = dir.join("main.nova");
+    std::fs::write(&source, "fn main() { println(current_exe().unwrap()) }\n")
+        .expect("the program");
+    let same = |printed: &[u8], want: &std::path::Path| {
+        let printed = String::from_utf8_lossy(printed);
+        let got = std::fs::canonicalize(printed.trim_end()).expect("the printed path exists");
+        let want = std::fs::canonicalize(want).expect("the expected path exists");
+        assert_eq!(got, want);
+    };
+    let out = nova().arg("run").arg(&source).assert().success();
+    same(
+        &out.get_output().stdout,
+        &assert_cmd::cargo::cargo_bin("nova"),
+    );
+    let exe = build_process_program(&dir.join("bin"), "current", &source.to_string_lossy());
+    let out = Command::new(&exe).assert().success();
+    same(&out.get_output().stdout, &exe);
+    let _ = std::fs::remove_dir_all(&dir);
+}
+
 /// A guard on an or-pattern runs once, with the bindings of the first
 /// alternative that matches, and a false guard goes on to the next arm, not
 /// to a later alternative (5.0 spec §4.6; ADR 0049): `(1, 9)` binds `a` to
diff --git a/crates/nova-cli/tests/wasm_tests.rs b/crates/nova-cli/tests/wasm_tests.rs
index e05949b36f3dbb8414f6bcfbc44b15be1ad0740b..22410c709d8c10a54f34cf5292294438954ee3fc 100644
--- a/crates/nova-cli/tests/wasm_tests.rs
+++ b/crates/nova-cli/tests/wasm_tests.rs
@@ -144,6 +144,35 @@ fn a_native_only_function_is_e0092_on_wasm() {
     );
 }
 
+/// `std/process`'s `run`, `status`, `env`, `current_exe` and
+/// `ProcessOutput` are native only, so each is E0092 on WASM (5.0 spec
+/// §5.7, decision 6), and the same program checks natively.
+#[test]
+fn std_process_is_e0092_on_wasm() {
+    for (name, source) in [
+        (
+            "run",
+            "fn main() {\n    let _ = run(\"nova\", [\"--version\"])\n}\n",
+        ),
+        (
+            "status",
+            "fn main() {\n    let _ = status(\"nova\", [])\n}\n",
+        ),
+        ("env", "fn main() {\n    let _ = env(\"HOME\")\n}\n"),
+        ("current_exe", "fn main() {\n    let _ = current_exe()\n}\n"),
+        (
+            "ProcessOutput",
+            "fn code(o: ProcessOutput) -> Int { 0 }\nfn main() { println(\"x\") }\n",
+        ),
+    ] {
+        missing_on_wasm(
+            &format!("process-{name}"),
+            source,
+            &format!("`{name}` is in `std/process`, which target wasm does not have"),
+        );
+    }
+}
+
 /// A type of a native-only module, in a signature.
 #[test]
 fn a_native_only_type_is_e0092_on_wasm() {
@@ -2931,7 +2960,9 @@ const LEFT_OUT: &[(&str, &str)] = &[
     ("net_timeout", "std/fs, std/io, std/net"),
     ("nova_test", "E0601 (a test file, which `nova test` runs)"),
     ("process_args", "std/process"),
+    ("process_env", "std/fs, std/process"),
     ("process_exit", "std/io, std/process"),
+    ("process_run", "std/fs, std/io, std/process"),
 ];
 
 /// Every program in `tests/runtime/` is in the gate or left out, once, and
diff --git a/crates/nova-driver/tests/index_complete.rs b/crates/nova-driver/tests/index_complete.rs
index 2062ba0020bffbae82eef64704235703ddb8e31f..aec7d71b8ccfcce07b96ca5901783ee834b571b3 100644
--- a/crates/nova-driver/tests/index_complete.rs
+++ b/crates/nova-driver/tests/index_complete.rs
@@ -252,8 +252,8 @@ fn every_name_in_the_corpora_is_indexed() {
     // after 5.0c added `array_helpers`; 167 after `hidden_trait_methods`;
     // 168 after `char_and_bytes`; 170 after `string_builder` and
     // `string_builder_million`; 172 after `parse_numbers` and `float_bits`;
-    // 173 after `ordered_map`.
-    assert_eq!(programs.len(), 183, "{programs:#?}");
+    // 173 after `ordered_map`; 175 after `process_run` and `process_env`.
+    assert_eq!(programs.len(), 185, "{programs:#?}");
     let mut files = 0;
     let mut failures: Vec<String> = Vec::new();
     for (k, path) in programs.iter().enumerate() {
@@ -280,11 +280,11 @@ fn every_name_in_the_corpora_is_indexed() {
             every_name_is_covered(&a, file, &mut failures);
         }
     }
-    // 9 + 173 + 3 files of user code, 05-geometry's library's 2, the 8
+    // 9 + 175 + 3 files of user code, 05-geometry's library's 2, the 8
     // modules 07-fullstack-blog's client reaches from its `main.nova` (not
     // the server's 4, which only `server.nova` and `lib.nova` reach), and
     // native std's 22, `std/ui/ssr` among them.
-    assert_eq!(files, 217);
+    assert_eq!(files, 219);
     failures.sort();
     failures.dedup();
     assert!(
diff --git a/crates/nova-lsp/src/completion.rs b/crates/nova-lsp/src/completion.rs
index a53a97c300937c751ea51656cd53be7712717975..c4bc170ddd51d629bbf152b7057399d8bc0270ff 100644
--- a/crates/nova-lsp/src/completion.rs
+++ b/crates/nova-lsp/src/completion.rs
@@ -286,6 +286,21 @@ mod tests {
         }
     }
 
+    /// `std/process`'s new names are offered in every module (5.0 spec
+    /// §5.7).
+    #[test]
+    fn std_process_names_complete() {
+        let text = "fn main() {\n    \n}\n";
+        let names = labels_in(
+            "nova-lsp-complete-process-50c",
+            text,
+            "fn main() {\n    ".len(),
+        );
+        for want in ["ProcessOutput", "run", "status", "env", "current_exe"] {
+            assert!(names.contains(&want.to_string()), "{want}: {names:?}");
+        }
+    }
+
     #[test]
     fn a_signature_stops_at_its_body_outside_parentheses() {
         assert_eq!(
diff --git a/crates/nova-mir/tests/lower_tests.rs b/crates/nova-mir/tests/lower_tests.rs
index ed249a1b0e9813c1b5831af48aa5d4fdff793975..2d3253e5f89d3092757c1e568f7b3e46008a9810 100644
--- a/crates/nova-mir/tests/lower_tests.rs
+++ b/crates/nova-mir/tests/lower_tests.rs
@@ -2475,6 +2475,62 @@ fn array_methods_monomorphise_per_element_type() {
     assert_eq!(slices.len(), 2, "one `slice` per element type: {names:?}");
 }
 
+/// `std/process`'s wrappers call their builtins as the wire contract has
+/// them (5.0 spec §5.7): `run` one `process_run` and one of each take, and
+/// `status` one `process_run` and the status alone; `env` and
+/// `current_exe` hand their text over through `fs_take_string`.
+#[test]
+fn std_process_wrappers_call_their_runtime_builtins() {
+    let mir = mir_for(
+        "fn main() {\n\
+             let o = run(\"p\", [\"a\"])\n\
+             let s = status(\"p\", [])\n\
+             let e = env(\"HOME\")\n\
+             let x = current_exe()\n\
+             println(\"${o.is_ok()} ${s.is_ok()} ${e.is_some()} ${x.is_ok()}\")\n\
+         }",
+    );
+    let names = function_names(&mir);
+    let find = |part: &str| -> String {
+        names
+            .iter()
+            .find(|n| n.starts_with(part))
+            .unwrap_or_else(|| panic!("no function `{part}`: {names:?}"))
+            .to_string()
+    };
+    use nova_mir::RtFunc as R;
+    for (func, wants) in [
+        (
+            "run",
+            &[
+                (R::ProcessRun, 1),
+                (R::ProcessTakeStatus, 1),
+                (R::ProcessTakeStdout, 1),
+                (R::ProcessTakeStderr, 1),
+            ][..],
+        ),
+        (
+            "status",
+            &[
+                (R::ProcessRun, 1),
+                (R::ProcessTakeStatus, 1),
+                (R::ProcessTakeStdout, 0),
+                (R::ProcessTakeStderr, 0),
+            ][..],
+        ),
+        ("env", &[(R::ProcessEnv, 1), (R::FsTakeString, 1)][..]),
+        (
+            "current_exe",
+            &[(R::ProcessCurrentExe, 1), (R::FsTakeString, 1)][..],
+        ),
+    ] {
+        let stmts = stmts_of(&mir, &find(func));
+        for (rt, count) in wants {
+            assert_eq!(count_rt(&stmts, *rt), *count, "`{func}`, {rt:?}: {stmts:?}");
+        }
+    }
+}
+
 /// `OrderedMap`'s methods need `K: Hash + Eq`, as `Map`'s do: a key type
 /// without them is E0013 when the method is instantiated (5.0 spec §5.6).
 #[test]
diff --git a/crates/nova-runtime/src/process.rs b/crates/nova-runtime/src/process.rs
index 3dd38bd5ed1d6d0069e59fcd3e2f8db8c2d66e7e..fdd78eda0d44c6d841a60ae0af62172d01dce741 100644
--- a/crates/nova-runtime/src/process.rs
+++ b/crates/nova-runtime/src/process.rs
@@ -76,6 +76,88 @@ pub extern "C" fn nova_rt_process_exit(code: i64) -> ! {
 mod tests {
     use super::*;
 
+    /// A program found by its bare name on `PATH`, `.exe` included on
+    /// Windows, runs with its arguments as they are, and its status, and on
+    /// Unix both outputs, come back (5.0 spec §5.7). On Windows `cmd`'s own
+    /// reading of its command line is not Rust's, so only the status is
+    /// checked there; `tests/runtime/process_run.nova` checks the output
+    /// of a child that reads its arguments as Rust does, on all three
+    /// systems.
+    #[test]
+    fn a_program_on_path_runs_and_its_output_is_captured() {
+        #[cfg(unix)]
+        {
+            let script = "printf out; printf err >&2; exit 7";
+            let (status, out, err) = run_command("sh", &["-c", script], true).expect("it runs");
+            assert_eq!(status, 7);
+            assert_eq!(out, b"out");
+            assert_eq!(err, b"err");
+        }
+        #[cfg(windows)]
+        {
+            let (status, _, _) = run_command("cmd", &["/C", "exit 7"], true).expect("it runs");
+            assert_eq!(status, 7);
+        }
+    }
+
+    /// More output than a pipe holds, on both streams, does not stall the
+    /// run: both are read at once.
+    #[cfg(unix)]
+    #[test]
+    fn output_larger_than_a_pipe_is_read_whole() {
+        let script = "i=0; while [ $i -lt 20000 ]; do echo 0123456789; echo 0123456789 >&2; \
+                      i=$((i+1)); done";
+        let (status, out, err) = run_command("sh", &["-c", script], true).expect("it runs");
+        assert_eq!(status, 0);
+        assert_eq!(out.len(), 220_000);
+        assert_eq!(err.len(), 220_000);
+    }
+
+    /// A death by signal is `128 + signal`, as shells report it (5.0 spec
+    /// §12 item 22): `kill -9` gives 137.
+    #[cfg(unix)]
+    #[test]
+    fn a_death_by_signal_is_128_plus_the_signal() {
+        let (status, _, _) = run_command("sh", &["-c", "kill -9 $$"], true).expect("it runs");
+        assert_eq!(status, 137);
+    }
+
+    /// A missing program is `NotFound`, the kind `fail` maps to
+    /// `NOT_FOUND`.
+    #[test]
+    fn a_missing_program_is_not_found() {
+        let e = run_command("nova-no-such-program-5-0c", &[], true).expect_err("no program");
+        assert_eq!(e.kind(), std::io::ErrorKind::NotFound);
+        let e = run_command("nova-no-such-program-5-0c", &[], false).expect_err("no program");
+        assert_eq!(e.kind(), std::io::ErrorKind::NotFound);
+    }
+
+    /// A file that is not executable cannot be run: `PermissionDenied`.
+    #[cfg(unix)]
+    #[test]
+    fn a_file_that_cannot_be_run_is_permission_denied() {
+        let dir = std::env::temp_dir().join("nova-runtime-process-denied");
+        let _ = std::fs::remove_dir_all(&dir);
+        std::fs::create_dir_all(&dir).expect("a directory");
+        let file = dir.join("not-executable");
+        std::fs::write(&file, "echo hi\n").expect("a file");
+        let e = run_command(&file.to_string_lossy(), &[], true).expect_err("not runnable");
+        assert_eq!(e.kind(), std::io::ErrorKind::PermissionDenied);
+        let _ = std::fs::remove_dir_all(&dir);
+    }
+
+    /// An empty name, or one with `=` or NUL, is `None` without asking the
+    /// system; an unset name is `None`; a set one is its value (5.0 spec
+    /// §5.7). `PATH` is set on every system the tests run on.
+    #[test]
+    fn env_refuses_names_the_system_cannot_hold() {
+        for name in ["", "A=B", "=", "A\0B", "\0"] {
+            assert_eq!(env_value(name), None, "{name:?}");
+        }
+        assert_eq!(env_value("NOVA_RUNTIME_SURELY_UNSET_5_0C"), None);
+        assert!(env_value("PATH").is_some());
+    }
+
     #[test]
     fn an_index_in_range_is_that_argument() {
         let list = vec!["a".to_string(), "b c".to_string(), String::new()];
diff --git a/crates/nova-typeck/src/check.rs b/crates/nova-typeck/src/check.rs
index b865f62915d1e52ec81e4967d7ae3d361873a689..42b28abac1ee96f263c2ec9947aa1daced3cb2d7 100644
--- a/crates/nova-typeck/src/check.rs
+++ b/crates/nova-typeck/src/check.rs
@@ -17439,6 +17439,33 @@ mod tests {
                     (vec![Ty::Int], Ty::Unit),
                     "`process_exit(code)` in `std/process`'s `exit`",
                 ),
+                Builtin::ProcessRun => (
+                    (
+                        vec![Ty::String, Ty::Array(Box::new(Ty::String)), Ty::Bool],
+                        Ty::Int,
+                    ),
+                    "`process_run(program, args, true)` in `std/process`'s `run`",
+                ),
+                Builtin::ProcessTakeStatus => (
+                    (vec![], Ty::Int),
+                    "`process_take_status()` in `std/process`'s `run` and `status`",
+                ),
+                Builtin::ProcessTakeStdout => (
+                    (vec![], Ty::Bytes),
+                    "`process_take_stdout()` in `std/process`'s `run`",
+                ),
+                Builtin::ProcessTakeStderr => (
+                    (vec![], Ty::Bytes),
+                    "`process_take_stderr()` in `std/process`'s `run`",
+                ),
+                Builtin::ProcessEnv => (
+                    (vec![Ty::String], Ty::Bool),
+                    "`process_env(name)` in `std/process`'s `env`",
+                ),
+                Builtin::ProcessCurrentExe => (
+                    (vec![], Ty::Int),
+                    "`process_current_exe()` in `std/process`'s `current_exe`",
+                ),
                 Builtin::UiStatePresent => (
                     (vec![], Ty::Bool),
                     "`ui_state_present()` in `std/ui`'s `ui`",
@@ -17575,14 +17602,26 @@ mod tests {
         assert_eq!(builtin_signature(b), (vec![], Ty::Bool));
     }
 
-    /// The three `std/process` builtins are `STD_ONLY` and typed as
-    /// `std/process/lib.nova` calls them (docs/adr/0023).
+    /// The nine `std/process` builtins are `STD_ONLY` and typed as
+    /// `std/process/lib.nova` calls them (docs/adr/0023; 5.0 spec §5.7).
     #[test]
     fn the_process_builtins_are_std_only_with_their_signatures() {
         for (name, sig) in [
             ("process_arg_count", (vec![], Ty::Int)),
             ("process_arg", (vec![Ty::Int], Ty::String)),
             ("process_exit", (vec![Ty::Int], Ty::Unit)),
+            (
+                "process_run",
+                (
+                    vec![Ty::String, Ty::Array(Box::new(Ty::String)), Ty::Bool],
+                    Ty::Int,
+                ),
+            ),
+            ("process_take_status", (vec![], Ty::Int)),
+            ("process_take_stdout", (vec![], Ty::Bytes)),
+            ("process_take_stderr", (vec![], Ty::Bytes)),
+            ("process_env", (vec![Ty::String], Ty::Bool)),
+            ("process_current_exe", (vec![], Ty::Int)),
         ] {
             let b = nova_resolver::Builtin::STD_ONLY
                 .iter()
diff --git a/crates/nova-typeck/src/check/std_tests.rs b/crates/nova-typeck/src/check/std_tests.rs
index c6d9433c60e0ca5bc5fd95a815e58b5189bafcf9..0c9b10bdb143c9d1dcf3e35b8bb9030f54d49b05 100644
--- a/crates/nova-typeck/src/check/std_tests.rs
+++ b/crates/nova-typeck/src/check/std_tests.rs
@@ -8,6 +8,7 @@
 //!   program that names one is pointed to;
 //! - the new items' types;
 //! - `OrderedMap`'s types;
+//! - `std/process`'s types, and its builtins kept from user code.
 
 use super::*;
 use nova_diagnostics::{FileId, Severity};
@@ -251,6 +252,27 @@ fn a_builder_mutates_only_through_a_mutable_binding() {
     assert_eq!(e[0].code, "E0060");
 }
 
+/// The six process builtins are std's own (spec §5.7): a program that
+/// names one finds nothing.
+#[test]
+fn the_process_builtins_are_not_visible_to_a_program() {
+    for (call, name) in [
+        ("process_run(\"p\", [], true)", "process_run"),
+        ("process_take_status()", "process_take_status"),
+        ("process_take_stdout()", "process_take_stdout"),
+        ("process_take_stderr()", "process_take_stderr"),
+        ("process_env(\"HOME\")", "process_env"),
+        ("process_current_exe()", "process_current_exe"),
+    ] {
+        let e = errors(&format!("fn main() {{\n    let x = {call}\n}}\n"));
+        assert!(
+            e.iter()
+                .any(|d| d.code == "E0001" && d.message.contains(&format!("`{name}`"))),
+            "{name}: {e:#?}"
+        );
+    }
+}
+
 /// `OrderedMap`'s types (spec §5.6).
 #[test]
 fn ordered_map_has_the_specs_types() {
@@ -277,6 +299,25 @@ fn ordered_map_has_the_specs_types() {
     assert_eq!(local_ty(&r, "es"), Ty::Array(Box::new(pair)));
 }
 
+/// `std/process`'s types (spec §5.7).
+#[test]
+fn std_process_has_the_specs_types() {
+    let r = checked(
+        "fn main() {\n    let o = run(\"p\", [\"a\"]).unwrap()\n    let os = o.status\n    \
+         let oo = o.stdout\n    let oe = o.stderr\n    \
+         let st = status(\"p\", []).unwrap()\n    let ev = env(\"HOME\")\n    \
+         let ce = current_exe().unwrap()\n}\n",
+    );
+    assert!(r.diagnostics.is_empty(), "{:#?}", r.diagnostics);
+    assert!(option(&local_ty(&r, "ev"), Ty::String));
+    for name in ["os", "st"] {
+        assert_eq!(local_ty(&r, name), Ty::Int, "{name}");
+    }
+    assert_eq!(local_ty(&r, "oo"), Ty::Bytes);
+    assert_eq!(local_ty(&r, "oe"), Ty::Bytes);
+    assert_eq!(local_ty(&r, "ce"), Ty::String);
+}
+
 /// `insert` and `remove` take `mut self`, as `Map`'s do.
 #[test]
 fn an_ordered_map_mutates_only_through_a_mutable_binding() {
@@ -288,6 +329,26 @@ fn an_ordered_map_mutates_only_through_a_mutable_binding() {
     assert_eq!(e[0].code, "E0060");
 }
 
+/// `run`, `status` and `env` are in every module's scope, and a program's
+/// own definition of one shadows it, silently, as one of `args` does (spec
+/// §5.7, §10 risk 7).
+#[test]
+fn a_programs_own_run_status_and_env_shadow_stds() {
+    clean(
+        "fn run(n: Int) -> Int { n }\nfn status() -> String { \"ok\" }\n\
+         fn main() {\n    let env = 3\n    let a = run(env)\n    let b = status()\n    \
+         println(\"${a} ${b}\")\n}\n",
+    );
+}
+
+/// `run` takes its arguments as an array of `String`s, not one string a
+/// shell would split (spec §5.7).
+#[test]
+fn run_takes_an_array_of_strings() {
+    let e = errors("fn main() {\n    let _ = run(\"ls\", \"-l\")\n}\n");
+    assert!(e.iter().any(|d| d.code == "E0010"), "{e:#?}");
+}
+
 /// A program's impl that defines a method std's impl on the same type has,
 /// `iter` on arrays, is E0074 labelled at the program's impl, its primary
 /// label, and at std's, which the program cannot change (5.0 spec §5.5;
diff --git a/tests/runtime/process_env.nova b/tests/runtime/process_env.nova
new file mode 100644
index 0000000000000000000000000000000000000000..c27c60bc4f0f7ec0c77f4d43d0faec42fe7061ef
--- /dev/null
+++ b/tests/runtime/process_env.nova
@@ -0,0 +1,31 @@
+// `std/process`'s `env` and `current_exe` (5.0 spec §5.7). Its tests, and
+// the `--release` walk, set `NOVA_PROCESS_TEST_VALUE` on the program to the
+// value below, `=` and non-ASCII included, and `NOVA_PROCESS_TEST_EQ` to
+// `x=y`, and leave `NOVA_PROCESS_TEST_UNSET` unset; a name that is empty or
+// holds `=` or NUL is `None` without asking the system. Under `nova run`,
+// the program runs inside `nova`, so it sees `nova`'s environment, which the
+// test sets, and `current_exe()` is `nova`'s path; built, both are the
+// program's own.
+
+fn show(v: Option<String>) -> String {
+    match v {
+        Some(s) => "Some([${s}])"
+        None => "None"
+    }
+}
+
+async fn main() {
+    println("set: ${show(env("NOVA_PROCESS_TEST_VALUE"))}")
+    println("unset: ${show(env("NOVA_PROCESS_TEST_UNSET"))}")
+    println("empty name: ${show(env(""))}")
+    // The system would answer this one: glibc's `getenv` finds `y` for
+    // `NOVA_PROCESS_TEST_EQ=x` in `NOVA_PROCESS_TEST_EQ=x=y`, and macOS's the
+    // whole value, so `None` here is `env`'s own refusal. On Windows `=C:`
+    // names a variable the system keeps hidden.
+    println("name with =: ${show(env("NOVA_PROCESS_TEST_EQ=x"))}")
+    println("hidden name: ${show(env("=C:"))}")
+    println("name with NUL: ${show(env("NOVA_PROCESS_TEST_VALUE\0"))}")
+
+    let exe = current_exe().unwrap()
+    println("current_exe exists: ${exists(exe).await}")
+}
diff --git a/tests/runtime/process_env.stdout b/tests/runtime/process_env.stdout
new file mode 100644
index 0000000000000000000000000000000000000000..77b3bc2dd878d71ebfa16090495d4ba706cdd0a1
--- /dev/null
+++ b/tests/runtime/process_env.stdout
@@ -0,0 +1,7 @@
+set: Some([a = b, héllo ✓])
+unset: None
+empty name: None
+name with =: None
+hidden name: None
+name with NUL: None
+current_exe exists: true
diff --git a/tests/runtime/process_run.nova b/tests/runtime/process_run.nova
new file mode 100644
index 0000000000000000000000000000000000000000..6e2d4a3fd5ea84809e4a32c2499f4183a824b231
--- /dev/null
+++ b/tests/runtime/process_run.nova
@@ -0,0 +1,111 @@
+// `std/process`'s `run` and `status` (5.0 spec §5.7): the program runs
+// itself, found through `current_exe()`, as a child its first argument
+// names. The child prints each argument it was given, between brackets, to
+// its standard output, a line to its standard error, and exits with the
+// status its second argument names. Arguments with spaces, quotes, a
+// trailing backslash, an empty one and non-ASCII ones reach it as they are,
+// with no shell, on Windows too; more output than a pipe holds is captured
+// whole; `status` lets the child print where this program prints, in order,
+// after what this program wrote but had not flushed; a missing program is
+// `NotFound`. Built, not run with `nova run`, under
+// which `current_exe()` is `nova` itself.
+
+fn child(argv: Vec<String>) {
+    let code = argv.at(2).parse_int().unwrap_or(99)
+    if argv.at(1) == "big" {
+        for i in 0..5000 { println("line ${i} of the big output") }
+        eprintln("big stderr")
+        exit(code)
+    }
+    for i in 3..argv.len() { println("arg ${i - 3}: [${argv.at(i)}]") }
+    eprintln("child stderr, status ${code}")
+    exit(code)
+}
+
+// `[mode, code]` and then `rest`.
+fn child_args(mode: String, code: String, rest: [String]) -> [String] {
+    let mut v = Vec::from_array([mode, code])
+    for s in rest { v.push(s) }
+    v.to_array()
+}
+
+fn text(b: Bytes) -> String { b.to_string().unwrap_or("<not UTF-8>") }
+
+fn kind(e: IoError) -> String {
+    match e.kind {
+        NotFound => "NotFound"
+        PermissionDenied => "PermissionDenied"
+        _ => "another kind"
+    }
+}
+
+async fn main() {
+    let argv = args()
+    if argv.len() > 2 {
+        child(argv)
+        return
+    }
+    // A child that lost its arguments would run this part again, and start
+    // a child of its own, without end. The marker, in the temporary
+    // directory its tests give it, stops the second parent: it exists while
+    // the first one's children run, and a child that has its arguments never
+    // looks. A run that stops part way leaves it, so the tests clear the
+    // directory first.
+    let marker = "${temp_dir()}/nova-process-run-parent"
+    if exists(marker).await {
+        eprintln(
+            "process_run: ${marker} exists: a child lost its arguments, or an earlier run stopped",
+        )
+        exit(1)
+    }
+    write_string(marker, "").await.unwrap()
+    let exe = current_exe().unwrap()
+
+    let odd = [
+        "plain",
+        "two words",
+        "",
+        "say \"hi\"",
+        "back\\slash\\",
+        "tab\there",
+        "héllo wörld ✓",
+        "日本語",
+        "--flag=1",
+        "*",
+    ]
+    match run(exe, child_args("child", "3", odd)) {
+        Ok(out) => {
+            println("run: status ${out.status}")
+            print(text(out.stdout))
+            println("run: stderr ${text(out.stderr)}")
+        }
+        Err(e) => println("run failed: ${e.message}")
+    }
+
+    let quiet = run(exe, ["child", "0"]).unwrap()
+    println("no arguments: status ${quiet.status}, ${quiet.stdout.len()} bytes out")
+    println("status 255: ${run(exe, ["child", "255"]).unwrap().status}")
+
+    let big = run(exe, ["big", "4"]).unwrap()
+    let lines = text(big.stdout).split("\n")
+    println(
+        "big: status ${big.status}, ${big.stdout.len()} bytes, ${lines.len() - 1} lines, last ${lines[lines.len() - 2]}",
+    )
+    println("big: stderr ${text(big.stderr)}")
+
+    // Written through `Write`, which leaves a line without its break in
+    // the buffer: `status` flushes it before the child prints.
+    let _ = stdout().write(bytes_from_string("before status, unflushed, then ")).await
+    let code = status(exe, ["child", "5", "inherited", "ü"]).unwrap()
+    println("status: ${code}")
+
+    match run("nova-no-such-program-5-0c", ["x"]) {
+        Ok(_) => println("missing: ran")
+        Err(e) => println("missing run: ${kind(e)}")
+    }
+    match status("nova-no-such-program-5-0c", []) {
+        Ok(_) => println("missing: ran")
+        Err(e) => println("missing status: ${kind(e)}, message given: ${e.message.len() > 0}")
+    }
+    remove_file(marker).await.unwrap()
+}
diff --git a/tests/runtime/process_run.stdout b/tests/runtime/process_run.stdout
new file mode 100644
index 0000000000000000000000000000000000000000..b1f878100d8fc4d9259183ce0c9ec4fcbe7b7681
--- /dev/null
+++ b/tests/runtime/process_run.stdout
@@ -0,0 +1,23 @@
+run: status 3
+arg 0: [plain]
+arg 1: [two words]
+arg 2: []
+arg 3: [say "hi"]
+arg 4: [back\slash\]
+arg 5: [tab	here]
+arg 6: [héllo wörld ✓]
+arg 7: [日本語]
+arg 8: [--flag=1]
+arg 9: [*]
+run: stderr child stderr, status 3
+
+no arguments: status 0, 0 bytes out
+status 255: 255
+big: status 4, 138890 bytes, 5000 lines, last line 4999 of the big output
+big: stderr big stderr
+
+before status, unflushed, then arg 0: [inherited]
+arg 1: [ü]
+status: 5
+missing run: NotFound
+missing status: NotFound, message given: true
````

**Patch `t7-code`:**

<!-- patch t7-code -->

````diff
diff --git a/crates/nova-mir/src/lib.rs b/crates/nova-mir/src/lib.rs
index 792c43391169b9ca1fddd2a83b7ddd8b8b9e71c7..aeeab3219a89f92d100f0ac46415f9394ede8b77 100644
--- a/crates/nova-mir/src/lib.rs
+++ b/crates/nova-mir/src/lib.rs
@@ -548,6 +548,26 @@ rt_funcs! {
     ProcessArg,
     /// `(i64 code) -> unit` — `std::process::exit`; never returns.
     ProcessExit,
+    /// `(ptr program, ptr args, i8 capture) -> i64` — start `program` with
+    /// the `[String]` `args`, no shell, and wait for it; `capture` pipes its
+    /// standard output and error, and otherwise it inherits all three
+    /// streams. `std/fs`'s status code: on `0` the exit status, and the
+    /// output if captured, wait for the three takes below
+    /// (`crates/nova-runtime/src/process.rs`; 5.0 spec §5.7).
+    ProcessRun,
+    /// `() -> i64` — the last run's exit status, `128 + signal` for a death
+    /// by signal on Unix.
+    ProcessTakeStatus,
+    /// `() -> ptr` — the last captured run's standard output, as `Bytes`.
+    ProcessTakeStdout,
+    /// `() -> ptr` — the last captured run's standard error, as `Bytes`.
+    ProcessTakeStderr,
+    /// `(ptr name) -> i8` — whether the environment variable `name` is set
+    /// to UTF-8; its value then waits for `FsTakeString`.
+    ProcessEnv,
+    /// `() -> i64` — `std/fs`'s status code; on `0` the running
+    /// executable's path waits for `FsTakeString`.
+    ProcessCurrentExe,
     /// `() -> i8` — whether `std/ui` has stored its state for this thread
     /// (`crates/nova-runtime/src/ui.rs`; spec 4.3 §4).
     UiStatePresent,
@@ -668,6 +688,12 @@ impl RtFunc {
             RtFunc::ProcessArgCount => "nova_rt_process_arg_count",
             RtFunc::ProcessArg => "nova_rt_process_arg",
             RtFunc::ProcessExit => "nova_rt_process_exit",
+            RtFunc::ProcessRun => "nova_rt_process_run",
+            RtFunc::ProcessTakeStatus => "nova_rt_process_take_status",
+            RtFunc::ProcessTakeStdout => "nova_rt_process_take_stdout",
+            RtFunc::ProcessTakeStderr => "nova_rt_process_take_stderr",
+            RtFunc::ProcessEnv => "nova_rt_process_env",
+            RtFunc::ProcessCurrentExe => "nova_rt_process_current_exe",
             RtFunc::UiStatePresent => "nova_rt_ui_state_present",
             RtFunc::UiStateLoad => "nova_rt_ui_state_load",
             RtFunc::UiStateStore => "nova_rt_ui_state_store",
@@ -874,6 +900,10 @@ impl RtFunc {
             RtFunc::ProcessArgCount => (vec![], MirTy::I64),
             RtFunc::ProcessArg => (vec![MirTy::I64], MirTy::Ptr),
             RtFunc::ProcessExit => (vec![MirTy::I64], MirTy::Unit),
+            RtFunc::ProcessRun => (vec![MirTy::Ptr, MirTy::Ptr, MirTy::I8], MirTy::I64),
+            RtFunc::ProcessTakeStatus | RtFunc::ProcessCurrentExe => (vec![], MirTy::I64),
+            RtFunc::ProcessTakeStdout | RtFunc::ProcessTakeStderr => (vec![], MirTy::Ptr),
+            RtFunc::ProcessEnv => (vec![MirTy::Ptr], MirTy::I8),
             RtFunc::UiStatePresent => (vec![], MirTy::I8),
             RtFunc::UiStateLoad => (vec![], MirTy::Ptr),
             RtFunc::UiStateStore => (vec![MirTy::Ptr], MirTy::Unit),
@@ -1047,7 +1077,13 @@ impl RtFunc {
             | RtFunc::ShutdownRequested
             | RtFunc::ProcessArgCount
             | RtFunc::ProcessArg
-            | RtFunc::ProcessExit => WasmRuntime::Unavailable,
+            | RtFunc::ProcessExit
+            | RtFunc::ProcessRun
+            | RtFunc::ProcessTakeStatus
+            | RtFunc::ProcessTakeStdout
+            | RtFunc::ProcessTakeStderr
+            | RtFunc::ProcessEnv
+            | RtFunc::ProcessCurrentExe => WasmRuntime::Unavailable,
         }
     }
 }
diff --git a/crates/nova-mir/src/lower.rs b/crates/nova-mir/src/lower.rs
index fb82ee5a60dba640279501c040c3bb14ddb227f7..0f35ec9e082e848eaa136a3bf85ef7ae3c57e088 100644
--- a/crates/nova-mir/src/lower.rs
+++ b/crates/nova-mir/src/lower.rs
@@ -800,6 +800,12 @@ impl<'a> Lowerer<'a> {
                     Builtin::ProcessArgCount => Lowering::Runtime(RtFunc::ProcessArgCount),
                     Builtin::ProcessArg => Lowering::Runtime(RtFunc::ProcessArg),
                     Builtin::ProcessExit => Lowering::Runtime(RtFunc::ProcessExit),
+                    Builtin::ProcessRun => Lowering::Runtime(RtFunc::ProcessRun),
+                    Builtin::ProcessTakeStatus => Lowering::Runtime(RtFunc::ProcessTakeStatus),
+                    Builtin::ProcessTakeStdout => Lowering::Runtime(RtFunc::ProcessTakeStdout),
+                    Builtin::ProcessTakeStderr => Lowering::Runtime(RtFunc::ProcessTakeStderr),
+                    Builtin::ProcessEnv => Lowering::Runtime(RtFunc::ProcessEnv),
+                    Builtin::ProcessCurrentExe => Lowering::Runtime(RtFunc::ProcessCurrentExe),
                     Builtin::UiStatePresent => Lowering::Runtime(RtFunc::UiStatePresent),
                     Builtin::UiStateLoad => Lowering::Runtime(RtFunc::UiStateLoad),
                     Builtin::UiStateStore => Lowering::Runtime(RtFunc::UiStateStore),
diff --git a/crates/nova-resolver/src/lib.rs b/crates/nova-resolver/src/lib.rs
index 387fec6f6276ae5606fcb8ca04c27b584f3e987d..f09ecd9c46b26b7ace69737c1608f111a707378b 100644
--- a/crates/nova-resolver/src/lib.rs
+++ b/crates/nova-resolver/src/lib.rs
@@ -962,6 +962,45 @@ builtins! {
     /// after flushing stdout, and never returns. Runtime symbol
     /// `nova_rt_process_exit`. Std-only.
     ProcessExit,
+    /// `process_run(program: String, args: [String], capture: Bool) -> Int`
+    /// — start `program`, a path or a name searched for on `PATH`, with
+    /// `args` passed as they are, no shell, and wait for it. With `capture`
+    /// its standard input is empty and its standard output and error are
+    /// piped; without, it inherits all three streams. Returns `std/fs`'s
+    /// status code: `0`, and then the exit status, and the output when
+    /// captured, wait for [`Builtin::ProcessTakeStatus`],
+    /// [`Builtin::ProcessTakeStdout`] and [`Builtin::ProcessTakeStderr`];
+    /// otherwise an `IoErrorKind`'s code, with the message waiting for
+    /// `fs_last_error_message`. Backs `std/process`'s `run` and `status`
+    /// (5.0 spec §5.7). Runtime symbol `nova_rt_process_run`
+    /// (`crates/nova-runtime/src/process.rs`). Std-only.
+    ProcessRun,
+    /// `process_take_status() -> Int` — the exit status of the last
+    /// [`Builtin::ProcessRun`] that started its program: the exit code, or
+    /// `128 + signal` for a death by signal on Unix. Runtime symbol
+    /// `nova_rt_process_take_status`. Std-only.
+    ProcessTakeStatus,
+    /// `process_take_stdout() -> Bytes` — the standard output of the last
+    /// captured [`Builtin::ProcessRun`], held by the runtime until taken,
+    /// and then empty. Runtime symbol `nova_rt_process_take_stdout`.
+    /// Std-only.
+    ProcessTakeStdout,
+    /// `process_take_stderr() -> Bytes` — the standard error of the last
+    /// captured [`Builtin::ProcessRun`], as
+    /// [`Builtin::ProcessTakeStdout`]. Runtime symbol
+    /// `nova_rt_process_take_stderr`. Std-only.
+    ProcessTakeStderr,
+    /// `process_env(name: String) -> Bool` — whether the environment
+    /// variable `name` is set to UTF-8 text; its value then waits for
+    /// `fs_take_string`. `false` for an empty `name`, or one that holds `=`
+    /// or NUL, without asking the system. Backs `std/process`'s `env`.
+    /// Runtime symbol `nova_rt_process_env`. Std-only.
+    ProcessEnv,
+    /// `process_current_exe() -> Int` — `std/fs`'s status code; on `0` the
+    /// running executable's path waits for `fs_take_string`. Backs
+    /// `std/process`'s `current_exe`. Runtime symbol
+    /// `nova_rt_process_current_exe`. Std-only.
+    ProcessCurrentExe,
     /// `ui_state_present() -> Bool` — whether `std/ui` has stored its state
     /// for this thread, or this page (spec
     /// `docs/superpowers/specs/2026-10-17-phase-4-3-std-ui-reactivity-and-views-design.md`
@@ -1150,6 +1189,12 @@ impl Builtin {
             Builtin::ProcessArgCount => "process_arg_count",
             Builtin::ProcessArg => "process_arg",
             Builtin::ProcessExit => "process_exit",
+            Builtin::ProcessRun => "process_run",
+            Builtin::ProcessTakeStatus => "process_take_status",
+            Builtin::ProcessTakeStdout => "process_take_stdout",
+            Builtin::ProcessTakeStderr => "process_take_stderr",
+            Builtin::ProcessEnv => "process_env",
+            Builtin::ProcessCurrentExe => "process_current_exe",
             Builtin::UiStatePresent => "ui_state_present",
             Builtin::UiStateLoad => "ui_state_load",
             Builtin::UiStateStore => "ui_state_store",
@@ -1196,7 +1241,7 @@ impl Builtin {
     /// consecutive review rounds (see the Phase 2.2b whole-branch review),
     /// because the roster is duplicated information that only this array
     /// needs to stay exact.
-    pub const STD_ONLY: [Builtin; 94] = [
+    pub const STD_ONLY: [Builtin; 100] = [
         Builtin::StrCmp,
         Builtin::StrHash,
         Builtin::CharToInt,
@@ -1285,6 +1330,12 @@ impl Builtin {
         Builtin::ProcessArgCount,
         Builtin::ProcessArg,
         Builtin::ProcessExit,
+        Builtin::ProcessRun,
+        Builtin::ProcessTakeStatus,
+        Builtin::ProcessTakeStdout,
+        Builtin::ProcessTakeStderr,
+        Builtin::ProcessEnv,
+        Builtin::ProcessCurrentExe,
         Builtin::UiStatePresent,
         Builtin::UiStateLoad,
         Builtin::UiStateStore,
diff --git a/crates/nova-runtime/src/lib.rs b/crates/nova-runtime/src/lib.rs
index 12380288ff2fc3b161af16ea5648699a9b9a7c4e..7b08ab0669dd25e2b16ba7b9f26668152b374a4c 100644
--- a/crates/nova-runtime/src/lib.rs
+++ b/crates/nova-runtime/src/lib.rs
@@ -1281,6 +1281,30 @@ pub fn symbols() -> Vec<(&'static str, *const u8)> {
             "nova_rt_process_exit",
             process::nova_rt_process_exit as *const u8,
         ),
+        (
+            "nova_rt_process_run",
+            process::nova_rt_process_run as *const u8,
+        ),
+        (
+            "nova_rt_process_take_status",
+            process::nova_rt_process_take_status as *const u8,
+        ),
+        (
+            "nova_rt_process_take_stdout",
+            process::nova_rt_process_take_stdout as *const u8,
+        ),
+        (
+            "nova_rt_process_take_stderr",
+            process::nova_rt_process_take_stderr as *const u8,
+        ),
+        (
+            "nova_rt_process_env",
+            process::nova_rt_process_env as *const u8,
+        ),
+        (
+            "nova_rt_process_current_exe",
+            process::nova_rt_process_current_exe as *const u8,
+        ),
         (
             "nova_rt_ui_state_present",
             ui::nova_rt_ui_state_present as *const u8,
diff --git a/crates/nova-runtime/src/process.rs b/crates/nova-runtime/src/process.rs
index fdd78eda0d44c6d841a60ae0af62172d01dce741..373df7d3fdfa83d668cbd3b5a724fa6a3d3fbf36 100644
--- a/crates/nova-runtime/src/process.rs
+++ b/crates/nova-runtime/src/process.rs
@@ -18,9 +18,35 @@
 //!
 //! `pub`, so `crates/nova-runtime/tests/process_args_*.rs` can drive it, and
 //! so `nova-codegen-cranelift` can call [`set_args`].
+//!
+//! # Other programs, and the environment
+//!
+//! The runtime half of `std/process`'s `run`, `status`, `env` and
+//! `current_exe` (5.0 spec §5.7) is on `std/fs`'s wire contract
+//! (`crate::fs`'s module doc): each fallible builtin returns a status code,
+//! `0` or an `IoErrorKind`'s, and its payload waits to be taken.
+//! [`nova_rt_process_run`] starts a program through `std::process::Command`,
+//! which searches `PATH` for a bare name, `.exe` included on Windows, and
+//! passes each argument as it is, with no shell; it blocks the thread until
+//! the child ends. The child's exit status, and its output when captured,
+//! wait in this thread's [`STATUS`], [`STDOUT`] and [`STDERR`] for the three
+//! `nova_rt_process_take_*` builtins. They are Rust's own bytes, not GC
+//! objects, so nothing is rooted while they wait: the `Bytes` is allocated
+//! when it is taken. A failure goes through `crate::fs::fail`, which stashes
+//! its message for `nova_rt_fs_last_error_message`, as `std/fs`'s does.
+//! [`nova_rt_process_env`] and [`nova_rt_process_current_exe`] stash their
+//! text for `nova_rt_fs_take_string`.
+//!
+//! A thread's cells, not a task's slots: `run` and `status` are plain `fn`s
+//! whose wrappers take the payload straight after the run, with no `.await`
+//! between, so no other task runs in between on this thread. Nothing here
+//! panics, since a builtin can be called from an `async fn`'s poll: each
+//! cell is a `Cell` that is only swapped.
 
+use std::cell::Cell;
 use std::sync::OnceLock;
 
+use crate::fs::{fail, stash, Slot, OK};
 use crate::NovaStr;
 
 /// The program's arguments, once set or read.
@@ -72,6 +98,166 @@ pub extern "C" fn nova_rt_process_exit(code: i64) -> ! {
     std::process::exit(code as i32)
 }
 
+thread_local! {
+    /// The exit status of this thread's last run, until taken.
+    static STATUS: Cell<i64> = const { Cell::new(0) };
+    /// The standard output of this thread's last captured run, until taken.
+    static STDOUT: Cell<Vec<u8>> = const { Cell::new(Vec::new()) };
+    /// The standard error of this thread's last captured run, until taken.
+    static STDERR: Cell<Vec<u8>> = const { Cell::new(Vec::new()) };
+}
+
+/// What a finished child's status is to Nova: its exit code, or, for a
+/// death by signal on Unix, `128 + signal`, as shells report it (5.0 spec
+/// §5.7, §12 item 22). Windows always has a code, read as Rust's `i32`.
+/// `-1` stands for a status that is neither, which no system reports.
+fn exit_code(status: std::process::ExitStatus) -> i64 {
+    if let Some(code) = status.code() {
+        return i64::from(code);
+    }
+    #[cfg(unix)]
+    {
+        use std::os::unix::process::ExitStatusExt;
+        if let Some(signal) = status.signal() {
+            return 128 + i64::from(signal);
+        }
+    }
+    -1
+}
+
+/// Runs `program` with `args` and waits for it. With `capture`, its
+/// standard input is empty and its standard output and error are read
+/// whole, both at once, so a child that fills one pipe while the other is
+/// read cannot stall; without, it inherits all three of this process's
+/// streams, after this process's standard output is flushed, so what was
+/// printed before comes first. Returns the exit status and the two outputs,
+/// empty when not captured.
+fn run_command(
+    program: &str,
+    args: &[&str],
+    capture: bool,
+) -> std::io::Result<(i64, Vec<u8>, Vec<u8>)> {
+    use std::io::Write;
+    use std::process::{Command, Stdio};
+    let _ = std::io::stdout().flush();
+    let mut command = Command::new(program);
+    command.args(args);
+    if capture {
+        let out = command.stdin(Stdio::null()).output()?;
+        Ok((exit_code(out.status), out.stdout, out.stderr))
+    } else {
+        let status = command.status()?;
+        Ok((exit_code(status), Vec::new(), Vec::new()))
+    }
+}
+
+/// The value of the environment variable `name`, or `None` when it is
+/// unset or not UTF-8. A `name` that is empty or holds `=` or NUL is
+/// `None` without asking the system, which may panic on one (5.0 spec
+/// §5.7).
+fn env_value(name: &str) -> Option<String> {
+    if name.is_empty() || name.contains('=') || name.contains('\0') {
+        return None;
+    }
+    std::env::var(name).ok()
+}
+
+/// `process_run(program: String, args: [String], capture: Bool) -> Int`:
+/// runs `program`, as [`run_command`] does. Returns `OK`, the exit status
+/// and the output then waiting for the `nova_rt_process_take_*` builtins,
+/// or an `IoErrorKind`'s status code, through `crate::fs::fail`: a missing
+/// program is `NOT_FOUND`, one that cannot be run `PERMISSION_DENIED`.
+///
+/// # Safety
+/// `program` must point to a live `NovaStr`, and `args` to a live Nova
+/// `[String]`: its length in word 0, a `NovaStr` pointer in each word after.
+#[no_mangle]
+pub unsafe extern "C" fn nova_rt_process_run(
+    program: *const NovaStr,
+    args: *const u8,
+    capture: i8,
+) -> i64 {
+    let words = args as *const i64;
+    // SAFETY: forwarding this function's own contract.
+    let n = unsafe { *words }.max(0) as usize;
+    let list: Vec<&str> = (0..n)
+        // SAFETY: word `1 + i` is element `i` of the array, a `NovaStr`.
+        .map(|i| unsafe { crate::as_str(*words.add(1 + i) as *const NovaStr) })
+        .collect();
+    // SAFETY: forwarding this function's own contract.
+    let program = unsafe { crate::as_str(program) };
+    match run_command(program, &list, capture != 0) {
+        Ok((status, out, err)) => {
+            STATUS.with(|c| c.set(status));
+            STDOUT.with(|c| c.set(out));
+            STDERR.with(|c| c.set(err));
+            OK
+        }
+        Err(e) => fail(&e),
+    }
+}
+
+/// `process_take_status() -> Int`: the last run's exit status.
+#[no_mangle]
+pub extern "C" fn nova_rt_process_take_status() -> i64 {
+    STATUS.with(|c| c.replace(0))
+}
+
+/// `process_take_stdout() -> Bytes`: the last captured run's standard
+/// output, which is then empty.
+#[no_mangle]
+pub extern "C" fn nova_rt_process_take_stdout() -> *mut NovaStr {
+    crate::bytes::gc_bytes(&STDOUT.with(Cell::take))
+}
+
+/// `process_take_stderr() -> Bytes`: the last captured run's standard
+/// error, which is then empty.
+#[no_mangle]
+pub extern "C" fn nova_rt_process_take_stderr() -> *mut NovaStr {
+    crate::bytes::gc_bytes(&STDERR.with(Cell::take))
+}
+
+/// `process_env(name: String) -> Bool`: whether `name` is set to UTF-8
+/// text, as [`env_value`] says; its value then waits for
+/// `nova_rt_fs_take_string`.
+///
+/// # Safety
+/// `name` must point to a live `NovaStr`.
+#[no_mangle]
+pub unsafe extern "C" fn nova_rt_process_env(name: *const NovaStr) -> i8 {
+    // SAFETY: forwarding this function's own contract.
+    match env_value(unsafe { crate::as_str(name) }) {
+        Some(value) => {
+            stash(Slot::Buffer, crate::gc_str(&value));
+            1
+        }
+        None => 0,
+    }
+}
+
+/// `process_current_exe() -> Int`: `OK`, with the running executable's
+/// path waiting for `nova_rt_fs_take_string`, or an `IoErrorKind`'s status
+/// code. Under `nova run` the executable is `nova` itself. A path that is
+/// not valid Unicode is `INVALID_DATA`, as a file that is not UTF-8 is to
+/// `read_to_string`: a path with its bad bytes replaced would name another
+/// file.
+#[no_mangle]
+pub extern "C" fn nova_rt_process_current_exe() -> i64 {
+    match std::env::current_exe() {
+        Ok(path) => match path.to_str() {
+            Some(text) => {
+                stash(Slot::Buffer, crate::gc_str(text));
+                OK
+            }
+            None => fail(&std::io::Error::new(
+                std::io::ErrorKind::InvalidData,
+                "the executable's path is not valid Unicode",
+            )),
+        },
+        Err(e) => fail(&e),
+    }
+}
+
 #[cfg(test)]
 mod tests {
     use super::*;
diff --git a/crates/nova-typeck/src/check.rs b/crates/nova-typeck/src/check.rs
index 42b28abac1ee96f263c2ec9947aa1daced3cb2d7..7ada1cd73293d54de268aa387f5a1845353ef7e2 100644
--- a/crates/nova-typeck/src/check.rs
+++ b/crates/nova-typeck/src/check.rs
@@ -4579,6 +4579,12 @@ impl<'a> Checker<'a> {
             | Builtin::ProcessArgCount
             | Builtin::ProcessArg
             | Builtin::ProcessExit
+            | Builtin::ProcessRun
+            | Builtin::ProcessTakeStatus
+            | Builtin::ProcessTakeStdout
+            | Builtin::ProcessTakeStderr
+            | Builtin::ProcessEnv
+            | Builtin::ProcessCurrentExe
             | Builtin::UiStatePresent
             | Builtin::UiStateLoad
             | Builtin::UiStateStore
@@ -9096,6 +9102,13 @@ pub(crate) fn builtin_signature(builtin: Builtin) -> (Vec<Ty>, Ty) {
         Builtin::ProcessArgCount => (vec![], Ty::Int),
         Builtin::ProcessArg => (vec![Ty::Int], Ty::String),
         Builtin::ProcessExit => (vec![Ty::Int], Ty::Unit),
+        Builtin::ProcessRun => (
+            vec![Ty::String, Ty::Array(Box::new(Ty::String)), Ty::Bool],
+            Ty::Int,
+        ),
+        Builtin::ProcessTakeStatus | Builtin::ProcessCurrentExe => (vec![], Ty::Int),
+        Builtin::ProcessTakeStdout | Builtin::ProcessTakeStderr => (vec![], Ty::Bytes),
+        Builtin::ProcessEnv => (vec![Ty::String], Ty::Bool),
         Builtin::UiStatePresent => (vec![], Ty::Bool),
         Builtin::UiStateLoad => (vec![], Ty::Param(0)),
         Builtin::UiStateStore => (vec![Ty::Param(0)], Ty::Unit),
diff --git a/std/process/lib.nova b/std/process/lib.nova
index d4e0965f1fbdc34b4cc489b72853c0da0ccecc70..730a299d1a57fc773d7b0080676d470e96c3ddb1 100644
--- a/std/process/lib.nova
+++ b/std/process/lib.nova
@@ -1,10 +1,16 @@
-//! The running program: its command-line arguments, and ending it with an
-//! exit status.
+//! The running program and others: its command-line arguments, ending it
+//! with an exit status, running another program, and reading the
+//! environment.
 //!
 //! `args()` returns the arguments the program was started with, and
-//! `exit(code)` ends it at once with the status `code`. Every module sees
-//! these names without an import, and a module's own `args` or `exit`
-//! shadows the one here.
+//! `exit(code)` ends it at once with the status `code`. `run` runs another
+//! program and captures what it prints, and `status` runs one that prints
+//! where this program does; both wait for it to end. `env(name)` reads an
+//! environment variable, and `current_exe()` names the running executable.
+//! Every module sees these names without an import, and a module's own
+//! definition of one of them, such as a `run` or an `env`, shadows the one
+//! here. The module is native only: on WASM, naming one of these is an
+//! error.
 //!
 //! Reading the program's arguments. `exit` ends the program at once, so the
 //! example does not call it:
@@ -20,6 +26,11 @@
 //! let mut mine = args()
 //! mine.push("--verbose")
 //! assert_eq(args().len(), argv.len())
+//!
+//! // The environment, and the running executable, which `run` can start.
+//! let verbose = env("NOVA_EXAMPLE_VERBOSE").is_some()
+//! assert(!verbose || argv.len() >= 1, "env is None for an unset variable")
+//! assert(current_exe().is_ok(), "the system names the executable")
 //! ```
 
 // Nova standard library -- the running process: its arguments, and a way to
@@ -29,9 +40,24 @@
 // `args()` and `exit(1)` need no `import` (docs/adr/0004-stdlib-compile-model.md).
 // A program's own `args` or `exit` shadows these.
 //
-// Of `nova-spec/20-STDLIB.md`'s module-index entry, "spawn, env, args", only
-// `args` ships, with `exit`; `spawn` and `env` do not exist. See that file's
-// section 17, and docs/adr/0023-program-arguments.md.
+// Of `nova-spec/20-STDLIB.md`'s module-index entry, "spawn, env, args",
+// `args` ships, with `exit`, and since 5.0c `env`, with `run` and `status` in
+// place of `spawn` and `current_exe` beside them (docs/adr/0050). See that
+// file's section 17, and docs/adr/0023-program-arguments.md.
+//
+// # Running programs
+//
+// `run` and `status` are plain `fn`s that block the thread, executor
+// included, until the child ends (5.0 spec §5.7, decision 6): a compiler's
+// driver runs `clang` and waits for it, and `args()` is a plain `fn` too.
+// Each is on `std/fs`'s wire contract: `process_run` returns a status code,
+// `0` or an `IoErrorKind`'s (`io_error_kind_of`), and the child's exit status
+// and output wait in the runtime for the `process_take_*` builtins
+// (`crates/nova-runtime/src/process.rs`). As in `std/fs`, each wrapper takes
+// its payload straight after the call, with no `.await` between; a plain
+// `fn` has none to put there. `env` and `current_exe` hand their text over
+// through `fs_take_string`, and every failure's message through
+// `fs_last_error_message`, as `std/fs`'s do.
 
 // The program's arguments. Index 0 is the program itself: under
 // `nova run [FILE] -- ARGS` it is FILE as written (`src/main.nova` when
@@ -72,3 +98,122 @@ pub fn args() -> Vec<String> {
 /// 255, and Windows 32 bits. Under `nova run` it ends the `nova` process
 /// itself. Code after a call to `exit` never runs.
 pub fn exit(code: Int) { process_exit(code) }
+
+// What `run` returns: the child's exit status and everything it printed.
+// Named `ProcessOutput`, not `Output`, which trait code uses for an
+// associated type (5.0 spec §12 item 22).
+/// What a program that `run` ran left behind: its exit status, and what it
+/// wrote to its standard output and standard error.
+///
+/// ```
+/// let out = ProcessOutput { status: 0, stdout: bytes_from_string("hi\n"), stderr: bytes_from_string("") }
+/// assert_eq(out.stdout.to_string().unwrap(), "hi\n")
+/// assert_eq(out.stderr.len(), 0)
+/// ```
+pub record ProcessOutput {
+    /// The exit status: the program's exit code, or `128 + n` on Unix when
+    /// signal `n` ended it.
+    pub status: Int,
+    /// Everything the program wrote to its standard output.
+    pub stdout: Bytes,
+    /// Everything the program wrote to its standard error.
+    pub stderr: Bytes,
+}
+
+// The `IoError` for a failed `process_run` or `process_current_exe`, whose
+// status code is `code`.
+fn process_error(code: Int) -> IoError {
+    IoError { kind: io_error_kind_of(code), message: fs_last_error_message() }
+}
+
+/// Runs `program` with the arguments `args`, waits for it to end, and
+/// returns its exit status and what it printed.
+///
+/// `program` is a path, or a name looked for on `PATH`, as a shell would,
+/// with `.exe` added on Windows. Each argument reaches the program as it is,
+/// spaces and quotes included: no shell reads them. The program's standard
+/// input is empty, its standard output and standard error are captured,
+/// and it inherits this program's environment and working directory. Its
+/// exit status is its exit code, or `128 + n` on Unix when signal `n` ended
+/// it.
+///
+/// Returns an `IoError` of kind `NotFound` if there is no such program, and
+/// `PermissionDenied` if it cannot be run. A program that runs and fails
+/// is not an error: its status says so.
+///
+/// It blocks the thread until the program ends, every task included.
+///
+/// ```
+/// let missing = run("nova-no-such-program", ["--version"])
+/// let kind = match missing {
+///     Ok(_) => "ran"
+///     Err(e) => match e.kind {
+///         NotFound => "not found"
+///         _ => "another error"
+///     }
+/// }
+/// assert_eq(kind, "not found")
+/// ```
+pub fn run(program: String, args: [String]) -> Result<ProcessOutput, IoError> {
+    let code = process_run(program, args, true)
+    if code != 0 { return Err(process_error(code)) }
+    let exit_status = process_take_status()
+    let out = process_take_stdout()
+    let err = process_take_stderr()
+    Ok(ProcessOutput { status: exit_status, stdout: out, stderr: err })
+}
+
+/// Runs `program` with the arguments `args`, waits for it to end, and
+/// returns its exit status.
+///
+/// As `run`, but the program reads and writes this program's own standard
+/// input, output and error, so what it prints appears with what this
+/// program prints, in order.
+///
+/// Returns an `IoError` of kind `NotFound` if there is no such program, and
+/// `PermissionDenied` if it cannot be run. It blocks the thread until the
+/// program ends, every task included.
+///
+/// ```
+/// match status("nova-no-such-program", []) {
+///     Ok(code) => println("it ended with ${code}")
+///     Err(e) => assert(e.message.len() > 0, "the system says why")
+/// }
+/// ```
+pub fn status(program: String, args: [String]) -> Result<Int, IoError> {
+    let code = process_run(program, args, false)
+    if code != 0 { return Err(process_error(code)) }
+    Ok(process_take_status())
+}
+
+/// Returns the value of the environment variable `name`, or `None`.
+///
+/// `None` when the variable is not set, when its value is not valid UTF-8,
+/// and when `name` is empty or holds `=` or the NUL character, which no
+/// variable's name can. On Windows, names are not case-sensitive.
+///
+/// ```
+/// assert(env("NOVA_SURELY_UNSET_VARIABLE").is_none(), "nothing sets it")
+/// assert(env("").is_none(), "no variable has an empty name")
+/// assert(env("A=B").is_none(), "no variable's name holds `=`")
+/// ```
+pub fn env(name: String) -> Option<String> {
+    if process_env(name) { Some(fs_take_string()) } else { None }
+}
+
+/// Returns the path of the running executable.
+///
+/// Under `nova run`, the program runs inside `nova`, so this is the path of
+/// `nova` itself; a program built with `nova build` gets its own. Returns an
+/// `IoError` when the system cannot say, and one of kind `InvalidData` when
+/// the path is not valid Unicode.
+///
+/// ```
+/// let exe = current_exe().unwrap()
+/// assert(exe.len() > 0, "a path")
+/// ```
+pub fn current_exe() -> Result<String, IoError> {
+    let code = process_current_exe()
+    if code != 0 { return Err(process_error(code)) }
+    Ok(fs_take_string())
+}
````

### Task 8: ADR 0050 and the records

Spec §5.8, §9; decisions 17 to 22, 27, 28 and 40 to 44.

**Files:**
- Create: `docs/adr/0050-the-std-a-compiler-needs.md`
- Modify: `CHANGELOG.md`, `nova-spec/00-MASTER-SPEC.md`,
  `nova-spec/12-TYPESYSTEM.md`, `nova-spec/14-CODEGEN.md`,
  `nova-spec/20-STDLIB.md`

- [ ] **Step 1: The records.** `patch_block t8-code`:
  - **ADR 0050,** the std a compiler needs: its context (what N needs that
    std lacked; what the spec files say otherwise); its decisions, the
    builder over `Vec<String>`, byte access through a view, `to_bits` and
    `str_is_float` as runtime calls and shim entries with the `-nan` fix,
    `TyHead::Array`, inherent impls on arrays accepted and trait impls
    E0010, and E0074's second label, `OrderedMap` and its compaction,
    `std/process`'s functions and wire contract, `env`'s refusals and how
    they are pinned, std's inherent methods before a program's trait methods
    with E0105, and E0105 the one new code; its consequences, among them the
    runaway child and its guard, a program's `impl<T> [T]` that defines
    `iter` or `slice` and its errors inside std, the programs that compiled
    before and can change, and Windows' `.bat`; and its references.
  - **`CHANGELOG.md`,** Unreleased: Added (the builder, the conversions and
    bytes, parsing, `to_bits`, methods on arrays and `Vec`, `OrderedMap`,
    `std/process`) and Changed (`TyHead::Array` and the ten builtins,
    `STD_ONLY` 90 to 100; the shim's `-nan`; std's inherent methods before a
    program's trait methods, E0105, and a program's own inherent method of
    their names E0074, as is an `impl<T> [T]` defining `iter` or `slice`;
    E0074's labels; `[T]::slice` in diagnostics; the public-form note; the
    six process builtins; the walk's environment table).
  - **Dated notes,** each dated 2026-10-29 and citing the spec:
    `00-MASTER-SPEC.md`'s Phase 5 (5.0's three parts); `12-TYPESYSTEM.md`
    §5.2 (`TyHead::Array`, inherent impls only), its catalog's E0105 row and
    §8 (5.0c's diagnostics); `14-CODEGEN.md` §6.1 (the builder); and
    `20-STDLIB.md`'s module index (`std/process`, `OrderedMap`), §2
    (`std/core`'s conversions and `to_bits`), §12 (arrays, `Vec`,
    `OrderedMap`), §17 (`std/process`) and §18 (strings).
- [ ] **Step 2: Read them against the code.** Each record's claim against
  the tree: the ADR's decisions against Tasks 1 to 7 and this plan's
  decisions; the CHANGELOG's entries against the patches; every figure
  against this plan's. Any difference goes in the ledger and to Task 9's
  review, which fixes it in its one commit, so that this task's tree stays
  the replay's.
- [ ] **Step 3: The checks,** as the conventions list them, after the build.
- [ ] **Step 4: Commit**
  `docs: ADR 0050, the std a compiler needs, and the notes for 5.0c`.
- [ ] **Step 5: Run,** on the commit.
  - `git diff --check HEAD~1 HEAD`. Expected: nothing.
  - `git diff --stat 8f1d4153 HEAD -- . ':!docs/superpowers/plans'`.
    Expected: nothing: but for this plan's file, which the prototype does
    not hold, the tree is the prototype's last, `8f1d4153`. A difference
    stops the task: record it in the ledger.

**Patch `t8-code`:**

<!-- patch t8-code -->

````diff
diff --git a/CHANGELOG.md b/CHANGELOG.md
index 3fa3b798ed6838ece6e965cd0c8a2aba2da165f6..89285bac7d10a02aa1b5e55178d7d0657efe9d04 100644
--- a/CHANGELOG.md
+++ b/CHANGELOG.md
@@ -11,8 +11,8 @@ Nova uses [Semantic Versioning](https://semver.org/spec/v2.0.0.html).
 
 Phase 5.0a, the small language and CI (spec
 `docs/superpowers/specs/2026-10-26-phase-5-0-the-language-and-std-a-compiler-needs-design.md`
-§3; ADR 0048, Phase 5's scope and backend), and 5.0b, tuples and
-patterns (§4; ADR 0049).
+§3; ADR 0048, Phase 5's scope and backend), 5.0b, tuples and
+patterns (§4; ADR 0049), and 5.0c, std (§5; ADR 0050).
 
 ### Added
 
@@ -90,6 +90,60 @@ patterns (§4; ADR 0049).
 - **`for x in arr` over an array:** in order, `arr` evaluated once and each
   element read when the loop reaches it, with `break`, `continue` and a
   pattern, on every backend, WASM included.
+- **`StringBuilder`,** in std/strings: `new`, `push`, `push_char`,
+  `byte_len`, `is_empty`, `clear` and `to_string`, and `Display`, so
+  `"${b}"` interpolates it. `push` is constant time amortised and
+  `to_string` joins every part in one copy, so building a string from any
+  number of pushes is linear, where `+` or interpolation in a loop is
+  quadratic. `to_string` leaves the builder as it was. A record over a
+  `Vec<String>`, on every backend, WASM included.
+- **`Char` and `Int`, and a `String`'s bytes:** `'a'.to_int()`;
+  `char_from_int(n)`, `None` below 0, above `0x10FFFF` and for the
+  surrogates; `string_from_chars(cs)`; and `byte_len`, `byte_at(i)` and
+  `byte_slice(a, b)` on `String`, in constant time, counting bytes.
+  `byte_at` panics past the end, and `byte_slice` when a bound is out of
+  range, `a > b`, or a bound falls inside a character, so neither
+  allocates an `Option`.
+- **`parse_int`, `parse_int_radix` and `parse_float`** on `String`, each
+  an `Option`: an optional sign and digits, nothing else, and `None` out
+  of `Int`'s range, its least value included; radix 2 to 36, letters of
+  either case, no prefix, and a panic on any other radix; and the text
+  Rust's `f64::from_str` takes, with its value, `inf` and `nan` included,
+  the same on WASM.
+- **`Float.to_bits`,** the IEEE 754 bits as a signed `Int`, as Rust's
+  `f64::to_bits`: a runtime call natively and a shim entry on WASM, each
+  giving the bits of the value it is handed. A NaN made by arithmetic can
+  have other bits on another backend: `0.0 / 0.0` is `0xFFF8…` under
+  `nova run` and `0x7FF8…` built `--release`, where LLVM folds it. There
+  is no `from_bits`.
+- **Methods on arrays and between arrays and `Vec`:** `iter()`, which
+  yields each element in order, and `slice(a, b)`, a copy, on every
+  `[T]`; `Vec::to_array` and `Vec::from_array`, which copy; and `Vec::at(i)`
+  and `Vec::last()`, which read without an `Option` and panic where `get`
+  and `pop` give `None`. A program may write an inherent impl on arrays,
+  `impl<T> [T]` or `impl [Int]`, as on `String`, where any impl on an
+  array was E0010 before; a trait impl on an array stays E0010.
+- **`OrderedMap`,** in std/collections: a map that keeps its keys in the
+  order they were first inserted, with `new`, `len`, `is_empty`, `insert`,
+  `get`, `contains_key`, `remove`, `keys`, `values`, `entries`, which gives
+  `(key, value)` tuples, and `iter`, which yields them. Updating a key
+  keeps its place; removing it and inserting it again puts it last. The
+  order is the same on every run, where a `Map`'s is unspecified, and
+  `Map` does not change. Lookups go through a `Map`, so its keys need
+  `Hash` and `Eq`; the gaps removals leave are compacted as the map grows.
+  On every backend, WASM included (ADR 0050).
+- **`run`, `status`, `env` and `current_exe`,** in std/process, with the
+  record `ProcessOutput`: `run(program, args)` runs another program, a
+  path or a name looked for on `PATH`, with its arguments as they are and
+  no shell, waits for it, and returns its exit status and its captured
+  standard output and error; `status` runs one that reads and writes this
+  program's streams and returns its status. A death by signal on Unix is
+  `128 + signal`; a missing program is `NotFound`. Both block the thread,
+  every task included. `env(name)` reads an environment variable, and
+  `current_exe()` gives the running executable's path, `nova` itself
+  under `nova run`. Native only: on WASM, naming one is E0092. The four
+  names are in every module's scope, and a program's own shadows them
+  (ADR 0050).
 
 ### Changed
 
@@ -153,6 +207,58 @@ patterns (§4; ADR 0049).
   `release_golden_suite`, builds every golden program of `tests/runtime/`
   but the nine its exclusion table names with `--release`, through
   `clang`, runs it and compares its output; elsewhere it skips (ADR 0048).
+- **An array has a type head,** `hir::TyHead::Array`, so impls on `[T]` are
+  looked up as other types' are, after an array's built-in `len`; an
+  unknown method on an array is E0014 as before, its fix now offering the
+  impls' methods with `len`. Ten std-only builtins are new, `STD_ONLY`
+  growing from 90 to 100: `str_as_bytes` and `int_to_char`, register
+  moves, `str_is_float` and `float_to_bits`, runtime calls natively and
+  shim entries on WASM, and the six process builtins below.
+- **On WASM, the shim's `str_to_float` gives `-nan` a NaN with its sign
+  bit set,** as Rust's parser does natively; before, it dropped the sign,
+  which no program could see until `to_bits`.
+- **std's new inherent methods take a call before a program's trait
+  method of the same name, and a program's own inherent method of that
+  name is E0074.** `String`'s `byte_len`, `byte_at`, `byte_slice`,
+  `parse_int`, `parse_int_radix` and `parse_float`, `Char`'s `to_int`,
+  `Float`'s `to_bits`, `Vec`'s `at`, `last` and `to_array`, and an array's
+  `iter` and `slice` are std's inherent methods now, and `Vec::from_array`
+  its inherent function. A program whose `impl Size for String { fn
+  byte_len … }` made `"é".byte_len()` run its trait's method runs std's
+  now, as in Rust, and the new warning E0105 marks the hidden method, with
+  how to reach it: through a function generic over the trait, whose
+  `x.byte_len()` is the trait's, or by renaming it. A static trait
+  function `from_array` on `Vec<Int>` is warned of too: the path
+  `Vec::from_array(…)`, E0011 before, since `Vec` alone could not pick the
+  impl, is std's now, and the trait's is reached through a function
+  generic over the trait, as before; a trait method and a std function
+  of which only one takes `self` never meet, and draw nothing. E0105 marks
+  a hide by any of std's inherent methods a program can call, older ones
+  such as `Vec`'s `iter` or `Map`'s `len` too, private ones included,
+  since a method's privacy is not enforced at a call. A program's own
+  `impl String { fn byte_len … }`, which compiled before, is E0074, as is
+  an `impl<T> [T]` that defines `iter` or `slice`. E0074 between two
+  inherent impls labels both now, the program's impl as the primary label
+  when the other is std's.
+- **A method's name in a diagnostic** is the source's type where the
+  resolver's mangled one showed: `[T]::slice` and `[Int]::bump` for an
+  array's (E0016, E0060), not `Arr_T.slice`, and `Vec::at` for a generic
+  type's, not `Vec_T.at`. A plain type's stays `P.of`.
+- **An array in an interpolation** is E0013 as before, without the advice
+  to implement `Display`, which a trait impl on an array cannot do.
+- **Calling `char_to_int`, `int_to_char`, `str_from_chars`,
+  `str_as_bytes`, `str_is_float`, `str_to_float` or `float_to_bits` by
+  name** is E0001 as before, std's alone having them, now with a note
+  naming the public form, such as `c.to_int()`, where "did you mean"
+  offered `char_from_int` for `char_to_int`.
+- **Six std-only builtins run programs and read the environment,** on
+  `std/fs`'s wire contract: `process_run`, a status code, then
+  `process_take_status`, `process_take_stdout` and `process_take_stderr`,
+  and `process_env` and `process_current_exe`, whose text and errors go
+  through `fs_take_string` and `fs_last_error_message`.
+- **`release_golden_suite` sets environment variables on a program** its
+  `RELEASE_GOLDEN_ENV` table names, as it gives standard input:
+  `process_env` reads one.
 
 ### Fixed
 
diff --git a/docs/adr/0050-the-std-a-compiler-needs.md b/docs/adr/0050-the-std-a-compiler-needs.md
new file mode 100644
index 0000000000000000000000000000000000000000..10717384dcfb72a129cfaebc70c807f6321e64e2
--- /dev/null
+++ b/docs/adr/0050-the-std-a-compiler-needs.md
@@ -0,0 +1,214 @@
+# ADR 0050 — The std a compiler needs
+
+## Status
+
+Accepted (2026-10-29). Branch `claude/loving-archimedes-frtf5q-50c`
+(`docs/superpowers/specs/2026-10-26-phase-5-0-the-language-and-std-a-compiler-needs-design.md`
+§5 and §5.8). The builder over `Vec<String>`, `OrderedMap` beside `Map`,
+`Float.to_bits` with a shim, and `run` and `status` as plain blocking
+functions, native only, are the user's decisions 6 and 8 of 2026-10-26;
+the rest is what spec §13 left to the plan.
+
+## Context
+
+Phase 5's compiler, N, is written in Nova, and before 5.0c std lacked what
+a compiler reads and writes with. A lexer read a string a `Char` at a time
+through `chars()`, which builds an array, and had no way from a `Char` to
+its code or back, nor any parsing of numbers. Text was built with `+`, which
+is quadratic in a loop. A `Map`'s order is seeded per process, so a symbol
+table walked in order could not give the same output twice, and N's
+bootstrap compares its stages' output byte for byte. N's driver runs
+`clang` and reads `NOVA_CLANG` and the path of its own executable, and
+`std/process` had only `args` and `exit`.
+
+The spec files say otherwise in places: `14-CODEGEN.md` §6.1 builds strings
+with `String::with_capacity`, `20-STDLIB.md`'s index promises `spawn` in
+`std/process`, and `std/fs`, the model for I/O, is `async`.
+
+## Decision
+
+1. **`StringBuilder` is a std record over `Vec<String>`** (decision 8),
+   not `14-CODEGEN.md` §6.1's `String::with_capacity` nor a runtime byte
+   buffer. `push` is a `Vec` push; `to_string` copies the parts into an
+   array of their count and joins them in one copy through `str_join`, so
+   building is linear. There is no counted `str_join`: the extra pointer
+   copy costs little, and the `Vec`'s storage is private to
+   std/collections. It counts bytes, `byte_len`, and has no `len`.
+2. **Byte access goes through a view that copies nothing.** The std-only
+   builtin `str_as_bytes` reads a `String` as `Bytes`, a register move,
+   since the two share one layout and neither changes; `byte_len`,
+   `byte_at` and `byte_slice` are then `Bytes`' builtins, in constant time.
+   `byte_at` and `byte_slice` panic rather than return an `Option`, so a
+   lexer allocates nothing per byte, and `byte_slice` panics too on a bound
+   inside a character. `int_to_char` is the same move the other way, after
+   `char_from_int` checks the range in Nova.
+3. **`Float.to_bits` is a runtime call,** `float_to_bits`, natively, and a
+   shim entry on WASM, through a `DataView`; there is no MIR bit cast, so
+   no backend has a new statement, and there is no `from_bits`.
+   `parse_float` pairs `str_to_float` with a new `str_is_float`, so a
+   refused text is told from a `0.0`, by Rust's grammar on both targets.
+   The shim's `str_to_float` now gives `-nan` its sign bit, as Rust's
+   parser does natively, since `to_bits` makes the sign visible.
+4. **Arrays have a type head,** `TyHead::Array`, so std writes
+   `impl<T> [T]` (`iter`, `slice`). Before 5.0c every impl on an array was
+   E0010, "impl blocks are only supported on named types"; now an inherent
+   one, `impl<T> [T] { … }` or `impl [Int] { … }`, checks, as on `String`,
+   and one that defines `iter` or `slice` again is E0074. A trait impl on
+   an array stays E0010, "a trait cannot be implemented for an array" (the
+   user's ruling on the first fact-check): an array takes inherent impls
+   alone. The resolver's name for an array's method, `Arr_T.slice`, is
+   shown in diagnostics as `[T]::slice` (E0016, E0060), and a generic
+   type's, `Vec_T.at`, as `Vec::at`; an array in an interpolation is E0013
+   without the advice to implement `Display`.
+   E0074 between two inherent impls now labels both, the earlier one
+   "`iter` is also defined by this impl": the error is reported at the
+   later impl, which for a program's impl against std's is std's, so
+   before this label the program's own impl appeared nowhere in it.
+   `Vec::to_array` and `from_array` copy. `Vec` gains `at` and `last`,
+   which panic where `get` and `pop` give `None`, since `get` allocates a
+   `Some` per read.
+5. **`OrderedMap` is a new type beside `Map`** (decision 8), which does not
+   change: three `Vec`s, keys, values and a live flag, one slot per key in
+   insertion order, and a `Map<K, Int>` from key to slot, read for lookups
+   only, so the order never depends on a seed. `remove` leaves a gap.
+   **Only `insert` compacts, before it appends a new key, and only when
+   the gaps are at least half the slots.** Half keeps the slots within
+   twice the entries plus one after any insertion, the slack `Vec`'s
+   doubling allows, so a walk is linear in the entries; each compaction
+   walks at most twice the gaps it removes, so `insert` and `remove` stay
+   amortized constant time; and since a removal never moves an entry,
+   removing while an iterator runs skips nothing else. Same `Hash` and
+   `Eq` bounds as `Map`, and on both targets.
+6. **`std/process` has `run` and `status` in place of `spawn`, with `env`
+   and `current_exe`** (decision 6): plain `fn`s that block the thread,
+   executor included, though `std/fs` is `async`, since a driver runs a
+   tool and waits; native only, so on WASM, where the module is not
+   compiled, naming one is E0092. The program is found as Rust's `Command`
+   finds it, `PATH` and `.exe` included, its arguments passed with no
+   shell, the environment and working directory inherited; `run`'s
+   standard input is empty and both outputs are captured whole, read at
+   once so neither pipe stalls the child; `status` inherits all three
+   streams, after flushing standard output. A death by signal on Unix is
+   `128 + signal`. The record is `ProcessOutput`, not `Output`.
+7. **The process builtins are on `std/fs`'s wire contract.** Six std-only
+   builtins: `process_run(program, args, capture) -> Int`, a status code,
+   `0` or an `IoErrorKind`'s, then `process_take_status`,
+   `process_take_stdout` and `process_take_stderr`; `process_env(name) ->
+   Bool` and `process_current_exe() -> Int`, whose text waits for
+   `fs_take_string`; every failure's message waits for
+   `fs_last_error_message`, through `fs.rs`'s own `fail`, so a missing
+   program is `NotFound` and one that cannot be run `PermissionDenied` by
+   the same table. One builtin with a `capture` flag serves both `run` and
+   `status`, and the takes reuse `std/fs`'s where a payload is text, so
+   there are six names, not ten. The child's output waits in the thread's
+   cells as Rust bytes, not GC objects, so nothing is rooted while it
+   waits, and the `Bytes` is allocated when taken; a thread's cells, not a
+   task's slots, because the wrappers are plain `fn`s with nothing between
+   the run and its takes.
+8. **`env(name)` refuses an empty name, or one that holds `=` or NUL,**
+   without asking the system, which may panic on one. A test sets a
+   variable on a built program through its own process: `Command::env` in
+   `run_tests.rs`, and the `--release` walk's `RELEASE_GOLDEN_ENV` table,
+   as its `RELEASE_GOLDEN_STDIN` gives standard input. The `=` refusal is
+   pinned on Linux by a variable whose value holds `=`: with
+   `NOVA_PROCESS_TEST_EQ=x=y` set, glibc's `getenv` answers `y` for the
+   name `NOVA_PROCESS_TEST_EQ=x`, and macOS's the whole value, so the
+   program's `None` is `env`'s own; the empty and NUL refusals change no
+   result on Linux, where glibc and Rust refuse those names themselves,
+   and stay as the spec asks. Under `nova run` the
+   program sees `nova`'s environment, and `current_exe()` is `nova`.
+   `current_exe()` of a path that is not Unicode is `InvalidData`, since a
+   path with its bad bytes replaced names another file.
+9. **std's inherent method wins over a program's trait method of the same
+   name, as in Rust, and E0105 warns** (the user's ruling on the first
+   fact-check). 5.0c adds inherent methods to `String` (`byte_len`,
+   `byte_at`, `byte_slice`, `parse_int`, `parse_int_radix`,
+   `parse_float`), `Char` (`to_int`), `Float` (`to_bits`), `Vec` (`at`,
+   `last`, `to_array`, and the function `from_array`) and arrays (`iter`,
+   `slice`): fourteen. A method call takes an inherent method before a
+   trait's, so a program's `impl Size for String { fn byte_len … }`, whose
+   `"é".byte_len()` ran the trait's method before 5.0c, now runs std's;
+   and a path takes an inherent function before a trait's, so beside a
+   program's `impl Mk for Vec<Int> { fn from_array(a: [Int]) -> Vec<Int> …
+   }`, `Vec::from_array(a)`, E0011 at the base since `Vec` alone could not
+   pick the impl, is std's now. E0105, a
+   warning, is reported at each trait impl's method, or at the impl for a
+   default it takes, that one of std's inherent methods on an overlapping
+   type hides: "std's inherent method `byte_len` on `String` hides
+   `Size::byte_len`", with the note that Nova has no `Size::byte_len(x)`
+   call, so the trait's method is reached through a function generic over
+   `T: Size`, whose `x.byte_len()` is the trait's, or by renaming it; for a
+   static function, "std's inherent function `from_array` …", the generic
+   function's `T::from_array(…)`. Where only one of the two takes `self`
+   they never meet, and E0105 is quiet (the second fact-check). It counts
+   every std method a program can call, older ones such as `Vec::iter` or
+   `Map::len` and private ones such as `OrderedMap`'s `compact`, since a
+   method's privacy is not enforced at a call. A program's own inherent
+   method of one of these names, `impl String { fn byte_len … }`, compiled
+   before 5.0c and is E0074 now, labelled first at the program's impl.
+10. **One new diagnostic code, E0105.** E0092 covers `std/process`'s new
+   names on WASM; E0010, E0013, E0014, E0060 and E0074 cover the rest. A
+   program that
+   calls one of the std-only builtins std's surface now wraps by its own
+   name, `char_to_int`, `int_to_char`, `str_from_chars`, `str_as_bytes`,
+   `str_is_float`, `str_to_float` or `float_to_bits`, gets E0001 with a
+   note naming the public form, `c.to_int()` and the like, in place of
+   "did you mean", whose nearest name for `char_to_int` was
+   `char_from_int`, the conversion the other way.
+
+## Consequences
+
+- **`run` and `status` stop every task** while the child runs. An
+  asynchronous process, a `Command` builder with a working directory, an
+  environment or standard input, and processes on WASM are out of 5.0
+  (spec §11).
+- **`run`, `status`, `env` and `current_exe` join every module's scope,**
+  and a program's own shadows them silently, as `args` is shadowed; seven
+  programs of the repository define a `run` of their own and are
+  unchanged.
+- **A process test builds its program first,** since under `nova run`
+  `current_exe()` is `nova`; the programs that run themselves are tested
+  built and through the `--release` walk, not under `nova run`. On Windows
+  a running executable is locked, so a test removes its build after its
+  children end.
+- **A program that runs itself must guard against a child that lost its
+  arguments,** which would run the parent's part again without end: the
+  prototype's arguments-dropped mutant was a fork bomb. `process_run`
+  writes a marker in its temporary directory and stops if one is there;
+  its tests give it a fresh directory, then check that a run that finds
+  the marker starts nothing.
+- **An `OrderedMap` that only shrinks keeps its slots,** and with them its
+  dead keys and values, until it next grows; a key inserted while an
+  iterator runs can move the entries under it. `OrderedSet` is not built.
+- **A program's inherent `impl<T> [T]` is new and checks, but one that
+  defines `iter` or `slice` is E0074,** std's impl defining both. As with
+  `impl String`, std's own calls of the method then resolve to the
+  program's, so a program whose `slice` takes other arguments also sees
+  errors inside std, after the E0074; the E0074 comes first and labels the
+  program's impl.
+- **Programs that compiled before 5.0c can change.** A program's own
+  inherent method named as one of std's new ones (`impl String { fn
+  byte_len … }`, `impl Char { fn to_int … }`, `impl Float { fn to_bits
+  … }`, `impl<T> Vec<T> { fn last … }`) is E0074; a program's trait method
+  of such a name is hidden by std's at every method call on that type, and
+  E0105 says where; a static trait function `from_array` on `Vec<Int>`,
+  whose path was E0011 at the base, meets std's at the path, and E0105
+  says so too. No program in the repository meets the E0074;
+  `tests/runtime/hidden_trait_methods` hides three trait methods on
+  purpose, to show E0105's advice at work on every backend.
+- **On Windows, a `.bat` or `.cmd` program runs through `cmd.exe`,** whose
+  quoting is not the C runtime's; Rust's `Command` refuses arguments it
+  cannot pass to one safely, and `run` then gives an `IoError` of kind
+  `Other`. Every other program gets its arguments as they are.
+- **`STD_ONLY` grows from 90 to 100** over the part.
+
+## References
+
+- `docs/superpowers/specs/2026-10-26-phase-5-0-the-language-and-std-a-compiler-needs-design.md`
+  §5, §6, §7, §10 risks 7 and 9, §11, §12 items 18 to 24, and §13.
+- `nova-spec/20-STDLIB.md` §1, §2, §12, §17 and §18, `14-CODEGEN.md` §6.1
+  and `12-TYPESYSTEM.md` §5.2 and §8, and their notes of 2026-10-29.
+- `std/strings/lib.nova`, `std/core/lib.nova`, `std/collections/lib.nova`,
+  `std/process/lib.nova`; `crates/nova-runtime/src/process.rs`.
+- ADR 0011, I/O error kinds; ADR 0023, program arguments; ADR 0048,
+  Phase 5's scope.
diff --git a/nova-spec/00-MASTER-SPEC.md b/nova-spec/00-MASTER-SPEC.md
index ff068c299b05b376c3e20620003f4b5b95b729ed..21164213079096c22026af442813af6d080f4568 100644
--- a/nova-spec/00-MASTER-SPEC.md
+++ b/nova-spec/00-MASTER-SPEC.md
@@ -752,6 +752,17 @@ ADR. Tagging `v0.4.0` completes Phase 4 within that boundary.
 
 **Gate:** CI builds stage2 == stage1 byte-for-byte (or close — may differ in non-deterministic codegen, document tolerances).
 
+**Amended 2026-10-29 (branch `claude/loving-archimedes-frtf5q-50c`):** the
+language and std the port needs land first, as Phase 5.0, in three parts
+(`docs/superpowers/specs/2026-10-26-phase-5-0-the-language-and-std-a-compiler-needs-design.md`;
+ADR 0048): 5.0a, `?`, `+=` on places, the newline rule, `String + String`
+and the golden suite through LLVM in CI; 5.0b, tuples and patterns (ADR
+0049); 5.0c, std (ADR 0050): `StringBuilder`, conversions between `Char`,
+`Int` and a `String`'s bytes, number parsing, `Float.to_bits`, methods on
+arrays, `OrderedMap`, whose order makes a stage's output the same on every
+run, and `std/process`'s `run`, `status`, `env` and `current_exe`, for the
+driver that runs `clang`.
+
 ---
 
 ### Phase 6 — 1.0 Release (week 105–120)
diff --git a/nova-spec/12-TYPESYSTEM.md b/nova-spec/12-TYPESYSTEM.md
index e6fcd9e02865ed5ec309e3a955c4b927982a1a3e..619b80f44820825488b62e059562e602967b9503 100644
--- a/nova-spec/12-TYPESYSTEM.md
+++ b/nova-spec/12-TYPESYSTEM.md
@@ -149,6 +149,16 @@ Stored as `TraitDef { id, methods: Vec<MethodSig>, supers: Vec<TraitRef> }`.
   4. If multiple match → ambiguity error
   5. If none match → "no impl of `Display` for `Ty(x)`" error
 
+**Amended 2026-10-29 (branch `claude/loving-archimedes-frtf5q-50c`):** an
+array's head is `TyHead::Array`, whatever its element type, so an inherent
+impl may name `[T]` (`impl<T> [T]`, or `impl [Int]`), where every impl on
+an array was E0010 before, and `match_args` tells the two apart by the
+element type; a trait impl on an array stays E0010, "a trait cannot be
+implemented for an array" (the user's ruling on 5.0c's first fact-check)
+(`docs/superpowers/specs/2026-10-26-phase-5-0-the-language-and-std-a-compiler-needs-design.md`
+§5.5). An array's built-in `len` is looked up first, then its impls'
+methods; a tuple, a function type and a future still have no head.
+
 ### 5.3 Coherence (Orphan Rules)
 - An `impl Trait for Type` is allowed only if either `Trait` or `Type` is defined in current package
 - Prevents conflicting impls across packages
@@ -269,6 +279,7 @@ types differ.
 | E0102 | A variant hides one of std's; a warning (added 2026-10-26, Phase 5.0a) |
 | E0103 | This value is not used, or a line break ended a statement; a warning (added 2026-10-26, Phase 5.0a) |
 | E0104 | A pattern binds a name that starts with an upper-case letter, not a plain `let X` or `for X`; a warning (added 2026-10-28, Phase 5.0b) |
+| E0105 | One of std's inherent methods hides a program's trait method of the same name; a warning (added 2026-10-29, Phase 5.0c) |
 
 Each gets a fully detailed error in `nova-diagnostics::messages` with example, suggestion, and link.
 
@@ -372,6 +383,37 @@ so `match (1, 9) { (a, _) | (_, a) if a > 5 => "big", _ => "small" }` gives
 `small` in Nova and `big` in Rust. E0021 marks the alternative a guard
 cannot reach, `(_, a)` here, with that note.
 
+**Amended 2026-10-29 (branch `claude/loving-archimedes-frtf5q-50c`):** 5.0c,
+std
+(`docs/superpowers/specs/2026-10-26-phase-5-0-the-language-and-std-a-compiler-needs-design.md`
+§5, §6; `docs/adr/0050-the-std-a-compiler-needs.md`), takes one new code.
+**E0105,** a warning, is a program's trait method, defined in its impl or
+taken from the trait's default, that one of std's inherent methods on an
+overlapping type also defines: a method call takes the inherent method,
+as in Rust, so the trait's is never reached by `x.m()`, and a path takes
+an inherent function, so a static trait function is never reached by
+`Vec::from_array(…)`; the note says to call it through a function generic
+over the trait, or to rename it. A pair of which only one takes `self`
+never meets, and is not reported; any std method a program can call
+counts, older or private.
+**E0092** is `std/process`'s `run`, `status`, `env`, `current_exe` and
+`ProcessOutput` on WASM, where the module is not compiled, as for `args`.
+**E0074** is a program's inherent impl that defines one of std's new
+inherent methods again: `iter` or `slice` on arrays, on which no impl at
+all was accepted before 5.0c, and `byte_len`, `to_int`, `to_bits`, `last`
+and the rest on `String`, `Char`, `Float` and `Vec`, which compiled
+before. Between two inherent impls it now labels both, the program's
+impl first where the other is std's. **E0010** is a trait impl on an
+array, and **E0013** an interpolated array, without the advice to
+implement `Display`. **E0001** at a call of a std-only
+builtin that std now wraps, `char_to_int` and the like, notes the public
+form, `c.to_int()`, in place of "did you mean". **E0014** is an unknown
+method on an array, its fix offering std's array methods with `len`.
+**E0013** is an `OrderedMap` method instantiated with a key type that is
+not `Hash` and `Eq`, as for `Map`, and **E0060** an `insert` or `remove`
+on an `OrderedMap`, or a `push` on a `StringBuilder`, through a binding
+that is not `mut`.
+
 ---
 
 ## 9. Examples — Test Cases
diff --git a/nova-spec/14-CODEGEN.md b/nova-spec/14-CODEGEN.md
index a0abc028f2ffdbc3dd5a1c6fac90283ce135dbf3..ad7131d34fa2fe0002483afc3bdd050fbd136548 100644
--- a/nova-spec/14-CODEGEN.md
+++ b/nova-spec/14-CODEGEN.md
@@ -151,6 +151,15 @@ two or more parts was a chain of pairwise `nova_rt_str_concat` calls, each
 copying everything built so far. See `examples/05-json-api/BENCHMARK.md`, "AMENDMENT
 2026-10-01 (nary-interpolation)".
 
+**Amended 2026-10-29 (branch `claude/loving-archimedes-frtf5q-50c`):** the
+builder a program writes by hand is not `String::with_capacity` and
+`push_str` either: it is std/strings' `StringBuilder`, a record over a
+`Vec<String>` of the parts and their length in bytes
+(`docs/superpowers/specs/2026-10-26-phase-5-0-the-language-and-std-a-compiler-needs-design.md`
+§5.1). `push` is a `Vec` push; `to_string` copies the parts' pointers into
+an array of their count, then joins them with one `nova_rt_str_join`, one
+copy of the bytes. No backend and no runtime symbol changed for it.
+
 ### 6.2 Pattern Match
 Compiled to decision tree (Maranget). Each leaf is a basic block. Compiler emits:
 ```llvm
diff --git a/nova-spec/20-STDLIB.md b/nova-spec/20-STDLIB.md
index f7ba5a6246c5900c2db334265716d8c15ef9e3c3..696c564ae42f3530084addc433b62269b4e54bf1 100644
--- a/nova-spec/20-STDLIB.md
+++ b/nova-spec/20-STDLIB.md
@@ -58,6 +58,14 @@ Its functions are listed in
 `docs/superpowers/specs/2026-10-16-phase-4-2-std-dom-and-the-shim-design.md`
 §6 (ADR 0041).
 
+**Amended 2026-10-29 (branch `claude/loving-archimedes-frtf5q-50c`):** of
+`std/process`'s "spawn, env, args", `env` ships, and `run` and `status`
+stand in for `spawn`, beside `current_exe` (`docs/superpowers/specs/2026-10-26-phase-5-0-the-language-and-std-a-compiler-needs-design.md` §5.7, §5.8;
+`docs/adr/0050-the-std-a-compiler-needs.md`). They are native only, as
+the rest of the module is: on WASM, naming one is E0092. `std/collections`
+gains `OrderedMap` (§5.6) beside `Map`, on both targets; `Queue` is still
+not built. §17 and §12 have the details.
+
 **AMENDED 2026-10-17 (branch `claude/loving-archimedes-frtf5q-43`): `std/ui`
 and `std/ui/html`**, on both targets, are imported too: `import ui` for
 reactivity and views, `import ui::html` for the element builders, std's
@@ -188,6 +196,30 @@ pub record FilterIter<I: Iterator> { it: I, keep: fn(I::Item) -> Bool }
 // Int, Float, Bool, Char, String all impl: Display, Debug, Eq, Clone, Hash
 ```
 
+**Amended 2026-10-29 (branch `claude/loving-archimedes-frtf5q-50c`):**
+`std/core` converts between `Char` and `Int`, and gives a `Float`'s bits
+(`docs/superpowers/specs/2026-10-26-phase-5-0-the-language-and-std-a-compiler-needs-design.md`
+§5.2, §5.4):
+
+```nova
+impl Char { pub fn to_int(self) -> Int }
+pub fn char_from_int(n: Int) -> Option<Char>
+impl Float { pub fn to_bits(self) -> Int }
+```
+
+`to_int` is the scalar value, a register move. `char_from_int` is `None`
+below 0, above `0x10FFFF` and for the surrogates `0xD800` to `0xDFFF`; it
+checks the range in Nova, then the std-only builtin `int_to_char` makes the
+`Char`, a register move with no runtime symbol. `to_bits` is Rust's
+`f64::to_bits` read as a signed `Int`, through the std-only builtin
+`float_to_bits`: a runtime call natively and a shim entry on WASM, each
+giving the bits of the value it is handed, though a NaN made by arithmetic
+can differ by backend (`0.0 / 0.0` is `0xFFF8…` under `nova run` and
+`0x7FF8…` built `--release`, where LLVM folds it); there is no `from_bits`.
+The shim's
+`str_to_float` keeps a `-nan`'s sign bit, as Rust's parser does, so
+`"-nan".parse_float()` has the same bits on WASM.
+
 ---
 
 ## 3. `std/fmt`
@@ -2240,6 +2272,78 @@ no implementation at all — grepped, not assumed, and corroborated
 independently by `std/sync/lib.nova`'s ring buffer, whose comment explains
 that it built its own because this module "has no `Queue` to borrow".
 
+**Amended 2026-10-29 (branch `claude/loving-archimedes-frtf5q-50c`):** arrays
+have methods, and `Vec` converts to and from them
+(`docs/superpowers/specs/2026-10-26-phase-5-0-the-language-and-std-a-compiler-needs-design.md`
+§5.5):
+
+```nova
+impl<T> [T] {
+    pub fn iter(self) -> ArrayIter<T>
+    pub fn slice(self, start: Int, end: Int) -> [T]
+}
+pub record ArrayIter<T> { /* private */ }   // impl<T> Iterator for ArrayIter<T>
+
+impl<T> Vec<T> {
+    pub fn to_array(self) -> [T]
+    pub fn from_array(a: [T]) -> Vec<T>
+    pub fn at(self, i: Int) -> T
+    pub fn last(self) -> T
+}
+```
+
+An array's type head is `TyHead::Array`, so std writes `impl<T> [T]`, and
+an array's methods are looked up after its built-in `len`; a program may
+write an inherent impl on arrays as on `String`, and one that defines
+`iter` or `slice` again is E0074, while a trait impl on an array stays
+E0010. `iter` shares the array; `slice` copies
+its range, and panics when a bound is out of range or `start > end`.
+`to_array` and `from_array` copy, so neither shares storage. `at` and
+`last` read without an `Option`, panicking where `get` and `pop` give
+`None`, since `get` allocates a `Some` per read.
+
+**Amended 2026-10-29 (branch `claude/loving-archimedes-frtf5q-50c`):**
+`OrderedMap`, a map that keeps its keys in the order they were first
+inserted, beside `Map`, which does not change (`docs/superpowers/specs/2026-10-26-phase-5-0-the-language-and-std-a-compiler-needs-design.md` §5.6;
+`docs/adr/0050-the-std-a-compiler-needs.md`):
+
+```nova
+pub record OrderedMap<K, V> { /* private */ }
+
+impl<K: Hash + Eq, V> OrderedMap<K, V> {
+    pub fn new() -> OrderedMap<K, V>
+    pub fn len(self) -> Int
+    pub fn is_empty(self) -> Bool
+    pub fn insert(mut self, k: K, v: V) -> Option<V>
+    pub fn get(self, k: K) -> Option<V>
+    pub fn contains_key(self, k: K) -> Bool
+    pub fn remove(mut self, k: K) -> Option<V>
+    pub fn keys(self) -> [K]
+    pub fn values(self) -> [V]
+    pub fn entries(self) -> [(K, V)]
+    pub fn iter(self) -> OrderedMapIter<K, V>   // yields (K, V)
+}
+pub record OrderedMapIter<K, V> { /* private */ }
+```
+
+- **Order:** updating a key keeps its place; removing it and inserting it
+  again puts it last. `keys`, `values` and `entries` are copies in that
+  order, and `iter` walks it, the same on every run, where `Map`'s order
+  is unspecified and seeded per process. `Map`'s docs point here.
+- **Inside:** three `Vec`s, keys, values and a live flag, one slot per key
+  in insertion order, and a `Map<K, Int>` from key to slot, used for
+  lookups only. `remove` marks its slot dead, leaving a gap.
+- **Compaction:** only `insert`, before it appends a new key's slot, and
+  only when the gaps are at least half the slots, rebuilds the three
+  `Vec`s from the live slots and moves each position in the `Map`. So the
+  slots stay within twice the entries plus one after any insertion, `Vec`'s
+  own slack, `insert` and `remove` are amortized constant time, and a
+  removal never moves an entry, so removing while an iterator runs skips
+  nothing else. A map that only shrinks keeps its slots until it grows.
+- **WASM:** Nova over `Vec` and `Map`, so std/collections has it on both
+  targets. A key type without `Hash` and `Eq` is E0013 when a method is
+  instantiated, as for `Map`.
+
 ---
 
 ## 13. `std/sync` & `std/task`
@@ -2788,6 +2892,59 @@ pub fn exit(code: Int)
   `process_exit`, so `STD_ONLY` grows from 78 to 81.
 - See `docs/adr/0023-program-arguments.md`.
 
+**Amended 2026-10-29 (branch `claude/loving-archimedes-frtf5q-50c`):**
+`std/process` runs other programs and reads the environment (`docs/superpowers/specs/2026-10-26-phase-5-0-the-language-and-std-a-compiler-needs-design.md`
+§5.7; `docs/adr/0050-the-std-a-compiler-needs.md`). In place of the index's
+`spawn`, `run` and `status` are plain functions that block:
+
+```nova
+pub record ProcessOutput {
+    pub status: Int,
+    pub stdout: Bytes,
+    pub stderr: Bytes,
+}
+
+pub fn run(program: String, args: [String]) -> Result<ProcessOutput, IoError>
+pub fn status(program: String, args: [String]) -> Result<Int, IoError>
+pub fn env(name: String) -> Option<String>
+pub fn current_exe() -> Result<String, IoError>
+```
+
+- **`run`** starts `program` with `args`, its standard input empty, waits
+  for it, and captures its standard output and error whole, reading both
+  at once so neither pipe can stall the child. **`status`** does the same
+  with the three streams inherited, after flushing the program's standard
+  output, so what it printed before comes first. Both block the thread,
+  executor included, though `std/fs` is `async`: a compiler's driver runs
+  `clang` and waits.
+- **`program`** is a path, or a name searched for on `PATH`, as Rust's
+  `Command` does, `.exe` added on Windows. Each argument is passed as it
+  is, with no shell; Rust's `Command` quotes it for Windows. The child
+  inherits the environment and the working directory.
+- **The status** is the exit code; a death by signal on Unix is
+  `128 + signal`, as shells report it, so a child's panic, an abort, gives
+  134. Windows always has a code.
+- **Errors** follow `std/fs`'s mapping: a missing program is `NotFound`,
+  one that cannot be run `PermissionDenied`. A program that runs and fails
+  is not an error.
+- **`env(name)`** is `None` when the variable is unset, its value is not
+  UTF-8, or `name` is empty or holds `=` or NUL, which is not asked of the
+  system. **`current_exe()`** is the running executable's path, `nova`
+  itself under `nova run`; a path that is not Unicode is `InvalidData`.
+- **The wire contract** is `std/fs`'s: six `STD_ONLY` builtins,
+  `process_run(program, args, capture) -> Int`, a status code, then
+  `process_take_status`, `process_take_stdout` and `process_take_stderr`,
+  whose payload the runtime holds, as Rust bytes, until taken;
+  `process_env(name) -> Bool` and `process_current_exe() -> Int`, whose
+  text waits for `fs_take_string`; a failure's message waits for
+  `fs_last_error_message`. `STD_ONLY` grows from 94 to 100.
+- **The names are glob-imported** into every module, as `args` is, and a
+  program's own `run`, `status`, `env` or `current_exe` shadows them,
+  silently. The record is `ProcessOutput`, not `Output`.
+- **Not built:** a `Command` builder with a working directory, an
+  environment or standard input; asynchronous processes; processes on
+  WASM (§11 of the spec).
+
 ---
 
 ## 18. `std/strings`
@@ -2910,6 +3067,60 @@ impl String {
   `string_slice_start_after_end_panics` and
   `string_repeat_negative_count_panics`.
 
+**Amended 2026-10-29 (branch `claude/loving-archimedes-frtf5q-50c`):**
+`std/strings` gains a builder, byte access, number parsing and
+`string_from_chars`
+(`docs/superpowers/specs/2026-10-26-phase-5-0-the-language-and-std-a-compiler-needs-design.md`
+§5.1 to §5.3), each in the module's one `impl String` block but the free
+function and the record:
+
+```nova
+impl String {
+    pub fn byte_len(self) -> Int
+    pub fn byte_at(self, i: Int) -> Int
+    pub fn byte_slice(self, start: Int, end: Int) -> String
+    pub fn parse_int(self) -> Option<Int>
+    pub fn parse_int_radix(self, radix: Int) -> Option<Int>
+    pub fn parse_float(self) -> Option<Float>
+}
+pub fn string_from_chars(cs: [Char]) -> String
+
+pub record StringBuilder { /* private: parts: Vec<String>, bytes: Int */ }
+impl StringBuilder {
+    pub fn new() -> StringBuilder
+    pub fn push(mut self, s: String)
+    pub fn push_char(mut self, c: Char)
+    pub fn byte_len(self) -> Int
+    pub fn is_empty(self) -> Bool
+    pub fn clear(mut self)
+    pub fn to_string(self) -> String
+}
+impl Display for StringBuilder
+```
+
+- **Bytes, in constant time.** These three count bytes, unlike every other
+  index here. The std-only builtin `str_as_bytes` reads a `String` as a
+  `Bytes`, a register move, since the two share one layout and neither
+  changes, so they are `Bytes`' builtins on the string's own bytes.
+  `byte_at` panics past the end, and `byte_slice` when a bound is out of
+  range, `start > end`, or a bound falls inside a character; neither
+  allocates an `Option`.
+- **Numbers.** `parse_int` takes an optional sign and one or more ASCII
+  digits, nothing else, and is `None` outside `Int`'s range, its least
+  value included as text. `parse_int_radix` is the same in radix 2 to 36,
+  letters of either case, no prefix, and panics on any other radix.
+  `parse_float` takes the text Rust's `f64::from_str` takes, `inf`,
+  `infinity` and `nan` in any case included, and gives its value: the
+  std-only builtin `str_is_float` says whether, and `str_to_float` what.
+- **`StringBuilder`.** `push` is a `Vec` push, constant time amortised;
+  `to_string` copies the parts into an array of their count, then joins
+  them in one copy through `str_join`, so building n bytes is linear,
+  where `+` or interpolation in a loop is quadratic. It leaves the builder
+  as it was. `byte_len` is kept as parts are pushed; there is no `len`,
+  which would count characters by walking every part.
+- **Builtins.** `str_as_bytes`, `str_is_float`, `int_to_char` and
+  `float_to_bits` join the module's `STD_ONLY` builtins.
+
 ---
 
 ## 19. `std/bytes`
````

### Task 9: Verification, the mutants, the review, the PR, and the merge

Every step runs in a shell set up as the conventions say, the process limit
included.

- [ ] **Step 1: The full checks.** First, `df -h /` must show at least 6G
  free; if not, remove `target/debug/incremental` and older duplicate test
  executables.

  ```sh
  cargo build --locked --workspace
  cargo fmt --all -- --check
  cargo clippy --locked --all-targets --all-features -- -D warnings
  target/debug/nova fmt --check std examples tests/wasm tests/dom tests/browser/router-app tests/browser/ssr-app tests/ssr
  NOVA_REQUIRE_NODE=1 cargo test --locked --workspace --no-fail-fast > "$P/full-50c.txt" 2>&1
  grep '^test result' "$P/full-50c.txt" | awk '{p += $4; f += $6; i += $8} END {print p, f, i}'
  grep ' FAILED$' "$P/full-50c.txt"
  cargo test --locked --workspace -- --list > "$P/final-list-50c.txt" 2>&1
  ```

  Expected:
  - both formatters print nothing, and clippy ends with `Finished`, with no
    warning or error;
  - the suite, in 6 min 12 s: `2788 1 10`, the baseline's figures and 74
    more tests;
  - the one failure is Task 0's IPv6 test;
  - the two lists, compared binary by binary by name (a binary is its
    `Running` or `Doc-tests` line, hash stripped, an integration test's
    prefixed with the crate whose unit tests cargo listed just before it; a
    test a line ending `: test`), give 2725 tests in 110 binaries before and
    2799 in 110 after, 74 added and 0 gone, in the binaries that change:

    - `nova: tests/run_tests.rs (run_tests)`, 341 to 355:
      `a_child_killed_by_a_signal_is_128_plus_the_signal`;
      `array_helpers_run`; `char_and_bytes_run`;
      `current_exe_is_nova_under_run_and_the_program_when_built`;
      `each_std_panic_names_its_method`; `float_bits_runs`;
      `hidden_trait_methods_run`; `ordered_map_order_is_the_same_each_run`;
      `ordered_map_runs`; `parse_numbers_runs`; `process_env_runs`;
      `process_run_runs`; `string_builder_million_runs`;
      `string_builder_runs`;
    - `nova: tests/wasm_tests.rs (wasm_tests)`, 240 to 250:
      `each_std_panic_names_its_method_as_natively`; `gate::array_helpers`;
      `gate::char_and_bytes`; `gate::float_bits`;
      `gate::hidden_trait_methods`; `gate::ordered_map`;
      `gate::parse_numbers`; `gate::string_builder`;
      `gate::string_builder_million`; `std_process_is_e0092_on_wasm`;
    - `nova_codegen_llvm: tests/ir_tests.rs (ir_tests)`, 18 to 19:
      `float_constants_are_the_hex_that_to_bits_prints`;
    - `nova_codegen_wasm (unittests src/lib.rs)`, 34 to 35:
      `shim::tests::float_bits_is_rusts_to_bits`;
    - `nova_doc (unittests src/lib.rs)`, 61 to 62:
      `model::tests::an_impl_on_arrays_names_its_methods_after_array`;
    - `nova_lsp (unittests src/lib.rs)`, 31 to 33:
      `completion::tests::std_process_names_complete`;
      `completion::tests::std_records_of_5_0c_complete`;
    - `nova_mir: tests/lower_tests.rs (lower_tests)`, 67 to 74:
      `array_methods_monomorphise_per_element_type`;
      `byte_len_lowers_to_a_move_and_the_bytes_length`;
      `char_from_int_checks_the_range_and_moves`;
      `ordered_map_key_without_hash_reports_e0013`;
      `parse_float_asks_str_is_float_once`;
      `std_process_wrappers_call_their_runtime_builtins`;
      `to_bits_is_one_runtime_call`;
    - `nova_runtime (unittests src/lib.rs)`, 275 to 283:
      `process::tests::a_death_by_signal_is_128_plus_the_signal`;
      `process::tests::a_file_that_cannot_be_run_is_permission_denied`;
      `process::tests::a_missing_program_is_not_found`;
      `process::tests::a_program_on_path_runs_and_its_output_is_captured`;
      `process::tests::env_refuses_names_the_system_cannot_hold`;
      `process::tests::output_larger_than_a_pipe_is_read_whole`;
      `tests::float_to_bits_is_rusts_to_bits`;
      `tests::str_is_float_is_rusts_grammar`;
    - `nova_typeck (unittests src/lib.rs)`, 551 to 581:
      `check::std_tests::a_builder_mutates_only_through_a_mutable_binding`;
      `check::std_tests::a_program_may_write_an_impl_on_arrays`;
      `check::std_tests::a_programs_array_method_that_std_has_is_e0074`;
      `check::std_tests::a_programs_own_run_status_and_env_shadow_stds`;
      `check::std_tests::a_programs_trait_method_hidden_by_a_std_method_is_e0105`;
      `check::std_tests::a_static_trait_function_hidden_by_a_std_function_is_e0105`;
      `check::std_tests::a_std_only_builtin_named_by_a_program_gets_its_public_form`;
      `check::std_tests::a_trait_impl_on_an_array_is_e0010`;
      `check::std_tests::a_trait_method_hidden_by_floats_to_bits_is_e0105`;
      `check::std_tests::a_trait_method_hidden_by_strings_and_chars_methods_is_e0105`;
      `check::std_tests::a_trait_method_hidden_by_the_parsers_is_e0105`;
      `check::std_tests::a_trait_method_std_does_not_have_is_not_e0105`;
      `check::std_tests::an_arity_error_on_an_array_method_names_it_as_written`;
      `check::std_tests::an_array_iterators_adapters_infer_their_item`;
      `check::std_tests::an_arrays_head_is_array_whatever_its_element`;
      `check::std_tests::an_interpolated_array_is_e0013_without_the_display_advice`;
      `check::std_tests::an_ordered_map_mutates_only_through_a_mutable_binding`;
      `check::std_tests::an_unknown_array_method_is_e0014_with_its_methods_offered`;
      `check::std_tests::e0074_against_stds_array_impl_labels_the_programs_impl`;
      `check::std_tests::ordered_map_has_the_specs_types`;
      `check::std_tests::run_takes_an_array_of_strings`;
      `check::std_tests::std_array_methods_type_check_through_the_arrays_head`;
      `check::std_tests::std_process_has_the_specs_types`;
      `check::std_tests::the_builder_has_the_specs_types`;
      `check::std_tests::the_conversions_and_byte_access_have_the_specs_types`;
      `check::std_tests::the_new_builtins_are_not_visible_to_a_program`;
      `check::std_tests::the_parsers_have_the_specs_types`;
      `check::std_tests::the_process_builtins_are_not_visible_to_a_program`;
      `check::std_tests::to_bits_has_the_specs_type`;
      `check::std_tests::vecs_helpers_have_the_specs_types`.
- [ ] **Step 2: CI's Linux leg, as CI runs it.** The `test` job's
  `ubuntu-24.04` leg runs the whole suite with the golden suite required,
  with CI's own flags, `--locked --workspace --all-features --no-fail-fast`:

  ```sh
  NOVA_REQUIRE_CLANG=1 NOVA_CLANG=clang-18 NOVA_REQUIRE_NODE=1 cargo test --locked --workspace --all-features --no-fail-fast > "$P/ci-50c.txt" 2>&1
  grep '^test result' "$P/ci-50c.txt" | awk '{p += $4; f += $6; i += $8} END {print p, f, i}'
  grep ' FAILED$' "$P/ci-50c.txt"
  grep -c 'release_golden_suite ... ok' "$P/ci-50c.txt"
  ```

  Expected, in 6 min 18 s: `2788 1 10`, Step 1's figures, the one failure the
  IPv6 test, and `1`: the golden suite ran and passed inside the whole run,
  as on CI, where Step 1's run skips it.
- [ ] **Step 3: The golden suite.** `time G`. Expected, in 36 s of wall
  time:
  `release golden: 147 passed, 9 excluded, Ubuntu clang version 18.1.3 (1ubuntu1)`
  and `1 passed`: the base's 137 programs and 5.0c's ten, `process_env` with
  the two variables its `RELEASE_GOLDEN_ENV` entries set.
- [ ] **Step 4: The AST over the repository.** For every `.nova` file
  `git ls-tree -r --name-only e13630f0` lists,
  `"$P/nova-base-50c" parse --ast <file>` (Task 0's copy) on the file as
  `e13630f0` has it, from a `git archive e13630f0` unpacked under `$P`, and
  `target/debug/nova parse --ast <file>` on the tree's, each with its exit
  status appended, into two directories under `$P`; then compare each pair
  byte for byte, and remove the three directories. Expected:
  `ast: 285 files, 0 outside std differ, 4 std files differ; base exits nonzero on 2, the tree on 2`:
  5.0c changes no parser, and every file outside std parses to the same
  tree, spans included; the four std files that differ are the four whose
  text 5.0c changes, `std/collections`, `std/core`, `std/process` and
  `std/strings`.
- [ ] **Step 5: `nova check` over the repository.** For the same files, the
  base's `nova check <file>` and the tree's, each file not under a package
  alone and each package once from its root, output and exit status
  compared. Expected:
  `check: 251 checks, 0 outside std differ, 8 std files differ`. Every file
  outside std checks as at the base, so no repository program meets 5.0c's
  new global names, E0074, E0105 or a changed diagnostic: the seven that
  define a `run` of their own among them. The std files differ only where
  they are checked as user programs, which they are not built as:
  - `std/collections`: the base's E0010 "impl blocks are only supported on
    named types" and E0014 for `[T]::slice` become one E0074 for `iter`,
    std's `impl<T> [T]` meeting its own copy (decision 17);
  - `std/core`: E0074 for `to_int` and `to_bits` against std's own copy, and
    the public-form notes in place of "change `char_to_int` to
    `char_from_int`" (decision 19);
  - `std/strings`, `std/json`, `std/http` and `std/ui/ssr`: the public-form
    notes, for `char_to_int`, `str_from_chars`, `str_as_bytes`,
    `str_is_float` and `str_to_float`, where the base gave "change
    `str_from_chars` to `string_from_chars`" or nothing;
  - `std/bytes` and `std/fmt`: their E0074s against std's own copies are
    labelled at both, the file's impl first and std's copy second, "`len` is
    also defined by std's impl" and the like (decision 18).
  Then the probe decision 17 cites: a program with
  `impl<T> [T] { fn slice(self) -> Int { 0 } }` gives E0074, labelled at
  std's impl and at the program's, then E0016 inside std's
  `Vec::from_array`, whose `a.slice(0, a.len())` now resolves to the
  program's `slice`, as `impl String { fn split(self) -> Int }` gives
  E0016s inside std/http at the base. Then the fact-check's probes,
  each with the base's `nova` and the tree's:
  - `impl String { fn byte_len(self) -> Int { 99 } … }`, and the same for
    `Char`'s `to_int`, `Float`'s `to_bits` and `Vec<T>`'s `last`: clean at
    the base, one E0074 each now (decision 41);
  - `impl Size for String { fn byte_len … }` and
    `impl Top for Vec<Int> { fn last … }`, with calls of both: clean at the
    base, printing the traits' results, `5 -1`; now two E0105 warnings,
    "std's inherent method `byte_len` on `String` hides `Size::byte_len`"
    and the `last` one, and the program prints std's results, `6 5`
    (decision 40);
  - `impl Display for [Int]`, `impl Display for [String]`,
    `impl Hash for [Int]` and `impl Eq for [Int]`: four E0010 and two E0013
    at the base and now, the E0010s now "a trait cannot be implemented for
    an array" (decision 17);
  - `impl [Int] { fn slice(self, a: Int, b: Int) -> Int { a + b } }`: E0010
    and E0014 at the base, one E0074 now; `a.slice(1)` and `a.iter(3)` on
    `[1, 2]`: two E0014 at the base, two E0016 now, "`[T]::slice` takes 2
    argument(s) but 1 were supplied" (decision 42).
  - the second fact-check's (`$P/fc50c2/p/`): `fp` and `fp2`, a trait method
    `from_array(self)` or `new(self)` on `Vec<Int>` beside std's selfless
    functions, now draw nothing; `static`, a static `Mk::from_array`, draws
    "std's inherent function `from_array` on `Vec<Int>` hides
    `Mk::from_array`" and prints std's 3; `genvec` shows `Vec<T>`; `s/e60`
    gives "`[Int]::bump` mutates its receiver, but `a` is immutable" and
    `s/vecat` "`Vec::at` takes 1 argument(s) but 0 were supplied" (decisions
    40, 42 and 45).
- [ ] **Step 6: Where E0102, E0103, E0104 and E0105 fire.**
  `target/debug/nova check` on every `.nova` file `git ls-files` lists that
  is not under a package, and `nova check` from each package's root once,
  counting `warning[E0102]`, `warning[E0103]`, `warning[E0104]` and
  `warning[E0105]`. Expected:
  `fires: 261 checks, E0102 53, E0103 0, E0104 0, E0105 3`: E0102 to E0104
  as at 5.0b and none in 5.0c's ten programs; E0105 three times in
  `hidden_trait_methods`, which hides three trait methods on purpose, and
  nowhere else over std, the examples and the tests (decisions 40 and 45).
- [ ] **Step 7: The lockfile and Rust 1.78.**
  - `git diff --numstat e13630f0 -- Cargo.lock`. Expected: nothing.
  - Then:

    ```sh
    RUSTFLAGS="-D warnings" CARGO_TARGET_DIR=/dev/shm/msrv-50c cargo +1.78 check --locked --workspace
    rm -rf /dev/shm/msrv-50c
    ```

    Expected: clean, `Finished` in 42.50s. It checks the libraries
    and binaries, not the tests: `assert_cmd` 2.2.1's manifest does not
    parse on 1.78. Its target directory is in `/dev/shm`, memory, so the
    disk stays above 6G.
- [ ] **Step 8: The probe of `std/process` by hand,** under the process
  limit. In a directory of its own under `$P`, `main.nova`:

  ```nova
  fn kind(e: IoError) -> String {
      match e.kind {
          NotFound => "NotFound"
          PermissionDenied => "PermissionDenied"
          _ => "another"
      }
  }

  fn show(r: Result<Int, IoError>) -> String {
      match r {
          Ok(n) => "Ok(${n})"
          Err(e) => "Err(${kind(e)})"
      }
  }

  fn main() {
      println(current_exe().unwrap())
      println(show(status("false", [])))
      println(show(status("sh", ["-c", "exit 300"])))
      println(show(status("", [])))
      println(show(status("/tmp", [])))
  }
  ```

  Run it with `target/debug/nova run main.nova` (by its full path), then
  build it, `nova build main.nova -o probe`, and run `./probe`. Expected:
  the path of `target/debug/nova`, then `Ok(1)`, `Ok(44)` (Unix keeps the
  low eight bits of 300), `Err(NotFound)` and `Err(PermissionDenied)`;
  then the path of `probe` and the same four lines, as step 0's probe and
  the replay gave.
- [ ] **Step 9: The mutants.** One at a time, each edit is made, its command
  run, and the file restored with `git checkout -- <file>`, all on the last
  commit, in a shell under the process limit.
  - Each edit replaces one exact text, which occurs once in its file; "the
    line goes" removes the whole line holding it.
  - Each command starts with `cargo build --locked --workspace`, since
    `run_tests` and `wasm_tests` link the runtime library beside `nova`,
    which only that build writes, and several mutants change `nova-runtime`.
  - The replay's run took 34 min 56 s in all; most take under half a minute.
  - Each caught mutant must fail as written: the tests named, or the text
    named in the command's output. A mutant that passes is a gap: record it
    in the ledger, name it in the PR, and give it to the review (Step 12),
    which says whether a check is added.
  - **65 is the runaway child.** With the guard it fails `process_run_runs`
    at once, each child stopping at the marker; never run it, or any process
    mutant, without the process limit.
  - At the end, `git status --short` prints nothing, and
    `cargo build --locked --workspace` rebuilds the last commit's binaries
    (decision 38).

  1. **`Vec::at` checks the storage's length, not `len`.** `std/collections/lib.nova`:
     `if i < 0 || i >= self.len { panic("Vec::at index out of range") }` becomes `if i < 0 || i >= self.data.len() { panic("Vec::at index out of range") }`.
     - Run `cargo build --locked --workspace` and `R each_std_panic_names_its_method array_helpers_run`, each to its end.
     - Expected: fails: `each_std_panic_names_its_method`.
  2. **`Vec::last` does not check for an empty vector.** `std/collections/lib.nova`:
     `if self.len == 0 { panic("Vec::last on an empty vector") }`: the line goes.
     - Run `cargo build --locked --workspace` and `R each_std_panic_names_its_method array_helpers_run`, each to its end.
     - Expected: fails: `each_std_panic_names_its_method`.
  3. **`Vec::to_array` leaves element 1 its fill.** `std/collections/lib.nova`:
     `for i in 1..self.len { out[i] = self.data[i] }` becomes `for i in 2..self.len { out[i] = self.data[i] }`.
     - Run `cargo build --locked --workspace` and `R array_helpers_run string_builder_runs`, each to its end.
     - Expected: fails: `array_helpers_run`, `string_builder_runs`.
  4. **`Vec::from_array` shares the array.** `std/collections/lib.nova`:
     `Vec { len: a.len(), data: a.slice(0, a.len()) }` becomes `Vec { len: a.len(), data: a }`.
     - Run `cargo build --locked --workspace` and `R array_helpers_run`, each to its end.
     - Expected: fails: `array_helpers_run`.
  5. **`[T]::slice` does not check its end.** `std/collections/lib.nova`:
     `if end > self.len() { panic("[T]::slice end is past the end of the array") }`: the line goes.
     - Run `cargo build --locked --workspace` and `R each_std_panic_names_its_method`, each to its end.
     - Expected: fails: `each_std_panic_names_its_method`.
  6. **`[T]::slice` copies from 0, not from `start`.** `std/collections/lib.nova`:
     `for i in 1..n { out[i] = self[start + i] }` becomes `for i in 1..n { out[i] = self[i] }`.
     - Run `cargo build --locked --workspace` and `R array_helpers_run`, each to its end.
     - Expected: fails: `array_helpers_run`.
  7. **`ArrayIter` reads one past the end.** `std/collections/lib.nova`:
     `if self.i >= self.a.len() { return None }` becomes `if self.i > self.a.len() { return None }`.
     - Run `cargo build --locked --workspace` and `R array_helpers_run`, each to its end.
     - Expected: fails: `array_helpers_run`.
  8. **An array has no type head.** `crates/nova-hir/src/lib.rs`:
     `Ty::Array(_) => Some(TyHead::Array),`: the line goes.
     - Run `cargo build --locked --workspace` and `T check::std_tests`, each to its end.
     - Expected: fails: `check::std_tests::a_builder_mutates_only_through_a_mutable_binding`, `check::std_tests::a_program_may_write_an_impl_on_arrays`, `check::std_tests::a_programs_array_method_that_std_has_is_e0074`, `check::std_tests::a_programs_own_run_status_and_env_shadow_stds`, `check::std_tests::a_trait_impl_on_an_array_is_e0010`, `check::std_tests::a_trait_method_std_does_not_have_is_not_e0105`, `check::std_tests::an_arity_error_on_an_array_method_names_it_as_written`, `check::std_tests::an_array_iterators_adapters_infer_their_item`, `check::std_tests::an_arrays_head_is_array_whatever_its_element`, `check::std_tests::an_ordered_map_mutates_only_through_a_mutable_binding`, `check::std_tests::an_unknown_array_method_is_e0014_with_its_methods_offered`, `check::std_tests::e0074_against_stds_array_impl_labels_the_programs_impl`, `check::std_tests::ordered_map_has_the_specs_types`, `check::std_tests::std_array_methods_type_check_through_the_arrays_head`, `check::std_tests::std_process_has_the_specs_types`, `check::std_tests::the_builder_has_the_specs_types`, `check::std_tests::the_conversions_and_byte_access_have_the_specs_types`, `check::std_tests::the_parsers_have_the_specs_types`, `check::std_tests::to_bits_has_the_specs_type`, `check::std_tests::vecs_helpers_have_the_specs_types`.
  9. **An array's methods are not looked up in its impls.** `crates/nova-typeck/src/check.rs`:
     replace

     ```
                 if matches!(
                     self.resolve_method_on(&recv_ty, fcx, &method.value),
                     MethodRes::None
                 ) {
                     self.error(
                         "E0014",
     ```

     with

     ```
                 if true {
                     self.error(
                         "E0014",
     ```

     - Run `cargo build --locked --workspace` and `T check::std_tests`, each to its end.
     - Expected: fails: `check::std_tests::a_builder_mutates_only_through_a_mutable_binding`, `check::std_tests::a_program_may_write_an_impl_on_arrays`, `check::std_tests::a_programs_own_run_status_and_env_shadow_stds`, `check::std_tests::a_trait_impl_on_an_array_is_e0010`, `check::std_tests::a_trait_method_std_does_not_have_is_not_e0105`, `check::std_tests::an_arity_error_on_an_array_method_names_it_as_written`, `check::std_tests::an_array_iterators_adapters_infer_their_item`, `check::std_tests::an_ordered_map_mutates_only_through_a_mutable_binding`, `check::std_tests::an_unknown_array_method_is_e0014_with_its_methods_offered`, `check::std_tests::ordered_map_has_the_specs_types`, `check::std_tests::std_array_methods_type_check_through_the_arrays_head`, `check::std_tests::std_process_has_the_specs_types`, `check::std_tests::the_builder_has_the_specs_types`, `check::std_tests::the_conversions_and_byte_access_have_the_specs_types`, `check::std_tests::the_parsers_have_the_specs_types`, `check::std_tests::to_bits_has_the_specs_type`, `check::std_tests::vecs_helpers_have_the_specs_types`.
  10. **E0014's fix on an array leaves out `len`.** `crates/nova-typeck/src/check/fixes.rs`:
      `names.push("len".to_string());`: the line goes.
      - Run `cargo build --locked --workspace` and `T check::std_tests`, each to its end.
      - Expected: fails: `check::std_tests::an_unknown_array_method_is_e0014_with_its_methods_offered`.
  11. **An array's members leave out `len`.** `crates/nova-typeck/src/check.rs`:
      `if let Ty::Array(_) = recv_ty {` becomes `if let Ty::Tuple(_) = recv_ty {`.
      - Run `cargo build --locked --workspace` and `T check::tests::members_of_a_string_and_of_an_array`, each to its end.
      - Expected: fails: `check::tests::members_of_a_string_and_of_an_array`.
  12. **E0074 labels one impl alone.** `crates/nova-typeck/src/check.rs`:
      `if let Some(other) = other {` becomes `if let Some(other) = None::<Span> {`.
      - Run `cargo build --locked --workspace` and `T check::std_tests`, each to its end.
      - Expected: fails: `check::std_tests::e0074_against_stds_array_impl_labels_the_programs_impl`.
  13. **E0074 against std's impl puts std's impl first.** `crates/nova-typeck/src/check.rs`:
      `Some(a_span) if std_b => (a_span, Some(b_span)),` becomes `Some(a_span) if false && std_b => (a_span, Some(b_span)),`.
      - Run `cargo build --locked --workspace` and `T check::std_tests`, each to its end.
      - Expected: fails: `check::std_tests::e0074_against_stds_array_impl_labels_the_programs_impl`.
  14. **The docs name an impl on arrays `type`.** `crates/nova-doc/src/model.rs`:
      `Type::Array(_) => "Array".to_string(),` becomes `Type::Array(_) => "type".to_string(),`.
      - Run `cargo build --locked --workspace` and `T an_impl_on_arrays_names_its_methods_after_array`, each to its end.
      - Expected: fails: `model::tests::an_impl_on_arrays_names_its_methods_after_array`.
  15. **A trait impl on an array checks.** `crates/nova-typeck/src/check.rs`:
      `if trait_id.is_some() && self_head == TyHead::Array {` becomes `if false && trait_id.is_some() && self_head == TyHead::Array {`.
      - Run `cargo build --locked --workspace` and `T check::std_tests`, each to its end.
      - Expected: fails: `check::std_tests::a_trait_impl_on_an_array_is_e0010`.
  16. **An array method's name shows as the resolver's `Arr_T.slice`.** `crates/nova-typeck/src/check.rs`:
      `Some((ty, method)) if ty.starts_with("Arr_") =>` becomes `Some((ty, method)) if ty.starts_with("Arr_") && false =>`.
      - Run `cargo build --locked --workspace` and `T check::std_tests`, each to its end.
      - Expected: fails: `check::std_tests::an_arity_error_on_an_array_method_names_it_as_written`.
  17. **E0105 is never checked.** `crates/nova-typeck/src/check.rs`:
      `self.check_hidden_trait_methods(` becomes `let _ = (`.
      - Run `cargo build --locked --workspace` and `T check::std_tests`, each to its end.
      - Expected: fails: `check::std_tests::a_programs_trait_method_hidden_by_a_std_method_is_e0105`, `check::std_tests::a_static_trait_function_hidden_by_a_std_function_is_e0105`, `check::std_tests::a_trait_method_hidden_by_floats_to_bits_is_e0105`, `check::std_tests::a_trait_method_hidden_by_strings_and_chars_methods_is_e0105`, `check::std_tests::a_trait_method_hidden_by_the_parsers_is_e0105`.
  18. **E0105 is never checked, at Task 1's code.** `crates/nova-typeck/src/check.rs`:
      `self.check_hidden_trait_methods(` becomes `let _ = (`.
      - At Task 1's commit: `git checkout` it first, and `git checkout claude/loving-archimedes-frtf5q-50c` after.
      - Run `cargo build --locked --workspace` and `T check::std_tests`, each to its end.
      - Expected: fails: `check::std_tests::a_programs_trait_method_hidden_by_a_std_method_is_e0105`, `check::std_tests::a_static_trait_function_hidden_by_a_std_function_is_e0105`.
  19. **E0105 passes over a trait's default methods.** `crates/nova-typeck/src/check.rs`:
      `if m.default_def.is_some() && !a.methods.iter().any(|(n, _)| *n == m.name) {` becomes `if m.default_def.is_none() && !a.methods.iter().any(|(n, _)| *n == m.name) {`.
      - Run `cargo build --locked --workspace` and `T check::std_tests`, each to its end.
      - Expected: fails: `check::std_tests::a_trait_method_hidden_by_strings_and_chars_methods_is_e0105`.
  20. **E0105 fires for the program's own inherent method.** `crates/nova-typeck/src/check.rs`:
      `if b.trait_id.is_some() || !in_std.get(j).copied().unwrap_or(false) {` becomes `if b.trait_id.is_some() {`.
      - Run `cargo build --locked --workspace` and `T check::std_tests`, each to its end.
      - Expected: fails: `check::std_tests::a_trait_method_std_does_not_have_is_not_e0105`.
  21. **E0105 fires whatever type std's method is on.** `crates/nova-typeck/src/check.rs`:
      replace

      ```
                      if a.self_head != b.self_head
                          || !hir::self_types_overlap(&a.self_ty, a.generics, &b.self_ty, b.generics)
                      {
      ```

      with

      ```
                      if false {
      ```

      - Run `cargo build --locked --workspace` and `T check::std_tests`, each to its end.
      - Expected: fails: `check::std_tests::a_trait_method_hidden_by_strings_and_chars_methods_is_e0105`, `check::std_tests::a_trait_method_std_does_not_have_is_not_e0105`.
  22. **E0105 labels the impl, not the hidden method.** `crates/nova-typeck/src/check.rs`:
      `span.unwrap_or(spans[i]),` becomes `spans[i],`.
      - Run `cargo build --locked --workspace` and `T check::std_tests`, each to its end.
      - Expected: fails: `check::std_tests::a_programs_trait_method_hidden_by_a_std_method_is_e0105`.
  23. **E0105 pairs a method with a selfless function.** `crates/nova-typeck/src/check.rs`:
      `if std_has_self != *has_self {` becomes `if false && std_has_self != *has_self {`.
      - Run `cargo build --locked --workspace` and `T check::std_tests`, each to its end.
      - Expected: fails: `check::std_tests::a_trait_method_std_does_not_have_is_not_e0105`.
  24. **E0105 passes over two static functions.** `crates/nova-typeck/src/check.rs`:
      `if std_has_self != *has_self {` becomes `if !*has_self || std_has_self != *has_self {`.
      - Run `cargo build --locked --workspace` and `T check::std_tests`, each to its end.
      - Expected: fails: `check::std_tests::a_static_trait_function_hidden_by_a_std_function_is_e0105`.
  25. **E0105 words a static function's hide as a method call's.** `crates/nova-typeck/src/check.rs`:
      `let d = if *has_self {` becomes `let d = if true {`.
      - Run `cargo build --locked --workspace` and `T check::std_tests`, each to its end.
      - Expected: fails: `check::std_tests::a_static_trait_function_hidden_by_a_std_function_is_e0105`.
  26. **E0105 shows the impl's parameters as `T0`.** `crates/nova-typeck/src/check.rs`:
      `param_names.get(i).unwrap_or(&no_names),` becomes `&no_names,`.
      - Run `cargo build --locked --workspace` and `T check::std_tests`, each to its end.
      - Expected: fails: `check::std_tests::a_programs_trait_method_hidden_by_a_std_method_is_e0105`.
  27. **E0105 shows every call as `x.m()`.** `crates/nova-typeck/src/check.rs`:
      `if declared > usize::from(std_has_self) {` becomes `if declared > 99 {`.
      - Run `cargo build --locked --workspace` and `T check::std_tests`, each to its end.
      - Expected: fails: `check::std_tests::a_programs_trait_method_hidden_by_a_std_method_is_e0105`, `check::std_tests::a_static_trait_function_hidden_by_a_std_function_is_e0105`, `check::std_tests::a_trait_method_hidden_by_the_parsers_is_e0105`.
  28. **A generic type's method shows as `Vec_T.at`.** `crates/nova-typeck/src/check.rs`:
      `(Some(imp), Some((_, method))) => {` becomes `(Some(imp), Some((_, method))) if false => {`.
      - Run `cargo build --locked --workspace` and `T check::std_tests`, each to its end.
      - Expected: fails: `check::std_tests::an_arity_error_on_an_array_method_names_it_as_written`.
  29. **An interpolated array is told to implement `Display`.** `crates/nova-typeck/src/check.rs`:
      `let advice = if matches!(other, Ty::Array(_)) {` becomes `let advice = if false {`.
      - Run `cargo build --locked --workspace` and `T check::std_tests`, each to its end.
      - Expected: fails: `check::std_tests::an_interpolated_array_is_e0013_without_the_display_advice`.
  30. **`char_from_int` takes the surrogates.** `std/core/lib.nova`:
      `if n >= 0xD800 && n <= 0xDFFF { return None }`: the line goes.
      - Run `cargo build --locked --workspace` and `R char_and_bytes_run`, each to its end.
      - Expected: fails: `char_and_bytes_run`.
  31. **`char_from_int` takes a negative value.** `std/core/lib.nova`:
      replace

      ```
          if n < 0 { return None }
          if n > 0x10FFFF
      ```

      with

      ```
          if n > 0x10FFFF
      ```

      - Run `cargo build --locked --workspace` and `R char_and_bytes_run`, each to its end.
      - Expected: fails: `char_and_bytes_run`.
  32. **`str_as_bytes` copies, through `bytes_from_string`.** `crates/nova-mir/src/lower.rs`:
      `Builtin::StrAsBytes => Lowering::Reinterpret,` becomes `Builtin::StrAsBytes => Lowering::Runtime(RtFunc::BytesFromString),`.
      - Run `cargo build --locked --workspace` and `L byte_len`, each to its end.
      - Expected: fails: `byte_len_lowers_to_a_move_and_the_bytes_length`.
  33. **`byte_at` leaves its upper bound to `bytes_at`.** `std/strings/lib.nova`:
      `if i < 0 || i >= bytes_len(b) { panic("String::byte_at index out of range") }` becomes `if i < 0 { panic("String::byte_at index out of range") }`.
      - Run `cargo build --locked --workspace` and `R each_std_panic_names_its_method`, each to its end.
      - Expected: fails: `each_std_panic_names_its_method`.
  34. **`byte_slice` takes a start inside a character.** `std/strings/lib.nova`:
      `if start < n && continues_char(bytes_at(b, start)) {` becomes `if false && continues_char(bytes_at(b, start)) {`.
      - Run `cargo build --locked --workspace` and `R each_std_panic_names_its_method`, each to its end.
      - Expected: fails: `each_std_panic_names_its_method`.
  35. **A character's first byte reads as a continuation.** `std/strings/lib.nova`:
      `fn continues_char(b: Int) -> Bool { b >= 128 && b <= 191 }` becomes `fn continues_char(b: Int) -> Bool { b >= 128 }`.
      - Run `cargo build --locked --workspace` and `R char_and_bytes_run`, each to its end.
      - Expected: fails: `char_and_bytes_run`.
  36. **A std-only builtin's name gets a guess, not its public form.** `crates/nova-typeck/src/check/fixes.rs`:
      `STD_ONLY_PUBLIC_FORMS.iter().find(|(b, _)| *b == name.value)` becomes `None::<&(&str, &str)>`.
      - Run `cargo build --locked --workspace` and `T check::std_tests`, each to its end.
      - Expected: fails: `check::std_tests::a_std_only_builtin_named_by_a_program_gets_its_public_form`.
  37. **`str_as_bytes` is left out of `STD_ONLY`, so std cannot name it.** `crates/nova-resolver/src/lib.rs`:
      `Builtin::StrAsBytes,` becomes `Builtin::StrToFloat,`.
      - Run `cargo build --locked --workspace` and `T check::std_tests`, each to its end.
      - Expected: fails: `check::std_tests::a_builder_mutates_only_through_a_mutable_binding`, `check::std_tests::a_program_may_write_an_impl_on_arrays`, `check::std_tests::a_programs_own_run_status_and_env_shadow_stds`, `check::std_tests::a_trait_impl_on_an_array_is_e0010`, `check::std_tests::a_trait_method_std_does_not_have_is_not_e0105`, `check::std_tests::an_array_iterators_adapters_infer_their_item`, `check::std_tests::an_ordered_map_mutates_only_through_a_mutable_binding`, `check::std_tests::an_unknown_array_method_is_e0014_with_its_methods_offered`, `check::std_tests::ordered_map_has_the_specs_types`, `check::std_tests::std_array_methods_type_check_through_the_arrays_head`, `check::std_tests::std_process_has_the_specs_types`, `check::std_tests::the_builder_has_the_specs_types`, `check::std_tests::the_conversions_and_byte_access_have_the_specs_types`, `check::std_tests::the_parsers_have_the_specs_types`, `check::std_tests::to_bits_has_the_specs_type`, `check::std_tests::vecs_helpers_have_the_specs_types`.
  38. **The builder counts characters, not bytes.** `std/strings/lib.nova`:
      `self.bytes = self.bytes + s.byte_len()` becomes `self.bytes = self.bytes + s.len()`.
      - Run `cargo build --locked --workspace` and `R string_builder_runs`, each to its end.
      - Expected: fails: `string_builder_runs`.
  39. **`clear` keeps the byte count.** `std/strings/lib.nova`:
      replace

      ```
              self.parts.clear()
              self.bytes = 0
      ```

      with

      ```
              self.parts.clear()
      ```

      - Run `cargo build --locked --workspace` and `R string_builder_runs`, each to its end.
      - Expected: fails: `string_builder_runs`.
  40. **`to_string` joins with a space.** `std/strings/lib.nova`:
      `str_join("", self.parts.to_array())` becomes `str_join(" ", self.parts.to_array())`.
      - Run `cargo build --locked --workspace` and `R string_builder_runs string_builder_million_runs`, each to its end.
      - Expected: fails: `string_builder_million_runs`, `string_builder_runs`.
  41. **The builder interpolates as nothing.** `std/strings/lib.nova`:
      `fn fmt(self) -> String { self.to_string() }` becomes `fn fmt(self) -> String { "" }`.
      - Run `cargo build --locked --workspace` and `R string_builder_runs`, each to its end.
      - Expected: fails: `string_builder_runs`.
  42. **The runtime's `float_to_bits` drops a bit.** `crates/nova-runtime/src/lib.rs`:
      `v.to_bits() as i64` becomes `(v.to_bits() >> 1) as i64`.
      - Run `cargo build --locked --workspace`, `T float_to_bits_is_rusts_to_bits` and `R float_bits_runs`, each to its end.
      - Expected: fails: `float_bits_runs`, `tests::float_to_bits_is_rusts_to_bits`.
  43. **The shim's `floatBits` reads the bits unsigned.** `crates/nova-codegen-wasm/src/shim/float.js`:
      `return view.getBigInt64(0);` becomes `return view.getBigUint64(0);`.
      - Run `cargo build --locked --workspace`, `T float_bits_is_rusts_to_bits` and `W gate::float_bits`, each to its end.
      - Expected: fails: `shim::tests::float_bits_is_rusts_to_bits`.
  44. **The float helpers do not come with `float_to_bits` alone.** `crates/nova-codegen-wasm/src/shim.rs`:
      replace

      ```
              "str_is_float",
              "float_to_bits",
          ]) {
      ```

      with

      ```
              "str_is_float",
          ]) {
      ```

      - Run `cargo build --locked --workspace` and `T shim::tests::helpers_come_only_with_their_host_functions`, each to its end.
      - Expected: fails: `shim::tests::helpers_come_only_with_their_host_functions`.
  45. **`float_to_bits` is the shim's `float_to_str` on WASM.** `crates/nova-mir/src/lib.rs`:
      `RtFunc::FloatToBits => WasmRuntime::Host("float_to_bits"),` becomes `RtFunc::FloatToBits => WasmRuntime::Host("float_to_str"),`.
      - Run `cargo build --locked --workspace` and `W gate::float_bits gate::parse_numbers`, each to its end.
      - Expected: fails: `gate::float_bits`, `gate::parse_numbers`.
  46. **`float_to_bits` returns a `Float`.** `crates/nova-typeck/src/check.rs`:
      `Builtin::FloatToBits => (vec![Ty::Float], Ty::Int),` becomes `Builtin::FloatToBits => (vec![Ty::Float], Ty::Float),`.
      - Run `cargo build --locked --workspace` and `T check::tests::builtin_signatures_are_what_the_std_call_sites_use`, each to its end.
      - Expected: fails: `check::tests::builtin_signatures_are_what_the_std_call_sites_use`.
  47. **`parse_digits` does not check before it multiplies.** `std/strings/lib.nova`:
      `if acc < floor { return None }`: the line goes.
      - Run `cargo build --locked --workspace` and `R parse_numbers_runs`, each to its end.
      - Expected: fails: `parse_numbers_runs`.
  48. **`parse_digits` does not check before it subtracts.** `std/strings/lib.nova`:
      `if acc < least + d { return None }`: the line goes.
      - Run `cargo build --locked --workspace` and `R parse_numbers_runs`, each to its end.
      - Expected: fails: `parse_numbers_runs`.
  49. **A positive `-least` parses.** `std/strings/lib.nova`:
      `if acc == least { return None }`: the line goes.
      - Run `cargo build --locked --workspace` and `R parse_numbers_runs`, each to its end.
      - Expected: fails: `parse_numbers_runs`.
  50. **A digit beyond the radix is taken.** `std/strings/lib.nova`:
      `if d < 0 || d >= radix { return None }` becomes `if d < 0 { return None }`.
      - Run `cargo build --locked --workspace` and `R parse_numbers_runs`, each to its end.
      - Expected: fails: `parse_numbers_runs`.
  51. **Upper-case letters are not digits.** `std/strings/lib.nova`:
      `if b >= 65 && b <= 90 { return b - 55 }`: the line goes.
      - Run `cargo build --locked --workspace` and `R parse_numbers_runs`, each to its end.
      - Expected: fails: `parse_numbers_runs`.
  52. **A leading `+` is refused.** `std/strings/lib.nova`:
      `} else if first == 43 {` becomes `} else if first == 0 {`.
      - Run `cargo build --locked --workspace` and `R parse_numbers_runs`, each to its end.
      - Expected: fails: `parse_numbers_runs`.
  53. **Radix 1 is taken.** `std/strings/lib.nova`:
      `if radix < 2 || radix > 36 {` becomes `if radix < 1 || radix > 36 {`.
      - Run `cargo build --locked --workspace` and `R each_std_panic_names_its_method`, each to its end.
      - Expected: fails: `each_std_panic_names_its_method`.
  54. **The runtime's `str_is_float` says yes to everything.** `crates/nova-runtime/src/lib.rs`:
      `i8::from(as_str(s).parse::<f64>().is_ok())` becomes `1`.
      - Run `cargo build --locked --workspace`, `T str_is_float_is_rusts_grammar` and `R parse_numbers_runs`, each to its end.
      - Expected: fails: `parse_numbers_runs`, `tests::str_is_float_is_rusts_grammar`.
  55. **The shim's `str_is_float` says yes to everything.** `crates/nova-codegen-wasm/src/shim.rs`:
      `(FLOAT_TEXT.test(text(s)) ? 1 : 0)` becomes `(FLOAT_TEXT.test(text(s)) ? 1 : 1)`.
      - Run `cargo build --locked --workspace` and `W gate::parse_numbers`, each to its end.
      - Expected: fails: `gate::parse_numbers`.
  56. **The shim's `str_to_float` drops a `-nan`'s sign.** `crates/nova-codegen-wasm/src/shim/float.js`:
      `if (body === "nan") return s.startsWith("-") ? -NaN : NaN;` becomes `if (body === "nan") return NaN;`.
      - Run `cargo build --locked --workspace`, `T float_from_is_rusts_parse` and `W gate::parse_numbers gate::float_bits`, each to its end.
      - Expected: fails: `gate::float_bits`, `gate::parse_numbers`, `shim::tests::float_from_is_rusts_parse`.
  57. **`parse_float` takes any text.** `std/strings/lib.nova`:
      `if str_is_float(self) { Some(str_to_float(self)) } else { None }` becomes `Some(str_to_float(self))`.
      - Run `cargo build --locked --workspace` and `R parse_numbers_runs`, each to its end.
      - Expected: fails: `parse_numbers_runs`.
  58. **`str_is_float` calls the runtime's `str_to_float`.** `crates/nova-mir/src/lib.rs`:
      `RtFunc::StrIsFloat => "nova_rt_str_is_float",` becomes `RtFunc::StrIsFloat => "nova_rt_str_to_float",`.
      - Run `cargo build --locked --workspace` and `R parse_numbers_runs`, each to its end.
      - Expected: fails: `parse_numbers_runs`.
  59. **Compaction waits for more than half the slots.** `std/collections/lib.nova`:
      `if gaps > 0 && gaps * 2 >= slots { self.compact() }` becomes `if gaps > 0 && gaps * 2 > slots { self.compact() }`.
      - Run `cargo build --locked --workspace`, `R ordered_map` and `W gate::ordered_map`, each to its end.
      - Expected: fails: `gate::ordered_map`, `ordered_map_order_is_the_same_each_run`, `ordered_map_runs`.
  60. **Compaction does not move the positions in the index.** `std/collections/lib.nova`:
      `let _ = self.index.insert(k, keys.len())`: the line goes.
      - Run `cargo build --locked --workspace` and `R ordered_map`, each to its end.
      - Expected: fails: `ordered_map_order_is_the_same_each_run`, `ordered_map_runs`.
  61. **`remove` leaves its slot live.** `std/collections/lib.nova`:
      `self.live.set(at, false)`: the line goes.
      - Run `cargo build --locked --workspace` and `R ordered_map`, each to its end.
      - Expected: fails: `ordered_map_order_is_the_same_each_run`, `ordered_map_runs`.
  62. **The iterator yields dead slots.** `std/collections/lib.nova`:
      `if self.m.live.at(at) { return Some(` becomes `if true { return Some(`.
      - Run `cargo build --locked --workspace` and `R ordered_map`, each to its end.
      - Expected: fails: `ordered_map_order_is_the_same_each_run`, `ordered_map_runs`.
  63. **Updating the first key appends it again.** `std/collections/lib.nova`:
      `if at >= 0 {` becomes `if at > 0 {`.
      - Run `cargo build --locked --workspace` and `R ordered_map`, each to its end.
      - Expected: fails: `ordered_map_order_is_the_same_each_run`, `ordered_map_runs`.
  64. **`process_run` ignores its marker.** `tests/runtime/process_run.nova`:
      `if exists(marker).await {` becomes `if false {`.
      - Run `cargo build --locked --workspace` and `R process_run_runs`, each to its end.
      - Expected: fails: `process_run_runs`.
  65. **`run` drops the child's arguments (the runaway child).** `crates/nova-runtime/src/process.rs`:
      `command.args(args);`: the line goes.
      - Run `cargo build --locked --workspace` and `R process_run_runs`, each to its end.
      - Expected: fails: `process_run_runs`.
  66. **`status` does not flush standard output first.** `crates/nova-runtime/src/process.rs`:
      `let _ = std::io::stdout().flush();`: the line goes.
      - Run `cargo build --locked --workspace` and `R process_run_runs`, each to its end.
      - Expected: fails: `process_run_runs`.
  67. **`run` swaps standard output and error.** `crates/nova-runtime/src/process.rs`:
      `Ok((exit_code(out.status), out.stdout, out.stderr))` becomes `Ok((exit_code(out.status), out.stderr, out.stdout))`.
      - Run `cargo build --locked --workspace` and `R process_run_runs`, each to its end.
      - Expected: fails: `process_run_runs`.
  68. **`status` captures the child's output.** `std/process/lib.nova`:
      `let code = process_run(program, args, false)` becomes `let code = process_run(program, args, true)`.
      - Run `cargo build --locked --workspace` and `R process_run_runs`, each to its end.
      - Expected: fails: `process_run_runs`.
  69. **A death by signal is the signal's number.** `crates/nova-runtime/src/process.rs`:
      `return 128 + i64::from(signal);` becomes `return i64::from(signal);`.
      - Run `cargo build --locked --workspace`, `T process::tests::a_death_by_signal_is_128_plus_the_signal` and `R a_child_killed_by_a_signal_is_128_plus_the_signal`, each to its end.
      - Expected: fails: `a_child_killed_by_a_signal_is_128_plus_the_signal`, `process::tests::a_death_by_signal_is_128_plus_the_signal`.
  70. **`env` asks the system for any name.** `crates/nova-runtime/src/process.rs`:
      these lines go:

      ```
          if name.is_empty() || name.contains('=') || name.contains('\0') {
              return None;
          }
      ```

      - Run `cargo build --locked --workspace` and `R process_env_runs`, each to its end.
      - Expected: fails: `process_env_runs`.
  71. **`env` does not hand its value over.** `crates/nova-runtime/src/process.rs`:
      `stash(Slot::Buffer, crate::gc_str(&value));`: the line goes.
      - Run `cargo build --locked --workspace` and `R process_env_runs`, each to its end.
      - Expected: fails: `process_env_runs`.
  72. **`current_exe` names `nova` whatever runs.** `std/process/lib.nova`:
      replace

      ```
          Ok(fs_take_string())
      }
      ```

      with

      ```
          Ok("nova")
      }
      ```

      - Run `cargo build --locked --workspace` and `R current_exe_is_nova_under_run_and_the_program_when_built process_env_runs`, each to its end.
      - Expected: fails: `current_exe_is_nova_under_run_and_the_program_when_built`, `process_env_runs`.
  73. **`run` takes standard output twice.** `crates/nova-mir/src/lower.rs`:
      `Builtin::ProcessTakeStderr => Lowering::Runtime(RtFunc::ProcessTakeStderr),` becomes `Builtin::ProcessTakeStderr => Lowering::Runtime(RtFunc::ProcessTakeStdout),`.
      - Run `cargo build --locked --workspace` and `R process_run_runs`, each to its end.
      - Expected: fails: `process_run_runs`.
  74. **`env` is not exported.** `std/process/lib.nova`:
      `pub fn env(name: String) -> Option<String> {` becomes `fn env(name: String) -> Option<String> {`.
      - Run `cargo build --locked --workspace` and `W std_process_is_e0092_on_wasm`, each to its end.
      - Expected: fails: `std_process_is_e0092_on_wasm`.
  75. **The `--release` walk sets no variable.** `crates/nova-cli/tests/run_tests.rs`:
      `.envs(vars.iter().copied());` becomes `;`.
      - Run `cargo build --locked --workspace` and `G`, each to its end.
      - Expected: fails: `release_golden_suite`.
- [ ] **Step 10: The programs at each task** (decision 33). Every runtime
  program of the last tree, each `tests/runtime/*.nova` with a `.stdout`,
  run with `nova run` by nine builds: `target/debug/nova` built at each code
  commit, Task 1's to Task 8's, each checked out and built in turn and run
  as soon as it is built, so no copy of a binary takes the disk; and then
  Task 0's copy of the base's `nova`, beside a link to
  `target/debug/libnova_runtime.a`. Every program runs from one copy of the
  last tree under `$P`, with a fresh `NOVA_HOME` and temporary directory,
  its output and exit status kept, under the process limit. The tree ends at
  the last commit, built. Each task's runs must equal the last tree's, or be
  refused before running (an `error[` and no output); so must the base's,
  where it runs. Expected:
  `progcmp: 156 programs; base: 146 same, 10 refused, 0 differ; t1: 148 same, 8 refused, 0 differ; t2: 149 same, 7 refused, 0 differ; t3: 151 same, 5 refused, 0 differ; t4: 151 same, 5 refused, 0 differ; t5: 153 same, 3 refused, 0 differ; t6: 154 same, 2 refused, 0 differ; t7: 156 same, 0 refused, 0 differ; t8: 156 same, 0 refused, 0 differ;`.
  None differs. What each build refuses is the programs of the tasks after
  its own, each by an error before anything runs: the base all ten of
  5.0c's; Task 1's all but `array_helpers` and `hidden_trait_methods`; Task
  2's `string_builder`, `string_builder_million`, `parse_numbers`,
  `float_bits`, `ordered_map`, `process_run` and `process_env`; Tasks 3's
  and 4's the last five; Task 5's `ordered_map` and the two process
  programs; Task 6's the two process programs; Task 7's and Task 8's none.
  The last tree's own runs match 143 of the 156 `.stdout` files; the others
  take the arguments, input, files or variables their `run_tests.rs` test
  gives them (`process_run`, under `nova run`, starts `nova` itself as its
  child, which refuses `child` as a subcommand, and then panics on the empty
  output, the same under every build: its test builds it), and every build's
  run of them is compared all the same.
- [ ] **Step 11: The review's inputs.** The full run's figures, the new
  tests by binary, the golden suite's line and time, the AST, `check` and
  E0102 to E0105 counts, the programs at each task, and the mutants'
  outcome, with each gap, go in the ledger.
- [ ] **Step 12: One fresh read-only review, then one fix pass** (the user's
  workflow).
  - The reviewer reads `git diff $BASE..HEAD` (Task 0) against the spec, ADR
    0050 and this plan, and is told to hunt for:
    - a process path that can stall on a full pipe, lose output, read an
      argument wrongly, panic, map an error to the wrong kind, report a
      signal as anything but `128 + signal`, run a shell, or leave a payload
      for the next run; a test or program that could run its parent again
      without the guard (decisions 2, 8, 14, 21, 22 and 25);
    - `parse_int` or `parse_int_radix` accepting text the docs refuse, or
      wrapping instead of refusing; `parse_float` disagreeing with Rust's
      grammar or value on either target (decisions 6, 13 and 20);
    - byte access copying where it should not, reading past a bound, or
      slicing inside a character; a `Char` made from a value that is not a
      scalar value (decisions 5, 7 and 11);
    - `OrderedMap` losing or reordering an entry, a stale position after a
      compaction, an iterator that skips or repeats an entry under `remove`
      (decisions 3 and 16);
    - an array method looked up before `len`, an inherent impl on arrays
      refused or a trait impl on one accepted, `at` or `last` reading past
      `len`, a copy that shares, a resolver's name such as `Arr_T` in a
      diagnostic (decisions 10, 15, 17, 18, 23, 24 and 42);
    - E0105 missing where std's inherent method hides a program's trait
      method, or firing where it does not, a pair of which only one takes
      `self` among them; its advice not reaching the trait's method; a
      program's own inherent method of a new std name not E0074 (decisions
      40, 41 and 45);
    - `to_bits` other than Rust's on any backend for the same value
      (decisions 9, 13 and 44);
    - the builder counting other than bytes, or `to_string` changing it
      (decisions 4 and 12);
    - E0092, E0074's labels or the public-form note on the wrong case or in
      the wrong words (decisions 2, 18 and 19);
    - a commit between Task 1 and Task 8 that builds wrong code or runs a
      program other than as the last tree does (decisions 29 to 33);
    - the records: ADR 0050, the CHANGELOG, the dated notes;
    - each mutant of Step 9 that passes, and whether a check is added;
    - MSRV 1.78, a new crate, a change to `Cargo.lock`, a test that touches
      the internet.
  - Each finding is fixed in one commit, `5.0c: the review's fixes`, or
    answered in the ledger with the reason it stands.
  - After the fixes, Steps 1 and 2 run again.
- [ ] **Step 13: The PR.**
  - Push: `git push -u origin claude/loving-archimedes-frtf5q-50c`.
  - Open the PR to `main` with the repository's template headings, if it has
    one. The body names no local path and no person.
  - It holds:
    - what 5.0c adds: `StringBuilder`; `to_int`, `char_from_int`,
      `string_from_chars` and byte access; `parse_int`, `parse_int_radix`
      and `parse_float`; `Float.to_bits`; methods on arrays and `Vec`'s
      helpers; `OrderedMap`; `std/process`'s `run`, `status`, `env` and
      `current_exe`; and ADR 0050;
    - what a program can meet that it did not: an inherent impl on arrays is
      accepted, a trait impl on one still E0010, and one defining `iter` or
      `slice` E0074 (decision 17); std's new inherent methods run in place
      of a program's trait methods of their names, with E0105, and a
      program's own inherent method of such a name is E0074 (decisions 40
      and 41); E0074 labels both impls; a std-only builtin's name gets its
      public form as a note; an array method is `[T]::slice` in a
      diagnostic; `run`, `status`, `env` and `current_exe` are global names
      a program's own shadows; every repository `.nova` file outside std
      parses and checks as at the base;
    - the runaway child and its guard, and the process limit the tests and
      mutants ran under (decisions 22 and 34);
    - the figures: the suite's `2788 1 10`, the golden suite's
      `147 passed, 9 excluded`, and that `Cargo.lock` is unchanged;
    - the user's decisions, and this plan's decisions, numbered as here,
      with 17 to 20, 24 and 40 to 42 marked as differing from the spec;
    - the mutants and the gaps they found, and the review's findings and
      what came of each.
  - Read CI once it ends. Expected: green on all three systems; on the
    `test` job's `ubuntu-24.04` leg, `cargo test`'s log holds
    `test release_golden_suite ... ok`. Record the run's address in the
    ledger.
  - **If the suite fails on CI and not here,** record its failures, stop and
    ask the user; a Windows failure of a process test is the likeliest
    (decision 27).
- [ ] **Step 14: The merge,** only on the user's word.
  - By rebase, pinned to the PR's head commit. Never auto-merge.
  - Then `git fetch origin main` and `git diff <head> origin/main --stat`,
    which prints nothing: the merged tree is the PR's.
  - Then read CI's run on `main` once it ends: green, as Step 13's.

