# Phase 3.0, "Foundations", Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** A `nova` installed with `cargo install` builds and tests programs
with nothing beside it, knows what a project is, and every `v*` tag
publishes a GitHub release with per-target archives.

**Architecture:**
- **std** becomes the crate `nova-std`. Its `lib.rs` embeds `std/*/lib.nova`
  through paths inside its own package, and `nova-resolver` takes its
  sources from it.
- **The runtime library** is built by a nested `cargo build --release` in
  `nova-cli`'s build script: in place inside a workspace, or from a
  lockfile-free copy when packaged. It is gzip-compressed and embedded with
  `include_bytes!`. `nova-driver` unpacks it to
  `$NOVA_HOME/runtime/<version>-<crc32>/` the first time it links. It comes
  after `NOVA_RUNTIME_LIB` and before the executable's neighbours.
- **Projects:**
  - `nova-pm` parses `nova.toml` with toml_edit's parser, which keeps
    positions, and finds the nearest manifest;
  - `nova-cli` uses that for `run`, `build`, `check` and `test`, and gains
    `new`, `init` and `version`.
- **Releases and CI:**
  - `release.yml` packs, smoke-tests and publishes archives;
  - a new `ci.yml` job installs `nova` and runs the gate script on three
    systems.

**Tech Stack:**
- Rust: MSRV 1.78, edition 2021.
- Cargo build scripts.
- New crates: `flate2` and `crc32fast` for the payload; `toml_edit` 0.22 and
  `semver` 1 in `nova-pm`.
- clap 4.
- `assert_cmd` end-to-end tests.
- GitHub Actions, with bash on all three runners.
- Docker (`rust:1-slim`) for local Linux runs.

**Spec:** `docs/superpowers/specs/2026-10-07-phase-3-0-foundations-design.md`
(commits `5288dd1`, `3509072`, `4557176`), approved by the user on
2026-10-07. Read it before Task 1: it is the authority this plan argues
from. Its §13 lists the 18 decisions made while writing it.

## Global Constraints

- **MSRV 1.78, edition 2021.** CI's `cargo build`, `test`, `clippy` and
  `check` calls pass `--locked`, and so does every cargo call in this plan
  after the one that updates `Cargo.lock` (Tasks 1, 2, 3, 5 and 6 say
  which).
- **New dependencies, and no others** (spec §13 item 7):
  - `flate2` 1.1, which brings `miniz_oxide` 0.9, `adler2` and
    `simd-adler32`;
  - `semver` 1.

  `crc32fast` and `toml_edit` 0.22 are already in `Cargo.lock`. All four
  are declared once, in `[workspace.dependencies]`. `nova-pm` drops
  `serde`, `toml`, `anyhow` and `tracing`, which it never used, so `toml`
  leaves the lockfile.
- **Names and values, verbatim from the spec:**
  - **`NOVA_EMBED_RUNTIME`** (§4.1): `1` embeds; `0` does not; unset follows
    `PROFILE`, where `release` embeds; any other value fails the build and
    names the variable.
  - **The cache** (§4.2):
    - its path is `$NOVA_HOME/runtime/<version>-<crc32>/<library name>`;
    - `NOVA_HOME` defaults to `.nova` under `USERPROFILE` on Windows and
      under `HOME` elsewhere;
    - `<crc32>` is eight lowercase hex digits;
    - an empty variable counts as unset.
  - **The lookup order** (§4.2): `NOVA_RUNTIME_LIB`, then the embedded
    runtime, then the executable's directory and the two above it. A
    runtime that is embedded but fails to unpack is an error.
  - **`nova version`** (§4.3) prints exactly three lines: `nova <version>`,
    `target: <triple>`, and `runtime: embedded` or `runtime: not embedded`.
  - **The M codes** (§5.2):
    - M0001: not valid TOML;
    - M0002: a required table or key is missing;
    - M0003: a value has the wrong type or breaks its rule;
    - M0004: a dependency has neither `version` nor `path`;
    - M0005: a dependency is declared. The CLI raises it, not the parser;
    - M0006 (a warning): an unknown key or table.
  - **The name rule** (§5.1):
    - ASCII letters, digits, `-` and `_`;
    - starts with a letter;
    - at most 64 characters;
    - not `con`, `prn`, `aux`, `nul`, `com1`–`com9` or `lpt1`–`lpt9`, in
      any case.
  - **A version** (§5.1) is semver without build metadata.
  - **A dependency entry** (§5.1) is a requirement string, or a table with
    exactly one of `version` and `path`.
  - **The template** (§6.3), byte for byte, with `\n` line endings.
  - **Project mode** (§6):
    - with no file argument, the nearest `nova.toml` at or above the
      current directory makes a project;
    - a file argument means file mode, exactly as today, and no manifest
      is read;
    - outputs are `target/debug/<name>` and `target/release/<name>`, each
      with the executable suffix;
    - paths are relative at the root and absolute elsewhere;
    - `nova test`'s binary stays in the system temp directory.
- **Nothing commits a `nova.toml`.** Tests write one only inside their own
  fresh directory under the system temp directory. Discovery walks up, so a
  stray manifest in a shared directory would capture every test run below
  it. None sits above the checkout or the temp directory today (checked on
  2026-10-07).
- **Debug builds never embed** unless `NOVA_EMBED_RUNTIME=1`. CI's test,
  clippy and MSRV jobs stay debug.
- **Records** (spec §10):
  - ADR bodies stay unchanged, apart from the two new ADRs.
  - Specs and documents get dated notes that open with
    `**Amended 2026-10-07 (branch `phase-3-0-foundations`):**`.
  - The dated plans and specs under `docs/superpowers/` stay as they are.

  If you execute on a later day, change that date in every note you write.
- **Stops, from the standing workflow:**
  - Pushing the branch and opening the PR follow the standing workflow.
  - A merge happens only on the user's word, by rebase, verified by tree
    identity.
  - Nothing is published, and no tag is pushed.

## Review Focus

The five inputs most likely to bite a user that the spec's own tests leave
uncovered, most likely first. Each has its test in the task that owns the
code:

1. **A project under a path with spaces and Thai letters,** such as
   `C:\Users\...\โปรเจกต์ ของ ฉัน`, must run, build, link and test. The
   linker, the test binary's directory and the diagnostics all take that
   path → `a_project_under_a_path_with_spaces_and_thai_letters_works`
   (Task 6).
2. **A `nova.toml` saved with CRLF line endings or a UTF-8 byte-order mark**
   must parse, with its positions still right. Windows editors write CRLF,
   and PowerShell 5.1's `Set-Content -Encoding utf8` writes the mark →
   `a_manifest_with_crlf_line_endings_parses_and_points_right` and
   `a_manifest_saved_with_a_byte_order_mark_parses` (Task 5).
3. **A project whose `src/main.nova` imports a sibling module, run from a
   subdirectory,** must resolve the import. The entry path is absolute
   there, and modules resolve beside it →
   `an_import_resolves_when_run_from_a_subdirectory` (Task 6).
4. **A cache directory under a path with spaces and Thai letters,** such as
   a Windows profile `C:\Users\สมชาย ใจดี`, must unpack and link →
   `a_cache_path_with_spaces_and_thai_letters_works` (Task 3).
5. **A path-like name given to `nova new`,** such as `../escape` or `a/b`,
   must be refused before anything touches the disk → the name cases in
   `each_broken_rule_is_m0003` (Task 5), and
   `new_refuses_a_non_empty_directory_and_bad_names` (Task 7).

## File Structure

| File | Task | Responsibility |
|---|---|---|
| `crates/nova-runtime/Cargo.toml`, `build.rs` | 1 | `links = "nova_runtime"`; report the source directory |
| `crates/nova-cli/build_support.rs` (new) | 1 | The build script's decisions, using only `std` |
| `crates/nova-cli/build.rs` (new) | 1 | The nested runtime build, the payload, the exported variables |
| `crates/nova-cli/tests/build_support.rs` (new) | 1 | Tests of `build_support.rs` |
| `Cargo.toml` (workspace) | 1, 2, 5 | `[workspace.dependencies]`; `members` gains `"std"` |
| `Cargo.lock` | 1, 2, 5 | The new crates and `nova-std`; `toml` leaves |
| `std/Cargo.toml`, `std/lib.rs` (new) | 2 | The `nova-std` crate and its embedding tests |
| `crates/nova-resolver/Cargo.toml`, `src/lib.rs:1603-1645` | 2 | std's sources from `nova-std` |
| `crates/nova-driver/src/runtime_cache.rs` (new) | 3 | The embedded runtime, the cache, `locate` |
| `crates/nova-driver/src/link.rs` | 3 | `find_runtime_lib` through `locate`; `beside_exe` |
| `crates/nova-driver/src/lib.rs`, `Cargo.toml` | 3 | The module, re-exports, dependencies |
| `crates/nova-cli/src/embedded.rs` (new) | 4 | The payload and its numbers |
| `crates/nova-cli/src/cmd/version.rs` (new) | 4 | `nova version` |
| `crates/nova-cli/src/main.rs`, `src/cmd/mod.rs` | 4, 6, 7 | Subcommands; registering the payload |
| `crates/nova-cli/tests/project.rs` (new) | 4, 6, 7 | End-to-end tests of the project model |
| `crates/nova-pm/Cargo.toml`, `src/{lib,manifest,name,project}.rs`, `tests/{manifest,find_root}.rs` | 5 | Parsing `nova.toml`; names; discovery |
| `crates/nova-cli/src/project.rs` (new), `src/cmd/run.rs`, `src/cmd/test.rs`, `Cargo.toml` | 6 | Project mode |
| `crates/nova-cli/src/cmd/new.rs`, `src/template.rs` (new) | 7 | `nova new`, `nova init`, the template |
| `.gitattributes` (new), `.github/scripts/gate.sh` (new), `.github/workflows/ci.yml` | 8 | LF scripts, the gate script, the `install` job |
| `.github/scripts/release-notes.sh` (new), `.github/workflows/release.yml` | 9 | Releases |
| `docs/adr/0026-phase-3-scope.md`, `docs/adr/0027-runtime-and-std-in-an-installed-nova.md` (new) | 10 | The two ADRs |
| Records (Task 11 lists them) | 11 | Notes, CHANGELOG, README, ARCHITECTURE, the plan, the sweep |

## Conventions for every task

- **Working directory.** `D:\Projects\nona\nova`, which is Git Bash
  `/d/Projects/nona/nova`. The Bash tool resets its directory after each
  call, so write `cd /d/Projects/nona/nova && …` in one command.
- **Scratch directory.** `P=/c/Users/SAKEER~1/AppData/Local/Temp/gcm/p30`,
  outside the repository. Long output goes to a file there; read its tail.
- **Line endings.** The working tree is CRLF (`core.autocrlf=true`, from
  Git for Windows' system config). The Edit tool is fine. From Task 8 on,
  `.gitattributes` keeps `*.sh` files LF everywhere.
- **Write scripts and commit messages with the Write tool, never a Bash
  heredoc.** The Bash tool turns `\\` into `\`. Commit with
  `git commit -F $P/msg-<n>.txt`, then check `git log -1 --format=%s`.
- **Format before every commit:** run `cargo fmt --all`, then stage what it
  changed.
- **Chain a commit and what follows it with `&&`, never `;`.**
- **Stale runtime library.** `nova build` and `nova test` in the debug
  end-to-end tests link `target/debug/nova_runtime.lib`. Run
  `cargo build --locked -p nova-runtime` before any `nova-cli` test run
  that follows a runtime change.
- **Port 3000 must be free for a full Windows suite** (the
  `http_server_example_*` tests). Check with
  `netstat -ano | grep -E "[:.]3000 .*LISTENING"`, which prints nothing
  when the port is free. **The user's own servers sometimes hold it.** On
  2026-10-05 it was QuoteFlow's `next start --hostname 127.0.0.1`. Never
  stop such a process: stop and ask the user.
- **Linux runs** use the existing harness:
  `bash /c/Users/SAKEER~1/AppData/Local/Temp/gcm/linux/run.sh <cargo args>`.
  - It exports the checkout's tracked files as an LF tarball, uncommitted
    edits included. **Untracked files are not exported:** commit or
    `git add` new files first.
  - In a throwaway `rust:1-slim` container, it runs
    `cargo build --locked --workspace`, then `cargo <args>`. The target
    directory is the named volume `nova-linux-target`.
  - Docker Desktop must be running: `docker version` shows a `Server:`
    section. Start it with PowerShell
    `Start-Process "C:\Program Files\Docker\Docker\Docker Desktop.exe"`.
- **Mutants run only on committed work,** and are undone with
  `git checkout -- <file>`. Never commit a mutant.
- **Counting a full run.** Every full-suite step below is followed by this,
  with `<FILE>` replaced:

  ```bash
  sed -E 's/\x1b\[[0-9;]*m//g' <FILE> | grep -E "^test result:" | awk '{p+=$4; f+=$6; i+=$8; n++} END {print n" result lines: "p" passed, "f" failed, "i" ignored"}'
  ```
- **Release builds take minutes.** A cold `cargo install` here is several
  minutes. Give such a command a 10-minute timeout, or run it in the
  background and wait for its notification.

---

### Task 1: The runtime is built inside nova-cli's build (risk 1)

The spike built stand-ins; this task builds the real runtime in place,
inside this workspace, on Windows and on Linux, before anything depends on
it (spec §11 risk 1).

**Files:**
- Modify: `Cargo.toml` (workspace): `[workspace.dependencies]` gains
  `flate2` and `crc32fast`.
- Modify: `crates/nova-runtime/Cargo.toml` (`links`) and
  `crates/nova-runtime/build.rs`.
- Modify: `crates/nova-cli/Cargo.toml`.
- Create: `crates/nova-cli/build_support.rs`, `crates/nova-cli/build.rs` and
  `crates/nova-cli/tests/build_support.rs`.
- Modify: `Cargo.lock`, through cargo.

**Interfaces:**
- Consumes: nothing from earlier tasks.
- Produces, in `crates/nova-cli/build_support.rs`, which only `build.rs` and
  its test compile:
  - `pub fn should_embed(var: Option<&str>, profile: &str) -> Result<bool, String>`
  - `#[derive(Clone, Copy, Debug, PartialEq, Eq)] pub enum Route { InPlace, Copy }`
  - `pub fn route(runtime_dir: &Path) -> io::Result<Route>`
  - `pub fn copy_package(from: &Path, to: &Path) -> io::Result<()>`
  - `pub fn dep_info_sources(dep_info: &str) -> Vec<PathBuf>`
  - `pub fn workspace_lockfile(runtime_dir: &Path) -> Option<PathBuf>`
  - `pub fn staticlib_name(target_env: &str) -> &'static str`
- Produces, for Task 4, these variables, which `build.rs` sets for every
  target of `nova-cli` through `cargo:rustc-env`:
  - `NOVA_TARGET`, the target triple;
  - `NOVA_EMBEDDED_RUNTIME`, the payload's path (an empty file when nothing
    is embedded);
  - `NOVA_EMBEDDED_RUNTIME_CRC32` and `NOVA_EMBEDDED_RUNTIME_SIZE`, both
    decimal, and both `0` when nothing is embedded;
  - `NOVA_EMBEDDED_RUNTIME_NAME`, the library's file name.
- Produces, in `nova-runtime`: `links = "nova_runtime"`. Its build script's
  `cargo:manifest_dir=<dir>` line reaches its direct dependents as
  `DEP_NOVA_RUNTIME_MANIFEST_DIR`.

- [ ] **Step 1: Take the baseline, before anything changes**

Docker Desktop must be running for Step 15. Check it now with
`docker version | grep -c '^Server:'`, which prints `1`. Then:

```bash
cd /d/Projects/nona/nova && P=/c/Users/SAKEER~1/AppData/Local/Temp/gcm/p30 && mkdir -p $P && git status --short | wc -l && git log --oneline -1
```

Expected: `0` and `4557176 docs: correct the 3.0 spec after its fact-check`.

Then measure a cold install of today's `nova`. This takes several minutes:

```bash
cd /d/Projects/nona/nova && P=/c/Users/SAKEER~1/AppData/Local/Temp/gcm/p30 && rm -rf $P/before $P/before-target && start=$(date +%s) && cargo install --locked --path crates/nova-cli --root $P/before --target-dir $P/before-target > $P/before-install.log 2>&1; echo "exit=$? seconds=$(( $(date +%s) - start ))"; ls -l $P/before/bin/
```

Expected: `exit=0`, a time in seconds, and one file, `nova.exe`, with its
byte size. Write both figures into the ledger as
`Task 1: baseline install <s> s, nova.exe <bytes> bytes`. Task 8 takes the
same measurement after the change.

Then take the baseline full suite, after checking port 3000:

```bash
cd /d/Projects/nona/nova && P=/c/Users/SAKEER~1/AppData/Local/Temp/gcm/p30 && cargo build --locked -p nova-runtime 2>&1 | tail -1 && cargo test --locked --workspace --all-features --no-fail-fast > $P/suite-baseline.txt 2>&1; echo "exit=$?"
```

Count it with the conventions' counting line. Expected: 0 failed. CI's
Windows leg reported 1222 passed and 8 ignored at `06f3cb5`. Ledger the
local figures as the baseline that Task 12 compares against.

- [ ] **Step 2: Write the failing tests**

Create `crates/nova-cli/tests/build_support.rs`:

```rust
//! Tests for `build_support.rs`, the decisions behind nova-cli's build
//! script (spec
//! `docs/superpowers/specs/2026-10-07-phase-3-0-foundations-design.md`
//! §4.1), compiled here from the same file.

#[path = "../build_support.rs"]
mod build_support;

use std::path::{Path, PathBuf};

use build_support::{
    copy_package, dep_info_sources, route, should_embed, staticlib_name, workspace_lockfile,
    Route,
};

/// A fresh, empty directory under the system temp dir, unique to this test.
fn fresh_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "nova-build-support-{name}-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("create the test directory");
    dir
}

fn runtime_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../nova-runtime")
}

#[test]
fn the_profile_decides_when_the_variable_is_unset() {
    assert_eq!(should_embed(None, "release"), Ok(true));
    assert_eq!(should_embed(None, "debug"), Ok(false));
}

#[test]
fn the_variable_overrides_the_profile() {
    assert_eq!(should_embed(Some("1"), "debug"), Ok(true));
    assert_eq!(should_embed(Some("0"), "release"), Ok(false));
}

#[test]
fn any_other_value_of_the_variable_is_an_error_naming_it() {
    for value in ["", "yes", "true", "2"] {
        let error = should_embed(Some(value), "release").unwrap_err();
        assert!(error.contains("NOVA_EMBED_RUNTIME"), "{value:?}: {error}");
    }
}

#[test]
fn nova_runtime_inherits_from_the_workspace_so_it_builds_in_place() {
    assert_eq!(route(&runtime_dir()).unwrap(), Route::InPlace);
}

#[test]
fn a_manifest_without_workspace_keys_builds_a_copy() {
    let dir = fresh_dir("packaged");
    std::fs::write(
        dir.join("Cargo.toml"),
        "[package]\nedition = \"2021\"\nname = \"nova-runtime\"\nversion = \"0.2.0\"\n\
         # edition.workspace = true is only a comment here\n",
    )
    .unwrap();
    assert_eq!(route(&dir).unwrap(), Route::Copy);
}

#[test]
fn a_stray_cargo_toml_orig_does_not_change_the_route() {
    // `git mergetool` leaves `*.orig` backups, and `.gitignore` hides them.
    let dir = fresh_dir("stray-orig");
    std::fs::write(
        dir.join("Cargo.toml"),
        "[package]\nname = \"r\"\nedition.workspace = true\n",
    )
    .unwrap();
    std::fs::write(dir.join("Cargo.toml.orig"), "a merge backup\n").unwrap();
    assert_eq!(route(&dir).unwrap(), Route::InPlace);
}

#[test]
fn the_copy_leaves_out_the_lockfile_and_target_and_adds_a_workspace_table() {
    let from = fresh_dir("copy-from");
    std::fs::create_dir_all(from.join("src/inner")).unwrap();
    std::fs::create_dir_all(from.join("target/release")).unwrap();
    // No final newline, as a hand-written manifest may have.
    std::fs::write(from.join("Cargo.toml"), "[package]\nname = \"r\"").unwrap();
    std::fs::write(from.join("Cargo.lock"), "pinned versions\n").unwrap();
    std::fs::write(from.join("build.rs"), "fn main() {}\n").unwrap();
    std::fs::write(from.join("src/lib.rs"), "").unwrap();
    std::fs::write(from.join("src/inner/shim.c"), "int x;\n").unwrap();
    std::fs::write(from.join("target/release/junk"), "").unwrap();
    let to = fresh_dir("copy-to").join("rt-src");

    copy_package(&from, &to).unwrap();

    assert!(to.join("build.rs").is_file());
    assert!(to.join("src/lib.rs").is_file());
    assert!(to.join("src/inner/shim.c").is_file());
    assert!(!to.join("Cargo.lock").exists());
    assert!(!to.join("target").exists());
    assert_eq!(
        std::fs::read_to_string(to.join("Cargo.toml")).unwrap(),
        "[package]\nname = \"r\"\n\n[workspace]\n"
    );
}

#[test]
fn copying_again_replaces_the_old_copy() {
    let from = fresh_dir("recopy-from");
    std::fs::write(from.join("Cargo.toml"), "[package]\nname = \"r\"\n").unwrap();
    let to = fresh_dir("recopy-to").join("rt-src");
    std::fs::create_dir_all(&to).unwrap();
    std::fs::write(to.join("stale.rs"), "").unwrap();

    copy_package(&from, &to).unwrap();

    assert!(!to.join("stale.rs").exists());
    assert!(to.join("Cargo.toml").is_file());
}

#[test]
fn dep_info_sources_reads_the_first_rule_and_unescapes_spaces() {
    // Absolute paths for this platform; on Windows they carry a drive
    // letter, whose colon must not end the rule's target.
    let base = std::env::temp_dir();
    let target = base.join("rt").join("libnova_runtime.rlib");
    let lib = base.join("w").join("nova-runtime").join("src").join("lib.rs");
    let spaced = base.join("w").join("with space").join("x.rs");
    let escaped = spaced.display().to_string().replace(' ', "\\ ");
    let dep_info = format!(
        "{}: {} {} src/relative.rs\n\n{}:\n",
        target.display(),
        lib.display(),
        escaped,
        lib.display()
    );
    assert_eq!(dep_info_sources(&dep_info), vec![lib, spaced]);
}

#[test]
fn an_empty_dep_info_names_nothing() {
    assert!(dep_info_sources("").is_empty());
    assert!(dep_info_sources("\n\n").is_empty());
}

#[test]
fn the_workspace_lockfile_is_the_nearest_above_the_runtime() {
    let found = workspace_lockfile(&runtime_dir()).expect("a Cargo.lock above nova-runtime");
    let expected = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../Cargo.lock");
    assert_eq!(
        found.canonicalize().unwrap(),
        expected.canonicalize().unwrap()
    );
}

#[test]
fn the_staticlib_is_named_for_the_target() {
    assert_eq!(staticlib_name("msvc"), "nova_runtime.lib");
    assert_eq!(staticlib_name("gnu"), "libnova_runtime.a");
    assert_eq!(staticlib_name(""), "libnova_runtime.a");
}
```

- [ ] **Step 3: Run the tests, and watch them fail**

```bash
cd /d/Projects/nona/nova && cargo test --locked -p nova-cli --test build_support 2>&1 | grep -E "^error|could not" | head -5
```

Expected: a compile error, `couldn't read` … `build_support.rs`: the module
does not exist yet.

- [ ] **Step 4: Write `build_support.rs`**

Create `crates/nova-cli/build_support.rs`:

```rust
//! The decisions behind `build.rs`, which builds the runtime library and
//! embeds it in `nova` (spec
//! `docs/superpowers/specs/2026-10-07-phase-3-0-foundations-design.md`
//! §4.1). Only `std` is used here, so `tests/build_support.rs` compiles this
//! same file and tests each decision without running a build.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

/// Whether to embed the runtime. `NOVA_EMBED_RUNTIME` (`var`) decides when
/// it is set; otherwise the profile does, so a release build, which
/// `cargo install` makes by default, embeds, and a debug build does not.
pub fn should_embed(var: Option<&str>, profile: &str) -> Result<bool, String> {
    match var {
        None => Ok(profile == "release"),
        Some("1") => Ok(true),
        Some("0") => Ok(false),
        Some(other) => Err(format!(
            "NOVA_EMBED_RUNTIME must be 1 or 0, not {other:?}"
        )),
    }
}

/// How the nested build reaches the runtime's source.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Route {
    /// A workspace member: build it in place, against the workspace's
    /// lockfile, with `--locked`.
    InPlace,
    /// A packaged crate: build a copy without its lockfile, `--offline`.
    Copy,
}

/// The route for the runtime in `runtime_dir`, read from its `Cargo.toml`.
/// A workspace member inherits from its workspace (`edition.workspace =
/// true`), and `cargo package` always resolves that away, so a packaged
/// manifest never says `workspace = true`. A file such as `Cargo.toml.orig`
/// would be a weaker sign: `git mergetool` leaves `*.orig` backups in a
/// checkout, and `.gitignore` hides them.
pub fn route(runtime_dir: &Path) -> io::Result<Route> {
    let manifest = fs::read_to_string(runtime_dir.join("Cargo.toml"))?;
    let inherits = manifest.lines().any(|line| {
        let line = line.trim_start();
        !line.starts_with('#') && line.replace(' ', "").contains("workspace=true")
    });
    Ok(if inherits { Route::InPlace } else { Route::Copy })
}

