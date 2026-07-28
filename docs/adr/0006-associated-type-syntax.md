# ADR 0006 — Associated-type projection syntax: `Self::Item`, not `Self.Item`

## Status

Accepted (2026-07-28). Phase 2.2c, alongside the first associated types
(`trait Iterator { type Item }`).

This ADR exists because the decision **deviates from `nova-spec/`**, which
`agent.md` treats as binding: a deviation is only legitimate once it is recorded
here and the spec is corrected to match.

## Context

`nova-spec/20-STDLIB.md:95` specifies the iterator protocol as:

```
fn next(self) -> Option<Self.Item>
```

with a **dot** between `Self` and the associated type. Implementing associated
types required committing to one spelling, because the projection appears in
every signature that mentions an associated type — the trait declaration, each
impl, and every generic function bounded by the trait.

Two facts about the existing compiler, both established by probing it rather
than by reading the spec:

- **`A::B` in type position already parses.** It is rejected by *typeck*
  (`E0900: module-qualified type paths are not supported yet`), not by the
  grammar. So `::` costs zero parser work — the path form is already accepted
  and only needs a meaning.
- **`Self.Item` does not parse at all.** The spec's own signature, pasted
  verbatim into a trait, fails in the grammar rather than in typeck — measured
  on `a332528`:

  ```
  error[P0001]: expected `fn` (in function signature), found `<`
  ```

  A `P`-prefixed code is a parser error, so this never reaches type checking.
  (The design doc previously quoted this as ``expected `fn`, found `.` `` — the
  code and the layer are right, the token in the message is not: recovery
  reports the position of the `<` in `Option<`, not the offending `.`.)
  Supporting the dot would mean adding new type-position syntax.

## Decision

Nova spells an associated-type projection with `::`:

```nova
trait Iterator {
    type Item
    fn next(mut self) -> Option<Self::Item>
}

fn first<I: Iterator>(mut it: I) -> Option<I::Item> { it.next() }
```

`nova-spec/20-STDLIB.md` is corrected to match.

Reasons, in order of weight:

1. **`::` is free and `.` is not.** `Self::Item` reuses a path form the parser
   already produces; `Self.Item` needs new grammar. Spending parser work to
   arrive at the weaker option is not defensible.
2. **`::` is already Nova's "reach into a type" operator.** `P::new()` and
   `T::default()` are associated *functions*, resolved through the same
   two-segment path. Using `.` for associated *types* would mean two spellings
   for one idea, on the same types, differing only by what kind of member is
   named.
3. **`.` reads as field access**, which is what it means everywhere else in the
   language — `rec.f`, `self.v`. A dot that sometimes means "project a type out
   of a trait bound" and usually means "load a field" is a worse teaching story
   than a colon that always means "reach into a type".

## Consequences

- **Zero parser cost, as predicted.** Resolution is entirely in typeck: a
  two-segment path whose first segment names a generic parameter or `Self` is
  resolved against that base's bounds and becomes `Ty::Assoc`. Everything else
  keeps reporting `E0900`, unchanged.
- **The spelling is uniform across positions.** `Self::Item` in a trait,
  `I::Item` in a bounded function, and `Self::Item` in an impl all mean "the
  `Item` of the trait that constrains this base" and all parse identically.
- **One position is excluded, for coherence rather than syntax**: a projection
  may not appear in an impl's self type (`impl<T: It> Tr for W<T::Item>`).
  Impl selection recovers an impl's type arguments by structural matching and
  cannot invert a projection, so such an impl is unselectable *and* invisible
  to overlap checking. See the design doc §5.1.
- **`Self` is no longer usable as a type-parameter name** (`E0076`). It had been
  accepted, which allowed `impl<Self: It> W<Self>` — where `Self::Item` resolved
  against an ordinary parameter that merely happened to be named `Self`, rather
  than against the impl's self type. Two meanings for one token in one scope
  would have propagated through every normalization seam and every diagnostic
  that prints `Self`.

## Alternatives considered

- **`Self.Item`, as the spec wrote it.** Rejected on all three counts above.
  Nothing recommends it except that it was written down first. Note that
  associated *functions* (`P::new()`, `T::default()`) were implemented in
  Phase 2.1, after this spec text was written, so the dot was not chosen in
  preference to an already-working `::` — it simply predates it.
- **Both spellings, with `.` as an alias.** Rejected: two ways to write one
  thing, no gain, and every error message and formatter would have to pick one
  anyway.
- **A dedicated keyword or bracket form** (`Item of I`, `I[Item]`). Rejected as
  novelty with no precedent in the language and no advantage over a path.

## References

- Design doc: `docs/superpowers/specs/2026-07-28-associated-types-design.md`
  (§2 on what turned out not to be a prerequisite, §3 on this decision, §5.1 on
  the impl-self-type exception)
- Plan: `docs/superpowers/plans/2026-07-28-associated-types.md`
- Spec corrected: `nova-spec/20-STDLIB.md`
- Related: ADR 0004 (stdlib compile model — why `std/*` is Nova source compiled
  as implicit modules), ADR 0005 §1 (mutable receivers — whose open gap
  `Iterator::next(mut self)` forced closed)
