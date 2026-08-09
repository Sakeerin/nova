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
//! - **callee-saved registers**, flushed onto the stack by the `setjmp` shim in
//!   `gc_stack.c` (caller-saved registers hold no live root at a call boundary);
//! - **the stack**, scanned from the current frame up to the thread's base;
//! - **explicitly registered roots** ([`add_root`]/[`remove_root`]), for
//!   objects reachable from neither: a suspended async task's state is owned
//!   by the Rust executor while the task is parked, on no Nova stack and in
//!   no register, so it must be pinned by address instead.
//!
//! Marking is range-based, so interior pointers (e.g. an array-element address
//! held transiently) keep their containing object alive. Objects flagged
//! `scan = false` (string byte buffers) are leaves and are not traced.
//!
//! **A sweep really frees, so an address is not a durable identity for an
//! object.** An unmarked object's memory goes back to the system allocator
//! (`dealloc`) rather than into an arena this module keeps, so a later [`alloc`]
//! can hand the same address out for a wholly unrelated object. An address
//! therefore names an object only while that object is live. [`alloc`] starts an
//! address meaning what it means; the sweep in [`collect_with_roots`] ends it,
//! and so does the `#[cfg(test)]` `reset`, which is the only other `dealloc` in
//! this module. Every path that ends an address's meaning must tell the
//! executor's state-address lookup, so that "a freed address is always
//! forgotten" has no exception — a third such path would have to do the same.
//! This is the one place that property is stated, so that a reader who needs it
//! has somewhere to be pointed at instead of a copy to compare.
//!
//! Precise stack bounds are currently only implemented on Windows; on other
//! platforms collection is skipped (allocations leak, as before — never
//! unsafe).

use std::alloc::{alloc_zeroed, dealloc, handle_alloc_error, Layout};
use std::cell::RefCell;
use std::ffi::c_void;
use std::sync::OnceLock;

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

/// One live allocation the collector tracks.
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
    objects: Vec<Obj>,
    alloc_since_gc: usize,
    next_gc: usize,
    live_bytes: usize,
    /// Thread stack base (highest address); `0` = not captured, `usize::MAX` =
    /// this platform is unsupported (collection disabled).
    base: usize,
    collections: u64,
    freed_bytes: u64,
}

