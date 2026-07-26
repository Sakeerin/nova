# ADR 0005 — Mutable receivers and one-shot hashing

Two decisions that together make `std/collections` (Phase 2.2a) expressible.
They share a file because both answer the same question — what a collection's
methods are allowed to assume about the values handed to them — from opposite
ends: section 1 is about the *receiver*, section 2 about the *keys*.

Section 1 is accepted. Section 2 is a placeholder, filled in when the `Hash`
decision is made.

---

## 1. Mutable receivers

### Status

Accepted (2026-07-26). Phase 2.2a, immediately after `rec.field = v` became
legal (`270817b`).

### Context

Nova puts `mut` on **bindings**, not on record fields. There is no per-field
`mut`, and there is no `&mut` — `mut` is a permission attached to a `let`, and
`Checker::place_root` is the single oracle that decides whether a place is
reachable through such a binding. It walks field (`rec.f`) and index (`arr[i]`)
projections down to the root local and answers `Mutable`, `ImmutableLocal(name)`
or `NotAPlace`. Both existing assignment forms consult it and report `E0060`:

- `arr[i] = v` (Phase 1)
- `rec.f = v` (Phase 2.2a Task 1, ADR-less until this file)

Because the `mut` lives on the binding, a *mutating method* is necessarily
written by marking the receiver: `fn push(mut self, x: T)`. That is what makes
`self.len = self.len + 1` legal inside the body — `check_fn_body` registers each
parameter as a local with the parameter's own `is_mut`, so a `mut self` receiver
is a mutable root and a plain `self` receiver is not.

Nothing, however, constrained the **caller**. Before this decision:

```nova
let v = Vec::new()      // no `mut`
v.push(1)               // accepted — mutated `v` anyway
v.len = v.len + 1       // E0060: cannot assign to a field of immutable `v`
```

The same operation got two different answers depending on whether it was
spelled as a field assignment or wrapped in a one-line method. That reduces
`mut` to a formality: any mutation can be laundered through a method, so the
keyword gates a syntax rather than an effect. Phase 2.2a is about to add
`Vec`, `Map` and `Set` — the first types in the language whose whole purpose is
in-place mutation — so the rule has to be settled before their APIs are
written, not after.

### Decision

**Calling a method that declares `mut self` requires a mutable receiver
place.** Concretely, in `crates/nova-typeck/src/check.rs`:

- `Checker::mut_self: FxHashSet<DefId>` records every impl method whose `self`
  parameter is declared `mut`. It is populated in `collect_impls` beside
  `has_self`, with the same `.any(|p| p.name.value == "self" && p.is_mut)`
  scan — `.any`, not a look at `params[0]`, because `method_sig_parts` strips a
  `self` at *any* position and the parser accepts a misplaced receiver
  (`fn f(x: Int, mut self)`). Two predicates that disagree about the same
  parameter is how `has_self` bugs happen; both scan the whole list.
- `Checker::check_mutable_receiver` runs from `check_method_call`'s
  `MethodRes::Inherent` arm and classifies the receiver's **AST** with
  `place_root` — the AST, because the checked `hir::Expr` has already lost the
  projection shape `place_root` walks. `Mutable` passes; `ImmutableLocal(name)`
  and `NotAPlace` each report `E0060`, and the immutable-local case carries the
  same ``declare it as `let mut …` `` note the other two assignment forms
  attach.
- It is a **no-op for any method not in `mut_self`**, so a plain `self` reader
  (`fn get(self) -> Int`) is still callable on an immutable binding. Only the
  `mut` keyword demands anything of the caller.

`mut self` is a *declared* contract, checked at the call site. The call is
still emitted after the error so a single missing `let mut` does not cascade
into argument-type or arity noise.

### Alternatives considered

- **Java/Python semantics: let any binding call any method.** Mutation flows
  through the reference; `mut` stays a purely local statement about
  reassignment and direct field writes. Rejected: it is exactly the
  inconsistency described above. `rec.f = v` and `arr[i] = v` already demand a
  mutable root, and a language where `v.push(x)` is allowed but
  `v.items[0] = x` is not has no rule a reader can hold in their head — it has
  two.
- **Infer the mutator-ness of a method from its body** (a method is a mutator
  iff it assigns through `self`). Rejected on two counts. It makes a method's
  contract depend on its implementation, so adding one assignment inside a
  method body silently invalidates every existing call site — the opposite of
  what a signature is for. And it cannot work for a trait method declared
  without a body at all, which is precisely where the remaining gap below is.
