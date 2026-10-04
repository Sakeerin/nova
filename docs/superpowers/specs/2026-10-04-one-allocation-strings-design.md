# One allocation per runtime string — design

**Date:** 2026-10-04
**Status:** approved in conversation, section by section, then checked
against the code by a verification workflow (4 checkers, 22 findings, 21
upheld and applied); this document is the written spec for review.
**Scope:** `crates/nova-runtime` only — `lib.rs` (`gc_str_filled`,
`gc_str`, `nova_rt_str_new`, `NovaStr`), `bytes.rs` (`gc_bytes`), their
tests, and the documentation that describes the string representation.

## 1. Why

`examples/05-json-api/BENCHMARK.md`'s "(reprofile-4)" put allocation,
collection included, at 22.6–24.7% of the json-api's server thread, and
allocation proper at 7.4–7.5 us (15.9–17.4% of the thread) per ten-user
request. "(alloc-fast-path)" tried to make each allocation cheaper — one
heap borrow instead of two, constant-size zeroing — and established no
reduction: the cost moved between frames.

This design attacks the **count** of allocations instead. A measurement made
for it (§7.1, to be recorded as "(alloc-mix)") found, per ten-user
`GET /users` request, three fresh runs agreeing to 0.03 objects:

- 300.9 objects, of which 67.0 are leaf (`scan = false`) objects, 22.3%;
- 77.7% of all objects in the 16-byte class: 197.9 scanned and 36.0 leaf;
- about 11,750 zeroed bytes, 39.5% of them in leaf slots;
- no large objects.

Every runtime-made `String` or `Bytes` value is two GC objects: a 16-byte
**scanned** header `NovaStr { len, ptr }` and a separate **leaf** buffer the
header points to. The 67 leaf objects per request are those buffers. Making
each string one object removes about 67 of 301 allocations per request
(22%), their header zeroing, and as many objects for the collector to mark
and sweep.

The two candidates "(alloc-fast-path)" left untried were weighed against
this and set aside:

- **Skipping the zeroing of leaf buffers:** at most 22% of zeroing calls and
  40% of zeroed bytes, roughly 1% of the thread, behind a new unsafe API.
- **Free-slot runs in the slot search:** touches every allocation but only
  the slot search's fixed per-call overhead; its ceiling cannot be bounded
  without building it.

## 2. Goal and non-goals

**Goal:** one GC allocation per runtime-made `String` and `Bytes` value,
with output and GC semantics unchanged — conservative, non-moving,
thread-local, size-class pages (ADR 0020).

**Non-goals:**
- Zero-copy substrings, slices or views. They would need a scanned header
  and are explicitly incompatible with this design (§4.5).
- Changing `NovaStr`'s field layout, `as_str`, `as_bytes`, or any code
  generator.
- Faster allocation per call. Untouched.
- A gate run. That is a separate step after this lands.

## 3. Facts the design rests on

Established by two read-only mapping passes, each with a critic, on
`main` at `45d05b0`:

1. **Only three sites write a `NovaStr`'s `ptr`:** `gc_str_filled`
   (`lib.rs:152`), `gc_bytes` (`bytes.rs:39`) and `nova_rt_str_new`
   (`lib.rs:213`, static literal data). Every conversion between `String`
   and `Bytes` copies through `gc_bytes`. No generated code allocates or
   fills a `NovaStr`: both backends only call `nova_rt_str_new` with a
   static pointer. The only allocation with a fixed size of 16 is a closure
   fat pointer (Cranelift `lib.rs:621`, LLVM `lib.rs:449`). Every other
   `nova_rt_alloc` is a record, sum, array or closure environment sized
   from its fields. That size can also be 16, but none of them is a string
   header.
2. **Every production read** of a string's bytes goes through `as_str`
   (`lib.rs:197`) or `bytes::as_bytes` (`bytes.rs:48`), which read only
   `ptr` and `len`.
3. **Nothing roots, frees or identifies a string by its bytes' address.**
   Registered roots (`fs.rs` stash, async state slots) hold the header.
4. **No string bytes reach the OS asynchronously.** The net write future
   holds the header in a scanned state slot and re-issues a synchronous
   non-blocking write on each poll (`net.rs:1023-1038`, `1127-1135`).
5. **Marking is range-based** (`gc.rs:23-25`, `mark_word` at
   `gc.rs:534-563`): a word anywhere inside a small slot, or inside a large
   object's `[addr, addr + size)`, marks the whole object. A leaf is marked
   but never traced (`pages.rs:307-313`, `gc.rs:548-560`).
