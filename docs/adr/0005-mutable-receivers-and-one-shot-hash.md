# ADR 0005 — Mutable receivers and one-shot hashing

Two decisions that together make `std/collections` (Phase 2.2a) expressible.
They share a file because both answer the same question — what a collection's
methods are allowed to assume about the values handed to them — from opposite
ends: section 1 is about the *receiver*, section 2 about the *keys*.

Both sections are accepted. They are not independent in one direction: the
streaming `Hash` that section 2 rejects would need exactly the trait-method
receiver mutability that section 1 left as its open gap — and which section 1's
Migration path has since closed (Phase 2.2c), so that particular obstacle is no
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
  below. Done in Phase 2.2c, as the Migration path prescribed and at the cost it
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

- **AMENDED 2026-07-29 (Phase 2.2d): std may now launder a mutation through a
  plain `self` receiver, and `std/core`'s `Iterator` consumers do.** The
  self-enforcement promised two bullets above — "every std API that mutates must
  declare `mut self`; an accessor that forgets it will not compile" — **is no
  longer true.** `Iterator`'s `fold`, `count`, `any` and `collect` each mutate
  their receiver, declare plain `self`, and compile, by opening with
  `let mut it = self` and driving `it`.

  *Why.* Those four must advance the iterator, so `mut self` was the obvious
  declaration. But `mut self` rejects a *temporary* receiver — the bullet
  directly above — which made `v.iter().filter(f).map(g).collect()` an `E0060`.
  That is the form the Phase 2.2d increment was designed around, so the API's
  documented shape did not compile.

  *What was given up, precisely.* `place_root` returns three verdicts, and
  `mut self` rejects two of them: `NotAPlace` (the temporary) and
  `ImmutableLocal`. Only the first was the problem; the second was collateral.
  So this is now accepted and silently advances an immutable binding:

  ```nova
  let it = Cursor { n: 0, max: 3 }   // no `mut`
  it.count()   // 3
  it.count()   // 0   <- advanced, no diagnostic
  ```

  Under `mut self` **both** calls were rejected, each as
  `E0060: 'count' mutates its receiver, but 'it' is immutable` — which is the
  exact shape §1's Context cites as the reason this rule exists. **That loss is
  real and it was not the goal.** Nova has no third receiver form that accepts a
  temporary while rejecting an immutable local, so the two could not be
  separated.

  (An earlier draft of this amendment quoted that diagnostic as naming `next`,
  and as firing only on the first call. Both were wrong, and measured so: the
  message is built by `MutTarget::Receiver(m).immutable_message` in
  `crates/nova-typeck/src/check.rs`, which interpolates **the method actually
  invoked** — so a `count()` call says `count`, not the `next` it would have
  called internally — and the check runs per call site, so two calls give two
  diagnostics. Writing up a diagnostic observed for one call shape as the answer
  for a different one is the specific mistake this increment made repeatedly; it
  is corrected here rather than quietly, because a wrong quotation in an ADR is
  the kind of thing later work cites as settled.)

  *What bounds it.* The mechanism is untouched — `mut self` still reports `E0060`
  on a temporary and on an immutable local, verified on an inherent method, a
  trait method, and `next` itself. `next` remains `mut self`, so iterating by
  hand still requires a mutable binding. No new iterator state becomes reachable:
  a `let mut` binding could always be partially consumed and reused. And the
  compiler already launders precisely this way in its own `for` desugar
  (`check_for_iterator` binds `let mut __it = <expr>`), so `for x in <immutable
  local>` already advanced that local before this change — the escape hatch
  pre-existed, and `Iterator`'s consumers are its first *library* use.

  *If Nova ever gains move semantics, re-derive this.* The whole argument rests
  on `let mut it = self` aliasing rather than copying, which holds only because
  records are heap objects passed by pointer (`hir::Ty::Record` maps to
  `MirTy::Ptr`) and this compiler has no move semantics. Measured three ways,
  including a record exposing its own cursor field so that aliased, copied and
  fully-scanned are three distinguishable outcomes. See ADR 0007 §2.
