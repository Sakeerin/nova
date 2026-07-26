# Phase 2.2a Design — Field Assignment, Repeat Arrays, and `std/collections`

> Status: **approved** (2026-07-26). Scopes the first slice of `docs/phase-2-plan.md`
> §2.2 against the compiler's verified capabilities. Builds on Phase 2.1
> (`std/core`, merged at `d6c63b4`).

## 1. Goal

Ship `Vec`, `Hash`, `Map`, and `Set` as `std/collections` — written in Nova —
after adding the two language features they require.

**Gate:** a Nova program builds and mutates a `Vec` and a `Map` (including a
rehash and tombstone reuse) with correct output under `nova run`, `nova build`,
**and `NOVA_GC_STRESS=1`**.

## 2. Verified starting point

Probed against the compiler at `d6c63b4`. Design decisions below rest on these
facts, not assumptions.

**The plan's central premise is wrong, in our favour.** §2.2 says growable memory
means "the GC must track moved/resized blocks." It does not. Nova already has two
heap shapes:

| Shape | Structure | Data can grow? |
|---|---|---|
| Array | *one* block, `len` inline, elements at `+8` | No — growth moves the object others point at |
| `NovaStr` | header `{len, ptr}` (scanned) **+** a separate buffer | **Yes** |

A record holding an array field has the second shape: the record is one object,
the array another, and the array's only referent is the record's field slot. So
growth is "allocate a bigger array, copy, reassign the field" — the record's
address never changes and nothing any pointer points at ever moves. **The
conservative, non-moving collector needs no changes.**

**Why the collection windows inside growth are safe** — and it is *not* "the old
buffer stays reachable through the record". That holds for `Vec::push`, which
builds the new array completely before reassigning the field, but it is **false
for `Map::grow`**: `grow` reassigns `self.keys` *first*, and from that moment the
old key array is reachable only from a stack local while two more array
allocations and a per-iteration reinsert allocation all run. Any of those can
collect.

The real invariant is a property of the **calling convention**, not of the
record-plus-buffer shape:

> Every collection point is a call into `gc::alloc`. The collector scans the
> stack from the current frame up to the thread base, **plus the callee-saved
> registers**, which the `setjmp` shim in `gc_stack.c` spills onto that stack.
> The C ABI leaves a value that is live across a call in exactly one of those two
> places — a caller-saved register cannot hold a live root at a call boundary,
> because the callee is free to clobber it. So **any Nova value still live in the
> allocating frame or any of its callers is a root**, whether or not anything
> else refers to it.

Two corollaries worth stating because they are what make the invariant usable:
marking is **range-based**, so a derived or interior pointer (an
array-element address the backend computed and kept instead of the base) marks
its containing object just as well as the base pointer does; and `gc::alloc`
runs `maybe_collect` *before* it allocates, so the new block never needs to
survive a collection it does not yet exist for.

The consequence for a future collection author: the condition to check is "is
the only reference on some frame's stack (or in a callee-saved register) for the
whole window?", not "is it still reachable from a record". The shape-based
condition can be satisfied by code that is **not** safe — hand a buffer's only
reference to a `scan = false` leaf object (string byte buffers are flagged that
way and are never traced), or to a structure on the Rust side of the FFI
boundary, and the written condition is met while the buffer is collected out from
under you. That is why `NOVA_GC_STRESS=1` (collect on *every* allocation) is a
**gate criterion** for this phase rather than belt-and-braces: it is the only
thing that actually exercises the multi-allocation window inside `Map::grow`,
and the failure mode there is silent data loss, not a crash.

**Blockers found:**

| Needed for | Missing | Observed |
|---|---|---|
| `Vec::push` updating `len`/`data` | field assignment | `rec.f = v` → `E0900`, "assignment to anything but a local variable or array element" |
| `Vec` growth | runtime-length array | only literals; `[x; n]` → `P0001`; no `with_len` anywhere |
| `Map`, `Set` | `Hash` | absent from `std/core` (deferred from 2.1) |
| `iter()`, `for x in coll` | `Iterator` + associated types | `for` over a non-range → `E0900` |
| `Map::iter` returning pairs | tuples | `(Int, Int)` → `E0900` |

**Capabilities confirmed present:**

- Empty array literals work, including in a generic record field
  (`Box2<Int> { data: [] }` → `len 0`), so `Vec::new()` needs no new primitive.
- All bitwise operators (`^`, `<<`, `>>`, `&`, `|`) and `%` work on `Int`, so
  hash mixing and power-of-2 bucket masking are writable **in Nova**.
- `mut` on a *parameter* parses. `mut` on a record *field declaration* is a
  parse error — mutability is a property of the binding, not the field.
