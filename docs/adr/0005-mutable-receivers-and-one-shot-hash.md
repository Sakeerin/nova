# ADR 0005 — Mutable receivers and one-shot hashing

Two decisions that together make `std/collections` (Phase 2.2a) expressible.
They share a file because both answer the same question — what a collection's
methods are allowed to assume about the values handed to them — from opposite
ends: section 1 is about the *receiver*, section 2 about the *keys*.

Both sections are accepted. They are not independent in one direction: the
streaming `Hash` that section 2 rejects would need exactly the trait-method
receiver mutability that section 1 left as its open gap — and which section 1's
Migration path has since closed (Phase 2.2b), so that particular obstacle is no
longer what stands in section 2's way. See section 1's "Migration path: done"
for what changed and what section 2 still lacks.

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
- `hir::TraitMethod::mut_self` is the trait-side counterpart, populated in
  `collect_traits` with the identical scan. It has to be a flag on the trait
  rather than a lookup into `mut_self`, because trait dispatch resolves to
  `(trait_id, method_index)` with no impl to consult, and because
  `method_sig_parts` strips a `self` parameter whether or not it is `mut`, so
  `params` cannot carry it either.
- `Checker::check_mutable_receiver` runs from `check_method_call`'s
  `MethodRes::Inherent` arm and classifies the receiver's **AST** with
  `place_root` — the AST, because the checked `hir::Expr` has already lost the
  projection shape `place_root` walks. `Mutable` passes; `ImmutableLocal(name)`
  and `NotAPlace` each report `E0060`, and the immutable-local case carries the
  same ``declare it as `let mut …` `` note the other two assignment forms
  attach.
- The **trait** path runs the same `require_mutable_place` from
  `emit_trait_call`'s receiver arm, keyed on `hir::TraitMethod::mut_self`.
  `emit_trait_call` rather than `check_method_call`'s `MethodRes::Trait` arm —
  which is where the Migration path below originally pointed — because that arm
  is not the only receiver route: `try_display` reaches a
  `fmt(mut self) -> String` straight from string interpolation without passing
  through it. `emit_trait_call` is the one point every receiver route converges
  on, so `TraitCallSelf::Receiver` carries the receiver's AST alongside its
  checked form and the rule cannot be dodged by finding another way in.
- It is a **no-op for any method that does not declare `mut self`**, so a plain
  `self` reader (`fn get(self) -> Int`) is still callable on an immutable
  binding, on either path. Only the `mut` keyword demands anything of the
  caller.

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
  without a body at all, which is precisely the case the gap below was about —
  and the reason closing it needed a *declared* flag on `hir::TraitMethod`
  rather than anything inferred.
- **Put `mut` on record fields instead** (`record C { mut n: Int }`) and drop
  the binding-level rule. Rejected: it needs a second mutability oracle
  operating per field, which then has to be reconciled with `place_root` for
  index chains and with the field *read* path; and it cannot express
  `Vec::push`, whose mutation is of the receiver as a whole. Keeping `mut` on
  bindings keeps `place_root` the only answer to "may this be mutated".
- **Cover trait-method calls in the same change.** Deferred at the time, with
  the reasoning and the cost recorded under Consequences and Migration path
  below. Done in Phase 2.2b, as the Migration path prescribed and at the cost it
  predicted; the Consequences entry below records what the gap was and how it
  closed.

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
- **Trait-method calls were not covered at first. They are now.** Originally
  `MethodRes::Trait` dispatch resolved to `(trait_id, method_index)`, and for a
  generic receiver (`fn f<T: Tr>(x: T) { x.m() }`) there was no single impl whose
  receiver declaration could be consulted — the `mut self` would have to be
  declared on the *trait*, which `hir::TraitMethod` had no field for. So
  `impl Tr for P { fn m(mut self) { … } }` called as `p.m()` on an immutable `p`
  was accepted, silently mutating it. That hole was deliberately left open for
  Phase 2.2a, whose collections use **inherent** impls only, and closed in Phase
  2.2b exactly as the Migration path prescribed: `hir::TraitMethod` gained the
  flag, `check_impl_method_signatures` gained the conformance comparison that
  decides the disagreement case (`E0072`, beside the `has_self` agreement check
  it extends), and the call site consults the trait's flag.
  `mut` on a *parameter* already parsed, so the generic case
  (`fn f<T: Tr>(mut x: T) { x.m() }`) is writable rather than merely rejected.
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

