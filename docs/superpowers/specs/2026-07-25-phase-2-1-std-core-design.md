# Phase 2.1 Design — Compiler Prerequisites + `std/core`

> Status: **approved** (2026-07-25). Scopes `docs/phase-2-plan.md` §2.1 against the
> compiler's verified capabilities. Resolves plan decision 2 (stdlib compile model).

## 1. Goal

Ship the first real standard-library module, `std/core`, written in Nova: `Option`
and `Result` with their method sets, plus the foundational traits (`Display`,
`Debug`, `Eq`, `Ord`, `Clone`, `Default`) and impls of them for the primitives.

Three small compiler features must land first, because `std/core` cannot be
written without them.

**Gate:** a Nova program round-trips `Option`/`Result` through their methods and
prints a custom `Display` type, under both backends (`nova run` and `nova build`),
including under `NOVA_GC_STRESS=1`.

## 2. Scope decisions

Phase 2.1 in the plan is "`std/core` + `std/fmt` + `std/io`". That is not
buildable as written, and this design narrows it deliberately.

| Decision | Outcome | Why |
|---|---|---|
| `std/fmt` + `std/io` | **Deferred** to after async (2.3) | Every I/O signature in spec §4 is `async fn`, over `&mut [u8]` slices, returning `impl Read`. Async, references, and existential returns are all absent. Building a synchronous stand-in would be a spec deviation that gets rewritten in 2.3. |
| `Iterator` | **Deferred** to 2.2 | Needs associated types (`type Item`) — a major type-system feature. It has no consumers until `std/collections` provides `Vec`/`Map` to iterate, so associated types get designed alongside their first real users. |
| `Hash` / `Hasher` | **Deferred** to 2.2 | Only meaningful once `Map` exists. |
| `Copy` | **Cut** | A marker trait with no semantics in a GC'd language with no move semantics or borrow checker. |
| Stdlib compile model | **Implicit prelude, on-disk source** | See §4. Resolves plan decision 2. |

Everything else deferred: references, `impl Trait` returns, fixed-size arrays
(`[u8; 32]`), dotted module paths (`module std.core`), a disk search path, async.

## 3. Verified starting point

Probed with real programs against the compiler at `96551db`. This is what makes
the increment small — **these already work and run**:

- `impl<T> Option<T> { … }` — inherent impls on the prelude's own sum types
- `impl Display for Int` — trait impls on builtin primitives
- `fn map<U>(self, f: fn(T) -> U) -> Option<U>` — generic methods taking closures
  (unlocked by the Phase 2.0 generic-method work)
- `-> Self` as a trait method return type
- `trait Eq { fn ne(self, o: Self) -> Bool { !self.eq(o) } }` — default bodies

And this is what is missing (each verified, not assumed):

| Needed for | Missing | Observed today |
|---|---|---|
| `unwrap` | `panic` | `E0001: cannot find function 'panic'` |
| `Default`, constructors | associated functions | `P::new()` → `E0001: no variant 'new' on type 'P'` |
| `trait Ord: Eq` | supertrait enforcement | parses, then silently discarded by typeck |

## 4. Compile model (resolves plan decision 2)

`std/core` reaches a user program as an **implicit prelude module compiled from
on-disk Nova source**.

Today `nova-resolver` holds a `PRELUDE_SRC: &str` containing `Option`/`Result`,
lexes and parses it into a `$prelude` module, glob-imports its public names into
every user module, and lets user definitions shadow it. This design points that
same mechanism at real source at `std/core/lib.nova`, embedded with `include_str!`.

Chosen because:

- It is the plan's own recommendation ("compile-with-program initially; simplest,
  monomorphization already whole-program"), and it reuses machinery already
  shipped **and adversarially reviewed** for `Option`/`Result`.
- It is hermetic: `std/core` is baked into the compiler binary, so there is no
  "where is std installed", no version skew, and no new failure mode for the
  examples and test suite.
