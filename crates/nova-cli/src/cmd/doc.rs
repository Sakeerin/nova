//! `nova doc [--open] [--std]`: the documentation site (spec
//! `docs/superpowers/specs/2026-10-11-phase-3-5a-the-documentation-site-design.md`
//! §4; ADR 0034).

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use anyhow::{anyhow, bail, Result};
use clap::Args;
use nova_diagnostics::{render, Severity};
use nova_doc::Failure;

use crate::project::{self, Mode};

#[derive(Args)]
pub struct DocCmd {
    /// Open the site's start page afterwards, with $BROWSER or the
    /// system's opener.
    #[arg(long)]
    open: bool,

    /// Document std alone, inside a project or not.
    #[arg(long)]
    std: bool,
}

pub fn run(cmd: DocCmd) -> Result<()> {
    let mode = project::mode(None)?;
    let documented = if cmd.std {
        nova_doc::document_std().map_err(failure)?
    } else {
        let Mode::Project { root, .. } = &mode else {
            bail!("nova doc documents a project; run it inside one, or use `nova doc --std`");
        };
        project::sync(&mode)?;
        nova_doc::document_package(root).map_err(failure)?
    };
    render::emit_all(&documented.db, &documented.warnings);
    let dir = match &mode {
        Mode::Project { target_dir, .. } => target_dir.join("doc"),
        Mode::File(_) => PathBuf::from("target").join("doc"),
    };
    let start = nova_doc::write_site(&documented.site, &dir)?;
    for broken in nova_doc::check_links(&dir) {
        eprintln!("warning: {}: broken link `{}`", broken.page, broken.link);
    }
    println!("{}", start.display());
    if cmd.open {
        open(&start);
    }
    Ok(())
}

/// A failure as the command's error, its diagnostics printed first (plan
/// decision 15).
fn failure(f: Failure) -> anyhow::Error {
    match f {
        Failure::Diagnostics { db, diagnostics } => {
            render::emit_all(&db, &diagnostics);
            let errors = diagnostics
                .iter()
                .filter(|d| d.severity == Severity::Error)
                .count();
            anyhow!(
                "could not document due to {errors} previous error{}",
                if errors == 1 { "" } else { "s" }
            )
        }
        Failure::Other(why) => anyhow!(why),
    }
}

/// Open `page` (spec §4.1; plan decision 16) with `$BROWSER` when it is
/// set, else with the system's opener. A failure is a warning.
fn open(page: &Path) {
    let page = if page.is_absolute() {
        page.to_path_buf()
    } else {
        std::env::current_dir().map_or_else(|_| page.to_path_buf(), |d| d.join(page))
    };
    let mut command = match std::env::var_os("BROWSER") {
        Some(browser) if !browser.is_empty() => Command::new(browser),
        _ => opener(),
    };
    command
        .arg(&page)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    match command.spawn() {
        // nova exits next and the browser outlives it; the thread only
        // reaps it if it closes first.
        Ok(mut child) => {
            std::thread::spawn(move || {
                let _ = child.wait();
            });
        }
        Err(e) => eprintln!(
            "warning: could not open a browser ({e}); the site is at {}",
            page.display()
        ),
    }
}

#[cfg(target_os = "macos")]
fn opener() -> Command {
    Command::new("open")
}

#[cfg(windows)]
fn opener() -> Command {
    let mut c = Command::new("cmd");
    c.args(["/C", "start", ""]);
    c
}

#[cfg(not(any(windows, target_os = "macos")))]
fn opener() -> Command {
    Command::new("xdg-open")
}
