# Phase 3.0, "Foundations": design

- **Date:** 2026-10-07.
- **Status:** the user approved this design in three sections on
  2026-10-06 and 2026-10-07. This written spec awaits the user's review.
- **Branch:** `phase-3-0-foundations`, cut from `phase-3-plan` at `897ec91`.
- **Inputs:** `docs/phase-3-plan.md` §3 (decisions 1, 2 and 10) and §4's
  3.0 entry; `nova-spec/40-TOOLING.md` §1.1, §1.2, §4.1 and §10.

## 1. What 3.0 delivers

After 3.0, a `nova` installed with `cargo install` builds and tests programs
with no checkout beside it, and it knows what a project is.

1. ADR 0026 records Phase 3's scope (decision 10).
2. std becomes a crate, and the runtime library travels inside `nova`
   (decision 1, ADR 0027).
3. `nova-pm` parses `nova.toml` (decision 2).
4. `nova new <name>` and `nova init` write a new project.
5. `run`, `build`, `check` and `test` find the project by walking up to the
   nearest `nova.toml`.
6. `release.yml` creates a GitHub release for each `v*` tag, with archives
   attached.
7. `nova version` reports the version, the target and whether the runtime
   is embedded.

**The gate** (the plan's §4): CI on all three operating systems runs
`cargo install --path crates/nova-cli --root <temp dir>`, which installs
`nova` with nothing beside it. Then `nova new demo`, and `nova run`,
`nova build` and `nova test` inside `demo`, all succeed with no environment
variable set. §8 gives the job's steps.

## 2. The starting point (read on 2026-10-07)

- **The runtime library.** `find_runtime_lib`
  (`crates/nova-driver/src/link.rs:90`) tries `NOVA_RUNTIME_LIB`, then the
  directory holding the `nova` executable and the two directories above it.
  Otherwise it fails, advising `cargo build -p nova-runtime`.
  - `nova-runtime` builds as `["rlib", "staticlib"]`. Its `build.rs`
    compiles `src/gc_stack.c` into `nova_gc_stack`, which both of those
    carry.
  - `nova run` needs no runtime library file: the JIT uses the runtime that
    is linked into `nova` itself. `nova build` and `nova test` link the
    staticlib.
- **std.** `nova-resolver` embeds the 17 `std/*/lib.nova` files with
  `include_str!("../../../std/...")`: `STD_MODULES`, with 16 entries, and
  `STD_TEST_MODULE` (`crates/nova-resolver/src/lib.rs:1612-1645`). Those
  paths leave the crate, so a packaged `nova-resolver` cannot carry them.
- **Dependencies.** `nova-driver` depends on `nova-runtime`. `nova-cli`
  depends on neither `nova-runtime` nor `nova-pm`.
- **`nova-pm`** is one line of rustdoc. Its manifest lists
  `nova-diagnostics`, `serde`, `toml`, `anyhow` and `tracing`.
- **The CLI** (`crates/nova-cli/src/`) has the subcommands `parse`, `run`,
  `build`, `check` and `test`. clap's `version` attribute gives
  `--version`.
  - `run`, `build` and `check` take an optional file, which defaults to
    `src/main.nova`.
  - `build` writes `<file stem>`, with the platform's executable suffix, in
    the current directory unless `-o` names another path.
  - `test` takes only a filter and always compiles `src/main.nova`. Its
    test binary goes under the system temp directory.
- **No project files exist.** No tracked file is a `nova.toml`, including
  under `examples/`.
- **Releases.** `release.yml` builds four targets on each `v*` tag and
  uploads them as workflow artifacts. It creates no GitHub release. Its
  `x86_64-apple-darwin` build is cross-compiled on `macos-latest`, an Apple
  Silicon runner.
- **CI.** `ci.yml` runs `test` and `clippy` on three systems, plus `fmt`,
  and `msrv` (Rust 1.78, `cargo check`). Its `cargo build`, `test`,
  `clippy` and `check` calls all pass `--locked`.
- **The README** says nothing about installing.
- **The template already works.** Given §6.3's `src/main.nova`, today's
  compiler measured, on Windows on 2026-10-07:
  - `nova run` prints `Hello, Nova!`;
  - `nova check` passes;
  - `nova test` reports `1 passed; 0 failed; 0 trapped; 1 total`;
  - `nova build` writes an executable that prints the greeting.

## 3. std as a crate

- `std/` becomes the package `nova-std`, with:
  - a `std/Cargo.toml` (version `0.2.0` like the other crates, and the
    workspace's edition, rust-version, license and repository);
  - a `std/lib.rs`, named by `[lib] path = "lib.rs"`.

  The workspace's `members` gains `"std"`.
- `std/lib.rs` embeds each module through a path inside its own package,
  such as `include_str!("core/lib.nova")`, and exports the sources.
- `nova-resolver` depends on `nova-std`. `STD_MODULES` and
  `STD_TEST_MODULE` keep their names, their `$std.*` module names and their
  order. Only the place each source comes from changes.
- A `nova-std` test lists `std/*/lib.nova` on disk and checks that each file
  is embedded, and that nothing else is. A std module missing from
  `lib.rs` therefore fails that test.
- CI runs `cargo package -p nova-std --locked`, whose verification step
  builds the packaged copy on its own (§8).
- `nova-std` makes the workspace 22 crates. Its name returned 404 from
  crates.io's index on 2026-10-07.

## 4. The runtime inside `nova`

### 4.1 Building and embedding it

- **The runtime reports its source.** `nova-runtime` declares
  `links = "nova_runtime"`, and its build script also prints
  `cargo:manifest_dir=<its CARGO_MANIFEST_DIR>`. Cargo hands that to the
  crates that depend on it directly, as `DEP_NOVA_RUNTIME_MANIFEST_DIR`.
- **`nova-cli`** gains a direct dependency on `nova-runtime` (only direct
  dependents see `DEP_` variables) and a build script.
- **When to embed:**
  - `NOVA_EMBED_RUNTIME=1` embeds and `NOVA_EMBED_RUNTIME=0` does not.
  - When the variable is unset, the build script embeds if `PROFILE` is
    `release`. That covers `cargo install` and `release.yml`.
  - Any other value fails the build, naming the variable.

  So, unless the variable says otherwise, debug builds skip the nested
  build. That includes CI's test, clippy and MSRV jobs, which build debug.
- **The nested build:**
  - The command is
    `$CARGO build --lib --release --target $TARGET --target-dir $OUT_DIR/rt`.
  - `CARGO_TARGET_DIR`, `CARGO_BUILD_TARGET`, `RUSTFLAGS` and
    `CARGO_ENCODED_RUSTFLAGS` are removed from its environment.
  - Its target directory is never the outer build's, whose lock the outer
    build holds.
  - `--target` is always passed, so the cross-compiled macOS build gets a
    runtime for its own target, not the runner's.
- **Two cases:**
  - **In a workspace,** where the runtime's directory has no
    `Cargo.toml.orig`: `cargo package` writes that file into every
    package, and a workspace member lacks it. Here the nested build runs in
    place, with `--manifest-path <runtime>/Cargo.toml --locked`. The
    workspace's lockfile applies, and `--locked` stops the nested build
    from ever rewriting it.

    This case covers a checkout (`cargo build`, `cargo install --path`). It
    also covers `cargo install --git`, whose source is a checkout of the
    repository.
  - **From a package,** where `Cargo.toml.orig` is present, as it is for a
    crate from crates.io. Here the build script copies the runtime's
    directory into `$OUT_DIR/rt-src`, leaving out `Cargo.lock` and
    `target`, and builds the copy `--offline`. This is the spike's route:
    an offline resolution chooses only among crates already downloaded,
    and those include every version the outer build fetched.
- **Why the workspace case is not `--offline`.** `cargo install` ignores the
  lockfile unless it is given `--locked` (cargo-install's documentation,
  "Dealing with the Lockfile").
  - A `cargo install --git ...` without `--locked` can therefore download
    newer versions than the repository's `Cargo.lock` pins. An offline
    nested build that honours that lockfile would then fail on the first
    pinned version missing from the cache.
  - Without `--offline`, cargo fetches only what is missing. A `cargo build`
    in a checkout, which uses the workspace's lockfile, or an install with
    `--locked`, fetches nothing.

  The README recommends `--locked` anyway (§10).
- **The payload:**
  - The build script gzips the library with flate2, at its best
    compression, into `$OUT_DIR`.
  - It passes the payload's path, the library's CRC-32 and its
    uncompressed size to the crate through `cargo:rustc-env`.
  - The crate always applies `include_bytes!` to that path. When it is not
    embedding, the build script writes an empty file, and an empty payload
    means "not embedded".
  - The library's file name follows the target, not the build host:
    `nova_runtime.lib` for MSVC targets, `libnova_runtime.a` otherwise.
- **Reruns:** `cargo:rerun-if-changed` on the runtime's directory, and
  `cargo:rerun-if-env-changed=NOVA_EMBED_RUNTIME`.

### 4.2 Finding the library at link time

`find_runtime_lib` tries, in order:

1. `NOVA_RUNTIME_LIB`, unchanged. A path that does not exist is still an
   error.
2. The embedded runtime, if there is one, unpacked to the cache below.
3. The executable's directory and the two above it, unchanged.

The embedded runtime comes before the executable's neighbours, so a `nova`
that carries its runtime never links a stray older library that happens to
sit beside it.

- **Who owns what.** `nova-cli` hands the payload to `nova-driver` once, at
  startup. `nova-driver` keeps it and owns the unpacking. The order itself
  is a pure function, tested without touching the process environment.
- **The cache** is `$NOVA_HOME/runtime/<version>-<crc32>/<library name>`.
  - `NOVA_HOME` defaults to `.nova` in the home directory: `USERPROFILE` on
    Windows, `HOME` elsewhere. That is the directory `40-TOOLING.md` §10.1's
    installer uses.
  - `<version>` is `nova-cli`'s version.
  - `<crc32>` is the library's CRC-32 as eight hex digits, so two builds
    of one version whose libraries differ get different files.
- **Unpacking:**
  - If the file exists with the right size and CRC-32, `nova` uses it.
  - Otherwise `nova` decompresses into a temporary file beside it, named
    with the process id, and renames that into place.
  - If the rename fails but the file is now there and verifies, another
    process won the race. `nova` uses that file and deletes its temporary
    one.
  - A damaged copy fails verification and is replaced.
- **Failures** name the path that failed and suggest `NOVA_HOME` or
  `NOVA_RUNTIME_LIB`. When neither home variable is set, the error says so.

### 4.3 `nova version`

`nova version` is a new subcommand, which `40-TOOLING.md` §1.1 lists. clap's
`--version` stays as it is. The subcommand prints three lines:

```
nova 0.2.0
target: x86_64-pc-windows-msvc
runtime: embedded
```

The last line reads `runtime: not embedded` when the payload is empty. The
target is the triple `nova` was built for, `TARGET` in the build script.

## 5. `nova.toml`

### 5.1 What is parsed

- **`[package]`** is a required table.
- **`name`** is a required string:
  - ASCII letters, digits, `-` and `_`, starting with a letter;
  - at most 64 characters;
  - not a name Windows reserves for a device: `con`, `prn`, `aux`, `nul`,
    `com1` to `com9` and `lpt1` to `lpt9`, in any case. The name becomes a
    directory and an executable.
- **`version`** is a required string, a semver 2.0 version, checked by
  `semver::Version`.
- **`edition`** is a required string, and `"2026"` is the only edition.
- **`description`, `license` and `repository`** are optional strings.
- **`authors`, `keywords` and `categories`** are optional arrays of strings.
- **`[dependencies]` and `[dev-dependencies]`** are optional tables of
  entries.
  - An entry is a requirement string, such as `"1.2"`, or a table with
    `version`, `path`, or both.
  - A requirement must parse as `semver::VersionReq`, the requirement syntax
    Cargo uses (`40-TOOLING.md` §4.3 asks for Cargo's semantics).
  - 3.3 decides what an entry with both keys means.
- **Any other key or table** is reported with a warning and ignored. That
  includes `[features]`, `[build]`, `[[bin]]` and `[lib]`, and a `git` or
  `features` key inside an entry. A `{ git = "..." }` entry therefore warns,
  then fails for having neither `version` nor `path`.

### 5.2 Diagnostics

`nova-pm` reads the manifest with toml_edit's parser that keeps positions
(`ImDocument`, in toml_edit 0.22.27, already in `Cargo.lock`). Every
diagnostic therefore carries a position: the key or value at fault, the
table a required key is missing from, or the place the TOML stops parsing.
They are ordinary `nova-diagnostics` diagnostics, rendered like source
errors, with a new prefix, `M`:

| Code  | Severity | Meaning                                            |
|-------|----------|----------------------------------------------------|
| M0001 | error    | not valid TOML                                     |
| M0002 | error    | a required table or key is missing                 |
| M0003 | error    | a value has the wrong type, or breaks its rule     |
| M0004 | error    | a dependency has neither `version` nor `path`      |
| M0005 | error    | a dependency is declared; none resolve before 3.3  |
| M0006 | warning  | an unknown key or table, ignored                   |

M0005 covers both `[dependencies]` and `[dev-dependencies]`. Warnings
print, and the command goes on. Any error stops the command before it
compiles anything.

M0005 is raised by the CLI, not by the parser. So 3.3 removes it in one
place, and `nova-pm`'s parser stays the lasting API.

## 6. Projects

### 6.1 Finding the project

- `run`, `build` and `check` without a file argument, and `test` always (it
  takes no file), look for `nova.toml` in the current directory, then in
  each parent, nearest first.
- **Found:** project mode.
  - The directory holding the manifest is the root.
  - The manifest is parsed (§5).
  - The entry is `<root>/src/main.nova`. A missing entry is an error that
    names that path.
- **Not found:** today's behaviour, with `src/main.nova` relative to the
  current directory.
- **A file argument** means file mode, exactly as today. No manifest is
  read, even inside a project, and `build` writes `<file stem>` in the
  current directory.

### 6.2 Outputs and paths

- In project mode:
  - `nova build` writes `<root>/target/debug/<name>`;
  - `nova build --release` (the LLVM backend) writes
    `<root>/target/release/<name>`.

  Each gets the platform's executable suffix, and the directories are
  created as needed. `-o` overrides both.
- Project mode produces two paths: the entry, which the program's `args()`
  starts with, and the file `build` prints. Both are relative when the
  current directory is the root, as today (`src/main.nova`), and absolute
  from anywhere else.
- `nova test`'s binary stays in the system temp directory.

### 6.3 `nova new` and `nova init`

- **`nova new <name>`:**
  - checks the name against §5.1's rules before touching the disk;
  - refuses if `<name>` exists and is not an empty directory;
  - writes the template into `<name>/`.
- **`nova init [--name <name>]`** writes into the current directory.
  - The name defaults to the directory's own name. If that name breaks the
    rules, the error suggests `--name`.
  - It refuses a directory that already has a `nova.toml`.
  - It writes each template file that does not exist yet, never
    overwrites one, and prints which files it wrote and which it kept.
  - `--name` is an addition to `40-TOOLING.md` §1.1's bare `nova init`.
- Neither command runs `git init`.
- **The template**, with `demo` as the name:

  `nova.toml`:

  ```toml
  [package]
  name = "demo"
  version = "0.1.0"
  edition = "2026"

  [dependencies]
  ```

  `.gitignore`:

  ```
  /target
  ```

  `README.md`:

  ```markdown
  # demo

  `nova run` builds and runs `src/main.nova`; `nova test` runs its tests.
  ```

  `src/main.nova`:

  ```nova
  fn greeting() -> String {
      "Hello, Nova!"
  }

  fn main() {
      println(greeting())
  }

  @test
  fn greeting_says_hello() {
      assert_eq(greeting(), "Hello, Nova!")
  }
  ```

- Every file is written with `\n` line endings, on every system.
- **How it differs from `40-TOOLING.md` §1.2's template, on purpose:**
  - it has no `authors` placeholder, which would be published as written
    (`cargo new` also stopped writing `authors`; RFC 3052);
  - it has no `description`, and no `[dev-dependencies]` or `[build]`
    section;
  - it has no `tests/`: the test lives in `src/main.nova` until 3.3 brings
    `tests/`.

## 7. `release.yml`

`release.yml` has three jobs, so that only the last can write to the
repository. The workflow's default permission is `contents: read`.

1. **`build`**, for each of the four targets:
   - runs `cargo build --release --locked --target <target> -p nova-cli`;
   - packs `nova`, `README.md`, `LICENSE-MIT` and `LICENSE-APACHE` into
     `nova-<version>-<target>.tar.gz`, or a `.zip` for Windows. Here
     `<version>` is the version in `crates/nova-cli/Cargo.toml`;
   - smoke-tests the archive on the three targets the runner can execute:
     1. unpack it into an empty directory;
     2. `nova version` must say `runtime: embedded`;
     3. `nova new demo`;
     4. inside `demo`, `nova run` and the executable `nova build` writes
        must each print `Hello, Nova!`.

     `nova build` is the step that uses the embedded runtime; `nova run`
     does not. `x86_64-apple-darwin` is cross-built on an Apple Silicon
     runner, so it is built but not run;
   - uploads the archive.
2. **`prepare`:**
   - gathers the four archives and writes `SHA256SUMS`;
   - extracts the release notes, the CHANGELOG section headed
     `## [<version>]`, and fails if that section is empty;
   - on a tag, checks that the tag is `v<version>`;
   - uploads all of it.
3. **`publish`** runs on a `v*` tag only, with `contents: write`.
   - It runs `gh release create <tag> --verify-tag`, titled
     `Nova <version>`, with the notes and every file `prepare` uploaded.
   - A tag containing `-`, such as `v0.3.0-rc.1`, makes a pre-release.

The workflow also runs on pull requests that change `release.yml`, either
build script, `crates/nova-cli/Cargo.toml`, `crates/nova-runtime/Cargo.toml`
or `Cargo.lock`. On a pull request, `publish` is skipped. So 3.0's own PR
builds the embedded runtime for all four targets, including the cross-built
one the spike never tried, and runs it on three, before any tag exists.

## 8. The CI gate

A new `ci.yml` job, `install`, runs on ubuntu, windows and macos, in bash on
all three:

1. `cargo install --locked --path crates/nova-cli --root "$RUNNER_TEMP/nova"`.
2. Fail if any `NOVA_` variable is set, or if `$RUNNER_TEMP/nova/bin` holds
   anything but `nova`.
3. In a new, empty directory, `nova version` must print `runtime: embedded`.
   Then `nova new demo`.
4. Inside `demo`:
   - `nova run` must print `Hello, Nova!`;
   - after `nova build`, `target/debug/demo` must print it too;
   - `nova test` must report `1 passed; 0 failed`.
5. `cargo package -p nova-std --locked` (§3).

## 9. Testing

TDD throughout: each test is watched failing before the code it tests
exists.

- **`nova-pm` unit tests:**
  - the template's manifest parses;
  - each required table and key, missing (M0002);
  - each rule broken (M0003): a name that starts with a digit, one with a
    space, one of 65 characters, `con`; the version `1.0`; the edition
    `2021`; a value of the wrong type;
  - each dependency shape, accepted;
  - an entry with neither key (M0004);
  - a bad requirement (M0003);
  - unknown keys at each level (M0006);
  - invalid TOML (M0001);
  - a rendered error showing `nova.toml:<line>:<column>`.
- **Discovery unit tests:** the manifest is in the starting directory; it
  is in an ancestor; the nearer of two wins; there is none.
- **Unpacking unit tests**, with a small fake payload so that debug builds
  cover them:
  - the first use writes the file;
  - a second use reuses it, untouched;
  - a damaged copy is replaced;
  - two threads unpacking at once both get a verified file;
  - a `NOVA_HOME` that is a file gives an error that names it;
  - the lookup order, as a pure function.
- **`nova-std`:** the embedded set matches the files on disk (§3).
- **End to end,** in a new `crates/nova-cli/tests/project.rs` rather than
  the 9,650-line `run_tests.rs`:
  - `new` writes exactly the template, and `run` prints the greeting;
  - `new` refuses a non-empty directory and a bad name;
  - `init` names the project after its directory, keeps an existing
    `src/main.nova`, refuses an existing `nova.toml`, and suggests `--name`
    for a bad directory name;
  - all four commands work from a subdirectory;
  - `build` writes `target/debug/<name>`, and `-o` overrides it;
  - a file argument inside a project ignores even a broken manifest;
  - a declared dependency gives M0005;
  - a manifest error shows its line and column;
  - an unknown key warns, and the command proceeds;
  - outside any project, `src/main.nova` stays the default;
  - `nova version` in a debug build says `runtime: not embedded`.
- **Not unit-tested: the build script.** The gate (§8) and `release.yml`'s
  pull-request run (§7) are its tests. The plan's first task also builds it
  with `NOVA_EMBED_RUNTIME=1`, on Windows and in the Linux container, before
  anything depends on it.

## 10. Records

- **ADR 0026, Phase 3's scope:** decision 10, as the plan words it.
- **ADR 0027, the runtime and std in an installed `nova`:**
  - §3 and §4 of this spec;
  - the alternatives: downloading the library from a GitHub release,
    shipping it beside `nova` in archives, and Cargo's unstable artifact
    dependencies;
  - the spike's result;
  - the costs (§11).
- **`40-TOOLING.md` notes:**
  - §1.1: `version`, `new`, and `init --name`;
  - §1.2: how the template differs (§6.3);
  - §4.1: the parsed subset (§5);
  - §10: the cache under `~/.nova`.
- **CHANGELOG:** `[Unreleased]`.
- **ARCHITECTURE.md:** its crate list gains `nova-std`, and `nova-pm` is no
  longer a stub.
- **README.md** gains an Install section: `cargo install --locked --git
  https://github.com/Sakeerin/nova nova-cli`, and the archives on GitHub
  releases from the next tag on.
- **`docs/phase-3-plan.md`:** 3.0's entry points at this spec, and the
  plan's count of 21 crates gains `nova-std`.
- **The sweep:** `git grep` for what 3.0 makes false, minus the files the
  branch already touched. That covers:
  - `include_str!` paths into `std/`;
  - `find_runtime_lib`'s two-step order;
  - "no `nova.toml`";
  - `nova-pm` described as a stub;
  - `release.yml` uploading artifacts only.

## 11. Risks

1. **The in-place nested build is new.** The spike built stand-ins, never
   the real runtime inside this workspace. The plan's first task proves it
   on Windows and on Linux before anything depends on it.
2. **Install time.** A release build of `nova-cli` now also builds the
   runtime and its dependencies in release mode, unless
   `NOVA_EMBED_RUNTIME=0`. The spike did not measure this; the plan records
   `cargo install` times before and after.
3. **Size.** `nova` roughly doubles. Decision 1's figures: a 4.6 MB payload
   against a 7.2 MB Windows `nova`, and 7.9 MB against an 8.2 MB Linux one.
   The plan records the real sizes.
4. **The cache only grows.** It gains one directory per version and build,
   and nothing removes old ones, because `nova clean` is out of scope
   (ADR 0026).
5. **`gh release create` first runs for real on a tag,** and pushing a tag
   is the user's call. The pull-request run covers every step before it.
6. **macOS.** The gate is the first time the nested build runs on macOS.
   `release.yml`'s pull-request run is the first time it cross-compiles.
7. **Path length on Windows.** The nested build sits deep inside the outer
   `OUT_DIR`, and C compilers on Windows can fail on long paths. The plan's
   first task watches for this.
8. **CI time.** The `install` job is a cold release build on three systems,
   until its cache warms.

## 12. Not in 3.0

- Publishing anything, and versions on path dependencies (3.6).
- Resolving dependencies, `nova.lock`, `tests/`, and `nova new --lib`
  (3.3).
- `nova.toml` files for `examples/`. `60-EXAMPLES.md` asks for full
  projects (`nova.toml`, `src/`, `tests/` and `README.md`), which `tests/`
  completes in 3.3.
- Install scripts (`40-TOOLING.md` §10.1 and §10.2; 3.6), `nova clean` and
  `nova self ...` (ADR 0026).
- `[features]`, `[build]`, `[[bin]]`, `[lib]`, git dependencies and
  workspaces.

## 13. Decisions made while writing this spec

The three approved sections did not settle these. Each is open to the
user's review.

1. The workspace case of the nested build uses `--locked` without
   `--offline` (§4.1), so that `cargo install --git` without `--locked`
   works.
2. A package is told apart from a workspace member by its
   `Cargo.toml.orig`.
3. New dependencies: `flate2`, with `miniz_oxide` and `adler2`, both new to
   the lockfile; and `semver`. `crc32fast` and `toml_edit` are already in
   the lockfile. flate2 1.1.10 declares Rust 1.67 and semver 1.0.28
   declares 1.68. miniz_oxide and adler2 declare no minimum, so the MSRV
   job is their check.
4. The `M` diagnostic codes (§5.2), with M0005 raised by the CLI.
5. Name rules beyond the approved ones: ASCII only, at most 64 characters,
   and no Windows device names.
6. Paths are relative at the project root and absolute elsewhere (§6.2).
7. `release.yml`: three jobs, a smoke test that also runs `nova build`, and
   the extra paths in the pull-request trigger.
8. `cargo package -p nova-std` runs in the gate job.
9. The examples stay without `nova.toml` until 3.3.
10. `nova new` runs no `git init`, and `nova init` writes only the files
    that are missing.
