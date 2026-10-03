# A cheaper allocation fast path for the collector — design

**Date:** 2026-10-03
**Branch:** `alloc-fast-path`
**Base:** `main` == `origin/main` == `cb4a80a`, 775 commits. The last full
suite was 1173 passed / 0 failed / 8 ignored, on the PR #77 branch. PR #78
after it changed documentation only.

---

## 1. What this is, in one paragraph

`gc::alloc` in `crates/nova-runtime/src/gc.rs` is called for every object
Nova allocates, a few hundred times per json-api request.
`examples/05-json-api/BENCHMARK.md`'s "(reprofile-2)" amendment puts
allocation proper at 7.6–8.5 us per ten-user request, apart from the
collector's 4.4–4.9 us. **This change trims the path a small allocation
takes, without changing what `alloc` promises.**
- Every allocation reads the thread-local heap once instead of twice.
- A small request skips a `Layout` check it cannot fail.
- The stress flag is read from the heap rather than from a `OnceLock`.
- The smallest slots are zeroed with inline stores instead of a call to
  `memset`.

Collection policy, marking, sweeping, the page layout and every caller
stay as they are.

---

## 2. What is known, and from where

### 2.1 The costs (measured on `369a82c` with the scratch sampler)

"(reprofile-2)" sampled the json-api's server thread three times:

| part of `gc::alloc` | share of the thread |
|---|---|
| `gc::alloc`, collection included | 22.7–23.6% (12.0–13.3 us per request) |
| the collector, its callees included | 8.1–8.6% (4.4–4.9 us) |
| allocation proper: the rest | 14.4–15.0% (7.6–8.5 us) |
| — `Pages::alloc_slot` | 4.9–5.1% |
| — `gc::alloc`'s own code | 3.8–4.2% |
| — `memset`, called from `alloc_slot` | 2.1–2.6% |
| — a second `LocalKey::with` instance | 2.1–2.2% |

### 2.2 The path (read from source on `cb4a80a`)

For a request of `size` bytes with `scan` set as given:
1. `alloc` raises `size` to at least 8 and calls `heap_layout(size)`, which
   asks `Layout::from_size_align`, for every request.
2. It computes the class and the slot size `taken`, then calls
   `maybe_collect(taken)`.
3. `maybe_collect` does `HEAP.with(|h| h.borrow())` to test
   `alloc_since_gc + taken >= next_gc`, then reads `stress()`, a
   `OnceLock<bool>` over `NOVA_GC_STRESS`. If either holds it calls
   `collect()`.
4. `alloc` does a second `HEAP.with(|h| h.borrow_mut())` to take the slot,
   adds `taken` to `alloc_since_gc` and `live_bytes`, and returns.
5. `Pages::alloc_slot` → `take` scans the class's current page's
   allocated bitmap from a hint, sets the allocated and scan bits, and
   zeroes the slot with `std::ptr::write_bytes(addr, 0, page.slot)`. The
   size is a runtime value, so that is a `memset` call.

`stress()` has one caller, `maybe_collect`.

---

## 3. Goals, non-goals and success

**Goals.**
- One `HEAP` access and one `RefCell` borrow per allocation that does not
  collect.
- No `Layout` check for a request of `SMALL_MAX` bytes or less.
- No `OnceLock` read per allocation.
- No `memset` call for a slot of 128 bytes or less.

**Non-goals.**
- Any change to when a collection happens, what it marks or frees, the
  page layout, the size classes, the large-object path, or `next_gc`'s
  growth rule.
- Any change to what `alloc` returns. A slot is still fully zeroed when
  handed out.
- Uninitialised allocations, or a new slot-search structure. Both were
  considered and set aside; see section 9.

**Success.**
- Per allocation: a disjoint cut in a harness's ns per allocation, before
  against after.
- In the profile: `memset` under `alloc_slot` and the second `LocalKey`
  instance shrink, measured with the same sampler.
- On the server: a gain is claimed only if the ranges are disjoint. A
  saving of a few microseconds per request may not show there, and no
  target is set.

---

## 4. Design

### 4.1 The allocation flow

`alloc(size, scan)` becomes:

1. `let size = size.max(8);`
2. **Small.** If `size <= pages::SMALL_MAX`, the class is
   `pages::class_of(size)` and `taken` is `pages::CLASS_SIZES[class]`. No
   `Layout` is built. A size this small is always describable, which a
   test pins (section 6).
3. **Large.** Otherwise `heap_layout(size)` is called and its `None` still
   aborts with today's diagnostic. `taken` is `size`, as now.
4. **One borrow.** `HEAP.with(|h| { let mut h = h.borrow_mut(); ... })`
   tests `h.alloc_since_gc + taken >= h.next_gc || h.stress()`.
   - **Not over:** it takes the slot (or, for a large request, makes the
     `alloc_zeroed` allocation and pushes its `Obj`), adds `taken` to
     `alloc_since_gc` and `live_bytes`, records the test-only requested
     size, and returns the pointer from inside the same borrow.
   - **Over:** it returns without allocating.
5. **Over, slow path.** Outside the borrow, `alloc` calls `collect()`, then
   allocates in a second `HEAP.with` exactly as step 4's not-over branch
   does. That is today's order: collect, then allocate.

`maybe_collect` is removed; its test moves into step 4. The allocating
code shared by steps 4 and 5 lives in one private function taking
`&mut Heap`, so the two branches cannot drift apart.

### 4.2 The stress flag

`Heap` gains a field `stress: Option<bool>`, `None` in `Heap::new`. A
method `Heap::stress(&mut self) -> bool` reads `NOVA_GC_STRESS` from the
environment the first time and caches the answer in the field. The
free function `stress()` and its `OnceLock` are removed, since
`maybe_collect` was their only caller.

The flag is per thread, like `HEAP`. The environment is read once per
thread instead of once per process. It does not change while a program
runs, so every thread reads the same answer.

### 4.3 Zeroing small slots

`take` calls a new `zero_slot(addr, slot)` in place of `write_bytes`:

```rust
/// Zero the `slot` bytes at `addr`, with a constant size for the
/// smallest classes so each zeroing can compile to inline stores.
#[inline(always)]
unsafe fn zero_slot(addr: usize, slot: usize) {
    let p = addr as *mut u8;
    match slot {
        16 => std::ptr::write_bytes(p, 0, 16),
        32 => std::ptr::write_bytes(p, 0, 32),
        48 => std::ptr::write_bytes(p, 0, 48),
        64 => std::ptr::write_bytes(p, 0, 64),
        80 => std::ptr::write_bytes(p, 0, 80),
        96 => std::ptr::write_bytes(p, 0, 96),
        112 => std::ptr::write_bytes(p, 0, 112),
        128 => std::ptr::write_bytes(p, 0, 128),
        _ => std::ptr::write_bytes(p, 0, slot),
    }
}
```

Every slot is still fully zeroed. Whether the compiler emits inline
stores for the constant arms is not certain from the source. The sampled
profile in section 7 checks it, and if `memset`'s share does not fall,
this part is dropped rather than kept.

---

## 5. Edge cases and error handling

- **An undescribable size** still reaches `heap_layout`, since it is
  larger than `SMALL_MAX`, and still aborts with today's message.
- **`NOVA_GC_STRESS`** still collects before every allocation. Its
  end-to-end tests (section 6) cover it.
- **A platform without stack bounds:** `collect()` still sets
  `alloc_since_gc = 0` and returns, so the slow path allocates as today.
- **Re-entrancy:** `collect()` borrows `HEAP` itself. It is only ever
  called outside step 4's borrow, as `maybe_collect` was.

---

## 6. Testing

Characterization tests come first. They must pass on the base before any
change, and each must fail under a named mutant.

1. **Every class zeroes over stale bytes** (`gc/pages.rs`). For each of the
   24 classes: allocate a slot, write `0xAB` over the next slot's bytes,
   allocate that next slot, and check it is entirely zero. It extends
   `a_slot_is_zeroed_when_handed_out_even_over_stale_bytes`, which covers
   only the 64-byte class.
2. **The crossing allocation collects first** (`gc.rs`). On a reset heap,
   allocate until the next allocation's `taken` would reach `next_gc`.
   - Every allocation before it leaves `alloc_since_gc` growing by its
     `taken`.
   - The crossing allocation leaves `alloc_since_gc == taken`: reset by
     the collection, then charged for its own slot.
   - It holds on every platform. Even where collection is unsupported,
     `collect()` resets the count.