### Migration path: done

**The trait-method gap is closed** (Phase 2.2b), so the three steps below now
describe what was done rather than what is pending. Closing it changed neither
the rule, the diagnostic, nor `place_root`, exactly as predicted — it added a
second population site and a second call site:

1. **Done.** `mut_self: bool` on `hir::TraitMethod` beside `has_self`, populated
   in `collect_traits` with the identical `.any(…)` predicate.
2. **Done, but not where this said.** The receiver-agreement check the
   mutability comparison extends had already moved out of
   `check_impl_conformance` into `check_impl_method_signatures` (the `E0070`/
   `E0071` vs `E0072` split), so the comparison went there. It reports `E0072`
   naming both sides, and deliberately does *not* `continue` the way the
   receiver-*presence* mismatch does: a `mut` disagreement misaligns neither
   parameter list, so the rest of the signature is still worth checking.
3. **Done, but at a wider seam than this said.** The check runs from
   `emit_trait_call`'s receiver arm, not from `check_method_call`'s
   `MethodRes::Trait` arm, because that arm is not the only route to a receiver
   call: `try_display` reaches a `fmt(mut self) -> String` from string
   interpolation without passing through it. Five routes were measured, and
   before the change **all five** accepted a mutation through an immutable
   binding: a direct trait call, a generic bound, a supertrait bound, a trait
   default body delegating to a mutator, and string interpolation. Installing
   the check at `check_method_call` as written above would have closed three of
   the five and left the last two silently open. `TraitCallSelf::Receiver`
   therefore carries the receiver's AST alongside its checked form, so there is
   no way to hand `emit_trait_call` a receiver without also handing it the place
   to classify.

The lesson worth carrying forward: **a rule stated over a resolution *outcome*
must be enforced where the outcomes converge, not where one caller happens to
produce one.** The gap this ADR recorded was one missing call; re-opening it
needs only a fourth `emit_trait_call` caller if the AST ever stops travelling
with the receiver.

