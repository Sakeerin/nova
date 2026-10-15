//! The WASM target, end to end (spec
//! `docs/superpowers/specs/2026-10-14-phase-4-0-foundations-design.md`).

use std::path::{Path, PathBuf};

use assert_cmd::assert::Assert;
use assert_cmd::Command;

fn nova() -> Command {
    Command::cargo_bin("nova").expect("nova binary builds")
}

/// A fresh, empty directory with a fixed name, so each run replaces the
/// last.
fn fresh(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("nova-wasm-test-{name}"));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("create the test directory");
    dir
}

fn write(dir: &Path, files: &[(&str, &str)]) {
    for (path, text) in files {
        let path = dir.join(path);
        std::fs::create_dir_all(path.parent().expect("a parent")).expect("create a directory");
        std::fs::write(path, text).expect("write a file");
    }
}

/// `nova ARGS` in `dir`, with a fresh `NOVA_HOME` beside it, no colour and
/// no backtrace.
fn nova_in(dir: &Path, args: &[&str]) -> Assert {
    let home = dir.with_file_name(format!(
        "{}-home",
        dir.file_name().expect("a name").to_string_lossy()
    ));
    let _ = std::fs::remove_dir_all(&home);
    nova()
        .current_dir(dir)
        .env("NOVA_HOME", &home)
        .env("NO_COLOR", "1")
        .env("RUST_BACKTRACE", "0")
        .args(args)
        .assert()
}

fn stdout(a: &Assert) -> String {
    String::from_utf8_lossy(&a.get_output().stdout).replace("\r\n", "\n")
}

fn stderr(a: &Assert) -> String {
    String::from_utf8_lossy(&a.get_output().stderr).replace("\r\n", "\n")
}

/// A program WASM can build (spec §3.1).
#[test]
fn check_for_wasm_accepts_a_program() {
    let dir = fresh("check-ok");
    write(
        &dir,
        &[(
            "main.nova",
            "fn main() {\n    let n = 6 * 7\n    println(\"${n}\")\n}\n",
        )],
    );
    let a = nova_in(&dir, &["check", "--target", "wasm", "main.nova"]).success();
    assert_eq!(stdout(&a), "ok: main.nova\n");
}

/// `extern` is E0090 on WASM, and fine natively (spec §3.3).
#[test]
fn check_for_wasm_refuses_an_extern_block() {
    let dir = fresh("check-extern");
    write(
        &dir,
        &[(
            "main.nova",
            "extern \"C\" {\n    fn abs(x: Int) -> Int\n}\nfn main() { println(\"${abs(-3)}\") }\n",
        )],
    );
    let a = nova_in(&dir, &["check", "--target", "wasm", "main.nova"]).failure();
    assert!(
        stderr(&a).contains("error[E0090]: extern functions are not available on target wasm"),
        "{}",
        stderr(&a)
    );
    nova_in(&dir, &["check", "--target", "native", "main.nova"]).success();
    nova_in(&dir, &["check", "main.nova"]).success();
}

/// A native-only module is not compiled for WASM, so its names do not
/// resolve (spec §3.2).
#[test]
fn check_for_wasm_has_no_native_only_module() {
    let dir = fresh("check-process");
    write(
        &dir,
        &[("main.nova", "fn main() { println(\"${args().len()}\") }\n")],
    );
    let a = nova_in(&dir, &["check", "--target", "wasm", "main.nova"]).failure();
    assert!(
        stderr(&a).contains("error[E0001]: cannot find function `args` in this scope"),
        "{}",
        stderr(&a)
    );
    nova_in(&dir, &["check", "main.nova"]).success();
}

/// The memory builtins are `std/rt`'s: a user module cannot call one, even
/// on WASM (spec §3.3).
#[test]
fn a_user_module_cannot_call_a_memory_builtin() {
    let dir = fresh("check-mem");
    write(
        &dir,
        &[("main.nova", "fn main() { println(\"${mem_load64(8)}\") }\n")],
    );
    let a = nova_in(&dir, &["check", "--target", "wasm", "main.nova"]).failure();
    assert!(
        stderr(&a).contains("error[E0001]: cannot find function `mem_load64` in this scope"),
        "{}",
        stderr(&a)
    );
}

/// A program that needs a runtime function WASM lacks is E0091, from
/// `check` as from `build` (spec §3.3).
#[test]
fn check_for_wasm_reports_a_runtime_function_wasm_lacks() {
    let dir = fresh("check-async");
    write(
        &dir,
        &[("main.nova", "async fn main() { println(\"a\") }\n")],
    );
    let a = nova_in(&dir, &["check", "--target", "wasm", "main.nova"]).failure();
    assert!(
        stderr(&a).contains(
            "error[E0091]: this program needs `task_block_on`, which target wasm does not have"
        ),
        "{}",
        stderr(&a)
    );
    nova_in(&dir, &["check", "main.nova"]).success();
}

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
}

