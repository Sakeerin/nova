//! The sources of Nova's standard library, `std/*/lib.nova`, embedded at
//! build time so the compiler stays one self-contained executable.
//!
//! Every `include_str!` path stays inside this package, so `cargo package`
//! carries all 21 files, and its verification step builds the packaged copy
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
/// `std/ui/lib.nova`: reactivity and views, which a program imports.
pub const UI: &str = include_str!("ui/lib.nova");
/// `std/ui/html/lib.nova`: the element builders, which a program imports.
pub const UI_HTML: &str = include_str!("ui/html/lib.nova");
/// `std/rt/lib.nova`, the runtime of a WASM build, written in Nova. Not in
/// [`ALL`]: only a WASM build compiles it, and its few public items are for
/// std's other modules alone, so it is not documented (spec
/// `docs/superpowers/specs/2026-10-14-phase-4-0-foundations-design.md`
/// §3.2; 4.2 §5).
pub const RT: &str = include_str!("rt/lib.nova");

/// Every documented module, by its directory under `std/`: all but
/// [`RT`] and [`DOM`].
pub const ALL: [(&str, &str); 19] = [
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
    ("ui", UI),
    ("ui/html", UI_HTML),
];

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;
    use std::path::Path;

    /// Every `.nova` file under `std/` is embedded, and nothing else:
    /// [`super::ALL`]'s, `std/rt`'s as [`super::RT`], `std/dom`'s as
    /// [`super::DOM`].
    #[test]
    fn every_std_module_is_embedded() {
        fn walk(dir: &Path, under: &str, out: &mut BTreeSet<String>) {
            for entry in std::fs::read_dir(dir).expect("read a directory of std/") {
                let path = entry.expect("read an entry of std/").path();
                let name = path
                    .file_name()
                    .expect("a file name")
                    .to_string_lossy()
                    .into_owned();
                let rel = if under.is_empty() {
                    name
                } else {
                    format!("{under}/{name}")
                };
                if path.is_dir() {
                    walk(&path, &rel, out);
                } else if rel.ends_with(".nova") {
                    out.insert(rel);
                }
            }
        }
        let root = Path::new(env!("CARGO_MANIFEST_DIR"));
        let mut on_disk = BTreeSet::new();
        walk(root, "", &mut on_disk);
        let embedded: BTreeSet<String> = super::ALL
            .iter()
            .map(|(name, _)| format!("{name}/lib.nova"))
            .chain(["rt/lib.nova", "dom/lib.nova"].map(String::from))
            .collect();
        assert_eq!(embedded, on_disk);
        assert_eq!(on_disk.len(), 21);
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
        for (path, source) in [("rt/lib.nova", super::RT), ("dom/lib.nova", super::DOM)] {
            let on_disk = std::fs::read_to_string(root.join(path)).expect("read a std module");
            assert_eq!(source, on_disk, "std/{path}");
        }
    }
}
