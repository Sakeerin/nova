//! The tests of the self-hosted lexer (spec
//! `docs/superpowers/specs/2026-10-30-phase-5-1-the-lexer-the-parser-and-the-ast-design.md`
//! §3 and §6.2, ADR 0051):
//!
//! - S0's hidden `nova parse --dump-tokens FILE…`: absent from `--help`, a
//!   header per file, and `IOERR` with the exit code 2 for a file it cannot
//!   read.
//!
//! Each test writes its inputs to fixed directories under the system temp
//! directory.

use std::env::consts::EXE_SUFFIX;
use std::path::PathBuf;
use std::process::Command;

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
