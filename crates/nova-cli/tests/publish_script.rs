//! The publish script (spec
//! `docs/superpowers/specs/2026-10-12-phase-3-6a-release-preparation-design.md`
//! §5.2, §11.2): `.github/scripts/publish-crates.sh`, run with `bash`
//! against a fake `cargo` and a sparse index served by the loopback server.
//! Nothing reaches the internet, and no real cargo publishes anything.
//!
//! The fake `cargo` appends its arguments, one line per run, to
//! `cargo.log`, and answers as the test's `answers` file says, one line per
//! crate:
//! - no line, or `<crate> ok`: it succeeds;
//! - `<crate> 429 <n>`: its first `n` runs for that crate fail as crates.io
//!   refuses for its rate limit on new crates, then it succeeds;
//! - `<crate> fail`: every run for that crate fails for another reason.
//!
//! It also fails when `CARGO_REGISTRY_TOKEN` did not reach it.

#![cfg(unix)]

#[path = "../../nova-index/tests/support/mod.rs"]
mod support;

use std::collections::HashMap;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Command;

use support::{Response, Server};

const TOKEN: &str = "secret-token-value";
const VERSION: &str = "0.3.0";

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
}

fn script() -> PathBuf {
    root()
        .join(".github")
        .join("scripts")
        .join("publish-crates.sh")
}

/// The script's `crates=( … )` list, one crate per line, so that the
/// order lives in the script alone.
fn publish_list() -> Vec<String> {
    let text = std::fs::read_to_string(script()).expect("read publish-crates.sh");
    let mut lines = text.lines().map(str::trim);
    assert!(lines.any(|line| line == "crates=("), "no `crates=(` line");
    let list: Vec<String> = lines
        .take_while(|line| *line != ")")
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .map(str::to_string)
        .collect();
    assert_eq!(list.len(), 19, "{list:?}");
    list
}

/// A fresh, empty directory with a fixed name, so each run replaces the last.
fn fresh(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("nova-publish-script-{name}"));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("create the test directory");
    dir
}

/// The fake `cargo` (see the module's comment).
const FAKE_CARGO: &str = r#"#!/bin/sh
dir=$FAKE_CARGO_DIR
echo "$*" >> "$dir/cargo.log"
crate=
previous=
for arg in "$@"; do
  if [ "$previous" = "-p" ]; then crate=$arg; fi
  previous=$arg
done
if [ -z "${CARGO_REGISTRY_TOKEN:-}" ]; then
  echo "fake cargo: no CARGO_REGISTRY_TOKEN" >&2
  exit 3
fi
answer=$(awk -v crate="$crate" '$1 == crate { $1 = ""; sub(/^ /, ""); print; exit }' "$dir/answers")
runs=$(grep -cx "publish --locked -p $crate" "$dir/cargo.log")
case $answer in
  "429 "*)
    if [ "$runs" -le "${answer#429 }" ]; then
      echo "error: failed to publish to registry at https://crates.io" >&2
      echo "" >&2
      echo "Caused by:" >&2
      echo "  the remote server responded with an error (status 429 Too Many Requests): You have published too many new crates in a short period of time. Please try again after Mon, 12 Oct 2026 12:00:00 GMT or email help@crates.io to have your limit increased." >&2
      exit 101
    fi
    ;;
  fail)
    echo "error: failed to publish to registry at https://crates.io" >&2
    echo "" >&2
    echo "Caused by:" >&2
    echo "  the remote server responded with an error (status 400 Bad Request): fake cargo refused $crate" >&2
    exit 101
    ;;
esac
echo "   Uploading $crate (fake)"
echo "   Published $crate (fake)" >&2
"#;

/// A sparse index on the loopback server: each `(path, status, body)`,
/// and 404 for any other path.
fn index(files: &[(String, u16, String)]) -> Server {
    let files: HashMap<String, (u16, String)> = files
        .iter()
        .map(|(path, status, body)| (path.clone(), (*status, body.clone())))
        .collect();
    Server::start(move |request| match files.get(&request.path) {
        Some((status, body)) => Response::with_status(*status, body.clone()),
        None => Response::status(404),
    })
}

