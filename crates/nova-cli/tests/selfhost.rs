//! The tests of the self-hosted lexer (spec
//! `docs/superpowers/specs/2026-10-30-phase-5-1-the-lexer-the-parser-and-the-ast-design.md`
//! §3 and §6.2, ADR 0051):
//!
//! - S0's hidden `nova parse --dump-tokens FILE…`: absent from `--help`, a
//!   header per file, and `IOERR` with the exit code 2 for a file it cannot
//!   read;
//! - N's own `@test`s, which `nova test` runs in `selfhost/`: as many must
//!   pass as its sources hold;
//! - the differential: N, the Nova package in `selfhost/`, built once by
//!   `nova build`, must print the same token dump as S0, byte for byte, on
//!   every `.nova` file of the repository and N's own sources, also with a
//!   Windows checkout's line endings; on the fixed cases in
//!   `tests/selfhost/cases/`; on the edge inputs; and on the fuzzer's soup
//!   and mutated corpus files from fixed seeds (`tests/selfhost/fuzz.rs`).
//!
//! 5.1b (spec §4, ADR 0052): the hidden `nova parse --dump-ast FILE…`, and
//! N's `selfhost ast FILE…`, which must print the same tree, errors and
//! notes byte for byte on every input set above, the token dump's beside
//! it, and must each name every kind of node somewhere in the corpus and
//! the cases; the error cases in `tests/selfhost/cases/`, the lexer's and
//! the parser's tests' inline sources; the deep inputs, through both
//! differentials, through S0's `parse --check` and `check`, which must meet
//! the depth limit, P0002, and never overflow their stack, and through N on
//! a 1 MB main thread, Windows' size, at 128 and 129 levels; and S0's
//! big-stack thread, which keeps a panic's message and exit code, and
//! leaves `nova run`'s program on the main thread.
//!
//! 5.2a (spec
//! `docs/superpowers/specs/2026-11-02-phase-5-2-the-loader-and-the-resolver-design.md`
//! §3.4–§3.7; ADR 0053): S0's hidden `nova check --dump-resolution FILE…`
//! and `--std`, absent from `--help`, with every other `check` command
//! line parsed as before; and N's `selfhost resolve --std DIR FILE…` and
//! `--std-part`, which must print the same resolution dump byte for byte,
//! with the same exit codes: std's part once; every loose corpus program,
//! each file run alone or a root with the sibling files it imports
//! (packages are 5.2b's); the resolver's error cases in
//! `tests/selfhost/cases/resolve/`; and the name-level fuzzer's inputs
//! from fixed seeds, among which every resolver code `check` can give
//! natively occurs. Every form of the dump appears in those dumps.
//!
//! 5.2b (spec §4; ADR 0054): S0's hidden dump also takes a package's
//! module as a root (a file directly in a package's `src/` or `tests/`),
//! `--target wasm`, and `--test`, `nova test`'s resolution; std's WASM and
//! test-mode parts, the corpus's packages, its WASM programs and its files
//! with tests, the cases of each mode (`cases/resolve-wasm/`,
//! `cases/resolve-test/`) and the name fuzzer's package, WASM and
//! test-mode inputs go through it, among which every one of the resolver's
//! 11 codes, E0004 and E0001's two notes occur. N's `selfhost resolve`
//! takes the same roots, `--target wasm` and `--test`, and must print the
//! same dump of each, byte for byte, with `\` read as `/` in the paths of
//! E0004's and E0093's messages on Windows; a manifest N's minimal reader
//! refuses stops it, with a message, and no input of the differential has
//! one.
//!
//! `NOVA_SELFHOST_FUZZ=SEED:COUNT` runs a longer search locally, never in
//! CI. Every input that finds a difference becomes a fixed case.
//!
//! Each run builds N into a fixed directory under the system temp
//! directory, so the source tree gets no `target/`, and writes its inputs to
//! fixed directories there too. `nova build`, `nova test` and each side on
//! each batch are killed if they run past [`CHILD_LIMIT`].
//!
//! The target says `test = false`: `cargo test --workspace` skips this
//! file and `cargo test --workspace --test selfhost` runs it. CI runs it in
//! a job of its own on each system, N linked against a release runtime,
//! and N's own tests once more on the debug runtime (ADR 0055 §15).

#[path = "selfhost/compare.rs"]
mod compare;
#[path = "selfhost/fuzz.rs"]
mod fuzz;

use std::env::consts::EXE_SUFFIX;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::sync::OnceLock;
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use compare::{first_difference, report, settle_paths, Dump, AST, TOKENS};
use fuzz::SplitMix64;

/// How many files one process lexes.
const BATCH: usize = 50;

/// The fixed seeds and counts every run checks: 6,000 inputs of soup and
/// 750 mutated corpus files through the token dump, and 2,000 of the soup's
/// ([`AST_SOUP_SEED`]'s) and all 750 mutated files through the AST dump
/// too. Measured on a 4-core Linux machine, debug build: the whole file ran
/// in about 10 s with the token dump alone, building N included, and runs
/// in about 21 s (about 50 s of CPU) with the AST dump; all the soup through
/// both would add about 1 s of CPU.
const SOUP_SEEDS: [u64; 3] = [1, 2, 3];
const SOUP_COUNT: usize = 2000;
/// The one soup seed whose inputs go through the AST dump as well: soup is
/// made for the lexer, and most of it is one parse error after another, so
/// a third of it is kept for the parser, and its time for the mutations.
const AST_SOUP_SEED: u64 = 1;
const MUTATION_SEEDS: [u64; 3] = [101, 102, 103];
const MUTATION_COUNT: usize = 250;

/// The deep inputs' seeds, and how many of each seed's are mixed: with
/// every other kind at each of five depths, 90 inputs a seed.
const DEEP_SEEDS: [u64; 2] = [201, 202];
const DEEP_MIXED: usize = 20;

/// The beginning of each of the seven lex errors' messages
/// (nova-lexer's `error.rs`).
const LEX_ERRORS: [&str; 7] = [
    "unterminated string literal",
    "unterminated string interpolation",
    "unterminated block comment",
    "invalid escape sequence",
    "invalid unicode escape sequence",
    "invalid numeric literal",
    "unexpected character",
];

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
    // A copy, closed as it is, may still be open for writing in a child
    // that another test's thread forked meanwhile and has not yet exec'd:
    // running it then fails with ETXTBSY (26) until that child execs.
    let mut tries = 0;
    let extra = loop {
        match Command::new(&renamed)
            .args(["parse", "a.nova", "b.nova"])
            .output()
        {
            Err(e) if cfg!(unix) && e.raw_os_error() == Some(26) && tries < 50 => {
                tries += 1;
                std::thread::sleep(Duration::from_millis(100));
            }
            out => break out.expect("run nova under another name"),
        }
    };
    let stderr = String::from_utf8_lossy(&extra.stderr);
    let usage = format!("\nUsage: nova-renamed{EXE_SUFFIX} parse [OPTIONS] <FILE>\n");
    assert!(stderr.contains(&usage), "{stderr}");
    // An empty further file is refused in clap's words as well, naming the
    // first file after FILE, never the hidden list.
    for (args, named) in [
        (&["parse", "a.nova", ""][..], ""),
        (&["parse", "a.nova", "b.nova", ""], "b.nova"),
        (&["parse", "--ast", "a.nova", ""], ""),
    ] {
        let extra = Command::new(nova())
            .args(args)
            .output()
            .expect("run nova parse");
        let stderr = String::from_utf8_lossy(&extra.stderr);
        assert_eq!(extra.status.code(), Some(2), "{args:?}: {stderr}");
        assert_eq!(
            stderr,
            format!(
                "error: unexpected argument '{named}' found\n\n\
                 Usage: nova{EXE_SUFFIX} parse [OPTIONS] <FILE>\n\n\
                 For more information, try '--help'.\n"
            ),
            "{args:?}"
        );
    }
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

/// N, built once per test binary by `nova build -o` into a fixed temporary
/// directory, from `selfhost/` in place: `-o` keeps the build out of the
/// source tree.
fn selfhost() -> &'static Path {
    static BUILT: OnceLock<Result<PathBuf, String>> = OnceLock::new();
    once(&BUILT, build_selfhost)
}

/// What `cell` holds, made by `make` the first time any test asks. A
/// failure is kept too, so it is made once and every test that asks
/// reports it, rather than each running the failed build again.
fn once(
    cell: &OnceLock<Result<PathBuf, String>>,
    make: impl FnOnce() -> Result<PathBuf, String>,
) -> &Path {
    match cell.get_or_init(make) {
        Ok(path) => path,
        Err(e) => panic!("{e}"),
    }
}

fn build_selfhost() -> Result<PathBuf, String> {
    let out = fresh_dir("build");
    let home = fresh_dir("home");
    let exe = out.join(format!("selfhost{EXE_SUFFIX}"));
    let built = output_within(
        Command::new(nova())
            .current_dir(root().join("selfhost"))
            .env("NOVA_HOME", &home)
            .arg("build")
            .arg("-o")
            .arg(&exe)
            .stdin(Stdio::null()),
        CHILD_LIMIT,
        "nova build of selfhost/",
    )?;
    if !built.status.success() {
        return Err(format!(
            "nova build of selfhost/ failed:\n{}{}",
            String::from_utf8_lossy(&built.stdout),
            String::from_utf8_lossy(&built.stderr)
        ));
    }
    Ok(exe)
}

/// Every `.nova` file under `dir`, as paths relative to `root` with `/`
/// between their parts, skipping directories named `target` or
/// `node_modules`, directories whose names begin with `.`, symbolic links,
/// and the directories in `skip`.
fn nova_files(root: &Path, dir: &Path, skip: &[PathBuf], out: &mut Vec<String>) {
    let mut entries: Vec<_> = std::fs::read_dir(dir)
        .expect("read a directory")
        .map(|e| e.expect("read a directory entry"))
        .collect();
    entries.sort_by_key(|e| e.file_name());
    for e in entries {
        let name = e.file_name().to_string_lossy().into_owned();
        let kind = e.file_type().expect("a file type");
        let path = e.path();
        if kind.is_dir() {
            let skipped = name == "target"
                || name == "node_modules"
                || name.starts_with('.')
                || skip.contains(&path);
            if !skipped {
                nova_files(root, &path, skip, out);
            }
        } else if kind.is_file() && name.ends_with(".nova") {
            let rel = path.strip_prefix(root).expect("under the root");
            let parts: Vec<String> = rel
                .components()
                .map(|c| c.as_os_str().to_string_lossy().into_owned())
                .collect();
            out.push(parts.join("/"));
        }
    }
}

/// The corpus: every `.nova` file of the repository outside `selfhost/` and
/// the fixed cases (which their own test runs), sorted, then N's own
/// sources.
fn corpus() -> Vec<String> {
    let root = root();
    let mut files = Vec::new();
    let skip = [root.join("selfhost"), cases_dir()];
    nova_files(&root, &root, &skip, &mut files);
    let mut own = Vec::new();
    for sub in ["src", "tests"] {
        let dir = root.join("selfhost").join(sub);
        if dir.is_dir() {
            nova_files(&root, &dir, &[], &mut own);
        }
    }
    files.extend(own);
    files
}

fn cases_dir() -> PathBuf {
    root()
        .join("crates")
        .join("nova-cli")
        .join("tests")
        .join("selfhost")
        .join("cases")
}

/// One side's run over one batch: its output, as bytes, and exit code.
struct Run {
    stdout: Vec<u8>,
    code: Option<i32>,
    stderr: String,
}

/// A batch, named for a report: the command, and how many files from which.
fn batch_name(program: &Path, args: &[&str], files: &[String]) -> String {
    format!(
        "{} {} on the batch of {} files from {}",
        program.display(),
        args.join(" "),
        files.len(),
        files.first().map_or("(none)", String::as_str)
    )
}

/// `program` with `args` on `files`, within [`CHILD_LIMIT`]. A resolution
/// dump's stdout, S0's or N's, is settled here ([`settled_stdout`]), so
/// every comparison of the two sides that runs them through `run` takes
/// it settled.
fn run(program: &Path, args: &[&str], dir: &Path, files: &[String]) -> Result<Run, String> {
    let out = output_within(
        Command::new(program)
            .current_dir(dir)
            .args(args)
            .args(files)
            .stdin(Stdio::null()),
        CHILD_LIMIT,
        &batch_name(program, args, files),
    )?;
    Ok(Run {
        stdout: settled_stdout(args, out.stdout, cfg!(windows)),
        code: out.status.code(),
        stderr: String::from_utf8_lossy(&out.stderr).into_owned(),
    })
}

/// `stdout` of a run with `args` as S0's and N's are compared: a
/// resolution dump's, S0's `check --dump-resolution` or N's `resolve`, or
/// a HIR dump's, S0's `check --dump-hir` or N's `hir` (5.3a),
/// with `\` read as `/` in the paths of E0004's and E0093's messages on
/// `windows` ([`settle_paths`], ADR 0054 §7); any other as it is.
fn settled_stdout(args: &[&str], stdout: Vec<u8>, windows: bool) -> Vec<u8> {
    // The HIR dump prints the loader's diagnostics too (5.3a, spec 5.3
    // §3.6): S0's `check --dump-hir` and N's `hir` are settled alike.
    if args.starts_with(RESOLUTION_S0)
        || args.starts_with(HIR_S0)
        || args.starts_with(HIR_ITEMS_S0)
        || matches!(args.first(), Some(&"resolve" | &"hir"))
    {
        settle_paths(&stdout, windows)
    } else {
        stdout
    }
}

/// One batch, `files` relative to `dir`, through `nova` and `selfhost` with
/// `dump`'s arguments: S0's output, or the report of where the two part,
/// in their dumps or in their exit codes, both of which must be `code`. The
/// report gives both exit codes and the start of anything either side
/// wrote to stderr, so a side that crashed or printed nothing says so; a
/// side that runs past the limit is killed and reported with the batch.
/// A resolution dump's paths in E0004's and E0093's messages are compared
/// with `\` read as `/` on Windows, as [`run`] settles them.
fn compare_batch(
    dump: &Dump,
    dir: &Path,
    files: &[String],
    code: i32,
    nova: &Path,
    selfhost: &Path,
) -> Result<String, String> {
    // The two sides run at once.
    let (s0, n) = std::thread::scope(|scope| {
        let s0 = scope.spawn(|| run(nova, dump.s0, dir, files));
        let n = run(selfhost, dump.n, dir, files);
        (s0.join().expect("S0's thread"), n)
    });
    let (s0, n) = (s0?, n?);
    let mismatch = first_difference(&s0.stdout, &n.stdout);
    if mismatch.is_none() && s0.code == Some(code) && n.code == Some(code) {
        return Ok(String::from_utf8_lossy(&s0.stdout).into_owned());
    }
    let mut text = match &mismatch {
        Some(m) => report(m, dump, dir, nova, selfhost),
        None => format!("the dumps agree on {files:?}, but not the exit codes"),
    };
    let shown =
        |c: Option<i32>| c.map_or("none (killed by a signal)".to_string(), |c| c.to_string());
    text.push_str(&format!(
        "\nexit codes, each must be {code}: S0 {}, N {}",
        shown(s0.code),
        shown(n.code)
    ));
    for (side, stderr) in [("S0", &s0.stderr), ("N", &n.stderr)] {
        if !stderr.trim().is_empty() {
            let start: Vec<&str> = stderr.lines().take(40).collect();
            text.push_str(&format!("\n{side}'s stderr:\n{}", start.join("\n")));
        }
    }
    Err(text)
}

/// Run `files`, relative to `dir`, through S0's and N's `dump`, `batch`
/// files to a process, and fail with the report of the first difference.
/// Both sides must exit with `code`. Returns S0's output.
fn agree_on(dump: &Dump, dir: &Path, files: &[String], batch: usize, code: i32) -> String {
    let (nova, selfhost) = (nova(), selfhost());
    let mut all = String::new();
    for chunk in files.chunks(batch) {
        match compare_batch(dump, dir, chunk, code, &nova, selfhost) {
            Ok(out) => all.push_str(&out),
            Err(report) => panic!("{report}"),
        }
    }
    all
}

/// [`agree_on`] the token dump, then the AST dump (ADR 0052): every input
/// set goes through both. Returns S0's token dump, then its AST dump.
fn agree(dir: &Path, files: &[String], batch: usize, code: i32) -> String {
    let tokens = agree_on(&TOKENS, dir, files, batch, code);
    let ast = agree_on(&AST, dir, files, batch, code);
    tokens + &ast
}

/// Which dumps an input set goes through.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Dumps {
    /// 5.1a's token dump.
    Tokens,
    /// The token dump, then 5.1b's AST dump.
    TokensAndAst,
    /// 5.2a's resolution dump, each input a program's root beside
    /// [`fuzz::NAME_SIBLINGS`].
    Resolution,
}

/// Write `inputs` to a fixed directory named after `name`, as
/// `f00000.nova` and on, and check that both sides agree on them through
/// `dumps`. Returns S0's output: its token dump and, with the AST dump, its
/// AST dump after it; or its resolution dump. The fuzzer's generators all
/// come through here, 5.1b's deep nesting and 5.2a's name fuzzer included.
fn agree_on_inputs(name: &str, inputs: &[String], dumps: Dumps) -> String {
    if dumps == Dumps::Resolution {
        let (dir, files) = write_name_inputs(name, inputs);
        return agree_on_resolution(&dir, &files, 0);
    }
    let dir = fresh_dir(name);
    let mut files = Vec::with_capacity(inputs.len());
    for (i, input) in inputs.iter().enumerate() {
        let file = format!("f{i:05}.nova");
        std::fs::write(dir.join(&file), input).expect("write a fuzz input");
        files.push(file);
    }
    match dumps {
        Dumps::TokensAndAst => agree(&dir, &files, 100, 0),
        _ => agree_on(&TOKENS, &dir, &files, 100, 0),
    }
}

/// `count` mutated files from `seed`, each one of `files`, under `root`,
/// that the seed picks. The files are picked from their sorted list and
/// read with each `\r\n` as `\n`, so a seed gives the same inputs on every
/// system, a Windows checkout's line endings included, for the same files;
/// an untracked local `.nova` file is one more to pick, and changes them.
fn mutations_of(root: &Path, files: &[String], seed: u64, count: usize) -> Vec<String> {
    let mut files = files.to_vec();
    files.sort();
    let mut rng = SplitMix64::new(seed);
    (0..count)
        .map(|_| {
            let file = rng.pick(&files);
            let source = std::fs::read_to_string(root.join(file))
                .expect("read a corpus file")
                .replace("\r\n", "\n");
            fuzz::mutate(&mut rng, &source)
        })
        .collect()
}

/// The mutated corpus: [`mutations_of`] the corpus.
fn mutations(seed: u64, count: usize) -> Vec<String> {
    mutations_of(&root(), &corpus(), seed, count)
}

/// The corpus as a Windows checkout has it, written to a fixed directory
/// named after `name`: git's `core.autocrlf`, which GitHub's Windows
/// runners set, writes each `\n` of a text file as `\r\n`, and leaves a
/// file that already holds a `\r` as it is. Returns the directory and the
/// files, relative to it.
fn windows_corpus(name: &str) -> (PathBuf, Vec<String>) {
    let root = root();
    let dir = fresh_dir(name);
    let files = corpus();
    for file in &files {
        let text = std::fs::read(root.join(file)).expect("read a corpus file");
        let path = dir.join(file);
        std::fs::create_dir_all(path.parent().expect("a parent")).expect("create a directory");
        std::fs::write(&path, crlf(text)).expect("write a corpus file");
    }
    (dir, files)
}

/// `text` as git's `core.autocrlf` checks it out ([`windows_corpus`]):
/// each `\n` as `\r\n`, unless it already holds a `\r`.
fn crlf(text: Vec<u8>) -> Vec<u8> {
    if text.contains(&b'\r') {
        return text;
    }
    let mut out = Vec::with_capacity(text.len() + text.len() / 16);
    for &b in &text {
        if b == b'\n' {
            out.push(b'\r');
        }
        out.push(b);
    }
    out
}

/// The corpus's and the fixed cases' dumps, each compared once a run
/// ([`agree`], from the repository): their files and S0's AST dump, which
/// is N's. `the_corpus_dumps_agree`, `the_fixed_cases_agree` and
/// `every_kind_appears_in_the_dumps_of_the_corpus_and_the_cases` read the
/// same dumps, so whichever test asks first compares them and the others
/// wait for it (5.2b, ADR 0054 §9).
struct Compared {
    files: Vec<String>,
    ast: String,
}

fn compared(files: Vec<String>) -> Compared {
    let root = root();
    agree_on(&TOKENS, &root, &files, BATCH, 0);
    let ast = agree_on(&AST, &root, &files, BATCH, 0);
    Compared { files, ast }
}

/// [`Compared`] of the corpus.
fn corpus_compared() -> &'static Compared {
    static DUMPS: OnceLock<Compared> = OnceLock::new();
    DUMPS.get_or_init(|| compared(corpus()))
}

/// [`Compared`] of the fixed cases: every `.nova` file under [`cases_dir`].
fn cases_compared() -> &'static Compared {
    static DUMPS: OnceLock<Compared> = OnceLock::new();
    DUMPS.get_or_init(|| {
        let mut files = Vec::new();
        nova_files(&root(), &cases_dir(), &[], &mut files);
        compared(files)
    })
}

#[test]
fn the_corpus_dumps_agree() {
    let compared = corpus_compared();
    let files = &compared.files;
    // 281 under std/, tests/ and examples/ alone at 5.0c.
    assert!(files.len() >= 290, "only {} corpus files", files.len());
    assert!(files.iter().any(|f| f == "selfhost/src/lexer.nova"));
    // Each file's tree, so the AST dumps were compared too.
    let trees = compared
        .ast
        .lines()
        .filter(|l| l.starts_with("(File"))
        .count();
    assert_eq!(trees, files.len());
}

/// Both lexers must agree on the corpus as a Windows checkout has it
/// ([`windows_corpus`]) on every system, not on Windows alone: on each
/// file the checkout changes. On a Windows checkout, which already has
/// each `\r\n`, the copy changes no file, and `the_corpus_dumps_agree`
/// has compared those bytes (5.2b, ADR 0054 §9).
#[test]
fn the_corpus_dumps_agree_with_windows_line_endings() {
    let (dir, files) = windows_corpus("crlf");
    let root = root();
    let read = |path: PathBuf| std::fs::read(path).expect("read a corpus file");
    let changed: Vec<String> = files
        .into_iter()
        .filter(|f| read(root.join(f)) != read(dir.join(f)))
        .collect();
    if changed.is_empty() {
        let text = read(root.join("selfhost/src/lexer.nova"));
        assert!(text.windows(2).any(|w| w == b"\r\n"), "no file changed");
        return;
    }
    let dumps = agree(&dir, &changed, BATCH, 0);
    assert!(dumps.contains(" NL\n"), "the corpus has line breaks");
}

/// A seed's mutations are the same on every system: from the corpus as a
/// Windows checkout has it, listed in another order, they are the
/// corpus's.
#[test]
fn the_mutations_are_the_same_with_windows_line_endings() {
    let (dir, files) = windows_corpus("crlf-mutations");
    let mut reversed = files.clone();
    reversed.reverse();
    for seed in MUTATION_SEEDS {
        assert!(
            mutations_of(&dir, &reversed, seed, 50) == mutations_of(&root(), &files, seed, 50),
            "seed {seed} mutates the Windows checkout otherwise"
        );
    }
    // The name fuzzer's too, from the loose programs (5.2a).
    let loose = loose_corpus();
    let mut reversed = loose.clone();
    reversed.reverse();
    for seed in NAME_SEEDS {
        assert!(
            names_of(&dir, &reversed, seed, 50) == names_of(&root(), &loose, seed, 50),
            "seed {seed} name-mutates the Windows checkout otherwise"
        );
    }
    // And its WASM and package siblings' (5.2b).
    for seed in PACKAGE_NAME_SEEDS {
        assert!(
            package_names_of(&dir, &reversed, seed, PACKAGE_NAME_COUNT)
                == package_names_of(&root(), &loose, seed, PACKAGE_NAME_COUNT),
            "package seed {seed} name-mutates the Windows checkout otherwise"
        );
    }
    let wasm = loose_wasm_corpus();
    let mut reversed = wasm.clone();
    reversed.reverse();
    for seed in WASM_NAME_SEEDS {
        assert!(
            wasm_names_of(&dir, &reversed, seed, WASM_NAME_COUNT)
                == wasm_names_of(&root(), &wasm, seed, WASM_NAME_COUNT),
            "WASM seed {seed} name-mutates the Windows checkout otherwise"
        );
    }
}

#[test]
fn the_fixed_cases_agree() {
    let files = &cases_compared().files;
    assert!(files.len() >= 10, "only {} fixed cases", files.len());
}

#[test]
fn the_edge_inputs_agree() {
    agree_on_inputs("edges", &fuzz::edges(), Dumps::TokensAndAst);
}

