# Phase 2.1 — Compiler Prerequisites + `std/core` Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Ship `std/core` — Nova's first real standard-library module, written in Nova — after landing the three compiler features it requires.

**Architecture:** Two stages. Stage 1 (Tasks 1–6) adds compiler features in Rust: a `panic` builtin, a string-compare runtime function, correct signatures for self-less methods, associated-function call syntax (`T::new()`), and supertrait enforcement. Stage 2 (Tasks 7–10) writes `std/core/lib.nova` and loads it through the existing implicit-prelude mechanism, replacing the two-line `PRELUDE_SRC`.

**Tech Stack:** Rust 2021 workspace (`nova-lexer`, `nova-parser`, `nova-resolver`, `nova-typeck`, `nova-hir`, `nova-mir`, `nova-codegen-cranelift`, `nova-codegen-llvm`, `nova-runtime`, `nova-driver`, `nova-cli`); Nova source for `std/core`.

**Spec:** `docs/superpowers/specs/2026-07-25-phase-2-1-std-core-design.md`

## Global Constraints

- Repo root: `D:\Projects\nona\nova`. All `cargo` commands run there.
- Every task ends green on all three: `cargo test --workspace`, `cargo clippy --all-targets --all-features -- -D warnings`, `cargo fmt --check`.
- TDD is mandatory: write the failing test, **run it and see it fail**, then implement.
- Conventional commits, one logical change per commit, ending with:
  `Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>`
- No `unwrap()`/`expect()` in library code paths that can fail on user input (repo convention, `agent.md`). Tests may use them.
- Do **not** `git push`. The user pushes explicitly.
- Diagnostic codes are reused from the existing scheme: `E0001` unresolved name, `E0010` type mismatch, `E0014` bad method/field access, `E0016` wrong arity, `E0072` conformance, `E0900` unsupported.
- Nova primitives: `Int` is i64, `Float` f64, `Bool` i8, `Char` i64, `String` is a GC'd `NovaStr*`.

---

## File Structure

| File | Responsibility | Tasks |
|---|---|---|
| `crates/nova-runtime/src/lib.rs` | Add `nova_rt_panic_str`, `nova_rt_str_cmp`; register both in `symbols()` | 1, 2 |
| `crates/nova-mir/src/lib.rs` | `RtFunc::Panic`, `RtFunc::StrCmp` + symbol names + signatures | 1, 2 |
| `crates/nova-mir/src/lower.rs` | Map `Builtin::Panic` → `RtFunc::Panic` | 1 |
| `crates/nova-codegen-cranelift/src/lib.rs` | Add both to the `RT_FUNCS` declaration list | 1, 2 |
| `crates/nova-resolver/src/lib.rs` | `Builtin::Panic`; `STD_CORE_SRC` + `FileId` seam for the prelude | 1, 7 |
| `crates/nova-typeck/src/check.rs` | `panic` typing; self-less method signatures; `T::f()` calls; supertrait enforcement | 1, 3, 4, 5, 6 |
| `std/core/lib.nova` | **New.** `Option`/`Result` + methods, core traits, primitive impls | 7, 8, 9 |
| `crates/nova-cli/tests/run_tests.rs` | e2e tests under `nova run` and `nova build` | 10 |
| `tests/runtime/std_core.nova` + `.stdout` | **New.** e2e fixture | 10 |
| `docs/adr/0004-stdlib-compile-model.md` | **New.** Records the compile-model decision | 7 |
| `CHANGELOG.md` | `[Unreleased]` entries | 10 |

---

### Task 1: `panic(msg: String)` builtin

**Why:** `Result::unwrap` cannot be written without it. `panic` types as `Ty::Never`, and MIR already has **13 divergence guards** all keyed off `matches!(e.ty, Ty::Never)` (`crates/nova-mir/src/lower.rs:111`), covering `Let`, `Assign`, `if` joins, and match arms — so no new MIR work is needed.

**Files:**
- Modify: `crates/nova-runtime/src/lib.rs` (add `nova_rt_panic_str` near `nova_rt_panic`, ~line 155; add to `symbols()` ~line 170)
- Modify: `crates/nova-mir/src/lib.rs:102` (enum `RtFunc`), `:129` (`symbol`), `:145` (`signature`)
- Modify: `crates/nova-mir/src/lower.rs:558-562` (`Builtin` → `RtFunc`)
- Modify: `crates/nova-codegen-cranelift/src/lib.rs:183-192` (`RT_FUNCS`)
- Modify: `crates/nova-resolver/src/lib.rs:26-44` (`Builtin` enum, `name()`, `ALL`)
- Modify: `crates/nova-typeck/src/check.rs:2038-2079` (`check_builtin_call`)
- Test: `crates/nova-typeck/src/check.rs` (inline `mod tests`), `crates/nova-cli/tests/run_tests.rs`

**Interfaces:**
- Produces: `Builtin::Panic` with `name() == "panic"`; `RtFunc::Panic` with symbol `"nova_rt_panic_str"`, signature `(vec![MirTy::Ptr], MirTy::Unit)`; `panic(String) -> Ty::Never` at the call site. Task 8 uses `panic(...)` in Nova source.

- [ ] **Step 1: Write the failing typeck test**

Add to the inline `mod tests` in `crates/nova-typeck/src/check.rs` (alongside `async_trait_method_declaration_reports_e0900`):

```rust
    #[test]
    fn panic_typechecks_as_never_in_match_arm() {
        // `panic` diverges, so the match's type comes from the other arm.
        let r = check_src(
            "fn get(o: Option<Int>) -> Int {\n\
                 match o { Some(v) => v, None => panic(\"none\") }\n\
             }\n\
             fn main() { println(\"${get(Some(3))}\") }",
        );
        assert!(r.diagnostics.is_empty(), "{:?}", r.diagnostics);
    }

    #[test]
    fn panic_rejects_non_string_argument() {
        let r = check_src("fn main() { panic(7) }");
        assert!(error_codes(&r).contains(&"E0010"), "{:?}", r.diagnostics);
    }
```

- [ ] **Step 2: Run the tests and watch them fail**

```bash
cargo test -p nova-typeck --lib -- panic_typechecks_as_never panic_rejects_non_string
```

Expected: both FAIL. `panic_typechecks_as_never_in_match_arm` fails with a diagnostic list containing `E0001` ("cannot find function `panic` in this scope"); `panic_rejects_non_string_argument` fails because the codes list is `["E0001"]`, not containing `E0010`.

- [ ] **Step 3: Add the runtime shim**

In `crates/nova-runtime/src/lib.rs`, after `nova_rt_panic` (~line 160). Note `NovaStr`'s existing field names — mirror exactly how `nova_rt_println` reads its argument (read that function first and match its field access):

```rust
/// Abort the program with a panic message given as a Nova string.
///
/// # Safety
/// `s` must point to a valid `NovaStr`.
#[no_mangle]
pub unsafe extern "C" fn nova_rt_panic_str(s: *const NovaStr) -> ! {
    let msg = if s.is_null() { "" } else { (*s).as_str() };
    eprintln!("nova: panic: {msg}");
    std::process::abort();
}
```

If `NovaStr` has no `as_str()`, use the same ptr/len access `nova_rt_println` uses. Then register it in `symbols()`:

```rust
        ("nova_rt_panic_str", nova_rt_panic_str as *const u8),
```

- [ ] **Step 4: Add `RtFunc::Panic`**

`crates/nova-mir/src/lib.rs` — in `enum RtFunc` (after `CheckBounds`):

```rust
    /// `(str) -> !` — abort with a message.
    Panic,
```

In `symbol()`:

```rust
            RtFunc::Panic => "nova_rt_panic_str",
```

In `signature()` — declared as returning `Unit` because the call never returns; codegen needs no special case:

```rust
            RtFunc::Panic => (vec![MirTy::Ptr], MirTy::Unit),
```

- [ ] **Step 5: Add `Builtin::Panic` and lower it**

`crates/nova-resolver/src/lib.rs`: add `Panic` to `enum Builtin`, `Builtin::Panic => "panic"` to `name()`, and widen `ALL` to 3:

```rust
    pub const ALL: [Builtin; 3] = [Builtin::Println, Builtin::Print, Builtin::Panic];
```

`crates/nova-mir/src/lower.rs:558-562`:

```rust
                    Builtin::Panic => RtFunc::Panic,
```

`crates/nova-codegen-cranelift/src/lib.rs` — add to `RT_FUNCS`:

```rust
    RtFunc::Panic,
```

- [ ] **Step 6: Type `panic` in typeck**

In `crates/nova-typeck/src/check.rs`, `check_builtin_call` (~line 2038). The existing arm is `Builtin::Println | Builtin::Print => { … }` and ends by returning an expr with `ty: Ty::Unit`. Restructure so all three builtins share the one-String-argument checking but `Panic` yields `Ty::Never`:

```rust
            Builtin::Println | Builtin::Print | Builtin::Panic => {
                // …existing arity + String-argument checking, unchanged…
                let ty = if matches!(builtin, Builtin::Panic) {
                    Ty::Never
                } else {
                    Ty::Unit
                };
                hir::Expr {
                    kind: hir::ExprKind::Call {
                        func: hir::Callee::Builtin(builtin),
                        type_args: Vec::new(),
                        args: vec![arg],
                    },
                    ty,
                    span,
                }
            }
```

