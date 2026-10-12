//! The install scripts, `tools/install/install.sh` and `install.ps1` (spec
//! `docs/superpowers/specs/2026-10-12-phase-3-6a-release-preparation-design.md`
//! §6, §11.3), against a fake release that the loopback server serves: the
//! host's archive, packed as release.yml packs it around this test's own
//! `nova`, and its `SHA256SUMS`.
//!
//! `install.sh` runs (with `sh`, which is dash on Ubuntu) on Linux and
//! macOS; `install.ps1` runs in Windows PowerShell 5.1 on Windows, with
//! `NOVA_INSTALL_NO_PATH=1` so the user's PATH is never touched. Each run
//! has `NOVA_HOME` (and on Unix `HOME`) in the test's own directory, no
//! proxy, and `NOVA_INSTALL_BASE` at the loopback server, so every request
//! goes there. On a system the scripts have no archive for, only the tests
//! that need none run.

#[path = "../../nova-index/tests/support/mod.rs"]
mod support;

use std::collections::HashMap;
use std::env::consts::EXE_SUFFIX;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::{Arc, Mutex};

use support::{Response, Server};

/// The release the `latest` redirect names, and an older one.
const LATEST: &str = "1.2.3";
const OLDER: &str = "1.2.2";

/// The target whose archive the scripts pick on this system (spec §6.1
/// step 1), or `None` where there is none.
fn host_target() -> Option<&'static str> {
    if cfg!(all(target_os = "linux", target_arch = "x86_64")) {
        Some("x86_64-unknown-linux-gnu")
    } else if cfg!(all(target_os = "macos", target_arch = "aarch64")) {
        Some("aarch64-apple-darwin")
    } else if cfg!(all(target_os = "macos", target_arch = "x86_64")) {
        Some("x86_64-apple-darwin")
    } else if cfg!(all(target_os = "windows", target_arch = "x86_64")) {
        Some("x86_64-pc-windows-msvc")
    } else {
        None
    }
}

/// The host's target, or `None` after saying the test is skipped.
fn target_or_skip() -> Option<&'static str> {
    let target = host_target();
    if target.is_none() {
        eprintln!("skipped: the install scripts have no archive for this system");
    }
    target
}

/// `nova-<version>-<target>.tar.gz`, or `.zip` on Windows (release.yml).
fn archive_name(version: &str, target: &str) -> String {
    let extension = if cfg!(windows) { "zip" } else { "tar.gz" };
    format!("nova-{version}-{target}.{extension}")
}

fn repo() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("the repository holds crates/nova-cli")
        .to_path_buf()
}

/// A fresh, empty directory with a fixed name, so each run replaces the last.
fn fresh(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("nova-install-scripts-{name}"));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("create the test directory");
    dir
}

/// This test's own `nova`, which the fake archives carry.
fn nova() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_nova"))
}

fn stdout(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).into_owned()
}

fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

/// Run `command`, which must succeed, and return its stdout.
fn run_ok(command: &mut Command) -> String {
    let output = command
        .output()
        .unwrap_or_else(|e| panic!("running {command:?}: {e}"));
    assert!(
        output.status.success(),
        "{command:?} failed\n{}\n{}",
        stdout(&output),
        stderr(&output)
    );
    stdout(&output)
}

/// Windows PowerShell 5.1, started without the `PSModulePath` this test
/// inherited. Under PowerShell 7, which runs CI's `cargo test` on Windows,
/// that variable leads with PowerShell 7's module paths, and Windows
/// PowerShell started through another process, as here, then cannot load
/// its own `Microsoft.PowerShell.*` modules; without the variable it builds
/// its default (Microsoft's `about_PSModulePath`).
fn windows_powershell() -> Command {
    let mut command = Command::new("powershell");
    command.env_remove("PSModulePath");
    command
}

