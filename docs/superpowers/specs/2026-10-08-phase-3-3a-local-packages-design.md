# Phase 3.3a, "Local packages": design

- **Date:** 2026-10-08.
- **Status:** the design was approved in conversation on 2026-10-08, in five
  sections (§11 lists the user's answers). This written spec awaits the
  user's review.
- **Branch:** `phase-3-3a-local-packages`, cut from `main` at `ff3c54f`.
- **Inputs:**
  - `docs/phase-3-plan.md` §2 item 4, §3 decisions 2, 3 and 5, §4's 3.3
    entry, §5 and §6 risk 4;
  - `nova-spec/40-TOOLING.md` §4.1, §4.5 and §7;
  - ADR 0003 (the module model), ADR 0025 (its backlog) and ADR 0026;
  - the 3.0 spec's §5 and §6 (`nova.toml`, projects);
  - the 3.2 spec's §6.2 (which analysis owns a file).

## 1. What 3.3a delivers

The phase plan's 3.3 is split in two (§11, decision 1). This spec is the
first half; 3.3b, "The index and publishing", follows once it merges.

- **Packages.** A package is a directory with `nova.toml` and a library
  (`src/lib.nova`), a program (`src/main.nova`), or both.
- **Path dependencies.** `geom = { path = "../geom" }` resolves, and
  `import geom` reaches that library from any file of the package.
- **Module identity is (package, file).** Two packages may each have a
  `utils.nova`, and both can be in one program.
- **`tests/`.** `nova test` also runs the top-level `tests/*.nova` files,
  which import the package by name, its dev-dependencies, and each other.
- **Commands:**
  - `nova add <name> --path <dir> [--dev]` and `nova remove <name> [--dev]`;
  - `nova new --lib` and `nova init --lib`;
  - `nova fmt` also formats `tests/`.
- **The language server follows:**
  - each file has one owner, its own package's analysis;
  - an edit in a dependency re-checks its dependents;
  - manifest errors appear under `nova.toml`.

Registry dependencies, the resolver, fetching, `nova.lock`, `nova update`
and publishing are 3.3b's (§12).

## 2. The starting point (read on 2026-10-08)

### The driver (`crates/nova-driver/`)

- `analyze::load_program` (`analyze.rs:96`) loads the entry and every
  module it imports.
  - It names each module by its file stem.
  - It finds `import m` as `m.nova` in the entry's directory, the only
    directory it reads, and loads each name once.
- `strip_test_functions` (`lib.rs:652`) removes every `@test` function from
  every module when not testing.
- `check_file`, `compile_file`, `build_file`, `build_file_release` and
  `build_test_binary` (`lib.rs:44-198`) each take one source path.
- `nova-driver` does not depend on `nova-pm`.

### The resolver (`crates/nova-resolver/src/lib.rs`)

- `resolve_program` (`:1503`) builds one map from module name to module
  index over every module (`by_name`).
- `resolve_import` (`:2274`) looks an import's name up in that map, with
  these E0001 messages:
  - "cannot find module `m`";
  - "module `m` imports itself";
  - "`x` is not a public item of module `m`".
  
  A multi-segment path (`a::b`) is rejected as unsupported.
- Messages name a module as the import wrote it. `ModuleScope.name` is
  stored but no caller reads it (`Definitions::module_name`, `:1265`, has
  none).
- Symbols are mangled with the definition's `DefId`
  (`nova_mir::mangle`, `crates/nova-mir/src/lib.rs:1093`), so two
  functions of one name in two modules never clash below the resolver.

### The CLI and `nova-pm`

- `nova_pm::parse` (`crates/nova-pm/src/manifest.rs`) reads `[package]`,
  `[dependencies]` and `[dev-dependencies]`.
  - A dependency is a version requirement or a table with `version` or
    `path`, never both.
  - Its diagnostics are M0001–M0006.
- `project::mode` (`crates/nova-cli/src/project.rs`) has two cases:
  - a file argument means file mode, and no manifest is read;
  - otherwise the nearest `nova.toml` makes a project, which must have
    `src/main.nova`.
  
  Every declared dependency is an error, M0005 (`unresolved_dependencies`,
  "the one place 3.3 removes").
- `nova_pm::check_name` allows ASCII letters, digits, `-` and `_`, starting
  with a letter.
- `nova fmt` with no paths formats the project's `src/` (`cmd/fmt.rs:111`).
- `nova new` and `nova init` write a program template
  (`crates/nova-cli/src/template.rs`).
- `nova test` builds one test binary from the project's entry
  (`cmd/test.rs:160`).

### The language server (`crates/nova-lsp/`)

- `ProjectKey::entry` is `src/main.nova` (`workspace.rs:199`).
- A project's analysis publishes for every module its entry reaches
  (`checker.rs`, `publish_own`).
- `holds` is "under the project's directory", or for a loose file "beside
  it".

### CI

`.github/scripts/gate.sh` is the 3.0 gate. It makes, runs, builds and tests
a new project with the installed `nova`. CI's `install` job runs it on all
three systems, and release.yml's smoke test runs it too.

## 3. Packages and the graph (`nova-pm`)

### 3.1 What a package is

- A package is a directory with `nova.toml`, and one or both of:
  - `src/lib.nova`, a library;
  - `src/main.nova`, a program.
- A package with neither is refused by every command, with a message
  naming both files.

### 3.2 Path dependencies

- A `[dependencies]` or `[dev-dependencies]` entry with `path` resolves.
- The path is read relative to the directory of the manifest that declares
  it.
- The target directory must hold:
  - a `nova.toml` whose `package.name` equals the entry's key;
  - a `src/lib.nova`. A dependency is always a library.
- An entry with `version` and no `path` stays an error, M0005. Its message
  becomes "registry dependencies arrive with the package index", and its
  note says to use `path` for a local package.
- 3.0's rule that an entry has `version` or `path`, never both, stands.
  3.3b decides how a path dependency is published.

### 3.3 The graph

- Dependencies are followed transitively, each `path` relative to its own
  manifest.
- **Identity:** a package is identified by its canonical directory. That is
  `std::fs::canonicalize`, with Windows' `\\?\` prefix stripped, as the 3.2
  server's `real_path` does. One directory reached by two routes, a
  diamond, is one package.
- **One package per name per build:** two different directories whose
  manifests give the same `package.name` are an error. The root package
  counts.
- **Cycles** are an error, which lists the cycle by package name.
- **Dev-dependencies:** only the root package's count. A dependency's
  `[dev-dependencies]` are not read for the graph, as in Cargo.
- **Warnings:** every manifest's warnings (M0006) are reported, each
  against its own `nova.toml`.

### 3.4 Import names

- A dependency's import name is its key with each `-` replaced by `_`.
  `json-api` is imported as `import json_api` (§11, decision 2).
- These are errors:
  - two dependencies of one package whose import names coincide;
  - an import name that is one of `nova_lexer::KEYWORDS`.

### 3.5 Diagnostics

Each points at the dependency entry at fault in the manifest that declares
it, as M0001–M0006 do (3.0 spec §5).

| Code | Error |
|---|---|
| M0007 | the path is not a directory holding a `nova.toml` |
| M0008 | the dependency's `package.name` is not the entry's key |
| M0009 | the dependency has no `src/lib.nova` |
| M0010 | a dependency cycle, listed by name |
| M0011 | two packages in one build with the same name |
| M0012 | an import name that clashes with another, or is a keyword |

### 3.6 Interface

- `nova_pm::graph(root: &Path) -> (Option<Graph>, Vec<Diagnostic>)` reads
  every manifest from disk into a `FileDb` the caller passes in.
- A `Graph` holds packages by `PackageId`. Each package has:
  - its name and import name;
  - its canonical directory and parsed manifest;
  - whether it has `lib.nova` and `main.nova`;
  - its dependencies and dev-dependencies, as (import name, `PackageId`).
- `graph` is partial on error: the packages and edges that resolved are
  kept, with the diagnostics.
  - The CLI stops on any error, as it does for manifests today.
  - The language server goes on with what resolved (§6).

## 4. Modules and imports

### 4.1 Which package a file belongs to

- A file directly in a package's `src/` is a source module of that
  package.
- A file directly in its `tests/` is a test module of that package.
- Any other file is loose, as today: its imports are files beside it, and
  it has no dependencies.
- This holds for a file argument too: `nova run src/main.nova` inside a
  project sees the project's dependencies (§11, decision 7). Where
  `nova build` writes stays as 3.0 decided.

### 4.2 Module identity

- A module is (package, file stem).
- The loader keeps modules by (package, stem), so two packages' `utils`
  are two modules.
- A loose program is one package-less set of files, exactly as today.

### 4.3 What an import names

In a module of package P, `import x` (or `import x::{…}`):

1. **A file of the same package.** For a source module, P's
   `src/x.nova`; for a test module, P's `tests/x.nova`.
2. **A dependency.** If `x` is the import name of one of P's dependencies,
   that dependency's `src/lib.nova`.
   - For a test module of the root package, the root package itself counts
     as a dependency, under its import name, along with the root's
     dev-dependencies.
   - A source module never sees dev-dependencies.
3. **Both** a file and a dependency: **E0004**, "`x` is both a module of
   this package and a dependency". Its labels point at the import and name
   the file and the dependency's manifest entry (§11, decision 3).
4. **A dev-dependency imported from `src/`:** E0001, "cannot find module
   `x`", with the note "`x` is a dev-dependency, which only `tests/` files
   can import" (§11, decision 4).
5. **Neither:** today's E0001, "cannot find module `x`".

A dependency's own source modules import each other in its own `src/`, so
`lib.nova` in `geom` reaches `geom`'s `utils.nova`, not the app's. A program
reaches its own library by file name, `import lib`.

### 4.4 The loader

- `load_program` takes the root package's graph, or none for a loose
  program, and a list of root files:
  - the entry;
  - for `nova check`, the library too;
  - for `nova test` and the server, every `tests/*.nova`.
- It records, per module, a table from each import's first segment to the
  module it names, or to the diagnostic of §4.3.
- The driver's entry points take a description of what to load, not one
  path:

  ```rust
  pub struct Program {
      pub graph: Option<nova_pm::Graph>,
      pub roots: Vec<PathBuf>,
  }
  ```

  `Program::for_file(path)` finds the package from the file's location
  (§4.1). Each existing entry point becomes a wrapper over its `Program`
  form, so callers that pass a path keep working.
- `nova-driver` gains a dependency on `nova-pm`. That brings nothing new
  into `Cargo.lock`: `nova-pm`'s own dependencies, `semver` and
  `toml_edit`, are already there.

### 4.5 The resolver

- `ModuleSource` gains the import table, and `resolve_import` reads it in
  place of `by_name`, which goes.
- Everything after the lookup stays as it is:
  - globs and `import x::{a, b}`;
  - E0002 conflicts;
  - `pub` visibility;
  - std's glob into every module;
  - the "imports itself" check.
- `ModuleSource.name` becomes a display label:
  - a root-package file's stem, `utils`;
  - a dependency's library, by import name, `geom`;
  - a dependency's other file, `geom/utils`.
  
  Today's messages already print the import as written, so they read the
  same.

### 4.6 Tests and `main`

- `strip_test_functions` keeps `@test` functions only in the root
  package's modules, and only under `nova test` and in the server.
  Dependencies' tests are stripped always.
- A test in a test module is named `<file stem>::<function>`, for example
  `api::parses_users`. A test in `src/` keeps its bare name.
- E0601's check for `fn main` still looks only at the entry. A program has
  `src/main.nova` as its entry. A library-only root is checked in 3.2's
  module mode, without MIR.

### 4.7 A library's API is its `lib.nova`

- With no re-exports and no `geom::utils` paths (ADR 0003's deferrals, on
  ADR 0025's backlog), a dependent can name only the `pub` items of the
  dependency's `lib.nova`.
- A type declared in another file of the library can still flow through
  `lib.nova`'s functions, but the dependent cannot write its name.
- Library authors put their public types in `lib.nova`. Re-exports stay
  deferred (§11, decision 6), and ADR 0030 records the limit.

## 5. Commands

### 5.1 Which commands work on which package

- **`nova run` and `nova build`** need `src/main.nova`. In a library-only
  package they stop with "`geom` is a library: it has no src/main.nova;
  `nova check` and `nova test` work on it".
- **`nova check`** is one analysis whose roots are the program and the
  library, whichever exist, so a module both reach is checked once. MIR
  runs when there is a program.
- **Dependencies** are compiled into the program, as every module is today.
  They never get a `target/` of their own.
- **Manifest errors** (M-codes) stop every command before compiling, as
  today.

### 5.2 `nova test` and `tests/`

- One test binary holds:
  - the root package's `src/` tests;
  - every top-level `tests/*.nova`.
  
  Subdirectories of `tests/` are not read, as ADR 0003 defers nested
  directories.
- A test module may import:
  - the package itself, by its import name, which needs a library;
  - the root's dependencies and dev-dependencies;
  - other `tests/` files, by name.
  
  A `tests/` file with no `@test` is a helper module.
- Importing the package from a test module of a package with no library is
  E0001, with the note "`app` has no src/lib.nova".
- Each test still runs in its own process.
- `--filter` matches the full name, `api::parses_users` included.

### 5.3 `nova add` and `nova remove`

- `nova add <name> --path <dir> [--dev]`:
  - checks the target as §3.2 does: a `nova.toml` naming `<name>`, and a
    `src/lib.nova`;
  - writes `<name> = { path = "<dir>" }` into `[dependencies]`, or with
    `--dev` into `[dev-dependencies]`. `<dir>` is made relative to the
    manifest's directory, with `/` separators (§11, decision 8);
  - edits through `toml_edit`'s `DocumentMut`, so the manifest keeps its
    comments, order and layout, and creates the table if it is missing;
  - refuses an entry that already exists in that table;
  - refuses a `<name>` whose import name would clash (M0012), before
    writing anything.
- `nova remove <name> [--dev]` deletes the entry from the table, keeps the
  rest of the file, and fails if the entry is not there.
- The version form (`nova add json@^1.2`) and `nova update` are 3.3b's.

### 5.4 `nova fmt`

With no paths, in a project, it formats `src/` and `tests/`. Dependencies
are never formatted.

### 5.5 `nova new --lib` and `nova init --lib`

They write:
- `nova.toml` and `.gitignore`, as for a program;
- `src/lib.nova`, with one `pub` function and one `@test`;
- `tests/integration.nova`, which imports the package by its import name
  and tests that function. It is not `tests/<name>.nova`: a test file named
  like the package it imports would be E0004 (§4.3);
- a README whose line says `nova test` runs the library's tests and
  `tests/`.

The program template is unchanged.

## 6. The language server

- **One owner per file.**
  - A project's analysis publishes only for its own package's modules.
  - A dependency's files belong to that dependency's own project, and are
    published only when one of them is open there.
  - So opening the app never paints `geom`'s files, and two analyses never
    publish for the same file.
- **What a project's analysis covers:**
  - its roots: the program if it exists, else the library in module mode,
    plus every `tests/*.nova`, in test mode;
  - a `tests/` file is reached, so it is owned by its project's analysis;
  - a `src/` file nothing reaches is still checked on its own, as in 3.2,
    with its package's dependencies.
- **Re-checks across packages.**
  - An `Analysis` reports the directories of every package in its graph.
  - A project holds every path under any of them, so an edit, save or
    on-disk change in `geom` re-checks the app.
  - The re-check reads `geom`'s unsaved buffer through the overlay.
  - A changed `nova.toml` re-checks every open project, as in 3.2.
- **Manifest errors.**
  - The graph's diagnostics are published under each `nova.toml`'s URI,
    owned by the project whose graph read it.
  - The analysis goes on with the part of the graph that resolved, and an
    import of a dependency that failed gets the ordinary E0001.
- **`ProjectKey::entry`** becomes the program, else the library.
- **Completion** is unchanged: `import geom` binds `geom`'s `pub` names, and
  `names_in_scope` lists them.

## 7. Testing

### 7.1 `nova-pm`'s graph

- A diamond is one package.
- One test for each of M0007–M0012.
- A dependency's dev-dependencies are not read.
- Paths are relative to each manifest.
- A version-only entry is still M0005, with the new message.

### 7.2 The loader and the resolver

- Two packages, each with a `utils.nova`, both used in one program, and
  their functions called.
- E0004 for a file/dependency clash.
- E0001 with its note for a dev-dependency imported from `src/`.
- `tests/` files importing the package, a dev-dependency, and each other.
- A dependency's `@test` functions stripped, and the root's kept, under
  `nova test`.
- A library-only root checked without MIR.
- A file argument inside a project seeing its dependencies.
- Loose files unchanged.

### 7.3 The CLI, end to end through `nova`

- `nova new --lib` and `nova init --lib`, then `nova test` passing in each.
- `nova add --path` and `nova remove`, with a manifest's comments and
  layout kept byte for byte outside the edited entry.
- An app that builds and runs against a path library.
- `nova test` running `src/` and `tests/` tests, and `--filter` on
  `<file>::<name>`.
- `nova fmt` formatting `tests/`.
- `nova run` and `nova build` refused in a library, with the message.
- Each of M0007–M0012 rendered.

### 7.4 The language server

- The app's analysis publishes nothing for `geom`'s files.
- Editing `geom`'s open buffer re-checks the app.
- A `tests/` file gets its diagnostics.
- A manifest error is published under `nova.toml`.

### 7.5 Mutants

Each must fail a named test:
- the clash rule (E0004 never raised);
- the dev-dependency rule (dev-dependencies visible from `src/`);
- identity by canonical directory (by path as written);
- one owner per file (the app publishes for its dependencies' modules).

## 8. The gate

In CI, on all three systems, through the installed `nova`: a new script,
`.github/scripts/packages-gate.sh`, run by CI's `install` job after
`gate.sh`.

1. `nova new --lib geom`. Its `src/` gains a `utils.nova` with a function
   `lib.nova` uses.
2. `nova new app`. Its `src/` gains a different `utils.nova`.
3. In `app`, run `nova add geom --path ../geom`.
4. `app`'s `main.nova` imports both and prints from each, and `nova run`
   prints both lines.
5. `nova test` passes in both packages, `geom`'s `tests/` included.

The full suite passes on all three systems, since this changes module
identity (`docs/phase-3-plan.md` §6 risk 4).

## 9. Records

- **ADR 0030, "Package modules":**
  - identity as (package, file);
  - the import rules of §4.3, `-` → `_`, E0004, and dev-dependencies from
    `tests/` only;
  - a library's API is its `lib.nova`;
  - the language server's one-owner rule.
- **Dated notes:**
  - `nova-spec/40-TOOLING.md` §4.1 (path dependencies resolve; M0005
    narrowed), §4.5 (`add --path`, `remove`) and §7 (`tests/`);
  - ADR 0003 (module identity);
  - `nova-spec/12-TYPESYSTEM.md`'s code table (E0004);
  - `docs/phase-3-plan.md`'s 3.3 entry (the split into 3.3a and 3.3b).
- **Project documents:** `CHANGELOG.md`, and the README's project section.

## 10. Risks

1. **Module identity changes for every compile.**
   - The loader and resolver change runs under the full suite on all three
     systems.
   - Loose programs keep today's behaviour, which 3.0's and 3.2's tests
     pin.
2. **A mistake in a path or a canonical directory** could load one package
   twice, or two as one. §7.1's diamond and duplicate-name tests, and
   §7.5's identity mutant, cover it.
3. **The server's reach grows** across package directories. §6's
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

Taken while writing the spec, for the user's review:

7. **The package comes from the file's location**, for every command and
   in the server (§4.1). A file argument no longer means "no manifest".
   3.0's rule for where `nova build` writes is unchanged.
8. **`nova add --path` writes the path relative to the manifest's
   directory,** whatever the current directory (§5.3).
9. **`nova check` is one analysis with several roots** (§5.1), so a module
   is checked once.
10. **E-codes:** E0004 for the clash; E0001 with a note for a
    dev-dependency imported from `src/`.

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
- **Out of Phase 3** (ADR 0026): features, `[[bin]]`, `[lib]`, `[build]`
  and git dependencies, and `nova clean` and `nova install`.
