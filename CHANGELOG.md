# Changelog

All notable changes to Nova are documented here.

Format follows [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).
Nova uses [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

---

## [Unreleased]

### Added (Phase 2 — standard library core, in progress)
- Module system (Phase 2.0): multi-file programs with `import`. One file is one
  module (its file stem); `import m` brings all of module `m`'s `pub` items into
  scope, `import m::{a, b}` brings the named ones, and only `pub` items are
  importable — private items stay module-local (ADR 0003). The driver loads the
  entry plus every transitively imported `<name>.nova` beside it; the resolver
  builds per-module namespaces, enforces visibility, and merges all items into
  one program so whole-program monomorphization is unchanged. Cross-module
  records, functions, and traits (including generic bounds over an imported
  trait) work under both backends. Dangling imports and private-item imports
  report `E0001`. (`import as` aliases and qualified `m::name` paths are later
  increments.)
- Method-level generics (Phase 2.0): an inherent method may introduce its own
  generic parameters on top of the impl's — e.g. `impl<T> Box<T> { fn map<U>(self,
  f: fn(T) -> U) -> Box<U> { … } }`. The method's parameters are inferred from
  the call arguments (the impl's from the receiver), inline bounds (`<U: Show>`)
  are enforced at monomorphization, and each concrete instantiation is
  monomorphized separately. Implemented via a flat generic layout — impl
  parameters first, method parameters after — so substitution, bound checking,
  and monomorphization stay uniform. Generic parameters that shadow an impl
  parameter report `E0403`.
- Generic trait methods (Phase 2.0): a trait method may now declare its own
  generic parameters — `trait Mapper { fn remap<U>(self, f: fn(Int) -> U) -> U }`
  — in the trait declaration (required or default-bodied) and in its impls. The
  method's type arguments are inferred per call site (`Self` at `Param(0)`, the
  method's generics after) and threaded into monomorphization so each concrete
  instantiation is compiled separately; inline bounds are enforced at
  monomorphization (`E0013`). An impl method whose generic arity *or* bounds
  disagree with the trait's is rejected (`E0072`), so the trait method signature
  the caller programs against is the contract every impl honors — an impl may
  neither drop nor add a method-generic bound. Dispatch works both on concrete
  receivers and through a `T: Trait` bound in a generic function. `async` on a
  trait-method declaration is rejected (`E0900`) like every other method site.
- Duplicate generic parameter names are now rejected (`E0403`) at every site a
  generic list can be written — free functions, records, sum types, trait
  methods, and `impl` blocks — not only impl methods. A silent duplicate had
  kept just the last binding, leaving the earlier parameter a phantom the
  program could never name.
- `where` clauses (Phase 2.0): an out-of-line spelling of generic bounds on
  functions, impl blocks, and inherent methods — `fn label<T>(x: T) -> String
  where T: Show { … }` is equivalent to the inline `<T: Show>`, and
  `impl<T> Box<T> where T: Show { … }` is the conditional impl `impl<T: Show>`.
  Bounds accumulate on top of any inline bounds and are enforced at
  monomorphization. A `where` clause may only constrain one of the item's own
  type parameters; constraints on concrete/compound types (`where Box<T>:
  Trait`) and on trait methods are rejected with `E0900`.
- Prelude (Phase 2.0): `Option<T> = | Some(T) | None` and `Result<T, E> =
  | Ok(T) | Err(E)` are now built in — available in every module with no import
  or definition. They are compiled as an implicit module and glob-imported into
  every user module, so they use the ordinary generic sum-type and
  monomorphization machinery (and cost nothing when unused). A program may still
  define its own `Option`/`Result`, which shadows the prelude.
- extern / FFI (Phase 2.0): `extern "C" { fn sqrt(x: Float) -> Float }` declares
  external C functions callable like ordinary functions. Symbols resolve against
  the C runtime with no extra configuration — the Cranelift JIT's dlsym fallback
  under `nova run`, and the system linker under `nova build`. The emitted symbol
  is the raw (unmangled) declared name, imported into both backends. Supported:
  the C ABI (`"C"` or omitted) and FFI-safe scalar types — `Int`↔`int64_t`,
  `Float`↔`double`, `Bool`↔`_Bool`, and a unit (`void`) return. Non-scalar types
  (String, records, arrays — GC heap values), other ABIs, and generic/async/
  `where` extern declarations are rejected with `E0900`; symbols that collide
  with the compiler's own (`nova_*`, `main`) are reserved. Note: because `Int`
  is 64-bit, C functions that use narrower integers (32-bit `int`, e.g. `abs`,
  `getchar`) cannot be declared correctly yet and will truncate — declare only
  `int64_t`/`long long`/`double` C functions for now. (Pointers, strings,
  variadics, `link_name`, and fixed-width C integers are later increments.)
- `panic(msg: String)` builtin (Phase 2.1): aborts the process — prints
  `nova: panic: <msg>` to stderr and calls `std::process::abort()` (no
  unwinding) — via a new runtime function, `nova_rt_panic_str`. Typed
  `Never`, so a `panic(...)` call unifies with whatever type its context
  expects and can stand as a match arm's or `if`-branch's tail expression,
  e.g. the `None`/`Err` arm of `std/core`'s `unwrap()`. Declared in both
  codegen backends' runtime-declaration lists (Cranelift's `ALL_RT`, LLVM's
  `DECLS`). Like `println`/`print`, `panic` is seeded into *every* module's
  scope (`Builtin::GLOBAL`), so it is now a reserved word: a user
  `fn panic(...)` reports `E0002: duplicate definition of 'panic'` with the
  note "`panic` is a compiler builtin". This is deliberate — `panic` is
  user-visible language surface — and is the opposite of `str_cmp` below,
  which is scoped to `std/core` alone precisely so it does *not* reserve a
  name in user code.
- `nova_rt_str_cmp` (Phase 2.1): a runtime function comparing two strings
  byte-lexicographically and returning `-1`/`0`/`1`. Needed because Nova has
  no built-in string ordering to write one *in* Nova source — `String` has
  neither length nor indexing, and `String < String` is `E0013` by design —
  and `std/core`'s `Ord for String` needs one (below).
- Associated-function call syntax, `Type::f(args)` (Phase 2.1): a self-less
  method — one declared with no `self` receiver, in an inherent impl or a
  trait impl — is now callable as `Type::f(...)`, e.g. `P::new()` for
  `impl P { fn new() -> P { ... } }`, or `Int::zero()` for
  `trait Zero { fn zero() -> Self }` + `impl Zero for Int { ... }`. Also
  dispatches through a generic bound inside a generic function
  (`fn make<T: Zero>() -> T { T::zero() }` resolves `T::zero()` to whichever
  concrete impl the call site's `T` turns out to be). `Type` may now also be
  a primitive (`Int`/`Float`/`Bool`/`Char`/`String`) for both inherent and
  trait associated functions — primitive type names previously had no entry
  in the resolver's type namespace at all, so every `Primitive::f()` call
  fell through to a misleading `E0900: module-qualified paths are not
  supported yet`. `std/core`'s `Int::default()` (and the equivalent for
  every other primitive) depends on this.
- Supertraits, `trait Ord: Eq` (Phase 2.1): a trait declaration may name one
  or more supertraits; an impl of the subtrait for a type `R` must be paired
  with an impl of each direct supertrait for that same `R`, or `E0072` names
  the specific supertrait that is missing. A bound `T: Subtrait` (and a
  subtrait's own default-method bodies, reaching the supertrait through
  `Self`) has the supertrait's bounds folded in too, so `std/core`'s
  `Ord`-bounded code can call `Eq`'s `eq`/`ne` without a function separately
  requiring `T: Eq`. Diamond and cyclic supertrait graphs are deduplicated
  and the expansion always terminates. A trait's own `where` clause is now
  parsed and rejected as `E0900` (previously parsed and silently discarded
  with no effect at all — `trait B where Self: A` enforced nothing).
- `std/core`, Nova's first standard-library module (Phase 2.1): real Nova
  source at `std/core/lib.nova`, embedded into the compiler binary
  (`include_str!`) and compiled as one more implicit module — appended last
  and glob-imported into every user module at the lowest priority, so its
  names need no `import` and a user definition of the same name silently
  shadows it (`docs/adr/0004-stdlib-compile-model.md` records this compile
  model and why it is an embed rather than a disk search path or a
  precompiled artifact). Silent shadowing covers the *item* namespaces only:
  a user `impl<T> Option<T>` (or `impl<T, E> Result<T, E>`) that redefines a
  method `std/core` already provides — `map`, `unwrap`, `is_some`, … — is a
  normal overlapping-inherent-impl conflict and reports `E0074: method 'x' is
  defined by multiple overlapping inherent impls`; `std/core`'s impls get no
  immunity from coherence. The method names the six traits claim on the five
  primitives are likewise not shadowable (see ADR 0004's Consequences).
  Contents: `Option<T>`/`Result<T, E>` (previously a
  hardcoded two-line prelude string, now real source checked and diagnosed
  like any other module) gain full method sets — `Option`: `is_some`,
  `is_none`, `map`, `and_then`, `unwrap`, `unwrap_or`, `ok_or`; `Result`:
  `is_ok`, `is_err`, `map`, `map_err`, `and_then`, `unwrap`, `unwrap_or` —
  plus six core traits, each implemented for all five primitive types
  (`Int`, `Float`, `Bool`, `Char`, `String`): `Display` and `Debug` (a
  direct `.fmt()`/`.dbg()` call and a generic `T: Display`/`T: Debug` bound
  now work uniformly across primitives and user types alike; `Debug` quotes
  where `Display` does not — `String` as `"…"`, `Char` as `'…'`, with `Char`
  escaping the backslash, the delimiting quote, and the control escapes the
  lexer accepts, so its output round-trips as a Nova char literal. Known
  limitation: `Debug for String` cannot escape its content, so a string
  containing `"` or `\` debugs to something that is not a valid literal —
  Nova has no way to inspect a string's contents from Nova source, so closing
  this needs a new `std/core`-scoped builtin); `Eq` (`eq`, plus a defaulted `ne`);
  `Ord: Eq` (`cmp(self, other: Self) -> Ordering`; `Bool` orders via
  `if`/`else` and `String` via the new `str_cmp` builtin above — seeded only
  into `std/core`'s own module scope, not a globally reserved word, since
  `String` fails FFI-safety and a `nova_`-prefixed `extern` symbol is
  reserved, ruling out the two ways a library-level string comparison would
  normally reach the runtime); `Clone`; and `Default` (including
  `Default for Char`, `'\0'`).
