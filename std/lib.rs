//! The sources of Nova's standard library, `std/*/lib.nova`, embedded at
//! build time so the compiler stays one self-contained executable.
//!
//! Every `include_str!` path stays inside this package, so `cargo package`
//! carries all 19 files, and its verification step builds the packaged copy
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
/// `std/dom/lib.nova`, the page of a WASM build, which a program imports.
/// Not in [`ALL`]: only a WASM build compiles it, and the documentation
/// site is native std's (spec
/// `docs/superpowers/specs/2026-10-16-phase-4-2-std-dom-and-the-shim-design.md`
/// §3.2).
pub const DOM: &str = include_str!("dom/lib.nova");
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
/// `std/rt/lib.nova`, the runtime of a WASM build, written in Nova. Not in
/// [`ALL`]: only a WASM build compiles it, and its few public items are for
/// std's other modules alone, so it is not documented (spec
/// `docs/superpowers/specs/2026-10-14-phase-4-0-foundations-design.md`
/// §3.2; 4.2 §5).
pub const RT: &str = include_str!("rt/lib.nova");

/// Every documented module, by its directory under `std/`: all but
/// [`RT`].
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

    /// Every `std/*/lib.nova` on disk is embedded, and nothing else is:
    /// [`super::ALL`]'s, `std/rt`'s as [`super::RT`], and `std/dom`'s as
    /// [`super::DOM`].
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
            .chain(["rt".to_string(), "dom".to_string()])
            .collect();
        assert_eq!(embedded, on_disk);
        assert_eq!(on_disk.len(), 19);
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
        let rt = std::fs::read_to_string(root.join("rt").join("lib.nova"))
            .expect("read std/rt/lib.nova");
        assert_eq!(super::RT, rt, "std/rt/lib.nova");
    }
}
