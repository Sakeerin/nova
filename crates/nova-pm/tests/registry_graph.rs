//! The offline graph's registry packages (spec
//! `docs/superpowers/specs/2026-10-09-phase-3-3b-index-and-publishing-design.md`
//! §5.4, §10.2).

use std::path::{Path, PathBuf};

use nova_diagnostics::{render, FileDb};
use nova_pm::{graph_with, index_dir_name, requirements, Graph, Lock, LockedPackage, Offline};
use semver::Version;

const INDEX: &str = "https://example.test/index/";

/// A fresh, empty directory with a fixed name, so each run replaces the last.
fn fresh(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("nova-pm-registry-{name}"));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("create the test directory");
    dir
}

/// A package called `name` at `dir`: its `nova.toml` with `extra`
/// appended, and an empty `src/<file>` for each of `files`.
fn package(dir: &Path, name: &str, version: &str, files: &[&str], extra: &str) {
    std::fs::create_dir_all(dir.join("src")).unwrap();
    std::fs::write(
        dir.join("nova.toml"),
        format!(
            "[package]\nname = \"{name}\"\nversion = \"{version}\"\nedition = \"2026\"\n{extra}"
        ),
    )
    .unwrap();
    for file in files {
        std::fs::write(dir.join("src").join(file), "").unwrap();
    }
}

/// The library `name` `version`, unpacked in `registry` for [`INDEX`].
fn cached(registry: &Path, name: &str, version: &str, extra: &str) -> PathBuf {
    let dir = registry
        .join("src")
        .join(index_dir_name(INDEX))
        .join(format!("{name}-{version}"));
    package(&dir, name, version, &["lib.nova"], extra);
    dir
}

/// A lock on [`INDEX`] holding each `(name, version, dependencies)`.
fn lock(packages: &[(&str, &str, &[&str])]) -> Lock {
    Lock {
        index: INDEX.into(),
        packages: packages
            .iter()
            .map(|(name, version, deps)| LockedPackage {
                name: name.to_string(),
                version: Version::parse(version).unwrap(),
                checksum: "0".repeat(64),
                dependencies: deps.iter().map(|d| d.to_string()).collect(),
            })
            .collect(),
    }
}

fn write_lock(root: &Path, lock: &Lock) {
    std::fs::write(root.join("nova.lock"), lock.to_text()).unwrap();
}

fn offline(registry: &Path) -> Offline {
    Offline {
        registry: Some(registry.to_path_buf()),
        lock: None,
        dev: true,
    }
}

/// The graph of the package at `root`, its diagnostics' codes, and their
/// rendering.
fn build(root: &Path, offline: &Offline) -> (Option<Graph>, Vec<String>, String) {
    let mut db = FileDb::new();
    let (graph, diagnostics) = graph_with(root, None, offline, &mut db);
    let codes = diagnostics.iter().map(|d| d.code.clone()).collect();
    (graph, codes, render::render_to_string(&db, &diagnostics))
}

fn names(graph: &Graph) -> Vec<&str> {
    graph.packages.iter().map(|p| p.name.as_str()).collect()
}

const JSON_ENTRY: &str = "\n[dependencies]\njson = \"1\"\n";

#[test]
fn a_registry_entry_is_found_through_the_lock_and_the_cache() {
    let dir = fresh("found");
    let app = dir.join("app");
    package(&app, "app", "0.1.0", &["main.nova"], JSON_ENTRY);
    write_lock(&app, &lock(&[("json", "1.4.1", &[])]));
    let registry = dir.join("registry");
    let unpacked = cached(&registry, "json", "1.4.1", "");
    let (graph, codes, rendered) = build(&app, &offline(&registry));
    assert!(codes.is_empty(), "{rendered}");
    let graph = graph.unwrap();
    assert_eq!(names(&graph), ["app", "json"]);
    assert!(!graph.root().registry);
    assert!(graph.packages[1].registry);
    assert_eq!(graph.packages[1].canonical, nova_pm::real_path(&unpacked));
    assert_eq!(graph.root().dependencies[0].import_name, "json");
}

#[test]
fn a_registry_packages_own_dependencies_are_found_the_same_way() {
    let dir = fresh("transitive");
    let app = dir.join("app");
    package(
        &app,
        "app",
        "0.1.0",
        &["main.nova"],
        "\n[dependencies]\ngeom = \"0.2\"\n",
    );
    write_lock(
        &app,
        &lock(&[("geom", "0.2.0", &["json"]), ("json", "1.0.0", &[])]),
    );
    let registry = dir.join("registry");
    cached(&registry, "geom", "0.2.0", JSON_ENTRY);
    cached(&registry, "json", "1.0.0", "");
    let (graph, codes, rendered) = build(&app, &offline(&registry));
    assert!(codes.is_empty(), "{rendered}");
    let graph = graph.unwrap();
    assert_eq!(names(&graph), ["app", "geom", "json"]);
    assert!(graph.packages[1].registry && graph.packages[2].registry);
}