- Deferred from `std/core` (Phase 2.1), each needing its own design before
  it can be added rather than being an incidental extension of what's here:
  `std/fmt` (richer formatting beyond `Display`/`Debug`), `std/io`,
  `Iterator` (needs a laziness / `for`-loop-desugaring story), `Hash` (best
  designed alongside the collection types it would serve), and `Copy`
  (implicit-copy value semantics, tied to an ownership/move model Nova does
  not have yet).
- Record field assignment, `rec.f = v` (Phase 2.2a): records were immutable
  after construction, which blocked every collection and most future std work.
  Mutability reuses the existing `place_root` chain walk that array element
  assignment already used, so `rec.inner.f = v` and `make().f = v` are rejected
  at the root with `E0060` exactly as `arr[i] = v` is. The store mirrors the
  field *read*'s `8 * index` offset in both backends — the index/type lookup is
  now one shared function — so reads and writes cannot disagree about layout.
  Records are heap objects, so **assignment is alias-visible**: two bindings to
  the same record see each other's writes (`let mut alias = c` then
  `alias.n = 99` changes `c.n`), and the same holds through a `mut self` method
  because the receiver is passed as the same pointer, not copied. That is
  deliberate reference semantics, not an oversight, and it is executed under
  both backends by `tests/runtime/field_assign.nova`. The `E0900` fallback for
  an assignment target that is none of the assignable forms now names all three
  ("a local variable, array element, or record field") instead of only the two
  that predated this change.
