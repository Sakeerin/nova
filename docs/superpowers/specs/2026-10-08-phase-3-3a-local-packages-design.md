# Phase 3.3a, "Local packages": design

- **Date:** 2026-10-08.
- **Status:**
  - The design was approved in conversation on 2026-10-08, in five sections
    (§11 decisions 1–6).
  - A fact-check against the code followed (26 findings). This spec includes
    its corrections, and §11 decisions 7–27 were taken in writing and
    correcting it.
  - This written spec awaits the user's review.
- **Branch:** `phase-3-3a-local-packages`, cut from `main` at `ff3c54f`.
- **Inputs:**
  - `docs/phase-3-plan.md` §2 item 4, §3 decisions 2, 3 and 5, §4's 3.3
    entry, §5 and §6 risk 4;
  - `nova-spec/40-TOOLING.md` §4.1, §4.5 and §7;
  - ADR 0003 (the module model), ADR 0025 (its backlog) and ADR 0026;
  - the 3.0 spec's §5 and §6 (`nova.toml`, projects);
  - the 3.2 spec's §6.2 and §6.3 (which analysis owns a file, and where a
    diagnostic goes).

## 1. What 3.3a delivers

The phase plan's 3.3 is split in two (§11, decision 1). This spec is the
first half; 3.3b, "The index and publishing", follows once it merges.

- **Packages.** A package is a directory with `nova.toml` and a library
  (`src/lib.nova`), a program (`src/main.nova`), or both.
- **Path dependencies.** `geom = { path = "../geom" }` resolves, and
  `import geom` reaches that library from the package's `src/` and `tests/`
  files.
- **Module identity is (package, directory, file).** Two packages may each
  have a `utils.nova`, and so may a package's `src/` and `tests/`.
- **`tests/`.** `nova test` also runs the top-level `tests/*.nova` files.
  They import the package by name, its dependencies and dev-dependencies,
  and each other.
- **Commands:**
  - `nova add <name> --path <dir> [--dev]` and `nova remove <name> [--dev]`;
  - `nova new --lib` and `nova init --lib`;
  - `nova fmt` also formats `tests/`.
- **A program's `main` is its entry's.** An entry without `fn main` is E0601,
  even when an imported module declares one. Today that program silently
  runs the other module's `main` (§2).
- **The language server follows:**
  - each file, `nova.toml` included, has one owner: its own package's
    analysis;
  - an error inside a dependency shows on the dependent's manifest entry;
  - an edit in a dependency re-checks its dependents.

Registry dependencies, the resolver, fetching, `nova.lock`, `nova update`
and publishing are 3.3b's (§12).

## 2. The starting point (read on 2026-10-08)

### The driver (`crates/nova-driver/`)

- `analyze::load_program` (`analyze.rs:96`) loads the entry and every
  module it imports.
  - It names each module by its file stem.
  - It finds `import m` as `m.nova` in the entry's directory, the only
    directory it reads (`:101`, `:143`), and loads each name once (`:113`).
- `strip_test_functions` (`lib.rs:652`) removes every `@test` function from
  every module when not testing (`lib.rs:541-543`, `analyze.rs:183-185`).
- These entry points each take one source path:
  - `check_file`, `compile_file`, `build_file`, `build_file_release` and
    `build_test_binary` (`lib.rs:44-198`);
  - `run_file` (`lib.rs:438`);
  - `analyze` (`analyze.rs:160`).
- `check_file` always lowers to MIR. Module mode, which skips MIR, exists
  only as `analyze`'s `module_only` option. MIR is where E0011, E0013,
  E0075, E0078 and E0079 are found (`crates/nova-mir/src/mono.rs`,
  `lower.rs`).
- **How `main` is chosen.** `nova_mir::lower_module` takes the first
  function named `main` (`mono.rs:19`). The type checker emits functions in
  `DefId` order across every module (`crates/nova-typeck/src/check.rs:180-186`),
  and both code generators find `main` by name. So today an entry with no
  `fn main` runs the `main` of a module it imports. `build_test_binary`
  already renames a user `main` before adding its own (`lib.rs:175-215`).
- `nova-driver` does not depend on `nova-pm`.

### The resolver (`crates/nova-resolver/src/lib.rs`)

- `resolve_program` (`:1503`) builds one map from module name to module
  index over every module, std included (`by_name`, `:1619-1623`).
- `resolve_import` (`:2274`) looks an import's name up in that map. Its
  E0001 messages:
  - "cannot find module `m`";
  - "module `m` imports itself";
  - "`x` is not a public item of module `m`".
  
  A multi-segment path (`a::b`) is E0900, unsupported (`:2286-2292`).
