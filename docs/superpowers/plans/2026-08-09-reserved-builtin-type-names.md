# Reserve the built-in type names — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Declaring a type named after a built-in type is rejected at the declaration with `E0089`, instead of compiling and then failing at every use site with two identically-printed types.

**Architecture:** One check, in the resolver where type names are collected, beside the existing `E0002` duplicate-definition check. The reserved list is a `pub const` in `nova-resolver`, which `nova-typeck` already depends on, so the resolver's check and `convert_ty`'s built-in table can share one source of truth rather than two hardcoded copies.

**Tech Stack:** Rust (nova-resolver, nova-typeck).

**Spec:** `docs/superpowers/specs/2026-08-09-reserved-builtin-type-names-design.md`. **Read §2's probe table and §4 before starting** — §2 corrects four claims the originating review got wrong, and §4's non-goals are the scope boundary this task's own tests have to defend.

**Base:** `main` at `ec61446`. Create branch `reserved-type-names`.

**One task.** The check, its reserved list, the twelve rejections, the three non-goal tests and the CHANGELOG line are one deliverable — a reviewer could not meaningfully accept the check while rejecting the tests that define its boundary, and separating them would invite an implementation that over-rejects with nothing to catch it. The per-task review and the whole-branch review will therefore see nearly the same diff; that is expected for a change this size.

## Global Constraints

- **`cargo build --workspace` BEFORE `cargo test`.** `cargo test` does not regenerate `nova-runtime`'s staticlib. This plan does not touch it, but the habit costs nothing and its absence produces ~25 unrelated MSVC unresolved-symbol failures that read like a codegen bug.
- **`--no-fail-fast` is mandatory** on `cargo test --workspace`. Never pipe cargo output through `head`/`tail` before summing totals — it truncates and under-reports.
- **A zero-match `cargo test <filter>` EXITS 0.** Confirm `running N tests` is non-zero before treating a filtered run as evidence. `--exact` needs the fully-qualified path (e.g. `check::tests::<name>`), not a bare fn name.
- **Baseline: 44 targets, 819 passed, 0 failed, 8 ignored** at default parallelism. Take your own baseline; do not trust this number. The 8 ignored are conservative-scan GC tests deliberately gated by ADR 0010 — **do not touch them and do not try to fix them.**
- **Assert content, not just a code.** For every test, ask what one-character change survives it. `E0089`'s message carries two claims and the second is the one a user cannot otherwise discover, so asserting the code alone is not enough.
- **A module's doc must not assert its caller's policy, and no comment may narrate a measurement** (ADR 0009 §2). Invariant in the comment, measurement in the report. The digit-scan heuristic **under-detects** — "passes every other test in the workspace" and "more than once" both slipped through on earlier branches with no digit in either. Scan for measurement *phrases* and quantities written as words.
- **No `reason = "…"` in lint attributes** — workspace MSRV is 1.78 and that needs 1.81.
- Clippy `-D warnings` and `cargo fmt --all --check` clean before committing.
- **THE CODE WINS OVER THIS PLAN.** On the last three branches, implementers falsified plan claims eighteen times and were right every time, several of them mine — including three in the previous plan alone. Measure, report the correction, proceed correctly.
- **Do NOT push.** Commit on `reserved-type-names`. End the commit body with:
  ```
  Co-Authored-By: Claude Opus 5 <noreply@anthropic.com>
  ```

## File Structure

| File | Responsibility |
|---|---|
| `crates/nova-resolver/src/lib.rs` | `RESERVED_TYPE_NAMES`; the `E0089` check at both type-collection sites |
| `crates/nova-typeck/src/check.rs` | reference the shared list from the built-in name table; the non-goal and drift tests |
| `CHANGELOG.md` | the language-surface change |

---

### Task 1: Reserve the six built-in type names

**Files:**
- Modify: `crates/nova-resolver/src/lib.rs`
- Modify: `crates/nova-typeck/src/check.rs` (tests, and one doc reference)
- Modify: `CHANGELOG.md`

**Interfaces:**
- Produces: `pub const RESERVED_TYPE_NAMES: [&str; 6]` in `nova-resolver` — `["Int", "Float", "Bool", "Char", "String", "Future"]` — and diagnostic `E0089`.

- [ ] **Step 1: Confirm the ground truth, because the spec's line numbers are already known to drift**