/// The SHA-256 of `path`, in lower-case hex, from the system's own tool.
fn sha256(path: &Path) -> String {
    let printed = if cfg!(windows) {
        run_ok(
            windows_powershell()
                .args(["-NoProfile", "-NonInteractive", "-Command"])
                .arg("(Get-FileHash -LiteralPath $env:HASH_FILE -Algorithm SHA256).Hash")
                .env("HASH_FILE", path),
        )
    } else if Command::new("sha256sum").arg("--version").output().is_ok() {
        run_ok(Command::new("sha256sum").arg(path))
    } else {
        run_ok(Command::new("shasum").args(["-a", "256"]).arg(path))
    };
    let hash = printed
        .split_whitespace()
        .next()
        .unwrap_or("")
        .to_ascii_lowercase();
    assert!(
        hash.len() == 64 && hash.bytes().all(|b| b.is_ascii_hexdigit()),
        "not a SHA-256: {printed}"
    );
    hash
}

/// Pack release `version`'s archive for `target` in `dir` as release.yml
/// does: one directory, `nova-<version>-<target>`, holding `nova` (or
/// `nova.exe`), `README.md` and both licences; `tar -czf` on Unix and
/// `Compress-Archive` on Windows. Returns the archive's path. The packed
/// directory is removed, as each copy of a debug `nova` is large.
fn pack(dir: &Path, version: &str, target: &str) -> PathBuf {
    let name = format!("nova-{version}-{target}");
    let stage = dir.join(format!("stage-{version}"));
    let inner = stage.join(&name);
    std::fs::create_dir_all(&inner).expect("create the archive's directory");
    for file in ["README.md", "LICENSE-MIT", "LICENSE-APACHE"] {
        std::fs::copy(repo().join(file), inner.join(file)).expect("copy a release file");
    }
    std::fs::copy(nova(), inner.join(format!("nova{EXE_SUFFIX}"))).expect("copy nova");
    let archive = archive_name(version, target);
    if cfg!(windows) {
        run_ok(
            windows_powershell()
                .args(["-NoProfile", "-NonInteractive", "-Command"])
                .arg("Compress-Archive -LiteralPath $env:PACK_SOURCE -DestinationPath $env:PACK_ARCHIVE")
                .env("PACK_SOURCE", &inner)
                .env("PACK_ARCHIVE", stage.join(&archive)),
        );
    } else {
        run_ok(
            Command::new("tar")
                .arg("-czf")
                .arg(&archive)
                .arg(&name)
                .current_dir(&stage),
        );
    }
    std::fs::remove_dir_all(&inner).expect("remove the packed directory");
    stage.join(archive)
}

/// What a fake release's `SHA256SUMS` says of its archive.
enum Sums {
    /// The archive's own SHA-256.
    Right,
    /// Another SHA-256.
    Wrong,
    /// Nothing: the archive has no line.
    Missing,
}

/// Release `version`'s files for `target`, each by its path under
/// `/releases/download/`: the archive, and a `SHA256SUMS` that also lists
/// another asset, as a real release's does.
fn release(dir: &Path, version: &str, target: &str, sums: Sums) -> HashMap<String, Vec<u8>> {
    let archive = pack(dir, version, target);
    let name = archive.file_name().unwrap().to_str().unwrap().to_string();
    let hash = sha256(&archive);
    let other = format!(
        "{}  nova-vscode-{version}.vsix\n",
        "0123456789abcdef".repeat(4)
    );
    let text = match sums {
        Sums::Right => format!("{hash}  {name}\n{other}"),
        Sums::Wrong => {
            let wrong = if hash.starts_with("ab") { "cd" } else { "ab" }.repeat(32);
            format!("{wrong}  {name}\n{other}")
        }
        Sums::Missing => other,
    };
    let bytes = std::fs::read(&archive).expect("read the archive");
    HashMap::from([
        (format!("v{version}/{name}"), bytes),
        (format!("v{version}/SHA256SUMS"), text.into_bytes()),
    ])
}

/// A loopback stand-in for `github.com/Sakeerin/nova/releases`: `latest`
/// redirects to `tag/v<LATEST>`, which answers with a page, and
/// `download/<path>` answers with `files`' entry; anything else is a 404.
fn serve(files: HashMap<String, Vec<u8>>) -> Server {
    let url = Arc::new(Mutex::new(String::new()));
    let base = Arc::clone(&url);
    let server = Server::start(move |request| {
        let path = request.path.as_str();
        if path == "/releases/latest" {
            let base = base.lock().unwrap().clone();
            return Response::redirect(&format!("{base}/releases/tag/v{LATEST}"));
        }
        if path == format!("/releases/tag/v{LATEST}") {
            return Response::ok(format!("<html><title>Nova {LATEST}</title></html>"));
        }
        match path
            .strip_prefix("/releases/download/")
            .and_then(|file| files.get(file))
        {
            Some(bytes) => Response::ok(bytes.clone()),
            None => Response::status(404),
        }
    });
    *url.lock().unwrap() = server.url.clone();
    server
}