- **Trait-method calls were not covered at first. They are now.** Originally
  `MethodRes::Trait` dispatch resolved to `(trait_id, method_index)`, and for a
  generic receiver (`fn f<T: Tr>(x: T) { x.m() }`) there was no single impl whose
  receiver declaration could be consulted — the `mut self` would have to be
  declared on the *trait*, which `hir::TraitMethod` had no field for. So
  `impl Tr for P { fn m(mut self) { … } }` called as `p.m()` on an immutable `p`
  was accepted, silently mutating it. That hole was deliberately left open for
  Phase 2.2a, whose collections use **inherent** impls only, and closed in Phase
  2.2c exactly as the Migration path prescribed: `hir::TraitMethod` gained the
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

**The trait-method gap is closed** (Phase 2.2c), so the three steps below now
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
field; both (a) and (b) work as of Phase 2.2c, so this half of the objection has
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
methods checked at the call site (step 3). **All three now exist** (Phase 2.2c),
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

### Amendment, 2026-08-26 — `str_hash` is seeded and finalized, and which sentence above governs that

Nothing in the decision changes. `Hash` keeps `fn hash(self) -> Int`, the
migration stays unstarted, its prerequisites stand as recorded, and the
backend-independence requirement is untouched. What changed is the *computation*
behind `impl Hash for String`: `nova_rt_str_hash` is now seeded FNV-1a over the
bytes followed by splitmix64's finalizer, the seed drawn once per process from
`std::collections::hash_map::RandomState` inside the runtime. No intrinsic was
added, no `impl Hash` was edited, and no Nova-side source changed **behaviour** —
the Nova-side edits in that increment are comments, in `std/core`,
`std/collections` and `std/json` — so nothing has to be recompiled differently or
edited to keep compiling. That is a
source-compatibility statement and not a behavioural one: a program that read a
`String`'s hash, or `Map::keys()` order, and expected the same answer from a
later run of the same binary now gets a different one. The runtime function
documents its own reasoning and carries the measurements; they are not restated
here.

**Sentences in this section point opposite ways about whether that was
permitted.** The Consequences bullet says:

> **Hashes are not randomized per process**, so a `Map` is HashDoS-attackable by
> adversarial keys. FNV-1a is not collision-resistant and neither is `mix64`.
> Acceptable for Phase 2.2a; a seeded hasher is a `Hasher`-shaped question, i.e.
> it is the migration below.

The Migration path's closing paragraph says:

> Cheaper changes that this decision does *not* foreclose, because none of them
> touch `Hash`'s signature: replacing FNV-1a inside `nova_rt_str_hash`,
> replacing `mix64`'s constants or rounds, and seeding either from a
> process-start value.

**The closing paragraph governs this change.** Both are true of different
things, and the distinction is the object/function one: a **swappable seeded
`Hasher` object** — per-map hasher choice, a seed a caller supplies or selects —
is `Hasher`-shaped and does need the migration, because it needs somewhere to
put the hasher in the signature. **Seeding or replacing the one-shot function**
needs none of that, and the closing paragraph says so explicitly. This increment
did the second.

**Recorded because it nearly went the other way.** Read on its own, "a seeded
hasher is a `Hasher`-shaped question, i.e. it is the migration below" says the
seeding done here required a breaking change with a deprecation cycle, and a
later increment reading these paragraphs as precedent may reach the same wrong
conclusion. It is the narrower reading that is right: the bullet is about a
hasher *object*, not about the seed as such. `nova-spec/20-STDLIB.md` §7 and
`docs/adr/0018-std-json-scope-and-build-order.md` both named the migration as
the remedy for the `Map` exposure on the strength of the broader reading, and
both are corrected by their own dated amendments.

**The Phase 2.2a disclosure narrows; it does not go away.** "**Hashes are not
randomized per process**" is now **false for `String` keys** — they are
randomized per process, which is the whole of this change — and **still true for
`mix64`**, hence for `Int`, `Bool` and `Char` keys, whose buckets remain a
function of the key alone and remain attackable by chosen keys. That half is
load-bearing and is a decision rather than an oversight: `tests/runtime/hash.stdout`
pins `mix64`'s histograms as a specification of the low-bit spreading `Map`'s
masking depends on, and the exposed path is not the one `std/json` takes, whose
object keys are strings.

