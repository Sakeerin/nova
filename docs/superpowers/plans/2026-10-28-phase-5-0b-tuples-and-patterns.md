# Phase 5.0b, "Tuples and patterns", Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use
superpowers:subagent-driven-development (recommended) or
superpowers:executing-plans to implement this plan task-by-task. Steps use
checkbox (`- [ ]`) syntax for tracking.

**Goal:** The second of Phase 5.0's three parts: tuples and the patterns a
compiler written in Nova needs. Tuple types `(Int, String)`, literals
`(a, b)` and `(a,)`, and `t.0`, which the lexer makes possible by splitting
`1.0` after a `.`; tuples are anonymous records with no impls, laid out as
records are, on every backend. Patterns nest, to any depth, with guards,
or-patterns, `Char` literals and ranges; `let` and `for` destructure an
irrefutable pattern, E0022 decided by usefulness, which now reads nested,
tuple, or-, literal and range patterns and names nested witnesses. A `match`
with any such arm lowers to MIR by test chains, while a flat one keeps
today's switch, byte for byte. `for x in arr` runs over an array. Three
codes are new: the errors E0100 and E0101, and the warning E0104 for a
pattern binding that reads as a variant. A bare variant name inside a
variant pattern matches that variant, Rust's rule, the user's ruling on the
first fact-check. E0103 warns at a newline-rule split only where the old
reading could have been well typed, the user's ruling for 5.0b. ADR 0049
records the departures.

**Architecture:**
- **`crates/nova-lexer/src/{lib,tokens}.rs`:** `split_tuple_indexes` after a
  `.`; `Tokens::plain_decimal`. **`crates/nova-parser/src/grammar.rs`:** an
  integer after `.` is `Expr::Field`, P0001 unless plain decimal.
- **`crates/nova-hir/src/lib.rs`:** `Ty::Tuple` and its walks,
  `ExprKind::MakeTuple`, `TUPLE_HAS_NO_IMPLS`; the recursive `Pattern`,
  `bindings()`, `is_flat()`, and `Arm.guard`.
- **`crates/nova-typeck/src/check.rs`:** tuple types, literals and `t.N`;
  `check_pattern` recursing; guards; destructuring `let`; `to_useful_pat`
  and the witnesses over the new forms; `judge_split` and `split_operand`.
  **`check/patterns.rs`** (new): tuple, or- and range patterns, E0100,
  E0101, E0104, `require_irrefutable`, `for_loop_binding`,
  `check_or_alternatives`, `check_for_array`. **`check/fixes.rs`:**
  `Binding::Pattern` and E0060's note for it. **`usefulness.rs`:**
  `Ctor::{Char, Range, Tuple}`, `Pat::Or`, and witnesses that name values
  for E0022. **`infer.rs`, `lib.rs`:** tuples in unification and display.
- **`crates/nova-mir/src/{lib,lower,mono}.rs`:** a tuple is a pointer,
  `MakeRecord`, mangled `T…E`; `lower_chain_arms` and `lower_pattern_test`,
  and from Task 3 to Task 5 the refusal they replace; mono maps guards.
  **`crates/nova-resolver`:** a tuple as an impl's type argument's name.
- **Tests:** `nova-lexer`'s, `nova-parser`'s new `tuples.rs`, fmt's new
  `tuples.rs` and `patterns.rs`, `check/tuple_tests.rs` and
  `check/pattern_tests.rs` (new), `lang_tests.rs`, `usefulness.rs`'s,
  `nova-mir`'s `lower_tests.rs` and new `pattern_lowering.rs`,
  `ir_tests.rs`, the language server's, and six runtime programs: `tuples`,
  `tuples_async`, `patterns`, `patterns_async`, `match_effects` and
  `for_arrays`, under `run`, `build`, GC stress, WASM and the golden suite.
- **Records:** ADR 0049; `CHANGELOG.md`; dated notes in five files of
  `nova-spec/`.

**Tech Stack:**
- Rust: MSRV 1.78, edition 2021.
- **No new crate; `Cargo.lock` does not change** (spec §7.1).
- `clang`: Ubuntu's `clang-18`, 18.1.3, as 5.0a pinned it, for the golden
  suite; Node 22 for the WASM gate.

**Spec:**
`docs/superpowers/specs/2026-10-26-phase-5-0-the-language-and-std-a-compiler-needs-design.md`,
approved by the user on 2026-10-26.
- Read its §4, §6, §7, §10 to §13 before Task 1: it is the authority this
  plan argues from.
- Its header holds the user's decisions; the user's 5.0b ruling on E0103, of
  2026-10-27, is decision 3, and the user's ruling on the first fact-check,
  of 2026-10-28, decisions 18 and 42; ADR 0049 records both. Its §12 holds
  the decisions made while writing it, and its §13 what this plan settles.

**Checked on 2026-10-28, before execution:**
- **A prototype built 5.0b whole** on a local branch from `a3a95a2e` (`main`
  after 5.0a), in this tree: `73a2d684` and `2af45f30` (tuples), `c69cc91d`
  (patterns, the chains, `for` over arrays, the E0103 ruling, ADR 0049 and
  the records), then `41f011b4`, step 0's fixes (below), `8da2b8c9`, the
  tests the mutants' first run asked for (below), `b5a18c1d`, the first
  fact-check's fixes, and `ebd0ffad`, the second check's (both below). It
  settled spec §13's questions (decisions 12 to 16), and broke each fix and
  saw its tests fail: the lexer's split (3 tests), `plain_decimal` (the
  refusal test), the mangling made one constant or without its `E` (mono's
  test and the instances test), the tuple arm in `unify` (5 tests), E0011's
  error binding, the no-impls note, `mir_ty`, HIR's `subst`; the guard's
  false edge, an alternative falling through, the tag test of a one-variant
  sum, the exclusive range's end, shared or-locals, E0101, the or-expansion
  (10 tests), the tuple constructor of an unknown type, the guard keeping a
  match off the switch, the `Char` key, the irrefutability check, the
  alternative's reachability, the recursion into fields (46 tests), `for`
  over arrays, and the ruling's quiet splits and operand. Its full suite
  gave `2682 1 10` at `c69cc91d` and `2691 1 10` at `8da2b8c9`, its golden
  suite `137 passed, 9 excluded`, and its WASM gate 119 of 119, before the
  fact-checks' fixes.
- **Step 0, before the split,** `41f011b4 5.0b prototype: before the split`:
  - the three comments that still called nested patterns and `for x in arr`
    E0900 now say they work (decision 39);
  - a bare variant name inside a variant pattern: the E0001 for another
    type's variant gained its note, three tests pin the match, the note and
    that `Some(x)` still binds, and the CHANGELOG's Changed and ADR 0049
    record the change (decision 18);
  - ADR 0049 records each departure and each point settled: ranges not
    split, the unreachable alternative, no flat outer switch, the
    one-variant `let` through the switch, `(x, x)` unchanged, the E0103
    ruling, no `-` in a pattern, `let mut P`;
  - fmt and the language server probed on every new form: fmt's output was
    idempotent and reparsed to the same tree on all twelve probe files, and
    its self-check refused none; hover and completion found every binding in
    nested, or- and `for` patterns; two language-server tests pin the last
    (decision 40);
  - the newline rule probed against tuples and patterns: no silent change of
    meaning (decision 29).
- **The first fact-check, and its fixes,**
  `b5a18c1d 5.0b prototype: the first fact-check's fixes`, each code fix
  with a test and a mutant that breaks it (Task 8, Step 9):
  - E0103 was silent after a compound assignment to an element or a field
    and after a closure: `split_operand` reads through both (decision 3);
  - a bare variant name inside a variant pattern: the user ruled for Rust's
    rule, with the new warning E0104 for an upper-case binding; `Some(Leaf)`
    for a variant with fields is E0016; the CHANGELOG, ADR 0049 and the PR
    say a program can run differently, and that a catch-all hides it from
    exhaustiveness (decisions 18 and 42);
  - Tasks 3 and 4 left commits that would lower a nested or guarded match
    wrongly: lowering refuses one until Task 5's code (decision 35);
  - decision 26 is marked as differing; `13-RUNTIME.md` says the collector
    is conservative; E0022's catalog row covers `for`, E0001's note is in
    `12-TYPESYSTEM.md`, and the bound note names `Debug` (decision 46);
  - E0001's pattern no longer cascades an E0021 (decision 43); E0022's
    witness names a value, `(_, 0)` (decision 44); E0060's note on a
    pattern's binding says how to make it mutable (decision 24); and
    usefulness's cost in or-patterns is measured and recorded (decision 45);
  - Task 7's comparison with the prototype leaves this plan's file out; Task
    8's probe paths are spelled out; the `unwrap()` constraint lists its
    test files.
- **The second check, and its fixes,**
  `ebd0ffad 5.0b prototype: the second check's fixes`, each code fix with a
  test and a mutant:
  - a flat `Char` match built at Tasks 3 and 4 took its first arm for every
    character, the switch's `Char` key riding with Task 5: the key moves to
    Task 3, with `lower_tests.rs`'s test of it (decision 26), and Task 8,
    Step 10 runs every program of the last tree, and a `Char` probe, at
    Tasks 3 and 4 (decision 47);
  - the mutants left `target/debug/nova` a mutant's build: Step 9 ends by
    building the last commit (decision 48);
  - E0104 at a constant's name says to use a guard, and near a variant says
    "did you mean"; tests pin E0104 in a `for` pattern and once in an
    or-pattern (decision 42), and the skip of E0022 after a pattern error in
    `let` and `for` (decision 43);
  - E0060's note follows where the name is bound (decision 24);
  - E0022 names each missed value once, `(0, _)` and `(1, false)` for
    `for (1, true) in p` (decision 44);
  - `12-TYPESYSTEM.md`'s E0104 row and note exempt a plain `let X` or
    `for X`, and two long lines are rewrapped; this list's `2691`.
- **A replay split the prototype into this plan's tasks,** on the local
  branch `split/5-0b` from `a3a95a2e`, by a script, and ran each task's
  steps as written:
  - thirteen commits: a tests commit and a code commit for each of Tasks 1
    to 6, then Task 7, each the patch of its task below. The last commit's
    tree is the prototype head's (`git diff --stat ebd0ffad split/5-0b`
    prints nothing). Each patch applies with plain `git apply`, in order, on
    `a3a95a2e`, with no warning;
  - the split departs from the natural one in four places: usefulness rides
    with the patterns' checker (decision 33); `for` over arrays comes before
    the chains, and `for_arrays` with the chains (decision 34); the language
    server's `for` tests ride with Task 4 (decision 36); and Task 3 adds a
    refusal in lowering that Task 5 removes, and the flat switch's `Char`
    key, so no commit between lowers a match wrongly (decisions 35 and 26).
    A check of a first split found the first, the replay of two later splits
    the next two, and the two fact-checks the last;
  - each tests commit failed as its step says before its code, some by not
    compiling (decision 38), and passed after it; the whole suite, but the
    IPv6 test, `cargo fmt`, clippy with `-D warnings` and `nova fmt --check`
    passed after every task;
  - every "Expected:" below is the replay's, from a fresh `NOVA_HOME`, but
    Task 0's suite figures, which are 5.0a's last full run on `a3a95a2e`.
- **The final checks ran on the split's last commit:**
  - the full suite gave `2705 1 10` (4 min 44 s): the base's `2534 1 10` and
    171 tests more, which a comparison by binary finds as 171 added and 0
    gone (Task 8, Step 1); the one failure is the IPv6 test;
  - CI's Linux leg, the whole suite under
    `NOVA_REQUIRE_CLANG=1 NOVA_CLANG=clang-18 NOVA_REQUIRE_NODE=1 … --all-features`,
    run once as CI runs it: `2705 1 10`, the golden suite among them (Task
    8, Step 2);
  - the golden suite, in 30 s of wall time:
    `release golden: 137 passed, 9 excluded, Ubuntu clang version 18.1.3 (1ubuntu1)`;
  - the AST of every `.nova` file `a3a95a2e` tracks: 279 files, 0 differ;
    the three whose comments step 0 corrected differ in their spans alone,
    and are the same with the spans stripped;
  - `nova check` over the same files: 245 checks, 2 differ,
    `crates/nova-parser/tests/fixtures/generics.nova` and
    `crates/nova-parser/tests/fixtures/match.nova`, parser fixtures that use
    5.0b's forms (Task 8, Step 5);
  - E0102, E0103 and E0104: E0102 53 times, as at 5.0a (the parser fixture
    `sum_type.nova`, and std's sources checked by hand as user files), and
    E0103 and E0104 nowhere (251 checks);
  - the search for a bare upper-case name inside a variant pattern: 0 in
    1427 match arms;
  - fmt on step 0's and the fact-check's probes:
    `fmtprobe: 25 files, 25 ok, 0 with a parse error, 0 bad`: step 0's
    twelve and the fact-check's thirteen, all but two reformatted, each
    formatting again to the same text and parsing to the same tree, the
    newline rule's splits of four files included;
  - every runtime program at Tasks 3 and 4: 146 programs and the probe; at
    Task 3 142 run as the last tree does and Task 5's 4 are refused, at Task
    4 the same, none differs; the `Char` probe prints `A B Z other` at both;
  - Rust 1.78's check was clean, and `Cargo.lock` did not change.
- **The mutants.** 76 breakages of what 5.0b adds, each a text replaced in
  one file, run one at a time at the split's last commit, three at Task 3's
  (Task 8, Step 9): the lexer's split and `t.N`'s refusals (5), tuple
  typing, E0011, E0014 and the E0013 note (6), the mangling (2), each
  pattern form's checking and E0100/E0101 (11), usefulness, witnesses, the
  unreachable alternative and E0022 (10), the chains (7), `for`'s desugar
  (3), each arm of the E0103 ruling (12), the first fact-check's E0104,
  cascade, witness and note (5), the second check's E0104 notes, E0022
  skips, E0060 notes and witnesses (8), Task 3's refusal and `Char` key (3)
  and fmt's new forms (4), step 0's note among the patterns'. The first run,
  on the split of `41f011b4`, caught 50, left three passing (mutant 13,
  mono's name of a tuple of one; 48, a value of unknown type judged as
  known; and one of `split_operand`'s arm for a `let`, which was dead), and
  broke the build on four whose edits were written wrong, each rewritten
  since; `8da2b8c9` closed the three (decision 41). The second, on its
  split, caught all 57. The first fact-check added ten, for its fixes: the
  compound assignment's and the closure's operands and the wrapper left out,
  E0104 and its first letter, the cascade, the witness's value, E0060's
  note, and Task 3's refusal twice; its run caught all 67. The second check
  added nine: E0104's constant note and its guess, the two E0022 skips, the
  `let` pattern's and the loop variable's notes, the two witness rules, and
  the `Char` key at Task 3's commit. This run, on this split, caught 76 of
  76, in 49 min 44 s, and passed none.
- **What the checks left standing,** each recorded or handed on:
  - no `-` in a pattern: a negative literal or range end is P0001, before
    5.0b and after, and the P0001 at a `-1..=5` arm carries 5.0a's note
    about an arm the rule began, which names the rule rather than the
    missing form (decision 29); a later part can add `-` to `range_pattern`;
  - `(x, x)` binds twice with no error, as before 5.0b (decision 19);
  - `t.1(x)` is a method call, as a record's field is (decision 21);
  - `split_operand` approximates the old parser's precedence: it reads the
    rightmost operand of any binary operator, where after `a * b` the old
    reading's `-` took `a * b` whole; Nova's arithmetic operators take and
    give one type, so the two agree on the type the ruling asks about
    (decision 3);
  - a chain tests a sum's tag once per arm that names it (decision 16), and
    E0020 shows one witness per uncovered constructor, with `_` where E0022
    names a value (decision 44);
  - usefulness is exponential in the or-patterns of a row: sixteen
    `true | false` in one tuple pattern take 14 s in a debug build (decision
    45);
  - a bare variant name inside a variant pattern runs differently from
    before 5.0b, and behind a catch-all no check says so; E0104 catches the
    misspelt or unimported name, and no repository program writes either
    (decisions 18 and 42);
  - Rust 1.78 checks the libraries and binaries, not the tests: `assert_cmd`
    2.2.1's manifest does not parse on 1.78 (Task 8, Step 8).

## Global Constraints

- **The workspace's minimum Rust stays 1.78,** edition 2021. CI's MSRV job
  runs `cargo check --locked --workspace` with `RUSTFLAGS=-D warnings`. No
  `std::sync::LazyLock` or `LazyCell` (1.80), no `Option::is_none_or`
  (1.82), no `std::path::absolute` (1.79). 5.0b's Rust uses
  `str::split_once` (1.52), `let … else` (1.65) and `Option::is_some_and`
  (1.70), nothing newer.
- **No new crate, and `Cargo.lock` does not change** (spec §7.1). Any change
  to the lock stops the task: ask the user.
- **No `unwrap()` outside `#[cfg(test)]` and test files:** `expect("why")`,
  `unwrap_or…` or `?`. 5.0b's `unwrap()`s are only in test files and test
  modules: the `#[cfg(test)]` modules of `crates/nova-lsp/src/hover.rs` and
  `completion.rs`, `crates/nova-typeck/src/check/pattern_tests.rs` and
  `crates/nova-codegen-llvm/tests/ir_tests.rs`.
- **Tests never touch the internet.** `clang` is installed by CI's job,
  never by a test or by `nova` (spec §7.1). No test uses a port but those
  the existing tests bind.
- **Temporary names are fixed** (spec §7.1): the language server's new tests
  name their files under `std::env::temp_dir()`, in `nova-lsp-hover-tuple`,
  `nova-lsp-hover-pattern`, `nova-lsp-hover-for-pattern`,
  `nova-lsp-complete-pattern` and `nova-lsp-complete-for-pattern`, as an
  overlay that is never written; the golden suite builds into
  `nova-release-golden/`, as in 5.0a. A test that needs `$NOVA_HOME` sets a
  fresh one.
- **Never clean `/tmp/nova-*`:** `nova test` keeps built binaries in
  `/tmp/nova-test-bin`, a test whose binary goes fails, and the golden
  suite's directory is one of them.
- **A test that needs Node** passes without running when `node` is not on
  `PATH`, and fails instead under `NOVA_REQUIRE_NODE=1`, which CI sets. Run
  the WASM tests here with `NOVA_REQUIRE_NODE=1`: Node 22 is installed.
  5.0b's six programs join the WASM gate.
- **`release_golden_suite` runs only under `NOVA_REQUIRE_CLANG=1`,** which
  CI's Linux leg sets with `NOVA_CLANG=clang-18`; elsewhere it prints
  `release golden: not run (set NOVA_REQUIRE_CLANG=1 to run it)` and passes.
  Run it here with both set: this machine's `clang-18` is Ubuntu's 18.1.3,
  CI's pin. It walks every `tests/runtime/*.nova` with a `.stdout`, so each
  of 5.0b's programs runs through LLVM too.
- **`nova-cli`'s tests need two files beside `target/debug/nova`:**
  `libnova_runtime.a` and the `nova-bench-http` binary. A build of the
  workspace writes both; Task 0's build puts them there. Every run step
  starts with `cargo build --locked --workspace` (Conventions), so the
  `nova` the steps run by hand is the tree's.
- **Work in one tree.** Two worktrees of this repository sharing one
  `target/` share artifact names, so cargo can take one tree's build as
  fresh for the other (4.0's ledger). Never build in another worktree.
- **Keep `df -h /` above 6G.** `target/debug/incremental` and older
  duplicate test executables in `target/debug/deps` may be removed, between
  commands, never while one runs: cargo may be about to run the older one,
  and then fails with "No such file or directory" (the replay met this
  once). Every `cargo` command here selects the whole workspace
  (Conventions), so cargo keeps one copy of each test executable. Task 8's
  Rust 1.78 check uses its own target directory, in `/dev/shm`, removed
  after.
- **Never use the Write or Edit tool for text that holds a backslash-u
  escape,** a backslash and `u` followed by four hex digits or by a code in
  braces: they decode the escape into the character. None of 5.0b's patches
  holds one; apply them with `patch_block`, from the committed plan, and
  write anything that holds one with a script.
- **Nothing writes into the repository but the patches.** The golden suite
  builds under the system's temporary directory; Task 8's AST and `check`
  comparisons write their dumps under `$P`.
- **Merge by rebase, on the user's word only.** Never enable auto-merge.
  Nothing is published and no tag is made in 5.0b.
- **This machine is Linux.** macOS and Windows results come from CI, where
  `release_golden_suite` skips.

## Conventions

- **Scratch directory:** `$P` is the session's scratch directory, outside
  the repository. Set it in every shell before a command that names `$P`.
- **The ledger:** `$P/ledger-50b.md`. Every ruling made while executing goes
  there, with its cost if wrong; so do each task's figures.
- **Commits.** Each task's message is written to `$P/msg-50b-<task>.txt`:
  the subject the task names, a blank line, and the session's attribution
  lines. Then `git add -A`; `git status --short` lists only the task's
  files; and `git commit -F "$P/msg-50b-<task>.txt"`. A task with a tests
  patch and a code patch makes one commit, after both.
- **Runs.** The replay ran every command with these variables set. Each
  shell sets them too, with six shorthands, and runs every command from the
  repository's root:

  ```sh
  export CARGO_INCREMENTAL=0 NO_COLOR=1 RUST_BACKTRACE=0 NOVA_HOME="$P/home-50b"
  unset NOVA_RUNTIME_LIB NOVA_REQUIRE_NODE NOVA_REQUIRE_CLANG NOVA_CLANG
  mkdir -p "$NOVA_HOME"
  W() { NOVA_REQUIRE_NODE=1 cargo test --locked --workspace --test wasm_tests -- "$@"; }
  R() { cargo test --locked --workspace --test run_tests -- "$@"; }
  X() { cargo test --locked --workspace --test index_complete "$@"; }
  T() { cargo test --locked --workspace --no-fail-fast --lib -- "$@"; }
  S() { cargo test --locked --workspace --no-fail-fast "$@"; }
  G() { NOVA_REQUIRE_CLANG=1 NOVA_CLANG=clang-18 cargo test --locked --workspace --test run_tests -- release_golden_suite --nocapture "$@"; }
  ```

  So `W gate::tuples` runs that WASM gate test, `R tuples_run` that test
  of `run_tests` (under `run`, `build` and, where its program asks, GC
  stress), `X` the corpus test, `T check::pattern_tests` the tests of every
  crate's library whose path holds that filter, here the checker's tests
  of 5.0b's patterns, `S` the whole suite, and `G` the golden suite
  through LLVM as CI's Linux leg runs it. `NOVA_HOME` keeps a run by hand
  off `~/.nova`; every test that needs one sets its own.
- **Every `cargo` command selects the whole workspace,** `--workspace` with
  `--test NAME` or `--lib` to narrow it, never `-p`. Cargo unifies features
  over the packages a command selects, so each `-p` selection builds and
  links its own copy of every test executable it needs, about 1.3 GB a
  selection here, which the disk does not have (Global Constraints); a
  `--workspace` command reuses one copy. So `--test tuples` runs both
  `nova-parser`'s and `nova-fmt`'s `tuples.rs`, each its own `test result`
  line, and `T tuple` the library tests of every crate whose names hold
  `tuple`; such a command takes `--no-fail-fast`, as `T` does, so that a
  binary that fails does not stop the ones after it. `S` gives each code
  step the whole suite, about five minutes; its one failure is the IPv6 test
  (Task 0).
- **The build.** Each "Run" step below starts with

  ```sh
  cargo build --locked --workspace
  ```

  which ends with `Finished`, in about 10 seconds after a change to the
  front end and a few seconds otherwise, so that the `nova` the steps run
  by hand, and the runtime library beside it, are the tree's (Global
  Constraints). The replay began every step with it.
- **The patches.** Each task's code is in patch blocks below, each marked
  `<!-- patch NAME -->` and fenced with four backticks. They are the
  replay's commits, so apply them; do not retype them. From the repository's
  root:

  ```sh
  PLAN=docs/superpowers/plans/2026-10-28-phase-5-0b-tuples-and-patterns.md   # this file, committed
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
  nothing: no 5.0b patch adds a line with trailing whitespace, a CR or a
  backslash-u escape, so `git apply` warns for none. The only non-ASCII
  characters the patches add are `§`, `…`, `—` and the `─` of a heading
  comment in `lexer_tests.rs`. A block that does not apply means the tree
  differs from the plan's: stop, and record why in the ledger.
- **Expected outputs are written once, from a run, and read.** Each
  `.stdout` under `tests/runtime/` that 5.0b adds is in its task's tests
  patch: the prototype wrote them from the program's native run; the replay
  ran each under `nova run`, `nova build`, `--release` through `clang-18`,
  and WASM, and five of the six under GC stress (`match_effects_run` runs
  its program without it).
- **Each task** but Task 7 is test first: its tests' patch, their run and
  the expected failure, then its code's patch, its run and the expected
  pass. Task 7's patch is the records. Each task then runs its checks:
  - `cargo fmt --all -- --check`, which prints nothing;
  - the workspace's clippy,
    `cargo clippy --locked --all-targets --all-features -- -D warnings`, as
    CI runs it, which ends with `Finished` and no warning;
  - `target/debug/nova fmt --check std examples tests/wasm tests/dom tests/browser/router-app tests/browser/ssr-app tests/ssr`,
    with the `nova` the step's build made, which prints nothing. 5.0b's
    programs are under `tests/runtime`, which is not among them, as no
    program there is; the fmt corpus test formats every one of them (Task
    8).

  It ends with one commit. A difference from an `Expected:` is fixed in
  the smallest way and recorded in the ledger as a ruling, with its cost if
  wrong.

## Review Focus

The inputs most likely to bite a user, most likely first. Each has its check
in the task that owns the code:

1. **The test chains** (`lower_chain_arms`, `lower_pattern_test`): a match
   takes the chains when any arm has a guard or a pattern `is_flat` refuses,
   and the flat switch, byte for byte as before, otherwise; the scrutinee
   evaluated once, into the temporary the switch used; each arm's tests left
   to right and depth first, each sub-value projected once, just before its
   own tests, `SumField` or `RecordField`; no tag test for a sum of one
   variant; a range as `>=` then `<=`, both ends included; an or-pattern's
   alternatives testing the same value in turn and jumping to one block, so
   nothing after it is duplicated and every alternative writes the same
   locals; bindings as copies written as the chain reaches them; the guard
   after the tests, its false edge to the next arm; a trap after the last
   arm; a `Char` literal as a key of the flat switch; and, from Task 3 to
   Task 5's code, the refusal of any match the chains will take (decisions
   11, 13, 16, 26 and 35) → `pattern_lowering.rs`'s ten tests, the IR test
   that flat matches still `switch`, and `patterns`, `patterns_async` and
   `match_effects` under four backends and GC stress (Task 5), and
   `lower_tests.rs`'s refusal test (Tasks 3 and 4); mutants 35 to 41, and 70
   and 71 at Task 3's commit. A chain that jumps to the wrong block is a
   miscompile no checker catches: the reviewer reads each `terminate`
   against the spec's §4.6.
2. **Exhaustiveness and reachability over the new forms** (`usefulness.rs`,
   `to_useful_pat`, `check_or_alternatives`, `require_irrefutable`):
   or-patterns expanded, as rows and as the query; `Ctor::Tuple` the one
   constructor of a tuple type, also when the column's type is not yet
   known; `Char` literals and ranges each their own constructor, never
   split, which can only make a row more useful, so no false "unreachable"
   and no false "exhaustive"; nested witnesses (`Some(Err(_))`,
   `(_, false)`, `(1,)`); an unreachable alternative (E0021) only in a
   reachable, unguarded arm, against the earlier arms and the earlier
   alternatives of its own arm, and not again inside one already reported;
   E0022 for a refutable `let` or `for` pattern, with the values it misses,
   a missing literal named by a value (`(_, 0)`); no usefulness for a match
   with a pattern error, so no E0021 cascade (decisions 8, 15, 23, 28, 43
   and 44) → `usefulness.rs`'s eight new tests and `pattern_tests.rs`'s
   E0020, E0021 and E0022 tests (Task 3); mutants 25 to 34, and 59 and 60.
3. **Checking each pattern form** (`check_pattern`, `patterns.rs`): a
   variant's fields and a tuple's elements checked to any depth; a bare name
   that names a variant matching it at any depth, Rust's rule, E0001 with
   its note when it is another type's and E0016 when the variant has fields
   (5.0b's one change of meaning, decision 18); E0104 for a binding whose
   name starts with an upper-case letter, and not for `_Upper` (decision
   42); `Char` literals; ranges over `Int` and `Char` alone, E0101 for an
   empty one, `lo..hi` stored as `lo..=hi - 1`, a `Char` by its code point;
   every alternative binding the first one's names at their types (E0100,
   naming the binding); a guard a `Bool` that sees the arm's bindings;
   `let mut P` making each binding mutable, and E0060's note saying so; a
   destructuring `let` as a one-arm `match` (decisions 2, 9, 18, 22, 24, 25
   and 42) → `pattern_tests.rs` (Task 3); mutants 14 to 24, and 57, 58 and
   61. A bare name read as a binding where it names a variant, or as a
   variant where it names none, changes what a program runs with no error:
   the reviewer reads `check_pattern`'s `Bind` arm.
4. **The lexer's split and `t.N`:** a float after a `.` alone, split by its
   text, so `t.1.10` is `(t.1).10` and `0..1.5` is unchanged; `t.01`,
   `t.1_0`, `t.0x1` and `t.00` P0001 with "write `.1`"; `.N` on a value of
   unknown type E0011 once, the variable bound to the error type; an index
   past the end E0014, "no field `2` on `(Int, Int)`" (decisions 6, 14 and
   20) → `lexer_tests.rs`'s seven tests, `nova-parser`'s `tuples.rs` and
   fmt's `tuples.rs` (Task 1), `tuple_tests.rs` (Task 2); mutants 1 to 5, 8
   and 9.
5. **`Ty::Tuple` and its walks:** every walk over `Ty` in `nova-hir`,
   `nova-typeck`, the resolver and MIR reaches inside a tuple; unification
   checks the arity; the mangling `T<elements>E` gives distinct tuple types
   distinct instances, the shape of two earlier miscompiles (spec §10 risk
   3); no impl names a tuple, so `==`, interpolation and `Map` keys are
   E0013 with the note; a tuple lowers as a record, `MakeRecord`,
   `RecordField` and `SetField`, a pointer in every backend (decisions 1, 5
   and 12) → `tuple_tests.rs`, the HIR and infer tests, mono's mangling
   test, `lower_tests.rs`, and `tuples` and `tuples_async` under four
   backends and GC stress (Task 2); mutants 6, 7, 10, 11, 12 and 13.
6. **`for x in arr`:** the array evaluated once into `__arr`, its length
   read once, the index advanced before the body, so `continue` is safe,
   each element read when the loop reaches it, so a write to a later element
   is seen; the binding immutable; a pattern by `let`'s rule (decisions 7
   and 27) → `pattern_tests.rs`'s four `for` tests and `for_arrays` (Tasks 4
   and 5); mutants 42 to 44.
7. **E0103's ruling:** before `(` only after a function, before `[` only
   after an array, before `-` only after an `Int` or a `Float`, and wherever
   the type is not yet known; after a bare `return` or `break` always; the
   operand the old reading would have extended, read through binary
   operators, a `return`'s value and an assignment's, a compound
   assignment's desugared block to the value it writes, and a closure's
   body, a `let`'s value judged on its own; a quiet split reported nowhere,
   merged nowhere (decisions 3 and 41) → `lang_tests.rs`'s nine changed and
   new tests (Task 6), every repository file's E0103 count (Task 8); mutants
   45 to 56. A split judged quiet where the old reading was well typed is a
   silent change of meaning: the reviewer reads `split_operand` against the
   parser's precedence.
8. **fmt and the language server:** no code changed in either; fmt's tuple,
   pattern and guard layouts, idempotent and reparsing to the same tree, its
   self-check never refusing a file it formatted (decision 30) → fmt's
   `tuples.rs` and `patterns.rs`, the corpus test, Task 8's probes; hover
   and completion on bindings in nested, or- and `for` patterns → the
   language server's five tests (Tasks 2, 3 and 4); mutants 73 to 76.
9. **The records:** ADR 0049 against the spec's §4 and the user's decisions
   and rulings; the CHANGELOG's Added and Changed; the dated notes of `11`,
   `12` (and its catalog's E0100, E0101 and E0104), `13`, `14` and `20` →
   Task 7, Step 2.

## Decisions

Made while writing the spec (its header and §12), prototyping, splitting,
replaying and answering the first fact-check, each as built and each read
against the code. Where one departs from the spec, it says so and why, and
is marked **(differs)**; one that fills in what the spec left to the plan
(its §13) says so, unmarked. ADR 0049 records 5.0b's departures and the
points it settles (spec §12.1).

**The user's decisions, and the user's ruling, as built**

1. **Tuples have no impls, and reach every backend, WASM included** (the
   user's decision 3, spec §4.1). A tuple is an anonymous record:
   `Ty::Tuple` has no `TyHead`, so no impl names one
   (`impl Show for (Int, Int)` and `impl (Int, Bool)` are E0010,
   `no_impl_can_name_a_tuple`), and `==`, interpolation, a `Hash`, `Eq` or
   `Display` bound and a `Map` key are E0013 with the note "a tuple has no
   impls, so it has no `==`, `Hash`, `Display`, `Debug` or any other trait:
   compare or print its elements, or use a record that implements the trait"
   (mutant 10). It lowers as a record, `MakeRecord`, `RecordField` and
   `SetField`, so no backend changed, and `tuples` and `tuples_async` run on
   all four, under GC stress too (Task 2).
2. **Patterns: nesting, guards, or-patterns, and `Char` and range patterns,
   with no `is` and no `if let`** (the user's decision 4, spec §4.3). Every
   form of spec §4.3 checks, lowers and runs (Tasks 3 and 5); record, array,
   `@` and `Float` patterns stay E0900 (decision 10).
3. **E0103 warns at a newline-rule split only where the old reading could
   have been well typed** (the user's 5.0b ruling of 2026-10-27, which
   narrows the 5.0a plan's decision 37 for 5.0b on). `judge_split`, run on
   each statement of a block whose successor starts at a recorded split,
   finds the operand the old reading would have extended (`split_operand`:
   the rightmost operand of a binary operator; inside a unary operator for
   `(` and `[`; through a `return`'s value and an assignment's; through the
   block 5.0a's desugar makes of a compound assignment to an element or a
   field, `a[0] -= 2`, `r.x += 2` or `t.0.1 += 2`, to the value it writes;
   and into the body of a closure the source wrote, `|x| x`, whose body the
   old reading extends, but not into the closure that wraps a bare
   function's name; a `let`'s value is judged on its own) and marks the
   split quiet unless, before `(`, that operand is a function, before `[` an
   array, before `-` an `Int` or a `Float`, or its type is not yet known. A
   split after a bare `return` or `break` is never quiet.
   `warn_newline_splits` skips the quiet ones. So `g()` then `(a, b)` on the
   next line, `g` returning an `Int`, warns no more; after `let f = k()`,
   `k` returning a function, it still does, and so it does after `a[0] -= 2`
   or `let f = |x: Int| x` then `- 1` (Task 6; mutants 45 to 56). The first
   fact-check found the last two silent, the operand stopping at the
   desugar's block and at the closure: `split_operand` now reads through
   both
   (`a_split_after_a_compound_assignment_to_an_element_or_a_field_warns`,
   `a_split_after_a_closure_reads_its_body`). The one 5.0a test it moves:
   `e0103_reports_a_parenthesised_callee_after_a_line_break` counts one
   E0103 for `x` then `(g)()`, `x` an `Int`, where it counted two, and two
   after a function.
4. **5.0b is the second of three pull requests** (the user's decision 1):
   tuples, patterns, `for` over arrays and the E0103 ruling, after 5.0a's
   merge; std waits for 5.0c.

**The spec's decisions (its §12), as built**

5. **A tuple of one is `(T,)`, and `()` stays unit** (spec §12.11). `(T)`
   and `(e)` are `T` and `e`, as before
   (`a_parenthesised_type_and_expression_are_not_tuples`,
   `a_tuple_of_one_is_not_its_element`). A tuple has one element or more,
   with no upper limit: a 12-wide tuple and one nested five deep run in
   `tuples`.
6. **The lexer splits `1.0` after a `.` by its source text** (spec §12.12).
   `split_tuple_indexes` turns a float token after a `.` whose text is
   digits, `.`, digits into an integer, a `.` and an integer, so `t.1.0` is
   `(t.1).0`, `t.0.1.2` chains and `t.1.10` is `(t.1).10`; after a `..`, or
   any other token, and with a `_`, an exponent or a part too large for an
   integer, the float stays (mutants 1 to 3). The parser reads an integer
   after `.` as `Expr::Field` named by its digits, so the AST, fmt and the
   language server's field handling are unchanged.
7. **`for` takes an irrefutable pattern, by `let`'s rule** (spec §12.13).
   `for_loop_binding` binds a name or `_` as before, and any other pattern
   through `check_pattern` and `require_irrefutable` (E0022, "refutable
   pattern in a `for` loop"); `for (a, b) in 0..3` is E0010 (mutant 34).
8. **Literals and ranges are each their own constructor; usefulness does not
   split ranges** (spec §12.14, and §13's open point, settled). `Ctor::Int`,
   `Char`, `Str` and `Range` are equal only to themselves, of a type with
   too many values to list, so a match on them needs `_` or a name, and the
   same literal or range again is reported unreachable, an overlap not.
   Treating overlapping constructors as disjoint only drops rows from a
   specialization, which makes a row more useful: no arm is reported
   unreachable when it is not, and no match accepted when it is not
   exhaustive. What it gives up: an arm earlier ranges cover is not
   reported, and ranges that cover a whole type still need `_`
   (`the_same_literal_or_range_again_is_unreachable_but_an_overlap_is_not`,
   `ranges_are_not_split`).
9. **Range forms stay as the parser reads them:** `Int` `lo..hi` and
   `lo..=hi`, `Char` `lo..=hi` (spec §12.15). An end is a literal; there is
   no `-` in a pattern, so a negative end, or a negative literal, does not
   parse, before 5.0b and after (decision 29). `lo..hi` is stored as
   `lo..=hi - 1`, a `Char` by its code point; a range over another type, or
   with ends of two types, is E0010 (mutants 18 and 20).
10. **Record, array, `@` and `Float` patterns stay E0900** (spec §12.16),
    each with its own message: "record patterns", "array patterns", "`@`
    patterns", "`Float` literal patterns" (`the_forms_still_out_are_e0900`).
11. **Lowering by test chains, the flat switch kept** (spec §12.17, the
    recommendation taken without asking). A match lowers by chains when any
    arm has a guard or a pattern `hir::Pattern::is_flat` refuses (a `_`, a
    binding, a literal, or a variant whose fields are each `_` or a
    binding); otherwise exactly as before 5.0b. A guarded match of flat
    patterns takes the chains: spec §4.6's "as today's checker accepts them"
    excludes guards, which were E0900 (mutant 41). The prototype built 159
    runtime programs `--release` with the base's `nova` and its own and
    found their `.ll` byte-identical, so no existing program's code moved
    (decision 16).

**What spec §13 left to this plan, settled**

12. **Each site that walks `Ty`, and the mangling.** `Ty::Tuple` has an arm
    in `nova-hir`'s `subst`, `mentions_param`, `has_params`, `has_vars`,
    `has_assoc`, `has_error`, `match_pattern`, `normalize_within`,
    `shift_params`, `occurs` and `unify_patterns` (`head()` gives `None`);
    in `nova-typeck`'s `infer.rs` (`apply`, `occurs`, `unify`, which checks
    the arity), `check.rs` (`convert_ty`, `collect_self_projections`,
    `child_exprs`, `child_exprs_mut`, `finalize_expr`) and `display_ty` and
    `display_ty_named`; in the resolver's `type_full_name`, which names an
    impl's type argument (`Tup2_Int_Int`); and in MIR's `mir_ty` (a
    pointer), `mangle_ty`, mono's `type_name` and its specializer. A tuple
    type mangles as `T`, its elements' manglings, and `E`: each element's
    mangling is prefix-free and none begins with `E`, so `(Int, (Int, Int))`
    and `((Int, Int), Int)`, and `((Int,), Int)` and `((Int, Int),)`, get
    distinct strings, and `T` was not a leading letter before
    (`mangle_ty_gives_distinct_tuple_types_distinct_strings`,
    `a_generic_function_at_distinct_tuple_types_is_distinct_instances`;
    mutants 12 and 13).
13. **The HIR pattern and the chain's temporaries.** `hir::Pattern` is
    recursive: `Wildcard`, `Bind`, `LitInt`, `LitBool`, `LitStr`, `LitChar`,
    `Range { lo, hi }` (both ends included),
    `Variant { sum, variant, fields }`, `Tuple` and `Or`, with `bindings()`
    and `is_flat()`; `Arm` has `guard: Option<Expr>`. A chain projects each
    sub-value once, into a fresh temporary, just before its own tests
    (`SumField` for a variant's field, `RecordField` for a tuple's element),
    and skips a `_` (`each_projection_is_evaluated_once_per_chain`; mutant
    38).
14. **`t.01`, `t.1_0`, `t.0x1` and `t.00` are P0001,** "a tuple index is
    plain decimal digits, with no leading zero, `_` or prefix: write `.1`",
    as Rust refuses them, so each index has one spelling.
    `Tokens::plain_decimal` says which integers are spelled so; `t.1e5`
    stays "found float literal"
    (`an_index_that_is_not_plain_decimal_digits_is_refused`; mutants 4 and
    5).
15. **An unreachable alternative inside an or-pattern is reported** (spec
    §13). E0021, "unreachable alternative", labelled on the alternative
    alone, when its arm is reachable and has no guard; it is checked against
    the earlier arms and the earlier alternatives of its own arm, and an
    alternative nested inside one already reported is not reported again
    (`an_unreachable_alternative_is_e0021_on_it_alone`,
    `an_alternative_inside_one_reported_is_not_reported_again`,
    `a_whole_unreachable_arm_reports_the_arm_not_its_alternatives`; mutants
    31 and 32).
16. **No flat outer switch before the inner chains** (spec §13: "if MIR's
    size calls for it"). It does not: the prototype's 14-arm match three
    levels deep over a token is 190 statements in 67 blocks. A chain tests a
    tag once per arm that names it, where a switch would test it once; a
    later pre-switch, or a tree, needs no language change. ADR 0049 says so.

**Where the prototype departs from the spec, or settles what it left open**

17. **A flat `let` of a one-variant sum goes through the flat switch.**
    **(differs)** Spec §4.6 says `let` and `for` destructuring lower to
    projections. A destructuring `let` is a one-arm `match`
    (`patterns::let_pattern`), so `let Wrap(a, b) = w`, whose pattern is
    flat, takes today's switch, with one tag compare; a nested or tuple
    `let` lowers by chains, where a sum of one variant gets no tag test, so
    its projections are all there is
    (`let_destructuring_lowers_to_projections_alone`; mutant 39). One
    compare per flat `let` of a one-variant sum is the cost; a second
    lowering path for `let` was not worth it.
18. **A bare name inside a variant pattern that names a variant matches that
    variant: Rust's rule, the user's ruling of 2026-10-28.** **(differs)**
    The spec names no program whose meaning 5.0b changes; this changes one,
    in what it runs and not only in how it checks. Before 5.0b a variant's
    fields could only be names or `_`, so `Some(None)` bound a local named
    `None` and matched every `Some`; now each field is a pattern, and a bare
    name that resolves to a variant is that variant, the rule the top of a
    pattern always had. So `match c { Some(Red) => 1, _ => 2 }` gave `1` for
    every `Some` and gives `2` for `Some(Green)` now. Without a catch-all,
    exhaustiveness reports the change (E0020); behind one, nothing does. A
    name that is a variant of another type is E0001 there, "`Eof` is not a
    variant of the matched type", with the note "a name inside a pattern
    that names a variant matches that variant; to bind a value, use a
    lower-case name"; a variant with fields named bare, `Some(Leaf)` for
    `Leaf(Int)`, is E0016, as at the top of a pattern. The user chose Rust's
    rule over keeping the binding, with a warning for a binding that reads
    as a variant (decision 42). No repository program writes such a pattern:
    a search of every match arm `a3a95a2e` tracks for a bare upper-case name
    inside a variant's parentheses finds none in 1427 arms (Task 8, Step 5).
    The `check` comparison cannot show it, as a binding that becomes a
    variant checks the same. The CHANGELOG's Changed and ADR 0049 record it
    (`a_variant_name_inside_a_variant_matches_that_variant`,
    `another_types_variant_name_inside_a_variant_is_e0001_with_a_note`,
    `a_variant_with_fields_named_bare_inside_a_variant_is_e0016`,
    `a_lower_case_name_inside_a_variant_still_binds`; mutant 17).
19. **A name bound twice in one pattern, `(x, x)`, is unchanged from before
    5.0b:** both bind, the second shadowing the first, with no error
    (`W(x, x)` printed `2` with the base's `nova` and prints `2` now). Spec
    §4.3 is silent; a code for it waits for a later part.
20. **`.N` on a value of unknown type is E0011 once.** "the type of this
    value must be known before `.0` reads an element of it", with the note
    "annotate the value's type, for example `let p: (Int, Int) = …`"; the
    variable is then bound to the error type, so inference does not report
    it again (`an_index_on_a_value_of_unknown_type_is_one_e0011`; mutant 9).
    An index past the end, a name on a tuple and an index on another type
    are E0014, "no field `2` on `(Int, Int)`" (mutant 8).
21. **`t.1(x)` and `(t.1)(x)` are method calls,** E0014 "no method `1`": the
    parser drops the parentheses, as it does for a record's field. A program
    binds the element to a local first, as `tuples` does. Records have the
    same limit; neither changes in 5.0b.
22. **The binding rules of an or-pattern** (spec §4.3). The first
    alternative's names are the pattern's; a later alternative binds the
    same locals, so an or-pattern's arm has one set of locals whatever
    alternative matched. A missing name is E0100 on the alternative, "`x` is
    missing from this alternative", with a note naming the first
    alternative's binding; an extra one E0100 on the name, "`y` is not bound
    in the first alternative"; a name at two types E0100, "the alternatives
    bind `x` at different types", both types in the label (mutants 21 and
    22).
23. **E0022 by usefulness.** `require_irrefutable` asks usefulness for the
    values a `let` or `for` pattern misses and reports them, "`No` is not
    matched", "`(_, false)` is not matched", "`(_, 0)` is not matched"
    (decision 44), with the note listing what `let` and `for` take; it says
    nothing while the type is a variable, a parameter, the error type or
    `Never` (mutants 33 and 34).
24. **`let mut P` makes every binding of `P` mutable.** **(differs)** Spec
    §4.3 lists `mut x` among a pattern's forms; `mut` does not parse inside
    a pattern (P0001), so `let mut (a, b) = p` is the way to write it, and a
    destructured binding is immutable without it. E0060's note follows where
    the name is bound, where it said "declare it as `let mut a`", which does
    not apply: a destructuring `let`'s binding, "`a` is bound by a pattern,
    whose bindings are immutable: a destructuring `let mut (…) = …` makes
    each of them mutable, or rebind it as `let mut a = a` to allow
    assignment"; a match arm's or a `for` pattern's, the rebinding alone;
    and a plain `for x in` variable, as `for mut x` does not parse, "a `for`
    loop's variable is immutable: rebind it inside the loop's body as
    `let mut x = x` to allow assignment" (the second check)
    (`a_destructured_let_binding_is_immutable_without_mut`,
    `a_match_arm_or_a_loop_variable_is_rebound_to_be_mutable`,
    `a_for_loop_variable_over_an_array_is_immutable`; mutants 24, 61, 66 and
    67).
25. **An empty range keeps its pattern.** E0101, "this range matches
    nothing", labelled "`5..5` stops before its start" or "`9` is above
    `1`"; the pattern stays as written rather than `_`, so the arms after it
    are not also reported unreachable
    (`an_empty_range_does_not_make_the_arms_after_it_unreachable`; mutant
    19).
26. **The flat switch gains `Char` keys,** a `Char` literal switching on its
    code point as an `Int` does, so a flat match on a `Char` stays one
    switch. **(differs)** Spec §4.6 keeps the flat path for matches "as
    today's checker accepts them", and today's checker refused a `Char`
    pattern (E0900), so the spec sends a `Char` match to the chains. The
    switch is kept for it because a match on characters is a lexer's
    commonest shape: one switch, where the chains would compare each literal
    in turn, and the flat path already switches on an `Int` the same way.
    The key rides with Task 3, which makes `Char` patterns check: the second
    check found that with the key in Task 5, a flat `Char` match built at
    Tasks 3 and 4 switched on no key and took its first arm for every
    character (`A A A` for `'a'`, `'b'`, `'z'`, where the last tree prints
    `A B other`) (`a_flat_match_keeps_its_one_switch`, and from Task 3
    `lower_tests.rs`'s `a_flat_char_match_switches_on_its_code_points`;
    mutants 40, and 72 at Task 3's commit).
27. **`for p in a` over an array** (spec §4.5). `check_for_array` desugars
    it beside the range and iterator desugars: `__arr` holds `a`, `__n` its
    length, read once, and `__i` the index; each turn binds `p` to
    `__arr[__i]`, advances `__i`, then runs the body, so `continue` is safe
    and a write to a later element is seen. The hidden locals are unscoped,
    so no source name reaches them; the binding is immutable
    (`for_over_an_array_desugars_to_an_index_loop`,
    `a_for_loop_variable_over_an_array_is_immutable`; mutants 42 to 44). The
    message for anything else names arrays now: "`for` loops over anything
    but an integer range (`a..b`), an array, or a value implementing
    `Iterator` (try `.iter()`)".
28. **The witnesses** nest and render as patterns: `Some(Err(_))`,
    `(_, false)`, a tuple of one `(1,)`, a range `0..=9`, a `Char` `'a'`; an
    or-pattern inside a witness as its alternatives joined by ` | `
    (`a_missing_nested_variant_is_e0020_with_a_nested_witness`,
    `a_tuple_of_one_witness_keeps_its_comma`; mutants 25 and 30).
29. **The newline rule meets tuples and patterns without a silent change of
    meaning** (step 0's probes, Task 8). A tuple tail after a statement
    warns by decision 3; a tuple pattern arm after an arm that ended at a
    line break begins an arm, as 5.0a's
    `a_tuple_pattern_arm_after_an_arm_body_begins_an_arm` pins, and checks
    (`a_tuple_pattern_arm_after_a_bare_return_or_an_ordinary_arm_checks`);
    `let (a, b) = …` on the line after a statement begins a statement. A
    `-1..=5` arm is P0001, "expected pattern (in statement), found `-`",
    before the rule and after it: no `-` begins a pattern (decision 9), and
    the base read `g()` then `-1..=5 =>` as `g() - 1..=5`, then failed at
    the `=>`. The P0001 there carries 5.0a's note about an arm the rule
    began.
30. **fmt and the language server need no code.** fmt prints tuple types,
    literals, `t.N`, every pattern form, guards and ranges from the source,
    and its tests pin the layouts: a tuple of one keeps its comma, a long
    tuple breaks one element a line, a long or-pattern breaks before each
    `|`, an author's or-pattern across lines joins when it fits (Tasks 1 and
    3; mutants 73 to 76). Step 0's probes, twelve files of every new form,
    format, format again to the same text, and parse to the same tree,
    splits included (Task 8). The language server hovers a tuple as
    `(Int, Int)` through `display_ty`, and hovers and completes bindings in
    nested, or- and `for` patterns, with no code change; there is no
    completion of tuple indexes (spec §4.2).
31. **WASM: six programs join the gate, and no WASM code changes.**
    `tuples`, `tuples_async`, `patterns`, `patterns_async`, `match_effects`
    and `for_arrays` are in `wasm_tests.rs`'s `gate!` and pass under Node
    (Tasks 2 and 5).
32. **The records** (spec §4.7): ADR 0049; the CHANGELOG's Unreleased, Added
    (tuples, patterns, destructuring, `for` over arrays) and Changed
    (E0103's ruling, the bare variant name, the chains, the lexer's split);
    dated notes in `11-PARSER.md` §2, `12-TYPESYSTEM.md` §3 and §8 with the
    catalog's E0100 and E0101, `13-RUNTIME.md` §2.2, `14-CODEGEN.md` §4 and
    §6.2, and `20-STDLIB.md`.

**The split's decisions**

33. **Exhaustiveness rides with the patterns' checker, in Task 3.** The
    natural split gives usefulness a task of its own. Its new constructors
    are made only from checked patterns, so a task of usefulness alone
    leaves `Ctor::Char`, `Ctor::Range` and `Pat::Or` never constructed, and
    clippy's `-D warnings` fails (the first split's `cargo check` at that
    task: "variant `Or` is never constructed", "variants `Char` and `Range`
    are never constructed"); and `to_useful_pat` cannot take the new HIR
    patterns without them. Task 3 holds both.
34. **`for x in arr` comes before the chains, and `for_arrays` with them.**
    The patterns' programs loop over arrays (`for tok in toks`), so the
    chains' programs need Task 4; and `for_arrays` binds tuple patterns,
    which a one-arm match over a tuple lowers by chains, so it needs Task
    5. The first split had the chains first, and its Task 4 run failed
    `patterns_run`, `patterns_async_runs` and `match_effects_run` with
    "E0900: `for` loops over anything but an integer range … are not
    supported yet". So Task 4 is the desugar and its checker and language
    server tests, and Task 5 brings all four programs.
35. **From Task 3 to Task 5's code, lowering refuses a nested or guarded
    match** (the first fact-check). Task 3's checker accepts them, and
    before the chains the flat switch would read a nested field as `_` and
    ignore a guard, so a binary built at Task 3 or 4 would run one wrongly.
    Task 3's `lower_match` refuses a match with a guard or a pattern
    `is_flat` refuses: E0900, "matches with nested, tuple, or- or range
    patterns or guards are not supported yet", labelled "not lowered yet".
    MIR is where today's other lowering E0900s live (`lower_tests.rs` pins
    one), and the checker is complete at Task 3, so `nova check` accepts
    such a program while `nova build` and `nova run` refuse it, and no
    commit builds a binary that runs it wrongly.
    `a_nested_or_guarded_match_is_refused_until_chains` pins the refusal
    (Task 3; mutants 70 and 71, run at Task 3's commit); Task 5's tests
    patch drops that test and its code patch the refusal. Task 5's tests
    show it before the chains: nine of `pattern_lowering.rs`'s ten tests at
    the refusal, the IR test at the refusal, and the four programs, each
    refused with E0900; the tenth, `a_flat_match_keeps_its_one_switch`,
    passes, its matches flat (Task 5, Step 2).
36. **The language server's `for` tests ride with Task 4.** The first split
    put them in Task 3, with the other pattern tests, where they failed:
    `for` over an array is Task 4's.
37. **The programs ride with their features.** `tuples` and `tuples_async`
    with Task 2, the other four with Task 5; each brings its `run_tests.rs`
    test, its `gate!` entry and `index_complete.rs`'s counts, 171 programs
    and 205 files from Task 2, 175 and 209 from Task 5, in the same task
    (5.0a's lesson). `tuples_async`'s comment says the `;` before its tail
    tuple is optional only from Task 6, which makes it so. The three
    comments step 0 corrected ride with what made them stale: `std/http`'s
    and `http_serialise`'s with the chains (Task 5), `fs_read_dir`'s with
    `for` over arrays (Task 4).
38. **Tests that cannot stand first, and tests that pass first.** A test
    inside a crate's source, or naming what the code adds, does not compile
    before it: `lexer_tests.rs` (Task 1); the HIR, infer and mono tests,
    `tuple_tests.rs` and `lower_tests.rs` (Task 2); the HIR pattern tests,
    `usefulness.rs`'s and `pattern_tests.rs`'s (Task 3), each crate's whole
    test binary with it. These pass before their code, as they should: in
    Task 1, six of `nova-parser`'s `tuples.rs`, the forms the base already
    parsed (`a_tuple_literal_has_two_or_more_elements_or_a_trailing_comma`,
    `unit_and_a_parenthesised_expression_stay_what_they_are`,
    `tuple_types_parse_with_unit_and_a_parenthesised_type_unchanged`,
    `an_index_with_an_exponent_is_a_float_and_refused`,
    `the_newline_rule_is_off_inside_a_tuple_literal` and
    `a_tuple_on_the_line_after_a_statement_begins_a_new_one`), and three of
    fmt's (`a_tuple_of_two_loses_a_trailing_comma_when_it_fits`,
    `a_long_tuple_breaks_one_element_a_line` and
    `a_tail_tuple_after_a_statement_keeps_its_semicolon`); in Task 3, fmt's
    ten `patterns.rs` tests, fmt needing no code; in Task 4,
    `a_for_pattern_with_an_error_is_not_also_e0022`, the base's E0900 for
    the loop standing in for its error; in Task 5,
    `a_flat_match_keeps_its_one_switch`, the flat switch and its `Char` keys
    being Task 3's; and in Task 6,
    `a_split_after_a_bare_return_warns_whatever_comes_before`,
    `a_split_after_a_bare_return_before_a_tail_warns`,
    `a_split_after_a_value_of_unknown_type_warns` and
    `a_split_after_a_compound_assignment_to_an_element_or_a_field_warns`,
    splits the ruling leaves loud.

**Step 0's decisions, before the split**

39. **Three comments that called nested patterns and `for x in arr` E0900
    say they work** (`std/http/lib.nova`,
    `tests/runtime/http_serialise.nova`, `tests/runtime/fs_read_dir.nova`).
    A comment shifts the spans after it, so Task 8's AST check compares
    these three files' dumps with the spans stripped, and every other file's
    exactly.
40. **The probes found nothing to fix in fmt or the language server;** the
    language server gained two tests from them (decision 30). The newline
    probes found no silent change of meaning (decision 29).

**The mutants' gaps, and the fact-checks**

41. **The mutants' first run left three gaps,** each closed by
    `8da2b8c9 5.0b prototype: close the mutants' gaps`, with a test that
    fails with the mutant applied: mono naming a tuple of one in a bound's
    E0013 without its comma
    (`a_tuple_of_one_in_a_bound_message_keeps_its_comma`); E0103's ruling
    judging a value of unknown type as known
    (`a_split_after_a_value_of_unknown_type_warns`); and `split_operand`'s
    arm for a `let`, which no test reached because it was dead, a `let`'s
    value being judged on its own: the arm goes, and
    `the_split_operand_is_read_through_an_assignment` pins the assignment
    arm. A fourth test, `a_split_after_a_bare_return_before_a_tail_warns`,
    pins the bare `return`'s split where no "this value is not used" can
    stand in for it, as it can in
    `a_split_after_a_bare_return_warns_whatever_comes_before`.
42. **E0104, a warning for a binding that reads as a variant** (the user's
    ruling of 2026-10-28, with decision 18). A name in a pattern that starts
    with an upper-case letter and names no variant in scope binds, as any
    name does, and warns: E0104, "`Rde` binds a value: no variant of that
    name is in scope", labelled "this binding matches any value", with the
    note "a name that starts with an upper-case letter reads as a variant;
    if one was meant, check its spelling or import it; to bind a value, use
    a lower-case name". At a constant's name the note is "a pattern cannot
    match a constant by name; use a guard: `n if n == MAX`", and at a name
    near a variant, the matched sum's or one in scope, a second note, "did
    you mean `Empty`?", comes with the fix "change `Emtpy` to `Empty`" (the
    second check). It fires in a `match` arm at any depth, and in a
    destructuring `let` or `for` pattern, once in an or-pattern, whose later
    alternatives bind the first one's local; not for a lower-case name or a
    real variant, and not for `_Upper`, whose `_` says the binding is meant,
    as an unused-variable warning's `_` does. A plain `let X = …` or
    `for X in …` binds a name, not a pattern, and is not checked. E0104 was
    the catalog's next free code; `12-TYPESYSTEM.md` registers it. No
    repository file draws it (Task 8, Step 6)
    (`an_upper_case_binding_is_e0104`,
    `a_lower_case_or_underscored_binding_or_a_real_variant_is_not_e0104`,
    `an_upper_case_binding_in_an_or_pattern_warns_once`,
    `an_upper_case_binding_in_a_for_pattern_is_e0104`,
    `e0104_on_a_constant_says_to_use_a_guard`,
    `e0104_near_a_variant_says_which`; mutants 57, 58, 62 and 63).
43. **A match with a pattern error skips exhaustiveness and reachability.**
    E0001's pattern becomes `_`, which made every arm after it unreachable,
    an E0021 cascade; a pattern that adds an error now keeps its `match`
    from usefulness, as one keeps a `let` or `for` from E0022
    (`an_erroneous_pattern_does_not_make_the_arms_after_it_unreachable`,
    and, from the second check,
    `a_let_pattern_with_an_error_is_not_also_e0022` and
    `a_for_pattern_with_an_error_is_not_also_e0022`; mutants 59, 64 and 65).
44. **E0022's witness names a value where a literal is missing.**
    `let (a, 5) = p` reports "`(_, 0)` is not matched", where it said
    `(_, _)`: the first `Int` from 0 that no row covers, ranges included,
    the first `Char` from `'a'`, or `""` and then a letter for a `String`.
    Usefulness names values only when E0022 asks (`MatchCx::naming_values`):
    E0020's witnesses keep `_`, the catch-all the user adds there, and
    5.0a's tests of them stand. Each missed value is named once: a column
    that no row names gives `_`, not each constructor, and where E0022 asks
    the values under each present constructor are named too, so
    `for (1, true) in p` reports "`(0, _)`, `(1, false)` are not matched",
    where it said "`(0, true)`, `(0, false)`" and left `(1, false)` out (the
    second check) (mutants 60, 68 and 69).
45. **Usefulness is exponential in the or-patterns of a row, a known
    limit.** Each or-row is expanded whole: a tuple pattern of sixteen
    `true | false` takes 14 s to check in a debug build, fourteen 2.7 s,
    twelve 0.6 s. No program writes one; expanding lazily is a later part's,
    and ADR 0049 records the measurement.
46. **The records the first fact-check corrected:** `13-RUNTIME.md`'s note
    says the collector scans a tuple's slots conservatively, as a record's;
    `12-TYPESYSTEM.md`'s E0022 row covers `for`, its patterns note carries
    E0001's, E0016's and E0104's rules and decision 43, and its tuple bound
    note names `Debug`; ADR 0049's Consequences say exhaustiveness cannot
    catch decision 18's change behind a catch-all.
47. **Every program at Tasks 3 and 4 runs as at the end, or is refused**
    (the second check). With Task 3's refusal (decision 35) and its `Char`
    key (decision 26), a binary built at Task 3 or 4 either runs a program
    as the last tree's does or refuses it before it runs. Task 8, Step 10
    shows it: every runtime program of the last tree, and a flat `Char`
    match probe, run with the base's `nova`, Task 3's, Task 4's and the last
    tree's: 146 programs and the probe; at Task 3 142 run as the last tree
    does and Task 5's 4 are refused, at Task 4 the same, none differs; the
    `Char` probe prints `A B Z other` at both.
48. **The mutants leave the tree built** (the second check): the replay's
    last mutant left `target/debug/nova` a mutant's build. Task 8, Step 9
    ends with `cargo build --locked --workspace`, so `target/debug` holds
    the last commit's binaries for the steps after it.

---

### Task 0: The baseline

The branch is `claude/loving-archimedes-frtf5q-50b`, at this plan's commit,
whose parent is `a3a95a2e`, `main` after 5.0a. The plan's commit adds this
file alone, so the patches apply to it as to `a3a95a2e`.

- [ ] **Step 1: The tree.**
  - `git status --short` prints nothing, on
    `claude/loving-archimedes-frtf5q-50b`.
  - `git diff --stat a3a95a2e HEAD` lists only
    `docs/superpowers/plans/2026-10-28-phase-5-0b-tuples-and-patterns.md`.
  - `BASE=$(git rev-parse --short HEAD)`: the plan's commit, recorded in the
    ledger. Task 8's review reads from it.
  - `df -h /` shows at least 7G free; if not, remove
    `target/debug/incremental` and older duplicate test executables in
    `target/debug/deps` first, and the workspace crates' artifacts there
    that no build of this tree has written today, which none reads. The
    replay began its runs with 6.9G to 7.6G, removed older duplicate test
    executables before each command, and saw 6141M at the lowest, in the
    first form of Task 8, Step 10, which kept four copies of `nova`; the
    step now keeps none (decision 47).
- [ ] **Step 2: The tools.**
  - `clang-18 --version | head -n 1` prints
    `Ubuntu clang version 18.1.3 (1ubuntu1)`: CI's pin, for the golden
    suite.
  - `node --version` prints `v22.22.0`, for the WASM tests.
  - `cargo +1.78 --version` prints `cargo 1.78.0`, for Task 8.
- [ ] **Step 3: The suite.** In a shell set up as the conventions say:

  ```sh
  cargo build --locked --workspace
  NOVA_REQUIRE_NODE=1 cargo test --locked --workspace --no-fail-fast > "$P/base-50b.txt" 2>&1
  grep '^test result' "$P/base-50b.txt" | awk '{p += $4; f += $6; i += $8} END {print p, f, i}'
  grep ' FAILED$' "$P/base-50b.txt"
  cp target/debug/nova "$P/nova-base-50b"
  cargo test --locked --workspace -- --list > "$P/base-list-50b.txt" 2>&1
  grep -c ': test$' "$P/base-list-50b.txt"
  ```

  Expected, from 5.0a's last full run on `a3a95a2e` and not the
  replay's: `2534 1 10`, and one failure, `test
  net::tests::an_ipv6_connect_parks_on_its_first_poll_and_then_establishes
  ... FAILED`. It needs IPv6, which this machine lacks, as in every
  earlier phase. The replay built `a3a95a2e` and listed its tests, and
  found `2545` names, the run's 2534 + 1 + 10; its `nova` was byte for
  byte the copy the prototype's checks used. The copy of `nova` is the
  base's, for Task 8's AST and `check` comparisons; the list is the base's
  names, for Task 8's comparison. Task 8 expects `2705 1 10`: 5.0b's tests
  make 171 more, 171 added and 0 gone.
- [ ] **Step 4: The ledger.** Start `$P/ledger-50b.md`, headed with the base
  commit and the baseline's figures.

### Task 1: Tuples in the lexer and the parser: the split after `.`, and `t.N`

Spec §4.1, §4.2; decisions 5, 6 and 14.

**Files:**
- Modify:
  - `crates/nova-lexer/src/lib.rs` (`split_tuple_indexes`)
  - `crates/nova-lexer/src/tokens.rs` (`Tokens::plain_decimal`)
  - `crates/nova-parser/src/grammar.rs` (an integer after `.`)
- Test:
  - `crates/nova-lexer/tests/lexer_tests.rs` (seven tests)
  - `crates/nova-parser/tests/tuples.rs` (new, 12 tests)
  - `crates/nova-fmt/tests/tuples.rs` (new, seven tests)

- [ ] **Step 1: The tests.** `patch_block t1-tests`. The patch holds:
  - `lexer_tests.rs`: an integer after a `.` is an index; `1.0` after a `.`
    splits into `1`, `.`, `0`; the split chains (`t.0.1.2`); it follows the
    text, not the value (`t.1.10`); a float elsewhere (`0..1.5`, `x = 1.5`)
    is not split; one with a `_` or an exponent after a `.` is left whole;
    and only plain decimal integers are marked as indexes.
  - `nova-parser`'s `tuples.rs`: a tuple literal has two elements or more,
    or a trailing comma; `()` and `(e)` stay unit and the expression; tuple
    types, with `()` and `(T)` unchanged; `t.0` is `Expr::Field` named `0`;
    nested fields read left to right; postfix forms after a field
    (`t.0.len()`, `t.0.await`, `t.0.x.1`, and `+` and `*` around them); a
    field as a place (`t.0 = 1`, `t.1.0 += 2`); a float elsewhere stays one
    (`t.0 + 1.5`); `t.01`, `t.1_0`, `t.0x1`, `t.00` and `t.0b1` refused with
    "write `.1`" (or `.10`, `.0`); `t.1e5` a float and refused; the newline
    rule off inside a tuple literal; and a tuple on the line after a
    statement beginning a new one.
  - fmt's `tuples.rs`: `t.0.1` and `t.1.10` print as written; a space around
    a tuple field's dot goes; a tuple of one keeps its comma, in a type, a
    literal and a field; a tuple of two loses a trailing comma when it fits;
    a field write and `+=` print as written; a long tuple breaks one element
    a line; a tail tuple after a statement keeps its `;`.
- [ ] **Step 2: Run, and see the tests fail.** First the build
  (Conventions), which ends with `Finished`.
  - `cargo test --locked --workspace --test lexer_tests`. Expected: it does
    not compile, three
    ``error[E0599]: no method named `plain_decimal` found for struct `Tokens` in the current scope``,
    then
    ``error: could not compile `nova-lexer` (test "lexer_tests") due to 3 previous errors``
    (decision 38).
  - `cargo test --locked --workspace --no-fail-fast --test tuples`, which
    runs fmt's `tuples.rs`, then `nova-parser`'s. Expected: fmt's
    `3 passed; 4 failed`: `tuple_fields_print_as_written`,
    `a_space_around_the_dot_of_a_tuple_field_goes`,
    `a_tuple_of_one_keeps_its_comma` and
    `a_tuple_field_write_and_compound_assignment_print_as_written`, each
    "formatting failed: … P0001 … expected identifier (in field access),
    found integer literal" (or "found float literal"); the other three print
    what the base printed. Then the parser's `6 passed; 6 failed`. The six
    that fail are the index's: `a_tuple_field_is_named_by_its_index`,
    `nested_tuple_fields_read_left_to_right`,
    `a_tuple_field_takes_the_postfix_forms_after_it`,
    `a_tuple_field_is_a_place`, `a_float_elsewhere_stays_a_float` and
    `an_index_that_is_not_plain_decimal_digits_is_refused`; the helper's
    `assert!` on the parse errors (`tuples.rs:39:5`) or the refusal's
    message (`tuples.rs:237:9`). The six that pass are the forms the base
    already parsed as 5.0b does: literals, unit and parentheses, types, an
    exponent refused, the rule inside a literal and after a statement.
- [ ] **Step 3: The code.** `patch_block t1-code`:
  - **`nova-lexer`'s `lib.rs`:** `tokenize` passes its tokens through
    `split_tuple_indexes` before the `Eof`: after a `Dot`, a `Float` whose
    source text splits at its `.` into two runs of ASCII digits that each
    parse as an `i64` becomes `Int`, `Dot`, `Int`, with their spans.
  - **`tokens.rs`:** `Tokens.plain_decimal`, one flag a token, built in
    `Tokens::marked` (an `Int` whose text is all digits, with no leading
    zero but `0` itself), and `plain_decimal(index)`, `false` past the end.
  - **`grammar.rs`:** in the postfix loop, after a `.`, an `Int` becomes
    `Expr::Field { field: "N" }`, with P0001 "a tuple index is plain decimal
    digits, with no leading zero, `_` or prefix: write `.{n}`" unless
    `plain_decimal` holds.
- [ ] **Step 4: Run.** First the build, which ends with `Finished`, about 12 s.
  - `cargo test --locked --workspace --test lexer_tests`. Expected:
    `49 passed; 0 failed`.
  - `cargo test --locked --workspace --no-fail-fast --test tuples`.
    Expected: `7 passed; 0 failed`, fmt's, then `12 passed; 0 failed`, the
    parser's.
  - `S > "$P/t1-50b.txt" 2>&1`, then `grep '^test result' "$P/t1-50b.txt"
    | awk '{p += $4; f += $6; i += $8} END {print p, f, i}'` and `grep '
    FAILED$' "$P/t1-50b.txt"` (about 5 min 0 s). Expected: `2560 1 10`,
    Task 0's figures and Task 1's 26 tests, and the one failure Task 0's
    IPv6 test.
- [ ] **Step 5: The checks,** as the conventions list them.
- [ ] **Step 6: Commit**
  `lexer, parser: tuple indexes, the float split after a dot`.

**Patch `t1-tests`:**

<!-- patch t1-tests -->

````diff
diff --git a/crates/nova-fmt/tests/tuples.rs b/crates/nova-fmt/tests/tuples.rs
new file mode 100644
index 0000000000000000000000000000000000000000..63f224b2d47a45f182b8b1d57b191966d487290f
--- /dev/null
+++ b/crates/nova-fmt/tests/tuples.rs
@@ -0,0 +1,60 @@
+//! Tuples in fmt (5.0 spec
+//! `docs/superpowers/specs/2026-10-26-phase-5-0-the-language-and-std-a-compiler-needs-design.md`
+//! §4.1, §4.2): tuple types and literals print as they did, and `t.N`, a
+//! field named by its index, prints as written, so `t.1.0`, which the lexer
+//! splits after a `.`, reads back as the same tree.
+
+mod common;
+use common::{assert_formats, assert_stable};
+
+#[test]
+fn tuple_fields_print_as_written() {
+    assert_stable("fn f(t: ((Int, Int), Int)) -> Int { t.0.1 + t.1 }\n");
+    assert_stable(
+        "fn main() {\n    let n = t.0.1.2.3\n    let m = t.1.10\n    println(\"${n.0}\")\n}\n",
+    );
+}
+
+#[test]
+fn a_space_around_the_dot_of_a_tuple_field_goes() {
+    assert_formats(
+        "fn main() { let x = t . 0\n println(\"${x}\") }\n",
+        "fn main() {\n    let x = t.0\n    println(\"${x}\")\n}\n",
+    );
+}
+
+#[test]
+fn a_tuple_of_one_keeps_its_comma() {
+    assert_formats(
+        "fn f(p: (Int ,)) -> (Int ,) { (p.0 ,) }\n",
+        "fn f(p: (Int,)) -> (Int,) { (p.0,) }\n",
+    );
+}
+
+#[test]
+fn a_tuple_of_two_loses_a_trailing_comma_when_it_fits() {
+    assert_formats(
+        "fn f() -> (Int, Int,) { (1, 2,) }\n",
+        "fn f() -> (Int, Int) { (1, 2) }\n",
+    );
+}
+
+#[test]
+fn a_tuple_field_write_and_compound_assignment_print_as_written() {
+    assert_stable("fn main() {\n    let mut t = (1, 2)\n    t.0 = 3\n    t.1 += t.0\n}\n");
+}
+
+#[test]
+fn a_long_tuple_breaks_one_element_a_line() {
+    assert_formats(
+        "fn main() { let t = (\"a long string here\", \"another long string\", \"and a third, much, much longer one\", 12345) }\n",
+        "fn main() {\n    let t = (\n        \"a long string here\",\n        \"another long string\",\n        \"and a third, much, much longer one\",\n        12345,\n    )\n}\n",
+    );
+}
+
+#[test]
+fn a_tail_tuple_after_a_statement_keeps_its_semicolon() {
+    // The `;` ends the statement before the `(` without E0103 (5.0 spec
+    // §3.5), so fmt keeps it.
+    assert_stable("fn f() -> (Int, Int) {\n    g();\n    (1, 2)\n}\n");
+}
diff --git a/crates/nova-lexer/tests/lexer_tests.rs b/crates/nova-lexer/tests/lexer_tests.rs
index 40344d9bc7f1fd90a33e6562b6c1a9951d6f7727..27debe408d896fe458f44ab2054a0fe21edef943 100644
--- a/crates/nova-lexer/tests/lexer_tests.rs
+++ b/crates/nova-lexer/tests/lexer_tests.rs
@@ -710,3 +710,143 @@ fn the_marks_leave_the_tokens_as_they_were() {
 fn a_lone_carriage_return_is_a_line_break() {
     assert_eq!(marks("b\r(d)")[1], ("(".to_string(), true));
 }
+
+// ── Tuple indexes: the split after a `.` (5.0 spec §4.2) ───────────────────
+
+/// Each token with its source text, for the split's spans.
+fn texts(src: &str) -> Vec<(Token, String)> {
+    let mut db = FileDb::new();
+    let file = db.add("<split>", src);
+    let (tokens, errors) = lex(src, file);
+    assert!(errors.is_empty(), "{errors:?}");
+    tokens
+        .iter()
+        .filter(|t| t.value != Token::Eof)
+        .map(|t| {
+            let text = src[t.span.start as usize..t.span.end as usize].to_string();
+            (t.value.clone(), text)
+        })
+        .collect()
+}
+
+#[test]
+fn a_tuple_index_is_an_integer_after_a_dot() {
+    assert_eq!(
+        tokens("t.0"),
+        vec![ident("t"), Token::Dot, Token::Int(0), Token::Eof]
+    );
+}
+
+#[test]
+fn a_float_after_a_dot_splits_into_two_indexes() {
+    assert_eq!(
+        texts("t.1.0"),
+        vec![
+            (ident("t"), "t".to_string()),
+            (Token::Dot, ".".to_string()),
+            (Token::Int(1), "1".to_string()),
+            (Token::Dot, ".".to_string()),
+            (Token::Int(0), "0".to_string()),
+        ]
+    );
+}
+
+#[test]
+fn the_split_chains() {
+    assert_eq!(
+        tokens("t.0.1.2"),
+        vec![
+            ident("t"),
+            Token::Dot,
+            Token::Int(0),
+            Token::Dot,
+            Token::Int(1),
+            Token::Dot,
+            Token::Int(2),
+            Token::Eof
+        ]
+    );
+}
+
+#[test]
+fn the_split_follows_the_source_text_not_the_value() {
+    // `1.10` is the float 1.1, but the index after the second `.` is 10.
+    let t = texts("t.1.10");
+    assert_eq!(t[4], (Token::Int(10), "10".to_string()));
+    assert_eq!(t.len(), 5);
+}
+
+#[test]
+fn a_float_elsewhere_is_not_split() {
+    assert_eq!(tokens("1.0"), vec![Token::Float(1.0), Token::Eof]);
+    assert_eq!(
+        tokens("x = 1.5"),
+        vec![ident("x"), Token::Eq, Token::Float(1.5), Token::Eof]
+    );
+    // A `..` before it is a range, not a field access.
+    assert_eq!(
+        tokens("0..1.5"),
+        vec![Token::Int(0), Token::DotDot, Token::Float(1.5), Token::Eof]
+    );
+    assert_eq!(
+        tokens("0..1"),
+        vec![Token::Int(0), Token::DotDot, Token::Int(1), Token::Eof]
+    );
+    // A float followed by a `.` stays a float: `1.0.0` is not after one.
+    assert_eq!(
+        tokens("1.0.0"),
+        vec![Token::Float(1.0), Token::Dot, Token::Int(0), Token::Eof]
+    );
+}
+
+#[test]
+fn a_float_with_an_underscore_or_an_exponent_after_a_dot_is_left_whole() {
+    assert_eq!(
+        tokens("t.1_0.2"),
+        vec![ident("t"), Token::Dot, Token::Float(10.2), Token::Eof]
+    );
+    assert_eq!(
+        tokens("t.1.0e3"),
+        vec![ident("t"), Token::Dot, Token::Float(1000.0), Token::Eof]
+    );
+    assert_eq!(
+        tokens("t.1e3"),
+        vec![ident("t"), Token::Dot, Token::Float(1000.0), Token::Eof]
+    );
+}
+
+#[test]
+fn only_plain_decimal_integers_are_marked_as_tuple_indexes() {
+    let src = "t.0 t.7 t.12 t.01 t.1_0 t.0x1 t.00 t.0b1 t.0o7 x";
+    let mut db = FileDb::new();
+    let file = db.add("<plain>", src);
+    let (tokens, errors) = lex(src, file);
+    assert!(errors.is_empty(), "{errors:?}");
+    let plain: Vec<(String, bool)> = tokens
+        .iter()
+        .enumerate()
+        .filter(|(_, t)| matches!(t.value, Token::Int(_)))
+        .map(|(i, t)| {
+            let text = src[t.span.start as usize..t.span.end as usize].to_string();
+            (text, tokens.plain_decimal(i))
+        })
+        .collect();
+    let want: Vec<(String, bool)> = [
+        ("0", true),
+        ("7", true),
+        ("12", true),
+        ("01", false),
+        ("1_0", false),
+        ("0x1", false),
+        ("00", false),
+        ("0b1", false),
+        ("0o7", false),
+    ]
+    .iter()
+    .map(|(t, b)| (t.to_string(), *b))
+    .collect();
+    assert_eq!(plain, want);
+    // An identifier is no integer, and past the end is `false`.
+    assert!(!tokens.plain_decimal(tokens.len() - 2));
+    assert!(!tokens.plain_decimal(tokens.len() + 3));
+}
diff --git a/crates/nova-parser/tests/tuples.rs b/crates/nova-parser/tests/tuples.rs
new file mode 100644
index 0000000000000000000000000000000000000000..58cbca11bffb68d3635506998948f6c017661934
--- /dev/null
+++ b/crates/nova-parser/tests/tuples.rs
@@ -0,0 +1,276 @@
+//! Tuples in the parser (spec
+//! `docs/superpowers/specs/2026-10-26-phase-5-0-the-language-and-std-a-compiler-needs-design.md`
+//! §4.1, §4.2): tuple types and literals as the parser already read them,
+//! and `t.N`, a field whose name is its index. Each test renders a
+//! function's body, or its signature's types, in a compact form that leaves
+//! the spans out.
+
+use nova_ast::expr::{AssignOp, BinOp, Expr, Literal};
+use nova_ast::ty::Type;
+use nova_ast::{Block, Item, Stmt};
+use nova_diagnostics::FileDb;
+use nova_lexer::lex;
+use nova_parser::{parse, ParseError};
+
+fn parse_src(src: &str) -> (nova_ast::File, Vec<ParseError>) {
+    let mut db = FileDb::new();
+    let id = db.add("tuples.nova", src);
+    let (tokens, lex_errors) = lex(src, id);
+    assert!(lex_errors.is_empty(), "{lex_errors:?}");
+    let (file, errors) = parse(&tokens, id);
+    (file.expect("a file"), errors)
+}
+
+fn function(file: &nova_ast::File) -> &nova_ast::item::Function {
+    file.items
+        .iter()
+        .find_map(|i| match &i.value {
+            Item::Function(f) => Some(f),
+            _ => None,
+        })
+        .expect("a function")
+}
+
+/// The statements and tail of the body of `fn f() { <body> }`, one string
+/// each; a tail is marked `tail:`. Panics on a parse error.
+fn body(body: &str) -> Vec<String> {
+    let src = format!("fn f() {{\n{body}\n}}\n");
+    let (file, errors) = parse_src(&src);
+    assert!(errors.is_empty(), "{src}\n{errors:?}");
+    block(&function(&file).body.value)
+}
+
+/// The parse errors of `fn f() { <body> }`, and its tail.
+fn errors_of(body: &str) -> (Vec<String>, Vec<String>) {
+    let src = format!("fn f() {{\n{body}\n}}\n");
+    let (file, errors) = parse_src(&src);
+    (
+        errors.iter().map(|e| e.to_string()).collect(),
+        block(&function(&file).body.value),
+    )
+}
+
+/// The parameter types and the return type of `fn f<sig>`.
+fn sig(sig: &str) -> Vec<String> {
+    let src = format!("fn f{sig} {{}}\n");
+    let (file, errors) = parse_src(&src);
+    assert!(errors.is_empty(), "{src}\n{errors:?}");
+    let f = function(&file);
+    let mut out: Vec<String> = f.params.iter().map(|p| ty(&p.ty.value)).collect();
+    if let Some(r) = &f.return_ty {
+        out.push(format!("-> {}", ty(&r.value)));
+    }
+    out
+}
+
+fn ty(t: &Type) -> String {
+    match t {
+        Type::Path { path, args } => {
+            let name = path
+                .segments
+                .iter()
+                .map(|s| s.value.as_str())
+                .collect::<Vec<_>>()
+                .join("::");
+            if args.is_empty() {
+                name
+            } else {
+                let args: Vec<String> = args.iter().map(|a| ty(&a.value)).collect();
+                format!("{name}<{}>", args.join(", "))
+            }
+        }
+        Type::Tuple(items) => {
+            let items: Vec<String> = items.iter().map(|i| ty(&i.value)).collect();
+            format!("tuple({})", items.join(", "))
+        }
+        Type::Array(e) => format!("[{}]", ty(&e.value)),
+        other => format!("{other:?}"),
+    }
+}
+
+fn block(b: &Block) -> Vec<String> {
+    let mut out: Vec<String> = b
+        .stmts
+        .iter()
+        .map(|s| match &s.value {
+            Stmt::Let { init, .. } => format!(
+                "let = {}",
+                init.as_ref().map_or("-".to_string(), |e| show(&e.value))
+            ),
+            Stmt::Expr(e) => show(&e.value),
+            Stmt::Item(_) => "item".to_string(),
+        })
+        .collect();
+    if let Some(t) = &b.trailing {
+        out.push(format!("tail:{}", show(&t.value)));
+    }
+    out
+}
+
+fn list(xs: &[nova_diagnostics::Spanned<Expr>]) -> String {
+    xs.iter()
+        .map(|x| show(&x.value))
+        .collect::<Vec<_>>()
+        .join(", ")
+}
+
+fn show(e: &Expr) -> String {
+    match e {
+        Expr::Lit(Literal::Int(n)) => n.to_string(),
+        Expr::Lit(Literal::Float(f)) => format!("{f:?}"),
+        Expr::Path(p) => p
+            .segments
+            .iter()
+            .map(|s| s.value.as_str())
+            .collect::<Vec<_>>()
+            .join("::"),
+        Expr::Tuple(xs) => format!("tuple({})", list(xs)),
+        Expr::Field { target, field } => format!("{}.<{}>", show(&target.value), field.value),
+        Expr::Call { callee, args } => format!("{}({})", show(&callee.value), list(args)),
+        Expr::Binary { op, lhs, rhs } => {
+            let op = match op {
+                BinOp::Add => "+",
+                BinOp::Sub => "-",
+                BinOp::Mul => "*",
+                _ => "op",
+            };
+            format!("[{} {op} {}]", show(&lhs.value), show(&rhs.value))
+        }
+        Expr::Assign { op, lhs, rhs } => {
+            let op = match op {
+                AssignOp::Assign => "=",
+                AssignOp::AddAssign => "+=",
+                _ => "op=",
+            };
+            format!("{} {op} {}", show(&lhs.value), show(&rhs.value))
+        }
+        Expr::Await(x) => format!("{}.await", show(&x.value)),
+        other => format!("{other:?}"),
+    }
+}
+
+#[test]
+fn a_tuple_literal_has_two_or_more_elements_or_a_trailing_comma() {
+    assert_eq!(body("(1, 2)"), ["tail:tuple(1, 2)"]);
+    assert_eq!(body("(1, 2,)"), ["tail:tuple(1, 2)"]);
+    assert_eq!(body("(5,)"), ["tail:tuple(5)"]);
+    assert_eq!(
+        body("((1, 2), (3,))"),
+        ["tail:tuple(tuple(1, 2), tuple(3))"]
+    );
+}
+
+#[test]
+fn unit_and_a_parenthesised_expression_stay_what_they_are() {
+    assert_eq!(body("()"), ["tail:tuple()"]);
+    assert_eq!(body("(5)"), ["tail:5"]);
+    assert_eq!(body("((5))"), ["tail:5"]);
+}
+
+#[test]
+fn tuple_types_parse_with_unit_and_a_parenthesised_type_unchanged() {
+    assert_eq!(
+        sig("(a: (Int, String), b: (Int,), c: (Int), d: ()) -> (Bool, (Int, Int))"),
+        [
+            "tuple(Int, String)",
+            "tuple(Int)",
+            "Int",
+            "tuple()",
+            "-> tuple(Bool, tuple(Int, Int))"
+        ]
+    );
+    assert_eq!(
+        sig("(v: Vec<(Int, String)>, o: Option<(Int,)>, a: [(Int, Int)])"),
+        [
+            "Vec<tuple(Int, String)>",
+            "Option<tuple(Int)>",
+            "[tuple(Int, Int)]"
+        ]
+    );
+}
+
+#[test]
+fn a_tuple_field_is_named_by_its_index() {
+    assert_eq!(body("t.0"), ["tail:t.<0>"]);
+    assert_eq!(body("t.12"), ["tail:t.<12>"]);
+    assert_eq!(body("f(x).1"), ["tail:f(x).<1>"]);
+}
+
+#[test]
+fn nested_tuple_fields_read_left_to_right() {
+    // The lexer splits `1.0`, `0.1` and `1.10` after a `.`.
+    assert_eq!(body("t.1.0"), ["tail:t.<1>.<0>"]);
+    assert_eq!(body("t.0.1.2"), ["tail:t.<0>.<1>.<2>"]);
+    assert_eq!(body("t.1.10"), ["tail:t.<1>.<10>"]);
+    assert_eq!(body("t.0.1.2.3.4"), ["tail:t.<0>.<1>.<2>.<3>.<4>"]);
+}
+
+#[test]
+fn a_tuple_field_takes_the_postfix_forms_after_it() {
+    assert_eq!(body("t.0.len()"), ["tail:t.<0>.<len>()"]);
+    assert_eq!(body("t.0.await"), ["tail:t.<0>.await"]);
+    assert_eq!(body("t.0.x.1"), ["tail:t.<0>.<x>.<1>"]);
+    assert_eq!(body("t.0 + t.1 * 2"), ["tail:[t.<0> + [t.<1> * 2]]"]);
+}
+
+#[test]
+fn a_tuple_field_is_a_place() {
+    assert_eq!(body("t.0 = 1"), ["tail:t.<0> = 1"]);
+    assert_eq!(body("t.1.0 += 2"), ["tail:t.<1>.<0> += 2"]);
+}
+
+#[test]
+fn a_float_elsewhere_stays_a_float() {
+    assert_eq!(body("t.0 + 1.5"), ["tail:[t.<0> + 1.5]"]);
+}
+
+#[test]
+fn an_index_that_is_not_plain_decimal_digits_is_refused() {
+    for (src, written) in [
+        ("t.01", "1"),
+        ("t.1_0", "10"),
+        ("t.0x1", "1"),
+        ("t.00", "0"),
+        ("t.0b1", "1"),
+    ] {
+        let (errors, tail) = errors_of(src);
+        assert_eq!(
+            errors,
+            [format!(
+                "a tuple index is plain decimal digits, with no leading zero, `_` or prefix: \
+                 write `.{written}`"
+            )],
+            "{src}"
+        );
+        // The access is kept, so the checker and the editor still see it.
+        assert_eq!(tail, [format!("tail:t.<{written}>")], "{src}");
+    }
+}
+
+#[test]
+fn an_index_with_an_exponent_is_a_float_and_refused() {
+    let (errors, _) = errors_of("t.1e5");
+    assert_eq!(
+        errors,
+        ["expected identifier (in field access), found float literal"]
+    );
+}
+
+#[test]
+fn the_newline_rule_is_off_inside_a_tuple_literal() {
+    // A tuple literal is parenthesised, so a `(` or `-` at a line start
+    // inside it continues the line before (5.0 spec §3.5).
+    assert_eq!(
+        body("let t = (\n    f\n    (1),\n    a\n    - b,\n)\nt"),
+        ["let = tuple(f(1), [a - b])", "tail:t"]
+    );
+}
+
+#[test]
+fn a_tuple_on_the_line_after_a_statement_begins_a_new_one() {
+    assert_eq!(
+        body("let x = g()\n(x, x)"),
+        ["let = g()", "tail:tuple(x, x)"]
+    );
+    assert_eq!(body("g();\n(1, 2)"), ["g()", "tail:tuple(1, 2)"]);
+}
````

**Patch `t1-code`:**

<!-- patch t1-code -->

````diff
diff --git a/crates/nova-lexer/src/lib.rs b/crates/nova-lexer/src/lib.rs
index fe9a9fb6188e1fcbdc791da5084ed5d47f831349..9e6d237c391c0306ffec17fe3f97ad095a040414 100644
--- a/crates/nova-lexer/src/lib.rs
+++ b/crates/nova-lexer/src/lib.rs
@@ -142,6 +142,7 @@ impl<'src> Lexer<'src> {
             ));
         }
 
+        let mut tokens = self.split_tuple_indexes(tokens);
         tokens.push(Spanned::new(
             Token::Eof,
             Span::point(self.source.len() as u32, self.file),
@@ -150,6 +151,41 @@ impl<'src> Lexer<'src> {
         (tokens, errors)
     }
 
+    /// After a `.`, a float whose text is digits, `.`, digits becomes an
+    /// integer, a `.` and an integer, so `t.1.0` reads as `(t.1).0` and
+    /// `t.0.1.2` chains (spec
+    /// `docs/superpowers/specs/2026-10-26-phase-5-0-the-language-and-std-a-compiler-needs-design.md`
+    /// §4.2). It splits by the source text, not the value, so `t.1.10` is
+    /// `(t.1).10`. A `..` or any other token before it, an `_`, an exponent,
+    /// or a part too large for an integer leaves the float as it is.
+    fn split_tuple_indexes(&self, tokens: Vec<Spanned<Token>>) -> Vec<Spanned<Token>> {
+        let mut out: Vec<Spanned<Token>> = Vec::with_capacity(tokens.len());
+        for tok in tokens {
+            let after_dot = matches!(out.last(), Some(prev) if prev.value == Token::Dot);
+            if let (true, Token::Float(_)) = (after_dot, &tok.value) {
+                let (start, end) = (tok.span.start as usize, tok.span.end as usize);
+                let text = self.source.get(start..end).unwrap_or("");
+                if let Some((int, frac)) = text.split_once('.') {
+                    let digits = |s: &str| !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit());
+                    if let (true, true, Ok(a), Ok(b)) = (
+                        digits(int),
+                        digits(frac),
+                        int.parse::<i64>(),
+                        frac.parse::<i64>(),
+                    ) {
+                        let dot = start + int.len();
+                        out.push(Spanned::new(Token::Int(a), self.span(start, dot)));
+                        out.push(Spanned::new(Token::Dot, self.span(dot, dot + 1)));
+                        out.push(Spanned::new(Token::Int(b), self.span(dot + 1, end)));
+                        continue;
+                    }
+                }
+            }
+            out.push(tok);
+        }
+        out
+    }
+
     fn span(&self, start: usize, end: usize) -> Span {
         Span::new(start as u32, end as u32, self.file)
     }
diff --git a/crates/nova-lexer/src/tokens.rs b/crates/nova-lexer/src/tokens.rs
index 8c55e8201df905882b5585209451fd4cd9c80f4e..beef00ed5668f6e0d2665dec2e62824e5a058219 100644
--- a/crates/nova-lexer/src/tokens.rs
+++ b/crates/nova-lexer/src/tokens.rs
@@ -26,6 +26,9 @@ use crate::Token;
 pub struct Tokens {
     tokens: Vec<Spanned<Token>>,
     line_break_before: Vec<bool>,
+    /// For each token, whether it is an integer spelled in plain decimal
+    /// digits with no leading zero: what a tuple index `t.N` must be.
+    plain_decimal: Vec<bool>,
 }
 
 impl Tokens {
@@ -43,12 +46,34 @@ impl Tokens {
                 gap.iter().any(|&b| b == b'\n' || b == b'\r')
             })
             .collect();
+        let plain_decimal = tokens
+            .iter()
+            .map(|t| {
+                let text = bytes
+                    .get(t.span.start as usize..t.span.end as usize)
+                    .unwrap_or(&[]);
+                matches!(t.value, Token::Int(_))
+                    && !text.is_empty()
+                    && text.iter().all(u8::is_ascii_digit)
+                    && (text.len() == 1 || text[0] != b'0')
+            })
+            .collect();
         Self {
             tokens,
             line_break_before,
+            plain_decimal,
         }
     }
 
+    /// Whether the token at `index` is an integer written in plain decimal
+    /// digits with no leading zero (`0`, `7`, `12`; not `01`, `1_0` or
+    /// `0x1`): the only spelling a tuple index `t.N` takes (spec
+    /// `docs/superpowers/specs/2026-10-26-phase-5-0-the-language-and-std-a-compiler-needs-design.md`
+    /// §4.2). `false` past the end.
+    pub fn plain_decimal(&self, index: usize) -> bool {
+        self.plain_decimal.get(index).copied().unwrap_or(false)
+    }
+
     /// Whether a line break comes before the token at `index`. `false` past
     /// the end.
     pub fn line_break_before(&self, index: usize) -> bool {
diff --git a/crates/nova-parser/src/grammar.rs b/crates/nova-parser/src/grammar.rs
index d204e55f2a437aed632a1f6f29b0ecb93fc3f1aa..fecc344831f0e9ed34567633d01810072ca3d531 100644
--- a/crates/nova-parser/src/grammar.rs
+++ b/crates/nova-parser/src/grammar.rs
@@ -2109,6 +2109,29 @@ impl<'a> Parser<'a> {
                             .map(|s| s.span)
                             .unwrap_or(cur_span);
                         expr = Spanned::new(Expr::Await(Box::new(expr)), start.merge(end));
+                    } else if let Token::Int(n) = self.peek().clone() {
+                        // `t.N`, a tuple's element (spec
+                        // docs/superpowers/specs/2026-10-26-phase-5-0-the-language-and-std-a-compiler-needs-design.md
+                        // §4.2): plain decimal digits only, so `t.01`,
+                        // `t.1_0` and `t.0x1` are refused, as Rust does.
+                        let span = self.peek_span();
+                        if !self.tokens.plain_decimal(self.pos) {
+                            self.errors.push(ParseError::Custom {
+                                message: format!(
+                                    "a tuple index is plain decimal digits, with no leading \
+                                     zero, `_` or prefix: write `.{n}`"
+                                ),
+                                span,
+                            });
+                        }
+                        self.advance();
+                        expr = Spanned::new(
+                            Expr::Field {
+                                target: Box::new(expr),
+                                field: Spanned::new(n.to_string(), span),
+                            },
+                            start.merge(span),
+                        );
                     } else if !matches!(
                         self.peek(),
                         Token::Ident(_) | Token::SelfLower | Token::SelfUpper
````

### Task 2: Tuple types in the checker and MIR

Spec §4.1, §4.2; the user's decision 3; decisions 1, 5, 12, 20, 21, 31 and 37.

**Files:**
- Modify:
  - `crates/nova-hir/src/lib.rs` (`Ty::Tuple` and its walks,
    `ExprKind::MakeTuple`, `TUPLE_HAS_NO_IMPLS`)
  - `crates/nova-typeck/src/check.rs` (tuple types, literals, `t.N` read and
    write, E0011, E0014's wording, the no-impls note),
    `crates/nova-typeck/src/infer.rs`, `crates/nova-typeck/src/lib.rs`
  - `crates/nova-resolver/src/lib.rs` (`type_full_name`)
  - `crates/nova-mir/src/lib.rs` (`mir_ty`, `mangle_ty`),
    `crates/nova-mir/src/mono.rs`, `crates/nova-mir/src/lower.rs`
- Create: `tests/runtime/tuples.nova`, `tuples.stdout`, `tuples_async.nova`,
  `tuples_async.stdout`
- Test:
  - `crates/nova-typeck/src/check/tuple_tests.rs` (new, 22 tests), and
    `infer.rs`'s three
  - `crates/nova-hir/src/lib.rs`'s five, `crates/nova-mir/src/mono.rs`'s
    two, `crates/nova-mir/tests/lower_tests.rs`'s five
  - `crates/nova-lsp/src/hover.rs` (one test)
  - `crates/nova-cli/tests/run_tests.rs`, `wasm_tests.rs`,
    `crates/nova-driver/tests/index_complete.rs` (the two programs)

- [ ] **Step 1: The tests.** `patch_block t2-tests`. The patch holds:
  - `tuple_tests.rs`: tuple types and literals check, a literal builds
    `MakeTuple` and a field reads its index; `(T)` and `(e)` are not tuples,
    nor `(T,)` its element; the wrong arity or element type E0010; an
    element of unknown type E0011; tuples through generic functions,
    closures, function types and a record's field; an index past the end, a
    name on a tuple and an index on another type E0014; an index on a value
    of unknown type one E0011; an element written through a mutable binding;
    `==` and interpolation E0013 with the note, a record's `==` without it;
    no impl names a tuple; a method on a tuple E0014; `extern "C"` refuses
    one; a tuple displays as written. The module is declared in `check.rs`.
  - `infer.rs`: unification inside tuples, element by element; tuples of
    another arity or element type not unifying; the occurs check inside a
    tuple. **`nova-hir`:** `subst` and the queries inside a tuple; a tuple
    has no head; `match_pattern`, `normalize` and the overlap check element
    by element. **`mono.rs`:** distinct tuple types mangle apart; `mir_ty`
    maps a tuple to `Ptr`. **`lower_tests.rs`:** a tuple lowers to
    `MakeRecord`, an element write to `SetField`; a generic function at
    distinct tuple types is distinct instances; a tuple `Map` key is E0013
    with the note, and a tuple of one named with its comma there, `(Int,)`
    (decision 41).
  - `hover.rs`: a tuple local hovers with its type, `let q: (Int, Int)`.
  - **The programs:** `tuples`, generics, closures, nesting five deep, a
    12-wide tuple, writes, and every machine class as an element; and
    `tuples_async`, tuples of pointers held across `.await`s, its tail tuple
    after a statement written with a `;`. Each has its `.stdout`, a test in
    `run_tests.rs` under `run`, `build` and GC stress, an entry in
    `wasm_tests.rs`'s `gate!`, and `index_complete.rs`'s counts, 171
    programs and 205 files.
- [ ] **Step 2: Run, and see the tests fail.** First the build, which ends
  with `Finished`.
  - `T tuple`, the library tests of Task 2: `nova-hir`'s, `infer.rs`'s,
    `tuple_tests.rs`, mono's and the hover's. Expected: it does not compile,
    ``error[E0599]: no variant, associated function, or constant named `Tuple` found for enum `Ty` in the current scope``
    and
    ``error: could not compile `nova-hir` (lib test) due to 1 previous error``
    (decision 38).
  - `cargo test --locked --workspace --test lower_tests`. Expected: it does
    not compile,
    ``error[E0425]: cannot find value `TUPLE_HAS_NO_IMPLS` in crate `nova_hir` ``.
  - `R tuples_run tuples_async_runs`. Expected: `0 passed; 2 failed`,
    `nova run` exiting 1 with "E0900: tuple types are not supported yet" and
    "tuple expressions are not supported yet".
  - `W gate::tuples gate::tuples_async every_runtime_program_is_in_the_gate_or_left_out`.
    Expected: `1 passed; 2 failed`, the two programs, as above; the gate's
    own check passes, each program being in it.
  - `X`. Expected: `0 passed; 1 failed`:
    `every_name_in_the_corpora_is_indexed`, "tests/runtime/tuples.nova does
    not check cleanly: ["E0900 tuple types are not supported yet", …]".
- [ ] **Step 3: The code.** `patch_block t2-code`:
  - **`nova-hir`:** `Ty::Tuple(Vec<Ty>)`, with an arm in each walk of
    decision 12; `ExprKind::MakeTuple { elems }`; `FieldGet` and `FieldSet`
    serve a tuple's element by index; `TUPLE_HAS_NO_IMPLS`, the note.
  - **`check.rs`:** `convert_ty` takes `ast::Type::Tuple`; a tuple literal
    checks its elements left to right into `MakeTuple`;
    `record_field_index_and_ty` reads `t.N` off a `Ty::Tuple` by the index's
    digits, so `t.N` and `t.N = x` and `t.N += x` go the record field's way;
    `tuple_index_on_unknown` gives E0011 once and binds the variable to the
    error type; E0014's "no field `2` on `(Int, Int)`"; and
    `note_tuple_has_no_impls` after each E0013 on a tuple. **`infer.rs`:**
    `apply`, `occurs` and `unify`, which checks the arity. **`lib.rs`:**
    `display_ty` and `display_ty_named` through `tuple_text`, `(Int,)` for a
    tuple of one.
  - **The resolver:** `type_full_name` names a tuple type argument
    `Tup<n>_<elements>`.
  - **MIR:** `mir_ty` maps a tuple to `Ptr`; `mangle_ty` to `T<elements>E`;
    mono's `type_name` writes `(Int, String)`, and its bound note carries
    the no-impls note; the specializer maps `MakeTuple`; `lower.rs` lowers
    `MakeTuple` as `MakeRecord`.
- [ ] **Step 4: Run.** First the build, which ends with `Finished`.
  - `T tuple`. Expected: every `test result` `ok`, 33 passing in all:
    `nova-hir`'s 5, `nova-lsp`'s 1 (the hover), `nova-mir`'s 2 (mono's) and
    `nova-typeck`'s 25 (`tuple_tests.rs`'s 22 and `infer.rs`'s 3), the other
    libraries none.
  - `cargo test --locked --workspace --test lower_tests`. Expected:
    `66 passed; 0 failed`.
  - `R tuples_run tuples_async_runs`. Expected: `2 passed; 0 failed`.
  - `W gate::tuples gate::tuples_async every_runtime_program_is_in_the_gate_or_left_out`.
    Expected: `3 passed; 0 failed`.
  - `X`. Expected: `1 passed; 0 failed`, in about 25 seconds.
  - `S`, as in Task 1 (about 4 min 40 s). Expected: `2602 1 10`: Task 1's
    figures and Task 2's 42 tests, the one failure the IPv6 test.
  - `G`. Expected:
    `release golden: 133 passed, 9 excluded, Ubuntu clang version 18.1.3 (1ubuntu1)`
    and `1 passed`, in about 26 s: the two programs join the walk.
- [ ] **Step 5: The checks,** as the conventions list them.
- [ ] **Step 6: Commit** `typeck, mir: tuple types, literals and t.N`.

**Patch `t2-tests`:**

<!-- patch t2-tests -->

````diff
diff --git a/crates/nova-cli/tests/run_tests.rs b/crates/nova-cli/tests/run_tests.rs
index 9e1306ecceb4949d041b0f65a083e7cd7d967e3a..ce9c31381c18a1cc315f7b7a7d027f9fcfe9b895 100644
--- a/crates/nova-cli/tests/run_tests.rs
+++ b/crates/nova-cli/tests/run_tests.rs
@@ -12995,6 +12995,21 @@ fn string_concat_runs() {
     golden_three_ways("string_concat", true);
 }
 
+/// Tuple types, literals and fields (5.0 spec §4.1, §4.2): generics,
+/// closures, nesting, writes, and every machine class as an element, also
+/// under GC stress, since its tuples hold strings, records and closures.
+#[test]
+fn tuples_run() {
+    golden_three_ways("tuples", true);
+}
+
+/// Tuples that hold pointers, across `.await`s, under GC stress (5.0 spec
+/// §4.1, §7.3).
+#[test]
+fn tuples_async_runs() {
+    golden_three_ways("tuples_async", true);
+}
+
 /// E0102 (5.0 spec §3.7) is a warning: it prints, and the program still
 /// builds and runs, its own `Number` meaning its own variant.
 #[test]
diff --git a/crates/nova-cli/tests/wasm_tests.rs b/crates/nova-cli/tests/wasm_tests.rs
index 2e4958a55f4ee125602f9ed09745ccac26f50d6b..3dba9640f843318900ea00370ab8fba62fc6c9f1 100644
--- a/crates/nova-cli/tests/wasm_tests.rs
+++ b/crates/nova-cli/tests/wasm_tests.rs
@@ -1954,6 +1954,10 @@ mod gate {
         try_async,
         compound_assign_places,
         string_concat,
+        // Tuples: anonymous records, through MIR's records, so WASM needs
+        // nothing (5.0 spec §4.1).
+        tuples,
+        tuples_async,
     );
 
     /// The programs a test of their own in this module checks, by its name.
diff --git a/crates/nova-driver/tests/index_complete.rs b/crates/nova-driver/tests/index_complete.rs
index 2239ea5602e76608cd1f5b2f19abbdb74be23bf8..f8666d5f4488a54162594b3394817c6171c97aa2 100644
--- a/crates/nova-driver/tests/index_complete.rs
+++ b/crates/nova-driver/tests/index_complete.rs
@@ -246,8 +246,9 @@ fn every_name_in_the_corpora_is_indexed() {
     // `http_responses`, `http_server_wire`, `fs_rename`, `crypto_pbkdf2`,
     // `crypto_pbkdf2_refused` and `bytes_hex`, and `07-fullstack-blog`;
     // 159 runtime programs on 2026-10-26, after 5.0a added `try_operator`,
-    // `try_async`, `compound_assign_places` and `string_concat`.
-    assert_eq!(programs.len(), 169, "{programs:#?}");
+    // `try_async`, `compound_assign_places` and `string_concat`; 161 on
+    // 2026-10-27, after 5.0b added `tuples` and `tuples_async`.
+    assert_eq!(programs.len(), 171, "{programs:#?}");
     let mut files = 0;
     let mut failures: Vec<String> = Vec::new();
     for (k, path) in programs.iter().enumerate() {
@@ -274,11 +275,11 @@ fn every_name_in_the_corpora_is_indexed() {
             every_name_is_covered(&a, file, &mut failures);
         }
     }
-    // 9 + 159 + 3 files of user code, 05-geometry's library's 2, the 8
+    // 9 + 161 + 3 files of user code, 05-geometry's library's 2, the 8
     // modules 07-fullstack-blog's client reaches from its `main.nova` (not
     // the server's 4, which only `server.nova` and `lib.nova` reach), and
     // native std's 22, `std/ui/ssr` among them.
-    assert_eq!(files, 203);
+    assert_eq!(files, 205);
     failures.sort();
     failures.dedup();
     assert!(
diff --git a/crates/nova-hir/src/lib.rs b/crates/nova-hir/src/lib.rs
index 60a857845bc065a30e88606d449f1f95d2bb729a..b7acfcc79c73c000402bfee461dd7c62d694b35f 100644
--- a/crates/nova-hir/src/lib.rs
+++ b/crates/nova-hir/src/lib.rs
@@ -1885,4 +1885,80 @@ mod tests {
         }
         assert_eq!(normalize_ty(&proj(self_ty, 100), &impls), Ok(expected));
     }
+
+    // --- `Ty::Tuple` (5.0 spec §4.1): each walk over `Ty` reaches inside a
+    // tuple, and two tuple types are the same when their element types are.
+
+    fn tup(elems: Vec<Ty>) -> Ty {
+        Ty::Tuple(elems)
+    }
+
+    #[test]
+    fn subst_and_the_queries_reach_inside_a_tuple() {
+        let t = tup(vec![Ty::Param(0), tup(vec![Ty::Param(1), Ty::Int])]);
+        assert_eq!(
+            t.subst(&[Ty::Bool, Ty::String]),
+            tup(vec![Ty::Bool, tup(vec![Ty::String, Ty::Int])])
+        );
+        assert!(t.mentions_param(1));
+        assert!(!t.mentions_param(2));
+        assert!(t.has_params());
+        assert!(!tup(vec![Ty::Int]).has_params());
+        assert!(tup(vec![Ty::Int, Ty::Var(3)]).has_vars());
+        assert!(!tup(vec![Ty::Int]).has_vars());
+        assert!(tup(vec![Ty::Int, proj(Ty::Param(0), ITEM)]).has_assoc());
+        assert!(!tup(vec![Ty::Int]).has_assoc());
+        assert!(tup(vec![tup(vec![Ty::Error])]).has_error());
+        assert!(!tup(vec![Ty::Int]).has_error());
+    }
+
+    #[test]
+    fn a_tuple_has_no_head() {
+        // So no impl can name one, and no method resolves on one.
+        assert_eq!(tup(vec![Ty::Int, Ty::Int]).head(), None);
+        assert_eq!(tup(vec![Ty::Int]).head(), None);
+    }
+
+    #[test]
+    fn match_pattern_matches_a_tuple_element_by_element() {
+        let pat = tup(vec![Ty::Param(0), Ty::Param(0)]);
+        let mut out = Vec::new();
+        assert!(pat.match_pattern(&tup(vec![Ty::Int, Ty::Int]), &mut out));
+        assert_eq!(out, vec![Some(Ty::Int)]);
+        let mut out = Vec::new();
+        assert!(!pat.match_pattern(&tup(vec![Ty::Int, Ty::Bool]), &mut out));
+        // Another arity, or a one-tuple against its element, never matches.
+        let mut out = Vec::new();
+        assert!(!pat.match_pattern(&tup(vec![Ty::Int, Ty::Int, Ty::Int]), &mut out));
+        let mut out = Vec::new();
+        assert!(!tup(vec![Ty::Param(0)]).match_pattern(&Ty::Int, &mut out));
+    }
+
+    #[test]
+    fn normalize_reaches_a_projection_inside_a_tuple() {
+        let impls = [imp(rec(1, vec![]), 0, vec![(DefId(ITEM), Ty::Int)])];
+        assert_eq!(
+            normalize_ty(&tup(vec![Ty::Bool, proj(rec(1, vec![]), ITEM)]), &impls),
+            Ok(tup(vec![Ty::Bool, Ty::Int]))
+        );
+    }
+
+    #[test]
+    fn overlap_compares_tuples_element_by_element() {
+        // `Box<(T, Int)>` and `Box<(Bool, U)>` share `Box<(Bool, Int)>`.
+        let a = rec(1, vec![tup(vec![Ty::Param(0), Ty::Int])]);
+        let b = rec(1, vec![tup(vec![Ty::Bool, Ty::Param(0)])]);
+        assert!(self_types_overlap(&a, 1, &b, 1));
+        // `Box<(Int, Int)>` and `Box<(Int, Bool)>` share nothing, nor two
+        // arities.
+        let c = rec(1, vec![tup(vec![Ty::Int, Ty::Int])]);
+        let d = rec(1, vec![tup(vec![Ty::Int, Ty::Bool])]);
+        assert!(!self_types_overlap(&c, 0, &d, 0));
+        let e = rec(1, vec![tup(vec![Ty::Int, Ty::Int, Ty::Int])]);
+        assert!(!self_types_overlap(&c, 0, &e, 0));
+        // The occurs check reaches inside a tuple: `T` against `(T, Int)`.
+        let f = rec(1, vec![Ty::Param(0), Ty::Param(0)]);
+        let g = rec(1, vec![Ty::Param(0), tup(vec![Ty::Param(0), Ty::Int])]);
+        assert!(!self_types_overlap(&f, 1, &g, 1));
+    }
 }
diff --git a/crates/nova-lsp/src/hover.rs b/crates/nova-lsp/src/hover.rs
index a99fcbd71251287d38d96c6330da839edf0ddefa..162c7ae4b318c5af3b09c2a20cdd1ce509451e3b 100644
--- a/crates/nova-lsp/src/hover.rs
+++ b/crates/nova-lsp/src/hover.rs
@@ -270,6 +270,41 @@ mod tests {
         assert!(hover(&answer, text.find("class:").unwrap() as u32).is_none());
     }
 
+    /// A local of a tuple type hovers with the type as it is written, and
+    /// a field of one hovers with nothing to go to (5.0 spec §4.1, §4.2).
+    #[test]
+    fn a_tuple_local_hovers_with_its_type() {
+        let text = "fn div_mod(a: Int, b: Int) -> (Int, Int) { (a / b, a % b) }\n\n\
+                    fn main() {\n    let q = div_mod(7, 2)\n    let one = (q,)\n    \
+                    println(\"${q.0} ${one.0.1}\")\n}\n";
+        let path = std::env::temp_dir()
+            .join("nova-lsp-hover-tuple")
+            .join("main.nova");
+        let overlay = crate::workspace::Overlay::default().with(&path, text.to_string());
+        let options = crate::analysis::options(None, true);
+        let answer = crate::analysis::answering(&path, &overlay, &options).expect("an answer");
+        let shown = |at: usize| {
+            let lsp::HoverContents::Markup(found) =
+                hover(&answer, at as u32).expect("a hover").contents
+            else {
+                panic!("not Markdown");
+            };
+            found.value
+        };
+        assert_eq!(
+            shown(text.find("q.0").unwrap()),
+            "```nova\nlet q: (Int, Int)\n```"
+        );
+        assert_eq!(
+            shown(text.find("one.0").unwrap()),
+            "```nova\nlet one: ((Int, Int),)\n```"
+        );
+        assert_eq!(
+            shown(text.find("div_mod(7").unwrap()),
+            "```nova\nfn div_mod(a: Int, b: Int) -> (Int, Int)\n```"
+        );
+    }
+
     #[test]
     fn an_item_line_keeps_pub_async_and_where_and_stops_at_its_body() {
         let text = "    pub async fn f<T>(x: T) -> T\n    where T: Show\n    { x }\n";
diff --git a/crates/nova-mir/src/mono.rs b/crates/nova-mir/src/mono.rs
index 29014f9e49022907b78fdd11a69f3b0630b60f82..be3d9f8ce6c43cba4917546e9bd95f8e58e947e3 100644
--- a/crates/nova-mir/src/mono.rs
+++ b/crates/nova-mir/src/mono.rs
@@ -1147,6 +1147,56 @@ mod tests {
         }
     }
 
+    #[test]
+    fn mangle_ty_gives_distinct_tuple_types_distinct_strings() {
+        // A shared mangling has shipped as a miscompile twice (see above):
+        // every pair of these tuple types, which differ in arity, element
+        // type, nesting or element order, must mangle apart, and apart from
+        // the other one-argument constructors of the same element (5.0 spec
+        // §4.1, §10 risk 3).
+        use hir::Ty;
+        let t = |e: Vec<Ty>| Ty::Tuple(e);
+        let types = [
+            t(vec![Ty::Int]),
+            t(vec![Ty::Int, Ty::Int]),
+            t(vec![Ty::Int, Ty::Int, Ty::Int]),
+            t(vec![Ty::Int, Ty::Bool]),
+            t(vec![Ty::Bool, Ty::Int]),
+            t(vec![Ty::Int, t(vec![Ty::Int, Ty::Int])]),
+            t(vec![t(vec![Ty::Int, Ty::Int]), Ty::Int]),
+            t(vec![t(vec![Ty::Int]), Ty::Int]),
+            t(vec![t(vec![Ty::Int, Ty::Int])]),
+            t(vec![Ty::Array(Box::new(Ty::Int))]),
+            Ty::Array(Box::new(t(vec![Ty::Int]))),
+            Ty::Array(Box::new(Ty::Int)),
+            Ty::Future(Box::new(Ty::Int)),
+            Ty::Int,
+            Ty::Unit,
+        ];
+        let names: Vec<String> = types.iter().map(crate::mangle_ty).collect();
+        for (i, a) in names.iter().enumerate() {
+            for (j, b) in names.iter().enumerate().skip(i + 1) {
+                assert_ne!(a, b, "{:?} and {:?}", types[i], types[j]);
+            }
+        }
+        // And within `mangle`'s `_`-joined argument list.
+        let f = |args: &[Ty]| crate::mangle(DefId(3), "f", args);
+        assert_ne!(
+            f(&[t(vec![Ty::Int, Ty::Int]), Ty::Int]),
+            f(&[Ty::Int, t(vec![Ty::Int, Ty::Int])])
+        );
+    }
+
+    #[test]
+    fn mir_ty_maps_a_tuple_to_ptr() {
+        // A tuple is a heap record; `MirTy::Unit` would drop it from a
+        // signature, as the projection bug did.
+        assert_eq!(
+            crate::mir_ty(&hir::Ty::Tuple(vec![hir::Ty::Int])),
+            crate::MirTy::Ptr
+        );
+    }
+
     #[test]
     fn mir_ty_maps_future_to_ptr() {
         // A future value is the fat pointer { poll_code, state_ptr }.
diff --git a/crates/nova-mir/tests/lower_tests.rs b/crates/nova-mir/tests/lower_tests.rs
index 993687381f26d2297dc52d9d0ab7bdf84d96e5a6..d22bc0e2f696e4fbb2f439b1a9ba3e49945ce733 100644
--- a/crates/nova-mir/tests/lower_tests.rs
+++ b/crates/nova-mir/tests/lower_tests.rs
@@ -2215,3 +2215,134 @@ fn state_builtins_lower(kind: &str, rts: [nova_mir::RtFunc; 3]) {
         );
     }
 }
+
+// --- Tuples (5.0 spec §4.1): anonymous records ---
+
+/// A tuple lowers as a record does: one `MakeRecord` of its elements, with
+/// each element's machine class, and `t.N` is a `RecordField` at index `N`.
+/// So no backend sees a tuple.
+#[test]
+fn a_tuple_lowers_to_a_record() {
+    let mir = mir_for(
+        "fn f(a: Int, s: String) -> Bool {\n    let t = (a, s, true, 1.5)\n    t.2\n}\n\
+         fn main() { println(\"${f(1, \"s\")}\") }",
+    );
+    let stmts = stmts_of(&mir, "f");
+    let made: Vec<Vec<nova_mir::MirTy>> = stmts
+        .iter()
+        .filter_map(|s| match s {
+            nova_mir::Stmt::MakeRecord { fields, .. } => {
+                Some(fields.iter().map(|(_, ty)| *ty).collect())
+            }
+            _ => None,
+        })
+        .collect();
+    use nova_mir::MirTy;
+    assert_eq!(
+        made,
+        [vec![MirTy::I64, MirTy::Ptr, MirTy::I8, MirTy::F64]],
+        "{stmts:?}"
+    );
+    let reads: Vec<(u32, MirTy)> = stmts
+        .iter()
+        .filter_map(|s| match s {
+            nova_mir::Stmt::RecordField { index, ty, .. } => Some((*index, *ty)),
+            _ => None,
+        })
+        .collect();
+    assert_eq!(reads, [(2, MirTy::I8)], "{stmts:?}");
+}
+
+/// A write to an element is a `SetField` at its index.
+#[test]
+fn a_tuple_element_write_lowers_to_set_field() {
+    let mir = mir_for(
+        "fn f() -> Int {\n    let mut t = (1, \"s\")\n    t.0 = 5\n    t.1 = \"x\"\n    t.0\n}\n\
+         fn main() { println(\"${f()}\") }",
+    );
+    let stmts = stmts_of(&mir, "f");
+    let sets: Vec<(u32, nova_mir::MirTy)> = stmts
+        .iter()
+        .filter_map(|s| match s {
+            nova_mir::Stmt::SetField { index, ty, .. } => Some((*index, *ty)),
+            _ => None,
+        })
+        .collect();
+    assert_eq!(
+        sets,
+        [(0, nova_mir::MirTy::I64), (1, nova_mir::MirTy::Ptr)],
+        "{stmts:?}"
+    );
+}
+
+/// A generic function instantiated at two tuple types, and at tuple types
+/// whose nesting differs, is three instances with three symbols: one shared
+/// mangling would send every call to the first instance's code, the shape
+/// of two earlier miscompiles (5.0 spec §10 risk 3).
+#[test]
+fn a_generic_function_at_distinct_tuple_types_is_distinct_instances() {
+    let mir = mir_for(
+        "fn first<T>(x: T) -> T { x }\n\
+         fn main() {\n    let a = first((1, (2, 3)))\n    let b = first(((1, 2), 3))\n    \
+         let c = first((true, 1))\n    println(\"${a.0} ${b.1} ${c.1}\")\n}",
+    );
+    let mut names: Vec<&str> = function_names(&mir)
+        .into_iter()
+        .filter(|n| n.starts_with("first."))
+        .collect();
+    names.sort();
+    names.dedup();
+    assert_eq!(names.len(), 3, "{names:?}");
+}
+
+/// A tuple is not a `Map` key: it has no `Hash` and no `Eq` (decision 3),
+/// so instantiating `Map`'s methods with one is E0013, with the note that
+/// says why.
+#[test]
+fn a_tuple_map_key_is_e0013_with_a_note() {
+    let src = "fn main() {\n    let mut m: Map<(Int, Int), Int> = Map::new()\n    \
+               let p = m.insert((1, 2), 3)\n    println(\"${m.len()}\")\n}";
+    let file_id = FileId::DUMMY;
+    let (tokens, _) = lex(src, file_id);
+    let (ast, _) = parse(&tokens, file_id);
+    let resolved = resolve(&ast.expect("an AST"));
+    let checked = check(&resolved.file, &resolved.definitions);
+    assert!(checked.diagnostics.is_empty(), "{:?}", checked.diagnostics);
+    let diags = match lower_module(&checked.module) {
+        Ok(_) => panic!("a tuple key lowered"),
+        Err(d) => d,
+    };
+    assert!(!diags.is_empty());
+    for d in &diags {
+        assert_eq!(d.code, "E0013", "{d:?}");
+        assert!(d.message.contains("`(Int, Int): "), "{d:?}");
+        assert_eq!(d.notes, [nova_hir::TUPLE_HAS_NO_IMPLS], "{d:?}");
+    }
+    let traits: Vec<bool> = ["Hash", "Eq"]
+        .iter()
+        .map(|t| diags.iter().any(|d| d.message.contains(&format!(": {t}`"))))
+        .collect();
+    assert_eq!(traits, [true, true], "{diags:?}");
+}
+
+/// mono names a tuple of one with its comma in that E0013, as the checker
+/// and the parser write it, so the message names the type the program has.
+#[test]
+fn a_tuple_of_one_in_a_bound_message_keeps_its_comma() {
+    let src = "fn main() {\n    let mut m: Map<(Int,), Int> = Map::new()\n    \
+               let p = m.insert((1,), 3)\n    println(\"${m.len()}\")\n}";
+    let file_id = FileId::DUMMY;
+    let (tokens, _) = lex(src, file_id);
+    let (ast, _) = parse(&tokens, file_id);
+    let resolved = resolve(&ast.expect("an AST"));
+    let checked = check(&resolved.file, &resolved.definitions);
+    assert!(checked.diagnostics.is_empty(), "{:?}", checked.diagnostics);
+    let diags = match lower_module(&checked.module) {
+        Ok(_) => panic!("a tuple key lowered"),
+        Err(d) => d,
+    };
+    assert!(!diags.is_empty());
+    for d in &diags {
+        assert!(d.message.contains("`(Int,): "), "{d:?}");
+    }
+}
diff --git a/crates/nova-typeck/src/check.rs b/crates/nova-typeck/src/check.rs
index edd4bdfd0c63c694e2fb81d3ec828348254d5dc4..0e65e1da71f6553d82f876b2ca198334389d3b3f 100644
--- a/crates/nova-typeck/src/check.rs
+++ b/crates/nova-typeck/src/check.rs
@@ -22,6 +22,8 @@ use crate::{
 mod fixes;
 #[cfg(test)]
 mod lang_tests;
+#[cfg(test)]
+mod tuple_tests;
 mod view;
 
 use fixes::{Binding, Need};
diff --git a/crates/nova-typeck/src/check/tuple_tests.rs b/crates/nova-typeck/src/check/tuple_tests.rs
new file mode 100644
index 0000000000000000000000000000000000000000..c147e728a38e45627842c6519de7c2d0ce92235e
--- /dev/null
+++ b/crates/nova-typeck/src/check/tuple_tests.rs
@@ -0,0 +1,320 @@
+//! The checker's tests of 5.0b's tuples (spec
+//! `docs/superpowers/specs/2026-10-26-phase-5-0-the-language-and-std-a-compiler-needs-design.md`
+//! §4.1, §4.2): tuple types and literals, `t.N`, inference through
+//! generics, and the codes a tuple meets: E0010, E0011, E0013 and E0014.
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
+/// The one error of `src`, which has `code`.
+fn only(src: &str, code: &str) -> Diagnostic {
+    let all = errors(src);
+    assert_eq!(all.len(), 1, "one error in {all:#?}");
+    assert_eq!(all[0].code, code, "{all:#?}");
+    all[0].clone()
+}
+
+fn clean(src: &str) {
+    let all = checked(src).diagnostics;
+    assert!(all.is_empty(), "{all:#?}");
+}
+
+/// Every expression of `name`'s body, outermost first.
+fn exprs<'m>(module: &'m hir::Module, name: &str) -> Vec<&'m hir::Expr> {
+    fn walk<'e>(e: &'e hir::Expr, out: &mut Vec<&'e hir::Expr>) {
+        out.push(e);
+        for c in child_exprs(e) {
+            walk(c, out);
+        }
+    }
+    let f = module
+        .functions
+        .iter()
+        .find(|f| f.name == name)
+        .expect("the function");
+    let mut out = Vec::new();
+    walk(&f.body, &mut out);
+    out
+}
+
+// --- Types and literals ---
+
+#[test]
+fn tuple_types_and_literals_check() {
+    clean(
+        "fn div_mod(a: Int, b: Int) -> (Int, Int) { (a / b, a % b) }\n\
+         fn main() {\n    let p: (String, Int) = (\"x\", 1)\n    let one: (Int,) = (5,)\n    \
+         let q = div_mod(7, 2)\n    let n: ((Int, Int), (Bool, (Char, Float))) = ((1, 2), (true, ('c', 1.5)))\n    \
+         let v: Vec<(Int, String)> = Vec::new()\n    let o: Option<(Int, Bool)> = Some((1, true))\n    \
+         let u: ((), Int) = ((), 1)\n}\n",
+    );
+}
+
+#[test]
+fn a_tuple_literal_builds_a_tuple_and_a_field_reads_its_index() {
+    let r = checked(
+        "fn f() -> Int {\n    let t = (1, \"s\", true)\n    t.2\n    t.0\n}\nfn main() {}\n",
+    );
+    let all = exprs(&r.module, "f");
+    let made: Vec<&Ty> = all
+        .iter()
+        .filter(|e| matches!(e.kind, hir::ExprKind::MakeTuple { .. }))
+        .map(|e| &e.ty)
+        .collect();
+    assert_eq!(made, [&Ty::Tuple(vec![Ty::Int, Ty::String, Ty::Bool])]);
+    let reads: Vec<(u32, &Ty)> = all
+        .iter()
+        .filter_map(|e| match &e.kind {
+            hir::ExprKind::FieldGet { index, .. } => Some((*index, &e.ty)),
+            _ => None,
+        })
+        .collect();
+    assert_eq!(reads, [(2, &Ty::Bool), (0, &Ty::Int)]);
+}
+
+#[test]
+fn a_parenthesised_type_and_expression_are_not_tuples() {
+    clean("fn f(x: (Int)) -> Int { (x) }\nfn main() {}\n");
+    let r = checked("fn f() -> Int { ((5)) }\nfn main() {}\n");
+    assert!(exprs(&r.module, "f")
+        .iter()
+        .all(|e| !matches!(e.kind, hir::ExprKind::MakeTuple { .. })));
+}
+
+#[test]
+fn a_tuple_of_the_wrong_arity_is_e0010() {
+    let d = only(
+        "fn main() {\n    let p: (Int, Int) = (1, 2, 3)\n}\n",
+        "E0010",
+    );
+    assert!(d.message.contains("`(Int, Int, Int)`"), "{d:?}");
+    assert!(d.message.contains("`(Int, Int)`"), "{d:?}");
+}
+
+#[test]
+fn a_tuple_of_the_wrong_element_type_is_e0010() {
+    let d = only(
+        "fn want(p: (Int, Int)) -> Int { p.0 }\nfn main() {\n    let x = want((\"s\", 2))\n}\n",
+        "E0010",
+    );
+    assert!(d.message.contains("`(String, Int)`"), "{d:?}");
+}
+
+#[test]
+fn a_tuple_of_one_is_not_its_element() {
+    let d = only("fn main() {\n    let p: Int = (5,)\n}\n", "E0010");
+    assert!(d.message.contains("`(Int,)`"), "{d:?}");
+}
+
+#[test]
+fn an_element_whose_type_cannot_be_inferred_is_e0011() {
+    let all = errors("fn main() {\n    let p = ([], 1)\n}\n");
+    assert!(
+        !all.is_empty() && all.iter().all(|d| d.code == "E0011"),
+        "{all:#?}"
+    );
+}
+
+// --- Generics, closures, and other types' arguments ---
+
+#[test]
+fn tuples_infer_through_generic_functions() {
+    clean(
+        "fn swap<A, B>(p: (A, B)) -> (B, A) { (p.1, p.0) }\n\
+         fn first<A, B>(p: (A, B)) -> A { p.0 }\n\
+         fn main() {\n    let s: (Bool, Int) = swap((1, true))\n    let n: Int = first(swap((\"x\", 3)))\n}\n",
+    );
+    let d = only(
+        "fn swap<A, B>(p: (A, B)) -> (B, A) { (p.1, p.0) }\n\
+         fn main() {\n    let s: (Int, Bool) = swap((1, true))\n}\n",
+        "E0010",
+    );
+    assert!(d.message.contains("`(Bool, Int)`"), "{d:?}");
+}
+
+#[test]
+fn tuples_flow_through_closures_and_function_types() {
+    clean(
+        "fn apply(f: fn((Int, Int)) -> Int, p: (Int, Int)) -> Int { f(p) }\n\
+         fn main() {\n    let mul = |t: (Int, Int)| t.0 * t.1\n    let n = apply(mul, (2, 3))\n    \
+         let base = (1, 2)\n    let add = |x: Int| (base.0 + x, base.1 + x)\n    let r: (Int, Int) = add(1)\n}\n",
+    );
+}
+
+#[test]
+fn a_record_field_may_hold_a_tuple() {
+    clean(
+        "record Holder { pair: (String, Int) }\n\
+         fn main() {\n    let mut h = Holder { pair: (\"h\", 1) }\n    let n: Int = h.pair.1\n    h.pair = (\"g\", 2)\n}\n",
+    );
+}
+
+// --- `t.N` ---
+
+#[test]
+fn an_index_past_the_end_is_e0014() {
+    let d = only(
+        "fn main() {\n    let t = (1, 2)\n    let x = t.2\n}\n",
+        "E0014",
+    );
+    assert_eq!(d.message, "no field `2` on `(Int, Int)`");
+}
+
+#[test]
+fn a_name_on_a_tuple_is_e0014() {
+    let d = only(
+        "fn main() {\n    let t = (1, 2)\n    let x = t.x\n}\n",
+        "E0014",
+    );
+    assert_eq!(d.message, "no field `x` on `(Int, Int)`");
+}
+
+#[test]
+fn an_index_on_a_type_that_is_not_a_tuple_is_e0014() {
+    let d = only("fn main() {\n    let k = 5\n    let x = k.0\n}\n", "E0014");
+    assert_eq!(d.message, "cannot access field `0` on `Int`");
+    let d = only(
+        "record P { x: Int }\nfn main() {\n    let p = P { x: 1 }\n    let y = p.0\n}\n",
+        "E0014",
+    );
+    assert_eq!(d.message, "no field `0` on record `P`");
+}
+
+#[test]
+fn an_index_on_a_value_of_unknown_type_is_one_e0011() {
+    let d = only("fn main() {\n    let f = |p| p.0\n}\n", "E0011");
+    assert!(d.message.contains("`.0`"), "{d:?}");
+    let d = only(
+        "fn main() {\n    let f = |mut p| {\n        p.1 = 2\n    }\n}\n",
+        "E0011",
+    );
+    assert!(d.message.contains("`.1`"), "{d:?}");
+}
+
+#[test]
+fn an_element_is_written_through_a_mutable_binding() {
+    clean(
+        "fn main() {\n    let mut t = (1, \"a\")\n    t.0 = 2\n    t.0 += 3\n    t.1 += \"b\"\n    \
+         let mut n = ((1, 2), 3)\n    n.0.1 = 4\n}\n",
+    );
+    let d = only("fn main() {\n    let t = (1, 2)\n    t.0 = 3\n}\n", "E0060");
+    assert!(d.message.contains("`t`"), "{d:?}");
+    let d = only(
+        "fn main() {\n    let mut t = (1, 2)\n    t.0 = \"s\"\n}\n",
+        "E0010",
+    );
+    assert!(d.message.contains("field `0` has type `Int`"), "{d:?}");
+    let d = only(
+        "fn main() {\n    let mut t = (1, 2)\n    t.5 = 1\n}\n",
+        "E0014",
+    );
+    assert_eq!(d.message, "no field `5` on `(Int, Int)`");
+}
+
+// --- No impls (decision 3) ---
+
+#[test]
+fn equality_on_tuples_is_e0013_with_a_note() {
+    let d = only(
+        "fn main() {\n    let a = (1, 2)\n    let b = a == (1, 2)\n}\n",
+        "E0013",
+    );
+    assert!(d.message.contains("`(Int, Int)`"), "{d:?}");
+    assert_eq!(d.notes, [hir::TUPLE_HAS_NO_IMPLS]);
+}
+
+#[test]
+fn interpolating_a_tuple_is_e0013_with_a_note() {
+    let d = only(
+        "fn main() {\n    let a = (1, 2)\n    println(\"${a}\")\n}\n",
+        "E0013",
+    );
+    assert!(d.message.contains("`(Int, Int)`"), "{d:?}");
+    assert_eq!(d.notes, [hir::TUPLE_HAS_NO_IMPLS]);
+}
+
+#[test]
+fn equality_on_a_record_has_no_tuple_note() {
+    let d = only(
+        "record P { x: Int }\nfn main() {\n    let b = P { x: 1 } == P { x: 1 }\n}\n",
+        "E0013",
+    );
+    assert!(d.notes.is_empty(), "{d:?}");
+}
+
+#[test]
+fn no_impl_can_name_a_tuple() {
+    let all = errors(
+        "trait Show { fn show(self) -> String }\n\
+         impl Show for (Int, Int) { fn show(self) -> String { \"p\" } }\n\
+         impl (Int, Bool) { fn f(self) -> Int { 1 } }\nfn main() {}\n",
+    );
+    let codes: Vec<&str> = all.iter().map(|d| d.code.as_str()).collect();
+    assert_eq!(codes, ["E0010", "E0010"], "{all:#?}");
+}
+
+#[test]
+fn a_method_on_a_tuple_is_e0014() {
+    let d = only(
+        "fn main() {\n    let t = (1, 2)\n    let n = t.len()\n}\n",
+        "E0014",
+    );
+    assert_eq!(d.message, "no method `len` on type `(Int, Int)`");
+}
+
+#[test]
+fn extern_c_refuses_a_tuple() {
+    let all = errors("extern \"C\" {\n    fn f(p: (Int, Int)) -> Int\n}\nfn main() {}\n");
+    assert_eq!(all.len(), 1, "{all:#?}");
+    assert_eq!(all[0].code, "E0900");
+    assert!(all[0].message.contains("`(Int, Int)`"), "{:?}", all[0]);
+}
+
+// --- Display ---
+
+#[test]
+fn a_tuple_displays_as_written() {
+    let defs = nova_resolver::Definitions::default();
+    let show = |t: &Ty| crate::display_ty(t, &defs);
+    assert_eq!(show(&Ty::Tuple(vec![Ty::Int, Ty::String])), "(Int, String)");
+    assert_eq!(show(&Ty::Tuple(vec![Ty::Int])), "(Int,)");
+    assert_eq!(
+        show(&Ty::Tuple(vec![
+            Ty::Tuple(vec![Ty::Bool]),
+            Ty::Array(Box::new(Ty::Char))
+        ])),
+        "((Bool,), [Char])"
+    );
+    let named = crate::display_ty_named(
+        &Ty::Tuple(vec![Ty::Param(0), Ty::Var(3)]),
+        &defs,
+        &["K".to_string()],
+    );
+    assert_eq!(named, "(K, _)");
+}
diff --git a/crates/nova-typeck/src/infer.rs b/crates/nova-typeck/src/infer.rs
index 494e82ad9584068665e665364247d8b884c4895f..6a4ee8c1571d50192d2d8681f299f9159ceefcc9 100644
--- a/crates/nova-typeck/src/infer.rs
+++ b/crates/nova-typeck/src/infer.rs
@@ -338,4 +338,35 @@ mod tests {
             "apply must resolve the solved var inside `on`, not just pass the projection through"
         );
     }
+
+    // --- `Ty::Tuple` (5.0 spec §4.1) ---
+
+    #[test]
+    fn unify_solves_vars_inside_tuples_element_by_element() {
+        let mut cx = InferCtx::default();
+        let a = cx.fresh();
+        let b = cx.fresh();
+        let pat = Ty::Tuple(vec![a.clone(), Ty::Tuple(vec![b.clone()])]);
+        let ground = Ty::Tuple(vec![Ty::Int, Ty::Tuple(vec![Ty::String])]);
+        assert!(cx.unify(&pat, &ground));
+        assert_eq!(cx.apply(&pat), ground);
+    }
+
+    #[test]
+    fn tuples_of_different_arity_or_elements_do_not_unify() {
+        let mut cx = InferCtx::default();
+        let two = Ty::Tuple(vec![Ty::Int, Ty::Int]);
+        assert!(!cx.unify(&two, &Ty::Tuple(vec![Ty::Int, Ty::Int, Ty::Int])));
+        assert!(!cx.unify(&two, &Ty::Tuple(vec![Ty::Int, Ty::Bool])));
+        // A tuple of one is not its element.
+        assert!(!cx.unify(&Ty::Tuple(vec![Ty::Int]), &Ty::Int));
+        assert!(!cx.unify(&two, &Ty::Unit));
+    }
+
+    #[test]
+    fn occurs_check_reaches_inside_a_tuple() {
+        let mut cx = InferCtx::default();
+        let v = cx.fresh();
+        assert!(!cx.unify(&v, &Ty::Tuple(vec![Ty::Int, v.clone()])));
+    }
 }
diff --git a/tests/runtime/tuples.nova b/tests/runtime/tuples.nova
new file mode 100644
index 0000000000000000000000000000000000000000..34671b080ba67dc4ee3878b25beb58715c60f38a
--- /dev/null
+++ b/tests/runtime/tuples.nova
@@ -0,0 +1,127 @@
+// Tuple types, literals and fields (5.0 spec §4.1, §4.2): a tuple of one,
+// nested tuples read with `t.1.0` and `t.0.1.2`, writes and `+=` on an
+// element, tuples through generic functions, closures, `Option`, `Vec`,
+// arrays and a record's field, and elements of every machine class:
+// `Int`, `Float`, `Bool`, `Char`, `String`, a unit, a function and a
+// record.
+
+record Token { text: String, line: Int }
+
+record Holder { pair: (String, Int) }
+
+fn div_mod(a: Int, b: Int) -> (Int, Int) { (a / b, a % b) }
+
+fn swap<A, B>(p: (A, B)) -> (B, A) { (p.1, p.0) }
+
+fn first<A, B>(p: (A, B)) -> A { p.0 }
+
+fn pair_up<T>(a: T, b: T) -> (T, T) { (a, b) }
+
+fn apply(f: fn((Int, Int)) -> Int, p: (Int, Int)) -> Int { f(p) }
+
+fn min_max(xs: [Int]) -> Option<(Int, Int)> {
+    if xs.len() == 0 { return None }
+    let mut lo = xs[0]
+    let mut hi = xs[0]
+    let mut i = 1
+    while i < xs.len() {
+        if xs[i] < lo { lo = xs[i] }
+        if xs[i] > hi { hi = xs[i] }
+        i += 1
+    }
+    Some((lo, hi))
+}
+
+fn show_min_max(xs: [Int]) -> String {
+    match min_max(xs) {
+        Some(p) => "${p.0}..${p.1}"
+        None => "empty"
+    }
+}
+
+fn main() {
+    // Literals, a tuple of one, and a function returning a tuple.
+    let p: (String, Int) = ("x", 1)
+    let one = (5,)
+    let q = div_mod(17, 5)
+    println("${p.0} ${p.1} ${one.0} ${q.0} ${q.1}")
+
+    // Nested tuples: the lexer splits `1.0` after a `.`.
+    let n = ((1, 2), (3, (4, 5, (6, 7))))
+    println("${n.0.0} ${n.0.1} ${n.1.0} ${n.1.1.0} ${n.1.1.2.1}")
+    let deep = (((((10,),),),),)
+    println("${deep.0.0.0.0.0}")
+    let wide = (0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11)
+    println("${wide.10} ${wide.11}")
+
+    // Writes, and `+=` on an element, through a mutable binding.
+    let mut m = (1, "a", 2.5)
+    m.0 = 7
+    m.0 += 3
+    m.1 = "b"
+    m.1 += "c"
+    m.2 *= 2.0
+    println("${m.0} ${m.1} ${m.2}")
+    let mut nested = ((0, 0), 0)
+    nested.0.1 = 9
+    nested.0.0 -= 4
+    println("${nested.0.0} ${nested.0.1} ${nested.1}")
+
+    // A tuple is a reference, as a record is: a copy of the binding
+    // shares it.
+    let shared = m
+    m.0 = 100
+    println("${shared.0}")
+
+    // Generics: inference through the element types.
+    let s = swap(p)
+    println("${s.0} ${s.1}")
+    let s2 = swap(swap(("back", true)))
+    println("${s2.0} ${s2.1}")
+    println("${first((2.5, 'z'))} ${first(('z', 2.5))}")
+    let pp = pair_up(pair_up(1, 2), pair_up(3, 4))
+    println("${pp.0.1 + pp.1.0}")
+
+    // Closures that take, return and capture tuples.
+    let mul = |t: (Int, Int)| t.0 * t.1
+    println("${mul((6, 7))} ${apply(mul, (3, 4))}")
+    let base = (100, 200)
+    let add_base = |x: Int| (base.0 + x, base.1 + x)
+    let r = add_base(5)
+    println("${r.0} ${r.1}")
+
+    // `Option`, `Vec` and arrays of tuples.
+    println(show_min_max([3, 9, -2, 7]))
+    println(show_min_max([]))
+    let mut v: Vec<(Int, Token)> = Vec::new()
+    v.push((1, Token { text: "let", line: 1 }))
+    v.push((2, Token { text: "x", line: 1 }))
+    v.push((3, Token { text: "=", line: 2 }))
+    let mut i = 0
+    while i < v.len() {
+        match v.get(i) {
+            Some(t) => println("${t.0}: ${t.1.text}@${t.1.line}")
+            None => println("none")
+        }
+        i += 1
+    }
+    let arr = [("a", 1), ("b", 2), ("c", 3)]
+    let mut total = 0
+    let mut names = ""
+    let mut j = 0
+    while j < arr.len() {
+        total += arr[j].1
+        names += arr[j].0
+        j += 1
+    }
+    println("${names} ${total}")
+
+    // A record's field, and every machine class as an element.
+    let h = Holder { pair: ("h", 9) }
+    println("${h.pair.0} ${h.pair.1}")
+    let mixed = (true, 'c', 1.5, (), "s", |x: Int| x + 1, Token { text: "t", line: 3 })
+    let inc = mixed.5
+    println("${mixed.0} ${mixed.1} ${mixed.2} ${mixed.4} ${inc(41)} ${mixed.6.text}")
+    let u = mixed.3
+    println("unit ${u == ()}")
+}
diff --git a/tests/runtime/tuples.stdout b/tests/runtime/tuples.stdout
new file mode 100644
index 0000000000000000000000000000000000000000..76eed2b73f7f4b871d47d560831c0b89ad3696a8
--- /dev/null
+++ b/tests/runtime/tuples.stdout
@@ -0,0 +1,22 @@
+x 1 5 3 2
+1 2 3 4 7
+10
+10 11
+10 bc 5
+-4 9 0
+100
+1 x
+back true
+2.5 z
+5
+42 12
+105 205
+-2..9
+empty
+1: let@1
+2: x@1
+3: =@2
+abc 6
+h 9
+true c 1.5 s 42 t
+unit true
diff --git a/tests/runtime/tuples_async.nova b/tests/runtime/tuples_async.nova
new file mode 100644
index 0000000000000000000000000000000000000000..074738ae546c6df52a275890dd4583b2ffa42f08
--- /dev/null
+++ b/tests/runtime/tuples_async.nova
@@ -0,0 +1,69 @@
+// Tuples that hold pointers, held across `.await`s and through many
+// allocations (5.0 spec §4.1, §7.3): a test runs it under
+// `NOVA_GC_STRESS=1` too, which collects at every allocation, so an
+// element the collector failed to trace would be read after it was freed.
+
+record Token { text: String, line: Int }
+
+// A tail tuple on the line after a statement: the `;` ends the statement
+// before it, as the newline rule's E0103 asks (5.0 spec §3.5).
+async fn split_word(word: String, line: Int) -> (String, Token) {
+    yield_now().await;
+    ("<${word}>", Token { text: word, line: line })
+}
+
+async fn count_up(n: Int) -> (Int, [String]) {
+    let mut parts = ["start"]
+    let mut i = 0
+    while i < n {
+        yield_now().await
+        parts = [parts[0], "step ${i}"]
+        i += 1
+    }
+    return (n, parts)
+}
+
+async fn run() -> Int {
+    // A tuple built before an `.await` and read after it.
+    let held = ("kept ${40 + 2}", [1, 2, 3], Token { text: "tok", line: 7 })
+    yield_now().await
+    let mut garbage = 0
+    let mut i = 0
+    while i < 50 {
+        let s = "garbage ${i}"
+        garbage += s.len()
+        i += 1
+    }
+    println("${held.0} ${held.1[2]} ${held.2.text}@${held.2.line} ${garbage}")
+
+    // Tuples returned by async functions, kept in a `Vec` across more
+    // `.await`s.
+    let mut words: Vec<(String, Token)> = Vec::new()
+    let source = ["fn", "main", "nova"]
+    let mut line = 0
+    while line < source.len() {
+        words.push(split_word(source[line], line + 1).await)
+        line += 1
+    }
+    let mut k = 0
+    while k < words.len() {
+        match words.get(k) {
+            Some(t) => println("${t.0} ${t.1.text} ${t.1.line}")
+            None => println("none")
+        }
+        k += 1
+    }
+    let counted = count_up(3).await
+    println("${counted.0} ${counted.1[0]} ${counted.1[1]}")
+
+    // A nested tuple whose inner tuple is replaced while the outer one is
+    // held across an `.await`.
+    let mut outer = (("a", 1), "b")
+    yield_now().await
+    outer.0 = ("c ${outer.0.1 + 1}", 2)
+    yield_now().await
+    println("${outer.0.0} ${outer.0.1} ${outer.1}")
+    words.len()
+}
+
+fn main() { println("done ${block_on(run())}") }
diff --git a/tests/runtime/tuples_async.stdout b/tests/runtime/tuples_async.stdout
new file mode 100644
index 0000000000000000000000000000000000000000..140b902746c7e6826f54fbe49f3372a293e7b97a
--- /dev/null
+++ b/tests/runtime/tuples_async.stdout
@@ -0,0 +1,7 @@
+kept 42 3 tok@7 490
+<fn> fn 1
+<main> main 2
+<nova> nova 3
+3 start step 2
+c 2 2 b
+done 3
````

**Patch `t2-code`:**

<!-- patch t2-code -->

````diff
diff --git a/crates/nova-hir/src/lib.rs b/crates/nova-hir/src/lib.rs
index b7acfcc79c73c000402bfee461dd7c62d694b35f..908c5915d2b8c0ab65a28031e6017cd244ae7714 100644
--- a/crates/nova-hir/src/lib.rs
+++ b/crates/nova-hir/src/lib.rs
@@ -71,6 +71,12 @@ pub enum Ty {
     },
     /// A heap array `[T]` with elements of a single type.
     Array(Box<Ty>),
+    /// A tuple `(T1, …, Tn)`, n of one or more (`()` is [`Ty::Unit`]): an
+    /// anonymous record, laid out as a record is, with no impls and no
+    /// [`TyHead`] (spec
+    /// `docs/superpowers/specs/2026-10-26-phase-5-0-the-language-and-std-a-compiler-needs-design.md`
+    /// §4.1). Two tuple types are the same when their element types are.
+    Tuple(Vec<Ty>),
     /// A generic type parameter of the enclosing item (`T`).
     Param(u32),
     /// A projection onto a trait's associated type: `<on>::Name`, where
@@ -130,6 +136,7 @@ impl Ty {
                 args: a.iter().map(|t| t.subst(args)).collect(),
             },
             Ty::Array(elem) => Ty::Array(Box::new(elem.subst(args))),
+            Ty::Tuple(elems) => Ty::Tuple(elems.iter().map(|t| t.subst(args)).collect()),
             Ty::Future(out) => Ty::Future(Box::new(out.subst(args))),
             Ty::Assoc { on, assoc } => Ty::Assoc {
                 on: Box::new(on.subst(args)),
@@ -150,6 +157,7 @@ impl Ty {
                 args.iter().any(|a| a.mentions_param(idx))
             }
             Ty::Array(elem) => elem.mentions_param(idx),
+            Ty::Tuple(elems) => elems.iter().any(|e| e.mentions_param(idx)),
             Ty::Future(out) => out.mentions_param(idx),
             Ty::Assoc { on, .. } => on.mentions_param(idx),
             _ => false,
@@ -163,6 +171,7 @@ impl Ty {
             Ty::Fn { params, ret } => params.iter().any(Ty::has_params) || ret.has_params(),
             Ty::Sum { args, .. } | Ty::Record { args, .. } => args.iter().any(Ty::has_params),
             Ty::Array(elem) => elem.has_params(),
+            Ty::Tuple(elems) => elems.iter().any(Ty::has_params),
             Ty::Future(out) => out.has_params(),
             Ty::Assoc { on, .. } => on.has_params(),
             _ => false,
@@ -176,6 +185,7 @@ impl Ty {
             Ty::Fn { params, ret } => params.iter().any(Ty::has_vars) || ret.has_vars(),
             Ty::Sum { args, .. } | Ty::Record { args, .. } => args.iter().any(Ty::has_vars),
             Ty::Array(elem) => elem.has_vars(),
+            Ty::Tuple(elems) => elems.iter().any(Ty::has_vars),
             Ty::Future(out) => out.has_vars(),
             Ty::Assoc { on, .. } => on.has_vars(),
             _ => false,
@@ -195,6 +205,7 @@ impl Ty {
             Ty::Fn { params, ret } => params.iter().any(Ty::has_assoc) || ret.has_assoc(),
             Ty::Sum { args, .. } | Ty::Record { args, .. } => args.iter().any(Ty::has_assoc),
             Ty::Array(elem) => elem.has_assoc(),
+            Ty::Tuple(elems) => elems.iter().any(Ty::has_assoc),
             Ty::Future(out) => out.has_assoc(),
             _ => false,
         }
@@ -222,6 +233,7 @@ impl Ty {
             Ty::Fn { params, ret } => params.iter().any(Ty::has_error) || ret.has_error(),
             Ty::Sum { args, .. } | Ty::Record { args, .. } => args.iter().any(Ty::has_error),
             Ty::Array(elem) => elem.has_error(),
+            Ty::Tuple(elems) => elems.iter().any(Ty::has_error),
             Ty::Future(out) => out.has_error(),
             Ty::Assoc { on, .. } => on.has_error(),
             _ => false,
@@ -291,6 +303,9 @@ impl Ty {
                     && a1.iter().zip(a2).all(|(p, g)| p.match_pattern(g, out))
             }
             (Ty::Array(e1), Ty::Array(e2)) => e1.match_pattern(e2, out),
+            (Ty::Tuple(t1), Ty::Tuple(t2)) => {
+                t1.len() == t2.len() && t1.iter().zip(t2).all(|(p, g)| p.match_pattern(g, out))
+            }
             (Ty::Future(o1), Ty::Future(o2)) => o1.match_pattern(o2, out),
             (
                 Ty::Fn {
@@ -315,6 +330,14 @@ impl Ty {
     }
 }
 
+/// The note an E0013 on a tuple carries: a tuple has no impls, so it is never
+/// `Eq`, `Hash`, `Display` or `Debug`, nor any other trait (spec
+/// `docs/superpowers/specs/2026-10-26-phase-5-0-the-language-and-std-a-compiler-needs-design.md`
+/// §4.1).
+pub const TUPLE_HAS_NO_IMPLS: &str = "a tuple has no impls, so it has no `==`, `Hash`, \
+     `Display`, `Debug` or any other trait: compare or print its elements, or use a record \
+     that implements the trait";
+
 /// The nominal head of a type, used as the key for impl lookups
 /// (spec `12-TYPESYSTEM.md` §5.2, `ImplTable` indexed by trait + type head).
 #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
@@ -649,6 +672,12 @@ fn normalize_within(
         Ty::Array(elem) => Ok(Ty::Array(Box::new(normalize_within(
             elem, impls, depth, steps,
         )?))),
+        Ty::Tuple(elems) => Ok(Ty::Tuple(
+            elems
+                .iter()
+                .map(|e| normalize_within(e, impls, depth, steps))
+                .collect::<Result<_, _>>()?,
+        )),
         Ty::Future(out) => Ok(Ty::Future(Box::new(normalize_within(
             out, impls, depth, steps,
         )?))),
@@ -709,6 +738,7 @@ fn shift_params(t: &Ty, by: u32) -> Ty {
             args: args.iter().map(|a| shift_params(a, by)).collect(),
         },
         Ty::Array(elem) => Ty::Array(Box::new(shift_params(elem, by))),
+        Ty::Tuple(elems) => Ty::Tuple(elems.iter().map(|e| shift_params(e, by)).collect()),
         Ty::Future(out) => Ty::Future(Box::new(shift_params(out, by))),
         other => other.clone(),
     }
@@ -733,6 +763,7 @@ fn occurs(k: u32, t: &Ty, subst: &[Option<Ty>]) -> bool {
         }
         Ty::Sum { args, .. } | Ty::Record { args, .. } => args.iter().any(|a| occurs(k, a, subst)),
         Ty::Array(elem) => occurs(k, &elem, subst),
+        Ty::Tuple(elems) => elems.iter().any(|e| occurs(k, e, subst)),
         Ty::Future(out) => occurs(k, &out, subst),
         _ => false,
     }
@@ -782,6 +813,9 @@ fn unify_patterns(a: &Ty, b: &Ty, subst: &mut Vec<Option<Ty>>) -> bool {
                 && a1.iter().zip(a2).all(|(x, y)| unify_patterns(x, y, subst))
         }
         (Ty::Array(e1), Ty::Array(e2)) => unify_patterns(e1, e2, subst),
+        (Ty::Tuple(t1), Ty::Tuple(t2)) => {
+            t1.len() == t2.len() && t1.iter().zip(t2).all(|(x, y)| unify_patterns(x, y, subst))
+        }
         (Ty::Future(o1), Ty::Future(o2)) => unify_patterns(o1, o2, subst),
         (
             Ty::Fn {
@@ -1088,12 +1122,17 @@ pub enum ExprKind {
         record: DefId,
         fields: Vec<Expr>,
     },
-    /// Read field `index` of a record value.
+    /// Construct a tuple value: its elements in order, laid out as a
+    /// record's fields are. The expression's `ty` is the `Tuple`.
+    MakeTuple {
+        elems: Vec<Expr>,
+    },
+    /// Read field `index` of a record or a tuple value.
     FieldGet {
         target: Box<Expr>,
         index: u32,
     },
-    /// Store `record.field = value`. `index` is the field's position, so the
+    /// Store `record.field = value`, or `tuple.N = value`. `index` is the field's position, so the
     /// store offset is `8 * index` — the same layout `FieldGet` reads.
     /// Unit-typed.
     FieldSet {
diff --git a/crates/nova-mir/src/lib.rs b/crates/nova-mir/src/lib.rs
index a7ede53b2c88bcb52111497ca815ad81180ac155..55e1ac310c93d72764a6685c24ce272e8b30ab73 100644
--- a/crates/nova-mir/src/lib.rs
+++ b/crates/nova-mir/src/lib.rs
@@ -1299,6 +1299,7 @@ pub fn mir_ty(ty: &hir::Ty) -> MirTy {
         | hir::Ty::Sum { .. }
         | hir::Ty::Record { .. }
         | hir::Ty::Array(_)
+        | hir::Ty::Tuple(_)
         | hir::Ty::Future(_) => MirTy::Ptr,
         hir::Ty::Unit | hir::Ty::Never => MirTy::Unit,
         // Post-typeck these should not occur; map defensively. `Assoc` is
@@ -1374,8 +1375,18 @@ fn mangle_ty(ty: &hir::Ty) -> String {
             }
         }
         hir::Ty::Array(elem) => format!("A{}E", mangle_ty(elem)),
+        // A tuple is `T`, its elements' manglings in order, and `E`: each
+        // element's mangling is prefix-free and none begins with `E`, so
+        // distinct tuple types, `(Int, (Int, Int))` and `((Int, Int), Int)`
+        // among them, get distinct strings (spec
+        // `docs/superpowers/specs/2026-10-26-phase-5-0-the-language-and-std-a-compiler-needs-design.md`
+        // §4.1, §10 risk 3).
+        hir::Ty::Tuple(elems) => {
+            let es: Vec<String> = elems.iter().map(mangle_ty).collect();
+            format!("T{}E", es.join(""))
+        }
         // `U` is not otherwise used as a leading letter here (nullary:
-        // i/f/b/c/s/y/u/n; compound: F/S/R/A above), and the mangling must be
+        // i/f/b/c/s/y/u/n; compound: F/S/R/A/T above), and the mangling must be
         // output-dependent like `Array`'s `A{elem}E` rather than a shared
         // placeholder — see `mangle_ty_distinguishes_futures_by_output_type`.
         hir::Ty::Future(out) => format!("U{}E", mangle_ty(out)),
diff --git a/crates/nova-mir/src/lower.rs b/crates/nova-mir/src/lower.rs
index 1bc23857bbfa50a0d19591e0fd8e1e784627bdfe..636fc5aa44bbb6635b04bd7409d091c98a95458b 100644
--- a/crates/nova-mir/src/lower.rs
+++ b/crates/nova-mir/src/lower.rs
@@ -329,7 +329,9 @@ impl<'a> Lowerer<'a> {
                 });
                 t
             }
-            K::MakeRecord { fields, .. } => {
+            // A tuple is an anonymous record (spec §4.1): the same
+            // `MakeRecord`, so no backend sees a tuple.
+            K::MakeRecord { fields, .. } | K::MakeTuple { elems: fields } => {
                 let lowered: Vec<(Temp, MirTy)> = fields
                     .iter()
                     .map(|f| {
diff --git a/crates/nova-mir/src/mono.rs b/crates/nova-mir/src/mono.rs
index be3d9f8ce6c43cba4917546e9bd95f8e58e947e3..151ade6de303c89640ae3cca2c479cb6105e9dcb 100644
--- a/crates/nova-mir/src/mono.rs
+++ b/crates/nova-mir/src/mono.rs
@@ -273,18 +273,20 @@ pub fn lower_module_with(
                         .trait_def(trait_id)
                         .map(|t| t.name.clone())
                         .unwrap_or_else(|| format!("trait#{}", trait_id.0));
-                    diagnostics.push(
-                        Diagnostic::error(
-                            "E0013",
-                            format!(
-                                "trait bound `{}: {}` is not satisfied when instantiating `{}`",
-                                type_name(arg, module),
-                                trait_name,
-                                func.name
-                            ),
-                        )
-                        .with_primary_label(func.span, "required by this generic parameter"),
-                    );
+                    let mut diagnostic = Diagnostic::error(
+                        "E0013",
+                        format!(
+                            "trait bound `{}: {}` is not satisfied when instantiating `{}`",
+                            type_name(arg, module),
+                            trait_name,
+                            func.name
+                        ),
+                    )
+                    .with_primary_label(func.span, "required by this generic parameter");
+                    if matches!(arg, Ty::Tuple(_)) {
+                        diagnostic = diagnostic.with_note(hir::TUPLE_HAS_NO_IMPLS);
+                    }
+                    diagnostics.push(diagnostic);
                 }
             }
         }
@@ -609,6 +611,14 @@ fn type_name(ty: &Ty, module: &hir::Module) -> String {
             .map(|r| r.name.clone())
             .unwrap_or_else(|| "?".to_string()),
         Ty::Array(elem) => format!("[{}]", type_name(elem, module)),
+        Ty::Tuple(elems) => {
+            let names: Vec<String> = elems.iter().map(|e| type_name(e, module)).collect();
+            if names.len() == 1 {
+                format!("({},)", names[0])
+            } else {
+                format!("({})", names.join(", "))
+            }
+        }
         Ty::Future(out) => format!("Future<{}>", type_name(out, module)),
         Ty::Assoc { on, assoc } => {
             let name = module
@@ -784,6 +794,9 @@ impl Specializer<'_> {
                 record: *record,
                 fields: self.exprs(rec_fields),
             },
+            K::MakeTuple { elems } => K::MakeTuple {
+                elems: self.exprs(elems),
+            },
             K::FieldGet { target, index } => K::FieldGet {
                 target: self.boxed(target),
                 index: *index,
diff --git a/crates/nova-resolver/src/lib.rs b/crates/nova-resolver/src/lib.rs
index 324d15bbe98e90a378ea14f2582c4cb84f6e6b4a..afdd3223f23ee6229d86746eefe65bb70d99e788 100644
--- a/crates/nova-resolver/src/lib.rs
+++ b/crates/nova-resolver/src/lib.rs
@@ -3364,6 +3364,12 @@ fn type_full_name(ty: &nova_ast::Type) -> String {
             }
         }
         nova_ast::Type::Array(elem) => format!("Arr_{}", type_full_name(&elem.value)),
+        // A tuple, as an impl's type argument (`impl Box<(Int, Int)>`): no
+        // impl names a tuple itself (5.0 spec §4.1).
+        nova_ast::Type::Tuple(items) if !items.is_empty() => {
+            let parts: Vec<String> = items.iter().map(|a| type_full_name(&a.value)).collect();
+            format!("Tup{}_{}", items.len(), parts.join("_"))
+        }
         _ => "anon".to_string(),
     }
 }
diff --git a/crates/nova-typeck/src/check.rs b/crates/nova-typeck/src/check.rs
index 0e65e1da71f6553d82f876b2ca198334389d3b3f..bfae12a47d8a2410fca261d9f492ff05c021961e 100644
--- a/crates/nova-typeck/src/check.rs
+++ b/crates/nova-typeck/src/check.rs
@@ -2682,10 +2682,14 @@ impl<'a> Checker<'a> {
                     .collect(),
                 ret: Box::new(self.convert_ty(ret, generics, bounds)),
             },
-            ast::Type::Tuple(_) => {
-                self.unsupported(ty.span, "tuple types");
-                Ty::Error
-            }
+            // `(T1, …, Tn)`, n of one or more: `(T)` is `T` already, and
+            // `(T,)` a tuple of one (spec §4.1).
+            ast::Type::Tuple(items) => Ty::Tuple(
+                items
+                    .iter()
+                    .map(|t| self.convert_ty(t, generics, bounds))
+                    .collect(),
+            ),
             ast::Type::Ref { .. } | ast::Type::Ptr { .. } => {
                 self.unsupported(ty.span, "reference and pointer types");
                 Ty::Error
@@ -3551,7 +3555,17 @@ impl<'a> Checker<'a> {
             ast::Expr::ArrayRepeat { init, len } => self.check_array_repeat(fcx, init, len, span),
             ast::Expr::Index { target, index } => self.check_index(fcx, target, index, span),
             // --- deferred constructs ---
-            ast::Expr::Tuple(_) => self.unsupported_expr(span, "tuple expressions"),
+            ast::Expr::Tuple(items) => {
+                // A tuple literal, an anonymous record (spec §4.1): its
+                // elements left to right, its type theirs.
+                let elems: Vec<hir::Expr> = items.iter().map(|e| self.check_expr(fcx, e)).collect();
+                let ty = Ty::Tuple(elems.iter().map(|e| e.ty.clone()).collect());
+                hir::Expr {
+                    kind: hir::ExprKind::MakeTuple { elems },
+                    ty,
+                    span,
+                }
+            }
             ast::Expr::Closure { params, ret, body } => {
                 self.check_closure(fcx, params, ret, body, span)
             }
@@ -3650,6 +3664,7 @@ impl<'a> Checker<'a> {
                                         // Some path; use the interp span here.
                                         span,
                                     );
+                                    self.note_tuple_has_no_impls(&other);
                                 }
                             }
                         }
@@ -5899,6 +5914,9 @@ impl<'a> Checker<'a> {
         if matches!(recv_ty, Ty::Error) {
             return error_expr(span);
         }
+        if self.tuple_index_on_unknown(fcx, &recv_ty, field) {
+            return error_expr(span);
+        }
         self.error(
             "E0014",
             self.no_field_message(fcx, &recv_ty, &field.value),
@@ -5918,6 +5936,38 @@ impl<'a> Checker<'a> {
     /// is exactly what had happened before this was pulled out (the write
     /// path said "no field `x` on type `P`" and had no separate wording for a
     /// non-record receiver at all).
+    /// `t.N` on a value whose type is not yet known: E0011, as a method
+    /// call on one is (spec §4.2). Reports it and returns `true`; `false`
+    /// for any other field or receiver.
+    fn tuple_index_on_unknown(
+        &mut self,
+        fcx: &mut FnCtx,
+        recv_ty: &Ty,
+        field: &Spanned<String>,
+    ) -> bool {
+        let is_index = field.value.bytes().all(|b| b.is_ascii_digit());
+        if !is_index || !matches!(fcx.icx.apply(recv_ty), Ty::Var(_)) {
+            return false;
+        }
+        self.error(
+            "E0011",
+            format!(
+                "the type of this value must be known before `.{}` reads an element of it",
+                field.value
+            ),
+            field.span,
+        );
+        self.diagnostics
+            .last_mut()
+            .expect("just pushed")
+            .notes
+            .push("annotate the value's type, for example `let p: (Int, Int) = …`".to_string());
+        // Bound to the error type, so the variable is not reported again as
+        // one inference could not solve.
+        fcx.icx.unify(recv_ty, &Ty::Error);
+        true
+    }
+
     fn no_field_message(&self, fcx: &FnCtx, recv_ty: &Ty, field_name: &str) -> String {
         let record_name = match recv_ty {
             Ty::Record { def_id, .. } => self
@@ -5929,6 +5979,10 @@ impl<'a> Checker<'a> {
         };
         match record_name {
             Some(record_name) => format!("no field `{field_name}` on record `{record_name}`"),
+            // `t.2` on `(Int, Int)`, or `t.x` on a tuple (spec §4.2).
+            None if matches!(fcx.icx.apply(recv_ty), Ty::Tuple(_)) => {
+                format!("no field `{field_name}` on `{}`", self.show(recv_ty, fcx))
+            }
             None => format!(
                 "cannot access field `{field_name}` on `{}`",
                 self.show(recv_ty, fcx)
@@ -5956,7 +6010,14 @@ impl<'a> Checker<'a> {
         field: &str,
         span: Span,
     ) -> Option<(u32, Ty)> {
-        let Ty::Record { def_id, args } = fcx.icx.apply(recv_ty) else {
+        let resolved = fcx.icx.apply(recv_ty);
+        if let Ty::Tuple(elems) = &resolved {
+            // `t.N` (spec §4.2): the parser lets only plain decimal digits
+            // through, so the name is the index.
+            let index: usize = field.parse().ok()?;
+            return elems.get(index).map(|ty| (index as u32, ty.clone()));
+        }
+        let Ty::Record { def_id, args } = resolved else {
             return None;
         };
         let record = self.records.iter().find(|r| r.def_id == def_id)?;
@@ -6807,6 +6868,15 @@ impl<'a> Checker<'a> {
             ),
             span,
         );
+        self.note_tuple_has_no_impls(ty);
+    }
+
+    /// On the E0013 just reported, when `ty` is a tuple, say why (spec
+    /// §4.1).
+    fn note_tuple_has_no_impls(&mut self, ty: &Ty) {
+        if let (Ty::Tuple(_), Some(d)) = (ty, self.diagnostics.last_mut()) {
+            d.notes.push(hir::TUPLE_HAS_NO_IMPLS.to_string());
+        }
     }
 
     fn check_unary(
@@ -7195,6 +7265,10 @@ impl<'a> Checker<'a> {
         let Some((index, field_ty)) =
             self.record_field_index_and_ty(fcx, &recv_ty, &field.value, span)
         else {
+            if self.tuple_index_on_unknown(fcx, &recv_ty, field) {
+                self.check_expr(fcx, rhs);
+                return error_expr(span);
+            }
             // Same wording as the read path (`no_field_message`), including
             // its separate not-a-record case.
             self.error(
@@ -8040,6 +8114,11 @@ fn collect_self_projections(ty: &Ty, self_ty: &Ty, out: &mut Vec<DefId>) {
             }
         }
         Ty::Array(elem) => collect_self_projections(elem, self_ty, out),
+        Ty::Tuple(elems) => {
+            for e in elems {
+                collect_self_projections(e, self_ty, out);
+            }
+        }
         Ty::Future(fut_out) => collect_self_projections(fut_out, self_ty, out),
         Ty::Int
         | Ty::Float
@@ -8157,6 +8236,7 @@ fn child_exprs(expr: &hir::Expr) -> Vec<&hir::Expr> {
         K::Call { args, .. } => out.extend(args.iter()),
         K::MakeVariant { args, .. }
         | K::MakeRecord { fields: args, .. }
+        | K::MakeTuple { elems: args }
         | K::MakeArray { elems: args }
         | K::StrConcat(args) => out.extend(args.iter()),
         K::FieldGet { target, .. } | K::ArrayLen { target } => out.push(target),
@@ -8241,6 +8321,7 @@ fn child_exprs_mut(kind: &mut hir::ExprKind) -> Vec<&mut hir::Expr> {
         K::Call { args, .. } => out.extend(args.iter_mut()),
         K::MakeVariant { args, .. }
         | K::MakeRecord { fields: args, .. }
+        | K::MakeTuple { elems: args }
         | K::MakeArray { elems: args }
         | K::StrConcat(args) => out.extend(args.iter_mut()),
         K::FieldGet { target, .. } | K::ArrayLen { target } => out.push(target),
@@ -8810,6 +8891,7 @@ fn finalize_expr(expr: &mut hir::Expr, icx: &InferCtx, residual: &mut Vec<Span>)
         }
         hir::ExprKind::MakeVariant { args, .. }
         | hir::ExprKind::MakeRecord { fields: args, .. }
+        | hir::ExprKind::MakeTuple { elems: args }
         | hir::ExprKind::MakeArray { elems: args }
         | hir::ExprKind::StrConcat(args) => {
             for a in args {
diff --git a/crates/nova-typeck/src/infer.rs b/crates/nova-typeck/src/infer.rs
index 6a4ee8c1571d50192d2d8681f299f9159ceefcc9..19ac8ecc44055ebf3d35d9f276a54293ed72d10d 100644
--- a/crates/nova-typeck/src/infer.rs
+++ b/crates/nova-typeck/src/infer.rs
@@ -48,6 +48,7 @@ impl InferCtx {
                 args: args.iter().map(|a| self.apply(a)).collect(),
             },
             Ty::Array(elem) => Ty::Array(Box::new(self.apply(&elem))),
+            Ty::Tuple(elems) => Ty::Tuple(elems.iter().map(|e| self.apply(e)).collect()),
             Ty::Future(out) => Ty::Future(Box::new(self.apply(&out))),
             Ty::Assoc { on, assoc } => Ty::Assoc {
                 on: Box::new(self.apply(&on)),
@@ -68,6 +69,7 @@ impl InferCtx {
                 args.iter().any(|a| self.occurs(v, a))
             }
             Ty::Array(elem) => self.occurs(v, &elem),
+            Ty::Tuple(elems) => elems.iter().any(|e| self.occurs(v, e)),
             Ty::Future(out) => self.occurs(v, &out),
             Ty::Assoc { on, .. } => self.occurs(v, &on),
             _ => false,
@@ -159,6 +161,14 @@ impl InferCtx {
                         .all(|(x, y)| self.unify(x, y))
             }
             (Ty::Array(e1), Ty::Array(e2)) => self.unify(&e1.clone(), &e2.clone()),
+            (Ty::Tuple(t1), Ty::Tuple(t2)) => {
+                t1.len() == t2.len()
+                    && t1
+                        .clone()
+                        .iter()
+                        .zip(t2.clone().iter())
+                        .all(|(x, y)| self.unify(x, y))
+            }
             (Ty::Future(o1), Ty::Future(o2)) => self.unify(&o1.clone(), &o2.clone()),
             (Ty::Assoc { on: o1, assoc: a1 }, Ty::Assoc { on: o2, assoc: a2 }) => {
                 a1 == a2 && self.unify(&o1.clone(), &o2.clone())
diff --git a/crates/nova-typeck/src/lib.rs b/crates/nova-typeck/src/lib.rs
index 1acfbf01019fd58d967c35beff7a3c1610d9bf1d..989cd5db09dac3a02b7040fa737ba0acc01222d9 100644
--- a/crates/nova-typeck/src/lib.rs
+++ b/crates/nova-typeck/src/lib.rs
@@ -118,6 +118,7 @@ pub fn display_ty(ty: &Ty, defs: &Definitions) -> String {
             }
         }
         Ty::Array(elem) => format!("[{}]", display_ty(elem, defs)),
+        Ty::Tuple(elems) => tuple_text(elems.iter().map(|e| display_ty(e, defs)).collect()),
         Ty::Future(out) => format!("Future<{}>", display_ty(out, defs)),
         Ty::Param(i) => format!("T{i}"),
         Ty::Assoc { on, assoc } => {
@@ -129,6 +130,16 @@ pub fn display_ty(ty: &Ty, defs: &Definitions) -> String {
     }
 }
 
+/// A tuple type's text from its elements' texts: `(Int, String)`, and
+/// `(Int,)` for a tuple of one, as the parser reads it.
+pub fn tuple_text(elems: Vec<String>) -> String {
+    if elems.len() == 1 {
+        format!("({},)", elems[0])
+    } else {
+        format!("({})", elems.join(", "))
+    }
+}
+
 /// [`display_ty`] for hover (spec 3.4a §3.4): type parameter `i` prints as
 /// `names[i]`, and an unsolved variable or an error as `_`.
 pub fn display_ty_named(ty: &Ty, defs: &Definitions, names: &[String]) -> String {
@@ -145,6 +156,7 @@ pub fn display_ty_named(ty: &Ty, defs: &Definitions, names: &[String]) -> String
             }
         }
         Ty::Array(elem) => format!("[{}]", show(elem)),
+        Ty::Tuple(elems) => tuple_text(elems.iter().map(show).collect()),
         Ty::Future(out) => format!("Future<{}>", show(out)),
         Ty::Param(i) => names
             .get(*i as usize)
````

### Task 3: Patterns in HIR, the checker and usefulness

Spec §4.3, §4.4; the user's decision 4 and ruling of 2026-10-28; decisions
2, 7 to 10, 13, 15, 17 to 19, 22 to 25, 28, 30, 33, 35, 38 and 42 to 44.

**Files:**
- Create: `crates/nova-typeck/src/check/patterns.rs` (tuple, or- and range
  patterns, E0100, E0101, E0104, `require_irrefutable`, `for_loop_binding`,
  `check_or_alternatives`; Task 4 adds `check_for_array`)
- Modify:
  - `crates/nova-hir/src/lib.rs` (the recursive `Pattern`, `bindings`,
    `is_flat`; `Arm.guard`)
  - `crates/nova-typeck/src/check.rs` (`check_pattern` recursing, `Char`
    literals, the bare variant name's note, guards, the destructuring `let`,
    `for_loop_binding` in the range and iterator desugars, `ArmPat`,
    `to_useful_pat`, `render_witness`, the walks over guards)
  - `crates/nova-typeck/src/check/fixes.rs` (`Binding::Pattern`,
    `immutable_note`)
  - `crates/nova-typeck/src/usefulness.rs` (`Ctor::{Char, Range, Tuple}`,
    `Pat::Or`, `MatchCx::naming_values`)
  - `crates/nova-mir/src/lower.rs` (the flat path reads `fields` and takes a
    `Char` key; the refusal of a nested or guarded match, until Task 5),
    `crates/nova-mir/src/mono.rs` (guards)
- Test:
  - `crates/nova-typeck/src/check/pattern_tests.rs` (new, 45 tests; Task 4
    adds six), `usefulness.rs`'s eight, `nova-hir`'s two
  - `crates/nova-mir/tests/lower_tests.rs` (the refusal's test, which Task 5
    removes, and the `Char` switch's)
  - `crates/nova-fmt/tests/patterns.rs` (new, ten tests)
  - `crates/nova-lsp/src/hover.rs`, `completion.rs` (one test each)

- [ ] **Step 1: The tests.** `patch_block t3-tests`. The patch holds:
  - `pattern_tests.rs`: the recursive HIR pattern of a nested match, an
    or-pattern's one local in every alternative, a `Char` literal, guards
    that see the arm's bindings and must be `Bool`; E0100 for a missing or
    an extra binding, naming it, and for a binding at two types; nested
    or-patterns binding the outer first alternative's names; E0101, an empty
    range not making the arms after it unreachable, a one-value range not
    empty; E0010 for a range over another type, a tuple pattern of the wrong
    arity or type, a `Char` pattern against an `Int`; the bare variant name
    (decision 18): `Yes(No)` matching `No`, another type's variant E0001
    with the note, `Yes(Num)` for a variant with fields E0016, `Yes(x)`
    still binding; E0104 for `Yes(Rde)`, `let (A, b)` and a top-level
    `Whatever`, and not for a lower-case name, `_Upper` or a real variant,
    once in an or-pattern, a constant's own note, and "did you mean" near a
    variant (decision 42); an erroneous pattern not making the arms after it
    unreachable, nor a `let` also E0022 (decision 43); the forms still out
    (E0900); E0020 with a nested witness, a tuple of sums missing a
    combination, a tuple of one's witness keeping its comma; an or-pattern
    covering each alternative, a guarded arm covering nothing, literals and
    ranges needing a catch-all; E0021 for a nested arm an earlier one
    covers, the same literal or range again but not an overlap, an
    unreachable alternative on it alone, not again inside one reported, and
    a whole unreachable arm reported as the arm; a generic sum at depth
    exhaustive; `let` destructuring a tuple and a one-variant sum, immutable
    without `mut` and E0060's note saying how, as it does for a match arm's
    binding and a `for` loop's variable, a refutable pattern E0022 with the
    values it misses, `(_, 0)` among them, a destructured `let` as a one-arm
    `match`; a tuple pattern over a range E0010; a tuple-pattern arm after a
    bare `return` or an ordinary arm. The module is declared in `check.rs`.
  - `usefulness.rs`: a nested witness names the inner variant; a nested arm
    an earlier one covers is not useful; an or-pattern row covers each
    alternative; a tuple of sums needs every combination; a tuple is one
    constructor even when its type is unknown; literals and ranges inside a
    variant need a catch-all; ranges are not split; a generic sum at depth
    takes its arguments. **`nova-hir`:** a pattern's bindings, each local
    once, in order; only today's shapes are flat.
  - fmt's `patterns.rs`: tuple patterns, a tuple pattern of one, nested
    patterns, guards, or-patterns with a space around each `|`, a long
    or-pattern breaking before each `|`, `Char` literals and ranges, `Int`
    ranges, `let` and `for` destructuring, and a tuple-pattern arm after a
    bare `return` arm printing without a `;`.
  - The language server: a binding in a nested pattern hovers with its type,
    and completes in its arm, with a member after it.
  - `lower_tests.rs`: `a_nested_or_guarded_match_is_refused_until_chains`,
    `Some(Some(n))` and `Some(_) if true` each E0900 from lowering, the
    checker accepting both (decision 35); and
    `a_flat_char_match_switches_on_its_code_points`, a flat `Char` match's
    one switch on keys `97` and `98` (decision 26).
- [ ] **Step 2: Run, and see the tests fail.** First the build, which ends
  with `Finished`.
  - `T pattern flat usefulness`, the library tests of Task 3:
    `pattern_tests.rs`, `usefulness.rs`'s, `nova-hir`'s and the language
    server's. Expected: it does not compile,
    ``error: could not compile `nova-hir` (lib test) due to 9 previous errors``:
    ``variant `Pattern::Variant` has no field named `fields` `` and no
    `Tuple`, `Or`, `Range` or `LitChar` in `Pattern` (decision 38).
  - `cargo test --locked --workspace --test patterns`. Expected:
    `10 passed; 0 failed`: fmt prints the patterns the parser already read,
    and these tests pin that (decision 30).
  - `cargo test --locked --workspace --test lower_tests`. Expected:
    `66 passed; 2 failed`, each where its program must check cleanly:
    `a_nested_or_guarded_match_is_refused_until_chains`, the base's checker
    refusing `Some(Some(n))`, "E0900: nested patterns inside variants are
    not supported yet", and `a_flat_char_match_switches_on_its_code_points`,
    "E0900: float and char patterns are not supported yet".
- [ ] **Step 3: The code.** `patch_block t3-code`:
  - **`nova-hir`:** `Pattern` as decision 13 gives it, `PartialEq`;
    `bindings()`, each local once, an or-pattern's from its first
    alternative; `is_flat()`; `Arm.guard`.
  - **`usefulness.rs`:** `Ctor::Char`, `Ctor::Range(lo, hi)` and
    `Ctor::Tuple(arity)`; `Pat::Or`. A row whose head is an or-pattern
    stands for one row per alternative (`expand_or_heads`), and an
    or-pattern query is useful when an alternative is; specialization
    expands an or-row too; a tuple type's signature is its one constructor,
    and a column whose rows hold a tuple is read as a tuple of that arity
    when its type is not yet known (`tuple_ty`). A column no row names gives
    the witness `_`; where E0022 asks (`naming_values`), a missed literal is
    named by a value and the values under each present constructor are named
    too (decision 44). Its module doc says why ranges are not split
    (decision 8).
  - **`patterns.rs`:** `PatState`, the or-patterns being checked and
    `let mut`; `bind_pattern_name`, a later alternative's name the first
    one's local (E0100 otherwise), E0104 for a new binding whose name starts
    with an upper-case letter (`warn_upper_case_binding`: a constant's note,
    or "did you mean" near a variant), and each bound as
    `Binding::LetPattern` in a `let`, `Binding::Pattern` elsewhere;
    `tuple_pattern`, `or_pattern`, `range_pattern` (E0101; `lo..hi` stored
    as `lo..=hi - 1`); `require_irrefutable` (E0022 by usefulness, naming
    values), skipped after a pattern error; `LoopBind` and
    `for_loop_binding`; `let_pattern`, a one-arm `match`;
    `check_or_alternatives` with `or_alternative_spans`, `or_tree`,
    `ancestors` and `restrict_or`.
  - **`check.rs`:** `check_pattern` takes `Char` literals and the tuple, or-
    and range forms, recurses into a variant's fields, and gives the bare
    variant name's E0001 its note; the record, array, `@` and `Float` forms
    each their E0900; a guard checked as a `Bool` with the arm's bindings in
    scope; `ArmPat`, with each or-pattern's alternatives' spans;
    `check_or_alternatives` after an arm that is reachable and unguarded; no
    usefulness for a match whose patterns added an error (`error_count`); a
    destructuring `let` through `let_pattern` and `require_irrefutable`;
    `for_loop_binding` in place of `for_loop_var`; `?`'s desugar and the
    iterator desugar build `fields`; `to_useful_pat` recursing;
    `render_witness` for or-patterns, `Char` literals, ranges and tuples;
    `remap_pattern`, `child_exprs`, `child_exprs_mut` and `finalize_expr`
    over guards.
  - **`fixes.rs`:** `Binding::LetPattern` and `Binding::Pattern`;
    `immutable_note`, E0060's note by where the name is bound: a `let`
    pattern's by `let mut (…)` or a rebinding, an arm's or a `for` pattern's
    by a rebinding, a plain `for` variable's by a rebinding in the loop's
    body.
  - **MIR:** `lower.rs`'s flat path binds a variant's `fields` that are
    `Bind` and switches on a `Char` literal's code point (decision 26), and
    `lower_match` refuses a match with a guard or a pattern `is_flat`
    refuses, E0900, until Task 5 (decision 35); mono maps each arm's guard.
- [ ] **Step 4: Run.** First the build, which ends with `Finished`.
  - `T pattern flat usefulness`. Expected: every `test result` `ok`, 68
    passing in all: `nova-typeck`'s 60 (`pattern_tests.rs`'s 45 and
    `usefulness.rs`'s 14, its eight new among them, and one more whose name
    holds `pattern`), `nova-hir`'s 5 (its two new and three older),
    `nova-lsp`'s 2 and `nova-fmt`'s library 1, an older test.
  - `cargo test --locked --workspace --test patterns`. Expected:
    `10 passed; 0 failed`.
  - `cargo test --locked --workspace --test lower_tests`. Expected:
    `68 passed; 0 failed`, the refusal's test and the `Char` switch's among
    them.
  - `S`, as in Task 1 (about 4 min 40 s). Expected: `2671 1 10`: Task 2's
    figures and Task 3's 69 tests, the one failure the IPv6 test. No program
    in the suite matches by a nested pattern or a guard before Task 5, so
    the refusal stops none; Task 8, Step 10 runs every program of the last
    tree with this commit's `nova` (decision 47).
- [ ] **Step 5: The checks,** as the conventions list them.
- [ ] **Step 6: Commit**
  `typeck: nested, tuple, or-, Char and range patterns, guards, and destructuring`.

**Patch `t3-tests`:**

<!-- patch t3-tests -->

````diff
diff --git a/crates/nova-fmt/tests/patterns.rs b/crates/nova-fmt/tests/patterns.rs
new file mode 100644
index 0000000000000000000000000000000000000000..c2ad347fbea7765a48894d86f6e8ad8b17a4bf0f
--- /dev/null
+++ b/crates/nova-fmt/tests/patterns.rs
@@ -0,0 +1,105 @@
+//! 5.0b's patterns in fmt (5.0 spec
+//! `docs/superpowers/specs/2026-10-26-phase-5-0-the-language-and-std-a-compiler-needs-design.md`
+//! §4.3 to §4.5): each form the checker now takes prints as it reads, and
+//! a messy spelling of each reformats to it.
+
+mod common;
+use common::{assert_formats, assert_stable};
+
+#[test]
+fn tuple_patterns_print_as_written() {
+    assert_stable(
+        "fn f(p: (Int, (Bool, Char))) -> Int {\n    match p {\n        (0, (true, _)) => 1\n        (n, _) => n\n    }\n}\n",
+    );
+    assert_formats(
+        "fn f(p: (Int, Int)) -> Int { match p { ( a , b , ) => a } }\n",
+        "fn f(p: (Int, Int)) -> Int {\n    match p {\n        (a, b) => a\n    }\n}\n",
+    );
+}
+
+#[test]
+fn a_tuple_pattern_of_one_keeps_its_comma() {
+    assert_formats(
+        "fn main() {\n    let (a ,) = (1,)\n}\n",
+        "fn main() { let (a,) = (1,) }\n",
+    );
+}
+
+#[test]
+fn nested_patterns_print_as_written() {
+    assert_stable(
+        "fn f(o: Option<Result<Int, String>>) -> Int {\n    match o {\n        Some(Ok(n)) => n\n        Some(Err(_)) | None => 0\n    }\n}\n",
+    );
+}
+
+#[test]
+fn a_guard_prints_after_its_pattern() {
+    assert_stable(
+        "fn f(o: Option<Int>) -> Int {\n    match o {\n        Some(n) if n > 0 => n\n        _ => 0\n    }\n}\n",
+    );
+    assert_formats(
+        "fn f(n: Int) -> Int { match n { x   if   x > 0   => x, _ => 0 } }\n",
+        "fn f(n: Int) -> Int {\n    match n {\n        x if x > 0 => x\n        _ => 0\n    }\n}\n",
+    );
+}
+
+#[test]
+fn or_patterns_print_with_spaces_around_each_bar() {
+    assert_formats(
+        "fn f(n: Int) -> Int { match n { 0|1  |2 => 1, _ => 0 } }\n",
+        "fn f(n: Int) -> Int {\n    match n {\n        0 | 1 | 2 => 1\n        _ => 0\n    }\n}\n",
+    );
+    assert_stable(
+        "fn f(o: Option<Int>) -> Int {\n    match o {\n        Some(1 | 2) => 1\n        _ => 0\n    }\n}\n",
+    );
+}
+
+#[test]
+fn a_long_or_pattern_breaks_before_each_bar() {
+    assert_formats(
+        "fn f(t: Token) -> Int { match t { Token::Identifier(name) | Token::Keyword(name) | Token::Operator(name) | Token::Punctuation(name) => 1, _ => 0 } }\n",
+        "fn f(t: Token) -> Int {\n    match t {\n        Token::Identifier(name)\n            | Token::Keyword(name)\n            | Token::Operator(name)\n            | Token::Punctuation(name) => 1\n        _ => 0\n    }\n}\n",
+    );
+}
+
+#[test]
+fn char_literals_and_ranges_print_as_written() {
+    assert_stable(
+        "fn f(c: Char) -> Int {\n    match c {\n        'a'..='z' | 'A'..='Z' => 1\n        '\\n' => 2\n        '0' => 3\n        _ => 0\n    }\n}\n",
+    );
+    assert_formats(
+        "fn f(c: Char) -> Int { match c { 'a' ..= 'z' => 1, _ => 0 } }\n",
+        "fn f(c: Char) -> Int {\n    match c {\n        'a'..='z' => 1\n        _ => 0\n    }\n}\n",
+    );
+}
+
+#[test]
+fn int_ranges_print_as_written() {
+    assert_stable(
+        "fn f(n: Int) -> Int {\n    match n {\n        0..10 => 1\n        10..=99 => 2\n        _ => 0\n    }\n}\n",
+    );
+    assert_formats(
+        "fn f(n: Int) -> Int { match n { 0 .. 10 => 1, _ => 0 } }\n",
+        "fn f(n: Int) -> Int {\n    match n {\n        0..10 => 1\n        _ => 0\n    }\n}\n",
+    );
+}
+
+#[test]
+fn let_and_for_destructuring_print_as_written() {
+    assert_stable(
+        "fn main() {\n    let (q, r) = div_mod(7, 2)\n    let mut (a, b) = (1, 2)\n    for (name, n) in pairs { println(name) }\n}\n",
+    );
+    assert_formats(
+        "fn main() { let (q,r)=div_mod(7, 2)\n for (name,n) in pairs { println(name) } }\n",
+        "fn main() {\n    let (q, r) = div_mod(7, 2)\n    for (name, n) in pairs { println(name) }\n}\n",
+    );
+}
+
+#[test]
+fn a_tuple_pattern_arm_after_a_bare_return_arm_prints_without_a_semicolon() {
+    // The newline rule ends the bare `return` at its line break, so the
+    // next arm's `(` is not read as a call (5.0 spec §3.5).
+    assert_stable(
+        "fn f(p: (Int, Int)) -> Int {\n    match p {\n        (0, 0) => return 0\n        (a, b) => a + b\n    }\n}\n\nfn g(p: (Int, Int)) {\n    match p {\n        (0, _) => return\n        (a, _) => println(a)\n    }\n}\n",
+    );
+}
diff --git a/crates/nova-hir/src/lib.rs b/crates/nova-hir/src/lib.rs
index 908c5915d2b8c0ab65a28031e6017cd244ae7714..b3771f25334102e57f6f25a88ccc12b593a0101d 100644
--- a/crates/nova-hir/src/lib.rs
+++ b/crates/nova-hir/src/lib.rs
@@ -1302,6 +1302,60 @@ pub enum Pattern {
 mod tests {
     use super::*;
 
+    #[test]
+    fn a_patterns_bindings_are_each_local_once_in_order() {
+        use nova_resolver::DefId;
+        let (a, b) = (LocalId(3), LocalId(1));
+        // `(a, V(_, b)) | (a, V(b, _))`: an or-pattern's alternatives bind the
+        // same locals, so each is listed once, in the first one's order.
+        let alt = |first: Pattern, second: Pattern| {
+            Pattern::Tuple(vec![
+                Pattern::Bind(a),
+                Pattern::Variant {
+                    sum: DefId(0),
+                    variant: 0,
+                    fields: vec![first, second],
+                },
+            ])
+        };
+        let p = Pattern::Or(vec![
+            alt(Pattern::Wildcard, Pattern::Bind(b)),
+            alt(Pattern::Bind(b), Pattern::Wildcard),
+        ]);
+        assert_eq!(p.bindings(), vec![a, b]);
+        assert!(Pattern::Range { lo: 1, hi: 2 }.bindings().is_empty());
+    }
+
+    #[test]
+    fn only_todays_shapes_are_flat() {
+        use nova_resolver::DefId;
+        let variant = |fields| Pattern::Variant {
+            sum: DefId(0),
+            variant: 1,
+            fields,
+        };
+        for flat in [
+            Pattern::Wildcard,
+            Pattern::Bind(LocalId(0)),
+            Pattern::LitInt(1),
+            Pattern::LitBool(true),
+            Pattern::LitStr("s".to_string()),
+            Pattern::LitChar('c'),
+            variant(vec![Pattern::Bind(LocalId(0)), Pattern::Wildcard]),
+        ] {
+            assert!(flat.is_flat(), "{flat:?}");
+        }
+        for nested in [
+            variant(vec![Pattern::LitInt(0)]),
+            variant(vec![variant(vec![])]),
+            Pattern::Tuple(vec![Pattern::Wildcard]),
+            Pattern::Or(vec![Pattern::LitInt(0), Pattern::LitInt(1)]),
+            Pattern::Range { lo: 0, hi: 9 },
+        ] {
+            assert!(!nested.is_flat(), "{nested:?}");
+        }
+    }
+
     #[test]
     fn subst_recurses_into_a_projection_without_normalizing() {
         use nova_resolver::DefId;
diff --git a/crates/nova-lsp/src/completion.rs b/crates/nova-lsp/src/completion.rs
index 1787994f2d7f6b2dec873f59c3b679e6f2c06de4..548fa31f04e2c60d857470ba947e57a46541b72d 100644
--- a/crates/nova-lsp/src/completion.rs
+++ b/crates/nova-lsp/src/completion.rs
@@ -185,6 +185,34 @@ fn names(analysis: &Analysis, path: &Path) -> Vec<lsp::CompletionItem> {
 mod tests {
     use super::*;
 
+    /// A binding inside a nested pattern is offered in its arm, and a
+    /// member after it (5.0 spec §4.3).
+    #[test]
+    fn a_binding_in_a_nested_pattern_completes() {
+        let text = "record Tok { text: String }\n\n\
+                    fn f(o: Option<(Int, Tok)>) -> Int {\n    match o {\n        \
+                    Some((count, tok)) if count > 0 => {\n            \
+                    let a = tok.text.len()\n            \n            tok.\n            a\n        }\n        \
+                    _ => 0\n    }\n}\n\nfn main() {}\n";
+        let path = std::env::temp_dir()
+            .join("nova-lsp-complete-pattern")
+            .join("main.nova");
+        let overlay = Overlay::default().with(&path, text.to_string());
+        let labels = |offset: usize| -> Vec<String> {
+            complete(&path, text, offset as u32, &overlay)
+                .into_iter()
+                .map(|i| i.label)
+                .collect()
+        };
+        let blank = text.find("len()\n            ").unwrap() + "len()\n            ".len();
+        let names = labels(blank);
+        for want in ["count", "tok", "a", "o"] {
+            assert!(names.contains(&want.to_string()), "{want}: {names:?}");
+        }
+        let members = labels(text.find("tok.\n").unwrap() + 4);
+        assert!(members.contains(&"text".to_string()), "{members:?}");
+    }
+
     #[test]
     fn a_signature_stops_at_its_body_outside_parentheses() {
         assert_eq!(
diff --git a/crates/nova-lsp/src/hover.rs b/crates/nova-lsp/src/hover.rs
index 162c7ae4b318c5af3b09c2a20cdd1ce509451e3b..b73eb29ffbb3157059a3c98e531bf754b21b115c 100644
--- a/crates/nova-lsp/src/hover.rs
+++ b/crates/nova-lsp/src/hover.rs
@@ -305,6 +305,42 @@ mod tests {
         );
     }
 
+    /// A binding inside a nested pattern, an or-pattern's second
+    /// alternative's and a destructuring `let`'s hover with their types
+    /// (5.0 spec §4.3).
+    #[test]
+    fn a_binding_in_a_nested_pattern_hovers_with_its_type() {
+        let text = "record Tok { text: String }\n\n\
+                    fn f(o: Option<(Int, Tok)>, r: Result<Int, Int>) -> Int {\n    \
+                    let (lo, (hi, _)) = (1, (2, 3))\n    \
+                    let k = match r {\n        Ok(v) | Err(v) => v\n    }\n    \
+                    match o {\n        Some((count, tok)) if count > lo => count + tok.text.len() + hi + k\n        \
+                    _ => 0\n    }\n}\n\nfn main() {}\n";
+        let path = std::env::temp_dir()
+            .join("nova-lsp-hover-pattern")
+            .join("main.nova");
+        let overlay = crate::workspace::Overlay::default().with(&path, text.to_string());
+        let options = crate::analysis::options(None, true);
+        let answer = crate::analysis::answering(&path, &overlay, &options).expect("an answer");
+        let shown = |at: usize| {
+            let lsp::HoverContents::Markup(found) =
+                hover(&answer, at as u32).expect("a hover").contents
+            else {
+                panic!("not Markdown");
+            };
+            found.value
+        };
+        let count = shown(text.find("count + tok").unwrap());
+        assert!(count.contains("count: Int"), "{count}");
+        let tok = shown(text.find("tok.text").unwrap());
+        assert!(tok.contains("tok: Tok"), "{tok}");
+        let hi = shown(text.find("hi + k").unwrap());
+        assert!(hi.contains("hi: Int"), "{hi}");
+        // The second alternative's `v` is the first's local.
+        let v = shown(text.find("Err(v)").unwrap() + 4);
+        assert!(v.contains("v: Int"), "{v}");
+    }
+
     #[test]
     fn an_item_line_keeps_pub_async_and_where_and_stops_at_its_body() {
         let text = "    pub async fn f<T>(x: T) -> T\n    where T: Show\n    { x }\n";
diff --git a/crates/nova-mir/tests/lower_tests.rs b/crates/nova-mir/tests/lower_tests.rs
index d22bc0e2f696e4fbb2f439b1a9ba3e49945ce733..c01a40087afcbfb7f2415492947a9328bba1c0a7 100644
--- a/crates/nova-mir/tests/lower_tests.rs
+++ b/crates/nova-mir/tests/lower_tests.rs
@@ -2346,3 +2346,56 @@ fn a_tuple_of_one_in_a_bound_message_keeps_its_comma() {
         assert!(d.message.contains("`(Int,): "), "{d:?}");
     }
 }
+
+/// Until test-chain lowering (5.0 spec §4.6), lowering refuses a match it
+/// would lower wrongly, one with a nested pattern or a guard: E0900.
+#[test]
+fn a_nested_or_guarded_match_is_refused_until_chains() {
+    for body in [
+        "match o {\n        Some(Some(n)) => n\n        _ => 0\n    }",
+        "match o {\n        Some(_) if true => 1\n        _ => 0\n    }",
+    ] {
+        let src = format!(
+            "fn f(o: Option<Option<Int>>) -> Int {{\n    {body}\n}}\n\
+             fn main() {{ println(\"${{f(None)}}\") }}"
+        );
+        let file_id = FileId::DUMMY;
+        let (tokens, _) = lex(&src, file_id);
+        let (ast, _) = parse(&tokens, file_id);
+        let resolved = resolve(&ast.expect("an AST"));
+        let checked = check(&resolved.file, &resolved.definitions);
+        assert!(checked.diagnostics.is_empty(), "{:?}", checked.diagnostics);
+        let diags = match lower_module(&checked.module) {
+            Ok(_) => panic!("lowered: {body}"),
+            Err(d) => d,
+        };
+        assert!(diags.iter().any(|d| d.code == "E0900"), "{diags:?}");
+    }
+}
+
+/// A flat `Char` match, which the checker takes from Task 3 on, switches on
+/// its literals' code points, as an `Int` match switches on its literals.
+#[test]
+fn a_flat_char_match_switches_on_its_code_points() {
+    let mir = mir_for(
+        "fn f(c: Char) -> Int {\n    match c {\n        'a' => 1\n        'b' => 2\n        _ => 3\n    }\n}\n\
+         fn main() { println(\"${f('b')}\") }",
+    );
+    let instance = "f.";
+    let f = mir
+        .functions
+        .iter()
+        .find(|f| f.name == "f" || f.name.starts_with(instance))
+        .expect("f");
+    let keys: Vec<i64> = f
+        .blocks
+        .iter()
+        .find_map(|b| match &b.term {
+            nova_mir::Terminator::Switch { arms, .. } => {
+                Some(arms.iter().map(|(v, _)| *v).collect())
+            }
+            _ => None,
+        })
+        .expect("a switch");
+    assert_eq!(keys, ['a' as i64, 'b' as i64]);
+}
diff --git a/crates/nova-typeck/src/check.rs b/crates/nova-typeck/src/check.rs
index bfae12a47d8a2410fca261d9f492ff05c021961e..14fc6e4dc6c9940c5f9baebdd922fe5a8ccabc20 100644
--- a/crates/nova-typeck/src/check.rs
+++ b/crates/nova-typeck/src/check.rs
@@ -23,6 +23,8 @@ mod fixes;
 #[cfg(test)]
 mod lang_tests;
 #[cfg(test)]
+mod pattern_tests;
+#[cfg(test)]
 mod tuple_tests;
 mod view;
 
diff --git a/crates/nova-typeck/src/check/pattern_tests.rs b/crates/nova-typeck/src/check/pattern_tests.rs
new file mode 100644
index 0000000000000000000000000000000000000000..5a7e4b64e664556a7191006b4c2b93cd7551308f
--- /dev/null
+++ b/crates/nova-typeck/src/check/pattern_tests.rs
@@ -0,0 +1,734 @@
+//! The checker's tests of 5.0b's patterns (spec
+//! `docs/superpowers/specs/2026-10-26-phase-5-0-the-language-and-std-a-compiler-needs-design.md`
+//! §4.3, §4.4): nested, tuple, or-, `Char` and range patterns, guards,
+//! `let` and `for` destructuring, and the codes they meet: E0001, E0010,
+//! E0016, E0020, E0021, E0022, E0100, E0101 and E0104.
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
+fn of(src: &str, severity: Severity) -> Vec<Diagnostic> {
+    checked(src)
+        .diagnostics
+        .into_iter()
+        .filter(|d| d.severity == severity)
+        .collect()
+}
+
+/// The one error of `src`, which has `code`.
+fn only(src: &str, code: &str) -> Diagnostic {
+    let all = of(src, Severity::Error);
+    assert_eq!(all.len(), 1, "one error in {all:#?}");
+    assert_eq!(all[0].code, code, "{all:#?}");
+    all[0].clone()
+}
+
+/// The one warning of `src`, which has no error; it has `code`.
+fn only_warning(src: &str, code: &str) -> Diagnostic {
+    let errors = of(src, Severity::Error);
+    assert!(errors.is_empty(), "{errors:#?}");
+    let all = of(src, Severity::Warning);
+    assert_eq!(all.len(), 1, "one warning in {all:#?}");
+    assert_eq!(all[0].code, code, "{all:#?}");
+    all[0].clone()
+}
+
+fn clean(src: &str) {
+    let all = checked(src).diagnostics;
+    assert!(all.is_empty(), "{all:#?}");
+}
+
+/// The text of `src` under the primary label of `d`.
+fn labelled<'s>(src: &'s str, d: &Diagnostic) -> &'s str {
+    let label = d
+        .labels
+        .iter()
+        .find(|l| l.primary)
+        .expect("a primary label");
+    &src[label.span.start as usize..label.span.end as usize]
+}
+
+fn label_text(d: &Diagnostic) -> String {
+    d.labels
+        .iter()
+        .find(|l| l.primary)
+        .expect("a primary label")
+        .message
+        .clone()
+}
+
+/// The checked arms' patterns of the first `match` in `name`.
+fn patterns_of(r: &CheckResult, name: &str) -> Vec<hir::Pattern> {
+    fn walk<'e>(e: &'e hir::Expr, out: &mut Vec<&'e hir::Expr>) {
+        out.push(e);
+        for c in child_exprs(e) {
+            walk(c, out);
+        }
+    }
+    let f = r
+        .module
+        .functions
+        .iter()
+        .find(|f| f.name == name)
+        .expect("the function");
+    let mut all = Vec::new();
+    walk(&f.body, &mut all);
+    all.into_iter()
+        .find_map(|e| match &e.kind {
+            hir::ExprKind::Match { arms, .. } => {
+                Some(arms.iter().map(|a| a.pattern.clone()).collect())
+            }
+            _ => None,
+        })
+        .expect("a match")
+}
+
+const SUMS: &str = "type Opt<T> =\n  | Yes(T)\n  | No\n\
+                    type Res<T, E> =\n  | Win(T)\n  | Fail(E)\n\
+                    type Tok =\n  | Ident(String)\n  | Num(Int)\n  | Sym(Char)\n  | Eof\n\
+                    type Wrap =\n  | Wrap(Int, String)\n";
+
+fn with_sums(body: &str) -> String {
+    format!("{SUMS}{body}\nfn main() {{}}\n")
+}
+
+// --- The HIR pattern ---
+
+#[test]
+fn nested_patterns_check_to_a_recursive_hir_pattern() {
+    let src = with_sums(
+        "fn f(o: Opt<Res<(Int, Tok), String>>) -> Int {\n    match o {\n        \
+         Yes(Win((n, Tok::Num(0 | 1)))) => n\n        Yes(Win((_, Tok::Sym('a'..='z')))) => 1\n        \
+         Yes(Win((5..10, _))) => 2\n        Yes(Win(_)) => 3\n        Yes(Fail(\"x\")) => 4\n        \
+         Yes(Fail(_)) | No => 5\n    }\n}",
+    );
+    let r = checked(&src);
+    assert!(r.diagnostics.is_empty(), "{:#?}", r.diagnostics);
+    let pats = patterns_of(&r, "f");
+    let hir::Pattern::Variant { fields, .. } = &pats[0] else {
+        panic!("{:?}", pats[0])
+    };
+    let hir::Pattern::Variant { fields: inner, .. } = &fields[0] else {
+        panic!("{fields:?}")
+    };
+    let hir::Pattern::Tuple(elems) = &inner[0] else {
+        panic!("{inner:?}")
+    };
+    assert!(matches!(elems[0], hir::Pattern::Bind(_)), "{elems:?}");
+    let hir::Pattern::Variant { fields: num, .. } = &elems[1] else {
+        panic!("{elems:?}")
+    };
+    assert_eq!(
+        num[0],
+        hir::Pattern::Or(vec![hir::Pattern::LitInt(0), hir::Pattern::LitInt(1)])
+    );
+    // A `Char` range is stored by code point, an exclusive `Int` range with
+    // its end included.
+    let range = |p: &hir::Pattern| -> hir::Pattern {
+        let hir::Pattern::Variant { fields, .. } = p else {
+            panic!("{p:?}")
+        };
+        let hir::Pattern::Variant { fields, .. } = &fields[0] else {
+            panic!("{fields:?}")
+        };
+        let hir::Pattern::Tuple(elems) = &fields[0] else {
+            panic!("{fields:?}")
+        };
+        elems
+            .iter()
+            .find(|e| !matches!(e, hir::Pattern::Wildcard))
+            .cloned()
+            .expect("an element")
+    };
+    let hir::Pattern::Variant { fields: sym, .. } = range(&pats[1]) else {
+        panic!()
+    };
+    assert_eq!(
+        sym[0],
+        hir::Pattern::Range {
+            lo: 'a' as i64,
+            hi: 'z' as i64
+        }
+    );
+    assert_eq!(range(&pats[2]), hir::Pattern::Range { lo: 5, hi: 9 });
+    assert!(matches!(pats[5], hir::Pattern::Or(_)), "{:?}", pats[5]);
+}
+
+#[test]
+fn an_or_pattern_binds_one_local_in_every_alternative() {
+    let src = with_sums("fn f(p: (Int, Int)) -> Int {\n    match p {\n        (a, 0) | (0, a) => a\n        _ => 0\n    }\n}");
+    let r = checked(&src);
+    assert!(r.diagnostics.is_empty(), "{:#?}", r.diagnostics);
+    let pats = patterns_of(&r, "f");
+    let hir::Pattern::Or(alts) = &pats[0] else {
+        panic!("{:?}", pats[0])
+    };
+    assert_eq!(alts[0].bindings(), alts[1].bindings());
+    assert_eq!(alts[0].bindings().len(), 1);
+}
+
+#[test]
+fn a_char_literal_pattern_checks() {
+    clean(&with_sums(
+        "fn f(c: Char) -> Int {\n    match c {\n        'a' => 1\n        '\\n' | '\\t' => 2\n        _ => 3\n    }\n}",
+    ));
+}
+
+#[test]
+fn guards_see_the_arms_bindings_and_must_be_bool() {
+    clean(&with_sums(
+        "fn f(t: Tok) -> Int {\n    match t {\n        Num(n) if n > 0 => n\n        \
+         Ident(s) if s == \"x\" => 1\n        _ => 0\n    }\n}",
+    ));
+    let src = with_sums(
+        "fn f(n: Int) -> Int {\n    match n {\n        x if x => 1\n        _ => 0\n    }\n}",
+    );
+    let d = only(&src, "E0010");
+    assert!(d.message.contains("a match guard"), "{d:?}");
+    assert_eq!(labelled(&src, &d), "x");
+}
+
+// --- E0100 ---
+
+#[test]
+fn an_alternative_missing_a_binding_is_e0100_naming_it() {
+    let src = with_sums(
+        "fn f(t: Res<Int, Int>) -> Int {\n    match t {\n        Win(x) | Fail(_) => 1\n    }\n}",
+    );
+    let d = only(&src, "E0100");
+    assert_eq!(d.message, "the alternatives bind different names");
+    assert!(label_text(&d).contains("`x`"), "{d:?}");
+    assert_eq!(labelled(&src, &d), "Fail(_)");
+}
+
+#[test]
+fn an_alternative_with_an_extra_binding_is_e0100_naming_it() {
+    let src = with_sums(
+        "fn f(t: Res<Int, Int>) -> Int {\n    match t {\n        Win(_) | Fail(y) => 1\n    }\n}",
+    );
+    let d = only(&src, "E0100");
+    assert!(label_text(&d).contains("`y`"), "{d:?}");
+    assert_eq!(labelled(&src, &d), "y");
+}
+
+#[test]
+fn a_binding_at_two_types_is_e0100() {
+    let src = with_sums("fn f(t: Res<Int, String>) -> Int {\n    match t {\n        Win(x) | Fail(x) => 1\n    }\n}");
+    let d = only(&src, "E0100");
+    assert!(d.message.contains("different types"), "{d:?}");
+    assert!(label_text(&d).contains("`String`"), "{d:?}");
+    assert!(label_text(&d).contains("`Int`"), "{d:?}");
+}
+
+#[test]
+fn nested_or_patterns_bind_the_outer_first_alternatives_names() {
+    clean(&with_sums(
+        "fn f(t: (Opt<Res<Int, Int>>, Int)) -> Int {\n    match t {\n        \
+         (Yes(Win(x) | Fail(x)), _) | (No, x) => x\n    }\n}",
+    ));
+    let src = with_sums(
+        "fn f(t: Opt<Res<Int, Int>>) -> Int {\n    match t {\n        \
+         Yes(Win(x) | Fail(x)) | No => 1\n    }\n}",
+    );
+    let d = only(&src, "E0100");
+    assert_eq!(labelled(&src, &d), "No");
+}
+
+// --- E0101 ---
+
+#[test]
+fn an_empty_range_is_e0101() {
+    for (pat, why) in [
+        ("5..=4", "`5` is above `4`"),
+        ("5..5", "`5..5` stops before its start"),
+        ("9..2", "`9` is above `2`"),
+    ] {
+        let src = with_sums(&format!(
+            "fn f(n: Int) -> Int {{\n    match n {{\n        {pat} => 1\n        _ => 0\n    }}\n}}"
+        ));
+        let d = only(&src, "E0101");
+        assert_eq!(d.message, "this range matches nothing");
+        assert_eq!(label_text(&d), why);
+        assert_eq!(labelled(&src, &d), pat);
+    }
+    let src = with_sums(
+        "fn f(c: Char) -> Int {\n    match c {\n        'z'..='a' => 1\n        _ => 0\n    }\n}",
+    );
+    let d = only(&src, "E0101");
+    assert!(label_text(&d).contains("'z'"), "{d:?}");
+}
+
+#[test]
+fn an_empty_range_does_not_make_the_arms_after_it_unreachable() {
+    let src = with_sums(
+        "fn f(n: Int) -> Int {\n    match n {\n        3..=1 => 1\n        2 => 2\n        _ => 0\n    }\n}",
+    );
+    only(&src, "E0101");
+    assert!(of(&src, Severity::Warning).is_empty());
+}
+
+#[test]
+fn a_one_value_range_is_not_empty() {
+    clean(&with_sums(
+        "fn f(n: Int) -> Int {\n    match n {\n        4..=4 => 1\n        5..6 => 2\n        _ => 0\n    }\n}",
+    ));
+}
+
+// --- E0010 ---
+
+#[test]
+fn a_range_over_another_type_is_e0010() {
+    let src = with_sums(
+        "fn f(s: String) -> Int {\n    match s {\n        1..=3 => 1\n        _ => 0\n    }\n}",
+    );
+    let d = only(&src, "E0010");
+    assert!(d.message.contains("`Int`"), "{d:?}");
+    assert!(d.message.contains("`String`"), "{d:?}");
+    let src = with_sums(
+        "fn f(n: Int) -> Int {\n    match n {\n        'a'..='z' => 1\n        _ => 0\n    }\n}",
+    );
+    only(&src, "E0010");
+}
+
+#[test]
+fn a_tuple_pattern_of_the_wrong_arity_or_type_is_e0010() {
+    let src =
+        with_sums("fn f(p: (Int, Int)) -> Int {\n    match p {\n        (a, b, c) => a\n    }\n}");
+    let d = only(&src, "E0010");
+    assert!(d.message.contains("3 elements"), "{d:?}");
+    assert!(d.message.contains("`(Int, Int)`"), "{d:?}");
+    let src = with_sums("fn f(n: Int) -> Int {\n    match n {\n        (a, b) => a\n    }\n}");
+    let d = only(&src, "E0010");
+    assert!(d.message.contains("`Int`"), "{d:?}");
+}
+
+#[test]
+fn a_char_pattern_against_an_int_is_e0010() {
+    let src = with_sums(
+        "fn f(n: Int) -> Int {\n    match n {\n        'a' => 1\n        _ => 0\n    }\n}",
+    );
+    only(&src, "E0010");
+}
+
+// --- A bare name inside a variant (5.0b's one change of meaning) ---
+
+#[test]
+fn a_variant_name_inside_a_variant_matches_that_variant() {
+    // Before 5.0b, `Yes(No)` bound a local named `No`; now it is the variant.
+    let src = with_sums(
+        "fn f(o: Opt<Opt<Int>>) -> Int {\n    match o {\n        Yes(No) => 0\n        \
+         Yes(Yes(n)) => n\n        No => 1\n    }\n}",
+    );
+    let r = checked(&src);
+    assert!(r.diagnostics.is_empty(), "{:#?}", r.diagnostics);
+    let pats = patterns_of(&r, "f");
+    let hir::Pattern::Variant { fields, .. } = &pats[0] else {
+        panic!("{:?}", pats[0])
+    };
+    assert!(
+        matches!(&fields[0], hir::Pattern::Variant { fields, .. } if fields.is_empty()),
+        "{fields:?}"
+    );
+    // Without `Yes(Yes(n))` the match misses it: `No` inside covers no more.
+    let src = with_sums(
+        "fn f(o: Opt<Opt<Int>>) -> Int {\n    match o {\n        Yes(No) => 0\n        No => 1\n    }\n}",
+    );
+    let e = only(&src, "E0020");
+    assert!(e.message.contains("Yes(Yes(_))"), "{e:#?}");
+}
+
+#[test]
+fn another_types_variant_name_inside_a_variant_is_e0001_with_a_note() {
+    let src = with_sums(
+        "fn f(o: Opt<Int>) -> Int {\n    match o {\n        Yes(Eof) => 0\n        _ => 1\n    }\n}",
+    );
+    let e = only(&src, "E0001");
+    assert_eq!(e.message, "`Eof` is not a variant of the matched type");
+    assert_eq!(labelled(&src, &e), "Eof");
+    assert_eq!(
+        e.notes,
+        vec![
+            "a name inside a pattern that names a variant matches that variant; \
+             to bind a value, use a lower-case name"
+                .to_string()
+        ]
+    );
+}
+
+#[test]
+fn a_lower_case_name_inside_a_variant_still_binds() {
+    let src = with_sums(
+        "fn f(o: Opt<Int>) -> Int {\n    match o {\n        Yes(x) => x\n        No => 0\n    }\n}",
+    );
+    let r = checked(&src);
+    assert!(r.diagnostics.is_empty(), "{:#?}", r.diagnostics);
+    let pats = patterns_of(&r, "f");
+    let hir::Pattern::Variant { fields, .. } = &pats[0] else {
+        panic!("{:?}", pats[0])
+    };
+    assert!(matches!(fields[0], hir::Pattern::Bind(_)), "{fields:?}");
+}
+
+#[test]
+fn a_variant_with_fields_named_bare_inside_a_variant_is_e0016() {
+    // `Yes(Num)` is the variant `Num`, which has a field: E0016, where before
+    // 5.0b it bound a local named `Num`.
+    let src = with_sums(
+        "fn f(o: Opt<Tok>) -> Int {\n    match o {\n        Yes(Num) => 0\n        _ => 1\n    }\n}",
+    );
+    only(&src, "E0016");
+}
+
+#[test]
+fn an_upper_case_binding_is_e0104() {
+    // `Rde` names no variant, so it binds, and matches every value.
+    let src = with_sums(
+        "fn f(o: Opt<Int>) -> Int {\n    match o {\n        Yes(Rde) => Rde\n        No => 0\n    }\n}",
+    );
+    let d = only_warning(&src, "E0104");
+    assert_eq!(
+        d.message,
+        "`Rde` binds a value: no variant of that name is in scope"
+    );
+    assert_eq!(labelled(&src, &d), "Rde");
+    assert!(
+        d.notes[0].contains("to bind a value, use a lower-case name"),
+        "{d:?}"
+    );
+    // At the top of an arm, and in a destructuring `let`, too.
+    let src = with_sums(
+        "fn f(n: Int) -> Int {\n    let (A, b) = (n, 1)\n    match n {\n        0 => b\n        Whatever => Whatever + A\n    }\n}",
+    );
+    assert!(of(&src, Severity::Error).is_empty());
+    let all = of(&src, Severity::Warning);
+    let named: Vec<&str> = all.iter().map(|d| labelled(&src, d)).collect();
+    assert_eq!(named, ["A", "Whatever"], "{all:#?}");
+}
+
+#[test]
+fn a_lower_case_or_underscored_binding_or_a_real_variant_is_not_e0104() {
+    clean(&with_sums(
+        "fn f(o: Opt<Opt<Int>>) -> Int {\n    match o {\n        Yes(No) => 0\n        \
+         Yes(Yes(x)) => x\n        No => 1\n    }\n}\n\
+         fn g(o: Opt<Int>) -> Int {\n    match o {\n        Yes(_Upper) => 1\n        _ => 0\n    }\n}",
+    ));
+}
+
+#[test]
+fn an_upper_case_binding_in_an_or_pattern_warns_once() {
+    // The second alternative binds the first one's local: one E0104.
+    let src = with_sums(
+        "fn f(r: Res<Int, Int>) -> Int {\n    match r {\n        Win(Q) | Fail(Q) => Q\n    }\n}",
+    );
+    assert!(of(&src, Severity::Error).is_empty());
+    let all = of(&src, Severity::Warning);
+    assert_eq!(all.len(), 1, "{all:#?}");
+    assert_eq!(all[0].code, "E0104");
+}
+
+#[test]
+fn e0104_on_a_constant_says_to_use_a_guard() {
+    let src = with_sums(
+        "const MAX: Int = 9\nfn f(o: Opt<Int>) -> Int {\n    match o {\n        Yes(MAX) => 1\n        _ => 0\n    }\n}",
+    );
+    let d = only_warning(&src, "E0104");
+    assert_eq!(
+        d.notes,
+        ["a pattern cannot match a constant by name; use a guard: `n if n == MAX`"]
+    );
+    assert!(d.fixes.is_empty(), "{d:?}");
+}
+
+#[test]
+fn e0104_near_a_variant_says_which() {
+    let src = with_sums("fn f(t: Tok) -> Int {\n    match t {\n        Eot => 1\n    }\n}");
+    let d = only_warning(&src, "E0104");
+    assert!(d.notes.iter().any(|n| n == "did you mean `Eof`?"), "{d:?}");
+    assert_eq!(d.fixes.len(), 1, "{d:?}");
+    assert_eq!(d.fixes[0].title, "change `Eot` to `Eof`");
+    // A name near no variant gets no guess.
+    let src = with_sums(
+        "fn f(o: Opt<Int>) -> Int {\n    match o {\n        Yes(Zebra) => Zebra\n        No => 0\n    }\n}",
+    );
+    let d = only_warning(&src, "E0104");
+    assert_eq!(d.notes.len(), 1, "{d:?}");
+    assert!(d.fixes.is_empty(), "{d:?}");
+}
+
+#[test]
+fn an_erroneous_pattern_does_not_make_the_arms_after_it_unreachable() {
+    // `Eof` is `Tok`'s, so the first arm is E0001 and stands in as `_`; the
+    // arms after it are not reported unreachable for that.
+    let src = with_sums(
+        "fn f(o: Opt<Int>) -> Int {\n    match o {\n        Eof => 0\n        Yes(n) => n\n        No => 1\n    }\n}",
+    );
+    only(&src, "E0001");
+    let warnings = of(&src, Severity::Warning);
+    assert!(warnings.is_empty(), "{warnings:#?}");
+}
+
+#[test]
+fn the_forms_still_out_are_e0900() {
+    for (pat, what) in [
+        ("x @ 1", "`@` patterns"),
+        ("1.5", "`Float` literal patterns"),
+        ("[a, b]", "array patterns"),
+    ] {
+        let src = with_sums(&format!(
+            "fn f(n: Int) -> Int {{\n    match n {{\n        {pat} => 1\n        _ => 0\n    }}\n}}"
+        ));
+        let errors = of(&src, Severity::Error);
+        assert!(
+            errors
+                .iter()
+                .any(|d| d.code == "E0900" && d.message.starts_with(what)),
+            "{pat}: {errors:#?}"
+        );
+    }
+}
+
+// --- Exhaustiveness and reachability (spec §4.4) ---
+
+#[test]
+fn a_missing_nested_variant_is_e0020_with_a_nested_witness() {
+    let src = with_sums("fn f(o: Opt<Res<Int, String>>) -> Int {\n    match o {\n        Yes(Win(_)) => 1\n        No => 2\n    }\n}");
+    let d = only(&src, "E0020");
+    assert!(d.message.contains("`Yes(Fail(_))`"), "{d:?}");
+}
+
+#[test]
+fn a_tuple_of_sums_missing_a_combination_is_e0020() {
+    let src = with_sums(
+        "fn f(p: (Opt<Int>, Bool)) -> Int {\n    match p {\n        (Yes(_), _) => 1\n        (No, true) => 2\n    }\n}",
+    );
+    let d = only(&src, "E0020");
+    assert!(d.message.contains("`(No, false)`"), "{d:?}");
+}
+
+#[test]
+fn a_tuple_of_one_witness_keeps_its_comma() {
+    let src = with_sums("fn f(p: (Bool,)) -> Int {\n    match p {\n        (true,) => 1\n    }\n}");
+    let d = only(&src, "E0020");
+    assert!(d.message.contains("`(false,)`"), "{d:?}");
+}
+
+#[test]
+fn an_or_pattern_covers_each_alternative() {
+    clean(&with_sums(
+        "fn f(o: Opt<Res<Int, Int>>) -> Int {\n    match o {\n        Yes(Win(_) | Fail(_)) => 1\n        No => 2\n    }\n}",
+    ));
+}
+
+#[test]
+fn a_guarded_arm_covers_nothing() {
+    let src = with_sums(
+        "fn f(o: Opt<Int>) -> Int {\n    match o {\n        Yes(n) if n > 0 => n\n        No => 0\n    }\n}",
+    );
+    let d = only(&src, "E0020");
+    assert!(d.message.contains("`Yes(_)`"), "{d:?}");
+}
+
+#[test]
+fn literals_and_ranges_need_a_catch_all() {
+    let src = with_sums("fn f(c: Char) -> Int {\n    match c {\n        'a'..='z' => 1\n        'A'..='Z' => 2\n    }\n}");
+    let d = only(&src, "E0020");
+    assert!(d.message.contains("`_`"), "{d:?}");
+    let src = with_sums(
+        "fn f(t: Tok) -> Int {\n    match t {\n        Num(0..=9) => 1\n        Ident(\"x\") => 2\n        Sym(_) | Eof => 3\n    }\n}",
+    );
+    let d = only(&src, "E0020");
+    assert!(d.message.contains("`Ident(_)`"), "{d:?}");
+    assert!(d.message.contains("`Num(_)`"), "{d:?}");
+}
+
+#[test]
+fn a_nested_arm_an_earlier_arm_covers_is_unreachable() {
+    let src = with_sums(
+        "fn f(o: Opt<Opt<Int>>) -> Int {\n    match o {\n        Yes(_) => 1\n        Yes(Yes(1)) => 2\n        No => 3\n    }\n}",
+    );
+    let d = only_warning(&src, "E0021");
+    assert_eq!(labelled(&src, &d), "Yes(Yes(1))");
+}
+
+#[test]
+fn the_same_literal_or_range_again_is_unreachable_but_an_overlap_is_not() {
+    let src = with_sums("fn f(n: Int) -> Int {\n    match n {\n        1..=5 => 1\n        1..=5 => 2\n        _ => 3\n    }\n}");
+    let d = only_warning(&src, "E0021");
+    assert_eq!(labelled(&src, &d), "1..=5");
+    // Ranges are not split (5.0 spec §4.4): a value an earlier range
+    // covers is not reported.
+    clean(&with_sums(
+        "fn f(n: Int) -> Int {\n    match n {\n        1..=5 => 1\n        3 => 2\n        2..=4 => 3\n        _ => 4\n    }\n}",
+    ));
+}
+
+#[test]
+fn an_unreachable_alternative_is_e0021_on_it_alone() {
+    let src = with_sums(
+        "fn f(n: Int) -> Int {\n    match n {\n        0 | 1 | 0 => 1\n        _ => 2\n    }\n}",
+    );
+    let d = only_warning(&src, "E0021");
+    assert_eq!(d.message, "unreachable alternative");
+    let label = d.labels.iter().find(|l| l.primary).unwrap();
+    assert_eq!(
+        label.span.start as usize,
+        src.find("0 | 1 | 0").unwrap() + 8
+    );
+    // One an earlier arm covers, at depth.
+    let src = with_sums(
+        "fn f(o: Opt<Int>) -> Int {\n    match o {\n        Yes(1) => 1\n        Yes(2 | 1) => 2\n        _ => 3\n    }\n}",
+    );
+    let d = only_warning(&src, "E0021");
+    assert_eq!(labelled(&src, &d), "1");
+    // An alternative after a catch-all one.
+    let src = with_sums("fn f(o: Opt<Int>) -> Int {\n    match o {\n        Yes(_) | Yes(3) => 1\n        No => 2\n    }\n}");
+    let d = only_warning(&src, "E0021");
+    assert_eq!(labelled(&src, &d), "Yes(3)");
+}
+
+#[test]
+fn an_alternative_inside_one_reported_is_not_reported_again() {
+    let src = with_sums(
+        "fn f(o: Opt<Int>) -> Int {\n    match o {\n        Yes(_) | Yes(1 | 2) => 1\n        No => 2\n    }\n}",
+    );
+    let d = only_warning(&src, "E0021");
+    assert_eq!(labelled(&src, &d), "Yes(1 | 2)");
+}
+
+#[test]
+fn a_whole_unreachable_arm_reports_the_arm_not_its_alternatives() {
+    let src = with_sums(
+        "fn f(n: Int) -> Int {\n    match n {\n        _ => 1\n        2 | 3 => 2\n    }\n}",
+    );
+    let d = only_warning(&src, "E0021");
+    assert_eq!(d.message, "unreachable match arm");
+}
+
+#[test]
+fn a_generic_sum_matched_at_depth_is_exhaustive() {
+    clean(
+        "type Tree<T> =\n  | Leaf(T)\n  | Node(Tree<T>, Tree<T>)\n\
+         fn f<T>(t: Tree<T>) -> Int {\n    match t {\n        Leaf(_) => 1\n        \
+         Node(Leaf(_), _) => 2\n        Node(Node(_, _), _) => 3\n    }\n}\nfn main() {}\n",
+    );
+}
+
+// --- `let` and `for` ---
+
+#[test]
+fn let_destructures_a_tuple_and_a_one_variant_sum() {
+    let src = with_sums(
+        "fn f() -> Int {\n    let (a, (b, _)) = (1, (2, \"x\"))\n    let Wrap(n, s) = Wrap(3, \"y\")\n    \
+         let mut (c, d) = (4, 5)\n    c += d\n    a + b + n + c + s.len()\n}",
+    );
+    clean(&src);
+}
+
+#[test]
+fn a_destructured_let_binding_is_immutable_without_mut() {
+    let src = with_sums("fn f() -> Int {\n    let (a, b) = (1, 2)\n    a = 3\n    a + b\n}");
+    let d = only(&src, "E0060");
+    // The note says how a pattern's binding is made mutable.
+    assert!(
+        d.notes.iter().any(|n| n.contains("`let mut (…) = …`")),
+        "{d:?}"
+    );
+}
+
+#[test]
+fn a_match_arm_or_a_loop_variable_is_rebound_to_be_mutable() {
+    // `let mut (…)` is a `let`'s alone: an arm's binding is rebound.
+    let src = with_sums(
+        "fn f(o: Opt<Int>) -> Int {\n    match o {\n        Yes(z) => {\n            z = 1\n            z\n        }\n        No => 0\n    }\n}",
+    );
+    let d = only(&src, "E0060");
+    assert_eq!(
+        d.notes,
+        [
+            "`z` is bound by a pattern, whose bindings are immutable: rebind it as \
+          `let mut z = z` to allow assignment"
+        ]
+    );
+    // `for mut x` does not parse: the loop's variable is rebound in the body.
+    let src = with_sums("fn f() {\n    for x in 0..3 {\n        x = 2\n    }\n}");
+    let d = only(&src, "E0060");
+    assert_eq!(
+        d.notes,
+        [
+            "a `for` loop's variable is immutable: rebind it inside the loop's body as \
+          `let mut x = x` to allow assignment"
+        ]
+    );
+}
+
+#[test]
+fn a_let_pattern_with_an_error_is_not_also_e0022() {
+    // `Eof`, another type's variant, is E0001 and becomes `_`, which would
+    // make `Yes(_)` refutable.
+    let src = with_sums("fn f(o: Opt<Int>) {\n    let Yes(Eof) = o\n}");
+    let errors = of(&src, Severity::Error);
+    let codes: Vec<&str> = errors.iter().map(|d| d.code.as_str()).collect();
+    assert!(!codes.contains(&"E0022"), "{errors:#?}");
+    assert!(!codes.is_empty(), "{errors:#?}");
+}
+
+#[test]
+fn a_refutable_let_pattern_is_e0022_with_the_values_it_misses() {
+    let src = with_sums("fn f(o: Opt<Int>) -> Int {\n    let Yes(x) = o\n    x\n}");
+    let d = only(&src, "E0022");
+    assert_eq!(d.message, "refutable pattern in `let`");
+    assert!(label_text(&d).contains("`No`"), "{d:?}");
+    let src = with_sums("fn f(p: (Int, Bool)) -> Int {\n    let (n, true) = p\n    n\n}");
+    let d = only(&src, "E0022");
+    assert!(label_text(&d).contains("`(_, false)`"), "{d:?}");
+    // A literal's column names a value it misses, not `_`.
+    let src = with_sums("fn f(p: (Int, Int)) -> Int {\n    let (a, 5) = p\n    a\n}");
+    let d = only(&src, "E0022");
+    assert!(label_text(&d).contains("`(_, 0)`"), "{d:?}");
+}
+
+#[test]
+fn a_destructured_let_is_a_one_arm_match() {
+    let r = checked(&with_sums(
+        "fn f() -> Int {\n    let (a, b) = (1, 2)\n    a + b\n}",
+    ));
+    assert!(r.diagnostics.is_empty(), "{:#?}", r.diagnostics);
+    let pats = patterns_of(&r, "f");
+    assert_eq!(pats.len(), 1);
+    assert!(matches!(&pats[0], hir::Pattern::Tuple(e) if e.len() == 2));
+}
+
+#[test]
+fn a_tuple_pattern_over_a_range_is_e0010() {
+    let src = with_sums("fn f() {\n    for (a, b) in 0..3 {\n        let c = a\n    }\n}");
+    only(&src, "E0010");
+}
+
+// --- The newline rule and tuple patterns (5.0 spec §3.5) ---
+
+#[test]
+fn a_tuple_pattern_arm_after_a_bare_return_or_an_ordinary_arm_checks() {
+    clean(&with_sums(
+        "fn f(p: (Int, Int)) -> Int {\n    match p {\n        (0, 0) => return 0\n        (1, n) => n\n        \
+         (a, b) => a + b\n    }\n}\nfn g(p: (Int, Int)) {\n    match p {\n        (0, _) => return\n        \
+         (a, b) => {}\n    }\n}",
+    ));
+}
diff --git a/crates/nova-typeck/src/usefulness.rs b/crates/nova-typeck/src/usefulness.rs
index 8d80ecca9a58722cfd70b23c0bb480ae5263b8fa..b4c82efd6076758c8783ce53919bfd652e4a1749 100644
--- a/crates/nova-typeck/src/usefulness.rs
+++ b/crates/nova-typeck/src/usefulness.rs
@@ -319,4 +319,217 @@ mod tests {
         let w = cx().usefulness(&prior, &[ctor(Ctor::Int(1))], &[Ty::Int]);
         assert!(!w.is_empty(), "a fresh literal should be reachable");
     }
+
+    // --- 5.0b: nested, tuple, or- and range patterns (5.0 spec §4.4) ---
+
+    const OPT: DefId = DefId(1);
+    const RES: DefId = DefId(2);
+
+    /// `Opt<T> = Some(T) | None` and `Res<T, E> = Ok(T) | Err(E)`.
+    fn sums() -> Vec<SumType> {
+        let variant = |name: &str, fields: Vec<Ty>| nova_hir::Variant {
+            name: name.to_string(),
+            fields,
+        };
+        vec![
+            SumType {
+                def_id: OPT,
+                name: "Opt".to_string(),
+                generics: 1,
+                variants: vec![variant("Some", vec![Ty::Param(0)]), variant("None", vec![])],
+            },
+            SumType {
+                def_id: RES,
+                name: "Res".to_string(),
+                generics: 2,
+                variants: vec![
+                    variant("Ok", vec![Ty::Param(0)]),
+                    variant("Err", vec![Ty::Param(1)]),
+                ],
+            },
+        ]
+    }
+
+    fn opt(t: Ty) -> Ty {
+        Ty::Sum {
+            def_id: OPT,
+            args: vec![t],
+        }
+    }
+
+    fn res(t: Ty, e: Ty) -> Ty {
+        Ty::Sum {
+            def_id: RES,
+            args: vec![t, e],
+        }
+    }
+
+    fn some(p: Pat) -> Pat {
+        Pat::Ctor(Ctor::Variant(OPT, 0), vec![p])
+    }
+
+    fn none() -> Pat {
+        Pat::Ctor(Ctor::Variant(OPT, 1), vec![])
+    }
+
+    fn ok(p: Pat) -> Pat {
+        Pat::Ctor(Ctor::Variant(RES, 0), vec![p])
+    }
+
+    fn err(p: Pat) -> Pat {
+        Pat::Ctor(Ctor::Variant(RES, 1), vec![p])
+    }
+
+    fn tuple(ps: Vec<Pat>) -> Pat {
+        Pat::Ctor(Ctor::Tuple(ps.len() as u32), ps)
+    }
+
+    /// The single witness of `rows` against `ty`, rendered loosely.
+    fn witnesses(cx: &MatchCx, rows: Vec<Pat>, ty: Ty) -> Vec<Pat> {
+        let matrix: Vec<Vec<Pat>> = rows.into_iter().map(|p| vec![p]).collect();
+        cx.usefulness(&matrix, &[Pat::Wild], &[ty])
+            .into_iter()
+            .map(|mut w| w.remove(0))
+            .collect()
+    }
+
+    #[test]
+    fn a_nested_witness_names_the_inner_variant() {
+        let sums = sums();
+        let cx = MatchCx::new(&sums);
+        let ty = opt(res(Ty::Int, Ty::String));
+        let w = witnesses(&cx, vec![some(ok(Pat::Wild)), none()], ty);
+        assert_eq!(w.len(), 1, "{w:?}");
+        match &w[0] {
+            Pat::Ctor(Ctor::Variant(OPT, 0), args) => {
+                assert!(
+                    matches!(&args[0], Pat::Ctor(Ctor::Variant(RES, 1), _)),
+                    "{w:?}"
+                )
+            }
+            other => panic!("{other:?}"),
+        }
+    }
+
+    #[test]
+    fn a_nested_arm_an_earlier_one_covers_is_not_useful() {
+        let sums = sums();
+        let cx = MatchCx::new(&sums);
+        let ty = opt(opt(Ty::Int));
+        let prior = vec![vec![some(Pat::Wild)]];
+        assert!(cx
+            .usefulness(
+                &prior,
+                &[some(some(ctor(Ctor::Int(1))))],
+                std::slice::from_ref(&ty)
+            )
+            .is_empty());
+        // First match wins: the more specific arm first leaves the general
+        // one useful.
+        let prior = vec![vec![some(some(ctor(Ctor::Int(1))))]];
+        assert!(!cx.usefulness(&prior, &[some(Pat::Wild)], &[ty]).is_empty());
+    }
+
+    #[test]
+    fn an_or_pattern_row_covers_every_alternative() {
+        let sums = sums();
+        let cx = MatchCx::new(&sums);
+        let ty = opt(res(Ty::Int, Ty::Int));
+        let rows = vec![some(Pat::Or(vec![ok(Pat::Wild), err(Pat::Wild)])), none()];
+        assert!(witnesses(&cx, rows, ty.clone()).is_empty());
+        // An or-pattern as the candidate row is useful when one
+        // alternative is.
+        let prior = vec![vec![some(ok(Pat::Wild))]];
+        let q = Pat::Or(vec![some(ok(Pat::Wild)), none()]);
+        assert!(!cx
+            .usefulness(&prior, &[q], std::slice::from_ref(&ty))
+            .is_empty());
+        let q = Pat::Or(vec![some(ok(ctor(Ctor::Int(3))))]);
+        assert!(cx.usefulness(&prior, &[q], &[ty]).is_empty());
+    }
+
+    #[test]
+    fn a_tuple_of_sums_needs_every_combination() {
+        let sums = sums();
+        let cx = MatchCx::new(&sums);
+        let ty = Ty::Tuple(vec![opt(Ty::Int), Ty::Bool]);
+        let rows = vec![
+            tuple(vec![some(Pat::Wild), Pat::Wild]),
+            tuple(vec![none(), ctor(Ctor::Bool(true))]),
+        ];
+        let w = witnesses(&cx, rows.clone(), ty.clone());
+        assert_eq!(w.len(), 1, "{w:?}");
+        match &w[0] {
+            Pat::Ctor(Ctor::Tuple(2), args) => {
+                assert!(matches!(args[0], Pat::Ctor(Ctor::Variant(OPT, 1), _)));
+                assert!(matches!(args[1], Pat::Ctor(Ctor::Bool(false), _)));
+            }
+            other => panic!("{other:?}"),
+        }
+        let mut all = rows;
+        all.push(tuple(vec![Pat::Wild, ctor(Ctor::Bool(false))]));
+        assert!(witnesses(&cx, all, ty).is_empty());
+    }
+
+    #[test]
+    fn a_tuple_is_one_constructor_even_when_its_type_is_unknown() {
+        let sums = sums();
+        let cx = MatchCx::new(&sums);
+        let rows = vec![tuple(vec![Pat::Wild, Pat::Wild])];
+        assert!(witnesses(&cx, rows, Ty::Error).is_empty());
+    }
+
+    #[test]
+    fn literals_and_ranges_inside_a_variant_need_a_catch_all() {
+        let sums = sums();
+        let cx = MatchCx::new(&sums);
+        let ty = opt(Ty::Char);
+        let rows = vec![
+            some(ctor(Ctor::Range('a' as i64, 'z' as i64))),
+            some(ctor(Ctor::Char('_'))),
+            none(),
+        ];
+        let w = witnesses(&cx, rows.clone(), ty.clone());
+        assert_eq!(w.len(), 1, "{w:?}");
+        assert!(matches!(&w[0], Pat::Ctor(Ctor::Variant(OPT, 0), a) if matches!(a[0], Pat::Wild)));
+        let mut all = rows;
+        all.push(some(Pat::Wild));
+        assert!(witnesses(&cx, all, ty).is_empty());
+    }
+
+    #[test]
+    fn ranges_are_not_split() {
+        // The same range again is not useful; one inside it still is.
+        let prior = vec![vec![ctor(Ctor::Range(1, 10))]];
+        assert!(cx()
+            .usefulness(&prior, &[ctor(Ctor::Range(1, 10))], &[Ty::Int])
+            .is_empty());
+        assert!(!cx()
+            .usefulness(&prior, &[ctor(Ctor::Range(2, 3))], &[Ty::Int])
+            .is_empty());
+        assert!(!cx()
+            .usefulness(&prior, &[ctor(Ctor::Int(5))], &[Ty::Int])
+            .is_empty());
+        // A range covering every `Int` still needs a catch-all.
+        let all = vec![vec![ctor(Ctor::Range(i64::MIN, i64::MAX))]];
+        assert!(!cx().usefulness(&all, &[Pat::Wild], &[Ty::Int]).is_empty());
+    }
+
+    #[test]
+    fn a_generic_sum_at_depth_takes_its_arguments() {
+        let sums = sums();
+        let cx = MatchCx::new(&sums);
+        // `Opt<Opt<Bool>>`: `Some(Some(true))`, `Some(Some(false))`, `None`
+        // leave `Some(None)`.
+        let ty = opt(opt(Ty::Bool));
+        let rows = vec![
+            some(some(ctor(Ctor::Bool(true)))),
+            some(some(ctor(Ctor::Bool(false)))),
+            none(),
+        ];
+        let w = witnesses(&cx, rows, ty);
+        assert_eq!(w.len(), 1, "{w:?}");
+        assert!(matches!(&w[0], Pat::Ctor(Ctor::Variant(OPT, 0), a)
+            if matches!(a[0], Pat::Ctor(Ctor::Variant(OPT, 1), _))));
+    }
 }
````

**Patch `t3-code`:**

<!-- patch t3-code -->

````diff
diff --git a/crates/nova-hir/src/lib.rs b/crates/nova-hir/src/lib.rs
index b3771f25334102e57f6f25a88ccc12b593a0101d..042d501da14bfc4f92ae4f4248de51c88b6c8431 100644
--- a/crates/nova-hir/src/lib.rs
+++ b/crates/nova-hir/src/lib.rs
@@ -1275,27 +1275,92 @@ pub enum ExprKind {
 #[derive(Debug, Clone)]
 pub struct Arm {
     pub pattern: Pattern,
+    /// `pattern if guard => body`: a `Bool` the arm's bindings are in scope
+    /// for, evaluated after they bind; when it is false, matching goes on
+    /// with the next arm.
+    pub guard: Option<Expr>,
     pub body: Expr,
     pub span: Span,
 }
 
-/// Patterns after checking. Phase 1 supports flat patterns: variant
-/// destructuring binds directly to locals (or discards with `None`).
-#[derive(Debug, Clone)]
+/// A checked pattern (5.0 spec §4.3). Patterns nest: a variant's fields and
+/// a tuple's elements are patterns, to any depth. A pattern carries no type;
+/// the value it tests has the type of the scrutinee at its position.
+#[derive(Debug, Clone, PartialEq)]
 pub enum Pattern {
-    /// `_`
+    /// `_`, and `()` against the unit type.
     Wildcard,
-    /// A catch-all binding: `n => ...`
+    /// A binding, `n` or `mut n`, that matches anything.
     Bind(LocalId),
     LitInt(i64),
     LitBool(bool),
     LitStr(String),
-    /// `Variant(a, _, c)` — one binder slot per payload field.
+    LitChar(char),
+    /// An `Int` range, or a `Char` range by code point, with both ends
+    /// included: `lo..hi` is stored as `lo..=hi - 1`. The checker refuses
+    /// an empty range (E0101), so in a program that compiles `lo <= hi`.
+    Range {
+        lo: i64,
+        hi: i64,
+    },
+    /// `Variant(p, ...)`: one pattern per payload field.
     Variant {
         sum: DefId,
         variant: u32,
-        binders: Vec<Option<LocalId>>,
+        fields: Vec<Pattern>,
     },
+    /// `(p1, ..., pn)`, n of one or more.
+    Tuple(Vec<Pattern>),
+    /// `p1 | p2 | ...`: every alternative binds the same locals.
+    Or(Vec<Pattern>),
+}
+
+impl Pattern {
+    /// The locals this pattern binds, each once, in the order they are
+    /// first met (an or-pattern's are its first alternative's).
+    pub fn bindings(&self) -> Vec<LocalId> {
+        fn walk(p: &Pattern, out: &mut Vec<LocalId>) {
+            match p {
+                Pattern::Bind(l) => {
+                    if !out.contains(l) {
+                        out.push(*l);
+                    }
+                }
+                Pattern::Variant { fields: ps, .. } | Pattern::Tuple(ps) | Pattern::Or(ps) => {
+                    for q in ps {
+                        walk(q, out);
+                    }
+                }
+                Pattern::Wildcard
+                | Pattern::LitInt(_)
+                | Pattern::LitBool(_)
+                | Pattern::LitStr(_)
+                | Pattern::LitChar(_)
+                | Pattern::Range { .. } => {}
+            }
+        }
+        let mut out = Vec::new();
+        walk(self, &mut out);
+        out
+    }
+
+    /// Whether this pattern is one today's flat `match` lowering takes: a
+    /// `_`, a binding, a literal, or a variant whose fields are each `_` or
+    /// a binding.
+    pub fn is_flat(&self) -> bool {
+        match self {
+            Pattern::Wildcard
+            | Pattern::Bind(_)
+            | Pattern::LitInt(_)
+            | Pattern::LitBool(_)
+            | Pattern::LitStr(_)
+            | Pattern::LitChar(_) => true,
+            Pattern::Variant { fields, .. } => fields
+                .iter()
+                .all(|f| matches!(f, Pattern::Wildcard | Pattern::Bind(_))),
+            Pattern::Range { .. } | Pattern::Tuple(_) | Pattern::Or(_) => false,
+        }
+    }
 }
 
 #[cfg(test)]
diff --git a/crates/nova-mir/src/lower.rs b/crates/nova-mir/src/lower.rs
index 636fc5aa44bbb6635b04bd7409d091c98a95458b..3f97162c1af7f77dbf4485f45856bb8d895f991b 100644
--- a/crates/nova-mir/src/lower.rs
+++ b/crates/nova-mir/src/lower.rs
@@ -1278,6 +1278,29 @@ impl<'a> Lowerer<'a> {
         }
         let join = self.new_block();
 
+        // A match with a guard or a pattern today's flat lowering does not
+        // take is refused until test-chain lowering lands (5.0 spec §4.6),
+        // rather than lowered wrongly.
+        if !arms
+            .iter()
+            .all(|a| a.guard.is_none() && a.pattern.is_flat())
+        {
+            self.diagnostics.push(
+                Diagnostic::error(
+                    "E0900",
+                    "matches with nested, tuple, or- or range patterns or guards are not \
+                     supported yet",
+                )
+                .with_primary_label(e.span, "not lowered yet")
+                .with_note(
+                    "the checker accepts this match; lowering it to MIR by test chains \
+                     arrives in a later commit of 5.0b"
+                        .to_string(),
+                ),
+            );
+            return result;
+        }
+
         match &scrutinee.ty {
             Ty::Sum { .. } => {
                 let disc = self.new_temp(MirTy::I64);
@@ -1290,6 +1313,7 @@ impl<'a> Lowerer<'a> {
             Ty::Int | Ty::Char => {
                 self.lower_switch_arms(s, s, arms, result, join, |pat| match pat {
                     hir::Pattern::LitInt(v) => Some(*v),
+                    hir::Pattern::LitChar(c) => Some(*c as i64),
                     _ => None,
                 });
             }
@@ -1421,9 +1445,9 @@ impl<'a> Lowerer<'a> {
                 let dst = self.local_map[local.0 as usize];
                 self.copy(dst, scrut);
             }
-            hir::Pattern::Variant { binders, .. } => {
-                for (i, binder) in binders.iter().enumerate() {
-                    if let Some(local) = binder {
+            hir::Pattern::Variant { fields, .. } => {
+                for (i, field) in fields.iter().enumerate() {
+                    if let hir::Pattern::Bind(local) = field {
                         let dst = self.local_map[local.0 as usize];
                         let ty = self.temps[dst.0 as usize];
                         if ty != MirTy::Unit {
diff --git a/crates/nova-mir/src/mono.rs b/crates/nova-mir/src/mono.rs
index 151ade6de303c89640ae3cca2c479cb6105e9dcb..1253b648b98fe5b7e700141d381230a45e7c0807 100644
--- a/crates/nova-mir/src/mono.rs
+++ b/crates/nova-mir/src/mono.rs
@@ -894,6 +894,7 @@ impl Specializer<'_> {
                     .iter()
                     .map(|a| hir::Arm {
                         pattern: a.pattern.clone(),
+                        guard: a.guard.as_ref().map(|g| self.expr(g)),
                         body: self.expr(&a.body),
                         span: a.span,
                     })
diff --git a/crates/nova-typeck/src/check.rs b/crates/nova-typeck/src/check.rs
index 14fc6e4dc6c9940c5f9baebdd922fe5a8ccabc20..bce2787e79b0ea7cd9d82d99a15f8b2cff4d5737 100644
--- a/crates/nova-typeck/src/check.rs
+++ b/crates/nova-typeck/src/check.rs
@@ -3,7 +3,7 @@
 
 use nova_ast as ast;
 use nova_ast::item::{ExternItem, TraitItem, TypeDef};
-use nova_diagnostics::{suggest, Diagnostic, FileDb, Span, Spanned};
+use nova_diagnostics::{suggest, Diagnostic, FileDb, Severity, Span, Spanned};
 use nova_hir as hir;
 use nova_hir::{LocalId, Ty, TyHead};
 use nova_resolver::{
@@ -24,11 +24,13 @@ mod fixes;
 mod lang_tests;
 #[cfg(test)]
 mod pattern_tests;
+mod patterns;
 #[cfg(test)]
 mod tuple_tests;
 mod view;
 
 use fixes::{Binding, Need};
+use patterns::PatState;
 
 /// A collected function (or method) signature.
 #[derive(Debug, Clone)]
@@ -164,6 +166,7 @@ pub fn check_with(file: &ast::File, defs: &Definitions, options: &CheckOptions)
         sources: options.sources,
         bindings: FxHashMap::default(),
         qualified_callee: None,
+        pats: PatState::default(),
     };
     // Before any collection pass: every later pass builds a generic scope, and
     // one containing `Self` would give the name two meanings at once.
@@ -330,6 +333,9 @@ struct Checker<'a> {
     /// Set by `check_call` to its argument count just before it checks a
     /// two-segment callee path; taken by `check_path` (plan decision 7).
     qualified_callee: Option<usize>,
+    /// The pattern being checked: its or-patterns and `let mut` (5.0 spec
+    /// §4.3).
+    pats: PatState,
 }
 
 /// What the probe met, before its function's inference is finished.
@@ -3346,12 +3352,26 @@ impl<'a> Checker<'a> {
                         }
                         ast::Pattern::Wildcard => ("_".to_string(), false, pattern.span),
                         _ => {
-                            self.error(
-                                "E0022",
-                                "only irrefutable patterns (a name or `_`) are allowed in `let`",
-                                pattern.span,
-                            );
-                            ("_".to_string(), false, pattern.span)
+                            // Destructuring (5.0 spec §4.3): an irrefutable
+                            // pattern, bound by a one-arm `match`. `let mut`
+                            // makes each of its bindings mutable.
+                            self.pats.mutable = *is_mut;
+                            self.pats.in_let = true;
+                            let errors = self.error_count();
+                            let pat = self.check_pattern(fcx, pattern, &value.ty);
+                            self.pats.mutable = false;
+                            self.pats.in_let = false;
+                            if self.error_count() == errors {
+                                self.require_irrefutable(
+                                    fcx,
+                                    &pat,
+                                    &value.ty,
+                                    pattern.span,
+                                    "`let`",
+                                );
+                            }
+                            stmts.push(patterns::let_pattern(pat, value, stmt.span));
+                            continue;
                         }
                     };
                     let local = self.bind_local(
@@ -4539,8 +4559,7 @@ impl<'a> Checker<'a> {
         let end = fcx.new_local_unscoped("__end".to_string(), Ty::Int, false, span);
 
         fcx.scopes.push(FxHashMap::default());
-        let (var_name, var_span) = self.for_loop_var(pattern);
-        let i = self.bind_local(fcx, var_name, Ty::Int, false, var_span, Binding::Other);
+        let i = self.for_loop_binding(fcx, pattern, &Ty::Int);
         fcx.loop_depth += 1;
         let body_hir = self.check_block(fcx, &body.value, body.span);
         fcx.loop_depth -= 1;
@@ -4569,7 +4588,7 @@ impl<'a> Checker<'a> {
             span,
         };
         // Bind the immutable loop variable to the counter each iteration.
-        let bind_i = let_stmt(i, read(counter));
+        let bind_i = i.bind(read(counter), span);
         // increment: __i = __i + 1
         let incr = assign(
             counter,
@@ -4658,42 +4677,6 @@ impl<'a> Checker<'a> {
         }
     }
 
-    /// The name and span a `for` loop binds its element to. Shared by the range
-    /// and iterator desugars: they bind at different types but accept exactly
-    /// the same pattern forms, so `E0022` — and the choice to fall back to `_`
-    /// and keep checking the body rather than bail — is stated once here instead
-    /// of in two copies that can drift.
-    ///
-    /// A `for` loop variable is immutable in both desugars, so assigning it is
-    /// `E0060` — measured: `for i in 0..3 { i = i + 1 }` is
-    /// `error[E0060]: cannot assign to immutable variable 'i'`.
-    ///
-    /// The `..` in the `Ident` arm below discards `ast::Pattern::Ident`'s
-    /// `is_mut`, and that discard is **currently unreachable from source**: the
-    /// parser rejects a `mut` in this position before any pattern exists, so
-    /// there is no binding to make immutable and no `E0060` to compare against.
-    /// Measured, both spellings: `for mut i in 0..3` and `for mut x in v.iter()`
-    /// are each `error[P0001]: expected pattern (in statement), found 'mut'`.
-    /// An earlier version of this comment said `for mut i in …` "gets the same
-    /// immutable binding (and the same `E0060` on assignment) as `for i in …`",
-    /// which describes a program that cannot be written. The discard stays as
-    /// the right answer for if the parser ever accepts the form — not as a
-    /// statement about today's behaviour.
-    fn for_loop_var(&mut self, pattern: &Spanned<ast::Pattern>) -> (String, Span) {
-        match &pattern.value {
-            ast::Pattern::Ident { name, .. } => (name.value.clone(), name.span),
-            ast::Pattern::Wildcard => ("_".to_string(), pattern.span),
-            _ => {
-                self.error(
-                    "E0022",
-                    "a `for` loop variable must be a name or `_`",
-                    pattern.span,
-                );
-                ("_".to_string(), pattern.span)
-            }
-        }
-    }
-
     /// `for x in it { body }` desugars to
     /// `{ let mut __it = it
     ///    while true { match __it.next() { Some(x) => body, None => break } } }`.
@@ -4814,8 +4797,7 @@ impl<'a> Checker<'a> {
         };
 
         fcx.scopes.push(FxHashMap::default());
-        let (var_name, var_span) = self.for_loop_var(pattern);
-        let elem = self.bind_local(fcx, var_name, item_ty, false, var_span, Binding::Other);
+        let elem = self.for_loop_binding(fcx, pattern, &item_ty);
         fcx.loop_depth += 1;
         let body_hir = self.check_block(fcx, &body.value, body.span);
         fcx.loop_depth -= 1;
@@ -4826,14 +4808,15 @@ impl<'a> Checker<'a> {
             ty: Ty::Unit,
             span,
         };
-        let variant = |v, binders| hir::Pattern::Variant {
+        let variant = |v, fields| hir::Pattern::Variant {
             sum: next.option,
             variant: v,
-            binders,
+            fields,
         };
         let arms = vec![
             hir::Arm {
-                pattern: variant(next.some, vec![Some(elem)]),
+                pattern: variant(next.some, vec![elem.into_pattern()]),
+                guard: None,
                 // Wrapped in a block so the arm is `Unit` whatever the body
                 // evaluates to — the range desugar discards the body's value
                 // the same way, by making it a statement.
@@ -4845,6 +4828,7 @@ impl<'a> Checker<'a> {
             },
             hir::Arm {
                 pattern: variant(next.none, Vec::new()),
+                guard: None,
                 body: hir::Expr {
                     kind: hir::ExprKind::Break,
                     ty: Ty::Never,
@@ -5151,7 +5135,7 @@ impl<'a> Checker<'a> {
         if !is_option {
             let err_ty = args.get(1).cloned().unwrap_or(Ty::Error);
             let x = fcx.new_local_unscoped("__err".to_string(), err_ty.clone(), false, span);
-            bad_binders.push(Some(x));
+            bad_binders.push(hir::Pattern::Bind(x));
             bad_args.push(local_expr(x, err_ty, span));
         }
         let early = hir::Expr {
@@ -5168,8 +5152,9 @@ impl<'a> Checker<'a> {
                 pattern: hir::Pattern::Variant {
                     sum,
                     variant: good_idx,
-                    binders: vec![Some(v)],
+                    fields: vec![hir::Pattern::Bind(v)],
                 },
+                guard: None,
                 body: local_expr(v, payload.clone(), span),
                 span,
             },
@@ -5177,8 +5162,9 @@ impl<'a> Checker<'a> {
                 pattern: hir::Pattern::Variant {
                     sum,
                     variant: bad_idx,
-                    binders: bad_binders,
+                    fields: bad_binders,
                 },
+                guard: None,
                 body: hir::Expr {
                     kind: hir::ExprKind::Return(Some(Box::new(early))),
                     ty: Ty::Never,
@@ -6988,14 +6974,14 @@ impl<'a> Checker<'a> {
             // would.
             match self.mutable_fix(name, info.span) {
                 Some(fix) => self.push_fix(Some(fix)),
-                None => self
-                    .diagnostics
-                    .last_mut()
-                    .expect("just pushed")
-                    .notes
-                    .push(format!(
-                        "declare it as `let mut {name}` to allow assignment"
-                    )),
+                None => {
+                    let note = self.immutable_note(name, info.span, "assignment");
+                    self.diagnostics
+                        .last_mut()
+                        .expect("just pushed")
+                        .notes
+                        .push(note)
+                }
             }
         }
         let value = self.check_expr(fcx, rhs);
@@ -7101,7 +7087,7 @@ impl<'a> Checker<'a> {
                     "declare the enclosing method's receiver as `mut self` to allow mutation"
                         .to_string()
                 } else {
-                    format!("declare it as `let mut {name}` to allow mutation")
+                    self.immutable_note(&name, decl, "mutation")
                 };
                 self.diagnostics
                     .last_mut()
@@ -7348,18 +7334,31 @@ impl<'a> Checker<'a> {
         let result_ty = fcx.icx.fresh();
         let mut hir_arms = Vec::new();
         // Normalized (pattern, has-guard, its pattern's span, the whole arm's
-        // span) per arm, for the exhaustiveness and reachability analysis
-        // after all arms are checked.
-        let mut arm_pats: Vec<(usefulness::Pat, bool, Span, Span)> = Vec::new();
+        // span, its or-patterns' alternatives' spans) per arm, for the
+        // exhaustiveness and reachability analysis after all arms are
+        // checked.
+        let mut arm_pats: Vec<ArmPat> = Vec::new();
+        // An arm whose pattern was an error stands in as `_`; reading it so
+        // would report the arms after it unreachable, a second diagnostic
+        // for one mistake, so the analysis is skipped.
+        let mut bad_pattern = false;
 
         for arm in arms {
             let guarded = arm.guard.is_some();
-            if guarded {
-                self.unsupported(arm.pattern.span, "match guards");
-            }
             fcx.scopes.push(FxHashMap::default());
+            let errors = self.error_count();
             let pattern = self.check_pattern(fcx, &arm.pattern, &scrut.ty);
+            bad_pattern |= self.error_count() > errors;
             let upat = to_useful_pat(&pattern);
+            let mut or_spans = Vec::new();
+            patterns::or_alternative_spans(&arm.pattern, &pattern, &mut or_spans);
+            // A guard sees the arm's bindings and runs after they bind
+            // (5.0 spec §4.3).
+            let guard = arm.guard.as_ref().map(|g| {
+                let guard = self.check_expr(fcx, g);
+                self.expect_ty(fcx, &guard, &Ty::Bool, "a match guard");
+                guard
+            });
             let body = self.check_expr(fcx, &arm.body);
             fcx.scopes.pop();
             // A diverging arm (`Never`) imposes no constraint on the result
@@ -7378,20 +7377,24 @@ impl<'a> Checker<'a> {
                     body.span,
                 );
             }
-            arm_pats.push((
-                upat,
+            arm_pats.push(ArmPat {
+                pat: upat,
                 guarded,
-                arm.pattern.span,
-                arm.pattern.span.merge(arm.body.span),
-            ));
+                pat_span: arm.pattern.span,
+                arm_span: arm.pattern.span.merge(arm.body.span),
+                or_spans,
+            });
             hir_arms.push(hir::Arm {
                 pattern,
+                guard,
                 body,
                 span: arm.pattern.span,
             });
         }
 
-        self.check_match_usefulness(fcx, &scrut, &arm_pats, span);
+        if !bad_pattern {
+            self.check_match_usefulness(fcx, &scrut, &arm_pats, span);
+        }
 
         // If every arm diverged the result was never constrained; it is then
         // itself `Never`.
@@ -7419,7 +7422,7 @@ impl<'a> Checker<'a> {
         &mut self,
         fcx: &mut FnCtx,
         scrut: &hir::Expr,
-        arm_pats: &[(usefulness::Pat, bool, Span, Span)],
+        arm_pats: &[ArmPat],
         span: Span,
     ) {
         let scrut_ty = fcx.icx.apply(&scrut.ty);
@@ -7454,7 +7457,14 @@ impl<'a> Checker<'a> {
         // (unguarded) arm does. A guarded arm's match is conditional, so it
         // neither is reported nor counts toward coverage.
         let mut prior: Vec<Vec<usefulness::Pat>> = Vec::new();
-        for (pat, guarded, arm_span, arm) in arm_pats {
+        for ArmPat {
+            pat,
+            guarded,
+            pat_span: arm_span,
+            arm_span: arm,
+            or_spans,
+        } in arm_pats
+        {
             if *guarded {
                 continue;
             }
@@ -7472,6 +7482,8 @@ impl<'a> Checker<'a> {
                     d = d.with_fix(fix);
                 }
                 self.diagnostics.push(d);
+            } else {
+                self.check_or_alternatives(&cx, &prior, pat, or_spans, &col);
             }
             prior.push(vec![pat.clone()]);
         }
@@ -7510,10 +7522,25 @@ impl<'a> Checker<'a> {
     fn render_witness(&self, pat: &usefulness::Pat) -> String {
         match pat {
             usefulness::Pat::Wild => "_".to_string(),
+            usefulness::Pat::Or(alts) => alts
+                .iter()
+                .map(|a| self.render_witness(a))
+                .collect::<Vec<_>>()
+                .join(" | "),
             usefulness::Pat::Ctor(ctor, args) => match ctor {
                 usefulness::Ctor::Bool(b) => b.to_string(),
                 usefulness::Ctor::Int(v) => v.to_string(),
                 usefulness::Ctor::Str(s) => format!("{s:?}"),
+                usefulness::Ctor::Char(c) => format!("{c:?}"),
+                usefulness::Ctor::Range(lo, hi) => format!("{lo}..={hi}"),
+                usefulness::Ctor::Tuple(_) => {
+                    let inner: Vec<String> = args.iter().map(|a| self.render_witness(a)).collect();
+                    if inner.len() == 1 {
+                        format!("({},)", inner[0])
+                    } else {
+                        format!("({})", inner.join(", "))
+                    }
+                }
                 usefulness::Ctor::Variant(sum, vi) => {
                     let name = self
                         .sums
@@ -7555,8 +7582,12 @@ impl<'a> Checker<'a> {
                     self.unify_pattern_ty(fcx, &Ty::String, scrut_ty, pattern.span);
                     hir::Pattern::LitStr(v.clone())
                 }
-                ast::Literal::Float(_) | ast::Literal::Char(_) => {
-                    self.unsupported(pattern.span, "float and char patterns");
+                ast::Literal::Char(c) => {
+                    self.unify_pattern_ty(fcx, &Ty::Char, scrut_ty, pattern.span);
+                    hir::Pattern::LitChar(*c)
+                }
+                ast::Literal::Float(_) => {
+                    self.unsupported(pattern.span, "`Float` literal patterns");
                     hir::Pattern::Wildcard
                 }
             },
@@ -7573,23 +7604,24 @@ impl<'a> Checker<'a> {
                     // The name is a known constructor, but of a different type:
                     // reject it rather than silently binding a catch-all (which
                     // would mask uncovered cases), mirroring the `Path` and
-                    // `TupleStruct` arms.
-                    self.error(
-                        "E0001",
-                        format!("`{}` is not a variant of the matched type", name.value),
-                        pattern.span,
+                    // `TupleStruct` arms. Inside a variant this is new in
+                    // 5.0b: `Some(None)` bound a local named `None` before
+                    // nested patterns, and now matches the variant.
+                    self.diagnostics.push(
+                        Diagnostic::error(
+                            "E0001",
+                            format!("`{}` is not a variant of the matched type", name.value),
+                        )
+                        .with_primary_label(pattern.span, "here")
+                        .with_note(
+                            "a name inside a pattern that names a variant matches that variant; \
+                             to bind a value, use a lower-case name"
+                                .to_string(),
+                        ),
                     );
                     return hir::Pattern::Wildcard;
                 }
-                let local = self.bind_local(
-                    fcx,
-                    name.value.clone(),
-                    scrut_ty.clone(),
-                    *is_mut,
-                    name.span,
-                    Binding::Other,
-                );
-                hir::Pattern::Bind(local)
+                self.bind_pattern_name(fcx, name, scrut_ty, *is_mut)
             }
             ast::Pattern::Path(path) if path.segments.len() == 1 => {
                 let name = &path.segments[0].value;
@@ -7731,13 +7763,24 @@ impl<'a> Checker<'a> {
                 }
                 self.variant_pattern(fcx, sum_id, vi, fields, scrut_ty, pattern.span)
             }
-            ast::Pattern::Path(_)
-            | ast::Pattern::Binding { .. }
-            | ast::Pattern::Record { .. }
-            | ast::Pattern::Tuple(_)
-            | ast::Pattern::Array(_)
-            | ast::Pattern::Or(_)
-            | ast::Pattern::Range { .. } => {
+            ast::Pattern::Tuple(elems) => self.tuple_pattern(fcx, elems, scrut_ty, pattern.span),
+            ast::Pattern::Or(alts) => self.or_pattern(fcx, alts, scrut_ty),
+            ast::Pattern::Range { lo, hi, inclusive } => {
+                self.range_pattern(fcx, lo, hi, *inclusive, scrut_ty, pattern.span)
+            }
+            ast::Pattern::Binding { .. } => {
+                self.unsupported(pattern.span, "`@` patterns");
+                hir::Pattern::Wildcard
+            }
+            ast::Pattern::Record { .. } => {
+                self.unsupported(pattern.span, "record patterns");
+                hir::Pattern::Wildcard
+            }
+            ast::Pattern::Array(_) => {
+                self.unsupported(pattern.span, "array patterns");
+                hir::Pattern::Wildcard
+            }
+            ast::Pattern::Path(_) => {
                 self.unsupported(pattern.span, "this pattern form");
                 hir::Pattern::Wildcard
             }
@@ -7795,32 +7838,19 @@ impl<'a> Checker<'a> {
             Ty::Sum { args, .. } => args,
             _ => Vec::new(),
         };
-        let mut binders = Vec::new();
-        for (field_pat, field_ty) in fields.iter().zip(v.fields.iter()) {
-            let bound_ty = field_ty.subst(&scrut_args);
-            match &field_pat.value {
-                ast::Pattern::Wildcard => binders.push(None),
-                ast::Pattern::Ident { is_mut, name } => {
-                    let local = self.bind_local(
-                        fcx,
-                        name.value.clone(),
-                        bound_ty,
-                        *is_mut,
-                        name.span,
-                        Binding::Other,
-                    );
-                    binders.push(Some(local));
-                }
-                _ => {
-                    self.unsupported(field_pat.span, "nested patterns inside variants");
-                    binders.push(None);
-                }
-            }
-        }
+        // Each field is a pattern, to any depth (5.0 spec §4.3).
+        let fields = fields
+            .iter()
+            .zip(v.fields.iter())
+            .map(|(field_pat, field_ty)| {
+                let bound_ty = field_ty.subst(&scrut_args);
+                self.check_pattern(fcx, field_pat, &bound_ty)
+            })
+            .collect();
         hir::Pattern::Variant {
             sum: sum_id,
             variant: variant as u32,
-            binders,
+            fields,
         }
     }
 
@@ -8019,6 +8049,14 @@ impl<'a> Checker<'a> {
         display_ty(&fcx.icx.apply(ty), self.defs)
     }
 
+    /// How many errors have been reported so far.
+    fn error_count(&self) -> usize {
+        self.diagnostics
+            .iter()
+            .filter(|d| d.severity == Severity::Error)
+            .count()
+    }
+
     fn error(&mut self, code: &str, message: impl Into<String>, span: Span) {
         self.diagnostics
             .push(Diagnostic::error(code, message).with_primary_label(span, "here"));
@@ -8220,12 +8258,19 @@ fn remap_locals(expr: &mut hir::Expr, map: &FxHashMap<u32, u32>) {
 fn remap_pattern(pat: &mut hir::Pattern, remap: &impl Fn(&mut LocalId)) {
     match pat {
         hir::Pattern::Bind(local) => remap(local),
-        hir::Pattern::Variant { binders, .. } => {
-            for b in binders.iter_mut().flatten() {
-                remap(b);
+        hir::Pattern::Variant { fields: ps, .. }
+        | hir::Pattern::Tuple(ps)
+        | hir::Pattern::Or(ps) => {
+            for p in ps.iter_mut() {
+                remap_pattern(p, remap);
             }
         }
-        _ => {}
+        hir::Pattern::Wildcard
+        | hir::Pattern::LitInt(_)
+        | hir::Pattern::LitBool(_)
+        | hir::Pattern::LitStr(_)
+        | hir::Pattern::LitChar(_)
+        | hir::Pattern::Range { .. } => {}
     }
 }
 
@@ -8294,7 +8339,10 @@ fn child_exprs(expr: &hir::Expr) -> Vec<&hir::Expr> {
         }
         K::Match { scrutinee, arms } => {
             out.push(scrutinee);
-            out.extend(arms.iter().map(|a| &a.body));
+            for a in arms {
+                out.extend(a.guard.as_ref());
+                out.push(&a.body);
+            }
         }
         K::Return(v) => {
             if let Some(v) = v {
@@ -8378,7 +8426,10 @@ fn child_exprs_mut(kind: &mut hir::ExprKind) -> Vec<&mut hir::Expr> {
         }
         K::Match { scrutinee, arms } => {
             out.push(scrutinee);
-            out.extend(arms.iter_mut().map(|a| &mut a.body));
+            for a in arms.iter_mut() {
+                out.extend(a.guard.as_mut());
+                out.push(&mut a.body);
+            }
         }
         K::Return(v) => {
             if let Some(v) = v {
@@ -8731,9 +8782,21 @@ pub(crate) fn builtin_signature(builtin: Builtin) -> (Vec<Ty>, Ty) {
     }
 }
 
+/// A `match` arm's pattern as the exhaustiveness and reachability analysis
+/// reads it.
+struct ArmPat {
+    pat: usefulness::Pat,
+    guarded: bool,
+    pat_span: Span,
+    arm_span: Span,
+    /// Each or-pattern's alternatives' spans, in pre-order.
+    or_spans: Vec<Vec<Span>>,
+}
+
 /// Normalize a checked HIR pattern into the usefulness-algorithm form.
-/// Bindings and wildcards are both "match anything"; variant payloads are
-/// irrefutable in Phase 1, so each contributes a wildcard sub-pattern.
+/// Bindings and wildcards are both "match anything"; the conversion recurses
+/// through variants and tuples, and keeps or-patterns, which the algorithm
+/// expands (5.0 spec §4.4).
 fn to_useful_pat(p: &hir::Pattern) -> usefulness::Pat {
     use usefulness::{Ctor, Pat};
     match p {
@@ -8741,14 +8804,21 @@ fn to_useful_pat(p: &hir::Pattern) -> usefulness::Pat {
         hir::Pattern::LitInt(v) => Pat::Ctor(Ctor::Int(*v), Vec::new()),
         hir::Pattern::LitBool(v) => Pat::Ctor(Ctor::Bool(*v), Vec::new()),
         hir::Pattern::LitStr(v) => Pat::Ctor(Ctor::Str(v.clone()), Vec::new()),
+        hir::Pattern::LitChar(c) => Pat::Ctor(Ctor::Char(*c), Vec::new()),
+        hir::Pattern::Range { lo, hi } => Pat::Ctor(Ctor::Range(*lo, *hi), Vec::new()),
         hir::Pattern::Variant {
             sum,
             variant,
-            binders,
+            fields,
         } => Pat::Ctor(
             Ctor::Variant(*sum, *variant),
-            vec![Pat::Wild; binders.len()],
+            fields.iter().map(to_useful_pat).collect(),
+        ),
+        hir::Pattern::Tuple(elems) => Pat::Ctor(
+            Ctor::Tuple(elems.len() as u32),
+            elems.iter().map(to_useful_pat).collect(),
         ),
+        hir::Pattern::Or(alts) => Pat::Or(alts.iter().map(to_useful_pat).collect()),
     }
 }
 
@@ -8984,6 +9054,9 @@ fn finalize_expr(expr: &mut hir::Expr, icx: &InferCtx, residual: &mut Vec<Span>)
         hir::ExprKind::Match { scrutinee, arms } => {
             finalize_expr(scrutinee, icx, residual);
             for arm in arms {
+                if let Some(guard) = &mut arm.guard {
+                    finalize_expr(guard, icx, residual);
+                }
                 finalize_expr(&mut arm.body, icx, residual);
             }
         }
@@ -11498,10 +11571,10 @@ mod tests {
             hir::Pattern::Variant {
                 sum,
                 variant,
-                binders,
+                fields,
             } => {
                 let s = r.module.sum(*sum).expect("the scrutinee's sum");
-                (s.variants[*variant as usize].name.clone(), binders.len())
+                (s.variants[*variant as usize].name.clone(), fields.len())
             }
             other => panic!("a variant pattern, not {other:?}"),
         };
@@ -14279,6 +14352,7 @@ mod tests {
             sources: None,
             bindings: FxHashMap::default(),
             qualified_callee: None,
+            pats: PatState::default(),
         };
         let mut fcx = FnCtx {
             icx: InferCtx::default(),
@@ -14356,6 +14430,7 @@ mod tests {
             sources: None,
             bindings: FxHashMap::default(),
             qualified_callee: None,
+            pats: PatState::default(),
         };
         let mut fcx = FnCtx {
             icx: InferCtx::default(),
diff --git a/crates/nova-typeck/src/check/fixes.rs b/crates/nova-typeck/src/check/fixes.rs
index e05dde379e4b478276c1111ec81b87fb6644e5c0..6ef6a7eba6811a8ec78b9e9e2be252fac7440780 100644
--- a/crates/nova-typeck/src/check/fixes.rs
+++ b/crates/nova-typeck/src/check/fixes.rs
@@ -18,7 +18,13 @@ pub(super) enum Binding {
     /// As a function's, a method's or a closure's parameter, `self`
     /// included.
     Param,
-    /// By a pattern or a `for` loop, where `mut` does not parse.
+    /// By a name inside a destructuring `let`'s pattern, where `mut` does
+    /// not parse but `let mut P` makes every binding mutable.
+    LetPattern,
+    /// By a name inside a `match` arm's or a `for` loop's pattern, where
+    /// `mut` does not parse.
+    Pattern,
+    /// By a `for` loop's plain name, where `mut` does not parse.
     Other,
 }
 
@@ -90,6 +96,28 @@ impl<'a> Checker<'a> {
         }
     }
 
+    /// The note of an E0060 at `name`, declared at `decl`, when no fix is
+    /// offered: how to make it mutable where it is bound, so as to allow
+    /// `what` ("assignment", "mutation").
+    pub(super) fn immutable_note(&self, name: &str, decl: Span, what: &str) -> String {
+        match self.bindings.get(&decl) {
+            Some(Binding::LetPattern) => format!(
+                "`{name}` is bound by a pattern, whose bindings are immutable: a destructuring \
+                 `let mut (…) = …` makes each of them mutable, or rebind it as \
+                 `let mut {name} = {name}` to allow {what}"
+            ),
+            Some(Binding::Pattern) => format!(
+                "`{name}` is bound by a pattern, whose bindings are immutable: rebind it as \
+                 `let mut {name} = {name}` to allow {what}"
+            ),
+            Some(Binding::Other) => format!(
+                "a `for` loop's variable is immutable: rebind it inside the loop's body as \
+                 `let mut {name} = {name}` to allow {what}"
+            ),
+            _ => format!("declare it as `let mut {name}` to allow {what}"),
+        }
+    }
+
     /// Add `fix`, if there is one, to the diagnostic just pushed, unless
     /// this project does not own the module being checked (spec §3.1; plan
     /// decision 19).
diff --git a/crates/nova-typeck/src/check/patterns.rs b/crates/nova-typeck/src/check/patterns.rs
new file mode 100644
index 0000000000000000000000000000000000000000..878e7bfb5999d3ff6bbc1715b081cbe7be2afe86
--- /dev/null
+++ b/crates/nova-typeck/src/check/patterns.rs
@@ -0,0 +1,643 @@
+//! 5.0b's patterns in the checker (spec
+//! `docs/superpowers/specs/2026-10-26-phase-5-0-the-language-and-std-a-compiler-needs-design.md`
+//! §4.3 to §4.5): tuple, or- and range patterns, the binding rules of an
+//! or-pattern (E0100), empty ranges (E0101), irrefutable destructuring in
+//! `let` and `for` (E0022, decided by usefulness), and the unreachable
+//! alternatives of an or-pattern (E0021).
+
+use std::ops::Range;
+
+use nova_ast as ast;
+use nova_diagnostics::{suggest, Diagnostic, Span, Spanned};
+use nova_hir as hir;
+use nova_hir::{LocalId, Ty};
+use nova_resolver::{DefKind, Res, ScopeEntry, Target};
+
+use super::fixes::Binding;
+use super::{to_useful_pat, Checker, FnCtx};
+use crate::usefulness;
+
+/// The checker's state while it checks one pattern.
+#[derive(Default)]
+pub(super) struct PatState {
+    /// The or-patterns being checked, innermost last.
+    frames: Vec<OrFrame>,
+    /// Every binding the pattern makes is mutable: set for `let mut P`,
+    /// since `mut` does not parse inside a pattern.
+    pub(super) mutable: bool,
+    /// The pattern is a destructuring `let`'s, whose bindings `let mut P`
+    /// can make mutable (E0060's note says so).
+    pub(super) in_let: bool,
+}
+
+/// One or-pattern being checked.
+#[derive(Default)]
+struct OrFrame {
+    /// The first alternative's bindings, once it is checked: each later
+    /// alternative binds these same locals.
+    first: Option<Vec<PatBind>>,
+    /// The bindings of the alternative being checked, nested or-patterns'
+    /// included.
+    current: Vec<PatBind>,
+}
+
+#[derive(Clone)]
+struct PatBind {
+    name: String,
+    local: LocalId,
+    ty: Ty,
+    span: Span,
+}
+
+/// How a `for` loop binds each element: a plain local, as before 5.0b, or
+/// an irrefutable pattern.
+pub(super) enum LoopBind {
+    Local(LocalId),
+    Pattern(hir::Pattern),
+}
+
+impl LoopBind {
+    /// The pattern that binds an element, for a `match` arm.
+    pub(super) fn into_pattern(self) -> hir::Pattern {
+        match self {
+            LoopBind::Local(l) => hir::Pattern::Bind(l),
+            LoopBind::Pattern(p) => p,
+        }
+    }
+
+    /// The statement that binds `init`: a `let` for a plain local, and for
+    /// a pattern a one-arm `match`, whose arm binds the pattern's locals for
+    /// the rest of the block (MIR's locals are the function's).
+    pub(super) fn bind(self, init: hir::Expr, span: Span) -> hir::Expr {
+        match self {
+            LoopBind::Local(local) => hir::Expr {
+                kind: hir::ExprKind::Let {
+                    local,
+                    init: Box::new(init),
+                },
+                ty: Ty::Unit,
+                span,
+            },
+            LoopBind::Pattern(pattern) => let_pattern(pattern, init, span),
+        }
+    }
+}
+
+/// `let P = init` for a destructuring `P`: a `match` with one arm, whose body
+/// is `()`.
+pub(super) fn let_pattern(pattern: hir::Pattern, init: hir::Expr, span: Span) -> hir::Expr {
+    hir::Expr {
+        kind: hir::ExprKind::Match {
+            scrutinee: Box::new(init),
+            arms: vec![hir::Arm {
+                pattern,
+                guard: None,
+                body: hir::Expr {
+                    kind: hir::ExprKind::Unit,
+                    ty: Ty::Unit,
+                    span,
+                },
+                span,
+            }],
+        },
+        ty: Ty::Unit,
+        span,
+    }
+}
+
+impl<'a> Checker<'a> {
+    /// Bind `name` in a pattern. Inside the second or a later alternative of
+    /// an or-pattern, the name is the first alternative's local, at the
+    /// same type (E0100 otherwise).
+    pub(super) fn bind_pattern_name(
+        &mut self,
+        fcx: &mut FnCtx,
+        name: &Spanned<String>,
+        ty: &Ty,
+        is_mut: bool,
+    ) -> hir::Pattern {
+        let first = self
+            .pats
+            .frames
+            .iter()
+            .rev()
+            .find_map(|f| f.first.as_ref())
+            .map(|first| first.iter().find(|b| b.name == name.value).cloned());
+        let bind = match first {
+            Some(Some(b)) => {
+                if !fcx.icx.unify(ty, &b.ty) {
+                    let (here, there) = (self.show(ty, fcx), self.show(&b.ty, fcx));
+                    self.diagnostics.push(
+                        Diagnostic::error(
+                            "E0100",
+                            format!("the alternatives bind `{}` at different types", name.value),
+                        )
+                        .with_primary_label(
+                            name.span,
+                            format!(
+                                "`{}` is `{here}` here, `{there}` in the first alternative",
+                                name.value
+                            ),
+                        ),
+                    );
+                }
+                self.note_use(name.span, Target::Local(b.span));
+                b
+            }
+            found => {
+                if found.is_some() {
+                    self.diagnostics.push(
+                        Diagnostic::error("E0100", "the alternatives bind different names")
+                            .with_primary_label(
+                                name.span,
+                                format!("`{}` is not bound in the first alternative", name.value),
+                            ),
+                    );
+                }
+                // A binding whose name starts with an upper-case letter is
+                // most often a variant misspelt or not in scope: as a binding
+                // it matches every value (the user's ruling of 2026-10-28).
+                if name.value.chars().next().is_some_and(char::is_uppercase) {
+                    self.warn_upper_case_binding(fcx, name, ty);
+                }
+                let is_mut = is_mut || self.pats.mutable;
+                let binding = if self.pats.in_let {
+                    Binding::LetPattern
+                } else {
+                    Binding::Pattern
+                };
+                let local = self.bind_local(
+                    fcx,
+                    name.value.clone(),
+                    ty.clone(),
+                    is_mut,
+                    name.span,
+                    binding,
+                );
+                PatBind {
+                    name: name.value.clone(),
+                    local,
+                    ty: ty.clone(),
+                    span: name.span,
+                }
+            }
+        };
+        let local = bind.local;
+        for frame in &mut self.pats.frames {
+            if !frame.current.iter().any(|b| b.name == bind.name) {
+                frame.current.push(bind.clone());
+            }
+        }
+        hir::Pattern::Bind(local)
+    }
+
+    /// E0104 at `name`, a pattern's binding that starts with an upper-case
+    /// letter (the user's ruling of 2026-10-28): a constant's name gets its
+    /// own note, and a name near a variant's "did you mean".
+    fn warn_upper_case_binding(&mut self, fcx: &FnCtx, name: &Spanned<String>, ty: &Ty) {
+        let is_const = matches!(
+            self.defs.resolve_value(self.cur_module, &name.value),
+            Some(Res::Def(id)) if matches!(self.defs.def(id).kind, DefKind::Const { .. })
+        );
+        let note = if is_const {
+            format!(
+                "a pattern cannot match a constant by name; use a guard: `n if n == {}`",
+                name.value
+            )
+        } else {
+            "a name that starts with an upper-case letter reads as a variant; if one was \
+             meant, check its spelling or import it; to bind a value, use a lower-case name"
+                .to_string()
+        };
+        let mut d = Diagnostic::warning(
+            "E0104",
+            format!(
+                "`{}` binds a value: no variant of that name is in scope",
+                name.value
+            ),
+        )
+        .with_primary_label(name.span, "this binding matches any value")
+        .with_note(note);
+        // The variants the name could mean: the matched sum's, and those in
+        // scope.
+        let mut variants: Vec<String> = Vec::new();
+        if let Some(sum) = super::fixes::scrutinee_sum(fcx, ty) {
+            if let DefKind::Sum { variants: vs, .. } = &self.defs.def(sum).kind {
+                variants.extend(vs.iter().map(|v| v.name.clone()));
+            }
+        }
+        for (n, entry) in self.defs.names_in_scope(self.cur_module) {
+            if matches!(entry, ScopeEntry::Value(Res::Variant(..))) {
+                variants.push(n);
+            }
+        }
+        let near = if is_const {
+            None
+        } else {
+            suggest::closest(&name.value, variants.iter().map(String::as_str))
+        };
+        if let Some(to) = near {
+            d = d.with_note(format!("did you mean `{to}`?"));
+        }
+        self.diagnostics.push(d);
+        if near.is_some() {
+            self.push_fix(suggest::change_to(
+                name,
+                variants.iter().map(String::as_str),
+            ));
+        }
+    }
+
+    /// `(p1, ..., pn)`, and `()` against the unit type.
+    pub(super) fn tuple_pattern(
+        &mut self,
+        fcx: &mut FnCtx,
+        elems: &[Spanned<ast::Pattern>],
+        scrut_ty: &Ty,
+        span: Span,
+    ) -> hir::Pattern {
+        if elems.is_empty() {
+            self.unify_pattern_ty(fcx, &Ty::Unit, scrut_ty, span);
+            return hir::Pattern::Wildcard;
+        }
+        let tys = match fcx.icx.apply(scrut_ty) {
+            Ty::Tuple(tys) if tys.len() == elems.len() => Some(tys),
+            Ty::Tuple(tys) => {
+                self.error(
+                    "E0010",
+                    format!(
+                        "this pattern has {} element{} but the matched value is a tuple of {}, `{}`",
+                        elems.len(),
+                        if elems.len() == 1 { "" } else { "s" },
+                        tys.len(),
+                        self.show(scrut_ty, fcx),
+                    ),
+                    span,
+                );
+                None
+            }
+            Ty::Error => Some(vec![Ty::Error; elems.len()]),
+            _ => {
+                let fresh: Vec<Ty> = elems.iter().map(|_| fcx.icx.fresh()).collect();
+                let tuple = Ty::Tuple(fresh.clone());
+                if fcx.icx.unify(scrut_ty, &tuple) {
+                    Some(fresh)
+                } else {
+                    self.error(
+                        "E0010",
+                        format!(
+                            "a tuple pattern cannot match a value of type `{}`",
+                            self.show(scrut_ty, fcx),
+                        ),
+                        span,
+                    );
+                    None
+                }
+            }
+        };
+        let Some(tys) = tys else {
+            // Bind the elements' names anyway, so their uses are not E0001
+            // too.
+            for e in elems {
+                self.check_pattern(fcx, e, &Ty::Error);
+            }
+            return hir::Pattern::Wildcard;
+        };
+        hir::Pattern::Tuple(
+            elems
+                .iter()
+                .zip(tys.iter())
+                .map(|(e, t)| self.check_pattern(fcx, e, t))
+                .collect(),
+        )
+    }
+
+    /// `p1 | p2 | ...`: each alternative binds the first one's names, at
+    /// their types (E0100).
+    pub(super) fn or_pattern(
+        &mut self,
+        fcx: &mut FnCtx,
+        alts: &[Spanned<ast::Pattern>],
+        scrut_ty: &Ty,
+    ) -> hir::Pattern {
+        self.pats.frames.push(OrFrame::default());
+        let mut out = Vec::new();
+        for (i, alt) in alts.iter().enumerate() {
+            self.pats.frames.last_mut().expect("pushed").current.clear();
+            out.push(self.check_pattern(fcx, alt, scrut_ty));
+            let frame = self.pats.frames.last_mut().expect("pushed");
+            match &frame.first {
+                None if i == 0 => frame.first = Some(frame.current.clone()),
+                None => {}
+                Some(first) => {
+                    let missing: Vec<String> = first
+                        .iter()
+                        .filter(|b| !frame.current.iter().any(|c| c.name == b.name))
+                        .map(|b| b.name.clone())
+                        .collect();
+                    for name in missing {
+                        self.diagnostics.push(
+                            Diagnostic::error("E0100", "the alternatives bind different names")
+                                .with_primary_label(
+                                    alt.span,
+                                    format!("`{name}` is missing from this alternative"),
+                                )
+                                .with_note(format!(
+                                    "every alternative of an or-pattern binds the same names; \
+                                     the first one binds `{name}`"
+                                )),
+                        );
+                    }
+                }
+            }
+        }
+        self.pats.frames.pop();
+        hir::Pattern::Or(out)
+    }
+
+    /// `lo..hi` and `lo..=hi`, over `Int`, and `lo..=hi` over `Char`, as
+    /// the parser reads them. An empty range is E0101.
+    pub(super) fn range_pattern(
+        &mut self,
+        fcx: &mut FnCtx,
+        lo: &Spanned<ast::Pattern>,
+        hi: &Spanned<ast::Pattern>,
+        inclusive: bool,
+        scrut_ty: &Ty,
+        span: Span,
+    ) -> hir::Pattern {
+        use ast::{Literal, Pattern as P};
+        let (lo_v, hi_v, ty, shown) = match (&lo.value, &hi.value) {
+            (P::Lit(Literal::Int(a)), P::Lit(Literal::Int(b))) => {
+                (*a, *b, Ty::Int, (a.to_string(), b.to_string()))
+            }
+            (P::Lit(Literal::Char(a)), P::Lit(Literal::Char(b))) => (
+                *a as i64,
+                *b as i64,
+                Ty::Char,
+                (format!("{a:?}"), format!("{b:?}")),
+            ),
+            _ => {
+                self.error(
+                    "E0010",
+                    "a range pattern's ends must be two `Int` or two `Char` literals",
+                    span,
+                );
+                return hir::Pattern::Wildcard;
+            }
+        };
+        self.unify_pattern_ty(fcx, &ty, scrut_ty, span);
+        let empty = if inclusive { lo_v > hi_v } else { lo_v >= hi_v };
+        if empty {
+            let (a, b) = shown;
+            let why = if lo_v == hi_v {
+                format!("`{a}..{b}` stops before its start")
+            } else {
+                format!("`{a}` is above `{b}`")
+            };
+            self.diagnostics.push(
+                Diagnostic::error("E0101", "this range matches nothing")
+                    .with_primary_label(span, why),
+            );
+            // Kept as written, not as `_`, so the arms after it are not
+            // reported unreachable as well; the program does not compile.
+        }
+        hir::Pattern::Range {
+            lo: lo_v,
+            hi: if inclusive { hi_v } else { hi_v - 1 },
+        }
+    }
+
+    /// A destructuring pattern in a `let` or a `for` must match every value
+    /// of `ty`: E0022, with the values it misses, when usefulness finds one.
+    pub(super) fn require_irrefutable(
+        &mut self,
+        fcx: &mut FnCtx,
+        pat: &hir::Pattern,
+        ty: &Ty,
+        span: Span,
+        what: &str,
+    ) {
+        let ty = fcx.icx.apply(ty);
+        if matches!(ty, Ty::Var(_) | Ty::Param(_) | Ty::Error | Ty::Never) {
+            return;
+        }
+        let sums = self.sums.clone();
+        let cx = usefulness::MatchCx::new(&sums).naming_values();
+        let row = vec![to_useful_pat(pat)];
+        let witnesses = cx.usefulness(&[row], &[usefulness::Pat::Wild], &[ty]);
+        if witnesses.is_empty() {
+            return;
+        }
+        let mut rendered: Vec<String> = witnesses
+            .iter()
+            .filter_map(|w| w.first())
+            .map(|p| format!("`{}`", self.render_witness(p)))
+            .collect();
+        rendered.dedup();
+        self.diagnostics.push(
+            Diagnostic::error("E0022", format!("refutable pattern in {what}"))
+                .with_primary_label(
+                    span,
+                    format!(
+                        "{} {} not matched",
+                        rendered.join(", "),
+                        if rendered.len() == 1 { "is" } else { "are" }
+                    ),
+                )
+                .with_note(
+                    "`let` and `for` take a pattern every value matches: names, `_`, tuples \
+                     of these, and a variant of a sum that has only one; use `match` for the rest"
+                        .to_string(),
+                ),
+        );
+    }
+
+    /// How a `for` loop binds each element of type `ty`: a name or `_` as
+    /// before, or an irrefutable pattern (5.0 spec §4.3, §12 item 13).
+    ///
+    /// A `for` loop variable is immutable, so assigning it is `E0060`. `mut`
+    /// does not parse in this position (`for mut i in …` is P0001), so the
+    /// `Ident` arm's `is_mut` is always `false`.
+    pub(super) fn for_loop_binding(
+        &mut self,
+        fcx: &mut FnCtx,
+        pattern: &Spanned<ast::Pattern>,
+        ty: &Ty,
+    ) -> LoopBind {
+        match &pattern.value {
+            ast::Pattern::Ident { name, .. } => LoopBind::Local(self.bind_local(
+                fcx,
+                name.value.clone(),
+                ty.clone(),
+                false,
+                name.span,
+                Binding::Other,
+            )),
+            ast::Pattern::Wildcard => LoopBind::Local(self.bind_local(
+                fcx,
+                "_".to_string(),
+                ty.clone(),
+                false,
+                pattern.span,
+                Binding::Other,
+            )),
+            _ => {
+                let errors = self.error_count();
+                let pat = self.check_pattern(fcx, pattern, ty);
+                if self.error_count() == errors {
+                    self.require_irrefutable(fcx, &pat, ty, pattern.span, "a `for` loop");
+                }
+                LoopBind::Pattern(pat)
+            }
+        }
+    }
+
+    /// Report each alternative of an or-pattern in a reachable, unguarded
+    /// arm that no value can reach: every value it matches is matched by an
+    /// earlier arm, or by an earlier alternative of the same or-pattern. An
+    /// alternative inside one already reported is not reported again.
+    pub(super) fn check_or_alternatives(
+        &mut self,
+        cx: &usefulness::MatchCx,
+        prior: &[Vec<usefulness::Pat>],
+        pat: &usefulness::Pat,
+        spans: &[Vec<Span>],
+        col: &[Ty],
+    ) {
+        let tree = or_tree(pat);
+        let mut dead: Vec<(usize, usize)> = Vec::new();
+        for (node, (parent, alts)) in tree.iter().enumerate() {
+            if ancestors(&tree, *parent).any(|a| dead.contains(&a)) {
+                continue;
+            }
+            for k in 0..*alts {
+                let q = restrict_or(pat, node, k..k + 1);
+                let mut matrix = prior.to_vec();
+                if k > 0 {
+                    matrix.push(vec![restrict_or(pat, node, 0..k)]);
+                }
+                if !cx
+                    .usefulness(&matrix, std::slice::from_ref(&q), col)
+                    .is_empty()
+                {
+                    continue;
+                }
+                dead.push((node, k));
+                let Some(span) = spans.get(node).and_then(|s| s.get(k)) else {
+                    continue;
+                };
+                self.diagnostics.push(
+                    Diagnostic::warning("E0021", "unreachable alternative")
+                        .with_primary_label(*span, "this alternative is never reached")
+                        .with_note(
+                            "an earlier arm, or an earlier alternative of this or-pattern, \
+                             already matches every value this one would"
+                                .to_string(),
+                        ),
+                );
+            }
+        }
+    }
+}
+
+/// The spans of each or-pattern's alternatives in `hir`, in the pre-order
+/// [`or_tree`] numbers them in. The checked pattern follows the source's
+/// shape wherever it has an or-pattern inside it; where it does not (a
+/// pattern replaced by `_` after an error), it has none to number.
+pub(super) fn or_alternative_spans(
+    ast: &Spanned<ast::Pattern>,
+    hir: &hir::Pattern,
+    out: &mut Vec<Vec<Span>>,
+) {
+    match (&ast.value, hir) {
+        (ast::Pattern::Or(a), hir::Pattern::Or(h)) if a.len() == h.len() => {
+            out.push(a.iter().map(|x| x.span).collect());
+            for (x, y) in a.iter().zip(h) {
+                or_alternative_spans(x, y, out);
+            }
+        }
+        (ast::Pattern::TupleStruct { fields: a, .. }, hir::Pattern::Variant { fields: h, .. })
+        | (ast::Pattern::Tuple(a), hir::Pattern::Tuple(h))
+            if a.len() == h.len() =>
+        {
+            for (x, y) in a.iter().zip(h) {
+                or_alternative_spans(x, y, out);
+            }
+        }
+        _ => {
+            // Keep the numbering: one entry, with no spans, per or-pattern.
+            let count = or_tree(&to_useful_pat(hir)).len();
+            out.extend(std::iter::repeat(Vec::new()).take(count));
+        }
+    }
+}
+
+/// Each or-pattern of `pat`, in pre-order: the or-pattern and alternative it
+/// sits in, if any, and its number of alternatives.
+fn or_tree(pat: &usefulness::Pat) -> Vec<(Option<(usize, usize)>, usize)> {
+    fn walk(
+        p: &usefulness::Pat,
+        parent: Option<(usize, usize)>,
+        out: &mut Vec<(Option<(usize, usize)>, usize)>,
+    ) {
+        match p {
+            usefulness::Pat::Wild => {}
+            usefulness::Pat::Ctor(_, args) => {
+                for a in args {
+                    walk(a, parent, out);
+                }
+            }
+            usefulness::Pat::Or(alts) => {
+                let me = out.len();
+                out.push((parent, alts.len()));
+                for (k, a) in alts.iter().enumerate() {
+                    walk(a, Some((me, k)), out);
+                }
+            }
+        }
+    }
+    let mut out = Vec::new();
+    walk(pat, None, &mut out);
+    out
+}
+
+/// The or-pattern and alternative `start` names, and those it sits in.
+fn ancestors(
+    tree: &[(Option<(usize, usize)>, usize)],
+    start: Option<(usize, usize)>,
+) -> impl Iterator<Item = (usize, usize)> + '_ {
+    std::iter::successors(start, move |(node, _)| tree[*node].0)
+}
+
+/// `pat` with its `target`-th or-pattern, counted as [`or_tree`] counts,
+/// cut to the alternatives in `keep`.
+fn restrict_or(pat: &usefulness::Pat, target: usize, keep: Range<usize>) -> usefulness::Pat {
+    fn go(
+        p: &usefulness::Pat,
+        target: usize,
+        keep: &Range<usize>,
+        n: &mut usize,
+    ) -> usefulness::Pat {
+        match p {
+            usefulness::Pat::Wild => usefulness::Pat::Wild,
+            usefulness::Pat::Ctor(c, args) => usefulness::Pat::Ctor(
+                c.clone(),
+                args.iter().map(|a| go(a, target, keep, n)).collect(),
+            ),
+            usefulness::Pat::Or(alts) => {
+                let me = *n;
+                *n += 1;
+                let mut kept = Vec::new();
+                for (k, a) in alts.iter().enumerate() {
+                    let a = go(a, target, keep, n);
+                    if me != target || keep.contains(&k) {
+                        kept.push(a);
+                    }
+                }
+                usefulness::Pat::Or(kept)
+            }
+        }
+    }
+    go(pat, target, &keep, &mut 0)
+}
diff --git a/crates/nova-typeck/src/usefulness.rs b/crates/nova-typeck/src/usefulness.rs
index b4c82efd6076758c8783ce53919bfd652e4a1749..59192bef63f7b12bfa5f8ec9b3de1420ebcca522 100644
--- a/crates/nova-typeck/src/usefulness.rs
+++ b/crates/nova-typeck/src/usefulness.rs
@@ -13,11 +13,23 @@
 //! The search reconstructs **witness** patterns for the values a non-exhaustive
 //! match fails to cover (e.g. `Some(_)`, `false`).
 //!
-//! The algorithm is written for arbitrarily nested patterns; Nova's Phase 1
-//! pattern language only ever feeds it shallow rows (variant payloads are
-//! irrefutable binds/wildcards), but the recursion, specialization and default
-//! matrices are the general ones, so the checker stays correct as the pattern
-//! language grows.
+//! The algorithm is written for arbitrarily nested patterns, and since 5.0b
+//! (spec `docs/superpowers/specs/2026-10-26-phase-5-0-the-language-and-std-a-compiler-needs-design.md`
+//! §4.4) the checker feeds it nested ones: variants and tuples to any depth,
+//! or-patterns, which it expands into their alternatives, and `Int`, `Char`
+//! and `String` literals and ranges.
+//!
+//! **Ranges are not split.** A literal and a range are each a constructor of
+//! their own, equal only to the same literal or the same range, of a type
+//! with too many values to list. So a `match` on them needs a `_` or a name to
+//! be exhaustive, and a value an earlier range covers is still useful to a
+//! later arm unless the same range or literal came first. Treating
+//! overlapping constructors as disjoint only ever drops rows from a
+//! specialization, which makes a row *more* useful: an arm is never reported
+//! unreachable when it is not, and a match is never accepted when it is not
+//! exhaustive. What it gives up is reporting an arm that earlier ranges
+//! cover, and accepting a match whose ranges cover a whole type without a
+//! `_` (5.0 spec §4.4, §12 item 14).
 
 use nova_hir::{SumType, Ty};
 use nova_resolver::DefId;
@@ -30,6 +42,11 @@ pub enum Ctor {
     Bool(bool),
     Int(i64),
     Str(String),
+    Char(char),
+    /// An `Int` range, or a `Char` range by code point, both ends included.
+    Range(i64, i64),
+    /// The one constructor of a tuple type of this arity.
+    Tuple(u32),
 }
 
 /// A pattern in the checker's normalized form: either a wildcard (which also
@@ -39,12 +56,17 @@ pub enum Ctor {
 pub enum Pat {
     Wild,
     Ctor(Ctor, Vec<Pat>),
+    /// An or-pattern: it matches what any alternative matches.
+    Or(Vec<Pat>),
 }
 
 /// Context for the algorithm: the module's sum-type declarations, used to
 /// enumerate a type's constructor signature and their arities.
 pub struct MatchCx<'a> {
     sums: &'a [SumType],
+    /// Whether a witness names a value a column's literals and ranges miss,
+    /// as E0022 shows it, rather than `_`, as E0020 does.
+    name_values: bool,
 }
 
 /// One row of witness patterns (aligned with the current columns).
@@ -52,7 +74,19 @@ type Witness = Vec<Pat>;
 
 impl<'a> MatchCx<'a> {
     pub fn new(sums: &'a [SumType]) -> Self {
-        Self { sums }
+        Self {
+            sums,
+            name_values: false,
+        }
+    }
+
+    /// This context, its witnesses naming a value of an `Int`, `Char` or
+    /// `String` column that the column's literals and ranges miss.
+    pub fn naming_values(self) -> Self {
+        Self {
+            name_values: true,
+            ..self
+        }
     }
 
     /// Witnesses that `q` matches but no row of `matrix` does. An empty result
@@ -68,8 +102,27 @@ impl<'a> MatchCx<'a> {
                 Vec::new()
             };
         }
+        // A row whose head is an or-pattern stands for one row per
+        // alternative.
+        let expanded;
+        let matrix = if matrix.iter().any(|r| matches!(r[0], Pat::Or(_))) {
+            expanded = expand_or_heads(matrix);
+            &expanded[..]
+        } else {
+            matrix
+        };
 
         match &q[0] {
+            // `q` is useful when one of its alternatives is.
+            Pat::Or(alts) => {
+                let mut out = Vec::new();
+                for a in alts {
+                    let mut row = vec![a.clone()];
+                    row.extend_from_slice(&q[1..]);
+                    out.extend(self.usefulness(matrix, &row, col_tys));
+                }
+                out
+            }
             Pat::Ctor(c, args) => {
                 let spec = self.specialize(matrix, c);
                 let spec_tys = self.specialized_tys(c, col_tys);
@@ -82,6 +135,8 @@ impl<'a> MatchCx<'a> {
             }
             Pat::Wild => {
                 let present = column_ctors(matrix);
+                let col0 = tuple_ty(&col_tys[0], &present);
+                let col_tys = &with_head(col0, col_tys)[..];
                 if self.is_complete(&col_tys[0], &present) {
                     // Every constructor of the type is present: `q` is useful
                     // only if it is useful under some specific constructor.
@@ -113,6 +168,23 @@ impl<'a> MatchCx<'a> {
                             out.push(row);
                         }
                     }
+                    // Where E0022 asks, also the values under each present
+                    // constructor, so `for (1, true) in p` names `(1, false)`
+                    // beside `(0, _)`.
+                    if self.name_values {
+                        for c in &present {
+                            let arity = self.arity(c);
+                            let spec = self.specialize(matrix, c);
+                            let spec_tys = self.specialized_tys(c, col_tys);
+                            let mut sq = vec![Pat::Wild; arity];
+                            sq.extend_from_slice(&q[1..]);
+                            out.extend(
+                                self.usefulness(&spec, &sq, &spec_tys)
+                                    .into_iter()
+                                    .map(|w| reassemble(c, w, arity)),
+                            );
+                        }
+                    }
                     out
                 }
             }
@@ -138,6 +210,17 @@ impl<'a> MatchCx<'a> {
                     nr.extend_from_slice(&row[1..]);
                     out.push(nr);
                 }
+                Pat::Or(alts) => {
+                    let rows: Vec<Vec<Pat>> = alts
+                        .iter()
+                        .map(|a| {
+                            let mut r = vec![a.clone()];
+                            r.extend_from_slice(&row[1..]);
+                            r
+                        })
+                        .collect();
+                    out.extend(self.specialize(&rows, c));
+                }
             }
         }
         out
@@ -164,7 +247,13 @@ impl<'a> MatchCx<'a> {
                     .map(|v| v.fields.iter().map(|f| f.subst(&args)).collect())
                     .unwrap_or_default()
             }
-            Ctor::Bool(_) | Ctor::Int(_) | Ctor::Str(_) => Vec::new(),
+            Ctor::Tuple(n) => match ty0 {
+                Ty::Tuple(elems) if elems.len() == *n as usize => elems.clone(),
+                _ => vec![Ty::Error; *n as usize],
+            },
+            Ctor::Bool(_) | Ctor::Int(_) | Ctor::Str(_) | Ctor::Char(_) | Ctor::Range(..) => {
+                Vec::new()
+            }
         }
     }
 
@@ -174,7 +263,8 @@ impl<'a> MatchCx<'a> {
                 .variant(*sum_id, *vi)
                 .map(|v| v.fields.len())
                 .unwrap_or(0),
-            Ctor::Bool(_) | Ctor::Int(_) | Ctor::Str(_) => 0,
+            Ctor::Tuple(n) => *n as usize,
+            Ctor::Bool(_) | Ctor::Int(_) | Ctor::Str(_) | Ctor::Char(_) | Ctor::Range(..) => 0,
         }
     }
 
@@ -193,6 +283,7 @@ impl<'a> MatchCx<'a> {
                         .collect()
                 })
                 .unwrap_or_default(),
+            Ty::Tuple(elems) => vec![Ctor::Tuple(elems.len() as u32)],
             _ => Vec::new(),
         }
     }
@@ -209,6 +300,15 @@ impl<'a> MatchCx<'a> {
     fn missing_heads(&self, ty: &Ty, present: &[Ctor]) -> Vec<Pat> {
         let sig = self.signature(ty);
         if sig.is_empty() {
+            // A type with too many values to list: where asked, name one the
+            // column's literals and ranges miss, so the witness says which
+            // value is not matched, `(_, 0)` rather than `(_, _)`.
+            let named = missing_value(ty, present).filter(|_| self.name_values);
+            return vec![named.map_or(Pat::Wild, |c| Pat::Ctor(c, Vec::new()))];
+        }
+        if present.is_empty() {
+            // No row names a constructor here: `_` says it all, where
+            // listing each would repeat the rest of the witness per value.
             return vec![Pat::Wild];
         }
         let missing: Vec<Pat> = sig
@@ -231,6 +331,106 @@ impl<'a> MatchCx<'a> {
     }
 }
 
+/// A value of `ty`, an `Int`, a `Char` or a `String`, that no literal or
+/// range of `present` matches: the first from `0`, `'a'` or `""` on. `None`
+/// when `present` holds none of the type's constructors (`_` says it all)
+/// or a short search finds no such value.
+fn missing_value(ty: &Ty, present: &[Ctor]) -> Option<Ctor> {
+    let covers = |v: i64| {
+        present.iter().any(|c| match c {
+            Ctor::Int(n) => *n == v,
+            Ctor::Char(ch) => *ch as i64 == v,
+            Ctor::Range(lo, hi) => *lo <= v && v <= *hi,
+            _ => false,
+        })
+    };
+    let first_free = |start: i64| {
+        let mut v = start;
+        for _ in 0..=present.len() {
+            if !covers(v) {
+                return Some(v);
+            }
+            let past = present.iter().find_map(|c| match c {
+                Ctor::Range(lo, hi) if *lo <= v && v <= *hi => Some(*hi),
+                _ => None,
+            });
+            v = past.unwrap_or(v).checked_add(1)?;
+        }
+        None
+    };
+    match ty {
+        Ty::Int
+            if present
+                .iter()
+                .any(|c| matches!(c, Ctor::Int(_) | Ctor::Range(..))) =>
+        {
+            first_free(0).map(Ctor::Int)
+        }
+        Ty::Char
+            if present
+                .iter()
+                .any(|c| matches!(c, Ctor::Char(_) | Ctor::Range(..))) =>
+        {
+            first_free('a' as i64)
+                .and_then(|v| u32::try_from(v).ok())
+                .and_then(char::from_u32)
+                .map(Ctor::Char)
+        }
+        Ty::String if present.iter().any(|c| matches!(c, Ctor::Str(_))) => {
+            std::iter::once(String::new())
+                .chain(('a'..='z').map(String::from))
+                .find(|s| !present.contains(&Ctor::Str(s.clone())))
+                .map(Ctor::Str)
+        }
+        _ => None,
+    }
+}
+
+/// `matrix` with each row whose head is an or-pattern replaced by one row
+/// per alternative, until no head is one.
+fn expand_or_heads(matrix: &[Vec<Pat>]) -> Vec<Vec<Pat>> {
+    let mut out = Vec::new();
+    for row in matrix {
+        match &row[0] {
+            Pat::Or(alts) => {
+                let rows: Vec<Vec<Pat>> = alts
+                    .iter()
+                    .map(|a| {
+                        let mut r = vec![a.clone()];
+                        r.extend_from_slice(&row[1..]);
+                        r
+                    })
+                    .collect();
+                out.extend(expand_or_heads(&rows));
+            }
+            _ => out.push(row.clone()),
+        }
+    }
+    out
+}
+
+/// The first column's type, as a tuple of the arity the column's rows use
+/// when inference has not yet made it one: a tuple has one constructor
+/// whatever its elements' types.
+fn tuple_ty(ty: &Ty, present: &[Ctor]) -> Option<Ty> {
+    if matches!(ty, Ty::Tuple(_)) {
+        return None;
+    }
+    present.iter().find_map(|c| match c {
+        Ctor::Tuple(n) => Some(Ty::Tuple(vec![Ty::Error; *n as usize])),
+        _ => None,
+    })
+}
+
+/// `col_tys` with its first type replaced by `head`, if there is one.
+fn with_head(head: Option<Ty>, col_tys: &[Ty]) -> Vec<Ty> {
+    let mut tys = col_tys.to_vec();
+    if let Some(h) = head {
+        tys[0] = h;
+    }
+    tys
+}
+
 /// Distinct constructors appearing in the matrix's first column.
 fn column_ctors(matrix: &[Vec<Pat>]) -> Vec<Ctor> {
     let mut out: Vec<Ctor> = Vec::new();
````

### Task 4: `for x in arr`

Spec §4.5; decisions 7, 27, 34, 36 and 37.

**Files:**
- Modify:
  - `crates/nova-typeck/src/check/patterns.rs` (`check_for_array`)
  - `crates/nova-typeck/src/check.rs` (`check_for_iterator` hands an array
    to it; the message names arrays)
  - `tests/runtime/fs_read_dir.nova` (a comment)
- Test:
  - `crates/nova-typeck/src/check/pattern_tests.rs` (six tests)
  - `crates/nova-lsp/src/hover.rs`, `completion.rs` (one test each)

- [ ] **Step 1: The tests.** `patch_block t4-tests`. The patch holds:
  - `pattern_tests.rs`: `for` destructuring over an array of tuples, and
    plain and `_` loops over arrays; a refutable `for` pattern E0022,
    "refutable pattern in a `for` loop", naming each missed value once,
    `(0, _)` and `(1, false)` for `for (1, true) in ps` (decision 44); E0104
    for `for (A, c) in xs` (decision 42); no E0022 after a pattern error
    (decision 43); a loop variable over an array immutable (E0060), its
    note, and a `for` pattern's, saying to rebind it (decision 24); and the
    desugar, an index loop that reads the length once and indexes once. The
    module doc names `for` over an array from here on.
  - The language server: a `for` pattern's bindings, and a binding two
    variants deep in an or-pattern, hover with their types where they are
    bound and used, and complete in the body, each once, with a member after
    one.
- [ ] **Step 2: Run, and see the tests fail.** First the build, which ends
  with `Finished`.
  - `T check::pattern_tests::for_ check::pattern_tests::a_refutable_for check::pattern_tests::a_for_ check::pattern_tests::an_upper_case_binding_in_a_for bindings_in_a_for_pattern`.
    Expected: `nova-lsp`'s
    `0 passed; 2 failed; 0 ignored; 0 measured; 29 filtered out`,
    `bindings_in_a_for_pattern_and_an_or_pattern_complete` and
    `bindings_in_a_for_pattern_and_a_nested_or_pattern_hover`, the loop's
    bindings unknown; and `nova-typeck`'s
    `1 passed; 5 failed; 0 ignored; 0 measured; 529 filtered out`, each
    failure with E0900, "`for` loops over anything but an integer range
    (`a..b`) or a value implementing `Iterator` (try `.iter()`) are not
    supported yet", where it expects its own outcome; every other library's
    `ok`, with none run. The one that passes,
    `a_for_pattern_with_an_error_is_not_also_e0022`, asks for an error and
    no E0022, which the E0900 gives (decision 38).
- [ ] **Step 3: The code.** `patch_block t4-code`:
  - **`patterns.rs`:** `check_for_array`, the desugar of decision 27, into
    unscoped `__arr`, `__n` and `__i`, binding each element through
    `for_loop_binding`'s `LoopBind`.
  - **`check.rs`:** `check_for_iterator` hands a `Ty::Array` to it before
    looking for an `Iterator`, and its E0900 names arrays.
  - **`fs_read_dir.nova`:** its comment says `for e in entries` works now
    (decision 39).
- [ ] **Step 4: Run.** First the build, which ends with `Finished`.
  - `T check::pattern_tests::for_ check::pattern_tests::a_refutable_for check::pattern_tests::a_for_ check::pattern_tests::an_upper_case_binding_in_a_for bindings_in_a_for_pattern`.
    Expected: every `test result` `ok`, `nova-lsp`'s `2 passed` and
    `nova-typeck`'s `6 passed`.
  - `S`, as in Task 1 (about 4 min 50 s). Expected: `2679 1 10`: Task 3's
    figures and Task 4's 8 tests, the one failure the IPv6 test.
  - `R fs_read_dir`. Expected: `2 passed; 0 failed`.
- [ ] **Step 5: The checks,** as the conventions list them.
- [ ] **Step 6: Commit** `typeck: for over an array`.

**Patch `t4-tests`:**

<!-- patch t4-tests -->

````diff
diff --git a/crates/nova-lsp/src/completion.rs b/crates/nova-lsp/src/completion.rs
index 548fa31f04e2c60d857470ba947e57a46541b72d..ddee7cbd0f661c1b1208db6124b15f68715b3dba 100644
--- a/crates/nova-lsp/src/completion.rs
+++ b/crates/nova-lsp/src/completion.rs
@@ -213,6 +213,45 @@ mod tests {
         assert!(members.contains(&"text".to_string()), "{members:?}");
     }
 
+    /// A `for` pattern's bindings, and an or-pattern's, are offered in the
+    /// body, each once, and a member after one (5.0 spec §4.3, §4.5).
+    #[test]
+    fn bindings_in_a_for_pattern_and_an_or_pattern_complete() {
+        let text = "record Tok { text: String }\n\n\
+                    fn f(o: Result<(Int, Tok), (Int, Tok)>, arr: [(Int, Tok)]) -> Int {\n    \
+                    for (left, right) in arr {\n        \n        right.\n    }\n    \
+                    match o {\n        Ok((num, tok)) | Err((num, tok)) => {\n            \n            \
+                    tok.\n            num\n        }\n    }\n}\n\nfn main() {}\n";
+        let path = std::env::temp_dir()
+            .join("nova-lsp-complete-for-pattern")
+            .join("main.nova");
+        let overlay = Overlay::default().with(&path, text.to_string());
+        let labels = |offset: usize| -> Vec<String> {
+            complete(&path, text, offset as u32, &overlay)
+                .into_iter()
+                .map(|i| i.label)
+                .collect()
+        };
+        let count = |names: &[String], want: &str| names.iter().filter(|n| *n == want).count();
+        let body = text.find("arr {\n        ").unwrap() + "arr {\n        ".len();
+        let names = labels(body);
+        for want in ["left", "right", "arr", "o"] {
+            assert_eq!(count(&names, want), 1, "{want}: {names:?}");
+        }
+        let arm = text.find("=> {\n            ").unwrap() + "=> {\n            ".len();
+        let names = labels(arm);
+        for want in ["num", "tok", "arr", "o"] {
+            assert_eq!(count(&names, want), 1, "{want}: {names:?}");
+        }
+        for at in [
+            text.find("right.\n").unwrap() + 6,
+            text.find("tok.\n").unwrap() + 4,
+        ] {
+            let members = labels(at);
+            assert!(members.contains(&"text".to_string()), "{members:?}");
+        }
+    }
+
     #[test]
     fn a_signature_stops_at_its_body_outside_parentheses() {
         assert_eq!(
diff --git a/crates/nova-lsp/src/hover.rs b/crates/nova-lsp/src/hover.rs
index b73eb29ffbb3157059a3c98e531bf754b21b115c..226dd61c2f341d1d9459f0bf6f1a66086bbffda7 100644
--- a/crates/nova-lsp/src/hover.rs
+++ b/crates/nova-lsp/src/hover.rs
@@ -341,6 +341,43 @@ mod tests {
         assert!(v.contains("v: Int"), "{v}");
     }
 
+    /// A `for` pattern's bindings, and a binding two variants deep in an
+    /// or-pattern, hover with their types where they are bound and where
+    /// they are used (5.0 spec §4.3, §4.5).
+    #[test]
+    fn bindings_in_a_for_pattern_and_a_nested_or_pattern_hover() {
+        let text = "fn f(o: Option<Result<Int, String>>, arr: [(Int, String)]) -> Int {\n    \
+                    let mut n = 0\n    \
+                    for (a, b) in arr {\n        n = n + a + b.len()\n    }\n    \
+                    match o {\n        Some(Ok(x)) | Some(Err(_)) if n > 0 => x\n        \
+                    Some(Ok(y)) => y\n        _ => 0\n    }\n}\n\nfn main() {}\n";
+        let path = std::env::temp_dir()
+            .join("nova-lsp-hover-for-pattern")
+            .join("main.nova");
+        let overlay = crate::workspace::Overlay::default().with(&path, text.to_string());
+        let options = crate::analysis::options(None, true);
+        let answer = crate::analysis::answering(&path, &overlay, &options).expect("an answer");
+        let shown = |at: usize| {
+            let lsp::HoverContents::Markup(found) =
+                hover(&answer, at as u32).expect("a hover").contents
+            else {
+                panic!("not Markdown");
+            };
+            found.value
+        };
+        for (at, want) in [
+            (text.find("(a, b)").unwrap() + 1, "let a: Int"),
+            (text.find("(a, b)").unwrap() + 4, "let b: String"),
+            (text.find("+ a +").unwrap() + 2, "let a: Int"),
+            (text.find("b.len").unwrap(), "let b: String"),
+            (text.find("Ok(x)").unwrap() + 3, "let x: Int"),
+            (text.find("=> x").unwrap() + 3, "let x: Int"),
+            (text.find("Ok(y)").unwrap() + 3, "let y: Int"),
+        ] {
+            assert_eq!(shown(at), format!("```nova\n{want}\n```"), "at {at}");
+        }
+    }
+
     #[test]
     fn an_item_line_keeps_pub_async_and_where_and_stops_at_its_body() {
         let text = "    pub async fn f<T>(x: T) -> T\n    where T: Show\n    { x }\n";
diff --git a/crates/nova-typeck/src/check/pattern_tests.rs b/crates/nova-typeck/src/check/pattern_tests.rs
index 5a7e4b64e664556a7191006b4c2b93cd7551308f..97a9a20b3ba9a2fb95473b67f45d3f2f4a55b9ca 100644
--- a/crates/nova-typeck/src/check/pattern_tests.rs
+++ b/crates/nova-typeck/src/check/pattern_tests.rs
@@ -1,8 +1,8 @@
 //! The checker's tests of 5.0b's patterns (spec
 //! `docs/superpowers/specs/2026-10-26-phase-5-0-the-language-and-std-a-compiler-needs-design.md`
-//! §4.3, §4.4): nested, tuple, or-, `Char` and range patterns, guards,
-//! `let` and `for` destructuring, and the codes they meet: E0001, E0010,
-//! E0016, E0020, E0021, E0022, E0100, E0101 and E0104.
+//! §4.3 to §4.5): nested, tuple, or-, `Char` and range patterns, guards,
+//! `let` and `for` destructuring, `for` over an array, and the codes they
+//! meet: E0001, E0010, E0016, E0020, E0021, E0022, E0100, E0101 and E0104.
 
 use super::*;
 use nova_diagnostics::{FileId, Severity};
@@ -716,12 +716,108 @@ fn a_destructured_let_is_a_one_arm_match() {
     assert!(matches!(&pats[0], hir::Pattern::Tuple(e) if e.len() == 2));
 }
 
+#[test]
+fn for_destructures_over_an_array_and_an_iterator() {
+    clean(&with_sums(
+        "fn f(xs: [(Int, String)]) -> Int {\n    let mut n = 0\n    for (i, s) in xs {\n        n += i + s.len()\n    }\n    \
+         for x in [1, 2, 3] {\n        n += x\n    }\n    for _ in xs {\n        n += 1\n    }\n    n\n}",
+    ));
+}
+
+#[test]
+fn a_refutable_for_pattern_is_e0022() {
+    let src =
+        with_sums("fn f(xs: [Opt<Int>]) {\n    for Yes(x) in xs {\n        let y = x\n    }\n}");
+    let d = only(&src, "E0022");
+    assert_eq!(d.message, "refutable pattern in a `for` loop");
+    // Each missed value once: `(0, _)` for the literal's column, and
+    // `(1, false)` under the literal, not `(0, true)` and `(0, false)`.
+    let src = with_sums(
+        "fn f(ps: [(Int, Bool)]) {\n    for (1, true) in ps {\n        let y = 1\n    }\n}",
+    );
+    let d = only(&src, "E0022");
+    let label = label_text(&d);
+    assert!(
+        label.contains("`(0, _)`") && label.contains("`(1, false)`"),
+        "{d:?}"
+    );
+    assert!(!label.contains("`(0, true)`"), "{d:?}");
+}
+
+#[test]
+fn an_upper_case_binding_in_a_for_pattern_is_e0104() {
+    let src = with_sums(
+        "fn f(xs: [(Int, Int)]) -> Int {\n    let mut n = 0\n    for (A, c) in xs {\n        n += A + c\n    }\n    n\n}",
+    );
+    let d = only_warning(&src, "E0104");
+    assert_eq!(labelled(&src, &d), "A");
+}
+
+#[test]
+fn a_for_pattern_with_an_error_is_not_also_e0022() {
+    let src =
+        with_sums("fn f(xs: [Opt<Int>]) {\n    for Yes(Eof) in xs {\n        let y = 1\n    }\n}");
+    let errors = of(&src, Severity::Error);
+    let codes: Vec<&str> = errors.iter().map(|d| d.code.as_str()).collect();
+    assert!(!codes.contains(&"E0022"), "{errors:#?}");
+    assert!(!codes.is_empty(), "{errors:#?}");
+}
+
 #[test]
 fn a_tuple_pattern_over_a_range_is_e0010() {
     let src = with_sums("fn f() {\n    for (a, b) in 0..3 {\n        let c = a\n    }\n}");
     only(&src, "E0010");
 }
 
+#[test]
+fn a_for_loop_variable_over_an_array_is_immutable() {
+    let src = with_sums("fn f(xs: [Int]) {\n    for x in xs {\n        x = 1\n    }\n}");
+    let d = only(&src, "E0060");
+    assert!(
+        d.notes[0].contains("rebind it inside the loop's body"),
+        "{d:?}"
+    );
+    // A `for` pattern's binding is rebound too; `let mut (…)` is a `let`'s.
+    let src =
+        with_sums("fn f(xs: [(Int, Int)]) {\n    for (a, b) in xs {\n        a = b\n    }\n}");
+    let d = only(&src, "E0060");
+    assert!(d.notes[0].contains("rebind it as `let mut a = a`"), "{d:?}");
+    assert!(!d.notes[0].contains("let mut (…)"), "{d:?}");
+}
+
+#[test]
+fn for_over_an_array_desugars_to_an_index_loop() {
+    let r = checked(&with_sums(
+        "fn f(xs: [Int]) -> Int {\n    let mut n = 0\n    for x in xs {\n        n += x\n    }\n    n\n}",
+    ));
+    assert!(r.diagnostics.is_empty(), "{:#?}", r.diagnostics);
+    fn walk<'e>(e: &'e hir::Expr, out: &mut Vec<&'e hir::Expr>) {
+        out.push(e);
+        for c in child_exprs(e) {
+            walk(c, out);
+        }
+    }
+    let f = r.module.functions.iter().find(|f| f.name == "f").unwrap();
+    let mut all = Vec::new();
+    walk(&f.body, &mut all);
+    assert_eq!(
+        all.iter()
+            .filter(|e| matches!(e.kind, hir::ExprKind::ArrayLen { .. }))
+            .count(),
+        1,
+        "the length is read once"
+    );
+    assert_eq!(
+        all.iter()
+            .filter(|e| matches!(e.kind, hir::ExprKind::Index { .. }))
+            .count(),
+        1
+    );
+    assert!(all
+        .iter()
+        .any(|e| matches!(e.kind, hir::ExprKind::While { .. })));
+}
+
 // --- The newline rule and tuple patterns (5.0 spec §3.5) ---
 
 #[test]
````

**Patch `t4-code`:**

<!-- patch t4-code -->

````diff
diff --git a/crates/nova-typeck/src/check.rs b/crates/nova-typeck/src/check.rs
index bce2787e79b0ea7cd9d82d99a15f8b2cff4d5737..8560a8641b28e283f356c4e605d613dccc6ade25 100644
--- a/crates/nova-typeck/src/check.rs
+++ b/crates/nova-typeck/src/check.rs
@@ -4729,11 +4729,15 @@ impl<'a> Checker<'a> {
         if matches!(iter_ty, Ty::Error) {
             return error_expr(span);
         }
+        if let Ty::Array(elem) = &iter_ty {
+            let elem = (**elem).clone();
+            return self.check_for_array(fcx, pattern, iterable, elem, body, span);
+        }
         let Some(next) = self.iterator_next(fcx, &iter_ty) else {
             self.unsupported(
                 iter.span,
-                "`for` loops over anything but an integer range (`a..b`) or a value \
-                 implementing `Iterator` (try `.iter()`)",
+                "`for` loops over anything but an integer range (`a..b`), an array, or a \
+                 value implementing `Iterator` (try `.iter()`)",
             );
             return error_expr(span);
         };
diff --git a/crates/nova-typeck/src/check/patterns.rs b/crates/nova-typeck/src/check/patterns.rs
index 878e7bfb5999d3ff6bbc1715b081cbe7be2afe86..913c8da998fdd566cc25202822a76332f268abcf 100644
--- a/crates/nova-typeck/src/check/patterns.rs
+++ b/crates/nova-typeck/src/check/patterns.rs
@@ -641,3 +641,107 @@ fn restrict_or(pat: &usefulness::Pat, target: usize, keep: Range<usize>) -> usef
     }
     go(pat, target, &keep, &mut 0)
 }
+
+impl<'a> Checker<'a> {
+    /// `for p in a`, `a` an array (5.0 spec §4.5), desugars to
+    /// `{ let __arr = a  let __n = __arr.len()  let mut __i = 0
+    ///    while __i < __n { <bind p to __arr[__i]>  __i = __i + 1  body } }`.
+    ///
+    /// `a` is evaluated once, and each element is read when the loop reaches
+    /// it, so a write to a later element is seen. The index is advanced
+    /// before the body, so `continue` is safe. The hidden locals are
+    /// unscoped, so no source name reaches or shadows them.
+    pub(super) fn check_for_array(
+        &mut self,
+        fcx: &mut FnCtx,
+        pattern: &Spanned<ast::Pattern>,
+        iterable: hir::Expr,
+        elem: Ty,
+        body: &Spanned<ast::Block>,
+        span: Span,
+    ) -> hir::Expr {
+        let arr_ty = iterable.ty.clone();
+        let arr = fcx.new_local_unscoped("__arr".to_string(), arr_ty.clone(), false, span);
+        let len = fcx.new_local_unscoped("__n".to_string(), Ty::Int, false, span);
+        let idx = fcx.new_local_unscoped("__i".to_string(), Ty::Int, true, span);
+
+        fcx.scopes.push(Default::default());
+        let bind = self.for_loop_binding(fcx, pattern, &elem);
+        fcx.loop_depth += 1;
+        let body_hir = self.check_block(fcx, &body.value, body.span);
+        fcx.loop_depth -= 1;
+        fcx.scopes.pop();
+
+        let expr = |kind, ty| hir::Expr { kind, ty, span };
+        let read = |local, ty| expr(hir::ExprKind::Local(local), ty);
+        let let_stmt = |local, init| {
+            expr(
+                hir::ExprKind::Let {
+                    local,
+                    init: Box::new(init),
+                },
+                Ty::Unit,
+            )
+        };
+        let element = expr(
+            hir::ExprKind::Index {
+                target: Box::new(read(arr, arr_ty.clone())),
+                index: Box::new(read(idx, Ty::Int)),
+            },
+            elem,
+        );
+        let advance = expr(
+            hir::ExprKind::Assign {
+                local: idx,
+                value: Box::new(expr(
+                    hir::ExprKind::Binary {
+                        op: hir::BinOp::Add,
+                        lhs: Box::new(read(idx, Ty::Int)),
+                        rhs: Box::new(expr(hir::ExprKind::IntLit(1), Ty::Int)),
+                    },
+                    Ty::Int,
+                )),
+            },
+            Ty::Unit,
+        );
+        let cond = expr(
+            hir::ExprKind::Binary {
+                op: hir::BinOp::Lt,
+                lhs: Box::new(read(idx, Ty::Int)),
+                rhs: Box::new(read(len, Ty::Int)),
+            },
+            Ty::Bool,
+        );
+        let while_body = expr(
+            hir::ExprKind::Block {
+                stmts: vec![bind.bind(element, span), advance, body_hir],
+                trailing: None,
+            },
+            Ty::Unit,
+        );
+        let len_init = expr(
+            hir::ExprKind::ArrayLen {
+                target: Box::new(read(arr, arr_ty)),
+            },
+            Ty::Int,
+        );
+        expr(
+            hir::ExprKind::Block {
+                stmts: vec![
+                    let_stmt(arr, iterable),
+                    let_stmt(len, len_init),
+                    let_stmt(idx, expr(hir::ExprKind::IntLit(0), Ty::Int)),
+                    expr(
+                        hir::ExprKind::While {
+                            cond: Box::new(cond),
+                            body: Box::new(while_body),
+                        },
+                        Ty::Unit,
+                    ),
+                ],
+                trailing: None,
+            },
+            Ty::Unit,
+        )
+    }
+}
diff --git a/tests/runtime/fs_read_dir.nova b/tests/runtime/fs_read_dir.nova
index a83583bf3a214a6989db5c2b7859c3297c98d114..f866e99861fd3c0498a2bd7753748b08b303b30b 100644
--- a/tests/runtime/fs_read_dir.nova
+++ b/tests/runtime/fs_read_dir.nova
@@ -27,10 +27,9 @@ async fn go() {
     let _ = write_string("${d}/Zebra.txt", "z").await
     match read_dir(d).await {
         Ok(entries) => {
-            // Arrays have no `.iter()`/`for x in arr` (only `.len()` and
-            // indexing are built in -- see the task report), so this walks by
-            // index rather than the `for e in entries.iter()` shape a `Vec`
-            // would use.
+            // Written before 5.0b, when arrays had no `for x in arr`, so
+            // this walks by index; `for e in entries` works now, and so does
+            // this.
             let n = entries.len()
             let mut i = 0
             while i < n {
````

### Task 5: Lowering by test chains, and the four pattern programs

Spec §4.6, §7.3; decisions 11, 13, 16, 17, 26, 31, 34, 35 and 37.

**Files:**
- Modify:
  - `crates/nova-mir/src/lower.rs` (`lower_match` chooses the chains, where
    Task 3 refused; `lower_chain_arms`, `lower_pattern_test`,
    `branch_or_fail`, `test_int_eq`, `test_int_cmp`, `concrete`; the flat
    switch's `Char` key is Task 3's)
  - `std/http/lib.nova`, `tests/runtime/http_serialise.nova` (comments)
- Create: `tests/runtime/patterns.nova`, `patterns_async.nova`,
  `match_effects.nova`, `for_arrays.nova`, each with its `.stdout`
- Test:
  - `crates/nova-mir/tests/pattern_lowering.rs` (new, ten tests), and
    `lower_tests.rs`, which loses Task 3's refusal test
  - `crates/nova-codegen-llvm/tests/ir_tests.rs` (one test)
  - `crates/nova-cli/tests/run_tests.rs`, `wasm_tests.rs`,
    `crates/nova-driver/tests/index_complete.rs` (the four programs)

- [ ] **Step 1: The tests.** `patch_block t5-tests`. The patch holds:
  - `pattern_lowering.rs`, MIR from source: a flat match keeps its one
    switch, its `Char` keys `97` and `98` among them; a nested match lowers
    by chains, with no switch; each projection once per chain; the scrutinee
    once; a false guard's branch to the next arm's chain; an or-pattern
    filling its bindings on every path, each local copied into once per
    alternative, and sharing one body; a range as two compares, its ends
    included; `let` destructuring as projections alone, with no tag test; a
    `Bool` literal as a branch on the value; a string literal at depth as
    one `StrEq`.
  - `ir_tests.rs`: a nested match emits chains and a flat one still
    `switch`es, in the LLVM IR.
  - `lower_tests.rs`: `a_nested_or_guarded_match_is_refused_until_chains`
    goes, as the chains lower what it refuses (decision 35).
  - **The programs:** `patterns`, nested, guarded, or-, `Char` and range
    patterns, tuple patterns and `let` destructuring over a token type;
    `patterns_async`, nested matches and guards that `.await`;
    `match_effects`, a scrutinee evaluated once and guards run in arm order,
    each at most once, only after its pattern matched, which it prints; and
    `for_arrays`, `for` over arrays, empty, with `break` and `continue`,
    tuple patterns, nested loops and a write the loop sees. Each has its
    `.stdout`, a test in `run_tests.rs`, under `run` and `build`, and under
    GC stress for all but `match_effects`, an entry in `wasm_tests.rs`'s
    `gate!`, and `index_complete.rs`'s counts, 175 programs and 209 files.
- [ ] **Step 2: Run, and see the tests fail.** First the build, which ends
  with `Finished`.
  - `cargo test --locked --workspace --test pattern_lowering`. Expected:
    `1 passed; 9 failed`. The nine fail at "MIR lowering failed: [Diagnostic
    { severity: Error, code: "E0900", message: "matches with nested, tuple,
    or- or range patterns or guards are not supported yet" …", Task 3's
    refusal. `a_flat_match_keeps_its_one_switch` passes, its matches flat
    and its `Char` keys Task 3's (decision 26).
  - `cargo test --locked --workspace --test ir_tests -- match`. Expected:
    `3 passed; 1 failed`:
    `a_nested_match_emits_chains_and_a_flat_one_still_switches`, at "mir
    lowering: [Diagnostic { … code: "E0900" …", the refusal.
  - `R patterns_run patterns_async_runs match_effects_run for_arrays_run`.
    Expected: `0 passed; 4 failed`, each `nova run` exiting with `code=1`
    and "error[E0900]: matches with nested, tuple, or- or range patterns or
    guards are not supported yet": the refusal, where without it the flat
    path would run them wrongly (decision 35).
  - `W gate::patterns gate::patterns_async gate::match_effects gate::for_arrays every_runtime_program_is_in_the_gate_or_left_out`.
    Expected: `1 passed; 4 failed`, the four programs, as above.
  - `X`. Expected: `1 passed; 0 failed`: the four programs check cleanly
    since Task 4, and only their lowering is missing.
- [ ] **Step 3: The code.** `patch_block t5-code`:
  - **`lower_match`:** Task 3's refusal goes; when any arm has a guard or a
    pattern `is_flat` refuses, `lower_chain_arms` on the scrutinee's
    temporary, then the join; the flat switch otherwise, with Task 3's
    `Char` keys.
  - **`lower_chain_arms`:** per arm, a fresh `next` block; the pattern's
    tests; the guard's `Branch` to the body or `next`; the body, copied into
    the result; `Goto` the join; then `next`. After the last arm a `Trap`.
  - **`lower_pattern_test`:** `_` nothing; a binding a copy; `Int` and
    `Char` an `==`; `Bool` a `Branch` on the value; `String` a `StrEq` call;
    a range `>=` then `<=`; a variant its tag (none for a sum of one
    variant), then each field not `_` projected by `SumField` and tested; a
    tuple each element by `RecordField`; an or-pattern each alternative in
    turn, failing to the next, matching to one shared block.
  - **The comments** of `std/http/lib.nova` and `http_serialise.nova` say
    nested patterns work now (decision 39).
- [ ] **Step 4: Run.** First the build, which ends with `Finished`.
  - `cargo test --locked --workspace --test pattern_lowering`. Expected:
    `10 passed; 0 failed`.
  - `cargo test --locked --workspace --test ir_tests -- match`. Expected:
    `4 passed; 0 failed`.
  - `R patterns_run patterns_async_runs match_effects_run for_arrays_run`.
    Expected: `4 passed; 0 failed`.
  - `W gate::patterns gate::patterns_async gate::match_effects gate::for_arrays every_runtime_program_is_in_the_gate_or_left_out`.
    Expected: `5 passed; 0 failed`.
  - `X`. Expected: `1 passed; 0 failed`.
  - `S`, as in Task 1 (about 5 min 0 s). Expected: `2697 1 10`: Task 4's
    figures and Task 5's 18 tests, net of the refusal's test that goes, the
    one failure the IPv6 test.
  - `R http_serialise`. Expected: `1 passed; 0 failed`.
  - `G`. Expected:
    `release golden: 137 passed, 9 excluded, Ubuntu clang version 18.1.3 (1ubuntu1)`
    and `1 passed`: the four programs through LLVM.
- [ ] **Step 5: The checks,** as the conventions list them.
- [ ] **Step 6: Commit**
  `mir: lower a match by test chains, flat matches unchanged`.

**Patch `t5-tests`:**

<!-- patch t5-tests -->

````diff
diff --git a/crates/nova-cli/tests/run_tests.rs b/crates/nova-cli/tests/run_tests.rs
index ce9c31381c18a1cc315f7b7a7d027f9fcfe9b895..e9c24c889b04e1105a2378e06f635f77aad269af 100644
--- a/crates/nova-cli/tests/run_tests.rs
+++ b/crates/nova-cli/tests/run_tests.rs
@@ -13010,6 +13010,36 @@ fn tuples_async_runs() {
     golden_three_ways("tuples_async", true);
 }
 
+/// Nested, guarded, or-, `Char` and range patterns, tuple patterns, and
+/// `let` destructuring (5.0 spec §4.3, §4.6), also under GC stress, since
+/// its patterns project strings and sums.
+#[test]
+fn patterns_run() {
+    golden_three_ways("patterns", true);
+}
+
+/// Nested matches and guards that `.await`, in `async fn`s, under GC stress
+/// (5.0 spec §4.6, §7.3).
+#[test]
+fn patterns_async_runs() {
+    golden_three_ways("patterns_async", true);
+}
+
+/// A `match`'s scrutinee is evaluated once, and its guards run in arm
+/// order, each at most once, only after its pattern matched (5.0 spec
+/// §4.3, §4.6).
+#[test]
+fn match_effects_run() {
+    golden_three_ways("match_effects", false);
+}
+
+/// `for p in arr` over an array (5.0 spec §4.5): empty, `break`,
+/// `continue`, tuple patterns, and a write the loop sees.
+#[test]
+fn for_arrays_run() {
+    golden_three_ways("for_arrays", true);
+}
+
 /// E0102 (5.0 spec §3.7) is a warning: it prints, and the program still
 /// builds and runs, its own `Number` meaning its own variant.
 #[test]
diff --git a/crates/nova-cli/tests/wasm_tests.rs b/crates/nova-cli/tests/wasm_tests.rs
index 3dba9640f843318900ea00370ab8fba62fc6c9f1..82fed9e7436f0beb4d44e1969e6d1a23d06189ca 100644
--- a/crates/nova-cli/tests/wasm_tests.rs
+++ b/crates/nova-cli/tests/wasm_tests.rs
@@ -1958,6 +1958,12 @@ mod gate {
         // nothing (5.0 spec §4.1).
         tuples,
         tuples_async,
+        // Patterns and `for` over arrays: test chains of MIR's existing
+        // statements, so WASM needs nothing (5.0 spec §4.3 to §4.6).
+        patterns,
+        patterns_async,
+        match_effects,
+        for_arrays,
     );
 
     /// The programs a test of their own in this module checks, by its name.
diff --git a/crates/nova-codegen-llvm/tests/ir_tests.rs b/crates/nova-codegen-llvm/tests/ir_tests.rs
index 2064911490aa76feb237ed59e45d114f782d9f71..28b9bf2760805c81e75f3929df7a5ea5b2714166 100644
--- a/crates/nova-codegen-llvm/tests/ir_tests.rs
+++ b/crates/nova-codegen-llvm/tests/ir_tests.rs
@@ -142,6 +142,31 @@ fn sum_match_still_switches_on_i64() {
     assert_well_formed(&ir);
 }
 
+/// 5.0b (5.0 spec §4.6): a match with a nested pattern lowers by test
+/// chains, compares and conditional branches, with no `switch`; a flat
+/// match beside it keeps its `switch`.
+#[test]
+fn a_nested_match_emits_chains_and_a_flat_one_still_switches() {
+    let nested = ir_for(
+        "type E = | A(Int) | B\n\
+         fn f(o: (E, Int)) -> Int { match o { (A(1..=3), _) => 1, (A(_), n) if n > 0 => 2, _ => 3 } }\n\
+         fn main() { println(\"${f((B, 1))}\") }",
+    );
+    let body = &nested[nested.find("define").unwrap()..];
+    assert!(!body.contains("switch "), "{nested}");
+    assert!(nested.contains("icmp eq i64"), "{nested}");
+    assert!(nested.contains("icmp sge i64"), "{nested}");
+    assert!(nested.contains("br i1"), "{nested}");
+    assert_well_formed(&nested);
+    let flat = ir_for(
+        "type E = | A(Int) | B\n\
+         fn f(e: E) -> Int { match e { A(n) => n, B => 0 } }\n\
+         fn main() { println(\"${f(B)}\") }",
+    );
+    assert!(flat.contains("switch i64"), "{flat}");
+    assert_well_formed(&flat);
+}
+
 #[test]
 fn floats_use_hex_constants_and_fp_ops() {
     let ir = ir_for(
diff --git a/crates/nova-driver/tests/index_complete.rs b/crates/nova-driver/tests/index_complete.rs
index f8666d5f4488a54162594b3394817c6171c97aa2..301faefeeaabca0c9d310f9c075b610525c29bd1 100644
--- a/crates/nova-driver/tests/index_complete.rs
+++ b/crates/nova-driver/tests/index_complete.rs
@@ -247,8 +247,9 @@ fn every_name_in_the_corpora_is_indexed() {
     // `crypto_pbkdf2_refused` and `bytes_hex`, and `07-fullstack-blog`;
     // 159 runtime programs on 2026-10-26, after 5.0a added `try_operator`,
     // `try_async`, `compound_assign_places` and `string_concat`; 161 on
-    // 2026-10-27, after 5.0b added `tuples` and `tuples_async`.
-    assert_eq!(programs.len(), 171, "{programs:#?}");
+    // 2026-10-27, after 5.0b added `tuples` and `tuples_async`; 165 after
+    // `patterns`, `patterns_async`, `match_effects` and `for_arrays`.
+    assert_eq!(programs.len(), 175, "{programs:#?}");
     let mut files = 0;
     let mut failures: Vec<String> = Vec::new();
     for (k, path) in programs.iter().enumerate() {
@@ -275,11 +276,11 @@ fn every_name_in_the_corpora_is_indexed() {
             every_name_is_covered(&a, file, &mut failures);
         }
     }
-    // 9 + 161 + 3 files of user code, 05-geometry's library's 2, the 8
+    // 9 + 165 + 3 files of user code, 05-geometry's library's 2, the 8
     // modules 07-fullstack-blog's client reaches from its `main.nova` (not
     // the server's 4, which only `server.nova` and `lib.nova` reach), and
     // native std's 22, `std/ui/ssr` among them.
-    assert_eq!(files, 205);
+    assert_eq!(files, 209);
     failures.sort();
     failures.dedup();
     assert!(
diff --git a/crates/nova-mir/tests/lower_tests.rs b/crates/nova-mir/tests/lower_tests.rs
index c01a40087afcbfb7f2415492947a9328bba1c0a7..def2254f861dec6bec9900baa69e02e091b79401 100644
--- a/crates/nova-mir/tests/lower_tests.rs
+++ b/crates/nova-mir/tests/lower_tests.rs
@@ -2347,32 +2347,6 @@ fn a_tuple_of_one_in_a_bound_message_keeps_its_comma() {
     }
 }
 
-/// Until test-chain lowering (5.0 spec §4.6), lowering refuses a match it
-/// would lower wrongly, one with a nested pattern or a guard: E0900.
-#[test]
-fn a_nested_or_guarded_match_is_refused_until_chains() {
-    for body in [
-        "match o {\n        Some(Some(n)) => n\n        _ => 0\n    }",
-        "match o {\n        Some(_) if true => 1\n        _ => 0\n    }",
-    ] {
-        let src = format!(
-            "fn f(o: Option<Option<Int>>) -> Int {{\n    {body}\n}}\n\
-             fn main() {{ println(\"${{f(None)}}\") }}"
-        );
-        let file_id = FileId::DUMMY;
-        let (tokens, _) = lex(&src, file_id);
-        let (ast, _) = parse(&tokens, file_id);
-        let resolved = resolve(&ast.expect("an AST"));
-        let checked = check(&resolved.file, &resolved.definitions);
-        assert!(checked.diagnostics.is_empty(), "{:?}", checked.diagnostics);
-        let diags = match lower_module(&checked.module) {
-            Ok(_) => panic!("lowered: {body}"),
-            Err(d) => d,
-        };
-        assert!(diags.iter().any(|d| d.code == "E0900"), "{diags:?}");
-    }
-}
-
 /// A flat `Char` match, which the checker takes from Task 3 on, switches on
 /// its literals' code points, as an `Int` match switches on its literals.
 #[test]
diff --git a/crates/nova-mir/tests/pattern_lowering.rs b/crates/nova-mir/tests/pattern_lowering.rs
new file mode 100644
index 0000000000000000000000000000000000000000..b5b023927adbd4654a435bd8d7aefa2300292d03
--- /dev/null
+++ b/crates/nova-mir/tests/pattern_lowering.rs
@@ -0,0 +1,273 @@
+//! `match` lowering by test chains (5.0 spec
+//! `docs/superpowers/specs/2026-10-26-phase-5-0-the-language-and-std-a-compiler-needs-design.md`
+//! §4.6): a flat match keeps its one `Switch`; any other match lowers wholly
+//! by chains, which evaluate the scrutinee and each projection once, send a
+//! false guard to the next arm's chain, and fill an or-pattern's bindings
+//! on every path.
+
+use nova_diagnostics::FileId;
+use nova_mir::{Function, Stmt, Terminator};
+
+fn mir_for(src: &str) -> nova_mir::Module {
+    let file_id = FileId::DUMMY;
+    let (tokens, lex_errors) = nova_lexer::lex(src, file_id);
+    assert!(lex_errors.is_empty(), "lex errors: {lex_errors:?}");
+    let (ast, parse_errors) = nova_parser::parse(&tokens, file_id);
+    assert!(parse_errors.is_empty(), "parse errors: {parse_errors:?}");
+    let resolved = nova_resolver::resolve(&ast.expect("no AST"));
+    assert!(
+        resolved.diagnostics.is_empty(),
+        "resolve: {:?}",
+        resolved.diagnostics
+    );
+    let checked = nova_typeck::check(&resolved.file, &resolved.definitions);
+    assert!(
+        checked.diagnostics.is_empty(),
+        "typeck: {:?}",
+        checked.diagnostics
+    );
+    nova_mir::lower_module(&checked.module).expect("MIR lowering failed")
+}
+
+/// The function named `name`, or its one instance.
+fn func<'m>(mir: &'m nova_mir::Module, name: &str) -> &'m Function {
+    let instance = format!("{name}.");
+    mir.functions
+        .iter()
+        .find(|f| f.name == name || f.name.starts_with(&instance))
+        .unwrap_or_else(|| panic!("no function `{name}`"))
+}
+
+fn stmts(f: &Function) -> Vec<&Stmt> {
+    f.blocks.iter().flat_map(|b| &b.stmts).collect()
+}
+
+fn count(f: &Function, pred: impl Fn(&Stmt) -> bool) -> usize {
+    stmts(f).into_iter().filter(|s| pred(s)).count()
+}
+
+fn switches(f: &Function) -> usize {
+    f.blocks
+        .iter()
+        .filter(|b| matches!(b.term, Terminator::Switch { .. }))
+        .count()
+}
+
+fn branches(f: &Function) -> usize {
+    f.blocks
+        .iter()
+        .filter(|b| matches!(b.term, Terminator::Branch { .. }))
+        .count()
+}
+
+const SUMS: &str = "type Opt<T> =\n  | Yes(T)\n  | No\n\
+                    type Wrap =\n  | Wrap(Int, String)\n";
+
+fn program(body: &str, call: &str) -> String {
+    format!("{SUMS}{body}\nfn main() {{ println(\"${{{call}}}\") }}\n")
+}
+
+#[test]
+fn a_flat_match_keeps_its_one_switch() {
+    let mir = mir_for(&program(
+        "fn f(o: Opt<Int>) -> Int { match o { Yes(x) => x, No => 0 } }",
+        "f(No)",
+    ));
+    let f = func(&mir, "f");
+    assert_eq!(switches(f), 1);
+    assert_eq!(branches(f), 0);
+    // A `Char` literal match is flat too: one switch on code points.
+    let mir = mir_for(&program(
+        "fn f(c: Char) -> Int { match c { 'a' => 1, 'b' => 2, _ => 3 } }",
+        "f('a')",
+    ));
+    let f = func(&mir, "f");
+    let arms: Vec<i64> = f
+        .blocks
+        .iter()
+        .find_map(|b| match &b.term {
+            Terminator::Switch { arms, .. } => Some(arms.iter().map(|(v, _)| *v).collect()),
+            _ => None,
+        })
+        .expect("a switch");
+    assert_eq!(arms, ['a' as i64, 'b' as i64]);
+    assert_eq!(branches(f), 0);
+}
+
+#[test]
+fn a_nested_match_lowers_by_chains_with_no_switch() {
+    let mir = mir_for(&program(
+        "fn f(o: Opt<Opt<Int>>) -> Int { match o { Yes(Yes(1)) => 1, Yes(_) => 2, No => 3 } }",
+        "f(No)",
+    ));
+    let f = func(&mir, "f");
+    assert_eq!(switches(f), 0);
+    // One tag test per variant each arm names: two, one and one.
+    assert_eq!(count(f, |s| matches!(s, Stmt::SumTag { .. })), 4);
+}
+
+#[test]
+fn each_projection_is_evaluated_once_per_chain() {
+    let mir = mir_for(&program(
+        "fn f(o: Opt<(Int, Int)>) -> Int { match o { Yes((a, b)) => a + b, No => 0 } }",
+        "f(No)",
+    ));
+    let f = func(&mir, "f");
+    assert_eq!(count(f, |s| matches!(s, Stmt::SumField { .. })), 1);
+    assert_eq!(count(f, |s| matches!(s, Stmt::RecordField { .. })), 2);
+}
+
+#[test]
+fn the_scrutinee_is_evaluated_once() {
+    let mir = mir_for(&program(
+        "fn g() -> (Int, Int) { (1, 2) }\n\
+         fn f() -> Int { match g() { (1, _) => 1, (_, 2) => 2, _ => 3 } }",
+        "f()",
+    ));
+    let f = func(&mir, "f");
+    let calls = count(
+        f,
+        |s| matches!(s, Stmt::Call { callee, .. } if callee.starts_with("g")),
+    );
+    assert_eq!(calls, 1);
+}
+
+#[test]
+fn a_false_guard_jumps_to_the_next_arms_chain() {
+    let mir = mir_for(&program(
+        "fn f(n: Int) -> Int { match n { x if x > 5 => 1, 7 => 2, _ => 3 } }",
+        "f(7)",
+    ));
+    let f = func(&mir, "f");
+    // The guard's compare is the one `>` in the function; the block it
+    // ends in branches on it, and its false edge enters the next arm,
+    // whose first test loads the literal 7.
+    let (guard_block, else_) = f
+        .blocks
+        .iter()
+        .find_map(|b| {
+            let has_gt = b.stmts.iter().any(|s| {
+                matches!(
+                    s,
+                    Stmt::Bin {
+                        op: nova_hir::BinOp::Gt,
+                        ..
+                    }
+                )
+            });
+            match (&b.term, has_gt) {
+                (Terminator::Branch { else_, .. }, true) => Some((b, *else_)),
+                _ => None,
+            }
+        })
+        .expect("the guard's branch");
+    assert!(
+        matches!(
+            f.blocks[else_.0 as usize].stmts.first(),
+            Some(Stmt::ConstInt(_, 7))
+        ),
+        "{:?} then {:?}",
+        guard_block,
+        f.blocks[else_.0 as usize]
+    );
+}
+
+#[test]
+fn an_or_pattern_fills_its_bindings_on_every_path_and_shares_one_body() {
+    let mir = mir_for(&program(
+        "fn f(p: (Int, Int)) -> Int { match p { (a, 0) | (0, a) => a, _ => 0 } }",
+        "f((0, 4))",
+    ));
+    let f = func(&mir, "f");
+    // `a` is written once in each alternative, into the same temporary.
+    let mut dsts: Vec<u32> = stmts(f)
+        .into_iter()
+        .filter_map(|s| match s {
+            Stmt::Copy { dst, .. } => Some(dst.0),
+            _ => None,
+        })
+        .collect();
+    dsts.sort();
+    let a = dsts
+        .windows(2)
+        .find(|w| w[0] == w[1])
+        .map(|w| w[0])
+        .expect("one local copied into twice");
+    assert_eq!(dsts.iter().filter(|d| **d == a).count(), 2, "{dsts:?}");
+    // Both alternatives' successes jump to one block.
+    let mut gotos: Vec<u32> = f
+        .blocks
+        .iter()
+        .filter_map(|b| match b.term {
+            Terminator::Goto(t) => Some(t.0),
+            _ => None,
+        })
+        .collect();
+    gotos.sort();
+    assert!(gotos.windows(2).any(|w| w[0] == w[1]), "{gotos:?}");
+}
+
+#[test]
+fn a_range_is_two_compares_with_its_ends_included() {
+    let mir = mir_for(&program(
+        "fn f(n: Int) -> Int { match n { 1..5 => 1, 10..=20 => 2, _ => 0 } }",
+        "f(3)",
+    ));
+    let f = func(&mir, "f");
+    let consts: Vec<i64> = stmts(f)
+        .into_iter()
+        .filter_map(|s| match s {
+            Stmt::ConstInt(_, v) => Some(*v),
+            _ => None,
+        })
+        .collect();
+    // `1..5` is tested as `1..=4`.
+    for end in [1, 4, 10, 20] {
+        assert!(consts.contains(&end), "{consts:?}");
+    }
+    assert!(!consts.contains(&5), "{consts:?}");
+    let op = |want: nova_hir::BinOp| count(f, |s| matches!(s, Stmt::Bin { op, .. } if *op == want));
+    assert_eq!(op(nova_hir::BinOp::Ge), 2);
+    assert_eq!(op(nova_hir::BinOp::Le), 2);
+}
+
+#[test]
+fn let_destructuring_lowers_to_projections_alone() {
+    let mir = mir_for(&program(
+        "fn f() -> Int {\n    let (Wrap(n, _), (m, k)) = (Wrap(1, \"s\"), (2, 3))\n    n + m + k\n}",
+        "f()",
+    ));
+    let f = func(&mir, "f");
+    assert_eq!(branches(f), 0);
+    assert_eq!(switches(f), 0);
+    // A sum of one variant needs no tag test.
+    assert_eq!(count(f, |s| matches!(s, Stmt::SumTag { .. })), 0);
+    assert_eq!(count(f, |s| matches!(s, Stmt::SumField { .. })), 1);
+}
+
+#[test]
+fn a_bool_literal_branches_on_the_value() {
+    let mir = mir_for(&program(
+        "fn f(p: (Bool, Int)) -> Int { match p { (true, n) => n, (false, _) => 0 } }",
+        "f((true, 1))",
+    ));
+    let f = func(&mir, "f");
+    assert_eq!(count(f, |s| matches!(s, Stmt::Bin { .. })), 0);
+    assert_eq!(branches(f), 2);
+}
+
+#[test]
+fn a_string_literal_at_depth_is_one_string_compare() {
+    let mir = mir_for(&program(
+        "fn f(o: Opt<String>) -> Int { match o { Yes(\"a\") => 1, Yes(_) => 2, No => 3 } }",
+        "f(No)",
+    ));
+    let f = func(&mir, "f");
+    assert_eq!(
+        count(
+            f,
+            |s| matches!(s, Stmt::CallRuntime { func, .. } if *func == nova_mir::RtFunc::StrEq)
+        ),
+        1
+    );
+}
diff --git a/tests/runtime/for_arrays.nova b/tests/runtime/for_arrays.nova
new file mode 100644
index 0000000000000000000000000000000000000000..fb9fa2759e216a151da36ee656f83ffdbdaac05d
--- /dev/null
+++ b/tests/runtime/for_arrays.nova
@@ -0,0 +1,69 @@
+// `for p in arr` over an array (5.0 spec §4.5): in order, an empty array,
+// `break` and `continue`, a tuple pattern and a nested one, a write to a
+// later element seen when the loop reaches it, the array evaluated once,
+// arrays of strings and of sums, and nested loops.
+
+record Counter { n: Int }
+
+fn numbers(mut c: Counter) -> [Int] {
+    c.n += 1;
+    [1, 2, 3]
+}
+
+fn main() {
+    let empty: [Int] = []
+    for x in empty { println("never ${x}") }
+    println("empty done")
+
+    let mut sum = 0
+    for x in [1, 2, 3, 4, 5, 6] {
+        if x == 2 { continue }
+        if x == 5 { break }
+        sum += x
+    }
+    println("sum ${sum}")
+
+    for (name, n) in [("a", 1), ("b", 2), ("c", 3)] { println("${name}=${n}") }
+
+    for (i, (word, ok)) in [(0, ("zero", true)), (1, ("one", false))] {
+        println("${i} ${word} ${ok}")
+    }
+
+    for (_, n) in [("skip", 10), ("skip", 20)] { println("n ${n}") }
+
+    let mut arr = [1, 2, 3, 4]
+    for x in arr {
+        if x == 1 { arr[2] = 30 }
+        println("x ${x}")
+    }
+
+    let c = Counter { n: 0 }
+    for x in numbers(c) { println("got ${x}") }
+    println("evaluated ${c.n} time")
+
+    for w in ["one", "two"] {
+        for opt in [Some(w), None] {
+            match opt {
+                Some(s) => println("some ${s}")
+                None => println("none")
+            }
+        }
+    }
+
+    let mut lens = 0
+    for s in ["abc", "de", "f"] { lens += s.len() }
+    println("lens ${lens}")
+
+    let mut found = 0 - 1
+    let grid = [[1, 2], [3, 4], [5, 6]]
+    for row in grid {
+        for v in row {
+            if v == 4 {
+                found = v
+                break
+            }
+        }
+        if found > 0 { break }
+    }
+    println("found ${found}")
+}
diff --git a/tests/runtime/for_arrays.stdout b/tests/runtime/for_arrays.stdout
new file mode 100644
index 0000000000000000000000000000000000000000..c8aa41842173f790716dff159c3181677e56ab40
--- /dev/null
+++ b/tests/runtime/for_arrays.stdout
@@ -0,0 +1,23 @@
+empty done
+sum 8
+a=1
+b=2
+c=3
+0 zero true
+1 one false
+n 10
+n 20
+x 1
+x 2
+x 30
+x 4
+got 1
+got 2
+got 3
+evaluated 1 time
+some one
+none
+some two
+none
+lens 6
+found 4
diff --git a/tests/runtime/match_effects.nova b/tests/runtime/match_effects.nova
new file mode 100644
index 0000000000000000000000000000000000000000..404b69fca77eabdbbde64555531499cb5db5ee11
--- /dev/null
+++ b/tests/runtime/match_effects.nova
@@ -0,0 +1,60 @@
+// The order and number of a `match`'s side effects (5.0 spec §4.3, §4.6):
+// the scrutinee is evaluated once, however many arms test it; each guard
+// runs only when its arm's pattern matched, after its bindings, in arm
+// order, at most once per match, and a false guard goes on to the next
+// arm; or-pattern alternatives run no guard twice.
+
+record Log { lines: Vec<String> }
+
+fn note(mut log: Log, s: String) { log.lines.push(s) }
+
+fn next_value(mut log: Log, v: Option<(Int, String)>) -> Option<(Int, String)> {
+    note(log, "scrutinee")
+    v
+}
+
+fn check(mut log: Log, label: String, result: Bool) -> Bool {
+    note(log, "guard ${label}")
+    result
+}
+
+fn run(log: Log, v: Option<(Int, String)>) -> String {
+    match next_value(log, v) {
+        Some((0, s)) if check(log, "zero ${s}", s == "keep") => "zero kept"
+        Some((n, "a") | (n, "b")) if check(log, "ab ${n}", n > 1) => "ab ${n}"
+        Some((n, s)) if check(log, "any ${n} ${s}", n % 2 == 0) => "even ${n}"
+        Some((n, _)) if check(log, "late ${n}", false) => "never"
+        Some(_) => "other"
+        None => "none"
+    }
+}
+
+fn flush(mut log: Log) {
+    let mut i = 0
+    while i < log.lines.len() {
+        match log.lines.get(i) {
+            Some(line) => println("  ${line}")
+            None => {}
+        }
+        i += 1
+    }
+    log.lines.clear()
+}
+
+fn main() {
+    let log = Log { lines: Vec::new() }
+    let cases = [
+        Some((0, "keep")),
+        Some((0, "drop")),
+        Some((3, "a")),
+        Some((1, "b")),
+        Some((4, "c")),
+        Some((5, "d")),
+        None,
+    ]
+    for c in cases {
+        let r = run(log, c)
+        println(r)
+        flush(log)
+    }
+}
diff --git a/tests/runtime/match_effects.stdout b/tests/runtime/match_effects.stdout
new file mode 100644
index 0000000000000000000000000000000000000000..66c15ed2e822a3bacc3d90d0698c478f21ec0a89
--- /dev/null
+++ b/tests/runtime/match_effects.stdout
@@ -0,0 +1,24 @@
+zero kept
+  scrutinee
+  guard zero keep
+even 0
+  scrutinee
+  guard zero drop
+  guard any 0 drop
+ab 3
+  scrutinee
+  guard ab 3
+other
+  scrutinee
+  guard ab 1
+  guard any 1 b
+  guard late 1
+even 4
+  scrutinee
+  guard any 4 c
+other
+  scrutinee
+  guard any 5 d
+  guard late 5
+none
+  scrutinee
diff --git a/tests/runtime/patterns.nova b/tests/runtime/patterns.nova
new file mode 100644
index 0000000000000000000000000000000000000000..b95c04688fe4acc57f7b34206410fa1791be414a
--- /dev/null
+++ b/tests/runtime/patterns.nova
@@ -0,0 +1,179 @@
+// Patterns (5.0 spec §4.3, §4.6): variants inside variants, tuples inside
+// variants and variants inside tuples, guards that fall through to the next
+// arm, or-patterns at the top and at depth that bind the same names,
+// `Char` literals and ranges, `Int` ranges, `String` and `Bool` literals
+// nested, a generic sum, a unit payload, and `let` destructuring of tuples
+// and of a sum with one variant. Its values hold strings and sums, so it
+// also runs under GC stress.
+
+type Token = | Ident(String) | Num(Int) | Sym(Char) | Pair(Token, Token) | Eof
+
+type Wrapper = | Wrap(String, Int)
+
+type Tree<T> = | Leaf(T) | Node(Tree<T>, Tree<T>)
+
+type Signal = | Ping(()) | Stop
+
+fn is_keyword(s: String) -> Bool { s == "fn" || s == "let" || s == "match" }
+
+fn classify(tok: Option<Token>) -> String {
+    match tok {
+        Some(Token::Ident(name)) if is_keyword(name) => "keyword ${name}"
+        Some(Token::Ident(name)) => "ident ${name}"
+        Some(Token::Num(0) | Token::Num(1)) => "small"
+        Some(Token::Num(n)) if n < 0 => "negative ${n}"
+        Some(Token::Num(10..20)) => "teen"
+        Some(Token::Num(n)) => "number ${n}"
+        Some(Token::Sym('a'..='z' | 'A'..='Z')) => "letter"
+        Some(Token::Sym('0'..='9')) => "digit"
+        Some(Token::Sym(c)) => "symbol ${c}"
+        Some(Token::Pair(Token::Num(a), Token::Num(b))) => "numbers ${a + b}"
+        Some(Token::Pair(Token::Ident(x), _) | Token::Pair(_, Token::Ident(x))) => "pair with ${x}"
+        Some(Token::Pair(_, _)) => "pair"
+        Some(Token::Eof) => "eof"
+        None => "nothing"
+    }
+}
+
+fn depth<T>(t: Tree<T>) -> Int {
+    match t {
+        Leaf(_) => 1
+        Node(Leaf(_), Leaf(_)) => 2
+        Node(l, r) => {
+            let a = depth(l)
+            let b = depth(r)
+            if a > b { a + 1 } else { b + 1 }
+        }
+    }
+}
+
+fn sum_leaves(t: Tree<Int>) -> Int {
+    match t {
+        Leaf(n) => n
+        Node(l, r) => sum_leaves(l) + sum_leaves(r)
+    }
+}
+
+fn grid(p: (Int, Int)) -> String {
+    match p {
+        (0, 0) => "origin"
+        (0, _) | (_, 0) => "axis"
+        (x, y) if x == y => "diagonal ${x}"
+        (x, y) if x + y == 0 => "anti ${x}"
+        (1..=3, 1..=3) => "near"
+        (_, _) => "far"
+    }
+}
+
+fn both(a: Option<Int>, b: Result<String, Int>) -> String {
+    match (a, b) {
+        (Some(n), Ok(s)) => "${n} and ${s}"
+        (Some(n), Err(e)) => "err ${n} ${e}"
+        (None, Err(e)) => "err ${e}"
+        (None, Ok("special")) => "special"
+        (None, Ok(s)) => "only ${s}"
+    }
+}
+
+fn pick(p: (Int, Int)) -> Int {
+    match p {
+        (a, 0) | (0, a) => a
+        (a, b) => a * b
+    }
+}
+
+fn flags(t: (Bool, Bool, String)) -> String {
+    match t {
+        (true, true, _) => "both"
+        (true, false, "x") => "first x"
+        (false, _, name) | (true, false, name) => "name ${name}"
+    }
+}
+
+fn signal(s: Signal) -> String {
+    match s {
+        Ping(()) => "ping"
+        Stop => "stop"
+    }
+}
+
+fn div_mod(a: Int, b: Int) -> (Int, Int) { (a / b, a % b) }
+
+fn nested_tuple() -> ((Int, String), (Bool, Char)) { ((7, "seven"), (true, 'z')) }
+
+fn main() {
+    let toks = [
+        Some(Token::Ident("fn")),
+        Some(Token::Ident("x")),
+        Some(Token::Num(1)),
+        Some(Token::Num(0 - 5)),
+        Some(Token::Num(15)),
+        Some(Token::Num(20)),
+        Some(Token::Sym('q')),
+        Some(Token::Sym('Q')),
+        Some(Token::Sym('7')),
+        Some(Token::Sym('+')),
+        Some(Token::Pair(Token::Num(2), Token::Num(3))),
+        Some(Token::Pair(Token::Sym('-'), Token::Ident("right"))),
+        Some(Token::Pair(Token::Ident("left"), Token::Eof)),
+        Some(Token::Pair(Token::Eof, Token::Eof)),
+        Some(Token::Eof),
+        None,
+    ]
+    for tok in toks { println(classify(tok)) }
+
+    let t = Node(Node(Leaf(1), Leaf(2)), Node(Leaf(3), Node(Leaf(4), Leaf(5))))
+    println("depth ${depth(t)} sum ${sum_leaves(t)}")
+    println("depth ${depth(Node(Leaf("a"), Leaf("b")))}")
+
+    let points = [(0, 0), (0, 4), (5, 0), (2, 2), (3, 0 - 3), (1, 3), (9, 4)]
+    for p in points { println(grid(p)) }
+
+    println(both(Some(1), Ok("one")))
+    println(both(Some(2), Err(20)))
+    println(both(None, Err(30)))
+    println(both(None, Ok("special")))
+    println(both(None, Ok("plain")))
+
+    println("${pick((4, 0))} ${pick((0, 5))} ${pick((2, 3))}")
+
+    println(flags((true, true, "a")))
+    println(flags((true, false, "x")))
+    println(flags((true, false, "y")))
+    println(flags((false, true, "z")))
+
+    println(signal(Ping(())))
+    println(signal(Stop))
+
+    let (q, r) = div_mod(17, 5)
+    println("${q} ${r}")
+    let ((n, word), (flag, ch)) = nested_tuple()
+    println("${n} ${word} ${flag} ${ch}")
+    let (_, (_, last)) = nested_tuple()
+    println("last ${last}")
+    let Wrap(label, count) = Wrap("apples", 3)
+    println("${label} ${count}")
+    let mut (lo, hi) = (1, 10)
+    lo += 1
+    hi -= 1
+    println("${lo}..${hi}")
+    let (one,) = (1,)
+    println("one ${one}")
+
+    // In a closure: a guard that reads a capture, and a destructuring
+    // `let` and `for` in its body.
+    let limit = 3
+    let size = |o: Option<(Int, String)>| match o {
+        Some((n, s)) if n > limit => "big ${s}"
+        Some((0, s) | (1, s)) => "tiny ${s}"
+        Some((_, s)) => "small ${s}"
+        None => "none"
+    }
+    println("${size(Some((5, "a")))} ${size(Some((1, "b")))} ${size(Some((2, "c")))} ${size(None)}")
+    let product = |p: (Int, Int)| {
+        let (a, b) = p
+        for (x, y) in [(a, b), (b, a)] { println("${x}-${y}") }
+        a * b
+    }
+    println("${product((3, 4))}")
+}
diff --git a/tests/runtime/patterns.stdout b/tests/runtime/patterns.stdout
new file mode 100644
index 0000000000000000000000000000000000000000..e1fcc144025bcefbaf1b126f8599d9776173fe4c
--- /dev/null
+++ b/tests/runtime/patterns.stdout
@@ -0,0 +1,47 @@
+keyword fn
+ident x
+small
+negative -5
+teen
+number 20
+letter
+letter
+digit
+symbol +
+numbers 5
+pair with right
+pair with left
+pair
+eof
+nothing
+depth 4 sum 15
+depth 2
+origin
+axis
+axis
+diagonal 2
+anti 3
+near
+far
+1 and one
+err 2 20
+err 30
+special
+only plain
+4 5 6
+both
+first x
+name y
+name z
+ping
+stop
+3 2
+7 seven true z
+last z
+apples 3
+2..9
+one 1
+big a tiny b small c none
+3-4
+4-3
+12
diff --git a/tests/runtime/patterns_async.nova b/tests/runtime/patterns_async.nova
new file mode 100644
index 0000000000000000000000000000000000000000..888f9e781dc50ca84eaf15c11a355f5599d137f6
--- /dev/null
+++ b/tests/runtime/patterns_async.nova
@@ -0,0 +1,89 @@
+// Nested matches, guards that `.await` and fall through, and destructured
+// tuples, in `async fn`s, across `.await`s and through many allocations
+// (5.0 spec §4.3, §4.6, §7.3): a test runs it under `NOVA_GC_STRESS=1`
+// too, so a projection or a binding the collector failed to see would be
+// read after it was freed.
+
+type Expr = | Num(Int) | Name(String) | Add(Expr, Expr) | Neg(Expr)
+
+async fn lookup(name: String) -> Option<Int> {
+    yield_now().await
+    match name {
+        "x" => Some(10)
+        "y" => Some(0 - 4)
+        _ => None
+    }
+}
+
+async fn is_big(n: Int) -> Bool {
+    yield_now().await
+    n > 100
+}
+
+async fn eval(e: Expr) -> Result<Int, String> {
+    match e {
+        Num(n) if is_big(n).await => Err("too big: ${n}")
+        Num(n) => Ok(n)
+        Name(s) => match lookup(s).await {
+            Some(v) => Ok(v)
+            None => Err("unbound ${s}")
+        }
+        Add(Num(0), rest) | Add(rest, Num(0)) => eval(rest).await
+        Add(a, b) => {
+            let l = eval(a).await
+            let r = eval(b).await
+            match (l, r) {
+                (Ok(x), Ok(y)) => Ok(x + y)
+                (Err(m), _) | (_, Err(m)) => Err(m)
+            }
+        }
+        Neg(Neg(inner)) => eval(inner).await
+        Neg(inner) => match eval(inner).await {
+            Ok(v) => Ok(0 - v)
+            Err(m) => Err(m)
+        }
+    }
+}
+
+async fn churn(label: String) -> (String, Int) {
+    let mut total = 0
+    let mut i = 0
+    while i < 40 {
+        let s = "${label} ${i}"
+        total += s.len()
+        i += 1
+    }
+    yield_now().await;
+    (label, total)
+}
+
+async fn run() -> Int {
+    let exprs = [
+        Num(5),
+        Num(500),
+        Name("x"),
+        Name("zz"),
+        Add(Num(0), Name("y")),
+        Add(Name("x"), Num(0)),
+        Add(Num(2), Add(Num(3), Name("x"))),
+        Add(Num(1), Name("nope")),
+        Neg(Neg(Num(7))),
+        Neg(Add(Num(1), Num(2))),
+    ]
+    let mut ok = 0
+    for e in exprs {
+        match eval(e).await {
+            Ok(v) => {
+                ok += 1
+                println("ok ${v}")
+            }
+            Err(m) => println("error ${m}")
+        }
+    }
+    let (label, total) = churn("word").await
+    let ((a, b), c) = ((churn("left").await, churn("right").await), "end")
+    println("${label} ${total} ${a.0} ${a.1} ${b.0} ${b.1} ${c}")
+    ok
+}
+
+fn main() { println("done ${block_on(run())}") }
diff --git a/tests/runtime/patterns_async.stdout b/tests/runtime/patterns_async.stdout
new file mode 100644
index 0000000000000000000000000000000000000000..6774754ac6c9c2d16581d1fdb5c617945d36a148
--- /dev/null
+++ b/tests/runtime/patterns_async.stdout
@@ -0,0 +1,12 @@
+ok 5
+error too big: 500
+ok 10
+error unbound zz
+ok -4
+ok 10
+ok 15
+error unbound nope
+ok 7
+ok -3
+word 270 left 270 right 310 end
+done 7
````

**Patch `t5-code`:**

<!-- patch t5-code -->

````diff
diff --git a/crates/nova-mir/src/lower.rs b/crates/nova-mir/src/lower.rs
index 3f97162c1af7f77dbf4485f45856bb8d895f991b..29236c1ae8d39549128becdf092a02f524519602 100644
--- a/crates/nova-mir/src/lower.rs
+++ b/crates/nova-mir/src/lower.rs
@@ -1279,25 +1279,14 @@ impl<'a> Lowerer<'a> {
         let join = self.new_block();
 
         // A match with a guard or a pattern today's flat lowering does not
-        // take is refused until test-chain lowering lands (5.0 spec §4.6),
-        // rather than lowered wrongly.
+        // take lowers wholly by test chains (5.0 spec §4.6); every other
+        // match lowers exactly as before 5.0b.
         if !arms
             .iter()
             .all(|a| a.guard.is_none() && a.pattern.is_flat())
         {
-            self.diagnostics.push(
-                Diagnostic::error(
-                    "E0900",
-                    "matches with nested, tuple, or- or range patterns or guards are not \
-                     supported yet",
-                )
-                .with_primary_label(e.span, "not lowered yet")
-                .with_note(
-                    "the checker accepts this match; lowering it to MIR by test chains \
-                     arrives in a later commit of 5.0b"
-                        .to_string(),
-                ),
-            );
+            self.lower_chain_arms(s, &scrutinee.ty, arms, result, join);
+            self.switch_to(join);
             return result;
         }
 
@@ -1470,6 +1459,208 @@ impl<'a> Lowerer<'a> {
             self.copy(result, v);
         }
     }
+
+    /// Lower a `match` by test chains (5.0 spec §4.6). The scrutinee is
+    /// already in `scrut`, evaluated once. Each arm, in order, tests its
+    /// pattern left to right and depth first; a failed test jumps to the next
+    /// arm's chain. After the tests the arm's guard runs, if it has one, and
+    /// a false guard jumps to the next arm too. After the last arm comes a
+    /// trap, which exhaustiveness makes unreachable.
+    fn lower_chain_arms(
+        &mut self,
+        scrut: Temp,
+        ty: &Ty,
+        arms: &[hir::Arm],
+        result: Temp,
+        join: BlockId,
+    ) {
+        for arm in arms {
+            let next = self.new_block();
+            self.lower_pattern_test(&arm.pattern, scrut, ty, next);
+            if let Some(guard) = &arm.guard {
+                let g = self.lower_expr(guard);
+                if !diverges(guard) {
+                    let body = self.new_block();
+                    self.terminate(Terminator::Branch {
+                        cond: g,
+                        then_: body,
+                        else_: next,
+                    });
+                    self.switch_to(body);
+                }
+            }
+            let v = self.lower_expr(&arm.body);
+            if !diverges(&arm.body) {
+                self.copy(result, v);
+            }
+            self.terminate(Terminator::Goto(join));
+            self.switch_to(next);
+        }
+        // Exhaustiveness was checked: no value gets here.
+        self.terminate(Terminator::Trap);
+    }
+
+    /// Test `value`, of type `ty`, against `pat` in the current block, and
+    /// jump to `fail` when it does not match; on a match, the current block
+    /// is where control goes on. Each sub-value is projected once, into a
+    /// temporary, just before its own tests. A binding is a copy, written as
+    /// the chain reaches it: one a later test fails after is dead, since
+    /// only the arm whose pattern matched reads its locals.
+    ///
+    /// An or-pattern tests its alternatives in turn on the same value; each
+    /// that matches binds the same locals and jumps to one block, where the
+    /// tests after the or-pattern go on, so nothing after it is duplicated.
+    fn lower_pattern_test(&mut self, pat: &hir::Pattern, value: Temp, ty: &Ty, fail: BlockId) {
+        match pat {
+            hir::Pattern::Wildcard => {}
+            hir::Pattern::Bind(local) => {
+                let dst = self.local_map[local.0 as usize];
+                self.copy(dst, value);
+            }
+            hir::Pattern::LitInt(n) => self.test_int_eq(value, *n, fail),
+            hir::Pattern::LitChar(c) => self.test_int_eq(value, *c as i64, fail),
+            hir::Pattern::LitBool(b) => {
+                let ok = self.new_block();
+                let (then_, else_) = if *b { (ok, fail) } else { (fail, ok) };
+                self.terminate(Terminator::Branch {
+                    cond: value,
+                    then_,
+                    else_,
+                });
+                self.switch_to(ok);
+            }
+            hir::Pattern::LitStr(lit) => {
+                let lit_t = self.new_temp(MirTy::Ptr);
+                self.push(Stmt::ConstStr(lit_t, lit.clone()));
+                let eq = self.new_temp(MirTy::I8);
+                self.push(Stmt::CallRuntime {
+                    dst: Some(eq),
+                    func: RtFunc::StrEq,
+                    args: vec![value, lit_t],
+                });
+                self.branch_or_fail(eq, fail);
+            }
+            hir::Pattern::Range { lo, hi } => {
+                self.test_int_cmp(value, hir::BinOp::Ge, *lo, fail);
+                self.test_int_cmp(value, hir::BinOp::Le, *hi, fail);
+            }
+            hir::Pattern::Variant {
+                sum,
+                variant,
+                fields,
+            } => {
+                // A sum of one variant needs no tag test, so an irrefutable
+                // `let` or `for` pattern lowers to projections alone.
+                let def = self.module.sum(*sum);
+                if def.map_or(true, |d| d.variants.len() > 1) {
+                    let tag = self.new_temp(MirTy::I64);
+                    self.push(Stmt::SumTag {
+                        dst: tag,
+                        sum: value,
+                    });
+                    self.test_int_eq(tag, *variant as i64, fail);
+                }
+                let args = match ty {
+                    Ty::Sum { args, .. } => args.clone(),
+                    _ => Vec::new(),
+                };
+                let field_tys: Vec<Ty> = def
+                    .and_then(|d| d.variants.get(*variant as usize))
+                    .map(|v| v.fields.iter().map(|f| self.concrete(f, &args)).collect())
+                    .unwrap_or_default();
+                for (i, field) in fields.iter().enumerate() {
+                    if matches!(field, hir::Pattern::Wildcard) {
+                        continue;
+                    }
+                    let fty = field_tys.get(i).cloned().unwrap_or(Ty::Error);
+                    let mty = mir_ty(&fty);
+                    let t = if mty == MirTy::Unit {
+                        self.unit_temp()
+                    } else {
+                        let t = self.new_temp(mty);
+                        self.push(Stmt::SumField {
+                            dst: t,
+                            sum: value,
+                            index: i as u32,
+                            ty: mty,
+                        });
+                        t
+                    };
+                    self.lower_pattern_test(field, t, &fty, fail);
+                }
+            }
+            hir::Pattern::Tuple(elems) => {
+                let tys = match ty {
+                    Ty::Tuple(tys) => tys.clone(),
+                    _ => Vec::new(),
+                };
+                for (i, elem) in elems.iter().enumerate() {
+                    if matches!(elem, hir::Pattern::Wildcard) {
+                        continue;
+                    }
+                    let ety = tys.get(i).cloned().unwrap_or(Ty::Error);
+                    let mty = mir_ty(&ety);
+                    let t = if mty == MirTy::Unit {
+                        self.unit_temp()
+                    } else {
+                        let t = self.new_temp(mty);
+                        self.push(Stmt::RecordField {
+                            dst: t,
+                            record: value,
+                            index: i as u32,
+                            ty: mty,
+                        });
+                        t
+                    };
+                    self.lower_pattern_test(elem, t, &ety, fail);
+                }
+            }
+            hir::Pattern::Or(alts) => {
+                let ok = self.new_block();
+                for (k, alt) in alts.iter().enumerate() {
+                    let last = k + 1 == alts.len();
+                    let alt_fail = if last { fail } else { self.new_block() };
+                    self.lower_pattern_test(alt, value, ty, alt_fail);
+                    self.terminate(Terminator::Goto(ok));
+                    if !last {
+                        self.switch_to(alt_fail);
+                    }
+                }
+                self.switch_to(ok);
+            }
+        }
+    }
+
+    /// Go on in a fresh block when `cond` holds; jump to `fail` when not.
+    fn branch_or_fail(&mut self, cond: Temp, fail: BlockId) {
+        let ok = self.new_block();
+        self.terminate(Terminator::Branch {
+            cond,
+            then_: ok,
+            else_: fail,
+        });
+        self.switch_to(ok);
+    }
+
+    /// `value == n`, an `Int`, `Char` or tag test.
+    fn test_int_eq(&mut self, value: Temp, n: i64, fail: BlockId) {
+        self.test_int_cmp(value, hir::BinOp::Eq, n, fail);
+    }
+
+    /// `value op n` on integers, failing to `fail`.
+    fn test_int_cmp(&mut self, value: Temp, op: hir::BinOp, n: i64, fail: BlockId) {
+        let c = self.new_temp(MirTy::I64);
+        self.push(Stmt::ConstInt(c, n));
+        let cond = self.bin_i64(op, value, c, MirTy::I8);
+        self.branch_or_fail(cond, fail);
+    }
+
+    /// A field's declared type at the scrutinee's type arguments, with any
+    /// projection normalized as mono normalizes the function's own types.
+    fn concrete(&self, field: &Ty, args: &[Ty]) -> Ty {
+        let ty = field.subst(args);
+        hir::normalize_ty(&ty, &self.module.impls).unwrap_or(ty)
+    }
 }
 
 fn operand_class(ty: &Ty) -> OperandClass {
diff --git a/std/http/lib.nova b/std/http/lib.nova
index d98f76215f68193715d0b8b4ab5bdd2907de6c09..631a7da6e03707c62f15398d3ac81a971ebbe428 100644
--- a/std/http/lib.nova
+++ b/std/http/lib.nova
@@ -1267,9 +1267,9 @@ pub async fn read_request(conn: TcpStream, limits: Limits) -> Result<Request, Ht
         }
         buf = buf.concat(chunk)
         // One arm, not two: `parse_request_head` already returns
-        // `Option<Request>`, so the loop assigns it straight through. It is
-        // also the form the language permits -- `Ok(Some(r))` is
-        // `E0900: nested patterns inside variants are not supported yet`.
+        // `Option<Request>`, so the loop assigns it straight through. A
+        // nested `Ok(Some(r))` would work since 5.0b, but would need a
+        // second arm for `Ok(None)`.
         match parse_request_head(buf, limits) {
             Ok(maybe) => head = maybe
             Err(e) => return Err(e)
diff --git a/tests/runtime/http_serialise.nova b/tests/runtime/http_serialise.nova
index 8e465f86011b68f89a385b6bb01c4d72e051b9ba..5d75897ea711df8ece7ca2eb960d7cf640eef633 100644
--- a/tests/runtime/http_serialise.nova
+++ b/tests/runtime/http_serialise.nova
@@ -20,9 +20,9 @@
 fn main() {
     let raw = bytes_from_string("GET /hi HTTP/1.1\r\nHost: a.example\r\nContent-Length: 0\r\n\r\n")
     match parse_request_head(raw, Limits::default()) {
-        // Two levels of `match`, not one: `Ok(Some(req))` is
-        // `E0900: nested patterns inside variants are not supported yet`.
-        // Measured on this tree; no std module uses a nested pattern either.
+        // Two levels of `match`, written before 5.0b, when `Ok(Some(req))`
+        // was E0900; nested patterns work now, and this form still does.
+        // No std module uses a nested pattern.
         Ok(maybe) => match maybe {
             Some(req) => {
                 println("path ${req.path}")
````

### Task 6: E0103's ruling

The user's 5.0b ruling of 2026-10-27; decisions 3, 29, 37 and 41.

**Files:**
- Modify:
  - `crates/nova-typeck/src/check.rs` (`quiet_splits`, the statements'
    successors in `check_block`, `judge_split`, `split_operand`,
    `warn_newline_splits` skipping a quiet split)
  - `tests/runtime/tuples_async.nova` (a comment)
- Test: `crates/nova-typeck/src/check/lang_tests.rs` (one test changed,
  eight new)

- [ ] **Step 1: The tests.** `patch_block t6-tests`. The patch holds:
  - `e0103_reports_a_parenthesised_callee_after_a_line_break`: `x` then
    `(g)()` gives one E0103, the unused `x`, since `x` is an `Int`; after
    `let h = g`, `h` then `(g)()` gives two.
  - `a_tuple_tail_after_a_call_that_returns_no_function_is_quiet`: `g()`, an
    `Int`, then `(1, 2)`; and `println("x")`, a unit, then `(1, 2)`.
  - `a_split_warns_only_after_a_value_the_old_reading_could_extend`: `(`
    after a function and not after an `Int` or a `String`; `[` after an
    array and not after an `Int`; `-` after an `Int`, after `x * 2.0` (the
    rightmost operand, a `Float`) and after `n == 1` (its rightmost operand,
    an `Int`), and not after a `String`.
  - `a_split_after_a_bare_return_warns_whatever_comes_before`, and
    `a_split_after_a_bare_return_before_a_tail_warns`, where the line after
    is a block's tail, so no "this value is not used" stands in for the
    split's own warning.
  - `a_split_after_a_value_of_unknown_type_warns`: `let m = make()`, `make`
    generic in its return type, then `(1, 2)`.
  - `the_split_operand_is_read_through_an_assignment`: `q = k()`, `k`
    returning a function, then `(n)` warns; `m = n` then `(n)` does not. The
    last three are decision 41's.
  - `a_split_after_a_compound_assignment_to_an_element_or_a_field_warns`:
    `a[0] -= 2`, `r.x += 2` and `t.1.0 += 2`, each then `- 1`.
  - `a_split_after_a_closure_reads_its_body`: `let f = |x: Int| x` then
    `- 1`, `let f = |i: Int| arr` then `[1][0]`, and `let f = |x: Int| g`
    then `(3)` warn; `let f = |x: Int| "s"` then `- 1` does not; `let h = g`
    then `(3)`, `g` a function, warns, its wrapper not read as a body. The
    last two are the first fact-check's (decision 3).
- [ ] **Step 2: Run, and see the tests fail.** First the build, which ends
  with `Finished`.
  - `T check::lang_tests`. Expected: `nova-typeck`'s `44 passed; 5 failed`,
    every other library's `ok` with none run:
    `a_tuple_tail_after_a_call_that_returns_no_function_is_quiet`
    (``["a line break ends the statement before this `(`"]``),
    `a_split_warns_only_after_a_value_the_old_reading_could_extend`
    (`assertion failed: split("let m = n", "(n)").is_empty()`),
    `e0103_reports_a_parenthesised_callee_after_a_line_break` (`left: 2`,
    `right: 1`), `the_split_operand_is_read_through_an_assignment`
    (`assertion failed: e0103_messages(&src("m = n")).is_empty()`) and
    `a_split_after_a_closure_reads_its_body`
    (`assertion failed: e0103_messages(&src("let f = |x: Int| \"s\"", "- 1")).is_empty()`):
    before the ruling every split warns. The other four new tests pass, as
    they should: each pins a split the ruling leaves loud, among them
    `a_split_after_a_compound_assignment_to_an_element_or_a_field_warns`.
- [ ] **Step 3: The code.** `patch_block t6-code`:
  - **`check.rs`:** the `Checker`'s `quiet_splits`, a set of spans;
    `check_block` gives each statement its successor's start, and after
    checking a `let`'s value or an expression statement calls `judge_split`,
    which finds the split recorded there, its operand by `split_operand`,
    and marks it quiet as decision 3 says; `warn_newline_splits` skips a
    quiet split.
  - **`tuples_async.nova`:** its comment says the `;` before its tail tuple
    is optional since the ruling, and fmt keeps it.
- [ ] **Step 4: Run.** First the build, which ends with `Finished`.
  - `T check::lang_tests`. Expected: `nova-typeck`'s `49 passed; 0 failed`,
    every `test result` `ok`.
  - `S`, as in Task 1 (about 4 min 50 s). Expected: `2705 1 10`: Task 5's
    figures and Task 6's 8 tests, the one failure the IPv6 test.
  - `R tuples_async_runs`. Expected: `1 passed; 0 failed`.
- [ ] **Step 5: The checks,** as the conventions list them.
- [ ] **Step 6: Commit**
  `typeck: E0103 only where the old reading could have been well typed`.

**Patch `t6-tests`:**

<!-- patch t6-tests -->

````diff
diff --git a/crates/nova-typeck/src/check/lang_tests.rs b/crates/nova-typeck/src/check/lang_tests.rs
index 75ed86579992b28627af1035df4e9c3bf3bdbd27..b2c9402d364cbaa59192aa9fa23fa67d421541fc 100644
--- a/crates/nova-typeck/src/check/lang_tests.rs
+++ b/crates/nova-typeck/src/check/lang_tests.rs
@@ -439,10 +439,12 @@ fn e0103_is_not_on_calls_assignments_conditions_or_a_tail() {
 
 #[test]
 fn e0103_reports_a_parenthesised_callee_after_a_line_break() {
-    // `x` then `(g)()`: `x` is an unused name, and the line break that ends
-    // it before `(g)()` is reported too, though `unused_value` cannot tell
-    // the call was written in parentheses.
-    assert_eq!(e0103("    x\n    (g)()"), 2);
+    // `x` then `(g)()`: `x` is an unused name. The line break that ends it
+    // before `(g)()` is not reported: `x` is an `Int`, so the old reading,
+    // `x((g))()`, could not have been well typed (the user's 5.0b ruling).
+    assert_eq!(e0103("    x\n    (g)()"), 1);
+    // After a function it could, so the split is reported too.
+    assert_eq!(e0103("    let h = g\n    h\n    (g)()"), 2);
 }
 
 // --- E0103 at the newline rule's splits (the user's ruling of 2026-10-27) ---
@@ -542,3 +544,135 @@ fn an_unused_value_at_a_split_gets_one_warning_with_the_line_breaks_note() {
     assert_eq!(all.len(), 1, "{all:?}");
     assert!(all[0].notes.is_empty(), "{all:?}");
 }
+
+// --- E0103 only where the old reading could have been well typed (the
+// user's 5.0b ruling of 2026-10-27) ---
+
+#[test]
+fn a_tuple_tail_after_a_call_that_returns_no_function_is_quiet() {
+    let all = e0103_messages(
+        "fn g() -> Int { 1 }\nfn f() -> (Int, Int) {\n    g()\n    (1, 2)\n}\nfn main() {}\n",
+    );
+    assert!(all.is_empty(), "{all:?}");
+    // A unit statement, as an `.await` or a `println` is.
+    let all =
+        e0103_messages("fn f() -> (Int, Int) {\n    println(\"x\")\n    (1, 2)\n}\nfn main() {}\n");
+    assert!(all.is_empty(), "{all:?}");
+}
+
+#[test]
+fn a_split_warns_only_after_a_value_the_old_reading_could_extend() {
+    let split = |prev: &str, line: &str| -> Vec<String> {
+        e0103_messages(&format!(
+            "fn k() -> fn(Int) -> Int {{ |y: Int| y }}\n\
+             fn f(n: Int, s: String, a: [Int], x: Float) -> Int {{\n    {prev}\n    {line}\n}}\nfn main() {{}}\n"
+        ))
+    };
+    let call = "a line break ends the statement before this `(`";
+    let index = "a line break ends the statement before this `[`";
+    let minus = "a line break ends the statement before this `-`";
+    // `(` after a function, and not after an `Int` or a `String`.
+    assert_eq!(split("let h = k()", "(n)"), [call]);
+    assert!(split("let m = n", "(n)").is_empty());
+    assert!(split("let t = s", "(n)").is_empty());
+    // `[` after an array, and not after an `Int`.
+    assert_eq!(split("let b = a", "[n][0]"), [index]);
+    assert!(split("let m = n", "[n][0]").is_empty());
+    // `-` after an `Int` or a `Float`, and not after a `String`; the
+    // operand is the rightmost one of a binary operator.
+    assert_eq!(split("let m = n", "-n"), [minus]);
+    assert_eq!(split("let y = x * 2.0", "-n"), [minus]);
+    assert_eq!(split("let b = n == 1", "-n"), [minus]);
+    assert!(split("let t = s", "-n").is_empty());
+}
+
+#[test]
+fn a_split_after_a_bare_return_warns_whatever_comes_before() {
+    let all =
+        e0103_messages("fn g() -> Int { 1 }\nfn f() {\n    return\n    (g())\n}\nfn main() {}\n");
+    assert!(
+        all.iter()
+            .any(|m| m == "this value is not used"
+                || m == "a line break ends this `return` or `break`"),
+        "{all:?}"
+    );
+}
+
+#[test]
+fn a_split_after_a_value_of_unknown_type_warns() {
+    // `m`'s type is not known when the split is judged, so the old reading,
+    // `make()(1, 2)`, could have been well typed.
+    let all = e0103_messages(
+        "fn make<T>() -> T {\n    panic(\"no\")\n}\n\
+         fn f() -> (Int, Int) {\n    let m = make()\n    (1, 2)\n}\nfn main() {}\n",
+    );
+    assert_eq!(all, ["a line break ends the statement before this `(`"]);
+}
+
+#[test]
+fn a_split_after_a_bare_return_before_a_tail_warns() {
+    // No "this value is not used" here: `(g())` is the block's tail.
+    let all = e0103_messages(
+        "fn g() {}\nfn f(c: Bool) {\n    if c {\n        return\n        (g())\n    }\n}\nfn main() {}\n",
+    );
+    assert_eq!(all, ["a line break ends this `return` or `break`"]);
+}
+
+#[test]
+fn the_split_operand_is_read_through_an_assignment() {
+    // `q = k()` then the tail `(n)`: the old reading, `q = k()(n)`, calls a
+    // function, so the split warns; after `m = n`, an `Int`, it does not.
+    let src = |prev: &str| {
+        format!(
+            "fn k() -> fn(Int) -> Int {{\n    |y: Int| y\n}}\n\
+             fn f(n: Int) -> Int {{\n    let mut q = k()\n    let mut m = 0\n    {prev}\n    (n)\n}}\nfn main() {{}}\n"
+        )
+    };
+    assert_eq!(
+        e0103_messages(&src("q = k()")),
+        ["a line break ends the statement before this `(`"]
+    );
+    assert!(e0103_messages(&src("m = n")).is_empty());
+}
+
+#[test]
+fn a_split_after_a_compound_assignment_to_an_element_or_a_field_warns() {
+    // `a[0] -= 2` then `- 1`: the old reading subtracted 1 from the 2.
+    let minus = "a line break ends the statement before this `-`";
+    let src = |stmt: &str| {
+        format!(
+            "record R {{ x: Int }}\nfn f(mut a: [Int], mut r: R, mut t: (Int, (Int, Int))) -> Int {{\n    \
+             {stmt}\n    - 1\n}}\nfn main() {{}}\n"
+        )
+    };
+    for stmt in ["a[0] -= 2", "r.x += 2", "t.1.0 += 2"] {
+        assert_eq!(e0103_messages(&src(stmt)), [minus], "{stmt}");
+    }
+}
+
+#[test]
+fn a_split_after_a_closure_reads_its_body() {
+    // The old reading extended the closure's body: `|x: Int| x - 1`,
+    // `|i: Int| arr[1]`, `|x: Int| g(3)`.
+    let src = |prev: &str, line: &str| {
+        format!(
+            "fn g(x: Int) -> Int {{ x }}\nfn f(arr: [Int]) -> Int {{\n    {prev}\n    {line}\n}}\nfn main() {{}}\n"
+        )
+    };
+    let warned = |prev: &str, line: &str, what: &str| {
+        assert_eq!(
+            e0103_messages(&src(prev, line)),
+            [format!(
+                "a line break ends the statement before this `{what}`"
+            )],
+            "{prev} / {line}"
+        );
+    };
+    warned("let f = |x: Int| x", "- 1", "-");
+    warned("let f = |i: Int| arr", "[1][0]", "[");
+    warned("let f = |x: Int| g", "(3)", "(");
+    // A body the old reading could not extend is quiet.
+    assert!(e0103_messages(&src("let f = |x: Int| \"s\"", "- 1")).is_empty());
+    // A bare function as a value is a function, not a body to read.
+    warned("let h = g", "(3)", "(");
+}
````

**Patch `t6-code`:**

<!-- patch t6-code -->

````diff
diff --git a/crates/nova-typeck/src/check.rs b/crates/nova-typeck/src/check.rs
index 8560a8641b28e283f356c4e605d613dccc6ade25..93679ca98149424e9b0e5cb581eb3ad30ea328de 100644
--- a/crates/nova-typeck/src/check.rs
+++ b/crates/nova-typeck/src/check.rs
@@ -167,6 +167,7 @@ pub fn check_with(file: &ast::File, defs: &Definitions, options: &CheckOptions)
         bindings: FxHashMap::default(),
         qualified_callee: None,
         pats: PatState::default(),
+        quiet_splits: FxHashSet::default(),
     };
     // Before any collection pass: every later pass builds a generic scope, and
     // one containing `Self` would give the name two meanings at once.
@@ -336,6 +337,10 @@ struct Checker<'a> {
     /// The pattern being checked: its or-patterns and `let mut` (5.0 spec
     /// §4.3).
     pats: PatState,
+    /// The newline rule's splits after which the old reading could not have
+    /// been well typed, so E0103 is not reported there (the user's 5.0b
+    /// ruling of 2026-10-27), by the split's span.
+    quiet_splits: FxHashSet<Span>,
 }
 
 /// What the probe met, before its function's inference is finished.
@@ -3307,7 +3312,17 @@ impl<'a> Checker<'a> {
     fn check_block(&mut self, fcx: &mut FnCtx, block: &ast::Block, span: Span) -> hir::Expr {
         fcx.scopes.push(FxHashMap::default());
         let mut stmts = Vec::new();
-        for stmt in &block.stmts {
+        // Where each statement's successor starts: a newline-rule split there
+        // follows this statement (the user's 5.0b ruling of 2026-10-27).
+        let nexts: Vec<Span> = block
+            .stmts
+            .iter()
+            .skip(1)
+            .map(|s| s.span)
+            .chain(block.trailing.as_ref().map(|t| t.span))
+            .collect();
+        for (i, stmt) in block.stmts.iter().enumerate() {
+            let next = nexts.get(i).copied();
             // Decision 4: the probe, between the block's start and this
             // statement, sees the scope as it is before the statement.
             self.probe_locals(fcx, span.file, span.start, stmt.span.start);
@@ -3323,6 +3338,7 @@ impl<'a> Checker<'a> {
                         continue;
                     };
                     let mut value = self.check_expr(fcx, init);
+                    self.judge_split(fcx, &value, next);
                     if let Some(annot) = ty {
                         let annot_ty =
                             self.convert_ty(annot, &fcx.generics.clone(), &fcx.param_bounds);
@@ -3392,7 +3408,9 @@ impl<'a> Checker<'a> {
                     });
                 }
                 ast::Stmt::Expr(e) => {
-                    stmts.push(self.check_expr(fcx, e));
+                    let checked = self.check_expr(fcx, e);
+                    self.judge_split(fcx, &checked, next);
+                    stmts.push(checked);
                     if let Some(what) = unused_value(e) {
                         self.diagnostics.push(
                             Diagnostic::warning("E0103", "this value is not used")
@@ -4921,6 +4939,9 @@ impl<'a> Checker<'a> {
     /// takes the note instead of a second warning.
     fn warn_newline_splits(&mut self) {
         for split in &self.file.newline_splits {
+            if self.quiet_splits.contains(&split.at) {
+                continue;
+            }
             let note = match split.kind {
                 ast::SplitKind::Value => {
                     "a `return` or `break` at the end of a line takes no value: to return \
@@ -4971,6 +4992,37 @@ impl<'a> Checker<'a> {
         }
     }
 
+    /// Whether the old reading of a newline-rule split after the statement
+    /// whose value is `value` could have been well typed (the user's 5.0b
+    /// ruling of 2026-10-27): a `(` only after a function, a `[` only after
+    /// an array, a `-` only after an `Int` or a `Float`. Where it could not,
+    /// the split is quiet: `g()` then `(a, b)` on the next line is two
+    /// statements, as the author meant, whatever `g` returns but a function.
+    /// A type not yet known could be any, so it warns; a split after a bare
+    /// `return` or `break` always warns. `next` is where the statement after
+    /// this one starts.
+    fn judge_split(&mut self, fcx: &FnCtx, value: &hir::Expr, next: Option<Span>) {
+        let Some(next) = next else { return };
+        let Some(split) = self
+            .file
+            .newline_splits
+            .iter()
+            .find(|s| s.at.file == next.file && s.at.start == next.start)
+        else {
+            return;
+        };
+        let operand = split_operand(value, split.kind, &fcx.pending_closures);
+        let could = match (split.kind, fcx.icx.apply(&operand.ty)) {
+            (ast::SplitKind::Value, _) | (_, Ty::Var(_)) => true,
+            (ast::SplitKind::Call, ty) => matches!(ty, Ty::Fn { .. }),
+            (ast::SplitKind::Index, ty) => matches!(ty, Ty::Array(_)),
+            (ast::SplitKind::Minus, ty) => matches!(ty, Ty::Int | Ty::Float),
+        };
+        if !could {
+            self.quiet_splits.insert(split.at);
+        }
+    }
+
     /// The `DefId` of std/core's sum named `name` (`Option` or `Result`),
     /// whatever the name means in the module being checked (5.0 spec §3.3).
     fn core_sum(&self, name: &str) -> Option<DefId> {
@@ -8786,6 +8838,57 @@ pub(crate) fn builtin_signature(builtin: Builtin) -> (Vec<Ty>, Ty) {
     }
 }
 
+/// The part of a statement's value that the old reading of a split of
+/// `kind` after it would have extended: a postfix `(` or `[` applies to the
+/// rightmost operand, inside a unary operator too; a binary `-` has the
+/// rightmost operand of a binary operator on its left, and a unary
+/// operator's whole expression. A `return`'s value and an assignment's are
+/// read through, a compound assignment to an element or a field through the
+/// block its desugar builds, and a closure through its body, which the old
+/// reading extended; a `let`'s value is passed here on its own. `closures`
+/// are the function's closures lifted so far, where a closure's body is.
+fn split_operand<'e>(
+    e: &'e hir::Expr,
+    kind: ast::SplitKind,
+    closures: &'e [hir::Function],
+) -> &'e hir::Expr {
+    use hir::ExprKind as K;
+    match &e.kind {
+        K::Binary { rhs, .. } | K::LogicalAnd { rhs, .. } | K::LogicalOr { rhs, .. } => {
+            split_operand(rhs, kind, closures)
+        }
+        K::Unary { expr, .. } if kind != ast::SplitKind::Minus => {
+            split_operand(expr, kind, closures)
+        }
+        K::Return(Some(v)) => split_operand(v, kind, closures),
+        K::Assign { value, .. } | K::FieldSet { value, .. } | K::IndexSet { value, .. } => {
+            split_operand(value, kind, closures)
+        }
+        // `a[i] op= v` and `r.f op= v`: `{ let __place = …; …; set }`.
+        K::Block {
+            stmts,
+            trailing: None,
+        } => match stmts.split_last() {
+            Some((set, lets))
+                if matches!(set.kind, K::IndexSet { .. } | K::FieldSet { .. })
+                    && lets.iter().all(|s| matches!(s.kind, K::Let { .. })) =>
+            {
+                split_operand(set, kind, closures)
+            }
+            _ => e,
+        },
+        // A closure the source wrote, not a bare function's wrapper.
+        K::MakeClosure { func, .. } => match closures
+            .iter()
+            .find(|f| f.def_id == *func && f.name.starts_with("closure$"))
+        {
+            Some(f) => split_operand(&f.body, kind, closures),
+            None => e,
+        },
+        _ => e,
+    }
+}
+
 /// A `match` arm's pattern as the exhaustiveness and reachability analysis
 /// reads it.
 struct ArmPat {
@@ -14357,6 +14460,7 @@ mod tests {
             bindings: FxHashMap::default(),
             qualified_callee: None,
             pats: PatState::default(),
+            quiet_splits: FxHashSet::default(),
         };
         let mut fcx = FnCtx {
             icx: InferCtx::default(),
@@ -14435,6 +14539,7 @@ mod tests {
             bindings: FxHashMap::default(),
             qualified_callee: None,
             pats: PatState::default(),
+            quiet_splits: FxHashSet::default(),
         };
         let mut fcx = FnCtx {
             icx: InferCtx::default(),
diff --git a/tests/runtime/tuples_async.nova b/tests/runtime/tuples_async.nova
index 074738ae546c6df52a275890dd4583b2ffa42f08..75056c341372b9ba880e4318e38bd888c08ed1d0 100644
--- a/tests/runtime/tuples_async.nova
+++ b/tests/runtime/tuples_async.nova
@@ -5,8 +5,9 @@
 
 record Token { text: String, line: Int }
 
-// A tail tuple on the line after a statement: the `;` ends the statement
-// before it, as the newline rule's E0103 asks (5.0 spec §3.5).
+// A tail tuple on the line after a statement. The `;` ends the statement
+// before it; since 5.0b's E0103 ruling it is optional here, as a `()`
+// could not have been called, and fmt keeps it.
 async fn split_word(word: String, line: Int) -> (String, Token) {
     yield_now().await;
     ("<${word}>", Token { text: word, line: line })
````

### Task 7: ADR 0049 and the records

Spec §4.7, §9; decisions 17, 18, 24, 26, 32, 39, 42, 45 and 46.

**Files:**
- Create: `docs/adr/0049-tuples-and-patterns.md`
- Modify: `CHANGELOG.md`, `nova-spec/11-PARSER.md`,
  `nova-spec/12-TYPESYSTEM.md`, `nova-spec/13-RUNTIME.md`,
  `nova-spec/14-CODEGEN.md`, `nova-spec/20-STDLIB.md`

- [ ] **Step 1: The records.** `patch_block t7-code`:
  - **ADR 0049,** tuples and patterns: its context (no tuples and flat
    patterns before 5.0b; what the spec files say more); its decisions,
    tuples as anonymous records with no impls, their layout's departure, the
    pattern forms in and out, ranges not split, lowering by chains, what
    spec §13 left open, and where the build departs (the one-variant `let`,
    the bare variant name by Rust's rule, so a program can run differently,
    and E0104, the user's ruling of 2026-10-28; `(x, x)`; the E0103 ruling,
    its operand read through a compound assignment and into a closure's
    body; no `-` in a pattern; `let mut P`); its consequences, among them
    that exhaustiveness cannot catch the bare name's change behind a
    catch-all, and usefulness's cost in or-patterns, measured; and its
    references.
  - **`CHANGELOG.md`,** Unreleased: Added (tuples, patterns, destructuring
    `let` and `for`, `for` over an array, and E0103's narrower warning) and
    Changed (the bare variant name, which can make a program run
    differently, with `Some(Leaf)` E0016 and the warning E0104; the chains;
    the lexer's split).
  - **Dated notes,** each dated 2026-10-27, or 2026-10-28 for what the
    user's second ruling added, and citing the spec: `11-PARSER.md` §2
    (tuples and `t.N`, the split, the refusals; patterns, `range_pattern`
    defined as the parser reads it); `12-TYPESYSTEM.md` §3 (`Ty::Tuple`, and
    the bound note naming `Debug`) and §8 (the catalog's E0100, E0101 and
    E0104, E0104 exempting a plain `let X` or `for X`, E0022's row covering
    `for`, the patterns note with E0001's, E0016's and E0104's rules and
    notes, the E0103 ruling, and 5.0b's codes); `13-RUNTIME.md` §2.2 (a
    tuple's layout is a record's, its slots scanned conservatively);
    `14-CODEGEN.md` §4 (the same) and §6.2 (chains, not a decision tree);
    and `20-STDLIB.md` (tuples exist; `channel` and `Map` stay as ADR 0017
    left them).
- [ ] **Step 2: Read them against the code.** Each record's claim against
  the tree: the ADR's decisions against Tasks 1 to 6 and this plan's
  decisions; the CHANGELOG's entries against the patches; the catalog's two
  codes' titles against the spec's §6 and the messages in `patterns.rs`;
  every figure against this plan's. Any difference goes in the ledger and to
  Task 8's review, which fixes it in its one commit, so that this task's
  tree stays the replay's.
- [ ] **Step 3: The checks,** as the conventions list them, after the build.
- [ ] **Step 4: Commit**
  `docs: ADR 0049, tuples and patterns, and the notes for 5.0b`.
- [ ] **Step 5: Run,** on the commit.
  - `git diff --check HEAD~1 HEAD`. Expected: nothing.
  - `git diff --stat ebd0ffad HEAD -- . ':!docs/superpowers/plans'`.
    Expected: nothing: but for this plan's file, which the prototype does
    not hold, the tree is the prototype's last, `ebd0ffad`. A difference
    stops the task: record it in the ledger.

**Patch `t7-code`:**

<!-- patch t7-code -->

````diff
diff --git a/CHANGELOG.md b/CHANGELOG.md
index 3257efa655c1d7831c2dc39acbacac9d2cc9ea90..812a8a7167b383ed38bc25b5336961b83d368fd8 100644
--- a/CHANGELOG.md
+++ b/CHANGELOG.md
@@ -11,7 +11,8 @@ Nova uses [Semantic Versioning](https://semver.org/spec/v2.0.0.html).
 
 Phase 5.0a, the small language and CI (spec
 `docs/superpowers/specs/2026-10-26-phase-5-0-the-language-and-std-a-compiler-needs-design.md`
-§3; ADR 0048, Phase 5's scope and backend).
+§3; ADR 0048, Phase 5's scope and backend), and 5.0b, tuples and
+patterns (§4; ADR 0049).
 
 ### Added
 
@@ -46,7 +47,43 @@ Phase 5.0a, the small language and CI (spec
   statement the parser before it would have continued:** a line that
   starts with `(`, `[` or `-`, and a line after a `return` or `break` at
   the end of a line. A `;` written at the end of the line before says the
-  break is meant, and silences it.
+  break is meant, and silences it. Before a `(`, `[` or `-` it warns only
+  where the old reading could have been well typed: after a function, an
+  array, or an `Int` or `Float` operand, so `g()` then a tuple `(a, b)` on
+  the next line does not warn unless `g` returns a function.
+- **Tuples:** tuple types `(Int, String)` and literals `(a, b)`, with a
+  tuple of one written `(a,)`; `()` is still unit and `(a)` still `a`.
+  `t.0` reads an element, counted from 0, `t.1.0` an element of an
+  element, and `t.0 = x` and `t.0 += x` write one through a mutable
+  binding. Tuples work as type arguments, in generic functions, closures,
+  `Option`, `Vec` and arrays, and across `.await`, on every backend, WASM
+  included, laid out as records are. A tuple has no impls, so `==`,
+  interpolation and a `Hash`, `Eq` or `Display` bound refuse it (E0013,
+  with a note that says why), and a tuple is not a `Map` key; no `impl` can
+  name one. A tuple of the wrong arity or element type is E0010, an index
+  past the end E0014, and `t.0` on a value whose type is not yet known
+  E0011. A tuple index is plain decimal digits: `t.01`, `t.1_0` and
+  `t.0x1` are P0001.
+- **Patterns nest, with guards, or-patterns, `Char` literals and ranges:**
+  a variant's fields and a tuple's elements are patterns to any depth,
+  `Some(Token::Ident(name)) if is_keyword(name) =>`, `Some(Int(0) | Int(1))`,
+  `'a'..='z'`, `0..10` and `10..=99`. A guard is a `Bool` that sees the
+  arm's bindings and runs after they bind; when it is false, matching goes
+  on with the next arm. Every alternative of an or-pattern binds the same
+  names at the same types (E0100 otherwise, naming the binding), and a range
+  that matches nothing is E0101. Exhaustiveness and unreachable arms cover
+  all of them: E0020 names a nested witness, `Some(Err(_))`, and E0021 also
+  warns at an or-pattern's alternative that nothing reaches. Literals and
+  ranges need a `_` or a name to be exhaustive; ranges are not split, so an
+  arm that earlier ranges cover is not reported. Record and array patterns,
+  `x @ p` and `Float` literal patterns are still E0900.
+- **Destructuring `let` and `for`:** `let (q, r) = div_mod(a, b)`,
+  `let Wrap(x, y) = w` for a sum with one variant, `let mut (a, b) = p`,
+  and `for (name, n) in pairs`. A pattern that does not match every value
+  is E0022, with the values it misses.
+- **`for x in arr` over an array:** in order, `arr` evaluated once and each
+  element read when the loop reaches it, with `break`, `continue` and a
+  pattern, on every backend, WASM included.
 
 ### Changed
 
@@ -76,6 +113,32 @@ Phase 5.0a, the small language and CI (spec
 - **The lexer marks each token that follows a line break,** a comment
   holding one included, and a lone `\r` counting as one: `lex` returns
   `Tokens`, which reads as a slice of the tokens, and `parse` takes it.
+- **A `match` with a nested, or-, range or tuple pattern, or a guard,
+  lowers to MIR by test chains:** the scrutinee once, then each arm's tests
+  in order, each projection once, a failed test or a false guard jumping to
+  the next arm; or-patterns join where they succeed. A match whose arms are
+  all flat lowers exactly as before, to one `switch`, so no existing
+  program's code changes. No MIR statement is new, so no backend changed.
+  `hir::Pattern` is recursive (`Variant { fields }`, `Tuple`, `Or`, `Range`,
+  `LitChar`) and `hir::Arm` has a `guard`.
+- **A bare name inside a variant pattern that names a variant now matches
+  that variant, as in Rust, and so a program can run differently:** before
+  5.0b a variant's fields could only be names or `_`, so `Some(Red)` bound a
+  local named `Red` and matched every `Some`; now it matches `Some(Red)`
+  alone, so in `match c { Some(Red) => 1, _ => 2 }` a `Some(Green)` now
+  takes the second arm, and exhaustiveness says nothing, since the `_`
+  covers it. A name that is a variant of another type is E0001 there, with
+  a note: to bind a value, use a lower-case name; a bare variant that has
+  fields, `Some(Leaf)` for `Leaf(String)`, is E0016. No repository program
+  wrote such a pattern. A new warning, E0104, marks a pattern binding whose
+  name starts with an upper-case letter, most often a variant misspelt or
+  not in scope (`Some(Rde)`, with "did you mean" for a near variant), or a
+  constant, which a pattern cannot match by name (a guard can); a name that
+  starts with `_` is not warned about.
+- **The lexer splits a float after a `.`:** `t.1.0` is `t`, `.`, `1`, `.`,
+  `0`, by the source text, so `t.1.10` is `(t.1).10`. A float anywhere
+  else, `0..1.5` included, is unchanged. `Tokens::plain_decimal` says
+  whether an integer is written in plain decimal digits.
 - **CI:** the `test` job's Linux leg runs on `ubuntu-24.04`, installs
   Ubuntu's `clang-18` and asserts that it is 18.1.3. With
   `NOVA_REQUIRE_CLANG=1` and `NOVA_CLANG=clang-18`, a new test,
diff --git a/docs/adr/0049-tuples-and-patterns.md b/docs/adr/0049-tuples-and-patterns.md
new file mode 100644
index 0000000000000000000000000000000000000000..6d75e3252fb481b28064cbd3a6376eb8614ff3ca
--- /dev/null
+++ b/docs/adr/0049-tuples-and-patterns.md
@@ -0,0 +1,162 @@
+# ADR 0049 — Tuples and patterns
+
+## Status
+
+Accepted (2026-10-27). Branch `claude/loving-archimedes-frtf5q-50b`
+(`docs/superpowers/specs/2026-10-26-phase-5-0-the-language-and-std-a-compiler-needs-design.md`
+§4 and §4.7). Tuples without impls and the pattern forms are the user's
+decisions 3 and 4 of 2026-10-26; lowering by test chains is the spec's
+recommendation, taken without asking, which the user can overturn (§12
+item 17).
+
+## Context
+
+Phase 5's compiler, N, is written in Nova, and before 5.0b Nova had no
+tuples and only flat patterns: a variant whose fields are names or `_`,
+a name, `_`, and `Int`, `Bool` and `String` literals. Every other form was
+E0900, and `let` took a name or `_`. The spike that ported the lexer to
+Nova wrote an inner `match` for each nested pattern and a mutated position
+for each returned pair (spec §2).
+
+The spec files say more than 5.0b builds: `13-RUNTIME.md` §2.2 lays a
+tuple out without a header when it is on the stack, `14-CODEGEN.md` §4
+flattens a small one, and §6.2 compiles a `match` to a Maranget decision
+tree. `11-PARSER.md` §2's `pattern` includes record, array and `@`
+patterns.
+
+## Decision
+
+1. **Tuples are anonymous records** (decision 3). `(T1, …, Tn)`, for n of
+   one or more, is `Ty::Tuple`; `()` stays unit and `(T)` is `T`. Two tuple
+   types are the same when their elements are. A tuple has no `TyHead`, so
+   no impl names one: no `==`, interpolation, `Eq`, `Hash` or `Display`,
+   and no `Map` key (E0013). `t.N` reads element `N`.
+2. **A tuple's layout is a record's:** MIR's `MakeRecord`, a heap object
+   with a header and one 8-byte slot an element, passed by reference, on
+   every backend, WASM included. This departs from `13-RUNTIME.md` §2.2 and
+   `14-CODEGEN.md` §4 as records already do. No backend sees a tuple.
+3. **The pattern forms in:** `_`, names, `Int`, `Bool`, `String` and `Char`
+   literals, variants by bare or qualified name with fields that are
+   patterns, tuples, or-patterns at any depth, `Int` ranges `lo..hi` and
+   `lo..=hi`, `Char` ranges `lo..=hi`, and guards. `let` and `for` take an
+   irrefutable pattern. **Out, still E0900:** record and array patterns,
+   `x @ p`, and `Float` literals; there is no `is`, `if let` or `while let`
+   (decision 4).
+4. **Usefulness does not split ranges.** A literal and a range are each a
+   constructor of their own, so a match on them needs `_` or a name, and an
+   arm earlier ranges cover is not reported unreachable (§12 item 14).
+5. **A `match` lowers by test chains, not a decision tree.** A match whose
+   arms are all flat, with no guard, lowers exactly as before 5.0b: one
+   `Switch`, or a chain of string compares, byte for byte. Any other match
+   lowers wholly by chains: the scrutinee is evaluated once, and each arm in
+   order tests its pattern left to right and depth first, on values
+   projected once each, binds, runs its guard, and on a failed test or a
+   false guard jumps to the next arm. An or-pattern's alternatives join
+   where they succeed. There is no flat switch before the inner chains.
+   No new MIR statement exists, so Cranelift, LLVM, WASM and the async
+   transform are unchanged.
+6. **What spec §4 left open (its §13), settled:**
+   - **A tuple index is plain decimal digits:** `t.01`, `t.1_0`, `t.0x1`
+     and `t.00` are P0001, so each index has one spelling. A tuple has one
+     element or more, with no upper limit.
+   - **A tuple type mangles as `T<elements>E`,** which is prefix-free, so
+     two distinct tuple types never share an instance's name.
+   - **The HIR pattern is recursive** (`Variant { fields }`, `Tuple`, `Or`,
+     `Range`, `Bind`, the literals, `LitChar` among them), and a chain
+     projects each sub-value once, into a temporary, just before its own
+     tests.
+   - **Ranges are not split** (item 4), and **an unreachable alternative of
+     an or-pattern is reported:** E0021, "unreachable alternative", on the
+     alternative alone, when its arm is reachable and has no guard, checked
+     against the earlier arms and the earlier alternatives of its own arm.
+   - **There is no flat switch before the inner chains** (item 5): MIR's
+     size did not call for one.
+7. **Where the build departs from spec §4, or from 5.0a:**
+   - **A flat `let` of a one-variant sum goes through the flat switch,**
+     `let Wrap(a, b) = w` testing the one tag once, where spec §4.6 says
+     `let` and `for` destructuring lower to projections alone. A
+     destructuring `let` is a one-arm `match`, so it takes the path any
+     match of its shape takes; a nested or tuple `let` lowers by chains,
+     where a sum with one variant gets no tag test.
+   - **A bare name inside a variant pattern that names a variant matches
+     that variant, as in Rust** (the user's ruling of 2026-10-28).
+     `Some(Red)` bound a local named `Red` before 5.0b, when a variant's
+     fields could only be names or `_`, and matched every `Some`; it is now
+     the variant, by the rule a bare name at the top of a pattern always
+     followed. So a program can run differently: in `match c { Some(Red)
+     => 1, _ => 2 }` a `Some(Green)` took the first arm and now takes the
+     second. A name that is a variant of another type is E0001 there, with
+     a note: a name inside a pattern that names a variant matches that
+     variant; to bind a value, use a lower-case name; a bare variant that
+     has fields, `Some(Leaf)`, is E0016. It is the one place 5.0b changes
+     what an existing program means; no repository program wrote such a
+     pattern.
+   - **E0104, a new warning, marks a pattern binding whose name starts with
+     an upper-case letter** (the same ruling): such a name is most often a
+     variant misspelt or not in scope, `Some(Rde)`, and as a binding it
+     matches every value. A constant's name gets its own note, to match it
+     with a guard, and a name near a variant a "did you mean". A name that
+     starts with `_` is not warned about, a leading `_` being the way to say
+     a name is meant, nor a plain `let X` or `for X`, which is not a
+     pattern.
+   - **A name bound twice in one pattern, `(x, x)`, is unchanged from
+     before 5.0b:** both bind, the second shadowing the first, with no
+     error.
+   - **E0103 warns at a newline-rule split only where the old reading
+     could have been well typed** (the user's ruling of 2026-10-27, which
+     narrows the 5.0a plan's decision 37 for 5.0b on): before `(` only
+     when the operand the old reading would have extended is a function,
+     before `[` only when it is an array, before `-` only when it is an
+     `Int` or a `Float`, and wherever its type is not yet known. That
+     operand is the rightmost one, read through a `return`'s value, an
+     assignment's, a compound assignment to an element or a field, and a
+     closure's body, which the old reading extended (`let f = |x: Int| x`
+     then `- 1` read as `|x: Int| x - 1`); a `let`'s value is judged on its
+     own. A split after a bare `return` or `break` warns as before. So
+     `g()` then a tuple `(a, b)` on the next line does not warn unless `g`
+     returns a function.
+   - **No `-` in a pattern:** the parser reads none, as before 5.0b, so a
+     negative literal or range end does not parse (P0001), at the start of
+     an arm as anywhere.
+   - **`let mut P` makes every binding of `P` mutable,** since `mut` does
+     not parse inside a pattern.
+
+## Consequences
+
+- **Chains test more at run time than a tree would:** a tag is loaded and
+  compared once per arm that names it, where a tree tests it once. They
+  duplicate no arm body and no test, so MIR grows with the patterns' size:
+  a 14-arm, three-level match over a token is 190 statements in 67 blocks.
+  A flat switch before the chains, or a tree, can come later without a
+  language change, when N's profile asks for it.
+- **The chains port simply to N in 5.4,** and keep every flat match's MIR,
+  so no golden output or IR test moved when they landed.
+- **A tuple costs an allocation,** as a record does; N returns pairs from
+  hot functions, which a later flattening could serve.
+- **A full-range match needs `_`:** `i64::MIN..=i64::MAX` alone, or every
+  `Char` range, is reported non-exhaustive; overlapping ranges are not
+  reported.
+- **A program with `Some(None)` as a nested pattern** checks, and can run,
+  differently: it matched any `Some` before, and matches `Some(None)` alone
+  now. Exhaustiveness reports the values left uncovered only when no
+  catch-all follows; behind a `_` the other values silently take the later
+  arm. E0104 catches the misspelt case, not this one, whose name is a real
+  variant.
+- **Usefulness is exponential in or-patterns:** they are expanded into
+  their alternatives, so a tuple of sixteen `true | false` alternatives
+  takes 14 s to check in a debug build (twelve 0.6 s, fourteen 2.7 s), the
+  work doubling with each two-way or-pattern. No program of the repository
+  comes near; a lazy expansion can come later.
+- **`Map::entries` and `channel` returning a tuple** stay out of 5.0 (spec
+  §11; ADR 0017).
+
+## References
+
+- `docs/superpowers/specs/2026-10-26-phase-5-0-the-language-and-std-a-compiler-needs-design.md`
+  §4, §6, §10 risks 2 and 3, §11 and §12 items 11 to 17.
+- `nova-spec/11-PARSER.md` §2, `12-TYPESYSTEM.md` §3 and §8,
+  `13-RUNTIME.md` §2.2, `14-CODEGEN.md` §4 and §6.2, and their notes of
+  2026-10-27.
+- `crates/nova-typeck/src/usefulness.rs`, `crates/nova-typeck/src/check/patterns.rs`,
+  `crates/nova-mir/src/lower.rs` (`lower_chain_arms`).
+- ADR 0017, `channel` without tuples; ADR 0048, Phase 5's scope.
diff --git a/nova-spec/11-PARSER.md b/nova-spec/11-PARSER.md
index f38a2fbf9a9715b91e1b3f9f031a394c321628cb..e015d7e47c39ffc0ae75288b807917fe8c5b4dcc 100644
--- a/nova-spec/11-PARSER.md
+++ b/nova-spec/11-PARSER.md
@@ -273,6 +273,61 @@ after an arm's body; a `;` written at the end of the line before says the
 break is meant, so there is no split to record. A view's children keep
 their own rule: a `;` ends a child before a `(` or `[`.
 
+**Amended 2026-10-27 (branch `claude/loving-archimedes-frtf5q-50b`):** **tuples
+and `t.N`**
+(`docs/superpowers/specs/2026-10-26-phase-5-0-the-language-and-std-a-compiler-needs-design.md`
+§4.1, §4.2). Tuple types, literals and patterns parse as the grammar above
+writes them, and as the parser already read them: `()` is unit, `(e)` and
+`(T)` are the expression and the type, and a tuple of one is written with a
+trailing comma, `(e,)` and `(T,)`. A tuple literal is parenthesised, so the
+newline rule is off inside it. **A tuple's element is a postfix `.` and an
+index:**
+
+```ebnf
+postfix_op  = … | "." INT_LIT ;   (* t.0, plain decimal digits *)
+```
+
+The index is plain decimal digits with no leading zero: `t.01`, `t.1_0`,
+`t.0x1` and `t.00` are refused (P0001, "a tuple index is plain decimal
+digits, with no leading zero, `_` or prefix"), as Rust does, so each index
+has one spelling. `t.1e5` is a float there, and P0001 as before. The node is
+`Expr::Field` whose field is the index's digits, so `t.0` is
+`Field { field: "0" }`. **The lexer splits a float after a `.`:** a float
+token whose text is digits, `.`, digits (`1.0`) that follows a `.` becomes an
+integer, a `.` and an integer, so `t.1.0` reads as `(t.1).0` and `t.0.1.2`
+chains. It splits by the source text, not the value, so `t.1.10` is
+`(t.1).10`; after a `..`, or after any other token, `1.0` stays a float, as
+does a float with a `_` or an exponent.
+
+**Amended 2026-10-27 (branch `claude/loving-archimedes-frtf5q-50b`):**
+**patterns**
+(`docs/superpowers/specs/2026-10-26-phase-5-0-the-language-and-std-a-compiler-needs-design.md`
+§4.3 to §4.5). The parser reads patterns as before; 5.0b's checker takes
+more of them. `range_pattern`, which the grammar above leaves undefined, is
+what the parser reads:
+
+```ebnf
+range_pattern = INT_LIT ( ".." | "..=" ) INT_LIT   (* 0..10, 0..=9 *)
+              | CHAR_LIT "..=" CHAR_LIT ;          (* 'a'..='z' *)
+```
+
+An end is a literal, never a path or an expression, and there is no `-` in a
+pattern, so a negative end does not parse. An alternative of an or-pattern
+is a pattern without a top-level `|`; parentheses group one, `Some(1 | 2)`
+needs none, and `(p)` is `p`. A tuple pattern of one is `(p,)`, and `()` is
+the unit pattern. `mut` does not parse inside a pattern; `let mut (a, b) =
+e` makes each binding of the pattern mutable. A guard, `pattern if expr =>`,
+ends at its `=>`, so the newline rule is off in it. **The checker takes**
+`_`, names, `Int`, `Bool`, `String` and `Char` literals, variants with field
+patterns, tuples, or-patterns at any depth, ranges and guards in `match`;
+an irrefutable pattern in `let` and `for`. Record and array patterns,
+`x @ p` and `Float` literals stay E0900. `for p in a` runs over an array
+`a` too, beside a range and an iterator: the checker desugars it to an index
+loop that evaluates `a` once and reads each element when it reaches it, the
+index advanced before the body. A match arm that starts with a
+tuple pattern, after an arm that ended at a line break, begins a new arm by
+the newline rule above.
+
 ---
 
 ## 3. AST Node Definitions (Rust)
diff --git a/nova-spec/12-TYPESYSTEM.md b/nova-spec/12-TYPESYSTEM.md
index d4d3f9cdf0f8af9c83347e0cb0464eba4375c41b..e11a8e1ab4c182c43a3049d81a9dda5e92230865 100644
--- a/nova-spec/12-TYPESYSTEM.md
+++ b/nova-spec/12-TYPESYSTEM.md
@@ -77,6 +77,25 @@ pub enum Ty {
 }
 ```
 
+**Amended 2026-10-27 (branch `claude/loving-archimedes-frtf5q-50b`):**
+`Ty::Tuple(Vec<Ty>)` is built
+(`docs/superpowers/specs/2026-10-26-phase-5-0-the-language-and-std-a-compiler-needs-design.md`
+§4.1), as `nova-hir`'s `Ty::Tuple`, for one element or more: `()` stays
+`Unit`, and `(T)` is `T`. **Tuples are anonymous records** (the user's
+decision 3, 2026-10-26): two tuple types are the same when their element
+types are, element by element and of the same arity; they unify, substitute,
+normalize and match an impl's self type through their elements, and serve as
+type arguments, in generic functions, closures, `Option`, `Vec` and across
+`.await`. **A tuple has no impls:** it has no `TyHead`, so no `impl` can name
+one (E0010, "impl blocks are only supported on named types"), no method
+resolves on one (E0014), and `==`, interpolation and a `Hash`, `Eq` or
+`Display` bound refuse it (E0013, with a note that a tuple has no impls); so
+a tuple is not a `Map` key. `extern "C"` refuses it, as any type but `Int`,
+`Float` and `Bool` (E0900). It displays as written: `(Int, String)`, and
+`(Int,)` for a tuple of one. `t.N` reads element `N`, counted from 0, and
+`t.N = e` and `t.N op= e` write it through a mutable binding, by a record
+field's rule.
+
 ---
 
 ## 4. Inference Algorithm
@@ -238,15 +257,18 @@ types differ.
 | E0015 | Ambiguous method call |
 | E0020 | Non-exhaustive match |
 | E0021 | Unreachable pattern |
-| E0022 | Refutable pattern in `let` |
+| E0022 | Refutable pattern in `let`, or in `for` (`for` added 2026-10-27, Phase 5.0b) |
 | E0030 | Recursive type without indirection |
 | E0031 | Cycle in trait inheritance |
 | E0040 | Cannot return reference to local |
 | E0050 | Async/await mismatch |
 | E0098 | `?` needs an `Option` or a `Result` (added 2026-10-26, Phase 5.0a) |
 | E0099 | `?` cannot return from this function (added 2026-10-26, Phase 5.0a) |
+| E0100 | The alternatives of an or-pattern bind different names, or a name at different types (added 2026-10-27, Phase 5.0b) |
+| E0101 | This range pattern matches nothing (added 2026-10-27, Phase 5.0b) |
 | E0102 | A variant hides one of std's; a warning (added 2026-10-26, Phase 5.0a) |
 | E0103 | This value is not used, or a line break ended a statement; a warning (added 2026-10-26, Phase 5.0a) |
+| E0104 | A pattern binds a name that starts with an upper-case letter, not a plain `let X` or `for X`; a warning (added 2026-10-28, Phase 5.0b) |
 
 Each gets a fully detailed error in `nova-diagnostics::messages` with example, suggestion, and link.
 
@@ -266,8 +288,73 @@ warns at a statement, not a block's tail, whose value can only be discarded:
 a unary or binary operator other than an assignment, `&&` or `||`, a
 literal, a name, or a parenthesised, tuple or array expression; and at each
 place the newline rule (`11-PARSER.md` §2) ends a statement the parser
-before it would have continued, unless a `;` is written there. E0100 and
-E0101 are 5.0b's.
+before it would have continued, unless a `;` is written there. E0100,
+E0101 and E0104 are 5.0b's.
+
+**Amended 2026-10-27 (branch `claude/loving-archimedes-frtf5q-50b`):** tuples
+(`docs/superpowers/specs/2026-10-26-phase-5-0-the-language-and-std-a-compiler-needs-design.md`
+§4.1, §4.2) take no new code. E0010 is a tuple of the wrong arity or
+element type, and an `impl` on a tuple; E0011 an element whose type cannot
+be inferred, and `t.N` on a value whose type is not yet known; E0013 `==`,
+interpolation, or a `Hash`, `Eq`, `Display`, `Debug` or other bound on a
+tuple, with the note that a tuple has no impls; E0014 an index past the
+end ("no field `2` on `(Int, Int)`"), a name on a tuple, `.N` on a type
+that is not a tuple, or a method on a tuple. P0001 is an index that is not plain decimal digits
+(`11-PARSER.md` §2).
+
+**Amended 2026-10-27 (branch `claude/loving-archimedes-frtf5q-50b`):** the
+codes 5.0b's patterns add
+(`docs/superpowers/specs/2026-10-26-phase-5-0-the-language-and-std-a-compiler-needs-design.md`
+§4.3, §4.4, §6), and the codes they reuse. **E0100,** "the alternatives
+bind different names": every alternative of an or-pattern binds the same
+names, which are the same locals, at the same types. The label names the
+binding: "`x` is missing from this alternative", on the alternative that
+lacks it, or "`y` is not bound in the first alternative", on the name; a
+name at two types is E0100 too, "the alternatives bind `x` at different
+types", with both types in the label. **E0101,** "this range matches
+nothing": `lo > hi`, or `lo >= hi` with `..`, in an `Int` or `Char` range.
+**E0010** is a pattern of the wrong type or arity, a tuple pattern of the
+wrong arity ("this pattern has 3 elements but the matched value is a tuple
+of 2"), a range over a type that is not `Int` or `Char`, and a guard that is
+not a `Bool`. **E0020** names nested witnesses (`Some(Err(_))`,
+`(None, false)`). **E0021** is an unreachable arm, as before, and an
+unreachable alternative of an or-pattern in a reachable unguarded arm,
+"unreachable alternative", on the alternative alone: every value it matches
+is matched by an earlier arm or an earlier alternative; one inside an
+alternative already reported is not reported again. **E0022** is a
+refutable pattern in a `let` or a `for`, decided by usefulness, with the
+values it misses ("`None` is not matched"). Usefulness takes a tuple as a
+type with one constructor, expands or-patterns, and takes each `Int`,
+`Char` and `String` literal and each range as a constructor of its own:
+ranges are not split, so a match on them needs `_` or a name, and an arm
+that earlier ranges cover is not reported. **E0001** inside a pattern: a
+bare name that resolves to a variant is that variant at any depth, as at the
+top of a pattern before 5.0b, so `Some(None)` is `Some` of `None`; a name
+that is a variant of another type is E0001, "`Eof` is not a variant of the
+matched type", with the note "a name inside a pattern that names a variant
+matches that variant; to bind a value, use a lower-case name", and a bare
+variant that has fields, `Some(Leaf)`, is E0016. **E0104,** a warning: a
+pattern binding whose name starts with an upper-case letter, "`Rde` binds a
+value: no variant of that name is in scope", with a note that such a name
+reads as a variant and that a lower-case name binds, or, for a constant's
+name, that a pattern cannot match a constant and a guard can, and "did you
+mean `Empty`?" with its fix for a name near a variant. A name that starts
+with `_` is not warned about, nor a plain `let X = …` or `for X in …`,
+which binds a name and not a pattern (the user's ruling of 2026-10-28). An arm whose
+pattern was an error is not read for reachability or exhaustiveness, so no
+second diagnostic follows the first.
+
+**Amended 2026-10-27 (branch `claude/loving-archimedes-frtf5q-50b`):**
+E0103 at a newline-rule split, narrowed by the user's ruling of 2026-10-27
+for 5.0b onward: before a `(`, `[` or `-`, it warns only where the old
+reading could have been well typed, by the type of the operand the old
+reading would have extended (the previous statement's value, its rightmost
+operand of a binary operator, read through a `return`, an assignment, a
+compound assignment to an element or a field, and a closure, whose body the
+old reading extended; a `let`'s value): a function before `(`, an array
+before `[`, an `Int` or a `Float` before `-`, or a type not yet known. So `g()` returning `Int`, then `(a, b)`
+on the next line, does not warn. A split after a bare `return` or `break`
+warns as before.
 
 ---
 
diff --git a/nova-spec/13-RUNTIME.md b/nova-spec/13-RUNTIME.md
index 750bfd66552c792026ceb711e2365f5105656605..73f6cecd41738f5c7a245331227b181a2c80d0f5 100644
--- a/nova-spec/13-RUNTIME.md
+++ b/nova-spec/13-RUNTIME.md
@@ -133,6 +133,17 @@ sweep really frees, so addresses are reused. Nothing may key a persistent table
 - Records = boxed by default (header + fields)
 - Sum types = tagged union: `{ tag: u32, payload: union { ... } }`
 
+**Amended 2026-10-27 (branch `claude/loving-archimedes-frtf5q-50b`):** a tuple is laid
+out as a record is, not as the line above says
+(`docs/superpowers/specs/2026-10-26-phase-5-0-the-language-and-std-a-compiler-needs-design.md`
+§4.1): MIR's `MakeRecord`, a heap object with one 8-byte slot an element,
+read by `RecordField` and written by `SetField`, on every backend, WASM
+included. It has a header like every heap object, on the stack or not, and
+the collector scans its slots conservatively, as it scans a record's: any
+slot that holds an address of a live object keeps that object. No backend
+sees a tuple. This departs from the line above as records already
+do; 5.0b's ADR 0049 records it.
+
 ### 2.3 String Encoding
 - UTF-8, immutable
 - Slicing returns `Str` (view) — no copy
diff --git a/nova-spec/14-CODEGEN.md b/nova-spec/14-CODEGEN.md
index 52d4203347fb60730f620eab2fec911745e90162..adbc0a9dc3dcb96db5d51d5e8227893bbb4c3fca 100644
--- a/nova-spec/14-CODEGEN.md
+++ b/nova-spec/14-CODEGEN.md
@@ -99,6 +99,12 @@ shim provides printing, `Float` text and case mapping.
 | Sum type | tagged: by reference | by reference |
 | Closures | (fn_ptr, env_ptr) | (table_idx, env_ptr) |
 
+**Amended 2026-10-27 (branch `claude/loving-archimedes-frtf5q-50b`):** a tuple is passed
+by reference, a heap pointer, as a record is, on both targets: it is never
+flattened
+(`docs/superpowers/specs/2026-10-26-phase-5-0-the-language-and-std-a-compiler-needs-design.md`
+§4.1; `13-RUNTIME.md` §2.2). 5.0b's ADR 0049 records the departure.
+
 ---
 
 ## 5. Compilation Flow Per Function
@@ -152,6 +158,24 @@ switch i32 %tag, label %default [ i32 0, label %case0
                                   i32 1, label %case1 ]
 ```
 
+**Amended 2026-10-27 (branch `claude/loving-archimedes-frtf5q-50b`):** a
+`match` is lowered to MIR by test chains, not a decision tree
+(`docs/superpowers/specs/2026-10-26-phase-5-0-the-language-and-std-a-compiler-needs-design.md`
+§4.6; ADR 0049). A match whose arms are all flat (`_`, a name, a literal,
+or a variant whose fields are names or `_`) and unguarded lowers as before:
+one `Switch` on the tag or the value (the `switch i64` above, or `i8` for a
+`Bool`), or a chain of string compares. Any other match lowers wholly by
+chains: the scrutinee is evaluated once into a temporary; each arm, in
+order, tests its pattern left to right and depth first, a tag (`SumTag`), an
+`Int`, `Char` or `Bool` value, a string (`StrEq`) or a range (two compares),
+each on a value projected once with `SumField` or `RecordField`; a failed
+test jumps to the next arm's chain. A sum of one variant takes no tag test.
+The arm binds its names as the chain reaches them, then runs its guard,
+whose false edge also goes to the next arm, then its body. An or-pattern's
+alternatives are tested in turn on one value and join where they succeed,
+so nothing after it is duplicated. A trap follows the last arm. No MIR
+statement is new, so the backends emit compares and conditional branches.
+
 ### 6.3 Generics
 Monomorphized: each unique instantiation gets its own compiled function.
 - `vec.push::<Int>(1)` and `vec.push::<String>("hi")` → two distinct functions in object file
diff --git a/nova-spec/20-STDLIB.md b/nova-spec/20-STDLIB.md
index 79d5902cfb676beaba5e13b1d287cc84064f34a2..f7ba5a6246c5900c2db334265716d8c15ef9e3c3 100644
--- a/nova-spec/20-STDLIB.md
+++ b/nova-spec/20-STDLIB.md
@@ -2154,6 +2154,13 @@ around exactly that. So `keys() -> [K]` is a substitute for part of `iter`,
 not a step toward the signature above; enumerating *values* or *pairs*
 still has no API at all.
 
+**Amended 2026-10-27 (branch `claude/loving-archimedes-frtf5q-50b`):** tuple types
+exist now (`docs/superpowers/specs/2026-10-26-phase-5-0-the-language-and-std-a-compiler-needs-design.md`
+§4.1), so the second count no longer holds: `(K, V)` is a type. The first
+does, and `Map` is unchanged in 5.0, which has no `Map::entries` or `iter`
+over tuples (that spec's §11). A tuple has no impls, so no tuple is a `Map`
+key.
+
 Two properties of `keys()` are documented at the method and are
 deliberately **not** guarantees: it returns **table order, not insertion
 order**, and its `[fill; n]` seed is forced rather than stylistic, since
@@ -2353,7 +2360,11 @@ The signature moved instead: `channel` now returns `Channel<T>`, the
 record this section's original code block declares immediately above the
 function and never returns. **The blocker was not lifted; it was routed
 around.** Nothing in this paragraph should be read as saying `channel` is
-unbuilt. The two that do remain are unchanged by that
+unbuilt. (Amended 2026-10-27, branch
+`claude/loving-archimedes-frtf5q-50b`: tuple types exist now, so that
+signature compiles; `channel` stays as ADR 0017 left it, returning
+`Channel<T>`, and reverting it is optional, not owed.) The two that do
+remain are unchanged by that
 increment, and both are `std/task` items rather than `std/sync` ones.
 `spawn_blocking<T>(f: fn() -> T) ->
 JoinHandle<T>` needs a thread pool to run `f` on — `nova-runtime`'s
````

### Task 8: Verification, the mutants, the review, the PR, and the merge

- [ ] **Step 1: The full checks.** First, `df -h /` must show at least 6G
  free; if not, remove `target/debug/incremental` and older duplicate test
  executables.

  ```sh
  cargo build --locked --workspace
  cargo fmt --all -- --check
  cargo clippy --locked --all-targets --all-features -- -D warnings
  target/debug/nova fmt --check std examples tests/wasm tests/dom tests/browser/router-app tests/browser/ssr-app tests/ssr
  NOVA_REQUIRE_NODE=1 cargo test --locked --workspace --no-fail-fast > "$P/full-50b.txt" 2>&1
  grep '^test result' "$P/full-50b.txt" | awk '{p += $4; f += $6; i += $8} END {print p, f, i}'
  grep ' FAILED$' "$P/full-50b.txt"
  cargo test --locked --workspace -- --list > "$P/final-list-50b.txt" 2>&1
  ```

  Expected:
  - both formatters print nothing, and clippy ends with `Finished`, with no
    warning or error;
  - the suite, in 4 min 44 s: `2705 1 10`, the baseline's figures and 171
    more tests;
  - the one failure is Task 0's IPv6 test;
  - the two lists, compared binary by binary by name (a binary is its
    `Running` or `Doc-tests` line, hash stripped, an integration test's
    prefixed with the crate whose unit tests cargo listed just before it, so
    two crates' `tests/tuples.rs` stay apart; a test a line ending
    `: test`), give 2545 tests in 106 binaries before and 2716 in 110 after,
    171 added and 0 gone: `nova-fmt`'s `patterns` (10 tests), `nova-fmt`'s
    `tuples` (7 tests), `nova-mir`'s `pattern_lowering` (10 tests) and
    `nova-parser`'s `tuples` (12 tests) added, and in the binaries that
    change these tests added:

    - `nova: tests/run_tests.rs (run_tests)`, 334 to 340: `for_arrays_run`;
      `match_effects_run`; `patterns_async_runs`; `patterns_run`;
      `tuples_async_runs`; `tuples_run`;
    - `nova: tests/wasm_tests.rs (wasm_tests)`, 234 to 240:
      `gate::for_arrays`; `gate::match_effects`; `gate::patterns`;
      `gate::patterns_async`; `gate::tuples`; `gate::tuples_async`;
    - `nova_codegen_llvm: tests/ir_tests.rs (ir_tests)`, 17 to 18:
      `a_nested_match_emits_chains_and_a_flat_one_still_switches`;
    - `nova_hir (unittests src/lib.rs)`, 23 to 30:
      `tests::a_patterns_bindings_are_each_local_once_in_order`;
      `tests::a_tuple_has_no_head`;
      `tests::match_pattern_matches_a_tuple_element_by_element`;
      `tests::normalize_reaches_a_projection_inside_a_tuple`;
      `tests::only_todays_shapes_are_flat`;
      `tests::overlap_compares_tuples_element_by_element`;
      `tests::subst_and_the_queries_reach_inside_a_tuple`;
    - `nova_lexer: tests/lexer_tests.rs (lexer_tests)`, 42 to 49:
      `a_float_after_a_dot_splits_into_two_indexes`;
      `a_float_elsewhere_is_not_split`;
      `a_float_with_an_underscore_or_an_exponent_after_a_dot_is_left_whole`;
      `a_tuple_index_is_an_integer_after_a_dot`;
      `only_plain_decimal_integers_are_marked_as_tuple_indexes`;
      `the_split_chains`; `the_split_follows_the_source_text_not_the_value`;
    - `nova_lsp (unittests src/lib.rs)`, 26 to 31:
      `completion::tests::a_binding_in_a_nested_pattern_completes`;
      `completion::tests::bindings_in_a_for_pattern_and_an_or_pattern_complete`;
      `hover::tests::a_binding_in_a_nested_pattern_hovers_with_its_type`;
      `hover::tests::a_tuple_local_hovers_with_its_type`;
      `hover::tests::bindings_in_a_for_pattern_and_a_nested_or_pattern_hover`;
    - `nova_mir (unittests src/lib.rs)`, 36 to 38:
      `mono::tests::mangle_ty_gives_distinct_tuple_types_distinct_strings`;
      `mono::tests::mir_ty_maps_a_tuple_to_ptr`;
    - `nova_mir: tests/lower_tests.rs (lower_tests)`, 61 to 67:
      `a_flat_char_match_switches_on_its_code_points`;
      `a_generic_function_at_distinct_tuple_types_is_distinct_instances`;
      `a_tuple_element_write_lowers_to_set_field`;
      `a_tuple_lowers_to_a_record`; `a_tuple_map_key_is_e0013_with_a_note`;
      `a_tuple_of_one_in_a_bound_message_keeps_its_comma`;
    - `nova_typeck (unittests src/lib.rs)`, 451 to 543:
      `check::lang_tests::a_split_after_a_bare_return_before_a_tail_warns`;
      `check::lang_tests::a_split_after_a_bare_return_warns_whatever_comes_before`;
      `check::lang_tests::a_split_after_a_closure_reads_its_body`;
      `check::lang_tests::a_split_after_a_compound_assignment_to_an_element_or_a_field_warns`;
      `check::lang_tests::a_split_after_a_value_of_unknown_type_warns`;
      `check::lang_tests::a_split_warns_only_after_a_value_the_old_reading_could_extend`;
      `check::lang_tests::a_tuple_tail_after_a_call_that_returns_no_function_is_quiet`;
      `check::lang_tests::the_split_operand_is_read_through_an_assignment`;
      `check::pattern_tests::a_binding_at_two_types_is_e0100`;
      `check::pattern_tests::a_char_literal_pattern_checks`;
      `check::pattern_tests::a_char_pattern_against_an_int_is_e0010`;
      `check::pattern_tests::a_destructured_let_binding_is_immutable_without_mut`;
      `check::pattern_tests::a_destructured_let_is_a_one_arm_match`;
      `check::pattern_tests::a_for_loop_variable_over_an_array_is_immutable`;
      `check::pattern_tests::a_for_pattern_with_an_error_is_not_also_e0022`;
      `check::pattern_tests::a_generic_sum_matched_at_depth_is_exhaustive`;
      `check::pattern_tests::a_guarded_arm_covers_nothing`;
      `check::pattern_tests::a_let_pattern_with_an_error_is_not_also_e0022`;
      `check::pattern_tests::a_lower_case_name_inside_a_variant_still_binds`;
      `check::pattern_tests::a_lower_case_or_underscored_binding_or_a_real_variant_is_not_e0104`;
      `check::pattern_tests::a_match_arm_or_a_loop_variable_is_rebound_to_be_mutable`;
      `check::pattern_tests::a_missing_nested_variant_is_e0020_with_a_nested_witness`;
      `check::pattern_tests::a_nested_arm_an_earlier_arm_covers_is_unreachable`;
      `check::pattern_tests::a_one_value_range_is_not_empty`;
      `check::pattern_tests::a_range_over_another_type_is_e0010`;
      `check::pattern_tests::a_refutable_for_pattern_is_e0022`;
      `check::pattern_tests::a_refutable_let_pattern_is_e0022_with_the_values_it_misses`;
      `check::pattern_tests::a_tuple_of_one_witness_keeps_its_comma`;
      `check::pattern_tests::a_tuple_of_sums_missing_a_combination_is_e0020`;
      `check::pattern_tests::a_tuple_pattern_arm_after_a_bare_return_or_an_ordinary_arm_checks`;
      `check::pattern_tests::a_tuple_pattern_of_the_wrong_arity_or_type_is_e0010`;
      `check::pattern_tests::a_tuple_pattern_over_a_range_is_e0010`;
      `check::pattern_tests::a_variant_name_inside_a_variant_matches_that_variant`;
      `check::pattern_tests::a_variant_with_fields_named_bare_inside_a_variant_is_e0016`;
      `check::pattern_tests::a_whole_unreachable_arm_reports_the_arm_not_its_alternatives`;
      `check::pattern_tests::an_alternative_inside_one_reported_is_not_reported_again`;
      `check::pattern_tests::an_alternative_missing_a_binding_is_e0100_naming_it`;
      `check::pattern_tests::an_alternative_with_an_extra_binding_is_e0100_naming_it`;
      `check::pattern_tests::an_empty_range_does_not_make_the_arms_after_it_unreachable`;
      `check::pattern_tests::an_empty_range_is_e0101`;
      `check::pattern_tests::an_erroneous_pattern_does_not_make_the_arms_after_it_unreachable`;
      `check::pattern_tests::an_or_pattern_binds_one_local_in_every_alternative`;
      `check::pattern_tests::an_or_pattern_covers_each_alternative`;
      `check::pattern_tests::an_unreachable_alternative_is_e0021_on_it_alone`;
      `check::pattern_tests::an_upper_case_binding_in_a_for_pattern_is_e0104`;
      `check::pattern_tests::an_upper_case_binding_in_an_or_pattern_warns_once`;
      `check::pattern_tests::an_upper_case_binding_is_e0104`;
      `check::pattern_tests::another_types_variant_name_inside_a_variant_is_e0001_with_a_note`;
      `check::pattern_tests::e0104_near_a_variant_says_which`;
      `check::pattern_tests::e0104_on_a_constant_says_to_use_a_guard`;
      `check::pattern_tests::for_destructures_over_an_array_and_an_iterator`;
      `check::pattern_tests::for_over_an_array_desugars_to_an_index_loop`;
      `check::pattern_tests::guards_see_the_arms_bindings_and_must_be_bool`;
      `check::pattern_tests::let_destructures_a_tuple_and_a_one_variant_sum`;
      `check::pattern_tests::literals_and_ranges_need_a_catch_all`;
      `check::pattern_tests::nested_or_patterns_bind_the_outer_first_alternatives_names`;
      `check::pattern_tests::nested_patterns_check_to_a_recursive_hir_pattern`;
      `check::pattern_tests::the_forms_still_out_are_e0900`;
      `check::pattern_tests::the_same_literal_or_range_again_is_unreachable_but_an_overlap_is_not`;
      `check::tuple_tests::a_method_on_a_tuple_is_e0014`;
      `check::tuple_tests::a_name_on_a_tuple_is_e0014`;
      `check::tuple_tests::a_parenthesised_type_and_expression_are_not_tuples`;
      `check::tuple_tests::a_record_field_may_hold_a_tuple`;
      `check::tuple_tests::a_tuple_displays_as_written`;
      `check::tuple_tests::a_tuple_literal_builds_a_tuple_and_a_field_reads_its_index`;
      `check::tuple_tests::a_tuple_of_one_is_not_its_element`;
      `check::tuple_tests::a_tuple_of_the_wrong_arity_is_e0010`;
      `check::tuple_tests::a_tuple_of_the_wrong_element_type_is_e0010`;
      `check::tuple_tests::an_element_is_written_through_a_mutable_binding`;
      `check::tuple_tests::an_element_whose_type_cannot_be_inferred_is_e0011`;
      `check::tuple_tests::an_index_on_a_type_that_is_not_a_tuple_is_e0014`;
      `check::tuple_tests::an_index_on_a_value_of_unknown_type_is_one_e0011`;
      `check::tuple_tests::an_index_past_the_end_is_e0014`;
      `check::tuple_tests::equality_on_a_record_has_no_tuple_note`;
      `check::tuple_tests::equality_on_tuples_is_e0013_with_a_note`;
      `check::tuple_tests::extern_c_refuses_a_tuple`;
      `check::tuple_tests::interpolating_a_tuple_is_e0013_with_a_note`;
      `check::tuple_tests::no_impl_can_name_a_tuple`;
      `check::tuple_tests::tuple_types_and_literals_check`;
      `check::tuple_tests::tuples_flow_through_closures_and_function_types`;
      `check::tuple_tests::tuples_infer_through_generic_functions`;
      `infer::tests::occurs_check_reaches_inside_a_tuple`;
      `infer::tests::tuples_of_different_arity_or_elements_do_not_unify`;
      `infer::tests::unify_solves_vars_inside_tuples_element_by_element`;
      `usefulness::tests::a_generic_sum_at_depth_takes_its_arguments`;
      `usefulness::tests::a_nested_arm_an_earlier_one_covers_is_not_useful`;
      `usefulness::tests::a_nested_witness_names_the_inner_variant`;
      `usefulness::tests::a_tuple_is_one_constructor_even_when_its_type_is_unknown`;
      `usefulness::tests::a_tuple_of_sums_needs_every_combination`;
      `usefulness::tests::an_or_pattern_row_covers_every_alternative`;
      `usefulness::tests::literals_and_ranges_inside_a_variant_need_a_catch_all`;
      `usefulness::tests::ranges_are_not_split`.
- [ ] **Step 2: CI's Linux leg, as CI runs it.** The `test` job's
  `ubuntu-24.04` leg runs the whole suite with the golden suite required,
  with CI's own flags, `--locked --workspace --all-features --no-fail-fast`:

  ```sh
  NOVA_REQUIRE_CLANG=1 NOVA_CLANG=clang-18 NOVA_REQUIRE_NODE=1 cargo test --locked --workspace --all-features --no-fail-fast > "$P/ci-50b.txt" 2>&1
  grep '^test result' "$P/ci-50b.txt" | awk '{p += $4; f += $6; i += $8} END {print p, f, i}'
  grep ' FAILED$' "$P/ci-50b.txt"
  grep -c 'release_golden_suite ... ok' "$P/ci-50b.txt"
  ```

  Expected, in 5 min 5 s: `2705 1 10`, Step 1's figures, the one
  failure the IPv6 test, and `1`: the golden suite ran and passed inside
  the whole run, as on CI, where Step 1's run skips it.
- [ ] **Step 3: The golden suite.** `time G`. Expected, in 30 s of wall
  time:
  `release golden: 137 passed, 9 excluded, Ubuntu clang version 18.1.3 (1ubuntu1)`
  and `1 passed`: the base's 131 programs and 5.0b's six.
- [ ] **Step 4: The AST over the repository.** For every `.nova` file
  `git ls-tree -r --name-only a3a95a2e` lists,
  `"$P/nova-base-50b" parse --ast <file>` (Task 0's copy) on the file as
  `a3a95a2e` has it, from a `git archive a3a95a2e` unpacked under `$P`, and
  `target/debug/nova parse --ast <file>` on the tree's, each with its exit
  status appended, into two directories under `$P`; then compare each pair
  byte for byte, but for the three files whose comments decision 39
  corrects, `std/http/lib.nova`, `tests/runtime/http_serialise.nova` and
  `tests/runtime/fs_read_dir.nova`, whose pairs are compared with every
  `Span { start, end, file }` replaced by `Span`; and remove the three
  directories. Expected:
  `ast: 279 files, 0 differ (3 compared with spans stripped)`, each of the
  three printed as `same with spans stripped`; the base exits nonzero on 2,
  the files that do not parse, and the tree on the same (decisions 6 and
  39).
- [ ] **Step 5: `nova check` over the repository.** For the same files, the
  base's `nova check <file>` and the tree's, each file not under a package
  alone and each package once from its root, output and exit status
  compared. Expected: `check: 245 checks, 2 differ`, the two parser fixtures
  that use 5.0b's forms: `crates/nova-parser/tests/fixtures/generics.nova`,
  whose E0900 for `for` over an array became the next real error, E0014 "no
  method `push` on array type `[?0]`"; and
  `crates/nova-parser/tests/fixtures/match.nova`, whose E0900s for its guard
  and its range pattern, and the E0021 after them, became E0601, no `main`.
  Every other file checks as at the base. Then the search decision 18 cites,
  since a bare name that becomes a variant checks the same: every line of a
  `.nova` file `a3a95a2e` tracks that holds a match arm (`… =>`, not a
  comment), each pattern's parenthesised variant arguments split at commas,
  and any argument that is one bare name starting with an upper-case letter
  printed. Expected:
  `barename: 1427 arm lines read, 0 bare upper-case names inside a variant`:
  no repository program writes a pattern whose meaning decision 18 changes.
- [ ] **Step 6: Where E0102, E0103 and E0104 fire.**
  `target/debug/nova check` on every `.nova` file `git ls-files` lists that
  is not under a package, and `nova check` from each package's root once,
  counting `warning[E0102]`, `warning[E0103]` and `warning[E0104]`.
  Expected: `fires: 251 checks, E0102 53, E0103 0, E0104 0`: E0104 nowhere,
  so no repository pattern binds an upper-case name; E0103 nowhere, as at
  5.0a, so no repository file has a split; E0102 as at 5.0a, twice in
  `crates/nova-parser/tests/fixtures/sum_type.nova` and 51 times in
  `std/core`, `std/crypto`, `std/http`, `std/io`, `std/json` and `std/log`'s
  `lib.nova` checked as user files, which already fail at the base
  (decisions 3 and 42).
- [ ] **Step 7: fmt and the newline rule on the probes.** For each `.nova`
  file of `$P/plan50b/probes/` (step 0's) and `$P/fc50b/p/` (the first
  fact-check's), a copy in scratch, formatted with `target/debug/nova fmt`:
  it must format, `nova fmt --check` must then pass, and `nova parse --ast`,
  spans stripped, must be the same before and after, `newline_splits`
  included; then `nova check` on the two newline probes,
  `$P/plan50b/probes/p08_newline.nova` and
  `$P/plan50b/probes/p11_tuple_tail_ruling.nova`, and the `-1..=5` probe,
  `$P/plan50b/probes-err/neg_arm.nova`. Expected:
  `fmtprobe: 25 files, 25 ok, 0 with a parse error, 0 bad`: step 0's twelve
  and the fact-check's thirteen, all but two reformatted, each formatting
  again to the same text and parsing to the same tree, the newline rule's
  splits of four files included; and `p08_newline.nova` checks with no
  diagnostic; `p11_tuple_tail_ruling.nova` with three E0103s, before `(`
  after `let f = k()`, before `[` after an array and before `-` after an
  `Int`, and none for `g()` then `(1, 2)`; `neg_arm.nova` with one P0001,
  "expected pattern (in statement), found `-`", at its `-1..=5` arm
  (decisions 29 and 30). Where the probes are missing, say so in the ledger.
- [ ] **Step 8: The lockfile and Rust 1.78.**
  - `git diff --numstat a3a95a2e -- Cargo.lock`. Expected: nothing.
  - Then:

    ```sh
    RUSTFLAGS="-D warnings" CARGO_TARGET_DIR=/dev/shm/msrv-50b cargo +1.78 check --locked --workspace
    rm -rf /dev/shm/msrv-50b
    ```

    Expected: clean, `Finished` in 37.74s. It checks the libraries and
    binaries, not the tests: `assert_cmd` 2.2.1's manifest does not parse
    on 1.78. Its target directory is in `/dev/shm`, memory, so the disk
    stays above 6G.
- [ ] **Step 9: The mutants.** One at a time, each edit is made, its command
  run, and the file restored with `git checkout -- <file>`. Each runs on the
  last commit, but for two, mutants 70 and 71, which break Task 3's refusal
  (decision 35) and so run on Task 3's commit, checked out for them and left
  after.
  - Each edit replaces one exact text, which occurs once in its file; "the
    line goes" removes the whole line holding it.
  - A `cargo test` command rebuilds what it needs, `nova` among it for
    `run_tests`; no mutant touches `nova-runtime`. `T --skip net::…` runs
    every library's tests but the IPv6 one, which fails here whatever the
    mutant (Task 0), so a mutant that passes shows as passing.
  - The replay's run took 49 min 44 s in all; most take under a minute.
  - Each caught mutant must fail as written: the tests named, or the text
    named in the command's output. A mutant that passes is a gap. The
    replay's last run left none: its first run's three gaps, mutants 13 and
    48 and the dead `let` arm, are closed by `8da2b8c9` (decision 41). Any
    mutant that passes is a new gap: record it in the ledger, name it in the
    PR, and give it to the review (Step 12), which says whether a check is
    added.
  - At the end, `git status --short` prints nothing, and
    `cargo build --locked --workspace` rebuilds the last commit's binaries
    (decision 48): the last mutant to run leaves `target/debug` built from
    its edit, and every step after this one runs `target/debug/nova`.

  1. **The lexer never splits a float after a `.`.** `crates/nova-lexer/src/lib.rs`:
     `let after_dot = matches!(out.last(), Some(prev) if prev.value == Token::Dot);` becomes `let after_dot = false;`.
     - Run `cargo test --locked --workspace --test lexer_tests` and `cargo test --locked --workspace --no-fail-fast --test tuples`, each to its end.
     - Expected: fails: `a_float_after_a_dot_splits_into_two_indexes`, `a_tuple_field_is_a_place`, `nested_tuple_fields_read_left_to_right`, `the_split_chains`, `the_split_follows_the_source_text_not_the_value`, `tuple_fields_print_as_written`.
  2. **The lexer splits a float after a `..` too.** `crates/nova-lexer/src/lib.rs`:
     `Some(prev) if prev.value == Token::Dot);` becomes `Some(prev) if matches!(prev.value, Token::Dot | Token::DotDot));`.
     - Run `cargo test --locked --workspace --test lexer_tests` and `cargo test --locked --workspace --no-fail-fast --test tuples`, each to its end.
     - Expected: fails: `a_float_elsewhere_is_not_split`.
  3. **The lexer splits by the float's value, not its text.** `crates/nova-lexer/src/lib.rs`:
     `frac.parse::<i64>(),` becomes `frac.trim_end_matches('0').parse::<i64>(),`.
     - Run `cargo test --locked --workspace --test lexer_tests` and `cargo test --locked --workspace --no-fail-fast --test tuples`, each to its end.
     - Expected: fails: `a_float_after_a_dot_splits_into_two_indexes`, `a_tuple_field_is_a_place`, `nested_tuple_fields_read_left_to_right`, `the_split_follows_the_source_text_not_the_value`, `tuple_fields_print_as_written`.
  4. **`t.01` passes as a tuple index.** `crates/nova-lexer/src/tokens.rs`:
     `&& (text.len() == 1 || text[0] != b'0')`: the line goes.
     - Run `cargo test --locked --workspace --no-fail-fast --test tuples`.
     - Expected: fails: `an_index_that_is_not_plain_decimal_digits_is_refused`.
  5. **The parser takes any integer as a tuple index.** `crates/nova-parser/src/grammar.rs`:
     `if !self.tokens.plain_decimal(self.pos) {` becomes `if false {`.
     - Run `cargo test --locked --workspace --no-fail-fast --test tuples`.
     - Expected: fails: `an_index_that_is_not_plain_decimal_digits_is_refused`.
  6. **Two tuple types of different arities unify.** `crates/nova-typeck/src/infer.rs`:
     replace

     ```
                     t1.len() == t2.len()
                         && t1

     ```

     with

     ```
                     t1

     ```

     - Run `T --skip net::tests::an_ipv6_connect_parks_on_its_first_poll_and_then_establishes`.
     - Expected: fails: `check::tuple_tests::a_tuple_of_the_wrong_arity_is_e0010`, `infer::tests::tuples_of_different_arity_or_elements_do_not_unify`.
  7. **`t.N` reads element 0's type whatever N is.** `crates/nova-typeck/src/check.rs`:
     `return elems.get(index).map(|ty| (index as u32, ty.clone()));` becomes `return elems.first().map(|ty| (index as u32, ty.clone()));`.
     - Run `T --skip net::tests::an_ipv6_connect_parks_on_its_first_poll_and_then_establishes` and `R tuples_run`, each to its end.
     - Expected: fails: `check::lang_tests::a_split_after_a_compound_assignment_to_an_element_or_a_field_warns`, `check::tuple_tests::a_record_field_may_hold_a_tuple`, `check::tuple_tests::a_tuple_literal_builds_a_tuple_and_a_field_reads_its_index`, `check::tuple_tests::an_element_is_written_through_a_mutable_binding`, `check::tuple_tests::an_index_past_the_end_is_e0014`, `check::tuple_tests::tuples_infer_through_generic_functions`, `tuples_run`.
  8. **An index past a tuple's end gets the record wording.** `crates/nova-typeck/src/check.rs`:
     `None if matches!(fcx.icx.apply(recv_ty), Ty::Tuple(_)) => {` becomes `None if false => {`.
     - Run `T --skip net::tests::an_ipv6_connect_parks_on_its_first_poll_and_then_establishes`.
     - Expected: fails: `check::tuple_tests::a_name_on_a_tuple_is_e0014`, `check::tuple_tests::an_element_is_written_through_a_mutable_binding`, `check::tuple_tests::an_index_past_the_end_is_e0014`.
  9. **`.N` on an unknown type leaves the variable unsolved.** `crates/nova-typeck/src/check.rs`:
     replace

     ```
             fcx.icx.unify(recv_ty, &Ty::Error);
             true

     ```

     with

     ```
             true

     ```

     - Run `T --skip net::tests::an_ipv6_connect_parks_on_its_first_poll_and_then_establishes`.
     - Expected: fails: `check::tuple_tests::an_index_on_a_value_of_unknown_type_is_one_e0011`.
  10. **E0013 on a tuple loses its note.** `crates/nova-typeck/src/check.rs`:
      `d.notes.push(hir::TUPLE_HAS_NO_IMPLS.to_string());`: the line goes.
      - Run `T --skip net::tests::an_ipv6_connect_parks_on_its_first_poll_and_then_establishes`.
      - Expected: fails: `check::tuple_tests::equality_on_tuples_is_e0013_with_a_note`, `check::tuple_tests::interpolating_a_tuple_is_e0013_with_a_note`.
  11. **MIR's type of a tuple is unit, not a pointer.** `crates/nova-mir/src/lib.rs`:
      replace

      ```
              | hir::Ty::Tuple(_)
              | hir::Ty::Future(_) => MirTy::Ptr,
      ```

      with

      ```
              | hir::Ty::Future(_) => MirTy::Ptr,
              hir::Ty::Tuple(_) => MirTy::Unit,
      ```

      - Run `T --skip net::tests::an_ipv6_connect_parks_on_its_first_poll_and_then_establishes` and `R tuples_run`, each to its end.
      - Expected: fails: `mono::tests::mir_ty_maps_a_tuple_to_ptr`, `tuples_run`.
  12. **A tuple's mangling loses its `E`.** `crates/nova-mir/src/lib.rs`:
      `format!("T{}E", es.join(""))` becomes `format!("T{}", es.join(""))`.
      - Run `T --skip net::tests::an_ipv6_connect_parks_on_its_first_poll_and_then_establishes` and `cargo test --locked --workspace --test lower_tests`, each to its end.
      - Expected: fails: `mono::tests::mangle_ty_gives_distinct_tuple_types_distinct_strings`.
  13. **Mono names a tuple of one without its comma.** `crates/nova-mir/src/mono.rs`:
      `format!("({},)", names[0])` becomes `format!("({})", names[0])`.
      - Run `T --skip net::tests::an_ipv6_connect_parks_on_its_first_poll_and_then_establishes` and `cargo test --locked --workspace --test lower_tests`, each to its end.
      - Expected: fails: `a_tuple_of_one_in_a_bound_message_keeps_its_comma`.
  14. **A `Char` literal pattern types as an `Int`.** `crates/nova-typeck/src/check.rs`:
      replace

      ```
      self.unify_pattern_ty(fcx, &Ty::Char, scrut_ty, pattern.span);
                          hir::Pattern::LitChar(*c)
      ```

      with

      ```
      self.unify_pattern_ty(fcx, &Ty::Int, scrut_ty, pattern.span);
                          hir::Pattern::LitChar(*c)
      ```

      - Run `T --skip net::tests::an_ipv6_connect_parks_on_its_first_poll_and_then_establishes`.
      - Expected: fails: `check::pattern_tests::a_char_literal_pattern_checks`, `check::pattern_tests::a_char_pattern_against_an_int_is_e0010`.
  15. **A tuple pattern of another arity passes.** `crates/nova-typeck/src/check/patterns.rs`:
      `Ty::Tuple(tys) if tys.len() == elems.len() => Some(tys),` becomes `Ty::Tuple(tys) => Some(tys),`.
      - Run `T --skip net::tests::an_ipv6_connect_parks_on_its_first_poll_and_then_establishes`.
      - Expected: fails: `check::pattern_tests::a_tuple_pattern_of_the_wrong_arity_or_type_is_e0010`.
  16. **A variant's fields are not checked as patterns.** `crates/nova-typeck/src/check.rs`:
      replace

      ```
      self.check_pattern(fcx, field_pat, &bound_ty)
      ```

      with

      ```
      {
                          let _ = (field_pat, bound_ty);
                          hir::Pattern::Wildcard
                      }
      ```

      - Run `T --skip net::tests::an_ipv6_connect_parks_on_its_first_poll_and_then_establishes`.
      - Expected: fails: `async_end_to_end::a_heap_value_allocated_inside_the_poll_fn_round_trips_through_a_state_slot`, `async_end_to_end::a_loop_accumulator_survives_the_spill_across_blocks`, `async_end_to_end::a_unit_output_await_still_polls_its_future_and_continues`, `async_end_to_end::a_unit_returning_async_fn_completes_and_reads_a_valid_output_slot`, `async_end_to_end::an_async_fns_parameters_reach_its_body_through_the_state_object`, `async_end_to_end::an_await_free_async_fn_returning_float_runs_and_produces_its_value`, `async_end_to_end::an_await_free_async_fn_returning_int_runs_and_produces_its_value`, `async_end_to_end::an_await_inside_a_loop_suspends_and_resumes_on_every_iteration`, `async_end_to_end::an_await_inside_a_match_arm_runs_the_arm_it_selected`, `async_end_to_end::an_await_on_a_twice_pending_future_resumes_at_the_same_await`, `async_end_to_end::an_await_on_an_already_ready_future_falls_through_to_its_continuation`, `async_end_to_end::mix64_agrees_with_the_runtime_finalizer_at_a_single_key`, `async_end_to_end::mix64_agrees_with_the_runtime_finalizer_over_keys_of_both_signs`, `async_end_to_end::parameter_slots_are_seeded_in_order`, `async_end_to_end::two_awaits_in_one_body_each_get_their_own_resume_state`, `check::lang_tests::compound_assignment_evaluates_the_index_once`, `check::lang_tests::compound_assignment_on_a_field_evaluates_the_receiver_once`, `check::lang_tests::compound_assignment_on_nested_places_checks`, `check::lang_tests::e0103_is_not_on_calls_assignments_conditions_or_a_tail`, `check::lang_tests::every_compound_operator_on_a_field_and_an_element_checks`, `check::lang_tests::map_err_then_try_checks`, `check::lang_tests::return_in_a_closure_checks_against_the_closures_type`, `check::lang_tests::string_compound_assignment_on_a_field_checks`, `check::lang_tests::string_plus_char_or_int_is_refused_with_a_note`, `check::lang_tests::string_plus_string_is_a_string`, `check::lang_tests::try_desugars_to_a_match_in_hir`, `check::lang_tests::try_gives_the_payload_type`, `check::lang_tests::try_in_a_closure_inside_a_constant_checks`, `check::lang_tests::try_in_a_closure_returns_from_the_closure_not_the_function`, `check::lang_tests::try_in_an_async_fn_uses_its_declared_return_type`, `check::lang_tests::try_infers_a_closures_return_type`, `check::lang_tests::try_on_option_in_an_option_function_checks`, `check::lang_tests::try_on_result_rewraps_a_different_ok_type`, `check::lang_tests::try_with_different_error_types_is_e0010_pointing_at_map_err`, `check::pattern_tests::a_binding_at_two_types_is_e0100`, `check::pattern_tests::a_char_literal_pattern_checks`, `check::pattern_tests::a_char_pattern_against_an_int_is_e0010`, `check::pattern_tests::a_destructured_let_binding_is_immutable_without_mut`, `check::pattern_tests::a_destructured_let_is_a_one_arm_match`, `check::pattern_tests::a_for_loop_variable_over_an_array_is_immutable`, `check::pattern_tests::a_for_pattern_with_an_error_is_not_also_e0022`, `check::pattern_tests::a_generic_sum_matched_at_depth_is_exhaustive`, `check::pattern_tests::a_guarded_arm_covers_nothing`, `check::pattern_tests::a_let_pattern_with_an_error_is_not_also_e0022`, `check::pattern_tests::a_lower_case_name_inside_a_variant_still_binds`, `check::pattern_tests::a_lower_case_or_underscored_binding_or_a_real_variant_is_not_e0104`, `check::pattern_tests::a_match_arm_or_a_loop_variable_is_rebound_to_be_mutable`, `check::pattern_tests::a_missing_nested_variant_is_e0020_with_a_nested_witness`, `check::pattern_tests::a_nested_arm_an_earlier_arm_covers_is_unreachable`, `check::pattern_tests::a_one_value_range_is_not_empty`, `check::pattern_tests::a_range_over_another_type_is_e0010`, `check::pattern_tests::a_refutable_for_pattern_is_e0022`, `check::pattern_tests::a_refutable_let_pattern_is_e0022_with_the_values_it_misses`, `check::pattern_tests::a_tuple_of_one_witness_keeps_its_comma`, `check::pattern_tests::a_tuple_of_sums_missing_a_combination_is_e0020`, `check::pattern_tests::a_tuple_pattern_arm_after_a_bare_return_or_an_ordinary_arm_checks`, `check::pattern_tests::a_tuple_pattern_of_the_wrong_arity_or_type_is_e0010`, `check::pattern_tests::a_tuple_pattern_over_a_range_is_e0010`, `check::pattern_tests::a_variant_name_inside_a_variant_matches_that_variant`, `check::pattern_tests::a_variant_with_fields_named_bare_inside_a_variant_is_e0016`, `check::pattern_tests::a_whole_unreachable_arm_reports_the_arm_not_its_alternatives`, `check::pattern_tests::an_alternative_inside_one_reported_is_not_reported_again`, `check::pattern_tests::an_alternative_missing_a_binding_is_e0100_naming_it`, `check::pattern_tests::an_alternative_with_an_extra_binding_is_e0100_naming_it`, `check::pattern_tests::an_empty_range_does_not_make_the_arms_after_it_unreachable`, `check::pattern_tests::an_empty_range_is_e0101`, `check::pattern_tests::an_erroneous_pattern_does_not_make_the_arms_after_it_unreachable`, `check::pattern_tests::an_or_pattern_binds_one_local_in_every_alternative`, `check::pattern_tests::an_or_pattern_covers_each_alternative`, `check::pattern_tests::an_unreachable_alternative_is_e0021_on_it_alone`, `check::pattern_tests::an_upper_case_binding_in_a_for_pattern_is_e0104`, `check::pattern_tests::an_upper_case_binding_in_an_or_pattern_warns_once`, `check::pattern_tests::an_upper_case_binding_is_e0104`, `check::pattern_tests::another_types_variant_name_inside_a_variant_is_e0001_with_a_note`, `check::pattern_tests::e0104_near_a_variant_says_which`, `check::pattern_tests::e0104_on_a_constant_says_to_use_a_guard`, `check::pattern_tests::for_destructures_over_an_array_and_an_iterator`, `check::pattern_tests::for_over_an_array_desugars_to_an_index_loop`, `check::pattern_tests::guards_see_the_arms_bindings_and_must_be_bool`, `check::pattern_tests::let_destructures_a_tuple_and_a_one_variant_sum`, `check::pattern_tests::literals_and_ranges_need_a_catch_all`, `check::pattern_tests::nested_or_patterns_bind_the_outer_first_alternatives_names`, `check::pattern_tests::nested_patterns_check_to_a_recursive_hir_pattern`, `check::pattern_tests::the_same_literal_or_range_again_is_unreachable_but_an_overlap_is_not`, `check::tests::a_binding_chain_longer_than_the_depth_limit_is_a_diagnostic`, `check::tests::a_binding_chain_that_resolves_to_an_enormous_type_is_a_diagnostic`, `check::tests::a_binding_projecting_onto_an_impl_parameter_is_not_a_cycle`, `check::tests::a_chain_of_associated_type_bindings_is_accepted`, `check::tests::a_chain_of_associated_type_bindings_normalizes_all_the_way`, `check::tests::a_child_or_text_of_another_type_is_e0010`, `check::tests::a_cyclic_associated_type_binding_nested_in_a_compound_type_is_rejected`, `check::tests::a_cyclic_associated_type_binding_nested_in_a_future_is_rejected`, `check::tests::a_default_methods_own_signature_resolves_its_own_traits_self_item`, `check::tests::a_for_loop_iterates_an_iterator`, `check::tests::a_for_loop_over_a_range_still_works`, `check::tests::a_for_loop_over_an_iterator_breaks_on_none`, `check::tests::a_for_loops_hidden_iterator_local_is_not_reachable_from_the_body`, `check::tests::a_generic_parameter_named_for_a_builtin_still_works`, `check::tests::a_genuinely_wrong_impl_signature_still_reports_e0072`, `check::tests::a_method_body_checks_against_its_normalized_return_type`, `check::tests::a_missing_member_name_adds_no_type_error`, `check::tests::a_parameter_named_self_is_rejected_but_still_scoped`, `check::tests::a_plain_self_trait_method_is_unaffected_by_the_mut_self_trait_rule`, `check::tests::a_projection_in_a_generic_functions_signature_normalizes_at_the_call`, `check::tests::a_projection_in_a_let_annotation_normalizes`, `check::tests::a_projection_in_a_trait_method_parameter_normalizes_at_the_call`, `check::tests::a_projection_in_an_impl_method_parameter_normalizes_in_the_body`, `check::tests::a_projection_in_an_impls_self_type_is_rejected`, `check::tests::a_projection_is_still_accepted_in_every_other_position_of_an_impl`, `check::tests::a_projection_nested_inside_a_compound_type_normalizes`, `check::tests::a_projection_on_a_concrete_type_normalizes_at_a_use_site`, `check::tests::a_projection_on_a_generic_parameter_resolves`, `check::tests::a_projection_projects_onto_the_correct_parameter_when_there_are_several`, `check::tests::a_projection_resolves_through_a_transitive_supertrait_bound`, `check::tests::a_projection_resolves_to_the_bounding_traits_own_associated_type_not_an_unrelated_one`, `check::tests::a_record_bound_is_not_enforced_at_construction`, `check::tests::a_record_bound_resolves_a_projection_through_a_supertrait`, `check::tests::a_record_bound_resolves_when_the_bounded_parameter_is_not_first`, `check::tests::a_record_field_initializer_normalizes_a_projection_once_its_parameter_is_concrete`, `check::tests::a_record_field_may_name_a_projection_on_a_bounded_parameter`, `check::tests::a_rejected_declarations_own_body_is_never_checked`, `check::tests::a_reported_binding_cycle_is_poisoned_and_does_not_cascade`, `check::tests::a_required_methods_own_signature_resolves_its_own_traits_self_item`, `check::tests::a_top_level_const_still_works_beside_an_impl`, `check::tests::a_trait_may_not_declare_one_name_twice`, `check::tests::a_trait_named_for_a_builtin_still_works`, `check::tests::a_trait_records_its_associated_types_in_order`, `check::tests::a_value_named_for_a_builtin_still_works`, `check::tests::a_view_desugars_to_std_uis_calls`, `check::tests::a_view_with_no_root_or_two_is_e0094`, `check::tests::a_views_parts_are_fixed_or_read_again_by_literalness`, `check::tests::a_void_element_with_children_is_e0095`, `check::tests::a_well_formed_test_is_accepted`, `check::tests::an_attribute_or_handler_of_another_type_is_e0097`, `check::tests::an_element_that_is_no_builder_is_e0096`, `check::tests::an_impl_binds_its_associated_type`, `check::tests::an_impl_may_echo_an_associated_type_inherited_from_a_supertrait`, `check::tests::an_impl_may_echo_the_projection_or_write_the_concrete_type`, `check::tests::applicable_inherent_method_still_takes_priority`, `check::tests::array_param_and_return_ok`, `check::tests::arrays_literal_index_len_ok`, `check::tests::associated_function_call_on_concrete_type`, `check::tests::associated_function_on_generic_impl_resolves_type_arg`, `check::tests::associated_function_with_args`, `check::tests::async_inherent_method_is_accepted`, `check::tests::awaiting_a_future_yields_its_output_type`, `check::tests::bare_fn_as_value_ok`, `check::tests::block_on_outside_async_is_allowed`, `check::tests::bool_match_true_and_false_is_exhaustive`, `check::tests::bounds_on_supported_positions_do_not_report_e0900`, `check::tests::break_and_continue_in_loops_ok`, `check::tests::break_in_while_condition_ok`, `check::tests::bytes_is_nameable_in_representative_positions`, `check::tests::closure_assigns_captured_variable_ok`, `check::tests::closure_calls_captured_fn_value_ok`, `check::tests::closure_capturing_local_ok`, `check::tests::closure_no_capture_ok`, `check::tests::closure_to_higher_order_ok`, `check::tests::compound_assignment_to_field_checks`, `check::tests::conditional_generic_impl_ok`, `check::tests::conformance_normalizes_a_selfless_methods_projection`, `check::tests::conformance_normalizes_inside_a_compound_type`, `check::tests::conformance_normalizes_parameter_types_not_only_the_return_type`, `check::tests::conformance_normalizes_through_the_matching_impl_not_the_only_one`, `check::tests::conformance_resolves_a_projection_bound_by_a_later_declared_impl`, `check::tests::conformance_still_rejects_a_wrong_arity_and_a_wrong_receiver`, `check::tests::conforming_impl_with_args_ok`, `check::tests::constants_ok`, `check::tests::cyclic_supertraits_do_not_hang`, `check::tests::declaring_a_type_named_for_a_builtin_is_rejected`, `check::tests::diamond_supertraits_are_not_ambiguous`, `check::tests::distinct_generics_are_accepted`, `check::tests::duplicate_supertrait_is_deduplicated`, `check::tests::every_reserved_name_really_is_a_builtin_type`, `check::tests::every_trait_call_agrees_with_its_callees_has_self`, `check::tests::exhaustive_match_ok`, `check::tests::extern_scalar_signature_is_accepted`, `check::tests::faithful_method_bound_impl_typechecks`, `check::tests::fibonacci_checks`, `check::tests::field_assignment_on_generic_record_substitutes_the_field_type`, `check::tests::field_assignment_picks_the_declared_field_index`, `check::tests::field_assignment_through_broken_receiver_does_not_cascade`, `check::tests::field_assignment_through_mutable_nested_record_ok`, `check::tests::field_assignment_typechecks`, `check::tests::fn_as_value_ok`, `check::tests::for_loop_does_not_shadow_user_end`, `check::tests::for_loop_over_range_ok`, `check::tests::function_typed_constant_called_directly_ok`, `check::tests::future_of_int_unifies_with_itself`, `check::tests::future_used_as_a_call_qualifier_is_rejected`, `check::tests::generic_async_fn_instantiates_at_float`, `check::tests::generic_bound_ok`, `check::tests::generic_extern_reports_e0900_without_cascade`, `check::tests::generic_identity_instantiates`, `check::tests::generic_impl_method_for_nongeneric_trait_method_reports_e0072`, `check::tests::generic_impl_of_subtrait_with_a_generic_supertrait_impl_typechecks`, `check::tests::generic_inherent_impl_ok`, `check::tests::generic_record_instantiates`, `check::tests::generic_sum_in_field_or_payload_typechecks`, `check::tests::generic_trait_impl_ok`, `check::tests::generic_trait_method_typechecks`, `check::tests::hash_dispatches_on_each_concrete_primitive`, `check::tests::hash_impls_typecheck_for_primitives`, `check::tests::hello_world_checks`, `check::tests::hir_trait_def_records_resolved_supertraits`, `check::tests::if_with_diverging_then_takes_else_type`, `check::tests::impl_method_bound_redundantly_spelling_supertrait_typechecks`, `check::tests::impl_of_subtrait_with_supertrait_typechecks`, `check::tests::index_set_nested_mutable_ok`, `check::tests::index_set_through_mutable_field_ok`, `check::tests::inherent_associated_function_on_a_primitive_resolves`, `check::tests::inherent_method_generic_infers_and_checks`, `check::tests::inherent_method_ok`, `check::tests::int_match_with_catch_all_ok`, `check::tests::known_gap_a_projection_parameter_before_its_determining_parameter`, `check::tests::map_methods_typecheck`, `check::tests::multiple_record_type_param_bounds_no_longer_report_e0900`, `check::tests::mut_self_is_a_mutable_root_inside_the_method_body`, `check::tests::mut_self_method_called_on_a_field_of_mut_self_typechecks`, `check::tests::mut_self_method_on_mutable_receiver_typechecks`, `check::tests::mut_self_trait_method_from_a_mut_self_default_body_is_accepted`, `check::tests::mut_self_trait_method_on_a_field_of_a_mutable_root_is_accepted`, `check::tests::mut_self_trait_method_on_a_mutable_receiver_is_accepted`, `check::tests::mut_self_trait_method_through_a_mut_generic_parameter_is_accepted`, `check::tests::mutable_assign_ok`, `check::tests::nested_closure_calling_local_ok`, `check::tests::non_applicable_inherent_fmt_does_not_block_display_bridge`, `check::tests::non_applicable_inherent_method_does_not_shadow_trait_method`, `check::tests::non_cyclic_constant_chain_ok`, `check::tests::non_overlapping_concrete_trait_impls_ok`, `check::tests::non_overlapping_inherent_impls_ok`, `check::tests::normalization_picks_the_impl_that_matches_the_receiver`, `check::tests::panic_typechecks_as_never_in_match_arm`, `check::tests::plain_self_method_on_immutable_receiver_still_typechecks`, `check::tests::qualified_variant_call_with_args_still_typechecks`, `check::tests::record_literal_and_field_access_ok`, `check::tests::record_literal_in_an_interpolation_reaches_both_interp_paths`, `check::tests::record_spread_base_ok`, `check::tests::record_type_param_bound_no_longer_reports_e0900`, `check::tests::redundant_same_trait_bounds_are_not_ambiguous`, `check::tests::rejected_type_arguments_on_a_projection_are_not_themselves_resolved`, `check::tests::repeat_array_elem_type_follows_the_init_expression`, `check::tests::repeat_array_typechecks_and_has_array_type`, `check::tests::repeated_param_inherent_impl_matches_only_uniform_receiver`, `check::tests::return_in_while_condition_ok`, `check::tests::self_item_in_a_trait_impl_projects_onto_the_impls_own_self_type`, `check::tests::self_item_resolves_in_an_impl_method_body_annotation_too`, `check::tests::selfless_method_param_types_are_declared_types`, `check::tests::selfless_method_params_are_not_shifted`, `check::tests::selfless_trait_impl_method_checks_conformance_without_panicking`, `check::tests::set_methods_typecheck`, `check::tests::signatures_are_not_compared_when_either_side_is_already_poisoned`, `check::tests::single_concrete_inherent_associated_function_still_resolves`, `check::tests::std_core_option_is_available_without_import`, `check::tests::std_core_option_result_methods_typecheck`, `check::tests::std_core_parses_and_typechecks_clean`, `check::tests::std_core_traits_and_primitive_impls_typecheck`, `check::tests::std_task_spawn_and_join_typecheck`, `check::tests::subtrait_bound_expansion_does_not_break_conformance_bound_sets`, `check::tests::subtrait_bound_resolves_the_supertrait_method_to_the_supertrait`, `check::tests::supertrait_impl_may_be_declared_after_the_subtrait_impl`, `check::tests::supertrait_method_callable_from_a_default_body`, `check::tests::supertrait_method_callable_through_an_impl_generic_bound`, `check::tests::supertrait_method_callable_through_subtrait_bound`, `check::tests::the_compiler_terminates_on_every_cyclic_binding_shape`, `check::tests::the_implicit_self_of_a_trait_body_is_not_a_user_written_parameter`, `check::tests::trait_associated_function_on_concrete_type`, `check::tests::trait_associated_function_result_type_is_the_qualifier`, `check::tests::trait_associated_function_through_bound`, `check::tests::trait_associated_function_through_bound_has_param_self_ty`, `check::tests::trait_associated_function_with_default_body_on_concrete_type`, `check::tests::trait_call_substitution_puts_self_before_the_methods_own_generics`, `check::tests::trait_impl_and_default_method_ok`, `check::tests::two_declarations_of_the_same_reserved_name_each_raise_e0089_not_e0002`, `check::tests::two_traits_with_the_same_associated_type_name_get_distinct_defids`, `check::tests::vec_methods_typecheck`, `check::tests::where_clause_on_function_enforces_bound`, `check::tests::where_clause_on_trait_impl_is_accepted`, `check::tests::yield_now_is_awaitable_inside_an_async_fn`, `check::tuple_tests::a_method_on_a_tuple_is_e0014`, `check::tuple_tests::a_name_on_a_tuple_is_e0014`, `check::tuple_tests::a_parenthesised_type_and_expression_are_not_tuples`, `check::tuple_tests::a_record_field_may_hold_a_tuple`, `check::tuple_tests::a_tuple_of_one_is_not_its_element`, `check::tuple_tests::a_tuple_of_the_wrong_arity_is_e0010`, `check::tuple_tests::a_tuple_of_the_wrong_element_type_is_e0010`, `check::tuple_tests::an_element_is_written_through_a_mutable_binding`, `check::tuple_tests::an_element_whose_type_cannot_be_inferred_is_e0011`, `check::tuple_tests::an_index_on_a_type_that_is_not_a_tuple_is_e0014`, `check::tuple_tests::an_index_on_a_value_of_unknown_type_is_one_e0011`, `check::tuple_tests::an_index_past_the_end_is_e0014`, `check::tuple_tests::equality_on_a_record_has_no_tuple_note`, `check::tuple_tests::equality_on_tuples_is_e0013_with_a_note`, `check::tuple_tests::extern_c_refuses_a_tuple`, `check::tuple_tests::interpolating_a_tuple_is_e0013_with_a_note`, `check::tuple_tests::no_impl_can_name_a_tuple`, `check::tuple_tests::tuple_types_and_literals_check`, `check::tuple_tests::tuples_flow_through_closures_and_function_types`, `check::tuple_tests::tuples_infer_through_generic_functions`, `completion::tests::a_binding_in_a_nested_pattern_completes`, `completion::tests::bindings_in_a_for_pattern_and_an_or_pattern_complete`, `doctest::tests::a_main_that_returns_a_value_runs_as_a_program_would`, `doctest::tests::a_std_doc_test_imports_nothing_and_names_its_modules_file`, `doctest::tests::doc_tests_are_named_by_the_sites_paths`, `doctest::tests::only_a_librarys_documented_items_give_doc_tests`, `hover::tests::a_binding_in_a_nested_pattern_hovers_with_its_type`, `hover::tests::bindings_in_a_for_pattern_and_a_nested_or_pattern_hover`, `model::tests::a_package_named_std_stops_it`, `model::tests::a_program_documents_every_item_and_its_library_only_its_exports`, `model::tests::an_impl_goes_with_its_type_and_a_trait_lists_its_impls`, `model::tests::std_is_one_package_of_modules_primitives_and_builtins`, `model::tests::two_impls_of_one_type_number_a_method_they_share`.
  17. **The E0001 at a bare variant name loses its note.** `crates/nova-typeck/src/check.rs`:
      replace

      ```
                              .with_primary_label(pattern.span, "here")
                              .with_note(
                                  "a name inside a pattern that names a variant matches that variant; \
                                   to bind a value, use a lower-case name"
                                      .to_string(),
                              ),
      ```

      with

      ```
                              .with_primary_label(pattern.span, "here"),
      ```

      - Run `T --skip net::tests::an_ipv6_connect_parks_on_its_first_poll_and_then_establishes`.
      - Expected: fails: `check::pattern_tests::another_types_variant_name_inside_a_variant_is_e0001_with_a_note`.
  18. **A range pattern does not type the value it tests.** `crates/nova-typeck/src/check/patterns.rs`:
      replace

      ```
              self.unify_pattern_ty(fcx, &ty, scrut_ty, span);
              let empty
      ```

      with

      ```
              let empty
      ```

      - Run `T --skip net::tests::an_ipv6_connect_parks_on_its_first_poll_and_then_establishes`.
      - Expected: fails: `check::pattern_tests::a_range_over_another_type_is_e0010`.
  19. **`lo..lo` is not E0101.** `crates/nova-typeck/src/check/patterns.rs`:
      `let empty = if inclusive { lo_v > hi_v } else { lo_v >= hi_v };` becomes `let empty = lo_v > hi_v;`.
      - Run `T --skip net::tests::an_ipv6_connect_parks_on_its_first_poll_and_then_establishes`.
      - Expected: fails: `check::pattern_tests::an_empty_range_is_e0101`.
  20. **`lo..hi` includes `hi`.** `crates/nova-typeck/src/check/patterns.rs`:
      `hi: if inclusive { hi_v } else { hi_v - 1 },` becomes `hi: hi_v,`.
      - Run `T --skip net::tests::an_ipv6_connect_parks_on_its_first_poll_and_then_establishes`, `cargo test --locked --workspace --test pattern_lowering` and `R patterns_run`, each to its end.
      - Expected: fails: `a_range_is_two_compares_with_its_ends_included`, `check::pattern_tests::nested_patterns_check_to_a_recursive_hir_pattern`, `patterns_run`.
  21. **An alternative missing a binding is not E0100.** `crates/nova-typeck/src/check/patterns.rs`:
      `for name in missing {` becomes `for name in missing.into_iter().take(0) {`.
      - Run `T --skip net::tests::an_ipv6_connect_parks_on_its_first_poll_and_then_establishes`.
      - Expected: fails: `check::pattern_tests::an_alternative_missing_a_binding_is_e0100_naming_it`, `check::pattern_tests::nested_or_patterns_bind_the_outer_first_alternatives_names`.
  22. **An alternative with an extra binding is not E0100.** `crates/nova-typeck/src/check/patterns.rs`:
      `if found.is_some() {` becomes `if false {`.
      - Run `T --skip net::tests::an_ipv6_connect_parks_on_its_first_poll_and_then_establishes`.
      - Expected: fails: `check::pattern_tests::an_alternative_with_an_extra_binding_is_e0100_naming_it`.
  23. **A guard need not be a `Bool`.** `crates/nova-typeck/src/check.rs`:
      `self.expect_ty(fcx, &guard, &Ty::Bool, "a match guard");`: the line goes.
      - Run `T --skip net::tests::an_ipv6_connect_parks_on_its_first_poll_and_then_establishes`.
      - Expected: fails: `check::pattern_tests::guards_see_the_arms_bindings_and_must_be_bool`.
  24. **`let mut (a, b)` binds immutable names.** `crates/nova-typeck/src/check/patterns.rs`:
      `let is_mut = is_mut || self.pats.mutable;` becomes `let is_mut = is_mut;`.
      - Run `T --skip net::tests::an_ipv6_connect_parks_on_its_first_poll_and_then_establishes`.
      - Expected: fails: `check::pattern_tests::let_destructures_a_tuple_and_a_one_variant_sum`.
  25. **Usefulness reads a variant's fields as `_`.** `crates/nova-typeck/src/check.rs`:
      `fields.iter().map(to_useful_pat).collect(),` becomes `vec![Pat::Wild; fields.len()],`.
      - Run `T --skip net::tests::an_ipv6_connect_parks_on_its_first_poll_and_then_establishes`.
      - Expected: fails: `check::pattern_tests::a_generic_sum_matched_at_depth_is_exhaustive`, `check::pattern_tests::a_lower_case_or_underscored_binding_or_a_real_variant_is_not_e0104`, `check::pattern_tests::a_missing_nested_variant_is_e0020_with_a_nested_witness`, `check::pattern_tests::a_variant_name_inside_a_variant_matches_that_variant`, `check::pattern_tests::an_unreachable_alternative_is_e0021_on_it_alone`, `check::pattern_tests::literals_and_ranges_need_a_catch_all`, `check::pattern_tests::nested_patterns_check_to_a_recursive_hir_pattern`.
  26. **A row whose head is an or-pattern is not expanded.** `crates/nova-typeck/src/usefulness.rs`:
      `let matrix = if matrix.iter().any(|r| matches!(r[0], Pat::Or(_))) {` becomes `let matrix = if false {`.
      - Run `T --skip net::tests::an_ipv6_connect_parks_on_its_first_poll_and_then_establishes`.
      - Expected: fails: `check::pattern_tests::an_alternative_inside_one_reported_is_not_reported_again`, `check::pattern_tests::an_or_pattern_covers_each_alternative`, `check::pattern_tests::an_unreachable_alternative_is_e0021_on_it_alone`, `check::pattern_tests::an_upper_case_binding_in_an_or_pattern_warns_once`, `check::pattern_tests::literals_and_ranges_need_a_catch_all`, `check::pattern_tests::nested_or_patterns_bind_the_outer_first_alternatives_names`, `check::pattern_tests::nested_patterns_check_to_a_recursive_hir_pattern`, `usefulness::tests::an_or_pattern_row_covers_every_alternative`.
  27. **An or-pattern is useful only through its first alternative.** `crates/nova-typeck/src/usefulness.rs`:
      replace

      ```
                      let mut out = Vec::new();
                      for a in alts {
                          let mut row = vec![a.clone()];
      ```

      with

      ```
                      let mut out = Vec::new();
                      for a in alts.iter().take(1) {
                          let mut row = vec![a.clone()];
      ```

      - Run `T --skip net::tests::an_ipv6_connect_parks_on_its_first_poll_and_then_establishes`.
      - Expected: fails: `usefulness::tests::an_or_pattern_row_covers_every_alternative`.
  28. **A tuple type lists no constructor.** `crates/nova-typeck/src/usefulness.rs`:
      `Ty::Tuple(elems) => vec![Ctor::Tuple(elems.len() as u32)],`: the line goes.
      - Run `T --skip net::tests::an_ipv6_connect_parks_on_its_first_poll_and_then_establishes`.
      - Expected: fails: `check::pattern_tests::a_destructured_let_binding_is_immutable_without_mut`, `check::pattern_tests::a_destructured_let_is_a_one_arm_match`, `check::pattern_tests::a_for_loop_variable_over_an_array_is_immutable`, `check::pattern_tests::a_tuple_of_one_witness_keeps_its_comma`, `check::pattern_tests::a_tuple_of_sums_missing_a_combination_is_e0020`, `check::pattern_tests::a_tuple_pattern_arm_after_a_bare_return_or_an_ordinary_arm_checks`, `check::pattern_tests::an_upper_case_binding_in_a_for_pattern_is_e0104`, `check::pattern_tests::an_upper_case_binding_is_e0104`, `check::pattern_tests::for_destructures_over_an_array_and_an_iterator`, `check::pattern_tests::let_destructures_a_tuple_and_a_one_variant_sum`, `check::pattern_tests::nested_or_patterns_bind_the_outer_first_alternatives_names`, `usefulness::tests::a_tuple_is_one_constructor_even_when_its_type_is_unknown`, `usefulness::tests::a_tuple_of_sums_needs_every_combination`.
  29. **A column of tuples whose type is unknown is not read as a tuple.** `crates/nova-typeck/src/usefulness.rs`:
      `let col0 = tuple_ty(&col_tys[0], &present);` becomes `let col0: Option<Ty> = None;`.
      - Run `T --skip net::tests::an_ipv6_connect_parks_on_its_first_poll_and_then_establishes`.
      - Expected: fails: `usefulness::tests::a_tuple_is_one_constructor_even_when_its_type_is_unknown`.
  30. **A witness tuple of one loses its comma.** `crates/nova-typeck/src/check.rs`:
      `format!("({},)", inner[0])` becomes `format!("({})", inner[0])`.
      - Run `T --skip net::tests::an_ipv6_connect_parks_on_its_first_poll_and_then_establishes`.
      - Expected: fails: `check::pattern_tests::a_tuple_of_one_witness_keeps_its_comma`.
  31. **An unreachable alternative is not reported.** `crates/nova-typeck/src/check.rs`:
      `self.check_or_alternatives(&cx, &prior, pat, or_spans, &col);`: the line goes.
      - Run `T --skip net::tests::an_ipv6_connect_parks_on_its_first_poll_and_then_establishes`.
      - Expected: fails: `check::pattern_tests::an_alternative_inside_one_reported_is_not_reported_again`, `check::pattern_tests::an_unreachable_alternative_is_e0021_on_it_alone`.
  32. **An alternative inside one reported is reported again.** `crates/nova-typeck/src/check/patterns.rs`:
      `if ancestors(&tree, *parent).any(|a| dead.contains(&a)) {` becomes `if false {`.
      - Run `T --skip net::tests::an_ipv6_connect_parks_on_its_first_poll_and_then_establishes`.
      - Expected: fails: `check::pattern_tests::an_alternative_inside_one_reported_is_not_reported_again`.
  33. **A refutable `let` pattern is not E0022.** `crates/nova-typeck/src/check.rs`:
      replace

      ```
                                      self.require_irrefutable(
                                          fcx,
                                          &pat,
                                          &value.ty,
                                          pattern.span,
                                          "`let`",
                                      );
      ```

      with

      ```

      ```

      - Run `T --skip net::tests::an_ipv6_connect_parks_on_its_first_poll_and_then_establishes`.
      - Expected: fails: `check::pattern_tests::a_refutable_let_pattern_is_e0022_with_the_values_it_misses`.
  34. **A refutable `for` pattern is not E0022.** `crates/nova-typeck/src/check/patterns.rs`:
      `` self.require_irrefutable(fcx, &pat, ty, pattern.span, "a `for` loop"); ``: the line goes.
      - Run `T --skip net::tests::an_ipv6_connect_parks_on_its_first_poll_and_then_establishes`.
      - Expected: fails: `check::pattern_tests::a_refutable_for_pattern_is_e0022`.
  35. **A false guard enters the body anyway.** `crates/nova-mir/src/lower.rs`:
      replace

      ```
                              then_: body,
                              else_: next,
      ```

      with

      ```
                              then_: body,
                              else_: body,
      ```

      - Run `cargo test --locked --workspace --test pattern_lowering`.
      - Expected: fails: `a_false_guard_jumps_to_the_next_arms_chain`.
  36. **An alternative that fails skips the alternatives after it.** `crates/nova-mir/src/lower.rs`:
      `self.lower_pattern_test(alt, value, ty, alt_fail);` becomes `self.lower_pattern_test(alt, value, ty, fail);`.
      - Run `cargo test --locked --workspace --test pattern_lowering` and `R patterns_run`, each to its end.
      - Expected: fails: `patterns_run`.
  37. **A chained match evaluates its scrutinee a second time.** `crates/nova-mir/src/lower.rs`:
      replace

      ```
                  self.lower_chain_arms(s, &scrutinee.ty, arms, result, join);
      ```

      with

      ```
                  let s2 = self.lower_expr(scrutinee);
                  self.lower_chain_arms(s2, &scrutinee.ty, arms, result, join);
      ```

      - Run `cargo test --locked --workspace --test pattern_lowering` and `R match_effects_run`, each to its end.
      - Expected: fails: `match_effects_run`, `the_scrutinee_is_evaluated_once`.
  38. **A variant's field is projected again before its tests.** `crates/nova-mir/src/lower.rs`:
      replace

      ```
                          self.lower_pattern_test(field, t, &fty, fail);
      ```

      with

      ```
                          let t = if mty == MirTy::Unit {
                              t
                          } else {
                              let t2 = self.new_temp(mty);
                              self.push(Stmt::SumField {
                                  dst: t2,
                                  sum: value,
                                  index: i as u32,
                                  ty: mty,
                              });
                              t2
                          };
                          self.lower_pattern_test(field, t, &fty, fail);
      ```

      - Run `cargo test --locked --workspace --test pattern_lowering`.
      - Expected: fails: `each_projection_is_evaluated_once_per_chain`, `let_destructuring_lowers_to_projections_alone`.
  39. **A sum of one variant gets a tag test.** `crates/nova-mir/src/lower.rs`:
      `if def.map_or(true, |d| d.variants.len() > 1) {` becomes `if true {`.
      - Run `cargo test --locked --workspace --test pattern_lowering`.
      - Expected: fails: `let_destructuring_lowers_to_projections_alone`.
  40. **A flat `Char` match loses its switch keys.** `crates/nova-mir/src/lower.rs`:
      `hir::Pattern::LitChar(c) => Some(*c as i64),`: the line goes.
      - Run `cargo test --locked --workspace --test pattern_lowering` and `R patterns_run`, each to its end.
      - Expected: fails: `a_flat_match_keeps_its_one_switch`.
  41. **A guarded flat match takes the flat switch.** `crates/nova-mir/src/lower.rs`:
      `.all(|a| a.guard.is_none() && a.pattern.is_flat())` becomes `.all(|a| a.pattern.is_flat())`.
      - Run `cargo test --locked --workspace --test pattern_lowering` and `R match_effects_run`, each to its end.
      - Expected: fails: `a_false_guard_jumps_to_the_next_arms_chain`.
  42. **`for` over an array takes the iterator path.** `crates/nova-typeck/src/check.rs`:
      `if let Ty::Array(elem) = &iter_ty {` becomes `if let (false, Ty::Array(elem)) = (true, &iter_ty) {`.
      - Run `T --skip net::tests::an_ipv6_connect_parks_on_its_first_poll_and_then_establishes` and `R for_arrays_run`, each to its end.
      - Expected: fails: `check::pattern_tests::a_for_loop_variable_over_an_array_is_immutable`, `check::pattern_tests::a_refutable_for_pattern_is_e0022`, `check::pattern_tests::an_upper_case_binding_in_a_for_pattern_is_e0104`, `check::pattern_tests::for_destructures_over_an_array_and_an_iterator`, `check::pattern_tests::for_over_an_array_desugars_to_an_index_loop`, `completion::tests::bindings_in_a_for_pattern_and_an_or_pattern_complete`, `for_arrays_run`, `hover::tests::bindings_in_a_for_pattern_and_a_nested_or_pattern_hover`.
  43. **The array's length is read at each iteration.** `crates/nova-typeck/src/check/patterns.rs`:
      replace

      ```
                      rhs: Box::new(read(len, Ty::Int)),
      ```

      with

      ```
                      rhs: Box::new(expr(
                          hir::ExprKind::ArrayLen {
                              target: Box::new(read(arr, arr_ty.clone())),
                          },
                          Ty::Int,
                      )),
      ```

      - Run `T --skip net::tests::an_ipv6_connect_parks_on_its_first_poll_and_then_establishes`.
      - Expected: fails: `check::pattern_tests::for_over_an_array_desugars_to_an_index_loop`.
  44. **The index advances after the body, so `continue` loops forever.** `crates/nova-typeck/src/check/patterns.rs`:
      `stmts: vec![bind.bind(element, span), advance, body_hir],` becomes `stmts: vec![bind.bind(element, span), body_hir, advance],`.
      - Run `timeout 600 cargo test --locked --workspace --test run_tests -- for_arrays_run; c=$?; [ $c = 124 ] && echo 'for_arrays_run timed out'; exit $c`, spelled out: `timeout` runs a program, not a shell function such as `R`. It takes the 600 seconds.
      - Expected: exits 124, its output holding `for_arrays_run timed out`.
  45. **A split before `(` warns after any value.** `crates/nova-typeck/src/check.rs`:
      `(ast::SplitKind::Call, ty) => matches!(ty, Ty::Fn { .. }),` becomes `(ast::SplitKind::Call, _) => true,`.
      - Run `T check::lang_tests`.
      - Expected: fails: `check::lang_tests::a_split_warns_only_after_a_value_the_old_reading_could_extend`, `check::lang_tests::a_tuple_tail_after_a_call_that_returns_no_function_is_quiet`, `check::lang_tests::e0103_reports_a_parenthesised_callee_after_a_line_break`, `check::lang_tests::the_split_operand_is_read_through_an_assignment`.
  46. **A split before `[` warns after any value.** `crates/nova-typeck/src/check.rs`:
      `(ast::SplitKind::Index, ty) => matches!(ty, Ty::Array(_)),` becomes `(ast::SplitKind::Index, _) => true,`.
      - Run `T check::lang_tests`.
      - Expected: fails: `check::lang_tests::a_split_warns_only_after_a_value_the_old_reading_could_extend`.
  47. **A split before `-` is quiet after a `Float`.** `crates/nova-typeck/src/check.rs`:
      `(ast::SplitKind::Minus, ty) => matches!(ty, Ty::Int | Ty::Float),` becomes `(ast::SplitKind::Minus, ty) => matches!(ty, Ty::Int),`.
      - Run `T check::lang_tests`.
      - Expected: fails: `check::lang_tests::a_split_warns_only_after_a_value_the_old_reading_could_extend`.
  48. **A split after a value of unknown type is judged as known.** `crates/nova-typeck/src/check.rs`:
      `(ast::SplitKind::Value, _) | (_, Ty::Var(_)) => true,` becomes `(ast::SplitKind::Value, _) => true,`.
      - Run `T check::lang_tests`.
      - Expected: fails: `check::lang_tests::a_split_after_a_value_of_unknown_type_warns`.
  49. **A split after a bare `return` is judged by type.** `crates/nova-typeck/src/check.rs`:
      replace

      ```
      (ast::SplitKind::Value, _) | (_, Ty::Var(_)) => true,
      ```

      with

      ```
      (_, Ty::Var(_)) => true,
                  (ast::SplitKind::Value, _) => false,
      ```

      - Run `T check::lang_tests`.
      - Expected: fails: `check::lang_tests::a_closures_bare_return_before_a_parenthesis_or_bracket_warns_once`, `check::lang_tests::a_split_after_a_bare_return_before_a_tail_warns`, `check::lang_tests::a_split_after_a_bare_return_warns`, `check::lang_tests::a_split_after_a_bare_return_warns_whatever_comes_before`.
  50. **A quiet split still warns.** `crates/nova-typeck/src/check.rs`:
      `if self.quiet_splits.contains(&split.at) {` becomes `if false {`.
      - Run `T check::lang_tests`.
      - Expected: fails: `check::lang_tests::a_split_after_a_closure_reads_its_body`, `check::lang_tests::a_split_warns_only_after_a_value_the_old_reading_could_extend`, `check::lang_tests::a_tuple_tail_after_a_call_that_returns_no_function_is_quiet`, `check::lang_tests::e0103_reports_a_parenthesised_callee_after_a_line_break`, `check::lang_tests::the_split_operand_is_read_through_an_assignment`.
  51. **A split after a `let` is never judged.** `crates/nova-typeck/src/check.rs`:
      `self.judge_split(fcx, &value, next);`: the line goes.
      - Run `T check::lang_tests`.
      - Expected: fails: `check::lang_tests::a_split_after_a_closure_reads_its_body`, `check::lang_tests::a_split_warns_only_after_a_value_the_old_reading_could_extend`.
  52. **The split's operand is a binary expression, not its right operand.** `crates/nova-typeck/src/check.rs`:
      `K::Binary { rhs, .. } | K::LogicalAnd { rhs, .. } | K::LogicalOr { rhs, .. } => {` becomes `K::LogicalAnd { rhs, .. } | K::LogicalOr { rhs, .. } => {`.
      - Run `T check::lang_tests`.
      - Expected: fails: `check::lang_tests::a_split_warns_only_after_a_value_the_old_reading_could_extend`.
  53. **The split's operand is the assignment, not its value.** `crates/nova-typeck/src/check.rs`:
      `K::Assign { value, .. } | K::FieldSet { value, .. } | K::IndexSet { value, .. } => {` becomes `K::FieldSet { value, .. } | K::IndexSet { value, .. } => {`.
      - Run `T check::lang_tests`.
      - Expected: fails: `check::lang_tests::the_split_operand_is_read_through_an_assignment`.
  54. **The split's operand is a compound assignment's block, not its write.** `crates/nova-typeck/src/check.rs`:
      `split_operand(set, kind, closures)` becomes `e`.
      - Run `T check::lang_tests`.
      - Expected: fails: `check::lang_tests::a_split_after_a_compound_assignment_to_an_element_or_a_field_warns`.
  55. **The split's operand is the closure, not its body.** `crates/nova-typeck/src/check.rs`:
      `Some(f) => split_operand(&f.body, kind, closures),` becomes `Some(_) => e,`.
      - Run `T check::lang_tests`.
      - Expected: fails: `check::lang_tests::a_split_after_a_closure_reads_its_body`.
  56. **A bare function's closure wrapper is read as a closure.** `crates/nova-typeck/src/check.rs`:
      `.find(|f| f.def_id == *func && f.name.starts_with("closure$"))` becomes `.find(|f| f.def_id == *func)`.
      - Run `T check::lang_tests`.
      - Expected: fails: `check::lang_tests::a_split_after_a_closure_reads_its_body`.
  57. **An upper-case binding is not E0104.** `crates/nova-typeck/src/check/patterns.rs`:
      `if name.value.chars().next().is_some_and(char::is_uppercase) {` becomes `if false {`.
      - Run `T --skip net::tests::an_ipv6_connect_parks_on_its_first_poll_and_then_establishes`.
      - Expected: fails: `check::pattern_tests::an_upper_case_binding_in_a_for_pattern_is_e0104`, `check::pattern_tests::an_upper_case_binding_in_an_or_pattern_warns_once`, `check::pattern_tests::an_upper_case_binding_is_e0104`, `check::pattern_tests::e0104_near_a_variant_says_which`, `check::pattern_tests::e0104_on_a_constant_says_to_use_a_guard`.
  58. **E0104 fires for any upper-case letter, `_Upper` too.** `crates/nova-typeck/src/check/patterns.rs`:
      `if name.value.chars().next().is_some_and(char::is_uppercase) {` becomes `if name.value.chars().any(char::is_uppercase) {`.
      - Run `T --skip net::tests::an_ipv6_connect_parks_on_its_first_poll_and_then_establishes`.
      - Expected: fails: `check::pattern_tests::a_lower_case_or_underscored_binding_or_a_real_variant_is_not_e0104`.
  59. **An erroneous pattern still feeds usefulness.** `crates/nova-typeck/src/check.rs`:
      replace

      ```
              if !bad_pattern {
                  self.check_match_usefulness(fcx, &scrut, &arm_pats, span);
              }
      ```

      with

      ```
              let _ = bad_pattern;
              self.check_match_usefulness(fcx, &scrut, &arm_pats, span);
      ```

      - Run `T --skip net::tests::an_ipv6_connect_parks_on_its_first_poll_and_then_establishes`.
      - Expected: fails: `check::pattern_tests::an_erroneous_pattern_does_not_make_the_arms_after_it_unreachable`.
  60. **E0022's witness names no value, only `_`.** `crates/nova-typeck/src/check/patterns.rs`:
      `let cx = usefulness::MatchCx::new(&sums).naming_values();` becomes `let cx = usefulness::MatchCx::new(&sums);`.
      - Run `T --skip net::tests::an_ipv6_connect_parks_on_its_first_poll_and_then_establishes`.
      - Expected: fails: `check::pattern_tests::a_refutable_for_pattern_is_e0022`, `check::pattern_tests::a_refutable_let_pattern_is_e0022_with_the_values_it_misses`.
  61. **A pattern's binding gets E0060's `let mut` note.** `crates/nova-typeck/src/check/fixes.rs`:
      `Some(Binding::LetPattern) => format!(` becomes `Some(Binding::Let) => format!(`.
      - Run `T --skip net::tests::an_ipv6_connect_parks_on_its_first_poll_and_then_establishes`.
      - Expected: fails: `check::pattern_tests::a_destructured_let_binding_is_immutable_without_mut`.
  62. **E0104 at a constant's name has the variant's note.** `crates/nova-typeck/src/check/patterns.rs`:
      `let note = if is_const {` becomes `let note = if false {`.
      - Run `T --skip net::tests::an_ipv6_connect_parks_on_its_first_poll_and_then_establishes`.
      - Expected: fails: `check::pattern_tests::e0104_on_a_constant_says_to_use_a_guard`.
  63. **E0104 near a variant names none.** `crates/nova-typeck/src/check/patterns.rs`:
      `suggest::closest(&name.value, variants.iter().map(String::as_str))` becomes `None::<&str>`.
      - Run `T --skip net::tests::an_ipv6_connect_parks_on_its_first_poll_and_then_establishes`.
      - Expected: fails: `check::pattern_tests::e0104_near_a_variant_says_which`.
  64. **A `let` pattern with an error is also E0022.** `crates/nova-typeck/src/check.rs`:
      replace

      ```
                                  if self.error_count() == errors {
                                      self.require_irrefutable(
      ```

      with

      ```
                                  if true {
                                      self.require_irrefutable(
      ```

      - Run `T --skip net::tests::an_ipv6_connect_parks_on_its_first_poll_and_then_establishes`.
      - Expected: fails: `check::pattern_tests::a_let_pattern_with_an_error_is_not_also_e0022`.
  65. **A `for` pattern with an error is also E0022.** `crates/nova-typeck/src/check/patterns.rs`:
      replace

      ```
                      if self.error_count() == errors {
                          self.require_irrefutable(fcx, &pat, ty, pattern.span, "a `for` loop");
      ```

      with

      ```
                      if true {
                          self.require_irrefutable(fcx, &pat, ty, pattern.span, "a `for` loop");
      ```

      - Run `T --skip net::tests::an_ipv6_connect_parks_on_its_first_poll_and_then_establishes`.
      - Expected: fails: `check::pattern_tests::a_for_pattern_with_an_error_is_not_also_e0022`.
  66. **A destructuring `let`'s binding gets a match arm's note.** `crates/nova-typeck/src/check.rs`:
      `self.pats.in_let = true;`: the line goes.
      - Run `T --skip net::tests::an_ipv6_connect_parks_on_its_first_poll_and_then_establishes`.
      - Expected: fails: `check::pattern_tests::a_destructured_let_binding_is_immutable_without_mut`.
  67. **A `for` loop's variable gets `let mut x`'s note.** `crates/nova-typeck/src/check/fixes.rs`:
      `Some(Binding::Other) => format!(` becomes `Some(Binding::Param) => format!(`.
      - Run `T --skip net::tests::an_ipv6_connect_parks_on_its_first_poll_and_then_establishes`.
      - Expected: fails: `check::pattern_tests::a_for_loop_variable_over_an_array_is_immutable`, `check::pattern_tests::a_match_arm_or_a_loop_variable_is_rebound_to_be_mutable`.
  68. **A column no row names lists every constructor in the witness.** `crates/nova-typeck/src/usefulness.rs`:
      replace

      ```
              if present.is_empty() {
                  // No row names
      ```

      with

      ```
              if false {
                  // No row names
      ```

      - Run `T --skip net::tests::an_ipv6_connect_parks_on_its_first_poll_and_then_establishes`.
      - Expected: fails: `check::pattern_tests::a_refutable_for_pattern_is_e0022`.
  69. **E0022's witnesses leave out the values under a present constructor.** `crates/nova-typeck/src/usefulness.rs`:
      replace

      ```
                          if self.name_values {
                              for c in &present {
      ```

      with

      ```
                          if false {
                              for c in &present {
      ```

      - Run `T --skip net::tests::an_ipv6_connect_parks_on_its_first_poll_and_then_establishes`.
      - Expected: fails: `check::pattern_tests::a_refutable_for_pattern_is_e0022`.
  70. **Until chains, a guarded flat match is lowered, not refused.** `crates/nova-mir/src/lower.rs`:
      `.all(|a| a.guard.is_none() && a.pattern.is_flat())` becomes `.all(|a| a.pattern.is_flat())`.
      - At Task 3's commit: `git checkout` it first, and `git checkout claude/loving-archimedes-frtf5q-50b` after.
      - Run `cargo test --locked --workspace --test lower_tests`.
      - Expected: fails: `a_nested_or_guarded_match_is_refused_until_chains`.
  71. **Until chains, a nested match is lowered, not refused.** `crates/nova-mir/src/lower.rs`:
      `.all(|a| a.guard.is_none() && a.pattern.is_flat())` becomes `.all(|a| a.guard.is_none())`.
      - At Task 3's commit: `git checkout` it first, and `git checkout claude/loving-archimedes-frtf5q-50b` after.
      - Run `cargo test --locked --workspace --test lower_tests`.
      - Expected: fails: `a_nested_or_guarded_match_is_refused_until_chains`.
  72. **From Task 3, a flat `Char` match's switch takes no key.** `crates/nova-mir/src/lower.rs`:
      `hir::Pattern::LitChar(c) => Some(*c as i64),`: the line goes.
      - At Task 3's commit: `git checkout` it first, and `git checkout claude/loving-archimedes-frtf5q-50b` after.
      - Run `cargo test --locked --workspace --test lower_tests`.
      - Expected: fails: `a_flat_char_match_switches_on_its_code_points`.
  73. **fmt drops a tuple type of one's comma.** `crates/nova-fmt/src/print/ty.rs`:
      `let last = if items.len() == 1 {` becomes `let last = if items.len() == 0 {`.
      - Run `cargo test --locked --workspace --no-fail-fast --test tuples`.
      - Expected: fails: `a_tuple_of_one_keeps_its_comma`.
  74. **fmt drops a tuple pattern of one's comma.** `crates/nova-fmt/src/print/pattern.rs`:
      `let last = if items.len() == 1 {` becomes `let last = if items.len() == 0 {`.
      - Run `cargo test --locked --workspace --test patterns`.
      - Expected: fails: `a_tuple_pattern_of_one_keeps_its_comma`.
  75. **fmt never breaks a long or-pattern.** `crates/nova-fmt/src/print/pattern.rs`:
      `rest.push(Doc::Line);` becomes `rest.push(text(" "));`.
      - Run `cargo test --locked --workspace --test patterns`.
      - Expected: fails: `a_long_or_pattern_breaks_before_each_bar`.
  76. **fmt swaps `..` and `..=`.** `crates/nova-fmt/src/print/pattern.rs`:
      `text(if *inclusive { "..=" } else { ".." }),` becomes `text(if *inclusive { ".." } else { "..=" }),`.
      - Run `cargo test --locked --workspace --test patterns`.
      - Expected: fails: `char_literals_and_ranges_print_as_written`, `int_ranges_print_as_written`.
- [ ] **Step 10: The programs at Tasks 3 and 4** (decision 47). Every
  runtime program of the last tree, each `tests/runtime/*.nova` with a
  `.stdout`, and a probe of a flat `Char` match (`'a'`, `'b'`, `'z'` and
  `_`), run with `nova run` by four builds: `target/debug/nova` built at
  Task 3's commit, at Task 4's and at the last, each checked out and built
  in turn and run as soon as it is built, so no copy of a binary takes the
  disk; and then Task 0's copy of the base's `nova`, beside a link to
  `target/debug/libnova_runtime.a`, which 5.0b does not change. Every
  program runs from one copy of the last tree under `$P`, with a fresh
  `NOVA_HOME`, its output and exit status kept. The tree ends at the last
  commit, built. Task 3's and Task 4's runs must each equal the last tree's,
  or be refused before running (an `error[` and no output); so must the
  base's, where it runs. Expected:
  `progcmp: 147 programs; t3: 143 same, 4 refused, 0 differ; t4: 143 same, 4 refused, 0 differ; base: 140 same, 7 refused, 0 differ`,
  146 programs and the probe: none differs. The four Task 3 and Task 4
  refuse are Task 5's programs, `for_arrays`, `match_effects`, `patterns`
  and `patterns_async`: at Task 3 by E0900 for their `for` over an array, at
  Task 4 by the refusal, E0900 "matches with nested, tuple, or- or range
  patterns or guards are not supported yet". The base refuses those four,
  `tuples`, `tuples_async` and the probe. The `Char` probe's outputs,
  `base [exit=1 ] t3 [A B Z other exit=0 ] t4 [A B Z other exit=0 ] final [A B Z other exit=0 ]`,
  show Task 3's and Task 4's switch on its keys. The last tree's own runs
  match 136 of the 147 `.stdout` files; the others take the arguments, input
  or files their `run_tests.rs` test gives them, and every build's run of
  them is compared all the same.
- [ ] **Step 11: The review's inputs.** The full run's figures, the new
  tests by binary, the golden suite's line and time, the AST, `check` and
  E0102/E0103/E0104 counts, the programs at Tasks 3 and 4, and the mutants'
  outcome, with each gap, go in the ledger.
- [ ] **Step 12: One fresh read-only review, then one fix pass** (the user's
  workflow).
  - The reviewer reads `git diff $BASE..HEAD` (Task 0) against the spec, ADR
    0049 and this plan, and is told to hunt for:
    - a chain that jumps to the wrong block, tests a value twice or projects
      it twice, binds a local on one path of an or-pattern and not another,
      runs a guard before the bindings or enters the body on a false guard,
      or lowers a flat unguarded match other than by the switch it had
      (decisions 11, 13, 17 and 26);
    - usefulness accepting a match that misses a value, or reporting an arm
      or an alternative unreachable when a value reaches it; a witness that
      is not a pattern the program could write, that names a value twice or
      leaves one out; E0022 missing or extra (decisions 8, 15, 23, 28, 43
      and 44);
    - a pattern form checked against the wrong type; an or-pattern whose
      alternatives bind different locals or types without E0100; a range
      stored with the wrong end; a bare name read as a binding where it
      names a variant, or as a variant where it names none; the note on the
      wrong case; E0104 on a real variant, on `_Upper`, missing on an
      upper-case binding, twice in an or-pattern, or with the wrong note or
      guess; E0060's note naming a fix that does not parse where the name is
      bound (decisions 9, 18, 22, 24, 25 and 42);
    - the lexer splitting a float it should leave, or leaving one it should
      split; a tuple index accepted in another spelling; E0011 or E0014 on
      the wrong case (decisions 6, 14 and 20);
    - a walk over `Ty` without its tuple arm; two distinct tuple types with
      one mangling; a tuple reaching a backend as anything but a record; an
      impl or a trait bound accepting a tuple (decisions 1 and 12);
    - `for` over an array evaluating the array or its length twice, reading
      an element before the loop reaches it, or making `continue` skip the
      advance (decision 27);
    - a split judged quiet where the old reading was well typed, or loud
      where it was not; the operand read wrongly through a binary operator,
      a unary operator, a `return`, an assignment, a compound assignment's
      block, a closure's body or a `let` (decision 3);
    - a commit between Tasks 3 and 5 that lowers a nested or guarded match
      instead of refusing it, or a flat match other than as the last tree
      does, or Task 5 leaving the refusal in (decisions 26, 35 and 47);
    - fmt printing a layout that parses to another tree, or refusing a file
      it formatted (decision 30);
    - the records: ADR 0049, the CHANGELOG, the dated notes, the catalog's
      E0100 and E0101 against the messages;
    - each mutant of Step 9 that passes, and whether a check is added;
    - MSRV 1.78, a new crate, a change to `Cargo.lock`, a test that touches
      the internet.
  - Each finding is fixed in one commit, `5.0b: the review's fixes`, or
    answered in the ledger with the reason it stands.
  - After the fixes, Steps 1 and 2 run again.
- [ ] **Step 13: The PR.**
  - Push: `git push -u origin claude/loving-archimedes-frtf5q-50b`.
  - Open the PR to `main` with the repository's template headings, if it has
    one. The body names no local path and no person.
  - It holds:
    - what 5.0b adds: tuples and `t.N`, nested, guarded, or-, `Char` and
      range patterns, destructuring `let` and `for`, `for` over arrays,
      lowering by test chains, E0100, E0101 and the warning E0104, the E0103
      ruling, and ADR 0049;
    - the one way a program runs differently: a bare name inside a variant
      pattern that names a variant now matches it, Rust's rule, so
      `Some(Red)` no longer matches every `Some`, and behind a catch-all no
      check says so (decision 18); E0104 warns at an upper-case binding; no
      repository program writes such a pattern (the search of Step 5); every
      repository `.nova` file parses to the same tree, and checks the same
      but for two parser fixtures that use 5.0b's forms;
    - the figures: the suite's `2705 1 10`, the golden suite's
      `137 passed, 9 excluded`, and that `Cargo.lock` is unchanged;
    - the user's decisions and ruling, and this plan's decisions, numbered
      as here, with 17, 18, 24 and 26 marked as differing from the spec;
    - the mutants and the gaps they found, and the review's findings and
      what came of each.
  - Read CI once it ends. Expected: green on all three systems; on the
    `test` job's `ubuntu-24.04` leg, `cargo test`'s log holds
    `test release_golden_suite ... ok`. Record the run's address in the
    ledger.
  - **If the suite fails on CI and not here,** record its failures and stop
    and ask the user.
- [ ] **Step 14: The merge,** only on the user's word.
  - By rebase, pinned to the PR's head commit. Never auto-merge.
  - Then `git fetch origin main` and `git diff <head> origin/main --stat`,
    which prints nothing: the merged tree is the PR's.
  - Then read CI's run on `main` once it ends: green, as Step 13's.