/// The paths the server was asked for, in order.
fn paths(server: &Server) -> Vec<String> {
    server.requests().into_iter().map(|r| r.path).collect()
}

/// The host's install script, run in `dir` against `server`, with
/// `NOVA_HOME` in `dir/nova-home`, on Unix `HOME` in `dir/home`, no
/// `NOVA_VERSION` and no proxy. On Windows, `NOVA_INSTALL_NO_PATH=1`.
fn script(dir: &Path, server: &Server) -> Command {
    let mut command = if cfg!(windows) {
        let mut command = windows_powershell();
        command
            .args([
                "-NoProfile",
                "-NonInteractive",
                "-ExecutionPolicy",
                "Bypass",
                "-File",
            ])
            .arg(repo().join("tools").join("install").join("install.ps1"))
            .env("NOVA_INSTALL_NO_PATH", "1");
        command
    } else {
        let home = dir.join("home");
        std::fs::create_dir_all(&home).expect("create HOME");
        let mut command = Command::new("sh");
        command
            .arg(repo().join("tools").join("install").join("install.sh"))
            .env("HOME", home);
        command
    };
    command
        .current_dir(dir)
        .env("NOVA_INSTALL_BASE", format!("{}/releases", server.url))
        .env("NOVA_HOME", dir.join("nova-home"))
        .env_remove("NOVA_VERSION");
    for proxy in [
        "http_proxy",
        "HTTP_PROXY",
        "https_proxy",
        "HTTPS_PROXY",
        "all_proxy",
        "ALL_PROXY",
    ] {
        command.env_remove(proxy);
    }
    command
}

fn run(command: &mut Command) -> Output {
    command.output().expect("run the install script")
}

/// `dir/nova-home/bin`, where the scripts install.
fn bin(dir: &Path) -> PathBuf {
    dir.join("nova-home").join("bin")
}

fn entries(dir: &Path) -> Vec<String> {
    let mut names: Vec<String> = std::fs::read_dir(dir)
        .unwrap_or_else(|e| panic!("{}: {e}", dir.display()))
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();
    names
}

/// `bin` holds this test's `nova` and nothing else.
fn assert_installed(bin: &Path) {
    let exe = format!("nova{EXE_SUFFIX}");
    assert_eq!(entries(bin), [exe.as_str()], "{}", bin.display());
    assert!(
        std::fs::read(bin.join(&exe)).unwrap() == std::fs::read(nova()).unwrap(),
        "{} is not the release's nova",
        bin.join(&exe).display()
    );
}

/// `out` holds every line this test's `nova version` prints, in order.
fn assert_prints_version(out: &str) {
    let ours = run_ok(Command::new(nova()).arg("version"));
    let ours: Vec<&str> = ours.lines().collect();
    assert!(ours[0].starts_with("nova "), "{ours:?}");
    let printed: Vec<&str> = out.lines().collect();
    assert!(
        printed
            .windows(ours.len())
            .any(|lines| lines == ours.as_slice()),
        "the installed nova version is not in:\n{out}"
    );
}

/// The paths of release `version`'s two downloads.
fn downloads(version: &str, target: &str) -> [String; 2] {
    [
        format!(
            "/releases/download/v{version}/{}",
            archive_name(version, target)
        ),
        format!("/releases/download/v{version}/SHA256SUMS"),
    ]
}

