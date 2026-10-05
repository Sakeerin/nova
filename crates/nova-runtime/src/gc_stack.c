/* Register flush for the conservative GC.
 *
 * `setjmp` spills the callee-saved registers into `regs` (a buffer on this
 * frame's stack), so any heap root held only in a callee-saved register at the
 * point of collection becomes visible to a plain stack scan. Caller-saved
 * registers need no flushing: the C ABI treats them as clobbered by the call
 * into the runtime, so the compiled Nova code has already spilled any live
 * root out of them before calling the allocator.
 *
 * We then scan the stack from just below `regs` (the lowest meaningful address)
 * up to `stack_base` (the thread's stack origin), which covers `regs` plus all
 * caller frames. `nova_gc_scan_range` is implemented in Rust.
 *
 * SPIKE (GCC/Clang only): glibc's and Apple's `setjmp` mangle some of the
 * registers they save, so a root held only in one of those would not look like
 * an address. `__builtin_unwind_init` makes every callee-saved register land
 * in this frame's save area instead, and the scan starts in a deeper,
 * non-inlined frame so all of this one is covered. The asm after the call
 * keeps `regs` live, so the call cannot become a tail call that pops this
 * frame first. MSVC, the Windows toolchain, keeps the original path.
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