/// Copy a packaged runtime from `from` to `to`, for [`Route::Copy`]:
/// everything except its top-level `Cargo.lock`, whose pinned versions may
/// be missing from an offline cache, and `target`. The copy's manifest gains
/// an empty `[workspace]` table, so cargo never takes the copy for a member
/// of a workspace that happens to surround the build's output directory.
pub fn copy_package(from: &Path, to: &Path) -> io::Result<()> {
    if to.exists() {
        fs::remove_dir_all(to)?;
    }
    fs::create_dir_all(to)?;
    for entry in fs::read_dir(from)? {
        let entry = entry?;
        let name = entry.file_name();
        if name == "Cargo.lock" || name == "target" {
            continue;
        }
        copy_tree(&entry.path(), &to.join(&name))?;
    }
    let manifest = to.join("Cargo.toml");
    let mut text = fs::read_to_string(&manifest)?;
    if !text.ends_with('\n') {
        text.push('\n');
    }
    text.push_str("\n[workspace]\n");
    fs::write(&manifest, text)
}

fn copy_tree(from: &Path, to: &Path) -> io::Result<()> {
    if fs::metadata(from)?.is_dir() {
        fs::create_dir_all(to)?;
        for entry in fs::read_dir(from)? {
            let entry = entry?;
            copy_tree(&entry.path(), &to.join(entry.file_name()))?;
        }
        Ok(())
    } else {
        fs::copy(from, to).map(|_| ())
    }
}

/// The source files a cargo dep-info file (`<artifact>.d`) names on its
/// first rule: everything after `": "`, split on whitespace, with `\ ` as an
/// escaped space. Only absolute paths are returned, because a relative one
/// is relative to a `build.dep-info-basedir` this script cannot know.
pub fn dep_info_sources(dep_info: &str) -> Vec<PathBuf> {
    let Some(rule) = dep_info.lines().find(|line| !line.trim().is_empty()) else {
        return Vec::new();
    };
    let Some((_, sources)) = rule.split_once(": ") else {
        return Vec::new();
    };
    sources
        .replace("\\ ", "\u{0}")
        .split_whitespace()
        .map(|source| PathBuf::from(source.replace('\u{0}', " ")))
        .filter(|source| source.is_absolute())
        .collect()
}

/// The lockfile an in-place build of the runtime uses: the first
/// `Cargo.lock` in the runtime's directory or above it.
pub fn workspace_lockfile(runtime_dir: &Path) -> Option<PathBuf> {
    runtime_dir
        .ancestors()
        .map(|dir| dir.join("Cargo.lock"))
        .find(|lock| lock.is_file())
}

/// The runtime staticlib's file name for a target whose
/// `CARGO_CFG_TARGET_ENV` is `target_env`: `nova_runtime.lib` for MSVC, and
/// `libnova_runtime.a` for the other toolchains this project builds with.
pub fn staticlib_name(target_env: &str) -> &'static str {
    if target_env == "msvc" {
        "nova_runtime.lib"
    } else {
        "libnova_runtime.a"
    }
}
```

- [ ] **Step 5: Run the tests, and watch them pass**

```bash
cd /d/Projects/nona/nova && cargo test --locked -p nova-cli --test build_support 2>&1 | grep -E "^test result|FAILED|panicked"
```

Expected: `test result: ok. 12 passed; 0 failed`.

- [ ] **Step 6: Commit the decisions, then prove one test bites**

Write `$P/msg-1a.txt`:

```
nova-cli: the build script's decisions, with tests

build_support.rs holds what the runtime-embedding build script decides:
- whether to embed (NOVA_EMBED_RUNTIME, else the profile);
- which route reaches the runtime's source (the manifest's workspace
  keys, not a Cargo.toml.orig that a merge backup could fake);
- how a packaged copy is made (no Cargo.lock or target, plus an empty
  [workspace]);