#[test]
fn an_entry_with_no_locked_package_is_m0005() {
    let dir = fresh("unlocked");
    let app = dir.join("app");
    package(&app, "app", "0.1.0", &["main.nova"], JSON_ENTRY);
    let registry = dir.join("registry");
    // No nova.lock at all, then a lock without `json`.
    for lock in [None, Some(lock(&[("http", "1.0.0", &[])]))] {
        if let Some(lock) = &lock {
            write_lock(&app, lock);
        }
        let (graph, codes, rendered) = build(&app, &offline(&registry));
        assert_eq!(codes, ["M0005"], "{rendered}");
        assert!(
            rendered.contains("dependency `json` is not downloaded yet; run `nova fetch`"),
            "{rendered}"
        );
        assert_eq!(names(&graph.unwrap()), ["app"]);
    }
}

#[test]
fn a_locked_version_that_no_longer_fits_is_m0005() {
    let dir = fresh("no-fit");
    let app = dir.join("app");
    package(
        &app,
        "app",
        "0.1.0",
        &["main.nova"],
        "\n[dependencies]\njson = \"2\"\n",
    );
    write_lock(&app, &lock(&[("json", "1.4.1", &[])]));
    let registry = dir.join("registry");
    cached(&registry, "json", "1.4.1", "");
    let (_, codes, rendered) = build(&app, &offline(&registry));
    assert_eq!(codes, ["M0005"], "{rendered}");
    assert!(
        rendered.contains(
            "dependency `json` is locked at 1.4.1, which does not meet `^2`; run `nova fetch`"
        ),
        "{rendered}"
    );
}

#[test]
fn a_locked_package_not_unpacked_is_m0005() {
    let dir = fresh("not-unpacked");
    let app = dir.join("app");
    package(&app, "app", "0.1.0", &["main.nova"], JSON_ENTRY);
    write_lock(&app, &lock(&[("json", "1.4.1", &[])]));
    let (_, codes, rendered) = build(&app, &offline(&dir.join("registry")));
    assert_eq!(codes, ["M0005"], "{rendered}");
    assert!(rendered.contains("is not downloaded yet"), "{rendered}");
}

#[test]
fn an_unreadable_lock_is_m0016_and_each_entry_m0005() {
    let dir = fresh("unreadable");
    let app = dir.join("app");
    package(
        &app,
        "app",
        "0.1.0",
        &["main.nova"],
        "\n[dependencies]\njson = \"1\"\nhttp = \"1\"\n",
    );
    std::fs::write(app.join("nova.lock"), "version = 2\nindex = \"x\"\n").unwrap();
    let (_, mut codes, rendered) = build(&app, &offline(&dir.join("registry")));
    codes.sort();
    assert_eq!(codes, ["M0005", "M0005", "M0016"], "{rendered}");
    assert!(rendered.contains("nova.lock cannot be read"), "{rendered}");
}

#[test]
fn no_nova_home_is_m0005_saying_to_set_it() {
    let dir = fresh("no-home");
    let app = dir.join("app");
    package(&app, "app", "0.1.0", &["main.nova"], JSON_ENTRY);
    write_lock(&app, &lock(&[("json", "1.4.1", &[])]));
    let offline = Offline {
        registry: None,
        lock: None,
        dev: true,
    };
    let (_, codes, rendered) = build(&app, &offline);
    assert_eq!(codes, ["M0005"], "{rendered}");
    assert!(
        rendered.contains("set NOVA_HOME to a writable directory"),
        "{rendered}"
    );
}

#[test]
fn a_downloaded_package_with_a_path_entry_is_m0017() {
    let dir = fresh("cached-path");
    let app = dir.join("app");
    package(&app, "app", "0.1.0", &["main.nova"], JSON_ENTRY);
    write_lock(&app, &lock(&[("json", "1.4.1", &["x"])]));
    let registry = dir.join("registry");
    cached(
        &registry,
        "json",
        "1.4.1",
        "\n[dependencies]\nx = { path = \"../x\" }\n",
    );
    let (_, codes, rendered) = build(&app, &offline(&registry));
    assert_eq!(codes, ["M0017"], "{rendered}");
    assert!(
        rendered.contains("downloaded package `json` 1.4.1 has a path dependency `x`"),
        "{rendered}"
    );
}

