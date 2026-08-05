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
  - The bound sits on each `impl`, not on the record's generic parameters: a
    bound on a record's generic parameter is rejected with `E0900`, not
    silently accepted (see "Fixed" below). On the impl it is real, and a
    non-`Hash` key is `E0013` at monomorphization. Reachability pruning rooted
    at `main` keeps a program that touches no collection from paying for any
    of it.
    *(Superseded in Phase 2.2d for **records**: a bound on a record's type
    parameter is now accepted, as a resolution scope for projections in field
    types, and is not enforced at construction. The **sum-type** form is
    unchanged and still `E0900`. `Map` and `Set` still carry their bounds on the
    impl, so nothing about this entry's collections changed. See the Phase 2.2d
    entry below and `docs/adr/0007-record-parameter-bounds.md` §1.)*
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
- `std/strings`, Nova's third standard-library module — five new runtime-backed
  intrinsics plus 18 inherent `String` methods, written in Nova (Phase 2.2b):
  - Five new std-only builtins (`Builtin::STD_ONLY` grows from `[Builtin; 3]` to
    `[Builtin; 8]`, so none becomes a reserved word in user code): `str_len_chars`,
    `str_chars` (`String -> [Char]`), `str_from_chars` (`[Char] -> String`),
    `str_to_upper` and `str_to_lower`, each backed by its own `nova_rt_str_*`
    runtime function. `str_chars` is the first intrinsic to construct a Nova
    array from the runtime (`{ len, elem0, elem1, … }`, scanned, matching
    codegen's own array layout byte-for-byte) — a layout mistake there would be
    a silent miscompile, not a crash, so it is pinned by a Nova-level test that
    reads `.len()` back and indexes elements, not by inspecting the Rust code.
  - `std/strings/lib.nova`, the third embedded std module (same compile model as
    `std/core` and `std/collections`: `include_str!`, appended last, glob-imported
    at lowest priority, silently shadowable — ADR 0004), holding the language's
    first inherent `impl String` block, with 18 methods: `len`, `is_empty`,
    `chars`, `char_at`, `slice`, `contains`, `starts_with`, `ends_with`,
    `index_of`, `split`, `trim`, `trim_start`, `trim_end`, `to_upper`, `to_lower`,
    `repeat`, `reverse`, `join`. **Every index and length is in codepoints
    (Unicode scalar values), never bytes** — `"café".len()` is 4 though its UTF-8
    is 5 bytes, and `"日本語".len()` is 3. Consequently these 18 names are now
    reserved on `String`, but by *shadowing* rather than by conflict: an
    inherent method wins by priority over a same-named trait method, so a user
    trait implementing e.g. `trim` for `String` still compiles and `s.trim()`
    silently resolves to the std method instead — gentler than the `E0015`
    ambiguity a second *trait* impl would cause, but still a permanent
    commitment.
  - Error-handling shape follows the `std/collections` precedent: `char_at`
    returns `Option<Char>` (`None` for an out-of-range *or* negative index,
    matching `Vec::get`); `slice(start, end)` panics on an invalid range
    (`start < 0`, `end > len`, or `start > end` — `start == end` is valid and
    yields `""`), matching `Vec::set`; `index_of` returns `Option<Int>` rather
    than encoding absence as `-1`. `split`'s pinned semantics: a missing
    separator yields a one-element array holding the whole string, never an
    empty one; adjacent/leading/trailing separators produce empty pieces with
    no collapsing; an empty separator splits into single codepoints (the
    JavaScript behaviour — Rust adds boundary empties, Python raises, so there
    is no consensus to inherit) and `"".split("")` is `[]`. `join` hangs off the
    separator (`",".join(parts)`, Python-style) rather than being a free
    function, so it does not take the name `join` away from every module via
    glob import. Case mapping (`to_upper`/`to_lower`) is whole-string, not
    `Char -> Char`, because it is not always 1-to-1: `"ß".to_upper()` is `"SS"`
    (2 codepoints, longer than the input) and `"İ".to_lower()` is `"i"` plus a
    combining dot-above (2 codepoints).
  - Deliberate limitations, accepted for this increment rather than overlooked:
    the `trim` family's whitespace test is an explicit list (space, `\t`, `\n`,
    `\r`, and four common Unicode spaces), not Unicode's full `White_Space`
    property; every method that decodes the string at all — `char_at` (the one
    the module's own header flags as the quadratic hazard when called in a
    loop), `slice`, `starts_with`, `ends_with` (which decodes twice, once per
    operand), `contains`, `index_of`, `split`, the `trim` family, `reverse`,
    `repeat`, `join`, and `std/core`'s `Debug for String` — decodes the whole
    string to a `[Char]` first, so each call is O(n) allocation — a 1 MB
    haystack allocates roughly 8 MB — accepted because the Nova-level API is
    unchanged if a `str_find` fast path is ever added underneath it; and there
    is no `replace`, no `pad_start`/`pad_end`, no `split_once`, and no
    `String -> Int`/`Float` parsing.
  - The whole module is exercised end-to-end by `tests/runtime/strings.nova`
    under `nova run`, `nova build` **and `NOVA_GC_STRESS=1`**: byte-vs-codepoint
    length, `chars()`'s array read back from Nova, both `char_at` boundaries,
    `slice`'s half-open boundary plus a nonzero-`start` offset with a
    multi-byte prefix, a round-trip through `slice`+`join` for ASCII/accented/
    CJK/emoji input, every pinned `split` row including a self-overlapping
    separator, search boundaries (an anchored vs. merely-occurring-somewhere
    needle, an odd-index mismatch inside the shared `chars_match_at`
    primitive that backs `starts_with`/`ends_with`/`index_of`/`contains`/
    `split`, empty needle, same-length haystack/needle), the trim family's
    own all-whitespace fallback, an odd-length whitespace run, non-ASCII
    whitespace and `\r`, `repeat`, `reverse`, and whole-string case mapping
    including both directions on `""`.
