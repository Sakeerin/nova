//! `nova build --ssr` and `--ssg` (spec
//! `docs/superpowers/specs/2026-10-19-phase-4-6-ssr-hydration-and-ssg-design.md`
//! §7, §9.4), on copies of `tests/ssr/site`: a project whose client,
//! `src/main.nova`, and server, `src/server.nova`, share `src/app.nova`,
//! with three static routes, one with a parameter, and the catch-all.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use assert_cmd::assert::Assert;
use assert_cmd::Command;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// A copy of `tests/ssr/site` in a fresh directory with a fixed name, so each
/// run replaces the last, and the fresh `NOVA_HOME` beside it.
fn site(name: &str) -> (PathBuf, PathBuf) {
    let base = std::env::temp_dir().join(format!("nova-ssr-build-{name}"));
    let _ = std::fs::remove_dir_all(&base);
    let dir = base.join("site");
    std::fs::create_dir_all(dir.join("src")).expect("create the project");
    let fixture = repo_root().join("tests/ssr/site");
    std::fs::copy(fixture.join("nova.toml"), dir.join("nova.toml")).expect("copy nova.toml");
    for entry in std::fs::read_dir(fixture.join("src")).expect("the fixture's sources") {
        let path = entry.expect("an entry").path();
        let to = dir.join("src").join(path.file_name().expect("a name"));
        std::fs::copy(&path, to).expect("copy a source");
    }
    (dir, base.join("home"))
}

