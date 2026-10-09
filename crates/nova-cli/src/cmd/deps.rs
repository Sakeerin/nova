//! `nova add <name>[@<req>] [--path <dir>] [--dev]` and `nova remove <name>
//! [--dev]`: edit the project's `nova.toml` (spec
//! `docs/superpowers/specs/2026-10-08-phase-3-3a-local-packages-design.md`
//! §5.3), keeping its comments, order and layout through `toml_edit`. The
//! registry form is spec
//! `docs/superpowers/specs/2026-10-09-phase-3-3b-index-and-publishing-design.md`
//! §6.1.

use std::path::{Component, Path, PathBuf};

use anyhow::{anyhow, bail, Context, Result};
use clap::Args;
use nova_diagnostics::{render, FileDb, Severity};
use nova_index::{Index, SyncRequest};
use nova_pm::{Offline, Unlock};
use toml_edit::{DocumentMut, InlineTable, Item, Value};

#[derive(Args)]
pub struct AddCmd {
    /// The dependency: a package name, or `name@requirement` for a package
    /// from the index, such as `json@1.4`.
    name: String,
    /// A local package instead: the directory holding its nova.toml, from
    /// the current directory.
    #[arg(long)]
    path: Option<PathBuf>,
    /// Add it to [dev-dependencies], which only tests/ files import.
    #[arg(long)]
    dev: bool,
}

#[derive(Args)]
pub struct RemoveCmd {
    /// The dependency's package name.
    name: String,
    /// Remove it from [dev-dependencies].
    #[arg(long)]
    dev: bool,
}

fn table_name(dev: bool) -> &'static str {
    if dev {
        "dev-dependencies"
    } else {
        "dependencies"
    }
}

/// The project's directory, and its `nova.toml`'s text.
fn manifest() -> Result<(PathBuf, String)> {
    let cwd = std::env::current_dir().context("reading the current directory")?;
    let root = nova_pm::find_root(&cwd).context("no nova.toml here or in any directory above")?;
    let path = root.join(nova_pm::MANIFEST);
    let text =
        std::fs::read_to_string(&path).with_context(|| format!("reading {}", path.display()))?;
    Ok((root, text))
}

/// `text`, parsed for editing.
fn document(text: &str) -> Result<DocumentMut> {
    text.parse::<DocumentMut>()
        .map_err(|error| anyhow!("nova.toml is not valid TOML: {error}"))
}

/// `document` as text, in `original`'s line endings.
fn to_text(document: &DocumentMut, original: &str) -> String {
    let text = document.to_string();
    if original.contains("\r\n") {
        text.replace("\r\n", "\n").replace('\n', "\r\n")
    } else {
        text
    }
}

/// Write `text` as the project's `nova.toml`.
fn write(root: &Path, text: &str) -> Result<()> {
    let path = root.join(nova_pm::MANIFEST);
    std::fs::write(&path, text).with_context(|| format!("writing {}", path.display()))
}

pub fn add(cmd: AddCmd) -> Result<()> {
    let (name, requirement) = match cmd.name.split_once('@') {
        Some((name, requirement)) => (name, Some(requirement)),
        None => (cmd.name.as_str(), None),
    };
    nova_pm::check_name(name).map_err(|message| anyhow!(message))?;
    if cmd.path.is_some() && requirement.is_some() {
        bail!(
            "`{}`: an entry has a version or a path, never both",
            cmd.name
        );
    }
    if let Some(requirement) = requirement {
        semver::VersionReq::parse(requirement)
            .map_err(|e| anyhow!("`{requirement}` is not a version requirement: {e}"))?;
    }
    let (root, text) = manifest()?;
    // A manifest with errors is refused, with its errors shown.
    let mut db = FileDb::new();
    let file = db.add(
        root.join(nova_pm::MANIFEST).display().to_string(),
        text.as_str(),
    );
    let (parsed, diagnostics) = nova_pm::parse(&text, file);
    let Some(parsed) =
        parsed.filter(|_| !diagnostics.iter().any(|d| d.severity == Severity::Error))
    else {
        render::emit_all(&db, &diagnostics);
        bail!("nova.toml has errors, so it was left unchanged");
    };
    for (table, entries) in [
        ("dependencies", &parsed.dependencies),
        ("dev-dependencies", &parsed.dev_dependencies),
    ] {
        if entries.iter().any(|entry| entry.name == name) {
            bail!("`{name}` is already in [{table}]");
        }
    }
    let table = table_name(cmd.dev);
    match &cmd.path {
        Some(path) => add_path(&root, &text, name, path, table),
        None => add_registry(&root, &text, name, requirement, table),
    }
}

/// `text` with `name = value` in `[table]`, in `text`'s line endings.
fn with_entry(text: &str, table: &str, name: &str, value: Value) -> Result<String> {
    let mut document = document(text)?;
    if document.get(table).is_none() {
        document.insert(table, toml_edit::table());
    }
    let entries = document
        .get_mut(table)
        .and_then(Item::as_table_like_mut)
        .with_context(|| format!("[{table}] in nova.toml is not a table"))?;
    entries.insert(name, Item::Value(value));
    Ok(to_text(&document, text))
}

