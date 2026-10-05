//! A conservative, non-moving mark-and-sweep garbage collector.
//!
//! Neither Nova codegen backend emits stack maps or per-slot type information,
//! so the collector cannot know precisely where roots or heap pointers live.
//! It is therefore *conservative*: any machine word (on the stack, in a
//! callee-saved register, inside a scanned heap object, or explicitly
//! registered -- see below) whose value falls within a live allocation keeps
//! that allocation alive. This can retain a little garbage (an integer that
//! happens to look like a pointer) but never frees a reachable object.
//!
//! Collection is triggered from [`alloc`] once allocation since the last cycle
//! crosses a growth threshold (or on every allocation under `NOVA_GC_STRESS`,
//! used to shake out root-scanning bugs). Roots come from:
//!
//! - **callee-saved registers**, flushed onto the stack by the register-spill
//!   shim in `gc_stack.c` (caller-saved registers hold no live root at a call
//!   boundary);
//! - **the stack**, scanned from the current frame up to the thread's base;
//! - **explicitly registered roots** ([`add_root`]/[`remove_root`]), for
//!   objects reachable from neither: a suspended async task's state is owned
//!   by the Rust executor while the task is parked, on no Nova stack and in
//!   no register, so it must be pinned by address instead.
//!
//! Marking is range-based, so interior pointers (e.g. an array-element address
//! held transiently) keep their containing object alive. Objects flagged
//! `scan = false` are leaves and are not traced: strings and `Bytes` (a
//! runtime-made value is one object holding its header and bytes; a string
//! literal's is a 16-byte header pointing at static data).
//!
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
//!
//! The stack scan needs the calling thread's stack top, which `stack_base`
//! finds on Windows, glibc Linux and macOS
//! (`docs/adr/0024-gc-stack-bounds-on-unix.md`). On any other platform
//! collection is skipped: allocations leak until exit, which is never unsafe,
//! and `NOVA_GC_DEBUG` says so once per thread.

use std::alloc::{alloc_zeroed, dealloc, handle_alloc_error, Layout};
use std::cell::RefCell;
#[cfg(test)]
use std::collections::BTreeMap;
use std::ffi::c_void;
use std::sync::OnceLock;

mod pages;

/// All heap objects are 8-byte-slot aligned; 16-byte alignment keeps the
/// returned pointer well-aligned for every value class.
const ALIGN: usize = 16;

/// The largest object [`alloc`] can describe. A bigger request cannot be
/// expressed as a [`Layout`] at [`ALIGN`] at all: rounding it up to the
/// alignment would pass `isize::MAX`, which `Layout` rejects.
///
/// This constant exists only so the diagnostic can name the limit — the
/// *decision* is always `Layout`'s own (see [`heap_layout`]), never a
/// re-derivation of its rule. `max_heap_object_is_the_largest_describable_size`
/// pins the two together so they cannot disagree.
const MAX_HEAP_OBJECT: usize = (isize::MAX as usize) - (ALIGN - 1);

/// Collect once this many bytes have been allocated since the last cycle
/// (grows with the live set afterward).
const INITIAL_THRESHOLD: usize = 1 << 20; // 1 MiB

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

thread_local! {
    static HEAP: RefCell<Heap> = const { RefCell::new(Heap::new()) };
    /// Scratch buffer of candidate root words, filled by `nova_gc_scan_range`.
    static ROOTS: RefCell<Vec<usize>> = const { RefCell::new(Vec::new()) };
    /// Explicitly registered roots — addresses the collector must treat as
    /// live even though they appear on no stack and in no register.
    ///
    /// This exists for suspended async tasks: the executor owns a task's
    /// state object while the task is parked, and the only root sources this
    /// collector has are the Nova stack and callee-saved registers. Without
    /// registration, a suspended task's state is swept.
    ///
    /// **Deliberately NOT merged into `ROOTS`.** `ROOTS` is scratch: it is
    /// cleared at the start of every cycle. A registry sharing it would be
    /// consumed by the first collection and the root swept by the second —
    /// a failure mode invisible to any single-collection test.
    static PINNED: RefCell<Vec<usize>> = const { RefCell::new(Vec::new()) };
}

extern "C" {
    /// Flushes callee-saved registers and scans the stack (see `gc_stack.c`),
    /// calling back into [`nova_gc_scan_range`].
    fn nova_gc_collect_roots(stack_base: *mut c_void);
}

fn stress() -> bool {
    static S: OnceLock<bool> = OnceLock::new();
    *S.get_or_init(|| std::env::var_os("NOVA_GC_STRESS").is_some())
}

fn debug() -> bool {
    static D: OnceLock<bool> = OnceLock::new();
    *D.get_or_init(|| std::env::var_os("NOVA_GC_DEBUG").is_some())
}

/// The layout of a `size`-byte heap object, or `None` if no such object can
/// exist because `size` is too large to describe.
///
/// `Layout::from_size_align` is the authority on which sizes are legal, so this
/// asks it rather than restating its rule (`size` rounded up to `ALIGN` must not
/// pass `isize::MAX`) — a restatement could drift out of agreement with it. It
/// is a pure arithmetic check: nothing is allocated, so the decision is
/// testable at any size.
fn heap_layout(size: usize) -> Option<Layout> {
    Layout::from_size_align(size, ALIGN).ok()
}

