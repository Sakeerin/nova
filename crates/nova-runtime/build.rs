//! Compiles the small C shim that flushes callee-saved registers onto the
//! stack so the conservative GC's stack scan can see roots held only in
//! registers: `__builtin_unwind_init` plus `setjmp` on GCC and Clang, `setjmp`
//! on MSVC (`src/gc_stack.c` says why). Doing this in C avoids depending on
//! architecture-specific inline assembly. The resulting static archive is
//! bundled into `nova-runtime`'s rlib and staticlib (default `+bundle`), so both
//! `nova run` (JIT) and `nova build` see the symbol.

fn main() {
    println!("cargo:rerun-if-changed=src/gc_stack.c");
    cc::Build::new()
        .file("src/gc_stack.c")
        .compile("nova_gc_stack");
}
