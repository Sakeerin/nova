//! `$NOVA_HOME`, the canonical index form, and the cache directory's name
//! (spec
//! `docs/superpowers/specs/2026-10-09-phase-3-3b-index-and-publishing-design.md`
//! §3.3, §5.1).

use std::path::PathBuf;

use nova_pm::{canonical_index, index_dir_name, local_index_path, nova_home};

#[test]
fn nova_home_is_the_variable_or_dot_nova_in_the_home_directory() {
    let p = |s: &str| Some(PathBuf::from(s));
    assert_eq!(nova_home(p("/n"), p("C:/u"), p("/h"), true), p("/n"));
    // An empty value counts as unset.
    assert_eq!(
        nova_home(p(""), p("C:/u"), p("/h"), true),
        Some(PathBuf::from("C:/u").join(".nova"))
    );
    assert_eq!(
        nova_home(None, p("C:/u"), p("/h"), false),
        Some(PathBuf::from("/h").join(".nova"))
    );
    assert_eq!(nova_home(None, None, p("/h"), true), None);
    assert_eq!(nova_home(None, None, None, false), None);
}

#[test]
fn an_https_index_has_one_canonical_form() {
    let canonical = canonical_index("HTTPS://Raw.Example.COM:443/Owner/Index/main").unwrap();
    assert_eq!(canonical, "https://raw.example.com/Owner/Index/main/");
    assert_eq!(
        canonical_index("https://raw.example.com/Owner/Index/main/").unwrap(),
        canonical
    );
    assert_eq!(
        canonical_index("https://raw.example.com:8443/x").unwrap(),
        "https://raw.example.com:8443/x/"
    );
    assert_eq!(
        canonical_index("https://example.com").unwrap(),
        "https://example.com/"
    );
}

#[test]
fn plain_http_is_only_for_a_loopback_ip() {
    assert_eq!(
        canonical_index("http://127.0.0.1:8080/i").unwrap(),
        "http://127.0.0.1:8080/i/"
    );
    assert_eq!(
        canonical_index("http://[::1]:9/").unwrap(),
        "http://[::1]:9/"
    );
    for refused in [
        "http://example.com/",
        "http://localhost:8080/",
        "https://user@example.com/",
        "https://example.com/?q=1",
    ] {
        assert!(canonical_index(refused).is_err(), "{refused}");
    }
}

#[test]
fn a_local_index_is_its_real_path_whatever_its_spelling() {
    let dir = std::env::temp_dir().join("nova-pm-index name");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let canonical = canonical_index(dir.to_str().unwrap()).unwrap();
    assert!(canonical.starts_with("file:///"), "{canonical}");
    assert!(canonical.ends_with("/nova-pm-index name/"), "{canonical}");
    assert!(!canonical.contains('\\'), "{canonical}");
    // The same directory as a file: URL, its space escaped.
    let real = nova_pm::real_path(&dir);
    let slashed = real.to_string_lossy().replace('\\', "/");
    let url = if slashed.starts_with('/') {
        format!("file://{slashed}")
    } else {
        format!("file:///{slashed}")
    };
    assert_eq!(
        canonical_index(&url.replace(' ', "%20")).unwrap(),
        canonical
    );
    let back = local_index_path(&canonical).expect("a local index");
    assert_eq!(nova_pm::real_path(&back), real);
    assert_eq!(local_index_path("https://example.com/"), None);
}

#[cfg(windows)]
#[test]
fn a_windows_drive_letter_is_upper_case_in_the_canonical_form() {
    let dir = std::env::temp_dir().join("nova-pm-index-drive");
    std::fs::create_dir_all(&dir).unwrap();
    let real = nova_pm::real_path(&dir)
        .to_string_lossy()
        .replace('\\', "/");
    let lower = format!("file:///{}{}", real[..1].to_ascii_lowercase(), &real[1..]);
    let canonical = canonical_index(&lower).unwrap();
    assert_eq!(&canonical[8..9], real[..1].to_ascii_uppercase());
    assert_eq!(
        canonical_index(&real.replace('/', "\\")).unwrap(),
        canonical
    );
}

#[test]
fn a_missing_local_index_keeps_its_canonical_form() {
    // A build with the index gone must still match nova.lock's `index`
    // (spec §3.3), whatever the temp directory's spelling: an 8.3 name on
    // Windows, `/var` for `/private/var` on macOS.
    let dir = std::env::temp_dir().join("nova-pm-index-missing");
    std::fs::create_dir_all(&dir).unwrap();
    let present = canonical_index(dir.to_str().unwrap()).unwrap();
    std::fs::remove_dir_all(&dir).unwrap();
    let absent = canonical_index(dir.to_str().unwrap()).unwrap();
    assert_eq!(absent, present);
    let deeper = canonical_index(dir.join("a").join("b").to_str().unwrap()).unwrap();
    assert_eq!(deeper, format!("{present}a/b/"));
}

#[test]
fn a_relative_path_is_not_an_index() {
    assert!(canonical_index("some/dir").is_err());
    assert!(canonical_index("file:relative/dir").is_err());
    assert!(canonical_index("file://elsewhere/srv/index").is_err());
}

#[test]
fn the_cache_directory_name_is_the_host_and_a_crc() {
    let name = index_dir_name("https://raw.githubusercontent.com/Sakeerin/nova-index/main/");
    assert!(name.starts_with("raw.githubusercontent.com-"), "{name}");
    assert_eq!(name.len(), "raw.githubusercontent.com-".len() + 8);
    let ipv6 = index_dir_name("http://[::1]:9/");
    assert!(!ipv6.contains([':', '[', ']']), "{ipv6}");
    assert!(index_dir_name("file:///srv/index/").starts_with("local-"));
    assert_ne!(
        index_dir_name("https://a.example/x/"),
        index_dir_name("https://a.example/y/")
    );
}
