# Finishing `Iterator` Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make `Iterator` usable — `for x in v.iter()`, plus lazy `map`/`filter` adapters and `collect`/`fold`/`count`/`any` — which first requires a bound on a record's type parameter to resolve projections in its field types.

**Architecture:** Four layers, in dependency order. A bound on a record parameter becomes a *resolution scope* (not a constraint), which is what lets `record MapIter<I: Iterator, U> { f: fn(I::Item) -> U }` name a projection at all. `check_for`'s existing non-range `E0900` arm is replaced by a `while true` + `next()` + `match` desugar. Two adapter records with `impl Iterator` land in `std/core`, and four consumers become default methods on the trait.

**Tech Stack:** Rust (nova-typeck, nova-ast), Nova (`std/core`), insta snapshots, `assert_cmd` e2e fixtures.

**Spec:** `docs/superpowers/specs/2026-07-29-iterator-finishing-design.md`. Read §1.1's probe table before starting — it records nine measured capabilities, two of which corrected the design.

**Base:** `main` at `9bc13f2`. Create branch `iterator-finishing`.

## Global Constraints

Every task's requirements implicitly include this section.

- **`cargo build --workspace` BEFORE `cargo test`.** `cargo test` does not regenerate `nova-runtime`'s staticlib, so ~25 `*_build_standalone` tests fail with a bogus MSVC unresolved-symbol error otherwise. Measured; CI shares the hole.
- **`--no-fail-fast` is mandatory** on `cargo test --workspace`. Without it cargo abandons later test targets on the first failure and under-reports.
- **A zero-match `cargo test <filter>` EXITS 0.** Before treating a filtered run as evidence, check the `running N tests` line is non-zero. Two filters in the previous plan matched zero tests for *completed* tasks, so their "run it and confirm it fails" steps proved nothing.
- **`cargo test --workspace` rebuilds `nova.exe`**; `cargo test -p <crate>` does not. So after reverting a mutation, run `cargo build --workspace` before any `nova check`/`nova run` probe. This cost two agents real findings on the previous branch, in both directions.
- **Baseline: 604 tests passing, 0 failed; clippy `-D warnings` and `cargo fmt --check` clean; 12 gate configurations green** (4 fixtures × run/build/GC-stress).
- **Nova has no `loop`.** `loop { … }` parses as an identifier followed by a record literal (`P0001: expected '}' (in record literal)`). Use `while true`.
- **`(self.f)(x)` is `E0014`.** A closure stored in a record field must be bound to a local first: `let g = self.f` then `g(x)`.
- **`Ty::Error` behaves oppositely at two consumers.** At `unify` it absorbs, so `assert!(diagnostics.is_empty())` can be vacuously true. At the impl signature comparison it is plain `PartialEq` with no absorption, so an `Error` on one side *forces* a mismatch.
- **A monomorphization-seam test is only as good as the `MirTy` classes it instantiates at.** `mir_ty` (`crates/nova-mir/src/lib.rs:445-452`) maps `Int` *and* `Char` to `MirTy::I64`, and `String`/`Fn`/`Sum`/`Record`/`Array` to `MirTy::Ptr` = `i64` on x86-64. **Only `Bool` and `Float` discriminate.** An `Int`/`String` pair proves nothing there.
- **Nova language limits:** no tuples; no references; `for` only over integer ranges (until Task 2); `///` does not parse — use `//`; `String + String` is `E0013`; `break`/`return` followed by a newline then an expression parses that expression as the value; an empty `Vec::new()` needs a type annotation; `Option<Int>` has no `Display` so cannot be interpolated — use `unwrap`/`is_some`/`is_none`.
- **Diagnostic codes in use:** `E0001`, `E0002`, `E0010`–`E0016`, `E0020`–`E0022`, `E0060`, `E0070`–`E0081`, `E0403`, `E0428`, `E0601`, `E0900`, `E0902`. `E0082` onward is free; say why if you add one.
- **Do NOT push.** Commit only — the repo owner pushes explicitly. Merges are fast-forward only; history is strictly linear (216 commits, 0 merge commits).
- End every commit message body with:
  ```
  Co-Authored-By: Claude Opus 5 <noreply@anthropic.com>
  ```

## File Structure

| File | Responsibility | Tasks |
|---|---|---|
| `crates/nova-typeck/src/check.rs` | record-bound resolution (`collect_records`), the `for` desugar (`check_for`), and all new `#[test]`s | 1, 2 |
| `std/core/lib.nova` | `MapIter`, `FilterIter`, their impls, and the four consumers as default methods on `Iterator` | 3, 4 |
| `crates/nova-cli/tests/run_tests.rs` | three registrations for the new fixture | 5 |
| `tests/runtime/iterator.{nova,stdout}` | the gate fixture | 5 |
| `docs/adr/0007-record-parameter-bounds.md` | why a record bound resolves but does not constrain | 5 |
| `nova-spec/20-STDLIB.md:93-104` | `Iterator`'s block gains the shipped default methods | 5 |
| `CHANGELOG.md` | the increment, including that a record bound is not enforced | 5 |

No new files in `crates/`. `hir::RecordType` is deliberately **not** modified — see Task 1 Step 5.

---

### Task 1: A bound on a record's type parameter resolves projections in its field types