- [ ] **Step 7: Run the tests and watch them pass**

```bash
cargo test -p nova-typeck --lib -- panic_typechecks_as_never panic_rejects_non_string
```

Expected: both PASS.

- [ ] **Step 8: Add the e2e test**

Create `tests/runtime/panic_unwrap.nova`:

```nova
fn get(o: Option<Int>) -> Int {
    match o { Some(v) => v, None => panic("called get on None") }
}

fn main() {
    println("${get(Some(7))}")
    let n = get(None)
    println("unreachable ${n}")
}
```

Add to `crates/nova-cli/tests/run_tests.rs`:

```rust
#[test]
fn panic_aborts_with_message() {
    let assert = nova()
        .arg("run")
        .arg(repo_root().join("tests/runtime/panic_unwrap.nova"))
        .assert()
        .failure();
    let out = assert.get_output();
    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stdout.contains('7'), "stdout was {stdout:?}");
    assert!(
        stderr.contains("nova: panic: called get on None"),
        "stderr was {stderr:?}"
    );
    assert!(!stdout.contains("unreachable"), "stdout was {stdout:?}");
}
```

- [ ] **Step 9: Verify everything**

```bash
cargo test --workspace && cargo clippy --all-targets --all-features -- -D warnings && cargo fmt --check
```

Expected: all pass. If `fmt` complains, run `cargo fmt` and re-run.

- [ ] **Step 10: Commit**

```bash
git add -A && git commit -m "feat(typeck): panic builtin

`panic(msg: String)` aborts with a message. It types as `Ty::Never`, so the
existing divergence guards in MIR lowering handle it with no new lowering
work: it composes in match arms, if joins, lets, and tail position.

Runtime `nova_rt_panic_str` takes a Nova string and mirrors the existing
`nova_rt_panic` ptr/len entry point.

Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>"
```

---

### Task 2: `nova_rt_str_cmp` runtime function

**Why:** `Ord for String` cannot be written in Nova — `String` has no length or indexing, and `String < String` is `E0013`. Verified: `Int`/`Float`/`Char` support `<`; `Bool` and `String` do not.

**Files:**
- Modify: `crates/nova-runtime/src/lib.rs` (beside `nova_rt_str_eq`; add to `symbols()`)
- Modify: `crates/nova-mir/src/lib.rs` (`RtFunc::StrCmp` + symbol + signature)
- Modify: `crates/nova-codegen-cranelift/src/lib.rs` (`RT_FUNCS`)
- Test: `crates/nova-runtime/src/lib.rs` (inline `mod tests`)

**Interfaces:**
- Consumes: `RtFunc` from Task 1.
- Produces: `nova_rt_str_cmp(a, b) -> i64` returning `-1`/`0`/`1` (byte-lexicographic). `RtFunc::StrCmp` signature `(vec![MirTy::Ptr, MirTy::Ptr], MirTy::I64)`. Task 9 reaches it through an `extern` declaration in `std/core/lib.nova`.

- [ ] **Step 1: Write the failing runtime test**

In `crates/nova-runtime/src/lib.rs`'s inline `mod tests` (match how existing tests there construct a `NovaStr` — read one first):

```rust
    #[test]
    fn str_cmp_orders_lexicographically() {
        let a = make_str("abc");
        let b = make_str("abd");
        let c = make_str("abc");
        unsafe {
            assert_eq!(nova_rt_str_cmp(a, b), -1);
            assert_eq!(nova_rt_str_cmp(b, a), 1);
            assert_eq!(nova_rt_str_cmp(a, c), 0);
        }
    }

    #[test]
    fn str_cmp_prefix_is_less() {
        let a = make_str("ab");
        let b = make_str("abc");
        unsafe { assert_eq!(nova_rt_str_cmp(a, b), -1) };
    }
```

If no `make_str` helper exists in that test module, write one using the same construction `nova_rt_str_new` uses, and reuse it.

- [ ] **Step 2: Run the tests and watch them fail**

```bash
cargo test -p nova-runtime --lib -- str_cmp
```

Expected: FAIL to compile — `cannot find function 'nova_rt_str_cmp' in this scope`.

- [ ] **Step 3: Implement it**

In `crates/nova-runtime/src/lib.rs`, beside `nova_rt_str_eq`:

```rust
/// Byte-lexicographic comparison of two Nova strings: `-1`, `0`, or `1`.
///
/// # Safety
/// `a` and `b` must point to valid `NovaStr` values.
#[no_mangle]
pub unsafe extern "C" fn nova_rt_str_cmp(a: *const NovaStr, b: *const NovaStr) -> i64 {
    let (x, y) = ((*a).as_str(), (*b).as_str());
    match x.as_bytes().cmp(y.as_bytes()) {
        std::cmp::Ordering::Less => -1,
        std::cmp::Ordering::Equal => 0,
        std::cmp::Ordering::Greater => 1,
    }
}
```

Use the same field/`as_str` access that `nova_rt_str_eq` uses. Register in `symbols()`:

```rust
        ("nova_rt_str_cmp", nova_rt_str_cmp as *const u8),
```

- [ ] **Step 4: Wire it into MIR and Cranelift**

`crates/nova-mir/src/lib.rs` — `enum RtFunc`:

```rust
    /// `(str, str) -> i64` — lexicographic compare: -1, 0, or 1.
    StrCmp,
```

`symbol()`:

```rust
            RtFunc::StrCmp => "nova_rt_str_cmp",
```

`signature()`:

```rust
            RtFunc::StrCmp => (vec![MirTy::Ptr, MirTy::Ptr], MirTy::I64),
```

`crates/nova-codegen-cranelift/src/lib.rs` — add `RtFunc::StrCmp,` to `RT_FUNCS`.

- [ ] **Step 5: Run the tests and watch them pass**

```bash
cargo test -p nova-runtime --lib -- str_cmp
```

Expected: both PASS.

- [ ] **Step 6: Verify and commit**

```bash
cargo test --workspace && cargo clippy --all-targets --all-features -- -D warnings && cargo fmt --check
git add -A && git commit -m "feat(runtime): nova_rt_str_cmp for string ordering

Byte-lexicographic compare returning -1/0/1. Needed because \`Ord for String\`
cannot be written in Nova: String has neither length nor indexing, and
\`String < String\` is E0013. Also needed by any sorted collection later.

Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>"
```

---

### Task 3: Fix self-less method signatures (root-cause bug fix)

**Why:** `collect_impls` unconditionally prepends the self type to **every** impl method's signature (`crates/nova-typeck/src/check.rs`, `params.insert(0, self_ty.clone())`), whether or not the method declares a `self` receiver. `check_fn_body` then **zips** `f.params` against `sig.params` (`:1306`), so for a self-less method every parameter is shifted by one. One root cause, three verified symptoms:

| Program | Today |
|---|---|
| `impl P { fn make(x: Int) -> Int { x + 1 } }` | false `E0010: mismatched operand types: 'P' vs 'Int'` — `x` mistyped as `P`. Where types coincidentally align this is a silent **miscompile**. |
| `P::new()` | `E0001: no variant 'new' on type 'P'` |
| `p.make()` (self-less, called on an instance) | `nova check` says `ok`, then Cranelift: `mismatched argument count: got 1, expected 0` → `internal codegen error` |

This task fixes the signature and rejects the instance call. Task 4 adds `T::f()` call syntax.

**Files:**
- Modify: `crates/nova-typeck/src/check.rs` — `Checker` struct (add `selfless` field), `collect_impls` (conditional insert), `emit_inherent_call`, `find_inherent_method`
- Test: `crates/nova-typeck/src/check.rs` inline `mod tests`

**Interfaces:**
- Produces: `Checker.selfless: FxHashSet<DefId>` — impl-method `DefId`s whose AST declares no `self` receiver. For those, `sigs[def_id].params` holds **only** the declared parameters (no prepended self). Tasks 4 and 5 read `selfless`.

- [ ] **Step 1: Write the failing tests**

Add to the inline `mod tests` in `crates/nova-typeck/src/check.rs`:

```rust
    #[test]
    fn selfless_method_params_are_not_shifted() {
        // `x: Int` must stay Int; a wrongly prepended `self` shifted it to `P`.
        let r = check_src(
            "record P { v: Int }\n\
             impl P { fn make(x: Int) -> Int { x + 1 } }\n\
             fn main() { println(\"ok\") }",
        );
        assert!(r.diagnostics.is_empty(), "{:?}", r.diagnostics);
    }

    #[test]
    fn selfless_method_called_on_instance_reports_e0014() {
        // Must be a clean diagnostic, never a codegen ICE.
        let r = check_src(
            "record P { v: Int }\n\
             impl P { fn make() -> P { P { v: 7 } } }\n\
             fn main() { let p = P { v: 0 }\n let q = p.make()\n println(\"${q.v}\") }",
        );
        assert!(error_codes(&r).contains(&"E0014"), "{:?}", r.diagnostics);
    }
```

- [ ] **Step 2: Run the tests and watch them fail**

```bash
cargo test -p nova-typeck --lib -- selfless_method_params_are_not_shifted selfless_method_called_on_instance
```

Expected: `selfless_method_params_are_not_shifted` FAILS with a diagnostics list containing `E0010` (`mismatched operand types: 'P' vs 'Int'`). `selfless_method_called_on_instance_reports_e0014` FAILS with an empty `[]` list (currently accepted, then ICEs at codegen).