#[test]
fn installs_the_latest_release_found_through_the_redirect() {
    let Some(target) = target_or_skip() else {
        return;
    };
    let dir = fresh("latest");
    let server = serve(release(&dir, LATEST, target, Sums::Right));

    let output = run(&mut script(&dir, &server));
    let (out, err) = (stdout(&output), stderr(&output));
    assert!(output.status.success(), "{out}\n{err}");
    assert_installed(&bin(&dir));
    assert_prints_version(&out);

    // install.sh's curl follows the redirect to the release's page;
    // install.ps1 only reads where it points.
    let mut expected = vec!["/releases/latest".to_string()];
    if !cfg!(windows) {
        expected.push(format!("/releases/tag/v{LATEST}"));
    }
    expected.extend(downloads(LATEST, target));
    assert_eq!(paths(&server), expected);

    // NOVA_HOME is not under HOME, so install.sh names the directory itself.
    if !cfg!(windows) {
        let line = format!(
            "nova: add nova to your PATH: export PATH=\"{}:$PATH\"",
            bin(&dir).display()
        );
        assert!(out.lines().any(|l| l == line), "no `{line}` in:\n{out}");
    }
}

#[test]
fn nova_version_fetches_that_version_and_never_asks_for_latest() {
    let Some(target) = target_or_skip() else {
        return;
    };
    let dir = fresh("chosen-version");
    let server = serve(release(&dir, OLDER, target, Sums::Right));

    let output = run(script(&dir, &server).env("NOVA_VERSION", OLDER));
    let (out, err) = (stdout(&output), stderr(&output));
    assert!(output.status.success(), "{out}\n{err}");
    assert_installed(&bin(&dir));
    assert_prints_version(&out);
    assert_eq!(paths(&server), downloads(OLDER, target));
}

#[test]
fn a_checksum_mismatch_installs_nothing() {
    let Some(target) = target_or_skip() else {
        return;
    };
    let dir = fresh("mismatch");
    let server = serve(release(&dir, LATEST, target, Sums::Wrong));
    // An older nova is installed already; a failed install leaves it.
    let exe = bin(&dir).join(format!("nova{EXE_SUFFIX}"));
    std::fs::create_dir_all(bin(&dir)).unwrap();
    std::fs::write(&exe, "an older nova").unwrap();

    let output = run(&mut script(&dir, &server));
    let (out, err) = (stdout(&output), stderr(&output));
    assert!(!output.status.success(), "{out}\n{err}");
    let archive = archive_name(LATEST, target);
    assert!(
        err.contains(&archive),
        "the error does not name {archive}:\n{err}"
    );
    assert_eq!(std::fs::read_to_string(&exe).unwrap(), "an older nova");
    assert_eq!(entries(&bin(&dir)), [format!("nova{EXE_SUFFIX}")]);
}

#[test]
fn an_archive_missing_from_sha256sums_installs_nothing() {
    let Some(target) = target_or_skip() else {
        return;
    };
    let dir = fresh("missing");
    let server = serve(release(&dir, LATEST, target, Sums::Missing));

    let output = run(&mut script(&dir, &server));
    let (out, err) = (stdout(&output), stderr(&output));
    assert!(!output.status.success(), "{out}\n{err}");
    let archive = archive_name(LATEST, target);
    assert!(
        err.contains(&archive),
        "the error does not name {archive}:\n{err}"
    );
    assert!(err.contains("SHA256SUMS"), "{err}");
    assert!(!dir.join("nova-home").exists(), "something was installed");
}

#[test]
fn running_it_again_replaces_the_binary() {
    let Some(target) = target_or_skip() else {
        return;
    };
    let dir = fresh("again");
    let server = serve(release(&dir, LATEST, target, Sums::Right));

    let first = run(&mut script(&dir, &server));
    assert!(
        first.status.success(),
        "{}\n{}",
        stdout(&first),
        stderr(&first)
    );
    assert_installed(&bin(&dir));
    // Stand in for an older nova; the second run must replace it.
    std::fs::write(bin(&dir).join(format!("nova{EXE_SUFFIX}")), "an older nova").unwrap();

    let second = run(&mut script(&dir, &server));
    let (out, err) = (stdout(&second), stderr(&second));
    assert!(second.status.success(), "{out}\n{err}");
    assert_installed(&bin(&dir));
    assert_prints_version(&out);
}