- `place_root` already walks field/index projection chains to the root binding,
  returning `Mutable` / `ImmutableLocal` / `NotAPlace`; it was built for
  `rec.data[0] = v`.

## 3. Scope

**In:** field assignment, repeat-array literals, the mutable-receiver rule,
`Vec`, `Hash`, `Map`, `Set`.

**Deferred, with reasons:**

| Item | Why |
|---|---|
| `iter()` on any collection; `for x in coll` | Needs `Iterator` + associated types. `Map`/`Set` iteration *additionally* needs tuples, which are `E0900`. Iteration is index-based on `Vec` for now. |
| `Queue`, `Deque` | YAGNI here; `Vec` covers the common need, and a ring buffer is cheap to add later. |
| `Vec::with_capacity` | Needs a `T` to fill with; Nova cannot express reserved-but-uninitialized capacity. Pure optimization. |
| `Hash for Float` | NaN never equals itself, so a NaN key is unreachable; `0.0`/`-0.0` hash differently unless normalized. A footgun in permanent public API. |
| `std/strings` | Its own slice; shares little with this work. |

## 4. Stage 1 — language prerequisites

### (a) Field assignment — `rec.field = v`

Records are currently **immutable after construction**, which blocks essentially
every future std module, not just collections. This is the substantive language
change in this phase.

The mutability analysis already exists (`place_root`), so the work is the store:
a `SetField` node threaded HIR → MIR → both backends, writing at
`base + 8*field_index` (records already lay fields out at `8*i`). Rule: `E0060`
unless the assignment is rooted at a `mut` local — identical to the array rule.

**Semantics to document, not discover:** records are heap objects, so field
assignment is **alias-visible**. After `let mut b = a`, `b.f = 1` changes what `a`
sees. This is ordinary reference semantics (Java, Python); Nova has no ownership
or borrow checking to prevent it.

### (b) Repeat-array literal — `[init; n]`

`n` is any runtime `Int` expression. Parser, typeck (element type from `init`,
length from `n`), a `MakeArrayRepeat` MIR statement, both backends.

Chosen over a "give me `n` zeroed slots" primitive because it never creates
uninitialized or null-filled memory: the filler is a real `T` the caller supplied.
This is what lets `Vec` grow with no `Default` bound and no unsafety obligation.

### (c) The mutable-receiver rule

With `mut` illegal on record fields, a mutating method is written
`fn push(mut self, x: T)`. Nothing otherwise forces the **caller's** receiver to
be mutable, so `v.push(x)` would mutate `v` even after `let v = …` — inconsistent
with `arr[0] = v` being rejected on an immutable binding.

**Decision:** a method declaring `mut self` requires a `Mutable` receiver place at
the call site, else `E0060`. Cheap (reuse `place_root` on the receiver), keeps
`mut` meaningful, and makes `let mut v = Vec::new()` read honestly. The cost is a
real rule the whole std API must respect.

## 5. Stage 2 — `std/collections`

### `Vec<T>`

```nova
pub record Vec<T> { len: Int, data: [T] }
```

No separate `cap` field — `data.len()` *is* the capacity.

`new`, `len`, `push(mut self, x: T)`, `pop(mut self) -> Option<T>`,
`get(self, i: Int) -> Option<T>`, `set(mut self, i: Int, v: T)`, `clear(mut self)`.

Growth doubles (from 4 when empty) and uses **the pushed element as the filler**:
allocate `[x; newcap]`, copy `0..len` back, and slot `len` is then already `x`, so
the push completes for free.

`get` returns `Option<T>` **by value**, not the spec's `Option<&T>` — Nova has no
references. For heap types the value *is* the pointer, so it still behaves
referentially. `set` on an out-of-range index panics with an explicit message
rather than relying on the array bounds check, so the diagnostic names the method.

**Known retention:** `pop` leaves the vacated slot holding its old reference until
overwritten, so a popped element stays reachable until then. Clearing it would
require a filler value; accepted and documented.

### `Hash`

```nova
pub trait Hash { fn hash(self) -> Int }
```

One-shot, not the spec's streaming `fn hash<H: Hasher>(self, h: H)`: a streaming
hasher must accumulate into a field, which is awkward through a parameter even
with field assignment, and adds a whole `Hasher` protocol for no benefit to a hash
map. **Spec deviation — recorded in ADR 0005 (§5.1).**

`Int`, `Bool`, `Char` use the **splitmix64 finalizer** — three rounds of
xor-shift-multiply with its published constants — written in Nova with `^`, `>>`,
and `*`. Identity hashing would cluster badly against power-of-2 masks, so a
concrete, known-good mixer is specified rather than left to invention. `String` requires
`nova_rt_str_hash` in the runtime, since Nova cannot walk a string's bytes;
it is reached through an **`std`-scoped builtin**, following `str_cmp` — *not* a
global reserved word, so user code may still define the name.