- which files the rerun list follows (the nested build's dep-info);
- the staticlib's name for the target.

It uses only std, so tests/build_support.rs compiles the same file.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
```

```bash
cd /d/Projects/nona/nova && cargo fmt --all && git add crates/nova-cli/build_support.rs crates/nova-cli/tests/build_support.rs && git commit -q -F /c/Users/SAKEER~1/AppData/Local/Temp/gcm/p30/msg-1a.txt && git log -1 --format=%s
```

Expected: `nova-cli: the build script's decisions, with tests`.

The mutant: make `route` trust the file. In `build_support.rs`, replace the
line `let manifest = fs::read_to_string(runtime_dir.join("Cargo.toml"))?;`
with
`if runtime_dir.join("Cargo.toml.orig").exists() { return Ok(Route::Copy); } let manifest = fs::read_to_string(runtime_dir.join("Cargo.toml"))?;`.
Then:

```bash
cd /d/Projects/nona/nova && cargo test --locked -p nova-cli --test build_support 2>&1 | grep -E "^test .*FAILED|^test result"; git checkout -- crates/nova-cli/build_support.rs && git status --short
```

Expected: `a_stray_cargo_toml_orig_does_not_change_the_route ... FAILED`,
then `1 failed`, then an empty status.

- [ ] **Step 7: Declare the dependencies and the runtime's `links`**

In the workspace `Cargo.toml`, under `[workspace.dependencies]` after the
`# Misc` block's last line (`tracing-subscriber = …`), add:

```toml

# The runtime library inside `nova`: nova-cli's build script compresses it,
# and nova-driver unpacks it (docs/adr/0027-runtime-and-std-in-an-installed-nova.md).
# flate2 is the standard gzip crate. 1.1.10 declares Rust 1.67, and it
# brings miniz_oxide 0.9, adler2 and simd-adler32, which declare no
# minimum, so the MSRV job is their check. All four are pure Rust, with no
# C build. crc32fast was already in the lockfile.
flate2 = "1.1"
crc32fast = "1.4"
```

In `crates/nova-runtime/Cargo.toml`, after `description = "GC and async
runtime for Nova (linked into compiled binaries)"`, add:

```toml
# `links` lets this crate's build script hand metadata to the crates that
# depend on it: nova-cli's build script reads DEP_NOVA_RUNTIME_MANIFEST_DIR
# to find this source and build the staticlib it embeds
# (docs/adr/0027-runtime-and-std-in-an-installed-nova.md).
links = "nova_runtime"
```

In `crates/nova-runtime/build.rs`, add a paragraph to the end of the
module's doc comment:

```rust
//!
//! It also reports this crate's source directory as `links` metadata
//! (`cargo:manifest_dir`), which reaches nova-cli's build script as
//! `DEP_NOVA_RUNTIME_MANIFEST_DIR`: that script builds this crate's
//! staticlib a second time, in release mode, and embeds it in `nova`.
```

Then add this as the first line of `main()`:

```rust
    println!(
        "cargo:manifest_dir={}",
        std::env::var("CARGO_MANIFEST_DIR").expect("cargo sets CARGO_MANIFEST_DIR")
    );
```

In `crates/nova-cli/Cargo.toml`, add after `tracing-subscriber = { workspace
= true }` in `[dependencies]`:

```toml
# Its `links` metadata hands build.rs the runtime's source directory (spec
# docs/superpowers/specs/2026-10-07-phase-3-0-foundations-design.md §4.1).
# The runtime is already linked into `nova` through nova-codegen-cranelift,
# so this adds no code; only a direct dependent receives the metadata.
nova-runtime = { path = "../nova-runtime" }
```

Then add a new section after `[dependencies]`, before `[dev-dependencies]`:

```toml
[build-dependencies]
flate2 = { workspace = true }
crc32fast = { workspace = true }
```

- [ ] **Step 8: Write `build.rs`**

Create `crates/nova-cli/build.rs`:

```rust
//! Builds the runtime library, `nova-runtime`'s staticlib, and embeds it in
//! `nova`, gzip-compressed, so an installed `nova` links programs with
//! nothing beside it (spec
//! `docs/superpowers/specs/2026-10-07-phase-3-0-foundations-design.md` §4.1;
//! `docs/adr/0027-runtime-and-std-in-an-installed-nova.md`).
//!
//! On stable Cargo one crate cannot take another's staticlib (artifact
//! dependencies are unstable), so this script runs a nested
//! `cargo build --release` of the runtime into its own `OUT_DIR`. Its
//! decisions live in `build_support.rs`, which `tests/build_support.rs`
//! tests.

mod build_support;

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;

use build_support::Route;

fn main() {
    let out = PathBuf::from(var("OUT_DIR"));
    let target = var("TARGET");
    let lib_name = build_support::staticlib_name(&var("CARGO_CFG_TARGET_ENV"));
    println!("cargo:rustc-env=NOVA_TARGET={target}");
    println!("cargo:rustc-env=NOVA_EMBEDDED_RUNTIME_NAME={lib_name}");
    println!("cargo:rerun-if-env-changed=NOVA_EMBED_RUNTIME");

    let setting = std::env::var_os("NOVA_EMBED_RUNTIME")
        .map(|value| value.to_string_lossy().into_owned());
    let embed = build_support::should_embed(setting.as_deref(), &var("PROFILE"))
        .unwrap_or_else(|message| panic!("{message}"));

    let payload = out.join("nova_runtime.gz");
    let (crc32, size) = if embed {
        let library = build_runtime(&out, &target, lib_name);
        let bytes = std::fs::read(&library)
            .unwrap_or_else(|e| panic!("reading {}: {e}", library.display()));
        compress(&bytes, &payload);
        (crc32fast::hash(&bytes), bytes.len())
    } else {
        // An empty payload means "not embedded" (spec §4.1).
        std::fs::write(&payload, b"")
            .unwrap_or_else(|e| panic!("writing {}: {e}", payload.display()));
        (0, 0)
    };
    println!("cargo:rustc-env=NOVA_EMBEDDED_RUNTIME={}", payload.display());
    println!("cargo:rustc-env=NOVA_EMBEDDED_RUNTIME_CRC32={crc32}");
    println!("cargo:rustc-env=NOVA_EMBEDDED_RUNTIME_SIZE={size}");
}

fn var(name: &str) -> String {
    std::env::var(name).unwrap_or_else(|_| panic!("cargo sets {name} for build scripts"))
}

/// Build the runtime's staticlib for `target` with a nested cargo, and
/// return its path. Prints a `rerun-if-changed` for everything the library
/// is built from (spec §4.1).
fn build_runtime(out: &Path, target: &str, lib_name: &str) -> PathBuf {
    let runtime = PathBuf::from(std::env::var("DEP_NOVA_RUNTIME_MANIFEST_DIR").expect(
        "nova-runtime's `links` metadata: its build.rs prints `cargo:manifest_dir`",
    ));
    let route = build_support::route(&runtime)
        .unwrap_or_else(|e| panic!("reading {}/Cargo.toml: {e}", runtime.display()));
    let target_dir = out.join("rt");
    let cargo = std::env::var_os("CARGO").unwrap_or_else(|| "cargo".into());
    let mut command = Command::new(cargo);
    command
        .args(["build", "--lib", "--release", "--target", target])
        .arg("--target-dir")
        .arg(&target_dir);
    match route {
        Route::InPlace => {
            // The workspace's lockfile applies; `--locked` never rewrites
            // it. Not `--offline`: a `cargo install --git` without
            // `--locked` may have fetched newer versions than the lockfile
            // pins, and cargo fetches only what is missing.
            command
                .arg("--manifest-path")
                .arg(runtime.join("Cargo.toml"))
                .arg("--locked");
        }
        Route::Copy => {
            let copy = out.join("rt-src");
            build_support::copy_package(&runtime, &copy).unwrap_or_else(|e| {
                panic!("copying {} to {}: {e}", runtime.display(), copy.display())
            });
            command
                .arg("--manifest-path")
                .arg(copy.join("Cargo.toml"))
                .arg("--offline");
        }
    }
    for name in [
        "CARGO_TARGET_DIR",
        "CARGO_BUILD_TARGET",
        "RUSTFLAGS",
        "CARGO_ENCODED_RUSTFLAGS",
    ] {
        command.env_remove(name);
    }
    let status = command
        .status()
        .expect("spawning the nested cargo build of nova-runtime");
    assert!(
        status.success(),
        "the nested cargo build of nova-runtime failed ({status})"
    );

    let release = target_dir.join(target).join("release");
    println!("cargo:rerun-if-changed={}", runtime.display());
    if route == Route::InPlace {
        // A packaged runtime's sources never change, and its copy is
        // rewritten on every run, so only the workspace route follows the
        // lockfile and the dep-info file's local sources, which cover path
        // dependencies such as nova-diagnostics.
        if let Some(lock) = build_support::workspace_lockfile(&runtime) {
            println!("cargo:rerun-if-changed={}", lock.display());
        }
        let dep_info =
            std::fs::read_to_string(release.join("libnova_runtime.d")).unwrap_or_default();
        for source in build_support::dep_info_sources(&dep_info) {
            println!("cargo:rerun-if-changed={}", source.display());
        }
    }
    let library = release.join(lib_name);
    assert!(
        library.is_file(),
        "the nested build made no {}",
        library.display()
    );
    library
}

/// Gzip `bytes` into `payload` at flate2's best compression.
fn compress(bytes: &[u8], payload: &Path) {
    let file = std::fs::File::create(payload)
        .unwrap_or_else(|e| panic!("creating {}: {e}", payload.display()));
    let mut encoder = flate2::write::GzEncoder::new(file, flate2::Compression::best());
    encoder
        .write_all(bytes)
        .unwrap_or_else(|e| panic!("compressing into {}: {e}", payload.display()));
    encoder
        .finish()
        .unwrap_or_else(|e| panic!("finishing {}: {e}", payload.display()));
}
```

- [ ] **Step 9: Build once without `--locked`, to update the lockfile**

```bash
cd /d/Projects/nona/nova && cargo build -p nova-cli 2>&1 | tail -2 && git diff Cargo.lock | grep -E '^[+-]name = ' | sort
```

Expected: `Finished`, then exactly four added packages, in some order:
`+name = "adler2"`, `+name = "flate2"`, `+name = "miniz_oxide"` and
`+name = "simd-adler32"`. Nothing is removed. Any other package means
another crate moved: stop, and ledger a ruling before you continue.

- [ ] **Step 10: A debug build embeds nothing**

```bash
cd /d/Projects/nona/nova && stat -c '%s %n' $(ls -t target/debug/build/nova-cli-*/out/nova_runtime.gz | head -1)
```

Expected: `0 …/out/nova_runtime.gz`.

- [ ] **Step 11: The in-place nested build works on Windows (risk 1)**

This runs the nested release build of the runtime and its dependencies. It
takes a few minutes the first time:

```bash
cd /d/Projects/nona/nova && P=/c/Users/SAKEER~1/AppData/Local/Temp/gcm/p30 && start=$(date +%s) && NOVA_EMBED_RUNTIME=1 cargo build --locked -p nova-cli > $P/embed-win.log 2>&1; echo "exit=$? seconds=$(( $(date +%s) - start ))"; tail -2 $P/embed-win.log
```

Expected: `exit=0` and `Finished`. If it fails, read the nested cargo's own
error in the log before changing anything. A path too long for the C
compiler (spec §11 risk 8) shows as a `cl.exe` or `lib.exe` error naming a
deep `out\rt\…` path.

Then record the sizes, and check the rerun list:

```bash
cd /d/Projects/nona/nova && O=$(dirname $(ls -t target/debug/build/nova-cli-*/output | head -1)) && stat -c '%s %n' $O/out/nova_runtime.gz $O/out/rt/x86_64-pc-windows-msvc/release/nova_runtime.lib && grep -c 'rerun-if-changed=.*nova-diagnostics' $O/output && grep -c 'rerun-if-changed=.*Cargo.lock' $O/output
```

Expected:
- two sizes, the payload a few MB and the library around 15 MB (decision 1
  measured 4.6 MB and 14.6 MB). Ledger both;
- `3` or more lines naming nova-diagnostics' sources;
- `1` line for `Cargo.lock`.

- [ ] **Step 12: The rerun list follows a path dependency**

```bash
cd /d/Projects/nona/nova && O=$(dirname $(ls -t target/debug/build/nova-cli-*/output | head -1)) && before=$(stat -c %Y $O/output) && sleep 2 && touch crates/nova-diagnostics/src/lib.rs && NOVA_EMBED_RUNTIME=1 cargo build --locked -p nova-cli 2>&1 | tail -1 && echo "reran: $([ $(stat -c %Y $O/output) -gt $before ] && echo yes || echo no)"
```

Expected: `Finished`, then `reran: yes`.

Then build without the variable. The build script reruns, because the
variable changed, and writes an empty payload again:

```bash
cd /d/Projects/nona/nova && cargo build --locked -p nova-cli 2>&1 | tail -1 && stat -c '%s' $(ls -t target/debug/build/nova-cli-*/out/nova_runtime.gz | head -1)
```

Expected: `Finished`, then `0`.

- [ ] **Step 13: MSRV, clippy and rustfmt**

```bash
cd /d/Projects/nona/nova && P=/c/Users/SAKEER~1/AppData/Local/Temp/gcm/p30 && RUSTUP_TOOLCHAIN=1.78 RUSTFLAGS="-D warnings" cargo check --locked --workspace --target-dir target/msrv > $P/msrv-1.txt 2>&1; echo "exit=$?"; tail -2 $P/msrv-1.txt
```

Expected: `exit=0` and `Finished`. This builds the build scripts, and so
flate2, miniz_oxide and simd-adler32, with Rust 1.78.

If a new crate fails on 1.78, pin the newest version of it that builds on
1.78 with `cargo update -p <crate> --precise <version>`, and rerun Step 9's
diff and this step. Ledger a ruling naming the crate and the version.

```bash
cd /d/Projects/nona/nova && cargo clippy --locked -p nova-cli -p nova-runtime --all-targets -- -D warnings 2>&1 | tail -1 && cargo fmt --all -- --check && echo fmt-ok
```

Expected: `Finished` and `fmt-ok`.

- [ ] **Step 14: Commit**

Write `$P/msg-1b.txt`:

```
nova-cli: build the runtime library inside nova's own build

nova-runtime declares `links = "nova_runtime"` and reports its source
directory. nova-cli's new build script runs a nested
`cargo build --release` of the runtime into OUT_DIR, gzips the staticlib,
and exports its path, CRC-32 and size for `include_bytes!`. The nested
build runs in place with --locked inside a workspace, or on a
lockfile-free copy with --offline from a package.

Only release-profile builds embed, unless NOVA_EMBED_RUNTIME says 1 or 0.
Reruns follow the runtime's directory, the dep-info file's local sources
and the lockfile.

Verified with the real runtime, in place, on Windows and on Linux
(spec risk 1).

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
```

```bash
cd /d/Projects/nona/nova && cargo fmt --all && git add Cargo.toml Cargo.lock crates/nova-runtime/Cargo.toml crates/nova-runtime/build.rs crates/nova-cli/Cargo.toml crates/nova-cli/build.rs && git commit -q -F /c/Users/SAKEER~1/AppData/Local/Temp/gcm/p30/msg-1b.txt && git log -1 --format=%s && git status --short | wc -l
```

Expected: `nova-cli: build the runtime library inside nova's own build`,
then `0`.

- [ ] **Step 15: The in-place nested build works on Linux (risk 1)**

A release build embeds by default, which is the `cargo install` path:

```bash
cd /d/Projects/nona/nova && P=/c/Users/SAKEER~1/AppData/Local/Temp/gcm/p30 && bash /c/Users/SAKEER~1/AppData/Local/Temp/gcm/linux/run.sh build --release --locked -p nova-cli > $P/embed-linux.log 2>&1; echo "exit=$?"; grep -E "^linux:|Finished|^error" $P/embed-linux.log | tail -4
```

Expected: `exit=0`, the `linux: source <rev>` line naming Step 14's
commit, and two `Finished` lines: the workspace's debug build, then this
release build.

Then read the payload and library sizes from the volume:

```bash
MSYS_NO_PATHCONV=1 docker run --rm -v nova-linux-target:/work/target rust:1-slim bash -c 'ls -l /work/target/release/build/nova-cli-*/out/nova_runtime.gz /work/target/release/build/nova-cli-*/out/rt/x86_64-unknown-linux-gnu/release/libnova_runtime.a'
```

Expected: both files exist. The payload is a few MB, and the library is
around 30 MB (decision 1 measured 7.9 MB and 30.2 MB). Ledger both. If
Linux needed a fix, commit it as its own commit, with a ruling in the
ledger.

### Task 2: std becomes the crate nova-std

**Files:**
- Modify: `Cargo.toml` (workspace): `members = ["crates/*", "std"]`.
- Create: `std/Cargo.toml` and `std/lib.rs`.
- Modify: `crates/nova-resolver/Cargo.toml`, and
  `crates/nova-resolver/src/lib.rs:1603-1645` (the doc comment and both
  tables).
- Modify: `Cargo.lock`, through cargo: `nova-std` is new.

**Interfaces:**
- Consumes: nothing from earlier tasks.
- Produces, in `nova_std`: one `pub const <MODULE>: &str` per module:
  `BYTES`, `COLLECTIONS`, `CORE`, `CRYPTO`, `FMT`, `FS`, `HTTP`, `IO`,
  `JSON`, `LOG`, `NET`, `PROCESS`, `STRINGS`, `SYNC`, `TASK`, `TEST` and
  `TIME`; and `pub const ALL: [(&str, &str); 17]`, each module's directory
  name with its source.
- Unchanged for every consumer: `nova_resolver::STD_MODULES:
  [(&str, &str); 16]` and `nova_resolver::STD_TEST_MODULE: (&str, &str)`,
  with the same names, `$std.*` module names and order.

- [ ] **Step 1: Make the crate, with its test and an empty table**

In the workspace `Cargo.toml`, change `members = ["crates/*"]` to
`members = ["crates/*", "std"]`.

Create `std/Cargo.toml`:

```toml
[package]
name = "nova-std"
version = "0.2.0"
edition.workspace = true
rust-version.workspace = true
license.workspace = true
repository.workspace = true
description = "The Nova standard library's sources, embedded for the compiler"

[lib]
path = "lib.rs"
# Not a benchmark target: `cargo bench` would otherwise build this under
# libtest's harness, which rejects criterion's `--output-format`. The
# workspace's only benchmark is `nova-lexer/benches/lex.rs`.
bench = false
```

Create `std/lib.rs`, with the tests and, for now, an empty `ALL`:

```rust
//! The sources of Nova's standard library, `std/*/lib.nova`, embedded at
//! build time so the compiler stays one self-contained executable.
//!
//! Every `include_str!` path stays inside this package, so `cargo package`
//! carries all 17 files, and its verification step builds the packaged copy
//! on its own (spec
//! `docs/superpowers/specs/2026-10-07-phase-3-0-foundations-design.md` §3).
//! `nova-resolver` gives each module its `$std.*` name and its place.

/// Every embedded module, by its directory under `std/`.
pub const ALL: [(&str, &str); 0] = [];

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;
    use std::path::Path;

    /// Every `std/*/lib.nova` on disk is embedded, and nothing else is.
    #[test]
    fn every_std_module_is_embedded() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"));
        let on_disk: BTreeSet<String> = std::fs::read_dir(root)
            .expect("read std/")
            .map(|entry| entry.expect("read an entry of std/").path())
            .filter(|path| path.join("lib.nova").is_file())
            .map(|path| {
                path.file_name()
                    .expect("a directory name")
                    .to_string_lossy()
                    .into_owned()
            })
            .collect();
        let embedded: BTreeSet<String> =
            super::ALL.iter().map(|(name, _)| name.to_string()).collect();
        assert_eq!(embedded, on_disk);
        assert_eq!(on_disk.len(), 17);
    }

    /// Each entry carries its own file, not a neighbour's.
    #[test]
    fn each_entry_is_its_own_file() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"));
        for (name, source) in super::ALL {
            let on_disk = std::fs::read_to_string(root.join(name).join("lib.nova"))
                .expect("read a std module");
            assert_eq!(source, on_disk, "std/{name}/lib.nova");
        }
    }
}
```

- [ ] **Step 2: Run the test, and watch it fail**

The new member changes `Cargo.lock`, so this first run omits `--locked`:

```bash
cd /d/Projects/nona/nova && cargo test -p nova-std 2>&1 | grep -E "^test |^test result|left|right" | head -8
```

Expected: `every_std_module_is_embedded ... FAILED`, its assertion showing
an empty `left` set and the 17 names on the `right`, and
`1 failed`. The other test passes, vacuously, over an empty table.

- [ ] **Step 3: Embed the 17 modules**

Replace the line `pub const ALL: [(&str, &str); 0] = [];` with:

```rust
/// `std/bytes/lib.nova`.
pub const BYTES: &str = include_str!("bytes/lib.nova");
/// `std/collections/lib.nova`.
pub const COLLECTIONS: &str = include_str!("collections/lib.nova");
/// `std/core/lib.nova`.
pub const CORE: &str = include_str!("core/lib.nova");
/// `std/crypto/lib.nova`.
pub const CRYPTO: &str = include_str!("crypto/lib.nova");
/// `std/fmt/lib.nova`.
pub const FMT: &str = include_str!("fmt/lib.nova");
/// `std/fs/lib.nova`.
pub const FS: &str = include_str!("fs/lib.nova");
/// `std/http/lib.nova`.
pub const HTTP: &str = include_str!("http/lib.nova");
/// `std/io/lib.nova`.
pub const IO: &str = include_str!("io/lib.nova");
/// `std/json/lib.nova`.
pub const JSON: &str = include_str!("json/lib.nova");
/// `std/log/lib.nova`.
pub const LOG: &str = include_str!("log/lib.nova");
/// `std/net/lib.nova`.
pub const NET: &str = include_str!("net/lib.nova");
/// `std/process/lib.nova`.
pub const PROCESS: &str = include_str!("process/lib.nova");
/// `std/strings/lib.nova`.
pub const STRINGS: &str = include_str!("strings/lib.nova");
/// `std/sync/lib.nova`.
pub const SYNC: &str = include_str!("sync/lib.nova");
/// `std/task/lib.nova`.
pub const TASK: &str = include_str!("task/lib.nova");
/// `std/test/lib.nova`, seeded only under `nova test`.
pub const TEST: &str = include_str!("test/lib.nova");
/// `std/time/lib.nova`.
pub const TIME: &str = include_str!("time/lib.nova");

/// Every embedded module, by its directory under `std/`.
pub const ALL: [(&str, &str); 17] = [
    ("bytes", BYTES),
    ("collections", COLLECTIONS),
    ("core", CORE),
    ("crypto", CRYPTO),
    ("fmt", FMT),
    ("fs", FS),
    ("http", HTTP),
    ("io", IO),
    ("json", JSON),
    ("log", LOG),
    ("net", NET),
    ("process", PROCESS),
    ("strings", STRINGS),
    ("sync", SYNC),
    ("task", TASK),
    ("test", TEST),
    ("time", TIME),
];
```

- [ ] **Step 4: Run the tests, and watch them pass**

```bash
cd /d/Projects/nona/nova && cargo test --locked -p nova-std 2>&1 | grep -E "^test result|FAILED"
```

Expected: `test result: ok. 2 passed; 0 failed`.

- [ ] **Step 5: Take std's sources from nova-std**

In `crates/nova-resolver/Cargo.toml`, add to `[dependencies]` after
`nova-parser = { path = "../nova-parser" }`:

```toml
# std's sources (spec 2026-10-07-phase-3-0-foundations-design.md §3).
nova-std = { path = "../../std" }
```

In `crates/nova-resolver/src/lib.rs`, replace this doc comment's two
sentences:

```rust
/// order listed here — `std/core` stays first so its module index is
/// unchanged from when it was the only embedded module. Each is embedded at
/// build time (`include_str!`, paths relative to this file) so the compiler
/// stays a single self-contained executable. Each name is `$std.*`, not a
```

with:

```rust
/// order listed here — `std/core` stays first so its module index is
/// unchanged from when it was the only embedded module. The sources come
/// from the `nova-std` crate, which embeds `std/` at build time through
/// paths inside its own package, so the compiler stays a single
/// self-contained executable and `cargo package` keeps the files. Each name
/// is `$std.*`, not a
```

Rewrap the rest of that comment only if rustfmt asks. Then replace
`STD_MODULES`' sixteen entries, and `STD_TEST_MODULE`'s value, so the two
read:

```rust
pub const STD_MODULES: [(&str, &str); 16] = [
    ("$std.core", nova_std::CORE),
    ("$std.bytes", nova_std::BYTES),
    ("$std.io", nova_std::IO),
    ("$std.fs", nova_std::FS),
    ("$std.collections", nova_std::COLLECTIONS),
    ("$std.strings", nova_std::STRINGS),
    ("$std.fmt", nova_std::FMT),
    ("$std.task", nova_std::TASK),
    ("$std.sync", nova_std::SYNC),
    ("$std.net", nova_std::NET),
    ("$std.time", nova_std::TIME),
    ("$std.json", nova_std::JSON),
    ("$std.log", nova_std::LOG),
    ("$std.http", nova_std::HTTP),
    ("$std.crypto", nova_std::CRYPTO),
    ("$std.process", nova_std::PROCESS),
];
```

```rust
pub const STD_TEST_MODULE: (&str, &str) = ("$std.test", nova_std::TEST);
```

Leave `STD_TEST_MODULE`'s doc comment as it is. Then update the lockfile,
and check that no `include_str!` into `std/` is left in the resolver:

```bash
cd /d/Projects/nona/nova && cargo build -p nova-resolver 2>&1 | tail -1 && git grep -c 'include_str!("../../../std' -- crates/ ; echo "grep-exit=$?"
```

Expected: `Finished`, then `grep-exit=1`: git grep found nothing.

- [ ] **Step 6: The whole suite still passes**

Order, names and sources are unchanged, so every std-using test is the
check here. Check port 3000 first.

```bash
cd /d/Projects/nona/nova && P=/c/Users/SAKEER~1/AppData/Local/Temp/gcm/p30 && cargo build --locked --workspace 2>&1 | tail -1 && cargo test --locked --workspace --all-features --no-fail-fast > $P/suite-2.txt 2>&1; echo "exit=$?"
```

Count it. Expected: 0 failed, and exactly 14 more passed than Task 1's
baseline: Task 1's 12 build-script tests and the two `nova-std` tests.

- [ ] **Step 7: Commit, then prove the embedding test bites**

Write `$P/msg-2.txt`:

```
std: make std/ the crate nova-std

std/lib.rs embeds each std/*/lib.nova through a path inside its own
package, so `cargo package` keeps all 17 files. nova-resolver's
STD_MODULES and STD_TEST_MODULE keep their names, order and $std.*
module names, and take their sources from nova-std instead of
`include_str!` paths that left the crate.

A test checks that the embedded set matches std/ on disk.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
```

```bash
cd /d/Projects/nona/nova && cargo fmt --all && git add Cargo.toml Cargo.lock std/Cargo.toml std/lib.rs crates/nova-resolver/Cargo.toml crates/nova-resolver/src/lib.rs && git commit -q -F /c/Users/SAKEER~1/AppData/Local/Temp/gcm/p30/msg-2.txt && git log -1 --format=%s
```

Expected: `std: make std/ the crate nova-std`.

The mutant: in `std/lib.rs`, delete the line `("time", TIME),`, and change
`[(&str, &str); 17]` to `[(&str, &str); 16]`. Then:

```bash
cd /d/Projects/nona/nova && cargo test --locked -p nova-std 2>&1 | grep -E "^test .*FAILED|^test result"; git checkout -- std/lib.rs && git status --short
```

Expected: `every_std_module_is_embedded ... FAILED`, then an empty status.

- [ ] **Step 8: std survives packaging**

On the committed, clean tree:

```bash
cd /d/Projects/nona/nova && cargo package -p nova-std --locked --list 2>/dev/null | grep -c '\.nova$' && cargo package -p nova-std --locked 2>&1 | grep -E "Packaged|Verifying|Finished|^error" | head -4
```

Expected: `17`, then `Packaged …`, `Verifying nova-std v0.2.0 …` and
`Finished`. The verification step built the packaged copy on its own.

---

### Task 3: The runtime cache, and where nova looks for the runtime

**Files:**
- Create: `crates/nova-driver/src/runtime_cache.rs`.
- Modify: `crates/nova-driver/src/link.rs`: `find_runtime_lib`, and a new
  `beside_exe`, at `:86-114`.
- Modify: `crates/nova-driver/src/lib.rs`: the module and its re-exports,
  after `mod link;` at `:9`.
- Modify: `crates/nova-driver/Cargo.toml`, and `Cargo.lock` through cargo.

**Interfaces:**
- Consumes: the workspace's `flate2` and `crc32fast` entries (Task 1).
- Produces, re-exported from `nova_driver`:
  - `#[derive(Clone, Copy, Debug)] pub struct EmbeddedRuntime { pub gz:
    &'static [u8], pub crc32: u32, pub size: u64, pub file_name: &'static
    str, pub version: &'static str }`
  - `pub fn set_embedded_runtime(runtime: EmbeddedRuntime)`, which records
    nothing for an empty `gz`.
- Produces, crate-private: `embedded_runtime() -> Option<&'static
  EmbeddedRuntime>`; `cache_root(nova_home, userprofile, home:
  Option<PathBuf>, windows: bool) -> Result<PathBuf>`;
  `unpack(&EmbeddedRuntime, root: &Path) -> Result<PathBuf>`;
  `locate(override_path: Option<PathBuf>, embedded:
  Option<&EmbeddedRuntime>, cache_root: impl FnOnce() -> Result<PathBuf>,
  beside_exe: impl FnOnce() -> Result<PathBuf>) -> Result<PathBuf>`.

- [ ] **Step 1: Write the tests, against stubs**

In `crates/nova-driver/Cargo.toml`, add to `[dependencies]` after
`nova-codegen-llvm = { path = "../nova-codegen-llvm" }`:

```toml
# Unpacking the runtime library nova-cli embeds (spec
# 2026-10-07-phase-3-0-foundations-design.md §4.2).
flate2 = { workspace = true }
crc32fast = { workspace = true }
```

In `crates/nova-driver/src/lib.rs`, after `mod link;`, add:

```rust
mod runtime_cache;

pub use runtime_cache::{set_embedded_runtime, EmbeddedRuntime};
```

Create `crates/nova-driver/src/runtime_cache.rs` with the real types, stub
functions and the full tests:

```rust
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

pub(crate) fn cache_root(
    _nova_home: Option<PathBuf>,
    _userprofile: Option<PathBuf>,
    _home: Option<PathBuf>,
    _windows: bool,
) -> Result<PathBuf> {
    bail!("not implemented")
}

pub(crate) fn unpack(_runtime: &EmbeddedRuntime, _root: &Path) -> Result<PathBuf> {
    bail!("not implemented")
}

fn verified(_path: &Path, _runtime: &EmbeddedRuntime) -> bool {
    false
}

pub(crate) fn locate(
    _override_path: Option<PathBuf>,
    _embedded: Option<&EmbeddedRuntime>,
    _cache_root: impl FnOnce() -> Result<PathBuf>,
    _beside_exe: impl FnOnce() -> Result<PathBuf>,
) -> Result<PathBuf> {
    bail!("not implemented")
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A fake library and its embedded form. The bytes are the same in
    /// every process, so a child process unpacks exactly what its parent
    /// does.
    fn fake_runtime() -> EmbeddedRuntime {
        let library: Vec<u8> = (0..4 * 1024 * 1024u32).map(|i| (i % 251) as u8).collect();
        let mut encoder =
            flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::fast());
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
        let dir = std::env::temp_dir().join(format!(
            "nova-runtime-cache-{name}-{}",
            std::process::id()
        ));
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
        let dir = library_path(&root, &runtime).parent().unwrap().to_path_buf();
        let leftovers: Vec<_> = fs::read_dir(dir)
            .unwrap()
            .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
            .filter(|name| name.ends_with(".tmp"))
            .collect();
        assert!(leftovers.is_empty(), "{leftovers:?}");
    }

    /// Two processes unpacking at once both get a verified file (spec §9,
    /// as Section 3 of the design asked). The test runs its own binary
    /// twice, and each child runs only `unpack_as_a_child_process`.
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
                    .spawn()
                    .unwrap()
            })
            .collect();
        for mut child in children {
            assert!(child.wait().unwrap().success());
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
        let windows = cache_root(None, Some("C:/Users/u".into()), Some("/home/u".into()), true);
        assert_eq!(
            windows.unwrap(),
            PathBuf::from("C:/Users/u").join(".nova").join("runtime")
        );
        let unix = cache_root(None, Some("C:/Users/u".into()), Some("/home/u".into()), false);
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
            let error = cache_root(None, None, None, windows).unwrap_err().to_string();
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
        let error = locate(Some(missing), None, never, never).unwrap_err().to_string();
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
```

- [ ] **Step 2: Run the tests, and watch them fail**

```bash
cd /d/Projects/nona/nova && cargo build -p nova-driver 2>&1 | tail -1 && cargo test --locked -p nova-driver runtime_cache 2>&1 | grep -E "^test result"
```

The first command omits `--locked`, because nova-driver's dependency list
changed. Expected: `Finished`, then `test result: FAILED. 1 passed; 16
failed`. Only `unpack_as_a_child_process` passes against the stubs: without
its variable it has nothing to do.

- [ ] **Step 3: Write the implementation**

Replace the four stub functions (`cache_root`, `unpack`, `verified`,
`locate`) with:

```rust
/// The cache's root, `$NOVA_HOME/runtime`. `NOVA_HOME` defaults to `.nova`
/// in the home directory, which is `USERPROFILE` on Windows and `HOME`
/// elsewhere. It takes the variables' values rather than reading them, so
/// tests need not change the process environment. An empty value counts as
/// unset.
pub(crate) fn cache_root(
    nova_home: Option<PathBuf>,
    userprofile: Option<PathBuf>,
    home: Option<PathBuf>,
    windows: bool,
) -> Result<PathBuf> {
    let set = |value: Option<PathBuf>| value.filter(|path| !path.as_os_str().is_empty());
    if let Some(nova_home) = set(nova_home) {
        return Ok(nova_home.join("runtime"));
    }
    let (home, name) = if windows {
        (set(userprofile), "USERPROFILE")
    } else {
        (set(home), "HOME")
    };
    match home {
        Some(home) => Ok(home.join(".nova").join("runtime")),
        None => bail!(
            "cannot place the runtime library's cache: neither NOVA_HOME nor {name} is \
             set; set NOVA_HOME to a writable directory, or NOVA_RUNTIME_LIB to a runtime \
             library"
        ),
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
                Err(error).with_context(|| format!("moving the runtime library to {}", path.display()))
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
        match fs::OpenOptions::new().write(true).create_new(true).open(&temp) {
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
```

- [ ] **Step 4: Route `find_runtime_lib` through `locate`**

In `crates/nova-driver/src/link.rs`, replace `find_runtime_lib` and its doc
comment (`:86-114`, from `/// Locate the runtime static library:` through
the function's closing brace) with:

```rust
/// Locate the runtime static library (spec
/// `docs/superpowers/specs/2026-10-07-phase-3-0-foundations-design.md`
/// §4.2): `NOVA_RUNTIME_LIB` first, then the runtime this `nova` carries,
/// unpacked to its cache, then the executable's neighbours.
fn find_runtime_lib() -> Result<PathBuf> {
    let var = |name: &str| std::env::var_os(name).map(PathBuf::from);
    runtime_cache::locate(
        var("NOVA_RUNTIME_LIB"),
        runtime_cache::embedded_runtime(),
        || runtime_cache::cache_root(var("NOVA_HOME"), var("USERPROFILE"), var("HOME"), cfg!(windows)),
        || {
            let exe = std::env::current_exe().context("locating the nova executable")?;
            beside_exe(&exe)
        },
    )
}

/// The runtime library in the executable's directory or one of the two
/// above it, where cargo leaves it in a checkout: the CLI binary and the
/// staticlib share a target directory, and test binaries sit one level
/// deeper, in `deps/`.
fn beside_exe(exe: &Path) -> Result<PathBuf> {
    let name = runtime_lib_name();
    for dir in exe.ancestors().skip(1).take(3) {
        let candidate = dir.join(name);
        if candidate.exists() {
            return Ok(candidate);
        }
    }
    bail!(
        "could not find the Nova runtime library ({name}) near {}, and this nova does not \
         carry one. Install nova with the release profile, which is `cargo install`'s \
         default, or build it with NOVA_EMBED_RUNTIME=1; in a checkout, \
         `cargo build -p nova-runtime` puts the library beside it. NOVA_RUNTIME_LIB can \
         also name one.",
        exe.display()
    )
}
```

Add `use crate::runtime_cache;` after the file's `use std::process::Command;`
line. Then append a test module at the end of `link.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn with_nothing_beside_it_the_error_says_how_to_get_a_runtime() {
        let dir = std::env::temp_dir().join(format!("nova-beside-exe-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("a/b/c")).unwrap();
        let error = beside_exe(&dir.join("a/b/c/nova.exe")).unwrap_err().to_string();
        assert!(error.contains("NOVA_EMBED_RUNTIME=1"), "{error}");
        assert!(error.contains("NOVA_RUNTIME_LIB"), "{error}");
        assert!(error.contains("cargo install"), "{error}");
    }
}
```

- [ ] **Step 5: Run the tests, and watch them pass**

```bash
cd /d/Projects/nona/nova && cargo test --locked -p nova-driver runtime_cache 2>&1 | grep -E "^test result|FAILED" && cargo test --locked -p nova-driver with_nothing_beside_it 2>&1 | grep -E "^test result"
```

Expected: `test result: ok. 17 passed; 0 failed` for the cache tests, and
`1 passed` for the link test.

- [ ] **Step 6: Linking still works with nothing embedded**

```bash
cd /d/Projects/nona/nova && cargo build --locked -p nova-runtime 2>&1 | tail -1 && cargo test --locked -p nova-cli --test run_tests build_hello_world_standalone 2>&1 | grep -E "^test result"
```

Expected: `1 passed`: a debug `nova` carries nothing, so the lookup reaches
the executable's neighbours as before.

- [ ] **Step 7: clippy, rustfmt, and commit**

```bash
cd /d/Projects/nona/nova && cargo clippy --locked -p nova-driver --all-targets -- -D warnings 2>&1 | tail -1 && cargo fmt --all && cargo fmt --all -- --check && echo fmt-ok
```

Expected: `Finished` and `fmt-ok`.

Write `$P/msg-3.txt`:

```
nova-driver: unpack the runtime nova carries, and look for it second

runtime_cache.rs holds the runtime a nova carries (registered once by
nova-cli), the cache under $NOVA_HOME/runtime/<version>-<crc32>/, and
the lookup order:
1. NOVA_RUNTIME_LIB;
2. the embedded runtime;
3. the executable's neighbours.

Unpacking writes a new temporary file (create_new; process id plus a
per-process counter) and renames it into place. Every use checks the
file's size and CRC-32. A runtime that is embedded but fails to unpack
is an error, not a fall-back.

The not-found error now says how to get a nova that carries its
runtime.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
```

```bash
cd /d/Projects/nona/nova && git add Cargo.lock crates/nova-driver && git commit -q -F /c/Users/SAKEER~1/AppData/Local/Temp/gcm/p30/msg-3.txt && git log -1 --format=%s
```

Expected: `nova-driver: unpack the runtime nova carries, and look for it
second`.

- [ ] **Step 8: Prove the no-fall-back test bites**

The mutant: in `runtime_cache.rs`'s `locate`, replace
`Some(runtime) => unpack(runtime, &cache_root()?),` with
`Some(runtime) => cache_root().and_then(|root| unpack(runtime, &root)).or_else(|_| beside_exe()),`.
Then:

```bash
cd /d/Projects/nona/nova && cargo test --locked -p nova-driver runtime_cache 2>&1 | grep -E "^test .*FAILED|^test result"; git checkout -- crates/nova-driver/src/runtime_cache.rs && git status --short
```

Expected: `an_embedded_runtime_that_fails_to_unpack_is_an_error_not_a_fall_back
... FAILED`, because the neighbours' closure panics. Then an empty status.

---

### Task 4: nova carries its runtime, and nova version

**Files:**
- Create: `crates/nova-cli/src/embedded.rs` and
  `crates/nova-cli/src/cmd/version.rs`.
- Modify: `crates/nova-cli/src/main.rs` and `crates/nova-cli/src/cmd/mod.rs`.
- Create: `crates/nova-cli/tests/project.rs`, with its first test.

**Interfaces:**
- Consumes: Task 1's `NOVA_TARGET`, `NOVA_EMBEDDED_RUNTIME`,
  `NOVA_EMBEDDED_RUNTIME_CRC32`, `NOVA_EMBEDDED_RUNTIME_SIZE` and
  `NOVA_EMBEDDED_RUNTIME_NAME`. Task 3's `nova_driver::EmbeddedRuntime` and
  `nova_driver::set_embedded_runtime`.
- Produces: `crate::embedded::PAYLOAD: &[u8]` and
  `crate::embedded::runtime() -> nova_driver::EmbeddedRuntime`; the
  `version` subcommand; and `tests/project.rs` with its `nova()` helper,
  which Tasks 6 and 7 extend.

- [ ] **Step 1: Write the failing test**

Create `crates/nova-cli/tests/project.rs`:

```rust
//! End-to-end tests of nova's project model (spec
//! `docs/superpowers/specs/2026-10-07-phase-3-0-foundations-design.md`):
//! `nova version`, project discovery, and `nova new` and `nova init`.
//!
//! Every test works in its own fresh directory under the system temp
//! directory, and writes a `nova.toml` only inside it. Project discovery
//! walks up from the current directory, so a stray `nova.toml` in a shared
//! directory would capture every test run below it.

use assert_cmd::Command;

fn nova() -> Command {
    Command::cargo_bin("nova").expect("nova binary builds")
}

#[test]
fn version_reports_the_build_and_whether_the_runtime_is_embedded() {
    // build.rs exports the payload's size to every target of the package,
    // so this holds whether or not NOVA_EMBED_RUNTIME was set.
    let embedded = env!("NOVA_EMBEDDED_RUNTIME_SIZE") != "0";
    let expected = format!(
        "nova {}\ntarget: {}\nruntime: {}\n",
        env!("CARGO_PKG_VERSION"),
        env!("NOVA_TARGET"),
        if embedded { "embedded" } else { "not embedded" }
    );
    nova().arg("version").assert().success().stdout(expected);
}
```

- [ ] **Step 2: Run it, and watch it fail**

```bash
cd /d/Projects/nona/nova && cargo test --locked -p nova-cli --test project 2>&1 | grep -E "unrecognized subcommand|^test result"
```

Expected: `unrecognized subcommand 'version'` in the captured stderr, then
`1 failed`. If the test does not compile instead, saying `NOVA_TARGET` is
not defined, then `cargo:rustc-env` does not reach integration tests, and
spec §9's claim is wrong. Stop and rule on it in the ledger: the fallback
is a `#[test]` inside `src/cmd/version.rs` that checks `text()` directly.

- [ ] **Step 3: Carry the payload, and add `nova version`**

Create `crates/nova-cli/src/embedded.rs`:

```rust
//! The runtime library this `nova` carries. `build.rs` writes it, gzipped,
//! and names it through these variables (spec
//! `docs/superpowers/specs/2026-10-07-phase-3-0-foundations-design.md`
//! §4.1). An empty payload means this `nova` carries none.

/// The gzip stream `build.rs` wrote; empty when nothing is embedded.
pub const PAYLOAD: &[u8] = include_bytes!(env!("NOVA_EMBEDDED_RUNTIME"));

/// The payload, as nova-driver takes it.
pub fn runtime() -> nova_driver::EmbeddedRuntime {
    nova_driver::EmbeddedRuntime {
        gz: PAYLOAD,
        crc32: env!("NOVA_EMBEDDED_RUNTIME_CRC32")
            .parse()
            .expect("build.rs writes the CRC-32 as a u32"),
        size: env!("NOVA_EMBEDDED_RUNTIME_SIZE")
            .parse()
            .expect("build.rs writes the size as a number"),
        file_name: env!("NOVA_EMBEDDED_RUNTIME_NAME"),
        version: env!("CARGO_PKG_VERSION"),
    }
}
```

Create `crates/nova-cli/src/cmd/version.rs`:

```rust
//! `nova version`: the version, the target `nova` was built for, and
//! whether it carries its runtime library (spec
//! `docs/superpowers/specs/2026-10-07-phase-3-0-foundations-design.md`
//! §4.3). clap's `--version` prints only the first of these.

pub fn run() -> anyhow::Result<()> {
    print!("{}", text(!crate::embedded::PAYLOAD.is_empty()));
    Ok(())
}

fn text(embedded: bool) -> String {
    format!(
        "nova {}\ntarget: {}\nruntime: {}\n",
        env!("CARGO_PKG_VERSION"),
        env!("NOVA_TARGET"),
        if embedded { "embedded" } else { "not embedded" }
    )
}
```

In `crates/nova-cli/src/cmd/mod.rs`, add `pub mod version;` after
`pub mod test;`.

In `crates/nova-cli/src/main.rs`:
- add `mod embedded;` after `mod cmd;`;
- change the module doc's second line, `//! Dispatches to subcommands:
  parse, build, run, fmt, lsp, test, doc, bundle.`, to
  `//! Dispatches to subcommands: parse, run, build, check, test and version.`;
- add this variant at the end of `enum Command`:

```rust
    /// Show the version, the target, and whether the runtime library is
    /// embedded.
    Version,
```

- in `main`, add this line before `let cli = Cli::parse();`:

```rust
    nova_driver::set_embedded_runtime(embedded::runtime());
```

- add this arm at the end of the `match`:

```rust
        Command::Version => cmd::version::run(),
```

- [ ] **Step 4: Run it, and watch it pass**

```bash
cd /d/Projects/nona/nova && cargo test --locked -p nova-cli --test project 2>&1 | grep -E "^test result|FAILED"
```

Expected: `test result: ok. 1 passed; 0 failed`.

- [ ] **Step 5: An embedded runtime links a program with nothing beside nova**

Build a debug `nova` that carries a runtime. Copy it alone into an empty
directory, then build a program with it:

```bash
cd /d/Projects/nona/nova && P=/c/Users/SAKEER~1/AppData/Local/Temp/gcm/p30 && NOVA_EMBED_RUNTIME=1 cargo build --locked -p nova-cli 2>&1 | tail -1 && rm -rf $P/alone && mkdir -p $P/alone/bin $P/alone/home $P/alone/work && cp target/debug/nova.exe $P/alone/bin/ && ls -l target/debug/nova.exe | awk '{print "debug nova.exe with runtime:", $5}'
```

Expected: `Finished`, then the size, which is larger by about the payload.
Ledger it. Then write the program with the Write tool, as
`$P/alone/work/hello.nova`:

```nova
fn main() {
    println("built with the runtime nova carries")
}
```

```bash
cd /c/Users/SAKEER~1/AppData/Local/Temp/gcm/p30/alone/work && env -u NOVA_RUNTIME_LIB NOVA_HOME=../home ../bin/nova.exe version && env -u NOVA_RUNTIME_LIB NOVA_HOME=../home ../bin/nova.exe build hello.nova -o hello.exe && ./hello.exe && ls ../home/runtime/
```

Expected:
- the three version lines, the last `runtime: embedded`;
- `built hello.exe`;
- the program's line;
- one directory, `0.2.0-` followed by eight hex digits.

Then run the end-to-end test against this build. Its expectation follows
the build's own flag:

```bash
cd /d/Projects/nona/nova && NOVA_EMBED_RUNTIME=1 cargo test --locked -p nova-cli --test project 2>&1 | grep -E "^test result"
```

Expected: `1 passed`. Then return to the ordinary debug build:

```bash
cd /d/Projects/nona/nova && cargo build --locked -p nova-cli 2>&1 | tail -1 && ./target/debug/nova.exe version | tail -1
```

Expected: `Finished`, then `runtime: not embedded`.

- [ ] **Step 6: clippy, rustfmt, and commit**

```bash
cd /d/Projects/nona/nova && cargo clippy --locked -p nova-cli --all-targets -- -D warnings 2>&1 | tail -1 && cargo fmt --all && cargo fmt --all -- --check && echo fmt-ok
```

Expected: `Finished` and `fmt-ok`.

Write `$P/msg-4.txt`:

```
nova-cli: carry the runtime library, and add `nova version`

embedded.rs includes the payload build.rs wrote, and main registers it
with nova-driver at startup, so a release-built nova links programs
with nothing beside it.

`nova version` prints the version, the target it was built for, and
whether the runtime is embedded. clap's --version is unchanged.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
```

```bash
cd /d/Projects/nona/nova && git add crates/nova-cli && git commit -q -F /c/Users/SAKEER~1/AppData/Local/Temp/gcm/p30/msg-4.txt && git log -1 --format=%s
```

Expected: `nova-cli: carry the runtime library, and add `nova version``.

---

### Task 5: nova-pm parses nova.toml

**Files:**
- Modify: `Cargo.toml` (workspace): `[workspace.dependencies]` gains `semver`
  and `toml_edit`.
- Modify: `crates/nova-pm/Cargo.toml` and `crates/nova-pm/src/lib.rs`.
- Create: `crates/nova-pm/src/manifest.rs`, `src/name.rs` and
  `src/project.rs`.
- Create: `crates/nova-pm/tests/manifest.rs` and `tests/find_root.rs`.
- Modify: `Cargo.lock`, through cargo.

**Interfaces:**
- Consumes: `nova_diagnostics::{Diagnostic, FileId, Severity, Span,
  FileDb, render::render_to_string}`.
- Produces, from `nova_pm`:
  - `pub fn parse(source: &str, file: FileId) -> (Option<Manifest>,
    Vec<Diagnostic>)`. The manifest is `Some` exactly when no diagnostic
    is an error.
  - `pub struct Manifest { pub package: Package, pub dependencies:
    Vec<Dependency>, pub dev_dependencies: Vec<Dependency> }`.
  - `pub struct Package { pub name: String, pub version: semver::Version,
    pub edition: String, pub description: Option<String>, pub license:
    Option<String>, pub repository: Option<String>, pub authors:
    Vec<String>, pub keywords: Vec<String>, pub categories: Vec<String> }`.
  - `pub struct Dependency { pub name: String, pub version:
    Option<semver::VersionReq>, pub path: Option<PathBuf>, pub span: Span
    }`, where `span` is the entry's key.
  - `pub fn check_name(name: &str) -> Result<(), String>`.
  - `pub fn find_root(start: &Path) -> Option<PathBuf>`.
  - `pub const MANIFEST: &str = "nova.toml"`.

- [ ] **Step 1: Dependencies, and the API as stubs**

In the workspace `Cargo.toml`, after the `crc32fast = "1.4"` line from
Task 1, add:

```toml

# nova.toml (nova-pm). toml_edit's ImDocument keeps every key's and value's
# position, which the M-coded diagnostics need; it was already in the
# lockfile, through toml, and declares Rust 1.66. semver is the crate Cargo
# itself uses for versions and requirements; 1.0.28 declares Rust 1.68 and
# has no dependencies.
semver = "1"
toml_edit = "0.22"
```

Replace `crates/nova-pm/Cargo.toml`'s `[dependencies]` section with:

```toml
[dependencies]
nova-diagnostics = { path = "../nova-diagnostics" }
semver = { workspace = true }
toml_edit = { workspace = true }
```

Replace `crates/nova-pm/src/lib.rs` with:

```rust
//! The package manager's foundations (Phase 3.0): `nova.toml` parsing,
//! package names, and finding a project (spec
//! `docs/superpowers/specs/2026-10-07-phase-3-0-foundations-design.md` §5
//! and §6.1). See ARCHITECTURE.md for the crate's place in the pipeline.

pub mod manifest;
mod name;
mod project;

pub use manifest::{parse, Dependency, Manifest, Package};
pub use name::check_name;
pub use project::{find_root, MANIFEST};
```

Create `crates/nova-pm/src/name.rs`, as a stub:

```rust
//! The rules for a package's name (spec §5.1).

/// Check a package name. Stub: accepts everything.
pub fn check_name(_name: &str) -> Result<(), String> {
    Ok(())
}
```

Create `crates/nova-pm/src/project.rs`, as a stub:

```rust
//! Finding a project (spec §6.1).

use std::path::{Path, PathBuf};

/// A project's manifest file name.
pub const MANIFEST: &str = "nova.toml";

/// Stub: finds nothing.
pub fn find_root(_start: &Path) -> Option<PathBuf> {
    None
}
```

Create `crates/nova-pm/src/manifest.rs` with the real types and a stub
`parse`:

```rust
//! `nova.toml`: what 3.0 reads, and the diagnostics that point into it
//! (spec `docs/superpowers/specs/2026-10-07-phase-3-0-foundations-design.md`
//! §5).

use std::path::PathBuf;

use nova_diagnostics::{Diagnostic, FileId, Span};

/// A parsed `nova.toml`.
#[derive(Debug, Clone, PartialEq)]
pub struct Manifest {
    pub package: Package,
    pub dependencies: Vec<Dependency>,
    pub dev_dependencies: Vec<Dependency>,
}

/// `[package]`.
#[derive(Debug, Clone, PartialEq)]
pub struct Package {
    pub name: String,
    pub version: semver::Version,
    /// Always `"2026"`, the only edition.
    pub edition: String,
    pub description: Option<String>,
    pub license: Option<String>,
    pub repository: Option<String>,
    pub authors: Vec<String>,
    pub keywords: Vec<String>,
    pub categories: Vec<String>,
}

/// One entry of `[dependencies]` or `[dev-dependencies]`: a version
/// requirement or a path, never both (spec §5.1).
#[derive(Debug, Clone, PartialEq)]
pub struct Dependency {
    pub name: String,
    pub version: Option<semver::VersionReq>,
    pub path: Option<PathBuf>,
    /// The entry's key, where a diagnostic about the entry points.
    pub span: Span,
}

/// Stub: reads nothing.
pub fn parse(_source: &str, _file: FileId) -> (Option<Manifest>, Vec<Diagnostic>) {
    (None, Vec::new())
}
```

Update the lockfile. This drops `toml` and adds `semver`:

```bash
cd /d/Projects/nona/nova && cargo build -p nova-pm 2>&1 | tail -1 && git diff Cargo.lock | grep -E '^[+-]name = ' | sort
```

Expected: `Finished`, then `+name = "semver"` and `-name = "toml"`. Also
`-name = "serde_spanned"` if nothing else needs it now. Any other package
means a ruling.

- [ ] **Step 2: Write the failing tests**

Create `crates/nova-pm/tests/manifest.rs`:

```rust
//! `nova.toml` parsing (spec
//! `docs/superpowers/specs/2026-10-07-phase-3-0-foundations-design.md` §5).

use nova_diagnostics::{render, FileDb};
use nova_pm::{parse, Manifest};

/// Parse `source` as a `nova.toml`. Return the manifest, the diagnostics'
/// codes in order, and their rendering.
fn check(source: &str) -> (Option<Manifest>, Vec<String>, String) {
    let mut db = FileDb::new();
    let file = db.add("nova.toml", source);
    let (manifest, diagnostics) = parse(source, file);
    let codes = diagnostics.iter().map(|d| d.code.clone()).collect();
    let rendered = render::render_to_string(&db, &diagnostics);
    (manifest, codes, rendered)
}

const TEMPLATE: &str =
    "[package]\nname = \"demo\"\nversion = \"0.1.0\"\nedition = \"2026\"\n\n[dependencies]\n";

/// A `[package]` table with every required key, `key` set to the TOML
/// value `value`.
fn package_with(key: &str, value: &str) -> String {
    let mut text = String::from("[package]\n");
    for (k, v) in [("name", "\"demo\""), ("version", "\"0.1.0\""), ("edition", "\"2026\"")] {
        let v = if k == key { value } else { v };
        text.push_str(&format!("{k} = {v}\n"));
    }
    text
}

#[test]
fn the_template_parses() {
    let (manifest, codes, _) = check(TEMPLATE);
    assert!(codes.is_empty(), "{codes:?}");
    let manifest = manifest.expect("a manifest");
    assert_eq!(manifest.package.name, "demo");
    assert_eq!(manifest.package.version, semver::Version::new(0, 1, 0));
    assert_eq!(manifest.package.edition, "2026");
    assert!(manifest.dependencies.is_empty());
    assert!(manifest.dev_dependencies.is_empty());
}

#[test]
fn the_optional_keys_are_read() {
    let source = "[package]\nname = \"demo\"\nversion = \"1.2.3-alpha.1\"\n\
                  edition = \"2026\"\ndescription = \"d\"\nlicense = \"MIT\"\n\
                  repository = \"https://example.com/r\"\nauthors = [\"A <a@example.com>\"]\n\
                  keywords = [\"k\"]\ncategories = [\"c\"]\n";
    let (manifest, codes, _) = check(source);
    assert!(codes.is_empty(), "{codes:?}");
    let package = manifest.unwrap().package;
    assert_eq!(package.version.to_string(), "1.2.3-alpha.1");
    assert_eq!(package.description.as_deref(), Some("d"));
    assert_eq!(package.license.as_deref(), Some("MIT"));
    assert_eq!(package.repository.as_deref(), Some("https://example.com/r"));
    assert_eq!(package.authors, ["A <a@example.com>"]);
    assert_eq!(package.keywords, ["k"]);
    assert_eq!(package.categories, ["c"]);
}

#[test]
fn a_missing_package_table_is_m0002_at_line_1_column_1() {
    for source in ["", "[dependencies]\n"] {
        let (manifest, codes, rendered) = check(source);
        assert!(manifest.is_none(), "{source:?}");
        assert_eq!(codes, ["M0002"], "{source:?}");
        assert!(rendered.contains("nova.toml:1:1"), "{rendered}");
    }
}

#[test]
fn each_missing_required_key_is_m0002_at_the_package_header() {
    for (key, source) in [
        ("name", "[package]\nversion = \"0.1.0\"\nedition = \"2026\"\n"),
        ("version", "[package]\nname = \"demo\"\nedition = \"2026\"\n"),
        ("edition", "[package]\nname = \"demo\"\nversion = \"0.1.0\"\n"),
    ] {
        let (manifest, codes, rendered) = check(source);
        assert!(manifest.is_none(), "{key}");
        assert_eq!(codes, ["M0002"], "{key}");
        assert!(rendered.contains(&format!("no `{key}`")), "{rendered}");
        assert!(rendered.contains("nova.toml:1:1"), "{rendered}");
    }
}

#[test]
fn each_broken_rule_is_m0003() {
    let long = format!("\"{}\"", "a".repeat(65));
    let cases = [
        ("name", "\"1abc\"", "must start with an ASCII letter"),
        ("name", "\"a b\"", "contains ' '"),
        ("name", "\"ไทย\"", "must start with an ASCII letter"),
        ("name", "\"aไทย\"", "contains 'ไ'"),
        ("name", long.as_str(), "the limit is 64"),
        ("name", "\"con\"", "reserves for a device"),
        ("name", "\"COM1\"", "reserves for a device"),
        ("name", "\"Lpt9\"", "reserves for a device"),
        // Review Focus 5: names that look like paths.
        ("name", "\"../escape\"", "must start with an ASCII letter"),
        ("name", "\"a/b\"", "contains '/'"),
        ("name", "5", "must be a string"),
        ("version", "\"1.0\"", "is not a version"),
        ("version", "\"1.0.0+build\"", "build metadata"),
        ("edition", "\"2021\"", "unknown edition"),
    ];
    for (key, value, expected) in cases {
        let source = package_with(key, value);
        let (manifest, codes, rendered) = check(&source);
        assert!(manifest.is_none(), "{source}");
        assert_eq!(codes, ["M0003"], "{source}");
        assert!(rendered.contains(expected), "{source}\n{rendered}");
    }
}

#[test]
fn each_dependency_shape_is_read() {
    let source = format!(
        "{TEMPLATE}a = \"1.2\"\nb = {{ version = \"^0.3\" }}\nc = {{ path = \"../c\" }}\n\n\
         [dependencies.d]\nversion = \"*\"\n\n[dev-dependencies]\ne = \"0.1\"\n"
    );
    let (manifest, codes, _) = check(&source);
    assert!(codes.is_empty(), "{codes:?}");
    let manifest = manifest.unwrap();
    let names: Vec<_> = manifest.dependencies.iter().map(|d| d.name.as_str()).collect();
    assert_eq!(names, ["a", "b", "c", "d"]);
    assert_eq!(
        manifest.dependencies[0].version,
        Some(semver::VersionReq::parse("1.2").unwrap())
    );
    assert_eq!(manifest.dependencies[2].version, None);
    assert_eq!(
        manifest.dependencies[2].path,
        Some(std::path::PathBuf::from("../c"))
    );
    assert_eq!(manifest.dev_dependencies[0].name, "e");
}

#[test]
fn an_entry_with_neither_key_is_m0004_and_one_with_both_is_m0003() {
    let (_, codes, _) = check(&format!("{TEMPLATE}x = {{ features = [\"f\"] }}\n"));
    assert_eq!(codes, ["M0006", "M0004"]);
    let (_, codes, _) = check(&format!("{TEMPLATE}x = {{ git = \"https://example.com/x\" }}\n"));
    assert_eq!(codes, ["M0006", "M0004"]);
    let (_, codes, rendered) =
        check(&format!("{TEMPLATE}x = {{ version = \"1\", path = \"../x\" }}\n"));
    assert_eq!(codes, ["M0003"]);
    assert!(rendered.contains("both"), "{rendered}");
}

#[test]
fn a_bad_requirement_is_m0003() {
    let (_, codes, rendered) = check(&format!("{TEMPLATE}x = \"^^1\"\n"));
    assert_eq!(codes, ["M0003"]);
    assert!(rendered.contains("^^1"), "{rendered}");
}

#[test]
fn unknown_keys_warn_at_every_level_and_leave_the_manifest() {
    let source = "[package]\nname = \"demo\"\nversion = \"0.1.0\"\nedition = \"2026\"\n\
                  homepage = \"h\"\n\n[dependencies]\nx = { version = \"1\", optional = true }\n\n\
                  [features]\ndefault = []\n\n[[bin]]\nname = \"b\"\n";
    let (manifest, codes, rendered) = check(source);
    assert_eq!(codes, ["M0006", "M0006", "M0006", "M0006"]);
    assert!(manifest.is_some());
    for key in ["features", "bin", "package.homepage", "dependencies.x.optional"] {
        assert!(rendered.contains(&format!("`{key}`")), "{key}: {rendered}");
    }
}

#[test]
fn invalid_toml_is_m0001_where_parsing_stops() {
    let (manifest, codes, rendered) = check("[package]\nname = \n");
    assert!(manifest.is_none());
    assert_eq!(codes, ["M0001"]);
    assert!(rendered.contains("nova.toml:2:"), "{rendered}");
}

#[test]
fn a_rendered_error_shows_its_line_and_column() {
    let (_, codes, rendered) = check(&package_with("edition", "\"2021\""));
    assert_eq!(codes, ["M0003"]);
    assert!(rendered.contains("nova.toml:4:11"), "{rendered}");
}

/// Review Focus 2.
#[test]
fn a_manifest_with_crlf_line_endings_parses_and_points_right() {
    let (manifest, codes, _) = check(&TEMPLATE.replace('\n', "\r\n"));
    assert!(codes.is_empty(), "{codes:?}");
    assert!(manifest.is_some());
    let crlf = package_with("edition", "\"2021\"").replace('\n', "\r\n");
    let (_, codes, rendered) = check(&crlf);
    assert_eq!(codes, ["M0003"]);
    assert!(rendered.contains("nova.toml:4:11"), "{rendered}");
}

/// Review Focus 2: PowerShell 5.1's `Set-Content -Encoding utf8` writes a
/// byte-order mark.
#[test]
fn a_manifest_saved_with_a_byte_order_mark_parses() {
    let (manifest, codes, _) = check(&format!("\u{feff}{TEMPLATE}"));
    assert!(codes.is_empty(), "{codes:?}");
    assert_eq!(manifest.unwrap().package.name, "demo");
}
```

Create `crates/nova-pm/tests/find_root.rs`:

```rust
//! Finding a project (spec
//! `docs/superpowers/specs/2026-10-07-phase-3-0-foundations-design.md`
//! §6.1).

use std::path::PathBuf;

use nova_pm::{find_root, MANIFEST};

/// A fresh, empty directory under the system temp dir, unique to this test.
/// No `nova.toml` sits above the temp dir on any machine this runs on
/// (checked on 2026-10-07), so `find_root` finds only what a test writes.
fn fresh_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("nova-find-root-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("create the test directory");
    dir
}

#[test]
fn the_manifest_in_the_starting_directory() {
    let dir = fresh_dir("here");
    std::fs::write(dir.join(MANIFEST), "").unwrap();
    assert_eq!(find_root(&dir), Some(dir));
}

#[test]
fn the_manifest_in_an_ancestor() {
    let dir = fresh_dir("ancestor");
    std::fs::write(dir.join(MANIFEST), "").unwrap();
    let deep = dir.join("a").join("b");
    std::fs::create_dir_all(&deep).unwrap();
    assert_eq!(find_root(&deep), Some(dir));
}

#[test]
fn the_nearer_of_two_wins() {
    let dir = fresh_dir("nearer");
    let inner = dir.join("inner");
    std::fs::create_dir_all(inner.join("src")).unwrap();
    std::fs::write(dir.join(MANIFEST), "").unwrap();
    std::fs::write(inner.join(MANIFEST), "").unwrap();
    assert_eq!(find_root(&inner.join("src")), Some(inner));
}

#[test]
fn a_directory_named_nova_toml_does_not_count() {
    let dir = fresh_dir("named-dir");
    std::fs::create_dir_all(dir.join(MANIFEST)).unwrap();
    assert_eq!(find_root(&dir), None);
}

#[test]
fn with_no_manifest_there_is_no_project() {
    assert_eq!(find_root(&fresh_dir("none")), None);
}
```

- [ ] **Step 3: Run the tests, and watch them fail**

```bash
cd /d/Projects/nona/nova && cargo test --locked --no-fail-fast -p nova-pm 2>&1 | grep -E "Running|^test result"
```

`--no-fail-fast` matters here: without it, cargo stops at the first failing
test binary, and the manifest tests never run. Expected:
- `tests/find_root.rs`: `2 passed; 3 failed`. The two that expect `None`
  pass against the stub;
- `tests/manifest.rs`: `0 passed; 13 failed`;
- the library's own unit tests and the doc-tests: `0 passed`.

- [ ] **Step 4: Write `check_name` and `find_root`**

Replace `crates/nova-pm/src/name.rs` with:

```rust
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
```

Replace the stub `find_root` in `crates/nova-pm/src/project.rs` with:

```rust
/// The nearest directory at or above `start` that holds a `nova.toml`
/// file.
pub fn find_root(start: &Path) -> Option<PathBuf> {
    start
        .ancestors()
        .find(|dir| dir.join(MANIFEST).is_file())
        .map(Path::to_path_buf)
}
```

- [ ] **Step 5: Write the parser**

Replace the stub `parse` in `crates/nova-pm/src/manifest.rs`, and the file's
`use` lines, with the following. The types stay as they are.

```rust
use std::ops::Range;
use std::path::PathBuf;

use nova_diagnostics::{Diagnostic, FileId, Severity, Span};
use toml_edit::{ImDocument, Item, Table, TableLike};

use crate::name::check_name;

/// The only edition this nova knows.
pub const EDITION: &str = "2026";

const PACKAGE_KEYS: [&str; 9] = [
    "name",
    "version",
    "edition",
    "description",
    "license",
    "repository",
    "authors",
    "keywords",
    "categories",
];
```

The types go here, unchanged. Then:

```rust
/// Parse `source`, the text of a `nova.toml` that a `FileDb` holds as
/// `file`. The parser keeps positions (toml_edit's `ImDocument`), so every
/// diagnostic carries one. The manifest is returned when no diagnostic is
/// an error; warnings (M0006) leave it in place.
pub fn parse(source: &str, file: FileId) -> (Option<Manifest>, Vec<Diagnostic>) {
    let document = match ImDocument::parse(source) {
        Ok(document) => document,
        Err(error) => {
            let diagnostic = Diagnostic::error("M0001", "nova.toml is not valid TOML")
                .with_primary_label(span(file, error.span()), error.message().trim().to_string());
            return (None, vec![diagnostic]);
        }
    };
    let mut checker = Checker {
        file,
        diagnostics: Vec::new(),
    };
    let manifest = checker.manifest(document.as_table());
    let failed = checker
        .diagnostics
        .iter()
        .any(|diagnostic| diagnostic.severity == Severity::Error);
    (manifest.filter(|_| !failed), checker.diagnostics)
}

fn span(file: FileId, range: Option<Range<usize>>) -> Span {
    let range = range.unwrap_or(0..0);
    Span::new(range.start as u32, range.end as u32, file)
}

struct Checker {
    file: FileId,
    diagnostics: Vec<Diagnostic>,
}

impl Checker {
    fn error(&mut self, code: &str, message: String, at: Option<Range<usize>>, label: &str) {
        let diagnostic =
            Diagnostic::error(code, message).with_primary_label(span(self.file, at), label);
        self.diagnostics.push(diagnostic);
    }

    fn manifest(&mut self, top: &Table) -> Option<Manifest> {
        self.unknown_keys(top, "", &["package", "dependencies", "dev-dependencies"]);
        let package = match top.get("package") {
            None => {
                self.error(
                    "M0002",
                    "nova.toml has no `[package]` table".to_string(),
                    None,
                    "expected `[package]` with `name`, `version` and `edition`",
                );
                None
            }
            Some(item) => match item.as_table_like() {
                Some(table) => self.package(table, item.span()),
                None => {
                    self.error(
                        "M0003",
                        format!("`package` must be a table, not {}", item.type_name()),
                        item.span(),
                        "expected a table",
                    );
                    None
                }
            },
        };
        let dependencies = self.dependencies(top, "dependencies");
        let dev_dependencies = self.dependencies(top, "dev-dependencies");
        Some(Manifest {
            package: package?,
            dependencies,
            dev_dependencies,
        })
    }

    fn package(&mut self, table: &dyn TableLike, at: Option<Range<usize>>) -> Option<Package> {
        self.unknown_keys(table, "package.", &PACKAGE_KEYS);
        let name = self
            .required(table, "name", &at)
            .and_then(|(name, at)| match check_name(&name) {
                Ok(()) => Some(name),
                Err(message) => {
                    self.error("M0003", message, at, "not a valid package name");
                    None
                }
            });
        let version = self
            .required(table, "version", &at)
            .and_then(|(text, at)| self.version(&text, at));
        let edition = self
            .required(table, "edition", &at)
            .and_then(|(edition, at)| {
                if edition == EDITION {
                    Some(edition)
                } else {
                    self.error(
                        "M0003",
                        format!("unknown edition `{edition}`"),
                        at,
                        "this nova supports edition \"2026\"",
                    );
                    None
                }
            });
        let description = self.optional_string(table, "description");
        let license = self.optional_string(table, "license");
        let repository = self.optional_string(table, "repository");
        let authors = self.string_array(table, "authors");
        let keywords = self.string_array(table, "keywords");
        let categories = self.string_array(table, "categories");
        Some(Package {
            name: name?,
            version: version?,
            edition: edition?,
            description,
            license,
            repository,
            authors,
            keywords,
            categories,
        })
    }

    /// A required string key of `[package]`, with its value's position.
    fn required(
        &mut self,
        table: &dyn TableLike,
        key: &str,
        table_at: &Option<Range<usize>>,
    ) -> Option<(String, Option<Range<usize>>)> {
        let Some(item) = table.get(key) else {
            self.error(
                "M0002",
                format!("`[package]` has no `{key}`"),
                table_at.clone(),
                &format!("add `{key} = \"...\"` to this table"),
            );
            return None;
        };
        let value = self.string(item, &format!("package.{key}"))?;
        Some((value, item.span()))
    }

    fn string(&mut self, item: &Item, what: &str) -> Option<String> {
        if let Some(value) = item.as_str() {
            return Some(value.to_string());
        }
        self.error(
            "M0003",
            format!("`{what}` must be a string, not {}", item.type_name()),
            item.span(),
            "expected a string",
        );
        None
    }

    fn optional_string(&mut self, table: &dyn TableLike, key: &str) -> Option<String> {
        let item = table.get(key)?;
        self.string(item, &format!("package.{key}"))
    }

    fn string_array(&mut self, table: &dyn TableLike, key: &str) -> Vec<String> {
        let Some(item) = table.get(key) else {
            return Vec::new();
        };
        let strings = item.as_array().and_then(|array| {
            array
                .iter()
                .map(|value| value.as_str().map(str::to_string))
                .collect::<Option<Vec<_>>>()
        });
        strings.unwrap_or_else(|| {
            self.error(
                "M0003",
                format!("`package.{key}` must be an array of strings"),
                item.span(),
                "expected an array of strings",
            );
            Vec::new()
        })
    }

    fn version(&mut self, text: &str, at: Option<Range<usize>>) -> Option<semver::Version> {
        match semver::Version::parse(text) {
            Ok(version) if version.build.is_empty() => Some(version),
            Ok(_) => {
                self.error(
                    "M0003",
                    format!("package version `{text}` carries build metadata"),
                    at,
                    "a package version takes no `+...` part",
                );
                None
            }
            Err(error) => {
                self.error(
                    "M0003",
                    format!("`{text}` is not a version: {error}"),
                    at,
                    "a version is MAJOR.MINOR.PATCH, such as `0.1.0`",
                );
                None
            }
        }
    }

    fn dependencies(&mut self, top: &Table, key: &str) -> Vec<Dependency> {
        let Some(item) = top.get(key) else {
            return Vec::new();
        };
        let Some(table) = item.as_table_like() else {
            self.error(
                "M0003",
                format!("`{key}` must be a table, not {}", item.type_name()),
                item.span(),
                "expected a table",
            );
            return Vec::new();
        };
        let mut found = Vec::new();
        for (name, entry) in table.iter() {
            let at = table.get_key_value(name).and_then(|(key, _)| key.span());
            if let Some(dependency) = self.dependency(key, name, entry, at) {
                found.push(dependency);
            }
        }
        found
    }

    fn dependency(
        &mut self,
        table: &str,
        name: &str,
        entry: &Item,
        at: Option<Range<usize>>,
    ) -> Option<Dependency> {
        let position = span(self.file, at.clone());
        if let Some(requirement) = entry.as_str() {
            let version = self.requirement(requirement, entry.span())?;
            return Some(Dependency {
                name: name.to_string(),
                version: Some(version),
                path: None,
                span: position,
            });
        }
        let Some(fields) = entry.as_table_like() else {
            self.error(
                "M0003",
                format!(
                    "dependency `{name}` must be a version requirement or a table, not {}",
                    entry.type_name()
                ),
                entry.span(),
                "expected a string or a table",
            );
            return None;
        };
        self.unknown_keys(fields, &format!("{table}.{name}."), &["version", "path"]);
        let version = match fields.get("version") {
            Some(item) => {
                let text = self.string(item, &format!("{table}.{name}.version"))?;
                Some(self.requirement(&text, item.span())?)
            }
            None => None,
        };
        let path = match fields.get("path") {
            Some(item) => Some(PathBuf::from(
                self.string(item, &format!("{table}.{name}.path"))?,
            )),
            None => None,
        };
        match (version, path) {
            (None, None) => {
                self.error(
                    "M0004",
                    format!("dependency `{name}` has neither `version` nor `path`"),
                    at,
                    "add `version = \"...\"` or `path = \"...\"`",
                );
                None
            }
            (Some(_), Some(_)) => {
                self.error(
                    "M0003",
                    format!("dependency `{name}` has both `version` and `path`"),
                    at,
                    "use one of them; what both would mean is not decided yet",
                );
                None
            }
            (version, path) => Some(Dependency {
                name: name.to_string(),
                version,
                path,
                span: position,
            }),
        }
    }

    fn requirement(&mut self, text: &str, at: Option<Range<usize>>) -> Option<semver::VersionReq> {
        match semver::VersionReq::parse(text) {
            Ok(requirement) => Some(requirement),
            Err(error) => {
                self.error(
                    "M0003",
                    format!("`{text}` is not a version requirement: {error}"),
                    at,
                    "a requirement looks like `1.2` or `^0.3`",
                );
                None
            }
        }
    }

    fn unknown_keys(&mut self, table: &dyn TableLike, prefix: &str, known: &[&str]) {
        for (key, item) in table.iter() {
            if known.contains(&key) {
                continue;
            }
            let at = table
                .get_key_value(key)
                .and_then(|(found, _)| found.span())
                .or_else(|| item.span());
            let diagnostic =
                Diagnostic::warning("M0006", format!("unknown key `{prefix}{key}`, ignored"))
                    .with_primary_label(span(self.file, at), "this nova does not read this key");
            self.diagnostics.push(diagnostic);
        }
    }
}
```

- [ ] **Step 6: Run the tests, and watch them pass**

```bash
cd /d/Projects/nona/nova && cargo test --locked --no-fail-fast -p nova-pm 2>&1 | grep -E "Running|^test result|FAILED|panicked" | head -12
```

Expected: `5 passed` for `find_root` and `13 passed` for `manifest`, with
none failed. If a position assertion fails, print the rendering and compare
it with the expected position. Change the expectation only if it was
wrong, and ledger which one was.

- [ ] **Step 7: clippy, rustfmt, commit, and prove the version test bites**

```bash
cd /d/Projects/nona/nova && cargo clippy --locked -p nova-pm --all-targets -- -D warnings 2>&1 | tail -1 && cargo fmt --all && cargo fmt --all -- --check && echo fmt-ok
```

Expected: `Finished` and `fmt-ok`.

Write `$P/msg-5.txt`:

```
nova-pm: parse nova.toml, check package names, find a project