- Messages name a module as the import wrote it. `ModuleScope.name` is
  stored but never read (`Definitions::module_name`, `:1265`, has no
  caller).
- `ModuleSource` is built by struct literal at:
  - `lib.rs:1458` (`resolve`), `:1549` and `:1558` (inside
    `resolve_program`), and `:2583`/`:2587` (`resolve_two`, which ten
    resolver tests use to resolve `import lib` by name);
  - `crates/nova-driver/src/analyze.rs:199-205` and `lib.rs:567-573`;
  - `crates/nova-mir/tests/lower_tests.rs:1181` and `:1269`;
  - `crates/nova-typeck/src/check.rs:16606`.
- `impl` blocks are collected program-wide (ADR 0003), and conflicting
  implementations are an error (`crates/nova-typeck/src/check.rs:1550-1575`).
  There is no orphan rule.
- Symbols are mangled with the definition's `DefId` (`nova_mir::mangle`,
  `crates/nova-mir/src/lib.rs:1093`). So two functions of one name in two
  modules never clash as symbols. `main` alone is chosen by name.

### The CLI and `nova-pm`

- `nova_pm::parse` (`crates/nova-pm/src/manifest.rs`) reads `[package]`,
  `[dependencies]` and `[dev-dependencies]`.
  - A dependency is a version requirement or a table with `version` or
    `path`. Both together is M0003 (`:345-353`).
  - It emits M0001–M0004 and M0006.
  - It accepts any table-like `[dependencies]`, an inline table or dotted
    keys included (`:273`).
- `project::mode` (`crates/nova-cli/src/project.rs`) has three outcomes:
  - a file argument means file mode, and no manifest is read;
  - with no manifest above the current directory, file mode on
    `src/main.nova` (`:47-49`);
  - otherwise the nearest `nova.toml` makes a project, which must have
    `src/main.nova`.
  
  M0005, one per declared dependency, is the CLI's
  (`unresolved_dependencies`, `:107-124`), "the one place 3.3 removes".
- Tests pin three of these behaviours (`crates/nova-cli/tests/project.rs`):
  - "a file argument never reads the manifest" (`:198-217`);
  - "project `demo` has no src/main.nova" (`:190-196`);
  - M0005's message contains the dependency's name (`:227`).
- `nova_pm::check_name` allows ASCII letters, digits, `-` and `_`. A name
  starts with a letter, is at most 64 characters, and is not a Windows
  device name (`crates/nova-pm/src/name.rs`). It accepts every keyword in
  `nova_lexer::KEYWORDS`.
- `nova fmt` with no paths formats the project's `src/`, recursively
  (`cmd/fmt.rs:110-128`).
- `nova new` and `nova init` write a program template. `template::files`
  returns a fixed four files, and `new` and `init` create only `src/`
  (`crates/nova-cli/src/template.rs`, `cmd/new.rs`).
- `nova test` builds one test binary from the project's entry
  (`cmd/test.rs:165-166`). Its filter is a positional argument, matched as a
  substring of each test's name (`:21-25`, `:236-241`).
- `toml_edit` is locked at 0.22.27, with its default features. Its
  `DocumentMut` keeps comments and layout through an edit.

### The language server (`crates/nova-lsp/`)

- `ProjectKey::of` assigns any file under a directory with `nova.toml` to
  that project (`workspace.rs:190-196`). `ProjectKey::entry` is
  `src/main.nova` (`:199-204`).
- `ProjectKey::holds` is a stateless method: "under the project's
  directory", or for a loose file "beside it" (`workspace.rs:209-217`).
  `Server::affected` (`lib.rs:261-269`) and the watched-files handler
  (`lib.rs:185-190`) call it, on the protocol thread.
- An `Analysis` exists only on the checker thread, which publishes straight
  to the connection (`lib.rs:59-74`, `checker.rs:83-121`). Nothing comes back
  to `Server`.
- `Analysis.modules` holds (FileId, path) per module (`analyze.rs:68-70`).
- A project's analysis publishes for every module its entry reaches
  (`checker.rs:178-183`). `convert::diagnostics_for` routes each diagnostic
  to the file of its primary label among those modules (`convert.rs:41`), and
  drops related labels outside them.
- The file watcher is the client's, registered for `**/*.nova` and
  `**/nova.toml` (`lib.rs:300-307`). VS Code watches only its workspace
  folders.
- `real_path` is in `nova-lsp` (`workspace.rs:245-251`).
- Completion analyses an unreached file with `analyze(path)` alone
  (`completion.rs:50-63`).

### CI

- `.github/scripts/gate.sh` is the 3.0 gate. It makes, runs and builds a
  new project with the installed `nova`, and with `--test` tests it.
