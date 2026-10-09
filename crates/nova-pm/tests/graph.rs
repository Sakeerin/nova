//! The package graph (spec
//! `docs/superpowers/specs/2026-10-08-phase-3-3a-local-packages-design.md`
//! §3, §7.1).

use std::path::{Path, PathBuf};

use nova_diagnostics::{render, FileDb};
use nova_pm::{graph, graph_from, Graph, PackageId};

/// A fresh, empty directory under the system temp directory. Its name is
/// fixed, so each run replaces the last.
fn fresh(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("nova-pm-graph-{name}"));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("create the test directory");
    dir
}

/// A package called `name` at `dir`: its `nova.toml` with `extra` appended,
/// and an empty `src/<file>` for each of `files`.
fn package(dir: &Path, name: &str, files: &[&str], extra: &str) {
    std::fs::create_dir_all(dir.join("src")).unwrap();
    std::fs::write(
        dir.join("nova.toml"),
        format!("[package]\nname = \"{name}\"\nversion = \"0.1.0\"\nedition = \"2026\"\n{extra}"),
    )
    .unwrap();
    for file in files {
        std::fs::write(dir.join("src").join(file), "").unwrap();
    }
}

/// The graph of the package at `root`, its diagnostics' codes, and their
/// rendering.
fn build(root: &Path) -> (Option<Graph>, Vec<String>, String) {
    let mut db = FileDb::new();
    let (graph, diagnostics) = graph(root, &mut db);
    let codes = diagnostics.iter().map(|d| d.code.clone()).collect();
    (graph, codes, render::render_to_string(&db, &diagnostics))
}

fn names(graph: &Graph) -> Vec<&str> {
    graph.packages.iter().map(|p| p.name.as_str()).collect()
}

const LIB: &[&str] = &["lib.nova"];
const MAIN: &[&str] = &["main.nova"];

#[test]
fn a_path_dependency_resolves_and_a_diamond_is_one_package() {
    let dir = fresh("diamond");
    package(
        &dir.join("app"),
        "app",
        MAIN,
        "\n[dependencies]\na = { path = \"../a\" }\nb = { path = \"../b\" }\n",
    );
    package(
        &dir.join("a"),
        "a",
        LIB,
        "\n[dependencies]\nc = { path = \"../c\" }\n",
    );
    package(
        &dir.join("b"),
        "b",
        LIB,
        "\n[dependencies]\nc = { path = \"../c\" }\n",
    );
    package(&dir.join("c"), "c", LIB, "");
    let (graph, codes, _) = build(&dir.join("app"));
    assert!(codes.is_empty(), "{codes:?}");
    let graph = graph.unwrap();
    assert_eq!(names(&graph), ["app", "a", "b", "c"]);
    assert_eq!(
        graph.package(PackageId(1)).dependencies[0].package,
        PackageId(3)
    );
    assert_eq!(
        graph.package(PackageId(2)).dependencies[0].package,
        PackageId(3)
    );
    assert!(graph.root().has_main && !graph.root().has_lib);
    assert_eq!(
        graph.package(PackageId(1)).dir,
        dir.join("app").join("../a")
    );
}

#[test]
fn spellings_of_one_directory_are_one_package() {
    // Review Focus 1: a trailing slash, a `./`, and a detour through `..`.
    let dir = fresh("spellings");
    package(
        &dir.join("app"),
        "app",
        MAIN,
        "\n[dependencies]\na = { path = \"../a/\" }\nb = { path = \"./../b\" }\n",
    );
    package(
        &dir.join("a"),
        "a",
        LIB,
        "\n[dependencies]\nc = { path = \"../c\" }\n",
    );
    package(
        &dir.join("b"),
        "b",
        LIB,
        "\n[dependencies]\nc = { path = \"../b/../c/\" }\n",
    );
    package(&dir.join("c"), "c", LIB, "");
    let (graph, codes, rendered) = build(&dir.join("app"));
    assert!(codes.is_empty(), "{rendered}");
    assert_eq!(names(&graph.unwrap()), ["app", "a", "b", "c"]);
}

