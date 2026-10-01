# A size-class page heap for the collector — design

**Date:** 2026-10-01
**Branch:** `gc-page-heap`
**Base:** `main` == `origin/main` == `331dec5`, 744 commits. The last full
suite was 1147 passed / 0 failed / 8 ignored across 45 targets, on the PR
#61 branch. PRs #62 and #63 after it changed documentation only.

---

## 1. What this is, in one paragraph

The collector in `crates/nova-runtime/src/gc.rs` allocates every object
separately from the system allocator and records it in a `Vec<Obj>`. Each
collection rebuilds and sorts an index of every object so marking can
binary-search it, then frees each dead object with its own `dealloc` call
and one `HashMap` removal. `examples/05-json-api/BENCHMARK.md`'s
"(gc-phase-cost)" amendment measured that per-object work at 69.3–70.7% of
collection time. **This change puts every object of 2 KiB or less into
64 KiB pages of fixed-size slots.** Each page tracks its slots with
allocated, marked and scan bitmaps. A sweep frees slots by bitmap
arithmetic instead of calling `dealloc`, marking finds a slot by searching
a few dozen page bases instead of a 40.5k-entry index, and allocation takes
a free bit instead of calling the system allocator. Objects over 2 KiB keep
today's path. The collector stays conservative, non-moving and
thread-local, and `gc::alloc`'s signature and every caller stay as they
are.

---

## 2. What is known, and from where

### 2.1 The costs (measured on `c34420e` plus scratch timers)

From `examples/05-json-api/BENCHMARK.md`, ten users, 200 connections. Per
request, in the runs that kept request counts:

| cost | µs per request | source amendment |
|---|---|---|
| collector, whole | 93.8–101.6 | gc-phase-cost |
| — sweep (`dealloc` 21.6–25.0, `forget_freed_state` 6.8–8.8, rest 12.4–14.6) | 42.9–46.2 | gc-phase-cost |
| — mark | 25.8–28.0 | gc-phase-cost |
| — sort the index | 19.6–21.2 | gc-phase-cost |
| — build the index, clear marks | 2.6–3.2 | gc-phase-cost |
| allocation outside collection (65.6–77.4 ns × ~516) | 33.8–39.9 | noncollector-cost |
| everything outside both | 92.9–101.6 | noncollector-cost |

This design targets the sweep, the sort, the index build and clearing, and
allocation: 65.5–70.7 µs per request of collector work plus allocation's
33.8–39.9 (the two come from different runs). It cannot remove all of
that: a bitmap sweep and a slot allocation still cost something. **How
much disappears is a prediction, and the measurement in section 9 is what
settles it.**

### 2.2 The objects (measured on an earlier build)

The "(alloc-per-request)" amendment's size histogram, ten users, before
PRs #60 and #61 cut the count from 963 to 516 objects per request:

| ≤16 B | 17–64 B | 65–256 B | 257–4096 B | >4096 B |
|---|---|---|---|---|
| 739.1 obj | 92.0 | 107.0 | 25.0 | 0 |

So 77% of objects were 16 bytes or smaller, and none were over 4 KiB.
Whether any of the 25 objects between 257 and 4096 bytes exceed 2 KiB is
not measured; section 9 counts the large path's use.

### 2.3 The path (read from source on `331dec5`)

- `gc::alloc(size, scan)` floors `size` at 8, aborts on a size no `Layout`
  can describe, calls `maybe_collect(size)`, then `alloc_zeroed` at 16-byte
  alignment, and pushes an `Obj { addr, size, scan, marked }`.
- `collect` captures the stack base (giving up on platforms without one,
  where collection is skipped and objects leak), copies `PINNED` into
  `ROOTS`, and calls `nova_gc_collect_roots`. `collect_with_roots` clears
  every mark, builds and sorts `(start, end, index)`, marks from the roots
  through `mark_word`'s `partition_point`, then sweeps with `swap_remove`,
  `dealloc` and `task::forget_freed_state` per freed object.
- The next threshold is max(1 MiB, 2 × live bytes). `NOVA_GC_STRESS`
  collects on every allocation and `NOVA_GC_DEBUG` prints one line per
  collection.
- `task.rs`'s `BY_STATE` maps state-object addresses to task ids. Its
  invariant is that a key is present only for a state object that is still
  the allocation it was spawned with, so a freed address must be forgotten
  before it can be reissued.

---

## 3. Goals, non-goals and success

