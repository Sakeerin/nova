//! `nova dev [FILE]`: build a program into its output directory, serve it on
//! `127.0.0.1`, and build it again when a source changes, then reload the
//! page, or show the build's errors over it (the 4.5 spec
//! `docs/superpowers/specs/2026-10-18-phase-4-5-router-and-nova-dev-design.md`
//! §4). This module is the command, its builds and its watcher; the server
//! and the event stream are `crate::dev::server`.

use std::collections::BTreeMap;
use std::io::Write;
use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime};

use anyhow::{Context, Result};
use clap::Args;

use crate::bundle;
use crate::cmd::run::wasm_output;
use crate::dev::server::{self, Events, Server};
use crate::project::{self, Mode};

#[derive(Args)]
pub struct DevCmd {
    /// The Nova source file to serve (default: as for `nova run`).
    file: Option<PathBuf>,

    /// The port to serve on, on 127.0.0.1; 0 lets the system choose one.
    #[arg(long, default_value_t = 3000)]
    port: u16,

    /// Open the page in the system's browser after the first build.
    #[arg(long)]
    open: bool,

    /// The directory to build into and serve (default: dist/ at the
    /// project's root, or in the current directory).
    #[arg(short, long)]
    output: Option<PathBuf>,
}

/// How often the watcher reads the sources' times and sizes (spec §4.6).
const POLL: Duration = Duration::from_millis(200);

pub fn run(cmd: DevCmd) -> Result<()> {
    // What `nova build --target wasm` refuses, refused before anything is
    // bound or built (spec §4.1, §4.2).
    let mode = project::mode(cmd.file.clone())?;
    project::refuse_library(&mode)?;
    if let Some(root) = project::package_root(&mode) {
        project::refuse_cached(&root)?;
    }
    let out = wasm_output(cmd.output, &mode);
    let root = match &mode {
        Mode::Project { root, .. } => Some(root.as_path()),
        Mode::File(_) => None,
    };
    bundle::check_output_for(root, &out)?;

    let listener = match TcpListener::bind(("127.0.0.1", cmd.port)) {
        Ok(listener) => listener,
        Err(e) if e.kind() == std::io::ErrorKind::AddrInUse => {
            eprintln!(
                "nova dev: port {} is in use; choose another with --port",
                cmd.port
            );
            std::process::exit(1);
        }
        Err(e) => return Err(e).with_context(|| format!("binding 127.0.0.1:{}", cmd.port)),
    };
    let port = listener
        .local_addr()
        .context("reading the bound address")?
        .port();

    let events = Arc::new(Events::started_now());
    let builder = Builder {
        file: cmd.file,
        out: out.clone(),
        events: Arc::clone(&events),
    };
    let watched = Watched::of(&mode, &out);
    // Read before the first build, so that an edit made during it starts
    // another.
    let mut seen = watched.snapshot();
    builder.build();

    let url = format!("http://127.0.0.1:{port}/");
    println!("nova dev: serving {url}");
    let _ = std::io::stdout().flush();
    if cmd.open {
        open(&url);
    }

    std::thread::spawn(move || loop {
        seen = watched.changed(&seen);
        builder.build();
    });
    server::serve(
        listener,
        Server {
            port,
            dist: out,
            events,
        },
    );
    Ok(())
}

/// Runs the builds, one at a time, each in a child process (spec §4.3).
struct Builder {
    file: Option<PathBuf>,
    out: PathBuf,
    events: Arc<Events>,
}

impl Builder {
    /// One build: `rebuilt in <ms> ms` and `reload` when it succeeds; when
    /// it fails, its errors on stderr and in `error`. Whatever else the
    /// child writes on stderr, such as notes, is printed first.
    fn build(&self) {
        let started = Instant::now();
        match self.child() {
            Ok(notes) => {
                eprint!("{notes}");
                eprintln!("rebuilt in {} ms", started.elapsed().as_millis());
                self.events.built(Ok(()));
            }
            Err(errors) => {
                eprint!("{errors}");
                if !errors.ends_with('\n') {
                    eprintln!();
                }
                self.events.built(Err(errors));
            }
        }
    }