#[test]
fn a_version_only_entry_is_m0005_and_names_the_dependency() {
    let dir = fresh("m0005");
    package(&dir, "app", MAIN, "\n[dependencies]\nhttp = \"1.0\"\n");
    let (graph, codes, rendered) = build(&dir);
    assert_eq!(codes, ["M0005"]);
    // Spec 3.3b §5.4: a registry entry is found through nova.lock and the
    // cache, and with no lock it is not downloaded yet.
    assert!(
        rendered.contains("dependency `http` is not downloaded yet; run `nova fetch`"),
        "{rendered}"
    );
    assert_eq!(names(&graph.unwrap()), ["app"]);
}

#[test]
fn a_path_without_a_manifest_is_m0007_at_the_path() {
    let dir = fresh("m0007");
    package(
        &dir,
        "app",
        MAIN,
        "\n[dependencies]\ngeom = { path = \"../nowhere\" }\n",
    );
    let (_, codes, rendered) = build(&dir);
    assert_eq!(codes, ["M0007"]);
    assert!(
        rendered.contains("is not a directory holding a nova.toml"),
        "{rendered}"
    );
    assert!(rendered.contains("nova.toml:7:17"), "{rendered}");
}

#[test]
fn a_misnamed_dependency_is_m0008() {
    let dir = fresh("m0008");
    package(
        &dir.join("app"),
        "app",
        MAIN,
        "\n[dependencies]\ngeom = { path = \"../geom\" }\n",
    );
    package(&dir.join("geom"), "geometry", LIB, "");
    let (_, codes, rendered) = build(&dir.join("app"));
    assert_eq!(codes, ["M0008"]);
    assert!(
        rendered.contains("dependency `geom` is the package `geometry`"),
        "{rendered}"
    );
}

#[test]
fn a_dependency_without_a_library_is_m0009() {
    let dir = fresh("m0009");
    package(
        &dir.join("app"),
        "app",
        MAIN,
        "\n[dependencies]\ngeom = { path = \"../geom\" }\n",
    );
    package(&dir.join("geom"), "geom", MAIN, "");
    let (_, codes, rendered) = build(&dir.join("app"));
    assert_eq!(codes, ["M0009"]);
    assert!(rendered.contains("is not a library"), "{rendered}");
}

#[test]
fn a_cycle_is_m0010_and_lists_its_packages() {
    let dir = fresh("m0010");
    package(
        &dir.join("app"),
        "app",
        LIB,
        "\n[dependencies]\na = { path = \"../a\" }\n",
    );
    package(
        &dir.join("a"),
        "a",
        LIB,
        "\n[dependencies]\napp = { path = \"../app\" }\n",
    );
    let (_, codes, rendered) = build(&dir.join("app"));
    assert_eq!(codes, ["M0010"]);
    assert!(
        rendered.contains("dependency cycle: app -> a -> app"),
        "{rendered}"
    );
}

#[test]
fn a_path_to_the_package_itself_is_m0010() {
    let dir = fresh("m0010-self");
    package(
        &dir,
        "app",
        MAIN,
        "\n[dependencies]\nme = { path = \".\" }\n",
    );
    let (_, codes, rendered) = build(&dir);
    assert_eq!(codes, ["M0010"]);
    assert!(rendered.contains("`app` depends on itself"), "{rendered}");
}