**Goals.**
- Remove the per-collection index rebuild and sort for small objects.
- Remove the per-object `dealloc` and `HashMap` removal from the sweep.
- Make small-object allocation cheaper than a system allocator call.
- Return empty memory to the system beyond a bounded reserve.

**Non-goals.**
- Precise or moving collection, generations, or concurrency.
- Collection off Windows. It stays skipped there, as today.
- Any change to codegen, to `gc::alloc`'s signature, or to its callers.
- Finer size classes, or tuning the class list beyond section 4.1.

**Success** is all of the following.
- The whole suite, the GC-stress gates, clippy and rustfmt pass.
- Ten-user throughput on `examples/05-json-api` improves, with before and
  after ranges disjoint over at least three alternated fresh-process
  readings each.
- A rerun of the phase and allocation decomposition shows which costs went
  away.
- The peak working set before and after is reported beside throughput.

There is no fixed req/sec target.

---

## 4. Design

### 4.1 Size classes

Twenty-four classes, all multiples of 16 bytes:

| range | step | classes |
|---|---|---|
| 16–128 | 16 | 16, 32, 48, 64, 80, 96, 112, 128 |
| 160–256 | 32 | 160, 192, 224, 256 |
| 320–512 | 64 | 320, 384, 448, 512 |
| 640–1024 | 128 | 640, 768, 896, 1024 |
| 1280–2048 | 256 | 1280, 1536, 1792, 2048 |

A request of `size` bytes (after the 8-byte floor) takes the smallest class
of at least `size`. A 128-entry table indexed by `(size - 1) / 16` maps a
size to its class.

Rounding waste:
- **Up to 128 bytes**, a request is rounded up to the next multiple of 16,
  which adds at most 15 bytes. As a fraction that can be large, for example
  17 bytes in a 32-byte slot. A request of 8 to 16 bytes takes a 16-byte
  slot.
- **Above 128 bytes**, rounding adds less than 25% of the requested size.
  The worst case is 1025 bytes in a 1280-byte slot, 24.9%.
- **Against today**, how the system allocator's own granularity rounds
  today's 16-byte-aligned layouts is not measured. So how much of this
  waste is new is not known.

A request over 2048 bytes is a large object (4.4).

### 4.2 Pages and their descriptors

- **A page is 65,536 bytes for one class**, from the system allocator at
  16-byte alignment (`ALIGN`). It is **not** aligned to its own size. Rust's
  Windows system allocator honours larger alignments by over-allocating,
  which would waste up to a page per page.
- **The page holds slots only.** A page of class `c` holds
  `floor(65536 / c)` slots, from 4,096 at 16 bytes to 32 at 2,048. Bytes
  after the last whole slot belong to no object.
- **Each page has a descriptor kept out of band**: its base, slot size,
  slot count, and three bitmaps of `[u64; 64]`:
  - **allocated:** the slot holds an object;
  - **marked:** set during marking, and all clear outside a collection;
  - **scan:** the slot's object is traced. Meaningful only where
    allocated is set.
- **Bits past the last slot are never set** in allocated or marked, and
  the allocator never returns them.
- **`alloc` still returns a bare pointer with nothing in front of it**, as
  `nova-spec/13-RUNTIME.md` requires.
- **Descriptors live in a vector whose indices are stable.** A released
  page's entry is put on a free list and reused, so per-class cursors and
  free lists can hold descriptor indices across page insertions.

### 4.3 The page directory and the heap's address range

- **The directory** is a `Vec<(base, descriptor index)>` sorted by base. It
  changes only when a page is added or released, by an O(pages) insertion
  or removal, never per object.
- **The heap's address range** `[lo, hi)` spans every page and every large
  object. It is recomputed when a page or large object is added, and after
  any sweep that released one.

### 4.4 Large objects

- **An object over 2048 bytes** is allocated exactly as today:
  `alloc_zeroed` at `ALIGN`, recorded as an `Obj { addr, size, scan,
  marked }` in a `large` vector.
- **The large vector is sorted by address at the start of each
  collection.** It is short; the per-collection sort over the whole heap
  becomes a sort of these few.
- **The large sweep uses `retain`**, which preserves that order, so the
  vector is still sorted when the state map is pruned (4.9).

### 4.5 Allocation

`alloc(size, scan)` keeps its 8-byte floor, its `heap_layout` check and
abort, and its call to `maybe_collect` before the heap is borrowed. Then:

**Small objects:**
1. Look the class up in the table.
2. The class keeps a cursor: a current page and a bitmap-word hint. Search
   the current page's allocated bitmap from the hint for a word with a
   zero bit inside the slot count, and take its lowest zero bit
   (`trailing_zeros` of the inverted word).
