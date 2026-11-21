//! The golden programs of `tests/runtime/` as a built binary runs them
//! (5.0 spec §3.2; ADR 0048), shared by the two walks that build them:
//! `run_tests.rs`'s `release_golden_suite`, S0's `nova build --release`,
//! and `selfhost.rs`'s `s1_builds_the_golden_suite`, N's own `selfhost
//! build` (spec
//! `docs/superpowers/specs/2026-11-19-phase-5-5b-the-driver-and-linking-design.md`
//! §6.1). Both include this file, so there is one copy of the walk, of its
//! tables and of how a binary is run and judged. A file in a subdirectory
//! of `tests/` is no test target of its own; what one includer leaves
//! unused is allowed dead there.

#![allow(dead_code)]

use std::path::{Path, PathBuf};

/// The programs of `tests/runtime/` that `release_golden_suite` leaves out,
/// each with why: what their own tests set up, which a built binary run
/// alone does not have (5.0 spec §3.2).
pub const RELEASE_GOLDEN_EXCLUDED: &[(&str, &str)] = &[
    (
        "process_args",
        "prints `args()`, whose index 0 is the file as `nova run` was given it",
    ),
    ("file_open_dir", "its test makes the directory it opens"),
    (
        "fs_permission_denied",
        "its test makes a file it cannot read, and skips as root",
    ),
    ("net_interleave", "its test serves the port it connects to"),
    ("net_lifetime", "its test serves the port it connects to"),
    ("net_refused", "its test reserves the port it connects to"),
    ("net_roundtrip", "its test serves the port it connects to"),
    ("net_timeout", "its test serves the port it connects to"),
    ("nova_test", "a `@test` file, which `nova test` runs"),
];

/// The standard input `release_golden_suite` gives a program; every other
/// program's is closed.
pub const RELEASE_GOLDEN_STDIN: &[(&str, &str)] = &[("io_streams", "hello-stdin")];

/// The environment variables `release_golden_suite` sets on a program,
/// beside those it inherits: `process_env` reads the two its tests set (5.0
/// spec §5.7).
pub const RELEASE_GOLDEN_ENV: &[(&str, &str, &str)] = &[
    ("process_env", "NOVA_PROCESS_TEST_VALUE", PROCESS_TEST_VALUE),
    ("process_env", "NOVA_PROCESS_TEST_EQ", PROCESS_TEST_EQ),
];

/// The exit status `release_golden_suite` expects of a program, where it is
/// not 0: a program that panics after printing its `.stdout` aborts.
pub const RELEASE_GOLDEN_STATUS: &[(&str, i32)] = &[];

/// How long one `nova build --release`, or one release binary, may run.
pub const RELEASE_GOLDEN_LIMIT: std::time::Duration = std::time::Duration::from_secs(60);

/// How much of a process's standard error a failure quotes: its last
/// `RELEASE_GOLDEN_STDERR` characters.
pub const RELEASE_GOLDEN_STDERR: usize = 2000;

/// `stderr` for a failure's message: its end, where a panic's message is,
/// cut to `RELEASE_GOLDEN_STDERR` characters, or a note that it is empty.
pub fn quoted_stderr(stderr: &str) -> String {
    let stderr = stderr.trim_end();
    if stderr.is_empty() {
        return "\n--- stderr: empty ---".to_string();
    }
    let count = stderr.chars().count();
    if count <= RELEASE_GOLDEN_STDERR {
        return format!("\n--- stderr ---\n{stderr}");
    }
    let tail: String = stderr.chars().skip(count - RELEASE_GOLDEN_STDERR).collect();
    format!("\n--- stderr, its last {RELEASE_GOLDEN_STDERR} of {count} characters ---\n{tail}")
}

/// What `command` did within `RELEASE_GOLDEN_LIMIT`, given `stdin` or a
/// closed standard input: its standard output, its standard error and its
/// exit status, or why it has none, with its standard error quoted.
pub fn run_within_limit(
    mut command: std::process::Command,
    stdin: Option<&str>,
) -> Result<(String, String, Option<i32>), String> {
    use std::io::{Read, Write};
    use std::process::Stdio;
    fn drain(mut pipe: impl Read + Send + 'static) -> std::thread::JoinHandle<Vec<u8>> {
        std::thread::spawn(move || {
            let mut out = Vec::new();
            let _ = pipe.read_to_end(&mut out);
            out
        })
    }
    let mut child = command
        .stdin(if stdin.is_some() {
            Stdio::piped()
        } else {
            Stdio::null()
        })
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("cannot start: {e}"))?;
    let out = drain(child.stdout.take().expect("piped stdout"));
    let err = drain(child.stderr.take().expect("piped stderr"));
    if let Some(text) = stdin {
        let mut pipe = child.stdin.take().expect("piped stdin");
        let _ = pipe.write_all(text.as_bytes());
    }
    let text = |h: std::thread::JoinHandle<Vec<u8>>| {
        String::from_utf8_lossy(&h.join().expect("a reader thread")).replace('\r', "")
    };
    let start = std::time::Instant::now();
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) if start.elapsed() > RELEASE_GOLDEN_LIMIT => {
                let _ = child.kill();
                let _ = child.wait();
                let _ = text(out);
                let stderr = text(err);
                return Err(format!(
                    "timed out after {RELEASE_GOLDEN_LIMIT:?}{}",
                    quoted_stderr(&stderr)
                ));
            }
            Ok(None) => std::thread::sleep(std::time::Duration::from_millis(20)),
            Err(e) => return Err(format!("cannot wait: {e}")),
        }
    };
    Ok((text(out), text(err), status.code()))
}

