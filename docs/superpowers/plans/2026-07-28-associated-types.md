# Associated Types Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add associated types to Nova's trait system (`type Item` in a trait, `Self::Item` / `I::Item` in type position), enforce `mut self` on trait methods, and land `Iterator` plus one generic implementor as the consumer that proves it.

**Architecture:** A new `Ty::Assoc { on, assoc }` projection, resolved by *normalization at three seams* rather than by deferred obligations — because `nova-typeck`'s unifier is a 210-line Robinson engine whose entire state is `vars: Vec<Option<Ty>>`, with no impl table and no constraint queue. Projections are normalized wherever the impl table *is* in scope: `check.rs` after `apply`, `check_impl_conformance`, and `mono.rs` after `subst`. This mirrors how trait *bounds* are already discharged at monomorphization rather than in `check_src`.

**Tech Stack:** Rust (nova-ast, nova-parser, nova-resolver, nova-hir, nova-typeck, nova-mir), Nova (std/core, std/collections), `cargo test` + committed `tests/runtime` stdout fixtures.

**Spec:** `docs/superpowers/specs/2026-07-28-associated-types-design.md`. **Read §5.1 before writing any code** — it pins six cases that each have a defensible opposite.

## Two deliberate refinements to the spec

Both found by reading actual signatures. Implement the plan, not the spec, where they differ:

1. **`Ty::Assoc { on: Box<Ty>, assoc: DefId }`, not `{ on, trait_id, index }`.** The spec wanted `index: u32` and said `display_ty` would look the name up in the trait's list — but `display_ty` (`crates/nova-typeck/src/lib.rs:35`) takes only `defs: &Definitions`, so it cannot see the trait table. Giving the associated type its own `DefId` (a new `DefKind::AssocType`, exactly as trait methods already get one) makes `display_ty` work unchanged via `defs.def(*assoc).name`, removes the index, and keeps `Ty` free of a `String`.
2. **`mir_ty` stays defensive; the loud check moves to mono.** The spec said `mir_ty` must be an explicit unreachable. But `mir_ty` (`crates/nova-mir/src/lib.rs:443`) already maps `Param`/`Var`/`Error` to `MirTy::Unit` *defensively*, and a panic there would violate the repo's no-panic-on-reachable-paths convention. So `Assoc` joins that defensive arm, and Task 7 adds the loud detection in mono where a **diagnostic** is possible. The spec's intent — a projection must never silently reach codegen — is preserved and better served.

## Global Constraints

- Run every `cargo` command from `D:\Projects\nona\nova`.
- Must end green on all three: `cargo test --workspace --no-fail-fast`, `cargo clippy --all-targets --all-features -- -D warnings`, `cargo fmt --check`. **`--no-fail-fast` is mandatory** — without it cargo abandons later targets and under-reports.
- **After editing `crates/nova-runtime/`, run `cargo build --workspace` BEFORE `cargo test`.** `cargo test` does not regenerate `nova-runtime`'s staticlib, which `nova build` links against, so ~25 `*_build_standalone` tests fail with an MSVC `unresolved external symbol` error that is not a real defect. (No task here should touch the runtime; if one does, this applies.)
- `cargo build -p nova-cli` after any `std/*.nova` edit before probing by hand. **Every program in the test suite compiles all std modules**, so many unrelated failures at once means your Nova source is wrong.
- The three existing gates must keep passing byte-identically: `tests/runtime/collections.nova`, `std_core.nova`, `strings.nova`. `.stdout` fixtures are **CRLF** in the checkout while the compiler emits **LF** — the harness normalises; compare by hand with `tr -d '\r'`, never a raw diff.
- No `unwrap()`/`expect()` in Rust library paths reachable from user input; prefer `.get(..)`. Tests may use them.
- `///` doc comments do **not** parse in Nova source — use `//`. Nova has no tuples, no references, `for` iterates integer ranges only, and **`String + String` is `E0013`**.
- **`break`/`return` followed by a newline then an expression parses that expression as the value.** Keep `break` immediately before a `}`.
- Every test needs its own temp directory (`std::env::temp_dir().join("nova-…")` + `create_dir_all`) — cargo runs tests in parallel threads and ~46 unique names already exist in `run_tests.rs`. **No `tempfile` dependency**; the repo does not use one.
- Conventional commit(s), each ending with exactly: `Co-Authored-By: Claude Opus 5 <noreply@anthropic.com>`
- **Never `git push`.**
- If a pre-existing test changes behaviour, investigate — do not edit it to match without understanding why. (Task 8 is the one deliberate exception, and it is explicit about why.)

## File Structure

| File | Responsibility | Tasks |
|---|---|---|
| `crates/nova-hir/src/lib.rs` | `Ty::Assoc` variant; `subst`, `head`, `match_pattern`, `mentions_param`, `self_types_overlap`; `TraitDef.assoc_types`; `TraitMethod.mut_self`; `ImplInfo.assoc_bindings` | 1, 2, 4, 8 |
| `crates/nova-typeck/src/infer.rs` | `apply`, `occurs`, `unify` arms for `Assoc` | 1 |
| `crates/nova-typeck/src/lib.rs` | `display_ty` renders `<on>::Name` | 1 |
| `crates/nova-typeck/src/check.rs` | `convert_ty` two-segment paths; `collect_traits` assoc list; conformance; the `normalize` helper; `E0060` on the trait path | 2–6, 8 |
| `crates/nova-ast/src/item.rs` | `TraitItem::AssocType` | 2 |
| `crates/nova-parser/src/grammar.rs` | `type Name` / `type Name: B + C` in a trait body | 2 |
| `crates/nova-resolver/src/lib.rs` | `DefKind::AssocType`, one `DefId` per associated type | 2 |
| `crates/nova-mir/src/lib.rs` | `mir_ty` defensive arm | 1 |
| `crates/nova-mir/src/mono.rs` | normalize after `subst`; diagnose a surviving `Assoc` | 7 |
| `std/core/lib.nova` | `pub trait Iterator` | 9 |
| `std/collections/lib.nova` | `VecIter<T>`, `impl<T> Iterator for VecIter<T>`, `Vec::iter` | 9 |
| `tests/runtime/assoc_types.{nova,stdout}` | the gate | 10 |
| `docs/adr/0006-associated-type-syntax.md`, `nova-spec/20-STDLIB.md`, `CHANGELOG.md` | the `::` deviation | 10 |

---

### Task 1: `Ty::Assoc` variant and every exhaustive match over `Ty`

Foundation with **no behaviour change**: add the variant and decide it at all nine sites so the workspace compiles green. Doing this alone first means a reviewer can check "did every match site decide correctly" without that being tangled with resolution logic.

**Files:**
- Modify: `crates/nova-hir/src/lib.rs` (variant, `subst`, `head`, `match_pattern`, `mentions_param`, `self_types_overlap`)
- Modify: `crates/nova-typeck/src/infer.rs` (`apply`, `occurs`, `unify`)
- Modify: `crates/nova-typeck/src/lib.rs` (`display_ty`)
- Modify: `crates/nova-mir/src/lib.rs` (`mir_ty`)

**Interfaces:**
- Consumes: nothing.
- Produces: `Ty::Assoc { on: Box<Ty>, assoc: DefId }`. `Ty::subst` recurses into `on` and does **not** normalize (callers do). `display_ty` renders `<on>::Name`.

- [ ] **Step 1: Write the failing tests**

In `crates/nova-typeck/src/infer.rs`'s `#[cfg(test)] mod tests`:

```rust
    // A projection is opaque to the unifier: two projections match only when
    // they name the same associated type on unifiable Self types. Anything
    // else must already have been normalized away by the caller, which is why
    // there is no Assoc-vs-concrete arm.
    #[test]
    fn assoc_unifies_only_with_the_same_projection() {
        use nova_resolver::DefId;
        let item = DefId(7);
        let other = DefId(8);
        let mut icx = InferCtx::default();
        let a = Ty::Assoc { on: Box::new(Ty::Int), assoc: item };
        let b = Ty::Assoc { on: Box::new(Ty::Int), assoc: item };
        assert!(icx.unify(&a, &b), "same projection on same Self");

        let c = Ty::Assoc { on: Box::new(Ty::Bool), assoc: item };
        assert!(!icx.unify(&a, &c), "same name, different Self");

        let d = Ty::Assoc { on: Box::new(Ty::Int), assoc: other };
        assert!(!icx.unify(&a, &d), "different associated type");

        assert!(!icx.unify(&a, &Ty::Int), "a projection is not Int");
    }

    // The Self type is an ordinary type position, so a variable inside it must
    // solve through unification like any other.
    #[test]
    fn assoc_unification_solves_a_var_inside_the_self_type() {
        use nova_resolver::DefId;
        let item = DefId(7);
        let mut icx = InferCtx::default();
        let v = icx.fresh();
        let a = Ty::Assoc { on: Box::new(v.clone()), assoc: item };
        let b = Ty::Assoc { on: Box::new(Ty::Int), assoc: item };
        assert!(icx.unify(&a, &b));
        assert_eq!(icx.apply(&v), Ty::Int);
    }

    // occurs must see through a projection, or `?0 == ?0::Item` would bind a
    // variable to a type containing itself and `apply` would not terminate.
    #[test]
    fn occurs_looks_inside_a_projection() {
        use nova_resolver::DefId;
        let mut icx = InferCtx::default();
        let v = icx.fresh();
        let proj = Ty::Assoc { on: Box::new(v.clone()), assoc: DefId(7) };
        assert!(!icx.unify(&v, &proj), "occurs check must reject ?0 = ?0::Item");
    }
```

