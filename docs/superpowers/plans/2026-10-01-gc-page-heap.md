# Size-class page heap Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Move every collector object of 2 KiB or less into 64 KiB pages of fixed-size slots. Allocation, marking and sweeping then work on bitmaps instead of on a per-object index and per-object `dealloc`. Then measure what that does to `examples/05-json-api`.

**Architecture:**
- **`crates/nova-runtime/src/gc/pages.rs`** is a new submodule of `gc.rs`. It owns size classes, page descriptors with allocated, marked and scan bitmaps, a sorted page directory, per-class allocation cursors, and an empty-page reserve.
- **`gc.rs`** keeps the policy: the trigger, roots, the large-object path for objects over 2 KiB, and the mark loop over pages and large objects.
- **`task.rs`'s per-freed-object `forget_freed_state`** becomes one `prune_freed_states` call per collection.

**Tech Stack:** Rust 2021, MSRV 1.78, no new dependencies.

**Spec:** `docs/superpowers/specs/2026-10-01-gc-page-heap-design.md`

## Global Constraints

- **Toolchain:** edition 2021, `rust-version = "1.78"`. Use nothing stabilised after 1.78, so no inline `const { }` blocks and no `[const { ... }; N]`.
- **Dependencies:** none added.
- **The public surface stays put:**
  - `gc::alloc(size: usize, scan: bool) -> *mut u8` keeps its signature, its 8-byte floor and its `nova: panic:` abort on an undescribable size.
  - Every caller stays as it is, and codegen is untouched.
- **Constants**, verbatim from the spec:
  - `PAGE_BYTES = 65536`;
  - `SMALL_MAX = 2048`;
  - 24 classes: `16, 32, 48, 64, 80, 96, 112, 128, 160, 192, 224, 256, 320, 384, 448, 512, 640, 768, 896, 1024, 1280, 1536, 1792, 2048`;
  - `RESERVE_PAGES = 16`;
  - `ALIGN = 16`.
- **Pages** are allocated at `ALIGN` and are not aligned to their own size.
- **The debug line keeps its exact format:** `nova-gc: collection {N} freed {X} bytes, {Y} objects live ({Z} bytes)`.
- **Off Windows**, collection stays skipped (`stack_base` returns `None`).
- **The collector stays conservative, non-moving and thread-local.** Interior pointers keep their object alive. `scan = false` objects are leaves. `add_root`/`remove_root` keep their multiset semantics.
- **The invariant:** every address a collection frees is forgotten by `BY_STATE` before anything can be allocated again.
- **Gates:**
  - `cargo clippy --locked --workspace --all-targets --all-features -- -D warnings`;
  - `cargo fmt --all --check`;
  - `cargo test --locked --workspace`.
- **The ignored tests stay ignored.** Tests that call the real `collect()` remain `#[ignore]`d per `docs/adr/0010-conservative-scan-root-test-gating.md`.
- **Commits** go on branch `gc-page-heap`, and every commit message ends with `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`.
- **Measurement:**
  - one fresh process per reading;
  - ranges, never points;
  - a gain is claimed only on disjoint ranges;
  - every binary's byte size is recorded;
  - every sibling `NAME`/`NAME.exe` is deleted before `nova build -o NAME.exe`;
  - predictions are written before measuring.

## Rulings carried from spec to plan

Each of these is a deliberate difference from the spec's wording, stated here so the executor does not "fix" it back.

1. **Spec 4.3 says the heap's address range is maintained**, recomputed whenever a page or large object is added or released.
   - *Ruling:* compute it once at the start of each marking (`heap_range`).
   - *Why:* the range is only ever read during marking, so the observable behaviour is identical and there is less state to keep right.
2. **Spec 4.8 says each class's cursor resets to "the first page on its list".**
   - *Ruling:* `Vec::pop`, which takes the last.
   - *Why:* which page with space is used first is not observable to callers.
3. **Spec 8 asks for a new `task.rs` test that a live task's key survives a collection.**
   - *Ruling:* that test already exists as `a_reachable_futures_key_survives_a_collection_so_a_second_read_resolves`, and it stays. Only the "frees only non-state objects" test is new.
4. **Spec 7's list of records.** The per-object notification hook (`forget_freed_state`, called once per freed object) is cited as existing by:
   - ADRs 0009, 0012, 0016 and 0017;
   - `nova-spec/13-RUNTIME.md` §3.4;
   - `nova-spec/20-STDLIB.md`;
   - `std/sync/lib.nova`.

   *Ruling:* Task 4 amends all of them. The set-difference sweep found them with the extra token `per-object`.
5. **Spec 4.8 has `live_bytes` updated "as today"**, which today means decrementing it by the freed bytes.
   - *Ruling:* recompute it exactly at each collection, from the page sweep's live bytes plus the large objects' sizes.
   - *Why:* a recomputed figure cannot drift.

## Review Focus

Five conditions the spec implies that no test in its own list exercises. Each now has a test in the owning task.

1. **A heap holding only large objects (> 2 KiB)**, with no pages at all, still collects correctly. The range then comes from the large objects alone. Test: `a_large_only_heap_collects` (Task 3).
2. **A reserve page reused by a different class** is reformatted for the new class's slot size and count. Test: inside `empty_pages_past_the_reserve_go_back_and_the_reserve_is_reused_first` (Task 2).
3. **Words exactly at the range edges.** `lo - 1` and `hi` mark nothing, and `lo` marks the first slot. Test: `words_outside_the_heap_range_mark_nothing` (Task 3).
4. **A request smaller than its slot** (8 bytes in a 16-byte slot), reusing a slot whose previous object left a pointer in the tail, must not trace that stale pointer. Test: `a_reused_slot_does_not_trace_its_previous_objects_words` (Task 3).
5. **After a sweep that releases pages, no class's cursor names a released or reserve page.** Every class allocates inside a page the directory holds, with its own slot size. Test: `after_a_releasing_sweep_every_class_allocates_inside_a_page_it_holds` (Task 2).

---

### Task 1: Size classes, pages and slot allocation

**Files:**
- Create: `crates/nova-runtime/src/gc/pages.rs`
- Modify: `crates/nova-runtime/src/gc.rs`, adding the `mod pages;` declaration after the `use` lines, near line 47.

**Interfaces:**
- Consumes: `super::ALIGN` (`gc.rs`, `const ALIGN: usize = 16`).
- Produces, all `pub(super)`:
  - `const PAGE_BYTES: usize`, `SMALL_MAX: usize`, `NUM_CLASSES: usize`, `CLASS_SIZES: [usize; NUM_CLASSES]`, `RESERVE_PAGES: usize`;
  - `fn class_of(size: usize) -> usize`;
  - `enum Loc { Outside, Tail, Slot { desc: usize, index: usize } }`;
  - `struct Pages` with `const fn new() -> Pages`, `fn alloc_slot(&mut self, class: usize, scan: bool) -> usize`, `fn locate(&self, w: usize) -> Loc`, `fn slot_scan(&self, addr: usize) -> Option<bool>` and `fn bounds(&self) -> Option<(usize, usize)>`;
  - test-only: `fn page_count(&self) -> usize`, `fn live_slots(&self) -> usize`, `fn page_of(&self, addr: usize) -> Option<(usize, usize, usize)>` returning `(base, slot, nslots)`, `fn directory_is_sorted(&self) -> bool` and `fn free_all(&mut self)`.

- [ ] **Step 1: Declare the module, and write the failing tests**

In `crates/nova-runtime/src/gc.rs`, directly after `use std::sync::OnceLock;`, add:

```rust

// Wired into the collector by Task 3 of
// docs/superpowers/plans/2026-10-01-gc-page-heap.md; until then only its own
// tests use it.
#[allow(dead_code)]
mod pages;
```

Create `crates/nova-runtime/src/gc/pages.rs` containing only the test module below, so the tests fail to compile against a module that has no items yet:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    /// The table's class for every size is the first class at least that big.
    #[test]
    fn class_of_is_the_smallest_class_that_fits() {
        for size in 1..=SMALL_MAX {
            let want = CLASS_SIZES.iter().position(|&c| c >= size).unwrap();
            assert_eq!(class_of(size), want, "size {size}");
        }
        assert_eq!(CLASS_SIZES[class_of(16)], 16);
        assert_eq!(CLASS_SIZES[class_of(17)], 32);
        assert_eq!(CLASS_SIZES[class_of(128)], 128);
        assert_eq!(CLASS_SIZES[class_of(129)], 160);
        assert_eq!(CLASS_SIZES[class_of(2048)], 2048);
    }

    #[test]
    fn class_sizes_ascend_in_multiples_of_sixteen_up_to_small_max() {
        assert!(CLASS_SIZES.windows(2).all(|p| p[0] < p[1]));
        assert!(CLASS_SIZES.iter().all(|&c| c % 16 == 0));
        assert_eq!(CLASS_SIZES[NUM_CLASSES - 1], SMALL_MAX);
        // The smallest class has the most slots, and its bitmap must hold them.
        assert!(PAGE_BYTES / CLASS_SIZES[0] <= 64 * BITMAP_WORDS);
    }

    #[test]
    fn slots_come_lowest_first_and_record_their_scan_flag() {
        let mut p = Pages::new();
        let a = p.alloc_slot(class_of(16), true);
        let b = p.alloc_slot(class_of(16), false);
        assert_eq!(b, a + 16, "the lowest free slot comes next");
        assert_eq!(p.slot_scan(a), Some(true));
        assert_eq!(p.slot_scan(b), Some(false));
        assert_eq!(p.slot_scan(a + 1), None, "not a slot's start");
        assert_eq!(p.slot_scan(b + 16), None, "a free slot");
        p.free_all();
    }

    #[test]
    fn a_slot_is_zeroed_when_handed_out_even_over_stale_bytes() {
        let mut p = Pages::new();
        let a = p.alloc_slot(class_of(64), true);
        // The next slot is free page memory this test may scribble on.
        unsafe { std::ptr::write_bytes((a + 64) as *mut u8, 0xAB, 64) };
        let b = p.alloc_slot(class_of(64), true);
        assert_eq!(b, a + 64);
        let bytes = unsafe { std::slice::from_raw_parts(b as *const u8, 64) };
        assert!(
            bytes.iter().all(|&x| x == 0),
            "a slot was handed out still holding stale bytes"
        );
        p.free_all();
    }

    #[test]
    fn a_page_holds_whole_slots_and_its_tail_is_not_a_slot() {
        let mut p = Pages::new();
        let a = p.alloc_slot(class_of(48), true);
        let (base, slot, nslots) = p.page_of(a).unwrap();
        assert_eq!((slot, nslots), (48, 1365), "65536 / 48 = 1365, 16 bytes over");
        let tail = base + nslots * slot;
        assert!(matches!(p.locate(tail - 1), Loc::Slot { index: 1364, .. }));
        assert_eq!(p.locate(tail), Loc::Tail);
        assert_eq!(p.locate(base + PAGE_BYTES - 1), Loc::Tail);
        assert_eq!(p.locate(base + PAGE_BYTES), Loc::Outside);
        assert_eq!(p.locate(base - 1), Loc::Outside);
        p.free_all();
    }

    #[test]
    fn filling_a_page_adds_another_and_the_directory_stays_sorted() {
        let mut p = Pages::new();
        let c = class_of(16);
        let n = PAGE_BYTES / 16;
        let addrs: Vec<usize> = (0..=n).map(|_| p.alloc_slot(c, false)).collect();
        assert_eq!(p.page_count(), 2);
        assert!(p.directory_is_sorted());
        let mut distinct = addrs.clone();
        distinct.sort_unstable();
        distinct.dedup();
        assert_eq!(distinct.len(), n + 1, "every slot handed out once");
        assert_eq!(p.live_slots(), n + 1);
        p.free_all();
    }

    #[test]
    fn bounds_span_every_page() {
        let mut p = Pages::new();
        assert_eq!(p.bounds(), None);
        let addrs: Vec<usize> = (0..NUM_CLASSES).map(|c| p.alloc_slot(c, true)).collect();
        assert_eq!(p.page_count(), NUM_CLASSES);
        let (lo, hi) = p.bounds().unwrap();
        for a in addrs {
            let (base, _, _) = p.page_of(a).unwrap();
            assert!(lo <= base && base + PAGE_BYTES <= hi);
        }
        assert!(p.directory_is_sorted());
        p.free_all();
    }
}
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test --locked -p nova-runtime gc::pages 2>&1 | tail -20`
Expected: FAIL to compile, with `cannot find function `class_of`` and `cannot find type `Pages``, among others.

- [ ] **Step 3: Write the implementation above the test module**

Put this at the top of `crates/nova-runtime/src/gc/pages.rs`, above `#[cfg(test)] mod tests`:

```rust
//! Size-class pages: where the collector keeps every object of
//! [`SMALL_MAX`] bytes or less.
//!
//! A page is [`PAGE_BYTES`] bytes from the system allocator, holding slots of
//! one size class and nothing else. What the collector records about a slot
//! lives out of band in the page's descriptor: an allocated, a marked and a
//! scan bitmap, one bit per slot. `gc.rs` owns the policy -- when to collect,
//! the roots, objects too big for a page -- and this module owns slots, pages
//! and the directory that maps an address to its page. The decision is
//! `docs/adr/0020-size-class-page-heap.md`.

use std::alloc::{alloc, dealloc, handle_alloc_error, Layout};

/// Bytes in one page.
pub(super) const PAGE_BYTES: usize = 1 << 16;

/// The largest object a page holds; anything bigger takes `gc.rs`'s large
/// path.
pub(super) const SMALL_MAX: usize = 2048;

/// How many size classes there are.
pub(super) const NUM_CLASSES: usize = 24;

/// The slot size of each class, ascending.
pub(super) const CLASS_SIZES: [usize; NUM_CLASSES] = [
    16, 32, 48, 64, 80, 96, 112, 128, // steps of 16
    160, 192, 224, 256, // steps of 32
    320, 384, 448, 512, // steps of 64
    640, 768, 896, 1024, // steps of 128
    1280, 1536, 1792, 2048, // steps of 256
];

/// Empty pages a sweep keeps for reuse instead of returning them: 1 MiB, the
/// same as `gc.rs`'s `INITIAL_THRESHOLD`.
pub(super) const RESERVE_PAGES: usize = 16;

/// Words in each bitmap: enough for the most slots any page has, which is
/// `PAGE_BYTES / 16`, at the smallest class.
const BITMAP_WORDS: usize = PAGE_BYTES / 16 / 64;

/// `CLASS_OF[(size - 1) / 16]` is the class of a `size`-byte request.
const CLASS_OF: [u8; SMALL_MAX / 16] = class_table();

const fn class_table() -> [u8; SMALL_MAX / 16] {
    let mut table = [0u8; SMALL_MAX / 16];
    let mut i = 0;
    let mut class = 0;
    while i < table.len() {
        // Entry `i` covers sizes `16 * i + 1 ..= 16 * (i + 1)`. Every class is
        // a multiple of 16, so the smallest class that fits the top of that
        // range fits all of it.
        while CLASS_SIZES[class] < 16 * (i + 1) {
            class += 1;
        }
        table[i] = class as u8;
        i += 1;
    }
    table
}

/// The class of a `size`-byte request, for `1 <= size <= SMALL_MAX`.
pub(super) fn class_of(size: usize) -> usize {
    debug_assert!((1..=SMALL_MAX).contains(&size), "class_of({size})");
    CLASS_OF[(size - 1) / 16] as usize
}

fn page_layout() -> Layout {
    Layout::from_size_align(PAGE_BYTES, super::ALIGN)
        .expect("a page's size and alignment are describable")
}

/// One page's out-of-band record.
struct Page {
    base: usize,
    class: usize,
    slot: usize,
    nslots: usize,
    /// In the empty-page reserve: in the directory, but owned by no class.
    reserved: bool,
    alloc: [u64; BITMAP_WORDS],
    mark: [u64; BITMAP_WORDS],
    scan: [u64; BITMAP_WORDS],
}

impl Page {
    fn new(base: usize, class: usize) -> Page {
        let slot = CLASS_SIZES[class];
        Page {
            base,
            class,
            slot,
            nslots: PAGE_BYTES / slot,
            reserved: false,
            alloc: [0; BITMAP_WORDS],
            mark: [0; BITMAP_WORDS],
            scan: [0; BITMAP_WORDS],
        }
    }

    /// Bitmap words in use, at one bit per slot.
    fn words(&self) -> usize {
        self.nslots.div_ceil(64)
    }

    /// The bits of word `w` that name real slots.
    fn valid(&self, w: usize) -> u64 {
        let n = self.nslots - 64 * w;
        if n >= 64 {
            u64::MAX
        } else {
            (1u64 << n) - 1
        }
    }

    /// One past the end of the last whole slot.
    fn end(&self) -> usize {
        self.base + self.nslots * self.slot
    }
}

/// Where an address falls, as far as pages are concerned.
#[derive(Debug, PartialEq, Eq)]
pub(super) enum Loc {
    /// In no page's memory.
    Outside,
    /// In a page's memory, past its last whole slot.
    Tail,
    /// In slot `index` of the page whose descriptor is `desc`.
    Slot { desc: usize, index: usize },
}

/// A const empty list, so `[NO_PAGES; NUM_CLASSES]` can build an array of
/// `Vec`s in a `const fn` on Rust 1.78.
const NO_PAGES: Vec<usize> = Vec::new();

/// Every page, and what allocation needs to find a free slot fast.
pub(super) struct Pages {
    /// Descriptors. An index is stable for as long as its page exists.
    descs: Vec<Page>,
    /// Descriptor indices naming no page, for reuse.
    spare: Vec<usize>,
    /// `(base, descriptor index)` for every page, sorted by base.
    dir: Vec<(usize, usize)>,
    /// Each class's page being allocated from, if any.
    current: [Option<usize>; NUM_CLASSES],
    /// Each class's first bitmap word that may hold a free bit.
    hint: [usize; NUM_CLASSES],
    /// Each class's other pages with a free slot, as of the last sweep.
    with_space: [Vec<usize>; NUM_CLASSES],
    /// Empty pages kept for reuse, at most `RESERVE_PAGES`.
    reserve: Vec<usize>,
}

impl Pages {
    pub(super) const fn new() -> Pages {
        Pages {
            descs: Vec::new(),
            spare: Vec::new(),
            dir: Vec::new(),
            current: [None; NUM_CLASSES],
            hint: [0; NUM_CLASSES],
            with_space: [NO_PAGES; NUM_CLASSES],
            reserve: Vec::new(),
        }
    }

    /// Hand out a zeroed slot of `class`, taking a reserve page or a new one
    /// if every page of the class is full. Returns the slot's address.
    pub(super) fn alloc_slot(&mut self, class: usize, scan: bool) -> usize {
        loop {
            if let Some(desc) = self.current[class] {
                if let Some(addr) = self.take(desc, class, scan) {
                    return addr;
                }
                self.current[class] = self.with_space[class].pop();
            } else {
                self.current[class] = Some(self.page_for(class));
            }
            self.hint[class] = 0;
        }
    }

    /// The lowest free slot of page `desc` at or after `class`'s hint.
    fn take(&mut self, desc: usize, class: usize, scan: bool) -> Option<usize> {
        let page = &mut self.descs[desc];
        for w in self.hint[class]..page.words() {
            let free = !page.alloc[w] & page.valid(w);
            if free == 0 {
                continue;
            }
            let bit = free.trailing_zeros() as usize;
            let mask = 1u64 << bit;
            page.alloc[w] |= mask;
            if scan {
                page.scan[w] |= mask;
            } else {
                page.scan[w] &= !mask;
            }
            self.hint[class] = w;
            let addr = page.base + (64 * w + bit) * page.slot;
            // SAFETY: slot `64 * w + bit` is below `nslots` (masked by
            // `valid`), so all `slot` bytes from `addr` lie inside this page's
            // own allocation. Zeroed so unwritten fields read as null and a
            // previous object's words are never traced again.
            unsafe { std::ptr::write_bytes(addr as *mut u8, 0, page.slot) };
            return Some(addr);
        }
        None
    }

    /// A page for `class`: from the reserve if it holds one, else new.
    fn page_for(&mut self, class: usize) -> usize {
        if let Some(desc) = self.reserve.pop() {
            let base = self.descs[desc].base;
            self.descs[desc] = Page::new(base, class);
            return desc;
        }
        let layout = page_layout();
        // SAFETY: `layout` has a nonzero size. The memory is not zeroed here:
        // `take` zeroes every slot as it hands it out.
        let p = unsafe { alloc(layout) };
        if p.is_null() {
            handle_alloc_error(layout);
        }
        let base = p as usize;
        let page = Page::new(base, class);
        let desc = match self.spare.pop() {
            Some(desc) => {
                self.descs[desc] = page;
                desc
            }
            None => {
                self.descs.push(page);
                self.descs.len() - 1
            }
        };
        let at = self.dir.partition_point(|&(b, _)| b < base);
        self.dir.insert(at, (base, desc));
        desc
    }

    /// Which page and slot `w` falls in, if any.
    pub(super) fn locate(&self, w: usize) -> Loc {
        let i = self.dir.partition_point(|&(b, _)| b <= w);
        if i == 0 {
            return Loc::Outside;
        }
        let (base, desc) = self.dir[i - 1];
        if w - base >= PAGE_BYTES {
            return Loc::Outside;
        }
        let page = &self.descs[desc];
        if w >= page.end() {
            return Loc::Tail;
        }
        Loc::Slot {
            desc,
            index: (w - base) / page.slot,
        }
    }

    /// `Some(scan)` if `addr` is the start of an allocated slot.
    pub(super) fn slot_scan(&self, addr: usize) -> Option<bool> {
        let Loc::Slot { desc, index } = self.locate(addr) else {
            return None;
        };
        let page = &self.descs[desc];
        let mask = 1u64 << (index % 64);
        let start = page.base + index * page.slot;
        (addr == start && page.alloc[index / 64] & mask != 0)
            .then_some(page.scan[index / 64] & mask != 0)
    }

    /// The lowest page base and the highest page end, if there is a page.
    pub(super) fn bounds(&self) -> Option<(usize, usize)> {
        Some((self.dir.first()?.0, self.dir.last()?.0 + PAGE_BYTES))
    }

    /// Test-only: how many pages exist, reserve included.
    #[cfg(test)]
    pub(super) fn page_count(&self) -> usize {
        self.dir.len()
    }

    /// Test-only: how many slots are allocated across every page.
    #[cfg(test)]
    pub(super) fn live_slots(&self) -> usize {
        self.dir
            .iter()
            .map(|&(_, d)| {
                self.descs[d]
                    .alloc
                    .iter()
                    .map(|w| w.count_ones() as usize)
                    .sum::<usize>()
            })
            .sum()
    }

    /// Test-only: `(base, slot size, slot count)` of the page holding slot
    /// address `addr`.
    #[cfg(test)]
    pub(super) fn page_of(&self, addr: usize) -> Option<(usize, usize, usize)> {
        let Loc::Slot { desc, .. } = self.locate(addr) else {
            return None;
        };
        let page = &self.descs[desc];
        Some((page.base, page.slot, page.nslots))
    }

    /// Test-only: the directory is in base order with no overlap, and every
    /// descriptor it names is in use and agrees about its base.
    #[cfg(test)]
    pub(super) fn directory_is_sorted(&self) -> bool {
        self.dir.windows(2).all(|p| p[0].0 + PAGE_BYTES <= p[1].0)
            && self
                .dir
                .iter()
                .all(|&(b, d)| self.descs[d].base == b && !self.spare.contains(&d))
    }

    /// Test-only: return every page to the system and start empty.
    #[cfg(test)]
    pub(super) fn free_all(&mut self) {
        for &(base, _) in &self.dir {
            // SAFETY: every directory base came from `alloc(page_layout())`
            // in `page_for`, and leaves the directory only when freed.
            unsafe { dealloc(base as *mut u8, page_layout()) };
        }
        *self = Pages::new();
    }
}
```

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo test --locked -p nova-runtime gc::pages 2>&1 | tail -15`
Expected: `test result: ok. 7 passed; 0 failed`, the 7 being the tests in `gc::pages::tests`.

- [ ] **Step 5: Lint and format**

Run: `cargo fmt --all && cargo clippy --locked -p nova-runtime --all-targets --all-features -- -D warnings 2>&1 | tail -5`
Expected: `Finished` with no warnings.
- If clippy flags `needless_range_loop` in `take`, rewrite the loop as `for (w, word) in page.alloc.iter_mut().enumerate().take(words).skip(start)` and keep `page.scan[w]` indexed. Ledger it as a ruling.
- Nothing else is expected to be flagged.

- [ ] **Step 6: Commit**

```bash
git add crates/nova-runtime/src/gc.rs crates/nova-runtime/src/gc/pages.rs
git commit -m "feat(gc): size classes, pages and slot allocation for a page heap