"FNV-1a is not collision-resistant and neither is `mix64`" **stays true as
written.** Seeded, finalized FNV-1a is not a cryptographic hash and is not
SipHash. What the change buys is resistance to a *precomputed* collision set —
an attacker who cannot learn the seed cannot build one offline — plus the
diffusion the runtime function records. An adversary who can observe timing and
adapt is out of scope. No record should be read as claiming more.

**The seed is readable from Nova, and the channel is this decision's own shape
rather than a defect in the seeding.** `("").hash()` returns splitmix64's
finalizer applied to the raw seed, because FNV's loop body never runs on an empty
string, and that finalizer is a bijection with a published inverse — so one call
from ordinary Nova code recovers the seed exactly. Measured on the seeding
increment's own tree: one `("").hash()` was inverted to a seed, and that seed
then predicted two further hashes from the same process exactly. The channel is
`fn hash(self) -> Int` handing a caller a whole 64-bit result in one call, which
is what this ADR decided and which predates the seeding. That gives the
out-of-scope sentence above a concrete mechanism rather than a category: an
attacker who can obtain one `String` hash from a running process can recover that
process's seed and construct collisions for it, while an attacker who cannot
observe the process still cannot build a colliding set before it starts, the seed
being drawn per process. Precomputation resistance is the half that survives.
Nothing here reopens the migration question, and `Hash`'s signature is unchanged.

### Amendment, 2026-08-27 — the reason given for `str_hash` has gone stale; the builtin stands

From the `hashdos-resistance-test` increment, later than the 2026-08-26
amendment above and written by a different hand. **Nothing in the Decision
moves.** `Hash` keeps `fn hash(self) -> Int`, `str_hash` stays a
`Builtin::STD_ONLY` builtin backed by `nova_rt_str_hash`, the migration stays
unstarted with its prerequisites as recorded, and no signature, impl, fixture or
behaviour changes. What is corrected here is a *justification*, and only the
first half of one.

The Decision's supporting-pieces bullet says `str_hash` is backed by the runtime

> because Nova cannot walk a string's bytes: `String` has no length, indexing or
> iteration, and is not FFI-safe, so no `extern` can reach it either.

**The first clause was true when written and is now stale.** `std/bytes` exposes
`bytes_from_string`, `byte_at` and `to_ints`, reachable from a user module with
no import at all — the registered fixture `tests/runtime/bytes_basics.nova`
calls `bytes_from_string("hi")` as its first statement and is checked against a
golden — and `std/strings` gives `String` a `len`, `chars`, `char_at` and
`slice`. Nova can walk a `String`'s bytes. The arithmetic a hash needs also runs
in Nova already: `mix64` in `std/core` computes splitmix64's finalizer in
ordinary `Int` shift, xor and multiply.

**Stale, not scoped.** The capability reaches `std/core` itself and not merely
user modules: every member of `Builtin::STD_ONLY` is seeded into every std
module's scope with no condition beyond `is_std_module`
(`crates/nova-resolver/src/lib.rs`, `resolve_program`'s seeding pass), which is
already why `std/core`'s own `impl Debug for String` calls `str_chars` above a
comment recording that it is "already visible here with no import". An argument
that the sentence stayed true because it was *scoped* to `$std.core` being the
first `STD_MODULES` entry was made during that increment and withdrawn inside
it: that array's order is documented as significant "only in that it fixes
module indices", so position gates no capability.

**The second clause stands unchanged.** `String` is still not FFI-safe and no
`extern` reaches it.

**This does not leave `str_hash` unjustified, and must not be read as proposing
its removal.** The standing reason is duplication, not incapacity:
`nova_rt_str_hash` is where this project's string hash lives, and an FNV-1a
written in Nova would be a second copy of it to keep in step with the first.
That cost is concrete rather than hypothetical now, because that function is
seeded FNV-1a followed by splitmix64's finalizer over a per-process seed (the
2026-08-26 amendment above), so a Nova copy would have to track the algorithm,
its constants and the seed — and the one channel to the seed that amendment
documents, inverting `("").hash()`, is recorded there as something to narrow
rather than to build on.

