//! The runtime library this `nova` carries. `build.rs` writes it, gzipped,
//! and names it through these variables (spec
//! `docs/superpowers/specs/2026-10-07-phase-3-0-foundations-design.md`
//! §4.1). An empty payload means this `nova` carries none.

/// The gzip stream `build.rs` wrote; empty when nothing is embedded.
pub const PAYLOAD: &[u8] = include_bytes!(env!("NOVA_EMBEDDED_RUNTIME"));

/// The payload, as nova-driver takes it.
pub fn runtime() -> nova_driver::EmbeddedRuntime {
    nova_driver::EmbeddedRuntime {
        gz: PAYLOAD,
        crc32: env!("NOVA_EMBEDDED_RUNTIME_CRC32")
            .parse()
            .expect("build.rs writes the CRC-32 as a u32"),
        size: env!("NOVA_EMBEDDED_RUNTIME_SIZE")
            .parse()
            .expect("build.rs writes the size as a number"),
        file_name: env!("NOVA_EMBEDDED_RUNTIME_NAME"),
        version: env!("CARGO_PKG_VERSION"),
    }
}