In `crates/nova-hir/src/lib.rs`'s test module:

```rust
    #[test]
    fn subst_recurses_into_a_projection_without_normalizing() {
        use nova_resolver::DefId;
        let proj = Ty::Assoc { on: Box::new(Ty::Param(0)), assoc: DefId(7) };
        // subst has no impl table, so it substitutes and stops. Normalizing is
        // the caller's job (typeck's `normalize`, or mono after subst).
        assert_eq!(
            proj.subst(&[Ty::Int]),
            Ty::Assoc { on: Box::new(Ty::Int), assoc: DefId(7) }
        );
    }

    #[test]
    fn a_projection_has_no_head_and_no_param_of_its_own() {
        use nova_resolver::DefId;
        let proj = Ty::Assoc { on: Box::new(Ty::Param(2)), assoc: DefId(7) };
        // No head: impl lookup cannot key on an unnormalized projection.
        assert!(proj.head().is_none());
        // But it does mention the parameter in its Self type, which
        // `E0073`'s unused-impl-parameter check depends on.
        assert!(proj.mentions_param(2));
        assert!(!proj.mentions_param(0));
    }
```

- [ ] **Step 2: Run them to verify they fail**

Run: `cargo test -p nova-typeck --no-fail-fast assoc` and `cargo test -p nova-hir --no-fail-fast projection`
Expected: FAIL to compile — `Ty` has no variant `Assoc`.

- [ ] **Step 3: Add the variant**

In `crates/nova-hir/src/lib.rs`, after `Ty::Param(u32)`:

```rust
    /// A projection onto a trait's associated type: `<on>::Name`, where
    /// `assoc` is the associated type's own `DefId` (see
    /// `DefKind::AssocType`).
    ///
    /// This is the one `Ty` variant that is **not** a type by itself — it is a
    /// *request* for one, answerable only with the impl table. It is therefore
    /// normalized away at three seams (typeck's `normalize`,
    /// `check_impl_conformance`, and `mono` after `subst`), and the unifier
    /// never has to decide a projection against a concrete type.
    ///
    /// `on` is `Param(k)` inside a generic body, a concrete type at an
    /// ordinary use site, and — provably — never an unsolved `Var`:
    /// `check_method_call` rejects an uninferred receiver with `E0011` before
    /// any return type is computed, and a user-written `I::Item` names a
    /// generic parameter. See the design doc §4.2.
    Assoc { on: Box<Ty>, assoc: DefId },
```

- [ ] **Step 4: Decide it at all nine sites**

`crates/nova-hir/src/lib.rs`:

```rust
// In Ty::subst — substitute into the Self type; do NOT normalize (no impl table here).
            Ty::Assoc { on, assoc } => Ty::Assoc {
                on: Box::new(on.subst(args)),
                assoc: *assoc,
            },

// In Ty::head — a projection has no nominal head until normalized, so impl
// lookup cannot key on it. Falls into the existing `_ => None` arm; add a test
// rather than an arm.

// In Ty::mentions_param — recurse into the Self type.
            Ty::Assoc { on, .. } => on.mentions_param(idx),

// In Ty::match_pattern — match structurally: same associated type, and the
// Self types must match.
            (Ty::Assoc { on: p, assoc: pa }, Ty::Assoc { on: g, assoc: ga }) => {
                pa == ga && p.match_pattern(g, out)
            }

// In self_types_overlap — conservative. A projection's value is unknown until
// normalized, so assume it could coincide with anything unless both sides are
// projections that provably differ. A false E0074 is a loud error; a missed
// overlap is a silent miscompile, so err toward overlap.
            (Ty::Assoc { on: a1, assoc: x }, Ty::Assoc { on: b1, assoc: y }) => {
                x != y || self_types_overlap(a1, a_generics, b1, b_generics)
            }
            (Ty::Assoc { .. }, _) | (_, Ty::Assoc { .. }) => true,
```

`crates/nova-typeck/src/infer.rs`:

```rust
// In apply — recurse into the Self type.
            Ty::Assoc { on, assoc } => Ty::Assoc {
                on: Box::new(self.apply(&on)),
                assoc,
            },

// In occurs — recurse, or `?0 = ?0::Item` would build an infinite type.
            Ty::Assoc { on, .. } => self.occurs(v, &on),

// In unify, placed AFTER the Var/Error/Never arms and beside the other
// structural arms:
            (
                Ty::Assoc { on: o1, assoc: a1 },
                Ty::Assoc { on: o2, assoc: a2 },
            ) => a1 == a2 && self.unify(&o1.clone(), &o2.clone()),
```

`crates/nova-typeck/src/lib.rs`, in `display_ty`:

```rust
        Ty::Assoc { on, assoc } => {
            format!("{}::{}", display_ty(on, defs), defs.def(*assoc).name)
        }
```

`crates/nova-mir/src/lib.rs`, in `mir_ty` — join the existing defensive arm:

```rust
        // Post-typeck these should not occur; map defensively. `Assoc` in
        // particular is normalized away by `mono` before lowering, and mono
        // reports a diagnostic if one survives — so reaching here is a
        // compiler bug, but one that must not panic in a library path.
        hir::Ty::Param(_) | hir::Ty::Var(_) | hir::Ty::Error | hir::Ty::Assoc { .. } => MirTy::Unit,
```

- [ ] **Step 5: Run the tests and the whole suite**

Run: `cargo test -p nova-typeck --no-fail-fast assoc`, `cargo test -p nova-hir --no-fail-fast`, then `cargo test --workspace --no-fail-fast`
Expected: the new tests PASS; **everything else unchanged** — this task adds no behaviour, so any pre-existing test that changes is a mistake in one of the nine arms. Investigate rather than adjust.

- [ ] **Step 6: Confirm no match site was missed**

Run: `cargo clippy --all-targets --all-features -- -D warnings`
Expected: clean. A missed exhaustive match is a compile error, so a green build is the proof. **Then grep for wildcards that may have silently swallowed the new variant** — these are the real risk, because they compile:

```bash
git grep -n '_ =>' -- crates/nova-hir/src/lib.rs crates/nova-typeck/src/infer.rs crates/nova-typeck/src/lib.rs
```

For each hit, decide whether `Assoc` falling into it is correct (as in `head()`) or a silent wrong answer. Report the list and your decision for each in the report.

- [ ] **Step 7: Commit**

```bash
git add crates/nova-hir/src/lib.rs crates/nova-typeck/src/infer.rs crates/nova-typeck/src/lib.rs crates/nova-mir/src/lib.rs
git commit -m "feat(hir): add the Ty::Assoc projection variant"
```

---

### Task 2: Parse `type Item` in a trait, give it a `DefId`, record it in the trait table

**Files:**
- Modify: `crates/nova-ast/src/item.rs` (`TraitItem::AssocType`)
- Modify: `crates/nova-parser/src/grammar.rs` (trait body)
- Modify: `crates/nova-resolver/src/lib.rs` (`DefKind::AssocType`)
- Modify: `crates/nova-hir/src/lib.rs` (`TraitDef.assoc_types`)
- Modify: `crates/nova-typeck/src/check.rs` (`collect_traits`; `E0900` for bounds)

**Interfaces:**
- Consumes: Task 1's `Ty::Assoc`.
- Produces: `ast::TraitItem::AssocType { name: Spanned<String>, bounds: Vec<Spanned<Path>> }`; `DefKind::AssocType { trait_def: DefId }`; `hir::TraitDef.assoc_types: Vec<(String, DefId)>`. Later tasks resolve a name to its `DefId` through `TraitDef.assoc_types`.

- [ ] **Step 1: Write the failing tests**

In `crates/nova-typeck/src/check.rs`'s test module:

```rust
    #[test]
    fn a_trait_records_its_associated_types_in_order() {
        let r = check_src(
            "trait Pair { type A\n type B\n fn get(self) -> Int }\nfn main() { }",
        );
        assert!(r.diagnostics.is_empty(), "{:?}", r.diagnostics);
        let t = r
            .module
            .traits
            .iter()
            .find(|t| t.name == "Pair")
            .expect("trait Pair collected");
        let names: Vec<&str> = t.assoc_types.iter().map(|(n, _)| n.as_str()).collect();
        // Order matters: it is declaration order, and two associated types is
        // the case that catches an implementation assuming there is only one.
        assert_eq!(names, ["A", "B"]);
        // Each gets its own DefId, so `display_ty` can name it.
        assert_ne!(t.assoc_types[0].1, t.assoc_types[1].1);
    }

    #[test]
    fn a_bound_on_an_associated_type_reports_e0900() {
        // Rejected rather than silently dropped — the same rule this project
        // applies to record and sum type-parameter bounds, because a bound
        // that enforces nothing is worse than no bound.
        let r = check_src("trait It { type Item: Display }\nfn main() { }");
        let d = r
            .diagnostics
            .iter()
            .find(|d| d.code == "E0900")
            .expect("E0900 for a bound on an associated type");
        assert!(
            d.message.contains("associated type"),
            "message should name the construct: {}",
            d.message
        );
    }
```