6. **Ten functions hold only a bytes pointer across a GC allocation** and
   rely on it keeping the bytes alive:
   - `nova_rt_str_chars` and `nova_rt_bytes_to_ints`, each of which
     allocates its array block while holding the source's bytes;
   - the four `gc_str_filled` callers whose `fill` reads their arguments:
     concat, concat_n, join and json_quote;
   - `nova_rt_http_parse_request`;
   - the three `gc_bytes` callers that pass a slice of a live string:
     `bytes_from_string`, `bytes_to_string_unchecked` and `bytes_slice`.

   With one object per string, a bytes pointer keeps the whole string
   alive, which is at least as strong.

## 4. Design

### 4.1 Layout

A runtime-made `String` or `Bytes` value is **one leaf object** of
`16 + max(len, 1)` bytes:

```
base + 0   len: u64
base + 8   ptr: *const u8   (= base + 16)
base + 16  the bytes, len of them
```

- **Leaf:** `scan = false`. The object holds no pointer to any other GC
  object, so nothing it references can be freed early.
- **`max(len, 1)`** keeps `ptr` strictly inside its own object for every
  length. Without it, an empty string's `ptr = base + 16` would sit one
  past a 16-byte slot, in the next slot. An empty string costs 32 bytes, as
  today.

### 4.2 `gc_str_filled` (and through it `gc_str` and `fs::gc_message`)

1. Compute the size as `16usize.saturating_add(len.max(1))`. An overflow
   becomes `usize::MAX`, which `gc::alloc`'s `heap_layout` check rejects
   with the existing size-limit abort, never a too-small object. A caller
   whose own length arithmetic wrapped to a small value is still caught by
   `Fill::put`'s bounds check, as today.
2. `gc::alloc(size, false)` once.
3. Write the header: `len` and `ptr = base + 16`. Writing it before `fill`
   is a convention, not a requirement any test can observe: during `fill`
   nothing reads the header, the collector never reads a leaf's words, and
   `base` is held in the frame either way.
4. Build `Fill { out: from_raw_parts_mut(base + 16, len), at: 0 }`, run
   `fill`, and keep the existing `debug_assert_eq!(out.at, len)`.
5. Return `base` as the `*mut NovaStr`.

The GC-safety comment changes from "the buffer is held in this frame across
the header's allocation" to: there is now one allocation; `fill` runs after
it and must not allocate GC memory; whatever `fill` reads from the GC heap
stays alive because the caller's pointers are roots (unchanged). [Amended
2026-10-04: the no-allocation rule is a convention, not a memory-safety
requirement; see the note at the end of §4.5.]

### 4.3 `gc_bytes`

Same layout, same size arithmetic, header first, then
`copy_nonoverlapping(bytes, base + 16, len)`. It gains the GC-safety
paragraph it never had: three callers pass a slice of a live GC string,
which stays alive across this allocation because the caller's slice
pointer is a root and marking is range-based. **Both helpers change in the
same commit**, keeping their "identical layout, deliberately" contract.

### 4.4 `nova_rt_str_new`

The literal header becomes a 16-byte **leaf**. Its `ptr` targets static
data, which the collector never frees; tracing it never reached a GC
object. Its `# Safety` contract already requires static data; the doc
comment (`lib.rs:202-206`) says that the leaf flag depends on it.

### 4.5 Invariants, documented on `NovaStr`

- **A `NovaStr`'s `ptr` never points into another GC object.** With leaf
  headers this is load-bearing for memory safety: a header that pointed
  into another GC object would not keep it alive. A future zero-copy slice
  must not reuse `NovaStr` with a leaf header.
- **`gc_str_filled`'s `fill` must not allocate GC memory.** Every current
  `fill` only calls `Fill::put`.

Neither invariant can be pinned by a test (§5); both are stated where a
future change would break them.

[Amended 2026-10-04, after merge as PR #90: the sentence above was wrong
on both counts, as the branch's final review found. Where a `NovaStr`'s
`ptr` may be written can be pinned. A source guard,
`only_the_two_builders_write_a_novastr_ptr` in
`crates/nova-runtime/src/lib.rs`, scans the production code of every `.rs`
file under the runtime's `src`. It fails on a `.ptr =` assignment outside
`alloc_str_object` and `nova_rt_str_new`, a mutable borrow of a `.ptr`, a
`NovaStr` struct literal or `impl` block, or any production call to
`nova_rt_str_new`. It is a text scan, not a proof; its doc lists what it
cannot see. What the code generators pass to `nova_rt_str_new` still rests
on that function's `# Safety` contract. The second rule is a convention,
not a memory-safety invariant: the new object stays reachable from
`gc_str_filled`'s own frame across `fill`. The `NovaStr` doc comment now
says so.]

