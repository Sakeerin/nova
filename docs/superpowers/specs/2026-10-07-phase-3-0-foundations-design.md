# Phase 3.0, "Foundations": design

- **Date:** 2026-10-07.
- **Status:** the user approved this design in three sections on
  2026-10-06 and 2026-10-07. This written spec awaits the user's review.
- **Branch:** `phase-3-0-foundations`, cut from `phase-3-plan` at `897ec91`.
- **Inputs:** `docs/phase-3-plan.md` §3 (decisions 1, 2 and 10) and §4's
  3.0 entry; `nova-spec/40-TOOLING.md` §1.1, §1.2, §4.1 and §10.

## 1. What 3.0 delivers

After 3.0, a `nova` installed with `cargo install`, which builds with the
release profile unless told otherwise (§4.1), builds and tests programs with
no checkout beside it, and it knows what a project is.

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
variable set. §8 gives the job's steps and makes this precise: it installs
with `--locked`, like CI's other cargo builds, and it checks that no `NOVA_`
variable is set. CI itself sets others, such as `CARGO_TERM_COLOR`.

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
- **Dependencies.** `nova-runtime` is a normal dependency of one crate,
  `nova-codegen-cranelift`, which `nova-driver` depends on; `nova-driver`
  lists `nova-runtime` itself only under `[dev-dependencies]`. `nova-cli`
  depends directly on neither `nova-runtime` nor `nova-pm`.
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
  - a `std/Cargo.toml`: version `0.2.0` like the other crates; the
    workspace's edition, rust-version, license and repository; and a
    `description`, which every crate has and `cargo package` warns
    without;
  - a `std/lib.rs`, named by `[lib] path = "lib.rs"`, and `bench = false`
    under `[lib]`, as in all 21 crates. Without it, `benchmarks.yml`'s
    `cargo bench --workspace -- --output-format bencher` fails: libtest's
    harness rejects `--output-format`.

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
  `cargo:manifest_dir=<its CARGO_MANIFEST_DIR>`. Cargo passes that, as
  `DEP_NOVA_RUNTIME_MANIFEST_DIR`, to the build scripts of the packages
  that depend on `nova-runtime`. Its documentation says that metadata
  reaches only immediate dependents.
- **`nova-cli`** gains `nova-runtime` as a normal `[dependencies]` entry,
  as the spike's stand-in had it, so that its new build script receives
  that variable.
- **When to embed:**
  - `NOVA_EMBED_RUNTIME=1` embeds and `NOVA_EMBED_RUNTIME=0` does not.
  - When the variable is unset, the build script embeds if `PROFILE` is
    `release`. That covers `cargo install` and `release.yml`.
  - Any other value fails the build, naming the variable.

  So, unless the variable says otherwise, debug builds skip the nested
  build. That includes CI's test, clippy and MSRV jobs, which build debug.
  It also means `cargo install --debug`, or a profile that inherits from
  `dev`, gives a `nova` without its runtime. `nova build` and `nova test`
  then fail with an error that says how to get one (§4.2).
- **The nested build:**
  - The command is
    `$CARGO build --lib --release --target $TARGET --target-dir $OUT_DIR/rt`.
  - `CARGO_TARGET_DIR`, `CARGO_BUILD_TARGET`, `RUSTFLAGS` and
    `CARGO_ENCODED_RUSTFLAGS` are removed from its environment.
  - Its target directory is never the outer build's, whose lock the outer
    build holds.
  - `--target` is always passed, so the cross-compiled macOS build gets a
    runtime for its own target, not the runner's.