- CI's `install` job runs it with `--test` on all three systems
  (`ci.yml:223`).
- release.yml's smoke test runs it without `--test`, on its smoke targets
  (`release.yml:86`).

### The file system

`std::fs::read_dir` returns entries in no defined order (alphabetical on
NTFS, hashed on ext4). Windows and macOS file systems are case-insensitive
by default.

## 3. Packages and the graph (`nova-pm`)

### 3.1 What a package is

- A package is a directory with `nova.toml`, and one or both of:
  - `src/lib.nova`, a library;
  - `src/main.nova`, a program.
- A package with neither is **M0013**, on its `[package]` table.
  - `nova run`, `build`, `check` and `test` render it and stop.
  - `nova fmt`, `add` and `remove` still work, since they compile nothing.
  - The server publishes it under that `nova.toml`.

### 3.2 Path dependencies

- A `[dependencies]` or `[dev-dependencies]` entry with `path` resolves.
- The path is read relative to the directory of the manifest that declares
  it.
- The target directory must hold:
  - a `nova.toml` whose `package.name` equals the entry's key;
  - a `src/lib.nova`. A dependency is always a library.
- An entry with `version` and no `path` stays M0005, which moves from the
  CLI into `nova_pm::graph`.
  - Its message keeps the name: "dependency `http` is a registry
    dependency; registry dependencies arrive with the package index".
  - Its note says to use `path` for a local package.
- 3.0's rule that an entry has `version` or `path`, never both, stands
  (M0003). 3.3b decides how a path dependency is published.

### 3.3 The graph

- **Following.** Dependencies are followed transitively, each `path`
  relative to its own manifest.
- **Identity.** A package is identified by its canonical directory:
  `std::fs::canonicalize`, with Windows' `\\?\` prefix stripped.
  - `real_path`, which does this, moves from `nova-lsp` into `nova-pm`, and
    the server uses it from there.
  - One directory reached by two routes, a diamond, is one package.
- **One package per name per build.** Two different directories whose
  manifests give the same `package.name` are M0011. The root package counts.
- **Cycles** are M0010.
  - Cycles are found over `[dependencies]` edges.
  - A path that names the declaring package's own directory is a cycle of
    one.
  - A dev-dependency never makes a cycle. Only the root's dev-dependencies
    are read. One that depends back on the root reaches the root's own
    directory, which is the same package, and a source module never sees
    dev-dependencies.
- **Dev-dependencies.** Only the root package's are read. A dependency's
  `[dev-dependencies]` are ignored, as in Cargo.
- **Warnings.** Every manifest's warnings (M0006) are reported, each
  against its own `nova.toml`.

### 3.4 Import names

- A dependency's import name is its key with each `-` replaced by `_`.
  `json-api` is imported as `import json_api` (§11, decision 2).
- A package's own import name is made the same way from its
  `package.name`. Its test modules see it (§4.3).
- **M0012**, for any of:
  - two entries of one package whose import names coincide, where
    `[dependencies]` and `[dev-dependencies]` count together, since a test
    module sees both;
  - the same key in both tables;
  - for the root package, a dependency or dev-dependency whose import name
    is the root's own;
  - an import name that is one of `nova_lexer::KEYWORDS`, the root's own
    included when it has a library.
- `nova new --lib` and `nova init --lib` refuse a name whose import name is
  a keyword, before writing anything.

### 3.5 Diagnostics

| Code | Error | Where it points |
|---|---|---|
| M0005 | a registry (version-only) dependency | the entry |
| M0007 | the path is not a directory holding a `nova.toml` | the entry's `path` |
| M0008 | the dependency's `package.name` is not the entry's key | the entry's key |
| M0009 | the dependency has no `src/lib.nova` | the entry's `path` |
| M0010 | a dependency cycle, listed by name and manifest | the entry that closes the cycle |
| M0011 | two packages in one build with the same name, naming both directories | the second entry that reaches the name |
| M0012 | an import-name clash, a repeated key, or a keyword | the later entry |
| M0013 | a package with neither `src/lib.nova` nor `src/main.nova` | `[package]` |

Each points into the manifest that declares it, as M0001–M0006 do (3.0
spec §5).

### 3.6 Interface

- `nova_pm::graph(root: &Path, db: &mut FileDb) -> (Option<Graph>, Vec<Diagnostic>)`
  reads every manifest from disk into `db`, so each diagnostic's label
  renders.
- A `Graph` holds packages by `PackageId`. Each package has:
  - its name and import name;
  - its canonical directory and parsed manifest;
  - whether it has `lib.nova` and `main.nova`;
  - its dependencies and dev-dependencies, as (import name, `PackageId`),
    each with its entry's span.
