//! `nova.toml` parsing (spec
//! `docs/superpowers/specs/2026-10-07-phase-3-0-foundations-design.md` §5).

use nova_diagnostics::{render, FileDb};
use nova_pm::{parse, Manifest};

/// Parse `source` as a `nova.toml`. Return the manifest, the diagnostics'
/// codes in order, and their rendering.
fn check(source: &str) -> (Option<Manifest>, Vec<String>, String) {
    let mut db = FileDb::new();
    let file = db.add("nova.toml", source);
    let (manifest, diagnostics) = parse(source, file);
    let codes = diagnostics.iter().map(|d| d.code.clone()).collect();
    let rendered = render::render_to_string(&db, &diagnostics);
    (manifest, codes, rendered)
}

const TEMPLATE: &str =
    "[package]\nname = \"demo\"\nversion = \"0.1.0\"\nedition = \"2026\"\n\n[dependencies]\n";

/// A `[package]` table with every required key, `key` set to the TOML
/// value `value`.
fn package_with(key: &str, value: &str) -> String {
    let mut text = String::from("[package]\n");
    for (k, v) in [
        ("name", "\"demo\""),
        ("version", "\"0.1.0\""),
        ("edition", "\"2026\""),
    ] {
        let v = if k == key { value } else { v };
        text.push_str(&format!("{k} = {v}\n"));
    }
    text
}

#[test]
fn the_template_parses() {
    let (manifest, codes, _) = check(TEMPLATE);
    assert!(codes.is_empty(), "{codes:?}");
    let manifest = manifest.expect("a manifest");
    assert_eq!(manifest.package.name, "demo");
    assert_eq!(manifest.package.version, semver::Version::new(0, 1, 0));
    assert_eq!(manifest.package.edition, "2026");
    assert!(manifest.dependencies.is_empty());
    assert!(manifest.dev_dependencies.is_empty());
}

#[test]
fn the_optional_keys_are_read() {
    let source = "[package]\nname = \"demo\"\nversion = \"1.2.3-alpha.1\"\n\
                  edition = \"2026\"\ndescription = \"d\"\nlicense = \"MIT\"\n\
                  repository = \"https://example.com/r\"\nauthors = [\"A <a@example.com>\"]\n\
                  keywords = [\"k\"]\ncategories = [\"c\"]\n";
    let (manifest, codes, _) = check(source);
    assert!(codes.is_empty(), "{codes:?}");
    let package = manifest.unwrap().package;
    assert_eq!(package.version.to_string(), "1.2.3-alpha.1");
    assert_eq!(package.description.as_deref(), Some("d"));
    assert_eq!(package.license.as_deref(), Some("MIT"));
    assert_eq!(package.repository.as_deref(), Some("https://example.com/r"));
    assert_eq!(package.authors, ["A <a@example.com>"]);
    assert_eq!(package.keywords, ["k"]);
    assert_eq!(package.categories, ["c"]);
}

#[test]
fn a_missing_package_table_is_m0002_at_line_1_column_1() {
    for source in ["", "[dependencies]\n"] {
        let (manifest, codes, rendered) = check(source);
        assert!(manifest.is_none(), "{source:?}");
        assert_eq!(codes, ["M0002"], "{source:?}");
        assert!(rendered.contains("nova.toml:1:1"), "{rendered}");
    }
}

#[test]
fn each_missing_required_key_is_m0002_at_the_package_header() {
    for (key, source) in [
        (
            "name",
            "[package]\nversion = \"0.1.0\"\nedition = \"2026\"\n",
        ),
        (
            "version",
            "[package]\nname = \"demo\"\nedition = \"2026\"\n",
        ),
        (
            "edition",
            "[package]\nname = \"demo\"\nversion = \"0.1.0\"\n",
        ),
    ] {
        let (manifest, codes, rendered) = check(source);
        assert!(manifest.is_none(), "{key}");
        assert_eq!(codes, ["M0002"], "{key}");
        assert!(rendered.contains(&format!("no `{key}`")), "{rendered}");
        assert!(rendered.contains("nova.toml:1:1"), "{rendered}");
    }
}

#[test]
fn each_broken_rule_is_m0003() {
    let long = format!("\"{}\"", "a".repeat(65));
    let cases = [
        ("name", "\"1abc\"", "must start with an ASCII letter"),
        ("name", "\"a b\"", "contains ' '"),
        ("name", "\"ไทย\"", "must start with an ASCII letter"),
        ("name", "\"aไทย\"", "contains 'ไ'"),
        ("name", long.as_str(), "the limit is 64"),
        ("name", "\"con\"", "reserves for a device"),
        ("name", "\"COM1\"", "reserves for a device"),
        ("name", "\"Lpt9\"", "reserves for a device"),
        // Review Focus 5: names that look like paths.
        ("name", "\"../escape\"", "must start with an ASCII letter"),
        ("name", "\"a/b\"", "contains '/'"),
        ("name", "5", "must be a string"),
        ("version", "\"1.0\"", "is not a version"),
        ("version", "\"1.0.0+build\"", "build metadata"),
        ("edition", "\"2021\"", "unknown edition"),
    ];
    for (key, value, expected) in cases {
        let source = package_with(key, value);
        let (manifest, codes, rendered) = check(&source);
        assert!(manifest.is_none(), "{source}");
        assert_eq!(codes, ["M0003"], "{source}");
        assert!(rendered.contains(expected), "{source}\n{rendered}");
    }
}

