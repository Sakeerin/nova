# ADR 0008 — Attribute validation and test isolation for `@test`

Three decisions taken to ship `@test` / `nova test` (the increment
`.superpowers/sdd/2026-08-05-nova-test/` plans). They share a file because
all three answer the same underlying question — what may a program assume
`nova test` actually checked? — from three angles: section 1 is about
whether an attribute the compiler doesn't recognize can be written at all,
section 2 is about how a finished test process is judged, and section 3 is
about what happens to a user's own `fn main` once tests exist alongside it.

Section 3 was **not anticipated by this increment's design doc**. It surfaced
as a Critical defect during Task 3's implementation review — the design doc
described `nova test` as "compiling the program once, reusing `nova build`'s
existing path" and never asked what becomes of the entry point that path
already requires. This document is the first place the resolution is written
down.

All three are accepted, and each names a real loss rather than arguing it
away.

---

## 1. Attributes are item-level, arguments are bare identifiers, and an unknown one is always an error

### Status

Accepted (2026-08-06). Tasks 1-2 of this increment: `@name(args)` parses at
item position, and the resolver validates every attribute it finds.

### Context

`@test` needed *some* syntax, and the design settled on attributes attached
to an item — `@test` / `@test(should_panic)` immediately before a `fn` — with
arguments restricted to bare identifiers rather than arbitrary expressions or
`key = value` pairs. `KNOWN_ATTRIBUTES: &[&str] = &["test"]` and
`KNOWN_TEST_ARGS: &[&str] = &["should_panic"]`
(`crates/nova-resolver/src/lib.rs`) are the entire recognized vocabulary.

The question this section actually answers is what happens when an attribute
*outside* that vocabulary is written — `@tset` for `@test`, or
`@test(shuold_panic)` for the one accepted argument. Two shapes were
available: accept and ignore (the attribute parses, contributes nothing, and
compilation proceeds as if it were never written — how an unrecognized
attribute is commonly treated in languages that use them for optional
tooling hints), or reject outright.

### Decision

**An attribute name outside `KNOWN_ATTRIBUTES` is always `E0082`**, at every
site an attribute can be written — a function, record, type declaration,
trait, impl block, const, `import`, `module`, and `extern` block all carry an
`attrs` field and are validated uniformly
(`crates/nova-resolver/src/lib.rs`'s `collect_item`, whose `match` over
`Item` has no wildcard arm, so the compiler itself guarantees every item kind
is covered). `@test` on anything other than a function is `E0083`; a `@test`
function with parameters, generics, or a non-`Unit` return type is `E0084`;
an unknown `@test(...)` argument is `E0085`. None of the four is a warning
and none is suppressible.

**Why the "always an error" half matters more than the syntax half.** A
mistyped `@tset` that compiled successfully would define a function that
*looks like* a test — sits beside real `@test` functions, reads like one on a
skim — and never runs as one, because it was never collected. `nova test`
would report every *other* test passing and simply never mention it, which
is indistinguishable from a clean run to anyone not counting test names
against the source by hand. That is the worst member of a family this
project keeps finding and re-finding: a record-parameter bound that is a
resolution scope and not a constraint, silently accepted when no field type
uses it (ADR 0007 §1, case 3); an impl-level `const` parsed and discarded;
`pub` on a method accepted and having no effect; record field visibility
parsed and never enforced. Every one of those is "parses, then enforces
nothing" — but each of them fails *open* into behaviour a reader could
eventually notice (a bound that doesn't bound, a visibility that doesn't
restrict). A silently-ignored `@tset` fails open into a test that isn't
there, and a test that isn't there produces no symptom at all. This is why
the vocabulary is closed and violations are hard errors rather than the
softer, more forward-compatible option most attribute systems choose.

### Residual gaps, stated plainly rather than left to be inferred

