# 40 — Tooling Specification

> Phase: 3 (most), 0–1 (CLI skeleton)
> Crates: `nova-cli`, `nova-fmt`, `nova-lsp`, `nova-pm`, `nova-doc`

---

## 1. The `nova` CLI

Single binary, all subcommands. Built with `clap` derive.

### 1.1 Subcommands

```
nova new <name>              Create new project from template
nova init                    Initialize project in current dir
nova run [file]              Compile + run (default: src/main.nova)
nova build [--release]       Build for current target
nova build --target wasm     Build for browser
nova test [filter]           Run tests
nova bench [filter]          Run benchmarks
nova check                   Type-check without codegen
nova fmt [--check]           Format files
nova lint                    Run linter (warnings beyond type errors)
nova doc [--open]            Generate documentation
nova add <pkg>[@version]     Add dependency
nova remove <pkg>            Remove dependency
nova update [pkg]            Update dependencies
nova publish                 Publish to registry
nova install <pkg>           Install binary package globally
nova bundle                  Bundle frontend (alias for build --target wasm)
nova dev                     Dev server with HMR
nova lsp                     Run LSP server (called by editors)
nova repl                    Interactive REPL
nova clean                   Remove build artifacts
nova version                 Show version
nova help [cmd]              Show help
```

**Amended 2026-10-07 (branch `phase-3-0-foundations`):** `nova version`,
`nova new <name>` and `nova init` exist. `nova init` also takes
`--name <name>`, and defaults to the directory's own name. With no file
argument, `run`, `build` and `check` find the project by walking up from the
current directory to the nearest `nova.toml`, and so does `test`, which
takes no file; outside any project they keep `src/main.nova`. In a project,
`build` writes `target/debug/<name>`, and `build --release` writes
`target/release/<name>`
(`docs/superpowers/specs/2026-10-07-phase-3-0-foundations-design.md` §6).

**Amended 2026-10-07 (branch `phase-3-1-formatter`):** `nova fmt` exists, as
`nova fmt [PATH]... [--check] [--stdin]`. With no path it formats the
project's `src/`, or `src/` outside a project when `src/main.nova` exists;
paths may name files or directories. It exits 0, 1 when `--check` finds a
file that would change, or 2 on any error. ADR 0028 reads the master spec's
"only `--check`" as "no style options"
(`docs/superpowers/specs/2026-10-07-phase-3-1-formatter-design.md` §7).

**Amended 2026-10-09 (branch `phase-3-3b-index-publishing`):** `nova
add <pkg>[@<req>]`, `nova update [pkg]`, `nova publish` and `nova login`
exist, with two commands this list lacks. `nova fetch` downloads what
`nova.lock` names, and `nova package` packs and verifies a library
(`docs/superpowers/specs/2026-10-09-phase-3-3b-index-and-publishing-design.md`
§6; ADR 0031).

### 1.2 Project Template (`nova new`)

```
my-app/
├── nova.toml
├── README.md
├── .gitignore
├── src/
│   └── main.nova
└── tests/
    └── basic_test.nova
```

`nova.toml`:
```toml
[package]
name = "my-app"
version = "0.1.0"
edition = "2026"
authors = ["Your Name <you@example.com>"]
description = "A new Nova project"

[dependencies]
# http = "1.0"

[dev-dependencies]
# test-utils = "0.2"

[build]
target = "native"
```

**Amended 2026-10-07 (branch `phase-3-0-foundations`):** `nova new` writes
`nova.toml`, `.gitignore`, `README.md` and `src/main.nova`. Its `nova.toml`
has only `name`, `version = "0.1.0"`, `edition = "2026"` and an empty
`[dependencies]`:
- no `authors` placeholder, which would be published as written;
- no `description`;
- no `[dev-dependencies]` or `[build]`.

There is no `tests/` until Phase 3.3, and the template's one test is in
`src/main.nova` (spec §6.3).

---

## 2. Formatter (`nova fmt`)

### 2.1 Principles (from gofmt playbook)
- **No options.** Style is fixed.
- Always 4 spaces indent
- Always trailing comma in multi-line lists
- Max line length: 100 (soft, breaks on operator boundaries)
- Always braces around blocks (`if x { ... }`, never `if x then ...`)
- Imports sorted alphabetically, std first then third-party
- Always exactly one blank line between top-level items
- Always one space around binary operators
- Never spaces inside parentheses