- The mutable-receiver rule (Phase 2.2a, `docs/adr/0005-mutable-receivers-and-one-shot-hash.md`
  §1): **calling a method that declares `mut self` now requires a mutable
  receiver place**, reported as `E0060` with the same ``declare it as `let mut
  …` `` note the two assignment forms carry — except when the immutable root is
  a method's own receiver, where the note says to declare it as `mut self`,
  since `let mut self` is not Nova syntax and the advice would be
  unfollowable. All three forms (`arr[i] = v`, `rec.f = v`, and a `mut self`
  call) now share one `require_mutable_place` helper, so the classification, the
  code and the note exist once. Previously `let v = Vec::new()`
  followed by `v.push(1)` was accepted while the equivalent `v.len = v.len + 1`
  was `E0060` — the same effect got two different answers depending on whether
  it was spelled as a field assignment or wrapped in a one-line method, which
  reduced `mut` to gating a syntax rather than an effect. The receiver may be
  any place, not just a bare local (`self.map.insert(k, v)` from inside a
  `mut self` method resolves through the `self` root — that is how `Set` is
  built on `Map`), a temporary receiver (`make().bump()`) is rejected as not a
  place, and the check is a no-op for plain `self` readers, so only the `mut`
  keyword demands anything of callers. Consequently every mutating std API
  declares `mut self` and every caller needs `let mut`. **Known gap, documented
  rather than closed:** trait-method calls are *not* covered — for a generic
  receiver there is no single impl to consult and `hir::TraitMethod` has no
  receiver-mutability field, so `impl Tr for P { fn m(mut self) { … } }` called
  as `p.m()` on an immutable `p` is still accepted. The collections use
  inherent impls only; ADR 0005 §1 records the three-step migration path and
  why closing it first needs a conformance rule for an impl whose receiver
  mutability disagrees with its trait's.
- Repeat-array literal, `[init; n]` (Phase 2.2a): arrays could only come from
  element-by-element literals, so there was no way to allocate one of *runtime*
  length — exactly what a growable collection needs. `init` is a
  **caller-supplied** value rather than a zero or null fill, which is what
  keeps a fresh array from ever holding uninitialized memory and is why no
  `Default` bound is needed anywhere in `std/collections`: `Vec::push` fills
  with the element being pushed, and `Map` fills its key/value arrays with the
  pair being inserted (`state`'s `0` filler happens to be exactly the "empty"
  tag, so a fresh table is empty by construction). `init` is evaluated
  **once**, and that one value is stored into every slot — these are *not* `n`
  copies, so for a heap element type all `n` slots are the same object:
  `[Cell { n: 0 }; 3]` is one `Cell` seen three times and `a[0].n = 42` shows
  through `a[1]` and `a[2]`, and `[Vec::new(); rows]` is one `Vec`, not `rows`
  of them. That is the same deliberate reference semantics as field assignment
  above (Nova has no `Copy` and so no per-slot clone to insert), and
  `tests/runtime/array_repeat.nova` executes the record case under both
  backends. The fill loop is emitted in MIR with the existing block machinery,
  so both backends need only the new `ArrayAlloc` statement. **Both ends of the
  length range abort** rather than being clamped — `[x; -1]` and
  `[x; 1 << 60]` both call the same `nova_rt_panic_str` path, with "array
  length must not be negative" and "array length is too large" — following
  `check_bounds`' abort-on-bad-input precedent. Both bounds are memory safety,
  because the backends compute the allocation size as `8 * len + 8` with
  *wrapping* arithmetic: a large negative length overflows the multiplication,
  and a length above `(i64::MAX - 8) / 8` wraps the size back to negative,
  which `gc::alloc`'s `size.max(8)` clamps to an 8-byte block that the
  deliberately unchecked fill loop then runs off the end of. A clamp instead of
  an abort would also let a clamped-to-zero capacity silently spin a growable
  collection.
