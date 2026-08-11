# A byte type, and byte-based file I/O — Design

**Status:** approved 2026-08-12. Increment 2 of 4 in the decomposition of `std/fmt` + `std/io`.

**Base:** `main` at `50bc1ea` (`std/fs` on Strings merged and pushed; 864 tests, 8 deliberately
ignored).

---

## 1. Why this, and why now

Increment 1 shipped `std/fs` over `String` because **Nova has no byte type**. Measured, not inferred:
`fn f(x: u8)` is `E0001 cannot find type 'u8'`. That is what blocks `fs::read`, `fs::write`, and
everything in increment 3 — the `Read`/`Write` traits and `stdin`/`stdout`/`stderr`.

**One roadmap question is settled here rather than deferred: Nova's byte I/O is buffer-returning.**
`read` hands back a fresh buffer; `write` takes one. The spec's `read(self, buf: &mut [u8])` is
buffer-*filling*, which needs references — `&Int` is `E0900 reference and pointer types are not
supported yet`, measured. In a garbage-collected language a returned buffer is the idiomatic shape and
costs an allocation the language is already paying everywhere else. **So references leave the roadmap
permanently**, and `nova-spec/20-STDLIB.md` §4 is amended rather than built against.

## 2. What is established, and how

**The right-hand column is how each was established.** This project has repeatedly shipped a claim
measured on one shape and stated for all of them, and a claim recalled from memory arrives without the
hedging its original measurement had.

| Claim | How established |
|---|---|
| `u8` is not a type: `fn f(x: u8)` is `E0001 cannot find type 'u8'` | **measured** — `nova check` |
| `&Int` is `E0900 reference and pointer types are not supported yet` | **measured** |
| `[Int]` arrays work end to end | **measured** |
| `mir_ty` maps `Int|Char → I64`, `Float → F64`, `Bool → I8`, and `String|Fn|Sum|Record|Array|Future → Ptr` (`nova-mir/src/lib.rs:608-619`) | read |
| `NovaStr` is `{len: u64, ptr: *const u8}`; its byte buffer is a **GC leaf** (`gc::alloc(len.max(1), false)`) and its header is **scanned** (`nova-runtime/src/lib.rs:43-48`) | read |
| `NovaStr` is the **only** struct declared in `nova-runtime/src/lib.rs`, private ones included | **measured** — `grep -cE '^\s*(pub )?struct '` returns 1 |
| Arrays are `8 + 8*len` bytes, scanned, guarded by `MAX_ARRAY_LEN` | read |
| `std/strings` builds **18 methods on 5 intrinsics**; `char_at` and `index_of` are Nova-level over `str_chars` | read |
| `String::char_at(i) -> Option<Char>` and `index_of(needle) -> Option<Int>` — **`Option`, not abort**; negative index yields `None` | read |
| `RESERVED_TYPE_NAMES` currently holds six names | read |

**Recorded in memory but NOT re-derived here — re-derive before relying on it:** that `Float` is
strictly stronger than `Bool` at the monomorphization seam because it crosses register banks, and that
`mono.rs` early-returns on types without projections.

## 3. The rejected alternative, and the measured reason

**`u8` as a scalar primitive plus `[u8]` over the existing array machinery** is the obvious design. It
reuses indexing syntax, `ArrayGet`/`ArraySet` and the bounds checks. It is rejected for three reasons,
the first of which is the decisive one:

1. **`u8` would map to `MirTy::I8` — the same as `Bool`.** Only `Bool` (`I8`) and `Float` (`F64`) are
   disjoint at `mir_ty` today, and that is what gives monomorphization tests any discriminating power
   at all. A second `I8` type means a mono bug confusing `Bool` with `u8` is invisible, and a test
   instantiated at those two types tests nothing. `Bytes → Ptr` joins five types that already collide
   and introduces **no new** collision class.
2. **8× memory** — one 8-byte slot per byte.
3. **Conversion at each FFI boundary**, since any `nova_rt_*` function taking or returning a byte
   buffer wants it packed — `NovaStr`'s already is.

`[Int]` with a documented 0..255 convention is worse: unsound, and it has problems 2 and 3 as well.

## 4. Representation

`Bytes` is a new `hir::Ty` variant mapping to `MirTy::Ptr`, represented exactly as `NovaStr` is: a
**scanned header** `{len, ptr}` pointing at a **GC leaf buffer**. That shape is not a guess — it is
what every Nova `String` already is.