- [ ] **Step 2: Run them to verify they fail**

Run: `cargo test -p nova-typeck --no-fail-fast associated_types`
Expected: FAIL — the parser rejects `type A` inside a trait with `P0001: expected 'fn' (in function signature), found 'type'`.

- [ ] **Step 3: Add the AST variant**

In `crates/nova-ast/src/item.rs`, in `enum TraitItem`:

```rust
    /// An associated type declaration: `type Item`.
    ///
    /// `bounds` is parsed (`type Item: Display`) but **not** supported —
    /// `nova-typeck` reports `E0900` for a non-empty list, the same way a
    /// `where` clause on a trait is parsed here and rejected there. Parsing it
    /// gives a precise span and a real diagnostic instead of a syntax error.
    AssocType {
        name: Spanned<String>,
        bounds: Vec<Spanned<Path>>,
    },
```

- [ ] **Step 4: Parse it**

In `crates/nova-parser/src/grammar.rs`, `parse_trait_decl`'s body loop is at **`:536`**. Traced, so use this rather than searching:

- **`Token::Type` exists** (`crates/nova-lexer/src/lib.rs:576`) — the `type` keyword is already lexed, for type aliases.
- **`self.parse_trait_bounds()` is the reusable bound-list parser** — `parse_trait_decl` already calls it at `:530` for `trait B: A` supertraits. Call it for `type Item: Display`; do not write a second bound parser.
- **The `type` check must come BEFORE `parse_function_sig()`, not after.** That loop opens with `let saved_pos = self.pos; let saved_errors_len = self.errors.len();` and then *speculatively* calls `parse_function_sig()`, rolling back if a body brace follows. Reaching `parse_function_sig()` with `type Item` ahead makes it fail and fall into `sync_to_stmt_boundary()`, producing a spurious parse error even once your arm exists. Guard on `self.check(&Token::Type)` at the top of the loop body and `continue`.

Add an insta snapshot test in `crates/nova-parser/tests/parser_tests.rs` (the repo convention) covering `trait It { type Item  fn next(self) -> Int }` and `trait It { type Item: Display }`.

- [ ] **Step 5: Give each associated type a `DefId`**

In `crates/nova-resolver/src/lib.rs`, add to `DefKind`:

```rust
    /// An associated type declared in a trait (`type Item`). Carries its
    /// owning trait so a projection can be checked against the right trait
    /// without a separate lookup table.
    AssocType { trait_def: DefId },
```

In the trait-collection pass, walk `TraitItem::AssocType` and `push_def` one `Def` per associated type, named after it. Do **not** insert it into the module's type namespace — `Item` is not a type name you can write bare; it is only reachable through a projection. Add a resolver test asserting `resolve_type(module, "Item")` is `None` for a program declaring `trait It { type Item }`, so a later change cannot silently make it a global type name.

- [ ] **Step 6: Record it in the trait table**

In `crates/nova-hir/src/lib.rs`, add to `TraitDef`:

```rust
    /// Associated types this trait declares, in declaration order, each with
    /// its own `DefId` (`DefKind::AssocType`). An impl must bind every one of
    /// them; `check_impl_conformance` enforces that.
    pub assoc_types: Vec<(String, DefId)>,
```

In `check.rs`'s `collect_traits`, fill it from the AST, and report `E0900` for a non-empty `bounds`:

```rust
                ast::TraitItem::AssocType { name, bounds } => {
                    if !bounds.is_empty() {
                        self.unsupported(name.span, "trait bounds on an associated type");
                    }
                    // …record (name.value.clone(), def_id) in assoc_types…
                }
```

Update every `TraitDef { … }` construction site — the compiler will point at them.

- [ ] **Step 7: Run the tests**

Run: `cargo test -p nova-parser --no-fail-fast`, `cargo test -p nova-typeck --no-fail-fast associated_types`, `cargo test -p nova-resolver --no-fail-fast`, then `cargo test --workspace --no-fail-fast`
Expected: all PASS. Accept new insta snapshots with `cargo insta accept` only after reading them.

- [ ] **Step 8: Commit**

```bash
git add crates/nova-ast crates/nova-parser crates/nova-resolver crates/nova-hir crates/nova-typeck
git commit -m "feat(trait): parse and collect associated type declarations"
```

---

### Task 3: `convert_ty` resolves `Self::Item` and `I::Item`

**Files:**
- Modify: `crates/nova-typeck/src/check.rs:1481-1487` (`convert_ty`, the two-segment path case)

**Interfaces:**
- Consumes: Task 1's `Ty::Assoc`, Task 2's `TraitDef.assoc_types`.
- Produces: a two-segment type path whose first segment names a generic parameter (or `Self`) becomes `Ty::Assoc`. Module-qualified paths keep reporting `E0900` unchanged.

- [ ] **Step 1: Write the failing test**

```rust
    #[test]
    fn a_projection_on_a_generic_parameter_resolves() {
        // `I::Item` where I is a generic parameter bounded by a trait that
        // declares `Item`. No impl is needed: the projection stays abstract.
        let r = check_src(
            "trait It { type Item\n fn get(self) -> Int }\n\
             fn first<I: It>(x: I) -> I::Item { panic(\"unreachable\") }\n\
             fn main() { }",
        );
        assert!(r.diagnostics.is_empty(), "{:?}", r.diagnostics);
    }

    #[test]
    fn a_module_qualified_type_path_still_reports_e0900() {
        // The two-segment path case now has a second meaning; the original
        // one must survive with its original message.
        let r = check_src("fn f(x: some_mod::Thing) -> Int { 1 }\nfn main() { }");
        let d = r
            .diagnostics
            .iter()
            .find(|d| d.code == "E0900")
            .expect("E0900 for a module-qualified type path");
        assert!(
            d.message.contains("module-qualified type paths"),
            "original message preserved: {}",
            d.message
        );
    }

    #[test]
    fn a_projection_naming_an_undeclared_associated_type_is_an_error() {
        let r = check_src(
            "trait It { type Item\n fn get(self) -> Int }\n\
             fn f<I: It>(x: I) -> I::Nope { panic(\"unreachable\") }\n\
             fn main() { }",
        );
        assert!(
            !r.diagnostics.is_empty(),
            "`I::Nope` must not silently typecheck"
        );
    }
```

- [ ] **Step 2: Run them to verify they fail**

Run: `cargo test -p nova-typeck --no-fail-fast projection`
Expected: the first and third FAIL with `E0900: module-qualified type paths are not supported yet`; the second already passes and must keep passing.

- [ ] **Step 3: Implement**

Replace the early `path.segments.len() != 1` rejection in `convert_ty` with a two-way split. A two-segment path is a **projection** when its first segment names an in-scope generic parameter or `Self`; otherwise it is a module path and keeps the existing `E0900`.

```rust
                if path.segments.len() == 2 {
                    let base = path.segments[0].value.as_str();
                    // `Self` inside a trait or impl is that scope's own
                    // parameter 0 (see hir::TraitMethod's doc comment).
                    let base_param = if base == "Self" {
                        generics.get("Self").copied()
                    } else {
                        generics.get(base).copied()
                    };
                    if let Some(idx) = base_param {
                        let assoc_name = path.segments[1].value.as_str();
                        // Find the associated type among the traits bounding
                        // this parameter. Searching the bounds (rather than
                        // every trait) is what makes `I::Item` mean "the Item
                        // of the trait I is bounded by".
                        return self.resolve_projection(idx, assoc_name, ty.span);
                    }
                    self.unsupported(ty.span, "module-qualified type paths");
                    return Ty::Error;
                }
                if path.segments.len() != 1 {
                    self.unsupported(ty.span, "module-qualified type paths");
                    return Ty::Error;
                }
```

Add `resolve_projection`, which needs the bounds of parameter `idx` in the current scope.

**This step was the plan's least-specified one; it has since been traced, so use this rather than going looking.** There is exactly **one** bound table and it is uniform across trait methods, impl methods and free functions — the plan's worry about "the wrong table" was unfounded:

