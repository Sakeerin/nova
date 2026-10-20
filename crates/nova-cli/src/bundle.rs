//! What `nova build --target wasm` writes beside `app.js` and `app.wasm`
//! (the 4.4 spec
//! `docs/superpowers/specs/2026-10-17-phase-4-4-component-syntax-design.md`
//! §7): a project's own page, `assets/` from its `public/`, `wasm-opt`
//! under `--release`, and `manifest.json`.

use std::collections::BTreeMap;
use std::path::{Component, Path, PathBuf};

use anyhow::{bail, Context, Result};
use nova_driver::Program;

use crate::project::{self, Mode};

/// The files the compiler writes into `dist/` (spec 4.0 §7).
const BUILT: [&str; 4] = ["index.html", "app.js", "app.wasm", "package.json"];

/// What [`Bundle::finish`] listed in `manifest.json`, to which `nova build
/// --ssg` adds the pages its server wrote (the 4.6 spec
/// `docs/superpowers/specs/2026-10-19-phase-4-6-ssr-hydration-and-ssg-design.md`
/// §7).
pub struct Listing {
    wasm_opt: bool,
    /// Each file by its path from `dist/`, with `/` between names, and its
    /// size in bytes.
    files: BTreeMap<String, u64>,
}

/// What a build's bundle needs to know of its program.
pub struct Bundle {
    /// The project's root, beside its `nova.toml`; `None` for a program
    /// built from a single file, which keeps the generated page and has no
    /// `assets/`.
    pub root: Option<PathBuf>,
    pub name: String,
    pub version: Option<String>,
    pub release: bool,
}

impl Bundle {
    /// `name` and `version` from `nova.toml`, or the file's stem and none.
    pub fn of(mode: &Mode, program: &Program, release: bool) -> Bundle {
        match mode {
            Mode::Project { root, .. } => Bundle {
                root: Some(root.clone()),
                name: project::package_name(program),
                version: program
                    .graph
                    .as_ref()
                    .map(|g| g.root().manifest.package.version.to_string()),
                release,
            },
            Mode::File(file) => Bundle {
                root: None,
                name: file
                    .file_stem()
                    .map_or_else(|| "out".to_string(), |s| s.to_string_lossy().into_owned()),
                version: None,
                release,
            },
        }
    }

    /// Refuse an output directory that the copies would harm: the project's
    /// root or a directory holding it, whose `index.html` the generated page
    /// would overwrite and whose `assets/` a build replaces, or one inside
    /// `public/`, which would copy itself into itself.
    pub fn check_output(&self, out: &Path) -> Result<()> {
        check_output_for(self.root.as_deref(), out)
    }

    /// Before the compiler writes `out`: a link at a name the build writes,
    /// symbolic or, on Unix, hard, is removed, so that no write goes through
    /// it to a file elsewhere, such as the project's own page.
    pub fn prepare(&self, out: &Path) -> Result<()> {
        for name in BUILT.iter().chain(&["manifest.json"]) {
            let path = out.join(name);
            let link = std::fs::symlink_metadata(&path)
                .is_ok_and(|m| m.file_type().is_symlink() || hard_linked(&m));
            if link {
                std::fs::remove_file(&path)
                    .or_else(|_| std::fs::remove_dir(&path))
                    .with_context(|| format!("removing {}", path.display()))?;
            }
        }
        Ok(())
    }

    /// After the compiler has written `out`: the page, `assets/`,
    /// `wasm-opt` and `manifest.json`, in that order, so the manifest lists
    /// what is there at the end. `assets/` is the build's only when it
    /// copies a `public/` into it: otherwise it is left as it is, and not
    /// listed. Returns what the manifest lists.
    pub fn finish(&self, out: &Path) -> Result<Listing> {
        let mut copied = false;
        if let Some(root) = &self.root {
            let page = root.join("index.html");
            let to = out.join("index.html");
            // Never onto itself: a copy would empty it first.
            if page.is_file() && resolved(&page) != resolved(&to) {
                std::fs::copy(&page, &to)
                    .with_context(|| format!("copying {} to {}", page.display(), to.display()))?;
            }
            let assets = out.join("assets");
            let public = root.join("public");
            if public.is_dir() {
                let base = resolved(&public);
                if base.starts_with(resolved(root)) {
                    if std::fs::symlink_metadata(&assets).is_ok() {
                        remove(&assets)?;
                    }
                    copy_tree(&public, &assets, &base)?;
                    copied = true;
                } else {
                    eprintln!("public/ leads outside the project; it is not copied");
                }
            }
        }
        let wasm_opt = self.release && wasm_opt(out)?;
        let mut files = BTreeMap::new();
        for name in BUILT {
            files.insert(name.to_string(), size(&out.join(name))?);
        }
        if copied {
            list(&out.join("assets"), "assets", &mut files)?;
        }
        let listing = Listing { wasm_opt, files };
        self.write_manifest(out, &listing)?;
        Ok(listing)
    }

