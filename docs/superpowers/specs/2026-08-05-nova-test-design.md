# `nova test`, `@` attributes and `std/test` — Design

**Status:** approved 2026-08-05. Phase 2.5's test runner, pulled forward ahead of 2.3 (async).

**Base:** `main` at `884004c` (Phase 2.2d merged and pushed; 640 tests, 15 gate configurations).

---

## 1. Why this, and why now

Nova's standard library is 1,286 lines of Nova across three modules, and **there is no way to
write a test in Nova**. Every assertion about std lives in Rust: either a `#[test]` in
`crates/nova-cli/tests/run_tests.rs` driving a temp file through the CLI, or a
`tests/runtime/*.nova` fixture whose entire contract is "stdout matches this file".

That separation is the direct cause of this project's most persistent defect class. Phase 2.2b
shipped eight tasks out of ten where a one-character mutation survived the task's own tests.
Phase 2.2d produced seven instances of a diagnostic measured on one code shape being written up as
the answer for a different shape, by five different authors. In both cases the assertion was
written far from the thing it described — in another language, in another file — and the mismatch
was invisible until someone mutated the code.

A fixture that compares stdout can only ever say "the whole program printed the wrong bytes". It
cannot say "`Map::grow` kept a tombstone it should have dropped". The gate fixtures are valuable
and stay, but they are the wrong instrument for the majority of what std needs checked.

`nova test` is therefore not a convenience feature. It is the instrument this project has been
missing, and every later phase — async most of all, where the interesting bugs are races that a
stdout diff cannot see — depends on having it.

### 1.1 Probe table

Measured on `884004c` unless marked otherwise. **The two unmeasured rows are the design's real
risk and Task 1 must probe them before anything depends on them.**

| Question | Answer |
|---|---|
| Does `@` lex today? | **No.** `LexError::UnexpectedCharacter('@', span)` — `error.rs:30`. Non-fatal: the lexer collects it and continues, so adding the token needs no change to the error path. |
| Does Nova have any attribute syntax? | **No.** No `#[…]` or `@…` in `nova-lexer` or `nova-ast`. |
| What does `panic(msg)` do? | Prints `nova: panic: <msg>` to stderr, then `std::process::abort()` (`nova-runtime/src/lib.rs:287`, returns `!`). **Exit 127.** No unwinding anywhere in the runtime. |
| Array out of bounds? | `nova: panic: array index 7 out of bounds for length 3`, **exit 127** — a checked panic. |
| Integer division by zero? | **Exit 132, illegal instruction, no message.** *Not* a checked panic. This falsifies `20-STDLIB.md` §11's own `@test(should_panic)` example, which is `let _ = 1 / 0`. |
| Can the runtime read environment variables? | Yes — `std::env::var_os` for `NOVA_GC_STRESS` and `NOVA_GC_DEBUG` (`nova-runtime/src/gc.rs:102,107`). |
| How is `main` located? | By name: `module.functions.iter().find(|f| f.name == "main")` (`nova-mir/src/mono.rs:19`). Nothing requires it to come from user source. |
| Is a synthetic `hir::Function` named `main` constructible? | Yes — `nova-mir/src/mono.rs:648` builds one in its own unit tests, which fixes the required shape. |
| How are curated intrinsics declared? | The `builtins!` macro at `nova-resolver/src/lib.rs:62` generates `Builtin::ALL`; `STD_ONLY: [Builtin; 8]` (`:169`) is the subset seeded only into std modules' scopes, not user scopes. |
| CLI subcommand shape | `enum Command { Parse, Run, Build, Check }` at `nova-cli/src/main.rs:24`, clap-derived. |
| Can a free generic fn with two bounds call a method from each? | **Yes.** `fn assert_eq2<T: Eq + Debug>(a: T, b: T)` calling `a.ne(b)` and `a.dbg()` compiles and runs, instantiated at `Int`, `Bool` **and** `String` — three distinct `MirTy` classes (`I64`, `I8`, `Ptr`). No free function in std had two bounds before (`impl<T: Hash + Eq>` at `std/collections/lib.nova:347` was the closest precedent), so this was the design's largest unknown. |
| Can `${a.dbg()}` interpolate? | **Yes.** `panic("assertion failed: ${a.dbg()} != ${b.dbg()}")` produces `nova: panic: assertion failed: 1 != 3`, exit 127 — landing on the "panicked" row of §5 exactly as required. |

