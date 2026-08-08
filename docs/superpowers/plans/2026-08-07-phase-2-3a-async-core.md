# Async core — `async`/`await`, `Future<T>` and `std/task` — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make `async fn` and `.await` work in Nova — lowered to resumable state machines, driven by a single-threaded cooperative executor, with `std/task`'s `spawn`/`join`/`yield_now`/`block_on`.

**Architecture:** Five layers, and the parser is not one of them — `async` and `.await` already parse. A new `Ty::Future(Box<Ty>)` types `async fn f() -> T` as `fn() -> Future<T>` and `e.await` as `T`. A **post-monomorphization pass over MIR** rewrites each async function into a poll function whose environment *is* a heap state object holding the resume tag, the output, and every temp. A single-threaded executor in `nova-runtime` drives poll functions from a ready queue. A **persistent GC root registry** keeps suspended task states alive, because they sit on no Nova stack. `std/task` becomes the fourth embedded std module.

**Tech Stack:** Rust (nova-hir, nova-typeck, nova-mir, nova-runtime, nova-resolver, nova-driver), Nova (`std/task`), `assert_cmd` e2e fixtures.

**Spec:** `docs/superpowers/specs/2026-08-07-phase-2-3a-async-core-design.md`. **Read §1.1's probe table before starting** — eight measured rows, two of which falsify claims in `docs/phase-2-plan.md`.

**Base:** `main` at `0a0bc25`. Create branch `async-core`.

## Global Constraints

Every task's requirements implicitly include this section.

- **THE DOCUMENTS ARE LESS RELIABLE THAN THE CODE.** On the 2.2e branch, nine claims in task briefs were falsified by measurement, and **every implementer who contradicted one was right** — including a BLOCKED report that corrected a plan constraint and a Critical that corrected the design doc. If this plan says something the code contradicts, **the code wins**: report it, correct it, and proceed. Do not implement a thing you have measured to be wrong.
- **`cargo build --workspace` BEFORE `cargo test`.** `cargo test` does not regenerate `nova-runtime`'s staticlib (`target/debug/nova_runtime.lib`), which `nova build` links against. This task adds `nova_rt_*` symbols, so skipping it makes ~25 `*_build_standalone` tests fail with an MSVC `unresolved external symbol` that reads exactly like a codegen bug. Measured; CI shares the hole.
- **`--no-fail-fast` is mandatory** on `cargo test --workspace`, or cargo abandons later test targets on the first failure and under-reports.
- **A zero-match `cargo test <filter>` EXITS 0.** Check the `running N tests` line is non-zero before treating a filtered run as evidence. This has produced false "verified" claims three times on this project. `cargo test -p <crate> --lib --exact <bare_name>` matches **zero** tests — `--exact` needs the fully-qualified path (`check::tests::<name>`).
- **`cargo test --workspace` rebuilds `nova.exe`**; `cargo test -p <crate>` does not. `std/` is embedded via `include_str!`, so after editing `std/` or reverting a mutation, run `cargo build --workspace` before any `nova run`/`nova check` probe. This cost agents real findings on two previous branches, in both directions.
- **Baseline: 688 tests passing, 0 failed; clippy `-D warnings` and `cargo fmt --check` clean; 18 gate configurations green.** Take your own baseline; do not trust a number written here.
- **Assert content, not just that something failed.** Every Important finding on the last three branches was a test specified in a plan that checked an error code, a count, or a single character instead of the thing that matters. Before writing any assertion, ask **what one-character change to the implementation survives it**. A test over only *passing* inputs is the worst case — an all-passing-inputs test would survive `Eq::ne` returning `false`.
- **A monomorphization-seam test is only as good as the `MirTy` classes it instantiates at.** `mir_ty` (`crates/nova-mir/src/lib.rs:445-457`) maps `Int` *and* `Char` to `MirTy::I64`, and `String`/`Fn`/`Sum`/`Record`/`Array` to `MirTy::Ptr` = `i64` on x86-64. Only `Bool` (`I8`) and `Float` (`F64`) are disjoint from both, and **`Float` is strictly stronger** because it crosses register banks while `Bool`'s 0/1 survive an `I64` confusion intact. **Every generic async test instantiates at `Float`.**
- **Nova language limits, all measured on earlier branches:** no `loop` (use `while true`); no tuples; no references; `///` lexes to `Token::DocComment` but the parser attaches it to nothing; `(self.f)(x)` is `E0014` — bind `let g = self.f` first; a record literal cannot be a `for` iterable; an empty `Vec::new()` needs a type annotation; a bare trailing `-1` after a `}`-closed block parses as subtraction against the block — use `return -1`.
- **Diagnostic codes in use:** `E0001`, `E0002`, `E0010`–`E0016`, `E0020`–`E0022`, `E0060`, `E0070`–`E0085`, `E0207`, `E0403`, `E0428`, `E0601`, `E0900`, `E0902`. This plan adds `E0086` and `E0087`. `E0088` onward stays free.
- **Do NOT push.** Commit only — the repo owner pushes explicitly. Merges are fast-forward only; history is strictly linear (270 commits, **0 merge commits**).
- End every commit message body with:
  ```
  Co-Authored-By: Claude Opus 5 <noreply@anthropic.com>
  ```

## File Structure

| File | Responsibility | Tasks |
|---|---|---|
| `crates/nova-hir/src/lib.rs` | `Ty::Future`; `ExprKind::Await`; all 11 `Ty`-recursion sites | 1, 2 |
| `crates/nova-typeck/src/lib.rs:64` | `display_ty` arm for `Future` | 1 |
| `crates/nova-typeck/src/infer.rs` | `apply`/`occurs`/`unify` arms for `Future` | 1 |
| `crates/nova-typeck/src/check.rs:2394`, `:5080` | `Future` in **both** built-in type-name tables | 1 |
| `crates/nova-typeck/src/check.rs` | async fn signature typing, `.await`, `E0086`/`E0087`, the `E0900` lift | 2 |
| `crates/nova-mir/src/lib.rs:456`, `:495` | `mir_ty` and `mangle_ty` arms for `Future` | 1 |
| `crates/nova-mir/src/mono.rs:282` | `type_name` arm for `Future` | 1 |
| `crates/nova-runtime/src/gc.rs` | persistent root registry | 3 |
| `crates/nova-runtime/src/task.rs` | **new** — the executor, ready queue, `block_on` | 4 |
| `crates/nova-mir/src/lib.rs` (`rt_funcs!`) | `RtFunc::TaskSpawn`, `TaskBlockOn`, `TaskJoin`, `TaskYield` | 4 |
| `crates/nova-mir/src/async_lower.rs` | **new** — the state-machine transform | 5, 6 |
| `crates/nova-resolver/src/lib.rs:664` | `STD_MODULES` 3 → 4 | 7 |
| `std/task/lib.nova` | **new** — `spawn`, `JoinHandle`, `join`, `yield_now`, `block_on` | 7 |
| `crates/nova-driver/src/lib.rs` | wrap an `async fn main` in `block_on` | 7 |
| `tests/runtime/async_tasks.nova` + `.stdout` | the gate fixture | 8 |
| `crates/nova-cli/tests/run_tests.rs` | the three gate registrations | 8 |
| `docs/adr/0009-async-execution-model.md` | the model decision and its reversal of plan decision 1 | 8 |

No new crates. The transform lives in its own file (`async_lower.rs`) rather than inside `lower.rs` (1176 lines) or `mono.rs` (760 lines), because it is a distinct pass with a distinct input — a finished `Module` — and both existing files are already at the size where edits get unreliable.

---

### Task 1: `Ty::Future` plumbing

Pure type-system plumbing, no behaviour. `Ty::Array(Box<Ty>)` is the exact structural analogue — one boxed type argument, no `DefId` — so **every `Ty::Array` arm gets a `Ty::Future` sibling**. The measured list is below; verify it is still complete with `grep -rn "Ty::Array(" crates/ --include=*.rs`.

**Files:**
- Modify: `crates/nova-hir/src/lib.rs` — the `Ty` enum, and arms at `:108` (`subst`), `:127` (`mentions_param`), `:139` (`has_params`), `:151` (`has_vars`), `:169` (`has_assoc`), `:195` (`has_error`), `:262` (`match_pattern`), `:603` (`normalize_within`), `:661` (`shift_params`), `:684` (`occurs`), `:732` (`unify_patterns`)
- Modify: `crates/nova-typeck/src/lib.rs:64` (`display_ty`)
- Modify: `crates/nova-typeck/src/infer.rs:50`, `:69`, `:158` (`apply`, `occurs`, `unify`)
- Modify: `crates/nova-typeck/src/check.rs:2394` and `:5080` (both built-in type-name tables)
- Modify: `crates/nova-mir/src/lib.rs:456` (`mir_ty`), `:495`–`:524` (`mangle_ty`)
- Modify: `crates/nova-mir/src/mono.rs:282` (`type_name`)
- Test: `crates/nova-hir/src/lib.rs` `mod tests` (`:1184`), `crates/nova-typeck/src/check.rs` `mod tests` (`:7216`), `crates/nova-mir/src/mono.rs` `mod tests` (`:571`)

**Interfaces:**
- Produces: `hir::Ty::Future(Box<Ty>)`. `mir_ty(&Ty::Future(_)) == MirTy::Ptr`. `display_ty(&Ty::Future(Ty::Int), defs) == "Future<Int>"`. `mangle_ty(&Ty::Future(Ty::Int))` is distinct per output type. `convert_ty` maps the surface type `Future<T>` to it.
- Consumes: nothing.

`Ty::head()` (`nova-hir/src/lib.rs:203-214`) needs **no** arm — it ends in `_ => None`, and a future is not an impl target, exactly like `Ty::Array` and `Ty::Fn` today.

