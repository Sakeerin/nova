//! Where nova keeps its caches (spec
//! `docs/superpowers/specs/2026-10-09-phase-3-3b-index-and-publishing-design.md`
//! §5.1).

use std::path::PathBuf;

/// `$NOVA_HOME`: the variable when it is set, else `.nova` in the home
/// directory, which is `USERPROFILE` on Windows and `HOME` elsewhere.
/// `None` when neither is set; an empty value counts as unset. It takes the
/// variables' values rather than reading them, so tests need not change
/// the process environment.
pub fn nova_home(
    nova_home: Option<PathBuf>,
    userprofile: Option<PathBuf>,
    home: Option<PathBuf>,
    windows: bool,
) -> Option<PathBuf> {
    let set = |value: Option<PathBuf>| value.filter(|path| !path.as_os_str().is_empty());
    if let Some(nova_home) = set(nova_home) {
        return Some(nova_home);
    }
    let home = if windows { set(userprofile) } else { set(home) };
    home.map(|home| home.join(".nova"))
}

/// [`nova_home`] from this process's environment.
pub fn nova_home_from_env() -> Option<PathBuf> {
    let var = |name: &str| std::env::var_os(name).map(PathBuf::from);
    nova_home(
        var("NOVA_HOME"),
        var("USERPROFILE"),
        var("HOME"),
        cfg!(windows),
    )
}

/// `$NOVA_HOME/registry`, where downloaded packages live (spec §5.1).
pub fn registry_dir() -> Option<PathBuf> {
    nova_home_from_env().map(|home| home.join("registry"))
}