    /// After `nova build --ssg`'s server has written its pages into `out`:
    /// `manifest.json` again, with `pages`, each a path from `out` such as
    /// `about/index.html`, added to `listing`, and every file measured
    /// again, since the root's page replaced `index.html`.
    pub fn add_pages(&self, out: &Path, mut listing: Listing, pages: &[String]) -> Result<()> {
        for page in pages {
            listing.files.insert(page.clone(), 0);
        }
        for (name, bytes) in listing.files.iter_mut() {
            *bytes = size(&out.join(name))?;
        }
        self.write_manifest(out, &listing)
    }

    fn write_manifest(&self, out: &Path, listing: &Listing) -> Result<()> {
        let path = out.join("manifest.json");
        std::fs::write(&path, self.manifest(listing.wasm_opt, &listing.files))
            .with_context(|| format!("writing {}", path.display()))
    }

    /// `manifest.json`: the build's name, its options and its files, sorted,
    /// with their sizes in bytes, and nothing that changes between two
    /// builds of the same sources.
    fn manifest(&self, wasm_opt: bool, files: &BTreeMap<String, u64>) -> String {
        let version = self.version.as_deref().map_or("null".to_string(), quote);
        let files: Vec<String> = files
            .iter()
            .map(|(path, size)| format!("    {}: {size}", quote(path)))
            .collect();
        format!(
            "{{\n  \"name\": {},\n  \"version\": {version},\n  \"release\": {},\n  \"wasm_opt\": {wasm_opt},\n  \"files\": {{\n{}\n  }}\n}}\n",
            quote(&self.name),
            self.release,
            files.join(",\n"),
        )
    }
}

/// [`Bundle::check_output`] for a program whose project's root is `root`,
/// or for a single file with `None`, before there is a program: `nova dev`
/// refuses an output directory before its first build (the 4.5 spec
/// `docs/superpowers/specs/2026-10-18-phase-4-5-router-and-nova-dev-design.md`
/// §4.1).
pub fn check_output_for(root: Option<&Path>, out: &Path) -> Result<()> {
    let Some(root) = root else {
        return Ok(());
    };
    let out = resolved(out);
    if resolved(root).starts_with(&out) {
        bail!("the output directory holds the project; name another with `-o`");
    }
    if out.starts_with(resolved(&root.join("public"))) {
        bail!("the output directory is inside `public/`; name another with `-o`");
    }
    Ok(())
}

/// `s` as a JSON string.
fn quote(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

/// `wasm-opt -Oz app.wasm -o app.wasm` in `out`, when `wasm-opt` is on
/// `PATH`: whether it ran. One that fails fails the build.
fn wasm_opt(out: &Path) -> Result<bool> {
    let status = std::process::Command::new("wasm-opt")
        .args(["-Oz", "app.wasm", "-o", "app.wasm"])
        .current_dir(out)
        .status();
    match status {
        Ok(s) if s.success() => Ok(true),
        Ok(s) => bail!("wasm-opt failed: {s}"),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            eprintln!("wasm-opt not found on PATH; app.wasm is not optimized");
            Ok(false)
        }
        Err(e) => Err(e).context("running wasm-opt"),
    }
}

/// `from`'s files into `to`, directories kept. A symbolic link to a file
/// inside `public` is copied as the file; any other link is skipped, so a
/// copy can neither loop nor reach outside `public/`.
fn copy_tree(from: &Path, to: &Path, public: &Path) -> Result<()> {
    std::fs::create_dir_all(to).with_context(|| format!("creating {}", to.display()))?;
    let entries = std::fs::read_dir(from).with_context(|| format!("reading {}", from.display()))?;
    for entry in entries {
        let entry = entry.with_context(|| format!("reading {}", from.display()))?;
        let path = entry.path();
        let dest = to.join(entry.file_name());
        let kind = entry
            .file_type()
            .with_context(|| format!("reading {}", path.display()))?;
        let inside = || path.is_file() && resolved(&path).starts_with(public);
        if kind.is_dir() {
            copy_tree(&path, &dest, public)?;
        } else if kind.is_file() || (kind.is_symlink() && inside()) {
            std::fs::copy(&path, &dest)
                .with_context(|| format!("copying {} to {}", path.display(), dest.display()))?;
        }
    }
    Ok(())
}

/// Each file under `dir`, by its path from `dist/` with `/` between names.
fn list(dir: &Path, prefix: &str, files: &mut BTreeMap<String, u64>) -> Result<()> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Ok(());
    };
    for entry in entries {
        let entry = entry.with_context(|| format!("reading {}", dir.display()))?;
        let name = format!("{prefix}/{}", entry.file_name().to_string_lossy());
        let path = entry.path();
        if path.is_dir() {
            list(&path, &name, files)?;
        } else {
            files.insert(name, size(&path)?);
        }
    }
    Ok(())
}

