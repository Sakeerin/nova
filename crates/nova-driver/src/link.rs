//! Native linking for `nova build`: locate the `nova-runtime` static
//! library and invoke the platform linker (spec `14-CODEGEN.md` §11 —
//! system linker via cc-rs).

use std::path::{Path, PathBuf};
use std::process::Command;

use crate::runtime_cache;

use anyhow::{bail, Context, Result};
// `anyhow!` is only invoked from `link_msvc`'s `#[cfg(windows)]` body below;
// off Windows that function is the `unreachable!`-only stub, so an
// unqualified import here reads as unused there (`-D warnings` on the
// Clippy job, which runs on ubuntu-latest, is what turns that into a build
// failure).
#[cfg(windows)]
use anyhow::anyhow;

/// Compile textual LLVM IR (`ir`, a `.ll` file) to a native object file using
/// a discovered LLVM toolchain, optimizing at `-O2`.
///
/// Prefers `clang` (via `NOVA_CLANG` or PATH), falling back to `llc` (via
/// `NOVA_LLC` or PATH). Both emit a host-target object that the platform
/// linker then combines with the runtime — reusing [`link_executable`].
pub fn compile_ir_to_object(ir: &Path, object: &Path) -> Result<()> {
    let clang = std::env::var("NOVA_CLANG").unwrap_or_else(|_| "clang".to_string());
    if tool_available(&clang) {
        let mut cmd = Command::new(&clang);
        cmd.arg("-O2").arg("-c").arg(ir).arg("-o").arg(object);
        return run_tool(cmd, "clang");
    }
    let llc = std::env::var("NOVA_LLC").unwrap_or_else(|_| "llc".to_string());
    if tool_available(&llc) {
        let mut cmd = Command::new(&llc);
        cmd.arg("-O2")
            .arg("-filetype=obj")
            .arg(ir)
            .arg("-o")
            .arg(object);
        return run_tool(cmd, "llc");
    }
    bail!(
        "no LLVM toolchain found for `--release`: install LLVM (so `clang` or \
         `llc` is on PATH), or set NOVA_CLANG / NOVA_LLC to one"
    )
}

/// Whether a tool can be spawned (probes `<tool> --version`).
fn tool_available(tool: &str) -> bool {
    Command::new(tool).arg("--version").output().is_ok()
}

fn run_tool(mut cmd: Command, name: &str) -> Result<()> {
    let output = cmd
        .output()
        .with_context(|| format!("failed to spawn {name}: {:?}", cmd.get_program()))?;
    if !output.status.success() {
        bail!(
            "{name} failed ({}):\n{}\n{}",
            output.status,
            String::from_utf8_lossy(&output.stdout).trim(),
            String::from_utf8_lossy(&output.stderr).trim(),
        );
    }
    Ok(())
}

/// Link a Nova object file and the runtime static library into an
/// executable at `output`.
pub fn link_executable(object: &Path, output: &Path) -> Result<()> {
    let runtime = find_runtime_lib()?;
    if cfg!(windows) {
        link_msvc(object, &runtime, output)
    } else {
        link_cc(object, &runtime, output)
    }
}

/// The runtime static library name for the current platform.
fn runtime_lib_name() -> &'static str {
    if cfg!(windows) {
        "nova_runtime.lib"
    } else {
        "libnova_runtime.a"
    }
}

/// Locate the runtime static library (spec
/// `docs/superpowers/specs/2026-10-07-phase-3-0-foundations-design.md`
/// §4.2): `NOVA_RUNTIME_LIB` first, then the runtime this `nova` carries,
/// unpacked to its cache, then the executable's neighbours.
fn find_runtime_lib() -> Result<PathBuf> {
    let var = |name: &str| std::env::var_os(name).map(PathBuf::from);
    runtime_cache::locate(
        var("NOVA_RUNTIME_LIB"),
        runtime_cache::embedded_runtime(),
        || {
            runtime_cache::cache_root(
                var("NOVA_HOME"),
                var("USERPROFILE"),
                var("HOME"),
                cfg!(windows),
            )
        },
        || {
            let exe = std::env::current_exe().context("locating the nova executable")?;
            beside_exe(&exe)
        },
    )
}