**Files:**
- Modify: `crates/nova-typeck/src/check.rs:457` (drop the rejection), `:462-466` (pass real bounds)
- Test: `crates/nova-typeck/src/check.rs` test module

**Interfaces:**
- Consumes: nothing.
- Produces: `record M<I: Iterator, U> { it: I, f: fn(I::Item) -> U }` type-checks. Tasks 3 and 4 depend on this and on nothing else in this task.

- [ ] **Step 1: Write the failing tests**

```rust
    #[test]
    fn a_record_field_may_name_a_projection_on_a_bounded_parameter() {
        // The blocker this whole increment exists to remove. Without the bound
        // resolving here, a lazy `map` adapter cannot be written at all: its
        // field must be typed `fn(I::Item) -> U`.
        let r = check_src(
            "trait It { type Item\n fn next(mut self) -> Option<Self::Item> }\n\
             record M<I: It, U> { it: I, f: fn(I::Item) -> U }\n\
             fn main() { }",
        );
        assert_eq!(error_codes(&r), Vec::<&str>::new(), "{:?}", r.diagnostics);
    }

    #[test]
    fn a_record_bound_naming_an_unknown_trait_is_e0001() {
        // Resolution must report, not skip. A silently-dropped bound would put
        // this increment straight back into the "accepted and quietly ignored"
        // family the spec's §3.2 warns about.
        let r = check_src(
            "record M<I: NoSuchTrait> { it: I }\n\
             fn main() { }",
        );
        let d = r
            .diagnostics
            .iter()
            .find(|d| d.code == "E0001")
            .expect("E0001 for an unresolvable record bound");
        assert!(
            d.message.contains("NoSuchTrait"),
            "names the trait: {}",
            d.message
        );
    }

    #[test]
    fn a_projection_on_an_unbounded_record_parameter_is_still_e0001() {
        // The bound is what makes the projection resolvable, so without one the
        // old error must remain. This is the guard against "resolve projections
        // against every trait in scope", which would accept nonsense.
        let r = check_src(
            "trait It { type Item\n fn next(mut self) -> Option<Self::Item> }\n\
             record M<I, U> { it: I, f: fn(I::Item) -> U }\n\
             fn main() { }",
        );
        assert!(
            r.diagnostics.iter().any(|d| d.code == "E0001"),
            "an unbounded parameter has no `Item`: {:?}",
            r.diagnostics
        );
    }

    #[test]
    fn a_bound_on_a_sum_type_parameter_is_still_e0900() {
        // Records only. Nothing in this increment needs a bound on a sum
        // parameter, and leaving the rejection in place halves the surface.
        let r = check_src(
            "trait It { type Item\n fn next(mut self) -> Option<Self::Item> }\n\
             type S<I: It> = | A(I) | B\n\
             fn main() { }",
        );
        let d = r
            .diagnostics
            .iter()
            .find(|d| d.code == "E0900")
            .expect("E0900 still rejects a bound on a sum type parameter");
        assert!(d.message.contains("sum type"), "{}", d.message);
    }

    #[test]
    fn a_record_bound_is_not_enforced_at_construction() {
        // The spec's §3.2 decision, pinned so it cannot drift silently in
        // either direction. `Int` is not an `It`, and building `M<Int, …>` is
        // accepted: the bound is a resolution scope, not a constraint. Safety
        // comes from the impl instead — see the E0014 test below.
        let r = check_src(
            "trait It { type Item\n fn next(mut self) -> Option<Self::Item> }\n\
             record M<I: It> { it: I }\n\
             fn main() { let m = M { it: 3 }\n let _ = m }",
        );
        assert_eq!(error_codes(&r), Vec::<&str>::new(), "{:?}", r.diagnostics);
    }
```

- [ ] **Step 2: Run them and confirm which fail**

Run: `cargo test -p nova-typeck --no-fail-fast record_bound record_field_may_name a_projection_on_an_unbounded a_bound_on_a_sum`

**Check the `running N tests` line is non-zero** before reading the result — a zero-match run exits 0.

Expected: `a_record_field_may_name_a_projection_on_a_bounded_parameter` FAILS with `E0900` **and** `E0001` (both fire today — the rejection does not stop conversion). `a_record_bound_naming_an_unknown_trait_is_e0001` FAILS (today it is `E0900`, not `E0001`). `a_record_bound_is_not_enforced_at_construction` FAILS with `E0900`. The other two PASS already — they are guards, and their passing now is what proves they are guards rather than new behaviour.

- [ ] **Step 3: Remove the rejection for records only**

At `crates/nova-typeck/src/check.rs:457`, delete this line:

```rust
            self.reject_type_param_bounds(&decl.generics, "record type parameters");
```

Leave the call at `:517` (`"sum type parameters"`) untouched. Do **not** delete `reject_type_param_bounds` itself — the sum-type caller still needs it.

- [ ] **Step 4: Resolve the bounds and pass them to `convert_ty`**

`collect_records` currently builds only a generic scope and passes `&[]` for bounds. Replace the field-conversion block at `:459-468` so it resolves the bounds first, using the two existing helpers — `resolve_bounds` (`:2115`, `&[ast::TypeParam] -> Vec<Vec<DefId>>`) and `expand_bounds` (`:632`, folds in supertraits):