#[test]
fn the_soup_agrees_and_holds_every_lex_error() {
    let mut dumps = String::new();
    for seed in SOUP_SEEDS {
        dumps.push_str(&agree_on_inputs(
            &format!("soup-{seed}"),
            &fuzz::soups(seed, SOUP_COUNT),
            if seed == AST_SOUP_SEED {
                Dumps::TokensAndAst
            } else {
                Dumps::Tokens
            },
        ));
    }
    // One seed's trees were compared too.
    assert!(dumps.contains("\n(File"), "no tree in the soup's dumps");
    for error in LEX_ERRORS {
        let found = dumps.lines().any(|line| {
            line.starts_with("ERR ")
                && line
                    .splitn(4, ' ')
                    .nth(3)
                    .is_some_and(|m| m.starts_with(error))
        });
        assert!(found, "no `{error}` in the soup of seeds {SOUP_SEEDS:?}");
    }
}

#[test]
fn the_mutated_corpus_agrees() {
    for seed in MUTATION_SEEDS {
        let out = agree_on_inputs(
            &format!("mutated-{seed}"),
            &mutations(seed, MUTATION_COUNT),
            Dumps::TokensAndAst,
        );
        // Their trees were compared too.
        assert!(out.contains("\n(File"), "seed {seed}");
    }
}

/// `NOVA_SELFHOST_FUZZ=SEED:COUNT`: COUNT inputs of soup, COUNT mutated
/// corpus files and COUNT name-mutated loose programs from SEED. Unset, as
/// in CI, it does nothing.
#[test]
fn a_longer_search_from_the_environment() {
    let Ok(spec) = std::env::var("NOVA_SELFHOST_FUZZ") else {
        return;
    };
    let (seed, count) = spec
        .split_once(':')
        .and_then(|(s, c)| Some((s.parse::<u64>().ok()?, c.parse::<usize>().ok()?)))
        .expect("NOVA_SELFHOST_FUZZ is SEED:COUNT");
    agree_on_inputs(
        "search-soup",
        &fuzz::soups(seed, count),
        Dumps::TokensAndAst,
    );
    // Mutations go in batches of 500, each from a seed of its own, so a long
    // search keeps few inputs on disk at a time; the name fuzzer's beside
    // them.
    let mut done = 0;
    while done < count {
        let n = (count - done).min(500);
        agree_on_inputs(
            "search-mutated",
            &mutations(seed.wrapping_add(done as u64), n),
            Dumps::TokensAndAst,
        );
        agree_on_inputs(
            "search-names",
            &names(seed.wrapping_add(done as u64), n),
            Dumps::Resolution,
        );
        done += n;
    }
}

#[test]
fn unreadable_files_are_reported_alike() {
    let dir = fresh_dir("unreadable");
    std::fs::write(dir.join("ok.nova"), "let x = 1\n").expect("write ok.nova");
    std::fs::write(dir.join("latin1.nova"), b"let \xe9 = 1\n").expect("write latin1.nova");
    let files = ["missing.nova", "ok.nova", "latin1.nova"].map(String::from);
    let out = agree(&dir, &files, BATCH, 2);
    assert!(
        out.starts_with("== missing.nova\nIOERR NotFound\n== ok.nova\n0 3 Let\n"),
        "{out}"
    );
    assert!(
        out.ends_with("== latin1.nova\nIOERR InvalidData\n"),
        "{out}"
    );
    // An empty path after the first is a file that is not there, on both
    // sides; an empty first one is S0's clap error (ADR 0051).
    let files = ["ok.nova", ""].map(String::from);
    let out = agree(&dir, &files, BATCH, 2);
    assert!(out.ends_with("== \nIOERR NotFound\n"), "{out}");
}

/// No file at all: both sides refuse it, print nothing on stdout and exit
/// 2, for each dump: a resolution dump needs a root (or std's part), under
/// every mode. Test mode for the WASM target is refused alike, with a root
/// or std's part (5.2b, ADR 0054): `nova test` has no target.
#[test]
fn both_sides_refuse_no_file() {
    let wasm_test = Dump {
        s0: leak_args(&[RESOLUTION_S0, &["--test", "--target", "wasm"]].concat()),
        n: leak_args(&["resolve", "--std", &std_dir(), "--test", "--target", "wasm"]),
    };
    let wasm_test_std = Dump {
        s0: leak_args(&[wasm_test.s0, &["--std"]].concat()),
        n: leak_args(&[wasm_test.n, &["--std-part"]].concat()),
    };
    // The HIR dump's (5.3a): S0's `check --dump-hir` shares the resolution
    // dump's parser, N's `hir` the resolution dump's.
    let hir_wasm_test = Dump {
        s0: leak_args(&[hir_s0(), &["--test", "--target", "wasm"]].concat()),
        n: leak_args(
            &[
                n_hir(),
                &["--std", &std_dir(), "--test", "--target", "wasm"],
            ]
            .concat(),
        ),
    };
    let hir_wasm_test_std = Dump {
        s0: leak_args(&[hir_wasm_test.s0, &["--std"]].concat()),
        n: leak_args(&[hir_wasm_test.n, &["--std-part"]].concat()),
    };
    for dump in [
        &TOKENS,
        &AST,
        resolution(),
        resolution_in(Mode::Wasm),
        resolution_in(Mode::Test),
        hir_in(Mode::Check),
        hir_in(Mode::Wasm),
        hir_in(Mode::Test),
    ] {
        let out = compare_batch(dump, &root(), &[], 2, &nova(), selfhost());
        assert_eq!(out, Ok(String::new()));
    }
    let main = ["tests/runtime/array_helpers.nova".to_string()];
    assert!(root().join(&main[0]).is_file());
    for (dump, files) in [
        (&wasm_test, &main[..]),
        (&wasm_test_std, &[]),
        (&hir_wasm_test, &main[..]),
        (&hir_wasm_test_std, &[]),
    ] {
        let out = compare_batch(dump, &root(), files, 2, &nova(), selfhost());
        assert_eq!(out, Ok(String::new()));
    }
}

/// Both hidden forms take `--test` wherever an option may stand, and agree
/// on what each shape gives, stdout and exit code (5.2b, ADR 0054 §1):
/// before or after a root, beside an empty root (not found), twice
/// (refused), and with std's part for the WASM target in either order
/// (refused, as `nova test` has no target), and with a value (refused, as
/// clap refuses one for a flag); `--target VALUE` and `--target=VALUE`, as
/// clap takes them, an empty, repeated (either way round) or unknown value
/// refused. `--std` is N's
/// `--std-part`. A `--` is left out: N reads it as a root of that name, where S0's form
/// skips it, and the harness passes none (ADR 0053 §5).
#[test]
fn both_sides_take_test_in_every_position_alike() {
    let file = "tests/runtime/array_helpers.nova";
    assert!(root().join(file).is_file());
    let std = std_dir();
    let shapes: [(&[&str], i32); 15] = [
        (&["--test", file], 0),
        (&[file, "--test"], 0),
        (&["--test", ""], 2),
        (&["--test", "--test", file], 2),
        (&["--test=true", file], 2),
        (&["--target", "", file], 2),
        (&["--target", "", "--target", "wasm", file], 2),
        (&["--target", "wasm", "--target=native", file], 2),
        (&["--target", "wasm", "--target", "native", file], 2),
        (&["--test", "--std", "--target", "wasm"], 2),
        (&["--target", "wasm", "--std", "--test"], 2),
        (&["--target=wasm", "--std"], 0),
        (&["--target=", file], 2),
        (&["--target=wasm", "--target", "wasm", "--std"], 2),
        (&["--target=bogus", file], 2),
    ];
    // Both dumps: the HIR dump's forms take what the resolution dump's
    // take (5.3a, ADR 0055).
    for ((args, code), (s0, command)) in shapes.iter().flat_map(|shape| {
        [
            (shape, (RESOLUTION_S0, &["resolve"][..])),
            (shape, (hir_s0(), n_hir())),
        ]
    }) {
        let n: Vec<&str> = command
            .iter()
            .copied()
            .chain(["--std", std.as_str()])
            .chain(
                args.iter()
                    .map(|a| if *a == "--std" { "--std-part" } else { a }),
            )
            .collect();
        let dump = Dump {
            s0: leak_args(&[s0, *args].concat()),
            n: leak_args(&n),
        };
        if let Err(report) = compare_batch(&dump, &root(), &[], *code, &nova(), selfhost()) {
            panic!("{command:?} {args:?}: {report}");
        }
    }
}

/// A root whose package's manifest N's minimal reader refuses (ADR 0054
/// §5): S0 dumps it with the package graph's M-code, an error stopping
/// it before the loader, and N, which gives no M-code, prints nothing of
/// it, says why on stderr, naming the manifest, and exits 1. No input
/// of the differential has such a manifest; this pins what N does
/// instead of printing a partial dump.
#[test]
fn a_manifest_n_cannot_read_stops_n_with_a_message() {
    let dir = fresh_dir("resolution-refused");
    for (name, extra) in [
        ("unknown", "color = \"red\"\n"),
        ("registry", "[dependencies]\njson = \"1.0\"\n"),
    ] {
        let pkg = dir.join(name);
        std::fs::create_dir_all(pkg.join("src")).expect("create src/");
        std::fs::write(pkg.join("nova.toml"), format!("{MANIFEST}{extra}"))
            .expect("write nova.toml");
        std::fs::write(pkg.join("src").join("main.nova"), "fn main() {}\n")
            .expect("write main.nova");
        let root = format!("{name}/src/main.nova");
        let s0 = Command::new(nova())
            .current_dir(&dir)
            .args(RESOLUTION_S0)
            .arg(&root)
            .output()
            .expect("run nova check --dump-resolution");
        let stdout = String::from_utf8_lossy(&s0.stdout);
        assert_eq!(s0.status.code(), Some(0), "{name}: {stdout}");
        assert!(stdout.contains("\"M00"), "{name}: {stdout}");
        // An error among the graph's M-codes stops S0 before the loader,
        // with no module; the unknown key's warning does not.
        let stopped = format!("== {root}\n(Program target=native resolved=false)\n(Diagnostics\n");
        assert_eq!(
            stdout.starts_with(&stopped),
            name == "registry",
            "{name}: {stdout}"
        );
        let n = Command::new(selfhost())
            .current_dir(&dir)
            .args(resolution().n)
            .arg(&root)
            .output()
            .expect("run selfhost resolve");
        let stderr = String::from_utf8_lossy(&n.stderr);
        assert_eq!(n.status.code(), Some(1), "{name}: {stderr}");
        assert!(n.stdout.is_empty(), "{name}");
        assert!(
            stderr.starts_with(&format!(
                "error: selfhost resolve cannot dump {root}: {name}/nova.toml: "
            )),
            "{name}: {stderr}"
        );
    }
}

/// A copy of std's sources in a fresh directory named for `name`.
fn std_copy(name: &str) -> PathBuf {
    let std = fresh_dir(name);
    let from = root().join("std");
    let mut sources = Vec::new();
    nova_files(&from, &from, &[], &mut sources);
    for rel in &sources {
        let to = std.join(rel);
        std::fs::create_dir_all(to.parent().expect("a parent")).expect("create a std directory");
        std::fs::copy(from.join(rel), &to).expect("copy a std file");
    }
    std
}

/// A trait that an imported std module exports reaches each program that
/// shares std through std's exports rebased past the program's own ids
/// (ADR 0054 §9), as a function and a type do; std's `ui`, which a root
/// may import, exports none, so a copy of std whose `ui` ends with a
/// `pub trait` stands in: two loose roots of one and of three definitions
/// import `ui`, and N's dump of each lists the trait in its scope with
/// the id N's std part gives it, and is otherwise S0's, whose std is
/// built in and lacks it (the trait is std's last definition, so no
/// other id moves).
#[test]
fn a_trait_a_shared_std_module_exports_keeps_its_id_in_each_program() {
    let std = std_copy("trait-std");
    let ui = std.join("ui").join("lib.nova");
    let text = std::fs::read_to_string(&ui).expect("read ui");
    std::fs::write(&ui, format!("{text}\npub trait Shade {{}}\n")).expect("alter ui");
    let std = std.display().to_string();
    let dir = fresh_dir("trait-roots");
    let roots = ["a.nova".to_string(), "b.nova".to_string()];
    std::fs::write(dir.join(&roots[0]), "import ui\n\nfn main() {}\n").expect("write a.nova");
    std::fs::write(
        dir.join(&roots[1]),
        "import ui\n\nrecord R { n: Int }\nrecord Q { n: Int }\n\nfn main() {}\n",
    )
    .expect("write b.nova");
    let part = run(
        selfhost(),
        &["resolve", "--std", &std, "--std-part"],
        &dir,
        &[],
    )
    .expect("run N's std part");
    let part = String::from_utf8_lossy(&part.stdout);
    let id = part
        .split("(Def.Trait ")
        .find_map(|d| {
            d.split_once(' ')
                .filter(|(_, rest)| rest.starts_with("\"Shade\" "))
                .map(|(id, _)| id.to_string())
        })
        .unwrap_or_else(|| panic!("no Shade in std's part: {part}"));
    let line = format!("    (Trait \"Shade\" {id})\n");
    let s0 = run(&nova(), RESOLUTION_S0, &dir, &roots).expect("run S0");
    let n = run(selfhost(), &["resolve", "--std", &std], &dir, &roots).expect("run N");
    assert_eq!((s0.code, n.code), (Some(0), Some(0)), "{}", n.stderr);
    let n = String::from_utf8_lossy(&n.stdout);
    for part in parts_of(&n) {
        assert_eq!(part.matches(&line).count(), 1, "{line}{part}");
    }
    assert_eq!(n.replace(&line, ""), String::from_utf8_lossy(&s0.stdout));
}

/// N shares std resolved once only when std alone gives no diagnostic and
/// holds no `@test` function, either of which the shared path would drop
/// (ADR 0054 §9): with a std whose `core` defines a name twice, or whose
/// `ui` marks a function `@test` (under test mode), N resolves std again
/// for each program, so each program that reaches the resolver keeps
/// std's E0002, or lists std's test, and its dump is otherwise S0's, whose
/// std is built in (nova-resolver's own test pins S0's side); the rest
/// are S0's. The roots are the first six loose programs and the first four
/// package roots of the mode's cases.
#[test]
fn n_resolves_std_for_each_program_when_std_gives_a_diagnostic_or_a_test() {
    let alterations = [
        (
            Mode::Check,
            "core/lib.nova",
            "fn mix64(x: Int) -> Int {",
            "fn escape_common(x: Int) -> Int {",
            "(Error \"E0002\" \"duplicate definition of `escape_common`\"",
        ),
        (
            Mode::Test,
            "ui/lib.nova",
            "\nfn flush() {",
            "\n@test\nfn flush() {",
            " \"flush\" should_panic=false)",
        ),
    ];
    std::thread::scope(|scope| {
        let runs: Vec<_> = alterations
            .into_iter()
            .enumerate()
            .map(|(k, (mode, file, old, new, mark))| {
                scope.spawn(move || {
                    // A copy of std's sources, `file` altered.
                    let std = std_copy(&format!("altered-std-{k}"));
                    let text = std::fs::read_to_string(std.join(file))
                        .expect("read the std file")
                        .replace("\r\n", "\n");
                    assert_eq!(text.matches(old).count(), 1, "{file}: {old}");
                    std::fs::write(std.join(file), text.replacen(old, new, 1))
                        .expect("alter the std file");
                    let cases = mode_cases_dir(mode);
                    let found = case_roots(&cases);
                    let roots: Vec<String> = found.loose[..6]
                        .iter()
                        .chain(&found.packages[..4])
                        .cloned()
                        .collect();
                    let std = std.display().to_string();
                    let n_args: Vec<&str> = ["resolve", "--std", std.as_str()]
                        .into_iter()
                        .chain(mode.flags().iter().copied())
                        .collect();
                    let s0 = run(&nova(), resolution_in(mode).s0, &cases, &roots).expect("run S0");
                    let n = run(selfhost(), &n_args, &cases, &roots).expect("run N");
                    assert_eq!((s0.code, n.code), (Some(0), Some(0)), "{}", n.stderr);
                    let (s0, n) = (
                        String::from_utf8_lossy(&s0.stdout).into_owned(),
                        String::from_utf8_lossy(&n.stdout).into_owned(),
                    );
                    let (s0_parts, n_parts) = (parts_of(&s0), parts_of(&n));
                    assert_eq!(s0_parts.len(), roots.len(), "{s0}");
                    assert_eq!(n_parts.len(), roots.len(), "{n}");
                    let mut resolved = 0;
                    for (s0_part, n_part) in s0_parts.iter().zip(&n_parts) {
                        if !s0_part.contains(" resolved=true\n") {
                            assert_eq!(n_part, s0_part);
                            continue;
                        }
                        resolved += 1;
                        assert_eq!(n_part.matches(mark).count(), 1, "{file}: {n_part}");
                        assert_eq!(
                            without_parens(&without_block(n_part, mark)),
                            without_parens(s0_part),
                            "{file}: {n_part}"
                        );
                    }
                    assert!(resolved >= 6, "{resolved} of {} resolved", roots.len());
                })
            })
            .collect();
        joined(runs);
    });
}

/// `part` without the line holding `mark` and the lines below it indented
/// further, a diagnostic and its labels or a test's line.
fn without_block(part: &str, mark: &str) -> String {
    let indent = |l: &str| l.len() - l.trim_start().len();
    let mut out = String::new();
    let mut inside: Option<usize> = None;
    for line in part.split_inclusive('\n') {
        if let Some(depth) = inside {
            if indent(line) > depth {
                continue;
            }
            inside = None;
        }
        if line.contains(mark) {
            inside = Some(indent(line));
            continue;
        }
        out.push_str(line);
    }
    out
}

/// `text` with the `)`s that end each line taken off, so that a part
/// with a line taken out compares with one that never had it.
fn without_parens(text: &str) -> String {
    text.lines()
        .map(|l| l.trim_end_matches(')'))
        .collect::<Vec<_>>()
        .join("\n")
}

/// A package's identity (ADR 0054 §5), in a directory named `run`: `app`
/// depends on `../geom` and `../b`, and `b` on `geom` by its absolute
/// path; `app3` on `geom` by its absolute path and on `../b3`, and `b3` on
/// `../geom`, so the absolute path comes first; `up` on `../geom` and
/// `../climb`, and `climb` on `../../run/geom`, which climbs above the
/// current directory and comes back down; `up2` on `../../run/geom` and
/// on `../b3`, so the climbing path comes first; `past` on `../geom` and
/// `../b4`, and `b4` on `geom` by thirty `../` and its absolute path
/// without the leading `/`, which climbs past `/`, where a `..` stays at
/// `/`. Under a root given by its
/// absolute path both sides read one `geom` in each. N cannot name the
/// current directory, so under a relative root it refuses each graph as
/// M0011, saying the two paths may name one directory, where S0, which
/// reads the real path, resolves it; and through a link to `geom` N
/// refuses the graph as M0011 under any root, where S0 resolves it. No
/// other input of the differential holds an absolute dependency path, a
/// path climbing above the current directory or a link; on Windows N's
/// paths read no drive, which 5.5 takes up with a root written with `\`,
/// so the test is Unix's.
#[cfg(unix)]
#[test]
fn a_directory_spelled_two_ways_is_one_package_under_an_absolute_root() {
    let dir = fresh_dir("resolution-identity").join("run");
    let geom = dir.join("geom").display().to_string();
    let past = format!("{}{}", "../".repeat(30), geom.trim_start_matches('/'));
    let manifest = |name: &str, deps: &[(&str, &str)]| {
        let mut text = MANIFEST.replace("pkg", name);
        if !deps.is_empty() {
            text.push_str("[dependencies]\n");
        }
        for (dep, path) in deps {
            text.push_str(&format!("{dep} = {{ path = \"{path}\" }}\n"));
        }
        text
    };
    let main = |deps: &[&str]| {
        let imports: String = deps.iter().map(|d| format!("import {d}\n")).collect();
        format!("{imports}\nfn main() {{}}\n")
    };
    let lib = |name: &str| format!("import geom\n\npub fn {name}() -> Int {{ area() }}\n");
    let files = [
        (
            "app/nova.toml",
            manifest("app", &[("geom", "../geom"), ("b", "../b")]),
        ),
        ("app/src/main.nova", main(&["geom", "b"])),
        (
            "app3/nova.toml",
            manifest("app3", &[("geom", &geom), ("b3", "../b3")]),
        ),
        ("app3/src/main.nova", main(&["geom", "b3"])),
        (
            "up/nova.toml",
            manifest("up", &[("geom", "../geom"), ("climb", "../climb")]),
        ),
        ("up/src/main.nova", main(&["geom", "climb"])),
        (
            "up2/nova.toml",
            manifest("up2", &[("geom", "../../run/geom"), ("b3", "../b3")]),
        ),
        ("up2/src/main.nova", main(&["geom", "b3"])),
        (
            "past/nova.toml",
            manifest("past", &[("geom", "../geom"), ("b4", "../b4")]),
        ),
        ("past/src/main.nova", main(&["geom", "b4"])),
        ("geom/nova.toml", manifest("geom", &[])),
        (
            "geom/src/lib.nova",
            "pub fn area() -> Int { 1 }\n".to_string(),
        ),
        ("b/nova.toml", manifest("b", &[("geom", &geom)])),
        ("b/src/lib.nova", lib("twice")),
        ("b3/nova.toml", manifest("b3", &[("geom", "../geom")])),
        ("b3/src/lib.nova", lib("thrice")),
        ("b4/nova.toml", manifest("b4", &[("geom", &past)])),
        ("b4/src/lib.nova", lib("past")),
        (
            "climb/nova.toml",
            manifest("climb", &[("geom", "../../run/geom")]),
        ),
        ("climb/src/lib.nova", lib("climb")),
        (
            "linked/nova.toml",
            manifest("linked", &[("geom", "../geom"), ("c", "../c")]),
        ),
        ("linked/src/main.nova", main(&["geom", "c"])),
        ("c/nova.toml", manifest("c", &[("geom", "../link")])),
        ("c/src/lib.nova", lib("c")),
    ];
    for (path, text) in &files {
        let path = dir.join(path);
        std::fs::create_dir_all(path.parent().expect("a parent")).expect("create a directory");
        std::fs::write(path, text).expect("write a package's file");
    }
    std::os::unix::fs::symlink("geom", dir.join("link")).expect("link geom");
    let absolute = |root: &str| dir.join(root).display().to_string();
    let roots: Vec<String> = ["app", "app3", "up", "up2", "past"]
        .iter()
        .map(|p| absolute(&format!("{p}/src/main.nova")))
        .collect();
    let out = agree_on_resolution(&dir, &roots, 0);
    assert_eq!(parts_of(&out).len(), roots.len(), "{out}");
    for (root, part) in roots.iter().zip(parts_of(&out)) {
        assert!(
            part.starts_with(&format!("{root}\n(Program target=native resolved=true\n")),
            "{part}"
        );
        assert_eq!(part.matches("(Module ").count(), 3, "{part}");
    }
    let refused = |root: &str, why: &str| {
        let s0 = run(&nova(), RESOLUTION_S0, &dir, &[root.to_string()]).expect("run S0");
        let s0_out = String::from_utf8_lossy(&s0.stdout);
        assert_eq!(s0.code, Some(0), "{root}: {s0_out}");
        assert!(s0_out.contains(" resolved=true\n"), "{root}: {s0_out}");
        assert_eq!(s0_out.matches("(Module ").count(), 3, "{root}: {s0_out}");
        let n = run(selfhost(), resolution().n, &dir, &[root.to_string()]).expect("run N");
        assert_eq!(n.code, Some(1), "{root}: {}", n.stderr);
        assert!(n.stdout.is_empty(), "{root}");
        assert_eq!(
            n.stderr,
            format!(
                "error: selfhost resolve cannot dump {root}: two packages named `geom` in one \
                 build (M0011){why}\n"
            )
        );
    };
    let or_one = ", or one directory spelled by two paths, which N cannot tell apart without \
                  the current directory";
    for root in ["app", "app3", "up", "up2", "past"] {
        refused(&format!("{root}/src/main.nova"), or_one);
    }
    refused(&absolute("linked/src/main.nova"), "");
}

/// On Windows a resolution dump's E0004 and E0093 messages are compared
/// with `\` read as `/` ([`settle_paths`], ADR 0054 §7, and decision 4 of
/// its §8), and nothing else is; elsewhere nothing is changed.
#[test]
fn the_paths_of_e0004_and_e0093_are_settled_on_windows_alone() {
    let dump =
        "(Diagnostics\n  (Error \"E0004\" \"`a` is both: app\\x5csrc\\x5ca.nova, and the entry at \
                app\\x5cnova.toml:6\"\n    (Primary \"a\\x5cb\":0..1 \"x\"))\n  \
                (Error \"E0093\" \"rename sub\\x5cui.nova\"\n  (Error \"E0001\" \"a\\x5cb\"\n";
    let settled =
        "(Diagnostics\n  (Error \"E0004\" \"`a` is both: app/src/a.nova, and the entry at \
                   app/nova.toml:6\"\n    (Primary \"a\\x5cb\":0..1 \"x\"))\n  \
                   (Error \"E0093\" \"rename sub/ui.nova\"\n  (Error \"E0001\" \"a\\x5cb\"\n";
    assert_eq!(settle_paths(dump.as_bytes(), true), settled.as_bytes());
    assert_eq!(settle_paths(dump.as_bytes(), false), dump.as_bytes());
}