3. If the current page is full, move to the next page on the class's
   free-page list. That list holds every page of the class that had a free
   slot after the last sweep, plus pages added since.
4. If the list is empty, take a page from the empty reserve and set it up
   for this class, or failing that allocate a new page with `alloc`. Every
   slot is zeroed when it is handed out, so the page itself does not need
   `alloc_zeroed`. Insert the page into the directory and update the range.
5. Set the slot's allocated bit, set its scan bit to `scan`, write zeroes
   over its full slot size, and return it.

Zeroing the full slot keeps today's guarantee that unwritten fields read as
null. It also means a freed slot's stale contents can never be traced
after reuse.

**Large objects:** as in 4.4.

**The collection trigger keeps its form**, max(1 MiB, 2 × live). Both
`alloc_since_gc` and `live_bytes` now count bytes taken, which is a slot's
class size or a large object's size, not bytes requested. For the same
program the trigger therefore fires a little sooner, by at most the
rounding waste.

### 4.6 Collection: roots

Unchanged. `collect` captures the stack base, giving up on platforms
without one exactly as today. It copies `PINNED` into `ROOTS` and calls
`nova_gc_collect_roots`.

### 4.7 Collection: marking

- **No clear-marks pass over pages.** Every page's marked bitmap is already
  clear, left that way by the previous sweep. Only the large vector's
  records are unmarked, and it is sorted (4.4).
- **For each candidate word `w`**, from the roots and from scanned objects:
  1. Skip it if it is zero or outside `[lo, hi)`.
  2. Binary-search the directory for the last page whose base is `≤ w`. If
     `w` lies before that page's last whole slot ends, the slot is
     `(w − base) / slot_size`. A slot whose allocated bit is set and whose
     marked bit is clear gets marked. If its scan bit is set, `(slot
     address, slot size)` goes on the work stack.
  3. Otherwise binary-search the large vector. An unmarked object
     containing `w` is marked, and pushed as `(addr, size)` if it is
     scanned.
- **Draining the work stack** reads every aligned 8-byte word in each
  `(addr, len)` and treats it as a candidate.
- **Interior pointers keep their object alive**, because any address within
  a slot maps to that slot.
- **Scanning a slot's full size** instead of the requested size reads only
  the zeroed tail besides. Those words are rejected first, at step 1.
- **The work stack's `Vec` is kept between collections** rather than
  allocated each time.

### 4.8 Collection: sweeping

**Pages.** For each bitmap word in use:

```text
freed     = allocated & !marked
allocated = marked
marked    = 0
```

The freed bytes are `popcount(freed) × slot_size`, and the live slots are
`popcount(allocated)`. After its last word:
- a page with no allocated slot is **empty** (4.10);
- a page with at least one free slot joins its class's free-page list;
- a full page joins no list.

Each class's cursor is reset to the first page on its list, with its hint
at zero.

**Large objects.** Every record that is not marked is `dealloc`ed and
dropped by `retain`, and the rest are unmarked.

**Counters and output.** `live_bytes`, `freed_bytes`, `collections`,
`alloc_since_gc` and `next_gc` update as today. The `NOVA_GC_DEBUG` line
keeps its exact format:

```text
nova-gc: collection N freed X bytes, Y objects live (Z bytes)
```

Here Y is allocated slots plus large objects, so existing harness scripts
keep parsing it.

### 4.9 The executor's state map

- **What changes.** `task::forget_freed_state(addr)`, called once per freed
  object, is replaced by **`task::prune_freed_states(is_live)`**, called
  once per collection, after the sweep and before anything can be
  allocated again. It keeps only the `BY_STATE` keys for which `is_live`
  holds.
- **The predicate** `is_live(addr)` holds exactly when `addr` is the start
  of an allocated slot or of a large object, found by the same directory
  and large-vector searches as marking.
- **Why it is safe to run here.** It runs with `HEAP` borrowed, as
  `forget_freed_state` does today. That is safe for the same reason: it
  borrows only `BY_STATE` and allocates nothing through `gc::alloc`.
- **Cost:** O(entries in `BY_STATE`), which is O(live tasks), instead of
  O(freed objects).
- **The invariant is unchanged:** every address a collection frees is
  forgotten before it can be issued again. The test-only `reset` prunes
  with a predicate that rejects every address.

### 4.10 Releasing pages

- **The reserve.** An empty page goes into an empty reserve, capped at 16
  pages. That is `INITIAL_THRESHOLD / 65536`, 1 MiB.