impl Heap {
    const fn new() -> Self {
        Heap {
            objects: Vec::new(),
            alloc_since_gc: 0,
            next_gc: INITIAL_THRESHOLD,
            live_bytes: 0,
            base: 0,
            collections: 0,
            freed_bytes: 0,
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
    maybe_collect(size);
    // Zeroed so unwritten slots (e.g. skipped unit fields) read as null and are
    // never mistaken for pointers.
    let p = unsafe { alloc_zeroed(layout) };
    if p.is_null() {
        handle_alloc_error(layout);
    }
    HEAP.with(|h| {
        let mut h = h.borrow_mut();
        h.objects.push(Obj {
            addr: p as usize,
            size,
            scan,
            marked: false,
        });
        h.alloc_since_gc += size;
        h.live_bytes += size;
    });
    p
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
/// all — it only changes GC behaviour). This reaches into the tracked `Obj`
/// directly so a caller can assert the exact size and scan flag `alloc` was
/// given, not just what got written.
#[cfg(test)]
pub(crate) fn object_info(addr: usize) -> Option<(usize, bool)> {
    HEAP.with(|h| {
        h.borrow()
            .objects
            .iter()
            .find(|o| o.addr == addr)
            .map(|o| (o.size, o.scan))
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
/// platform-independent, where a `collect()`-based assertion is neither.
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
/// stack scan -- which has an implementation on Windows alone, and which
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
    // Capture the stack base once; give up (leak) on unsupported platforms.
    let base = HEAP.with(|h| {
        let mut h = h.borrow_mut();
        if h.base == 0 {
            h.base = stack_base().unwrap_or(usize::MAX);
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
        let mut h = h.borrow_mut();
        for o in &mut h.objects {
            o.marked = false;
        }
        // Sorted (start, end, object-index) for range lookup during marking.
        let mut index: Vec<(usize, usize, usize)> = h
            .objects
            .iter()
            .enumerate()
            .map(|(i, o)| (o.addr, o.addr + o.size, i))
            .collect();
        index.sort_unstable_by_key(|e| e.0);

        let mut work: Vec<usize> = Vec::new();
        for &w in roots {
            mark_word(w, &index, &mut h.objects, &mut work);
        }
        while let Some(oi) = work.pop() {
            let (addr, size, scan) = {
                let o = &h.objects[oi];
                (o.addr, o.size, o.scan)
            };
            if !scan {
                continue;
            }
            let mut p = addr;
            let end = addr + size;
            while p + 8 <= end {
                // SAFETY: [addr, end) is a live allocation this collector owns.
                let w = unsafe { *(p as *const usize) };
                mark_word(w, &index, &mut h.objects, &mut work);
                p += 8;
            }
        }

        // Sweep: free unmarked objects.
        let mut freed = 0usize;
        let mut i = 0;
        while i < h.objects.len() {
            if h.objects[i].marked {
                i += 1;
            } else {
                let o = h.objects.swap_remove(i);
                // Infallible, and not a user-input path: a tracked object's
                // size is one `alloc` already built a layout from.
                let layout = heap_layout(o.size)
                    .expect("a live object's size was accepted by heap_layout at allocation");
                // SAFETY: `addr`/`size` are from this object's own allocation.
                unsafe { dealloc(o.addr as *mut u8, layout) };
                // This address stops naming this object here (module doc
                // comment), and the executor keys a lookup on state-object
                // addresses. `task::forget_freed_state` states what that
                // requires and why this is the only place it can be
                // maintained; this call site deliberately does not restate it.
                //
                // Called with `HEAP` borrowed, which it must be able to
                // tolerate: it touches nothing this module owns -- it borrows
                // one unrelated thread-local and allocates nothing through
                // `alloc` -- so it cannot re-enter this collector or this
                // borrow.
                crate::task::forget_freed_state(o.addr);
                freed += o.size;
                // `swap_remove` moved a new element into `i`; re-check it.
            }
        }

        h.freed_bytes += freed as u64;
        h.live_bytes = h.live_bytes.saturating_sub(freed);
        h.alloc_since_gc = 0;
        h.collections += 1;
        h.next_gc = std::cmp::max(INITIAL_THRESHOLD, h.live_bytes.saturating_mul(2));
        if debug() {
            eprintln!(
                "nova-gc: collection {} freed {freed} bytes, {} objects live ({} bytes)",
                h.collections,
                h.objects.len(),
                h.live_bytes,
            );
        }
    });
}

/// Mark the object containing `w` (if any) and queue it for tracing.
fn mark_word(
    w: usize,
    index: &[(usize, usize, usize)],
    objects: &mut [Obj],
    work: &mut Vec<usize>,
) {
    if w == 0 {
        return;
    }
    // The only candidate is the object with the largest start <= w.
    let pos = index.partition_point(|e| e.0 <= w);
    if pos == 0 {
        return;
    }
    let (_start, end, oi) = index[pos - 1];
    if w < end && !objects[oi].marked {
        objects[oi].marked = true;
        work.push(oi);
    }
}

/// Push every aligned machine word in `[lo, hi)` as a candidate root. Called
/// from the `setjmp` shim with the register buffer and stack range.
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

#[cfg(not(windows))]
fn stack_base() -> Option<usize> {
    // Precise stack bounds for non-Windows platforms are a follow-up; until
    // then collection is skipped there.
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn reset() {
        HEAP.with(|h| {
            let mut h = h.borrow_mut();
            for o in h.objects.drain(..) {
                let layout = heap_layout(o.size).expect("tracked size is describable");
                unsafe { dealloc(o.addr as *mut u8, layout) };
                // The other place an address stops naming its object, and so
                // the other place the executor's state-address lookup has to
                // be told -- see the sweep in `collect_with_roots`. Here so
                // that "a freed address is always forgotten" has no exception,
                // this test-only path included.
                crate::task::forget_freed_state(o.addr);
            }
            h.alloc_since_gc = 0;
            h.live_bytes = 0;
            h.next_gc = INITIAL_THRESHOLD;
        });
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

    fn count() -> usize {
        HEAP.with(|h| h.borrow().objects.len())
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
    /// carries the size and alignment `alloc` will hand the system allocator.
    #[test]
    fn heap_layout_describes_ordinary_sizes() {
        let layout = heap_layout(24).expect("24 bytes is describable");
        assert_eq!(layout.size(), 24);
        assert_eq!(layout.align(), ALIGN);
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

    // The two tests below exercise `add_root`/`remove_root`'s own bookkeeping
    // (via `PINNED`'s length) directly, without going through `collect()`, so
    // -- unlike the tests in `mod registry` below -- they run on every
    // platform, including the two (Linux, macOS) where `collect()` itself is
    // a no-op (see `mod registry`'s doc comment).

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

    /// Tests exercising the real, stack-scanning `collect()` (every test
    /// above uses the deterministic `collect_with_roots` instead), to prove
    /// `PINNED` is correctly integrated into that path: seeded before the
    /// scan, surviving repeated collections, and its roots traced rather than
    /// merely marked.
    ///
    /// **Gated to Windows.** `stack_base` (Windows implementation at `:419`;
    /// the `#[cfg(not(windows))]` stub returning `None` is at `:432`) only
    /// implements precise stack bounds on Windows; off it, `collect()`
    /// (`:264`, early return at `:273-275`) sets `alloc_since_gc = 0` and
    /// returns *before* even looking at `PINNED` -- no scan, no mark, no
    /// sweep, on any platform this collector doesn't yet support. Off
    /// Windows, every `is_none()` assertion in this module (checking that
    /// something was swept) would fail outright, and every `is_some()`
    /// assertion (checking that something survived) would pass vacuously --
    /// identically to what `add_root` being `{}` would produce, which is the
    /// one thing a test in this file must never do. This is derived by
    /// inspection of `stack_base`/`collect()` above, not run on Linux or
    /// macOS; it does not need to be, since the mechanism (`collect()` never
    /// reaching `PINNED`) applies uniformly to every test below regardless of
    /// which assertion it makes. Not hypothetical either way:
    /// `.github/workflows/ci.yml` runs `cargo test --workspace --all-features`
    /// on `ubuntu-latest`, `windows-latest`, and `macos-latest`, so this would
    /// land red (and green for the wrong reason) on two of three CI jobs.
    /// `collect_with_roots` was considered as a platform-independent
    /// alternative and rejected: it bypasses `collect()`'s `PINNED`-seeding
    /// step entirely, which is the one thing this module needs to prove, so a
    /// `collect_with_roots`-based version would only re-test what
    /// `transitive_marking_keeps_referenced_objects` (above) already covers.
    ///
    /// **Unconditionally `#[ignore]`d, in debug and release alike** --
    /// independently of the Windows gate above, and no longer only under
    /// `--release` as this comment used to say. `collect()`'s conservative
    /// stack scan is also intermittently flaky in debug, under the test
    /// parallelism CI actually runs at: a stale stack word in an
    /// already-returned frame is read as a conservative root, well above the
    /// scanned range's low end (so *not* `&regs`, the register-flush buffer
    /// the `setjmp` shim seeds at the start of its own frame, which sits at
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
    /// same ungated-canary pattern a Critical review finding flagged, and the
    /// same pairing principle the Windows gate above exists for. Each test's
    /// own `#[ignore]` reason says which of the two it is, so that is not
    /// restated here. Reachable with `cargo test -- --ignored`, which CI runs
    /// as an advisory, `continue-on-error` step.
    #[cfg(windows)]
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
        /// scanner looks: a stack slot, or a callee-saved register the
        /// `setjmp` shim flushes to one. A plain `usize` copy of the address
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
            //    a stack slot, or a callee-saved register the setjmp shim flushes
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
            // would be exactly the pattern this file's Windows gate exists to
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