The spec cites `crates/nova-typeck/src/check.rs:2437` (`convert_ty`'s nullary table), `:5212` (`qualifier_self_ty`), and `:2421`/`:5221` for `Future`'s separate handling. It also warns that the figures the originating review carried were stale. **Re-derive all four before relying on them:**

```bash
grep -n '"Int" =>' crates/nova-typeck/src/check.rs
grep -n 'name == "Future"\|"Future" =>' crates/nova-typeck/src/check.rs
grep -n 'DefKind::Record { item_index }\|kind: DefKind::Sum {' crates/nova-resolver/src/lib.rs
grep -n 'E0002' crates/nova-resolver/src/lib.rs
```

Also confirm `E0089` is still free: `grep -rho "E0[0-9][0-9][0-9]" crates/ --include=*.rs | sort -u | tail`. Report any figure that moved.

- [ ] **Step 2: Write the failing tests**

In `crates/nova-typeck/src/check.rs`'s `mod tests`. The helper is `check_src(src) -> CheckResult`, whose fields are `{ module, diagnostics }` — there is no `defs` field.

```rust
#[test]
fn declaring_a_type_named_for_a_builtin_is_rejected() {
    // All six names, both declaration forms. A user type under one of these
    // names can never be named in a type annotation -- `convert_ty` resolves
    // the name to the built-in before it reaches `resolve_type` -- so the
    // declaration is rejected where the user can act on it rather than at
    // every annotation use site. [Corrected 2026-08-09 from "can never be
    // referred to" -- see the spec's §3.2 note. Construction and pattern
    // matching resolve outside `convert_ty` and this rejection breaks them
    // too; that was this plan's own instance of the same overclaim.]
    for name in nova_resolver::RESERVED_TYPE_NAMES {
        for src in [
            format!("record {name} {{ v: Bool }}\nfn main() {{ }}"),
            format!("type {name} = | A | B\nfn main() {{ }}"),
        ] {
            let r = check_src(&src);
            let d = r
                .diagnostics
                .iter()
                .find(|d| d.code == "E0089")
                .unwrap_or_else(|| panic!(
                    "expected E0089 for `{name}` in {src:?}, got {:?}",
                    r.diagnostics.iter().map(|d| &d.code).collect::<Vec<_>>()
                ));
            // Both halves of the message matter. The name identifies which
            // built-in was shadowed; the second half is the fact a user cannot
            // discover from the declaration alone, and a code-only assertion
            // would survive deleting it.
            assert!(
                d.message.contains(name),
                "E0089 must name the built-in it collides with; got {:?}",
                d.message
            );
            assert!(
                d.message.contains("built-in") || d.message.contains("builtin"),
                "E0089 must say the name belongs to a built-in type; got {:?}",
                d.message
            );
        }
    }
}

#[test]
fn every_reserved_name_really_is_a_builtin_type() {
    // The drift guard, in the direction that can be caught. If a name is
    // removed from `convert_ty`'s table while staying in the reserved list,
    // this fails -- annotating with it would become `E0001 cannot find type`.
    //
    // The other direction (a seventh built-in added without reserving it) is
    // NOT caught here and cannot be from a fixed list; the mitigation is the
    // pointer comment at the table itself (Step 4).
    for name in nova_resolver::RESERVED_TYPE_NAMES {
        let ann = if name == "Future" { "Future<Int>" } else { name };
        let r = check_src(&format!("fn f(x: {ann}) -> Int {{ 1 }}\nfn main() {{ }}"));
        assert!(
            !r.diagnostics.iter().any(|d| d.code == "E0001"),
            "`{name}` is in RESERVED_TYPE_NAMES but is not a built-in type name: {:?}",
            r.diagnostics.iter().map(|d| d.message.clone()).collect::<Vec<_>>()
        );
    }
}

#[test]
fn a_generic_parameter_named_for_a_builtin_still_works() {
    // NON-GOAL, pinned. `convert_ty` resolves generics BEFORE the built-in
    // table, so this shadowing is coherent rather than broken: the parameter
    // genuinely means the parameter. Compiling is not enough to show that --
    // the function must still behave as written, or the design's claim that
    // this case is sound is unfounded.
    let r = check_src("fn f<Int>(x: Int) -> Int { x }\nfn main() { }");
    assert!(
        r.diagnostics.is_empty(),
        "a generic parameter may shadow a built-in type name, got {:?}",
        r.diagnostics.iter().map(|d| d.message.clone()).collect::<Vec<_>>()
    );
}

#[test]
fn a_trait_named_for_a_builtin_still_works() {
    // NON-GOAL, pinned. Traits are a separate namespace: `trait Int` does not
    // shadow the type, and the return annotation below resolves to the
    // primitive.
    let r = check_src("trait Int { fn m(self) -> Int }\nfn main() { }");
    assert!(
        r.diagnostics.is_empty(),
        "a trait may be named for a built-in type, got {:?}",
        r.diagnostics.iter().map(|d| d.message.clone()).collect::<Vec<_>>()
    );
}

#[test]
fn an_unknown_type_name_is_still_e0001_not_e0089() {
    // The new check must not swallow the not-found path.
    let r = check_src("fn f(x: Nope) -> Int { 1 }\nfn main() { }");
    assert!(r.diagnostics.iter().any(|d| d.code == "E0001"));
    assert!(!r.diagnostics.iter().any(|d| d.code == "E0089"));
}

#[test]
fn an_ordinary_type_declaration_is_unaffected() {
    // Kills a check that fires on every type name rather than the reserved
    // ones. Weak on its own -- the whole suite would fail -- but it states the
    // boundary at the point the reader is looking at.
    let r = check_src("record Wrap { v: Int }\ntype Two = | A | B\nfn main() { }");
    assert!(!r.diagnostics.iter().any(|d| d.code == "E0089"));
}
```

**`a_generic_parameter_named_for_a_builtin_still_works` asserting only `is_empty()` is weaker than the comment claims.** It shows the program type-checks, not that `f(3)` returns `3`. Add a behavioural check at whatever layer this codebase can reach it — a `tests/runtime/` fixture, or an existing driver-level probe — and **if no layer can assert the value from a typeck test, say so in your report** rather than leaving the comment overstating.

- [ ] **Step 3: Run the tests to verify they fail**

```bash
cargo build --workspace && cargo test -p nova-typeck --no-fail-fast 2>&1 | tail -25
```

Expected: compile failure — `nova_resolver::RESERVED_TYPE_NAMES` does not exist. Once it compiles, the six-name test fails because nothing emits `E0089`, and the four boundary tests should already pass. **Confirm the boundary tests pass BEFORE the check exists** — they describe today's behaviour, and a boundary test that only passes afterwards was measuring the wrong thing.

- [ ] **Step 4: Add the list and the check**

In `crates/nova-resolver/src/lib.rs`, beside the existing `Builtin::GLOBAL` / `STD_ONLY` const arrays:

```rust
/// Type names the compiler owns, which a user type declaration may not take.
///
/// `nova-typeck`'s `convert_ty` resolves each of these to its built-in type
/// before consulting `resolve_type`, so a user type under one of these names
/// could never be named in an annotation: the declaration would compile and
/// every use of it would fail instead, reporting the same type on both sides
/// of a mismatch. Rejecting the declaration says that where a user can act on
/// it.
///
/// `Future` is here and is not a primitive: it is the one built-in type name
/// taking a type argument, handled separately from the nullary table.
/// `Unit` is deliberately absent — it is not a nameable type name; unit is
/// spelled `()`.
///
/// This is the list, and `convert_ty`'s table is expected to agree with it.
pub const RESERVED_TYPE_NAMES: [&str; 6] =
    ["Int", "Float", "Bool", "Char", "String", "Future"];
```

**Corrected 2026-08-09 (post-implementation review) — this sample text was wrong, do not copy it
verbatim.** "Every use of it would fail instead" overclaims: `check_record_literal` resolves a
record literal's head through `resolve_type` directly, and a sum type's variants live in the value
namespace, both independent of `convert_ty`. Construction and pattern matching worked before this
change; only naming the type in a type annotation was already broken. See the spec's §3.2 for the
full mechanism and what this means for the change's cost, and `crates/nova-resolver/src/lib.rs`'s
actual `RESERVED_TYPE_NAMES` doc comment for the corrected wording that shipped.

Then the check at both type-collection sites — the sum arm and the record arm identified in Step 1 — emitting `E0089` and **skipping the definition** rather than registering it, so a following use reports nothing extra. Model the diagnostic's construction and labelling on the neighbouring `E0002`.

Message shape: name the built-in, and state that a declaration under this name could not be named in any type annotation. Do not narrate the mechanism in the diagnostic; the doc comment above owns that.

*(Corrected 2026-08-09: originally "could never be referred to" — this plan's own instance of the §3.2 overclaim. Construction and pattern matching are a different, unaffected-by-`convert_ty` path, so "referred to" was too broad; "named in any type annotation" is the claim that is actually true.)*

Finally, add a one-line pointer at `convert_ty`'s built-in table naming `RESERVED_TYPE_NAMES` as the list a new built-in type name must also join. **That pointer is the only mitigation for the drift direction no test can catch**, so it must be at the table, not only in the const's own doc.

- [ ] **Step 5: Run the tests to verify they pass**

```bash
cargo build --workspace && cargo test --workspace --no-fail-fast 2>&1 | grep -E "^test result" | awk '{p+=$4; f+=$6; i+=$8} END {print "targets:", NR, "passed:", p, "failed:", f, "ignored:", i}'
```

**Any pre-existing test that declares a type named for a built-in will now fail.** If one does, that test was relying on the defect — report which, and what it was testing, before changing it.

- [ ] **Step 6: Kill four mutations by hand**

1. Remove `"Char"` from the list → the six-name test fails for `Char` only.
2. Extend the check to reject generic parameters → `a_generic_parameter_named_for_a_builtin_still_works` fails.
3. Extend the check to reject trait names → `a_trait_named_for_a_builtin_still_works` fails.
4. Make the check fire on every type declaration → `an_ordinary_type_declaration_is_unaffected` fails, and much else besides.

Revert each, then `cargo build --workspace` before any further probe.

- [ ] **Step 7: Verify and commit**

```bash
cargo clippy --workspace --all-targets -- -D warnings 2>&1 | tail -3
cargo fmt --all --check
```

Add a `CHANGELOG.md` entry under `[Unreleased]` recording this as a **language-surface change**: the six names, both forms, and that a declaration under one of these names was already unusable in any annotation.

*(Corrected 2026-08-09: dropped "and that nothing which previously worked breaks" — false. Construction, pattern matching, and inferred local bindings for a type declared under one of these six names worked before this change and do not after; see the spec's §3.2. File the entry as a breaking change, cross-filed under both `### Added` and `### Changed` per this changelog's own precedent for a behaviour change, not only under `### Added`.)*

```bash
git add crates/nova-resolver/src/lib.rs crates/nova-typeck/src/check.rs CHANGELOG.md
git commit -m "feat(resolver): reserve the built-in type names

A user type named Int, Float, Bool, Char, String or Future compiled and
was then unusable: convert_ty resolves those names to the built-in before
reaching resolve_type, so every annotation spelling the name meant the
built-in, and the only diagnostic came at the use site -- reporting the
same type on both sides of a mismatch. Declaring one is now E0089.

Nothing that worked breaks. Such a declaration was already unreferrable,
so any program using one already failed; this moves the diagnostic to the
declaration, where the user can act on it.

The reserved list lives in nova-resolver, which nova-typeck already
depends on, so the check and convert_ty's table share one source of truth
instead of two hardcoded copies.

Generic parameters and trait names are deliberately untouched and pinned
by tests: a generic parameter resolves before the built-in table, so
`fn f<Int>(x: Int)` shadows coherently rather than breaking, and traits
are a separate namespace.

Co-Authored-By: Claude Opus 5 <noreply@anthropic.com>"
```

**Note, 2026-08-09: this suggested message was committed essentially verbatim as `4ad1187`, false
claim included** ("Nothing that worked breaks. Such a declaration was already unreferrable, so any
program using one already failed…"). A commit message cannot be edited after the fact without
rewriting history, so it is left as the historical record of what this plan asked for and what was
believed true at the time; `CHANGELOG.md`'s corrected `[Unreleased]` entry is the correction of
record for a reader of the repository, and this note is the correction of record for a reader of
this plan. See the spec's §3.2 for the mechanism that was missed.

---

## Plan Self-Review

**Spec coverage.** §3's check → Steps 4; §3's placement argument → Step 4's site choice; §3.1's reject-rather-than-disambiguate → the message's second half, asserted in Step 2; §3.2's no-op claim → Step 5's instruction to report any pre-existing test that breaks; §4's three non-goals → three tests in Step 2 plus mutations 2 and 3; §5's test list → Step 2 in full; §5's mutation table → Step 6; §6.1's drift risk → the shared const plus `every_reserved_name_really_is_a_builtin_type`; §6.2's `Future`-is-not-in-the-prim-table trap → the const's doc comment and the `Future<Int>` special case in the drift test; §6.3's scope-creep risk → mutations 2 and 3; §7's DoD → Steps 5 and 7.

**One spec item I could not fully satisfy, flagged rather than hidden.** §6.1 asks that the reserved list and `convert_ty`'s tables "cannot drift apart unnoticed". The shared const removes one copy, and the drift test catches a name *leaving* the table. A **seventh built-in added without being reserved** is not catchable from a fixed list, so Step 4 requires a pointer comment at the table and Step 2's test comment says so explicitly. If the implementer finds a way to make that direction structurally impossible — an enum with a compiler-enforced exhaustive match, following this project's `rt_funcs!`/`builtins!` precedent — that is better than the comment, and worth the extra lines.

**Type consistency.** `RESERVED_TYPE_NAMES` is declared once in Step 4 and consumed by two tests in Step 2 under the same path. `E0089` appears in Steps 2, 4 and 6 with no other code used.

**Known weakness.** `a_generic_parameter_named_for_a_builtin_still_works` asserts absence of diagnostics, which is weaker than the non-goal it defends — Step 2 says so and asks for a behavioural assertion or an explicit report that no layer can provide one.
