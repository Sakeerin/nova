/* Register flush for the conservative GC.
 *
 * A heap root can be held only in a callee-saved register at the point of
 * collection. This shim writes the callee-saved registers into its own stack
 * frame, then scans from below that frame up to `stack_base` (the thread's
 * stack top), which covers the saved registers and every caller frame.
 * Caller-saved registers need no flushing: the C ABI treats them as clobbered
 * by the call into the runtime, so the compiled Nova code has already spilled
 * any live root out of them before calling the allocator.
 * `nova_gc_scan_range` is implemented in Rust (`gc.rs`).
 *
 * GCC and Clang: `setjmp` alone is not enough. glibc's x86-64 `setjmp` stores
 * the saved `rbp`, `rsp` and return address scrambled, and Apple's arm64
 * `setjmp` the frame pointer, link register and stack pointer, so a root held
 * only in one of those would not look like an address in the saved copy.
 * `__builtin_unwind_init()` makes this function save every callee-saved
 * register in its own frame instead. The scan runs in a separate non-inlined
 * function, so it starts below this frame and covers all of it. The empty
 * `asm` after the call reads `regs`, so the call cannot become a tail call
 * that pops this frame first. `setjmp` stays as a second copy: that is the
 * shape CI first ran (draft PR #98). No test can force a root into a
 * callee-saved register, so none discriminates this; see
 * `docs/adr/0024-gc-stack-bounds-on-unix.md`.
 *
 * MSVC: `setjmp` into `regs`, then a scan from `regs` up.
 */
#include <setjmp.h>

extern void nova_gc_scan_range(void *lo, void *hi);

#if defined(__GNUC__) || defined(__clang__)

static __attribute__((noinline)) void nova_gc_scan_from_below(void *stack_base) {
    void *volatile marker = 0;
    nova_gc_scan_range((void *)&marker, stack_base);
}

void nova_gc_collect_roots(void *stack_base) {
    jmp_buf regs;
    __builtin_unwind_init();
    setjmp(regs);
    nova_gc_scan_from_below(stack_base);
    __asm__ __volatile__("" : : "r"(&regs) : "memory");
}

#else

void nova_gc_collect_roots(void *stack_base) {
    jmp_buf regs;
    setjmp(regs);
    nova_gc_scan_range((void *)&regs, stack_base);
}

#endif
