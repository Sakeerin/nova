//! Import names, real paths, and the spans the graph's diagnostics point
//! at (spec
//! `docs/superpowers/specs/2026-10-08-phase-3-3a-local-packages-design.md`
//! §3.3–§3.5).

use nova_diagnostics::FileDb;
use nova_pm::{import_name, real_path};

#[test]
fn a_hyphen_becomes_an_underscore_in_an_import_name() {
    assert_eq!(import_name("json-api"), "json_api");
    assert_eq!(import_name("geom"), "geom");
    assert_eq!(import_name("a-b_c-d"), "a_b_c_d");
}

#[test]
fn a_real_path_is_canonical_without_the_verbatim_prefix() {
    let dir = std::env::temp_dir().join("nova-pm-real-path");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("a")).unwrap();
    let real = real_path(&dir.join("a").join(".."));
    assert_eq!(real, real_path(&dir));
    assert!(
        !real.to_string_lossy().starts_with(r"\\?\"),
        "{}",
        real.display()
    );
    // A path that does not exist comes back as it was given.
    let missing = dir.join("missing");
    assert_eq!(real_path(&missing), missing);
}

#[test]
fn the_package_table_and_a_dependency_path_keep_their_positions() {
    let source = "[package]\nname = \"demo\"\nversion = \"0.1.0\"\nedition = \"2026\"\n\n\
                  [dependencies]\ngeom = { path = \"../geom\" }\nhttp = \"1.0\"\n";
    let mut db = FileDb::new();
    let file = db.add("nova.toml", source);
    let (manifest, diagnostics) = nova_pm::parse(source, file);
    assert!(diagnostics.is_empty(), "{diagnostics:?}");
    let manifest = manifest.unwrap();
    assert_eq!(db.location(file, manifest.package.span.start), Some((1, 1)));
    let path = manifest.dependencies[0].path_span.expect("a path span");
    assert_eq!(&source[path.as_range()], "\"../geom\"");
    assert_eq!(manifest.dependencies[1].path_span, None);
}
#[test]
fn a_portable_name_is_one_every_system_can_hold() {
    for good in [
        "lib.nova",
        "README.ไทย.md",
        "a-b_c",
        "LICENSE",
        "con-x.nova",
        "x.y.z",
    ] {
        assert!(nova_pm::is_portable(good), "{good}");
    }
    for bad in [
        "",
        ".",
        "..",
        "a:b",
        "a\\b",
        "a/b",
        "a<b",
        "a>b",
        "a\"b",
        "a|b",
        "a?b",
        "a*b",
        "tab\there",
        "dot.",
        "space ",
        "con",
        "CON.nova",
        "aux.txt",
        "nul",
        "com1.x",
        "LPT9",
    ] {
        assert!(!nova_pm::is_portable(bad), "{bad:?}");
    }
}