parse() reads [package] (name, version and edition = "2026" required),
[dependencies] and [dev-dependencies] with toml_edit's parser, which
keeps positions, so every diagnostic points into the file:
- M0001: not valid TOML;
- M0002: a missing table or key;
- M0003: a wrong type or a broken rule;
- M0004: a dependency with neither version nor path;
- M0006: an unknown key, a warning.

check_name() holds the name rule. find_root() returns the nearest
directory holding a nova.toml.

nova-pm drops serde, toml, anyhow and tracing, which it never used.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
```

```bash
cd /d/Projects/nona/nova && git add Cargo.toml Cargo.lock crates/nova-pm && git commit -q -F /c/Users/SAKEER~1/AppData/Local/Temp/gcm/p30/msg-5.txt && git log -1 --format=%s
```

Expected: `nova-pm: parse nova.toml, check package names, find a project`.

The mutant: in `manifest.rs`, change `Ok(version) if version.build.is_empty()
=> Some(version),` to `Ok(version) => Some(version),`, and delete the
`Ok(_) => { … }` arm after it. Then:

```bash
cd /d/Projects/nona/nova && cargo test --locked -p nova-pm --test manifest 2>&1 | grep -E "^test .*FAILED|^test result"; git checkout -- crates/nova-pm/src/manifest.rs && git status --short
```

Expected: `each_broken_rule_is_m0003 ... FAILED`, then an empty status.

---

### Task 6: Project mode in run, build, check and test

**Files:**
- Create: `crates/nova-cli/src/project.rs`.
- Modify: `crates/nova-cli/src/cmd/run.rs`, the whole file.
- Modify: `crates/nova-cli/src/cmd/test.rs`: `test_entry_file` and the start
  of `run`, at `:160-170`.
- Modify: `crates/nova-cli/src/main.rs` (`mod project;`) and
  `crates/nova-cli/Cargo.toml` (`nova-pm`).
- Modify: `crates/nova-cli/tests/project.rs`, adding helpers and 12 tests.
- Modify: `Cargo.lock`, through cargo: nova-cli's dependency list.

**Interfaces:**
- Consumes: from Task 5, `nova_pm::find_root(&Path) -> Option<PathBuf>`,
  `nova_pm::parse(&str, FileId) -> (Option<Manifest>, Vec<Diagnostic>)`,
  `nova_pm::MANIFEST`, and `Manifest`, with its `package.name`,
  `dependencies` and `dev_dependencies` (each `Dependency` carrying `name`
  and `span`).
- Produces: in `crate::project`, `pub enum Mode { File(PathBuf), Project {
  entry: PathBuf, target_dir: PathBuf, name: String } }`, with `pub fn
  entry(&self) -> &Path`, and `pub fn mode(file: Option<PathBuf>) ->
  Result<Mode>`. Task 7's tests also rely on the helpers this task adds to
  `tests/project.rs`: `fresh_dir`, `manifest`, `write_project`, `stdout`,
  `stderr`, `built` and `HELLO`.

- [ ] **Step 1: Write the failing tests**

In `crates/nova-cli/tests/project.rs`, add after `use assert_cmd::Command;`:

```rust
use std::env::consts::EXE_SUFFIX;
use std::path::{Path, PathBuf};
```

Then append to the end of the file:

```rust
/// A fresh, empty directory under the system temp dir, unique to this test.
fn fresh_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("nova-project-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("create the test directory");
    dir
}