```rust
            let generics = generic_scope(&decl.generics);
            // A bound on a record's type parameter is a RESOLUTION SCOPE, not a
            // constraint: it exists so a field type may name a projection on
            // that parameter (`f: fn(I::Item) -> U`), which is what makes a
            // lazy iterator adapter expressible. It is deliberately NOT checked
            // at construction — `MakeRecord` carries no type arguments, and
            // monomorphization visits only instances reachable from `main`, so
            // enforcement would fire *sometimes*, which is worse than not at
            // all (the Phase 2.2a assessment; ADR 0007 records it). Safety comes
            // from the impl: `impl<I: Iterator, U> Iterator for MapIter<I, U>`
            // requires the bound, so a `MapIter<Int, U>` simply has no
            // `Iterator` impl and is inert.
            let mut bounds = self.resolve_bounds(&decl.generics);
            self.expand_bounds(&mut bounds);
            let fields = decl
                .fields
                .iter()
                .map(|f| hir::RecordField {
                    name: f.name.value.clone(),
                    ty: self.convert_ty(&f.ty, &generics, &bounds),
                })
                .collect();
```

`resolve_bounds` is what reports `E0001` for an unknown trait, which is why the second test passes once this lands.

- [ ] **Step 5: Confirm `hir::RecordType` needs no change**

Read `crates/nova-hir/src/lib.rs:870-876`. It stores `generics: u32` — a count, not a list — and no consumer needs the bounds, because the bound's entire job finishes during field-type conversion above. **Do not add a field.** Not adding one is what stops this leaking into MIR. Note in your report that you checked.

- [ ] **Step 6: Run the tests**

Run: `cargo test -p nova-typeck --no-fail-fast record_bound record_field_may_name a_projection_on_an_unbounded a_bound_on_a_sum`, then `cargo build --workspace && cargo test --workspace --no-fail-fast`

Expected: all five PASS; workspace 604 + 5 = 609, 0 failed. If any pre-existing test changes, that is a finding — report it rather than adjusting the test.

- [ ] **Step 7: Prove the guard tests are load-bearing**

Apply this mutation: in Step 4's code, replace `&bounds` with `&[]`. Run `cargo test -p nova-typeck --no-fail-fast record_field_may_name`. Expected: FAILS. Revert, then **`cargo build --workspace`** before any further probing.

Then apply a second mutation: pass `&bounds` but skip `expand_bounds`. Write a supertrait case (`trait Sub: It`, `record M<I: Sub> { f: fn(I::Item) -> Int }`) and confirm it fails without expansion and passes with it. If it passes either way, `expand_bounds` is not load-bearing here and you should say so rather than leave a call nothing exercises.

- [ ] **Step 8: Commit**

```bash
git add crates/nova-typeck/src/check.rs
git commit -m "feat(typeck): a record parameter's bound resolves projections in field types"
```

---

### Task 2: `for x in it`