#[test]
fn a_downloaded_package_unlike_its_lock_entry_is_m0005() {
    let dir = fresh("cached-unlike");
    let app = dir.join("app");
    package(&app, "app", "0.1.0", &["main.nova"], JSON_ENTRY);
    write_lock(&app, &lock(&[("json", "1.4.1", &[])]));
    let registry = dir.join("registry");
    cached(
        &registry,
        "json",
        "1.4.1",
        "\n[dependencies]\nhttp = \"1\"\n",
    );
    let (_, codes, rendered) = build(&app, &offline(&registry));
    assert_eq!(codes, ["M0005"], "{rendered}");
    assert!(
        rendered.contains("the downloaded copy of `json` 1.4.1 does not match nova.lock"),
        "{rendered}"
    );
}

#[test]
fn a_downloaded_manifests_warning_is_dropped_and_its_error_kept() {
    let dir = fresh("cached-warning");
    let app = dir.join("app");
    package(
        &app,
        "app",
        "0.1.0",
        &["main.nova"],
        "\n[dependencies]\njson = \"1\"\nhttp = \"1\"\n",
    );
    write_lock(
        &app,
        &lock(&[("http", "1.0.0", &[]), ("json", "1.4.1", &[])]),
    );
    let registry = dir.join("registry");
    // An unknown key is M0006, a warning; `bad = 5` is M0003, an error.
    cached(&registry, "json", "1.4.1", "colour = \"red\"\n");
    cached(&registry, "http", "1.0.0", "\n[dependencies]\nbad = 5\n");
    let (_, codes, rendered) = build(&app, &offline(&registry));
    assert_eq!(codes, ["M0003"], "{rendered}");
}

#[test]
fn the_lock_given_in_offline_is_used_instead_of_the_file() {
    let dir = fresh("given-lock");
    let app = dir.join("app");
    package(&app, "app", "0.1.0", &["main.nova"], JSON_ENTRY);
    let registry = dir.join("registry");
    cached(&registry, "json", "1.4.1", "");
    let offline = Offline {
        lock: Some(lock(&[("json", "1.4.1", &[])])),
        ..offline(&registry)
    };
    let (graph, codes, rendered) = build(&app, &offline);
    assert!(codes.is_empty(), "{rendered}");
    assert_eq!(names(&graph.unwrap()), ["app", "json"]);
}

#[test]
fn without_dev_the_roots_dev_dependencies_are_not_read() {
    let dir = fresh("no-dev");
    let app = dir.join("app");
    package(
        &app,
        "app",
        "0.1.0",
        &["lib.nova"],
        "\n[dev-dependencies]\njson = \"1\"\n",
    );
    let registry = dir.join("registry");
    let (_, codes, _) = build(&app, &offline(&registry));
    assert_eq!(codes, ["M0005"]);
    let offline = Offline {
        dev: false,
        ..offline(&registry)
    };
    let (graph, codes, rendered) = build(&app, &offline);
    assert!(codes.is_empty(), "{rendered}");
    assert_eq!(names(&graph.unwrap()), ["app"]);
}

#[test]
fn requirements_name_who_made_each_and_the_roots_entry() {
    let dir = fresh("requirements");
    let app = dir.join("app");
    // No source file: `nova add` syncs a package before it has one, so
    // this is not M0013 here.
    package(
        &app,
        "app",
        "0.1.0",
        &[],
        "\n[dependencies]\njson = \"1\"\nutil = { path = \"../util\" }\n\n\
         [dev-dependencies]\nkit = \"0.3\"\n",
    );
    package(
        &dir.join("util"),
        "util",
        "0.1.0",
        &["lib.nova"],
        "\n[dependencies]\nhttp = \"^0.2\"\n",
    );
    let mut db = FileDb::new();
    let (found, diagnostics) = requirements(&app, None, true, &mut db);
    assert!(
        diagnostics.is_empty(),
        "{}",
        render::render_to_string(&db, &diagnostics)
    );
    let text = |span: nova_diagnostics::Span| -> String {
        db.get_source(span.file).unwrap()[span.as_range()].to_string()
    };
    let listed: Vec<String> = found
        .iter()
        .map(|r| format!("{} {} {} {}", r.name, r.req, r.by, text(r.span)))
        .collect();
    assert_eq!(
        listed,
        [
            "json ^1 app json",
            "kit ^0.3 app kit",
            "http ^0.2 util util"
        ]
    );
    let mut db = FileDb::new();
    let (found, _) = requirements(&app, None, false, &mut db);
    let names: Vec<&str> = found.iter().map(|r| r.name.as_str()).collect();
    assert_eq!(names, ["json", "http"]);
}
