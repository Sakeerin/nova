//! A conservative, non-moving mark-and-sweep garbage collector.
//!
//! Neither Nova codegen backend emits stack maps or per-slot type information,
//! so the collector cannot know precisely where roots or heap pointers live.
//! It is therefore *conservative*: any machine word (on the stack, in a
//! callee-saved register, or inside a scanned heap object) whose value falls
//! within a live allocation keeps that allocation alive. This can retain a
//! little garbage (an integer that happens to look like a pointer) but never
//! frees a reachable object.
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

/// Register `ptr` as a root until [`remove_root`]. Idempotent per address is
/// **not** assumed: registering twice requires removing twice, so callers must
/// pair them exactly. The executor does (one add at spawn, one remove at
/// completion).
///
/// `gc` is a private module (see `lib.rs`), so until the executor lands and
/// calls this from non-test code, it is unreachable outside `#[cfg(test)]`,
/// where it already is used. `#[expect(dead_code)]` was tried first, to make
/// the lint itself flag this comment as stale once the executor calls this;
/// measured to not work here, because `--all-targets` also builds the test
/// target, where `dead_code` does not fire (the tests call this directly) so
/// the expectation is "unfulfilled" there instead -- the two targets need
/// opposite answers from the same attribute. Plain `#[allow]` is the one that
/// is correct for both.
#[allow(
    dead_code,
    reason = "consumed by the async executor (Task 4), not yet on this branch"
)]
pub fn add_root(ptr: *mut u8) {
    PINNED.with(|p| p.borrow_mut().push(ptr as usize));
}

