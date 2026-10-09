//! The index's format and a local index (spec
//! `docs/superpowers/specs/2026-10-09-phase-3-3b-index-and-publishing-design.md`
//! §3, §10.3).

use std::path::{Path, PathBuf};

use nova_index::{
    fill_dl, index_path, parse_config, parse_lines, Index, Line, LineDep, LocalReader, Location,
    Source, View, DEFAULT_INDEX,
};
use nova_pm::IndexView;

/// A fresh, empty directory with a fixed name, so each run replaces the last.
fn fresh(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("nova-index-index-{name}"));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn write(dir: &Path, path: &str, text: &str) {
    let path = dir.join(path);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, text).unwrap();
}

fn line(name: &str, vers: &str, deps: &[(&str, &str)]) -> Line {
    Line {
        name: name.into(),
        vers: vers.into(),
        deps: deps
            .iter()
            .map(|(n, r)| LineDep {
                name: n.to_string(),
                req: r.to_string(),
            })
            .collect(),
        cksum: "a".repeat(64),
        v: 1,
    }
}

#[test]
fn index_paths_follow_cargos_sparse_layout_in_lower_case() {
    for (name, path) in [
        ("a", "1/a"),
        ("ab", "2/ab"),
        ("abc", "3/a/abc"),
        ("geom", "ge/om/geom"),
        ("json-api", "js/on/json-api"),
        ("Geom", "ge/om/geom"),
        ("JSON_X", "js/on/json_x"),
    ] {
        assert_eq!(index_path(name), path, "{name}");
    }
}

#[test]
fn an_index_is_read_over_https_or_from_a_local_directory() {
    let http = Index::new("HTTPS://Example.Test/index").unwrap();
    assert_eq!(http.canonical, "https://example.test/index/");
    assert_eq!(
        http.source,
        Source::Http("https://example.test/index/".into())
    );
    let dir = fresh("local");
    let local = Index::new(dir.to_str().unwrap()).unwrap();
    assert!(
        local.canonical.starts_with("file:///"),
        "{}",
        local.canonical
    );
    let Source::Local(found) = &local.source else {
        panic!("{:?}", local.source);
    };
    assert_eq!(nova_pm::real_path(found), nova_pm::real_path(&dir));
    assert!(Index::new("http://example.test/").is_err());
    assert!(Index::new("relative/dir").is_err());
}

#[test]
fn an_unset_or_empty_nova_index_is_the_default() {
    for setting in [None, Some(""), Some("  ")] {
        let index = Index::from_setting(setting).unwrap();
        assert_eq!(index.canonical, DEFAULT_INDEX, "{setting:?}");
    }
    let error = Index::from_setting(Some("http://example.test/")).unwrap_err();
    assert!(error.starts_with("NOVA_INDEX: "), "{error}");
}

#[test]
fn a_tarball_is_at_an_absolute_url_or_relative_to_the_root() {
    let http = Index::new("https://example.test/index").unwrap();
    assert_eq!(
        http.tarball("dl/{name}-{version}.nova-pkg", "geom", "0.2.0")
            .unwrap(),
        Location::Url("https://example.test/index/dl/geom-0.2.0.nova-pkg".into())
    );
    assert_eq!(
        http.tarball("https://cdn.example.test/{name}/{version}", "geom", "0.2.0")
            .unwrap(),
        Location::Url("https://cdn.example.test/geom/0.2.0".into())
    );
    let dir = fresh("tarball");
    let local = Index::new(dir.to_str().unwrap()).unwrap();
    match local
        .tarball("dl/{name}-{version}.nova-pkg", "geom", "0.2.0")
        .unwrap()
    {
        Location::File(path) => assert!(path.ends_with("dl/geom-0.2.0.nova-pkg"), "{path:?}"),
        other => panic!("{other:?}"),
    }
    for bad in [
        "../{name}",
        "/abs/{name}",
        "a/../../{name}",
        "C:/x/{name}",
        "a\\b/{name}",
        "",
    ] {
        assert!(http.tarball(bad, "geom", "0.2.0").is_err(), "{bad}");
    }
    assert_eq!(
        fill_dl("{name}/{version}/{name}", "geom", "1.0.0"),
        "geom/1.0.0/geom"
    );
}

#[test]
fn a_line_is_written_as_one_json_object() {
    assert_eq!(
        line("geom", "0.2.0", &[("json-api", "^1.0")]).to_json(),
        format!(
            "{{\"name\":\"geom\",\"vers\":\"0.2.0\",\"deps\":[{{\"name\":\"json-api\",\
             \"req\":\"^1.0\"}}],\"cksum\":\"{}\",\"v\":1}}",
            "a".repeat(64)
        )
    );
}

#[test]
fn lines_of_a_later_format_are_skipped_and_bad_ones_noted() {
    let good = line("geom", "0.2.0", &[]).to_json();
    let later = good.replace("\"v\":1", "\"v\":2");
    let text = format!(
        "{good}\n{later}\nnot json\n{{\"name\":\"geom\"}}\n{}\n\n",
        good.replace("0.2.0", "zero")
    );
    let (lines, notes) = parse_lines(&text, "ge/om/geom");
    assert_eq!(lines, [line("geom", "0.2.0", &[])]);
    assert_eq!(notes.len(), 3, "{notes:?}");
    assert!(
        notes[0].starts_with("ge/om/geom:3: skipped a line that does not parse"),
        "{notes:?}"
    );
    assert!(notes[1].starts_with("ge/om/geom:4:"), "{notes:?}");
    assert!(notes[2].starts_with("ge/om/geom:5:"), "{notes:?}");
}

#[test]
fn config_json_has_dl_and_perhaps_api() {
    let config = parse_config(
        "{\"dl\": \"dl/{name}-{version}.nova-pkg\", \"api\": \"Sakeerin/nova-index\"}",
    )
    .unwrap();
    assert_eq!(config.dl, "dl/{name}-{version}.nova-pkg");
    assert_eq!(config.api.as_deref(), Some("Sakeerin/nova-index"));
    assert_eq!(parse_config("{\"dl\": \"x\"}").unwrap().api, None);
    assert!(parse_config("{}").is_err());
}

#[test]
fn a_local_index_is_read_through_a_view() {
    let dir = fresh("view");
    let text = format!(
        "{}\n{}\nnot json\n",
        line("geom", "0.1.0", &[]).to_json(),
        line("geom", "0.2.0", &[("json", "^1")]).to_json()
    );
    write(&dir, "ge/om/geom", &text);
    let mut reader = LocalReader { dir: dir.clone() };
    let mut view = View::new(&mut reader);
    let versions = view.versions("geom").unwrap().unwrap();
    let found: Vec<String> = versions.iter().map(|c| c.version.to_string()).collect();
    assert_eq!(found, ["0.1.0", "0.2.0"]);
    assert_eq!(versions[1].deps[0].0, "json");
    assert_eq!(view.versions("nope").unwrap(), None);
    assert_eq!(view.notes.len(), 1, "{:?}", view.notes);
}

#[test]
fn a_view_compares_names_exactly() {
    let dir = fresh("exact");
    write(
        &dir,
        "ge/om/geom",
        &format!("{}\n", line("Geom", "1.0.0", &[]).to_json()),
    );
    let mut reader = LocalReader { dir };
    let mut view = View::new(&mut reader);
    assert_eq!(view.versions("geom").unwrap(), None);
    assert!(view.versions("Geom").unwrap().is_some());
}