- **Discovery follows the entry file's `import` graph, not the package.**
  `nova test` collects `@test` functions from `src/main.nova` and every
  module it transitively `import`s (`nova-driver`'s `load_modules`) — the
  same set of files an ordinary `nova build` of that entry point would
  compile — not by scanning the whole source tree for every file that
  defines one. A `@test` function in a module nothing imports is therefore
  never discovered: it does not run, is not counted, and the run still
  reports `running 0 tests` / `test result: ok` / **exit 0**, indistinguishable
  from a project with no tests in that file at all. `nova-spec/20-STDLIB.md`
  §11 used to claim `nova test` "discovers all `@test` functions across the
  package"; corrected alongside this document to describe the import-graph
  behavior instead. Package-wide discovery (walking the source tree rather
  than the import graph) would close this gap but is future scope, not a
  defect in what shipped — the risk is naming it here so a test file that
  quietly stops being imported does not silently stop being run.

### Consequences

- **Adding a new attribute is always a compiler change.** `KNOWN_ATTRIBUTES`
  and `KNOWN_TEST_ARGS` are Rust-side `const` arrays; nothing in Nova source
  can register a new attribute name or a new `@test` argument. A future
  `@bench` or `@derive` (`nova-spec/20-STDLIB.md` §7 already assumes one)
  needs a resolver change, not a library addition. That is the cost of
  closing the vocabulary, paid on every future attribute rather than once.
- **Every attribute site pays a small, uniform validation cost** — nine
  `Item` variants, nine `attrs` fields, nine call sites into
  `validate_attrs_reject_test` or `validate_test_function` — rather than the
  six vis-carrying item kinds this increment's plan originally scoped. The
  three left out at first (`import`, `module`, `extern`) were a real gap
  found during Task 2: `@tset` before an `extern` block compiled with *zero*
  diagnostics, because neither the AST nor the parser gave those three item
  kinds anywhere to record an attribute at all — the exact failure mode this
  decision exists to prevent, one layer further down than where the decision
  was first drawn.
- **A well-formed `@test` function is collected exactly once even if `@test`
  is written twice on it** (`crates/nova-resolver/src/lib.rs`'s dedup
  guard), so a doubled attribute cannot register a function as two tests
  under the same name.

### Alternatives considered

- **Accept and ignore an unrecognized attribute name**, the shape many
  attribute systems use so that a tool understanding a newer vocabulary
  doesn't break an older compiler. Rejected for the reason above: this
  project's error budget is spent on invisible failures, and a mistyped test
  attribute is the most invisible one available — nothing fails, nothing
  warns, the CI stays green with one fewer test than the source implies.
- **Arbitrary expressions or `key = value` pairs as `@test(...)` arguments**,
  which is closer to what richer attribute systems support. Rejected as
  unnecessary scope for what this increment needs (one boolean-shaped flag,
  `should_panic`) and deferred until a real second argument shape is needed.

---

## 2. Test isolation is process-per-test, classified by stderr rather than exit code — and `assert_throws` is not implementable

### Status

Accepted (2026-08-07). Task 5: the `nova test` runner.

### Context

A failing Nova program does not raise a catchable exception. `panic(msg)`
lowers to `nova_rt_panic_str`, which prints `nova: panic: <msg>` to stderr
and calls `std::process::abort()` — declared to return `!`
(`crates/nova-runtime/src/lib.rs`) — and there is no unwinding anywhere in
the runtime. So an in-process test runner (call each `@test` function in a
loop, inside one `nova test` process) cannot survive a failing test to
report on the next one: the first panic ends the whole process. It also
cannot distinguish a checked panic from a genuine miscompile, because after
an abort there is no process left to inspect.

Running each test in its own process and classifying it from the *outside* —
by exit status and stderr — is what makes both problems disappear at once.
The question this section actually answers is what to classify *on*: the
exit code, or something else.

### Decision

**Every `@test` runs in its own process** (`NOVA_TEST_INDEX=<i>` against the
one compiled binary, `crates/nova-cli/src/cmd/test.rs`), and **the
discriminator between a checked panic and an unrelated crash is a substring
on stderr, never the exit code.** `classify` checks `output.status.success()`
first (clean vs. unclean), and only then asks stderr whether it contains
`nova: panic:` — unclean *and* the marker present is `Panicked`; unclean and
absent is `Trapped`. The exit code is carried on `Trapped` for display only;
nothing in the classification reads its value.

**Why the exit code specifically is disqualified, stated as measurement
rather than caution.** For the `1 / 0` trap itself, on this machine, through
the identical build, exactly two values were ever measured — not three, and
not a set of mutually disagreeing figures:

| Source | Value | What it observed |
|---|---|---|
| Git Bash, `$?` after running the program | `132` | the shell's POSIX-style translation of the illegal-instruction trap |
| Rust's own `std::process::Command`, `.status.code()` | `-1073741795` (`0xC000001D`, `STATUS_ILLEGAL_INSTRUCTION`) | the raw Windows exit value, unmediated |

Independently re-measured while writing this ADR, on this branch's own gate
fixture (`tests/runtime/nova_test.nova`'s `division_by_zero_traps`): raw
exit code `-1073741795` through Rust's process API, bit-for-bit the same
value Task 5 measured on a different fixture. **These two numbers are not
even clearly a disagreement.** Git Bash reports a process killed by a
signal as `128 + signal`, and `132 = 128 + 4`, where `4` is `SIGILL` under
the POSIX signal numbering Git Bash emulates — consistent with
`STATUS_ILLEGAL_INSTRUCTION` on the Windows side and with Git Bash's own
"Illegal instruction" wording for this identical program (Task 5's
measurement). Read that way, `132` and `0xC000001D` are plausibly two
encodings of one fact — an illegal-instruction trap — observed through two
different layers, not two figures that contradict each other.

*(An earlier draft of this section also folded in `0xC0000409` and
`0xC0000005` as though they were further measurements of this same `1 / 0`
trap. They were not, and citing them here was this ADR's own mistake,
corrected rather than left for a reader to re-discover: both figures come
from a *different* investigation entirely — Task 4's report on a genuine
`assert`/`assert_ne` panic's `std::process::abort()` exit
(`0xC0000409`) and a separate, unexplained, non-reproducing crash
(`0xC0000005`, `STATUS_ACCESS_VIOLATION`) — on different fixtures, neither
of them `1 / 0`. They are real numbers from real investigations on this
branch, just not evidence about *this* trap; see Task 4's and Task 5's own
reports for what they actually measured.)*

The argument against using the exit code does not depend on the two
`1 / 0` figures disagreeing, and is simpler than that framing suggested:
**the exit code alone cannot distinguish a checked panic from a hard
trap**, whether or not a given pair of measurements happens to agree. A
legitimate Nova panic also exits nonzero — `nova_rt_panic_str` calls the
same `std::process::abort()` a trap never reaches at all — so a classifier
keyed on "which specific nonzero value is this" would need a maintained,
platform-specific table of abort-shaped codes, with no guarantee a future
trap or panic doesn't produce a new one. `nova: panic:` on stderr, by
contrast, is written by the runtime itself, under Nova's own control, on
exactly the outcomes that should count as a panic — which is what makes it
the reliable signal, independent of exit-code agreement.

**Correcting a claim that would otherwise have been enshrined here: the
marker is not emitted only by `nova_rt_panic_str`.** Three call sites print
it independently, none delegating to another:

- `nova_rt_panic_str` (`crates/nova-runtime/src/lib.rs`) — user `panic(...)`
  and every `std/test` assertion (`assert`, `assert_eq`, `assert_ne`, all
  three built on `panic`).
- `nova_rt_check_bounds` (same file) — an array index outside `0..len`.
- `gc::alloc`'s oversized-allocation guard (`crates/nova-runtime/src/gc.rs`)
  — a size past `MAX_HEAP_OBJECT`.

This is load-bearing rather than incidental. `tests/runtime/nova_test.nova`'s
`array_out_of_bounds_panics` — the fixture's `should_panic` test that is
*supposed* to pass — reaches the marker through `nova_rt_check_bounds`
alone; it never calls `nova_rt_panic_str`. A classifier keyed to one specific
emitter (checking, say, that the message looks like a `panic(...)` call)
rather than a plain substring search over all of stderr would misclassify
that test as a trap. `classify`'s `stderr.lines().find(|line|
line.contains(PANIC_MARKER))` catches all three uniformly because it was
written to, not by accident.

**`assert_throws` (`nova-spec/20-STDLIB.md` §11) is therefore not
implementable under this design and is not provided.** Catching a panic to
compare it against an expected value needs unwinding to return control to
the calling test after the panic; there is no such mechanism anywhere in
this runtime, by design (`docs/adr/0002-phase1-leaking-allocator.md`'s
successors never added one, and process-per-test isolation above is built
specifically *because* there isn't one). `@test(should_panic)` — checking
that a test panics, without resuming after it and without inspecting the
value — is what process isolation *can* support, and is the only supported
way to assert that something panics.

### Residual gaps, stated plainly rather than left to be inferred

- **No per-test timeout.** A test that hangs is indistinguishable from one
  that is merely doing slow work; `nova test` will wait on that one
  subprocess forever. Not hypothetical: `impl Iterator for Int`'s `next`
  copies rather than advances a primitive `Self` (ADR 0007 §2), so
  `count()`/`fold`/`collect` on such an iterator never terminates — a real,
  already-shipped way to write a test that hangs `nova test` with no
  diagnostic.
- **A filter matching zero tests exits 0.** `nova test <typo>` prints
  `running 0 tests` and `test result: ok. 0 passed; 0 failed; 0 trapped; 0
  total`, then exits success — idiomatic (it matches `cargo test`'s own
  behaviour) but it means a typo'd CI filter silently reports green having
  run nothing.
- **The stderr rule is sound only while every marker-emitting site aborts
  immediately after printing it**, which is true of all three sites today
  (each is `eprintln!` immediately followed by `std::process::abort()`, with
  nothing observable in between) and is enforced by nothing for whatever the
  fourth site turns out to be. A future runtime addition that prints a line
  containing `nova: panic:` for some other reason, without aborting
  immediately, would be misread by `classify`'s plain substring search.

### Consequences

- **A test's process, not its return value, is the unit of result.** There
  is no way to write a Nova-level "does this raise an error" assertion other
  than `@test(should_panic)` on the whole function.
- **Every test pays a process-spawn cost.** For the small suites this
  compiler currently runs, that cost is not something this increment
  measured as a problem; a future large suite might reconsider it, but not
  by weakening the isolation this decision buys.
- **A hard trap and a checked panic are reported distinctly** —
  `TRAPPED (exit code N)` versus a checked panic's `FAILED` plus its message,
  or `ok` under `should_panic` — and `@test(should_panic)` inverts *only* the
  `Panicked` row. A trap is a failure whether or not `should_panic` is set.

### Alternatives considered

- **Classify by exit code** (e.g., a specific abort code means "panic").
  Rejected by the measurement above: no single code is stable even across
  three ways of observing the *same* program on the *same* machine, let
  alone portable to another platform.
- **An in-process runner with `catch_unwind`-style recovery.** Not available:
  `nova_rt_panic_str` calls `std::process::abort()`, which cannot be caught
  by anything, in-process or not. Building a catchable panic path would be a
  runtime-wide unwinding mechanism — far larger than this increment, and its
  own long-term question independent of testing.
- **Implement `assert_throws` by re-running the throwing call in a
  sub-subprocess and inspecting its output**, approximating "catch" via a
  second layer of process isolation *inside* a test body. Rejected: it only
  works for a call that can be isolated as its own entry point, doesn't
  compose (a test wanting to assert two different calls throw would need two
  nested binaries), and papers over the real answer, which is that this
  runtime has no unwinding.

---

## 3. Under `nova test`, a user's own `fn main` is shadowed, not the entry point

### Status

Accepted (2026-08-06), during Task 3's implementation, as the fix for a
Critical defect found in that task's review. Recorded here because the
design doc never anticipated the question and this is the first place the
resolution is written down.

### Context

`build_test_binary` synthesizes a dispatching `main` — `test_selector()`
into a local, then one `if sel == i { test_i() }` per collected test, falling
through to an inventory printer when `sel` is negative — and appends it to
`module.functions`. Monomorphization finds the entry point with
`module.functions.iter().find(|f| f.name == "main")`
(`crates/nova-mir/src/mono.rs:19`) — the *first* function named `"main"`.

Those two facts collide exactly when the source being compiled for testing
declares its own `fn main`. Appending puts the synthesized dispatcher
*last*; `.find` returns the *first* match. So a user's pre-existing `main`
won, unconditionally: the dispatcher became dead code monomorphization never
even reached, and the program that resulted was the user's own — compiling
and linking cleanly, with no diagnostic of any kind. Measured directly (Task
3's review): a source file with one `@test` function and its own `fn main()`
printing a distinct marker ran the `main`'s marker under
`NOVA_TEST_INDEX=0`, exit 0.

**This is not an edge case.** `nova build` and `nova run` both *require* a
`main` and reject its absence with `E0601`
(`crates/nova-mir/src/mono.rs`'s own diagnostic). A program that has both
`@test` functions and an ordinary entry point is therefore the unremarkable
case for anything that was a real program before it grew tests, not a rare
corner. Left unfixed, `nova test` would have silently run the wrong program
for the single most common shape of input it would ever see.

### Decision

**Every pre-existing function named `main` is renamed to
`"main.shadowed_by_nova_test"` before the synthesized dispatcher is pushed**
(`crates/nova-driver/src/lib.rs`'s `SHADOWED_USER_MAIN_NAME`,
`build_test_binary`), so the dispatcher is the only function left named
`"main"` and `mono`'s `.find` cannot help but select it.

Four parts, each deliberate:

- **Every one, not only the first.** The module system merges the entry
  file and every module it imports into one flat `hir::Module`, and
  duplicate-definition checking runs *per module*
  (`nova-resolver`'s own `same_name_in_two_modules_does_not_collide`
  precedent), so more than one function literally named `main` can coexist
  after merging — an entry file's `main` plus an imported module's `main` is
  a real, constructible program, not a hypothetical. The rename loop is an
  unconditional `for f in &mut module.functions`, not a `.find`-and-rename
  of the first match, specifically because fixing only the first would
  leave a second pre-existing `main` to win the identical bug one level
  down. Verified directly: a two-`main` probe (one in the entry file, one in
  an imported module) had both renamed and the dispatcher ran.
- **The new name contains a `.`.** The lexer's identifier grammar is
  `[a-zA-Z_][a-zA-Z0-9_]*` (`crates/nova-lexer/src/lib.rs`) — no `.` can
  ever appear inside one `Ident` token — so `"main.shadowed_by_nova_test"`
  is a name no Nova source can write, verified against the grammar rather
  than merely assumed. It can never collide with a name a user actually
  chose, the same technique `nova_mir::mangle`'s `name.<def_id>` symbols and
  the `$std.core` / `$prelude` internal module names already rely on.
- **Renamed, not deleted.** Anything that still refers to the function by
  `DefId` — which is how every call site resolves, never by name, once
  resolution has run — keeps resolving correctly; only the *name* changes.
  The function itself simply becomes unreachable from the new `main`, and
  monomorphization already prunes whatever `main` cannot reach, so no dead
  code reaches codegen.
- **The fix lives where the module is *assembled*** (`build_test_binary`, in
  `nova-driver`), **not in how the entry point is *discovered***
  (`mono`'s `.find(|f| f.name == "main")`, in `nova-mir`). Changing the
  lookup itself would alter how *every* consumer of `nova-mir` locates an
  entry point — `nova run`, `nova build`, `nova check` indirectly — for the
  benefit of exactly one subcommand. Renaming at assembly time confines the
  entire mechanism to the one function that needs it.

### Consequences

- **Under `nova test`, your `main` does not run — stated plainly, because a
  user reading only the feature's happy path could otherwise be surprised by
  it.** A file compiled for testing runs exactly one `@test` function per
  process, selected by `NOVA_TEST_INDEX`; whatever `fn main` the file also
  declares is compiled, becomes unreachable, and is never executed by
  `nova test` under any index.
- **A function *inside an impl* literally named `main` is swept by the same
  loop** (it iterates `module.functions`, which a plain top-level rename
  cannot distinguish from an impl method sharing the name), but this has no
  observable consequence: a method is called via `Callee::Def(DefId)`
  resolved before the rename runs, and `mangle` appends the `DefId`
  regardless of name, so no symbol collision is possible even in that case.
- **Every typeck pass that inspects a function's name runs *before* this
  rename**, since it operates on the already-checked `hir::Module` handed
  back from `FrontendContext::check`; only `mono`'s entry-point search and
  codegen's symbol table ever see the renamed form. No diagnostic anywhere
  reports on the pre-rename or post-rename name inconsistently.
- **This is a hand-built-HIR mechanism, not a source-level one.** There is
  no `@test`-adjacent attribute or opt-out for a user who genuinely wants
  their own `main` to run under `nova test` — the shadow is unconditional
  whenever `build_test_binary` is the entry point, which is every `nova
  test` invocation.

### Alternatives considered

- **Change `mono`'s entry-point lookup** to skip a `main` when the module
  also carries a synthesized test dispatcher (e.g., a flag on the function,
  or a different sentinel name for the *dispatcher* instead of the user's
  function). Rejected: it moves the special case into code every other
  subcommand also runs through, in exchange for no benefit — the rename
  achieves the identical outcome from the side that actually has the
  context (`build_test_binary` already knows it is building for tests;
  `mono` does not and should not need to).
- **Reject a source file that declares its own `main` under `nova test`**,
  forcing the user to remove or rename it. Rejected: it would make testing a
  program with `@test` functions and a real entry point — the ordinary case,
  per the context above — an error, which is a worse outcome than silently
  shadowing a function nothing under `nova test` was ever going to call
  anyway.
- **Delete the user's `main` outright** instead of renaming it. Rejected:
  nothing currently calls a user's `main` by name or `DefId` from elsewhere
  in a well-formed program, so the practical difference is small today, but
  renaming costs nothing extra and keeps the function resolvable for
  whatever future change might reference an entry point that isn't the
  active one (e.g., a diagnostic that wants to say "your `main` was here").

---

## References

- Plan: `.superpowers/sdd/2026-08-05-nova-test/`
- `crates/nova-resolver/src/lib.rs`: `KNOWN_ATTRIBUTES`, `KNOWN_TEST_ARGS`,
  `unknown_attribute`, `validate_attrs_reject_test`, `validate_test_function`
  (§1); `TestFn` (§1, §3)
- `crates/nova-cli/src/cmd/test.rs`: `Outcome`, `PANIC_MARKER`, `classify`
  (§2)
- `crates/nova-runtime/src/lib.rs`: `nova_rt_panic_str`, `nova_rt_check_bounds`
  (§2); `crates/nova-runtime/src/gc.rs`: `alloc`'s oversized-object guard (§2)
- `crates/nova-driver/src/lib.rs`: `SHADOWED_USER_MAIN_NAME`,
  `build_test_binary`, `synthesize_test_main` (§3)
- `crates/nova-mir/src/mono.rs:19`: the entry-point lookup `main.shadowed_by_
  nova_test` deliberately still loses to (§3)
- Gate: `tests/runtime/nova_test.{nova,stdout}`; the user-`main` fix is pinned
  by `a_test_binary_runs_the_selected_test_not_the_users_own_main`
  (`crates/nova-cli/tests/run_tests.rs`)
- Spec: `nova-spec/20-STDLIB.md` §11 (corrected alongside this document —
  `should_panic`'s example, `assert_throws`'s status, and the "discovers all
  `@test` functions across the package" claim §1's residual gaps corrects
  above);
  `nova-spec/50-TESTING.md` §§1.1-1.2 and 2.1 (the `tests/compile-pass` /
  `tests/compile-fail` harness) and separately §1.5 (the `tests/ui` harness,
  a different mechanism — WASM/Playwright, not §2.1's Rust integration
  test) — neither is implemented or replaced by this increment; see the
  `CHANGELOG` entry
- Related: ADR 0007 §2 (a primitive `Self`'s `next` copying rather than
  advancing — the residual-gaps hang example in §2 above)
