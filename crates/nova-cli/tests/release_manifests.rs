//! The workspace's manifests are ready for crates.io (spec
//! `docs/superpowers/specs/2026-10-12-phase-3-6a-release-preparation-design.md`
//! §3, §11.1; and spec
//! `docs/superpowers/specs/2026-10-14-phase-4-0-foundations-design.md` §8):
//! the 20 crates `nova` needs are published, the other three are not,
//! every dependency between workspace crates is an exact pin on the
//! workspace's version, `publish-crates.sh` uploads them in dependency
//! order, each tarball carries a README and both licence texts, and the
//! VS Code extension has the same version, and its lock the same name.
//! And the Release gate passes `--node` to a `gate.sh` that takes it (spec
//! `docs/superpowers/specs/2026-10-22-phase-4-8-the-gate-and-v0.4.0-design.md`
//! §8).

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::process::Command;

use serde_json::Value;

/// The members that `publish = false` keeps off crates.io (spec §3.1).
/// `nova-codegen-wasm` left this list with 4.0, which `nova-driver`'s
/// WASM builds need (spec 4.0 §8).
const UNPUBLISHED: [&str; 3] = ["nova-bench-http", "nova-bundler", "nova-test"];

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
fn exactly_the_three_crates_nova_does_not_need_are_unpublished() {
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
    assert_eq!(published(&members).len(), 20);
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
fn the_extension_has_the_workspace_version_and_its_name() {
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
    let name = &package["name"];
    assert_eq!(&lock["name"], name, "package-lock.json's name");
    assert_eq!(
        &lock["packages"][""]["name"], name,
        "package-lock.json's packages[\"\"] name"
    );
}

/// The first line of `step`, a step of the Release gate, that `wanted`
/// accepts; `what` names it in the panic when there is none.
fn line_of(step: &[&str], what: &str, wanted: impl Fn(&str) -> bool) -> usize {
    step.iter()
        .position(|line| wanted(line))
        .unwrap_or_else(|| panic!("release-gate.yml: a step with no {what}: {step:?}"))
}

/// The Release gate runs the `gate.sh` of the tag it checks, and its two
/// steps that run it pass `--node` only when that `gate.sh` takes it: each
/// greps the script for a fixed string, its `--node)` line, prints the
/// flags and passes them. A `gate.sh` whose line drifted from that string
/// would be run without `--node`, and would pass with its Node run skipped,
/// so the string must be in `gate.sh`, and both steps must look for it and
/// pass what they found.
#[test]
fn the_release_gate_passes_node_to_a_gate_sh_that_takes_it() {
    let workflow_path = root()
        .join(".github")
        .join("workflows")
        .join("release-gate.yml");
    let workflow = read(&workflow_path);
    let gate_path = root().join(".github").join("scripts").join("gate.sh");
    let gate = read(&gate_path);

    // A step begins at a `- ` six spaces in and holds the lines indented
    // further; its lines are kept trimmed.
    let mut steps: Vec<Vec<&str>> = Vec::new();
    let mut step: Option<Vec<&str>> = None;
    for line in workflow.lines() {
        if let Some(first) = line.strip_prefix("      - ") {
            steps.extend(step.replace(vec![first.trim()]));
        } else if line.starts_with("        ") {
            if let Some(step) = step.as_mut() {
                step.push(line.trim());
            }
        } else if !line.trim().is_empty() {
            steps.extend(step.take());
        }
    }
    steps.extend(step);
    let gating: Vec<&Vec<&str>> = steps
        .iter()
        .filter(|step| {
            step.iter()
                .any(|line| line.starts_with("bash .github/scripts/gate.sh "))
        })
        .collect();
    assert_eq!(
        gating.len(),
        2,
        "{}: the steps that run gate.sh",
        workflow_path.display()
    );

    let grep = "if grep -qF -- '";
    let found = ".github/scripts/gate.sh; then flags+=(--node); fi";
    let mut strings = BTreeSet::new();
    for step in gating {
        let start = line_of(step, "`flags=(--test)`", |line| line == "flags=(--test)");
        let test = line_of(step, "grep of gate.sh", |line| {
            line.starts_with(grep) && line.ends_with(found)
        });
        let echo = line_of(step, "echo of the flags", |line| {
            line == r#"echo "gate.sh ${flags[*]}""#
        });
        let call = line_of(step, "run of gate.sh with the flags", |line| {
            line.starts_with("bash .github/scripts/gate.sh ") && line.ends_with(r#" "${flags[@]}""#)
        });
        assert!(
            start < test && test < echo && echo < call,
            "{}: a step out of order: {step:?}",
            workflow_path.display()
        );
        let string = step[test][grep.len()..]
            .split('\'')
            .next()
            .expect("the grep's string");
        assert!(
            gate.lines().any(|line| line.contains(string)),
            "{}: no line holds {string:?}, which release-gate.yml greps for",
            gate_path.display()
        );
        strings.insert(string);
    }
    assert_eq!(strings.len(), 1, "the two steps grep for {strings:?}");
}
