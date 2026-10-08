# Phase 3.3a, "Local packages", Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** A Nova package can depend on another by path. `import geom` reaches
that library, module identity is (package, directory, file), `nova test`
runs `tests/`, `nova add --path` and `nova remove` edit the manifest, and
the language server follows.

**Architecture:**
- **`nova-pm` builds the package graph** from the manifests: path
  dependencies, import names, and the diagnostics M0005 and M0007–M0013.
- **`nova-driver`'s loader decides what every `import` names**, by package,
  and hands the resolver a table per module.
  - A `Program` describes what to load: the roots and the graph.
  - The resolver looks imports up in the table instead of one global map of
    names.
- **`nova-cli` builds a `Program` for each command**, and gains `nova add`,
  `nova remove`, `nova new --lib` and `nova init --lib`.
- **`nova-lsp` analyses each project as a `Program`.** It publishes only its
  own package's files and `nova.toml`, and shows a dependency's problems on
  the dependent's manifest entry.

**Tech Stack:**
- Rust: MSRV 1.78, edition 2021.
- No new crate:
  - `nova-pm` gains `nova-lexer`, for `KEYWORDS`;
  - `nova-driver` gains `nova-pm`;
  - `nova-cli` gains `toml_edit`, which is already locked at 0.22.27.
- Tests:
  - `assert_cmd` end-to-end tests through `nova`;
  - the 3.2 stdio client for the language server;
  - temp-directory package fixtures.

**Spec:** `docs/superpowers/specs/2026-10-08-phase-3-3a-local-packages-design.md`
(commits `a64aec7` and `a98089d`), approved by the user on 2026-10-08.
- Read it before Task 1: it is the authority this plan argues from.
- Its §11 lists 27 decisions, and its §2 lists the code as it stood.

## Global Constraints

- **The workspace's minimum Rust stays 1.78,** edition 2021. CI's MSRV job
  runs `cargo check --locked --workspace` with `RUSTFLAGS=-D warnings`.
- **`Cargo.lock` gains no package.** Only dependency lines change:
  - `nova-pm`'s entry gains `nova-lexer`;
  - `nova-driver`'s gains `nova-pm`;
  - `nova-cli`'s gains `toml_edit`.
- **Import names:** a package's import name is its name with each `-`
  replaced by `_` (spec §3.4).
- **A loose program resolves as today.** That is a file in no package, or a
  file not directly in a package's `src/` or `tests/` (spec §4.1). The one
  change is that imports match a file's case exactly (spec §4.3).
- **Every existing test passes,** except the ones this plan changes on
  purpose. Each is named in its task:
  - `a_project_without_src_main_nova_names_the_missing_entry` (Task 5) now
    expects M0013.
- **The codes:**
  - M0005 moves into `nova-pm`;
  - M0007–M0013 are new;
  - E0004 is new;
  - E0001 gains two notes (spec §3.5, §4.3).
- **Nothing is published.** No tag is pushed. The merge is by rebase, on
  the user's word only.

## Review Focus

The five inputs most likely to bite a user that the spec's tests leave
uncovered, most likely first. Each has its test in the task that owns the
code:

1. **A dependency path written another way:** `path = "../geom/"`, or
   `"./../geom"`. It must be the same package as `"../geom"`, with no M0011
   → `spellings_of_one_directory_are_one_package` (Task 2).
2. **An error inside a dependency, seen from the CLI.** It must be reported
   in the dependency's own file, with that file's path →
   `an_error_in_a_dependency_is_reported_in_its_own_file` (Task 4).
3. **A hyphenated package, end to end.** `nova add json-api --path
   ../json-api`, then `import json_api`, must build and run →
   `a_hyphenated_library_is_added_and_imported_with_an_underscore` (Task 7).
4. **An app and its dependency both declaring `fn main`** (a library with a
   demo `main`). The app's own `main` must run →
   `the_entry_main_runs_when_a_dependency_also_has_one` (Task 6).
5. **`nova add` run from a subdirectory of the project.** The path must be
   written relative to the manifest, not the current directory →
   `add_from_a_subdirectory_writes_a_path_relative_to_the_manifest` (Task 7).

## Decisions: where this plan settles what the spec leaves open

1. **The graph's interface** (spec §3.6):
   - `nova_pm::graph(root, db)` reads the root manifest from disk;
   - `nova_pm::graph_from(root, Some(text), db)` takes the root's text, so
     `nova add` can check a new entry before writing it.

   `Graph { packages: Vec<GraphPackage> }` has the root at `PackageId(0)`. A
   `GraphPackage` carries `dir` and `canonical`:
   - `dir` is spelled the way its files are read: the root as the caller
     gave it, and a dependency joined onto its declarer's `dir`, for example
     `../geom`;
   - `canonical` is `real_path(dir)`, the package's identity.
2. **Spans for M0013 and for the entries' paths.** `Package` gains `span`,
   the `[package]` table's position. `Dependency` gains `path_span`, the
   position of its `path` value (spec §3.5's "where it points").
3. **`Program { db, graph, roots: Vec<Root>, runs, diagnostics }`** (spec
   §4.4).
   - `Root { path, kind: RootKind::{Src, Tests, Loose} }`.
   - `runs` is true when MIR runs: the entry is `src/main.nova`, or a loose
     file.
   - Three constructors:
     - `Program::loose(path)`;
     - `Program::for_file(path)`, which follows spec §4.1;
     - `Program::for_package(root, Roots::{Program, Check, Test})`.
   - Every path-based entry point keeps its signature and builds
     `Program::for_file` (`check_file`, `compile_file`, `build_file`,
     `build_file_release`, `run_file`, `build_test_binary`, `analyze`).
     Each gains a `*_program` sibling: `check_program`, `compile_program`,
     `build_program`, `build_program_release`, `run_program`,
     `build_test_program` and `analyze_program`.
4. **The import table** is
   `HashMap<String, nova_resolver::ImportTarget>`, keyed by each import's
   first segment. `ImportTarget` is one of:
   - `Module(usize)`, the module at that index;
   - `Reported`: the loader has already reported the import.

   `ModuleSource::new(name, file)` makes a module with an empty table, and
   `name_imports(&mut [ModuleSource])` fills tables by module name.
5. **Exact case** (spec §4.3) is a directory listing, read once per
   directory per load. A name the disk holds in another case is not a match.
   A name the disk does not hold in any case may still be an editor's
   unsaved buffer, read through `Sources`.
6. **The entry's `main`** (spec §4.6). After type checking, every
   `hir::Function` named `main` is renamed to `main.not_the_entry`, unless
   it is declared in module 0. The lexer cannot produce that name. So
   `nova_mir::lower_module`'s search by name finds the entry's own `main`,
   or reports today's E0601.
7. **`nova add` takes `--path` in 3.3a.** Without it, the error says that
   registry dependencies arrive with the package index.
8. **`nova add`'s relative path** is computed between canonical directories.
   When they share no prefix (another Windows drive), the path is absolute.
9. **The server's dependency problems** (spec §6) are shown at
   `Graph::reached_through(package)`. That is the root's own entry through
   which the package is first reached, breadth first, dependencies before
   dev-dependencies.
   - The root's `nova.toml` is published under
     `uri::from_path(canonical/nova.toml)`.
   - Only the project's own analysis publishes it, never an unreached
     file's analysis.
10. **Package directories cross threads** through `Checker.dirs`, a
    `Mutex<HashMap<ProjectKey, Vec<PathKey>>>` written when a project's check
    is published, and read by `Checker::reaches`.
11. **Tests are named in the driver.** After resolution, each `TestFn` in a
    test module becomes `<file stem>::<name>`. The synthesized `main` prints
    `TestFn.name`, so the inventory `nova test` reads agrees.
12. **The library template:**
    - `src/lib.nova` has `pub fn greeting()` and one `@test`;
    - `tests/<import name>_test.nova` imports the package and tests
      `greeting()`.
13. **The CLI's `project::mode` no longer reads the manifest.** A command
    builds its `Program`, and the driver renders the graph's diagnostics, so
    M0006 warnings still appear and errors still stop it.

14. **`nova init` still refuses a directory that has a `nova.toml`.** That
    is 3.0's rule, pinned by
    `init_refuses_a_directory_that_already_has_a_manifest`. So spec §5.5's
    "beside an existing `src/main.nova`" is a directory with sources and no
    manifest yet. A library is added to an existing project by writing its
    `src/lib.nova`.
15. **`ProjectKey::entry` goes.** Spec §6 says it "becomes the program,
    else the library". Once a project's analysis is
    `Program::for_package(dir, Roots::Test)`, nothing needs an entry, and an
    unused function fails `-D warnings`.
16. **The root manifest's `FileId`** is `Program::manifest()`: the graph's
    root, or, when the root's manifest is too broken for a graph, the file
    its diagnostics are labelled in. `Analysis.manifest` carries it to the
    server, which publishes the graph's problems there either way.
17. **The CLI's commands (Task 5) come before tests and `main` (Task 6).**
    Task 6's end-to-end tests need `nova test` to take `tests/`, and the
    CLI to stop raising M0005 on a path dependency.
18. **`nova fmt` skips nested packages only in its own search** of `src/`
    and `tests/` (spec §5.4). Paths given on the command line are searched
    as before.
19. **`nova add`'s graph check ignores M0013.** Spec §5.3 lists M0013 among
    the errors that refuse an entry, and §3.1 says `nova add` still works in
    a package with neither target. A package with no source yet is not the
    new entry's fault, so §3.1 wins: every other graph error refuses.

## File Structure

| File | Task | Responsibility |
|---|---|---|
| `crates/nova-pm/src/{name,project,manifest,lib}.rs`, `tests/names.rs` (new) | 1 | `import_name`, `real_path`, `Package.span`, `Dependency.path_span` |
| `crates/nova-lsp/src/workspace.rs` | 1, 10 | `real_path` from `nova-pm`; `ProjectKey` for packages |
| `crates/nova-pm/src/graph.rs` (new), `src/lib.rs`, `Cargo.toml`, `tests/graph.rs` (new) | 2 | The package graph, M0005 and M0007–M0013 |
| `crates/nova-resolver/src/lib.rs` | 3 | `ImportTarget`, `ModuleSource::new`, `name_imports`, table lookup |
| `crates/nova-mir/tests/lower_tests.rs`, `crates/nova-typeck/src/check.rs` | 3 | `ModuleSource::new` at their sites |
| `crates/nova-driver/src/program.rs` (new), `src/analyze.rs`, `src/lib.rs`, `Cargo.toml`, `tests/packages.rs` (new) | 4, 6, 11 | `Program`, the loader, the `*_program` entry points, tests and `main` |
| `crates/nova-cli/src/project.rs`, `src/cmd/{run,test}.rs`, `tests/project.rs`, `tests/packages.rs` (new) | 5, 6 | Commands on packages |
| `crates/nova-cli/src/cmd/deps.rs` (new), `src/cmd/mod.rs`, `src/main.rs`, `Cargo.toml`, `tests/deps.rs` (new) | 7 | `nova add --path`, `nova remove` |
| `crates/nova-cli/src/template.rs`, `src/cmd/new.rs`, `tests/project.rs` | 8 | `nova new --lib`, `nova init --lib` |
| `crates/nova-cli/src/cmd/fmt.rs`, `tests/fmt.rs` | 9 | `nova fmt` on `tests/`, skipping nested packages |
| `crates/nova-lsp/src/{workspace,checker,completion,convert,lib}.rs`, `crates/nova-cli/tests/lsp.rs` | 10, 11 | The server on packages |
| `.github/scripts/packages-gate.sh` (new), `.github/workflows/ci.yml` | 12 | The 3.3a gate |
| `docs/adr/0030-package-modules.md` (new), and the records Task 13 lists | 13 | ADR, notes, CHANGELOG, README, sweep |

## Conventions for every task

- **Working directory.** `D:\Projects\nona\nova`, which is Git Bash
  `/d/Projects/nona/nova`. The Bash tool resets its directory after each
  call, so write `cd /d/Projects/nona/nova && …` in one command.
- **Scratch directory.** `P=/c/Users/SAKEER~1/AppData/Local/Temp/gcm/p33`,
  outside the repository. It holds the helpers `count.py`, `blockof.py`,
  `cmpblock.py`, `replace_once.py`, `insert_after.py`, `append.py` and
  `extract.py`, copied from `p32`. Long output goes to a file there; read
  its tail.
- **Counting a full run:** `python -X utf8 $P/count.py <FILE>`. It prints
  `N result lines: P passed, F failed, I ignored`.
- **Line endings.** The working tree is mostly CRLF (`core.autocrlf=true`),
  and the index LF.
  - The Edit tool is fine.
  - Write new files with the Write tool.
  - A Python edit keeps the file's own line ending:
    `nl = b"\r\n" if b"\r\n" in raw else b"\n"`.
  - `.github/scripts/*.sh` must stay LF; `.gitattributes` says so.
- **Write scripts and commit messages with the Write tool, never a Bash
  heredoc.** The Bash tool turns `\\` into `\`. Commit with
  `git commit -F $P/msg-<n>.txt`, then check `git log -1 --format=%s`.
- **Format before every commit:** run `cargo fmt --all`, then stage what it
  changed. rustfmt may rewrap the plan's code; that is expected.
- **Chain a commit and what follows it with `&&`, never `;`.**
- **Grep:** Git Bash's `grep -i` with two or more `-e` patterns aborts
  silently. Use `-i -E "a|b"` or `git grep`.
- **`MSYS_NO_PATHCONV=1`** before a command whose argument starts with `//`.
  Otherwise Git Bash rewrites it as a path.
- **Port 3000 must be free for a full Windows suite** (the
  `http_server_example_*` tests).
  - Check with `netstat -ano | grep -E "[:.]3000 .*LISTENING"`, which prints
    nothing when the port is free.
  - The user's own servers sometimes hold it. **Never stop such a process:
    stop and ask the user.**
- **Linux runs** use the existing harness:
  `bash /c/Users/SAKEER~1/AppData/Local/Temp/gcm/linux/run.sh <cargo args>`.
  - It exports the checkout's tracked files, uncommitted edits included.
    **Untracked files are not exported:** `git add` new files first.
  - Docker Desktop must be running: `docker version` shows a `Server:`
    section.
- **Mutants run only on committed work,** and are undone with
  `git checkout -- <file>`. Never commit a mutant.
- **Expected outputs are exact.** If a test fails on the first run of its
  task, read the failure before touching anything.
  - If the code does what this plan describes and the plan's expected value
    is wrong, correct the value and ledger it as a ruling.
  - If the code does something else, fix the code.
- **A test written after its code, on purpose,** is marked *guard* in its
  comment. It is expected to pass on its first run; the task says so.
- **Names from the existing code.** This plan quotes the code as it stood at
  `a98089d`. If a name differs (rustfmt, or the code moved), follow the
  code and ledger the difference as a ruling.

---

### Task 1: `nova-pm` basics: import names, real paths, and two spans

Spec §3.3, §3.4 and §3.5. These are the small pieces the graph is built
from.

**Files:**
- Modify: `crates/nova-pm/src/name.rs` (`import_name`)
- Modify: `crates/nova-pm/src/project.rs` (`real_path`, moved from `nova-lsp`)
- Modify: `crates/nova-pm/src/manifest.rs` (`Package.span`,
  `Dependency.path_span`)
- Modify: `crates/nova-pm/src/lib.rs` (exports)
- Modify: `crates/nova-lsp/src/workspace.rs` (uses `nova_pm::real_path`)
- Create: `crates/nova-pm/tests/names.rs`

**Interfaces:**
- Consumes: nothing.
- Produces:
  - `nova_pm::import_name(name: &str) -> String`;
  - `nova_pm::real_path(path: &Path) -> PathBuf`;
  - `nova_pm::Package.span: Span`, the `[package]` table's position;
  - `nova_pm::Dependency.path_span: Option<Span>`, its `path` value's
    position.

- [ ] **Step 1: Write the failing tests**

`crates/nova-pm/tests/names.rs`:

```rust
//! Import names, real paths, and the spans the graph's diagnostics point
//! at (spec
//! `docs/superpowers/specs/2026-10-08-phase-3-3a-local-packages-design.md`
//! §3.3–§3.5).

use nova_diagnostics::FileDb;
use nova_pm::{import_name, real_path};

#[test]
fn a_hyphen_becomes_an_underscore_in_an_import_name() {
    assert_eq!(import_name("json-api"), "json_api");
    assert_eq!(import_name("geom"), "geom");
    assert_eq!(import_name("a-b_c-d"), "a_b_c_d");
}

#[test]
fn a_real_path_is_canonical_without_the_verbatim_prefix() {
    let dir = std::env::temp_dir().join("nova-pm-real-path");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("a")).unwrap();
    let real = real_path(&dir.join("a").join(".."));
    assert_eq!(real, real_path(&dir));
    assert!(!real.to_string_lossy().starts_with(r"\\?\"), "{}", real.display());
    // A path that does not exist comes back as it was given.
    let missing = dir.join("missing");
    assert_eq!(real_path(&missing), missing);
}

#[test]
fn the_package_table_and_a_dependency_path_keep_their_positions() {
    let source = "[package]\nname = \"demo\"\nversion = \"0.1.0\"\nedition = \"2026\"\n\n\
                  [dependencies]\ngeom = { path = \"../geom\" }\nhttp = \"1.0\"\n";
    let mut db = FileDb::new();
    let file = db.add("nova.toml", source);
    let (manifest, diagnostics) = nova_pm::parse(source, file);
    assert!(diagnostics.is_empty(), "{diagnostics:?}");
    let manifest = manifest.unwrap();
    assert_eq!(db.location(file, manifest.package.span.start), Some((1, 1)));
    let path = manifest.dependencies[0].path_span.expect("a path span");
    assert_eq!(&source[path.as_range()], "\"../geom\"");
    assert_eq!(manifest.dependencies[1].path_span, None);
}
```

- [ ] **Step 2: Run them to verify they fail**

```bash
cd /d/Projects/nona/nova && cargo test --locked -p nova-pm --test names 2>&1 | grep -E "^error" | head -5
```

Expected: errors naming `import_name` and `real_path` (E0432), and the
missing fields `span` and `path_span` (E0609).

- [ ] **Step 3: Implement**

At the end of `crates/nova-pm/src/name.rs`:

```rust

/// A package's import name: its name with each `-` replaced by `_` (spec
/// `docs/superpowers/specs/2026-10-08-phase-3-3a-local-packages-design.md`
/// §3.4), so `json-api` is imported as `import json_api`.
pub fn import_name(name: &str) -> String {
    name.replace('-', "_")
}
```

At the end of `crates/nova-pm/src/project.rs`:

```rust

/// `path`, canonicalised when it exists, without Windows' `\\?\` prefix, so
/// that it is spelled as the file system spells it. A package is identified
/// by its directory's real path (spec 3.3a §3.3), and the language server
/// builds unopened files' URIs on it.
pub fn real_path(path: &Path) -> PathBuf {
    let real = std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
    match real.to_str().and_then(|s| s.strip_prefix(r"\\?\")) {
        Some(rest) => PathBuf::from(rest),
        None => real,
    }
}
```

In `crates/nova-pm/src/lib.rs`, replace the two `pub use` lines for `name`
and `project` with:

```rust
pub use name::{check_name, import_name};
pub use project::{find_root, real_path, MANIFEST};
```

In `crates/nova-pm/src/manifest.rs`:
- add to `struct Package`, after `pub categories: Vec<String>,`:

  ```rust
      /// Where `[package]` is, for a diagnostic about the package as a
      /// whole (M0013, spec 3.3a §3.5).
      pub span: Span,
  ```
- in `fn package`, add to the `Some(Package { … })` literal, after
  `categories,`:

  ```rust
              span: span(self.file, at),
  ```
- add to `struct Dependency`, after `pub path: Option<PathBuf>,`:

  ```rust
      /// Where its `path` value is, when it has one: M0007 and M0009 point
      /// there (spec 3.3a §3.5).
      pub path_span: Option<Span>,
  ```
- in `fn dependency`:
  - add `path_span: None,` after `path: None,` in the string case's
    literal;
  - add, after the `unknown_keys` call:

    ```rust
            let path_span = fields
                .get("path")
                .map(|item| span(self.file, item.span()));
    ```
  - add `path_span,` after `path,` in the last literal.

In `crates/nova-lsp/src/workspace.rs`, delete `pub fn real_path` and its doc
comment, and add `pub use nova_pm::real_path;` after
`use nova_driver::Sources;`.

- [ ] **Step 4: Run the tests to verify they pass**

```bash
cd /d/Projects/nona/nova && P=/c/Users/SAKEER~1/AppData/Local/Temp/gcm/p33 && cargo test --locked -p nova-pm -p nova-lsp > $P/t1.txt 2>&1; echo "exit=$?"; python -X utf8 $P/count.py $P/t1.txt; grep -E "FAILED|panicked|^warning|^error" $P/t1.txt | head
```

Expected: `exit=0` and 0 failed, the three new tests included, with no
warnings.

- [ ] **Step 5: Commit**

Write `$P/msg-1.txt`:

```
nova-pm: import names, real paths, and two spans

For the package graph (spec
docs/superpowers/specs/2026-10-08-phase-3-3a-local-packages-design.md
§3.3-§3.5):
- `import_name` turns `json-api` into `json_api`.
- `real_path` moves here from nova-lsp: a package is identified by its
  directory's real path. nova-lsp uses it from here.
- `Package.span` is where `[package]` is, and `Dependency.path_span`
  where an entry's `path` value is, so the graph's diagnostics can point
  there.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
```

```bash
cd /d/Projects/nona/nova && P=/c/Users/SAKEER~1/AppData/Local/Temp/gcm/p33 && cargo fmt --all && git add crates/nova-pm crates/nova-lsp && git commit -q -F $P/msg-1.txt && git log -1 --format=%s
```

Expected: `nova-pm: import names, real paths, and two spans`.

The task's test command: `cargo test --locked -p nova-pm -p nova-lsp`.

---

### Task 2: The package graph

Spec §3. This task covers M0005, which moves here from the CLI, and
M0007–M0013.

**Files:**
- Create: `crates/nova-pm/src/graph.rs`
- Modify: `crates/nova-pm/src/lib.rs` (`mod graph;` and exports)
- Modify: `crates/nova-pm/Cargo.toml` (`nova-lexer`)
- Create: `crates/nova-pm/tests/graph.rs`

**Interfaces:**
- Consumes: Task 1's `import_name`, `real_path`, `Package.span` and
  `Dependency.path_span`.
- Produces:
  - `nova_pm::graph(root: &Path, db: &mut FileDb) -> (Option<Graph>, Vec<Diagnostic>)`;
  - `nova_pm::graph_from(root: &Path, text: Option<&str>, db: &mut FileDb) -> (Option<Graph>, Vec<Diagnostic>)`;
  - `nova_pm::PackageId(pub u32)`, where the root is `PackageId(0)`;
  - `nova_pm::Edge { import_name, package: PackageId, span: Span }`;
  - `nova_pm::GraphPackage { name, import_name, dir, canonical, manifest,
    manifest_file: FileId, has_lib, has_main, dependencies: Vec<Edge>,
    dev_dependencies: Vec<Edge> }`;
  - `nova_pm::Graph { packages }`, with `root()`, `package(id)`, `dirs()`
    and `reached_through(id) -> Option<Span>`.

- [ ] **Step 1: Write the failing tests**

`crates/nova-pm/tests/graph.rs`:

```rust
//! The package graph (spec
//! `docs/superpowers/specs/2026-10-08-phase-3-3a-local-packages-design.md`
//! §3, §7.1).

use std::path::{Path, PathBuf};

use nova_diagnostics::{render, FileDb};
use nova_pm::{graph, graph_from, Graph, PackageId};

/// A fresh, empty directory under the system temp directory. Its name is
/// fixed, so each run replaces the last.
fn fresh(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("nova-pm-graph-{name}"));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("create the test directory");
    dir
}

/// A package called `name` at `dir`: its `nova.toml` with `extra` appended,
/// and an empty `src/<file>` for each of `files`.
fn package(dir: &Path, name: &str, files: &[&str], extra: &str) {
    std::fs::create_dir_all(dir.join("src")).unwrap();
    std::fs::write(
        dir.join("nova.toml"),
        format!("[package]\nname = \"{name}\"\nversion = \"0.1.0\"\nedition = \"2026\"\n{extra}"),
    )
    .unwrap();
    for file in files {
        std::fs::write(dir.join("src").join(file), "").unwrap();
    }
}

/// The graph of the package at `root`, its diagnostics' codes, and their
/// rendering.
fn build(root: &Path) -> (Option<Graph>, Vec<String>, String) {
    let mut db = FileDb::new();
    let (graph, diagnostics) = graph(root, &mut db);
    let codes = diagnostics.iter().map(|d| d.code.clone()).collect();
    (graph, codes, render::render_to_string(&db, &diagnostics))
}

fn names(graph: &Graph) -> Vec<&str> {
    graph.packages.iter().map(|p| p.name.as_str()).collect()
}

const LIB: &[&str] = &["lib.nova"];
const MAIN: &[&str] = &["main.nova"];

#[test]
fn a_path_dependency_resolves_and_a_diamond_is_one_package() {
    let dir = fresh("diamond");
    package(
        &dir.join("app"),
        "app",
        MAIN,
        "\n[dependencies]\na = { path = \"../a\" }\nb = { path = \"../b\" }\n",
    );
    package(&dir.join("a"), "a", LIB, "\n[dependencies]\nc = { path = \"../c\" }\n");
    package(&dir.join("b"), "b", LIB, "\n[dependencies]\nc = { path = \"../c\" }\n");
    package(&dir.join("c"), "c", LIB, "");
    let (graph, codes, _) = build(&dir.join("app"));
    assert!(codes.is_empty(), "{codes:?}");
    let graph = graph.unwrap();
    assert_eq!(names(&graph), ["app", "a", "b", "c"]);
    assert_eq!(graph.package(PackageId(1)).dependencies[0].package, PackageId(3));
    assert_eq!(graph.package(PackageId(2)).dependencies[0].package, PackageId(3));
    assert!(graph.root().has_main && !graph.root().has_lib);
    assert_eq!(graph.package(PackageId(1)).dir, dir.join("app").join("../a"));
}

#[test]
fn spellings_of_one_directory_are_one_package() {
    // Review Focus 1: a trailing slash, a `./`, and a detour through `..`.
    let dir = fresh("spellings");
    package(
        &dir.join("app"),
        "app",
        MAIN,
        "\n[dependencies]\na = { path = \"../a/\" }\nb = { path = \"./../b\" }\n",
    );
    package(&dir.join("a"), "a", LIB, "\n[dependencies]\nc = { path = \"../c\" }\n");
    package(&dir.join("b"), "b", LIB, "\n[dependencies]\nc = { path = \"../b/../c/\" }\n");
    package(&dir.join("c"), "c", LIB, "");
    let (graph, codes, rendered) = build(&dir.join("app"));
    assert!(codes.is_empty(), "{rendered}");
    assert_eq!(names(&graph.unwrap()), ["app", "a", "b", "c"]);
}

#[test]
fn a_version_only_entry_is_m0005_and_names_the_dependency() {
    let dir = fresh("m0005");
    package(&dir, "app", MAIN, "\n[dependencies]\nhttp = \"1.0\"\n");
    let (graph, codes, rendered) = build(&dir);
    assert_eq!(codes, ["M0005"]);
    assert!(rendered.contains("`http` is a registry dependency"), "{rendered}");
    assert!(rendered.contains("use `path"), "{rendered}");
    assert_eq!(names(&graph.unwrap()), ["app"]);
}

#[test]
fn a_path_without_a_manifest_is_m0007_at_the_path() {
    let dir = fresh("m0007");
    package(&dir, "app", MAIN, "\n[dependencies]\ngeom = { path = \"../nowhere\" }\n");
    let (_, codes, rendered) = build(&dir);
    assert_eq!(codes, ["M0007"]);
    assert!(rendered.contains("is not a directory holding a nova.toml"), "{rendered}");
    assert!(rendered.contains("nova.toml:7:17"), "{rendered}");
}

#[test]
fn a_misnamed_dependency_is_m0008() {
    let dir = fresh("m0008");
    package(&dir.join("app"), "app", MAIN, "\n[dependencies]\ngeom = { path = \"../geom\" }\n");
    package(&dir.join("geom"), "geometry", LIB, "");
    let (_, codes, rendered) = build(&dir.join("app"));
    assert_eq!(codes, ["M0008"]);
    assert!(rendered.contains("dependency `geom` is the package `geometry`"), "{rendered}");
}

#[test]
fn a_dependency_without_a_library_is_m0009() {
    let dir = fresh("m0009");
    package(&dir.join("app"), "app", MAIN, "\n[dependencies]\ngeom = { path = \"../geom\" }\n");
    package(&dir.join("geom"), "geom", MAIN, "");
    let (_, codes, rendered) = build(&dir.join("app"));
    assert_eq!(codes, ["M0009"]);
    assert!(rendered.contains("is not a library"), "{rendered}");
}

#[test]
fn a_cycle_is_m0010_and_lists_its_packages() {
    let dir = fresh("m0010");
    package(&dir.join("app"), "app", LIB, "\n[dependencies]\na = { path = \"../a\" }\n");
    package(&dir.join("a"), "a", LIB, "\n[dependencies]\napp = { path = \"../app\" }\n");
    let (_, codes, rendered) = build(&dir.join("app"));
    assert_eq!(codes, ["M0010"]);
    assert!(rendered.contains("dependency cycle: app -> a -> app"), "{rendered}");
}