- [ ] **Step 1: Write the failing tests**

In `crates/nova-mir/src/mono.rs`'s `mod tests` (it is a child of the crate root, so it can reach the private `crate::mangle_ty`; if that turns out not to compile, add a `#[cfg(test)] mod tests` to `nova-mir/src/lib.rs` instead and report the correction):

```rust
#[test]
fn mangle_ty_distinguishes_futures_by_output_type() {
    // A Future's mangled name must depend on its output type. `Ty::Assoc`
    // mangling to a constant "X" already shipped as a miscompile on this
    // project: two instantiations collided on one symbol and both dispatched
    // to the first's code. A constant here reproduces that exactly.
    let a = crate::mangle_ty(&hir::Ty::Future(Box::new(hir::Ty::Int)));
    let b = crate::mangle_ty(&hir::Ty::Future(Box::new(hir::Ty::Float)));
    let c = crate::mangle_ty(&hir::Ty::Future(Box::new(hir::Ty::Bool)));
    assert_ne!(a, b);
    assert_ne!(b, c);
    assert_ne!(a, c);
    // And distinct from the array mangling of the same element type, so the
    // two single-argument constructors cannot collide either.
    assert_ne!(a, crate::mangle_ty(&hir::Ty::Array(Box::new(hir::Ty::Int))));
}

#[test]
fn mir_ty_maps_future_to_ptr() {
    // A future value is the fat pointer { poll_code, state_ptr }.
    // MirTy::Unit would be catastrophic and silent: unit parameters are
    // DROPPED from the Cranelift signature, which is how the 2.2c projection
    // bug produced wrong values with exit 0 and no diagnostic.
    assert_eq!(
        crate::mir_ty(&hir::Ty::Future(Box::new(hir::Ty::Int))),
        crate::MirTy::Ptr
    );
}
```

In `crates/nova-hir/src/lib.rs`'s `mod tests`:

```rust
#[test]
fn future_recurses_into_its_output_type() {
    // Each predicate must look THROUGH the Future, not past it. A `_ => false`
    // catch-all would pass a test that only ever asked about Future<Int>.
    let p = || Ty::Param(0);
    assert!(Ty::Future(Box::new(p())).has_params());
    assert!(!Ty::Future(Box::new(Ty::Int)).has_params());
    assert!(Ty::Future(Box::new(Ty::Var(0))).has_vars());
    assert!(!Ty::Future(Box::new(Ty::Int)).has_vars());
    assert!(Ty::Future(Box::new(Ty::Error)).has_error());
    assert!(!Ty::Future(Box::new(Ty::Int)).has_error());
    assert!(Ty::Future(Box::new(p())).mentions_param(0));
    assert!(!Ty::Future(Box::new(p())).mentions_param(1));
}

#[test]
fn future_substitutes_its_output_type() {
    let subbed = Ty::Future(Box::new(Ty::Param(0))).subst(&[Ty::Float]);
    assert_eq!(subbed, Ty::Future(Box::new(Ty::Float)));
}

#[test]
fn future_match_pattern_binds_through_the_output() {
    // Recovers T from Future<T> against a concrete Future<Bool>.
    let mut out = Vec::new();
    assert!(Ty::Future(Box::new(Ty::Param(0)))
        .match_pattern(&Ty::Future(Box::new(Ty::Bool)), &mut out));
    assert_eq!(out, vec![Some(Ty::Bool)]);
    // And refuses a structural mismatch rather than binding anything.
    let mut out2 = Vec::new();
    assert!(!Ty::Future(Box::new(Ty::Param(0)))
        .match_pattern(&Ty::Array(Box::new(Ty::Bool)), &mut out2));
}
```

In `crates/nova-typeck/src/check.rs`'s `mod tests`:

```rust
#[test]
fn future_displays_with_its_output_type_in_a_real_diagnostic() {
    // display_ty must render the output, not a bare "Future". Two different
    // futures printing the same string is the `T{i}` debt this project already
    // carries in diagnostics; do not add another instance of it.
    //
    // Asserted through a real mismatch message rather than by calling
    // display_ty directly: `CheckResult` is `{ module, diagnostics }` and
    // exposes no `Definitions`, so a direct call would need a second resolve
    // in the test. Going through the diagnostic is also the stronger test —
    // it is the path a user actually sees.
    let r = check_src(
        "fn take(x: Future<Int>) -> Int { 1 }\n\
         fn f(y: Future<Float>) -> Int { take(y) }\n\
         fn main() {}",
    );
    let msgs: Vec<String> = r.diagnostics.iter().map(|d| d.message.clone()).collect();
    assert!(
        msgs.iter().any(|m| m.contains("Future<Int>") && m.contains("Future<Float>")),
        "expected a message naming both futures by their output types, got {msgs:?}"
    );
}

#[test]
fn bare_future_without_a_type_argument_is_rejected() {
    // `Future` takes exactly one argument. Both the zero-argument and the
    // two-argument spellings must be diagnosed, not silently accepted --
    // this is the arity path that no existing built-in type name exercises,
    // because Int/Float/Bool/Char/String are all nullary.
    let r = check_src("fn f(x: Future) -> Int { 1 }\nfn main() {}");
    assert!(
        r.diagnostics.iter().any(|d| d.code == "E0012"),
        "expected E0012, got {:?}",
        r.diagnostics.iter().map(|d| &d.code).collect::<Vec<_>>()
    );
}

#[test]
fn future_with_two_type_arguments_is_rejected() {
    let r = check_src("fn f(x: Future<Int, Bool>) -> Int { 1 }\nfn main() {}");
    assert!(r.diagnostics.iter().any(|d| d.code == "E0012"));
}

#[test]
fn future_of_int_and_future_of_float_do_not_unify() {
    // The unifier must descend into the output type. An arm that unified any
    // two Futures would make `Future<Int>` and `Future<Float>` interchangeable.
    let r = check_src(
        "fn take(x: Future<Int>) -> Int { 1 }\n\
         fn f(y: Future<Float>) -> Int { take(y) }\n\
         fn main() {}",
    );
    assert!(
        r.diagnostics.iter().any(|d| d.code == "E0010"),
        "expected a type mismatch, got {:?}",
        r.diagnostics.iter().map(|d| &d.code).collect::<Vec<_>>()
    );
}

#[test]
fn future_of_int_unifies_with_itself() {
    // The discriminating half of the test above, and NOT redundant with it:
    // an implementation whose `Future` unify arm always FAILED would satisfy
    // the mismatch test perfectly. Only this one rejects that.
    let r = check_src(
        "fn take(x: Future<Int>) -> Int { 1 }\n\
         fn f(y: Future<Int>) -> Int { take(y) }\n\
         fn main() {}",
    );
    assert!(
        r.diagnostics.is_empty(),
        "Future<Int> must unify with Future<Int>, got {:?}",
        r.diagnostics.iter().map(|d| d.message.clone()).collect::<Vec<_>>()
    );
}
```

- [ ] **Step 2: Run the tests to verify they fail**

```bash
cargo build --workspace && cargo test --workspace --no-fail-fast 2>&1 | tail -40
```

Expected: compile failure — `Ty` has no variant `Future`. That is the correct first failure; it proves the tests reference the thing being built.

- [ ] **Step 3: Add the variant and every recursion arm**

In `crates/nova-hir/src/lib.rs`, beside `Ty::Fn`:

```rust
    /// A future produced by calling an `async fn`; the payload is the
    /// function's declared return type (the future's *output*).
    ///
    /// Compiler-constructed only: there is no surface syntax that builds one,
    /// and no impls may be written on it (`head()` returns `None`). At runtime
    /// it is the same 2-word fat pointer as a function value —
    /// `{ poll_code, state_ptr }` — which is why it sits beside `Ty::Fn` here
    /// rather than being a record in `std/core`. Nova has no lang-item
    /// mechanism: `Option` and `Result` are ordinary prelude sums the compiler
    /// does not know by name, so a std record could not be recognized.
    Future(Box<Ty>),
```

Then add the sibling of each `Ty::Array` arm at the eleven `nova-hir` sites, the three `infer.rs` sites, `display_ty`, `mir_ty`, `mangle_ty` and `type_name`. Each is mechanical:

```rust
// nova-hir subst / normalize_within / shift_params (rebuilding arms)
Ty::Future(out) => Ty::Future(Box::new(out.subst(args))),
// nova-hir predicate arms
Ty::Future(out) => out.has_params(),
// nova-hir match_pattern / unify_patterns (paired arms)
(Ty::Future(o1), Ty::Future(o2)) => o1.match_pattern(o2, out),
// nova-typeck display_ty
Ty::Future(out) => format!("Future<{}>", display_ty(out, defs)),
// nova-mir mir_ty — add Future to the existing Ptr group
| hir::Ty::Future(_) => MirTy::Ptr,
// nova-mir mangle_ty — a distinct letter, not a shared placeholder
hir::Ty::Future(out) => format!("U{}E", mangle_ty(out)),
// nova-mir mono type_name
Ty::Future(out) => format!("Future<{}>", type_name(out, module)),
```

Compile after this step and fix every non-exhaustive-match error before moving on. `error[E0004]` is the mechanism that finds the sites this plan missed — **trust it over the list above** and report any extra site you had to touch.

- [ ] **Step 4: Teach both built-in type-name tables**

`Future` is the **first built-in type name that takes a type argument**, so it cannot join the nullary `prim` table at `check.rs:2394` as-is — that table's `if !args.is_empty()` guard reports `E0012` "takes no type arguments", which is the opposite of what `Future` needs. Handle it before that table:

```rust
                // `Future<T>` — the one built-in type name with an argument.
                // Handled ahead of the nullary `prim` table below, whose
                // `args.is_empty()` guard means the opposite here.
                if name == "Future" {
                    if args.len() != 1 {
                        self.error(
                            "E0012",
                            format!(
                                "`Future` takes exactly one type argument, found {}",
                                args.len()
                            ),
                            ty.span,
                        );
                        return Ty::Error;
                    }
                    let out = self.convert_ty(&args[0], generics, bounds);
                    return Ty::Future(Box::new(out));
                }
```

`check.rs:5080` (`qualifier_self_ty`) is the **second** site and must also be handled — there it should return `None`, because `Future::something()` is not a valid associated-function qualifier and falling through to `resolve_type` would report a confusing "unknown type". Add:

```rust
            // `Future` is compiler-constructed and carries no associated
            // functions; `Future::f()` is not a qualifier. Returning None here
            // (rather than falling through to resolve_type) keeps the
            // diagnostic about the qualifier instead of about a missing type.
            "Future" => return None,
```

**Both sites, not one.** The 2.2c structural-match fix reached two of four impl-lookup sites and the two it missed were a trait-dispatch path and a bound check that had silently drifted out of sync — that shipped as a miscompile. Re-grep for built-in type-name tables before you finish this step and report any third site.

- [ ] **Step 5: Run the tests to verify they pass**

```bash
cargo build --workspace && cargo test --workspace --no-fail-fast 2>&1 | tail -20
```

Expected: all Task 1 tests PASS, baseline count + 9, 0 failed.

- [ ] **Step 6: Kill the mangling mutation by hand**

Change `mangle_ty`'s new arm to `hir::Ty::Future(_) => "U".to_string()` and re-run:

```bash
cargo test -p nova-mir --no-fail-fast 2>&1 | tail -20
```

Expected: `mangle_ty_distinguishes_futures_by_output_type` FAILS. Revert the mutation, then `cargo build --workspace` before any further probe — a poisoned `nova.exe` has cost this project two retracted findings.

- [ ] **Step 7: Commit**

```bash
git add crates/nova-hir crates/nova-typeck crates/nova-mir
git commit -m "feat(types): add Ty::Future for async function results

Plumbing only, no behaviour: the variant plus a sibling for every
Ty::Array arm (11 in nova-hir, 3 in infer.rs, display_ty, mir_ty,
mangle_ty, mono type_name), and Future<T> in BOTH built-in type-name
tables. mangle_ty gets a real, output-dependent case rather than a
placeholder -- Ty::Assoc mangling to a constant already shipped as a
symbol-collision miscompile on this project.

Future is the first built-in type name taking a type argument, so it is
handled ahead of the nullary prim table whose arity guard means the
opposite.

Co-Authored-By: Claude Opus 5 <noreply@anthropic.com>"
```

---

### Task 2: Typing `async fn` and `.await`

Types the surface language and lifts the `E0900` rejections. **Ends with `nova check` accepting async code and `nova run` refusing it cleanly** — the transform arrives in Tasks 5–6, so this task deliberately leaves a diagnosed gap rather than a silent one.

**Files:**
- Modify: `crates/nova-hir/src/lib.rs` — `ExprKind::Await`
- Modify: `crates/nova-typeck/src/check.rs` — `:852` (make conditional), `:1244` (remove), `:2000` (remove), async signature typing, `ast::Expr::Await` handling
- Modify: `crates/nova-mir/src/lower.rs` — a diagnosed rejection for `ExprKind::Await`
- Test: `crates/nova-typeck/src/check.rs` `mod tests`

**Interfaces:**
- Consumes: `hir::Ty::Future` (Task 1).
- Produces: `hir::ExprKind::Await(Box<Expr>)`. An `async fn f(A) -> T` has `hir::Function.ret_ty == Ty::Future(T)` and a new `hir::Function.is_async: bool`. Diagnostics `E0086` (await outside an async fn) and `E0087` (await on a non-future).

- [ ] **Step 1: Write the failing tests**

In `crates/nova-typeck/src/check.rs`'s `mod tests`:

```rust
#[test]
fn async_fn_returns_a_future_of_its_declared_type() {
    // The declared return type is the future's OUTPUT, per the spec's own
    // signatures (`pub async fn join(self) -> T`). So passing the CALL of an
    // async fn where the output type is expected must be a mismatch.
    let r = check_src(
        "async fn f() -> Int { 1 }\n\
         fn g() -> Int { f() }\n\
         fn main() {}",
    );
    assert!(
        r.diagnostics.iter().any(|d| d.code == "E0010"),
        "calling an async fn yields Future<Int>, not Int; got {:?}",
        r.diagnostics.iter().map(|d| &d.code).collect::<Vec<_>>()
    );
}

#[test]
fn awaiting_a_future_yields_its_output_type() {
    // The positive case: inside an async fn, `.await` unwraps to the output
    // and type-checks against it. Assert CLEAN, and assert on the messages so
    // a spurious diagnostic is visible rather than counted.
    let r = check_src(
        "async fn f() -> Int { 1 }\n\
         async fn g() -> Int { f().await }\n\
         fn main() {}",
    );
    assert!(
        r.diagnostics.is_empty(),
        "expected no diagnostics, got {:?}",
        r.diagnostics.iter().map(|d| d.message.clone()).collect::<Vec<_>>()
    );
}

#[test]
fn awaiting_yields_the_output_not_the_future() {
    // Discriminates "await returns T" from "await returns Future<T>" — the
    // obvious off-by-one-layer bug, which the clean test above cannot see.
    let r = check_src(
        "async fn f() -> Int { 1 }\n\
         async fn g() -> Bool { f().await }\n\
         fn main() {}",
    );
    let msgs: Vec<String> = r.diagnostics.iter().map(|d| d.message.clone()).collect();
    assert!(
        msgs.iter().any(|m| m.contains("Int") && m.contains("Bool")),
        "expected an Int-vs-Bool mismatch naming both types, got {msgs:?}"
    );
}

#[test]
fn await_outside_an_async_fn_reports_e0086() {
    let r = check_src(
        "async fn f() -> Int { 1 }\n\
         fn g() -> Int { f().await }\n\
         fn main() {}",
    );
    let d = r
        .diagnostics
        .iter()
        .find(|d| d.code == "E0086")
        .unwrap_or_else(|| panic!("expected E0086, got {:?}",
            r.diagnostics.iter().map(|d| &d.code).collect::<Vec<_>>()));
    // Assert the message names the actual problem. A code-only assertion here
    // would survive swapping E0086's and E0087's message text.
    assert!(
        d.message.contains("async"),
        "E0086 must explain that await requires an async fn; got {:?}",
        d.message
    );
}

#[test]
fn await_on_a_non_future_reports_e0087() {
    let r = check_src(
        "async fn g() -> Int { let x = 1\n x.await }\n\
         fn main() {}",
    );
    let d = r
        .diagnostics
        .iter()
        .find(|d| d.code == "E0087")
        .unwrap_or_else(|| panic!("expected E0087, got {:?}",
            r.diagnostics.iter().map(|d| &d.code).collect::<Vec<_>>()));
    assert!(
        d.message.contains("Int"),
        "E0087 must name the type that is not a future; got {:?}",
        d.message
    );
}

#[test]
fn async_inherent_method_is_accepted() {
    // NOT optional: the spec's JoinHandle::join is `pub async fn join(self) -> T`,
    // an inherent async method. Task 7 cannot write std/task without this.
    let r = check_src(
        "record W { v: Int }\n\
         impl W { async fn get(self) -> Int { self.v } }\n\
         fn main() {}",
    );
    assert!(
        r.diagnostics.is_empty(),
        "expected no diagnostics, got {:?}",
        r.diagnostics.iter().map(|d| d.message.clone()).collect::<Vec<_>>()
    );
}

#[test]
fn async_trait_method_still_reports_e0900() {
    // Trait async needs associated-type futures; out of scope for 2.3a.
    // This pins the HALF-lift: :852 becomes conditional, it does not vanish.
    let r = check_src("trait T { async fn m(self) -> Int }\nfn main() {}");
    assert!(r.diagnostics.iter().any(|d| d.code == "E0900"));
}

#[test]
fn async_extern_fn_still_reports_e0900() {
    let r = check_src(
        "extern \"C\" { async fn c_thing() -> Int }\nfn main() {}",
    );
    assert!(r.diagnostics.iter().any(|d| d.code == "E0900"));
}

#[test]
fn generic_async_fn_instantiates_at_float() {
    // Float, not Int/Bool: mir_ty collapses Int/Char to I64 and five variants
    // to Ptr (= i64 on x86-64), so an Int-vs-String pair tests nothing at any
    // seam. Float is F64 and crosses register banks.
    let r = check_src(
        "async fn id<T>(x: T) -> T { x }\n\
         async fn g() -> Float { id(1.5).await }\n\
         fn main() {}",
    );
    assert!(
        r.diagnostics.is_empty(),
        "expected no diagnostics, got {:?}",
        r.diagnostics.iter().map(|d| d.message.clone()).collect::<Vec<_>>()
    );
}
```

- [ ] **Step 2: Run the tests to verify they fail**

```bash
cargo build --workspace && cargo test -p nova-typeck --no-fail-fast 2>&1 | tail -30
```

Expected: the async tests fail with `E0900` ("async functions are not supported yet"), and the `.await` ones additionally fail because `ast::Expr::Await` has no typing rule. Confirm the `running N tests` line is non-zero.

- [ ] **Step 3: Carry `is_async` into HIR and add `ExprKind::Await`**

In `crates/nova-hir/src/lib.rs`, add to `ExprKind`:

```rust
    /// `e.await` inside an `async fn`. `e` has type `Future<T>` and the
    /// `Await` expression has type `T`.
    ///
    /// Survives into MIR, where the async transform (`async_lower.rs`) turns
    /// each one into a suspend point. Reaching MIR lowering *outside* that
    /// transform is a compiler bug, not valid input.
    Await(Box<Expr>),
```