3. **Small sizes are describable.** `heap_layout(SMALL_MAX)` is `Some`.

**Mutants**, each run against the named tests:
- the threshold test using `>` instead of `>=`;
- never collecting;
- allocating before collecting on the slow path;
- one `zero_slot` arm zeroing only its first 8 bytes;
- one small class skipped by `zero_slot`.

**Existing gates, unchanged:**
- every unit test in `gc.rs` and `gc/pages.rs`;
- the twelve `*_under_gc_stress` end-to-end tests, which collect on every
  allocation;
- `cargo test --locked --workspace`;
- `cargo clippy --locked --workspace --all-targets --all-features -- -D
  warnings`;
- `cargo fmt --all --check`.

The JIT tests run before any `*_build_standalone` test is trusted for a
runtime mutant, because `cargo test -p nova-cli` does not refresh the
runtime library a standalone build links ("(fast-join)").

---

## 7. Measurement

- **Predictions** are written before any measurement.
- **Per allocation:** a scratch Nova harness, never committed, allocates
  small records and short strings in a loop and reports ns per
  allocation. It is built by release toolchains from the base and the
  branch, with every sibling `NAME` and `NAME.exe` deleted first, and each
  binary's byte size and SHA-256 recorded. The two are alternated, three
  runs each, one fresh process per run. A gain is claimed only if the
  ranges are disjoint.
- **Server:** the ten-user json-api, 200 connections, `--warmup 5
  --duration 15`. Alternated, one fresh process per reading, three pairs,
  and three more if the ranges overlap. The Bun equivalence check runs
  first against the branch's binary.
- **Profile:** the json-api on the branch, sampled with "(reprofile-2)"'s
  method, to compare the four parts in section 2.1. The sampler patch is
  reverted afterwards, and a plain build's size and missing sampler
  string confirm it.
- **The record:** a dated amendment in `examples/05-json-api/BENCHMARK.md`
  and CHANGELOG bullets, checked by an independent agent before commit.

---

## 8. Records to amend

- **`gc.rs`'s module doc comment.** It says collection is "triggered from
  [`alloc`]"; that stays true and needs no change.
- **`gc.rs`'s `collect()` test hook doc comment** (`:331`). It names
  `maybe_collect`'s threshold trigger, which this change folds into
  `alloc`, so it is updated to say `alloc`'s.
- **`docs/adr/0020-size-class-page-heap.md`** says "Allocation takes the
  lowest free bit and zeroes that slot." That stays true, so it needs no
  note.
- **CHANGELOG `[Unreleased]`:** a Changed bullet and a Measured bullet.

For a large request the `Layout` check still runs before the threshold
test adds `taken` to `alloc_since_gc`. That keeps the addition free of
overflow, which an earlier review established by putting the check first.

---

## 9. Risks, and what was set aside

- **The saving is uncertain.** The profile charges 7.6–8.5 us per request
  to allocation proper. This change removes some of it, not all, and a
  few microseconds may not show on the server.
- **Inlining.** If the constant-size arms still compile to `memset`, the
  zeroing part saves nothing. The profile shows it.
- **Uninitialised leaf allocations** would save more of the zeroing, but
  add an internal API with a safety contract. Set aside.
- **A run-based slot search** in `alloc_slot` targets its 4.9–5.1%, but is
  the largest change to `pages.rs`. Set aside; this design's profile will
  show how much of `alloc_slot` is left.
- **The pre-existing 0xC0000005 flake** may recur during the suite. It is
  recorded, not treated as caused by this change without evidence.

---

## 10. Decisions taken in design review

- **Approach:** trim the fast path, chosen over adding uninitialised leaf
  allocations or restructuring `alloc_slot`.
- **Section 1:** one `HEAP` borrow per allocation; collect outside it only
  when over the threshold; small sizes skip `Layout`; the stress flag
  cached in the heap.
- **Section 2:** constant-size zeroing for the eight smallest classes,
  every slot still fully zeroed, dropped if the profile shows no gain.
- **Section 3:** characterization tests first, five mutants, the existing
  gates, per-allocation and server measurement, and a profile of the
  branch.