/// The fault CI's Windows leg found in 5.2b (ADR 0054 §7): a test
/// compared S0's and N's resolution dumps of E0004 without settling its
/// paths, which S0 writes with `\` on Windows. [`run`] settles every
/// resolution dump, S0's and N's, so no comparison built on it can miss
/// that: here S0's dump of a dependency's E0004, each `/` of its message
/// written as Windows writes it, differs from N's as it is and equals it
/// settled as `run` settles it on Windows, under S0's arguments and N's;
/// a dump that is not a resolution dump is kept as it is.
#[test]
fn every_run_of_a_resolution_dump_settles_e0004s_paths_on_windows() {
    let cases = mode_cases_dir(Mode::Check);
    let root = ["a_dependencys_e0004_names_its_own_manifest/src/main.nova".to_string()];
    let s0 = run(&nova(), RESOLUTION_S0, &cases, &root).expect("run S0");
    let n = run(selfhost(), resolution().n, &cases, &root).expect("run N");
    assert_eq!((s0.code, n.code), (Some(0), Some(0)), "{}", n.stderr);
    assert_eq!(s0.stdout, n.stdout);
    let text = String::from_utf8_lossy(&s0.stdout);
    let windows: String = text
        .split_inclusive('\n')
        .map(|line| {
            if line.starts_with("  (Error \"E0004\" ") {
                line.replace('/', "\\x5c")
            } else {
                line.to_string()
            }
        })
        .collect();
    assert!(windows.contains("geo\\x5cnova.toml:7"), "{windows}");
    assert_ne!(windows.as_bytes(), &n.stdout[..]);
    for args in [RESOLUTION_S0, resolution().n] {
        let settled = settled_stdout(args, windows.clone().into_bytes(), true);
        assert_eq!(settled, n.stdout, "{args:?}");
    }
    let kept = settled_stdout(TOKENS.s0, windows.clone().into_bytes(), true);
    assert_eq!(kept, windows.as_bytes());
}

#[test]
fn the_mismatch_report_names_the_file_and_line() {
    let dir = fresh_dir("mismatch");
    std::fs::write(dir.join("a.nova"), "let a = 1\n").expect("write a.nova");
    std::fs::write(dir.join("b.nova"), "fn f() {}\n\nlet y = 2.5\n").expect("write b.nova");
    let files = ["a.nova", "b.nova"].map(String::from);
    let s0 = run(&nova(), TOKENS.s0, &dir, &files)
        .expect("run S0")
        .stdout;
    let s0 = String::from_utf8(s0).expect("a dump of UTF-8 files is UTF-8");
    let wrong = s0.replace("Float 4004000000000000", "Float 4004000000000001");
    assert_ne!(s0, wrong, "the dump holds 2.5's bits:\n{s0}");
    let m = first_difference(s0.as_bytes(), wrong.as_bytes()).expect("a difference");
    assert_eq!(m.file.as_deref(), Some("b.nova"));
    assert_eq!(m.dump_line, 10);
    let text = report(&m, &TOKENS, &dir, &nova(), Path::new("selfhost"));
    for part in [
        "in b.nova, at line 10 of its dump, source line 3",
        "S0 (nova parse --dump-tokens): 19 22 Float 4004000000000000",
        "N  (selfhost tokens):          19 22 Float 4004000000000001",
        "parse --dump-tokens b.nova > s0.txt",
        "selfhost tokens b.nova > n.txt",
    ] {
        assert!(text.contains(part), "{part:?} is not in:\n{text}");
    }
    // A side that stops before a file names that file.
    let short = &s0[..s0.rfind("== b.nova").expect("b's header")];
    let m = first_difference(s0.as_bytes(), short.as_bytes()).expect("a difference");
    assert_eq!((m.file.as_deref(), m.dump_line), (Some("b.nova"), 0));
    assert_eq!(m.n, None);
    let text = report(&m, &TOKENS, &dir, &nova(), Path::new("selfhost"));
    assert!(
        text.contains("in b.nova, at its header: one side's output ended before it"),
        "{text}"
    );
    // Bytes that are not UTF-8 differ as bytes, though each reads as the
    // same replacement character.
    let m = first_difference(
        b"== a.nova\n0 1 Ident \"\xff\"\n",
        b"== a.nova\n0 1 Ident \"\xfe\"\n",
    )
    .expect("a difference");
    assert_eq!((m.file.as_deref(), m.dump_line), (Some("a.nova"), 1));
    // An AST dump's line names its source line by its node's start or its
    // error's.
    std::fs::write(dir.join("c.nova"), "fn f() {\n    1\n}\n").expect("write c.nova");
    let files = ["b.nova", "c.nova"].map(String::from);
    let s0 = run(&nova(), AST.s0, &dir, &files).expect("run S0").stdout;
    let s0 = String::from_utf8(s0).expect("a dump of UTF-8 files is UTF-8");
    for (from, to, part) in [
        (
            "found `let`",
            "found `lot`",
            "in b.nova, at line 5 of its dump, source line 3",
        ),
        (
            "Int:1)",
            "Int:2)",
            "in c.nova, at line 5 of its dump, source line 2",
        ),
    ] {
        let wrong = s0.replace(from, to);
        assert_ne!(s0, wrong, "{from} is not in:\n{s0}");
        let m = first_difference(s0.as_bytes(), wrong.as_bytes()).expect("a difference");
        let text = report(&m, &AST, &dir, &nova(), Path::new("selfhost"));
        assert!(text.contains(part), "{part:?} is not in:\n{text}");
        assert!(text.contains("selfhost ast "), "{text}");
    }
}

/// A side that fails, here a stand-in for N that is `nova` given an
/// argument it refuses, so that it prints nothing: the report names the
/// file it never reached, both exit codes and its stderr. Two sides whose
/// dumps agree and whose exit codes do not are reported too.
#[test]
fn a_side_that_fails_is_reported_with_its_exit_code_and_stderr() {
    const BROKEN: Dump = Dump {
        s0: &["parse", "--dump-tokens"],
        n: &["parse", "--no-such-flag"],
    };
    const SAME: Dump = Dump {
        s0: &["parse", "--dump-tokens"],
        n: &["parse", "--dump-tokens"],
    };
    let dir = fresh_dir("broken");
    std::fs::write(dir.join("a.nova"), "let a = 1\n").expect("write a.nova");
    let nova = nova();
    let files = ["a.nova".to_string()];
    let text = compare_batch(&BROKEN, &dir, &files, 0, &nova, &nova).expect_err("N fails");
    for part in [
        "the dumps differ in a.nova, at its header: one side's output ended before it",
        "(selfhost parse --no-such-flag): (nothing: the output ended)",
        "exit codes, each must be 0: S0 0, N 2",
        "N's stderr:\nerror: unexpected argument '--no-such-flag' found",
    ] {
        assert!(text.contains(part), "{part:?} is not in:\n{text}");
    }
    let files = ["missing.nova".to_string()];
    let text = compare_batch(&SAME, &dir, &files, 0, &nova, &nova).expect_err("both exit 2");
    for part in [
        "the dumps agree on [\"missing.nova\"], but not the exit codes",
        "exit codes, each must be 0: S0 2, N 2",
    ] {
        assert!(text.contains(part), "{part:?} is not in:\n{text}");
    }
    assert!(compare_batch(&SAME, &dir, &files, 2, &nova, &nova).is_ok());
}

/// A process past its limit is killed, and the error names its batch: here
/// `nova lsp`, which reads its stdin, held open and empty, until it ends.
/// A limit that never fired would leave this test to fail at its own
/// deadline, a minute on.
#[test]
fn a_process_past_its_limit_is_killed_and_its_batch_named() {
    let (send, receive) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let files = ["a.nova", "b.nova"].map(String::from);
        let what = batch_name(&nova(), &["lsp"], &files);
        let mut lsp = Command::new(nova());
        lsp.arg("lsp").stdin(Stdio::piped());
        let _ = send.send(output_within(&mut lsp, Duration::from_millis(300), &what));
    });
    let out = receive
        .recv_timeout(Duration::from_secs(60))
        .expect("nova lsp was not killed at its limit");
    let error = out.expect_err("nova lsp does not end while its stdin is open");
    assert!(
        error.ends_with(
            " lsp on the batch of 2 files from a.nova did not finish within 300ms: killed"
        ),
        "{error}"
    );
}

/// A failed build of N is made once and reported by each test that asks.
#[test]
fn a_failed_build_is_made_once_and_reported_to_each_test() {
    let cell = OnceLock::new();
    let builds = std::cell::Cell::new(0);
    for _ in 0..2 {
        let failed = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            once(&cell, || {
                builds.set(builds.get() + 1);
                Err("nova build of selfhost/ failed".to_string())
            });
        }))
        .expect_err("a failed build fails the test that asks");
        assert_eq!(
            failed.downcast_ref::<String>().map(String::as_str),
            Some("nova build of selfhost/ failed")
        );
    }
    assert_eq!(builds.get(), 1, "the failed build ran again");
    // `selfhost()` asks `once`: however often a test asks, N is one.
    assert!(std::ptr::eq(selfhost(), selfhost()), "N was built again");
}

/// A pipe still open when the time is up, as when a process the child
/// started holds it after the child exits, is reported, not waited for:
/// here a reader that would finish two seconds on.
#[test]
fn a_pipe_still_open_at_the_deadline_is_reported() {
    let (stop, stopped) = std::sync::mpsc::channel::<()>();
    let held = std::thread::spawn(move || {
        let _ = stopped.recv_timeout(Duration::from_secs(2));
        b"late".to_vec()
    });
    let done = std::thread::spawn(|| b"out".to_vec());
    let start = Instant::now();
    let read = read_by([done, held], start + Duration::from_millis(100));
    assert!(read.is_none(), "a reader still open was waited for");
    assert!(start.elapsed() < Duration::from_secs(2));
    drop(stop);
    let done = std::thread::spawn(|| b"out".to_vec());
    let err = std::thread::spawn(Vec::new);
    let read = read_by([done, err], Instant::now() + Duration::from_secs(60));
    assert_eq!(read, Some([b"out".to_vec(), Vec::new()]));
}

/// Two lines that differ only in a final line break, which one side lacks,
/// read the same; the report marks the one without it.
#[test]
fn a_missing_final_line_break_is_marked() {
    let m = first_difference(b"== a.nova\n0 0 Eof\n", b"== a.nova\n0 0 Eof").expect("a difference");
    assert_eq!(m.s0.as_deref(), Some("0 0 Eof"));
    assert_eq!(m.n.as_deref(), Some("0 0 Eof (no line break)"));
    let text = report(&m, &TOKENS, Path::new("."), &nova(), Path::new("selfhost"));
    assert!(text.contains("0 0 Eof (no line break)"), "{text}");
    let m = first_difference(b"== a.nova\n1 2 Eof", b"== a.nova\n1 2 Eof\n").expect("a difference");
    assert_eq!(m.s0.as_deref(), Some("1 2 Eof (no line break)"));
    assert_eq!(m.n.as_deref(), Some("1 2 Eof"));
    // Lines that differ otherwise are shown as they are.
    let m = first_difference(b"== a.nova\n0 0 Eof\n", b"== a.nova\n0 1 Eof").expect("a difference");
    assert_eq!(m.n.as_deref(), Some("0 1 Eof"));
}

// --- 5.1b: the AST dump, the depth limit and S0's thread (ADR 0052) ---

#[test]
fn the_ast_dump_flag_is_hidden_and_dumps_each_file_after_a_header() {
    let help = Command::new(nova())
        .args(["parse", "--help"])
        .output()
        .expect("run nova parse --help");
    let help = String::from_utf8_lossy(&help.stdout);
    assert!(help.contains("--check"), "{help}");
    assert!(!help.contains("dump-ast"), "{help}");
    for other in ["--ast", "--check", "--dump-tokens"] {
        let both = Command::new(nova())
            .args(["parse", "--dump-ast", other, "a.nova"])
            .output()
            .expect("run nova parse");
        let stderr = String::from_utf8_lossy(&both.stderr);
        assert_eq!(both.status.code(), Some(2), "{other}: {stderr}");
        assert!(stderr.contains("cannot be used with"), "{other}: {stderr}");
    }
    let dir = fresh_dir("ast-headers");
    std::fs::write(dir.join("a.nova"), "fn f() {}\n").expect("write a.nova");
    std::fs::write(dir.join("b.nova"), "fn g( {").expect("write b.nova");
    let out = Command::new(nova())
        .current_dir(&dir)
        .args(["parse", "--dump-ast", "a.nova", "missing.nova", "b.nova"])
        .output()
        .expect("run nova parse --dump-ast");
    assert_eq!(
        String::from_utf8_lossy(&out.stdout),
        "== a.nova\n(File\n  (Item.Function 0..9\n    \
         (Function vis=Private is_async=false is_component=false name=\"f\"@3..4\n      \
         (Block 7..9))))\n\
         == missing.nova\nIOERR NotFound\n\
         == b.nova\n(File)\n\
         PARSE 6 7 expected identifier (in parameter name), found `{`\n\
         PARSE 6 7 expected `)` (in function parameters), found `{`\n"
    );
    assert_eq!(out.status.code(), Some(2));
}

/// What `line` of an AST dump names among [`nova_ast::dump::KINDS`]'s
/// forms: its node's kind, `(Kind`; each word outside quotes that is a
/// field `key=value`, after the kind, `(Kind key=value`, when the value
/// is a `bool`'s or `children=`'s; and each word that begins with a
/// literal's prefix, `Int:`.
fn kinds_on(line: &str, found: &mut std::collections::HashSet<String>) {
    let node = line.trim_start();
    let mut kind = String::new();
    if let Some(rest) = node.strip_prefix('(') {
        kind = rest
            .chars()
            .take_while(|c| *c != ' ' && *c != ')')
            .collect();
        found.insert(format!("({kind}"));
    }
    // Words, split at the spaces outside quotes; a quote inside one is
    // written `\x22`, so each `"` opens or closes one.
    let mut quoted = false;
    let mut word = String::new();
    for c in node.chars().chain([' ']) {
        match c {
            '"' => {
                quoted = !quoted;
                word.push(c);
            }
            ' ' if !quoted => {
                let w = word.trim_end_matches(')');
                if let Some((prefix, _)) = w.split_once(':') {
                    if ["Int", "Float", "Str", "Char", "Bool"].contains(&prefix) {
                        found.insert(format!("{prefix}:"));
                    }
                }
                match w.split_once('=') {
                    Some((_, "true" | "false" | "Some" | "None")) => {
                        found.insert(format!("({kind} {w}"));
                    }
                    Some(_) if !w.contains('"') => {
                        found.insert(w.to_string());
                    }
                    _ => {}
                }
                word.clear();
            }
            _ => word.push(c),
        }
    }
}

/// Every kind of node, field-less value and literal the dump can print
/// appears in the AST dump of some corpus file or fixed case (spec §4.3),
/// which both sides print byte for byte: `the_corpus_dumps_agree` and
/// `the_fixed_cases_agree` compare the same dumps ([`Compared`]), so each
/// side's holds every kind. 5.1 ran both sides over the files again here;
/// from 5.2b the dumps compared are read (ADR 0054 §9).
#[test]
fn every_kind_appears_in_the_dumps_of_the_corpus_and_the_cases() {
    let mut found = std::collections::HashSet::new();
    for compared in [corpus_compared(), cases_compared()] {
        for line in compared.ast.lines() {
            kinds_on(line, &mut found);
        }
    }
    let missing: Vec<&str> = nova_ast::dump::KINDS
        .iter()
        .copied()
        .filter(|k| !found.contains(*k))
        .collect();
    assert!(missing.is_empty(), "no file's dump holds {missing:?}");
}

#[test]
fn the_deep_inputs_agree() {
    for seed in DEEP_SEEDS {
        let inputs: Vec<String> = fuzz::deeps(seed, DEEP_MIXED)
            .into_iter()
            .map(|(_, _, src)| src)
            .collect();
        let out = agree_on_inputs(&format!("deep-{seed}"), &inputs, Dumps::TokensAndAst);
        // The AST dumps were compared too: past the limit, P0002.
        assert!(out.contains(" this is nested too deeply"), "seed {seed}");
    }
}

/// N, on a main thread of `stack` KB where `ulimit -s` can set it (Unix),
/// or of the platform's size (Windows: 1 MB): `selfhost ast file` in `dir`,
/// its exit code and stdout.
fn n_on_stack(dir: &Path, stack: Option<u32>, file: &str) -> (Option<i32>, String) {
    let mut cmd = match stack {
        Some(kb) => {
            let mut sh = Command::new("sh");
            sh.args([
                "-c",
                &format!(
                    "ulimit -s {kb} && exec '{}' ast '{file}'",
                    selfhost().display()
                ),
            ]);
            sh
        }
        None => {
            let mut n = Command::new(selfhost());
            n.args(["ast", file]);
            n
        }
    };
    let out = output_within(
        cmd.current_dir(dir).stdin(Stdio::null()),
        CHILD_LIMIT,
        &format!("selfhost ast {file} on a {stack:?} KB stack"),
    )
    .unwrap_or_else(|e| panic!("{e}"));
    (
        out.status.code(),
        String::from_utf8_lossy(&out.stdout).into_owned(),
    )
}

/// N's own stack (spec §4.4, §4.8): every kind of nesting the deep
/// generator writes, at 128 levels and at 129, through N on a main thread
/// of 8 MB, Linux's, and of 1 MB, Windows', on every system: it neither
/// overflows nor reports P0002 at 128, and reports one P0002 at 129. A
/// Nova program runs on its platform's main thread, so on Windows N runs
/// as it is, and elsewhere `ulimit -s` sets the size. N needed 256 KB at
/// most when measured (ADR 0052).
#[test]
fn n_parses_128_levels_of_every_kind_on_a_1_mb_main_thread() {
    let dir = fresh_dir("deep-n");
    let mut rng = SplitMix64::new(DEEP_SEEDS[0]);
    let mut files = Vec::new();
    for kind in fuzz::DEEP_KINDS {
        for depth in [128, 129] {
            let file = format!("{kind}-{depth}.nova");
            std::fs::write(dir.join(&file), fuzz::deep(&mut rng, kind, depth))
                .expect("write a deep input");
            files.push((depth, file));
        }
    }
    let stacks: &[Option<u32>] = if cfg!(unix) {
        &[Some(8192), Some(1024)]
    } else {
        &[None]
    };
    for &stack in stacks {
        for (depth, file) in &files {
            let (code, out) = n_on_stack(&dir, stack, file);
            let p0002 = out
                .lines()
                .filter(|l| l.starts_with("PARSE ") && l.contains("nested too deeply"))
                .count();
            let expected = usize::from(*depth > 128);
            assert!(
                code == Some(0) && p0002 == expected,
                "N on {file} with a {stack:?} KB stack: exit {code:?}, {p0002} P0002\n{out}"
            );
        }
    }
}

/// `nova` with `args` and `file`, in `dir`: its exit code and stderr.
fn nova_on(dir: &Path, args: &[&str], file: &str) -> (Option<i32>, String) {
    let out = output_within(
        Command::new(nova())
            .current_dir(dir)
            .env("NOVA_HOME", dir.join("home"))
            // The diagnostics are matched as plain text; termcolor's
            // `Auto` colours them on Windows, where it asks no terminal.
            .env("NO_COLOR", "1")
            .args(args)
            .arg(file)
            .stdin(Stdio::null()),
        CHILD_LIMIT,
        &format!("nova {} {file}", args.join(" ")),
    )
    .unwrap_or_else(|e| panic!("{e}"));
    (
        out.status.code(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
    )
}

/// S0 meets the depth limit, not its stack, on every deep input, on every
/// system (spec §4.4, §4.8): `nova parse --check` passes each at 128 levels
/// and below and gives one P0002 past it, as `nova check` does at 128 and
/// 129, every stage after the parser included.
#[test]
fn s0_meets_the_depth_limit_on_the_deep_inputs_not_its_stack() {
    let dir = fresh_dir("deep-s0");
    for (i, (kind, depth, src)) in fuzz::deeps(DEEP_SEEDS[0], DEEP_MIXED)
        .into_iter()
        .enumerate()
    {
        let file = format!("{kind}-{depth}-{i}.nova");
        std::fs::write(dir.join(&file), &src).expect("write a deep input");
        let mut commands: Vec<&[&str]> = vec![&["parse", "--check"]];
        if depth == 128 || depth == 129 {
            commands.push(&["check"]);
        }
        for args in commands {
            let (code, stderr) = nova_on(&dir, args, &file);
            let what = format!("nova {} {file}", args.join(" "));
            assert!(!stderr.contains("overflowed"), "{what}:\n{stderr}");
            let p0002 = stderr
                .matches("error[P0002]: this is nested too deeply")
                .count();
            if depth <= 128 {
                // Nested items and optional types are E0900 to the
                // checker; nothing else is an error.
                let errors_ok =
                    stderr.matches("error[").count() == stderr.matches("error[E0900]").count();
                let unsupported = kind == "items" || kind == "optionals";
                let code_ok = code == Some(0) || (unsupported && code == Some(1));
                assert!(
                    p0002 == 0 && errors_ok && code_ok,
                    "{what}: {code:?}\n{stderr}"
                );
            } else {
                assert!(p0002 == 1 && code == Some(1), "{what}: {code:?}\n{stderr}");
            }
        }
    }
}

/// A panic on the pipeline's thread is printed once, naming the thread
/// `main` as before, and `nova` exits 101: here `println!` into a full
/// device, Linux's `/dev/full`.
#[cfg(target_os = "linux")]
#[test]
fn a_panic_on_the_pipelines_thread_exits_101_with_its_message() {
    let dir = fresh_dir("panic");
    std::fs::write(dir.join("a.nova"), "fn main() {}\n").expect("write a.nova");
    let full = std::fs::OpenOptions::new()
        .write(true)
        .open("/dev/full")
        .expect("open /dev/full");
    let out = Command::new(nova())
        .current_dir(&dir)
        .args(["parse", "a.nova"])
        .env("RUST_BACKTRACE", "0")
        .stdout(full)
        .output()
        .expect("run nova parse");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert_eq!(out.status.code(), Some(101), "{stderr}");
    assert!(
        stderr.starts_with("\nthread 'main' ") || stderr.starts_with("thread 'main' "),
        "{stderr}"
    );
    assert_eq!(stderr.matches("panicked at").count(), 1, "{stderr}");
    assert!(
        stderr.contains("failed printing to stdout: No space left on device"),
        "{stderr}"
    );
}

/// ADR 0051's known bug: a decimal number of 4,000 characters overflowed
/// S0's main thread at 1 MB, Windows' size. On the pipeline's thread,
/// `nova parse --dump-tokens` lexes it with the main thread held at 1 MB.
#[cfg(unix)]
#[test]
fn a_long_decimal_number_lexes_with_a_1_mb_main_thread() {
    let dir = fresh_dir("long-number");
    std::fs::write(
        dir.join("a.nova"),
        format!("let a = {}\n", "1".repeat(4000)),
    )
    .expect("write a.nova");
    let script = format!(
        "ulimit -s 1024 && exec '{}' parse --dump-tokens a.nova",
        nova().display()
    );
    let out = output_within(
        Command::new("sh")
            .current_dir(&dir)
            .args(["-c", &script])
            .stdin(Stdio::null()),
        CHILD_LIMIT,
        "nova parse --dump-tokens under ulimit -s 1024",
    )
    .unwrap_or_else(|e| panic!("{e}"));
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert_eq!(
        out.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(stdout.contains(" Eof NL\n"), "{stdout}");
}

/// `nova run` compiles on the pipeline's thread and runs the program on the
/// main thread, so the program has the stack it has when built, the
/// platform's main thread's (ADR 0052): with the main thread held at 8 MB,
/// a recursion 100,000 deep runs, and one 2,000,000 deep, which 64 MB would
/// hold, overflows it.
#[cfg(unix)]
#[test]
fn nova_run_runs_the_program_on_the_main_thread() {
    let dir = fresh_dir("run-stack");
    let program = |n: usize| {
        format!(
            "fn down(n: Int) -> Int {{\n    if n == 0 {{ return 0 }}\n    1 + down(n - 1)\n}}\n\n\
             fn main() {{\n    println(\"${{down({n})}}\")\n}}\n"
        )
    };
    for (n, file) in [(100_000, "shallow.nova"), (2_000_000, "deep.nova")] {
        std::fs::write(dir.join(file), program(n)).expect("write a program");
    }
    let run = |file: &str| {
        let script = format!("ulimit -s 8192 && exec '{}' run {file}", nova().display());
        let out = output_within(
            Command::new("sh")
                .current_dir(&dir)
                .env("NOVA_HOME", dir.join("home"))
                .args(["-c", &script])
                .stdin(Stdio::null()),
            CHILD_LIMIT,
            &format!("nova run {file} under ulimit -s 8192"),
        )
        .unwrap_or_else(|e| panic!("{e}"));
        (
            out.status.code(),
            String::from_utf8_lossy(&out.stdout).into_owned(),
            String::from_utf8_lossy(&out.stderr).into_owned(),
        )
    };
    let (code, stdout, stderr) = run("shallow.nova");
    assert_eq!((code, stdout.as_str()), (Some(0), "100000\n"), "{stderr}");
    let (code, _, stderr) = run("deep.nova");
    assert_ne!(code, Some(0), "{stderr}");
    assert!(stderr.contains("has overflowed its stack"), "{stderr}");
}

// --- 5.2a: the resolution dump (spec
// `docs/superpowers/specs/2026-11-02-phase-5-2-the-loader-and-the-resolver-design.md`
// §3.4, §3.5; ADR 0053) ---

/// S0's arguments for the resolution dump, each file after them a
/// program's root.
const RESOLUTION_S0: &[&str] = &["check", "--dump-resolution"];

/// S0's HIR dump (5.3a, ADR 0055): the hidden `check --dump-hir`, before
/// the mode's flags and the roots.
const HIR_S0: &[&str] = &["check", "--dump-hir"];

/// S0's HIR dump's item part alone, 5.3a's dump (5.3b): the hidden `check
/// --dump-hir-items`, which the differential compares with N's `hir`
/// until N checks bodies ([`N_BODIES`]).
const HIR_ITEMS_S0: &[&str] = &["check", "--dump-hir-items"];

/// A package's `nova.toml` with what the package graph requires and
/// nothing else: the tests' packages (5.2b, ADR 0054).
const MANIFEST: &str = "[package]\nname = \"pkg\"\nversion = \"0.1.0\"\nedition = \"2026\"\n";

/// What a program is resolved under (5.2b, ADR 0054): `check`'s pipeline
/// natively, `check`'s for the WASM target, or `nova test`'s.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Mode {
    Check,
    Wasm,
    Test,
}

impl Mode {
    /// The arguments for the mode after [`RESOLUTION_S0`] on S0's side, and
    /// after `resolve --std DIR` on N's: the same on both.
    fn flags(self) -> &'static [&'static str] {
        match self {
            Mode::Check => &[],
            Mode::Wasm => &["--target", "wasm"],
            Mode::Test => &["--test"],
        }
    }
}

