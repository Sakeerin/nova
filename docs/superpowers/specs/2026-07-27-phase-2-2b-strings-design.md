# Phase 2.2b — `std/strings` design

> Date: 2026-07-27. Branch: `phase-2.2b-strings`, base `eb1b7f7`.
> Completes Phase 2.2 (`std/collections` + `std/strings`) per `docs/phase-2-plan.md` §4.

## 1. Why now, and why not `Iterator`

Phase 2.2 has two halves; `std/collections` shipped at `62d4438` and its debt paydown at
`eb1b7f7`. The other candidate for this increment was `Iterator` + associated types. It was
rejected for this increment after probing the compiler rather than reading the spec: the
spec's `Iterator` needs **four** separate compiler features that do not exist, one of them
invasive.

| `nova-spec/20-STDLIB.md` construct | State today (verified by probe) |
|---|---|
| `type Item` inside a trait | **does not parse** — `P0001: expected 'fn', found 'type'` |
| `Self.Item` / `Self::Item` as a type | no AST node, no `Ty` representation |
| `fn iter(self) -> impl Iterator<Item = &T>` | `impl Trait` does not parse; `&T` is `E0900` |
| `for x in coll` | `E0900` |
| `Map::iter()` yielding `(&K, &V)` | tuples are `E0900` |

Associated-type projection is the invasive one: it has to participate in Hindley-Milner
unification *and* in monomorphization. That is a language project, not a stdlib increment.

`std/strings` by contrast needs **no type-system work at all**, and it closes a defect that is
**already shipped**: `("a\"b").dbg()` produces `"a"b"`, which is not a valid Nova literal.
`std/core/lib.nova:168` flags this as needing settling before `Debug` is treated as stable, and
every later std module that formats a string inherits it.

## 2. Decisions

| # | Decision | Rationale |
|---|---|---|
| D1 | **Codepoint-level**, not bytes, not graphemes | `Char` is already a Unicode scalar value (a Rust `char`, lowered to `MirTy::I64`), so codepoints fit the language as it stands and need no Unicode data tables. Same tradeoff Rust and Python make. Graphemes would force a new Rust dependency *and* force every operation into Rust. |
| D2 | **Five primitive-shaped intrinsics; algorithms in Nova** | Smallest permanent ABI, and it honours the phase premise that std is written in Nova with FFI down to the runtime. The alternative — one intrinsic per operation — is ~12 permanent symbols and makes `std/strings` a wrapper rather than a module. |
| D3 | **Inherent `impl String`**, not free functions | Verified empirically: an inherent method **wins by priority** over a user trait method of the same name (prints `INHERENT`, no diagnostic), whereas two *traits* providing a name is `E0015 ambiguous`. So inherent methods reserve names by *shadowing*, which is strictly gentler than std/core's trait-based reservation. Free `pub fn`s would be glob-imported and reserve names like `split` and `join` in **every** module. |
| D4 | **`split` returns `[String]`**, not `Vec<String>` | Keeps `std/strings` depending on `std/core` alone. Nothing in the compiler enforces layering between std modules — they are all mutually visible — so the discipline has to be a convention, and a sibling dependency is harder to walk back than to adopt later. Costs a two-pass implementation (count, then fill), which is cheap over a `[Char]`. |
| D5 | **`Debug for String` fixed with no new ABI symbol** | See §5. `std/core/lib.nova:168` predicts a `nova_rt_str_escape` is required; it is not, once `str_chars` exists. |
| D6 | **Whole-string case mapping**, not `Char -> Char` | `ß -> SS` is 1-to-2. A `Char -> Char` signature cannot express it and would silently corrupt such input. |

## 3. Primitive layer

Five new intrinsics. All five join `Builtin::STD_ONLY` (which grows from `[Builtin; 3]` to
`[Builtin; 8]`), following the `str_cmp` / `str_hash` / `char_to_int` precedent: visible inside
std modules only, so none becomes a reserved word in user code.

| Nova-visible builtin | Runtime symbol | `MirTy` signature |
|---|---|---|
| `str_len_chars(String) -> Int` | `nova_rt_str_len_chars` | `(Ptr) -> I64` |
| `str_chars(String) -> [Char]` | `nova_rt_str_chars` | `(Ptr) -> Ptr` |
| `str_from_chars([Char]) -> String` | `nova_rt_str_from_chars` | `(Ptr) -> Ptr` |
| `str_to_upper(String) -> String` | `nova_rt_str_to_upper` | `(Ptr) -> Ptr` |
| `str_to_lower(String) -> String` | `nova_rt_str_to_lower` | `(Ptr) -> Ptr` |