/// A `nova.toml` for a package called `name`.
fn manifest(name: &str) -> String {
    format!("[package]\nname = \"{name}\"\nversion = \"0.1.0\"\nedition = \"2026\"\n")
}

/// Write a project called `name` into `dir`: its `nova.toml`, with `main`
/// as its `src/main.nova`.
fn write_project(dir: &Path, name: &str, main: &str) {
    std::fs::create_dir_all(dir.join("src")).expect("create src");
    std::fs::write(dir.join("nova.toml"), manifest(name)).expect("write nova.toml");
    std::fs::write(dir.join("src").join("main.nova"), main).expect("write src/main.nova");
}

fn stdout(assert: &assert_cmd::assert::Assert) -> String {
    String::from_utf8_lossy(&assert.get_output().stdout).into_owned()
}

fn stderr(assert: &assert_cmd::assert::Assert) -> String {
    String::from_utf8_lossy(&assert.get_output().stderr).into_owned()
}

/// `<dir>/target/<profile>/<name>`, with the platform's executable suffix.
fn built(dir: &Path, profile: &str, name: &str) -> PathBuf {
    dir.join("target").join(profile).join(format!("{name}{EXE_SUFFIX}"))
}

const HELLO: &str = "fn main() {\n    println(\"hello from the project\")\n}\n";

const WITH_TEST: &str =
    "fn main() {\n    println(\"main\")\n}\n\n@test\nfn passes() {\n    assert_eq(1 + 1, 2)\n}\n";

const PRINT_FIRST_ARG: &str = "fn main() {\n    let argv = args()\n    match argv.get(0) {\n        Some(s) => println(s)\n        None => println(\"no arguments\")\n    }\n}\n";

#[test]
fn every_command_finds_the_project_from_a_subdirectory() {
    let dir = fresh_dir("subdirectory");
    write_project(&dir, "demo", WITH_TEST);
    let sub = dir.join("src");
    nova().current_dir(&sub).arg("run").assert().success().stdout("main\n");
    nova().current_dir(&sub).arg("check").assert().success();
    let tested = nova().current_dir(&sub).arg("test").assert().success();
    assert!(stdout(&tested).contains("1 passed; 0 failed"), "{}", stdout(&tested));
    nova().current_dir(&sub).arg("build").assert().success();
    Command::new(built(&dir, "debug", "demo"))
        .assert()
        .success()
        .stdout("main\n");
}

#[test]
fn build_writes_target_debug_and_the_output_flag_overrides_it() {
    let dir = fresh_dir("build-output");
    write_project(&dir, "demo", HELLO);
    let out = nova().current_dir(&dir).arg("build").assert().success();
    let relative = Path::new("target").join("debug").join(format!("demo{EXE_SUFFIX}"));
    assert_eq!(stdout(&out), format!("built {}\n", relative.display()));
    Command::new(built(&dir, "debug", "demo"))
        .assert()
        .success()
        .stdout("hello from the project\n");
    let custom = dir.join(format!("custom{EXE_SUFFIX}"));
    nova()
        .current_dir(&dir)
        .arg("build")
        .arg("-o")
        .arg(&custom)
        .assert()
        .success();
    Command::new(&custom)
        .assert()
        .success()
        .stdout("hello from the project\n");
}

/// Like `release_builds_and_runs_when_clang_available` in run_tests.rs, this
/// skips when no `clang` is on PATH.
#[test]
fn build_release_writes_target_release() {
    let clang = std::process::Command::new("clang")
        .arg("--version")
        .output()
        .is_ok();
    if !clang {
        eprintln!("skipping: no clang on PATH");
        return;
    }
    let dir = fresh_dir("build-release");
    write_project(&dir, "demo", HELLO);
    nova()
        .current_dir(&dir)
        .args(["build", "--release"])
        .assert()
        .success();
    Command::new(built(&dir, "release", "demo"))
        .assert()
        .success()
        .stdout("hello from the project\n");
}

#[test]
fn paths_are_relative_at_the_root_and_absolute_from_a_subdirectory() {
    let dir = fresh_dir("paths");
    write_project(&dir, "demo", PRINT_FIRST_ARG);
    nova()
        .current_dir(&dir)
        .arg("run")
        .assert()
        .success()
        .stdout("src/main.nova\n");

    let sub = dir.join("src");
    let ran = nova().current_dir(&sub).arg("run").assert().success();
    let entry = PathBuf::from(stdout(&ran).trim_end());
    assert!(entry.is_absolute(), "{}", entry.display());
    assert_eq!(
        entry.canonicalize().unwrap(),
        dir.join("src").join("main.nova").canonicalize().unwrap()
    );

    let out = nova().current_dir(&sub).arg("build").assert().success();
    let printed = stdout(&out);
    let output = PathBuf::from(
        printed
            .trim_end()
            .strip_prefix("built ")
            .expect("`built <path>`"),
    );
    assert!(output.is_absolute(), "{}", output.display());
    assert_eq!(
        output.canonicalize().unwrap(),
        built(&dir, "debug", "demo").canonicalize().unwrap()
    );
}

#[test]
fn a_project_without_src_main_nova_names_the_missing_entry() {
    let dir = fresh_dir("no-entry");
    std::fs::write(dir.join("nova.toml"), manifest("demo")).unwrap();
    let out = nova().current_dir(&dir).arg("run").assert().failure();
    let err = stderr(&out);
    assert!(err.contains("project `demo` has no src/main.nova"), "{err}");
}

/// Passes before project mode exists, and must still pass after it: a file
/// argument never reads the manifest.
#[test]
fn a_file_argument_inside_a_project_ignores_even_a_broken_manifest() {
    let dir = fresh_dir("file-argument");
    std::fs::write(dir.join("nova.toml"), "this is not [ valid toml").unwrap();
    std::fs::write(dir.join("hello.nova"), HELLO).unwrap();
    nova()
        .current_dir(&dir)
        .args(["run", "hello.nova"])
        .assert()
        .success()
        .stdout("hello from the project\n");
    nova()
        .current_dir(&dir)
        .args(["build", "hello.nova"])
        .assert()
        .success();
    assert!(dir.join(format!("hello{EXE_SUFFIX}")).is_file());
}

#[test]
fn a_declared_dependency_is_m0005() {
    let dir = fresh_dir("dependency");
    write_project(&dir, "demo", HELLO);
    let with_dependency = format!("{}\n[dependencies]\nhttp = \"1.0\"\n", manifest("demo"));
    std::fs::write(dir.join("nova.toml"), with_dependency).unwrap();
    let out = nova().current_dir(&dir).arg("run").assert().failure();
    let err = stderr(&out);
    assert!(err.contains("M0005") && err.contains("`http`"), "{err}");
}

#[test]
fn a_manifest_error_shows_its_line_and_column() {
    let dir = fresh_dir("manifest-error");
    write_project(&dir, "demo", HELLO);
    std::fs::write(dir.join("nova.toml"), manifest("demo").replace("2026", "2021")).unwrap();
    let out = nova().current_dir(&dir).arg("build").assert().failure();
    let err = stderr(&out);
    assert!(err.contains("M0003") && err.contains("nova.toml:4:11"), "{err}");
}

#[test]
fn an_unknown_key_warns_and_the_command_proceeds() {
    let dir = fresh_dir("unknown-key");
    write_project(&dir, "demo", HELLO);
    let with_features = format!("{}\n[features]\ndefault = []\n", manifest("demo"));
    std::fs::write(dir.join("nova.toml"), with_features).unwrap();
    let out = nova()
        .current_dir(&dir)
        .arg("run")
        .assert()
        .success()
        .stdout("hello from the project\n");
    let err = stderr(&out);
    assert!(err.contains("M0006") && err.contains("`features`"), "{err}");
}

/// Passes before project mode exists, and must still pass after it.
#[test]
fn outside_any_project_src_main_nova_stays_the_default() {
    let dir = fresh_dir("no-project");
    std::fs::create_dir_all(dir.join("src")).unwrap();
    std::fs::write(dir.join("src").join("main.nova"), HELLO).unwrap();
    nova()
        .current_dir(&dir)
        .arg("run")
        .assert()
        .success()
        .stdout("hello from the project\n");
    nova().current_dir(&dir).arg("build").assert().success();
    assert!(dir.join(format!("main{EXE_SUFFIX}")).is_file());
}

/// Review Focus 1.
#[test]
fn a_project_under_a_path_with_spaces_and_thai_letters_works() {
    let dir = fresh_dir("spaces").join("โปรเจกต์ ของ ฉัน");
    write_project(&dir, "demo", WITH_TEST);
    nova().current_dir(&dir).arg("run").assert().success().stdout("main\n");
    nova().current_dir(&dir).arg("build").assert().success();
    Command::new(built(&dir, "debug", "demo"))
        .assert()
        .success()
        .stdout("main\n");
    let tested = nova().current_dir(&dir).arg("test").assert().success();
    assert!(stdout(&tested).contains("1 passed; 0 failed"), "{}", stdout(&tested));
}

/// Review Focus 3.
#[test]
fn an_import_resolves_when_run_from_a_subdirectory() {
    let dir = fresh_dir("import");
    write_project(
        &dir,
        "demo",
        "import util::{greeting}\n\nfn main() {\n    println(greeting())\n}\n",
    );
    std::fs::write(
        dir.join("src").join("util.nova"),
        "pub fn greeting() -> String {\n    \"from util\"\n}\n",
    )
    .unwrap();
    let docs = dir.join("docs");
    std::fs::create_dir_all(&docs).unwrap();
    nova()
        .current_dir(&docs)
        .arg("run")
        .assert()
        .success()
        .stdout("from util\n");
    nova().current_dir(&docs).arg("build").assert().success();
    Command::new(built(&dir, "debug", "demo"))
        .assert()
        .success()
        .stdout("from util\n");
}
```

- [ ] **Step 2: Run them, and watch the new ones fail**

```bash
cd /d/Projects/nona/nova && cargo build --locked -p nova-runtime 2>&1 | tail -1 && cargo test --locked -p nova-cli --test project 2>&1 | grep -E "^test |^test result"
```

Expected:
- passing: Task 4's version test, plus the two that pin unchanged
  behaviour,
  `a_file_argument_inside_a_project_ignores_even_a_broken_manifest` and
  `outside_any_project_src_main_nova_stays_the_default`;
- `build_release_writes_target_release` also passes when no `clang` is on
  PATH, because it skips;
- every other new test fails.

Ledger the exact counts.

- [ ] **Step 3: Write the project mode**

In `crates/nova-cli/Cargo.toml`, add after
`nova-driver = { path = "../nova-driver" }`:

```toml
nova-pm = { path = "../nova-pm" }
```

In `crates/nova-cli/src/main.rs`, add `mod project;` after `mod embedded;`.

Create `crates/nova-cli/src/project.rs`:

```rust
//! Which file a command works on, and where `build` writes, now that a
//! directory can be a project (spec
//! `docs/superpowers/specs/2026-10-07-phase-3-0-foundations-design.md` §6).

use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};
use nova_diagnostics::{render, Diagnostic, FileDb, Severity};
use nova_pm::Manifest;

/// What a command works on.
pub enum Mode {
    /// One file: a file argument, or `src/main.nova` outside any project.
    File(PathBuf),
    /// A project, found by walking up to its `nova.toml`.
    Project {
        /// `src/main.nova`: relative when the current directory is the
        /// project's root, as it has always been there, and absolute from
        /// anywhere else.
        entry: PathBuf,
        /// `target`, relative or absolute in the same way.
        target_dir: PathBuf,
        /// The package's name, which `build` names its output after.
        name: String,
    },
}

impl Mode {
    /// The source file the command compiles.
    pub fn entry(&self) -> &Path {
        match self {
            Mode::File(file) => file,
            Mode::Project { entry, .. } => entry,
        }
    }
}

/// The mode for a command given `file`, or none. A file argument always
/// means file mode, and no manifest is read. Otherwise the nearest
/// `nova.toml` at or above the current directory makes a project; with
/// none, `src/main.nova` stays the default.
pub fn mode(file: Option<PathBuf>) -> Result<Mode> {
    if let Some(file) = file {
        return Ok(Mode::File(file));
    }
    let cwd = std::env::current_dir().context("reading the current directory")?;
    let Some(root) = nova_pm::find_root(&cwd) else {
        return Ok(Mode::File(PathBuf::from("src/main.nova")));
    };
    // At the root, paths stay relative, exactly as before projects existed.
    let base = if root == cwd {
        PathBuf::new()
    } else {
        root.clone()
    };
    let manifest = read_manifest(&root, &base)?;
    let entry = if base.as_os_str().is_empty() {
        PathBuf::from("src/main.nova")
    } else {
        root.join("src").join("main.nova")
    };
    if !root.join("src").join("main.nova").is_file() {
        bail!(
            "project `{}` has no {}",
            manifest.package.name,
            entry.display()
        );
    }
    Ok(Mode::Project {
        entry,
        target_dir: base.join("target"),
        name: manifest.package.name,
    })
}