/// A crate's path in the sparse index: every listed name has four
/// characters or more, so `<chars 1-2>/<chars 3-4>/<name>`.
fn index_path(name: &str) -> String {
    assert!(name.len() >= 4, "{name}");
    format!("/{}/{}/{name}", &name[0..2], &name[2..4])
}

/// One line of a crate's index file, as crates.io writes it.
fn index_line(name: &str, version: &str, yanked: bool) -> String {
    format!(
        r#"{{"name":"{name}","vers":"{version}","deps":[],"cksum":"{}","features":{{}},"yanked":{yanked}}}"#,
        "0".repeat(64)
    )
}

/// A crate's index file listing `versions`.
fn index_file(name: &str, versions: &[(&str, bool)]) -> (String, u16, String) {
    let body: String = versions
        .iter()
        .map(|(version, yanked)| index_line(name, version, *yanked) + "\n")
        .collect();
    (index_path(name), 200, body)
}

struct Run {
    success: bool,
    code: Option<i32>,
    stdout: String,
    stderr: String,
    /// The fake cargo's runs, one line of arguments each.
    log: Vec<String>,
}

impl Run {
    fn output(&self) -> String {
        format!("--- stdout\n{}--- stderr\n{}", self.stdout, self.stderr)
    }
}

/// `bash publish-crates.sh ARGS` in a fresh directory called `name`, with
/// the fake cargo answering as `answers` says, `index_url` as the sparse
/// index, no wait after a rate limit, and the token set. Whatever happens,
/// the token's value is printed nowhere.
fn publish(name: &str, args: &[&str], answers: &str, index_url: &str) -> Run {
    let dir = fresh(name);
    let cargo = dir.join("cargo");
    std::fs::write(&cargo, FAKE_CARGO).expect("write the fake cargo");
    std::fs::set_permissions(&cargo, std::fs::Permissions::from_mode(0o755))
        .expect("make the fake cargo executable");
    std::fs::write(dir.join("answers"), answers).expect("write the answers");
    // On macOS, the system's bash 3.2, which a Homebrew bash first on PATH
    // would hide: the script must run under both.
    let bash = if cfg!(target_os = "macos") {
        "/bin/bash"
    } else {
        "bash"
    };
    let mut command = Command::new(bash);
    command
        .arg(script())
        .args(args)
        .current_dir(&dir)
        .env("CARGO", &cargo)
        .env("FAKE_CARGO_DIR", &dir)
        .env("NOVA_PUBLISH_INDEX", index_url)
        .env("NOVA_PUBLISH_WAIT", "0")
        .env("CARGO_REGISTRY_TOKEN", TOKEN);
    // Loopback only: a proxy in the environment must not see the requests.
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
    let output = command.output().expect("run bash publish-crates.sh");
    let run = Run {
        success: output.status.success(),
        code: output.status.code(),
        stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
        stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
        log: read_log(&dir),
    };
    assert!(
        !run.stdout.contains(TOKEN) && !run.stderr.contains(TOKEN),
        "the token was printed:\n{}",
        run.output()
    );
    run
}

fn read_log(dir: &Path) -> Vec<String> {
    match std::fs::read_to_string(dir.join("cargo.log")) {
        Ok(text) => text.lines().map(str::to_string).collect(),
        Err(_) => Vec::new(),
    }
}

fn publish_args(crates: &[String]) -> Vec<String> {
    crates
        .iter()
        .map(|crate_name| format!("publish --locked -p {crate_name}"))
        .collect()
}

/// The summary's lines, after its heading.
fn summary(run: &Run) -> Vec<String> {
    let mut lines = run.stdout.lines();
    assert!(
        lines.any(|line| line == format!("summary for {VERSION}:")),
        "no summary:\n{}",
        run.output()
    );
    lines.map(str::to_string).collect()
}