/// The runtime library in the executable's directory or one of the two
/// above it, where cargo leaves it in a checkout: the CLI binary and the
/// staticlib share a target directory, and test binaries sit one level
/// deeper, in `deps/`.
fn beside_exe(exe: &Path) -> Result<PathBuf> {
    let name = runtime_lib_name();
    for dir in exe.ancestors().skip(1).take(3) {
        let candidate = dir.join(name);
        if candidate.exists() {
            return Ok(candidate);
        }
    }
    bail!(
        "could not find the Nova runtime library ({name}) near {}, and this nova does not \
         carry one. Install nova with the release profile, which is `cargo install`'s \
         default, or build it with NOVA_EMBED_RUNTIME=1; in a checkout, \
         `cargo build -p nova-runtime` puts the library beside it. NOVA_RUNTIME_LIB can \
         also name one.",
        exe.display()
    )
}

/// System libraries the Rust-built runtime staticlib depends on
/// (from `rustc --print native-static-libs` for this toolchain).
///
/// `bcrypt.lib` and `advapi32.lib` were added alongside `std/crypto`:
/// `getrandom` (pulled in transitively by `ring`) resolves its Windows
/// entropy source, `BCryptGenRandom`, through the former, with the latter
/// backing `SystemFunction036` as its fallback on older Windows versions.
#[cfg(windows)]
const MSVC_LIBS: [&str; 8] = [
    "bcrypt.lib",
    "advapi32.lib",
    "kernel32.lib",
    "ntdll.lib",
    "userenv.lib",
    "ws2_32.lib",
    "dbghelp.lib",
    "msvcrt.lib",
];

#[cfg(windows)]
fn link_msvc(object: &Path, runtime: &Path, output: &Path) -> Result<()> {
    let target = match std::env::consts::ARCH {
        "x86_64" => "x86_64-pc-windows-msvc",
        "aarch64" => "aarch64-pc-windows-msvc",
        other => bail!("unsupported Windows architecture for linking: {other}"),
    };
    let tool = cc::windows_registry::find_tool(target, "link.exe").ok_or_else(|| {
        anyhow!(
            "MSVC link.exe not found — install the Visual Studio Build Tools \
             (the same requirement as the Rust MSVC toolchain)"
        )
    })?;
    let mut cmd = tool.to_command();
    cmd.arg("/NOLOGO")
        .arg(format!("/OUT:{}", output.display()))
        .arg("/SUBSYSTEM:CONSOLE")
        .arg(object)
        .arg(runtime)
        .args(MSVC_LIBS);
    run_linker(cmd)
}

#[cfg(not(windows))]
fn link_msvc(_object: &Path, _runtime: &Path, _output: &Path) -> Result<()> {
    unreachable!("MSVC linking is only used on Windows")
}

/// Unix-likes: drive the system C compiler as the linker front-end.
fn link_cc(object: &Path, runtime: &Path, output: &Path) -> Result<()> {
    let cc = std::env::var("CC").unwrap_or_else(|_| "cc".to_string());
    let mut cmd = Command::new(cc);
    cmd.arg("-o")
        .arg(output)
        .arg(object)
        .arg(runtime)
        .args(["-lpthread", "-ldl", "-lm"]);
    run_linker(cmd)
}

fn run_linker(mut cmd: Command) -> Result<()> {
    let output = cmd
        .output()
        .with_context(|| format!("failed to spawn linker: {:?}", cmd.get_program()))?;
    if !output.status.success() {
        bail!(
            "linking failed ({}):\n{}\n{}",
            output.status,
            String::from_utf8_lossy(&output.stdout).trim(),
            String::from_utf8_lossy(&output.stderr).trim(),
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn with_nothing_beside_it_the_error_says_how_to_get_a_runtime() {
        let dir = std::env::temp_dir().join(format!("nova-beside-exe-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("a/b/c")).unwrap();
        let error = beside_exe(&dir.join("a/b/c/nova.exe"))
            .unwrap_err()
            .to_string();
        assert!(error.contains("NOVA_EMBED_RUNTIME=1"), "{error}");
        assert!(error.contains("NOVA_RUNTIME_LIB"), "{error}");
        assert!(error.contains("cargo install"), "{error}");
    }
}