- **Release beyond the cap.** Empty pages past the cap are `dealloc`ed,
  removed from the directory and from their class's lists, and their
  descriptor entries freed. The range is recomputed.
- **Reuse.** A class needing a page takes one from the reserve before
  asking the system for one.
- **Effect.** A program that keeps cycling through the initial threshold
  reuses its pages without system calls. A program whose live set shrinks
  returns all but 1 MiB of empty pages.

### 4.11 Off Windows

Collection is still skipped, so nothing is ever freed. Pages fill and new
pages are added, and the program leaks as it does today. It no longer also
keeps a 32-byte `Obj` record for every small object.

---

## 5. Edge cases and error handling

- **An undescribable size** aborts with today's `nova: panic:` message,
  before anything else, unchanged.
- **A page or large allocation the system refuses** goes to
  `handle_alloc_error` with that allocation's layout, as today.
- **Re-entrancy:** `maybe_collect` runs before `HEAP` is borrowed for
  allocation, so a collection never runs inside an allocation's borrow.
  This is unchanged.
- **The conservative scan's new edges** all over-retain at worst, never
  free a reachable object:
  - A pointer one byte past an object's end, or into its rounding tail,
    keeps alive the allocated slot it lands in.
  - A pointer into a page's leftover tail keeps nothing alive.
  - A stale stack word that names a freed slot which has since been reused
    keeps the new object alive. The system allocator had the same hazard
    whenever it reissued an address.
- **`NOVA_GC_STRESS`** collects on every allocation through the same paths.
  It has nothing special to handle, but its gates run slower or faster with
  the new costs.

---

## 6. Test hooks

**Kept, with their signatures:**
- `object_info(addr)` returns the exact `(size, scan)` passed to `alloc`,
  or `None` if `addr` is not the start of a live object. The size comes from
  a map of requested sizes compiled only into test builds. The map is
  written at every allocation and is consulted only when `addr` is a live
  object's start, so stale entries are never read. The scan flag comes from
  the bitmap or the large record.
- `root_count`, `collect_for_test` and `sweep_with_roots_for_test` are
  unchanged.

**Changed, inside `gc.rs`'s test module:**
- `reset` frees every page and large object and clears the directory, the
  reserve, the cursors and the lists. It prunes `BY_STATE` with a
  reject-everything predicate and clears `PINNED`.
- `count` returns the number of live objects: allocated slots plus large
  objects.

**New, test-only:**
- `page_count()` and `reserve_count()`, for the release tests.
- `directory_is_sorted()`, which checks the directory's order and that
  every descriptor it names is in use.

---

## 7. Records to amend

- **ADR 0020, new:** "Size-class page heap". It records the decision, the
  two rejected designs (Immix-style mixed-size blocks, and free lists
  threaded through free slots) and why, the footprint trade, and the
  reserve cap.
- **`gc.rs`'s module doc comment:** the paragraph saying memory goes "back
  to the system allocator rather than into an arena", and its description
  of the sweep's `forget_freed_state` call. Small objects' memory now
  returns to a page's free slots, and an address is still not a durable
  identity.
- **`nova-spec/13-RUNTIME.md`:** the `Obj` side-table description, which
  becomes page descriptors plus large-object records and stays out of band,
  and §3.1's matching sentence.
- **`docs/adr/0009-async-execution-model.md` and `task.rs`'s doc comments**,
  where they name the per-freed-address `forget_freed_state` call.
- **`CHANGELOG.md`:** a `### Changed` entry under `[Unreleased]`.
- **The population for these edits is found by set difference, not
  recalled.** Run
  `comm -23 <(git grep -l TOKEN | sort) <(git diff --name-only main..HEAD | sort)`
  with each of `forget_freed_state`, `system allocator`, `arena` and
  `swap_remove` as TOKEN. Every file it lists is examined.

---

## 8. Testing

Every new test is written first and watched failing.

**Existing tests stay unchanged in intent:**
- `unrooted_objects_are_freed`, `no_roots_frees_everything`,
  `transitive_marking_keeps_referenced_objects`,
  `interior_pointer_keeps_object` and `leaf_objects_are_not_traced`;
- the registry multiset tests;
- the `heap_layout` tests;
- `task.rs`'s tests that a freed state's `BY_STATE` key goes with it;
- `run_tests.rs`'s
  `a_recycled_state_address_does_not_resolve_a_never_spawned_future`, which
  exercises address reuse end to end.