### 4.6 Cost by length

Derived from `CLASS_SIZES` (`pages.rs:25-31`), not measured:

| `len` | today: header slot + buffer slot | one object |
|---|---|---|
| 0 | 16 + 16 = 32 | 32 |
| 1–112 | equal | equal |
| 128 | 16 + 128 = 144 | 160 |
| 256 | 16 + 256 = 272 | 320 |
| 1024 | 16 + 1024 = 1040 | 1280 |
| 2032 | 16 + 2048 = 2064 | 2048 |
| 2033–2048 | 16 + 2048 | large path, exact size |

The difference lies between −16 and +240 bytes per string, at a class
boundary. Lengths 2033–2048 move to the large-object path: a system
allocation and an entry in `h.large`. The collection trigger charges the
slot or the exact large size, so collection cadence shifts with it.

## 5. Tests, written first

**Rewritten and renamed:** `bytes.rs`'s
`a_bytes_value_has_a_scanned_header_over_a_leaf_buffer` becomes
`a_bytes_value_is_one_leaf_object_with_its_bytes_inline`, and
`a_bytes_buffer_above_the_gc_floor_is_tracked_at_its_exact_size` becomes
`a_bytes_value_is_tracked_at_its_full_size`. They assert the new layout
through `object_info`, which answers only for an object's exact start and
reports the post-floor requested size. The doc comments that name or
describe these tests are rewritten in the same commit: `bytes.rs:296-310`,
`333-337` and `351-353`.

**New, each watched failing before the change.** In what follows,
`expected` is the Rust `&'static str` the string was built from, never a
read taken earlier from the GC heap.

1. **Layout of every helper.** For `gc_str`, `gc_str_filled` (through
   `nova_rt_str_concat`), `gc_bytes` and `nova_rt_bytes_to_string_unchecked`,
   at `len` of 0, 2, a length whose `16 + len` crosses a class boundary
   (for example 113), and 2033 (the large path):
   - `gc::object_info(s as usize) == Some((16 + max(len, 1), false))`;
   - `(*s).ptr as usize == s as usize + 16`;
   - `(*s).len == len as u64`, and the bytes read back equal `expected`.
2. A literal header from `nova_rt_str_new` is `Some((16, false))`.
3. **An empty string's `ptr` lies inside its own object,** both bounds:
   `s as usize + 16 <= ptr as usize` and
   `(ptr as usize) < s as usize + gc::object_info(s as usize).unwrap().0`.
4. **Liveness through the bytes pointer,** at `len` of 2, 113 and 2033:
   - (a) before: `gc::object_info(s as usize).is_some()`;
   - (b) run `gc::sweep_with_roots_for_test(&[(*s).ptr as usize + k])` with
     `0 <= k < max(len, 1)` (for the empty string, only `k = 0`);
   - (c) after: `gc::object_info(s as usize) == Some((16 + max(len, 1),
     false))`, the liveness assertion. Today it returns `None`, because the
     header's slot is freed;
   - (d) control: a second runtime string left out of the roots has
     `object_info == None` afterwards;
   - (e) only then, `as_str(s) == expected`.

   It uses explicit roots, so it runs deterministically on every platform.

**Deliberately not tested:** the header-first order and the rule that
`fill` must not allocate (§4.2, §4.5). Neither has an observable effect a
test could catch. [Amended 2026-10-04: where a `NovaStr`'s `ptr` may be
written, which the §4.5 invariant rests on and this section did not list,
is now pinned by a source guard; see the note at the end of §4.5.]

**Named mutants, each run with exit codes and result lines checked:**

| mutant | expected to fail |
|---|---|
| a runtime string's object left `scan = true` | new test 1 |
| the header's `ptr` set to `base + 8` while `fill` still writes at `base + 16` | new test 1, test 4 through its content comparison, and the existing content tests and JSON fixtures |
| `max(len, 1)` dropped | new tests 1 and 3 |
| only `gc_str_filled` changed, `gc_bytes` left two-object | new test 1 for `gc_bytes` |
| `nova_rt_str_new` left `scan = true` | new test 2 |