- The shape is `Vec<Vec<DefId>>`, indexed by parameter position, i.e. **indexed like `generics`' values**. It appears as the local `bounds` during signature collection and as `FnCtx.param_bounds` (`crates/nova-typeck/src/check.rs:190`) inside a body, populated from `sig.bounds`.
- In `collect_signatures` (`:1218`) the order is already `resolve_bounds` → `apply_where` → `expand_bounds` → **then** `convert_ty`. So bounds are fully resolved *before* any type conversion, and no reordering is needed.
- **`convert_ty` cannot see them today.** Its signature is `fn convert_ty(&mut self, ty, generics: &FxHashMap<String, u32>)` — names only. Add a `bounds: &[Vec<DefId>]` parameter (or bundle the pair into one small struct if that reads better). There are **19** call sites in `check.rs`; exactly **one** passes an empty generic scope, and that one passes an empty slice.
- **`expand_bounds` has already folded supertraits in**, so a bound of `Ord` also carries `Eq`. That means `I::Item` resolves against the *transitive* bound set: if `I: Ord` and `Ord: Eq` and `Eq` declared `Item`, it is found. That is the desirable behaviour, but state it in a comment, because it is a consequence of ordering rather than an explicit decision.

If more than one bounding trait declares the same associated-type name, report an ambiguity error rather than picking the first — with supertraits folded in, this is reachable, not hypothetical. If none does, report that the name is not an associated type of any bound on that parameter.

- [ ] **Step 4: Run the tests**

Run: `cargo test -p nova-typeck --no-fail-fast projection`, then `cargo test --workspace --no-fail-fast`
Expected: all PASS, and the existing `module-qualified type paths` tests unchanged.

- [ ] **Step 5: Commit**

```bash
git add crates/nova-typeck/src/check.rs
git commit -m "feat(typeck): resolve Self::Item and I::Item to a projection"
```

---

### Task 4: Impls bind associated types, and conformance checks the set

**Files:**
- Modify: `crates/nova-hir/src/lib.rs` (`ImplInfo.assoc_bindings`)
- Modify: `crates/nova-typeck/src/check.rs` (impl collection; `check_impl_conformance`)

**Interfaces:**
- Consumes: Tasks 1–3.
- Produces: `ImplInfo.assoc_bindings: Vec<(DefId, Ty)>` — the associated type's `DefId` mapped to the bound type, with the impl's own `Param(k)` still in it. Task 5's `normalize` reads this.

- [ ] **Step 1: Write the failing tests**

```rust
    #[test]
    fn an_impl_binds_its_associated_type() {
        let r = check_src(
            "trait It { type Item\n fn get(self) -> Int }\n\
             record W<T> { v: T }\n\
             impl<T> It for W<T> { type Item = T\n fn get(self) -> Int { 1 } }\n\
             fn main() { }",
        );
        assert!(r.diagnostics.is_empty(), "{:?}", r.diagnostics);
        let i = r
            .module
            .impls
            .iter()
            .find(|i| i.trait_id.is_some())
            .expect("the trait impl was collected");
        // Bound to the impl's OWN parameter, which is what makes `subst` the
        // thing that carries it — a binding to a primitive would not.
        assert_eq!(i.assoc_bindings.len(), 1);
        assert_eq!(i.assoc_bindings[0].1, Ty::Param(0));
    }

    #[test]
    fn an_impl_missing_an_associated_type_reports_e0070() {
        let r = check_src(
            "trait It { type Item\n fn get(self) -> Int }\n\
             record W { v: Int }\n\
             impl It for W { fn get(self) -> Int { 1 } }\n\
             fn main() { }",
        );
        let d = r
            .diagnostics
            .iter()
            .find(|d| d.code == "E0070")
            .expect("E0070 for a missing associated type");
        assert!(d.message.contains("Item"), "names the missing type: {}", d.message);
    }

    #[test]
    fn an_impl_binding_an_undeclared_associated_type_reports_e0071() {
        let r = check_src(
            "trait It { type Item\n fn get(self) -> Int }\n\
             record W { v: Int }\n\
             impl It for W { type Item = Int\n type Extra = Bool\n fn get(self) -> Int { 1 } }\n\
             fn main() { }",
        );
        let d = r
            .diagnostics
            .iter()
            .find(|d| d.code == "E0071")
            .expect("E0071 for an undeclared associated type");
        assert!(d.message.contains("Extra"), "names it: {}", d.message);
    }
```

**On the two codes — this corrects an earlier version of this plan, which said E0072 for both directions.** That was wrong in both. `check_impl_conformance` already runs a three-code scheme for methods, and associated types must join it rather than invent a fourth meaning:

| code | means | existing site |
|---|---|---|
| `E0070` | the impl is **missing** something the trait requires | `check.rs:1242`, `"impl of trait \`{}\` is missing method(s): {}"` |
| `E0071` | the impl provides something **not a member** of the trait | `check.rs:1088`, `"method \`{name}\` is not a member of trait \`{}\`"` |
| `E0072` | the item exists on both sides but its **shape disagrees** (arity, param type, return type, generic count, bounds) | six sites, `check.rs:1113`–`1221` |

A missing binding is the E0070 case and an undeclared one is the E0071 case. E0072 is *not* free for either: Task 6 needs it for the shape case that genuinely arises — an impl whose method signature disagrees with the trait's *after* the trait's projection is normalized through this impl's binding.

Prefer extending the **existing** `E0070` missing-list and `E0071` not-a-member sites over adding parallel ones, if the surrounding code makes that natural — one diagnostic listing every missing item reads better than two. Use "associated type(s)" in the wording either way, so the message never claims a missing `Item` is a missing *method*.

- [ ] **Step 2: Run them to verify they fail**

Run: `cargo test -p nova-typeck --no-fail-fast assoc_binding`
Expected: FAIL — `type Item = T` inside an `impl` does not parse yet.

- [ ] **Step 3: Parse `type Name = Type` in an impl body**

The impl body loop is `parse_impl_block` at **`crates/nova-parser/src/grammar.rs:592`**, and its fallthrough error is `expected: "fn or const inside impl"` at `:634`. Traced against `HEAD` after Task 2 landed, so use this rather than searching — but re-confirm, since Task 3 and this task's own Step 3 both shift these:

- **`ast::ImplBlock` has PARALLEL VECS, not a unified item list** — `functions: Vec<Function>` and `consts: Vec<ConstDecl>`, the last two fields of the struct at `crates/nova-ast/src/item.rs:140`. So an associated-type binding is a **third vector**, `assoc_types: Vec<AssocTypeBinding>`, with its own small struct `{ name: Spanned<String>, ty: Spanned<Type> }` — **not** a variant in a shared enum. The plan's earlier "put it beside the impl's items" was vague; this is what it means concretely.
- The body loop `match self.peek()`es on `Token::Fn | Token::Async` and `Token::Const`, with a `_` arm that emits the error above. Add a `Token::Type` arm beside them. Unlike the trait-body loop, there is **no speculative parse to sequence around** here — the `match` dispatches on one token, so arm order does not matter.
- `parse_visibility()` runs before the `match`, so a `pub type Item = …` would parse and then be silently ignored. Decide it deliberately: an impl's associated-type binding has no meaningful visibility of its own, so reject a non-private `vis` with a diagnostic rather than dropping it.

Add an insta snapshot test.

- [ ] **Step 3b: Fix the zero-progress recovery loop in the same body — it is an infinite loop today**

While adding the `Token::Type` arm, fix the bug that made me find it. **`impl W { type Item = T }` currently HANGS the compiler** (`nova check` never terminates — measured, exit 124 on a 12-second timeout), and so does any other item-start token inside an impl body: `record`, `trait`, `impl`, `import` all reproduce it.

Root cause, confirmed: `sync_to_item_boundary` (`crates/nova-parser/src/grammar.rs:138-156`) `break`s at any of ten item-start tokens — `Fn`, `Pub`, `Record`, `Trait`, `Impl`, `Type`, `Const`, `Import`, `Module`, `Extern` — **without consuming one**. The impl-body loop's `_` arm (`:611-618`) pushes its error and then calls it, so the loop re-peeks the identical token and repeats forever. Non-item tokens like `42` or `let` recover fine, because `sync_to_item_boundary` consumes those.

Adding a `Token::Type` arm alone would fix only the instance I happened to trip over and leave four others hanging. **Guarantee progress instead**: the `_` arm must consume at least one token before or after syncing, so a token that is both unexpected *and* an item boundary cannot be re-peeked. Verify all five shapes above now terminate with a diagnostic, and add a `#[test]` for at least two of them — a hang is untestable by assertion, so the test's value is that it *completes*.

This is **pre-existing**, not introduced by this branch: `Token::Type` has always been an item-start token (top-level type aliases), and the `_` arm has always called `sync_to_item_boundary`. It may well share a root cause with the separately-queued parser hang from the `std/strings` phase (a keyword used as an impl method name plus a following generic trait method) — if your fix makes that repro terminate too, say so in the report, because that would let the queued task be closed.

- [ ] **Step 4: Record and check the bindings**

Add to `hir::ImplInfo`:

```rust
    /// Associated types this impl binds, keyed by the associated type's own
    /// `DefId`. The bound type may contain the impl's `Param(k)`, so
    /// normalization must substitute the impl's arguments before using it.
    pub assoc_bindings: Vec<(DefId, Ty)>,
```