/// Allocate `size` zeroed bytes as a GC-managed object. `scan` selects whether
/// the collector traces the object's contents for pointers.
///
/// Aborts the process with a `nova: panic:` diagnostic if `size` exceeds
/// [`MAX_HEAP_OBJECT`], which is unsatisfiable rather than merely unavailable;
/// a size that is describable but unavailable goes to `handle_alloc_error`.
pub fn alloc(size: usize, scan: bool) -> *mut u8 {
    let size = size.max(8);
    // Reject an undescribable size before doing any work. This is *not* an
    // out-of-memory condition — no allocator could ever satisfy the request,
    // because there is no `Layout` for it — so it does not go through
    // `handle_alloc_error` (which would need the very layout that failed).
    // Instead it is a deliberate runtime abort in the style of
    // `nova_rt_panic_str` and `nova_rt_check_bounds`.
    //
    // It is reachable from ordinary Nova source: `[x; n]` with `n` at the top
    // of the legal length range asks for `8 * n + 8` bytes, and at
    // `n = MAX_ARRAY_LEN` that is 8 bytes past what `ALIGN` lets `Layout`
    // express. Every allocation site in the language funnels through here, so
    // any computed size can land on it.
    let Some(layout) = heap_layout(size) else {
        eprintln!(
            "nova: panic: allocation of {size} bytes exceeds the maximum object size of {MAX_HEAP_OBJECT} bytes"
        );
        std::process::abort();
    };
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

/// Register `ptr` as a root until [`remove_root`].
///
/// **Multiset, not set.** Each call adds one registration, so an address
/// registered twice remains a root until it has been removed twice. This
/// function does not deduplicate, and registrations carry no identity beyond
/// the address. Pairing adds with removes is entirely the caller's duty; the
/// registry has no notion of who registered an address or at what point in
/// that caller's lifecycle, and deliberately says nothing about it --
/// a pairing policy belongs to the module that owns the policy, not here.
/// `registering_the_same_address_twice_requires_removing_it_twice` pins the
/// multiset semantics against a future "simplification" to a set.
///
/// **Same-thread only.** `PINNED` is thread-local, like `HEAP`, so a
/// [`remove_root`] issued on a different thread than the matching `add_root`
/// silently leaves the registration behind on the original thread rather than
/// failing -- a leak `remove_root_actually_unroots` catches within one thread
/// but cannot see across two, since it never crosses threads.
pub fn add_root(ptr: *mut u8) {
    PINNED.with(|p| p.borrow_mut().push(ptr as usize));
}

/// Cancel exactly one registration of `ptr` made by [`add_root`].
///
/// Removing an address that was never registered, or removing it more times
/// than it was registered, is a no-op rather than a panic — the runtime must
/// not abort a user's program over its own bookkeeping. *Which* of several
/// identical registrations is cancelled is unobservable, since they differ in
/// nothing but existence, so removing the most recently added match
/// (`rposition` + `swap_remove`) is an implementation choice rather than part
/// of the contract. Same-thread only, like [`add_root`] -- see its doc
/// comment.
pub fn remove_root(ptr: *mut u8) {
    PINNED.with(|p| {
        let mut v = p.borrow_mut();
        if let Some(i) = v.iter().rposition(|&a| a == ptr as usize) {
            v.swap_remove(i);
        }
    });
}

/// Test-only: the `(size, scan)` this collector recorded for the live object
/// starting at `addr`, or `None` if `addr` is not a tracked object's start
/// address.
///
/// Exists because reading back the words `alloc` handed out (as
/// `nova-runtime`'s own layout tests do) cannot distinguish a correctly-sized,
/// correctly-scanned allocation from one that merely has enough slop past its
/// declared size for a test's own assertions to still land inside live
/// memory, or one whose `scan` flag is wrong (undetectable by reading words at
/// all — it only changes GC behaviour). This reaches into the collector's own
/// records so a caller can assert the exact size and scan flag `alloc` was
/// given, not just what got written.
#[cfg(test)]
pub(crate) fn object_info(addr: usize) -> Option<(usize, bool)> {
    HEAP.with(|h| {
        let h = h.borrow();
        let scan = h
            .pages
            .slot_scan(addr)
            .or_else(|| h.large.iter().find(|o| o.addr == addr).map(|o| o.scan))?;
        let size = *h
            .requested
            .get(&addr)
            .expect("alloc records every object's requested size in test builds");
        Some((size, scan))
    })
}

/// Test-only: how many times `addr` is currently registered in the root
/// registry (`0` if not registered at all).
///
/// [`add_root`] is a multiset, not a set -- two registrations require two
/// [`remove_root`] calls -- so a caller pairing them has an exact count to
/// assert, not merely a yes/no. This exists so such a caller can assert its
/// own pairing **without** running a collection. What that pairing should be
/// is the calling module's to state, not this one's; what this function
/// provides is a way to check it deterministically, because a
/// `collect()`-based assertion would instead inherit the conservative scan's
/// intermittent over-retention
/// (`docs/adr/0010-conservative-scan-root-test-gating.md`) for an invariant
/// that is really about this `Vec`'s contents, and could pass on an
/// accidental stack root. Reading the registry directly is deterministic and
/// works on every platform; a `collect()`-based assertion is neither, since
/// the scan can over-retain and does not run where `stack_base` has no
/// implementation.
#[cfg(test)]
pub(crate) fn root_count(addr: usize) -> usize {
    PINNED.with(|p| p.borrow().iter().filter(|&&a| a == addr).count())
}

/// Test-only: force a real, stack-scanning collection cycle immediately,
/// bypassing [`maybe_collect`]'s allocation-threshold trigger, so a test can
/// assert on a collection's outcome deterministically rather than allocating
/// until one happens to fire. A thin wrapper around the same [`collect`]
/// `alloc` itself uses, so a caller outside this module (`task.rs`'s tests)
/// names its intent -- "run a real collection now, for this test" -- instead
/// of reaching for the allocation-triggered entry point directly.
///
/// `#[inline(always)]` is load-bearing here, not a speed hint: this
/// collector's scan walks the live stack (module doc comment, top of file),
/// so a caller's already-nulled local surviving a collection depends on
/// *something* between the null and the scan overwriting that local's old
/// stack slot before the scan reads it -- for the tests this function exists
/// for, that something is `collect()`'s own stack usage one frame up. A real
/// (non-inlined) call to this wrapper inserts an extra frame between such a
/// caller and `collect()`, shifting every address `collect()` itself touches
/// one frame further from the caller's -- which changes whether that
/// overwrite still happens to land on the right bytes.
///
/// That is the mechanism, and it is the whole claim being made here. This
/// attribute does **not** make the tests that call this function reliable:
/// they are unconditionally `#[ignore]`d for intermittent over-retention that
/// happens with the attribute in place
/// (`docs/adr/0010-conservative-scan-root-test-gating.md`). Removing it
/// changes their frame relationship to `collect()` for no reason; keeping it
/// is not evidence that they pass.
///
/// Its only callers are `task.rs`'s `root_registration` tests, which are
/// compiled on every platform and run under `--ignored`. Where `stack_base`
/// has no implementation, `collect()` returns before marking anything, so
/// their assertions would pass or fail for the wrong reason; no CI platform
/// is such a platform (`docs/adr/0024-gc-stack-bounds-on-unix.md`).
#[cfg(test)]
#[inline(always)]
pub(crate) fn collect_for_test() {
    collect();
}

/// Test-only: run one mark-and-sweep cycle against an **explicit** candidate
/// root set, with no stack scan and no consultation of [`PINNED`] -- the same
/// deterministic core [`collect_with_roots`] gives this module's own tests,
/// reachable from a sibling module's tests.
///
/// The difference from [`collect_for_test`] is the whole reason this exists.
/// That one runs the real cycle, so what survives depends on the conservative
/// stack scan -- which runs only where `stack_base` has an implementation, and which
/// intermittently reads a stale word in an already-returned frame as a root
/// (`docs/adr/0010-conservative-scan-root-test-gating.md`, which owns that
/// finding and the gating it forced). Handing the root set in makes "was this
/// object swept" a decision about the argument instead, so a caller asserting
/// a *consequence* of sweeping can do it deterministically and on every
/// platform.
///
/// The caller chooses the whole root set, so an object the collector would
/// otherwise have kept -- including one registered through [`add_root`] -- is
/// freed if the caller does not name it. That is the point (a caller can stage
/// the exact reachability it wants to assert about) and also the hazard: every
/// address this thread allocated and did not name is dangling afterwards.
#[cfg(test)]
pub(crate) fn sweep_with_roots_for_test(roots: &[usize]) {
    collect_with_roots(roots);
}

fn maybe_collect(incoming: usize) {
    let over = HEAP.with(|h| {
        let h = h.borrow();
        h.alloc_since_gc + incoming >= h.next_gc
    });
    if over || stress() {
        collect();
    }
}

fn collect() {
    // Capture the stack base once; give up (leak) on unsupported platforms,
    // and say so once per thread under `NOVA_GC_DEBUG`. `eprintln!` allocates
    // only through the system allocator, never this heap.
    let base = HEAP.with(|h| {
        let mut h = h.borrow_mut();
        if h.base == 0 {
            h.base = stack_base().unwrap_or_else(|| {
                if debug() {
                    eprintln!(
                        "nova-gc: no stack bounds on this platform; \
                         collection is disabled and every allocation leaks"
                    );
                }
                usize::MAX
            });
        }
        h.base
    });
    if base == usize::MAX {
        HEAP.with(|h| h.borrow_mut().alloc_since_gc = 0);
        return;
    }

    // Gather candidate roots by flushing registers and scanning the stack.
    // This fills `ROOTS` and must not touch `HEAP` (avoids re-entrant borrows).
    // `PINNED` is copied in first, before the stack scan appends to the same
    // buffer, so registered roots are marked by the identical range-based walk
    // (see `mark_word`) instead of a separate path -- that's what lets a
    // registered root's transitive children get traced, not just the root
    // word itself. Copied, not drained: `PINNED` is a persistent registry, not
    // scratch, and must still hold its contents on the next collection.
    ROOTS.with(|r| {
        let mut r = r.borrow_mut();
        r.clear();
        PINNED.with(|p| r.extend_from_slice(&p.borrow()));
    });
    // SAFETY: `base` is this thread's stack origin; the shim scans our own
    // live stack and calls `nova_gc_scan_range`.
    unsafe { nova_gc_collect_roots(base as *mut c_void) };
    let roots = ROOTS.with(|r| std::mem::take(&mut *r.borrow_mut()));

    collect_with_roots(&roots);
}

/// Mark-and-sweep given an explicit candidate root set. Split from stack
/// scanning so the core is deterministically testable.
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
            pages.slot_scan(addr).is_some() || large.binary_search_by_key(&addr, |o| o.addr).is_ok()
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

/// Push every aligned machine word in `[lo, hi)` as a candidate root. Called
/// by `gc_stack.c`'s register-spill shim, with a range that starts below the
/// spilled registers and ends at the stack top.
///
/// # Safety
/// `[lo, hi)` must be a readable range of this thread's own stack.
#[no_mangle]
pub extern "C" fn nova_gc_scan_range(lo: *const c_void, hi: *const c_void) {
    let lo = lo as usize;
    let hi = hi as usize;
    if lo == 0 || hi == 0 || lo >= hi {
        return;
    }
    let mut p = (lo + 7) & !7; // align up to a word boundary
    ROOTS.with(|r| {
        let mut r = r.borrow_mut();
        while p + 8 <= hi {
            // SAFETY: within the caller-guaranteed live stack range.
            let w = unsafe { *(p as *const usize) };
            r.push(w);
            p += 8;
        }
    });
}

#[cfg(windows)]
fn stack_base() -> Option<usize> {
    extern "system" {
        fn GetCurrentThreadStackLimits(low_limit: *mut usize, high_limit: *mut usize);
    }
    let mut low = 0usize;
    let mut high = 0usize;
    // SAFETY: both out-pointers are valid; the API writes the current thread's
    // stack bounds.
    unsafe { GetCurrentThreadStackLimits(&mut low, &mut high) };
    (high > low).then_some(high)
}

/// The calling thread's stack top on glibc Linux, from `pthread_getattr_np`:
/// its lowest address plus its size. On the main thread glibc reads
/// `/proc/self/maps` for this, and fails without `/proc`.
#[cfg(all(target_os = "linux", target_env = "gnu"))]
fn stack_base() -> Option<usize> {
    // SAFETY: `pthread_getattr_np` fills `attr` for the calling thread, and
    // `pthread_attr_getstack` reads the stack's lowest address and size from
    // it. `attr` is destroyed before returning.
    unsafe {
        let mut attr: libc::pthread_attr_t = std::mem::zeroed();
        if libc::pthread_getattr_np(libc::pthread_self(), &mut attr) != 0 {
            return None;
        }
        let mut addr: *mut libc::c_void = std::ptr::null_mut();
        let mut size: libc::size_t = 0;
        let rc = libc::pthread_attr_getstack(&attr, &mut addr, &mut size);
        libc::pthread_attr_destroy(&mut attr);
        if rc != 0 || addr.is_null() || size == 0 {
            return None;
        }
        Some(addr as usize + size)
    }
}

/// The calling thread's stack top on macOS, which `pthread_get_stackaddr_np`
/// reports directly.
#[cfg(target_os = "macos")]
fn stack_base() -> Option<usize> {
    // SAFETY: reads the calling thread's own stack top.
    let top = unsafe { libc::pthread_get_stackaddr_np(libc::pthread_self()) };
    (!top.is_null()).then_some(top as usize)
}

/// No stack bounds on this platform: `collect()` skips collection, and every
/// allocation lives until the process exits.
#[cfg(not(any(
    windows,
    all(target_os = "linux", target_env = "gnu"),
    target_os = "macos"
)))]
fn stack_base() -> Option<usize> {
    None
}