- **Put `mut` on record fields instead** (`record C { mut n: Int }`) and drop
  the binding-level rule. Rejected: it needs a second mutability oracle
  operating per field, which then has to be reconciled with `place_root` for
  index chains and with the field *read* path; and it cannot express
  `Vec::push`, whose mutation is of the receiver as a whole. Keeping `mut` on
  bindings keeps `place_root` the only answer to "may this be mutated".
- **Cover trait-method calls in the same change.** Deferred, with the reasoning
  and the cost recorded under Consequences and Migration path below.

### Consequences

- **`let mut` is now part of every mutating std API's usage.**
  `let mut v = Vec::new()` is required before `v.push(x)`; `let v = …` is a
  compile error at the first mutation. Symmetrically, **every std API that
  mutates must declare `mut self`** — an accessor that forgets it will not
  compile (its own body cannot assign through `self`), and a reader that adds
  it needlessly forces `let mut` on callers for no reason.
- **The receiver may be any place, not only a bare local.** `place_root` walks
  the whole chain, so `self.map.insert(k, v)` from inside a `mut self` method
  resolves to `Mutable` through the `self` root — that is how `Set` is built on
  `Map` — while `o.inner.bump()` on an immutable `o` is rejected at the root.
- **A temporary receiver is rejected**: `make().bump()` is `NotAPlace` and
  reports `E0060`, because the mutation could not be observed by anyone.
- **Gap: trait-method calls are not covered.** `MethodRes::Trait` dispatch
  resolves to `(trait_id, method_index)`, and for a generic receiver
  (`fn f<T: Tr>(x: T) { x.m() }`) there is no single impl whose receiver
  declaration could be consulted — the `mut self` would have to be declared on
  the *trait*, which `hir::TraitMethod` has no field for. So
  `impl Tr for P { fn m(mut self) { … } }` called as `p.m()` on an immutable
  `p` is accepted today. This is a real hole in the rule, deliberately left
  open: the collections in Phase 2.2a use **inherent** impls only, and closing
  it well means also deciding what happens when an impl's receiver mutability
  disagrees with its trait's (a new conformance rule, cf. the existing
  `has_self` agreement check and its `E0072`/`E0014` family). Closing it is
  cheap and mechanical once that is decided — see Migration path.
- **`mut self` does not copy the receiver, and mutation is alias-visible.**
  Records are heap objects; `mut` is a permission on a binding, not an
  ownership or aliasing claim, and Nova has no borrow checker. So two mutable
  bindings to the same record see each other's writes:

  ```nova
  let mut c = Counter { n: 0, label: "hits" }
  let mut alias = c
  alias.n = 99
  println("alias visible: ${c.n}")   // 99
  ```

  The same holds through a `mut self` method — `c.bump()` is visible through
  `alias` — because the receiver is passed as the same pointer, not copied.
  Both are executed and pinned by `tests/runtime/field_assign.nova` under the
  JIT and a standalone build. This is deliberate reference semantics, and it is
  the reason the receiver rule is
  about *permission to mutate* rather than about exclusivity — it prevents
  mutating through a binding that did not ask for the right, and promises
  nothing about who else can see the result.
- **The error is raised at the call site**, so a `mut self` method nobody calls
  still compiles, exactly as an uncalled ambiguous trait impl does (ADR 0004).
- The diagnostic names the callee by its impl-qualified `Def` name (`P.bump`),
  matching every other inherent-method diagnostic in this file — the spelling
  `arity_errors_name_the_callee_uniformly` already pins.

### Migration path

`Checker::mut_self` plus `check_mutable_receiver` is the single seam. Closing
the trait-method gap does not change the rule, the diagnostic, or `place_root`;
it adds a second population site and a second call site:

1. Add `mut_self: bool` to `hir::TraitMethod` beside `has_self`, populated in
   `collect_traits` with the identical `.any(…)` predicate.
2. Extend `check_impl_conformance`'s existing `has_self` agreement check to the
   receiver's mutability, so an impl cannot declare `mut self` for a trait
   method that does not, or vice versa.
3. Call the same `check_mutable_receiver` logic from `check_method_call`'s
   `MethodRes::Trait` arm, keyed on the trait method's flag instead of the
   impl method's `DefId`.

Whether `mut self` should eventually be inferable for *closure* receivers, or
extended to a real `&mut`-style borrow discipline, is out of scope here: both
would be new rules layered over `place_root`, not replacements for it.

---

## 2. One-shot hashing

_Placeholder. To be written when the `Hash` decision is made (Phase 2.2a
Task 6); no part of section 1 depends on it._