and to `Function`:

```rust
    /// Whether this function was declared `async`. `ret_ty` is already
    /// `Ty::Future(output)` when true; this flag is what tells the MIR async
    /// pass which functions to transform, since a non-async function may also
    /// return a `Future` (by calling an async fn and not awaiting it).
    pub is_async: bool,
```

That last sentence is the reason the flag is needed at all — do not try to infer async-ness from `ret_ty`.

- [ ] **Step 4: Type the signature, the body and `.await`**

Signature: where a function's `ret_ty` is built, wrap it when `f.is_async`. The body is then checked against the **output** type, not the future — `async fn f() -> Int { 1 }` has a body of type `Int`.

`.await` typing, in the expression checker beside the other postfix forms:

```rust
            ast::Expr::Await(inner) => {
                let e = self.check_expr(fcx, inner, None);
                if !fcx.in_async {
                    self.diagnostics.push(
                        Diagnostic::error(
                            "E0086",
                            "`.await` is only allowed inside an `async fn`".to_string(),
                        )
                        .with_primary_label(expr.span, "await outside an async function")
                        .with_note(
                            "make the enclosing function `async`, or drive the \
                             future to completion with `block_on`"
                                .to_string(),
                        ),
                    );
                    return error_expr(expr.span);
                }
                let out = match fcx.icx.apply(&e.ty) {
                    Ty::Future(out) => (*out).clone(),
                    Ty::Error => return error_expr(expr.span),
                    other => {
                        self.diagnostics.push(
                            Diagnostic::error(
                                "E0087",
                                format!(
                                    "`.await` expects a future, found `{}`",
                                    display_ty(&other, self.defs)
                                ),
                            )
                            .with_primary_label(inner.span, "not a future"),
                        );
                        return error_expr(expr.span);
                    }
                };
                hir::Expr {
                    kind: hir::ExprKind::Await(Box::new(e)),
                    ty: out,
                    span: expr.span,
                }
            }
```

`fcx.in_async` is a new `bool` on `FnCtx`, set when entering an async function's body and **reset to `false` when entering a closure body** — the same discipline `loop_depth` already uses at `check.rs:285`, and for the same reason: a closure inside an async fn is its own non-async function.

The `E0900` lift, precisely:
- `:2000` (free functions) — **delete** the rejection.
- `:1244` (impl block functions) — **delete** the rejection.
- `:852` — make conditional. It currently covers trait *and* impl methods from one loop; keep it firing only when the method's owner is a trait. `:950` (the default-body guard) and `:2086` (extern fns) are **unchanged**.

- [ ] **Step 5: Reject `Await` in MIR lowering, with a diagnostic**

Tasks 5–6 add the transform. Until then `ExprKind::Await` must not reach `lower.rs` silently. Add an arm that reports an internal-compiler-error diagnostic (follow how `lower.rs` already handles impossible input — grep for `E0601`), **not** a `panic!`, `todo!` or `unreachable!`: this is a library path, and `mir_ty`'s own comment records that reaching a should-be-impossible arm "is a compiler bug, but one that must not panic in a library path".

- [ ] **Step 6: Run the tests to verify they pass**

```bash
cargo build --workspace && cargo test --workspace --no-fail-fast 2>&1 | tail -20
```

Expected: all Task 2 tests PASS, baseline + 9 more, 0 failed.

- [ ] **Step 7: Probe the end-to-end state deliberately**

```bash
printf 'async fn f() -> Int { 1 }\nasync fn g() -> Int { f().await }\nfn main() { }\n' > /tmp/a.nova
cargo run -q -p nova-cli -- check /tmp/a.nova
```

Expected: `check` succeeds. Record what `nova run /tmp/a.nova` does — it should report the Step 5 diagnostic, not crash and not silently succeed. **If it silently succeeds, stop and report**: that means the async function was never reached from `main`, and the probe proved nothing.

- [ ] **Step 8: Commit**

```bash
git add crates/nova-hir crates/nova-typeck crates/nova-mir
git commit -m "feat(typeck): type async fn and .await

An async fn's declared return type becomes its future's OUTPUT, so the
fn types as fn(A) -> Future<T> and its body checks against T. Adds
E0086 (.await outside an async fn) and E0087 (.await on a non-future).

Lifts E0900 for free functions and inherent methods; inherent async
methods are required, not optional, because the spec's JoinHandle::join
is one. Trait methods and extern fns stay rejected.

hir::Function.is_async is carried explicitly rather than inferred from
ret_ty: a NON-async function can also return a Future, by calling an
async fn without awaiting it.

MIR lowering rejects ExprKind::Await with a diagnostic until the
transform lands, so the gap is diagnosed rather than silent.

Co-Authored-By: Claude Opus 5 <noreply@anthropic.com>"
```

---

### Task 3: Persistent GC root registry

The highest-risk item in the branch, done first among the runtime work and **testable entirely in Rust**, with no dependency on Tasks 1–2.

**Why it is needed:** the collector's only root sources are the Nova stack and callee-saved registers (`gc.rs:14-17`). A suspended task's state object is held by the Rust executor — it is on no Nova stack and in no register — so without this it is freed while the task is suspended.

**Files:**
- Modify: `crates/nova-runtime/src/gc.rs`
- Test: `crates/nova-runtime/src/gc.rs` `mod tests` (`:368`)

**Interfaces:**
- Produces: `gc::add_root(ptr: *mut u8)`, `gc::remove_root(ptr: *mut u8)`, both thread-local like the heap. Registered addresses are treated as roots by `collect()` **in addition to** the stack scan.
- Consumes: nothing.

- [ ] **Step 1: Write the failing tests**

In `crates/nova-runtime/src/gc.rs`'s `mod tests`:

```rust
#[test]
fn a_registered_root_survives_a_collection_with_no_stack_reference() {
    // The exact scenario: an object reachable ONLY through the registry.
    // `black_box` is not enough on its own here -- the point is that after
    // the pointer is registered we must NOT keep it in a live local that the
    // conservative stack scan would find anyway, or the test passes with the
    // registry doing nothing. So we register, then overwrite the local.
    let obj = alloc(64, true);
    add_root(obj);
    let addr = obj as usize;
    let obj = std::ptr::null_mut::<u8>();
    std::hint::black_box(obj);

    collect();

    assert!(
        object_info(addr).is_some(),
        "a registered root was swept; the registry is not seeding the mark set"
    );
    remove_root(addr as *mut u8);
}

#[test]
fn an_unregistered_object_is_swept() {
    // The discriminating half. Without this, the test above passes even if
    // collect() never frees anything at all.
    let obj = alloc(64, true);
    let addr = obj as usize;
    let obj = std::ptr::null_mut::<u8>();
    std::hint::black_box(obj);

    collect();

    assert!(
        object_info(addr).is_none(),
        "an unreachable, unregistered object survived; this test cannot \
         discriminate a working registry from a collector that frees nothing"
    );
}

#[test]
fn remove_root_actually_unroots() {
    // Otherwise add/remove is a leak, and every completed task's state is
    // retained for the process lifetime.
    let obj = alloc(64, true);
    let addr = obj as usize;
    add_root(obj);
    remove_root(obj);
    let obj = std::ptr::null_mut::<u8>();
    std::hint::black_box(obj);

    collect();

    assert!(object_info(addr).is_none(), "remove_root did not unroot");
}

#[test]
fn a_registered_root_keeps_its_transitive_children_alive() {
    // The registry seeds the mark set; marking must then TRACE. A registry
    // that marked only the registered object itself would free a suspended
    // task's locals while keeping its state header -- the exact bug, one
    // level down, and invisible to the first test.
    let parent = alloc(16, true);
    let child = alloc(32, true);
    let child_addr = child as usize;
    unsafe { (parent as *mut usize).write(child as usize) };
    add_root(parent);
    let child = std::ptr::null_mut::<u8>();
    std::hint::black_box(child);

    collect();

    assert!(
        object_info(child_addr).is_some(),
        "a child reachable only through a registered root was swept"
    );
    remove_root(parent);
}

#[test]
fn the_registry_survives_more_than_one_collection() {
    // ROOTS (gc.rs:91) is a SCRATCH buffer cleared at the start of every
    // cycle. If the registry were folded into it, the first collection would
    // consume it and the second would sweep the root. That failure mode is
    // invisible to any single-collection test.
    let obj = alloc(64, true);
    let addr = obj as usize;
    add_root(obj);
    let obj = std::ptr::null_mut::<u8>();
    std::hint::black_box(obj);

    collect();
    collect();
    collect();

    assert!(
        object_info(addr).is_some(),
        "the registry did not survive repeated collections; it is probably \
         sharing the scratch ROOTS buffer"
    );
    remove_root(addr as *mut u8);
}
```

- [ ] **Step 2: Run the tests to verify they fail**

```bash
cargo test -p nova-runtime --no-fail-fast 2>&1 | tail -30
```

Expected: compile failure — no `add_root`/`remove_root`. Confirm `running N tests` is non-zero once they compile.

- [ ] **Step 3: Implement the registry**

Add a **separate, persistent** thread-local beside `HEAP` and `ROOTS` in `gc.rs`:

```rust
thread_local! {
    static HEAP: RefCell<Heap> = const { RefCell::new(Heap::new()) };
    /// Scratch buffer of candidate root words, filled by `nova_gc_scan_range`.
    static ROOTS: RefCell<Vec<usize>> = const { RefCell::new(Vec::new()) };
    /// Explicitly registered roots — addresses the collector must treat as
    /// live even though they appear on no stack and in no register.
    ///
    /// This exists for suspended async tasks: the executor owns a task's
    /// state object while the task is parked, and the only root sources this
    /// collector has are the Nova stack and callee-saved registers. Without
    /// registration, a suspended task's state is swept.
    ///
    /// **Deliberately NOT merged into `ROOTS`.** `ROOTS` is scratch: it is
    /// cleared at the start of every cycle. A registry sharing it would be
    /// consumed by the first collection and the root swept by the second —
    /// a failure mode invisible to any single-collection test.
    static PINNED: RefCell<Vec<usize>> = const { RefCell::new(Vec::new()) };
}
```