/// The error cases of the resolver (spec §3.5, §4): the inline sources of
/// `nova-resolver`'s tests and of the type checker's tests that reach the
/// resolver, each a file, or a directory whose `main.nova` is the root of
/// a program of several modules; and, from 5.2b (ADR 0054), the programs
/// the driver's, `run_tests.rs`'s and `wasm_tests.rs`'s tests resolve as
/// packages, for the WASM target or under test mode, packages each a
/// directory holding a `nova.toml` ([`case_roots`]). They are where the
/// mode they are resolved under puts them: `cases/resolve/` under `check`,
/// `cases/resolve-wasm/` for the WASM target, and `cases/resolve-test/`
/// under test mode. The directory names the mode, so a case needs no
/// marker of its own.
fn mode_cases_dir(mode: Mode) -> PathBuf {
    cases_dir().join(mode_cases_name(mode))
}

/// [`mode_cases_dir`]'s last component.
fn mode_cases_name(mode: Mode) -> &'static str {
    match mode {
        Mode::Check => "resolve",
        Mode::Wasm => "resolve-wasm",
        Mode::Test => "resolve-test",
    }
}

/// The programs of a directory of cases, each a root named from that
/// directory but those in `apart` (ADR 0053 §9, ADR 0054):
/// - a `.nova` file is a program;
/// - a directory holding a `nova.toml` is a package, and each `.nova` file
///   directly in its `src/`, then in its `tests/`, sorted, is a program's
///   root, a module of that package (`packages`);
/// - any other directory is a loose program whose root is its `main.nova`,
///   but one holding a module named for a std module (`dom.nova`,
///   `ui.nova`, `RESERVED_MODULE_NAMES`) runs from inside it, its root
///   named `main.nova` (`apart`): E0093 names that module by its path, the
///   root's directory joined with its file name, which S0 writes with `\`
///   on Windows and N with `/`, so the path must have no directory part
///   (ADR 0053 §5).
struct CaseRoots {
    loose: Vec<String>,
    apart: Vec<PathBuf>,
    packages: Vec<String>,
}

fn case_roots(cases: &Path) -> CaseRoots {
    let mut entries: Vec<_> = std::fs::read_dir(cases)
        .expect("read the resolver's cases")
        .map(|e| e.expect("a directory entry"))
        .collect();
    entries.sort_by_key(|e| e.file_name());
    let mut roots = CaseRoots {
        loose: Vec::new(),
        apart: Vec::new(),
        packages: Vec::new(),
    };
    for e in entries {
        let path = e.path();
        let name = e.file_name().to_string_lossy().into_owned();
        if path.is_dir() && path.join("nova.toml").is_file() {
            let before = roots.packages.len();
            for sub in ["src", "tests"] {
                let Ok(dir) = std::fs::read_dir(path.join(sub)) else {
                    continue;
                };
                let mut files: Vec<String> = dir
                    .map(|e| e.expect("a directory entry").file_name())
                    .map(|n| n.to_string_lossy().into_owned())
                    .filter(|n| n.ends_with(".nova"))
                    .collect();
                files.sort();
                roots
                    .packages
                    .extend(files.into_iter().map(|f| format!("{name}/{sub}/{f}")));
            }
            assert!(
                roots.packages.len() > before,
                "{} has no root",
                path.display()
            );
        } else if path.is_dir() {
            assert!(
                path.join("main.nova").is_file(),
                "{} has no main.nova",
                path.display()
            );
            if ["dom", "ui"]
                .iter()
                .any(|std| path.join(format!("{std}.nova")).is_file())
            {
                roots.apart.push(path);
            } else {
                roots.loose.push(format!("{name}/main.nova"));
            }
        } else if name.ends_with(".nova") {
            roots.loose.push(name);
        }
    }
    roots
}

/// [`agree_on_resolution_in`] `mode` on the resolver's cases in `cases`
/// ([`mode_cases_dir`], or a copy of it) ([`case_roots`]): the loose
/// programs and the packages' roots from `cases`, in one process a side,
/// then each program apart from its own directory. Returns S0's output,
/// which is N's, and how many programs it holds.
fn agree_on_resolve_cases(mode: Mode, cases: &Path) -> (String, usize) {
    let roots = case_roots(cases);
    let count = roots.loose.len() + roots.apart.len() + roots.packages.len();
    let from_cases: Vec<String> = roots.loose.into_iter().chain(roots.packages).collect();
    let mut all = agree_on_resolution_in(mode, cases, &from_cases, 0);
    for dir in &roots.apart {
        all.push_str(&agree_on_resolution_in(
            mode,
            dir,
            &["main.nova".to_string()],
            0,
        ));
    }
    (all, count)
}

/// Whether `file`, a corpus file relative to the repository, is directly
/// in a package's `src/` or `tests/` (`nova_driver::package_of`).
fn in_package(file: &str) -> bool {
    let path = root().join(file);
    let parent = path.parent().expect("a parent");
    matches!(
        parent.file_name().and_then(|n| n.to_str()),
        Some("src" | "tests")
    ) && parent
        .parent()
        .is_some_and(|p| p.join("nova.toml").is_file())
}

/// The loose programs of the corpus: every corpus file but those directly
/// in a package's `src/` or `tests/` ([`package_corpus`]), each a file run
/// alone or a root with the sibling files it imports.
fn loose_corpus() -> Vec<String> {
    corpus().into_iter().filter(|f| !in_package(f)).collect()
}

/// The corpus files directly in a package's `src/` or `tests/` (5.2b):
/// each a root, a module of its package.
fn package_corpus() -> Vec<String> {
    corpus().into_iter().filter(|f| in_package(f)).collect()
}

/// The corpus files resolved for the WASM target (spec §4.2): those under
/// `tests/dom/` and `tests/wasm/`, packages included.
fn wasm_corpus() -> Vec<String> {
    corpus()
        .into_iter()
        .filter(|f| f.starts_with("tests/dom/") || f.starts_with("tests/wasm/"))
        .collect()
}

/// The corpus files resolved under test mode (spec §4.3): those that hold
/// a line beginning with `@test`, after its indentation.
fn test_corpus() -> Vec<String> {
    let root = root();
    corpus()
        .into_iter()
        .filter(|f| {
            let text = std::fs::read_to_string(root.join(f)).expect("read a corpus file");
            text.lines().any(|l| l.trim_start().starts_with("@test"))
        })
        .collect()
}

/// The resolution dump through both sides under `check` (ADR 0053):
/// [`resolution_in`] [`Mode::Check`].
fn resolution() -> &'static Dump {
    resolution_in(Mode::Check)
}

/// The resolution dump through both sides under `mode` (ADR 0053, ADR
/// 0054): S0's `check --dump-resolution`, and N's `resolve --std DIR`, DIR
/// the repository's `std/`, each with the mode's flags and followed by the
/// roots.
fn resolution_in(mode: Mode) -> &'static Dump {
    static DUMPS: [OnceLock<Dump>; 3] = [OnceLock::new(), OnceLock::new(), OnceLock::new()];
    DUMPS[mode as usize].get_or_init(|| dump_with(mode, &[]))
}

/// std's part through both sides under `mode`: S0's `check
/// --dump-resolution --std`, and N's `resolve --std DIR --std-part`, each
/// with the mode's flags.
fn std_resolution(mode: Mode) -> &'static Dump {
    static DUMPS: [OnceLock<Dump>; 3] = [OnceLock::new(), OnceLock::new(), OnceLock::new()];
    DUMPS[mode as usize].get_or_init(|| dump_with(mode, &["--std"]))
}

/// The resolution dump under `mode`, with `extra` after S0's flags: `--std`
/// is N's `--std-part`, and any other is passed to both as it is.
fn dump_with(mode: Mode, extra: &[&str]) -> Dump {
    let s0: Vec<&str> = RESOLUTION_S0
        .iter()
        .chain(mode.flags())
        .chain(extra)
        .copied()
        .collect();
    let std_dir = std_dir();
    let mut n = vec!["resolve", "--std", std_dir.as_str()];
    n.extend(mode.flags());
    n.extend(
        extra
            .iter()
            .map(|a| if *a == "--std" { "--std-part" } else { a }),
    );
    Dump {
        s0: leak_args(&s0),
        n: leak_args(&n),
    }
}

/// The repository's `std/`, which N reads its std modules from.
fn std_dir() -> String {
    root().join("std").display().to_string()
}

/// `args` for a [`Dump`], which holds them for the whole run.
fn leak_args(args: &[&str]) -> &'static [&'static str] {
    let owned: Vec<&'static str> = args
        .iter()
        .map(|a| &*Box::leak(a.to_string().into_boxed_str()))
        .collect();
    Box::leak(owned.into_boxed_slice())
}

/// [`agree_on_resolution_in`] [`Mode::Check`].
fn agree_on_resolution(dir: &Path, files: &[String], code: i32) -> String {
    agree_on_resolution_in(Mode::Check, dir, files, code)
}

/// [`agree_on`] the resolution dump under `mode`: `files`, relative to
/// `dir`, each a program's root, through both sides, all in one process a
/// side, each side exiting with `code`; fail with the report of the first
/// difference. Returns S0's output, which is N's. N reads and parses std
/// once per process, about 0.4 s with the debug runtime (0.55 s for the
/// WASM target, which also parses `STD_MODULES` for `missing`), so a set
/// of roots goes to one process, not to batches (5.2b, ADR 0054 §9), and
/// the callers run their sets side by side.
fn agree_on_resolution_in(mode: Mode, dir: &Path, files: &[String], code: i32) -> String {
    if files.is_empty() {
        return String::new();
    }
    compare_batch(resolution_in(mode), dir, files, code, &nova(), selfhost())
        .unwrap_or_else(|report| panic!("{report}"))
}

/// [`agree_on_resolution_in`] `files` in `parts` processes a side, side by
/// side, root `i` in part `i % parts`: for a set whose roots are slow to
/// load, the corpus's packages (`selfhost/src/`'s each reach most of N),
/// which in one process would make the longest wait of the file. Returns
/// S0's output, each root's part in the roots' order, each checked to be
/// its root's, and none left over, so no root is dropped or dealt twice.
fn agree_on_resolution_in_parts(
    mode: Mode,
    dir: &Path,
    files: &[String],
    code: i32,
    parts: usize,
) -> String {
    agree_in_parts(files, parts, |part| {
        agree_on_resolution_in(mode, dir, part, code)
    })
}

/// `agree` on `files` in `parts` processes a side, side by side, root `i`
/// in part `i % parts` ([`agree_on_resolution_in_parts`]): each root's
/// part of the outputs in the roots' order, each checked to be its root's,
/// and none left over. The HIR dump's sets go through it too (5.3a).
fn agree_in_parts(
    files: &[String],
    parts: usize,
    agree: impl Fn(&[String]) -> String + Sync,
) -> String {
    let parts = parts.clamp(1, files.len().max(1));
    let dealt: Vec<Vec<String>> = (0..parts)
        .map(|k| files.iter().skip(k).step_by(parts).cloned().collect())
        .collect();
    let agree = &agree;
    let outs: Vec<String> = std::thread::scope(|scope| {
        let runs: Vec<_> = dealt
            .iter()
            .map(|part| scope.spawn(move || agree(part)))
            .collect();
        joined(runs)
    });
    // Each output's parts, by its roots' headers, dealt back in order.
    let mut split: Vec<std::collections::VecDeque<String>> = outs
        .iter()
        .map(|out| {
            let mut roots = std::collections::VecDeque::new();
            for line in out.split_inclusive('\n') {
                if line.starts_with("== ") || roots.is_empty() {
                    roots.push_back(String::new());
                }
                roots.back_mut().expect("a root's part").push_str(line);
            }
            roots
        })
        .collect();
    let mut all = String::new();
    for (i, root) in files.iter().enumerate() {
        let part = split[i % parts].pop_front().expect("each root's part");
        assert!(
            part.starts_with(&format!("== {root}\n")),
            "root {i}, {root}, was dealt the part of {}",
            part.lines().next().unwrap_or("")
        );
        all.push_str(&part);
    }
    assert!(
        split.iter().all(|s| s.is_empty()),
        "a part no root was dealt"
    );
    all
}

/// `nova check`'s help names neither hidden flag, and every `nova check`
/// command line whose first argument after `check` is not
/// `--dump-resolution` is parsed as before the flags (ADR 0053 §1): each
/// shape here gives the base's error, byte for byte, tips and usage line
/// included, which names neither flag; with `--help` before the flag,
/// `check`'s help. The dump's own parser refuses its misuses with one
/// error and the exit code 2.
#[test]
fn the_resolution_dump_flags_are_hidden_and_refused_as_before() {
    let help = Command::new(nova())
        .args(["check", "--help"])
        .output()
        .expect("run nova check --help");
    let help = String::from_utf8_lossy(&help.stdout).into_owned();
    assert!(help.contains("--target"), "{help}");
    assert!(!help.contains("dump-resolution"), "{help}");
    assert!(!help.contains("--std"), "{help}");
    assert!(!help.contains("--test"), "{help}");
    let out = Command::new(nova())
        .args(["check", "--help", "--dump-resolution"])
        .output()
        .expect("run nova check --help --dump-resolution");
    assert_eq!(out.status.code(), Some(0));
    assert_eq!(String::from_utf8_lossy(&out.stdout), help);
    // The base's errors: (arguments, the argument named, the usage line's
    // arguments); a tip follows a refused flag, as clap gives it.
    let (opts, file, target) = ("[OPTIONS] [FILE]", "<FILE>", "--target <TARGET> [FILE]");
    for (args, named, usage) in [
        (&["check", "a.nova", "b.nova"][..], "b.nova", opts),
        (&["check", "a.nova", ""], "", opts),
        (&["check", "--std"], "--std", opts),
        (&["check", "--std", "a.nova"], "--std", opts),
        (&["check", "--", "a.nova", "b.nova"], "b.nova", opts),
        (
            &["check", "--target", "wasm", "a.nova", "b.nova"],
            "b.nova",
            opts,
        ),
        (&["check", "a.nova", "--std"], "--std", file),
        (&["check", "--st", "a.nova"], "--st", opts),
        (&["check", "--stdd", "a.nova"], "--stdd", opts),
        (&["check", "--std-part", "a.nova"], "--std-part", opts),
        (&["check", "--test", "a.nova"], "--test", opts),
        (&["check", "--test"], "--test", opts),
        (&["check", "a.nova", "--test"], "--test", file),
        (
            &["check", "--test", "--dump-resolution", "a.nova"],
            "--test",
            opts,
        ),
        (&["check", "--dump", "a.nova"], "--dump", opts),
        (&["check", "--dump-res", "a.nova"], "--dump-res", opts),
        (
            &["check", "--dump-resolutio", "a.nova"],
            "--dump-resolutio",
            opts,
        ),
        (&["check", "--std=1", "a.nova"], "--std", opts),
        (
            &["check", "--dump-resolution=1", "a.nova"],
            "--dump-resolution",
            opts,
        ),
        (&["check", "--std", "--std", "a.nova"], "--std", opts),
        (
            &["check", "a.nova", "b.nova", "--target", "bogus"],
            "b.nova",
            opts,
        ),
        (&["check", "--std", "--help"], "--std", opts),
        (&["check", "--std", "-h"], "--std", opts),
        (&["check", "a.nova", "b.nova", "--bogus"], "b.nova", opts),
        (
            &["check", "a.nova", "--dump-resolution"],
            "--dump-resolution",
            file,
        ),
        (
            &["check", "--target", "native", "--dump-resolution", "a.nova"],
            "--dump-resolution",
            target,
        ),
    ] {
        let out = Command::new(nova())
            .args(args)
            .output()
            .expect("run nova check");
        let stderr = String::from_utf8_lossy(&out.stderr);
        let tip = if named.starts_with('-') {
            format!("  tip: to pass '{named}' as a value, use '-- {named}'\n\n")
        } else {
            String::new()
        };
        assert_eq!(out.status.code(), Some(2), "{args:?}: {stderr}");
        assert_eq!(
            stderr,
            format!(
                "error: unexpected argument '{named}' found\n\n{tip}\
                 Usage: nova{EXE_SUFFIX} check {usage}\n\n\
                 For more information, try '--help'.\n"
            ),
            "{args:?}"
        );
        assert!(out.stdout.is_empty(), "{args:?}");
    }
    // The dump's misuses, which the base has no form of; `--test` is
    // native only, as `nova test` is (5.2b).
    for args in [
        &["--stdd"][..],
        &["--std", "--std"],
        &["--target", "bogus", "a.nova"],
        &["--help"],
        &["-h"],
        &["--dump-resolution", "a.nova"],
        &["--test", "--test", "a.nova"],
        &["--test=1", "a.nova"],
        &["--tests", "a.nova"],
        &["--test", "--target", "wasm", "a.nova"],
        &["--target=wasm", "--test", "--std"],
        &["--test", "--std", "a.nova"],
        &["--test"],
    ] {
        let out = Command::new(nova())
            .args(RESOLUTION_S0)
            .args(args)
            .output()
            .expect("run nova check --dump-resolution");
        let stderr = String::from_utf8_lossy(&out.stderr);
        assert_eq!(out.status.code(), Some(2), "{args:?}: {stderr}");
        assert!(stderr.starts_with("error: "), "{args:?}: {stderr}");
        assert_eq!(stderr.matches("error:").count(), 1, "{args:?}: {stderr}");
        // A usage line, when clap gives one, is the dump's own.
        assert_eq!(
            stderr.matches("Usage: ").count(),
            stderr
                .matches("Usage: nova check --dump-resolution ")
                .count(),
            "{args:?}: {stderr}"
        );
        assert!(out.stdout.is_empty(), "{args:?}");
    }
}

/// Each root's part follows a header with its path as given; a root that
/// cannot be read is `IOERR` and its kind, the rest are dumped, and the
/// exit code is 2; a root in a package is a module of the package (ADR
/// 0054); a program that fails to load holds the diagnostics so far;
/// `--std` prints std's part for the target and mode. N prints the same
/// for the same roots.
#[test]
fn the_resolution_dump_has_a_header_per_root_and_std_its_own_part() {
    let dir = fresh_dir("resolution-headers");
    std::fs::write(dir.join("a.nova"), "fn main() {}\n").expect("write a.nova");
    std::fs::write(dir.join("b.nova"), "fn g( {").expect("write b.nova");
    std::fs::create_dir_all(dir.join("pkg").join("src")).expect("create pkg/src");
    std::fs::write(dir.join("pkg").join("nova.toml"), MANIFEST).expect("write nova.toml");
    std::fs::write(
        dir.join("pkg").join("src").join("main.nova"),
        "fn main() {}\n",
    )
    .expect("write pkg/src/main.nova");
    let out = Command::new(nova())
        .current_dir(&dir)
        .args(RESOLUTION_S0)
        .args(["a.nova", "missing.nova", "pkg/src/main.nova", "b.nova"])
        .output()
        .expect("run nova check --dump-resolution");
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert_eq!(out.status.code(), Some(2), "{stdout}");
    assert!(
        stdout.starts_with(
            "== a.nova\n(Program target=native resolved=true\n  \
             (Module 0 \"a\" package=None)\n  (StdStart module=1 def=1 item=1)\n  \
             (Def.Fn u0 \"main\" \"a\":3..7 item=0)\n  (Scope 0\n"
        ),
        "{stdout}"
    );
    assert!(
        stdout.contains(
            "  (Items 0..1 0))\n(Diagnostics)\n\
             == missing.nova\nIOERR NotFound\n\
             == pkg/src/main.nova\n(Program target=native resolved=true\n  \
             (Module 0 \"main\" package=0)\n"
        ),
        "{stdout}"
    );
    assert!(
        stdout.contains(
            "  (Items 0..1 0))\n(Diagnostics)\n\
             == b.nova\n(Program target=native resolved=false\n  \
             (Module 0 \"b\" package=None))\n(Diagnostics\n  \
             (Error \"P0001\" \"expected identifier (in parameter name), found `{`\"\n    \
             (Primary \"b\":6..7 \"here\"))\n"
        ),
        "{stdout}"
    );
    // N prints the same, a package's root, a file that is not UTF-8 and a
    // root beside a `nova.toml` that is a directory, which makes no package,
    // included.
    std::fs::write(dir.join("latin1.nova"), b"fn \xe9() {}\n").expect("write latin1.nova");
    std::fs::create_dir_all(dir.join("dirpkg").join("nova.toml")).expect("create nova.toml/");
    std::fs::create_dir_all(dir.join("dirpkg").join("src")).expect("create dirpkg/src");
    std::fs::write(
        dir.join("dirpkg").join("src").join("main.nova"),
        "fn main() {}\n",
    )
    .expect("write dirpkg/src/main.nova");
    let files = [
        "a.nova",
        "missing.nova",
        "pkg/src/main.nova",
        "b.nova",
        "dirpkg/src/main.nova",
        "latin1.nova",
    ]
    .map(String::from);
    let out = agree_on_resolution(&dir, &files, 2);
    assert!(
        out.contains("== dirpkg/src/main.nova\n(Program target=native resolved=true\n"),
        "{out}"
    );
    assert!(
        out.ends_with("== latin1.nova\nIOERR InvalidData\n"),
        "{out}"
    );
    // No root: nothing printed, and 2; nor a root with `--std`.
    for args in [&[][..], &["--std", "a.nova"]] {
        let out = Command::new(nova())
            .current_dir(&dir)
            .args(RESOLUTION_S0)
            .args(args)
            .output()
            .expect("run nova check --dump-resolution");
        assert_eq!(out.status.code(), Some(2), "{args:?}");
        assert!(out.stdout.is_empty(), "{args:?}");
    }
    // An empty root is a file that is not there (ADR 0053 §1), not a
    // refused argument.
    let out = Command::new(nova())
        .current_dir(&dir)
        .args(RESOLUTION_S0)
        .arg("")
        .output()
        .expect("run nova check --dump-resolution \"\"");
    assert_eq!(out.status.code(), Some(2));
    assert_eq!(
        String::from_utf8_lossy(&out.stdout),
        "== \nIOERR NotFound\n"
    );
    assert!(out.stderr.is_empty());
    for (args, head) in [
        (
            &["--target", "native"][..],
            "(Std target=native modules=21 ",
        ),
        (&["--target", "wasm"], "(Std target=wasm modules=16 "),
        (&["--test"], "(Std target=native mode=test modules=22 "),
    ] {
        let target = args.join(" ");
        let out = Command::new(nova())
            .args(RESOLUTION_S0)
            .arg("--std")
            .args(args)
            .output()
            .expect("run nova check --dump-resolution --std");
        let stdout = String::from_utf8_lossy(&out.stdout);
        assert_eq!(out.status.code(), Some(0), "{target}");
        assert!(stdout.starts_with(head), "{target}: {}", &stdout[..80]);
        assert!(stdout.ends_with("))\n(Diagnostics)\n"), "{target}");
        assert!(
            !stdout.contains(" u"),
            "{target}: std's part names a user definition"
        );
    }
}

