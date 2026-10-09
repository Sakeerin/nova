//! Packing and unpacking (spec
//! `docs/superpowers/specs/2026-10-09-phase-3-3b-index-and-publishing-design.md`
//! §5.2, §6.5, §10.3).

use std::io::Read;
use std::path::{Path, PathBuf};

use nova_index::{pack, sha256_hex, unpack, unpack_limited, Limits};
use semver::Version;

/// A fresh, empty directory with a fixed name, so each run replaces the last.
fn fresh(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("nova-index-pack-{name}"));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn write(dir: &Path, path: &str, bytes: &[u8]) {
    let path = dir.join(path);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, bytes).unwrap();
}

fn manifest(name: &str, version: &str, extra: &str) -> String {
    format!("[package]\nname = \"{name}\"\nversion = \"{version}\"\nedition = \"2026\"\n{extra}")
}

/// The library `geom` 0.1.0 at `dir`.
fn geom(dir: &Path) {
    write(dir, "nova.toml", manifest("geom", "0.1.0", "").as_bytes());
    write(dir, "src/lib.nova", b"pub fn area() -> Int {\n    9\n}\n");
}

fn v010() -> Version {
    Version::new(0, 1, 0)
}

/// Each entry's path and header, in order.
fn entries(tarball: &[u8]) -> Vec<(String, tar::Header)> {
    let mut archive = tar::Archive::new(flate2::read::GzDecoder::new(tarball));
    archive
        .entries()
        .unwrap()
        .map(|entry| {
            let entry = entry.unwrap();
            let path = String::from_utf8(entry.path_bytes().into_owned()).unwrap();
            (path, entry.header().clone())
        })
        .collect()
}

/// A tarball holding each `(path, entry type, data)` exactly as given, with
/// no checks on the path, as a hostile index could serve. The path is
/// written straight into the header, since `tar`'s own setters refuse `..`
/// and absolute paths. Paths and data are `&str`, so every case's tuple
/// has one type.
fn crafted(entries: &[(&str, tar::EntryType, &str)]) -> Vec<u8> {
    let mut builder = tar::Builder::new(Vec::new());
    for (path, kind, data) in entries {
        let mut header = tar::Header::new_gnu();
        header.as_old_mut().name[..path.len()].copy_from_slice(path.as_bytes());
        header.set_entry_type(*kind);
        header.set_size(data.len() as u64);
        header.set_mode(0o644);
        if *kind == tar::EntryType::Symlink {
            header.set_link_name("elsewhere").unwrap();
        }
        header.set_cksum();
        builder.append(&header, data.as_bytes()).unwrap();
    }
    let tar = builder.into_inner().unwrap();
    let mut gz = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
    std::io::Write::write_all(&mut gz, &tar).unwrap();
    gz.finish().unwrap()
}

const GEOM_TOML: &str = "[package]\nname = \"geom\"\nversion = \"0.1.0\"\nedition = \"2026\"\n";

#[test]
fn packing_twice_gives_identical_bytes() {
    let dir = fresh("twice");
    geom(&dir);
    let first = pack(&dir, "geom", &v010()).unwrap();
    let second = pack(&dir, "geom", &v010()).unwrap();
    assert_eq!(first.bytes, second.bytes);
    assert_eq!(first.checksum, sha256_hex(&first.bytes));
    assert_eq!(first.checksum.len(), 64);
    assert!(first
        .checksum
        .bytes()
        .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b)));
    assert_eq!(first.files, 2);
}

#[test]
fn only_the_published_files_are_packed() {
    let dir = fresh("published");
    geom(&dir);
    for path in [
        "src/util/more.nova",
        "tests/area.nova",
        "README.md",
        "LICENSE",
        ".git/config",
        "src/.hidden.nova",
        "target/debug/x",
        "nova.lock",
        "notes.txt",
        "docs/guide.md",
    ] {
        write(&dir, path, b"x");
    }
    let packed = pack(&dir, "geom", &v010()).unwrap();
    let listed = entries(&packed.bytes);
    let paths: Vec<&str> = listed.iter().map(|(p, _)| p.as_str()).collect();
    assert_eq!(
        paths,
        [
            "geom-0.1.0/LICENSE",
            "geom-0.1.0/README.md",
            "geom-0.1.0/nova.toml",
            "geom-0.1.0/src/lib.nova",
            "geom-0.1.0/src/util/more.nova",
            "geom-0.1.0/tests/area.nova",
        ]
    );
    for (path, header) in &listed {
        assert_eq!(header.mode().unwrap(), 0o644, "{path}");
        assert_eq!(header.mtime().unwrap(), 0, "{path}");
        assert_eq!(header.uid().unwrap(), 0, "{path}");
        assert_eq!(header.entry_type(), tar::EntryType::Regular, "{path}");
    }
    assert_eq!(packed.files, 6);
}