- [ ] **Step 3: Track self-less methods**

Add the field to `struct Checker` (beside `sigs`/`method_locs`):

```rust
    /// Impl methods that declare no `self` receiver (associated functions).
    /// Their `sigs` entry holds only the declared parameters — no prepended
    /// self type — so `check_fn_body`'s params/sig zip stays aligned.
    selfless: FxHashSet<DefId>,
```

Initialise it wherever `Checker` is constructed (`FxHashSet::default()`). Confirm `FxHashSet` is imported in this file; if only `FxHashMap` is, add it to the `rustc_hash` import.

- [ ] **Step 4: Make the self insert conditional**

In `collect_impls`, replace the unconditional insert:

```rust
                let (mut params, ret) = self.method_sig_parts(&f.params, &f.return_ty, &scope);
                params.insert(0, self_ty.clone());
```

with:

```rust
                let (mut params, ret) = self.method_sig_parts(&f.params, &f.return_ty, &scope);
                // `self` is stripped by `method_sig_parts` and re-inserted as the
                // receiver — but only for methods that actually declare one. For
                // an associated function (`fn new() -> P`) inserting it would
                // shift every parameter by one against `f.params`, which
                // `check_fn_body` zips positionally.
                let has_self = f.params.first().is_some_and(|p| p.name.value == "self");
                if has_self {
                    params.insert(0, self_ty.clone());
                } else {
                    self.selfless.insert(def_id);
                }
```

- [ ] **Step 5: Reject instance calls on self-less methods**

`find_inherent_method` is the lookup used by `resolve_method_on` for receiver-based calls. Exclude self-less methods there so a `p.make()` call reports the existing `E0014: no method 'make' on type 'P'` instead of dispatching:

```rust
    fn find_inherent_method(&self, recv_ty: &Ty, head: TyHead, name: &str) -> Option<DefId> {
        self.impls
            .iter()
            .filter(|i| i.trait_id.is_none() && i.self_head == head)
            // The receiver must fit the impl's self-type pattern, not just its
            // head, so `impl<T> Pair<T, T>` is skipped for `Pair<Int, String>`.
            .filter(|i| i.match_args(recv_ty).is_some())
            .find_map(|i| {
                i.methods
                    .iter()
                    .find(|(n, d)| n == name && !self.selfless.contains(d))
                    .map(|(_, d)| *d)
            })
    }
```

- [ ] **Step 6: Guard `emit_inherent_call`**

`emit_inherent_call` computes `let expected_args = sig.params.len().saturating_sub(1);` and unifies `sig.params[0]` with the receiver. That is now valid only for methods with a receiver. Add a defensive guard at its top so no future caller can reintroduce the ICE:

```rust
        if self.selfless.contains(&def_id) {
            let mname = self.defs.def(def_id).name.clone();
            self.error(
                "E0014",
                format!(
                    "`{mname}` is an associated function with no `self` receiver; \
                     call it as `Type::{mname}(…)`"
                ),
                span,
            );
            return error_expr(span);
        }
```

- [ ] **Step 7: Run the tests and watch them pass**

```bash
cargo test -p nova-typeck --lib -- selfless_method_params_are_not_shifted selfless_method_called_on_instance
```

Expected: both PASS.

- [ ] **Step 8: Confirm the ICE is gone end-to-end**

```bash
cargo build -p nova-cli
```

Write the offending program to a scratch file and run it:

```bash
printf 'record P { v: Int }\nimpl P { fn make() -> P { P { v: 7 } } }\nfn main() { let p = P { v: 0 }\n let q = p.make()\n println("${q.v}") }\n' > /d/tmp/ice.nova
./target/debug/nova.exe run /d/tmp/ice.nova
```

Expected: a clean `error[E0014]` naming `make`, exit 1, and **no** "internal codegen error" and no Cranelift verifier dump.

- [ ] **Step 9: Verify and commit**

```bash
cargo test --workspace && cargo clippy --all-targets --all-features -- -D warnings && cargo fmt --check
git add -A && git commit -m "fix(typeck): don't prepend self to associated-function signatures

collect_impls inserted the self type at params[0] for every impl method,
including ones declaring no \`self\`. check_fn_body zips f.params against
sig.params positionally, so an associated function's parameters were all
shifted by one — \`fn make(x: Int)\` typed \`x\` as the self type. Three
symptoms from the one cause: false type errors (a silent miscompile wherever
the shifted types happened to align), a bogus \"no variant\" error for
\`P::new()\`, and a Cranelift verifier ICE (\"got 1, expected 0\") for a
self-less method called on an instance, which \`nova check\` accepted.

The insert is now conditional on an actual \`self\` receiver; self-less methods
are tracked so receiver-based lookup skips them and reports E0014.

Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>"
```

---

### Task 4: Associated-function calls on concrete types (`P::new()`)

**Why:** `Default::default()` and every constructor need it. `check_call` (`crates/nova-typeck/src/check.rs:1873`) handles a `Path` callee only when `path.segments.len() == 1`; a two-segment callee falls through to `check_path` (`:1778`), which treats `Type::Name` as a qualified **variant** and errors `no variant 'new' on type 'P'`.

**Files:**
- Modify: `crates/nova-typeck/src/check.rs` — `check_call` (new two-segment branch), new `emit_assoc_call` helper
- Test: `crates/nova-typeck/src/check.rs` inline `mod tests`

**Interfaces:**
- Consumes: `Checker.selfless` (Task 3).
- Produces: `fn emit_assoc_call(&mut self, fcx: &mut FnCtx, def_id: DefId, args: Vec<hir::Expr>, span: Span) -> hir::Expr` — emits `hir::ExprKind::Call { func: hir::Callee::Def(def_id), type_args, args }` with fresh inference vars for the impl's generics. Task 9 relies on `Int::default()` resolving.

- [ ] **Step 1: Write the failing tests**

```rust
    #[test]
    fn associated_function_call_on_concrete_type() {
        let r = check_src(
            "record P { v: Int }\n\
             impl P { fn new() -> P { P { v: 7 } } }\n\
             fn main() { let p = P::new()\n println(\"${p.v}\") }",
        );
        assert!(r.diagnostics.is_empty(), "{:?}", r.diagnostics);
    }

    #[test]
    fn associated_function_with_args() {
        let r = check_src(
            "record P { v: Int }\n\
             impl P { fn of(x: Int) -> P { P { v: x } } }\n\
             fn main() { let p = P::of(5)\n println(\"${p.v}\") }",
        );
        assert!(r.diagnostics.is_empty(), "{:?}", r.diagnostics);
    }

    #[test]
    fn associated_function_wrong_arity_reports_e0016() {
        let r = check_src(
            "record P { v: Int }\n\
             impl P { fn of(x: Int) -> P { P { v: x } } }\n\
             fn main() { let p = P::of()\n println(\"${p.v}\") }",
        );
        assert!(error_codes(&r).contains(&"E0016"), "{:?}", r.diagnostics);
    }

    #[test]
    fn unknown_associated_function_still_reports_e0001() {
        let r = check_src(
            "record P { v: Int }\n\
             fn main() { let p = P::nope()\n println(\"x\") }",
        );
        assert!(error_codes(&r).contains(&"E0001"), "{:?}", r.diagnostics);
    }
```

- [ ] **Step 2: Run the tests and watch them fail**

```bash
cargo test -p nova-typeck --lib -- associated_function unknown_associated_function
```

Expected: the first three FAIL with a diagnostics list containing `E0001` ("no variant 'new'/'of' on type 'P'"). `unknown_associated_function_still_reports_e0001` should already PASS — it is a guard that the existing error path survives.

- [ ] **Step 3: Add the two-segment call branch**

In `check_call`, immediately after the existing `if path.segments.len() == 1 { … }` block and before the fallback that treats the callee as a value expression, insert:

```rust
            if path.segments.len() == 2 {
                let ty_name = path.segments[0].value.as_str();
                let fn_name = path.segments[1].value.as_str();
                if let Some(type_id) = self.defs.resolve_type(self.cur_module, ty_name) {
                    // `Type::Variant(args)` keeps its existing meaning.
                    if let Some(vi) = self.variant_index(type_id, fn_name) {
                        let checked: Vec<hir::Expr> =
                            args.iter().map(|a| self.check_expr(fcx, a)).collect();
                        return self.make_variant(fcx, type_id, vi, checked, span);
                    }
                    // Otherwise: an associated function on an inherent impl.
                    if let Some(def_id) = self.find_assoc_fn(type_id, fn_name) {
                        let checked: Vec<hir::Expr> =
                            args.iter().map(|a| self.check_expr(fcx, a)).collect();
                        return self.emit_assoc_call(fcx, def_id, checked, span);
                    }
                }
            }
```

- [ ] **Step 4: Add the lookup and emit helpers**

Add beside `find_inherent_method`. `find_assoc_fn` keys off the impl's self-type **head** (an associated function has no receiver to structurally match against):