**Unchanged and expected to pass:** the full workspace suite, including
every `NOVA_GC_STRESS` fixture. Those discriminate only on Windows, where
the collector frees memory.

## 6. Documentation

**Rewritten, because they state the representation in the present tense:**

- runtime comments:
  - `gc.rs:25`;
  - `lib.rs:9-10`, `102-103`, `113-114`, `122-128`, `135`, `202-206` and
    `209-210`, and `NovaStr`'s doc, which gains the §4.5 invariant;
  - `bytes.rs:5-6`, `23-24`, `26-27`, `30-31` and `101`, plus the test doc
    comments named in §5.
- `crates/nova-hir/src/lib.rs:33-34`;
- `std/bytes/lib.nova:6-7`;
- `nova-spec/13-RUNTIME.md`:
  - `131` ("`String` = heap object: `{ len: usize, data: ptr<u8> }`"),
    rewritten to say one object with the bytes inline at base + 16;
  - `136-139`, given a note that a future `Str` view must not be a
    `NovaStr` with a leaf header (§4.5);
  - `173` ("string byte buffers — are leaves").
- `nova-spec/20-STDLIB.md:234-235`;
- the doc comments on tests in `crates/nova-cli/tests/run_tests.rs` at
  about `2809-2810`, `6711-6712` and `6813-6814`.

Optional rewording, still true under one object: the "result's buffer"
sentences at `lib.rs:275-276`, `296-298`, `320`, `324-325` and `387-390`.

**Given dated notes, not rewritten:** statements in historical records that
a string is a header plus a separate buffer:

- `examples/05-json-api/BENCHMARK.md`: `1494-1498`, `1510-1512`,
  `2227-2229`, `5009-5010` and `5017-5018`;
- `CHANGELOG.md`: `496-500`, `573-574` and `3949-3951`;
- `docs/benchmarks/README.md:878-899`, the canonical statement of the
  allocations-per-header arithmetic. Its restatements are left as they are,
  because ADR 0019:341 already points readers to the README: `CHANGELOG.md`
  `34-35` and `1947-1949`, ADR 0019:338-340, `BENCHMARK.md:1067-1072`, the
  example's `README.md:101-104`, and `docs/benchmarks/server.nova:18-19`.

**The implementation step re-runs a gutter-normalising sweep,** because a
line-oriented grep can miss wrapped phrasing. Its terms: "two-object", "two
objects", "two allocations", "`NovaStr` node", "header over", "leaf buffer",
"string byte buffer", "GC buffer", "{len, ptr}", "{ len, data", and
"header" near "buffer". Hits where "header" means an HTTP header are set
aside.

## 7. Measurement

### 7.1 Before (already measured, to be recorded)

"(alloc-mix)": a scratch patch, never committed, counting objects, zeroed
bytes and requested bytes per size class and scan flag in `gc::alloc`. It
printed cumulative totals at each collection under `NOVA_GC_DEBUG`. Ten
users, 200 connections, `--duration 15 --warmup 0`, three fresh runs. Per
request is the last collection's totals over the generator's count.
Figures in §1.

### 7.2 After

Predictions are written before any of the change exists. Then:

1. **Object count:** the same scratch counters on the new build. Expect
   about 234 objects per request, with no 16-byte scanned objects from
   strings.
2. **Per call:** "(json-quote)"'s picosecond harness, `stringify` on three
   strings and `users_json` at ten users. `before` and `after` are
   alternated over five runs, one fresh process each.
3. **Server:** ten users, 200 connections, `--warmup 5 --duration 15`, six
   alternated pairs, three in each order. A gain is claimed only on
   disjoint ranges.
4. **Identity:** byte size and SHA-256 for every binary, and sibling names
   deleted before each `nova build -o`.

## 8. Risks

- **The invariant in §4.5 becomes load-bearing.** It holds today by
  construction (§3.1), and the `NovaStr` doc states it.
- **Size-class shifts** may cost or save bytes per string (§4.6) and move
  collection cadence. They are measured, not assumed, in §7.2.
- **The stress fixtures** discriminate only on Windows. New test 4 checks
  bytes-pointer liveness deterministically on every platform.
- **Lengths 2033–2048 move to the large path,** with its per-collection
  sort and linear `object_info`. Rare in this workload: no large objects
  were seen. Tests 1 and 4 cover the move.

## 9. Out of scope

Skipping zeroing, free-slot runs, `gc::alloc`'s per-call path, the
collector, any code generator, and the gate run.
