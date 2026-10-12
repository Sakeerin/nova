//! The workspace's manifests are ready for crates.io (spec
//! `docs/superpowers/specs/2026-10-12-phase-3-6a-release-preparation-design.md`
//! §3, §11.1): the 19 crates `nova` needs are published, the other four are
//! not, every dependency between workspace crates is an exact pin on the
//! workspace's version, `publish-crates.sh` uploads them in dependency
//! order, each tarball carries a README and both licence texts, and the
//! VS Code extension has the same version.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::process::Command;

use serde_json::Value;

/// The members that `publish = false` keeps off crates.io (spec §3.1).
const UNPUBLISHED: [&str; 4] = [
    "nova-bench-http",
    "nova-bundler",
    "nova-codegen-wasm",
    "nova-test",
];

const REPOSITORY: &str = "https://github.com/Sakeerin/nova";

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
}

fn read(path: &Path) -> String {
    std::fs::read_to_string(path).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

/// `[workspace.package]`'s `version` in the root `Cargo.toml`.
fn workspace_version() -> String {
    let manifest: toml_edit::DocumentMut = read(&root().join("Cargo.toml"))
        .parse()
        .expect("the root Cargo.toml parses");
    manifest
        .get("workspace")
        .and_then(|w| w.get("package"))
        .and_then(|p| p.get("version"))
        .and_then(|v| v.as_str())
        .expect("[workspace.package] sets no version")
        .to_string()
}

/// One workspace member, as `cargo metadata` describes it.
struct Member {
    version: String,
    manifest_dir: PathBuf,
    published: bool,
    /// Its normal and build dependencies on other members, with their
    /// requirements.
    deps: Vec<(String, String)>,
    readme: Option<String>,
    repository: Option<String>,
    homepage: Option<String>,
    description: String,
}

fn members() -> BTreeMap<String, Member> {
    let output = Command::new(env!("CARGO"))
        .args([
            "metadata",
            "--no-deps",
            "--offline",
            "--format-version",
            "1",
        ])
        .arg("--manifest-path")
        .arg(root().join("Cargo.toml"))
        .output()
        .expect("running cargo metadata");
    assert!(
        output.status.success(),
        "cargo metadata failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let metadata: Value = serde_json::from_slice(&output.stdout).expect("cargo metadata's JSON");
    let packages = metadata["packages"].as_array().expect("packages");
    let names: BTreeSet<&str> = packages
        .iter()
        .map(|p| p["name"].as_str().expect("a package name"))
        .collect();
    let text = |value: &Value| value.as_str().map(str::to_string);
    packages
        .iter()
        .map(|p| {
            let name = p["name"].as_str().expect("a package name").to_string();
            // `publish = false` is `[]`; no `publish` key is `null`.
            let published = match &p["publish"] {
                Value::Null => true,
                Value::Array(registries) => !registries.is_empty(),
                other => panic!("{name}: publish is {other}"),
            };
            let deps = p["dependencies"]
                .as_array()
                .expect("dependencies")
                .iter()
                .filter(|d| names.contains(d["name"].as_str().expect("a dependency name")))
                .filter(|d| d["kind"].is_null() || d["kind"] == "build")
                .map(|d| {
                    (
                        d["name"].as_str().expect("a dependency name").to_string(),
                        d["req"].as_str().expect("a requirement").to_string(),
                    )
                })
                .collect();
            let manifest = PathBuf::from(p["manifest_path"].as_str().expect("manifest_path"));
            let member = Member {
                version: p["version"].as_str().expect("a version").to_string(),
                manifest_dir: manifest.parent().expect("a directory").to_path_buf(),
                published,
                deps,
                readme: text(&p["readme"]),
                repository: text(&p["repository"]),
                homepage: text(&p["homepage"]),
                description: text(&p["description"]).unwrap_or_default(),
            };
            (name, member)
        })
        .collect()
}

fn published(members: &BTreeMap<String, Member>) -> BTreeSet<&str> {
    members
        .iter()
        .filter(|(_, m)| m.published)
        .map(|(name, _)| name.as_str())
        .collect()
}

/// The `crates=( … )` list in `.github/scripts/publish-crates.sh`, one
/// crate per line.
fn publish_list() -> Vec<String> {
    let path = root()
        .join(".github")
        .join("scripts")
        .join("publish-crates.sh");
    let script = read(&path);
    let mut lines = script.lines().map(str::trim);
    assert!(
        lines.any(|line| line == "crates=("),
        "{}: no `crates=(` line",
        path.display()
    );
    let list: Vec<String> = lines
        .take_while(|line| *line != ")")
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .map(str::to_string)
        .collect();
    assert!(!list.is_empty(), "{}: an empty list", path.display());
    list
}

#[test]
fn exactly_the_four_crates_nova_does_not_need_are_unpublished() {
    let members = members();
    let unpublished: BTreeSet<&str> = members
        .iter()
        .filter(|(_, m)| !m.published)
        .map(|(name, _)| name.as_str())
        .collect();
    assert_eq!(
        unpublished,
        UNPUBLISHED.into_iter().collect::<BTreeSet<_>>()
    );
    assert_eq!(published(&members).len(), 19);
}

#[test]
fn every_member_has_the_workspace_version() {
    let version = workspace_version();
    for (name, member) in members() {
        assert_eq!(member.version, version, "{name}'s version");
    }
}

#[test]
fn the_workspace_pins_each_published_crate_exactly() {
    let version = workspace_version();
    let manifest: toml_edit::DocumentMut = read(&root().join("Cargo.toml"))
        .parse()
        .expect("the root Cargo.toml parses");
    let table = manifest
        .get("workspace")
        .and_then(|w| w.get("dependencies"))
        .and_then(|d| d.as_table_like())
        .expect("[workspace.dependencies]");
    let members = members();
    let mut pinned = BTreeSet::new();
    for (name, item) in table.iter() {
        let Some(member) = members.get(name) else {
            continue;
        };
        pinned.insert(name);
        let pin = item.get("version").and_then(|v| v.as_str());
        assert_eq!(pin, Some(format!("={version}").as_str()), "{name}'s pin");
        let path = item
            .get("path")
            .and_then(|v| v.as_str())
            .unwrap_or_else(|| panic!("{name}: no path"));
        let dir = root().join(path);
        assert_eq!(
            dir.canonicalize().expect("the pin's path exists"),
            member
                .manifest_dir
                .canonicalize()
                .expect("the member's directory"),
            "{name}'s path"
        );
    }
    assert_eq!(pinned, published(&members));
}

#[test]
fn every_dependency_between_members_is_an_exact_pin_on_the_workspace_version() {
    let version = workspace_version();
    let members = members();
    let published = published(&members);
    for (name, member) in &members {
        for (dep, req) in &member.deps {
            // A path dependency with no version reads `*`.
            assert_eq!(req, &format!("={version}"), "{name}'s dependency on {dep}");
            if member.published {
                assert!(
                    published.contains(dep.as_str()),
                    "{name} is published but depends on {dep}, which is not"
                );
            }
        }
    }
}

#[test]
fn the_publish_list_is_the_published_crates_in_dependency_order() {
    let members = members();
    let list = publish_list();
    let listed: BTreeSet<&str> = list.iter().map(String::as_str).collect();
    assert_eq!(
        listed.len(),
        list.len(),
        "a crate is listed twice: {list:?}"
    );
    assert_eq!(listed, published(&members));
    for (position, name) in list.iter().enumerate() {
        for (dep, _) in &members[name].deps {
            let before = list[..position].contains(dep);
            assert!(before, "{name} is listed before its dependency {dep}");
        }
    }
}

#[test]
fn every_published_crate_carries_a_readme_both_licences_and_its_links() {
    let members = members();
    let mit = std::fs::read(root().join("LICENSE-MIT")).expect("the root's LICENSE-MIT");
    let apache = std::fs::read(root().join("LICENSE-APACHE")).expect("the root's LICENSE-APACHE");
    for (name, member) in members.iter().filter(|(_, m)| m.published) {
        let readme = member
            .readme
            .as_deref()
            .unwrap_or_else(|| panic!("{name} sets no readme"));
        let readme = member.manifest_dir.join(readme);
        assert!(
            readme.is_file(),
            "{name}: {} is not a file",
            readme.display()
        );
        for (file, root_text) in [("LICENSE-MIT", &mit), ("LICENSE-APACHE", &apache)] {
            let path = member.manifest_dir.join(file);
            let copy = std::fs::read(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
            assert!(
                &copy == root_text,
                "{} differs from the root's",
                path.display()
            );
        }
        assert_eq!(member.repository.as_deref(), Some(REPOSITORY), "{name}");
        assert_eq!(member.homepage.as_deref(), Some(REPOSITORY), "{name}");
        assert!(
            member.description.contains("Nova"),
            "{name}'s description does not name Nova: {:?}",
            member.description
        );
        if name != "nova-cli" {
            assert!(
                member.description.contains("internal"),
                "{name}'s description does not say it is internal: {:?}",
                member.description
            );
        }
    }
}

#[test]
fn the_extension_has_the_workspace_version() {
    let version = workspace_version();
    let extension = root().join("tools").join("vscode-nova");
    let json = |file: &str| -> Value {
        let path = extension.join(file);
        serde_json::from_str(&read(&path)).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
    };
    let package = json("package.json");
    assert_eq!(package["version"], version.as_str(), "package.json");
    let lock = json("package-lock.json");
    assert_eq!(lock["version"], version.as_str(), "package-lock.json");
    assert_eq!(
        lock["packages"][""]["version"],
        version.as_str(),
        "package-lock.json's packages[\"\"]"
    );
}
