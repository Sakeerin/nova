//! `init({ wasm, print })` (spec
//! `docs/superpowers/specs/2026-11-28-phase-6-1-the-playground-design.md`
//! §3.5, §5 test 3): with `print(fd, text)`, the shim hands it each write
//! to standard output (1) and error (2), whole and in order, and Node's
//! streams get nothing; without it, `node app.js` prints as before. The
//! program is compiled by [`nova_playground::compile`] on the host.

use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::time::{Duration, Instant};

use nova_playground::{compile, Outcome};

/// Four writes, to both streams, two of them not ending a line.
const SOURCE: &str =
    "fn main() {\n    print(\"a\")\n    println(\"b\")\n    eprintln(\"c\")\n    print(\"d\")\n}\n";

/// A fresh, empty directory with a fixed name, so each run replaces the
/// last.
fn fresh(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("nova-playground-test-{name}"));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("create the test directory");
    dir
}

/// `node`, or `None` when it is not on `PATH`, in which case the test
/// passes without running; under `NOVA_REQUIRE_NODE=1`, as in CI, a missing
/// `node` fails it instead (as `nova-cli`'s `tests/wasm_tests.rs` does).
fn node() -> Option<Command> {
    let found = Command::new("node")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);
    if found {
        return Some(Command::new("node"));
    }
    assert_ne!(
        std::env::var("NOVA_REQUIRE_NODE").as_deref(),
        Ok("1"),
        "NOVA_REQUIRE_NODE=1, but `node` is not on PATH"
    );
    eprintln!("skipped: `node` is not on PATH");
    None
}

/// `app.js`, `app.wasm` and a `package.json` saying they are ES modules,
/// for [`SOURCE`], in `dir`.
fn build(dir: &Path) {
    let Outcome::Compiled { wasm, js, .. } = compile(SOURCE) else {
        panic!("not compiled: {:?}", compile(SOURCE));
    };
    std::fs::write(dir.join("app.js"), js).expect("write app.js");
    std::fs::write(dir.join("app.wasm"), wasm).expect("write app.wasm");
    std::fs::write(dir.join("package.json"), "{\"type\":\"module\"}\n")
        .expect("write package.json");
}

/// `node`'s run, killed after a minute: a shim that regresses may hang
/// rather than fail. Its output is a few bytes, which no pipe holds back.
fn run(mut node: Command, dir: &Path, script: &str) -> Output {
    let mut child = node
        .arg(dir.join(script))
        .current_dir(dir)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("start node");
    let start = Instant::now();
    while child.try_wait().expect("wait for node").is_none() {
        if start.elapsed() > Duration::from_secs(60) {
            let _ = child.kill();
            panic!("node ran for more than a minute");
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    child.wait_with_output().expect("node's output")
}

fn text(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).replace("\r\n", "\n")
}

#[test]
fn print_takes_every_write_whole_and_in_order() {
    let Some(node) = node() else {
        return;
    };
    let dir = fresh("print-hook");
    build(&dir);
    // The hook's calls, printed once `init` resolves: standard output holds
    // nothing else when Node's streams got none of the writes.
    std::fs::write(
        dir.join("hook.mjs"),
        "import { readFileSync } from \"node:fs\";\n\
         import init from \"./app.js\";\n\
         const writes = [];\n\
         const wasm = readFileSync(new URL(\"app.wasm\", import.meta.url));\n\
         await init({ wasm, print: (fd, text) => writes.push([fd, text]) });\n\
         process.stdout.write(JSON.stringify(writes));\n",
    )
    .expect("write hook.mjs");
    let out = run(node, &dir, "hook.mjs");
    assert!(out.status.success(), "{}", text(&out.stderr));
    assert_eq!(
        text(&out.stdout),
        "[[1,\"a\"],[1,\"b\\n\"],[2,\"c\\n\"],[1,\"d\"]]"
    );
    assert_eq!(text(&out.stderr), "");
}

#[test]
fn without_print_node_app_js_prints_as_before() {
    let Some(node) = node() else {
        return;
    };
    let dir = fresh("print-hook-none");
    build(&dir);
    let out = run(node, &dir, "app.js");
    assert!(out.status.success(), "{}", text(&out.stderr));
    assert_eq!(text(&out.stdout), "ab\nd");
    assert_eq!(text(&out.stderr), "c\n");
}
