//! Finding a project (spec
//! `docs/superpowers/specs/2026-10-07-phase-3-0-foundations-design.md`
//! §6.1).

use std::path::PathBuf;

use nova_pm::{find_root, MANIFEST};

/// A fresh, empty directory under the system temp dir, unique to this test.
/// No `nova.toml` sits above the temp dir on any machine this runs on
/// (checked on 2026-10-07), so `find_root` finds only what a test writes.
fn fresh_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("nova-find-root-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("create the test directory");
    dir
}

#[test]
fn the_manifest_in_the_starting_directory() {
    let dir = fresh_dir("here");
    std::fs::write(dir.join(MANIFEST), "").unwrap();
    assert_eq!(find_root(&dir), Some(dir));
}

#[test]
fn the_manifest_in_an_ancestor() {
    let dir = fresh_dir("ancestor");
    std::fs::write(dir.join(MANIFEST), "").unwrap();
    let deep = dir.join("a").join("b");
    std::fs::create_dir_all(&deep).unwrap();
    assert_eq!(find_root(&deep), Some(dir));
}

#[test]
fn the_nearer_of_two_wins() {
    let dir = fresh_dir("nearer");
    let inner = dir.join("inner");
    std::fs::create_dir_all(inner.join("src")).unwrap();
    std::fs::write(dir.join(MANIFEST), "").unwrap();
    std::fs::write(inner.join(MANIFEST), "").unwrap();
    assert_eq!(find_root(&inner.join("src")), Some(inner));
}

#[test]
fn a_directory_named_nova_toml_does_not_count() {
    let dir = fresh_dir("named-dir");
    std::fs::create_dir_all(dir.join(MANIFEST)).unwrap();
    assert_eq!(find_root(&dir), None);
}

#[test]
fn with_no_manifest_there_is_no_project() {
    assert_eq!(find_root(&fresh_dir("none")), None);
}