Because `Bytes` is an opaque pointer whose every operation is an intrinsic, **neither codegen backend
changes.** Cranelift and the textual LLVM emitter see a pointer, exactly as they do for `String`.

`Bytes` and `String` are therefore **structurally identical and semantically distinct**: same layout,
but `String` carries a UTF-8 guarantee and `Bytes` does not. Nothing in the type system converts
between them implicitly.

## 5. Surface

### 5.1 `std/bytes` — a seventh `STD_MODULES` entry

Following the precedent that `String` is a `Ty` variant whose methods live in `std/strings`:

```nova
impl Bytes {
    pub fn len(self) -> Int
    pub fn byte_at(self, i: Int) -> Option<Int>          // Option, per String::char_at
    pub fn slice(self, start: Int, end: Int) -> Bytes
    pub fn concat(self, other: Bytes) -> Bytes
    pub fn index_of(self, needle: Bytes) -> Option<Int>
    pub fn contains(self, needle: Bytes) -> Bool         // index_of().is_some(), per String
    pub fn to_ints(self) -> [Int]                        // mirrors str_chars
    pub fn to_string(self) -> Option<String>             // None on invalid UTF-8
}

pub fn bytes_from_ints(ints: [Int]) -> Bytes
pub fn bytes_from_string(s: String) -> Bytes

impl Eq for Bytes                                       // eq / ne
```

**`to_string` returns `Option`, not `Result<String, IoError>`.** There is exactly one failure mode —
the bytes are not UTF-8 — so a kind adds nothing, and it avoids coupling a byte type to an I/O error
type.

**`byte_at` yields `Int`**, since Nova has no narrower integer. The value is always in `0..=255`.

### 5.2 Ten byte intrinsics

`bytes_len`, `bytes_at`, `bytes_slice`, `bytes_concat`, `bytes_to_ints`, `bytes_from_ints`,
`bytes_from_string`, `bytes_is_utf8`, `bytes_to_string_unchecked`, `bytes_eq`.

`index_of`, `contains`, and every bounds check are **Nova-level**, exactly as `std/strings` builds 18
methods on 5 intrinsics. `bytes_at` is an intrinsic rather than `to_ints()[i]` because the latter is
O(n) per access.

**`bytes_is_utf8` and `bytes_to_string_unchecked` are a deliberate pair**: together they let
`to_string` be a Nova-level `if`, avoiding a status-code protocol for what is one boolean.

**`bytes_from_ints` aborts on a value outside `0..=255`.** A `Result` there would infect every
construction path for a caller error.

### 5.3 `fs::read` and `fs::write`

```nova
pub async fn read(path: String) -> Result<Bytes, IoError>
pub async fn write(path: String, content: Bytes) -> Result<(), IoError>
```

Three more intrinsics — `fs_read`, `fs_take_bytes`, `fs_write` — over increment 1's existing boundary,
where **the status code is the error kind** and payloads travel in a GC-rooted thread-local slot.

**No new slot.** `String` and `Bytes` share `{len, ptr}`, so the existing payload slot serves both; it
is renamed to reflect that it holds a buffer rather than specifically a string. The slot protocol is
already recorded as the thing increment 3 should replace, and adding a third slot would deepen that
debt for no benefit.

**`STD_ONLY` therefore goes 30 → 43** (ten byte intrinsics plus three fs). `STD_MODULES` goes 6 → 7.

## 6. Consequences worth stating up front

- **`Bytes` joins `RESERVED_TYPE_NAMES` (six → seven), and that is a breaking change.** A program
  declaring `record Bytes` or `type Bytes = …` now gets `E0089`. The reserved-names branch established
  that this class needs a `### Changed` CHANGELOG entry as well as `### Added`, and that any "nothing
  that works breaks" claim about it would be **false** — construction and pattern matching worked
  before.
- **References leave the roadmap.** Nothing in increments 2–4 needs them once I/O is
  buffer-returning; `nova-spec/20-STDLIB.md` §4 is amended.
- **ADR 0011's recorded §5 deviation shrinks.** It says `read`, `write`, `open` and `File` are deferred
  for want of a byte type; after this increment only `open` and `File` remain. Amend it in place with a
  dated note.
- **`fs_invalid_data` stops needing its Rust-side workaround.** That fixture currently has the harness
  write raw non-UTF-8 bytes because Nova could not; with `fs::write` and
  `bytes_from_ints([0xFF, 0xFE])` it becomes self-contained Nova. Convert it.

## 7. Testing