Not wired into the collector yet. Every object of 2 KiB or less will live
in a 64 KiB page of one size class, tracked by out-of-band allocated and
scan bitmaps. Slots are handed out lowest first and zeroed.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 2: Marking and sweeping pages, the reserve, and release

**Files:**
- Modify: `crates/nova-runtime/src/gc/pages.rs`

**Interfaces:**
- Consumes: Task 1's `Pages`, `Page`, `Loc`, `locate`, `page_layout` and `RESERVE_PAGES`.
- Produces, all `pub(super)`:
  - `enum Hit { Outside, Done, Push(usize, usize) }`;
  - `struct Swept { freed_bytes: usize, live_slots: usize, live_bytes: usize }`, with all fields `pub(super)`;
  - `fn try_mark(&mut self, w: usize) -> Hit`;
  - `fn sweep(&mut self) -> Swept`;
  - test-only: `fn reserve_count(&self) -> usize`.

- [ ] **Step 1: Write the failing tests**

Append inside `mod tests` in `pages.rs`:

```rust
    #[test]
    fn try_mark_marks_an_allocated_slot_once_and_pushes_only_scanned_ones() {
        let mut p = Pages::new();
        let traced = p.alloc_slot(class_of(32), true);
        let leaf = p.alloc_slot(class_of(32), false);
        assert_eq!(
            p.try_mark(traced + 5),
            Hit::Push(traced, 32),
            "an interior word names its slot, and the whole slot is traced"
        );
        assert_eq!(p.try_mark(traced), Hit::Done, "already marked");
        assert_eq!(p.try_mark(leaf), Hit::Done, "a leaf is marked, not traced");
        let s = p.sweep();
        assert_eq!(s.live_slots, 2, "both were marked");
        p.free_all();
    }

    #[test]
    fn try_mark_passes_over_free_slots_tails_and_outside_words() {
        let mut p = Pages::new();
        let a = p.alloc_slot(class_of(48), true);
        let (base, slot, nslots) = p.page_of(a).unwrap();
        assert_eq!(p.try_mark(a + slot), Hit::Done, "a free slot");
        assert_eq!(p.try_mark(base + nslots * slot), Hit::Done, "the page tail");
        assert_eq!(p.try_mark(base - 1), Hit::Outside);
        let s = p.sweep();
        assert_eq!(s.live_slots, 0, "none of those marked the allocated slot");
        p.free_all();
    }

    #[test]
    fn sweep_frees_the_unmarked_keeps_the_marked_and_clears_every_mark() {
        let mut p = Pages::new();
        let c = class_of(64);
        let keep = p.alloc_slot(c, true);
        let gone = p.alloc_slot(c, true);
        assert_eq!(p.try_mark(keep), Hit::Push(keep, 64));
        let s = p.sweep();
        assert_eq!((s.freed_bytes, s.live_slots, s.live_bytes), (64, 1, 64));
        assert_eq!(p.slot_scan(keep), Some(true));
        assert_eq!(p.slot_scan(gone), None);
        // The mark was cleared, so a second sweep with nothing marked frees
        // `keep` too.
        let s = p.sweep();
        assert_eq!((s.freed_bytes, s.live_slots), (64, 0));
        p.free_all();
    }

    #[test]
    fn a_freed_slot_is_handed_out_again_zeroed() {
        let mut p = Pages::new();
        let c = class_of(32);
        let keep = p.alloc_slot(c, true);
        let freed = p.alloc_slot(c, true);
        unsafe { std::ptr::write_bytes(freed as *mut u8, 0xCD, 32) };
        p.try_mark(keep);
        p.sweep();
        let again = p.alloc_slot(c, true);
        assert_eq!(again, freed, "the lowest free slot is the one just freed");
        let bytes = unsafe { std::slice::from_raw_parts(again as *const u8, 32) };
        assert!(bytes.iter().all(|&x| x == 0));
        p.free_all();
    }

    #[test]
    fn a_reused_slot_takes_the_scan_flag_it_is_given_now() {
        let mut p = Pages::new();
        let c = class_of(16);
        let keep = p.alloc_slot(c, true);
        let s1 = p.alloc_slot(c, true);
        let s2 = p.alloc_slot(c, false);
        p.try_mark(keep);
        p.sweep();
        let r1 = p.alloc_slot(c, false);
        let r2 = p.alloc_slot(c, true);
        assert_eq!((r1, r2), (s1, s2));
        assert_eq!(p.slot_scan(r1), Some(false), "scanned before, a leaf now");
        assert_eq!(p.slot_scan(r2), Some(true), "a leaf before, scanned now");
        p.free_all();
    }

    #[test]
    fn empty_pages_past_the_reserve_go_back_and_the_reserve_is_reused_first() {
        let mut p = Pages::new();
        let c = class_of(16);
        let per_page = PAGE_BYTES / 16;
        for _ in 0..per_page * (RESERVE_PAGES + 4) {
            p.alloc_slot(c, false);
        }
        assert_eq!(p.page_count(), RESERVE_PAGES + 4);
        let s = p.sweep();
        assert_eq!(s.live_slots, 0);
        assert_eq!(p.page_count(), RESERVE_PAGES, "pages past the reserve were released");
        assert_eq!(p.reserve_count(), RESERVE_PAGES);
        assert!(p.directory_is_sorted());
        // Review Focus 2: a reserve page reused by another class is sized for it.
        let big = p.alloc_slot(class_of(2048), true);
        assert_eq!(p.page_count(), RESERVE_PAGES, "a reserve page was reused, none added");
        assert_eq!(p.reserve_count(), RESERVE_PAGES - 1);
        let (_, slot, nslots) = p.page_of(big).unwrap();
        assert_eq!((slot, nslots), (2048, 32));
        assert_eq!(p.alloc_slot(class_of(2048), true), big + 2048);
        p.free_all();
    }

    /// Review Focus 5. Three classes get twelve pages each, 36 in all:
    /// - class 16 keeps a slot in every page, so none of its pages empties;
    /// - class 224 (292 slots a page) keeps slots 0, 1000, 2000 and 3000,
    ///   which sit in pages 0, 3, 6 and 10, so 8 of its pages empty;
    /// - class 2048 keeps slot 0 only, so 11 of its pages empty.
    ///
    /// That is 19 empty pages. The reserve takes 16 and 3 are released.
    #[test]
    fn after_a_releasing_sweep_every_class_allocates_inside_a_page_it_holds() {
        let mut p = Pages::new();
        let classes = [class_of(16), class_of(200), class_of(2048)];
        let mut kept = Vec::new();
        for &c in &classes {
            let per_page = PAGE_BYTES / CLASS_SIZES[c];
            for i in 0..per_page * 12 {
                let a = p.alloc_slot(c, true);
                if i % 1000 == 0 {
                    kept.push(a);
                }
            }
        }
        assert_eq!(p.page_count(), 36);
        for &a in &kept {
            p.try_mark(a);
        }
        p.sweep();
        assert_eq!(p.page_count(), 33);
        assert_eq!(p.reserve_count(), RESERVE_PAGES);
        assert!(p.directory_is_sorted());
        for &c in &classes {
            for _ in 0..3 {
                let a = p.alloc_slot(c, true);
                assert!(
                    matches!(p.locate(a), Loc::Slot { .. }),
                    "class {c} handed out an address in no page the directory holds"
                );
                assert_eq!(p.page_of(a).unwrap().1, CLASS_SIZES[c]);
            }
        }
        assert!(p.directory_is_sorted());
        p.free_all();
    }
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test --locked -p nova-runtime gc::pages 2>&1 | tail -20`
Expected: FAIL to compile, with `no method named `try_mark``, `cannot find type `Hit`` and `no method named `reserve_count``, among others.

- [ ] **Step 3: Write the implementation**

Add after `enum Loc` in `pages.rs`:

```rust
/// What [`Pages::try_mark`] did with a candidate word.
#[derive(Debug, PartialEq, Eq)]
pub(super) enum Hit {
    /// The word is in no page, so the caller should try the large objects.
    Outside,
    /// Nothing left to do. The word fell in a page tail, a free slot or an
    /// already-marked slot, or it newly marked a leaf.
    Done,
    /// Newly marked and scanned: trace `len` bytes from `addr`.
    Push(usize, usize),
}

/// What a sweep found across every page.
pub(super) struct Swept {
    pub(super) freed_bytes: usize,
    pub(super) live_slots: usize,
    pub(super) live_bytes: usize,
}
```

