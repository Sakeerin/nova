//! Compiles the small C shim that flushes callee-saved registers onto the
//! stack so the conservative GC's stack scan can see roots held only in
//! registers: `__builtin_unwind_init` plus `setjmp` on GCC and Clang, `setjmp`
//! on MSVC (`src/gc_stack.c` says why). Doing this in C avoids depending on
//! architecture-specific inline assembly. The resulting static archive is
//! bundled into `nova-runtime`'s rlib and staticlib (default `+bundle`), so both
//! `nova run` (JIT) and `nova build` see the symbol.
//!
//! It also reports this crate's source directory as `links` metadata
//! (`cargo:manifest_dir`), which reaches nova-cli's build script as
//! `DEP_NOVA_RUNTIME_MANIFEST_DIR`: that script builds this crate's
//! staticlib a second time, in release mode, and embeds it in `nova`.

fn main() {
    println!(
        "cargo:manifest_dir={}",
        std::env::var("CARGO_MANIFEST_DIR").expect("cargo sets CARGO_MANIFEST_DIR")
    );
    println!("cargo:rerun-if-changed=src/gc_stack.c");
    cc::Build::new()
        .file("src/gc_stack.c")
        .compile("nova_gc_stack");
}
