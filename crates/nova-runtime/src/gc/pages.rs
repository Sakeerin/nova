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
            for (a, m) in page.alloc[..words].iter_mut().zip(&mut page.mark[..words]) {
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

    /// Test-only: every cursor names a page its class owns, and every
    /// reserve page is empty and owned by no class. A sweep must leave both
    /// true, or a class could allocate into a page the reserve or the system
    /// now holds.
    #[cfg(test)]
    pub(super) fn cursors_are_sound(&self) -> bool {
        let in_dir = |d: usize| self.dir.iter().any(|&(_, x)| x == d);
        let owned =
            |d: usize, c: usize| in_dir(d) && !self.descs[d].reserved && self.descs[d].class == c;
        (0..NUM_CLASSES).all(|c| {
            self.current[c].iter().all(|&d| owned(d, c))
                && self.with_space[c].iter().all(|&d| owned(d, c))
        }) && self.reserve.iter().all(|&d| {
            in_dir(d) && self.descs[d].reserved && self.descs[d].alloc.iter().all(|&w| w == 0)
        })
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
        assert_eq!(
            (slot, nslots),
            (48, 1365),
            "65536 / 48 = 1365, 16 bytes over"
        );
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
        assert!(p.cursors_are_sound());
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
        assert!(p.cursors_are_sound());
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
        assert!(p.cursors_are_sound());
        assert_eq!((s.freed_bytes, s.live_slots, s.live_bytes), (64, 1, 64));
        assert_eq!(p.slot_scan(keep), Some(true));
        assert_eq!(p.slot_scan(gone), None);
        // The mark was cleared, so a second sweep with nothing marked frees
        // `keep` too.
        let s = p.sweep();
        assert!(p.cursors_are_sound());
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
        assert!(p.cursors_are_sound());
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
        assert!(p.cursors_are_sound());
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
        assert!(p.cursors_are_sound());
        assert_eq!(s.live_slots, 0);
        assert_eq!(
            p.page_count(),
            RESERVE_PAGES,
            "pages past the reserve were released"
        );
        assert_eq!(p.reserve_count(), RESERVE_PAGES);
        assert!(p.directory_is_sorted());
        // Review Focus 2: a reserve page reused by another class is sized for it.
        let big = p.alloc_slot(class_of(2048), true);
        assert_eq!(
            p.page_count(),
            RESERVE_PAGES,
            "a reserve page was reused, none added"
        );
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
        assert!(p.cursors_are_sound());
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
}