In `check_impl_conformance` (`crates/nova-typeck/src/check.rs:1074`), after the existing method loop, compare the set the trait declares against the set the impl binds and report a diagnostic for each difference, naming the type — **`E0070` for a missing binding, `E0071` for an undeclared one**, per the table in Step 1. Both directions matter: a missing binding means every projection through this impl is unresolvable, and an extra one is a typo the user wants told about.

Note the shape of the existing missing-method check (`:1235-1249`): it filters `tr.methods` by `default_def.is_none() && !provided.contains(name)`. Associated types have **no defaults** in this increment, so every declared one is required — the filter is simply "not provided", with no default to exempt. If you later add defaults, this is the line that must learn about them.

- [ ] **Step 4b: Make `Self::Item` resolve inside an impl — Task 6 cannot start without it**

Task 3's review found that Task 6's own Step 1 test is currently **unreachable**, and that Task 3's `resolve_projection` cannot express what it needs. This step closes both. It lands here because this is the impl task; leaving it to Task 6 would mean discovering it mid-task.

The problem, verified: `impl R { fn h(self) -> Self::Item { … } }` reports `error[E0900]: module-qualified type paths are not supported yet` — unchanged from before the branch. `Self` is not special-cased anywhere; it is an ordinary entry in the `generics` map, inserted **only** by `self_generic_scope()` (`crates/nova-typeck/src/check.rs:5442`), which trait paths use and impls do not.

`resolve_projection` (`:1701`) is closer to reusable than an earlier draft of this step claimed — **correcting that draft**: it already takes `bounds: &[DefId]`, i.e. an explicit candidate-trait list that its *caller* looked up by index. It does **not** index a bounds table itself. The one thing that blocks reuse is that it takes `idx: u32` and hardcodes `on: Box::new(Ty::Param(idx))`, which an impl's `Self` cannot be: for `impl<T> Tr for W<T>` it is the compound `W<Param(0)>`, with no parameter index at all.

So the generalization is **one parameter, not a restructure**: change `idx: u32` to `on: Ty` and use it directly in the `[assoc]` arm. `base: &str` stays as-is — it is only used to render the two error messages, and the impl caller passes `"Self"`. The existing call site becomes `self.resolve_projection(Ty::Param(idx), base, assoc_name, &bounds[idx as usize], span)` or equivalent; confirm against the real code, since Task 3's fix round moved these lines.

Then add the impl-scope caller, passing the impl's self type and — for a trait impl — the trait it implements as the single candidate. Decide and state in a comment what an **inherent** impl (`impl W { … }`, no trait) should do with `Self::Item`: it has no trait, so there is no associated type to find. Note that `resolve_projection`'s existing `[]` arm already produces a reasonable diagnostic for an empty candidate list (`no associated type \`Item\` on any bound of \`Self\``) — judge whether that message is right for an inherent impl, where "bound" is the wrong word, or whether this case deserves its own wording. Either way it must be a diagnostic, not silence.

Test all three scopes resolve: `Self::Item` in a trait method (already works), in a trait-impl method (new), and rejected with a clear message in an inherent impl.

**Also correct Task 6's Step 2 expectation while you are here** — it predicts its test fails "because conformance is comparing an unnormalized projection against a concrete type." Before this step, it would actually have failed with `E0900` on the impl side, before conformance ran at all. After this step, Task 6's stated cause becomes the real one.

- [ ] **Step 4c: Reject `Self` as a user-written type-parameter name — this is what makes 4b unambiguous**

Task 3's re-review found that **`Self` is currently a legal type-parameter name**, which quietly gives Step 4b two possible meanings for `Self` in the same scope. Verified on `1c4cd47`:

```nova
trait It { type Item
 fn get(self) -> Int }
record W<Self> { v: Self }
impl<Self: It> W<Self> { fn peek(self) -> Self::Item { panic("x") } }
fn main() { }
```

`nova check` → **`ok`**. So `Self::Item` inside an impl *already* resolves today — as `Assoc { on: Param(0) }` — whenever the user writes `<Self>` themselves. The mechanism, all confirmed: `parse_ident` accepts `Token::SelfUpper` and returns the plain string `"Self"` (`crates/nova-parser/src/grammar.rs:2281-2284`), so `parse_generics_opt` (`:372`) admits it like any identifier; `generic_scope` (`crates/nova-typeck/src/check.rs:5432`) then inserts whatever name the user wrote; and impls build their scope with exactly that function (`:757`). `fn f<Self: It>(x: Self) -> Self::Item` is `ok` for the same reason.

Without this step, Step 4b would have to branch on "is there a user-written `Self` in `generics`?" and carry two meanings of `Self` forward into Tasks 5–8 and every diagnostic that prints the word. Reject the name instead:

- **Policy: a generic parameter may not be named `Self`.** Report a **new** code — `E0076` is free and sits in the right band, next to `E0073` (`check.rs:780`, "impl generic parameter not used in the self type"), which is the same kind of "this generic declaration is invalid" error and is raised a few lines after the `generic_scope` call this check belongs beside. Do not reuse `E0073` and do **not** use `E0900`: this is not an unimplemented feature, it is a name that will never be legal.
- Apply it to **every** generic declaration — `fn`, `record`, `trait`, `impl`, and method-level generics — not just impls. One shared helper called wherever generics are collected. A `record W<Self>` is exactly as confusing as an `impl<Self>`.
- **Safe to do:** `Self` is used as a type-parameter name nowhere in `std/`, `examples/`, or `tests/` (checked), and it is not currently reserved anywhere outside the lexer's `#[token("Self")]`. Expect zero pre-existing test churn; if a test does break, read it before touching it.
- Test both that `impl<Self: It> …` is now rejected with `E0076` naming the parameter, and that a plain `Self` inside a trait body still works — the rejection must not touch the legitimate implicit `Self` that `self_generic_scope` inserts, which is not a user-written parameter at all.