`str_len_chars` exists separately from `str_chars` so a length query does not allocate.

### 3.1 Touchpoints per builtin (verified, not assumed)

1. `crates/nova-resolver/src/lib.rs` — `Builtin` variant, its `name()` arm, and membership in
   `STD_ONLY` (whose length annotation must be bumped).
2. `crates/nova-typeck/src/check.rs` — a `builtin_signature` arm giving `(Vec<Ty>, Ty)`.
3. `crates/nova-typeck/src/check.rs` — the diagnostic-`hint` match (currently
   `Builtin::StrCmp | StrHash | CharToInt => ""`). **Exhaustive**; will not compile until the
   new variants are added.
4. `crates/nova-mir/src/lower.rs` — the `hir::Callee::Builtin(b)` match. **Deliberately
   exhaustive** ("so a new builtin has to decide here whether it is a runtime call; `None` is
   not 'unhandled' but 'handled without one'"). All five map to `Some(RtFunc::…)`.
5. `crates/nova-mir/src/lib.rs` — `RtFunc` variant, `symbol()`, `signature()`.
6. `crates/nova-runtime/src/lib.rs` — the `extern "C"` function, plus registration in
   `symbols()` for the JIT.

Both codegen backends need **no changes**: `RtFunc::ALL` is the single source of truth for
declarations in both (Phase 2.1 follow-up `02ccee6`), and
`every_rt_func_is_declared_with_its_real_signature` fails if a variant is left unwired.

### 3.2 The sharp edge: `str_chars` builds a Nova array in the runtime

**No existing intrinsic constructs a Nova array.** `str_chars` is the first, so it must
reproduce codegen's layout exactly:

- one block, `{ len: i64, elem0, elem1, … }`, element *i* at byte offset `8 + 8*i`;
- total size `8 + 8*n`, allocated **scanned** (`gc::alloc(size, true)`), matching
  `nova_rt_alloc`, which has no scan parameter and always scans;
- `Char` elements are `i64` Unicode scalar values, so a scanned array of them can produce
  false retention under the conservative collector. That is the collector's existing
  behaviour for any `[Int]`, not a new hazard.

A layout mistake here is a **silent miscompile**, not a crash. It is therefore pinned by a
Nova-level test that reads `.len()` back and indexes elements — never by inspection of the
Rust code.

### 3.3 Invalid scalars

There is **no `Int -> Char` conversion in the language** (verified: `let c: Char = 65` is
`E0010`, `'a' + 1` is `E0010`, and no `IntToChar` builtin or `RtFunc` exists). So every `Char`
is a valid scalar by construction and `str_from_chars` cannot receive a bad one from Nova
source. It still validates defensively, following the convention already set by
`nova_rt_char_to_str` at `crates/nova-runtime/src/lib.rs:167`:
`char::from_u32(v).unwrap_or(char::REPLACEMENT_CHARACTER)` — substitute, do not panic.

## 4. Nova surface

A third embedded std module, `std/strings/lib.nova`; `STD_MODULES` grows from 2 entries to 3.
Every index and length below is in **codepoints**.

```nova
impl String {
    pub fn len(self) -> Int                        // codepoints, not bytes
    pub fn is_empty(self) -> Bool
    pub fn chars(self) -> [Char]
    pub fn char_at(self, i: Int) -> Option<Char>   // None when out of range
    pub fn slice(self, start: Int, end: Int) -> String   // panics on a bad range
    pub fn contains(self, needle: String) -> Bool
    pub fn starts_with(self, prefix: String) -> Bool
    pub fn ends_with(self, suffix: String) -> Bool
    pub fn index_of(self, needle: String) -> Option<Int>
    pub fn split(self, sep: String) -> [String]
    pub fn trim(self) -> String
    pub fn trim_start(self) -> String
    pub fn trim_end(self) -> String
    pub fn to_upper(self) -> String
    pub fn to_lower(self) -> String
    pub fn repeat(self, n: Int) -> String
    pub fn reverse(self) -> String
    pub fn join(self, parts: [String]) -> String   // separator is the receiver
}
```

**`join` hangs off the separator** (`",".join(parts)`, as in Python) rather than being a free
function, for the reason in D3: a `pub fn join` would be glob-imported and take the name
`join` in every module.

### 4.1 Error behaviour follows the collections precedent exactly

- `char_at` returns `Option<Char>` — the shape `Vec::get` already uses for an index query.
- `slice` **panics** on an invalid range — the shape `Vec::set` already uses for an index that
  must be valid.
- `index_of` returns `Option<Int>`, so absence is not encoded as `-1`.

Consistency with `Vec` is the whole argument; a caller should not have to remember which
collection reports absence which way.

### 4.2 Exact semantics of the cases a reader could interpret two ways

Pinned here because each has a defensible alternative, and picking the other one silently is
how a wrong implementation passes review:

| Case | Decision |
|---|---|
| `slice(start, end)` bounds | **`start` inclusive, `end` exclusive** — half-open, like every array convention in the language. `slice(0, self.len())` is the whole string. |
| `slice` invalid range | Panics when `start < 0`, `end > len`, or `start > end`. `start == end` is **valid** and yields `""`. |
| `index_of` | Index of the **first** occurrence, in codepoints. |
| `index_of` / `contains` with `""` | `contains("") == true`, `index_of("") == Some(0)` — the empty string occurs at every position, and position 0 is the first. |
| `starts_with("")` / `ends_with("")` | Both `true`, for the same reason. |
| `split` with `sep == ""` | **Splits into single-codepoint strings**: `"abc".split("") == ["a", "b", "c"]`, and `"".split("") == []`. There is no consensus to inherit here — JavaScript gives exactly this, Rust adds boundary empties (`["", "a", "b", "c", ""]`), Python raises. The JavaScript behaviour is chosen as the most useful and least surprising, and because it makes `"".join(s.split("")) == s` hold. |
| `split` with no occurrence | Returns a one-element array containing the whole string, **never** an empty array. |
| `split` adjacent/leading/trailing separators | Produces empty strings at those positions: `",a,".split(",") == ["", "a", ""]`. No collapsing, no trimming. |
| `repeat(n)` for `n < 0` | Panics. `repeat(0) == ""`. |
| `char_at` with a negative index | `None`, not a panic — `Option` already expresses out-of-range, and `Vec::get` treats a negative index the same way. |
| `reverse` | Reverses **codepoints**. A combining accent therefore detaches from its base character; that is inherent to codepoint-level (D1) and is documented on the method. |

### 4.3 Whitespace

The `trim` family trims by Unicode whitespace, decided per `Char`. `Char` cannot currently be
asked whether it is whitespace, so the predicate is an explicit comparison list in Nova
(space, `\t`, `\n`, `\r`, and the common Unicode spaces). This is a deliberate, documented
approximation — an exact `char::is_whitespace` would need a sixth intrinsic, which is not
worth a permanent ABI symbol for this increment. The list lives in one private helper so
there is exactly one place to correct.

### 4.4 Coherence check before writing the impl

`std/core` already provides `impl Display for String`, `impl Debug for String`,
`impl Eq for String`, `impl Ord for String` and `impl Hash for String` — all **trait** impls,
which cannot overlap an inherent one. Verified: all 14 `String` impls in `std/` are trait
impls, so **this increment introduces the first inherent `impl String` in the language** —
`impl String { fn shout }` was confirmed to work in a user program, but never before from an
std module.

Two consequences to watch rather than assume:

- An inherent impl on a **primitive** from an std module is a new configuration. If a second
  inherent `impl String` block ever appears in a different std module, whether that is
  rejected (`E0074` is specified for *trait* impls) or silently accepted with one shadowing
  the other is unknown. Keep all of `std/strings`' methods in **one** block so the question
  does not arise here, and raise it as a compiler finding if it surfaces.
- Inherent methods win over trait methods by priority (D3). Since `std/core` already provides
  `Display`, `Debug`, `Eq`, `Ord`, `Clone`, `Default` and `Hash` for `String`, none of the 18
  method names in §4 may collide with `fmt`, `dbg`, `eq`, `ne`, `cmp`, `clone`, `default` or
  `hash` — otherwise `std/strings` would silently shadow a core trait method on `String`
  only. None of the 18 do; that is a constraint on future additions, not a current problem.

## 5. `Debug for String`, fixed without new ABI

`Debug for Char` in `std/core/lib.nova:157` already contains the escaping logic (`\\`, `'`,
`\n`, `\t`, `\r`, `\0`). Once `str_chars` exists, `Debug for String` is that same logic mapped
over `str_chars(self)`, with `"` escaped instead of `'`:

- it lives in **std/core**, using only the builtin — no dependency on `std/strings`, so no
  layering inversion;
- it needs **no** `nova_rt_str_escape`, contradicting the prediction in
  `std/core/lib.nova:168-177`, whose comment must be corrected as part of this work rather
  than left describing a plan that was not followed.

The per-character escape logic is shared between `Debug for Char` and `Debug for String`
rather than written twice — the `Map::get`/`remove` duplication from Phase 2.2a is the
cautionary precedent.

## 6. Accepted costs

- **Every inspection allocates.** `starts_with`, `contains`, `index_of`, `split` and the
  `trim` family each decompose the whole string to a `[Char]` first, so a 1 MB haystack
  allocates roughly 8 MB. Accepted because the **Nova-level API is identical** whether or not
  a `str_find` fast path exists later: adding `nova_rt_str_find` is a pure optimization behind
  an unchanged signature. Nothing has to be redesigned to make it fast.
- **`String` gains 18 reserved method names**, by shadowing (D3). A user trait declaring
  `trim` and implementing it for `String` compiles, and `s.trim()` silently resolves to the
  std method. This is gentler than the `E0015` that std/core's trait methods cause, but it is
  still a permanent commitment and belongs in the CHANGELOG.
- **`trim` uses an approximate whitespace set** (§4.2).

## 7. Gate

`tests/runtime/strings.nova` + `.stdout`, run under **`nova run`, `nova build` and
`NOVA_GC_STRESS=1`**, matching the collections gate. It must cover, at minimum:

1. **Byte length ≠ codepoint length** — `"café".len() == 4` while its UTF-8 is 5 bytes; and a
   multi-byte-per-char case such as `"日本語".len() == 3`.
2. **`ß -> SS`** through `to_upper`, proving case mapping is whole-string and not `Char -> Char`.
3. **`("a\"b").dbg()`** producing a valid Nova literal — the defect that motivated this phase.
4. **Round-trip** `str_from_chars(str_chars(s)) == s` for ASCII, accented, CJK and emoji input.
5. **`str_chars`'s array layout** — read `.len()` back and index elements from Nova (§3.2).
6. Empty-string and single-char edge cases for every method that scans: `"".split(",")`,
   `"".trim()`, `"".reverse()`, `slice(0, 0)`, `repeat(0)`, `",".join([])`.
7. `split` with a separator that is absent, leading, trailing, repeated, equal to the whole
   string, and **empty** — every row of the §4.2 table needs a line in the fixture, since that
   table is the part a reader could most plausibly implement backwards.
8. `slice`'s half-open boundary at both ends (`slice(0, 0)`, `slice(0, len)`,
   `slice(len, len)`) and each of its three panic conditions.