Add to `impl Pages`, after `bounds`:

```rust
    /// Mark the allocated slot `w` points into, if it is not marked yet.
    pub(super) fn try_mark(&mut self, w: usize) -> Hit {
        let (desc, index) = match self.locate(w) {
            Loc::Outside => return Hit::Outside,
            Loc::Tail => return Hit::Done,
            Loc::Slot { desc, index } => (desc, index),
        };
        let page = &mut self.descs[desc];
        let (word, mask) = (index / 64, 1u64 << (index % 64));
        if page.alloc[word] & mask == 0 || page.mark[word] & mask != 0 {
            return Hit::Done;
        }
        page.mark[word] |= mask;
        if page.scan[word] & mask == 0 {
            return Hit::Done;
        }
        Hit::Push(page.base + index * page.slot, page.slot)
    }

    /// Free every allocated slot that is not marked, clear every mark, put
    /// empty pages in the reserve or back to the system, and point each
    /// class's cursor at a page with space.
    pub(super) fn sweep(&mut self) -> Swept {
        let mut out = Swept {
            freed_bytes: 0,
            live_slots: 0,
            live_bytes: 0,
        };
        let mut empty = Vec::new();
        for list in &mut self.with_space {
            list.clear();
        }
        for &(_, desc) in &self.dir {
            let page = &mut self.descs[desc];
            if page.reserved {
                continue;
            }
            let words = page.words();
            let (mut live, mut freed) = (0usize, 0usize);
            for (a, m) in page.alloc[..words]
                .iter_mut()
                .zip(&mut page.mark[..words])
            {
                // A mark is only ever set on an allocated slot, so the
                // survivors are exactly the marks.
                freed += (*a & !*m).count_ones() as usize;
                *a = *m;
                live += a.count_ones() as usize;
                *m = 0;
            }
            out.freed_bytes += freed * page.slot;
            out.live_slots += live;
            out.live_bytes += live * page.slot;
            if live == 0 {
                empty.push(desc);
            } else if live < page.nslots {
                self.with_space[page.class].push(desc);
            }
        }
        for desc in empty {
            if self.reserve.len() < RESERVE_PAGES {
                self.descs[desc].reserved = true;
                self.reserve.push(desc);
            } else {
                self.release(desc);
            }
        }
        for (cur, list) in self.current.iter_mut().zip(&mut self.with_space) {
            *cur = list.pop();
        }
        self.hint = [0; NUM_CLASSES];
        out
    }

    /// Return the empty page `desc` to the system and forget it.
    fn release(&mut self, desc: usize) {
        let base = self.descs[desc].base;
        let at = self.dir.partition_point(|&(b, _)| b < base);
        debug_assert_eq!(self.dir.get(at), Some(&(base, desc)));
        self.dir.remove(at);
        // SAFETY: `base` came from `alloc(page_layout())` in `page_for`, and
        // is freed only here, once, as it leaves the directory.
        unsafe { dealloc(base as *mut u8, page_layout()) };
        self.spare.push(desc);
    }

    /// Test-only: how many empty pages the reserve holds.
    #[cfg(test)]
    pub(super) fn reserve_count(&self) -> usize {
        self.reserve.len()
    }
```

In `page_for`, the reserve branch already builds a fresh `Page::new(base, class)`, whose `reserved` is `false`. Leave that as it is.

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo test --locked -p nova-runtime gc::pages 2>&1 | tail -15`
Expected: `test result: ok. 14 passed; 0 failed`.

- [ ] **Step 5: Lint and format**

Run: `cargo fmt --all && cargo clippy --locked -p nova-runtime --all-targets --all-features -- -D warnings 2>&1 | tail -5`
Expected: `Finished` with no warnings.

- [ ] **Step 6: Commit**

```bash
git add crates/nova-runtime/src/gc/pages.rs
git commit -m "feat(gc): mark and sweep pages by bitmap, with a 16-page reserve

A sweep frees unmarked slots 64 at a time and clears every mark. It keeps
up to 16 empty pages for reuse and returns the rest to the system. Not
wired into the collector yet.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 3: Put the collector on the page heap, and prune the state map once per collection

**Files:**
- Modify: `crates/nova-runtime/src/gc.rs`:
  - the module doc (lines 1-41);
  - imports;
  - `mod pages;`;
  - `Obj`, `Heap` and `Heap::new` (lines 68-103);
  - `alloc` (158-208);
  - `object_info` (240-262);
  - `collect_with_roots` and `mark_word` (395-499);
  - the test module's `reset` and `count` (548-583), plus new tests.
- Modify: `crates/nova-runtime/src/task.rs`:
  - doc comments at 224 and 1411;
  - `forget_freed_state` (343-363), which becomes `prune_freed_states`;
  - one new test after `a_reachable_futures_key_survives_a_collection_so_a_second_read_resolves`.

**Interfaces:**
- Consumes: everything Tasks 1 and 2 produce.
- Produces:
  - `pub(crate) fn prune_freed_states(is_live: &dyn Fn(usize) -> bool)` in `task.rs`;
  - test-only in `gc.rs`: `fn heap_bounds() -> (usize, usize)`.
- Removes: `task::forget_freed_state`.

- [ ] **Step 1: Write the failing tests**

In `gc.rs`'s `mod tests`, after `leaf_objects_are_not_traced`, add:

```rust
    #[test]
    fn sizes_over_small_max_take_the_large_path() {
        reset();
        let small = alloc(pages::SMALL_MAX, true) as usize;
        let large = alloc(pages::SMALL_MAX + 1, true) as usize;
        HEAP.with(|h| {
            let h = h.borrow();
            assert_eq!(h.pages.slot_scan(small), Some(true));
            assert_eq!(h.pages.slot_scan(large), None);
            assert!(h
                .large
                .iter()
                .any(|o| o.addr == large && o.size == pages::SMALL_MAX + 1));
        });
        assert_eq!(object_info(small), Some((pages::SMALL_MAX, true)));
        assert_eq!(object_info(large), Some((pages::SMALL_MAX + 1, true)));
    }

    #[test]
    fn a_small_large_small_chain_survives_from_one_root() {
        reset();
        let tail = alloc(16, true) as usize;
        let mid = alloc(4096, true) as *mut usize;
        let head = alloc(16, true) as *mut usize;
        unsafe {
            *mid.add(100) = tail;
            *head = mid as usize;
        }
        let _unreachable_large = alloc(4096, true);
        collect_with_roots(&[head as usize]);
        assert_eq!(count(), 3, "head, mid and tail survive; the other large object does not");
        assert!(object_info(tail).is_some());
        assert!(object_info(mid as usize).is_some());
    }

    /// Review Focus 1: no pages at all, so the range is the large objects'.
    #[test]
    fn a_large_only_heap_collects() {
        reset();
        let keep = alloc(3000, false) as usize;
        let _gone = alloc(5000, true);
        collect_with_roots(&[keep + 1234]);
        assert_eq!(count(), 1);
        assert_eq!(object_info(keep), Some((3000, false)));
    }

    #[test]
    fn a_pointer_into_a_rounding_tail_keeps_its_object() {
        reset();
        let a = alloc(17, true) as usize; // a 32-byte slot
        collect_with_roots(&[a + 20]);
        assert_eq!(count(), 1);
    }

    /// Review Focus 3.
    #[test]
    fn words_outside_the_heap_range_mark_nothing() {
        reset();
        let a = alloc(16, true) as usize;
        let (lo, hi) = heap_bounds();
        assert_eq!(lo, a, "the only page's first slot starts the range");
        collect_with_roots(&[lo - 1, hi]);
        assert_eq!(count(), 0);
        let _b = alloc(16, true);
        collect_with_roots(&[heap_bounds().0]);
        assert_eq!(count(), 1, "the range's first byte is inside it");
    }

    /// Review Focus 4, and the zeroing guarantee seen from the collector: an
    /// 8-byte object in a 16-byte slot must not trace what the slot's
    /// previous occupant left in its second word.
    #[test]
    fn a_reused_slot_does_not_trace_its_previous_objects_words() {
        reset();
        let victim = alloc(16, true) as usize;
        let old = alloc(16, true) as *mut usize;
        unsafe { *old.add(1) = victim };
        collect_with_roots(&[victim]);
        assert_eq!(count(), 1, "`old` is freed, its stale word left behind");
        let new = alloc(8, true) as usize;
        assert_eq!(new, old as usize, "the freed slot is the lowest free one");
        collect_with_roots(&[new]);
        assert_eq!(
            count(),
            1,
            "the reused slot's zeroed tail kept the old pointer's target alive"
        );
        assert!(object_info(victim).is_none());
    }
```

In `task.rs`'s test module, after `a_reachable_futures_key_survives_a_collection_so_a_second_read_resolves`, add:

```rust
    /// Pruning removes the keys of freed states and nothing else. With every
    /// state the map names in the root set, a sweep frees only objects that
    /// are not task states, and the map comes out exactly as it went in.
    #[test]
    fn a_sweep_freeing_only_non_state_objects_removes_no_key() {
        let fut = make_future(poll_ready_now, 0);
        let state = state_of(fut);
        unsafe { nova_rt_task_spawn(fut) };
        unsafe { nova_rt_task_block_on(make_future(poll_ready_now, 0)) };
        unsafe { nova_rt_task_release(fut) };
        let keys: Vec<usize> = BY_STATE.with(|m| m.borrow().keys().copied().collect());
        assert!(keys.contains(&state), "spawn must have registered this state");
        for _ in 0..100 {
            gc::alloc(24, true);
        }
        let before = BY_STATE.with(|m| m.borrow().clone());

        gc::sweep_with_roots_for_test(&keys);

        assert_eq!(
            BY_STATE.with(|m| m.borrow().clone()),
            before,
            "a sweep that freed no state removed a key"
        );
    }
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test --locked -p nova-runtime 2>&1 | grep -E "^error|cannot find|no field" | head -10`
Expected: FAIL to compile, with `cannot find function `heap_bounds``, `no field `pages` on type` and `no field `large` on type`, among others.

- [ ] **Step 3: Rewrite the heap types and `alloc` in `gc.rs`**

Change the import line to:

```rust
use std::alloc::{alloc_zeroed, dealloc, handle_alloc_error, Layout};
use std::cell::RefCell;
#[cfg(test)]
use std::collections::BTreeMap;
use std::ffi::c_void;
use std::sync::OnceLock;

mod pages;
```

That removes Task 1's comment and the `#[allow(dead_code)]` above `mod pages;`.

Replace `struct Obj`, `struct Heap` and `impl Heap` (lines 68-103) with:

```rust
/// A large object's out-of-band record: one over [`pages::SMALL_MAX`] bytes,
/// which gets a system allocation of its own.
struct Obj {
    /// Address returned to the mutator.
    addr: usize,
    /// Allocation size in bytes.
    size: usize,
    /// Whether to trace this object's words for further pointers.
    scan: bool,
    /// Set during the mark phase.
    marked: bool,
}

struct Heap {
    /// Every object of [`pages::SMALL_MAX`] bytes or less.
    pages: pages::Pages,
    /// Every larger object. Sorted by address from the start of each
    /// collection until it ends; allocation appends in between.
    large: Vec<Obj>,
    /// The mark phase's `(addr, len)` ranges still to trace. Kept between
    /// collections so each one does not allocate it afresh.
    work: Vec<(usize, usize)>,
    alloc_since_gc: usize,
    next_gc: usize,
    live_bytes: usize,
    /// Thread stack base (highest address); `0` = not captured, `usize::MAX` =
    /// this platform is unsupported (collection disabled).
    base: usize,
    collections: u64,
    freed_bytes: u64,
    /// Test builds only: the size each object was requested at, which
    /// rounding to a size class would otherwise hide from [`object_info`].
    /// Written at every allocation; read only for a live object's start.
    #[cfg(test)]
    requested: BTreeMap<usize, usize>,
}

impl Heap {
    const fn new() -> Self {
        Heap {
            pages: pages::Pages::new(),
            large: Vec::new(),
            work: Vec::new(),
            alloc_since_gc: 0,
            next_gc: INITIAL_THRESHOLD,
            live_bytes: 0,
            base: 0,
            collections: 0,
            freed_bytes: 0,
            #[cfg(test)]
            requested: BTreeMap::new(),
        }
    }
}
```

In `alloc`, replace everything from `maybe_collect(size);` to the end of the function with:

```rust
    // A small object takes a whole slot of its class, so a slot's full size
    // is what it costs the collection trigger and the live count.
    let class = (size <= pages::SMALL_MAX).then(|| pages::class_of(size));
    let taken = class.map_or(size, |c| pages::CLASS_SIZES[c]);
    maybe_collect(taken);
    HEAP.with(|h| {
        let mut h = h.borrow_mut();
        let p = match class {
            // Zeroed by `alloc_slot`.
            Some(c) => h.pages.alloc_slot(c, scan) as *mut u8,
            None => {
                // Zeroed so unwritten slots (e.g. skipped unit fields) read as
                // null and are never mistaken for pointers.
                let p = unsafe { alloc_zeroed(layout) };
                if p.is_null() {
                    handle_alloc_error(layout);
                }
                h.large.push(Obj {
                    addr: p as usize,
                    size,
                    scan,
                    marked: false,
                });
                p
            }
        };
        h.alloc_since_gc += taken;
        h.live_bytes += taken;
        #[cfg(test)]
        h.requested.insert(p as usize, size);
        p
    })
}
```

- [ ] **Step 4: Rewrite `object_info`**

Replace its body, keeping its doc comment except the last sentence, which becomes "This reaches into the collector's own records so a caller can assert the exact size and scan flag `alloc` was given, not just what got written.":

```rust
#[cfg(test)]
pub(crate) fn object_info(addr: usize) -> Option<(usize, bool)> {
    HEAP.with(|h| {
        let h = h.borrow();
        let scan = h.pages.slot_scan(addr).or_else(|| {
            h.large
                .iter()
                .find(|o| o.addr == addr)
                .map(|o| o.scan)
        })?;
        let size = *h
            .requested
            .get(&addr)
            .expect("alloc records every object's requested size in test builds");
        Some((size, scan))
    })
}
```

- [ ] **Step 5: Rewrite `collect_with_roots` and `mark_word`, and add `heap_range`**

Replace `fn collect_with_roots` and `fn mark_word` in full with:

```rust
fn collect_with_roots(roots: &[usize]) {
    HEAP.with(|h| {
        let mut guard = h.borrow_mut();
        let h = &mut *guard;
        // The previous sweep left every page mark and every large record
        // unmarked. Sort the large records for the range lookup; the large
        // sweep's `retain` keeps that order, and the prune below relies on it.
        h.large.sort_unstable_by_key(|o| o.addr);
        let (lo, hi) = heap_range(&h.pages, &h.large);

        let mut work = std::mem::take(&mut h.work);
        for &w in roots {
            mark_word(w, lo, hi, &mut h.pages, &mut h.large, &mut work);
        }
        while let Some((addr, len)) = work.pop() {
            let mut p = addr;
            let end = addr + len;
            while p + 8 <= end {
                // SAFETY: [addr, end) is a live object this collector owns: a
                // slot's full size, or a large object's.
                let w = unsafe { *(p as *const usize) };
                mark_word(w, lo, hi, &mut h.pages, &mut h.large, &mut work);
                p += 8;
            }
        }
        h.work = work;

        // Sweep: free unmarked slots and unmarked large objects.
        let swept = h.pages.sweep();
        let mut freed = swept.freed_bytes;
        h.large.retain_mut(|o| {
            if o.marked {
                o.marked = false;
                return true;
            }
            // Infallible, and not a user-input path: a tracked object's size
            // is one `alloc` already built a layout from.
            let layout = heap_layout(o.size)
                .expect("a live object's size was accepted by heap_layout at allocation");
            // SAFETY: `addr`/`size` are from this object's own allocation.
            unsafe { dealloc(o.addr as *mut u8, layout) };
            freed += o.size;
            false
        });

        // Every address this collection freed stops naming its object here
        // (module doc comment), and the executor keys a lookup on state-object
        // addresses. It is told once, before anything can be allocated again,
        // with a predicate that holds for exactly the live objects' starts.
        // Called with `HEAP` borrowed, which `prune_freed_states` tolerates: it
        // borrows one unrelated thread-local and allocates nothing through
        // `alloc`, so it cannot re-enter this collector or this borrow.
        let (pages, large) = (&h.pages, &h.large);
        crate::task::prune_freed_states(&|addr| {
            pages.slot_scan(addr).is_some()
                || large.binary_search_by_key(&addr, |o| o.addr).is_ok()
        });

        h.freed_bytes += freed as u64;
        h.live_bytes = swept.live_bytes + h.large.iter().map(|o| o.size).sum::<usize>();
        h.alloc_since_gc = 0;
        h.collections += 1;
        h.next_gc = std::cmp::max(INITIAL_THRESHOLD, h.live_bytes.saturating_mul(2));
        if debug() {
            eprintln!(
                "nova-gc: collection {} freed {freed} bytes, {} objects live ({} bytes)",
                h.collections,
                swept.live_slots + h.large.len(),
                h.live_bytes,
            );
        }
    });
}

/// The smallest range holding every page and every large object, or the
/// empty range `(0, 0)` when there is neither.
fn heap_range(pages: &pages::Pages, large: &[Obj]) -> (usize, usize) {
    let (mut lo, mut hi) = pages.bounds().unwrap_or((usize::MAX, 0));
    for o in large {
        lo = lo.min(o.addr);
        hi = hi.max(o.addr + o.size);
    }
    if lo < hi {
        (lo, hi)
    } else {
        (0, 0)
    }
}

/// Mark whatever `w` points into, if anything, and queue it for tracing if
/// it is scanned. `[lo, hi)` spans every page and large object, so most words
/// that are not pointers -- zero included -- stop at the first comparison.
fn mark_word(
    w: usize,
    lo: usize,
    hi: usize,
    pages: &mut pages::Pages,
    large: &mut [Obj],
    work: &mut Vec<(usize, usize)>,
) {
    if w < lo || w >= hi {
        return;
    }
    match pages.try_mark(w) {
        pages::Hit::Push(addr, len) => work.push((addr, len)),
        pages::Hit::Done => {}
        pages::Hit::Outside => {
            // The only candidate is the large object with the largest start <= w.
            let pos = large.partition_point(|o| o.addr <= w);
            if pos == 0 {
                return;
            }
            let o = &mut large[pos - 1];
            if w < o.addr + o.size && !o.marked {
                o.marked = true;
                if o.scan {
                    work.push((o.addr, o.size));
                }
            }
        }
    }
}
```

- [ ] **Step 6: Replace the test module's `reset` and `count`, and add `heap_bounds`**

```rust
    fn reset() {
        HEAP.with(|h| {
            let mut h = h.borrow_mut();
            h.pages.free_all();
            for o in h.large.drain(..) {
                let layout = heap_layout(o.size).expect("tracked size is describable");
                unsafe { dealloc(o.addr as *mut u8, layout) };
            }
            h.requested.clear();
            h.alloc_since_gc = 0;
            h.live_bytes = 0;
            h.next_gc = INITIAL_THRESHOLD;
        });
        // The other place every address stops naming its object, and so the
        // other place the executor's state-address lookup has to be told --
        // see the sweep in `collect_with_roots`. Here so that "a freed address
        // is always forgotten" has no exception, this test-only path included.
        crate::task::prune_freed_states(&|_| false);
        // Clears every piece of thread-local state this module owns, not
        // just `HEAP`, so a test that calls `reset()` starts from a fully
        // blank slate rather than trusting that nothing else could have left
        // `PINNED` non-empty. (Checked, not assumed: Rust's default test
        // harness gives every `#[test]` fn its own freshly spawned thread --
        // true even at `--test-threads=1` -- so a prior test's `PINNED` entry
        // does not actually reach a later one under `cargo test` as run
        // today. This clears anyway, since that's a property of the current
        // harness, not of this function's contract, and the pairing tests
        // below assert `PINNED`'s exact length.)
        PINNED.with(|p| p.borrow_mut().clear());
    }

    /// Live objects: allocated slots plus large objects.
    fn count() -> usize {
        HEAP.with(|h| {
            let h = h.borrow();
            h.pages.live_slots() + h.large.len()
        })
    }

    /// The range marking uses right now.
    fn heap_bounds() -> (usize, usize) {
        HEAP.with(|h| {
            let h = h.borrow();
            heap_range(&h.pages, &h.large)
        })
    }
```

- [ ] **Step 7: Replace `forget_freed_state` in `task.rs`**

Replace the function and its doc comment (lines 343-363) with:

```rust
/// Drop every [`BY_STATE`] key whose state object the collector has just
/// freed. `is_live` holds for exactly the start addresses of objects that
/// survived the collection.
///
/// The removal half of that map's own invariant, and the reason `gc.rs` calls
/// in here rather than this module watching for it: a freed address can be
/// reissued for an unrelated object (`gc.rs`'s module doc comment), and a
/// collection is where that transition happens. It calls this once, after it
/// has freed and before anything can be allocated again, so a key cannot
/// start naming the wrong thing.
///
/// **Removal only.** Nothing here inserts a key, and nothing here touches
/// `TASKS`, so this cannot introduce a key whose id `TASKS` does not have --
/// `spawn_internal` is still the only thing that puts a key in this map or an
/// entry in `TASKS`, and it takes the id from `TASKS` itself.
///
/// Costs one predicate call per key, which is per live task, not per freed
/// object.
pub(crate) fn prune_freed_states(is_live: &dyn Fn(usize) -> bool) {
    BY_STATE.with(|m| m.borrow_mut().retain(|&addr, _| is_live(addr)));
}
```

At line 224, change "[`forget_freed_state`] removes, at the moment the address stops" to "[`prune_freed_states`] removes, at the moment the address stops". The next line, "meaning anything.", stays.

At line 1411, change "([`forget_freed_state`]), and that cannot have happened while a live future" to "([`prune_freed_states`]), and that cannot have happened while a live future".

- [ ] **Step 8: Rewrite `gc.rs`'s module-doc paragraph on address identity**

Replace the paragraph beginning `//! **A sweep really frees, so an address is not a durable identity for an` and ending `//! has somewhere to be pointed at instead of a copy to compare.` with:

```rust
//! **Small objects live in size-class pages; large ones do not.** An object of
//! `pages::SMALL_MAX` bytes or less takes a slot in a 64 KiB page of one size
//! class (`gc/pages.rs`), tracked by out-of-band bitmaps. A bigger one gets a
//! system allocation of its own and an `Obj` record. Either way, `alloc`
//! returns a bare pointer with nothing in front of it.
//!
//! **A sweep really frees, so an address is not a durable identity for an
//! object.** An unmarked small object's slot goes back to its page's free
//! slots, and the next [`alloc`] of that size class can hand the same address
//! out for a wholly unrelated object. An unmarked large object's memory goes
//! back to the system allocator (`dealloc`), which can do the same. An address
//! therefore names an object only while that object is live. [`alloc`] starts
//! an address meaning what it means. The sweep in [`collect_with_roots`] ends
//! it, and so does the `#[cfg(test)]` `reset`, which frees everything. Every
//! path that ends an address's meaning must tell the executor's state-address
//! lookup before anything can be allocated again, so that "a freed address is
//! always forgotten" has no exception. Both do it by calling
//! `task::prune_freed_states` once, after freeing, and a third such path would
//! have to do the same. This is the one place that property is stated, so
//! that a reader who needs it has somewhere to be pointed at instead of a copy
//! to compare.
```

- [ ] **Step 9: Run the runtime tests**

Run: `cargo test --locked -p nova-runtime 2>&1 | grep -E "^test result|FAILED|panicked" | head -20`
Expected: every `test result:` line shows `0 failed`, and the new tests pass:
- `gc.rs`: `sizes_over_small_max_take_the_large_path`, `a_small_large_small_chain_survives_from_one_root`, `a_large_only_heap_collects`, `a_pointer_into_a_rounding_tail_keeps_its_object`, `words_outside_the_heap_range_mark_nothing` and `a_reused_slot_does_not_trace_its_previous_objects_words`;
- `task.rs`: `a_sweep_freeing_only_non_state_objects_removes_no_key`.

The existing gc tests and every `object_info` caller in `bytes.rs`, `fs.rs`, `lib.rs` and `task.rs` must still pass.

- [ ] **Step 10: Rebuild the workspace so the release and debug staticlibs carry the new runtime, then run the whole suite**

Run: `cargo build --locked --workspace 2>&1 | tail -2 && cargo test --locked --workspace > "$WS/task3-suite.log" 2>&1; grep -E "^test result" "$WS/task3-suite.log" | awk '{p+=$4; f+=$6; i+=$8} END {print p" passed / "f" failed / "i" ignored"}'`

`$WS` is this plan's workspace directory.

Expected: `0 failed`. The count is about 1147 + 21, one per new test across Tasks 1-3, with 8 ignored.
- If a `nova-cli` test fails with exit code 0xC0000005, or with empty stdout and an empty stderr, rerun that test alone three times.
- If it then passes, tally it in `docs/adr/0008-attributes-and-test-isolation.md` §4 as a recurrence and continue.
- If it fails alone, it is a real failure: use superpowers:systematic-debugging.

- [ ] **Step 11: Lint and format**

Run: `cargo fmt --all && cargo clippy --locked --workspace --all-targets --all-features -- -D warnings 2>&1 | tail -5`
Expected: `Finished` with no warnings.

- [ ] **Step 12: Commit**

```bash
git add crates/nova-runtime/src/gc.rs crates/nova-runtime/src/task.rs
git commit -m "feat(gc): allocate small objects from size-class pages

Objects of 2 KiB or less now live in 64 KiB pages and are marked and swept
by bitmap. That removes the per-collection index sort and the per-object
dealloc. Larger objects keep a system allocation each.

The executor's state map is pruned once per collection, against a
live-start predicate, instead of once per freed object.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 4: Records: ADR 0020, the spec chapters and every document naming the old mechanism

**Files:**
- Create: `docs/adr/0020-size-class-page-heap.md`
- Modify:
  - `nova-spec/13-RUNTIME.md`: the `Obj` block near line 97, the §3.1 sentence near line 168, and the §3.4 paragraph near line 217;
  - `docs/adr/0009-async-execution-model.md`, near lines 172 and 430;
  - `docs/adr/0012-file-descriptor-lifecycle.md`, near lines 31 and 171;
  - `docs/adr/0016-std-sync-partial-close.md`, near line 144;
  - `docs/adr/0017-std-sync-channel-shape.md`, near line 520;
  - `nova-spec/20-STDLIB.md`, near line 2020;
  - `std/sync/lib.nova`, the comment near line 35;
  - `CHANGELOG.md`, under `[Unreleased]` → `### Changed`.

**Interfaces:**
- Consumes: the merged code of Tasks 1-3, for the names `gc/pages.rs`, `prune_freed_states`, `SMALL_MAX` and `RESERVE_PAGES`.
- Produces: no code.

- [ ] **Step 1: Write a failing check for the stale claims**

Write `$WS/check_records.sh`:

```bash
#!/bin/bash
# RED while any living record still states the replaced mechanism as current.
cd /d/Projects/nona/nova
bad=0
need() { grep -q -- "$2" "$1" || { echo "MISSING in $1: $2"; bad=1; }; }
need docs/adr/0020-size-class-page-heap.md "Size-class page heap"
need nova-spec/13-RUNTIME.md "Amended 2026-10-01 (gc-page-heap)"
need docs/adr/0009-async-execution-model.md "Amended 2026-10-01 (gc-page-heap)"
need docs/adr/0012-file-descriptor-lifecycle.md "Amended 2026-10-01 (gc-page-heap)"
need docs/adr/0016-std-sync-partial-close.md "Amended 2026-10-01 (gc-page-heap)"
need docs/adr/0017-std-sync-channel-shape.md "Amended 2026-10-01 (gc-page-heap)"
need nova-spec/20-STDLIB.md "Amended 2026-10-01 (gc-page-heap)"
need std/sync/lib.nova "no longer has a per-object hook"
need CHANGELOG.md "size-class pages"
[ $bad -eq 0 ] && echo "records clean"
exit $bad
```

Run: `bash "$WS/check_records.sh"`
Expected: FAIL, with every `MISSING` line printed.

- [ ] **Step 2: Write ADR 0020**

Create `docs/adr/0020-size-class-page-heap.md`:

```markdown
# ADR 0020 — Size-class page heap

## Status

Accepted (2026-10-01). Design:
`docs/superpowers/specs/2026-10-01-gc-page-heap-design.md`.

## Context

The collector (`crates/nova-runtime/src/gc.rs`) gave every object its own
system allocation and an `Obj` record in one vector. Each collection rebuilt
and sorted an index of every object so marking could binary-search it, then
freed each dead object with its own `dealloc` and its own
`task::forget_freed_state` call. `examples/05-json-api/BENCHMARK.md`'s
"(gc-phase-cost)" amendment measured that per-object work (sweep, clearing
marks, building and sorting the index) at 69.3–70.7% of collection time,
and the collector at 93.8–101.6 µs of a ten-user request. The gate allows
100 µs for the whole request.

## Decision

- **Pages of one size class.** Every object of 2048 bytes or less
  (`pages::SMALL_MAX`) lives in a 65,536-byte page of one of 24 size
  classes, from 16 to 2048 bytes.
- **Bitmaps, kept out of band.** Each page's descriptor holds allocated,
  marked and scan bitmaps, outside the page, so `alloc` still returns a
  bare pointer with nothing in front of it.
- **Marking** finds a slot by binary-searching a directory of page bases.
- **The sweep** frees unmarked slots by bitmap arithmetic, 64 at a time.
- **Allocation** takes the lowest free bit and zeroes that slot.
- **Large objects keep the old path.** Anything over 2048 bytes keeps its
  own system allocation and an `Obj` record, in a short vector sorted at
  each collection.
- **Empty pages are bounded.** A sweep keeps up to 16 empty pages
  (`pages::RESERVE_PAGES`, 1 MiB) in a reserve and returns the rest to the
  system.
- **The executor's state map** is pruned once per collection by
  `task::prune_freed_states`, against a predicate that holds for exactly
  the start addresses of the survivors. There is therefore no longer a
  per-object notification of any kind.

The collector stays conservative, non-moving and thread-local, and still
skips collection off Windows.

## Alternatives rejected

- **Mixed-size blocks with an object-start bitmap (Immix-style).** No
  rounding waste, but it needs hole-finding and fragmentation handling. It
  also costs more per object to look up and sweep than a fixed slot
  division, for a heap whose objects were 77% 16 bytes or smaller
  ("(alloc-per-request)").
- **Pages whose free slots are threaded into a list through their first
  word.** Allocation is a pop, but the sweep has to write to every freed
  slot, which is the per-object cost being removed. Freed slots would also
  hold pointer-shaped words.
- **Local fixes on the old heap** (a radix sort, pruning the state map once)
  and **recycling freed blocks per size**. Each leaves the per-collection
  index, and none is sized like the gap to the gate.

## Consequences

- **Rounding waste.** Up to 128 bytes, a request rounds up to the next
  multiple of 16, at most 15 bytes. Above that it adds less than 25% of the
  request. How much of this is new against the system allocator's own
  granularity is not measured.
- **Partly filled pages** hold memory that the system allocator would have
  had back. The measurement amendment for this change reports the peak
  working set.
- **The per-object hook is gone.** ADRs 0012, 0016 and 0017,
  `nova-spec/13-RUNTIME.md` §3.4 and `nova-spec/20-STDLIB.md` describe that
  hook as existing, and carry dated amendments. Their decisions stand.
- **A thread that exits** leaks its pages, as it leaked its objects before.
  Its minimum is one page per size class it used.
```

- [ ] **Step 3: Amend `nova-spec/13-RUNTIME.md`**

After the closing fence of the `struct Obj { ... }` block near line 106, insert:

```markdown

**Amended 2026-10-01 (gc-page-heap):** that record now describes only objects
over 2048 bytes. Every smaller object lives in a slot of a 64 KiB size-class
page, and its allocated, marked and scan flags are bits in the page's
descriptor (`crates/nova-runtime/src/gc/pages.rs`). Both kinds of metadata stay
out of band, so `alloc` still returns a bare pointer with nothing in front of
it. ADR 0020 records the decision.
```

After the §3.1 sentence ending "Nothing may key a persistent table on an object's address." near line 171, insert:

```markdown

**Amended 2026-10-01 (gc-page-heap):** an unmarked object of 2048 bytes or less
now goes back to its page's free slots rather than to the system allocator.
The next allocation of its size class can take the same address, so the
property above holds more strongly than before. Only larger objects still go
back to the system allocator. ADR 0020 records the decision.
```

After the §3.4 paragraph ending "tells a table keyed on anything else (a file descriptor, say) nothing at all." near line 222, insert:

```markdown

**Amended 2026-10-01 (gc-page-heap):** that hook no longer exists. The
collector now frees small objects by bitmap, and it tells the executor's state
map once per collection, through `task::prune_freed_states` and a predicate on
the survivors' addresses, instead of once per freed object. There is no
per-object notification of any kind, so the argument above holds a fortiori.
ADR 0020 records the decision.
```

- [ ] **Step 4: Amend ADRs 0009, 0012, 0016 and 0017, `20-STDLIB.md` and `std/sync/lib.nova`**

Each insertion goes immediately after the paragraph or bullet named. The text is verbatim.

**`docs/adr/0009-async-execution-model.md`**, after the bullet containing "(`gc.rs`'s sweep calls `task::forget_freed_state`; the property that makes this necessary is stated" (near line 172), as a continuation of that bullet:

```markdown
  **Amended 2026-10-01 (gc-page-heap):** the sweep no longer calls a hook per
  freed object. The collector prunes the map once per collection, through
  `task::prune_freed_states`, before anything can be allocated again. The
  property and its home in `gc.rs`'s module doc comment are unchanged.
  ADR 0020.
```

**ADR 0009**, at the end of the paragraph containing "since the collector has a per-object hook" (near line 430):

```markdown
  **Amended 2026-10-01 (gc-page-heap):** that per-object hook no longer
  exists (ADR 0020), so the question it raised is moot. ADR 0012's decision
  stands.
```

**`docs/adr/0012-file-descriptor-lifecycle.md`**, after the paragraph ending "**A reader who finds this hook will reasonably ask why `File` does not register with it.**" (near line 43):

```markdown

**Amended 2026-10-01 (gc-page-heap):** that hook no longer exists. The
collector frees small objects by bitmap and prunes the executor's state map
once per collection (`task::prune_freed_states`), so it notifies nothing per
object at all (ADR 0020). Reason 1 below is therefore stronger, not weaker:
there is no per-object notification left to register with. Reason 2 is
unchanged, and so is the decision.
```

**ADR 0012**, in References, after the bullet beginning "`crates/nova-runtime/src/gc.rs`: the sweep loop and its":

```markdown
  **Amended 2026-10-01 (gc-page-heap):** that call is gone; see the
  amendment under Context and ADR 0020.
```

**`docs/adr/0016-std-sync-partial-close.md`**, after the paragraph ending "only the shape of the resulting trade-off does." (near line 149):

```markdown

**Amended 2026-10-01 (gc-page-heap):** the per-object notification hook
ADR 0012 describes no longer exists (ADR 0020). That does not change this
decision: a `MutexGuard`'s release never reached the collector anyway.
```

**`docs/adr/0017-std-sync-channel-shape.md`**, at the end of the References bullet that begins "`docs/adr/0012-file-descriptor-lifecycle.md`: the precedent for":

```markdown
  **Amended 2026-10-01 (gc-page-heap):** the per-object hook that bullet
  mentions no longer exists (ADR 0020), and the conclusion is unchanged.
```

**`nova-spec/20-STDLIB.md`**, after the paragraph containing "**ADR 0012 says the opposite**" (near line 2026):

```markdown

**Amended 2026-10-01 (gc-page-heap):** the hook ADR 0012 names no longer
exists. The collector now prunes the executor's state map once per
collection instead of notifying per freed object (ADR 0020). The conclusion
about `MutexGuard` is unchanged.
```

**`std/sync/lib.nova`**, replace the parenthetical comment beginning `// (An earlier version of this comment claimed ADR 0012 shows the collector` and ending `// transfer here.)` with:

```text
// (An earlier version of this comment claimed ADR 0012 shows the collector
// "offers no per-object hook". When written, ADR 0012 showed the opposite:
// the hook existed and the ADR named it, and its argument was that the hook
// reported *the dying object's own address*, which tells an `fd`-keyed table
// nothing. Since 2026-10-01 the collector no longer has a per-object hook at
// all (ADR 0020). Either way the argument is about `File`, not about a
// pure-Nova guard, and does not transfer here.)
```

- [ ] **Step 5: Add the CHANGELOG entry**

Under `## [Unreleased]` → `### Changed`, as the first bullet:

```markdown
- **The collector allocates small objects from size-class pages.** Every
  object of 2048 bytes or less now takes a slot in a 64 KiB page of one of
  24 size classes, tracked by out-of-band allocated, marked and scan
  bitmaps.
  - Marking finds a slot by searching a directory of page bases, instead
    of a sorted index of every object rebuilt at each collection.
  - The sweep frees slots by bitmap instead of calling `dealloc` per
    object.
  - Up to 16 empty pages (1 MiB) are kept for reuse, and the rest go back
    to the system.
  - Larger objects keep a system allocation each.
  - The executor's state map is pruned once per collection
    (`task::prune_freed_states`), instead of once per freed object. So the
    collector no longer has a per-object notification hook, and ADRs 0009,
    0012, 0016 and 0017 and `nova-spec/13-RUNTIME.md` and `20-STDLIB.md`
    carry dated amendments saying so.

  `gc::alloc`'s signature, its callers and codegen are unchanged.
  `docs/adr/0020-size-class-page-heap.md` records the decision, and the
  measured effect is under `### Measured`.
```

- [ ] **Step 6: Run the check, then the set-difference sweep**

Run: `bash "$WS/check_records.sh"`
Expected: `records clean`.

Run:

```bash
cd /d/Projects/nona/nova
for t in forget_freed_state 'system allocator' arena swap_remove 'per-object' 'objects.len' 'Obj {'; do
  echo "== $t"
  comm -23 <(git grep -l -- "$t" | sort) <(git diff --name-only main..HEAD | sort)
done
```

Expected: every file printed is one of these, and nothing else:
- a dated historical record: a `docs/superpowers/plans/` or `docs/superpowers/specs/` file older than 2026-10-01, a released section of `CHANGELOG.md`, or a dated amendment in `examples/05-json-api/BENCHMARK.md`;
- `docs/adr/0002-phase1-leaking-allocator.md`, a superseded ADR about the Phase 1 leaking allocator;
- `crates/nova-runtime/src/lib.rs`, whose line 684 says a size "the system allocator cannot satisfy" aborts, which is still true of pages and large objects.

Open each other file, decide whether it states the replaced mechanism as current, and amend it in the same style as Step 4. Ledger each file and the decision on it.

- [ ] **Step 7: Run the std tests, since `std/sync/lib.nova` changed**

Run: `cargo test --locked -p nova-cli sync_ 2>&1 | grep -E "^test result" | head -3`
Expected: `0 failed`.

- [ ] **Step 8: Commit**

```bash
git add docs/adr/0020-size-class-page-heap.md nova-spec/13-RUNTIME.md nova-spec/20-STDLIB.md docs/adr/0009-async-execution-model.md docs/adr/0012-file-descriptor-lifecycle.md docs/adr/0016-std-sync-partial-close.md docs/adr/0017-std-sync-channel-shape.md std/sync/lib.nova CHANGELOG.md
git commit -m "docs: record the size-class page heap and retire the per-object hook

ADR 0020 records the decision and its rejected alternatives. Every living
record that described the sweep's per-object forget_freed_state hook as
current now carries a dated amendment, and none of their decisions
change.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 5: Mutation audit and the full gates

**Files:**
- Modify: none committed. Each mutation is applied, tested and reverted with `git checkout -- <file>`.

**Interfaces:**
- Consumes: the code of Tasks 1-3.
- Produces: a ledger line per mutation, `Task 5: mutation <n> -> <failing test names>`. The PR description carries the same list.

- [ ] **Step 1: Apply each mutation alone, run the named tests, record and revert**

Run every command with `-p nova-runtime`. After each mutation, run `git diff --stat` to confirm only the one file changed. Run the command, record which tests fail, then `git checkout -- <file>`.

| # | mutation (in `pages.rs` unless stated) | command | must fail |
|---|---|---|---|
| 1 | in `take`, delete the `write_bytes` line | `cargo test --locked -p nova-runtime -- a_slot_is_zeroed a_freed_slot_is_handed_out_again a_reused_slot_does_not_trace` | all three |
| 2 | in `try_mark`, change `if page.alloc[word] & mask == 0 \|\| page.mark[word] & mask != 0` to `if page.mark[word] & mask != 0` | `cargo test --locked -p nova-runtime -- try_mark_passes_over` | `try_mark_passes_over_free_slots_tails_and_outside_words` |
| 3 | in `sweep`, delete `*m = 0;` | `cargo test --locked -p nova-runtime -- sweep_frees_the_unmarked` | `sweep_frees_the_unmarked_keeps_the_marked_and_clears_every_mark` |
| 4 | in `gc.rs`'s `collect_with_roots`, change the prune predicate to `&\|_\| true` | `cargo test --locked -p nova-runtime -- a_swept_states_key_is_dropped` | `a_swept_states_key_is_dropped_so_a_recycled_address_cannot_misresolve` |
| 5 | in `gc.rs`'s `collect_with_roots`, change the prune predicate to `&\|_\| false` | `cargo test --locked -p nova-runtime -- a_reachable_futures_key a_sweep_freeing_only` | both |
| 6 | in `sweep`, change `if self.reserve.len() < RESERVE_PAGES` to `if true` | `cargo test --locked -p nova-runtime -- empty_pages_past_the_reserve after_a_releasing_sweep` | both |
| 7 | in `gc.rs`'s `mark_word`, delete `if w < lo \|\| w >= hi { return; }` | `cargo test --locked -p nova-runtime -- words_outside_the_heap_range` | not required to fail: see the note |

**Mutation 7's note.** Without the range check, `try_mark` still returns `Outside` for `lo - 1` and the large lookup still misses, so this test may pass. The check is a fast path, not a correctness guard. Record what the test actually did. If it passed, ledger a ruling that the range check is performance-only and covered by none of the correctness tests.

Every other row must fail as stated. If one does not, the claimed coverage is false: add the missing test in the owning task's file, watch it fail against the mutant, revert the mutant, watch it pass, and commit.

- [ ] **Step 2: Run the full gates on the final tree**

```bash
cargo build --locked --workspace
cargo test --locked --workspace > "$WS/task5-suite.log" 2>&1; grep -E "^test result" "$WS/task5-suite.log" | awk '{p+=$4; f+=$6; i+=$8} END {print p" passed / "f" failed / "i" ignored"}'
cargo clippy --locked --workspace --all-targets --all-features -- -D warnings 2>&1 | tail -2
cargo fmt --all --check && echo fmt-clean
```

Expected: `0 failed` with 8 ignored, clippy `Finished` with no warnings, and `fmt-clean`.
- Every `*_under_gc_stress` test is in that run. Confirm with `grep -c "under_gc_stress ... ok" "$WS/task5-suite.log"`, which should report 12, one per `*_under_gc_stress` test in `run_tests.rs`.
- Treat the 0xC0000005 flake as in Task 3, Step 10.

- [ ] **Step 3: Ledger the suite totals.** No commit.

---

### Task 6: Measure the change and record it