**`mix64` cuts the other way, and saying so is the point.** Splitmix64's
finalizer *is* written in Nova, in `std/core` — the `mix64` bullet sits
immediately above the `str_hash` one, and the paragraph below both spells out
the two `Int` workarounds its Nova source needs. So the line this amendment
draws is not "Nova cannot do hash arithmetic", which it plainly can, but that
the string hash's byte loop and its seed have one home.
Read the `str_hash` bullet as "the runtime owns this computation", not as "Nova
is incapable of the arithmetic".

**Recorded because the ruling went the other way first, and the sequence is the
instructive part.** The increment that found this checked this ADR early —
against test-assertability, per-process randomisation, precomputation resistance
and seed readability — found nothing it asserts falsified, and ruled that no
amendment was needed. That ruling was correct when it was made. The byte-walking
claim became false later in the same increment, once the `std/bytes` and
`std/strings` surface was established, and nobody returned to a ruling a later
finding had invalidated. A check that examines the right things and is then
overtaken is a different failure from a check never made, and only the first is
invisible to a reader who sees the word "checked". That increment's design record
carries the reversal, at
`docs/superpowers/specs/2026-08-27-hashdos-resistance-test-design.md` section 11,
items (a) and (e).

The same stale wording appears elsewhere in the tree —
`crates/nova-runtime/src/lib.rs` above `nova_rt_str_hash`, the
`Builtin::StrHash` and `Builtin::StrLenChars` doc comments in
`crates/nova-resolver/src/lib.rs`, and the `Hash` comment in `std/core/lib.nova`
among them — stale there in the same way. That increment changed no product
code, so those are flagged in its own records rather than edited here, and this
amendment does not close them.

### Amendment, 2026-08-28 — the disclosure closes for `Int`, `Bool` and `Char`; `mix64` is untouched

From the `seeded-mix64` increment, later than both amendments above and written
by a different hand. **Nothing in the Decision moves.** `Hash` keeps
`fn hash(self) -> Int`, the migration stays unstarted with its prerequisites as
recorded, the backend-independence requirement stands, `Hash for Float` is still
absent, and `mix64` and `char_to_int` keep the shapes the supporting-pieces
bullets give them. What changed is the *computation* behind the impls the
Decision names for `Int`, `Bool` and `Char`, and not the one it names for
`String`: each of those three computes `mix64(key ^ int_hash_seed())`, over a
per-process seed drawn inside the runtime from
`std::collections::hash_map::RandomState`, by a call separate from the one the
2026-08-26 amendment's `str_hash` seed uses.

What separates this from that amendment is worth naming, because its wording
leaned on both halves. **An intrinsic was added**: `int_hash_seed`, a
`Builtin::STD_ONLY` builtin backed by `nova_rt_int_hash_seed`, where that
amendment could say "No intrinsic was added". And **`impl Hash` blocks were
edited**, where that amendment could say none was. Neither costs a user anything
to keep compiling — `std` is embedded in the compiler with `include_str!` and
recompiled on every `nova` invocation, and `pub trait Hash { fn hash(self) ->
Int }` is untouched — so this stays a source-compatibility statement and not a
behavioural one. A program that read an `Int`, `Bool` or `Char` hash, or a
`Map`'s `keys()` order over such keys, and expected the same answer from a later
run of the same binary now gets a different one.

**The seed goes into `mix64`'s INPUT, and the placement is load-bearing rather
than stylistic.** The Consequences bullet above requires bucket selection to be
`hash & (cap - 1)`, so a post-XOR `mix64(x) ^ seed` would leave the consulted
bits as `(mix64(x) & mask)` XOR `(seed & mask)` — a permutation of the buckets
that leaves every colliding pair still colliding. It would look like seeding and
buy nothing. `hashdos_precomputed_int_key_set_does_not_survive_a_new_process`
(`crates/nova-cli/tests/run_tests.rs`) executes that distinction rather than
arguing it: one Nova process searches for a 32-key set sharing a bucket under
its own seed, and a second, separately launched process re-hashes that same set
and is asserted to spread it. Its threshold, that threshold's derived bound, and
the coverage it does not give are in its own doc comment and are not restated
here.

