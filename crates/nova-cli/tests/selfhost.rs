//! The tests of the self-hosted lexer (spec
//! `docs/superpowers/specs/2026-10-30-phase-5-1-the-lexer-the-parser-and-the-ast-design.md`
//! §3 and §6.2, ADR 0051):
//!
//! - S0's hidden `nova parse --dump-tokens FILE…`: absent from `--help`, a
//!   header per file, and `IOERR` with the exit code 2 for a file it cannot
//!   read;
//! - N's own `@test`s, which `nova test` runs in `selfhost/`: as many must
//!   pass as its sources hold.
//!
//! Each test writes its inputs to fixed directories under the system temp
//! directory.

use std::env::consts::EXE_SUFFIX;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

/// The repository, two levels above this crate.
fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join("..")
}

/// How long one process a test starts may run: `nova test`, `nova build`,
/// or one side on one batch. Each takes a second or two here, and CI's
/// Windows runners are several times slower; a lexer that loops forever
/// fails its test at this limit rather than holding CI for hours.
const CHILD_LIMIT: Duration = Duration::from_secs(120);

/// `cmd`'s output, as `Command::output` gives it, if it exits and its
/// stdout and stderr close within `limit`; otherwise an error that names it
/// by `what`. Its stdout and stderr are read on threads of their own, so a
/// child that fills a pipe cannot stall the wait; its stdin is what `cmd`
/// sets.
/// - Past the limit, or if it cannot be waited for, it is killed. `kill`
///   reaches the child alone, not a process it started: an N `@test` that
///   looped would leave its test process running, orphaned, when
///   `nova test` is killed, but that process holds none of these pipes,
///   since `nova test` reads its output itself.
/// - After it exits, its pipes are read to their end within what is left
///   of the limit ([`read_by`]): a process it started may still hold them
///   open, and that is reported rather than waited for. After a kill the
///   readers are not waited for at all.
fn output_within(cmd: &mut Command, limit: Duration, what: &str) -> Result<Output, String> {
    fn drain<R: Read + Send + 'static>(pipe: Option<R>) -> JoinHandle<Vec<u8>> {
        std::thread::spawn(move || {
            let mut bytes = Vec::new();
            if let Some(mut pipe) = pipe {
                let _ = pipe.read_to_end(&mut bytes);
            }
            bytes
        })
    }
    let mut child = cmd
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("{what} could not start: {e}"))?;
    let stdout = drain(child.stdout.take());
    let stderr = drain(child.stderr.take());
    let deadline = Instant::now() + limit;
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) if Instant::now() < deadline => std::thread::sleep(Duration::from_millis(5)),
            Ok(None) => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(format!("{what} did not finish within {limit:?}: killed"));
            }
            Err(e) => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(format!("{what} could not be waited for: {e}: killed"));
            }
        }
    };
    let [stdout, stderr] = read_by([stdout, stderr], deadline).ok_or_else(|| {
        format!(
            "{what} exited ({status}), but its stdout or stderr was still open at \
             {limit:?}: a process it started holds it"
        )
    })?;
    Ok(Output {
        status,
        stdout,
        stderr,
    })
}

/// What each of `readers` read, once both have finished, if they finish by
/// `deadline`; `None` if either is still reading then.
fn read_by(readers: [JoinHandle<Vec<u8>>; 2], deadline: Instant) -> Option<[Vec<u8>; 2]> {
    while readers.iter().any(|reader| !reader.is_finished()) {
        if Instant::now() >= deadline {
            return None;
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    Some(readers.map(|reader| reader.join().expect("read a pipe")))
}

fn nova() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_nova"))
}

/// A fresh, empty directory under the system temp dir. Its name is fixed,
/// so each run replaces the last run's.
fn fresh_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("nova-selfhost-{name}"));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("create the test directory");
    dir
}