- A second embedded std module (Phase 2.2a): `std/core` was loaded through a
  seam that assumed exactly one implicit module. It is now a list, so
  `std/collections` lives in its own file (`std/collections/lib.nova`) with the
  same compile model as `std/core` (ADR 0004 — embedded with `include_str!`,
  appended last, glob-imported at lowest priority, silently shadowable). The
  driver registers a `FileId` per std module so diagnostics still name a real
  file, and the std-only builtin gating now asks whether a module is *a* std
  module rather than *the* std module.
- `Hash` (Phase 2.2a, ADR 0005 §2): `pub trait Hash { fn hash(self) -> Int }`
  in `std/core`, with impls for `Int`, `Bool`, `Char` and `String` and the
  contract that `a.eq(b)` implies `a.hash() == b.hash()`. It is **one-shot**
  rather than `nova-spec`'s streaming `Hasher` protocol, which Nova cannot
  express: a hasher must accumulate into a field, needing `mut` on a parameter
  plus a `mut self` *trait* method — precisely the gap §1 leaves open — and the
  whole mechanism would then rest on alias visibility rather than on anything
  the type says. ADR 0005 §2 records that this is a commitment, not a stopgap:
  `hash` is the trait's only method, so switching shapes would break every impl
  and call site. Backed by `mix64`, the splitmix64 finalizer (module-private, so
  it enters no user namespace) for `Int`/`Bool`/`Char`, and two std-only
  builtins: `str_hash` (over the runtime's new FNV-1a `nova_rt_str_hash`,
  because `String` has no length, indexing or iteration and is not FFI-safe, so
  Nova cannot walk its bytes) and `char_to_int` (Nova has no `as` casts and no
  other `Char` → `Int` conversion; it is the first builtin with no runtime
  function at all, since `Char` and `Int` are both `MirTy::I64` and `nova-mir`
  lowers it to a register move). Being std-scoped rather than global, neither
  becomes a reserved word. **Mask a hash; never shift one and never read its
  high bits** — `hash & (cap - 1)` over a power-of-two capacity is the only
  supported way to get a bucket index, because a hash spans the full `Int`
  range including negatives (so `hash % cap` can be a negative index), the high
  bits are not an independent second hash, and `mix64`'s guarantees are stated
  over its whole 64-bit result. **There is deliberately no `Hash for Float`**, a
  documented deviation from `20-STDLIB.md`: NaN never equals itself, so a NaN
  key would be inserted and then unfindable even by the expression that
  produced it, and `0.0 == -0.0` while their bit patterns differ, so any
  bitwise hash would break the `eq` ⇒ equal-hash contract. That needs a NaN
  decision belonging with the `Ord for Float` caveat, and `float_has_no_hash_impl`
  pins the absence so re-adding it is a deliberate act.
- `std/collections`, Nova's second standard-library module — `Vec`, `Map` and
  `Set`, written **in Nova** (Phase 2.2a):
  - `Vec<T>`: `new`, `len`, `is_empty`, `push`, `pop`, `get`, `set`, `clear`.
    Growth doubles from 4 by allocating `[x; newcap]` with the pushed element
    as the filler and copying the existing elements back; the record object's
    address never changes and the array's only referent is that field, so the
    conservative non-moving collector needs no special handling. `get` returns
    `Option<T>` *by value* rather than the spec's `Option<&T>` — Nova has no
    references, and for heap types the value is the pointer, so it still
    behaves referentially. `set` out of range panics.
  - `Map<K, V>` for `K: Hash + Eq`: `new`, `len`, `is_empty`, `insert`, `get`,
    `contains_key`, `remove`. Open addressing with linear probing over a
    power-of-two capacity, so a bucket is `hash & (cap - 1)`. Removal leaves a
    **tombstone**, which is what keeps probe chains intact across a deletion —
    including chains that wrap past the end of the table — and tombstones count
    toward the 3/4 load threshold, so a remove-heavy workload cannot degrade
    into an all-tombstone scan. `insert` probes *past* a tombstone to either the
    key itself or an empty slot before storing back into the first tombstone it
    passed, so a replacement can never leave a second, permanently shadowed
    copy behind the hole. Growth doubles and reinserts only the live entries,
    which is also what clears the tombstones. `insert` returns the previous
    value; `remove` returns the removed one.
  - `Set<T>` for `T: Hash + Eq`: `new`, `len`, `is_empty`, `insert`,
    `contains`, `remove`, backed by a `Map<T, Bool>` so the probing, tombstone
    and growth logic lives in exactly one place. `insert` and `remove` report
    whether the set changed.
  - The bound sits on each `impl`, not on the record's generic parameters as
    `20-STDLIB.md` writes it, because **a bound on a record's generic parameter
    parses but is silently dropped** by the current compiler — on the impl it is
    real, and a non-`Hash` key is `E0013` at monomorphization. Reachability
    pruning rooted at `main` keeps a program that touches no collection from
    paying for any of it.
  - The whole module is exercised end-to-end by `tests/runtime/collections.nova`
    under `nova run`, `nova build` **and `NOVA_GC_STRESS=1`** (collect on every
    allocation): `Vec` across three growths, `Map` through two rehashes with the
    load-factor arithmetic visible, mid-chain and wrapping-chain removals with
    lookups past the hole, tombstone reuse, replacement-behind-tombstones with
    no shadowed duplicate, a user record as a key/element with its own `Hash`
    and `Eq`, `Map<String, Int>` through the runtime hash, negative `Int` keys,
    and `Set` dedup.
