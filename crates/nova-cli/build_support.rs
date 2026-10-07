//! The decisions behind `build.rs`, which builds the runtime library and
//! embeds it in `nova` (spec
//! `docs/superpowers/specs/2026-10-07-phase-3-0-foundations-design.md`
//! §4.1). Only `std` is used here, so `tests/build_support.rs` compiles this
//! same file and tests each decision without running a build.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

/// Whether to embed the runtime. `NOVA_EMBED_RUNTIME` (`var`) decides when
/// it is set; otherwise the profile does, so a release build, which
/// `cargo install` makes by default, embeds, and a debug build does not.
pub fn should_embed(var: Option<&str>, profile: &str) -> Result<bool, String> {
    match var {
        None => Ok(profile == "release"),
        Some("1") => Ok(true),
        Some("0") => Ok(false),
        Some(other) => Err(format!("NOVA_EMBED_RUNTIME must be 1 or 0, not {other:?}")),
    }
}

/// How the nested build reaches the runtime's source.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Route {
    /// A workspace member: build it in place, against the workspace's
    /// lockfile, with `--locked`.
    InPlace,
    /// A packaged crate: build a copy without its lockfile, `--offline`.
    Copy,
}

/// The route for the runtime in `runtime_dir`, read from its `Cargo.toml`.
/// A workspace member inherits from its workspace (`edition.workspace =
/// true`), and `cargo package` always resolves that away, so a packaged
/// manifest never says `workspace = true`. A file such as `Cargo.toml.orig`
/// would be a weaker sign: `git mergetool` leaves `*.orig` backups in a
/// checkout, and `.gitignore` hides them.
pub fn route(runtime_dir: &Path) -> io::Result<Route> {
    let manifest = fs::read_to_string(runtime_dir.join("Cargo.toml"))?;
    let inherits = manifest.lines().any(|line| {
        let line = line.trim_start();
        !line.starts_with('#') && line.replace(' ', "").contains("workspace=true")
    });
    Ok(if inherits {
        Route::InPlace
    } else {
        Route::Copy
    })
}

/// Copy a packaged runtime from `from` to `to`, for [`Route::Copy`]:
/// everything except its top-level `Cargo.lock`, whose pinned versions may
/// be missing from an offline cache, and `target`. The copy's manifest gains
/// an empty `[workspace]` table, so cargo never takes the copy for a member
/// of a workspace that happens to surround the build's output directory.
pub fn copy_package(from: &Path, to: &Path) -> io::Result<()> {
    if to.exists() {
        fs::remove_dir_all(to)?;
    }
    fs::create_dir_all(to)?;
    for entry in fs::read_dir(from)? {
        let entry = entry?;
        let name = entry.file_name();
        if name == "Cargo.lock" || name == "target" {
            continue;
        }
        copy_tree(&entry.path(), &to.join(&name))?;
    }
    let manifest = to.join("Cargo.toml");
    let mut text = fs::read_to_string(&manifest)?;
    if !text.ends_with('\n') {
        text.push('\n');
    }
    text.push_str("\n[workspace]\n");
    fs::write(&manifest, text)
}

fn copy_tree(from: &Path, to: &Path) -> io::Result<()> {
    if fs::metadata(from)?.is_dir() {
        fs::create_dir_all(to)?;
        for entry in fs::read_dir(from)? {
            let entry = entry?;
            copy_tree(&entry.path(), &to.join(entry.file_name()))?;
        }
        Ok(())
    } else {
        fs::copy(from, to).map(|_| ())
    }
}

/// The source files a cargo dep-info file (`<artifact>.d`) names on its
/// first rule: everything after `": "`, split on whitespace, with `\ ` as an
/// escaped space. Only absolute paths are returned, because a relative one
/// is relative to a `build.dep-info-basedir` this script cannot know.
pub fn dep_info_sources(dep_info: &str) -> Vec<PathBuf> {
    let Some(rule) = dep_info.lines().find(|line| !line.trim().is_empty()) else {
        return Vec::new();
    };
    let Some((_, sources)) = rule.split_once(": ") else {
        return Vec::new();
    };
    sources
        .replace("\\ ", "\u{0}")
        .split_whitespace()
        .map(|source| PathBuf::from(source.replace('\u{0}', " ")))
        .filter(|source| source.is_absolute())
        .collect()
}

/// The lockfile an in-place build of the runtime uses: the first
/// `Cargo.lock` in the runtime's directory or above it.
pub fn workspace_lockfile(runtime_dir: &Path) -> Option<PathBuf> {
    runtime_dir
        .ancestors()
        .map(|dir| dir.join("Cargo.lock"))
        .find(|lock| lock.is_file())
}

/// The runtime staticlib's file name for a target whose
/// `CARGO_CFG_TARGET_ENV` is `target_env`: `nova_runtime.lib` for MSVC, and
/// `libnova_runtime.a` for the other toolchains this project builds with.
pub fn staticlib_name(target_env: &str) -> &'static str {
    if target_env == "msvc" {
        "nova_runtime.lib"
    } else {
        "libnova_runtime.a"
    }
}
