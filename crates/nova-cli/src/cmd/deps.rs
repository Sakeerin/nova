//! `nova add <name> --path <dir> [--dev]` and `nova remove <name> [--dev]`:
//! edit the project's `nova.toml` (spec
//! `docs/superpowers/specs/2026-10-08-phase-3-3a-local-packages-design.md`
//! §5.3), keeping its comments, order and layout through `toml_edit`.

use std::path::{Component, Path, PathBuf};

use anyhow::{anyhow, bail, Context, Result};
use clap::Args;
use nova_diagnostics::{render, FileDb, Severity};
use toml_edit::{DocumentMut, InlineTable, Item, Value};

#[derive(Args)]
pub struct AddCmd {
    /// The dependency's package name.
    name: String,
    /// The directory holding its nova.toml, from the current directory.
    /// Registry dependencies arrive with the package index.
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
    nova_pm::check_name(&cmd.name).map_err(|message| anyhow!(message))?;
    let Some(path) = &cmd.path else {
        bail!(
            "registry dependencies arrive with the package index; use `--path <dir>` for a \
             local package"
        );
    };
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
        if entries.iter().any(|entry| entry.name == cmd.name) {
            bail!("`{}` is already in [{table}]", cmd.name);
        }
    }

    // Relative to the manifest's directory, whatever the current one.
    let cwd = std::env::current_dir().context("reading the current directory")?;
    let written = relative(
        &nova_pm::real_path(&root),
        &nova_pm::real_path(&cwd.join(path)),
    );
    let mut document = document(&text)?;
    let table = table_name(cmd.dev);
    if document.get(table).is_none() {
        document.insert(table, toml_edit::table());
    }
    let entries = document
        .get_mut(table)
        .and_then(Item::as_table_like_mut)
        .with_context(|| format!("[{table}] in nova.toml is not a table"))?;
    let mut entry = InlineTable::new();
    entry.insert("path", Value::from(written.as_str()));
    entries.insert(&cmd.name, Item::Value(Value::InlineTable(entry)));
    let new_text = to_text(&document, &text);

    // The graph as it would be: any error refuses, and nothing is written.
    // M0013, a package with no source yet, is not the entry's fault (spec
    // §3.1).
    let mut db = FileDb::new();
    let (_, diagnostics) = nova_pm::graph_from(&root, Some(&new_text), &mut db);
    if diagnostics
        .iter()
        .any(|d| d.severity == Severity::Error && d.code != "M0013")
    {
        render::emit_all(&db, &diagnostics);
        bail!("`{}` was not added; nova.toml was left unchanged", cmd.name);
    }
    write(&root, &new_text)?;
    println!(
        "added {} = {{ path = \"{written}\" }} to [{table}]",
        cmd.name
    );
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