    /// `nova build --target wasm [FILE] -o <out>`, the running binary, with
    /// no colour: its stderr, as `Ok` when it succeeds and `Err` when it
    /// fails. Its errors carry no backtrace, which would only fill the
    /// overlay; a panic still has one when `RUST_BACKTRACE` asks.
    fn child(&self) -> Result<String, String> {
        let exe = std::env::current_exe()
            .map_err(|e| format!("nova dev: cannot find the nova binary: {e}"))?;
        let mut command = Command::new(exe);
        command.args(["build", "--target", "wasm"]);
        if let Some(file) = &self.file {
            command.arg(file);
        }
        command
            .arg("-o")
            .arg(&self.out)
            .env("NO_COLOR", "1")
            .env("RUST_LIB_BACKTRACE", "0")
            .stdin(Stdio::null());
        let output = command
            .output()
            .map_err(|e| format!("nova dev: cannot run the build: {e}"))?;
        let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
        if output.status.success() {
            Ok(stderr)
        } else if stderr.trim().is_empty() {
            Err(format!("the build failed: {}\n", output.status))
        } else {
            Err(stderr)
        }
    }
}

/// Each watched file's modification time and size.
type Snapshot = BTreeMap<PathBuf, (Option<SystemTime>, u64)>;

/// What the watcher reads (spec §4.6): for a project, every `.nova` file
/// under its root, `nova.toml`, `index.html` and everything under
/// `public/`; for a single file, the `.nova` files of its directory, not
/// below it, and the file. Under a project's root, a directory named
/// `target`, the output directory and a directory whose name begins with
/// `.` are skipped.
struct Watched {
    /// Where `.nova` files are looked for.
    sources: PathBuf,
    /// Whether they are looked for below `sources` too: for a project, but
    /// not for a single file, whose program cannot import a module from a
    /// directory below its own (E0900), so that `nova dev ~/x.nova` does
    /// not read a whole tree every 200 ms.
    deep: bool,
    /// Files read whether or not they end in `.nova`.
    files: Vec<PathBuf>,
    /// Directories whose every file is read.
    trees: Vec<PathBuf>,
    /// The output directory, never read, so a build does not start another.
    out: PathBuf,
}

impl Watched {
    fn of(mode: &Mode, out: &Path) -> Watched {
        let here = |dir: &Path| {
            if dir.as_os_str().is_empty() {
                PathBuf::from(".")
            } else {
                dir.to_path_buf()
            }
        };
        match mode {
            Mode::Project { root, .. } => Watched {
                sources: here(root),
                deep: true,
                files: vec![root.join(nova_pm::MANIFEST), root.join("index.html")],
                trees: vec![root.join("public")],
                out: out.to_path_buf(),
            },
            Mode::File(file) => Watched {
                sources: here(file.parent().unwrap_or(Path::new(""))),
                deep: false,
                files: vec![file.clone()],
                trees: Vec::new(),
                out: out.to_path_buf(),
            },
        }
    }

    fn snapshot(&self) -> Snapshot {
        let mut seen = Snapshot::new();
        let out = bundle::resolved(&self.out);
        sources(&self.sources, &out, self.deep, &mut seen);
        for file in &self.files {
            stat(file, &mut seen);
        }
        for tree in &self.trees {
            everything(tree, &out, &mut seen);
        }
        seen
    }

    /// Polls every 200 ms until a file is added, removed or changed since
    /// `seen`, then until one more poll sees nothing new: what that last
    /// poll read.
    fn changed(&self, seen: &Snapshot) -> Snapshot {
        settled(seen, || {
            std::thread::sleep(POLL);
            self.snapshot()
        })
    }
}

/// What `poll` returns once it differs from `seen` and one more poll
/// returns the same, so that an editor that writes twice starts one build
/// (spec §4.6).
fn settled<T: PartialEq>(seen: &T, mut poll: impl FnMut() -> T) -> T {
    let mut now = poll();
    while now == *seen {
        now = poll();
    }
    loop {
        let next = poll();
        if next == now {
            return now;
        }
        now = next;
    }
}

/// Whether the directory `dir` is the output directory `out`, resolved.
fn is_out(dir: &Path, out: &Path) -> bool {
    dir.canonicalize().is_ok_and(|dir| dir == out)
}

/// The `.nova` files in `dir`, and when `deep` those below it, as `nova fmt`
/// finds them: directories named `target` or beginning with `.` are
/// skipped, and so is `out`. A link to a directory is not followed.
fn sources(dir: &Path, out: &Path, deep: bool, seen: &mut Snapshot) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().into_owned();
        let Ok(kind) = entry.file_type() else {
            continue;
        };
        if kind.is_dir() {
            if deep && name != "target" && !name.starts_with('.') && !is_out(&path, out) {
                sources(&path, out, deep, seen);
            }
        } else if name.ends_with(".nova") {
            stat(&path, seen);
        }
    }
}

