//! The runtime library a `nova` carries, and where `nova build` and
//! `nova test` find the one they link (spec
//! `docs/superpowers/specs/2026-10-07-phase-3-0-foundations-design.md` §4.2;
//! `docs/adr/0027-runtime-and-std-in-an-installed-nova.md`).
//!
//! nova-cli's build script embeds the library gzip-compressed. This module
//! unpacks it once per version and build into
//! `$NOVA_HOME/runtime/<version>-<crc32>/`, and every later link reuses that
//! file after checking its size and CRC-32.

use std::fs;
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::OnceLock;

use anyhow::{bail, Context, Result};

/// The runtime library a `nova` carries, as its build script embedded it.
#[derive(Clone, Copy, Debug)]
pub struct EmbeddedRuntime {
    /// The library, gzip-compressed. Empty when this `nova` carries none.
    pub gz: &'static [u8],
    /// The uncompressed library's CRC-32.
    pub crc32: u32,
    /// The uncompressed library's size in bytes.
    pub size: u64,
    /// The library's file name for the target `nova` was built for.
    pub file_name: &'static str,
    /// nova-cli's version: the first half of the cache directory's name.
    pub version: &'static str,
}

static EMBEDDED: OnceLock<EmbeddedRuntime> = OnceLock::new();

/// Record the runtime this `nova` carries. nova-cli calls this once, at
/// startup. An empty payload records nothing, so the lookup goes on to the
/// executable's neighbours.
pub fn set_embedded_runtime(runtime: EmbeddedRuntime) {
    if !runtime.gz.is_empty() {
        let _ = EMBEDDED.set(runtime);
    }
}

/// The runtime [`set_embedded_runtime`] recorded, if any.
pub(crate) fn embedded_runtime() -> Option<&'static EmbeddedRuntime> {
    EMBEDDED.get()
}

/// The cache's root, `$NOVA_HOME/runtime`. `NOVA_HOME` defaults to `.nova`
/// in the home directory, which is `USERPROFILE` on Windows and `HOME`
/// elsewhere. It takes the variables' values rather than reading them, so
/// tests need not change the process environment. An empty value counts as
/// unset. The rule is `nova_pm::nova_home`'s (spec 3.3b §5.1).
pub(crate) fn cache_root(
    nova_home: Option<PathBuf>,
    userprofile: Option<PathBuf>,
    home: Option<PathBuf>,
    windows: bool,
) -> Result<PathBuf> {
    match nova_pm::nova_home(nova_home, userprofile, home, windows) {
        Some(home) => Ok(home.join("runtime")),
        None => {
            let name = if windows { "USERPROFILE" } else { "HOME" };
            bail!(
                "cannot place the runtime library's cache: neither NOVA_HOME nor {name} is \
                 set; set NOVA_HOME to a writable directory, or NOVA_RUNTIME_LIB to a runtime \
                 library"
            )
        }
    }
}

/// Make sure `<root>/<version>-<crc32>/<file name>` holds `runtime`'s
/// library, unpacking it when it is missing or damaged, and return its path.
pub(crate) fn unpack(runtime: &EmbeddedRuntime, root: &Path) -> Result<PathBuf> {
    let dir = root.join(format!("{}-{:08x}", runtime.version, runtime.crc32));
    let path = dir.join(runtime.file_name);
    if verified(&path, runtime) {
        return Ok(path);
    }
    fs::create_dir_all(&dir).with_context(|| {
        format!(
            "creating the runtime library's cache {}; set NOVA_HOME to a writable \
             directory, or NOVA_RUNTIME_LIB to a runtime library",
            dir.display()
        )
    })?;
    let (file, temp) = new_temp_file(&dir, runtime.file_name)?;
    if let Err(error) = decompress(runtime, file) {
        let _ = fs::remove_file(&temp);
        return Err(error.context(format!(
            "unpacking the runtime library into {}",
            temp.display()
        )));
    }
    match fs::rename(&temp, &path) {
        Ok(()) => Ok(path),
        Err(error) => {
            let _ = fs::remove_file(&temp);
            // Another process may have put a good copy there first.
            if verified(&path, runtime) {
                Ok(path)
            } else {
                Err(error)
                    .with_context(|| format!("moving the runtime library to {}", path.display()))
            }
        }
    }
}

