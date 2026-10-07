//! `nova fmt`: format Nova source files (spec
//! `docs/superpowers/specs/2026-10-07-phase-3-1-formatter-design.md` §7).

use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use clap::Args;
use nova_fmt::{FileError, FormatError};

#[derive(Args)]
pub struct FmtCmd {
    /// Files or directories to format (default: the project's `src/`).
    paths: Vec<PathBuf>,

    /// Write nothing; print each file that would change, and exit 1 if any
    /// would.
    #[arg(long)]
    check: bool,

    /// Read source from standard input and write it, formatted, to standard
    /// output.
    #[arg(long, conflicts_with = "paths")]
    stdin: bool,
}

/// Run `nova fmt` and return its exit code (spec §7.2): 0 when everything
/// is formatted, 1 when `--check` finds a file that is not, and 2 on any
/// error, which outranks a file that would change.
pub fn run(cmd: FmtCmd) -> i32 {
    let code = if cmd.stdin {
        stdin(cmd.check)
    } else {
        files(&cmd.paths, cmd.check)
    };
    let _ = std::io::stdout().flush();
    code
}

fn stdin(check: bool) -> i32 {
    let mut source = String::new();
    if let Err(e) = std::io::stdin().read_to_string(&mut source) {
        eprintln!("error: reading standard input: {e}");
        return 2;
    }
    match nova_fmt::format_text(&source) {
        Ok(out) if check => i32::from(out != source),
        Ok(out) => {
            print!("{out}");
            0
        }
        Err(e) => {
            report("<stdin>", &e);
            2
        }
    }
}

fn files(paths: &[PathBuf], check: bool) -> i32 {
    let files = match collect(paths) {
        Ok(files) => files,
        Err(message) => {
            eprintln!("error: {message}");
            return 2;
        }
    };
    let mut code = 0;
    for file in &files {
        let shown = file.display().to_string();
        match nova_fmt::format_file(file) {
            Ok(f) if !f.changed() => {}
            Ok(_) if check => {
                println!("would reformat: {shown}");
                code = code.max(1);
            }
            Ok(f) => {
                if let Err(e) = write(file, &f.formatted) {
                    eprintln!("error: writing {shown}: {e}");
                    code = 2;
                }
            }
            Err(FileError::Io(e)) => {
                eprintln!("error: reading {shown}: {e}");
                code = 2;
            }
            Err(FileError::NotUtf8) => {
                eprintln!("error: {shown} is not UTF-8, so it was left unchanged");
                code = 2;
            }
            Err(FileError::Format(e)) => {
                report(&shown, &e);
                code = 2;
            }
        }
    }
    code
}

/// Print why `file` was left unchanged: its diagnostics as `nova check`
/// prints them, or what the self-check found (spec §7.2).
fn report(file: &str, e: &FormatError) {
    match e {
        FormatError::Syntax { rendered, .. } => eprint!("{rendered}"),
        FormatError::Internal { first_difference } => eprintln!(
            "error: nova fmt could not format {file} safely, so it was left unchanged: \
             {first_difference}"
        ),
    }
}

/// The files to format, sorted (spec §7.1): the paths given, each
/// directory searched for `*.nova`. With none, the project's `src/`, or
/// outside a project `src/` if `src/main.nova` exists.
fn collect(paths: &[PathBuf]) -> Result<Vec<PathBuf>, String> {
    let mut files = Vec::new();
    if paths.is_empty() {
        let cwd =
            std::env::current_dir().map_err(|e| format!("reading the current directory: {e}"))?;
        let src = match nova_pm::find_root(&cwd) {
            Some(root) => root.join("src"),
            None if Path::new("src/main.nova").is_file() => PathBuf::from("src"),
            None => {
                return Err("no project here, and no src/main.nova: \
                            name the files or directories to format"
                    .to_owned())
            }
        };
        search(&src, &mut files).map_err(|e| format!("searching {}: {e}", src.display()))?;
    }
    for path in paths {
        if path.is_dir() {
            search(path, &mut files).map_err(|e| format!("searching {}: {e}", path.display()))?;
        } else if path.is_file() {
            files.push(path.clone());
        } else {
            return Err(format!("{} does not exist", path.display()));
        }
    }
    files.sort();
    files.dedup();
    Ok(files)
}

/// Every `*.nova` file under `dir`, but not in `target/`, in a directory
/// whose name begins with `.`, or behind a symbolic link to a directory.
fn search(dir: &Path, files: &mut Vec<PathBuf>) -> std::io::Result<()> {
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let kind = entry.file_type()?;
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().into_owned();
        if kind.is_dir() {
            if name != "target" && !name.starts_with('.') {
                search(&path, files)?;
            }
        } else if name.ends_with(".nova") && (kind.is_file() || path.is_file()) {
            files.push(path);
        }
    }
    Ok(())
}

/// Write `text` over `file` by way of a temporary file in the same
/// directory, renamed over it, so that an interrupted run never leaves half
/// a file (spec §7.1).
fn write(file: &Path, text: &str) -> std::io::Result<()> {
    let name = file
        .file_name()
        .map_or_else(String::new, |n| n.to_string_lossy().into_owned());
    let tmp = file.with_file_name(format!(".{name}.nova-fmt.tmp"));
    std::fs::write(&tmp, text)?;
    std::fs::rename(&tmp, file).inspect_err(|_| {
        let _ = std::fs::remove_file(&tmp);
    })
}