/// std's part, every loose corpus program and every resolver case give the
/// same resolution dump on both sides, byte for byte (spec §3.5, §3.7):
/// std's part once, then the programs, [`BATCH`] to a process. 5.2b (spec
/// §4, §4.5; ADR 0054) adds std's WASM and test-mode parts, the corpus's
/// packages' files natively, its WASM programs (`tests/dom/`,
/// `tests/wasm/`) for the WASM target and its files with tests under test
/// mode, and the cases of every mode, each compared on both sides. Every
/// form of the dump's `KINDS` appears in these dumps, every form they print
/// is listed, and no form of `LATER_KINDS` appears; E0084 and E0085 occur
/// only under test mode, which keeps `@test` functions, and E0090 only for
/// the WASM target. The dumps are S0's and N's alike, so each holds them.
#[test]
fn the_resolution_dumps_agree_on_std_the_corpus_and_the_cases() {
    let root = root();
    // std's parts side by side, while the corpus and the cases are listed.
    let std_parts: Vec<String> = std::thread::scope(|scope| {
        let parts: Vec<_> = [Mode::Check, Mode::Wasm, Mode::Test]
            .into_iter()
            .map(|mode| {
                let root = &root;
                scope.spawn(move || {
                    compare_batch(std_resolution(mode), root, &[], 0, &nova(), selfhost())
                        .unwrap_or_else(|report| panic!("{mode:?}: {report}"))
                })
            })
            .collect();
        joined(parts)
    });
    for (part, mode) in std_parts.iter().zip([Mode::Check, Mode::Wasm, Mode::Test]) {
        assert!(part.ends_with("))\n(Diagnostics)\n"), "{mode:?}");
    }
    for (part, head) in std_parts.iter().zip([
        "(Std target=native modules=21 ",
        "(Std target=wasm modules=16 ",
        "(Std target=native mode=test modules=22 ",
    ]) {
        assert!(part.starts_with(head), "{}", &part[..80]);
    }
    let loose = loose_corpus();
    // 251 loose programs at 5.2a.
    assert!(loose.len() >= 240, "only {} loose programs", loose.len());
    // 5.2b: the 67 files directly in a package's src/ or tests/, in 11
    // packages (the nine the spec counts, `05-geometry`'s dependency and
    // the VS Code extension's fixture), each a root, which 5.2a skipped.
    let packages = package_corpus();
    let package_dirs: std::collections::BTreeSet<&str> = packages
        .iter()
        .filter_map(|f| f.rsplitn(3, '/').nth(2))
        .collect();
    assert!(
        packages.len() >= 60,
        "only {} package files",
        packages.len()
    );
    assert!(
        package_dirs.contains("examples/05-geometry/shapes") && package_dirs.len() >= 10,
        "{package_dirs:?}"
    );
    // 39 WASM programs at 5.2b, six of them a package's; 9 files with tests.
    let wasm = wasm_corpus();
    assert!(wasm.len() >= 35, "only {} WASM programs", wasm.len());
    let tests = test_corpus();
    assert!(tests.len() >= 8, "only {} files with tests", tests.len());
    eprintln!(
        "the resolution pass compares {} loose programs, {} package files in {} packages, \
         {} WASM programs and {} files with tests",
        loose.len(),
        packages.len(),
        package_dirs.len(),
        wasm.len(),
        tests.len()
    );
    // The corpus and the cases side by side.
    let (check, wasm_dumps, test_dumps, counts) = std::thread::scope(|scope| {
        let cases: Vec<_> = [Mode::Check, Mode::Wasm, Mode::Test]
            .into_iter()
            .map(|mode| scope.spawn(move || agree_on_resolve_cases(mode, &mode_cases_dir(mode))))
            .collect();
        let package_dumps =
            scope.spawn(|| agree_on_resolution_in_parts(Mode::Check, &root, &packages, 0, 3));
        let wasm_dumps = scope.spawn(|| agree_on_resolution_in(Mode::Wasm, &root, &wasm, 0));
        let test_dumps = scope.spawn(|| agree_on_resolution_in(Mode::Test, &root, &tests, 0));
        let mut check = agree_on_resolution(&root, &loose, 0);
        check.push_str(&package_dumps.join().expect("the packages' thread"));
        let mut wasm_dumps = wasm_dumps.join().expect("the WASM programs' thread");
        let mut test_dumps = test_dumps.join().expect("the tests' thread");
        let mut counts = Vec::new();
        for (mode, case) in [Mode::Check, Mode::Wasm, Mode::Test].into_iter().zip(cases) {
            let (dumps, count) = case.join().expect("a mode's cases' thread");
            counts.push(count);
            match mode {
                Mode::Check => check.push_str(&dumps),
                Mode::Wasm => wasm_dumps.push_str(&dumps),
                Mode::Test => test_dumps.push_str(&dumps),
            }
        }
        (check, wasm_dumps, test_dumps, counts)
    });
    // 103 cases at 5.2a; at 5.2b 145 programs under `check` (42 of them a
    // package's roots), 30 for the WASM target and 126 under test mode.
    assert!(counts[0] >= 130, "only {} resolver cases", counts[0]);
    assert!(counts[1] >= 25, "only {} WASM cases", counts[1]);
    assert!(counts[2] >= 115, "only {} test-mode cases", counts[2]);
    // Each root's part, so every program was compared.
    let parts = |dumps: &str, head: &str| dumps.lines().filter(|l| l.starts_with(head)).count();
    assert_eq!(
        parts(&check, "(Program target=native resolved="),
        loose.len() + packages.len() + counts[0]
    );
    assert_eq!(
        parts(&wasm_dumps, "(Program target=wasm resolved="),
        wasm.len() + counts[1]
    );
    assert_eq!(
        parts(&test_dumps, "(Program target=native mode=test resolved="),
        tests.len() + counts[2]
    );
    // E0084 and E0085 only under test mode, E0090 only for the WASM
    // target (decision 8; spec §4.2, §4.3), though cases of every mode hold
    // the programs that would give them; and test mode gives both.
    for (dumps, absent) in [
        (&check, &["E0084", "E0085", "E0090"][..]),
        (&wasm_dumps, &["E0084", "E0085"]),
        (&test_dumps, &["E0090"]),
    ] {
        let codes = codes_in(dumps);
        for code in absent {
            assert!(!codes.contains_key(*code), "{code} in {codes:?}");
        }
    }
    let test_codes = codes_in(&test_dumps);
    assert!(
        test_codes.contains_key("E0084") && test_codes.contains_key("E0085"),
        "{test_codes:?}"
    );
    assert!(codes_in(&wasm_dumps).contains_key("E0090"));
    let mut found = std::collections::HashSet::new();
    for dumps in [&check, &wasm_dumps, &test_dumps]
        .into_iter()
        .chain(&std_parts)
    {
        for line in dumps.lines() {
            found.extend(nova_driver::resolution_dump::forms_on(line));
        }
    }
    let missing: Vec<&str> = nova_driver::resolution_dump::KINDS
        .iter()
        .copied()
        .filter(|k| !found.contains(*k))
        .collect();
    assert!(missing.is_empty(), "no dump holds {missing:?}");
    let unlisted: Vec<&String> = found
        .iter()
        .filter(|f| !nova_driver::resolution_dump::listed(f))
        .collect();
    assert!(unlisted.is_empty(), "{unlisted:?} are in no list");
    let later: Vec<&str> = nova_driver::resolution_dump::LATER_KINDS
        .iter()
        .copied()
        .filter(|k| found.contains(*k))
        .collect();
    assert!(
        later.is_empty(),
        "{later:?} appear, which LATER_KINDS says no program prints"
    );
}

/// The resolver's cases of every mode as a Windows checkout has them
/// ([`crlf`]): git gives no `.gitattributes` rule for `cases/`, so CI's
/// Windows runner compares them with `\r\n`, `nova.toml`s included, and
/// every system does here. A mode whose copy changes no file, as on a
/// Windows checkout, which already has each `\r\n`, is not compared
/// again: `the_resolution_dumps_agree_on_std_the_corpus_and_the_cases`
/// has compared those bytes (5.2b, ADR 0054 §9).
#[test]
fn the_resolution_dumps_agree_on_the_cases_with_windows_line_endings() {
    let dir = fresh_dir("crlf-resolve");
    let mut crlf_files = 0;
    let mut changed_modes = Vec::new();
    for mode in [Mode::Check, Mode::Wasm, Mode::Test] {
        let from = mode_cases_dir(mode);
        let to = dir.join(mode_cases_name(mode));
        let (held, changed) = copy_crlf(&from, &to);
        crlf_files += held;
        if changed > 0 {
            changed_modes.push(mode);
        }
    }
    assert!(
        crlf_files > 300,
        "only {crlf_files} cases have CRLF line endings"
    );
    std::thread::scope(|scope| {
        let modes: Vec<_> = changed_modes
            .into_iter()
            .map(|mode| {
                let cases = dir.join(mode_cases_name(mode));
                scope.spawn(move || (mode, agree_on_resolve_cases(mode, &cases)))
            })
            .collect();
        for handle in modes {
            let (mode, (dumps, count)) = handle.join().expect("a mode's thread");
            let parts = dumps.lines().filter(|l| l.starts_with("(Program ")).count();
            assert_eq!(parts, count, "{mode:?}");
        }
    });
}

/// Copy every file under `from` to `to`, as a Windows checkout has it
/// ([`crlf`]); returns how many copies hold a `\r`, and how many differ
/// from their files.
fn copy_crlf(from: &Path, to: &Path) -> (usize, usize) {
    let (mut held, mut changed) = (0, 0);
    let mut entries: Vec<_> = std::fs::read_dir(from)
        .expect("read a directory of cases")
        .map(|e| e.expect("a directory entry"))
        .collect();
    entries.sort_by_key(|e| e.file_name());
    std::fs::create_dir_all(to).expect("create a directory");
    for e in entries {
        let target = to.join(e.file_name());
        if e.path().is_dir() {
            let (h, c) = copy_crlf(&e.path(), &target);
            held += h;
            changed += c;
        } else {
            let original = std::fs::read(e.path()).expect("read a case");
            let text = crlf(original.clone());
            held += usize::from(text.contains(&b'\r'));
            changed += usize::from(text != original);
            std::fs::write(&target, text).expect("write a case");
        }
    }
    (held, changed)
}

/// However its root is spelled, both sides form the same paths from it:
/// E0093 names the sibling it refuses by the root's directory, as
/// `Path::parent` gives it, joined with the sibling's file name. On Windows
/// `Path::join` writes `\` where N writes `/`, so there only a root with no
/// directory is compared (ADR 0053 §5). On Linux a file name that is not
/// UTF-8 sits beside the root too, which S0's listing drops and N's keeps
/// with U+FFFD: no import can name it, so the dumps still agree.
#[test]
fn a_root_spelled_any_way_names_its_siblings_alike() {
    let dir = fresh_dir("resolution-spellings");
    let sub = dir.join("sub");
    std::fs::create_dir_all(&sub).expect("create sub");
    std::fs::write(sub.join("main.nova"), "import ui\n\nfn main() {}\n").expect("write main.nova");
    std::fs::write(sub.join("ui.nova"), "pub fn f() {}\n").expect("write ui.nova");
    std::fs::write(sub.join("other.nova"), "import shapes\n\nfn main() {}\n")
        .expect("write other.nova");
    #[cfg(target_os = "linux")]
    {
        use std::os::unix::ffi::OsStrExt;
        let name = std::ffi::OsStr::from_bytes(b"shapes\xff.nova");
        std::fs::write(sub.join(name), "pub fn g() {}\n").expect("write a non-UTF-8 name");
    }
    let mut spellings = vec![("sub", "main.nova".to_string())];
    if !cfg!(windows) {
        for root in [
            "sub/main.nova",
            "./sub/main.nova",
            "sub/./main.nova",
            "sub//main.nova",
            "./sub/.//main.nova",
            "sub/../sub/main.nova",
        ] {
            spellings.push((".", root.to_string()));
        }
        for root in ["./main.nova", ".//main.nova", "../sub/main.nova"] {
            spellings.push(("sub", root.to_string()));
        }
        spellings.push((".", sub.join("main.nova").display().to_string()));
    }
    // The roots of each directory in one process.
    for cwd in [".", "sub"] {
        let roots: Vec<String> = spellings
            .iter()
            .filter(|(c, _)| *c == cwd)
            .map(|(_, root)| root.clone())
            .collect();
        if roots.is_empty() {
            continue;
        }
        let out = agree_on_resolution(&dir.join(cwd), &roots, 0);
        assert_eq!(
            out.matches("(Error \"E0093\"").count(),
            roots.len(),
            "{out}"
        );
    }
    let out = agree_on_resolution(&sub, &["other.nova".to_string()], 0);
    assert!(
        out.contains("(Error \"E0001\" \"cannot find module `shapes`\""),
        "{out}"
    );
}

/// A root with no directory part, and a package's (ADR 0053 §5): the
/// directory both sides list for the root's siblings is then `.`, so a
/// directory named `ui.nova` beside `main.nova` is E0093's, though it
/// cannot be read, and a root directly in a package's `src/` or `tests/`,
/// named from the package's directory, is a module of the package, as one
/// named from outside it is (ADR 0054). An empty root is a file that is
/// not there.
#[test]
fn a_root_with_no_directory_part_or_in_a_package_loads_alike() {
    let dir = fresh_dir("resolution-roots");
    std::fs::write(dir.join("main.nova"), "import ui\n\nfn main() {}\n").expect("write main.nova");
    std::fs::create_dir_all(dir.join("ui.nova")).expect("create ui.nova/");
    let out = agree_on_resolution(&dir, &["main.nova".to_string()], 0);
    assert!(
        out.contains("(Program target=native resolved=false\n"),
        "{out}"
    );
    assert_eq!(out.matches("(Error \"E0093\"").count(), 1, "{out}");
    let pkg = dir.join("pkg");
    for sub in ["src", "tests"] {
        std::fs::create_dir_all(pkg.join(sub)).expect("create a package's directory");
        std::fs::write(pkg.join(sub).join("main.nova"), "fn main() {}\n").expect("write main.nova");
    }
    std::fs::write(pkg.join("nova.toml"), MANIFEST).expect("write nova.toml");
    let roots = ["src/main.nova", "tests/main.nova"].map(String::from);
    let out = agree_on_resolution(&pkg, &roots, 0);
    for (root, label) in [("src/main.nova", "main"), ("tests/main.nova", "tests/main")] {
        assert!(
            out.contains(&format!(
                "== {root}\n(Program target=native resolved=true\n  \
                 (Module 0 \"{label}\" package=0)\n"
            )),
            "{out}"
        );
    }
    let roots = ["pkg/tests/main.nova", ""].map(String::from);
    let out = agree_on_resolution(&dir, &roots, 2);
    assert!(
        out.starts_with(
            "== pkg/tests/main.nova\n(Program target=native resolved=true\n  \
             (Module 0 \"tests/main\" package=0)\n"
        ),
        "{out}"
    );
    assert!(out.ends_with("\n== \nIOERR NotFound\n"), "{out}");
}

/// A root's package is found as S0's `package_of` finds it (ADR 0053 §3,
/// ADR 0054), on both sides: by asking whether the path `<dir>/nova.toml` is a file, or a link to
/// one, not by looking for that exact name in a listing of `<dir>`. So a
/// `Nova.toml` makes a package on a case-insensitive disk (macOS's and
/// Windows' by default) on both sides, and on a case-sensitive one on
/// neither; and, but on Windows, a link to a file makes one, a dangling
/// link none, and a directory that can be searched but not listed still
/// holds its `nova.toml` for both (as root, which lists it anyway, that
/// part is any other package). The test asks the disk whether it folds case.
#[test]
fn a_package_is_found_by_its_manifests_path_on_both_sides() {
    #[cfg(unix)]
    let _ = restore_listing(
        &std::env::temp_dir()
            .join("nova-selfhost-resolution-manifests")
            .join("locked"),
    );
    let dir = fresh_dir("resolution-manifests");
    let package = |name: &str| {
        std::fs::create_dir_all(dir.join(name).join("src")).expect("create a package's src/");
        std::fs::write(
            dir.join(name).join("src").join("main.nova"),
            "fn main() {}\n",
        )
        .expect("write main.nova");
        dir.join(name)
    };
    std::fs::write(package("cased").join("Nova.toml"), MANIFEST).expect("write Nova.toml");
    let folds = dir.join("cased").join("nova.toml").is_file();
    #[cfg(unix)]
    let (more, _relisted) = {
        use std::os::unix::fs::{symlink, PermissionsExt};
        std::fs::write(dir.join("real.toml"), MANIFEST).expect("write real.toml");
        symlink("../real.toml", package("linked").join("nova.toml")).expect("link nova.toml");
        symlink("missing.toml", package("dangling").join("nova.toml")).expect("link nova.toml");
        std::fs::write(package("locked").join("nova.toml"), MANIFEST).expect("write nova.toml");
        let relisted = Relisted(dir.join("locked"));
        std::fs::set_permissions(dir.join("locked"), std::fs::Permissions::from_mode(0o311))
            .expect("make locked/ unlistable");
        (
            vec![("linked", true), ("dangling", false), ("locked", true)],
            relisted,
        )
    };
    #[cfg(not(unix))]
    let more = Vec::new();
    let cases: Vec<(&str, bool)> = [("cased", folds)].into_iter().chain(more).collect();
    let roots: Vec<String> = cases
        .iter()
        .map(|(name, _)| format!("{name}/src/main.nova"))
        .collect();
    let out = agree_on_resolution(&dir, &roots, 0);
    for (root, (_, package)) in roots.iter().zip(&cases) {
        let part = if *package {
            "(Program target=native resolved=true\n  (Module 0 \"main\" package=0)\n"
        } else {
            "(Program target=native resolved=true\n  (Module 0 \"main\" package=None)\n"
        };
        assert!(
            out.contains(&format!("== {root}\n{part}")),
            "{root} (a case-folding disk: {folds}): {out}"
        );
    }
}

/// A directory made listable again when dropped, so that `fresh_dir` can
/// remove it next time, whether the test that locked it passed or failed.
#[cfg(unix)]
struct Relisted(PathBuf);

#[cfg(unix)]
impl Drop for Relisted {
    fn drop(&mut self) {
        let _ = restore_listing(&self.0);
    }
}

#[cfg(unix)]
fn restore_listing(dir: &Path) -> std::io::Result<()> {
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(dir, std::fs::Permissions::from_mode(0o755))
}

/// A module's label is its file's stem, which may hold what the dump's
/// strings escape: a root named `q"uo\te.nova` is the module
/// `"q\x22uo\x5cte"` on both sides. Windows refuses `"` in a file name.
#[cfg(not(windows))]
#[test]
fn a_root_whose_name_needs_escapes_is_printed_alike() {
    let dir = fresh_dir("resolution-escapes");
    let root = "q\"uo\\te.nova".to_string();
    std::fs::write(dir.join(&root), "fn main() {}\n").expect("write the root");
    let out = agree_on_resolution(&dir, std::slice::from_ref(&root), 0);
    assert!(
        out.contains("(Module 0 \"q\\x22uo\\x5cte\" package=None)"),
        "{out}"
    );
}

/// A root whose path is not UTF-8 (ADR 0053 §5): S0 dumps its program,
/// while N, whose arguments arrive as UTF-8 with U+FFFD in place of what
/// is not, cannot open it and prints `IOERR NotFound`, exiting 2, as for
/// 5.1's dumps (ADR 0051). No input of the differential has such a path;
/// this pins the difference. Linux only: macOS and Windows refuse such a
/// name.
#[cfg(target_os = "linux")]
#[test]
fn a_root_whose_path_is_not_utf8_is_dumped_by_s0_and_not_found_by_n() {
    use std::ffi::OsStr;
    use std::os::unix::ffi::OsStrExt;
    let dir = fresh_dir("resolution-non-utf8");
    std::fs::create_dir(dir.join(OsStr::from_bytes(b"sub\xfe"))).expect("create sub");
    for root in [&b"r\xff.nova"[..], b"sub\xfe/main.nova"] {
        let root = OsStr::from_bytes(root);
        std::fs::write(dir.join(root), "fn main() {}\n").expect("write the root");
        let s0 = Command::new(nova())
            .current_dir(&dir)
            .args(RESOLUTION_S0)
            .arg(root)
            .output()
            .expect("run nova check --dump-resolution");
        let stdout = String::from_utf8_lossy(&s0.stdout);
        assert_eq!(s0.status.code(), Some(0), "{root:?}: {stdout}");
        assert!(
            stdout.contains("\n(Program target=native resolved=true\n"),
            "{root:?}: {stdout}"
        );
        let n = Command::new(selfhost())
            .current_dir(&dir)
            .args(resolution().n)
            .arg(root)
            .output()
            .expect("run selfhost resolve");
        let stdout = String::from_utf8_lossy(&n.stdout);
        assert_eq!(n.status.code(), Some(2), "{root:?}: {stdout}");
        assert!(stdout.ends_with("\nIOERR NotFound\n"), "{root:?}: {stdout}");
    }
}

/// The name fuzzer's fixed seeds and counts every run checks (spec §3.5),
/// and, from 5.2b (spec §4.5, ADR 0054), its WASM and package siblings'.
/// The loose inputs are also resolved under test mode, which keeps their
/// `@test` functions; the package's under `check`, each in a package whose
/// manifests are one of four forms ([`fuzz::package_manifests`]), and the
/// first seed's also under test mode and the second's for the WASM target,
/// in the process of that mode's loose inputs, which keeps the fuzzer's
/// time in proportion (ADR 0054).
const NAME_SEEDS: [u64; 3] = [301, 302, 303];
const NAME_COUNT: usize = 50;
const WASM_NAME_SEEDS: [u64; 3] = [311, 312, 313];
const WASM_NAME_COUNT: usize = 20;
const PACKAGE_NAME_SEEDS: [u64; 3] = [321, 322, 323];
const PACKAGE_NAME_COUNT: usize = 30;

/// The resolver's 11 codes (spec §2). `check` gives a loose native program
/// 8 of them; E0084 and E0085 are a `@test` function's, which only test
/// mode keeps, and E0090 is the WASM target's (decision 8).
const RESOLVER_CODES: [&str; 11] = [
    "E0001", "E0002", "E0082", "E0083", "E0084", "E0085", "E0089", "E0090", "E0092", "E0102",
    "E0900",
];
const TEST_MODE_CODES: [&str; 2] = ["E0084", "E0085"];
const WASM_CODES: [&str; 1] = ["E0090"];

/// The two notes the loader puts on E0001 in a package (spec §4.1): a
/// package with no library imported from its own `tests/`, and a
/// dev-dependency imported from `src/`.
const E0001_NOTES: [&str; 2] = [
    " has no src/lib.nova\")",
    " is a dev-dependency, which only `tests/` files can import\")",
];

/// `count` name-mutated programs from `seed` ([`fuzz::name_mutate`]), each
/// one of `files`, under `root`, that the seed picks and that parses. As
/// for [`mutations_of`], the files are picked from their sorted list and
/// read with each `\r\n` as `\n`, so a seed gives the same inputs on every
/// system. Input `i` is written as `f{i:05}.nova`, which a self-import
/// names.
fn names_of(root: &Path, files: &[String], seed: u64, count: usize) -> Vec<String> {
    names_of_with(root, files, seed, count, fuzz::name_mutate)
}

/// [`names_of`], each input made by `mutate`, which takes the rng, the
/// source and the input's stem.
fn names_of_with<T>(
    root: &Path,
    files: &[String],
    seed: u64,
    count: usize,
    mutate: impl Fn(&mut SplitMix64, &str, &str) -> T,
) -> Vec<T> {
    let mut files = files.to_vec();
    files.sort();
    let mut rng = SplitMix64::new(seed);
    (0..count)
        .map(|i| loop {
            let file = rng.pick(&files);
            let source = std::fs::read_to_string(root.join(file))
                .expect("read a corpus file")
                .replace("\r\n", "\n");
            if fuzz::parses(&source) {
                break mutate(&mut rng, &source, &format!("f{i:05}"));
            }
        })
        .collect()
}

/// The name-mutated loose corpus: [`names_of`] it.
fn names(seed: u64, count: usize) -> Vec<String> {
    names_of(&root(), &loose_corpus(), seed, count)
}

/// The loose WASM programs of the corpus, which [`wasm_names`] mutates.
fn loose_wasm_corpus() -> Vec<String> {
    wasm_corpus()
        .into_iter()
        .filter(|f| !in_package(f))
        .collect()
}

/// The name-mutated loose WASM programs ([`fuzz::wasm_name_mutate`]).
fn wasm_names_of(root: &Path, files: &[String], seed: u64, count: usize) -> Vec<String> {
    names_of_with(root, files, seed, count, fuzz::wasm_name_mutate)
}