### `Map<K: Hash + Eq, V>` and `Set<T: Hash + Eq>`

```nova
pub record Map<K, V> { len: Int, keys: [K], vals: [V], state: [Int] }
pub record Set<T>    { map: Map<T, Bool> }
```

Open addressing with linear probing. `state` is `0` empty / `1` occupied / `2`
tombstone. Capacity is a power of two so the index is `hash & (cap - 1)`, and the
initial capacity on first insert is 8.

The arrays are allocated lazily on first insert: `keys` filled with the inserted
key, `vals` with the inserted value (the same trick as `Vec` — the filler is
always a real value, never uninitialized), and `state` with `0`, which is exactly
the "empty" tag, so a freshly allocated table is correctly empty by construction.

`Map`: `new`, `insert(mut self, k, v) -> Option<V>`, `get(self, k) -> Option<V>`,
`remove(mut self, k) -> Option<V>`, `contains_key(self, k) -> Bool`, `len`.

**Load factor, stated exactly** (integer arithmetic only — no `Float`): grow when
`(occupied + tombstones + 1) * 4 > cap * 3`, i.e. above 3/4 full. Tombstones count
toward the threshold, so a remove-heavy workload cannot degrade into an
all-tombstone scan. Growth doubles `cap` and reinserts only `occupied` entries,
which is also what clears tombstones.

`len` counts `occupied` only, so it is unaffected by tombstones.

`Set` wraps `Map` rather than duplicating the probing logic — a `Bool` per entry
is a good trade for not maintaining two copies of the trickiest code here.

## 5.1 ADR 0005 — the two decisions worth recording

One ADR covers both, since both outlive this increment:

1. **The mutable-receiver rule** (§4c) is a *language* rule that constrains every
   future std API, so the reasoning must be findable later — including the
   alternative (Java/Python-style reference mutation through any binding) and why
   it was rejected as making `mut` inconsistent with the existing array rule.
2. **One-shot `Hash`** instead of the spec's streaming `Hasher` protocol (§5),
   with the migration note: moving to a streaming hasher later would change
   `Hash`'s only method and therefore every impl, so this is a deliberate
   commitment rather than a stopgap.

Also record, as consequences rather than decisions: field assignment is
alias-visible (§4a), and `Vec::with_capacity` is absent (§3).

## 6. Data flow

```
std/collections/lib.nova  (Nova source)
  → embedded and compiled as part of the implicit std module (ADR 0004 seam)
  → Vec/Map/Set are ordinary generic records; methods are ordinary Nova code
  → growth: `[x; n]` allocates a new array; the field assignment swaps it in
  → the record's address never changes; only the field's target does
  → the existing conservative collector traces record → array unchanged
```

## 7. Error handling

- `get`, `pop`, `remove` return `Option`; absence is never a panic.
- `set` out of range panics with a message naming the method.
- Allocation failure remains the runtime's existing `handle_alloc_error`.
- Field assignment not rooted at a `mut` local → `E0060`.
- Calling a `mut self` method on an immutable receiver → `E0060`.
- A `Map` key type lacking `Hash` or `Eq` → the existing bound machinery
  (`E0013` at monomorphization).

## 8. Testing

- **Rust unit:** field assignment and `[init; n]` in typeck (including the
  `E0060` cases and the mutable-receiver rule); `MakeArrayRepeat` lowering;
  `nova_rt_str_hash` determinism and distribution sanity.
- **Nova e2e** (`tests/runtime/`), each under `nova run`, `nova build` + execute,
  **and `NOVA_GC_STRESS=1`** — the last is the critical one, because growth
  allocates heavily and the buffer swap is exactly where a conservative
  non-moving collector could go wrong:
  - `Vec` across several doublings, with `pop`, `set`, `clear`, and `get`
    in/out of range.
  - `Map` with **forced collisions** (keys chosen to share a bucket), tombstone
    reuse after `remove`, and at least one rehash; `Set` dedup.
  - A `Map<String, Int>` to exercise the runtime hash path.
- **Alias semantics:** a test pinning that field assignment through one binding
  is visible through another, so the documented reference semantics is
  deliberate rather than incidental.
- Adversarial-review workflow after the increment, per the established loop.

## 9. Out of scope

`Iterator`, associated types, tuples, `iter()` on any collection, `for x in coll`,
`Queue`, `Deque`, `Vec::with_capacity`, `Hash for Float`, `std/strings`,
references, and any GC change.
