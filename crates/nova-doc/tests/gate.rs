//! The phase plan's 3.5 gate, for the site (spec
//! `docs/superpowers/specs/2026-10-11-phase-3-5a-the-documentation-site-design.md`
//! §16.4, §17): for std, and for an example that uses a dependency, every
//! public item has an anchor and a link check finds no broken link.
//!
//! Both are guards, written after the code on purpose: a missing anchor or
//! a broken link is a bug in the model, to be fixed there, not here.

mod common;

use std::path::{Path, PathBuf};

use common::Source;

/// A fresh, empty directory with a fixed name, so each run replaces the
/// last.
fn fresh(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("nova-doc-gate-{name}"));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn repo() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// `from` copied to `to`, without any `target/`.
fn copy(from: &Path, to: &Path) {
    std::fs::create_dir_all(to).unwrap();
    for entry in std::fs::read_dir(from).unwrap() {
        let entry = entry.unwrap();
        let name = entry.file_name();
        if name == "target" {
            continue;
        }
        let path = entry.path();
        if path.is_dir() {
            copy(&path, &to.join(&name));
        } else {
            std::fs::copy(&path, to.join(&name)).unwrap();
        }
    }
}

#[test]
fn std_has_every_anchor_and_no_broken_link() {
    let dir = fresh("std").join("doc");
    let Ok(documented) = nova_doc::document_std() else {
        panic!("std is not documented")
    };
    nova_doc::write_site(&documented.site, &dir).expect("the site is written");
    let missing = common::missing(&dir, &common::anchors(&common::std_sources()));
    assert!(
        missing.is_empty(),
        "{} missing:\n{}",
        missing.len(),
        missing.join("\n")
    );
    assert_eq!(nova_doc::check_links(&dir), Vec::new());
    for (name, _) in nova_std::ALL {
        assert!(
            dir.join("std").join(name).join("index.html").is_file(),
            "std/{name}"
        );
    }
    for p in common::PRIMITIVES {
        assert!(
            dir.join("std")
                .join(format!("primitive.{p}.html"))
                .is_file(),
            "{p}"
        );
    }
}

#[test]
fn the_example_has_every_anchor_and_no_broken_link() {
    let example = fresh("example").join("geometry");
    copy(&repo().join("examples").join("05-geometry"), &example);
    let documented = match nova_doc::document_package(&example) {
        Ok(d) => d,
        Err(nova_doc::Failure::Diagnostics { db, diagnostics }) => panic!(
            "{}",
            nova_diagnostics::render::render_to_string(&db, &diagnostics)
        ),
        Err(nova_doc::Failure::Other(why)) => panic!("{why}"),
    };
    assert!(documented.warnings.is_empty());
    let dir = example.join("target").join("doc");
    nova_doc::write_site(&documented.site, &dir).expect("the site is written");
    let text = |path: &str| std::fs::read_to_string(example.join(path)).unwrap();
    let mut sources = vec![
        Source {
            page: "geometry/index.html".to_string(),
            package: "geometry".to_string(),
            file: common::parse(&text("src/main.nova")),
            all: true,
        },
        Source {
            page: "shapes/index.html".to_string(),
            package: "shapes".to_string(),
            file: common::parse(&text("shapes/src/lib.nova")),
            all: false,
        },
        Source {
            page: "shapes/units/index.html".to_string(),
            package: "shapes".to_string(),
            file: common::parse(&text("shapes/src/units.nova")),
            all: false,
        },
    ];
    sources.extend(common::std_sources());
    let missing = common::missing(&dir, &common::anchors(&sources));
    assert!(
        missing.is_empty(),
        "{} missing:\n{}",
        missing.len(),
        missing.join("\n")
    );
    assert_eq!(nova_doc::check_links(&dir), Vec::new());
}
