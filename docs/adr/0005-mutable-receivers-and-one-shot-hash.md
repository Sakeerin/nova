# ADR 0005 — Mutable receivers and one-shot hashing

Two decisions that together make `std/collections` (Phase 2.2a) expressible.
They share a file because both answer the same question — what a collection's
methods are allowed to assume about the values handed to them — from opposite
ends: section 1 is about the *receiver*, section 2 about the *keys*.

Both sections are accepted. They are not independent in one direction: the
streaming `Hash` that section 2 rejects would need exactly the trait-method
receiver mutability that section 1 leaves as its open gap.

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

### Status

Accepted (2026-07-26). Phase 2.2a Task 6, immediately before `Map` and `Set`,
which cannot be written without it. `Hash` was deferred from Phase 2.1, where
`std/core`'s other traits landed.

### Context

`nova-spec` specifies `Hash` in the shape Rust uses:

```nova
trait Hash { fn hash<H: Hasher>(self, h: H) }
```

A value *writes itself into* a hasher, which accumulates state and is asked
for a digest at the end. That shape is what lets a composite hash without
allocating (a record feeds each field to the same hasher), and it lets the
hasher be swapped per map (SipHash for HashDoS resistance, FxHash for speed).

Nova cannot express it. A `Hasher` has to accumulate, and the only mutation
Nova has is a field write through a binding with the `mut` permission — which
section 1 makes explicit is a permission attached to a *place*, not a claim
about ownership. So a streaming hasher must be a record whose state field the
`hash` body assigns:

```nova
fn hash<H: Hasher>(self, mut h: H) { h.write_int(self.x) }
```

