# ADR 0007 — A bound on a record's type parameter is a resolution scope, not a constraint

Two decisions taken to make `std/core`'s lazy iterator adapters (Phase 2.2d)
expressible. They share a file because both are concessions made to ship one
API — `v.iter().filter(f).map(g).collect()` — and both trade a property the
compiler used to advertise for the ability to write that line at all. Section 1
is about what a record *declaration* may say; section 2 is about what a trait
*method* may demand of its receiver.

Both sections are accepted, and each names a real loss rather than arguing it
away.

---

## 1. Bounds on a record's type parameters

### Status

Accepted (2026-07-29). Phase 2.2d, as the first step of the increment that adds
`for x in it` and `Iterator`'s six default methods.

### Context

A lazy `map` needs a record that holds the source iterator and the mapping
function:

```nova
pub record MapIter<I, U> { it: I, f: fn(I::Item) -> U }
```

The field type `fn(I::Item) -> U` names a projection on `I`. A projection is
resolved against the bounds of the base it is projected from — that is the whole
mechanism ADR 0006 installed — so with `I` unbounded there is nothing to resolve
`I::Item` against, and the declaration is `E0001`
(`a_projection_on_an_unbounded_record_parameter_is_still_e0001`,
`crates/nova-typeck/src/check.rs`).

The obvious fix was to write the bound:

```nova
pub record MapIter<I: Iterator, U> { it: I, f: fn(I::Item) -> U }
```

which had been rejected since Phase 2.2a with
`E0900: trait bounds on record type parameters are not supported yet` — and
rejected once *per bounded parameter*, so a two-bound record reported twice.

There is a workaround that type-checks and it is worth recording because it
looks like an answer and is not. Give the record a second parameter for the item
type:

```nova
pub record MapIter<I, A, U> { it: I, f: fn(A) -> U }
```

This declares fine — no projection, so no bound needed. It cannot be *driven*.
Nothing ties `A` to `I::Item`, so `impl<I: Iterator, A, U> Iterator for
MapIter<I, A, U>` has no way to say "and `A` is `I`'s item type"; Nova has no
equality constraints (no `where I::Item == A`), so the impl can only hope the
caller passed a consistent triple. The type that results is not the type that
was wanted, it merely compiles.

### Decision

**A bound on a *record*'s type parameter is accepted, and it is a resolution
scope rather than a constraint.** It exists so that a field type may name a
projection on that parameter. It is not checked when the record is constructed.

**Records only.** A bound on a *sum type*'s parameter keeps reporting `E0900`
(`a_bound_on_a_sum_type_parameter_is_still_e0900`,
`crates/nova-typeck/src/check.rs`). Nothing in this increment needs one, and
leaving the rejection in place halves the surface this decision has to defend.

### Why it is not enforced

Not "not yet got round to" — there is no place to put the check that would fire
reliably:

- **`MakeRecord` carries no type arguments.** A record literal lowers to a
  `MakeRecord` whose operands are the field values and nothing else. The
  instantiation (`MapIter<Int, …>` rather than `MapIter<VecIter<Int>, …>`)
  survives only in the enclosing `Expr.ty`, which lowering discards.
- **MIR erases records to `Ptr`.** By the time monomorphization is looking at
  bounds, `hir::Ty::Record` has become `MirTy::Ptr`; the type arguments are gone.
- **The natural home has no field for it.** `nova-hir`'s `RecordType` keeps only
  a generic *count* (`generics: u32`, `crates/nova-hir/src/lib.rs`) and no
  bounds, and `nova-mir`'s `mono` checks bounds solely against a function's own
  `bounds` (`for (i, bounds) in func.bounds.iter()`,
  `crates/nova-mir/src/mono.rs`). Neither position can enforce anything about a
  record's parameters.
- **Monomorphization visits only what `main` reaches.** So even a partial check
  bolted on would fire for instantiations reachable from `main` and stay silent
  for the rest — a rule that holds *sometimes* is subtler and worse than one that
  visibly does not hold at all.

This is Phase 2.2a's assessment of the same question, re-affirmed rather than
revisited.

### Why that is safe — three cases, each measured, and they do not collapse

This is the part of the decision that has been got wrong twice, and the way it
was got wrong is worth more than the answer. Earlier drafts of the plan and of
`nova-spec` each claimed **one uniform** answer to "what happens if you
instantiate a record's bounded parameter with a type that does not satisfy the
bound": first `E0014`, then `E0013`. Both were wrong.

