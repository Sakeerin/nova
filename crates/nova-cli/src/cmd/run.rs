//! `nova run [FILE]` — compile and execute a Nova program,
//! `nova build [FILE]` — compile to a standalone executable, a page with
//! `--target wasm`, or with `--ssr` and `--ssg` a project's client and
//! server, and
//! `nova check [FILE]` — type-check without running.
//!
//! With no FILE, each works on the project around the current directory, or
//! on `src/main.nova` outside any project (`crate::project`).

use std::io::{BufRead, BufReader, Write};
use std::path::{Component, Path, PathBuf};
use std::process::{Command, Stdio};

use anyhow::{bail, Context, Result};
use clap::Args;
use nova_driver::{Outcome, Program, Roots};

use crate::bundle::{Bundle, Listing};
use crate::project::{self, Mode};

#[derive(Args)]
pub struct RunCmd {
    /// The Nova source file to run (default: the project's src/main.nova,
    /// found through the nearest nova.toml; or src/main.nova).
    file: Option<PathBuf>,

    /// Arguments for the program, after `--`: `nova run [FILE] -- ARGS...`.
    /// The program's `args()` is FILE followed by these.
    #[arg(last = true)]
    args: Vec<std::ffi::OsString>,
}

#[derive(Args)]
pub struct CheckCmd {
    /// The Nova source file to check (default: as for `nova run`).
    file: Option<PathBuf>,

    /// What to check for: `native`, or `wasm`, with WASM's std and rules.
    #[arg(long, value_enum, default_value_t = TargetArg::Native)]
    target: TargetArg,
}

/// What `--target` names (spec
/// `docs/superpowers/specs/2026-10-14-phase-4-0-foundations-design.md`
/// §3.1).
#[derive(Clone, Copy, Debug, PartialEq, Eq, clap::ValueEnum)]
pub enum TargetArg {
    /// A native executable.
    Native,
    /// A WASM module and its JS shim.
    Wasm,
}

impl From<TargetArg> for nova_driver::Target {
    fn from(target: TargetArg) -> Self {
        match target {
            TargetArg::Native => nova_driver::Target::Native,
            TargetArg::Wasm => nova_driver::Target::Wasm,
        }
    }
}

#[derive(Args)]
pub struct BuildCmd {
    /// The Nova source file to build (default: as for `nova run`).
    file: Option<PathBuf>,

    /// Output executable path (default: target/debug/<name> in a project,
    /// or target/release/<name> with --release; otherwise `<file stem>` in
    /// the current directory; each with the platform executable suffix).
    /// With `--target wasm`, `--ssr` or `--ssg`, the page's directory; the
    /// server of `--ssr` is always target/<profile>/<name>-server.
    #[arg(short, long)]
    output: Option<PathBuf>,

    /// Optimizing build via the LLVM backend (emits LLVM IR and compiles it
    /// with a discovered `clang`/`llc`); the default is the fast Cranelift
    /// backend. With `--target wasm`, optimizes app.wasm for size with
    /// binaryen's `wasm-opt` when it is on PATH. With `--ssr` or `--ssg`,
    /// both builds.
    #[arg(long)]
    release: bool,

    /// What to build for: `native`, an executable (the default); or
    /// `wasm`, a page in a directory (default: dist/ at the project's root,
    /// or in the current directory), which `-o` names.
    #[arg(long, value_enum)]
    target: Option<TargetArg>,

    /// Build a project for server-side rendering: the client from
    /// src/main.nova for WASM, as `--target wasm` builds it, into dist/ or
    /// the directory `-o` names; and the server from src/server.nova,
    /// natively, into target/debug/<name>-server, or
    /// target/release/<name>-server with --release.
    #[arg(long)]
    ssr: bool,

    /// Build as `--ssr` does, then generate the static routes' pages: run
    /// the server from the project's root with `--ssg <dist>`, which writes
    /// dist/<path>/index.html for each, and list them in dist/manifest.json.
    #[arg(long)]
    ssg: bool,
}

#[derive(Args)]
pub struct BundleCmd {
    /// The Nova source file to build (default: as for `nova run`).
    file: Option<PathBuf>,

    /// The directory to write (default: dist/ at the project's root, or in
    /// the current directory).
    #[arg(short, long)]
    output: Option<PathBuf>,
}

/// The program's entry, which `nova run` passes as its first argument and
/// `nova check` names.
fn entry(program: &Program) -> PathBuf {
    program
        .roots
        .first()
        .map(|root| root.path.clone())
        .unwrap_or_default()
}

