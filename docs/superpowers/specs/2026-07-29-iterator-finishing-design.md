# Finishing `Iterator`: record-parameter bounds, `for x in it`, and lazy adapters — design

> Date: 2026-07-29. Base `3c8127e` (`main`, Phase 2.2c pushed).
> Second and final increment of the `Iterator` work. Phase 2.2c (associated types +
> `Iterator`/`VecIter`/`Vec::iter`) is complete and pushed.

## 1. Why this increment exists, and one correction to 2.2c's design

Phase 2.2c shipped `trait Iterator { type Item   fn next(mut self) -> Option<Self::Item> }`
with `VecIter` and `Vec::iter`. It is complete and it is barely usable: iterating means a
hand-written `while` plus a `match` on the `Option`.

**2.2c's design doc claimed `for x in it` and the default methods were "purely additive" once
associated types landed. That is false, and probing found it before any code was written.** A
lazy `map` adapter needs a record field typed `fn(I::Item) -> U`, and in a *record declaration*
`I` carries no bound, so the projection cannot resolve:

```
record MapIter<I, U> { it: I, f: fn(I::Item) -> U }
  error[E0001]: no associated type `Item` on any bound of `I`

record MapIter<I: Iterator, U> { … }
  error[E0900]: trait bounds on record type parameters are not supported yet
  error[E0001]: no associated type `Item` on any bound of `I`
```

The obvious workaround — carry the item type as its own parameter, `record M<I, A, U> { it: I,
f: fn(A) -> U }` — *type-checks* but **cannot be driven**: nothing ties `A` to `I::Item`, so
`g(n.unwrap())` passes an `I::Item` to a function expecting an unrelated `A`
(`E0010: this value has type 'fn(T1) -> T2' and cannot be called with these arguments`). Nova
has no equality constraints, so there is no way to relate them.

So this increment is a **language change plus a library**, not a library alone.

### 1.1 What probing established before design (all measured on `3c8127e`)

| Question | Answer |
|---|---|
| Adapter record generic over the inner iterator | **works** |
| Default method returning `MapIter<Self, U>` | **works** |
| `impl<I: It, U> It for MapIter<I, U>` | **works** |
| `type Item = I::Item` (projection-valued binding) | **works** |
| Storing a closure in a record field and calling it | **works**, but only as `let g = self.f` then `g(x)` — `(self.f)(x)` is `E0014` |
| Projection in a record *field type* | **`E0001`** — the blocker |
| Bound on a record parameter | **`E0900`** — deliberately, since 2.2a |
| `std/core` referencing `Vec` | **works** — whole-program merge, no layering mechanism |
| A bare `loop { … }` | **does not exist** — parses as an identifier plus a record literal (`P0001`) |

Two of these corrected the design mid-discussion: the missing `loop` (the desugar must use
`while true`), and `std/core` being free to reference `Vec` (which removed the architectural
argument that would otherwise have forced `collect` off the trait).

## 2. Scope

**In:** record-parameter bounds as *resolution scope*; `for x in it`; `map` and `filter` as lazy
adapters; `collect`, `fold`, `count`, `any` as consumers; a runtime gate fixture; an ADR.

**Out, deliberately:** `IntoIterator` (so `for x in v` stays illegal — `for x in v.iter()`);
`take`/`skip`/`enumerate`/`all`/`find`/`sum` (`enumerate` needs tuples, `sum` needs a numeric
bound, neither exists); bounds on **sum type** parameters (nothing here needs one); enforcement
of a record bound at construction (§3.2); `Set`/`String` iterators (`chars()` already returns an
indexable `[Char]`).

## 3. Record-parameter bounds as resolution scope

### 3.1 The change is small, because 2.2c made it small

`convert_ty` already takes `bounds: &[Vec<DefId>]` — threaded through 18 call sites by 2.2c's
Task 3. `collect_records` currently passes `&[]`, with a comment saying why
(`crates/nova-typeck/src/check.rs:462-466`):

> `// No bounds: reject_type_param_bounds above rejects any bound on a record's generic
> parameters outright.`

So the work is: stop calling `reject_type_param_bounds` for records, build the table with the
existing `resolve_bounds`/`expand_bounds` helpers, and pass it. Projections in field types then
resolve through exactly the machinery `I::Item` already uses in function and impl signatures.
`ast::TypeParam` already carries `bounds`, so nothing new is parsed.

