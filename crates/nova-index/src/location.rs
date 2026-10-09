//! Which index, and where its tarballs are (spec §3.2, §3.3).

use std::path::PathBuf;

/// The index nova uses unless `NOVA_INDEX` names another (spec §3.3).
pub const DEFAULT_INDEX: &str = "https://raw.githubusercontent.com/Sakeerin/nova-index/main/";

/// Where an index's files are read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Source {
    /// Over HTTP: the canonical URL, ending in `/`.
    Http(String),
    /// A directory on this machine.
    Local(PathBuf),
}

/// Where a tarball is.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Location {
    Url(String),
    File(PathBuf),
}

/// An index, known by its canonical form (spec §3.3).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Index {
    pub canonical: String,
    pub source: Source,
}

impl Index {
    /// The index at `location`: an `https://` URL (`http://` on a loopback
    /// IP only), a `file:` URL, or an absolute path.
    pub fn new(location: &str) -> Result<Index, String> {
        let canonical = nova_pm::canonical_index(location)?;
        let source = match nova_pm::local_index_path(&canonical) {
            Some(dir) => Source::Local(dir),
            None => Source::Http(canonical.clone()),
        };
        Ok(Index { canonical, source })
    }

    /// The index a `NOVA_INDEX` of `setting` names: the default when it is
    /// unset or empty.
    pub fn from_setting(setting: Option<&str>) -> Result<Index, String> {
        match setting.map(str::trim).filter(|s| !s.is_empty()) {
            Some(location) => Index::new(location).map_err(|e| format!("NOVA_INDEX: {e}")),
            None => Index::new(DEFAULT_INDEX),
        }
    }

    /// The index this process's `NOVA_INDEX` names.
    pub fn from_env() -> Result<Index, String> {
        let setting = std::env::var("NOVA_INDEX").ok();
        Index::from_setting(setting.as_deref())
    }

    /// Where `name` `version`'s tarball is, from `config.json`'s `dl`
    /// (spec §3.2): an absolute URL, or a path under the index's root made
    /// only of plain `/`-separated names.
    pub fn tarball(&self, dl: &str, name: &str, version: &str) -> Result<Location, String> {
        let filled = fill_dl(dl, name, version);
        let lower = filled.to_ascii_lowercase();
        if lower.starts_with("https://") || lower.starts_with("http://") {
            return Ok(Location::Url(filled));
        }
        let parts: Vec<&str> = filled.split('/').collect();
        let plain = |part: &&str| {
            !part.is_empty() && *part != "." && *part != ".." && !part.contains([':', '\\'])
        };
        if !parts.iter().all(plain) {
            return Err(format!(
                "config.json's `dl` gives `{filled}`, which is neither a URL nor a path under \
                 the index"
            ));
        }
        Ok(match &self.source {
            Source::Http(base) => Location::Url(format!("{base}{filled}")),
            Source::Local(dir) => {
                Location::File(parts.iter().fold(dir.clone(), |path, part| path.join(part)))
            }
        })
    }
}

/// `dl` with `{name}` and `{version}` filled in.
pub fn fill_dl(dl: &str, name: &str, version: &str) -> String {
    dl.replace("{name}", name).replace("{version}", version)
}

/// The path of `name`'s file in an index (spec §3.1): Cargo's sparse
/// layout, in lower case. Package names are ASCII.
pub fn index_path(name: &str) -> String {
    let name = name.to_ascii_lowercase();
    match name.len() {
        0 | 1 => format!("1/{name}"),
        2 => format!("2/{name}"),
        3 => format!("3/{}/{name}", &name[..1]),
        _ => format!("{}/{}/{name}", &name[..2], &name[2..4]),
    }
}