/// Read and check `<root>/nova.toml`, rendering its diagnostics. Any error,
/// a declared dependency (M0005) included, stops the command.
fn read_manifest(root: &Path, base: &Path) -> Result<Manifest> {
    let path = root.join(nova_pm::MANIFEST);
    let source =
        std::fs::read_to_string(&path).with_context(|| format!("reading {}", path.display()))?;
    let mut db = FileDb::new();
    let file = db.add(
        base.join(nova_pm::MANIFEST).display().to_string(),
        source.as_str(),
    );
    let (manifest, mut diagnostics) = nova_pm::parse(&source, file);
    if let Some(manifest) = &manifest {
        diagnostics.extend(unresolved_dependencies(manifest));
    }
    if !diagnostics.is_empty() {
        render::emit_all(&db, &diagnostics);
    }
    let errors = diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.severity == Severity::Error)
        .count();
    match manifest {
        Some(manifest) if errors == 0 => Ok(manifest),
        _ => bail!(
            "could not read nova.toml due to {errors} previous error{}",
            if errors == 1 { "" } else { "s" }
        ),
    }
}

/// M0005, one per declared dependency: nothing resolves them before 3.3,
/// and an `import` of one would otherwise fail later, and less clearly
/// (spec §5.2). This is the one place 3.3 removes.
fn unresolved_dependencies(manifest: &Manifest) -> Vec<Diagnostic> {
    manifest
        .dependencies
        .iter()
        .chain(&manifest.dev_dependencies)
        .map(|dependency| {
            Diagnostic::error(
                "M0005",
                format!("dependency `{}` cannot be used yet", dependency.name),
            )
            .with_primary_label(dependency.span, "declared here")
            .with_note("this nova does not resolve dependencies; remove the entry for now")
        })
        .collect()
}
```

Replace `crates/nova-cli/src/cmd/run.rs` with:

```rust
//! `nova run [FILE]` — compile and execute a Nova program,
//! `nova build [FILE]` — compile to a standalone executable, and
//! `nova check [FILE]` — type-check without running.
//!
//! With no FILE, each works on the project around the current directory, or
//! on `src/main.nova` outside any project (`crate::project`).

use std::path::PathBuf;

use anyhow::{Context, Result};
use clap::Args;
use nova_driver::Outcome;

use crate::project::{self, Mode};

#[derive(Args)]
pub struct RunCmd {
    /// The Nova source file to run (default: the project's src/main.nova,
    /// found through the nearest nova.toml; or src/main.nova).
    file: Option<PathBuf>,

    /// Arguments for the program, after `--`: `nova run [FILE] -- ARGS...`.
    /// The program's `args()` is FILE followed by these.
    #[arg(last = true)]
    args: Vec<std::ffi::OsString>,
}

#[derive(Args)]
pub struct CheckCmd {
    /// The Nova source file to check (default: as for `nova run`).
    file: Option<PathBuf>,
}

#[derive(Args)]
pub struct BuildCmd {
    /// The Nova source file to build (default: as for `nova run`).
    file: Option<PathBuf>,

    /// Output executable path (default: target/debug/<name> in a project,
    /// or target/release/<name> with --release; otherwise `<file stem>` in
    /// the current directory; each with the platform executable suffix).
    #[arg(short, long)]
    output: Option<PathBuf>,

    /// Optimizing build via the LLVM backend (emits LLVM IR and compiles it
    /// with a discovered `clang`/`llc`); the default is the fast Cranelift
    /// backend.
    #[arg(long)]
    release: bool,
}

pub fn run(cmd: RunCmd) -> Result<()> {
    let mode = project::mode(cmd.file)?;
    let file = mode.entry();
    let mut args = vec![file.to_string_lossy().into_owned()];
    args.extend(cmd.args.iter().map(|a| a.to_string_lossy().into_owned()));
    match nova_driver::run_file(file, args)? {
        Outcome::Ok(()) => Ok(()),
        Outcome::Failed { errors } => anyhow::bail!(
            "could not compile due to {errors} previous error{}",
            if errors == 1 { "" } else { "s" }
        ),
    }
}

pub fn build(cmd: BuildCmd) -> Result<()> {
    let mode = project::mode(cmd.file)?;
    let output = match (cmd.output, &mode) {
        (Some(output), _) => output,
        (None, Mode::Project { target_dir, name, .. }) => {
            let dir = target_dir.join(if cmd.release { "release" } else { "debug" });
            std::fs::create_dir_all(&dir)
                .with_context(|| format!("creating {}", dir.display()))?;
            dir.join(format!("{name}{}", std::env::consts::EXE_SUFFIX))
        }
        (None, Mode::File(file)) => {
            let stem = file
                .file_stem()
                .map(|s| s.to_string_lossy().into_owned())
                .unwrap_or_else(|| "out".to_string());
            PathBuf::from(format!("{stem}{}", std::env::consts::EXE_SUFFIX))
        }
    };
    let file = mode.entry();
    let built = if cmd.release {
        nova_driver::build_file_release(file, &output)?
    } else {
        nova_driver::build_file(file, &output)?
    };
    match built {
        Outcome::Ok(path) => {
            println!("built {}", path.display());
            Ok(())
        }
        Outcome::Failed { errors } => anyhow::bail!(
            "could not compile due to {errors} previous error{}",
            if errors == 1 { "" } else { "s" }
        ),
    }
}

pub fn check(cmd: CheckCmd) -> Result<()> {
    let mode = project::mode(cmd.file)?;
    let file = mode.entry();
    match nova_driver::check_file(file)? {
        Outcome::Ok(()) => {
            println!("ok: {}", file.display());
            Ok(())
        }
        Outcome::Failed { errors } => {
            anyhow::bail!("found {errors} error{}", if errors == 1 { "" } else { "s" })
        }
    }
}
```

In `crates/nova-cli/src/cmd/test.rs`, delete `test_entry_file` and its doc
comment (`:160-166`). Then change the first two lines of `pub fn run`,

```rust
    let file = test_entry_file();
    let (exe, tests) = nova_driver::build_test_binary(&file)?;
```

to:

```rust
    // `nova test`'s only positional argument is the filter
    // (`nova-spec/40-TOOLING.md:20`: `nova test [filter]`, no `[file]`,
    // unlike `run`/`build`/`check`), so it always works on the project
    // around the current directory, or on `src/main.nova` outside any.
    let mode = crate::project::mode(None)?;
    let (exe, tests) = nova_driver::build_test_binary(mode.entry())?;
```

If the compiler then reports `use std::path::PathBuf;` unused in `test.rs`,
remove that import.

- [ ] **Step 4: Run them, and watch them pass**

The first build updates `Cargo.lock` for `nova-pm`, so it omits `--locked`:

```bash
cd /d/Projects/nona/nova && cargo build -p nova-cli 2>&1 | tail -1 && cargo test --locked -p nova-cli --test project 2>&1 | grep -E "^test result|FAILED"
```

Expected: `test result: ok. 13 passed; 0 failed`.

- [ ] **Step 5: Every existing nova-cli test still passes**

The defaults of `run`, `build`, `check` and `test` changed, so the whole
end-to-end suite is the check. Check port 3000 first.

```bash
cd /d/Projects/nona/nova && P=/c/Users/SAKEER~1/AppData/Local/Temp/gcm/p30 && cargo build --locked --workspace 2>&1 | tail -1 && cargo test --locked -p nova-cli --no-fail-fast > $P/cli-6.txt 2>&1; echo "exit=$?"
```

Count it. Expected: 0 failed.

- [ ] **Step 6: Commit, then prove the paths test bites**

```bash
cd /d/Projects/nona/nova && cargo clippy --locked -p nova-cli --all-targets -- -D warnings 2>&1 | tail -1 && cargo fmt --all && cargo fmt --all -- --check && echo fmt-ok
```

Expected: `Finished` and `fmt-ok`.

Write `$P/msg-6.txt`:

```
nova-cli: run, build, check and test find the project

With no file argument, the four commands walk up to the nearest
nova.toml. They render its diagnostics, refuse a declared dependency
(M0005), and work on <root>/src/main.nova. In a project, build writes
target/debug/<name>, or target/release/<name> with --release.

Paths stay relative at the project root, as before, and are absolute
from anywhere else. A file argument still means exactly that file, with
no manifest read. With no nova.toml, src/main.nova stays the default.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
```

```bash
cd /d/Projects/nona/nova && git add Cargo.lock crates/nova-cli && git commit -q -F /c/Users/SAKEER~1/AppData/Local/Temp/gcm/p30/msg-6.txt && git log -1 --format=%s
```

Expected: `nova-cli: run, build, check and test find the project`.

The mutant: in `project.rs`, change
`let base = if root == cwd {` to `let base = if false {`. Then:

```bash
cd /d/Projects/nona/nova && cargo test --locked -p nova-cli --test project 2>&1 | grep -E "^test .*FAILED|^test result"; git checkout -- crates/nova-cli/src/project.rs && git status --short
```

Expected: `paths_are_relative_at_the_root_and_absolute_from_a_subdirectory
... FAILED`, among others, then an empty status.

---

### Task 7: nova new and nova init

**Files:**
- Create: `crates/nova-cli/src/template.rs` and
  `crates/nova-cli/src/cmd/new.rs`.
- Modify: `crates/nova-cli/src/main.rs`, the whole file, and
  `crates/nova-cli/src/cmd/mod.rs`.
- Modify: `crates/nova-cli/tests/project.rs`, adding 6 tests.

**Interfaces:**
- Consumes: from Task 5, `nova_pm::check_name(&str) -> Result<(),
  String>` and `nova_pm::MANIFEST`. From Task 6, the test helpers
  `fresh_dir`, `stdout`, `stderr` and `HELLO`.
- Produces: `crate::template::files(name: &str) -> [(&'static str, String);
  4]`, in the order `nova.toml`, `.gitignore`, `README.md`,
  `src/main.nova`; and the subcommands `new` and `init`.

- [ ] **Step 1: Write the failing tests**

Append to `crates/nova-cli/tests/project.rs`:

```rust
const TEMPLATE_MAIN: &str = "fn greeting() -> String {\n    \"Hello, Nova!\"\n}\n\nfn main() {\n    println(greeting())\n}\n\n@test\nfn greeting_says_hello() {\n    assert_eq(greeting(), \"Hello, Nova!\")\n}\n";

fn read(path: impl AsRef<Path>) -> String {
    let path = path.as_ref();
    std::fs::read_to_string(path).unwrap_or_else(|e| panic!("reading {}: {e}", path.display()))
}

#[test]
fn new_writes_exactly_the_template_and_it_runs_and_tests() {
    let dir = fresh_dir("new");
    let out = nova()
        .current_dir(&dir)
        .args(["new", "demo"])
        .assert()
        .success();
    assert_eq!(
        stdout(&out),
        "created `demo`: nova.toml, .gitignore, README.md, src/main.nova\n"
    );
    let project = dir.join("demo");
    assert_eq!(
        read(project.join("nova.toml")),
        "[package]\nname = \"demo\"\nversion = \"0.1.0\"\nedition = \"2026\"\n\n[dependencies]\n"
    );
    assert_eq!(read(project.join(".gitignore")), "/target\n");
    assert_eq!(
        read(project.join("README.md")),
        "# demo\n\n`nova run` builds and runs `src/main.nova`; `nova test` runs its tests.\n"
    );
    assert_eq!(read(project.join("src").join("main.nova")), TEMPLATE_MAIN);
    nova()
        .current_dir(&project)
        .arg("run")
        .assert()
        .success()
        .stdout("Hello, Nova!\n");
    let tested = nova().current_dir(&project).arg("test").assert().success();
    assert!(stdout(&tested).contains("1 passed; 0 failed"), "{}", stdout(&tested));
}

#[test]
fn new_refuses_a_non_empty_directory_and_bad_names() {
    let dir = fresh_dir("new-refuses");
    std::fs::create_dir_all(dir.join("taken")).unwrap();
    std::fs::write(dir.join("taken").join("file.txt"), "x").unwrap();
    let out = nova()
        .current_dir(&dir)
        .args(["new", "taken"])
        .assert()
        .failure();
    assert!(stderr(&out).contains("not an empty directory"), "{}", stderr(&out));
    assert!(!dir.join("taken").join("nova.toml").exists());

    // An empty directory is fine.
    std::fs::create_dir_all(dir.join("empty")).unwrap();
    nova()
        .current_dir(&dir)
        .args(["new", "empty"])
        .assert()
        .success();
    assert!(dir.join("empty").join("nova.toml").is_file());

    // Review Focus 5: a bad name is refused before anything touches the
    // disk, names that look like paths included.
    for bad in ["1abc", "a b", "con", "../nova-escape-test", "a/b"] {
        let out = nova().current_dir(&dir).args(["new", bad]).assert().failure();
        let err = stderr(&out);
        assert!(err.contains("package name") || err.contains("reserves"), "{bad}: {err}");
    }
    let mut left: Vec<String> = std::fs::read_dir(&dir)
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    left.sort();
    assert_eq!(left, ["empty", "taken"]);
    assert!(!dir.parent().unwrap().join("nova-escape-test").exists());
}

#[test]
fn init_names_the_project_after_its_directory_and_reports_what_it_wrote() {
    let dir = fresh_dir("init").join("my-app");
    std::fs::create_dir_all(&dir).unwrap();
    let out = nova().current_dir(&dir).arg("init").assert().success();
    assert_eq!(
        stdout(&out),
        "wrote: nova.toml, .gitignore, README.md, src/main.nova\n"
    );
    assert!(read(dir.join("nova.toml")).contains("name = \"my-app\""));
    nova()
        .current_dir(&dir)
        .arg("run")
        .assert()
        .success()
        .stdout("Hello, Nova!\n");
}

#[test]
fn init_keeps_existing_files_and_says_so() {
    let dir = fresh_dir("init-keeps").join("keeper");
    std::fs::create_dir_all(dir.join("src")).unwrap();
    std::fs::write(dir.join("src").join("main.nova"), HELLO).unwrap();
    std::fs::write(dir.join("README.md"), "mine\n").unwrap();
    let out = nova().current_dir(&dir).arg("init").assert().success();
    assert_eq!(
        stdout(&out),
        "wrote: nova.toml, .gitignore\nkept: README.md, src/main.nova\n"
    );
    assert_eq!(read(dir.join("src").join("main.nova")), HELLO);
    assert_eq!(read(dir.join("README.md")), "mine\n");
    nova()
        .current_dir(&dir)
        .arg("run")
        .assert()
        .success()
        .stdout("hello from the project\n");
}

#[test]
fn init_refuses_a_directory_that_already_has_a_manifest() {
    let dir = fresh_dir("init-refuses").join("existing");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("nova.toml"), "# mine\n").unwrap();
    let out = nova().current_dir(&dir).arg("init").assert().failure();
    assert!(stderr(&out).contains("already has a nova.toml"), "{}", stderr(&out));
    assert_eq!(read(dir.join("nova.toml")), "# mine\n");
    assert!(!dir.join("src").exists());
}

#[test]
fn init_suggests_the_name_flag_for_a_bad_directory_name() {
    let dir = fresh_dir("init-bad-name").join("1 bad name");
    std::fs::create_dir_all(&dir).unwrap();
    let out = nova().current_dir(&dir).arg("init").assert().failure();
    assert!(stderr(&out).contains("nova init --name"), "{}", stderr(&out));
    assert!(!dir.join("nova.toml").exists());
    nova()
        .current_dir(&dir)
        .args(["init", "--name", "good"])
        .assert()
        .success();
    assert!(read(dir.join("nova.toml")).contains("name = \"good\""));
}
```

- [ ] **Step 2: Run them, and watch them fail**

```bash
cd /d/Projects/nona/nova && cargo test --locked -p nova-cli --test project 2>&1 | grep -E "^test .*FAILED|^test result"
```

Expected: the six new tests fail, on `unrecognized subcommand 'new'` or
`'init'`, and the 13 earlier ones pass.

- [ ] **Step 3: Write the template and the two commands**

Create `crates/nova-cli/src/template.rs`:

```rust
//! The files `nova new` and `nova init` write (spec
//! `docs/superpowers/specs/2026-10-07-phase-3-0-foundations-design.md`
//! §6.3), with `\n` line endings on every system.

const MAIN: &str = concat!(
    "fn greeting() -> String {\n",
    "    \"Hello, Nova!\"\n",
    "}\n",
    "\n",
    "fn main() {\n",
    "    println(greeting())\n",
    "}\n",
    "\n",
    "@test\n",
    "fn greeting_says_hello() {\n",
    "    assert_eq(greeting(), \"Hello, Nova!\")\n",
    "}\n",
);

/// The template for a project called `name`, as (path, text) pairs. `name`
/// has passed `nova_pm::check_name`, so it needs no TOML escaping.
pub fn files(name: &str) -> [(&'static str, String); 4] {
    [
        (
            "nova.toml",
            format!(
                "[package]\nname = \"{name}\"\nversion = \"0.1.0\"\nedition = \"2026\"\n\n\
                 [dependencies]\n"
            ),
        ),
        (".gitignore", "/target\n".to_string()),
        (
            "README.md",
            format!(
                "# {name}\n\n`nova run` builds and runs `src/main.nova`; `nova test` runs its \
                 tests.\n"
            ),
        ),
        ("src/main.nova", MAIN.to_string()),
    ]
}
```

Create `crates/nova-cli/src/cmd/new.rs`:

```rust
//! `nova new <name>` and `nova init [--name <name>]`: write a new project
//! (spec `docs/superpowers/specs/2026-10-07-phase-3-0-foundations-design.md`
//! §6.3). Neither runs `git init`.

use std::fs;
use std::io::{self, Write};
use std::path::Path;

use anyhow::{anyhow, bail, Context, Result};
use clap::Args;

#[derive(Args)]
pub struct NewCmd {
    /// The project's name, which is also the directory `nova new` creates.
    name: String,
}

#[derive(Args)]
pub struct InitCmd {
    /// The project's name (default: the current directory's name).
    #[arg(long)]
    name: Option<String>,
}

/// `nova new <name>`: the template, in a new directory called `<name>`.
pub fn new(cmd: NewCmd) -> Result<()> {
    nova_pm::check_name(&cmd.name).map_err(|message| anyhow!(message))?;
    let dir = Path::new(&cmd.name);
    if dir.exists() && !is_empty_dir(dir)? {
        bail!("`{}` already exists and is not an empty directory", cmd.name);
    }
    fs::create_dir_all(dir.join("src")).with_context(|| format!("creating {}", dir.display()))?;
    let mut wrote = Vec::new();
    for (path, text) in crate::template::files(&cmd.name) {
        if write_new(&dir.join(path), &text)? {
            wrote.push(path);
        }
    }
    println!("created `{}`: {}", cmd.name, wrote.join(", "));
    Ok(())
}

/// `nova init`: the template, into the current directory. It writes only
/// the files that are missing, and never overwrites one.
pub fn init(cmd: InitCmd) -> Result<()> {
    let cwd = std::env::current_dir().context("reading the current directory")?;
    if cwd.join(nova_pm::MANIFEST).exists() {
        bail!("this directory already has a nova.toml");
    }
    let name = match cmd.name {
        Some(name) => {
            nova_pm::check_name(&name).map_err(|message| anyhow!(message))?;
            name
        }
        None => {
            let name = cwd
                .file_name()
                .map(|name| name.to_string_lossy().into_owned())
                .unwrap_or_default();
            nova_pm::check_name(&name).map_err(|message| {
                anyhow!("{message}; name the project with `nova init --name <name>`")
            })?;
            name
        }
    };
    fs::create_dir_all(cwd.join("src")).context("creating src")?;
    let mut wrote = Vec::new();
    let mut kept = Vec::new();
    for (path, text) in crate::template::files(&name) {
        if write_new(&cwd.join(path), &text)? {
            wrote.push(path);
        } else {
            kept.push(path);
        }
    }
    let wrote = if wrote.is_empty() {
        "nothing".to_string()
    } else {
        wrote.join(", ")
    };
    println!("wrote: {wrote}");
    if !kept.is_empty() {
        println!("kept: {}", kept.join(", "));
    }
    Ok(())
}

fn is_empty_dir(dir: &Path) -> Result<bool> {
    if !dir.is_dir() {
        return Ok(false);
    }
    let mut entries = fs::read_dir(dir).with_context(|| format!("reading {}", dir.display()))?;
    Ok(entries.next().is_none())
}

/// Create `path` holding `text`, unless it already exists. Returns whether
/// it wrote the file.
fn write_new(path: &Path, text: &str) -> Result<bool> {
    match fs::OpenOptions::new().write(true).create_new(true).open(path) {
        Ok(mut file) => {
            file.write_all(text.as_bytes())
                .with_context(|| format!("writing {}", path.display()))?;
            Ok(true)
        }
        Err(error) if error.kind() == io::ErrorKind::AlreadyExists => Ok(false),
        Err(error) => Err(error).with_context(|| format!("creating {}", path.display())),
    }
}
```

In `crates/nova-cli/src/cmd/mod.rs`, add `pub mod new;` after
`pub mod parse;`. Replace `crates/nova-cli/src/main.rs` with:

```rust
//! The `nova` command-line tool.
//!
//! Dispatches to subcommands: parse, run, build, check, test, new, init and
//! version. Phase 0 implemented `nova parse`, Phase 1 `nova run` (Cranelift
//! JIT) and `nova check`, and Phase 3.0 the project commands (spec
//! `docs/superpowers/specs/2026-10-07-phase-3-0-foundations-design.md`).

mod cmd;
mod embedded;
mod project;
mod template;

use anyhow::Result;
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(
    name = "nova",
    about = "The Nova programming language toolchain",
    version
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Parse a Nova source file and print the AST (for debugging).
    Parse(cmd::parse::ParseCmd),
    /// Compile and run a Nova program.
    Run(cmd::run::RunCmd),
    /// Compile a Nova program to a standalone executable.
    Build(cmd::run::BuildCmd),
    /// Type-check a Nova program without running it.
    Check(cmd::run::CheckCmd),
    /// Compile and run `@test` functions, one process per test.
    Test(cmd::test::TestCmd),
    /// Create a new project in a new directory.
    New(cmd::new::NewCmd),
    /// Make the current directory a project.
    Init(cmd::new::InitCmd),
    /// Show the version, the target, and whether the runtime library is
    /// embedded.
    Version,
}

fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive(tracing::Level::WARN.into()),
        )
        .init();

    nova_driver::set_embedded_runtime(embedded::runtime());
    let cli = Cli::parse();
    match cli.command {
        Command::Parse(cmd) => cmd::parse::run(cmd),
        Command::Run(cmd) => cmd::run::run(cmd),
        Command::Build(cmd) => cmd::run::build(cmd),
        Command::Check(cmd) => cmd::run::check(cmd),
        Command::Test(cmd) => cmd::test::run(cmd),
        Command::New(cmd) => cmd::new::new(cmd),
        Command::Init(cmd) => cmd::new::init(cmd),
        Command::Version => cmd::version::run(),
    }
}
```

- [ ] **Step 4: Run them, and watch them pass**

```bash
cd /d/Projects/nona/nova && cargo test --locked -p nova-cli --test project 2>&1 | grep -E "^test result|FAILED"
```

Expected: `test result: ok. 19 passed; 0 failed`.

- [ ] **Step 5: clippy, rustfmt, and commit**

```bash
cd /d/Projects/nona/nova && cargo clippy --locked -p nova-cli --all-targets -- -D warnings 2>&1 | tail -1 && cargo fmt --all && cargo fmt --all -- --check && echo fmt-ok
```

Expected: `Finished` and `fmt-ok`.

Write `$P/msg-7.txt`:

```
nova-cli: nova new and nova init

`nova new <name>` checks the name before touching the disk, refuses a
directory that exists and is not empty, and writes the template:
- nova.toml with name, version 0.1.0, edition 2026 and an empty
  [dependencies];
- .gitignore with /target;
- a README;
- src/main.nova, whose one @test passes.

`nova init [--name <name>]` writes the same into the current directory,
named after it by default. It refuses one that has a nova.toml, writes
only the missing files, and says which it wrote and which it kept.
Neither runs git init.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
```

```bash
cd /d/Projects/nona/nova && git add crates/nova-cli && git commit -q -F /c/Users/SAKEER~1/AppData/Local/Temp/gcm/p30/msg-7.txt && git log -1 --format=%s
```

Expected: `nova-cli: nova new and nova init`.

---

### Task 8: The gate script, and CI's install job

**Files:**
- Create: `.gitattributes`, `.github/scripts/gate.sh`.
- Modify: `.github/workflows/ci.yml`: a new `install` job after `msrv`.

**Interfaces:**
- Consumes: Tasks 1–7. The gate needs all of them.
- Produces: `.github/scripts/gate.sh NOVA WORKDIR [--test]`, which exits 0
  and prints `gate: passed` when `nova` carries its runtime and makes,
  runs, builds and, with `--test`, tests a new project. Task 9's smoke test
  reuses it.

- [ ] **Step 1: Keep shell scripts LF everywhere**

Git for Windows' system config sets `core.autocrlf=true`, so a checkout
would give a shell script CRLF line endings, and bash rejects the carriage
returns. Create `.gitattributes`:

```
# Shell scripts run under bash on every CI runner, Windows included, and
# bash rejects the carriage returns a CRLF checkout would give them.
*.sh text eol=lf
```

- [ ] **Step 2: Write the gate script**

Create `.github/scripts/gate.sh` with the Write tool, which writes LF:

```bash
#!/usr/bin/env bash
# The Phase 3.0 gate (spec
# docs/superpowers/specs/2026-10-07-phase-3-0-foundations-design.md §7 and
# §8): a `nova` with nothing beside it carries its runtime library, and
# makes, runs and builds a new project; with --test it also tests it. CI's
# `install` job and release.yml's smoke test both run this script.
#
# Usage: gate.sh NOVA WORKDIR [--test]
#   NOVA     the nova executable to check
#   WORKDIR  where to make the project; it must not exist yet
set -euo pipefail

if [ $# -lt 2 ] || [ $# -gt 3 ]; then
  echo "usage: gate.sh NOVA WORKDIR [--test]" >&2
  exit 2
fi
nova=$1
work=$2
with_test=${3:-}

if env | grep -q '^NOVA_'; then
  env | grep '^NOVA_' >&2
  echo "gate: a NOVA_ variable is set, and the gate needs none" >&2
  exit 1
fi

mkdir "$work"
cd "$work"

"$nova" version | tee version.txt
if ! grep -qx 'runtime: embedded' version.txt; then
  echo "gate: this nova does not carry its runtime library" >&2
  exit 1
fi

"$nova" new demo
cd demo

ran=$("$nova" run)
if [ "$ran" != "Hello, Nova!" ]; then
  echo "gate: nova run printed '$ran'" >&2
  exit 1
fi

"$nova" build
built=$(./target/debug/demo)
if [ "$built" != "Hello, Nova!" ]; then
  echo "gate: the program nova build wrote printed '$built'" >&2
  exit 1
fi

if [ "$with_test" = "--test" ]; then
  "$nova" test | tee test.txt
  if ! grep -q '1 passed; 0 failed' test.txt; then
    echo "gate: nova test did not pass the template's one test" >&2
    exit 1
  fi
fi
echo "gate: passed"
```

Check the line endings and the `NOVA_` variables of this shell:

```bash
cd /d/Projects/nona/nova && python -X utf8 -c "print(open('.github/scripts/gate.sh','rb').read().count(b'\r'))" && env | grep -c '^NOVA_'
```

Expected: `0` carriage returns. The second count is how many `NOVA_`
variables this shell has. If it is not `0`, run the gate steps below under
`env -u <NAME>` for each, and ledger that.

- [ ] **Step 3: The gate fails a nova that carries nothing**

A copy of the debug `nova`, alone in a directory, carries no runtime:

```bash
cd /d/Projects/nona/nova && P=/c/Users/SAKEER~1/AppData/Local/Temp/gcm/p30 && rm -rf $P/plain $P/gate-red && mkdir -p $P/plain && cp target/debug/nova.exe $P/plain/ && bash .github/scripts/gate.sh $P/plain/nova.exe $P/gate-red; echo "exit=$?"
```

Expected: the three version lines ending `runtime: not embedded`, then
`gate: this nova does not carry its runtime library`, then `exit=1`.

- [ ] **Step 4: Install the branch's nova, measure it, and pass the gate**

A cold install, measured as in Task 1 Step 1. It takes several minutes:

```bash
cd /d/Projects/nona/nova && P=/c/Users/SAKEER~1/AppData/Local/Temp/gcm/p30 && rm -rf $P/after $P/after-target && start=$(date +%s) && cargo install --locked --path crates/nova-cli --root $P/after --target-dir $P/after-target > $P/after-install.log 2>&1; echo "exit=$? seconds=$(( $(date +%s) - start ))"; ls -l $P/after/bin/
```

Expected: `exit=0`, a time, and one file, `nova.exe`. Ledger
`Task 8: install <s> s (baseline <s> s), nova.exe <bytes> bytes (baseline
<bytes>)`. These are the figures spec §11 risks 2 and 3 asked for.

Then run the gate. `USERPROFILE` and `HOME` point into the scratch
directory, so the runtime cache lands there and not in the user's own home.
The gate's own check sees no `NOVA_` variable:

```bash
cd /d/Projects/nona/nova && P=/c/Users/SAKEER~1/AppData/Local/Temp/gcm/p30 && rm -rf $P/home $P/gate-green && mkdir -p $P/home && USERPROFILE="$(cygpath -w $P/home)" HOME="$(cygpath -w $P/home)" bash .github/scripts/gate.sh $P/after/bin/nova.exe $P/gate-green --test; echo "exit=$?"; ls $P/home/.nova/runtime/
```

Expected:
- `runtime: embedded`;
- `created `demo`: …`;
- `built target\debug\demo.exe`;
- the test summary with `1 passed; 0 failed`;
- `gate: passed` and `exit=0`;
- one cache directory, `0.2.0-` followed by eight hex digits.

- [ ] **Step 5: Add the install job to CI**

Append this job to `.github/workflows/ci.yml`, after the `msrv` job, keeping
its two-space indentation under `jobs:`:

```yaml

  # The Phase 3.0 gate (spec
  # docs/superpowers/specs/2026-10-07-phase-3-0-foundations-design.md §8):
  # `nova`, installed with nothing beside it and no NOVA_ variable set,
  # makes, runs, builds and tests a new project. `cargo install` builds with
  # the release profile, so this is also the only job here that runs the
  # nested runtime build (nova-cli's build.rs); the jobs above build debug.
  install:
    name: Installed nova (${{ matrix.os }})
    runs-on: ${{ matrix.os }}
    strategy:
      fail-fast: false
      matrix:
        os: [ubuntu-latest, windows-latest, macos-latest]
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - uses: Swatinem/rust-cache@v2
      - name: Install nova from this checkout
        shell: bash
        run: cargo install --locked --path crates/nova-cli --root "$RUNNER_TEMP/nova"
      - name: Nothing but nova was installed
        shell: bash
        run: |
          set -euo pipefail
          ls -A "$RUNNER_TEMP/nova/bin"
          case "$(ls -A "$RUNNER_TEMP/nova/bin")" in
            nova|nova.exe) ;;
            *) echo "::error::the install put more than nova into bin/"; exit 1 ;;
          esac
      - name: The installed nova makes, runs, builds and tests a project
        shell: bash
        run: bash .github/scripts/gate.sh "$RUNNER_TEMP/nova/bin/nova" "$RUNNER_TEMP/gate" --test
      - name: std survives packaging
        run: cargo package -p nova-std --locked