**`hir::RecordType` gains nothing.** It stores `generics: u32` — a count — and no consumer needs
the bounds, because the bound's whole job finishes during field-type conversion. Not adding a
field is what stops this leaking into MIR.

Records only. Sum types keep `E0900`.

### 3.2 The bound is a resolution scope, NOT a constraint

`record MapIter<I: Iterator, U>` means **"resolve projections on `I` against `Iterator`"**. It
does *not* mean "reject `MapIter<Int, U>`".

**Why not enforce.** Phase 2.2a assessed real enforcement and rejected it, and the reasons
stand: `ExprKind::MakeRecord` carries no type arguments at all — the instantiation survives only
in the enclosing `Expr.ty`, which lowering discards, and MIR erases records to `Ptr`. Worse,
monomorphization visits only instances reachable from `main`, so enforcement would fire
*sometimes*, which is a subtler defect than not firing at all.

**Why that is safe here.** Correctness comes from the impl, not the record:
`impl<I: Iterator, U> Iterator for MapIter<I, U>` requires the bound, so a `MapIter<Int, U>` has
**no `Iterator` impl**. It constructs, and it is inert — `m.next()` is an ordinary
`E0014: no method 'next'`. Nothing is silently wrong; the bogus instantiation is merely useless.

**The risk to design against.** "Accepted and quietly ignored" is this project's most-repeated
defect: impl-level `const`s discarded (fixed `3c8127e`), record bounds themselves, record field
visibility, `pub` on methods. A bound that *looks* like a constraint and is not checked belongs
to that family. The mitigations are that the ADR states the semantics plainly, the doc comment at
the resolution site states them, and the CHANGELOG says a record bound is not enforced at
construction and why. **A reader must not have to infer this.**

## 4. `for x in it`

`check_for` (`crates/nova-typeck/src/check.rs:3878`) already isolates the range case behind a
pattern match whose `else` arm is a single `E0900`. This increment replaces that arm; the range
path is untouched.

```
for x in <it> { body }

  ⇒  { let __it = <it>                    // hidden, unscoped, mut
       while true {
         let __n = __it.next()
         match __n { Some(x) => { body }
                     None    => break } } }
```

- **`while true`, not `loop`** — Nova has no `loop` (§1.1).
- **`__it` is unscoped**, via `new_local_unscoped`, exactly as the range desugar's `__i`/`__end`
  are: it must neither collide with nor shadow a source identifier.
- **`__it` is `mut`**, which satisfies 2.2c Task 8's receiver rule so the user never writes `mut`
  for a loop. The user's `x` stays immutable, as in the range form, so assigning it is `E0060`.
- **`x` binds at the normalized `Self::Item`**, which is why this could not precede 2.2c.
- **The `Iterator` bound discharges at monomorphization** (`E0013`), where every other bound
  does — not in `check_src`.

No implicit `.iter()`. There is no `IntoIterator`, and special-casing `Vec` in the desugar would
bake a `std/collections` type into the compiler, which is far worse than the one-method coupling
§5 accepts in std source.

## 5. The adapters and consumers

All in `std/core`, beside `Iterator`.

```nova
record MapIter<I: Iterator, U> { it: I, f: fn(I::Item) -> U }

impl<I: Iterator, U> Iterator for MapIter<I, U> {
    type Item = U
    fn next(mut self) -> Option<U> { … }
}

record FilterIter<I: Iterator> { it: I, keep: fn(I::Item) -> Bool }

impl<I: Iterator> Iterator for FilterIter<I> {
    type Item = I::Item                 // projection-valued binding
    fn next(mut self) -> Option<I::Item> { … }   // while true: skip until keep
}
```

`map` and `filter` are default methods on `Iterator` returning `MapIter<Self, U>` and
`FilterIter<Self>`.

| consumer | signature |
|---|---|
| `fold` | `fn fold<A>(mut self, init: A, f: fn(A, Self::Item) -> A) -> A` |
| `count` | `fn count(mut self) -> Int` |
| `any` | `fn any(mut self, p: fn(Self::Item) -> Bool) -> Bool` |
| `collect` | `fn collect(mut self) -> Vec<Self::Item>` |

- **`count`, not `len`.** These sources are single-pass and consuming; `len` would invite the
  assumption that it is cheap and non-destructive, which is how `Vec::len` behaves.
- **`any` is not written over `fold`.** `fold` visits everything, so short-circuiting needs its
  own loop — otherwise `any` is nominal rather than real.