---

## 2. Scope

**In:**

- `@name` and `@name(arg, …)` attribute syntax on items — lexer, AST, parser, resolver.
- `@test` and `@test(should_panic)`, collected by the compiler.
- A synthesized `main` that dispatches to one test by index.
- `nova test [filter]` — compile once, run one process per test, report.
- `std/test` with `assert`, `assert_eq`, `assert_ne`.

**Out, each for a stated reason:**

- **`assert_throws`** (spec §11) — **not implementable.** It must catch a panic; panic aborts and
  there is no unwinding. `@test(should_panic)` is the substitute and works only because the check
  is at process level.
- **`@bench`** — needs timing, which needs `std/time`, which is phase 2.3.
- **Parallel execution** (spec §11 says "in parallel") — deferred, not blocked. Once each test is
  already its own process, parallelism is a scheduling change in the runner and nothing else.
- **`nova test --doc`** — blocked upstream: `///` does not parse in Nova at all.
- **Populating `tests/compile-fail`, `tests/compile-pass`, `tests/ui`** — those three directories
  are committed and empty, and `50-TESTING.md` §2.1 specifies a harness for them. Worth doing, but
  it is a Rust-side harness rather than a language feature, and mixing the two doubles this
  increment. Its own increment, after this one.

---

## 3. Attribute syntax

A new `Token::At`, and on each item an `attrs: Vec<Attribute>` where
`Attribute { name: Ident, args: Vec<Ident>, span: Span }`.

```nova
@test
fn add_works() { assert_eq(1 + 1, 2) }

@test(should_panic)
fn out_of_bounds_panics() { let xs = [1, 2, 3]  let _ = xs[7] }
```

Three deliberate restrictions:

- **Item-level only.** Not on expressions, statements, fields, or trait members. Nothing in the
  spec needs them there — `@test`/`@bench` are on functions, `@derive` on type declarations.
- **Arguments are bare identifiers.** Not expressions, not literals. The spec's only two forms are
  `@test(should_panic)` and `@derive(Copy, Clone)`, both lists of identifiers. Widening this later
  is additive; narrowing it would not be.
- **An unknown attribute is a hard error (`E0082`).** Not a warning, not ignored. This project's
  single most-repeated defect is "parses, then silently enforces nothing" — record-parameter
  bounds, impl-level `const`s, `pub` on methods, record field visibility. A mistyped `@tset` that
  compiles to a test that never runs would be the worst possible instance of it, because the
  failure mode is *a test that appears to exist and does not*.

The known-attribute set starts as `{test}`. `@bench` and `@derive` join it when their features
land; until then they are `E0082`, which is honest.

The cost of the third choice: adding an attribute is always a compiler change, never a library
convention. Given `@derive` is a type-system feature and `@bench` needs a runner, none of the
spec's attributes could have been library-defined anyway.

### 3.1 Where attributes are rejected

`@test` is only meaningful on a zero-argument, zero-generic, `Unit`-returning free function. Each
of these is a separate diagnostic rather than one catch-all, because a catch-all here would repeat
`E0900`'s existing problem of reporting "not supported" when the real fault is specific:

- on anything other than a function → `E0083`
- on a function with parameters, generics, or a non-`Unit` return → `E0084`
- `@test(x)` where `x` is not `should_panic` → `E0085`

---

## 4. Collection and the synthesized `main`

The resolver records which definitions carry `@test`, in source order. Order matters and must be
stable: the runner addresses tests **by index**, so a reordering between the enumeration run and a
test run would run the wrong test.

