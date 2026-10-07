//! The rules for a package's name (spec
//! `docs/superpowers/specs/2026-10-07-phase-3-0-foundations-design.md`
//! §5.1).

/// Names Windows reserves for devices. A package name becomes a directory
/// and an executable, so it may be none of these, in any case.
const WINDOWS_DEVICES: [&str; 22] = [
    "con", "prn", "aux", "nul", "com1", "com2", "com3", "com4", "com5", "com6", "com7", "com8",
    "com9", "lpt1", "lpt2", "lpt3", "lpt4", "lpt5", "lpt6", "lpt7", "lpt8", "lpt9",
];

/// Check a package name: ASCII letters, digits, `-` and `_`, starting with
/// a letter, at most 64 characters, and not a Windows device name. The
/// error says which rule the name breaks.
pub fn check_name(name: &str) -> Result<(), String> {
    let Some(first) = name.chars().next() else {
        return Err("a package name cannot be empty".to_string());
    };
    if !first.is_ascii_alphabetic() {
        return Err(format!(
            "package name `{name}` must start with an ASCII letter"
        ));
    }
    if let Some(bad) = name
        .chars()
        .find(|c| !(c.is_ascii_alphanumeric() || *c == '-' || *c == '_'))
    {
        return Err(format!(
            "package name `{name}` contains {bad:?}; a name may use only ASCII letters, \
             digits, `-` and `_`"
        ));
    }
    if name.len() > 64 {
        return Err(format!(
            "package name `{name}` is {} characters long; the limit is 64",
            name.len()
        ));
    }
    if WINDOWS_DEVICES.contains(&name.to_ascii_lowercase().as_str()) {
        return Err(format!("`{name}` is a name Windows reserves for a device"));
    }
    Ok(())
}