- Deferred from `std/collections` (Phase 2.2a), each blocked on a language
  feature rather than on effort:
  - **Iteration on any collection** — `iter()` and `for x in coll`. Needs
    `Iterator` *plus* associated types (`type Item`), which Nova does not have;
    `for` currently works only over integer ranges. Iterating a `Map`'s pairs
    additionally needs tuples, which Nova also lacks, so even
    `for (k, v) in m` has no expressible element type. This is the single
    biggest gap: today a collection can only be read back through the keys or
    indices the caller already holds.
  - `Queue` / `Deque` — a ring buffer is expressible, but its `pop_front` would
    want the same iteration story to be useful, and `20-STDLIB.md`'s shape is
    not settled.
  - `Vec::with_capacity` — it would need a `T` to fill the reserved slots with,
    and Nova cannot express reserved-but-uninitialized capacity at all (which is
    the same reason `[init; n]` takes a caller-supplied filler).
  - `Hash for Float` — see the `Hash` entry above; it needs a NaN decision.
  - `std/strings` — string operations beyond `Eq`/`Ord`/`Display`. `String` has
    no length, indexing or iteration from Nova source, so every operation needs
    a new std-scoped builtin plus a runtime function; that is a module-sized
    design, not an increment on this one.

### Fixed (Phase 2)
- An allocation whose size is too large to *describe* now aborts with a Nova
  diagnostic instead of a Rust panic and backtrace. `gc::alloc` built its
  `Layout` with `Layout::from_size_align(size, ALIGN).expect("valid heap
  layout")`; at `ALIGN = 16` a size that rounds up past `isize::MAX` makes that
  call fail, and the `expect` ended the process with "thread caused
  non-unwinding panic" — an `expect` on a path reachable from user input, which
  the repo convention forbids. It was reachable at the very top of the *legal*
  array-length range: `[x; MAX_ARRAY_LEN]` asks for `8 * len + 8` =
  9223372036854775800 bytes, which both length guards accept and `ALIGN` rounds
  8 bytes too far. The check now lives in `gc::alloc`, the choke point every
  allocation site in the language funnels through (records, strings, closures,
  sum construction, `Vec`/`Map`/`Set` growth), rather than in one lowering, and
  it asks `Layout::from_size_align` whether the size is legal rather than
  restating its rule. The message names both the request and the limit
  ("allocation of N bytes exceeds the maximum object size of M bytes"), so it
  is distinguishable from a genuine out-of-memory, which still reports
  "memory allocation of N bytes failed" through `handle_alloc_error` and is
  what a merely-too-big length such as `2^40` produces. The neighbouring
  behaviours are unchanged: `[x; -1]` and `[x; 1 << 60]` still abort in the
  lowering's own guards, since those exist to stop a *different* bug (the
  wrapping `8 * len + 8` size arithmetic collapsing to a tiny block).
- A trait bound on a **record** or **sum type** type parameter
  (`record Keyed<K: Hash, V>`, `type Wrap<T: Hash> = …`) is now rejected with
  `E0900` instead of being silently discarded. It parsed and then enforced
  nothing: `hir::RecordType`/`hir::SumType` carry no bounds, and monomorphization
  discharges only function and impl bounds, so `Keyed { k: NoHash { … }, v: 2 }`
  compiled and ran with a `NoHash` that had no `Hash` impl. Enforcing the bound
  instead would need a notion of "record instantiation site" that no pass has —
  a record's type arguments survive only in the enclosing expression's `Ty`,
  `ExprKind::MakeRecord` does not record them, and MIR erases them — so the
  construct is rejected loudly, following the precedent set for
  `trait B where Self: A`. Write the bound on the `impl` block instead
  (`impl<K: Hash + Eq, V> Map<K, V>`), where it *is* enforced; that is what
  `std/collections` already does, so no stdlib or test-suite program changes.
  Bounds on functions, impl blocks, generic trait methods, and `where` clauses
  are unaffected. `nova-spec/20-STDLIB.md`'s `Map`/`Set` declarations, which had
  shown the unenforced form, now show the enforced one.
- A `${…}` string-interpolation hole now ends at the `}` matching its `${`
  rather than at the first `}`, so an expression containing braces works inside
  one — most visibly a record literal (`"${f(R { v: 1 })}"`), nested to any
  depth, and a block expression (`"${if a { 1 } else { 2 }}"`). Previously these
  produced two confusing errors, the first being "expected `}` (in record
  literal), found `}`", and every affected call site had to bind the value to a
  local first. A `}` inside a nested string, char, or raw-string literal within
  the hole is text, not structure (`"${g("}")}"`). A hole left unclosed is now
  reported as "unterminated string interpolation" instead of cascading into
  parse errors. A record literal in a hole also parses where the enclosing
  string sits in an `if`/`while`/`for`/`match` scrutinee.
