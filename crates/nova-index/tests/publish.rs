//! Publishing to a local index (spec
//! `docs/superpowers/specs/2026-10-09-phase-3-3b-index-and-publishing-design.md`
//! §7).

use std::path::PathBuf;

use nova_index::{publish_local, Index, Line};

fn fresh(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("nova-index-publish-{name}"));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// A local index whose `dl` is `dl`.
fn index(name: &str, dl: &str) -> (PathBuf, Index) {
    let dir = fresh(name);
    std::fs::write(dir.join("config.json"), format!("{{\"dl\":\"{dl}\"}}")).unwrap();
    let index = Index::new(dir.to_str().unwrap()).unwrap();
    (dir, index)
}

fn line(name: &str, vers: &str) -> Line {
    Line {
        name: name.into(),
        vers: vers.into(),
        deps: Vec::new(),
        cksum: "c".repeat(64),
        v: 1,
    }
}

const DL: &str = "dl/{name}-{version}.nova-pkg";

#[test]
fn the_tarball_is_written_where_dl_says_then_the_line_appended() {
    let (dir, index) = index("writes", DL);
    publish_local(&index, &line("geom", "0.1.0"), b"one").unwrap();
    publish_local(&index, &line("geom", "0.2.0"), b"two").unwrap();
    assert_eq!(
        std::fs::read(dir.join("dl/geom-0.1.0.nova-pkg")).unwrap(),
        b"one"
    );
    assert_eq!(
        std::fs::read(dir.join("dl/geom-0.2.0.nova-pkg")).unwrap(),
        b"two"
    );
    assert_eq!(
        std::fs::read_to_string(dir.join("ge/om/geom")).unwrap(),
        format!(
            "{}\n{}\n",
            line("geom", "0.1.0").to_json(),
            line("geom", "0.2.0").to_json()
        )
    );
}

#[test]
fn a_version_already_there_is_refused_and_nothing_written() {
    let (dir, index) = index("again", DL);
    publish_local(&index, &line("geom", "0.1.0"), b"one").unwrap();
    let error = publish_local(&index, &line("geom", "0.1.0"), b"other").unwrap_err();
    assert!(
        error.contains("geom 0.1.0 is already in the index"),
        "{error}"
    );
    assert_eq!(
        std::fs::read(dir.join("dl/geom-0.1.0.nova-pkg")).unwrap(),
        b"one"
    );
}

#[test]
fn a_name_differing_only_in_case_is_refused() {
    let (_, index) = index("case", DL);
    publish_local(&index, &line("Geom", "0.1.0"), b"one").unwrap();
    let error = publish_local(&index, &line("geom", "0.2.0"), b"two").unwrap_err();
    assert!(
        error.contains("the index has `Geom`, which differs from `geom` only in case"),
        "{error}"
    );
}

#[test]
fn a_local_index_needs_a_dl_under_it() {
    let (_, index) = index("url", "https://example.test/{name}");
    let error = publish_local(&index, &line("geom", "0.1.0"), b"one").unwrap_err();
    assert!(error.contains("must be a path under the index"), "{error}");
}