- `graph` is partial on error: the packages and edges that resolved are
  kept, with the diagnostics.
  - The CLI stops on any error, as it does for manifests today.
  - The server goes on with what resolved (§6).
- `nova-pm` gains a dependency on `nova-lexer`, for `KEYWORDS`. There is no
  cycle: `nova-lexer` depends only on `nova-diagnostics`, `logos` and
  `thiserror`.

## 4. Modules and imports

### 4.1 Which package a file belongs to

- A file directly in a package's `src/` is a **source module** of that
  package.
- A file directly in its `tests/` is a **test module** of that package.
- **Any other file is loose,** as today. Its imports are files beside it, it
  has no dependencies, and its directory's `nova.toml` is never read, even
  inside a project. This keeps 3.0's pinned test (`project.rs:198-217`).
- **A file argument directly in `src/` or `tests/`** is a module of its
  package, so `nova run src/main.nova` sees the project's dependencies
  (§11, decision 7).
  - This supersedes the 3.0 spec's §6.1 for such files. A broken manifest
    now stops `nova run src/main.nova`.
  - Where `nova build` writes stays as 3.0 decided.

### 4.2 Module identity

- A module is (package, directory, file stem), where the directory is
  `src` or `tests`.
- So two packages' `utils` are two modules, and so are one package's
  `src/utils.nova` and `tests/utils.nova` (§11, decision 11).
- A loose program is one package-less set of files, as today. The one
  change is that imports match a file's case exactly (§4.3).

### 4.3 What an import names

In a module of package P, `import x` (or `import x::{…}`):

1. **A file of the same package and directory.** For a source module, P's
   `src/x.nova`; for a test module, P's `tests/x.nova`.
2. **A dependency.** If `x` is the import name of one of P's dependencies,
   that dependency's `src/lib.nova`.
   - A test module of the root package also sees the root's
     dev-dependencies, and the root package itself, under its own import
     name.
   - A source module never sees dev-dependencies.
3. **Both** a file and a dependency: **E0004**, "`x` is both a module of
   this package and a dependency" (§11, decision 3).
   - Its primary label is the import.
   - Its message names the file, and the manifest entry with its path and
     line, so it reads in full where a related label cannot show (§6).
4. **A dev-dependency imported from `src/`:** E0001, "cannot find module
   `x`", with the note "`x` is a dev-dependency, which only `tests/` files
   can import" (§11, decision 4).
5. **The package itself imported from a test module of a package with no
   library:** E0001, with the note "`app` has no src/lib.nova".
6. **Neither:** today's E0001, "cannot find module `x`".

Further rules:
- **Case.** A file matches only if its name is exactly `x.nova`, compared
  against the directory listing on every system (§11, decision 24). So
  `import Utils` does not load `utils.nova` on Windows or macOS, as it
  already does not on Linux.
- **Inside a dependency.** Its source modules import each other in its own
  `src/`, so `lib.nova` in `geom` reaches `geom`'s `utils.nova`, not the
  app's.
- **Program and library.** A program reaches its own library by file name,
  `import lib`.

### 4.4 The loader and `Program`

- The driver's entry points take a description of what to load, not one
  path:

  ```rust
  pub struct Program {
      /// Every manifest the graph read, and later every source file.
      pub db: FileDb,
      pub graph: Option<nova_pm::Graph>,
      /// The entry first.
      pub roots: Vec<PathBuf>,
      /// The graph's diagnostics.
      pub diagnostics: Vec<Diagnostic>,
  }
  ```

- **Building one:**
  - `Program::for_file(path)` finds the package from the file's location
    (§4.1), and for a loose file has no graph.
  - `Program::for_package(root, Roots)` builds the roots of a command
    (below).
  - Each existing entry point becomes a wrapper over its `Program` form, so
    callers that pass a path keep working.
- **Roots per command.** The roots are, in this order: the program, the
  library, then `tests/*.nova` sorted by file name. A command takes the
  ones it needs:

  | | program | library | `tests/*.nova` |
  |---|---|---|---|
  | `nova run`, `nova build` | yes | | |
  | `nova check` | yes | yes | |
  | `nova test` | yes | yes | yes |
  | the server | yes | yes | yes |

  The loader walks breadth first from the roots in that order. So test order
  and module order are the same on every system (§11, decision 22).
- **MIR runs when the roots include a program,** or for a loose file, as
  today. Otherwise the program is checked in module mode, which is new for
  the CLI.
  - Module mode skips MIR's checks (E0011, E0013, E0075, E0078, E0079).
  - So a library can pass `nova check` and still fail in a dependent's build.
  - ADR 0030 records this (§11, decision 23).