#[test]
fn the_token_dump_flag_is_hidden() {
    let help = Command::new(nova())
        .args(["parse", "--help"])
        .output()
        .expect("run nova parse --help");
    let help = String::from_utf8_lossy(&help.stdout);
    assert!(help.contains("--check"), "{help}");
    assert!(!help.contains("dump-tokens"), "{help}");
    // A second file without the flag is refused as it was before the flag.
    let extra = Command::new(nova())
        .args(["parse", "a.nova", "b.nova"])
        .output()
        .expect("run nova parse");
    let stderr = String::from_utf8_lossy(&extra.stderr);
    assert_eq!(extra.status.code(), Some(2), "{stderr}");
    assert!(
        stderr.contains("unexpected argument 'b.nova' found"),
        "{stderr}"
    );
    assert!(!stderr.contains("dump-tokens"), "{stderr}");
    // Its usage line names the binary by the file name it was run by, as
    // clap's own errors do: here a link to `nova` under another name.
    let renamed = fresh_dir("renamed").join(format!("nova-renamed{EXE_SUFFIX}"));
    if std::fs::hard_link(nova(), &renamed).is_err() {
        std::fs::copy(nova(), &renamed).expect("copy nova");
    }
    let extra = Command::new(&renamed)
        .args(["parse", "a.nova", "b.nova"])
        .output()
        .expect("run nova under another name");
    let stderr = String::from_utf8_lossy(&extra.stderr);
    let usage = format!("\nUsage: nova-renamed{EXE_SUFFIX} parse [OPTIONS] <FILE>\n");
    assert!(stderr.contains(&usage), "{stderr}");
    // A first argument with no file name in UTF-8 gives clap no name, and
    // its usage line says `parse` alone. Only Unix's `Command` sets it.
    #[cfg(unix)]
    {
        use std::ffi::OsStr;
        use std::os::unix::ffi::OsStrExt;
        use std::os::unix::process::CommandExt;
        for arg0 in [&b""[..], b"/", b"..", b"\xff\xfe"] {
            let extra = Command::new(nova())
                .arg0(OsStr::from_bytes(arg0))
                .args(["parse", "a.nova", "b.nova"])
                .output()
                .expect("run nova with another first argument");
            let stderr = String::from_utf8_lossy(&extra.stderr);
            assert!(
                stderr.contains("\nUsage: parse [OPTIONS] <FILE>\n"),
                "{arg0:?}: {stderr}"
            );
        }
    }
    // The flag takes neither `--ast` nor `--check`.
    for other in ["--ast", "--check"] {
        let both = Command::new(nova())
            .args(["parse", "--dump-tokens", other, "a.nova"])
            .output()
            .expect("run nova parse");
        let stderr = String::from_utf8_lossy(&both.stderr);
        assert_eq!(both.status.code(), Some(2), "{other}: {stderr}");
        assert!(stderr.contains("cannot be used with"), "{other}: {stderr}");
    }
}

/// Each file's dump follows a header with its path as given; a file that
/// cannot be read is `IOERR` and its kind, the files after it are dumped,
/// and the exit code is 2.
#[test]
fn the_token_dump_has_a_header_per_file_and_ioerr_for_a_missing_one() {
    let dir = fresh_dir("headers");
    std::fs::write(dir.join("a.nova"), "let a = 1\n").expect("write a.nova");
    std::fs::write(dir.join("b.nova"), "b").expect("write b.nova");
    let out = Command::new(nova())
        .current_dir(&dir)
        .args(["parse", "--dump-tokens", "a.nova", "missing.nova", "b.nova"])
        .output()
        .expect("run nova parse --dump-tokens");
    assert_eq!(
        String::from_utf8_lossy(&out.stdout),
        "== a.nova\n0 3 Let\n4 5 Ident \"a\"\n6 7 Eq\n8 9 Int 1 PD\n10 10 Eof NL\n\
         == missing.nova\nIOERR NotFound\n\
         == b.nova\n0 1 Ident \"b\"\n1 1 Eof\n"
    );
    assert_eq!(out.status.code(), Some(2));
}

/// How many `@test` functions N's sources hold: the lines that are `@test`
/// in the `.nova` files of `selfhost/src/` and `selfhost/tests/`.
fn selfhost_test_count() -> usize {
    let mut count = 0;
    for sub in ["src", "tests"] {
        let Ok(entries) = std::fs::read_dir(root().join("selfhost").join(sub)) else {
            continue;
        };
        for entry in entries {
            let path = entry.expect("read a directory entry").path();
            if path.extension().is_some_and(|e| e == "nova") {
                let text = std::fs::read_to_string(&path).expect("read an N source");
                count += text.lines().filter(|l| l.trim() == "@test").count();
            }
        }
    }
    assert!(count > 0, "no @test in selfhost/");
    count
}

/// N's own `@test`s. `nova test` writes nothing into the package, so it
/// runs in place. Each one must pass, so as many must pass as the sources
/// hold: a run of no test at all fails.
#[test]
fn the_selfhost_tests_pass() {
    let home = fresh_dir("test-home");
    let out = output_within(
        Command::new(nova())
            .current_dir(root().join("selfhost"))
            .env("NOVA_HOME", &home)
            .arg("test")
            .stdin(Stdio::null()),
        CHILD_LIMIT,
        "nova test in selfhost/",
    )
    .unwrap_or_else(|e| panic!("{e}"));
    let stdout = String::from_utf8_lossy(&out.stdout);
    let all = format!(" {} passed; 0 failed;", selfhost_test_count());
    assert!(
        out.status.success() && stdout.contains(&all),
        "`{all}` is not in:\n{stdout}{}",
        String::from_utf8_lossy(&out.stderr)
    );
}