- **Two cases, told apart by the runtime's own manifest:**
  - **In a workspace,** where the manifest inherits from the workspace: it
    has `.workspace = true` keys, as `nova-runtime`'s does. `cargo package`
    always resolves those away, so a packaged manifest never has them.

    Checking for a file would be weaker. `cargo package` writes
    `Cargo.toml.orig` into every package, but a stray backup of that name
    could sit in a checkout: `git mergetool` leaves `*.orig` files, and
    `.gitignore` hides them.

    Here the nested build runs in place, with
    `--manifest-path <runtime>/Cargo.toml --locked`. The workspace's
    lockfile applies, and `--locked` stops the nested build from ever
    rewriting it.

    This case covers a checkout (`cargo build`, `cargo install --path`). It
    also covers `cargo install --git`, whose source is a checkout of the
    repository.
  - **From a package,** where the manifest has no such keys, as for a crate
    from crates.io. Here the build script:
    1. copies the runtime's directory into `$OUT_DIR/rt-src`, leaving out
       `Cargo.lock` and `target`;
    2. appends an empty `[workspace]` table to the copy's manifest, so cargo
       never takes the copy for a member of a workspace that happens to
       surround `OUT_DIR`;
    3. builds the copy `--offline`.

    This is the spike's route. An offline resolution chooses only among
    crates already downloaded, and those include every version the outer
    build fetched.
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
- **Reruns.** The build script reruns when anything that goes into the
  library changes:
  - the runtime's directory. `cargo:rerun-if-changed` on a directory scans
    all of it, so this covers `build.rs` and `gc_stack.c` too;
  - each local source file named in the dep-info file the nested build
    writes beside the library. That covers path dependencies such as
    `nova-diagnostics`. Today's debug `libnova_runtime.d` names 18 files of
    `nova-runtime` and 3 of `nova-diagnostics`, and no registry crate;
  - in the workspace case, the lockfile, which pins the registry versions
    the dep-info file leaves out;
  - `NOVA_EMBED_RUNTIME`, through `cargo:rerun-if-env-changed`.

  Without these, a reinstall could embed a stale runtime while `nova run`
  uses the fresh one: `cargo install --path` reuses the workspace's target
  directory.

### 4.2 Finding the library at link time

`find_runtime_lib` tries, in order:

1. `NOVA_RUNTIME_LIB`, unchanged. A path that does not exist is still an
   error.
2. The embedded runtime, if there is one, unpacked to the cache below.
3. The executable's directory and the two above it, unchanged.

When a runtime is embedded, failing to unpack it is an error. Step 3 is
tried only when nothing is embedded. With the embedded runtime before the
executable's neighbours, a `nova` that carries its runtime never links a
stray older library that happens to sit beside it.

When nothing is embedded and step 3 finds nothing, the error says how to get
a `nova` that carries its runtime: install it with the release profile,
which is `cargo install`'s default, or build it with
`NOVA_EMBED_RUNTIME=1`. Setting `NOVA_RUNTIME_LIB` also works.

- **Who owns what.** `nova-cli` hands the payload to `nova-driver` once, at
  startup. `nova-driver` keeps it and owns the unpacking. The order itself
  is a pure function, tested without touching the process environment.
- **The cache** is `$NOVA_HOME/runtime/<version>-<crc32>/<library name>`.
  - `NOVA_HOME` defaults to `.nova` in the home directory: `USERPROFILE` on
    Windows, `HOME` elsewhere. That is the directory `40-TOOLING.md` §10.1's
    installer uses.
  - `<version>` is `nova-cli`'s version.
  - `<crc32>` is the library's CRC-32 as eight hex digits. Two builds of
    one version whose libraries differ get different files, unless their
    CRC-32 values collide, about one chance in four billion.
- **Unpacking:**
  - If the file exists with the right size and CRC-32, `nova` uses it.
  - Otherwise `nova` decompresses into a new temporary file beside it, and
    renames that into place. The file is opened with `create_new`, and its
    name joins the process id to a per-process counter, so no two threads
    or processes ever write the same temporary file. If a file left by a
    crashed run already has the name, `nova` moves on to the next number.
  - If the rename fails but the file is now there and verifies, another
    process won the race. `nova` uses that file and deletes its temporary
    one.
  - A damaged copy fails verification and is replaced.
- **Failures** name the path that failed and suggest `NOVA_HOME` or
  `NOVA_RUNTIME_LIB`. When neither home variable is set, the error says so.
  The cache's location is a pure function of the values of `NOVA_HOME`,
  `USERPROFILE` and `HOME`, so tests cover it without changing the
  environment.

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
- **`version`** is a required string: `MAJOR.MINOR.PATCH` with an
  optional pre-release, as approved. `semver::Version` parses it, and a
  version carrying build metadata, such as `1.0.0+build`, is M0003.