/// What one release binary did: its standard output, its standard error
/// and its exit status, or why it has none.
pub fn run_release_binary(
    exe: &std::path::Path,
    tmp: &std::path::Path,
    stdin: Option<&str>,
    vars: &[(&str, &str)],
) -> Result<(String, String, Option<i32>), String> {
    let mut command = std::process::Command::new(exe);
    // Each program's temporary files go in a directory of its own, so none
    // races the same fixture's `run` test.
    command
        .env("TMPDIR", tmp)
        .env("TMP", tmp)
        .env("TEMP", tmp)
        .env_remove("NOVA_GC_STRESS")
        .envs(vars.iter().copied());
    run_within_limit(command, stdin)
}

/// The value `process_env`'s tests, and the `--release` walk, give
/// `NOVA_PROCESS_TEST_VALUE`: `=` and non-ASCII included.
pub const PROCESS_TEST_VALUE: &str = "a = b, héllo ✓";

/// The value they give `NOVA_PROCESS_TEST_EQ`: with it set, glibc's
/// `getenv("NOVA_PROCESS_TEST_EQ=x")` finds `y`, and macOS's the whole
/// value, so the program's `None` for that name is `env`'s own refusal of a
/// name holding `=`, seen on every system (5.0 spec §5.7).
pub const PROCESS_TEST_EQ: &str = "x=y";

/// `tests/runtime/`, where the golden programs are.
pub fn golden_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("tests")
        .join("runtime")
}

/// The golden walk: every program of `tests/runtime/` that has a `.stdout`
/// file, by name, sorted, less those `RELEASE_GOLDEN_EXCLUDED` names. The
/// tables must name programs that exist, and the walk must hold at least
/// 5.0a's 131.
pub fn golden_programs() -> Vec<String> {
    let runtime = golden_dir();
    let mut programs: Vec<String> = std::fs::read_dir(&runtime)
        .expect("tests/runtime")
        .filter_map(|e| {
            let path = e.expect("a directory entry").path();
            let stem = path.file_stem()?.to_str()?.to_string();
            let is_nova = path.extension().and_then(|x| x.to_str()) == Some("nova");
            (is_nova && path.with_extension("stdout").exists()).then_some(stem)
        })
        .collect();
    programs.sort();
    for (name, _) in RELEASE_GOLDEN_EXCLUDED {
        assert!(
            programs.iter().any(|p| p == name),
            "the exclusion table names `{name}`, which has no golden program"
        );
    }
    for (name, _, _) in RELEASE_GOLDEN_ENV {
        assert!(
            programs.iter().any(|p| p == name),
            "the environment table names `{name}`, which has no golden program"
        );
    }
    let walk: Vec<String> = programs
        .iter()
        .filter(|p| !RELEASE_GOLDEN_EXCLUDED.iter().any(|(n, _)| n == p))
        .cloned()
        .collect();
    // A floor, 5.0a's count: a walk that found fewer programs would pass
    // having checked less than it should.
    assert!(
        walk.len() >= 131,
        "the walk found {} programs, fewer than 5.0a's 131",
        walk.len()
    );
    walk
}

/// Golden program `name`'s binary `exe` run, its temporary files in `tmp`,
/// with the standard input and the variables the tables give it: `Ok` when
/// its standard output is its `.stdout` file and its exit status the one
/// `RELEASE_GOLDEN_STATUS` expects, else why not.
pub fn run_golden(name: &str, exe: &Path, tmp: &Path) -> Result<(), String> {
    let source = golden_dir().join(format!("{name}.nova"));
    let stdin = RELEASE_GOLDEN_STDIN
        .iter()
        .find(|(n, _)| *n == name)
        .map(|(_, t)| *t);
    let want_status = RELEASE_GOLDEN_STATUS
        .iter()
        .find(|(n, _)| *n == name)
        .map_or(0, |(_, c)| *c);
    let expected = std::fs::read_to_string(source.with_extension("stdout"))
        .expect("the .stdout file")
        .replace('\r', "");
    let vars: Vec<(&str, &str)> = RELEASE_GOLDEN_ENV
        .iter()
        .filter(|(n, _, _)| *n == name)
        .map(|(_, k, v)| (*k, *v))
        .collect();
    let (out, stderr, status) = run_release_binary(exe, tmp, stdin, &vars)?;
    if status != Some(want_status) {
        return Err(format!(
            "exit status {status:?}, expected {want_status}{}",
            quoted_stderr(&stderr)
        ));
    }
    if out != expected {
        return Err(format!(
            "standard output differs\n--- expected ---\n{expected}\
             --- got ---\n{out}{}",
            quoted_stderr(&stderr)
        ));
    }
    Ok(())
}
