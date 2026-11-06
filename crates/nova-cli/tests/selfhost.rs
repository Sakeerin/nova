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
//! `NOVA_SELFHOST_FUZZ=SEED:COUNT` runs a longer search locally, never in
//! CI. Every input that finds a difference becomes a fixed case.
//!
//! Each run builds N into a fixed directory under the system temp
//! directory, so the source tree gets no `target/`, and writes its inputs to
//! fixed directories there too. `nova build`, `nova test` and each side on
//! each batch are killed if they run past [`CHILD_LIMIT`].

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

use compare::{first_difference, report, Dump, AST, TOKENS};
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

/// `program` with `args` on `files`, within [`CHILD_LIMIT`].
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
        stdout: out.stdout,
        code: out.status.code(),
        stderr: String::from_utf8_lossy(&out.stderr).into_owned(),
    })
}

/// One batch, `files` relative to `dir`, through `nova` and `selfhost` with
/// `dump`'s arguments: S0's output, or the report of where the two part,
/// in their dumps or in their exit codes, both of which must be `code`. The
/// report gives both exit codes and the start of anything either side
/// wrote to stderr, so a side that crashed or printed nothing says so; a
/// side that runs past the limit is killed and reported with the batch.
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

#[test]
fn the_corpus_dumps_agree() {
    let files = corpus();
    // 281 under std/, tests/ and examples/ alone at 5.0c.
    assert!(files.len() >= 290, "only {} corpus files", files.len());
    assert!(files.iter().any(|f| f == "selfhost/src/lexer.nova"));
    let dumps = agree(&root(), &files, BATCH, 0);
    // Each file's tree, so the AST dumps were compared too.
    let trees = dumps.lines().filter(|l| l.starts_with("(File")).count();
    assert_eq!(trees, files.len());
}

/// Both lexers must agree on the corpus as a Windows checkout has it
/// ([`windows_corpus`]) on every system, not on Windows alone.
#[test]
fn the_corpus_dumps_agree_with_windows_line_endings() {
    let (dir, files) = windows_corpus("crlf");
    let dumps = agree(&dir, &files, BATCH, 0);
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
}

