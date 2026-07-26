# Phase 2.2a — Field Assignment, Repeat Arrays, and `std/collections` Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Ship `Vec`, `Hash`, `Map`, and `Set` as `std/collections`, written in Nova, after adding the two language features they require.

**Architecture:** Stage 1 (Tasks 1–3) adds compiler features in Rust: field assignment (`rec.f = v`), a mutable-receiver rule, and a repeat-array literal (`[init; n]`). Stage 2 (Tasks 4–9) generalizes the embedded-std seam to more than one module, then writes the collections in Nova. Growth is "allocate a bigger array, reassign the field" — the record's address never changes, so the conservative non-moving GC is untouched.

**Tech Stack:** Rust 2021 workspace (`nova-lexer`, `nova-parser`, `nova-ast`, `nova-resolver`, `nova-typeck`, `nova-hir`, `nova-mir`, `nova-codegen-cranelift`, `nova-codegen-llvm`, `nova-runtime`, `nova-driver`, `nova-cli`); Nova source for `std/`.

**Spec:** `docs/superpowers/specs/2026-07-26-phase-2-2a-collections-design.md`

## Global Constraints

- Repo root: `D:\Projects\nona\nova`. All `cargo` commands run there.
- Every task ends green on all three: `cargo test --workspace --no-fail-fast`, `cargo clippy --all-targets --all-features -- -D warnings`, `cargo fmt --check`. **`--no-fail-fast` is required** — without it cargo abandons later test targets on the first failure and under-reports.
- TDD is mandatory: write the failing test, **run it and see it fail**, then implement.
- Conventional commits, one logical change per commit, ending with:
  `Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>`
- No `unwrap()`/`expect()` in Rust library paths that can fail on user input. Slicing/indexing panics identically — prefer `.get(..)`. Tests may use them.
- Every new or changed diagnostic needs a test asserting its code.
- Do **not** `git push`. The user pushes explicitly.
- Nova primitives: `Int` is i64, `Float` f64, `Bool` i8, `Char` i64, `String` is a GC'd `NovaStr*`.
- Heap layouts: a record is one block with fields at `8*i`. An array is one block, `{ len: i64, elem0, elem1, … }`, elements at `8 + 8*i`, allocated with `nova_rt_alloc(8 + 8*n)`. `gc::alloc` returns **zeroed** memory.
- Diagnostic codes reused from the existing scheme: `E0010` type mismatch, `E0013` unsatisfied bound, `E0014` bad field/method access, `E0060` assignment to an immutable place, `E0900` unsupported.
- **`assert!(diagnostics.is_empty())` can be silently vacuous** — `Ty::Error` unifies with anything (`crates/nova-typeck/src/infer.rs`). Where a test's point is a value or type, assert the resolved HIR type or the runtime output.
- **Nova source gotcha (now fixed, but know it):** a record literal inside string interpolation used to fail; `${f(R { v: 1 })}` works as of `6e7a132`.

---

## File Structure

| File | Responsibility | Tasks |
|---|---|---|
| `crates/nova-ast/src/expr.rs` | `Expr::ArrayRepeat` variant | 3 |
| `crates/nova-parser/src/grammar.rs` | Parse `[init; n]` in the array-literal branch | 3 |
| `crates/nova-hir/src/lib.rs` | `ExprKind::FieldSet`, `ExprKind::ArrayRepeat` | 1, 3 |
| `crates/nova-typeck/src/check.rs` | `check_field_set`, mutable-receiver rule, `[init; n]` checking | 1, 2, 3 |
| `crates/nova-mir/src/lib.rs` | `Stmt::SetField`, `Stmt::ArrayAlloc` | 1, 3 |
| `crates/nova-mir/src/lower.rs` | Lower `FieldSet`; lower `ArrayRepeat` to alloc + fill loop | 1, 3 |
| `crates/nova-mir/src/mono.rs` | Substitute through the two new HIR nodes | 1, 3 |
| `crates/nova-codegen-cranelift/src/lib.rs` | Emit `SetField`, `ArrayAlloc` | 1, 3 |
| `crates/nova-codegen-llvm/src/lib.rs` | Emit `SetField`, `ArrayAlloc` | 1, 3 |
| `crates/nova-resolver/src/lib.rs` | Embedded-std seam: one module → a list; scope `Hash`'s builtin to std | 4, 6 |
| `crates/nova-driver/src/lib.rs` | Register each std module's `FileId` | 4 |
| `crates/nova-runtime/src/lib.rs` | `nova_rt_str_hash` | 6 |
| `std/collections/lib.nova` | **New.** `Vec`, `Map`, `Set` | 4, 5, 7, 8 |
| `std/core/lib.nova` | `Hash` trait + primitive impls | 6 |
| `docs/adr/0005-mutable-receivers-and-one-shot-hash.md` | **New.** Both recorded decisions | 2, 6 |
| `tests/runtime/collections.{nova,stdout}` | **New.** e2e fixture | 9 |
| `CHANGELOG.md` | `[Unreleased]` entries | 9 |

---

### Task 1: Field assignment — `rec.field = v`

**Why:** Records are immutable after construction today, which blocks `Vec::push` and essentially every future std module. This is the substantive language change of the phase.

**Files:**
- Modify: `crates/nova-hir/src/lib.rs` (add `ExprKind::FieldSet` beside `IndexSet`, ~line 702)
- Modify: `crates/nova-typeck/src/check.rs` (`check_assign` ~line 4055; add `check_field_set` beside `check_index_set` ~line 4167)
- Modify: `crates/nova-mir/src/lib.rs` (add `Stmt::SetField` beside `ArraySet`, ~line 348)
- Modify: `crates/nova-mir/src/lower.rs`, `crates/nova-mir/src/mono.rs`
- Modify: `crates/nova-codegen-cranelift/src/lib.rs` (beside `Stmt::RecordField` ~line 668), `crates/nova-codegen-llvm/src/lib.rs`
- Test: inline `mod tests` in `crates/nova-typeck/src/check.rs`; `crates/nova-cli/tests/run_tests.rs`

**Interfaces:**
- Produces: `hir::ExprKind::FieldSet { target: Box<hir::Expr>, index: u32, value: Box<hir::Expr> }`, typed `Ty::Unit`. `mir::Stmt::SetField { record: Temp, index: u32, value: Temp, ty: MirTy }`. Tasks 5, 7, 8 write Nova code that relies on this.

- [ ] **Step 1: Write the failing tests**

Add to the inline `mod tests` in `crates/nova-typeck/src/check.rs`:

```rust
    #[test]
    fn field_assignment_typechecks() {
        let r = check_src(
            "record P { v: Int }\n\
             fn main() { let mut p = P { v: 1 }\n p.v = 7\n println(\"${p.v}\") }",
        );
        assert!(r.diagnostics.is_empty(), "{:?}", r.diagnostics);
    }

    #[test]
    fn field_assignment_to_immutable_reports_e0060() {
        let r = check_src(
            "record P { v: Int }\n\
             fn main() { let p = P { v: 1 }\n p.v = 7\n println(\"${p.v}\") }",
        );
        assert!(error_codes(&r).contains(&"E0060"), "{:?}", r.diagnostics);
    }

    #[test]
    fn field_assignment_type_mismatch_reports_e0010() {
        let r = check_src(
            "record P { v: Int }\n\
             fn main() { let mut p = P { v: 1 }\n p.v = \"s\"\n println(\"${p.v}\") }",
        );
        assert!(error_codes(&r).contains(&"E0010"), "{:?}", r.diagnostics);
    }

    #[test]
    fn assignment_to_unknown_field_reports_e0014() {
        let r = check_src(
            "record P { v: Int }\n\
             fn main() { let mut p = P { v: 1 }\n p.nope = 7\n println(\"${p.v}\") }",
        );
        assert!(error_codes(&r).contains(&"E0014"), "{:?}", r.diagnostics);
    }
```

- [ ] **Step 2: Run the tests and watch them fail**

```bash
cargo test -p nova-typeck --lib -- field_assignment assignment_to_unknown_field
```

Expected: all four FAIL. The first three fail with a diagnostics list containing `E0900` ("assignment to anything but a local variable or array element"); record the exact text.

- [ ] **Step 3: Add the HIR node**

`crates/nova-hir/src/lib.rs`, beside `IndexSet`:

```rust
    /// Store `record.field = value`. `index` is the field's position, so the
    /// store offset is `8 * index` — the same layout `FieldGet` reads.
    FieldSet {
        target: Box<Expr>,
        index: u32,
        value: Box<Expr>,
    },
```

Then add `FieldSet` to `child_exprs`, `child_exprs_mut`, and `finalize_expr` in `crates/nova-typeck/src/check.rs`, and to `subst_expr` in `crates/nova-mir/src/mono.rs`. **The compiler's exhaustive matches will tell you every site** — follow them all rather than guessing.

- [ ] **Step 4: Add the typeck path**

In `check_assign` (`crates/nova-typeck/src/check.rs` ~line 4055), before the `E0900` fallback, add a `Field` arm mirroring the existing `Index` arm:

```rust
        // Field assignment `rec.field = v`.
        if let ast::Expr::Field { target, field } = &lhs.value {
            return self.check_field_set(fcx, op, target, field, rhs, span);
        }
```