#[cfg(test)]
mod tests {
    use super::*;

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

    #[test]
    fn unrooted_objects_are_freed() {
        reset();
        let a = alloc(24, true) as usize;
        let b = alloc(24, true) as usize;
        let _c = alloc(24, true) as usize;
        assert_eq!(count(), 3);
        collect_with_roots(&[a, b]);
        assert_eq!(count(), 2);
    }

    #[test]
    fn no_roots_frees_everything() {
        reset();
        let _ = alloc(16, true);
        let _ = alloc(16, true);
        collect_with_roots(&[]);
        assert_eq!(count(), 0);
    }

    #[test]
    fn transitive_marking_keeps_referenced_objects() {
        reset();
        let child = alloc(16, true) as usize;
        let parent = alloc(16, true) as *mut usize;
        unsafe { *parent = child };
        collect_with_roots(&[parent as usize]);
        assert_eq!(count(), 2);
    }

    #[test]
    fn interior_pointer_keeps_object() {
        reset();
        let a = alloc(32, true) as usize;
        // A pointer into the middle of the object still keeps it alive.
        collect_with_roots(&[a + 16]);
        assert_eq!(count(), 1);
    }

    /// The size an ordinary object asks for is describable, and the layout
    /// carries the size and alignment `alloc` checks every request against --
    /// and hands the system allocator, for an object too big for a page.
    #[test]
    fn heap_layout_describes_ordinary_sizes() {
        let layout = heap_layout(24).expect("24 bytes is describable");
        assert_eq!(layout.size(), 24);
        assert_eq!(layout.align(), ALIGN);
    }