- Unused items cost nothing — only reachable functions are monomorphized.

**Rejected alternatives.** A *disk search path* (`NOVA_STD`, `import core`) is the
right eventual destination but needs nested-import-path work and introduces
deployment concerns now rather than later. A *precompiled std artifact* is
premature: generic functions cannot be precompiled (they instantiate per use
site), and it needs serialized HIR, ABI stability, and incremental infrastructure
(`salsa` is not yet a dependency).

**Migration is designed in.** The Nova source lives on disk as real `.nova` files,
never inline in a Rust string, and the driver passes `std/core`'s `FileId` through
one narrow seam (§7). Moving to a search path later changes only that seam — the
Nova source is untouched. This needs an ADR (`docs/adr/0004-stdlib-compile-model.md`).

## 5. Stage 1 — compiler prerequisites

Three independent features, each its own commit with tests, plus one small runtime
addition (`nova_rt_str_cmp`, see §6) alongside (a)'s shim.

### (a) `panic(msg: String)` builtin

A third variant alongside `Builtin::Println` / `Builtin::Print`, following that
established pattern. The runtime already exports
`nova_rt_panic(msg: *const u8, len: u64) -> !` and registers it in the JIT symbol
table; it needs a NovaStr-taking shim so the calling convention matches
`nova_rt_println`.

`panic` types as returning **`Ty::Never`**, which already exists and already joins
correctly in `if`/`match` arms (fixed in `9daea15`). So this type-checks with no
new inference work:

```nova
fn unwrap(self) -> T { match self { Ok(v) => v, Err(_) => panic("unwrap on Err") } }
```

`panic` aborts; it does not unwind and cannot be caught. Documented limitation.

### (b) Associated functions (`T::new()`)

Self-less methods are in a broken half-state today, and this fixes a real bug:

| Form | Today | After |
|---|---|---|
| `impl P { fn new() -> P {…} }`, unused | compiles, runs | unchanged |
| `P::new()` | `E0001: no variant 'new' on type 'P'` | resolves and calls |
| `p.make()` on a self-less method | **codegen ICE** — Cranelift `mismatched argument count: got 1, expected 0`, while `nova check` reports `ok` | clean diagnostic |

Work: path-call resolution learns to find a self-less inherent method on the named
type; instance-calling a self-less method becomes a diagnostic instead of an ICE.

**Extension, with a fallback.** `Default` is only meaningful if `T::default()`
works inside `fn f<T: Default>()`, not merely `Int::default()`. This is the same
bound-dispatch machinery `lower_trait_call` already uses for instance methods; the
only difference is that `Self` comes from the explicit `T::` qualifier rather than
a receiver. It is the one item in this design that could not be de-risked from
outside. **If it resists: ship `Default` for concrete types only and move generic
`T::default()` to 2.2 alongside the collections that consume it.**

### (c) Supertrait enforcement

`trait Ord: Eq` parses today and is then discarded, so writing it is a lie. Two
halves:

1. `impl Ord for T` requires `T: Eq` — else a conformance error.
2. A bound `T: Ord` implies `T: Eq` when bounds are checked at monomorphization.

## 6. Stage 2 — `std/core/lib.nova`

```nova
pub type Option<T>    = | Some(T) | None
pub type Result<T, E> = | Ok(T)   | Err(E)
pub type Ordering     = | Less | Equal | Greater

impl<T> Option<T> {
    is_some, is_none, map<U>, and_then<U>, unwrap, unwrap_or, ok_or<E>
}
impl<T, E> Result<T, E> {
    is_ok, is_err, map<U>, map_err<F>, and_then<U>, unwrap, unwrap_or
}

pub trait Display { fn fmt(self) -> String }
pub trait Debug   { fn dbg(self) -> String }
pub trait Eq      { fn eq(self, other: Self) -> Bool
                    fn ne(self, other: Self) -> Bool { !self.eq(other) } }
pub trait Ord: Eq { fn cmp(self, other: Self) -> Ordering }
pub trait Clone   { fn clone(self) -> Self }
pub trait Default { fn default() -> Self }
```