```rust
/// Register `ptr` as a root until [`remove_root`]. Idempotent per address is
/// **not** assumed: registering twice requires removing twice, so callers must
/// pair them exactly. The executor does (one add at spawn, one remove at
/// completion).
pub fn add_root(ptr: *mut u8) {
    PINNED.with(|p| p.borrow_mut().push(ptr as usize));
}

/// Unregister one registration of `ptr`. Removing an address that was never
/// registered is a no-op rather than a panic — the runtime must not abort a
/// user's program over its own bookkeeping.
pub fn remove_root(ptr: *mut u8) {
    PINNED.with(|p| {
        let mut v = p.borrow_mut();
        if let Some(i) = v.iter().rposition(|&a| a == ptr as usize) {
            v.swap_remove(i);
        }
    });
}
```

In `collect()`, seed the candidate list from `PINNED` **before** `nova_gc_collect_roots` runs the stack scan, so the registered addresses go through the same range-based marking (which is what makes Step 1's transitive test pass — do not write a separate mark path for them).

- [ ] **Step 4: Run the tests to verify they pass**

```bash
cargo test -p nova-runtime --no-fail-fast 2>&1 | tail -20
```

Expected: all five PASS.

- [ ] **Step 5: Kill two mutations by hand**

1. Make `remove_root` a no-op (`pub fn remove_root(_ptr: *mut u8) {}`). Expected: `remove_root_actually_unroots` FAILS.
2. Change `collect()` to clear `PINNED` after seeding. Expected: `the_registry_survives_more_than_one_collection` FAILS.

Revert both, then `cargo build --workspace`.

- [ ] **Step 6: Commit**

```bash
git add crates/nova-runtime/src/gc.rs
git commit -m "feat(gc): persistent root registry for off-stack roots

The collector's only root sources are the Nova stack and callee-saved
registers. A suspended async task's state object is held by the Rust
executor -- on no stack, in no register -- so it would be swept while
the task is parked.

PINNED is a separate thread-local from ROOTS, deliberately: ROOTS is a
scratch buffer cleared at the start of every cycle, so a registry
sharing it would be consumed by the first collection and the root swept
by the second, which no single-collection test can see.

Registered addresses are seeded before the stack scan and go through the
same range-based marking, so a root's transitive children stay alive.

Co-Authored-By: Claude Opus 5 <noreply@anthropic.com>"
```

---

### Task 4: The executor

Also fully testable in Rust and independent of Tasks 1–2: drive the executor with a **hand-written Rust poll function** that mimics what Tasks 5–6 will generate. That is the whole point of doing it here — the ABI gets exercised and pinned before any codegen depends on it.

**Files:**
- Create: `crates/nova-runtime/src/task.rs`
- Modify: `crates/nova-runtime/src/lib.rs` — `mod task;` and the `nova_rt_task_*` exports
- Modify: `crates/nova-mir/src/lib.rs` — four `rt_funcs!` entries
- Test: `crates/nova-runtime/src/task.rs` `#[cfg(test)] mod tests`

**Interfaces:**
- Consumes: `gc::add_root`/`gc::remove_root` (Task 3), `gc::alloc`.
- Produces, and Tasks 5–7 depend on these exact signatures:

```rust
pub type PollFn = unsafe extern "C" fn(state: *mut u8, task_ctx: *mut u8) -> i64;
pub const POLL_PENDING: i64 = 0;
pub const POLL_READY: i64 = 1;

// State object layout, which async_lower.rs must reproduce exactly:
//   slot 0 (offset 0)  : resume tag (i64)
//   slot 1 (offset 8)  : output value
//   slot 2.. (offset 16): one slot per MIR temp
pub const STATE_SLOT_TAG: usize = 0;
pub const STATE_SLOT_OUTPUT: usize = 1;
pub const STATE_SLOT_TEMPS: usize = 2;
```

- `nova_rt_task_spawn(future: *mut u8) -> i64` — queue a task from a `{poll_code, state}` fat pointer, return a task id.
- `nova_rt_task_block_on(future: *mut u8) -> i64` — run the executor until that future completes; return its output slot.
- `nova_rt_task_is_done(id: i64) -> i8` and `nova_rt_task_take_output(id: i64) -> i64` — what `JoinHandle::join` polls on.
- `nova_rt_task_yield(task_ctx: *mut u8)` — mark the current task as wanting another turn.

- [ ] **Step 1: Write the failing tests**

In `crates/nova-runtime/src/task.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    /// A hand-written poll function shaped exactly like the ones
    /// `async_lower.rs` will generate: it reads and writes the resume tag in
    /// slot 0, writes its result to slot 1, and returns PENDING/READY.
    ///
    /// Suspends once, then completes with 42.
    unsafe extern "C" fn poll_suspend_once(state: *mut u8, _ctx: *mut u8) -> i64 {
        let slots = state as *mut i64;
        let tag = *slots.add(STATE_SLOT_TAG);
        if tag == 0 {
            *slots.add(STATE_SLOT_TAG) = 1;
            POLL_PENDING
        } else {
            *slots.add(STATE_SLOT_OUTPUT) = 42;
            POLL_READY
        }
    }

    unsafe extern "C" fn poll_ready_now(state: *mut u8, _ctx: *mut u8) -> i64 {
        *(state as *mut i64).add(STATE_SLOT_OUTPUT) = 7;
        POLL_READY
    }

    /// Build the `{ poll_code, state_ptr }` fat pointer the compiler emits.
    fn make_future(f: PollFn, temps: usize) -> *mut u8 {
        let state = gc::alloc((STATE_SLOT_TEMPS + temps) * 8, true);
        let fat = gc::alloc(16, true);
        unsafe {
            (fat as *mut usize).write(f as usize);
            (fat as *mut usize).add(1).write(state as usize);
        }
        fat
    }

    #[test]
    fn block_on_runs_a_ready_future_and_returns_its_output() {
        let fut = make_future(poll_ready_now, 0);
        assert_eq!(unsafe { nova_rt_task_block_on(fut) }, 7);
    }

    #[test]
    fn block_on_re_polls_a_pending_future_until_it_completes() {
        // Discriminates a real re-queue from "poll once and return whatever
        // is in the output slot", which would return 0 here.
        let fut = make_future(poll_suspend_once, 0);
        assert_eq!(unsafe { nova_rt_task_block_on(fut) }, 42);
    }

    #[test]
    fn a_spawned_task_runs_to_completion_and_reports_done() {
        let fut = make_future(poll_suspend_once, 0);
        let id = unsafe { nova_rt_task_spawn(fut) };
        // Not done before the executor has run it at all.
        assert_eq!(unsafe { nova_rt_task_is_done(id) }, 0);
        let root = make_future(poll_ready_now, 0);
        unsafe { nova_rt_task_block_on(root) };
        assert_eq!(unsafe { nova_rt_task_is_done(id) }, 1);
        assert_eq!(unsafe { nova_rt_task_take_output(id) }, 42);
    }

    #[test]
    fn two_pending_tasks_interleave_round_robin() {
        // The determinism the sub-phase gate depends on. Records the order
        // poll calls actually happen in, and asserts an ALTERNATING order --
        // not merely that both ran, which a run-to-completion scheduler would
        // also satisfy while producing different output in the gate fixture.
        static ORDER: std::sync::Mutex<Vec<i64>> = std::sync::Mutex::new(Vec::new());
        ORDER.lock().expect("lock").clear();

        unsafe extern "C" fn poll_a(state: *mut u8, _c: *mut u8) -> i64 {
            record(state, 1)
        }
        unsafe extern "C" fn poll_b(state: *mut u8, _c: *mut u8) -> i64 {
            record(state, 2)
        }
        unsafe fn record(state: *mut u8, who: i64) -> i64 {
            let slots = state as *mut i64;
            let tag = *slots.add(STATE_SLOT_TAG);
            ORDER.lock().expect("lock").push(who);
            if tag < 2 {
                *slots.add(STATE_SLOT_TAG) = tag + 1;
                POLL_PENDING
            } else {
                *slots.add(STATE_SLOT_OUTPUT) = who;
                POLL_READY
            }
        }

        let a = make_future(poll_a, 0);
        let b = make_future(poll_b, 0);
        unsafe {
            nova_rt_task_spawn(a);
            nova_rt_task_spawn(b);
            nova_rt_task_block_on(make_future(poll_suspend_once, 0));
        }
        let order = ORDER.lock().expect("lock").clone();
        let a_positions: Vec<usize> =
            order.iter().enumerate().filter(|(_, &w)| w == 1).map(|(i, _)| i).collect();
        let b_positions: Vec<usize> =
            order.iter().enumerate().filter(|(_, &w)| w == 2).map(|(i, _)| i).collect();
        assert!(a_positions.len() >= 3 && b_positions.len() >= 3, "order = {order:?}");
        assert!(
            a_positions[0] < b_positions[0] && b_positions[0] < a_positions[1],
            "expected round-robin interleaving, got {order:?}"
        );
    }

> **CORRECTED AFTER TASK 3. The skeleton below is known-broken — read this first.**
>
> Task 3 wrote the same "null out the local, then `black_box`" technique and
> **measured that it does not work**: three of its five tests passed with
> `add_root` reduced to a no-op. Three independent leaks were found — a plain
> `usize` copy surviving the call; **`let`-shadowing not clearing the original
> stack slot** (exactly what `let fut = std::ptr::null_mut()` below does, it binds
> a new name rather than overwriting); and a **callee-saved register** retaining
> the value within one stack frame.
>
> **Against a conservative collector, "I dropped the reference" is not a testable
> claim.** The only reliable way to prove an object unreachable is to ensure no
> in-range bit pattern for it exists anywhere in the frame or in registers. Task 3
> built the tooling in `crates/nova-runtime/src/gc.rs`: `hide`/`reveal` (bitwise
> complement, both `#[inline(never)]`), `mut`-plus-reassignment instead of
> shadowing, and an `#[inline(never)]` setup fn to put the risky frame behind a
> call boundary. Reuse them; they may need `pub(crate)`.
>
> **Prove your version discriminates**: neuter `gc::add_root`, confirm the test
> FAILS, revert, rebuild. If it still passes, the test is worthless.
>
> **Gate it `#[cfg(windows)]`.** `stack_base()` returns `None` elsewhere, so
> `collect()` returns before marking and any `is_some()` assertion passes
> vacuously — Task 3 shipped exactly that and it red-lighted 2 of the 3 CI jobs.
>
> **Add the negative partner test** (an unspawned, unregistered state object must
> be swept by the same collection). The positive assertion alone passes both
> against an executor that registers nothing and against a collector that frees
> nothing.

    #[test]
    fn a_spawned_tasks_state_is_registered_as_a_gc_root() {
        // KNOWN-BROKEN SKELETON — the assertion's shape is right, the way it
        // makes the object unreachable is not. See the box above.
        let fut = make_future(poll_suspend_once, 0);
        let state = unsafe { (fut as *mut usize).add(1).read() };
        unsafe { nova_rt_task_spawn(fut) };
        let fut = std::ptr::null_mut::<u8>();
        std::hint::black_box(fut);
        gc::collect_for_test();
        assert!(
            gc::object_info(state).is_some(),
            "a parked task's state object was swept; add_root is not wired"
        );
    }

    #[test]
    fn a_re_entrant_block_on_panics() {
        // Nesting an executor inside a poll would run a task from inside
        // another task's frame. Diagnose it instead of corrupting the queue.
        //
        // `AssertUnwindSafe` is required: the closure captures a `*mut u8`,
        // and raw pointers are not `UnwindSafe`, so a bare `catch_unwind`
        // does not compile. Asserting unwind-safety is correct here — the
        // pointer is GC-owned and no invariant spans the panic.
        //
        // This test assumes the unwind panic strategy, which is the
        // workspace default. If a profile ever sets `panic = "abort"`, this
        // must become a subprocess test instead; report that rather than
        // deleting the test.
        unsafe extern "C" fn poll_reenters(_s: *mut u8, _c: *mut u8) -> i64 {
            let inner = make_future(poll_ready_now, 0);
            nova_rt_task_block_on(inner)
        }
        let fut = make_future(poll_reenters, 0);
        let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| unsafe {
            nova_rt_task_block_on(fut)
        }));
        assert!(r.is_err(), "re-entrant block_on must panic, not nest");
    }
}
```

`gc::collect_for_test` and `gc::object_info` are both currently `pub(crate)` or private (`gc.rs:182`, `:202`). Task 3 already tests inside `gc.rs`; here the calls come from a sibling module, so **expose them as `pub(crate)`** if they are not already, and report which you changed.

- [ ] **Step 2: Run the tests to verify they fail**

```bash
cargo test -p nova-runtime --no-fail-fast 2>&1 | tail -30
```

Expected: compile failure — `task` module does not exist.

- [ ] **Step 3: Implement the executor**

`crates/nova-runtime/src/task.rs`, single-threaded, thread-local state so the heap invariant is untouched:

```rust
//! A single-threaded cooperative executor.
//!
//! **Single-threaded is a correctness requirement, not a simplification.** The
//! collector keeps its entire heap in a `thread_local!` (`gc.rs:88`), so an
//! object allocated on one thread lives in that thread's heap map and only
//! that thread's collector can see or free it. A second thread running Nova
//! code would free objects the first still holds. See ADR 0009.

