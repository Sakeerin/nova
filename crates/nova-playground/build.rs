//! The shadow stack's size, on wasm32 alone (spec
//! `docs/superpowers/specs/2026-11-28-phase-6-1-the-playground-design.md`
//! §3.3). Rust's stack sits first in the module's linear memory, 1 MiB by
//! default, and the front end and the backend's structurer recurse deeply
//! enough to need more: with 4 MiB, the browser's own stack runs out first,
//! so an overflow is one kind of error. A link argument here, unlike
//! `rustflags` in `.cargo/config.toml`, travels with the crate and no
//! `RUSTFLAGS` drops it; no host linker sees it.

fn main() {
    if std::env::var("CARGO_CFG_TARGET_ARCH").as_deref() == Ok("wasm32") {
        println!("cargo:rustc-link-arg-cdylib=-zstack-size=4194304");
    }
}