#[test]
fn two_packages_with_one_name_are_m0011() {
    let dir = fresh("m0011");
    package(
        &dir.join("app"),
        "app",
        MAIN,
        "\n[dependencies]\na = { path = \"../a\" }\nb = { path = \"../b\" }\n",
    );
    package(
        &dir.join("a"),
        "a",
        LIB,
        "\n[dependencies]\nc = { path = \"../c1\" }\n",
    );
    package(
        &dir.join("b"),
        "b",
        LIB,
        "\n[dependencies]\nc = { path = \"../c2\" }\n",
    );
    package(&dir.join("c1"), "c", LIB, "");
    package(&dir.join("c2"), "c", LIB, "");
    let (_, codes, rendered) = build(&dir.join("app"));
    assert_eq!(codes, ["M0011"]);
    assert!(
        rendered.contains("two packages named `c` in one build"),
        "{rendered}"
    );
    assert!(
        rendered.contains("c1") && rendered.contains("c2"),
        "{rendered}"
    );
}

#[test]
fn import_name_clashes_and_keywords_are_m0012() {
    let dir = fresh("m0012");
    for (name, package_name) in [
        ("geom", "geom"),
        ("json-api", "json-api"),
        ("json_api", "json_api"),
    ] {
        package(&dir.join(name), package_name, LIB, "");
    }
    let cases = [
        ("keyword", "\n[dependencies]\nmatch = { path = \"../geom\" }\n", "which is a keyword"),
        (
            "both-tables",
            "\n[dependencies]\ngeom = { path = \"../geom\" }\n\n[dev-dependencies]\ngeom = { path = \"../geom\" }\n",
            "`geom` is in both [dependencies] and [dev-dependencies]",
        ),
        (
            "one-import-name",
            "\n[dependencies]\njson-api = { path = \"../json-api\" }\njson_api = { path = \"../json_api\" }\n",
            "`json-api` and `json_api` are both imported as `json_api`",
        ),
        ("own-name", "\n[dependencies]\napp = { path = \"../geom\" }\n", "this package's own name"),
    ];
    for (case, extra, message) in cases {
        let root = dir.join(format!("app-{case}"));
        package(&root, "app", MAIN, extra);
        let (_, codes, rendered) = build(&root);
        assert_eq!(codes, ["M0012"], "{case}: {rendered}");
        assert!(rendered.contains(message), "{case}: {rendered}");
    }
}

#[test]
fn a_library_named_like_a_keyword_is_m0012() {
    let dir = fresh("m0012-root");
    package(&dir, "match", LIB, "");
    let (_, codes, rendered) = build(&dir);
    assert_eq!(codes, ["M0012"]);
    assert!(
        rendered.contains("library `match` would be imported as `match`"),
        "{rendered}"
    );
}

#[test]
fn a_package_with_neither_target_is_m0013_at_its_package_table() {
    let dir = fresh("m0013");
    package(&dir, "app", &[], "");
    let (_, codes, rendered) = build(&dir);
    assert_eq!(codes, ["M0013"]);
    assert!(
        rendered.contains("has neither src/lib.nova nor src/main.nova"),
        "{rendered}"
    );
    assert!(rendered.contains("nova.toml:1:1"), "{rendered}");
}

#[test]
fn a_dependencys_dev_dependencies_are_not_read() {
    let dir = fresh("dev-not-read");
    package(
        &dir.join("app"),
        "app",
        MAIN,
        "\n[dependencies]\na = { path = \"../a\" }\n",
    );
    package(
        &dir.join("a"),
        "a",
        LIB,
        "\n[dev-dependencies]\nx = { path = \"../nowhere\" }\n",
    );
    let (graph, codes, rendered) = build(&dir.join("app"));
    assert!(codes.is_empty(), "{rendered}");
    assert!(graph
        .unwrap()
        .package(PackageId(1))
        .dev_dependencies
        .is_empty());
}

#[test]
fn a_dev_dependency_that_depends_back_on_the_root_is_no_cycle() {
    let dir = fresh("dev-back");
    package(
        &dir.join("app"),
        "app",
        LIB,
        "\n[dev-dependencies]\nhelper = { path = \"../helper\" }\n",
    );
    package(
        &dir.join("helper"),
        "helper",
        LIB,
        "\n[dependencies]\napp = { path = \"../app\" }\n",
    );
    let (graph, codes, rendered) = build(&dir.join("app"));
    assert!(codes.is_empty(), "{rendered}");
    let graph = graph.unwrap();
    assert_eq!(names(&graph), ["app", "helper"]);
    assert_eq!(
        graph.package(PackageId(1)).dependencies[0].package,
        PackageId(0)
    );
}