/// Whether `path` holds `runtime`'s library: the right size and CRC-32. A
/// file that cannot be read does not; unpacking it again reports why.
fn verified(path: &Path, runtime: &EmbeddedRuntime) -> bool {
    let Ok(mut file) = fs::File::open(path) else {
        return false;
    };
    match file.metadata() {
        Ok(metadata) if metadata.len() == runtime.size => {}
        _ => return false,
    }
    let mut hasher = crc32fast::Hasher::new();
    let mut buffer = vec![0u8; 1 << 16];
    loop {
        match file.read(&mut buffer) {
            Ok(0) => break,
            Ok(read) => hasher.update(&buffer[..read]),
            Err(_) => return false,
        }
    }
    hasher.finalize() == runtime.crc32
}

static TEMP_COUNTER: AtomicU64 = AtomicU64::new(0);

/// A new temporary file beside the library. The process id and a
/// per-process counter make its name unique, and `create_new` refuses a
/// name a crashed run left behind, in which case the next number is tried.
fn new_temp_file(dir: &Path, file_name: &str) -> Result<(fs::File, PathBuf)> {
    loop {
        let n = TEMP_COUNTER.fetch_add(1, Ordering::Relaxed);
        let temp = dir.join(format!("{file_name}.{}-{n}.tmp", std::process::id()));
        match fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temp)
        {
            Ok(file) => return Ok((file, temp)),
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => continue,
            Err(error) => {
                return Err(error).with_context(|| format!("creating {}", temp.display()))
            }
        }
    }
}

/// Decompress `runtime` into `file`, checking the result's size and CRC-32.
fn decompress(runtime: &EmbeddedRuntime, mut file: fs::File) -> Result<()> {
    let mut decoder = flate2::read::GzDecoder::new(runtime.gz);
    let mut hasher = crc32fast::Hasher::new();
    let mut size = 0u64;
    let mut buffer = vec![0u8; 1 << 16];
    loop {
        let read = decoder.read(&mut buffer).context("decompressing")?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
        file.write_all(&buffer[..read]).context("writing")?;
        size += read as u64;
    }
    file.flush().context("writing")?;
    if size != runtime.size || hasher.finalize() != runtime.crc32 {
        bail!(
            "the runtime library this nova carries is damaged: it unpacked to {size} bytes \
             with a different CRC-32, not the {} bytes it was built with",
            runtime.size
        );
    }
    Ok(())
}