`nova test` then synthesizes, at **HIR level**, a function named `main`:

```
fn main() {
    let sel = test_selector()
    if sel == 0 { <test 0>() }
    if sel == 1 { <test 1>() }
    …
    if sel < 0 { println("<count>") ; println("<name 0>") ; … }
}
```

**Why HIR and not generated Nova source.** Generating source is how `std` reaches a program and it
is tempting here, but it would route the call through the resolver, which enforces `pub` — and
test functions are not `pub`. Constructing `hir::Function` directly sidesteps visibility, and
because `mono.rs:19` finds `main` by name, **MIR, monomorphization and both backends need no
changes at all.** That is the property that keeps this increment small.

`test_selector() -> Int` is a new `STD_ONLY` builtin reading an environment variable, following
`str_cmp` exactly: a `Res::Builtin` checked in typeck and lowered to a `CallRuntime` in MIR.
`STD_ONLY` keeps it out of user scopes, so it introduces no name a program could collide with.

**The enumeration sentinel is a negative index**, not merely "out of range". A negative value is
unambiguous and needs no knowledge of the test count to produce; "greater than or equal to the
count" would require the runner to already know the count it is asking for. An absent or
unparseable environment variable reads as negative, so running the test binary directly — with no
variable set at all — prints the test inventory rather than silently doing nothing.

---

## 5. The runner

`nova test [filter]` compiles the program **once** to a standalone executable, reusing `nova
build`'s existing path, then:

1. Runs it with an out-of-range selector to read the test count and names. One extra process, no
   second compilation.
2. Filters names by substring (`40-TOOLING.md:20`).
3. For each selected test, runs the binary with that index and observes the result.

Three outcomes, and keeping them distinct is the entire justification for process isolation:

| Observation | Verdict |
|---|---|
| exit 0 | **passed** |
| nonzero exit **and** stderr contains `nova: panic:` | **panicked** — message reported |
| nonzero exit **and** no such line on stderr | **hard trap** — reported as such, distinctly |

**The discriminator is stderr, not the exit code** (see risk 4). 127 and 132 were the values
measured here, but the mapping from `abort()` to an exit code is platform-dependent, whereas
`nova: panic:` is emitted by `nova_rt_panic_str` itself and is therefore the portable signal. The
exit code says whether the run was clean; stderr says *how* it was unclean.

`@test(should_panic)` passes on the middle row **only**. A hard trap is not a panic: exit 132
means the program executed an illegal instruction, which is what a miscompile looks like. Treating
132 as "it panicked as expected" would let a codegen bug masquerade as a passing test. This is not
hypothetical — Phase 2.2d found a real case where deleting one guard turned a clean type-check into
an exit-132 crash on legal source.

An in-process runner cannot make this distinction at all: after an abort there is no process left
to record anything.

---

## 6. `std/test`

A fourth embedded std module. Signatures follow `20-STDLIB.md` §11:

```nova
pub fn assert(cond: Bool, msg: String)
pub fn assert_eq<T: Eq + Debug>(a: T, b: T)
pub fn assert_ne<T: Eq + Debug>(a: T, b: T)
```

Failure calls `panic`, so a failing assertion is exit 127 with a message — the "panicked" row
above. `assert_eq`'s message reports both values via `dbg` (ADR 0004: `Debug`'s method is `dbg`,
not `fmt`).

**`std/test` is seeded only when compiling under `nova test`.** A top-level `pub fn assert` in an
always-embedded module would be glob-imported and take the names `assert`, `assert_eq` and
`assert_ne` in every module of every program. That is the hazard that made `join` hang off the
separator (`",".join(parts)`) rather than being a free function. The cost is that `assert` is
unavailable in ordinary code; `panic` already serves that need.

---

## 7. Diagnostics

