//! Compiles the small C shim that flushes callee-saved registers onto the
//! stack (via `setjmp`) so the conservative GC's stack scan can see roots held
//! only in registers. Doing this in C avoids depending on architecture-specific
//! inline assembly; `setjmp` is portable. The resulting static archive is
//! bundled into `nova-runtime`'s rlib and staticlib (default `+bundle`), so both
//! `nova run` (JIT) and `nova build` see the symbol.

fn main() {
    println!("cargo:rerun-if-changed=src/gc_stack.c");
    cc::Build::new()
        .file("src/gc_stack.c")
        .compile("nova_gc_stack");
}