pub fn run(cmd: RunCmd) -> Result<()> {
    let mode = project::mode(cmd.file)?;
    project::refuse_library(&mode)?;
    project::sync(&mode)?;
    let program = project::program_to_run(&mode)?;
    let mut args = vec![entry(&program).to_string_lossy().into_owned()];
    args.extend(cmd.args.iter().map(|a| a.to_string_lossy().into_owned()));
    match nova_driver::run_program(program, args)? {
        Outcome::Ok(()) => Ok(()),
        Outcome::Failed { errors } => anyhow::bail!(
            "could not compile due to {errors} previous error{}",
            if errors == 1 { "" } else { "s" }
        ),
    }
}

/// `nova bundle`: `nova build --target wasm` (spec
/// `docs/superpowers/specs/2026-10-14-phase-4-0-foundations-design.md`
/// §3.1).
pub fn bundle(cmd: BundleCmd) -> Result<()> {
    build(BuildCmd {
        file: cmd.file,
        output: cmd.output,
        release: false,
        target: Some(TargetArg::Wasm),
        ssr: false,
        ssg: false,
    })
}

/// `nova build --target wasm`: `dist/` at the project's root, or in the
/// current directory, unless `-o` names it (spec 4.0 §7), with what the
/// 4.4 spec's §7 adds: the project's page, `assets/`, `manifest.json`, and
/// `wasm-opt` under `--release`.
fn build_wasm(cmd: BuildCmd) -> Result<()> {
    let mode = project::mode(cmd.file)?;
    project::refuse_library(&mode)?;
    project::sync(&mode)?;
    build_client(&mode, cmd.output, cmd.release).map(|_| ())
}

/// A page built for WASM from `mode`'s program, as `nova build --target
/// wasm` builds it, and `built <dir>` printed. Returns its directory, its
/// bundle, and what its manifest lists.
fn build_client(
    mode: &Mode,
    output: Option<PathBuf>,
    release: bool,
) -> Result<(PathBuf, Bundle, Listing)> {
    let program = project::program_to_run(mode)?;
    let output = wasm_output(output, mode);
    let bundle = Bundle::of(mode, &program, release);
    bundle.check_output(&output)?;
    bundle.prepare(&output)?;
    match nova_driver::build_program_wasm(program, &output)? {
        Outcome::Ok(dir) => {
            let listing = bundle.finish(&dir)?;
            println!("built {}", dir.display());
            Ok((dir, bundle, listing))
        }
        Outcome::Failed { errors } => bail!(could_not_compile(errors)),
    }
}

fn could_not_compile(errors: usize) -> String {
    format!(
        "could not compile due to {errors} previous error{}",
        if errors == 1 { "" } else { "s" }
    )
}

/// `nova build --ssr` and `--ssg` (the 4.6 spec
/// `docs/superpowers/specs/2026-10-19-phase-4-6-ssr-hydration-and-ssg-design.md`
/// §7): the project's client, from `src/main.nova`, into `dist/` or the
/// directory `-o` names, then its server, from `src/server.nova`, into
/// `target/<profile>/<name>-server`. `--ssg` then runs the server with
/// `--ssg <dist>` and lists the pages it wrote in the manifest. What it
/// refuses, it refuses before anything is built.
fn build_ssr(cmd: BuildCmd) -> Result<()> {
    let flag = if cmd.ssg { "--ssg" } else { "--ssr" };
    if cmd.file.is_some() {
        bail!("`{flag}` builds the project's src/main.nova and src/server.nova; it takes no file");
    }
    if cmd.target.is_some() {
        bail!(
            "`{flag}` builds the client for wasm and the server natively; it takes no `--target`"
        );
    }
    let mode = project::mode(None)?;
    let Mode::Project { root, target_dir } = &mode else {
        bail!("`{flag}` builds a project, and there is no nova.toml here or above");
    };
    project::refuse_library(&mode)?;
    if !root.join("src").join("server.nova").is_file() {
        bail!("`{flag}` needs src/server.nova, the server's entry");
    }
    project::sync(&mode)?;
    let (dist, bundle, listing) = build_client(&mode, cmd.output, cmd.release)?;

    let program = mode.program(Roots::Server);
    let dir = target_dir.join(if cmd.release { "release" } else { "debug" });
    if !program.has_errors() {
        std::fs::create_dir_all(&dir).with_context(|| format!("creating {}", dir.display()))?;
    }
    let server = dir.join(format!(
        "{}-server{}",
        project::package_name(&program),
        std::env::consts::EXE_SUFFIX
    ));
    let built = if cmd.release {
        nova_driver::build_program_release(program, &server)?
    } else {
        nova_driver::build_program(program, &server)?
    };
    match built {
        Outcome::Ok(path) => println!("built {}", path.display()),
        Outcome::Failed { errors } => bail!(could_not_compile(errors)),
    }
    if !cmd.ssg {
        return Ok(());
    }
    let pages = generate(root, &server, &dist)?;
    bundle.add_pages(&dist, listing, &pages)
}