After this step, `Self` in an impl means exactly one thing (the impl's self type), which is the assumption Step 4b's implementation and Task 6's normalization both rest on. **Update the comment at `check.rs:1550` once this lands**, so it describes the finished state: `Self` is an ordinary `generics` key, populated only by `self_generic_scope` for trait bodies and default methods, because `E0076` now rejects the user-written path.

- [ ] **Step 5: Run the tests**

Run: `cargo test -p nova-typeck --no-fail-fast assoc_binding`, then `cargo test --workspace --no-fail-fast`
Expected: all PASS.

- [ ] **Step 6: Commit**

```bash
git add crates/nova-ast crates/nova-parser crates/nova-hir crates/nova-typeck
git commit -m "feat(trait): bind associated types in impls and check the set"
```

---

### Task 5: Normalization seam 1 — `check.rs`

**Files:**
- Modify: `crates/nova-typeck/src/check.rs` (a `normalize` helper; call it where types are consumed)

**Interfaces:**
- Consumes: Task 4's `ImplInfo.assoc_bindings`.
- Produces: `fn normalize(&self, ty: &Ty) -> Ty` — resolves `Assoc { on: <concrete> }` through the impl table, recursing into compound types, and leaves `Assoc { on: Param(_) }` alone.

- [ ] **Step 1: Write the failing test**

```rust
    #[test]
    fn a_projection_on_a_concrete_type_normalizes_at_a_use_site() {
        // `w.get_item()` returns `Self::Item`; with Self = W<Int> that is Int,
        // so assigning it to an Int must typecheck with no annotation.
        let r = check_src(
            "trait It { type Item\n fn get_item(self) -> Self::Item }\n\
             record W<T> { v: T }\n\
             impl<T> It for W<T> { type Item = T\n fn get_item(self) -> T { self.v } }\n\
             fn main() { let w = W { v: 7 }\n let n: Int = w.get_item()\n println(\"${n}\") }",
        );
        assert!(r.diagnostics.is_empty(), "{:?}", r.diagnostics);
    }

    #[test]
    fn a_projection_normalizes_to_the_wrong_type_is_an_error() {
        // The negative direction: Self::Item is Int here, so binding it to a
        // Bool must fail. Without this, a `normalize` that returned Ty::Error
        // or Ty::Never for everything would pass the positive test.
        let r = check_src(
            "trait It { type Item\n fn get_item(self) -> Self::Item }\n\
             record W<T> { v: T }\n\
             impl<T> It for W<T> { type Item = T\n fn get_item(self) -> T { self.v } }\n\
             fn main() { let w = W { v: 7 }\n let b: Bool = w.get_item()\n println(\"${b}\") }",
        );
        assert!(
            r.diagnostics.iter().any(|d| d.code == "E0010"),
            "expected a type mismatch: {:?}",
            r.diagnostics
        );
    }
```

The second test is the one that matters. A `normalize` that resolves everything to `Ty::Error` would satisfy the first test alone, because `Error` unifies with anything.

- [ ] **Step 2: Run them to verify they fail**

Run: `cargo test -p nova-typeck --no-fail-fast normalize`
Expected: the first FAILS (the projection never resolves, so `Assoc` fails to unify with `Int`); the second may pass vacuously — note that, it is why both exist.

- [ ] **Step 3: Implement `normalize`**

Resolve `on` first (it may itself contain a projection), take its `head()`, find the impl of the projection's owning trait for that head, recover the impl's type arguments with `match_pattern`, look up the binding, and `subst` the impl's arguments into it. Recurse into `Fn`, `Sum`, `Record`, `Array` so a projection nested inside a compound type is reached. Leave `Assoc { on: Param(_) }` untouched — that is Task 7's job.

Call it immediately after `fcx.icx.apply(..)` at the points that consume a type: the method-call return type, `let` annotations, and function return types. **Do not** call it inside `unify` — the whole design depends on projections being gone before unification, and normalizing inside would reintroduce the impl-table dependency the unifier deliberately lacks.

- [ ] **Step 4: Run the tests**

Run: `cargo test -p nova-typeck --no-fail-fast normalize`, then `cargo test --workspace --no-fail-fast`
Expected: both PASS.

- [ ] **Step 5: Prove the negative test is not vacuous**

Temporarily make `normalize` return `Ty::Error` for every `Assoc`. Rebuild and run both tests. Expected: the **positive** test still passes (because `Error` unifies with anything) while the **negative** test now fails to report `E0010`. That asymmetry is the point — record it, then restore.

- [ ] **Step 6: Commit**

```bash
git add crates/nova-typeck/src/check.rs
git commit -m "feat(typeck): normalize projections on concrete self types"
```

---

### Task 6: Normalization seam 2 — impl conformance

The spec's named risk #2: conformance compares a trait signature that may contain a projection against an impl signature that is already concrete. Normalizing one side only yields either a spurious `E0072` on every impl, or an accepted impl whose method has the wrong type.

**Files:**
- Modify: `crates/nova-typeck/src/check.rs` (`check_impl_conformance`)

**Interfaces:**
- Consumes: Task 5's `normalize`.
- Produces: conformance that accepts either spelling and still rejects a genuinely wrong signature.

- [ ] **Step 1: Write the failing tests**

```rust
    #[test]
    fn an_impl_may_echo_the_projection_or_write_the_concrete_type() {
        // Both spellings must be accepted (design doc §5.1).
        for ret in ["T", "Self::Item"] {
            let src = format!(
                "trait It {{ type Item\n fn get_item(self) -> Self::Item }}\n\
                 record W<T> {{ v: T }}\n\
                 impl<T> It for W<T> {{ type Item = T\n fn get_item(self) -> {ret} {{ self.v }} }}\n\
                 fn main() {{ }}"
            );
            let r = check_src(&src);
            assert!(r.diagnostics.is_empty(), "ret = {ret}: {:?}", r.diagnostics);
        }
    }

    #[test]
    fn a_genuinely_wrong_impl_signature_still_reports_e0072() {
        // The risk is that normalizing to make the two spellings agree also
        // makes everything agree. Self::Item is T here, so returning Bool is
        // wrong and must still be caught.
        let r = check_src(
            "trait It { type Item\n fn get_item(self) -> Self::Item }\n\
             record W<T> { v: T }\n\
             impl<T> It for W<T> { type Item = T\n fn get_item(self) -> Bool { true } }\n\
             fn main() { }",
        );
        assert!(
            r.diagnostics.iter().any(|d| d.code == "E0072"),
            "expected a conformance error: {:?}",
            r.diagnostics
        );
    }
```

- [ ] **Step 2: Run them to verify they fail**

Run: `cargo test -p nova-typeck --no-fail-fast conformance_projection`
Expected: the first FAILS for at least one spelling. If it fails for **both**, conformance is comparing an unnormalized projection against a concrete type — expected before the fix.

- [ ] **Step 3: Implement**

In `check_impl_conformance`, substitute the impl's self type for `Param(0)` in the trait's declared signature, then `normalize` **both** sides before comparing. Substituting alone is not enough: it turns `Self::Item` into `Assoc { on: W<Param(0)> }`, which is still a projection.

- [ ] **Step 4: Run the tests**

Run: `cargo test -p nova-typeck --no-fail-fast conformance_projection`, then `cargo test --workspace --no-fail-fast`
Expected: all PASS. Watch the existing conformance tests especially — `E0072` is exercised by several, and normalizing both sides is exactly the change that could make a real mismatch compare equal.

- [ ] **Step 5: Commit**

```bash
git add crates/nova-typeck/src/check.rs
git commit -m "fix(typeck): normalize both sides when checking impl conformance"
```

---

### Task 7: Normalization seam 3 — monomorphization, and the surviving-projection diagnostic

**Files:**
- Modify: `crates/nova-mir/src/mono.rs`

**Interfaces:**
- Consumes: Tasks 1–6.
- Produces: after `subst`, any `Assoc` whose `on` became concrete is resolved; one that survives is a diagnostic, not silent `MirTy::Unit`.

- [ ] **Step 1: Write the failing test**

In `crates/nova-cli/tests/run_tests.rs`:

```rust
/// A generic function whose signature mentions a projection, instantiated at
/// TWO different types. One instantiation would pass even if `subst` dropped
/// the binding and every projection resolved to the same thing; two cannot.
#[test]
fn a_projection_resolves_per_instantiation_at_monomorphization() {
    let src = "trait It { type Item\n fn get_item(self) -> Self::Item }\n\
               record W<T> { v: T }\n\
               impl<T> It for W<T> { type Item = T\n fn get_item(self) -> T { self.v } }\n\
               fn unwrap_item<I: It>(x: I) -> I::Item { x.get_item() }\n\
               fn main() {\n\
                   let a = unwrap_item(W { v: 7 })\n\
                   let b = unwrap_item(W { v: true })\n\
                   println(\"${a} ${b}\")\n\
               }";
    let dir = std::env::temp_dir().join("nova-assoc-mono");
    std::fs::create_dir_all(&dir).expect("temp dir");
    let path = dir.join("main.nova");
    std::fs::write(&path, src).expect("write");
    nova().arg("run").arg(&path).assert().success().stdout("7 true\n");
}
```

- [ ] **Step 2: Run it to verify it fails**

Run: `cargo build -p nova-cli` then `cargo test -p nova-cli --test run_tests a_projection_resolves_per_instantiation`
Expected: FAIL. The projection survives to lowering, `mir_ty` maps it to `MirTy::Unit`, and the program prints the wrong thing or the backend errors.

- [ ] **Step 3: Implement**

In `mono.rs`, after `specialize`/`subst` produces a function instance, walk its types and normalize every `Assoc` whose `on` is now concrete, using the same impl-table logic as Task 5 — factor that logic so it is written **once** and called from both, rather than reimplemented. Phase 2.2a's headline defect was one probe scan existing in two copies that could drift; two copies of projection normalization is the same hazard with worse consequences.

Then, if an `Assoc` remains after normalization, emit a diagnostic naming the projection rather than letting it reach `mir_ty`. Use `E0013`'s neighbourhood in `mono.rs` as the model for how mono reports.

- [ ] **Step 4: Run the tests**

Run: `cargo build -p nova-cli`, `cargo test -p nova-cli --test run_tests a_projection_resolves_per_instantiation`, then `cargo test --workspace --no-fail-fast`
Expected: PASS, printing `7 true`.

- [ ] **Step 5: Prove both instantiations matter**

Temporarily make the mono normalization cache its first result and reuse it for every projection. Rebuild, run the test. Expected: **FAILS**, printing something like `7 7` — the second instantiation gets the first's `Item`. Restore. This proves the two-instantiation test earns its keep; a single-instantiation test would have passed.

- [ ] **Step 6: Commit**

```bash
git add crates/nova-mir/src/mono.rs crates/nova-cli/tests/run_tests.rs
git commit -m "feat(mir): normalize projections at monomorphization"
```

---

### Task 8: Enforce `mut self` on trait methods

ADR 0005 recorded this as an open gap with an explicit gate: closing it is *"a hard gate before any `mut self` trait method lands"*. `Iterator::next` (Task 9) is that first method, so this must land before it.

**Files:**
- Modify: `crates/nova-hir/src/lib.rs` (`TraitMethod.mut_self`)
- Modify: `crates/nova-typeck/src/check.rs` (`collect_traits`; `MethodRes::Trait` arm of `check_method_call`; `check_impl_conformance`)
- Modify: `docs/adr/0005-mutable-receivers-and-one-shot-hash.md` (record the gap as closed)

**Interfaces:**
- Consumes: nothing from earlier tasks.
- Produces: `hir::TraitMethod.mut_self: bool`; `E0060` on trait-dispatched calls; conformance checking `mut self` in both directions.

- [ ] **Step 1: Write the failing tests**

```rust
    #[test]
    fn a_mut_self_trait_method_on_an_immutable_receiver_reports_e0060() {
        let r = check_src(
            "trait Bump { fn bump(mut self) }\n\
             record C { n: Int }\n\
             impl Bump for C { fn bump(mut self) { self.n = self.n + 1 } }\n\
             fn main() { let c = C { n: 1 }\n c.bump() }",
        );
        assert!(
            r.diagnostics.iter().any(|d| d.code == "E0060"),
            "expected E0060 on a mut-self trait method through an immutable binding: {:?}",
            r.diagnostics
        );
    }

    #[test]
    fn a_mut_self_trait_method_on_a_mutable_receiver_is_accepted() {
        let r = check_src(
            "trait Bump { fn bump(mut self) }\n\
             record C { n: Int }\n\
             impl Bump for C { fn bump(mut self) { self.n = self.n + 1 } }\n\
             fn main() { let mut c = C { n: 1 }\n c.bump()\n println(\"${c.n}\") }",
        );
        assert!(r.diagnostics.is_empty(), "{:?}", r.diagnostics);
    }

    #[test]
    fn an_impl_disagreeing_about_mut_self_is_a_conformance_error() {
        // Both directions: the trait says mut, the impl does not, and vice
        // versa. Either way the receiver's mutability requirement would be
        // decided by whichever table the caller happened to consult.
        for (t, i) in [("mut self", "self"), ("self", "mut self")] {
            let src = format!(
                "trait Bump {{ fn bump({t}) }}\n\
                 record C {{ n: Int }}\n\
                 impl Bump for C {{ fn bump({i}) {{ }} }}\n\
                 fn main() {{ }}"
            );
            let r = check_src(&src);
            assert!(
                r.diagnostics.iter().any(|d| d.code == "E0072"),
                "trait `{t}` vs impl `{i}` must be a conformance error: {:?}",
                r.diagnostics
            );
        }
    }
```

- [ ] **Step 2: Run them to verify they fail**

Run: `cargo test -p nova-typeck --no-fail-fast mut_self_trait`
Expected: the first and third FAIL (today the gap is open — measured: a `mut self` trait method on a non-`mut` binding silently mutates it); the second passes already.

- [ ] **Step 3: Implement**

Add `mut_self: bool` to `hir::TraitMethod`, fill it in `collect_traits` from the AST receiver, call `check_mutable_receiver` from `check_method_call`'s `MethodRes::Trait` arm — which currently dispatches straight to `emit_trait_call` with no mutability check at all — and compare `mut_self` in `check_impl_conformance`.

- [ ] **Step 4: Deliberately flip the test that pins today's behaviour**

`trait_method_mut_self_is_not_enforced_on_immutable_receiver_known_gap` asserts the permissive behaviour on purpose, so that closing the gap could not happen silently. **It must now flip, and that is the intended outcome, not a test bent to fit.**

- Rename it to `trait_method_mut_self_is_enforced_on_immutable_receiver`.
- Invert the assertion: expect `E0060` where it expected no diagnostics.
- Rewrite its doc comment: drop "documents a known gap, not a desired behaviour", state the enforced rule, and cite this plan's spec plus ADR 0005 §1's migration path as what authorised the change.

Then update ADR 0005 itself: the migration path now describes something done, not pending.

- [ ] **Step 5: Check nothing in std relied on the permissive behaviour**

Run: `git grep -n 'mut self' -- std/` and confirm every hit is an **inherent** method (there were nine, all inherent, at the time this plan was written — re-verify rather than trusting that).

Run: `cargo build -p nova-cli` then all three gates. Expected: byte-identical. If a gate moves, some std code was relying on the gap and that is a finding to report, not to work around.

- [ ] **Step 6: Run everything**

Run: `cargo test --workspace --no-fail-fast`, `cargo clippy --all-targets --all-features -- -D warnings`, `cargo fmt --check`
Expected: all green.

- [ ] **Step 7: Commit**

```bash
git add crates/nova-hir crates/nova-typeck docs/adr/0005-mutable-receivers-and-one-shot-hash.md
git commit -m "fix(typeck): enforce the mutable-receiver rule for trait methods"
```

---

### Task 9: `Iterator` in std/core, `VecIter` and `Vec::iter` in std/collections

**Files:**
- Modify: `std/core/lib.nova` (`pub trait Iterator`)
- Modify: `std/collections/lib.nova` (`VecIter<T>`, its impl, `Vec::iter`)

**Interfaces:**
- Consumes: everything above.
- Produces: `Iterator` with `type Item` and `fn next(mut self) -> Option<Self::Item>`; `VecIter<T>`; `Vec<T>::iter(self) -> VecIter<T>`.

- [ ] **Step 1: Write the failing test**

```rust
/// Iterating a Vec by hand through the Iterator trait: no `for` desugar and no
/// default methods exist yet, so this is what iteration looks like today.
#[test]
fn a_vec_iterates_to_exhaustion_through_the_iterator_trait() {
    let src = "fn main() {\n\
                   let mut v: Vec<Int> = Vec::new()\n\
                   v.push(10)\n\
                   v.push(20)\n\
                   let mut it = v.iter()\n\
                   let mut total = 0\n\
                   let mut steps = 0\n\
                   while steps < 5 {\n\
                       match it.next() {\n\
                           Some(x) => total = total + x,\n\
                           None => steps = 99,\n\
                       }\n\
                       if steps == 99 { steps = 5 } else { steps = steps + 1 }\n\
                   }\n\
                   println(\"total=${total}\")\n\
                   let mut e: Vec<Int> = Vec::new()\n\
                   let mut ei = e.iter()\n\
                   match ei.next() { Some(x) => println(\"unexpected ${x}\"), None => println(\"empty ok\") }\n\
               }";
    let dir = std::env::temp_dir().join("nova-assoc-veciter");
    std::fs::create_dir_all(&dir).expect("temp dir");
    let path = dir.join("main.nova");
    std::fs::write(&path, src).expect("write");
    nova()
        .arg("run")
        .arg(&path)
        .assert()
        .success()
        .stdout("total=30\nempty ok\n");
}
```

**The loop shape above is deliberately awkward** because `break` inside a `match` arm followed by a newline hits the ASI-style parse pitfall in the Global Constraints. If a cleaner loop compiles, use it — but verify it, do not assume.

- [ ] **Step 2: Run it to verify it fails**

Run: `cargo test -p nova-cli --test run_tests a_vec_iterates_to_exhaustion`
Expected: FAIL — no `Iterator`, no `Vec::iter`.

- [ ] **Step 3: Add `Iterator` to std/core**

```nova
// The iteration protocol. `next` advances the iterator and yields the next
// element, or `None` once exhausted.
//
// `mut self` is load-bearing: an iterator must advance, which means mutating
// its own state. Enforcing the mutable-receiver rule for trait methods was a
// prerequisite for declaring this trait at all — see ADR 0005 §1.
//
// NOT YET: no `for x in it` desugar, and no default methods (`map`, `filter`,
// `collect`, `fold`), so iteration is a hand-written `while` plus a `match`.
// Both are deliberately deferred; see the Phase 2.2c design doc.
pub trait Iterator {
    type Item
    fn next(mut self) -> Option<Self::Item>
}
```

- [ ] **Step 4: Add `VecIter` and `Vec::iter` to std/collections**

```nova
// A cursor over a `Vec<T>`. Holds the vector and the next index to yield.
//
// The vector is held by value, and records are heap objects passed by
// pointer, so this shares the caller's storage rather than copying it — a
// `push` during iteration is visible to the iterator. That is the same
// alias-visibility the whole collections module has (see the module header);
// it is documented rather than prevented, because preventing it needs
// borrow tracking the language does not have.
pub record VecIter<T> { v: Vec<T>, i: Int }

impl<T> Iterator for VecIter<T> {
    type Item = T
    fn next(mut self) -> Option<T> {
        if self.i >= self.v.len() { return None }
        let x = self.v.get(self.i)
        self.i = self.i + 1
        x
    }
}
```

and inside the **existing** `impl<T> Vec<T>` block — do not open a second one:

```nova
    // A cursor over this vector's elements, starting at index 0.
    pub fn iter(self) -> VecIter<T> { VecIter { v: self, i: 0 } }
```

- [ ] **Step 5: Run the tests**

Run: `cargo build -p nova-cli`, `cargo test -p nova-cli --test run_tests a_vec_iterates_to_exhaustion`, then `cargo test --workspace --no-fail-fast`
Expected: PASS. **Every program in the suite compiles all std modules**, so a broad failure means the Nova source is wrong, not the tests.

- [ ] **Step 6: Confirm the three existing gates are byte-identical**

Run each of `collections`, `std_core`, `strings` under `nova run` and compare with `tr -d '\r'`. Expected: identical. `Vec` gained a method and `std/core` gained a trait, so nothing existing should move — if a gate does move, find out why before proceeding.

- [ ] **Step 7: Commit**

```bash
git add std/core/lib.nova std/collections/lib.nova crates/nova-cli/tests/run_tests.rs
git commit -m "feat(std): add the Iterator trait and Vec::iter"
```

---

### Task 10: Gate fixture, ADR, spec correction, CHANGELOG

**Files:**
- Create: `tests/runtime/assoc_types.{nova,stdout}`, `docs/adr/0006-associated-type-syntax.md`
- Modify: `crates/nova-cli/tests/run_tests.rs`, `nova-spec/20-STDLIB.md`, `CHANGELOG.md`

**Interfaces:**
- Consumes: everything.
- Produces: `assoc_types_run`, `assoc_types_build_standalone`, `assoc_types_under_gc_stress`.

- [ ] **Step 1: Write the fixture**

`tests/runtime/assoc_types.nova`, covering the spec's §7 list. Nothing in it may panic — a panic aborts the process and silently truncates the gate.

1. A trait with an associated type bound to **the impl's own generic parameter**, and a value obtained through it.
2. `it.next()` on a concrete `VecIter<Int>` typing as `Option<Int>` with no annotation.
3. A **generic** function whose signature mentions the projection, called at **two** instantiations (`Int` and `Bool`) — one alone cannot distinguish per-instantiation resolution from a cached first answer.
4. A trait with **two** associated types, proving the implementation does not assume there is only one.
5. A `Vec` iterated to exhaustion, and a `Vec::new()` whose first `next` is already `None`.

- [ ] **Step 2: Generate the expected output, then read every line**

```bash
cargo build -p nova-cli
./target/debug/nova.exe run tests/runtime/assoc_types.nova > tests/runtime/assoc_types.stdout
```

**Do not stop there.** A fixture generated from actual output pins whatever the code does, bugs included. Read every line and confirm it is what you decided in advance. Report which lines you predicted and whether any surprised you.

- [ ] **Step 3: Add the three gate tests**

Model them exactly on `strings_run` / `strings_build_standalone` / `strings_under_gc_stress`, including the `.replace("\r\n", "\n")` normalisation. `NOVA_GC_STRESS=1` matters here: `VecIter` holds a `Vec` which holds an array, so a collection during iteration must not lose the backing storage.

- [ ] **Step 4: Ask what mutation survives the fixture**

Every task in the preceding phase shipped a test that a one-character mutation survived, each caught only in review, and the pattern never varied: the survivor sat at a boundary the obvious test skipped. Before committing, mutate `std/collections/lib.nova` and the normalization code and check the fixture notices. **Run at least four**, including:

- `VecIter::next`'s `self.i >= self.v.len()` → `>`, which should walk one past the end.
- `self.i = self.i + 1` deleted, which should loop forever or repeat the first element — pick a mutation whose failure is observable, not a hang.
- the mono normalization caching its first answer (Task 7 Step 5's mutation) — the fixture's two instantiations should catch it.
- binding `type Item = T` changed to a fixed primitive in the impl, which should break one instantiation but not the other.

Report each with its actual output. A mutation that survives is a missing assertion.

- [ ] **Step 5: Write ADR 0006 for the syntax deviation**

`nova-spec/20-STDLIB.md:93` writes `Option<Self.Item>` with a dot; Nova uses `::`. Record: the decision, that `A::B` in type position already parsed while the dot form needed new grammar, that `::` is already Nova's reach-into-a-type operator for associated *functions* (`P::new`, `T::default`) so a dot would mean two spellings for one idea, and that `.` reads as field access everywhere else. Follow ADR 0004 and 0005's structure.

- [ ] **Step 6: Correct the spec**

Change `nova-spec/20-STDLIB.md`'s `Iterator` to `fn next(mut self) -> Option<Self::Item>` — both the `::` and the `mut self`. Add a line pointing at ADR 0006. Do **not** silently rewrite it: the spec is the authority, and a corrected line with no recorded reason is how a future reader loses the argument.

- [ ] **Step 7: Update the CHANGELOG**

Under `[Unreleased]`: associated types (`type Item`, `Self::Item` / `I::Item`, bindings in impls, `E0900` for bounds on them); the `::` deviation with its ADR; **the `mut self` trait-method enforcement as the behaviour change it is** — code that compiled before may now be `E0060`; and `Iterator` + `VecIter` + `Vec::iter`. State plainly what is *not* there: no `for x in it`, no `map`/`filter`/`collect`/`fold`, no `Set`/`String` iterator, `Map::iter` still impossible without tuples.

- [ ] **Step 8: Final verification**

```bash
cargo test --workspace --no-fail-fast
```
```bash
cargo clippy --all-targets --all-features -- -D warnings
```
```bash
cargo fmt --check
```

Plus all four gates by hand in all three modes:

```bash
for f in collections std_core strings assoc_types; do diff <(tr -d '\r' < tests/runtime/$f.stdout) <(./target/debug/nova.exe run tests/runtime/$f.nova | tr -d '\r') && echo "$f OK"; done
```

- [ ] **Step 9: Commit**

```bash
git add tests/runtime/assoc_types.nova tests/runtime/assoc_types.stdout crates/nova-cli/tests/run_tests.rs docs/adr/0006-associated-type-syntax.md nova-spec/20-STDLIB.md CHANGELOG.md
git commit -m "test(trait): add the associated-types end-to-end gate"
```

---

## Plan Self-Review

**Spec coverage** — every section maps to a task:

| Spec | Task |
|---|---|
| §3 `::` syntax + ADR | 3 (resolution), 10 (ADR + spec correction) |
| §4 `Ty::Assoc` representation | 1 |
| §4.1 three normalization seams | 5 (check.rs), 6 (conformance), 7 (mono) |
| §4.2 `Assoc { on: Var }` unreachable | 1 (the variant's doc comment records the argument; Step 4's `occurs`/`unify` tests cover a `Var` inside `on`, which is a different and legal case) |
| §4.3 blast radius, nine sites | 1, Steps 4 and 6 |
| §5 surface (`Iterator`, `VecIter`, `Vec::iter`) | 9 |
| §5.1 the six pinned cases | 6 (either spelling), 5 (`Self::Item` in an impl), 3 (projection anywhere a type may appear), 9 (existing `impl<T> Vec<T>` block), 4 (`E0072` both directions), 1 (no backwards inference — `unify` has no `Assoc`-vs-concrete arm) |
| §6 `mut self` enforcement + flipping the gap test | 8 |
| §7 gate, items 1–5 | 10 Step 1 |
| §7 `#[test]`s, items 6–10 | 2 (E0900 bounds), 4 (E0072 missing/extra), 8 (E0060 + mismatch) |
| §8 what is left broken | 10 Step 7 (CHANGELOG states it) |
| §9 risks 1–4 | 1 (`mir_ty` defensive + Task 7's diagnostic), 6 (conformance), 8 Steps 4–5 (flipping the test, checking std), 1 Step 4 (`self_types_overlap` conservative) |
| §10 definition of done | 10 Step 8 |

**Placeholder scan:** no "TBD"/"TODO"/"handle edge cases". Three steps deliberately say *read the surrounding code before writing* rather than giving code — Task 3 Step 3's `resolve_projection` (the per-parameter bound table differs between a trait method, an impl method and a free function, and naming the wrong one is the likely bug), Task 2 Step 4's parser alternative (it must reuse the existing bound-list parser), and Task 4 Step 3's impl-body `type` alternative. These are instructions to look, not omissions — inventing a field name I have not verified is how two briefs in the last phase cited things that did not exist.

**Type consistency:** `Ty::Assoc { on: Box<Ty>, assoc: DefId }` is spelled identically in Tasks 1, 3, 4, 5, 7. `TraitDef.assoc_types: Vec<(String, DefId)>` (Task 2) is what Task 3 searches and Task 4 compares against. `ImplInfo.assoc_bindings: Vec<(DefId, Ty)>` (Task 4) is what Tasks 5 and 7 read. `TraitMethod.mut_self: bool` (Task 8) is used only there. `normalize` is introduced in Task 5 and **factored** for reuse in Task 7 rather than reimplemented — called out explicitly because duplicating it is the same hazard as Phase 2.2a's duplicated probe scan.

**Every Nova construct used in the plan's test code exists and was verified this session:** records with generics, `impl<T> Trait for R<T>`, `mut self` methods, field assignment, `match` on `Option` with a bound payload, `while` with a counter, string interpolation, `Vec::new`/`push`/`get`/`len`, `panic`. The one shape I flagged rather than assumed is `break` inside a `match` arm (Task 9 Step 1), which is why that loop is written with a sentinel instead.

**~~Known plan risk~~ — resolved before Task 3 was dispatched.** I had flagged `resolve_projection` as the least specified step, on the theory that the bound table was scope-dependent and I might have under-modelled the threading. Traced it instead: there is exactly **one** table (`Vec<Vec<DefId>>`, indexed like `generics`' values — the local `bounds` during collection, `FnCtx.param_bounds` inside a body), it is uniform across trait methods, impl methods and free functions, and `collect_signatures` already resolves it *before* calling `convert_ty`. Task 3's step now names all of it.

The one real defect the trace found: **`convert_ty` cannot see the bounds** — it takes parameter names only — so it needs a new argument threaded through 19 call sites. The plan originally implied `resolve_projection` could just read them, which was wrong.
