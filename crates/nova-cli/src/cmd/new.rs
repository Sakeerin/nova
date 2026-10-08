//! `nova new <name>` and `nova init [--name <name>]`, each with `--lib` for a
//! library: write a new project (spec
//! `docs/superpowers/specs/2026-10-07-phase-3-0-foundations-design.md` §6.3,
//! and 3.3a's §5.5). Neither runs `git init`.

use std::fs;
use std::io::{self, Write};
use std::path::Path;

use anyhow::{anyhow, bail, Context, Result};
use clap::Args;

use crate::template::Kind;

#[derive(Args)]
pub struct NewCmd {
    /// The project's name, which is also the directory `nova new` creates.
    name: String,
    /// Make a library: src/lib.nova and tests/, in place of src/main.nova.
    #[arg(long)]
    lib: bool,
}

#[derive(Args)]
pub struct InitCmd {
    /// The project's name (default: the current directory's name).
    #[arg(long)]
    name: Option<String>,
    /// Make a library: src/lib.nova and tests/, in place of src/main.nova.
    #[arg(long)]
    lib: bool,
}

/// `nova new <name>`: the template, in a new directory called `<name>`.
pub fn new(cmd: NewCmd) -> Result<()> {
    nova_pm::check_name(&cmd.name).map_err(|message| anyhow!(message))?;
    let kind = kind(&cmd.name, cmd.lib)?;
    let dir = Path::new(&cmd.name);
    if dir.exists() && !is_empty_dir(dir)? {
        bail!(
            "`{}` already exists and is not an empty directory",
            cmd.name
        );
    }
    for sub in dirs(kind) {
        fs::create_dir_all(dir.join(sub)).with_context(|| format!("creating {}", dir.display()))?;
    }
    let mut wrote = Vec::new();
    for (path, text) in crate::template::files(&cmd.name, kind) {
        if write_new(&dir.join(&path), &text)? {
            wrote.push(path);
        }
    }
    println!("created `{}`: {}", cmd.name, wrote.join(", "));
    Ok(())
}

/// `nova init`: the template, into the current directory. It writes only
/// the files that are missing, and never overwrites one.
pub fn init(cmd: InitCmd) -> Result<()> {
    let cwd = std::env::current_dir().context("reading the current directory")?;
    if cwd.join(nova_pm::MANIFEST).exists() {
        bail!("this directory already has a nova.toml");
    }
    let name = match cmd.name {
        Some(name) => {
            nova_pm::check_name(&name).map_err(|message| anyhow!(message))?;
            name
        }
        None => {
            let name = cwd
                .file_name()
                .map(|name| name.to_string_lossy().into_owned())
                .unwrap_or_default();
            nova_pm::check_name(&name).map_err(|message| {
                anyhow!("{message}; name the project with `nova init --name <name>`")
            })?;
            name
        }
    };
    let kind = kind(&name, cmd.lib)?;
    for sub in dirs(kind) {
        fs::create_dir_all(cwd.join(sub)).with_context(|| format!("creating {sub}"))?;
    }
    let mut wrote = Vec::new();
    let mut kept = Vec::new();
    for (path, text) in crate::template::files(&name, kind) {
        if write_new(&cwd.join(&path), &text)? {
            wrote.push(path);
        } else {
            kept.push(path);
        }
    }
    let wrote = if wrote.is_empty() {
        "nothing".to_string()
    } else {
        wrote.join(", ")
    };
    println!("wrote: {wrote}");
    if !kept.is_empty() {
        println!("kept: {}", kept.join(", "));
    }
    Ok(())
}

fn is_empty_dir(dir: &Path) -> Result<bool> {
    if !dir.is_dir() {
        return Ok(false);
    }
    let mut entries = fs::read_dir(dir).with_context(|| format!("reading {}", dir.display()))?;
    Ok(entries.next().is_none())
}

/// The template `lib` asks for. A library's import name must not be a
/// keyword (spec 3.3a §3.4), so it is refused before anything is written.
fn kind(name: &str, lib: bool) -> Result<Kind> {
    if !lib {
        return Ok(Kind::Program);
    }
    let import = nova_pm::import_name(name);
    if nova_lexer::KEYWORDS.contains(&import.as_str()) {
        bail!(
            "a library called `{name}` would be imported as `{import}`, which is a keyword; \
             choose another name"
        );
    }
    Ok(Kind::Library)
}

/// The directories the template of `kind` writes into.
fn dirs(kind: Kind) -> &'static [&'static str] {
    match kind {
        Kind::Program => &["src"],
        Kind::Library => &["src", "tests"],
    }
}

/// Create `path` holding `text`, unless it already exists. Returns whether
/// it wrote the file.
fn write_new(path: &Path, text: &str) -> Result<bool> {
    match fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
    {
        Ok(mut file) => {
            file.write_all(text.as_bytes())
                .with_context(|| format!("writing {}", path.display()))?;
            Ok(true)
        }
        Err(error) if error.kind() == io::ErrorKind::AlreadyExists => Ok(false),
        Err(error) => Err(error).with_context(|| format!("creating {}", path.display())),
    }
}