/// Whether `meta` is a file with another name, a hard link, so a write to
/// one writes the other. Unix alone tells, on stable Rust.
#[cfg(unix)]
fn hard_linked(meta: &std::fs::Metadata) -> bool {
    use std::os::unix::fs::MetadataExt;
    meta.is_file() && meta.nlink() > 1
}

#[cfg(not(unix))]
fn hard_linked(_: &std::fs::Metadata) -> bool {
    false
}

fn size(path: &Path) -> Result<u64> {
    Ok(std::fs::metadata(path)
        .with_context(|| format!("reading {}", path.display()))?
        .len())
}

/// A file, or a directory and all it holds.
fn remove(path: &Path) -> Result<()> {
    let dir = std::fs::symlink_metadata(path).is_ok_and(|m| m.is_dir());
    if dir {
        std::fs::remove_dir_all(path)
    } else {
        std::fs::remove_file(path)
    }
    .with_context(|| format!("removing {}", path.display()))
}

/// `path` absolute, with its links resolved as far as it exists and each
/// `..` after that taken away, so two spellings of one directory compare
/// equal before either is created: `nope/..` is the current directory, as
/// `create_dir_all` and every write through it find. Windows takes each
/// `..` away by the text first, before it looks at the disk.
pub fn resolved(path: &Path) -> PathBuf {
    let mut out = PathBuf::new();
    let start = std::env::current_dir().unwrap_or_default();
    let whole = if path.is_absolute() {
        path.to_path_buf()
    } else {
        start.join(path)
    };
    let whole = if cfg!(windows) {
        by_text(&whole)
    } else {
        whole
    };
    for part in whole.components() {
        match part {
            Component::CurDir => {}
            Component::ParentDir => {
                out.pop();
            }
            other => out.push(other),
        }
        if let Ok(real) = out.canonicalize() {
            out = real;
        }
    }
    out
}

/// `path` with each `.` and `..` taken away by the text alone.
fn by_text(path: &Path) -> PathBuf {
    let mut out = PathBuf::new();
    for part in path.components() {
        match part {
            Component::CurDir => {}
            Component::ParentDir => {
                out.pop();
            }
            other => out.push(other),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::{quote, Bundle};

    /// `finish` never copies the project's page onto itself, which would
    /// empty it, even when `check_output` is not asked first and the output
    /// directory is spelled another way.
    #[test]
    fn the_page_is_never_copied_onto_itself() {
        let dir = std::env::temp_dir().join("nova-cli-bundle-page-onto-itself");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("sub")).expect("a directory");
        for (name, text) in [
            ("index.html", "mine"),
            ("app.js", "js"),
            ("app.wasm", "wasm"),
            ("package.json", "{}"),
        ] {
            std::fs::write(dir.join(name), text).expect("a file");
        }
        let bundle = Bundle {
            root: Some(dir.clone()),
            name: "site".to_string(),
            version: None,
            release: false,
        };
        bundle
            .finish(&dir.join("sub").join(".."))
            .expect("the bundle finishes");
        let page = std::fs::read_to_string(dir.join("index.html")).expect("the page");
        assert_eq!(page, "mine");
    }

    /// `add_pages` lists the pages `nova build --ssg`'s server wrote, with
    /// their sizes, and measures the files already listed again, since the
    /// root's page has replaced `index.html` (the 4.6 spec §7).
    #[test]
    fn ssg_pages_join_the_manifest_with_their_sizes() {
        let dir = std::env::temp_dir().join("nova-cli-bundle-ssg-pages");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("about")).expect("a directory");
        for (name, text) in [
            ("index.html", "page"),
            ("app.js", "js"),
            ("app.wasm", "wasm"),
            ("package.json", "{}"),
        ] {
            std::fs::write(dir.join(name), text).expect("a file");
        }
        let bundle = Bundle {
            root: None,
            name: "site".to_string(),
            version: None,
            release: false,
        };
        let listing = bundle.finish(&dir).expect("the bundle finishes");
        std::fs::write(dir.join("index.html"), "the root's page").expect("a page");
        std::fs::write(dir.join("about").join("index.html"), "about").expect("a page");
        bundle
            .add_pages(&dir, listing, &["about/index.html".to_string()])
            .expect("the pages are listed");
        let manifest = std::fs::read_to_string(dir.join("manifest.json")).expect("the manifest");
        assert!(
            manifest.contains(
                "    \"about/index.html\": 5,\n    \"app.js\": 2,\n    \"app.wasm\": 4,\n    \"index.html\": 15,\n"
            ),
            "{manifest}"
        );
    }

    #[test]
    fn quote_escapes_what_json_needs() {
        assert_eq!(quote("plain"), "\"plain\"");
        assert_eq!(quote("a\"b\\c\nd"), "\"a\\\"b\\\\c\\u000ad\"");
        assert_eq!(quote("ไทย"), "\"ไทย\"");
    }
}