- **Both allocations get the two-guard treatment** this project has established twice: assert
  `gc::object_info`'s tracked `(size, scan)` for the header (**scanned**) *and* the buffer (**leaf**),
  not merely the bytes read back. A scanned buffer is silent over-retention; an unscanned header frees
  the buffer under the caller.
- **Round-trip properties rather than per-case checks**: `from_string(s).to_string() == Some(s)`;
  `from_ints(b.to_ints()) == b`; `b.slice(0, b.len()) == b`; `a.concat(b).len() == a.len() + b.len()`.
- **Invalid UTF-8 is now constructible in Nova** — `bytes_from_ints([0xFF, 0xFE]).to_string()` is
  `None`. This is also what lets `fs_invalid_data` be rewritten.
- **A mono-seam test must instantiate at `Float`, never at `Bool`** — `Bytes → Ptr` is indistinguishable
  at `mir_ty` from the six types already mapped there (`String`, `Fn`, `Sum`, `Record`, `Array`,
  `Future`), and memory records `Float` as strictly stronger than `Bool` because it crosses register
  banks. That last point is **recorded, not re-derived here.**
- **`fs::read`/`fs::write` get a byte round-trip** through a path under `temp_dir()`, unique per
  process, asserting the bytes survive unchanged — including a byte sequence that is **not** valid
  UTF-8, so the path cannot silently be going through `String`.
- Suite stays green at 864 plus the new tests, with the 8 ADR-0010 tests still ignored and untouched.

Mutation targets, named here rather than left to review:

| Mutation | Must be killed by |
|---|---|
| Header allocated leaf instead of scanned | the `gc::object_info` layout test |
| Buffer allocated scanned instead of leaf | the same test's second assertion |
| `bytes_is_utf8` returns `true` unconditionally | `bytes_from_ints([0xFF, 0xFE]).to_string()` is `None` |
| Off-by-one in `bytes_slice`'s end bound | a slice whose result length is asserted |
| `bytes_at` drops its bounds check | `byte_at(len)` yields `None` |
| `bytes_eq` compares lengths only | two equal-length buffers differing in one byte |
| `fs::read` routes through `String` | the non-UTF-8 round-trip fixture |
| `bytes_from_ints` accepts 256 | a construction test expecting an abort |

## 8. Non-goals, each deliberate

- **No `b"…"` literal syntax.** `bytes_from_ints` and `bytes_from_string` cover construction; a literal
  is lexer work this increment otherwise does not need.
- **No mutable byte buffer**, and no `&mut` anywhere. Buffer-returning I/O is what makes that possible.
- **No `Read`/`Write` traits, no `stdin`/`stdout`/`stderr`, no `open`/`File`** — increment 3, which also
  needs concrete `Stdin`/`Stdout`/`Stderr` types because `impl Trait` in return position **does not
  parse** (`P0001`, measured).
- **No poller, no `Wait::Io`, no change to the drive loop** — increment 4. In particular the drive
  loop's `None => report_deadlock()` default arm is left alone; it becomes wrong only when a `Wait`
  variant can carry no deadline.
- **The payload-slot protocol is not redesigned here.** Moving payloads into the per-task state object
  is recorded as increment 3's decision, before a real poller inserts the `.await` the current protocol
  forbids.

## 9. Definition of done

- `Bytes` works end to end under both backends, `nova run` and `nova build`, and under
  `NOVA_GC_STRESS=1`.
- Header scanned and buffer leaf, both pinned by `gc::object_info`.
- All eight methods plus both constructors and `Eq` behave as specified, with `Option` — not an abort —
  for out-of-range `byte_at` and a missing `index_of`.
- `fs::read` and `fs::write` round-trip bytes that are not valid UTF-8.
- `fs_invalid_data` is self-contained Nova, with no harness-side byte writing.
- `RESERVED_TYPE_NAMES` holds seven names; `E0089` fires for `record Bytes` and `type Bytes = …`.
- ADR 0011 amended for the narrowed §5 deviation; `nova-spec/20-STDLIB.md` §4 amended for
  buffer-returning I/O; CHANGELOG entries under **both** `### Added` and `### Changed`.
- Suite green, clippy `-D warnings` and `cargo fmt --all --check` clean.
- **Before committing, run the claim sweep** over everything written: grep the added lines for
  `always`, `every`, `only`, `any`, `never`, `all`, `cannot` and, per hit, delete the quantifier or
  state the measurement behind it. Two things it structurally cannot catch, so check them by reading: a
  doc quoting a **literal diagnostic string**, and **a sentence this change falsified but did not
  touch** — `std/fs`'s and `std/io`'s module docs both describe a `String`-only world.