/// The name-mutated loose corpus as modules of [`fuzz::PACKAGE_FILES`]'
/// package ([`fuzz::package_name_mutate`]): whether each is in `tests/`,
/// and its text.
fn package_names_of(root: &Path, files: &[String], seed: u64, count: usize) -> Vec<(bool, String)> {
    names_of_with(root, files, seed, count, fuzz::package_name_mutate)
}

/// Write `inputs` to a fixed directory named after `name`, as
/// `f00000.nova` and on, beside [`fuzz::NAME_SIBLINGS`]: the directory, and
/// the inputs' files, each a program's root.
fn write_name_inputs(name: &str, inputs: &[String]) -> (PathBuf, Vec<String>) {
    let dir = fresh_dir(name);
    let files = write_name_inputs_in(&dir, inputs);
    (dir, files)
}

/// [`write_name_inputs`] into `dir`, which exists: the inputs' files.
fn write_name_inputs_in(dir: &Path, inputs: &[String]) -> Vec<String> {
    for (stem, source) in fuzz::NAME_SIBLINGS {
        std::fs::write(dir.join(format!("{stem}.nova")), source).expect("write a sibling");
    }
    let mut files = Vec::with_capacity(inputs.len());
    for (i, input) in inputs.iter().enumerate() {
        let file = format!("f{i:05}.nova");
        std::fs::write(dir.join(&file), input).expect("write a name-fuzz input");
        files.push(file);
    }
    files
}

/// Each seed's inputs, [`write_name_inputs`] into a directory named for
/// the seed in a fixed directory named after `name`: that directory, and
/// every seed's roots, relative to it (`301/f00000.nova`), so that one
/// process a side resolves all the seeds' inputs ([`seed_parts`]).
fn write_seeded_inputs(name: &str, seeded: &[(u64, Vec<String>)]) -> (PathBuf, Vec<String>) {
    let dir = fresh_dir(name);
    let mut roots = Vec::new();
    for (seed, inputs) in seeded {
        let sub = dir.join(seed.to_string());
        std::fs::create_dir_all(&sub).expect("create a seed's directory");
        let files = write_name_inputs_in(&sub, inputs);
        roots.extend(files.into_iter().map(|f| format!("{seed}/{f}")));
    }
    (dir, roots)
}

/// The parts of `dump`, a resolution dump of [`write_seeded_inputs`]'
/// roots, whose roots are `seed`'s, each from its `== ` header on.
fn seed_parts(dump: &str, seed: u64) -> String {
    parts_under(dump, &seed.to_string())
}

/// The parts of `dump` whose roots are in the directory `dir`, each from
/// its `== ` header on.
fn parts_under(dump: &str, dir: &str) -> String {
    let header = format!("== {dir}/");
    let mut out = String::new();
    let mut keep = false;
    for line in dump.split_inclusive('\n') {
        if line.starts_with("== ") {
            keep = line.starts_with(&header);
        }
        if keep {
            out.push_str(line);
        }
    }
    out
}

/// Write [`fuzz::PACKAGE_FILES`] to `dir`, which exists, once for each
/// form of [`fuzz::package_manifests`], `v0/` to `v3/`, each with its
/// manifests in that form and [`fuzz::NAME_SIBLINGS`] in its `app/src/` and
/// `app/tests/`, and input `i` as `f{i:05}.nova` in the `app/tests/` or
/// `app/src/` of form `i % 4`: the inputs' roots, relative to `dir`.
fn write_package_inputs_in(dir: &Path, inputs: &[(bool, String)]) -> Vec<String> {
    let forms = fuzz::package_manifests();
    // Form 3's manifests end every line with `\r\n`, which no other input
    // reaching N's manifest reader through the fuzzer does.
    for text in [&forms[3].0, &forms[3].1] {
        assert_eq!(text.matches("\r\n").count(), text.matches('\n').count());
        assert!(text.contains("\r\n"), "{text:?}");
    }
    for (k, (app_manifest, dep_manifest)) in forms.iter().enumerate() {
        let form = dir.join(format!("v{k}"));
        for (path, text) in fuzz::PACKAGE_FILES {
            let text = match path {
                "app/nova.toml" => app_manifest.as_str(),
                "dep/nova.toml" => dep_manifest.as_str(),
                _ => text,
            };
            let path = form.join(path);
            std::fs::create_dir_all(path.parent().expect("a parent")).expect("create a directory");
            std::fs::write(path, text).expect("write a package's file");
        }
        for sub in ["src", "tests"] {
            let app = form.join("app").join(sub);
            std::fs::create_dir_all(&app).expect("create a package's directory");
            for (stem, source) in fuzz::NAME_SIBLINGS {
                std::fs::write(app.join(format!("{stem}.nova")), source).expect("write a sibling");
            }
        }
    }
    let mut roots = Vec::with_capacity(inputs.len());
    for (i, (tests, input)) in inputs.iter().enumerate() {
        let sub = if *tests { "tests" } else { "src" };
        let root = format!("v{}/app/{sub}/f{i:05}.nova", i % forms.len());
        std::fs::write(dir.join(&root), input).expect("write a name-fuzz input");
        roots.push(root);
    }
    roots
}

/// How many diagnostics of each code `dump`, a resolution dump, holds.
fn codes_in(dump: &str) -> std::collections::BTreeMap<String, usize> {
    let mut codes = std::collections::BTreeMap::new();
    for line in dump.lines() {
        for kind in ["  (Error \"", "  (Warning \""] {
            if let Some(rest) = line.strip_prefix(kind) {
                let code = rest.split('"').next().unwrap_or("").to_string();
                *codes.entry(code).or_insert(0) += 1;
            }
        }
    }
    codes
}

/// What each of `handles` gave, in order: a thread that panicked fails the
/// test with its panic.
fn joined<T>(handles: Vec<std::thread::ScopedJoinHandle<'_, T>>) -> Vec<T> {
    handles
        .into_iter()
        .map(|h| h.join().unwrap_or_else(|e| std::panic::resume_unwind(e)))
        .collect()
}

/// Each program's part of `dump`, by its root's header.
fn parts_of(dump: &str) -> Vec<&str> {
    dump.split("== ").filter(|p| !p.is_empty()).collect()
}

/// Both sides give the same resolution dump of each of the name fuzzer's
/// inputs from the fixed seeds, and every input reaches the resolver; and,
/// from 5.2b, its inputs under test mode, its WASM sibling's for the WASM
/// target and its package sibling's, with the package's manifests in four
/// forms, one seed's also under test mode and another's for the WASM
/// target. A package's input reaches the resolver, or the loader stops it
/// with E0004 or a noted E0001, and E0004 names its entry on more than one
/// line of a manifest, as the forms place it. Among
/// them every one of the resolver's 11 codes, E0004 and E0001's two notes
/// occur twice or more; E0084 and E0085 only under test mode, E0090 only
/// for the WASM target. The loose inputs of every seed are written to
/// `names/<seed>/` and the WASM ones to `wasm-names/<seed>/`, each set
/// resolved in one process a side and mode ([`write_seeded_inputs`]).
#[test]
fn the_name_fuzzer_agrees_and_gives_every_resolver_code() {
    let root = root();
    let mut all = std::collections::BTreeMap::new();
    let mut add = |dump: &str, mode: Mode, what: &str| {
        let codes = codes_in(dump);
        let absent: &[&str] = match mode {
            Mode::Check => &["E0084", "E0085", "E0090"],
            Mode::Wasm => &TEST_MODE_CODES,
            Mode::Test => &WASM_CODES,
        };
        for code in absent {
            assert!(!codes.contains_key(*code), "{what}: {code} in {codes:?}");
        }
        for (code, n) in codes {
            *all.entry(code).or_insert(0) += n;
        }
        for note in E0001_NOTES {
            *all.entry(note.to_string()).or_insert(0) += dump.matches(note).count();
        }
    };
    let loose = loose_corpus();
    let loose_wasm = loose_wasm_corpus();
    // Every seed's loose inputs in one process a side under `check`, again
    // under test mode, and the WASM inputs in one, each mode in its own
    // thread, as each seed's package inputs, which run from inside their
    // own package; then counted in order, seed by seed.
    let names_inputs: Vec<(u64, Vec<String>)> = NAME_SEEDS
        .iter()
        .map(|&seed| (seed, names(seed, NAME_COUNT)))
        .collect();
    let (names_dir, names_roots) = write_seeded_inputs("names", &names_inputs);
    let wasm_inputs: Vec<(u64, Vec<String>)> = WASM_NAME_SEEDS
        .iter()
        .map(|&seed| {
            let inputs = wasm_names_of(&root, &loose_wasm, seed, WASM_NAME_COUNT);
            (seed, inputs)
        })
        .collect();
    let (wasm_dir, mut wasm_roots) = write_seeded_inputs("wasm-names", &wasm_inputs);
    let package_inputs: Vec<(u64, Vec<(bool, String)>)> = PACKAGE_NAME_SEEDS
        .iter()
        .map(|&seed| {
            (
                seed,
                package_names_of(&root, &loose, seed, PACKAGE_NAME_COUNT),
            )
        })
        .collect();
    // One seed's package inputs under test mode, and another's for the WASM
    // target, in the process of the loose inputs of that mode.
    let mut test_roots = names_roots.clone();
    for ((seed, inputs), (dir, roots)) in package_inputs[..2]
        .iter()
        .zip([(&names_dir, &mut test_roots), (&wasm_dir, &mut wasm_roots)])
    {
        let sub = format!("package-{seed}");
        std::fs::create_dir_all(dir.join(&sub)).expect("create a package seed's directory");
        let found = write_package_inputs_in(&dir.join(&sub), inputs);
        roots.extend(found.into_iter().map(|r| format!("{sub}/{r}")));
    }
    let (check, test, wasm, package_dumps) = std::thread::scope(|scope| {
        let test = scope.spawn(|| agree_on_resolution_in(Mode::Test, &names_dir, &test_roots, 0));
        let wasm = scope.spawn(|| agree_on_resolution_in(Mode::Wasm, &wasm_dir, &wasm_roots, 0));
        let packages: Vec<_> = package_inputs
            .iter()
            .map(|(seed, inputs)| {
                let dir = fresh_dir(&format!("package-names-{seed}"));
                let roots = write_package_inputs_in(&dir, inputs);
                scope.spawn(move || (*seed, inputs.len(), agree_on_resolution(&dir, &roots, 0)))
            })
            .collect();
        let check = agree_on_resolution(&names_dir, &names_roots, 0);
        let [test, wasm] =
            [test, wasm].map(|h| h.join().unwrap_or_else(|e| std::panic::resume_unwind(e)));
        (check, test, wasm, joined(packages))
    });
    for (seed, inputs) in &names_inputs {
        let count = inputs.len();
        let dump = seed_parts(&check, *seed);
        let resolved = dump.matches("resolved=true").count();
        assert_eq!(
            resolved, count,
            "seed {seed}: an input did not reach the resolver"
        );
        add(&dump, Mode::Check, &format!("seed {seed}"));
        let test = seed_parts(&test, *seed);
        assert_eq!(test.matches("resolved=true").count(), count);
        add(&test, Mode::Test, &format!("seed {seed} under test mode"));
    }
    for (seed, inputs) in &wasm_inputs {
        let dump = seed_parts(&wasm, *seed);
        assert_eq!(dump.matches("resolved=true").count(), inputs.len());
        add(&dump, Mode::Wasm, &format!("WASM seed {seed}"));
    }
    let under_modes = [
        (
            Mode::Test,
            parts_under(&test, &format!("package-{}", package_inputs[0].0)),
        ),
        (
            Mode::Wasm,
            parts_under(&wasm, &format!("package-{}", package_inputs[1].0)),
        ),
    ];
    let package_parts = package_dumps
        .into_iter()
        .map(|(seed, count, dump)| (Mode::Check, seed, count, dump))
        .chain(
            under_modes
                .into_iter()
                .zip(&package_inputs)
                .map(|((mode, dump), (seed, inputs))| (mode, *seed, inputs.len(), dump)),
        );
    // The line of each E0004's entry in a manifest, which the manifests'
    // forms put on different lines.
    let mut entry_lines = std::collections::BTreeSet::new();
    for (mode, seed, count, dump) in package_parts {
        for at in dump.split("and the entry at ").skip(1) {
            let entry = at.split('"').next().unwrap_or("");
            entry_lines.insert(entry.rsplit(':').next().unwrap_or("").to_string());
        }
        let parts = parts_of(&dump);
        assert_eq!(parts.len(), count);
        for part in parts {
            let stopped_by_the_loader = part.contains("\n  (Error \"E0004\"")
                || E0001_NOTES.iter().any(|n| part.contains(n));
            assert!(
                part.contains(" resolved=true\n") || stopped_by_the_loader,
                "package seed {seed}: an input did not reach the resolver: {part}"
            );
            assert!(part.contains(" package=0"), "{part}");
        }
        add(
            &dump,
            mode,
            &format!("package seed {seed} ({})", mode_cases_name(mode)),
        );
    }
    assert!(
        entry_lines.len() >= 2,
        "E0004's entries all on one line of a manifest: {entry_lines:?}"
    );
    eprintln!(
        "codes over seeds {NAME_SEEDS:?}, {WASM_NAME_SEEDS:?} and {PACKAGE_NAME_SEEDS:?}: {all:?}"
    );
    for code in RESOLVER_CODES.iter().chain(&["E0004"]).chain(&E0001_NOTES) {
        let n = all.get(*code).copied().unwrap_or(0);
        assert!(n >= 2, "{code} occurs {n} times, under 2, in {all:?}");
    }
}

// === 5.3a: the HIR dump (spec 5.3 §3.6, ADR 0055) ===
//
// Part A builds S0's side: the hidden `check --dump-hir`, its cases
// (`cases/check/`, `cases/check-wasm/`, `cases/check-test/`) and the type
// fuzzer's item mutations, each run through [`run`]. Until N's `selfhost
// hir` exists ([`N_HIR`]), every set runs through S0 alone and its
// assertions hold on S0's dump; part B ports N's side, sets `N_HIR`, and
// every set below is then compared byte for byte, exit codes included,
// through [`compare_batch`].

/// Whether N has `selfhost hir --std DIR [--target wasm] [--test] FILE…`
/// (or `--std-part`), the HIR dump's other side (spec 5.3 §3.3): part B of
/// 5.3a sets it.
const N_HIR: bool = true;

// From part B on, N's side is compared: the HIR dump's checks hold on
// both sides, never on S0's alone.
const _: () = assert!(N_HIR, "5.3a's part B compares N's side");

/// Whether N's `selfhost hir` checks bodies and prints every function
/// (5.3b, spec 5.3 §4.3), as S0's `check --dump-hir` does. Until it does,
/// N is compared with S0's item part alone, `check --dump-hir-items`
/// ([`hir_s0`]), 5.3a's checks hold as they did, and the full dump's
/// inputs (std's three parts, the corpus and the cases, with both line
/// endings) run through S0 alone, whose dumps must be well formed and
/// hold every form and every code. 5.3b's later parts port N's bodies and
/// set it, and every set is then compared byte for byte.
const N_BODIES: bool = false;

/// S0's side of the HIR dump that N's `hir` prints: the full dump once N
/// checks bodies ([`N_BODIES`]), its item part before.
fn hir_s0() -> &'static [&'static str] {
    if N_BODIES {
        HIR_S0
    } else {
        HIR_ITEMS_S0
    }
}

/// N's side of [`hir_s0`]: `hir`, or `hir --items` for the item part
/// alone ([`n_hir_for`]).
fn n_hir() -> &'static [&'static str] {
    n_hir_for(hir_s0())
}

/// N's command for S0's HIR dump `s0` (5.3b): `hir --items` for S0's
/// `check --dump-hir-items`, 5.3a's item part, and `hir` for the full
/// dump, which N prints from 5.3b's part 2 on.
fn n_hir_for(s0: &[&str]) -> &'static [&'static str] {
    if s0 == HIR_ITEMS_S0 {
        &["hir", "--items"]
    } else {
        &["hir"]
    }
}

/// The item codes of 5.3a (spec 5.3 §3.2): each occurs among the HIR
/// dumps of the cases, the corpus and the fuzzer's inputs. E0092 is the
/// WASM target's.
const ITEM_CODES: [&str; 16] = [
    "E0001", "E0010", "E0012", "E0015", "E0070", "E0071", "E0072", "E0073", "E0074", "E0076",
    "E0077", "E0078", "E0092", "E0105", "E0403", "E0900",
];

/// The type checker's cases under `mode` (5.3a): `cases/check/` under
/// `check`, `cases/check-wasm/` for the WASM target and `cases/check-test/`
/// under test mode, laid out as the resolver's ([`case_roots`]).
fn hir_cases_name(mode: Mode) -> &'static str {
    match mode {
        Mode::Check => "check",
        Mode::Wasm => "check-wasm",
        Mode::Test => "check-test",
    }
}

fn hir_cases_dir(mode: Mode) -> PathBuf {
    cases_dir().join(hir_cases_name(mode))
}

/// The HIR dump through both sides under `mode`: S0's `check --dump-hir`
/// and N's `hir --std DIR`, each with the mode's flags and followed by the
/// roots.
fn hir_in(mode: Mode) -> &'static Dump {
    static DUMPS: [OnceLock<Dump>; 3] = [OnceLock::new(), OnceLock::new(), OnceLock::new()];
    DUMPS[mode as usize].get_or_init(|| hir_dump_with(mode, &[]))
}

/// std's part of the HIR dump under `mode`: S0's `check --dump-hir --std`,
/// N's `hir --std DIR --std-part`.
fn std_hir(mode: Mode) -> &'static Dump {
    static DUMPS: [OnceLock<Dump>; 3] = [OnceLock::new(), OnceLock::new(), OnceLock::new()];
    DUMPS[mode as usize].get_or_init(|| hir_dump_with(mode, &["--std"]))
}

/// [`dump_with`] for the HIR dump.
fn hir_dump_with(mode: Mode, extra: &[&str]) -> Dump {
    hir_dump_from(hir_s0(), mode, extra)
}

/// [`hir_dump_with`], S0's side `s0` (the full dump or its item part).
fn hir_dump_from(s0: &[&str], mode: Mode, extra: &[&str]) -> Dump {
    let command = n_hir_for(s0);
    let s0: Vec<&str> = s0
        .iter()
        .chain(mode.flags())
        .chain(extra)
        .copied()
        .collect();
    let std_dir = std_dir();
    let mut n = command.to_vec();
    n.extend(["--std", std_dir.as_str()]);
    n.extend(mode.flags());
    n.extend(
        extra
            .iter()
            .map(|a| if *a == "--std" { "--std-part" } else { a }),
    );
    Dump {
        s0: leak_args(&s0),
        n: leak_args(&n),
    }
}

/// `dump` on `files`, relative to `dir`, in one process a side, each side
/// exiting with `code`: compared through [`compare_batch`] once N has the
/// HIR dump, and through S0 alone ([`run`]) before. Returns S0's output.
fn agree_on_hir_dump(dump: &Dump, dir: &Path, files: &[String], code: i32) -> String {
    if N_HIR {
        return compare_batch(dump, dir, files, code, &nova(), selfhost())
            .unwrap_or_else(|report| panic!("{report}"));
    }
    let s0 = run(&nova(), dump.s0, dir, files).unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(
        s0.code,
        Some(code),
        "{}: {}",
        batch_name(&nova(), dump.s0, files),
        s0.stderr
    );
    String::from_utf8(s0.stdout).expect("the HIR dump is UTF-8")
}

/// [`agree_on_hir_dump`] the HIR dump under `mode` on `files`, each a
/// program's root.
fn agree_on_hir_in(mode: Mode, dir: &Path, files: &[String], code: i32) -> String {
    if files.is_empty() {
        return String::new();
    }
    agree_on_hir_dump(hir_in(mode), dir, files, code)
}

/// [`agree_on_resolve_cases`] for the HIR dump: the loose programs and
/// the packages' roots of `cases` in one process a side, then each program
/// apart from its own directory. Returns S0's output and how many programs
/// it holds.
fn agree_on_hir_cases(mode: Mode, cases: &Path) -> (String, usize) {
    let roots = case_roots(cases);
    let count = roots.loose.len() + roots.apart.len() + roots.packages.len();
    let from_cases: Vec<String> = roots.loose.into_iter().chain(roots.packages).collect();
    let mut all = agree_on_hir_in(mode, cases, &from_cases, 0);
    for dir in &roots.apart {
        all.push_str(&agree_on_hir_in(mode, dir, &["main.nova".to_string()], 0));
    }
    (all, count)
}

/// The forms `dumps` print, as the resolution dump's [`forms_on`] reads
/// them ([`nova_driver::resolution_dump::forms_on`]).
fn hir_forms(dumps: &[&str]) -> std::collections::HashSet<String> {
    let mut found = std::collections::HashSet::new();
    for dump in dumps {
        for line in dump.lines() {
            found.extend(nova_driver::resolution_dump::forms_on(line));
        }
    }
    found
}

/// `nova check`'s help names no `--dump-hir`, every `nova check` command
/// line whose first argument after `check` is neither hidden flag is
/// parsed as the base parses it (the resolution dump's test holds the
/// rest), and the HIR dump's own parser refuses what the resolution dump's
/// refuses, under its own usage line (5.3a, ADR 0055).
#[test]
fn the_hir_dump_flag_is_hidden_and_refused_as_the_resolution_dumps() {
    let help = Command::new(nova())
        .args(["check", "--help"])
        .output()
        .expect("run nova check --help");
    let help = String::from_utf8_lossy(&help.stdout).into_owned();
    assert!(help.contains("--target"), "{help}");
    assert!(!help.contains("dump-hir"), "{help}");
    let out = Command::new(nova())
        .args(["check", "--help", "--dump-hir"])
        .output()
        .expect("run nova check --help --dump-hir");
    assert_eq!(out.status.code(), Some(0));
    assert_eq!(String::from_utf8_lossy(&out.stdout), help);
    let (opts, file, target) = ("[OPTIONS] [FILE]", "<FILE>", "--target <TARGET> [FILE]");
    for (args, named, usage) in [
        (&["check", "--dump-hi", "a.nova"][..], "--dump-hi", opts),
        (&["check", "--dump-hirr", "a.nova"], "--dump-hirr", opts),
        (&["check", "--dump-hir=1", "a.nova"], "--dump-hir", opts),
        (&["check", "--dump-hir=", "a.nova"], "--dump-hir", opts),
        (&["check", "a.nova", "--dump-hir"], "--dump-hir", file),
        (
            &["check", "--target", "native", "--dump-hir", "a.nova"],
            "--dump-hir",
            target,
        ),
        (&["check", "--test", "--dump-hir", "a.nova"], "--test", opts),
    ] {
        let out = Command::new(nova())
            .args(args)
            .output()
            .expect("run nova check");
        let stderr = String::from_utf8_lossy(&out.stderr);
        let tip = format!("  tip: to pass '{named}' as a value, use '-- {named}'\n\n");
        assert_eq!(out.status.code(), Some(2), "{args:?}: {stderr}");
        assert_eq!(
            stderr,
            format!(
                "error: unexpected argument '{named}' found\n\n{tip}\
                 Usage: nova{EXE_SUFFIX} check {usage}\n\n\
                 For more information, try '--help'.\n"
            ),
            "{args:?}"
        );
        assert!(out.stdout.is_empty(), "{args:?}");
    }
    for args in [
        &["--stdd"][..],
        &["--std", "--std"],
        &["--target", "bogus", "a.nova"],
        &["--target=", "a.nova"],
        &["--help"],
        &["-h"],
        &["--dump-hir", "a.nova"],
        &["--dump-resolution", "a.nova"],
        &["--test", "--test", "a.nova"],
        &["--test=1", "a.nova"],
        &["--tests", "a.nova"],
        &["--test", "--target", "wasm", "a.nova"],
        &["--target=wasm", "--test", "--std"],
        &["--std", "a.nova"],
        &["--test", "--std", "a.nova"],
        &["--test"],
        &[],
    ] {
        // 5.3b's item part alone, `--dump-hir-items`, refuses as the full
        // dump does, under its own usage line.
        for form in [HIR_S0, HIR_ITEMS_S0] {
            let flag = form[1];
            let out = Command::new(nova())
                .args(form)
                .args(args)
                .output()
                .expect("run nova check --dump-hir");
            let stderr = String::from_utf8_lossy(&out.stderr);
            assert_eq!(out.status.code(), Some(2), "{args:?}: {stderr}");
            assert!(stderr.starts_with("error: "), "{args:?}: {stderr}");
            assert_eq!(stderr.matches("error:").count(), 1, "{args:?}: {stderr}");
            assert_eq!(
                stderr.matches("Usage: ").count(),
                stderr
                    .matches(&format!("Usage: nova check {flag} "))
                    .count(),
                "{args:?}: {stderr}"
            );
            // The form's own refusals, which are not clap's, name its flag.
            if !stderr.contains("Usage: ") && !stderr.contains("[possible values: ") {
                assert!(
                    stderr.starts_with(&format!("error: {flag} ")),
                    "{args:?}: {stderr}"
                );
            }
            assert!(out.stdout.is_empty(), "{args:?}");
        }
    }
}