`NOVA_GC_STRESS=1` carries real weight here, not belt-and-braces: `str_chars` and
`str_from_chars` introduce two new allocation shapes reachable from a builtin — a scanned
scalar array, and a leaf byte buffer plus a scanned header — and the collector must see the
intermediate `[Char]` as live across the allocations that follow it.

## 8. Out of scope

Deferred deliberately, not overlooked:

- `replace`, `pad_start` / `pad_end`, `split_once`.
- `String -> Int` / `String -> Float` parsing. There is currently no way to parse a number at
  all, and `std/json` will need it, but it raises its own questions (radix, overflow, leading
  `+`, surrounding whitespace) that would widen this increment.
- Grapheme-cluster segmentation (D1).
- `nova_rt_str_find` and other fast paths (§6).
- An exact `char::is_whitespace` intrinsic (§4.2).
- `Iterator`, associated types, tuples, `for x in coll` (§1).

## 9. Risks

1. **`str_chars`'s array layout** is the highest-consequence risk in the increment: wrong
   offsets or a wrong scan flag give a silent miscompile or a collector bug, not an error.
   Mitigation: §3.2's Nova-level test, plus `NOVA_GC_STRESS=1`.
2. **Codepoint indices are O(n)**, so a loop doing `char_at(i)` for each `i` is quadratic. The
   module doc must direct callers to `chars()` once and index the array. Worth stating in the
   module header the way the `hash & (cap - 1)` rule is stated beside `Hash`.
3. **Three exhaustive `match` sites** (§3.1) must each be updated; the compiler enforces this,
   which is why they were written exhaustively.
4. **Doc drift**, the failure mode this branch's predecessor actually hit: `std/core`'s
   comment at :168 predicts a solution this design does not use, and the typeck hint comment
   says the std-only builtins "are called from one hand-written std/core site each", which
   stops being true. Both must be corrected in the same commits that falsify them.

## 10. Definition of done

- Five intrinsics wired through all six touchpoints, both backends unchanged.
- `std/strings/lib.nova` embedded as the third std module, surface per §4.
- `Debug for String` produces valid Nova literals; std/core's stale comment corrected.
- The §7 gate passes byte-identically under all three run modes.
- `cargo test --workspace --no-fail-fast`, `cargo clippy --all-targets --all-features -D warnings`
  and `cargo fmt --check` all green.
- CHANGELOG records the new surface and the 18 shadowed method names.