struct Task {
    poll: PollFn,
    state: *mut u8,
    done: bool,
    output: i64,
}

thread_local! {
    static QUEUE: RefCell<VecDeque<i64>> = const { RefCell::new(VecDeque::new()) };
    static TASKS: RefCell<Vec<Option<Task>>> = const { RefCell::new(Vec::new()) };
    static IN_BLOCK_ON: Cell<bool> = const { Cell::new(false) };
}
```

`spawn` allocates a task slot, `gc::add_root(state)`, pushes the id, returns it. On completion, store the output slot's value into `Task::output`, set `done`, and `gc::remove_root(state)` — the output is copied out *before* unrooting, or a heap output is freed before anyone reads it.

`block_on` sets `IN_BLOCK_ON` (panicking if already set), pushes the root task, then loops: pop an id, poll it, on `POLL_PENDING` push it to the **back** (round-robin), on `POLL_READY` finish it. Exit when the root task is done. Clear `IN_BLOCK_ON` on the way out, including on the panic path.

Then add the four `rt_funcs!` entries in `crates/nova-mir/src/lib.rs` with accurate signature doc comments, matching the existing style:

```rust
    /// `(ptr to { poll_code, state }) -> i64` — queue a task, return its id.
    TaskSpawn,
    /// `(ptr to { poll_code, state }) -> i64` — drive the executor until this
    /// future completes; returns its output slot.
    TaskBlockOn,
    /// `(i64 task_id) -> i8` — whether a spawned task has completed.
    TaskIsDone,
    /// `(i64 task_id) -> i64` — a completed task's output.
    TaskTakeOutput,
```

`rt_funcs!` generates `ALL` from this same list, so a variant cannot exist without being declared to both backends — that is the guarantee `02ccee6` bought and it needs nothing extra here.

- [ ] **Step 4: Run the tests to verify they pass**

```bash
cargo build --workspace && cargo test -p nova-runtime --no-fail-fast 2>&1 | tail -20
```

Expected: all seven PASS.

- [ ] **Step 5: Kill three mutations by hand**

1. Change the pending re-queue from `push_back` to `push_front`. Expected: `two_pending_tasks_interleave_round_robin` FAILS. (If it passes, the interleaving test is not discriminating and must be fixed before proceeding — this is the exact "exercises but does not discriminate" shape that cost 2.2b eight tasks.)
2. Delete the `gc::add_root(state)` call in `spawn`. Expected: `a_spawned_tasks_state_is_registered_as_a_gc_root` FAILS.
3. Delete the `gc::remove_root` call on completion. Expected: no test fails — **this is a genuine equivalent mutant for observable behaviour**, since the consequence is a leak, not a wrong answer. Record it as such rather than inventing a test; a leak check belongs with the reclamation test in `gc.rs`, and adding one is optional here.

Revert all three, then `cargo build --workspace`.

- [ ] **Step 6: Commit**

```bash
git add crates/nova-runtime crates/nova-mir/src/lib.rs
git commit -m "feat(runtime): single-threaded cooperative task executor

Ready queue, round-robin re-polling, block_on, spawn/is_done/output,
plus the four RtFunc entries. Single-threaded is a correctness
requirement: the GC heap is thread_local, so a second thread running
Nova code would free objects the first still holds (ADR 0009).

Driven in tests by hand-written poll functions shaped exactly like the
ones async_lower.rs will generate, so the state-object layout and the
poll ABI are pinned before any codegen depends on them. The
round-robin assertion checks an ALTERNATING order, not merely that both
tasks ran -- a run-to-completion scheduler satisfies the latter while
producing different gate output.

A spawned task's state is registered with the GC root registry, and the
output is copied out before unrooting so a heap output is not freed
before it is read. Re-entrant block_on panics rather than nesting.

Co-Authored-By: Claude Opus 5 <noreply@anthropic.com>"
```

---

### Task 5: The transform, part 1 — await-free async functions

Splits the transform in two so each half is independently reviewable. This half builds the state object, the poll function and the fat pointer, for an async function with **no awaits** — which is a complete, runnable end-to-end path (`async fn f() -> Int { 1 }` called via `block_on`).

**Files:**
- Create: `crates/nova-mir/src/async_lower.rs`
- Modify: `crates/nova-mir/src/lib.rs` — `mod async_lower;`
- Modify: `crates/nova-mir/src/mono.rs` — call the pass after the `Module` is built
- Test: `crates/nova-mir/src/async_lower.rs` `#[cfg(test)] mod tests`

**Interfaces:**
- Consumes: `hir::Function.is_async` (Task 2); the layout constants and `POLL_*` values from Task 4 (**re-declare them here with a comment pointing at `nova-runtime/src/task.rs` — `nova-mir` must not depend on `nova-runtime`; a test asserts the two agree**).
- Produces: `async_lower::transform(module: &mut Module)`. Every function that was async becomes `takes_env: true` with signature `(state_ptr, task_ctx) -> i64`, renamed with a `$poll` suffix; a wrapper under the original mangled name allocates the state and returns the `{poll_code, state}` fat pointer.