/// Each root's part follows a header with its path as given; a root that
/// cannot be read is `IOERR` and its kind, the rest are dumped, and the
/// exit code is 2; a program that stops before the type checker prints
/// `checked=false` and the diagnostics so far; a package's root is a module
/// of its package; `--std` prints std's part for the target and mode, the
/// same as std's part of any program, and a program's part holds only its
/// own entries (5.3a).
#[test]
fn the_hir_dump_has_a_header_per_root_and_std_its_own_part() {
    let dir = fresh_dir("hir-headers");
    std::fs::write(dir.join("a.nova"), "record P { x: Int }\nfn main() {}\n")
        .expect("write a.nova");
    std::fs::write(dir.join("b.nova"), "fn g( {").expect("write b.nova");
    std::fs::write(dir.join("c.nova"), "fn f(x: Nope) {}\nfn f() {}\n").expect("write c.nova");
    std::fs::write(dir.join("latin1.nova"), b"fn \xe9() {}\n").expect("write latin1.nova");
    std::fs::create_dir_all(dir.join("pkg").join("src")).expect("create pkg/src");
    std::fs::write(dir.join("pkg").join("nova.toml"), MANIFEST).expect("write nova.toml");
    std::fs::write(
        dir.join("pkg").join("src").join("main.nova"),
        "import shapes\nfn main() {}\n",
    )
    .expect("write main.nova");
    std::fs::write(
        dir.join("pkg").join("src").join("shapes.nova"),
        "pub type Shape =\n  | Dot\n",
    )
    .expect("write shapes.nova");
    let files: Vec<String> = [
        "a.nova",
        "missing.nova",
        "b.nova",
        "c.nova",
        "latin1.nova",
        "pkg/src/main.nova",
    ]
    .iter()
    .map(|f| f.to_string())
    .collect();
    let out = agree_on_hir_in(Mode::Check, &dir, &files, 2);
    assert!(
        out.contains("== latin1.nova\nIOERR InvalidData\n== pkg/src/main.nova\n"),
        "{out}"
    );
    let headers: Vec<&str> = out.lines().filter(|l| l.starts_with("== ")).collect();
    assert_eq!(
        headers,
        files.iter().map(|f| format!("== {f}")).collect::<Vec<_>>()
    );
    assert!(
        out.contains("== missing.nova\nIOERR NotFound\n== b.nova\n"),
        "{out}"
    );
    assert!(
        out.contains("== a.nova\n(Hir target=native checked=true\n  (StdStart module=1 def=2 item=2)\n  (Record u0 \"P\" generics=0\n    (Field \"x\" Int))\n  (Sig u1 generics=0\n    (Ret ())))\n(Diagnostics)\n"),
        "{out}"
    );
    assert!(
        out.contains("== b.nova\n(Hir target=native checked=false)\n(Diagnostics\n  (Error \"P"),
        "{out}"
    );
    // E0002 is the resolver's: the type checker is not reached.
    assert!(
        out.contains(
            "== c.nova\n(Hir target=native checked=false)\n(Diagnostics\n  (Error \"E0002\""
        ),
        "{out}"
    );
    assert!(
        out.contains(
            "== pkg/src/main.nova\n(Hir target=native checked=true\n  (StdStart module=2 "
        ),
        "{out}"
    );
    assert!(
        out.contains("  (Sum u1 \"Shape\" generics=0\n    (Variant \"Dot\"))"),
        "{out}"
    );
    for (mode, head) in [
        (Mode::Check, "(HirStd target=native records="),
        (Mode::Wasm, "(HirStd target=wasm records="),
        (Mode::Test, "(HirStd target=native mode=test records="),
    ] {
        let std = agree_on_hir_dump(std_hir(mode), &dir, &[], 0);
        assert!(std.starts_with(head), "{}", &std[..80]);
        assert!(std.ends_with(")\n(Diagnostics)\n"), "{mode:?}");
        assert!(
            !std.contains(" u"),
            "{mode:?}: a program's id in std's part"
        );
    }
}

/// The HIR dump's settling: S0's `check --dump-hir` and N's `hir` read `\`
/// as `/` in E0004's and E0093's paths on Windows, as the resolution
/// dump's are (5.3a, spec 5.3 §3.6).
#[test]
fn every_run_of_a_hir_dump_settles_e0004s_paths_on_windows() {
    let windows = b"(Diagnostics\n  (Error \"E0004\" \"`a` is both: app\\x5csrc\\x5ca.nova, and the entry at app\\x5cnova.toml:6\"\n".to_vec();
    let settled = settled_stdout(RESOLUTION_S0, windows.clone(), true);
    assert_ne!(settled, windows);
    let hir_args: Vec<&str> = HIR_S0.iter().chain(["a.nova"].iter()).copied().collect();
    assert_eq!(settled_stdout(&hir_args, windows.clone(), true), settled);
    let items_args: Vec<&str> = HIR_ITEMS_S0
        .iter()
        .chain(["a.nova"].iter())
        .copied()
        .collect();
    assert_eq!(settled_stdout(&items_args, windows.clone(), true), settled);
    assert_eq!(
        settled_stdout(&["hir", "--std", "std"], windows.clone(), true),
        settled
    );
    assert_eq!(settled_stdout(&hir_args, windows.clone(), false), windows);
}

/// std's three parts, every corpus root (natively, for the WASM target
/// and under test mode) and the cases of each mode, through the HIR dump
/// (5.3a, spec 5.3 §3.6): each root's part is there, every form the dump
/// prints is listed and every listed form but [`nova_driver::hir_dump::LATER_KINDS`]
/// occurs, and each of the 16 item codes occurs, E0092 only for the WASM
/// target.
#[test]
fn the_hir_dumps_agree_on_std_the_corpus_and_the_cases() {
    let root = root();
    let std_parts: Vec<String> = std::thread::scope(|scope| {
        let parts: Vec<_> = [Mode::Check, Mode::Wasm, Mode::Test]
            .into_iter()
            .map(|mode| {
                let root = &root;
                scope.spawn(move || agree_on_hir_dump(std_hir(mode), root, &[], 0))
            })
            .collect();
        joined(parts)
    });
    let loose = loose_corpus();
    let packages = package_corpus();
    let wasm = wasm_corpus();
    let tests = test_corpus();
    // The resolution dump's floors (5.2b): 251 loose programs, 74 package
    // files, 39 WASM programs and 10 files with tests at 5.3a.
    assert!(loose.len() >= 240, "only {} loose programs", loose.len());
    assert!(
        packages.len() >= 60,
        "only {} package files",
        packages.len()
    );
    assert!(wasm.len() >= 35, "only {} WASM programs", wasm.len());
    assert!(tests.len() >= 8, "only {} files with tests", tests.len());
    let (check, wasm_dumps, test_dumps, counts) = std::thread::scope(|scope| {
        let cases: Vec<_> = [Mode::Check, Mode::Wasm, Mode::Test]
            .into_iter()
            .map(|mode| scope.spawn(move || agree_on_hir_cases(mode, &hir_cases_dir(mode))))
            .collect();
        let package_dumps = scope.spawn(|| {
            agree_in_parts(&packages, 3, |part| {
                agree_on_hir_in(Mode::Check, &root, part, 0)
            })
        });
        let wasm_dumps = scope.spawn(|| agree_on_hir_in(Mode::Wasm, &root, &wasm, 0));
        let test_dumps = scope.spawn(|| agree_on_hir_in(Mode::Test, &root, &tests, 0));
        let mut check = agree_on_hir_in(Mode::Check, &root, &loose, 0);
        check.push_str(&package_dumps.join().expect("the packages' thread"));
        let mut wasm_dumps = wasm_dumps.join().expect("the WASM programs' thread");
        let mut test_dumps = test_dumps.join().expect("the tests' thread");
        let mut counts = Vec::new();
        for (mode, case) in [Mode::Check, Mode::Wasm, Mode::Test].into_iter().zip(cases) {
            let (dumps, count) = case.join().expect("a mode's cases' thread");
            counts.push(count);
            match mode {
                Mode::Check => check.push_str(&dumps),
                Mode::Wasm => wasm_dumps.push_str(&dumps),
                Mode::Test => test_dumps.push_str(&dumps),
            }
        }
        (check, wasm_dumps, test_dumps, counts)
    });
    // 5.3a's cases: 164 under `check` (154 extracted, 10 by hand), 2 for
    // the WASM target, 1 under test mode.
    assert!(counts[0] >= 150, "only {} type-checker cases", counts[0]);
    assert!(counts[1] >= 2, "only {} WASM cases", counts[1]);
    assert!(counts[2] >= 1, "only {} test-mode cases", counts[2]);
    let parts = |dumps: &str, head: &str| dumps.lines().filter(|l| l.starts_with(head)).count();
    assert_eq!(
        parts(&check, "(Hir target=native checked="),
        loose.len() + packages.len() + counts[0]
    );
    assert_eq!(
        parts(&wasm_dumps, "(Hir target=wasm checked="),
        wasm.len() + counts[1]
    );
    assert_eq!(
        parts(&test_dumps, "(Hir target=native mode=test checked="),
        tests.len() + counts[2]
    );
    // Most programs reach the type checker.
    let checked = parts(&check, "(Hir target=native checked=true");
    assert!(
        checked * 10 > parts(&check, "(Hir ") * 8,
        "{checked} checked"
    );
    let found = hir_forms(&[
        &check,
        &wasm_dumps,
        &test_dumps,
        &std_parts[0],
        &std_parts[1],
        &std_parts[2],
    ]);
    let listed = |f: &String| {
        f.ends_with("=<n>")
            || f.ends_with("=<id>")
            || nova_driver::hir_dump::KINDS.contains(&f.as_str())
            || nova_driver::hir_dump::LATER_KINDS.contains(&f.as_str())
    };
    let unlisted: Vec<&String> = found.iter().filter(|f| !listed(f)).collect();
    assert!(unlisted.is_empty(), "{unlisted:?} are in no list");
    let missing: Vec<&str> = nova_driver::hir_dump::KINDS
        .iter()
        .copied()
        .filter(|k| !found.contains(*k))
        .collect();
    assert!(missing.is_empty(), "no HIR dump holds {missing:?}");
    let later: Vec<&str> = nova_driver::hir_dump::LATER_KINDS
        .iter()
        .copied()
        .filter(|k| found.contains(*k))
        .collect();
    assert!(later.is_empty(), "{later:?} appear");
    let mut all = codes_in(&check);
    for (code, n) in codes_in(&test_dumps) {
        *all.entry(code).or_insert(0) += n;
    }
    // The resolver's E0092 (a std module the target lacks, imported)
    // occurs natively too; the item passes' only for the WASM target.
    assert!(
        wasm_dumps
            .contains("which target wasm does not have\"\n    (Primary \"every_item_e0092_site\""),
        "no item E0092"
    );
    for (code, n) in codes_in(&wasm_dumps) {
        *all.entry(code).or_insert(0) += n;
    }
    let absent: Vec<&str> = ITEM_CODES
        .iter()
        .copied()
        .filter(|c| !all.contains_key(*c))
        .collect();
    assert!(absent.is_empty(), "{absent:?} occur nowhere: {all:?}");
    eprintln!(
        "the HIR pass dumps {} loose programs, {} package files, {} WASM programs, {} files \
         with tests and {:?} cases; codes {all:?}",
        loose.len(),
        packages.len(),
        wasm.len(),
        tests.len(),
        counts
    );
}

/// The type checker's cases of every mode as a Windows checkout has them
/// ([`crlf`]), as the resolver's are compared
/// (`the_resolution_dumps_agree_on_the_cases_with_windows_line_endings`):
/// a mode whose copy changes no file is not run again.
#[test]
fn the_hir_dumps_agree_on_the_cases_with_windows_line_endings() {
    let dir = fresh_dir("crlf-hir");
    let mut crlf_files = 0;
    let mut changed_modes = Vec::new();
    for mode in [Mode::Check, Mode::Wasm, Mode::Test] {
        let (held, changed) = copy_crlf(&hir_cases_dir(mode), &dir.join(hir_cases_name(mode)));
        crlf_files += held;
        if changed > 0 {
            changed_modes.push(mode);
        }
    }
    assert!(
        crlf_files > 150,
        "only {crlf_files} cases have CRLF line endings"
    );
    std::thread::scope(|scope| {
        let modes: Vec<_> = changed_modes
            .into_iter()
            .map(|mode| {
                let cases = dir.join(hir_cases_name(mode));
                scope.spawn(move || (mode, agree_on_hir_cases(mode, &cases)))
            })
            .collect();
        for handle in modes {
            let (mode, (dumps, count)) = handle.join().expect("a mode's thread");
            let parts = dumps.lines().filter(|l| l.starts_with("(Hir ")).count();
            assert_eq!(parts, count, "{mode:?}");
        }
    });
}

/// N shares std's item tables only when std alone gives no diagnostic, as
/// S0 does (5.3a, spec 5.3 §3.4): with a std whose `core` names a type it
/// lacks, an item E0001 and no resolver diagnostic, N checks std again
/// for each program, so each program that reaches the type checker keeps
/// std's E0001 among its diagnostics, and its dump is otherwise S0's,
/// whose std is built in; std's part holds it too. S0's own fallback is
/// unreachable today: its built-in std gives no item diagnostic, so no
/// input reaches it (the 5.3a plan's mutant 50, which passes by design).
/// The roots are the first eight loose cases of `cases/check/`.
#[test]
fn n_checks_std_for_each_program_when_std_gives_an_item_diagnostic() {
    let std = std_copy("item-error-std");
    let file = "core/lib.nova";
    let (old, new) = ("fn mix64(x: Int) -> Int {", "fn mix64(x: Intt) -> Int {");
    let text = std::fs::read_to_string(std.join(file))
        .expect("read the std file")
        .replace("\r\n", "\n");
    assert_eq!(text.matches(old).count(), 1, "{file}: {old}");
    std::fs::write(std.join(file), text.replacen(old, new, 1)).expect("alter the std file");
    let mark = "(Error \"E0001\" \"cannot find type `Intt`\"";
    let std = std.display().to_string();
    let cases = hir_cases_dir(Mode::Check);
    let roots: Vec<String> = case_roots(&cases).loose[..8].to_vec();
    let s0 = run(&nova(), hir_s0(), &cases, &roots).expect("run S0");
    let n_args = [n_hir(), &["--std", &std]].concat();
    let n = run(selfhost(), &n_args, &cases, &roots).expect("run N");
    assert_eq!((s0.code, n.code), (Some(0), Some(0)), "{}", n.stderr);
    let (s0, n) = (
        String::from_utf8_lossy(&s0.stdout).into_owned(),
        String::from_utf8_lossy(&n.stdout).into_owned(),
    );
    let (s0_parts, n_parts) = (parts_of(&s0), parts_of(&n));
    assert_eq!(n_parts.len(), roots.len(), "{n}");
    let mut checked = 0;
    for (s0_part, n_part) in s0_parts.iter().zip(&n_parts) {
        if !s0_part.contains(" checked=true\n") {
            assert_eq!(n_part, s0_part);
            continue;
        }
        checked += 1;
        assert_eq!(n_part.matches(mark).count(), 1, "{n_part}");
        assert_eq!(
            without_parens(&without_block(n_part, mark)),
            without_parens(s0_part),
            "{n_part}"
        );
    }
    assert!(checked >= 6, "{checked} of {} checked", roots.len());
    let part = run(
        selfhost(),
        &[n_hir(), &["--std", &std, "--std-part"]].concat(),
        &cases,
        &[],
    )
    .expect("run N's std part");
    assert_eq!(part.code, Some(0));
    assert_eq!(
        String::from_utf8_lossy(&part.stdout).matches(mark).count(),
        1
    );
}

/// N with `args` in `dir`, on a main thread of `stack` KB where `ulimit -s`
/// can set it (Unix), or of the platform's size (Windows: 1 MB): its exit
/// code and stdout, settled as [`run`] settles it ([`settled_stdout`]).
fn n_args_on_stack(dir: &Path, stack: Option<u32>, args: &[&str]) -> (Option<i32>, Vec<u8>) {
    let mut cmd = match stack {
        Some(kb) => {
            // `$0` is N and `$@` its arguments, so that no path is quoted.
            let mut sh = Command::new("sh");
            sh.arg("-c")
                .arg(format!("ulimit -s {kb} && exec \"$0\" \"$@\""))
                .arg(selfhost())
                .args(args);
            sh
        }
        None => {
            let mut n = Command::new(selfhost());
            n.args(args);
            n
        }
    };
    let out = output_within(
        cmd.current_dir(dir).stdin(Stdio::null()),
        CHILD_LIMIT,
        &format!("selfhost {} on a {stack:?} KB stack", args.join(" ")),
    )
    .unwrap_or_else(|e| panic!("{e}"));
    (
        out.status.code(),
        settled_stdout(args, out.stdout, cfg!(windows)),
    )
}

/// N's item passes on its own stack (spec 5.3 §3.7, ADR 0055 §12): every
/// written nesting of a type, 128 levels deep, the parser's limit, in each
/// item position (a parameter's and a return's type, a field, a variant's
/// payload, a constant, a trait's method, an impl's self type, an impl's
/// method alike and unlike its trait's, an associated type's binding, a
/// `where` bound and an extern's signature), through N's `selfhost hir` on
/// a main thread of 1 MB, as
/// [`n_parses_128_levels_of_every_kind_on_a_1_mb_main_thread`] runs N's
/// parser: N's dump is S0's, which S0 checks on its 64 MB thread. The item
/// passes walk arrays, a generic record's arguments, tuples and `fn` types'
/// parameters and returns all 128 levels deep. The other five kinds' depth
/// is the parser's alone, which the parser's test covers: parentheses are
/// gone once parsed (`(T)` is `T`, so the item passes see `Int`), and for
/// optionals, references, mutable references and pointers both sides stop
/// at the outermost level with one E0900 and walk no further inside. They
/// stay here to pin that on both sides.
#[test]
fn n_checks_items_holding_128_levels_of_every_kind_on_a_1_mb_main_thread() {
    // Each kind's level, written around `Int`: an optional's first `?` is
    // not a level, each after it one more.
    const KINDS: [(&str, &str, &str); 10] = [
        ("arrays", "[", "]"),
        ("generics", "W<", ">"),
        ("tuples", "(", ", Int)"),
        ("fn-parameters", "fn(", ") -> Int"),
        ("fn-returns", "fn() -> ", ""),
        ("parentheses", "(", ")"),
        ("optionals", "", "?"),
        ("references", "& ", ""),
        ("mutable-references", "&mut ", ""),
        ("pointers", "*", ""),
    ];
    // Every item position of a type, `{T}` the deep type.
    const ITEMS: [&str; 12] = [
        "fn f(x: {T}) {}\n",
        "fn g() -> {T} {}\n",
        "record R { x: {T} }\n",
        "type S = | A({T}) | B\n",
        "const C: {T} = 1\n",
        "trait Tr { fn m(self, x: {T}) -> {T} }\n",
        "trait Tr2 {}\nimpl Tr2 for {T} {}\n",
        "trait Tr3 { fn m(self, x: {T}) -> {T} }\nrecord Q {}\nimpl Tr3 for Q { fn m(self, x: {T}) -> {T} {} }\n",
        "trait Tr4 { fn m(self, x: Int) -> Int }\nrecord M {}\nimpl Tr4 for M { fn m(self, x: {T}) -> {T} {} }\n",
        "trait It {\n    type Item\n    fn get(self) -> Self::Item\n}\nrecord P {}\nimpl It for P {\n    type Item = {T}\n    fn get(self) -> {T} {}\n}\n",
        "fn h<U>(x: U) where {T}: Eq {}\n",
        "extern \"C\" {\n    fn e(x: {T}) -> {T}\n}\n",
    ];
    let dir = fresh_dir("deep-n-items");
    let mut files = Vec::new();
    for (kind, open, close) in KINDS {
        let extra = if kind == "optionals" { "?" } else { "" };
        let ty = format!("{}Int{}{extra}", open.repeat(128), close.repeat(128));
        let items: Vec<String> = ITEMS.iter().map(|item| item.replace("{T}", &ty)).collect();
        let file = format!("{kind}-128.nova");
        std::fs::write(
            dir.join(&file),
            format!("record W<T> {{ x: T }}\n\n{}", items.join("\n")),
        )
        .expect("write a deep input");
        files.push(file);
    }
    let s0 = run(&nova(), hir_s0(), &dir, &files).expect("run S0");
    assert_eq!(s0.code, Some(0), "{}", s0.stderr);
    // At the parser's limit, not past it: every program reaches the type
    // checker.
    let text = String::from_utf8_lossy(&s0.stdout);
    assert_eq!(
        text.matches(" checked=true\n").count(),
        files.len(),
        "{text}"
    );
    assert!(!text.contains("\"P0002\""), "{text}");
    let std_dir = std_dir();
    let mut args = n_hir().to_vec();
    args.extend(["--std", std_dir.as_str()]);
    args.extend(files.iter().map(String::as_str));
    let stack = if cfg!(unix) { Some(1024) } else { None };
    let (code, n) = n_args_on_stack(&dir, stack, &args);
    assert_eq!(code, Some(0), "N's `selfhost hir` on a {stack:?} KB stack");
    if let Some(m) = first_difference(&s0.stdout, &n) {
        panic!(
            "{}",
            report(&m, hir_in(Mode::Check), &dir, &nova(), selfhost())
        );
    }
}

/// The type fuzzer's seeds (5.3a): its own, past the name fuzzer's.
const TYPE_SEEDS: [u64; 3] = [401, 402, 403];
const TYPE_COUNT: usize = 40;

/// The loose corpus programs the type fuzzer starts from: those that
/// import no module, whose HIR dump under `check` reaches the type checker
/// with no diagnostic, so that every diagnostic of an input is its
/// mutations'. Read from S0's dump, which is the same on every system.
fn type_bases() -> Vec<String> {
    let root = root();
    let candidates: Vec<String> = loose_corpus()
        .into_iter()
        .filter(|f| {
            let text = std::fs::read_to_string(root.join(f)).expect("read a corpus file");
            !text.lines().any(|l| l.starts_with("import "))
                && fuzz::parses(&text.replace("\r\n", "\n"))
        })
        .collect();
    let s0 = run(&nova(), hir_s0(), &root, &candidates).unwrap_or_else(|e| panic!("{e}"));
    let dump = String::from_utf8(s0.stdout).expect("the HIR dump is UTF-8");
    let mut bases = Vec::new();
    let mut current: Option<&str> = None;
    let mut clean = false;
    for line in dump.lines().chain(["== "]) {
        if let Some(file) = line.strip_prefix("== ") {
            if let (Some(f), true) = (current, clean) {
                bases.push(f.to_string());
            }
            current = Some(file);
            clean = false;
        } else if line == "(Diagnostics)" {
            clean = true;
        } else if line.starts_with("(Hir target=native checked=false") {
            current = None;
        }
    }
    bases
}

/// `count` type-mutated programs from `seed` ([`fuzz::type_mutate`]), each
/// one of `bases` that the seed picks, read with each `\r\n` as `\n`, so a
/// seed gives the same inputs on every system.
fn type_inputs(bases: &[String], seed: u64, count: usize) -> Vec<String> {
    let root = root();
    let mut bases = bases.to_vec();
    bases.sort();
    let mut rng = SplitMix64::new(seed);
    (0..count)
        .map(|i| {
            let file = rng.pick(&bases);
            let source = std::fs::read_to_string(root.join(file))
                .expect("read a corpus file")
                .replace("\r\n", "\n");
            fuzz::type_mutate(&mut rng, &source, i)
        })
        .collect()
}

/// The type fuzzer's item mutations (5.3a, spec 5.3 §3.6): each seed's
/// inputs reach the type checker, and each code the mutations aim at
/// occurs at least twice over the fixed seeds.
#[test]
fn the_type_fuzzers_item_inputs_agree_and_hold_each_aimed_code() {
    let bases = type_bases();
    // 160 of the loose corpus at 5.3a.
    assert!(bases.len() >= 100, "only {} bases", bases.len());
    let seeded: Vec<(u64, Vec<String>)> = TYPE_SEEDS
        .iter()
        .map(|&seed| (seed, type_inputs(&bases, seed, TYPE_COUNT)))
        .collect();
    let dir = fresh_dir("types");
    let mut roots = Vec::new();
    for (seed, inputs) in &seeded {
        let sub = dir.join(seed.to_string());
        std::fs::create_dir_all(&sub).expect("create a seed's directory");
        for (i, input) in inputs.iter().enumerate() {
            let file = format!("f{i:05}.nova");
            std::fs::write(sub.join(&file), input).expect("write a type-fuzz input");
            roots.push(format!("{seed}/{file}"));
        }
    }
    let dump = agree_on_hir_in(Mode::Check, &dir, &roots, 0);
    let unchecked: Vec<&str> = dump
        .split("== ")
        .filter(|p| p.contains("(Hir target=native checked=false"))
        .map(|p| p.lines().next().unwrap_or(""))
        .collect();
    assert!(
        unchecked.is_empty(),
        "inputs that stop before the type checker: {unchecked:?}"
    );
    let codes = codes_in(&dump);
    let mut aimed: Vec<&str> = fuzz::TYPE_ITEMS.iter().map(|(c, _)| *c).collect();
    aimed.sort();
    aimed.dedup();
    let short: Vec<(&str, usize)> = aimed
        .iter()
        .map(|c| (*c, codes.get(*c).copied().unwrap_or(0)))
        .filter(|(_, n)| *n < 2)
        .collect();
    assert!(
        short.is_empty(),
        "aimed codes under two: {short:?}; all: {codes:?}"
    );
    for seed in TYPE_SEEDS {
        let part = seed_parts(&dump, seed);
        assert_eq!(part.matches("(Hir ").count(), TYPE_COUNT, "seed {seed}");
    }
    eprintln!("the type fuzzer's codes over seeds {TYPE_SEEDS:?}: {codes:?}");
}

