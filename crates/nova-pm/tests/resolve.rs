//! The resolver (spec
//! `docs/superpowers/specs/2026-10-09-phase-3-3b-index-and-publishing-design.md`
//! §4).

use std::collections::HashMap;

use nova_diagnostics::{FileId, Span};
use nova_pm::{
    resolve, Candidate, IndexView, Lock, LockedPackage, Requirement, ResolveError, Unlock,
};
use semver::{Version, VersionReq};

/// An index held in memory: each name's versions.
struct Fake(HashMap<&'static str, Vec<Candidate>>);

impl IndexView for Fake {
    fn versions(&mut self, name: &str) -> Result<Option<Vec<Candidate>>, String> {
        Ok(self.0.get(name).cloned())
    }
}

fn index(entries: Vec<(&'static str, Vec<Candidate>)>) -> Fake {
    Fake(entries.into_iter().collect())
}

/// A version with its dependencies.
fn c(version: &str, deps: &[(&str, &str)]) -> Candidate {
    Candidate {
        version: Version::parse(version).unwrap(),
        deps: deps
            .iter()
            .map(|(n, r)| (n.to_string(), VersionReq::parse(r).unwrap()))
            .collect(),
        checksum: format!("sum-{version}"),
    }
}

/// A requirement the root, `app`, makes.
fn req(name: &str, r: &str) -> Requirement {
    Requirement {
        name: name.into(),
        req: VersionReq::parse(r).unwrap(),
        by: "app".into(),
        span: Span::new(0, 0, FileId::DUMMY),
    }
}

fn lock_of(pairs: &[(&str, &str)]) -> Lock {
    Lock {
        index: "https://example.test/".into(),
        packages: pairs
            .iter()
            .map(|(n, v)| LockedPackage {
                name: n.to_string(),
                version: Version::parse(v).unwrap(),
                checksum: format!("sum-{v}"),
                dependencies: vec![],
            })
            .collect(),
    }
}

fn picked(result: &[LockedPackage]) -> Vec<String> {
    result
        .iter()
        .map(|p| format!("{} {}", p.name, p.version))
        .collect()
}

fn ok(reqs: &[Requirement], lock: Option<&Lock>, unlock: Unlock, index: &mut Fake) -> Vec<String> {
    match resolve(reqs, lock, &unlock, index) {
        Ok(result) => picked(&result),
        Err(ResolveError::Diagnostic(d)) => panic!("{} {} {:?}", d.code, d.message, d.notes),
        Err(ResolveError::Index(e)) => panic!("{e}"),
    }
}

fn failed(
    reqs: &[Requirement],
    lock: Option<&Lock>,
    unlock: Unlock,
    index: &mut Fake,
) -> nova_diagnostics::Diagnostic {
    match resolve(reqs, lock, &unlock, index) {
        Err(ResolveError::Diagnostic(d)) => d,
        other => panic!("expected a diagnostic: {:?}", other.map(|r| picked(&r))),
    }
}

#[test]
fn picks_the_newest_matching_version() {
    let mut index = index(vec![(
        "json",
        vec![c("1.0.0", &[]), c("1.4.1", &[]), c("2.0.0", &[])],
    )]);
    assert_eq!(
        ok(&[req("json", "^1")], None, Unlock::Nothing, &mut index),
        ["json 1.4.1"]
    );
}

#[test]
fn a_zero_major_caret_stays_within_its_minor() {
    let mut index = index(vec![(
        "geom",
        vec![c("0.2.0", &[]), c("0.2.5", &[]), c("0.3.0", &[])],
    )]);
    assert_eq!(
        ok(&[req("geom", "^0.2")], None, Unlock::Nothing, &mut index),
        ["geom 0.2.5"]
    );
}

#[test]
fn a_pre_release_only_when_a_requirement_names_one() {
    let mut index = index(vec![(
        "json",
        vec![c("1.0.0", &[]), c("1.1.0-beta.1", &[])],
    )]);
    assert_eq!(
        ok(&[req("json", "^1")], None, Unlock::Nothing, &mut index),
        ["json 1.0.0"]
    );
    assert_eq!(
        ok(
            &[req("json", "^1.1.0-beta.1")],
            None,
            Unlock::Nothing,
            &mut index
        ),
        ["json 1.1.0-beta.1"]
    );
}

#[test]
fn a_locked_version_that_fits_is_kept() {
    let mut index = index(vec![("json", vec![c("1.0.0", &[]), c("1.4.1", &[])])]);
    let lock = lock_of(&[("json", "1.0.0")]);
    assert_eq!(
        ok(
            &[req("json", "^1")],
            Some(&lock),
            Unlock::Nothing,
            &mut index
        ),
        ["json 1.0.0"]
    );
}

#[test]
fn a_tightened_requirement_moves_only_its_package() {
    // Review Focus 1: `json = "1"` edited to `"1.4"`.
    let mut index = index(vec![
        (
            "json",
            vec![c("1.2.0", &[]), c("1.4.0", &[]), c("1.5.0", &[])],
        ),
        ("http", vec![c("1.0.0", &[]), c("1.1.0", &[])]),
    ]);
    let lock = lock_of(&[("http", "1.0.0"), ("json", "1.2.0")]);
    assert_eq!(
        ok(
            &[req("json", "^1.4"), req("http", "^1")],
            Some(&lock),
            Unlock::Nothing,
            &mut index
        ),
        ["http 1.0.0", "json 1.5.0"]
    );
}

#[test]
fn backtracking_finds_an_older_version() {
    let mut index = index(vec![
        (
            "a",
            vec![c("1.0.0", &[("c", "^1")]), c("1.1.0", &[("c", "^2")])],
        ),
        ("b", vec![c("1.0.0", &[("c", "^1")])]),
        ("c", vec![c("1.0.0", &[]), c("2.0.0", &[])]),
    ]);
    assert_eq!(
        ok(
            &[req("a", "^1"), req("b", "^1")],
            None,
            Unlock::Nothing,
            &mut index
        ),
        ["a 1.0.0", "b 1.0.0", "c 1.0.0"]
    );
}

#[test]
fn an_impossible_conflict_is_m0015_naming_each_requirer() {
    let mut index = index(vec![
        ("geom", vec![c("1.0.0", &[("json", "^2")])]),
        ("json", vec![c("1.0.0", &[]), c("2.0.0", &[])]),
    ]);
    let d = failed(
        &[req("json", "^1"), req("geom", "^1")],
        None,
        Unlock::Nothing,
        &mut index,
    );
    assert_eq!(d.code, "M0015");
    assert!(
        d.message
            .contains("no version of `json` meets every requirement"),
        "{}",
        d.message
    );
    assert!(
        d.message.contains("`^1` from `app`") && d.message.contains("`^2` from `geom`"),
        "{}",
        d.message
    );
}

#[test]
fn a_package_the_index_lacks_is_m0014() {
    let mut index = index(vec![]);
    let d = failed(&[req("json", "^1")], None, Unlock::Nothing, &mut index);
    assert_eq!(d.code, "M0014");
    assert!(
        d.message.contains("the index has no package `json`"),
        "{}",
        d.message
    );
}

#[test]
fn updating_one_name_moves_only_it() {
    let mut index = index(vec![
        ("a", vec![c("1.0.0", &[]), c("1.5.0", &[])]),
        ("b", vec![c("1.0.0", &[]), c("1.5.0", &[])]),
    ]);
    let lock = lock_of(&[("a", "1.0.0"), ("b", "1.0.0")]);
    assert_eq!(
        ok(
            &[req("a", "^1"), req("b", "^1")],
            Some(&lock),
            Unlock::One("a".into()),
            &mut index
        ),
        ["a 1.5.0", "b 1.0.0"]
    );
}

#[test]
fn updating_one_name_says_when_the_others_block_it() {
    let mut index = index(vec![
        (
            "a",
            vec![c("1.0.0", &[("b", "^1")]), c("2.0.0", &[("b", "^2")])],
        ),
        ("b", vec![c("1.0.0", &[]), c("2.0.0", &[])]),
    ]);
    let lock = lock_of(&[("a", "1.0.0"), ("b", "1.0.0")]);
    let d = failed(
        &[req("a", "^2")],
        Some(&lock),
        Unlock::One("a".into()),
        &mut index,
    );
    assert_eq!(d.code, "M0015");
    assert!(
        d.notes.iter().any(|n| n.contains("run `nova update`")),
        "{:?}",
        d.notes
    );
}

#[test]
fn a_plain_sync_falls_back_when_the_lock_cannot_be_kept() {
    // `d` is new, and needs a newer `b` than the one locked.
    let mut index = index(vec![
        ("b", vec![c("1.0.0", &[]), c("1.1.0", &[])]),
        ("d", vec![c("1.0.0", &[("b", "^1.1")])]),
    ]);
    let lock = lock_of(&[("b", "1.0.0")]);
    assert_eq!(
        ok(
            &[req("b", "^1"), req("d", "^1")],
            Some(&lock),
            Unlock::Nothing,
            &mut index
        ),
        ["b 1.1.0", "d 1.0.0"]
    );
}

#[test]
fn the_result_is_sorted_with_each_packages_dependencies() {
    let mut index = index(vec![
        ("geom", vec![c("1.0.0", &[("json", "^1"), ("http", "^1")])]),
        ("json", vec![c("1.0.0", &[])]),
        ("http", vec![c("1.0.0", &[])]),
    ]);
    let result = resolve(&[req("geom", "^1")], None, &Unlock::Nothing, &mut index).unwrap();
    assert_eq!(picked(&result), ["geom 1.0.0", "http 1.0.0", "json 1.0.0"]);
    assert_eq!(result[0].dependencies, ["http", "json"]);
    assert_eq!(result[0].checksum, "sum-1.0.0");
}

#[test]
fn an_index_error_is_passed_on() {
    struct Broken;
    impl IndexView for Broken {
        fn versions(&mut self, _: &str) -> Result<Option<Vec<Candidate>>, String> {
            Err("cannot reach the index".into())
        }
    }
    match resolve(&[req("json", "^1")], None, &Unlock::Nothing, &mut Broken) {
        Err(ResolveError::Index(message)) => assert_eq!(message, "cannot reach the index"),
        _ => panic!("expected the index's error"),
    }
}

#[test]
fn a_conflict_moves_only_the_packages_it_names() {
    // Final review I4: `d` is new and needs a newer `b` than the locked one.
    // Only `b` moves; `a` and `c` keep their locked versions though newer
    // ones fit (spec §4.3).
    let mut index = index(vec![
        ("a", vec![c("1.0.0", &[]), c("1.5.0", &[])]),
        ("b", vec![c("1.0.0", &[]), c("1.1.0", &[])]),
        ("c", vec![c("1.0.0", &[]), c("1.5.0", &[])]),
        ("d", vec![c("1.0.0", &[("b", "^1.1")])]),
    ]);
    let lock = lock_of(&[("a", "1.0.0"), ("b", "1.0.0"), ("c", "1.0.0")]);
    assert_eq!(
        ok(
            &[
                req("a", "^1"),
                req("b", "^1"),
                req("c", "^1"),
                req("d", "^1")
            ],
            Some(&lock),
            Unlock::Nothing,
            &mut index
        ),
        ["a 1.0.0", "b 1.1.0", "c 1.0.0", "d 1.0.0"]
    );
}