#### The paragraph headed "The Phase 2.2a disclosure narrows" is amended as a whole

Its clauses do not all turn over the same way, and the clause-by-clause list
below says which does what: one survives and loses only the inference drawn
from it, the others turn over. The paragraph is amended whole rather than clause
by clause because patching any single clause would leave the others standing.
This increment's commit message states the stronger form, that the paragraph
"turns over in every clause at once"; that is retracted here, and it cannot be
corrected where it stands, a commit message being immutable. The first bullet
below is the clause it is wrong about. The paragraph's wording keeps its place
and this subsection governs it. Of
"**Hashes are not randomized per process**" it says the disclosure is

> now **false for `String` keys** ... and **still true for `mix64`**, hence for
> `Int`, `Bool` and `Char` keys, whose buckets remain a function of the key
> alone and remain attackable by chosen keys. That half is load-bearing and is
> a decision rather than an oversight: `tests/runtime/hash.stdout` pins
> `mix64`'s histograms as a specification of the low-bit spreading `Map`'s
> masking depends on, and the exposed path is not the one `std/json` takes,
> whose object keys are strings.

Clause by clause:

- **"still true for `mix64`" is still true, and no longer carries the
  consequence drawn from it.** `mix64` is the function this ADR describes, bit
  for bit; the seeding sits in the impls that call it. So "hence for
  `Int`, `Bool` and `Char` keys" no longer follows, and the Phase 2.2a
  disclosure is now false for those three as well as for `String`. What stays
  unrandomized is a module-private mixer, not a key type.
- **"whose buckets remain a function of the key alone" is false.** A key's
  bucket is a function of the key and of this process's seed. It is fixed for
  the whole of one run and can differ in the next run of the same binary — not
  must, since two seeds may agree on one key.
- **"remain attackable by chosen keys" now splits the way the 2026-08-26
  amendment records it splitting for `String`.** An attacker who cannot read a
  hash out of the running process can no longer build a colliding set for it;
  one who can read a single primitive hash out of it still can, for the reason
  the seed-recovery subsection below gives.
- **"a decision rather than an oversight" was a decision, and this increment
  took the other side of it.** It was right when it was written: the histograms
  it names were real evidence, and trading a specification for a partial win on
  a path `std/json` does not take was a defensible call on the information then
  available. What this increment established is that the trade was not forced —
  the fixture keeps its oracle property under bounds derived from distributions,
  so the specification did not have to be spent in order to seed the impls, and
  `mix64` itself did not have to be touched. Read the earlier sentence as the
  position this increment argued against rather than as a mistake in the record.
- **The histograms it cites as its reason are deleted.**
  `tests/runtime/hash.nova` printed two `&7` histograms and prints neither now.
  Its `buckets reached` line prints a bound where it printed a count, bounded
  below at 6 of 8 over the 64 multiples of 8; so does its `keys -64..63 reach`
  line, bounded below at 76 of 256 buckets over 128 keys spanning both signs;
  and so does its negative-hash count over the keys 0 to 255, confined to
  `80..176`. One exact count did not move at all and its standing changed
  instead: the complement-collision count still reads `0`, and it is now a
  theorem rather than a measurement over sampled seeds, since `mix64` is
  injective and `x ^ s == (-1-x) ^ s` would need `x == -1-x`, which no `Int`
  satisfies.
  `tests/runtime/collections.nova` separately dropped a printed count of how
  many of 16 `Int` keys hash negative, which under seeding is
  `Binomial(16, 1/2)`, and which `collections_run`,
  `collections_build_standalone` and `collections_under_gc_stress` would each
  have had to agree on in a separately launched process.

#### "Hashes must be backend-independent" keeps its requirement and loses its example

That bullet says `tests/runtime/hash.nova` runs under both backends against the
same fixture,

> whose expected bucket histograms were computed independently from splitmix64's
> finalizer rather than recorded from a run.

**The requirement is untouched and the histograms are gone.** The fixture still
runs under the JIT and the object backend against one golden, and the property
that sentence exists to assert — that the golden is an oracle rather than a
recording of what the implementation happened to print — is intact by a
different route: each replacement bound is computed from a distribution by
finite exact summation over rationals, inclusion-exclusion across an occupancy
distribution or a binomial tail, rather than read off a run. So is the flake
budget the fixture header sums from them.