- **The import table.** For each module, the loader records a table from
  each import's first segment to the module it names, or to the diagnostic
  of §4.3.
- **`nova-driver` gains a dependency on `nova-pm`.** `Cargo.lock` gains no
  package: `nova-driver`'s entry gains a `nova-pm` line, and `nova-pm`'s
  gains `nova-lexer`.

### 4.5 The resolver

- `ModuleSource` gains the import table, and `resolve_import` reads it in
  place of `by_name`, which goes (§11, decision 19).
- `nova_resolver::name_imports(&[ModuleSource])` fills each table by module
  name, which is today's lookup. It serves:
  - loose programs, which therefore resolve exactly as now;
  - the resolver's own tests (`resolve`, `resolve_two`);
  - the other struct-literal sites of §2.
  
  Std modules get empty tables; std's sources contain no `import`.
- Everything after the lookup stays as it is:
  - globs and `import x::{a, b}`;
  - E0002 conflicts;
  - `pub` visibility;
  - std's glob into every module;
  - the "imports itself" check.
- `ModuleSource.name` becomes a display label:
  - a root source file's stem, `utils`;
  - a root test file, `tests/utils`;
  - a dependency's library, by import name, `geom`;
  - a dependency's other file, `geom/utils`.
  
  Today's messages already print the import as written.

### 4.6 Tests and `main`

- **Which tests are kept.** `strip_test_functions` keeps `@test` functions
  only in the root package's modules, and only under `nova test` and in the
  server. Dependencies' tests are always stripped.
- **Which tests run.** Those in modules the roots reach, as today.
  - A `src/` file nothing reaches contributes no tests.
  - This differs from `40-TOOLING.md` §7's `src/**`, which a dated note
    records (§9).
- **Names.** A test in a test module is named `<file stem>::<function>`, for
  example `api::parses_users`. A test in `src/` keeps its bare name.
- **`main` is the entry's** (§11, decision 12):
  - The driver finds the program's `main` in the entry module, through
    `Definitions::resolve_value(ModuleId(0), "main")`.
  - If the entry has none, that is E0601, even when another module declares
    a `main`.
  - Every other module's function named `main` is renamed before MIR, as
    `build_test_binary` renames a user `main` today. So `mono.rs`'s search
    by name finds only the entry's.
  - This is new work, not a property of today's code (§2).

### 4.7 A library's API is its `lib.nova`

- With no re-exports and no `geom::utils` paths (ADR 0003's deferrals, on
  ADR 0025's backlog), a dependent can name only the `pub` items of the
  dependency's `lib.nova`.
- A type declared in another file of the library can still flow through
  `lib.nova`'s functions, but the dependent cannot write its name.
- Library authors put their public types in `lib.nova`. Re-exports stay
  deferred (§11, decision 6).

### 4.8 Coherence across packages

- `impl` blocks stay program-wide, with no orphan rule.
- Two packages that implement the same trait for the same type therefore
  conflict when one program uses both, with today's error.
- ADR 0030 records this (§11, decision 26).

## 5. Commands

### 5.1 Which commands work on which package

- **`nova run` and `nova build`** need `src/main.nova`. In a library-only
  package they stop with "`geom` is a library: it has no src/main.nova;
  `nova check` and `nova test` work on it". This replaces the message
  `project.rs:190-196` pins.
- **`nova check`** checks the program and the library, whichever exist, in
  one analysis, so a module both reach is checked once.
  - MIR runs when there is a program.
  - `tests/` is not checked, as today's `nova check` strips tests (§11,
    decision 23).
- **Dependencies** are compiled into the program, as every module is today.
  They never get a `target/` of their own.
- **Manifest errors** stop every command before compiling, as today.

### 5.2 `nova test` and `tests/`

- One test binary holds:
  - the tests of every `src/` module the program and library reach;
  - every top-level `tests/*.nova`.
  
  Subdirectories of `tests/` are not read, as ADR 0003 defers nested
  directories.
- A test module may import:
  - the package itself, by its import name;
  - the root's dependencies and dev-dependencies;
  - other `tests/` files, by name.
  
  A `tests/` file with no `@test` is a helper module.
- Each test still runs in its own process.
- The filter argument is a substring of the full name, so `api::` selects
  one file's tests.

### 5.3 `nova add` and `nova remove`

`nova add <name> --path <dir> [--dev]`:
- Refuses a manifest that has errors, rendering them first.
- Edits `[dependencies]`, or with `--dev` `[dev-dependencies]`:
  - through `toml_edit`'s `DocumentMut`, so the manifest keeps its
    comments, order and layout;
  - whether the table is a table, an inline table or dotted keys;
  - creating it if it is missing;
  - refusing any other kind of value.