- [ ] **Step 1: Write the failing test**

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_await_free_async_fn_becomes_a_poll_fn_and_a_wrapper() {
        // Build a Module with one async function returning 1, run the pass,
        // and assert on the SHAPE it produced -- not merely that the pass ran.
        let mut m = /* see Step 3's helper */ module_with_async_const_fn(1);
        transform(&mut m);

        let poll = m.functions.iter().find(|f| f.name.ends_with("$poll"))
            .expect("a $poll function was produced");
        assert!(poll.takes_env, "the poll fn's env IS the state object");
        assert_eq!(poll.params, 1, "poll takes task_ctx as its one real param");
        assert_eq!(poll.ret, MirTy::I64, "poll returns a status, not the value");

        let wrapper = m.functions.iter().find(|f| !f.name.ends_with("$poll")
            && f.name.contains("f"))
            .expect("the original symbol survives as a wrapper");
        assert_eq!(wrapper.ret, MirTy::Ptr, "the wrapper returns a future");
        assert!(
            wrapper.blocks.iter().any(|b| b.stmts.iter().any(|s|
                matches!(s, Stmt::MakeClosure { .. }))),
            "the wrapper must build the {{poll_code, state}} fat pointer"
        );
    }

    #[test]
    fn the_poll_fn_writes_its_result_to_the_output_slot_not_the_return() {
        // Discriminates the real design from "return the value directly",
        // which would type-confuse a Float output through an i64 return.
        let mut m = module_with_async_const_fn(1);
        transform(&mut m);
        let poll = m.functions.iter().find(|f| f.name.ends_with("$poll")).expect("poll fn");
        assert!(
            poll.blocks.iter().any(|b| b.stmts.iter().any(|s|
                matches!(s, Stmt::SetField { index, .. } if *index == STATE_SLOT_OUTPUT as u32))),
            "the result must be stored to the output slot"
        );
    }

    #[test]
    fn a_non_async_function_is_left_byte_identical() {
        // The pass must be a no-op for everything else. Without this, a
        // greedy pass that rewrote every function would still pass the tests
        // above.
        let mut m = module_with_plain_fn();
        let before = format!("{:?}", m.functions);
        transform(&mut m);
        assert_eq!(before, format!("{:?}", m.functions));
    }

    #[test]
    fn the_state_layout_constants_match_the_runtime() {
        // nova-mir cannot depend on nova-runtime, so the layout is declared
        // twice. This is the pin that keeps the two copies honest -- a drift
        // here is a silent miscompile, the same hazard str_chars had as the
        // first intrinsic to build a Nova array in the runtime.
        assert_eq!(STATE_SLOT_TAG, 0);
        assert_eq!(STATE_SLOT_OUTPUT, 1);
        assert_eq!(STATE_SLOT_TEMPS, 2);
        assert_eq!(POLL_PENDING, 0);
        assert_eq!(POLL_READY, 1);
    }
}
```

- [ ] **Step 2: Run the test to verify it fails**

```bash
cargo test -p nova-mir --no-fail-fast 2>&1 | tail -30
```

Expected: compile failure — no `async_lower` module.

- [ ] **Step 3: Implement the await-free transform**

Write the `module_with_async_const_fn` / `module_with_plain_fn` helpers by hand, following the pattern `mono.rs`'s own `mod tests` (`:571`) already uses to build `Module`s field by field.

The transform, per function where the HIR was async:
1. Rename it to `<mangled>$poll`, set `takes_env = true`, `params = 1` (the `task_ctx`), `ret = MirTy::I64`.
2. Rewrite every temp access to a `RecordField`/`SetField` against the env at `STATE_SLOT_TEMPS + i`. `RecordField` and `SetField` already mirror each other's `8 * index` offset, which is exactly the layout Task 4's tests pin.
3. Replace each `Terminator::Return(Some(t))` with `SetField { record: env, index: STATE_SLOT_OUTPUT, value: t }` followed by a return of `POLL_READY`. `Return(None)` stores a unit and returns `POLL_READY` too.
4. Emit a new wrapper function under the original mangled name: allocate the state record with `STATE_SLOT_TEMPS + temps.len()` slots, store `0` into the tag slot, and `MakeClosure { code: "<mangled>$poll", .. }` to build the fat pointer.

The wrapper must **also copy the async function's real parameters into their temp slots** — an async fn's arguments are passed to the wrapper, not to `poll`.

- [ ] **Step 4: Run the tests to verify they pass**

```bash
cargo build --workspace && cargo test -p nova-mir --no-fail-fast 2>&1 | tail -20
```

Expected: all four PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/nova-mir
git commit -m "feat(mir): async transform for await-free functions

A post-monomorphization pass: each async fn becomes a \$poll function
whose env IS its heap state object, plus a wrapper under the original
symbol that allocates the state and returns the { poll_code, state }
fat pointer. Runs on the finished Module, so it sees no generics and
lands once for both backends.

The result goes to the state's output slot rather than the return value,
which stays an i64 status -- returning the value directly would
type-confuse a Float output through an i64 return.

The state layout is declared here AND in nova-runtime (nova-mir must not
depend on it); a test pins the two copies together, the same drift
hazard str_chars had as the first intrinsic to build a Nova array.

Co-Authored-By: Claude Opus 5 <noreply@anthropic.com>"
```

---

### Task 6: The transform, part 2 — splitting at await points

**Files:**
- Modify: `crates/nova-mir/src/async_lower.rs`
- Modify: `crates/nova-mir/src/lower.rs` — lower `hir::ExprKind::Await` to an await marker instead of the Task 2 diagnostic
- Test: `crates/nova-mir/src/async_lower.rs` `mod tests`; `tests/runtime/` probe

**Interfaces:**
- Consumes: everything from Task 5.
- Produces: an async function with N awaits becomes a poll function with N+1 resume states dispatched by a `Switch` on `STATE_SLOT_TAG`.

- [ ] **Step 1: Write the failing tests**

```rust
#[test]
fn one_await_produces_two_resume_states_dispatched_by_a_switch() {
    let mut m = module_with_async_fn_awaiting_once();
    transform(&mut m);
    let poll = m.functions.iter().find(|f| f.name.ends_with("$poll")).expect("poll fn");
    let entry = &poll.blocks[0];
    let arms = match &entry.term {
        Terminator::Switch { arms, .. } => arms.clone(),
        other => panic!("entry must dispatch on the resume tag, found {other:?}"),
    };
    assert!(arms.len() >= 2, "one await means two resume states, got {arms:?}");
    // The tags must be DISTINCT. A switch whose arms all target one block is
    // the mutation this test exists to kill.
    let targets: std::collections::HashSet<_> = arms.iter().map(|(_, b)| *b).collect();
    assert_eq!(targets.len(), arms.len(), "resume arms must target distinct blocks");
}

#[test]
fn a_suspend_stores_the_next_tag_before_returning_pending() {
    // Without the store, the task resumes at state 0 forever -- an infinite
    // loop, not a wrong value, which is why an output-only assertion misses it.
    let mut m = module_with_async_fn_awaiting_once();
    transform(&mut m);
    let poll = m.functions.iter().find(|f| f.name.ends_with("$poll")).expect("poll fn");
    let stores_tag = poll.blocks.iter().any(|b| {
        b.stmts.iter().any(|s| matches!(s,
            Stmt::SetField { index, .. } if *index == STATE_SLOT_TAG as u32))
    });
    assert!(stores_tag, "a suspend must advance the resume tag");
}
```

- [ ] **Step 2: Run to verify they fail**

```bash
cargo test -p nova-mir --no-fail-fast 2>&1 | tail -30
```

- [ ] **Step 3: Implement the split**

For each await in a block: everything after it becomes a new block; at the split point emit `SetField` of the next tag, then return `POLL_PENDING`. The entry block becomes `Switch { disc: <tag loaded from env>, arms: [(0, first), (1, resume_1), ...], default: Trap }`.

