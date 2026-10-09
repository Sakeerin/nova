//! `nova.lock` (spec
//! `docs/superpowers/specs/2026-10-09-phase-3-3b-index-and-publishing-design.md`
//! §4.6).

use nova_diagnostics::{render, FileDb};
use nova_pm::{parse_lock, Lock, LockedPackage};
use semver::Version;

fn lock() -> Lock {
    Lock {
        index: "https://example.test/index/".into(),
        packages: vec![
            LockedPackage {
                name: "geom".into(),
                version: Version::new(0, 2, 0),
                checksum: "a".repeat(64),
                dependencies: vec!["json-api".into()],
            },
            LockedPackage {
                name: "json-api".into(),
                version: Version::parse("1.4.1").unwrap(),
                checksum: "b".repeat(64),
                dependencies: vec![],
            },
        ],
    }
}

#[test]
fn a_lock_round_trips_through_its_text() {
    let text = lock().to_text();
    assert!(
        text.starts_with(
            "# Written by nova. Commit it for a program.\nversion = 1\n\
             index = \"https://example.test/index/\"\n"
        ),
        "{text}"
    );
    assert!(
        text.contains("\n[[package]]\nname = \"geom\"\nversion = \"0.2.0\"\n"),
        "{text}"
    );
    assert!(text.contains("dependencies = [\"json-api\"]\n"), "{text}");
    assert!(text.contains("dependencies = []\n"), "{text}");
    assert!(!text.contains('\r'));
    let mut db = FileDb::new();
    let file = db.add("nova.lock", text.as_str());
    assert_eq!(parse_lock(&text, file).unwrap(), lock());
}

#[test]
fn packages_are_found_by_name() {
    let lock = lock();
    assert_eq!(lock.find("geom").unwrap().version, Version::new(0, 2, 0));
    assert!(lock.find("http").is_none());
}

#[test]
fn packages_come_back_sorted_by_name() {
    let text = "version = 1\nindex = \"x\"\n\n[[package]]\nname = \"zeta\"\nversion = \"1.0.0\"\n\
                checksum = \"c\"\n\n[[package]]\nname = \"alpha\"\nversion = \"1.0.0\"\nchecksum = \"c\"\n";
    let mut db = FileDb::new();
    let file = db.add("nova.lock", text);
    let lock = parse_lock(text, file).unwrap();
    let names: Vec<&str> = lock.packages.iter().map(|p| p.name.as_str()).collect();
    assert_eq!(names, ["alpha", "zeta"]);
}

#[test]
fn an_unreadable_lock_is_m0016() {
    for (text, says) in [
        ("version = 2\nindex = \"x\"\n", "its `version` is not 1"),
        ("version = 1\n", "it has no `index`"),
        (
            "version = 1\nindex = \"x\"\n[[package]]\nname = \"geom\"\n",
            "lacks its name, version or checksum",
        ),
        (
            "version = 1\nindex = \"x\"\n[[package]]\nname = \"geom\"\nversion = \"one\"\nchecksum = \"c\"\n",
            "version `one`",
        ),
        ("this is not toml = [", "nova.lock cannot be read"),
    ] {
        let mut db = FileDb::new();
        let file = db.add("nova.lock", text);
        let diagnostic = parse_lock(text, file).unwrap_err();
        assert_eq!(diagnostic.code, "M0016", "{text}");
        let rendered = render::render_to_string(&db, &[diagnostic]);
        assert!(rendered.contains(says), "{text}: {rendered}");
        assert!(rendered.contains("run `nova update`"), "{rendered}");
    }
}