- Writes `<name> = { path = "<dir>" }`.
  - `<dir>` is made relative to the manifest's directory, with `/`
    separators (§11, decision 8).
  - Where no relative path exists, as on another Windows drive, it writes
    the absolute path.
- Checks the new entry before writing, by building the graph as it would be
  with it. Any graph error refuses, and nothing is written. That covers:
  - M0007–M0013;
  - a dependency on the package itself;
  - an entry that already exists in either table.

`nova remove <name> [--dev]`:
- Deletes the entry, and fails if it is not there.
- Comment lines directly above the entry belong to it in `toml_edit`, and
  go with it. The rest of the file is kept byte for byte.

The version form (`nova add json@^1.2`) and `nova update` are 3.3b's.

### 5.4 `nova fmt`

- With no paths, in a project, it formats `src/` and `tests/`, recursively
  as now.
- It skips any directory below them that holds a `nova.toml`, so a path
  dependency kept inside `src/` or `tests/` is not formatted.
- A missing `tests/` is not an error.

### 5.5 `nova new --lib` and `nova init --lib`

- They write:
  - `nova.toml` and `.gitignore`, as for a program;
  - `src/lib.nova`, with one `pub` function and one `@test`;
  - `tests/<import name>_test.nova`, which imports the package and tests
    that function. The `_test` suffix keeps its stem from ever equalling
    the import name, which would be E0004 (§4.3);
  - a README whose line says `nova test` runs the library's tests and
    `tests/`.
- They create `tests/` as well as `src/`. `template::files` returns each
  kind's files.
- `nova init` and `nova init --lib` write only their template's missing
  files, and never overwrite.
  - `nova init --lib` beside an existing `src/main.nova` adds the library
    and `tests/`, making a package with both.
  - `nova init` beside an existing `src/lib.nova` adds `src/main.nova`.
- The program template is unchanged.

## 6. The language server

- **One owner per file.**
  - A project's analysis publishes only for its own package's modules and
    its own `nova.toml`.
  - A dependency's files and manifest belong to that dependency's own
    project, and are published only when one of its files is open there.
  - So two analyses never publish for the same file.
- **What a project's analysis covers.**
  - Its roots are §4.4's: the program, the library and every `tests/*.nova`,
    in test mode. When there is no program, it uses module mode.
  - A `tests/` file is reached, so it is owned by its project's analysis.
  - A `src/` file nothing reaches is still checked on its own, as in 3.2,
    with its package's dependencies.
  - A file in no `src/` or `tests/` is loose (§4.1): `ProjectKey::of`
    returns `Loose` for it.
  - `ProjectKey::entry` becomes the program, else the library.
- **A problem inside a dependency** found by a dependent's analysis is
  shown on the dependent's manifest entry through which the dependency is
  reached (§11, decision 14).
  - That covers an error in a dependency's `.nova` file, and an M-code in a
    dependency's manifest.
  - The message gains where it really is, for example
    "(in geom: ../geom/src/lib.nova:3:5)".
  - So the user sees that the dependency is broken without the dependent
    publishing for the dependency's files. This extends 3.2's §6.3
    fallback, which puts std's errors on the entry's first line.
  - `Analysis.modules` gains each module's package, so `publish_own` and
    `diagnostics_for` can tell own modules from a dependency's.
- **Re-checks across packages** (§11, decision 15).
  - The checker shares each project's package directories with the
    protocol thread, through a map like `Checker.newest`.
  - `holds` and `affected` read it. Before a project's first check
    finishes, it holds its own directory only.
  - So an edit or save in an open file of `geom` re-checks the app, reading
    `geom`'s buffer through the overlay.
  - A change on disk is seen through the client's watcher, which VS Code
    runs only in its workspace folders. A dependency outside them is
    re-checked on edits and saves in the editor, but not when another
    program changes its files. ADR 0030 records the limit.
  - A changed `nova.toml` re-checks every open project, as in 3.2.
- **Manifest errors.** The graph's diagnostics for a project's own
  `nova.toml` are published under its URI. The analysis goes on with the part
  of the graph that resolved, and an import of a dependency that failed
  gets the ordinary E0001.
- **Completion** builds its analysis with `Program::for_file`, so an
  unreached `tests/` file sees its package. `names_in_scope` already lists
  what `import geom` binds.

## 7. Testing

### 7.1 `nova-pm`'s graph

- A diamond is one package.
- One test for each of M0007–M0013, each pointing where §3.5 says.
- A self-dependency is M0010.
- A dev-dependency that depends back on the root is no cycle.
- A dependency's dev-dependencies are not read.
- Paths are relative to each manifest.
- A version-only entry is M0005, with the dependency's name in the message.
- M0012's cases:
  - two entries with one import name, across both tables;
  - the same key in both tables;
  - a dependency named like the root;
  - a keyword.