- **`edition`** is a required string, and `"2026"` is the only edition.
- **`description`, `license` and `repository`** are optional strings.
- **`authors`, `keywords` and `categories`** are optional arrays of strings.
- **`[dependencies]` and `[dev-dependencies]`** are optional tables of
  entries.
  - An entry is a requirement string, such as `"1.2"`, or a table with
    either `version` or `path`, as approved.
  - A table with both is M0003 for now. 3.3 decides what both would mean,
    and allowing it later breaks no manifest, while forbidding it later
    would.
  - A requirement must parse as `semver::VersionReq`, the requirement syntax
    Cargo uses (`40-TOOLING.md` §4.3 asks for Cargo's semantics).
- **Any other key or table** is reported with a warning and ignored. That
  includes `[features]`, `[build]`, `[[bin]]` and `[lib]`, and a `git` or
  `features` key inside an entry. A `{ git = "..." }` entry therefore warns,
  then fails for having neither `version` nor `path`.

### 5.2 Diagnostics

`nova-pm` reads the manifest with toml_edit's parser that keeps positions
(`ImDocument`, in toml_edit 0.22.27, already in `Cargo.lock`). Every
diagnostic therefore carries a position:
- the key or value at fault;
- the table a required key is missing from, or line 1, column 1 when
  `[package]` itself is missing;
- or the place the TOML stops parsing.

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

M0005 is raised by the CLI, not by the parser, at the entry's position: the
parsed manifest keeps a position for each dependency entry. So 3.3 removes
M0005 in one place, and `nova-pm`'s parser stays the lasting API.

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
    section. 3.0 parses `description` and `authors`, so a literal reading
    of the plan's 3.0 entry, "§1.2's template with only the keys 3.0
    parses", would keep both. This is the template Section 2 showed and
    the user approved (§13);
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
   - smoke-tests the archive on the three targets its runner executes
     natively:
     1. unpack it into an empty directory;
     2. `nova version` must say `runtime: embedded`;
     3. `nova new demo`;
     4. inside `demo`, `nova run` and the executable `nova build` writes
        must each print `Hello, Nova!`.

     `nova build` is the step that uses the embedded runtime; `nova run`
     does not. The workflow cross-builds `x86_64-apple-darwin` on an Apple
     Silicon runner today, and this design keeps that choice, so that
     archive is built but not run;
   - uploads the archive.
2. **`prepare`:**
   - gathers the four archives and writes `SHA256SUMS`;
   - extracts the release notes and fails if they are empty. The notes are
     the CHANGELOG section whose heading starts with `## [<version>]`,
     closing bracket included, up to the next `## [`. Headings carry a
     date, as in `## [0.2.0] - 2026-10-06`, and the bracket keeps
     `## [0.2.0-alpha.4]` from matching `0.2.0`;
   - on a tag, checks that the tag is `v<version>`;
   - on a pull request, prints the exact `gh release create` command that
     `publish` would run;
   - uploads the archives, `SHA256SUMS` and the notes.
3. **`publish`** runs on a `v*` tag only, with `contents: write`.
   - It runs `gh release create <tag> --verify-tag`, titled
     `Nova <version>`, with `--notes-file` for the notes.
   - The release's assets are the four archives and `SHA256SUMS`; the
     notes are its text, not an asset.
   - A tag containing `-`, such as `v0.3.0-rc.1`, makes a pre-release.

The workflow also runs on pull requests that change `release.yml`, either
build script, `crates/nova-cli/Cargo.toml`, `crates/nova-runtime/Cargo.toml`
or `Cargo.lock`. On a pull request, `publish` is skipped. So 3.0's own PR
builds the embedded runtime for all four targets, including the cross-built
one the spike never tried, and runs it on three, before any tag exists.

Changes to the runtime's sources do not trigger the workflow. A change that
breaks only the cross-built target therefore still shows first on a tag,
as it does today (§11).

## 8. The CI gate

A new `ci.yml` job, `install`, runs on ubuntu, windows and macos, in bash on
all three:

1. `cargo install --locked --path crates/nova-cli --root "$RUNNER_TEMP/nova"`.
2. Fail if any `NOVA_` variable is set, or if `$RUNNER_TEMP/nova/bin` holds
   anything but `nova` (`nova.exe` on Windows).
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
  - each required table and key, missing (M0002), with a missing
    `[package]` reported at line 1, column 1;
  - each rule broken (M0003):
    - a name that starts with a digit, one with a space, one with a
      non-ASCII letter, one of 65 characters, `con`;
    - the versions `1.0` and `1.0.0+build`;
    - the edition `2021`;
    - a value of the wrong type;
  - each dependency shape, accepted;
  - an entry with neither key (M0004), and one with both (M0003);
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
  - two processes unpacking at once both get a verified file, as Section 3
    asked. The test runs its own binary twice. Two threads in one process
    are tested too;
  - a `NOVA_HOME` that is a file gives an error that names it;
  - an embedded runtime that fails to unpack is an error, with no fall
    back to step 3;
  - the cache's location, as a pure function, with `NOVA_HOME`,
    `USERPROFILE` and `HOME` each set or unset, including all three unset;
  - the lookup order, as a pure function.
- **The build script's decisions,** kept in a module that uses only `std`
  and that a test also compiles:
  - whether to embed, for each value of `NOVA_EMBED_RUNTIME` and `PROFILE`;
  - the route, for a manifest with and without workspace keys, including a
    workspace member next to a stray `Cargo.toml.orig`;
  - the copy, which leaves out `Cargo.lock` and `target` and appends
    `[workspace]`;
  - reading the rerun list from a dep-info file.
- **`nova-std`:** the embedded set matches the files on disk (§3).
- **End to end,** in a new `crates/nova-cli/tests/project.rs` rather than
  the 11,199-line `run_tests.rs`:
  - `new` writes exactly the template, and `run` prints the greeting;
  - `new` refuses a non-empty directory and a bad name;
  - `init` names the project after its directory, keeps an existing
    `src/main.nova`, refuses an existing `nova.toml`, and suggests `--name`
    for a bad directory name;
  - `init` prints which files it wrote and which it kept;
  - all four commands work from a subdirectory;
  - paths are relative from the root and absolute from a subdirectory, in
    `args()` and in what `build` prints;
  - a project without `src/main.nova` gives an error naming that path;
  - `build` writes `target/debug/<name>`, and `-o` overrides it;
  - `build --release` writes `target/release/<name>`. Like
    `release_builds_and_runs_when_clang_available`, it skips when no
    `clang` is on `PATH`;
  - a file argument inside a project ignores even a broken manifest;
  - a declared dependency gives M0005;
  - a manifest error shows its line and column;
  - an unknown key warns, and the command proceeds;
  - outside any project, `src/main.nova` stays the default;
  - `nova version`'s last line matches the build's own embed flag. The
    build script exports that flag to every target of the package, so the
    test holds whether or not `NOVA_EMBED_RUNTIME` was set.
- **The nested cargo call itself** is not unit-tested.
  - The gate (§8) and `release.yml`'s pull-request run (§7) test it. Both
    build from a checkout, so both take the workspace route.
  - The package route's nested build first runs for real at 3.6's
    crates.io install. Running it earlier needs the real crates packaged,
    which is 3.6's work: path dependencies have no versions yet, as the
    plan's §2 item 8 records (§11).
  - The plan's first task builds with `NOVA_EMBED_RUNTIME=1`, on Windows
    and in the Linux container, before anything depends on it.
  - Its last task installs from the branch with
    `cargo install --locked --git <the local repository>`, the route the
    README gives.

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
- **ARCHITECTURE.md:** its crate table gains `nova-std`, and
  `nova-bench-http`, which it already leaves out. It lists 20 of today's 21
  crates.
- **README.md** gains an Install section: `cargo install --locked --git
  https://github.com/Sakeerin/nova nova-cli`, and the archives on GitHub
  releases from the next tag on.
- **`docs/phase-3-plan.md`:**
  - 3.0's entry points at this spec;
  - the plan's count of 21 crates gains `nova-std`;
  - decision 1's "roughly doubles `nova`" becomes "grows by about two
    thirds on Windows and doubles on Linux" (§11).
- **The sweep:** `git grep` for what 3.0 makes false, minus the files the
  branch already touched. That covers:
  - `include_str!` paths into `std/`;
  - `find_runtime_lib`'s two-step order;
  - "no `nova.toml`";
  - `nova-pm` described as a stub, as the plan's §2 item 9 does;
  - `release.yml` uploading artifacts only.

## 11. Risks

1. **The in-place nested build is new.** The spike built stand-ins, never
   the real runtime inside this workspace. The plan's first task proves it
   on Windows and on Linux before anything depends on it.
2. **Install time.** A release build of `nova-cli` now also builds the
   runtime and its dependencies in release mode, unless
   `NOVA_EMBED_RUNTIME=0`. The spike did not measure this; the plan records
   `cargo install` times before and after.
3. **Size.** `nova` grows by about two thirds on Windows and doubles on
   Linux. Decision 1's figures: a 4.6 MB payload on a 7.2 MB Windows
   `nova`, and 7.9 MB on an 8.2 MB Linux one. The plan records the real
   sizes.
4. **The cache only grows.** It gains one directory per version and build,
   and nothing removes old ones, because no Phase 3 command manages it.
   ADR 0026 leaves out `nova clean` and `nova self ...`.
5. **`gh release create` first runs for real on a tag,** and pushing a tag
   is the user's call. The pull-request run covers every step before it.
   GitHub's documentation also refuses a release made with `GITHUB_TOKEN`
   when its target commit changes `.github/workflows/` relative to the
   default branch. Tagging the default branch's own head, as the release
   practice does, avoids that.
6. **The package route is untested until 3.6.** The spike ran it on
   stand-ins only, and 3.0's tests reach only its decisions and its copy
   (§9).
7. **macOS.** The gate is the first time the nested build runs on macOS.
   `release.yml`'s pull-request run is the first time it cross-compiles.
   After 3.0, a runtime change that breaks only the cross-built target
   still shows first on a tag, as it does today (§7).
8. **Path length on Windows.** The nested build sits deep inside the outer
   `OUT_DIR`, and C compilers on Windows can fail on long paths. The plan's
   first task watches for this.
9. **CI time.** The `install` job is a cold release build on three systems,
   until its cache warms.

## 12. Not in 3.0

- Publishing anything, and versions on path dependencies (3.6).
- Resolving dependencies, `nova.lock` and `tests/` (3.3). Also a library
  template, `nova new --lib`, which this spec assigns to 3.3 because
  decision 3 makes `src/lib.nova` what `import <pkg>` reads.
- `nova.toml` files for `examples/`. `60-EXAMPLES.md` asks for full
  projects (`nova.toml`, `src/`, `tests/` and `README.md`), which `tests/`
  completes in 3.3.
- Install scripts (`40-TOOLING.md` §10.1 and §10.2; 3.6), `nova clean` and
  `nova self ...` (ADR 0026).
- `[features]`, `[build]`, `[[bin]]`, `[lib]`, git dependencies and
  workspaces.

## 13. Decisions made while writing this spec

The approved sections did not settle these, and each is open to the user's
review. Items 11 and 13 differ from what an approved section said, and
item 12 from a literal reading of the plan. The rest add detail the
sections left open.

1. The workspace case of the nested build uses `--locked` without
   `--offline` (§4.1), so that `cargo install --git` without `--locked`
   works.
2. The route is chosen by the runtime manifest's workspace keys, not by a
   file such as `Cargo.toml.orig` (§4.1).
3. The packaged copy gets an empty `[workspace]` table (§4.1).
4. Reruns follow the nested build's dep-info file and the lockfile (§4.1).
5. Any `NOVA_EMBED_RUNTIME` value but `1` or `0` fails the build (§4.1).
6. An embedded runtime that fails to unpack is an error, with no fall back
   to the executable's neighbours (§4.2).
7. New dependencies:
   - `flate2`, with `miniz_oxide` 0.9, `adler2` and `simd-adler32`. All
     four are new to the lockfile. flate2 1.1.10 turns on miniz_oxide's
     `simd` feature, which brings in `simd-adler32`;
   - and `semver`.

   `crc32fast` and `toml_edit` are already in the lockfile. flate2 1.1.10
   declares Rust 1.67, and semver 1.0.28 declares 1.68. miniz_oxide, adler2
   and simd-adler32 declare no minimum, so the MSRV job is their check.
8. The `M` diagnostic codes (§5.2), with M0005 raised by the CLI.
9. Name rules beyond the approved ones: ASCII only, at most 64 characters,
   and no Windows device names (§5.1).
10. Paths are relative at the project root and absolute elsewhere (§6.2).
11. A dependency entry with only `git` warns and then fails, M0006 then
    M0004 (§5.1). Section 2 said git dependencies are "a warning and
    ignored". Dropping the entry would leave the program's `import` to fail
    later, which is what M0005 exists to prevent.
12. The template has no `description` or `authors`, though 3.0 parses both
    (§6.3). That is the template Section 2 showed. A literal reading of the
    plan's 3.0 entry would keep them.
13. In `release.yml` (§7), on pull requests `prepare` prints the release
    command; Section 3 had the final job print it. That way the job holding
    `contents: write` never runs on a pull request. Also new: three jobs, a
    smoke test that runs `nova build` too, the release title, the failure
    on empty notes, and the extra paths in the pull-request trigger.
14. The gate (§8) also checks that `bin` holds only `nova`, and it runs
    `cargo package -p nova-std`.
15. A table with both `version` and `path` is refused for now, and so is
    build metadata in a version (§5.1). Both keep to the approved rules
    literally.
16. The examples stay without `nova.toml` until 3.3 (§12).
17. `nova new` runs no `git init`, and `nova init` writes only the files
    that are missing (§6.3).
18. Records beyond the approved list (§10): the 40-TOOLING §10 note, the
    plan's three edits, and `nova-bench-http` in ARCHITECTURE.md.
