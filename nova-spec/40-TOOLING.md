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

### 3.2 Architecture
- `tower-lsp` for protocol
- `salsa` for incremental query system (parse → resolve → typecheck queries cached per file)
- File watcher to invalidate cache on disk changes
- Workspace-aware: scans `nova.toml` to find roots

### 3.3 Editor Extensions
- `tools/vscode-nova/` — TypeScript-based, registers language + connects to `nova lsp`
- `tools/zed-nova/` — Zed extension config (TOML)
- `tools/nvim-nova/` — Neovim Lua config + tree-sitter grammar
- All use the same `nova lsp` backend

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

### 4.2 Lock file (`nova.lock`)
- TOML, similar to Cargo.lock
- Records exact versions + hashes
- Committed for binaries, optional for libraries

### 4.3 Resolver
- Semver-based
- Compatible with Cargo's resolver semantics
- Edition compatibility rules

### 4.4 Registry
- Central registry at `registry.novalang.dev` (run separately)
- Backend: Rust (axum) + Postgres + S3
- Mirror-friendly (full index downloadable)
- `cargo`-like sparse index format

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

### 4.6 Publishing
- `nova package` creates `.nova-pkg` (gzip tar)
- Includes: source files, `nova.toml`, README, LICENSE
- Excludes: `target/`, `nova.lock` (for libs), VCS dirs
- Verification: must compile with no warnings

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

### 10.2 Windows
```powershell
irm https://novalang.dev/install.ps1 | iex
```

### 10.3 Versioning
- `nova self update` updates the binary
- `nova self uninstall` removes everything
- Multiple toolchains: `nova +stable build`, `nova +nightly build` (Phase 6)
