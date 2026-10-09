//! HTTP and downloads, against loopback servers (spec
//! `docs/superpowers/specs/2026-10-09-phase-3-3b-index-and-publishing-design.md`
//! §3.3, §5.2, §10.4).

mod support;

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use nova_index::{fetch_package, pack, sha256_hex, Config, Http, HttpReader, Index, Reader, View};
use nova_pm::{IndexView, LockedPackage};
use semver::Version;
use support::{Response, Server};

/// A fresh, empty directory with a fixed name, so each run replaces the last.
fn fresh(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("nova-index-http-{name}"));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn write(dir: &Path, path: &str, bytes: &[u8]) {
    let path = dir.join(path);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, bytes).unwrap();
}

/// Every file under `dir`; none when it does not exist.
fn files_under(dir: &Path) -> Vec<PathBuf> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut files = Vec::new();
    for entry in entries {
        let path = entry.unwrap().path();
        if path.is_dir() {
            files.extend(files_under(&path));
        } else {
            files.push(path);
        }
    }
    files
}

/// `geom` 0.1.0, packed from a source under `dir`.
fn geom_tarball(dir: &Path) -> Vec<u8> {
    let source = dir.join("source");
    write(
        &source,
        "nova.toml",
        b"[package]\nname = \"geom\"\nversion = \"0.1.0\"\nedition = \"2026\"\n",
    );
    write(
        &source,
        "src/lib.nova",
        b"pub fn area() -> Int {\n    9\n}\n",
    );
    pack(&source, "geom", &Version::new(0, 1, 0)).unwrap().bytes
}

fn locked(checksum: &str) -> LockedPackage {
    LockedPackage {
        name: "geom".into(),
        version: Version::new(0, 1, 0),
        checksum: checksum.into(),
        dependencies: Vec::new(),
    }
}

fn config() -> Config {
    Config {
        dl: "dl/{name}-{version}.nova-pkg".into(),
        api: None,
    }
}

#[test]
fn a_file_is_fetched_over_loopback_http() {
    let server = Server::start(|request| match request.path.as_str() {
        "/a" => Response::ok("hello"),
        "/broken" => Response::status(500),
        _ => Response::status(404),
    });
    let http = Http::new();
    let url = |path: &str| format!("{}{path}", server.url);
    assert_eq!(http.get(&url("/a"), 100).unwrap(), Some(b"hello".to_vec()));
    assert_eq!(http.get(&url("/none"), 100).unwrap(), None);
    let error = http.get(&url("/broken"), 100).unwrap_err();
    assert!(error.contains("500"), "{error}");
}

#[test]
fn redirects_are_followed_only_where_http_is_allowed() {
    let server = Server::start(|request| match request.path.as_str() {
        "/moved" => Response::redirect("/a"),
        "/away" => Response::redirect("http://example.com/x"),
        "/loop" => Response::redirect("/loop"),
        "/a" => Response::ok("here"),
        _ => Response::status(404),
    });
    let http = Http::new();
    let url = |path: &str| format!("{}{path}", server.url);
    assert_eq!(
        http.get(&url("/moved"), 100).unwrap(),
        Some(b"here".to_vec())
    );
    let error = http.get(&url("/away"), 100).unwrap_err();
    assert!(error.contains("refused http://example.com/x"), "{error}");
    let error = http.get(&url("/loop"), 100).unwrap_err();
    assert!(error.contains("too many redirects"), "{error}");
    let loops = server
        .requests()
        .iter()
        .filter(|r| r.path == "/loop")
        .count();
    assert_eq!(loops, 6);
    // Refused before any connection is made.
    assert!(http.get("http://example.com/", 100).is_err());
}

#[test]
fn a_body_over_its_limit_is_refused() {
    let server = Server::start(|_| Response::ok(vec![b'x'; 100]));
    assert!(Http::new().get(&format!("{}/big", server.url), 10).is_err());
}

#[test]
fn requests_name_nova_and_carry_no_credentials() {
    let server = Server::start(|_| Response::ok("x"));
    Http::new().get(&format!("{}/a", server.url), 10).unwrap();
    let request = &server.requests()[0];
    let agent = request.header("user-agent").unwrap();
    assert!(agent.starts_with("nova/"), "{request:?}");
    assert!(request.header("authorization").is_none(), "{request:?}");
}