- Deferred from `std/strings` (Phase 2.2b), each blocked on a language feature
  or a scope decision rather than on effort: `replace`, `pad_start`/`pad_end`,
  `split_once`; `String -> Int`/`Float` parsing (needed by `std/json` later, but
  it raises its own questions — radix, overflow, leading `+`, surrounding
  whitespace — that would widen this increment); grapheme-cluster segmentation
  (Nova's `Char` is a Unicode scalar value, not a grapheme); an exact
  `char::is_whitespace` intrinsic (the approximate list above stands in for
  it); and `nova_rt_str_find`/other fast paths for the O(n)-per-call cost
  noted above.
- **Associated types (Phase 2.2c)** — `trait Iterator { type Item }` and
  projections written `Self::Item` / `I::Item`. A trait may declare associated
  types; an impl binds each one (`type Item = T`), and conformance checks the
  set in both directions — `E0070` for a binding the trait requires and the
  impl omits, `E0071` for one the trait never declared.
  - **Syntax is `::`, deviating from `nova-spec/20-STDLIB.md:95`, which wrote
    `Self.Item` with a dot** (ADR 0006, and the spec is corrected). `::` reuses
    a path form the parser already produced — `A::B` in type position always
    parsed and was rejected by *typeck*, so the projection syntax cost zero
    parser work — and it is already Nova's reach-into-a-type operator
    (`P::new()`, `T::default()`). `Self.Item` does not parse at all.
  - **Represented as `Ty::Assoc { on, assoc }`**, where `assoc` is the
    associated type's own `DefId` under a new `DefKind::AssocType`. Resolved by
    **normalization at seams, not by deferred obligations**: the unifier is a
    210-line Robinson engine whose entire state is `vars: Vec<Option<Ty>>`, with
    no impl table and no constraint queue, and giving it one would have been the
    larger change. The shared core is `hir::normalize_ty(&Ty, &[ImplInfo])` in
    `nova-hir` — a free function over a slice, which is the one signature both
    the type checker (`&self.impls`) and monomorphization (`&module.impls`) can
    satisfy. It is never called from `unify`.
  - Normalization runs **wherever a checked type is consumed** — considerably
    more places in the type checker than the design's three predicted seams —
    plus impl signature checking and, after `subst`, monomorphization. (No count
    is given deliberately: three readers counting the call sites during review
    got three different answers depending on whether they counted logical seams
    or `self.normalize(` calls, and a number here would go stale on the next
    task. `grep` is the authority.) Impl signature checking needed a **separate
    pass after the impl table is complete**: `collect_impls` calls conformance
    ten lines before pushing the `ImplInfo`, so normalizing in place cannot see
    the impl being checked, and hoisting the push instead would make resolution
    depend on declaration order — which Nova deliberately does not have for
    impls.
  - **`Self` is no longer a legal generic-parameter name (`E0076`).** It had
    been accepted, so `impl<Self: It> W<Self>` type-checked with `Self` meaning
    an ordinary parameter rather than the impl's self type — two meanings for
    one token in one scope.
  - Cyclic bindings are rejected (`E0077`, `type Item = Self::Item` and mutual
    chains), while the legitimate chain `type A = Self::B` / `type B = Int`
    still resolves. Normalization is bounded by **two** independent allowances
    and reports rather than diverging: a depth limit (`E0078`, reachable from a
    chain longer than 64) and a total-work allowance (`E0078`, for a *branching*
    chain, which is exponential in depth). Both are load-bearing and measured:
    dropping the work allowance makes a 58-line accepted program take longer
    than 60 seconds; dropping the depth limit overflows the stack.
  - A projection that somehow survives to monomorphization is `E0079` rather
    than reaching code generation. That path is not reachable from source today
    — every probe of it hit an earlier diagnostic first — and is pinned by a
    `nova-mir` unit test, which is the honest way to test a backstop.
  - **The mutable-receiver rule now covers trait methods** (`E0060`), closing
    ADR 0005 §1's documented gap, which `Iterator::next(mut self)` required.
    The check sits at the single point where a trait call's receiver is emitted,
    because the gap turned out to have **five** routes, not one: a direct call,
    a generic bound, a supertrait bound, a trait default body delegating to a
    mutator, and string interpolation reaching a `fmt(mut self)` through a path
    that bypasses ordinary method dispatch entirely.

    **This is a behaviour change, not only an addition.** Code that compiled
    before may now report `E0060`: calling a `mut self` trait method on an
    immutable binding was silently accepted and did mutate. The fix at a call
    site is `let mut x = …`; at a function parameter it is `mut x: T` in the
    signature — note the `E0060` message currently suggests `let mut` even for
    a parameter, which is the wrong advice for that case and is queued. Nothing
    in `std` relied on the permissive behaviour — `VecIter::next` is std's only
    `mut self` in a trait impl and nothing in `std` calls it; every other
    `mut self` method there is in an inherent impl and takes the pre-existing
    route — and no gate fixture output moved.
  - Deliberately out of scope: `Map::iter()` yielding key/value pairs, which
    needs tuples; generic associated types (`I::Item<Int>` is `E0012`); and
    bounds on an associated type (`type Item: Display` is `E0900`).
  - **Known limitation — a projection parameter must not precede the parameter
    that determines it.** `fn f<I: It>(y: I::Item, x: I)` *declares* fine, but
    every call reports `argument to 'f' has type 'Int' but '?0::Item' was
    expected`: `I` is not yet pinned when the first argument is checked, and
    normalization has nothing to resolve. **There is no workaround** —
    annotating the argument does not help — so put the determining parameter
    first (`fn f<I: It>(x: I, y: I::Item)`), which works. This is the price of
    resolving projections at seams instead of deferring them to a constraint
    queue, and the `?0` in that message is an internal inference variable that
    should not be user-visible; both are recorded in the design doc §4.2.
  - **`Iterator` in `std/core`, `VecIter<T>` and `Vec::iter()` in
    `std/collections`** — the consumer the whole increment exists for.
    `pub trait Iterator { type Item  fn next(mut self) -> Option<Self::Item> }`,
    with `pub record VecIter<T> { v: Vec<T>, i: Int }` and
    `impl<T> Iterator for VecIter<T> { type Item = T }`. `iter()` went into
    `std/collections`' **existing** `impl<T> Vec<T>` block rather than a second
    inherent impl on the same type, which nothing in std or the test suite
    exercises. `Item` is bound to the impl's own parameter, not to a primitive,
    so every projection through it goes through `subst` — a monomorphic
    `type Item = Char` would have left that path untested while appearing to
    pass. The impl writes `-> Option<T>` rather than echoing
    `-> Option<Self::Item>`; both are accepted, and the equivalence was checked
    on the shipped impl, not only on a test-local trait.
    - **An iterator must be held in a `mut` binding or arrive as a `mut`
      parameter**, or `next()` is `E0060`. That is the visible face of the
      `mut self` trait-method rule above, and `mut` on a *parameter* is what
      carries it when the iterator arrives as an argument — there is no `let mut`
      to reach for there. (The `E0060` note still advises `let mut` on that
      route, which is wrong advice for a parameter; queued.)
    - `mut self` on `next` is load-bearing, not stylistic: with plain `self` on
      both the trait and the impl, `VecIter::next`'s own body does not compile
      (`E0060`, cannot assign to a field of immutable self). Measured, which is
      why the trait could not be declared before ADR 0005 §1's gap closed.
    - **`Iterator` is implemented for no primitive**, unlike the six traits
      above it in `std/core`, so `next` is *not* taken away from user code on
      `Int`/`Float`/`Bool`/`Char`/`String` the way `fmt`/`eq`/`cmp`/`clone`/
      `default` are (ADR 0004, "method names are not soft-reserved").
    - `VecIter` holds the `Vec` by pointer, so it **aliases** the caller's
      storage rather than copying it: a `push` during iteration is visible to
      the iterator, and an element appended after `next` has already answered
      `None` is still yielded by the following call. Documented and pinned by a
      test rather than prevented — preventing it needs borrow tracking the
      language does not have. Record field visibility is also parsed and never
      enforced, so `VecIter`'s cursor (like `Vec`'s `len`) is writable from any
      user program; pre-existing, and bounded — breaking the invariant produces
      a bounds-check abort, not memory unsafety.
    - Iterating today means a hand-written `while` plus a `match` on the
      `Option`. Still absent, all deliberate: **no `for x in it`** desugar; **no
      default methods**, so no `map`/`filter`/`collect`/`fold`; **no `Set` or
      `String` iterator** (`chars()` already returns an indexable `[Char]`, so
      nothing regresses); **no `IntoIterator`**; and no backwards inference —
      unifying a projection with a concrete type never deduces its `Self`.
  - **The gate:** `tests/runtime/assoc_types.{nova,stdout}`, run three ways
    (`assoc_types_run`, `assoc_types_build_standalone`,
    `assoc_types_under_gc_stress`) — a fourth committed fixture beside
    `collections`, `std_core` and `strings`, and the first coverage of
    associated-type code through the object-file backend and under
    `NOVA_GC_STRESS=1` at all.
    - One measured fact from building it is worth recording, because it makes an
      obvious-looking test toothless: **`mir_ty` maps `Int` *and* `Char` to
      `MirTy::I64`, and `String`/`Record`/`Sum`/`Array` to `MirTy::Ptr`, which is
      `pointer_type()` — `types::I64` on x86-64.** So at the level a backend can
      see, `Int`, `Char`, `String` and every heap type are one type. A generic
      function naming a projection resolved to the *wrong* one of them
      miscompiles silently: `fn first_or<I: Iterator>(mut it: I, dflt: I::Item)
      -> I::Item` at `Vec<Int>` + `Vec<String>` survives a mutation that makes
      monomorphization's normalization cache its first answer, byte-identically,
      and so does `Vec<Int>` + `Vec<Char>`. Only `Bool` (`I8`) and `Float`
      (`F64`) have distinguishable machine classes. Every generic block in the
      fixture therefore instantiates at `Bool` and at `Float`, and each one
      independently kills that mutation.
  - **Three constructs that compiled before now do not.** All three were
    silently accepted, and each was found by review or probing rather than
    predicted:
    - **A projection in an impl's self type** — `impl<T: It> Tr for W<T::Item>`
      — is now `E0900`. It type-checked, but impl selection recovers an impl's
      arguments by structural matching and cannot invert a projection, so such an
      impl could never be selected; worse, it was invisible to overlap checking,
      so it coexisted with `impl Tr for W<Int>` without the `E0074` that pair
      would otherwise get. Dead code that also punched a hole in coherence.
    - **A trait declaring the same associated type twice**, and — the same
      defect wearing different clothes — **a trait declaring the same method
      name twice** — are now `E0403`, the existing duplicate-name code. Both
      previously kept one binding silently.
  - **Fixed, all pre-existing and none introduced by this increment:**
    - `Ty::Error` no longer reaches a user-facing `E0072` as the literal
      `{error}`. A poisoned or unresolvable associated-type binding produced its
      real diagnostic *plus* a meaningless follow-on comparing against
      `{error}`. The cause is worth recording: `Ty` derives `PartialEq` with no
      `Error` absorption, so at the impl signature comparison an `Error` on one
      side **forces** a mismatch — the exact opposite of its behaviour at
      `unify`, where it absorbs. The guard is transitive, because
      `Option<{error}>` is a `Sum`, not a `Ty::Error`.
    - **An impl may now echo a supertrait's associated type.** A trait method
      could name `Self::Elem` inherited from a supertrait, but its impl writing
      the same signature reported `E0001` — the trait side resolved against the
      expanded supertrait bounds and the impl side did not.
    - **Parser recovery no longer escapes an impl body.** One bad token inside
      an `impl` consumed every following top-level item, because the item-boundary
      sync did not treat `}` as a stop — so a following `record` was reported as
      an illegal impl member and `fn main` was parsed *into* the impl and
      discarded with it. Fixing it needed a checked-progress guard rather than
      the obvious "advance first": `parse_file`'s own loop syncs *without*
      advancing, and `}` is the first stop it has no arm for, so the obvious fix
      made `nova check` hang on a two-line file — measured, and caught only
      because the plan required verifying the invariant it also asserted.