/// `node`, or `None` when it is not on `PATH`, in which case the test
/// passes without running; under `NOVA_REQUIRE_NODE=1`, as in CI, a missing
/// `node` fails it instead (spec §9.1).
fn node() -> Option<Command> {
    let found = std::process::Command::new("node")
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

/// Build `file` for WASM into `dir/dist`, which must succeed.
fn build_wasm(dir: &Path, file: &Path) -> PathBuf {
    let dist = dir.join("dist");
    let file = file.to_string_lossy().into_owned();
    let out = dist.to_string_lossy().into_owned();
    nova_in(dir, &["build", "--target", "wasm", &file, "-o", &out]).success();
    dist
}

/// `node dist/app.js`, with no backtrace from `nova` involved.
fn run_wasm(mut node: Command, dist: &Path) -> Assert {
    node.arg(dist.join("app.js")).assert()
}

/// `dist/` holds the page, the shim and the module, and `nova build`
/// names it (spec §7).
#[test]
fn build_for_wasm_writes_dist() {
    let dir = fresh("build-dist");
    write(&dir, &[("main.nova", "fn main() { println(\"hi\") }\n")]);
    let a = nova_in(&dir, &["build", "--target", "wasm", "main.nova"]).success();
    assert_eq!(stdout(&a), "built dist\n");
    let dist = dir.join("dist");
    let page = std::fs::read_to_string(dist.join("index.html")).expect("index.html");
    assert!(page.contains("import init from \"./app.js\";"), "{page}");
    let js = std::fs::read_to_string(dist.join("app.js")).expect("app.js");
    assert!(js.contains("export default async function init("), "{js}");
    let wasm = std::fs::read(dist.join("app.wasm")).expect("app.wasm");
    assert_eq!(&wasm[..4], b"\0asm");
}

/// `nova bundle` is `nova build --target wasm`, and `-o` names the
/// directory; anything else in it stays (spec §3.1, §7).
#[test]
fn bundle_is_build_for_wasm() {
    let dir = fresh("bundle");
    write(
        &dir,
        &[
            ("main.nova", "fn main() { println(\"hi\") }\n"),
            ("out/keep.txt", "kept"),
        ],
    );
    let a = nova_in(&dir, &["bundle", "main.nova", "-o", "out"]).success();
    assert_eq!(stdout(&a), "built out\n");
    for name in ["index.html", "app.js", "app.wasm", "keep.txt"] {
        assert!(dir.join("out").join(name).is_file(), "{name}");
    }
}

/// In a project, `dist/` is at the project's root (spec §7).
#[test]
fn a_project_builds_into_dist_at_its_root() {
    let dir = fresh("project");
    write(
        &dir,
        &[
            (
                "nova.toml",
                "[package]\nname = \"demo\"\nversion = \"0.1.0\"\nedition = \"2026\"\n",
            ),
            ("src/main.nova", "fn main() { println(\"hi\") }\n"),
        ],
    );
    nova_in(&dir.join("src"), &["build", "--target", "wasm"]).success();
    assert!(dir.join("dist").join("app.wasm").is_file());
}

/// `--release` names the LLVM backend, which WASM does not use.
#[test]
fn release_does_not_apply_to_wasm() {
    let dir = fresh("release");
    write(&dir, &[("main.nova", "fn main() { println(\"hi\") }\n")]);
    let a = nova_in(
        &dir,
        &["build", "--target", "wasm", "--release", "main.nova"],
    )
    .failure();
    assert!(
        stderr(&a).contains("`--release` does not apply to `--target wasm`"),
        "{}",
        stderr(&a)
    );
}

/// The gate: hello world prints under Node what its native build prints
/// (spec §1).
#[test]
fn hello_world_runs_under_node() {
    let Some(node) = node() else { return };
    let dir = fresh("hello");
    let dist = build_wasm(
        &dir,
        &repo_root().join("examples/01-hello-world/src/main.nova"),
    );
    let a = run_wasm(node, &dist).success();
    assert_eq!(stdout(&a), "Hello, World!\n");
}

/// The gate's size check: hello world's module and shim are under 30 KB
/// gzipped (spec §9.5).
#[test]
fn hello_world_is_under_30_kb_gzipped() {
    use std::io::Write;
    let dir = fresh("hello-size");
    let dist = build_wasm(
        &dir,
        &repo_root().join("examples/01-hello-world/src/main.nova"),
    );
    let mut total = 0;
    for name in ["app.wasm", "app.js"] {
        let bytes = std::fs::read(dist.join(name)).expect("read a dist file");
        let mut gz = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::best());
        gz.write_all(&bytes).expect("compress");
        total += gz.finish().expect("compress").len();
    }
    println!("hello world: {total} bytes gzipped");
    assert!(total < 30 * 1024, "{total} bytes gzipped");
}

/// A panic ends the program with its message and an abort's status, 134;
/// what it printed first stays (spec §6).
#[test]
fn a_panic_exits_134_with_its_message() {
    let Some(node) = node() else { return };
    let dir = fresh("panic");
    write(
        &dir,
        &[(
            "main.nova",
            "fn main() {\n    println(\"before\")\n    panic(\"boom\")\n}\n",
        )],
    );
    let dist = build_wasm(&dir, Path::new("main.nova"));
    let a = run_wasm(node, &dist).code(134);
    assert_eq!(stdout(&a), "before\n");
    assert_eq!(stderr(&a), "nova: panic: boom\n");
}

/// A trap, here a division by zero, ends it with a trap's status, 132.
#[test]
fn a_trap_exits_132() {
    let Some(node) = node() else { return };
    let dir = fresh("trap");
    write(
        &dir,
        &[(
            "main.nova",
            "fn d(a: Int, b: Int) -> Int { a / b }\nfn main() { println(\"${d(1, 0)}\") }\n",
        )],
    );
    let dist = build_wasm(&dir, Path::new("main.nova"));
    let a = run_wasm(node, &dist).code(132);
    assert_eq!(stderr(&a), "nova: trap: divide by zero\n");
}