The second is the instructive one. `E0013` **was** genuinely measured — but on a
*function's* bound (`fn f<I: It>`, pinned by
`a_trait_bound_needs_an_impl_that_fits_structurally_not_just_by_head`) and on a
record whose field types do not name a projection at all. It was then transcribed
onto `MapIter`, whose field type does. **Different record shape, different
diagnostic.** Generalizing a measurement past the shape it was taken on is the
specific error, and it is the second time this project's documents have made it.

The real behaviour, all three probed on the shipped compiler and each now pinned
by a test in `crates/nova-cli/tests/run_tests.rs`:

**Case 1 — the bound's purpose is to make a projection resolvable in a field
type.** That is `MapIter { it: I, f: fn(I::Item) -> U }` and
`FilterIter { it: I, keep: fn(I::Item) -> Bool }` — the only two shapes this
increment ships. A wrong instantiation is caught **at construction**, even
though the value is never driven:

```nova
let m = MapIter { it: 5, f: |x| x }
```

```
error[E0079]: `Int::Item` is still an unresolved associated type after
              instantiating `main`; no impl in scope binds it for this Self type
```

This is **not** the bound check. It is the surviving-projection check built in
Phase 2.2c Task 7: substituting `I := Int` leaves `Int::Item` standing in `f`'s
declared type, no impl binds `Item` for `Int`, and monomorphization refuses it.
So for the shapes that matter the outcome is *earlier and stronger* than the
decision above promises — the bound was meant to leave construction
unconstrained and let only *use* fail, but a field that actually names the
projection enforces it at construction anyway, by a mechanism with nothing to do
with the bound. Pinned by
`a_wrong_instantiation_of_a_projection_shaped_record_is_e0079_at_construction`,
which deliberately does not drive the iterator, because firing at construction
is the property.

**Case 2 — the bound reaches no field type, but is exercised through a bounded
impl method.** `record Boxed<K: Hash + Eq, V> { k: K, v: V }` with
`impl<K: Hash + Eq, V> Boxed<K, V> { fn key(self) -> K }`. Building
`Boxed { k: NoHash { … }, v: 7 }` is accepted; instantiating `key` is not, one
diagnostic per unsatisfied bound:

```
error[E0013]: trait bound `NoHash: Hash` is not satisfied when instantiating `Boxed_K_V.key`
error[E0013]: trait bound `NoHash: Eq` is not satisfied when instantiating `Boxed_K_V.key`
```

The bound on the **impl** is real, exactly as `Map`'s and `Set`'s always were.
Pinned by `an_unused_record_bound_is_still_enforced_through_a_bounded_impl_method`,
which asserts the bound *spelling* and not merely the code, since `E0013` is the
code for every unsatisfied bound in the language.

**Case 3 — the bound reaches no field type and is never exercised.** The same
`Boxed`, the same non-conforming `K`, but only the *unbounded* field is read, so
no bounded method is ever instantiated:

```nova
let b = Boxed { k: NoHash { z: 1 }, v: 7 }
println("${b.v}")     // prints 7
```

**Compiles, runs, prints. No diagnostic anywhere.** This is the residual hole,
and it is stated here plainly rather than left to be inferred, because case 2
does *not* always save you. Pinned as accepted by
`a_record_bound_no_field_type_uses_is_silently_accepted_when_never_exercised`,
which asserts success on the exact stdout — a test that merely omitted an error
assertion would also pass if the program broke for an unrelated reason.

So the sentence an earlier draft of this decision carried — "a bogus
instantiation is never silently useless" — is **false**. Case 3 is exactly that.
Three cases, three answers; there is no single verdict to write.

### Consequences

- **A record bound looks like a constraint and is not one.** That is the risk,
  named rather than softened. A reader who writes
  `record Holder<T: Display> { v: T }` will reasonably expect
  `Holder { v: SomeNonDisplay }` to be rejected, and — case 3 — it is not.
- **It belongs to a family this project keeps finding and fixing:** impl-level
  `const`s parsed and discarded, record field visibility parsed and never
  enforced, `pub` on methods accepted and ignored. All "accepted and quietly
  ignored" defects, all found by probing rather than predicted. This one differs
  only in being *chosen* and written down, which is the entire purpose of this
  file.
- **The mitigation is documentation in three places, not code**: this ADR, the
  comments above `MapIter` and `FilterIter` in `std/core/lib.nova`, and the
  `CHANGELOG` entry. That is a weaker mitigation than a check and is accepted as
  such.