Add `check_field_set` beside `check_index_set`. Model the mutability block on `check_index_set`'s exactly (it already walks the whole chain via `place_root`, so `rec.inner.f = v` and `make().f = v` are both handled):

```rust
    /// Check `target.field = value`.
    fn check_field_set(
        &mut self,
        fcx: &mut FnCtx,
        op: ast::AssignOp,
        target: &Spanned<ast::Expr>,
        field: &Spanned<String>,
        rhs: &Spanned<ast::Expr>,
        span: Span,
    ) -> hir::Expr {
        if !matches!(op, ast::AssignOp::Assign) {
            self.unsupported(span, "compound assignment to a record field");
            return error_expr(span);
        }
        // The record's storage must be reachable through a mutable binding.
        match self.place_root(fcx, target) {
            PlaceRoot::Mutable => {}
            PlaceRoot::ImmutableLocal(name) => {
                self.error(
                    "E0060",
                    format!("cannot assign to a field of immutable `{name}`"),
                    span,
                );
                self.diagnostics
                    .last_mut()
                    .expect("just pushed")
                    .notes
                    .push(format!("declare it as `let mut {name}` to allow mutation"));
            }
            PlaceRoot::NotAPlace => {
                self.error(
                    "E0060",
                    "cannot assign to a field of a temporary or non-assignable value".to_string(),
                    span,
                );
            }
        }
        let rec = self.check_expr(fcx, target);
        let recv_ty = fcx.icx.apply(&rec.ty);
        // Resolve the field to its index and declared type. `FieldGet` already
        // does this for reads — reuse the same lookup so reads and writes cannot
        // disagree about layout.
        let Some((index, field_ty)) = self.record_field_index_and_ty(fcx, &recv_ty, &field.value)
        else {
            self.error(
                "E0014",
                format!(
                    "no field `{}` on type `{}`",
                    field.value,
                    self.show(&recv_ty, fcx)
                ),
                field.span,
            );
            return error_expr(span);
        };
        let value = self.check_expr(fcx, rhs);
        if !fcx.icx.unify(&value.ty, &field_ty) {
            self.error(
                "E0010",
                format!(
                    "field `{}` has type `{}` but `{}` was assigned",
                    field.value,
                    self.show(&field_ty, fcx),
                    self.show(&value.ty, fcx),
                ),
                rhs.span,
            );
        }
        hir::Expr {
            kind: hir::ExprKind::FieldSet {
                target: Box::new(rec),
                index,
                value: Box::new(value),
            },
            ty: Ty::Unit,
            span,
        }
    }
```

`record_field_index_and_ty` does not exist yet — you are creating it. **Read how the field-read path (`ast::Expr::Field` → `hir::ExprKind::FieldGet`) resolves a field name to its index and substituted type, and factor that lookup into a shared helper used by both paths** — reads and writes must not disagree about layout or generic substitution, and duplicating the logic is exactly how they would drift. Give it this signature:

```rust
    /// Resolve a field name on a record type to its `(index, substituted type)`.
    /// Shared by the field read and field write paths so they cannot disagree.
    fn record_field_index_and_ty(
        &mut self,
        fcx: &mut FnCtx,
        recv_ty: &Ty,
        field: &str,
    ) -> Option<(u32, Ty)>
```

The read path must then call it too, not keep its own copy. If that refactor changes any existing behavior, investigate — it should be a pure extraction.

- [ ] **Step 5: Lower to MIR**

`crates/nova-mir/src/lib.rs`, beside `ArraySet`:

```rust
    /// Store field `index` of a record. Mirrors `RecordField`'s `8 * index`
    /// offset, so reads and writes stay in the same layout.
    SetField {
        record: Temp,
        index: u32,
        value: Temp,
        ty: MirTy,
    },
```

In `crates/nova-mir/src/lower.rs`'s `lower_expr`, beside the `K::IndexSet` arm:

```rust
            K::FieldSet {
                target,
                index,
                value,
            } => {
                let rec = self.lower_expr(target);
                let ty = mir_ty(&value.ty);
                let v = self.lower_expr(value);
                self.push(Stmt::SetField {
                    record: rec,
                    index: *index,
                    value: v,
                    ty,
                });
                self.unit_temp()
            }
```

Check the `IndexSet` arm for the evaluation-order and `diverges` conventions it follows and match them — a past review found a record-literal evaluation-order bug, so order matters here.

- [ ] **Step 6: Emit in both backends**

Cranelift (`crates/nova-codegen-cranelift/src/lib.rs`), beside `Stmt::RecordField`:

```rust
            Stmt::SetField {
                record,
                index,
                value,
                ty,
            } => {
                if self.cg.cl_ty(*ty).is_none() {
                    return Ok(());
                }
                let ptr = self.use_temp(*record)?;
                let v = self.use_temp(*value)?;
                let offset = (8 * index) as i32;
                self.builder
                    .ins()
                    .store(MemFlags::trusted(), v, ptr, offset);
            }
```

LLVM (`crates/nova-codegen-llvm/src/lib.rs`): follow how `Stmt::RecordField` computes its address (a `getelementptr`-style offset then a `load`) and emit the mirrored `store`. A `MirTy::Unit` field is skipped, exactly as the Cranelift arm skips it.

- [ ] **Step 7: Run the tests and watch them pass**

```bash
cargo test -p nova-typeck --lib -- field_assignment assignment_to_unknown_field
```

Expected: all four PASS.

- [ ] **Step 8: Add the e2e test, including the documented alias semantics**

Create `tests/runtime/field_assign.nova`:

```nova
record Counter { n: Int, label: String }

fn main() {
    let mut c = Counter { n: 0, label: "hits" }
    c.n = c.n + 1
    c.n = c.n + 1
    println("${c.label}=${c.n}")

    // Records are heap objects, so assignment through one binding is visible
    // through another. This is deliberate reference semantics (ADR 0005).
    let mut alias = c
    alias.n = 99
    println("alias visible: ${c.n}")

    // A field holding a heap value.
    c.label = "misses"
    println("${c.label}")
}
```

Create `tests/runtime/field_assign.stdout`:

```
hits=2
alias visible: 99
misses
```

Add to `crates/nova-cli/tests/run_tests.rs`, following the neighbouring `*_run` / `*_build_standalone` pairs (read one — some use a `build_and_run` helper):

```rust
#[test]
fn field_assign_run() {
    let expected = std::fs::read_to_string(repo_root().join("tests/runtime/field_assign.stdout"))
        .expect("expected-output fixture exists")
        .replace("\r\n", "\n");
    nova()
        .arg("run")
        .arg(repo_root().join("tests/runtime/field_assign.nova"))
        .assert()
        .success()
        .stdout(expected);
}

#[test]
fn field_assign_build_standalone() {
    let expected = std::fs::read_to_string(repo_root().join("tests/runtime/field_assign.stdout"))
        .expect("expected-output fixture exists")
        .replace("\r\n", "\n");
    let out = build_and_run("tests/runtime/field_assign.nova", "field_assign");
    assert_eq!(out.replace("\r\n", "\n"), expected);
}
```

- [ ] **Step 9: Verify everything**

```bash
cargo test --workspace --no-fail-fast && cargo clippy --all-targets --all-features -- -D warnings && cargo fmt --check
```

Expected: all pass. If `fmt` complains, run `cargo fmt` and re-run.

- [ ] **Step 10: Commit**

```bash
git add -A && git commit -m "feat: record field assignment

\`rec.field = v\` now compiles. Records were immutable after construction,
which blocked every collection and most future std work.

Mutability reuses the existing \`place_root\` chain walk, so \`rec.inner.f = v\`
and \`make().f = v\` are rejected at the root exactly as array element
assignment already was (E0060). The store mirrors \`RecordField\`'s 8*index
offset in both backends, so reads and writes cannot disagree about layout.

Records are heap objects, so assignment is alias-visible; that is deliberate
reference semantics and is pinned by a test.

Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>"
```

---

### Task 2: The mutable-receiver rule + ADR 0005

**Why:** With `mut` illegal on record *fields*, a mutating method is written `fn push(mut self, …)`. Nothing otherwise forces the **caller's** receiver to be mutable, so `v.push(x)` would mutate `v` after `let v = …` — inconsistent with `arr[0] = v` being rejected on an immutable binding. This makes `mut` mean something.

**Files:**
- Modify: `crates/nova-typeck/src/check.rs` (`check_method_call`; the method-signature collection that records `mut self`)
- Create: `docs/adr/0005-mutable-receivers-and-one-shot-hash.md`
- Test: inline `mod tests` in `crates/nova-typeck/src/check.rs`

**Interfaces:**
- Consumes: `place_root` and `PlaceRoot` (existing).
- Produces: a `Checker` set of method `DefId`s that declare `mut self` — name it `mut_self: FxHashSet<DefId>`, mirroring the existing `selfless: FxHashSet<DefId>`. Tasks 5, 7, 8 write `mut self` methods and callers that must therefore use `let mut`.

- [ ] **Step 1: Write the failing tests**