#[test]
fn a_file_name_outside_ascii_survives_packing_and_unpacking() {
    // Review Focus 3.
    let dir = fresh("thai");
    geom(&dir);
    let thai = "README.ไทย.md";
    write(&dir, thai, "สวัสดี\n".as_bytes());
    let packed = pack(&dir, "geom", &v010()).unwrap();
    assert!(entries(&packed.bytes)
        .iter()
        .any(|(path, _)| path == &format!("geom-0.1.0/{thai}")));
    let dest = fresh("thai-out").join("geom-0.1.0");
    unpack(&packed.bytes, "geom", &v010(), &[], &dest).unwrap();
    assert_eq!(std::fs::read_to_string(dest.join(thai)).unwrap(), "สวัสดี\n");
}

/// A symbolic link at `path` to `target`; `false` where links need a
/// privilege this run lacks (Windows without developer mode).
fn link(target: &Path, path: &Path) -> bool {
    #[cfg(unix)]
    return std::os::unix::fs::symlink(target, path).is_ok();
    #[cfg(windows)]
    return std::os::windows::fs::symlink_file(target, path).is_ok();
}

#[test]
fn a_symbolic_link_is_refused() {
    let dir = fresh("link");
    geom(&dir);
    if !link(&dir.join("src/lib.nova"), &dir.join("src/again.nova")) {
        return;
    }
    let error = pack(&dir, "geom", &v010()).unwrap_err();
    assert!(error.contains("symbolic link"), "{error}");
    assert!(error.contains("again.nova"), "{error}");
}

#[cfg(unix)]
#[test]
fn a_name_that_is_not_portable_is_refused() {
    let dir = fresh("not-portable");
    geom(&dir);
    write(&dir, "src/a:b.nova", b"");
    let error = pack(&dir, "geom", &v010()).unwrap_err();
    assert!(error.contains("a:b.nova"), "{error}");
}

#[cfg(target_os = "linux")]
#[test]
fn two_names_differing_only_in_case_are_refused() {
    let dir = fresh("case");
    geom(&dir);
    write(&dir, "src/Util.nova", b"");
    write(&dir, "src/util.nova", b"");
    let error = pack(&dir, "geom", &v010()).unwrap_err();
    assert!(error.contains("differ only in case"), "{error}");
}

#[test]
fn unpacking_refuses_what_could_escape_or_merge() {
    use tar::EntryType::{Char, Directory, Regular, Symlink};
    let cases: Vec<(&str, Vec<(&str, tar::EntryType, &str)>)> = vec![
        (
            "a parent directory",
            vec![("geom-0.1.0/../evil", Regular, "x")],
        ),
        ("an absolute path", vec![("/etc/evil", Regular, "x")]),
        ("outside the package", vec![("other-1.0.0/x", Regular, "x")]),
        ("a link", vec![("geom-0.1.0/src/lib.nova", Symlink, "")]),
        ("a device", vec![("geom-0.1.0/dev", Char, "")]),
        (
            "not portable",
            vec![("geom-0.1.0/src/a:b.nova", Regular, "x")],
        ),
        (
            "differ only in case",
            vec![
                ("geom-0.1.0/src/Util.nova", Regular, "x"),
                ("geom-0.1.0/src/util.nova", Regular, "x"),
            ],
        ),
        ("a backslash", vec![("geom-0.1.0\\..\\evil", Regular, "x")]),
    ];
    for (what, mut list) in cases {
        list.push(("geom-0.1.0/nova.toml", Regular, GEOM_TOML));
        list.push(("geom-0.1.0/src/", Directory, ""));
        let out = fresh("refused");
        let dest = out.join("geom-0.1.0");
        let result = unpack(&crafted(&list), "geom", &v010(), &[], &dest);
        assert!(result.is_err(), "{what}: unpacked");
        assert!(!dest.exists(), "{what}: left the package");
        let left: Vec<_> = std::fs::read_dir(&out).unwrap().collect();
        assert!(left.is_empty(), "{what}: left {left:?}");
        assert!(!out.parent().unwrap().join("evil").exists(), "{what}");
    }
}