- Cross-module symbol collision: two modules each defining a same-named item
  (function, generic function, or same-named type's inherent method) no longer
  collapse to one symbol at monomorphization. Symbols are now mangled by their
  owning `DefId`, fixing silent wrong-dispatch and a memory-unsafe type
  confusion; qualified/nested import paths (`a::b`) are rejected with `E0900`.
- A generic sum type used as a record field or a sum-variant payload
  (e.g. `record Slot { tag: Option<Int> }`) no longer gets a spurious `E0012`
  "expects 0 type arguments": type arity is precomputed, so it no longer depends
  on collection order (this also fixes forward-referenced generic records).
- Importing a module that exports a name coinciding with a prelude name
  (`Option`/`Result`/`Some`/`None`/`Ok`/`Err`) no longer raises a spurious
  `E0002`: the prelude is glob-imported last, as the lowest-priority binding, so
  a local definition *or* an import of the same name shadows it.
- Nested generic type annotations whose closing brackets abut (`Option<Option<
  Int>>`, `Box<Box<Int>>`) now parse: a glued `>>`/`>>>` token is split when
  closing generic argument lists (the `>>` right-shift operator is unaffected).
- Calling an `extern` function whose C symbol cannot be resolved no longer
  crashes `nova run` with a Rust panic — the JIT's finalize-time panic is caught
  and reported as a clean `E0902`, mirroring the `nova build` linker error.
- Two modules declaring the same C symbol with conflicting signatures now report
  `E0075` instead of crashing codegen / emitting invalid LLVM IR.
- Self-less methods (Phase 2.1): an impl method declaring no `self` receiver
  had its parameter types silently shifted by one slot — signature
  collection unconditionally prepended the receiver's type ahead of every
  method's declared parameters, even when the method had none. Three
  independent symptoms followed from the one root cause, all now fixed: a
  silent miscompile when the shifted types happened not to conflict (a later
  parameter checked against the wrong declared type, with no diagnostic
  produced at all); a bogus `E0001: no variant 'f' on type 'Type'` when
  calling such a method by qualified syntax (a two-segment path was until
  then understood only as a sum-type variant constructor); and a Cranelift
  ICE that `nova check` had already accepted — `nova check` reported exit 0
  for a self-less method called on an instance (`p.make()`), and only `nova
  run`/`nova build` crashed, with a verifier error ("mismatched argument
  count: got 1, expected 0") surfaced as "internal codegen error (this is a
  compiler bug)". Self-less methods are now tracked explicitly, so their
  signatures are never shifted, and calling one on an instance is a clean
  `E0014`. The same family of bug existed on the trait-method dispatch path
  too — a receiver-less trait method called on an instance ICE'd the same
  way, and a trait/impl pair disagreeing about whether a method takes
  `self`, in either direction, was accepted with no conformance check at
  all — now `E0014` and `E0072` respectively, with no ICE either way.
- `Type::f()` on an inherent impl no longer dispatches by impl declaration
  order. An associated function is selected by the impl's nominal head alone
  (deliberately, so `Box::make(5)` works before the qualifier's type argument
  is known), so two *disjoint concrete* impls of one generic type —
  `impl Box<Int> { fn tag() }` and `impl Box<Bool> { fn tag() }` — were both
  candidates and the first one declared silently won. Coherence does not catch
  that pair either: their self types do not overlap, so there is no `E0074`.
  Reordering the two `impl` lines changed the program's output with no
  diagnostic at all. Now every candidate is collected and an ambiguous
  qualifier reports `E0015`, mirroring the trait-associated-function path; the
  single-candidate case is unchanged.

## [0.1.0] - 2026-07-23

Phase 1 (MVP compiler) milestone. Gate verified: all gate programs compile and
run via `nova run` (Cranelift JIT) and `nova build` (native executables), the
workspace test suite is green, and `clippy -D warnings` + `cargo fmt --check`
pass.

### Added (Phase 1 — MVP compiler)
- `nova run <file>`: compile and execute Nova programs natively via the
  Cranelift JIT; `nova check <file>`: type-check only
- `nova build <file> [-o out]`: compile to a standalone native executable —
  Cranelift object emission with an exported C `main` wrapper, linked
  against the `nova-runtime` static library via the platform linker
  (MSVC `link.exe` through cc-rs on Windows, `cc` elsewhere); gate
  programs produce ~130 KB executables
- `nova-resolver`: item-level name resolution (functions, sum types,
  variants), builtin prelude (`println`, `print`), E0002 duplicates
- `nova-typeck` + `nova-hir`: Hindley-Milner inference with occurs check,
  explicit generics at function boundaries, sum types with minimal
  exhaustiveness checking (E0020), typed & desugared HIR output
- `nova-mir`: monomorphization reachable from `main`, CFG lowering,
  match compilation to switches, short-circuit lowering
- `nova-runtime`: C-ABI strings, console output, sum allocation
  (leaking allocator pending GC — ADR 0002)
- `nova-codegen-cranelift`: MIR → native code via cranelift-jit
- Records: declarations, literals (explicit fields, shorthand, `..base`
  spread), field access, generic records; boxed as tagless heap structs
- Traits: inherent methods, trait declarations with required and default
  methods, trait impls, method-call resolution by receiver type
  (E0015 on ambiguity), generic trait bounds verified at monomorphization
  (E0013), impl conformance (E0070/E0071), static dispatch; string
  interpolation bridges to a user `Display` (`fmt(self) -> String`)
- For loops over integer ranges (`for i in a..b` / `a..=b`), desugared to
  a counter-driven `while`
- Closures (`|x| body`) with by-value capture, and bare functions used as
  values: both compile to fat pointers `{ code, env }` with an env-first
  ABI; lifted to standalone functions and monomorphized like generics
- `break` and `continue` in `while`/`for` loops (E0080 outside a loop);
  `continue` in a `for` still advances the counter
- Top-level `const NAME: T = value` (compiled as a zero-arg function,
  referenced by call); constants may reference other constants; a cyclic
  constant is reported as E0081
- Arrays `[T]`: literals `[a, b, c]`, indexing `arr[i]`, element assignment
  `arr[i] = v` (mutable base required), and `arr.len()`; out-of-bounds
  access aborts with a message (heap layout `{ len, elems… }`)
- Match exhaustiveness and reachability via Maranget's usefulness algorithm:
  a `match` is non-exhaustive (`E0020`) when a wildcard row is still useful
  against the arms, and the diagnostic names witness patterns for the uncovered
  values (`Some(_)`, `false`, `_`); an arm is unreachable (`E0021`) when it is
  useless against the earlier arms. This fixes `match` on `Bool` being rejected
  when both `true` and `false` are covered, and detects redundant arms that the
  previous catch-all-only check missed
- Generic impl blocks: `impl<T> Box<T> { … }` (inherent) and
  `impl<T> Trait for Box<T> { … }` (trait), with the impl's type parameters
  usable in method signatures and bodies; a method is monomorphized per
  instance by recovering the impl's type arguments from the receiver type.
  Conditional impls `impl<T: Bound> Trait for Box<T>` are supported — the
  bound on the impl's parameter is verified at monomorphization (E0013),
  including transitively through nested generic impls (`where` clauses on
  impls remain unsupported)
- Garbage collector: `nova-runtime` now reclaims heap memory with a
  conservative, non-moving mark-and-sweep collector (`gc.rs`), replacing the
  leaking allocator (supersedes ADR 0002). All heap values — records, sums,
  arrays, closures, and strings — route through `gc::alloc`; collection is
  triggered at allocation past a growth threshold. Roots are found by scanning
  the stack plus callee-saved registers (flushed via a small `setjmp` C shim),
  and marking is range-based so interior pointers keep their object alive. It
  needs no codegen support and no external GC library. `NOVA_GC_DEBUG` logs
  collections; `NOVA_GC_STRESS` collects on every allocation (used to validate
  root scanning — the whole e2e suite passes under it). Precise stack bounds
  are implemented on Windows; other platforms retain leak-until-exit for now
- `nova build --release`: optimizing build through a new LLVM backend
  (`nova-codegen-llvm`) that emits textual LLVM IR from MIR and compiles it
  with a discovered LLVM toolchain (`clang`, or `llc`, at `-O2`; override with
  `NOVA_CLANG`/`NOVA_LLC`), then links via the same platform linker as the
  debug build. The IR mirrors the Cranelift backend's layouts and runtime ABI,
  so a program behaves identically across `nova run`, `nova build`, and
  `nova build --release`. Requires LLVM ≥ 15 (opaque pointers); with no
  toolchain found, the generated `.ll` is left in place with a clear message
- Gate programs verified end-to-end: hello-world, fibonacci,
  match-on-enum, generic functions, records, traits, for-loops, closures,
  break/continue, constants, arrays, generic impls (e2e stdout tests under
  both `nova run` and `nova build`)

### Fixed
- Lexer: leading whitespace in a string segment directly after `${expr}`
  was skipped when resuming string mode
- Lexer: position drifted into comment text because the logos wrapper
  advanced by the token length, ignoring skipped comment/whitespace
  trivia — any program with a comment failed to parse
- Lexer: a comment immediately before a string (spurious error cascade)
  or raw string (silent mis-lex to `Ident("r")` + plain string) — comments
  are now skipped before literal dispatch (adversarial review)
- Typeck: a trait impl whose method signature diverged from the trait
  declaration (arity, parameter, or return type) was accepted and
  miscompiled — a wrong parameter type was memory-unsafe; now `E0072`
  (adversarial review)
- `nova check` now runs monomorphization so it catches unsatisfied trait
  bounds (`E0013`) that previously only `nova run`/`nova build` rejected
  (adversarial review)
- Record literal field initializers now evaluate in source order (adversarial review)
- Typeck: closure return type is resolved through inference (closure bodies
  previously returned `()` and discarded their result)
- Typeck: closures now capture all referenced enclosing locals — assignment
  targets and called function values were missed, causing miscompiles or a
  compiler panic (adversarial review)
- Typeck: `for` loops use an independent hidden counter (assigning the loop
  variable is rejected, not silently corrupting the trip count), unscoped
  counter locals (no name capture), and an overflow-safe inclusive form
  (an inclusive range ending at `Int::MAX` no longer loops forever)
  (adversarial review)
- Typeck: an `if`/`match` with a diverging branch (`return`/`break`/
  `continue`) is typed by its non-diverging branch instead of `Never`, and
  the `while` lowering guards a diverging condition — a `Never`-typed
  condition (e.g. `while (if c { return x } else { b }) {}`) previously
  crashed codegen with an internal error (adversarial review)
- Typeck: `break`/`continue` in a loop's own condition now target that loop
  (were rejected as "outside a loop" or mis-scoped) (adversarial review)
- Typeck: a function-typed value that is not a local (e.g. a fn-typed
  constant `CONST(args)`, or a fn returned from a field/call) can now be
  called directly instead of erroring with E0900 (adversarial review)
- Typeck: element assignment `arr[i] = v` now checks the mutability of the
  place's root binding, walking through field and nested-index projections —
  `rec.data[0] = v` and `grid[0][1] = v` on an immutable binding, and
  `make()[0] = v` on a temporary, were silently accepted and mutated
  immutable heap storage; now `E0060` (adversarial review)
- Typeck: a restricted inherent method (`impl<T> Pair<T, T> { … }`) no longer
  shadows an applicable trait method for a receiver it does not fit (e.g.
  `Pair<Int, String>`) — method resolution now falls through to the trait impl
  instead of rejecting the call, and the string-interpolation `Display` bridge
  is likewise no longer blocked (adversarial review)
- Monomorphization: the trait-bound satisfaction check no longer has a
  recursion depth cap that could accept an unsatisfiable bound for a very
  deeply nested conditional-impl type; the recursion is well-founded on the
  finite structure of the type, so deep nests are checked exactly
  (adversarial review)
- Impl selection is now structural at every site. `resolve_method_full` (trait
  dispatch) and the monomorphization bound check selected the *first* impl
  sharing a type head, so a program with two non-overlapping impls for the same
  head (`impl Foo for Pair<Int, Bool>` + `impl Foo for Pair<Int, String>`) was
  accepted or rejected depending on declaration order; both now scan for the
  impl that structurally fits (adversarial review)
- Impl methods are mangled by their full self type, not just its head, so two
  concrete impls sharing a head (`Pair<Int, Bool>` vs `Pair<Int, Int>`) no
  longer collide to one symbol and miscompile each other's calls (adversarial
  review)
- Overlapping implementations are rejected (`E0074`): two trait impls of the
  same trait whose self types share a ground instance, or two inherent impls
  that overlap and define the same method (Phase 1 has no specialization);
  previously dispatch silently depended on declaration order (adversarial
  review)
- An impl type parameter that never appears in the self type is rejected
  (`E0073`) instead of leaking an uninferrable variable that made every method
  on the impl uncallable (adversarial review)
- Exhaustiveness: an empty `match` on a value of generic type (`match x { }`
  where `x: T`) was silently accepted and trapped at runtime; a match with no
  arms is now reported non-exhaustive (`E0020`) for any inhabited scrutinee
  (adversarial review)
- A bare identifier pattern that names a nullary variant of a *different* sum
  type is now rejected (`E0001`) rather than silently treated as a catch-all
  binding, which had masked uncovered cases and produced a spurious
  unreachable-arm warning — matching the `Path`/`TupleStruct` pattern arms
  (adversarial review)
- LLVM backend: a `match` on a `Bool` emitted `switch i64` over the `i8`
  scrutinee, producing a type-mismatched module that LLVM rejects (so
  `--release` failed for any boolean match); the switch now uses the
  discriminant's own type (adversarial review)
- `nova build`: intermediate object/IR files are now named so they can never
  alias the `-o` output path — previously `-o out.ll` (or `-o out.obj`) made an
  intermediate the output file and deleted the built binary on success
  (adversarial review)

### Known limitations / follow-ups (not Phase 1 blockers)
- Precise GC stack bounds are implemented on Windows; other platforms skip
  collection (leak-until-exit) until their stack-bounds query is added
- `nova build --release` needs an LLVM toolchain (`clang`/`llc`, ≥ 15) on the
  machine to produce the final binary from the emitted IR
- Spec drift to reconcile in Phase 2: chumsky 0.9 (spec calls for 0.10),
  `salsa` not yet integrated, `fuzz/` targets not yet written

## [0.0.0] - 2026-05-10

Phase 0 (Foundation) milestone. Gate verified: `examples/01-hello-world` and
`examples/02-fibonacci` parse to AST with zero errors.

### Added
- Initial workspace setup (Phase 0)
- `nova-diagnostics`: error reporting infrastructure with codespan-reporting
- `nova-lexer`: full token set for Nova source files (logos-based)
- `nova-ast`: AST node type definitions
- `nova-parser`: recursive descent + Pratt parser (chumsky-based)
- `nova-cli`: `nova parse <file>` command for parser testing
- CI workflow (cargo test, fmt, clippy)
- Snapshot testing harness via `insta`
- Example files: hello-world, fibonacci