/// `nova add <name> --path <dir>` (spec 3.3a §5.3).
fn add_path(root: &Path, text: &str, name: &str, path: &Path, table: &str) -> Result<()> {
    // Relative to the manifest's directory, whatever the current one.
    let cwd = std::env::current_dir().context("reading the current directory")?;
    let written = relative(
        &nova_pm::real_path(root),
        &nova_pm::real_path(&cwd.join(path)),
    );
    let mut entry = InlineTable::new();
    entry.insert("path", Value::from(written.as_str()));
    let new_text = with_entry(text, table, name, Value::InlineTable(entry))?;
    // The graph as it would be: any error refuses, and nothing is written.
    // M0013, a package with no source yet, and M0005, a registry entry not
    // yet downloaded, are not the new entry's fault (spec 3.3a §3.1, 3.3b
    // §6.1).
    let mut db = FileDb::new();
    let (_, diagnostics) = nova_pm::graph_from(root, Some(&new_text), &mut db);
    if diagnostics
        .iter()
        .any(|d| d.severity == Severity::Error && d.code != "M0013" && d.code != "M0005")
    {
        render::emit_all(&db, &diagnostics);
        bail!("`{name}` was not added; nova.toml was left unchanged");
    }
    write(root, &new_text)?;
    println!("added {name} = {{ path = \"{written}\" }} to [{table}]");
    Ok(())
}

/// `nova add <name>[@<req>]` (spec 3.3b §6.1). The manifest it would write
/// is synced in memory, with `"*"` when no requirement was given, so the
/// version fits the rest of the graph; the version chosen is then
/// written, as Cargo writes it. The graph is checked with the lock it
/// would write, and only then are `nova.toml` and `nova.lock` written.
fn add_registry(
    root: &Path,
    text: &str,
    name: &str,
    requirement: Option<&str>,
    table: &str,
) -> Result<()> {
    let refused = || format!("`{name}` was not added; nova.toml and nova.lock were left unchanged");
    crate::project::refuse_cached(root)?;
    let proposed = with_entry(text, table, name, Value::from(requirement.unwrap_or("*")))?;
    let index = Index::from_env().map_err(|e| anyhow!(e))?;
    let mut reader = nova_index::reader_for(&index);
    let registry = nova_pm::registry_dir();
    let mut db = FileDb::new();
    let result = nova_index::sync(
        SyncRequest {
            root,
            manifest: Some(&proposed),
            dev: true,
            unlock: Unlock::Nothing,
            write: false,
            index: &index,
            reader: reader.as_mut(),
            registry: registry.as_deref(),
        },
        &mut db,
    );
    let synced = crate::project::finish(result, &db).with_context(refused)?;
    let lock = synced.lock.with_context(refused)?;
    let written = match requirement {
        Some(requirement) => requirement.to_string(),
        None => lock
            .find(name)
            .map(|package| package.version.to_string())
            .with_context(refused)?,
    };
    let new_text = with_entry(text, table, name, Value::from(written.as_str()))?;
    let offline = Offline {
        registry,
        lock: Some(lock.clone()),
        dev: true,
    };
    let mut db = FileDb::new();
    let (_, diagnostics) = nova_pm::graph_with(root, Some(&new_text), &offline, &mut db);
    if diagnostics
        .iter()
        .any(|d| d.severity == Severity::Error && d.code != "M0013")
    {
        render::emit_all(&db, &diagnostics);
        bail!(refused());
    }
    write(root, &new_text)?;
    nova_index::write_lock(root, &lock).map_err(|e| anyhow!(e))?;
    println!("added {name} = \"{written}\" to [{table}]");
    Ok(())
}

pub fn remove(cmd: RemoveCmd) -> Result<()> {
    let (root, text) = manifest()?;
    let mut document = document(&text)?;
    let table = table_name(cmd.dev);
    // The comment lines directly above an entry are its decor, and go with
    // it.
    let removed = document
        .get_mut(table)
        .and_then(Item::as_table_like_mut)
        .and_then(|entries| entries.remove(&cmd.name));
    if removed.is_none() {
        bail!("`{}` is not in [{table}]", cmd.name);
    }
    write(&root, &to_text(&document, &text))?;
    println!("removed {} from [{table}]", cmd.name);
    Ok(())
}

/// `target` relative to `base`, both canonical, with `/` separators. When
/// they share no root, as on two Windows drives, `target` itself, with `/`
/// separators (spec §5.3).
fn relative(base: &Path, target: &Path) -> String {
    let from: Vec<Component> = base.components().collect();
    let to: Vec<Component> = target.components().collect();
    if from.first() != to.first() {
        return target.to_string_lossy().replace('\\', "/");
    }
    let common = from.iter().zip(&to).take_while(|(a, b)| a == b).count();
    let mut parts = vec!["..".to_string(); from.len() - common];
    parts.extend(
        to[common..]
            .iter()
            .map(|part| part.as_os_str().to_string_lossy().into_owned()),
    );
    if parts.is_empty() {
        ".".to_string()
    } else {
        parts.join("/")
    }
}