```rust
    /// Find a self-less method named `name` on an inherent impl of the type
    /// `type_id`. Associated functions have no receiver, so selection is by the
    /// impl's nominal head only.
    fn find_assoc_fn(&self, type_id: DefId, name: &str) -> Option<DefId> {
        let head = TyHead::Named(type_id);
        self.impls
            .iter()
            .filter(|i| i.trait_id.is_none() && i.self_head == head)
            .find_map(|i| {
                i.methods
                    .iter()
                    .find(|(n, d)| n == name && self.selfless.contains(d))
                    .map(|(_, d)| *d)
            })
    }

    /// Emit a call to an associated function. The impl's generic parameters
    /// cannot be recovered from a receiver, so they become fresh inference
    /// variables resolved by the surrounding context (e.g. a `let` annotation);
    /// an unresolved one is reported by the existing residual-variable check.
    fn emit_assoc_call(
        &mut self,
        fcx: &mut FnCtx,
        def_id: DefId,
        args: Vec<hir::Expr>,
        span: Span,
    ) -> hir::Expr {
        let Some(sig) = self.sigs.get(&def_id).cloned() else {
            return error_expr(span);
        };
        if args.len() != sig.params.len() {
            let fname = self.defs.def(def_id).name.clone();
            self.error(
                "E0016",
                format!(
                    "`{fname}` takes {} argument(s) but {} were supplied",
                    sig.params.len(),
                    args.len()
                ),
                span,
            );
            return error_expr(span);
        }
        let type_args: Vec<Ty> = (0..sig.generics).map(|_| fcx.icx.fresh()).collect();
        for (arg, param) in args.iter().zip(sig.params.iter()) {
            let expected = param.subst(&type_args);
            if !fcx.icx.unify(&arg.ty, &expected) {
                self.error(
                    "E0010",
                    format!(
                        "argument has type `{}` but `{}` was expected",
                        self.show(&arg.ty, fcx),
                        self.show(&expected, fcx),
                    ),
                    arg.span,
                );
            }
        }
        let ret = sig.ret.subst(&type_args);
        hir::Expr {
            kind: hir::ExprKind::Call {
                func: hir::Callee::Def(def_id),
                type_args,
                args,
            },
            ty: ret,
            span,
        }
    }
```

`TyHead::Named` is the assumed constructor for a nominal head. Check `TyHead`'s real definition in `crates/nova-hir/src/lib.rs` and use the correct variant; if heads are obtained via `Ty::head()`, build the type's head the same way `collect_impls` does when it computes `self_head`.

- [ ] **Step 5: Run the tests and watch them pass**

```bash
cargo test -p nova-typeck --lib -- associated_function unknown_associated_function
```

Expected: all four PASS.

- [ ] **Step 6: Confirm end-to-end under both backends**

```bash
cargo build -p nova-cli
printf 'record P { v: Int }\nimpl P { fn new() -> P { P { v: 7 } } }\nfn main() { let p = P::new()\n println("${p.v}") }\n' > /d/tmp/assoc.nova
./target/debug/nova.exe run /d/tmp/assoc.nova
./target/debug/nova.exe build /d/tmp/assoc.nova -o /d/tmp/assoc.exe && /d/tmp/assoc.exe
```

Expected: `7` from both.

- [ ] **Step 7: Verify and commit**

```bash
cargo test --workspace && cargo clippy --all-targets --all-features -- -D warnings && cargo fmt --check
git add -A && git commit -m "feat(typeck): associated function calls (\`Type::f(…)\`)

A two-segment path callee resolved only as a qualified variant, so \`P::new()\`
reported \"no variant 'new' on type 'P'\". It now falls through to a self-less
inherent method on the named type. Impl generics become fresh inference
variables, recovered from the call's context rather than a receiver.

Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>"
```

---

### Task 5: Trait associated functions and `T::default()` through a bound

**Why:** `Default` is only meaningful if `T::default()` works inside `fn f<T: Default>()`. **This is the one task the spec flags as carrying real risk.** If it resists after a genuine attempt, stop and take the documented fallback: keep `Default` for concrete types (Task 4 already delivers `Int::default()`), move generic `T::default()` to Phase 2.2, and record the deferral in the spec and CHANGELOG. Do not let this task block Tasks 6–10.

**Files:**
- Modify: `crates/nova-hir/src/lib.rs` — `TraitMethod` gains `has_self: bool`
- Modify: `crates/nova-typeck/src/check.rs` — `collect_traits` records `has_self`; `check_impl_conformance` compares it; `check_call` handles a generic-param qualifier; `emit_trait_assoc_call`
- Modify: `crates/nova-mir/src/lower.rs` — lower a receiver-less trait call
- Test: `crates/nova-typeck/src/check.rs` inline `mod tests`; `crates/nova-mir/tests/lower_tests.rs`

**Interfaces:**
- Consumes: `Checker.selfless` (Task 3), `emit_assoc_call` (Task 4).
- Produces: `hir::TraitMethod.has_self: bool`. `Ty::Param(k)` as a path qualifier resolves through `fcx.param_bounds[k]`.

- [ ] **Step 1: Write the failing tests**

```rust
    #[test]
    fn trait_associated_function_on_concrete_type() {
        let r = check_src(
            "trait Zero { fn zero() -> Self }\n\
             impl Zero for Int { fn zero() -> Int { 0 } }\n\
             fn main() { println(\"${Int::zero()}\") }",
        );
        assert!(r.diagnostics.is_empty(), "{:?}", r.diagnostics);
    }

    #[test]
    fn trait_associated_function_through_bound() {
        let r = check_src(
            "trait Zero { fn zero() -> Self }\n\
             impl Zero for Int { fn zero() -> Int { 0 } }\n\
             fn make<T: Zero>() -> T { T::zero() }\n\
             fn main() { let n: Int = make()\n println(\"${n}\") }",
        );
        assert!(r.diagnostics.is_empty(), "{:?}", r.diagnostics);
    }

    #[test]
    fn impl_self_receiver_disagreeing_with_trait_reports_e0072() {
        // The trait declares an associated function; the impl gives it a
        // receiver. That is a signature mismatch, not a silent difference.
        let r = check_src(
            "trait Zero { fn zero() -> Self }\n\
             record R { v: Int }\n\
             impl Zero for R { fn zero(self) -> R { self } }\n\
             fn main() { }",
        );
        assert!(error_codes(&r).contains(&"E0072"), "{:?}", r.diagnostics);
    }
```

- [ ] **Step 2: Run the tests and watch them fail**

```bash
cargo test -p nova-typeck --lib -- trait_associated_function impl_self_receiver_disagreeing
```

Expected: all three FAIL. Record the exact messages — they show whether `collect_traits` currently mis-signs receiver-less trait methods.

- [ ] **Step 3: Record `has_self` on trait methods**

`crates/nova-hir/src/lib.rs`, in `struct TraitMethod` (which documents "`self` is the implicit receiver and is not in `params`"):

```rust
    /// Whether the method declares a `self` receiver. A method without one is
    /// an associated function, called as `Type::name(…)` with no receiver.
    pub has_self: bool,
```

In `collect_traits`' table loop, the destructuring already yields `params`; add `has_self` alongside `is_async` in both `TraitItem::Required` and `TraitItem::Provided` arms:

```rust
                        sig.params.first().is_some_and(|p| p.name.value == "self"),
```

and set it when pushing `hir::TraitMethod { … has_self, … }`. For the default-body signature loop, prepend `Ty::Param(0)` **only** when `has_self` — mirroring Task 3's conditional in `collect_impls`.

- [ ] **Step 4: Compare `has_self` in conformance**

In `check_impl_conformance`, beside the generic-arity and bound checks added earlier, reject a receiver disagreement:

```rust
            let impl_has_self = !self.selfless.contains(def_id);
            if impl_has_self != trait_method.has_self {
                let (want, got) = if trait_method.has_self {
                    ("a `self` receiver", "none")
                } else {
                    ("no `self` receiver", "one")
                };
                self.error(
                    "E0072",
                    format!(
                        "method `{name}` has {got} but trait `{}` declares {want}",
                        tr.name
                    ),
                    span,
                );
                continue;
            }
```

- [ ] **Step 5: Resolve a generic-param qualifier at the call site**

Extend Task 4's two-segment branch in `check_call`. Before `resolve_type`, check whether the first segment names a generic parameter in scope; if so, look through its bounds for a trait declaring a receiver-less method of that name:

```rust
                // `T::zero()` where `T` is a generic parameter: dispatch through
                // its bounds, exactly as a bounded instance method does.
                if let Some(&k) = fcx.generics.get(ty_name) {
                    let matches: Vec<(DefId, u32)> = fcx
                        .param_bounds
                        .get(k as usize)
                        .into_iter()
                        .flatten()
                        .filter_map(|&tid| {
                            self.trait_method_index(tid, fn_name).map(|i| (tid, i))
                        })
                        .collect();
                    if let [(tid, idx)] = matches.as_slice() {
                        let checked: Vec<hir::Expr> =
                            args.iter().map(|a| self.check_expr(fcx, a)).collect();
                        return self.emit_trait_assoc_call(
                            fcx, *tid, *idx, Ty::Param(k), checked, span,
                        );
                    }
                }
```

For a concrete qualifier (`Int::zero()`), add a trait-impl fallback after `find_assoc_fn` fails: find an impl of a trait for that type whose trait method has `has_self == false`, then call `emit_trait_assoc_call` with the concrete self type.

- [ ] **Step 6: Add `emit_trait_assoc_call`**

Model it on `emit_trait_call`, minus the receiver. `self_ty` is the qualifier (concrete type or `Ty::Param(k)`), and `subst` is `[self_ty] ++ method type args`, matching the flat layout `emit_trait_call` established:

```rust
    /// Emit a call to a trait associated function (no receiver). `self_ty` comes
    /// from the path qualifier — a concrete type, or `Param(k)` when dispatching
    /// through a generic parameter's bound.
    fn emit_trait_assoc_call(
        &mut self,
        fcx: &mut FnCtx,
        trait_id: DefId,
        method_idx: u32,
        self_ty: Ty,
        args: Vec<hir::Expr>,
        span: Span,
    ) -> hir::Expr {
        let tm = self.traits[self
            .traits
            .iter()
            .position(|t| t.def_id == trait_id)
            .expect("trait exists")]
        .methods[method_idx as usize]
            .clone();
        let type_args: Vec<Ty> = (0..tm.generics).map(|_| fcx.icx.fresh()).collect();
        let mut subst = Vec::with_capacity(1 + type_args.len());
        subst.push(self_ty.clone());
        subst.extend(type_args.iter().cloned());
        if args.len() != tm.params.len() {
            self.error(
                "E0016",
                format!(
                    "`{}` takes {} argument(s) but {} were supplied",
                    tm.name,
                    tm.params.len(),
                    args.len()
                ),
                span,
            );
            return error_expr(span);
        }
        for (arg, param) in args.iter().zip(tm.params.iter()) {
            let expected = param.subst(&subst);
            if !fcx.icx.unify(&arg.ty, &expected) {
                self.error(
                    "E0010",
                    format!(
                        "argument has type `{}` but `{}` was expected",
                        self.show(&arg.ty, fcx),
                        self.show(&expected, fcx),
                    ),
                    arg.span,
                );
            }
        }
        hir::Expr {
            kind: hir::ExprKind::TraitCall {
                trait_id,
                method: method_idx,
                self_ty,
                type_args,
                receiver: Box::new(unit_expr(span)),
                args,
            },
            ty: tm.ret.subst(&subst),
            span,
        }
    }
```

`ExprKind::TraitCall` has a non-optional `receiver`. Prefer the smallest change that keeps MIR honest: either add `has_self: bool` to the `TraitCall` variant and skip the receiver when lowering in `crates/nova-mir/src/lower.rs`'s `lower_trait_call`, or make `receiver` an `Option<Box<hir::Expr>>`. **Pick one and apply it consistently** across `check.rs`, `child_exprs`, `child_exprs_mut`, `finalize_expr`, `mono.rs`'s `subst_expr`, and `lower.rs`. If you invent a `unit_expr` helper instead, define it beside `error_expr` in `check.rs`.

- [ ] **Step 7: Add the MIR test**

In `crates/nova-mir/tests/lower_tests.rs`, following the existing `generic_trait_method_monomorphizes_per_instance` pattern, assert that `make<T: Zero>() -> T` called at `T = Int` monomorphizes to the `Int` impl's `zero` and that the lowered call passes **no** receiver argument.

- [ ] **Step 8: Run the tests and watch them pass**

```bash
cargo test -p nova-typeck --lib -- trait_associated_function impl_self_receiver_disagreeing
cargo test -p nova-mir
```

Expected: PASS. **If Step 5/6 cannot be completed cleanly, revert this task's changes, take the documented fallback, and continue with Task 6.**

- [ ] **Step 9: Verify and commit**

```bash
cargo test --workspace && cargo clippy --all-targets --all-features -- -D warnings && cargo fmt --check
git add -A && git commit -m "feat(typeck): trait associated functions and dispatch through bounds

A trait method may declare no \`self\` receiver, making it an associated
function called as \`Type::name(…)\` — including \`T::name(…)\` inside a
generic function, dispatched through T's bound the way bounded instance
methods already are. Conformance rejects an impl that disagrees with the
trait about the receiver (E0072).

Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>"
```

---

### Task 6: Supertrait enforcement (`trait Ord: Eq`)

**Why:** `trait B: A` parses today (`crates/nova-parser/src/grammar.rs:517-519` fills `TraitDecl.supertraits`) and is then **silently discarded** — no reader exists in `nova-resolver` or `nova-typeck`. Writing it is a lie.

**Files:**
- Modify: `crates/nova-hir/src/lib.rs` — `TraitDef` gains `supertraits: Vec<DefId>`
- Modify: `crates/nova-typeck/src/check.rs` — `collect_traits` resolves them; `collect_impls`/`check_impl_conformance` require them; bound checking treats them as implied
- Modify: `crates/nova-mir/src/mono.rs` — `impl_satisfies` accepts a supertrait bound satisfied via a subtrait impl
- Test: `crates/nova-typeck/src/check.rs` inline `mod tests`

**Interfaces:**
- Produces: `hir::TraitDef.supertraits: Vec<DefId>`.

- [ ] **Step 1: Write the failing tests**

```rust
    #[test]
    fn impl_of_subtrait_without_supertrait_reports_e0072() {
        let r = check_src(
            "trait A { fn a(self) -> Int }\n\
             trait B: A { fn b(self) -> Int }\n\
             record R { v: Int }\n\
             impl B for R { fn b(self) -> Int { 2 } }\n\
             fn main() { }",
        );
        assert!(error_codes(&r).contains(&"E0072"), "{:?}", r.diagnostics);
    }

    #[test]
    fn impl_of_subtrait_with_supertrait_typechecks() {
        let r = check_src(
            "trait A { fn a(self) -> Int }\n\
             trait B: A { fn b(self) -> Int }\n\
             record R { v: Int }\n\
             impl A for R { fn a(self) -> Int { 1 } }\n\
             impl B for R { fn b(self) -> Int { 2 } }\n\
             fn main() { }",
        );
        assert!(r.diagnostics.is_empty(), "{:?}", r.diagnostics);
    }

    #[test]
    fn supertrait_method_callable_through_subtrait_bound() {
        // `T: B` implies `T: A`, so `a()` is callable.
        let r = check_src(
            "trait A { fn a(self) -> Int }\n\
             trait B: A { fn b(self) -> Int }\n\
             record R { v: Int }\n\
             impl A for R { fn a(self) -> Int { 1 } }\n\
             impl B for R { fn b(self) -> Int { 2 } }\n\
             fn sum<T: B>(x: T) -> Int { x.a() + x.b() }\n\
             fn main() { println(\"${sum(R { v: 0 })}\") }",
        );
        assert!(r.diagnostics.is_empty(), "{:?}", r.diagnostics);
    }
```

- [ ] **Step 2: Run the tests and watch them fail**

```bash
cargo test -p nova-typeck --lib -- impl_of_subtrait supertrait_method_callable
```

Expected: `impl_of_subtrait_without_supertrait_reports_e0072` FAILS with `[]` (supertraits ignored, so the missing `impl A` goes unnoticed). `supertrait_method_callable_through_subtrait_bound` FAILS with `E0014`/`E0015` (the bound `T: B` does not imply `A`, so `x.a()` does not resolve). `impl_of_subtrait_with_supertrait_typechecks` should already PASS.

- [ ] **Step 3: Resolve supertraits into the trait table**

Add `pub supertraits: Vec<DefId>` to `hir::TraitDef`. In `collect_traits`, resolve each `Spanned<Path>` in `decl.supertraits` with `self.defs.resolve_trait(self.cur_module, name)`, reporting `E0001` (`cannot find trait '<name>'`) for an unresolved one, and deduplicate — mirroring how `resolve_bounds` dedupes, since a repeated trait id would read as two providers and cause a false `E0015` ambiguity.

- [ ] **Step 4: Require supertrait impls**

In `check_impl_conformance` (which already receives `trait_id` and `self_ty`), after the method checks, require an impl of each supertrait for the same self type. Reuse the existing impl lookup that `check_impl_coherence`/`resolve_method_on` use — find an impl whose `trait_id` is the supertrait and whose `self_ty` matches structurally:

```rust
        for &super_id in &tr.supertraits {
            let satisfied = self
                .impls
                .iter()
                .any(|i| i.trait_id == Some(super_id) && i.match_args(self_ty).is_some());
            if !satisfied {
                let sname = self
                    .traits
                    .iter()
                    .find(|t| t.def_id == super_id)
                    .map(|t| t.name.clone())
                    .unwrap_or_default();
                self.error(
                    "E0072",
                    format!(
                        "the trait `{}` requires `{}`, which `{}` does not implement",
                        tr.name,
                        sname,
                        display_ty(self_ty, self.defs),
                    ),
                    span,
                );
            }
        }
```

Note `check_impl_conformance` runs during `collect_impls`, so `self.impls` may not yet hold later impls. If the guard test `impl_of_subtrait_with_supertrait_typechecks` starts failing because of declaration order, move this loop into a separate pass that runs after all impls are collected — beside `check_impl_coherence`, which already works that way — and give it the impl's span from the same `impl_spans` vector.

- [ ] **Step 5: Make subtrait bounds imply supertrait bounds**