**New tests in `gc.rs`:**
- **Size classes:** every size from 8 to 2048 maps to the smallest class
  that fits, checked exhaustively against the table in 4.1. The boundaries
  16, 17, 128, 129, 2048 and 2049 are asserted by name, and 2049 takes the
  large path.
- **Slot reuse is zeroed:** fill a slot with nonzero bytes, free it,
  allocate the same class again until that address returns, and assert
  every byte of its slot size is zero.
- **A rounding-tail pointer** keeps its object alive.
- **A page-tail pointer** keeps nothing alive. Use the 48-byte class, whose
  page has 16 leftover bytes.
- **A word just outside `[lo, hi)`** marks nothing.
- **Mixed chains:** a small → large → small chain survives from one root,
  and an unreachable large object is freed.
- **Scan bits are per slot:** a leaf beside traced slots in the same page
  is not traced. A slot reused with the opposite scan flag honours the new
  one, in both directions.
- **Release:** allocate more than 16 pages' worth of one class and free it
  all. `page_count` drops to the reserve's 16 and `reserve_count` is 16. The
  next allocation of any class takes a reserve page and adds no new page.
- **Directory:** after interleaved allocation, freeing and release across
  several classes, `directory_is_sorted()` holds.

**New tests in `task.rs`:**
- A live task's `BY_STATE` key survives a collection.
- A collection that frees only objects which are not task states removes
  no key.

**Mutations.** Each is applied once and must make a named test fail:
- skip the zeroing at allocation;
- drop the allocated-bit check in marking;
- skip clearing marks in the sweep;
- make the prune predicate always true;
- remove the reserve cap.

A mutant that hangs is judged by what the mutant does, not by what the
shipped code does. Each result is recorded in the PR.

**Gates:**
- `cargo test --locked --workspace`, including every `*_under_gc_stress`
  test in `crates/nova-cli/tests/run_tests.rs`, among them
  `gate_async_tasks_under_gc_stress` and
  `interpolation_nary_under_gc_stress`;
- `cargo clippy --locked --workspace --all-targets --all-features -- -D warnings`;
- `cargo fmt --all --check`.

Any recurrence of the pre-existing 0xC0000005 child crash is tallied in
`docs/adr/0008-attributes-and-test-isolation.md` §4, not dismissed.

---

## 9. Measurement

- **Predictions** are written before any measurement: throughput,
  collector cost per request, allocation cost per allocation, and peak
  working set.
- **Builds:** release-runtime builds of `examples/05-json-api` on `main` and
  on the branch. Every sibling `NAME` and `NAME.exe` is deleted before each
  `nova build -o NAME.exe`, and each binary's byte size is recorded.
- **Throughput:** at least three readings per build, alternated, one fresh
  process each. Ten users, 200 connections, `--warmup 5 --duration 15`. A
  gain is claimed only if the ranges are disjoint.
- **Decomposition:** scratch phase timers adapted to the new phases: root
  scan, large sort, mark, page sweep, large sweep, prune, plus the
  calibrated one-in-16 allocation sample. They run in at least three fresh
  processes with each run's load-generator RESULT line saved to a file, and
  are reported per request beside "(gc-phase-cost)". Never committed.
- **The large path's use:** objects and bytes per request taken by the
  large path, from the same scratch build.
- **Footprint:** each process's peak working set before and after, read
  from the OS before it is killed.
- **The record:** a dated amendment in `examples/05-json-api/BENCHMARK.md`
  and a CHANGELOG Measured bullet, checked by an independent agent before
  commit.

---

## 10. Risks

- **The saving is uncertain.** Marking may not get much cheaper if most of
  its cost is reading the 650 KB of survivors rather than the lookup.
- **Fragmentation.** Partly filled pages across 24 classes can hold more
  memory than the system allocator did. The footprint measurement exists
  to show this.
- **The 2 KiB boundary** may leave a frequent allocation on the large path.
  Section 9 counts it.
- **The pre-existing 0xC0000005 flake** may recur during the suite. It is
  recorded, not treated as caused by this change without evidence.

---

## 11. Decisions taken in design review

- **Direction:** a size-class page heap, chosen over local fixes, recycling
  freed blocks, or measuring the scanned words first.
- **Success:** a disjoint throughput gain plus a rerun decomposition, with
  no fixed target.
- **Footprint:** bounded. Rounding waste is accepted, empty pages beyond a
  1 MiB reserve are returned, and the peak working set is reported.
- **Approach A:** 64 KiB pages with side bitmaps, classes from 16 bytes to
  2 KiB, and today's path above that.
- **State map:** pruned once per collection instead of once per freed
  object.