#[test]
fn a_dependency_back_on_a_root_without_a_library_is_m0009() {
    let dir = fresh("dev-back-no-lib");
    package(
        &dir.join("app"),
        "app",
        MAIN,
        "\n[dev-dependencies]\nhelper = { path = \"../helper\" }\n",
    );
    package(
        &dir.join("helper"),
        "helper",
        LIB,
        "\n[dependencies]\napp = { path = \"../app\" }\n",
    );
    let (_, codes, rendered) = build(&dir.join("app"));
    assert_eq!(codes, ["M0009"], "{rendered}");
    assert!(
        rendered.contains("dependency `app` is not a library"),
        "{rendered}"
    );
}

#[test]
fn paths_are_relative_to_each_manifest() {
    let dir = fresh("relative");
    package(
        &dir.join("apps").join("app"),
        "app",
        MAIN,
        "\n[dependencies]\na = { path = \"../../libs/a\" }\n",
    );
    package(
        &dir.join("libs").join("a"),
        "a",
        LIB,
        "\n[dependencies]\nb = { path = \"../b\" }\n",
    );
    package(&dir.join("libs").join("b"), "b", LIB, "");
    let (graph, codes, rendered) = build(&dir.join("apps").join("app"));
    assert!(codes.is_empty(), "{rendered}");
    assert_eq!(names(&graph.unwrap()), ["app", "a", "b"]);
}

#[test]
fn reached_through_names_the_roots_own_entry() {
    let dir = fresh("reached");
    package(
        &dir.join("app"),
        "app",
        MAIN,
        "\n[dependencies]\na = { path = \"../a\" }\n",
    );
    package(
        &dir.join("a"),
        "a",
        LIB,
        "\n[dependencies]\nc = { path = \"../c\" }\n",
    );
    package(&dir.join("c"), "c", LIB, "");
    let (graph, codes, _) = build(&dir.join("app"));
    assert!(codes.is_empty(), "{codes:?}");
    let graph = graph.unwrap();
    let entry = graph.root().dependencies[0].span;
    assert_eq!(graph.reached_through(PackageId(1)), Some(entry));
    assert_eq!(graph.reached_through(PackageId(2)), Some(entry));
    assert_eq!(graph.reached_through(PackageId(0)), None);
}

#[test]
fn graph_from_reads_the_roots_manifest_from_the_text() {
    let dir = fresh("from-text");
    package(&dir.join("app"), "app", MAIN, "");
    package(&dir.join("a"), "a", LIB, "");
    let text = "[package]\nname = \"app\"\nversion = \"0.1.0\"\nedition = \"2026\"\n\n\
                [dependencies]\na = { path = \"../a\" }\n";
    let mut db = FileDb::new();
    let (graph, diagnostics) = graph_from(&dir.join("app"), Some(text), &mut db);
    assert!(diagnostics.is_empty(), "{diagnostics:?}");
    assert_eq!(names(&graph.unwrap()), ["app", "a"]);
}

#[test]
fn every_manifests_warnings_are_kept() {
    let dir = fresh("warnings");
    package(
        &dir.join("app"),
        "app",
        MAIN,
        "\n[dependencies]\na = { path = \"../a\" }\n",
    );
    package(&dir.join("a"), "a", LIB, "\n[features]\ndefault = []\n");
    let (graph, codes, rendered) = build(&dir.join("app"));
    assert_eq!(codes, ["M0006"]);
    assert!(rendered.contains("`features`"), "{rendered}");
    assert_eq!(names(&graph.unwrap()), ["app", "a"]);
}