Add a helper that expands a bound list with all transitive supertraits, and use it where per-parameter bounds are built (`resolve_bounds`' callers) so `T: B` yields `[B, A]`:

```rust
    /// Expand a bound list with the transitive supertraits of each trait, so a
    /// bound `T: B` also provides `B`'s supertrait `A`. Deduplicated, because a
    /// repeated trait id would read as two method providers (a false E0015).
    fn with_supertraits(&self, bounds: &[DefId]) -> Vec<DefId> {
        let mut out: Vec<DefId> = Vec::new();
        let mut stack: Vec<DefId> = bounds.to_vec();
        while let Some(id) = stack.pop() {
            if out.contains(&id) {
                continue;
            }
            out.push(id);
            if let Some(t) = self.traits.iter().find(|t| t.def_id == id) {
                stack.extend(t.supertraits.iter().copied());
            }
        }
        out
    }
```

The `while let`/`contains` loop terminates even on a cyclic `trait A: B` / `trait B: A`, because each id is pushed to `out` at most once.

Apply it when building `FnSig.bounds` for functions, impls, and methods — the same places `resolve_bounds` results land. `Ty::Param(k)` method resolution then finds supertrait methods with no change to `resolve_method_on`.

- [ ] **Step 6: Accept supertrait bounds at monomorphization**

`crates/nova-mir/src/mono.rs`'s `impl_satisfies` checks whether a concrete type has an impl for a required trait. A `T: A` requirement must be satisfiable when only `impl B for R` is written and `B: A`… which Step 4 makes impossible (an `impl B` now requires an `impl A`). So no change should be needed. Confirm by running `cargo test -p nova-mir`; if a bound-check failure appears, expand the required-trait set there with `TraitDef.supertraits` the same way Step 5 does.

- [ ] **Step 7: Run the tests and watch them pass**

```bash
cargo test -p nova-typeck --lib -- impl_of_subtrait supertrait_method_callable
```

Expected: all three PASS.

- [ ] **Step 8: Verify and commit**

```bash
cargo test --workspace && cargo clippy --all-targets --all-features -- -D warnings && cargo fmt --check
git add -A && git commit -m "feat(typeck): enforce supertraits

\`trait B: A\` parsed and was then discarded, so the declaration meant nothing.
An \`impl B for T\` now requires \`impl A for T\` (E0072), and a bound \`T: B\`
implies \`T: A\`, making supertrait methods callable through a subtrait bound.

Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>"
```

---

### Task 7: `std/core/lib.nova` scaffold, the `FileId` seam, and ADR 0004

**Why:** Moves `Option`/`Result` out of the two-line Rust `PRELUDE_SRC` into real Nova source on disk, and gives that source a **real `FileId`** so an error inside `std/core` produces a diagnostic pointing at the actual file instead of `FileId::DUMMY`. This seam is what later becomes a disk search path.

**Files:**
- Create: `std/core/lib.nova`
- Create: `docs/adr/0004-stdlib-compile-model.md`
- Modify: `crates/nova-resolver/src/lib.rs:391-411` (`PRELUDE_SRC` → `STD_CORE_SRC` via `include_str!`; `prelude_file` takes a `FileId`; `resolve_program` accepts it)
- Modify: `crates/nova-driver/src/lib.rs` (register the source in the `FileDb`, pass the `FileId`)
- Test: `crates/nova-resolver/src/lib.rs` or `crates/nova-typeck/src/check.rs` inline tests

**Interfaces:**
- Produces: `pub const STD_CORE_SRC: &str` and `pub const STD_CORE_NAME: &str = "$std.core"` in `nova-resolver`. `resolve_program(&[ModuleSource], std_core_file: FileId)` — the driver supplies the `FileId`; `resolve(&File)` (the single-module test wrapper) passes `FileId::DUMMY`.

- [ ] **Step 1: Write the failing test**

Add to `crates/nova-typeck/src/check.rs`'s inline `mod tests`:

```rust
    #[test]
    fn std_core_parses_and_typechecks_clean() {
        // The implicit std/core module must itself be error-free; a program that
        // uses nothing from it must produce no diagnostics.
        let r = check_src("fn main() { println(\"hi\") }");
        assert!(r.diagnostics.is_empty(), "{:?}", r.diagnostics);
    }

    #[test]
    fn std_core_option_is_available_without_import() {
        let r = check_src(
            "fn main() {\n\
                 let o = Some(3)\n\
                 match o { Some(v) => println(\"${v}\"), None => println(\"none\") }\n\
             }",
        );
        assert!(r.diagnostics.is_empty(), "{:?}", r.diagnostics);
    }
```

Both pass today (`PRELUDE_SRC` provides `Option`). They are the regression net for this task: they must **still** pass after the source moves to disk. Run them now and record that they pass.

```bash
cargo test -p nova-typeck --lib -- std_core_parses std_core_option_is_available
```

- [ ] **Step 2: Create `std/core/lib.nova`**

Exactly the current prelude contents, so this step is a pure move with no behavior change:

```nova
// Nova standard library — core.
//
// Compiled as an implicit module and glob-imported into every user module, so
// these names need no `import`. A user definition of the same name shadows the
// one here (see docs/adr/0004-stdlib-compile-model.md).

pub type Option<T> = | Some(T) | None

pub type Result<T, E> = | Ok(T) | Err(E)
```

- [ ] **Step 3: Point the resolver at it**

In `crates/nova-resolver/src/lib.rs`, replace the `PRELUDE_SRC` constant:

```rust
/// The `std/core` source, compiled as an implicit module. Embedded at build time
/// so the compiler is self-contained; the path is relative to this file.
pub const STD_CORE_SRC: &str = include_str!("../../../std/core/lib.nova");

/// Module name of the implicit `std/core`. Not a valid identifier, so it can
/// never collide with a user module or be named in an `import`.
const STD_CORE_NAME: &str = "$std.core";
```

Update `prelude_file` to take a `FileId` and to report failures against it rather than only `debug_assert`-ing. Because `std/core` is now substantial, a syntax error in it must be visible instead of silently yielding an empty module:

```rust
/// Lex and parse the implicit `std/core` module. Its source ships with the
/// compiler, so any failure is a compiler bug — but it is reported against the
/// real file so it is debuggable rather than silently dropped.
fn std_core_file(file_id: FileId) -> (File, Vec<Diagnostic>) {
    let (tokens, lex_errors) = nova_lexer::lex(STD_CORE_SRC, file_id);
    let mut diags: Vec<Diagnostic> = lex_errors
        .iter()
        .map(|e| Diagnostic::error("L0001", e.to_string()).with_primary_label(e.span(), "here"))
        .collect();
    let (ast, parse_errors) = nova_parser::parse(&tokens, file_id);
    diags.extend(parse_errors.iter().map(|e| {
        Diagnostic::error("P0001", e.to_string()).with_primary_label(e.span(), "here")
    }));
    (ast.unwrap_or_default(), diags)
}
```

If `nova_ast::File` has no `Default`, return `Option<File>` and have `resolve_program` push the diagnostics and skip the implicit module. Thread the `FileId` through `resolve_program`, keep every existing rename consistent (`PRELUDE_NAME` → `STD_CORE_NAME`, `prelude_mid`, `import_prelude`), and have the single-module `resolve(&File)` wrapper pass `FileId::DUMMY`.

- [ ] **Step 4: Register the source in the driver**

In `crates/nova-driver/src/lib.rs`, before calling `resolve_program`, register the embedded source so its `FileId` resolves to a real named file in diagnostics:

```rust
        let std_core_file = self
            .db
            .add("<std/core>".to_string(), nova_resolver::STD_CORE_SRC);
```

Pass `std_core_file` to `resolve_program`. Match `FileDb::add`'s actual signature (the existing call is `self.db.add(path.display().to_string(), source.as_str())`).

- [ ] **Step 5: Run the tests and watch them still pass**

```bash
cargo test -p nova-typeck --lib -- std_core_parses std_core_option_is_available
cargo test --workspace
```

Expected: PASS — a pure move. If `Option` becomes unresolvable, the `include_str!` path or the module-name rename is wrong.

- [ ] **Step 6: Write ADR 0004**

Create `docs/adr/0004-stdlib-compile-model.md` following the format of `docs/adr/0003-*`. Record: the decision (implicit prelude compiled from on-disk Nova source, embedded with `include_str!`); the alternatives rejected (disk search path — needs nested import paths and adds deployment failure modes; precompiled artifact — generics cannot be precompiled, and it needs serialized HIR plus incremental infrastructure); the consequences (`std/core` names are visible everywhere with user definitions shadowing; a user redefining a method `std/core` defines on `Option` gets `E0074`; the compiler binary carries `std/core`); and the migration path (the driver-supplied `FileId` is the single seam; swapping the embed for a disk read leaves the Nova source untouched).

- [ ] **Step 7: Verify and commit**

```bash
cargo test --workspace && cargo clippy --all-targets --all-features -- -D warnings && cargo fmt --check
git add -A && git commit -m "refactor(resolver): move the prelude into std/core/lib.nova (ADR 0004)

Option and Result move from a two-line Rust string constant into real Nova
source at std/core/lib.nova, embedded with include_str! and compiled as the
implicit module exactly as before — a pure move, no behavior change.

The driver now registers that source in its FileDb and passes the FileId in, so
an error inside std/core points at a real file instead of FileId::DUMMY. That
seam is also the one a future disk search path replaces, leaving the Nova
source untouched.

Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>"
```

---

### Task 8: `Option` and `Result` method sets

**Why:** The heart of `std/core`. Verified already supported: inherent impls on the prelude's own sum types, generic methods taking closures (`map<U>`), and `-> Self`.

**Files:**
- Modify: `std/core/lib.nova`
- Test: `crates/nova-typeck/src/check.rs` inline `mod tests`

**Interfaces:**
- Consumes: `panic` (Task 1).
- Produces: `Option<T>`: `is_some`, `is_none`, `map<U>`, `and_then<U>`, `unwrap`, `unwrap_or`, `ok_or<E>`. `Result<T, E>`: `is_ok`, `is_err`, `map<U>`, `map_err<F>`, `and_then<U>`, `unwrap`, `unwrap_or`. Task 10's e2e fixture calls these.

- [ ] **Step 1: Write the failing test**

```rust
    #[test]
    fn std_core_option_result_methods_typecheck() {
        let r = check_src(
            "fn dbl(n: Int) -> Int { n * 2 }\n\
             fn main() {\n\
                 let a = Some(21).map(dbl).unwrap_or(0)\n\
                 let b = Some(1).is_some()\n\
                 let c: Result<Int, String> = Some(2).ok_or(\"none\")\n\
                 let d = c.map(dbl).unwrap_or(0)\n\
                 println(\"${a} ${b} ${d}\")\n\
             }",
        );
        assert!(r.diagnostics.is_empty(), "{:?}", r.diagnostics);
    }
```

- [ ] **Step 2: Run it and watch it fail**

```bash
cargo test -p nova-typeck --lib -- std_core_option_result_methods
```

Expected: FAIL with `E0014: no method 'map' on type 'Option<Int>'`.

- [ ] **Step 3: Add the impls to `std/core/lib.nova`**

```nova
impl<T> Option<T> {
    pub fn is_some(self) -> Bool {
        match self { Some(_) => true, None => false }
    }

    pub fn is_none(self) -> Bool { !self.is_some() }

    pub fn map<U>(self, f: fn(T) -> U) -> Option<U> {
        match self { Some(v) => Some(f(v)), None => None }
    }

    pub fn and_then<U>(self, f: fn(T) -> Option<U>) -> Option<U> {
        match self { Some(v) => f(v), None => None }
    }

    pub fn unwrap(self) -> T {
        match self { Some(v) => v, None => panic("called `unwrap` on a `None` value") }
    }

    pub fn unwrap_or(self, default: T) -> T {
        match self { Some(v) => v, None => default }
    }

    pub fn ok_or<E>(self, err: E) -> Result<T, E> {
        match self { Some(v) => Ok(v), None => Err(err) }
    }
}

impl<T, E> Result<T, E> {
    pub fn is_ok(self) -> Bool {
        match self { Ok(_) => true, Err(_) => false }
    }

    pub fn is_err(self) -> Bool { !self.is_ok() }

    pub fn map<U>(self, f: fn(T) -> U) -> Result<U, E> {
        match self { Ok(v) => Ok(f(v)), Err(e) => Err(e) }
    }

    pub fn map_err<F>(self, f: fn(E) -> F) -> Result<T, F> {
        match self { Ok(v) => Ok(v), Err(e) => Err(f(e)) }
    }

    pub fn and_then<U>(self, f: fn(T) -> Result<U, E>) -> Result<U, E> {
        match self { Ok(v) => f(v), Err(e) => Err(e) }
    }

    pub fn unwrap(self) -> T {
        match self { Ok(v) => v, Err(_) => panic("called `unwrap` on an `Err` value") }
    }

    pub fn unwrap_or(self, default: T) -> T {
        match self { Ok(v) => v, Err(_) => default }
    }
}
```

- [ ] **Step 4: Run it and watch it pass**

```bash
cargo test -p nova-typeck --lib -- std_core_option_result_methods
```

Expected: PASS. If `unwrap`'s `panic` arm errors, Task 1 is incomplete — `panic` must type as `Ty::Never`. If `map_err`'s `Err(e) => Err(e)` errors, the two `Err`s hold different type parameters (`E` vs `F`); check the inference error text before changing the Nova source.

- [ ] **Step 5: Confirm it runs**

```bash
cargo build -p nova-cli
printf 'fn dbl(n: Int) -> Int { n * 2 }\nfn main() {\n  println("${Some(21).map(dbl).unwrap()}")\n  let r: Result<Int, String> = Ok(4)\n  println("${r.map(dbl).unwrap_or(0)}")\n}\n' > /d/tmp/oc.nova
./target/debug/nova.exe run /d/tmp/oc.nova
```

Expected: `42` then `8`.

- [ ] **Step 6: Verify and commit**

```bash
cargo test --workspace && cargo clippy --all-targets --all-features -- -D warnings && cargo fmt --check
git add -A && git commit -m "feat(std): Option and Result method sets in std/core

is_some/is_none/map/and_then/unwrap/unwrap_or/ok_or on Option, and
is_ok/is_err/map/map_err/and_then/unwrap/unwrap_or on Result — written in Nova,
using the generic methods and closures Phase 2.0 landed. unwrap diverges
through the new panic builtin.

Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>"
```

---

### Task 9: Core traits and primitive impls

**Why:** Completes `std/core`. `Display` makes the existing interpolation convention official: `check_interp` already bridges to a `fmt(self) -> String` method by name. Primitive interpolation is unaffected — `check.rs:1738` matches `Ty::Int | Ty::Float | Ty::Bool | Ty::Char` natively **before** reaching that bridge.

**Files:**
- Modify: `std/core/lib.nova`
- Test: `crates/nova-typeck/src/check.rs` inline `mod tests`

**Interfaces:**
- Consumes: `nova_rt_str_cmp` (Task 2), supertraits (Task 6), associated functions (Tasks 4–5).
- Produces: traits `Display`, `Debug`, `Eq`, `Ord: Eq`, `Clone`, `Default`; `Ordering`; impls for `Int`, `Float`, `Bool`, `Char`, `String`.

- [ ] **Step 1: Write the failing test**

```rust
    #[test]
    fn std_core_traits_and_primitive_impls_typecheck() {
        let r = check_src(
            "fn show_all<T: Display>(x: T) -> String { x.fmt() }\n\
             fn main() {\n\
                 println(show_all(3))\n\
                 println(\"${(1).eq(1)}\")\n\
                 let o = (\"a\").cmp(\"b\")\n\
                 match o { Less => println(\"less\"), Equal => println(\"eq\"), \
                           Greater => println(\"gt\") }\n\
             }",
        );
        assert!(r.diagnostics.is_empty(), "{:?}", r.diagnostics);
    }
```

- [ ] **Step 2: Run it and watch it fail**

```bash
cargo test -p nova-typeck --lib -- std_core_traits_and_primitive_impls
```

Expected: FAIL — `cannot find trait 'Display'`.

- [ ] **Step 3: Add the traits**

Append to `std/core/lib.nova`:

```nova
pub type Ordering = | Less | Equal | Greater

pub trait Display { fn fmt(self) -> String }

pub trait Debug { fn dbg(self) -> String }

pub trait Eq {
    fn eq(self, other: Self) -> Bool
    fn ne(self, other: Self) -> Bool { !self.eq(other) }
}

pub trait Ord: Eq { fn cmp(self, other: Self) -> Ordering }

pub trait Clone { fn clone(self) -> Self }

pub trait Default { fn default() -> Self }
```

- [ ] **Step 4: Add the primitive impls**

`Ord` needs care — verified: `<` works for `Int`/`Float`/`Char`, but **not** `Bool` or `String`. `Bool` is written with `if` alone; `String` uses the runtime compare.

```nova
extern "C" { fn nova_rt_str_cmp(a: String, b: String) -> Int }

impl Display for Int    { fn fmt(self) -> String { "${self}" } }
impl Display for Float  { fn fmt(self) -> String { "${self}" } }
impl Display for Bool   { fn fmt(self) -> String { "${self}" } }
impl Display for Char   { fn fmt(self) -> String { "${self}" } }
impl Display for String { fn fmt(self) -> String { self } }

impl Debug for Int    { fn dbg(self) -> String { "${self}" } }
impl Debug for Float  { fn dbg(self) -> String { "${self}" } }
impl Debug for Bool   { fn dbg(self) -> String { "${self}" } }
impl Debug for Char   { fn dbg(self) -> String { "${self}" } }
impl Debug for String { fn dbg(self) -> String { "\"${self}\"" } }

impl Eq for Int    { fn eq(self, other: Int) -> Bool { self == other } }
impl Eq for Float  { fn eq(self, other: Float) -> Bool { self == other } }
impl Eq for Bool   { fn eq(self, other: Bool) -> Bool { self == other } }
impl Eq for Char   { fn eq(self, other: Char) -> Bool { self == other } }
impl Eq for String { fn eq(self, other: String) -> Bool { self == other } }

impl Ord for Int {
    fn cmp(self, other: Int) -> Ordering {
        if self < other { Less } else { if self == other { Equal } else { Greater } }
    }
}
impl Ord for Float {
    fn cmp(self, other: Float) -> Ordering {
        if self < other { Less } else { if self == other { Equal } else { Greater } }
    }
}
impl Ord for Char {
    fn cmp(self, other: Char) -> Ordering {
        if self < other { Less } else { if self == other { Equal } else { Greater } }
    }
}
impl Ord for Bool {
    // `<` is not defined for Bool; false sorts before true.
    fn cmp(self, other: Bool) -> Ordering {
        if self {
            if other { Equal } else { Greater }
        } else {
            if other { Less } else { Equal }
        }
    }
}
impl Ord for String {
    // String has neither length nor indexing in Nova, so ordering comes from
    // the runtime's byte-lexicographic compare.
    fn cmp(self, other: String) -> Ordering {
        let c = nova_rt_str_cmp(self, other)
        if c < 0 { Less } else { if c == 0 { Equal } else { Greater } }
    }
}

impl Clone for Int    { fn clone(self) -> Int { self } }
impl Clone for Float  { fn clone(self) -> Float { self } }
impl Clone for Bool   { fn clone(self) -> Bool { self } }
impl Clone for Char   { fn clone(self) -> Char { self } }
impl Clone for String { fn clone(self) -> String { self } }

impl Default for Int    { fn default() -> Int { 0 } }
impl Default for Float  { fn default() -> Float { 0.0 } }
impl Default for Bool   { fn default() -> Bool { false } }
impl Default for String { fn default() -> String { "" } }
```

Two things to check against reality while implementing:

1. **`extern` FFI type rules.** `require_ffi_safe` in `crates/nova-typeck/src/check.rs` may reject `String` as an extern parameter (it is a GC pointer, not a scalar). If it does, do **not** loosen the FFI check — instead expose the compare the way other string operations reach the runtime, i.e. add it as an internal operation rather than a user-visible `extern`. Read how `StrEq` is emitted for `==` and follow that path.
2. **`Ord for Bool`/`Char` and `Default for Char`.** If `Default for Char` needs a character literal Nova cannot express, omit `Default for Char` — the spec does not require an exhaustive matrix.

- [ ] **Step 5: Run it and watch it pass**

```bash
cargo test -p nova-typeck --lib -- std_core_traits_and_primitive_impls
```

Expected: PASS.

- [ ] **Step 6: Confirm interpolation did not regress**

```bash
cargo build -p nova-cli
printf 'fn main() {\n  let n = 7\n  println("n=${n} f=${1.5} b=${true} c=${*a*} s=${"x"}")\n}\n' | tr '*' "'" > /d/tmp/interp.nova
./target/debug/nova.exe run /d/tmp/interp.nova
```

Expected: `n=7 f=1.5 b=true c=a s=x` — the native path, unchanged by the new `Display for Int`.

- [ ] **Step 7: Verify and commit**

```bash
cargo test --workspace && cargo clippy --all-targets --all-features -- -D warnings && cargo fmt --check
git add -A && git commit -m "feat(std): core traits and primitive impls in std/core

Display, Debug, Eq, Ord: Eq, Clone, Default, and Ordering, with impls for Int,
Float, Bool, Char, and String. Display formalizes the interpolation
convention — a type with \`fmt(self) -> String\` interpolates — while primitives
keep the native fast path, which check_interp takes before the fmt bridge.

Ord accounts for Nova's uneven comparison support: Int/Float/Char use \`<\`,
Bool is ordered with \`if\`, and String uses the runtime byte-lexicographic
compare.

Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>"
```

---

### Task 10: End-to-end gate, GC stress, CHANGELOG

**Why:** The plan's gate: a program round-trips `Option`/`Result` and a custom `Display` under **both** backends, plus `NOVA_GC_STRESS=1` per established convention.

**Files:**
- Create: `tests/runtime/std_core.nova`, `tests/runtime/std_core.stdout`
- Modify: `crates/nova-cli/tests/run_tests.rs`
- Modify: `CHANGELOG.md`

**Interfaces:**
- Consumes: everything from Tasks 1–9.

- [ ] **Step 1: Write the fixture**

`tests/runtime/std_core.nova`:

```nova
record Point { x: Int, y: Int }

impl Display for Point {
    fn fmt(self) -> String { "(${self.x}, ${self.y})" }
}

fn dbl(n: Int) -> Int { n * 2 }

fn describe<T: Display>(x: T) -> String { "<${x.fmt()}>" }

fn main() {
    // Option
    println("${Some(21).map(dbl).unwrap()}")
    println("${Some(1).is_some()} ${Some(1).is_none()}")
    let missing: Option<Int> = None
    println("${missing.unwrap_or(99)}")

    // Result
    let ok: Result<Int, String> = Ok(4)
    println("${ok.map(dbl).unwrap()}")
    let bad: Result<Int, String> = Err("nope")
    println("${bad.unwrap_or(-1)}")
    println("${bad.is_err()}")

    // Option -> Result
    let converted = Some(5).ok_or("none")
    println("${converted.unwrap()}")

    // Custom Display, direct and through a bound
    let p = Point { x: 1, y: 2 }
    println(p.fmt())
    println(describe(p))
    println("${p}")

    // Primitive traits
    println("${(3).eq(3)} ${(3).ne(4)}")
    match ("a").cmp("b") {
        Less => println("less"),
        Equal => println("equal"),
        Greater => println("greater"),
    }
    println("${Int::default()}")
}
```

`tests/runtime/std_core.stdout`:

```
42
true false
99
8
-1
true
5
(1, 2)
<(1, 2)>
(1, 2)
true true
less
0
```

- [ ] **Step 2: Run it and reconcile**

```bash
cargo build -p nova-cli
./target/debug/nova.exe run tests/runtime/std_core.nova
```

Compare against the fixture. If output differs, decide which is wrong: a genuine bug in Tasks 1–9 must be fixed, and only a mistaken expectation in the fixture may be edited. `Int::default()` requires Task 5; if Task 5 took its documented fallback, keep the line (a concrete qualifier is exactly what the fallback supports) — but if `describe(p)` fails, that is a real bound-dispatch bug worth fixing.

- [ ] **Step 3: Add the e2e tests**

In `crates/nova-cli/tests/run_tests.rs`, following the existing `generic_trait_methods_run` / `generic_trait_methods_build_standalone` pattern:

```rust
#[test]
fn std_core_run() {
    let expected = std::fs::read_to_string(repo_root().join("tests/runtime/std_core.stdout"))
        .expect("expected-output fixture exists")
        .replace("\r\n", "\n");
    nova()
        .arg("run")
        .arg(repo_root().join("tests/runtime/std_core.nova"))
        .assert()
        .success()
        .stdout(expected);
}

#[test]
fn std_core_build_standalone() {
    let expected = std::fs::read_to_string(repo_root().join("tests/runtime/std_core.stdout"))
        .expect("expected-output fixture exists")
        .replace("\r\n", "\n");
    let out = build_and_run("tests/runtime/std_core.nova", "std_core");
    assert_eq!(out.replace("\r\n", "\n"), expected);
}

#[test]
fn std_core_under_gc_stress() {
    let expected = std::fs::read_to_string(repo_root().join("tests/runtime/std_core.stdout"))
        .expect("expected-output fixture exists")
        .replace("\r\n", "\n");
    nova()
        .env("NOVA_GC_STRESS", "1")
        .arg("run")
        .arg(repo_root().join("tests/runtime/std_core.nova"))
        .assert()
        .success()
        .stdout(expected);
}
```

Check `build_and_run`'s real return type at `crates/nova-cli/tests/run_tests.rs:557` and adapt if it does not return `String`.

- [ ] **Step 4: Run the e2e tests**

```bash
cargo test -p nova-cli -- std_core
```

Expected: all three PASS.

- [ ] **Step 5: Test that unused `std/core` contributes no symbols**

Monomorphization must emit only reachable items, or every program would carry all
of `std/core`. Add to `crates/nova-mir/tests/lower_tests.rs`, following the
existing helpers there for lowering a source string to a MIR module:

```rust
#[test]
fn unused_std_core_emits_no_symbols() {
    // std/core is compiled into every program as the implicit prelude, but a
    // program that uses none of it must not carry any of its functions.
    let module = lower_src("fn main() { println(\"hi\") }");
    let names: Vec<&str> = module.functions.iter().map(|f| f.name.as_str()).collect();
    assert!(
        names.iter().all(|n| !n.contains("Option") && !n.contains("Result")),
        "unused std/core leaked into the module: {names:?}"
    );
}
```

Use whatever the file's existing lowering helper is called (read the top of
`lower_tests.rs`); if it is not `lower_src`, adapt the call and keep the
assertion. Run it:

```bash
cargo test -p nova-mir -- unused_std_core_emits_no_symbols
```

Expected: PASS. A failure means monomorphization is retaining unreachable
`std/core` items, which must be fixed before this task closes.

- [ ] **Step 6: Update the CHANGELOG**

In `CHANGELOG.md`'s `[Unreleased]` section, add entries for: the `panic` builtin; associated functions (`Type::f()`) and the self-less-method signature fix, naming the three symptoms; supertrait enforcement; `nova_rt_str_cmp`; and `std/core` with its module contents, the compile model (pointing at ADR 0004), and what is deferred (`std/fmt`, `std/io`, `Iterator`, `Hash`, `Copy`). Match the existing entry style — a bold-free bullet with a `(Phase 2.1)` marker and a short rationale.

- [ ] **Step 7: Final verification**

```bash
cargo test --workspace && cargo clippy --all-targets --all-features -- -D warnings && cargo fmt --check
```

Expected: all green.

- [ ] **Step 8: Commit**

```bash
git add -A && git commit -m "test(std): end-to-end gate for std/core

Round-trips Option and Result through their methods, exercises a custom Display
directly, through a \`T: Display\` bound, and via interpolation, and checks the
primitive Eq/Ord/Default impls — under nova run, nova build, and
NOVA_GC_STRESS=1.

Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>"
```

---

## After the plan

Run the adversarial-review workflow over the whole increment, per the established project loop, then fix confirmed findings. Do **not** push; the user pushes explicitly.
