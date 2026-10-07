//! The sources of Nova's standard library, `std/*/lib.nova`, embedded at
//! build time so the compiler stays one self-contained executable.
//!
//! Every `include_str!` path stays inside this package, so `cargo package`
//! carries all 17 files, and its verification step builds the packaged copy
//! on its own (spec
//! `docs/superpowers/specs/2026-10-07-phase-3-0-foundations-design.md` §3).
//! `nova-resolver` gives each module its `$std.*` name and its place.

/// `std/bytes/lib.nova`.
pub const BYTES: &str = include_str!("bytes/lib.nova");
/// `std/collections/lib.nova`.
pub const COLLECTIONS: &str = include_str!("collections/lib.nova");
/// `std/core/lib.nova`.
pub const CORE: &str = include_str!("core/lib.nova");
/// `std/crypto/lib.nova`.
pub const CRYPTO: &str = include_str!("crypto/lib.nova");
/// `std/fmt/lib.nova`.
pub const FMT: &str = include_str!("fmt/lib.nova");
/// `std/fs/lib.nova`.
pub const FS: &str = include_str!("fs/lib.nova");
/// `std/http/lib.nova`.
pub const HTTP: &str = include_str!("http/lib.nova");
/// `std/io/lib.nova`.
pub const IO: &str = include_str!("io/lib.nova");
/// `std/json/lib.nova`.
pub const JSON: &str = include_str!("json/lib.nova");
/// `std/log/lib.nova`.
pub const LOG: &str = include_str!("log/lib.nova");
/// `std/net/lib.nova`.
pub const NET: &str = include_str!("net/lib.nova");
/// `std/process/lib.nova`.
pub const PROCESS: &str = include_str!("process/lib.nova");
/// `std/strings/lib.nova`.
pub const STRINGS: &str = include_str!("strings/lib.nova");
/// `std/sync/lib.nova`.
pub const SYNC: &str = include_str!("sync/lib.nova");
/// `std/task/lib.nova`.
pub const TASK: &str = include_str!("task/lib.nova");
/// `std/test/lib.nova`, seeded only under `nova test`.
pub const TEST: &str = include_str!("test/lib.nova");
/// `std/time/lib.nova`.
pub const TIME: &str = include_str!("time/lib.nova");

/// Every embedded module, by its directory under `std/`.
pub const ALL: [(&str, &str); 17] = [
    ("bytes", BYTES),
    ("collections", COLLECTIONS),
    ("core", CORE),
    ("crypto", CRYPTO),
    ("fmt", FMT),
    ("fs", FS),
    ("http", HTTP),
    ("io", IO),
    ("json", JSON),
    ("log", LOG),
    ("net", NET),
    ("process", PROCESS),
    ("strings", STRINGS),
    ("sync", SYNC),
    ("task", TASK),
    ("test", TEST),
    ("time", TIME),
];

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;
    use std::path::Path;

    /// Every `std/*/lib.nova` on disk is embedded, and nothing else is.
    #[test]
    fn every_std_module_is_embedded() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"));
        let on_disk: BTreeSet<String> = std::fs::read_dir(root)
            .expect("read std/")
            .map(|entry| entry.expect("read an entry of std/").path())
            .filter(|path| path.join("lib.nova").is_file())
            .map(|path| {
                path.file_name()
                    .expect("a directory name")
                    .to_string_lossy()
                    .into_owned()
            })
            .collect();
        let embedded: BTreeSet<String> = super::ALL
            .iter()
            .map(|(name, _)| name.to_string())
            .collect();
        assert_eq!(embedded, on_disk);
        assert_eq!(on_disk.len(), 17);
    }

    /// Each entry carries its own file, not a neighbour's.
    #[test]
    fn each_entry_is_its_own_file() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"));
        for (name, source) in super::ALL {
            let on_disk = std::fs::read_to_string(root.join(name).join("lib.nova"))
                .expect("read a std module");
            assert_eq!(source, on_disk, "std/{name}/lib.nova");
        }
    }
}