/// `--ssg`'s run of the server: from the project's root, with
/// `--ssg <dist>`, its output passed through. Returns the pages it says it
/// wrote, each a path from `dist`, from its `ssg: wrote <file>` lines.
///
/// A server that fails ends `nova` once its output has passed through:
/// `nova` prints `Error: the server failed to generate the pages, with exit
/// code <code>: <server>` and exits with that code; or, when a signal stops
/// the server, which so has no exit code, it prints `…, stopped by signal
/// <number>: …` and exits with 1. A `ssg: wrote` line's file, if relative,
/// is read from the server's working directory, the project's root, and
/// `dist` is made absolute the same way, so the answer does not hang on
/// where `nova` runs; a line whose file does not then lie in `dist`, each
/// name below it a plain one by Rust's path components (no `..`, root or
/// prefix; a `.` piece and a trailing `/` are normalised away), fails the
/// build, with an error naming the line, once the server has ended well;
/// such a file is never listed.
fn generate(root: &Path, server: &Path, dist: &Path) -> Result<Vec<String>> {
    let cwd = std::env::current_dir().context("reading the current directory")?;
    // The server runs in the project's root, so a relative path to it, or
    // to `dist`, is made absolute unless that root is the current directory.
    let from_root = |path: &Path| {
        if path.is_absolute() || root.as_os_str().is_empty() {
            path.to_path_buf()
        } else {
            cwd.join(path)
        }
    };
    let dist = from_root(dist);
    // The server's working directory, absolute, from which a relative path
    // in its lines is read, and `dist` as the server reads it.
    let base = cwd.join(root);
    let dist_from_base = base.join(&dist);
    let mut command = Command::new(cwd.join(server));
    command
        .arg("--ssg")
        .arg(&dist)
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit());
    if !root.as_os_str().is_empty() {
        command.current_dir(root);
    }
    let mut child = command
        .spawn()
        .with_context(|| format!("running {}", server.display()))?;
    let stdout = child.stdout.take().context("the server's output")?;
    let mut pages = Vec::new();
    let mut stray: Option<String> = None;
    let mut out = std::io::stdout();
    for line in BufReader::new(stdout).split(b'\n') {
        let line = line.context("reading the server's output")?;
        out.write_all(&line)
            .and_then(|()| out.write_all(b"\n"))
            .and_then(|()| out.flush())
            .context("writing the server's output")?;
        let text = String::from_utf8_lossy(&line);
        let text = text.trim_end_matches('\r');
        let Some(file) = text.strip_prefix("ssg: wrote ") else {
            continue;
        };
        match page_in(&dist_from_base, &base.join(file)) {
            Some(page) => pages.push(page),
            None => {
                stray.get_or_insert_with(|| text.to_string());
            }
        }
    }
    let status = child
        .wait()
        .with_context(|| format!("running {}", server.display()))?;
    if !status.success() {
        eprintln!(
            "Error: the server failed to generate the pages, {}: {}",
            ended(status),
            server.display()
        );
        std::process::exit(status.code().filter(|&c| c != 0).unwrap_or(1));
    }
    if let Some(line) = stray {
        bail!(
            "the server's line `{line}` names no file inside {}",
            dist.display()
        );
    }
    Ok(pages)
}

/// The page a `ssg: wrote <file>` line names: `file`'s path from `dist`,
/// with `/` between names, when it lies in `dist` and each name below it is
/// a plain one by Rust's path components, with no `..`, root or prefix (a
/// `.` piece and a trailing `/` are normalised away, as `components` does);
/// `None` otherwise, `dist` itself included. Both paths are compared as
/// given, and nothing is asked of the file system: `generate` makes both
/// absolute from the server's working directory first.
fn page_in(dist: &Path, file: &Path) -> Option<String> {
    let page = file.strip_prefix(dist).ok()?;
    let mut names = Vec::new();
    for c in page.components() {
        match c {
            Component::Normal(name) => names.push(name.to_string_lossy().into_owned()),
            _ => return None,
        }
    }
    if names.is_empty() {
        return None;
    }
    Some(names.join("/"))
}

