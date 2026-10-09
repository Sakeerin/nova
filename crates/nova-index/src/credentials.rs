//! The stored GitHub token (spec §6.7): `$NOVA_HOME/credentials.toml`,
//! `[github] token = "…"`, one token for any GitHub index.

use std::path::{Path, PathBuf};

use crate::download::write_private;

pub fn credentials_path(home: &Path) -> PathBuf {
    home.join("credentials.toml")
}

/// The token stored under `home`, if there is one.
pub fn load_token(home: &Path) -> Result<Option<String>, String> {
    let path = credentials_path(home);
    let text = match std::fs::read_to_string(&path) {
        Ok(text) => text,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(format!("cannot read {}: {error}", path.display())),
    };
    let document = toml_edit::ImDocument::parse(text.as_str())
        .map_err(|_| format!("{} cannot be read; run `nova login` again", path.display()))?;
    Ok(document
        .get("github")
        .and_then(|github| github.get("token"))
        .and_then(toml_edit::Item::as_str)
        .map(str::to_string))
}

/// Store `token` under `home`, owner-only on Unix (spec §6.7). A token is
/// letters, digits and `_`, as GitHub's are; anything else is refused,
/// unprinted.
pub fn store_token(home: &Path, token: &str) -> Result<PathBuf, String> {
    if token.is_empty() || !token.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
        return Err("that is not a GitHub token".to_string());
    }
    let path = credentials_path(home);
    write_private(&path, format!("[github]\ntoken = \"{token}\"\n").as_bytes())
        .map_err(|e| format!("cannot write {}: {e}", path.display()))?;
    Ok(path)
}