fn summary_line(crate_name: &str, what: &str) -> String {
    format!("  {crate_name:<24} {what}")
}

#[test]
fn publishes_every_crate_in_the_lists_order() {
    let list = publish_list();
    let server = index(&[]);
    let run = publish("in-order", &[VERSION], "", &server.url);
    assert!(run.success, "{}", run.output());
    assert_eq!(run.log, publish_args(&list), "{}", run.output());
    let published: Vec<String> = run
        .stdout
        .lines()
        .filter(|line| line.starts_with("published "))
        .map(str::to_string)
        .collect();
    let expected: Vec<String> = list
        .iter()
        .map(|crate_name| format!("published {crate_name} {VERSION}"))
        .collect();
    assert_eq!(published, expected, "{}", run.output());
    let expected: Vec<String> = list
        .iter()
        .map(|crate_name| summary_line(crate_name, "published"))
        .collect();
    assert_eq!(summary(&run), expected, "{}", run.output());
    // Each crate's index file was asked for once, before its publish.
    let asked: Vec<String> = server
        .requests()
        .iter()
        .map(|request| format!("{} {}", request.method, request.path))
        .collect();
    let expected: Vec<String> = list
        .iter()
        .map(|crate_name| format!("GET {}", index_path(crate_name)))
        .collect();
    assert_eq!(asked, expected);
}

#[test]
fn skips_a_crate_the_index_lists_at_the_version_and_publishes_one_listed_at_others() {
    let list = publish_list();
    let (at_version, yanked_at_version, at_others) = (&list[0], &list[1], &list[2]);
    let server = index(&[
        index_file(at_version, &[("0.2.0", false), (VERSION, false)]),
        // A yanked version is still taken.
        index_file(yanked_at_version, &[(VERSION, true)]),
        // A prerelease of the same numbers, and longer versions that start
        // with them, are other versions.
        index_file(
            at_others,
            &[
                ("0.2.0", false),
                ("0.3.0-alpha.1", false),
                ("0.3.0-rc.1", false),
                ("0.3.01", false),
                ("10.3.0", false),
            ],
        ),
    ]);
    let run = publish("skips", &[VERSION], "", &server.url);
    assert!(run.success, "{}", run.output());
    assert_eq!(run.log, publish_args(&list[2..]), "{}", run.output());
    for skipped in [at_version, yanked_at_version] {
        assert!(
            run.stdout
                .lines()
                .any(|line| line == format!("skip {skipped} {VERSION} (already on crates.io)")),
            "{}",
            run.output()
        );
    }
    assert!(
        run.stdout
            .lines()
            .any(|line| line == format!("published {at_others} {VERSION}")),
        "{}",
        run.output()
    );
    let summary = summary(&run);
    assert_eq!(summary[0], summary_line(at_version, "skipped"));
    assert_eq!(summary[1], summary_line(yanked_at_version, "skipped"));
    assert_eq!(summary[2], summary_line(at_others, "published"));
}

#[test]
fn waits_out_the_rate_limit_retries_the_same_crate_then_goes_on() {
    let list = publish_list();
    let limited = &list[3];
    let server = index(&[]);
    let run = publish(
        "rate-limit",
        &[VERSION],
        &format!("{limited} 429 2\n"),
        &server.url,
    );
    assert!(run.success, "{}", run.output());
    let mut expected = publish_args(&list[..4]);
    expected.insert(3, expected[3].clone());
    expected.insert(3, expected[3].clone());
    expected.extend(publish_args(&list[4..]));
    assert_eq!(run.log, expected, "{}", run.output());
    let waits = run
        .stdout
        .lines()
        .filter(|line| *line == "rate limited: waiting 0 s")
        .count();
    assert_eq!(waits, 2, "{}", run.output());
    assert_eq!(summary(&run)[3], summary_line(limited, "published"));
}

