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
//! - **the stack**, scanned from the current frame up to the thread's base.
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

/// Allocate `size` zeroed bytes as a GC-managed object. `scan` selects whether
/// the collector traces the object's contents for pointers.
pub fn alloc(size: usize, scan: bool) -> *mut u8 {
    let size = size.max(8);
    maybe_collect(size);
    let layout = Layout::from_size_align(size, ALIGN).expect("valid heap layout");
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
    ROOTS.with(|r| r.borrow_mut().clear());
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
                let layout = Layout::from_size_align(o.size, ALIGN).expect("valid heap layout");
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
                let layout = Layout::from_size_align(o.size, ALIGN).unwrap();
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
}