| Code | Meaning |
|---|---|
| `E0082` | unknown attribute — names the attribute and lists the known set |
| `E0083` | `@test` on something that is not a function |
| `E0084` | `@test` on a function with parameters, generics, or a non-`Unit` return |
| `E0085` | unknown `@test` argument — only `should_panic` is accepted |

`E0082` onward was free as of Phase 2.2d, which used codes up to `E0081` plus `E0403`, `E0428`,
`E0601`, `E0900`, `E0902`.

---

## 8. Gate

A Nova file under `tests/` exercising, with `nova test` output asserted:

1. A passing test.
2. A failing `assert_eq` — the reported message must contain **both** values, so a message that
   dropped one is visible.
3. A `should_panic` test that panics via a checked path (array out of bounds), passing.
4. **A test that divides by zero: reported as a hard trap, and explicitly *not* satisfying
   `should_panic`.** This is the discriminating line — it is the only gate item that fails if the
   runner collapses exit 127 and exit 132, and it is why `20-STDLIB.md` §11's example must be
   corrected rather than copied.
5. `nova test <filter>` selecting a strict subset, with the unselected tests confirmed *not* run.
6. An unknown attribute rejected as `E0082`.
7. The test binary under `NOVA_GC_STRESS=1`.

Item 4 is the one to protect. Items 1–3 would all pass against a runner that treated any nonzero
exit as "failed", which is precisely the design this document rejects.

---

## 9. Risks

1. ~~**The two unmeasured probe rows.**~~ **Closed before planning.** Both were measured after this
   design was approved: `assert_eq<T: Eq + Debug>` is buildable exactly as `20-STDLIB.md` §11 spells
   it, verified at `Int`/`Bool`/`String`, with `${a.dbg()}` interpolating correctly and a failure
   landing on §5's "panicked" row. The contingency this risk described — splitting into
   single-bound helpers or a concrete-type-per-primitive set — is not needed. Recorded rather than
   deleted, because the *reason* it was a risk still holds for the next such signature: no free
   function in std had two bounds, so this was unprecedented rather than merely untested.
2. **Test index stability.** The runner addresses tests by index across separate processes. Any
   nondeterminism in collection order — a `HashMap` iteration anywhere on the path — silently runs
   the wrong test and reports the wrong name. Collection must be source-ordered and that ordering
   must be pinned by a test, not assumed.
3. **A fourth embedded module.** Phase 2.2b's third module needed only a length annotation
   changed, because every consumer iterates `STD_MODULES`. A conditionally-seeded fourth module is
   new configuration: it is the first module that is *not* always present, so anything that assumes
   a fixed module set is now wrong.
4. **Exit-code portability.** 127 and 132 were measured on Windows through Git Bash. The mapping
   from `abort()` to an exit code is platform- and shell-dependent, so the runner must not
   hard-code 127 as "panic". The reliable signal is *stderr contains `nova: panic:`*; the exit code
   distinguishes clean from unclean but should not be the discriminator on its own.
5. **A test that hangs.** Nothing here bounds runtime. `impl Iterator for Int` already produces a
   non-terminating `count()` (Phase 2.2d, ADR 0007). A per-test timeout is not in scope, but the
   runner must not be the reason a hang is indistinguishable from slow work — note it.

---

## 10. Definition of done

- `@test` and `@test(should_panic)` parse, resolve, and are rejected with specific codes where
  meaningless.
- `nova test [filter]` reports pass, fail-with-message, and hard-trap distinctly.
- `std/test`'s three assertions work and are unavailable outside `nova test`.
- All seven gate items pass, item 4 included.
- `20-STDLIB.md` §11's `should_panic` example corrected, with the div-by-zero finding recorded.
- ADR: attributes are item-level with identifier arguments and unknown ones are errors;
  `assert_throws` is not implementable without unwinding.
- The three empty `tests/compile-*` directories: either populated or removed, not left as dead
  scaffolding a reader mistakes for coverage.
- 640 existing tests still pass; 15 existing gate configurations still green.
