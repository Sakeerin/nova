# Phase 3.3b, "The index and publishing", Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** A Nova package can depend on a package published to an index.
`nova add json`, `nova update`, `nova fetch`, `nova package`, `nova login`
and `nova publish` work against a git-backed index, read over HTTPS or
from a local directory, with versions pinned in `nova.lock` and packages
downloaded once into a checksum-verified cache.

**Architecture:**
- **`nova-pm` stays pure:**
  - the `$NOVA_HOME` rule;
  - the canonical index form and the cache directory's name;
  - `nova.lock`;
  - the resolver;
  - an offline graph that finds registry packages through the lock and
    the cache.
- **A new crate, `nova-index`, owns everything that touches the network:**
  - reading an index (HTTPS, a local directory, or the GitHub API);
  - downloading and unpacking;
  - packing;
  - the sync step;
  - publishing, and the stored token.
- **`nova-cli`:**
  - syncs before `run`, `build`, `check` and `test`;
  - gains `fetch`, `update`, `package`, `publish` and `login`;
  - teaches `nova add` the registry form.
- **The driver** counts warnings and hides a dependency's.
- **The language server** never syncs. It shows M0005 for an entry the
  cache cannot satisfy, and re-checks when `nova.lock` changes.

**Tech Stack:**
- Rust: MSRV 1.78, edition 2021.
- New crates:
  - `ureq =3.2.1`, with rustls on the `ring` already locked;
  - `tar 0.4`;
  - `base64 0.22`.
- Already locked: `serde`, `serde_json`, `flate2`, `crc32fast`, `semver`,
  `toml_edit`, `ring`.
- Tests:
  - `assert_cmd` end to end;
  - loopback HTTP servers written for the tests;
  - temporary `NOVA_HOME`s and local indexes.

**Spec:** `docs/superpowers/specs/2026-10-09-phase-3-3b-index-and-publishing-design.md`
(commits `7591506` and `77c2ee5`), approved by the user on 2026-10-09.
- Read it before Task 1: it is the authority this plan argues from.
- Its §14 lists 35 decisions, and its §2 the code as it stood.

## Global Constraints

- **The workspace's minimum Rust stays 1.78,** edition 2021. CI's MSRV job
  runs `cargo check --locked --workspace` with `RUSTFLAGS=-D warnings` on
  Linux; Task 8 also runs it on Windows here.
- **No test reaches the internet.** Every test that syncs sets
  `NOVA_HOME` to a fresh temporary directory, and `NOVA_INDEX` to a local
  index or a loopback server (spec §10).
- **Nothing is published for real, and no tag is pushed.** The by-hand
  publish to `Sakeerin/nova-index` (spec §11.2) is the user's, on the
  user's word; this plan never runs it. The merge is by rebase, on the
  user's word only.
- **The token** goes only to `api.github.com`, `uploads.github.com`, or a
  loopback `NOVA_GITHUB_API`. It never appears in output, a command line or
  an error (spec §8).
- **The codes:**
  - M0005 changes meaning: a registry entry the cache cannot satisfy;
  - M0014, M0015, M0016 and M0017 are new.
- **Every existing test passes,** except the ones this plan changes on
  purpose, each named in its task:
  - `a_version_only_entry_is_m0005_and_names_the_dependency` (Task 4);
  - `a_declared_dependency_is_m0005`, renamed
    `a_declared_dependency_the_index_lacks_is_m0014` (Task 11);
  - the M0005 case of `each_graph_error_is_rendered_and_stops_the_command`
    (Task 11);
  - the `nova add geom` case of `add_refuses_without_writing` (Task 11).
- **`Cargo.lock`** gains only `nova-index` and the packages Tasks 7 and 8
  list, each checked against Rust 1.78.

## Review Focus

The five inputs most likely to bite a user that the spec's tests leave
uncovered, most likely first. Each has its test in the task that owns the
code:

1. **A hand-tightened requirement.** `json = "1"` edited to `"1.4"` while
   `nova.lock` holds 1.2.0: the next build moves `json` to the newest 1.x
   at or above 1.4, and every other locked version stays →
   `a_tightened_requirement_moves_only_its_package` (Task 3).
2. **A manifest with CRLF line endings under the registry form of `nova
   add`.** It must keep its CRLF →
   `add_from_the_index_keeps_a_crlf_manifests_line_endings` (Task 11).
3. **A file name outside ASCII**, such as a Thai `README.ไทย.md`, packed
   and unpacked: it must arrive unchanged →
   `a_file_name_outside_ascii_survives_packing_and_unpacking` (Task 7).
4. **A package name with capital letters,** `Geom`. It must be published
   to the lower-case index path `ge/om/geom`, and resolved by its exact
   name → `a_capitalised_name_lives_at_the_lower_case_path` (Task 10).
5. **A download cut off part-way.** Nothing must be left in the cache,
   and the next fetch succeeds →
   `a_cut_off_download_leaves_nothing_and_the_next_one_works` (Task 8).

## Decisions: where this plan settles what the spec leaves open

1. **The interfaces.**
   - **`nova-pm`:**
     - `nova_home`, `nova_home_from_env` and `registry_dir`;
     - `canonical_index`, `local_index_path` and `index_dir_name`;
     - `is_portable`;
     - `Lock`, `LockedPackage`, `parse_lock` and `LOCKFILE`;
     - `Candidate`, `IndexView`, `Requirement`, `Unlock`, `ResolveError`
       and `resolve`;
     - `Offline` with `graph_with`, and `requirements`.
   - **`nova-index`:**
     - `Index`, `Source` and `Location`;
     - the `Reader` trait, with `LocalReader`, `HttpReader`, `ApiReader`,
       `View` and `reader_for`;
     - `Line`, `Config` and their parsers;
     - `pack`, `unpack`, `Http` and `fetch_package`;
     - `SyncRequest`, `Synced`, `SyncError`, `sync` and `write_lock`;
     - `check_new`, `publish_local`, `GitHub` and `publish_github`;
     - `load_token` and `store_token`.
   - **`nova-driver`:** `Program::for_package_in`, `Checked` and
     `check_program_counted`.
2. **A plain sync keeps the lock when it can.**
   - Every locked version that still fits is kept (spec §4.3).
   - When the kept versions cannot all stay, the search runs again with no
     lock, and any error comes from that second search.
   - `nova update <name>` has no second search. Its error says to run
     `nova update`.
3. **A downloaded package's manifest is checked by names.** Its
   `[dependencies]` names must equal its locked `dependencies`, which the
   resolver took from its index line (spec §5.2). No entry may be a
   `path`. Requirements are the index line's, which the resolver used.
4. **The HTTP client follows redirects itself.**
   - ureq's `max_redirects` is 0. `Http::get` follows up to five, each
     checked against the https-or-loopback rule, never carrying the
     token.
   - The timeout is 60 seconds and the User-Agent is `nova/<version>`.
   - An HTTP error status is a value, not an `Err`.
5. **Rust 1.78.** `.cargo/config.toml` sets `[resolver]
   incompatible-rust-versions = "fallback"`, so Cargo picks versions that
   build on the workspace's `rust-version`. Task 8 then checks the
   lockfile with Rust 1.78 on Windows here, and CI's MSRV job does so on
   Linux.
6. **The tarball format:**
   - GNU headers, files only (unpacking creates directories);
   - mode `0644`, modification time 0, owner and group 0;
   - entries sorted by path;
   - gzip with modification time 0 and no file name.
7. **The verification's directory** is
   `<temp>/nova-verify-<process id>/`. It is removed before and after.
8. **Downloads are reported on standard error,** as `downloaded <name>
   <version>`. `nova fetch` prints `fetched <n> package(s)` or `nothing
   to fetch`.
9. **A package root inside the registry directory is refused** by
   `project::sync` (and so by every command that syncs), and by `nova
   package` and `nova publish`.
10. **The language server and cached files.** A project whose directory
    is inside the registry directory publishes nothing. No new
    `ProjectKey` variant is needed.
11. **The tests' loopback server** is `crates/nova-index/tests/support/mod.rs`.
    `nova-cli`'s tests include it with `#[path]`.
12. **The GitHub API's headers:** `Authorization: Bearer <token>`,
    `Accept: application/vnd.github+json`, `X-GitHub-Api-Version:
    2022-11-28`.
13. **`nova update <name>` for a name not in `nova.lock`** is an error:
    "`<name>` is not in nova.lock".
14. **An index line's `deps`** are the manifest's `[dependencies]`
    requirements as `semver` prints them, e.g. `^1.0`.
15. **The dependencies that verification uses for warnings.** A warning in
    a registry package is never shown, so every warning the check shows
    is the package's own.
16. **The by-hand gate (spec §11.2) is not a task.** Task 16's PR body
    says it waits for the user.
17. **A missing local index keeps its canonical form.** The real path is
    taken of the longest part that exists, then the rest is appended.
    Otherwise Windows' 8.3 names and macOS's `/var` would give a renamed
    index another form, and "a build needs no index" would resolve again.
18. **`$NOVA_HOME` is needed only to download.** `SyncRequest.registry` is
    an `Option`. A project with no registry dependency syncs with no
    `$NOVA_HOME` and no index read, so every existing test runs as
    before.
19. **The lock is read lazily** by the graph, at its first registry
    entry. A project with none never reads one.
20. **A downloaded manifest's warnings never leave the graph,** and the
    driver drops the warnings of a downloaded package's modules. A
    warning with no label is always shown.
21. **`nova run` and `nova build` refuse a library before syncing** (spec
    §5.3: "after the CLI's own refusals"). So a library with a graph
    error now hears "is a library" first.
22. **A sync's errors stop a command** before the driver runs, each
    diagnostic rendered once. The sync renders no warnings: the driver's
    graph shows those.
23. **Verification comes before writing.** `target/package/` gets the
    tarball only after the verification passes.
24. **The new lockfile packages are listed as Tasks 7 and 8 add them,**
    with each version's `rust_version` and edition read from crates.io,
    rather than predicted here (spec §9). The fallback resolver picks the
    versions, and the ledger and the PR body carry the list. The packages
    only Windows or macOS builds are marked. Windows' are checked by the
    MSRV build here; macOS's by their crates.io records.

## File Structure

| File | Task | Responsibility |
|---|---|---|
| `crates/nova-pm/src/home.rs` (new), `src/index_name.rs` (new), `src/lib.rs`, `Cargo.toml`, `tests/home.rs` (new) | 1 | `$NOVA_HOME`, the canonical index form, `<idx>` |
| `crates/nova-driver/src/runtime_cache.rs` | 1 | `cache_root` on `nova_pm::nova_home` |
| `crates/nova-pm/src/lock.rs` (new), `tests/lock.rs` (new) | 2 | `nova.lock` |
| `crates/nova-pm/src/resolve.rs` (new), `tests/resolve.rs` (new) | 3 | The resolver |
| `crates/nova-pm/src/graph.rs`, `tests/graph.rs`, `tests/registry_graph.rs` (new) | 4 | The offline graph, `requirements` |
| `crates/nova-driver/src/{program,lib,analyze}.rs`, `tests/registry.rs` (new) | 5 | `for_package_in`, warnings counted and a dependency's hidden |
| `crates/nova-index/` (new): `Cargo.toml`, `src/{lib,location,line,read}.rs`, `tests/index.rs` | 6 | The crate, the index format, readers |
| `crates/nova-index/src/{pack,cache}.rs`, `tests/pack.rs`; `crates/nova-pm/src/name.rs`, `tests/names.rs` | 7 | Packing and unpacking, `is_portable` |
| `crates/nova-index/src/{http,download}.rs`, `src/read.rs`, `tests/support/mod.rs`, `tests/http.rs`, `.cargo/config.toml` (new), `Cargo.lock` | 8 | HTTP, downloads |
| `crates/nova-index/src/sync.rs`, `tests/sync.rs` | 9 | The sync step |
| `crates/nova-index/src/publish.rs`, `tests/publish.rs`; `crates/nova-cli/src/cmd/package.rs` (new), `src/project.rs`, `src/cmd/mod.rs`, `src/main.rs`, `Cargo.toml`, `tests/registry.rs` (new) | 10 | `nova package`, `nova publish` to a local index |
| `crates/nova-cli/src/project.rs`, `src/cmd/{run,test,deps}.rs`, `src/cmd/fetch.rs` (new), `tests/{registry,project,packages,deps}.rs` | 11 | Syncing commands, `fetch`, `update`, registry `add` |
| `crates/nova-index/src/{github,credentials,download}.rs`, `tests/github.rs`, `tests/support/fake_github.rs`; `crates/nova-cli/src/cmd/{login,package}.rs`, `tests/registry.rs`; `Cargo.toml` | 12 | GitHub publishing, `nova login` |
| `crates/nova-lsp/src/{lib,checker}.rs`, `crates/nova-cli/tests/{lsp.rs,lsp_client/mod.rs}` | 13 | The server and the lock |
| `.github/scripts/registry-gate.sh` (new), `.github/workflows/ci.yml` | 14 | The 3.3b gate |
| `docs/adr/0031-package-index-and-publishing.md` (new), and the records Task 15 lists | 15 | ADR, notes, CHANGELOG, README, ARCHITECTURE, sweep |
| `$P/pr-body.md`, outside the repository | 16 | Final verification, mutants, the PR body |

## Conventions for every task

- **Working directory.** `D:\Projects\nona\nova`, which is Git Bash
  `/d/Projects/nona/nova`. The Bash tool resets its directory after each
  call, so write `cd /d/Projects/nona/nova && …` in one command.
- **Scratch directory.** `P=/c/Users/SAKEER~1/AppData/Local/Temp/gcm/p34`,
  outside the repository. Copy the helpers `count.py`, `extract.py`,
  `insert_after.py`, `replace_once.py` and `append.py` from `…/p33` before
  Task 1. Long output goes to a file there; read its tail.
- **Extract a task's code from its brief** with `extract.py BRIEF MARKER
  OUT`. It writes the first fenced block after the line holding MARKER,
  dedented. Never retype plan code.
- **Counting a full run:** `python -X utf8 $P/count.py <FILE>`. It prints
  `N result lines: P passed, F failed, I ignored`.
- **Line endings.** The working tree is mostly CRLF (`core.autocrlf=true`),
  and the index LF.
  - A Python edit keeps the file's own line ending.
  - New files may be LF.
  - `.github/scripts/*.sh` must stay LF.
- **Write scripts and commit messages with the Write tool, never a Bash
  heredoc.** The Bash tool turns `\\` into `\`, and this plan's Rust has
  many backslashes. Commit with `git commit -F $P/msg-<n>.txt`, then check
  `git log -1 --format=%s`.
- **A new dependency line in `Cargo.lock`.** `cargo test --locked` refuses
  to write it. Run `cargo check -p <crate> --offline` once (Tasks 7 and
  8: without `--offline`, since they download new crates), read `git diff
  Cargo.lock`, then run the locked tests.
- **Format before every commit:** run `cargo fmt --all`, then stage what it
  changed.
- **Chain a commit and what follows it with `&&`, never `;`.**
- **Port 3000 must be free for a full Windows suite.**
  - Check with `netstat -ano | grep -E "[:.]3000 .*LISTENING"`.
  - **Never stop a process that holds it: stop and ask the user.**
- **Linux runs** use `bash /c/Users/SAKEER~1/AppData/Local/Temp/gcm/linux/run.sh <cargo args>`.
  It exports tracked files, so `git add` new files first. Docker must be
  running.
- **Mutants run only on committed work,** and are undone with `git checkout
  -- <file>`.
- **Expected outputs are exact.** If a step's output differs:
  - when the code does what this plan describes and the expected value is
    wrong, correct the value and ledger a ruling;
  - when the code does something else, fix the code.
- **A test written after its code, on purpose,** is marked *guard* in its
  comment, and is expected to pass on its first run.
- **Names from the existing code** are quoted as they stood at `77c2ee5`.
  If a name differs, follow the code and ledger the difference.

---

### Task 1: `$NOVA_HOME`, the canonical index form, and the cache's name

Spec §3.3 and §5.1.

**Files:**
- Create: `crates/nova-pm/src/home.rs`, `crates/nova-pm/src/index_name.rs`
- Modify: `crates/nova-pm/src/lib.rs`, `crates/nova-pm/Cargo.toml` (`crc32fast`)
- Modify: `crates/nova-driver/src/runtime_cache.rs` (`cache_root` uses `nova_pm::nova_home`)
- Create: `crates/nova-pm/tests/home.rs`

**Interfaces:**
- Consumes: `nova_pm::real_path`.
- Produces:
  - `nova_pm::nova_home(Option<PathBuf>, Option<PathBuf>, Option<PathBuf>, bool) -> Option<PathBuf>`;
  - `nova_pm::nova_home_from_env() -> Option<PathBuf>`;
  - `nova_pm::registry_dir() -> Option<PathBuf>`;
  - `nova_pm::canonical_index(&str) -> Result<String, String>`;
  - `nova_pm::local_index_path(&str) -> Option<PathBuf>`;
  - `nova_pm::index_dir_name(&str) -> String`.

- [ ] **Step 1: Write the failing tests**

`crates/nova-pm/tests/home.rs`:

```rust
//! `$NOVA_HOME`, the canonical index form, and the cache directory's name
//! (spec
//! `docs/superpowers/specs/2026-10-09-phase-3-3b-index-and-publishing-design.md`
//! §3.3, §5.1).

use std::path::PathBuf;

use nova_pm::{canonical_index, index_dir_name, local_index_path, nova_home};

#[test]
fn nova_home_is_the_variable_or_dot_nova_in_the_home_directory() {
    let p = |s: &str| Some(PathBuf::from(s));
    assert_eq!(nova_home(p("/n"), p("C:/u"), p("/h"), true), p("/n"));
    // An empty value counts as unset.
    assert_eq!(
        nova_home(p(""), p("C:/u"), p("/h"), true),
        Some(PathBuf::from("C:/u").join(".nova"))
    );
    assert_eq!(
        nova_home(None, p("C:/u"), p("/h"), false),
        Some(PathBuf::from("/h").join(".nova"))
    );
    assert_eq!(nova_home(None, None, p("/h"), true), None);
    assert_eq!(nova_home(None, None, None, false), None);
}

#[test]
fn an_https_index_has_one_canonical_form() {
    let canonical = canonical_index("HTTPS://Raw.Example.COM:443/Owner/Index/main").unwrap();
    assert_eq!(canonical, "https://raw.example.com/Owner/Index/main/");
    assert_eq!(
        canonical_index("https://raw.example.com/Owner/Index/main/").unwrap(),
        canonical
    );
    assert_eq!(
        canonical_index("https://raw.example.com:8443/x").unwrap(),
        "https://raw.example.com:8443/x/"
    );
    assert_eq!(canonical_index("https://example.com").unwrap(), "https://example.com/");
}

#[test]
fn plain_http_is_only_for_a_loopback_ip() {
    assert_eq!(
        canonical_index("http://127.0.0.1:8080/i").unwrap(),
        "http://127.0.0.1:8080/i/"
    );
    assert_eq!(canonical_index("http://[::1]:9/").unwrap(), "http://[::1]:9/");
    for refused in [
        "http://example.com/",
        "http://localhost:8080/",
        "https://user@example.com/",
        "https://example.com/?q=1",
    ] {
        assert!(canonical_index(refused).is_err(), "{refused}");
    }
}

#[test]
fn a_local_index_is_its_real_path_whatever_its_spelling() {
    let dir = std::env::temp_dir().join("nova-pm-index name");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let canonical = canonical_index(dir.to_str().unwrap()).unwrap();
    assert!(canonical.starts_with("file:///"), "{canonical}");
    assert!(canonical.ends_with("/nova-pm-index name/"), "{canonical}");
    assert!(!canonical.contains('\\'), "{canonical}");
    // The same directory as a file: URL, its space escaped.
    let real = nova_pm::real_path(&dir);
    let slashed = real.to_string_lossy().replace('\\', "/");
    let url = if slashed.starts_with('/') {
        format!("file://{slashed}")
    } else {
        format!("file:///{slashed}")
    };
    assert_eq!(canonical_index(&url.replace(' ', "%20")).unwrap(), canonical);
    let back = local_index_path(&canonical).expect("a local index");
    assert_eq!(nova_pm::real_path(&back), real);
    assert_eq!(local_index_path("https://example.com/"), None);
}

#[cfg(windows)]
#[test]
fn a_windows_drive_letter_is_upper_case_in_the_canonical_form() {
    let dir = std::env::temp_dir().join("nova-pm-index-drive");
    std::fs::create_dir_all(&dir).unwrap();
    let real = nova_pm::real_path(&dir).to_string_lossy().replace('\\', "/");
    let lower = format!("file:///{}{}", real[..1].to_ascii_lowercase(), &real[1..]);
    let canonical = canonical_index(&lower).unwrap();
    assert_eq!(&canonical[8..9], real[..1].to_ascii_uppercase());
    assert_eq!(canonical_index(&real.replace('/', "\\")).unwrap(), canonical);
}

#[test]
fn a_missing_local_index_keeps_its_canonical_form() {
    // A build with the index gone must still match nova.lock's `index`
    // (spec §3.3), whatever the temp directory's spelling: an 8.3 name on
    // Windows, `/var` for `/private/var` on macOS.
    let dir = std::env::temp_dir().join("nova-pm-index-missing");
    std::fs::create_dir_all(&dir).unwrap();
    let present = canonical_index(dir.to_str().unwrap()).unwrap();
    std::fs::remove_dir_all(&dir).unwrap();
    let absent = canonical_index(dir.to_str().unwrap()).unwrap();
    assert_eq!(absent, present);
    let deeper = canonical_index(dir.join("a").join("b").to_str().unwrap()).unwrap();
    assert_eq!(deeper, format!("{present}a/b/"));
}

#[test]
fn a_relative_path_is_not_an_index() {
    assert!(canonical_index("some/dir").is_err());
    assert!(canonical_index("file:relative/dir").is_err());
    assert!(canonical_index("file://elsewhere/srv/index").is_err());
}

#[test]
fn the_cache_directory_name_is_the_host_and_a_crc() {
    let name = index_dir_name("https://raw.githubusercontent.com/Sakeerin/nova-index/main/");
    assert!(name.starts_with("raw.githubusercontent.com-"), "{name}");
    assert_eq!(name.len(), "raw.githubusercontent.com-".len() + 8);
    let ipv6 = index_dir_name("http://[::1]:9/");
    assert!(!ipv6.contains([':', '[', ']']), "{ipv6}");
    assert!(index_dir_name("file:///srv/index/").starts_with("local-"));
    assert_ne!(
        index_dir_name("https://a.example/x/"),
        index_dir_name("https://a.example/y/")
    );
}
```

- [ ] **Step 2: Run them to verify they fail**

```bash
cd /d/Projects/nona/nova && cargo test --locked -p nova-pm --test home 2>&1 | grep -E "^error" | head -3
```

Expected: `error[E0432]: unresolved imports` naming `canonical_index`,
`index_dir_name`, `local_index_path` and `nova_home`.

- [ ] **Step 3: Implement**

`crates/nova-pm/src/home.rs`:

```rust
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
```

`crates/nova-pm/src/index_name.rs`:

```rust
//! An index's canonical form, and its cache directory's name (spec
//! `docs/superpowers/specs/2026-10-09-phase-3-3b-index-and-publishing-design.md`
//! §3.3 and §5.1).

use std::path::{Path, PathBuf};

use crate::real_path;

/// The canonical form of an index's location: how indexes are compared,
/// and what `nova.lock` and the cache are keyed by.
///
/// - `https://…`, or `http://` on `127.0.0.1` or `[::1]` only: the scheme
///   and host in lower case, no default port, one `/` at the end.
/// - A `file:` URL or an absolute path: the directory's real path, as
///   `file:///` and the path with `/` separators and an upper-case drive
///   letter, with one `/` at the end.
pub fn canonical_index(location: &str) -> Result<String, String> {
    let location = location.trim();
    let lower = location.to_ascii_lowercase();
    if lower.starts_with("https://") || lower.starts_with("http://") {
        return canonical_http(location);
    }
    if lower.starts_with("file:") {
        let path = file_url_path(&location[5..])?;
        return Ok(canonical_local(&path));
    }
    let path = Path::new(location);
    if path.is_absolute() {
        return Ok(canonical_local(path));
    }
    Err(format!(
        "`{location}` is not an index: give an https:// URL, a file: URL or an absolute path"
    ))
}

/// The directory a canonical `file:` index names; `None` for an HTTP one.
pub fn local_index_path(canonical: &str) -> Option<PathBuf> {
    let rest = canonical.strip_prefix("file://")?.trim_end_matches('/');
    Some(PathBuf::from(strip_drive_slash(rest)))
}

/// The cache directory's name for a canonical index (spec §5.1): its host
/// without a port, keeping only `a-z`, `0-9`, `.` and `-` (`local` for a
/// local index), then `-` and the CRC-32 of the canonical form.
pub fn index_dir_name(canonical: &str) -> String {
    let host = match canonical.split_once("://") {
        Some((scheme, rest)) if scheme != "file" => {
            let authority = rest.split('/').next().unwrap_or("");
            split_port(authority)
                .0
                .chars()
                .filter(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || *c == '.' || *c == '-')
                .collect()
        }
        _ => "local".to_string(),
    };
    format!("{host}-{:08x}", crc32fast::hash(canonical.as_bytes()))
}

fn canonical_http(url: &str) -> Result<String, String> {
    let (scheme, rest) = url.split_once("://").expect("the caller checked the scheme");
    let scheme = scheme.to_ascii_lowercase();
    if rest.contains(['?', '#']) {
        return Err(format!("`{url}`: an index URL has no query or fragment"));
    }
    let (authority, path) = match rest.find('/') {
        Some(i) => (&rest[..i], &rest[i..]),
        None => (rest, "/"),
    };
    if authority.is_empty() || authority.contains('@') {
        return Err(format!("`{url}`: an index URL needs a host, and no user name"));
    }
    let authority = authority.to_ascii_lowercase();
    let (host, port) = split_port(&authority);
    if scheme == "http" && host != "127.0.0.1" && host != "[::1]" {
        return Err(format!(
            "`{url}`: plain http:// is allowed only for 127.0.0.1 and [::1]; use https://"
        ));
    }
    let default = if scheme == "https" { "443" } else { "80" };
    let authority = match port {
        Some(port) if port != default => format!("{host}:{port}"),
        _ => host.to_string(),
    };
    Ok(format!(
        "{scheme}://{authority}{}/",
        path.trim_end_matches('/')
    ))
}

/// An authority's host and port. An IPv6 host keeps its brackets.
fn split_port(authority: &str) -> (&str, Option<&str>) {
    if let Some(end) = authority.rfind(']') {
        return match authority[end + 1..].strip_prefix(':') {
            Some(port) => (&authority[..=end], Some(port)),
            None => (authority, None),
        };
    }
    match authority.rsplit_once(':') {
        Some((host, port)) => (host, Some(port)),
        None => (authority, None),
    }
}

/// The local path a `file:` URL's remainder names (RFC 8089): `//` with an
/// empty or `localhost` host dropped, percent-escapes decoded, and on
/// Windows the `/` before a drive letter dropped.
fn file_url_path(rest: &str) -> Result<PathBuf, String> {
    let path = match rest.strip_prefix("//") {
        Some(after) => {
            let (host, path) = match after.find('/') {
                Some(i) => (&after[..i], &after[i..]),
                None => (after, ""),
            };
            if !host.is_empty() && !host.eq_ignore_ascii_case("localhost") {
                return Err(format!(
                    "file://{after}: a file: index must be a directory on this machine"
                ));
            }
            path
        }
        None => rest,
    };
    let decoded = percent_decode(path)?;
    let path = strip_drive_slash(&decoded).to_string();
    if !Path::new(&path).is_absolute() {
        return Err(format!("file:{rest} does not name an absolute path"));
    }
    Ok(PathBuf::from(path))
}

/// `/C:/x` as `C:/x`; anything else unchanged.
fn strip_drive_slash(path: &str) -> &str {
    let bytes = path.as_bytes();
    if bytes.len() >= 3 && bytes[0] == b'/' && bytes[1].is_ascii_alphabetic() && bytes[2] == b':' {
        &path[1..]
    } else {
        path
    }
}

fn percent_decode(text: &str) -> Result<String, String> {
    let bytes = text.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' {
            let byte = text
                .get(i + 1..i + 3)
                .filter(|hex| hex.bytes().all(|b| b.is_ascii_hexdigit()))
                .and_then(|hex| u8::from_str_radix(hex, 16).ok());
            let Some(byte) = byte else {
                return Err(format!("`{text}` has a bad percent-escape"));
            };
            out.push(byte);
            i += 3;
        } else {
            out.push(bytes[i]);
            i += 1;
        }
    }
    String::from_utf8(out).map_err(|_| format!("`{text}` is not UTF-8 once decoded"))
}

/// The real path of `path`'s longest existing ancestor, with the rest
/// appended. A local index that does not exist, or no longer does, keeps
/// the canonical form it has when it does: Windows' 8.3 names and macOS's
/// `/var` for `/private/var` are resolved either way.
fn real_prefix(path: &Path) -> PathBuf {
    let mut existing = path;
    let mut rest = Vec::new();
    while !existing.exists() {
        match (existing.parent(), existing.file_name()) {
            (Some(parent), Some(name)) => {
                rest.push(name);
                existing = parent;
            }
            _ => break,
        }
    }
    let mut real = real_path(existing);
    for name in rest.iter().rev() {
        real.push(name);
    }
    real
}

/// A local directory's canonical form.
fn canonical_local(path: &Path) -> String {
    let mut text = real_prefix(path).to_string_lossy().replace('\\', "/");
    // An upper-case drive letter, so `c:` and `C:` are one index.
    if text.as_bytes().get(1) == Some(&b':') {
        if let Some(drive) = text.get_mut(..1) {
            drive.make_ascii_uppercase();
        }
    }
    let text = text.trim_end_matches('/');
    if text.starts_with('/') {
        format!("file://{text}/")
    } else {
        format!("file:///{text}/")
    }
}
```

In `crates/nova-pm/src/lib.rs`:
- add `mod home;` and `mod index_name;` after `mod graph;`;
- add, after the `pub use graph::…;` line:

  ```rust
  pub use home::{nova_home, nova_home_from_env, registry_dir};
  pub use index_name::{canonical_index, index_dir_name, local_index_path};
  ```

In `crates/nova-pm/Cargo.toml`, add after `nova-lexer = …`:

```toml
crc32fast = { workspace = true }
```

In `crates/nova-driver/src/runtime_cache.rs`, `cache_root`'s body becomes:

```rust
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
```

Its doc comment's sentence on the defaults stays; add "The rule is
`nova_pm::nova_home`'s (spec 3.3b §5.1)."

- [ ] **Step 4: Run the tests to verify they pass**

```bash
cd /d/Projects/nona/nova && P=/c/Users/SAKEER~1/AppData/Local/Temp/gcm/p34 && cargo check -p nova-pm --offline > /dev/null 2>&1; git diff --stat -- Cargo.lock; cargo test --locked -p nova-pm -p nova-driver > $P/t1.txt 2>&1; echo "exit=$?"; python -X utf8 $P/count.py $P/t1.txt; grep -E "FAILED|panicked|^warning|^error" $P/t1.txt | head
```

Expected:
- `Cargo.lock` changes by one line, `crc32fast` in `nova-pm`'s entry;
- `exit=0`, 0 failed, no warnings, the new tests included (8 on
  Windows, 7 elsewhere);
- the runtime cache's four `cache_root` tests pass unchanged.

- [ ] **Step 5: Commit**

Write `$P/msg-1.txt`:

```
nova-pm: $NOVA_HOME, the canonical index form, and the cache's name

For the package index (spec
docs/superpowers/specs/2026-10-09-phase-3-3b-index-and-publishing-design.md
§3.3, §5.1):
- `nova_home` is the $NOVA_HOME rule, moved here from the driver's
  runtime cache, which now uses it and keeps its own message.
  `registry_dir` is $NOVA_HOME/registry.
- `canonical_index` gives an index one form: an https:// URL with its
  scheme and host in lower case, no default port and one trailing `/`
  (http:// only on a loopback IP), or a local directory's real path as a
  file:/// URL with an upper-case drive letter. The real path is taken
  from the longest part that exists, so a local index that is missing
  keeps its form. `local_index_path` turns a local one back into a
  directory.
- `index_dir_name` names an index's cache directory: its host and the
  CRC-32 of its canonical form, so `[::1]:8080` or a drive letter never
  reaches a directory name. nova-pm gains crc32fast, already locked.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
```

```bash
cd /d/Projects/nona/nova && P=/c/Users/SAKEER~1/AppData/Local/Temp/gcm/p34 && cargo fmt --all && git add crates/nova-pm crates/nova-driver Cargo.lock && git commit -q -F $P/msg-1.txt && git log -1 --format=%s
```

Expected: `nova-pm: $NOVA_HOME, the canonical index form, and the cache's name`.

The task's test command: `cargo test --locked -p nova-pm -p nova-driver`.

---

### Task 2: `nova.lock`

Spec §4.6.

**Files:**
- Create: `crates/nova-pm/src/lock.rs`, `crates/nova-pm/tests/lock.rs`
- Modify: `crates/nova-pm/src/lib.rs`

**Interfaces:**
- Consumes: nothing new.
- Produces:
  - `nova_pm::LOCKFILE: &str` (`"nova.lock"`);
  - `nova_pm::Lock { index: String, packages: Vec<LockedPackage> }`, with
    `find(&self, &str) -> Option<&LockedPackage>` and `to_text(&self) -> String`;
  - `nova_pm::LockedPackage { name: String, version: semver::Version, checksum: String, dependencies: Vec<String> }`;
  - `nova_pm::parse_lock(&str, FileId) -> Result<Lock, Diagnostic>` (M0016).

- [ ] **Step 1: Write the failing tests**

`crates/nova-pm/tests/lock.rs`:

```rust
//! `nova.lock` (spec
//! `docs/superpowers/specs/2026-10-09-phase-3-3b-index-and-publishing-design.md`
//! §4.6).

use nova_diagnostics::{render, FileDb};
use nova_pm::{parse_lock, Lock, LockedPackage};
use semver::Version;

fn lock() -> Lock {
    Lock {
        index: "https://example.test/index/".into(),
        packages: vec![
            LockedPackage {
                name: "geom".into(),
                version: Version::new(0, 2, 0),
                checksum: "a".repeat(64),
                dependencies: vec!["json-api".into()],
            },
            LockedPackage {
                name: "json-api".into(),
                version: Version::parse("1.4.1").unwrap(),
                checksum: "b".repeat(64),
                dependencies: vec![],
            },
        ],
    }
}

#[test]
fn a_lock_round_trips_through_its_text() {
    let text = lock().to_text();
    assert!(
        text.starts_with(
            "# Written by nova. Commit it for a program.\nversion = 1\n\
             index = \"https://example.test/index/\"\n"
        ),
        "{text}"
    );
    assert!(
        text.contains("\n[[package]]\nname = \"geom\"\nversion = \"0.2.0\"\n"),
        "{text}"
    );
    assert!(text.contains("dependencies = [\"json-api\"]\n"), "{text}");
    assert!(text.contains("dependencies = []\n"), "{text}");
    assert!(!text.contains('\r'));
    let mut db = FileDb::new();
    let file = db.add("nova.lock", text.as_str());
    assert_eq!(parse_lock(&text, file).unwrap(), lock());
}

#[test]
fn packages_are_found_by_name() {
    let lock = lock();
    assert_eq!(lock.find("geom").unwrap().version, Version::new(0, 2, 0));
    assert!(lock.find("http").is_none());
}

#[test]
fn packages_come_back_sorted_by_name() {
    let text = "version = 1\nindex = \"x\"\n\n[[package]]\nname = \"zeta\"\nversion = \"1.0.0\"\n\
                checksum = \"c\"\n\n[[package]]\nname = \"alpha\"\nversion = \"1.0.0\"\nchecksum = \"c\"\n";
    let mut db = FileDb::new();
    let file = db.add("nova.lock", text);
    let lock = parse_lock(text, file).unwrap();
    let names: Vec<&str> = lock.packages.iter().map(|p| p.name.as_str()).collect();
    assert_eq!(names, ["alpha", "zeta"]);
}

#[test]
fn an_unreadable_lock_is_m0016() {
    for (text, says) in [
        ("version = 2\nindex = \"x\"\n", "its `version` is not 1"),
        ("version = 1\n", "it has no `index`"),
        (
            "version = 1\nindex = \"x\"\n[[package]]\nname = \"geom\"\n",
            "lacks its name, version or checksum",
        ),
        (
            "version = 1\nindex = \"x\"\n[[package]]\nname = \"geom\"\nversion = \"one\"\nchecksum = \"c\"\n",
            "version `one`",
        ),
        ("this is not toml = [", "nova.lock cannot be read"),
    ] {
        let mut db = FileDb::new();
        let file = db.add("nova.lock", text);
        let diagnostic = parse_lock(text, file).unwrap_err();
        assert_eq!(diagnostic.code, "M0016", "{text}");
        let rendered = render::render_to_string(&db, &[diagnostic]);
        assert!(rendered.contains(says), "{text}: {rendered}");
        assert!(rendered.contains("run `nova update`"), "{rendered}");
    }
}
```

- [ ] **Step 2: Run them to verify they fail**

```bash
cd /d/Projects/nona/nova && cargo test --locked -p nova-pm --test lock 2>&1 | grep -E "^error" | head -3
```

Expected: `error[E0432]: unresolved imports` naming `parse_lock`, `Lock`
and `LockedPackage`.

- [ ] **Step 3: Implement**

`crates/nova-pm/src/lock.rs`:

```rust
//! `nova.lock` (spec
//! `docs/superpowers/specs/2026-10-09-phase-3-3b-index-and-publishing-design.md`
//! §4.6): the registry packages a project builds with, and the index they
//! came from.

use std::ops::Range;

use nova_diagnostics::{Diagnostic, FileId, Span};
use semver::Version;
use toml_edit::{ImDocument, Item};

/// The lockfile's name, beside `nova.toml`.
pub const LOCKFILE: &str = "nova.lock";

/// A project's `nova.lock`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Lock {
    /// The canonical form of the index its packages came from.
    pub index: String,
    /// Sorted by name.
    pub packages: Vec<LockedPackage>,
}

/// One registry package of a [`Lock`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LockedPackage {
    pub name: String,
    pub version: Version,
    /// The SHA-256 of its tarball, 64 lower-case hex digits.
    pub checksum: String,
    /// The names of its dependencies, sorted.
    pub dependencies: Vec<String>,
}

impl Lock {
    pub fn find(&self, name: &str) -> Option<&LockedPackage> {
        self.packages.iter().find(|p| p.name == name)
    }

    /// The lock as nova writes it, with `\n` line endings.
    pub fn to_text(&self) -> String {
        let mut out = String::from("# Written by nova. Commit it for a program.\nversion = 1\n");
        out.push_str(&format!("index = {}\n", quote(&self.index)));
        for package in &self.packages {
            out.push_str("\n[[package]]\n");
            out.push_str(&format!("name = {}\n", quote(&package.name)));
            out.push_str(&format!(
                "version = {}\n",
                quote(&package.version.to_string())
            ));
            out.push_str(&format!("checksum = {}\n", quote(&package.checksum)));
            let names: Vec<String> = package.dependencies.iter().map(|n| quote(n)).collect();
            out.push_str(&format!("dependencies = [{}]\n", names.join(", ")));
        }
        out
    }
}

/// `text` as a TOML basic string.
fn quote(text: &str) -> String {
    let mut out = String::from("\"");
    for c in text.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            c if c.is_control() => out.push_str(&format!("\\u{:04X}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

/// Parse `text`, the contents of a `nova.lock` that a `FileDb` holds as
/// `file`. Anything a lock written by nova would not hold is M0016.
pub fn parse_lock(text: &str, file: FileId) -> Result<Lock, Diagnostic> {
    let bad = |message: &str, at: Option<Range<usize>>| {
        let at = at.unwrap_or(0..0);
        Diagnostic::error("M0016", format!("nova.lock cannot be read: {message}"))
            .with_primary_label(Span::new(at.start as u32, at.end as u32, file), "here")
            .with_note("delete nova.lock, or run `nova update`")
    };
    let document = ImDocument::parse(text).map_err(|e| bad(e.message().trim(), e.span()))?;
    let table = document.as_table();
    if table.get("version").and_then(Item::as_integer) != Some(1) {
        return Err(bad(
            "its `version` is not 1",
            table.get("version").and_then(Item::span),
        ));
    }
    let index = table
        .get("index")
        .and_then(Item::as_str)
        .ok_or_else(|| bad("it has no `index`", None))?
        .to_string();
    let mut packages = Vec::new();
    if let Some(item) = table.get("package") {
        let tables = item
            .as_array_of_tables()
            .ok_or_else(|| bad("`package` is not a list of tables", item.span()))?;
        for package in tables.iter() {
            let field = |key: &str| package.get(key).and_then(Item::as_str);
            let (Some(name), Some(version), Some(checksum)) =
                (field("name"), field("version"), field("checksum"))
            else {
                return Err(bad(
                    "a package lacks its name, version or checksum",
                    package.span(),
                ));
            };
            let version = Version::parse(version)
                .map_err(|e| bad(&format!("version `{version}`: {e}"), package.span()))?;
            let dependencies = match package.get("dependencies") {
                None => Vec::new(),
                Some(item) => item
                    .as_array()
                    .and_then(|array| {
                        array
                            .iter()
                            .map(|v| v.as_str().map(str::to_string))
                            .collect::<Option<Vec<_>>>()
                    })
                    .ok_or_else(|| bad("`dependencies` is not a list of names", item.span()))?,
            };
            packages.push(LockedPackage {
                name: name.to_string(),
                version,
                checksum: checksum.to_string(),
                dependencies,
            });
        }
    }
    packages.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(Lock { index, packages })
}
```

In `crates/nova-pm/src/lib.rs`, add `mod lock;` after `mod index_name;`,
and after the `pub use index_name::…;` line:

```rust
pub use lock::{parse_lock, Lock, LockedPackage, LOCKFILE};
```

- [ ] **Step 4: Run the tests to verify they pass**

```bash
cd /d/Projects/nona/nova && P=/c/Users/SAKEER~1/AppData/Local/Temp/gcm/p34 && cargo test --locked -p nova-pm > $P/t2.txt 2>&1; echo "exit=$?"; python -X utf8 $P/count.py $P/t2.txt; grep -E "FAILED|panicked|^warning|^error" $P/t2.txt | head
```

Expected: `exit=0`, 0 failed, no warnings, the four new tests included.

If toml_edit names `Item::span` or `TomlError::span` differently, follow
the names `crates/nova-pm/src/manifest.rs` already uses, and ledger it.

- [ ] **Step 5: Commit**

Write `$P/msg-2.txt`:

```
nova-pm: nova.lock

`Lock` is a project's nova.lock (spec
docs/superpowers/specs/2026-10-09-phase-3-3b-index-and-publishing-design.md
§4.6): the canonical index its packages came from, and one entry per
registry package with its version, checksum and dependencies' names,
sorted by name. `to_text` writes it with `\n` endings; `parse_lock` reads
it back, and anything a lock written by nova would not hold is M0016,
"delete nova.lock, or run `nova update`".

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
```

```bash
cd /d/Projects/nona/nova && P=/c/Users/SAKEER~1/AppData/Local/Temp/gcm/p34 && cargo fmt --all && git add crates/nova-pm && git commit -q -F $P/msg-2.txt && git log -1 --format=%s
```

Expected: `nova-pm: nova.lock`.

The task's test command: `cargo test --locked -p nova-pm`.

---

### Task 3: The resolver

Spec §4.1–§4.5.

**Files:**
- Create: `crates/nova-pm/src/resolve.rs`, `crates/nova-pm/tests/resolve.rs`
- Modify: `crates/nova-pm/src/lib.rs`

**Interfaces:**
- Consumes: Task 2's `Lock` and `LockedPackage`.
- Produces:
  - `nova_pm::Candidate { version: Version, deps: Vec<(String, VersionReq)>, checksum: String }`;
  - `nova_pm::IndexView`, a trait with
    `fn versions(&mut self, name: &str) -> Result<Option<Vec<Candidate>>, String>`;
  - `nova_pm::Requirement { name: String, req: VersionReq, by: String, span: Span }`;
  - `nova_pm::Unlock::{Nothing, All, One(String)}`;
  - `nova_pm::ResolveError::{Diagnostic(Diagnostic), Index(String)}`;
  - `nova_pm::resolve(&[Requirement], Option<&Lock>, &Unlock, &mut dyn IndexView) -> Result<Vec<LockedPackage>, ResolveError>`.

- [ ] **Step 1: Write the failing tests**

`crates/nova-pm/tests/resolve.rs`:

```rust
//! The resolver (spec
//! `docs/superpowers/specs/2026-10-09-phase-3-3b-index-and-publishing-design.md`
//! §4).

use std::collections::HashMap;

use nova_diagnostics::{FileId, Span};
use nova_pm::{
    resolve, Candidate, IndexView, Lock, LockedPackage, Requirement, ResolveError, Unlock,
};
use semver::{Version, VersionReq};

/// An index held in memory: each name's versions.
struct Fake(HashMap<&'static str, Vec<Candidate>>);

impl IndexView for Fake {
    fn versions(&mut self, name: &str) -> Result<Option<Vec<Candidate>>, String> {
        Ok(self.0.get(name).cloned())
    }
}

fn index(entries: Vec<(&'static str, Vec<Candidate>)>) -> Fake {
    Fake(entries.into_iter().collect())
}

/// A version with its dependencies.
fn c(version: &str, deps: &[(&str, &str)]) -> Candidate {
    Candidate {
        version: Version::parse(version).unwrap(),
        deps: deps
            .iter()
            .map(|(n, r)| (n.to_string(), VersionReq::parse(r).unwrap()))
            .collect(),
        checksum: format!("sum-{version}"),
    }
}

/// A requirement the root, `app`, makes.
fn req(name: &str, r: &str) -> Requirement {
    Requirement {
        name: name.into(),
        req: VersionReq::parse(r).unwrap(),
        by: "app".into(),
        span: Span::new(0, 0, FileId::DUMMY),
    }
}

fn lock_of(pairs: &[(&str, &str)]) -> Lock {
    Lock {
        index: "https://example.test/".into(),
        packages: pairs
            .iter()
            .map(|(n, v)| LockedPackage {
                name: n.to_string(),
                version: Version::parse(v).unwrap(),
                checksum: format!("sum-{v}"),
                dependencies: vec![],
            })
            .collect(),
    }
}

fn picked(result: &[LockedPackage]) -> Vec<String> {
    result
        .iter()
        .map(|p| format!("{} {}", p.name, p.version))
        .collect()
}

fn ok(
    reqs: &[Requirement],
    lock: Option<&Lock>,
    unlock: Unlock,
    index: &mut Fake,
) -> Vec<String> {
    match resolve(reqs, lock, &unlock, index) {
        Ok(result) => picked(&result),
        Err(ResolveError::Diagnostic(d)) => panic!("{} {} {:?}", d.code, d.message, d.notes),
        Err(ResolveError::Index(e)) => panic!("{e}"),
    }
}

fn failed(
    reqs: &[Requirement],
    lock: Option<&Lock>,
    unlock: Unlock,
    index: &mut Fake,
) -> nova_diagnostics::Diagnostic {
    match resolve(reqs, lock, &unlock, index) {
        Err(ResolveError::Diagnostic(d)) => d,
        other => panic!("expected a diagnostic: {:?}", other.map(|r| picked(&r))),
    }
}

#[test]
fn picks_the_newest_matching_version() {
    let mut index = index(vec![(
        "json",
        vec![c("1.0.0", &[]), c("1.4.1", &[]), c("2.0.0", &[])],
    )]);
    assert_eq!(
        ok(&[req("json", "^1")], None, Unlock::Nothing, &mut index),
        ["json 1.4.1"]
    );
}

#[test]
fn a_zero_major_caret_stays_within_its_minor() {
    let mut index = index(vec![(
        "geom",
        vec![c("0.2.0", &[]), c("0.2.5", &[]), c("0.3.0", &[])],
    )]);
    assert_eq!(
        ok(&[req("geom", "^0.2")], None, Unlock::Nothing, &mut index),
        ["geom 0.2.5"]
    );
}

#[test]
fn a_pre_release_only_when_a_requirement_names_one() {
    let mut index = index(vec![("json", vec![c("1.0.0", &[]), c("1.1.0-beta.1", &[])])]);
    assert_eq!(
        ok(&[req("json", "^1")], None, Unlock::Nothing, &mut index),
        ["json 1.0.0"]
    );
    assert_eq!(
        ok(&[req("json", "^1.1.0-beta.1")], None, Unlock::Nothing, &mut index),
        ["json 1.1.0-beta.1"]
    );
}

#[test]
fn a_locked_version_that_fits_is_kept() {
    let mut index = index(vec![("json", vec![c("1.0.0", &[]), c("1.4.1", &[])])]);
    let lock = lock_of(&[("json", "1.0.0")]);
    assert_eq!(
        ok(&[req("json", "^1")], Some(&lock), Unlock::Nothing, &mut index),
        ["json 1.0.0"]
    );
}

#[test]
fn a_tightened_requirement_moves_only_its_package() {
    // Review Focus 1: `json = "1"` edited to `"1.4"`.
    let mut index = index(vec![
        ("json", vec![c("1.2.0", &[]), c("1.4.0", &[]), c("1.5.0", &[])]),
        ("http", vec![c("1.0.0", &[]), c("1.1.0", &[])]),
    ]);
    let lock = lock_of(&[("http", "1.0.0"), ("json", "1.2.0")]);
    assert_eq!(
        ok(
            &[req("json", "^1.4"), req("http", "^1")],
            Some(&lock),
            Unlock::Nothing,
            &mut index
        ),
        ["http 1.0.0", "json 1.5.0"]
    );
}

#[test]
fn backtracking_finds_an_older_version() {
    let mut index = index(vec![
        ("a", vec![c("1.0.0", &[("c", "^1")]), c("1.1.0", &[("c", "^2")])]),
        ("b", vec![c("1.0.0", &[("c", "^1")])]),
        ("c", vec![c("1.0.0", &[]), c("2.0.0", &[])]),
    ]);
    assert_eq!(
        ok(&[req("a", "^1"), req("b", "^1")], None, Unlock::Nothing, &mut index),
        ["a 1.0.0", "b 1.0.0", "c 1.0.0"]
    );
}

#[test]
fn an_impossible_conflict_is_m0015_naming_each_requirer() {
    let mut index = index(vec![
        ("geom", vec![c("1.0.0", &[("json", "^2")])]),
        ("json", vec![c("1.0.0", &[]), c("2.0.0", &[])]),
    ]);
    let d = failed(
        &[req("json", "^1"), req("geom", "^1")],
        None,
        Unlock::Nothing,
        &mut index,
    );
    assert_eq!(d.code, "M0015");
    assert!(
        d.message.contains("no version of `json` meets every requirement"),
        "{}",
        d.message
    );
    assert!(
        d.message.contains("`^1` from `app`") && d.message.contains("`^2` from `geom`"),
        "{}",
        d.message
    );
}

#[test]
fn a_package_the_index_lacks_is_m0014() {
    let mut index = index(vec![]);
    let d = failed(&[req("json", "^1")], None, Unlock::Nothing, &mut index);
    assert_eq!(d.code, "M0014");
    assert!(d.message.contains("the index has no package `json`"), "{}", d.message);
}

#[test]
fn updating_one_name_moves_only_it() {
    let mut index = index(vec![
        ("a", vec![c("1.0.0", &[]), c("1.5.0", &[])]),
        ("b", vec![c("1.0.0", &[]), c("1.5.0", &[])]),
    ]);
    let lock = lock_of(&[("a", "1.0.0"), ("b", "1.0.0")]);
    assert_eq!(
        ok(
            &[req("a", "^1"), req("b", "^1")],
            Some(&lock),
            Unlock::One("a".into()),
            &mut index
        ),
        ["a 1.5.0", "b 1.0.0"]
    );
}

#[test]
fn updating_one_name_says_when_the_others_block_it() {
    let mut index = index(vec![
        ("a", vec![c("1.0.0", &[("b", "^1")]), c("2.0.0", &[("b", "^2")])]),
        ("b", vec![c("1.0.0", &[]), c("2.0.0", &[])]),
    ]);
    let lock = lock_of(&[("a", "1.0.0"), ("b", "1.0.0")]);
    let d = failed(
        &[req("a", "^2")],
        Some(&lock),
        Unlock::One("a".into()),
        &mut index,
    );
    assert_eq!(d.code, "M0015");
    assert!(
        d.notes.iter().any(|n| n.contains("run `nova update`")),
        "{:?}",
        d.notes
    );
}

#[test]
fn a_plain_sync_falls_back_when_the_lock_cannot_be_kept() {
    // `d` is new, and needs a newer `b` than the one locked.
    let mut index = index(vec![
        ("b", vec![c("1.0.0", &[]), c("1.1.0", &[])]),
        ("d", vec![c("1.0.0", &[("b", "^1.1")])]),
    ]);
    let lock = lock_of(&[("b", "1.0.0")]);
    assert_eq!(
        ok(
            &[req("b", "^1"), req("d", "^1")],
            Some(&lock),
            Unlock::Nothing,
            &mut index
        ),
        ["b 1.1.0", "d 1.0.0"]
    );
}

#[test]
fn the_result_is_sorted_with_each_packages_dependencies() {
    let mut index = index(vec![
        ("geom", vec![c("1.0.0", &[("json", "^1"), ("http", "^1")])]),
        ("json", vec![c("1.0.0", &[])]),
        ("http", vec![c("1.0.0", &[])]),
    ]);
    let result = resolve(&[req("geom", "^1")], None, &Unlock::Nothing, &mut index).unwrap();
    assert_eq!(picked(&result), ["geom 1.0.0", "http 1.0.0", "json 1.0.0"]);
    assert_eq!(result[0].dependencies, ["http", "json"]);
    assert_eq!(result[0].checksum, "sum-1.0.0");
}

#[test]
fn an_index_error_is_passed_on() {
    struct Broken;
    impl IndexView for Broken {
        fn versions(&mut self, _: &str) -> Result<Option<Vec<Candidate>>, String> {
            Err("cannot reach the index".into())
        }
    }
    match resolve(&[req("json", "^1")], None, &Unlock::Nothing, &mut Broken) {
        Err(ResolveError::Index(message)) => assert_eq!(message, "cannot reach the index"),
        _ => panic!("expected the index's error"),
    }
}
```

- [ ] **Step 2: Run them to verify they fail**

```bash
cd /d/Projects/nona/nova && cargo test --locked -p nova-pm --test resolve 2>&1 | grep -E "^error" | head -3
```

Expected: `error[E0432]: unresolved imports` naming `resolve`, `Candidate`,
`IndexView`, `Requirement`, `ResolveError` and `Unlock`.

- [ ] **Step 3: Implement**

`crates/nova-pm/src/resolve.rs`:

```rust
//! The resolver (spec
//! `docs/superpowers/specs/2026-10-09-phase-3-3b-index-and-publishing-design.md`
//! §4): one version of each package name per build, caret ranges, and a
//! backtracking search that keeps locked versions.

use std::collections::{BTreeMap, HashMap};

use nova_diagnostics::{Diagnostic, Span};
use semver::{Version, VersionReq};

use crate::lock::{Lock, LockedPackage};

/// One version of a package, as the index lists it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Candidate {
    pub version: Version,
    /// Its `[dependencies]`: names and requirements.
    pub deps: Vec<(String, VersionReq)>,
    /// The SHA-256 of its tarball.
    pub checksum: String,
}

/// What the resolver asks of an index.
pub trait IndexView {
    /// Every version of `name`, or `Ok(None)` when the index has no
    /// package of that name. `Err` when the index cannot be read.
    fn versions(&mut self, name: &str) -> Result<Option<Vec<Candidate>>, String>;
}

/// A requirement on a package, and where it came from.
#[derive(Debug, Clone)]
pub struct Requirement {
    pub name: String,
    pub req: VersionReq,
    /// The package that made it, named in M0015.
    pub by: String,
    /// The root's manifest entry through which it is reached, where M0014
    /// and M0015 point (spec §4.5).
    pub span: Span,
}

/// What a resolution may change (spec §4.4).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Unlock {
    /// Keep every locked version that still fits. When they cannot all
    /// stay, resolve again as if there were no lock.
    Nothing,
    /// `nova update`: ignore the lock.
    All,
    /// `nova update <name>`: only that name may move.
    One(String),
}

/// Why a resolution failed.
#[derive(Debug)]
pub enum ResolveError {
    /// M0014 or M0015, ready to render.
    Diagnostic(Diagnostic),
    /// The index could not be read: why.
    Index(String),
}

/// The packages `requirements` need, newest versions first, keeping
/// `lock`'s versions as `unlock` says. Sorted by name, each with its
/// dependencies' names.
pub fn resolve(
    requirements: &[Requirement],
    lock: Option<&Lock>,
    unlock: &Unlock,
    index: &mut dyn IndexView,
) -> Result<Vec<LockedPackage>, ResolveError> {
    let pins = |except: Option<&str>| -> HashMap<String, Version> {
        lock.map(|lock| {
            lock.packages
                .iter()
                .filter(|p| Some(p.name.as_str()) != except)
                .map(|p| (p.name.clone(), p.version.clone()))
                .collect()
        })
        .unwrap_or_default()
    };
    let mut search = Search {
        index,
        known: HashMap::new(),
        pinned: HashMap::new(),
        strict: false,
        failure: None,
    };
    match unlock {
        Unlock::All => search.run(requirements, HashMap::new(), false),
        Unlock::One(name) => search
            .run(requirements, pins(Some(name.as_str())), true)
            .map_err(|error| match error {
                ResolveError::Diagnostic(d) if d.code == "M0015" => ResolveError::Diagnostic(
                    d.with_note("the other locked versions were kept; run `nova update` to let them move too"),
                ),
                other => other,
            }),
        Unlock::Nothing => {
            let pinned = pins(None);
            if pinned.is_empty() {
                return search.run(requirements, pinned, false);
            }
            match search.run(requirements, pinned, false) {
                Err(ResolveError::Diagnostic(_)) => {
                    search.run(requirements, HashMap::new(), false)
                }
                result => result,
            }
        }
    }
}

/// Why a search failed: a package, and every requirement on it.
struct Failure {
    name: String,
    reqs: Vec<Requirement>,
}

struct State {
    reqs: Vec<Requirement>,
    /// Package names in the order they were first reached.
    order: Vec<String>,
    chosen: BTreeMap<String, Candidate>,
}

struct Search<'a> {
    index: &'a mut dyn IndexView,
    /// Every version of each package asked for so far, newest first.
    known: HashMap<String, Vec<Candidate>>,
    /// Locked versions to keep.
    pinned: HashMap<String, Version>,
    /// Whether a pinned version that does not fit leaves no candidate
    /// (`nova update <name>`) rather than letting its package move.
    strict: bool,
    /// The first conflict met, for M0015.
    failure: Option<Failure>,
}

impl Search<'_> {
    fn run(
        &mut self,
        requirements: &[Requirement],
        pinned: HashMap<String, Version>,
        strict: bool,
    ) -> Result<Vec<LockedPackage>, ResolveError> {
        self.pinned = pinned;
        self.strict = strict;
        self.failure = None;
        let mut order: Vec<String> = requirements.iter().map(|r| r.name.clone()).collect();
        order.sort();
        order.dedup();
        let mut state = State {
            reqs: requirements.to_vec(),
            order,
            chosen: BTreeMap::new(),
        };
        if self.solve(&mut state)? {
            return Ok(state
                .chosen
                .into_iter()
                .map(|(name, candidate)| {
                    let mut dependencies: Vec<String> =
                        candidate.deps.iter().map(|(n, _)| n.clone()).collect();
                    dependencies.sort();
                    dependencies.dedup();
                    LockedPackage {
                        name,
                        version: candidate.version,
                        checksum: candidate.checksum,
                        dependencies,
                    }
                })
                .collect());
        }
        let failure = self.failure.take().expect("a failed search records why");
        Err(ResolveError::Diagnostic(conflict(&failure)))
    }

    /// Choose a version for the next unchosen package, depth first.
    fn solve(&mut self, state: &mut State) -> Result<bool, ResolveError> {
        let Some(name) = state
            .order
            .iter()
            .find(|n| !state.chosen.contains_key(*n))
            .cloned()
        else {
            return Ok(true);
        };
        let reqs: Vec<Requirement> = state
            .reqs
            .iter()
            .filter(|r| r.name == name)
            .cloned()
            .collect();
        let fitting: Vec<Candidate> = self
            .versions(&name, &reqs)?
            .into_iter()
            .filter(|c| reqs.iter().all(|r| r.req.matches(&c.version)))
            .collect();
        let kept = self
            .pinned
            .get(&name)
            .map(|version| fitting.iter().position(|c| &c.version == version));
        let tried = match kept {
            // A locked version that still fits is the only candidate.
            Some(Some(i)) => vec![fitting[i].clone()],
            Some(None) if self.strict => Vec::new(),
            _ => fitting,
        };
        let via = reqs[0].span;
        for candidate in tried {
            let reqs_before = state.reqs.len();
            let order_before = state.order.len();
            let mut new_names = Vec::new();
            for (dep, req) in &candidate.deps {
                state.reqs.push(Requirement {
                    name: dep.clone(),
                    req: req.clone(),
                    by: name.clone(),
                    span: via,
                });
                if !state.order.contains(dep) && !new_names.contains(dep) {
                    new_names.push(dep.clone());
                }
            }
            new_names.sort();
            state.order.extend(new_names);
            // A new requirement on a package already chosen must fit it.
            let clash = candidate.deps.iter().find(|(dep, req)| {
                state
                    .chosen
                    .get(dep)
                    .is_some_and(|chosen| !req.matches(&chosen.version))
            });
            match clash {
                Some((dep, _)) => {
                    let on_dep = state.reqs.iter().filter(|r| &r.name == dep).cloned().collect();
                    self.note(dep, on_dep);
                }
                None => {
                    state.chosen.insert(name.clone(), candidate.clone());
                    if self.solve(state)? {
                        return Ok(true);
                    }
                    state.chosen.remove(&name);
                }
            }
            state.order.truncate(order_before);
            state.reqs.truncate(reqs_before);
        }
        self.note(&name, reqs);
        Ok(false)
    }

    /// Every version of `name`, newest first; M0014 when there is none.
    fn versions(
        &mut self,
        name: &str,
        reqs: &[Requirement],
    ) -> Result<Vec<Candidate>, ResolveError> {
        if !self.known.contains_key(name) {
            let Some(mut versions) = self.index.versions(name).map_err(ResolveError::Index)?
            else {
                return Err(ResolveError::Diagnostic(
                    Diagnostic::error("M0014", format!("the index has no package `{name}`"))
                        .with_primary_label(reqs[0].span, "required here"),
                ));
            };
            versions.sort_by(|a, b| b.version.cmp(&a.version));
            self.known.insert(name.to_string(), versions);
        }
        Ok(self.known[name].clone())
    }

    /// Remember the first conflict met.
    fn note(&mut self, name: &str, reqs: Vec<Requirement>) {
        if self.failure.is_none() {
            self.failure = Some(Failure {
                name: name.to_string(),
                reqs,
            });
        }
    }
}

/// M0015: no version of a package meets every requirement on it.
fn conflict(failure: &Failure) -> Diagnostic {
    let wants: Vec<String> = failure
        .reqs
        .iter()
        .map(|r| format!("`{}` from `{}`", r.req, r.by))
        .collect();
    Diagnostic::error(
        "M0015",
        format!(
            "no version of `{}` meets every requirement: {}",
            failure.name,
            wants.join(", ")
        ),
    )
    .with_primary_label(failure.reqs[0].span, "required here")
}
```

In `crates/nova-pm/src/lib.rs`, add `mod resolve;` after `mod project;`,
and after the `pub use project::…;` line:

```rust
pub use resolve::{resolve, Candidate, IndexView, Requirement, ResolveError, Unlock};
```

- [ ] **Step 4: Run the tests to verify they pass**

```bash
cd /d/Projects/nona/nova && P=/c/Users/SAKEER~1/AppData/Local/Temp/gcm/p34 && cargo test --locked -p nova-pm > $P/t3.txt 2>&1; echo "exit=$?"; python -X utf8 $P/count.py $P/t3.txt; grep -E "FAILED|panicked|^warning|^error" $P/t3.txt | head
```

Expected: `exit=0`, 0 failed, no warnings, the 13 new tests included.

- [ ] **Step 5: Commit**

Write `$P/msg-3.txt`:

```
nova-pm: the resolver

`resolve` picks one version of each package name (spec
docs/superpowers/specs/2026-10-09-phase-3-3b-index-and-publishing-design.md
§4): caret ranges through semver, a pre-release only when a requirement
names one, and a depth-first search that backtracks on a conflict.
Packages are taken in the order they are first reached, ties broken by
name, so the same inputs give the same versions.

Locked versions are kept: one that still fits is the only candidate. A
plain sync that cannot keep them all resolves again with no lock;
`nova update <name>` keeps the others and says to run `nova update` when
they block it. M0014 is a package the index lacks; M0015 lists each
requirement on a package no version meets, and who made it.

The index is behind `IndexView`, so the resolver never touches the
network.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
```

```bash
cd /d/Projects/nona/nova && P=/c/Users/SAKEER~1/AppData/Local/Temp/gcm/p34 && cargo fmt --all && git add crates/nova-pm && git commit -q -F $P/msg-3.txt && git log -1 --format=%s
```

Expected: `nova-pm: the resolver`.

The task's test command: `cargo test --locked -p nova-pm`.

---

### Task 4: The offline graph

Spec §4.1, §4.5, §5.4, §10.2.

**Files:**
- Modify: `crates/nova-pm/src/graph.rs` (replaced whole, below), `crates/nova-pm/src/lib.rs`
- Modify: `crates/nova-pm/tests/graph.rs:118-130` (`a_version_only_entry_is_m0005_and_names_the_dependency`)
- Create: `crates/nova-pm/tests/registry_graph.rs`

**Interfaces:**
- Consumes:
  - Task 1's `registry_dir` and `index_dir_name`;
  - Task 2's `Lock`, `parse_lock` and `LOCKFILE`;
  - Task 3's `Requirement`.
- Produces:
  - `GraphPackage.registry: bool`;
  - `nova_pm::Offline { registry: Option<PathBuf>, lock: Option<Lock>, dev: bool }`, with
    `Offline::from_env() -> Offline` (`dev: true`, `lock: None`);
  - `nova_pm::graph_with(&Path, Option<&str>, &Offline, &mut FileDb) -> (Option<Graph>, Vec<Diagnostic>)`.
    `graph` and `graph_from` keep their signatures and call it with
    `Offline::from_env()`;
  - `nova_pm::requirements(&Path, Option<&str>, bool, &mut FileDb) -> (Vec<Requirement>, Vec<Diagnostic>)`.
- The M0005 messages, exactly:
  - "dependency `json` is not downloaded yet; run `nova fetch`";
  - "dependency `json` is locked at 1.4.1, which does not meet `^2`; run `nova fetch`";
  - "dependency `json` cannot be found: cannot place the package cache:
    set NOVA_HOME to a writable directory";
  - "the downloaded copy of `json` 1.4.1 does not match nova.lock; delete
    <dir> and run `nova fetch`".
- M0017 in the graph: "downloaded package `json` 1.4.1 has a path
  dependency `x`".

The order of the checks on a registry entry is:
1. the lock;
2. the locked package;
3. its fit;
4. the registry directory;
5. the unpacked copy.

So a project with no `nova.lock` gets "not downloaded yet" whatever the
environment. The lock is read lazily, at the first registry entry, so a
project with no registry entry never reads one.

- [ ] **Step 1: Write the failing tests**

`crates/nova-pm/tests/registry_graph.rs`:

```rust
//! The offline graph's registry packages (spec
//! `docs/superpowers/specs/2026-10-09-phase-3-3b-index-and-publishing-design.md`
//! §5.4, §10.2).

use std::path::{Path, PathBuf};

use nova_diagnostics::{render, FileDb};
use nova_pm::{graph_with, index_dir_name, requirements, Graph, Lock, LockedPackage, Offline};
use semver::Version;

const INDEX: &str = "https://example.test/index/";

/// A fresh, empty directory with a fixed name, so each run replaces the last.
fn fresh(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("nova-pm-registry-{name}"));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("create the test directory");
    dir
}

/// A package called `name` at `dir`: its `nova.toml` with `extra`
/// appended, and an empty `src/<file>` for each of `files`.
fn package(dir: &Path, name: &str, version: &str, files: &[&str], extra: &str) {
    std::fs::create_dir_all(dir.join("src")).unwrap();
    std::fs::write(
        dir.join("nova.toml"),
        format!(
            "[package]\nname = \"{name}\"\nversion = \"{version}\"\nedition = \"2026\"\n{extra}"
        ),
    )
    .unwrap();
    for file in files {
        std::fs::write(dir.join("src").join(file), "").unwrap();
    }
}

/// The library `name` `version`, unpacked in `registry` for [`INDEX`].
fn cached(registry: &Path, name: &str, version: &str, extra: &str) -> PathBuf {
    let dir = registry
        .join("src")
        .join(index_dir_name(INDEX))
        .join(format!("{name}-{version}"));
    package(&dir, name, version, &["lib.nova"], extra);
    dir
}

/// A lock on [`INDEX`] holding each `(name, version, dependencies)`.
fn lock(packages: &[(&str, &str, &[&str])]) -> Lock {
    Lock {
        index: INDEX.into(),
        packages: packages
            .iter()
            .map(|(name, version, deps)| LockedPackage {
                name: name.to_string(),
                version: Version::parse(version).unwrap(),
                checksum: "0".repeat(64),
                dependencies: deps.iter().map(|d| d.to_string()).collect(),
            })
            .collect(),
    }
}

fn write_lock(root: &Path, lock: &Lock) {
    std::fs::write(root.join("nova.lock"), lock.to_text()).unwrap();
}

fn offline(registry: &Path) -> Offline {
    Offline {
        registry: Some(registry.to_path_buf()),
        lock: None,
        dev: true,
    }
}

/// The graph of the package at `root`, its diagnostics' codes, and their
/// rendering.
fn build(root: &Path, offline: &Offline) -> (Option<Graph>, Vec<String>, String) {
    let mut db = FileDb::new();
    let (graph, diagnostics) = graph_with(root, None, offline, &mut db);
    let codes = diagnostics.iter().map(|d| d.code.clone()).collect();
    (graph, codes, render::render_to_string(&db, &diagnostics))
}

fn names(graph: &Graph) -> Vec<&str> {
    graph.packages.iter().map(|p| p.name.as_str()).collect()
}

const JSON_ENTRY: &str = "\n[dependencies]\njson = \"1\"\n";

#[test]
fn a_registry_entry_is_found_through_the_lock_and_the_cache() {
    let dir = fresh("found");
    let app = dir.join("app");
    package(&app, "app", "0.1.0", &["main.nova"], JSON_ENTRY);
    write_lock(&app, &lock(&[("json", "1.4.1", &[])]));
    let registry = dir.join("registry");
    let unpacked = cached(&registry, "json", "1.4.1", "");
    let (graph, codes, rendered) = build(&app, &offline(&registry));
    assert!(codes.is_empty(), "{rendered}");
    let graph = graph.unwrap();
    assert_eq!(names(&graph), ["app", "json"]);
    assert!(!graph.root().registry);
    assert!(graph.packages[1].registry);
    assert_eq!(graph.packages[1].canonical, nova_pm::real_path(&unpacked));
    assert_eq!(graph.root().dependencies[0].import_name, "json");
}

#[test]
fn a_registry_packages_own_dependencies_are_found_the_same_way() {
    let dir = fresh("transitive");
    let app = dir.join("app");
    package(&app, "app", "0.1.0", &["main.nova"], "\n[dependencies]\ngeom = \"0.2\"\n");
    write_lock(
        &app,
        &lock(&[("geom", "0.2.0", &["json"]), ("json", "1.0.0", &[])]),
    );
    let registry = dir.join("registry");
    cached(&registry, "geom", "0.2.0", JSON_ENTRY);
    cached(&registry, "json", "1.0.0", "");
    let (graph, codes, rendered) = build(&app, &offline(&registry));
    assert!(codes.is_empty(), "{rendered}");
    let graph = graph.unwrap();
    assert_eq!(names(&graph), ["app", "geom", "json"]);
    assert!(graph.packages[1].registry && graph.packages[2].registry);
}

#[test]
fn an_entry_with_no_locked_package_is_m0005() {
    let dir = fresh("unlocked");
    let app = dir.join("app");
    package(&app, "app", "0.1.0", &["main.nova"], JSON_ENTRY);
    let registry = dir.join("registry");
    // No nova.lock at all, then a lock without `json`.
    for lock in [None, Some(lock(&[("http", "1.0.0", &[])]))] {
        if let Some(lock) = &lock {
            write_lock(&app, lock);
        }
        let (graph, codes, rendered) = build(&app, &offline(&registry));
        assert_eq!(codes, ["M0005"], "{rendered}");
        assert!(
            rendered.contains("dependency `json` is not downloaded yet; run `nova fetch`"),
            "{rendered}"
        );
        assert_eq!(names(&graph.unwrap()), ["app"]);
    }
}

#[test]
fn a_locked_version_that_no_longer_fits_is_m0005() {
    let dir = fresh("no-fit");
    let app = dir.join("app");
    package(&app, "app", "0.1.0", &["main.nova"], "\n[dependencies]\njson = \"2\"\n");
    write_lock(&app, &lock(&[("json", "1.4.1", &[])]));
    let registry = dir.join("registry");
    cached(&registry, "json", "1.4.1", "");
    let (_, codes, rendered) = build(&app, &offline(&registry));
    assert_eq!(codes, ["M0005"], "{rendered}");
    assert!(
        rendered.contains(
            "dependency `json` is locked at 1.4.1, which does not meet `^2`; run `nova fetch`"
        ),
        "{rendered}"
    );
}

#[test]
fn a_locked_package_not_unpacked_is_m0005() {
    let dir = fresh("not-unpacked");
    let app = dir.join("app");
    package(&app, "app", "0.1.0", &["main.nova"], JSON_ENTRY);
    write_lock(&app, &lock(&[("json", "1.4.1", &[])]));
    let (_, codes, rendered) = build(&app, &offline(&dir.join("registry")));
    assert_eq!(codes, ["M0005"], "{rendered}");
    assert!(rendered.contains("is not downloaded yet"), "{rendered}");
}

#[test]
fn an_unreadable_lock_is_m0016_and_each_entry_m0005() {
    let dir = fresh("unreadable");
    let app = dir.join("app");
    package(
        &app,
        "app",
        "0.1.0",
        &["main.nova"],
        "\n[dependencies]\njson = \"1\"\nhttp = \"1\"\n",
    );
    std::fs::write(app.join("nova.lock"), "version = 2\nindex = \"x\"\n").unwrap();
    let (_, mut codes, rendered) = build(&app, &offline(&dir.join("registry")));
    codes.sort();
    assert_eq!(codes, ["M0005", "M0005", "M0016"], "{rendered}");
    assert!(rendered.contains("nova.lock cannot be read"), "{rendered}");
}

#[test]
fn no_nova_home_is_m0005_saying_to_set_it() {
    let dir = fresh("no-home");
    let app = dir.join("app");
    package(&app, "app", "0.1.0", &["main.nova"], JSON_ENTRY);
    write_lock(&app, &lock(&[("json", "1.4.1", &[])]));
    let offline = Offline {
        registry: None,
        lock: None,
        dev: true,
    };
    let (_, codes, rendered) = build(&app, &offline);
    assert_eq!(codes, ["M0005"], "{rendered}");
    assert!(
        rendered.contains("set NOVA_HOME to a writable directory"),
        "{rendered}"
    );
}

#[test]
fn a_downloaded_package_with_a_path_entry_is_m0017() {
    let dir = fresh("cached-path");
    let app = dir.join("app");
    package(&app, "app", "0.1.0", &["main.nova"], JSON_ENTRY);
    write_lock(&app, &lock(&[("json", "1.4.1", &["x"])]));
    let registry = dir.join("registry");
    cached(
        &registry,
        "json",
        "1.4.1",
        "\n[dependencies]\nx = { path = \"../x\" }\n",
    );
    let (_, codes, rendered) = build(&app, &offline(&registry));
    assert_eq!(codes, ["M0017"], "{rendered}");
    assert!(
        rendered.contains("downloaded package `json` 1.4.1 has a path dependency `x`"),
        "{rendered}"
    );
}

#[test]
fn a_downloaded_package_unlike_its_lock_entry_is_m0005() {
    let dir = fresh("cached-unlike");
    let app = dir.join("app");
    package(&app, "app", "0.1.0", &["main.nova"], JSON_ENTRY);
    write_lock(&app, &lock(&[("json", "1.4.1", &[])]));
    let registry = dir.join("registry");
    cached(&registry, "json", "1.4.1", "\n[dependencies]\nhttp = \"1\"\n");
    let (_, codes, rendered) = build(&app, &offline(&registry));
    assert_eq!(codes, ["M0005"], "{rendered}");
    assert!(
        rendered.contains("the downloaded copy of `json` 1.4.1 does not match nova.lock"),
        "{rendered}"
    );
}

#[test]
fn a_downloaded_manifests_warning_is_dropped_and_its_error_kept() {
    let dir = fresh("cached-warning");
    let app = dir.join("app");
    package(
        &app,
        "app",
        "0.1.0",
        &["main.nova"],
        "\n[dependencies]\njson = \"1\"\nhttp = \"1\"\n",
    );
    write_lock(
        &app,
        &lock(&[("http", "1.0.0", &[]), ("json", "1.4.1", &[])]),
    );
    let registry = dir.join("registry");
    // An unknown key is M0006, a warning; `bad = 5` is M0003, an error.
    cached(&registry, "json", "1.4.1", "colour = \"red\"\n");
    cached(&registry, "http", "1.0.0", "\n[dependencies]\nbad = 5\n");
    let (_, codes, rendered) = build(&app, &offline(&registry));
    assert_eq!(codes, ["M0003"], "{rendered}");
}

#[test]
fn the_lock_given_in_offline_is_used_instead_of_the_file() {
    let dir = fresh("given-lock");
    let app = dir.join("app");
    package(&app, "app", "0.1.0", &["main.nova"], JSON_ENTRY);
    let registry = dir.join("registry");
    cached(&registry, "json", "1.4.1", "");
    let offline = Offline {
        lock: Some(lock(&[("json", "1.4.1", &[])])),
        ..offline(&registry)
    };
    let (graph, codes, rendered) = build(&app, &offline);
    assert!(codes.is_empty(), "{rendered}");
    assert_eq!(names(&graph.unwrap()), ["app", "json"]);
}

#[test]
fn without_dev_the_roots_dev_dependencies_are_not_read() {
    let dir = fresh("no-dev");
    let app = dir.join("app");
    package(
        &app,
        "app",
        "0.1.0",
        &["lib.nova"],
        "\n[dev-dependencies]\njson = \"1\"\n",
    );
    let registry = dir.join("registry");
    let (_, codes, _) = build(&app, &offline(&registry));
    assert_eq!(codes, ["M0005"]);
    let offline = Offline {
        dev: false,
        ..offline(&registry)
    };
    let (graph, codes, rendered) = build(&app, &offline);
    assert!(codes.is_empty(), "{rendered}");
    assert_eq!(names(&graph.unwrap()), ["app"]);
}

#[test]
fn requirements_name_who_made_each_and_the_roots_entry() {
    let dir = fresh("requirements");
    let app = dir.join("app");
    // No source file: `nova add` syncs a package before it has one, so
    // this is not M0013 here.
    package(
        &app,
        "app",
        "0.1.0",
        &[],
        "\n[dependencies]\njson = \"1\"\nutil = { path = \"../util\" }\n\n\
         [dev-dependencies]\nkit = \"0.3\"\n",
    );
    package(
        &dir.join("util"),
        "util",
        "0.1.0",
        &["lib.nova"],
        "\n[dependencies]\nhttp = \"^0.2\"\n",
    );
    let mut db = FileDb::new();
    let (found, diagnostics) = requirements(&app, None, true, &mut db);
    assert!(
        diagnostics.is_empty(),
        "{}",
        render::render_to_string(&db, &diagnostics)
    );
    let text = |span: nova_diagnostics::Span| -> String {
        db.get_source(span.file).unwrap()[span.as_range()].to_string()
    };
    let listed: Vec<String> = found
        .iter()
        .map(|r| format!("{} {} {} {}", r.name, r.req, r.by, text(r.span)))
        .collect();
    assert_eq!(
        listed,
        ["json ^1 app json", "kit ^0.3 app kit", "http ^0.2 util util"]
    );
    let mut db = FileDb::new();
    let (found, _) = requirements(&app, None, false, &mut db);
    let names: Vec<&str> = found.iter().map(|r| r.name.as_str()).collect();
    assert_eq!(names, ["json", "http"]);
}
```

In `crates/nova-pm/tests/graph.rs`, the body of
`a_version_only_entry_is_m0005_and_names_the_dependency` becomes:

```rust
    let dir = fresh("m0005");
    package(&dir, "app", MAIN, "\n[dependencies]\nhttp = \"1.0\"\n");
    let (graph, codes, rendered) = build(&dir);
    assert_eq!(codes, ["M0005"]);
    // Spec 3.3b §5.4: a registry entry is found through nova.lock and the
    // cache, and with no lock it is not downloaded yet.
    assert!(
        rendered.contains("dependency `http` is not downloaded yet; run `nova fetch`"),
        "{rendered}"
    );
    assert_eq!(names(&graph.unwrap()), ["app"]);
```

- [ ] **Step 2: Run them to verify they fail**

```bash
cd /d/Projects/nona/nova && cargo test --locked -p nova-pm --test registry_graph 2>&1 | grep -E "^error" | head -3; cargo test --locked -p nova-pm --test graph a_version_only 2>&1 | grep -E "panicked|test result"
```

Expected:
- `error[E0432]: unresolved imports` naming `graph_with`, `requirements`
  and `Offline`;
- the changed graph test panics on its new message: `test result:
  FAILED. 0 passed; 1 failed`.

- [ ] **Step 3: Implement**

Replace `crates/nova-pm/src/graph.rs` with:

```rust
//! The package graph (specs
//! `docs/superpowers/specs/2026-10-08-phase-3-3a-local-packages-design.md`
//! §3 and
//! `docs/superpowers/specs/2026-10-09-phase-3-3b-index-and-publishing-design.md`
//! §5.4): the root package, and every package its dependencies reach,
//! found by path or through `nova.lock` and the registry directory. The
//! graph never touches the network.

use std::collections::{HashMap, VecDeque};
use std::path::{Path, PathBuf};

use nova_diagnostics::{Diagnostic, FileDb, FileId, Severity, Span};
use semver::VersionReq;

use crate::lock::{parse_lock, Lock, LOCKFILE};
use crate::manifest::{Dependency, Manifest};
use crate::resolve::Requirement;
use crate::{import_name, index_dir_name, real_path, registry_dir, MANIFEST};

/// A package's index in its [`Graph`]. The root is `PackageId(0)`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct PackageId(pub u32);

/// A dependency that resolved.
#[derive(Debug, Clone)]
pub struct Edge {
    /// The name the dependent imports it by (spec §3.4).
    pub import_name: String,
    pub package: PackageId,
    /// The entry in the declaring manifest.
    pub span: Span,
}

/// One package of a [`Graph`].
#[derive(Debug, Clone)]
pub struct GraphPackage {
    pub name: String,
    pub import_name: String,
    /// Where its files are read: the root as the caller spelled it, and a
    /// dependency joined onto its declarer's `dir`, such as `../geom`.
    pub dir: PathBuf,
    /// [`real_path`] of `dir`, which identifies the package (spec §3.3).
    pub canonical: PathBuf,
    pub manifest: Manifest,
    /// Its `nova.toml`, in the caller's `FileDb`.
    pub manifest_file: FileId,
    pub has_lib: bool,
    pub has_main: bool,
    /// Whether it was found in the registry directory (spec 3.3b §5.4).
    /// Its warnings are not shown.
    pub registry: bool,
    pub dependencies: Vec<Edge>,
    /// Read for the root package only (spec §3.3).
    pub dev_dependencies: Vec<Edge>,
}

/// The root package, and every package its dependencies reach.
#[derive(Debug, Clone)]
pub struct Graph {
    /// The root first.
    pub packages: Vec<GraphPackage>,
}

impl Graph {
    pub fn root(&self) -> &GraphPackage {
        &self.packages[0]
    }

    pub fn package(&self, id: PackageId) -> &GraphPackage {
        &self.packages[id.0 as usize]
    }

    /// Every package's canonical directory, the root's first.
    pub fn dirs(&self) -> Vec<PathBuf> {
        self.packages.iter().map(|p| p.canonical.clone()).collect()
    }

    /// The root's own entry through which `id` is first reached, breadth
    /// first, dependencies before dev-dependencies. `None` for the root.
    /// The language server shows a dependency's problems there (spec §6).
    pub fn reached_through(&self, id: PackageId) -> Option<Span> {
        let root = self.root();
        let mut via: HashMap<PackageId, Span> = HashMap::new();
        let mut queue = VecDeque::new();
        for edge in root.dependencies.iter().chain(&root.dev_dependencies) {
            if edge.package != PackageId(0) && !via.contains_key(&edge.package) {
                via.insert(edge.package, edge.span);
                queue.push_back(edge.package);
            }
        }
        while let Some(package) = queue.pop_front() {
            let span = via[&package];
            for edge in &self.package(package).dependencies {
                if edge.package != PackageId(0) && !via.contains_key(&edge.package) {
                    via.insert(edge.package, span);
                    queue.push_back(edge.package);
                }
            }
        }
        via.get(&id).copied()
    }
}

/// Where the graph finds registry packages (spec 3.3b §5.4).
#[derive(Debug, Clone)]
pub struct Offline {
    /// `$NOVA_HOME/registry`. `None` when `$NOVA_HOME` cannot be found.
    pub registry: Option<PathBuf>,
    /// The lock to use instead of the root's `nova.lock`: `nova add`
    /// checks the lock it would write this way.
    pub lock: Option<Lock>,
    /// Whether the root's `[dev-dependencies]` are read. Publishing's
    /// verification reads none (spec 3.3b §6.6).
    pub dev: bool,
}

impl Offline {
    /// The registry directory from this process's environment, the root's
    /// own `nova.lock`, and its dev-dependencies.
    pub fn from_env() -> Offline {
        Offline {
            registry: registry_dir(),
            lock: None,
            dev: true,
        }
    }
}

/// Read the package at `root`, the directory holding its `nova.toml` (empty
/// for the current directory), and every package its dependencies reach.
/// Every manifest goes into `db`, so each diagnostic's label renders.
///
/// The graph is partial on error: what resolved is kept, with the
/// diagnostics. It is `None` only when the root's own manifest cannot be
/// read or parsed.
pub fn graph(root: &Path, db: &mut FileDb) -> (Option<Graph>, Vec<Diagnostic>) {
    graph_with(root, None, &Offline::from_env(), db)
}

/// [`graph`], with the root's `nova.toml` taken from `text` when it is
/// given: `nova add` checks a new entry this way before writing it.
pub fn graph_from(
    root: &Path,
    text: Option<&str>,
    db: &mut FileDb,
) -> (Option<Graph>, Vec<Diagnostic>) {
    graph_with(root, text, &Offline::from_env(), db)
}

/// [`graph_from`], with registry packages found as `offline` says.
pub fn graph_with(
    root: &Path,
    text: Option<&str>,
    offline: &Offline,
    db: &mut FileDb,
) -> (Option<Graph>, Vec<Diagnostic>) {
    let mut builder = Builder::new(root, db, offline, Mode::Graph);
    let Some(package) = builder.read(root, text, false) else {
        return (None, builder.diagnostics);
    };
    builder.check_root(&package);
    builder.packages.push(package);
    builder.walk();
    let Builder {
        diagnostics,
        packages,
        ..
    } = builder;
    (Some(Graph { packages }), diagnostics)
}

/// Every version entry that the root and its path packages declare, for
/// the resolver (spec 3.3b §4.1): who made each one, and the root's entry
/// through which it is reached. The root's dev-dependencies count when
/// `dev` is set. The root's `nova.toml` is `text` when it is given.
///
/// Registry packages are not read: their requirements come from the index.
/// The diagnostics are the manifests' and the path graph's, without M0013,
/// since `nova add` syncs a package before it has a source file.
pub fn requirements(
    root: &Path,
    text: Option<&str>,
    dev: bool,
    db: &mut FileDb,
) -> (Vec<Requirement>, Vec<Diagnostic>) {
    let offline = Offline {
        registry: None,
        lock: None,
        dev,
    };
    let mut builder = Builder::new(root, db, &offline, Mode::Collect);
    let Some(package) = builder.read(root, text, false) else {
        return (Vec::new(), builder.diagnostics);
    };
    builder.check_root(&package);
    builder.packages.push(package);
    builder.walk();
    let graph = Graph {
        packages: std::mem::take(&mut builder.packages),
    };
    let found = builder
        .requirements
        .into_iter()
        .map(|(mut requirement, from)| {
            if let Some(span) = graph.reached_through(from) {
                requirement.span = span;
            }
            requirement
        })
        .collect();
    (found, builder.diagnostics)
}

/// Whether `name` is one of Nova's keywords, which no import name may be.
fn is_keyword(name: &str) -> bool {
    nova_lexer::KEYWORDS.contains(&name)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Visit {
    New,
    OnPath,
    Done,
}

/// What the builder does with a version entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Mode {
    /// Find its package through the lock and the registry directory.
    Graph,
    /// Record it as a [`Requirement`].
    Collect,
}

/// The root's `nova.lock`, once read.
enum LockState {
    Missing,
    Unreadable,
    Read(Lock),
}

struct Builder<'a> {
    db: &'a mut FileDb,
    offline: &'a Offline,
    mode: Mode,
    /// The root's directory, where its `nova.lock` is.
    root: PathBuf,
    diagnostics: Vec<Diagnostic>,
    packages: Vec<GraphPackage>,
    /// Read at the first registry entry.
    lock: Option<LockState>,
    /// In [`Mode::Collect`]: each version entry, and the package that made
    /// it.
    requirements: Vec<(Requirement, PackageId)>,
}

impl<'a> Builder<'a> {
    fn new(root: &Path, db: &'a mut FileDb, offline: &'a Offline, mode: Mode) -> Builder<'a> {
        Builder {
            db,
            offline,
            mode,
            root: root.to_path_buf(),
            diagnostics: Vec::new(),
            packages: Vec::new(),
            lock: None,
            requirements: Vec::new(),
        }
    }
}

impl Builder<'_> {
    fn error(&mut self, code: &str, message: String, at: Span, label: &str) {
        self.diagnostics
            .push(Diagnostic::error(code, message).with_primary_label(at, label));
    }

    /// M0013 and M0012 on the root package itself. M0013 is the graph's
    /// alone (see [`requirements`]).
    fn check_root(&mut self, package: &GraphPackage) {
        if self.mode == Mode::Graph && !package.has_lib && !package.has_main {
            self.error(
                "M0013",
                format!(
                    "package `{}` has neither src/lib.nova nor src/main.nova",
                    package.name
                ),
                package.manifest.package.span,
                "a package needs a library, a program, or both",
            );
        }
        if package.has_lib && is_keyword(&package.import_name) {
            self.error(
                "M0012",
                format!(
                    "library `{}` would be imported as `{}`, which is a keyword",
                    package.name, package.import_name
                ),
                package.manifest.package.span,
                "choose another name",
            );
        }
    }

    /// Expand every package, breadth first from the root, then find the
    /// cycles.
    fn walk(&mut self) {
        let mut queue = VecDeque::from([PackageId(0)]);
        while let Some(id) = queue.pop_front() {
            self.expand(id, &mut queue);
        }
        self.cycles();
    }

    /// The package whose manifest is `dir/nova.toml`, or `text` when it is
    /// given, with no edges yet. `None` when the manifest cannot be read or
    /// has errors. A registry package's warnings are dropped (spec 3.3b
    /// §5.4).
    fn read(&mut self, dir: &Path, text: Option<&str>, registry: bool) -> Option<GraphPackage> {
        let path = dir.join(MANIFEST);
        let source = match text {
            Some(text) => text.to_string(),
            None => match std::fs::read_to_string(&path) {
                Ok(source) => source,
                Err(error) => {
                    self.diagnostics.push(Diagnostic::error(
                        "M0001",
                        format!("cannot read {}: {error}", path.display()),
                    ));
                    return None;
                }
            },
        };
        let file = self.db.add(path.display().to_string(), source.as_str());
        let (manifest, diagnostics) = crate::manifest::parse(&source, file);
        self.diagnostics.extend(
            diagnostics
                .into_iter()
                .filter(|d| !registry || d.severity == Severity::Error),
        );
        let manifest = manifest?;
        let canonical = real_path(if dir.as_os_str().is_empty() {
            Path::new(".")
        } else {
            dir
        });
        let src = dir.join("src");
        Some(GraphPackage {
            name: manifest.package.name.clone(),
            import_name: import_name(&manifest.package.name),
            dir: dir.to_path_buf(),
            canonical,
            has_lib: src.join("lib.nova").is_file(),
            has_main: src.join("main.nova").is_file(),
            registry,
            manifest,
            manifest_file: file,
            dependencies: Vec::new(),
            dev_dependencies: Vec::new(),
        })
    }

    /// Resolve package `id`'s entries, and for the root its
    /// dev-dependencies when they are read, queueing each package read for
    /// the first time.
    fn expand(&mut self, id: PackageId, queue: &mut VecDeque<PackageId>) {
        let package = &self.packages[id.0 as usize];
        let mut entries: Vec<(Dependency, bool)> = package
            .manifest
            .dependencies
            .iter()
            .cloned()
            .map(|d| (d, false))
            .collect();
        if id == PackageId(0) && self.offline.dev {
            entries.extend(
                package
                    .manifest
                    .dev_dependencies
                    .iter()
                    .cloned()
                    .map(|d| (d, true)),
            );
        }
        let usable = self.names_ok(id, &entries);
        for ((dependency, dev), ok) in entries.iter().zip(usable) {
            if !ok {
                continue;
            }
            if let Some(edge) = self.edge(id, dependency, queue) {
                let package = &mut self.packages[id.0 as usize];
                if *dev {
                    package.dev_dependencies.push(edge);
                } else {
                    package.dependencies.push(edge);
                }
            }
        }
    }

    /// M0012 (spec §3.4): which of package `id`'s entries may be resolved.
    /// `[dependencies]` and `[dev-dependencies]` count together, since a
    /// test module sees both.
    fn names_ok(&mut self, id: PackageId, entries: &[(Dependency, bool)]) -> Vec<bool> {
        let own = self.packages[id.0 as usize].import_name.clone();
        let mut seen: Vec<(String, String)> = Vec::new();
        let mut ok = Vec::new();
        for (dependency, _) in entries {
            let name = import_name(&dependency.name);
            let problem = if is_keyword(&name) {
                Some(format!(
                    "dependency `{}` would be imported as `{name}`, which is a keyword",
                    dependency.name
                ))
            } else if let Some((_, key)) = seen.iter().find(|(n, _)| *n == name) {
                Some(if *key == dependency.name {
                    format!(
                        "`{}` is in both [dependencies] and [dev-dependencies]",
                        dependency.name
                    )
                } else {
                    format!(
                        "`{key}` and `{}` are both imported as `{name}`",
                        dependency.name
                    )
                })
            } else if id == PackageId(0) && name == own {
                Some(format!(
                    "dependency `{}` would be imported as `{name}`, this package's own name",
                    dependency.name
                ))
            } else {
                None
            };
            match problem {
                Some(message) => {
                    self.error("M0012", message, dependency.span, "choose another name");
                    ok.push(false);
                }
                None => {
                    seen.push((name, dependency.name.clone()));
                    ok.push(true);
                }
            }
        }
        ok
    }

    /// Resolve one entry of package `from` (spec §3.2, §3.3), reading the
    /// package it names if this is the first time.
    fn edge(
        &mut self,
        from: PackageId,
        dependency: &Dependency,
        queue: &mut VecDeque<PackageId>,
    ) -> Option<Edge> {
        let path = match (&dependency.path, &dependency.version) {
            (Some(path), _) => path,
            (None, Some(req)) => return self.registry_edge(from, dependency, &req.clone(), queue),
            // The manifest parser gives every entry one or the other.
            (None, None) => return None,
        };
        let at = dependency.path_span.unwrap_or(dependency.span);
        let declarer = &self.packages[from.0 as usize];
        let dir = declarer.dir.join(path);
        let declarer_name = declarer.name.clone();
        let declarer_canonical = declarer.canonical.clone();
        if !dir.join(MANIFEST).is_file() {
            self.error(
                "M0007",
                format!(
                    "dependency `{}`: {} is not a directory holding a nova.toml",
                    dependency.name,
                    dir.display()
                ),
                at,
                "no nova.toml here",
            );
            return None;
        }
        let canonical = real_path(&dir);
        if canonical == declarer_canonical {
            self.error(
                "M0010",
                format!("dependency cycle: `{declarer_name}` depends on itself"),
                at,
                "this is the package's own directory",
            );
            return None;
        }
        let package = match self.packages.iter().position(|p| p.canonical == canonical) {
            Some(index) => {
                let found = &self.packages[index].name;
                if *found != dependency.name {
                    let message =
                        format!("dependency `{}` is the package `{found}`", dependency.name);
                    self.error(
                        "M0008",
                        message,
                        dependency.span,
                        "an entry's key must be the package's name",
                    );
                    return None;
                }
                // Only the root can be reached again without its library
                // checked: a dev-dependency that depends back on it.
                if !self.packages[index].has_lib {
                    self.error(
                        "M0009",
                        format!(
                            "dependency `{}` is not a library: {} has no src/lib.nova",
                            dependency.name,
                            dir.display()
                        ),
                        at,
                        "a dependency needs src/lib.nova",
                    );
                    return None;
                }
                PackageId(index as u32)
            }
            None => {
                let new = self.read(&dir, None, false)?;
                self.admit(new, dependency, at, queue)?
            }
        };
        Some(Edge {
            import_name: import_name(&dependency.name),
            package,
            span: dependency.span,
        })
    }

    /// A package read for the first time, checked against the entry that
    /// named it (M0008, M0009, M0011), then added to the graph and queued.
    fn admit(
        &mut self,
        new: GraphPackage,
        dependency: &Dependency,
        at: Span,
        queue: &mut VecDeque<PackageId>,
    ) -> Option<PackageId> {
        if new.name != dependency.name {
            let message = format!(
                "dependency `{}` is the package `{}`",
                dependency.name, new.name
            );
            self.error(
                "M0008",
                message,
                dependency.span,
                "an entry's key must be the package's name",
            );
            return None;
        }
        if !new.has_lib {
            self.error(
                "M0009",
                format!(
                    "dependency `{}` is not a library: {} has no src/lib.nova",
                    dependency.name,
                    new.dir.display()
                ),
                at,
                "a dependency needs src/lib.nova",
            );
            return None;
        }
        if let Some(other) = self.packages.iter().find(|p| p.name == new.name) {
            let message = format!(
                "two packages named `{}` in one build: {} and {}",
                new.name,
                other.dir.display(),
                new.dir.display()
            );
            self.error(
                "M0011",
                message,
                dependency.span,
                "the second package of that name",
            );
            return None;
        }
        self.packages.push(new);
        let id = PackageId(self.packages.len() as u32 - 1);
        queue.push_back(id);
        Some(id)
    }

    /// Resolve a version entry of package `from` through the lock and the
    /// registry directory (spec 3.3b §5.4), or in [`Mode::Collect`] record
    /// it.
    fn registry_edge(
        &mut self,
        from: PackageId,
        dependency: &Dependency,
        req: &VersionReq,
        queue: &mut VecDeque<PackageId>,
    ) -> Option<Edge> {
        if self.mode == Mode::Collect {
            let by = self.packages[from.0 as usize].name.clone();
            let requirement = Requirement {
                name: dependency.name.clone(),
                req: req.clone(),
                by,
                span: dependency.span,
            };
            self.requirements.push((requirement, from));
            return None;
        }
        let name = &dependency.name;
        let found = self
            .lock()
            .map(|lock| (lock.index.clone(), lock.find(name).cloned()));
        let Some((index, Some(locked))) = found else {
            self.missing(
                dependency,
                format!("dependency `{name}` is not downloaded yet; run `nova fetch`"),
            );
            return None;
        };
        if !req.matches(&locked.version) {
            self.missing(
                dependency,
                format!(
                    "dependency `{name}` is locked at {}, which does not meet `{req}`; run \
                     `nova fetch`",
                    locked.version
                ),
            );
            return None;
        }
        let Some(registry) = self.offline.registry.clone() else {
            self.missing(
                dependency,
                format!(
                    "dependency `{name}` cannot be found: cannot place the package cache: set \
                     NOVA_HOME to a writable directory"
                ),
            );
            return None;
        };
        let dir = registry
            .join("src")
            .join(index_dir_name(&index))
            .join(format!("{name}-{}", locked.version));
        if !dir.join(MANIFEST).is_file() {
            self.missing(
                dependency,
                format!("dependency `{name}` is not downloaded yet; run `nova fetch`"),
            );
            return None;
        }
        let canonical = real_path(&dir);
        let package = match self.packages.iter().position(|p| p.canonical == canonical) {
            Some(index) => PackageId(index as u32),
            None => {
                let new = self.read(&dir, None, true)?;
                // Unpacking checked both (spec §5.2); a copy edited since is
                // refused here.
                if let Some(entry) = new.manifest.dependencies.iter().find(|d| d.path.is_some()) {
                    self.error(
                        "M0017",
                        format!(
                            "downloaded package `{name}` {} has a path dependency `{}`",
                            locked.version, entry.name
                        ),
                        dependency.span,
                        "declared here",
                    );
                    return None;
                }
                let mut names: Vec<String> = new
                    .manifest
                    .dependencies
                    .iter()
                    .map(|d| d.name.clone())
                    .collect();
                names.sort();
                if names != locked.dependencies {
                    self.missing(
                        dependency,
                        format!(
                            "the downloaded copy of `{name}` {} does not match nova.lock; delete \
                             {} and run `nova fetch`",
                            locked.version,
                            dir.display()
                        ),
                    );
                    return None;
                }
                self.admit(new, dependency, dependency.span, queue)?
            }
        };
        Some(Edge {
            import_name: import_name(name),
            package,
            span: dependency.span,
        })
    }

    /// M0005 on `dependency`: a registry entry the cache cannot satisfy.
    fn missing(&mut self, dependency: &Dependency, message: String) {
        self.diagnostics.push(
            Diagnostic::error("M0005", message).with_primary_label(dependency.span, "declared here"),
        );
    }

    /// The lock registry entries are found through: [`Offline::lock`], else
    /// the root's `nova.lock`, read once. M0016 when it cannot be read.
    fn lock(&mut self) -> Option<&Lock> {
        if self.lock.is_none() {
            let state = match &self.offline.lock {
                Some(lock) => LockState::Read(lock.clone()),
                None => {
                    let path = self.root.join(LOCKFILE);
                    match std::fs::read_to_string(&path) {
                        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                            LockState::Missing
                        }
                        Err(error) => {
                            self.diagnostics.push(
                                Diagnostic::error(
                                    "M0016",
                                    format!("nova.lock cannot be read: {error}"),
                                )
                                .with_note("delete nova.lock, or run `nova update`"),
                            );
                            LockState::Unreadable
                        }
                        Ok(text) => {
                            let file = self.db.add(path.display().to_string(), text.as_str());
                            match parse_lock(&text, file) {
                                Ok(lock) => LockState::Read(lock),
                                Err(diagnostic) => {
                                    self.diagnostics.push(diagnostic);
                                    LockState::Unreadable
                                }
                            }
                        }
                    }
                }
            };
            self.lock = Some(state);
        }
        match &self.lock {
            Some(LockState::Read(lock)) => Some(lock),
            _ => None,
        }
    }

    /// M0010 for each cycle of `[dependencies]` edges (spec §3.3), found
    /// depth first from every package.
    fn cycles(&mut self) {
        let count = self.packages.len();
        let mut state = vec![Visit::New; count];
        let mut path = Vec::new();
        for start in 0..count {
            if state[start] == Visit::New {
                self.visit(PackageId(start as u32), &mut state, &mut path);
            }
        }
    }

    fn visit(&mut self, id: PackageId, state: &mut [Visit], path: &mut Vec<PackageId>) {
        state[id.0 as usize] = Visit::OnPath;
        path.push(id);
        for edge in self.packages[id.0 as usize].dependencies.clone() {
            match state[edge.package.0 as usize] {
                Visit::OnPath => {
                    let start = path.iter().position(|p| *p == edge.package).unwrap_or(0);
                    let names: Vec<&str> = path[start..]
                        .iter()
                        .chain([&edge.package])
                        .map(|p| self.packages[p.0 as usize].name.as_str())
                        .collect();
                    let message = format!("dependency cycle: {}", names.join(" -> "));
                    self.error("M0010", message, edge.span, "this entry closes the cycle");
                }
                Visit::New => self.visit(edge.package, state, path),
                Visit::Done => {}
            }
        }
        path.pop();
        state[id.0 as usize] = Visit::Done;
    }
}
```

In `crates/nova-pm/src/lib.rs`, the `pub use graph::…;` line becomes:

```rust
pub use graph::{
    graph, graph_from, graph_with, requirements, Edge, Graph, GraphPackage, Offline, PackageId,
};
```

The crate's doc comment gains the 3.3b spec: after "`…/2026-10-08-phase-3-3a-local-packages-design.md`
§3)." it reads "…§3), and the index's offline side: `$NOVA_HOME`,
`nova.lock` and the resolver (spec
`docs/superpowers/specs/2026-10-09-phase-3-3b-index-and-publishing-design.md`
§4, §5)."

- [ ] **Step 4: Run the tests to verify they pass**

```bash
cd /d/Projects/nona/nova && P=/c/Users/SAKEER~1/AppData/Local/Temp/gcm/p34 && cargo test --locked -p nova-pm -p nova-driver > $P/t4.txt 2>&1; echo "exit=$?"; python -X utf8 $P/count.py $P/t4.txt; grep -E "FAILED|panicked|^warning|^error" $P/t4.txt | head; cargo test --locked -p nova-cli --test project --test packages --test deps > $P/t4-cli.txt 2>&1; echo "exit=$?"; python -X utf8 $P/count.py $P/t4-cli.txt
```

Expected:
- the first run: `exit=0`, 0 failed, no warnings, the 13 new tests
  included;
- the second: `exit=0`, 0 failed.

The CLI's three M0005 tests still pass, since each asserts the code and
the name, not the old message:
- `a_declared_dependency_is_m0005`;
- `each_graph_error_is_rendered_and_stops_the_command`;
- `add_refuses_without_writing`, whose `nova add geom` case is the CLI's
  own refusal.

Task 11 changes all three.

- [ ] **Step 5: Commit**

Write `$P/msg-4.txt`:

```
nova-pm: the offline graph finds registry packages

A version entry is no longer refused: the graph finds its package through
the root's nova.lock and the registry directory (spec
docs/superpowers/specs/2026-10-09-phase-3-3b-index-and-publishing-design.md
§5.4), and never touches the network.

- `graph_with` takes an `Offline`: the registry directory (from
  $NOVA_HOME, or given by a test), the lock (nova add passes the one it
  would write), and whether the root's dev-dependencies are read
  (publishing's verification reads none). `graph` and `graph_from` keep
  their signatures.
- M0005 now means a registry entry the cache cannot satisfy: not locked,
  locked at a version that no longer fits, not unpacked, or $NOVA_HOME
  not found. An unreadable lock is M0016. A downloaded copy edited since
  it was unpacked, with a path entry (M0017) or dependencies unlike the
  lock's, is refused.
- `GraphPackage.registry` marks a downloaded package; its manifest's
  warnings are dropped.
- `requirements` lists every version entry of the root and its path
  packages, with who made each and the root's entry it is reached
  through, for the resolver.

The graph test that pinned the old M0005 message now pins the new one.
nova-cli's three tests that pin it change in the CLI task.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
```

```bash
cd /d/Projects/nona/nova && P=/c/Users/SAKEER~1/AppData/Local/Temp/gcm/p34 && cargo fmt --all && git add crates/nova-pm && git commit -q -F $P/msg-4.txt && git log -1 --format=%s
```

Expected: `nova-pm: the offline graph finds registry packages`.

The task's test command: `cargo test --locked -p nova-pm -p nova-driver`.

---

### Task 5: The driver counts warnings and hides a dependency's

Spec §5.4, §6.6.

**Files:**
- Modify: `crates/nova-driver/src/program.rs`, `crates/nova-driver/src/lib.rs`, `crates/nova-driver/src/analyze.rs`
- Create: `crates/nova-driver/tests/registry.rs`

**Interfaces:**
- Consumes: Task 4's `Offline`, `graph_with` and `GraphPackage.registry`; Task 1's `index_dir_name`.
- Produces:
  - `Program::for_package_in(&Path, Roots, &nova_pm::Offline) -> Program`;
    `for_package` calls it with `Offline::from_env()`;
  - `nova_driver::Checked { errors: usize, warnings: usize }`;
  - `nova_driver::check_program_counted(Program) -> anyhow::Result<Checked>`.
    `check_program` wraps it, unchanged in behaviour.
- A warning is a dependency's when it has at least one label, and every
  label is in a registry package's module. A warning with no label is
  always shown.

- [ ] **Step 1: Write the failing tests**

`crates/nova-driver/tests/registry.rs`:

```rust
//! A registry package in the driver (spec
//! `docs/superpowers/specs/2026-10-09-phase-3-3b-index-and-publishing-design.md`
//! §5.4, §6.6): its warnings are neither shown nor counted, and its errors
//! are.

use std::path::{Path, PathBuf};

use nova_driver::{
    analyze_program, check_program_counted, Checked, DiskSources, Options, Program, Roots,
};
use nova_pm::Offline;

const INDEX: &str = "https://example.test/index/";

/// A fresh, empty directory with a fixed name, so each run replaces the last.
fn fresh(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("nova-driver-registry-{name}"));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// Write each `(path, text)` under `dir`.
fn write(dir: &Path, files: &[(&str, &str)]) {
    for (path, text) in files {
        let path = dir.join(path);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, text).unwrap();
    }
}

fn manifest(name: &str, version: &str, extra: &str) -> String {
    format!("[package]\nname = \"{name}\"\nversion = \"{version}\"\nedition = \"2026\"\n{extra}")
}

/// A match whose second arm is unreachable: E0021, a warning.
const UNREACHABLE: &str = "pub fn pick(n: Int) -> Int {\n    match n { _ => 1, 0 => 2 }\n}\n";

/// `dir/app`, depending on `json = "1"` locked at 1.0.0, with `main`, and
/// `dir/registry` holding `json` 1.0.0 whose `lib.nova` is `json_lib`.
fn project(dir: &Path, main: &str, json_lib: &str) -> (PathBuf, Offline) {
    let app = dir.join("app");
    write(
        &app,
        &[
            (
                "nova.toml",
                manifest("app", "0.1.0", "\n[dependencies]\njson = \"1\"\n").as_str(),
            ),
            (
                "nova.lock",
                "version = 1\nindex = \"https://example.test/index/\"\n\n[[package]]\n\
                 name = \"json\"\nversion = \"1.0.0\"\nchecksum = \"00\"\ndependencies = []\n",
            ),
            ("src/main.nova", main),
        ],
    );
    let registry = dir.join("registry");
    let json = registry
        .join("src")
        .join(nova_pm::index_dir_name(INDEX))
        .join("json-1.0.0");
    write(
        &json,
        &[
            ("nova.toml", manifest("json", "1.0.0", "").as_str()),
            ("src/lib.nova", json_lib),
        ],
    );
    let offline = Offline {
        registry: Some(registry),
        lock: None,
        dev: true,
    };
    (app, offline)
}

const MAIN: &str = "import json\n\nfn main() {\n    println(\"${pick(3)}\")\n}\n";

fn codes(program: Program) -> Vec<String> {
    let analysis =
        analyze_program(program, &DiskSources, &Options::default()).expect("the entry reads");
    analysis.diagnostics.iter().map(|d| d.code.clone()).collect()
}

#[test]
fn a_dependencys_warning_is_neither_shown_nor_counted() {
    let dir = fresh("dependency-warning");
    let (app, offline) = project(&dir, MAIN, UNREACHABLE);
    let checked =
        check_program_counted(Program::for_package_in(&app, Roots::Program, &offline)).unwrap();
    assert_eq!(
        checked,
        Checked {
            errors: 0,
            warnings: 0
        }
    );
    assert!(codes(Program::for_package_in(&app, Roots::Program, &offline)).is_empty());
}

#[test]
fn the_packages_own_warning_is_counted() {
    let dir = fresh("own-warning");
    let main = "import json\n\nfn own(n: Int) -> Int {\n    match n { _ => 1, 0 => 2 }\n}\n\n\
                fn main() {\n    println(\"${pick(own(3))}\")\n}\n";
    let (app, offline) = project(&dir, main, UNREACHABLE);
    let checked =
        check_program_counted(Program::for_package_in(&app, Roots::Program, &offline)).unwrap();
    assert_eq!(
        checked,
        Checked {
            errors: 0,
            warnings: 1
        }
    );
    assert_eq!(
        codes(Program::for_package_in(&app, Roots::Program, &offline)),
        ["E0021"]
    );
}

#[test]
fn a_dependencys_error_is_shown_and_counted() {
    let dir = fresh("dependency-error");
    let (app, offline) = project(&dir, MAIN, "pub fn pick(n: Int) -> Int {\n    \"nine\"\n}\n");
    let checked =
        check_program_counted(Program::for_package_in(&app, Roots::Program, &offline)).unwrap();
    assert_eq!(checked.errors, 1);
    assert_eq!(
        codes(Program::for_package_in(&app, Roots::Program, &offline)).len(),
        1
    );
}

#[test]
fn a_path_dependencys_warning_is_still_shown() {
    let dir = fresh("path-warning");
    write(
        &dir.join("geom"),
        &[
            ("nova.toml", manifest("geom", "0.1.0", "").as_str()),
            ("src/lib.nova", UNREACHABLE),
        ],
    );
    let app = dir.join("app");
    write(
        &app,
        &[
            (
                "nova.toml",
                manifest("app", "0.1.0", "\n[dependencies]\ngeom = { path = \"../geom\" }\n")
                    .as_str(),
            ),
            (
                "src/main.nova",
                "import geom\n\nfn main() {\n    println(\"${pick(3)}\")\n}\n",
            ),
        ],
    );
    let offline = Offline {
        registry: Some(dir.join("registry")),
        lock: None,
        dev: true,
    };
    let checked =
        check_program_counted(Program::for_package_in(&app, Roots::Program, &offline)).unwrap();
    assert_eq!(checked.warnings, 1);
}

#[test]
fn for_package_in_reads_no_dev_dependencies_when_told() {
    let dir = fresh("no-dev");
    let app = dir.join("app");
    write(
        &app,
        &[
            (
                "nova.toml",
                manifest("app", "0.1.0", "\n[dev-dependencies]\nkit = \"1\"\n").as_str(),
            ),
            ("src/lib.nova", "pub fn one() -> Int {\n    1\n}\n"),
        ],
    );
    let with_dev = Offline {
        registry: Some(dir.join("registry")),
        lock: None,
        dev: true,
    };
    assert_eq!(
        codes(Program::for_package_in(&app, Roots::Check, &with_dev)),
        ["M0005"]
    );
    let without = Offline {
        dev: false,
        ..with_dev
    };
    let checked =
        check_program_counted(Program::for_package_in(&app, Roots::Check, &without)).unwrap();
    assert_eq!(
        checked,
        Checked {
            errors: 0,
            warnings: 0
        }
    );
}
```

- [ ] **Step 2: Run them to verify they fail**

```bash
cd /d/Projects/nona/nova && cargo test --locked -p nova-driver --test registry 2>&1 | grep -E "^error" | head -3
```

Expected: `error[E0432]: unresolved imports` naming `check_program_counted`
and `Checked`, and E0599 for `for_package_in`.

- [ ] **Step 3: Implement**

In `crates/nova-driver/src/program.rs`:

1. The `use nova_diagnostics::…;` line keeps its names. The `use
   nova_pm::…;` line becomes:

   ```rust
   use nova_pm::{Edge, Graph, GraphPackage, Offline, PackageId};
   ```

2. `Program::for_package` becomes the two functions below. The body is
   today's, with `nova_pm::graph(root, &mut db)` replaced by `graph_with`:

```rust
    /// The package whose `nova.toml` is in `root` (empty for the current
    /// directory), with the roots `roots` names that exist.
    pub fn for_package(root: &Path, roots: Roots) -> Program {
        Program::for_package_in(root, roots, &Offline::from_env())
    }

    /// [`Program::for_package`], with registry packages found as `offline`
    /// says (spec 3.3b §5.4): tests pass the registry directory, and
    /// publishing's verification reads no dev-dependencies.
    pub fn for_package_in(root: &Path, roots: Roots, offline: &Offline) -> Program {
        let mut db = FileDb::new();
        let (graph, diagnostics) = nova_pm::graph_with(root, None, offline, &mut db);
        let mut list = Vec::new();
        let main = under(root, "src/main.nova");
        if main.is_file() {
            list.push(Root {
                path: main,
                kind: RootKind::Src,
            });
        }
        let runs = !list.is_empty();
        let lib = under(root, "src/lib.nova");
        if roots != Roots::Program && lib.is_file() {
            list.push(Root {
                path: lib,
                kind: RootKind::Src,
            });
        }
        if roots == Roots::Test {
            list.extend(
                test_files(&under(root, "tests"))
                    .into_iter()
                    .map(|path| Root {
                        path,
                        kind: RootKind::Tests,
                    }),
            );
        }
        Program {
            db,
            graph,
            roots: list,
            runs,
            diagnostics,
        }
    }
```

3. After `fn test_files`, add:

```rust
/// The files of `modules` that belong to a registry package (spec 3.3b
/// §5.4). A registry manifest's warnings never leave the graph.
pub(crate) fn registry_files(graph: Option<&Graph>, modules: &[Loaded]) -> HashSet<FileId> {
    let Some(graph) = graph else {
        return HashSet::new();
    };
    modules
        .iter()
        .filter(|m| m.package.is_some_and(|p| graph.package(p).registry))
        .map(|m| m.file)
        .collect()
}

/// Whether `d` is a warning about a registry package alone, which is not
/// shown, as Cargo caps a dependency's lints: it has a label, and every
/// label is in `registry`.
pub(crate) fn a_dependencys_warning(d: &Diagnostic, registry: &HashSet<FileId>) -> bool {
    d.severity == Severity::Warning
        && !d.labels.is_empty()
        && d.labels.iter().all(|label| registry.contains(&label.span.file))
}
```

In `crates/nova-driver/src/lib.rs`:

1. `use std::hash::{Hash, Hasher};` gains `use std::collections::HashSet;`
   on the line above.

2. `check_program` is replaced by:

```rust
/// What [`check_program_counted`] rendered.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Checked {
    pub errors: usize,
    /// The warnings shown. A registry package's are not (spec 3.3b §5.4).
    pub warnings: usize,
}

/// [`check_file`] on `program` (spec 3.3a §5.1). MIR runs only when the
/// program runs: its entry is `src/main.nova`, or a loose file.
pub fn check_program(program: Program) -> Result<Outcome<()>> {
    let checked = check_program_counted(program)?;
    if checked.errors > 0 {
        Ok(Outcome::Failed {
            errors: checked.errors,
        })
    } else {
        Ok(Outcome::Ok(()))
    }
}

/// [`check_program`], counting the errors and warnings it rendered.
/// Publishing's verification fails on either (spec 3.3b §6.6).
pub fn check_program_counted(program: Program) -> Result<Checked> {
    let runs = program.runs;
    let mut ctx = FrontendContext::new(program)?;
    if let Some((module, _tests, _fresh_def_id)) = ctx.check(false)? {
        if runs {
            if let Err(diags) = nova_mir::lower_module(&module) {
                ctx.render(&diags);
            }
        }
    }
    Ok(Checked {
        errors: ctx.errors,
        warnings: ctx.warnings,
    })
}
```

3. `struct FrontendContext` gains two fields after `errors`:

```rust
    warnings: usize,
    /// The registry packages' modules, once loaded: a warning only about
    /// them is not shown.
    registry_files: HashSet<FileId>,
```

   and `FrontendContext::new`'s initializer gains `warnings: 0,` and
   `registry_files: HashSet::new(),` after `errors: 0,`.

4. `fn render` becomes:

```rust
    fn render(&mut self, diagnostics: &[Diagnostic]) {
        let shown: Vec<Diagnostic> = diagnostics
            .iter()
            .filter(|d| !program::a_dependencys_warning(d, &self.registry_files))
            .cloned()
            .collect();
        self.errors += shown
            .iter()
            .filter(|d| d.severity == Severity::Error)
            .count();
        self.warnings += shown
            .iter()
            .filter(|d| d.severity == Severity::Warning)
            .count();
        if !shown.is_empty() {
            render::emit_all(&self.db, &shown);
        }
    }
```

5. In `fn load_modules`, the line `self.render(&load.diagnostics);`
   becomes two:

```rust
        self.registry_files = program::registry_files(self.graph.as_ref(), &load.modules);
        self.render(&load.diagnostics);
```

In `crates/nova-driver/src/analyze.rs`:

1. `use std::path::{Path, PathBuf};` gains `use std::collections::HashSet;`
   on the line above, and the `use crate::program::…;` line becomes:

   ```rust
   use crate::program::{a_dependencys_warning, load_program, registry_files, Load, Program};
   ```

2. After `let dropped = load.dropped;`, add:

```rust
    let registry = registry_files(graph.as_ref(), &load.modules);
```

   This line must come before `let mut modules = load.modules;`, which
   moves the modules out of `load`.

3. Each of the three `analysis.diagnostics = diagnostics;` becomes:

```rust
    analysis.diagnostics = shown(diagnostics, &registry);
```

4. After `fn has_error`, add:

```rust
/// `diagnostics` without a registry package's warnings (spec 3.3b §5.4).
fn shown(mut diagnostics: Vec<Diagnostic>, registry: &HashSet<FileId>) -> Vec<Diagnostic> {
    diagnostics.retain(|d| !a_dependencys_warning(d, registry));
    diagnostics
}
```

Since `load.dropped` is read before `load.modules` moves, write the new
line between them: `let dropped = load.dropped;`, then `let registry =
…;`, then `let mut modules = load.modules;`.

- [ ] **Step 4: Run the tests to verify they pass**

```bash
cd /d/Projects/nona/nova && P=/c/Users/SAKEER~1/AppData/Local/Temp/gcm/p34 && cargo test --locked -p nova-pm -p nova-driver > $P/t5.txt 2>&1; echo "exit=$?"; python -X utf8 $P/count.py $P/t5.txt; grep -E "FAILED|panicked|^warning|^error" $P/t5.txt | head
```

Expected:
- `exit=0`, 0 failed, no warnings;
- the five new tests included;
- every earlier driver test unchanged, `packages.rs` and `analyze.rs`
  included.

- [ ] **Step 5: Commit**

Write `$P/msg-5.txt`:

```
nova-driver: count warnings, and hide a dependency's

A warning about a registry package alone is no longer shown or counted,
by the CLI and the language server alike, as Cargo caps a dependency's
lints (spec
docs/superpowers/specs/2026-10-09-phase-3-3b-index-and-publishing-design.md
§5.4). A warning counts as the dependency's when it has a label and every
label is in a registry package's module; errors are always shown. The
graph already drops a registry manifest's warnings.

`check_program_counted` returns the errors and warnings rendered, for
publishing's verification, which fails on either (§6.6); `check_program`
wraps it. `Program::for_package_in` takes the graph's `Offline`, so tests
pass the registry directory and the verification reads no
dev-dependencies.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
```

```bash
cd /d/Projects/nona/nova && P=/c/Users/SAKEER~1/AppData/Local/Temp/gcm/p34 && cargo fmt --all && git add crates/nova-driver && git commit -q -F $P/msg-5.txt && git log -1 --format=%s
```

Expected: `nova-driver: count warnings, and hide a dependency's`.

The task's test command: `cargo test --locked -p nova-pm -p nova-driver`.

---

### Task 6: The `nova-index` crate: the index's format and a local reader

Spec §3.1–§3.3, §9, §10.3.

**Files:**
- Create: `crates/nova-index/Cargo.toml`
- Create: `crates/nova-index/src/lib.rs`, `src/location.rs`, `src/line.rs`, `src/read.rs`
- Create: `crates/nova-index/tests/index.rs`
- Modify: `Cargo.lock` (the new workspace member only)

**Interfaces:**
- Consumes:
  - Task 1's `canonical_index` and `local_index_path`;
  - Task 3's `Candidate` and `IndexView`.
- Produces:
  - `nova_index::DEFAULT_INDEX`;
  - `Index { canonical: String, source: Source }`, with:
    - `Index::new(&str) -> Result<Index, String>`;
    - `Index::from_setting(Option<&str>) -> Result<Index, String>`;
    - `Index::from_env() -> Result<Index, String>`;
    - `Index::tarball(&self, dl: &str, name: &str, version: &str) -> Result<Location, String>`;
  - `Source::{Http(String), Local(PathBuf)}` and `Location::{Url(String), File(PathBuf)}`;
  - `index_path(&str) -> String`;
  - `Line { name, vers, deps: Vec<LineDep>, cksum, v }` and `LineDep { name, req }`,
    with `Line::to_json(&self) -> String` and
    `Line::candidate(&self) -> Result<Candidate, String>`;
  - `LINE_VERSION: u64` (1);
  - `parse_lines(&str, &str) -> (Vec<Line>, Vec<String>)`;
  - `Config { dl: String, api: Option<String> }` and
    `parse_config(&str) -> Result<Config, String>`;
  - `fill_dl(&str, &str, &str) -> String`;
  - `trait Reader { fn file(&mut self, &str) -> Result<Option<String>, String>; fn config(&mut self) -> Result<Config, String> /* provided */ }`;
  - `LocalReader { dir: PathBuf }`;
  - `View<'a>`, with `View::new(&'a mut dyn Reader)`, a public
    `notes: Vec<String>`, and `impl IndexView`.

A note is the index's, not the compiler's. It reads:
`<path>:<line>: skipped a line that does not parse: <reason>`.

- [ ] **Step 1: Create the crate and write the failing tests**

`crates/nova-index/Cargo.toml`:

```toml
[package]
name = "nova-index"
version = "0.2.0"
edition.workspace = true
rust-version.workspace = true
license.workspace = true
repository.workspace = true
description = "The package index for Nova: reading, downloading, packing and publishing"

[lib]
# Not a benchmark target: `cargo bench` would otherwise build this under
# libtest's harness, which rejects criterion's `--output-format`. The
# workspace's only benchmark is `nova-lexer/benches/lex.rs`.
bench = false

[dependencies]
nova-diagnostics = { path = "../nova-diagnostics" }
nova-pm = { path = "../nova-pm" }
semver = { workspace = true }
# The index's lines and config.json (spec
# docs/superpowers/specs/2026-10-09-phase-3-3b-index-and-publishing-design.md
# §3.1, §3.2). Both were already in the lockfile.
serde = { workspace = true }
serde_json = { workspace = true }
```

`crates/nova-index/src/lib.rs`, the first version:

```rust
//! The package index (spec
//! `docs/superpowers/specs/2026-10-09-phase-3-3b-index-and-publishing-design.md`):
//! reading an index over HTTPS, from a local directory or through the
//! GitHub API, downloading, unpacking and packing packages, the sync step,
//! and publishing. Everything nova does on the network is in this crate.
//! See ARCHITECTURE.md for its place in the pipeline.

mod line;
mod location;
mod read;

pub use line::{parse_config, parse_lines, Config, Line, LineDep, LINE_VERSION};
pub use location::{fill_dl, index_path, Index, Location, Source, DEFAULT_INDEX};
pub use read::{LocalReader, Reader, View};
```

`crates/nova-index/tests/index.rs`:

```rust
//! The index's format and a local index (spec
//! `docs/superpowers/specs/2026-10-09-phase-3-3b-index-and-publishing-design.md`
//! §3, §10.3).

use std::path::{Path, PathBuf};

use nova_index::{
    fill_dl, index_path, parse_config, parse_lines, Index, Line, LineDep, LocalReader, Location,
    Source, View, DEFAULT_INDEX,
};
use nova_pm::IndexView;

/// A fresh, empty directory with a fixed name, so each run replaces the last.
fn fresh(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("nova-index-index-{name}"));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn write(dir: &Path, path: &str, text: &str) {
    let path = dir.join(path);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, text).unwrap();
}

fn line(name: &str, vers: &str, deps: &[(&str, &str)]) -> Line {
    Line {
        name: name.into(),
        vers: vers.into(),
        deps: deps
            .iter()
            .map(|(n, r)| LineDep {
                name: n.to_string(),
                req: r.to_string(),
            })
            .collect(),
        cksum: "a".repeat(64),
        v: 1,
    }
}

#[test]
fn index_paths_follow_cargos_sparse_layout_in_lower_case() {
    for (name, path) in [
        ("a", "1/a"),
        ("ab", "2/ab"),
        ("abc", "3/a/abc"),
        ("geom", "ge/om/geom"),
        ("json-api", "js/on/json-api"),
        ("Geom", "ge/om/geom"),
        ("JSON_X", "js/on/json_x"),
    ] {
        assert_eq!(index_path(name), path, "{name}");
    }
}

#[test]
fn an_index_is_read_over_https_or_from_a_local_directory() {
    let http = Index::new("HTTPS://Example.Test/index").unwrap();
    assert_eq!(http.canonical, "https://example.test/index/");
    assert_eq!(http.source, Source::Http("https://example.test/index/".into()));
    let dir = fresh("local");
    let local = Index::new(dir.to_str().unwrap()).unwrap();
    assert!(local.canonical.starts_with("file:///"), "{}", local.canonical);
    let Source::Local(found) = &local.source else {
        panic!("{:?}", local.source);
    };
    assert_eq!(nova_pm::real_path(found), nova_pm::real_path(&dir));
    assert!(Index::new("http://example.test/").is_err());
    assert!(Index::new("relative/dir").is_err());
}

#[test]
fn an_unset_or_empty_nova_index_is_the_default() {
    for setting in [None, Some(""), Some("  ")] {
        let index = Index::from_setting(setting).unwrap();
        assert_eq!(index.canonical, DEFAULT_INDEX, "{setting:?}");
    }
    let error = Index::from_setting(Some("http://example.test/")).unwrap_err();
    assert!(error.starts_with("NOVA_INDEX: "), "{error}");
}

#[test]
fn a_tarball_is_at_an_absolute_url_or_relative_to_the_root() {
    let http = Index::new("https://example.test/index").unwrap();
    assert_eq!(
        http.tarball("dl/{name}-{version}.nova-pkg", "geom", "0.2.0")
            .unwrap(),
        Location::Url("https://example.test/index/dl/geom-0.2.0.nova-pkg".into())
    );
    assert_eq!(
        http.tarball("https://cdn.example.test/{name}/{version}", "geom", "0.2.0")
            .unwrap(),
        Location::Url("https://cdn.example.test/geom/0.2.0".into())
    );
    let dir = fresh("tarball");
    let local = Index::new(dir.to_str().unwrap()).unwrap();
    match local
        .tarball("dl/{name}-{version}.nova-pkg", "geom", "0.2.0")
        .unwrap()
    {
        Location::File(path) => assert!(path.ends_with("dl/geom-0.2.0.nova-pkg"), "{path:?}"),
        other => panic!("{other:?}"),
    }
    for bad in ["../{name}", "/abs/{name}", "a/../../{name}", "C:/x/{name}", "a\\b/{name}", ""] {
        assert!(http.tarball(bad, "geom", "0.2.0").is_err(), "{bad}");
    }
    assert_eq!(
        fill_dl("{name}/{version}/{name}", "geom", "1.0.0"),
        "geom/1.0.0/geom"
    );
}

#[test]
fn a_line_is_written_as_one_json_object() {
    assert_eq!(
        line("geom", "0.2.0", &[("json-api", "^1.0")]).to_json(),
        format!(
            "{{\"name\":\"geom\",\"vers\":\"0.2.0\",\"deps\":[{{\"name\":\"json-api\",\
             \"req\":\"^1.0\"}}],\"cksum\":\"{}\",\"v\":1}}",
            "a".repeat(64)
        )
    );
}

#[test]
fn lines_of_a_later_format_are_skipped_and_bad_ones_noted() {
    let good = line("geom", "0.2.0", &[]).to_json();
    let later = good.replace("\"v\":1", "\"v\":2");
    let text = format!(
        "{good}\n{later}\nnot json\n{{\"name\":\"geom\"}}\n{}\n\n",
        good.replace("0.2.0", "zero")
    );
    let (lines, notes) = parse_lines(&text, "ge/om/geom");
    assert_eq!(lines, [line("geom", "0.2.0", &[])]);
    assert_eq!(notes.len(), 3, "{notes:?}");
    assert!(
        notes[0].starts_with("ge/om/geom:3: skipped a line that does not parse"),
        "{notes:?}"
    );
    assert!(notes[1].starts_with("ge/om/geom:4:"), "{notes:?}");
    assert!(notes[2].starts_with("ge/om/geom:5:"), "{notes:?}");
}

#[test]
fn config_json_has_dl_and_perhaps_api() {
    let config = parse_config(
        "{\"dl\": \"dl/{name}-{version}.nova-pkg\", \"api\": \"Sakeerin/nova-index\"}",
    )
    .unwrap();
    assert_eq!(config.dl, "dl/{name}-{version}.nova-pkg");
    assert_eq!(config.api.as_deref(), Some("Sakeerin/nova-index"));
    assert_eq!(parse_config("{\"dl\": \"x\"}").unwrap().api, None);
    assert!(parse_config("{}").is_err());
}

#[test]
fn a_local_index_is_read_through_a_view() {
    let dir = fresh("view");
    let text = format!(
        "{}\n{}\nnot json\n",
        line("geom", "0.1.0", &[]).to_json(),
        line("geom", "0.2.0", &[("json", "^1")]).to_json()
    );
    write(&dir, "ge/om/geom", &text);
    let mut reader = LocalReader { dir: dir.clone() };
    let mut view = View::new(&mut reader);
    let versions = view.versions("geom").unwrap().unwrap();
    let found: Vec<String> = versions.iter().map(|c| c.version.to_string()).collect();
    assert_eq!(found, ["0.1.0", "0.2.0"]);
    assert_eq!(versions[1].deps[0].0, "json");
    assert_eq!(view.versions("nope").unwrap(), None);
    assert_eq!(view.notes.len(), 1, "{:?}", view.notes);
}

#[test]
fn a_view_compares_names_exactly() {
    let dir = fresh("exact");
    write(
        &dir,
        "ge/om/geom",
        &format!("{}\n", line("Geom", "1.0.0", &[]).to_json()),
    );
    let mut reader = LocalReader { dir };
    let mut view = View::new(&mut reader);
    assert_eq!(view.versions("geom").unwrap(), None);
    assert!(view.versions("Geom").unwrap().is_some());
}
```

Add the crate to the lockfile, then run the tests:

```bash
cd /d/Projects/nona/nova && cargo check -p nova-index --offline > /dev/null 2>&1; git diff --stat -- Cargo.lock; cargo test --locked -p nova-index --test index 2>&1 | grep -E "^error" | head -3
```

- [ ] **Step 2: Verify they fail**

Expected:
- `Cargo.lock` gains the `nova-index` entry only;
- `error[E0583]: file not found for module` naming `line`, `location`
  and `read`.

- [ ] **Step 3: Implement**

`crates/nova-index/src/location.rs`:

```rust
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
```

`crates/nova-index/src/line.rs`:

```rust
//! An index file's lines, and `config.json` (spec §3.1, §3.2).

use nova_pm::Candidate;
use semver::{Version, VersionReq};
use serde::{Deserialize, Serialize};

/// The line format this nova reads and writes.
pub const LINE_VERSION: u64 = 1;

/// One version of a package: one line of its index file. The fields are in
/// the order they are written.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Line {
    pub name: String,
    pub vers: String,
    /// Its `[dependencies]`, never its dev-dependencies.
    pub deps: Vec<LineDep>,
    /// The tarball's SHA-256, 64 lower-case hex digits.
    pub cksum: String,
    pub v: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LineDep {
    pub name: String,
    pub req: String,
}

impl Line {
    /// The line as an index file holds it, without its `\n`.
    pub fn to_json(&self) -> String {
        serde_json::to_string(self).expect("a line always serializes")
    }

    /// The resolver's view of this version.
    pub fn candidate(&self) -> Result<Candidate, String> {
        let version =
            Version::parse(&self.vers).map_err(|e| format!("version `{}`: {e}", self.vers))?;
        let deps = self
            .deps
            .iter()
            .map(|dep| {
                VersionReq::parse(&dep.req)
                    .map(|req| (dep.name.clone(), req))
                    .map_err(|e| format!("requirement `{}` on `{}`: {e}", dep.req, dep.name))
            })
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Candidate {
            version,
            deps,
            checksum: self.cksum.clone(),
        })
    }
}

/// The lines of an index file that `file` names. A line of a later format
/// (its `v` is not [`LINE_VERSION`]) is skipped silently, so the format
/// can grow; a line that does not parse is skipped with a note, and never
/// stops a command (spec §3.1).
pub fn parse_lines(text: &str, file: &str) -> (Vec<Line>, Vec<String>) {
    let mut lines = Vec::new();
    let mut notes = Vec::new();
    for (index, raw) in text.lines().enumerate() {
        let raw = raw.trim();
        if raw.is_empty() {
            continue;
        }
        let note = |reason: String| {
            format!(
                "{file}:{}: skipped a line that does not parse: {reason}",
                index + 1
            )
        };
        let value: serde_json::Value = match serde_json::from_str(raw) {
            Ok(value) => value,
            Err(error) => {
                notes.push(note(error.to_string()));
                continue;
            }
        };
        match value.get("v").and_then(serde_json::Value::as_u64) {
            Some(LINE_VERSION) => {}
            Some(_) => continue,
            None => {
                notes.push(note("it has no `v`".to_string()));
                continue;
            }
        }
        let parsed = serde_json::from_value::<Line>(value)
            .map_err(|e| e.to_string())
            .and_then(|line| line.candidate().map(|_| line));
        match parsed {
            Ok(line) => lines.push(line),
            Err(reason) => notes.push(note(reason)),
        }
    }
    (lines, notes)
}

/// `config.json`, at an index's root (spec §3.2).
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Config {
    /// Where tarballs are: a template with `{name}` and `{version}`.
    pub dl: String,
    /// The GitHub repository `nova publish` writes to, `owner/name`.
    #[serde(default)]
    pub api: Option<String>,
}

pub fn parse_config(text: &str) -> Result<Config, String> {
    serde_json::from_str(text).map_err(|e| format!("the index's config.json does not parse: {e}"))
}
```

`crates/nova-index/src/read.rs`:

```rust
//! Reading an index's files (spec §3.3), and the resolver's view of them.

use std::path::PathBuf;

use nova_pm::{Candidate, IndexView};

use crate::line::{parse_config, parse_lines, Config};
use crate::location::index_path;

/// How an index's files are read: from a directory, over HTTP, or through
/// the GitHub API.
pub trait Reader {
    /// The text of the file at `path`, `/`-separated under the index's
    /// root. `Ok(None)` when there is no such file.
    fn file(&mut self, path: &str) -> Result<Option<String>, String>;

    /// The index's `config.json`.
    fn config(&mut self) -> Result<Config, String> {
        match self.file("config.json")? {
            Some(text) => parse_config(&text),
            None => Err("the index has no config.json".to_string()),
        }
    }
}

/// A local index, read from its directory.
pub struct LocalReader {
    pub dir: PathBuf,
}

impl Reader for LocalReader {
    fn file(&mut self, path: &str) -> Result<Option<String>, String> {
        let full = path
            .split('/')
            .fold(self.dir.clone(), |full, part| full.join(part));
        match std::fs::read_to_string(&full) {
            Ok(text) => Ok(Some(text)),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(error) => Err(format!("cannot read {}: {error}", full.display())),
        }
    }
}

/// The resolver's view of an index through a [`Reader`]. Each name's file
/// is read when the resolver first asks for it, and the notes about lines
/// that do not parse are kept for the command to print.
pub struct View<'a> {
    reader: &'a mut dyn Reader,
    pub notes: Vec<String>,
}

impl<'a> View<'a> {
    pub fn new(reader: &'a mut dyn Reader) -> View<'a> {
        View {
            reader,
            notes: Vec::new(),
        }
    }
}

impl IndexView for View<'_> {
    fn versions(&mut self, name: &str) -> Result<Option<Vec<Candidate>>, String> {
        let path = index_path(name);
        let Some(text) = self.reader.file(&path)? else {
            return Ok(None);
        };
        let (lines, notes) = parse_lines(&text, &path);
        self.notes.extend(notes);
        // `Geom` and `geom` share a file; names compare exactly, as
        // manifests do (spec §3.1).
        let candidates: Vec<Candidate> = lines
            .iter()
            .filter(|line| line.name == name)
            .filter_map(|line| line.candidate().ok())
            .collect();
        Ok((!candidates.is_empty()).then_some(candidates))
    }
}
```

- [ ] **Step 4: Run the tests to verify they pass**

```bash
cd /d/Projects/nona/nova && P=/c/Users/SAKEER~1/AppData/Local/Temp/gcm/p34 && cargo test --locked -p nova-index > $P/t6.txt 2>&1; echo "exit=$?"; python -X utf8 $P/count.py $P/t6.txt; grep -E "FAILED|panicked|^warning|^error" $P/t6.txt | head
```

Expected: `exit=0`, 0 failed, no warnings, 9 tests.

- [ ] **Step 5: Commit**

Write `$P/msg-6.txt`:

```
nova-index: the index's format, and a local index

A new crate holds everything nova does with a package index (spec
docs/superpowers/specs/2026-10-09-phase-3-3b-index-and-publishing-design.md
§9). It is `nova-index` because `nova-registry` is taken on crates.io.
This first part needs no network:
- `Index` is an index by its canonical form, read over HTTP or from a
  local directory; `NOVA_INDEX` names it, and the default is
  Sakeerin/nova-index on raw.githubusercontent.com (§3.3).
- `index_path` is Cargo's sparse layout in lower case (§3.1).
- `Line` is one version, one JSON line. A line of a later format is
  skipped silently; one that does not parse is skipped with a note
  naming the file and line, which never stops a command.
- `Config` is config.json: `dl`, a URL template absolute or relative to
  the index's root, and `api`, the GitHub repository publishing writes.
- `Reader` reads an index's files; `LocalReader` reads a directory.
  `View` is the resolver's `IndexView` over a reader, comparing names
  exactly.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
```

```bash
cd /d/Projects/nona/nova && P=/c/Users/SAKEER~1/AppData/Local/Temp/gcm/p34 && cargo fmt --all && git add crates/nova-index Cargo.lock && git commit -q -F $P/msg-6.txt && git log -1 --format=%s
```

Expected: `nova-index: the index's format, and a local index`.

The task's test command: `cargo test --locked -p nova-index`.

---

### Task 7: Packing and unpacking

Spec §5.1, §5.2, §6.5, §8, §10.3.

**Files:**
- Modify: `crates/nova-pm/src/name.rs`, `crates/nova-pm/src/lib.rs`, `crates/nova-pm/tests/names.rs` (`is_portable`)
- Create: `crates/nova-index/src/pack.rs`, `crates/nova-index/src/cache.rs`
- Modify: `crates/nova-index/src/lib.rs`, `crates/nova-index/Cargo.toml`, `Cargo.lock`
- Create: `crates/nova-index/tests/pack.rs`

**Interfaces:**
- Consumes: `nova_pm::parse` (the manifest), Task 6's crate.
- Produces:
  - `nova_pm::is_portable(&str) -> bool`;
  - `nova_index::MAX_TARBALL: u64` (10 MiB);
  - `Packed { bytes: Vec<u8>, files: usize, checksum: String }`;
  - `pack(&Path, &str, &semver::Version) -> Result<Packed, String>`;
  - `sha256_hex(&[u8]) -> String`;
  - `Limits { bytes: u64, entries: usize }`, and `LIMITS` (100 MiB,
    10,000 entries);
  - `unpack(&[u8], &str, &Version, &[String], &Path) -> Result<(), String>`;
  - `unpack_limited(&[u8], &str, &Version, &[String], &Path, &Limits) -> Result<(), String>`.
- The tarball: entries `<name>-<version>/<path>`, GNU headers, regular
  files only, mode `0644`, modification time 0, owner and group 0, sorted
  by path; gzip with modification time 0, no file name, OS byte 255.
- `unpack` writes to a temporary directory beside `dest`, named
  `.<name>-<version>.<pid>-<n>.tmp`, then renames it into place. When
  `dest` already holds a `nova.toml`, it does nothing.

- [ ] **Step 1: Write the failing tests**

Append to `crates/nova-pm/tests/names.rs`:

```rust
#[test]
fn a_portable_name_is_one_every_system_can_hold() {
    for good in ["lib.nova", "README.ไทย.md", "a-b_c", "LICENSE", "con-x.nova", "x.y.z"] {
        assert!(nova_pm::is_portable(good), "{good}");
    }
    for bad in [
        "", ".", "..", "a:b", "a\\b", "a/b", "a<b", "a>b", "a\"b", "a|b", "a?b", "a*b",
        "tab\there", "dot.", "space ", "con", "CON.nova", "aux.txt", "nul", "com1.x", "LPT9",
    ] {
        assert!(!nova_pm::is_portable(bad), "{bad:?}");
    }
}
```

If `names.rs` does not import from `nova_pm` by path, keep the full
`nova_pm::is_portable` as written.

`crates/nova-index/tests/pack.rs`:

```rust
//! Packing and unpacking (spec
//! `docs/superpowers/specs/2026-10-09-phase-3-3b-index-and-publishing-design.md`
//! §5.2, §6.5, §10.3).

use std::io::Read;
use std::path::{Path, PathBuf};

use nova_index::{pack, sha256_hex, unpack, unpack_limited, Limits};
use semver::Version;

/// A fresh, empty directory with a fixed name, so each run replaces the last.
fn fresh(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("nova-index-pack-{name}"));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn write(dir: &Path, path: &str, bytes: &[u8]) {
    let path = dir.join(path);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, bytes).unwrap();
}

fn manifest(name: &str, version: &str, extra: &str) -> String {
    format!("[package]\nname = \"{name}\"\nversion = \"{version}\"\nedition = \"2026\"\n{extra}")
}

/// The library `geom` 0.1.0 at `dir`.
fn geom(dir: &Path) {
    write(dir, "nova.toml", manifest("geom", "0.1.0", "").as_bytes());
    write(dir, "src/lib.nova", b"pub fn area() -> Int {\n    9\n}\n");
}

fn v010() -> Version {
    Version::new(0, 1, 0)
}

/// Each entry's path and header, in order.
fn entries(tarball: &[u8]) -> Vec<(String, tar::Header)> {
    let mut archive = tar::Archive::new(flate2::read::GzDecoder::new(tarball));
    archive
        .entries()
        .unwrap()
        .map(|entry| {
            let entry = entry.unwrap();
            let path = String::from_utf8(entry.path_bytes().into_owned()).unwrap();
            (path, entry.header().clone())
        })
        .collect()
}

/// A tarball holding each `(path, entry type, data)` exactly as given, with
/// no checks on the path, as a hostile index could serve. The path is
/// written straight into the header, since `tar`'s own setters refuse `..`
/// and absolute paths. Paths and data are `&str`, so every case's tuple
/// has one type.
fn crafted(entries: &[(&str, tar::EntryType, &str)]) -> Vec<u8> {
    let mut builder = tar::Builder::new(Vec::new());
    for (path, kind, data) in entries {
        let mut header = tar::Header::new_gnu();
        header.as_old_mut().name[..path.len()].copy_from_slice(path.as_bytes());
        header.set_entry_type(*kind);
        header.set_size(data.len() as u64);
        header.set_mode(0o644);
        if *kind == tar::EntryType::Symlink {
            header.set_link_name("elsewhere").unwrap();
        }
        header.set_cksum();
        builder.append(&header, data.as_bytes()).unwrap();
    }
    let tar = builder.into_inner().unwrap();
    let mut gz = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
    std::io::Write::write_all(&mut gz, &tar).unwrap();
    gz.finish().unwrap()
}

const GEOM_TOML: &str =
    "[package]\nname = \"geom\"\nversion = \"0.1.0\"\nedition = \"2026\"\n";

#[test]
fn packing_twice_gives_identical_bytes() {
    let dir = fresh("twice");
    geom(&dir);
    let first = pack(&dir, "geom", &v010()).unwrap();
    let second = pack(&dir, "geom", &v010()).unwrap();
    assert_eq!(first.bytes, second.bytes);
    assert_eq!(first.checksum, sha256_hex(&first.bytes));
    assert_eq!(first.checksum.len(), 64);
    assert!(first
        .checksum
        .bytes()
        .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b)));
    assert_eq!(first.files, 2);
}

#[test]
fn only_the_published_files_are_packed() {
    let dir = fresh("published");
    geom(&dir);
    for path in [
        "src/util/more.nova",
        "tests/area.nova",
        "README.md",
        "LICENSE",
        ".git/config",
        "src/.hidden.nova",
        "target/debug/x",
        "nova.lock",
        "notes.txt",
        "docs/guide.md",
    ] {
        write(&dir, path, b"x");
    }
    let packed = pack(&dir, "geom", &v010()).unwrap();
    let listed = entries(&packed.bytes);
    let paths: Vec<&str> = listed.iter().map(|(p, _)| p.as_str()).collect();
    assert_eq!(
        paths,
        [
            "geom-0.1.0/LICENSE",
            "geom-0.1.0/README.md",
            "geom-0.1.0/nova.toml",
            "geom-0.1.0/src/lib.nova",
            "geom-0.1.0/src/util/more.nova",
            "geom-0.1.0/tests/area.nova",
        ]
    );
    for (path, header) in &listed {
        assert_eq!(header.mode().unwrap(), 0o644, "{path}");
        assert_eq!(header.mtime().unwrap(), 0, "{path}");
        assert_eq!(header.uid().unwrap(), 0, "{path}");
        assert_eq!(header.entry_type(), tar::EntryType::Regular, "{path}");
    }
    assert_eq!(packed.files, 6);
}

#[test]
fn a_file_name_outside_ascii_survives_packing_and_unpacking() {
    // Review Focus 3.
    let dir = fresh("thai");
    geom(&dir);
    let thai = "README.ไทย.md";
    write(&dir, thai, "สวัสดี\n".as_bytes());
    let packed = pack(&dir, "geom", &v010()).unwrap();
    assert!(entries(&packed.bytes)
        .iter()
        .any(|(path, _)| path == &format!("geom-0.1.0/{thai}")));
    let dest = fresh("thai-out").join("geom-0.1.0");
    unpack(&packed.bytes, "geom", &v010(), &[], &dest).unwrap();
    assert_eq!(
        std::fs::read_to_string(dest.join(thai)).unwrap(),
        "สวัสดี\n"
    );
}

/// A symbolic link at `path` to `target`; `false` where links need a
/// privilege this run lacks (Windows without developer mode).
fn link(target: &Path, path: &Path) -> bool {
    #[cfg(unix)]
    return std::os::unix::fs::symlink(target, path).is_ok();
    #[cfg(windows)]
    return std::os::windows::fs::symlink_file(target, path).is_ok();
}

#[test]
fn a_symbolic_link_is_refused() {
    let dir = fresh("link");
    geom(&dir);
    if !link(&dir.join("src/lib.nova"), &dir.join("src/again.nova")) {
        return;
    }
    let error = pack(&dir, "geom", &v010()).unwrap_err();
    assert!(error.contains("symbolic link"), "{error}");
    assert!(error.contains("again.nova"), "{error}");
}

#[cfg(unix)]
#[test]
fn a_name_that_is_not_portable_is_refused() {
    let dir = fresh("not-portable");
    geom(&dir);
    write(&dir, "src/a:b.nova", b"");
    let error = pack(&dir, "geom", &v010()).unwrap_err();
    assert!(error.contains("a:b.nova"), "{error}");
}

#[cfg(target_os = "linux")]
#[test]
fn two_names_differing_only_in_case_are_refused() {
    let dir = fresh("case");
    geom(&dir);
    write(&dir, "src/Util.nova", b"");
    write(&dir, "src/util.nova", b"");
    let error = pack(&dir, "geom", &v010()).unwrap_err();
    assert!(error.contains("differ only in case"), "{error}");
}

#[test]
fn unpacking_refuses_what_could_escape_or_merge() {
    use tar::EntryType::{Char, Directory, Regular, Symlink};
    let cases: Vec<(&str, Vec<(&str, tar::EntryType, &str)>)> = vec![
        ("a parent directory", vec![("geom-0.1.0/../evil", Regular, "x")]),
        ("an absolute path", vec![("/etc/evil", Regular, "x")]),
        ("outside the package", vec![("other-1.0.0/x", Regular, "x")]),
        ("a link", vec![("geom-0.1.0/src/lib.nova", Symlink, "")]),
        ("a device", vec![("geom-0.1.0/dev", Char, "")]),
        ("not portable", vec![("geom-0.1.0/src/a:b.nova", Regular, "x")]),
        (
            "differ only in case",
            vec![
                ("geom-0.1.0/src/Util.nova", Regular, "x"),
                ("geom-0.1.0/src/util.nova", Regular, "x"),
            ],
        ),
        ("a backslash", vec![("geom-0.1.0\\..\\evil", Regular, "x")]),
    ];
    for (what, mut list) in cases {
        list.push(("geom-0.1.0/nova.toml", Regular, GEOM_TOML));
        list.push(("geom-0.1.0/src/", Directory, ""));
        let out = fresh("refused");
        let dest = out.join("geom-0.1.0");
        let result = unpack(&crafted(&list), "geom", &v010(), &[], &dest);
        assert!(result.is_err(), "{what}: unpacked");
        assert!(!dest.exists(), "{what}: left the package");
        let left: Vec<_> = std::fs::read_dir(&out).unwrap().collect();
        assert!(left.is_empty(), "{what}: left {left:?}");
        assert!(!out.parent().unwrap().join("evil").exists(), "{what}");
    }
}

#[test]
fn the_manifest_must_be_the_index_lines() {
    use tar::EntryType::Regular;
    let deps = ["json".to_string()];
    let cases = [
        ("another name", manifest("other", "0.1.0", "\n[dependencies]\njson = \"1\"\n")),
        ("another version", manifest("geom", "0.2.0", "\n[dependencies]\njson = \"1\"\n")),
        (
            "a path dependency",
            manifest("geom", "0.1.0", "\n[dependencies]\njson = { path = \"../json\" }\n"),
        ),
        ("other dependencies", manifest("geom", "0.1.0", "\n[dependencies]\nhttp = \"1\"\n")),
        ("a broken manifest", "[package\n".to_string()),
    ];
    for (what, text) in cases {
        let tarball = crafted(&[("geom-0.1.0/nova.toml", Regular, text.as_str())]);
        let dest = fresh("manifest").join("geom-0.1.0");
        let error = unpack(&tarball, "geom", &v010(), &deps, &dest).unwrap_err();
        assert!(!dest.exists(), "{what}");
        assert!(!error.is_empty(), "{what}");
    }
    let good = manifest("geom", "0.1.0", "\n[dependencies]\njson = \"1\"\n");
    let tarball = crafted(&[("geom-0.1.0/nova.toml", Regular, good.as_str())]);
    let dest = fresh("manifest-good").join("geom-0.1.0");
    unpack(&tarball, "geom", &v010(), &deps, &dest).unwrap();
    assert!(dest.join("nova.toml").is_file());
}

#[test]
fn limits_stop_unpacking() {
    use tar::EntryType::Regular;
    let hundred = "x".repeat(100);
    let tarball = crafted(&[
        ("geom-0.1.0/nova.toml", Regular, GEOM_TOML),
        ("geom-0.1.0/src/lib.nova", Regular, hundred.as_str()),
    ]);
    let tight = Limits {
        bytes: 99,
        entries: 10,
    };
    let dest = fresh("limits").join("geom-0.1.0");
    let error = unpack_limited(&tarball, "geom", &v010(), &[], &dest, &tight).unwrap_err();
    assert!(error.contains("the limit of 99 bytes"), "{error}");
    let few = Limits {
        bytes: 1 << 20,
        entries: 1,
    };
    let error = unpack_limited(&tarball, "geom", &v010(), &[], &dest, &few).unwrap_err();
    assert!(error.contains("entries"), "{error}");
    assert!(!dest.exists());
}

#[test]
fn a_package_already_unpacked_is_not_unpacked_again() {
    let dest = fresh("already").join("geom-0.1.0");
    write(&dest, "nova.toml", GEOM_TOML.as_bytes());
    // Not a tarball at all: it is never read.
    unpack(b"garbage", "geom", &v010(), &[], &dest).unwrap();
}

/// Two processes unpacking one package at once both succeed and leave one
/// copy (spec §5.2). Each child runs only `unpack_as_a_child_process`, its
/// output piped so its own `test result:` line stays out of this run's.
#[test]
fn two_processes_unpacking_at_once_both_succeed() {
    let out = fresh("processes");
    let source = fresh("processes-source");
    geom(&source);
    let packed = pack(&source, "geom", &v010()).unwrap();
    let tarball = out.join("geom-0.1.0.nova-pkg");
    std::fs::write(&tarball, &packed.bytes).unwrap();
    let me = std::env::current_exe().unwrap();
    let children: Vec<_> = (0..2)
        .map(|_| {
            std::process::Command::new(&me)
                .args(["--exact", "unpack_as_a_child_process", "--nocapture"])
                .env("NOVA_TEST_UNPACK", &tarball)
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
    let mut names: Vec<String> = std::fs::read_dir(out.join("src"))
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();
    assert_eq!(names, ["geom-0.1.0"]);
    let mut text = String::new();
    std::fs::File::open(out.join("src/geom-0.1.0/src/lib.nova"))
        .unwrap()
        .read_to_string(&mut text)
        .unwrap();
    assert!(text.contains("area"));
}

/// Does nothing unless `NOVA_TEST_UNPACK` names a tarball, which only the
/// test above sets, for its children. It unpacks into `src/` beside it.
#[test]
fn unpack_as_a_child_process() {
    let Some(tarball) = std::env::var_os("NOVA_TEST_UNPACK") else {
        return;
    };
    let tarball = PathBuf::from(tarball);
    let bytes = std::fs::read(&tarball).unwrap();
    let dest = tarball.parent().unwrap().join("src").join("geom-0.1.0");
    unpack(&bytes, "geom", &v010(), &[], &dest).unwrap();
    assert!(dest.join("src/lib.nova").is_file());
}
```

- [ ] **Step 2: Add the dependencies, and run the tests to verify they fail**

In `crates/nova-index/Cargo.toml`, append to `[dependencies]`:

```toml
# Packing and unpacking (spec §6.5, §5.2). tar's default features add
# xattr, which nova never reads; flate2 is the workspace's gzip; ring, as
# nova-runtime has it, gives the SHA-256 the index records.
tar = { version = "0.4", default-features = false }
flate2 = { workspace = true }
ring = "0.17"
```

Then:

```bash
cd /d/Projects/nona/nova && P=/c/Users/SAKEER~1/AppData/Local/Temp/gcm/p34 && cargo check -p nova-index > $P/c7.txt 2>&1; tail -3 $P/c7.txt; git diff -- Cargo.lock | grep -E '^[+-](name|version) = ' ; cargo test --locked -p nova-index --test pack 2>&1 | grep -E "^error" | head -3; cargo test --locked -p nova-pm --test names 2>&1 | grep -E "^error" | head -2
```

This `cargo check` is online: `tar` and what it needs are new.

Expected:
- `Cargo.lock` gains `tar` and `filetime`, and perhaps a `windows-sys`
  version with its `windows-targets` family. List every name and version
  added in the ledger.
- For each one, read its `rust_version` and `edition` from crates.io:

  ```bash
  curl -s -A "nova-build-check" https://crates.io/api/v1/crates/NAME/VERSION | python -X utf8 -c "import json,sys; v=json.load(sys.stdin)['version']; print(v['num'], v.get('rust_version'), v.get('edition'))"
  ```

  None may need more than Rust 1.78, or be edition 2024. If one does,
  stop: pin an older version with `cargo update -p NAME --precise V`,
  and ledger it.
- The tests fail to build: `error[E0432]: unresolved imports` naming
  `pack`, `sha256_hex`, `unpack`, `unpack_limited` and `Limits`; and in
  nova-pm, E0425 for `is_portable`.

- [ ] **Step 3: Implement**

In `crates/nova-pm/src/name.rs`, after `check_name`:

```rust
/// Whether `component`, one name in a path, is valid on Windows, macOS
/// and Linux (spec
/// `docs/superpowers/specs/2026-10-09-phase-3-3b-index-and-publishing-design.md`
/// §6.5):
/// - not empty, `.` or `..`;
/// - none of `/ : \ < > " | ? *` or a control character;
/// - no trailing `.` or space;
/// - not a Windows device name, with or without an extension.
pub fn is_portable(component: &str) -> bool {
    if component.is_empty() || component == "." || component == ".." {
        return false;
    }
    if component
        .chars()
        .any(|c| c.is_control() || "/:\\<>\"|?*".contains(c))
    {
        return false;
    }
    if component.ends_with('.') || component.ends_with(' ') {
        return false;
    }
    let stem = component.split('.').next().unwrap_or(component);
    !WINDOWS_DEVICES.contains(&stem.to_ascii_lowercase().as_str())
}
```

In `crates/nova-pm/src/lib.rs`, the `pub use name::…;` line becomes:

```rust
pub use name::{check_name, import_name, is_portable};
```

`crates/nova-index/src/pack.rs`:

```rust
//! `nova package`'s tarball (spec §6.5).

use std::io::Write;
use std::path::{Path, PathBuf};

use flate2::{Compression, GzBuilder};
use semver::Version;

/// The largest tarball nova packs or downloads (spec §5.2, §6.5).
pub const MAX_TARBALL: u64 = 10 * 1024 * 1024;

/// A packed package.
#[derive(Debug, Clone)]
pub struct Packed {
    pub bytes: Vec<u8>,
    /// How many files it holds.
    pub files: usize,
    /// The SHA-256 of `bytes`.
    pub checksum: String,
}

/// `bytes`'s SHA-256, as 64 lower-case hex digits.
pub fn sha256_hex(bytes: &[u8]) -> String {
    let digest = ring::digest::digest(&ring::digest::SHA256, bytes);
    digest
        .as_ref()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

/// Pack the package at `root` as `name` `version` (spec §6.5): its
/// `nova.toml`, `src/`, `tests/`, and top-level `README*` and `LICENSE*`
/// files, leaving out any name that starts with `.`. Every entry is under
/// `<name>-<version>/`, sorted, with fixed times, modes and owners, so the
/// same source always packs to the same bytes.
///
/// A symbolic link or a name that is not portable, anywhere in what would
/// be packed, refuses the whole package, as does a tarball over 10 MiB.
pub fn pack(root: &Path, name: &str, version: &Version) -> Result<Packed, String> {
    let mut files: Vec<(String, PathBuf)> = Vec::new();
    for entry in sorted_names(root)? {
        let path = root.join(&entry);
        let included = entry == "nova.toml"
            || entry == "src"
            || entry == "tests"
            || entry.starts_with("README")
            || entry.starts_with("LICENSE");
        if !included {
            continue;
        }
        let kind = kind_of(&path, &entry)?;
        match kind {
            Kind::Dir if entry == "src" || entry == "tests" => {
                walk(&path, &entry, &mut files)?
            }
            Kind::File => files.push((entry.clone(), path)),
            Kind::Dir => {}
        }
    }
    let mut lower: Vec<(String, &str)> = files
        .iter()
        .map(|(rel, _)| (rel.to_lowercase(), rel.as_str()))
        .collect();
    lower.sort();
    if let Some(pair) = lower.windows(2).find(|pair| pair[0].0 == pair[1].0) {
        return Err(format!(
            "`{}` and `{}` differ only in case, which Windows and macOS cannot hold apart",
            pair[0].1, pair[1].1
        ));
    }
    files.sort();
    let prefix = format!("{name}-{version}");
    let mut tar = tar::Builder::new(Vec::new());
    for (rel, path) in &files {
        let data =
            std::fs::read(path).map_err(|e| format!("cannot read {}: {e}", path.display()))?;
        let mut header = tar::Header::new_gnu();
        header.set_entry_type(tar::EntryType::Regular);
        header.set_size(data.len() as u64);
        header.set_mode(0o644);
        header.set_mtime(0);
        header.set_uid(0);
        header.set_gid(0);
        tar.append_data(&mut header, format!("{prefix}/{rel}"), data.as_slice())
            .map_err(|e| format!("cannot pack {rel}: {e}"))?;
    }
    let tar = tar
        .into_inner()
        .map_err(|e| format!("cannot pack: {e}"))?;
    let mut gz = GzBuilder::new()
        .mtime(0)
        .operating_system(255)
        .write(Vec::new(), Compression::default());
    gz.write_all(&tar)
        .map_err(|e| format!("cannot compress: {e}"))?;
    let bytes = gz.finish().map_err(|e| format!("cannot compress: {e}"))?;
    if bytes.len() as u64 > MAX_TARBALL {
        return Err(format!(
            "the package is {} bytes packed; the limit is 10 MiB",
            bytes.len()
        ));
    }
    let checksum = sha256_hex(&bytes);
    Ok(Packed {
        bytes,
        files: files.len(),
        checksum,
    })
}

enum Kind {
    File,
    Dir,
}

/// What `path` is, refusing a link (packing never follows one), anything
/// but a file or a directory, and a name that is not portable.
fn kind_of(path: &Path, rel: &str) -> Result<Kind, String> {
    let metadata = std::fs::symlink_metadata(path)
        .map_err(|e| format!("cannot read {}: {e}", path.display()))?;
    if metadata.file_type().is_symlink() {
        return Err(format!(
            "{rel} is a symbolic link; nova package never follows links"
        ));
    }
    let name = rel.rsplit('/').next().unwrap_or(rel);
    if !nova_pm::is_portable(name) {
        return Err(format!(
            "{rel}: `{name}` is not a name every system can hold (spec §6.5)"
        ));
    }
    if metadata.is_dir() {
        Ok(Kind::Dir)
    } else if metadata.is_file() {
        Ok(Kind::File)
    } else {
        Err(format!("{rel} is neither a file nor a directory"))
    }
}

/// Every file under `dir`, whose path in the package is `rel`, leaving out
/// names that start with `.`.
fn walk(dir: &Path, rel: &str, files: &mut Vec<(String, PathBuf)>) -> Result<(), String> {
    for entry in sorted_names(dir)? {
        if entry.starts_with('.') {
            continue;
        }
        let path = dir.join(&entry);
        let child = format!("{rel}/{entry}");
        match kind_of(&path, &child)? {
            Kind::Dir => walk(&path, &child, files)?,
            Kind::File => files.push((child, path)),
        }
    }
    Ok(())
}

/// The names in `dir`, sorted. A name that is not UTF-8 is refused.
fn sorted_names(dir: &Path) -> Result<Vec<String>, String> {
    let entries =
        std::fs::read_dir(dir).map_err(|e| format!("cannot read {}: {e}", dir.display()))?;
    let mut names = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|e| format!("cannot read {}: {e}", dir.display()))?;
        let name = entry.file_name().into_string().map_err(|name| {
            format!(
                "{}: `{}` is not UTF-8",
                dir.display(),
                name.to_string_lossy()
            )
        })?;
        names.push(name);
    }
    names.sort();
    Ok(names)
}
```

`crates/nova-index/src/cache.rs`:

```rust
//! Unpacking a downloaded package into the cache (spec §5.1, §5.2).

use std::collections::HashSet;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use semver::Version;

/// How much unpacking may write.
#[derive(Debug, Clone, Copy)]
pub struct Limits {
    /// Bytes of file contents.
    pub bytes: u64,
    /// Entries in the tarball.
    pub entries: usize,
}

/// 100 MiB and 10,000 entries (spec §5.2).
pub const LIMITS: Limits = Limits {
    bytes: 100 * 1024 * 1024,
    entries: 10_000,
};

/// [`unpack_limited`] within [`LIMITS`].
pub fn unpack(
    tarball: &[u8],
    name: &str,
    version: &Version,
    deps: &[String],
    dest: &Path,
) -> Result<(), String> {
    unpack_limited(tarball, name, version, deps, dest, &LIMITS)
}

/// Unpack `tarball`, the package `name` `version` whose index line names
/// `deps` (sorted), into `dest` (spec §5.2). Nothing is written to `dest`
/// unless every check passes:
/// - each entry is a file or a directory under `<name>-<version>/`, with
///   portable names, no two differing only in case;
/// - within `limits`;
/// - its `nova.toml` names `name` and `version`, has no path entry, and its
///   `[dependencies]` are `deps`.
///
/// The package is unpacked beside `dest` under a name made unique by the
/// process id and a counter, then renamed into place. If another process
/// got there first, its copy is used. When `dest` already holds a
/// `nova.toml`, nothing is done.
pub fn unpack_limited(
    tarball: &[u8],
    name: &str,
    version: &Version,
    deps: &[String],
    dest: &Path,
    limits: &Limits,
) -> Result<(), String> {
    if dest.join(nova_pm::MANIFEST).is_file() {
        return Ok(());
    }
    let parent = dest
        .parent()
        .ok_or_else(|| format!("{} has no parent directory", dest.display()))?;
    std::fs::create_dir_all(parent).map_err(|e| cannot_write(parent, &e))?;
    let temp = new_temp_dir(parent, &format!("{name}-{version}"))?;
    let result = extract(tarball, name, version, &temp, limits)
        .and_then(|()| check_manifest(&temp, name, version, deps));
    if let Err(error) = result {
        let _ = std::fs::remove_dir_all(&temp);
        return Err(error);
    }
    match std::fs::rename(&temp, dest) {
        Ok(()) => Ok(()),
        Err(error) => {
            let _ = std::fs::remove_dir_all(&temp);
            // Another process may have put its copy there first.
            if dest.join(nova_pm::MANIFEST).is_file() {
                Ok(())
            } else {
                Err(cannot_write(dest, &error))
            }
        }
    }
}

fn cannot_write(path: &Path, error: &std::io::Error) -> String {
    format!(
        "cannot write the package cache at {}: {error}; set NOVA_HOME to a writable directory",
        path.display()
    )
}

static TEMP_COUNTER: AtomicU64 = AtomicU64::new(0);

/// A new, empty directory in `parent` for unpacking `stem`. A name a killed
/// run left behind is never read: `create_dir` refuses it, and the next
/// number is tried.
fn new_temp_dir(parent: &Path, stem: &str) -> Result<PathBuf, String> {
    loop {
        let n = TEMP_COUNTER.fetch_add(1, Ordering::Relaxed);
        let temp = parent.join(format!(".{stem}.{}-{n}.tmp", std::process::id()));
        match std::fs::create_dir(&temp) {
            Ok(()) => return Ok(temp),
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(error) => return Err(cannot_write(&temp, &error)),
        }
    }
}

/// Write the tarball's entries under `root`, checking each.
fn extract(
    tarball: &[u8],
    name: &str,
    version: &Version,
    root: &Path,
    limits: &Limits,
) -> Result<(), String> {
    let prefix = format!("{name}-{version}");
    let bad = |why: String| format!("the tarball of {name} {version} is refused: {why}");
    let mut archive = tar::Archive::new(flate2::read::GzDecoder::new(tarball));
    let entries = archive.entries().map_err(|e| bad(e.to_string()))?;
    let mut seen: HashSet<String> = HashSet::new();
    let mut count = 0usize;
    let mut written = 0u64;
    for entry in entries {
        let mut entry = entry.map_err(|e| bad(e.to_string()))?;
        count += 1;
        if count > limits.entries {
            return Err(bad(format!("it has more than {} entries", limits.entries)));
        }
        let path = String::from_utf8(entry.path_bytes().into_owned())
            .map_err(|_| bad("an entry's name is not UTF-8".to_string()))?;
        let kind = entry.header().entry_type();
        if kind != tar::EntryType::Regular && kind != tar::EntryType::Directory {
            return Err(bad(format!("`{path}` is not a file or a directory")));
        }
        let mut parts: Vec<&str> = path.split('/').collect();
        if kind == tar::EntryType::Directory && parts.last() == Some(&"") {
            parts.pop();
        }
        if parts.first() != Some(&prefix.as_str()) {
            return Err(bad(format!("`{path}` is not under {prefix}/")));
        }
        let rest = &parts[1..];
        if let Some(part) = rest.iter().find(|part| !nova_pm::is_portable(part)) {
            return Err(bad(format!("`{path}`: `{part}` is not a portable name")));
        }
        if rest.is_empty() {
            continue;
        }
        let rel = rest.join("/");
        if !seen.insert(rel.to_lowercase()) {
            return Err(bad(format!(
                "`{path}` differs only in case from another entry"
            )));
        }
        let target = rest.iter().fold(root.to_path_buf(), |p, part| p.join(part));
        if kind == tar::EntryType::Directory {
            std::fs::create_dir_all(&target).map_err(|e| cannot_write(&target, &e))?;
            continue;
        }
        if let Some(dir) = target.parent() {
            std::fs::create_dir_all(dir).map_err(|e| cannot_write(dir, &e))?;
        }
        let room = limits.bytes - written;
        let mut data = Vec::new();
        (&mut entry)
            .take(room + 1)
            .read_to_end(&mut data)
            .map_err(|e| bad(e.to_string()))?;
        if data.len() as u64 > room {
            return Err(bad(format!(
                "its contents pass the limit of {} bytes",
                limits.bytes
            )));
        }
        written += data.len() as u64;
        std::fs::write(&target, &data).map_err(|e| cannot_write(&target, &e))?;
    }
    Ok(())
}

/// The unpacked `nova.toml` must be the index line's (spec §5.2).
fn check_manifest(root: &Path, name: &str, version: &Version, deps: &[String]) -> Result<(), String> {
    let bad = |why: String| format!("the downloaded {name} {version} is refused: {why}");
    let path = root.join(nova_pm::MANIFEST);
    let text = std::fs::read_to_string(&path).map_err(|_| bad("it has no nova.toml".into()))?;
    let (manifest, _) = nova_pm::parse(&text, nova_diagnostics::FileId::DUMMY);
    let manifest = manifest.ok_or_else(|| bad("its nova.toml has errors".into()))?;
    if manifest.package.name != name || manifest.package.version != *version {
        return Err(bad(format!(
            "its nova.toml is {} {}",
            manifest.package.name, manifest.package.version
        )));
    }
    if let Some(entry) = manifest.dependencies.iter().find(|d| d.path.is_some()) {
        return Err(bad(format!("it has a path dependency `{}`", entry.name)));
    }
    let mut names: Vec<String> = manifest
        .dependencies
        .iter()
        .map(|d| d.name.clone())
        .collect();
    names.sort();
    if names != deps {
        return Err(bad(format!(
            "its dependencies are [{}], and its index line's are [{}]",
            names.join(", "),
            deps.join(", ")
        )));
    }
    Ok(())
}
```

In `crates/nova-index/src/lib.rs`, add `mod cache;` and `mod pack;`, and:

```rust
pub use cache::{unpack, unpack_limited, Limits, LIMITS};
pub use pack::{pack, sha256_hex, Packed, MAX_TARBALL};
```

- [ ] **Step 4: Run the tests to verify they pass**

```bash
cd /d/Projects/nona/nova && P=/c/Users/SAKEER~1/AppData/Local/Temp/gcm/p34 && cargo test --locked -p nova-pm -p nova-index > $P/t7.txt 2>&1; echo "exit=$?"; python -X utf8 $P/count.py $P/t7.txt; grep -E "FAILED|panicked|^warning|^error" $P/t7.txt | head
```

Expected:
- `exit=0`, 0 failed, no warnings;
- on Windows, 10 new tests in `pack.rs` (`a_name_that_is_not_portable_is_refused`
  and `two_names_differing_only_in_case_are_refused` are Unix and Linux
  only), and 1 in `names.rs`.

If `a_symbolic_link_is_refused` returned early, because this Windows has
no developer mode, ledger that. It runs on Linux and macOS in CI.

Then run the two Unix-only tests on Linux:

```bash
cd /d/Projects/nona/nova && git add -A crates/nova-index crates/nova-pm && bash /c/Users/SAKEER~1/AppData/Local/Temp/gcm/linux/run.sh test --locked -p nova-index --test pack 2>&1 | tail -5
```

Expected: `test result: ok. 12 passed; 0 failed`.

- [ ] **Step 5: Commit**

Write `$P/msg-7.txt`:

```
nova-index: packing and unpacking

`pack` makes the tarball `nova package` writes (spec
docs/superpowers/specs/2026-10-09-phase-3-3b-index-and-publishing-design.md
§6.5): nova.toml, src/, tests/ and the top-level README* and LICENSE*
files, under <name>-<version>/, leaving out any name starting with `.`.
It is reproducible: entries sorted, files only, mode 0644, time 0,
owner 0, and gzip with no time or file name. A symbolic link, a name
that some system cannot hold, two names differing only in case, or more
than 10 MiB refuse the whole package.

`unpack` refuses anything that could write outside the package or that
Windows or macOS would merge: a `..`, an absolute path, a link, a
device, an entry outside <name>-<version>/, a name that is not portable,
or two differing only in case. It stops at 100 MiB or 10,000 entries.
The manifest must be the index line's: its name, version and
dependencies, with no path entry. It unpacks under a name unique to the
process, then renames into place, so two processes can share the cache.

`nova_pm::is_portable` is the name rule, beside the package name's.
New in Cargo.lock: tar and what it needs (listed in the plan's ledger).

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
```

```bash
cd /d/Projects/nona/nova && P=/c/Users/SAKEER~1/AppData/Local/Temp/gcm/p34 && cargo fmt --all && git add crates/nova-pm crates/nova-index Cargo.lock && git commit -q -F $P/msg-7.txt && git log -1 --format=%s
```

Expected: `nova-index: packing and unpacking`.

The task's test command: `cargo test --locked -p nova-pm -p nova-index`.

---

### Task 8: HTTP and downloads

Spec §3.3, §5.1, §5.2, §8, §9, §10.4.

**Files:**
- Create: `.cargo/config.toml`
- Create: `crates/nova-index/src/http.rs`, `crates/nova-index/src/download.rs`
- Modify: `crates/nova-index/src/read.rs` (`HttpReader`, `reader_for`), `crates/nova-index/src/lib.rs`, `crates/nova-index/Cargo.toml`, `Cargo.lock`
- Create: `crates/nova-index/tests/support/mod.rs`, `crates/nova-index/tests/http.rs`

**Interfaces:**
- Consumes:
  - Task 6's `Index`, `Location`, `Config` and `Reader`;
  - Task 7's `unpack`, `sha256_hex` and `MAX_TARBALL`;
  - Task 2's `LockedPackage`.
- Produces:
  - `Http::new() -> Http` and `Http::get(&self, url: &str, limit: u64) -> Result<Option<Vec<u8>>, String>`
    (`None` on 404). `pub(crate) Http::agent(&self) -> &ureq::Agent`, for
    Task 12;
  - `check_url(&str) -> Result<(), String>`;
  - `MAX_INDEX_FILE: u64` (10 MiB);
  - `HttpReader { base: String, http: Http }`, with `HttpReader::new(&str)`;
  - `reader_for(&Index) -> Box<dyn Reader>`;
  - `fetch_package(&Index, &Config, &Http, &LockedPackage, registry: &Path) -> Result<PathBuf, String>`.
    It returns the unpacked directory;
  - `pub(crate) write_atomically(&Path, &[u8]) -> std::io::Result<()>`,
    in `download.rs`.
- The test server, `tests/support/mod.rs`:
  - `Server::start(F) -> Server`, where `F: Fn(&Request) -> Response + Send + Sync + 'static`;
  - `Server.url` (`http://127.0.0.1:<port>`, no trailing `/`) and
    `Server::requests() -> Vec<Request>`;
  - `Request { method, path, headers, body }`, with
    `Request::header(&str) -> Option<&str>`;
  - `Response { status, headers, body, cut_after: Option<usize> }`, with
    `Response::status(u16)`, `Response::ok(body)`,
    `Response::with_status(u16, body)` and `Response::redirect(&str)`.

The redirect rule: a status of 301, 302, 303, 307 or 308 is followed to its
`Location`. That goes up to five times, each target checked by
`check_url`. No request made by `Http` ever carries an `Authorization`
header.

- [ ] **Step 1: The resolver setting and the dependency**

`.cargo/config.toml`:

```toml
# For each dependency, Cargo picks the newest version whose `rust-version`
# this workspace's 1.78 can build (spec
# docs/superpowers/specs/2026-10-09-phase-3-3b-index-and-publishing-design.md
# §9). Cargo 1.84 and later read this. Every CI job passes `--locked`, so
# cargo 1.78 on the MSRV job never resolves and never needs it.
[resolver]
incompatible-rust-versions = "fallback"
```

In `crates/nova-index/Cargo.toml`, append to `[dependencies]`:

```toml
# HTTP for the index and downloads (spec §9). ureq 3.3.0 and later need
# Rust 1.85; 3.2.1 needs 1.71.1. `rustls` is rustls on the `ring` already
# locked, with Mozilla's roots from webpki-roots, so no system TLS library
# is needed. Default features are off, so no transparent gzip decoding.
ureq = { version = "=3.2.1", default-features = false, features = ["rustls"] }
```

```bash
cd /d/Projects/nona/nova && P=/c/Users/SAKEER~1/AppData/Local/Temp/gcm/p34 && cargo check -p nova-index > $P/c8.txt 2>&1; tail -3 $P/c8.txt; git diff -- Cargo.lock | grep -E '^\+(name|version) = ' | paste - - > $P/added8.txt; cat $P/added8.txt
```

Expected:
- the check passes;
- `$P/added8.txt` lists the new packages. That is ureq, ureq-proto,
  rustls, rustls-pki-types, rustls-webpki, webpki-roots, http, httparse
  (if not already there), base64, log, percent-encoding, utf-8, zeroize
  and subtle, or close to that list.

- [ ] **Step 2: Check every new package against Rust 1.78**

For each `name` and `version` in `$P/added8.txt`, read crates.io's record,
with a neutral User-Agent:

```bash
cd /d/Projects/nona/nova && P=/c/Users/SAKEER~1/AppData/Local/Temp/gcm/p34 && while read -r n v; do n=${n#name = }; n=${n//\"/}; v=${v#version = }; v=${v//\"/}; curl -s -A "nova-build-check" "https://crates.io/api/v1/crates/$n/$v" | python -X utf8 -c "import json,sys; d=json.load(sys.stdin)['version']; print(d['crate'], d['num'], 'rust', d.get('rust_version'), 'edition', d.get('edition'))"; sleep 1; done < $P/added8.txt | tee $P/msrv8.txt
```

Expected:
- every `rust` is 1.78 or lower, or `None`;
- no `edition` is 2024;
- `zeroize` is 1.8.x and `ureq-proto` is 0.5.x.

Copy `$P/msrv8.txt` into the ledger. Mark the packages only Windows or
macOS builds: read `[target.'cfg(…)'.dependencies]` in each one's
`Cargo.toml` under `~/.cargo/registry/src/*/`. Then check the whole
workspace with the MSRV compiler on Windows here:

```bash
cd /d/Projects/nona/nova && RUSTUP_TOOLCHAIN=1.78.0 RUSTFLAGS="-D warnings" cargo check --locked --workspace 2>&1 | tail -3
```

Expected: `Finished`, no error. If a package fails, pin an older version
with `cargo update -p NAME --precise V`, ledger it, and repeat both steps.

- [ ] **Step 3: Write the failing tests**

`crates/nova-index/tests/support/mod.rs`:

```rust
//! A loopback HTTP/1.1 server for tests (plan decision 11). It binds
//! 127.0.0.1 on a free port, answers each request with what its handler
//! returns, and records every request. nova-cli's tests include this file
//! by path.

#![allow(dead_code)]

use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, Mutex};

/// A request the server received.
#[derive(Debug, Clone)]
pub struct Request {
    pub method: String,
    /// The path and query, as sent.
    pub path: String,
    /// Each header, its name in lower case.
    pub headers: Vec<(String, String)>,
    pub body: Vec<u8>,
}

impl Request {
    pub fn header(&self, name: &str) -> Option<&str> {
        let name = name.to_ascii_lowercase();
        self.headers
            .iter()
            .find(|(n, _)| *n == name)
            .map(|(_, v)| v.as_str())
    }
}

/// What the handler answers.
#[derive(Debug, Clone)]
pub struct Response {
    pub status: u16,
    pub headers: Vec<(String, String)>,
    pub body: Vec<u8>,
    /// Close the connection after this many bytes of the body, though its
    /// `Content-Length` promises them all.
    pub cut_after: Option<usize>,
}

impl Response {
    pub fn status(status: u16) -> Response {
        Response {
            status,
            headers: Vec::new(),
            body: Vec::new(),
            cut_after: None,
        }
    }

    pub fn ok(body: impl Into<Vec<u8>>) -> Response {
        Response::with_status(200, body)
    }

    pub fn with_status(status: u16, body: impl Into<Vec<u8>>) -> Response {
        Response {
            body: body.into(),
            ..Response::status(status)
        }
    }

    pub fn redirect(location: &str) -> Response {
        let mut response = Response::status(302);
        response
            .headers
            .push(("Location".to_string(), location.to_string()));
        response
    }
}

/// A running server. It stops when the test process ends.
pub struct Server {
    /// `http://127.0.0.1:<port>`, with no trailing `/`.
    pub url: String,
    requests: Arc<Mutex<Vec<Request>>>,
}

impl Server {
    pub fn start<F>(handler: F) -> Server
    where
        F: Fn(&Request) -> Response + Send + Sync + 'static,
    {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind a loopback port");
        let url = format!("http://127.0.0.1:{}", listener.local_addr().unwrap().port());
        let requests = Arc::new(Mutex::new(Vec::new()));
        let recorded = Arc::clone(&requests);
        let handler = Arc::new(handler);
        std::thread::spawn(move || {
            for stream in listener.incoming().flatten() {
                let recorded = Arc::clone(&recorded);
                let handler = Arc::clone(&handler);
                std::thread::spawn(move || serve(stream, &*handler, &recorded));
            }
        });
        Server { url, requests }
    }

    /// Every request so far, in order.
    pub fn requests(&self) -> Vec<Request> {
        self.requests.lock().unwrap().clone()
    }
}

/// Answer requests on one connection until the client closes it.
fn serve(
    stream: TcpStream,
    handler: &dyn Fn(&Request) -> Response,
    recorded: &Mutex<Vec<Request>>,
) {
    let Ok(read_half) = stream.try_clone() else {
        return;
    };
    let mut reader = BufReader::new(read_half);
    let mut stream = stream;
    loop {
        let mut line = String::new();
        if reader.read_line(&mut line).unwrap_or(0) == 0 {
            return;
        }
        let mut parts = line.split_whitespace();
        let method = parts.next().unwrap_or("").to_string();
        let path = parts.next().unwrap_or("").to_string();
        let mut headers = Vec::new();
        loop {
            let mut header = String::new();
            if reader.read_line(&mut header).unwrap_or(0) == 0 {
                return;
            }
            let header = header.trim_end();
            if header.is_empty() {
                break;
            }
            if let Some((name, value)) = header.split_once(':') {
                headers.push((name.trim().to_ascii_lowercase(), value.trim().to_string()));
            }
        }
        let length = headers
            .iter()
            .find(|(name, _)| name == "content-length")
            .and_then(|(_, value)| value.parse().ok())
            .unwrap_or(0);
        let mut body = vec![0; length];
        if reader.read_exact(&mut body).is_err() {
            return;
        }
        let request = Request {
            method,
            path,
            headers,
            body,
        };
        recorded.lock().unwrap().push(request.clone());
        let response = handler(&request);
        let mut head = format!(
            "HTTP/1.1 {} X\r\nContent-Length: {}\r\n",
            response.status,
            response.body.len()
        );
        for (name, value) in &response.headers {
            head.push_str(&format!("{name}: {value}\r\n"));
        }
        head.push_str("\r\n");
        if stream.write_all(head.as_bytes()).is_err() {
            return;
        }
        if let Some(cut) = response.cut_after {
            let _ = stream.write_all(&response.body[..cut]);
            let _ = stream.flush();
            return;
        }
        if stream.write_all(&response.body).is_err() {
            return;
        }
        let _ = stream.flush();
    }
}
```

`crates/nova-index/tests/http.rs`:

```rust
//! HTTP and downloads, against loopback servers (spec
//! `docs/superpowers/specs/2026-10-09-phase-3-3b-index-and-publishing-design.md`
//! §3.3, §5.2, §10.4).

mod support;

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use nova_index::{
    fetch_package, pack, sha256_hex, Config, Http, HttpReader, Index, Reader, View,
};
use nova_pm::{IndexView, LockedPackage};
use semver::Version;
use support::{Response, Server};

/// A fresh, empty directory with a fixed name, so each run replaces the last.
fn fresh(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("nova-index-http-{name}"));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn write(dir: &Path, path: &str, bytes: &[u8]) {
    let path = dir.join(path);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, bytes).unwrap();
}

/// Every file under `dir`; none when it does not exist.
fn files_under(dir: &Path) -> Vec<PathBuf> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut files = Vec::new();
    for entry in entries {
        let path = entry.unwrap().path();
        if path.is_dir() {
            files.extend(files_under(&path));
        } else {
            files.push(path);
        }
    }
    files
}

/// `geom` 0.1.0, packed from a source under `dir`.
fn geom_tarball(dir: &Path) -> Vec<u8> {
    let source = dir.join("source");
    write(
        &source,
        "nova.toml",
        b"[package]\nname = \"geom\"\nversion = \"0.1.0\"\nedition = \"2026\"\n",
    );
    write(&source, "src/lib.nova", b"pub fn area() -> Int {\n    9\n}\n");
    pack(&source, "geom", &Version::new(0, 1, 0)).unwrap().bytes
}

fn locked(checksum: &str) -> LockedPackage {
    LockedPackage {
        name: "geom".into(),
        version: Version::new(0, 1, 0),
        checksum: checksum.into(),
        dependencies: Vec::new(),
    }
}

fn config() -> Config {
    Config {
        dl: "dl/{name}-{version}.nova-pkg".into(),
        api: None,
    }
}

#[test]
fn a_file_is_fetched_over_loopback_http() {
    let server = Server::start(|request| match request.path.as_str() {
        "/a" => Response::ok("hello"),
        "/broken" => Response::status(500),
        _ => Response::status(404),
    });
    let http = Http::new();
    let url = |path: &str| format!("{}{path}", server.url);
    assert_eq!(http.get(&url("/a"), 100).unwrap(), Some(b"hello".to_vec()));
    assert_eq!(http.get(&url("/none"), 100).unwrap(), None);
    let error = http.get(&url("/broken"), 100).unwrap_err();
    assert!(error.contains("500"), "{error}");
}

#[test]
fn redirects_are_followed_only_where_http_is_allowed() {
    let server = Server::start(|request| match request.path.as_str() {
        "/moved" => Response::redirect("/a"),
        "/away" => Response::redirect("http://example.com/x"),
        "/loop" => Response::redirect("/loop"),
        "/a" => Response::ok("here"),
        _ => Response::status(404),
    });
    let http = Http::new();
    let url = |path: &str| format!("{}{path}", server.url);
    assert_eq!(
        http.get(&url("/moved"), 100).unwrap(),
        Some(b"here".to_vec())
    );
    let error = http.get(&url("/away"), 100).unwrap_err();
    assert!(error.contains("refused http://example.com/x"), "{error}");
    let error = http.get(&url("/loop"), 100).unwrap_err();
    assert!(error.contains("too many redirects"), "{error}");
    let loops = server
        .requests()
        .iter()
        .filter(|r| r.path == "/loop")
        .count();
    assert_eq!(loops, 6);
    // Refused before any connection is made.
    assert!(http.get("http://example.com/", 100).is_err());
}

#[test]
fn a_body_over_its_limit_is_refused() {
    let server = Server::start(|_| Response::ok(vec![b'x'; 100]));
    assert!(Http::new()
        .get(&format!("{}/big", server.url), 10)
        .is_err());
}

#[test]
fn requests_name_nova_and_carry_no_credentials() {
    let server = Server::start(|_| Response::ok("x"));
    Http::new()
        .get(&format!("{}/a", server.url), 10)
        .unwrap();
    let request = &server.requests()[0];
    let agent = request.header("user-agent").unwrap();
    assert!(agent.starts_with("nova/"), "{request:?}");
    assert!(request.header("authorization").is_none(), "{request:?}");
}

#[test]
fn an_http_index_is_read_through_a_view() {
    let line = r#"{"name":"geom","vers":"0.1.0","deps":[],"cksum":"00","v":1}"#;
    let server = Server::start(move |request| match request.path.as_str() {
        "/index/ge/om/geom" => Response::ok(format!("{line}\n")),
        "/index/config.json" => Response::ok(r#"{"dl":"dl/{name}-{version}.nova-pkg"}"#),
        _ => Response::status(404),
    });
    let mut reader = HttpReader::new(&format!("{}/index/", server.url));
    assert_eq!(reader.config().unwrap().dl, "dl/{name}-{version}.nova-pkg");
    let mut view = View::new(&mut reader);
    assert_eq!(view.versions("geom").unwrap().unwrap().len(), 1);
    assert_eq!(view.versions("json").unwrap(), None);
}

#[test]
fn a_tarball_is_downloaded_checked_and_unpacked() {
    let dir = fresh("download");
    let tarball = geom_tarball(&dir);
    let checksum = sha256_hex(&tarball);
    let served = tarball.clone();
    let server = Server::start(move |request| match request.path.as_str() {
        "/dl/geom-0.1.0.nova-pkg" => Response::ok(served.clone()),
        _ => Response::status(404),
    });
    let index = Index::new(&server.url).unwrap();
    let registry = dir.join("registry");
    let dest =
        fetch_package(&index, &config(), &Http::new(), &locked(&checksum), &registry).unwrap();
    assert!(dest.join("src/lib.nova").is_file());
    let idx = nova_pm::index_dir_name(&index.canonical);
    assert!(registry
        .join("cache")
        .join(&idx)
        .join("geom-0.1.0.nova-pkg")
        .is_file());
    // Once unpacked, nothing is requested again.
    let before = server.requests().len();
    fetch_package(&index, &config(), &Http::new(), &locked(&checksum), &registry).unwrap();
    assert_eq!(server.requests().len(), before);
}

#[test]
fn a_tarball_whose_checksum_is_wrong_is_refused_and_discarded() {
    let dir = fresh("checksum");
    let tarball = geom_tarball(&dir);
    let actual = sha256_hex(&tarball);
    let server = Server::start(move |_| Response::ok(tarball.clone()));
    let index = Index::new(&server.url).unwrap();
    let registry = dir.join("registry");
    let wrong = "0".repeat(64);
    let error =
        fetch_package(&index, &config(), &Http::new(), &locked(&wrong), &registry).unwrap_err();
    assert!(error.contains(&actual) && error.contains(&wrong), "{error}");
    assert!(files_under(&registry).is_empty(), "{:?}", files_under(&registry));
}

#[test]
fn a_cut_off_download_leaves_nothing_and_the_next_one_works() {
    // Review Focus 5.
    let dir = fresh("cut-off");
    let tarball = geom_tarball(&dir);
    let checksum = sha256_hex(&tarball);
    let calls = Arc::new(AtomicUsize::new(0));
    let counted = Arc::clone(&calls);
    let server = Server::start(move |_| {
        let mut response = Response::ok(tarball.clone());
        if counted.fetch_add(1, Ordering::SeqCst) == 0 {
            response.cut_after = Some(tarball.len() / 2);
        }
        response
    });
    let index = Index::new(&server.url).unwrap();
    let registry = dir.join("registry");
    let first = fetch_package(&index, &config(), &Http::new(), &locked(&checksum), &registry);
    assert!(first.is_err(), "{first:?}");
    assert!(files_under(&registry).is_empty(), "{:?}", files_under(&registry));
    let dest =
        fetch_package(&index, &config(), &Http::new(), &locked(&checksum), &registry).unwrap();
    assert!(dest.join("src/lib.nova").is_file());
    assert_eq!(calls.load(Ordering::SeqCst), 2);
}

#[test]
fn a_local_indexs_tarball_is_read_from_its_directory() {
    let dir = fresh("local");
    let tarball = geom_tarball(&dir);
    let checksum = sha256_hex(&tarball);
    let index_dir = dir.join("index");
    write(&index_dir, "dl/geom-0.1.0.nova-pkg", &tarball);
    let index = Index::new(index_dir.to_str().unwrap()).unwrap();
    let dest = fetch_package(
        &index,
        &config(),
        &Http::new(),
        &locked(&checksum),
        &dir.join("registry"),
    )
    .unwrap();
    assert!(dest.join("nova.toml").is_file());
}
```

```bash
cd /d/Projects/nona/nova && cargo test --locked -p nova-index --test http 2>&1 | grep -E "^error" | head -3
```

Expected: `error[E0432]: unresolved imports` naming `fetch_package`,
`Http` and `HttpReader`.

- [ ] **Step 4: Implement**

`crates/nova-index/src/http.rs`:

```rust
//! HTTP, for reading an index and downloading tarballs (spec §3.3, §5.2,
//! §8). Redirects are followed here rather than by ureq, so each target is
//! checked, and no request made here carries a token.

use std::time::Duration;

/// Redirects followed before giving up.
const MAX_REDIRECTS: usize = 5;

/// The largest index file read: 10 MiB.
pub const MAX_INDEX_FILE: u64 = 10 * 1024 * 1024;

/// An HTTP client for reads that need no credentials.
pub struct Http {
    agent: ureq::Agent,
}

impl Default for Http {
    fn default() -> Self {
        Http::new()
    }
}

impl Http {
    /// A status is a value, not an error; ureq follows no redirect; a
    /// request takes at most 60 seconds; the User-Agent names nova and its
    /// version only. ureq's default configuration reads the proxy from
    /// `HTTPS_PROXY`, `HTTP_PROXY` and `NO_PROXY` (spec §9).
    pub fn new() -> Http {
        let config = ureq::Agent::config_builder()
            .http_status_as_error(false)
            .max_redirects(0)
            .max_redirects_will_error(false)
            .timeout_global(Some(Duration::from_secs(60)))
            .user_agent(format!("nova/{}", env!("CARGO_PKG_VERSION")))
            .build();
        Http {
            agent: ureq::Agent::new_with_config(config),
        }
    }

    pub(crate) fn agent(&self) -> &ureq::Agent {
        &self.agent
    }

    /// GET `url`, reading at most `limit` bytes of its body. `Ok(None)` for
    /// a 404. Up to five redirects are followed, each checked by
    /// [`check_url`].
    pub fn get(&self, url: &str, limit: u64) -> Result<Option<Vec<u8>>, String> {
        let mut url = url.to_string();
        for _ in 0..=MAX_REDIRECTS {
            check_url(&url)?;
            let mut response = self
                .agent
                .get(&url)
                .call()
                .map_err(|e| format!("cannot reach {}: {e}", shown(&url)))?;
            let status = response.status().as_u16();
            if matches!(status, 301 | 302 | 303 | 307 | 308) {
                let location = response
                    .headers()
                    .get("location")
                    .and_then(|value| value.to_str().ok())
                    .ok_or_else(|| format!("{} redirected nowhere", shown(&url)))?
                    .to_string();
                url = resolve_location(&url, &location);
                continue;
            }
            return match status {
                200 => response
                    .body_mut()
                    .with_config()
                    .limit(limit)
                    .read_to_vec()
                    .map(Some)
                    .map_err(|e| format!("cannot read {}: {e}", shown(&url))),
                404 => Ok(None),
                _ => Err(format!("{} answered HTTP {status}", shown(&url))),
            };
        }
        Err(format!("too many redirects, the last to {}", shown(&url)))
    }
}

/// Whether nova may fetch `url` (spec §3.3): `https://`, or `http://` on
/// 127.0.0.1 or [::1] only.
pub fn check_url(url: &str) -> Result<(), String> {
    let lower = url.to_ascii_lowercase();
    if lower.starts_with("https://") {
        return Ok(());
    }
    if let Some(rest) = lower.strip_prefix("http://") {
        let authority = rest.split(['/', '?', '#']).next().unwrap_or("");
        let host = match authority.rfind(']') {
            Some(end) => &authority[..=end],
            None => authority.split(':').next().unwrap_or(""),
        };
        if host == "127.0.0.1" || host == "[::1]" {
            return Ok(());
        }
    }
    Err(format!(
        "refused {}: only https://, or http:// on 127.0.0.1 or [::1], is allowed",
        shown(url)
    ))
}

/// `url` without its query, for messages: a signed download URL's query
/// is long, and says nothing a reader needs.
pub(crate) fn shown(url: &str) -> &str {
    url.split('?').next().unwrap_or(url)
}

/// A redirect's `Location`, against the URL that gave it.
fn resolve_location(base: &str, location: &str) -> String {
    let lower = location.to_ascii_lowercase();
    if lower.starts_with("http://") || lower.starts_with("https://") {
        return location.to_string();
    }
    let (scheme, rest) = base.split_once("://").unwrap_or(("https", base));
    if let Some(authority_and_path) = location.strip_prefix("//") {
        return format!("{scheme}://{authority_and_path}");
    }
    let authority = rest.split('/').next().unwrap_or("");
    if location.starts_with('/') {
        return format!("{scheme}://{authority}{location}");
    }
    let path = &rest[authority.len()..];
    let dir = &path[..path.rfind('/').map_or(0, |i| i + 1)];
    let dir = if dir.is_empty() { "/" } else { dir };
    format!("{scheme}://{authority}{dir}{location}")
}
```

`crates/nova-index/src/download.rs`:

```rust
//! Downloading a package into the cache (spec §5.1, §5.2).

use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use nova_pm::LockedPackage;

use crate::cache::unpack;
use crate::http::Http;
use crate::line::Config;
use crate::location::{Index, Location};
use crate::pack::{sha256_hex, MAX_TARBALL};

/// Make `locked` available in `registry` (`$NOVA_HOME/registry`), and
/// return its unpacked directory, `src/<idx>/<name>-<version>` (spec
/// §5.1). Nothing is done when it is already unpacked. The tarball comes
/// from `cache/<idx>/` when one there has the locked checksum, and is
/// otherwise downloaded from where `config`'s `dl` says. A tarball is
/// written to the cache, and unpacked, only after its SHA-256 matches the
/// lock (spec §5.2).
pub fn fetch_package(
    index: &Index,
    config: &Config,
    http: &Http,
    locked: &LockedPackage,
    registry: &Path,
) -> Result<PathBuf, String> {
    let idx = nova_pm::index_dir_name(&index.canonical);
    let stem = format!("{}-{}", locked.name, locked.version);
    let dest = registry.join("src").join(&idx).join(&stem);
    if dest.join(nova_pm::MANIFEST).is_file() {
        return Ok(dest);
    }
    let cached = registry
        .join("cache")
        .join(&idx)
        .join(format!("{stem}.nova-pkg"));
    let bytes = match std::fs::read(&cached) {
        Ok(bytes) if sha256_hex(&bytes) == locked.checksum => bytes,
        _ => {
            let bytes = download(index, config, http, locked)?;
            let actual = sha256_hex(&bytes);
            if actual != locked.checksum {
                return Err(format!(
                    "the tarball of {} {} has SHA-256 {actual}, but nova.lock says {}; it was \
                     discarded",
                    locked.name, locked.version, locked.checksum
                ));
            }
            write_atomically(&cached, &bytes).map_err(|e| {
                format!(
                    "cannot write the package cache at {}: {e}; set NOVA_HOME to a writable \
                     directory",
                    cached.display()
                )
            })?;
            bytes
        }
    };
    unpack(
        &bytes,
        &locked.name,
        &locked.version,
        &locked.dependencies,
        &dest,
    )?;
    Ok(dest)
}

fn download(
    index: &Index,
    config: &Config,
    http: &Http,
    locked: &LockedPackage,
) -> Result<Vec<u8>, String> {
    let version = locked.version.to_string();
    match index.tarball(&config.dl, &locked.name, &version)? {
        Location::Url(url) => http.get(&url, MAX_TARBALL)?.ok_or_else(|| {
            format!(
                "the index has no tarball for {} {version} at {}",
                locked.name,
                crate::http::shown(&url)
            )
        }),
        Location::File(path) => {
            let size = std::fs::metadata(&path)
                .map_err(|e| format!("cannot read {}: {e}", path.display()))?
                .len();
            if size > MAX_TARBALL {
                return Err(format!(
                    "{} is {size} bytes; the limit is 10 MiB",
                    path.display()
                ));
            }
            std::fs::read(&path).map_err(|e| format!("cannot read {}: {e}", path.display()))
        }
    }
}

static TEMP_COUNTER: AtomicU64 = AtomicU64::new(0);

/// Write `bytes` to `path` through a temporary file beside it, unique to
/// this process, renamed into place. `path`'s directory is created.
pub(crate) fn write_atomically(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    if let Some(dir) = path.parent() {
        if !dir.as_os_str().is_empty() {
            std::fs::create_dir_all(dir)?;
        }
    }
    let name = path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    let (mut file, temp) = loop {
        let n = TEMP_COUNTER.fetch_add(1, Ordering::Relaxed);
        let temp = path.with_file_name(format!(".{name}.{}-{n}.tmp", std::process::id()));
        match std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temp)
        {
            Ok(file) => break (file, temp),
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(error) => return Err(error),
        }
    };
    let written = file.write_all(bytes).and_then(|()| file.flush());
    drop(file);
    let result = written.and_then(|()| std::fs::rename(&temp, path));
    if result.is_err() {
        let _ = std::fs::remove_file(&temp);
    }
    result
}
```

In `crates/nova-index/src/read.rs`, add after `LocalReader`'s impl:

```rust
/// An index read over HTTP from `base`, its canonical URL.
pub struct HttpReader {
    pub base: String,
    pub http: Http,
}

impl HttpReader {
    pub fn new(base: &str) -> HttpReader {
        HttpReader {
            base: base.to_string(),
            http: Http::new(),
        }
    }
}

impl Reader for HttpReader {
    fn file(&mut self, path: &str) -> Result<Option<String>, String> {
        let url = format!("{}{path}", self.base);
        match self.http.get(&url, MAX_INDEX_FILE)? {
            None => Ok(None),
            Some(bytes) => String::from_utf8(bytes)
                .map(Some)
                .map_err(|_| format!("{url} is not UTF-8")),
        }
    }
}

/// The reader for `index`'s own files: its directory, or HTTP.
pub fn reader_for(index: &Index) -> Box<dyn Reader> {
    match &index.source {
        Source::Local(dir) => Box::new(LocalReader { dir: dir.clone() }),
        Source::Http(base) => Box::new(HttpReader::new(base)),
    }
}
```

Its `use crate::location::index_path;` line becomes the two lines:

```rust
use crate::http::{Http, MAX_INDEX_FILE};
use crate::location::{index_path, Index, Source};
```

If ureq's `user_agent` does not take a `String`, pass
`format!(…).as_str()` instead, and ledger it.

In `crates/nova-index/src/lib.rs`, add `mod download;` and `mod http;`, and:

```rust
pub use download::fetch_package;
pub use http::{check_url, Http, MAX_INDEX_FILE};
```

`pub use read::…;` gains `reader_for` and `HttpReader`.

- [ ] **Step 5: Run the tests to verify they pass**

```bash
cd /d/Projects/nona/nova && P=/c/Users/SAKEER~1/AppData/Local/Temp/gcm/p34 && cargo test --locked -p nova-index > $P/t8.txt 2>&1; echo "exit=$?"; python -X utf8 $P/count.py $P/t8.txt; grep -E "FAILED|panicked|^warning|^error" $P/t8.txt | head
```

Expected:
- `exit=0`, 0 failed, no warnings;
- the 9 tests of `http.rs` included.

A firewall prompt for the test binary on Windows means a test bound
somewhere other than 127.0.0.1. Stop and find it.

- [ ] **Step 6: Commit**

Write `$P/msg-8.txt`:

```
nova-index: HTTP and downloads

ureq 3.2.1 with rustls on the ring already locked (spec
docs/superpowers/specs/2026-10-09-phase-3-3b-index-and-publishing-design.md
§9). ureq 3.3.0 and later need Rust 1.85. .cargo/config.toml makes Cargo
prefer versions that build on the workspace's 1.78. Every package
Cargo.lock gains was checked against crates.io's rust_version and
edition, and the workspace with Rust 1.78 on Windows; the list is in the
plan's ledger.

`Http` follows redirects itself, up to five, only to https:// or to
http:// on 127.0.0.1 or [::1], and sends no credentials. Its User-Agent
is nova and its version. `HttpReader` reads an index over HTTP.

`fetch_package` downloads a locked package's tarball, from a URL or a
local index's directory, refuses it unless its SHA-256 is the lock's,
then writes it to cache/<idx>/ and unpacks it to src/<idx>/. A download
cut off part-way leaves nothing behind.

The tests run against a loopback server written for them, which
nova-cli's tests share.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
```

```bash
cd /d/Projects/nona/nova && P=/c/Users/SAKEER~1/AppData/Local/Temp/gcm/p34 && cargo fmt --all && git add .cargo crates/nova-index Cargo.lock && git commit -q -F $P/msg-8.txt && git log -1 --format=%s
```

Expected: `nova-index: HTTP and downloads`.

The task's test command: `cargo test --locked -p nova-index`.

---

### Task 9: The sync step

Spec §4.4, §4.6, §5.3, §10.1, §10.5.

**Files:**
- Create: `crates/nova-index/src/sync.rs`, `crates/nova-index/tests/sync.rs`
- Modify: `crates/nova-index/src/lib.rs`

**Interfaces:**
- Consumes:
  - Task 4's `requirements`;
  - Task 3's `resolve`, `Unlock` and `ResolveError`;
  - Task 2's `Lock`, `parse_lock` and `LOCKFILE`;
  - Task 8's `fetch_package`, `Http` and `write_atomically`;
  - Task 6's `View` and `Reader`.
- Produces:
  - `SyncRequest<'a>`, with these fields:
    - `root: &'a Path`;
    - `manifest: Option<&'a str>`;
    - `dev: bool`;
    - `unlock: Unlock`;
    - `write: bool`;
    - `index: &'a Index`;
    - `reader: &'a mut dyn Reader`;
    - `registry: Option<&'a Path>`. It is `None` when `$NOVA_HOME` cannot
      be found, which is an error only when something must be
      downloaded;
  - `Synced { lock: Option<Lock>, before: Option<Lock>, unpacked: Vec<String>, notes: Vec<String> }`.
    Each entry of `unpacked` is `"<name> <version>"`;
  - `SyncError::{Diagnostics(Vec<Diagnostic>), Other(String)}`, deriving
    `Debug`;
  - `sync(SyncRequest<'_>, &mut FileDb) -> Result<Synced, SyncError>`;
  - `write_lock(&Path, &Lock) -> Result<(), String>`.

The sync's rules (spec §4.4, §4.6):
1. **Requirements:** the root's and its path packages'. Any error stops
   the sync. Warnings are left for the command's own graph to show.
2. **The lock:** `nova.lock` is read. An unreadable one is M0016, except
   under `Unlock::All`, which ignores it.
3. **A lock from another index** pins nothing.
4. **No resolution** under `Unlock::Nothing` when the lock is usable and
   already answers every requirement: each requirement is locked at a
   version that meets it, and every package reached has its own
   dependencies locked. The new lock is then the old one pruned to what
   the requirements reach.
5. **Otherwise it resolves** through `View` over the request's reader.
6. **Every locked package not unpacked is downloaded.** `config.json` is
   read once, only when something is missing.
7. **The lock is written last,** and only when `write` is set and its
   contents changed or something was unpacked.
8. **With no requirement and no earlier lock,** there is no lock. With
   no requirement and an earlier one, the lock is kept with no packages.

- [ ] **Step 1: Write the failing tests**

`crates/nova-index/tests/sync.rs`:

```rust
//! The sync step (spec
//! `docs/superpowers/specs/2026-10-09-phase-3-3b-index-and-publishing-design.md`
//! §4.4, §4.6, §5.3), against a local index.

use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use nova_diagnostics::FileDb;
use nova_index::{
    index_path, pack, reader_for, sync, Index, Line, LineDep, SyncError, SyncRequest, Synced,
};
use nova_pm::Unlock;
use semver::{Version, VersionReq};

/// A fresh, empty directory with a fixed name, so each run replaces the last.
fn fresh(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("nova-index-sync-{name}"));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn write(path: &Path, bytes: &[u8]) {
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, bytes).unwrap();
}

fn manifest(name: &str, version: &str, extra: &str) -> String {
    format!("[package]\nname = \"{name}\"\nversion = \"{version}\"\nedition = \"2026\"\n{extra}")
}

/// A local index in `dir/index`, and a registry in `dir/home/registry`.
struct Fixture {
    dir: PathBuf,
    index_dir: PathBuf,
    registry: PathBuf,
}

impl Fixture {
    fn new(name: &str) -> Fixture {
        let dir = fresh(name);
        let index_dir = dir.join("index");
        write(
            &index_dir.join("config.json"),
            br#"{"dl":"dl/{name}-{version}.nova-pkg"}"#,
        );
        Fixture {
            registry: dir.join("home").join("registry"),
            index_dir,
            dir,
        }
    }

    /// Publish the library `name` `version` with `deps` by hand: its
    /// tarball where `dl` says, and its line appended.
    fn publish(&self, name: &str, version: &str, deps: &[(&str, &str)]) {
        let source = self.dir.join("sources").join(format!("{name}-{version}"));
        let mut extra = String::new();
        if !deps.is_empty() {
            extra.push_str("\n[dependencies]\n");
            for (dep, req) in deps {
                extra.push_str(&format!("{dep} = \"{req}\"\n"));
            }
        }
        write(
            &source.join("nova.toml"),
            manifest(name, version, &extra).as_bytes(),
        );
        write(
            &source.join("src").join("lib.nova"),
            b"pub fn one() -> Int {\n    1\n}\n",
        );
        let packed = pack(&source, name, &Version::parse(version).unwrap()).unwrap();
        write(
            &self
                .index_dir
                .join("dl")
                .join(format!("{name}-{version}.nova-pkg")),
            &packed.bytes,
        );
        let line = Line {
            name: name.into(),
            vers: version.into(),
            deps: deps
                .iter()
                .map(|(dep, req)| LineDep {
                    name: dep.to_string(),
                    req: VersionReq::parse(req).unwrap().to_string(),
                })
                .collect(),
            cksum: packed.checksum,
            v: 1,
        };
        let file = self.index_dir.join(index_path(name));
        let mut text = std::fs::read_to_string(&file).unwrap_or_default();
        text.push_str(&line.to_json());
        text.push('\n');
        write(&file, text.as_bytes());
    }

    fn index(&self) -> Index {
        Index::new(self.index_dir.to_str().unwrap()).unwrap()
    }

    /// The program `app` in `dir/app`, with `extra` appended to its manifest.
    fn app(&self, extra: &str) -> PathBuf {
        let app = self.dir.join("app");
        write(
            &app.join("nova.toml"),
            manifest("app", "0.1.0", extra).as_bytes(),
        );
        write(&app.join("src").join("main.nova"), b"fn main() {}\n");
        app
    }

    fn sync_with(
        &self,
        root: &Path,
        manifest: Option<&str>,
        dev: bool,
        unlock: Unlock,
        write: bool,
    ) -> Result<Synced, SyncError> {
        let index = self.index();
        let mut reader = reader_for(&index);
        let mut db = FileDb::new();
        sync(
            SyncRequest {
                root,
                manifest,
                dev,
                unlock,
                write,
                index: &index,
                reader: reader.as_mut(),
                registry: Some(&self.registry),
            },
            &mut db,
        )
    }

    fn sync(&self, root: &Path, unlock: Unlock) -> Result<Synced, SyncError> {
        self.sync_with(root, None, true, unlock, true)
    }

    fn unpacked(&self, name: &str, version: &str) -> PathBuf {
        let idx = nova_pm::index_dir_name(&self.index().canonical);
        self.registry
            .join("src")
            .join(idx)
            .join(format!("{name}-{version}"))
    }
}

/// `name version` for each locked package.
fn locked(synced: &Synced) -> Vec<String> {
    synced
        .lock
        .as_ref()
        .map(|lock| {
            lock.packages
                .iter()
                .map(|p| format!("{} {}", p.name, p.version))
                .collect()
        })
        .unwrap_or_default()
}

fn codes(error: SyncError) -> Vec<String> {
    match error {
        SyncError::Diagnostics(list) => list.into_iter().map(|d| d.code).collect(),
        SyncError::Other(message) => panic!("not a diagnostic: {message}"),
    }
}

#[test]
fn a_first_sync_resolves_downloads_and_writes_the_lock() {
    let f = Fixture::new("first");
    f.publish("json", "1.0.0", &[]);
    f.publish("json", "1.2.0", &[]);
    let app = f.app("\n[dependencies]\njson = \"1\"\n");
    let synced = f.sync(&app, Unlock::Nothing).unwrap();
    assert_eq!(locked(&synced), ["json 1.2.0"]);
    assert_eq!(synced.unpacked, ["json 1.2.0"]);
    assert!(f.unpacked("json", "1.2.0").join("src/lib.nova").is_file());
    let text = std::fs::read_to_string(app.join("nova.lock")).unwrap();
    assert!(text.contains("name = \"json\"\nversion = \"1.2.0\""), "{text}");
    assert!(!text.contains('\r'));
    assert!(text.contains(&format!("index = \"{}\"", f.index().canonical)));
}

#[test]
fn a_complete_lock_needs_no_index() {
    let f = Fixture::new("offline");
    f.publish("json", "1.0.0", &[]);
    let app = f.app("\n[dependencies]\njson = \"1\"\n");
    f.sync(&app, Unlock::Nothing).unwrap();
    let before = std::fs::read_to_string(app.join("nova.lock")).unwrap();
    std::fs::rename(&f.index_dir, f.dir.join("gone")).unwrap();
    let synced = f.sync(&app, Unlock::Nothing).unwrap();
    assert!(synced.unpacked.is_empty());
    assert_eq!(std::fs::read_to_string(app.join("nova.lock")).unwrap(), before);
}

#[test]
fn a_new_entry_keeps_the_other_locked_versions() {
    let f = Fixture::new("keep");
    f.publish("json", "1.0.0", &[]);
    let app = f.app("\n[dependencies]\njson = \"1\"\n");
    f.sync(&app, Unlock::Nothing).unwrap();
    f.publish("json", "1.1.0", &[]);
    f.publish("http", "0.3.0", &[]);
    f.app("\n[dependencies]\njson = \"1\"\nhttp = \"0.3\"\n");
    let synced = f.sync(&app, Unlock::Nothing).unwrap();
    assert_eq!(locked(&synced), ["http 0.3.0", "json 1.0.0"]);
}

#[test]
fn update_moves_every_version_and_update_one_only_that() {
    let f = Fixture::new("update");
    f.publish("json", "1.0.0", &[]);
    f.publish("http", "0.3.0", &[]);
    let app = f.app("\n[dependencies]\njson = \"1\"\nhttp = \"0.3\"\n");
    f.sync(&app, Unlock::Nothing).unwrap();
    f.publish("json", "1.1.0", &[]);
    f.publish("http", "0.3.1", &[]);
    let synced = f.sync(&app, Unlock::One("http".into())).unwrap();
    assert_eq!(locked(&synced), ["http 0.3.1", "json 1.0.0"]);
    assert_eq!(synced.before.unwrap().packages.len(), 2);
    let synced = f.sync(&app, Unlock::All).unwrap();
    assert_eq!(locked(&synced), ["http 0.3.1", "json 1.1.0"]);
}

#[test]
fn the_lock_is_pruned_when_an_entry_goes_without_reading_the_index() {
    let f = Fixture::new("prune");
    f.publish("json", "1.0.0", &[]);
    f.publish("http", "0.3.0", &[]);
    let app = f.app("\n[dependencies]\njson = \"1\"\nhttp = \"0.3\"\n");
    f.sync(&app, Unlock::Nothing).unwrap();
    f.app("\n[dependencies]\njson = \"1\"\n");
    std::fs::rename(&f.index_dir, f.dir.join("gone")).unwrap();
    let synced = f.sync(&app, Unlock::Nothing).unwrap();
    assert_eq!(locked(&synced), ["json 1.0.0"]);
    let text = std::fs::read_to_string(app.join("nova.lock")).unwrap();
    assert!(!text.contains("http"), "{text}");
}

#[test]
fn no_registry_dependency_makes_no_lock_and_an_emptied_one_is_kept() {
    let f = Fixture::new("no-lock");
    let app = f.app("");
    let synced = f.sync(&app, Unlock::Nothing).unwrap();
    assert!(synced.lock.is_none());
    assert!(!app.join("nova.lock").exists());
    f.publish("json", "1.0.0", &[]);
    f.app("\n[dependencies]\njson = \"1\"\n");
    f.sync(&app, Unlock::Nothing).unwrap();
    f.app("");
    let synced = f.sync(&app, Unlock::Nothing).unwrap();
    assert!(synced.lock.unwrap().packages.is_empty());
    let text = std::fs::read_to_string(app.join("nova.lock")).unwrap();
    assert!(!text.contains("[[package]]"), "{text}");
}

#[test]
fn a_conflict_is_m0015_and_writes_nothing() {
    let f = Fixture::new("conflict");
    f.publish("json", "1.0.0", &[]);
    let app = f.app("\n[dependencies]\njson = \"9\"\n");
    assert_eq!(codes(f.sync(&app, Unlock::Nothing).unwrap_err()), ["M0015"]);
    assert!(!app.join("nova.lock").exists());
}

#[test]
fn an_unknown_package_is_m0014() {
    let f = Fixture::new("unknown");
    let app = f.app("\n[dependencies]\nnope = \"1\"\n");
    assert_eq!(codes(f.sync(&app, Unlock::Nothing).unwrap_err()), ["M0014"]);
}

#[test]
fn a_failed_download_writes_no_lock() {
    // The lock is written last (spec §4.6): a lock never names a package
    // that is not unpacked.
    let f = Fixture::new("failed-download");
    f.publish("json", "1.0.0", &[]);
    std::fs::remove_file(f.index_dir.join("dl").join("json-1.0.0.nova-pkg")).unwrap();
    let app = f.app("\n[dependencies]\njson = \"1\"\n");
    assert!(matches!(
        f.sync(&app, Unlock::Nothing),
        Err(SyncError::Other(_))
    ));
    assert!(!app.join("nova.lock").exists());
}

#[test]
fn a_proposed_manifest_is_synced_without_writing_the_lock() {
    let f = Fixture::new("proposed");
    f.publish("json", "1.0.0", &[]);
    let app = f.app("");
    let proposed = manifest("app", "0.1.0", "\n[dependencies]\njson = \"1\"\n");
    let synced = f
        .sync_with(&app, Some(&proposed), true, Unlock::Nothing, false)
        .unwrap();
    assert_eq!(locked(&synced), ["json 1.0.0"]);
    assert!(!app.join("nova.lock").exists());
    assert!(f.unpacked("json", "1.0.0").is_dir());
}

#[test]
fn the_lock_is_rewritten_after_unpacking_even_unchanged() {
    let f = Fixture::new("rewrite");
    f.publish("json", "1.0.0", &[]);
    let app = f.app("\n[dependencies]\njson = \"1\"\n");
    f.sync(&app, Unlock::Nothing).unwrap();
    let lock = app.join("nova.lock");
    let old = SystemTime::now() - Duration::from_secs(3600);
    std::fs::File::options()
        .write(true)
        .open(&lock)
        .unwrap()
        .set_modified(old)
        .unwrap();
    std::fs::remove_dir_all(f.unpacked("json", "1.0.0")).unwrap();
    let synced = f.sync(&app, Unlock::Nothing).unwrap();
    assert_eq!(synced.unpacked, ["json 1.0.0"]);
    let modified = std::fs::metadata(&lock).unwrap().modified().unwrap();
    assert!(modified > old + Duration::from_secs(60));
}

#[test]
fn an_unreadable_lock_is_m0016_except_under_update() {
    let f = Fixture::new("unreadable");
    f.publish("json", "1.0.0", &[]);
    let app = f.app("\n[dependencies]\njson = \"1\"\n");
    std::fs::write(app.join("nova.lock"), "not a lock [").unwrap();
    assert_eq!(codes(f.sync(&app, Unlock::Nothing).unwrap_err()), ["M0016"]);
    let synced = f.sync(&app, Unlock::All).unwrap();
    assert_eq!(locked(&synced), ["json 1.0.0"]);
}

#[test]
fn a_lock_from_another_index_pins_nothing() {
    let f = Fixture::new("other-index");
    f.publish("json", "1.0.0", &[]);
    f.publish("json", "1.1.0", &[]);
    let app = f.app("\n[dependencies]\njson = \"1\"\n");
    std::fs::write(
        app.join("nova.lock"),
        "version = 1\nindex = \"https://elsewhere.test/\"\n\n[[package]]\nname = \"json\"\n\
         version = \"1.0.0\"\nchecksum = \"00\"\ndependencies = []\n",
    )
    .unwrap();
    let synced = f.sync(&app, Unlock::Nothing).unwrap();
    assert_eq!(locked(&synced), ["json 1.1.0"]);
    assert_eq!(synced.lock.unwrap().index, f.index().canonical);
}

#[test]
fn a_path_packages_registry_entries_are_resolved_and_dev_ones_only_with_dev() {
    let f = Fixture::new("path-and-dev");
    f.publish("json", "1.0.0", &[]);
    f.publish("kit", "0.1.0", &[]);
    let util = f.dir.join("util");
    write(
        &util.join("nova.toml"),
        manifest("util", "0.1.0", "\n[dependencies]\njson = \"1\"\n").as_bytes(),
    );
    write(&util.join("src").join("lib.nova"), b"");
    let app = f.app(
        "\n[dependencies]\nutil = { path = \"../util\" }\n\n[dev-dependencies]\nkit = \"0.1\"\n",
    );
    let synced = f.sync(&app, Unlock::Nothing).unwrap();
    assert_eq!(locked(&synced), ["json 1.0.0", "kit 0.1.0"]);
    let synced = f
        .sync_with(&app, None, false, Unlock::All, false)
        .unwrap();
    assert_eq!(locked(&synced), ["json 1.0.0"]);
}

#[test]
fn a_registry_packages_own_dependencies_are_locked_and_downloaded() {
    let f = Fixture::new("transitive");
    f.publish("json", "1.0.0", &[]);
    f.publish("geom", "0.2.0", &[("json", "1")]);
    let app = f.app("\n[dependencies]\ngeom = \"0.2\"\n");
    let synced = f.sync(&app, Unlock::Nothing).unwrap();
    assert_eq!(locked(&synced), ["geom 0.2.0", "json 1.0.0"]);
    assert_eq!(synced.lock.unwrap().packages[0].dependencies, ["json"]);
    assert!(f.unpacked("json", "1.0.0").is_dir());
}

#[test]
fn notes_about_bad_index_lines_are_returned() {
    let f = Fixture::new("notes");
    f.publish("json", "1.0.0", &[]);
    let file = f.index_dir.join(index_path("json"));
    let mut text = std::fs::read_to_string(&file).unwrap();
    text.push_str("not json\n");
    std::fs::write(&file, text).unwrap();
    let app = f.app("\n[dependencies]\njson = \"1\"\n");
    let synced = f.sync(&app, Unlock::Nothing).unwrap();
    assert_eq!(synced.notes.len(), 1, "{:?}", synced.notes);
    assert!(synced.notes[0].starts_with("js/on/json:2:"), "{:?}", synced.notes);
}
```

```bash
cd /d/Projects/nona/nova && cargo test --locked -p nova-index --test sync 2>&1 | grep -E "^error" | head -3
```

Expected: `error[E0432]: unresolved imports` naming `sync`, `SyncError`,
`SyncRequest` and `Synced`.

- [ ] **Step 2: Implement**

`crates/nova-index/src/sync.rs`:

```rust
//! The sync step (spec §4.4, §4.6, §5.3): resolve when the lock needs it,
//! download what the cache lacks, and write `nova.lock` last, so a lock
//! never names a package that is not unpacked.

use std::collections::{BTreeMap, VecDeque};
use std::path::Path;

use nova_diagnostics::{Diagnostic, FileDb, Severity};
use nova_pm::{parse_lock, Lock, LockedPackage, Requirement, ResolveError, Unlock, LOCKFILE};

use crate::download::{fetch_package, write_atomically};
use crate::http::Http;
use crate::location::Index;
use crate::read::{Reader, View};

/// What to sync.
pub struct SyncRequest<'a> {
    /// The root package's directory, empty for the current directory.
    pub root: &'a Path,
    /// The root's `nova.toml` when it is not the file on disk: `nova add`
    /// syncs the manifest it would write.
    pub manifest: Option<&'a str>,
    /// Whether the root's dev-dependencies are resolved. Publishing's
    /// verification resolves none.
    pub dev: bool,
    pub unlock: Unlock,
    /// Whether to write `nova.lock`. `nova add` writes it itself, with
    /// `nova.toml`, once both pass.
    pub write: bool,
    pub index: &'a Index,
    /// How index files are read: for a GitHub index while publishing,
    /// through the API; otherwise [`crate::reader_for`]'s.
    pub reader: &'a mut dyn Reader,
    /// `$NOVA_HOME/registry`. `None` when `$NOVA_HOME` cannot be found,
    /// which is an error only when something must be downloaded (spec
    /// §5.1).
    pub registry: Option<&'a Path>,
}

/// What a sync did.
#[derive(Debug)]
pub struct Synced {
    /// The lock as it now stands. `None` when the project has no registry
    /// dependency and never had a lock.
    pub lock: Option<Lock>,
    /// The lock before the sync.
    pub before: Option<Lock>,
    /// Each package downloaded and unpacked, as `<name> <version>`.
    pub unpacked: Vec<String>,
    /// Notes about index lines that do not parse (spec §3.1).
    pub notes: Vec<String>,
}

/// Why a sync stopped.
#[derive(Debug)]
pub enum SyncError {
    /// Manifest, graph or resolution diagnostics, to render with the
    /// `FileDb` the sync was given.
    Diagnostics(Vec<Diagnostic>),
    /// An index that cannot be reached, a cache that cannot be written.
    Other(String),
}

/// Sync the package at `request.root` (spec §5.3).
pub fn sync(request: SyncRequest<'_>, db: &mut FileDb) -> Result<Synced, SyncError> {
    let SyncRequest {
        root,
        manifest,
        dev,
        unlock,
        write,
        index,
        reader,
        registry,
    } = request;
    let (requirements, diagnostics) = nova_pm::requirements(root, manifest, dev, db);
    if diagnostics.iter().any(|d| d.severity == Severity::Error) {
        return Err(SyncError::Diagnostics(diagnostics));
    }
    let before = match read_lock(root, db) {
        Ok(lock) => lock,
        // `nova update` ignores the lock, so it replaces an unreadable one.
        Err(_) if unlock == Unlock::All => None,
        Err(diagnostic) => return Err(SyncError::Diagnostics(vec![diagnostic])),
    };
    if requirements.is_empty() && before.is_none() {
        return Ok(Synced {
            lock: None,
            before,
            unpacked: Vec::new(),
            notes: Vec::new(),
        });
    }
    // A lock from another index pins nothing (spec §4.4).
    let usable = before
        .as_ref()
        .filter(|lock| lock.index == index.canonical);
    let mut notes = Vec::new();
    let packages = match usable {
        _ if requirements.is_empty() => Vec::new(),
        Some(lock) if unlock == Unlock::Nothing && answers(&requirements, lock) => {
            reached(&requirements, lock)
        }
        _ => {
            let mut view = View::new(&mut *reader);
            let result = nova_pm::resolve(&requirements, usable, &unlock, &mut view);
            notes = std::mem::take(&mut view.notes);
            result.map_err(|error| match error {
                ResolveError::Diagnostic(diagnostic) => SyncError::Diagnostics(vec![diagnostic]),
                ResolveError::Index(why) => SyncError::Other(unreachable(index, &why)),
            })?
        }
    };
    let lock = Lock {
        index: index.canonical.clone(),
        packages,
    };
    let idx = nova_pm::index_dir_name(&index.canonical);
    let missing: Vec<&LockedPackage> = lock
        .packages
        .iter()
        .filter(|p| {
            !registry.is_some_and(|registry| {
                registry
                    .join("src")
                    .join(&idx)
                    .join(format!("{}-{}", p.name, p.version))
                    .join(nova_pm::MANIFEST)
                    .is_file()
            })
        })
        .collect();
    let mut unpacked = Vec::new();
    if !missing.is_empty() {
        // $NOVA_HOME is needed only when something must be downloaded.
        let registry = registry.ok_or_else(|| {
            SyncError::Other(
                "cannot place the package cache: set NOVA_HOME to a writable directory".into(),
            )
        })?;
        let config = reader
            .config()
            .map_err(|why| SyncError::Other(unreachable(index, &why)))?;
        let http = Http::new();
        for package in missing {
            fetch_package(index, &config, &http, package, registry).map_err(SyncError::Other)?;
            unpacked.push(format!("{} {}", package.name, package.version));
        }
    }
    if write && (before.as_ref() != Some(&lock) || !unpacked.is_empty()) {
        write_lock(root, &lock).map_err(SyncError::Other)?;
    }
    Ok(Synced {
        lock: Some(lock),
        before,
        unpacked,
        notes,
    })
}

fn unreachable(index: &Index, why: &str) -> String {
    format!("cannot reach the index at {}: {why}", index.canonical)
}

/// The root's `nova.lock`: `None` when there is none, M0016 when it cannot
/// be read.
fn read_lock(root: &Path, db: &mut FileDb) -> Result<Option<Lock>, Diagnostic> {
    let path = root.join(LOCKFILE);
    match std::fs::read_to_string(&path) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(
            Diagnostic::error("M0016", format!("nova.lock cannot be read: {error}"))
                .with_note("delete nova.lock, or run `nova update`"),
        ),
        Ok(text) => {
            let file = db.add(path.display().to_string(), text.as_str());
            parse_lock(&text, file).map(Some)
        }
    }
}

/// Whether `lock` already answers `requirements` (spec §4.4): each one is
/// locked at a version that meets it, and every package reached has its
/// own dependencies locked.
fn answers(requirements: &[Requirement], lock: &Lock) -> bool {
    requirements.iter().all(|requirement| {
        lock.find(&requirement.name)
            .is_some_and(|locked| requirement.req.matches(&locked.version))
    }) && reached(requirements, lock).iter().all(|package| {
        package
            .dependencies
            .iter()
            .all(|name| lock.find(name).is_some())
    })
}

/// The locked packages `requirements` reach, sorted by name: the lock,
/// pruned (spec §4.6).
fn reached(requirements: &[Requirement], lock: &Lock) -> Vec<LockedPackage> {
    let mut found: BTreeMap<String, LockedPackage> = BTreeMap::new();
    let mut queue: VecDeque<String> = requirements.iter().map(|r| r.name.clone()).collect();
    while let Some(name) = queue.pop_front() {
        if found.contains_key(&name) {
            continue;
        }
        if let Some(package) = lock.find(&name) {
            queue.extend(package.dependencies.iter().cloned());
            found.insert(name, package.clone());
        }
    }
    found.into_values().collect()
}

/// Write `lock` as `root`'s `nova.lock`, through a temporary file renamed
/// into place.
pub fn write_lock(root: &Path, lock: &Lock) -> Result<(), String> {
    let path = root.join(LOCKFILE);
    write_atomically(&path, lock.to_text().as_bytes())
        .map_err(|e| format!("cannot write {}: {e}", path.display()))
}
```

In `crates/nova-index/src/lib.rs`, add `mod sync;` and:

```rust
pub use sync::{sync, write_lock, SyncError, SyncRequest, Synced};
```

- [ ] **Step 3: Run the tests to verify they pass**

```bash
cd /d/Projects/nona/nova && P=/c/Users/SAKEER~1/AppData/Local/Temp/gcm/p34 && cargo test --locked -p nova-index > $P/t9.txt 2>&1; echo "exit=$?"; python -X utf8 $P/count.py $P/t9.txt; grep -E "FAILED|panicked|^warning|^error" $P/t9.txt | head
```

Expected: `exit=0`, 0 failed, no warnings, the 16 new tests included.

- [ ] **Step 4: Commit**

Write `$P/msg-9.txt`:

```
nova-index: the sync step

`sync` brings a project's nova.lock and the cache up to date (spec
docs/superpowers/specs/2026-10-09-phase-3-3b-index-and-publishing-design.md
§5.3):
- it reads the requirements of the root and its path packages, and any
  manifest error stops it;
- it resolves only when the lock does not already answer them, when the
  lock is from another index, or under `nova update`, which also
  replaces an unreadable lock; otherwise the lock is pruned to what is
  reached, with no network at all;
- it downloads each locked package not yet unpacked;
- it writes nova.lock last, when its contents changed or anything was
  unpacked, so a lock never names a package that is missing and an
  editor watching the lock re-checks.

`nova add` syncs the manifest it would write without writing the lock;
publishing's verification reads the index through its own reader.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
```

```bash
cd /d/Projects/nona/nova && P=/c/Users/SAKEER~1/AppData/Local/Temp/gcm/p34 && cargo fmt --all && git add crates/nova-index && git commit -q -F $P/msg-9.txt && git log -1 --format=%s
```

Expected: `nova-index: the sync step`.

The task's test command: `cargo test --locked -p nova-index`.

---

### Task 10: `nova package`, and `nova publish` to a local index

Spec §3.1, §6.5, §6.6, §6.8, §7 ("A local index"), §10.5.

**Files:**
- Create: `crates/nova-index/src/publish.rs`, `crates/nova-index/tests/publish.rs`
- Modify: `crates/nova-index/src/lib.rs`
- Create: `crates/nova-cli/src/cmd/package.rs`, `crates/nova-cli/tests/registry.rs`
- Modify: `crates/nova-cli/src/cmd/mod.rs`, `crates/nova-cli/src/main.rs`, `crates/nova-cli/src/project.rs` (`refuse_cached`), `crates/nova-cli/Cargo.toml`, `Cargo.lock`

**Interfaces:**
- Consumes:
  - Task 9's `sync`, `SyncRequest` and `SyncError`;
  - Task 7's `pack`, `unpack` and `Packed`;
  - Task 6's `Index`, `Line`, `LineDep`, `LINE_VERSION`, `Reader`, `reader_for` and `index_path`;
  - Task 5's `check_program_counted` and `Program::for_package_in`;
  - Task 4's `Offline`.
- Produces:
  - `nova_index::check_new(existing: &str, file: &str, line: &Line) -> Result<(), String>`;
  - `nova_index::publish_local(&Index, &Line, &[u8]) -> Result<(), String>`;
  - `nova-cli`'s `project::refuse_cached(&Path) -> Result<()>`;
  - the commands `nova package` and `nova publish`.
- The messages:
  - M0017: "a published package cannot have a path dependency; publish
    `util` first and depend on its version", labelled "a path dependency";
  - "`geom` has no src/lib.nova: only a library can be published";
  - "geom 0.1.0 is already in the index; a published version is never
    replaced";
  - "the index has `Geom`, which differs from `geom` only in case; a name
    keeps the spelling it was first published with";
  - "a local index's `dl` in config.json must be a path under the index,
    not a URL";
  - "`<dir>` is a downloaded package in nova's cache; it is read only";
  - "verification failed: 0 errors and 1 warning in the packed package;
    nothing was written".
- `nova package` prints:

  ```
  packed geom 0.1.0: <root>/target/package/geom-0.1.0.nova-pkg
    2 files, 489 bytes
    sha256 <hex>
  ```

  `nova publish` then prints "published geom 0.1.0 to <canonical>".

Verification comes before anything is written: `target/package/` gets the
tarball only after it passes.

- [ ] **Step 1: Write the failing tests**

`crates/nova-index/tests/publish.rs`:

```rust
//! Publishing to a local index (spec
//! `docs/superpowers/specs/2026-10-09-phase-3-3b-index-and-publishing-design.md`
//! §7).

use std::path::PathBuf;

use nova_index::{publish_local, Index, Line};

fn fresh(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("nova-index-publish-{name}"));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// A local index whose `dl` is `dl`.
fn index(name: &str, dl: &str) -> (PathBuf, Index) {
    let dir = fresh(name);
    std::fs::write(dir.join("config.json"), format!("{{\"dl\":\"{dl}\"}}")).unwrap();
    let index = Index::new(dir.to_str().unwrap()).unwrap();
    (dir, index)
}

fn line(name: &str, vers: &str) -> Line {
    Line {
        name: name.into(),
        vers: vers.into(),
        deps: Vec::new(),
        cksum: "c".repeat(64),
        v: 1,
    }
}

const DL: &str = "dl/{name}-{version}.nova-pkg";

#[test]
fn the_tarball_is_written_where_dl_says_then_the_line_appended() {
    let (dir, index) = index("writes", DL);
    publish_local(&index, &line("geom", "0.1.0"), b"one").unwrap();
    publish_local(&index, &line("geom", "0.2.0"), b"two").unwrap();
    assert_eq!(std::fs::read(dir.join("dl/geom-0.1.0.nova-pkg")).unwrap(), b"one");
    assert_eq!(std::fs::read(dir.join("dl/geom-0.2.0.nova-pkg")).unwrap(), b"two");
    assert_eq!(
        std::fs::read_to_string(dir.join("ge/om/geom")).unwrap(),
        format!(
            "{}\n{}\n",
            line("geom", "0.1.0").to_json(),
            line("geom", "0.2.0").to_json()
        )
    );
}

#[test]
fn a_version_already_there_is_refused_and_nothing_written() {
    let (dir, index) = index("again", DL);
    publish_local(&index, &line("geom", "0.1.0"), b"one").unwrap();
    let error = publish_local(&index, &line("geom", "0.1.0"), b"other").unwrap_err();
    assert!(error.contains("geom 0.1.0 is already in the index"), "{error}");
    assert_eq!(std::fs::read(dir.join("dl/geom-0.1.0.nova-pkg")).unwrap(), b"one");
}

#[test]
fn a_name_differing_only_in_case_is_refused() {
    let (_, index) = index("case", DL);
    publish_local(&index, &line("Geom", "0.1.0"), b"one").unwrap();
    let error = publish_local(&index, &line("geom", "0.2.0"), b"two").unwrap_err();
    assert!(
        error.contains("the index has `Geom`, which differs from `geom` only in case"),
        "{error}"
    );
}

#[test]
fn a_local_index_needs_a_dl_under_it() {
    let (_, index) = index("url", "https://example.test/{name}");
    let error = publish_local(&index, &line("geom", "0.1.0"), b"one").unwrap_err();
    assert!(error.contains("must be a path under the index"), "{error}");
}
```

`crates/nova-cli/tests/registry.rs`:

```rust
//! The package index end to end through `nova` (spec
//! `docs/superpowers/specs/2026-10-09-phase-3-3b-index-and-publishing-design.md`
//! §6, §10.5). Every command runs with `NOVA_HOME` and `NOVA_INDEX` set to
//! the test's own directories, so nothing reaches the internet.

use std::env::consts::EXE_SUFFIX;
use std::path::{Path, PathBuf};

use assert_cmd::Command;
use nova_index::{Index, Line};
use nova_pm::IndexView;

/// A fresh, empty directory with a fixed name, so each run replaces the last.
fn fresh(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("nova-registry-{name}"));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("create the test directory");
    dir
}

/// Write each `(path, text)` under `dir`.
fn write(dir: &Path, files: &[(&str, &str)]) {
    for (path, text) in files {
        let path = dir.join(path);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, text).unwrap();
    }
}

fn read(path: &Path) -> String {
    std::fs::read_to_string(path).unwrap_or_else(|e| panic!("reading {}: {e}", path.display()))
}

fn stdout(assert: &assert_cmd::assert::Assert) -> String {
    String::from_utf8_lossy(&assert.get_output().stdout).into_owned()
}

fn stderr(assert: &assert_cmd::assert::Assert) -> String {
    String::from_utf8_lossy(&assert.get_output().stderr).into_owned()
}

fn manifest(name: &str, version: &str, extra: &str) -> String {
    format!("[package]\nname = \"{name}\"\nversion = \"{version}\"\nedition = \"2026\"\n{extra}")
}

const AREA: &str = "pub fn area() -> Int {\n    9\n}\n";
const MAIN_AREA: &str = "import geom\n\nfn main() {\n    println(\"${area()}\")\n}\n";
/// A match whose second arm is unreachable: E0021, a warning.
const UNREACHABLE: &str = "pub fn pick(n: Int) -> Int {\n    match n { _ => 1, 0 => 2 }\n}\n";

/// A local index in `dir/index` whose `dl` is relative, and a `NOVA_HOME`
/// in `dir/home`.
struct Fixture {
    dir: PathBuf,
    index: PathBuf,
    home: PathBuf,
}

impl Fixture {
    fn new(name: &str) -> Fixture {
        let dir = fresh(name);
        let index = dir.join("index");
        write(
            &index,
            &[("config.json", r#"{"dl":"dl/{name}-{version}.nova-pkg"}"#)],
        );
        Fixture {
            home: dir.join("home"),
            index,
            dir,
        }
    }

    /// `nova`, in `cwd`, with this fixture's index and home.
    fn nova(&self, cwd: &Path) -> Command {
        let mut command = Command::cargo_bin("nova").expect("nova binary builds");
        command
            .current_dir(cwd)
            .env("NOVA_INDEX", &self.index)
            .env("NOVA_HOME", &self.home);
        command
    }

    /// The library `name` `version` in `dir/sources/<name>-<version>`.
    fn library(&self, name: &str, version: &str, extra: &str, lib: &str) -> PathBuf {
        let dir = self.dir.join("sources").join(format!("{name}-{version}"));
        write(
            &dir,
            &[
                ("nova.toml", manifest(name, version, extra).as_str()),
                ("src/lib.nova", lib),
            ],
        );
        dir
    }

    /// Publish it with `nova publish`.
    fn publish(&self, name: &str, version: &str, extra: &str, lib: &str) {
        let source = self.library(name, version, extra, lib);
        self.nova(&source).arg("publish").assert().success();
    }

    /// Publish it without `nova publish`'s verification, as a package
    /// published by an older or careless nova could be.
    fn publish_by_hand(&self, name: &str, version: &str, lib: &str) {
        let source = self.library(name, version, "", lib);
        let packed =
            nova_index::pack(&source, name, &semver::Version::parse(version).unwrap()).unwrap();
        let line = Line {
            name: name.into(),
            vers: version.into(),
            deps: Vec::new(),
            cksum: packed.checksum,
            v: 1,
        };
        nova_index::publish_local(&self.index(), &line, &packed.bytes).unwrap();
    }

    fn index(&self) -> Index {
        Index::new(self.index.to_str().unwrap()).unwrap()
    }

    /// The program `app` in `dir/app`, with `extra` appended to its
    /// manifest and `main` as its `src/main.nova`.
    fn app(&self, extra: &str, main: &str) -> PathBuf {
        let app = self.dir.join("app");
        write(
            &app,
            &[
                ("nova.toml", manifest("app", "0.1.0", extra).as_str()),
                ("src/main.nova", main),
            ],
        );
        app
    }

    /// Where `name` `version` is unpacked.
    fn cached(&self, name: &str, version: &str) -> PathBuf {
        self.home
            .join("registry")
            .join("src")
            .join(nova_pm::index_dir_name(&self.index().canonical))
            .join(format!("{name}-{version}"))
    }
}

#[test]
fn package_writes_a_reproducible_tarball_and_reports_it() {
    let f = Fixture::new("package");
    let geom = f.library("geom", "0.1.0", "", AREA);
    let out = stdout(&f.nova(&geom).arg("package").assert().success());
    let tarball = geom
        .join("target")
        .join("package")
        .join("geom-0.1.0.nova-pkg");
    let bytes = std::fs::read(&tarball).unwrap();
    assert!(out.contains("packed geom 0.1.0: "), "{out}");
    assert!(out.contains("geom-0.1.0.nova-pkg"), "{out}");
    assert!(out.contains("2 files"), "{out}");
    assert!(out.contains(&nova_index::sha256_hex(&bytes)), "{out}");
    f.nova(&geom).arg("package").assert().success();
    assert_eq!(std::fs::read(&tarball).unwrap(), bytes);
}

#[test]
fn publish_writes_the_tarball_where_dl_says_and_appends_a_line() {
    let f = Fixture::new("publish");
    f.publish("geom", "0.1.0", "", AREA);
    let text = read(&f.index.join("ge").join("om").join("geom"));
    assert!(
        text.starts_with("{\"name\":\"geom\",\"vers\":\"0.1.0\",\"deps\":[],\"cksum\":\""),
        "{text}"
    );
    assert!(text.ends_with("\"v\":1}\n"), "{text}");
    assert!(f.index.join("dl").join("geom-0.1.0.nova-pkg").is_file());
}

#[test]
fn a_version_already_published_is_refused() {
    let f = Fixture::new("again");
    f.publish("geom", "0.1.0", "", AREA);
    let file = f.index.join("ge").join("om").join("geom");
    let before = read(&file);
    let source = f.library("geom", "0.1.0", "", AREA);
    let out = f.nova(&source).arg("publish").assert().failure();
    assert!(
        stderr(&out).contains("geom 0.1.0 is already in the index"),
        "{}",
        stderr(&out)
    );
    assert_eq!(read(&file), before);
}

#[test]
fn a_capitalised_name_lives_at_the_lower_case_path() {
    // Review Focus 4.
    let f = Fixture::new("capital");
    f.publish("Geom", "0.1.0", "", AREA);
    let text = read(&f.index.join("ge").join("om").join("geom"));
    assert!(text.contains("\"name\":\"Geom\""), "{text}");
    let source = f.library("geom", "0.2.0", "", AREA);
    let out = f.nova(&source).arg("publish").assert().failure();
    assert!(
        stderr(&out).contains("differs from `geom` only in case"),
        "{}",
        stderr(&out)
    );
    // Resolved by its exact name only.
    let mut reader = nova_index::LocalReader {
        dir: f.index.clone(),
    };
    let mut view = nova_index::View::new(&mut reader);
    assert!(view.versions("Geom").unwrap().is_some());
    assert_eq!(view.versions("geom").unwrap(), None);
}

#[test]
fn a_path_dependency_is_m0017_and_a_path_dev_dependency_is_allowed() {
    let f = Fixture::new("m0017");
    f.library("util", "0.1.0", "", AREA);
    let geom = f.library(
        "geom",
        "0.1.0",
        "\n[dependencies]\nutil = { path = \"../util-0.1.0\" }\n",
        AREA,
    );
    let out = f.nova(&geom).arg("package").assert().failure();
    let err = stderr(&out);
    assert!(err.contains("M0017"), "{err}");
    assert!(
        err.contains("publish `util` first and depend on its version"),
        "{err}"
    );
    assert!(!geom.join("target").exists());
    let geom = f.library(
        "geom",
        "0.1.0",
        "\n[dev-dependencies]\nutil = { path = \"../util-0.1.0\" }\n",
        AREA,
    );
    f.nova(&geom).arg("package").assert().success();
}

#[test]
fn package_refuses_a_program_and_a_broken_manifest() {
    let f = Fixture::new("refusals");
    let app = f.app("", "fn main() {}\n");
    let out = f.nova(&app).arg("package").assert().failure();
    assert!(
        stderr(&out).contains("only a library can be published"),
        "{}",
        stderr(&out)
    );
    let broken = f.library("geom", "0.1.0", "colour = \n", AREA);
    let out = f.nova(&broken).arg("package").assert().failure();
    assert!(stderr(&out).contains("M0001"), "{}", stderr(&out));
    assert!(!broken.join("target").exists());
}

#[test]
fn verification_fails_on_a_warning_in_the_package() {
    let f = Fixture::new("own-warning");
    let geom = f.library("geom", "0.1.0", "", UNREACHABLE);
    let out = f.nova(&geom).arg("package").assert().failure();
    let err = stderr(&out);
    assert!(err.contains("E0021"), "{err}");
    assert!(err.contains("0 errors and 1 warning"), "{err}");
    assert!(!geom.join("target").exists());
}

#[test]
fn verification_fails_on_a_dependency_not_published() {
    let f = Fixture::new("unpublished");
    let geom = f.library("geom", "0.1.0", "\n[dependencies]\njson = \"1\"\n", AREA);
    let out = f.nova(&geom).arg("package").assert().failure();
    assert!(stderr(&out).contains("M0014"), "{}", stderr(&out));
}

#[test]
fn verification_passes_despite_a_warning_in_a_dependency() {
    let f = Fixture::new("dependency-warning");
    f.publish_by_hand("json", "1.0.0", UNREACHABLE);
    let geom = f.library(
        "geom",
        "0.1.0",
        "\n[dependencies]\njson = \"1\"\n",
        "import json\n\npub fn area() -> Int {\n    pick(3)\n}\n",
    );
    f.nova(&geom).arg("package").assert().success();
    // The verification downloaded `json`; nova refuses to work inside its
    // cache.
    let out = f
        .nova(&f.cached("json", "1.0.0"))
        .arg("package")
        .assert()
        .failure();
    assert!(
        stderr(&out).contains("is a downloaded package in nova's cache; it is read only"),
        "{}",
        stderr(&out)
    );
}

#[test]
fn publishing_to_a_local_index_needs_a_relative_dl() {
    let f = Fixture::new("absolute-dl");
    write(
        &f.index,
        &[("config.json", r#"{"dl":"https://example.test/{name}"}"#)],
    );
    let geom = f.library("geom", "0.1.0", "", AREA);
    let out = f.nova(&geom).arg("publish").assert().failure();
    assert!(
        stderr(&out).contains("must be a path under the index"),
        "{}",
        stderr(&out)
    );
}
```

`EXE_SUFFIX` is used by Task 11's tests. Until then, write the import as
`#[allow(unused_imports)] use std::env::consts::EXE_SUFFIX;`. Task 11
removes the `allow`.

```bash
cd /d/Projects/nona/nova && cargo test --locked -p nova-index --test publish 2>&1 | grep -E "^error" | head -2; cargo test --locked -p nova-cli --test registry 2>&1 | grep -E "^error" | head -3
```

- [ ] **Step 2: Verify they fail**

Expected:
- `error[E0432]: unresolved import` naming `publish_local`;
- `error[E0433]: failed to resolve: use of undeclared crate or module`
  naming `nova_index` and `semver` in `nova-cli`'s test.

- [ ] **Step 3: Implement**

`crates/nova-index/src/publish.rs`:

```rust
//! Publishing to a local index (spec §7, "A local index"). GitHub
//! publishing is in `github.rs`.

use crate::download::write_atomically;
use crate::line::{parse_lines, Line};
use crate::location::{index_path, Index, Location, Source};
use crate::read::{LocalReader, Reader};

/// Spec §7, step 1: `line` must be a version not yet in the index, under a
/// name spelled as any already in its file (§3.1). `existing` is the
/// file's text, and `file` its path in the index.
pub fn check_new(existing: &str, file: &str, line: &Line) -> Result<(), String> {
    let (lines, _) = parse_lines(existing, file);
    if let Some(other) = lines.iter().find(|l| l.name != line.name) {
        return Err(format!(
            "the index has `{}`, which differs from `{}` only in case; a name keeps the \
             spelling it was first published with",
            other.name, line.name
        ));
    }
    if lines.iter().any(|l| l.vers == line.vers) {
        return Err(format!(
            "{} {} is already in the index; a published version is never replaced",
            line.name, line.vers
        ));
    }
    Ok(())
}

/// Publish `tarball` and `line` to `index`, a local directory: the tarball
/// where `config.json`'s `dl` says, then the line appended to the
/// package's file, each through a temporary file renamed into place. The
/// line is the commit point: until it is written, nothing points at the
/// tarball.
pub fn publish_local(index: &Index, line: &Line, tarball: &[u8]) -> Result<(), String> {
    let Source::Local(dir) = &index.source else {
        return Err(format!("{} is not a local index", index.canonical));
    };
    let mut reader = LocalReader { dir: dir.clone() };
    let config = reader.config()?;
    let file = index_path(&line.name);
    let existing = reader.file(&file)?.unwrap_or_default();
    check_new(&existing, &file, line)?;
    let Location::File(target) = index.tarball(&config.dl, &line.name, &line.vers)? else {
        return Err(
            "a local index's `dl` in config.json must be a path under the index, not a URL"
                .to_string(),
        );
    };
    write_atomically(&target, tarball)
        .map_err(|e| format!("cannot write {}: {e}", target.display()))?;
    let mut text = existing;
    if !text.is_empty() && !text.ends_with('\n') {
        text.push('\n');
    }
    text.push_str(&line.to_json());
    text.push('\n');
    let path = file
        .split('/')
        .fold(dir.clone(), |path, part| path.join(part));
    write_atomically(&path, text.as_bytes())
        .map_err(|e| format!("cannot write {}: {e}", path.display()))
}
```

In `crates/nova-index/src/lib.rs`, add `mod publish;` and
`pub use publish::{check_new, publish_local};`.

In `crates/nova-cli/Cargo.toml`, after `nova-lsp = …`:

```toml
# The package index: syncing, packing and publishing (spec
# docs/superpowers/specs/2026-10-09-phase-3-3b-index-and-publishing-design.md
# §9).
nova-index = { path = "../nova-index" }
# `nova add json@1.4` checks its requirement.
semver = { workspace = true }
```

In `crates/nova-cli/src/project.rs`:
- the `use std::path::PathBuf;` line becomes `use std::path::{Path, PathBuf};`;
- append:

```rust
/// Refuse to work inside nova's cache of downloaded packages, which is
/// never edited in place (spec 3.3b §5.1, §5.4).
pub fn refuse_cached(root: &Path) -> Result<()> {
    let Some(registry) = nova_pm::registry_dir() else {
        return Ok(());
    };
    let dir = nova_pm::real_path(if root.as_os_str().is_empty() {
        Path::new(".")
    } else {
        root
    });
    if dir.starts_with(nova_pm::real_path(&registry)) {
        bail!(
            "`{}` is a downloaded package in nova's cache; it is read only",
            dir.display()
        );
    }
    Ok(())
}
```

`crates/nova-cli/src/cmd/package.rs`:

```rust
//! `nova package` and `nova publish` (spec
//! `docs/superpowers/specs/2026-10-09-phase-3-3b-index-and-publishing-design.md`
//! §6.5, §6.6, §6.8, §7).

use std::path::{Path, PathBuf};

use anyhow::{anyhow, bail, Context, Result};
use nova_diagnostics::{render, Diagnostic, FileDb, Severity};
use nova_driver::{Program, Roots};
use nova_index::{Index, Line, LineDep, Packed, Reader, Source, SyncError, SyncRequest, LINE_VERSION};
use nova_pm::{Manifest, Offline, Unlock};

/// A package packed and verified.
struct Prepared {
    manifest: Manifest,
    packed: Packed,
    /// `target/package/<name>-<version>.nova-pkg`.
    path: PathBuf,
}

impl Prepared {
    /// Its index line (spec §3.1): its `[dependencies]` only, each
    /// requirement as `semver` prints it (plan decision 14).
    fn line(&self) -> Line {
        Line {
            name: self.manifest.package.name.clone(),
            vers: self.manifest.package.version.to_string(),
            deps: self
                .manifest
                .dependencies
                .iter()
                .filter_map(|d| {
                    d.version.as_ref().map(|req| LineDep {
                        name: d.name.clone(),
                        req: req.to_string(),
                    })
                })
                .collect(),
            cksum: self.packed.checksum.clone(),
            v: LINE_VERSION,
        }
    }
}

pub fn package() -> Result<()> {
    let index = Index::from_env().map_err(|e| anyhow!(e))?;
    let mut reader = nova_index::reader_for(&index);
    let prepared = prepare(&index, reader.as_mut())?;
    report(&prepared);
    Ok(())
}

pub fn publish() -> Result<()> {
    let index = Index::from_env().map_err(|e| anyhow!(e))?;
    match &index.source {
        Source::Local(_) => {
            let mut reader = nova_index::reader_for(&index);
            let prepared = prepare(&index, reader.as_mut())?;
            report(&prepared);
            nova_index::publish_local(&index, &prepared.line(), &prepared.packed.bytes)
                .map_err(|e| anyhow!(e))?;
            let package = &prepared.manifest.package;
            println!(
                "published {} {} to {}",
                package.name, package.version, index.canonical
            );
            Ok(())
        }
        Source::Http(_) => bail!(
            "`nova publish` writes only to a local index so far; {} is read over HTTP",
            index.canonical
        ),
    }
}

/// Pack the package around the current directory, then verify the
/// tarball (spec §6.5, §6.6). The tarball is written to `target/package/`
/// only once both pass.
fn prepare(index: &Index, reader: &mut dyn Reader) -> Result<Prepared> {
    let cwd = std::env::current_dir().context("reading the current directory")?;
    let root =
        nova_pm::find_root(&cwd).context("no nova.toml here or in any directory above")?;
    crate::project::refuse_cached(&root)?;
    let manifest_path = root.join(nova_pm::MANIFEST);
    let text = std::fs::read_to_string(&manifest_path)
        .with_context(|| format!("reading {}", manifest_path.display()))?;
    let mut db = FileDb::new();
    let file = db.add(manifest_path.display().to_string(), text.as_str());
    let (manifest, diagnostics) = nova_pm::parse(&text, file);
    let Some(manifest) =
        manifest.filter(|_| !diagnostics.iter().any(|d| d.severity == Severity::Error))
    else {
        render::emit_all(&db, &diagnostics);
        bail!("nova.toml has errors; nothing was packed");
    };
    let name = manifest.package.name.clone();
    if !root.join("src").join("lib.nova").is_file() {
        bail!("`{name}` has no src/lib.nova: only a library can be published");
    }
    let paths: Vec<Diagnostic> = manifest
        .dependencies
        .iter()
        .filter(|d| d.path.is_some())
        .map(|d| {
            Diagnostic::error(
                "M0017",
                format!(
                    "a published package cannot have a path dependency; publish `{}` first and \
                     depend on its version",
                    d.name
                ),
            )
            .with_primary_label(d.span, "a path dependency")
        })
        .collect();
    if !paths.is_empty() {
        render::emit_all(&db, &paths);
        bail!("`{name}` was not packed");
    }
    let version = manifest.package.version.clone();
    let packed = nova_index::pack(&root, &name, &version).map_err(|e| anyhow!(e))?;
    let mut deps: Vec<String> = manifest
        .dependencies
        .iter()
        .map(|d| d.name.clone())
        .collect();
    deps.sort();
    verify(&packed, &name, &version, &deps, index, reader)?;
    let path = root
        .join("target")
        .join("package")
        .join(format!("{name}-{version}.nova-pkg"));
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).with_context(|| format!("creating {}", dir.display()))?;
    }
    std::fs::write(&path, &packed.bytes)
        .with_context(|| format!("writing {}", path.display()))?;
    Ok(Prepared {
        manifest,
        packed,
        path,
    })
}

/// Spec §6.6: the tarball, unpacked in `<temp>/nova-verify-<pid>/` (plan
/// decision 7), must resolve with no lock and no dev-dependencies, then
/// check with no error and no warning. A dependency's warnings are never
/// shown, so every warning counted is the package's own (plan decision 15).
fn verify(
    packed: &Packed,
    name: &str,
    version: &semver::Version,
    deps: &[String],
    index: &Index,
    reader: &mut dyn Reader,
) -> Result<()> {
    let dir = std::env::temp_dir().join(format!("nova-verify-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let root = dir.join(format!("{name}-{version}"));
    let result = nova_index::unpack(&packed.bytes, name, version, deps, &root)
        .map_err(|e| anyhow!(e))
        .and_then(|()| check_unpacked(&root, index, reader));
    let _ = std::fs::remove_dir_all(&dir);
    result
}

fn check_unpacked(root: &Path, index: &Index, reader: &mut dyn Reader) -> Result<()> {
    let registry = nova_pm::registry_dir();
    let mut db = FileDb::new();
    let request = SyncRequest {
        root,
        manifest: None,
        dev: false,
        unlock: Unlock::All,
        write: false,
        index,
        reader,
        registry: registry.as_deref(),
    };
    let synced = match nova_index::sync(request, &mut db) {
        Ok(synced) => synced,
        Err(SyncError::Diagnostics(diagnostics)) => {
            render::emit_all(&db, &diagnostics);
            bail!("verification failed: the package's dependencies do not resolve; nothing was written");
        }
        Err(SyncError::Other(why)) => bail!("verification failed: {why}"),
    };
    for note in &synced.notes {
        eprintln!("note: {note}");
    }
    for package in &synced.unpacked {
        eprintln!("downloaded {package}");
    }
    let offline = Offline {
        registry,
        lock: synced.lock,
        dev: false,
    };
    let checked =
        nova_driver::check_program_counted(Program::for_package_in(root, Roots::Check, &offline))?;
    if checked.errors > 0 || checked.warnings > 0 {
        bail!(
            "verification failed: {} and {} in the packed package; nothing was written",
            count(checked.errors, "error"),
            count(checked.warnings, "warning")
        );
    }
    Ok(())
}

fn count(n: usize, word: &str) -> String {
    format!("{n} {word}{}", if n == 1 { "" } else { "s" })
}

fn report(prepared: &Prepared) {
    let package = &prepared.manifest.package;
    println!(
        "packed {} {}: {}",
        package.name,
        package.version,
        prepared.path.display()
    );
    println!(
        "  {}, {} bytes",
        count(prepared.packed.files, "file"),
        prepared.packed.bytes.len()
    );
    println!("  sha256 {}", prepared.packed.checksum);
}
```

In `crates/nova-cli/src/cmd/mod.rs`, add `pub mod package;` after
`pub mod new;`.

In `crates/nova-cli/src/main.rs`:
1. In `enum Command`, after `Remove(…)`:

   ```rust
       /// Pack the library into target/package/, and verify the tarball.
       Package,
       /// Pack and verify the library, then publish it to the package
       /// index.
       Publish,
   ```

2. In the `match`, after `Command::Remove(cmd) => …`:

   ```rust
           Command::Package => cmd::package::package(),
           Command::Publish => cmd::package::publish(),
   ```

3. The file's doc comment: the subcommand list gains "package, publish",
   and its last sentence gains ", and Phase 3.3b the package index (spec
   `docs/superpowers/specs/2026-10-09-phase-3-3b-index-and-publishing-design.md`)".

- [ ] **Step 4: Run the tests to verify they pass**

```bash
cd /d/Projects/nona/nova && P=/c/Users/SAKEER~1/AppData/Local/Temp/gcm/p34 && cargo check -p nova-cli --offline > /dev/null 2>&1; git diff --stat -- Cargo.lock; (cargo test --locked -p nova-index && cargo test --locked -p nova-cli --test registry --test deps --test packages) > $P/t10.txt 2>&1; echo "exit=$?"; python -X utf8 $P/count.py $P/t10.txt; grep -E "FAILED|panicked|^warning|^error" $P/t10.txt | head
```

Expected:
- `Cargo.lock` changes in `nova-cli`'s entry only: `nova-index` and
  `semver`;
- `exit=0`, 0 failed, no warnings;
- 4 tests in `publish.rs` and 10 in `registry.rs`;
- `deps.rs` and `packages.rs` unchanged and passing.

- [ ] **Step 5: Commit**

Write `$P/msg-10.txt`:

```
nova package, and nova publish to a local index

`nova package` packs the library around the current directory into
target/package/<name>-<version>.nova-pkg and prints its path, file
count, size and SHA-256 (spec
docs/superpowers/specs/2026-10-09-phase-3-3b-index-and-publishing-design.md
§6.5). It refuses, writing nothing:
- a manifest with errors;
- a package without src/lib.nova;
- a path entry in [dependencies], as M0017. Path dev-dependencies are
  allowed, since no dependent reads them;
- what `pack` refuses.

Before anything is written, it verifies the tarball (§6.6). It unpacks
it in a temporary directory, resolves its dependencies afresh with no
lock and no dev-dependencies, and checks it as `nova check` does. Any
error or warning fails it; a dependency's warnings are not shown, so
only the package's own count.

`nova publish` then writes to a local index (§7): the tarball where
config.json's `dl` says, then its line, which is the commit point. The
index is append-only, and a name keeps its first spelling's case.
GitHub publishing comes next.

nova refuses to work inside its own cache of downloaded packages.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
```

```bash
cd /d/Projects/nona/nova && P=/c/Users/SAKEER~1/AppData/Local/Temp/gcm/p34 && cargo fmt --all && git add crates/nova-index crates/nova-cli Cargo.lock && git commit -q -F $P/msg-10.txt && git log -1 --format=%s
```

Expected: `nova package, and nova publish to a local index`.

The task's test command: `bash -c "cargo test --locked -p nova-index && cargo test --locked -p nova-cli --test registry --test deps --test packages"`.

---

### Task 11: Commands that sync, `nova fetch`, `nova update`, and `nova add` from the index

Spec §5.3, §6.1–§6.4, §10.5.

**Files:**
- Modify: `crates/nova-cli/src/project.rs`, `crates/nova-cli/src/cmd/run.rs`, `crates/nova-cli/src/cmd/test.rs`, `crates/nova-cli/src/cmd/deps.rs`, `crates/nova-cli/src/cmd/mod.rs`, `crates/nova-cli/src/main.rs`
- Create: `crates/nova-cli/src/cmd/fetch.rs`
- Modify (tests): `crates/nova-cli/tests/registry.rs`
- Modify (tests, on purpose):
  - `crates/nova-cli/tests/project.rs`: `a_declared_dependency_is_m0005`
    is renamed `a_declared_dependency_the_index_lacks_is_m0014`;
  - `crates/nova-cli/tests/packages.rs`: the M0005 case of
    `each_graph_error_is_rendered_and_stops_the_command`;
  - `crates/nova-cli/tests/deps.rs`: the `nova add geom` case of
    `add_refuses_without_writing`.

**Interfaces:**
- Consumes:
  - Task 10's `refuse_cached`;
  - Task 9's `sync`, `Synced`, `SyncError` and `write_lock`;
  - Task 4's `graph_with` and `Offline`;
  - Task 2's `parse_lock`.
- Produces, in `nova-cli`'s `project` module:
  - `package_root(&Mode) -> Option<PathBuf>`;
  - `refuse_library(&Mode) -> Result<()>`;
  - `sync(&Mode) -> Result<()>`;
  - `sync_with(&Mode, Unlock) -> Result<Option<Synced>>`;
  - `finish(Result<Synced, SyncError>, &FileDb) -> Result<Synced>`.
- Produces the commands `nova fetch` and `nova update [NAME]`.
- `nova add` takes `NAME[@REQ]`. Without `--path`, it adds a registry
  entry.

`nova run` and `nova build` refuse a library before syncing (spec §5.3:
"after the CLI's own refusals"). So a library with a graph error now
hears "is a library" first. `program_to_run` keeps its own check.

A sync's graph errors now stop a command before the driver runs, with
"could not resolve the dependencies due to N previous error(s)". The
diagnostics are the same, each rendered once.

- [ ] **Step 1: Write the failing tests**

In `crates/nova-cli/tests/registry.rs`, the `EXE_SUFFIX` import loses its
`#[allow(unused_imports)]`, and these tests are appended:

```rust
#[test]
fn add_from_the_index_writes_the_version_chosen_and_the_lock() {
    let f = Fixture::new("add");
    f.publish("geom", "0.1.0", "", AREA);
    f.publish("geom", "0.2.0", "", AREA);
    let app = f.app("", MAIN_AREA);
    let out = stdout(&f.nova(&app).args(["add", "geom"]).assert().success());
    assert!(
        out.contains("added geom = \"0.2.0\" to [dependencies]"),
        "{out}"
    );
    assert!(read(&app.join("nova.toml")).contains("geom = \"0.2.0\""));
    let lock = read(&app.join("nova.lock"));
    assert!(lock.contains("name = \"geom\"\nversion = \"0.2.0\""), "{lock}");
}

#[test]
fn add_with_a_requirement_writes_it_as_given() {
    let f = Fixture::new("add-requirement");
    f.publish("geom", "0.1.0", "", AREA);
    f.publish("geom", "0.2.0", "", AREA);
    let app = f.app("", MAIN_AREA);
    f.nova(&app).args(["add", "geom@0.1"]).assert().success();
    assert!(read(&app.join("nova.toml")).contains("geom = \"0.1\""));
    assert!(read(&app.join("nova.lock")).contains("version = \"0.1.0\""));
}

#[test]
fn add_from_the_index_keeps_a_crlf_manifests_line_endings() {
    // Review Focus 2.
    let f = Fixture::new("add-crlf");
    f.publish("geom", "0.1.0", "", AREA);
    let app = f.app("", MAIN_AREA);
    let crlf = manifest("app", "0.1.0", "\n[dependencies]\n").replace('\n', "\r\n");
    std::fs::write(app.join("nova.toml"), &crlf).unwrap();
    f.nova(&app).args(["add", "geom"]).assert().success();
    let text = read(&app.join("nova.toml"));
    assert!(text.contains("geom = \"0.1.0\"\r\n"), "{text:?}");
    assert!(!text.replace("\r\n", "").contains('\n'), "{text:?}");
}

#[test]
fn a_refused_add_leaves_both_files_untouched() {
    let f = Fixture::new("add-refused");
    f.publish("geom", "0.1.0", "", AREA);
    f.publish("json", "1.0.0", "", AREA);
    let app = f.app("", MAIN_AREA);
    f.nova(&app).args(["add", "geom"]).assert().success();
    let manifest_before = read(&app.join("nova.toml"));
    let lock_before = read(&app.join("nova.lock"));
    for (args, expected) in [
        (&["add", "nope"][..], "M0014"),
        (&["add", "json@9"][..], "M0015"),
        (&["add", "x@1", "--path", "../x"][..], "never both"),
    ] {
        let out = f.nova(&app).args(args).assert().failure();
        assert!(stderr(&out).contains(expected), "{args:?}: {}", stderr(&out));
        assert_eq!(read(&app.join("nova.toml")), manifest_before, "{args:?}");
        assert_eq!(read(&app.join("nova.lock")), lock_before, "{args:?}");
    }
}

#[test]
fn an_app_runs_builds_and_tests_with_a_published_library() {
    let f = Fixture::new("end-to-end");
    f.publish("geom", "0.1.0", "", AREA);
    let app = f.app("", MAIN_AREA);
    write(
        &app,
        &[(
            "tests/area.nova",
            "import geom\n\n@test\nfn area_is_nine() {\n    assert_eq(area(), 9)\n}\n",
        )],
    );
    f.nova(&app).args(["add", "geom"]).assert().success();
    let out = f.nova(&app).arg("run").assert().success();
    assert_eq!(stdout(&out).trim(), "9");
    f.nova(&app).arg("build").assert().success();
    let exe = app
        .join("target")
        .join("debug")
        .join(format!("app{EXE_SUFFIX}"));
    let ran = std::process::Command::new(&exe).output().unwrap();
    assert_eq!(String::from_utf8_lossy(&ran.stdout).trim(), "9");
    f.nova(&app).arg("test").assert().success();
}

#[test]
fn update_moves_to_a_newer_version_and_says_so() {
    let f = Fixture::new("update");
    f.publish("geom", "0.1.0", "", AREA);
    let app = f.app("", MAIN_AREA);
    f.nova(&app).args(["add", "geom@0.1"]).assert().success();
    f.publish("geom", "0.1.1", "", AREA);
    let out = stdout(&f.nova(&app).arg("update").assert().success());
    assert!(out.contains("geom 0.1.0 -> 0.1.1"), "{out}");
    let out = stdout(&f.nova(&app).arg("update").assert().success());
    assert!(out.contains("nothing to update"), "{out}");
}

#[test]
fn update_one_name_leaves_the_others() {
    let f = Fixture::new("update-one");
    f.publish("geom", "0.1.0", "", AREA);
    f.publish("json", "1.0.0", "", AREA);
    let app = f.app("", MAIN_AREA);
    f.nova(&app).args(["add", "geom@0.1"]).assert().success();
    f.nova(&app).args(["add", "json@1"]).assert().success();
    f.publish("geom", "0.1.1", "", AREA);
    f.publish("json", "1.0.1", "", AREA);
    let out = stdout(&f.nova(&app).args(["update", "geom"]).assert().success());
    assert!(out.contains("geom 0.1.0 -> 0.1.1"), "{out}");
    assert!(!out.contains("json"), "{out}");
    assert!(read(&app.join("nova.lock")).contains("version = \"1.0.0\""));
    let out = f.nova(&app).args(["update", "nope"]).assert().failure();
    assert!(
        stderr(&out).contains("`nope` is not in nova.lock"),
        "{}",
        stderr(&out)
    );
}

#[test]
fn fetch_fills_the_cache_and_a_build_needs_no_index() {
    let f = Fixture::new("fetch");
    f.publish("geom", "0.1.0", "", AREA);
    let app = f.app("", MAIN_AREA);
    f.nova(&app).args(["add", "geom"]).assert().success();
    std::fs::remove_dir_all(&f.home).unwrap();
    let out = stdout(&f.nova(&app).arg("fetch").assert().success());
    assert!(out.contains("fetched 1 package"), "{out}");
    let out = stdout(&f.nova(&app).arg("fetch").assert().success());
    assert!(out.contains("nothing to fetch"), "{out}");
    std::fs::rename(&f.index, f.dir.join("index-gone")).unwrap();
    f.nova(&app).arg("build").assert().success();
}

#[test]
fn a_command_inside_the_cache_is_refused() {
    let f = Fixture::new("inside-cache");
    f.publish("geom", "0.1.0", "", AREA);
    let app = f.app("", MAIN_AREA);
    f.nova(&app).args(["add", "geom"]).assert().success();
    let out = f
        .nova(&f.cached("geom", "0.1.0"))
        .arg("check")
        .assert()
        .failure();
    assert!(
        stderr(&out).contains("is a downloaded package in nova's cache; it is read only"),
        "{}",
        stderr(&out)
    );
}

#[test]
fn a_library_is_refused_by_run_before_anything_syncs() {
    let f = Fixture::new("library-run");
    let geom = f.library("geom", "0.1.0", "\n[dependencies]\nnope = \"1\"\n", AREA);
    let out = f.nova(&geom).arg("run").assert().failure();
    let err = stderr(&out);
    assert!(err.contains("`geom` is a library"), "{err}");
    assert!(!err.contains("M0014"), "{err}");
}
```

In `crates/nova-cli/tests/project.rs`, `a_declared_dependency_is_m0005`
becomes:

```rust
#[test]
fn a_declared_dependency_the_index_lacks_is_m0014() {
    let dir = fresh_dir("dependency");
    write_project(&dir, "demo", HELLO);
    let with_dependency = format!("{}\n[dependencies]\nhttp = \"1.0\"\n", manifest("demo"));
    std::fs::write(dir.join("nova.toml"), with_dependency).unwrap();
    // An empty local index and a cache of the test's own, so nothing
    // reaches the internet (spec 3.3b §10).
    let index = fresh_dir("dependency-index");
    std::fs::write(
        index.join("config.json"),
        r#"{"dl":"dl/{name}-{version}.nova-pkg"}"#,
    )
    .unwrap();
    let out = nova()
        .current_dir(&dir)
        .env("NOVA_INDEX", &index)
        .env("NOVA_HOME", index.join("home"))
        .arg("run")
        .assert()
        .failure();
    let err = stderr(&out);
    assert!(err.contains("M0014") && err.contains("`http`"), "{err}");
}
```

In `crates/nova-cli/tests/packages.rs`, in
`each_graph_error_is_rendered_and_stops_the_command`:
1. the case `("M0005", "\n[dependencies]\nhttp = \"1.0\"\n")` becomes
   `("M0014", "\n[dependencies]\nhttp = \"1.0\"\n")`;
2. before `let cases = [`, add:

   ```rust
       // An empty local index and a cache of the test's own (spec 3.3b
       // §10): `nova check` syncs first.
       let index = dir.join("index");
       write(
           &index,
           &[("config.json", r#"{"dl":"dl/{name}-{version}.nova-pkg"}"#)],
       );
   ```

3. the loop's command becomes:

   ```rust
           let out = nova()
               .current_dir(&app)
               .env("NOVA_INDEX", &index)
               .env("NOVA_HOME", dir.join("home"))
               .arg("check")
               .assert()
               .failure();
   ```

In `crates/nova-cli/tests/deps.rs`, in `add_refuses_without_writing`:
1. the case `(&["add", "geom"], "registry dependencies arrive with the package index")`
   becomes `(&["add", "geom"], "M0014")`;
2. before `let cases:`, add the same `index` as `packages.rs`;
3. the loop's command becomes:

   ```rust
           let out = nova()
               .current_dir(&app)
               .env("NOVA_INDEX", &index)
               .env("NOVA_HOME", dir.join("home"))
               .args(args)
               .assert()
               .failure();
   ```

```bash
cd /d/Projects/nona/nova && P=/c/Users/SAKEER~1/AppData/Local/Temp/gcm/p34 && cargo test --locked -p nova-cli --test registry --test project --test packages --test deps > $P/t11-red.txt 2>&1; python -X utf8 $P/count.py $P/t11-red.txt; grep -E "^test .* FAILED$" $P/t11-red.txt | sort
```

- [ ] **Step 2: Verify they fail**

Expected: these fail, and nothing else:
- the 10 new `registry.rs` tests. `nova add geom` is still refused, and
  `fetch` and `update` are unknown subcommands;
- `a_declared_dependency_the_index_lacks_is_m0014`, which sees M0005;
- `each_graph_error_is_rendered_and_stops_the_command`;
- `add_refuses_without_writing`.

- [ ] **Step 3: Implement**

`crates/nova-cli/src/project.rs`:
1. the `use` lines become:

   ```rust
   use std::path::{Path, PathBuf};

   use anyhow::{anyhow, bail, Context, Result};
   use nova_diagnostics::{render, FileDb, Severity};
   use nova_driver::{Program, Roots};
   use nova_index::{Index, SyncError, SyncRequest, Synced};
   use nova_pm::Unlock;
   ```

2. append:

```rust
/// The package a command syncs (spec 3.3b §5.3): the project, or the
/// package a file argument is directly in. `None` for a loose file.
pub fn package_root(mode: &Mode) -> Option<PathBuf> {
    match mode {
        Mode::Project { root, .. } => Some(root.clone()),
        Mode::File(file) => nova_driver::package_of(file).map(|(root, _)| root),
    }
}

/// `nova run` and `nova build` refuse a project that is only a library
/// before anything is synced (spec 3.3a §5.1, 3.3b §5.3). A manifest with
/// errors is left for the graph to report.
pub fn refuse_library(mode: &Mode) -> Result<()> {
    let Mode::Project { root, .. } = mode else {
        return Ok(());
    };
    let src = root.join("src");
    if !src.join("lib.nova").is_file() || src.join("main.nova").is_file() {
        return Ok(());
    }
    let text = std::fs::read_to_string(root.join(nova_pm::MANIFEST)).unwrap_or_default();
    let (Some(manifest), _) = nova_pm::parse(&text, nova_diagnostics::FileId::DUMMY) else {
        return Ok(());
    };
    bail!(
        "`{}` is a library: it has no src/main.nova; `nova check` and `nova test` work on it",
        manifest.package.name
    )
}

/// Sync the package `mode` works on before a command compiles it, as
/// `nova fetch` does (spec 3.3b §5.3).
pub fn sync(mode: &Mode) -> Result<()> {
    sync_with(mode, Unlock::Nothing).map(|_| ())
}

/// The sync, with `unlock`. `Ok(None)` for a loose file, which has nothing
/// to sync.
pub fn sync_with(mode: &Mode, unlock: Unlock) -> Result<Option<Synced>> {
    let Some(root) = package_root(mode) else {
        return Ok(None);
    };
    refuse_cached(&root)?;
    let index = Index::from_env().map_err(|e| anyhow!(e))?;
    let mut reader = nova_index::reader_for(&index);
    let registry = nova_pm::registry_dir();
    let mut db = FileDb::new();
    let result = nova_index::sync(
        SyncRequest {
            root: &root,
            manifest: None,
            dev: true,
            unlock,
            write: true,
            index: &index,
            reader: reader.as_mut(),
            registry: registry.as_deref(),
        },
        &mut db,
    );
    finish(result, &db).map(Some)
}

/// A sync's outcome: its notes and downloads printed on standard error,
/// its diagnostics rendered.
pub fn finish(result: Result<Synced, SyncError>, db: &FileDb) -> Result<Synced> {
    match result {
        Ok(synced) => {
            for note in &synced.notes {
                eprintln!("note: {note}");
            }
            for package in &synced.unpacked {
                eprintln!("downloaded {package}");
            }
            Ok(synced)
        }
        Err(SyncError::Diagnostics(diagnostics)) => {
            render::emit_all(db, &diagnostics);
            let errors = diagnostics
                .iter()
                .filter(|d| d.severity == Severity::Error)
                .count();
            bail!(
                "could not resolve the dependencies due to {errors} previous error{}",
                if errors == 1 { "" } else { "s" }
            )
        }
        Err(SyncError::Other(why)) => Err(anyhow!(why)),
    }
}
```

In `crates/nova-cli/src/cmd/run.rs`:
- in `run` and `build`, after `let mode = project::mode(cmd.file)?;`:

  ```rust
      project::refuse_library(&mode)?;
      project::sync(&mode)?;
  ```

- in `check`, after its `let mode = …;`: `project::sync(&mode)?;`.

In `crates/nova-cli/src/cmd/test.rs`'s `run`, after
`let mode = crate::project::mode(None)?;`:
`crate::project::sync(&mode)?;`.

`crates/nova-cli/src/cmd/fetch.rs`:

```rust
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
```

In `crates/nova-cli/src/cmd/mod.rs`, add `pub mod fetch;` after
`pub mod deps;`.

In `crates/nova-cli/src/main.rs`:
1. `/// Add a path dependency to nova.toml.` becomes
   `/// Add a dependency to nova.toml.`;
2. after `Remove(…)`, before `Package`:

   ```rust
       /// Download what nova.lock names and the cache lacks, resolving first
       /// if the lock does not answer nova.toml.
       Fetch,
       /// Move locked versions to the newest that fit: every package, or one.
       Update(cmd::fetch::UpdateCmd),
   ```

3. in the `match`:

   ```rust
           Command::Fetch => cmd::fetch::fetch(),
           Command::Update(cmd) => cmd::fetch::update(cmd),
   ```

4. the doc comment's subcommand list gains "fetch, update".

In `crates/nova-cli/src/cmd/deps.rs`:
1. The file's doc comment's first line becomes `//! \`nova add <name>[@<req>] [--path <dir>] [--dev]\` and \`nova remove <name> [--dev]\`:`,
   and it gains "The registry form is spec
   `docs/superpowers/specs/2026-10-09-phase-3-3b-index-and-publishing-design.md`
   §6.1."
2. The `use` lines become:

   ```rust
   use std::path::{Component, Path, PathBuf};

   use anyhow::{anyhow, bail, Context, Result};
   use clap::Args;
   use nova_diagnostics::{render, FileDb, Severity};
   use nova_index::{Index, SyncRequest};
   use nova_pm::{Offline, Unlock};
   use toml_edit::{DocumentMut, InlineTable, Item, Value};
   ```

3. `AddCmd` becomes:

```rust
#[derive(Args)]
pub struct AddCmd {
    /// The dependency: a package name, or `name@requirement` for a package
    /// from the index, such as `json@1.4`.
    name: String,
    /// A local package instead: the directory holding its nova.toml, from
    /// the current directory.
    #[arg(long)]
    path: Option<PathBuf>,
    /// Add it to [dev-dependencies], which only tests/ files import.
    #[arg(long)]
    dev: bool,
}
```

4. `pub fn add` is replaced by the four functions below, and the rest of
   the file is kept:

```rust
pub fn add(cmd: AddCmd) -> Result<()> {
    let (name, requirement) = match cmd.name.split_once('@') {
        Some((name, requirement)) => (name, Some(requirement)),
        None => (cmd.name.as_str(), None),
    };
    nova_pm::check_name(name).map_err(|message| anyhow!(message))?;
    if cmd.path.is_some() && requirement.is_some() {
        bail!(
            "`{}`: an entry has a version or a path, never both",
            cmd.name
        );
    }
    if let Some(requirement) = requirement {
        semver::VersionReq::parse(requirement)
            .map_err(|e| anyhow!("`{requirement}` is not a version requirement: {e}"))?;
    }
    let (root, text) = manifest()?;
    // A manifest with errors is refused, with its errors shown.
    let mut db = FileDb::new();
    let file = db.add(
        root.join(nova_pm::MANIFEST).display().to_string(),
        text.as_str(),
    );
    let (parsed, diagnostics) = nova_pm::parse(&text, file);
    let Some(parsed) =
        parsed.filter(|_| !diagnostics.iter().any(|d| d.severity == Severity::Error))
    else {
        render::emit_all(&db, &diagnostics);
        bail!("nova.toml has errors, so it was left unchanged");
    };
    for (table, entries) in [
        ("dependencies", &parsed.dependencies),
        ("dev-dependencies", &parsed.dev_dependencies),
    ] {
        if entries.iter().any(|entry| entry.name == name) {
            bail!("`{name}` is already in [{table}]");
        }
    }
    let table = table_name(cmd.dev);
    match &cmd.path {
        Some(path) => add_path(&root, &text, name, path, table),
        None => add_registry(&root, &text, name, requirement, table),
    }
}

/// `text` with `name = value` in `[table]`, in `text`'s line endings.
fn with_entry(text: &str, table: &str, name: &str, value: Value) -> Result<String> {
    let mut document = document(text)?;
    if document.get(table).is_none() {
        document.insert(table, toml_edit::table());
    }
    let entries = document
        .get_mut(table)
        .and_then(Item::as_table_like_mut)
        .with_context(|| format!("[{table}] in nova.toml is not a table"))?;
    entries.insert(name, Item::Value(value));
    Ok(to_text(&document, text))
}

/// `nova add <name> --path <dir>` (spec 3.3a §5.3).
fn add_path(root: &Path, text: &str, name: &str, path: &Path, table: &str) -> Result<()> {
    // Relative to the manifest's directory, whatever the current one.
    let cwd = std::env::current_dir().context("reading the current directory")?;
    let written = relative(
        &nova_pm::real_path(root),
        &nova_pm::real_path(&cwd.join(path)),
    );
    let mut entry = InlineTable::new();
    entry.insert("path", Value::from(written.as_str()));
    let new_text = with_entry(text, table, name, Value::InlineTable(entry))?;
    // The graph as it would be: any error refuses, and nothing is written.
    // M0013, a package with no source yet, and M0005, a registry entry not
    // yet downloaded, are not the new entry's fault (spec 3.3a §3.1, 3.3b
    // §6.1).
    let mut db = FileDb::new();
    let (_, diagnostics) = nova_pm::graph_from(root, Some(&new_text), &mut db);
    if diagnostics.iter().any(|d| {
        d.severity == Severity::Error && d.code != "M0013" && d.code != "M0005"
    }) {
        render::emit_all(&db, &diagnostics);
        bail!("`{name}` was not added; nova.toml was left unchanged");
    }
    write(root, &new_text)?;
    println!("added {name} = {{ path = \"{written}\" }} to [{table}]");
    Ok(())
}

/// `nova add <name>[@<req>]` (spec 3.3b §6.1). The manifest it would write
/// is synced in memory, with `"*"` when no requirement was given, so the
/// version fits the rest of the graph; the version chosen is then
/// written, as Cargo writes it. The graph is checked with the lock it
/// would write, and only then are `nova.toml` and `nova.lock` written.
fn add_registry(
    root: &Path,
    text: &str,
    name: &str,
    requirement: Option<&str>,
    table: &str,
) -> Result<()> {
    let refused = || format!("`{name}` was not added; nova.toml and nova.lock were left unchanged");
    crate::project::refuse_cached(root)?;
    let proposed = with_entry(text, table, name, Value::from(requirement.unwrap_or("*")))?;
    let index = Index::from_env().map_err(|e| anyhow!(e))?;
    let mut reader = nova_index::reader_for(&index);
    let registry = nova_pm::registry_dir();
    let mut db = FileDb::new();
    let result = nova_index::sync(
        SyncRequest {
            root,
            manifest: Some(&proposed),
            dev: true,
            unlock: Unlock::Nothing,
            write: false,
            index: &index,
            reader: reader.as_mut(),
            registry: registry.as_deref(),
        },
        &mut db,
    );
    let synced = crate::project::finish(result, &db).with_context(refused)?;
    let lock = synced.lock.with_context(refused)?;
    let written = match requirement {
        Some(requirement) => requirement.to_string(),
        None => lock
            .find(name)
            .map(|package| package.version.to_string())
            .with_context(refused)?,
    };
    let new_text = with_entry(text, table, name, Value::from(written.as_str()))?;
    let offline = Offline {
        registry,
        lock: Some(lock.clone()),
        dev: true,
    };
    let mut db = FileDb::new();
    let (_, diagnostics) = nova_pm::graph_with(root, Some(&new_text), &offline, &mut db);
    if diagnostics
        .iter()
        .any(|d| d.severity == Severity::Error && d.code != "M0013")
    {
        render::emit_all(&db, &diagnostics);
        bail!(refused());
    }
    write(root, &new_text)?;
    nova_index::write_lock(root, &lock).map_err(|e| anyhow!(e))?;
    println!("added {name} = \"{written}\" to [{table}]");
    Ok(())
}
```

- [ ] **Step 4: Run the tests to verify they pass**

Check that port 3000 is free first (Conventions). Then run the whole
`nova-cli` suite:

```bash
cd /d/Projects/nona/nova && P=/c/Users/SAKEER~1/AppData/Local/Temp/gcm/p34 && netstat -ano | grep -E "[:.]3000 .*LISTENING"; cargo test --locked -p nova-cli > $P/t11.txt 2>&1; echo "exit=$?"; python -X utf8 $P/count.py $P/t11.txt; grep -E "^test .* FAILED$|panicked|^warning|^error" $P/t11.txt | head
```

Expected:
- `exit=0`, 0 failed, no warnings;
- `registry.rs` has 20 tests;
- every other `nova-cli` test passes, the three changed ones included.

A `0xc0000005` in a `run_tests` test is the known flake (ADR 0008 §4).
Rerun that test alone, and ledger it.

- [ ] **Step 5: Commit**

Write `$P/msg-11.txt`:

```
nova-cli: commands sync, nova fetch, nova update, and nova add from the index

Before `nova run`, `build`, `check` and `test` work on a package, they
sync it (spec
docs/superpowers/specs/2026-10-09-phase-3-3b-index-and-publishing-design.md
§5.3): nova.lock is brought up to date and what it names is downloaded,
after the CLI's own refusals, so `nova run` in a library is refused
before anything is fetched. A loose file syncs nothing, and a project
with no registry dependency needs neither the index nor $NOVA_HOME.
Downloads are reported on standard error, and nothing runs inside nova's
own cache.

- `nova fetch` is the sync alone, and reports what it fetched.
- `nova update` lets every locked version move. `nova update <name>`
  lets only that one move, and refuses a name not in nova.lock. Each
  change is printed, as `geom 0.1.0 -> 0.1.1`, or "nothing to update".
- `nova add <name>[@<req>]` without --path adds a registry entry (§6.1).
  It syncs the manifest it would write in memory, writes the version
  chosen as Cargo does (or the requirement given), and checks the graph
  with the lock it would write. Only then are nova.toml and nova.lock
  written. Its line endings are kept. `--path` with `@` is refused, and
  `--path` no longer minds another entry not yet downloaded.

The three tests that pinned "registry dependencies arrive with the
package index" now see M0014 against an empty local index of their own.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
```

```bash
cd /d/Projects/nona/nova && P=/c/Users/SAKEER~1/AppData/Local/Temp/gcm/p34 && cargo fmt --all && git add crates/nova-cli && git commit -q -F $P/msg-11.txt && git log -1 --format=%s
```

Expected: `nova-cli: commands sync, nova fetch, nova update, and nova add from the index`.

The task's test command: `cargo test --locked -p nova-cli`.

---

### Task 12: Publishing to GitHub, and `nova login`

Spec §3.3, §6.6–§6.8, §7, §8, §10.4.

**Files:**
- Create: `crates/nova-index/src/github.rs`, `crates/nova-index/src/credentials.rs`
- Modify: `crates/nova-index/src/download.rs` (`write_private`), `crates/nova-index/src/lib.rs`, `crates/nova-index/Cargo.toml`
- Modify: `Cargo.toml` (workspace `base64`), `crates/nova-cli/Cargo.toml` (dev-dependency `base64`), `Cargo.lock`
- Create: `crates/nova-index/tests/support/fake_github.rs`, `crates/nova-index/tests/github.rs`
- Modify: `crates/nova-index/tests/support/mod.rs` (`pub mod fake_github;`)
- Create: `crates/nova-cli/src/cmd/login.rs`
- Modify: `crates/nova-cli/src/cmd/package.rs` (the GitHub arm of `publish`), `crates/nova-cli/src/cmd/mod.rs`, `crates/nova-cli/src/main.rs`, `crates/nova-cli/tests/registry.rs`

**Interfaces:**
- Consumes:
  - Task 10's `check_new`, `prepare` and `report`;
  - Task 8's `Http`, `check_url`, `shown` and `write_atomically`;
  - Task 6's `Reader`, `reader_for` and `index_path`.
- Produces, in `nova-index`:
  - `GitHub::new(repo: &str, token: &str, api: Option<&str>) -> Result<GitHub, String>`
    and `GitHub::from_env(repo, token)`, which reads `NOVA_GITHUB_API`;
  - `GitHub::can_push(&self) -> Result<bool, String>`;
  - `GitHub::read_file(&self, path) -> Result<Option<(String, String)>, String>`,
    giving the text and the blob `sha`;
  - `GitHub::write_file(&self, path, text, sha: Option<&str>, message) -> Result<Written, String>`;
  - `GitHub::release(&self, tag) -> Result<Release, String>`;
  - `GitHub::delete_asset(&self, id: u64) -> Result<(), String>`;
  - `GitHub::upload_asset(&self, release: u64, name, bytes) -> Result<(), String>`;
  - `Written::{Done, Conflict}` and `Release { id: u64, assets: Vec<(u64, String)> }`;
  - `publish_github(&GitHub, &Line, &[u8]) -> Result<(), String>`;
  - `ApiReader<'a> { github: &'a GitHub }`, which implements `Reader`;
  - `credentials_path(&Path) -> PathBuf`;
  - `load_token(&Path) -> Result<Option<String>, String>`;
  - `store_token(&Path, &str) -> Result<PathBuf, String>`;
  - `pub(crate) write_private(&Path, &[u8]) -> std::io::Result<()>`.
- The command `nova login`.
- The stand-in GitHub for tests, in `tests/support/fake_github.rs`:
  - `FakeGitHub::start(config: &str) -> FakeGitHub`, with `.server` and
    `.state: Arc<Mutex<State>>`;
  - `FakeGitHub::TOKEN` (`"ghp_test"`), `FakeGitHub::REPO`
    (`"owner/index"`) and `FakeGitHub::api(&self) -> String`;
  - `State { files, releases, push, conflicts, interloper }`.
  It serves:
  - the Contents API, releases and assets under `/repos/owner/index`;
  - the files raw under `/raw/`;
  - assets under `/dl/<name>`.

Every API request carries three headers:
- `Authorization: Bearer <token>`;
- `Accept: application/vnd.github+json`;
- `X-GitHub-Api-Version: 2022-11-28`.

No API request follows a redirect, since ureq's `max_redirects` is 0.
The token is never in an error, in output, or on a command line.

- [ ] **Step 1: Dependencies**

In the workspace `Cargo.toml`, after `serde_json = "1"`:

```toml
# The GitHub Contents API carries files as base64 (spec
# docs/superpowers/specs/2026-10-09-phase-3-3b-index-and-publishing-design.md
# §7). ureq already brings 0.22; it declares Rust 1.48.
base64 = "0.22"
```

In `crates/nova-index/Cargo.toml`, append to `[dependencies]`:

```toml
# The Contents API's base64, and credentials.toml (spec §6.7, §7). Both
# were already in the lockfile.
base64 = { workspace = true }
toml_edit = { workspace = true }
```

In `crates/nova-cli/Cargo.toml`, append to `[dev-dependencies]`:

```toml
# The stand-in GitHub API the registry tests share with nova-index's.
base64 = { workspace = true }
```

```bash
cd /d/Projects/nona/nova && cargo check -p nova-index -p nova-cli --offline > /dev/null 2>&1; git diff -- Cargo.lock | grep -E '^[+-]' | grep -v '^[+-]{3}'
```

Expected: only the new dependency lines in the `nova-index` and
`nova-cli` entries, and no new `[[package]]`.

- [ ] **Step 2: Write the failing tests**

`crates/nova-index/tests/support/mod.rs` gains, after its `use` lines:

```rust
pub mod fake_github;
```

`crates/nova-index/tests/support/fake_github.rs`:

```rust
//! A stand-in for GitHub (plan Task 12): the Contents API for one
//! repository's files, its releases and their assets, the files raw as
//! raw.githubusercontent.com serves them (under `/raw/`), and the assets
//! as release downloads (under `/dl/`). Every request is recorded by the
//! `Server` underneath.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use base64::Engine;

use super::{Request, Response, Server};

/// What the stand-in holds.
#[derive(Default)]
pub struct State {
    /// Each file by path: its text and its blob sha.
    pub files: HashMap<String, (String, String)>,
    /// Each release by tag: its id and its assets, `(id, name, bytes)`.
    pub releases: HashMap<String, (u64, Vec<(u64, String, Vec<u8>)>)>,
    /// The last id handed out.
    pub next_id: u64,
    /// Whether the token may push.
    pub push: bool,
    /// How many writes to refuse with 409, each after appending
    /// `interloper` to the file, as another publisher would.
    pub conflicts: usize,
    pub interloper: Option<String>,
}

impl State {
    fn id(&mut self) -> u64 {
        self.next_id += 1;
        self.next_id
    }

    fn put(&mut self, path: &str, text: String) {
        let sha = format!("sha-{}", self.id());
        self.files.insert(path.to_string(), (text, sha));
    }
}

pub struct FakeGitHub {
    pub server: Server,
    pub state: Arc<Mutex<State>>,
}

impl FakeGitHub {
    pub const TOKEN: &'static str = "ghp_test";
    pub const REPO: &'static str = "owner/index";

    /// The stand-in, its repository holding `config` as config.json, with
    /// push access.
    pub fn start(config: &str) -> FakeGitHub {
        let mut state = State {
            push: true,
            ..State::default()
        };
        state.put("config.json", config.to_string());
        let state = Arc::new(Mutex::new(state));
        let shared = Arc::clone(&state);
        let server = Server::start(move |request| handle(&shared, request));
        FakeGitHub { server, state }
    }

    /// What `NOVA_GITHUB_API` is set to.
    pub fn api(&self) -> String {
        self.server.url.clone()
    }

    /// The text of a file in the repository.
    pub fn file(&self, path: &str) -> Option<String> {
        self.state
            .lock()
            .unwrap()
            .files
            .get(path)
            .map(|(text, _)| text.clone())
    }
}

fn json(status: u16, value: serde_json::Value) -> Response {
    Response::with_status(status, value.to_string())
}

fn release_json(id: u64, assets: &[(u64, String, Vec<u8>)]) -> serde_json::Value {
    let assets: Vec<serde_json::Value> = assets
        .iter()
        .map(|(id, name, _)| serde_json::json!({"id": id, "name": name}))
        .collect();
    serde_json::json!({"id": id, "assets": assets})
}

fn handle(state: &Mutex<State>, request: &Request) -> Response {
    let mut state = state.lock().unwrap();
    let (path, query) = request
        .path
        .split_once('?')
        .unwrap_or((request.path.as_str(), ""));
    if let Some(file) = path.strip_prefix("/raw/") {
        return match state.files.get(file) {
            Some((text, _)) => Response::ok(text.clone()),
            None => Response::status(404),
        };
    }
    if let Some(name) = path.strip_prefix("/dl/") {
        let found = state
            .releases
            .values()
            .flat_map(|(_, assets)| assets)
            .find(|(_, asset, _)| asset == name)
            .map(|(_, _, bytes)| bytes.clone());
        return found.map_or(Response::status(404), Response::ok);
    }
    let bearer = format!("Bearer {}", FakeGitHub::TOKEN);
    if request.header("authorization") != Some(bearer.as_str()) {
        return json(401, serde_json::json!({"message": "Bad credentials"}));
    }
    let Some(rest) = path.strip_prefix(&format!("/repos/{}", FakeGitHub::REPO)) else {
        return Response::status(404);
    };
    let base64 = base64::engine::general_purpose::STANDARD;
    match request.method.as_str() {
        "GET" if rest.is_empty() => {
            json(200, serde_json::json!({"permissions": {"push": state.push}}))
        }
        "GET" if rest.starts_with("/contents/") => {
            match state.files.get(&rest["/contents/".len()..]) {
                Some((text, sha)) => {
                    // GitHub breaks the base64 into lines.
                    let encoded = base64.encode(text);
                    let lines: Vec<&str> = encoded
                        .as_bytes()
                        .chunks(60)
                        .map(|chunk| std::str::from_utf8(chunk).unwrap())
                        .collect();
                    json(
                        200,
                        serde_json::json!({"content": lines.join("\n"), "sha": sha, "encoding": "base64"}),
                    )
                }
                None => json(404, serde_json::json!({"message": "Not Found"})),
            }
        }
        "PUT" if rest.starts_with("/contents/") => {
            let file = rest["/contents/".len()..].to_string();
            let body: serde_json::Value = serde_json::from_slice(&request.body).unwrap();
            if state.conflicts > 0 {
                state.conflicts -= 1;
                let mut text = state
                    .files
                    .get(&file)
                    .map(|(text, _)| text.clone())
                    .unwrap_or_default();
                text.push_str(state.interloper.as_deref().unwrap_or(""));
                state.put(&file, text);
                return json(409, serde_json::json!({"message": "is at a different sha"}));
            }
            let current = state.files.get(&file).map(|(_, sha)| sha.clone());
            if body["sha"].as_str().map(str::to_string) != current {
                return json(409, serde_json::json!({"message": "sha does not match"}));
            }
            let content = base64
                .decode(body["content"].as_str().unwrap())
                .unwrap();
            state.put(&file, String::from_utf8(content).unwrap());
            json(201, serde_json::json!({"content": {"path": file}}))
        }
        "GET" if rest.starts_with("/releases/tags/") => {
            match state.releases.get(&rest["/releases/tags/".len()..]) {
                Some((id, assets)) => json(200, release_json(*id, assets)),
                None => json(404, serde_json::json!({"message": "Not Found"})),
            }
        }
        "POST" if rest == "/releases" => {
            let body: serde_json::Value = serde_json::from_slice(&request.body).unwrap();
            let tag = body["tag_name"].as_str().unwrap().to_string();
            let id = state.id();
            state.releases.insert(tag, (id, Vec::new()));
            json(201, release_json(id, &[]))
        }
        "DELETE" if rest.starts_with("/releases/assets/") => {
            let id: u64 = rest["/releases/assets/".len()..].parse().unwrap();
            for (_, assets) in state.releases.values_mut() {
                assets.retain(|(asset, _, _)| *asset != id);
            }
            Response::status(204)
        }
        "POST" if rest.starts_with("/releases/") && rest.ends_with("/assets") => {
            let release: u64 = rest["/releases/".len()..rest.len() - "/assets".len()]
                .parse()
                .unwrap();
            let name = query.strip_prefix("name=").unwrap_or("").to_string();
            let id = state.id();
            let body = request.body.clone();
            match state
                .releases
                .values_mut()
                .find(|(release_id, _)| *release_id == release)
            {
                Some((_, assets)) => {
                    assets.push((id, name.clone(), body));
                    json(201, serde_json::json!({"id": id, "name": name}))
                }
                None => Response::status(404),
            }
        }
        _ => Response::status(404),
    }
}
```

`crates/nova-index/tests/github.rs`:

```rust
//! Publishing to GitHub against a stand-in (spec
//! `docs/superpowers/specs/2026-10-09-phase-3-3b-index-and-publishing-design.md`
//! §6.7, §7, §8, §10.4).

mod support;

use nova_index::{
    load_token, publish_github, store_token, ApiReader, GitHub, Line, Reader,
};
use support::fake_github::FakeGitHub;

const CONFIG: &str = r#"{"dl":"dl/{name}-{version}.nova-pkg","api":"owner/index"}"#;

fn line(name: &str, vers: &str) -> Line {
    Line {
        name: name.into(),
        vers: vers.into(),
        deps: Vec::new(),
        cksum: "c".repeat(64),
        v: 1,
    }
}

fn github(fake: &FakeGitHub) -> GitHub {
    GitHub::new(FakeGitHub::REPO, FakeGitHub::TOKEN, Some(&fake.api())).unwrap()
}

/// `METHOD /path` for each request, the query left out.
fn calls(fake: &FakeGitHub) -> Vec<String> {
    fake.server
        .requests()
        .iter()
        .map(|r| format!("{} {}", r.method, r.path.split('?').next().unwrap()))
        .collect()
}

#[test]
fn a_first_publish_reads_releases_uploads_then_appends() {
    let fake = FakeGitHub::start(CONFIG);
    publish_github(&github(&fake), &line("geom", "0.1.0"), b"tarball").unwrap();
    assert_eq!(
        calls(&fake),
        [
            "GET /repos/owner/index/contents/ge/om/geom",
            "GET /repos/owner/index/releases/tags/geom-0.1.0",
            "POST /repos/owner/index/releases",
            "POST /repos/owner/index/releases/2/assets",
            "PUT /repos/owner/index/contents/ge/om/geom",
        ]
    );
    assert_eq!(
        fake.file("ge/om/geom").unwrap(),
        format!("{}\n", line("geom", "0.1.0").to_json())
    );
    for request in fake.server.requests() {
        assert_eq!(request.header("authorization"), Some("Bearer ghp_test"));
        assert_eq!(request.header("accept"), Some("application/vnd.github+json"));
        assert_eq!(request.header("x-github-api-version"), Some("2022-11-28"));
    }
    let upload = &fake.server.requests()[3];
    assert!(upload.path.ends_with("?name=geom-0.1.0.nova-pkg"), "{upload:?}");
    assert_eq!(upload.header("content-type"), Some("application/gzip"));
    assert_eq!(upload.body, b"tarball");
}

#[test]
fn a_stale_sha_is_read_again_and_tried_once_more() {
    let fake = FakeGitHub::start(CONFIG);
    publish_github(&github(&fake), &line("geom", "0.1.0"), b"one").unwrap();
    {
        let mut state = fake.state.lock().unwrap();
        state.conflicts = 1;
        state.interloper = Some(format!("{}\n", line("geom", "0.1.5").to_json()));
    }
    publish_github(&github(&fake), &line("geom", "0.2.0"), b"two").unwrap();
    let text = fake.file("ge/om/geom").unwrap();
    assert_eq!(text.lines().count(), 3, "{text}");
    assert!(text.ends_with(&format!("{}\n", line("geom", "0.2.0").to_json())));
}

#[test]
fn two_stale_shas_give_up_with_nothing_appended() {
    let fake = FakeGitHub::start(CONFIG);
    {
        let mut state = fake.state.lock().unwrap();
        state.conflicts = 2;
        state.interloper = Some(String::new());
    }
    let error = publish_github(&github(&fake), &line("geom", "0.1.0"), b"one").unwrap_err();
    assert!(error.contains("run `nova publish` again"), "{error}");
    assert_eq!(fake.file("ge/om/geom").unwrap_or_default(), "");
}

#[test]
fn an_existing_version_is_refused_before_anything_is_written() {
    let fake = FakeGitHub::start(CONFIG);
    publish_github(&github(&fake), &line("geom", "0.1.0"), b"one").unwrap();
    let before = fake.server.requests().len();
    let error = publish_github(&github(&fake), &line("geom", "0.1.0"), b"two").unwrap_err();
    assert!(error.contains("already in the index"), "{error}");
    let after: Vec<String> = calls(&fake)[before..].to_vec();
    assert_eq!(after, ["GET /repos/owner/index/contents/ge/om/geom"]);
}

#[test]
fn a_resumed_publish_replaces_the_earlier_attempts_asset() {
    let fake = FakeGitHub::start(CONFIG);
    {
        let mut state = fake.state.lock().unwrap();
        let id = state.next_id + 1;
        state.next_id = id + 1;
        state.releases.insert(
            "geom-0.1.0".into(),
            (id, vec![(id + 1, "geom-0.1.0.nova-pkg".into(), b"old".to_vec())]),
        );
    }
    publish_github(&github(&fake), &line("geom", "0.1.0"), b"new").unwrap();
    let calls = calls(&fake);
    assert!(
        calls.iter().any(|c| c.starts_with("DELETE /repos/owner/index/releases/assets/")),
        "{calls:?}"
    );
    let state = fake.state.lock().unwrap();
    let (_, assets) = &state.releases["geom-0.1.0"];
    assert_eq!(assets.len(), 1);
    assert_eq!(assets[0].2, b"new");
}

#[test]
fn the_api_reader_reads_index_files_through_the_api() {
    let fake = FakeGitHub::start(CONFIG);
    publish_github(&github(&fake), &line("geom", "0.1.0"), b"one").unwrap();
    let github = github(&fake);
    let mut reader = ApiReader { github: &github };
    assert_eq!(reader.config().unwrap().api.as_deref(), Some("owner/index"));
    let text = reader.file("ge/om/geom").unwrap().unwrap();
    assert!(text.contains("\"vers\":\"0.1.0\""), "{text}");
    assert_eq!(reader.file("js/on/json").unwrap(), None);
}

#[test]
fn nova_github_api_is_only_ever_a_loopback_address() {
    for refused in [
        "http://example.com",
        "https://example.com",
        "http://127.0.0.1",
        "http://127.0.0.1:80@example.com",
        "http://localhost:8080",
    ] {
        assert!(
            GitHub::new("owner/index", "t", Some(refused)).is_err(),
            "{refused}"
        );
    }
    assert!(GitHub::new("owner/index", "t", Some("http://127.0.0.1:9")).is_ok());
    assert!(GitHub::new("owner/index", "t", Some("http://[::1]:9/")).is_ok());
    assert!(GitHub::new("not a repo", "t", None).is_err());
}

#[test]
fn the_token_never_appears_in_an_error() {
    let fake = FakeGitHub::start(CONFIG);
    let wrong = GitHub::new(FakeGitHub::REPO, "ghp_wrong_secret", Some(&fake.api())).unwrap();
    let error = publish_github(&wrong, &line("geom", "0.1.0"), b"one").unwrap_err();
    assert!(!error.contains("ghp_wrong_secret"), "{error}");
    assert!(!wrong.can_push().unwrap_err().contains("ghp_wrong_secret"));
}

#[test]
fn a_token_is_stored_for_its_owner_and_read_back() {
    let home = std::env::temp_dir().join("nova-index-github-credentials");
    let _ = std::fs::remove_dir_all(&home);
    assert_eq!(load_token(&home).unwrap(), None);
    let path = store_token(&home, "ghp_abc123").unwrap();
    assert_eq!(load_token(&home).unwrap().as_deref(), Some("ghp_abc123"));
    assert_eq!(
        std::fs::read_to_string(&path).unwrap(),
        "[github]\ntoken = \"ghp_abc123\"\n"
    );
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = std::fs::metadata(&path).unwrap().permissions().mode();
        assert_eq!(mode & 0o777, 0o600);
    }
    for bad in ["", "two words", "quote\"d", "line\nbreak"] {
        assert!(store_token(&home, bad).is_err(), "{bad:?}");
    }
}
```

Append to `crates/nova-cli/tests/registry.rs`:

```rust
#[path = "../../nova-index/tests/support/mod.rs"]
mod support;

use support::fake_github::FakeGitHub;

/// A stand-in GitHub index: its config.json read raw from the stand-in,
/// whose API it also is, with `dl` at its release downloads. The URL is
/// known only once the stand-in runs, so config.json is written then.
fn github_index() -> FakeGitHub {
    let fake = FakeGitHub::start("{}");
    let config = format!(
        r#"{{"dl":"{}/dl/{{name}}-{{version}}.nova-pkg","api":"owner/index"}}"#,
        fake.server.url
    );
    fake.state
        .lock()
        .unwrap()
        .files
        .insert("config.json".into(), (config, "sha-config".into()));
    fake
}

/// `nova` as `f.nova` does, but reading `fake`'s index.
fn nova_on(f: &Fixture, fake: &FakeGitHub, cwd: &Path) -> Command {
    let mut command = f.nova(cwd);
    command
        .env("NOVA_INDEX", format!("{}/raw/", fake.server.url))
        .env("NOVA_GITHUB_API", fake.api());
    command
}

#[test]
fn login_checks_the_token_and_stores_it_unprinted() {
    let f = Fixture::new("login");
    let fake = github_index();
    let app = f.app("", "fn main() {}\n");
    let out = nova_on(&f, &fake, &app)
        .arg("login")
        .write_stdin(format!("{}\n", FakeGitHub::TOKEN))
        .assert()
        .success();
    assert!(!stdout(&out).contains(FakeGitHub::TOKEN));
    assert!(!stderr(&out).contains(FakeGitHub::TOKEN));
    let stored = read(&f.home.join("credentials.toml"));
    assert!(stored.contains(FakeGitHub::TOKEN), "{stored}");
    fake.state.lock().unwrap().push = false;
    let out = nova_on(&f, &fake, &app)
        .arg("login")
        .write_stdin(format!("{}\n", FakeGitHub::TOKEN))
        .assert()
        .failure();
    assert!(stderr(&out).contains("cannot push to owner/index"), "{}", stderr(&out));
    let out = nova_on(&f, &fake, &app)
        .arg("login")
        .write_stdin("ghp_wrong\n")
        .assert()
        .failure();
    assert!(!stderr(&out).contains("ghp_wrong"), "{}", stderr(&out));
}

#[test]
fn publish_to_a_github_index_goes_through_the_api() {
    let f = Fixture::new("publish-github");
    let fake = github_index();
    let geom = f.library("geom", "0.1.0", "", AREA);
    let out = nova_on(&f, &fake, &geom).arg("publish").assert().failure();
    assert!(
        stderr(&out).contains("not logged in: run `gh auth token | nova login` first"),
        "{}",
        stderr(&out)
    );
    nova_on(&f, &fake, &geom)
        .arg("login")
        .write_stdin(FakeGitHub::TOKEN)
        .assert()
        .success();
    let out = nova_on(&f, &fake, &geom).arg("publish").assert().success();
    let printed = format!("{}{}", stdout(&out), stderr(&out));
    assert!(printed.contains("up to five minutes"), "{printed}");
    assert!(!printed.contains(FakeGitHub::TOKEN), "{printed}");
    let text = fake.state.lock().unwrap().files["ge/om/geom"].0.clone();
    assert!(text.contains("\"vers\":\"0.1.0\""), "{text}");
    // The token went to the API only.
    for request in fake.server.requests() {
        if request.header("authorization").is_some() {
            assert!(request.path.starts_with("/repos/"), "{request:?}");
        }
    }
    // An app depends on it, reading the index raw and the tarball from
    // the release, with no token.
    let app = f.app("", MAIN_AREA);
    nova_on(&f, &fake, &app).args(["add", "geom"]).assert().success();
    let out = nova_on(&f, &fake, &app).arg("run").assert().success();
    assert_eq!(stdout(&out).trim(), "9");
}
```

```bash
cd /d/Projects/nona/nova && cargo test --locked -p nova-index --test github 2>&1 | grep -E "^error" | head -3; cargo test --locked -p nova-cli --test registry 2>&1 | grep -E "^error" | head -3
```

- [ ] **Step 3: Verify they fail**

Expected:
- `error[E0432]: unresolved imports` naming `load_token`,
  `publish_github`, `store_token`, `ApiReader` and `GitHub`;
- `nova-cli`'s test builds, since it uses only the stand-in;
  `login_checks_the_token_and_stores_it_unprinted` and
  `publish_to_a_github_index_goes_through_the_api` fail:
  `login` is an unknown subcommand.

Run the second command without the `grep` to see them fail, and read
the two failures.

- [ ] **Step 4: Implement**

In `crates/nova-index/src/download.rs`, `write_atomically` becomes the two
functions below and a shared one. The body is today's, with the `private`
branch added:

```rust
/// Write `bytes` to `path` through a temporary file beside it, unique to
/// this process, renamed into place. `path`'s directory is created.
pub(crate) fn write_atomically(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    write_new(path, bytes, false)
}

/// [`write_atomically`], the temporary file created readable by its owner
/// only (mode 0600) on Unix before anything is written to it (spec §6.7).
/// On Windows it takes its directory's permissions.
pub(crate) fn write_private(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    write_new(path, bytes, true)
}

fn write_new(path: &Path, bytes: &[u8], private: bool) -> std::io::Result<()> {
    if let Some(dir) = path.parent() {
        if !dir.as_os_str().is_empty() {
            std::fs::create_dir_all(dir)?;
        }
    }
    let name = path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    let (mut file, temp) = loop {
        let n = TEMP_COUNTER.fetch_add(1, Ordering::Relaxed);
        let temp = path.with_file_name(format!(".{name}.{}-{n}.tmp", std::process::id()));
        let mut options = std::fs::OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            if private {
                options.mode(0o600);
            }
        }
        #[cfg(not(unix))]
        let _ = private;
        match options.open(&temp) {
            Ok(file) => break (file, temp),
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(error) => return Err(error),
        }
    };
    let written = file.write_all(bytes).and_then(|()| file.flush());
    drop(file);
    let result = written.and_then(|()| std::fs::rename(&temp, path));
    if result.is_err() {
        let _ = std::fs::remove_file(&temp);
    }
    result
}
```

`crates/nova-index/src/credentials.rs`:

```rust
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
```

`crates/nova-index/src/github.rs`:

```rust
//! Publishing to an index on GitHub (spec §7, §8): the index's files
//! through the Contents API, and each version's tarball as a release
//! asset. Every request here carries the token, and goes only to
//! api.github.com and uploads.github.com, or to a loopback
//! `NOVA_GITHUB_API` in tests. None follows a redirect.

use base64::Engine;

use crate::http::{shown, Http};
use crate::line::Line;
use crate::location::index_path;
use crate::pack::MAX_TARBALL;
use crate::publish::check_new;
use crate::read::Reader;

const API: &str = "https://api.github.com";
const UPLOADS: &str = "https://uploads.github.com";

/// The GitHub repository behind an index, and a token for it.
pub struct GitHub {
    http: Http,
    token: String,
    api: String,
    uploads: String,
    /// `owner/name`, from the index's config.json `api`.
    pub repo: String,
}

/// What a file write did.
#[derive(Debug, PartialEq, Eq)]
pub enum Written {
    Done,
    /// The file changed since it was read.
    Conflict,
}

/// A release: its id, and its assets as `(id, name)`.
#[derive(Debug)]
pub struct Release {
    pub id: u64,
    pub assets: Vec<(u64, String)>,
}

type Answer = (u16, Vec<u8>);

impl GitHub {
    /// The API for `repo`, with `token`. `api`, for tests, replaces both API
    /// hosts, and may only be `http://127.0.0.1:<port>` or
    /// `http://[::1]:<port>` (spec §8).
    pub fn new(repo: &str, token: &str, api: Option<&str>) -> Result<GitHub, String> {
        let valid = |part: &str| {
            !part.is_empty()
                && part
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || "-_.".contains(c))
        };
        if !repo.split_once('/').is_some_and(|(owner, name)| valid(owner) && valid(name)) {
            return Err(format!(
                "config.json's `api` must be `owner/repository`, not `{repo}`"
            ));
        }
        let (api, uploads) = match api.filter(|a| !a.is_empty()) {
            None => (API.to_string(), UPLOADS.to_string()),
            Some(url) => {
                loopback_only(url)?;
                let base = url.trim_end_matches('/').to_string();
                (base.clone(), base)
            }
        };
        Ok(GitHub {
            http: Http::new(),
            token: token.to_string(),
            api,
            uploads,
            repo: repo.to_string(),
        })
    }

    /// [`GitHub::new`], with `NOVA_GITHUB_API` from the environment.
    pub fn from_env(repo: &str, token: &str) -> Result<GitHub, String> {
        let api = std::env::var("NOVA_GITHUB_API").ok();
        GitHub::new(repo, token, api.as_deref())
    }

    fn url(&self, rest: &str) -> String {
        format!("{}/repos/{}{rest}", self.api, self.repo)
    }

    fn authorized<B>(&self, request: ureq::RequestBuilder<B>) -> ureq::RequestBuilder<B> {
        request
            .header("Authorization", format!("Bearer {}", self.token))
            .header("Accept", "application/vnd.github+json")
            .header("X-GitHub-Api-Version", "2022-11-28")
    }

    fn get(&self, url: &str) -> Result<Answer, String> {
        answer(self.authorized(self.http.agent().get(url)).call(), url)
    }

    fn delete(&self, url: &str) -> Result<Answer, String> {
        answer(self.authorized(self.http.agent().delete(url)).call(), url)
    }

    fn post(&self, url: &str, content_type: &str, body: &[u8]) -> Result<Answer, String> {
        let request = self
            .authorized(self.http.agent().post(url))
            .header("Content-Type", content_type);
        answer(request.send(body), url)
    }

    fn put(&self, url: &str, body: &[u8]) -> Result<Answer, String> {
        let request = self
            .authorized(self.http.agent().put(url))
            .header("Content-Type", "application/json");
        answer(request.send(body), url)
    }

    /// Whether the token can push to the repository (spec §6.7).
    pub fn can_push(&self) -> Result<bool, String> {
        let (status, body) = self.get(&self.url(""))?;
        match status {
            200 => Ok(json(&body)?["permissions"]["push"].as_bool() == Some(true)),
            401 => Err("GitHub refused the token".to_string()),
            404 => Err(format!(
                "the repository {} does not exist, or the token cannot see it",
                self.repo
            )),
            _ => Err(unexpected(status, &self.repo)),
        }
    }

    /// The file at `path` on the repository's default branch, and its blob
    /// sha; `None` when there is none.
    pub fn read_file(&self, path: &str) -> Result<Option<(String, String)>, String> {
        let (status, body) = self.get(&self.url(&format!("/contents/{}", encode_path(path))))?;
        match status {
            200 => {
                let value = json(&body)?;
                let content: String = value["content"]
                    .as_str()
                    .unwrap_or("")
                    .chars()
                    .filter(|c| !c.is_whitespace())
                    .collect();
                let bytes = base64::engine::general_purpose::STANDARD
                    .decode(content)
                    .map_err(|e| format!("GitHub sent {path} in bad base64: {e}"))?;
                let text = String::from_utf8(bytes)
                    .map_err(|_| format!("{path} in {} is not UTF-8", self.repo))?;
                let sha = value["sha"].as_str().unwrap_or("").to_string();
                Ok(Some((text, sha)))
            }
            404 => Ok(None),
            401 => Err("GitHub refused the token".to_string()),
            _ => Err(unexpected(status, &self.repo)),
        }
    }

    /// Write `text` at `path`, replacing the blob `sha` read with it, or
    /// creating the file when `sha` is `None`.
    pub fn write_file(
        &self,
        path: &str,
        text: &str,
        sha: Option<&str>,
        message: &str,
    ) -> Result<Written, String> {
        let mut body = serde_json::json!({
            "message": message,
            "content": base64::engine::general_purpose::STANDARD.encode(text),
        });
        if let Some(sha) = sha {
            body["sha"] = serde_json::Value::from(sha);
        }
        let url = self.url(&format!("/contents/{}", encode_path(path)));
        let (status, _) = self.put(&url, body.to_string().as_bytes())?;
        match status {
            200 | 201 => Ok(Written::Done),
            409 => Ok(Written::Conflict),
            401 => Err("GitHub refused the token".to_string()),
            _ => Err(unexpected(status, &self.repo)),
        }
    }

    /// The release tagged `tag`, made when there is none.
    pub fn release(&self, tag: &str) -> Result<Release, String> {
        let (status, body) = self.get(&self.url(&format!("/releases/tags/{}", encode(tag))))?;
        match status {
            200 => return parse_release(&body),
            404 => {}
            401 => return Err("GitHub refused the token".to_string()),
            _ => return Err(unexpected(status, &self.repo)),
        }
        let request = serde_json::json!({"tag_name": tag, "name": tag});
        let (status, body) = self.post(
            &self.url("/releases"),
            "application/json",
            request.to_string().as_bytes(),
        )?;
        match status {
            201 => parse_release(&body),
            _ => Err(unexpected(status, &self.repo)),
        }
    }

    pub fn delete_asset(&self, id: u64) -> Result<(), String> {
        let (status, _) = self.delete(&self.url(&format!("/releases/assets/{id}")))?;
        match status {
            204 | 404 => Ok(()),
            _ => Err(unexpected(status, &self.repo)),
        }
    }

    pub fn upload_asset(&self, release: u64, name: &str, bytes: &[u8]) -> Result<(), String> {
        let url = format!(
            "{}/repos/{}/releases/{release}/assets?name={}",
            self.uploads,
            self.repo,
            encode(name)
        );
        let (status, _) = self.post(&url, "application/gzip", bytes)?;
        match status {
            201 => Ok(()),
            _ => Err(unexpected(status, &self.repo)),
        }
    }
}

/// `NOVA_GITHUB_API` may only be a loopback address with a port (spec §8).
fn loopback_only(url: &str) -> Result<(), String> {
    let lower = url.to_ascii_lowercase();
    let port = lower
        .strip_prefix("http://127.0.0.1:")
        .or_else(|| lower.strip_prefix("http://[::1]:"))
        .map(|rest| rest.trim_end_matches('/'));
    match port {
        Some(port) if port.parse::<u16>().is_ok() => Ok(()),
        _ => Err(format!(
            "NOVA_GITHUB_API may only be http://127.0.0.1:<port> or http://[::1]:<port>, not \
             {url}"
        )),
    }
}

/// A response's status and body. An error names the URL, never a header.
fn answer(
    result: Result<ureq::http::Response<ureq::Body>, ureq::Error>,
    url: &str,
) -> Result<Answer, String> {
    let mut response = result.map_err(|e| format!("cannot reach {}: {e}", shown(url)))?;
    let status = response.status().as_u16();
    let body = response
        .body_mut()
        .with_config()
        .limit(MAX_TARBALL)
        .read_to_vec()
        .map_err(|e| format!("cannot read {}: {e}", shown(url)))?;
    Ok((status, body))
}

fn json(body: &[u8]) -> Result<serde_json::Value, String> {
    serde_json::from_slice(body).map_err(|e| format!("GitHub's answer does not parse: {e}"))
}

fn unexpected(status: u16, repo: &str) -> String {
    format!("GitHub answered HTTP {status} for {repo}")
}

fn parse_release(body: &[u8]) -> Result<Release, String> {
    let value = json(body)?;
    let id = value["id"]
        .as_u64()
        .ok_or("GitHub's release has no id")?;
    let assets = value["assets"]
        .as_array()
        .map(|assets| {
            assets
                .iter()
                .filter_map(|a| Some((a["id"].as_u64()?, a["name"].as_str()?.to_string())))
                .collect()
        })
        .unwrap_or_default();
    Ok(Release { id, assets })
}

/// `text` percent-encoded for one path segment or query value.
fn encode(text: &str) -> String {
    text.bytes()
        .map(|b| {
            if b.is_ascii_alphanumeric() || b"-._~".contains(&b) {
                (b as char).to_string()
            } else {
                format!("%{b:02X}")
            }
        })
        .collect()
}

fn encode_path(path: &str) -> String {
    path.split('/').map(encode).collect::<Vec<_>>().join("/")
}

/// `text`, an index file, with `line` appended.
fn appended(text: &str, line: &Line) -> String {
    let mut text = text.to_string();
    if !text.is_empty() && !text.ends_with('\n') {
        text.push('\n');
    }
    text.push_str(&line.to_json());
    text.push('\n');
    text
}

/// Publish `line` and `tarball` to `github` (spec §7):
/// 1. read the package's index file, and refuse a version already there
///    or a name in another case;
/// 2. find or make the release `<name>-<version>`;
/// 3. upload the tarball as its asset, deleting an earlier attempt's;
/// 4. append the line with the file's `sha`, the commit point; if the file
///    changed in between, read it again, check again, and try once more.
pub fn publish_github(github: &GitHub, line: &Line, tarball: &[u8]) -> Result<(), String> {
    let file = index_path(&line.name);
    let tag = format!("{}-{}", line.name, line.vers);
    let asset = format!("{tag}.nova-pkg");
    let (text, sha) = github.read_file(&file)?.map_or((String::new(), None), |(t, s)| (t, Some(s)));
    check_new(&text, &file, line)?;
    let release = github.release(&tag)?;
    for (id, name) in &release.assets {
        if *name == asset {
            github.delete_asset(*id)?;
        }
    }
    github.upload_asset(release.id, &asset, tarball)?;
    let message = format!("Publish {} {}", line.name, line.vers);
    if github.write_file(&file, &appended(&text, line), sha.as_deref(), &message)?
        == Written::Done
    {
        return Ok(());
    }
    let (text, sha) = github.read_file(&file)?.map_or((String::new(), None), |(t, s)| (t, Some(s)));
    check_new(&text, &file, line)?;
    match github.write_file(&file, &appended(&text, line), sha.as_deref(), &message)? {
        Written::Done => Ok(()),
        Written::Conflict => Err(format!(
            "{file} changed twice while {} {} was being published; run `nova publish` again",
            line.name, line.vers
        )),
    }
}

/// An index's files read through the GitHub API, which is never stale, as
/// publishing's verification needs (spec §3.3, §6.6).
pub struct ApiReader<'a> {
    pub github: &'a GitHub,
}

impl Reader for ApiReader<'_> {
    fn file(&mut self, path: &str) -> Result<Option<String>, String> {
        Ok(self.github.read_file(path)?.map(|(text, _)| text))
    }
}
```

In `crates/nova-index/src/lib.rs`, add `mod credentials;` and
`mod github;`, and:

```rust
pub use credentials::{credentials_path, load_token, store_token};
pub use github::{publish_github, ApiReader, GitHub, Release, Written};
```

`crates/nova-cli/src/cmd/login.rs`:

```rust
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
```

`IsTerminal` needs Rust 1.70, which the 1.78 floor covers.

In `crates/nova-cli/src/cmd/package.rs`:
1. the `use nova_index::…;` line gains `load_token`, `publish_github`,
   `ApiReader` and `GitHub`;
2. `publish`'s `Source::Http(_)` arm becomes:

```rust
        Source::Http(_) => {
            let config = nova_index::reader_for(&index)
                .config()
                .map_err(|e| anyhow!("cannot read the index at {}: {e}", index.canonical))?;
            let Some(repo) = config.api else {
                bail!(
                    "the index at {} has no `api` in its config.json, so nova cannot publish \
                     to it",
                    index.canonical
                );
            };
            let home = nova_pm::nova_home_from_env()
                .context("cannot read the credentials: set NOVA_HOME")?;
            let token = load_token(&home)
                .map_err(|e| anyhow!(e))?
                .context("not logged in: run `gh auth token | nova login` first")?;
            let github = GitHub::from_env(&repo, &token).map_err(|e| anyhow!(e))?;
            // The verification reads the index through the API, which is
            // never stale (spec §6.6).
            let mut api = ApiReader { github: &github };
            let prepared = prepare(&index, &mut api)?;
            report(&prepared);
            publish_github(&github, &prepared.line(), &prepared.packed.bytes)
                .map_err(|e| anyhow!(e))?;
            let package = &prepared.manifest.package;
            println!(
                "published {} {} to {repo}; other machines may take up to five minutes to see it",
                package.name, package.version
            );
            Ok(())
        }
```

In `crates/nova-cli/src/cmd/mod.rs`, add `pub mod login;` after
`pub mod fmt;`. In `crates/nova-cli/src/main.rs`:
1. after `Update(…)`:

   ```rust
       /// Store a GitHub token for `nova publish`, read from standard input:
       /// `gh auth token | nova login`.
       Login,
   ```

2. in the `match`, `Command::Login => cmd::login::login(),`;
3. the doc comment's subcommand list gains "login".

- [ ] **Step 5: Run the tests to verify they pass**

```bash
cd /d/Projects/nona/nova && P=/c/Users/SAKEER~1/AppData/Local/Temp/gcm/p34 && (cargo test --locked -p nova-index && cargo test --locked -p nova-cli --test registry) > $P/t12.txt 2>&1; echo "exit=$?"; python -X utf8 $P/count.py $P/t12.txt; grep -E "FAILED|panicked|^warning|^error" $P/t12.txt | head
```

Expected:
- `exit=0`, 0 failed, no warnings;
- 9 tests in `github.rs`;
- 22 in `registry.rs`.

Then the credentials test's Unix branch, on Linux:

```bash
cd /d/Projects/nona/nova && git add -A crates/nova-index crates/nova-cli && bash /c/Users/SAKEER~1/AppData/Local/Temp/gcm/linux/run.sh test --locked -p nova-index --test github 2>&1 | tail -3
```

Expected: `test result: ok. 9 passed; 0 failed`.

- [ ] **Step 6: Commit**

Write `$P/msg-12.txt`:

```
Publishing to GitHub, and nova login

`nova publish` to an index whose config.json names a GitHub repository
(spec
docs/superpowers/specs/2026-10-09-phase-3-3b-index-and-publishing-design.md
§7) takes four steps:
1. it reads the package's index file through the Contents API, and
   stops on a version already there or a name in another case;
2. it finds or makes the release <name>-<version>;
3. it uploads the tarball as its asset, deleting an earlier attempt's;
4. it appends the line with the file's sha. That is the commit point:
   if the file changed meanwhile, it reads, checks and tries once more.

Its verification reads the index through the API too, so a dependency
published a moment ago is seen. Afterwards it says other machines may
take up to five minutes to see the new version.

`nova login` reads a token from a pipe, never a terminal. It checks
that the token can push to the index's repository, and stores it in
$NOVA_HOME/credentials.toml. The file is owner-only on Unix; on Windows,
nova warns when $NOVA_HOME is outside the user's profile. The token is
never printed.

The token goes only to the API hosts, or to a loopback NOVA_GITHUB_API
in tests. No request carrying it follows a redirect, and no error names
it (§8). The tests run against a stand-in GitHub that records every
request.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
```

```bash
cd /d/Projects/nona/nova && P=/c/Users/SAKEER~1/AppData/Local/Temp/gcm/p34 && cargo fmt --all && git add Cargo.toml Cargo.lock crates/nova-index crates/nova-cli && git commit -q -F $P/msg-12.txt && git log -1 --format=%s
```

Expected: `Publishing to GitHub, and nova login`.

The task's test command: `bash -c "cargo test --locked -p nova-index && cargo test --locked -p nova-cli --test registry"`.

---

### Task 13: The language server and `nova.lock`

Spec §5.4, §5.5, §10.6.

**Files:**
- Modify: `crates/nova-lsp/src/lib.rs` (the watcher and its handler), `crates/nova-lsp/src/checker.rs` (`check`)
- Modify: `crates/nova-cli/tests/lsp_client/mod.rs` (`start_with_env`), `crates/nova-cli/tests/lsp.rs`

**Interfaces:**
- Consumes:
  - Task 1's `registry_dir` and `index_dir_name`;
  - Task 4's offline graph (M0005), and Task 5's dropping of a
    dependency's warnings, both reached through `analyze_program`.
- Produces: `lsp_client::Client::start_with_env(&Path, bool, &[(&str, &Path)]) -> Client`.
  `Client::start` calls it with no variables.

The server never syncs. Two things change:
- it watches `**/nova.lock` too, and a change to one re-checks every open
  project, as a `nova.toml` change does;
- a project whose directory, or a loose file, is under the registry
  directory gets no check and no publish.

Three of the five tests are *guards*. M0005 on the manifest entry is Task
4's, and the hidden warning is Task 5's; these tests pin both through the
server.

- [ ] **Step 1: Write the failing tests**

In `crates/nova-cli/tests/lsp_client/mod.rs`, `start` becomes the two
functions below. The body of `start_with_env` is today's `start`, with the
`Command` built first so the variables can be set:

```rust
    /// Start `nova lsp` and initialize it. With `watch`, the client offers
    /// dynamic registration of watched files, as VS Code does.
    pub fn start(root: &Path, watch: bool) -> Client {
        Client::start_with_env(root, watch, &[])
    }

    /// [`Client::start`], with each of `env` set for the server, such as
    /// `NOVA_HOME`.
    pub fn start_with_env(root: &Path, watch: bool, env: &[(&str, &Path)]) -> Client {
        let mut command = Command::new(assert_cmd::cargo::cargo_bin("nova"));
        command
            .arg("lsp")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit());
        for (name, value) in env {
            command.env(name, value);
        }
        let mut child = command.spawn().expect("start nova lsp");
        let stdin = child.stdin.take();
        let stdout = child.stdout.take().expect("piped stdout");
        let (tx, rx) = mpsc::channel();
        std::thread::spawn(move || {
            let mut reader = BufReader::new(stdout);
            while let Some(message) = read_message(&mut reader) {
                if tx.send(message).is_err() {
                    break;
                }
            }
        });
        let mut client = Client {
            child,
            stdin,
            messages: rx,
            next_id: 1,
            unread: Vec::new(),
        };
        let capabilities = if watch {
            json!({ "workspace": { "didChangeWatchedFiles": { "dynamicRegistration": true } } })
        } else {
            json!({})
        };
        client.request(
            "initialize",
            json!({ "processId": null, "rootUri": file_uri(root), "capabilities": capabilities }),
        );
        client.notify("initialized", json!({}));
        client
    }
```

Append to `crates/nova-cli/tests/lsp.rs`:

```rust
// === Phase 3.3b: registry packages (spec
// docs/superpowers/specs/2026-10-09-phase-3-3b-index-and-publishing-design.md
// §5.5, §10.6) ===

const INDEX: &str = "https://example.test/index/";

/// `dir/app`, which depends on `geom = "<req>"` and runs `APP_MAIN`, and
/// `dir/home`, the server's `NOVA_HOME`.
fn registry_app(name: &str, req: &str) -> (std::path::PathBuf, std::path::PathBuf) {
    let dir = fresh_dir(name);
    let app = dir.join("app");
    std::fs::create_dir_all(app.join("src")).unwrap();
    std::fs::write(
        app.join("nova.toml"),
        format!(
            "{}\n[dependencies]\ngeom = \"{req}\"\n",
            MANIFEST.replace("demo", "app")
        ),
    )
    .unwrap();
    std::fs::write(app.join("src").join("main.nova"), APP_MAIN).unwrap();
    (app, dir.join("home"))
}

/// `geom` 0.1.0, locked in `app`'s nova.lock and unpacked under `home`,
/// with `lib` as its lib.nova. Its directory.
fn lock_and_cache(
    app: &std::path::Path,
    home: &std::path::Path,
    lib: &str,
) -> std::path::PathBuf {
    let geom = home
        .join("registry")
        .join("src")
        .join(nova_pm::index_dir_name(INDEX))
        .join("geom-0.1.0");
    std::fs::create_dir_all(geom.join("src")).unwrap();
    std::fs::write(geom.join("nova.toml"), MANIFEST.replace("demo", "geom")).unwrap();
    std::fs::write(geom.join("src").join("lib.nova"), lib).unwrap();
    std::fs::write(
        app.join("nova.lock"),
        format!(
            "version = 1\nindex = \"{INDEX}\"\n\n[[package]]\nname = \"geom\"\n\
             version = \"0.1.0\"\nchecksum = \"00\"\ndependencies = []\n"
        ),
    )
    .unwrap();
    geom
}

#[test]
fn an_entry_not_downloaded_is_m0005_on_its_manifest_entry() {
    // A guard: Task 4's graph, through the server.
    let (app, home) = registry_app("registry-unfetched", "0.1");
    let mut client = Client::start_with_env(&app, false, &[("NOVA_HOME", &home)]);
    open(
        &mut client,
        &file_uri(&app.join("src").join("main.nova")),
        APP_MAIN,
    );
    let params = client.diagnostics(&file_uri(&app.join("nova.toml")), nonempty);
    assert_eq!(codes(&params), ["M0005"], "{params}");
    let message = params["diagnostics"][0]["message"].as_str().unwrap();
    assert!(message.contains("run `nova fetch`"), "{message}");
}

#[test]
fn a_locked_version_that_no_longer_fits_is_m0005() {
    // A guard, as above.
    let (app, home) = registry_app("registry-no-fit", "0.2");
    lock_and_cache(&app, &home, GEOMETRY_FIXED);
    let mut client = Client::start_with_env(&app, false, &[("NOVA_HOME", &home)]);
    open(
        &mut client,
        &file_uri(&app.join("src").join("main.nova")),
        APP_MAIN,
    );
    let params = client.diagnostics(&file_uri(&app.join("nova.toml")), nonempty);
    assert_eq!(codes(&params), ["M0005"], "{params}");
    let message = params["diagnostics"][0]["message"].as_str().unwrap();
    assert!(
        message.contains("is locked at 0.1.0, which does not meet `^0.2`"),
        "{message}"
    );
}

#[test]
fn a_changed_lock_rechecks_the_project() {
    let (app, home) = registry_app("registry-relock", "0.1");
    let toml = file_uri(&app.join("nova.toml"));
    let mut client = Client::start_with_env(&app, true, &[("NOVA_HOME", &home)]);
    open(
        &mut client,
        &file_uri(&app.join("src").join("main.nova")),
        APP_MAIN,
    );
    client.diagnostics(&toml, nonempty);
    let registration = client.wait_for(|m| m["method"] == "client/registerCapability");
    let globs: Vec<String> = registration["params"]["registrations"][0]["registerOptions"]
        ["watchers"]
        .as_array()
        .unwrap()
        .iter()
        .map(|w| w["globPattern"].as_str().unwrap().to_string())
        .collect();
    assert!(globs.iter().any(|g| g == "**/nova.lock"), "{globs:?}");
    // What `nova fetch` leaves: the package unpacked, then the lock.
    lock_and_cache(&app, &home, GEOMETRY_FIXED);
    client.notify(
        "workspace/didChangeWatchedFiles",
        json!({ "changes": [{ "uri": file_uri(&app.join("nova.lock")), "type": 1 }] }),
    );
    client.diagnostics(&toml, |p| !nonempty(p));
}

#[test]
fn a_file_in_the_cache_gets_nothing_published() {
    let (app, home) = registry_app("registry-cached-file", "0.1");
    let geom = lock_and_cache(&app, &home, GEOMETRY_BROKEN);
    let mut client = Client::start_with_env(&app, false, &[("NOVA_HOME", &home)]);
    client.clear_unread();
    let lib = file_uri(&geom.join("src").join("lib.nova"));
    open(&mut client, &lib, GEOMETRY_BROKEN);
    let sentinel = sentinel(&mut client, "registry-cached-file");
    assert_eq!(client.last_diagnostics_before(&lib, &sentinel), None);
}

#[test]
fn a_dependencys_warning_is_not_shown() {
    // A guard: Task 5's dropping, through the server.
    let (app, home) = registry_app("registry-warning", "0.1");
    lock_and_cache(
        &app,
        &home,
        "pub fn area() -> Int {\n    match 1 { _ => 1, 0 => 2 }\n}\n",
    );
    let mut client = Client::start_with_env(&app, false, &[("NOVA_HOME", &home)]);
    let main = file_uri(&app.join("src").join("main.nova"));
    open(&mut client, &main, APP_MAIN);
    let params = client.diagnostics(&main, |_| true);
    assert!(!nonempty(&params), "{params}");
    let params = client.diagnostics(&file_uri(&app.join("nova.toml")), |_| true);
    assert!(!nonempty(&params), "{params}");
}
```

```bash
cd /d/Projects/nona/nova && P=/c/Users/SAKEER~1/AppData/Local/Temp/gcm/p34 && cargo test --locked -p nova-cli --test lsp registry > $P/t13-red.txt 2>&1; python -X utf8 $P/count.py $P/t13-red.txt; grep -E "^test .* (ok|FAILED)$" $P/t13-red.txt
```

- [ ] **Step 2: Verify they fail**

Expected:
- `a_changed_lock_rechecks_the_project` FAILED. There is no `**/nova.lock`
  in the registration;
- `a_file_in_the_cache_gets_nothing_published` FAILED. The cached file's
  E0010 is published;
- the three guards ok.

- [ ] **Step 3: Implement**

In `crates/nova-lsp/src/lib.rs`:
1. `register_watcher`'s doc comment becomes `/// Ask the client to watch
   \`.nova\` files, \`nova.toml\` and \`nova.lock\` (spec §6.1; 3.3b §5.5).`,
   and its list becomes:

   ```rust
           let watchers = ["**/*.nova", "**/nova.toml", "**/nova.lock"]
   ```

2. In the `DidChangeWatchedFiles` arm, the `let manifest = …;` line
   becomes:

   ```rust
                       // A lock changes when `nova fetch` has unpacked
                       // something (spec 3.3b §4.6, §5.5).
                       let manifest = paths
                           .iter()
                           .any(|p| p.ends_with("nova.toml") || p.ends_with("nova.lock"));
   ```

In `crates/nova-lsp/src/checker.rs`:
1. at the start of `check`'s body, before `let mut out = Vec::new();`:

```rust
    // A downloaded package belongs to no project: nothing is checked or
    // published for its files (spec 3.3b §5.5).
    if in_the_cache(&job.project) {
        return Some((Vec::new(), Vec::new()));
    }
```

2. after `check`:

```rust
/// Whether `project` is inside nova's cache of downloaded packages.
fn in_the_cache(project: &ProjectKey) -> bool {
    let Some(registry) = nova_pm::registry_dir() else {
        return false;
    };
    let path = match project {
        ProjectKey::Root(dir) => dir.as_path(),
        ProjectKey::Loose(file) => file.as_path(),
    };
    PathKey::of(path).is_under(&PathKey::of(&registry))
}
```

`check`'s doc comment gains a line: "- a project in nova's cache of
downloaded packages is not checked (spec 3.3b §5.5)."

- [ ] **Step 4: Run the tests to verify they pass**

Check that port 3000 is free first. Then:

```bash
cd /d/Projects/nona/nova && P=/c/Users/SAKEER~1/AppData/Local/Temp/gcm/p34 && cargo test --locked -p nova-lsp -p nova-cli --test lsp > $P/t13.txt 2>&1; echo "exit=$?"; python -X utf8 $P/count.py $P/t13.txt; grep -E "^test .* FAILED$|panicked|^warning|^error" $P/t13.txt | head
```

Expected:
- `exit=0`, 0 failed, no warnings;
- the 5 new tests pass;
- every earlier `lsp.rs` test passes too.

If cargo refuses `--test lsp` for `nova-lsp`, which has no test target,
run `cargo test --locked -p nova-cli --test lsp` alone. `nova-lsp` has no
tests of its own to miss. Ledger it.

- [ ] **Step 5: Commit**

Write `$P/msg-13.txt`:

```
nova-lsp: re-check on nova.lock, and leave cached packages alone

The language server never syncs (spec
docs/superpowers/specs/2026-10-09-phase-3-3b-index-and-publishing-design.md
§5.5). An entry the cache cannot satisfy is M0005 on the manifest entry,
and a dependency's warnings are not shown; both come from the driver.

- It now watches **/nova.lock too. A change to one re-checks every open
  project, as a nova.toml change does. That is why a sync rewrites the
  lock after unpacking anything: `nova fetch` in a terminal clears the
  editor's M0005.
- A file in nova's cache of downloaded packages belongs to no project,
  so nothing is checked or published for it. Opening one, as
  go-to-definition will in 3.4, shows it without diagnostics.

The test client gains `start_with_env`, so a test gives the server its
own NOVA_HOME.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
```

```bash
cd /d/Projects/nona/nova && P=/c/Users/SAKEER~1/AppData/Local/Temp/gcm/p34 && cargo fmt --all && git add crates/nova-lsp crates/nova-cli && git commit -q -F $P/msg-13.txt && git log -1 --format=%s
```

Expected: `nova-lsp: re-check on nova.lock, and leave cached packages alone`.

The task's test command: `cargo test --locked -p nova-cli --test lsp`.

---

### Task 14: The 3.3b gate in CI

Spec §11.1.

**Files:**
- Create: `.github/scripts/registry-gate.sh` (LF endings, which `.gitattributes` already gives `.github/scripts/*.sh`)
- Modify: `.github/workflows/ci.yml` (the `install` job)

**Interfaces:**
- Consumes the installed `nova`'s commands: `new`, `publish`, `add`,
  `run`, `build` and `test`.
- Produces `registry-gate.sh NOVA WORKDIR`. It exits 0 and prints
  `registry-gate: passed`.

- [ ] **Step 1: Write the gate**

`.github/scripts/registry-gate.sh`:

```bash
#!/usr/bin/env bash
# The Phase 3.3b gate (spec
# docs/superpowers/specs/2026-10-09-phase-3-3b-index-and-publishing-design.md
# §11.1): with the installed nova and a local index, a library is
# published, and a program depends on it from the index, runs, builds and
# tests; then, with the index gone, it builds again from nova.lock and the
# cache alone. NOVA_INDEX and NOVA_HOME are set in this script's process
# only. CI's `install` job runs it after packages-gate.sh, on all three
# systems.
#
# Usage: registry-gate.sh NOVA WORKDIR
#   NOVA     the nova executable to check
#   WORKDIR  where to make the index and the packages; it must not exist yet
set -euo pipefail

if [ $# -ne 2 ]; then
  echo "usage: registry-gate.sh NOVA WORKDIR" >&2
  exit 2
fi
nova=$1
work=$2

mkdir "$work"
cd "$work"
# Absolute, since step 5 renames the index from inside app/.
work=$(pwd)
# nova on Windows needs a Windows path: Git Bash's `pwd -W` gives C:/...
if root=$(pwd -W 2>/dev/null); then :; else root=$(pwd); fi

# 1. A local index whose tarballs are under it.
mkdir index
printf '%s\n' '{"dl":"dl/{name}-{version}.nova-pkg"}' > index/config.json
export NOVA_INDEX="$root/index"
export NOVA_HOME="$root/home"

# 2. A library, published to it.
"$nova" new --lib geom
(cd geom && "$nova" publish) | tee publish.txt
if ! grep -q 'published geom 0.1.0' publish.txt; then
  echo "registry-gate: nova publish did not say it published geom 0.1.0" >&2
  exit 1
fi

# 3. A program that depends on it from the index.
"$nova" new app
cd app
"$nova" add geom
cat > src/main.nova <<'EOF'
import geom

fn main() {
    println("from geom: ${greeting()}")
}

@test
fn greeting_comes_from_geom() {
    assert_eq(greeting(), "Hello, Nova!")
}
EOF

# 4. It runs and builds, and the tests of both packages pass.
expected='from geom: Hello, Nova!'
ran=$("$nova" run)
if [ "$ran" != "$expected" ]; then
  echo "registry-gate: nova run printed '$ran'" >&2
  exit 1
fi
"$nova" build
built=$(./target/debug/app)
if [ "$built" != "$expected" ]; then
  echo "registry-gate: the program nova build wrote printed '$built'" >&2
  exit 1
fi
"$nova" test | tee test.txt
if ! grep -q '1 passed; 0 failed' test.txt; then
  echo "registry-gate: nova test did not pass the app's test" >&2
  exit 1
fi
(cd ../geom && "$nova" test) | tee ../geom-test.txt
if ! grep -q '2 passed; 0 failed' ../geom-test.txt; then
  echo "registry-gate: nova test did not pass geom's two tests" >&2
  exit 1
fi

# 5. With the index gone, nova.lock and the cache are enough.
mv "$work/index" "$work/index-gone"
rm -rf target
"$nova" build
built=$(./target/debug/app)
if [ "$built" != "$expected" ]; then
  echo "registry-gate: without the index, the program printed '$built'" >&2
  exit 1
fi
echo "registry-gate: passed"
```

- [ ] **Step 2: Run it against a debug nova here**

```bash
cd /d/Projects/nona/nova && P=/c/Users/SAKEER~1/AppData/Local/Temp/gcm/p34 && git ls-files --eol .github/scripts/registry-gate.sh; python -X utf8 -c "import sys; d=open(sys.argv[1],'rb').read(); print('CR' if b'\r' in d else 'LF only')" .github/scripts/registry-gate.sh; cargo build --locked -p nova-cli 2>&1 | tail -1; rm -rf $P/registry-gate; bash .github/scripts/registry-gate.sh "$(pwd)/target/debug/nova.exe" $P/registry-gate 2>&1 | tail -5
```

Expected:
- `LF only`;
- the build finishes;
- the gate's last line is `registry-gate: passed`.

`git ls-files` shows nothing until Step 4 stages the file.

Before you run it, check that the gate's `NOVA_HOME` is under `$P`:
nothing may be written to the user's own `~/.nova`.

- [ ] **Step 3: Add it to CI**

In `.github/workflows/ci.yml`'s `install` job, after the step "The
installed nova builds a program against a path library":

```yaml
      # The Phase 3.3b gate (spec
      # docs/superpowers/specs/2026-10-09-phase-3-3b-index-and-publishing-design.md
      # §11.1): a local index only; CI never writes the real one.
      - name: The installed nova publishes to a local index and builds from it
        shell: bash
        run: bash .github/scripts/registry-gate.sh "$RUNNER_TEMP/nova/bin/nova" "$RUNNER_TEMP/registry"
```

- [ ] **Step 4: Commit**

Write `$P/msg-14.txt`:

```
CI: the Phase 3.3b gate

registry-gate.sh (spec
docs/superpowers/specs/2026-10-09-phase-3-3b-index-and-publishing-design.md
§11.1) works with the installed nova and a local index, in the install
job on all three systems:
- it publishes a library to the index;
- a program adds it from the index, runs, builds, and passes its tests,
  as the library does;
- with the index renamed away, the program builds again from nova.lock
  and the cache alone.

NOVA_INDEX and NOVA_HOME are set in the script's own process. CI writes
only this local index, never the real one.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
```

```bash
cd /d/Projects/nona/nova && P=/c/Users/SAKEER~1/AppData/Local/Temp/gcm/p34 && git add .github/scripts/registry-gate.sh .github/workflows/ci.yml && git ls-files --eol .github/scripts/registry-gate.sh && git commit -q -F $P/msg-14.txt && git log -1 --format=%s
```

Expected:
- `i/lf    w/lf    attr/text eol=lf`;
- `CI: the Phase 3.3b gate`.

The task's test command: `bash -c 'rm -rf /c/Users/SAKEER~1/AppData/Local/Temp/gcm/p34/registry-gate-done && bash .github/scripts/registry-gate.sh "$(pwd)/target/debug/nova.exe" /c/Users/SAKEER~1/AppData/Local/Temp/gcm/p34/registry-gate-done'`.

---

### Task 15: The records

Spec §12. ADR 0031, dated notes, the project documents, and the
set-difference sweep.

**Files:**
- Create: `docs/adr/0031-package-index-and-publishing.md`
- Modify (dated notes):
  - `nova-spec/40-TOOLING.md` §1.1, §4.1–§4.6 and §10.1;
  - `docs/phase-3-plan.md`, its 3.3 entry and §5;
  - `docs/adr/0026-phase-3-scope.md`;
  - `agent.md`'s crate table.
- Modify: `CHANGELOG.md`, `README.md`, `ARCHITECTURE.md`
- Modify: whatever Step 5's sweep finds

**Interfaces:**
- Consumes: every task's behaviour, and the ledger's rulings.
- Produces: nothing code depends on.

- [ ] **Step 1: ADR 0031**

`docs/adr/0031-package-index-and-publishing.md`:

```markdown
# ADR 0031: The package index and publishing

## Status

Accepted, 2026-10-09 (Phase 3.3b, branch `phase-3-3b-index-publishing`;
spec `docs/superpowers/specs/2026-10-09-phase-3-3b-index-and-publishing-design.md`).

## Context

ADR 0026 made the registry a git-backed index: a public GitHub repository
holding `40-TOOLING.md` §4.4's sparse index, with tarballs as release
assets, written through GitHub's REST API and read over HTTPS. ADR 0030
gave Nova packages with path dependencies. Phase 3.3b adds what a version
entry needs: an index to resolve against, a lock, a cache, and the
commands that publish.

## Decision

1. **The index** is Cargo's sparse layout, in lower case: `ge/om/geom`
   holds `geom`'s versions, one JSON line each, `{"name", "vers", "deps",
   "cksum", "v"}`. `v` is the line format's version; a reader skips a
   line whose `v` it does not know, so the format can grow. A line that
   does not parse is skipped with a note. `config.json` at the root gives
   `dl`, where tarballs are, and `api`, the repository publishing writes.
   `NOVA_INDEX` names an index: `https://`, `http://` on a loopback IP for
   tests, a `file:` URL or an absolute path. The default is
   `Sakeerin/nova-index` on raw.githubusercontent.com.
2. **One canonical form per index.** URLs and paths are compared as one
   form, which keys `nova.lock` and the cache. A mirror is another index:
   transparent mirrors wait.
3. **One version of each name per build,** with Cargo's caret
   requirements, and pre-releases only when a requirement names one.
   Several major versions side by side would need each package's imports
   to name a version, which waits for its own ADR. The resolver is a
   backtracking search in a fixed order. It keeps every locked version
   that still fits, so editing one entry moves as few versions as
   possible.
4. **`nova.lock`** records the index and each registry package's version,
   SHA-256 and dependencies. It is written last by every command that
   syncs, so it never names a package that is not unpacked.
5. **The graph stays offline.** `nova run`, `build`, `check` and `test`
   sync first: they resolve when the lock does not answer the manifests,
   and download what the cache lacks. The driver and the language server
   never touch the network. They read the lock and
   `$NOVA_HOME/registry/src/<idx>/<name>-<version>/`. An entry the cache
   cannot satisfy is M0005; M0014 to M0017 are the index's errors. A
   dependency's warnings are not shown.
6. **A downloaded package is not trusted:**
   - its SHA-256 must match the lock's before it is unpacked;
   - its size and entry count are bounded;
   - every entry must be a file or a directory inside the package, with
     a name every system can hold;
   - its manifest must match its index line, with no path dependency.
7. **Publishing** packs the library reproducibly, verifies the tarball,
   and only then writes. The verification unpacks it, resolves it afresh,
   and checks it with no error and no warning of its own. To GitHub, it:
   - reads the index file through the Contents API;
   - creates the release `<name>-<version>`;
   - uploads the tarball, replacing an earlier attempt's asset;
   - appends the line with the file's `sha`, which is the commit point.

   The index is append-only. A name keeps the case it was first published
   with.
8. **The token** is stored by `nova login` from a pipe, owner-only on
   Unix. It is sent only to api.github.com and uploads.github.com, never
   on a redirect, and never printed. On Windows the file takes its
   directory's permissions: the user's profile by default, as Cargo's
   credentials do.
9. **The network crate is `ureq`, pinned at 3.2.1,** with rustls on the
   `ring` the runtime already uses. It needs nothing installed beside
   `nova`, and builds on Rust 1.78; ureq 3.3.0 and later need 1.85. The
   crate holding all of this is `nova-index`, because `nova-registry` is
   taken on crates.io.

## Alternatives

- **Resolving in the driver**, so every build could touch the network.
  Refused: the language server would hang on a slow index, and a build
  with no network should work from the lock.
- **Shelling out to `git` or `curl`.** Refused: `nova` would need them
  installed, and their versions and proxies would vary.
- **Several major versions of one package in a build.** Waits: imports
  would have to name a version.
- **Reading the raw files while publishing.** Refused: raw.githubusercontent.com
  caches for up to five minutes, so a dependency published a moment ago
  would be missing. Publishing reads through the API.

## Consequences

- **Freshness.** Another machine may not see a new version for up to five
  minutes; `nova publish` says so.
- **GitHub's limits.** A publish takes about six API requests, against
  5,000 an hour for an authenticated user. Each version adds about 200
  bytes to the index repository; tarballs are release assets, which do
  not count towards its size. The by-hand publish (spec §11.2) measures
  these, and its figures are added here when it runs.
- **Yanking** is not in 3.3; a published version stays.
- **A stale editor.** The language server sees a fetch only through the
  lock's change, which is why a sync rewrites the lock after unpacking.
- **The cache is never cleaned:** `nova clean` is out of Phase 3 (ADR
  0026).

## References

- `nova-spec/40-TOOLING.md` §1.1, §4 and §10.1, and their dated notes.
- `docs/adr/0026-phase-3-scope.md` and `docs/adr/0030-package-modules.md`.
- `docs/phase-3-plan.md` §3 decisions 4 and 5, and §5.
```

- [ ] **Step 2: Dated notes**

Each note is its own paragraph, inserted after the line named, with a
blank line before it.

In `nova-spec/40-TOOLING.md`:
- after the §1.1 note that ends
  `` (`docs/superpowers/specs/2026-10-07-phase-3-1-formatter-design.md` §7). ``:

  ```markdown
  **Amended 2026-10-09 (branch `phase-3-3b-index-publishing`):** `nova
  add <pkg>[@<req>]`, `nova update [pkg]`, `nova publish` and `nova login`
  exist, with two commands this list lacks. `nova fetch` downloads what
  `nova.lock` names, and `nova package` packs and verifies a library
  (`docs/superpowers/specs/2026-10-09-phase-3-3b-index-and-publishing-design.md`
  §6; ADR 0031).
  ```
- after the §4.1 note that ends `with each \`-\` replaced by \`_\` (ADR 0030).`:

  ```markdown
  **Amended 2026-10-09 (branch `phase-3-3b-index-publishing`):** a version
  entry resolves against the package index. M0005 now means an entry the
  cache cannot satisfy: not downloaded, locked at a version that no longer
  fits, or `NOVA_HOME` not found. M0014–M0017 are the index's errors: a
  package the index lacks, a conflict no version meets, an unreadable
  `nova.lock`, and a path dependency in a package to publish (ADR 0031).
  ```
- after §4.2's last bullet, `- Committed for binaries, optional for libraries`:

  ```markdown
  **Amended 2026-10-09 (branch `phase-3-3b-index-publishing`):** as built,
  `nova.lock` holds `version = 1`, the index's canonical form, and one
  `[[package]]` per registry package: its name, version, SHA-256 and
  dependencies' names. A path package is in the tree, so it is not
  locked. Every command that syncs writes it last, with `\n` endings
  (ADR 0031).
  ```
- after §4.3's last bullet, `- Edition compatibility rules`:

  ```markdown
  **Amended 2026-10-09 (branch `phase-3-3b-index-publishing`):** one
  version of each name per build, with Cargo's caret requirements. Locked
  versions that still fit are kept. Edition rules are out of Phase 3 (ADR
  0026) (ADR 0031).
  ```
- after §4.4's last bullet, `` - `cargo`-like sparse index format ``:

  ```markdown
  **Amended 2026-10-09 (branch `phase-3-3b-index-publishing`):** the index
  is a GitHub repository, `Sakeerin/nova-index`, read over HTTPS, with
  tarballs as release assets (ADR 0026). There is no server to run, and
  `registry.novalang.dev` does not exist. `NOVA_INDEX` names another
  index, a local directory included (ADR 0031).
  ```
- after the §4.5 note that ends `3.3b's.`:

  ```markdown
  **Amended 2026-10-09 (branch `phase-3-3b-index-publishing`):** as built,
  `nova add <pkg>` writes the version it resolved, and `nova add
  <pkg>@<req>` the requirement given. `nova update <pkg>` lets only that
  package move. `nova login` reads a token from a pipe. `nova owner` is
  out of Phase 3 (ADR 0026) (ADR 0031).
  ```
- after §4.6's last bullet, `- Verification: must compile with no warnings`:

  ```markdown
  **Amended 2026-10-09 (branch `phase-3-3b-index-publishing`):** a
  `.nova-pkg` holds `nova.toml`, `src/`, `tests/` and the top-level
  `README*` and `LICENSE*`, under `<name>-<version>/`. It leaves out
  `target/`, `nova.lock` and every name starting with `.`, and is
  reproducible to the byte. Only a library is published, with no path in
  `[dependencies]`. The verification unpacks the tarball, resolves it
  afresh, and checks it as `nova check` does: any error, or any warning
  of its own, stops it (ADR 0031).
  ```
- after the §10.1 note that ends `installs a \`nova\` that needs nothing beside it (ADR 0027).`:

  ```markdown
  **Amended 2026-10-09 (branch `phase-3-3b-index-publishing`):** `~/.nova`
  also holds `registry/`, the downloaded packages, and `credentials.toml`,
  the token `nova login` stores (ADR 0031).
  ```

In `docs/phase-3-plan.md`:
- after the 3.3 entry's note that ends `has the rest of this entry, and its two-part gate.`:

  ```markdown
  **Amended 2026-10-09:** 3.3b, "The index and publishing", is built
  (`docs/superpowers/specs/2026-10-09-phase-3-3b-index-and-publishing-design.md`,
  branch `phase-3-3b-index-publishing`; ADR 0031). Its CI gate passes on
  three systems. The by-hand publish to the real index waits for the
  user's word.
  ```
- after §5's bullet that ends `a published version is never overwritten.`:

  ```markdown
  **Amended 2026-10-09 (branch `phase-3-3b-index-publishing`):** on
  Windows the token's file takes its directory's permissions, the user's
  profile by default, so "readable only by its owner" holds there only as
  far as the profile's do. `nova login` warns when `NOVA_HOME` is outside
  the profile (ADR 0031).
  ```

In `docs/adr/0026-phase-3-scope.md`, after its Consequences bullet that
ends `which 3.3 measures.`:

```markdown
**Amended 2026-10-09 (branch `phase-3-3b-index-publishing`):** decision 1
is built as ADR 0031 records. Yanking is not in 3.3: a published version
stays, and its line never changes. The measurements wait for the by-hand
publish.
```

In `agent.md`, after the crate table's note that ends
`(\`docs/adr/0029-the-language-server.md\`).`:

```markdown
**Amended 2026-10-09 (branch `phase-3-3b-index-publishing`):** `nova-index`
is the package index's client, built on `ureq` 3.2.1, pinned
(`docs/adr/0031-package-index-and-publishing.md`). `nova-cli` depends on
it; the driver and `nova-lsp` do not.
```

- [ ] **Step 3: The project documents**

`ARCHITECTURE.md`'s crate table:
- the `nova-pm` row becomes `` | `nova-pm` | Package manager: `nova.toml`, the package graph, `nova.lock` and the resolver; it never touches the network (ADR 0030, ADR 0031) | ``;
- after it, add `` | `nova-index` | The package index: reading, downloading, packing, the sync step and publishing (ADR 0031) | ``.

`CHANGELOG.md`, `[Unreleased]`, at the end of `### Added`, after
`  own diagnostics.`:

```markdown
- **The package index** (ADR 0031).
  - `json = "1.4"` in `[dependencies]` resolves against the index that
    `NOVA_INDEX` names, by default `Sakeerin/nova-index` on GitHub. One
    version of each name per build, recorded in `nova.lock`.
  - `nova run`, `build`, `check` and `test` download what the lock names
    into `~/.nova/registry/`, checked against its SHA-256. With the lock
    and the cache complete, they need no network.
  - M0014–M0017 report a package the index lacks, a conflict, an
    unreadable lock, and a path dependency in a package to publish.
- **`nova add <name>[@<req>]`** adds a dependency from the index, writing
  the version it resolved. **`nova update [<name>]`** and **`nova fetch`**
  move and download locked versions.
- **`nova package`** packs a library reproducibly and verifies it.
  **`nova publish`** publishes it, to a local index or to GitHub. **`nova
  login`** stores the GitHub token it needs, read from a pipe.
- **The language server** re-checks when `nova.lock` changes, and leaves
  downloaded packages alone.
```

`CHANGELOG.md`, `[Unreleased]`, at the end of `### Changed`, after
`  so, and a package with neither target is M0013.`:

```markdown
- A version-only dependency is no longer M0005 "a registry dependency":
  it resolves against the index, and M0005 means an entry the cache
  cannot satisfy, with `nova fetch` to fix it.
- `nova run`, `build`, `check` and `test` sync a package's dependencies
  first, and a sync's errors stop them before compiling.
- A warning in a downloaded dependency is not shown.
```

`README.md`'s `### Packages` section: its last sentence, `Registry
dependencies and \`nova publish\` come in a later release.`, becomes:

```markdown
`nova add json` adds a package from the index instead, and `nova.lock`
records the version. `nova update` moves it, and `nova fetch` downloads
what the lock names. To publish a library, `gh auth token | nova login`
once, then `nova publish`.
```

- [ ] **Step 4: Commit the records**

Write `$P/msg-15a.txt`:

```
docs: record Phase 3.3b, the index and publishing

- ADR 0031, "The package index and publishing", which records:
  - the index and its line format, one canonical form per index, and
    one version per name;
  - the lock, written last, and the offline graph;
  - what a download must pass, publishing and its verification, the
    token's rules, and the pinned ureq.
- Dated notes: 40-TOOLING §1.1, §4.1–§4.6 and §10.1; the phase plan's
  3.3 entry and §5; ADR 0026; agent.md's crate table.
- ARCHITECTURE's crate table gains nova-index. CHANGELOG [Unreleased]
  has three changes in behaviour, and the README's packages section now
  covers the index.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
```

```bash
cd /d/Projects/nona/nova && P=/c/Users/SAKEER~1/AppData/Local/Temp/gcm/p34 && git add docs/adr/0031-package-index-and-publishing.md docs/adr/0026-phase-3-scope.md nova-spec/40-TOOLING.md docs/phase-3-plan.md agent.md ARCHITECTURE.md CHANGELOG.md README.md && git commit -q -F $P/msg-15a.txt && git log -1 --format=%s && git show --stat HEAD | tail -1
```

Expected: `docs: record Phase 3.3b, the index and publishing`, with
`8 files changed`.

- [ ] **Step 5: The set-difference sweep**

A grep proves only what it was pointed at. List every living document,
and every Rust doc comment, that names what this branch changed, minus
the files the branch touched:

```bash
cd /d/Projects/nona/nova && P=/c/Users/SAKEER~1/AppData/Local/Temp/gcm/p34 && git diff --name-only 002739a HEAD > $P/touched.txt && { for t in "M0005" "registry" "Registry" "nova.lock" "lock file" "lockfile" "nova publish" "nova login" "nova update" "nova fetch" "nova package" "nova add" "NOVA_HOME" "NOVA_INDEX" ".nova-pkg" "credentials" "a later release" "path dependenc" "~/.nova"; do git grep -l -F -- "$t" -- '*.md' '*.yml' '*.toml' '*.sh' '*.json' | sed "s|^|$t: |"; done; for t in "M0005" "registry dependenc" "arrive with the package index" "cache_root" "nova_home" "check_program(" "for_package(" "graph_from("; do git grep -l -F -- "$t" -- '*.rs' | sed "s|^|$t: |"; done; } | grep -v -E ": (docs/superpowers/|docs/adr/00[0-2]|tools/vscode-nova/node_modules/|Cargo.lock)" | while IFS= read -r line; do f=${line#*: }; grep -q -x -F -- "$f" $P/touched.txt || echo "$line"; done | sort | tee $P/sweep.txt | wc -l
```

`docs/adr/00[0-2]*` is left out because ADRs before 0030 are history.
ADR 0026 and ADR 0030 are touched or named in the notes above.

Read every hit in `$P/sweep.txt`, in context. For each, decide whether it
now says something untrue or incomplete about any of these:
- version dependencies, M0005, `nova.lock`, the index or the cache;
- `NOVA_HOME`'s contents;
- the commands `nova add`, `update`, `fetch`, `package`, `publish` and
  `login`;
- which commands touch the network.

If it does, add a dated note there, or fix a non-document such as a
script or a doc comment, and add the file to the commit below. If not,
nothing changes.

Ledger each file as `Task 15: sweep <file> -> <note added | unaffected:
why>`. A file listed for a word it uses in another sense is unaffected:
"registry" for a Windows registry, `nova add --path` used as before, or
the lockfile meaning `Cargo.lock`.

If the sweep changed anything, write `$P/msg-15b.txt`:

```
docs: notes the index sweep found

The set-difference sweep over the living documents (spec
docs/superpowers/specs/2026-10-09-phase-3-3b-index-and-publishing-design.md
§12):
<one line per file changed, saying what changed>

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
```

and commit the files it lists with `git commit -q -F $P/msg-15b.txt`, then
check `git log -1 --format=%s`.

The task's test command: `cargo test --locked -p nova-cli --test project`.
The records touch no code, but the sweep may touch a doc comment. This
confirms the CLI still builds and its project tests pass.

---

### Task 16: Final verification

Before the PR, run here everything CI runs, on the finished branch, plus
the gate by name.

**Files:**
- Create: `$P/pr-body.md` (outside the repository)

**Interfaces:**
- Consumes: the whole branch.
- Produces: the figures for the PR body.

- [ ] **Step 1: The full suite on Windows**

```bash
cd /d/Projects/nona/nova && P=/c/Users/SAKEER~1/AppData/Local/Temp/gcm/p34 && git status --short | wc -l && netstat -ano | grep -E "[:.]3000 .*LISTENING"; cargo build --locked -p nova-runtime 2>&1 | tail -1 && cargo test --locked --workspace --all-features --no-fail-fast > $P/suite-final.txt 2>&1; echo "exit=$?"; python -X utf8 $P/count.py $P/suite-final.txt
```

Expected:
- `0` uncommitted files;
- nothing from `netstat`. If it prints anything, stop and ask the user,
  as the Conventions say;
- `exit=0`, 0 failed, 9 ignored;
- passed: `main`'s (`002739a`) 1566 plus this plan's 128 new tests, so
  1694.

The 128 on Windows, by task:

| Task | New tests |
|---|---|
| 1 | 8 |
| 2 | 4 |
| 3 | 13 |
| 4 | 13 |
| 5 | 5 |
| 6 | 9 |
| 7 | 10 in `pack.rs`, 1 in `names.rs` |
| 8 | 9 |
| 9 | 16 |
| 10 | 4 in `publish.rs`, 10 in `registry.rs` |
| 11 | 10 |
| 12 | 9 in `github.rs`, 2 in `registry.rs` |
| 13 | 5 |

The renamed test of Task 11 is not new. If the figure differs, recount
the new tests by name from each task's output, and ledger the difference.

- [ ] **Step 2: What CI's other jobs run**

```bash
cd /d/Projects/nona/nova && P=/c/Users/SAKEER~1/AppData/Local/Temp/gcm/p34 && cargo fmt --all -- --check; echo "fmt exit=$?"; cargo clippy --locked --all-targets --all-features -- -D warnings > $P/clippy.txt 2>&1; echo "clippy exit=$?"; tail -3 $P/clippy.txt
```

Expected: `fmt exit=0` and `clippy exit=0`.
- A clippy finding is fixed in the code, never allowed by attribute.
- The fix is its own commit, `<crate>: what clippy found`.
- Step 1 then runs again.

```bash
cd /d/Projects/nona/nova && P=/c/Users/SAKEER~1/AppData/Local/Temp/gcm/p34 && RUSTUP_TOOLCHAIN=1.78.0 RUSTFLAGS="-D warnings" cargo check --locked --workspace --target-dir target/msrv > $P/msrv.txt 2>&1; echo "msrv exit=$?"; tail -3 $P/msrv.txt; git diff 002739a HEAD -- Cargo.lock | grep -E '^[+-]name = ' | sort | tee $P/lock-added.txt
```

Expected:
- `msrv exit=0`;
- every line starts with `+`. No package left the lockfile;
- the names are `nova-index` and exactly the packages Task 7 and Task 8
  ledgered. Compare them by name with the ledger's lists, and ledger the
  comparison.

- [ ] **Step 3: The full suite on Linux**

```bash
cd /d/Projects/nona/nova && P=/c/Users/SAKEER~1/AppData/Local/Temp/gcm/p34 && docker version 2>&1 | grep -c "^Server:"; bash /c/Users/SAKEER~1/AppData/Local/Temp/gcm/linux/run.sh test --locked --workspace --all-features --no-fail-fast > $P/suite-linux-final.txt 2>&1; echo "exit=$?"; python -X utf8 $P/count.py $P/suite-linux-final.txt
```

Expected:
- `1` (Docker's server is up), and `exit=0`;
- 0 failed, 10 ignored;
- passed: `main`'s 1562 plus 129, so 1691.

Linux has two tests Windows lacks, the Unix-only ones of Task 7, and
lacks Task 1's drive-letter test. It is also where the credentials file's
mode is checked, and where a symbolic link is really refused.

- [ ] **Step 4: The gate, item by item**

Spec §11.1, each part with what shows it:

```bash
cd /d/Projects/nona/nova && P=/c/Users/SAKEER~1/AppData/Local/Temp/gcm/p34 && W=$(mktemp -d "$P/gate.XXXX") && cargo build --locked -p nova-cli -p nova-runtime 2>&1 | tail -1 && bash .github/scripts/registry-gate.sh "$PWD/target/debug/nova.exe" "$W/work" > $P/gate-final.txt 2>&1; echo "exit=$?"; tail -1 $P/gate-final.txt; grep -E "an_app_runs_builds_and_tests_with_a_published_library|fetch_fills_the_cache_and_a_build_needs_no_index|publish_writes_the_tarball_where_dl_says_and_appends_a_line" $P/suite-final.txt
```

Expected: `exit=0`, `registry-gate: passed`, and three lines ending in
`ok`. Ledger them:
- **a library published to a local index:** the gate script, and
  `publish_writes_the_tarball_where_dl_says_and_appends_a_line`;
- **a program adds it, runs, builds and tests:** the gate script, and
  `an_app_runs_builds_and_tests_with_a_published_library`;
- **with the index gone, the lock and the cache are enough:** the gate
  script, and `fetch_fills_the_cache_and_a_build_needs_no_index`;
- **CI's three systems:** the `install` job's new step, read from the
  PR's CI.

The by-hand half (spec §11.2) is the user's, on the user's word. This
plan never runs it.

- [ ] **Step 5: The mutants**

Spec §10.7: each mutant must fail at least one named test. Run each only
on committed work.
1. Make the edit.
2. Run its test command and read the named test's line: it must say
   `FAILED`. A mutant that aborts the build reads as nothing failed, so
   check the name.
3. Restore the file with `git checkout -- <file>`, and confirm
   `git status --short` is empty before the next one.

In the table, `\|` stands for `|`.

| # | Rule | File | Edit | Test that must fail |
|---|---|---|---|---|
| 1 | the checksum check | `crates/nova-index/src/download.rs` | in `fetch_package`, `if actual != locked.checksum {` → `if false {` | `cargo test --locked -p nova-index --test http` → `a_tarball_whose_checksum_is_wrong_is_refused_and_discarded` |
| 2 | locked versions kept | `crates/nova-pm/src/resolve.rs` | in `Search::solve`, `Some(Some(i)) => vec![fitting[i].clone()],` → `Some(Some(_)) => fitting.clone(),` | `cargo test --locked -p nova-pm --test resolve` → `a_tightened_requirement_moves_only_its_package` |
| 3 | the index is append-only | `crates/nova-index/src/publish.rs` | in `check_new`, `if lines.iter().any(\|l\| l.vers == line.vers) {` → `if false {` | `cargo test --locked -p nova-index --test publish` → `a_version_already_there_is_refused_and_nothing_written` |
| 4 | path refusal, packing | `crates/nova-cli/src/cmd/package.rs` | in `prepare`, `.filter(\|d\| d.path.is_some())` → `.filter(\|_\| false)` | `cargo test --locked -p nova-cli --test registry` → `a_path_dependency_is_m0017_and_a_path_dev_dependency_is_allowed` |
| 5 | path refusal, unpacking | `crates/nova-index/src/cache.rs` | in `check_manifest`, `.find(\|d\| d.path.is_some())` → `.find(\|_\| false)` | `cargo test --locked -p nova-index --test pack` → `the_manifest_must_be_the_index_lines` |
| 6 | no `..` when unpacking | `crates/nova-index/src/cache.rs` | in `extract`, `.find(\|part\| !nova_pm::is_portable(part))` → `.find(\|part\| !nova_pm::is_portable(part) && **part != "..")` | `cargo test --locked -p nova-index --test pack` → `unpacking_refuses_what_could_escape_or_merge` |
| 7 | no token with a download | `crates/nova-index/src/http.rs` | in `Http::get`, `.get(&url)` → `.get(&url).header("Authorization", "Bearer ghp_test")` | `cargo test --locked -p nova-index --test http` → `requests_name_nova_and_carry_no_credentials` |
| 8 | the lock written last | `crates/nova-index/src/sync.rs` | in `sync`, after `let idx = nova_pm::index_dir_name(&index.canonical);` insert `if write { write_lock(root, &lock).map_err(SyncError::Other)?; }` | `cargo test --locked -p nova-index --test sync` → `a_failed_download_writes_no_lock` |

Mutant 6 may write `evil` into the test's own temporary directory, which
the next run replaces; nothing outside it.

Ledger each as `Task 16: mutant <n> -> <test> FAILED (restored)`.

- [ ] **Step 6: Draft the PR body**

Write `$P/pr-body.md` with the Write tool, filling the `<…>` from the
ledger and from Steps 1 to 4:

```markdown
## Phase 3.3b, "The index and publishing"

A Nova package can depend on a published package. `nova add json`
resolves against a git-backed index, `nova.lock` pins the version, and
`nova run`, `build`, `check` and `test` download what it names into a
checksum-verified cache. `nova package`, `nova login` and `nova publish`
publish a library, to a local index or to GitHub. The language server
never touches the network.

- Spec: `docs/superpowers/specs/2026-10-09-phase-3-3b-index-and-publishing-design.md`
- Plan: `docs/superpowers/plans/2026-10-09-phase-3-3b-index-and-publishing.md`
- ADR 0031, "The package index and publishing"

### What changed

- **`nova-pm`.**
  - `$NOVA_HOME`, an index's canonical form, and its cache directory.
  - `nova.lock`.
  - The resolver: caret ranges, one version per name, backtracking, and
    locked versions kept.
  - The offline graph, which finds registry packages through the lock
    and the cache. M0005 has a new meaning; M0014–M0017 are new.
- **`nova-index`, a new crate.**
  - The index's format, and readers for a directory, HTTPS and the GitHub
    API.
  - Reproducible packing, and unpacking that trusts nothing.
  - HTTP through `ureq` 3.2.1, pinned, with rustls on the `ring` already
    locked; downloads checked against the lock's SHA-256.
  - The sync step, which writes the lock last.
  - Publishing to a local index or to GitHub, and the stored token.
- **`nova-driver`** counts warnings, hides a dependency's, and takes an
  explicit registry directory.
- **`nova-cli`.**
  - `run`, `build`, `check` and `test` sync first.
  - `nova fetch`, `nova update [name]`, `nova package`, `nova publish`
    and `nova login`.
  - `nova add` from the index.
- **`nova-lsp`** re-checks when `nova.lock` changes, and publishes nothing
  for a downloaded package.
- **CI.** `.github/scripts/registry-gate.sh`, in the `install` job, and
  `.cargo/config.toml`'s MSRV-aware resolver setting.
- **Records.** ADR 0031, and dated notes in:
  - 40-TOOLING;
  - the phase plan;
  - ADR 0026;
  - agent.md.

  Also the CHANGELOG, the README and ARCHITECTURE.

### Changes in behaviour

- A version-only dependency resolves against the index, and M0005 means
  an entry the cache cannot satisfy.
- `nova run`, `build`, `check` and `test` sync first, so a project with
  registry dependencies may touch the network. With a complete lock and
  cache, it does not.
- A warning in a downloaded dependency is not shown.

### New dependencies

`Cargo.lock` gains `nova-index` and:

<the ledger's lists from Tasks 7 and 8, each package with its version,
its `rust_version` and its edition>

Each builds on Rust 1.78: the MSRV check passes on Windows here and in CI
on Linux. The packages only Windows or macOS builds are:
<the ledger's list, or "None">.

### The gate (spec §11)

- **In CI:** `registry-gate.sh` passes locally (<result>), and in CI's
  `install` job on all three systems.
  `an_app_runs_builds_and_tests_with_a_published_library` and
  `fetch_fills_the_cache_and_a_build_needs_no_index` cover the same ground
  in the suite.
- **By hand (spec §11.2): not run.** Publishing to the real
  `Sakeerin/nova-index` waits for the user's word. Its transcript and
  figures go into ADR 0031 when it runs.

### Tests

| | Windows (local) | Linux (container) |
|---|---|---|
| `002739a` | 1566 passed, 9 ignored | 1562 passed, 10 ignored |
| this branch | <Step 1's figures> | <Step 3's figures> |

No test reaches the internet: every test that syncs uses a local index or
a loopback server, and its own `NOVA_HOME`. Each mutant of spec §10.7
fails its named test. Clippy, rustfmt and the MSRV check (Rust 1.78) pass.

### Decisions to review

The plan's "Decisions: where this plan settles what the spec leaves open",
items 1 to 24, and the rulings made while executing it:

<every `Ruling:` line from the ledger, in order>

### Deferred minors

<every `minor (deferred)` line from the ledger, or "None.">

🤖 Generated with [Claude Code](https://claude.com/claude-code)
```

Scrub it as for 3.2: no local paths, process ids or user names.

The task's test command:
`cargo test --locked --workspace --all-features --no-fail-fast`.

---

## After the last task

These follow the standing workflow, not this plan:

1. **The final review.** A fresh reviewer, on the most capable model,
   reads the whole branch, from `git merge-base main HEAD` to `HEAD`. It
   gets the spec, this plan, its Review Focus section verbatim, and the
   ledger's `Ruling:` lines. It is read-only, and runs no cargo, no
   scripts and no network.
2. **The fix pass.** Each Critical or Important finding gets a test that
   fails first, then the fix, then a green suite. Minor findings are
   deferred to the PR body.
3. **Push and open the PR** against `main`, with `$P/pr-body.md` as its
   body. Then read the PR's CI once, when the user returns; never poll
   it.
4. **Merge only on the user's word,** by rebase, then verify that `main`'s
   tree is the PR head's.
5. **The by-hand publish (spec §11.2)** is a separate step, on the user's
   word. Creating `Sakeerin/nova-index`, logging in, and publishing to it
   are all the user's to start.