- **A future increment may replace it with real enforcement.** The prerequisite
  is concrete: `MakeRecord` would have to carry the record's type arguments
  through lowering, at which point the check has somewhere to live. That is a
  much larger change than this increment could carry, and Phase 2.2a's objection
  to attempting it stands. If it ever lands, case 3's test is the one that has to
  be deliberately changed — which is why it exists.
- **`E0900` still rejects the sum-type form**, so widening this decision is also
  a deliberate act with a test to change.

### Alternatives considered

- **Eager `map`/`filter` returning `Vec`.** No adapter record, so no projection
  in a field type, so no bound needed and this whole question never arises.
  Rejected: it allocates a fresh `Vec` per stage, so a two-stage chain over *n*
  elements allocates twice rather than not at all, and it makes `map` on an
  infinite or expensive source unusable. Laziness is the property the adapters
  exist for.
- **Reject a bound on a parameter that no field type uses.** This would close
  case 3 by making the inert form an error, keeping the bound legal only where
  it does something. Declined: such a bound is harmless, and the check is a
  second analysis over every record declaration — new code, new diagnostic, new
  tests — for no user benefit, since the programs it would reject are exactly
  the programs that already work.
- **Thread type arguments through `MakeRecord`** and check the bound properly.
  The honest answer, and much larger: lowering, the MIR instruction, and
  monomorphization all change, and 2.2a's objection (records are erased to `Ptr`,
  so the arguments would have to be carried purely for checking) stands. Left as
  the migration path above rather than rejected outright.

---

## 2. Why `Iterator`'s four consumers take plain `self`

### Status

Accepted (2026-07-29). Phase 2.2d Task 4, during implementation — this is a
decision the plan did not anticipate, and it amends ADR 0005 §1. That amendment
is appended to ADR 0005 itself; this section is the fuller account.

### Context

`fold`, `count`, `any` and `collect` were specified as `mut self`, and the
reasoning was straightforward: each must advance the iterator, and advancing is
mutation, so the receiver should declare it.

But ADR 0005 §1 deliberately rejects a **temporary** receiver for a `mut self`
method — `place_root` classifies a call result `NotAPlace`, and the rule requires
`Mutable`. So the exact form this increment was designed around did not compile:

```
error[E0060]: `collect` mutates its receiver, which cannot be a temporary
```

for `v.iter().filter(f).map(g).collect()`. The adapters chained fine — `map` and
`filter` take plain `self` and return a new record — so it was only the
consumers that broke the chain, and only at the last call.

### Decision

**The four consumers take plain `self` and open with `let mut it = self`,**
driving `it` instead of `self`.

**`next` remains the only `mut self` method** in the trait. It is the one a
caller invokes repeatedly on a binding they hold, so requiring that binding to be
mutable is exactly right, and keeping it `mut self` is what stops this decision
from being "iterators no longer need `mut`".

### What this gives up in ADR 0005

The *mechanism* is intact. `mut self` still reports `E0060` on a temporary and on
an immutable local, and that was verified on three separate routes — an inherent
method, a trait method, and `next` itself. Nothing about how the rule works
changed, and no route into it was removed.

The *property* ADR 0005 advertised is partly given up. Its Consequences promised:

> every std API that mutates must declare `mut self` — an accessor that forgets
> it will not compile

Four shipped `std/core` consumers now mutate, declare plain `self`, and compile.
The self-enforcement that sentence claimed is no longer true, and ADR 0005 has
been amended in place to say so.

That is deliberately weaker than the wording this decision originally carried,
which was that it "routes around the rule rather than weakening it." That
sentence was retracted in `f7a1308`: it is too strong, because the promise ADR
0005 made was about what std *can* do, and this changes what std can do.

### Why the collateral loss was unavoidable — and it is a real loss

`place_root` returns three verdicts (`crates/nova-typeck/src/check.rs`,
`require_mutable_place` and its caller): `Mutable`, `ImmutableLocal(name)` and
`NotAPlace`. A `mut self` receiver rejects the latter two.

**Only `NotAPlace` — the temporary — blocked the advertised chain.** Dropping
`mut self` relaxes both. Losing `ImmutableLocal` was **collateral**, because Nova
has no third receiver form that accepts a temporary while still rejecting an
immutable local, and inventing one would be a new receiver kind layered over
`place_root` rather than a use of it.

The observable consequence, measured:

```nova
let it = Cursor { n: 0, max: 3 }   // no `mut`
it.count()   // 3
it.count()   // 0   <- advanced, no diagnostic
```

Under `mut self` the first call alone was `E0060`, and an immutable binding
silently mutated is the exact shape ADR 0005 §1's Context cites as the reason the
receiver rule exists at all. **This loss is real and it was not the goal.**

