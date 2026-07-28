# Associated types (and the `Iterator` trait that motivates them) — design

> Date: 2026-07-28. Branch: `assoc-types-iterator`, base `eeabcd4`.
> First increment of the `Iterator` work. Phase 2.2 (`std/collections` + `std/strings`) is
> complete and pushed.

## 1. Scope, and what is deliberately *not* here

`Iterator` as the spec writes it needs four things Nova does not have. This increment does
**one** of them — associated types — plus the smallest real consumer that proves them.

| Needed for full `Iterator` | This increment |
|---|---|
| `type Item` in a trait, and `Self::Item` / `I::Item` as a type | **yes** |
| A `mut self` trait method that is actually *checked* | **yes** (§6 — `next` forces it) |
| `for x in it { … }` desugar | no — next increment |
| `map` / `filter` / `collect` / `fold` default methods | no — needs laziness decisions |
| `impl Trait` return position | **not needed at all** (§2) |
| Tuples, for `Map::iter()` yielding pairs | no — `Vec`/`Set`/`chars` do not need them |

Decomposed this way on purpose: associated-type projection is the invasive change, and
bundling it with an undesigned iteration protocol would make a review unable to separate
"is the type system sound" from "is this the right API". Once this lands, the for-loop
desugar and the default methods are purely additive.

## 2. Two of the four "prerequisites" turned out not to be prerequisites

Both established by probing the compiler rather than reading the spec:

- **`impl Trait` return position is unnecessary.** The spec writes
  `fn iter(self) -> impl Iterator<Item = &T>`, but `&T` does not exist either, so that
  signature is unachievable regardless of `impl Trait`. Naming the concrete iterator —
  `fn iter(self) -> VecIter<T>` — needs nothing new.
- **`A::B` in type position already parses.** It is rejected by *typeck*
  (`E0900: module-qualified type paths are not supported yet`), not by the grammar. So the
  projection syntax costs **zero parser work**.

## 3. Syntax: `Self::Item`, a deliberate deviation from the spec

`nova-spec/20-STDLIB.md:95` writes `fn next(self) -> Option<Self.Item>` — a **dot**. Nova will
use `::` instead, and the spec will be corrected. (**Corrected citation**: this said `:93`,
which is the `pub trait Iterator {` line; the dot signature is two lines down.)

Reasons, in order of weight:

1. **`::` is free; `.` is not.** `Self::Item` reuses a path form the parser already accepts.
   `Self.Item` fails in the grammar today and would need new type-position syntax. **Corrected
   message**: this claimed `P0001: expected 'fn', found '.'`; the actual output is
   `error[P0001]: expected \`fn\` (in function signature), found \`<\`` — same code, same layer
   (a `P` code is a parser error, so it never reaches typeck), but recovery reports the `<` of
   `Option<` rather than the offending `.`.
2. **`::` is already Nova's "reach into a type" operator** — `P::new()` and `T::default()` are
   associated *functions*. Using `.` for associated types would mean two spellings for one
   idea, on the same types.
3. `.` reads as field access, which is what it means everywhere else in the language.

This needs an **ADR** (deviations from `nova-spec/` are binding per `agent.md`), in the style
of ADR 0004 and 0005.

## 4. Representation: `Ty::Assoc`, normalized at three seams

```rust
Ty::Assoc { on: Box<Ty>, assoc: DefId }
```

`assoc` is the associated type's **own** `DefId`, under a new `DefKind::AssocType` — exactly as
trait methods already get one. Nothing depends on string comparison after resolution.

**Corrected.** This section originally specified `{ on, trait_id: DefId, index: u32 }`, where
`index` was the position in the trait's associated-type list and `display_ty` would look the
name up there. That does not work: `display_ty` (`crates/nova-typeck/src/lib.rs:35`) takes only
`defs: &Definitions` and cannot see the trait table, so it could not turn an index back into a
name. Giving the associated type its own `DefId` makes `display_ty` work unchanged via
`defs.def(*assoc).name`, drops the index entirely, and still keeps `Ty` free of a `String`.
Implemented this way in Task 1; recorded here so the design doc matches the code.

### 4.1 Why normalization, and not a constraint queue

`crates/nova-typeck/src/infer.rs` is a 210-line Robinson unifier whose entire state is
`vars: Vec<Option<Ty>>`. It has **no** access to the impl table and **no** obligation queue, so
a projection reaching `unify` could be neither normalized nor deferred. Rather than restructure
the unifier, the projection is resolved wherever the impl table *is* in scope:

| Seam | What it resolves | Why it must be there |
|---|---|---|
| `check.rs`, after `icx.apply` | `Assoc { on: VecIter<Int> }` → `Int` | ordinary use sites: `it.next()` must type as `Option<Int>` |
| `check_impl_conformance` | the impl writes `Option<T>`; the trait declares `Option<Self::Item>` | otherwise every impl of a trait with an associated type is a spurious `E0072` |
| `mono.rs`, after `subst` | `Assoc { on: Param(i) }` once `Param(i)` is concrete | generic bodies: `fn count<I: Iterator>(it: I)` |

`unify` gains exactly one arm: two `Assoc`s unify when trait and index match and their `on`
unify. No other pairing can arise, because everything else was normalized first.

**This follows the grain of the codebase rather than fighting it.** Trait *bounds* are already
discharged at monomorphization (`E0013`, `crates/nova-mir/src/mono.rs:117`) rather than in
`check_src`. Projection resolution uses the same seam for the same reason: mono is the first
point where a generic parameter is known.

### 4.2 The case that would have forced a constraint queue cannot arise

`Assoc { on: Var(_) }` — a projection on a Self type that is still an unsolved inference
variable — is the one shape that needs deferral. It is **structurally unreachable**:
`check_method_call` (`crates/nova-typeck/src/check.rs:3626`) already rejects an uninferred
receiver with `E0011: cannot infer the receiver's type; add a type annotation`, before any
return type is computed. And a user-written `I::Item` names a generic parameter, so its `on` is
a `Param`, never a `Var`.

It is therefore an explicit internal error, not a silent wrong answer — and the reason is
recorded here so a future change to `check_method_call`'s receiver handling reveals that it
would reopen this.

### 4.3 Blast radius

Adding a `Ty` variant forces a decision in every exhaustive match over it. Enumerated so none
is discovered late:

| Site | Behaviour for `Assoc` |
|---|---|
| `Ty::subst` (nova-hir) | substitute into `on`; the *caller* normalizes afterwards |
| `InferCtx::apply` | recurse into `on` |
| `InferCtx::occurs` | recurse into `on` |
| `InferCtx::unify` | the one new arm (§4.1) |
| `display_ty` | render as `<on>::Name`, looking the name up in the trait's list — so `display_ty` needs the trait table in scope, which it has in `check.rs`; a projection printed as `Assoc(3)` in a diagnostic would be useless |
| `mir_ty` (nova-mir) | **unreachable** — a projection at codegen is a bug, not a lowering case |
| `Ty::match_pattern` | match structurally on `on` |
| `hir::self_types_overlap` | conservative: treat as overlapping unless the `on`s provably differ |
| `TyHead::of` | `None` — a projection has no head until normalized |

`mir_ty` being unreachable is the load-bearing one. It must fail loudly rather than defaulting
to a pointer, because a silent default is how a projection would reach codegen and miscompile.

## 5. Surface

```nova
pub trait Iterator {
    type Item
    fn next(mut self) -> Option<Self::Item>
}

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

impl<T> Vec<T> {
    pub fn iter(self) -> VecIter<T> { VecIter { v: self, i: 0 } }
}
```

Rules:

- **Multiple associated types per trait** are allowed — it is a list, and restricting it to one
  would be arbitrary.
- **Bounds on an associated type** (`type Item: Display`) are rejected with **`E0900`**. This
  matches how record and sum type-parameter bounds are handled: this project rejects a bound it
  cannot enforce rather than accepting one that enforces nothing, which is the defect the
  Phase 2.2a debt branch existed to remove.
- **No defaults** (`type Item = Int` in the trait). YAGNI, and it interacts with conformance
  checking in ways nothing needs yet.
- The implementor is **generic** — `impl<T> Iterator for VecIter<T> { type Item = T }`. Binding
  `Item` to the impl's own parameter is what exercises `subst` and therefore the mono seam; a
  monomorphic `type Item = Char` would leave that path untested while appearing to pass.

`VecIter` lives in `std/collections` beside `Vec`; `Iterator` lives in `std/core`. Std modules
are mutually visible, so this is a convention rather than a constraint — but it keeps the trait
with the other core traits and the iterator with the collection it iterates.

### 5.1 Cases a reader could resolve two ways, pinned

Each of these has a defensible opposite, so picking silently is how an implementation diverges
from the design without anyone noticing:

| Case | Decision |
|---|---|
| May an impl write the concrete type (`-> Option<T>`) or must it echo the projection (`-> Option<Self::Item>`)? | **Either is accepted.** Conformance normalizes both sides before comparing. Requiring the projection would be noise; requiring the concrete type would forbid a legitimate spelling. |
| Does `Self::Item` inside an impl body or signature resolve to that impl's binding? | **Yes** — inside `impl<T> Iterator for VecIter<T>`, `Self::Item` normalizes to `T`. |
| Where may a projection appear? | **Anywhere a type may appear, with one exception** — trait and impl signatures, function signatures, `let` annotations, record fields all accept one. The exception is an **impl's self type**: `impl<T: It> Tr for W<T::Item>` must be rejected. **Corrected** — this row originally claimed no position needed restricting, "extra rules for no gain". That is false, and measured: such an impl type-checks clean today, does **not** conflict with `impl Tr for W<Int>` (while the control `impl<T> Tr for W<T>` vs `W<Int>` correctly reports `E0074`), and can never be selected anyway, because `match_pattern` recovers an impl's arguments by structural matching and cannot invert a projection. So it is simultaneously dead and a hole in overlap checking. This is the same reason Rust forbids the position. Task 11 closes it. |
| `Vec::iter` — new `impl<T> Vec<T>` block, or the existing one? | **The existing one.** `std/collections` already has `impl<T> Vec<T>`; a second inherent impl on the same type is the untested configuration `std/strings` was careful to avoid. |
| Diagnostic for an impl that omits a required `type Item`, or binds one the trait never declared | **`E0070`** for the omission, **`E0071`** for the undeclared binding, with the name in the message. **Corrected** — this row originally said `E0072` for both, which was wrong in both directions: `check_impl_conformance` already runs a three-code scheme, `E0070` = missing a required item (`check.rs:1242`), `E0071` = not a member of the trait (`:1088`), `E0072` = the item exists on both sides but its shape disagrees. `E0072` must stay free for the shape case §4.1's seam 2 actually produces. |
| Is a projection *inferred backwards* — does unifying `Assoc{..}` with `Int` deduce `on`? | **No.** Projections are resolved, never solved for. Nothing here needs it, and it would require the constraint machinery §4.1 avoids. |
| May a user name a generic parameter `Self`? | **No** — rejected with **`E0076`** at every generic declaration (`fn`, `record`, `trait`, `impl`, method-level). **Added during implementation**, not in the original design: `Self` was an accepted identifier (`parse_ident` maps `Token::SelfUpper` to the plain string `"Self"`), so `generic_scope` gave a user-written `<Self>` its own entry — including in an impl, which meant `impl<Self: It> W<Self> { fn peek(self) -> Self::Item }` already type-checked, resolving `Self` as an ordinary parameter rather than the impl's self type. Two meanings for one token in one scope would have propagated through every normalization seam and every diagnostic that prints `Self`. Rejecting the name is what makes "`Self` in an impl means the impl's self type" true rather than usually-true. Zero migration cost: no `std/`, `examples/`, or `tests/` code named a parameter `Self`. |

## 6. The `mut self` trait-method gap must close here

`next` must advance the iterator, so it mutates a field, so it needs `mut self`. ADR 0005
recorded the trait-method half of the mutable-receiver rule as an **open gap** with an explicit
gate: closing it is *"a hard gate before any `mut self` trait method lands"*. `Iterator::next`
is that first method.

The gap is live — measured, not assumed:

```nova
trait Bump { fn bump(mut self) -> Int }
impl Bump for C { fn bump(mut self) -> Int { self.n = self.n + 1  self.n } }
let c = C { n: 1 }        // NOT mut
c.bump()                  // returns 2 — silently mutated an immutable binding
```

So this increment also:

- adds a `mut_self` flag to `hir::TraitMethod`;
- enforces `E0060` on trait-dispatched calls, not only inherent ones — `check_method_call`'s
  `MethodRes::Trait` arm currently dispatches to `emit_trait_call` without calling
  `check_mutable_receiver` at all;
- checks it in **impl conformance**, so an impl cannot declare `mut self` for a trait method
  that does not, or the reverse.

The existing test `trait_method_mut_self_is_not_enforced_on_immutable_receiver_known_gap` pins
today's permissive behaviour on purpose. It must **flip deliberately**: rename it, invert the
assertion to expect `E0060`, and rewrite its comment from "documents a known gap" to the
enforced rule, citing this spec. That test exists precisely so this cannot happen silently, so
editing it is the intended outcome — not a test being bent to fit.