### 7.2 The loader and the resolver

- Two packages, each with a `utils.nova`, both used in one program, and
  their functions called.
- One package's `src/utils.nova` and `tests/utils.nova`, both used.
- E0004 for a file/dependency clash, its message naming both.
- E0001 with its note for a dev-dependency imported from `src/`.
- `import Utils` beside `utils.nova` is E0001 on every system.
- `tests/` files importing the package, a dependency, a dev-dependency, and
  each other.
- A dependency's `@test` functions stripped, and the root's kept, under
  `nova test`; tests named `<stem>::<fn>`, in sorted file order.
- An app with no `main` whose dependency declares one: E0601.
- A library-only root checked in module mode.
- A file argument in `src/` seeing its dependencies.
- A loose file inside a project, beside a broken `nova.toml`, still running.

### 7.3 The CLI, end to end through `nova`

- `nova new --lib` and `nova init --lib`, then `nova test` passing in each.
- `nova init --lib` beside a `src/main.nova`.
- `nova new --lib match` refused.
- `nova add --path` and `nova remove`, with the manifest kept byte for byte
  outside the edited entry and its comment lines.
- `nova add` refusing a self-dependency, a cycle and an existing entry,
  without writing.
- An app that builds and runs against a path library.
- `nova test` running `src/` and `tests/` tests, and the filter `api::`.
- `nova fmt` formatting `tests/` and skipping a nested package.
- `nova run` and `nova build` refused in a library, with the message.
- Each of M0005 and M0007–M0013 rendered.

### 7.4 The language server

- The app's analysis publishes nothing for `geom`'s files or manifest.
- An error in `geom` shows on the app's `geom` entry, with its place.
- Editing `geom`'s open buffer re-checks the app.
- A `tests/` file gets its diagnostics.
- A manifest error is published under that project's `nova.toml`.

### 7.5 Mutants