/// Every file under `dir`, `out` aside. A link to a directory is not
/// followed.
fn everything(dir: &Path, out: &Path, seen: &mut Snapshot) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        match entry.file_type() {
            Ok(kind) if kind.is_dir() => {
                if !is_out(&path, out) {
                    everything(&path, out, seen);
                }
            }
            Ok(_) => stat(&path, seen),
            Err(_) => {}
        }
    }
}

/// `path`'s time and size, when it is there, through any link; a missing
/// file is left out, so that its coming and going is a change.
fn stat(path: &Path, seen: &mut Snapshot) {
    match std::fs::metadata(path) {
        Ok(meta) if meta.is_file() => {
            seen.insert(path.to_path_buf(), (meta.modified().ok(), meta.len()));
        }
        Ok(_) => {}
        // A link to nothing is still a file of the program's.
        Err(_) if std::fs::symlink_metadata(path).is_ok() => {
            seen.insert(path.to_path_buf(), (None, 0));
        }
        Err(_) => {}
    }
}

/// The command that opens `url` in the system's browser (spec §4.1).
fn opener(url: &str) -> Command {
    if cfg!(windows) {
        let mut command = Command::new("cmd");
        command.args(["/c", "start", "", url]);
        command
    } else if cfg!(target_os = "macos") {
        let mut command = Command::new("open");
        command.arg(url);
        command
    } else {
        let mut command = Command::new("xdg-open");
        command.arg(url);
        command
    }
}