#[test]
fn a_path_to_the_package_itself_is_m0010() {
    let dir = fresh("m0010-self");
    package(&dir, "app", MAIN, "\n[dependencies]\nme = { path = \".\" }\n");
    let (_, codes, rendered) = build(&dir);
    assert_eq!(codes, ["M0010"]);
    assert!(rendered.contains("`app` depends on itself"), "{rendered}");
}

#[test]
fn two_packages_with_one_name_are_m0011() {
    let dir = fresh("m0011");
    package(
        &dir.join("app"),
        "app",
        MAIN,
        "\n[dependencies]\na = { path = \"../a\" }\nb = { path = \"../b\" }\n",
    );
    package(&dir.join("a"), "a", LIB, "\n[dependencies]\nc = { path = \"../c1\" }\n");
    package(&dir.join("b"), "b", LIB, "\n[dependencies]\nc = { path = \"../c2\" }\n");
    package(&dir.join("c1"), "c", LIB, "");
    package(&dir.join("c2"), "c", LIB, "");
    let (_, codes, rendered) = build(&dir.join("app"));
    assert_eq!(codes, ["M0011"]);
    assert!(rendered.contains("two packages named `c` in one build"), "{rendered}");
    assert!(rendered.contains("c1") && rendered.contains("c2"), "{rendered}");
}

#[test]
fn import_name_clashes_and_keywords_are_m0012() {
    let dir = fresh("m0012");
    for (name, package_name) in [("geom", "geom"), ("json-api", "json-api"), ("json_api", "json_api")] {
        package(&dir.join(name), package_name, LIB, "");
    }
    let cases = [
        ("keyword", "\n[dependencies]\nmatch = { path = \"../geom\" }\n", "which is a keyword"),
        (
            "both-tables",
            "\n[dependencies]\ngeom = { path = \"../geom\" }\n\n[dev-dependencies]\ngeom = { path = \"../geom\" }\n",
            "`geom` is in both [dependencies] and [dev-dependencies]",
        ),
        (
            "one-import-name",
            "\n[dependencies]\njson-api = { path = \"../json-api\" }\njson_api = { path = \"../json_api\" }\n",
            "`json-api` and `json_api` are both imported as `json_api`",
        ),
        ("own-name", "\n[dependencies]\napp = { path = \"../geom\" }\n", "this package's own name"),
    ];
    for (case, extra, message) in cases {
        let root = dir.join(format!("app-{case}"));
        package(&root, "app", MAIN, extra);
        let (_, codes, rendered) = build(&root);
        assert_eq!(codes, ["M0012"], "{case}: {rendered}");
        assert!(rendered.contains(message), "{case}: {rendered}");
    }
}

#[test]
fn a_library_named_like_a_keyword_is_m0012() {
    let dir = fresh("m0012-root");
    package(&dir, "match", LIB, "");
    let (_, codes, rendered) = build(&dir);
    assert_eq!(codes, ["M0012"]);
    assert!(rendered.contains("library `match` would be imported as `match`"), "{rendered}");
}

#[test]
fn a_package_with_neither_target_is_m0013_at_its_package_table() {
    let dir = fresh("m0013");
    package(&dir, "app", &[], "");
    let (_, codes, rendered) = build(&dir);
    assert_eq!(codes, ["M0013"]);
    assert!(rendered.contains("has neither src/lib.nova nor src/main.nova"), "{rendered}");
    assert!(rendered.contains("nova.toml:1:1"), "{rendered}");
}

#[test]
fn a_dependencys_dev_dependencies_are_not_read() {
    let dir = fresh("dev-not-read");
    package(&dir.join("app"), "app", MAIN, "\n[dependencies]\na = { path = \"../a\" }\n");
    package(&dir.join("a"), "a", LIB, "\n[dev-dependencies]\nx = { path = \"../nowhere\" }\n");
    let (graph, codes, rendered) = build(&dir.join("app"));
    assert!(codes.is_empty(), "{rendered}");
    assert!(graph.unwrap().package(PackageId(1)).dev_dependencies.is_empty());
}

#[test]
fn a_dev_dependency_that_depends_back_on_the_root_is_no_cycle() {
    let dir = fresh("dev-back");
    package(&dir.join("app"), "app", LIB, "\n[dev-dependencies]\nhelper = { path = \"../helper\" }\n");
    package(&dir.join("helper"), "helper", LIB, "\n[dependencies]\napp = { path = \"../app\" }\n");
    let (graph, codes, rendered) = build(&dir.join("app"));
    assert!(codes.is_empty(), "{rendered}");
    let graph = graph.unwrap();
    assert_eq!(names(&graph), ["app", "helper"]);
    assert_eq!(graph.package(PackageId(1)).dependencies[0].package, PackageId(0));
}

#[test]
fn a_dependency_back_on_a_root_without_a_library_is_m0009() {
    let dir = fresh("dev-back-no-lib");
    package(&dir.join("app"), "app", MAIN, "\n[dev-dependencies]\nhelper = { path = \"../helper\" }\n");
    package(&dir.join("helper"), "helper", LIB, "\n[dependencies]\napp = { path = \"../app\" }\n");
    let (_, codes, rendered) = build(&dir.join("app"));
    assert_eq!(codes, ["M0009"], "{rendered}");
    assert!(rendered.contains("dependency `app` is not a library"), "{rendered}");
}

#[test]
fn paths_are_relative_to_each_manifest() {
    let dir = fresh("relative");
    package(
        &dir.join("apps").join("app"),
        "app",
        MAIN,
        "\n[dependencies]\na = { path = \"../../libs/a\" }\n",
    );
    package(&dir.join("libs").join("a"), "a", LIB, "\n[dependencies]\nb = { path = \"../b\" }\n");
    package(&dir.join("libs").join("b"), "b", LIB, "");
    let (graph, codes, rendered) = build(&dir.join("apps").join("app"));
    assert!(codes.is_empty(), "{rendered}");
    assert_eq!(names(&graph.unwrap()), ["app", "a", "b"]);
}

#[test]
fn reached_through_names_the_roots_own_entry() {
    let dir = fresh("reached");
    package(&dir.join("app"), "app", MAIN, "\n[dependencies]\na = { path = \"../a\" }\n");
    package(&dir.join("a"), "a", LIB, "\n[dependencies]\nc = { path = \"../c\" }\n");
    package(&dir.join("c"), "c", LIB, "");
    let (graph, codes, _) = build(&dir.join("app"));
    assert!(codes.is_empty(), "{codes:?}");
    let graph = graph.unwrap();
    let entry = graph.root().dependencies[0].span;
    assert_eq!(graph.reached_through(PackageId(1)), Some(entry));
    assert_eq!(graph.reached_through(PackageId(2)), Some(entry));
    assert_eq!(graph.reached_through(PackageId(0)), None);
}

#[test]
fn graph_from_reads_the_roots_manifest_from_the_text() {
    let dir = fresh("from-text");
    package(&dir.join("app"), "app", MAIN, "");
    package(&dir.join("a"), "a", LIB, "");
    let text = "[package]\nname = \"app\"\nversion = \"0.1.0\"\nedition = \"2026\"\n\n\
                [dependencies]\na = { path = \"../a\" }\n";
    let mut db = FileDb::new();
    let (graph, diagnostics) = graph_from(&dir.join("app"), Some(text), &mut db);
    assert!(diagnostics.is_empty(), "{diagnostics:?}");
    assert_eq!(names(&graph.unwrap()), ["app", "a"]);
}

#[test]
fn every_manifests_warnings_are_kept() {
    let dir = fresh("warnings");
    package(&dir.join("app"), "app", MAIN, "\n[dependencies]\na = { path = \"../a\" }\n");
    package(&dir.join("a"), "a", LIB, "\n[features]\ndefault = []\n");
    let (graph, codes, rendered) = build(&dir.join("app"));
    assert_eq!(codes, ["M0006"]);
    assert!(rendered.contains("`features`"), "{rendered}");
    assert_eq!(names(&graph.unwrap()), ["app", "a"]);
}
```

- [ ] **Step 2: Run them to verify they fail**

```bash
cd /d/Projects/nona/nova && cargo test --locked -p nova-pm --test graph 2>&1 | grep -E "^error" | head -3
```

Expected: `error[E0432]: unresolved imports` naming `graph`, `graph_from`,
`Graph` and `PackageId`.

- [ ] **Step 3: Write `graph.rs`**

`crates/nova-pm/src/graph.rs`:

```rust
//! The package graph (spec
//! `docs/superpowers/specs/2026-10-08-phase-3-3a-local-packages-design.md`
//! §3): the root package, and every package its path dependencies reach.

use std::collections::{HashMap, VecDeque};
use std::path::{Path, PathBuf};

use nova_diagnostics::{Diagnostic, FileDb, FileId, Span};

use crate::manifest::{Dependency, Manifest};
use crate::{import_name, real_path, MANIFEST};

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

/// Read the package at `root`, the directory holding its `nova.toml` (empty
/// for the current directory), and every package its path dependencies
/// reach. Every manifest goes into `db`, so each diagnostic's label renders.
///
/// The graph is partial on error: what resolved is kept, with the
/// diagnostics. It is `None` only when the root's own manifest cannot be
/// read or parsed.
pub fn graph(root: &Path, db: &mut FileDb) -> (Option<Graph>, Vec<Diagnostic>) {
    graph_from(root, None, db)
}

/// [`graph`], with the root's `nova.toml` taken from `text` when it is
/// given: `nova add` checks a new entry this way before writing it.
pub fn graph_from(
    root: &Path,
    text: Option<&str>,
    db: &mut FileDb,
) -> (Option<Graph>, Vec<Diagnostic>) {
    let mut builder = Builder {
        db,
        diagnostics: Vec::new(),
        packages: Vec::new(),
    };
    let Some(package) = builder.read(root, text) else {
        return (None, builder.diagnostics);
    };
    if !package.has_lib && !package.has_main {
        builder.error(
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
        builder.error(
            "M0012",
            format!(
                "library `{}` would be imported as `{}`, which is a keyword",
                package.name, package.import_name
            ),
            package.manifest.package.span,
            "choose another name",
        );
    }
    builder.packages.push(package);
    let mut queue = VecDeque::from([PackageId(0)]);
    while let Some(id) = queue.pop_front() {
        builder.expand(id, &mut queue);
    }
    builder.cycles();
    let Builder {
        diagnostics,
        packages,
        ..
    } = builder;
    (Some(Graph { packages }), diagnostics)
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

struct Builder<'a> {
    db: &'a mut FileDb,
    diagnostics: Vec<Diagnostic>,
    packages: Vec<GraphPackage>,
}

impl Builder<'_> {
    fn error(&mut self, code: &str, message: String, at: Span, label: &str) {
        self.diagnostics
            .push(Diagnostic::error(code, message).with_primary_label(at, label));
    }

    /// The package whose manifest is `dir/nova.toml`, or `text` when it is
    /// given, with no edges yet. `None` when the manifest cannot be read or
    /// has errors.
    fn read(&mut self, dir: &Path, text: Option<&str>) -> Option<GraphPackage> {
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
        self.diagnostics.extend(diagnostics);
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
            manifest,
            manifest_file: file,
            dependencies: Vec::new(),
            dev_dependencies: Vec::new(),
        })
    }

    /// Resolve package `id`'s entries, and for the root its
    /// dev-dependencies, queueing each package read for the first time.
    fn expand(&mut self, id: PackageId, queue: &mut VecDeque<PackageId>) {
        let package = &self.packages[id.0 as usize];
        let mut entries: Vec<(Dependency, bool)> = package
            .manifest
            .dependencies
            .iter()
            .cloned()
            .map(|d| (d, false))
            .collect();
        if id == PackageId(0) {
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
        let Some(path) = &dependency.path else {
            self.diagnostics.push(
                Diagnostic::error(
                    "M0005",
                    format!(
                        "dependency `{}` is a registry dependency; registry dependencies \
                         arrive with the package index",
                        dependency.name
                    ),
                )
                .with_primary_label(dependency.span, "declared here")
                .with_note("use `path = \"...\"` for a local package"),
            );
            return None;
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
                let new = self.read(&dir, None)?;
                if new.name != dependency.name {
                    let message =
                        format!("dependency `{}` is the package `{}`", dependency.name, new.name);
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
                            dir.display()
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
                        dir.display()
                    );
                    self.error("M0011", message, dependency.span, "the second package of that name");
                    return None;
                }
                self.packages.push(new);
                let id = PackageId(self.packages.len() as u32 - 1);
                queue.push_back(id);
                id
            }
        };
        Some(Edge {
            import_name: import_name(&dependency.name),
            package,
            span: dependency.span,
        })
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

In `crates/nova-pm/src/lib.rs`:
- add `mod graph;` after `pub mod manifest;`;
- add `pub use graph::{graph, graph_from, Edge, Graph, GraphPackage, PackageId};`
  after `pub use manifest::…;`;
- change the module comment's first line to "The package manager: `nova.toml`
  parsing, package names, finding a project, and the package graph (Phase
  3.0 and 3.3a)".

In `crates/nova-pm/Cargo.toml`, add after `nova-diagnostics = …`:

```toml
nova-lexer = { path = "../nova-lexer" }
```

- [ ] **Step 4: Run the tests to verify they pass**

```bash
cd /d/Projects/nona/nova && P=/c/Users/SAKEER~1/AppData/Local/Temp/gcm/p33 && cargo test --locked -p nova-pm > $P/t2.txt 2>&1; echo "exit=$?"; python -X utf8 $P/count.py $P/t2.txt; grep -E "FAILED|panicked|^warning|^error" $P/t2.txt | head; git diff --stat -- Cargo.lock
```

Expected:
- `exit=0`, 0 failed, the 19 new tests included, no warnings;
- `Cargo.lock` changes by one line, `nova-lexer` in `nova-pm`'s entry.

If `a_path_without_a_manifest_is_m0007_at_the_path` reports another column,
read where `toml_edit` puts a string value's span. Correct the column only
if the label sits on the `"../nowhere"` value, and ledger it.

- [ ] **Step 5: Commit**

Write `$P/msg-2.txt`:

```
nova-pm: the package graph

`nova_pm::graph(root, db)` reads the root package and every package its
path dependencies reach (spec
docs/superpowers/specs/2026-10-08-phase-3-3a-local-packages-design.md §3):
- **Paths.** Each is read relative to the manifest that declares it. A
  package is identified by its canonical directory, so a diamond, or one
  directory spelled two ways, is one package.
- **Dev-dependencies.** Only the root's are read.
- **Codes:**
  - M0005: a registry dependency. It moves here from the CLI and keeps
    the dependency's name.
  - M0007: a path with no nova.toml.
  - M0008: a key that is not the package's name.
  - M0009: a dependency that is not a library.
  - M0010: a cycle, a path to the package itself included.
  - M0011: two packages with one name.
  - M0012: an import-name clash, a key in both tables, a name like the
    package's own, or a keyword.
  - M0013: a package with neither target.
- **A partial graph.** It is kept on error, for the language server.

`graph_from` takes the root's text, for `nova add` to check an entry
before writing it. `reached_through` names the root's entry through which
a package is reached. nova-pm gains nova-lexer, for KEYWORDS.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
```

```bash
cd /d/Projects/nona/nova && P=/c/Users/SAKEER~1/AppData/Local/Temp/gcm/p33 && cargo fmt --all && git add crates/nova-pm Cargo.lock && git commit -q -F $P/msg-2.txt && git log -1 --format=%s
```

Expected: `nova-pm: the package graph`.

The task's test command: `cargo test --locked -p nova-pm`.

---

### Task 3: The resolver reads an import table

Spec §4.5. Nothing changes for any program yet: every site that builds a
`ModuleSource` fills its table by name, which is today's lookup.

**Files:**
- Modify: `crates/nova-resolver/src/lib.rs`:
  - `ModuleSource`, `ImportTarget`, `name_imports`;
  - `resolve`, `resolve_program` and `resolve_import`;
  - `resolve_two` and three new tests in the inline `mod tests`.
- Modify: `crates/nova-mir/tests/lower_tests.rs:1181`, `:1269`
- Modify: `crates/nova-typeck/src/check.rs:16606`
- Modify: `crates/nova-driver/src/analyze.rs:199-205`, `crates/nova-driver/src/lib.rs:567-573`

**Interfaces:**
- Consumes: nothing.
- Produces:
  - `nova_resolver::ImportTarget::{Module(usize), Reported}`;
  - `ModuleSource { name, file, imports: HashMap<String, ImportTarget> }`
    and `ModuleSource::new(name, file)`;
  - `nova_resolver::name_imports(modules: &mut [ModuleSource])`.

- [ ] **Step 1: Write the failing tests**

In `crates/nova-resolver/src/lib.rs`'s `mod tests`, after `fn error_codes`:

```rust

    // === Phase 3.3a: import tables (spec
    // docs/superpowers/specs/2026-10-08-phase-3-3a-local-packages-design.md §4.5) ===

    #[test]
    fn an_import_table_can_name_a_module_by_any_label() {
        let main = parse_file("import geom::{area}\nfn main() { let a = area() }\n");
        let lib = parse_file("pub fn area() -> Int { 1 }\n");
        let mut sources = [ModuleSource::new("main", &main), ModuleSource::new("geom", &lib)];
        sources[1].name = "a label no import names".to_string();
        sources[0]
            .imports
            .insert("geom".to_string(), ImportTarget::Module(1));
        let std_files: Vec<FileId> = STD_MODULES.iter().map(|_| FileId::DUMMY).collect();
        let p = resolve_program(&sources, &std_files, None);
        assert!(p.diagnostics.is_empty(), "{:?}", p.diagnostics);
        assert!(matches!(
            p.definitions.resolve_value(ModuleId(0), "area"),
            Some(Res::Def(_))
        ));
    }

    #[test]
    fn a_reported_import_binds_nothing_and_says_nothing() {
        let main = parse_file("import geom\nfn main() {}\n");
        let mut sources = [ModuleSource::new("main", &main)];
        sources[0]
            .imports
            .insert("geom".to_string(), ImportTarget::Reported);
        let std_files: Vec<FileId> = STD_MODULES.iter().map(|_| FileId::DUMMY).collect();
        let p = resolve_program(&sources, &std_files, None);
        assert!(p.diagnostics.is_empty(), "{:?}", p.diagnostics);
    }

    #[test]
    fn an_import_missing_from_the_table_cannot_find_its_module() {
        let main = parse_file("import geom\nfn main() {}\n");
        let lib = parse_file("pub fn area() -> Int { 1 }\n");
        // No `name_imports`: the table is empty, whatever the modules' names.
        let sources = [ModuleSource::new("main", &main), ModuleSource::new("geom", &lib)];
        let std_files: Vec<FileId> = STD_MODULES.iter().map(|_| FileId::DUMMY).collect();
        let p = resolve_program(&sources, &std_files, None);
        assert_eq!(error_codes(&p.diagnostics), ["E0001"]);
        assert!(p.diagnostics[0].message.contains("cannot find module `geom`"));
    }
```

- [ ] **Step 2: Run them to verify they fail**

```bash
cd /d/Projects/nona/nova && cargo test --locked -p nova-resolver --lib 2>&1 | grep -E "^error" | head -3
```

Expected: errors naming `ModuleSource::new` and `ImportTarget` (E0599,
E0433).

- [ ] **Step 3: Implement**

In `crates/nova-resolver/src/lib.rs`, replace
`pub struct ModuleSource<'a> { … }` and its doc comment with:

```rust
/// What an `import` names, by its first segment (spec 3.3a §4.5).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImportTarget {
    /// The module at this index of [`resolve_program`]'s `modules`.
    Module(usize),
    /// The driver has already reported this import (E0004, or a
    /// dev-dependency imported from `src/`): bind nothing, and say nothing
    /// more.
    Reported,
}

/// A source module: a parsed file, its label, and what its imports name.
pub struct ModuleSource<'a> {
    /// A label for the module. Messages print an import as written, so the
    /// label is never a lookup key.
    pub name: String,
    pub file: &'a File,
    /// What each import names, by its first segment. An import missing here
    /// is "cannot find module".
    pub imports: std::collections::HashMap<String, ImportTarget>,
}

impl<'a> ModuleSource<'a> {
    /// A module with an empty import table; [`name_imports`] fills it by
    /// name.
    pub fn new(name: impl Into<String>, file: &'a File) -> Self {
        ModuleSource {
            name: name.into(),
            file,
            imports: std::collections::HashMap::new(),
        }
    }
}

/// Fill each module's import table by module name, as a program without
/// packages has always resolved (ADR 0003): `import m` names the first
/// module called `m`.
pub fn name_imports(modules: &mut [ModuleSource]) {
    let names: Vec<String> = modules.iter().map(|m| m.name.clone()).collect();
    for module in modules.iter_mut() {
        for item in &module.file.items {
            let Item::Import(import) = &item.value else {
                continue;
            };
            let Some(first) = import.path.value.segments.first() else {
                continue;
            };
            if let Some(index) = names.iter().position(|n| *n == first.value) {
                module
                    .imports
                    .insert(first.value.clone(), ImportTarget::Module(index));
            }
        }
    }
}
```

In `pub fn resolve`, replace `let sources = [ModuleSource { … }];` with:

```rust
    let mut sources = [ModuleSource::new("main", file)];
    name_imports(&mut sources);
```

In `resolve_program`, build `all` with tables:
- the user modules' map closure becomes:

  ```rust
          .map(|m| ModuleSource {
              name: m.name.clone(),
              file: m.file,
              imports: m.imports.clone(),
          })
  ```
- the std modules' becomes
  `file.as_ref().map(|file| ModuleSource::new(name.to_string(), file))`.

Replace Pass 2, from `let by_name: FxHashMap<&str, usize> = all` to the end
of its loop, with:

```rust
    // Pass 2: resolve `import`s, binding other modules' public names. What
    // each import names is its module's table (spec 3.3a §4.5).
    for (mid, m) in all.iter().enumerate() {
        for item in &m.file.items {
            if let Item::Import(imp) = &item.value {
                resolve_import(
                    &mut definitions,
                    &exports,
                    &m.imports,
                    mid,
                    imp,
                    &mut diagnostics,
                );
            }
        }
    }
```

In `fn resolve_import`:
- replace the parameter `by_name: &FxHashMap<&str, usize>,` with
  `imports: &std::collections::HashMap<String, ImportTarget>,`;
- replace the `let Some(&target) = by_name.get(target_name) else { … };`
  block with:

  ```rust
      let target = match imports.get(target_name) {
          Some(ImportTarget::Module(target)) => *target,
          Some(ImportTarget::Reported) => return,
          None => {
              diagnostics.push(
                  Diagnostic::error("E0001", format!("cannot find module `{target_name}`"))
                      .with_primary_label(span, "no such module"),
              );
              return;
          }
      };
  ```

In `mod tests`, `fn resolve_two` becomes:

```rust
    fn resolve_two(main_src: &str, lib_src: &str) -> ProgramResolution {
        let main = parse_file(main_src);
        let lib = parse_file(lib_src);
        let mut sources = [ModuleSource::new("main", &main), ModuleSource::new("lib", &lib)];
        name_imports(&mut sources);
        let std_files: Vec<FileId> = STD_MODULES.iter().map(|_| FileId::DUMMY).collect();
        resolve_program(&sources, &std_files, None)
    }
```

The other sites:
- `crates/nova-mir/tests/lower_tests.rs`: both
  `let sources = [ModuleSource { name: "main".to_string(), file: &ast, }];`
  become `let sources = [ModuleSource::new("main", &ast)];`;
- `crates/nova-typeck/src/check.rs`, in `probe_src`, the literal becomes
  `let module = nova_resolver::ModuleSource::new("main", &parsed.file);`;
- `crates/nova-driver/src/analyze.rs` and `crates/nova-driver/src/lib.rs`:
  - each `.map(|(name, file)| ModuleSource { name: name.clone(), file, })`
    becomes `.map(|(name, file)| ModuleSource::new(name.clone(), file))`;
  - the `let` takes `mut`, and `nova_resolver::name_imports(&mut …);`
    follows it. Task 4 replaces both with the loader's tables.

If the build names another `ModuleSource { … }` literal, give it the same
treatment and ledger it.

- [ ] **Step 4: Run the tests to verify they pass**

```bash
cd /d/Projects/nona/nova && P=/c/Users/SAKEER~1/AppData/Local/Temp/gcm/p33 && cargo test --locked -p nova-resolver -p nova-typeck -p nova-mir -p nova-driver > $P/t3.txt 2>&1; echo "exit=$?"; python -X utf8 $P/count.py $P/t3.txt; grep -E "FAILED|panicked|^warning|^error" $P/t3.txt | head
```

Expected: `exit=0` and 0 failed, the three new tests included, no warnings.
Every other test passes unchanged: the lookup is the same.

- [ ] **Step 5: Commit**

Write `$P/msg-3.txt`:

```
nova-resolver: imports resolve through a per-module table

`ModuleSource` carries a table from each import's first segment to the
module it names (spec
docs/superpowers/specs/2026-10-08-phase-3-3a-local-packages-design.md
§4.5). An entry is either the module at an index, or `Reported`, an
import the driver has already reported. `resolve_import` reads that table
instead of one global map of names. A module's name is now only a label.

`name_imports` fills tables by module name, which is how a program
without packages has always resolved. Every existing site uses it, so
nothing changes yet; the driver's loader gives packages their own tables
next.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
```

```bash
cd /d/Projects/nona/nova && P=/c/Users/SAKEER~1/AppData/Local/Temp/gcm/p33 && cargo fmt --all && git add crates && git commit -q -F $P/msg-3.txt && git log -1 --format=%s
```

Expected: `nova-resolver: imports resolve through a per-module table`.

The task's test command:
`cargo test --locked -p nova-resolver -p nova-typeck -p nova-mir -p nova-driver`.

---

### Task 4: `Program` and the package loader

Spec §4.1–§4.4. This task changes what every compile loads, so it ends
with the full `nova-cli` suite.

**Files:**
- Create: `crates/nova-driver/src/program.rs`
- Modify: `crates/nova-driver/src/analyze.rs` (`Loaded` and `load_program`
  move out; `Analysis` gains two fields; `analyze_program`)
- Modify: `crates/nova-driver/src/lib.rs`:
  - `mod program;` and the exports;
  - the `*_program` entry points, with the path wrappers;
  - `FrontendContext`;
  - `strip_test_functions`.
- Modify: `crates/nova-driver/Cargo.toml` (`nova-pm`)
- Create: `crates/nova-driver/tests/packages.rs`

**Interfaces:**
- Consumes:
  - Task 2's `nova_pm::{graph, Graph, GraphPackage, Edge, PackageId}`;
  - Task 3's `ImportTarget` and `ModuleSource { name, file, imports }`.
- Produces:
  - `nova_driver::{Program, Root, RootKind, Roots}`:
    - `Program { db: FileDb, graph: Option<Graph>, roots: Vec<Root>, runs: bool, diagnostics: Vec<Diagnostic> }`;
    - `Program::loose(&Path)`, `Program::for_file(&Path)`,
      `Program::for_package(&Path, Roots)` and `Program::has_errors()`;
    - `Root { path: PathBuf, kind: RootKind }`;
    - `RootKind::{Src, Tests, Loose}`;
    - `Roots::{Program, Check, Test}`.
  - `nova_driver::analyze_program(Program, &dyn Sources, &Options) -> io::Result<Analysis>`.
  - `nova_driver::package_of(&Path) -> Option<(PathBuf, RootKind)>`: the
    package directory a file is directly in the `src/` or `tests/` of.
  - `Analysis.module_packages: Vec<Option<PackageId>>`, parallel to
    `Analysis.modules`, and `Analysis.graph: Option<Graph>`.
  - In `nova_driver`:
    - `check_program(Program) -> Result<Outcome<()>>`;
    - `compile_program(Program) -> Result<Outcome<CompiledProgram>>`;
    - `build_program(Program, &Path) -> Result<Outcome<PathBuf>>`;
    - `build_program_release(Program, &Path) -> Result<Outcome<PathBuf>>`;
    - `run_program(Program, Vec<String>) -> Result<Outcome<()>>`;
    - `build_test_program(Program) -> Result<(PathBuf, Vec<TestFn>)>`.
  - In the crate only: `program::{Loaded, Load, load_program}`, where
    `Loaded` has `name`, `path`, `file`, `ast`,
    `package: Option<PackageId>`, `test_file: bool` and
    `imports: HashMap<String, ImportTarget>`.

- [ ] **Step 1: Write the failing tests**

`crates/nova-driver/tests/packages.rs`:

```rust
//! The loader on packages (spec
//! `docs/superpowers/specs/2026-10-08-phase-3-3a-local-packages-design.md`
//! §4, §7.2).

use std::path::{Path, PathBuf};

use nova_driver::{analyze, analyze_program, Analysis, DiskSources, Options, Program, Roots};
use nova_pm::PackageId;

/// A fresh, empty directory with a fixed name, so each run replaces the last.
fn fresh(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("nova-driver-packages-{name}"));
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

/// A `nova.toml` for `name`, with `extra` appended.
fn manifest(name: &str, extra: &str) -> String {
    format!("[package]\nname = \"{name}\"\nversion = \"0.1.0\"\nedition = \"2026\"\n{extra}")
}

const GEOM_DEPENDENCY: &str = "\n[dependencies]\ngeom = { path = \"../geom\" }\n";

/// `dir/geom`: a library whose `lib.nova` uses its own `utils.nova`.
fn geom(dir: &Path) {
    write(
        &dir.join("geom"),
        &[
            ("nova.toml", manifest("geom", "").as_str()),
            (
                "src/lib.nova",
                "import utils\n\npub fn area() -> Int {\n    width() * width()\n}\n",
            ),
            ("src/utils.nova", "pub fn width() -> Int {\n    3\n}\n"),
        ],
    );
}

fn options(tests: bool) -> Options {
    Options {
        tests,
        ..Options::default()
    }
}

fn check(program: Program) -> Analysis {
    analyze_program(program, &DiskSources, &options(false)).expect("the entry reads")
}

fn codes(a: &Analysis) -> Vec<&str> {
    a.diagnostics.iter().map(|d| d.code.as_str()).collect()
}

fn messages(a: &Analysis) -> String {
    a.diagnostics
        .iter()
        .map(|d| format!("{} {} {:?}\n", d.code, d.message, d.notes))
        .collect()
}

fn paths(a: &Analysis) -> Vec<&Path> {
    a.modules.iter().map(|(_, path)| path.as_path()).collect()
}

#[test]
fn two_packages_utils_are_two_modules() {
    let dir = fresh("two-utils");
    geom(&dir);
    let app = dir.join("app");
    write(
        &app,
        &[
            ("nova.toml", manifest("app", GEOM_DEPENDENCY).as_str()),
            (
                "src/main.nova",
                "import geom\nimport utils\n\nfn main() {\n    println(\"area ${area()}\")\n    println(label())\n}\n",
            ),
            ("src/utils.nova", "pub fn label() -> String {\n    \"app utils\"\n}\n"),
        ],
    );
    let a = check(Program::for_package(&app, Roots::Program));
    assert!(a.diagnostics.is_empty(), "{}", messages(&a));
    let paths = paths(&a);
    assert_eq!(paths.len(), 4, "{paths:?}");
    assert!(paths[1].ends_with("geom/src/lib.nova"), "{paths:?}");
    assert!(paths[2].ends_with("app/src/utils.nova"), "{paths:?}");
    assert!(paths[3].ends_with("geom/src/utils.nova"), "{paths:?}");
    let ids = [PackageId(0), PackageId(1), PackageId(0), PackageId(1)];
    assert_eq!(a.module_packages, ids.map(Some));
}

#[test]
fn src_utils_and_tests_utils_are_two_modules() {
    let dir = fresh("src-and-tests");
    write(
        &dir,
        &[
            ("nova.toml", manifest("app", "").as_str()),
            ("src/main.nova", "import utils\n\nfn main() {\n    println(label())\n}\n"),
            ("src/utils.nova", "pub fn label() -> String {\n    \"src\"\n}\n"),
            (
                "tests/api.nova",
                "import utils\n\n@test\nfn the_tests_utils_is_seen() {\n    assert_eq(helper(), \"tests\")\n}\n",
            ),
            ("tests/utils.nova", "pub fn helper() -> String {\n    \"tests\"\n}\n"),
        ],
    );
    let program = Program::for_package(&dir, Roots::Test);
    let a = analyze_program(program, &DiskSources, &options(true)).unwrap();
    assert!(a.diagnostics.is_empty(), "{}", messages(&a));
    // The roots, then what `main` imports.
    let paths = paths(&a);
    assert_eq!(paths.len(), 4, "{paths:?}");
    assert!(paths[1].ends_with("tests/api.nova"), "{paths:?}");
    assert!(paths[2].ends_with("tests/utils.nova"), "{paths:?}");
    assert!(paths[3].ends_with("src/utils.nova"), "{paths:?}");
}

#[test]
fn a_file_and_a_dependency_with_one_name_is_e0004() {
    let dir = fresh("clash");
    geom(&dir);
    let app = dir.join("app");
    write(
        &app,
        &[
            ("nova.toml", manifest("app", GEOM_DEPENDENCY).as_str()),
            ("src/main.nova", "import geom\n\nfn main() {}\n"),
            ("src/geom.nova", "pub fn area() -> Int {\n    1\n}\n"),
        ],
    );
    let a = check(Program::for_package(&app, Roots::Program));
    assert_eq!(codes(&a), ["E0004"], "{}", messages(&a));
    let message = &a.diagnostics[0].message;
    assert!(
        message.contains("`geom` is both a module of this package and a dependency"),
        "{message}"
    );
    assert!(message.contains("geom.nova") && message.contains("nova.toml:7"), "{message}");
}

#[test]
fn a_dev_dependency_imported_from_src_is_e0001_with_a_note() {
    let dir = fresh("dev-from-src");
    geom(&dir);
    let app = dir.join("app");
    let extra = "\n[dev-dependencies]\ngeom = { path = \"../geom\" }\n";
    write(
        &app,
        &[
            ("nova.toml", manifest("app", extra).as_str()),
            ("src/main.nova", "import geom\n\nfn main() {}\n"),
        ],
    );
    let a = check(Program::for_package(&app, Roots::Program));
    assert_eq!(codes(&a), ["E0001"], "{}", messages(&a));
    assert!(a.diagnostics[0].message.contains("cannot find module `geom`"));
    assert_eq!(
        a.diagnostics[0].notes,
        ["`geom` is a dev-dependency, which only `tests/` files can import"]
    );
}

#[test]
fn an_import_matches_a_files_case_exactly() {
    let dir = fresh("case");
    write(
        &dir,
        &[
            ("main.nova", "import Utils\n\nfn main() {}\n"),
            ("utils.nova", "pub fn f() -> Int {\n    1\n}\n"),
        ],
    );
    let a = check(Program::loose(&dir.join("main.nova")));
    assert_eq!(codes(&a), ["E0001"], "{}", messages(&a));
    assert!(a.diagnostics[0].message.contains("cannot find module `Utils`"));
}

#[test]
fn tests_files_import_the_package_a_dev_dependency_and_each_other() {
    let dir = fresh("tests-imports");
    geom(&dir);
    write(
        &dir.join("helper"),
        &[
            ("nova.toml", manifest("helper", "").as_str()),
            ("src/lib.nova", "pub fn twice(n: Int) -> Int {\n    n * 2\n}\n"),
        ],
    );
    let app = dir.join("app");
    let extra = "\n[dependencies]\ngeom = { path = \"../geom\" }\n\n\
                 [dev-dependencies]\nhelper = { path = \"../helper\" }\n";
    write(
        &app,
        &[
            ("nova.toml", manifest("app", extra).as_str()),
            ("src/lib.nova", "pub fn name() -> String {\n    \"app\"\n}\n"),
            (
                "tests/api.nova",
                "import app\nimport geom\nimport helper\nimport util\n\n@test\n\
                 fn sees_all_four() {\n    assert_eq(name(), \"app\")\n    \
                 assert_eq(area(), 9)\n    assert_eq(twice(2), 4)\n    assert_eq(one(), 1)\n}\n",
            ),
            ("tests/util.nova", "pub fn one() -> Int {\n    1\n}\n"),
        ],
    );
    let program = Program::for_package(&app, Roots::Test);
    let a = analyze_program(program, &DiskSources, &options(true)).unwrap();
    assert!(a.diagnostics.is_empty(), "{}", messages(&a));
}

#[test]
fn a_file_argument_in_src_sees_its_packages_dependencies() {
    let dir = fresh("file-argument");
    geom(&dir);
    let app = dir.join("app");
    write(
        &app,
        &[
            ("nova.toml", manifest("app", GEOM_DEPENDENCY).as_str()),
            ("src/main.nova", "import geom\n\nfn main() {\n    println(\"${area()}\")\n}\n"),
        ],
    );
    let entry = app.join("src").join("main.nova");
    let a = analyze(&entry, &DiskSources, &options(false)).unwrap();
    assert!(a.diagnostics.is_empty(), "{}", messages(&a));
    assert_eq!(a.modules.len(), 3, "{:?}", paths(&a));
}

#[test]
fn a_loose_file_beside_a_broken_manifest_reads_no_manifest() {
    let dir = fresh("loose");
    write(
        &dir,
        &[
            ("nova.toml", "this is not [ valid toml"),
            ("hello.nova", "import helper\n\nfn main() {\n    println(greeting())\n}\n"),
            ("helper.nova", "pub fn greeting() -> String {\n    \"hi\"\n}\n"),
        ],
    );
    let program = Program::for_file(&dir.join("hello.nova"));
    assert!(program.graph.is_none() && program.diagnostics.is_empty());
    let a = check(program);
    assert!(a.diagnostics.is_empty(), "{}", messages(&a));
}

#[test]
fn an_error_in_a_dependency_is_reported_in_its_own_file() {
    // Review Focus 2: the CLI renders this analysis's diagnostics the same
    // way, through the same `FileDb`.
    let dir = fresh("dependency-error");
    write(
        &dir.join("geom"),
        &[
            ("nova.toml", manifest("geom", "").as_str()),
            ("src/lib.nova", "pub fn area() -> Int {\n    \"nine\"\n}\n"),
        ],
    );
    let app = dir.join("app");
    write(
        &app,
        &[
            ("nova.toml", manifest("app", GEOM_DEPENDENCY).as_str()),
            ("src/main.nova", "import geom\n\nfn main() {\n    println(\"${area()}\")\n}\n"),
        ],
    );
    let a = check(Program::for_package(&app, Roots::Program));
    assert_eq!(a.diagnostics.len(), 1, "{}", messages(&a));
    let rendered = nova_diagnostics::render::render_to_string(&a.db, &a.diagnostics);
    let place = Path::new("geom").join("src").join("lib.nova");
    assert!(rendered.contains(&format!("{}:", place.display())), "{rendered}");
}

#[test]
fn a_library_without_a_program_is_checked_as_a_module() {
    let dir = fresh("library-only");
    geom(&dir);
    let program = Program::for_package(&dir.join("geom"), Roots::Check);
    assert!(!program.runs);
    // MIR would find no `main`: E0601.
    let a = check(program);
    assert!(a.diagnostics.is_empty(), "{}", messages(&a));
}

#[test]
fn a_graph_error_stops_the_analysis_before_loading() {
    let dir = fresh("graph-error");
    let extra = "\n[dependencies]\ngeom = { path = \"../nowhere\" }\n";
    write(
        &dir,
        &[
            ("nova.toml", manifest("app", extra).as_str()),
            ("src/main.nova", "fn main() {}\n"),
        ],
    );
    let a = check(Program::for_package(&dir, Roots::Program));
    assert_eq!(codes(&a), ["M0007"], "{}", messages(&a));
    assert!(a.modules.is_empty());
}
```

- [ ] **Step 2: Run them to verify they fail**

```bash
cd /d/Projects/nona/nova && cargo test --locked -p nova-driver --test packages 2>&1 | grep -E "^error" | head -3
```

Expected: `error[E0432]: unresolved imports` naming `analyze_program`,
`Program` and `Roots`, and `nova_pm` unresolved (E0433).

- [ ] **Step 3: Write `program.rs`**

`crates/nova-driver/src/program.rs`:

```rust
//! What the driver loads (spec
//! `docs/superpowers/specs/2026-10-08-phase-3-3a-local-packages-design.md`
//! §4): a [`Program`] names its roots and carries the package graph, and
//! [`load_program`] decides what each `import` names.

use std::collections::{HashMap, HashSet, VecDeque};
use std::path::{Path, PathBuf};

use nova_diagnostics::{Diagnostic, FileDb, FileId, Severity, Span};
use nova_pm::{Edge, Graph, GraphPackage, PackageId};
use nova_resolver::ImportTarget;

use crate::analyze::Sources;

/// The roots a command takes (spec §4.4's table), each only if it exists.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Roots {
    /// `src/main.nova`: `nova run` and `nova build`.
    Program,
    /// `src/main.nova`, then `src/lib.nova`: `nova check`.
    Check,
    /// Those, then `tests/*.nova` sorted by name: `nova test` and the
    /// language server.
    Test,
}

/// Where a root is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RootKind {
    /// Directly in the root package's `src/`.
    Src,
    /// Directly in the root package's `tests/`.
    Tests,
    /// In no package (spec §4.1).
    Loose,
}

/// A file the loader starts from.
#[derive(Debug, Clone)]
pub struct Root {
    pub path: PathBuf,
    pub kind: RootKind,
}

/// What to load (spec §4.4).
pub struct Program {
    /// Every manifest the graph read. The loader adds every source file.
    pub db: FileDb,
    /// `None` for a loose program, or when the root's manifest cannot be
    /// read or parsed.
    pub graph: Option<Graph>,
    /// The entry first.
    pub roots: Vec<Root>,
    /// Whether MIR runs: the entry is `src/main.nova`, or a loose file.
    pub runs: bool,
    /// The graph's diagnostics.
    pub diagnostics: Vec<Diagnostic>,
}

impl Program {
    /// A loose program: `path`, and the files beside it that it imports.
    pub fn loose(path: &Path) -> Program {
        Program {
            db: FileDb::new(),
            graph: None,
            roots: vec![Root {
                path: path.to_path_buf(),
                kind: RootKind::Loose,
            }],
            runs: true,
            diagnostics: Vec::new(),
        }
    }

    /// The program whose entry is `path` (spec §4.1). It is a module of its
    /// package when it is directly in a package's `src/` or `tests/`, and
    /// otherwise loose, reading no manifest.
    pub fn for_file(path: &Path) -> Program {
        let Some((root, kind)) = package_of(path) else {
            return Program::loose(path);
        };
        let mut db = FileDb::new();
        let (graph, diagnostics) = nova_pm::graph(&root, &mut db);
        let runs = kind == RootKind::Src && path.file_stem().is_some_and(|stem| stem == "main");
        Program {
            db,
            graph,
            roots: vec![Root {
                path: path.to_path_buf(),
                kind,
            }],
            runs,
            diagnostics,
        }
    }

    /// The package whose `nova.toml` is in `root` (empty for the current
    /// directory), with the roots `roots` names that exist.
    pub fn for_package(root: &Path, roots: Roots) -> Program {
        let mut db = FileDb::new();
        let (graph, diagnostics) = nova_pm::graph(root, &mut db);
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

    /// Whether the graph found an error.
    pub fn has_errors(&self) -> bool {
        self.diagnostics
            .iter()
            .any(|d| d.severity == Severity::Error)
    }
}

/// `relative`, a `/`-separated path, under `root`. Under an empty root it
/// is `relative` as written, so `nova run` at a project's root still names
/// `src/main.nova` on every system, as it always has.
fn under(root: &Path, relative: &str) -> PathBuf {
    if root.as_os_str().is_empty() {
        PathBuf::from(relative)
    } else {
        relative.split('/').fold(root.to_path_buf(), |path, part| path.join(part))
    }
}

/// The package `path` is a module of, and where in it, when `path` is
/// directly in the `src/` or `tests/` of a directory holding a `nova.toml`
/// (spec §4.1). The language server finds its projects the same way.
pub fn package_of(path: &Path) -> Option<(PathBuf, RootKind)> {
    let parent = path.parent()?;
    let kind = match parent.file_name()?.to_str()? {
        "src" => RootKind::Src,
        "tests" => RootKind::Tests,
        _ => return None,
    };
    let root = parent.parent()?;
    root.join(nova_pm::MANIFEST)
        .is_file()
        .then(|| (root.to_path_buf(), kind))
}

/// The `*.nova` files directly in `dir`, sorted by name (spec §4.4). A
/// missing directory has none.
fn test_files(dir: &Path) -> Vec<PathBuf> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut files: Vec<PathBuf> = entries
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "nova") && path.is_file())
        .collect();
    files.sort();
    files
}

/// One module [`load_program`] read.
pub(crate) struct Loaded {
    /// A label for messages (spec §4.5): `utils`, `tests/utils`, `geom` or
    /// `geom/utils`.
    pub name: String,
    pub path: PathBuf,
    pub file: FileId,
    pub ast: nova_ast::File,
    /// Its package. `None` for a loose module.
    pub package: Option<PackageId>,
    /// Whether it is directly in its package's `tests/`.
    pub test_file: bool,
    /// What each of its imports names, by first segment (spec §4.3).
    pub imports: HashMap<String, ImportTarget>,
}

/// What [`load_program`] found.
#[derive(Default)]
pub(crate) struct Load {
    /// In load order: `modules[i]` is `ModuleId(i)`.
    pub modules: Vec<Loaded>,
    /// Lex, parse and import diagnostics, in load order.
    pub diagnostics: Vec<Diagnostic>,
    /// The names of the items the parser dropped.
    pub dropped: Vec<String>,
}

/// A module's identity (spec §4.2).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum Place {
    /// A file directly in a package's `src/`, or `tests/` when `tests`.
    Package {
        package: PackageId,
        tests: bool,
        stem: String,
    },
    /// A loose file.
    Loose { dir: PathBuf, stem: String },
}

impl Place {
    fn package(&self) -> Option<PackageId> {
        match self {
            Place::Package { package, .. } => Some(*package),
            Place::Loose { .. } => None,
        }
    }

    fn is_test(&self) -> bool {
        matches!(self, Place::Package { tests: true, .. })
    }
}

/// What an import names, before every module is loaded.
enum Pending {
    Place(Place),
    Reported,
}

/// What [`Loader::find`] found for one import.
enum Found {
    /// A module: its identity and its file.
    Module(Place, PathBuf),
    /// An error, reported here (spec §4.3, rules 3–5).
    Reported(Diagnostic),
    /// Nothing: the resolver reports "cannot find module".
    Nothing,
}

/// Read, lex and parse every module the roots reach, breadth first from the
/// roots in order (spec §4.4), deciding what each import names (§4.3).
///
/// A module that cannot be read is skipped, and an import of it is then
/// "cannot find module". `Err` only when the entry itself cannot be read.
/// Without a graph, every root is loose.
pub(crate) fn load_program(
    graph: Option<&Graph>,
    roots: &[Root],
    sources: &dyn Sources,
    db: &mut FileDb,
) -> std::io::Result<Load> {
    let mut loader = Loader {
        graph,
        sources,
        listings: HashMap::new(),
    };
    let mut load = Load::default();
    let mut pending: Vec<Vec<(String, Pending)>> = Vec::new();
    let mut places: HashMap<Place, usize> = HashMap::new();
    let mut tried: HashSet<Place> = HashSet::new();
    let mut queue: VecDeque<(Place, PathBuf)> = roots
        .iter()
        .map(|root| (loader.root_place(root), root.path.clone()))
        .collect();
    let mut is_entry = true;

    while let Some((place, path)) = queue.pop_front() {
        let entry = std::mem::replace(&mut is_entry, false);
        if !tried.insert(place.clone()) {
            continue;
        }
        let source = match sources.read(&path) {
            Ok(source) => source,
            Err(e) if entry => return Err(e),
            Err(_) => continue,
        };
        let file = db.add(path.display().to_string(), source.as_str());

        let (tokens, lex_errors) = nova_lexer::lex(&source, file);
        load.diagnostics.extend(lex_errors.iter().map(|e| {
            Diagnostic::error("L0001", e.to_string()).with_primary_label(e.span(), "here")
        }));
        let parsed = nova_parser::parse_recovering(&tokens, file);
        load.diagnostics.extend(parsed.errors.iter().map(|e| {
            Diagnostic::error("P0001", e.to_string()).with_primary_label(e.span(), "here")
        }));
        load.dropped
            .extend(parsed.dropped.into_iter().map(|n| n.value));

        // Only a single-segment import names a module; the resolver rejects
        // the others.
        let mut imports = Vec::new();
        for item in &parsed.file.items {
            let nova_ast::Item::Import(import) = &item.value else {
                continue;
            };
            let [segment] = import.path.value.segments.as_slice() else {
                continue;
            };
            let name = segment.value.clone();
            match loader.find(&place, &name, import.path.span, db) {
                Found::Module(target, target_path) => {
                    queue.push_back((target.clone(), target_path));
                    imports.push((name, Pending::Place(target)));
                }
                Found::Reported(diagnostic) => {
                    load.diagnostics.push(diagnostic);
                    imports.push((name, Pending::Reported));
                }
                Found::Nothing => {}
            }
        }
        places.insert(place.clone(), load.modules.len());
        load.modules.push(Loaded {
            name: loader.label(&place),
            path,
            file,
            ast: parsed.file,
            package: place.package(),
            test_file: place.is_test(),
            imports: HashMap::new(),
        });
        pending.push(imports);
    }

    for (module, imports) in load.modules.iter_mut().zip(pending) {
        for (name, target) in imports {
            let target = match target {
                Pending::Place(place) => match places.get(&place) {
                    Some(&index) => ImportTarget::Module(index),
                    // It could not be read.
                    None => continue,
                },
                Pending::Reported => ImportTarget::Reported,
            };
            module.imports.insert(name, target);
        }
    }
    Ok(load)
}

struct Loader<'a> {
    graph: Option<&'a Graph>,
    sources: &'a dyn Sources,
    /// Each directory's file names, read once (spec §4.3, decision 5).
    listings: HashMap<PathBuf, Vec<String>>,
}

impl<'a> Loader<'a> {
    fn root_place(&self, root: &Root) -> Place {
        let stem = crate::FrontendContext::module_name(&root.path);
        match (root.kind, self.graph) {
            (RootKind::Src, Some(_)) => Place::Package {
                package: PackageId(0),
                tests: false,
                stem,
            },
            (RootKind::Tests, Some(_)) => Place::Package {
                package: PackageId(0),
                tests: true,
                stem,
            },
            _ => Place::Loose {
                dir: root.path.parent().map(Path::to_path_buf).unwrap_or_default(),
                stem,
            },
        }
    }

    /// The module's label (spec §4.5).
    fn label(&self, place: &Place) -> String {
        match place {
            Place::Loose { stem, .. } => stem.clone(),
            Place::Package {
                package: PackageId(0),
                tests,
                stem,
            } => {
                if *tests {
                    format!("tests/{stem}")
                } else {
                    stem.clone()
                }
            }
            Place::Package { package, stem, .. } => {
                let import = &self.graph().package(*package).import_name;
                if stem == "lib" {
                    import.clone()
                } else {
                    format!("{import}/{stem}")
                }
            }
        }
    }

    /// The graph, borrowed for `'a` rather than from `self`, so `has` can
    /// run while it is held.
    fn graph(&self) -> &'a Graph {
        self.graph.expect("a package's module has a graph")
    }

    /// What `import name`, at `at` in the module at `from`, names (spec
    /// §4.3).
    fn find(&mut self, from: &Place, name: &str, at: Span, db: &FileDb) -> Found {
        let (package, tests) = match from {
            Place::Loose { dir, .. } => {
                if !self.has(dir, name) {
                    return Found::Nothing;
                }
                let place = Place::Loose {
                    dir: dir.clone(),
                    stem: name.to_string(),
                };
                return Found::Module(place, dir.join(format!("{name}.nova")));
            }
            Place::Package { package, tests, .. } => (*package, *tests),
        };
        let graph = self.graph();
        let dir = module_dir(graph.package(package), tests);
        let edge = visible_edges(graph, package, tests)
            .into_iter()
            .find(|edge| edge.import_name == name);
        let file = self.has(&dir, name).then(|| {
            (
                Place::Package {
                    package,
                    tests,
                    stem: name.to_string(),
                },
                dir.join(format!("{name}.nova")),
            )
        });
        match (file, edge) {
            (Some((_, path)), Some(edge)) => Found::Reported(clash(name, &path, &edge, at, db)),
            (Some((place, path)), None) => Found::Module(place, path),
            (None, Some(edge)) => {
                let target = graph.package(edge.package);
                if !target.has_lib {
                    // The package itself, seen from its own tests (rule 5).
                    return Found::Reported(
                        Diagnostic::error("E0001", format!("cannot find module `{name}`"))
                            .with_primary_label(at, "no such module")
                            .with_note(format!("`{}` has no src/lib.nova", target.name)),
                    );
                }
                Found::Module(
                    Place::Package {
                        package: edge.package,
                        tests: false,
                        stem: "lib".to_string(),
                    },
                    module_dir(target, false).join("lib.nova"),
                )
            }
            (None, None) => {
                let dev = !tests
                    && package == PackageId(0)
                    && graph
                        .root()
                        .dev_dependencies
                        .iter()
                        .any(|edge| edge.import_name == name);
                if dev {
                    Found::Reported(
                        Diagnostic::error("E0001", format!("cannot find module `{name}`"))
                            .with_primary_label(at, "no such module")
                            .with_note(format!(
                                "`{name}` is a dev-dependency, which only `tests/` files can import"
                            )),
                    )
                } else {
                    Found::Nothing
                }
            }
        }
    }

    /// Whether `dir` holds `<stem>.nova`, its name matching exactly (spec
    /// §4.3). A name the disk does not hold in any case may be an editor's
    /// unsaved buffer, read through `Sources`.
    fn has(&mut self, dir: &Path, stem: &str) -> bool {
        let name = format!("{stem}.nova");
        let listing = self.listings.entry(dir.to_path_buf()).or_insert_with(|| {
            std::fs::read_dir(dir)
                .map(|entries| {
                    entries
                        .filter_map(Result::ok)
                        .filter_map(|entry| entry.file_name().into_string().ok())
                        .collect()
                })
                .unwrap_or_default()
        });
        if listing.iter().any(|n| *n == name) {
            return true;
        }
        if listing.iter().any(|n| n.eq_ignore_ascii_case(&name)) {
            return false;
        }
        self.sources.read(&dir.join(&name)).is_ok()
    }
}

/// A package's `src/`, or `tests/` when `tests`.
fn module_dir(package: &GraphPackage, tests: bool) -> PathBuf {
    package.dir.join(if tests { "tests" } else { "src" })
}

/// The dependencies a module of `package` sees (spec §4.3). A test module
/// also sees the root's dev-dependencies, and the root itself by its import
/// name. Only the root's test modules are ever loaded.
fn visible_edges(graph: &Graph, package: PackageId, tests: bool) -> Vec<Edge> {
    let p = graph.package(package);
    let mut edges = p.dependencies.clone();
    if tests {
        edges.extend(p.dev_dependencies.iter().cloned());
        edges.push(Edge {
            import_name: p.import_name.clone(),
            package,
            span: p.manifest.package.span,
        });
    }
    edges
}

/// E0004 (spec §4.3, rule 3): `name` is both `file` and the dependency
/// `edge`. The message names both, so it reads in full where a related
/// label cannot show.
fn clash(name: &str, file: &Path, edge: &Edge, at: Span, db: &FileDb) -> Diagnostic {
    let manifest = db.get_name(edge.span.file).unwrap_or("nova.toml");
    let line = db
        .location(edge.span.file, edge.span.start)
        .map_or(0, |(line, _)| line);
    Diagnostic::error(
        "E0004",
        format!(
            "`{name}` is both a module of this package and a dependency: {}, and the entry \
             at {manifest}:{line}",
            file.display()
        ),
    )
    .with_primary_label(at, "this import is ambiguous")
    .with_secondary_label(edge.span, "the dependency")
}
```

- [ ] **Step 4: Use it in `analyze.rs`**

In `crates/nova-driver/src/analyze.rs`:
- replace the module comment's last paragraph ("The CLI's entry points
  share [`load_program`] …") with:

  ```rust
  //! The CLI's entry points share [`crate::program::load_program`] with it,
  //! through [`DiskSources`], and keep their own staged behaviour.
  ```
- replace `use std::collections::{HashSet, VecDeque};` and the `use`
  lines after it with:

  ```rust
  use std::path::{Path, PathBuf};

  use nova_diagnostics::{Diagnostic, FileDb, FileId, Severity};
  use nova_pm::{Graph, PackageId};
  use nova_resolver::{Definitions, ModuleSource};
  use nova_typeck::{CheckOptions, ProbePoint, ProbeResult};

  use crate::program::{load_program, Load, Program};
  ```
- in `struct Analysis`, add after `pub modules: Vec<(FileId, PathBuf)>,`:

  ```rust
      /// Each module's package, parallel to `modules`. `None` for a loose
      /// module (spec 3.3a §6).
      pub module_packages: Vec<Option<PackageId>>,
      /// The package graph, as far as it resolved.
      pub graph: Option<Graph>,
  ```
- delete `pub(crate) struct Loaded { … }` and `pub(crate) fn load_program(…) { … }`,
  with their doc comments;
- replace `pub fn analyze(…) { … }` with:

  ```rust
  /// Run the front end on `entry` for the language server (spec §3): the
  /// program [`Program::for_file`] finds for it.
  pub fn analyze(
      entry: &Path,
      sources: &dyn Sources,
      options: &Options,
  ) -> std::io::Result<Analysis> {
      analyze_program(Program::for_file(entry), sources, options)
  }

  /// Run the front end on `program` (spec 3.3a §4.4, §6). MIR runs only
  /// when the program runs and `options.module_only` is off.
  pub fn analyze_program(
      program: Program,
      sources: &dyn Sources,
      options: &Options,
  ) -> std::io::Result<Analysis> {
      let Program {
          mut db,
          graph,
          roots,
          runs,
          mut diagnostics,
      } = program;
      let stop = |diags: &[Diagnostic]| !options.keep_going && has_error(diags);
      let load = if stop(&diagnostics) {
          Load::default()
      } else {
          load_program(graph.as_ref(), &roots, sources, &mut db)?
      };
      diagnostics.extend(load.diagnostics);
      let dropped = load.dropped;
      let mut modules = load.modules;
      let mut analysis = Analysis {
          db,
          diagnostics: Vec::new(),
          modules: modules.iter().map(|m| (m.file, m.path.clone())).collect(),
          module_packages: modules.iter().map(|m| m.package).collect(),
          graph,
          definitions: None,
          module: None,
          probe: ProbeResult::default(),
      };
      if stop(&diagnostics) {
          analysis.diagnostics = diagnostics;
          return Ok(analysis);
      }

      if !options.tests {
          crate::strip_test_functions(&mut modules);
      }
      let std_files: Vec<FileId> = nova_resolver::STD_MODULES
          .iter()
          .map(|&(name, src)| {
              let short = name.strip_prefix("$std.").unwrap_or(name);
              analysis.db.add(format!("<std/{short}>"), src)
          })
          .collect();
      let extra_std = options.tests.then(|| {
          let (name, src) = nova_resolver::STD_TEST_MODULE;
          let short = name.strip_prefix("$std.").unwrap_or(name);
          let file = analysis.db.add(format!("<std/{short}>"), src);
          (nova_resolver::STD_TEST_MODULE, file)
      });
      let module_sources: Vec<ModuleSource> = modules
          .iter()
          .map(|m| ModuleSource {
              name: m.name.clone(),
              file: &m.ast,
              imports: m.imports.clone(),
          })
          .collect();
      let resolved = nova_resolver::resolve_program(&module_sources, &std_files, extra_std);
      diagnostics.extend(resolved.diagnostics);
      if stop(&diagnostics) {
          analysis.diagnostics = diagnostics;
          analysis.definitions = Some(resolved.definitions);
          return Ok(analysis);
      }

      let probe = options.probe.as_ref().and_then(|p| {
          analysis
              .modules
              .iter()
              .find(|(_, path)| sources.same_file(path, &p.path))
              .map(|(file, _)| ProbePoint {
                  file: *file,
                  offset: p.offset,
              })
      });
      let checked = nova_typeck::check_with(
          &resolved.file,
          &resolved.definitions,
          &CheckOptions { probe },
      );
      diagnostics.extend(checked.diagnostics);
      // MIR lowering assumes a well-formed program (spec §3.3), and a
      // program without an entry `main` is checked as a module (3.3a §4.4).
      let module_only = options.module_only || !runs;
      if !module_only && !has_error(&diagnostics) {
          if let Err(mir) = nova_mir::lower_module(&checked.module) {
              diagnostics.extend(mir);
          }
      }
      if options.keep_going {
          diagnostics.retain(|d| !about_a_dropped_name(d, &dropped));
      }
      analysis.diagnostics = diagnostics;
      analysis.definitions = Some(resolved.definitions);
      analysis.module = Some(checked.module);
      analysis.probe = checked.probe;
      Ok(analysis)
  }
  ```

- [ ] **Step 5: The entry points and `FrontendContext`**

In `crates/nova-driver/src/lib.rs`:
- after `mod link;`, add `mod program;`;
- replace the `pub use analyze::…;` line with:

  ```rust
  pub use analyze::{analyze, analyze_program, Analysis, DiskSources, Options, Probe, Sources};
  pub use program::{package_of, Program, Root, RootKind, Roots};
  ```
- add `use program::Loaded;` after `use nova_resolver::…;`;
- replace the body of `check_file` with `check_program(Program::for_file(path))`,
  keeping its doc comment, and add after it:

  ```rust
  /// [`check_file`] on `program` (spec 3.3a §5.1). MIR runs only when the
  /// program runs: its entry is `src/main.nova`, or a loose file.
  pub fn check_program(program: Program) -> Result<Outcome<()>> {
      let runs = program.runs;
      let mut ctx = FrontendContext::new(program)?;
      let Some((module, _tests, _fresh_def_id)) = ctx.check(false)? else {
          return Ok(Outcome::Failed { errors: ctx.errors });
      };
      if runs {
          if let Err(diags) = nova_mir::lower_module(&module) {
              ctx.render(&diags);
          }
      }
      if ctx.errors > 0 {
          Ok(Outcome::Failed { errors: ctx.errors })
      } else {
          Ok(Outcome::Ok(()))
      }
  }
  ```
- `compile_file`: rename it `compile_program(program: Program)`, with
  `lower_to_mir(program)?` in place of `lower_to_mir(path)?`, and add above
  it:

  ```rust
  /// Compile a file to native code via the Cranelift JIT.
  pub fn compile_file(path: &Path) -> Result<Outcome<CompiledProgram>> {
      compile_program(Program::for_file(path))
  }

  /// [`compile_file`] on `program`.
  ```
- `build_file` and `build_file_release` likewise:
  - each becomes `build_program(program: Program, output: &Path)` and
    `build_program_release(program: Program, output: &Path)`, with
    `lower_to_mir(program)?`;
  - each keeps a path wrapper with its doc comment:

    ```rust
    pub fn build_file(path: &Path, output: &Path) -> Result<Outcome<PathBuf>> {
        build_program(Program::for_file(path), output)
    }
    ```

    and the same for `build_file_release`, which calls
    `build_program_release`;
  - the renamed functions' doc comment is "[`build_file`] on `program`." and
    "[`build_file_release`] on `program`.".
- `build_test_binary`: its body becomes
  `build_test_program(Program::for_file(path))`. Add after it, carrying the
  old body:

  ```rust
  /// [`build_test_binary`] on `program`: the test binary of its roots.
  pub fn build_test_program(program: Program) -> Result<(PathBuf, Vec<TestFn>)> {
      let entry = program
          .roots
          .first()
          .map(|root| root.path.clone())
          .unwrap_or_default();
      let mut ctx = FrontendContext::new(program)?;
  ```

  The old body follows from `let Some((mut module, tests, fresh_def_id))`,
  with `path_fingerprint(&entry)` and
  `FrontendContext::module_name(&entry)` in place of `path`'s.
- `lower_to_mir(path: &Path)` becomes `lower_to_mir(program: Program)`,
  whose first line is `let mut ctx = FrontendContext::new(program)?;`.
- `run_file` becomes a wrapper,
  `run_program(Program::for_file(path), args)`, and `run_program(program,
  args)` carries its body with `compile_program(program)?`. Its doc comment
  is "[`run_file`] on `program`.".
- replace `struct FrontendContext`, and in `impl FrontendContext` the
  functions `load` and `load_modules`, with:

  ```rust
  /// Shared front-end state: what to load, the file database, and error
  /// accounting.
  struct FrontendContext {
      db: FileDb,
      graph: Option<nova_pm::Graph>,
      roots: Vec<Root>,
      errors: usize,
  }

  impl FrontendContext {
      /// Take `program`, rendering its graph's diagnostics. When they hold
      /// no error, the entry must exist: imported modules are read lazily.
      fn new(program: Program) -> Result<Self> {
          let Program {
              db,
              graph,
              roots,
              diagnostics,
              ..
          } = program;
          let mut ctx = FrontendContext {
              db,
              graph,
              roots,
              errors: 0,
          };
          ctx.render(&diagnostics);
          if ctx.errors == 0 {
              let entry = ctx
                  .entry()
                  .context("nothing to compile: the package has no source file for this")?;
              std::fs::File::open(entry)
                  .with_context(|| format!("failed to open {}", entry.display()))?;
          }
          Ok(ctx)
      }

      /// The first root.
      fn entry(&self) -> Option<&Path> {
          self.roots.first().map(|root| root.path.as_path())
      }
  ```

  `render` and `module_name` stay as they are. Then:

  ```rust
      /// Load, lex and parse every module the roots reach, from disk,
      /// rendering the lex, parse and import diagnostics. The loading itself
      /// is [`program::load_program`], which the language server shares.
      fn load_modules(&mut self) -> Result<Vec<Loaded>> {
          let entry = self.entry().map(Path::to_path_buf).unwrap_or_default();
          let load =
              program::load_program(self.graph.as_ref(), &self.roots, &DiskSources, &mut self.db)
                  .with_context(|| format!("failed to read {}", entry.display()))?;
          self.render(&load.diagnostics);
          Ok(load.modules)
      }
  ```
- in `fn check`:
  - its first statement becomes:

    ```rust
            // The graph's errors were rendered by `new`.
            if self.errors > 0 {
                return Ok(None);
            }
            let mut modules = self.load_modules()?;
    ```
  - its `sources` become:

    ```rust
            let sources: Vec<ModuleSource> = modules
                .iter()
                .map(|m| ModuleSource {
                    name: m.name.clone(),
                    file: &m.ast,
                    imports: m.imports.clone(),
                })
                .collect();
    ```

    replacing Task 3's `name_imports` call.
- `strip_test_functions(modules: &mut [(String, nova_ast::File)])` becomes
  `strip_test_functions(modules: &mut [Loaded])`, with its loop
  `for module in modules.iter_mut() { module.ast.items.retain(…) }`. Its
  doc comment stays.

In `crates/nova-driver/Cargo.toml`, add after `nova-mir = …`:

```toml
nova-pm = { path = "../nova-pm" }
```

- [ ] **Step 6: Run the tests to verify they pass**

```bash
cd /d/Projects/nona/nova && P=/c/Users/SAKEER~1/AppData/Local/Temp/gcm/p33 && cargo test --locked -p nova-driver > $P/t4.txt 2>&1; echo "exit=$?"; python -X utf8 $P/count.py $P/t4.txt; grep -E "FAILED|panicked|^warning|^error" $P/t4.txt | head; git diff --stat -- Cargo.lock
```

Expected:
- `exit=0`, 0 failed, the 11 new tests included, no warnings;
- `Cargo.lock` changes by one line, `nova-pm` in `nova-driver`'s entry.

Then the whole CLI and server, since every compile now goes through the
loader. Port 3000 must be free first:

```bash
cd /d/Projects/nona/nova && P=/c/Users/SAKEER~1/AppData/Local/Temp/gcm/p33 && netstat -ano | grep -E "[:.]3000 .*LISTENING"; cargo test --locked -p nova-cli -p nova-lsp > $P/t4-cli.txt 2>&1; echo "exit=$?"; python -X utf8 $P/count.py $P/t4-cli.txt; grep -E "FAILED|panicked" $P/t4-cli.txt | head
```

Expected: no `LISTENING` line, then `exit=0` and 0 failed. The CLI still
reads the manifest itself until Task 5, so a project's M0006 warning prints
twice in between; `an_unknown_key_warns_and_the_command_proceeds` checks
only that it appears.

- [ ] **Step 7: Commit**

Write `$P/msg-4.txt`:

```
nova-driver: Program, and a loader that knows packages

What to load is now a `Program` (spec
docs/superpowers/specs/2026-10-08-phase-3-3a-local-packages-design.md
§4.4): the roots, the package graph and its diagnostics.
- `Program::for_file` finds a file's package (§4.1). A file directly in a
  package's src/ or tests/ is a module of it; any other file is loose
  and reads no manifest.
- `Program::for_package` takes a command's roots: the program, the
  library, then tests/*.nova sorted by name.

The loader walks from the roots breadth first and decides what each
import names (§4.3):
- a file of the same package and directory;
- else a dependency's lib.nova;
- both is E0004, naming the file and the manifest entry;
- a dev-dependency from src/ is E0001 with a note;
- test modules see dev-dependencies and the package itself.
A file matches only in its exact case, against the directory listing.
Each module gets the import table the resolver reads.

Every path-based entry point keeps its signature and gains a `*_program`
form. MIR runs only when the program runs (its entry is src/main.nova or
a loose file), so a library is checked as a module.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
```

```bash
cd /d/Projects/nona/nova && P=/c/Users/SAKEER~1/AppData/Local/Temp/gcm/p33 && cargo fmt --all && git add crates/nova-driver Cargo.lock && git commit -q -F $P/msg-4.txt && git log -1 --format=%s
```

Expected: `nova-driver: Program, and a loader that knows packages`.

The task's test command: `cargo test --locked -p nova-driver`.

---

### Task 5: Commands on packages

Spec §5.1 and §5.2. The CLI builds a `Program` for each command and stops
reading the manifest itself.

**Files:**
- Modify: `crates/nova-cli/src/project.rs` (`Mode`, `mode`,
  `program_to_run`, `package_name`; `read_manifest` and
  `unresolved_dependencies` go)
- Modify: `crates/nova-cli/src/cmd/run.rs`, `crates/nova-cli/src/cmd/test.rs`
- Modify: `crates/nova-cli/tests/project.rs` (the M0013 test)
- Create: `crates/nova-cli/tests/packages.rs`

**Interfaces:**
- Consumes: Task 4's `Program`, `Roots`, `check_program`, `run_program`,
  `build_program`, `build_program_release` and `build_test_program`.
- Produces:
  - `project::Mode::{File(PathBuf), Project { root, target_dir }}`;
  - `Mode::program(&self, Roots) -> Program`;
  - `project::program_to_run(&Mode) -> Result<Program>`;
  - `project::package_name(&Program) -> String`.
  - `crates/nova-cli/tests/packages.rs`'s helpers, which Task 6 reuses:
    `nova()`, `fresh`, `write`, `manifest`, `stdout`, `stderr`, `geom`.

- [ ] **Step 1: Write the failing tests**

`crates/nova-cli/tests/packages.rs`:

```rust
//! Packages end to end through `nova` (spec
//! `docs/superpowers/specs/2026-10-08-phase-3-3a-local-packages-design.md`
//! §5, §7.3).

use std::path::{Path, PathBuf};

use assert_cmd::Command;

fn nova() -> Command {
    Command::cargo_bin("nova").expect("nova binary builds")
}

/// A fresh, empty directory with a fixed name, so each run replaces the last.
fn fresh(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("nova-packages-{name}"));
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

/// A `nova.toml` for `name`, with `extra` appended.
fn manifest(name: &str, extra: &str) -> String {
    format!("[package]\nname = \"{name}\"\nversion = \"0.1.0\"\nedition = \"2026\"\n{extra}")
}

fn stdout(assert: &assert_cmd::assert::Assert) -> String {
    String::from_utf8_lossy(&assert.get_output().stdout).into_owned()
}

fn stderr(assert: &assert_cmd::assert::Assert) -> String {
    String::from_utf8_lossy(&assert.get_output().stderr).into_owned()
}

const GEOM_DEPENDENCY: &str = "\n[dependencies]\ngeom = { path = \"../geom\" }\n";

/// `dir/geom`: a library whose `lib.nova` uses its own `utils.nova`.
fn geom(dir: &Path) {
    write(
        &dir.join("geom"),
        &[
            ("nova.toml", manifest("geom", "").as_str()),
            (
                "src/lib.nova",
                "import utils\n\npub fn area() -> Int {\n    width() * width()\n}\n",
            ),
            ("src/utils.nova", "pub fn width() -> Int {\n    3\n}\n"),
        ],
    );
}

/// `dir/app`, which depends on `geom` and has its own `utils.nova`.
fn app(dir: &Path) -> PathBuf {
    let app = dir.join("app");
    write(
        &app,
        &[
            ("nova.toml", manifest("app", GEOM_DEPENDENCY).as_str()),
            (
                "src/main.nova",
                "import geom\nimport utils\n\nfn main() {\n    println(\"area ${area()}\")\n    println(label())\n}\n",
            ),
            ("src/utils.nova", "pub fn label() -> String {\n    \"app utils\"\n}\n"),
        ],
    );
    app
}

#[test]
fn an_app_runs_and_builds_against_a_path_library() {
    let dir = fresh("run");
    geom(&dir);
    let app = app(&dir);
    nova()
        .current_dir(&app)
        .arg("run")
        .assert()
        .success()
        .stdout("area 9\napp utils\n");
    nova().current_dir(&app).arg("build").assert().success();
    let exe = app
        .join("target")
        .join("debug")
        .join(format!("app{}", std::env::consts::EXE_SUFFIX));
    let out = std::process::Command::new(&exe).output().expect("run the build");
    assert_eq!(String::from_utf8_lossy(&out.stdout), "area 9\napp utils\n");
}

#[test]
fn run_and_build_are_refused_in_a_library() {
    let dir = fresh("refused");
    geom(&dir);
    for command in ["run", "build"] {
        let out = nova()
            .current_dir(dir.join("geom"))
            .arg(command)
            .assert()
            .failure();
        let err = stderr(&out);
        assert!(
            err.contains(
                "`geom` is a library: it has no src/main.nova; `nova check` and `nova test` work on it"
            ),
            "{command}: {err}"
        );
    }
}

#[test]
fn check_checks_the_library_and_the_program() {
    let dir = fresh("check");
    write(
        &dir,
        &[
            ("nova.toml", manifest("both", "").as_str()),
            ("src/main.nova", "fn main() {}\n"),
            // `main.nova` does not import it.
            ("src/lib.nova", "pub fn area() -> Int {\n    \"nine\"\n}\n"),
        ],
    );
    let out = nova().current_dir(&dir).arg("check").assert().failure();
    let err = stderr(&out);
    assert!(err.contains("lib.nova:"), "{err}");
    geom(&dir);
    nova()
        .current_dir(dir.join("geom"))
        .arg("check")
        .assert()
        .success()
        .stdout("ok: src/lib.nova\n");
}

#[test]
fn test_runs_the_library_and_the_tests_directory() {
    let dir = fresh("test");
    geom(&dir);
    let geom = dir.join("geom");
    write(
        &geom,
        &[
            (
                "src/lib.nova",
                "import utils\n\npub fn area() -> Int {\n    width() * width()\n}\n\n\
                 @test\nfn area_is_nine() {\n    assert_eq(area(), 9)\n}\n",
            ),
            (
                "tests/api.nova",
                "import geom\n\n@test\nfn parses() {\n    assert_eq(area(), 9)\n}\n",
            ),
        ],
    );
    let out = nova().current_dir(&geom).arg("test").assert().success();
    let printed = stdout(&out);
    assert!(printed.contains("running 2 tests"), "{printed}");
    assert!(printed.contains("area_is_nine ... ok"), "{printed}");
    assert!(printed.contains("parses ... ok"), "{printed}");
}

#[test]
fn each_graph_error_is_rendered_and_stops_the_command() {
    let dir = fresh("graph-errors");
    geom(&dir);
    write(&dir.join("program"), &[("nova.toml", manifest("program", "").as_str()), ("src/main.nova", "fn main() {}\n")]);
    write(&dir.join("geometry"), &[("nova.toml", manifest("geometry", "").as_str()), ("src/lib.nova", "")]);
    let cases = [
        ("M0005", "\n[dependencies]\nhttp = \"1.0\"\n"),
        ("M0007", "\n[dependencies]\ngeom = { path = \"../nowhere\" }\n"),
        ("M0008", "\n[dependencies]\ngeom = { path = \"../geometry\" }\n"),
        ("M0009", "\n[dependencies]\nprogram = { path = \"../program\" }\n"),
        ("M0010", "\n[dependencies]\nme = { path = \".\" }\n"),
        ("M0012", "\n[dependencies]\nmatch = { path = \"../geom\" }\n"),
    ];
    for (code, extra) in cases {
        let app = dir.join(format!("app-{code}"));
        write(&app, &[("nova.toml", manifest("app", extra).as_str()), ("src/main.nova", "fn main() {}\n")]);
        let out = nova().current_dir(&app).arg("check").assert().failure();
        let err = stderr(&out);
        assert!(err.contains(code), "{code}: {err}");
    }
    // M0011: two packages named `c`.
    write(&dir.join("c1"), &[("nova.toml", manifest("c", "").as_str()), ("src/lib.nova", "")]);
    write(&dir.join("c2"), &[("nova.toml", manifest("c", "").as_str()), ("src/lib.nova", "")]);
    write(&dir.join("a"), &[("nova.toml", manifest("a", "\n[dependencies]\nc = { path = \"../c1\" }\n").as_str()), ("src/lib.nova", "")]);
    write(&dir.join("b"), &[("nova.toml", manifest("b", "\n[dependencies]\nc = { path = \"../c2\" }\n").as_str()), ("src/lib.nova", "")]);
    let both = "\n[dependencies]\na = { path = \"../a\" }\nb = { path = \"../b\" }\n";
    let app = dir.join("app-M0011");
    write(&app, &[("nova.toml", manifest("app", both).as_str()), ("src/main.nova", "fn main() {}\n")]);
    let out = nova().current_dir(&app).arg("run").assert().failure();
    assert!(stderr(&out).contains("M0011"), "{}", stderr(&out));
    // M0013: neither target.
    let empty = dir.join("empty");
    write(&empty, &[("nova.toml", manifest("empty", "").as_str())]);
    let out = nova().current_dir(&empty).arg("test").assert().failure();
    assert!(stderr(&out).contains("M0013"), "{}", stderr(&out));
}

#[test]
fn a_file_argument_in_src_reads_its_packages_manifest() {
    // A guard: the driver's `Program::for_file` (Task 4) already does this.
    let dir = fresh("file-argument");
    geom(&dir);
    let app = app(&dir);
    nova()
        .current_dir(&app)
        .args(["run", "src/main.nova"])
        .assert()
        .success()
        .stdout("area 9\napp utils\n");
}
```

In `crates/nova-cli/tests/project.rs`, the test
`a_project_without_src_main_nova_names_the_missing_entry` keeps its name.
Its assertion becomes:

```rust
    assert!(
        err.contains("M0013") && err.contains("has neither src/lib.nova nor src/main.nova"),
        "{err}"
    );
```

and a comment above `#[test]` says why:

```rust
/// Phase 3.3a: a package with neither target is M0013 (spec
/// `docs/superpowers/specs/2026-10-08-phase-3-3a-local-packages-design.md`
/// §3.1).
```

- [ ] **Step 2: Run them to verify they fail**

```bash
cd /d/Projects/nona/nova && P=/c/Users/SAKEER~1/AppData/Local/Temp/gcm/p33 && cargo test --locked -p nova-cli --test packages --test project > $P/t5-red.txt 2>&1; grep -E "^test .*(FAILED|ok)$" $P/t5-red.txt | sort
```

Expected: 6 FAILED, and `a_file_argument_in_src_reads_its_packages_manifest`
ok (a guard):
- `an_app_runs_and_builds_against_a_path_library`: the CLI still says
  M0005 "cannot be used yet" of the path dependency;
- `run_and_build_are_refused_in_a_library`: "project `geom` has no
  src/main.nova";
- `check_checks_the_library_and_the_program`: `nova check` succeeds
  without reaching `lib.nova`;
- `test_runs_the_library_and_the_tests_directory`: "project `geom` has
  no src/main.nova";
- `each_graph_error_is_rendered_and_stops_the_command`: M0005 in place of
  M0007;
- `a_project_without_src_main_nova_names_the_missing_entry`: no M0013.

Every other `project.rs` test is ok. The guard passes because a file
argument is file mode, where the CLI reads no manifest of its own, and
Task 4's `Program::for_file` reads the package's.

- [ ] **Step 3: Implement**

`crates/nova-cli/src/project.rs` becomes:

```rust
//! Which program a command works on, and where `build` writes (spec
//! `docs/superpowers/specs/2026-10-07-phase-3-0-foundations-design.md` §6,
//! and for packages
//! `docs/superpowers/specs/2026-10-08-phase-3-3a-local-packages-design.md`
//! §5.1).

use std::path::PathBuf;

use anyhow::{bail, Context, Result};
use nova_driver::{Program, Roots};

/// What a command works on.
pub enum Mode {
    /// One file: a file argument, or `src/main.nova` outside any project.
    File(PathBuf),
    /// A project, found by walking up to its `nova.toml`.
    Project {
        /// Its directory: empty when it is the current directory, so that
        /// paths stay relative there as they always have, and absolute from
        /// anywhere else.
        root: PathBuf,
        /// `target`, relative or absolute in the same way.
        target_dir: PathBuf,
    },
}

impl Mode {
    /// The program the command compiles: a file's (spec 3.3a §4.1), or the
    /// project's with the roots `roots` names. The driver renders the
    /// graph's diagnostics, so a manifest error stops the command there.
    pub fn program(&self, roots: Roots) -> Program {
        match self {
            Mode::File(file) => Program::for_file(file),
            Mode::Project { root, .. } => Program::for_package(root, roots),
        }
    }
}

/// The mode for a command given `file`, or none. A file argument always
/// means file mode; it reads its package's manifest only when it is directly
/// in a package's `src/` or `tests/` (spec 3.3a §4.1). Otherwise the
/// nearest `nova.toml` at or above the current directory makes a project;
/// with none, `src/main.nova` stays the default.
pub fn mode(file: Option<PathBuf>) -> Result<Mode> {
    if let Some(file) = file {
        return Ok(Mode::File(file));
    }
    let cwd = std::env::current_dir().context("reading the current directory")?;
    let Some(root) = nova_pm::find_root(&cwd) else {
        return Ok(Mode::File(PathBuf::from("src/main.nova")));
    };
    // At the root, paths stay relative, exactly as before projects existed.
    let root = if root == cwd { PathBuf::new() } else { root };
    Ok(Mode::Project {
        target_dir: root.join("target"),
        root,
    })
}

/// The program `nova run` and `nova build` compile. A project that is only
/// a library has none, and is refused (spec 3.3a §5.1).
pub fn program_to_run(mode: &Mode) -> Result<Program> {
    let program = mode.program(Roots::Program);
    if let (Mode::Project { .. }, Some(graph)) = (mode, &program.graph) {
        let root = graph.root();
        if !program.has_errors() && root.has_lib && !root.has_main {
            bail!(
                "`{}` is a library: it has no src/main.nova; `nova check` and `nova test` \
                 work on it",
                root.name
            );
        }
    }
    Ok(program)
}

/// The project's package name, which `nova build` names its output after.
pub fn package_name(program: &Program) -> String {
    program
        .graph
        .as_ref()
        .map_or_else(|| "out".to_string(), |graph| graph.root().name.clone())
}
```

In `crates/nova-cli/src/cmd/run.rs`:
- `use nova_driver::Outcome;` becomes `use nova_driver::{Outcome, Program, Roots};`;
- add this helper after the `BuildCmd` struct:

  ```rust
  /// The program's entry, which `nova run` passes as its first argument and
  /// `nova check` names.
  fn entry(program: &Program) -> PathBuf {
      program
          .roots
          .first()
          .map(|root| root.path.clone())
          .unwrap_or_default()
  }
  ```
- `run` becomes:

  ```rust
  pub fn run(cmd: RunCmd) -> Result<()> {
      let mode = project::mode(cmd.file)?;
      let program = project::program_to_run(&mode)?;
      let mut args = vec![entry(&program).to_string_lossy().into_owned()];
      args.extend(cmd.args.iter().map(|a| a.to_string_lossy().into_owned()));
      match nova_driver::run_program(program, args)? {
          Outcome::Ok(()) => Ok(()),
          Outcome::Failed { errors } => anyhow::bail!(
              "could not compile due to {errors} previous error{}",
              if errors == 1 { "" } else { "s" }
          ),
      }
  }
  ```
- in `build`:
  - after `let mode = …;`, add `let program = project::program_to_run(&mode)?;`;
  - the project arm of the `output` match becomes:

    ```rust
            (None, Mode::Project { target_dir, .. }) => {
                let dir = target_dir.join(if cmd.release { "release" } else { "debug" });
                if !program.has_errors() {
                    std::fs::create_dir_all(&dir)
                        .with_context(|| format!("creating {}", dir.display()))?;
                }
                dir.join(format!(
                    "{}{}",
                    project::package_name(&program),
                    std::env::consts::EXE_SUFFIX
                ))
            }
    ```
  - `let file = mode.entry();` goes, and the two driver calls become
    `nova_driver::build_program_release(program, &output)?` and
    `nova_driver::build_program(program, &output)?`;
- `check` becomes:

  ```rust
  pub fn check(cmd: CheckCmd) -> Result<()> {
      let mode = project::mode(cmd.file)?;
      let program = mode.program(Roots::Check);
      let entry = entry(&program);
      match nova_driver::check_program(program)? {
          Outcome::Ok(()) => {
              println!("ok: {}", entry.display());
              Ok(())
          }
          Outcome::Failed { errors } => {
              anyhow::bail!("found {errors} error{}", if errors == 1 { "" } else { "s" })
          }
      }
  }
  ```

In `crates/nova-cli/src/cmd/test.rs`, in `run`, replace
`let (exe, tests) = nova_driver::build_test_binary(mode.entry())?;` with:

```rust
    let program = mode.program(nova_driver::Roots::Test);
    let (exe, tests) = nova_driver::build_test_program(program)?;
```

- [ ] **Step 4: Run the tests to verify they pass**

```bash
cd /d/Projects/nona/nova && P=/c/Users/SAKEER~1/AppData/Local/Temp/gcm/p33 && netstat -ano | grep -E "[:.]3000 .*LISTENING"; cargo test --locked -p nova-cli > $P/t5.txt 2>&1; echo "exit=$?"; python -X utf8 $P/count.py $P/t5.txt; grep -E "FAILED|panicked|^warning|^error" $P/t5.txt | head
```

Expected: no `LISTENING` line, then `exit=0`, 0 failed and no warnings.

If a test outside the two files named here fails, it pinned the old CLI's
own manifest reading (its "could not read nova.toml" message, say). Read
it: if it pins a message the spec replaces, update it and ledger a ruling.
Otherwise the code is wrong.

- [ ] **Step 5: Commit**

Write `$P/msg-5.txt`:

```
nova-cli: commands work on packages

Each command builds a `Program` (spec
docs/superpowers/specs/2026-10-08-phase-3-3a-local-packages-design.md
§5.1, §5.2):
- `nova run` and `nova build` take src/main.nova. A library without one
  is refused: "`geom` is a library: it has no src/main.nova; `nova
  check` and `nova test` work on it".
- `nova check` checks the program and the library in one analysis.
- `nova test` takes those and every tests/*.nova.

The CLI no longer reads the manifest itself: the driver renders the
graph's diagnostics, so warnings still print and errors still stop the
command. Its M0005 goes, now that the graph raises it. A package with
neither target is M0013, which the pinned test now expects.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
```

```bash
cd /d/Projects/nona/nova && P=/c/Users/SAKEER~1/AppData/Local/Temp/gcm/p33 && cargo fmt --all && git add crates/nova-cli && git commit -q -F $P/msg-5.txt && git log -1 --format=%s
```

Expected: `nova-cli: commands work on packages`.

The task's test command:
`cargo test --locked -p nova-cli --test packages --test project`.

---

### Task 6: Tests and `main` across packages

Spec §4.6 and §5.2: a dependency's tests are stripped, a `tests/` file's
tests are named after it, and a program's `main` is its entry's.

**Files:**
- Modify: `crates/nova-driver/src/lib.rs`:
  - `strip_test_functions` keeps the root's tests;
  - `name_tests` and `keep_the_entry_main`, used in `FrontendContext::check`.
- Modify: `crates/nova-driver/src/analyze.rs` (both, in `analyze_program`)
- Modify: `crates/nova-driver/tests/packages.rs`,
  `crates/nova-cli/tests/packages.rs` (new tests)

**Interfaces:**
- Consumes:
  - Task 4's `Loaded { package, test_file, path, .. }` and
    `strip_test_functions(&mut [Loaded])`;
  - Task 5's `crates/nova-cli/tests/packages.rs` helpers.
- Produces (in the crate only):
  - `strip_test_functions(&mut [Loaded], keep_root_tests: bool)`;
  - `keep_the_entry_main(&mut hir::Module, &Definitions)`;
  - `name_tests(&mut [TestFn], &Definitions, &[Loaded])`.

- [ ] **Step 1: Write the failing tests**

Append to `crates/nova-driver/tests/packages.rs`:

```rust

#[test]
fn an_entry_without_main_is_e0601_even_when_a_dependency_has_one() {
    let dir = fresh("no-main");
    write(
        &dir.join("geom"),
        &[
            ("nova.toml", manifest("geom", "").as_str()),
            (
                "src/lib.nova",
                "pub fn area() -> Int {\n    9\n}\n\nfn main() {\n    println(\"geom's demo\")\n}\n",
            ),
        ],
    );
    let app = dir.join("app");
    write(
        &app,
        &[
            ("nova.toml", manifest("app", GEOM_DEPENDENCY).as_str()),
            ("src/main.nova", "import geom\n\nfn run() {\n    println(\"${area()}\")\n}\n"),
        ],
    );
    let a = check(Program::for_package(&app, Roots::Program));
    assert_eq!(codes(&a), ["E0601"], "{}", messages(&a));
}

#[test]
fn a_dependencys_tests_are_stripped_and_the_roots_kept() {
    let dir = fresh("strip");
    write(
        &dir.join("geom"),
        &[
            ("nova.toml", manifest("geom", "").as_str()),
            (
                "src/lib.nova",
                "pub fn area() -> Int {\n    9\n}\n\n@test\nfn geom_checks_its_area() {\n    assert_eq(area(), 9)\n}\n",
            ),
        ],
    );
    let app = dir.join("app");
    write(
        &app,
        &[
            ("nova.toml", manifest("app", GEOM_DEPENDENCY).as_str()),
            (
                "src/main.nova",
                "import geom\n\nfn main() {\n    println(\"${area()}\")\n}\n\n\
                 @test\nfn app_checks_the_area() {\n    assert_eq(area(), 9)\n}\n",
            ),
        ],
    );
    let program = Program::for_package(&app, Roots::Test);
    let a = analyze_program(program, &DiskSources, &options(true)).unwrap();
    assert!(a.diagnostics.is_empty(), "{}", messages(&a));
    let defs = a.definitions.as_ref().unwrap();
    let defined = |name: &str| defs.defs().iter().any(|d| d.name == name);
    assert!(defined("app_checks_the_area"));
    assert!(!defined("geom_checks_its_area"));
}
```

Append to `crates/nova-cli/tests/packages.rs`:

```rust

#[test]
fn tests_are_named_by_file_and_run_in_sorted_order() {
    let dir = fresh("test-names");
    write(
        &dir,
        &[
            ("nova.toml", manifest("shapes", "").as_str()),
            (
                "src/lib.nova",
                "pub fn area() -> Int {\n    9\n}\n\n@test\nfn in_the_library() {\n    assert_eq(area(), 9)\n}\n",
            ),
            ("tests/b.nova", "import shapes\n\n@test\nfn second() {\n    assert_eq(area(), 9)\n}\n"),
            ("tests/a.nova", "import shapes\n\n@test\nfn first() {\n    assert_eq(area(), 9)\n}\n"),
        ],
    );
    let out = nova().current_dir(&dir).arg("test").assert().success();
    let printed = stdout(&out);
    let ran: Vec<&str> = printed
        .lines()
        .filter(|l| l.starts_with("test ") && l.ends_with(" ... ok"))
        .collect();
    assert_eq!(
        ran,
        [
            "test in_the_library ... ok",
            "test a::first ... ok",
            "test b::second ... ok"
        ],
        "{printed}"
    );
    // The filter is a substring of the full name (spec §5.2).
    let out = nova()
        .current_dir(&dir)
        .args(["test", "a::"])
        .assert()
        .success();
    let printed = stdout(&out);
    assert!(
        printed.contains("running 1 test\n") && printed.contains("test a::first ... ok"),
        "{printed}"
    );
}

#[test]
fn a_dependencys_tests_do_not_run_in_its_dependent() {
    let dir = fresh("dependency-tests");
    write(
        &dir.join("geom"),
        &[
            ("nova.toml", manifest("geom", "").as_str()),
            (
                "src/lib.nova",
                "pub fn area() -> Int {\n    9\n}\n\n@test\nfn geom_checks_its_area() {\n    assert_eq(area(), 9)\n}\n",
            ),
        ],
    );
    let app = dir.join("app");
    write(
        &app,
        &[
            ("nova.toml", manifest("app", GEOM_DEPENDENCY).as_str()),
            (
                "src/main.nova",
                "import geom\n\nfn main() {\n    println(\"${area()}\")\n}\n\n\
                 @test\nfn app_checks_the_area() {\n    assert_eq(area(), 9)\n}\n",
            ),
        ],
    );
    let out = nova().current_dir(&app).arg("test").assert().success();
    let printed = stdout(&out);
    assert!(printed.contains("running 1 test\n"), "{printed}");
    assert!(!printed.contains("geom_checks_its_area"), "{printed}");
}

#[test]
fn the_entry_main_runs_when_a_dependency_also_has_one() {
    // Review Focus 4, a guard: the type checker emits the entry module's
    // functions first, so its `main` wins today, and the rename keeps it so.
    let dir = fresh("two-mains");
    write(
        &dir.join("geom"),
        &[
            ("nova.toml", manifest("geom", "").as_str()),
            (
                "src/lib.nova",
                "pub fn area() -> Int {\n    9\n}\n\nfn main() {\n    println(\"geom's demo\")\n}\n",
            ),
        ],
    );
    let app = dir.join("app");
    write(
        &app,
        &[
            ("nova.toml", manifest("app", GEOM_DEPENDENCY).as_str()),
            ("src/main.nova", "import geom\n\nfn main() {\n    println(\"app ${area()}\")\n}\n"),
        ],
    );
    nova()
        .current_dir(&app)
        .arg("run")
        .assert()
        .success()
        .stdout("app 9\n");
    nova().current_dir(&app).arg("build").assert().success();
    let exe = app
        .join("target")
        .join("debug")
        .join(format!("app{}", std::env::consts::EXE_SUFFIX));
    let out = std::process::Command::new(&exe).output().expect("run the build");
    assert_eq!(String::from_utf8_lossy(&out.stdout), "app 9\n");
}
```

- [ ] **Step 2: Run them to verify they fail**

```bash
cd /d/Projects/nona/nova && P=/c/Users/SAKEER~1/AppData/Local/Temp/gcm/p33 && cargo test --locked -p nova-driver --test packages > $P/t6-red-driver.txt 2>&1; cargo test --locked -p nova-cli --test packages > $P/t6-red-cli.txt 2>&1; grep -hE "^test .*(FAILED|ok)$" $P/t6-red-driver.txt $P/t6-red-cli.txt | grep -E "e0601|stripped|named_by_file|do_not_run|entry_main"
```

Expected:
- FAILED: `an_entry_without_main_is_e0601_even_when_a_dependency_has_one`
  (no E0601: geom's `main` is taken);
- FAILED: `a_dependencys_tests_are_stripped_and_the_roots_kept`;
- FAILED: `tests_are_named_by_file_and_run_in_sorted_order` (`test first
  ... ok`);
- FAILED: `a_dependencys_tests_do_not_run_in_its_dependent` (`running 2
  tests`);
- ok: `the_entry_main_runs_when_a_dependency_also_has_one` (a guard).

- [ ] **Step 3: Implement**

In `crates/nova-driver/src/lib.rs`:
- the resolver import becomes
  `use nova_resolver::{Builtin, Def, DefId, DefKind, Definitions, ModuleId, ModuleSource, TestFn};`,
  and add `use nova_pm::PackageId;` after it;
- after `const SHADOWED_USER_MAIN_NAME`, add:

  ```rust
  /// What every function called `main` outside the entry module is renamed
  /// to (spec
  /// `docs/superpowers/specs/2026-10-08-phase-3-3a-local-packages-design.md`
  /// §4.6): a name the lexer cannot produce, like
  /// [`SHADOWED_USER_MAIN_NAME`].
  const NOT_THE_ENTRY_MAIN: &str = "main.not_the_entry";

  /// Rename every `main` declared outside the entry module, `ModuleId(0)`,
  /// so that `nova_mir::lower_module`'s search by name finds the entry's own
  /// `main`, or reports E0601 (spec 3.3a §4.6). A call reaches its function
  /// by `DefId`, so the rename breaks no call.
  fn keep_the_entry_main(module: &mut hir::Module, definitions: &Definitions) {
      for function in &mut module.functions {
          if function.name != "main" {
              continue;
          }
          let def = definitions.defs().get(function.def_id.0 as usize);
          if let Some(Def {
              kind: DefKind::Fn { item_index },
              ..
          }) = def
          {
              if definitions.module_of(*item_index) != ModuleId(0) {
                  function.name = NOT_THE_ENTRY_MAIN.to_string();
              }
          }
      }
  }

  /// Name each test of a `tests/` file `<file stem>::<function>` (spec 3.3a
  /// §4.6). The synthesized `main` prints these names as its inventory, so
  /// `nova test` and the binary agree on them.
  fn name_tests(tests: &mut [TestFn], definitions: &Definitions, modules: &[Loaded]) {
      for test in tests {
          let def = definitions.defs().get(test.def_id.0 as usize);
          let Some(Def {
              kind: DefKind::Fn { item_index },
              ..
          }) = def
          else {
              continue;
          };
          let module = definitions.module_of(*item_index).0 as usize;
          if let Some(module) = modules.get(module).filter(|m| m.test_file) {
              test.name = format!(
                  "{}::{}",
                  FrontendContext::module_name(&module.path),
                  test.name
              );
          }
      }
  }
  ```
- in `FrontendContext::check`:
  - `if !with_test_module { strip_test_functions(&mut modules); }`, with the
    long comment above it, becomes
    `strip_test_functions(&mut modules, with_test_module);`. The comment
    stays, and gains a last paragraph:

    ```rust
            // Phase 3.3a: under `nova test` the root package's tests are
            // kept and a dependency's stripped (spec
            // docs/superpowers/specs/2026-10-08-phase-3-3a-local-packages-design.md
            // §4.6).
    ```
  - `let tests = resolved.tests;` becomes:

    ```rust
            let mut tests = resolved.tests;
            name_tests(&mut tests, &resolved.definitions, &modules);
    ```
  - its last line, `Ok(Some((checked.module, tests, fresh_def_id)))`,
    becomes:

    ```rust
            let mut module = checked.module;
            keep_the_entry_main(&mut module, &resolved.definitions);
            Ok(Some((module, tests, fresh_def_id)))
    ```
- `strip_test_functions`:
  - its doc comment's first line becomes these lines, and the rest stays:

    ```rust
    /// Remove top-level `@test` functions from `modules`, in place: from every
    /// module, or with `keep_root_tests` only from a dependency's (spec 3.3a
    /// §4.6). `nova test` and the language server keep the root package's
    /// tests, and a loose program's; a dependency's tests never run in its
    /// dependent's binary.
    ///
    /// What follows was written when this ran only for `nova run`, `build`
    /// and `check`, and every module was the program's own.
    ```
  - its signature and loop become:

    ```rust
    fn strip_test_functions(modules: &mut [Loaded], keep_root_tests: bool) {
        for module in modules.iter_mut() {
            let root = matches!(module.package, None | Some(PackageId(0)));
            if keep_root_tests && root {
                continue;
            }
            module.ast.items.retain(|item| {
                !matches!(
                    &item.value,
                    nova_ast::Item::Function(f) if f.attrs.iter().any(|a| a.name.value == "test")
                )
            });
        }
    }
    ```

In `crates/nova-driver/src/analyze.rs`, in `analyze_program`:
- `if !options.tests { crate::strip_test_functions(&mut modules); }` becomes
  `crate::strip_test_functions(&mut modules, options.tests);`;
- the MIR block, and the line that stores the module, become:

  ```rust
      let module_only = options.module_only || !runs;
      let mut module = checked.module;
      if !module_only && !has_error(&diagnostics) {
          crate::keep_the_entry_main(&mut module, &resolved.definitions);
          if let Err(mir) = nova_mir::lower_module(&module) {
              diagnostics.extend(mir);
          }
      }
  ```

  and `analysis.module = Some(checked.module);` becomes
  `analysis.module = Some(module);`.

- [ ] **Step 4: Run the tests to verify they pass**

```bash
cd /d/Projects/nona/nova && P=/c/Users/SAKEER~1/AppData/Local/Temp/gcm/p33 && netstat -ano | grep -E "[:.]3000 .*LISTENING"; cargo test --locked -p nova-driver -p nova-cli > $P/t6.txt 2>&1; echo "exit=$?"; python -X utf8 $P/count.py $P/t6.txt; grep -E "FAILED|panicked|^warning|^error" $P/t6.txt | head
```

Expected: no `LISTENING` line, then `exit=0`, 0 failed and no warnings.
Every older `nova test` test passes unchanged: a loose program's tests are
the root's, and a `src/` test keeps its bare name.

- [ ] **Step 5: Commit**

Write `$P/msg-6.txt`:

```
nova-driver: a dependency's tests, test names, and the entry's main

Spec docs/superpowers/specs/2026-10-08-phase-3-3a-local-packages-design.md
§4.6:
- **Tests.** Under `nova test`, only the root package's `@test` functions
  are kept; a dependency's are stripped, so they never run in a
  dependent's binary.
- **Names.** A test in tests/<file>.nova is named `<file>::<function>`,
  so `nova test api::` selects one file's tests. A test in src/ keeps
  its bare name.
- **main.** Every function called `main` outside the entry module is
  renamed before MIR, so a program's `main` is its entry's. An entry
  without one is E0601 even when a dependency declares a `main`.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
```

```bash
cd /d/Projects/nona/nova && P=/c/Users/SAKEER~1/AppData/Local/Temp/gcm/p33 && cargo fmt --all && git add crates/nova-driver crates/nova-cli && git commit -q -F $P/msg-6.txt && git log -1 --format=%s
```

Expected: `nova-driver: a dependency's tests, test names, and the entry's main`.

The task's test command:
`cargo test --locked -p nova-driver --test packages && cargo test --locked -p nova-cli --test packages`.

---

### Task 7: `nova add --path` and `nova remove`

Spec §5.3.

**Files:**
- Create: `crates/nova-cli/src/cmd/deps.rs`
- Modify: `crates/nova-cli/src/cmd/mod.rs`, `crates/nova-cli/src/main.rs`
  (two commands)
- Modify: `crates/nova-cli/Cargo.toml` (`toml_edit`)
- Create: `crates/nova-cli/tests/deps.rs`

**Interfaces:**
- Consumes:
  - Task 1's `nova_pm::real_path`;
  - Task 2's `nova_pm::graph_from`;
  - `nova_pm::{parse, check_name, find_root, MANIFEST}`.
- Produces: `cmd::deps::{AddCmd, RemoveCmd, add, remove}`.

- [ ] **Step 1: Write the failing tests**

`crates/nova-cli/tests/deps.rs`:

```rust
//! `nova add --path` and `nova remove` (spec
//! `docs/superpowers/specs/2026-10-08-phase-3-3a-local-packages-design.md`
//! §5.3, §7.3).

use std::path::{Path, PathBuf};

use assert_cmd::Command;

fn nova() -> Command {
    Command::cargo_bin("nova").expect("nova binary builds")
}

/// A fresh, empty directory with a fixed name, so each run replaces the last.
fn fresh(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("nova-deps-{name}"));
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

fn stderr(assert: &assert_cmd::assert::Assert) -> String {
    String::from_utf8_lossy(&assert.get_output().stderr).into_owned()
}

/// A library called `name` at `dir/<name>`, whose `version()` says `name`.
fn library(dir: &Path, name: &str) {
    write(
        &dir.join(name),
        &[
            (
                "nova.toml",
                format!("[package]\nname = \"{name}\"\nversion = \"0.1.0\"\nedition = \"2026\"\n")
                    .as_str(),
            ),
            (
                "src/lib.nova",
                format!("pub fn version() -> String {{\n    \"{name}\"\n}}\n").as_str(),
            ),
        ],
    );
}

const APP_MANIFEST: &str = "[package]\nname = \"app\" # the app\nversion = \"0.1.0\"\nedition = \"2026\"\n\n# Libraries.\n[dependencies]\n";

/// `dir/app`, with `APP_MANIFEST` and a `main` printing `version()`.
fn app(dir: &Path, import: &str) -> PathBuf {
    let app = dir.join("app");
    write(
        &app,
        &[
            ("nova.toml", APP_MANIFEST),
            (
                "src/main.nova",
                format!("import {import}\n\nfn main() {{\n    println(version())\n}}\n").as_str(),
            ),
        ],
    );
    app
}

#[test]
fn add_writes_a_path_entry_and_keeps_the_rest_of_the_manifest() {
    let dir = fresh("add");
    library(&dir, "geom");
    library(&dir, "helper");
    let app = app(&dir, "geom");
    nova()
        .current_dir(&app)
        .args(["add", "geom", "--path", "../geom"])
        .assert()
        .success();
    assert_eq!(
        read(&app.join("nova.toml")),
        format!("{APP_MANIFEST}geom = {{ path = \"../geom\" }}\n")
    );
    nova()
        .current_dir(&app)
        .arg("run")
        .assert()
        .success()
        .stdout("geom\n");
    nova()
        .current_dir(&app)
        .args(["add", "helper", "--path", "../helper", "--dev"])
        .assert()
        .success();
    let text = read(&app.join("nova.toml"));
    assert!(
        text.starts_with(&format!("{APP_MANIFEST}geom = {{ path = \"../geom\" }}\n")),
        "{text}"
    );
    assert!(
        text.contains("[dev-dependencies]\nhelper = { path = \"../helper\" }\n"),
        "{text}"
    );
}

#[test]
fn add_edits_an_inline_dependency_table() {
    let dir = fresh("add-inline");
    library(&dir, "geom");
    let app = app(&dir, "geom");
    std::fs::write(
        app.join("nova.toml"),
        "dependencies = {}\n\n[package]\nname = \"app\"\nversion = \"0.1.0\"\nedition = \"2026\"\n",
    )
    .unwrap();
    nova()
        .current_dir(&app)
        .args(["add", "geom", "--path", "../geom"])
        .assert()
        .success();
    let text = read(&app.join("nova.toml"));
    assert!(text.starts_with("dependencies = {"), "{text}");
    nova()
        .current_dir(&app)
        .arg("run")
        .assert()
        .success()
        .stdout("geom\n");
}

#[test]
fn remove_takes_the_entry_and_its_comment_lines() {
    let dir = fresh("remove");
    let before = "[package]\nname = \"app\"\nversion = \"0.1.0\"\nedition = \"2026\"\n\n\
                  [dependencies]\n# Shapes.\ngeom = { path = \"../geom\" }\n\n\
                  [dev-dependencies]\n# Kept.\nhelper = { path = \"../helper\" }\n";
    write(&dir, &[("nova.toml", before), ("src/main.nova", "fn main() {}\n")]);
    nova()
        .current_dir(&dir)
        .args(["remove", "geom"])
        .assert()
        .success();
    assert_eq!(
        read(&dir.join("nova.toml")),
        before.replace("# Shapes.\ngeom = { path = \"../geom\" }\n", "")
    );
    let out = nova()
        .current_dir(&dir)
        .args(["remove", "geom"])
        .assert()
        .failure();
    assert!(
        stderr(&out).contains("`geom` is not in [dependencies]"),
        "{}",
        stderr(&out)
    );
    nova()
        .current_dir(&dir)
        .args(["remove", "helper", "--dev"])
        .assert()
        .success();
    assert!(!read(&dir.join("nova.toml")).contains("helper ="));
}

#[test]
fn add_refuses_without_writing() {
    let dir = fresh("refuses");
    library(&dir, "geom");
    // `geom2` depends on `app`, so adding it closes a cycle.
    write(
        &dir.join("geom2"),
        &[
            (
                "nova.toml",
                "[package]\nname = \"geom2\"\nversion = \"0.1.0\"\nedition = \"2026\"\n\n\
                 [dependencies]\napp = { path = \"../app\" }\n",
            ),
            ("src/lib.nova", ""),
        ],
    );
    let app = app(&dir, "geom");
    write(&app, &[("src/lib.nova", "")]);
    let manifest = app.join("nova.toml");
    // The package's own directory is M0010 before its name is compared; a
    // key equal to the package's own name would be M0012 first.
    let cases: [(&[&str], &str); 4] = [
        (&["add", "me", "--path", "."], "M0010"),
        (&["add", "geom2", "--path", "../geom2"], "M0010"),
        (&["add", "geom", "--path", "../nowhere"], "M0007"),
        (&["add", "geom"], "registry dependencies arrive with the package index"),
    ];
    for (args, expected) in cases {
        let out = nova().current_dir(&app).args(args).assert().failure();
        assert!(stderr(&out).contains(expected), "{args:?}: {}", stderr(&out));
        assert_eq!(read(&manifest), APP_MANIFEST, "{args:?} wrote");
    }
    nova()
        .current_dir(&app)
        .args(["add", "geom", "--path", "../geom"])
        .assert()
        .success();
    let added = read(&manifest);
    let again: [&[&str]; 2] = [
        &["add", "geom", "--path", "../geom"],
        &["add", "geom", "--path", "../geom", "--dev"],
    ];
    for args in again {
        let out = nova().current_dir(&app).args(args).assert().failure();
        assert!(
            stderr(&out).contains("`geom` is already in [dependencies]"),
            "{args:?}: {}",
            stderr(&out)
        );
        assert_eq!(read(&manifest), added);
    }
    // A manifest with errors is refused, its errors shown.
    std::fs::write(&manifest, APP_MANIFEST.replace("2026", "2021")).unwrap();
    let out = nova()
        .current_dir(&app)
        .args(["add", "geom", "--path", "../geom"])
        .assert()
        .failure();
    assert!(stderr(&out).contains("M0003"), "{}", stderr(&out));
    assert_eq!(read(&manifest), APP_MANIFEST.replace("2026", "2021"));
}

#[test]
fn add_works_in_a_package_with_no_source_yet() {
    // Spec §3.1: M0013 does not stop `nova add`, which compiles nothing.
    let dir = fresh("no-source");
    library(&dir, "geom");
    let app = dir.join("app");
    write(&app, &[("nova.toml", APP_MANIFEST)]);
    nova()
        .current_dir(&app)
        .args(["add", "geom", "--path", "../geom"])
        .assert()
        .success();
    assert_eq!(
        read(&app.join("nova.toml")),
        format!("{APP_MANIFEST}geom = {{ path = \"../geom\" }}\n")
    );
}

#[test]
fn add_from_a_subdirectory_writes_a_path_relative_to_the_manifest() {
    // Review Focus 5.
    let dir = fresh("subdirectory");
    library(&dir, "geom");
    let app = app(&dir, "geom");
    nova()
        .current_dir(app.join("src"))
        .args(["add", "geom", "--path", "../../geom"])
        .assert()
        .success();
    assert_eq!(
        read(&app.join("nova.toml")),
        format!("{APP_MANIFEST}geom = {{ path = \"../geom\" }}\n")
    );
}

#[test]
fn a_hyphenated_library_is_added_and_imported_with_an_underscore() {
    // Review Focus 3.
    let dir = fresh("hyphen");
    library(&dir, "json-api");
    let app = app(&dir, "json_api");
    nova()
        .current_dir(&app)
        .args(["add", "json-api", "--path", "../json-api"])
        .assert()
        .success();
    nova()
        .current_dir(&app)
        .arg("run")
        .assert()
        .success()
        .stdout("json-api\n");
}
```

- [ ] **Step 2: Run them to verify they fail**

```bash
cd /d/Projects/nona/nova && P=/c/Users/SAKEER~1/AppData/Local/Temp/gcm/p33 && cargo test --locked -p nova-cli --test deps > $P/t7-red.txt 2>&1; grep -E "^test .*(FAILED|ok)$" $P/t7-red.txt; grep -m1 "unrecognized subcommand" $P/t7-red.txt
```

Expected: 7 FAILED, and the line `error: unrecognized subcommand 'add'` (or
`'remove'`).

- [ ] **Step 3: Implement**

`crates/nova-cli/src/cmd/deps.rs`:

```rust
//! `nova add <name> --path <dir> [--dev]` and `nova remove <name> [--dev]`:
//! edit the project's `nova.toml` (spec
//! `docs/superpowers/specs/2026-10-08-phase-3-3a-local-packages-design.md`
//! §5.3), keeping its comments, order and layout through `toml_edit`.

use std::path::{Component, Path, PathBuf};

use anyhow::{anyhow, bail, Context, Result};
use clap::Args;
use nova_diagnostics::{render, FileDb, Severity};
use toml_edit::{DocumentMut, InlineTable, Item, Value};

#[derive(Args)]
pub struct AddCmd {
    /// The dependency's package name.
    name: String,
    /// The directory holding its nova.toml, from the current directory.
    /// Registry dependencies arrive with the package index.
    #[arg(long)]
    path: Option<PathBuf>,
    /// Add it to [dev-dependencies], which only tests/ files import.
    #[arg(long)]
    dev: bool,
}

#[derive(Args)]
pub struct RemoveCmd {
    /// The dependency's package name.
    name: String,
    /// Remove it from [dev-dependencies].
    #[arg(long)]
    dev: bool,
}

fn table_name(dev: bool) -> &'static str {
    if dev {
        "dev-dependencies"
    } else {
        "dependencies"
    }
}

/// The project's directory, and its `nova.toml`'s text.
fn manifest() -> Result<(PathBuf, String)> {
    let cwd = std::env::current_dir().context("reading the current directory")?;
    let root = nova_pm::find_root(&cwd).context("no nova.toml here or in any directory above")?;
    let path = root.join(nova_pm::MANIFEST);
    let text =
        std::fs::read_to_string(&path).with_context(|| format!("reading {}", path.display()))?;
    Ok((root, text))
}

/// `text`, parsed for editing.
fn document(text: &str) -> Result<DocumentMut> {
    text.parse::<DocumentMut>()
        .map_err(|error| anyhow!("nova.toml is not valid TOML: {error}"))
}

/// `document` as text, in `original`'s line endings.
fn to_text(document: &DocumentMut, original: &str) -> String {
    let text = document.to_string();
    if original.contains("\r\n") {
        text.replace("\r\n", "\n").replace('\n', "\r\n")
    } else {
        text
    }
}

/// Write `text` as the project's `nova.toml`.
fn write(root: &Path, text: &str) -> Result<()> {
    let path = root.join(nova_pm::MANIFEST);
    std::fs::write(&path, text).with_context(|| format!("writing {}", path.display()))
}

pub fn add(cmd: AddCmd) -> Result<()> {
    nova_pm::check_name(&cmd.name).map_err(|message| anyhow!(message))?;
    let Some(path) = &cmd.path else {
        bail!(
            "registry dependencies arrive with the package index; use `--path <dir>` for a \
             local package"
        );
    };
    let (root, text) = manifest()?;
    // A manifest with errors is refused, with its errors shown.
    let mut db = FileDb::new();
    let file = db.add(root.join(nova_pm::MANIFEST).display().to_string(), text.as_str());
    let (parsed, diagnostics) = nova_pm::parse(&text, file);
    let Some(parsed) = parsed.filter(|_| !diagnostics.iter().any(|d| d.severity == Severity::Error))
    else {
        render::emit_all(&db, &diagnostics);
        bail!("nova.toml has errors, so it was left unchanged");
    };
    for (table, entries) in [
        ("dependencies", &parsed.dependencies),
        ("dev-dependencies", &parsed.dev_dependencies),
    ] {
        if entries.iter().any(|entry| entry.name == cmd.name) {
            bail!("`{}` is already in [{table}]", cmd.name);
        }
    }

    // Relative to the manifest's directory, whatever the current one.
    let cwd = std::env::current_dir().context("reading the current directory")?;
    let written = relative(&nova_pm::real_path(&root), &nova_pm::real_path(&cwd.join(path)));
    let mut document = document(&text)?;
    let table = table_name(cmd.dev);
    if document.get(table).is_none() {
        document.insert(table, toml_edit::table());
    }
    let entries = document
        .get_mut(table)
        .and_then(Item::as_table_like_mut)
        .with_context(|| format!("[{table}] in nova.toml is not a table"))?;
    let mut entry = InlineTable::new();
    entry.insert("path", Value::from(written.as_str()));
    entries.insert(&cmd.name, Item::Value(Value::InlineTable(entry)));
    let new_text = to_text(&document, &text);

    // The graph as it would be: any error refuses, and nothing is written.
    // M0013, a package with no source yet, is not the entry's fault (spec
    // §3.1).
    let mut db = FileDb::new();
    let (_, diagnostics) = nova_pm::graph_from(&root, Some(&new_text), &mut db);
    if diagnostics
        .iter()
        .any(|d| d.severity == Severity::Error && d.code != "M0013")
    {
        render::emit_all(&db, &diagnostics);
        bail!("`{}` was not added; nova.toml was left unchanged", cmd.name);
    }
    write(&root, &new_text)?;
    println!("added {} = {{ path = \"{written}\" }} to [{table}]", cmd.name);
    Ok(())
}

pub fn remove(cmd: RemoveCmd) -> Result<()> {
    let (root, text) = manifest()?;
    let mut document = document(&text)?;
    let table = table_name(cmd.dev);
    // The comment lines directly above an entry are its decor, and go with
    // it.
    let removed = document
        .get_mut(table)
        .and_then(Item::as_table_like_mut)
        .and_then(|entries| entries.remove(&cmd.name));
    if removed.is_none() {
        bail!("`{}` is not in [{table}]", cmd.name);
    }
    write(&root, &to_text(&document, &text))?;
    println!("removed {} from [{table}]", cmd.name);
    Ok(())
}

/// `target` relative to `base`, both canonical, with `/` separators. When
/// they share no root, as on two Windows drives, `target` itself, with `/`
/// separators (spec §5.3).
fn relative(base: &Path, target: &Path) -> String {
    let from: Vec<Component> = base.components().collect();
    let to: Vec<Component> = target.components().collect();
    if from.first() != to.first() {
        return target.to_string_lossy().replace('\\', "/");
    }
    let common = from.iter().zip(&to).take_while(|(a, b)| a == b).count();
    let mut parts = vec!["..".to_string(); from.len() - common];
    parts.extend(
        to[common..]
            .iter()
            .map(|part| part.as_os_str().to_string_lossy().into_owned()),
    );
    if parts.is_empty() {
        ".".to_string()
    } else {
        parts.join("/")
    }
}
```

In `crates/nova-cli/src/cmd/mod.rs`, add `pub mod deps;` before
`pub mod fmt;`.

In `crates/nova-cli/src/main.rs`:
- add to `enum Command`, after `Init(cmd::new::InitCmd),`:

  ```rust
      /// Add a path dependency to nova.toml.
      Add(cmd::deps::AddCmd),
      /// Remove a dependency from nova.toml.
      Remove(cmd::deps::RemoveCmd),
  ```
- add to the dispatch, after `Command::Init(cmd) => cmd::new::init(cmd),`:

  ```rust
          Command::Add(cmd) => cmd::deps::add(cmd),
          Command::Remove(cmd) => cmd::deps::remove(cmd),
  ```

In `crates/nova-cli/Cargo.toml`, add after `nova-lsp = …`:

```toml
# `nova add` and `nova remove` edit nova.toml, keeping its comments and
# layout (spec docs/superpowers/specs/2026-10-08-phase-3-3a-local-packages-design.md §5.3).
toml_edit = { workspace = true }
```

- [ ] **Step 4: Run the tests to verify they pass**

```bash
cd /d/Projects/nona/nova && P=/c/Users/SAKEER~1/AppData/Local/Temp/gcm/p33 && cargo test --locked -p nova-cli --test deps > $P/t7.txt 2>&1; echo "exit=$?"; python -X utf8 $P/count.py $P/t7.txt; grep -E "FAILED|panicked|^warning|^error" $P/t7.txt | head; git diff --stat -- Cargo.lock
```

Expected:
- `exit=0`, 7 passed, 0 failed, no warnings;
- `Cargo.lock` changes by one line, `toml_edit` in `nova-cli`'s entry.

The two exact manifests assume `toml_edit` writes a new entry as
`geom = { path = "../geom" }`. If it spaces it otherwise, and the file is
still the old text plus one entry line, correct the expected text and
ledger it.

- [ ] **Step 5: Commit**

Write `$P/msg-7.txt`:

```
nova-cli: nova add --path and nova remove

Spec docs/superpowers/specs/2026-10-08-phase-3-3a-local-packages-design.md
§5.3:
- `nova add <name> --path <dir> [--dev]` writes `<name> = { path =
  "<dir>" }` through toml_edit, so the manifest keeps its comments,
  order and layout.
  - The table may be a table or an inline table, or be missing.
  - The path is relative to the manifest's directory, whatever the
    current one, or absolute across Windows drives.
  - It refuses a manifest with errors and an entry already in either
    table.
  - It checks the graph as it would be before writing, so a cycle, a
    self-dependency or a bad path writes nothing. A package with no
    source yet (M0013) can still add one.
  - Without `--path`, it says registry dependencies arrive with the
    package index.
- `nova remove <name> [--dev]` deletes the entry and the comment lines
  directly above it, and fails if it is not there.

nova-cli gains toml_edit, already in the lockfile.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
```

```bash
cd /d/Projects/nona/nova && P=/c/Users/SAKEER~1/AppData/Local/Temp/gcm/p33 && cargo fmt --all && git add crates/nova-cli Cargo.lock && git commit -q -F $P/msg-7.txt && git log -1 --format=%s
```

Expected: `nova-cli: nova add --path and nova remove`.

The task's test command: `cargo test --locked -p nova-cli --test deps`.

---

### Task 8: `nova new --lib` and `nova init --lib`

Spec §5.5.

**Files:**
- Modify: `crates/nova-cli/src/template.rs` (`Kind`, the library files)
- Modify: `crates/nova-cli/src/cmd/new.rs` (`--lib`)
- Modify: `crates/nova-cli/tests/project.rs` (new tests)

**Interfaces:**
- Consumes: Task 1's `nova_pm::import_name`; Task 6's test naming.
- Produces: `template::{Kind, files(name, Kind) -> Vec<(String, String)>}`.

- [ ] **Step 1: Write the failing tests**

Append to `crates/nova-cli/tests/project.rs`:

```rust

// === Phase 3.3a: library templates (spec
// docs/superpowers/specs/2026-10-08-phase-3-3a-local-packages-design.md §5.5) ===

const TEMPLATE_LIB: &str = "pub fn greeting() -> String {\n    \"Hello, Nova!\"\n}\n\n@test\nfn greeting_says_hello() {\n    assert_eq(greeting(), \"Hello, Nova!\")\n}\n";

#[test]
fn new_lib_writes_a_library_and_its_tests_pass() {
    let dir = fresh_dir("new-lib");
    let out = nova()
        .current_dir(&dir)
        .args(["new", "--lib", "json-api"])
        .assert()
        .success();
    assert_eq!(
        stdout(&out),
        "created `json-api`: nova.toml, .gitignore, README.md, src/lib.nova, \
         tests/json_api_test.nova\n"
    );
    let project = dir.join("json-api");
    assert_eq!(read(project.join("src").join("lib.nova")), TEMPLATE_LIB);
    assert_eq!(
        read(project.join("tests").join("json_api_test.nova")),
        "import json_api\n\n@test\nfn greeting_is_public() {\n    assert_eq(greeting(), \"Hello, Nova!\")\n}\n"
    );
    assert!(read(project.join("README.md")).contains("`nova test` runs the tests in"));
    assert!(!project.join("src").join("main.nova").exists());
    let tested = nova().current_dir(&project).arg("test").assert().success();
    let printed = stdout(&tested);
    assert!(printed.contains("2 passed; 0 failed"), "{printed}");
    assert!(
        printed.contains("test json_api_test::greeting_is_public ... ok"),
        "{printed}"
    );
}

#[test]
fn new_lib_refuses_a_keyword_name_before_writing() {
    let dir = fresh_dir("new-lib-keyword");
    let out = nova()
        .current_dir(&dir)
        .args(["new", "--lib", "match"])
        .assert()
        .failure();
    assert!(
        stderr(&out).contains("would be imported as `match`, which is a keyword"),
        "{}",
        stderr(&out)
    );
    assert!(!dir.join("match").exists());
    // A program may be called `match`: nothing imports it.
    nova()
        .current_dir(&dir)
        .args(["new", "match"])
        .assert()
        .success();
}

#[test]
fn init_lib_beside_a_program_makes_a_package_with_both() {
    let dir = fresh_dir("init-lib").join("both");
    std::fs::create_dir_all(dir.join("src")).unwrap();
    std::fs::write(dir.join("src").join("main.nova"), HELLO).unwrap();
    let out = nova()
        .current_dir(&dir)
        .args(["init", "--lib"])
        .assert()
        .success();
    assert_eq!(
        stdout(&out),
        "wrote: nova.toml, .gitignore, README.md, src/lib.nova, tests/both_test.nova\n"
    );
    assert_eq!(read(dir.join("src").join("main.nova")), HELLO);
    nova()
        .current_dir(&dir)
        .arg("run")
        .assert()
        .success()
        .stdout("hello from the project\n");
    let tested = nova().current_dir(&dir).arg("test").assert().success();
    assert!(
        stdout(&tested).contains("2 passed; 0 failed"),
        "{}",
        stdout(&tested)
    );
}
```

- [ ] **Step 2: Run them to verify they fail**

```bash
cd /d/Projects/nona/nova && P=/c/Users/SAKEER~1/AppData/Local/Temp/gcm/p33 && cargo test --locked -p nova-cli --test project > $P/t8-red.txt 2>&1; grep -E "^test .*(FAILED|ok)$" $P/t8-red.txt | grep -E "lib"
```

Expected: the three new tests FAILED (`--lib` is an unexpected argument).

- [ ] **Step 3: Implement**

In `crates/nova-cli/src/template.rs`:
- the module comment becomes:

  ```rust
  //! The files `nova new` and `nova init` write (spec
  //! `docs/superpowers/specs/2026-10-07-phase-3-0-foundations-design.md`
  //! §6.3, and for a library
  //! `docs/superpowers/specs/2026-10-08-phase-3-3a-local-packages-design.md`
  //! §5.5), with `\n` line endings on every system.
  ```
- after `const MAIN`, add:

  ```rust
  const LIB: &str = concat!(
      "pub fn greeting() -> String {\n",
      "    \"Hello, Nova!\"\n",
      "}\n",
      "\n",
      "@test\n",
      "fn greeting_says_hello() {\n",
      "    assert_eq(greeting(), \"Hello, Nova!\")\n",
      "}\n",
  );

  /// What a template makes.
  #[derive(Debug, Clone, Copy, PartialEq, Eq)]
  pub enum Kind {
      /// `src/main.nova`.
      Program,
      /// `src/lib.nova`, and a test in `tests/`.
      Library,
  }
  ```
- `pub fn files` becomes:

  ```rust
  /// The template of `kind` for a project called `name`, as (path, text)
  /// pairs. `name` has passed `nova_pm::check_name`, so it needs no TOML
  /// escaping.
  pub fn files(name: &str, kind: Kind) -> Vec<(String, String)> {
      let mut files = vec![
          (
              "nova.toml".to_string(),
              format!(
                  "[package]\nname = \"{name}\"\nversion = \"0.1.0\"\nedition = \"2026\"\n\n\
                   [dependencies]\n"
              ),
          ),
          (".gitignore".to_string(), "/target\n".to_string()),
      ];
      match kind {
          Kind::Program => {
              files.push((
                  "README.md".to_string(),
                  format!(
                      "# {name}\n\n`nova run` builds and runs `src/main.nova`; `nova test` runs \
                       its tests.\n"
                  ),
              ));
              files.push(("src/main.nova".to_string(), MAIN.to_string()));
          }
          Kind::Library => {
              // `_test` keeps the file's name from ever being the import name,
              // which would be E0004 (spec 3.3a §5.5).
              let import = nova_pm::import_name(name);
              files.push((
                  "README.md".to_string(),
                  format!(
                      "# {name}\n\nA library: other packages import it as `import {import}`. \
                       `nova test` runs the tests in `src/lib.nova` and `tests/`.\n"
                  ),
              ));
              files.push(("src/lib.nova".to_string(), LIB.to_string()));
              files.push((
                  format!("tests/{import}_test.nova"),
                  format!(
                      "import {import}\n\n@test\nfn greeting_is_public() {{\n    \
                       assert_eq(greeting(), \"Hello, Nova!\")\n}}\n"
                  ),
              ));
          }
      }
      files
  }
  ```

In `crates/nova-cli/src/cmd/new.rs`:
- the module comment's first line gains the library: "`nova new <name>`
  and `nova init [--name <name>]`, each with `--lib` for a library: write a
  new project (spec
  `docs/superpowers/specs/2026-10-07-phase-3-0-foundations-design.md` §6.3,
  and 3.3a's §5.5). Neither runs `git init`.";
- add `use crate::template::Kind;`;
- add to both `NewCmd` and `InitCmd`:

  ```rust
      /// Make a library: src/lib.nova and tests/, in place of src/main.nova.
      #[arg(long)]
      lib: bool,
  ```
- add, after `fn is_empty_dir`:

  ```rust
  /// The template `lib` asks for. A library's import name must not be a
  /// keyword (spec 3.3a §3.4), so it is refused before anything is written.
  fn kind(name: &str, lib: bool) -> Result<Kind> {
      if !lib {
          return Ok(Kind::Program);
      }
      let import = nova_pm::import_name(name);
      if nova_lexer::KEYWORDS.contains(&import.as_str()) {
          bail!(
              "a library called `{name}` would be imported as `{import}`, which is a keyword; \
               choose another name"
          );
      }
      Ok(Kind::Library)
  }

  /// The directories the template of `kind` writes into.
  fn dirs(kind: Kind) -> &'static [&'static str] {
      match kind {
          Kind::Program => &["src"],
          Kind::Library => &["src", "tests"],
      }
  }
  ```
- in `new`:
  - after `nova_pm::check_name(…)?;`, add `let kind = kind(&cmd.name, cmd.lib)?;`;
  - `fs::create_dir_all(dir.join("src"))…?;` becomes:

    ```rust
        for sub in dirs(kind) {
            fs::create_dir_all(dir.join(sub))
                .with_context(|| format!("creating {}", dir.display()))?;
        }
    ```
  - the loop becomes
    `for (path, text) in crate::template::files(&cmd.name, kind) {`, with
    `write_new(&dir.join(&path), &text)?`;
- in `init`:
  - after the `let name = match … };` statement, add
    `let kind = kind(&name, cmd.lib)?;`;
  - `fs::create_dir_all(cwd.join("src")).context("creating src")?;` becomes:

    ```rust
        for sub in dirs(kind) {
            fs::create_dir_all(cwd.join(sub)).with_context(|| format!("creating {sub}"))?;
        }
    ```
  - the loop becomes `for (path, text) in crate::template::files(&name, kind) {`,
    with `write_new(&cwd.join(&path), &text)?`.

`wrote` and `kept` now hold `String`s, and `join(", ")` is unchanged.

- [ ] **Step 4: Run the tests to verify they pass**

```bash
cd /d/Projects/nona/nova && P=/c/Users/SAKEER~1/AppData/Local/Temp/gcm/p33 && cargo test --locked -p nova-cli --test project > $P/t8.txt 2>&1; echo "exit=$?"; python -X utf8 $P/count.py $P/t8.txt; grep -E "FAILED|panicked|^warning|^error" $P/t8.txt | head
```

Expected: `exit=0`, 0 failed, no warnings. The program template's tests
pass unchanged.

- [ ] **Step 5: Commit**

Write `$P/msg-8.txt`:

```
nova-cli: nova new --lib and nova init --lib

A library template (spec
docs/superpowers/specs/2026-10-08-phase-3-3a-local-packages-design.md
§5.5): src/lib.nova with a public function and a test, and
tests/<import name>_test.nova, which imports the package and tests that
function. The `_test` suffix keeps that file's name from being the import
name, which would be E0004. The README says what `nova test` runs.

A library whose import name is a keyword is refused before anything is
written; a program may still be called `match`. `nova init --lib` beside
an existing src/main.nova makes a package with both. The program
template is unchanged.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
```

```bash
cd /d/Projects/nona/nova && P=/c/Users/SAKEER~1/AppData/Local/Temp/gcm/p33 && cargo fmt --all && git add crates/nova-cli && git commit -q -F $P/msg-8.txt && git log -1 --format=%s
```

Expected: `nova-cli: nova new --lib and nova init --lib`.

The task's test command: `cargo test --locked -p nova-cli --test project`.

---

### Task 9: `nova fmt` on `tests/`

Spec §5.4.

**Files:**
- Modify: `crates/nova-cli/src/cmd/fmt.rs` (`collect`, `search`)
- Modify: `crates/nova-cli/tests/fmt.rs` (one test)

**Interfaces:**
- Consumes: nothing new.
- Produces: `search(dir, files, skip_packages: bool)`, inside `fmt.rs`.

- [ ] **Step 1: Write the failing test**

Append to `crates/nova-cli/tests/fmt.rs`:

```rust

#[test]
fn no_path_formats_tests_too_and_skips_a_nested_package() {
    // Phase 3.3a (spec
    // docs/superpowers/specs/2026-10-08-phase-3-3a-local-packages-design.md §5.4).
    let dir = fresh_dir("tests-dir");
    let manifest = "[package]\nname = \"demo\"\nversion = \"0.1.0\"\nedition = \"2026\"\n";
    write(&dir, "nova.toml", manifest);
    let main = write(&dir, "src/main.nova", UNFORMATTED);
    let test = write(&dir, "tests/api.nova", UNFORMATTED);
    write(&dir, "src/vendor/geom/nova.toml", manifest.replace("demo", "geom"));
    let nested = write(&dir, "src/vendor/geom/src/lib.nova", UNFORMATTED);
    nova().arg("fmt").current_dir(&dir).assert().success();
    assert_eq!(read(&main), FORMATTED);
    assert_eq!(read(&test), FORMATTED);
    assert_eq!(read(&nested), UNFORMATTED, "a nested package is its own");
}
```

- [ ] **Step 2: Run it to verify it fails**

```bash
cd /d/Projects/nona/nova && cargo test --locked -p nova-cli --test fmt no_path_formats_tests_too 2>&1 | grep -E "^test |panicked" | head -3
```

Expected: FAILED, the `tests/api.nova` assertion: it is left unformatted.

- [ ] **Step 3: Implement**

In `crates/nova-cli/src/cmd/fmt.rs`:
- `collect`'s doc comment becomes "The files to format, sorted (spec §7.1):
  the paths given, each directory searched for `*.nova`. With none, the
  project's `src/` and `tests/`, skipping a package nested in them (3.3a
  §5.4), or outside a project `src/` if `src/main.nova` exists.";
- in `collect`, the block from `let src = match nova_pm::find_root(&cwd) {`
  to the `search(&src, …)` line becomes:

  ```rust
          let (src, tests) = match nova_pm::find_root(&cwd) {
              Some(root) => (root.join("src"), Some(root.join("tests"))),
              None if Path::new("src/main.nova").is_file() => (PathBuf::from("src"), None),
              None => {
                  return Err("no project here, and no src/main.nova: \
                              name the files or directories to format"
                      .to_owned())
              }
          };
          search(&src, &mut files, true)
              .map_err(|e| format!("searching {}: {e}", src.display()))?;
          // A missing tests/ is not an error.
          if let Some(tests) = tests.filter(|tests| tests.is_dir()) {
              search(&tests, &mut files, true)
                  .map_err(|e| format!("searching {}: {e}", tests.display()))?;
          }
  ```
- the paths loop's call becomes `search(path, &mut files, false)`;
- `search` becomes:

  ```rust
  /// Every `*.nova` file under `dir`, but not in `target/`, in a directory
  /// whose name begins with `.`, or behind a symbolic link to a directory.
  /// With `skip_packages`, a directory holding a `nova.toml` is skipped too:
  /// it is another package (spec 3.3a §5.4).
  fn search(dir: &Path, files: &mut Vec<PathBuf>, skip_packages: bool) -> std::io::Result<()> {
      for entry in std::fs::read_dir(dir)? {
          let entry = entry?;
          let kind = entry.file_type()?;
          let path = entry.path();
          let name = entry.file_name().to_string_lossy().into_owned();
          if kind.is_dir() {
              let package = skip_packages && path.join(nova_pm::MANIFEST).is_file();
              if name != "target" && !name.starts_with('.') && !package {
                  search(&path, files, skip_packages)?;
              }
          } else if name.ends_with(".nova") && (kind.is_file() || path.is_file()) {
              files.push(path);
          }
      }
      Ok(())
  }
  ```

- [ ] **Step 4: Run the tests to verify they pass**

```bash
cd /d/Projects/nona/nova && P=/c/Users/SAKEER~1/AppData/Local/Temp/gcm/p33 && cargo test --locked -p nova-cli --test fmt > $P/t9.txt 2>&1; echo "exit=$?"; python -X utf8 $P/count.py $P/t9.txt; grep -E "FAILED|panicked|^warning|^error" $P/t9.txt | head
```

Expected: `exit=0`, 0 failed, no warnings.

- [ ] **Step 5: Commit**

Write `$P/msg-9.txt`:

```
nova-cli: nova fmt formats tests/ and skips nested packages

With no paths, in a project, `nova fmt` formats src/ and tests/ (spec
docs/superpowers/specs/2026-10-08-phase-3-3a-local-packages-design.md
§5.4). It skips any directory below them that holds a nova.toml, so a
path dependency kept inside src/ is not formatted. A missing tests/ is
not an error. Paths given on the command line are searched as before.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
```

```bash
cd /d/Projects/nona/nova && P=/c/Users/SAKEER~1/AppData/Local/Temp/gcm/p33 && cargo fmt --all && git add crates/nova-cli && git commit -q -F $P/msg-9.txt && git log -1 --format=%s
```

Expected: `nova-cli: nova fmt formats tests/ and skips nested packages`.

The task's test command: `cargo test --locked -p nova-cli --test fmt`.

---

### Task 10: The language server analyses packages

Spec §6: a project's analysis takes the program, the library and `tests/`,
and publishes only its own package's modules.

**Files:**
- Modify: `crates/nova-lsp/src/workspace.rs` (`ProjectKey::of`; `entry`
  goes)
- Modify: `crates/nova-lsp/src/checker.rs` (`run`, `check`, `publish_own`)
- Modify: `crates/nova-lsp/src/convert.rs` (own modules are the root
  package's)
- Modify: `crates/nova-lsp/src/completion.rs` (`analysis_at`)
- Modify: `crates/nova-cli/tests/lsp.rs` (new tests)

**Interfaces:**
- Consumes:
  - Task 4's `analyze_program`, `Program::{loose, for_file, for_package}`,
    `Roots::Test`, `Analysis.module_packages` and `nova_driver::package_of`
    (public since Task 4);
  - `nova_pm::PackageId`.
- Produces:
  - `ProjectKey::of(path)`: `Root(dir)` only for a file directly in a
    package's `src/` or `tests/`;
  - in `crates/nova-cli/tests/lsp.rs`: `APP_MAIN` and
    `app_and_library(name, lib) -> (PathBuf, PathBuf)`, which Task 11
    reuses.

- [ ] **Step 1: Write the failing tests**

Append to `crates/nova-cli/tests/lsp.rs`:

```rust

// === Phase 3.3a: packages (spec
// docs/superpowers/specs/2026-10-08-phase-3-3a-local-packages-design.md §6, §7.4) ===

const APP_MAIN: &str = "import geom\n\nfn main() {\n    let a: Int = area()\n}\n";

/// `app`, which depends on `geom` by path, in one fresh directory. `geom`'s
/// `src/lib.nova` is `lib`.
fn app_and_library(name: &str, lib: &str) -> (std::path::PathBuf, std::path::PathBuf) {
    let dir = fresh_dir(name);
    let geom = dir.join("geom");
    std::fs::create_dir_all(geom.join("src")).unwrap();
    std::fs::write(geom.join("nova.toml"), MANIFEST.replace("demo", "geom")).unwrap();
    std::fs::write(geom.join("src").join("lib.nova"), lib).unwrap();
    let app = dir.join("app");
    std::fs::create_dir_all(app.join("src")).unwrap();
    let manifest = format!(
        "{}\n[dependencies]\ngeom = {{ path = \"../geom\" }}\n",
        MANIFEST.replace("demo", "app")
    );
    std::fs::write(app.join("nova.toml"), manifest).unwrap();
    std::fs::write(app.join("src").join("main.nova"), APP_MAIN).unwrap();
    (app, geom)
}

#[test]
fn the_apps_analysis_publishes_nothing_for_its_dependency() {
    let (app, geom) = app_and_library("dependency-owner", GEOMETRY_BROKEN);
    let main_uri = file_uri(&app.join("src").join("main.nova"));
    let mut client = Client::start(&app, false);
    open(&mut client, &main_uri, APP_MAIN);
    client.diagnostics(&main_uri, |_| true);
    // Each sentinel's publish comes after everything the app's check sent.
    let first = sentinel(&mut client, "dependency-owner-1");
    let lib = file_uri(&geom.join("src").join("lib.nova"));
    let published = client.last_diagnostics_before(&lib, &first);
    assert!(published.is_none(), "the app published for geom's lib.nova: {published:?}");
    let second = sentinel(&mut client, "dependency-owner-2");
    let manifest = file_uri(&geom.join("nova.toml"));
    let published = client.last_diagnostics_before(&manifest, &second);
    assert!(published.is_none(), "the app published for geom's nova.toml: {published:?}");
}

#[test]
fn a_tests_file_gets_its_diagnostics() {
    // A guard: the server already checked an unreached file on its own, and
    // the driver finds a tests/ file's package (Task 4).
    let dir = project("tests-file", &[("lib.nova", GEOMETRY_FIXED)]);
    let text = "import demo\n\n@test\nfn area_is_text() {\n    let s: String = area()\n}\n";
    std::fs::create_dir_all(dir.join("tests")).unwrap();
    let file = dir.join("tests").join("api.nova");
    std::fs::write(&file, text).unwrap();
    let uri = file_uri(&file);
    let mut client = Client::start(&dir, false);
    open(&mut client, &uri, text);
    let params = client.diagnostics(&uri, nonempty);
    assert_eq!(codes(&params), ["E0010"], "{params}");
}

#[test]
fn a_library_without_a_program_is_checked_as_a_module() {
    // A guard: a library gets its own errors, and no E0601.
    let dir = project("library-only", &[("lib.nova", GEOMETRY_BROKEN)]);
    let uri = file_uri(&dir.join("src").join("lib.nova"));
    let mut client = Client::start(&dir, false);
    open(&mut client, &uri, GEOMETRY_BROKEN);
    let params = client.diagnostics(&uri, nonempty);
    assert_eq!(codes(&params), ["E0010"], "{params}");
}
```

- [ ] **Step 2: Run them to verify they fail**

```bash
cd /d/Projects/nona/nova && P=/c/Users/SAKEER~1/AppData/Local/Temp/gcm/p33 && cargo test --locked -p nova-cli --test lsp > $P/t10-red.txt 2>&1; grep -E "^test .*(FAILED|ok)$" $P/t10-red.txt | grep -E "dependency|tests_file|library_without"
```

Expected:
- FAILED: `the_apps_analysis_publishes_nothing_for_its_dependency`. Since
  Task 4 the app's analysis reaches `geom`'s `lib.nova`, and publishes for
  every module it reaches;
- ok: `a_tests_file_gets_its_diagnostics` and
  `a_library_without_a_program_is_checked_as_a_module` (guards).

- [ ] **Step 3: Implement**

In `crates/nova-lsp/src/workspace.rs`:
- `enum ProjectKey`'s doc comment and its `Root` variant become:

  ```rust
  /// A project, or a loose file (spec §6.2, and 3.3a §6). Two keys are equal
  /// when their paths' `PathKey`s are.
  #[derive(Debug, Clone)]
  pub enum ProjectKey {
      /// A package, by its directory in its real spelling. It owns the files
      /// directly in its `src/` and `tests/`.
      Root(PathBuf),
  ```
- `ProjectKey::of` becomes:

  ```rust
      /// The project `path` belongs to: the package whose `src/` or `tests/`
      /// it is directly in, as the driver finds it (spec 3.3a §4.1), or none.
      pub fn of(path: &Path) -> ProjectKey {
          match nova_driver::package_of(path) {
              Some((root, _)) => ProjectKey::Root(real_path(&root)),
              None => ProjectKey::Loose(path.to_path_buf()),
          }
      }
  ```
- delete `pub fn entry(&self) -> PathBuf { … }` and its doc comment: a
  project's analysis is now `Program::for_package`, which needs no entry.

In `crates/nova-lsp/src/checker.rs`:
- the imports become:

  ```rust
  use std::collections::{HashMap, HashSet};
  use std::path::{Path, PathBuf};
  use std::sync::{mpsc, Arc, Mutex};

  use nova_diagnostics::FileId;
  use nova_driver::{analyze_program, Analysis, Options, Program, Roots, Sources};
  use nova_pm::PackageId;
  ```
- `fn run` becomes:

  ```rust
  fn run(program: Program, overlay: &Overlay, module_only: bool) -> Result<Analysis, Failed> {
      let entry = program
          .roots
          .first()
          .map(|root| root.path.clone())
          .unwrap_or_default();
      let options = Options {
          keep_going: true,
          tests: true,
          module_only,
          probe: None,
      };
      match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
          analyze_program(program, overlay, &options)
      })) {
          Ok(Ok(a)) => Ok(a),
          Ok(Err(e)) => {
              tracing::warn!("nova lsp: cannot read {}: {e}", entry.display());
              Err(Failed::Unreadable)
          }
          Err(_) => {
              tracing::warn!("nova lsp: the front end panicked on {}", entry.display());
              Err(Failed::Panicked)
          }
      }
  }
  ```
- `check`'s doc comment's first two items become:

  ```rust
  /// - a project's analysis, from its program, library and `tests/` (spec
  ///   3.3a §6), owns its own package's modules;
  /// - each open project file it does not reach is checked on its own, as a
  ///   module;
  ```
- in `check`:
  - the loose arm's call becomes `run(Program::loose(file), &job.overlay, module_only)`;
  - the `ProjectKey::Root(_)` arm becomes `ProjectKey::Root(dir)`, and its
    first `run` call and comment become:

    ```rust
                // The program, the library and tests/, in test mode; a
                // library alone is checked as a module (spec 3.3a §6).
                match run(Program::for_package(dir, Roots::Test), &job.overlay, false) {
    ```
  - the comment `` // No `src/main.nova`: every open file is checked on its own. ``
    becomes `// Nothing to read: every open file is checked on its own.`;
  - the open documents' call becomes
    `run(Program::for_file(&doc.path), &job.overlay, true)`;
- `publish_own` becomes:

  ```rust
  /// One publish per file `analysis` owns (spec 3.3a §6): with `all`, each
  /// module of the root package, else its entry alone. A dependency's files
  /// are its own project's to publish.
  fn publish_own(job: &Job, analysis: &Analysis, all: bool, out: &mut Vec<Publish>) {
      let Some(&(entry, _)) = analysis.modules.first() else {
          return;
      };
      let uri_of = |path: &Path| match job
          .open
          .iter()
          .find(|d| PathKey::of(&d.path) == PathKey::of(path))
      {
          Some(doc) => doc.uri.clone(),
          None => uri::from_path(path),
      };
      let owned: Vec<&(FileId, PathBuf)> = if all {
          analysis
              .modules
              .iter()
              .zip(&analysis.module_packages)
              .filter(|(_, package)| matches!(package, None | Some(PackageId(0))))
              .map(|(module, _)| module)
              .collect()
      } else {
          analysis.modules.iter().take(1).collect()
      };
      for (file, path) in owned {
          let version = job
              .open
              .iter()
              .find(|d| PathKey::of(&d.path) == PathKey::of(path))
              .map(|d| d.version);
          out.push(Publish {
              uri: uri_of(path),
              version,
              diagnostics: convert::diagnostics_for(analysis, *file, entry, &uri_of),
          });
      }
  }
  ```

In `crates/nova-lsp/src/convert.rs`:
- add `use nova_pm::PackageId;`;
- in `diagnostics_for`, `let own: Vec<FileId> = …;` becomes:

  ```rust
      // A dependency's modules are its own project's (spec 3.3a §6).
      let own: Vec<FileId> = analysis
          .modules
          .iter()
          .zip(&analysis.module_packages)
          .filter(|(_, package)| matches!(package, None | Some(PackageId(0))))
          .map(|((file, _), _)| *file)
          .collect();
  ```
- its doc comment's "in one of the program's own files" becomes "in one
  of the root package's own modules", and "One with no label in the
  program's files" becomes "One with no label there".

In `crates/nova-lsp/src/completion.rs`:
- `use nova_driver::{analyze, Analysis, Options, Probe};` becomes
  `use nova_driver::{analyze, analyze_program, Analysis, Options, Probe, Program, Roots};`;
- in `analysis_at`, the project branch becomes:

  ```rust
      let project = ProjectKey::of(path);
      if let ProjectKey::Root(dir) = &project {
          let program = Program::for_package(dir, Roots::Test);
          if let Some(a) = guarded(|| analyze_program(program, overlay, &options).ok()) {
  ```

  The rest of the branch is unchanged. Its doc comment says "its project's
  analysis, if that reaches it, or else its own, which
  `Program::for_file` finds the package of (spec 3.3a §6)".

- [ ] **Step 4: Run the tests to verify they pass**

```bash
cd /d/Projects/nona/nova && P=/c/Users/SAKEER~1/AppData/Local/Temp/gcm/p33 && cargo test --locked -p nova-lsp -p nova-driver > $P/t10-lib.txt 2>&1; echo "lib exit=$?"; cargo test --locked -p nova-cli --test lsp > $P/t10.txt 2>&1; echo "exit=$?"; python -X utf8 $P/count.py $P/t10.txt; grep -hE "FAILED|panicked|^warning|^error" $P/t10-lib.txt $P/t10.txt | head
```

Expected: `lib exit=0`, then `exit=0`, 0 failed and no warnings. Every 3.2
server test passes unchanged, `a_nested_project_owns_its_own_files`
included.

- [ ] **Step 5: Commit**

Write `$P/msg-10.txt`:

```
nova-lsp: projects are packages, and publish only their own modules

Spec docs/superpowers/specs/2026-10-08-phase-3-3a-local-packages-design.md
§6:
- **Projects.** A file directly in a package's src/ or tests/ belongs to
  that package's project, found as the driver finds it. Any other file
  is loose.
- **Analysis.** A project's analysis takes the program, the library and
  every tests/*.nova, in test mode; a library alone is checked as a
  module. An open file it does not reach is checked on its own, with its
  package's dependencies.
- **One owner per file.** A project publishes only its own package's
  modules. A dependency's files are its own project's to publish, so two
  analyses never publish for one file.

A dependency's problems fall back to the entry's first line for now.
Completion analyses the project the same way.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
```

```bash
cd /d/Projects/nona/nova && P=/c/Users/SAKEER~1/AppData/Local/Temp/gcm/p33 && cargo fmt --all && git add crates/nova-lsp crates/nova-cli && git commit -q -F $P/msg-10.txt && git log -1 --format=%s
```

Expected: `nova-lsp: projects are packages, and publish only their own modules`.

The task's test command:
`cargo test --locked -p nova-lsp && cargo test --locked -p nova-cli --test lsp`.

---

### Task 11: Dependency problems, manifests, and re-checks across packages

Spec §6: a dependency's problem shows on the dependent's manifest entry, a
project publishes its own `nova.toml`, and an edit in a dependency
re-checks its dependents.

**Files:**
- Modify: `crates/nova-driver/src/program.rs` (`Program::manifest`),
  `crates/nova-driver/src/analyze.rs` (`Analysis.manifest`)
- Modify: `crates/nova-lsp/src/convert.rs` (where a diagnostic is shown)
- Modify: `crates/nova-lsp/src/checker.rs` (`Checker.dirs`, `reaches`;
  `publish_own` publishes `nova.toml`)
- Modify: `crates/nova-lsp/src/lib.rs` (`affected` and the watched-files
  handler use `reaches`)
- Modify: `crates/nova-cli/tests/lsp.rs` (new tests)

**Interfaces:**
- Consumes:
  - Task 2's `Graph::{reached_through, dirs, package}` and
    `GraphPackage.manifest_file`;
  - Task 10's `publish_own` and `APP_MAIN`/`app_and_library`.
- Produces:
  - `Program::manifest(&self) -> Option<FileId>`;
  - `Analysis.manifest: Option<FileId>`;
  - `Checker::reaches(&self, &ProjectKey, &Path) -> bool`;
  - `convert::diagnostics_for(analysis, file, entry, manifest: Option<FileId>, uri_of)`.

- [ ] **Step 1: Write the failing tests**

Append to `crates/nova-cli/tests/lsp.rs`:

```rust

#[test]
fn a_dependency_error_shows_on_the_apps_manifest_entry() {
    let (app, _geom) = app_and_library("dependency-error", GEOMETRY_BROKEN);
    let mut client = Client::start(&app, false);
    open(&mut client, &file_uri(&app.join("src").join("main.nova")), APP_MAIN);
    let params = client.diagnostics(&file_uri(&app.join("nova.toml")), nonempty);
    assert_eq!(codes(&params), ["E0010"], "{params}");
    let d = &params["diagnostics"][0];
    assert_eq!(d["range"]["start"]["line"], 6, "the `geom` entry: {params}");
    let message = d["message"].as_str().unwrap();
    assert!(
        message.contains("(in geom: ") && message.contains("lib.nova:1:"),
        "{message}"
    );
}

#[test]
fn editing_a_dependency_rechecks_its_dependent() {
    let (app, geom) = app_and_library("dependency-edit", GEOMETRY_FIXED);
    let toml = file_uri(&app.join("nova.toml"));
    let lib = file_uri(&geom.join("src").join("lib.nova"));
    let mut client = Client::start(&app, false);
    open(&mut client, &file_uri(&app.join("src").join("main.nova")), APP_MAIN);
    client.diagnostics(&toml, |p| !nonempty(p));
    open(&mut client, &lib, GEOMETRY_FIXED);
    client.diagnostics(&lib, |p| !nonempty(p));
    // geom's buffer, never saved: the app reads it through the overlay.
    change(&mut client, &lib, 2, GEOMETRY_BROKEN);
    let params = client.diagnostics(&toml, nonempty);
    assert_eq!(codes(&params), ["E0010"], "{params}");
}

#[test]
fn a_manifest_error_is_published_under_its_nova_toml() {
    let dir = project("manifest-error", &[("main.nova", "fn main() {}\n")]);
    std::fs::write(dir.join("nova.toml"), MANIFEST.replace("2026", "2021")).unwrap();
    let mut client = Client::start(&dir, false);
    open(
        &mut client,
        &file_uri(&dir.join("src").join("main.nova")),
        "fn main() {}\n",
    );
    let params = client.diagnostics(&file_uri(&dir.join("nova.toml")), nonempty);
    assert_eq!(codes(&params), ["M0003"], "{params}");
    assert_eq!(params["diagnostics"][0]["range"]["start"]["line"], 3, "{params}");
}

#[test]
fn a_dependencys_manifest_is_published_by_its_own_project_only() {
    let (app, geom) = app_and_library("dependency-manifest", GEOMETRY_FIXED);
    let warned = format!(
        "{}\n[features]\ndefault = []\n",
        MANIFEST.replace("demo", "geom")
    );
    std::fs::write(geom.join("nova.toml"), warned).unwrap();
    let geom_toml = file_uri(&geom.join("nova.toml"));
    let mut client = Client::start(&app, false);
    open(&mut client, &file_uri(&app.join("src").join("main.nova")), APP_MAIN);
    let sentinel = sentinel(&mut client, "dependency-manifest");
    let published = client.last_diagnostics_before(&geom_toml, &sentinel);
    assert!(published.is_none(), "the app published geom's nova.toml: {published:?}");
    client.clear_unread();
    open(
        &mut client,
        &file_uri(&geom.join("src").join("lib.nova")),
        GEOMETRY_FIXED,
    );
    let params = client.diagnostics(&geom_toml, nonempty);
    assert_eq!(codes(&params), ["M0006"], "{params}");
}
```

- [ ] **Step 2: Run them to verify they fail**

```bash
cd /d/Projects/nona/nova && P=/c/Users/SAKEER~1/AppData/Local/Temp/gcm/p33 && cargo test --locked -p nova-cli --test lsp > $P/t11-red.txt 2>&1; grep -E "^test .*(FAILED|ok)$" $P/t11-red.txt | grep -E "manifest|dependency_error|editing"
```

Expected: the four new tests FAILED. Nothing publishes a `nova.toml` yet,
so each waits for a publish that never comes and panics with "no matching
message". `the_apps_analysis_publishes_nothing_for_its_dependency` stays
ok.

- [ ] **Step 3: The driver records the manifest**

In `crates/nova-driver/src/program.rs`, add to `impl Program`, after
`has_errors`:

```rust
    /// The root package's `nova.toml` in `db`, once read. The language
    /// server publishes the graph's problems under it (spec §6).
    pub fn manifest(&self) -> Option<FileId> {
        match &self.graph {
            Some(graph) => Some(graph.root().manifest_file),
            // Without a graph only the root's manifest was read, and each of
            // its diagnostics is labelled in it.
            None => self
                .diagnostics
                .iter()
                .flat_map(|d| &d.labels)
                .map(|label| label.span.file)
                .next(),
        }
    }
```

In `crates/nova-driver/src/analyze.rs`:
- add to `struct Analysis`, after `pub graph: Option<Graph>,`:

  ```rust
      /// The root package's `nova.toml`, when the program has one
      /// ([`Program::manifest`]).
      pub manifest: Option<FileId>,
  ```
- in `analyze_program`, add `let manifest = program.manifest();` before
  `let Program { … } = program;`, and `manifest,` after `graph,` in the
  `Analysis` literal.

- [ ] **Step 4: Where a diagnostic is shown**

In `crates/nova-lsp/src/convert.rs`:
- the imports become:

  ```rust
  use std::collections::HashMap;
  use std::path::Path;

  use lsp_types as lsp;
  use nova_diagnostics::{Diagnostic, FileId, Label, LineIndex, Severity, Span};
  use nova_driver::Analysis;
  use nova_pm::PackageId;
  ```
- `diagnostics_for` becomes:

  ```rust
  /// The LSP diagnostics `analysis` has for `file` (spec §6.3, and 3.3a §6).
  ///
  /// Each diagnostic is shown in one place:
  /// - the file of its primary label, or of its first label, among the root
  ///   package's own modules;
  /// - else, given the project's `manifest`, that `nova.toml` for a label in
  ///   it;
  /// - else, for a label in a dependency's file or manifest, the root's
  ///   manifest entry through which the dependency is reached, its message
  ///   naming where it really is, such as "(in geom: ../geom/src/lib.nova:3:5)";
  /// - else `entry`'s first line, naming the place it has: the 3.2
  ///   fallback, for E0601 and for labels in std.
  ///
  /// Its other labels in the own modules become related information, under
  /// the URIs `uri_of` gives their paths.
  pub fn diagnostics_for(
      analysis: &Analysis,
      file: FileId,
      entry: FileId,
      manifest: Option<FileId>,
      uri_of: &dyn Fn(&Path) -> String,
  ) -> Vec<lsp::Diagnostic> {
      // A dependency's modules are its own project's (spec 3.3a §6).
      let own: Vec<FileId> = analysis
          .modules
          .iter()
          .zip(&analysis.module_packages)
          .filter(|(_, package)| matches!(package, None | Some(PackageId(0))))
          .map(|((file, _), _)| *file)
          .collect();
      let mut indexes: HashMap<FileId, LineIndex> = HashMap::new();
      let mut out = Vec::new();
      for d in &analysis.diagnostics {
          let shown = shown(analysis, d, entry, manifest, &own);
          if shown.file != file {
              continue;
          }
          let at = span_range(analysis, &mut indexes, shown.at);
          let mut related = Vec::new();
          for other in &d.labels {
              if shown.label.is_some_and(|l| std::ptr::eq(l, other))
                  || !own.contains(&other.span.file)
              {
                  continue;
              }
              let Some((_, path)) = analysis.modules.iter().find(|(f, _)| *f == other.span.file)
              else {
                  continue;
              };
              let Some(uri) = crate::uri::parse(&uri_of(path)) else {
                  continue;
              };
              related.push(lsp::DiagnosticRelatedInformation {
                  location: lsp::Location {
                      uri,
                      range: span_range(analysis, &mut indexes, other.span),
                  },
                  message: other.message.clone(),
              });
          }
          out.push(lsp::Diagnostic {
              range: at,
              severity: Some(severity(d.severity)),
              code: Some(lsp::NumberOrString::String(d.code.clone())),
              source: Some("nova".to_string()),
              message: message(d, shown.suffix.as_deref()),
              related_information: (!related.is_empty()).then_some(related),
              ..Default::default()
          });
      }
      out
  }

  /// Where one diagnostic is shown.
  struct Shown<'d> {
      file: FileId,
      at: Span,
      /// The label it is shown at, when that is one of its own.
      label: Option<&'d Label>,
      /// Where it really is, when it is shown somewhere else.
      suffix: Option<String>,
  }

  fn shown<'d>(
      analysis: &Analysis,
      d: &'d Diagnostic,
      entry: FileId,
      manifest: Option<FileId>,
      own: &[FileId],
  ) -> Shown<'d> {
      let label = d
          .labels
          .iter()
          .find(|l| l.primary && own.contains(&l.span.file))
          .or_else(|| d.labels.iter().find(|l| own.contains(&l.span.file)));
      if let Some(l) = label {
          return Shown {
              file: l.span.file,
              at: l.span,
              label: Some(l),
              suffix: None,
          };
      }
      let first = d.labels.iter().find(|l| l.primary).or(d.labels.first());
      if let (Some(manifest), Some(l)) = (manifest, first) {
          if l.span.file == manifest {
              return Shown {
                  file: manifest,
                  at: l.span,
                  label: Some(l),
                  suffix: None,
              };
          }
          let graph = analysis.graph.as_ref();
          let reached = package_of_file(analysis, l.span.file)
              .filter(|package| *package != PackageId(0))
              .and_then(|package| Some((package, graph?.reached_through(package)?)));
          if let (Some(graph), Some((package, at))) = (graph, reached) {
              let name = &graph.package(package).name;
              return Shown {
                  file: manifest,
                  at,
                  label: None,
                  suffix: Some(format!("(in {name}: {})", place(analysis, l.span))),
              };
          }
      }
      Shown {
          file: entry,
          at: Span::point(0, entry),
          label: None,
          suffix: d
              .labels
              .first()
              .map(|l| format!("(at {})", place(analysis, l.span))),
      }
  }

  /// The package `file` belongs to: a module's, or a manifest's.
  fn package_of_file(analysis: &Analysis, file: FileId) -> Option<PackageId> {
      if let Some(i) = analysis.modules.iter().position(|(f, _)| *f == file) {
          return analysis.module_packages[i];
      }
      let graph = analysis.graph.as_ref()?;
      graph
          .packages
          .iter()
          .position(|p| p.manifest_file == file)
          .map(|i| PackageId(i as u32))
  }

  /// `name:line:column` of where `span` starts.
  fn place(analysis: &Analysis, span: Span) -> String {
      let name = analysis.db.get_name(span.file).unwrap_or("?");
      match analysis.db.location(span.file, span.start) {
          Some((line, column)) => format!("{name}:{line}:{column}"),
          None => name.to_string(),
      }
  }
  ```
- `fn message` becomes:

  ```rust
  /// The message, its notes one per line, and where it really is when it is
  /// shown somewhere else, such as `(at <std/core>:12:5)`.
  fn message(d: &Diagnostic, suffix: Option<&str>) -> String {
      let mut m = d.message.clone();
      for note in &d.notes {
          m.push('\n');
          m.push_str(note);
      }
      if let Some(suffix) = suffix {
          m.push('\n');
          m.push_str(suffix);
      }
      m
  }
  ```

- [ ] **Step 5: Publishing `nova.toml`, and re-checks**

In `crates/nova-lsp/src/checker.rs`:
- `struct Checker` gains a third field, after `newest`:

  ```rust
      /// Each project's package directories, from its last published check
      /// (spec 3.3a §6, decision 10).
      dirs: Arc<Mutex<HashMap<ProjectKey, Vec<PathKey>>>>,
  ```
- `spawn` becomes:

  ```rust
      pub fn spawn(publish: impl Fn(Publish) + Send + 'static) -> Checker {
          let (jobs, queue) = mpsc::channel::<Job>();
          let newest: Arc<Mutex<HashMap<ProjectKey, u64>>> = Arc::default();
          let dirs: Arc<Mutex<HashMap<ProjectKey, Vec<PathKey>>>> = Arc::default();
          let seen = Arc::clone(&newest);
          let written = Arc::clone(&dirs);
          std::thread::Builder::new()
              .name("nova-lsp-checker".to_string())
              .spawn(move || serve(&queue, &seen, &written, &publish))
              .expect("start the checker thread");
          Checker { jobs, newest, dirs }
      }
  ```
- add after `submit`:

  ```rust
      /// Whether a change to `path` can change `project`'s diagnostics (spec
      /// 3.3a §6): it is under one of the project's package directories.
      /// Before a project's first check is published, and for a loose file,
      /// that is [`ProjectKey::holds`].
      pub fn reaches(&self, project: &ProjectKey, path: &Path) -> bool {
          let dirs = self.dirs.lock().expect("the dirs map");
          match dirs.get(project) {
              Some(dirs) if !dirs.is_empty() => {
                  let key = PathKey::of(path);
                  dirs.iter().any(|dir| key.is_under(dir))
              }
              _ => project.holds(path),
          }
      }
  ```
- `serve` takes `dirs: &Mutex<HashMap<ProjectKey, Vec<PathKey>>>` after
  `newest`, and:
  - `Some(Vec::new())` for a clear job becomes `Some((Vec::new(), Vec::new()))`;
  - `let Some(results) = results else {` becomes
    `let Some((results, reached)) = results else {`;
  - its last `if !job.clear { … }` becomes:

    ```rust
                let mut known = dirs.lock().expect("the dirs map");
                if job.clear {
                    known.remove(&job.project);
                } else {
                    published.insert(job.project.clone(), now);
                    known.insert(job.project.clone(), reached);
                }
    ```
- `check` returns `Option<(Vec<Publish>, Vec<PathKey>)>`, "the publishes,
  and the project's package directories". In it:
  - `let mut dirs: Vec<PathKey> = Vec::new();` after `let mut out = Vec::new();`;
  - first thing in the `ProjectKey::Root(dir)` arm:
    `dirs.push(PathKey::of(dir));`;
  - in that arm's `Ok(a)`, before `reached = …`:

    ```rust
                        if let Some(graph) = &a.graph {
                            dirs = graph.dirs().iter().map(|d| PathKey::of(d)).collect();
                        }
    ```
  - its last line becomes `Some((out, dirs))`;
- `publish_own` becomes:

  ```rust
  /// One publish per file `analysis` owns (spec 3.3a §6): with `all`, each
  /// module of the root package and the root's `nova.toml`, else its entry
  /// alone. A dependency's files are its own project's to publish.
  fn publish_own(job: &Job, analysis: &Analysis, all: bool, out: &mut Vec<Publish>) {
      let manifest = analysis.manifest.filter(|_| all);
      // A package with no module to read has only its manifest.
      let Some(entry) = analysis
          .modules
          .first()
          .map(|(file, _)| *file)
          .or(manifest)
      else {
          return;
      };
      let open = |path: &Path| {
          job.open
              .iter()
              .find(|d| PathKey::of(&d.path) == PathKey::of(path))
      };
      let uri_of = |path: &Path| match open(path) {
          Some(doc) => doc.uri.clone(),
          None => uri::from_path(path),
      };
      let mut owned: Vec<(FileId, PathBuf)> = if all {
          analysis
              .modules
              .iter()
              .zip(&analysis.module_packages)
              .filter(|(_, package)| matches!(package, None | Some(PackageId(0))))
              .map(|(module, _)| module.clone())
              .collect()
      } else {
          analysis.modules.iter().take(1).cloned().collect()
      };
      if let Some(file) = manifest {
          if let Some(name) = analysis.db.get_name(file) {
              owned.push((file, PathBuf::from(name)));
          }
      }
      for (file, path) in &owned {
          out.push(Publish {
              uri: uri_of(path),
              version: open(path).map(|d| d.version),
              diagnostics: convert::diagnostics_for(analysis, *file, entry, manifest, &uri_of),
          });
      }
  }
  ```

In `crates/nova-lsp/src/lib.rs`:
- in the `DidChangeWatchedFiles` handler, `project.holds(p)` becomes
  `self.checker.reaches(project, p)`;
- `affected` becomes:

  ```rust
      /// The projects a change to `path` can affect: its own, and every open
      /// project that reaches it (spec 3.3a §6). A project reaches its
      /// packages' directories; a loose file's analysis reads the modules
      /// beside it, so editing one re-checks the loose files that may import
      /// it.
      fn affected(&self, path: &Path) -> Vec<ProjectKey> {
          let mut out = vec![ProjectKey::of(path)];
          for project in self.workspace.projects() {
              if self.checker.reaches(&project, path) && !out.contains(&project) {
                  out.push(project);
              }
          }
          out
      }
  ```

- [ ] **Step 6: Run the tests to verify they pass**

```bash
cd /d/Projects/nona/nova && P=/c/Users/SAKEER~1/AppData/Local/Temp/gcm/p33 && cargo test --locked -p nova-lsp -p nova-driver > $P/t11-lib.txt 2>&1; echo "lib exit=$?"; cargo test --locked -p nova-cli --test lsp > $P/t11.txt 2>&1; echo "exit=$?"; python -X utf8 $P/count.py $P/t11.txt; grep -hE "FAILED|panicked|^warning|^error" $P/t11-lib.txt $P/t11.txt | head
```

Expected: `lib exit=0`, then `exit=0`, 0 failed, no warnings.
`a_diagnostic_with_no_place_goes_on_the_entrys_first_line` still passes:
E0601 has no label, so it still goes on the entry's first line.

- [ ] **Step 7: Commit**

Write `$P/msg-11.txt`:

```
nova-lsp: a dependency's problems, manifests, and re-checks

Spec docs/superpowers/specs/2026-10-08-phase-3-3a-local-packages-design.md
§6:
- **A dependency's problem** shows on the root's manifest entry through
  which the dependency is reached. Its message says where it really is,
  "(in geom: ../geom/src/lib.nova:1:24)". That covers an error in a
  dependency's file, and an M-code in its manifest.
- **The project's own nova.toml** is published with the graph's
  problems. Only the project's own analysis publishes it.
- **Re-checks across packages.** The checker shares each project's
  package directories with the protocol thread, so an edit or save in an
  open file of a dependency re-checks its dependents, which read the
  buffer through the overlay. Before a project's first check, it holds
  its own directory only.

The driver's `Program::manifest` and `Analysis.manifest` name the root's
nova.toml, even when it is too broken for a graph.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
```

```bash
cd /d/Projects/nona/nova && P=/c/Users/SAKEER~1/AppData/Local/Temp/gcm/p33 && cargo fmt --all && git add crates/nova-lsp crates/nova-driver crates/nova-cli && git commit -q -F $P/msg-11.txt && git log -1 --format=%s
```

Expected: `nova-lsp: a dependency's problems, manifests, and re-checks`.

The task's test command:
`cargo test --locked -p nova-lsp && cargo test --locked -p nova-cli --test lsp`.

---

### Task 12: The 3.3a gate

Spec §8.

**Files:**
- Create: `.github/scripts/packages-gate.sh` (LF)
- Modify: `.github/workflows/ci.yml` (the `install` job)

**Interfaces:**
- Consumes: the installed `nova`, with Tasks 5–9's commands.
- Produces: `packages-gate.sh NOVA WORKDIR`.

- [ ] **Step 1: Write the gate**

`.github/scripts/packages-gate.sh`:

```bash
#!/usr/bin/env bash
# The Phase 3.3a gate (spec
# docs/superpowers/specs/2026-10-08-phase-3-3a-local-packages-design.md §8):
# with the installed nova, a library and a program that each have their
# own utils.nova, the program depending on the library by path. It runs
# and prints from both, and `nova test` passes in each. CI's `install` job
# runs it after gate.sh, on all three systems.
#
# Usage: packages-gate.sh NOVA WORKDIR
#   NOVA     the nova executable to check
#   WORKDIR  where to make the packages; it must not exist yet
set -euo pipefail

if [ $# -ne 2 ]; then
  echo "usage: packages-gate.sh NOVA WORKDIR" >&2
  exit 2
fi
nova=$1
work=$2

mkdir "$work"
cd "$work"

# 1. A library whose lib.nova uses its own utils.nova. The template's
#    tests/geom_test.nova tests greeting(), which stays.
"$nova" new --lib geom
cat > geom/src/utils.nova <<'EOF'
pub fn width() -> Int {
    3
}
EOF
cat > geom/src/lib.nova <<'EOF'
import utils

pub fn greeting() -> String {
    "Hello, Nova!"
}

pub fn area() -> Int {
    width() * width()
}

@test
fn area_is_nine() {
    assert_eq(area(), 9)
}
EOF

# 2. A program with a different utils.nova.
"$nova" new app
cat > app/src/utils.nova <<'EOF'
pub fn label() -> String {
    "app utils"
}
EOF
cat > app/src/main.nova <<'EOF'
import geom
import utils

fn main() {
    println("area ${area()}")
    println(label())
}

@test
fn area_comes_from_geom() {
    assert_eq(area(), 9)
}
EOF

# 3. The dependency, added by path.
cd app
"$nova" add geom --path ../geom

# 4. Both utils modules, in one program.
ran=$("$nova" run)
expected=$'area 9\napp utils'
if [ "$ran" != "$expected" ]; then
  echo "packages-gate: nova run printed '$ran'" >&2
  exit 1
fi

# 5. The tests of each package: the app's one, and the library's own with
#    tests/geom_test.nova's.
"$nova" test | tee test.txt
if ! grep -q '1 passed; 0 failed' test.txt; then
  echo "packages-gate: nova test did not pass the app's test" >&2
  exit 1
fi
cd ../geom
"$nova" test | tee test.txt
if ! grep -q '2 passed; 0 failed' test.txt; then
  echo "packages-gate: nova test did not pass geom's two tests" >&2
  exit 1
fi
if ! grep -q 'geom_test::greeting_is_public ... ok' test.txt; then
  echo "packages-gate: tests/geom_test.nova did not run" >&2
  exit 1
fi
echo "packages-gate: passed"
```

- [ ] **Step 2: Run it against a fresh build**

The gate is a guard: Tasks 5–9 made it pass. A debug `nova` finds the
runtime library beside it, so build both first:

```bash
cd /d/Projects/nona/nova && P=/c/Users/SAKEER~1/AppData/Local/Temp/gcm/p33 && cargo build --locked -p nova-cli -p nova-runtime 2>&1 | tail -1 && W=$(mktemp -d "$P/gate.XXXX") && bash .github/scripts/packages-gate.sh "$PWD/target/debug/nova.exe" "$W/work" > $P/t12.txt 2>&1; echo "exit=$?"; tail -3 $P/t12.txt; git ls-files --eol .github/scripts/packages-gate.sh; python -X utf8 -c "print(open('.github/scripts/packages-gate.sh','rb').read().count(b'\r'))"
```

Expected:
- `exit=0`, and the last line `packages-gate: passed`;
- before it is staged, `git ls-files --eol` prints nothing; the
  carriage-return count is `0`.

The work directory stays under `$P`, for the user.

- [ ] **Step 3: Run it in CI**

In `.github/workflows/ci.yml`, add after the step
`The installed nova makes, runs, builds and tests a project`:

```yaml
      # The Phase 3.3a gate (spec
      # docs/superpowers/specs/2026-10-08-phase-3-3a-local-packages-design.md §8).
      - name: The installed nova builds a program against a path library
        shell: bash
        run: bash .github/scripts/packages-gate.sh "$RUNNER_TEMP/nova/bin/nova" "$RUNNER_TEMP/packages"
```

Check the YAML still parses:

```bash
cd /d/Projects/nona/nova && python -X utf8 -c "import yaml,sys; d=yaml.safe_load(open('.github/workflows/ci.yml')); print([s.get('name') for s in d['jobs']['install']['steps']])"
```

Expected: the list of step names, with the new step after `The installed
nova makes, runs, builds and tests a project`. If `yaml` is not
installed, the same check is `git diff .github/workflows/ci.yml`, read by
eye: the new step's indentation matches its neighbours'.

- [ ] **Step 4: Commit**

Write `$P/msg-12.txt`:

```
ci: the Phase 3.3a gate, a program against a path library

.github/scripts/packages-gate.sh (spec
docs/superpowers/specs/2026-10-08-phase-3-3a-local-packages-design.md §8)
runs in the install job after gate.sh, on all three systems, through the
installed nova:
- `nova new --lib geom`, whose lib.nova uses its own utils.nova;
- `nova new app`, with a different utils.nova;
- `nova add geom --path ../geom`, then `nova run` prints from both;
- `nova test` passes in both, tests/geom_test.nova included.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
```

```bash
cd /d/Projects/nona/nova && P=/c/Users/SAKEER~1/AppData/Local/Temp/gcm/p33 && git add .github/scripts/packages-gate.sh .github/workflows/ci.yml && git ls-files --eol .github/scripts/packages-gate.sh && git commit -q -F $P/msg-12.txt && git log -1 --format=%s
```

Expected: `i/lf    w/lf    attr/text eol=lf` (or the attribute line
`.gitattributes` gives `*.sh`), then
`ci: the Phase 3.3a gate, a program against a path library`.

The task's test command:
`bash .github/scripts/packages-gate.sh "$PWD/target/debug/nova.exe" "$(mktemp -d /c/Users/SAKEER~1/AppData/Local/Temp/gcm/p33/gate.XXXX)/work"`.

---

### Task 13: The records

Spec §9. ADR 0030, dated notes, the project documents, and the
set-difference sweep.

**Files:**
- Create: `docs/adr/0030-package-modules.md`
- Modify (dated notes):
  - `nova-spec/40-TOOLING.md` §4.1, §4.5 and §7;
  - `docs/adr/0003-phase2-module-model.md`;
  - `nova-spec/12-TYPESYSTEM.md`'s code table;
  - `docs/superpowers/specs/2026-10-07-phase-3-0-foundations-design.md` §6.1;
  - `docs/phase-3-plan.md`'s 3.3 entry.
- Modify: `CHANGELOG.md`, `README.md`
- Modify: whatever Step 5's sweep finds

**Interfaces:**
- Consumes: every task's behaviour, and the ledger's rulings.
- Produces: nothing code depends on.

- [ ] **Step 1: ADR 0030**

`docs/adr/0030-package-modules.md`:

```markdown
# ADR 0030: Package modules

## Status

Accepted, 2026-10-08 (Phase 3.3a, branch `phase-3-3a-local-packages`;
spec `docs/superpowers/specs/2026-10-08-phase-3-3a-local-packages-design.md`).

## Context

ADR 0003 gave Nova modules: one file each, named by its stem, all in the
entry's directory, found through one program-wide map from name to module.
Phase 3.3 adds packages. A program depends on libraries by path, and each
library brings its own files. Two packages may both have a `utils.nova`,
and a package's `tests/` may have one too. One map from name to module
cannot hold them.

## Decision

1. **A module is (package, directory, file stem).** The directory is
   `src` or `tests`. A file in no package's `src/` or `tests/` is loose,
   and resolves as before.
2. **The loader decides what an import names,** by package, and gives the
   resolver a table per module. The resolver looks each import up in its
   module's table; a module's name is only a label. A loose program's
   tables are filled by name, which is the old lookup.
3. **What `import x` names**, in a module of package P:
   - P's own `x.nova`, in the same directory;
   - else the library of P's dependency whose import name is `x`. An
     import name is the package's name with each `-` replaced by `_`;
   - a test module also sees the root's dev-dependencies, and the package
     itself;
   - **both** a file and a dependency is E0004. Neither wins;
   - a dev-dependency imported from `src/` is E0001, with a note;
   - a file matches only in its exact case, on every system.
4. **A library's API is its `lib.nova`.** Re-exports and `geom::utils`
   paths stay deferred (ADR 0003, ADR 0025), so a dependent names only
   `lib.nova`'s `pub` items. Library authors put their public types there.
5. **A program's `main` is its entry's.** Every other module's `main` is
   renamed before MIR. An entry without one is E0601, even when a
   dependency declares a `main`.
6. **A library is checked as a module.** MIR runs only when the entry is
   `src/main.nova` or a loose file. Module mode skips MIR's checks: E0011,
   E0013, E0075, E0078 and E0079. So a library can pass `nova check` and
   still fail in a dependent's build.
7. **Coherence stays program-wide,** with no orphan rule. Two packages that
   implement one trait for one type conflict, with today's error, when one
   program uses both.
8. **The language server: one owner per file.**
   - A project's analysis publishes only its own package's modules and its
     own `nova.toml`.
   - A dependency's problem shows on the dependent's manifest entry, with
     where it really is in the message.
   - The checker shares each project's package directories with the
     protocol thread, so an edit in an open dependency re-checks its
     dependents.
   - The client watches files only in its workspace folders. A dependency
     outside them is re-checked on edits and saves in the editor, but not
     when another program changes its files.

## Alternatives

- **Package-qualified names in the resolver's one map**, `geom/utils`.
  Rejected: the resolver would learn packages, dependencies and
  dev-dependencies, which the loader already knows, and the clash rule
  would live in two places.
- **Compiling each package separately,** with an interface per library.
  Rejected for now: Nova has no interface format, and symbols are already
  mangled by `DefId`, so one program-wide compile has no clashes to avoid.
- **A file shadowing a dependency of the same name, or the reverse.**
  Rejected: whichever won, adding a file or a dependency would silently
  change what an existing import means. E0004 makes the user choose.

## Consequences

- Every compile goes through the loader. A loose program resolves exactly
  as before, except that imports match case on Windows and macOS too.
- `nova run src/main.nova` inside a project reads its manifest; any other
  file argument is still loose (the 3.0 spec's §6.1, amended).
- `nova test` runs the root's `src/` tests that the roots reach, and every
  top-level `tests/*.nova`. A dependency's tests never run in its
  dependent's binary.
- A dependency is compiled into each program that uses it; it never gets a
  `target/` of its own.

## References

- `docs/superpowers/specs/2026-10-08-phase-3-3a-local-packages-design.md`
- `docs/superpowers/plans/2026-10-08-phase-3-3a-local-packages.md`
- ADR 0003 (the module model), ADR 0025 (Phase 2's boundary and backlog),
  ADR 0029 (the language server)
- `docs/phase-3-plan.md` §3, decision 3, and §4's 3.3 entry
```

- [ ] **Step 2: Dated notes**

Each note is inserted as its own paragraph, after the line named, with a
blank line before it.

In `nova-spec/40-TOOLING.md`:
- after `M0001–M0006 point at the key or value at fault (spec §5).`:

  ```markdown
  **Amended 2026-10-08 (branch `phase-3-3a-local-packages`):** path
  dependencies resolve
  (`docs/superpowers/specs/2026-10-08-phase-3-3a-local-packages-design.md`
  §3). Each `path` is read relative to its own manifest, and a package is
  identified by its canonical directory. M0005 now covers only a
  version-only entry, and `nova_pm::graph` raises it, not the CLI.
  M0007–M0013 are the graph's errors. A dependency is imported by its name
  with each `-` replaced by `_` (ADR 0030).
  ```
- after the code block of §4.5 that ends with `nova owner add/rm <user> <pkg>`:

  ```markdown
  **Amended 2026-10-08 (branch `phase-3-3a-local-packages`):** 3.3a has
  `nova add <pkg> --path <dir> [--dev]` and `nova remove <pkg> [--dev]`.
  `nova add` writes the path relative to the manifest, keeps the
  manifest's comments and layout, and checks the graph before it writes.
  The version form, `nova update`, `nova publish` and `nova login` are
  3.3b's.
  ```
- after `` - `--bench` runs benchmarks instead ``:

  ```markdown
  **Amended 2026-10-08 (branch `phase-3-3a-local-packages`):** as built,
  `nova test` runs the tests of the `src/` files that the program and the
  library reach, not all of `src/**`, and every top-level `tests/*.nova`,
  sorted by name. A test in `tests/api.nova` is named `api::<function>`,
  and the filter is a positional substring of that name. Subdirectories of
  `tests/` are not read (ADR 0003's deferral). A dependency's tests are
  never run.
  ```

In `docs/adr/0003-phase2-module-model.md`, after its last line,
`  nested module directories, and re-exports.`:

```markdown
**Amended 2026-10-08 (branch `phase-3-3a-local-packages`):** with packages,
a module is (package, directory, file stem), and the loader decides what
each import names (ADR 0030). A file in no package is loose, and resolves
by name as this ADR describes, except that a file's name must match its
import's case exactly.
```

In `nova-spec/12-TYPESYSTEM.md`, after the row `| E0003 | Private item access |`,
add the row:

```markdown
| E0004 | An import names both a module of the package and a dependency (added 2026-10-08, Phase 3.3a, ADR 0030) |
```

In `docs/superpowers/specs/2026-10-07-phase-3-0-foundations-design.md`
§6.1, after the bullet `- **A file argument** means file mode, …`, which
ends `  current directory.`:

```markdown
**Amended 2026-10-08 (branch `phase-3-3a-local-packages`):** a file
argument directly in a package's `src/` or `tests/` is a module of that
package, so `nova run src/main.nova` reads the manifest and sees the
dependencies, and a broken manifest stops it
(`docs/superpowers/specs/2026-10-08-phase-3-3a-local-packages-design.md`
§4.1). Any other file argument reads no manifest, and where `nova build`
writes is unchanged.
```

In `docs/phase-3-plan.md`, after the 3.3 entry's paragraph that ends
`from git rather than crates.io.`:

```markdown
  **Amended 2026-10-08:** 3.3 is two sub-phases, each with its own spec.
  3.3a, "Local packages", has package modules, path dependencies,
  `tests/`, `nova add --path`, `nova remove` and `nova new --lib`
  (`docs/superpowers/specs/2026-10-08-phase-3-3a-local-packages-design.md`,
  branch `phase-3-3a-local-packages`). 3.3b, "The index and publishing",
  has the rest of this entry, and its two-part gate.
```

- [ ] **Step 3: The project documents**

`CHANGELOG.md`, `[Unreleased]`, at the end of `### Added`, after
``   `format_file` would format the file. ``:

```markdown
- **Packages with path dependencies** (ADR 0030).
  - `geom = { path = "../geom" }` in `[dependencies]` makes `import geom`
    reach that library's `src/lib.nova`. A `-` in a name becomes `_` in
    its import.
  - Each package's files are its own: two packages may each have a
    `utils.nova`, and so may one package's `src/` and `tests/`.
  - M0007–M0013 report a bad path, a misnamed or non-library dependency,
    a cycle, two packages with one name, an import-name clash, and a
    package with no target. E0004 reports a file and a dependency with
    one name.
- **`nova add <name> --path <dir> [--dev]`** and **`nova remove <name>
  [--dev]`** edit `nova.toml`, keeping its comments and layout.
- **`nova new --lib`** and **`nova init --lib`** make a library, with a
  test in `tests/`.
- **`nova test` runs `tests/*.nova`.** A test there is named
  `<file>::<function>`.
- **`nova check`** checks the library as well as the program.
- **The language server** follows packages. A dependency's problems show
  on the manifest's entry for it, and a project's `nova.toml` gets its
  own diagnostics.
```

`CHANGELOG.md`, `[Unreleased]`, at the end of `### Changed`, after
`  programs report fewer follow-on errors.`:

```markdown
- An entry without `fn main` is E0601, even when a module it imports
  declares one. That module's `main` used to run.
- An import matches a file's name in its exact case on every system, so
  `import Utils` no longer loads `utils.nova` on Windows or macOS.
- `nova run src/main.nova` inside a project reads its manifest and sees
  its dependencies. Other file arguments are unchanged.
- A declared dependency is no longer M0005 "cannot be used yet": a path
  dependency resolves, and a version-only one is M0005 "a registry
  dependency".
- `nova run` and `nova build` in a library without `src/main.nova` say
  so, and a package with neither target is M0013.
```

`README.md`: after the paragraph of `### Editor support` that ends
`` on your PATH, or the one the `nova.server.path` setting names. ``, add:

```markdown

### Packages

A package is a directory with `nova.toml` and a `src/lib.nova` library, a
`src/main.nova` program, or both. `nova new --lib geom` makes a library.
In another package, `nova add geom --path ../geom` adds it as a
dependency, and `import geom` then reaches its `lib.nova`. `nova test` runs
the tests in `src/` and in `tests/*.nova`, which import the package by its
name. Registry dependencies and `nova publish` come in a later release.
```

- [ ] **Step 4: Commit the records**

Write `$P/msg-13a.txt`:

```
docs: record Phase 3.3a, local packages

- ADR 0030, "Package modules": identity as (package, directory, file);
  the loader's import rules, `-` as `_`, E0004, dev-dependencies from
  tests/ only, exact case; a library's API is its lib.nova; a program's
  main is its entry's; module mode for libraries and what it skips;
  coherence across packages; the language server's one-owner rule and
  its watching limit.
- Dated notes: 40-TOOLING §4.1, §4.5 and §7, ADR 0003, the 3.0 spec's
  §6.1, and the phase plan's 3.3 entry; E0004 in 12-TYPESYSTEM's table.
- CHANGELOG [Unreleased], with three changes in behaviour, and the
  README's packages section.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
```

```bash
cd /d/Projects/nona/nova && P=/c/Users/SAKEER~1/AppData/Local/Temp/gcm/p33 && git add docs/adr/0030-package-modules.md docs/adr/0003-phase2-module-model.md nova-spec docs/superpowers/specs/2026-10-07-phase-3-0-foundations-design.md docs/phase-3-plan.md CHANGELOG.md README.md && git commit -q -F $P/msg-13a.txt && git log -1 --format=%s && git show --stat HEAD | tail -1
```

Expected: `docs: record Phase 3.3a, local packages`, with `8 files changed`.

- [ ] **Step 5: The set-difference sweep**

A grep proves only what it was pointed at. List every living document that
names what this branch changed, minus the files the branch touched:

```bash
cd /d/Projects/nona/nova && P=/c/Users/SAKEER~1/AppData/Local/Temp/gcm/p33 && git diff --name-only ff3c54f HEAD > $P/touched.txt && { for t in "M0005" "cannot be used yet" "dependencies" "nova add" "nova remove" "lib.nova" "tests/" "nova new" "nova init" "nova test" "file argument" "E0601" "import "; do git grep -l -F -- "$t" -- '*.md' '*.yml' '*.toml' '*.sh' '*.json' | sed "s|^|$t: |"; done; for t in "M0005" "cannot be used yet" "file argument" "read_manifest" "unresolved_dependencies" "ProjectKey::entry" "load_program"; do git grep -l -F -- "$t" -- '*.rs' | sed "s|^|$t: |"; done; } | grep -v -E ": (docs/superpowers/|docs/adr/|tools/vscode-nova/node_modules/)" | while IFS= read -r line; do f=${line#*: }; grep -q -x -F -- "$f" $P/touched.txt || echo "$line"; done | sort | tee $P/sweep.txt | wc -l
```

Read every hit in `$P/sweep.txt`, in context. For each, decide whether it
now says something untrue or incomplete about dependencies, module
identity, what an import names, `nova test`'s files, a file argument's
manifest, or which `main` runs.
- If it does, add a dated note there, or fix a non-document such as a
  script or a doc comment, and add the file to the commit below.
- If not, nothing changes.

Ledger each file as `Task 13: sweep <file> -> <note added | unaffected:
why>`. A file listed for a word it uses in another sense is unaffected: an
`import` in an example that resolves as before, or "dependencies" meaning
Rust's.

If the sweep changed anything, write `$P/msg-13b.txt`:

```
docs: notes the packages sweep found

The set-difference sweep over the living documents (spec
docs/superpowers/specs/2026-10-08-phase-3-3a-local-packages-design.md §9):
<one line per file changed, saying what changed>

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
```

and commit the files it lists with `git commit -q -F $P/msg-13b.txt`, then
check `git log -1 --format=%s`.

The task's test command: `cargo test --locked -p nova-cli --test project`.
The records touch no code but the sweep may touch a doc comment; this
confirms the CLI still builds and its project tests pass.

---

### Task 14: Final verification

Before the PR, run here everything CI runs, on the finished branch, plus
the gate by name.

**Files:**
- Create: `$P/pr-body.md` (outside the repository)

**Interfaces:**
- Consumes: the whole branch.
- Produces: the figures for the PR body.

- [ ] **Step 1: The full suite on Windows**

```bash
cd /d/Projects/nona/nova && P=/c/Users/SAKEER~1/AppData/Local/Temp/gcm/p33 && git status --short | wc -l && netstat -ano | grep -E "[:.]3000 .*LISTENING"; cargo build --locked -p nova-runtime 2>&1 | tail -1 && cargo test --locked --workspace --all-features --no-fail-fast > $P/suite-final.txt 2>&1; echo "exit=$?"; python -X utf8 $P/count.py $P/suite-final.txt
```

Expected:
- `0` uncommitted files;
- nothing from `netstat`. If it prints anything, stop and ask the user, as
  the Conventions say;
- `exit=0`, 0 failed;
- passed: `main`'s (`ff3c54f`) 1499 plus this plan's 65 new tests, so
  1564; 9 ignored.

The 65 are Task 1's 3, Task 2's 19, Task 3's 3, Task 4's 11, Task 5's 6,
Task 6's 5, Task 7's 7, Task 8's 3, Task 9's 1, Task 10's 3 and Task 11's
4. If the figure differs, recount the new tests by name from each task's
output, and ledger the difference.

- [ ] **Step 2: What CI's other jobs run**

```bash
cd /d/Projects/nona/nova && P=/c/Users/SAKEER~1/AppData/Local/Temp/gcm/p33 && cargo fmt --all -- --check; echo "fmt exit=$?"; cargo clippy --locked --all-targets --all-features -- -D warnings > $P/clippy.txt 2>&1; echo "clippy exit=$?"; tail -3 $P/clippy.txt
```

Expected: `fmt exit=0` and `clippy exit=0`.
- A clippy finding is fixed in the code, never allowed by attribute.
- The fix is its own commit, `<crate>: what clippy found`.
- Step 1 then runs again.

```bash
cd /d/Projects/nona/nova && P=/c/Users/SAKEER~1/AppData/Local/Temp/gcm/p33 && RUSTUP_TOOLCHAIN=1.78.0 RUSTFLAGS="-D warnings" cargo check --locked --workspace --target-dir target/msrv > $P/msrv.txt 2>&1; echo "msrv exit=$?"; tail -3 $P/msrv.txt; git diff ff3c54f HEAD -- Cargo.lock | grep -E "^[+-] " 
```

Expected: `msrv exit=0`, and the lockfile diff is three added lines:
`+ "nova-lexer",` (in `nova-pm`), `+ "nova-pm",` (in `nova-driver`) and
`+ "toml_edit",` (in `nova-cli`), and no `name = ` line.

- [ ] **Step 3: The full suite on Linux**

```bash
cd /d/Projects/nona/nova && P=/c/Users/SAKEER~1/AppData/Local/Temp/gcm/p33 && docker version 2>&1 | grep -c "^Server:"; bash /c/Users/SAKEER~1/AppData/Local/Temp/gcm/linux/run.sh test --locked --workspace --all-features --no-fail-fast > $P/suite-linux-final.txt 2>&1; echo "exit=$?"; python -X utf8 $P/count.py $P/suite-linux-final.txt
```

Expected:
- `1` (Docker's server is up), and `exit=0`;
- 0 failed;
- passed: `main`'s 1495 plus 65, so 1560; 10 ignored.

Linux is where a case mismatch was always an error, and where `read_dir`
order is hashed, so this run is the check of the exact-case rule and the
sorted `tests/` on a second file system.

- [ ] **Step 4: The gate, item by item**

Spec §8, each with what shows it:

```bash
cd /d/Projects/nona/nova && P=/c/Users/SAKEER~1/AppData/Local/Temp/gcm/p33 && W=$(mktemp -d "$P/gate.XXXX") && cargo build --locked -p nova-cli -p nova-runtime 2>&1 | tail -1 && bash .github/scripts/packages-gate.sh "$PWD/target/debug/nova.exe" "$W/work" > $P/gate-final.txt 2>&1; echo "exit=$?"; tail -1 $P/gate-final.txt; grep -E "an_app_runs_and_builds_against_a_path_library|test_runs_the_library_and_the_tests_directory|tests_are_named_by_file_and_run_in_sorted_order|the_apps_analysis_publishes_nothing_for_its_dependency" $P/suite-final.txt
```

Expected: `exit=0`, `packages-gate: passed`, and four lines ending in `ok`.
Ledger them:
- **a library and a program, each with a `utils.nova`, the program
  depending on the library by path, printing from both:** the gate script,
  and `an_app_runs_and_builds_against_a_path_library`;
- **`nova test` passes in both, `tests/` included:** the gate script, and
  `test_runs_the_library_and_the_tests_directory`;
- **CI's three systems:** the `install` job's new step, read from the PR's
  CI.

- [ ] **Step 5: The mutants**

Spec §7.5: each mutant must fail at least one named test. Run each only on
committed work.
1. Make the edit.
2. Run its test command and read the named test's line: it must say
   `FAILED`. A mutant that aborts the build reads as nothing failed, so
   check the name.
3. Restore the file with `git checkout -- <file>`, and confirm
   `git status --short` is empty before the next one.

In the table, `\|` stands for `|`.

| # | Rule | File | Edit | Test that must fail |
|---|---|---|---|---|
| 1 | the clash rule | `crates/nova-driver/src/program.rs` | in `Loader::find`, the arm `(Some((_, path)), Some(edge)) => Found::Reported(clash(name, &path, &edge, at, db)),` → `(Some((place, path)), Some(_)) => Found::Module(place, path),` | `cargo test --locked -p nova-driver --test packages` → `a_file_and_a_dependency_with_one_name_is_e0004` |
| 2 | the dev-dependency rule | `crates/nova-driver/src/program.rs` | in `visible_edges`, move `edges.extend(p.dev_dependencies.iter().cloned());` above `if tests {` | `cargo test --locked -p nova-driver --test packages` → `a_dev_dependency_imported_from_src_is_e0001_with_a_note` |
| 3 | identity by canonical directory | `crates/nova-pm/src/graph.rs` | in `Builder::edge`, `.position(\|p\| p.canonical == canonical)` → `.position(\|p\| p.dir == dir)` | `cargo test --locked -p nova-pm --test graph` → `spellings_of_one_directory_are_one_package` |
| 4 | identity's directory | `crates/nova-driver/src/program.rs` | in `Loader::find`, the found file's `Place::Package { package, tests, stem: name.to_string() }` → `Place::Package { package, tests: false, stem: name.to_string() }` | `cargo test --locked -p nova-driver --test packages` → `src_utils_and_tests_utils_are_two_modules` |
| 5 | `main` from the entry | `crates/nova-driver/src/analyze.rs` | in `analyze_program`, delete `crate::keep_the_entry_main(&mut module, &resolved.definitions);` | `cargo test --locked -p nova-driver --test packages` → `an_entry_without_main_is_e0601_even_when_a_dependency_has_one` |
| 6 | one owner per file | `crates/nova-lsp/src/checker.rs` | in `publish_own`, `.filter(\|(_, package)\| matches!(package, None \| Some(PackageId(0))))` → `.filter(\|_\| true)` | `cargo test --locked -p nova-cli --test lsp` → `the_apps_analysis_publishes_nothing_for_its_dependency` |

Mutant 5 deletes the call in `analyze_program`; `FrontendContext::check`'s
call is the CLI's, which `the_entry_main_runs_when_a_dependency_also_has_one`
guards.

Ledger each as `Task 14: mutant <n> -> <test> FAILED (restored)`.

- [ ] **Step 6: Draft the PR body**

Write `$P/pr-body.md` with the Write tool, filling the `<…>` from the
ledger and from Steps 1 to 4:

```markdown
## Phase 3.3a, "Local packages"

A Nova package can depend on another by path. `import geom` reaches that
library, each package's files are its own, `nova test` runs `tests/`, and
`nova add --path` and `nova remove` edit the manifest. The language server
follows.

- Spec: `docs/superpowers/specs/2026-10-08-phase-3-3a-local-packages-design.md`
- Plan: `docs/superpowers/plans/2026-10-08-phase-3-3a-local-packages.md`
- ADR 0030, "Package modules"

### What changed

- **`nova-pm`: the package graph.** Path dependencies, each relative to
  its own manifest; a package identified by its canonical directory;
  import names with `-` as `_`; M0005 moved here, and M0007–M0013.
- **The resolver** looks each import up in its module's table, not in one
  map of names. A loose program's tables are filled by name, as before.
- **`nova-driver`: `Program` and the loader.**
  - Module identity is (package, directory, file).
  - The loader decides what each import names; E0004 for a file and a
    dependency with one name; dev-dependencies from `tests/` only; exact
    case.
  - A dependency's tests are stripped, a `tests/` file's tests are named
    after it, and a program's `main` is its entry's.
  - A library is checked as a module.
- **`nova-cli`.**
  - Each command builds a `Program`: `nova check` takes the library too,
    `nova test` takes `tests/`, and `nova run` refuses a library.
  - `nova add --path`, `nova remove`, `nova new --lib`, `nova init --lib`.
  - `nova fmt` formats `tests/` and skips nested packages.
- **`nova-lsp`.** Projects are packages, each publishing only its own
  modules and its own `nova.toml`. A dependency's problems show on the
  manifest's entry for it, and an edit in a dependency re-checks its
  dependents.
- **CI.** `.github/scripts/packages-gate.sh`, in the `install` job.
- **Records.** ADR 0030; dated notes in 40-TOOLING, ADR 0003,
  12-TYPESYSTEM, the 3.0 spec and the phase plan; the CHANGELOG and the
  README.

### Changes in behaviour

- An entry without `fn main` is E0601, even when an imported module has
  one.
- Imports match a file's case exactly on every system.
- `nova run src/main.nova` inside a project reads its manifest.

### The gate (spec §8)

`packages-gate.sh` passes locally (<result>) and in CI's `install` job on
all three systems. `an_app_runs_and_builds_against_a_path_library` and
`test_runs_the_library_and_the_tests_directory` cover the same ground in
the suite.

### Tests

| | Windows (local) | Linux (container) |
|---|---|---|
| `ff3c54f` | 1499 passed, 9 ignored | 1495 passed, 10 ignored |
| this branch | <Step 1's figures> | <Step 3's figures> |

Each mutant of spec §7.5 fails its named test. Clippy, rustfmt and the MSRV
check (Rust 1.78) pass. `Cargo.lock` gains no package.

### Decisions to review

The plan's "Decisions: where this plan settles what the spec leaves open",
items 1 to 19, and the rulings made while executing it:

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

1. **The final review.** A fresh reviewer, on the most capable model, reads
   the whole branch, from `git merge-base main HEAD` to `HEAD`. It gets the
   spec, this plan, its Review Focus section verbatim, and the ledger's
   `Ruling:` lines. It is read-only, and runs no cargo and no scripts.
2. **The fix pass.** Each Critical or Important finding gets a test that
   fails first, then the fix, then a green suite. Minor findings are
   deferred to the PR body.
3. **Push and open the PR** against `main`, with `$P/pr-body.md` as its
   body. Then read the PR's CI once when the user returns; never poll it.
4. **Merge only on the user's word,** by rebase, then verify that `main`'s
   tree is the PR head's.
