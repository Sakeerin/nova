//! Builds the runtime library, `nova-runtime`'s staticlib, and embeds it in
//! `nova`, gzip-compressed, so an installed `nova` links programs with
//! nothing beside it (spec
//! `docs/superpowers/specs/2026-10-07-phase-3-0-foundations-design.md` §4.1;
//! `docs/adr/0027-runtime-and-std-in-an-installed-nova.md`).
//!
//! On stable Cargo one crate cannot take another's staticlib (artifact
//! dependencies are unstable), so this script runs a nested
//! `cargo build --release` of the runtime into its own `OUT_DIR`. Its
//! decisions live in `build_support.rs`, which `tests/build_support.rs`
//! tests.

mod build_support;

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;

use build_support::Route;

fn main() {
    let out = PathBuf::from(var("OUT_DIR"));
    let target = var("TARGET");
    let lib_name = build_support::staticlib_name(&var("CARGO_CFG_TARGET_ENV"));
    println!("cargo:rustc-env=NOVA_TARGET={target}");
    println!("cargo:rustc-env=NOVA_EMBEDDED_RUNTIME_NAME={lib_name}");
    println!("cargo:rerun-if-env-changed=NOVA_EMBED_RUNTIME");

    let setting =
        std::env::var_os("NOVA_EMBED_RUNTIME").map(|value| value.to_string_lossy().into_owned());
    let embed = build_support::should_embed(setting.as_deref(), &var("PROFILE"))
        .unwrap_or_else(|message| panic!("{message}"));

    let payload = out.join("nova_runtime.gz");
    let (crc32, size) = if embed {
        let library = build_runtime(&out, &target, lib_name);
        let bytes = std::fs::read(&library)
            .unwrap_or_else(|e| panic!("reading {}: {e}", library.display()));
        compress(&bytes, &payload);
        (crc32fast::hash(&bytes), bytes.len())
    } else {
        // An empty payload means "not embedded" (spec §4.1).
        std::fs::write(&payload, b"")
            .unwrap_or_else(|e| panic!("writing {}: {e}", payload.display()));
        (0, 0)
    };
    println!(
        "cargo:rustc-env=NOVA_EMBEDDED_RUNTIME={}",
        payload.display()
    );
    println!("cargo:rustc-env=NOVA_EMBEDDED_RUNTIME_CRC32={crc32}");
    println!("cargo:rustc-env=NOVA_EMBEDDED_RUNTIME_SIZE={size}");
}

fn var(name: &str) -> String {
    std::env::var(name).unwrap_or_else(|_| panic!("cargo sets {name} for build scripts"))
}

/// Build the runtime's staticlib for `target` with a nested cargo, and
/// return its path. Prints a `rerun-if-changed` for everything the library
/// is built from (spec §4.1).
fn build_runtime(out: &Path, target: &str, lib_name: &str) -> PathBuf {
    let runtime = PathBuf::from(
        std::env::var("DEP_NOVA_RUNTIME_MANIFEST_DIR")
            .expect("nova-runtime's `links` metadata: its build.rs prints `cargo:manifest_dir`"),
    );
    let route = build_support::route(&runtime)
        .unwrap_or_else(|e| panic!("reading {}/Cargo.toml: {e}", runtime.display()));
    let target_dir = out.join("rt");
    let cargo = std::env::var_os("CARGO").unwrap_or_else(|| "cargo".into());
    let mut command = Command::new(cargo);
    command
        .args(["build", "--lib", "--release", "--target", target])
        .arg("--target-dir")
        .arg(&target_dir);
    match route {
        Route::InPlace => {
            // The workspace's lockfile applies; `--locked` never rewrites
            // it. Not `--offline`: a `cargo install --git` without
            // `--locked` may have fetched newer versions than the lockfile
            // pins, and cargo fetches only what is missing.
            command
                .arg("--manifest-path")
                .arg(runtime.join("Cargo.toml"))
                .arg("--locked");
        }
        Route::Copy => {
            let copy = out.join("rt-src");
            build_support::copy_package(&runtime, &copy).unwrap_or_else(|e| {
                panic!("copying {} to {}: {e}", runtime.display(), copy.display())
            });
            command
                .arg("--manifest-path")
                .arg(copy.join("Cargo.toml"))
                .arg("--offline");
        }
    }
    for name in [
        "CARGO_TARGET_DIR",
        "CARGO_BUILD_TARGET",
        "RUSTFLAGS",
        "CARGO_ENCODED_RUSTFLAGS",
    ] {
        command.env_remove(name);
    }
    let status = command
        .status()
        .expect("spawning the nested cargo build of nova-runtime");
    assert!(
        status.success(),
        "the nested cargo build of nova-runtime failed ({status})"
    );

    let release = target_dir.join(target).join("release");
    println!("cargo:rerun-if-changed={}", runtime.display());
    if route == Route::InPlace {
        // A packaged runtime's sources never change, and its copy is
        // rewritten on every run, so only the workspace route follows the
        // lockfile and the dep-info file's local sources, which cover path
        // dependencies such as nova-diagnostics.
        if let Some(lock) = build_support::workspace_lockfile(&runtime) {
            println!("cargo:rerun-if-changed={}", lock.display());
        }
        let dep_info =
            std::fs::read_to_string(release.join("libnova_runtime.d")).unwrap_or_default();
        for source in build_support::dep_info_sources(&dep_info) {
            println!("cargo:rerun-if-changed={}", source.display());
        }
    }
    let library = release.join(lib_name);
    assert!(
        library.is_file(),
        "the nested build made no {}",
        library.display()
    );
    library
}

/// Gzip `bytes` into `payload` at flate2's best compression.
fn compress(bytes: &[u8], payload: &Path) {
    let file = std::fs::File::create(payload)
        .unwrap_or_else(|e| panic!("creating {}: {e}", payload.display()));
    let mut encoder = flate2::write::GzEncoder::new(file, flate2::Compression::best());
    encoder
        .write_all(bytes)
        .unwrap_or_else(|e| panic!("compressing into {}: {e}", payload.display()));
    encoder
        .finish()
        .unwrap_or_else(|e| panic!("finishing {}: {e}", payload.display()));
}
