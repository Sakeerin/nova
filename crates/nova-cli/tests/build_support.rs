//! Tests for `build_support.rs`, the decisions behind nova-cli's build
//! script (spec
//! `docs/superpowers/specs/2026-10-07-phase-3-0-foundations-design.md`
//! §4.1), compiled here from the same file.

#[path = "../build_support.rs"]
mod build_support;

use std::path::{Path, PathBuf};

use build_support::{
    copy_package, dep_info_sources, route, should_embed, staticlib_name, workspace_lockfile, Route,
};

/// A fresh, empty directory under the system temp dir, unique to this test.
fn fresh_dir(name: &str) -> PathBuf {
    // A fixed name, so each run replaces the last run's directory: a name with
    // the process id in it left one more behind on every run.
    let dir = std::env::temp_dir().join(format!("nova-build-support-{name}"));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("create the test directory");
    dir
}

fn runtime_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../nova-runtime")
}

#[test]
fn the_profile_decides_when_the_variable_is_unset() {
    assert_eq!(should_embed(None, "release"), Ok(true));
    assert_eq!(should_embed(None, "debug"), Ok(false));
}

#[test]
fn the_variable_overrides_the_profile() {
    assert_eq!(should_embed(Some("1"), "debug"), Ok(true));
    assert_eq!(should_embed(Some("0"), "release"), Ok(false));
}

#[test]
fn any_other_value_of_the_variable_is_an_error_naming_it() {
    for value in ["", "yes", "true", "2"] {
        let error = should_embed(Some(value), "release").unwrap_err();
        assert!(error.contains("NOVA_EMBED_RUNTIME"), "{value:?}: {error}");
    }
}

#[test]
fn nova_runtime_inherits_from_the_workspace_so_it_builds_in_place() {
    assert_eq!(route(&runtime_dir()).unwrap(), Route::InPlace);
}

#[test]
fn a_manifest_without_workspace_keys_builds_a_copy() {
    let dir = fresh_dir("packaged");
    std::fs::write(
        dir.join("Cargo.toml"),
        "[package]\nedition = \"2021\"\nname = \"nova-runtime\"\nversion = \"0.2.0\"\n\
         # edition.workspace = true is only a comment here\n",
    )
    .unwrap();
    assert_eq!(route(&dir).unwrap(), Route::Copy);
}

#[test]
fn a_stray_cargo_toml_orig_does_not_change_the_route() {
    // `git mergetool` leaves `*.orig` backups, and `.gitignore` hides them.
    let dir = fresh_dir("stray-orig");
    std::fs::write(
        dir.join("Cargo.toml"),
        "[package]\nname = \"r\"\nedition.workspace = true\n",
    )
    .unwrap();
    std::fs::write(dir.join("Cargo.toml.orig"), "a merge backup\n").unwrap();
    assert_eq!(route(&dir).unwrap(), Route::InPlace);
}

#[test]
fn the_copy_leaves_out_the_lockfile_and_target_and_adds_a_workspace_table() {
    let from = fresh_dir("copy-from");
    std::fs::create_dir_all(from.join("src/inner")).unwrap();
    std::fs::create_dir_all(from.join("target/release")).unwrap();
    // No final newline, as a hand-written manifest may have.
    std::fs::write(from.join("Cargo.toml"), "[package]\nname = \"r\"").unwrap();
    std::fs::write(from.join("Cargo.lock"), "pinned versions\n").unwrap();
    std::fs::write(from.join("build.rs"), "fn main() {}\n").unwrap();
    std::fs::write(from.join("src/lib.rs"), "").unwrap();
    std::fs::write(from.join("src/inner/shim.c"), "int x;\n").unwrap();
    std::fs::write(from.join("target/release/junk"), "").unwrap();
    let to = fresh_dir("copy-to").join("rt-src");

    copy_package(&from, &to).unwrap();

    assert!(to.join("build.rs").is_file());
    assert!(to.join("src/lib.rs").is_file());
    assert!(to.join("src/inner/shim.c").is_file());
    assert!(!to.join("Cargo.lock").exists());
    assert!(!to.join("target").exists());
    assert_eq!(
        std::fs::read_to_string(to.join("Cargo.toml")).unwrap(),
        "[package]\nname = \"r\"\n\n[workspace]\n"
    );
}

#[test]
fn copying_again_replaces_the_old_copy() {
    let from = fresh_dir("recopy-from");
    std::fs::write(from.join("Cargo.toml"), "[package]\nname = \"r\"\n").unwrap();
    let to = fresh_dir("recopy-to").join("rt-src");
    std::fs::create_dir_all(&to).unwrap();
    std::fs::write(to.join("stale.rs"), "").unwrap();

    copy_package(&from, &to).unwrap();

    assert!(!to.join("stale.rs").exists());
    assert!(to.join("Cargo.toml").is_file());
}

#[test]
fn dep_info_sources_reads_the_first_rule_and_unescapes_spaces() {
    // Absolute paths for this platform; on Windows they carry a drive
    // letter, whose colon must not end the rule's target.
    let base = std::env::temp_dir();
    let target = base.join("rt").join("libnova_runtime.rlib");
    let lib = base
        .join("w")
        .join("nova-runtime")
        .join("src")
        .join("lib.rs");
    let spaced = base.join("w").join("with space").join("x.rs");
    let escaped = spaced.display().to_string().replace(' ', "\\ ");
    let dep_info = format!(
        "{}: {} {} src/relative.rs\n\n{}:\n",
        target.display(),
        lib.display(),
        escaped,
        lib.display()
    );
    assert_eq!(dep_info_sources(&dep_info), vec![lib, spaced]);
}

#[test]
fn an_empty_dep_info_names_nothing() {
    assert!(dep_info_sources("").is_empty());
    assert!(dep_info_sources("\n\n").is_empty());
}

#[test]
fn the_workspace_lockfile_is_the_nearest_above_the_runtime() {
    let found = workspace_lockfile(&runtime_dir()).expect("a Cargo.lock above nova-runtime");
    let expected = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../Cargo.lock");
    assert_eq!(
        found.canonicalize().unwrap(),
        expected.canonicalize().unwrap()
    );
}

#[test]
fn the_staticlib_is_named_for_the_target() {
    assert_eq!(staticlib_name("msvc"), "nova_runtime.lib");
    assert_eq!(staticlib_name("gnu"), "libnova_runtime.a");
    assert_eq!(staticlib_name(""), "libnova_runtime.a");
}
