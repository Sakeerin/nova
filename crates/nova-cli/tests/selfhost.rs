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

use compare::{first_difference, report, Dump, TOKENS};
use fuzz::SplitMix64;

/// How many files one process lexes.
const BATCH: usize = 50;

/// The fixed seeds and counts every run checks: 6,000 inputs of soup and
/// 750 mutated corpus files. Measured on a 4-core Linux machine, debug
/// build: the whole file runs in about 10 s, building N included.
const SOUP_SEEDS: [u64; 3] = [1, 2, 3];
const SOUP_COUNT: usize = 2000;
const MUTATION_SEEDS: [u64; 3] = [101, 102, 103];
const MUTATION_COUNT: usize = 250;

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
    let s0 = run(nova, dump.s0, dir, files)?;
    let n = run(selfhost, dump.n, dir, files)?;
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

/// [`agree_on`] the token dump.
fn agree(dir: &Path, files: &[String], batch: usize, code: i32) -> String {
    agree_on(&TOKENS, dir, files, batch, code)
}

/// Write `inputs` to a fixed directory named after `name`, as
/// `f00000.nova` and on, and check that both sides agree on them. Returns
/// S0's output. The fuzzer's generators all come through here, 5.1b's deep
/// nesting included.
fn agree_on_inputs(name: &str, inputs: &[String]) -> String {
    let dir = fresh_dir(name);
    let mut files = Vec::with_capacity(inputs.len());
    for (i, input) in inputs.iter().enumerate() {
        let file = format!("f{i:05}.nova");
        std::fs::write(dir.join(&file), input).expect("write a fuzz input");
        files.push(file);
    }
    agree(&dir, &files, 100, 0)
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
        let crlf = if text.contains(&b'\r') {
            text
        } else {
            let mut out = Vec::with_capacity(text.len() + text.len() / 16);
            for &b in &text {
                if b == b'\n' {
                    out.push(b'\r');
                }
                out.push(b);
            }
            out
        };
        let path = dir.join(file);
        std::fs::create_dir_all(path.parent().expect("a parent")).expect("create a directory");
        std::fs::write(&path, crlf).expect("write a corpus file");
    }
    (dir, files)
}

#[test]
fn the_corpus_dumps_agree() {
    let files = corpus();
    // 281 under std/, tests/ and examples/ alone at 5.0c.
    assert!(files.len() >= 290, "only {} corpus files", files.len());
    assert!(files.iter().any(|f| f == "selfhost/src/lexer.nova"));
    agree(&root(), &files, BATCH, 0);
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
    agree_on_inputs("edges", &fuzz::edges());
}

#[test]
fn the_soup_agrees_and_holds_every_lex_error() {
    let mut dumps = String::new();
    for seed in SOUP_SEEDS {
        dumps.push_str(&agree_on_inputs(
            &format!("soup-{seed}"),
            &fuzz::soups(seed, SOUP_COUNT),
        ));
    }
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
        agree_on_inputs(&format!("mutated-{seed}"), &mutations(seed, MUTATION_COUNT));
    }
}

/// `NOVA_SELFHOST_FUZZ=SEED:COUNT`: COUNT inputs of soup and COUNT mutated
/// corpus files from SEED. Unset, as in CI, it does nothing.
#[test]
fn a_longer_search_from_the_environment() {
    let Ok(spec) = std::env::var("NOVA_SELFHOST_FUZZ") else {
        return;
    };
    let (seed, count) = spec
        .split_once(':')
        .and_then(|(s, c)| Some((s.parse::<u64>().ok()?, c.parse::<usize>().ok()?)))
        .expect("NOVA_SELFHOST_FUZZ is SEED:COUNT");
    agree_on_inputs("search-soup", &fuzz::soups(seed, count));
    // Mutations go in batches of 500, each from a seed of its own, so a long
    // search keeps few inputs on disk at a time.
    let mut done = 0;
    while done < count {
        let n = (count - done).min(500);
        agree_on_inputs(
            "search-mutated",
            &mutations(seed.wrapping_add(done as u64), n),
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
}

/// No file at all: both sides refuse it, print nothing and exit 2.
#[test]
fn both_sides_refuse_no_file() {
    let out = compare_batch(&TOKENS, &root(), &[], 2, &nova(), selfhost());
    assert_eq!(out, Ok(String::new()));
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