**Files:**
- Modify: `examples/05-json-api/BENCHMARK.md` (a new amendment before `## What was measured, and with what`), and `CHANGELOG.md` under `[Unreleased]` → `### Measured`.
- Scratch only, never committed:
  - `/tmp/gcm/pg/` for binaries and logs;
  - `scratchpad/gc-pageheap-prof.patch`.

**Interfaces:**
- Consumes: the final tree of Tasks 1-5, `main` at `331dec5` as the baseline, and `D:/Projects/nona/nova/target/release/nova-bench-http.exe`.
- Produces: the amendment "AMENDMENT 2026-10-01 (gc-page-heap)", or dated the day it runs.

- [ ] **Step 1: Write the predictions file before building anything**

```bash
mkdir -p /tmp/gcm/pg && cat > /tmp/gcm/pg/predict.txt <<'EOF'
Predictions for the page heap, written before any measurement:
- ten-user throughput: before 4200-4800 req/sec (PR #61 measured 4257-4770); after 5500-8000
- collector per request (profiling build): 15-35 us, from 93.8-101.6
- allocation per call (calibrated sample): 15-40 ns, from 65.6-77.4
- objects taking the large path: under 5 per request
- peak working set: after between 0.8x and 1.5x before
EOF
date >> /tmp/gcm/pg/predict.txt; cat /tmp/gcm/pg/predict.txt
```

- [ ] **Step 2: Build the before and after binaries**

```bash
cd /d/Projects/nona/nova
git worktree add ../nova-before 331dec5
(cd ../nova-before && cargo build --release --locked --workspace 2>&1 | tail -1)
rm -f /tmp/gcm/pg/before /tmp/gcm/pg/before.exe
../nova-before/target/release/nova build examples/05-json-api/src/main.nova -o /tmp/gcm/pg/before.exe
cargo build --release --locked --workspace 2>&1 | tail -1
rm -f /tmp/gcm/pg/after /tmp/gcm/pg/after.exe
target/release/nova build examples/05-json-api/src/main.nova -o /tmp/gcm/pg/after.exe
ls -l /tmp/gcm/pg/before.exe /tmp/gcm/pg/after.exe | awk '{print $5, $9}'
```

Expected: two files. Record both byte sizes. `before.exe` should be 691,712 bytes, matching "(noncollector-cost)"'s plain build. If it is not, stop and find out why before measuring.

- [ ] **Step 3: Write the peak-memory harness**

Create `/tmp/gcm/pg/run_pk.sh`:

```bash
#!/bin/bash
# usage: run_pk.sh LABEL -- one fresh server process; BIN selects the binary
label=$1
D=/d/Projects/nona/nova
OUT=/tmp/gcm/pg/$label
rm -f $OUT.out $OUT.err
${BIN:?} >$OUT.out 2>$OUT.err &
pid=$!
for i in $(seq 1 50); do
  port=$(grep -o 'listening on 127.0.0.1:[0-9]*' $OUT.out | grep -o '[0-9]*$')
  [ -n "$port" ] && break; sleep 0.1
done
for n in 1 2 3 4 5 6 7 8 9 10; do
  curl -s -o /dev/null -X POST -H 'content-type: application/json' \
    -d "{\"name\":\"User Number $n\",\"email\":\"user$n@example.com\"}" http://127.0.0.1:$port/users
done
bytes=$(curl -s http://127.0.0.1:$port/users | wc -c)
res=$(MSYS_NO_PATHCONV=1 $D/target/release/nova-bench-http.exe --addr 127.0.0.1:$port --path /users --connections 200 --duration 15 --warmup 5 | grep RESULT)
wpid=$(cat /proc/$pid/winpid)
peak=$(powershell.exe -NoProfile -Command "(Get-Process -Id $wpid).PeakWorkingSet64" | tr -d '\r')
kill $pid 2>/dev/null; sleep 0.5
echo "$label body_bytes=$bytes peak_ws=$peak | $res" | tee -a /tmp/gcm/pg/results.log
```

- [ ] **Step 4: Take at least three alternated readings of each build**

```bash
cd /tmp/gcm/pg && : > results.log
for r in 1 2 3; do
  BIN=/tmp/gcm/pg/before.exe bash run_pk.sh before_$r
  BIN=/tmp/gcm/pg/after.exe bash run_pk.sh after_$r
done
cat results.log
```

Expected:
- six lines, each with `errors=0` and `body_bytes=604`;
- `rps=` and `peak_ws=` on every line.

**A gain is claimed only if the after range lies wholly above the before range.** If the ranges overlap, take three more of each and report all readings. Do not claim a gain on overlapping ranges.

- [ ] **Step 5: Build the scratch decomposition binary**

Apply the scratch timers to `gc.rs` in the main checkout:
- **Allocation:** time one `alloc` call in 16, skipping any call during which a collection ran.
- **Collection:** time each phase of `collect_with_roots`:
  - root scan, in `collect`;
  - the large sort;
  - mark;
  - the page sweep;
  - the large sweep;
  - prune;
  - the whole collection.
- **The large path:** count the objects and bytes it takes.

Print everything under `NOVA_GC_DEBUG` on one `nova-gc-pg:` line per collection. The patch:

```rust
// SCRATCH -- never committed.
thread_local! {
    // 0 cols, 1 scan, 2 lsort, 3 mark, 4 psweep, 5 lsweep, 6 prune, 7 total,
    // 8 alloc calls, 9 sampled alloc ns, 10 samples, 11 large objs, 12 large bytes
    static PG: RefCell<[u64; 13]> = const { RefCell::new([0; 13]) };
}
fn pg_add(i: usize, v: u64) {
    PG.with(|p| p.borrow_mut()[i] += v);
}
```

- **Wrapping `alloc`:** rename the real `alloc` to `alloc_inner` and add:
  ```rust
  pub fn alloc(size: usize, scan: bool) -> *mut u8 {
      pg_add(8, 1);
      if PG.with(|p| p.borrow()[8]) % 16 != 0 {
          return alloc_inner(size, scan);
      }
      let c0 = HEAP.with(|h| h.borrow().collections);
      let t = std::time::Instant::now();
      let p = alloc_inner(size, scan);
      let ns = t.elapsed().as_nanos() as u64;
      if HEAP.with(|h| h.borrow().collections) == c0 {
          pg_add(9, ns);
          pg_add(10, 1);
      }
      p
  }
  ```
- **The large path:** in `alloc_inner`'s large branch, add `pg_add(11, 1); pg_add(12, size as u64);`.
- **Timing phases:** in `collect`, wrap the roots gathering in `let t = Instant::now(); ... pg_add(1, ...)`. In `collect_with_roots`, wrap each phase the same way, with indices 2-6, and wrap the whole of `collect` with index 7.
- **Printing:** when `debug()` is set, after each collection, print all 13 counters as `nova-gc-pg: cols=.. scan=.. lsort=.. mark=.. psweep=.. lsweep=.. prune=.. total=.. calls=.. alloc_ns=.. samples=.. large_objs=.. large_bytes=..`.

Save the diff with `git diff > scratchpad/gc-pageheap-prof.patch`, then:

```bash
cargo build --release --locked --workspace 2>&1 | tail -1
rm -f /tmp/gcm/pg/prof /tmp/gcm/pg/prof.exe
target/release/nova build examples/05-json-api/src/main.nova -o /tmp/gcm/pg/prof.exe
ls -l /tmp/gcm/pg/prof.exe | awk '{print $5}'
git checkout -- crates/nova-runtime/src/gc.rs && git status --short
cargo build --release --locked --workspace 2>&1 | tail -1
```

Expected:
- `git status` shows a clean tree after the checkout;
- the release toolchain is rebuilt without the timers.

- [ ] **Step 6: Take three decomposition readings with every RESULT line kept**

```bash
cd /tmp/gcm/pg && : > prof.log
for r in 1 2 3; do
  NOVA_GC_DEBUG=1 BIN=/tmp/gcm/pg/prof.exe bash -c '
    OUT=/tmp/gcm/pg/prof_'$r'
    rm -f $OUT.out $OUT.err
    $BIN >$OUT.out 2>$OUT.err & pid=$!
    for i in $(seq 1 50); do port=$(grep -o "listening on 127.0.0.1:[0-9]*" $OUT.out | grep -o "[0-9]*$"); [ -n "$port" ] && break; sleep 0.1; done
    for n in 1 2 3 4 5 6 7 8 9 10; do curl -s -o /dev/null -X POST -H "content-type: application/json" -d "{\"name\":\"User Number $n\",\"email\":\"user$n@example.com\"}" http://127.0.0.1:$port/users; done
    res=$(MSYS_NO_PATHCONV=1 /d/Projects/nona/nova/target/release/nova-bench-http.exe --addr 127.0.0.1:$port --path /users --connections 200 --duration 15 --warmup 0 | grep RESULT)
    kill $pid; sleep 0.5
    echo "prof_'$r' | $res | $(grep nova-gc-pg $OUT.err | tail -1)" >> /tmp/gcm/pg/prof.log'
done
cat prof.log
```

Expected: three lines, each carrying `errors=0`, `requests=` and a `nova-gc-pg:` line.
- Per request: each cumulative counter ÷ `requests`.
- Per allocation: `alloc_ns / samples − 35.8` ns, the calibrated clock bias from "(noncollector-cost)".

- [ ] **Step 7: Write the amendment and the CHANGELOG bullet**

Insert "## AMENDMENT 2026-10-01 (gc-page-heap): small objects in size-class pages" before `## What was measured, and with what` in `examples/05-json-api/BENCHMARK.md`. Use the measured values only. It must contain:
1. **A headline.** State whether the throughput ranges are disjoint, and the after range. Give the gate's remaining factor as 10000 ÷ the after range.
2. **"How it was measured":** the binaries and their byte sizes, the configuration, the alternation, and the predictions table, with each verdict checked against `predict.txt`.
3. **"Throughput and footprint":** every reading, rps and peak working set, before and after.
4. **"Where a request's time goes now":** the decomposition per request beside the "(gc-phase-cost)" figures, the allocation cost per call, and the large path's objects and bytes per request.
5. **"What this does not settle":** at least, one host; how much rounding waste is new; and anything the readings left unresolved.

Add a `### Measured` bullet to `CHANGELOG.md` under `[Unreleased]` that summarises the same figures in rounded form and points at the amendment.

- [ ] **Step 8: Have the record checked independently, then commit**

Dispatch a fresh agent with no cargo, builds or edits allowed. It gets the diff of `BENCHMARK.md` and `CHANGELOG.md` plus `/tmp/gcm/pg/{predict.txt,results.log,prof.log}`. Ask it to:
- recompute every number from those logs;
- check every verdict against `predict.txt`;
- flag every quantifier and causal claim the data does not support.

Fix each finding, then re-dispatch the same agent on the revision until it reports nothing WRONG or UNSUPPORTED.

```bash
git worktree remove ../nova-before
git add examples/05-json-api/BENCHMARK.md CHANGELOG.md
git commit -m "docs(json-api): measure the size-class page heap

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

Replace the commit subject's description with the measured headline before committing. For example: "page heap: 4257-4770 -> X-Y req/sec, disjoint".