#[test]
fn an_http_index_is_read_through_a_view() {
    let line = r#"{"name":"geom","vers":"0.1.0","deps":[],"cksum":"00","v":1}"#;
    let server = Server::start(move |request| match request.path.as_str() {
        "/index/ge/om/geom" => Response::ok(format!("{line}\n")),
        "/index/config.json" => Response::ok(r#"{"dl":"dl/{name}-{version}.nova-pkg"}"#),
        _ => Response::status(404),
    });
    let mut reader = HttpReader::new(&format!("{}/index/", server.url));
    assert_eq!(reader.config().unwrap().dl, "dl/{name}-{version}.nova-pkg");
    let mut view = View::new(&mut reader);
    assert_eq!(view.versions("geom").unwrap().unwrap().len(), 1);
    assert_eq!(view.versions("json").unwrap(), None);
}

#[test]
fn a_tarball_is_downloaded_checked_and_unpacked() {
    let dir = fresh("download");
    let tarball = geom_tarball(&dir);
    let checksum = sha256_hex(&tarball);
    let served = tarball.clone();
    let server = Server::start(move |request| match request.path.as_str() {
        "/dl/geom-0.1.0.nova-pkg" => Response::ok(served.clone()),
        _ => Response::status(404),
    });
    let index = Index::new(&server.url).unwrap();
    let registry = dir.join("registry");
    let dest = fetch_package(
        &index,
        &config(),
        &Http::new(),
        &locked(&checksum),
        &registry,
    )
    .unwrap();
    assert!(dest.join("src/lib.nova").is_file());
    let idx = nova_pm::index_dir_name(&index.canonical);
    assert!(registry
        .join("cache")
        .join(&idx)
        .join("geom-0.1.0.nova-pkg")
        .is_file());
    // Once unpacked, nothing is requested again.
    let before = server.requests().len();
    fetch_package(
        &index,
        &config(),
        &Http::new(),
        &locked(&checksum),
        &registry,
    )
    .unwrap();
    assert_eq!(server.requests().len(), before);
}

#[test]
fn a_tarball_whose_checksum_is_wrong_is_refused_and_discarded() {
    let dir = fresh("checksum");
    let tarball = geom_tarball(&dir);
    let actual = sha256_hex(&tarball);
    let server = Server::start(move |_| Response::ok(tarball.clone()));
    let index = Index::new(&server.url).unwrap();
    let registry = dir.join("registry");
    let wrong = "0".repeat(64);
    let error =
        fetch_package(&index, &config(), &Http::new(), &locked(&wrong), &registry).unwrap_err();
    assert!(error.contains(&actual) && error.contains(&wrong), "{error}");
    assert!(
        files_under(&registry).is_empty(),
        "{:?}",
        files_under(&registry)
    );
}

#[test]
fn a_cut_off_download_leaves_nothing_and_the_next_one_works() {
    // Review Focus 5.
    let dir = fresh("cut-off");
    let tarball = geom_tarball(&dir);
    let checksum = sha256_hex(&tarball);
    let calls = Arc::new(AtomicUsize::new(0));
    let counted = Arc::clone(&calls);
    let server = Server::start(move |_| {
        let mut response = Response::ok(tarball.clone());
        if counted.fetch_add(1, Ordering::SeqCst) == 0 {
            response.cut_after = Some(tarball.len() / 2);
        }
        response
    });
    let index = Index::new(&server.url).unwrap();
    let registry = dir.join("registry");
    let first = fetch_package(
        &index,
        &config(),
        &Http::new(),
        &locked(&checksum),
        &registry,
    );
    assert!(first.is_err(), "{first:?}");
    assert!(
        files_under(&registry).is_empty(),
        "{:?}",
        files_under(&registry)
    );
    let dest = fetch_package(
        &index,
        &config(),
        &Http::new(),
        &locked(&checksum),
        &registry,
    )
    .unwrap();
    assert!(dest.join("src/lib.nova").is_file());
    assert_eq!(calls.load(Ordering::SeqCst), 2);
}

#[test]
fn a_local_indexs_tarball_is_read_from_its_directory() {
    let dir = fresh("local");
    let tarball = geom_tarball(&dir);
    let checksum = sha256_hex(&tarball);
    let index_dir = dir.join("index");
    write(&index_dir, "dl/geom-0.1.0.nova-pkg", &tarball);
    let index = Index::new(index_dir.to_str().unwrap()).unwrap();
    let dest = fetch_package(
        &index,
        &config(),
        &Http::new(),
        &locked(&checksum),
        &dir.join("registry"),
    )
    .unwrap();
    assert!(dest.join("nova.toml").is_file());
}