### 2.2 Implementation
- Re-uses parser to get AST
- Formats by walking AST and emitting tokens
- Use **PrettyPrinter** approach: build doc tree, render with width budget (algorithm: Wadler's prettier paper)

### 2.3 Modes
```
nova fmt              # format all files in project
nova fmt --check      # exit non-zero if changes needed (CI-friendly)
nova fmt path/file.nova   # format single file
nova fmt --stdin      # read from stdin, write to stdout
```

### 2.4 EditorConfig integration
Respects `.editorconfig` for line endings and final newline only.

**Amended 2026-10-07 (branch `phase-3-1-formatter`):** §2.1 to §2.4 as built
(`docs/superpowers/specs/2026-10-07-phase-3-1-formatter-design.md` §5 to §7;
ADR 0028):
- The 100 columns are counted in Unicode scalar values. A construct that
  does not fit breaks in its own way, and a line holding something that
  cannot break, such as a long string, may run past 100.
- Imports are sorted within each run of consecutive `import` items, by path,
  and each `{…}` list by name, in byte order. "std first" waits for packages
  (3.3).
- The author's parentheses are kept, one pair each. Statements print without
  `;`, and match arms without `,`, except where the parser would otherwise
  read two as one.
- Every comment is kept, and every run checks its output: the same AST, the
  same comments, or the file is left unchanged.
- §2.3: paths may also name directories, and there may be several.
- §2.4: `.editorconfig`'s `end_of_line` (`lf` or `crlf`) wins, then the
  file's own line ending, then LF; one final newline unless
  `insert_final_newline = false`. The files are found by walking up to one
  with `root = true`, and EditorConfig's globs are matched, except numeric
  ranges.

**Amended 2026-10-10 (branch `phase-3-4b-fixes-colour`):** a blank line
between imports ends a group, as in gofmt, and each group is sorted on
its own (ADR 0028). Organize imports writes two groups, the
dependencies and then the project's own modules: "std first then
third-party", in a Nova whose std needs no import (ADR 0033).

---

## 3. LSP Server (`nova lsp`)

### 3.1 Capabilities

| Feature | Phase | Notes |
|---|---|---|
| Diagnostics (errors, warnings) | 3 | Uses incremental compilation via salsa |
| Hover | 3 | Show types and docs |
| Goto Definition | 3 | |
| Find References | 3 | |
| Rename | 3 | Cross-file |
| Completion | 3 | Type-aware |
| Code Actions | 3 | Fix-its, organize imports |
| Format on save | 3 | Calls `nova-fmt` |
| Semantic Highlighting | 3 | |
| Inlay Hints | 4 | Inferred types, parameter names |
| Code Lens | 4 | Run/debug test buttons |
| Call Hierarchy | 4 | |

**Amended 2026-10-08 (branch `phase-3-2-lsp-core`):** 3.2 delivers
diagnostics, completion and format on save
(`docs/superpowers/specs/2026-10-08-phase-3-2-lsp-core-design.md`).
Diagnostics come from re-checking the whole program, not from `salsa`
(ADR 0029). Completion offers members after `.`; elsewhere the locals,
the module's names, the primitive types and the keywords. The other Phase
3 rows are 3.4's.

**Amended 2026-10-10 (branch `phase-3-4a-navigation`):** 3.4a delivers
hover, go to definition, find references and rename
(`docs/superpowers/specs/2026-10-10-phase-3-4a-navigation-design.md`;
ADR 0032). They read an index the type checker records while it
resolves names, not salsa's queries. References and rename reach the
owning project, and rename checks itself by analysing the renamed
program again. Code actions and semantic highlighting are 3.4b's.

**Amended 2026-10-10 (branch `phase-3-4b-fixes-colour`):** 3.4b delivers
code actions and semantic highlighting
(`docs/superpowers/specs/2026-10-10-phase-3-4b-fixes-and-colour-design.md`;
ADR 0033). Diagnostics carry fixes, made where each error is found,
which `nova check` prints as `help:` lines and code actions offer;
organize imports is a source action. Semantic tokens cover names only.
With 3.4a, every Phase 3 row of this table is delivered.

### 3.2 Architecture
- `tower-lsp` for protocol
- `salsa` for incremental query system (parse → resolve → typecheck queries cached per file)
- File watcher to invalidate cache on disk changes
- Workspace-aware: scans `nova.toml` to find roots

**Amended 2026-10-08 (branch `phase-3-2-lsp-core`):** as built, the stack
is `lsp-server` 0.7.8, pinned, and `lsp-types` 0.97, not `tower-lsp`.
Analyses run on one checker thread, re-checking the whole program, not
through `salsa`. The file watcher is the client's, registered
dynamically. A file's project is the nearest directory holding
`nova.toml`, found on demand rather than by a scan. ADR 0029 has the
reasons and the measured budget.

### 3.3 Editor Extensions
- `tools/vscode-nova/` — TypeScript-based, registers language + connects to `nova lsp`
- `tools/zed-nova/` — Zed extension config (TOML)
- `tools/nvim-nova/` — Neovim Lua config + tree-sitter grammar
- All use the same `nova lsp` backend

**Amended 2026-10-08 (branch `phase-3-2-lsp-core`):** `tools/vscode-nova/`
exists. It is TypeScript with `vscode-languageclient` 10, a TextMate
grammar, and a smoke test in CI, and it runs the installed `nova`. The
Zed and Neovim extensions are outside Phase 3 (ADR 0026).

---

## 4. Package Manager (`nova-pm`)

### 4.1 Manifest (`nova.toml`)
```toml
[package]
name = "..."
version = "..."
edition = "2026"
description = "..."
license = "..."
repository = "..."
keywords = ["..."]
categories = ["..."]

[dependencies]
http = "1.0"
postgres = { version = "0.5", features = ["pool"] }
my-fork = { git = "https://github.com/me/fork", branch = "main" }
local = { path = "../local-pkg" }

[dev-dependencies]
test-utils = "0.2"

[features]
default = ["http"]
http = []
postgres = ["dep:postgres"]

[build]
target = "native"
profile = "release"

[[bin]]
name = "myapp"
path = "src/main.nova"

[lib]
path = "src/lib.nova"
```

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

**Amended 2026-10-08 (branch `phase-3-3a-local-packages`):** path
dependencies resolve
(`docs/superpowers/specs/2026-10-08-phase-3-3a-local-packages-design.md`
§3). Each `path` is read relative to its own manifest, and a package is
identified by its canonical directory. M0005 now covers only a
version-only entry, and `nova_pm::graph` raises it, not the CLI.
M0007–M0013 are the graph's errors. A dependency is imported by its name
with each `-` replaced by `_` (ADR 0030).

**Amended 2026-10-09 (branch `phase-3-3b-index-publishing`):** a version
entry resolves against the package index. M0005 now means an entry the
cache cannot satisfy: not downloaded, locked at a version that no longer
fits, or `NOVA_HOME` not found. M0014–M0017 are the index's errors: a
package the index lacks, a conflict no version meets, an unreadable
`nova.lock`, and a path dependency in a package to publish (ADR 0031).

### 4.2 Lock file (`nova.lock`)
- TOML, similar to Cargo.lock
- Records exact versions + hashes
- Committed for binaries, optional for libraries

**Amended 2026-10-09 (branch `phase-3-3b-index-publishing`):** as built,
`nova.lock` holds `version = 1`, the index's canonical form, and one
`[[package]]` per registry package: its name, version, SHA-256 and
dependencies' names. A path package is in the tree, so it is not
locked. Every command that syncs writes it last, with `\n` endings
(ADR 0031).

### 4.3 Resolver
- Semver-based
- Compatible with Cargo's resolver semantics
- Edition compatibility rules

**Amended 2026-10-09 (branch `phase-3-3b-index-publishing`):** one
version of each name per build, with Cargo's caret requirements. Locked
versions that still fit are kept. Edition rules are out of Phase 3 (ADR
0026; ADR 0031).

### 4.4 Registry
- Central registry at `registry.novalang.dev` (run separately)
- Backend: Rust (axum) + Postgres + S3
- Mirror-friendly (full index downloadable)
- `cargo`-like sparse index format

**Amended 2026-10-09 (branch `phase-3-3b-index-publishing`):** the index
is a GitHub repository, `Sakeerin/nova-index`, read over HTTPS, with
tarballs as release assets (ADR 0026). There is no server to run, and
`registry.novalang.dev` does not exist. `NOVA_INDEX` names another
index, a local directory included (ADR 0031).

### 4.5 Commands

```
nova add <pkg>           Resolve + add to manifest + update lock
nova add <pkg>@^1.2      Specific version
nova add <pkg> --dev     Add to dev-deps
nova remove <pkg>
nova update              Update all
nova update <pkg>        Update specific
nova publish             Pack + upload (requires login)
nova login               Auth via token
nova owner add/rm <user> <pkg>
```

**Amended 2026-10-08 (branch `phase-3-3a-local-packages`):** 3.3a has
`nova add <pkg> --path <dir> [--dev]` and `nova remove <pkg> [--dev]`.
`nova add` writes the path relative to the manifest, keeps the
manifest's comments and layout, and checks the graph before it writes.
The version form, `nova update`, `nova publish` and `nova login` are
3.3b's.

**Amended 2026-10-09 (branch `phase-3-3b-index-publishing`):** as built,
`nova add <pkg>` writes the version it resolved, and `nova add
<pkg>@<req>` the requirement given. `nova update <pkg>` lets only that
package move. `nova login` reads a token from a pipe. `nova owner` is
out of Phase 3 (ADR 0026; ADR 0031).

### 4.6 Publishing
- `nova package` creates `.nova-pkg` (gzip tar)
- Includes: source files, `nova.toml`, README, LICENSE
- Excludes: `target/`, `nova.lock` (for libs), VCS dirs
- Verification: must compile with no warnings

**Amended 2026-10-09 (branch `phase-3-3b-index-publishing`):** a
`.nova-pkg` holds `nova.toml`, `src/`, `tests/` and the top-level
`README*` and `LICENSE*`, under `<name>-<version>/`. It leaves out
`target/`, `nova.lock` and every name starting with `.`, and is
reproducible to the byte. Only a library is published, with no path in
`[dependencies]`. The verification unpacks the tarball, resolves it
afresh, and checks it as `nova check` does: any error, or any warning
of its own, stops it (ADR 0031).

---

## 5. Doc Generator (`nova doc`)

- Parses `///` doc comments
- Markdown rendered to HTML
- Output: `target/doc/` static site
- Search via pre-built index (like rustdoc)
- Code examples in docs are testable: `nova test --doc`
- Theme: clean, light/dark mode toggle, similar to rustdoc visually

---

## 6. REPL (`nova repl`)

- `rustyline` for line editing
- Each input compiled with Cranelift JIT, executed in-process
- Variables persist across inputs
- `:help`, `:type`, `:load`, `:reload`, `:quit` meta-commands

---

## 7. Test Runner (`nova test`)

- Discovers `@test` functions in:
  - `src/**/*.nova` (in-module tests)
  - `tests/**/*.nova` (integration)
- Compiles tests as separate binary
- Runs in parallel (default = num CPUs)
- Captures stdout/stderr (shows on failure only)
- Output: TAP-compatible + pretty default
- `--filter <name>` runs only matching
- `--bench` runs benchmarks instead

**Amended 2026-10-08 (branch `phase-3-3a-local-packages`):** as built,
`nova test` runs the tests of the `src/` files that the program and the
library reach, not all of `src/**`, and every top-level `tests/*.nova`,
sorted by name. A test in `tests/api.nova` is named `api::<function>`,
and the filter is a positional substring of that name. Subdirectories of
`tests/` are not read (ADR 0003's deferral). A dependency's tests are
never run.

---

## 8. Linter (`nova lint`)

Beyond type errors, warn on:
- Unused variables, imports, functions
- Dead code (unreachable)
- Inefficient patterns (e.g. `String::new() + "..."`)
- Style violations not enforced by formatter
- Suspicious patterns (e.g. ignoring `Result`)

Each warning has stable code, can be `@allow(unused)` / `@deny(unused)`.

---

## 9. CI Integration

Provide `.github/workflows/nova.yml` template:
```yaml
name: CI
on: [push, pull_request]
jobs:
  build:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: novalang/setup-nova@v1
        with: { version: stable }
      - run: nova fmt --check
      - run: nova lint
      - run: nova test
      - run: nova build --release
```

---

## 10. Installer

### 10.1 Unix (curl-pipe-bash)
```bash
curl -sSf https://novalang.dev/install.sh | sh
```
Downloads `nova` binary for detected platform, places in `~/.nova/bin/`, adds to PATH.

**Amended 2026-10-07 (branch `phase-3-0-foundations`):** `~/.nova` also holds
`runtime/<version>-<crc32>/`, where `nova` unpacks the runtime library it
carries the first time it links a program; `NOVA_HOME` moves the whole
directory. Until the install scripts exist (Phase 3.6),
`cargo install --locked --git https://github.com/Sakeerin/nova nova-cli`
installs a `nova` that needs nothing beside it (ADR 0027).

**Amended 2026-10-09 (branch `phase-3-3b-index-publishing`):** `~/.nova`
also holds `registry/`, the downloaded packages, and `credentials.toml`,
the token `nova login` stores (ADR 0031).

### 10.2 Windows
```powershell
irm https://novalang.dev/install.ps1 | iex
```

### 10.3 Versioning
- `nova self update` updates the binary
- `nova self uninstall` removes everything
- Multiple toolchains: `nova +stable build`, `nova +nightly build` (Phase 6)