### Why it is safe

`let mut it = self` **aliases** rather than copies. This compiler has no move
semantics anywhere, and records are heap objects passed by pointer
(`hir::Ty::Record` maps to `MirTy::Ptr`), so `it` names exactly the storage
`self` pointed to — the caller's iterator genuinely advances through a consumer,
rather than the consumer advancing a private copy and the caller seeing nothing.

This was measured rather than assumed, because a silent value-copy would look
identical to every test that only checks a consumer's *return* value. The
decisive measurement gives the iterator a record that exposes its own cursor
field, so that *aliased*, *copied* and *fully-scanned* are three distinguishable
outcomes: after `a.any(|x| x > 1)` on a `Cursor { n: 0, max: 5 }`, reading `a.n`
back gives `3` — not `0`, which would mean a copy, and not `5`, which would mean
the short-circuit was lost. Pinned by
`an_iterators_own_storage_still_advances_when_a_consumer_takes_plain_self`.

Two further routes agree, and each fails differently if the aliasing does not
hold. A source whose `next` prints shows the elements really being pulled
*through* the consumer
(`iterator_any_short_circuits_and_does_not_scan_past_the_first_match`), and the
gate fixture calls `count()` twice on the same binding — `3`, then `0`
(`exhaust_consumed`, `tests/runtime/iterator.nova`), which a copying consumer
would report as `3` both times.

**If a future change gives Nova move semantics, re-derive this.** The whole
argument rests on the copy not happening. Nothing in the type system says so;
it is a property of the current lowering.

### What bounds the loss

Three things, all of them limits that already existed:

- **`next` stays `mut self`.** Driving an iterator by hand — the `while` plus
  `match` form, and any user implementor's own code — still requires a mutable
  binding or a `mut` parameter.
- **No new iterator state becomes reachable.** A `mut` binding could always be
  partially consumed and then reused, so nothing is observable now that was not
  observable before; what changed is only which receivers these four methods
  accept syntactically.
- **The compiler already launders a mutation this way, in its own `for`
  desugar.** `check_for_iterator` binds its hidden iterator as
  `let mut __it = <expr>` regardless of the source expression's mutability, so
  `for x in <an immutable local>` already advanced that local before this trait
  had a single default method. `Iterator`'s consumers are the first *library* use
  of a pre-existing escape hatch, not a new one — and the `for` desugar is the
  precedent this decision is modelled on, for the same reason: it needs a mutable
  place to call `next` through and makes one rather than demanding the caller
  provide it.

### Consequences

- **A consumer now accepts a temporary, so `v.iter().count()` compiles.** That
  is the intent, and it is the whole reason for the change.
- **A consumer also accepts an immutable local and advances it silently.** That
  is not the intent. Both follow from the same relaxation and only the first was
  wanted; there is no version of this change that gets one without the other.
  Pinned as accepted, current behaviour by
  `iterator_any_short_circuits_and_does_not_scan_past_the_first_match`.
- **`std/core`'s `Iterator` doc comment carries the same explanation**, above
  `fold`, because that is where the next person to add a seventh method will
  look — and the decision they need is "plain `self`, and rebind".

### Alternatives considered

- **Keep `mut self` and require an intermediate binding.** The caller writes
  `let mut it = v.iter().filter(f).map(g)` and then `it.collect()`. Rejected: the
  chained form is the API's advertised shape and the reason the increment exists;
  a std API that cannot be called the way its own documentation writes it is not
  finished.
- **A third receiver form** that accepts a temporary while rejecting an immutable
  local. This is the only thing that would have avoided the collateral loss.
  Rejected as far too large for this increment — a new receiver kind means new
  syntax, a fourth `place_root` verdict, and a rule a reader has to hold in their
  head alongside `self` and `mut self` — and it is the migration path if the
  silent-advance case ever proves to matter in practice.
- **Have the consumers take `mut self` and construct their own source.** Not
  expressible: a consumer receives an already-built adapter tower and cannot
  rebuild it.

## References

- Plan: `.superpowers/sdd/2026-07-29-iterator-finishing/`
- Section 1's three cases are pinned in `crates/nova-cli/tests/run_tests.rs`;
  the increment's end-to-end gate is `tests/runtime/iterator.{nova,stdout}`
- Related: ADR 0005 §1 (mutable receivers — amended by section 2 above),
  ADR 0006 (`::` projection syntax, which section 1's field types depend on),
  ADR 0004 (stdlib compile model — why `std/*` is Nova source)
- Spec: `nova-spec/20-STDLIB.md` (`Iterator`'s shipped signatures)
