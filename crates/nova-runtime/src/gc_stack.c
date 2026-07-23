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
 */
#include <setjmp.h>

extern void nova_gc_scan_range(void *lo, void *hi);

void nova_gc_collect_roots(void *stack_base) {
    jmp_buf regs;
    setjmp(regs);
    nova_gc_scan_range((void *)&regs, stack_base);
}