```

No YAML parser is installed on this host (PyYAML is absent), so the PR's
own CI run is the first syntax check of this file. Re-read the job once
against the `msrv` job above it for indentation.

- [ ] **Step 6: Commit**

Write `$P/msg-8.txt`:

```
ci: install nova and run the Phase 3.0 gate on three systems

The new `install` job runs `cargo install --locked --path
crates/nova-cli` into a temp root and checks that bin/ holds only nova.
.github/scripts/gate.sh then has the installed nova, with no NOVA_
variable set, report its runtime as embedded and make a project with
`nova new`, which `nova run`, `nova build` and `nova test` must pass.
The job also checks that std survives `cargo package`.

.gitattributes keeps *.sh files LF, which bash on the Windows runner
needs.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
```

```bash
cd /d/Projects/nona/nova && git add .gitattributes .github/scripts/gate.sh .github/workflows/ci.yml && git commit -q -F /c/Users/SAKEER~1/AppData/Local/Temp/gcm/p30/msg-8.txt && git log -1 --format=%s && git ls-files --eol .github/scripts/gate.sh
```

Expected: the subject, then `i/lf w/lf attr/text eol=lf`.

---

### Task 9: release.yml publishes a GitHub release with archives

**Files:**
- Create: `.github/scripts/release-notes.sh`.
- Modify: `.github/workflows/release.yml`, the whole file.

**Interfaces:**
- Consumes: Task 8's `gate.sh`.
- Produces: `.github/scripts/release-notes.sh VERSION CHANGELOG`, which
  prints that version's section and fails when it is missing or empty.

- [ ] **Step 1: Watch the notes script fail, because it does not exist**

```bash
cd /d/Projects/nona/nova && bash .github/scripts/release-notes.sh 0.2.0 CHANGELOG.md; echo "exit=$?"
```

Expected: `No such file or directory`, then `exit=127`.

- [ ] **Step 2: Write it**

Create `.github/scripts/release-notes.sh` with the Write tool:

```bash
#!/usr/bin/env bash
# Print CHANGELOG.md's section for one version: the lines after the heading
# that starts with `## [VERSION]`, closing bracket included so that 0.2.0
# never matches 0.2.0-alpha.4, up to the next `## [` heading. Fails when the
# section is missing or empty (spec
# docs/superpowers/specs/2026-10-07-phase-3-0-foundations-design.md §7).
#
# Usage: release-notes.sh VERSION CHANGELOG
set -euo pipefail