```rust
    #[test]
    fn mut_self_method_on_immutable_receiver_reports_e0060() {
        let r = check_src(
            "record P { v: Int }\n\
             impl P { fn bump(mut self) { self.v = self.v + 1 } }\n\
             fn main() { let p = P { v: 1 }\n p.bump()\n println(\"${p.v}\") }",
        );
        assert!(error_codes(&r).contains(&"E0060"), "{:?}", r.diagnostics);
    }

    #[test]
    fn mut_self_method_on_mutable_receiver_typechecks() {
        let r = check_src(
            "record P { v: Int }\n\
             impl P { fn bump(mut self) { self.v = self.v + 1 } }\n\
             fn main() { let mut p = P { v: 1 }\n p.bump()\n println(\"${p.v}\") }",
        );
        assert!(r.diagnostics.is_empty(), "{:?}", r.diagnostics);
    }

    #[test]
    fn plain_self_method_on_immutable_receiver_still_typechecks() {
        // Guard: only `mut self` demands a mutable receiver.
        let r = check_src(
            "record P { v: Int }\n\
             impl P { fn get(self) -> Int { self.v } }\n\
             fn main() { let p = P { v: 1 }\n println(\"${p.get()}\") }",
        );
        assert!(r.diagnostics.is_empty(), "{:?}", r.diagnostics);
    }
```

- [ ] **Step 2: Run the tests and watch them fail**

```bash
cargo test -p nova-typeck --lib -- mut_self_method plain_self_method
```

Expected: `mut_self_method_on_immutable_receiver_reports_e0060` FAILS with an empty `[]` list (currently accepted). The other two should already PASS — they are guards. **If `mut_self_method_on_mutable_receiver_typechecks` fails, Task 1 is incomplete.**

- [ ] **Step 3: Record which methods declare `mut self`**

Add the field to `struct Checker` beside `selfless`:

```rust
    /// Methods whose `self` receiver is declared `mut`. Calling one requires a
    /// mutable receiver place at the call site, so `mut` keeps the meaning it
    /// already has for `arr[i] = v` and `rec.f = v`.
    mut_self: FxHashSet<DefId>,
```

Initialise it where `Checker` is constructed. Populate it in `collect_impls` where `has_self` is computed — the receiver is the `ast::Param` named `self`, and `Param` has an `is_mut` field:

```rust
                if f.params.iter().any(|p| p.name.value == "self" && p.is_mut) {
                    self.mut_self.insert(def_id);
                }
```

Note the existing self-ness predicate uses `.any(…)`, not `.first()`, because `method_sig_parts` strips a `self` at **any** position. Use `.any(…)` here too, for the same reason.

- [ ] **Step 4: Enforce it at the call site**

In `check_method_call`, once the call resolves to an inherent method `def_id` (the `MethodRes::Inherent` arm), require a mutable receiver place. The receiver's **AST** node is needed for `place_root`, so thread it in — `check_call` already has it as `target` in the `ast::Expr::Field` branch that builds method calls:

```rust
        if self.mut_self.contains(&def_id) {
            match self.place_root(fcx, receiver_ast) {
                PlaceRoot::Mutable => {}
                PlaceRoot::ImmutableLocal(name) => {
                    let mname = self.defs.def(def_id).name.clone();
                    self.error(
                        "E0060",
                        format!("`{mname}` mutates its receiver, but `{name}` is immutable"),
                        span,
                    );
                    self.diagnostics
                        .last_mut()
                        .expect("just pushed")
                        .notes
                        .push(format!("declare it as `let mut {name}` to allow mutation"));
                }
                PlaceRoot::NotAPlace => {
                    let mname = self.defs.def(def_id).name.clone();
                    self.error(
                        "E0060",
                        format!(
                            "`{mname}` mutates its receiver, which cannot be a temporary"
                        ),
                        span,
                    );
                }
            }
        }
```

**Decide and report** whether trait-method calls (`MethodRes::Trait`) also need this. A trait method's `mut self` lives on the *trait* declaration, not the impl, so enforcing it there needs a `mut_self` flag on `hir::TraitMethod` (mirroring the existing `has_self`). The collections in Tasks 5/7/8 use **inherent** impls only, so the inherent path is sufficient for this plan — if you defer the trait path, say so explicitly and note it as a gap rather than leaving it silent.

- [ ] **Step 5: Run the tests and watch them pass**

```bash
cargo test -p nova-typeck --lib -- mut_self_method plain_self_method
```

Expected: all three PASS.

- [ ] **Step 6: Write ADR 0005**

Create `docs/adr/0005-mutable-receivers-and-one-shot-hash.md`, following the structure of `docs/adr/0004-stdlib-compile-model.md` (read it first). Cover section 1 now; Task 6 appends section 2.

Section 1 — **mutable receivers**: the decision (a method declaring `mut self` requires a `Mutable` receiver place, else `E0060`); the alternative rejected (Java/Python-style mutation through any binding, rejected because it makes `mut` inconsistent with the existing `arr[i] = v` and `rec.f = v` rules); the consequences (`let mut v = Vec::new()` is required, every std API that mutates must declare `mut self`, and the trait-method path is or is not covered — state which); and that field assignment is **alias-visible** because records are heap objects, with the `let mut alias = c` example from Task 1's fixture.

Leave a section-2 heading for Task 6 so the file has one obvious insertion point.

- [ ] **Step 7: Verify and commit**

```bash
cargo test --workspace --no-fail-fast && cargo clippy --all-targets --all-features -- -D warnings && cargo fmt --check
git add -A && git commit -m "feat(typeck): a \`mut self\` method requires a mutable receiver (ADR 0005)

Nova puts \`mut\` on bindings, not record fields, so a mutating method is
written \`fn push(mut self, …)\`. Nothing forced the caller's receiver to be
mutable, so \`v.push(x)\` would mutate \`v\` after \`let v = …\` while a direct
\`v.field = x\` was rejected — the same operation, two answers.

Calling a \`mut self\` method now requires a mutable receiver place (E0060),
reusing the same \`place_root\` walk as the other assignment forms.

Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>"
```

---

### Task 3: Repeat-array literal — `[init; n]`

**Why:** `Vec` growth needs a runtime-length array, and there is currently no way to allocate one — arrays come only from literals. Using a caller-supplied `init` means no uninitialized or null-filled memory and no `Default` bound.

**Files:**
- Modify: `crates/nova-ast/src/expr.rs` (`Expr::ArrayRepeat`)
- Modify: `crates/nova-parser/src/grammar.rs` (array-literal branch, ~line 1560-1588)
- Modify: `crates/nova-hir/src/lib.rs` (`ExprKind::ArrayRepeat`)
- Modify: `crates/nova-typeck/src/check.rs`, `crates/nova-mir/src/lib.rs`, `crates/nova-mir/src/lower.rs`, `crates/nova-mir/src/mono.rs`
- Modify: `crates/nova-codegen-cranelift/src/lib.rs`, `crates/nova-codegen-llvm/src/lib.rs`
- Test: `crates/nova-parser/tests/parser_tests.rs`, inline `mod tests` in `check.rs`, `crates/nova-cli/tests/run_tests.rs`

**Interfaces:**
- Produces: `[init; n]` syntax; `hir::ExprKind::ArrayRepeat { init: Box<hir::Expr>, len: Box<hir::Expr> }` typed `Ty::Array(elem)`; `mir::Stmt::ArrayAlloc { dst: Temp, len: Temp }` which allocates `8 + 8*len` bytes and stores `len` at offset 0. Task 5 uses `[x; n]` for `Vec` growth.

- [ ] **Step 1: Write the failing tests**

