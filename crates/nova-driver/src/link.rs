//! Native linking for `nova build`: locate the `nova-runtime` static
//! library and invoke the platform linker (spec `14-CODEGEN.md` §11 —
//! system linker via cc-rs).

use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{anyhow, bail, Context, Result};

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

/// Locate the runtime static library: `NOVA_RUNTIME_LIB` override first,
/// then next to the `nova` executable (cargo places both the CLI binary
/// and the staticlib in the same target directory; test binaries live one
/// level deeper in `deps/`).
fn find_runtime_lib() -> Result<PathBuf> {
    if let Some(p) = std::env::var_os("NOVA_RUNTIME_LIB") {
        let p = PathBuf::from(p);
        if p.exists() {
            return Ok(p);
        }
        bail!(
            "NOVA_RUNTIME_LIB points to {}, which does not exist",
            p.display()
        );
    }
    let exe = std::env::current_exe().context("locating the nova executable")?;
    let name = runtime_lib_name();
    for dir in exe.ancestors().skip(1).take(3) {
        let candidate = dir.join(name);
        if candidate.exists() {
            return Ok(candidate);
        }
    }
    bail!(
        "could not find the Nova runtime library ({name}) near {}; \
         build it with `cargo build -p nova-runtime` or set NOVA_RUNTIME_LIB",
        exe.display()
    )
}

/// System libraries the Rust-built runtime staticlib depends on
/// (from `rustc --print native-static-libs` for this toolchain).
#[cfg(windows)]
const MSVC_LIBS: [&str; 6] = [
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