#[test]
fn each_dependency_shape_is_read() {
    let source = format!(
        "{TEMPLATE}a = \"1.2\"\nb = {{ version = \"^0.3\" }}\nc = {{ path = \"../c\" }}\n\n\
         [dependencies.d]\nversion = \"*\"\n\n[dev-dependencies]\ne = \"0.1\"\n"
    );
    let (manifest, codes, _) = check(&source);
    assert!(codes.is_empty(), "{codes:?}");
    let manifest = manifest.unwrap();
    let names: Vec<_> = manifest
        .dependencies
        .iter()
        .map(|d| d.name.as_str())
        .collect();
    assert_eq!(names, ["a", "b", "c", "d"]);
    assert_eq!(
        manifest.dependencies[0].version,
        Some(semver::VersionReq::parse("1.2").unwrap())
    );
    assert_eq!(manifest.dependencies[2].version, None);
    assert_eq!(
        manifest.dependencies[2].path,
        Some(std::path::PathBuf::from("../c"))
    );
    assert_eq!(manifest.dev_dependencies[0].name, "e");
}

#[test]
fn an_entry_with_neither_key_is_m0004_and_one_with_both_is_m0003() {
    let (_, codes, _) = check(&format!("{TEMPLATE}x = {{ features = [\"f\"] }}\n"));
    assert_eq!(codes, ["M0006", "M0004"]);
    let (_, codes, _) = check(&format!(
        "{TEMPLATE}x = {{ git = \"https://example.com/x\" }}\n"
    ));
    assert_eq!(codes, ["M0006", "M0004"]);
    let (_, codes, rendered) = check(&format!(
        "{TEMPLATE}x = {{ version = \"1\", path = \"../x\" }}\n"
    ));
    assert_eq!(codes, ["M0003"]);
    assert!(rendered.contains("both"), "{rendered}");
}

#[test]
fn a_bad_requirement_is_m0003() {
    let (_, codes, rendered) = check(&format!("{TEMPLATE}x = \"^^1\"\n"));
    assert_eq!(codes, ["M0003"]);
    assert!(rendered.contains("^^1"), "{rendered}");
}

#[test]
fn unknown_keys_warn_at_every_level_and_leave_the_manifest() {
    let source = "[package]\nname = \"demo\"\nversion = \"0.1.0\"\nedition = \"2026\"\n\
                  homepage = \"h\"\n\n[dependencies]\nx = { version = \"1\", optional = true }\n\n\
                  [features]\ndefault = []\n\n[[bin]]\nname = \"b\"\n";
    let (manifest, codes, rendered) = check(source);
    assert_eq!(codes, ["M0006", "M0006", "M0006", "M0006"]);
    assert!(manifest.is_some());
    for key in [
        "features",
        "bin",
        "package.homepage",
        "dependencies.x.optional",
    ] {
        assert!(rendered.contains(&format!("`{key}`")), "{key}: {rendered}");
    }
}

#[test]
fn invalid_toml_is_m0001_where_parsing_stops() {
    let (manifest, codes, rendered) = check("[package]\nname = \n");
    assert!(manifest.is_none());
    assert_eq!(codes, ["M0001"]);
    assert!(rendered.contains("nova.toml:2:"), "{rendered}");
}

#[test]
fn a_rendered_error_shows_its_line_and_column() {
    let (_, codes, rendered) = check(&package_with("edition", "\"2021\""));
    assert_eq!(codes, ["M0003"]);
    assert!(rendered.contains("nova.toml:4:11"), "{rendered}");
}

/// Review Focus 2.
#[test]
fn a_manifest_with_crlf_line_endings_parses_and_points_right() {
    let (manifest, codes, _) = check(&TEMPLATE.replace('\n', "\r\n"));
    assert!(codes.is_empty(), "{codes:?}");
    assert!(manifest.is_some());
    let crlf = package_with("edition", "\"2021\"").replace('\n', "\r\n");
    let (_, codes, rendered) = check(&crlf);
    assert_eq!(codes, ["M0003"]);
    assert!(rendered.contains("nova.toml:4:11"), "{rendered}");
}

/// Review Focus 2: PowerShell 5.1's `Set-Content -Encoding utf8` writes a
/// byte-order mark.
#[test]
fn a_manifest_saved_with_a_byte_order_mark_parses() {
    let (manifest, codes, _) = check(&format!("\u{feff}{TEMPLATE}"));
    assert!(codes.is_empty(), "{codes:?}");
    assert_eq!(manifest.unwrap().package.name, "demo");
}