Plus impls of `Display`, `Debug`, `Eq`, `Clone`, and `Default` for `Int`, `Float`,
`Bool`, `Char`, and `String`.

`Ord` for the primitives needs care, because Nova's comparison operators are not
uniformly available (verified):

| Type | `<` today | `Ord` impl |
|---|---|---|
| `Int`, `Float`, `Char` | works | direct, via `<` / `==` |
| `Bool` | `E0013` — not defined | written with `if` alone; no operators needed |
| `String` | `E0013` — not defined | **needs a new `nova_rt_str_cmp` runtime function** (lexicographic); `String` has no length or indexing in Nova, so this cannot be written in Nova source |

`nova_rt_str_cmp` is a small addition (a few lines of Rust beside `nova_rt_str_eq`)
and is needed by any sorted collection later, so it lands here with the `panic`
shim. `Eq for String` needs nothing new — `==` on `String` already works via
`nova_rt_str_eq`.

`Display` makes the existing convention official: string interpolation already
bridges to a `fmt(self) -> String` method by name, so a type with
`impl Display for T` interpolates for free.

**Primitive interpolation is unaffected.** `check_interp` matches
`Ty::Int | Ty::Float | Ty::Bool | Ty::Char` natively (`ExprKind::ToStr`) *before*
reaching the `fmt` bridge, so `impl Display for Int` does not hijack the fast
path. A test asserts the two stringifications agree.

## 7. Data flow

```
std/core/lib.nova  (on disk)
  → include_str! in nova-resolver
  → registered in the driver's FileDb (real FileId)      ← the one seam
  → lexed / parsed as the implicit prelude module
  → glob-imported into every user module (user defs shadow)
  → typeck → MIR (monomorphization) → codegen            ← all unchanged
```

**Diagnostics inside `std/core` get real spans.** The prelude currently parses
under `FileId::DUMMY` guarded by a `debug_assert` — adequate for two type
declarations, inadequate for ~150 lines where a genuine error would point at a
dummy file. The driver owns the `FileDb`, so it registers the `std/core` source
and passes the `FileId` to the resolver. This is the same seam that later becomes
the disk search path (§4).

## 8. Error handling

- A user's own `Option`/`Result` still shadows `std/core`'s (existing behavior).
- Redefining a method `std/core` already defines on `Option` → `E0074`
  (overlapping inherent impls conflict only on methods both define, so *adding*
  distinct methods stays legal).
- `unwrap` on the wrong variant → `nova: panic: …` on stderr, process aborts,
  non-zero exit.
- An error inside `std/core` itself is a compiler bug and must surface with a real
  span (§7), not a dummy file.

## 9. Testing

- **Rust unit:** `std/core` parses and type-checks clean; unused `std/core`
  contributes no symbols (monomorphization reachability).
- **Nova e2e** (`tests/runtime/`): `Option`/`Result` method round-trips; a custom
  `Display`; primitive `Eq`/`Ord` (including `String` ordering through
  `nova_rt_str_cmp`, and `Bool` ordering); `unwrap` on `Err` asserting **both** the
  stderr message and the exit code. Each under `nova run` *and* `nova build`, plus
  `NOVA_GC_STRESS=1`, per established convention.
- **Regression:** the self-less-method ICE from §5(b) becomes a test asserting a
  clean diagnostic.
- **Gate:** §1, under both backends.
- **Adversarial-review workflow** after the increment, per the established loop.

## 10. Out of scope

`std/fmt`, `std/io`, `Iterator`, associated types, `Hash`/`Hasher`, `Copy`, `Vec`
and growable memory, references, `impl Trait` returns, fixed-size arrays, async,
dotted module paths, the disk search path, and precompiled std artifacts.