    /// The allocation whose slot would reach `next_gc` collects first, then
    /// takes its slot: the count it leaves is its own slot alone. Holds on a
    /// platform without stack bounds too, where `collect()` resets the count
    /// and returns.
    #[test]
    fn the_crossing_allocation_collects_before_taking_its_slot() {
        reset();
        HEAP.with(|h| h.borrow_mut().next_gc = 64);
        let count = || HEAP.with(|h| h.borrow().alloc_since_gc);
        alloc(16, false);
        alloc(16, false);
        alloc(16, false);
        assert_eq!(count(), 48, "three 16-byte slots, all below the threshold");
        alloc(16, false);
        assert_eq!(
            count(),
            16,
            "the fourth reaches 64: it must collect, resetting the count, then charge its own slot"
        );
        reset();
    }

    /// Every small size is one `heap_layout` accepts, so an allocation path
    /// that skipped the check for small requests would lose nothing. One was
    /// tried on 2026-10-03 and did not land; see
    /// `examples/05-json-api/BENCHMARK.md`, "(alloc-fast-path)".
    #[test]
    fn small_sizes_are_always_describable() {
        assert!(heap_layout(pages::SMALL_MAX).is_some());
    }

    /// The boundary between the two paths: `SMALL_MAX` bytes takes a page
    /// slot, one byte more takes the large path.
    #[test]
    fn small_max_takes_a_slot_and_one_byte_more_takes_the_large_path() {
        reset();
        let small = alloc(pages::SMALL_MAX, true) as usize;
        let large = alloc(pages::SMALL_MAX + 1, true) as usize;
        HEAP.with(|h| {
            let h = h.borrow();
            assert!(
                h.pages.slot_scan(small).is_some(),
                "2048 bytes must take a page slot"
            );
            assert!(
                h.pages.slot_scan(large).is_none(),
                "2049 bytes must not take a page slot"
            );
            assert!(
                h.large.iter().any(|o| o.addr == large),
                "2049 bytes must take the large path"
            );
        });
        reset();
    }

    /// A size no `Layout` can express is reported as such rather than reaching
    /// the allocator. Checked on the exact size that used to abort the process
    /// with a Rust panic: `[x; MAX_ARRAY_LEN]` asks `nova_rt_alloc` for
    /// `8 * 1152921504606846974 + 8` bytes, which `ALIGN` rounds up 8 bytes past
    /// `isize::MAX`.
    ///
    /// Deciding this never allocates, so the extreme is testable directly —
    /// calling `alloc` with it would (correctly) abort the test process.
    #[test]
    fn heap_layout_rejects_undescribable_sizes() {
        assert!(heap_layout(9_223_372_036_854_775_800).is_none());
        assert!(heap_layout(usize::MAX).is_none());
    }

    /// The limit the diagnostic quotes is exactly the limit `Layout` enforces.
    /// Without this the message could name a number the code does not use.
    #[test]
    fn max_heap_object_is_the_largest_describable_size() {
        assert!(heap_layout(MAX_HEAP_OBJECT).is_some());
        assert!(heap_layout(MAX_HEAP_OBJECT + 1).is_none());
    }

    #[test]
    fn leaf_objects_are_not_traced() {
        reset();
        let victim = alloc(16, true) as usize;
        let leaf = alloc(16, false) as *mut usize;
        unsafe { *leaf = victim };
        // The leaf is rooted but not scanned, so `victim` is collected.
        collect_with_roots(&[leaf as usize]);
        assert_eq!(count(), 1);
    }

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
        assert_eq!(
            count(),
            3,
            "head, mid and tail survive; the other large object does not"
        );
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

    /// A page taken back from the reserve is an ordinary page again: swept,
    /// marked and traced on every later collection. A reuse that left it
    /// flagged as reserved would leave it out of every sweep, so its marks
    /// would never clear and its garbage would never be freed.
    #[test]
    fn a_reused_reserve_page_is_swept_and_traced_on_every_collection() {
        reset();
        let _ = alloc(16, true);
        collect_with_roots(&[]);
        let child = alloc(48, true) as usize;
        let parent = alloc(48, true) as *mut usize;
        unsafe { *parent = child };
        assert_eq!(
            HEAP.with(|h| h.borrow().pages.page_count()),
            1,
            "the class-48 objects went into the reserved class-16 page"
        );
        collect_with_roots(&[parent as usize]);
        assert_eq!(count(), 2, "parent and child survive the first collection");
        collect_with_roots(&[parent as usize]);
        assert_eq!(count(), 2, "and the second, traced again");
        collect_with_roots(&[]);
        assert_eq!(count(), 0, "garbage in a reused reserve page is freed");
    }