Parser test in `crates/nova-parser/tests/parser_tests.rs` (match the file's existing style — check whether it uses `insta` snapshots before inventing one):

```rust
#[test]
fn parses_repeat_array_literal() {
    let src = "fn main() { let n = 3\n let a = [0; n] }";
    let (tokens, lex_errors) = nova_lexer::lex(src, nova_diagnostics::FileId::DUMMY);
    assert!(lex_errors.is_empty(), "{lex_errors:?}");
    let (ast, parse_errors) = nova_parser::parse(&tokens, nova_diagnostics::FileId::DUMMY);
    assert!(parse_errors.is_empty(), "{parse_errors:?}");
    assert!(ast.is_some());
}
```

Typeck tests in `check.rs`:

```rust
    #[test]
    fn repeat_array_typechecks_and_has_array_type() {
        let r = check_src(
            "fn main() { let n = 3\n let a = [7; n]\n println(\"${a.len()} ${a[0]}\") }",
        );
        assert!(r.diagnostics.is_empty(), "{:?}", r.diagnostics);
    }

    #[test]
    fn repeat_array_non_int_length_reports_e0010() {
        let r = check_src("fn main() { let a = [7; \"three\"]\n println(\"${a[0]}\") }");
        assert!(error_codes(&r).contains(&"E0010"), "{:?}", r.diagnostics);
    }
```

- [ ] **Step 2: Run the tests and watch them fail**

```bash
cargo test -p nova-parser --test parser_tests -- parses_repeat_array_literal
cargo test -p nova-typeck --lib -- repeat_array
```

Expected: the parser test FAILS with a `P0001` "expected `]` (in array literal), found `;`"; both typeck tests FAIL for the same parse reason.

- [ ] **Step 3: Add the AST variant and parse it**

`crates/nova-ast/src/expr.rs`:

```rust
    /// `[init; n]` — an array of `n` copies of `init`, `n` evaluated at runtime.
    ArrayRepeat {
        init: Box<Spanned<Expr>>,
        len: Box<Spanned<Expr>>,
    },
```

In `crates/nova-parser/src/grammar.rs`'s array-literal branch: after parsing the **first** element, if the next token is `Semi`, parse the length expression, expect `]`, and return `Expr::ArrayRepeat`. Otherwise continue the existing comma loop unchanged. Confirm the semicolon token's real name in `crates/nova-lexer` (`Token::Semi` vs `Token::Semicolon`) rather than assuming.

- [ ] **Step 4: Check it in typeck**

Add an `ast::Expr::ArrayRepeat` arm to `check_expr`:

```rust
            ast::Expr::ArrayRepeat { init, len } => {
                let init_hir = self.check_expr(fcx, init);
                let len_hir = self.check_expr(fcx, len);
                self.expect_ty(fcx, &len_hir, &Ty::Int, "an array length");
                let elem_ty = fcx.icx.apply(&init_hir.ty);
                hir::Expr {
                    kind: hir::ExprKind::ArrayRepeat {
                        init: Box::new(init_hir),
                        len: Box::new(len_hir),
                    },
                    ty: Ty::Array(Box::new(elem_ty)),
                    span,
                }
            }
```

Add the matching `ExprKind::ArrayRepeat` to `crates/nova-hir/src/lib.rs`, then follow the compiler's exhaustive-match errors to `child_exprs`, `child_exprs_mut`, `finalize_expr`, and `mono.rs`'s `subst_expr`.

- [ ] **Step 5: Lower to MIR as alloc + fill loop**

Add one statement to `crates/nova-mir/src/lib.rs`:

```rust
    /// Allocate an array of `len` elements: `8 + 8*len` zeroed bytes with `len`
    /// stored at offset 0. Elements are filled by the lowering's own loop.
    ArrayAlloc {
        dst: Temp,
        len: Temp,
    },
```

In `lower.rs`, lower `ArrayRepeat` as: evaluate `init` and `len`, emit `ArrayAlloc`, then emit a **counted loop** in MIR that stores `init` into each slot via the existing `Stmt::ArraySet`. Build the loop with the same block/branch helpers the `for`-desugar and `while` lowering already use — read one of them first and match its shape. Doing the loop in MIR rather than in codegen means both backends need only `ArrayAlloc`.

`gc::alloc` zeroes, so a `len` of `0` yields a valid empty array with no loop iterations, and a negative `len` must not allocate a wild size — clamp with `len.max(0)` semantics in the lowering (or emit a guard) and **state which you chose**.

- [ ] **Step 6: Emit `ArrayAlloc` in both backends**

Cranelift — model it on the `Stmt::MakeArray` arm (~line 685), which already does exactly this for a static length:

```rust
            Stmt::ArrayAlloc { dst, len } => {
                let n = self.use_temp(*len)?;
                let eight = self.builder.ins().iconst(types::I64, 8);
                let bytes = self.builder.ins().imul(n, eight);
                let size = self.builder.ins().iadd(bytes, eight);
                let alloc = self.rt("nova_rt_alloc");
                let ptr = self
                    .call_func_id(alloc, &[size])?
                    .ok_or_else(|| anyhow!("alloc returns a value"))?;
                self.builder.ins().store(MemFlags::trusted(), n, ptr, 0);
                self.def_temp(*dst, ptr);
            }
```

LLVM: emit the same shape — `mul`/`add` for the size, a call to `@nova_rt_alloc`, then a `store` of the length at offset 0. Follow how the existing `MakeArray` arm emits its allocation and length store.

- [ ] **Step 7: Add a MIR lowering test, then run everything**

The fill loop is lowered in MIR rather than codegen, so pin that it actually allocates and stores. Add to `crates/nova-mir/tests/lower_tests.rs`, using the file's existing helpers (check their real names at the top — recent tasks used `mir_for` and `function_names`):

```rust
#[test]
fn repeat_array_lowers_to_alloc_plus_fill_loop() {
    let m = mir_for("fn main() { let n = 3\n let a = [7; n]\n println(\"${a[0]}\") }");
    let main = m
        .functions
        .iter()
        .find(|f| f.name == "main")
        .expect("main exists");
    let stmts: Vec<&Stmt> = main.blocks.iter().flat_map(|b| b.stmts.iter()).collect();
    assert!(
        stmts.iter().any(|s| matches!(s, Stmt::ArrayAlloc { .. })),
        "expected an ArrayAlloc"
    );
    assert!(
        stmts.iter().any(|s| matches!(s, Stmt::ArraySet { .. })),
        "expected the fill loop's ArraySet"
    );
    // The fill loop needs more than one block, unlike a static array literal.
    assert!(main.blocks.len() > 1, "expected a loop, got {} block(s)", main.blocks.len());
}
```

```bash
cargo test -p nova-parser --test parser_tests -- parses_repeat_array_literal
cargo test -p nova-typeck --lib -- repeat_array
cargo test -p nova-mir -- repeat_array_lowers_to_alloc_plus_fill_loop
```

Expected: PASS.

- [ ] **Step 8: Add the e2e test**

Create `tests/runtime/array_repeat.nova`:

```nova
fn main() {
    let n = 4
    let a = [7; n]
    println("${a.len()}")
    println("${a[0]} ${a[3]}")

    let mut b = [0; n]
    b[2] = 5
    println("${b[1]} ${b[2]}")

    // A zero length is valid and allocates an empty array.
    let e = [1; 0]
    println("empty: ${e.len()}")

    // The filler may be a heap value.
    let s = ["hi"; 2]
    println("${s[0]} ${s[1]}")
}
```

Create `tests/runtime/array_repeat.stdout`:

```
4
7 7
0 5
empty: 0
hi hi
```

Add `array_repeat_run` and `array_repeat_build_standalone` to `crates/nova-cli/tests/run_tests.rs`, following the neighbouring pairs.

- [ ] **Step 9: Verify and commit**

```bash
cargo test --workspace --no-fail-fast && cargo clippy --all-targets --all-features -- -D warnings && cargo fmt --check
git add -A && git commit -m "feat: repeat-array literal \`[init; n]\`

Arrays could only come from literals, so there was no way to allocate one of
runtime length — which is exactly what a growable collection needs.

\`init\` is a caller-supplied value, so a fresh array is never uninitialized or
null-filled and no \`Default\` bound is required. The fill loop is emitted in MIR
using the existing block machinery, so both backends need only the new
\`ArrayAlloc\` statement.

Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>"
```

---

### Task 4: A second embedded std module

**Why:** `std/core` is loaded through a single-module seam (ADR 0004). `std/collections` is a second module, and keeping it in its own file keeps both readable — `std/core` is already ~175 lines and the collections add several hundred.

**Files:**
- Create: `std/collections/lib.nova`
- Modify: `crates/nova-resolver/src/lib.rs` (the `STD_CORE_SRC` / `std_core_module` / `std_core_mid` / `import_std_core` / builtin-gating seam)
- Modify: `crates/nova-driver/src/lib.rs` (register each std module's `FileId`)
- Test: inline `mod tests` in `crates/nova-resolver/src/lib.rs`

**Interfaces:**
- Produces: a list of embedded std modules rather than one. Name the list `STD_MODULES: [(&str, &str); 2]` of `(module_name, source)` and expose the set of std `ModuleId`s so the builtin gating can ask "is this *a* std module" instead of "is this *the* std module". Tasks 5–8 add code to `std/collections/lib.nova`.

- [ ] **Step 1: Write the failing test**

```rust
    #[test]
    fn std_collections_module_is_compiled_and_visible() {
        // A name defined in std/collections must resolve from a user module,
        // exactly as std/core's names do.
        let r = resolve_src("fn main() { }");
        assert!(
            r.definitions.resolve_type(ModuleId(0), "Vec").is_some(),
            "Vec should be visible from a user module"
        );
    }
```

Use whatever helper the file's existing std/core visibility tests use (e.g. `resolve_src`, and the `user_type_shadows_std_core` test shows how they reason about the std module boundary) — read them and match.

- [ ] **Step 2: Run it and watch it fail**

```bash
cargo test -p nova-resolver --lib -- std_collections_module_is_compiled_and_visible
```

Expected: FAIL — `Vec` does not exist yet.

- [ ] **Step 3: Create the module with a placeholder-free stub**

Create `std/collections/lib.nova`:

```nova
// Nova standard library — collections.
//
// Compiled as an implicit module and glob-imported into every user module, so
// these names need no `import`. A user definition of the same name shadows the
// one here (see docs/adr/0004-stdlib-compile-model.md).
//
// `Vec` grows by allocating a larger `[T]` and reassigning the field: the record
// object's address never changes and the array's only referent is that field, so
// the conservative non-moving collector needs no special handling.

pub record Vec<T> { len: Int, data: [T] }
```

Tasks 5, 7, and 8 fill in the methods and the other two types. Declaring `Vec` now is what makes Step 1's test meaningful.

- [ ] **Step 4: Generalize the seam**

In `crates/nova-resolver/src/lib.rs`, replace the single `STD_CORE_SRC` / `STD_CORE_NAME` pair with a list, keeping `std/core` **first** so its declaration order is unchanged:

```rust
/// The embedded standard-library modules, compiled as implicit modules and
/// glob-imported into every user module. Order is significant only in that it
/// fixes module indices; user modules always come first.
pub const STD_MODULES: [(&str, &str); 2] = [
    ("$std.core", include_str!("../../../std/core/lib.nova")),
    ("$std.collections", include_str!("../../../std/collections/lib.nova")),
];
```

Then thread the plural through: `resolve_program` takes one `FileId` **per** std module (a slice), parses each, appends each after the user modules, glob-imports each, and skips self-import for each. The builtin gating that scopes `str_cmp` to std must become "seed into any std module", not "the std module" — that gating is the one place a subtle bug hides, so check it explicitly.

The driver registers each source: read the existing single `self.db.add("<std/core>", …)` call and generalize it to one per entry, naming each `<std/NAME>` so diagnostics still point at a real file.

- [ ] **Step 5: Run the test and the suite**

```bash
cargo test -p nova-resolver --lib -- std_collections_module_is_compiled_and_visible
cargo test --workspace --no-fail-fast
```

Expected: PASS. If many unrelated tests fail, a module index or the glob-import loop is wrong — the std modules must come **after** all user modules.

- [ ] **Step 6: Prove `str_cmp` is still std-scoped**

The existing test `user_fn_named_str_cmp_is_not_a_reserved_word` guards that a user may define `str_cmp`. Confirm it still passes, and confirm `std/core`'s `Ord for String` still works (it calls `str_cmp`):

```bash
cargo test -p nova-resolver --lib -- user_fn_named_str_cmp_is_not_a_reserved_word
cargo test -p nova-cli --test run_tests -- std_core
```

Expected: PASS. This is the regression the generalization is most likely to break.

- [ ] **Step 7: Verify and commit**

```bash
cargo test --workspace --no-fail-fast && cargo clippy --all-targets --all-features -- -D warnings && cargo fmt --check
git add -A && git commit -m "refactor(resolver): allow more than one embedded std module

std/core was loaded through a seam that assumed exactly one implicit module.
std/collections is the second, and keeping each in its own file keeps both
readable. The seam is now a list, the driver registers a FileId per module so
diagnostics still name a real file, and the std-scoped builtin gating asks
whether a module is *a* std module rather than *the* std module.

Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>"
```

---

### Task 5: `Vec<T>`

**Why:** The foundation of the slice, and the first real consumer of both new language features.

**Files:**
- Modify: `std/collections/lib.nova`
- Test: inline `mod tests` in `crates/nova-typeck/src/check.rs`; e2e comes in Task 9

**Interfaces:**
- Consumes: field assignment (Task 1), `mut self` rule (Task 2), `[init; n]` (Task 3).
- Produces: `Vec<T>` with `new() -> Vec<T>`, `len(self) -> Int`, `push(mut self, x: T)`, `pop(mut self) -> Option<T>`, `get(self, i: Int) -> Option<T>`, `set(mut self, i: Int, v: T)`, `clear(mut self)`. Tasks 7 and 9 use these.

- [ ] **Step 1: Write the failing test**

```rust
    #[test]
    fn vec_methods_typecheck() {
        let r = check_src(
            "fn main() {\n\
                 let mut v = Vec::new()\n\
                 v.push(1)\n\
                 v.push(2)\n\
                 println(\"${v.len()}\")\n\
                 match v.get(0) { Some(x) => println(\"${x}\"), None => println(\"none\") }\n\
                 match v.pop() { Some(x) => println(\"${x}\"), None => println(\"none\") }\n\
                 v.set(0, 9)\n\
                 v.clear()\n\
                 println(\"${v.len()}\")\n\
             }",
        );
        assert!(r.diagnostics.is_empty(), "{:?}", r.diagnostics);
    }
```

- [ ] **Step 2: Run it and watch it fail**

```bash
cargo test -p nova-typeck --lib -- vec_methods_typecheck
```

Expected: FAIL — no `new` on `Vec`, no `push`, etc.

- [ ] **Step 3: Implement `Vec` in Nova**

Replace the `Vec` declaration in `std/collections/lib.nova` with:

```nova
pub record Vec<T> { len: Int, data: [T] }

impl<T> Vec<T> {
    /// An empty vector. No storage is allocated until the first `push`.
    pub fn new() -> Vec<T> { Vec { len: 0, data: [] } }

    pub fn len(self) -> Int { self.len }

    pub fn is_empty(self) -> Bool { self.len == 0 }

    /// Append `x`, growing if full.
    ///
    /// Growth allocates `[x; newcap]` — using the pushed element as the filler,
    /// so no slot is ever uninitialized and no `Default` bound is needed — then
    /// copies the existing elements back. Slot `self.len` is then already `x`,
    /// so the append needs no further store on the growth path.
    pub fn push(mut self, x: T) {
        if self.len == self.data.len() {
            let newcap = if self.len == 0 { 4 } else { self.len * 2 }
            let mut grown = [x; newcap]
            for i in 0..self.len { grown[i] = self.data[i] }
            self.data = grown
        } else {
            self.data[self.len] = x
        }
        self.len = self.len + 1
    }

    pub fn pop(mut self) -> Option<T> {
        if self.len == 0 {
            None
        } else {
            let last = self.data[self.len - 1]
            self.len = self.len - 1
            Some(last)
        }
    }

    pub fn get(self, i: Int) -> Option<T> {
        if i < 0 { None } else { if i >= self.len { None } else { Some(self.data[i]) } }
    }

    pub fn set(mut self, i: Int, v: T) {
        if i < 0 { panic("Vec::set index out of range") }
        if i >= self.len { panic("Vec::set index out of range") }
        self.data[i] = v
    }

    pub fn clear(mut self) { self.len = 0 }
}
```

Note the growth path already stores `x` at slot `self.len` via the filler, which is why `push` does not store again after growing. If you restructure it, keep that reasoning correct or add the store back.

- [ ] **Step 4: Run the test and watch it pass**

```bash
cargo test -p nova-typeck --lib -- vec_methods_typecheck
```

Expected: PASS. If `push` reports `E0060`, Task 2's rule is firing on `self` — a `mut self` parameter must itself count as a mutable root in `place_root`; fix that in Task 2's code rather than weakening `Vec`.

- [ ] **Step 5: Confirm it runs, across several doublings**

```bash
cargo build -p nova-cli
```

```bash
printf 'fn main() {\n  let mut v = Vec::new()\n  for i in 0..10 { v.push(i * i) }\n  println("${v.len()}")\n  println("${v.get(0).unwrap_or(-1)} ${v.get(9).unwrap_or(-1)} ${v.get(10).unwrap_or(-1)}")\n  println("${v.pop().unwrap_or(-1)} ${v.len()}")\n}\n' > /d/tmp/vec.nova
./target/debug/nova.exe run /d/tmp/vec.nova
```

Expected: `10`, then `0 81 -1`, then `81 9`. Ten pushes cross the 4→8→16 growth boundaries, so this exercises growth twice. Also run it under `NOVA_GC_STRESS=1` and confirm identical output — growth allocates heavily, and this is where a conservative collector would show a problem.

- [ ] **Step 6: Verify and commit**

```bash
cargo test --workspace --no-fail-fast && cargo clippy --all-targets --all-features -- -D warnings && cargo fmt --check
git add -A && git commit -m "feat(std): Vec<T> in std/collections

A growable vector written in Nova: new/len/is_empty/push/pop/get/set/clear.
Growth doubles from 4 and uses the pushed element as the filler, so no slot is
ever uninitialized and no Default bound is needed.

get returns Option<T> by value rather than the spec's Option<&T>, since Nova
has no references; for heap types the value is the pointer, so it still behaves
referentially. with_capacity is omitted: it would need a T to fill with, and
Nova cannot express reserved-but-uninitialized capacity.

Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>"
```

---

### Task 6: `Hash` + `nova_rt_str_hash`

**Why:** `Map` and `Set` need hashing. `Hash` was deferred from Phase 2.1.

**Files:**
- Modify: `std/core/lib.nova` (the `Hash` trait and primitive impls live with the other core traits)
- Modify: `crates/nova-runtime/src/lib.rs` (`nova_rt_str_hash`)
- Modify: `crates/nova-mir/src/lib.rs` (`RtFunc::StrHash`)
- Modify: `crates/nova-resolver/src/lib.rs` (a std-scoped `str_hash` builtin), `crates/nova-typeck/src/check.rs` (`check_builtin_call`), `crates/nova-mir/src/lower.rs`
- Modify: `docs/adr/0005-mutable-receivers-and-one-shot-hash.md` (section 2)
- Test: `crates/nova-runtime/src/lib.rs` inline tests; inline `mod tests` in `check.rs`

**Interfaces:**
- Consumes: `RtFunc::ALL` is now macro-generated — adding a variant to the macro's list is all that is needed; both backends derive their declarations from it, so **no codegen edits**.
- Produces: `pub trait Hash { fn hash(self) -> Int }` with impls for `Int`, `Bool`, `Char`, `String`; a std-scoped builtin `str_hash(s: String) -> Int`. Tasks 7 and 8 use `Hash`.

- [ ] **Step 1: Write the failing tests**

Runtime test in `crates/nova-runtime/src/lib.rs`'s inline `mod tests` (use the existing `make_str` helper):

```rust
    #[test]
    fn str_hash_is_deterministic_and_distinguishes() {
        let a = make_str("hello");
        let b = make_str("hello");
        let c = make_str("world");
        unsafe {
            assert_eq!(nova_rt_str_hash(a), nova_rt_str_hash(b));
            assert_ne!(nova_rt_str_hash(a), nova_rt_str_hash(c));
        }
    }

    #[test]
    fn str_hash_handles_empty() {
        let e = make_str("");
        // Must not panic and must be stable.
        unsafe { assert_eq!(nova_rt_str_hash(e), nova_rt_str_hash(make_str(""))) };
    }
```

Typeck test in `check.rs`:

```rust
    #[test]
    fn hash_impls_typecheck_for_primitives() {
        let r = check_src(
            "fn h<T: Hash>(x: T) -> Int { x.hash() }\n\
             fn main() {\n\
                 println(\"${h(7)} ${h(true)} ${h(\"s\")}\")\n\
             }",
        );
        assert!(r.diagnostics.is_empty(), "{:?}", r.diagnostics);
    }
```

- [ ] **Step 2: Run them and watch them fail**

```bash
cargo test -p nova-runtime --lib -- str_hash
cargo test -p nova-typeck --lib -- hash_impls_typecheck_for_primitives
```

Expected: the runtime tests FAIL to compile (`cannot find function 'nova_rt_str_hash'`); the typeck test FAILS with `cannot find trait 'Hash'`.

- [ ] **Step 3: Add the runtime hash**

In `crates/nova-runtime/src/lib.rs`, beside `nova_rt_str_cmp` (FNV-1a — small, well-known, and adequate for a hash map):

```rust
/// FNV-1a hash of a Nova string's bytes, as an i64.
///
/// # Safety
/// `s` must point to a valid `NovaStr`.
#[no_mangle]
pub unsafe extern "C" fn nova_rt_str_hash(s: *const NovaStr) -> i64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in as_str(s).as_bytes() {
        h ^= *b as u64;
        h = h.wrapping_mul(0x0000_0100_0000_01b3);
    }
    h as i64
}
```

Register it in `symbols()`, and add `StrHash` to the `RtFunc` macro list in `crates/nova-mir/src/lib.rs` with symbol `"nova_rt_str_hash"` and signature `(vec![MirTy::Ptr], MirTy::I64)`. Both backends derive declarations from that list, so no codegen changes — confirm by grepping for a hand-written declaration list and finding none.

- [ ] **Step 4: Expose it as a std-scoped builtin**

Follow `str_cmp` exactly: add a `StrHash` variant to `Builtin`, put it in the **std-only** list (not `GLOBAL`), give it `name() == "str_hash"`, type it in `check_builtin_call` as one `String` argument returning `Ty::Int`, and map it to `RtFunc::StrHash` in `lower.rs`. It must **not** become a globally reserved word — a user may still define `str_hash`.

- [ ] **Step 5: Add `Hash` to `std/core/lib.nova`**

```nova
pub trait Hash { fn hash(self) -> Int }

// The splitmix64 finalizer: three xor-shift/multiply rounds. Identity hashing
// would cluster badly against Map's power-of-two bucket masks, so mix.
fn mix64(x: Int) -> Int {
    let a = x ^ (x >> 30)
    let b = a * -4658895280553007687
    let c = b ^ (b >> 27)
    let d = c * -7723592293110705685
    d ^ (d >> 31)
}

impl Hash for Int    { fn hash(self) -> Int { mix64(self) } }
impl Hash for Bool   { fn hash(self) -> Int { if self { mix64(1) } else { mix64(0) } } }
impl Hash for Char   { fn hash(self) -> Int { mix64(self.to_int()) } }
impl Hash for String { fn hash(self) -> Int { str_hash(self) } }
```

Two things to settle rather than guess:

1. **The splitmix64 constants** are `0xbf58476d1ce4e5b9` and `0x94d049bb133111eb`, which exceed `i64::MAX`. The two's-complement negatives above are those bit patterns. **Verify** Nova's lexer accepts these literals and that `*` wraps rather than trapping on overflow; if either fails, compute the hash in the runtime instead (`nova_rt_mix64`) and say why.
2. **`Char::to_int()` may not exist.** Check what `Char` supports; if there is no conversion, either add one or hash `Char` through a runtime helper. Do not silently drop `Hash for Char` — report it if you must.

`mix64` is a module-private helper (no `pub`), so it does not enter user namespaces.

- [ ] **Step 6: Run the tests and watch them pass**

```bash
cargo test -p nova-runtime --lib -- str_hash
cargo test -p nova-typeck --lib -- hash_impls_typecheck_for_primitives
cargo test -p nova-resolver --lib -- str_cmp
```

Expected: PASS, including the `str_cmp` scoping test (the new builtin must not have disturbed it).

- [ ] **Step 7: Append ADR 0005 section 2**

Add the **one-shot `Hash`** section to `docs/adr/0005-mutable-receivers-and-one-shot-hash.md`: the decision (`fn hash(self) -> Int` rather than the spec's streaming `fn hash<H: Hasher>(self, h: H)`); why (a streaming hasher must accumulate into a field, which is awkward through a parameter even with field assignment, and adds a `Hasher` protocol that buys a hash map nothing); the migration note (changing to streaming later would change `Hash`'s only method and therefore every impl, so this is a commitment, not a stopgap); and that `Hash for Float` is deliberately absent because NaN never equals itself — a NaN key would be unreachable — and `0.0`/`-0.0` hash differently unless normalized.

- [ ] **Step 8: Verify and commit**

```bash
cargo test --workspace --no-fail-fast && cargo clippy --all-targets --all-features -- -D warnings && cargo fmt --check
git add -A && git commit -m "feat(std): one-shot Hash and nova_rt_str_hash (ADR 0005)

trait Hash { fn hash(self) -> Int }, with the splitmix64 finalizer for Int,
Bool and Char, and FNV-1a over the bytes for String. Identity hashing would
cluster badly against Map's power-of-two masks, so a known-good mixer is used
rather than an invented one.

Hash is one-shot rather than the spec's streaming Hasher protocol: a streaming
hasher must accumulate into a field, which buys a hash map nothing. String
hashing needs the runtime because Nova cannot walk a string's bytes; it is
reached through an std-scoped builtin, so \`str_hash\` is not reserved in user
code. No Hash for Float — a NaN key would be unreachable.

Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>"
```

---

### Task 7: `Map<K, V>`

**Why:** The main payload of the slice. Open addressing reuses the array machinery directly and avoids a per-entry allocation.

**Files:**
- Modify: `std/collections/lib.nova`
- Test: inline `mod tests` in `crates/nova-typeck/src/check.rs`; e2e in Task 9

**Interfaces:**
- Consumes: `Hash` (Task 6), `Eq` (from `std/core`), `[init; n]` (Task 3), field assignment (Task 1).
- Produces: `Map<K, V>` with `new`, `len`, `insert(mut self, k: K, v: V) -> Option<V>`, `get(self, k: K) -> Option<V>`, `contains_key(self, k: K) -> Bool`, `remove(mut self, k: K) -> Option<V>`. Task 8 wraps this.

- [ ] **Step 1: Write the failing test**

```rust
    #[test]
    fn map_methods_typecheck() {
        let r = check_src(
            "fn main() {\n\
                 let mut m = Map::new()\n\
                 let prev = m.insert(1, \"one\")\n\
                 println(\"${m.len()} ${m.contains_key(1)}\")\n\
                 match m.get(1) { Some(s) => println(s), None => println(\"none\") }\n\
                 match m.remove(1) { Some(s) => println(s), None => println(\"none\") }\n\
                 println(\"${m.len()}\")\n\
             }",
        );
        assert!(r.diagnostics.is_empty(), "{:?}", r.diagnostics);
    }
```

Also add the bound-enforcement test, since `Map`'s whole key contract is `K: Hash + Eq`:

```rust
    #[test]
    fn map_key_without_hash_reports_e0013() {
        // A record that implements neither Hash nor Eq cannot be a Map key.
        // The bound is enforced at monomorphization by the existing machinery.
        let r = check_src(
            "record Unhashable { v: Int }\n\
             fn main() {\n\
                 let mut m = Map::new()\n\
                 let k = Unhashable { v: 1 }\n\
                 let p = m.insert(k, 1)\n\
                 println(\"${m.len()}\")\n\
             }",
        );
        assert!(error_codes(&r).contains(&"E0013"), "{:?}", r.diagnostics);
    }
```

- [ ] **Step 2: Run them and watch them fail**

```bash
cargo test -p nova-typeck --lib -- map_methods_typecheck map_key_without_hash
```

Expected: both FAIL — no `Map`. Note `map_key_without_hash_reports_e0013` will initially fail for the wrong reason (`Map` is unresolved rather than the bound being violated), so **re-check it after Step 3** to confirm it then fails-or-passes for the *bound* reason. If it reports `E0013` only because `check_src` runs monomorphization, confirm that is the same path a real `nova check` takes.

- [ ] **Step 3: Implement `Map` in Nova**

Append to `std/collections/lib.nova`. States: `0` empty, `1` occupied, `2` tombstone.

```nova
/// A hash map with open addressing and linear probing.
///
/// Capacity is a power of two, so a bucket index is `hash & (cap - 1)`. The
/// three arrays are allocated together on first insert, using the inserted key
/// and value as fillers, so no slot is ever uninitialized. `state` is filled
/// with 0, which is exactly the "empty" tag, so a fresh table is empty by
/// construction.
pub record Map<K, V> { len: Int, used: Int, keys: [K], vals: [V], state: [Int] }

impl<K: Hash + Eq, V> Map<K, V> {
    pub fn new() -> Map<K, V> {
        Map { len: 0, used: 0, keys: [], vals: [], state: [] }
    }

    /// Number of live entries; unaffected by tombstones.
    pub fn len(self) -> Int { self.len }

    pub fn is_empty(self) -> Bool { self.len == 0 }

    /// Bucket for `k` in a table of capacity `cap` (a power of two).
    fn slot_of(self, k: K, cap: Int) -> Int { k.hash() & (cap - 1) }

    /// Insert, returning the previous value for `k` if there was one.
    pub fn insert(mut self, k: K, v: V) -> Option<V> {
        if self.state.len() == 0 {
            self.keys = [k; 8]
            self.vals = [v; 8]
            self.state = [0; 8]
        } else {
            // Grow above 3/4 load. `used` counts occupied plus tombstones, so a
            // remove-heavy workload cannot degrade into an all-tombstone scan.
            if (self.used + 1) * 4 > self.state.len() * 3 { self.grow(k, v) }
        }
        let cap = self.state.len()
        let mut i = self.slot_of(k, cap)
        let mut first_free = -1
        let mut probes = 0
        while probes < cap {
            let st = self.state[i]
            if st == 0 {
                let at = if first_free >= 0 { first_free } else { i }
                self.keys[at] = k
                self.vals[at] = v
                self.state[at] = 1
                self.len = self.len + 1
                if first_free < 0 { self.used = self.used + 1 }
                return None
            }
            if st == 1 {
                if self.keys[i].eq(k) {
                    let old = self.vals[i]
                    self.vals[i] = v
                    return Some(old)
                }
            } else {
                if first_free < 0 { first_free = i }
            }
            i = (i + 1) & (cap - 1)
            probes = probes + 1
        }
        panic("Map::insert found no free slot")
    }

    pub fn get(self, k: K) -> Option<V> {
        let cap = self.state.len()
        if cap == 0 { return None }
        let mut i = self.slot_of(k, cap)
        let mut probes = 0
        while probes < cap {
            let st = self.state[i]
            if st == 0 { return None }
            if st == 1 {
                if self.keys[i].eq(k) { return Some(self.vals[i]) }
            }
            i = (i + 1) & (cap - 1)
            probes = probes + 1
        }
        None
    }

    pub fn contains_key(self, k: K) -> Bool {
        match self.get(k) { Some(_) => true, None => false }
    }

    /// Remove `k`, leaving a tombstone so later probes still reach entries
    /// that were inserted past this slot.
    pub fn remove(mut self, k: K) -> Option<V> {
        let cap = self.state.len()
        if cap == 0 { return None }
        let mut i = self.slot_of(k, cap)
        let mut probes = 0
        while probes < cap {
            let st = self.state[i]
            if st == 0 { return None }
            if st == 1 {
                if self.keys[i].eq(k) {
                    let old = self.vals[i]
                    self.state[i] = 2
                    self.len = self.len - 1
                    return Some(old)
                }
            }
            i = (i + 1) & (cap - 1)
            probes = probes + 1
        }
        None
    }

    /// Double the capacity and reinsert live entries, which also clears
    /// tombstones. `fk`/`fv` are fillers for the fresh arrays.
    fn grow(mut self, fk: K, fv: V) {
        let oldcap = self.state.len()
        let newcap = oldcap * 2
        let oldkeys = self.keys
        let oldvals = self.vals
        let oldstate = self.state
        self.keys = [fk; newcap]
        self.vals = [fv; newcap]
        self.state = [0; newcap]
        self.len = 0
        self.used = 0
        for j in 0..oldcap {
            if oldstate[j] == 1 {
                let ignored = self.insert(oldkeys[j], oldvals[j])
            }
        }
    }
}
```

Four things to check rather than assume, reporting what you find:

1. **`return` inside a `while` inside a method** — confirm early `return` works in this position; if not, restructure with a result local rather than changing the algorithm.
2. **`self.keys[i] = k`** is an index-assign whose base is a *field*. `place_root` walks field chains, so `mut self` should satisfy it — verify.
3. **`grow` calls `insert`**, which itself checks the load factor. After `grow` resets `used` to 0 the reinserts cannot re-trigger growth, but confirm there is no recursion, and that `grow` is reached only from `insert`'s pre-check.
4. **`let ignored = …`** — if Nova warns on or rejects an unused binding, or allows a bare expression statement, use whichever form the rest of `std` uses.

- [ ] **Step 4: Run the test and watch it pass**

```bash
cargo test -p nova-typeck --lib -- map_methods_typecheck map_key_without_hash
```

Expected: PASS.

- [ ] **Step 5: Verify the hard behaviors at runtime**

```bash
cargo build -p nova-cli
```

Write a program that forces collisions, reuses a tombstone, and triggers a rehash, then check each printed value by hand:

```bash
printf 'fn main() {\n  let mut m = Map::new()\n  for i in 0..20 { let p = m.insert(i, i * 10) }\n  println("len=${m.len()}")\n  println("${m.get(0).unwrap_or(-1)} ${m.get(19).unwrap_or(-1)} ${m.get(20).unwrap_or(-1)}")\n  let r = m.remove(5)\n  println("removed=${r.unwrap_or(-1)} len=${m.len()} has5=${m.contains_key(5)}")\n  let p2 = m.insert(5, 555)\n  println("reinsert=${m.get(5).unwrap_or(-1)} len=${m.len()}")\n  let old = m.insert(0, 111)\n  println("replaced=${old.unwrap_or(-1)} now=${m.get(0).unwrap_or(-1)} len=${m.len()}")\n}\n' > /d/tmp/map.nova
./target/debug/nova.exe run /d/tmp/map.nova
NOVA_GC_STRESS=1 ./target/debug/nova.exe run /d/tmp/map.nova
```

Expected: `len=20`; `0 190 -1`; `removed=50 len=19 has5=false`; `reinsert=555 len=20`; `replaced=0 now=111 len=20`. Twenty inserts from capacity 8 force at least two rehashes. **The two runs must agree**; if `NOVA_GC_STRESS=1` differs, stop — that is a GC-interaction bug, not a Map bug, and it must be understood rather than worked around.

- [ ] **Step 6: Verify and commit**

```bash
cargo test --workspace --no-fail-fast && cargo clippy --all-targets --all-features -- -D warnings && cargo fmt --check
git add -A && git commit -m "feat(std): Map<K, V> in std/collections

An open-addressed, linearly-probed hash map written in Nova. Capacity is a
power of two so a bucket is \`hash & (cap - 1)\`; the key, value and state
arrays are allocated on first insert with the inserted pair as fillers, and
state's 0 filler is exactly the empty tag.

Tombstones keep probe chains intact across removals and count toward the 3/4
load threshold, so a remove-heavy workload cannot degrade into an
all-tombstone scan; growth doubles and reinserts only live entries, which is
also what clears them.

Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>"
```

---

### Task 8: `Set<T>`

**Why:** Completes the slice. Wrapping `Map` avoids a second copy of the probing logic — the trickiest code here.

**Files:**
- Modify: `std/collections/lib.nova`
- Test: inline `mod tests` in `crates/nova-typeck/src/check.rs`

**Interfaces:**
- Consumes: `Map` (Task 7).
- Produces: `Set<T>` with `new`, `len`, `insert(mut self, v: T) -> Bool` (true if newly added), `contains(self, v: T) -> Bool`, `remove(mut self, v: T) -> Bool`.

- [ ] **Step 1: Write the failing test**

```rust
    #[test]
    fn set_methods_typecheck() {
        let r = check_src(
            "fn main() {\n\
                 let mut s = Set::new()\n\
                 println(\"${s.insert(1)} ${s.insert(1)}\")\n\
                 println(\"${s.len()} ${s.contains(1)} ${s.contains(2)}\")\n\
                 println(\"${s.remove(1)} ${s.len()}\")\n\
             }",
        );
        assert!(r.diagnostics.is_empty(), "{:?}", r.diagnostics);
    }
```

- [ ] **Step 2: Run it and watch it fail**

```bash
cargo test -p nova-typeck --lib -- set_methods_typecheck
```

Expected: FAIL — no `Set`.

- [ ] **Step 3: Implement `Set` in Nova**

Append to `std/collections/lib.nova`:

```nova
/// A hash set, backed by a `Map` with a placeholder value. Reusing `Map` keeps
/// the probing, tombstone and growth logic in exactly one place.
pub record Set<T> { map: Map<T, Bool> }

impl<T: Hash + Eq> Set<T> {
    pub fn new() -> Set<T> { Set { map: Map::new() } }

    pub fn len(self) -> Int { self.map.len() }

    pub fn is_empty(self) -> Bool { self.map.len() == 0 }

    /// Add `v`; returns whether it was newly added.
    pub fn insert(mut self, v: T) -> Bool {
        match self.map.insert(v, true) { Some(_) => false, None => true }
    }

    pub fn contains(self, v: T) -> Bool { self.map.contains_key(v) }

    /// Remove `v`; returns whether it was present.
    pub fn remove(mut self, v: T) -> Bool {
        match self.map.remove(v) { Some(_) => true, None => false }
    }
}
```

Note `self.map.insert(…)` calls a `mut self` method on a **field**, so Task 2's rule must accept a field path rooted at `mut self`. `place_root` walks field chains, so it should — verify, and if it does not, fix `place_root`'s handling rather than dropping `mut` from `Set`.

- [ ] **Step 4: Run the test and watch it pass**

```bash
cargo test -p nova-typeck --lib -- set_methods_typecheck
```

Expected: PASS.

- [ ] **Step 5: Verify and commit**

```bash
cargo build -p nova-cli
printf 'fn main() {\n  let mut s = Set::new()\n  for i in 0..10 { let a = s.insert(i %% 4) }\n  println("len=${s.len()}")\n  println("${s.contains(0)} ${s.contains(3)} ${s.contains(4)}")\n  println("${s.remove(0)} ${s.remove(0)} len=${s.len()}")\n}\n' > /d/tmp/set.nova
./target/debug/nova.exe run /d/tmp/set.nova
cargo test --workspace --no-fail-fast && cargo clippy --all-targets --all-features -- -D warnings && cargo fmt --check
```

Expected: `len=4`; `true true false`; `true false len=3`. Ten inserts of four distinct values exercise dedup.

```bash
git add -A && git commit -m "feat(std): Set<T> in std/collections

A hash set backed by Map<T, Bool>, so the probing, tombstone and growth logic
lives in exactly one place. insert and remove report whether the set changed.

Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>"
```

---

### Task 9: End-to-end gate, GC stress, CHANGELOG

**Why:** The spec's gate: `Vec` and `Map` — including a rehash and tombstone reuse — correct under `nova run`, `nova build`, **and `NOVA_GC_STRESS=1`**. The GC-stress run is the point: growth allocates heavily and the buffer swap is exactly where a conservative non-moving collector would fail.

**Files:**
- Create: `tests/runtime/collections.nova`, `tests/runtime/collections.stdout`
- Modify: `crates/nova-cli/tests/run_tests.rs`, `CHANGELOG.md`

**Interfaces:**
- Consumes: everything from Tasks 1–8.

- [ ] **Step 1: Write the fixture**

Create `tests/runtime/collections.nova`. Cover: `Vec` across several doublings with `pop`/`set`/`clear` and `get` in and out of range; `Map` with **forced collisions** (keys sharing a bucket), tombstone reuse after `remove`, a rehash, and value replacement; a `Map<String, Int>` to exercise the runtime hash path; and `Set` dedup.

```nova
fn main() {
    // Vec across two growth boundaries (4 -> 8 -> 16).
    let mut v = Vec::new()
    for i in 0..10 { v.push(i * i) }
    println("vec len=${v.len()}")
    println("${v.get(0).unwrap_or(-1)} ${v.get(9).unwrap_or(-1)} ${v.get(10).unwrap_or(-1)}")
    v.set(0, 100)
    println("set0=${v.get(0).unwrap_or(-1)}")
    println("pop=${v.pop().unwrap_or(-1)} len=${v.len()}")
    v.clear()
    println("cleared=${v.len()} empty=${v.is_empty()}")

    // Map: rehash, replacement, tombstone reuse.
    let mut m = Map::new()
    for i in 0..20 { let p = m.insert(i, i * 10) }
    println("map len=${m.len()}")
    println("${m.get(0).unwrap_or(-1)} ${m.get(19).unwrap_or(-1)} ${m.get(20).unwrap_or(-1)}")
    let old = m.insert(3, 333)
    println("replaced=${old.unwrap_or(-1)} now=${m.get(3).unwrap_or(-1)} len=${m.len()}")
    println("removed=${m.remove(7).unwrap_or(-1)} has7=${m.contains_key(7)} len=${m.len()}")
    let re = m.insert(7, 777)
    println("tombstone reuse=${m.get(7).unwrap_or(-1)} len=${m.len()}")

    // Forced collisions: with a power-of-two capacity, keys differing by the
    // capacity land in the same bucket, so these exercise probe chains.
    let mut c = Map::new()
    let a1 = c.insert(0, 1)
    let a2 = c.insert(8, 2)
    let a3 = c.insert(16, 3)
    println("collide ${c.get(0).unwrap_or(-1)} ${c.get(8).unwrap_or(-1)} ${c.get(16).unwrap_or(-1)} len=${c.len()}")
    println("mid removal=${c.remove(8).unwrap_or(-1)} still16=${c.get(16).unwrap_or(-1)}")

    // String keys go through the runtime hash.
    let mut sm = Map::new()
    let s1 = sm.insert("alpha", 1)
    let s2 = sm.insert("beta", 2)
    println("str ${sm.get("alpha").unwrap_or(-1)} ${sm.get("beta").unwrap_or(-1)} ${sm.get("gamma").unwrap_or(-1)}")

    // Set dedup.
    let mut s = Set::new()
    for i in 0..10 { let a = s.insert(i % 4) }
    println("set len=${s.len()} has3=${s.contains(3)} has4=${s.contains(4)}")
    println("remove=${s.remove(0)} again=${s.remove(0)} len=${s.len()}")
}
```

The `mid removal` line is the important one: removing a key from the middle of a probe chain must leave a tombstone so `16` is still reachable. A naive "mark empty" implementation prints `-1` there and passes every other line.

- [ ] **Step 2: Generate and hand-verify the expected output**

```bash
cargo build -p nova-cli
./target/debug/nova.exe run tests/runtime/collections.nova
```

Write the output to `tests/runtime/collections.stdout`. **Then check every line by hand against the Nova source** — a fixture captured from a run agrees with a buggy implementation just as happily. In particular confirm `still16=3` (tombstone correctness) and `map len=20`.

If any line is wrong, fix the implementation — **only a mistaken expectation may be edited**.

- [ ] **Step 3: Add the three e2e tests**

Add to `crates/nova-cli/tests/run_tests.rs`, following the `std_core_run` / `std_core_build_standalone` / `std_core_under_gc_stress` trio as the model:

```rust
#[test]
fn collections_run() {
    let expected = std::fs::read_to_string(repo_root().join("tests/runtime/collections.stdout"))
        .expect("expected-output fixture exists")
        .replace("\r\n", "\n");
    nova()
        .arg("run")
        .arg(repo_root().join("tests/runtime/collections.nova"))
        .assert()
        .success()
        .stdout(expected);
}

#[test]
fn collections_build_standalone() {
    let expected = std::fs::read_to_string(repo_root().join("tests/runtime/collections.stdout"))
        .expect("expected-output fixture exists")
        .replace("\r\n", "\n");
    let out = build_and_run("tests/runtime/collections.nova", "collections");
    assert_eq!(out.replace("\r\n", "\n"), expected);
}

#[test]
fn collections_under_gc_stress() {
    let expected = std::fs::read_to_string(repo_root().join("tests/runtime/collections.stdout"))
        .expect("expected-output fixture exists")
        .replace("\r\n", "\n");
    nova()
        .env("NOVA_GC_STRESS", "1")
        .arg("run")
        .arg(repo_root().join("tests/runtime/collections.nova"))
        .assert()
        .success()
        .stdout(expected);
}
```

- [ ] **Step 4: Run them**

```bash
cargo test -p nova-cli --test run_tests -- collections
```

Expected: all three PASS. The GC-stress case is the one that matters most.

- [ ] **Step 5: Confirm unused collections still cost nothing**

`std/collections` is compiled into every program, so monomorphization must keep pruning it. There is an existing test (`std_core_types_used_without_methods_emit_no_symbols` in `crates/nova-mir/tests/lower_tests.rs`) asserting a program that uses no `std/core` methods emits only `main`. Extend or mirror it so a program that touches no collection emits no `Vec`/`Map`/`Set` functions either:

```bash
cargo test -p nova-mir -- emit_no_symbols
```

Expected: PASS. If it fails, unreachable collection methods are being emitted and every Nova binary just grew.

- [ ] **Step 6: Update the CHANGELOG**

Add to `CHANGELOG.md`'s `[Unreleased]` section, matching the existing entry style (read the Phase 2.1 entries first). Cover: field assignment (`rec.f = v`) and that it is alias-visible; the mutable-receiver rule with `E0060` and a pointer to ADR 0005; the `[init; n]` repeat-array literal and why a caller-supplied filler avoids uninitialized memory; a second embedded std module; `Hash` (one-shot, with the `Float` omission and its reason); `Vec`, `Map`, `Set` with their capabilities; and what is deferred and why — `iter()`/`for x in coll` (needs `Iterator` plus associated types, and pairs additionally need tuples), `Queue`/`Deque`, `Vec::with_capacity`, `Hash for Float`, `std/strings`.

- [ ] **Step 7: Final verification**

```bash
cargo test --workspace --no-fail-fast && cargo clippy --all-targets --all-features -- -D warnings && cargo fmt --check
```

Expected: all green.

- [ ] **Step 8: Commit**

```bash
git add -A && git commit -m "test(std): end-to-end gate for std/collections

Vec across two growth boundaries, Map through a rehash, value replacement and
tombstone reuse, forced probe-chain collisions, String keys through the runtime
hash, and Set dedup — under nova run, nova build, and NOVA_GC_STRESS=1.

The stress run is the point: growth allocates heavily and the array swap is
exactly where a conservative non-moving collector would fail.

Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>"
```

---

## After the plan

Run the adversarial-review workflow over the whole increment, per the established project loop, then fix confirmed findings. Do **not** push; the user pushes explicitly.