#[test]
fn the_manifest_must_be_the_index_lines() {
    use tar::EntryType::Regular;
    let deps = ["json".to_string()];
    let cases = [
        (
            "another name",
            manifest("other", "0.1.0", "\n[dependencies]\njson = \"1\"\n"),
        ),
        (
            "another version",
            manifest("geom", "0.2.0", "\n[dependencies]\njson = \"1\"\n"),
        ),
        (
            "a path dependency",
            manifest(
                "geom",
                "0.1.0",
                "\n[dependencies]\njson = { path = \"../json\" }\n",
            ),
        ),
        (
            "other dependencies",
            manifest("geom", "0.1.0", "\n[dependencies]\nhttp = \"1\"\n"),
        ),
        ("a broken manifest", "[package\n".to_string()),
    ];
    for (what, text) in cases {
        let tarball = crafted(&[("geom-0.1.0/nova.toml", Regular, text.as_str())]);
        let dest = fresh("manifest").join("geom-0.1.0");
        let error = unpack(&tarball, "geom", &v010(), &deps, &dest).unwrap_err();
        assert!(!dest.exists(), "{what}");
        assert!(!error.is_empty(), "{what}");
    }
    let good = manifest("geom", "0.1.0", "\n[dependencies]\njson = \"1\"\n");
    let tarball = crafted(&[("geom-0.1.0/nova.toml", Regular, good.as_str())]);
    let dest = fresh("manifest-good").join("geom-0.1.0");
    unpack(&tarball, "geom", &v010(), &deps, &dest).unwrap();
    assert!(dest.join("nova.toml").is_file());
}

#[test]
fn limits_stop_unpacking() {
    use tar::EntryType::Regular;
    let hundred = "x".repeat(100);
    let tarball = crafted(&[
        ("geom-0.1.0/nova.toml", Regular, GEOM_TOML),
        ("geom-0.1.0/src/lib.nova", Regular, hundred.as_str()),
    ]);
    let tight = Limits {
        bytes: 99,
        entries: 10,
    };
    let dest = fresh("limits").join("geom-0.1.0");
    let error = unpack_limited(&tarball, "geom", &v010(), &[], &dest, &tight).unwrap_err();
    assert!(error.contains("the limit of 99 bytes"), "{error}");
    let few = Limits {
        bytes: 1 << 20,
        entries: 1,
    };
    let error = unpack_limited(&tarball, "geom", &v010(), &[], &dest, &few).unwrap_err();
    assert!(error.contains("entries"), "{error}");
    assert!(!dest.exists());
}

#[test]
fn a_package_already_unpacked_is_not_unpacked_again() {
    let dest = fresh("already").join("geom-0.1.0");
    write(&dest, "nova.toml", GEOM_TOML.as_bytes());
    // Not a tarball at all: it is never read.
    unpack(b"garbage", "geom", &v010(), &[], &dest).unwrap();
}

/// Two processes unpacking one package at once both succeed and leave one
/// copy (spec §5.2). Each child runs only `unpack_as_a_child_process`, its
/// output piped so its own `test result:` line stays out of this run's.
#[test]
fn two_processes_unpacking_at_once_both_succeed() {
    let out = fresh("processes");
    let source = fresh("processes-source");
    geom(&source);
    let packed = pack(&source, "geom", &v010()).unwrap();
    let tarball = out.join("geom-0.1.0.nova-pkg");
    std::fs::write(&tarball, &packed.bytes).unwrap();
    let me = std::env::current_exe().unwrap();
    let children: Vec<_> = (0..2)
        .map(|_| {
            std::process::Command::new(&me)
                .args(["--exact", "unpack_as_a_child_process", "--nocapture"])
                .env("NOVA_TEST_UNPACK", &tarball)
                .stdout(std::process::Stdio::piped())
                .stderr(std::process::Stdio::piped())
                .spawn()
                .unwrap()
        })
        .collect();
    for child in children {
        let output = child.wait_with_output().unwrap();
        assert!(
            output.status.success(),
            "a child failed:\n{}\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }
    let mut names: Vec<String> = std::fs::read_dir(out.join("src"))
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();
    assert_eq!(names, ["geom-0.1.0"]);
    let mut text = String::new();
    std::fs::File::open(out.join("src/geom-0.1.0/src/lib.nova"))
        .unwrap()
        .read_to_string(&mut text)
        .unwrap();
    assert!(text.contains("area"));
}

/// Does nothing unless `NOVA_TEST_UNPACK` names a tarball, which only the
/// test above sets, for its children. It unpacks into `src/` beside it.
#[test]
fn unpack_as_a_child_process() {
    let Some(tarball) = std::env::var_os("NOVA_TEST_UNPACK") else {
        return;
    };
    let tarball = PathBuf::from(tarball);
    let bytes = std::fs::read(&tarball).unwrap();
    let dest = tarball.parent().unwrap().join("src").join("geom-0.1.0");
    unpack(&bytes, "geom", &v010(), &[], &dest).unwrap();
    assert!(dest.join("src/lib.nova").is_file());
}