#[cfg(unix)]
#[test]
fn install_sh_prints_the_path_line_only_when_the_directory_is_not_on_path() {
    let Some(target) = target_or_skip() else {
        return;
    };
    let dir = fresh("path");
    let server = serve(release(&dir, LATEST, target, Sums::Right));
    // NOVA_HOME is HOME's .nova, as by default, so the line shows $HOME.
    let nova_home = dir.join("home").join(".nova");
    let path = std::env::var("PATH").unwrap_or_default();
    let line = "nova: add nova to your PATH: export PATH=\"$HOME/.nova/bin:$PATH\"";

    let output = run(script(&dir, &server)
        .env("NOVA_HOME", &nova_home)
        .env("PATH", &path));
    let (out, err) = (stdout(&output), stderr(&output));
    assert!(output.status.success(), "{out}\n{err}");
    assert!(out.lines().any(|l| l == line), "no `{line}` in:\n{out}");

    let on_path = format!("{}:{path}", nova_home.join("bin").display());
    let output = run(script(&dir, &server)
        .env("NOVA_HOME", &nova_home)
        .env("PATH", on_path));
    let (out, err) = (stdout(&output), stderr(&output));
    assert!(output.status.success(), "{out}\n{err}");
    assert!(
        !out.contains("add nova to your PATH"),
        "a PATH line, though nova is on PATH:\n{out}"
    );
    assert_installed(&nova_home.join("bin"));
}

#[cfg(unix)]
#[test]
fn install_sh_on_an_unknown_system_names_cargo_install() {
    use std::os::unix::fs::PermissionsExt;

    let dir = fresh("unknown-system");
    let server = serve(HashMap::new());
    let fake = dir.join("fake-bin");
    std::fs::create_dir_all(&fake).unwrap();
    let uname = fake.join("uname");
    let path = format!(
        "{}:{}",
        fake.display(),
        std::env::var("PATH").unwrap_or_default()
    );

    for (os, arch) in [("FreeBSD", "amd64"), ("Linux", "aarch64")] {
        std::fs::write(
            &uname,
            format!("#!/bin/sh\ncase \"$1\" in\n  -m) echo {arch} ;;\n  *) echo {os} ;;\nesac\n"),
        )
        .unwrap();
        std::fs::set_permissions(&uname, std::fs::Permissions::from_mode(0o755)).unwrap();

        let output = run(script(&dir, &server).env("PATH", &path));
        let (out, err) = (stdout(&output), stderr(&output));
        assert!(!output.status.success(), "{out}\n{err}");
        let message = format!(
            "no ready-built nova for {os} {arch}; install it with: cargo install nova-cli --locked"
        );
        assert!(err.contains(&message), "no `{message}` in:\n{err}");
    }
    assert_eq!(paths(&server), Vec::<String>::new());
    assert!(!dir.join("nova-home").exists());
}

#[cfg(windows)]
#[test]
fn install_ps1_on_an_unknown_system_names_cargo_install() {
    let dir = fresh("unknown-system");
    let server = serve(HashMap::new());

    let output = run(script(&dir, &server)
        .env("PROCESSOR_ARCHITECTURE", "ARM64")
        .env_remove("PROCESSOR_ARCHITEW6432"));
    let (out, err) = (stdout(&output), stderr(&output));
    assert!(!output.status.success(), "{out}\n{err}");
    let message =
        "no ready-built nova for Windows_NT ARM64; install it with: cargo install nova-cli --locked";
    assert!(err.contains(message), "no `{message}` in:\n{err}");
    assert_eq!(paths(&server), Vec::<String>::new());
    assert!(!dir.join("nova-home").exists());
}

/// Windows PowerShell 5.1 reads a script that has no byte-order mark in
/// the system's code page, and `irm | iex` would choke on a mark, so both
/// scripts are plain ASCII; `install.sh` also has Unix line ends, for `sh`.
#[test]
fn the_scripts_are_plain_ascii() {
    for name in ["install.sh", "install.ps1"] {
        let path = repo().join("tools").join("install").join(name);
        let bytes = std::fs::read(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
        if let Some(at) = bytes.iter().position(|b| !b.is_ascii()) {
            panic!("{name} has a byte that is not ASCII at offset {at}");
        }
    }
    let sh =
        std::fs::read_to_string(repo().join("tools").join("install").join("install.sh")).unwrap();
    assert!(
        sh.starts_with("#!/bin/sh\n"),
        "install.sh does not start with #!/bin/sh"
    );
    assert!(!sh.contains('\r'), "install.sh has a carriage return");
}
