//! `nova login` (spec
//! `docs/superpowers/specs/2026-10-09-phase-3-3b-index-and-publishing-design.md`
//! §6.7): store a GitHub token for publishing, read from a pipe.

use std::io::{BufRead, IsTerminal};
use std::path::Path;

use anyhow::{anyhow, bail, Context, Result};
use nova_index::{GitHub, Index};

pub fn login() -> Result<()> {
    let stdin = std::io::stdin();
    if stdin.is_terminal() {
        bail!("pipe the token in, e.g. `gh auth token | nova login`");
    }
    let mut line = String::new();
    stdin
        .lock()
        .read_line(&mut line)
        .context("reading the token from standard input")?;
    let token = line.trim();
    if token.is_empty() {
        bail!("no token on standard input; pipe it in, e.g. `gh auth token | nova login`");
    }
    let home = nova_pm::nova_home_from_env()
        .context("cannot place the credentials: set NOVA_HOME to a writable directory")?;
    let index = Index::from_env().map_err(|e| anyhow!(e))?;
    let config = nova_index::reader_for(&index)
        .config()
        .map_err(|e| anyhow!("cannot read the index at {}: {e}", index.canonical))?;
    let Some(repo) = config.api else {
        bail!(
            "the index at {} has no `api` in its config.json; it is not a GitHub index, and \
             needs no login",
            index.canonical
        );
    };
    let github = GitHub::from_env(&repo, token).map_err(|e| anyhow!(e))?;
    if !github.can_push().map_err(|e| anyhow!(e))? {
        bail!("the token cannot push to {repo}");
    }
    let path = nova_index::store_token(&home, token).map_err(|e| anyhow!(e))?;
    if cfg!(windows) && !inside_profile(&home) {
        eprintln!(
            "warning: {} is outside your user profile, so others may be able to read the \
             token stored in it",
            home.display()
        );
    }
    println!("logged in to {repo}; the token is in {}", path.display());
    Ok(())
}

/// Whether `home` is inside the user's profile, whose permissions guard a
/// file on Windows (spec §6.7).
fn inside_profile(home: &Path) -> bool {
    let Some(profile) = std::env::var_os("USERPROFILE") else {
        return false;
    };
    nova_pm::real_path(home).starts_with(nova_pm::real_path(Path::new(&profile)))
}
