# Reserve the built-in type names — Design

**Status:** approved 2026-08-09. Follow-up 3 from Phase 2.3a's whole-branch review, widened by probing.

**Base:** `main` at `3d5c4c4` (async core and the task-identity fix merged and pushed; 819 tests, 8 deliberately ignored).

---

## 1. Why this, and why now

A user can declare a type whose name is a built-in type's, and the declaration is then **permanently
unusable** — `convert_ty` resolves those names to the built-in *before* it ever reaches
`resolve_type`, so every annotation spelling the name means the built-in. The declaration compiles;
only the use site fails, and it fails with a message naming the same type on both sides:

```nova
record Bool { v: Int }
fn get(b: Bool) -> Int { b.v }
```
```
error[E0014]: cannot access field `v` on `Bool`
error[E0010]: argument to `get` has type `Bool` but `Bool` was expected
```

`record Int { v: Bool }` compiles clean on its own, with no diagnostic at all until something tries
to use it.

The whole-branch review raised this for `Future` only, on the reasoning that `Future` is the first
*generic* compiler-known name and therefore the first that can print identically to a user type "of
the same name and arity". **That reasoning is wrong, and the probe above is the counter-example:**
a nullary user record named `Bool` collides with the nullary primitive `Bool` exactly. Every
built-in type name already has this defect. `Future` is the newest instance, not the first.

Reserving a name is a language-surface decision that becomes a breaking change once user code
spells it, which is the argument for doing it now rather than after `std/io` exists.

## 2. Probe table

Measured against `3d5c4c4`.

| Claim | Measured | Consequence |
|---|---|---|
| Only `Future` collides identically | **False.** `record Bool { … }` + `fn get(b: Bool)` yields ``argument to `get` has type `Bool` but `Bool` was expected`` | The fix is the whole class, not `Future` alone |
| `Unit` is a nameable type name | **False.** `fn f(x: Unit)` is `E0001 cannot find type 'Unit'` | The reserved list is **six**, not seven |
| A generic parameter named `Int` is broken too | **False.** `fn f<Int>(x: Int) -> Int { x }` compiles and `f(3)` returns 3 — `convert_ty` checks generics *before* the built-in table, so the shadowing is coherent | Generic parameters are out of scope; see §4 |
| A type alias can reach this | **False.** `type String = Int` is already `E0900`, unsupported in this compiler | No check needed, but placement should let aliases inherit it |
| A trait named `Int` shadows the type | **False.** `trait Int { fn m(self) -> Int }` compiles, the return type resolves to the primitive, and the trait stays usable as a bound — traits are a separate namespace | Trait names are out of scope; see §4 |
| There is no sibling check to model on | **False.** A duplicate type name is `E0002 duplicate definition of type`, raised where names are collected | The new check belongs beside it |
| Highest allocated code in the `E00xx` band | `E0088` | `E0089` is free |

## 3. The change

**Reject a `record` or sum-`type` declaration whose name is one of six built-in type names —
`Int`, `Float`, `Bool`, `Char`, `String`, `Future` — with a new `E0089`.**

The check goes in `crates/nova-resolver/src/lib.rs`, where type names are collected and where the
sibling `E0002` duplicate-definition check already lives: the sum arm near `:877` and the record arm
near `:939`. Placing it there rather than in typeck means it fires at the declaration, which is the
only place the user can act on it, and means a type alias would inherit it for free if aliases ever
stop being `E0900`.

The message must say two things, because the second is the part a user cannot otherwise discover:
that the name belongs to a built-in type, and that a declaration under this name could never be
referred to — every annotation spelling it resolves to the built-in.

### 3.1 Why reject rather than improve the diagnostic

Making `display_ty` distinguish the built-in from a user type would fix the identical-printing
message, and nothing else. The declared type would remain permanently unusable in type position;
the clearer error would explain a permanent uselessness at the use site instead of preventing it at
the declaration. Rejecting is the smaller, more honest fix.