/// Open `url` in the system's browser. A failure is a note on stderr, not
/// an error: the URL is printed already.
fn open(url: &str) {
    let mut command = opener(url);
    let program = command.get_program().to_string_lossy().into_owned();
    let child = command
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn();
    match child {
        Ok(mut child) => {
            std::thread::spawn(move || match child.wait() {
                Ok(status) if !status.success() => {
                    eprintln!("note: `{program}` could not open a browser: {status}");
                }
                Ok(_) => {}
                Err(e) => eprintln!("note: `{program}` could not open a browser: {e}"),
            });
        }
        Err(e) => eprintln!("note: could not run `{program}` to open a browser: {e}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A fresh directory with a fixed name, so each run replaces the last.
    fn fresh(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("nova-dev-watch-{name}"));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("create the test directory");
        dir
    }

    fn write(dir: &Path, files: &[&str]) {
        for path in files {
            let path = dir.join(path);
            std::fs::create_dir_all(path.parent().unwrap()).expect("a directory");
            std::fs::write(path, "x").expect("a file");
        }
    }

    fn names(dir: &Path, seen: &Snapshot) -> Vec<String> {
        seen.keys()
            .map(|p| {
                p.strip_prefix(dir)
                    .unwrap_or(p)
                    .to_string_lossy()
                    .replace('\\', "/")
            })
            .collect()
    }

    #[test]
    fn a_project_watches_its_sources_manifest_page_and_public() {
        let dir = fresh("project");
        write(
            &dir,
            &[
                "nova.toml",
                "index.html",
                "notes.md",
                "src/main.nova",
                "src/ui/view.nova",
                "src/ui/view.txt",
                "tests/t.nova",
                "public/logo.png",
                "public/css/site.css",
                "target/debug/gen.nova",
                "src/target/skip.nova",
                ".git/hook.nova",
                "dist/app.nova",
                "build/out.nova",
            ],
        );
        let mode = Mode::Project {
            root: dir.clone(),
            target_dir: dir.join("target"),
        };
        let seen = Watched::of(&mode, &dir.join("dist")).snapshot();
        assert_eq!(
            names(&dir, &seen),
            [
                "build/out.nova",
                "index.html",
                "nova.toml",
                "public/css/site.css",
                "public/logo.png",
                "src/main.nova",
                "src/ui/view.nova",
                "tests/t.nova",
            ]
        );
        // `-o` elsewhere in the project skips that directory instead, by
        // any spelling.
        let seen = Watched::of(&mode, &dir.join("src").join("..").join("build")).snapshot();
        assert!(names(&dir, &seen).contains(&"dist/app.nova".to_string()));
        assert!(!names(&dir, &seen).contains(&"build/out.nova".to_string()));
    }

    /// The `.nova` files of the file's directory, not those below it, which
    /// its program cannot import (E0900): a change to one starts no build.
    #[test]
    fn a_single_file_watches_the_nova_files_of_its_directory() {
        let dir = fresh("file");
        write(
            &dir,
            &[
                "app/main.nova",
                "app/util.nova",
                "app/page.html",
                "app/dist/x.nova",
                "app/sub/deep.nova",
                "app/sub/deeper/deepest.nova",
                "other.nova",
            ],
        );
        let mode = Mode::File(dir.join("app").join("main.nova"));
        let watched = Watched::of(&mode, &dir.join("app").join("dist"));
        let seen = watched.snapshot();
        assert_eq!(names(&dir, &seen), ["app/main.nova", "app/util.nova"]);
        std::fs::write(dir.join("app/sub/deep.nova"), "longer").unwrap();
        std::fs::write(dir.join("app/sub/new.nova"), "x").unwrap();
        assert_eq!(watched.snapshot(), seen);
    }

    #[test]
    fn a_change_an_addition_and_a_removal_are_each_seen() {
        let dir = fresh("changes");
        write(&dir, &["nova.toml", "src/main.nova"]);
        let mode = Mode::Project {
            root: dir.clone(),
            target_dir: dir.join("target"),
        };
        let watched = Watched::of(&mode, &dir.join("dist"));
        let first = watched.snapshot();
        std::fs::write(dir.join("src/main.nova"), "longer").unwrap();
        let second = watched.changed(&first);
        assert_ne!(first, second);
        std::fs::write(dir.join("src/more.nova"), "x").unwrap();
        let third = watched.changed(&second);
        assert_eq!(third.len(), 3);
        std::fs::remove_file(dir.join("src/more.nova")).unwrap();
        let fourth = watched.changed(&third);
        assert_eq!(fourth, second);
        // The output directory changing is no change.
        std::fs::create_dir_all(dir.join("dist")).unwrap();
        std::fs::write(dir.join("dist/app.nova"), "x").unwrap();
        assert_eq!(watched.snapshot(), fourth);
    }

    /// A link to nothing is still a file of the program's, with no time and
    /// no size, so that its target's coming and going is a change.
    #[cfg(unix)]
    #[test]
    fn a_link_to_nothing_is_watched_as_a_file() {
        let dir = fresh("dangling");
        write(&dir, &["nova.toml", "src/main.nova"]);
        let link = dir.join("src").join("linked.nova");
        std::os::unix::fs::symlink(dir.join("src").join("gone.nova"), &link).expect("a link");
        let mode = Mode::Project {
            root: dir.clone(),
            target_dir: dir.join("target"),
        };
        let watched = Watched::of(&mode, &dir.join("dist"));
        let dangling = watched.snapshot();
        assert_eq!(dangling.get(&link), Some(&(None, 0)));
        assert_eq!(dangling.len(), 3);
        std::fs::write(dir.join("src").join("gone.nova"), "back").unwrap();
        let back = watched.snapshot();
        assert_eq!(back.get(&link).map(|(_, size)| *size), Some(4));
        assert_ne!(back, dangling);
    }

    #[test]
    fn a_build_waits_for_a_poll_that_sees_nothing_new() {
        // Polls of a file written three times, then left alone: the change
        // is seen at the third poll, and settles at the sixth.
        let mut polls = [0, 0, 1, 2, 3, 3, 4].into_iter();
        let mut taken = 0;
        let settled = settled(&0, || {
            taken += 1;
            polls.next().expect("a poll")
        });
        assert_eq!((settled, taken), (3, 6));
    }

    #[test]
    fn the_opener_is_the_systems() {
        let command = opener("http://127.0.0.1:1/");
        let args: Vec<_> = command
            .get_args()
            .map(|a| a.to_string_lossy().into_owned())
            .collect();
        let program = command.get_program().to_string_lossy().into_owned();
        if cfg!(windows) {
            assert_eq!(program, "cmd");
            assert_eq!(args, ["/c", "start", "", "http://127.0.0.1:1/"]);
        } else if cfg!(target_os = "macos") {
            assert_eq!(program, "open");
            assert_eq!(args, ["http://127.0.0.1:1/"]);
        } else {
            assert_eq!(program, "xdg-open");
            assert_eq!(args, ["http://127.0.0.1:1/"]);
        }
    }
}
