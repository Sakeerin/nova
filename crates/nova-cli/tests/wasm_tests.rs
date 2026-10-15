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

/// An allocation over 1 GB, the memory's limit, is refused with "out of
/// memory" before any page arithmetic could wrap (spec 4.1 §6.2).
#[test]
fn an_allocation_over_1_gb_is_out_of_memory() {
    let Some(node) = node() else { return };
    let dir = fresh("oom");
    write(
        &dir,
        &[(
            "main.nova",
            "fn main() {\n    println(\"before\")\n    let n = 200000000\n    let xs = [0; n]\n    println(\"${xs.len()}\")\n}\n",
        )],
    );
    let dist = build_wasm(&dir, Path::new("main.nova"));
    let a = run_wasm(node, &dist).code(134);
    assert_eq!(stdout(&a), "before\n");
    assert_eq!(stderr(&a), "nova: panic: out of memory\n");
}

/// One of the gate's programs (spec §9.2): built for WASM, it prints under
/// Node what its `.stdout` file says the native build prints.
fn gate_program(name: &str) {
    let Some(node) = node() else { return };
    let source = repo_root()
        .join("tests/runtime")
        .join(format!("{name}.nova"));
    let expected = std::fs::read_to_string(source.with_extension("stdout"))
        .expect("the expected output")
        .replace("\r\n", "\n");
    let dir = fresh(&format!("gate-{name}"));
    let dist = build_wasm(&dir, &source);
    let a = run_wasm(node, &dist).success();
    assert_eq!(stdout(&a), expected);
}

/// One of the gate's panicking programs: it fails with the native build's
/// message on stderr and an abort's status, as `run_tests.rs` checks it
/// natively, and what it printed first passes `printed`. It ends by its
/// own panic, not by a stack overflow or a trap.
fn gate_panics(name: &str, message: &str, printed: fn(&str) -> bool) {
    let Some(node) = node() else { return };
    let source = repo_root()
        .join("tests/runtime")
        .join(format!("{name}.nova"));
    let dir = fresh(&format!("gate-{name}"));
    let dist = build_wasm(&dir, &source);
    let a = run_wasm(node, &dist).code(134);
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
    );

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