- **Iteration (Phase 2.2d)** — `for x in it` over any `Iterator`, plus six
  default methods, so iterating no longer means a hand-written `while` and a
  `match` on the `Option`.
  - **`for x in it` desugars to the loop you would have written**:
    `let mut __it = it` and then `while true { match __it.next() { Some(x) =>
    body, None => break } }`. The hidden iterator is bound `mut` regardless of
    the source expression's mutability, so `for x in <an immutable local>`
    advances that local; the loop variable itself stays immutable, exactly as in
    the range form. The integer-range form (`for i in 0..n`) is unchanged and
    still takes its own counter-driven path.
  - **`for x in v` is NOT supported — write `for x in v.iter()`.** There is no
    `IntoIterator`, deliberately, so a container is not an iterator. `for x in 5`
    and every other non-iterator reports `E0900` naming both accepted forms. The
    desugar keys on `next`'s *shape* rather than on std's `Iterator`'s identity —
    the same duck-typing string interpolation uses for `fmt` — so a user trait
    declaring `next` is as good an iterator as std's.
  - **Six default methods**: `map`, `filter`, `fold`, `count`, `any`, `collect`.
    `map` and `filter` are **lazy** — each returns an adapter record
    (`MapIter<Self, U>`, `FilterIter<Self>`) that pulls one element per `next`,
    so nothing is consumed until a consumer runs, and `f` is never called for
    elements nobody asks for. `collect` returns `Vec<Self::Item>`; `any`
    short-circuits (which is why it is not written over `fold`). Adapters chain
    on adapters, so `v.iter().filter(f).map(g).collect()` works as one
    expression.
  - **`collect` makes `std/core` depend on `std/collections` for the first
    time**, because it returns a `Vec`. Accepted deliberately: one method and one
    type, and the whole-program merge means there is no layering *mechanism* to
    violate, only a convention. The alternative, `Vec::from_iter(it)` in
    `std/collections`, keeps `std/core` free of collections and reads worse at
    every call site.
  - **A bound on a record's type parameter now resolves projections in field
    types, and is NOT enforced at construction** (`docs/adr/0007-record-parameter-bounds.md`
    §1). `record MapIter<I: Iterator, U> { it: I, f: fn(I::Item) -> U }` used to
    be `E0900`; it is now accepted, and the bound is a **resolution scope** —
    it exists so the field type may name `I::Item` at all. It is deliberately
    *not* a constraint, because `MakeRecord` carries no type arguments and MIR
    erases records to `Ptr`, so there is nowhere for the check to live that would
    fire reliably. This is the one thing in this increment a reader could
    reasonably be surprised by, so: what happens instead is **three different
    answers**, not one. Where a field type names the projection (std's two
    adapters), a wrong instantiation is `E0079` **at construction**, even
    undriven — earlier and stricter than the bound would have been. Where it does
    not, but a bounded impl method is instantiated, it is `E0013`. Where neither
    holds, it **compiles, runs and prints, with no diagnostic** — a residual hole,
    pinned by a test as accepted so it cannot be rediscovered as a bug or closed
    silently. A bound on a *sum type*'s parameter is still `E0900`.
  - **`Iterator`'s four consumers take plain `self`, not `mut self`** (ADR 0007
    §2, which amends ADR 0005 §1). `mut self` rejects a temporary receiver, so
    `v.iter().filter(f).map(g).collect()` — the form this increment exists for —
    was `E0060`. The consumers now open with `let mut it = self` and drive `it`.
    `next` remains the only `mut self` method, so driving an iterator by hand
    still needs a mutable binding. **The cost is real and is not the goal:** the
    same relaxation also lets a consumer accept an *immutable* local and advance
    it silently (`let it = …; it.count()` twice gives `3` then `0`, no
    diagnostic). ADR 0005's promise that "every std API that mutates must declare
    `mut self`" no longer holds, and ADR 0005 has been amended in place to say so.
  - **Adapters share their source by pointer, so mutating a source mid-iteration
    is observable.** A `MapIter` holds its inner iterator the way `VecIter` holds
    its `Vec` — records are heap objects, and there is no copy anywhere — so
    advancing an adapter advances its source, and a `push` to the underlying
    vector partway through a chain is visible to it. The same alias visibility
    `VecIter` already documented; preventing it needs borrow tracking Nova lacks.
    It is also what makes `let mut it = self` inside a consumer correct rather
    than merely convenient.
  - **The gate:** `tests/runtime/iterator.{nova,stdout}`, run three ways
    (`iterator_run`, `iterator_build_standalone`, `iterator_under_gc_stress`) —
    joining `collections`, `std_core`, `strings` and `assoc_types` among the
    fixtures driven through all three of those configurations. Every generic
    block in it is instantiated at `Bool` **and** at
    `Float`, which is not decoration: `mir_ty` collapses `Int` and `Char` to
    `MirTy::I64` and every heap type to `MirTy::Ptr`, so those are one machine
    class and a wrong `Item` hides in them. Measured on this fixture — an
    `Int`-only reduction of it survives a mutation that makes monomorphization's
    normalization cache its first answer **byte-identically**, and of the two
    distinguishable classes only the `Float` lines catch that one while the `Bool`
    lines pass. `Float` is the strictly stronger of the two — `Bool` is `MirTy::I8`
    and its only values 0 and 1 survive an `I64` confusion intact in the low byte
    of a register, whereas `Float` is `MirTy::F64` and crosses register banks — so
    the fixture carries both but the `Float` half is the one that must not be
    trimmed. (`assoc_types`' header records its own kill on the `Bool` half of the
    *same* mutation, so what flips which class catches it is the usage shape, not
    the mutation.)