### 3.2 Nothing that works breaks

Every program that declares such a type is **already broken** — it simply fails later and worse. A
declaration alone compiles today, so a program can only be affected if it also uses the type, and
any such use already fails. This is a no-op for working code, which is why reserving all six costs
no more than reserving `Future`.

## 4. Non-goals, and why each is deliberate

Each is pinned by a test, so a later tidy-up cannot quietly widen the rule:

- **Generic parameters stay legal.** `fn f<Int>(x: Int) -> Int { x }` works correctly — measured.
  `convert_ty` resolves generics before the built-in table, so the parameter genuinely shadows the
  primitive and the function behaves as written. Rejecting it would be a breaking change to
  something that is not broken, unlike the declaration case which is a no-op for working code.
- **Trait names stay legal.** Traits are a separate namespace; `trait Int` does not shadow the type
  and remains usable as a bound — measured.
- **Value names are untouched.** A different namespace again, and not the reported defect.
- **Type aliases need no check** because they are already `E0900`.

## 5. Testing

- One case per reserved name, for `record` and for sum `type` — twelve declarations, each rejected
  with `E0089`. A list-driven test is fine; asserting the code alone is not, because the message's
  second half (that the declaration could never be referred to) is the part carrying the value.
- **The three non-goals, each asserted positively:** a generic parameter named `Int` still compiles
  *and returns the right answer*; `trait Int` still compiles; a value named for a built-in is
  unaffected. Compiling is not enough for the generic case — it worked correctly before and must
  still work correctly, or the "coherent shadowing" claim in §4 is unfounded.
- **`E0001 cannot find type` is unaffected** for a genuinely unknown name. The new check must not
  swallow the existing not-found path.
- The full suite stays green at 819 + the new tests, with the 8 ADR 0010 tests still ignored.

Mutation targets, named here rather than left to review:

| Mutation | Must be killed by |
|---|---|
| Drop one name from the reserved list | that name's two declaration cases |
| Reject generic parameters too | the generic non-goal test |
| Reject trait names too | the trait non-goal test |
| Fire on every type name, not just reserved ones | any ordinary `record`/sum declaration in the suite |

## 6. Risks

1. **The reserved list must match `convert_ty`'s table, and there are two such tables.** Verified
   at `3d5c4c4`: the nullary table is `crates/nova-typeck/src/check.rs:2437` (`convert_ty`) and
   `:5212` (`qualifier_self_ty`). A name reserved here but absent there — or the reverse — puts the
   two out of step. The 2.2c precedent is that this project has already shipped a miscompile from
   two lookup sites drifting apart, so the list needs a single source of truth or a test pinning
   them together. **Re-derive these line numbers before relying on them**: the figures the
   whole-branch review carried (`:2394`, `:5080`) were already stale by the time this spec was
   written, because the async work grew that file.
2. **`Future` is in the list but is not in the nullary `prim` table** — verified: it is handled at
   `check.rs:2421`, ahead of the table at `:2437`, because it is the only built-in type name taking
   a type argument, and separately at `:5221` in `qualifier_self_ty`. **A check deriving its list
   from the `prim` table alone would silently omit `Future`** — the name this follow-up was
   originally about.
3. **Scope creep toward generic parameters.** The rule reads as "a built-in type name is never
   redeclarable", and the generic case looks like an omission rather than a decision. §4 and its
   test are what keep it a decision.

## 7. Definition of done

- All six names rejected in both declaration forms, with a message naming the built-in and stating
  the declaration could never be referred to.
- All three non-goals still work, each pinned by a test, with the generic case asserting behaviour
  rather than mere compilation.
- The reserved list and `convert_ty`'s tables cannot drift apart unnoticed.
- Suite green, clippy `-D warnings` and `cargo fmt --check` clean.
- `CHANGELOG.md` records the new rejection as a language-surface change.