#[test]
fn the_fixed_cases_agree() {
    let root = root();
    let mut files = Vec::new();
    nova_files(&root, &cases_dir(), &[], &mut files);
    assert!(files.len() >= 10, "only {} fixed cases", files.len());
    agree(&root, &files, BATCH, 0);
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
/// 2, for each dump: a resolution dump needs a root (or std's part).
#[test]
fn both_sides_refuse_no_file() {
    for dump in [&TOKENS, &AST, resolution()] {
        let out = compare_batch(dump, &root(), &[], 2, &nova(), selfhost());
        assert_eq!(out, Ok(String::new()));
    }
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
/// appears in S0's dump of some corpus file or fixed case, and in N's
/// (spec §4.3): it follows from the dumps' being equal, which
/// `the_corpus_dumps_agree` and `the_fixed_cases_agree` check, and each
/// side is checked on its own all the same.
#[test]
fn every_kind_appears_in_the_dumps_of_the_corpus_and_the_cases() {
    let root = root();
    let mut files = corpus();
    nova_files(&root, &cases_dir(), &[], &mut files);
    for (side, program, args) in [("S0", nova(), AST.s0), ("N", selfhost().into(), AST.n)] {
        let mut found = std::collections::HashSet::new();
        for chunk in files.chunks(BATCH) {
            let out = run(&program, args, &root, chunk).unwrap_or_else(|e| panic!("{e}"));
            assert_eq!(out.code, Some(0), "{side}: {}", out.stderr);
            for line in String::from_utf8_lossy(&out.stdout).lines() {
                kinds_on(line, &mut found);
            }
        }
        let missing: Vec<&str> = nova_ast::dump::KINDS
            .iter()
            .copied()
            .filter(|k| !found.contains(*k))
            .collect();
        assert!(
            missing.is_empty(),
            "no file's dump by {side} holds {missing:?}"
        );
    }
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

/// A package's `nova.toml` with what the package graph requires and
/// nothing else: the tests' packages (5.2b, ADR 0054).
const MANIFEST: &str = "[package]\nname = \"pkg\"\nversion = \"0.1.0\"\nedition = \"2026\"\n";

/// The error cases of the resolver (spec §3.5): the inline sources of
/// `nova-resolver`'s tests and of the type checker's tests that reach the
/// resolver, each a file, or a directory whose `main.nova` is the root of
/// a program of several modules.
fn resolve_cases_dir() -> PathBuf {
    cases_dir().join("resolve")
}

/// [`agree_on_resolution`] the resolver's cases in `cases`
/// ([`resolve_cases_dir`], or a copy of it), from `cases`: each file in it,
/// and each directory's `main.nova`, [`BATCH`] roots to a process; but a
/// directory holding a module named for a std module (`dom.nova`,
/// `ui.nova`, `RESERVED_MODULE_NAMES`) from inside it, its root named
/// `main.nova`. E0093 names that module by its path, the root's directory
/// joined with its file name: S0 joins with `Path::join`, which writes `\`
/// on Windows, and N with `/`, so the path must have no directory part
/// (ADR 0053 §5). Returns S0's output and how many programs it holds.
fn agree_on_resolve_cases(cases: &Path) -> (String, usize) {
    let mut entries: Vec<_> = std::fs::read_dir(cases)
        .expect("read the resolver's cases")
        .map(|e| e.expect("a directory entry"))
        .collect();
    entries.sort_by_key(|e| e.file_name());
    let mut roots = Vec::new();
    let mut apart = Vec::new();
    for e in entries {
        let path = e.path();
        let name = e.file_name().to_string_lossy().into_owned();
        if path.is_dir() {
            assert!(
                path.join("main.nova").is_file(),
                "{} has no main.nova",
                path.display()
            );
            if ["dom", "ui"]
                .iter()
                .any(|std| path.join(format!("{std}.nova")).is_file())
            {
                apart.push(path);
            } else {
                roots.push(format!("{name}/main.nova"));
            }
        } else if name.ends_with(".nova") {
            roots.push(name);
        }
    }
    let mut all = agree_on_resolution(cases, &roots, 0);
    for dir in &apart {
        all.push_str(&agree_on_resolution(dir, &["main.nova".to_string()], 0));
    }
    (all, roots.len() + apart.len())
}

/// The programs of the corpus that the resolution dump compares in 5.2a:
/// every corpus file but those directly in a package's `src/` or
/// `tests/`, which are 5.2b's.
fn loose_corpus() -> Vec<String> {
    let root = root();
    corpus()
        .into_iter()
        .filter(|f| {
            let path = root.join(f);
            let parent = path.parent().expect("a parent");
            let in_package = matches!(
                parent.file_name().and_then(|n| n.to_str()),
                Some("src" | "tests")
            ) && parent
                .parent()
                .is_some_and(|p| p.join("nova.toml").is_file());
            !in_package
        })
        .collect()
}

/// The resolution dump through both sides (ADR 0053): S0's `check
/// --dump-resolution`, and N's `resolve --std DIR`, DIR the repository's
/// `std/`, each followed by the roots.
fn resolution() -> &'static Dump {
    static DUMP: OnceLock<Dump> = OnceLock::new();
    DUMP.get_or_init(|| Dump {
        s0: RESOLUTION_S0,
        n: leak_args(&["resolve", "--std", &std_dir()]),
    })
}

/// std's part through both sides: S0's `check --dump-resolution --std`,
/// and N's `resolve --std DIR --std-part`.
fn std_resolution() -> &'static Dump {
    static DUMP: OnceLock<Dump> = OnceLock::new();
    DUMP.get_or_init(|| Dump {
        s0: &["check", "--dump-resolution", "--std"],
        n: leak_args(&["resolve", "--std", &std_dir(), "--std-part"]),
    })
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

/// [`agree_on`] the resolution dump: `files`, relative to `dir`, each a
/// program's root, through both sides, [`BATCH`] roots to a process and
/// the batches side by side, each side exiting with `code`; fail with the
/// report of the first difference. Returns S0's output, which is N's.
/// S0 parses std again for each program (about 0.065 s, debug) and N once
/// per process (about 0.4 s with the debug runtime), so the batches run at
/// once.
fn agree_on_resolution(dir: &Path, files: &[String], code: i32) -> String {
    let (nova, selfhost) = (nova(), selfhost());
    let outs: Vec<Result<String, String>> = std::thread::scope(|scope| {
        let runs: Vec<_> = files
            .chunks(BATCH)
            .map(|chunk| {
                let nova = &nova;
                scope.spawn(move || compare_batch(resolution(), dir, chunk, code, nova, selfhost))
            })
            .collect();
        runs.into_iter()
            .map(|r| r.join().expect("a batch's thread"))
            .collect()
    });
    let mut all = String::new();
    for out in outs {
        match out {
            Ok(out) => all.push_str(&out),
            Err(report) => panic!("{report}"),
        }
    }
    all
}

/// S0's resolution dump of `files` from `dir`, which must exit with
/// `code`: what a package's roots are checked by until N dumps packages
/// (ADR 0054).
fn s0_resolution(dir: &Path, files: &[String], code: i32) -> String {
    let out = Command::new(nova())
        .current_dir(dir)
        .args(RESOLUTION_S0)
        .args(files)
        .output()
        .expect("run nova check --dump-resolution");
    let stdout = String::from_utf8_lossy(&out.stdout).into_owned();
    assert_eq!(out.status.code(), Some(code), "{stdout}");
    stdout
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
    // The dump's misuses, which the base has no form of.
    for args in [
        &["--stdd"][..],
        &["--std", "--std"],
        &["--target", "bogus", "a.nova"],
        &["--help"],
        &["-h"],
        &["--dump-resolution", "a.nova"],
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
/// `--std` prints std's part for the target. N prints the same for the
/// same roots but the package's, whose part S0 alone is checked for until
/// N dumps packages.
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
    // N prints the same, a file that is not UTF-8 included, and a root
    // beside a `nova.toml` that is a directory, which makes no package.
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
    for (target, head) in [
        ("native", "(Std target=native modules=21 "),
        ("wasm", "(Std target=wasm modules=16 "),
    ] {
        let out = Command::new(nova())
            .args(RESOLUTION_S0)
            .args(["--std", "--target", target])
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
/// std's part once, then the programs, [`BATCH`] to a process. Every form
/// of the dump's `KINDS` appears in these dumps, every form they print is
/// listed, and no form of `LATER_KINDS` appears, nor any of the three
/// codes 5.2b's test mode and WASM target give: the dumps are S0's and
/// N's alike, so each holds them.
#[test]
fn the_resolution_dumps_agree_on_std_the_corpus_and_the_cases() {
    let root = root();
    let std = compare_batch(std_resolution(), &root, &[], 0, &nova(), selfhost())
        .unwrap_or_else(|report| panic!("{report}"));
    assert!(
        std.starts_with("(Std target=native modules=21 "),
        "{}",
        &std[..80]
    );
    let loose = loose_corpus();
    // 251 loose programs at 5.2a.
    assert!(loose.len() >= 240, "only {} loose programs", loose.len());
    // The files in a package, which are 5.2b's, are skipped, and said to
    // be: 66 at 5.2a.
    let skipped = corpus().len() - loose.len();
    eprintln!(
        "the resolution pass skips {skipped} corpus files in a package's src/ or tests/ (5.2b's)"
    );
    assert!(skipped >= 60, "only {skipped} corpus files in packages");
    // The corpus and the cases side by side.
    let (mut dumps, (cases, count)) = std::thread::scope(|scope| {
        let cases = scope.spawn(|| agree_on_resolve_cases(&resolve_cases_dir()));
        let corpus = agree_on_resolution(&root, &loose, 0);
        (corpus, cases.join().expect("the cases' thread"))
    });
    // 103 cases at 5.2a.
    assert!(count >= 90, "only {count} resolver cases");
    dumps.push_str(&cases);
    // Each root's part, so every program was compared.
    let parts = dumps.lines().filter(|l| l.starts_with("(Program ")).count();
    assert_eq!(parts, loose.len() + count);
    // None of the codes that only 5.2b's test mode and WASM target give
    // (decision 8), though cases hold the programs that would give them.
    let codes = codes_in(&dumps);
    for code in LATER_RESOLVER_CODES {
        assert!(!codes.contains_key(code), "{code} in {codes:?}");
    }
    let mut found = std::collections::HashSet::new();
    for line in dumps.lines().chain(std.lines()) {
        found.extend(nova_driver::resolution_dump::forms_on(line));
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
        "{later:?} appear, which LATER_KINDS says 5.2a's programs never print"
    );
}

/// The resolver's cases as a Windows checkout has them ([`crlf`]): git
/// gives no `.gitattributes` rule for `cases/`, so CI's Windows runner
/// compares them with `\r\n`, and every system does here.
#[test]
fn the_resolution_dumps_agree_on_the_cases_with_windows_line_endings() {
    let dir = fresh_dir("crlf-resolve");
    let mut files = Vec::new();
    nova_files(&resolve_cases_dir(), &resolve_cases_dir(), &[], &mut files);
    for file in &files {
        let text = std::fs::read(resolve_cases_dir().join(file)).expect("read a case");
        let path = dir.join(file);
        std::fs::create_dir_all(path.parent().expect("a parent")).expect("create a directory");
        std::fs::write(&path, crlf(text)).expect("write a case");
    }
    let crlf_files = files
        .iter()
        .filter(|f| std::fs::read(dir.join(f)).is_ok_and(|t| t.contains(&b'\r')))
        .count();
    assert!(
        crlf_files > 90,
        "only {crlf_files} cases have CRLF line endings"
    );
    let (dumps, count) = agree_on_resolve_cases(&dir);
    let parts = dumps.lines().filter(|l| l.starts_with("(Program ")).count();
    assert_eq!(parts, count);
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
/// named from outside it is (ADR 0054; S0's part alone until N dumps
/// packages). An empty root is a file that is not there.
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
    let out = s0_resolution(&pkg, &roots, 0);
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
    let out = s0_resolution(&dir, &roots, 2);
    assert!(
        out.starts_with(
            "== pkg/tests/main.nova\n(Program target=native resolved=true\n  \
             (Module 0 \"tests/main\" package=0)\n"
        ),
        "{out}"
    );
    assert!(out.ends_with("\n== \nIOERR NotFound\n"), "{out}");
    let out = agree_on_resolution(&dir, &["".to_string()], 2);
    assert_eq!(out, "== \nIOERR NotFound\n");
}

/// A root's package is found as S0's `package_of` finds it (ADR 0053 §3):
/// by asking whether the path `<dir>/nova.toml` is a file, or a link to
/// one, not by looking for that exact name in a listing of `<dir>`. So a
/// `Nova.toml` makes a package on a case-insensitive disk (macOS's and
/// Windows' by default) on both sides, and on a case-sensitive one on
/// neither; and, but on Windows, a link to a file makes one, a dangling
/// link none, and a directory that can be searched but not listed still
/// holds its `nova.toml` for both (as root, which lists it anyway, that
/// part is any other package). The test asks the disk whether it folds case.
/// S0's parts are checked, and N's of the roots in no package until N
/// dumps packages.
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
    let loose: Vec<String> = roots
        .iter()
        .zip(&cases)
        .filter(|(_, (_, package))| !*package)
        .map(|(root, _)| root.clone())
        .collect();
    agree_on_resolution(&dir, &loose, 0);
    let out = s0_resolution(&dir, &roots, 0);
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

/// The name fuzzer's fixed seeds and counts every run checks (spec §3.5).
const NAME_SEEDS: [u64; 3] = [301, 302, 303];
const NAME_COUNT: usize = 50;

/// The resolver's codes that `check`'s pipeline gives a loose program on
/// the native target. Of its 11, E0084 and E0085 are a `@test` function's,
/// which `check` strips before resolving, and E0090 is WASM's: they come
/// with 5.2b's test mode and target.
const NATIVE_RESOLVER_CODES: [&str; 8] = [
    "E0001", "E0002", "E0082", "E0083", "E0089", "E0092", "E0102", "E0900",
];
const LATER_RESOLVER_CODES: [&str; 3] = ["E0084", "E0085", "E0090"];

/// `count` name-mutated programs from `seed` ([`fuzz::name_mutate`]), each
/// one of `files`, under `root`, that the seed picks and that parses. As
/// for [`mutations_of`], the files are picked from their sorted list and
/// read with each `\r\n` as `\n`, so a seed gives the same inputs on every
/// system. Input `i` is written as `f{i:05}.nova`, which a self-import
/// names.
fn names_of(root: &Path, files: &[String], seed: u64, count: usize) -> Vec<String> {
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
                break fuzz::name_mutate(&mut rng, &source, &format!("f{i:05}"));
            }
        })
        .collect()
}

/// The name-mutated loose corpus: [`names_of`] it.
fn names(seed: u64, count: usize) -> Vec<String> {
    names_of(&root(), &loose_corpus(), seed, count)
}

/// Write `inputs` to a fixed directory named after `name`, as
/// `f00000.nova` and on, beside [`fuzz::NAME_SIBLINGS`]: the directory, and
/// the inputs' files, each a program's root.
fn write_name_inputs(name: &str, inputs: &[String]) -> (PathBuf, Vec<String>) {
    let dir = fresh_dir(name);
    for (stem, source) in fuzz::NAME_SIBLINGS {
        std::fs::write(dir.join(format!("{stem}.nova")), source).expect("write a sibling");
    }
    let mut files = Vec::with_capacity(inputs.len());
    for (i, input) in inputs.iter().enumerate() {
        let file = format!("f{i:05}.nova");
        std::fs::write(dir.join(&file), input).expect("write a name-fuzz input");
        files.push(file);
    }
    (dir, files)
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

/// Both sides give the same resolution dump of each of the name fuzzer's
/// inputs from the fixed seeds, every input reaches the resolver, and among
/// them every resolver code `check` gives a loose native program occurs
/// twice or more, and none of the three only test mode and WASM give.
#[test]
fn the_name_fuzzer_agrees_and_gives_every_native_resolver_code() {
    let mut all = std::collections::BTreeMap::new();
    for seed in NAME_SEEDS {
        let inputs = names(seed, NAME_COUNT);
        let dump = agree_on_inputs(&format!("names-{seed}"), &inputs, Dumps::Resolution);
        let resolved = dump.matches("resolved=true").count();
        assert_eq!(
            resolved,
            inputs.len(),
            "seed {seed}: an input did not reach the resolver"
        );
        for (code, n) in codes_in(&dump) {
            *all.entry(code).or_insert(0) += n;
        }
    }
    eprintln!("codes over seeds {NAME_SEEDS:?}: {all:?}");
    // At 5.2a: E0001 46, E0002 63, E0082 14, E0083 8, E0089 10, E0092 13,
    // E0102 50, E0900 18.
    for code in NATIVE_RESOLVER_CODES {
        let n = all.get(code).copied().unwrap_or(0);
        assert!(n >= 2, "{code} occurs {n} times, under 2, in {all:?}");
    }
    for code in LATER_RESOLVER_CODES {
        assert!(!all.contains_key(code), "{code} in {all:?}");
    }
}