/// `find_runtime_lib`'s order, over explicit inputs (spec §4.2):
/// `NOVA_RUNTIME_LIB`, then the embedded runtime, then the executable's
/// neighbours. When a runtime is embedded, failing to unpack it is an error;
/// the neighbours are tried only when nothing is embedded.
pub(crate) fn locate(
    override_path: Option<PathBuf>,
    embedded: Option<&EmbeddedRuntime>,
    cache_root: impl FnOnce() -> Result<PathBuf>,
    beside_exe: impl FnOnce() -> Result<PathBuf>,
) -> Result<PathBuf> {
    if let Some(path) = override_path {
        if path.exists() {
            return Ok(path);
        }
        bail!(
            "NOVA_RUNTIME_LIB points to {}, which does not exist",
            path.display()
        );
    }
    match embedded {
        Some(runtime) => unpack(runtime, &cache_root()?),
        None => beside_exe(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A fake library and its embedded form. The bytes are the same in
    /// every process, so a child process unpacks exactly what its parent
    /// does.
    fn fake_runtime() -> EmbeddedRuntime {
        let library: Vec<u8> = (0..4 * 1024 * 1024u32).map(|i| (i % 251) as u8).collect();
        let mut encoder = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::fast());
        encoder.write_all(&library).unwrap();
        let gz = encoder.finish().unwrap();
        EmbeddedRuntime {
            gz: Box::leak(gz.into_boxed_slice()),
            crc32: crc32fast::hash(&library),
            size: library.len() as u64,
            file_name: "fake_runtime.lib",
            version: "0.0.0-test",
        }
    }

    /// A fresh, empty directory under the system temp dir, unique to this
    /// test.
    fn fresh_dir(name: &str) -> PathBuf {
        // A fixed name, so each run replaces the last run's directory: a name with
        // the process id in it left one more behind on every run.
        let dir = std::env::temp_dir().join(format!("nova-runtime-cache-{name}"));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn library_path(root: &Path, runtime: &EmbeddedRuntime) -> PathBuf {
        root.join(format!("0.0.0-test-{:08x}", runtime.crc32))
            .join("fake_runtime.lib")
    }

    fn never() -> Result<PathBuf> {
        panic!("this step of the lookup must not run")
    }

    #[test]
    fn the_first_use_writes_the_library() {
        let runtime = fake_runtime();
        let root = fresh_dir("first");
        let path = unpack(&runtime, &root).unwrap();
        assert_eq!(path, library_path(&root, &runtime));
        assert!(verified(&path, &runtime));
    }

    #[test]
    fn a_second_use_reuses_the_file_untouched() {
        let runtime = fake_runtime();
        let root = fresh_dir("second");
        let path = unpack(&runtime, &root).unwrap();
        let written = fs::metadata(&path).unwrap().modified().unwrap();
        std::thread::sleep(std::time::Duration::from_millis(50));
        assert_eq!(unpack(&runtime, &root).unwrap(), path);
        assert_eq!(fs::metadata(&path).unwrap().modified().unwrap(), written);
    }

    #[test]
    fn a_damaged_copy_is_replaced() {
        let runtime = fake_runtime();
        let root = fresh_dir("damaged");
        let path = unpack(&runtime, &root).unwrap();
        // The right size, the wrong bytes.
        fs::write(&path, vec![0u8; runtime.size as usize]).unwrap();
        assert!(!verified(&path, &runtime));
        unpack(&runtime, &root).unwrap();
        assert!(verified(&path, &runtime));
        // The wrong size.
        fs::write(&path, b"short").unwrap();
        unpack(&runtime, &root).unwrap();
        assert!(verified(&path, &runtime));
    }

    #[test]
    fn threads_unpacking_at_once_all_get_a_verified_file() {
        let runtime = fake_runtime();
        let root = fresh_dir("threads");
        let barrier = std::sync::Barrier::new(8);
        std::thread::scope(|scope| {
            let handles: Vec<_> = (0..8)
                .map(|_| {
                    scope.spawn(|| {
                        barrier.wait();
                        unpack(&runtime, &root)
                    })
                })
                .collect();
            for handle in handles {
                let path = handle.join().unwrap().unwrap();
                assert!(verified(&path, &runtime));
            }
        });
        let dir = library_path(&root, &runtime)
            .parent()
            .unwrap()
            .to_path_buf();
        let leftovers: Vec<_> = fs::read_dir(dir)
            .unwrap()
            .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
            .filter(|name| name.ends_with(".tmp"))
            .collect();
        assert!(leftovers.is_empty(), "{leftovers:?}");
    }

    /// Two processes unpacking at once both get a verified file (spec §9,
    /// as Section 3 of the design asked). The test runs its own binary
    /// twice, and each child runs only `unpack_as_a_child_process`. Their
    /// output is piped, not inherited: an inherited stdout would put each
    /// child's own `test result:` line into the parent's output, and so
    /// into every count of a full run.
    #[test]
    fn processes_unpacking_at_once_both_get_a_verified_file() {
        let root = fresh_dir("processes");
        let me = std::env::current_exe().unwrap();
        let children: Vec<_> = (0..2)
            .map(|_| {
                std::process::Command::new(&me)
                    .args([
                        "--exact",
                        "runtime_cache::tests::unpack_as_a_child_process",
                        "--nocapture",
                    ])
                    .env("NOVA_TEST_UNPACK_ROOT", &root)
                    .stdout(std::process::Stdio::piped())
                    .stderr(std::process::Stdio::piped())
                    .spawn()
                    .unwrap()
            })
            .collect();
        for child in children {
            let output = child.wait_with_output().unwrap();
            assert!(
                output.status.success(),
                "a child failed:\n{}\n{}",
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            );
        }
        let runtime = fake_runtime();
        assert!(verified(&library_path(&root, &runtime), &runtime));
    }

    /// Does nothing unless `NOVA_TEST_UNPACK_ROOT` names a root to unpack
    /// into, which only the test above sets, for its child processes.
    #[test]
    fn unpack_as_a_child_process() {
        let Some(root) = std::env::var_os("NOVA_TEST_UNPACK_ROOT") else {
            return;
        };
        let runtime = fake_runtime();
        let path = unpack(&runtime, Path::new(&root)).unwrap();
        assert!(verified(&path, &runtime));
    }

    #[test]
    fn a_nova_home_that_is_a_file_gives_an_error_naming_it() {
        let runtime = fake_runtime();
        let file = fresh_dir("home-is-a-file").join("not-a-directory");
        fs::write(&file, b"").unwrap();
        let error = format!("{:#}", unpack(&runtime, &file.join("runtime")).unwrap_err());
        assert!(error.contains("not-a-directory"), "{error}");
        assert!(error.contains("NOVA_HOME"), "{error}");
    }

    /// Review Focus 4: a Windows profile such as `C:\Users\สมชาย ใจดี`.
    #[test]
    fn a_cache_path_with_spaces_and_thai_letters_works() {
        let runtime = fake_runtime();
        let root = fresh_dir("path").join("โฟลเดอร์ มี ช่องว่าง");
        let path = unpack(&runtime, &root).unwrap();
        assert!(verified(&path, &runtime));
    }

    #[test]
    fn nova_home_decides_the_root() {
        let root = cache_root(
            Some("D:/n".into()),
            Some("C:/Users/u".into()),
            Some("/home/u".into()),
            true,
        )
        .unwrap();
        assert_eq!(root, PathBuf::from("D:/n").join("runtime"));
    }

    #[test]
    fn without_nova_home_the_root_is_under_the_home_directory() {
        let windows = cache_root(
            None,
            Some("C:/Users/u".into()),
            Some("/home/u".into()),
            true,
        );
        assert_eq!(
            windows.unwrap(),
            PathBuf::from("C:/Users/u").join(".nova").join("runtime")
        );
        let unix = cache_root(
            None,
            Some("C:/Users/u".into()),
            Some("/home/u".into()),
            false,
        );
        assert_eq!(
            unix.unwrap(),
            PathBuf::from("/home/u").join(".nova").join("runtime")
        );
    }

    #[test]
    fn an_empty_variable_counts_as_unset() {
        let root = cache_root(Some("".into()), Some("C:/Users/u".into()), None, true);
        assert_eq!(
            root.unwrap(),
            PathBuf::from("C:/Users/u").join(".nova").join("runtime")
        );
    }

    #[test]
    fn with_no_home_at_all_the_error_names_the_variables() {
        for windows in [true, false] {
            let error = cache_root(None, None, None, windows)
                .unwrap_err()
                .to_string();
            assert!(error.contains("NOVA_HOME"), "{error}");
            let home = if windows { "USERPROFILE" } else { "HOME" };
            assert!(error.contains(home), "{error}");
        }
    }

    #[test]
    fn nova_runtime_lib_comes_first() {
        let runtime = fake_runtime();
        let lib = fresh_dir("override").join("my_runtime.lib");
        fs::write(&lib, b"").unwrap();
        assert_eq!(
            locate(Some(lib.clone()), Some(&runtime), never, never).unwrap(),
            lib
        );
    }

    #[test]
    fn a_missing_nova_runtime_lib_is_an_error() {
        let missing = PathBuf::from("no/such/runtime.lib");
        let error = locate(Some(missing), None, never, never)
            .unwrap_err()
            .to_string();
        assert!(error.contains("NOVA_RUNTIME_LIB"), "{error}");
    }

    #[test]
    fn the_embedded_runtime_comes_before_the_executables_neighbours() {
        let runtime = fake_runtime();
        let root = fresh_dir("embedded-first");
        let path = locate(None, Some(&runtime), || Ok(root.clone()), never).unwrap();
        assert_eq!(path, library_path(&root, &runtime));
    }

    #[test]
    fn an_embedded_runtime_that_fails_to_unpack_is_an_error_not_a_fall_back() {
        let runtime = fake_runtime();
        let error = locate(None, Some(&runtime), || bail!("no home here"), never)
            .unwrap_err()
            .to_string();
        assert!(error.contains("no home here"), "{error}");
    }

    #[test]
    fn with_nothing_embedded_the_neighbours_are_tried() {
        let lib = PathBuf::from("beside/nova_runtime.lib");
        assert_eq!(locate(None, None, never, || Ok(lib.clone())).unwrap(), lib);
    }
}