which needs (a) `mut` on a *parameter*, (b) `write_int` to be a `mut self`
trait method — precisely the case section 1 records as an open gap, since
`hir::TraitMethod` has no receiver-mutability field — and (c) the caller to
observe the accumulated state afterwards, which works only because records
are reference values (section 1's alias-visible note), i.e. the whole
mechanism would rest on aliasing rather than on anything the type says. It is
also viral: `Hasher` becomes a second public trait with a method per primitive
type, every `Hash` impl gets a type parameter, and every call site has to
build a hasher and finish it.

What a hash map actually needs from `Hash` is one integer per key.

### Decision

**`Hash` is one-shot**, in `std/core/lib.nova` beside the other core traits:

```nova
pub trait Hash { fn hash(self) -> Int }
```

with impls for `Int`, `Bool`, `Char` and `String`, and the contract that
`a.eq(b)` implies `a.hash() == b.hash()`. Supporting pieces:

- **`mix64`, the splitmix64 finalizer** (module-private, so it enters no user
  namespace) backs `Int`, `Bool` and `Char`. It is not decoration: `Map` selects
  buckets with `hash & (cap - 1)` over a power-of-two capacity, so only the
  **low** bits of a hash are consulted, and `fn hash(self) -> Int { self }`
  would put every multiple of the capacity in bucket 0. A known-good mixer is
  used rather than an invented one.
- **`str_hash`, a std-only compiler builtin** (`Builtin::STD_CORE_ONLY`,
  beside `str_cmp`) backed by the runtime's FNV-1a `nova_rt_str_hash`, because
  Nova cannot walk a string's bytes: `String` has no length, indexing or
  iteration, and is not FFI-safe, so no `extern` can reach it either. Being
  std-only rather than `Builtin::GLOBAL`, it does not become a reserved word —
  a user program may still define `fn str_hash`.
- **`char_to_int`, a second std-only builtin**, because `Hash for Char` needs
  the codepoint and Nova has no `Char` → `Int` conversion at all (`as` casts
  are unsupported, and `Char` has no methods beyond its trait impls). Unlike
  every other builtin it is *not* a runtime call: `Char` and `Int` are both
  `MirTy::I64`, so `nova-mir` lowers it to a register move rather than adding
  a permanent runtime ABI symbol whose body would be the identity function.

Two properties of `mix64` under Nova's semantics were verified rather than
assumed, since both would have silently produced a broken hash:

- The constants `0xbf58476d1ce4e5b9` and `0x94d049bb133111eb` exceed `Int`'s
  range and are written as the two's-complement negatives of those bit
  patterns. The lexer accepts them (each is `-` applied to a literal below
  `i64::MAX`) and `*` wraps rather than trapping, so the arithmetic agrees
  with the canonical version modulo 2^64.
- Nova's `>>` is **arithmetic**, and there is no `>>>`, so this is splitmix64's
  finalizer with arithmetic shifts — not bit-identical to the canonical version
  wherever an intermediate goes negative. Since the low bits are the only ones
  bucket selection reads, they are *measured* rather than argued: the histograms
  in `tests/runtime/hash.nova` are the evidence, and they were computed from an
  independent model of the same function rather than recorded from a run. A
  second consequence is that every `x ^ (x >> k)` clears the sign bit, so
  `mix64` is not a bijection: it loses at most 3 bits of range, and always
  returns a non-negative `Int`.

### Consequences

- **A composite type's `Hash` must combine children by hand**, e.g.
  `fn hash(self) -> Int { mix64(self.x.hash() ^ (self.y.hash() * 31)) }`.
  There is no derive and no accumulator; that is the cost of the shape.
- **Bucket selection must mask, not take a remainder.** `mix64` is
  non-negative but `str_hash` reinterprets a `u64` as `i64`, so a `String`
  hash may be negative and `hash % cap` could yield a negative index.
  `hash & (cap - 1)` is non-negative for every `i64` and is what `Map` will
  use.
- **Hashes are not randomized per process**, so a `Map` is HashDoS-attackable
  by adversarial keys. FNV-1a is not collision-resistant and neither is
  `mix64`. Acceptable for Phase 2.2a; a seeded hasher is a `Hasher`-shaped
  question, i.e. it is the migration below.
- **Hashes must be backend-independent.** `tests/runtime/hash.nova` runs under
  both the JIT and the object backend against the same fixture, whose expected
  bucket histograms were computed independently from splitmix64's finalizer
  rather than recorded from a run.
- **`Hash for Float` is deliberately absent**, a second deviation from
  `20-STDLIB.md`, which lists `Float` among the primitives that implement
  `Hash`. NaN never equals itself, so a
  NaN key would be unreachable — inserted and then not findable, including by
  the very expression that produced it, since `Eq for Float` is already
  documented as non-reflexive there. And `0.0 == -0.0` while their bit patterns
  differ, so any bitwise hash would break the `eq` ⇒ equal-hash contract unless
  the impl normalized both first. A `Float` key needs a decision about NaN
  (reject, normalize to a canonical NaN, or total-order it) that belongs with
  the `Ord for Float` caveat, not smuggled in with hashing. Pinned by
  `float_has_no_hash_impl`, so re-adding it is a deliberate act with a test to
  change.

### Migration path

This is a **commitment, not a stopgap**: `hash` is `Hash`'s only method, so
moving to the streaming shape changes the trait's entire surface and therefore
*every* impl and every call site — in std and in user code alike. There is no
version of that which is source-compatible, and no adapter that helps: a
one-shot impl cannot be driven by a hasher (it has nowhere to write), and a
streaming impl cannot answer `hash()` without a hasher to hand it. So the
switch, if it ever happens, is a breaking change with a deprecation cycle, and
it should happen only if a concrete need appears that one-shot cannot serve —
per-map hasher choice, or HashDoS resistance via a seed.

The prerequisites are all in section 1's territory, which is why the two
decisions share a file: `mut` on parameters, receiver mutability declared on
`hir::TraitMethod` (section 1's Migration path steps 1–2), and `mut self` trait
methods checked at the call site (step 3). Until those exist, the streaming
shape is not merely inconvenient in Nova — it is not writable.

Cheaper changes that this decision does *not* foreclose, because none of them
touch `Hash`'s signature: replacing FNV-1a inside `nova_rt_str_hash`, replacing
`mix64`'s constants or rounds, and seeding either from a process-start value.
The one-shot shape fixes what a hash *is* (one `Int` per value), not how it is
computed.