Each must fail a named test:
- the clash rule (E0004 never raised);
- the dev-dependency rule (dev-dependencies visible from `src/`);
- identity by canonical directory (by path as written);
- identity's directory (`src` and `tests` merged);
- `main` from the entry (any module's `main` accepted);
- one owner per file (the app publishes for its dependencies' modules).

## 8. The gate

A new script, `.github/scripts/packages-gate.sh`, runs in CI's `install`
job after `gate.sh`, on all three systems, through the installed `nova`:

1. `nova new --lib geom`. Its `src/` gains a `utils.nova` with a function
   `lib.nova` uses.
2. `nova new app`. Its `src/` gains a different `utils.nova`.
3. In `app`, run `nova add geom --path ../geom`.
4. `app`'s `main.nova` imports both and prints from each, and `nova run`
   prints both lines.
5. `nova test` passes in both packages, `geom`'s `tests/geom_test.nova`
   included.

The full suite passes on all three systems, since this changes module
identity (`docs/phase-3-plan.md` §6 risk 4).

## 9. Records

- **ADR 0030, "Package modules":**
  - identity as (package, directory, file);
  - the import rules of §4.3: `-` → `_`, E0004, dev-dependencies from
    `tests/` only, exact case;
  - a library's API is its `lib.nova`;
  - a program's `main` is its entry's;
  - module mode for libraries, and what it does not check;
  - coherence across packages, with no orphan rule;
  - the language server's one-owner rule, and its watching limit.
- **Dated notes:**
  - `nova-spec/40-TOOLING.md` §4.1 (path dependencies resolve; M0005 moves
    and narrows), §4.5 (`add --path`, `remove`) and §7 (`tests/`, and only
    reached `src/` files' tests);
  - ADR 0003 (module identity);
  - `nova-spec/12-TYPESYSTEM.md`'s code table (E0004);
  - the 3.0 spec's §6.1 (a file argument in `src/` or `tests/` reads the
    manifest);
  - `docs/phase-3-plan.md`'s 3.3 entry (the split into 3.3a and 3.3b).
- **`CHANGELOG.md`**, including three changes in behaviour:
  - an entry with no `main` is E0601 even when an imported module has one;
  - imports match a file's case exactly on every system;
  - `nova run src/main.nova` inside a project reads its manifest.
- **The README's** project section.

## 10. Risks

1. **Module identity changes for every compile.**
   - The loader and resolver change runs under the full suite on all three
     systems.
   - Loose programs resolve through `name_imports`, today's lookup, which
     3.0's and 3.2's tests pin.
2. **A mistake in a path or a canonical directory** could load one package
   twice, or two as one. §7.1's diamond and duplicate-name tests, and
   §7.5's identity mutants, cover it.
3. **Exact-case imports** could break a program that imported with the
   wrong case on Windows or macOS. The CHANGELOG says so, and the suite runs
   on both.
4. **The server's reach grows** across package directories. §6's
   one-owner rule and §7.4's tests keep a dependency's diagnostics in one
   place.

## 11. Decisions

The user's, on 2026-10-08:

1. **3.3 is two specs:** 3.3a "Local packages" (this one), then 3.3b "The
   index and publishing".
2. **Import names:** `-` becomes `_` (Cargo's rule); clashes and keywords
   are errors.
3. **A file and a dependency with one name:** an error, E0004. Neither wins.
4. **Dev-dependencies:** importable from `tests/` only.
5. **Architecture:** the loader decides what an import names, and the
   resolver reads its table (approach 1 of three).
6. **Re-exports stay deferred:** a library's API is its `lib.nova`.

Taken while writing the spec, and in correcting it after the fact-check,
for the user's review:

7. **The package comes from the file's location** (§4.1). A file directly
   in `src/` or `tests/` is a module of its package, a file argument
   included. Any other file is loose and reads no manifest.
8. **`nova add --path` writes the path relative to the manifest's
   directory,** whatever the current directory, or absolute across drives
   (§5.3).
9. **`nova check` is one analysis with several roots** (§5.1), so a module
   is checked once.
10. **E-codes:** E0004 for the clash; E0001 with a note for a
    dev-dependency imported from `src/`, or for a package importing itself
    from tests without a library.
11. **Identity includes the directory** (`src` or `tests`), so `tests/`
    helpers may share names with `src/` files (§4.2).
12. **A program's `main` is its entry's,** found by module, with every
    other `main` renamed before MIR (§4.6).
13. **`nova test` roots** are the program, the library and `tests/` (§4.4).
14. **A dependency's problems** show on the dependent's manifest entry,
    with their real place in the message, and each `nova.toml` is published
    only by its own project (§6).
15. **Re-checks across packages** use a map the checker shares with the
    protocol thread. Changes on disk outside the editor's workspace are not
    seen (§6).
16. **M0012 also covers** the root's own import name, both tables
    together, a repeated key, and keywords. The library templates refuse a
    keyword name (§3.4).
17. **The library template's test file** is `tests/<import name>_test.nova`
    (§5.5).
18. **M0005 moves into `nova_pm::graph`,** keeping the dependency's name in
    its message (§3.2).
19. **`ModuleSource` carries its import table.** `name_imports` keeps
    today's lookup for loose programs and tests (§4.5).
20. **Cycles** are found over regular edges. A self-path is M0010, and each
    code points where §3.5's table says. A package with neither target is
    M0013 (§3.1, §3.3).
21. **`nova add` checks the graph before writing.** It edits table, inline
    and dotted forms, and refuses a broken manifest. `nova remove` takes the
    entry's comment lines with it (§5.3).
22. **Order:** roots in a fixed order and `tests/` sorted by file name. The
    filter is a positional substring. Only reached `src/` files' tests run
    (§4.4, §4.6, §5.2).
23. **`nova check` leaves `tests/` out,** as it strips tests today, while
    the server includes them. A library-only root is checked in module
    mode (§4.4, §5.1).
24. **Imports compare a file's name exactly,** on every system (§4.3).
25. **`nova fmt`** skips nested packages, and a missing `tests/` is fine
    (§5.4).
26. **Coherence** stays program-wide, with no orphan rule; conflicts across
    packages are today's error (§4.8).
27. **`real_path` moves into `nova-pm`,** and `nova-pm` gains `nova-lexer`
    for `KEYWORDS` (§3.3, §3.6).

## 12. Not in 3.3a

- **3.3b, "The index and publishing":**
  - registry dependencies and the semver resolver;
  - fetching into a checksummed cache, and `nova.lock`;
  - `nova add <pkg>@<req>` and `nova update`;
  - `nova package`, `nova login` and `nova publish`;
  - the two-part gate: CI against a local file index, and once by hand
    against the user's real index.
- **Still deferred** (ADR 0003, ADR 0025): re-exports, `import … as`,
  qualified `m::name` paths, and nested module directories, which also
  leaves out `src/` and `tests/` subdirectories.
- **Waiting** (`docs/phase-3-plan.md` §3 decision 2, the 3.0 spec §5.1):
  features, `[[bin]]`, `[lib]`, `[build]` and git dependencies.
- **Out of Phase 3** (ADR 0026): `nova clean` and `nova install`.
- **Watching a dependency outside the editor's workspace** for changes made
  by other programs (§6).