if [ $# -ne 2 ]; then
  echo "usage: release-notes.sh VERSION CHANGELOG" >&2
  exit 2
fi
notes=$(awk -v head="## [$1]" '
  index($0, "## [") == 1 { if (found) exit; found = (index($0, head) == 1); next }
  found { print }
' "$2")
if ! printf '%s' "$notes" | grep -q '[^[:space:]]'; then
  echo "release-notes: $2 has no notes for $1" >&2
  exit 1
fi
printf '%s\n' "$notes"
```

- [ ] **Step 3: It picks exactly one section, and refuses an empty one**

```bash
cd /d/Projects/nona/nova && N=.github/scripts/release-notes.sh && bash $N 0.2.0 CHANGELOG.md | grep -m1 -v '^\s*$' && bash $N 0.2.0 CHANGELOG.md | grep -c '^## \[' ; bash $N 0.2.0-alpha.4 CHANGELOG.md | grep -m1 -v '^\s*$' && bash $N 9.9.9 CHANGELOG.md; echo "exit=$?"; bash $N Unreleased CHANGELOG.md; echo "exit=$?"
```

Expected:
- `**Phase 2, "Standard Library Core", is complete**, within the boundary`,
  which is `CHANGELOG.md`'s line 14;
- `0`: no other version's heading came along;
- the first non-blank line under `## [0.2.0-alpha.4]`, which is not the
  same line;
- `release-notes: CHANGELOG.md has no notes for 9.9.9`, then `exit=1`;
- the same refusal for `Unreleased`, whose section is still empty here,
  then `exit=1`.

- [ ] **Step 4: Write the workflow**

Replace `.github/workflows/release.yml` with:

```yaml
name: Release

# On a `v*` tag: build the four targets, pack and smoke-test each archive,
# and publish a GitHub release with the archives and SHA256SUMS. On a pull
# request that changes how `nova` is built or released, run every job but
# the publish, so a broken release shows before any tag exists (spec
# docs/superpowers/specs/2026-10-07-phase-3-0-foundations-design.md §7).
on:
  push:
    tags: ['v*']
  pull_request:
    paths:
      - .github/workflows/release.yml
      - .github/scripts/**
      - crates/nova-cli/build.rs
      - crates/nova-cli/build_support.rs
      - crates/nova-runtime/build.rs
      - crates/nova-cli/Cargo.toml
      - crates/nova-runtime/Cargo.toml
      - Cargo.lock

# Only `publish` may write to the repository.
permissions:
  contents: read

jobs:
  build:
    name: Build (${{ matrix.target }})
    runs-on: ${{ matrix.os }}
    strategy:
      fail-fast: false
      matrix:
        include:
          - os: ubuntu-latest
            target: x86_64-unknown-linux-gnu
            smoke: true
          # Cross-built on an Apple Silicon runner, as before. This design
          # keeps that, so this archive is built but not run.
          - os: macos-latest
            target: x86_64-apple-darwin
            smoke: false
          - os: macos-latest
            target: aarch64-apple-darwin
            smoke: true
          - os: windows-latest
            target: x86_64-pc-windows-msvc
            smoke: true
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
        with:
          targets: ${{ matrix.target }}
      - uses: Swatinem/rust-cache@v2
      - name: Build release binary
        run: cargo build --release --locked --target ${{ matrix.target }} -p nova-cli
      - name: Pack the archive
        shell: bash
        run: |
          set -euo pipefail
          version=$(tr -d '\r' < crates/nova-cli/Cargo.toml | sed -n 's/^version = "\(.*\)"$/\1/p' | head -n 1)
          name="nova-$version-${{ matrix.target }}"
          mkdir -p "dist/$name"
          cp README.md LICENSE-MIT LICENSE-APACHE "dist/$name/"
          if [ "${{ runner.os }}" = "Windows" ]; then
            cp "target/${{ matrix.target }}/release/nova.exe" "dist/$name/"
            (cd dist && 7z a -tzip "$name.zip" "$name" > /dev/null)
            echo "ARCHIVE=dist/$name.zip" >> "$GITHUB_ENV"
          else
            cp "target/${{ matrix.target }}/release/nova" "dist/$name/"
            tar -C dist -czf "dist/$name.tar.gz" "$name"
            echo "ARCHIVE=dist/$name.tar.gz" >> "$GITHUB_ENV"
          fi
      - name: Smoke-test the archive
        if: matrix.smoke
        shell: bash
        run: |
          set -euo pipefail
          unpacked="$RUNNER_TEMP/unpacked"
          mkdir -p "$unpacked"
          case "$ARCHIVE" in
            *.zip) 7z x -o"$unpacked" "$ARCHIVE" > /dev/null ;;
            *) tar -C "$unpacked" -xzf "$ARCHIVE" ;;
          esac
          bash .github/scripts/gate.sh "$(ls -d "$unpacked"/nova-*)/nova" "$RUNNER_TEMP/smoke"
      - uses: actions/upload-artifact@v4
        with:
          name: archive-${{ matrix.target }}
          path: ${{ env.ARCHIVE }}
          if-no-files-found: error

  prepare:
    name: Prepare the release
    needs: build
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: actions/download-artifact@v4
        with:
          pattern: archive-*
          path: release/dist
          merge-multiple: true
      - name: Checksums, notes, and the release command
        shell: bash
        run: |
          set -euo pipefail
          version=$(tr -d '\r' < crates/nova-cli/Cargo.toml | sed -n 's/^version = "\(.*\)"$/\1/p' | head -n 1)
          count=$(ls release/dist | wc -l)
          if [ "$count" -ne 4 ]; then
            echo "::error::expected 4 archives, found $count"
            ls release/dist
            exit 1
          fi
          (cd release/dist && sha256sum * > SHA256SUMS && cat SHA256SUMS)
          bash .github/scripts/release-notes.sh "$version" CHANGELOG.md > release/notes.md
          if [ "$GITHUB_EVENT_NAME" = "push" ]; then
            if [ "$GITHUB_REF_NAME" != "v$version" ]; then
              echo "::error::the tag $GITHUB_REF_NAME does not match nova-cli's version $version"
              exit 1
            fi
            tag=$GITHUB_REF_NAME
          else
            tag="v$version"
          fi
          prerelease=""
          case "$tag" in *-*) prerelease=" --prerelease" ;; esac
          echo "publish runs: gh release create $tag --verify-tag --title \"Nova $version\" --notes-file notes.md$prerelease dist/*"
      - uses: actions/upload-artifact@v4
        with:
          name: release
          path: release
          if-no-files-found: error

  publish:
    name: Publish the release
    needs: prepare
    if: github.event_name == 'push' && startsWith(github.ref, 'refs/tags/v')
    runs-on: ubuntu-latest
    permissions:
      contents: write
    steps:
      - uses: actions/download-artifact@v4
        with:
          name: release
          path: release
      - name: Create the GitHub release
        shell: bash
        env:
          GH_TOKEN: ${{ github.token }}
        run: |
          set -euo pipefail
          version="${GITHUB_REF_NAME#v}"
          prerelease=()
          case "$GITHUB_REF_NAME" in *-*) prerelease=(--prerelease) ;; esac
          gh release create "$GITHUB_REF_NAME" --repo "$GITHUB_REPOSITORY" --verify-tag \
            --title "Nova $version" --notes-file release/notes.md \
            ${prerelease[@]+"${prerelease[@]}"} release/dist/*
```

- [ ] **Step 5: Rehearse the pack and the smoke test locally**

Task 8's installed `nova` stands in for the release build. `tar` stands in
for 7-Zip, which this host lacks:

```bash
cd /d/Projects/nona/nova && P=/c/Users/SAKEER~1/AppData/Local/Temp/gcm/p30 && version=$(tr -d '\r' < crates/nova-cli/Cargo.toml | sed -n 's/^version = "\(.*\)"$/\1/p' | head -n 1) && echo "version=$version" && name="nova-$version-x86_64-pc-windows-msvc" && rm -rf $P/dist $P/unpacked $P/smoke && mkdir -p "$P/dist/$name" $P/unpacked && cp README.md LICENSE-MIT LICENSE-APACHE $P/after/bin/nova.exe "$P/dist/$name/" && tar -C $P/dist -czf "$P/dist/$name.tar.gz" "$name" && tar -C $P/unpacked -xzf "$P/dist/$name.tar.gz" && (cd $P/dist && sha256sum *.tar.gz) && USERPROFILE="$(cygpath -w $P/home)" HOME="$(cygpath -w $P/home)" bash .github/scripts/gate.sh "$(ls -d $P/unpacked/nova-*)/nova" $P/smoke; echo "exit=$?"
```

Expected: `version=0.2.0`, one checksum line, `gate: passed` and `exit=0`.
The four-target build, 7-Zip, the artifact hand-offs and the publish job
are first exercised by the PR's own run of this workflow, and the publish
only by a tag (spec §11 risk 5).

- [ ] **Step 6: Commit**

Write `$P/msg-9.txt`:

```
release: build archives and publish a GitHub release per v* tag

Three jobs:
- build packs each target's nova, README and both licences into
  nova-<version>-<target>.tar.gz (.zip on Windows), and runs gate.sh on
  each archive the runner can execute;
- prepare writes SHA256SUMS, takes the version's CHANGELOG section as
  the notes (release-notes.sh), checks the tag against nova-cli's
  version, and prints the publish command;
- publish, on v* tags only and the only job with contents: write, runs
  `gh release create --verify-tag`. A tag with a '-' makes a
  pre-release.

Pull requests that change how nova is built or released run everything
but publish.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
```

```bash
cd /d/Projects/nona/nova && git add .github/scripts/release-notes.sh .github/workflows/release.yml && git commit -q -F /c/Users/SAKEER~1/AppData/Local/Temp/gcm/p30/msg-9.txt && git log -1 --format=%s
```

Expected: `release: build archives and publish a GitHub release per v* tag`.

---

### Task 10: ADRs 0026 and 0027

**Files:**
- Create: `docs/adr/0026-phase-3-scope.md` and
  `docs/adr/0027-runtime-and-std-in-an-installed-nova.md`.

**Interfaces:**
- Consumes: Task 1's and Task 8's measured sizes, from the ledger.
- Produces: the two ADRs that the code comments and Task 11's records
  already name.

- [ ] **Step 1: Write ADR 0026**

Create `docs/adr/0026-phase-3-scope.md`:

```markdown
# ADR 0026 — Phase 3's scope: a git-backed package index, and what the phase leaves out

## Status

Accepted (2026-10-07). Branch `phase-3-0-foundations`. The decisions are the
user's, made on 2026-10-06 (`docs/phase-3-plan.md` §1, and §3 decision 10).

## Context

`nova-spec/00-MASTER-SPEC.md` §3 lists Phase 3, "Tooling", in eight
positions:

1. the formatter;
2. `nova-pm`, with `nova.toml` and a lock file;
3. a registry server, in a separate repository, on Rust, Postgres and S3;
4. the LSP server;
5. the VSCode extension;
6. `nova doc`;
7. a REPL;
8. a debugger.

Its gate: "External user can `cargo install nova-cli`, init a project, write
code with autocomplete, format, and publish a package."

`nova-spec/40-TOOLING.md` describes more than those positions need. It
includes §4.4's registry at `registry.novalang.dev`, §8's linter, §9's CI
template, and commands such as `nova install` and `nova self update`. No
registry server or domain exists, and running one is an operations
commitment the gate does not require.

## Decision

1. **The registry is a git-backed index,** in place of position 3's
   registry server.
   - A public GitHub repository the user owns holds `40-TOOLING.md` §4.4's
     Cargo-style sparse index: one file per package, and one JSON line per
     version, with its SHA-256.
   - Package tarballs (`.nova-pkg`, §4.6) are attached to that repository
     as release assets.
   - `nova publish` uploads the tarball, and commits the index line through
     GitHub's REST API.
   - The index is read over plain HTTPS, so any static host can mirror it,
     and a hosted registry can serve the same index later.
2. **Phase 3 leaves out:**
   - the REPL (`40-TOOLING.md` §6; position 7) and the debugger
     (position 8);
   - `nova lint` (§8) and `nova bench`;
   - `nova install`, `nova clean`, `nova owner`, and `nova self update` and
     `nova self uninstall` (§1.1, §4.5, §10.3);
   - §9's CI template and its `setup-nova` action;
   - §7's parallel test runs and TAP output, and §4.3's edition rules;
   - the Zed and Neovim extensions (§3.3);
   - the LSP rows §3.1 marks Phase 4 (inlay hints, code lens, call
     hierarchy), and the Phase 4 commands in §1.1 (`build --target wasm`,
     `bundle`, `dev`).
3. **A sub-phase's spec may pull one of these back in,** with a ruling
   that names this ADR.

## Consequences

- **Publishing needs a GitHub token** with write access to the index
  repository, which `nova login` stores. 3.3's spec decides how it is
  stored.
- **The index is append-only:** a published version's line never changes.
  3.3 records the exact rules, yanking included.
- **No server to run.** The costs move to GitHub's API limits and to the
  index repository's size, which 3.3 measures.
- **A REPL and a debugger wait for a later phase.** The master spec's
  Phase 3 list is not amended; this ADR records the difference.
- **Nothing removes old runtime caches** (ADR 0027), because `nova clean`
  and `nova self` are out of this phase.

## References

- `nova-spec/00-MASTER-SPEC.md` §3, Phase 3.
- `nova-spec/40-TOOLING.md` §1.1, §3, §4, and §6 to §10.
- `docs/phase-3-plan.md` §1, and §3 decisions 4 and 10.
- `docs/superpowers/specs/2026-10-07-phase-3-0-foundations-design.md` §10.
```

- [ ] **Step 2: Write ADR 0027**

Create `docs/adr/0027-runtime-and-std-in-an-installed-nova.md`. Fill the four
sizes, and the two install times, from the ledger's Task 1 and Task 8 lines:

```markdown
# ADR 0027 — The runtime library and std inside an installed `nova`

## Status

Accepted (2026-10-07). Branch `phase-3-0-foundations`
(`docs/superpowers/specs/2026-10-07-phase-3-0-foundations-design.md` §3 and
§4; `docs/phase-3-plan.md` §3 decision 1, with its spike result).

## Context

Before Phase 3.0, `nova` worked only beside a checkout:

- **The runtime library.** `nova build` and `nova test` link the runtime's
  staticlib, which `find_runtime_lib` (`crates/nova-driver/src/link.rs`)
  looked for at `NOVA_RUNTIME_LIB`, or in the executable's directory and
  the two above it. `cargo install` installs only the executable.
- **std.** `nova-resolver` embedded std with
  `include_str!("../../../std/...")`. Those paths leave the crate, so a
  packaged `nova-resolver` cannot carry them.

Cargo's artifact dependencies, which would hand `nova-cli` the runtime's
staticlib directly, are unstable. A spike on 2026-10-06, with stand-in
crates, found a route on stable Cargo (the plan's decision 1).

## Decision

1. **std is the crate `nova-std`,** whose package directory is `std/`. Its
   `lib.rs` embeds each `std/*/lib.nova` through a path inside the package,
   so `cargo package` keeps all 17 files, and its verification step builds
   the packaged copy on its own. `nova-resolver` takes the sources from it.
2. **The runtime library is built inside `nova-cli`'s build.**
   - `nova-runtime` declares `links = "nova_runtime"`, and its build script
     reports its source directory. That reaches `nova-cli`'s build script
     as `DEP_NOVA_RUNTIME_MANIFEST_DIR`.
   - `nova-cli`'s build script runs a nested
     `cargo build --lib --release --target $TARGET` into its own `OUT_DIR`.
   - In a workspace (a checkout, `cargo install --path` or
     `cargo install --git`), it builds in place with `--locked`.
   - From a package, it builds a copy without `Cargo.lock`, with an empty
     `[workspace]` table, `--offline`.
   - The runtime manifest's workspace keys choose the route; a file such as
     `Cargo.toml.orig` could be a stray merge backup.
   - The library is gzip-compressed with flate2 and embedded with
     `include_bytes!`. Only release-profile builds embed, unless
     `NOVA_EMBED_RUNTIME` says `1` or `0`.
3. **`nova` finds a runtime library** at `NOVA_RUNTIME_LIB`, then in the
   runtime it carries, then in the executable's directory and the two above
   it.
   - It unpacks the runtime it carries to
     `$NOVA_HOME/runtime/<version>-<crc32>/`. `NOVA_HOME` defaults to
     `.nova` in the home directory.
   - It writes a new temporary file and renames it into place, and it
     checks the file's size and CRC-32 on every use.
   - A runtime it carries but cannot unpack is an error, not a fall-back.
4. **`nova version`** reports whether the runtime is embedded.

## Alternatives

- **Download the library from a GitHub release on first use.** This needs
  a published release, which an unreleased commit lacks, and network
  access at link time. It stays a fallback, if embedding ever fails.
- **Ship the library beside `nova` in release archives.** This covers
  archives, but not `cargo install`, which installs only the executable.
- **Cargo's artifact dependencies** (`-Z bindeps`): unstable.

## Consequences

- **Size.** Measured on this branch: `nova.exe` went from <B> to <A> bytes
  on Windows, and the payload is <W> bytes on Windows and <L> bytes on
  Linux. Decision 1's estimate was a 4.6 MB payload on a 7.2 MB Windows
  `nova`, about two thirds more, and 7.9 MB on an 8.2 MB Linux one, about
  double.
- **Install time.** A release build of `nova-cli` builds the runtime a
  second time, in release mode. A cold `cargo install` took <T1> s before
  and <T2> s after, on the development host.
- **A stale copy can hide in release builds.** A release `nova` links the
  runtime it carries before one beside it. After editing the runtime,
  rebuild `nova-cli` too, whose build script reruns on the runtime's
  sources, or set `NOVA_RUNTIME_LIB`. Rebuilding `nova-runtime` alone is
  not enough.
- **The cache only grows:** one directory per version and build. No Phase
  3 command removes old ones (ADR 0026).
- **The package route first runs for real at 3.6's crates.io install.**
  3.0 tests only its decisions and its copy.

## References

- `docs/phase-3-plan.md` §2 items 1 and 8, and §3 decision 1.
- `docs/superpowers/specs/2026-10-07-phase-3-0-foundations-design.md` §3,
  §4 and §11.
- `crates/nova-cli/build.rs`, `crates/nova-cli/build_support.rs`,
  `crates/nova-driver/src/runtime_cache.rs` and `std/lib.rs`.
```

Replace `<B>`, `<A>`, `<W>`, `<L>`, `<T1>` and `<T2>` with the ledger's
figures, written with thousands separators. Then check that none is left:

```bash
cd /d/Projects/nona/nova && grep -c '<[A-Z][0-9]*>' docs/adr/0027-runtime-and-std-in-an-installed-nova.md
```

Expected: `0`.

- [ ] **Step 3: Commit**

Write `$P/msg-10.txt`:

```
docs: ADR 0026 (Phase 3's scope) and ADR 0027 (runtime and std delivery)

ADR 0026 records the user's decision to replace the master spec's
registry server with a git-backed index, and what Phase 3 leaves out.

ADR 0027 records how std and the runtime library reach an installed nova:
- std is the crate nova-std;
- a nested cargo build embeds the runtime, gzipped;
- the lookup order;
- the per-version cache.

It also gives the alternatives, and the measured size and install-time
costs.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
```

```bash
cd /d/Projects/nona/nova && git add docs/adr/0026-phase-3-scope.md docs/adr/0027-runtime-and-std-in-an-installed-nova.md && git commit -q -F /c/Users/SAKEER~1/AppData/Local/Temp/gcm/p30/msg-10.txt && git log -1 --format=%s
```

Expected: the subject line above.

---

### Task 11: Records, and the sweep

**Files:**
- Modify: `nova-spec/40-TOOLING.md` (four notes), `CHANGELOG.md`,
  `ARCHITECTURE.md`, `README.md`, `docs/phase-3-plan.md` and
  `docs/benchmarks/README.md`.
- Modify: whatever else the sweep in Step 7 finds.

**Interfaces:**
- Consumes: everything above, and the ADR file names from Task 10.
- Produces: no code.

- [ ] **Step 1: Four notes in 40-TOOLING**

In `nova-spec/40-TOOLING.md`, after the closing fence of §1.1's command
block (the block that ends with `nova help [cmd]`), add a blank line and:

```markdown
**Amended 2026-10-07 (branch `phase-3-0-foundations`):** `nova version`,
`nova new <name>` and `nova init` exist. `nova init` also takes
`--name <name>`, and defaults to the directory's own name. With no file
argument, `run`, `build` and `check` find the project by walking up from the
current directory to the nearest `nova.toml`, and so does `test`, which
takes no file; outside any project they keep `src/main.nova`. In a project,
`build` writes `target/debug/<name>`, and `build --release` writes
`target/release/<name>`
(`docs/superpowers/specs/2026-10-07-phase-3-0-foundations-design.md` §6).
```

After the closing fence of §1.2's `nova.toml` block (the one ending
`target = "native"`), add a blank line and:

```markdown
**Amended 2026-10-07 (branch `phase-3-0-foundations`):** `nova new` writes
`nova.toml`, `.gitignore`, `README.md` and `src/main.nova`. Its `nova.toml`
has only `name`, `version = "0.1.0"`, `edition = "2026"` and an empty
`[dependencies]`:
- no `authors` placeholder, which would be published as written;
- no `description`;
- no `[dev-dependencies]` or `[build]`.

There is no `tests/` until Phase 3.3, and the template's one test is in
`src/main.nova` (spec §6.3).
```

After the closing fence of §4.1's manifest block (the one ending
`path = "src/lib.nova"`), add a blank line and:

```markdown
**Amended 2026-10-07 (branch `phase-3-0-foundations`):** `nova-pm` parses
three tables:
- `[package]`: `name`, `version` and `edition` are required; `description`,
  `license`, `repository`, `authors`, `keywords` and `categories` are
  optional;
- `[dependencies]` and `[dev-dependencies]`, whose entries are a version
  requirement, or a table with `version` or `path`.

Every other key warns (M0006) and is ignored. Nothing resolves dependencies
before Phase 3.3, so a declared one is an error (M0005). Diagnostics
M0001–M0006 point at the key or value at fault (spec §5).
```

At the end of §10.1, after the line that begins `Downloads \`nova\` binary`,
add a blank line and:

```markdown
**Amended 2026-10-07 (branch `phase-3-0-foundations`):** `~/.nova` also holds
`runtime/<version>-<crc32>/`, where `nova` unpacks the runtime library it
carries the first time it links a program; `NOVA_HOME` moves the whole
directory. Until the install scripts exist (Phase 3.6),
`cargo install --locked --git https://github.com/Sakeerin/nova nova-cli`
installs a `nova` that needs nothing beside it (ADR 0027).
```

- [ ] **Step 2: The CHANGELOG**

In `CHANGELOG.md`, below `## [Unreleased]` and above `## [0.2.0] -
2026-10-06`, add:

```markdown
### Added

- **Projects.** `nova new <name>` and `nova init [--name <name>]` write a
  project: `nova.toml`, `.gitignore`, `README.md`, and a `src/main.nova`
  whose one test passes. With no file argument, `run`, `build`, `check` and
  `test` find the project by walking up to the nearest `nova.toml`. In a
  project, `build` writes `target/debug/<name>`, or `target/release/<name>`
  with `--release`.
- **`nova.toml`** is parsed by `nova-pm`: `[package]`, where `name`,
  `version` and `edition = "2026"` are required, plus `[dependencies]` and
  `[dev-dependencies]`. Diagnostics point at the key or value at fault
  (M0001–M0004), and unknown keys warn (M0006). Nothing resolves
  dependencies yet, so a declared one is an error (M0005).
- **An installed `nova` needs nothing beside it.**
  - A release-profile build, which is what `cargo install` makes, builds
    the runtime library and embeds it, gzip-compressed.
  - `nova` unpacks it to `~/.nova/runtime/<version>-<crc32>/` the first
    time it links; `NOVA_HOME` moves that directory.
  - `NOVA_RUNTIME_LIB` still comes first, and `NOVA_EMBED_RUNTIME=1` or `0`
    overrides the profile.

  ADR 0027.
- **`nova version`** prints the version, the target `nova` was built for,
  and whether it carries its runtime.
- **Releases.** Each `v*` tag gets a GitHub release with an archive per
  target and `SHA256SUMS`. The archives are smoke-tested on the three
  targets the runners execute. Pull requests that change how `nova` is
  built run the same workflow, without the publish.
- **CI** installs `nova` on all three systems and runs the gate script,
  `.github/scripts/gate.sh`, against it.
- **`std/` is the crate `nova-std`,** so `cargo package` keeps std's
  sources.
- **ADR 0026** records Phase 3's scope.

### Changed

- In a project, `nova build` without `-o` writes `target/debug/<name>`,
  or `target/release/<name>` with `--release`, instead of `<file stem>` in
  the current directory. Outside a project, and with a file argument,
  nothing changes.
- `nova-pm` no longer depends on `serde`, `toml`, `anyhow` or `tracing`.
```

Leave a blank line before `## [0.2.0]`.

- [ ] **Step 3: ARCHITECTURE.md's crate table**

In `ARCHITECTURE.md`'s `## Crates` table:
- after the row `| \`nova-runtime\` | GC, async runtime, panic handling |`,
  add `| \`nova-std\` (in \`std/\`) | The standard library's \`.nova\` sources, embedded for the compiler |`;
- change `| \`nova-pm\` | Package manager |` to
  `| \`nova-pm\` | Package manager: \`nova.toml\` parsing and project discovery so far |`;
- after the row `| \`nova-doc\` | Documentation generator |`, add
  `| \`nova-bench-http\` | Keep-alive HTTP load generator for benchmarking \`std/http\` |`.

The table then lists all 22 crates. Check:

```bash
cd /d/Projects/nona/nova && sed -n '/^## Crates/,/^## Key Design/p' ARCHITECTURE.md | grep -c '^| `nova'
```

Expected: `22`.

- [ ] **Step 4: The README's Install section**

In `README.md`, after the first `---` line (line 6, below the intro's two
quoted lines), add:

````markdown

## Install

```bash
cargo install --locked --git https://github.com/Sakeerin/nova nova-cli
nova new hello
cd hello
nova run
```

`--locked` builds with the dependency versions this repository tests with.
The install also builds Nova's runtime library and embeds it in `nova`, so
the installed `nova` needs nothing beside it. It unpacks the library into
`~/.nova/runtime/` the first time it links a program, and `NOVA_HOME` moves
that directory. From the next release on, each GitHub release also carries
ready-built archives for Linux, macOS and Windows.

---
````

- [ ] **Step 5: The Phase 3 plan**

In `docs/phase-3-plan.md`:

1. At the end of §1's bullet "**Install: git now, crates.io at the
   release.**", which ends `and test without a checkout (decision 1).`, add
   on its own lines, indented two spaces like the bullet's continuation:

   ```markdown
     **Amended 2026-10-07 (branch `phase-3-0-foundations`):** 3.0 adds a
     22nd crate, `nova-std` (`std/`). Its name returned 404 from crates.io's
     index on 2026-10-07. The README recommends `cargo install --locked
     --git …`, which builds with the tested lockfile.
   ```

2. In decision 1's spike result, change `Embedding it compressed roughly
   doubles \`nova\`.` to `Embedding it compressed grows \`nova\` by about two
   thirds on Windows and doubles it on Linux.`

3. In §4's 3.0 entry, after its `- **Gate:** …` bullet, add:

   ```markdown
   - **Spec:** `docs/superpowers/specs/2026-10-07-phase-3-0-foundations-design.md`,
     built on the branch `phase-3-0-foundations`.
   ```

4. In §2, add a note paragraph inside items 1, 2, 3, 8 and 9, after each
   item's last line, indented three spaces:

   ```markdown
      **Amended 2026-10-07 (branch `phase-3-0-foundations`):** resolved by
      3.0. A release-profile `nova` carries its runtime library and unpacks
      it to `$NOVA_HOME/runtime/` (ADR 0027).
   ```

   That is item 1. For item 2: `resolved in \`release.yml\` by 3.0: each
   \`v*\` tag now gets a GitHub release with per-target archives; the next
   tag is its first real run.` For item 3: `3.0 adds \`nova.toml\`,
   \`nova new\`, \`nova init\` and project discovery; dependencies, the lock
   file and \`tests/\` wait for 3.3.` For item 8: `3.0 removes the first
   blocker, because std is now the crate \`nova-std\`; the other two remain
   for 3.6.` For item 9: `\`nova-pm\` now parses \`nova.toml\` (3.0), and
   \`flate2\` and \`semver\` are in \`Cargo.lock\`; \`toml\` no longer is.`

- [ ] **Step 6: The benchmarks README**

In `docs/benchmarks/README.md`, after the paragraph that ends `and one out
of \`target/release/\` links the release-profile one.`, add a blank line
and:

```markdown
**Amended 2026-10-07 (branch `phase-3-0-foundations`):** a release-profile
`nova` now carries its own release runtime, and links it before the one
beside it (ADR 0027), so the profile conclusion above stands. But after
editing the runtime, rebuild `nova-cli` too, or set `NOVA_RUNTIME_LIB`.
Otherwise the server links the copy embedded at nova-cli's last build.
```

- [ ] **Step 7: The sweep, a set difference**

Search the repository for what 3.0 made false, minus the files this branch
already touched:

```bash
cd /d/Projects/nona/nova && P=/c/Users/SAKEER~1/AppData/Local/Temp/gcm/p30 && B=$(git merge-base main HEAD) && git diff --name-only $B > $P/touched.txt && for t in 'include_str!("../../../std' 'find_runtime_lib' 'NOVA_RUNTIME_LIB' 'no `nova.toml`' 'project model' 'one-line `lib.rs`' 'workflow artifacts' 'GitHub release' '21 workspace' '21 crates' 'roughly doubles' 'src/main.nova` in the current directory' 'Dispatches to subcommands'; do echo "== $t"; git grep -n -F "$t" -- . | grep -v -F -f $P/touched.txt; done > $P/sweep.txt; wc -l < $P/sweep.txt
```

Read `$P/sweep.txt` whole, and decide each hit:
- **A statement of a current fact that 3.0 made false:**
  - in a document, add a dated note, worded like the notes above;
  - in a source comment, rewrite the comment in place.
- **A historical record: leave it.** That covers past CHANGELOG entries,
  ADR bodies, the dated plans and specs under `docs/superpowers/`, and
  dated benchmark write-ups such as `docs/benchmarks/http-fixed-response.md`
  and `examples/05-json-api/BENCHMARK.md`.

Ledger one line per hit: the file and line, and `noted`, `rewritten` or
`historical`. `crates/nova-mir/src/lib.rs:742` says std is
`include_str!`'d into the compiler. That is still true, through
`nova-std`; rewrite it only if the sentence names the resolver.

- [ ] **Step 8: Commit**

```bash
cd /d/Projects/nona/nova && git diff --check && git status --short
```

Expected: no whitespace errors, and only the files Steps 1 to 7 changed.

Write `$P/msg-11.txt`:

```
docs: record Phase 3.0 in the specs, the CHANGELOG and the guides

- 40-TOOLING gains dated notes on the new commands and project
  discovery (§1.1), the template's differences (§1.2), the parsed subset
  of nova.toml (§4.1), and the runtime cache under ~/.nova (§10).
- The CHANGELOG's [Unreleased] lists 3.0.
- ARCHITECTURE.md's crate table gains nova-std and nova-bench-http.
- The README gains an Install section.
- The Phase 3 plan points at the spec, corrects the size figure, and
  marks the §2 items 3.0 resolves.
- The benchmarks README warns that a release nova links the runtime it
  carries.

The sweep's decisions are in the ledger.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
```

```bash
cd /d/Projects/nona/nova && git add -A -- nova-spec docs CHANGELOG.md ARCHITECTURE.md README.md crates && git commit -q -F /c/Users/SAKEER~1/AppData/Local/Temp/gcm/p30/msg-11.txt && git log -1 --format=%s
```

Expected: `docs: record Phase 3.0 in the specs, the CHANGELOG and the
guides`.

---

### Task 12: Final verification

**Files:** none changed, unless a step finds a defect. A defect gets its own
commit, with a test that failed first.

**Interfaces:**
- Consumes: the whole branch.
- Produces: the evidence the PR body cites.

- [ ] **Step 1: The full Windows suite**

Check port 3000 first.

```bash
cd /d/Projects/nona/nova && P=/c/Users/SAKEER~1/AppData/Local/Temp/gcm/p30 && cargo build --locked --workspace 2>&1 | tail -1 && cargo build --locked -p nova-runtime 2>&1 | tail -1 && cargo test --locked --workspace --all-features --no-fail-fast > $P/suite-final.txt 2>&1; echo "exit=$?"
```

Count it. Expected: 0 failed. The passed count is Task 1's baseline plus
the tests this branch adds, which this counts:

```bash
cd /d/Projects/nona/nova && git diff $(git merge-base main HEAD) -- '*.rs' | grep -cE '^\+\s*#\[test\]'
```

Expected: `69`, as 12 + 2 + 17 + 1 + 13 + 5 + 19. The ignored count is
unchanged.

- [ ] **Step 2: clippy, rustfmt and MSRV, as CI runs them**

```bash
cd /d/Projects/nona/nova && P=/c/Users/SAKEER~1/AppData/Local/Temp/gcm/p30 && cargo clippy --locked --all-targets --all-features -- -D warnings > $P/clippy-final.txt 2>&1; echo "clippy exit=$?"; cargo fmt --all -- --check && echo fmt-ok; RUSTUP_TOOLCHAIN=1.78 RUSTFLAGS="-D warnings" cargo check --locked --workspace --target-dir target/msrv > $P/msrv-final.txt 2>&1; echo "msrv exit=$?"
```

Expected: `clippy exit=0`, `fmt-ok` and `msrv exit=0`.

- [ ] **Step 3: Linux: the full suite, and the gate on an installed nova**

```bash
cd /d/Projects/nona/nova && P=/c/Users/SAKEER~1/AppData/Local/Temp/gcm/p30 && bash /c/Users/SAKEER~1/AppData/Local/Temp/gcm/linux/run.sh test --locked --workspace --all-features --no-fail-fast > $P/linux-final.txt 2>&1; echo "exit=$?"
```

Count it. Expected: 0 failed. Then install in the container, and run the
gate there. `git show` gives the script's committed, LF bytes:

```bash
cd /d/Projects/nona/nova && P=/c/Users/SAKEER~1/AppData/Local/Temp/gcm/p30 && L=/c/Users/SAKEER~1/AppData/Local/Temp/gcm/linux && bash $L/run.sh install --locked --path crates/nova-cli --root /work/target/inst > $P/linux-install.txt 2>&1; echo "install exit=$?"; git show HEAD:.github/scripts/gate.sh > $L/gate.sh && MSYS_NO_PATHCONV=1 docker run --rm -v nova-linux-target:/work/target -v "C:/Users/SAKEER~1/AppData/Local/Temp/gcm/linux:/probe:ro" rust:1-slim bash /probe/gate.sh /work/target/inst/bin/nova /tmp/gate --test; echo "gate exit=$?"
```

Expected: `install exit=0`, then `runtime: embedded`, the template's run,
build and test, `gate: passed` and `gate exit=0`.

- [ ] **Step 4: The README's route: install from git**

This installs from the branch's committed state through cargo's git source,
the route the README gives. It is another cold release build. It leaves a
clone under `~/.cargo/git/`; list the new entries for the user, and delete
nothing.

```bash
cd /d/Projects/nona/nova && P=/c/Users/SAKEER~1/AppData/Local/Temp/gcm/p30 && ls ~/.cargo/git/checkouts/ > $P/git-before.txt 2>/dev/null; rm -rf $P/git && cargo install --locked --git file:///D:/Projects/nona/nova --branch phase-3-0-foundations nova-cli --root $P/git > $P/git-install.txt 2>&1; echo "exit=$?"; ls ~/.cargo/git/checkouts/ | comm -13 $P/git-before.txt -; rm -rf $P/gate-git && USERPROFILE="$(cygpath -w $P/home)" HOME="$(cygpath -w $P/home)" bash .github/scripts/gate.sh $P/git/bin/nova.exe $P/gate-git --test; echo "gate exit=$?"
```

Expected: `exit=0`, the new checkout directory's name, then `gate: passed`
and `gate exit=0`.

- [ ] **Step 5: The ledger and the PR's figures**

Ledger, in one block:
- the Windows and Linux suite counts, against Task 1's baseline;
- the install times and sizes from Tasks 1 and 8;
- the payload and library sizes from Task 1's Steps 11 and 15;
- the git-install result;
- every ruling made so far.

The PR body below cites them.

---

## The PR

Opened by superpowers:finishing-a-development-branch after the final review,
on the user's choice. The branch also carries the Phase 3 plan's three
commits (`79bc503`, `3e35992` and `897ec91`) and the spec's three, so the
PR says so.

**Title:** `Phase 3.0, Foundations: an installed nova that needs nothing beside it, and projects`

**Body:**

```markdown
## Summary

Phase 3.0, "Foundations"
(`docs/superpowers/specs/2026-10-07-phase-3-0-foundations-design.md`;
plan `docs/superpowers/plans/2026-10-07-phase-3-0-foundations.md`). This
branch also carries the Phase 3 plan (`docs/phase-3-plan.md`), which the
user approved on 2026-10-06.

- **An installed `nova` builds and tests with nothing beside it.**
  nova-cli's build script builds the runtime library with a nested
  `cargo build --release` and embeds it gzip-compressed. nova-driver
  unpacks it to `$NOVA_HOME/runtime/<version>-<crc32>/`. `std/` is now
  the crate `nova-std`. ADR 0027.
- **Projects.** `nova.toml` is parsed by nova-pm, with M-coded
  diagnostics. `nova new` and `nova init` write a template whose test
  passes. `run`, `build`, `check` and `test` find the nearest
  `nova.toml`, and `build` writes `target/debug/<name>`.
- **`nova version`** reports the target, and whether the runtime is
  embedded.
- **Releases.** `release.yml` packs per-target archives, smoke-tests
  them, and publishes a GitHub release with `SHA256SUMS` per `v*` tag.
  This PR runs it without the publish.
- **The gate.** A new CI job installs `nova` and runs
  `.github/scripts/gate.sh` on three systems.
- **ADR 0026** records Phase 3's scope.

## Measurements (development host)

- Cold `cargo install`: <T1> s before, <T2> s after.
- `nova.exe`: <B> bytes before, <A> after.
- Payload: <W> bytes on Windows, <L> bytes on Linux.

## Test plan

- [x] Windows: the full suite, <passed>/0/<ignored>, which is the
      baseline plus the 69 tests this branch adds.
- [x] Linux in Docker: the full suite, 0 failed; the gate on a
      `cargo install`ed nova.
- [x] clippy (`-D warnings`), rustfmt, and MSRV 1.78.
- [x] The gate on Windows, on a cold `cargo install --path` and on a
      `cargo install --git` of this branch.
- [x] A mutant per key test: the route, the std set, no fall-back, build
      metadata, and relative paths.
- [ ] CI: the three `install` legs, and `release.yml`'s four builds
      including the cross-built `x86_64-apple-darwin`, plus `prepare`.

## Not in this PR

Publishing anything, dependency resolution and `tests/` (3.3), the
install scripts (3.6), and every item ADR 0026 leaves out.

🤖 Generated with [Claude Code](https://claude.com/claude-code)
```

Fill the `<…>` figures from the ledger before opening the PR.
