//! `nova fetch` and `nova update [<name>]` (spec
//! `docs/superpowers/specs/2026-10-09-phase-3-3b-index-and-publishing-design.md`
//! §6.3, §6.4).

use std::path::PathBuf;

use anyhow::{anyhow, bail, Result};
use clap::Args;
use nova_diagnostics::FileDb;
use nova_pm::{Lock, Unlock, LOCKFILE};

use crate::project::{self, Mode};

#[derive(Args)]
pub struct UpdateCmd {
    /// Update only this package; every other locked version stays.
    name: Option<String>,
}

/// The project around the current directory, which these commands need.
fn project() -> Result<(Mode, PathBuf)> {
    let mode = project::mode(None)?;
    let Mode::Project { root, .. } = &mode else {
        bail!("no nova.toml here or in any directory above");
    };
    let root = root.clone();
    Ok((mode, root))
}

pub fn fetch() -> Result<()> {
    let (mode, _) = project()?;
    let synced = project::sync_with(&mode, Unlock::Nothing)?
        .ok_or_else(|| anyhow!("no nova.toml here or in any directory above"))?;
    match synced.unpacked.len() {
        0 => println!("nothing to fetch"),
        n => println!("fetched {n} package{}", if n == 1 { "" } else { "s" }),
    }
    Ok(())
}

pub fn update(cmd: UpdateCmd) -> Result<()> {
    let (mode, root) = project()?;
    let unlock = match cmd.name {
        None => Unlock::All,
        Some(name) => {
            // Plan decision 13. An unreadable lock is the sync's to report,
            // as M0016.
            match std::fs::read_to_string(root.join(LOCKFILE)) {
                Err(_) => bail!("`{name}` is not in nova.lock"),
                Ok(text) => {
                    let mut db = FileDb::new();
                    let file = db.add(LOCKFILE, text.as_str());
                    if let Ok(lock) = nova_pm::parse_lock(&text, file) {
                        if lock.find(&name).is_none() {
                            bail!("`{name}` is not in nova.lock");
                        }
                    }
                }
            }
            Unlock::One(name)
        }
    };
    let synced = project::sync_with(&mode, unlock)?
        .ok_or_else(|| anyhow!("no nova.toml here or in any directory above"))?;
    let changes = changes(synced.before.as_ref(), synced.lock.as_ref());
    if changes.is_empty() {
        println!("nothing to update");
    }
    for change in changes {
        println!("{change}");
    }
    Ok(())
}

/// What an update changed, by name: `json 1.2.0 -> 1.4.1`, `+ http 0.3.0`
/// for a new package, `- old 1.0.0` for one no longer needed.
fn changes(before: Option<&Lock>, after: Option<&Lock>) -> Vec<String> {
    let none = Vec::new();
    let old = before.map_or(&none, |lock| &lock.packages);
    let new = after.map_or(&none, |lock| &lock.packages);
    let mut lines = Vec::new();
    for package in new {
        match old.iter().find(|p| p.name == package.name) {
            Some(was) if was.version != package.version => lines.push(format!(
                "{} {} -> {}",
                package.name, was.version, package.version
            )),
            Some(_) => {}
            None => lines.push(format!("+ {} {}", package.name, package.version)),
        }
    }
    for package in old {
        if !new.iter().any(|p| p.name == package.name) {
            lines.push(format!("- {} {}", package.name, package.version));
        }
    }
    lines
}