/// `nova` in `dir`, with `NOVA_HOME` at `home`, no colour and no backtrace.
fn nova_at(dir: &Path, home: &Path) -> Command {
    let mut cmd = Command::cargo_bin("nova").expect("nova binary builds");
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

/// The server's executable under `target/<profile>/`.
fn server(dir: &Path, profile: &str) -> PathBuf {
    dir.join(built_server(profile))
}

/// The server's executable from the project's root, `target/<profile>/`
/// and its name, joined as `nova` joins them, so it prints as `nova` does
/// on every system.
fn built_server(profile: &str) -> PathBuf {
    Path::new("target")
        .join(profile)
        .join(format!("site-server{}", std::env::consts::EXE_SUFFIX))
}

/// The files `manifest.json` in `dist` lists, with their sizes, as
/// `bundle.rs` writes it: one `"path": size` line each.
fn listed(dist: &Path) -> BTreeMap<String, u64> {
    let text = std::fs::read_to_string(dist.join("manifest.json")).expect("manifest.json");
    let files = text.split("\"files\": {").nth(1).expect("a files object");
    files
        .lines()
        .filter_map(|line| {
            let (path, size) = line.trim().trim_end_matches(',').split_once("\": ")?;
            Some((path.trim_start_matches('"').to_string(), size.parse().ok()?))
        })
        .collect()
}

/// Each file the manifest lists has the size it gives.
fn sizes_match(dist: &Path) {
    for (path, size) in listed(dist) {
        let real = std::fs::metadata(dist.join(&path))
            .unwrap_or_else(|e| panic!("{path}: {e}"))
            .len();
        assert_eq!(real, size, "{path}");
    }
}

/// `--ssr` (spec §7): the client into `dist/`, as `--target wasm` builds it,
/// the page's root still empty and no page of a route written, and the
/// server into `target/debug/site-server`, which then runs: given `--ssg`,
/// it writes the pages. With `-o`, the client goes to the directory named,
/// and the server to the same place as before.
#[test]
fn ssr_builds_the_client_and_the_server() {
    let (dir, home) = site("ssr");
    let a = nova_at(&dir, &home)
        .args(["build", "--ssr"])
        .assert()
        .success();
    let exe = built_server("debug");
    assert_eq!(stdout(&a), format!("built dist\nbuilt {}\n", exe.display()));
    let dist = dir.join("dist");
    let names: Vec<String> = listed(&dist).into_keys().collect();
    assert_eq!(names, ["app.js", "app.wasm", "index.html", "package.json"]);
    sizes_match(&dist);
    let page = std::fs::read_to_string(dist.join("index.html")).expect("the page");
    assert!(page.contains("<div id=\"app\"></div>"), "{page}");
    assert!(!dist.join("about").exists());
    assert!(server(&dir, "debug").is_file());

    let generated = std::process::Command::new(server(&dir, "debug"))
        .current_dir(&dir)
        .args(["--ssg", "dist"])
        .output()
        .expect("run the server");
    assert!(generated.status.success(), "{generated:?}");
    assert!(dist.join("about/index.html").is_file());

    let _ = std::fs::remove_dir_all(&dist);
    std::fs::remove_file(server(&dir, "debug")).expect("remove the server");
    let a = nova_at(&dir, &home)
        .args(["build", "--ssr", "-o", "public_html"])
        .assert()
        .success();
    assert_eq!(
        stdout(&a),
        format!("built public_html\nbuilt {}\n", exe.display())
    );
    assert!(dir.join("public_html/app.wasm").is_file());
    assert!(!dist.exists());
    assert!(server(&dir, "debug").is_file());
}

/// `--ssg` (spec §7, §9.4): it builds as `--ssr` does, then runs the server
/// from the project's root with `--ssg dist`, whose output passes through.
/// That writes one page per static route, `/` over `dist/index.html`, and
/// none for the route with a parameter or the catch-all; and the manifest
/// lists each page with its size, and `index.html` with its new one. The
/// server's executable stays.
#[test]
fn ssg_writes_a_page_per_static_route_and_lists_them() {
    let (dir, home) = site("ssg");
    let a = nova_at(&dir, &home)
        .args(["build", "--ssg"])
        .assert()
        .success();
    assert_eq!(
        stdout(&a),
        format!(
            "built dist\nbuilt {}\n\
             ssg: wrote dist/index.html\n\
             ssg: wrote dist/about/index.html\n\
             ssg: wrote dist/docs/intro/index.html\n\
             stopped\n",
            built_server("debug").display()
        )
    );
    let dist = dir.join("dist");
    let names: Vec<String> = listed(&dist).into_keys().collect();
    assert_eq!(
        names,
        [
            "about/index.html",
            "app.js",
            "app.wasm",
            "docs/intro/index.html",
            "index.html",
            "package.json"
        ]
    );
    sizes_match(&dist);
    for (path, view) in [
        (
            "index.html",
            "<p>Home <a href=\"/about\" data-nova-link=\"\">About</a></p>",
        ),
        ("about/index.html", "<p>About</p>"),
        ("docs/intro/index.html", "<p>Intro</p>"),
    ] {
        let page = std::fs::read_to_string(dist.join(path)).expect("a page");
        assert!(
            page.contains(&format!("<!--[-->{view}<!--]-->")),
            "{path}: {page}"
        );
        assert!(page.contains("id=\"nova-data\""), "{path}: {page}");
        assert!(
            page.contains("import init from \"/app.js\""),
            "{path}: {page}"
        );
    }
    assert!(!dist.join("posts").exists());
    assert!(server(&dir, "debug").is_file());
}

/// A build removes the pages a previous `--ssg` wrote, which its manifest
/// lists, with the directories that leaves empty (ADR 0045): a `--target
/// wasm` build, as `nova dev` makes, and an `--ssr` build each leave no
/// page, and a manifest that lists none; and an `--ssg` build after a
/// route is removed leaves no page for it.
#[test]
fn a_build_removes_the_pages_ssg_wrote() {
    let (dir, home) = site("stale");
    let dist = dir.join("dist");
    let built = ["app.js", "app.wasm", "index.html", "package.json"];
    for build in [&["build", "--target", "wasm"][..], &["build", "--ssr"]] {
        nova_at(&dir, &home)
            .args(["build", "--ssg"])
            .assert()
            .success();
        assert!(dist.join("about/index.html").is_file(), "{build:?}");
        assert!(dist.join("docs/intro/index.html").is_file(), "{build:?}");
        nova_at(&dir, &home).args(build).assert().success();
        for gone in ["about", "docs"] {
            assert!(!dist.join(gone).exists(), "{build:?}: {gone}");
        }
        let names: Vec<String> = listed(&dist).into_keys().collect();
        assert_eq!(names, built, "{build:?}");
        let page = std::fs::read_to_string(dist.join("index.html")).expect("the page");
        assert!(page.contains("<div id=\"app\"></div>"), "{build:?}: {page}");
        assert!(!page.contains("nova-data"), "{build:?}: {page}");
    }

    nova_at(&dir, &home)
        .args(["build", "--ssg"])
        .assert()
        .success();
    let app = dir.join("src/app.nova");
    let source = std::fs::read_to_string(&app).expect("app.nova");
    let route = "        Route { path: \"/docs/intro\", component: Intro },\n";
    assert!(source.contains(route));
    std::fs::write(&app, source.replace(route, "")).expect("app.nova without the route");
    let a = nova_at(&dir, &home)
        .args(["build", "--ssg"])
        .assert()
        .success();
    assert!(!stdout(&a).contains("docs"), "{}", stdout(&a));
    assert!(!dist.join("docs").exists());
    assert!(dist.join("about/index.html").is_file());
    let names: Vec<String> = listed(&dist).into_keys().collect();
    assert_eq!(
        names,
        [
            "about/index.html",
            "app.js",
            "app.wasm",
            "index.html",
            "package.json"
        ]
    );
    sizes_match(&dist);
}

/// `--ssr --ssg` is `--ssg`, from a subdirectory too: the server still runs
/// from the project's root, and is given the client's directory, here the
/// one `-o` names from the subdirectory, so the pages land beside the
/// client's files and the manifest lists them.
#[test]
fn ssr_and_ssg_together_is_ssg_from_anywhere_in_the_project() {
    let (dir, home) = site("both");
    let src = dir.join("src");
    let a = nova_at(&src, &home)
        .args(["build", "--ssr", "--ssg", "-o", "../out"])
        .assert()
        .success();
    let out = stdout(&a);
    assert!(out.starts_with("built ../out\nbuilt "), "{out}");
    assert_eq!(out.matches("ssg: wrote ").count(), 3, "{out}");
    assert!(out.ends_with("/docs/intro/index.html\nstopped\n"), "{out}");
    let out_dir = dir.join("out");
    assert!(out_dir.join("about/index.html").is_file());
    assert!(out_dir.join("docs/intro/index.html").is_file());
    assert!(!dir.join("dist").exists() && !src.join("about").exists());
    assert!(listed(&out_dir).contains_key("docs/intro/index.html"));
    sizes_match(&out_dir);
}

/// A server that fails under `--ssg` fails the build with its exit code,
/// after its output, which `nova` names in the same words on every system:
/// the pages it wrote are on disk, but the manifest does not list them.
#[test]
fn a_failing_server_fails_ssg_with_its_status() {
    let (dir, home) = site("fails");
    std::fs::copy(dir.join("src/refuse.nova"), dir.join("src/server.nova"))
        .expect("the refusing server");
    let a = nova_at(&dir, &home)
        .args(["build", "--ssg"])
        .assert()
        .code(3);
    let out = stdout(&a);
    assert!(
        out.ends_with("ssg: wrote dist/index.html\nssg: wrote dist/about/index.html\n"),
        "{out}"
    );
    let err = stderr(&a);
    assert!(
        err.contains("ssg: the route path \"/a/../b\" names no file"),
        "{err}"
    );
    assert!(
        err.contains(&format!(
            "Error: the server failed to generate the pages, with exit code 3: {}\n",
            built_server("debug").display()
        )),
        "{err}"
    );
    assert!(dir.join("dist/about/index.html").is_file());
    assert!(!listed(&dir.join("dist")).contains_key("about/index.html"));
}

/// A server whose `ssg: wrote` line names a file outside the output
/// directory, `dist/../nova.toml`, fails `--ssg` once it has ended well,
/// with an error naming the line, and the manifest lists none of its pages,
/// not even the one inside `dist/` (ADR 0045).
#[test]
fn ssg_refuses_a_page_outside_the_output_directory() {
    let (dir, home) = site("stray");
    std::fs::copy(dir.join("src/stray.nova"), dir.join("src/server.nova"))
        .expect("the stray server");
    let a = nova_at(&dir, &home)
        .args(["build", "--ssg"])
        .assert()
        .failure();
    let out = stdout(&a);
    assert!(
        out.ends_with("ssg: wrote dist/index.html\nssg: wrote dist/../nova.toml\n"),
        "{out}"
    );
    let err = stderr(&a);
    assert!(
        err.contains(
            "Error: the server's line `ssg: wrote dist/../nova.toml` names no file inside dist\n"
        ),
        "{err}"
    );
    let names: Vec<String> = listed(&dir.join("dist")).into_keys().collect();
    assert_eq!(names, ["app.js", "app.wasm", "index.html", "package.json"]);
}

/// A server whose `ssg: wrote` line names its page by an absolute path in
/// the output directory has it listed, whether `nova` runs in the project's
/// root, which gives the server `--ssg dist`, or in `src/`, which gives it
/// that directory made absolute: a relative path is read from the server's
/// working directory, the project's root, and the output directory is made
/// absolute the same way, so the pages do not hang on where `nova` runs
/// (ADR 0045).
#[test]
fn ssg_takes_a_page_named_by_an_absolute_path() {
    let (dir, home) = site("absolute");
    // The project's root as `nova`'s working directory reads it there: with
    // its links resolved, as the system gives it, on Unix.
    let root = if cfg!(windows) {
        dir.clone()
    } else {
        dir.canonicalize().expect("the project's root")
    };
    let quoted = root
        .display()
        .to_string()
        .replace('\\', "\\\\")
        .replace('"', "\\\"");
    std::fs::write(
        dir.join("src/server.nova"),
        format!(
            "//! Given `--ssg <dir>`, writes `<dir>/abs/index.html` and names it by\n\
             //! an absolute path, `<dir>` read from the project's root if relative.\n\
             \n\
             async fn main() {{\n\
             \x20   let dir = args().get(2).unwrap_or(\"dist\")\n\
             \x20   let absolute = dir.starts_with(\"/\") || dir.slice(1, 2) == \":\"\n\
             \x20   let at = if absolute {{ dir }} else {{ \"{quoted}/${{dir}}\" }}\n\
             \x20   let _ = create_dir_all(\"${{at}}/abs\").await\n\
             \x20   let _ = write_string(\"${{at}}/abs/index.html\", \"absolute\").await\n\
             \x20   println(\"ssg: wrote ${{at}}/abs/index.html\")\n\
             }}\n"
        ),
    )
    .expect("the absolute server");
    for at in [dir.clone(), dir.join("src")] {
        let a = nova_at(&at, &home)
            .args(["build", "--ssg"])
            .assert()
            .success();
        let out = stdout(&a);
        assert!(
            out.contains("\nssg: wrote ") && out.ends_with("/abs/index.html\n"),
            "{out}"
        );
        let files = listed(&dir.join("dist"));
        assert_eq!(files.get("abs/index.html"), Some(&8), "{files:?}");
        sizes_match(&dir.join("dist"));
    }
}

/// The refusals (spec §7), each before anything is built: a file argument;
/// `--target`; a library; no `src/server.nova`, for each flag; and, outside
/// any project, no `nova.toml`.
#[test]
fn ssr_and_ssg_refuse_before_building() {
    let (dir, home) = site("refusals");
    let refused = |dir: &Path, args: &[&str], says: &str| {
        let a = nova_at(dir, &home).args(args).assert().failure();
        assert!(stderr(&a).contains(says), "{args:?}: {}", stderr(&a));
        assert!(!dir.join("dist").exists(), "{args:?}");
        assert!(!dir.join("target").exists(), "{args:?}");
    };
    refused(
        &dir,
        &["build", "--ssr", "src/main.nova"],
        "`--ssr` builds the project's src/main.nova and src/server.nova; it takes no file",
    );
    refused(
        &dir,
        &["build", "--ssg", "--target", "wasm"],
        "`--ssg` builds the client for wasm and the server natively; it takes no `--target`",
    );
    refused(
        &dir,
        &["build", "--ssr", "--target", "native"],
        "`--ssr` builds the client for wasm and the server natively; it takes no `--target`",
    );
    std::fs::remove_file(dir.join("src/server.nova")).expect("remove the server");
    refused(
        &dir,
        &["build", "--ssr"],
        "`--ssr` needs src/server.nova, the server's entry",
    );
    refused(
        &dir,
        &["build", "--ssr", "--ssg"],
        "`--ssg` needs src/server.nova, the server's entry",
    );
    std::fs::copy(
        repo_root().join("tests/ssr/site/src/server.nova"),
        dir.join("src/server.nova"),
    )
    .expect("put the server back");
    std::fs::rename(dir.join("src/main.nova"), dir.join("src/lib.nova")).expect("a library");
    refused(
        &dir,
        &["build", "--ssg"],
        "`site` is a library: it has no src/main.nova",
    );
    let loose = dir.with_file_name("loose");
    std::fs::create_dir_all(loose.join("src")).expect("a directory");
    std::fs::write(loose.join("src/server.nova"), "fn main() {}\n").expect("a server");
    refused(
        &loose,
        &["build", "--ssr"],
        "`--ssr` builds a project, and there is no nova.toml here or above",
    );
}

/// `--release` applies to both builds: the manifest says so, and the server
/// is `target/release/site-server`. Skipped where no `clang` is on PATH, as
/// every release build's test is.
#[test]
fn release_applies_to_the_client_and_the_server() {
    let clang = std::process::Command::new("clang")
        .arg("--version")
        .output()
        .is_ok();
    if !clang {
        eprintln!("skipping: no clang on PATH");
        return;
    }
    let (dir, home) = site("release");
    nova_at(&dir, &home)
        .args(["build", "--ssr", "--release"])
        .assert()
        .success();
    let manifest = std::fs::read_to_string(dir.join("dist/manifest.json")).expect("manifest");
    assert!(manifest.contains("\"release\": true"), "{manifest}");
    assert!(server(&dir, "release").is_file());
    assert!(!server(&dir, "debug").exists());
}