## 7. Gate

A new `tests/runtime/assoc_types.{nova,stdout}` fixture under `nova run`, `nova build` and
`NOVA_GC_STRESS=1`, matching the three existing gates. It must cover:

1. A trait with an associated type, an impl binding it to **the impl's own generic parameter**,
   and a value obtained through it — proving `subst` carries the binding.
2. `it.next()` on a concrete `VecIter<Int>` typing as `Option<Int>` — the `check.rs` seam.
3. A **generic** function over the trait whose signature mentions the projection
   (`fn first<I: Iterator>(it: I) -> Option<I::Item>`) called at two different instantiations —
   the mono seam, and the reason one instantiation alone is insufficient.
4. A trait with **two** associated types, to prove `index` is used rather than assumed zero.
5. Iterating a `Vec` to exhaustion so `next` returns `None` at the end, and a `Vec::new()` whose
   first `next` is already `None`.

Separately, as `#[test]`s rather than fixture lines (each of these aborts or fails to compile,
which a fixture cannot contain):

6. `type Item: Display` → `E0900`.
7. An impl that omits a required `type Item` → `E0070` naming the missing name (**corrected** from `E0072`; see §5.1).
8. An impl binding an associated type the trait does not declare → `E0071` naming it (**corrected** from `E0072`).
9. `E0060` on a `mut self` **trait** method called through an immutable binding (§6).
10. An impl/trait `mut self` mismatch in either direction → a conformance error.

## 8. What this deliberately leaves broken

Stated so the next increment inherits a list rather than a surprise:

- **No `for x in it`.** Iterating means calling `next` in a `while` loop and matching the
  `Option` by hand.
- **No default methods**, so no `map`/`filter`/`collect`/`fold`.
- **No `Set` or `String` iterator** — only `Vec`. `chars()` already returns `[Char]`, which is
  indexable, so nothing regresses.
- **`Map::iter()` remains impossible** — it yields pairs and tuples are still `E0900`.
- **No `IntoIterator`.** `for` (when it arrives) will require an explicit `.iter()` unless a
  later increment adds one.
- Projections are resolved, never *inferred backwards*: `Assoc` unifying with a concrete type
  does not deduce `on`. Nothing in this increment needs it.

## 9. Risks

1. **`mir_ty` defaulting instead of failing** for `Assoc` would let a projection reach codegen
   and miscompile silently. It must be an explicit unreachable, and a test should prove a
   projection never arrives — most cheaply by asserting the fixture's generic function
   monomorphizes to concrete instances.
2. **`check_impl_conformance` is where this most plausibly goes subtly wrong**: it compares
   signatures, and now one side may contain a projection while the other is already concrete.
   Normalizing only one side yields either spurious `E0072` on every impl, or — worse — an
   accepted impl whose method has the wrong type.
3. **Flipping the `known_gap` test is a behaviour change to shipped code.** Any std code
   relying on the permissive behaviour breaks; there is none today (all nine `mut self` methods
   in std are inherent), but that must be re-checked rather than assumed at implementation
   time.
4. **`self_types_overlap` treating projections conservatively** could reject impls that are
   actually disjoint. Accepting that for now — a false `E0074` is a loud error, whereas a
   missed overlap is a silent miscompile.

## 10. Definition of done

- `type Item` parses in a trait, resolves, and appears in the trait table.
- `type Item = X` in an impl binds it; a missing or extra binding is a diagnostic.
- `Self::Item` and `I::Item` work in type position; `A::B` for a *module* path still reports
  `E0900` with its existing message.
- All three normalization seams work, each with a test that fails if that seam is removed.
- `E0060` fires for `mut self` trait methods on immutable receivers; conformance checks
  `mut self` both ways; the `known_gap` test is deliberately flipped.
- `Iterator` in `std/core`, `VecIter<T>` + `Vec::iter()` in `std/collections`.
- The §7 gate passes byte-identically under all three run modes.
- `cargo test --workspace --no-fail-fast`, `cargo clippy --all-targets --all-features -D warnings`,
  `cargo fmt --check` all green.
- An ADR records the `::`-over-`.` deviation; `nova-spec/20-STDLIB.md` is corrected to match.
- CHANGELOG records associated types, the syntax deviation, and the `mut self` enforcement
  change as the behaviour change it is.