- **`collect` couples `std/core` to `std/collections`.** Accepted deliberately: one method, one
  type, and the whole-program merge means there is no layering mechanism to violate, only a
  convention. The alternative considered was `Vec::from_iter(it)` in `std/collections`, which
  keeps `std/core` free of collections and reads worse.
- **`let g = self.f` then `g(x)`** is required — `(self.f)(x)` is `E0014`. This is an internal
  idiom, and it needs a comment saying why, or it will be "simplified" back.
- **Adapters hold their source by pointer**, so a chain inherits the alias visibility 2.2c
  documented and pinned for `VecIter`: mutating the source mid-iteration is observable. Documented,
  not prevented — preventing it needs borrow tracking Nova does not have.
- **`FilterIter` has no `U`**, so its `Item` is a projection and every use traverses the
  projection machinery twice. This makes 2.2c Task 7's F4 unit test (`Assoc { on: Assoc }`)
  reachable from real source for the first time.

## 6. Diagnostics

The increment's user interface is its errors.

| Case | Behaviour |
|---|---|
| `for x in <not a range, not an Iterator>` | reworded `E0900`. Today's text — "`for` loops over anything but an integer range" — becomes false when §4 lands. The new text must name both accepted forms **and** mention `.iter()`, because `for x in v` is the mistake people will make. |
| `record M<I: NoSuchTrait>` | `E0001` on the trait name, not a silent skip |
| `MapIter<Int, U>` then `.next()` | `E0014: no method 'next'`, by design (§3.2). **The implementing task must read the emitted message and add a note if it misleads** — the user's real error is that `Int` is not an `Iterator`, not that a method is missing. Record the decision either way; do not leave it unexamined. |
| `record M<I: Iterator> { v: Int }` — bound on a parameter no field type uses | **Accepted silently.** Rejecting it was considered and declined: the bound is inert, harmless, and detecting "unused by any field type" is a second analysis for no user benefit. Noted here so its absence is a decision rather than an oversight. |
| An iterable whose `Iterator` bound is unsatisfied | `E0013` at monomorphization, as everywhere else |

## 7. Gate

`tests/runtime/iterator.{nova,stdout}`, registered three ways beside the four existing fixtures
(`_run`, `_build_standalone`, `_under_gc_stress`). It must cover:

1. `for` over a `Vec` via `.iter()`.
2. `for` over an integer range — **still working**, since §4 edits that function.
3. Exhaustion, and an empty source whose first `next` is already `None`.
4. A two-stage chain `.filter(…).map(…)`, proving adapter-on-adapter.
5. All four consumers.
6. **Every generic block instantiated at `Bool` or `Float`.** Carried forward from 2.2c Task 10:
   `mir_ty` maps `Int` *and* `Char` to `MirTy::I64`, and `String`/`Fn`/`Sum`/`Record`/`Array` to
   `MirTy::Ptr` = `i64` on x86-64, so **only `Bool` and `Float` have distinguishable machine
   classes**. An `Int`/`String` pair tests nothing at the monomorphization seam.

Separately as `#[test]`s, since a fixture cannot contain a compile failure: the reworded `E0900`;
`E0001` for an unresolvable record bound; and the inert-instantiation `E0014`.

## 8. Risks

1. **The unenforced record bound reads as a constraint.** Mitigated by §3.2's three-place
   documentation requirement, not by code. This is the risk most likely to become a future
   complaint.
2. **Adapter chains multiply monomorphization.** `v.iter().filter(f).map(g)` is
   `MapIter<FilterIter<VecIter<Int>>, U>` — each distinct chain shape is a distinct instantiation.
   Fine at this scale; worth knowing before someone writes a ten-stage chain.
3. **`for` edits a function the range loop depends on.** Item 2 of the gate exists because of
   this, and 2.2c's `check_for` has hygiene properties (unscoped counter, immutable loop variable)
   that the new path must match rather than reinvent.
4. **`collect`'s coupling sets a precedent.** One method today; the reason it is acceptable should
   be recorded so the next such request is decided rather than assumed.

## 9. Definition of done

604 tests plus the new ones, 0 failed; clippy `-D warnings` and `cargo fmt --check` clean; all
**fifteen** gate configurations green (five fixtures × run/build/GC-stress) with the four
pre-existing ones byte-identical; ADR 0007 recording the record-bound semantics; `nova-spec`'s
`Iterator` block updated with the shipped default methods; CHANGELOG stating that a record bound
is not enforced at construction.