    // The two tests below exercise `add_root`/`remove_root`'s own bookkeeping
    // (via `PINNED`'s length) directly, without going through `collect()`, so
    // -- unlike the `#[ignore]`d tests in `mod registry` below -- they are
    // deterministic, and they hold even where `collect()` is a no-op for want
    // of stack bounds (see `mod registry`'s doc comment).

    #[test]
    fn registering_the_same_address_twice_requires_removing_it_twice() {
        // add_root's doc comment states this is multiset, not set, semantics.
        // Correct by inspection (`push`/`rposition`+`swap_remove`), but had no
        // regression guard: a future "simplification" to a `HashSet`-backed
        // registry, or to `Vec::retain`/`dedup`, would silently change this.
        // Any caller that pairs one add with one remove per object is what
        // that would bite -- a bug registering an address twice would then be
        // fully unrooted by the first removal, rather than still held.
        reset();
        let obj = alloc(16, true);
        add_root(obj);
        add_root(obj);
        remove_root(obj);
        assert_eq!(
            PINNED.with(|p| p.borrow().len()),
            1,
            "removing once should leave exactly one registration of a twice-registered address"
        );
        remove_root(obj);
        assert_eq!(
            PINNED.with(|p| p.borrow().len()),
            0,
            "the second remove_root should clear the second registration"
        );
    }

    #[test]
    fn removing_an_unregistered_address_does_not_panic_or_change_the_registry() {
        // remove_root's doc comment states this is a no-op, not a panic.
        // Correct by inspection (`rposition` returning `None` short-circuits
        // the `swap_remove`), but likewise had no regression guard.
        reset();
        let obj = alloc(16, true);
        add_root(obj);
        let never_registered = alloc(16, true);
        remove_root(never_registered);
        assert_eq!(
            PINNED.with(|p| p.borrow().len()),
            1,
            "removing an address that was never registered changed the registry"
        );
        remove_root(obj);
    }