/// How a server that failed ended, in the same words on every system:
/// `with exit code <code>`, or, on Unix, `stopped by signal <number>` when
/// a signal stopped it and it has no code.
fn ended(status: std::process::ExitStatus) -> String {
    if let Some(code) = status.code() {
        return format!("with exit code {code}");
    }
    #[cfg(unix)]
    {
        use std::os::unix::process::ExitStatusExt;
        if let Some(signal) = status.signal() {
            return format!("stopped by signal {signal}");
        }
    }
    "with no exit code".to_string()
}

/// The directory `nova build --target wasm` writes, which `nova dev` builds
/// into and serves: `output` when `-o` names it, or `dist/` at the
/// project's root, or in the current directory.
pub fn wasm_output(output: Option<PathBuf>, mode: &Mode) -> PathBuf {
    match (output, mode) {
        (Some(output), _) => output,
        (None, Mode::Project { root, .. }) => root.join("dist"),
        (None, Mode::File(_)) => PathBuf::from("dist"),
    }
}

pub fn build(cmd: BuildCmd) -> Result<()> {
    if cmd.ssr || cmd.ssg {
        return build_ssr(cmd);
    }
    if cmd.target == Some(TargetArg::Wasm) {
        return build_wasm(cmd);
    }
    let mode = project::mode(cmd.file)?;
    project::refuse_library(&mode)?;
    project::sync(&mode)?;
    let program = project::program_to_run(&mode)?;
    let output = match (cmd.output, &mode) {
        (Some(output), _) => output,
        (None, Mode::Project { target_dir, .. }) => {
            let dir = target_dir.join(if cmd.release { "release" } else { "debug" });
            if !program.has_errors() {
                std::fs::create_dir_all(&dir)
                    .with_context(|| format!("creating {}", dir.display()))?;
            }
            dir.join(format!(
                "{}{}",
                project::package_name(&program),
                std::env::consts::EXE_SUFFIX
            ))
        }
        (None, Mode::File(file)) => {
            let stem = file
                .file_stem()
                .map(|s| s.to_string_lossy().into_owned())
                .unwrap_or_else(|| "out".to_string());
            PathBuf::from(format!("{stem}{}", std::env::consts::EXE_SUFFIX))
        }
    };
    let built = if cmd.release {
        nova_driver::build_program_release(program, &output)?
    } else {
        nova_driver::build_program(program, &output)?
    };
    match built {
        Outcome::Ok(path) => {
            println!("built {}", path.display());
            Ok(())
        }
        Outcome::Failed { errors } => bail!(could_not_compile(errors)),
    }
}

pub fn check(cmd: CheckCmd) -> Result<()> {
    let mode = project::mode(cmd.file)?;
    project::sync(&mode)?;
    let program = mode.program(Roots::Check);
    let entry = entry(&program);
    match nova_driver::check_program_for(program, cmd.target.into())? {
        Outcome::Ok(()) => {
            println!("ok: {}", entry.display());
            Ok(())
        }
        Outcome::Failed { errors } => {
            anyhow::bail!("found {errors} error{}", if errors == 1 { "" } else { "s" })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A page `--ssg` lists lies in the output directory, each name below it
    /// a plain one (ADR 0045).
    #[test]
    fn a_page_lies_in_dist_with_plain_names() {
        let dist = Path::new("out/dist");
        let page = |file: &str| page_in(dist, Path::new(file));
        assert_eq!(page("out/dist/index.html").as_deref(), Some("index.html"));
        assert_eq!(
            page("out/dist/about/index.html").as_deref(),
            Some("about/index.html")
        );
        assert_eq!(page("out/dist/../nova.toml"), None);
        assert_eq!(page("out/dist/a/../../nova.toml"), None);
        assert_eq!(page("out/elsewhere/index.html"), None);
        assert_eq!(page("/out/dist/index.html"), None);
        assert_eq!(page("out/dist"), None);
        assert_eq!(page("out/dist/"), None);
    }

    /// A failed server's end is told by its code, or on Unix by the signal
    /// that stopped it, and never by `ExitStatus`'s own words, which differ
    /// between systems.
    #[cfg(unix)]
    #[test]
    fn a_failed_server_is_told_by_its_code_or_its_signal() {
        use std::os::unix::process::ExitStatusExt;
        let code = std::process::ExitStatus::from_raw(3 << 8);
        assert_eq!(ended(code), "with exit code 3");
        let killed = std::process::ExitStatus::from_raw(9);
        assert_eq!(ended(killed), "stopped by signal 9");
    }
}