The header qualifies those figures twice over, and this bullet should be read
with both qualifications. Each such figure is **exact for a model** in which
the hashes involved are independent and uniform, and they are not independent —
every one of them is a deterministic function of a single 64-bit seed. So read
each figure as exact for the model and as an estimate for the run. And any
figure that sums non-disjoint events is an upper bound within that model, which
the header labels Bonferroni where that applies. The derivations, with the
figure for each line, are in `tests/runtime/hash.nova`'s header and are not
restated here.

The complement-pair and both-signs regressions this bullet points at are still
in that fixture, and the header records what the golden split cost them: the
both-signs bucket count that used to move under a missing second shift mask is
now a bound wide enough to admit that mutant's value, which is why the canonical
splitmix64 vectors are still asserted there rather than dropped.

#### The privacy decision and the reserved freedom were tested and held

The Consequences bullet on composite `Hash` says a combiner cannot call `mix64`,
"which is module-private to `std/core` on purpose — it is an implementation
detail of the primitive impls, not a published utility, and publishing it would
make its exact definition a compatibility surface", and the Migration path's
closing paragraph reserves "replacing `mix64`'s constants or rounds". During this
increment a ruling to publish `mix64`, so that the fixture could call it
directly, was made and then **retracted on exactly those grounds**. `mix64`
is still module-private, and the freedom to replace its constants or rounds is
still reserved rather than spent. The canonical splitmix64 vectors stayed in the
fixture by another route: it recovers the seed, XORs it into each vector's input
so that `.hash()` cancels it, and carries its own inverse of `mix64`'s
arithmetic to do the recovery. The cost is that the inverse has to change if
those constants ever do, which the fixture names at itself.

#### The seed-recovery disclosure gains its second half

The 2026-08-26 amendment above — not the privacy-decision subsection
immediately preceding this one — records that `("").hash()` recovers the string
seed exactly in one call. **The `Int` seed has the same channel**, stated here
rather than left to be inferred: `(0).hash()` is `mix64(0 ^ seed)`, which is
`mix64(seed)`,
and `mix64` is a bijection, so one call plus an inverse yields the seed exactly.
That is no longer only asserted — `tests/runtime/hash.nova` carries a `mix64_inv`
and performs the recovery, because cancelling the seed is how it reaches the
canonical vectors at all.

The consequence takes the shape that amendment already uses. An attacker who
can obtain one `Int`, `Bool` or `Char` hash from a running process can recover
that process's seed and construct collisions for it; an attacker who cannot
observe the process still cannot build a colliding set before it starts, the
seed being drawn per process. Precomputation resistance is the half that
survives. The channel is not a defect in the seeding: it is
`fn hash(self) -> Int` handing a caller a whole 64-bit result in one call, which
is what this ADR decided and which predates both seedings. **So the `Int`,
`Bool` and `Char` path is exactly as strong as the `String` path and no
stronger**, and no record should be read as claiming otherwise. The two seeds
are separate draws, which buys that they differ **effectively certainly** —
equality is possible by chance at about 2^-64 — rather than that they are
independent; whether recovering one yields the other is a question about the
keyed hash `RandomState` supplies under a related key, and this increment does
not answer it. `int_hash_seed`'s doc comment in
`crates/nova-runtime/src/lib.rs` carries that argument, read off `std`'s own
source — how `RandomState::new` hands out `(k0, k1)` from a single cached draw,
so that two calls share `k1` and differ in `k0` — and states outright that this
increment does not verify the related-key question and that
`int_and_str_hash_seeds_are_drawn_separately` does not test it. No measurement
stands behind that argument. The measured seed figures nearby, in
`str_hash_seed`'s comment, are about the string seed and bear on
recoverability, not on the related-key question.

"FNV-1a is not collision-resistant and neither is `mix64`" **stays true as
written**, and so does the sentence that an adversary who can observe timing and
adapt is out of scope. Neither is claimed against here. Nothing in this
amendment reopens the migration question, and `Hash`'s signature is unchanged.