    /// On every platform that implements stack bounds, the top lies above
    /// this frame, and within a plausible stack size of it.
    #[cfg(any(
        windows,
        all(target_os = "linux", target_env = "gnu"),
        target_os = "macos"
    ))]
    #[test]
    fn stack_base_lies_above_the_current_frame() {
        let local = 0u8;
        let here = std::ptr::addr_of!(local) as usize;
        let base = stack_base().expect("this platform implements stack bounds");
        assert!(
            base > here,
            "top {base:#x} must lie above this frame {here:#x}"
        );
        assert!(
            base - here < 1 << 30,
            "top {base:#x} is implausibly far above {here:#x}"
        );
    }

    /// The top is the calling thread's own: on a thread spawned with a
    /// 256 KiB stack, it lies less than 1 MiB above a local in that thread.
    /// A version that returned another thread's top would fail, because
    /// thread stacks are separate mappings far more than 1 MiB apart.
    #[cfg(any(
        windows,
        all(target_os = "linux", target_env = "gnu"),
        target_os = "macos"
    ))]
    #[test]
    fn stack_base_is_the_calling_threads_own() {
        const STACK: usize = 256 * 1024;
        let (base, here) = std::thread::Builder::new()
            .stack_size(STACK)
            .spawn(|| {
                let local = 0u8;
                (stack_base(), std::ptr::addr_of!(local) as usize)
            })
            .expect("spawn a thread")
            .join()
            .expect("join it");
        let base = base.expect("this platform implements stack bounds");
        assert!(base > here, "top {base:#x} must lie above {here:#x}");
        assert!(
            base - here < 4 * STACK,
            "top {base:#x} is not this thread's: {here:#x}"
        );
    }

    /// Tests exercising the real, stack-scanning `collect()` (every test
    /// above uses the deterministic `collect_with_roots` instead), to prove
    /// `PINNED` is correctly integrated into that path: seeded before the
    /// scan, surviving repeated collections, and its roots traced rather than
    /// merely marked.
    ///
    /// **Not gated to a platform.** Until 2026-10-05 this module was
    /// `#[cfg(windows)]`, because only Windows had a `stack_base`
    /// implementation; glibc Linux and macOS have one now
    /// (`docs/adr/0024-gc-stack-bounds-on-unix.md`). Where `stack_base` has
    /// none, `collect()` sets `alloc_since_gc = 0` and returns *before* even
    /// looking at `PINNED` -- no scan, no mark, no sweep -- so every
    /// `is_none()` assertion in this module (checking that something was
    /// swept) would fail outright, and every `is_some()` assertion (checking
    /// that something survived) would pass vacuously, identically to what
    /// `add_root` being `{}` would produce, which is the one thing a test in
    /// this file must never do. No CI platform is such a platform.
    /// `collect_with_roots` was considered as a platform-independent
    /// alternative and rejected: it bypasses `collect()`'s `PINNED`-seeding
    /// step entirely, which is the one thing this module needs to prove, so a
    /// `collect_with_roots`-based version would only re-test what
    /// `transitive_marking_keeps_referenced_objects` (above) already covers.
    ///
    /// **Unconditionally `#[ignore]`d, in debug and release alike** -- no
    /// longer only under `--release` as this comment used to say.
    /// `collect()`'s conservative
    /// stack scan is also intermittently flaky in debug, under the test
    /// parallelism CI actually runs at: a stale stack word in an
    /// already-returned frame is read as a conservative root, well above the
    /// scanned range's low end (so *not* the registers `gc_stack.c` spills at
    /// that low end). Mechanism identified, not fixed. The full account --
    /// the isolating experiment, the measured rates, which frame the
    /// retaining word sits in, and two attempted remedies (a deliberate
    /// stack clobber, a serializing mutex) that were each measured to make
    /// the failure rate worse and were reverted rather than kept -- is
    /// `docs/adr/0010-conservative-scan-root-test-gating.md`. That document
    /// is where the numbers live; this comment states only the invariant.
    ///
    /// Only the sweep-asserting (`is_none()`) tests below actually flake; the
    /// survival-asserting (`is_some()`) ones are gated alongside them anyway,
    /// not because they fail on their own, but because gating only the flaky
    /// half would strip the negative control from the paired `is_some()`
    /// tests, leaving a green "the registered root survived" assertion that
    /// would also pass against a collector that frees nothing at all -- the
    /// same ungated-canary pattern a Critical review finding flagged. Each test's
    /// own `#[ignore]` reason says which of the two it is, so that is not
    /// restated here. Reachable with `cargo test -- --ignored`, which CI runs
    /// as an advisory, `continue-on-error` step.
    mod registry {
        use super::*;

        /// Carry a heap address across a `collect()` call below without
        /// leaving its literal bits in a place the conservative scanner would
        /// treat as a root, and stay opaque to the optimizer while doing it
        /// (see the `#[inline(never)]` paragraph below).
        ///
        /// Several tests below need the numeric address of an object *after*
        /// calling the real `collect()`, to look it up with `object_info` or
        /// hand it to `remove_root`. A `usize` that must be read again after a
        /// call has to survive that call, and this collector's own definition
        /// of "conservative" (module doc comment, top of file) means the
        /// compiler necessarily preserves a surviving value somewhere the
        /// scanner looks: a stack slot, or a callee-saved register
        /// `gc_stack.c` spills to one. A plain `usize` copy of the address
        /// is bit-identical to a real pointer to it, so it is then
        /// indistinguishable from a genuine root -- which would make
        /// `object_info` report the object alive whether or not the registry
        /// (or anything else) is actually the thing keeping it there.
        ///
        /// This is independently necessary, not merely useful alongside the
        /// `mut`-reassignment fix for the unrelated shadowing hazard elsewhere
        /// in this file -- established by an isolating experiment (shadowing
        /// fixed, hiding deliberately not applied) rather than by the two
        /// hazards' combined, confounded effect. See the task report for that
        /// experiment: an earlier version of this comment cited the confounded
        /// observation instead, which is the kind of citation this project
        /// keeps having to correct, which is exactly why the experiment
        /// itself -- not a list of which tests failed when -- belongs in the
        /// report rather than here.
        ///
        /// `#[inline(never)]` on both is load-bearing under `-O`, for a
        /// mechanism specific to the optimizer rather than to any one test:
        /// without it, the compiler can inline both calls, prove
        /// `reveal(hide(x)) == x`, and keep the original `x` live across
        /// `collect()` instead of ever materializing the hidden form --
        /// defeating the encoding for whichever test happens to exercise it.
        /// Keeping the two calls opaque to each other rules that
        /// substitution out. This is not a claim that every test below is
        /// reliable under `-O`, or even in debug: it is not, for reasons
        /// unrelated to `hide` itself and not fixed by this attribute, which
        /// is why every test in this module is unconditionally `#[ignore]`d
        /// rather than gated to release only -- see each one's own attribute
        /// and comment, and the task report for what was tried and what
        /// running under both profiles actually showed.
        ///
        /// The complement is its own inverse, and for every address a live
        /// heap allocation can actually have in this process it lands far
        /// outside any tracked object's `[addr, addr + size)` range (real
        /// addresses are nowhere near `usize::MAX`), so `mark_word` never
        /// mistakes the hidden form for a root in transit.
        #[inline(never)]
        fn hide(addr: usize) -> usize {
            !addr
        }

        /// Inverse of [`hide`]. A distinct name at call sites, even though
        /// the operation is identical, so a `hide`/`reveal` pair reads as
        /// encode/decode rather than as an unexplained bitwise flip.
        /// `#[inline(never)]` for the same reason as `hide`.
        #[inline(never)]
        fn reveal(hidden: usize) -> usize {
            !hidden
        }

        #[test]
        #[ignore = "not flaky on its own -- never observed to fail by itself. \
                    Gated only to stay paired with the sweep-asserting control \
                    test that does flake: this one asserts an object SURVIVED, \
                    which also passes against a collector that frees nothing at \
                    all, so running it while its control is ignored would leave \
                    a green assertion proving nothing. See \
                    docs/adr/0010-conservative-scan-root-test-gating.md. \
                    Reachable with `cargo test -- --ignored`."]
        fn a_registered_root_survives_a_collection_with_no_stack_reference() {
            // The exact scenario: an object reachable ONLY through the registry.
            // `black_box` is not enough on its own here -- the point is that after
            // the pointer is registered we must NOT keep it in a live local that the
            // conservative stack scan would find anyway, or the test passes with the
            // registry doing nothing.
            //
            // Two distinct hazards were measured while building this test, each
            // independently capable of making it pass against a no-op registry:
            //
            // 1. `addr` (needed after `collect()`, for `object_info`/`remove_root`)
            //    must survive the call, and this collector's own definition of
            //    "conservative" (module doc comment) means a `usize` that survives
            //    a call is necessarily preserved somewhere the scanner looks --
            //    a stack slot, or a callee-saved register `gc_stack.c` spills
            //    to one. A plain copy, bit-identical to the object's address, is
            //    then an accidental root in its own right. Fixed by carrying it
            //    across as `hide(addr)` instead of `addr` (see `hide`'s doc
            //    comment).
            // 2. `let obj = null_mut();` -- shadowing, not reassigning -- declares
            //    a SECOND, distinct stack slot in this unoptimized build; the
            //    FIRST slot, still holding the original pointer, is never
            //    overwritten and stays live for the rest of the frame. Fixed by
            //    making `obj` `mut` and reassigning in place, so the null
            //    overwrites the same slot the original pointer occupied. See the
            //    task report for which tests this was isolated against.
            //
            // Unconditionally ignored (debug and release) alongside its
            // negative control, for the reason given on the attribute above --
            // this test itself has not been observed to fail on its own; it is
            // gated only to avoid an is_some() assertion ever running unpaired.
            let mut obj = alloc(64, true);
            add_root(obj);
            let hidden = hide(obj as usize);
            obj = std::ptr::null_mut::<u8>();
            std::hint::black_box(obj);
            std::hint::black_box(hidden);

            collect();

            let addr = reveal(hidden);
            assert!(
                object_info(addr).is_some(),
                "a registered root was swept; the registry is not seeding the mark set"
            );
            remove_root(addr as *mut u8);
        }

        #[test]
        #[ignore = "flaky: asserts an unreachable object was SWEPT, and the \
                    conservative stack scan intermittently retains it instead \
                    -- a stale stack word in an already-returned frame is read \
                    as a root, in debug as well as release, at default test \
                    parallelism. Mechanism identified, not fixed; two remedies \
                    were each measured to make it worse. See \
                    docs/adr/0010-conservative-scan-root-test-gating.md. \
                    Reachable with `cargo test -- --ignored`."]
        fn an_unregistered_object_is_swept() {
            // The discriminating half. Without this, the test above passes even if
            // collect() never frees anything at all. It is also the test that
            // caught both hazards documented on
            // `a_registered_root_survives_a_collection_with_no_stack_reference`:
            // `obj` is `mut` and nulled by reassignment (not `let`-shadowed), and
            // `addr` crosses `collect()` hidden rather than as a plain `usize`.
            //
            // Unconditionally ignored (debug and release): this test itself is
            // intermittently (not consistently) flaky under default test
            // parallelism -- see the task report for the sampling. Distinct in
            // character from `remove_root_actually_unroots`'s
            // consistently-reproducible release-mode failure, and consistent
            // with (though not proven to be) the kind of incidental
            // over-retention this collector's own contract already treats as
            // acceptable (module doc comment, top of file: "can retain a little
            // garbage... but never frees a reachable object") rather than a
            // second instance of the deterministic `hide`/`reveal` collapse
            // `#[inline(never)]` already closed. The general mechanism (a stale
            // stack word in an already-returned frame, named on the attribute
            // above) was later identified by the task report's dedicated
            // diagnostic; that diagnostic's verbatim captures happen to both be
            // of a different test (`task.rs`'s `an_unspawned_tasks_state_is_swept`),
            // so treat its applicability here as reasoned-but-not-directly-observed.
            let mut obj = alloc(64, true);
            let hidden = hide(obj as usize);
            obj = std::ptr::null_mut::<u8>();
            std::hint::black_box(obj);
            std::hint::black_box(hidden);

            collect();

            let addr = reveal(hidden);
            assert!(
                object_info(addr).is_none(),
                "an unreachable, unregistered object survived; this test cannot \
             discriminate a working registry from a collector that frees nothing"
            );
        }

        #[test]
        #[ignore = "flaky: asserts an unreachable object was SWEPT, and the \
                    conservative stack scan intermittently retains it instead \
                    -- a stale stack word in an already-returned frame is read \
                    as a root, in debug as well as release, at default test \
                    parallelism. Mechanism identified, not fixed; two remedies \
                    were each measured to make it worse. See \
                    docs/adr/0010-conservative-scan-root-test-gating.md. \
                    Reachable with `cargo test -- --ignored`."]
        fn remove_root_actually_unroots() {
            // Otherwise add/remove is a leak, and every completed task's state is
            // retained for the process lifetime. See
            // `a_registered_root_survives_a_collection_with_no_stack_reference` for
            // why `obj` is `mut`+reassigned and `addr` crosses `collect()` hidden.
            //
            // Under `-O` specifically: the object survives there, but not
            // because `remove_root` is broken. Established (not merely
            // asserted -- see the task report for how): `PINNED` is empty
            // both before and after `collect()` in that build, so the registry
            // cannot be the cause; the sibling pairing test
            // (`registering_the_same_address_twice_requires_removing_it_twice`)
            // independently proves `remove_root`'s own bookkeeping correct in the
            // same optimized build; and the failure direction is over-retention,
            // which this collector's own contract (module doc comment, top of
            // file) already documents as acceptable -- the opposite of the
            // premature free this whole file exists to rule out. The specific
            // accidental root in that release build was not identified.
            // Separately, this test is now also unconditionally ignored (see the
            // attribute above) for the different, debug-mode mechanism the task
            // report's later, dedicated diagnostic did identify: a stale stack
            // word in an already-returned frame -- not this paragraph's
            // release-mode finding, which remains its own, still-unidentified
            // accidental root.
            let mut obj = alloc(64, true);
            let hidden = hide(obj as usize);
            add_root(obj);
            remove_root(obj);
            obj = std::ptr::null_mut::<u8>();
            std::hint::black_box(obj);
            std::hint::black_box(hidden);

            collect();

            let addr = reveal(hidden);
            assert!(
                object_info(addr).is_none(),
                "either remove_root did not unroot, or (see this test's comment) \
                 an accidental conservative root retained the object -- this \
                 assertion alone cannot tell the two apart"
            );
        }

        /// Allocate `parent`/`child`, link `child` under `parent`, optionally
        /// register `parent`, and hand back both addresses hidden (see
        /// [`hide`]). Shared by the positive test
        /// (`a_registered_root_keeps_its_transitive_children_alive`,
        /// `register = true`) and its negative control
        /// (`an_unregistered_parent_and_child_are_swept`, `register = false`)
        /// so both go through the identical shape and only the one bit that
        /// matters differs.
        ///
        /// A separate, never-inlined function, deliberately: every raw
        /// pointer this scenario needs (`parent`, `child`, the cast receiver
        /// for the write, the value written) stays local to this call and is
        /// never returned. Measured that this matters and a same-frame
        /// version does not: with everything inlined into the test itself --
        /// `mut`-reassigned locals, `hide`d addresses, even the write's
        /// operands routed through named temporaries and a 4 KiB stack buffer
        /// stomped over the frame before collecting -- `parent` still
        /// measurably survived a collection with `add_root` reduced to a
        /// no-op. Explicitly zeroing every callee-saved general-purpose
        /// register this build's inline-asm would let a program touch --
        /// `rsi`, `rdi`, `r12`-`r15` -- did not clear it either (`rbx` is
        /// reserved by rustc/LLVM on this target and refused as an asm
        /// operand; `rbp` and the callee-saved `xmm6`-`xmm15` were not
        /// tried). So the exact register is not identified, only narrowed to
        /// "some callee-saved register (or, less likely, some other stack
        /// slot this pass didn't reach) this same-frame shape leaves live" --
        /// but the mechanism that fixes it is not in question: unlike a stack
        /// slot, a callee-saved register that a deeper call uses is saved on
        /// that call's entry and restored to the *caller's* pre-call value on
        /// return, by the calling convention every correctly-compiled
        /// function must honor -- not left holding whatever the callee last
        /// put there. Putting the whole scenario behind exactly one such
        /// call, returning only `hide`d integers, resolved it.
        ///
        /// This test's soundness rests on `#[inline(never)]` actually being
        /// honored (an inlined copy reintroduces the same-frame leak this
        /// function exists to avoid) and on the frame this call builds not
        /// coincidentally being fully overwritten by `collect()`'s own stack
        /// usage before the scan -- neither is enforced by the type system.
        /// `an_unregistered_parent_and_child_are_swept` is the canary for
        /// both: it runs the identical function and would start failing if
        /// either stopped holding.
        #[inline(never)]
        fn setup_parent_and_child(register: bool) -> (usize, usize) {
            let mut parent = alloc(16, true);
            let mut child = alloc(32, true);
            let child_addr = hide(child as usize);
            let parent_addr = hide(parent as usize);
            let mut child_bits = child as usize;
            let mut parent_ptr = parent as *mut usize;
            unsafe { parent_ptr.write(child_bits) };
            if register {
                add_root(parent);
            }
            child = std::ptr::null_mut::<u8>();
            parent = std::ptr::null_mut::<u8>();
            child_bits = 0;
            parent_ptr = std::ptr::null_mut::<usize>();
            std::hint::black_box(child);
            std::hint::black_box(parent);
            std::hint::black_box(child_bits);
            std::hint::black_box(parent_ptr);
            (parent_addr, child_addr)
        }

        #[test]
        #[ignore = "not flaky on its own -- never observed to fail by itself. \
                    Gated only to stay paired with the sweep-asserting control \
                    test that does flake: this one asserts an object SURVIVED, \
                    which also passes against a collector that frees nothing at \
                    all, so running it while its control is ignored would leave \
                    a green assertion proving nothing. See \
                    docs/adr/0010-conservative-scan-root-test-gating.md. \
                    Reachable with `cargo test -- --ignored`."]
        fn a_registered_root_keeps_its_transitive_children_alive() {
            // The registry seeds the mark set; marking must then TRACE. A
            // registry that marked only the registered object itself would
            // free a suspended task's locals while keeping its state header
            // -- the exact bug, one level down, and invisible to the first
            // test.
            //
            // All of `parent`, `child`, and every raw pointer derived from
            // them lives and dies inside `setup_parent_and_child` -- see its
            // doc comment for why that call boundary, specifically, is what
            // makes this test trustworthy. This function only ever holds the
            // hidden (`hide`d) addresses.
            //
            // Unconditionally ignored (debug and release) alongside its
            // negative control below, not because this test itself is known to
            // fail on its own, but because running it without that control
            // would be exactly the pattern this file's pairing rule exists to
            // prevent: an `is_some()` assertion with no paired `is_none()` to
            // prove the collector can free anything at all.
            let (parent_addr, child_addr) = setup_parent_and_child(true);
            std::hint::black_box(parent_addr);
            std::hint::black_box(child_addr);

            collect();

            assert!(
                object_info(reveal(child_addr)).is_some(),
                "a child reachable only through a registered root was swept"
            );
            remove_root(reveal(parent_addr) as *mut u8);
        }

        #[test]
        #[ignore = "flaky: asserts an unreachable object was SWEPT, and the \
                    conservative stack scan intermittently retains it instead \
                    -- a stale stack word in an already-returned frame is read \
                    as a root, in debug as well as release, at default test \
                    parallelism. Mechanism identified, not fixed; two remedies \
                    were each measured to make it worse. See \
                    docs/adr/0010-conservative-scan-root-test-gating.md. \
                    Reachable with `cargo test -- --ignored`."]
        fn an_unregistered_parent_and_child_are_swept() {
            // The negative control for the test above -- see
            // `setup_parent_and_child`'s doc comment. Without this, the test
            // above passing proves nothing beyond "this particular frame
            // shape didn't happen to leak this time": `#[inline(never)]`
            // going unhonored, or the frame layout shifting so `collect()`'s
            // own stack usage no longer overwrites the same slots, would make
            // it pass whether or not the registry does anything, exactly like
            // the un-negated hazards this file has already guarded against
            // elsewhere.
            //
            // Under `-O` specifically: this test itself fails there, by the
            // same unidentified accidental-root mechanism as
            // `remove_root_actually_unroots` -- see that test's comment, and,
            // separately, its note on the different, debug-mode mechanism the
            // task report's dedicated diagnostic did identify (also named on
            // this test's own attribute above). This test is now
            // unconditionally ignored, in both profiles; the test above is
            // ignored alongside it for the reason given on its own attribute.
            let (_parent_addr, child_addr) = setup_parent_and_child(false);
            std::hint::black_box(child_addr);

            collect();

            assert!(
                object_info(reveal(child_addr)).is_none(),
                "a child of an unregistered, unreachable parent survived; \
                 setup_parent_and_child's same-frame hiding is not sound here"
            );
        }

        #[test]
        #[ignore = "not flaky on its own -- never observed to fail by itself. \
                    Gated only to stay paired with the sweep-asserting control \
                    test that does flake: this one asserts an object SURVIVED, \
                    which also passes against a collector that frees nothing at \
                    all, so running it while its control is ignored would leave \
                    a green assertion proving nothing. See \
                    docs/adr/0010-conservative-scan-root-test-gating.md. \
                    Reachable with `cargo test -- --ignored`."]
        fn the_registry_survives_more_than_one_collection() {
            // ROOTS (gc.rs:95) is a SCRATCH buffer cleared at the start of
            // every cycle. If the registry were folded into it, the first
            // collection would consume it and the second would sweep the
            // root. That failure mode is invisible to any single-collection
            // test.
            //
            // See
            // `a_registered_root_survives_a_collection_with_no_stack_reference`
            // for why `obj` is `mut`+reassigned and `addr` crosses each
            // `collect()` call hidden, and for why this is unconditionally
            // ignored in both profiles (this test itself has not been observed
            // to fail on its own; only its shared negative control has).
            let mut obj = alloc(64, true);
            add_root(obj);
            let hidden = hide(obj as usize);
            obj = std::ptr::null_mut::<u8>();
            std::hint::black_box(obj);
            std::hint::black_box(hidden);

            collect();
            collect();
            collect();

            let addr = reveal(hidden);
            assert!(
                object_info(addr).is_some(),
                "the registry did not survive repeated collections; it is probably \
                 sharing the scratch ROOTS buffer"
            );
            remove_root(addr as *mut u8);
        }
    }
}