// === 5.3b: the full HIR dump (spec 5.3 §4.3, §4.6, ADR 0056) ===
//
// Part 1 builds S0's side: the full `check --dump-hir`, every function
// after the item part, and the body cases (`cases/check*`, the body-level
// tests' sources and hand cases). Until N checks bodies ([`N_BODIES`]),
// the full dump's sets run through S0 alone, through [`run`], and their
// assertions hold on S0's dump; later parts set `N_BODIES`, and every set
// below is then compared byte for byte through [`compare_batch`].

/// The type checker's body codes (spec 5.3 §4.4), beside the item codes
/// ([`ITEM_CODES`]): each of the 38 occurs among the full dumps of the
/// cases, the corpus and std.
const BODY_CODES: [&str; 22] = [
    "E0011", "E0013", "E0014", "E0016", "E0020", "E0021", "E0022", "E0060", "E0080", "E0081",
    "E0086", "E0087", "E0094", "E0095", "E0096", "E0097", "E0098", "E0099", "E0100", "E0101",
    "E0103", "E0104",
];

/// The full HIR dump through both sides under `mode`: S0's `check
/// --dump-hir` and N's `hir --std DIR`, each with the mode's flags.
fn full_hir_in(mode: Mode) -> &'static Dump {
    static DUMPS: [OnceLock<Dump>; 3] = [OnceLock::new(), OnceLock::new(), OnceLock::new()];
    DUMPS[mode as usize].get_or_init(|| hir_dump_from(HIR_S0, mode, &[]))
}

/// std's part of the full HIR dump under `mode`.
fn full_std_hir(mode: Mode) -> &'static Dump {
    static DUMPS: [OnceLock<Dump>; 3] = [OnceLock::new(), OnceLock::new(), OnceLock::new()];
    DUMPS[mode as usize].get_or_init(|| hir_dump_from(HIR_S0, mode, &["--std"]))
}

/// `dump`, a full HIR dump, on `files` relative to `dir`, each side
/// exiting with `code`: compared through [`compare_batch`] once N checks
/// bodies ([`N_BODIES`]), and run through S0 alone ([`run`]) before, its
/// dump then well formed ([`assert_well_formed`]). Returns S0's output.
fn agree_on_full_hir(dump: &Dump, dir: &Path, files: &[String], code: i32) -> String {
    if N_BODIES {
        return compare_batch(dump, dir, files, code, &nova(), selfhost())
            .unwrap_or_else(|report| panic!("{report}"));
    }
    let s0 = run(&nova(), dump.s0, dir, files).unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(
        s0.code,
        Some(code),
        "{}: {}",
        batch_name(&nova(), dump.s0, files),
        s0.stderr
    );
    let text = String::from_utf8(s0.stdout).expect("the HIR dump is UTF-8");
    assert_well_formed(&text);
    text
}

/// [`agree_on_full_hir`] under `mode` on `files`, each a program's root.
fn agree_on_full_hir_in(mode: Mode, dir: &Path, files: &[String], code: i32) -> String {
    if files.is_empty() {
        return String::new();
    }
    agree_on_full_hir(full_hir_in(mode), dir, files, code)
}

/// [`agree_on_hir_cases`] for the full dump.
fn agree_on_full_hir_cases(mode: Mode, cases: &Path) -> (String, usize) {
    let roots = case_roots(cases);
    let count = roots.loose.len() + roots.apart.len() + roots.packages.len();
    let from_cases: Vec<String> = roots.loose.into_iter().chain(roots.packages).collect();
    let mut all = agree_on_full_hir_in(mode, cases, &from_cases, 0);
    for dir in &roots.apart {
        all.push_str(&agree_on_full_hir_in(
            mode,
            dir,
            &["main.nova".to_string()],
            0,
        ));
    }
    (all, count)
}

/// [`agree_on_full_hir_cases`] on the cases of `mode` as they are, once
/// a run, for the tests that read them.
fn full_hir_cases(mode: Mode) -> &'static (String, usize) {
    static CASES: [OnceLock<(String, usize)>; 3] =
        [OnceLock::new(), OnceLock::new(), OnceLock::new()];
    CASES[mode as usize].get_or_init(|| agree_on_full_hir_cases(mode, &hir_cases_dir(mode)))
}

/// `dump` is well formed, as the grammar's nesting has it (ADR 0055 §2,
/// ADR 0056): after each `== ` header, an `IOERR` line or a part whose
/// every line starts two spaces deeper for each node still open and with
/// `(`, whose parentheses outside the quotes never close more than is open
/// and balance by the part's end, and whose quotes close on their line.
fn assert_well_formed(dump: &str) {
    let mut depth = 0usize;
    let mut part = "";
    for (n, line) in dump.lines().enumerate() {
        if line.starts_with("== ") || line.starts_with("IOERR ") {
            assert_eq!(depth, 0, "{part}: a node open before line {n}");
            part = line;
            continue;
        }
        let indent = line.len() - line.trim_start_matches(' ').len();
        assert!(
            indent == 2 * depth && line[indent..].starts_with('('),
            "{part}: line {n} at depth {depth}: {line}"
        );
        let mut quoted = false;
        for c in line[indent..].chars() {
            match c {
                '"' => quoted = !quoted,
                '(' if !quoted => depth += 1,
                ')' if !quoted => {
                    assert!(depth > 0, "{part}: line {n} closes too much: {line}");
                    depth -= 1;
                }
                _ => {}
            }
        }
        assert!(!quoted, "{part}: line {n} leaves a quote open: {line}");
    }
    assert_eq!(depth, 0, "{part}: a node open at the end");
}

/// The well-formedness check refuses a line at the wrong depth, a closing
/// parenthesis too many, a node left open and a quote left open, and
/// takes parentheses inside quotes as text.
#[test]
fn the_well_formedness_check_refuses_what_it_should() {
    let good = "== a.nova\n(Hir target=native checked=true\n  (Function u0 \"(\" a=1\n    (Ret ())))\n(Diagnostics)\n== b.nova\nIOERR NotFound\n";
    assert_well_formed(good);
    for bad in [
        "(Hir\n(Ret ()))\n",
        "(Hir\n  (Ret ())))\n",
        "(Hir\n  (Ret ())\n",
        "(Hir \"open)\n",
        "== a.nova\n(Hir\n  (Ret ())\n== b.nova\n",
    ] {
        let caught = std::panic::catch_unwind(|| assert_well_formed(bad));
        assert!(caught.is_err(), "{bad:?} passed");
    }
}

/// std's three full parts, every corpus root (natively, for the WASM
/// target and under test mode) and the cases of each mode, through the
/// full HIR dump (5.3b, spec 5.3 §4.6): each root's part is there; every
/// form the dump prints is listed, every listed form but
/// [`nova_driver::hir_dump::LATER_KINDS`] occurs, functions' forms among
/// them; std's parts name no program's id, each its functions; and each of
/// the 38 codes occurs, E0092 only for the WASM target. Through S0 alone
/// until N checks bodies ([`N_BODIES`]).
#[test]
fn the_full_hir_dumps_agree_on_std_the_corpus_and_the_cases() {
    let root = root();
    let std_parts: Vec<String> = std::thread::scope(|scope| {
        let parts: Vec<_> = [Mode::Check, Mode::Wasm, Mode::Test]
            .into_iter()
            .map(|mode| {
                let root = &root;
                scope.spawn(move || agree_on_full_hir(full_std_hir(mode), root, &[], 0))
            })
            .collect();
        joined(parts)
    });
    for (part, head) in std_parts.iter().zip([
        "(HirStd target=native records=",
        "(HirStd target=wasm records=",
        "(HirStd target=native mode=test records=",
    ]) {
        assert!(part.starts_with(head), "{}", &part[..80]);
        assert!(part
            .lines()
            .next()
            .is_some_and(|l| l.contains(" functions=")));
        assert!(part.contains("\n  (Function s"), "{head}");
        assert!(part.ends_with(")\n(Diagnostics)\n"), "{head}");
        let program_id = |w: &str| {
            w.len() > 1 && w.starts_with('u') && w[1..].bytes().all(|b| b.is_ascii_digit())
        };
        assert!(
            !part.split([' ', '(', ')', '\n']).any(program_id),
            "{head}: a program's id in std's part"
        );
    }
    let loose = loose_corpus();
    let packages = package_corpus();
    let wasm = wasm_corpus();
    let tests = test_corpus();
    let (check, wasm_dumps, test_dumps, counts) = std::thread::scope(|scope| {
        let cases: Vec<_> = [Mode::Check, Mode::Wasm, Mode::Test]
            .into_iter()
            .map(|mode| scope.spawn(move || full_hir_cases(mode).clone()))
            .collect();
        let package_dumps = scope.spawn(|| {
            agree_in_parts(&packages, 3, |part| {
                agree_on_full_hir_in(Mode::Check, &root, part, 0)
            })
        });
        let wasm_dumps = scope.spawn(|| agree_on_full_hir_in(Mode::Wasm, &root, &wasm, 0));
        let test_dumps = scope.spawn(|| agree_on_full_hir_in(Mode::Test, &root, &tests, 0));
        let mut check = agree_on_full_hir_in(Mode::Check, &root, &loose, 0);
        check.push_str(&package_dumps.join().expect("the packages' thread"));
        let mut wasm_dumps = wasm_dumps.join().expect("the WASM programs' thread");
        let mut test_dumps = test_dumps.join().expect("the tests' thread");
        let mut counts = Vec::new();
        for (mode, case) in [Mode::Check, Mode::Wasm, Mode::Test].into_iter().zip(cases) {
            let (dumps, count) = case.join().expect("a mode's cases' thread");
            counts.push(count);
            match mode {
                Mode::Check => check.push_str(&dumps),
                Mode::Wasm => wasm_dumps.push_str(&dumps),
                Mode::Test => test_dumps.push_str(&dumps),
            }
        }
        (check, wasm_dumps, test_dumps, counts)
    });
    // 5.3b's cases: 5.3a's 164, 646 of the body-level tests' sources and
    // 13 by hand under `check`; 2 for the WASM target; 5 under test mode.
    assert!(counts[0] >= 800, "only {} type-checker cases", counts[0]);
    assert!(counts[1] >= 2, "only {} WASM cases", counts[1]);
    assert!(counts[2] >= 5, "only {} test-mode cases", counts[2]);
    let parts = |dumps: &str, head: &str| dumps.lines().filter(|l| l.starts_with(head)).count();
    assert_eq!(
        parts(&check, "(Hir target=native checked="),
        loose.len() + packages.len() + counts[0]
    );
    assert_eq!(
        parts(&wasm_dumps, "(Hir target=wasm checked="),
        wasm.len() + counts[1]
    );
    assert_eq!(
        parts(&test_dumps, "(Hir target=native mode=test checked="),
        tests.len() + counts[2]
    );
    let found = hir_forms(&[
        &check,
        &wasm_dumps,
        &test_dumps,
        &std_parts[0],
        &std_parts[1],
        &std_parts[2],
    ]);
    let kinds = || {
        nova_driver::hir_dump::KINDS
            .iter()
            .chain(nova_driver::hir_dump::BODY_KINDS)
    };
    let listed = |f: &String| {
        f.ends_with("=<n>")
            || f.ends_with("=<id>")
            || kinds().any(|k| k == f)
            || nova_driver::hir_dump::LATER_KINDS.contains(&f.as_str())
    };
    let unlisted: Vec<&String> = found.iter().filter(|f| !listed(f)).collect();
    assert!(unlisted.is_empty(), "{unlisted:?} are in no list");
    let missing: Vec<&str> = kinds().copied().filter(|k| !found.contains(*k)).collect();
    assert!(missing.is_empty(), "no full HIR dump holds {missing:?}");
    let mut all = codes_in(&check);
    for dumps in [&test_dumps, &wasm_dumps] {
        for (code, n) in codes_in(dumps) {
            *all.entry(code).or_insert(0) += n;
        }
    }
    let absent: Vec<&str> = ITEM_CODES
        .iter()
        .chain(&BODY_CODES)
        .copied()
        .filter(|c| !all.contains_key(*c))
        .collect();
    assert!(absent.is_empty(), "{absent:?} occur nowhere: {all:?}");
    eprintln!(
        "the full HIR dumps of {} loose programs, {} package files, {} WASM programs, {} files \
         with tests and {:?} cases: {} lines natively; codes {all:?}",
        loose.len(),
        packages.len(),
        wasm.len(),
        tests.len(),
        counts,
        check.lines().count(),
    );
}

/// The type checker's cases of every mode as a Windows checkout has them
/// ([`crlf`]), through the full HIR dump: each part is there, and the
/// codes are those of the cases as they are, which a `\r\n` moves no
/// diagnostic from or to. Through S0 alone until N checks bodies
/// ([`N_BODIES`]).
#[test]
fn the_full_hir_dumps_agree_on_the_cases_with_windows_line_endings() {
    let dir = fresh_dir("crlf-full-hir");
    let mut crlf_files = 0;
    let mut changed_modes = Vec::new();
    for mode in [Mode::Check, Mode::Wasm, Mode::Test] {
        let (held, changed) = copy_crlf(&hir_cases_dir(mode), &dir.join(hir_cases_name(mode)));
        crlf_files += held;
        if changed > 0 {
            changed_modes.push(mode);
        }
    }
    assert!(
        crlf_files > 700,
        "only {crlf_files} cases have CRLF line endings"
    );
    std::thread::scope(|scope| {
        let modes: Vec<_> = changed_modes
            .into_iter()
            .map(|mode| {
                let cases = dir.join(hir_cases_name(mode));
                scope.spawn(move || {
                    let crlf = agree_on_full_hir_cases(mode, &cases);
                    (mode, crlf, full_hir_cases(mode))
                })
            })
            .collect();
        for handle in modes {
            let (mode, (dumps, count), (lf, _)) = handle.join().expect("a mode's thread");
            let parts = dumps.lines().filter(|l| l.starts_with("(Hir ")).count();
            assert_eq!(parts, count, "{mode:?}");
            assert_eq!(codes_in(&dumps), codes_in(lf), "{mode:?}");
        }
    });
}

// === 5.3b, part 2 on: the full dump of the inputs N checks ===
//
// While 5.3b is being built ([`N_BODIES`] false), N's `hir` checks a
// program's bodies and prints every function, as S0's `check --dump-hir`
// does, or refuses the program when its bodies need what N does not check
// yet: it prints `UNPORTED` and why after the root's header, and exits 3.
// So a half-checked body is never compared as though it were complete:
// `n_bodies.txt` lists the inputs whose full program part N gives, and
// the test below holds it both ways. Each part that ports more adds the
// inputs N then gives; the last sets `N_BODIES`, and every set above is
// compared byte for byte, the list and this test removed.

/// The inputs whose full program part N gives ([`N_BODIES`] false): each
/// line of `n_bodies.txt` a mode and an input, a corpus root relative to
/// the repository or a case's root relative to the cases' parent
/// (`cases/check/a.nova`, a case run apart `cases/check/d/main.nova`).
fn n_bodies_listed() -> std::collections::BTreeSet<(String, String)> {
    let path = cases_dir()
        .parent()
        .expect("the cases' parent")
        .join("n_bodies.txt");
    let text = std::fs::read_to_string(&path).expect("read n_bodies.txt");
    text.lines()
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .map(|l| {
            let (mode, input) = l.split_once(' ').expect("a mode and an input");
            (mode.to_string(), input.to_string())
        })
        .collect()
}

/// A mode's name in `n_bodies.txt`.
fn mode_name(mode: Mode) -> &'static str {
    match mode {
        Mode::Check => "check",
        Mode::Wasm => "wasm",
        Mode::Test => "test",
    }
}

/// Each root's part of `dump`, by the header line that starts it.
fn parts_by_root(dump: &str) -> Vec<(String, String)> {
    let mut parts: Vec<(String, String)> = Vec::new();
    for line in dump.split_inclusive('\n') {
        if let Some(root) = line.strip_prefix("== ") {
            parts.push((
                root.trim_end_matches(['\n', '\r']).to_string(),
                String::new(),
            ));
        } else if let Some((_, part)) = parts.last_mut() {
            part.push_str(line);
        }
    }
    parts
}

/// What N's full dump gave for one input, against the list.
#[derive(Default)]
struct BodiesCompared {
    agree: usize,
    refused: Vec<((String, String), String)>,
    unlisted: Vec<String>,
    differ: Vec<String>,
}

/// One set of inputs, `roots` relative to `dir` under `mode`, each named
/// `prefix` and its root in the list: N's full `hir` on every root, then
/// S0's `check --dump-hir` on the roots N gives, in batches.
fn n_bodies_on(
    mode: Mode,
    dir: &Path,
    roots: &[String],
    prefix: &str,
    listed: &std::collections::BTreeSet<(String, String)>,
) -> BodiesCompared {
    let std_dir = std_dir();
    let mut n_args = vec!["hir", "--std", std_dir.as_str()];
    n_args.extend(mode.flags());
    let s0_args: Vec<&str> = HIR_S0.iter().chain(mode.flags()).copied().collect();
    let mut out = BodiesCompared::default();
    for chunk in roots.chunks(BATCH) {
        let n = run(selfhost(), &n_args, dir, chunk).unwrap_or_else(|e| panic!("{e}"));
        let n_text = String::from_utf8(n.stdout).expect("N's dump is UTF-8");
        let n_parts = parts_by_root(&n_text);
        let names: Vec<&str> = n_parts.iter().map(|(r, _)| r.as_str()).collect();
        assert_eq!(names, chunk, "N's headers, {}: {}", dir.display(), n.stderr);
        let refused = n_parts
            .iter()
            .filter(|(_, p)| p.starts_with("UNPORTED "))
            .count();
        assert_eq!(
            n.code,
            Some(if refused > 0 { 3 } else { 0 }),
            "N on {chunk:?}: {}",
            n.stderr
        );
        let given: Vec<String> = n_parts
            .iter()
            .filter(|(_, p)| !p.starts_with("UNPORTED "))
            .map(|(r, _)| r.clone())
            .collect();
        let s0_parts = if given.is_empty() {
            Vec::new()
        } else {
            let s0 = run(&nova(), &s0_args, dir, &given).unwrap_or_else(|e| panic!("{e}"));
            assert_eq!(s0.code, Some(0), "S0 on {given:?}: {}", s0.stderr);
            parts_by_root(&String::from_utf8(s0.stdout).expect("S0's dump is UTF-8"))
        };
        let mut s0_parts = s0_parts.into_iter();
        for (root, part) in n_parts {
            let input = format!("{prefix}{root}");
            let key = (mode_name(mode).to_string(), input.clone());
            if let Some(why) = part.strip_prefix("UNPORTED ") {
                out.refused.push((key, why.trim_end().to_string()));
                continue;
            }
            let (s0_root, s0_part) = s0_parts.next().expect("S0's part");
            assert_eq!(s0_root, root);
            if !listed.contains(&key) {
                out.unlisted.push(format!("{} {input}", key.0));
            } else if s0_part != part {
                let at = first_difference(s0_part.as_bytes(), part.as_bytes());
                out.differ.push(format!("{} {input}: {at:?}", key.0));
            } else {
                out.agree += 1;
            }
        }
    }
    out
}

/// N's full HIR dump of every input of the full dump's sets but std's
/// parts (the corpus natively, for the WASM target and under test mode,
/// and the cases of each mode) is S0's, byte for byte, for each input
/// `n_bodies.txt` lists, and N refuses every other input, printing
/// `UNPORTED` and why (5.3b, while it is being built, ADR 0056): every
/// listed input agrees, and an input not listed is one N refuses. std's
/// full part of each mode, which holds std's bodies, is listed as
/// `<mode> std` (5.3b part 4).
#[test]
fn n_gives_the_full_dump_of_the_listed_inputs_and_refuses_the_rest() {
    let listed = n_bodies_listed();
    let root = root();
    let mut sets: Vec<(Mode, PathBuf, Vec<String>, String)> = vec![
        (Mode::Check, root.clone(), loose_corpus(), String::new()),
        (Mode::Check, root.clone(), package_corpus(), String::new()),
        (Mode::Wasm, root.clone(), wasm_corpus(), String::new()),
        (Mode::Test, root.clone(), test_corpus(), String::new()),
    ];
    for mode in [Mode::Check, Mode::Wasm, Mode::Test] {
        let name = hir_cases_name(mode);
        let roots = case_roots(&hir_cases_dir(mode));
        let from_cases: Vec<String> = roots.loose.into_iter().chain(roots.packages).collect();
        sets.push((
            mode,
            hir_cases_dir(mode),
            from_cases,
            format!("cases/{name}/"),
        ));
        for dir in roots.apart {
            let apart = dir
                .file_name()
                .expect("a name")
                .to_string_lossy()
                .into_owned();
            sets.push((
                mode,
                dir,
                vec!["main.nova".to_string()],
                format!("cases/{name}/{apart}/"),
            ));
        }
    }
    let compared: Vec<BodiesCompared> = std::thread::scope(|scope| {
        let handles: Vec<_> = sets
            .iter()
            .map(|(mode, dir, roots, prefix)| {
                let listed = &listed;
                scope.spawn(move || n_bodies_on(*mode, dir, roots, prefix, listed))
            })
            .collect();
        joined(handles)
    });
    let mut all = BodiesCompared::default();
    for c in compared {
        all.agree += c.agree;
        all.refused.extend(c.refused);
        all.unlisted.extend(c.unlisted);
        all.differ.extend(c.differ);
    }
    // std's full part of each mode, listed as `<mode> std`: N's is S0's,
    // byte for byte, or N refuses it, as a root's.
    for mode in [Mode::Check, Mode::Wasm, Mode::Test] {
        let key = (mode_name(mode).to_string(), "std".to_string());
        let std_dir = std_dir();
        let mut args = vec!["hir", "--std", std_dir.as_str()];
        args.extend(mode.flags());
        args.push("--std-part");
        let n = run(selfhost(), &args, &root, &[]).unwrap_or_else(|e| panic!("{e}"));
        let n_text = String::from_utf8(n.stdout).expect("N's dump is UTF-8");
        if let Some(why) = n_text.strip_prefix("UNPORTED ") {
            assert_eq!(n.code, Some(3), "{mode:?}: {}", n.stderr);
            all.refused.push((key, why.trim_end().to_string()));
            continue;
        }
        assert_eq!(n.code, Some(0), "{mode:?}: {}", n.stderr);
        let mut s0_args: Vec<&str> = HIR_S0.iter().chain(mode.flags()).copied().collect();
        s0_args.push("--std");
        let s0 = run(&nova(), &s0_args, &root, &[]).unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(s0.code, Some(0), "S0's std part, {mode:?}: {}", s0.stderr);
        let s0_text = String::from_utf8(s0.stdout).expect("S0's dump is UTF-8");
        if !listed.contains(&key) {
            all.unlisted.push(format!("{} std", key.0));
        } else if s0_text != n_text {
            let at = first_difference(s0_text.as_bytes(), n_text.as_bytes());
            all.differ.push(format!("{} std: {at:?}", key.0));
        } else {
            all.agree += 1;
        }
    }
    let stale: Vec<&(String, String)> = all
        .refused
        .iter()
        .map(|(key, _)| key)
        .filter(|key| listed.contains(*key))
        .collect();
    let mut why: std::collections::BTreeMap<&str, usize> = std::collections::BTreeMap::new();
    for (_, w) in &all.refused {
        *why.entry(w.as_str()).or_insert(0) += 1;
    }
    eprintln!(
        "N gives {} inputs' full dump as S0's, refuses {}: {why:#?}",
        all.agree,
        all.refused.len()
    );
    assert!(
        all.differ.is_empty(),
        "N's full dump differs from S0's: {:#?}",
        all.differ
    );
    assert!(
        all.unlisted.is_empty(),
        "N gives these inputs' full dump, S0's, but n_bodies.txt does not list them: {:#?}",
        all.unlisted
    );
    assert!(
        stale.is_empty() && all.agree == listed.len(),
        "n_bodies.txt lists {} inputs, of which N gives {}; refused: {stale:#?}",
        listed.len(),
        all.agree
    );
    // 5.3b's part 2: 384 inputs, 352 cases and 32 corpus roots.
    // 5.3b's part 4: 1,159 inputs, 806 cases, 350 corpus roots and std's
    // three parts.
    assert!(all.agree >= 1150, "only {} inputs", all.agree);
}
