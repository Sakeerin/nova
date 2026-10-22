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
    nova_at(dir, &home).args(args).assert()
}

/// `nova` in `dir`, with a fresh `NOVA_HOME` at `home`, no colour and no
/// backtrace, to be given its arguments.
fn nova_at(dir: &Path, home: &Path) -> Command {
    let _ = std::fs::remove_dir_all(home);
    let mut cmd = nova();
    cmd.current_dir(dir)
        .env("NOVA_HOME", home)
        .env("NO_COLOR", "1")
        .env("RUST_BACKTRACE", "0");
    cmd
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

/// A user's `extern "js"` block is E0090 on WASM, as any `extern` block
/// is, and natively its ABI is E0900: only std declares JS functions (spec
/// 4.2 §3.1).
#[test]
fn a_users_extern_js_is_refused_on_both_targets() {
    let dir = fresh("check-extern-js");
    write(
        &dir,
        &[(
            "main.nova",
            "extern \"js\" {\n    fn now() -> Int\n}\nfn main() { println(\"${now()}\") }\n",
        )],
    );
    let wasm = nova_in(&dir, &["check", "--target", "wasm", "main.nova"]).failure();
    assert!(stderr(&wasm).contains("error[E0090]"), "{}", stderr(&wasm));
    let native = nova_in(&dir, &["check", "main.nova"]).failure();
    assert!(
        stderr(&native).contains(
            "error[E0900]: extern ABI `\"js\"` is not supported; only the C ABI is supported"
        ),
        "{}",
        stderr(&native)
    );
}

/// A name from a std module WASM does not compile is E0092 there, naming
/// the module, and the same program checks natively (spec 4.1 §3.2).
fn missing_on_wasm(name: &str, source: &str, error: &str) {
    let dir = fresh(&format!("missing-{name}"));
    write(&dir, &[("main.nova", source)]);
    let a = nova_in(&dir, &["check", "--target", "wasm", "main.nova"]).failure();
    assert!(
        stderr(&a).contains(&format!("error[E0092]: {error}")),
        "{}",
        stderr(&a)
    );
    assert!(!stderr(&a).contains("E0001"), "{}", stderr(&a));
    assert!(!stderr(&a).contains("E0900"), "{}", stderr(&a));
    nova_in(&dir, &["check", "main.nova"]).success();
}

/// A function of a native-only module (spec 4.1 §3.2).
#[test]
fn a_native_only_function_is_e0092_on_wasm() {
    missing_on_wasm(
        "function",
        "fn main() { println(\"${args().len()}\") }\n",
        "`args` is in `std/process`, which target wasm does not have",
    );
}

/// A type of a native-only module, in a signature.
#[test]
fn a_native_only_type_is_e0092_on_wasm() {
    missing_on_wasm(
        "type",
        "fn size(f: File) -> Int { 0 }\nfn main() { println(\"x\") }\n",
        "`File` is in `std/fs`, which target wasm does not have",
    );
}

/// A qualified call into a native-only module's type, which was E0900.
#[test]
fn a_native_only_qualified_call_is_e0092_on_wasm() {
    missing_on_wasm(
        "qualified",
        "fn main() {\n    let o = OpenOptions::reading()\n    println(\"x\")\n}\n",
        "`OpenOptions` is in `std/fs`, which target wasm does not have",
    );
}

/// A record literal of a native-only module's record.
#[test]
fn a_native_only_record_literal_is_e0092_on_wasm() {
    missing_on_wasm(
        "record",
        "fn main() {\n    let s = Stdin {}\n    println(\"x\")\n}\n",
        "`Stdin` is in `std/io`, which target wasm does not have",
    );
}

/// A variant of a native-only module's type, in a pattern.
#[test]
fn a_native_only_variant_pattern_is_e0092_on_wasm() {
    missing_on_wasm(
        "pattern",
        "fn name(k: IoErrorKind) -> String {\n    match k {\n        IoErrorKind::NotFound => \"nf\"\n        _ => \"other\"\n    }\n}\nfn main() { println(\"x\") }\n",
        "`IoErrorKind` is in `std/io`, which target wasm does not have",
    );
}

/// A variant with a payload of a native-only module's type, in a pattern:
/// the name it binds is not E0001 either.
#[test]
fn a_native_only_payload_pattern_is_e0092_on_wasm() {
    missing_on_wasm(
        "payload-pattern",
        "fn name(m: Method) -> String {\n    match m {\n        Method::Unknown(s) => s\n        _ => \"known\"\n    }\n}\nfn main() { println(\"x\") }\n",
        "`Method` is in `std/http`, which target wasm does not have",
    );
}

/// A program imports `std/dom` by name, on WASM; natively the import is
/// E0092, and without it the module's names are unknown (spec 4.2 §3.2).
#[test]
fn dom_is_imported_by_name_and_only_on_wasm() {
    let dir = fresh("import-dom");
    write(
        &dir,
        &[
            (
                "main.nova",
                "import dom::{Element}\nfn f(e: Element) -> Int { 0 }\nfn main() { println(\"x\") }\n",
            ),
            (
                "bare.nova",
                "fn f(e: Element) -> Int { 0 }\nfn main() { println(\"x\") }\n",
            ),
        ],
    );
    nova_in(&dir, &["check", "--target", "wasm", "main.nova"]).success();
    let native = nova_in(&dir, &["check", "main.nova"]).failure();
    assert!(
        stderr(&native)
            .contains("error[E0092]: `dom` is a std module, which target native does not have"),
        "{}",
        stderr(&native)
    );
    let bare = nova_in(&dir, &["check", "--target", "wasm", "bare.nova"]).failure();
    assert!(
        stderr(&bare).contains("error[E0001]: cannot find type `Element`"),
        "{}",
        stderr(&bare)
    );
}

/// A native program imports `std/ui/ssr` by name; on WASM the import is
/// E0092, and its names, never in any prelude, are unknown there without it,
/// as natively (spec 4.6 §6.1, §8.4).
#[test]
fn ssr_is_imported_by_name_and_only_natively() {
    let dir = fresh("import-ssr");
    write(
        &dir,
        &[
            (
                "main.nova",
                "import ui\nimport ui::ssr::{Page, page}\nfn f(p: Page) -> Int { 0 }\nfn main() { println(\"x\") }\n",
            ),
            (
                "bare.nova",
                "fn main() {\n    let p = page\n    println(\"x\")\n}\n",
            ),
        ],
    );
    nova_in(&dir, &["check", "main.nova"]).success();
    let wasm = nova_in(&dir, &["check", "--target", "wasm", "main.nova"]).failure();
    assert!(
        stderr(&wasm)
            .contains("error[E0092]: `ui::ssr` is a std module, which target wasm does not have"),
        "{}",
        stderr(&wasm)
    );
    for target in ["native", "wasm"] {
        let bare = nova_in(&dir, &["check", "--target", target, "bare.nova"]).failure();
        assert!(
            stderr(&bare).contains("error[E0001]: cannot find `page` in this scope"),
            "{target}: {}",
            stderr(&bare)
        );
    }
}

/// A module of the program under a std module's name is E0093, a file
/// beside a loose program's or a package's (spec 4.2 §3.2).
#[test]
fn a_module_under_a_std_name_is_e0093() {
    let dir = fresh("std-name");
    write(
        &dir,
        &[
            ("loose/main.nova", "import ui\nfn main() { hi() }\n"),
            ("loose/ui.nova", "pub fn hi() { println(\"hi\") }\n"),
            (
                "app/nova.toml",
                "[package]\nname = \"app\"\nversion = \"0.1.0\"\nedition = \"2026\"\n",
            ),
            ("app/src/main.nova", "import dom\nfn main() { hi() }\n"),
            ("app/src/dom.nova", "pub fn hi() { println(\"hi\") }\n"),
        ],
    );
    let loose = nova_in(&dir.join("loose"), &["check", "main.nova"]).failure();
    assert!(
        stderr(&loose).contains("error[E0093]: `ui` is the name of a std module"),
        "{}",
        stderr(&loose)
    );
    for target in [&["check"][..], &["check", "--target", "wasm"][..]] {
        let app = nova_in(&dir.join("app"), target).failure();
        let err = stderr(&app);
        assert!(
            err.contains("error[E0093]: `dom` is the name of a std module")
                && err.contains("dom.nova"),
            "{err}"
        );
    }
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

/// Build `source` for WASM and run it under Node from a page-like harness
/// that counts an interval's ticks and records the count at each write to
/// stdout, with `init`'s options for stress and debug. The harness prints
/// "responsive" when ticks fell between the program's first write and its
/// last, which a turn that holds the thread throughout can never do.
fn ticks_between(name: &str, source: &str) -> Assert {
    let node = node().expect("the caller checked for node");
    let dir = fresh(name);
    write(
        &dir,
        &[
            ("main.nova", source),
            (
                "run.mjs",
                "import init from \"./dist/app.js\";\n\
                 let ticks = 0;\n\
                 const at = [];\n\
                 const write = process.stdout.write.bind(process.stdout);\n\
                 process.stdout.write = (...args) => { at.push(ticks); return write(...args); };\n\
                 const timer = setInterval(() => ticks++, 5);\n\
                 await init({ gcStress: true, gcDebug: true });\n\
                 clearInterval(timer);\n\
                 process.stdout.write = write;\n\
                 console.log(at[at.length - 1] > at[0] ? \"responsive\" : `blocked ${at}`);\n",
            ),
        ],
    );
    build_wasm(&dir, Path::new("main.nova"));
    let mut node = node;
    node.arg(dir.join("run.mjs"))
        .timeout(std::time::Duration::from_secs(60))
        .assert()
        .success()
}

/// While an `async fn main` sleeps, it has returned to JS, which runs
/// other work, here an interval's ticks between the lines around the sleep;
/// `init` resolves when the program ends; and `init`'s options set the
/// collector's switches, as a page would (spec 4.1 §5, §6.6, §7.2).
#[test]
fn an_async_main_returns_to_js_while_it_sleeps() {
    if node().is_none() {
        return;
    }
    let a = ticks_between(
        "event-loop",
        "async fn main() {\n    println(\"sleeping\")\n    sleep(Duration::from_millis(100)).await\n    println(\"woke\")\n}\n",
    );
    assert_eq!(stdout(&a), "sleeping\nwoke\nresponsive\n");
    assert!(
        stderr(&a).starts_with("nova-gc: collection 1 freed "),
        "{}",
        stderr(&a)
    );
}

/// A task that waits for a channel yields in a loop, so while the sender
/// sleeps the queue never empties; a turn stops at its time budget and
/// returns to JS all the same (ADR 0040).
#[test]
fn a_task_waiting_in_a_loop_returns_to_js() {
    if node().is_none() {
        return;
    }
    let source = std::fs::read_to_string(repo_root().join("tests/wasm/turn_budget.nova"))
        .expect("the program");
    let a = ticks_between("turn-budget", &source);
    assert_eq!(stdout(&a), "waiting\ngot 7\nresponsive\n");
}

/// Only a module with an `async fn main` exports the turn: one without
/// keeps 4.0's shape (spec 4.1 §5).
#[test]
fn only_an_async_main_exports_the_turn() {
    let Some(node) = node() else { return };
    let dir = fresh("turn-export");
    write(
        &dir,
        &[
            ("sync/main.nova", "fn main() { println(\"a\") }\n"),
            ("async/main.nova", "async fn main() { println(\"a\") }\n"),
            (
                "exports.cjs",
                "const fs = require(\"fs\");\nconst m = new WebAssembly.Module(fs.readFileSync(process.argv[2]));\nconsole.log(WebAssembly.Module.exports(m).map((e) => e.name).join(\" \"));\n",
            ),
        ],
    );
    let mut seen = Vec::new();
    for kind in ["sync", "async"] {
        let dist = build_wasm(&dir.join(kind), Path::new("main.nova"));
        let mut node = Command::new("node");
        let a = node
            .arg(dir.join("exports.cjs"))
            .arg(dist.join("app.wasm"))
            .assert()
            .success();
        seen.push(stdout(&a));
    }
    drop(node);
    assert_eq!(seen[0], "main memory nova_alloc nova_alloc_leaf\n");
    assert_eq!(
        seen[1],
        "main memory nova_alloc nova_alloc_leaf nova_turn nova_gc_switches\n"
    );
}

/// A panic in a later turn, on a timer, ends the program as one in `main`
/// does (spec 4.1 §5).
#[test]
fn a_panic_in_a_later_turn_exits_134() {
    let Some(node) = node() else { return };
    let dir = fresh("late-panic");
    write(
        &dir,
        &[(
            "main.nova",
            "async fn main() {\n    println(\"before\")\n    sleep(Duration::from_millis(10)).await\n    panic(\"late\")\n}\n",
        )],
    );
    let dist = build_wasm(&dir, Path::new("main.nova"));
    let a = run_wasm(node, &dist).code(134);
    assert_eq!(stdout(&a), "before\n");
    assert_eq!(stderr(&a), "nova: panic: late\n");
}

/// Tasks of an `async fn main` that wait on each other are reported by a
/// turn, as natively, and abort (spec 4.1 §4.5, §5).
#[test]
fn a_deadlock_under_async_main_is_reported_as_natively() {
    let dir = fresh("async-deadlock");
    write(
        &dir,
        &[(
            "main.nova",
            "async fn wait_for(h: JoinHandle<()>) { h.join().await }\n\n\
             async fn main() {\n    let mut h = JoinHandle { fut: yield_now() }\n    \
             let t = spawn(wait_for(h))\n    h.fut = t.fut\n    t.join().await\n    \
             println(\"unreachable\")\n}\n",
        )],
    );
    let report = "nova: deadlock: 2 tasks are parked and none can wake\n  \
                  task 0 is waiting for task 1 to finish\n  \
                  task 1 is waiting for task 1 to finish\n";
    let native = nova_in(&dir, &["run", "main.nova"]).failure();
    assert_eq!(stderr(&native), report);
    let Some(node) = node() else { return };
    let dist = build_wasm(&dir, Path::new("main.nova"));
    let a = run_wasm(node, &dist).code(134);
    assert_eq!(stdout(&a), "");
    assert_eq!(stderr(&a), report);
}

/// An `async fn main`, which was E0091, checks and runs on WASM (spec 4.1
/// §4.4).
#[test]
fn an_async_main_runs_under_node() {
    let dir = fresh("async-main");
    write(
        &dir,
        &[("main.nova", "async fn main() { println(\"a\") }\n")],
    );
    nova_in(&dir, &["check", "--target", "wasm", "main.nova"]).success();
    let Some(node) = node() else { return };
    let dist = build_wasm(&dir, Path::new("main.nova"));
    let a = run_wasm(node, &dist).success();
    assert_eq!(stdout(&a), "a\n");
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

/// `node dist/app.js`, with no backtrace from `nova` involved, killed
/// after a minute: a channel program that regresses hangs rather than
/// fails (spec 4.1 §9.2).
fn run_wasm(node: Command, dist: &Path) -> Assert {
    run_wasm_with(node, dist, &[])
}

/// [`run_wasm`], with `env` set: the collector's switches.
fn run_wasm_with(mut node: Command, dist: &Path, env: &[(&str, &str)]) -> Assert {
    node.arg(dist.join("app.js"))
        .envs(env.iter().copied())
        .timeout(std::time::Duration::from_secs(60))
        .assert()
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
    // By its absolute path, so a `HistoryRouter` page reloads under any path
    // (the user's decision of 2026-10-18; ADR 0044).
    assert!(page.contains("import init from \"/app.js\";"), "{page}");
    let js = std::fs::read_to_string(dist.join("app.js")).expect("app.js");
    assert!(js.contains("export default async function init("), "{js}");
    let wasm = std::fs::read(dist.join("app.wasm")).expect("app.wasm");
    assert_eq!(&wasm[..4], b"\0asm");
    let package = std::fs::read_to_string(dist.join("package.json")).expect("package.json");
    assert_eq!(package, "{\"type\":\"module\"}\n");
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
    for name in [
        "index.html",
        "app.js",
        "app.wasm",
        "package.json",
        "keep.txt",
    ] {
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

/// A program built from a single file gets the generated page, which has
/// `#app`, no `assets/`, and a manifest named after the file (spec 4.4 §7).
#[test]
fn a_single_file_gets_the_generated_page_and_a_manifest() {
    let dir = fresh("bundle-file");
    write(&dir, &[("hello.nova", "fn main() { println(\"hi\") }\n")]);
    nova_in(&dir, &["build", "--target", "wasm", "hello.nova"]).success();
    let dist = dir.join("dist");
    let page = std::fs::read_to_string(dist.join("index.html")).expect("index.html");
    assert!(page.contains("<div id=\"app\"></div>"), "{page}");
    assert!(!dist.join("assets").exists());
    let manifest = std::fs::read_to_string(dist.join("manifest.json")).expect("manifest.json");
    assert!(
        manifest.starts_with(
            "{\n  \"name\": \"hello\",\n  \"version\": null,\n  \"release\": false,\n  \"wasm_opt\": false,\n"
        ),
        "{manifest}"
    );
    assert_eq!(
        manifest_files(&dist),
        ["app.js", "app.wasm", "index.html", "package.json"]
    );
}

/// The files `dist/manifest.json` lists, each checked against its size.
fn manifest_files(dist: &Path) -> Vec<String> {
    let text = std::fs::read_to_string(dist.join("manifest.json")).expect("manifest.json");
    let manifest: serde_json::Value = serde_json::from_str(&text).expect("JSON");
    let files = manifest["files"].as_object().expect("files");
    for (path, size) in files {
        let on_disk = std::fs::metadata(dist.join(path))
            .expect("a listed file")
            .len();
        assert_eq!(size.as_u64(), Some(on_disk), "{path}");
    }
    files.keys().cloned().collect()
}

/// A project's own `index.html` is copied, and its `public/` becomes
/// `assets/`, nested files included; on Unix, a link to a file inside
/// `public/` is copied as the file, and a link to a directory, or to a file
/// outside `public/`, is skipped. The manifest lists them all, and is the
/// same after a second build; a file removed from `public/` is gone from
/// `assets/` after the next build (spec 4.4 §7, §9.3).
#[test]
fn a_projects_own_page_and_public_go_into_dist() {
    let dir = fresh("bundle-project");
    let page = "<!doctype html>\n<title>mine</title>\n<div id=\"app\"></div>\n<script type=\"module\">import init from \"./app.js\"; init();</script>\n";
    write(
        &dir,
        &[
            (
                "nova.toml",
                "[package]\nname = \"site\"\nversion = \"1.2.3\"\nedition = \"2026\"\n",
            ),
            ("src/main.nova", "fn main() { println(\"hi\") }\n"),
            ("index.html", page),
            ("public/style.css", "body {}\n"),
            ("public/img/logo.txt", "logo"),
            ("secret.txt", "not public"),
        ],
    );
    #[cfg(unix)]
    for (target, link) in [
        ("public", "public/loop"),
        ("public/img/logo.txt", "public/logo-link.txt"),
        ("secret.txt", "public/secret-link.txt"),
    ] {
        std::os::unix::fs::symlink(dir.join(target), dir.join(link)).expect("a link");
    }
    // Links already at names the build writes, symbolic and hard: no write
    // goes through them.
    #[cfg(unix)]
    for (target, link) in [
        ("index.html", "dist/index.html"),
        ("secret.txt", "dist/manifest.json"),
        ("secret.txt", "dist/app.js"),
    ] {
        std::fs::create_dir_all(dir.join("dist")).expect("dist");
        std::os::unix::fs::symlink(dir.join(target), dir.join(link)).expect("a link");
    }
    #[cfg(unix)]
    std::fs::hard_link(dir.join("secret.txt"), dir.join("dist/package.json")).expect("a hard link");
    nova_in(&dir, &["build", "--target", "wasm"]).success();
    assert_eq!(
        std::fs::read_to_string(dir.join("index.html")).unwrap(),
        page
    );
    assert_eq!(
        std::fs::read_to_string(dir.join("secret.txt")).unwrap(),
        "not public"
    );
    let dist = dir.join("dist");
    let read = |p: &str| std::fs::read_to_string(dist.join(p)).expect(p);
    assert_eq!(read("index.html"), page);
    assert_eq!(read("assets/style.css"), "body {}\n");
    assert_eq!(read("assets/img/logo.txt"), "logo");
    assert!(!dist.join("assets/loop").exists());
    assert!(!dist.join("assets/secret-link.txt").exists());
    let linked: &[&str] = if cfg!(unix) {
        &["assets/logo-link.txt"]
    } else {
        &[]
    };
    for name in linked {
        assert_eq!(read(name), "logo");
    }
    let manifest = read("manifest.json");
    assert!(
        manifest.starts_with(
            "{\n  \"name\": \"site\",\n  \"version\": \"1.2.3\",\n  \"release\": false,\n  \"wasm_opt\": false,\n"
        ),
        "{manifest}"
    );
    let mut files = vec![
        "app.js",
        "app.wasm",
        "assets/img/logo.txt",
        "assets/style.css",
        "index.html",
        "package.json",
    ];
    files.extend(linked);
    files.sort_unstable();
    assert_eq!(manifest_files(&dist), files);
    nova_in(&dir, &["build", "--target", "wasm"]).success();
    assert_eq!(read("manifest.json"), manifest);
    std::fs::remove_file(dir.join("public/style.css")).expect("remove a file");
    nova_in(&dir, &["build", "--target", "wasm"]).success();
    assert!(!dist.join("assets/style.css").exists());
    assert!(!read("manifest.json").contains("style.css"));
}

/// A project with no `public/` leaves an `assets/` already in its output
/// directory as it is, and its manifest does not list it: a build replaces
/// `assets/` only when it copies a `public/` into it (spec 4.4 §7).
#[test]
fn an_assets_the_build_does_not_write_is_left_alone() {
    let dir = fresh("bundle-own-assets");
    write(
        &dir,
        &[
            (
                "nova.toml",
                "[package]\nname = \"site\"\nversion = \"0.1.0\"\nedition = \"2026\"\n",
            ),
            ("src/main.nova", "fn main() { println(\"hi\") }\n"),
            ("www/assets/photo.txt", "photo"),
            ("www/about.html", "about"),
        ],
    );
    nova_in(&dir, &["build", "--target", "wasm", "-o", "www"]).success();
    let www = dir.join("www");
    let read = |p: &str| std::fs::read_to_string(www.join(p)).expect(p);
    assert_eq!(read("assets/photo.txt"), "photo");
    assert_eq!(read("about.html"), "about");
    assert_eq!(
        manifest_files(&www),
        ["app.js", "app.wasm", "index.html", "package.json"]
    );
}

/// An output directory the copies would harm is refused before anything
/// is written: the project's root, however spelled, one holding it, and one
/// inside `public/`. The project's page and its own `assets/` stay.
#[test]
fn dist_may_not_hold_the_project_or_be_inside_public() {
    // The project sits one level down, so `..` is this test's own
    // directory: a build the guard let through writes there, and nowhere
    // else.
    let outer = fresh("bundle-refused");
    let dir = outer.join("site");
    write(
        &dir,
        &[
            (
                "nova.toml",
                "[package]\nname = \"site\"\nversion = \"0.1.0\"\nedition = \"2026\"\n",
            ),
            ("src/main.nova", "fn main() { println(\"hi\") }\n"),
            ("index.html", "mine"),
            ("public/a.txt", "a"),
            ("assets/own.txt", "own"),
        ],
    );
    #[cfg(unix)]
    std::os::unix::fs::symlink("..", dir.join("uplink")).expect("a link");
    let spellings: &[&str] = if cfg!(unix) {
        &[".", "..", "nope/..", "src/..", "uplink"]
    } else {
        &[".", "..", "nope/..", "src/.."]
    };
    for out in spellings {
        let a = nova_in(&dir, &["build", "--target", "wasm", "-o", out]).failure();
        assert!(
            stderr(&a).contains("the output directory holds the project"),
            "{out}: {}",
            stderr(&a)
        );
    }
    for built in [
        outer.join("app.wasm"),
        outer.join("assets"),
        dir.join("app.wasm"),
    ] {
        assert!(!built.exists(), "{}", built.display());
    }
    let a = nova_in(&dir, &["build", "--target", "wasm", "-o", "public/out"]).failure();
    assert!(stderr(&a).contains("inside `public/`"), "{}", stderr(&a));
    assert_eq!(
        std::fs::read_to_string(dir.join("index.html")).unwrap(),
        "mine"
    );
    assert_eq!(
        std::fs::read_to_string(dir.join("assets/own.txt")).unwrap(),
        "own"
    );
    assert!(!dir.join("public/out").exists());
}

/// A `public/` that is a link leading outside the project is not copied,
/// and the build says so (spec 4.4 §7: a copy cannot leave the project).
#[cfg(unix)]
#[test]
fn a_public_that_leads_outside_is_not_copied() {
    let outer = fresh("bundle-public-outside");
    let dir = outer.join("site");
    write(
        &outer,
        &[
            (
                "site/nova.toml",
                "[package]\nname = \"site\"\nversion = \"0.1.0\"\nedition = \"2026\"\n",
            ),
            ("site/src/main.nova", "fn main() { println(\"hi\") }\n"),
            ("elsewhere/x.txt", "not the project's"),
        ],
    );
    std::os::unix::fs::symlink(outer.join("elsewhere"), dir.join("public")).expect("a link");
    let a = nova_in(&dir, &["build", "--target", "wasm"]).success();
    assert!(
        stderr(&a).contains("public/ leads outside the project; it is not copied"),
        "{}",
        stderr(&a)
    );
    assert!(!dir.join("dist/assets").exists());
    assert!(!std::fs::read_to_string(dir.join("dist/manifest.json"))
        .expect("manifest.json")
        .contains("x.txt"));
}

/// A stand-in for `wasm-opt`: it copies its input, the argument after
/// `-Oz`, to its output, the one after `-o`, and prints its arguments.
const WASM_OPT: &str = "fn main() { block_on(go()) }

async fn go() {
    let argv = args()
    let mut line = \"wasm-opt:\"
    let mut i = 1
    while i < argv.len() {
        match argv.get(i) {
            Some(a) => line = \"${line} ${a}\"
            None => {}
        }
        i = i + 1
    }
    println(line)
    match argv.get(2) {
        Some(input) => match argv.get(4) {
            Some(output) => match read(input).await {
                Ok(bytes) => {
                    let _ = write(output, bytes).await
                }
                Err(e) => exit(2)
            }
            None => exit(2)
        }
        None => exit(2)
    }
}
";

/// `--release` runs `wasm-opt -Oz` when it is on `PATH`, says so in one
/// line when it is not, and fails when it fails (spec 4.4 §7, §9.3).
#[test]
fn release_runs_wasm_opt_when_it_is_on_path() {
    let dir = fresh("release");
    write(
        &dir,
        &[
            ("main.nova", "fn main() { println(\"hi\") }\n"),
            ("standin/ok.nova", WASM_OPT),
            ("standin/bad.nova", "fn main() { exit(3) }\n"),
        ],
    );
    let exe = format!("wasm-opt{}", std::env::consts::EXE_SUFFIX);
    for (name, bin) in [("ok", "bin-ok"), ("bad", "bin-bad")] {
        let out = format!("{bin}/{exe}");
        std::fs::create_dir_all(dir.join(bin)).expect("a bin directory");
        nova_in(
            &dir,
            &["build", &format!("standin/{name}.nova"), "-o", &out],
        )
        .success();
    }
    let path_with = |bin: &str| {
        let mut dirs = vec![dir.join(bin)];
        dirs.extend(std::env::split_paths(
            &std::env::var_os("PATH").unwrap_or_default(),
        ));
        std::env::join_paths(dirs).expect("a PATH")
    };
    let home = dir.with_file_name("nova-wasm-test-release-home");
    let build = ["build", "--target", "wasm", "--release", "main.nova"];

    // None on PATH: the build says so, and the manifest that none ran.
    std::fs::create_dir_all(dir.join("bin-none")).expect("an empty directory");
    let a = nova_at(&dir, &home)
        .env("PATH", dir.join("bin-none"))
        .args(build)
        .assert()
        .success();
    assert!(
        stderr(&a).contains("wasm-opt not found on PATH; app.wasm is not optimized"),
        "{}",
        stderr(&a)
    );
    let manifest = std::fs::read_to_string(dir.join("dist/manifest.json")).expect("manifest");
    assert!(
        manifest.contains("\"release\": true,\n  \"wasm_opt\": false,"),
        "{manifest}"
    );

    // The stand-in: run with `-Oz` on `app.wasm`, in `dist/`.
    let a = nova_at(&dir, &home)
        .env("PATH", path_with("bin-ok"))
        .args(build)
        .assert()
        .success();
    assert!(
        stdout(&a).contains("wasm-opt: -Oz app.wasm -o app.wasm\n"),
        "{}",
        stdout(&a)
    );
    let manifest = std::fs::read_to_string(dir.join("dist/manifest.json")).expect("manifest");
    assert!(
        manifest.contains("\"release\": true,\n  \"wasm_opt\": true,"),
        "{manifest}"
    );
    let wasm = std::fs::read(dir.join("dist/app.wasm")).expect("app.wasm");
    assert_eq!(&wasm[..4], b"\0asm");

    // One that fails fails the build; without `--release` none runs.
    let a = nova_at(&dir, &home)
        .env("PATH", path_with("bin-bad"))
        .args(build)
        .assert()
        .failure();
    assert!(stderr(&a).contains("wasm-opt failed"), "{}", stderr(&a));
    let a = nova_at(&dir, &home)
        .env("PATH", path_with("bin-bad"))
        .args(["build", "--target", "wasm", "main.nova"])
        .assert()
        .success();
    assert!(!stderr(&a).contains("wasm-opt"), "{}", stderr(&a));
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

/// Recursion deeper than V8's stack ends as a native stack overflow does:
/// the message, and an abort's status (spec §6, plan decision 17).
#[test]
fn a_stack_overflow_exits_134_with_its_message() {
    let Some(node) = node() else { return };
    let dir = fresh("overflow");
    write(
        &dir,
        &[(
            "main.nova",
            "fn down(n: Int) -> Int { if n == 0 { 0 } else { 1 + down(n - 1) } }\nfn main() { println(\"${down(100000000)}\") }\n",
        )],
    );
    let dist = build_wasm(&dir, Path::new("main.nova"));
    let a = run_wasm(node, &dist).code(134);
    assert_eq!(stderr(&a), "nova: stack overflow\n");
}

/// `node <dir>/dist/app.js` runs the program when `<dir>` is reached
/// through a symbolic link, as a temporary directory is on macOS: Node
/// resolves the link in `import.meta.url`, so `app.js` must too.
#[cfg(unix)]
#[test]
fn app_js_runs_through_a_symbolic_link() {
    let Some(node) = node() else { return };
    let dir = fresh("symlink");
    write(
        &dir,
        &[("real/main.nova", "fn main() { println(\"hi\") }\n")],
    );
    build_wasm(&dir.join("real"), Path::new("main.nova"));
    std::os::unix::fs::symlink(dir.join("real"), dir.join("link")).expect("a link");
    let a = run_wasm(node, &dir.join("link").join("dist")).success();
    assert_eq!(stdout(&a), "hi\n");
}

/// A project's own `package.json` does not change how Node reads
/// `dist/app.js`: `dist/package.json` says it is a module.
#[test]
fn app_js_runs_beside_a_commonjs_package_json() {
    let Some(node) = node() else { return };
    let dir = fresh("commonjs");
    write(
        &dir,
        &[
            ("package.json", "{\"type\":\"commonjs\"}\n"),
            ("main.nova", "fn main() { println(\"hi\") }\n"),
        ],
    );
    let dist = build_wasm(&dir, Path::new("main.nova"));
    let a = run_wasm(node, &dist).success();
    assert_eq!(stdout(&a), "hi\n");
    assert_eq!(stderr(&a), "");
}

/// A function needing more locals than WebAssembly allows is refused when
/// it is built, naming the limit, and not reported as a compiler bug.
#[test]
fn a_function_over_the_local_limit_is_refused() {
    let dir = fresh("locals");
    let mut source = String::from("fn main() {\n");
    for i in 0..12000 {
        source.push_str(&format!("    println(\"line {i}: ${{{i} * 2}}\")\n"));
    }
    source.push_str("}\n");
    write(&dir, &[("main.nova", &source)]);
    let a = nova_in(&dir, &["build", "--target", "wasm", "main.nova"]).failure();
    let err = stderr(&a);
    assert!(err.contains("WebAssembly allows 50000"), "{err}");
    assert!(!err.contains("compiler bug"), "{err}");
}

/// A program in `tests/wasm/`, held to its `.stdout` file natively and,
/// built for WASM, under Node: the two must print the same (spec §9.3).
fn differential(name: &str) {
    let source = repo_root().join("tests/wasm").join(format!("{name}.nova"));
    let expected = std::fs::read_to_string(source.with_extension("stdout"))
        .expect("the expected output")
        .replace("\r\n", "\n");
    let dir = fresh(&format!("diff-{name}"));
    let native = nova_in(&dir, &["run", &source.to_string_lossy()]).success();
    assert_eq!(stdout(&native), expected, "natively");
    let Some(node) = node() else { return };
    let dist = build_wasm(&dir, &source);
    let wasm = run_wasm(node, &dist).success();
    assert_eq!(stdout(&wasm), expected, "on WASM");
}

/// `std/rt`'s string and bytes functions are `nova-runtime`'s (spec §5.3).
#[test]
fn std_rt_strings_and_bytes_match_nova_runtime() {
    differential("strings_and_bytes");
}

/// `check_bounds` panics with `nova-runtime`'s message (spec §5.3).
#[test]
fn an_index_out_of_bounds_panics_as_natively() {
    let Some(node) = node() else { return };
    let dir = fresh("bounds");
    write(
        &dir,
        &[(
            "main.nova",
            "fn main() {\n    let xs = [1, 2, 3]\n    let i = xs.len() + 2\n    println(\"${xs[i]}\")\n}\n",
        )],
    );
    let dist = build_wasm(&dir, Path::new("main.nova"));
    let a = run_wasm(node, &dist).code(134);
    assert_eq!(
        stderr(&a),
        "nova: panic: array index 5 out of bounds for length 3\n"
    );
}

/// The shim's `Float` text is `nova-runtime`'s, exact ties included (spec
/// §6, §9.3).
#[test]
fn float_text_matches_nova_runtime() {
    differential("float_text");
}

/// The shim's `Float::fixed` rounds as `nova-runtime`'s does, half to even
/// on the exact value (spec §6, §9.3).
#[test]
fn float_fixed_matches_nova_runtime() {
    differential("float_fixed");
}

/// Case mapping and hashing on WASM agree with `nova-runtime` (spec §6,
/// §9.3).
#[test]
fn case_and_hash_match_nova_runtime() {
    differential("case_and_hash");
}

/// The allocator's size classes and large objects keep every object
/// apart (spec 4.1 §6.2).
#[test]
fn the_allocator_keeps_objects_apart() {
    differential("alloc_classes");
}

/// `block_on` wakes due deadlines between polls while a task waits for a
/// channel in a loop, so the sender's sleep ends: it runs the queue until
/// it is empty, with no time budget (spec 4.1 §4.4).
#[test]
fn block_on_wakes_a_sleeper_while_a_task_waits_in_a_loop() {
    differential("block_on_waits_in_a_loop");
}

/// An allocation over 1 GB, the memory's limit, is refused with "out of
/// memory": 1.6 GB, more pages than memory can have, and a size just under
/// 2^63, which `gc_alloc` refuses before rounding it up to a multiple of 8
/// would wrap it negative (spec 4.1 §6.2).
#[test]
fn an_allocation_over_1_gb_is_out_of_memory() {
    if node().is_none() {
        return;
    }
    for (name, n) in [("oom", "200000000"), ("oom-wrap", "1152921504606846974")] {
        let dir = fresh(name);
        write(
            &dir,
            &[(
                "main.nova",
                &format!(
                    "fn main() {{\n    println(\"before\")\n    let n = {n}\n    let xs = [0; n]\n    println(\"${{xs.len()}}\")\n}}\n"
                ),
            )],
        );
        let dist = build_wasm(&dir, Path::new("main.nova"));
        let node = node().expect("node is on PATH");
        let a = run_wasm(node, &dist).code(134);
        assert_eq!(stdout(&a), "before\n", "{n}");
        assert_eq!(stderr(&a), "nova: panic: out of memory\n", "{n}");
    }
}

/// A program in `tests/wasm/` that needs the collector: natively, then
/// built for WASM and run with `env`, it prints the same. Returns what the
/// WASM build wrote on stderr, the collector's lines among it.
fn collected(name: &str, env: &[(&str, &str)]) -> String {
    let source = repo_root().join("tests/wasm").join(format!("{name}.nova"));
    let expected = std::fs::read_to_string(source.with_extension("stdout"))
        .expect("the expected output")
        .replace("\r\n", "\n");
    let dir = fresh(&format!("gc-{name}"));
    let native = nova_in(&dir, &["run", &source.to_string_lossy()]).success();
    assert_eq!(stdout(&native), expected, "natively");
    let Some(node) = node() else {
        return String::new();
    };
    let dist = build_wasm(&dir, &source);
    let wasm = run_wasm_with(node, &dist, env).success();
    assert_eq!(stdout(&wasm), expected, "on WASM");
    stderr(&wasm)
}

/// The objects live after each collection, from `nova-runtime`'s
/// `NOVA_GC_DEBUG` line, which the WASM collector writes too.
fn live_after_each(stderr: &str) -> Vec<u64> {
    stderr
        .lines()
        .filter(|l| l.starts_with("nova-gc: collection "))
        .map(|l| {
            let (_, rest) = l.split_once(" bytes, ").expect("the freed bytes");
            let (live, _) = rest.split_once(" objects live").expect("the live count");
            live.parse().expect("a count")
        })
        .collect()
}

/// The collector reclaims what no task reaches: 300 turns of 1,000 records
/// each, and no collection finds 1,000 objects live, as
/// `gc_reclaims_garbage` checks natively (spec 4.1 §9.3).
#[test]
fn the_collector_reclaims_what_no_task_reaches() {
    let err = collected("gc_reclaims", &[("NOVA_GC_DEBUG", "1")]);
    if node().is_none() {
        return;
    }
    let live = live_after_each(&err);
    assert!(live.len() >= 3, "{err}");
    assert!(live.iter().all(|&n| n < 1000), "{err}");
}

/// A record only a task reaches survives a collection at the end of every
/// turn (spec 4.1 §9.3).
#[test]
fn a_record_only_a_task_reaches_survives() {
    let err = collected(
        "gc_keeps_task_records",
        &[("NOVA_GC_STRESS", "1"), ("NOVA_GC_DEBUG", "1")],
    );
    if node().is_none() {
        return;
    }
    assert!(live_after_each(&err).len() >= 50, "{err}");
}

/// A record only a detached task reaches survives: nothing holds the
/// task's handle, so only the executor's root for the task keeps its state
/// until it completes (spec 4.1 §6.4).
#[test]
fn a_record_only_a_detached_task_reaches_survives() {
    let err = collected(
        "gc_keeps_detached_tasks",
        &[("NOVA_GC_STRESS", "1"), ("NOVA_GC_DEBUG", "1")],
    );
    if node().is_none() {
        return;
    }
    assert!(live_after_each(&err).len() >= 100, "{err}");
}

/// A yield still yields once collections have freed slots for reuse: the
/// allocator zeroes every slot it hands out, and the yield future's state
/// starts at tag 0 by that (spec 4.1 §6.2).
#[test]
fn a_yield_yields_after_collections() {
    let err = collected(
        "gc_yields_after_collections",
        &[("NOVA_GC_STRESS", "1"), ("NOVA_GC_DEBUG", "1")],
    );
    if node().is_none() {
        return;
    }
    assert!(live_after_each(&err).len() >= 4, "{err}");
}

/// A turn stops at its time budget while a task waits for a channel in a
/// loop and the sender sleeps for 300 ms, so the collector runs during the
/// wait, at the end of each turn; one turn that held the thread until the
/// send would end with two collections in all (ADR 0040).
#[test]
fn a_turn_stops_at_its_budget_and_collects() {
    let err = collected(
        "turn_budget",
        &[("NOVA_GC_STRESS", "1"), ("NOVA_GC_DEBUG", "1")],
    );
    if node().is_none() {
        return;
    }
    assert!(live_after_each(&err).len() >= 5, "{err}");
}

/// 20,000 records a task keeps survive collections: their array is a
/// large object, and tracing it queues them all, past the worklist's first
/// page (spec 4.1 §6.4).
#[test]
fn many_records_a_task_keeps_survive() {
    let err = collected(
        "gc_keeps_many",
        &[("NOVA_GC_STRESS", "1"), ("NOVA_GC_DEBUG", "1")],
    );
    if node().is_none() {
        return;
    }
    let live = live_after_each(&err);
    assert!(live.len() >= 5, "{err}");
    assert!(live.iter().all(|&n| n > 20000), "{err}");
}

/// A future built where a freed state was names no old task: joining it,
/// never spawned, aborts as natively, where the executor's index still held
/// the old address it would find a finished task (spec 4.1 §4.2, §6.4).
#[test]
fn a_freed_state_names_no_old_task() {
    let source = repo_root().join("tests/wasm/gc_forgets_freed_states.nova");
    let message = "nova: panic: nova_rt_task_join_future: this future was never spawned, \
                   so there is no task to ask about";
    let dir = fresh("gc-freed-states");
    let native = nova_in(&dir, &["run", &source.to_string_lossy()]).failure();
    assert!(stderr(&native).contains(message), "{}", stderr(&native));
    let Some(node) = node() else { return };
    let dist = build_wasm(&dir, &source);
    let env = [("NOVA_GC_STRESS", "1"), ("NOVA_GC_DEBUG", "1")];
    let a = run_wasm_with(node, &dist, &env).code(134);
    assert_eq!(stdout(&a), "");
    let err = stderr(&a);
    assert!(err.contains(message), "{err}");
    assert!(!live_after_each(&err).is_empty(), "{err}");
}

/// A program of `tests/dom/` (spec 4.2 §10.2): built for WASM, run under
/// Node by its harness, `<name>.mjs`, which installs the fake DOM, drives
/// the page and prints it, with `env` set. Its stdout is `<name>.stdout`.
/// Returns its stderr.
fn dom_program(name: &str, env: &[(&str, &str)]) -> String {
    match dom_run(name, env) {
        Some(a) => stderr(&a.success()),
        None => String::new(),
    }
}

/// [`dom_program`]'s run, whatever its status, or `None` without `node`.
fn dom_run(name: &str, env: &[(&str, &str)]) -> Option<Assert> {
    let node = node()?;
    let tests = repo_root().join("tests/dom");
    let expected = std::fs::read_to_string(tests.join(format!("{name}.stdout")))
        .expect("the expected output")
        .replace("\r\n", "\n");
    let kind = if env.is_empty() { "dom" } else { "dom-stress" };
    let dir = fresh(&format!("{kind}-{name}"));
    for file in ["fake_dom.mjs".to_string(), format!("{name}.mjs")] {
        std::fs::copy(tests.join(&file), dir.join(&file)).expect("copy the harness");
    }
    build_wasm(&dir, &tests.join(format!("{name}.nova")));
    let mut node = node;
    let a = node
        .arg(dir.join(format!("{name}.mjs")))
        .envs(env.iter().copied())
        .timeout(std::time::Duration::from_secs(60))
        .assert();
    assert_eq!(stdout(&a), expected, "{}", stderr(&a));
    Some(a)
}

/// Under `NOVA_GC_STRESS` and `NOVA_GC_DEBUG`.
const STRESS: [(&str, &str); 2] = [("NOVA_GC_STRESS", "1"), ("NOVA_GC_DEBUG", "1")];

/// std/dom builds a page: elements and text, attributes and properties,
/// inserting, removing and walking, and selectors, matching and not (spec
/// 4.2 §6.2). Its `main` registers no closure JS may call, so even under
/// stress nothing collects: `main` is not a call from JS (§5.2).
#[test]
fn std_dom_builds_a_page() {
    dom_program("build", &[]);
    let err = dom_program("build", &STRESS);
    assert!(live_after_each(&err).is_empty(), "{err}");
}

/// A module that registers a closure JS may call exports `nova_call`, and
/// the turn and the switches with it, though its `main` is synchronous; one
/// that only builds a page exports neither (spec 4.2 §5.2).
#[test]
fn a_listener_exports_the_call_and_the_turn() {
    let Some(node) = node() else { return };
    let dir = fresh("call-export");
    write(
        &dir,
        &[
            ("build/main.nova", "import dom\nfn main() { body().set_text(\"x\") }\n"),
            (
                "listen/main.nova",
                "import dom\nfn main() {\n    let _ = body().add_event_listener(\"click\", |e: Event| println(\"x\"))\n}\n",
            ),
            (
                "exports.cjs",
                "const fs = require(\"fs\");\nconst m = new WebAssembly.Module(fs.readFileSync(process.argv[2]));\nconsole.log(WebAssembly.Module.exports(m).map((e) => e.name).join(\" \"));\n",
            ),
        ],
    );
    let mut seen = Vec::new();
    for kind in ["build", "listen"] {
        let dist = build_wasm(&dir.join(kind), Path::new("main.nova"));
        let mut node = Command::new("node");
        let a = node
            .arg(dir.join("exports.cjs"))
            .arg(dist.join("app.wasm"))
            .assert()
            .success();
        seen.push(stdout(&a));
    }
    drop(node);
    assert_eq!(seen[0], "main memory nova_alloc nova_alloc_leaf\n");
    assert_eq!(
        seen[1],
        "main memory nova_alloc nova_alloc_leaf nova_call nova_turn nova_gc_switches\n"
    );
}

/// The gate (spec 4.2 §1): a button and a count, and three clicks make the
/// count 3. Under stress the collector runs after `main` and after each
/// click, and the listener's closure, a root of std/rt's table, survives.
/// Each click's event handle is released at the end of its call, and only
/// the label's, which the closure holds, stays live.
#[test]
fn the_counter_counts_clicks() {
    dom_program("counter", &[]);
    let err = dom_program("counter", &STRESS);
    if node().is_none() {
        return;
    }
    assert_eq!(live_after_each(&err).len(), 4, "{err}");
    assert_eq!(calls_after_each(&err), [1, 1, 1, 1], "{err}");
    assert_eq!(
        handles_released(&err),
        [(3, 1), (1, 1), (1, 1), (1, 1)],
        "{err}"
    );
}

/// Listeners: a removed one runs no more; `stop_propagation`,
/// `prevent_default`, an event's kind, target and key; and a window
/// listener (spec 4.2 §5, §6.2).
/// The table holds seven entries after `main`, one of them the stopper
/// added again in the entry its removal freed, and six once the first
/// click has removed `once`.
#[test]
fn listeners_run_until_removed() {
    dom_program("listeners", &[]);
    let err = dom_program("listeners", &STRESS);
    if node().is_none() {
        return;
    }
    let calls = calls_after_each(&err);
    assert_eq!(calls.first(), Some(&7), "{err}");
    assert_eq!(calls.last(), Some(&6), "{err}");
}

/// A call from JS inside one runs its handler only: under stress the
/// collector runs after `main` and after each outer click, three times,
/// and never frees the record the outer handler holds (spec 4.2 §5.2).
#[test]
fn a_nested_call_from_js_waits_for_the_outer_one() {
    dom_program("nested", &[]);
    let err = dom_program("nested", &STRESS);
    if node().is_none() {
        return;
    }
    assert_eq!(live_after_each(&err).len(), 3, "{err}");
}

/// A panic in a listener ends the program: its message, an abort's status,
/// and the next click calls nothing. A panic in a listener called from
/// inside another, which the page keeps as a browser does, stops the outer
/// one too (spec 4.2 §5.3). Under stress the collector ran first, after
/// `main` and after the first click.
#[test]
fn a_panic_in_a_listener_ends_the_program() {
    for env in [&[][..], &STRESS[..]] {
        let Some(a) = dom_run("panic", env) else {
            return;
        };
        let a = a.code(134);
        let err = stderr(&a);
        assert!(err.contains("nova: panic: boom\n"), "{err}");
        let collections = if env.is_empty() { 0 } else { 2 };
        assert_eq!(live_after_each(&err).len(), collections, "{err}");
    }
}

/// The shim keeps one timer: a turn a click asks for sooner is scheduled
/// while the one `main`'s long sleep asked for is pending, and comes first
/// (spec 4.2 §7.3).
#[test]
fn a_sooner_turn_replaces_a_later_timer() {
    dom_program("scheduler", &[]);
    let err = dom_program("scheduler", &STRESS);
    if node().is_none() {
        return;
    }
    assert!(!live_after_each(&err).is_empty(), "{err}");
}

/// `block_on` in a task is re-entrant wherever the task runs: one a click
/// spawned, which the click's turn polls, panics as one under an `async fn
/// main` does (spec 4.2 §5.2).
#[test]
fn block_on_in_a_task_a_click_spawned_panics() {
    for env in [&[][..], &STRESS[..]] {
        let Some(a) = dom_run("reentrant", env) else {
            return;
        };
        let a = a.code(134);
        let err = stderr(&a);
        assert!(
            err.contains("nova: panic: nova_rt_task_block_on called re-entrantly"),
            "{err}"
        );
    }
}

/// Timers: a timeout, an interval cleared after three ticks, and a cleared
/// timeout that never runs, in order; clearing a timer again, or one that
/// has run, releases no entry another timer took; a timeout too long for
/// JS does not fire at once; `init` resolves while they are still pending.
/// Each entry of std/rt's table leaves it when its timeout fires or its
/// timer is cleared, so none is left at the end (spec 4.2 §6.2, §7.3).
#[test]
fn timers_run_in_order_until_cleared() {
    dom_program("timers", &[]);
    let err = dom_program("timers", &STRESS);
    if node().is_none() {
        return;
    }
    assert_eq!(calls_after_each(&err).last(), Some(&0), "{err}");
}

/// The location and history change without a load, and the window hears
/// `hashchange` and `popstate` after the calls that cause them (spec 4.2
/// §6.2).
#[test]
fn the_location_and_history_change_in_place() {
    dom_program("history", &[]);
    let err = dom_program("history", &STRESS);
    if node().is_none() {
        return;
    }
    assert!(!live_after_each(&err).is_empty(), "{err}");
}

/// `fetch` from an `async fn main`: replies of 200 and 404, a failure, a
/// POST with a header and a body, and a header value that would make a
/// second header, refused before sending; and from a click handler, through
/// a task it spawns, which waits on its token after the call returns (spec
/// 4.2 §6.2, §7).
#[test]
fn fetch_waits_for_the_reply() {
    dom_program("fetch", &[]);
    let err = dom_program("fetch", &STRESS);
    if node().is_none() {
        return;
    }
    assert!(!live_after_each(&err).is_empty(), "{err}");
}

/// A `fetch` awaited inside `block_on` cannot be answered: a panic that
/// names `async fn main` (spec 4.2 §7.2).
#[test]
fn a_fetch_in_block_on_panics() {
    for env in [&[][..], &STRESS[..]] {
        let Some(a) = dom_run("fetch_block_on", env) else {
            return;
        };
        let a = a.code(134);
        assert!(
            stderr(&a).contains(
                "nova: panic: nova_rt_task_block_on: a task waits for JS, as `fetch` does, and JS cannot answer while block_on holds the thread; await it from an `async fn main` instead"
            ),
            "{}",
            stderr(&a)
        );
    }
}

/// A panic in an interval's call ends the program under Node: the interval
/// stops calling in and clears itself, so Node exits with an abort's
/// status, and does not wait for the interval for ever (spec 4.2 §5.3).
#[test]
fn a_panic_in_an_interval_ends_the_process() {
    let Some(node) = node() else { return };
    let dir = fresh("dom-interval-panic");
    write(
        &dir,
        &[(
            "main.nova",
            "import dom\nfn main() {\n    let _ = set_interval(1, | | panic(\"tick\"))\n    println(\"main done\")\n}\n",
        )],
    );
    let dist = build_wasm(&dir, Path::new("main.nova"));
    let a = run_wasm(node, &dist).code(134);
    assert_eq!(stdout(&a), "main done\n");
    assert_eq!(stderr(&a), "nova: panic: tick\n");
}

/// The entries of std/rt's table of closures JS may call in use after each
/// collection, from `NOVA_GC_DEBUG`'s `nova-dom:` lines.
fn calls_after_each(stderr: &str) -> Vec<u64> {
    stderr
        .lines()
        .filter(|l| l.starts_with("nova-dom: collection "))
        .map(|l| {
            let (_, rest) = l.split_once(" live, ").expect("the live count");
            let (calls, _) = rest.split_once(" calls").expect("the calls");
            calls.parse().expect("a count")
        })
        .collect()
}

/// The `nova-dom:` lines of `NOVA_GC_DEBUG`: each collection's handles
/// released, and those the shim's table still holds.
fn handles_released(stderr: &str) -> Vec<(u64, u64)> {
    stderr
        .lines()
        .filter_map(|l| l.strip_prefix("nova-dom: collection "))
        .map(|rest| {
            let (_, rest) = rest.split_once(" released ").expect("released");
            let (released, rest) = rest.split_once(" handles, ").expect("handles");
            let (live, _) = rest.split_once(" live").expect("live");
            (
                released.parse().expect("a count"),
                live.parse().expect("a count"),
            )
        })
        .collect()
}

/// A handle is released once no value holds it: over 50 turns of 200
/// nodes each, the collector releases nearly all of them, and the shim's
/// table, which counts its own slots, never holds 400 at once (spec 4.2
/// §4.2).
#[test]
fn handles_are_released_when_no_value_holds_them() {
    dom_program("handles", &[]);
    let err = dom_program("handles", &STRESS);
    if node().is_none() {
        return;
    }
    let lines = handles_released(&err);
    let released: u64 = lines.iter().map(|(r, _)| r).sum();
    assert!(released >= 9_900, "{err}");
    assert!(lines.iter().all(|&(_, live)| live < 400), "{err}");
}

/// An `Element` built by hand whose box names no live handle is a panic
/// when used (spec 4.2 §4.1; decision 5 of the plan says what a box that
/// happens to name one reaches).
#[test]
fn a_forged_element_panics() {
    let Some(node) = node() else { return };
    let dir = fresh("dom-forged");
    write(
        &dir,
        &[(
            "main.nova",
            "import dom\nfn main() {\n    let e = Element { box: 0 }\n    println(\"before\")\n    e.set_text(\"x\")\n}\n",
        )],
    );
    let dist = build_wasm(&dir, Path::new("main.nova"));
    let a = run_wasm(node, &dist).code(134);
    assert_eq!(stdout(&a), "before\n");
    assert_eq!(stderr(&a), "nova: panic: std/dom: handle 0 is not live\n");
}

/// `std/rt`'s public functions are std's alone: a user's module cannot
/// call one (spec 4.2 §5).
#[test]
fn std_rts_functions_are_std_internal() {
    let dir = fresh("rt-internal");
    write(
        &dir,
        &[(
            "main.nova",
            "fn main() { println(\"${rt_handle_box(1)}\") }\n",
        )],
    );
    let a = nova_in(&dir, &["check", "--target", "wasm", "main.nova"]).failure();
    assert!(
        stderr(&a).contains("error[E0001]: cannot find function `rt_handle_box`"),
        "{}",
        stderr(&a)
    );
}

/// One of the gate's programs (spec §9.2): built for WASM, it prints under
/// Node what its `.stdout` file says the native build prints.
fn gate_program(name: &str) {
    gate_program_with(name, "gate", &[]);
}

/// [`gate_program`] in a directory named for `kind`, with `env` set.
fn gate_program_with(name: &str, kind: &str, env: &[(&str, &str)]) {
    let Some(node) = node() else { return };
    let source = repo_root()
        .join("tests/runtime")
        .join(format!("{name}.nova"));
    let expected = std::fs::read_to_string(source.with_extension("stdout"))
        .expect("the expected output")
        .replace("\r\n", "\n");
    let dir = fresh(&format!("{kind}-{name}"));
    let dist = build_wasm(&dir, &source);
    let a = run_wasm_with(node, &dist, env).success();
    assert_eq!(stdout(&a), expected);
}

/// One of the gate's panicking programs: it fails with the native build's
/// message on stderr and an abort's status, as `run_tests.rs` checks it
/// natively, and what it printed first passes `printed`. It ends by its
/// own panic, not by a stack overflow or a trap.
fn gate_panics(name: &str, message: &str, printed: fn(&str) -> bool) {
    gate_panics_with(name, "gate", &[], message, printed);
}

/// [`gate_panics`] in a directory named for `kind`, with `env` set.
fn gate_panics_with(
    name: &str,
    kind: &str,
    env: &[(&str, &str)],
    message: &str,
    printed: fn(&str) -> bool,
) {
    let Some(node) = node() else { return };
    let source = repo_root()
        .join("tests/runtime")
        .join(format!("{name}.nova"));
    let dir = fresh(&format!("{kind}-{name}"));
    let dist = build_wasm(&dir, &source);
    let a = run_wasm_with(node, &dist, env).code(134);
    let err = stderr(&a);
    assert!(err.contains(message), "stderr: {err}");
    assert!(
        !err.contains("stack overflow") && !err.contains("nova: trap"),
        "stderr: {err}"
    );
    assert!(printed(&stdout(&a)), "stdout: {}", stdout(&a));
}

/// Each line of `text`, with a leading `to_iso8601` timestamp, where there
/// is a well-formed one, written `<time>`.
fn untimed(text: &str) -> Vec<String> {
    let stamp = |s: &str| {
        s.len() == 24
            && s.bytes().enumerate().all(|(i, c)| match i {
                4 | 7 => c == b'-',
                10 => c == b'T',
                13 | 16 => c == b':',
                19 => c == b'.',
                23 => c == b'Z',
                _ => c.is_ascii_digit(),
            })
    };
    text.lines()
        .map(|l| match l.split_once(' ') {
            Some((s, rest)) if stamp(s) => format!("<time> {rest}"),
            _ => l.to_string(),
        })
        .collect()
}

/// One of the gate's log programs (spec 4.1 §9.2): built for WASM, it
/// writes these lines under Node, as `run_tests.rs` checks them natively,
/// and its stdout is its `.stdout` file where it has one.
fn gate_log(name: &str, out: &[&str], err: &[&str]) {
    let Some(node) = node() else { return };
    let source = repo_root()
        .join("tests/runtime")
        .join(format!("{name}.nova"));
    let dir = fresh(&format!("gate-{name}"));
    let dist = build_wasm(&dir, &source);
    let a = run_wasm(node, &dist).success();
    if let Ok(expected) = std::fs::read_to_string(source.with_extension("stdout")) {
        assert_eq!(
            stdout(&a),
            expected.replace("\r\n", "\n"),
            "the .stdout file"
        );
    }
    assert_eq!(untimed(&stdout(&a)), out, "stdout");
    assert_eq!(untimed(&stderr(&a)), err, "stderr");
}

macro_rules! gate {
    ($($name:ident),+ $(,)?) => {
        /// The programs this macro made a test for.
        pub const PROGRAMS: &[&str] = &[$(stringify!($name)),+];
        $(
            #[test]
            fn $name() {
                super::gate_program(stringify!($name));
            }
        )+
    };
}

/// The gate's `tests/runtime/` programs, by behaviour (spec §9.2).
mod gate {
    gate!(
        // Control flow and constants.
        break_continue,
        constants,
        for_loops,
        // Data.
        array_repeat,
        arrays,
        collections,
        field_assign,
        map_keys,
        match_enum,
        records,
        // Functions, generics and traits.
        assoc_fn,
        assoc_types,
        closures,
        generic_impls,
        generic_trait_methods,
        generics,
        iterator,
        method_generics,
        supertraits,
        trait_assoc_fn,
        traits,
        where_clauses,
        // std.
        prelude_option_result,
        std_core,
        strings,
        // Bytes.
        bytes_basics,
        bytes_hex,
        // Formatting.
        fmt_float_edge,
        fmt_float_fixed,
        fmt_int_pad,
        fmt_string_pad,
        // JSON.
        json_depth_empty,
        json_depth_leaf,
        json_object_forged_map,
        json_parse_numbers,
        json_parse_strings,
        json_parse_values,
        json_render_deep,
        json_render_guard_under,
        json_round_trip,
        json_stringify,
        json_stringify_escapes,
        // Channels, without a task.
        channel_clamps_buffer_below_one,
        channel_close_refuses_send,
        channel_close_then_drain,
        channel_enqueue_wraps_after_dequeue,
        channel_fifo_order,
        channel_full_refuses,
        channel_uncontended,
        // Time.
        system_time_iso8601,
        system_time_now,
        time_conversions,
        time_saturation,
        // Tasks, under `block_on`.
        async_tasks,
        interpolation_nary,
        spawned_output_survives_release,
        task_sleep_order,
        // `async fn main`, on the event loop: sleeps and timeouts.
        time_elapsed,
        timeout_elapsed,
        timeout_elapsed_then_join,
        timeout_elapsed_twice,
        timeout_join_elapsed,
        timeout_join_ok,
        timeout_ok,
        timeout_value,
        // Mutexes and channels between tasks.
        channel_recv_suspends_only_to_retry,
        channel_send_refuses_when_closed,
        channel_send_suspends_only_to_retry,
        channel_two_tasks_blocking,
        sync_mutex_int_set_serialises,
        sync_mutex_release_is_idempotent,
        sync_mutex_stale_guard_cannot_steal,
        sync_mutex_stale_guard_cannot_write,
        sync_mutex_try_lock_fails_when_held,
        sync_mutex_two_tasks_serialise,
        sync_mutex_uncontended,
        // Bytes, hashing and JSON's traits, which 4.0's build ran already.
        bytes_api,
        hash,
        hash_diffusion,
        json_traits,
        // `std/ui`: reactivity, rendering to a string, and its state across
        // turns (spec 4.3 §9.2).
        ui_cleanup,
        ui_collect,
        ui_memo,
        ui_render,
        ui_signal,
        // `component` and `view { … }` (spec 4.4 §9.2).
        ui_view,
        // `render_hydratable` (spec 4.6 §9.2).
        ui_hydratable,
    );

    /// The programs a test of their own in this module checks, by its name.
    pub const ALSO: &[&str] = &[
        "panic_unwrap",
        "bytes_from_ints_too_high",
        "bytes_from_ints_too_low",
        "json_render_guard",
        "json_stringify_cycle",
        "log_default_level",
        "log_init_with_threshold",
        "log_reconfigure_after_logging",
        "log_init_resets_to_default",
        "log_stdout_output",
        "log_level_labels",
        "forged_join_handle",
        "recycled_task_state",
        "task_deadlock",
        "eprint_family",
        "ui_memo_set",
        "ui_loop",
        "ui_render_bad_name",
        "ui_router",
        "ui_router_no_to",
        "ui_router_server",
    ];

    /// `eprint` and `eprintln` write to stderr, as `run_tests.rs` checks
    /// natively.
    #[test]
    fn eprint_family() {
        let Some(node) = super::node() else { return };
        let source = super::repo_root().join("tests/runtime/eprint_family.nova");
        let expected = std::fs::read_to_string(source.with_extension("stdout"))
            .expect("the expected output")
            .replace("\r\n", "\n");
        let dir = super::fresh("gate-eprint_family");
        let dist = super::build_wasm(&dir, &source);
        let a = super::run_wasm(node, &dist).success();
        assert_eq!(super::stdout(&a), expected);
        assert_eq!(super::stderr(&a), "ab\n");
    }

    /// `examples/03-producer-consumer`: two tasks and a channel (spec 4.1
    /// §1).
    #[test]
    fn producer_consumer_example() {
        let Some(node) = super::node() else { return };
        let dir = super::fresh("gate-producer-consumer");
        let dist = super::build_wasm(
            &dir,
            &super::repo_root().join("examples/03-producer-consumer/src/main.nova"),
        );
        let a = super::run_wasm(node, &dist).success();
        assert_eq!(
            super::stdout(&a),
            "got 1\ngot 2\ngot 3\ngot 4\ngot 5\ndone\n"
        );
    }

    #[test]
    fn forged_join_handle() {
        super::gate_panics(
            "forged_join_handle",
            "nova: panic: nova_rt_task_join_future: this future was never spawned, so there is no task to ask about",
            str::is_empty,
        );
    }

    #[test]
    fn recycled_task_state() {
        super::gate_panics(
            "recycled_task_state",
            "nova: panic: nova_rt_task_join_future: this future was never spawned, so there is no task to ask about",
            str::is_empty,
        );
    }

    #[test]
    fn task_deadlock() {
        super::gate_panics(
            "task_deadlock",
            "nova: deadlock: 2 tasks are parked and none can wake\n  \
             task 1 is waiting for task 2 to finish\n  \
             task 2 is waiting for task 1 to finish\n",
            str::is_empty,
        );
    }

    #[test]
    fn log_default_level() {
        super::gate_log(
            "log_default_level",
            &["done"],
            &[
                "<time> INFO yes-info",
                "<time> WARN yes-warn",
                "<time> ERROR yes-error",
            ],
        );
    }

    #[test]
    fn log_init_with_threshold() {
        super::gate_log(
            "log_init_with_threshold",
            &["done"],
            &["<time> WARN kept-warn", "<time> ERROR kept-error"],
        );
    }

    #[test]
    fn log_reconfigure_after_logging() {
        super::gate_log(
            "log_reconfigure_after_logging",
            &["done"],
            &["<time> INFO before", "<time> ERROR after-kept"],
        );
    }

    #[test]
    fn log_init_resets_to_default() {
        super::gate_log(
            "log_init_resets_to_default",
            &["done"],
            &["<time> INFO yes"],
        );
    }

    #[test]
    fn log_stdout_output() {
        super::gate_log("log_stdout_output", &["<time> INFO on-stdout"], &[]);
    }

    #[test]
    fn log_level_labels() {
        super::gate_log(
            "log_level_labels",
            &[
                "<time> TRACE a",
                "<time> DEBUG b",
                "<time> INFO c",
                "<time> WARN d",
                "<time> ERROR e",
            ],
            &[],
        );
    }

    #[test]
    fn ui_memo_set() {
        super::gate_panics(
            "ui_memo_set",
            "nova: panic: std/ui: set() on a memo",
            |out| out == "doubled 2\n",
        );
    }

    #[test]
    fn ui_loop() {
        super::gate_panics(
            "ui_loop",
            "nova: panic: std/ui: effects did not settle",
            str::is_empty,
        );
    }

    #[test]
    fn ui_render_bad_name() {
        super::gate_panics(
            "ui_render_bad_name",
            "nova: panic: std/ui: \"p onclick=x\" is not a tag name",
            |out| out == "<my-tag>fine</my-tag>\n",
        );
    }

    /// A link with no `to` panics before it needs a page (spec 4.5 §3.3).
    #[test]
    fn ui_router_no_to() {
        super::gate_panics(
            "ui_router_no_to",
            "nova: panic: link: no `to` attribute",
            |out| out == "<a href=\"/fine\" data-nova-link=\"\">fine</a>\n",
        );
    }

    /// `std/ui/router`'s paths and queries (spec 4.5 §3.2 to §3.5): a
    /// router reads the page's location on WASM, so the program runs under
    /// the fake DOM, at `http://localhost/`, whose location is `/` as the
    /// native one is, and prints what it prints natively, as it is and under
    /// stress.
    #[test]
    fn ui_router() {
        router_program("ui_router");
    }

    /// The router's functions for a server (spec 4.6 §5, §9.2): on WASM too,
    /// under the fake DOM, as `ui_router` runs.
    #[test]
    fn ui_router_server() {
        router_program("ui_router_server");
    }

    /// `tests/runtime/<name>.nova` under the fake DOM, as it is and under
    /// stress, printing what it prints natively.
    fn router_program(name: &str) {
        let Some(_) = super::node() else { return };
        let tests = super::repo_root().join("tests");
        let source = tests.join(format!("runtime/{name}.nova"));
        let expected = std::fs::read_to_string(source.with_extension("stdout"))
            .expect("the expected output")
            .replace("\r\n", "\n");
        let dir = super::fresh(&format!("gate-{name}"));
        std::fs::copy(tests.join("dom/fake_dom.mjs"), dir.join("fake_dom.mjs"))
            .expect("copy the fake DOM");
        std::fs::write(
            dir.join("run.mjs"),
            "import { install } from \"./fake_dom.mjs\";\ninstall();\n\
             const { default: init } = await import(\"./dist/app.js\");\nawait init();\n",
        )
        .expect("the harness");
        super::build_wasm(&dir, &source);
        for env in [&[][..], &super::STRESS[..]] {
            let a = super::Command::new("node")
                .arg(dir.join("run.mjs"))
                .envs(env.iter().copied())
                .timeout(std::time::Duration::from_secs(60))
                .assert()
                .success();
            assert_eq!(super::stdout(&a), expected, "{}", super::stderr(&a));
        }
    }

    #[test]
    fn panic_unwrap() {
        super::gate_panics("panic_unwrap", "nova: panic: called get on None", |out| {
            out.contains('7') && !out.contains("unreachable")
        });
    }

    #[test]
    fn bytes_from_ints_too_high() {
        super::gate_panics(
            "bytes_from_ints_too_high",
            "nova: panic: nova_rt_bytes_from_ints: element out of range 0..=255",
            |out| !out.contains('1'),
        );
    }

    #[test]
    fn bytes_from_ints_too_low() {
        super::gate_panics(
            "bytes_from_ints_too_low",
            "nova: panic: nova_rt_bytes_from_ints: element out of range 0..=255",
            |out| !out.contains('1'),
        );
    }

    #[test]
    fn json_render_guard() {
        super::gate_panics(
            "json_render_guard",
            "nova: panic: stringify: nesting too deep or cyclic value",
            |out| out == "built, about to render\n",
        );
    }

    #[test]
    fn json_stringify_cycle() {
        super::gate_panics(
            "json_stringify_cycle",
            "nova: panic: stringify: nesting too deep or cyclic value",
            |out| out == "about to render a cyclic value\n",
        );
    }
}

macro_rules! stressed {
    ($($name:ident),+ $(,)?) => {
        $(
            #[test]
            fn $name() {
                super::gate_program_with(
                    stringify!($name),
                    "stress",
                    &[("NOVA_GC_STRESS", "1")],
                );
            }
        )+
    };
}

/// Under `NOVA_GC_STRESS`, which collects at the end of every turn, the
/// gate's async programs, and the programs native runs under it, print the
/// same (spec 4.1 §9.3). A program with no `async fn main` has one turn,
/// and never collects.
mod stress {
    stressed!(
        // `async fn main`.
        time_elapsed,
        timeout_elapsed,
        timeout_elapsed_then_join,
        timeout_elapsed_twice,
        timeout_join_elapsed,
        timeout_join_ok,
        timeout_ok,
        timeout_value,
        channel_recv_suspends_only_to_retry,
        channel_send_refuses_when_closed,
        channel_send_suspends_only_to_retry,
        channel_two_tasks_blocking,
        sync_mutex_int_set_serialises,
        sync_mutex_release_is_idempotent,
        sync_mutex_stale_guard_cannot_steal,
        sync_mutex_stale_guard_cannot_write,
        sync_mutex_try_lock_fails_when_held,
        sync_mutex_two_tasks_serialise,
        sync_mutex_uncontended,
        // What `run_tests.rs` runs under it natively.
        async_tasks,
        interpolation_nary,
        spawned_output_survives_release,
        std_core,
        collections,
        strings,
        assoc_types,
        iterator,
        json_round_trip,
        json_stringify_escapes,
        json_parse_strings,
        bytes_api,
        ui_cleanup,
        ui_collect,
        ui_memo,
        ui_render,
        ui_signal,
        ui_hydratable,
    );

    #[test]
    fn recycled_task_state() {
        super::gate_panics_with(
            "recycled_task_state",
            "stress",
            &[("NOVA_GC_STRESS", "1")],
            "nova: panic: nova_rt_task_join_future: this future was never spawned, so there is no task to ask about",
            str::is_empty,
        );
    }
}

/// The gate of 4.3 (spec
/// `docs/superpowers/specs/2026-10-17-phase-4-3-std-ui-reactivity-and-views-design.md`
/// §1): the counter written with a signal and builder calls, which three
/// clicks make read 3. Under stress the collector runs after `main` and
/// after each click, and keeps what the page uses through `std/ui`'s state:
/// the button's listener, one call, and its handles.
#[test]
fn ui_the_builder_counter_counts_clicks() {
    dom_program("ui_counter", &[]);
    let err = dom_program("ui_counter", &STRESS);
    if node().is_none() {
        return;
    }
    assert_eq!(live_after_each(&err).len(), 4, "{err}");
    assert_eq!(calls_after_each(&err), [1, 1, 1, 1], "{err}");
}

/// `examples/06-counter-spa`, built from its project into `dir/dist`, with
/// `NOVA_HOME` beside `dir`.
fn build_counter_spa(dir: &Path) -> PathBuf {
    let dist = dir.join("dist");
    let home = dir.with_file_name(format!(
        "{}-home",
        dir.file_name().expect("a name").to_string_lossy()
    ));
    nova_at(&repo_root().join("examples/06-counter-spa"), &home)
        .args(["build", "--target", "wasm", "-o", &dist.to_string_lossy()])
        .assert()
        .success();
    dist
}

/// The gate of 4.4 (spec
/// `docs/superpowers/specs/2026-10-17-phase-4-4-component-syntax-design.md`
/// §9.4): `06-counter-spa` takes its own page, and under the fake DOM `+`
/// three times and `-` once leave `Count: 2`, as it is and under stress,
/// where each collection keeps the same handles and the two buttons'
/// calls.
#[test]
fn the_counter_spa_counts_clicks() {
    let page = std::fs::read_to_string(repo_root().join("examples/06-counter-spa/index.html"))
        .expect("the example's page");
    let dir = fresh("counter-spa");
    let dist = build_counter_spa(&dir);
    assert_eq!(
        std::fs::read_to_string(dist.join("index.html")).expect("index.html"),
        page
    );
    if node().is_none() {
        return;
    }
    let tests = repo_root().join("tests/dom");
    let expected = std::fs::read_to_string(tests.join("counter_spa.stdout"))
        .expect("the expected output")
        .replace("\r\n", "\n");
    for file in ["fake_dom.mjs", "counter_spa.mjs"] {
        std::fs::copy(tests.join(file), dir.join(file)).expect("copy the harness");
    }
    for env in [&[][..], &STRESS[..]] {
        let mut node = Command::new("node");
        let a = node
            .arg(dir.join("counter_spa.mjs"))
            .envs(env.iter().copied())
            .timeout(std::time::Duration::from_secs(60))
            .assert()
            .success();
        assert_eq!(stdout(&a), expected, "{}", stderr(&a));
        if !env.is_empty() {
            let err = stderr(&a);
            let calls = calls_after_each(&err);
            assert!(!calls.is_empty(), "{err}");
            assert!(calls.iter().all(|&c| c == 2), "{err}");
        }
    }
}

/// The gate's size: `06-counter-spa`'s module and shim are under 50 KB
/// gzipped, without `wasm-opt` (spec 4.4 §1, §9.4).
#[test]
fn the_counter_spa_is_under_50_kb_gzipped() {
    use std::io::Write;
    let dist = build_counter_spa(&fresh("counter-spa-size"));
    let mut total = 0;
    for name in ["app.wasm", "app.js"] {
        let bytes = std::fs::read(dist.join(name)).expect("read a dist file");
        let mut gz = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::best());
        gz.write_all(&bytes).expect("compress");
        total += gz.finish().expect("compress").len();
    }
    println!("06-counter-spa: {total} bytes gzipped");
    assert!(total < 50 * 1024, "{total} bytes gzipped");
}

/// `show` and `for_each` rebuild their part of the page between their
/// markers, and a hidden branch's cleanups run (spec 4.3 §6.3). Under
/// stress, a rebuilt row's listener leaves std/rt's table: after two more
/// rows, the page holds eight calls, not thirteen. And the old rows' handles
/// are released (§9.3): setting the same rows again leaves as many live, and
/// once no row is left, 14 are, where a page that kept the rows built first
/// would hold their text's too.
#[test]
fn ui_show_and_for_each_rebuild_their_part() {
    dom_program("ui_show_for_each", &[]);
    let err = dom_program("ui_show_for_each", &STRESS);
    if node().is_none() {
        return;
    }
    assert_eq!(calls_after_each(&err), [6, 6, 7, 8, 8, 8, 8, 4], "{err}");
    let live: Vec<u64> = handles_released(&err).iter().map(|&(_, l)| l).collect();
    assert_eq!(live, [20, 20, 23, 26, 26, 26, 26, 14], "{err}");
}

/// A dynamic class, a `value` property, fixed and dynamic, and `on_input`
/// given what was typed; a handler runs in a batch and reads without
/// subscribing, even when an effect clicked (spec 4.3 §5.2, §6.2).
#[test]
fn ui_attributes_follow_their_signals() {
    dom_program("ui_attrs", &[]);
    dom_program("ui_attrs", &STRESS);
}

/// A mount goes after what its element holds; `on_mount` runs once the
/// mount's nodes are in the page, in order, but not for a render disposed
/// first, and at once outside a mount; a selector nothing matches panics
/// (spec 4.3 §5.2, §7.1).
#[test]
fn ui_mount_runs_on_mount_and_panics_without_a_match() {
    for env in [&[][..], &STRESS[..]] {
        let Some(a) = dom_run("ui_mount", env) else {
            return;
        };
        let err = stderr(&a.code(134));
        assert!(
            err.contains("nova: panic: std/ui: mount: no element matches \"#nope\"\n"),
            "{err}"
        );
    }
}

/// `mount` checks names as `render_to_string` does, and refuses
/// `innerHTML` before any node of the view is in the page (spec 4.3 §7.2).
#[test]
fn ui_mount_never_inserts_html() {
    for env in [&[][..], &STRESS[..]] {
        let Some(a) = dom_run("ui_mount_html", env) else {
            return;
        };
        let err = stderr(&a.code(134));
        assert!(
            err.contains(
                "nova: panic: std/ui: \"innerHTML\" parses HTML, which std/ui does not insert\n"
            ),
            "{err}"
        );
    }
}

/// A hydration test of `tests/dom/` (spec
/// `docs/superpowers/specs/2026-10-19-phase-4-6-ssr-hydration-and-ssg-design.md`
/// §9.3), the pair end to end: the project `tests/dom/<name>/` has a server,
/// `src/server.nova`, which, run natively from the project, prints a line
/// per root of the page, its id and the HTML `render_hydratable` writes for
/// it; and a client, `src/main.nova`, which, built for WASM, hydrates those
/// roots. The harness, `<name>.mjs`, reads the server's output from
/// `server.txt` beside it, puts each root's HTML into the page through the
/// fake DOM's parser, runs the client with `env` set, and drives and prints
/// the page. Its stdout is `<name>.stdout`. Returns its stderr.
fn hydration_program(name: &str, env: &[(&str, &str)]) -> String {
    let Some(mut node) = node() else {
        return String::new();
    };
    let tests = repo_root().join("tests/dom");
    let project = tests.join(name);
    let expected = std::fs::read_to_string(tests.join(format!("{name}.stdout")))
        .expect("the expected output")
        .replace("\r\n", "\n");
    let kind = if env.is_empty() {
        "hydrate"
    } else {
        "hydrate-stress"
    };
    let dir = fresh(&format!("{kind}-{name}"));
    let home = dir.join("nova-home");
    let server = nova_at(&project, &home)
        .args(["run", "src/server.nova"])
        .assert()
        .success();
    std::fs::write(dir.join("server.txt"), stdout(&server)).expect("write the server's output");
    let dist = dir.join("dist");
    nova_at(&project, &home)
        .args(["build", "--target", "wasm", "-o", &dist.to_string_lossy()])
        .assert()
        .success();
    for file in ["fake_dom.mjs".to_string(), format!("{name}.mjs")] {
        std::fs::copy(tests.join(&file), dir.join(&file)).expect("copy the harness");
    }
    let a = node
        .arg(dir.join(format!("{name}.mjs")))
        .envs(env.iter().copied())
        .timeout(std::time::Duration::from_secs(60))
        .assert()
        .success();
    assert_eq!(stdout(&a), expected, "{}", stderr(&a));
    stderr(&a)
}

/// Hydration in place (spec 4.6 §4.1, §4.2): each root keeps the server's
/// nodes, as the same objects and in order, and gains only the empty texts
/// the server could not write. `on_mount` runs once the walk has added the
/// handlers, so the click it makes counts. Clicks update the adopted nodes,
/// a dynamic attribute and a property; an adopted row's handler runs; `show`
/// and `for_each` build again between the server's markers, nested regions
/// too; texts side by side, empty ones and a carriage return are adopted,
/// and the dynamic one writes to its own node; `hydrate_with` reads the
/// server's data, escaped for a `script` element, from the `script` after
/// its root, past whitespace and a comment, though the view holds an
/// element whose id is `nova-data` too; with no data after its root,
/// `hydrate_with` walks a root the server filled with its fallback, and
/// builds its fallback's view into an empty one (the user's decision 13,
/// ADR 0045), as it does when the first element after the root holds other
/// guests as JSON but is a `div` with the data's id and type, or a `script`
/// with its id and no type, or a `p`, the data's `script` coming only after
/// it; a root with nothing, or only whitespace, a form feed among it, gets
/// the view built, the whitespace removed first, with no report; and a text
/// of 70,000 characters, fixed or dynamic, which the fake DOM's parser
/// splits in two as Chromium's does, is joined into its first node, the
/// other removed, with no report (ADR 0045). Under stress the collector
/// keeps the adopted nodes' seven handlers, and the rebuilt rows' three
/// listeners replace the adopted rows' two: eight calls.
#[test]
fn hydrate_adopts_the_servers_nodes() {
    hydration_program("hydrate", &[]);
    let err = hydration_program("hydrate", &STRESS);
    if node().is_none() {
        return;
    }
    assert_eq!(calls_after_each(&err), [7, 7, 7, 7, 7, 8, 8, 8, 8], "{err}");
}

/// A mismatch (spec 4.6 §4.3): for each root, the first difference, in the
/// spec's words, reported with `console.error`: another tag, another text
/// (each side clipped to 40 characters), a missing region, its end, a
/// missing node, a node left over in an element and in the root, a missing
/// separator, or one where the client has no text, and a comment that is no
/// marker, which a server written by hand can write. Then the root
/// holds the client's view, built afresh, and none of the server's nodes.
/// The walk's cleanups run; its `on_mount` callbacks never do; and its
/// effects and handlers stop: a click on the server's button, which the walk
/// had adopted, does nothing, and the count's effect runs once for a click
/// on the client's. Under stress, the client's button's call is the only
/// one left in std/rt's table.
#[test]
fn hydrate_reports_a_mismatch_and_builds_again() {
    hydration_program("hydrate_mismatch", &[]);
    let err = hydration_program("hydrate_mismatch", &STRESS);
    if node().is_none() {
        return;
    }
    let calls = calls_after_each(&err);
    assert!(!calls.is_empty(), "{err}");
    assert!(calls.iter().all(|&c| c == 1), "{err}");
}

/// `hydrate` and `hydrate_with` panic (spec 4.6 §4.1): when no element
/// matches, as `mount` does, naming the function; and when the data's JSON,
/// in the `nova-data` script after the root, does not parse, or its
/// `from_json` refuses it. A page with no data is no panic: `hydrate_with`
/// uses its fallback (the user's decision 13, ADR 0045), as
/// `hydrate_adopts_the_servers_nodes` checks.
#[test]
fn hydrate_panics_without_its_root_or_on_data_that_does_not_parse() {
    let Some(_) = node() else { return };
    let dir = fresh("hydrate-panics");
    std::fs::copy(
        repo_root().join("tests/dom/fake_dom.mjs"),
        dir.join("fake_dom.mjs"),
    )
    .expect("copy the fake DOM");
    for (name, call, data, message) in [
        (
            "root",
            "hydrate(\"#nope\", | | text(\"x\"))",
            None,
            "std/ui: hydrate: no element matches \"#nope\"",
        ),
        (
            "root_with",
            "hydrate_with(\"#nope\", \"none\", |s: String| text(s))",
            Some("\"x\""),
            "std/ui: hydrate_with: no element matches \"#nope\"",
        ),
        (
            "bad_json",
            "hydrate_with(\"#app\", \"none\", |s: String| text(s))",
            Some("[\"x\","),
            "std/ui: hydrate_with: the page's data does not parse: expected a value",
        ),
        (
            "bad_value",
            "hydrate_with(\"#app\", \"none\", |s: String| text(s))",
            Some("7"),
            "std/ui: hydrate_with: the page's data does not parse: expected a string",
        ),
    ] {
        let data = match data {
            Some(json) => format!(
                "const data = document.createElement(\"script\");\n\
                 data.setAttribute(\"type\", \"application/json\");\n\
                 data.setAttribute(\"id\", \"nova-data\");\n\
                 data.textContent = {json:?};\n\
                 document.body.appendChild(data);\n"
            ),
            None => String::new(),
        };
        write(
            &dir,
            &[
                (
                    &format!("{name}/main.nova"),
                    &format!(
                        "import ui\nimport ui::html::{{text}}\n\nfn main() {{\n    {call}\n    println(\"not reached\")\n}}\n"
                    ),
                ),
                (
                    &format!("{name}/run.mjs"),
                    &format!(
                        "import {{ install }} from \"../fake_dom.mjs\";\ninstall();\n\
                         const app = document.createElement(\"div\");\n\
                         app.setAttribute(\"id\", \"app\");\n\
                         app.innerHTML = \"x\";\n\
                         document.body.appendChild(app);\n{data}\
                         const {{ default: init }} = await import(\"./dist/app.js\");\nawait init();\n"
                    ),
                ),
            ],
        );
        build_wasm(&dir.join(name), Path::new("main.nova"));
        let a = Command::new("node")
            .arg(dir.join(name).join("run.mjs"))
            .timeout(std::time::Duration::from_secs(60))
            .assert()
            .code(134);
        assert_eq!(stdout(&a), "", "{name}");
        let err = stderr(&a);
        assert!(
            err.contains(&format!("nova: panic: {message}\n")),
            "{name}: {err}"
        );
    }
}

/// The router by the hash (spec
/// `docs/superpowers/specs/2026-10-18-phase-4-5-router-and-nova-dev-design.md`
/// §3, §5.2): the root route at `/`; a link's click, its default prevented,
/// that builds its route once though the `hashchange` it causes comes a turn
/// later; `navigate` with a query; the back button; a typed hash only `*`
/// matches; `navigate` to `/posts/café` and to `/posts/a b`, each built
/// once, though the hash holds them percent-encoded; to `/posts/6#sec`,
/// whose second `#` starts a fragment the location drops; and a
/// percent-encoded parameter, from a `to` with no `/`. The menu's links,
/// built beside a `show` of the router before it shows, have rooted paths
/// until it shows, its `#` while it is shown, and rooted paths again once
/// it is hidden, when a link's click is the browser's. Under stress the
/// collector runs after each step and keeps the router's state, at std/rt's
/// word 112, and the five buttons' handlers: five calls before the router
/// shows, seven with its two window listeners while it is shown, and five
/// once it is hidden.
#[test]
fn router_routes_by_the_hash() {
    dom_program("router_hash", &[]);
    let err = dom_program("router_hash", &STRESS);
    if node().is_none() {
        return;
    }
    let calls = calls_after_each(&err);
    let shown = calls
        .iter()
        .position(|&c| c != 5)
        .expect("a collection with the router shown");
    assert!(shown > 0, "{err}");
    let hidden = shown
        + calls[shown..]
            .iter()
            .position(|&c| c != 7)
            .expect("a collection once the router is hidden");
    assert!(calls[hidden..].iter().all(|&c| c == 5), "{err}");
}

/// Two routers on a page (spec 4.5 §3.1, §3.3): the router mounted later
/// takes a link's click, its default prevented, and both show the route;
/// once the later one is disposed, no router is the one mounted last, and
/// the earlier one, still listening, leaves a link's click to the browser:
/// its default is not prevented, and the router does not navigate (the fake
/// DOM follows no link, so the page stays as it was). Under stress the
/// collector keeps both routers' window listeners and the button's handler,
/// five calls, until the later router's two leave std/rt's table: three.
#[test]
fn a_router_not_mounted_last_leaves_a_click_to_the_browser() {
    dom_program("router_two", &[]);
    let err = dom_program("router_two", &STRESS);
    if node().is_none() {
        return;
    }
    let calls = calls_after_each(&err);
    let disposed = calls
        .iter()
        .position(|&c| c != 5)
        .expect("a collection after the disposal");
    assert!(disposed > 0, "{err}");
    assert!(calls[disposed..].iter().all(|&c| c == 3), "{err}");
}

/// The router by the history API (spec 4.5 §3, §5.2): the same steps as by
/// the hash, through `pushState` and `popstate`; then the clicks left to the
/// browser, their default not prevented and the page as it was: a link's
/// click with each modifier key and with another button, a link with a
/// `target` or a `download`, and a plain `<a>`. A click on a link's child
/// navigates. A handler reads no route's parameters, and `navigate` to the
/// current location written decoded adds no entry, while a link to
/// `/posts/a%2Fb` from `/posts/a/b`, or to `?q=a%26b=c` from `?q=a&b=c`,
/// adds one. Once the router's owner is disposed, a link's click is the
/// browser's, and `navigate` panics. Under stress the router's two
/// listeners leave std/rt's table then: seven calls become five.
#[test]
fn router_routes_by_the_history_api() {
    for env in [&[][..], &STRESS[..]] {
        let Some(a) = dom_run("router_history", env) else {
            return;
        };
        let err = stderr(&a.code(134));
        assert!(
            err.contains("nova: panic: navigate: no router is mounted\n"),
            "{err}"
        );
        if !env.is_empty() {
            let calls = calls_after_each(&err);
            let (last, before) = calls.split_last().expect("collections");
            assert_eq!(*last, 5, "{err}");
            assert!(!before.is_empty(), "{err}");
            assert!(before.iter().all(|&c| c == 7), "{err}");
        }
    }
}

/// `import ui::html::{div}` checks on both targets; a program's own nested
/// path is still E0900, `ui::nope` is E0001, and `signal` without an import
/// is E0001 (spec 4.3 §9.5).
#[test]
fn ui_imports_check_on_both_targets() {
    let dir = fresh("ui-imports");
    write(
        &dir,
        &[
            (
                "ok.nova",
                "import ui\nimport ui::html::{div, text}\n\nfn main() {\n    println(render_to_string(| | div([], [text(\"x\")])))\n}\n",
            ),
            ("nested.nova", "import a::b\n\nfn main() {}\n"),
            ("nope.nova", "import ui::nope\n\nfn main() {}\n"),
            ("bare.nova", "fn main() {\n    let s = signal(0)\n}\n"),
        ],
    );
    for target in ["native", "wasm"] {
        nova_in(&dir, &["check", "--target", target, "ok.nova"]).success();
        for (file, code, says) in [
            ("nested.nova", "E0900", "qualified or nested import paths"),
            ("nope.nova", "E0001", "cannot find module `ui::nope`"),
            ("bare.nova", "E0001", "cannot find function `signal`"),
        ] {
            let a = nova_in(&dir, &["check", "--target", target, file]).failure();
            let err = stderr(&a);
            assert!(
                err.contains(&format!("error[{code}]")) && err.contains(says),
                "{target} {file}: {err}"
            );
        }
    }
}

/// The `tests/runtime/` programs WASM does not run, each with its reason:
/// the native-only std modules it names, each E0092 on WASM, or the error
/// that is why it never can (spec 4.1 §9.2).
const LEFT_OUT: &[(&str, &str)] = &[
    ("bytes_reserved", "E0089 (its error is the point)"),
    ("crypto_hashes", "std/crypto"),
    ("crypto_pbkdf2", "std/crypto"),
    ("crypto_pbkdf2_refused", "std/crypto, std/process"),
    ("crypto_random", "std/crypto"),
    ("extern_ffi", "E0090 (an extern block)"),
    ("file_errors", "std/fs, std/io"),
    ("file_lifetime", "std/fs, std/io"),
    ("file_open_dir", "std/fs, std/io"),
    ("file_roundtrip", "std/fs"),
    ("fs_already_exists", "std/fs, std/io"),
    ("fs_bytes_errors", "std/fs, std/io"),
    ("fs_bytes_roundtrip", "std/fs"),
    ("fs_dirs", "std/fs"),
    ("fs_invalid_data", "std/fs, std/io"),
    ("fs_io_types", "std/io"),
    ("fs_not_found", "std/fs, std/io"),
    ("fs_permission_denied", "std/fs, std/io"),
    ("fs_read_dir", "std/fs"),
    ("fs_rename", "std/fs, std/io"),
    ("fs_roundtrip", "std/fs"),
    ("fs_write_then_read", "std/fs"),
    ("http_keepalive", "std/http, std/net"),
    ("http_limits", "std/http"),
    ("http_malformed", "std/http"),
    ("http_offsets", "std/http"),
    ("http_partial", "std/http"),
    ("http_serialise", "std/http"),
    ("http_listen_on", "std/http, std/net, std/process"),
    ("http_server_dispatch", "std/http"),
    ("io_broken_stderr", "std/io"),
    ("io_broken_stdout", "std/io"),
    ("io_read_stdin_write_only", "std/io"),
    ("io_read_to_end", "std/io"),
    ("io_streams", "std/io"),
    ("net_interleave", "std/fs, std/net"),
    ("net_lifetime", "std/fs, std/io, std/net"),
    ("net_listener_accept", "std/net"),
    ("net_refused", "std/fs, std/io, std/net"),
    ("net_roundtrip", "std/fs, std/net"),
    ("net_timeout", "std/fs, std/io, std/net"),
    ("nova_test", "E0601 (a test file, which `nova test` runs)"),
    ("process_args", "std/process"),
    ("process_exit", "std/io, std/process"),
];

/// Every program in `tests/runtime/` is in the gate or left out, once, and
/// each name in the lists is a program: a new program must join one or the
/// other (spec 4.1 §9.2).
#[test]
fn every_runtime_program_is_in_the_gate_or_left_out() {
    let mut programs: Vec<String> = std::fs::read_dir(repo_root().join("tests/runtime"))
        .expect("tests/runtime")
        .filter_map(|e| {
            let path = e.expect("an entry").path();
            (path.extension()? == "nova").then(|| {
                path.file_stem()
                    .expect("a name")
                    .to_string_lossy()
                    .into_owned()
            })
        })
        .collect();
    programs.sort();
    let mut listed: Vec<String> = gate::PROGRAMS
        .iter()
        .chain(gate::ALSO)
        .chain(LEFT_OUT.iter().map(|(n, _)| n))
        .map(|n| n.to_string())
        .collect();
    listed.sort();
    assert_eq!(listed, programs);
}

/// Each program left out is refused on WASM for its reason: E0092 naming
/// exactly its modules, or its error (spec 4.1 §9.2).
#[test]
fn each_left_out_program_is_refused_for_its_reason() {
    let dir = fresh("left-out");
    for (name, reason) in LEFT_OUT {
        let source = repo_root()
            .join("tests/runtime")
            .join(format!("{name}.nova"));
        let a = nova_in(
            &dir,
            &["check", "--target", "wasm", &source.to_string_lossy()],
        )
        .failure();
        let err = stderr(&a);
        if reason.starts_with("std/") {
            let mut modules: Vec<&str> = err
                .lines()
                .filter(|l| l.starts_with("error[E0092]: "))
                .filter_map(|l| l.split('`').nth(3))
                .collect();
            modules.sort();
            modules.dedup();
            assert_eq!(modules.join(", "), *reason, "{name}: {err}");
        } else {
            assert!(
                err.contains(&format!("error[{}]", &reason[..5])),
                "{name}: {err}"
            );
        }
    }
}