Awaiting a future means calling its poll function: load `{poll_code, state}` from the awaited temp, `CallIndirect` it, and branch on the status — `POLL_READY` falls through to the continuation (reading the inner future's output slot), `POLL_PENDING` suspends this task at the same tag so the await is retried. **Retried at the same tag, not the next one** — otherwise a future that pends twice is resumed past its own await.

- [ ] **Step 4: Run to verify they pass, then probe end to end**

```bash
cargo build --workspace && cargo test --workspace --no-fail-fast 2>&1 | tail -20
```

Task 7 supplies `block_on` to Nova code, so a full end-to-end run is not possible yet. Confirm here that `nova check` still accepts the Task 2 probe and that no `*_build_standalone` test regressed.

- [ ] **Step 5: Kill two mutations by hand**

1. Point every `Switch` arm at the same block. Expected: `one_await_produces_two_resume_states_dispatched_by_a_switch` FAILS.
2. Delete the tag `SetField` at a suspend. Expected: `a_suspend_stores_the_next_tag_before_returning_pending` FAILS.

Revert both, `cargo build --workspace`.

- [ ] **Step 6: Commit**

```bash
git add crates/nova-mir
git commit -m "feat(mir): split async bodies at await points

An async fn with N awaits becomes a poll fn with N+1 resume states,
dispatched by a Switch on the resume tag -- an existing terminator, so
neither backend needs a new construct. A suspend stores the next tag
before returning PENDING; without that store the task resumes at state
0 forever, which is a hang rather than a wrong value and so is invisible
to any output assertion.

Awaiting polls the inner future through CallIndirect and, on PENDING,
suspends at the SAME tag rather than the next one -- otherwise a future
that pends twice resumes past its own await.

Co-Authored-By: Claude Opus 5 <noreply@anthropic.com>"
```

---

### Task 7: `std/task` and `async fn main`

**Files:**
- Create: `std/task/lib.nova`
- Modify: `crates/nova-resolver/src/lib.rs:664` — `STD_MODULES` `[(&str, &str); 3]` → `; 4]`
- Modify: `crates/nova-driver/src/lib.rs` — wrap an `async fn main` in `block_on`
- Modify: `crates/nova-mir/src/lower.rs` — the four `RtFunc` call sites
- Test: `crates/nova-typeck/src/check.rs` `mod tests`; `crates/nova-cli/tests/run_tests.rs`

**Interfaces:**
- Consumes: Tasks 1–6.
- Produces:

```nova
pub fn spawn<T>(fut: Future<T>) -> JoinHandle<T>
pub record JoinHandle<T> { id: Int }
impl<T> JoinHandle<T> { pub async fn join(self) -> T }
pub async fn yield_now()
pub fn block_on<T>(fut: Future<T>) -> T
```

- [ ] **Step 1: Write the failing gate probe**

`tests/runtime/async_tasks.nova` — the fixture Task 8 registers. Two spawned tasks that each suspend twice, so the interleaving is visible in the output:

```nova
async fn counter(name: String, n: Int) -> Int {
    let mut i = 0
    while i < n {
        println("${name} step ${i}")
        yield_now().await
        i = i + 1
    }
    n
}

async fn run() -> Int {
    let a = spawn(counter("a", 3))
    let b = spawn(counter("b", 3))
    let ra = a.join().await
    let rb = b.join().await
    ra + rb
}

fn main() {
    println("total ${block_on(run())}")
}
```

Write `tests/runtime/async_tasks.stdout` **from the measured output**, not from what this plan predicts. The exact interleaving is determined by the executor's queue discipline, and predicting it here is precisely the "diagnostic measured on one shape written up as the answer for another" error this project has made on three consecutive branches. Run it, read it, check the order is genuinely alternating, then save it.

- [ ] **Step 2: Run to verify it fails**

```bash
cargo build --workspace && cargo run -q -p nova-cli -- run tests/runtime/async_tasks.nova
```

Expected: `E0001` — `spawn`, `yield_now`, `block_on` and `JoinHandle` do not exist.

- [ ] **Step 3: Write `std/task/lib.nova`**

`spawn`/`block_on` bind the Task 4 builtins. `yield_now` is the one async fn in std/task whose body must suspend exactly once — write it as a state machine the transform can express (a single await of an already-pending primitive), and if the transform cannot express it, **report that as a blocker rather than working around it**: it is the minimal suspension shape and everything else depends on it.

`JoinHandle::join` polls `nova_rt_task_is_done` and awaits until true, then reads `nova_rt_task_take_output`. Note the record-field-bound rule from ADR 0007: a bound on `JoinHandle`'s parameter is a resolution scope only, so put any `T`-bound on the `impl`.

- [ ] **Step 4: Add the 4th std module and the `async fn main` wrap**

`STD_MODULES` becomes `[(&str, &str); 4]` with `("$std.task", include_str!("../../../std/task/lib.nova"))`. 2.2b established that only the length annotation changes — every consumer iterates it. The driver allocates one `FileId` per entry, so verify `nova-driver/src/lib.rs:593-606`'s comment still reads true and update it if not.

For `async fn main`: the driver wraps it in `block_on`. `mono.rs` finds the entry point by the name `main`, so the wrapper must be what is called `main` — the same constraint 2.2e's synthesized `main` worked under.

- [ ] **Step 5: Run and measure**

```bash
cargo build --workspace && cargo run -q -p nova-cli -- run tests/runtime/async_tasks.nova
```

Save the measured output to `tests/runtime/async_tasks.stdout`. Then run the same fixture under `nova build` and `NOVA_GC_STRESS=1` and confirm all three agree **byte for byte**. A disagreement between backends is a finding, not something to normalize away.

- [ ] **Step 6: Add typeck tests for the module surface**

```rust
#[test]
fn std_task_spawn_and_join_typecheck() {
    let r = check_src(
        "async fn f() -> Float { 1.5 }\n\
         async fn g() -> Float { let h = spawn(f())\n h.join().await }\n\
         fn main() { }",
    );
    assert!(
        r.diagnostics.is_empty(),
        "expected no diagnostics, got {:?}",
        r.diagnostics.iter().map(|d| d.message.clone()).collect::<Vec<_>>()
    );
}

#[test]
fn block_on_outside_async_is_allowed() {
    // block_on is the entry point FROM sync code; it must not be E0086.
    let r = check_src(
        "async fn f() -> Int { 1 }\nfn main() { let x = block_on(f()) }",
    );
    assert!(
        !r.diagnostics.iter().any(|d| d.code == "E0086"),
        "block_on is a call, not an await"
    );
}
```

Note `Float` in the first test, per the Global Constraints.

- [ ] **Step 7: Commit**

```bash
git add std/task crates/nova-resolver crates/nova-driver crates/nova-mir tests/runtime crates/nova-typeck
git commit -m "feat(std): std/task -- spawn, JoinHandle, join, yield_now, block_on

The fourth embedded std module; STD_MODULES 3 -> 4, which needs only the
length annotation since every consumer iterates it. An async fn main is
wrapped in block_on by the driver, and the wrapper is what is named main
because mono finds the entry point by name.

block_on is a real std/task export, not just the driver's private entry:
it is how async gets tested via @test fn t() { block_on(f()) } with no
change to the test runner.

The gate fixture's expected output is MEASURED, not predicted -- the
interleaving depends on the executor's queue discipline.

Co-Authored-By: Claude Opus 5 <noreply@anthropic.com>"
```

---

### Task 8: Gate, ADR 0009, and the document sweep

**Files:**
- Modify: `crates/nova-cli/tests/run_tests.rs` — three registrations
- Create: `docs/adr/0009-async-execution-model.md`
- Modify: `docs/phase-2-plan.md` — decision 1, **edited in place**
- Modify: `CHANGELOG.md`

- [ ] **Step 1: Register the three gate configurations**

Copy the `std_core` trio at `crates/nova-cli/tests/run_tests.rs:1080-1112` exactly — `async_tasks_runs`, `async_tasks_build_standalone`, `async_tasks_under_gc_stress` — reading expected output from `tests/runtime/async_tasks.stdout` with the same `.replace("\r\n", "\n")` normalization.

- [ ] **Step 2: Verify the full gate**

```bash
cargo build --workspace && cargo test --workspace --no-fail-fast 2>&1 | tail -25
cargo clippy --workspace --all-targets -- -D warnings 2>&1 | tail -5
cargo fmt --check
```

Expected: 21 gate configurations, 0 failed, clippy and fmt clean. **Report the actual test count**; do not restate a number from this plan.

- [ ] **Step 3: Write ADR 0009**

Follow the house format of `docs/adr/0008-attributes-and-test-isolation.md`. Record: the model (single-threaded cooperative state machines); that it **reverses `docs/phase-2-plan.md` decision 1**, with the `thread_local!` GC measurement as the reason; what is given up (real parallelism, `spawn_blocking`); and the known gaps — no parking or waking, no cancellation, no async trait methods, and all temps spilled into the state object rather than only those live across a suspend.

- [ ] **Step 4: Sweep the documents that assert the old decision**

Edit `docs/phase-2-plan.md` decision 1 **in place** to record that (b) was chosen against and point at ADR 0009. Then grep for anything else asserting thread-per-task or Tokio-for-tasks:

```bash
grep -rn "thread-per-task\|thread per task" docs/ nova-spec/
```

**This step is not bookkeeping.** The 2.2a debt branch — a branch whose entire subject was documented-but-unenforced claims — shipped three new instances of exactly this, including a `CHANGELOG.md` asserting the old behaviour 57 lines above the entry announcing the new one, both in release 0.2.0. A commit that changes a decision must sweep every document asserting the old one.

- [ ] **Step 5: Update the CHANGELOG**

Add to `[Unreleased]`, in the established voice: what shipped, the execution model and its ADR, and the three known gaps.

- [ ] **Step 6: Commit**

```bash
git add crates/nova-cli/tests/run_tests.rs docs CHANGELOG.md
git commit -m "test(gate): async gate fixture, ADR 0009, document sweep

Three gate configurations (run/build/GC-stress) take the total 18 -> 21.
ADR 0009 records the single-threaded state-machine model and its
reversal of phase-2-plan.md decision 1, which is edited in place rather
than left contradicting it -- the 2.2a debt branch shipped three
instances of exactly that, one of them in the same CHANGELOG release.

Co-Authored-By: Claude Opus 5 <noreply@anthropic.com>"
```

---

## Plan Self-Review

**Spec coverage.** Every spec section maps to a task: §2/§2.1/§2.2 model → ADR in Task 8; §4 transform → Tasks 5–6; §4.1 all-temps → Task 5 Step 3 and ADR gaps; §4.2 poll ABI → Task 4 interfaces; §5 `Ty::Future` → Task 1; §5.1 both name sites → Task 1 Step 4; §6 GC registry → Task 3; §7 diagnostics and the `E0900` lift → Task 2; §8 `std/task` → Task 7; §9 gate → Task 8; §10 mutations → the hand-mutation steps in Tasks 1, 3, 4, 6; §11 risks → carried into the task that owns each; §12 done → Task 8.

**One spec item deliberately not its own task:** §11.4's `gc::object_info` layout pin. It is folded into Task 4 Step 1 (`a_spawned_tasks_state_is_registered_as_a_gc_root` reads the real tracked object) and Task 5's `the_state_layout_constants_match_the_runtime`, because a separate task for one assertion would not carry its own test cycle.

**Type consistency.** `STATE_SLOT_TAG`/`STATE_SLOT_OUTPUT`/`STATE_SLOT_TEMPS`, `POLL_PENDING`/`POLL_READY` and `PollFn` are declared once in Task 4's Interfaces and used unchanged in Tasks 5–6. `gc::add_root`/`gc::remove_root` are declared in Task 3 and consumed in Task 4. `hir::Function.is_async` is declared in Task 2 and consumed in Task 5. `hir::ExprKind::Await` is declared in Task 2 and consumed in Task 6.

**Known ordering constraint.** Tasks 3 and 4 depend on neither 1 nor 2 and can run in parallel with them. Tasks 5 → 6 → 7 → 8 are strictly sequential. **Dispatch fixers sequentially regardless** — the 2.2a debt branch established that two agents mutating one checkout trip over each other's temporary mutations and each other's phantom `git status` entries.