/// Unregister one registration of `ptr`. Removing an address that was never
/// registered is a no-op rather than a panic — the runtime must not abort a
/// user's program over its own bookkeeping.
#[allow(
    dead_code,
    reason = "consumed by the async executor (Task 4), not yet on this branch"
)]
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
            }
            h.alloc_since_gc = 0;
            h.live_bytes = 0;
            h.next_gc = INITIAL_THRESHOLD;
        });
    }

    fn count() -> usize {
        HEAP.with(|h| h.borrow().objects.len())
    }

    /// Carry a heap address across a `collect()` call below without leaving
    /// its literal bits in a place the conservative scanner would treat as a
    /// root.
    ///
    /// Several tests below need the numeric address of an object *after*
    /// calling the real `collect()`, to look it up with `object_info` or hand
    /// it to `remove_root`. A `usize` that must be read again after a call
    /// has to survive that call, and this collector's own definition of
    /// "conservative" (module doc comment, top of file) means the compiler
    /// necessarily preserves a surviving value somewhere the scanner looks: a
    /// stack slot, or a callee-saved register the `setjmp` shim flushes to
    /// one. A plain `usize` copy of the address is bit-identical to a real
    /// pointer to it, so it is then indistinguishable from a genuine root —
    /// which would make `object_info` report the object alive whether or not
    /// the registry (or anything else) is actually the thing keeping it
    /// there.
    ///
    /// This is measured, not hypothetical: the first version of the tests
    /// below carried `addr` across `collect()` as a plain `usize`, and with
    /// that shape `an_unregistered_object_is_swept` and
    /// `remove_root_actually_unroots` both failed (the object was never
    /// swept), while separately neutering `add_root` into a no-op left
    /// `a_registered_root_survives_a_collection_with_no_stack_reference`,
    /// `a_registered_root_keeps_its_transitive_children_alive`, and
    /// `the_registry_survives_more_than_one_collection` all still passing —
    /// i.e. every test that used a plain `usize` this way was either
    /// unpassable or passed regardless of whether the registry did anything.
    ///
    /// The complement is its own inverse, and for every address a live heap
    /// allocation can actually have in this process it lands far outside any
    /// tracked object's `[addr, addr + size)` range (real addresses are
    /// nowhere near `usize::MAX`), so `mark_word` never mistakes the hidden
    /// form for a root in transit.
    fn hide(addr: usize) -> usize {
        !addr
    }

    /// Inverse of [`hide`]. A distinct name at call sites, even though the
    /// operation is identical, so a `hide`/`reveal` pair reads as encode/decode
    /// rather than as an unexplained bitwise flip.
    fn reveal(hidden: usize) -> usize {
        !hidden
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

    #[test]
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
        //    overwritten and stays live for the rest of the frame. Measured
        //    directly: with shadowing, `an_unregistered_object_is_swept` and
        //    `remove_root_actually_unroots` both failed -- their objects were
        //    never swept -- even though the first of those two never calls
        //    `add_root` at all. Fixed by making `obj` `mut` and reassigning in
        //    place, so the null overwrites the same slot the original pointer
        //    occupied.
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
    fn an_unregistered_object_is_swept() {
        // The discriminating half. Without this, the test above passes even if
        // collect() never frees anything at all. It is also the test that
        // caught both hazards documented on
        // `a_registered_root_survives_a_collection_with_no_stack_reference`:
        // `obj` is `mut` and nulled by reassignment (not `let`-shadowed), and
        // `addr` crosses `collect()` hidden rather than as a plain `usize`.
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
    fn remove_root_actually_unroots() {
        // Otherwise add/remove is a leak, and every completed task's state is
        // retained for the process lifetime. See
        // `a_registered_root_survives_a_collection_with_no_stack_reference` for
        // why `obj` is `mut`+reassigned and `addr` crosses `collect()` hidden.
        let mut obj = alloc(64, true);
        let hidden = hide(obj as usize);
        add_root(obj);
        remove_root(obj);
        obj = std::ptr::null_mut::<u8>();
        std::hint::black_box(obj);
        std::hint::black_box(hidden);

        collect();

        let addr = reveal(hidden);
        assert!(object_info(addr).is_none(), "remove_root did not unroot");
    }

    /// Allocate `parent`/`child`, link `child` under `parent`, register
    /// `parent`, and hand back both addresses hidden (see [`hide`]). Used only
    /// by `a_registered_root_keeps_its_transitive_children_alive`.
    ///
    /// A separate, never-inlined function, deliberately: every raw pointer
    /// this scenario needs (`parent`, `child`, the cast receiver for the
    /// write, the value written) stays local to this call and is never
    /// returned. Measured that this matters and a same-frame version does
    /// not: with everything inlined into the test itself -- `mut`-reassigned
    /// locals, `hide`d addresses, even the write's operands routed through
    /// named temporaries and a 4 KiB stack buffer stomped over the frame
    /// before collecting -- `parent` still measurably survived a collection
    /// with `add_root` reduced to a no-op. Explicitly zeroing every
    /// callee-saved general-purpose register this build's inline-asm would
    /// let a program touch -- `rsi`, `rdi`, `r12`-`r15` -- did not clear it
    /// either (`rbx` is reserved by rustc/LLVM on this target and refused as
    /// an asm operand; `rbp` and the callee-saved `xmm6`-`xmm15` were not
    /// tried). So the exact register is not identified, only narrowed to
    /// "some callee-saved register (or, less likely, some other stack slot
    /// this pass didn't reach) this same-frame shape leaves live" -- but the
    /// mechanism that fixes it is not in question: unlike a stack slot, a
    /// callee-saved register that a deeper call uses is saved on that call's
    /// entry and restored to the *caller's* pre-call value on return, by the
    /// calling convention every correctly-compiled function must honor -- not
    /// left holding whatever the callee last put there. Putting the whole
    /// scenario behind exactly one such call, returning only `hide`d
    /// integers, resolved it.
    #[inline(never)]
    fn setup_registered_parent_and_child() -> (usize, usize) {
        let mut parent = alloc(16, true);
        let mut child = alloc(32, true);
        let child_addr = hide(child as usize);
        let parent_addr = hide(parent as usize);
        let mut child_bits = child as usize;
        let mut parent_ptr = parent as *mut usize;
        unsafe { parent_ptr.write(child_bits) };
        add_root(parent);
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
    fn a_registered_root_keeps_its_transitive_children_alive() {
        // The registry seeds the mark set; marking must then TRACE. A registry
        // that marked only the registered object itself would free a suspended
        // task's locals while keeping its state header -- the exact bug, one
        // level down, and invisible to the first test.
        //
        // All of `parent`, `child`, and every raw pointer derived from them
        // lives and dies inside `setup_registered_parent_and_child` -- see its
        // doc comment for why that call boundary, specifically, is what makes
        // this test trustworthy. This function only ever holds the hidden
        // (`hide`d) addresses.
        let (parent_addr, child_addr) = setup_registered_parent_and_child();
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
    fn the_registry_survives_more_than_one_collection() {
        // ROOTS (gc.rs:95) is a SCRATCH buffer cleared at the start of every
        // cycle. If the registry were folded into it, the first collection would
        // consume it and the second would sweep the root. That failure mode is
        // invisible to any single-collection test.
        //
        // See `a_registered_root_survives_a_collection_with_no_stack_reference`
        // for why `obj` is `mut`+reassigned and `addr` crosses each `collect()`
        // call hidden.
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