What is still out of scope, and unchanged by the above: whether `mut self`
should eventually be inferable for *closure* receivers, or extended to a real
`&mut`-style borrow discipline. Both would be new rules layered over
`place_root`, not replacements for it.

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
trait method — at the time this was decided, precisely the case section 1
recorded as an open gap, since `hir::TraitMethod` had no receiver-mutability
field; both (a) and (b) work as of Phase 2.2b, so this half of the objection has
lapsed and the rest below has not — and (c) the caller to
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
- **`str_hash`, a std-only compiler builtin** (`Builtin::STD_ONLY`,
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

`Int` has neither the unsigned type nor the logical shift the canonical
splitmix64 is written against, so `mix64` needs two workarounds. Both are
load-bearing, and the second was originally got *wrong* in a way no
single-sign test could see — recorded here because the failure mode is
instructive, not merely historical:

- The constants `0xbf58476d1ce4e5b9` and `0x94d049bb133111eb` exceed `Int`'s
  range and are written as the two's-complement negatives of those bit
  patterns. Hex would be the obvious spelling and does not work: the lexer's
  `lex_hex_int` parses with `i64::from_str_radix`, so `0xbf58476d1ce4e5b9`
  silently fails to lex. Each negative *is* `-` applied to a decimal literal
  below `i64::MAX`, which lexes, and `*` wraps rather than trapping, so the
  arithmetic agrees modulo 2^64.
- Nova's `>>` is **arithmetic** and there is no `>>>`, so **each shift is
  masked** to clear the sign extension: `& (2^(64-k) - 1)`. This is not a
  refinement, it is the difference between a hash function and a broken one.
  Arithmetic shift commutes with complement (`!(x >> k) == (!x) >> k`) and XOR
  is invariant under complementing both operands, so unmasked,
  `x ^ (x >> k) == !x ^ ((!x) >> k)`: stage 1 is exactly 2-to-1 under
  `x ↔ !x == -1 - x`, and since every later stage is a function of stage 1's
  output, the whole finalizer collapsed with
  **`mix64(x) == mix64(-1 - x)` identically**. Identically, not congruently —
  so each pair would share a bucket at *every* capacity, and resizing, which
  is a hash map's only answer to collisions, could never separate them.
  With the masks, `mix64` is bit-identical to canonical splitmix64 (verified
  against an independent implementation, including for negative keys) and
  therefore a bijection.

The lesson for anyone adding a hash impl here: **the defect was invisible to
every single-sign test.** Consecutive keys, multiples of the capacity, and an
all-negative sample all showed textbook-uniform low bits (chi² ≈ df), because
each sample contained at most one member of any complement pair. What exposed
it is one line — `(0).hash() != (-1).hash()` — plus a bucket count over keys
spanning both signs, which the unmasked mixer failed 58-of-256 where uniform
is ~101. Both are now in `tests/runtime/hash.nova`.

### Consequences

- **A composite type's `Hash` must combine children by hand**, e.g.
  `fn hash(self) -> Int { self.x.hash() ^ (self.y.hash() * 31) }`. There is no
  derive and no accumulator; that is the cost of the shape. Note the combiner
  cannot call `mix64`, which is module-private to `std/core` on purpose — it is
  an implementation detail of the primitive impls, not a published utility, and
  publishing it would make its exact definition a compatibility surface.
- **Mask a hash. Never shift one, and never read its high bits.**
  `hash & (cap - 1)` over a power-of-two capacity is the only supported way to
  derive a bucket index, and the rule is stated beside `Hash` in
  `std/core/lib.nova` because that is where `Map`'s author will look. Three
  independent reasons: a hash spans the full `Int` range including negatives
  (both `mix64` and `str_hash`, the latter reinterpreting an unsigned FNV-1a
  result), so `hash % cap` can be a negative index, whereas `&` with a positive
  mask is non-negative for every `i64`; the high bits are not an independent
  second hash, so a `hash >> 57` tag byte is not uncorrelated with the bucket
  it accompanies, and for a negative `str_hash` the sign extension shrinks such
  a tag's range on exactly the keys it exists to distinguish; and `mix64`'s
  guarantees are stated over the whole 64-bit result, not over any slice of it.
- **Hashes are not randomized per process**, so a `Map` is HashDoS-attackable
  by adversarial keys. FNV-1a is not collision-resistant and neither is
  `mix64`. Acceptable for Phase 2.2a; a seeded hasher is a `Hasher`-shaped
  question, i.e. it is the migration below.
- **Hashes must be backend-independent.** `tests/runtime/hash.nova` runs under
  both the JIT and the object backend against the same fixture, whose expected
  bucket histograms were computed independently from splitmix64's finalizer
  rather than recorded from a run. That fixture is also where the
  complement-pair and both-signs regressions live, so a future change to
  `mix64` that reintroduces a structural fold fails there rather than in
  `Map`'s probe chains.
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
methods checked at the call site (step 3). **All three now exist** (Phase 2.2b),
so the streaming shape became *writable* — which changes nothing about this
decision. Every reason to reject it stood on its own: `Hash`'s signature is its
whole surface, the shape is viral across every impl and call site, and a
streaming hasher's correctness would rest on records being alias-visible
reference values rather than on anything the type says. Writability was never the
argument; it was only the thing that made the argument academic. Treat the
unblocking as removing the excuse, not as reopening the question — reopening it
still needs the concrete need named above (per-map hasher choice, or HashDoS
resistance via a seed).

Cheaper changes that this decision does *not* foreclose, because none of them
touch `Hash`'s signature: replacing FNV-1a inside `nova_rt_str_hash`, replacing
`mix64`'s constants or rounds, and seeding either from a process-start value.
The one-shot shape fixes what a hash *is* (one `Int` per value), not how it is
computed.