#[test]
fn gives_up_after_thirty_rate_limited_attempts() {
    let list = publish_list();
    let limited = &list[2];
    let server = index(&[]);
    let run = publish(
        "rate-limit-30",
        &[VERSION],
        &format!("{limited} 429 30\n"),
        &server.url,
    );
    assert!(!run.success, "{}", run.output());
    let mut expected = publish_args(&list[..2]);
    for _ in 0..30 {
        expected.push(format!("publish --locked -p {limited}"));
    }
    assert_eq!(run.log, expected, "{}", run.output());
    assert!(run.stderr.contains("30 attempts"), "{}", run.output());
}

#[test]
fn stops_at_any_other_failure_with_cargos_output_and_publishes_nothing_after_it() {
    let list = publish_list();
    let failing = &list[5];
    let server = index(&[]);
    let run = publish(
        "failure",
        &[VERSION],
        &format!("{failing} fail\n"),
        &server.url,
    );
    assert!(!run.success, "{}", run.output());
    // Once: no retry, and no crate after it.
    assert_eq!(run.log, publish_args(&list[..6]), "{}", run.output());
    assert!(
        run.output().contains(&format!(
            "(status 400 Bad Request): fake cargo refused {failing}"
        )),
        "{}",
        run.output()
    );
    assert!(!run.stdout.contains("rate limited"), "{}", run.output());
    let summary = summary(&run);
    assert_eq!(summary[4], summary_line(&list[4], "published"));
    assert_eq!(summary[5], summary_line(failing, "failed"));
    assert_eq!(summary[6], summary_line(&list[6], "not tried"));
    // The index was asked for it and the five before it, and no other.
    assert_eq!(server.requests().len(), 6);
}

#[test]
fn stops_when_the_index_answers_neither_200_nor_404() {
    let list = publish_list();
    let broken = &list[4];
    let server = index(&[(index_path(broken), 500, "oops".to_string())]);
    let run = publish("index-500", &[VERSION], "", &server.url);
    assert!(!run.success, "{}", run.output());
    assert_eq!(run.log, publish_args(&list[..4]), "{}", run.output());
    let url = format!("{}{}", server.url, index_path(broken));
    assert!(run.stderr.contains(&url), "{}", run.output());
    assert!(run.stderr.contains("500"), "{}", run.output());
}

#[test]
fn stops_when_the_index_cannot_be_reached() {
    let list = publish_list();
    // Nothing listens on loopback port 1.
    let run = publish("index-down", &[VERSION], "", "http://127.0.0.1:1");
    assert!(!run.success, "{}", run.output());
    assert!(run.log.is_empty(), "{}", run.output());
    assert!(
        run.stderr
            .contains(&format!("http://127.0.0.1:1{}", index_path(&list[0]))),
        "{}",
        run.output()
    );
}

#[test]
fn never_prints_the_token() {
    // `publish` checks every run; this one goes through a rate limit and a
    // failure, the paths that print cargo's output.
    let list = publish_list();
    let server = index(&[]);
    let answers = format!("{} 429 1\n{} fail\n", list[0], list[1]);
    let run = publish("token", &[VERSION], &answers, &server.url);
    assert!(!run.success, "{}", run.output());
    assert_eq!(run.log.len(), 3, "{}", run.output());
}

#[test]
fn refuses_a_missing_or_malformed_version_before_anything_runs() {
    let server = index(&[]);
    let run = publish("no-version", &[], "", &server.url);
    assert_eq!(run.code, Some(2), "{}", run.output());
    assert!(
        run.stderr.contains("usage: publish-crates.sh VERSION"),
        "{}",
        run.output()
    );
    for bad in ["v0.3.0", "0.3", "0.3.0; true", ""] {
        let run = publish("bad-version", &[bad], "", &server.url);
        assert_eq!(run.code, Some(2), "{bad}: {}", run.output());
        assert!(run.log.is_empty(), "{bad}: {}", run.output());
    }
    assert!(server.requests().is_empty());
}