### Changed (Phase 2 — behaviour changes, Phase 2.2c)

Filed here as well as under Added, because these change the meaning of code that
already compiled. Full detail is in the associated-types entry above.

- **A `mut self` trait method now requires a mutable receiver** (`E0060`), closing
  ADR 0005 §1's gap. Calling one through an immutable binding was silently
  accepted and did mutate. Fix: `let mut x = …` at a call site, `mut x: T` at a
  parameter. Reachable through five routes including string interpolation, so a
  program can hit this without an obvious method call.
- **Three constructs that compiled are now rejected**: a projection in an impl's
  self type (`E0900`), a trait declaring the same associated type twice, and a
  trait declaring the same method name twice (both `E0403`). All three were
  silently accepted, and the first was invisible to overlap checking.

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
  compiled and ran with a `NoHash` that had no `Hash` impl.
  *(Superseded in Phase 2.2d for **records only** — and note that the
  `Keyed { k: NoHash { … }, v: 2 }` behaviour described here is deliberately back:
  it is case 3 of `docs/adr/0007-record-parameter-bounds.md` §1, accepted so that
  a field type may name a projection on a bounded parameter, which is what lazy
  iterator adapters require. The mechanism sentence above — no bounds on
  `RecordType`, mono discharging only function and impl bounds — is still exactly
  true, and is the reason the bound is still not enforced. The **sum-type** half
  of this entry stands unchanged.)* Enforcing the bound
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
- Record field *assignment* diagnostics now match the field *read* path's
  wording: an unknown field on a record reports "no field `x` on record `P`"
  (it used to say "no field `x` on type `P`"), and a receiver that is not a
  record at all now gets its own "cannot access field `x` on `Int`" message
  instead of being folded into the unknown-field one — the distinction the
  read path already made. `check_field_set` also no longer drops an
  independent mistake on the right-hand side when the field name itself is
  wrong: `p.nope = undefined_fn()` now reports `undefined_fn` as unresolved
  too, rather than only the unknown-field error, matching how the array path
  (`a[i] = undefined_fn()`) already behaved. The cascade guard for a receiver
  that is already `Ty::Error` is unchanged — it still reports exactly one
  error, not two.
- `Debug for String` now escapes its contents into a valid Nova literal
  (Phase 2.2b): `("a\"b").dbg()` previously produced `"a"b"`, which is not
  valid Nova source — noted as a known limitation in the `std/core` entry
  above. The fix reuses `Debug for Char`'s existing per-character escape
  table (`\\`, `\n`, `\t`, `\r`, `\0`) through one shared private helper,
  decoding the string with the new `str_chars` builtin (see `std/strings`
  above) rather than the dedicated `nova_rt_str_escape` that `std/core`'s
  stale comment had predicted — so the fix needed no new ABI symbol.
  `String` escapes `"` where `Char` escapes `'`, and additionally escapes
  every `$` as `\u{24}`: a string literal (unlike a char literal) opens an
  interpolation hole on `$` immediately followed by `{`, so a literal `$`
  left unescaped in the output could silently reopen one when pasted back
  as source — the whole-branch review caught this gap in the initial fix,
  where a string built to contain `${` still printed it unescaped. With that
  arm in place, both `Debug for Char` and `Debug for String` round-trip back
  through the lexer to the original value.

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