**Files:**
- Modify: `crates/nova-typeck/src/check.rs:3878-3892` (`check_for`'s non-range arm)
- Test: `crates/nova-typeck/src/check.rs` test module

**Interfaces:**
- Consumes: nothing from Task 1 (independent; ordered second only because Task 5's fixture needs both).
- Produces: `for x in <expr implementing Iterator> { … }`. `x` binds at the normalized `Self::Item`.

- [ ] **Step 1: Write the failing tests**

```rust
    #[test]
    fn a_for_loop_iterates_an_iterator() {
        let r = check_src(
            "trait It { type Item\n fn next(mut self) -> Option<Self::Item> }\n\
             record Once { v: Int, done: Bool }\n\
             impl It for Once { type Item = Int\n\
              fn next(mut self) -> Option<Int> { if self.done { None } else { self.done = true\n Some(self.v) } } }\n\
             fn main() { for x in Once { v: 7, done: false } { println(\"${x}\") } }",
        );
        assert_eq!(error_codes(&r), Vec::<&str>::new(), "{:?}", r.diagnostics);
    }

    #[test]
    fn a_for_loop_over_a_range_still_works() {
        // This task edits the function the range loop lives in, so the range
        // path needs its own assertion here rather than relying on the older
        // range tests being run.
        let r = check_src("fn main() { for i in 0..3 { println(\"${i}\") } }");
        assert_eq!(error_codes(&r), Vec::<&str>::new(), "{:?}", r.diagnostics);
    }

    #[test]
    fn a_for_loops_variable_binds_at_the_projected_item_type() {
        // `x` must be the normalized `Self::Item` (here `Bool`), not the
        // projection and not an inference variable. Bool rather than Int
        // deliberately: `mir_ty` collapses Int and Char to the same machine
        // type, so a wrong item type among them is invisible downstream.
        let r = check_src(
            "trait It { type Item\n fn next(mut self) -> Option<Self::Item> }\n\
             record OnceB { v: Bool, done: Bool }\n\
             impl It for OnceB { type Item = Bool\n\
              fn next(mut self) -> Option<Bool> { if self.done { None } else { self.done = true\n Some(self.v) } } }\n\
             fn main() { for x in OnceB { v: true, done: false } { let y: Int = x\n let _ = y } }",
        );
        let d = r
            .diagnostics
            .iter()
            .find(|d| d.code == "E0010")
            .expect("E0010: the loop variable is Bool, not Int");
        assert!(d.message.contains("Bool"), "{}", d.message);
    }

    #[test]
    fn a_for_loops_variable_is_immutable() {
        // Same rule the range loop already enforces. Without it the desugar
        // could hand out a mutable binding by accident.
        let r = check_src(
            "trait It { type Item\n fn next(mut self) -> Option<Self::Item> }\n\
             record Once { v: Int, done: Bool }\n\
             impl It for Once { type Item = Int\n\
              fn next(mut self) -> Option<Int> { if self.done { None } else { self.done = true\n Some(self.v) } } }\n\
             fn main() { for x in Once { v: 7, done: false } { x = 1 } }",
        );
        assert!(
            r.diagnostics.iter().any(|d| d.code == "E0060"),
            "{:?}",
            r.diagnostics
        );
    }

    #[test]
    fn a_for_loop_over_a_non_iterator_names_both_accepted_forms() {
        // The existing message says "anything but an integer range", which
        // becomes false the moment this task lands. `for x in v` is the mistake
        // people will actually make, so the text must mention `.iter()`.
        let r = check_src("fn main() { for x in 3 { println(\"${x}\") } }");
        let d = r
            .diagnostics
            .iter()
            .find(|d| d.code == "E0900")
            .expect("E0900 for a non-range non-iterator");
        assert!(
            d.message.contains("iter()"),
            "points at the fix: {}",
            d.message
        );
    }
```

- [ ] **Step 2: Run them and confirm they fail**

Run: `cargo test -p nova-typeck --no-fail-fast for_loop`

**Check `running N tests` is non-zero.** Expected: `a_for_loop_over_a_range_still_works` PASSES (guard). The other four FAIL — the three iterator ones with the current `E0900` ("anything but an integer range"), and the message one because today's text has no `iter()`.

- [ ] **Step 3: Replace the non-range arm**

`check_for` at `:3878` opens by destructuring a range and bailing otherwise. Replace that `else` block (`:3886-3892`) with a call to a new `check_for_iterator`, keeping the range path exactly as it is:

```rust
        let ast::Expr::Range { lo, hi, inclusive } = &iter.value else {
            return self.check_for_iterator(fcx, pattern, iter, body, span);
        };
```

- [ ] **Step 4: Implement `check_for_iterator`**

Add it immediately after `check_for`. It builds the same shape as the range desugar — a hidden unscoped local plus a `while` — so read the range path first and mirror its hygiene rather than inventing new conventions:

```rust
    /// `for x in it { body }` desugars to
    /// `{ let __it = it; while true { match __it.next() { Some(x) => body,
    ///    None => break } } }`.
    ///
    /// `while true`, not `loop`: Nova has no `loop` keyword — `loop { … }`
    /// parses as an identifier followed by a record literal.
    ///
    /// `__it` is unscoped (so it can neither collide with nor shadow a source
    /// identifier) and `mut` (so `next`'s `mut self` receiver is satisfied
    /// without the user writing `mut`, per ADR 0005 §1). The user's `x` stays
    /// immutable, exactly as in the range form, so assigning it is `E0060`.
    ///
    /// The `Iterator` bound is discharged at monomorphization (`E0013`) like
    /// every other bound, not here.
    fn check_for_iterator(
        &mut self,
        fcx: &mut FnCtx,
        pattern: &Spanned<ast::Pattern>,
        iter: &Spanned<ast::Expr>,
        body: &Spanned<ast::Block>,
        span: Span,
    ) -> hir::Expr {
        // …
    }
```

The body must: check `iter`; resolve its `Iterator` impl to get the normalized `Item` (reuse the same path `emit_trait_call` uses — do **not** write a second impl-selection routine, since head-only selection has shipped as a miscompile in this codebase twice); create `__it` via `fcx.new_local_unscoped("__it".to_string(), iter_ty, true, span)`; push a scope; bind the pattern's identifier at the item type as immutable; and emit the `while true` + `match` structure.

If the iterable's type has no `Iterator` impl, emit the reworded `E0900`:

```rust
        self.unsupported(
            iter.span,
            "`for` loops over anything but an integer range (`a..b`) or a value \
             implementing `Iterator` (try `.iter()`)",
        );
```

Note that `unsupported` appends "are not supported yet", so phrase the `what` to read as a plural clause.

- [ ] **Step 5: Run the tests**

Run: `cargo test -p nova-typeck --no-fail-fast for_loop`, then `cargo build --workspace && cargo test --workspace --no-fail-fast`

Expected: all five PASS; 609 + 5 = 614, 0 failed.

- [ ] **Step 6: Prove the desugar end to end, not just in typeck**

Write `$TMP/forit.nova` using `std`'s real `Vec`, and run it:

```nova
fn main() { let mut v = Vec::new()
 v.push(1)
 v.push(2)
 for x in v.iter() { println("${x}") }
 for i in 0..2 { println("range ${i}") } }
```

Run `cargo build --workspace` first, then `target/debug/nova.exe run <path>`. Expected output `1 2 range 0 range 1`, each on its own line. A typeck-only test cannot catch a lowering or codegen fault in the new `while`/`match` structure.

- [ ] **Step 7: Mutation check**

Apply each and confirm something fails, reverting and running `cargo build --workspace` between them: (a) make `__it` non-`mut` — expect `E0060`; (b) make the loop variable `mut` — expect `a_for_loops_variable_is_immutable` to fail; (c) drop the `None => break` arm — expect a non-exhaustive-match or a hang, and if it hangs, note that the fixture in Task 5 must be run under `timeout`.

- [ ] **Step 8: Commit**

```bash
git add crates/nova-typeck/src/check.rs
git commit -m "feat(typeck): for x in it, over any Iterator"
```

---

### Task 3: `MapIter` and `FilterIter`

**Files:**
- Modify: `std/core/lib.nova` (after `trait Iterator` at `:166-169`)
- Test: `crates/nova-cli/tests/run_tests.rs`

**Interfaces:**
- Consumes: Task 1 (the record bound is what makes `fn(I::Item) -> U` legal).
- Produces: `record MapIter<I: Iterator, U> { it: I, f: fn(I::Item) -> U }` with `type Item = U`; `record FilterIter<I: Iterator> { it: I, keep: fn(I::Item) -> Bool }` with `type Item = I::Item`. Task 4 adds `map`/`filter` to the trait, returning these.

- [ ] **Step 1: Write the failing test**

In `crates/nova-cli/tests/run_tests.rs`:

```rust
#[test]
fn iterator_adapters_chain_and_are_lazy() {
    // Constructed directly rather than via `.map()`/`.filter()`, which Task 4
    // adds — so this task is testable on its own. Bool and Float instances are
    // deliberate: `mir_ty` collapses Int, Char, String and every heap type to
    // one machine type, so an Int-only chain cannot see a wrong item type.
    let src = "fn main() {\n\
      let mut v = Vec::new()\n\
      v.push(1)\n\
      v.push(2)\n\
      v.push(3)\n\
      let mut m = MapIter { it: v.iter(), f: |n| n * 10 }\n\
      for x in m { println(\"${x}\") }\n\
      let mut f = FilterIter { it: v.iter(), keep: |n| n > 1 }\n\
      for x in f { println(\"keep ${x}\") }\n\
      let mut c = MapIter { it: FilterIter { it: v.iter(), keep: |n| n > 1 }, f: |n| n > 2 }\n\
      for b in c { println(\"chain ${b}\") }\n\
    }";
    // chain yields Bool: filter keeps 2,3 then map gives false,true
    assert_runs_with(src, "10\n20\n30\nkeep 2\nkeep 3\nchain false\nchain true\n");
}
```

If `assert_runs_with` does not exist, read how a neighbouring test in that file drives `nova run` against inline source and follow it exactly; do not invent a second harness.

- [ ] **Step 2: Run it and confirm it fails**

Run: `cargo build --workspace && cargo test -p nova-cli --no-fail-fast iterator_adapters`

**Check `running N tests` is non-zero.** Expected: FAIL — `MapIter` is not defined.

- [ ] **Step 3: Add the two records and impls to `std/core/lib.nova`**

Insert after `trait Iterator`'s closing brace (`:169`):

```nova
// A lazy `map`: yields `f(x)` for each `x` the inner iterator yields.
//
// `f: fn(I::Item) -> U` is why `I` carries a bound. A bound on a record's type
// parameter is a resolution scope, not a constraint (ADR 0007): it lets this
// field name `I::Item` at all. It is not checked when a `MapIter` is built —
// `impl<I: Iterator, U> Iterator for MapIter<I, U>` below is what makes a
// `MapIter` iterable, so a `MapIter<Int, U>` merely has no impl and is inert.
//
// Holds `it` by pointer, like every record, so it SHARES the source iterator
// rather than copying it — advancing this adapter advances the source, and
// mutating the source mid-iteration is observable. Same alias visibility
// `VecIter` documents; preventing it needs borrow tracking Nova lacks.
pub record MapIter<I: Iterator, U> { it: I, f: fn(I::Item) -> U }

impl<I: Iterator, U> Iterator for MapIter<I, U> {
    type Item = U
    fn next(mut self) -> Option<U> {
        let n = self.it.next()
        if n.is_none() { None } else {
            // Bound to a local first: `(self.f)(x)` is `E0014: no method f`.
            let g = self.f
            Some(g(n.unwrap()))
        }
    }
}

// A lazy `filter`: yields only the elements `keep` accepts.
//
// `type Item = I::Item` is a projection-valued binding — the associated type is
// itself a projection — which is the `Assoc { on: Assoc }` shape Phase 2.2c
// could only reach from a hand-built unit test. This is its first appearance in
// real source.
pub record FilterIter<I: Iterator> { it: I, keep: fn(I::Item) -> Bool }

impl<I: Iterator> Iterator for FilterIter<I> {
    type Item = I::Item
    fn next(mut self) -> Option<I::Item> {
        // `while true`, not `loop`: Nova has no `loop` keyword.
        while true {
            let n = self.it.next()
            if n.is_none() { return None }
            let p = self.keep
            if p(n.unwrap()) { return Some(n.unwrap()) }
        }
        None
    }
}
```

Two things to verify rather than assume as you write this. First, whether `n.unwrap()` may be called twice — if `Option` is consumed by `unwrap`, bind it once (`let x = n.unwrap()`) and reuse. Second, whether the trailing `None` after `while true` is reachable in the checker's view; if it reports unreachable code, restructure rather than suppress.

- [ ] **Step 4: Run the test**

Run: `cargo build --workspace && cargo test -p nova-cli --no-fail-fast iterator_adapters`, then `cargo test --workspace --no-fail-fast`

Expected: PASS; 614 + 1 = 615, 0 failed. **All four existing gate fixtures must stay byte-identical** — you changed `std/core`, which every program compiles. If one moves, report it; do not update the `.stdout`.

- [ ] **Step 5: Mutation check**

Apply each, revert, and `cargo build --workspace` between: (a) `FilterIter::next` returns `Some` unconditionally — expect the `keep` assertions to fail; (b) `MapIter::next` returns the inner item unmapped — expect the `10 20 30` line to fail; (c) `FilterIter`'s `type Item = I::Item` changed to a concrete `Int` — expect the `chain false/true` case to fail, since that chain's item is `Bool`. If (c) survives, the chain is not exercising the projection and needs a better case.

- [ ] **Step 6: Commit**

```bash
git add std/core/lib.nova crates/nova-cli/tests/run_tests.rs
git commit -m "feat(std): lazy MapIter and FilterIter adapters"
```

---

### Task 4: `map`, `filter`, `collect`, `fold`, `count`, `any`

**Files:**
- Modify: `std/core/lib.nova` (`trait Iterator`'s body, `:166-169`)
- Test: `crates/nova-cli/tests/run_tests.rs`

**Interfaces:**
- Consumes: Task 3's `MapIter`/`FilterIter`.
- Produces: `map`, `filter`, `collect`, `fold`, `count`, `any` as default methods on `Iterator`. Task 5's fixture uses all six.

- [ ] **Step 1: Write the failing test**

```rust
#[test]
fn iterator_default_methods_work_and_chain() {
    let src = "fn main() {\n\
      let mut v = Vec::new()\n\
      v.push(1)\n\
      v.push(2)\n\
      v.push(3)\n\
      let got = v.iter().filter(|n| n > 1).map(|n| n * 10).collect()\n\
      println(\"${got.len()}\")\n\
      println(\"${got.get(0).unwrap()}\")\n\
      println(\"${v.iter().fold(0, |a, x| a + x)}\")\n\
      println(\"${v.iter().count()}\")\n\
      println(\"${v.iter().any(|n| n > 2)}\")\n\
      println(\"${v.iter().any(|n| n > 9)}\")\n\
      println(\"${v.iter().map(|n| n > 2).any(|b| b)}\")\n\
    }";
    assert_runs_with(src, "2\n20\n6\n3\ntrue\nfalse\ntrue\n");
}
```

The last line matters: it chains `map` to `Bool` then consumes with `any`, so the consumer sees a **`Bool`** item rather than `Int` — the only way this test can see a wrong item type at the monomorphization seam.

- [ ] **Step 2: Run it and confirm it fails**

Run: `cargo build --workspace && cargo test -p nova-cli --no-fail-fast iterator_default_methods`

**Check `running N tests` is non-zero.** Expected: FAIL — no method `filter`.

- [ ] **Step 3: Add the six default methods**

Replace `trait Iterator`'s body at `std/core/lib.nova:166-169`:

```nova
pub trait Iterator {
    type Item
    fn next(mut self) -> Option<Self::Item>

    // --- adapters (lazy: nothing is consumed until a consumer runs) ---

    fn map<U>(self, f: fn(Self::Item) -> U) -> MapIter<Self, U> {
        MapIter { it: self, f: f }
    }

    fn filter(self, keep: fn(Self::Item) -> Bool) -> FilterIter<Self> {
        FilterIter { it: self, keep: keep }
    }

    // --- consumers (each drives the iterator to exhaustion or short-circuits) ---

    // The fold primitive. `count` and `collect` could be written over it;
    // `any` cannot, because `fold` visits every element and `any` must stop.
    fn fold<A>(mut self, init: A, f: fn(A, Self::Item) -> A) -> A {
        let mut acc = init
        while true {
            let n = self.next()
            if n.is_none() { return acc }
            let g = f
            acc = g(acc, n.unwrap())
        }
        acc
    }

    // `count`, not `len`: this consumes the iterator, where `Vec::len` is cheap
    // and non-destructive. Naming it `len` would invite the wrong assumption.
    fn count(mut self) -> Int {
        let mut n = 0
        while true {
            let x = self.next()
            if x.is_none() { return n }
            n = n + 1
        }
        n
    }

    // Short-circuits, which is why it is not written over `fold`.
    fn any(mut self, p: fn(Self::Item) -> Bool) -> Bool {
        while true {
            let n = self.next()
            if n.is_none() { return false }
            let q = p
            if q(n.unwrap()) { return true }
        }
        false
    }

    // Returns a `Vec`, which makes `std/core` depend on `std/collections` for
    // the first time. Accepted deliberately: one method, one type, and the
    // whole-program merge means there is no layering mechanism to violate, only
    // a convention. The alternative was `Vec::from_iter(it)` in
    // `std/collections`, which keeps `std/core` free of collections and reads
    // worse at the call site.
    fn collect(mut self) -> Vec<Self::Item> {
        let mut out = Vec::new()
        while true {
            let n = self.next()
            if n.is_none() { return out }
            out.push(n.unwrap())
        }
        out
    }
}
```

Two things to check rather than assume. `Vec::new()` inside `collect` may need its element type inferred from the first `push` — if it reports `E0011`, an annotation is needed and the comment should say why. And `out.push` requires a mutable receiver, so `out` must be `mut`; confirm `mut` on a `let` inside a trait default method behaves as it does in a free function.

- [ ] **Step 4: Run the test**

Run: `cargo build --workspace && cargo test -p nova-cli --no-fail-fast iterator_default_methods`, then `cargo test --workspace --no-fail-fast`

Expected: PASS; 615 + 1 = 616, 0 failed, four existing fixtures byte-identical.

- [ ] **Step 5: Prove laziness is real, not nominal**

Laziness is the property this design was chosen for, and none of the tests above can see it — a lazy and an eager `map` produce identical output. Write a program whose source iterator has an observable side effect per `next` (print inside a hand-written iterator's `next`), build a `map` over it, and **do not consume it**. Expected: nothing printed. Then consume it and expect the prints interleaved with the mapped output rather than all up front. Add this as a test; without it, `map` could be silently eager and every other assertion would still pass.

- [ ] **Step 6: Mutation check**

Apply each, revert, `cargo build --workspace` between: (a) `any` returns `false` unconditionally — expect the `true` lines to fail; (b) `any` written as a `fold` that visits everything — expect the laziness/short-circuit test to fail; (c) `count` returns `0` — expect the count line; (d) `collect` returns an empty `Vec` — expect the length line.

- [ ] **Step 7: Commit**

```bash
git add std/core/lib.nova crates/nova-cli/tests/run_tests.rs
git commit -m "feat(std): map, filter, collect, fold, count, any on Iterator"
```

---

### Task 5: Gate fixture, ADR 0007, spec and CHANGELOG

**Files:**
- Create: `tests/runtime/iterator.nova`, `tests/runtime/iterator.stdout`, `docs/adr/0007-record-parameter-bounds.md`
- Modify: `crates/nova-cli/tests/run_tests.rs` (three registrations), `nova-spec/20-STDLIB.md:93-104`, `CHANGELOG.md`

**Interfaces:**
- Consumes: Tasks 1–4.
- Produces: `iterator_run`, `iterator_build_standalone`, `iterator_under_gc_stress`.

- [ ] **Step 1: Write the fixture**

`tests/runtime/iterator.nova` must cover, in this order, with a printed line per item so a wrong value is visible rather than merely a wrong count:

1. `for` over a `Vec` via `.iter()`.
2. `for` over an integer range — **still working**, since Task 2 edited that function.
3. Exhaustion: iterate to the end, then confirm a further `next()` is `None`.
4. An empty source (`let mut e: Vec<Int> = Vec::new()`) whose **first** `next()` is already `None`.
5. A two-stage chain `.filter(…).map(…)`, proving adapter-on-adapter.
6. All six methods from Task 4.
7. **Every generic block instantiated at `Bool` or at `Float`.** `mir_ty` maps `Int` and `Char` to `MirTy::I64` and every heap type to `MirTy::Ptr` = `i64`, so **only `Bool` and `Float` have distinguishable machine classes** — an `Int`-only fixture cannot see a wrong item type at the monomorphization seam. This is the single most important line in this task.

- [ ] **Step 2: Generate the `.stdout` from real output, then read it**

```bash
cargo build --workspace
target/debug/nova.exe run tests/runtime/iterator.nova > tests/runtime/iterator.stdout
```

Then **open the file and check every line says what you intended.** Generating it from output makes the test tautological unless a human reads it. Note the harness normalises CRLF via `.replace("\r\n", "\n")`; if you compare by hand, use `tr -d '\r'`.

- [ ] **Step 3: Register the three configurations**

Copy the `assoc_types_run` / `assoc_types_build_standalone` / `assoc_types_under_gc_stress` trio at `crates/nova-cli/tests/run_tests.rs:2604` onward, substituting `iterator` for `assoc_types`. The GC-stress one sets `.env("NOVA_GC_STRESS", "1")`. Read all three before copying — the build variant compares captured stdout rather than asserting on the command.

- [ ] **Step 4: Run all fifteen gate configurations**

Run: `cargo build --workspace && cargo test --workspace --no-fail-fast`

Expected: 616 + 3 = 619, 0 failed; **fifteen** gate configurations green (5 fixtures × 3), with the twelve pre-existing ones byte-identical.

- [ ] **Step 5: Prove the fixture discriminates**

For at least three of the seven fixture items, apply a mutation that should break it and confirm the fixture fails. A fixture line that passes under a broken compiler is worse than no line, because it reads as coverage. State which three you checked and what each caught.

- [ ] **Step 6: Write ADR 0007**

`docs/adr/0007-record-parameter-bounds.md`, in the style of ADR 0005 and 0006 (read one first). It must record:

- **Status:** Accepted, 2026-07-29, Phase 2.2d.
- **Context:** a lazy adapter needs `fn(I::Item) -> U` in a record field; in a record declaration `I` has no bound, so the projection is `E0001`; adding the bound was `E0900`, rejected since Phase 2.2a. The `A`-parameter workaround type-checks but cannot be driven, because nothing ties `A` to `I::Item` and Nova has no equality constraints.
- **Decision:** a bound on a record's type parameter is a **resolution scope**, not a constraint. Records only; sum types keep `E0900`.
- **Why not enforced:** `MakeRecord` carries no type arguments — the instantiation survives only in the enclosing `Expr.ty`, which lowering discards, and MIR erases records to `Ptr`. Monomorphization visits only instances reachable from `main`, so enforcement would fire *sometimes*, which is subtler than not firing at all. This is Phase 2.2a's assessment, re-affirmed.
- **Why that is safe:** correctness comes from the impl. `MapIter<Int, U>` has no `Iterator` impl, so it constructs and is inert; `.next()` on it is an ordinary `E0014`.
- **Consequences, stated plainly:** a record bound looks like a constraint and is not one. Name this as the risk, and name the family it belongs to — impl-level `const`s discarded, record bounds, record field visibility, `pub` on methods — all "accepted and quietly ignored" defects this project has fixed. State that the mitigation is documentation in three places rather than code, and that a future increment may replace it with real enforcement if `MakeRecord` ever carries type arguments.
- **Alternatives considered:** eager `map`/`filter` returning `Vec` (no projection needed, allocates per stage); rejecting a bound on a parameter no field type uses (declined — inert, and a second analysis for no user benefit); threading type args through `MakeRecord` (much larger, and 2.2a's objection stands).

- [ ] **Step 7: Update `nova-spec` and the CHANGELOG**

In `nova-spec/20-STDLIB.md`, `Iterator`'s block at `:93-104` lists `// default methods: map, filter, collect, fold, ...` as a comment. Replace that with the six methods as shipped, with their real signatures, and delete the comment. Keep the existing `::` and `mut self` notes.

In `CHANGELOG.md`, under the existing `### Added (Phase 2 …)`, add the increment. It must state:

- `for x in it` over any `Iterator`, and that `for x in v` is **not** supported — write `v.iter()`, because there is no `IntoIterator`.
- The six methods, with `map`/`filter` lazy and `collect` returning `Vec<Self::Item>`.
- That `collect` makes `std/core` depend on `std/collections`, and why that was accepted.
- **A bound on a record's type parameter now resolves projections in field types and is NOT enforced at construction** — with a pointer to ADR 0007. Do not bury this; it is the one thing a reader could reasonably be surprised by.
- Adapters share their source by pointer, so mutating a source mid-iteration is observable.

Do **not** state a count of anything. Counts in this project's documents have gone stale in three consecutive review rounds; describe behaviour instead.

- [ ] **Step 8: Full verification and commit**

```bash
cargo build --workspace
cargo test --workspace --no-fail-fast
cargo clippy --all-targets --all-features -- -D warnings
cargo fmt --check
```

All four clean; 619 tests, 0 failed; fifteen gate configurations green.

```bash
git add tests/runtime/iterator.nova tests/runtime/iterator.stdout \
        crates/nova-cli/tests/run_tests.rs docs/adr/0007-record-parameter-bounds.md \
        nova-spec/20-STDLIB.md CHANGELOG.md
git commit -m "test(gate): iterator fixture; docs: ADR 0007, spec, CHANGELOG"
```

---

## Plan Self-Review

**Spec coverage** — every section maps to a task:

| Spec section | Task |
|---|---|
| §1 / §1.1 the blocker and the probe table | context for all; §1.1's `loop` and `(self.f)(x)` findings are Global Constraints |
| §2 scope (in/out) | 1 (records only, not sums), 3–4 (the six methods), 5 (no `IntoIterator`) |
| §3.1 the change is small | 1 Steps 3–5 |
| §3.2 resolution scope, not a constraint | 1 Step 1's fifth test + Step 4's comment; 5 Step 6 (ADR) |
| §4 the `for` desugar | 2 |
| §5 adapters | 3 |
| §5 consumers | 4 |
| §6 diagnostics | 2 Step 1 (reworded `E0900`), 1 Step 1 (`E0001`), 5 Step 6 (the `E0014` decision) |
| §7 gate, items 1–6 | 5 Step 1 |
| §7 the `#[test]`s a fixture cannot hold | 1 and 2's test blocks |
| §8 risks 1–4 | 1 Step 4's comment, 5 Step 6 (ADR consequences), 2 Step 1's range guard, 4 Step 3's `collect` comment |
| §9 definition of done | 5 Step 8 |

**Placeholder scan:** the two `// …` markers in Task 2 Step 4 and Task 3 Step 3 are deliberate — each is followed by prose stating exactly what the body must do and which existing routine to reuse. Every other code block is complete. No "TBD", no "handle edge cases", no "similar to Task N".

**Type consistency:** `MapIter<I, U> { it, f }` and `FilterIter<I> { it, keep }` are spelled identically in Tasks 3, 4 and 5. `MapIter::Item = U`, `FilterIter::Item = I::Item` throughout. `check_for_iterator`'s signature in Task 2 Step 4 matches `check_for`'s at `:3878`. `resolve_bounds` returns `Vec<Vec<DefId>>` and `expand_bounds` takes `&mut [Vec<DefId>]`, matching `:2115` and `:632`.

**Two gaps found and fixed while reviewing:** Task 4 had no test that laziness is real — a lazy and an eager `map` produce identical output, so every assertion would pass either way; Step 5 now requires a side-effecting source. And Task 1's `expand_bounds` call had no test exercising it, so Step 7 now requires a supertrait case or an explicit report that the call is inert.
