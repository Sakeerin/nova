# Phase 3 Plan — Tooling

> Status: **draft** (2026-10-06). Derived from `nova-spec/00-MASTER-SPEC.md`
> §3 (Phase 3) and `nova-spec/40-TOOLING.md`. The user's decisions of
> 2026-10-06 are in §1. Each sub-phase below gets its own spec, plan and
> review before it is built.

## 1. Goal (from the spec)

Phase 3 = "Tooling". The master spec's goal is "DX matches or exceeds
Rust/Go", and its gate reads: "External user can `cargo install nova-cli`,
init a project, write code with autocomplete, format, and publish a package."

The master spec lists eight items for Phase 3: the formatter (`nova-fmt`),
the package manager (`nova-pm`, with `nova.toml` and a lock file), a registry
server, the language server (`nova-lsp`), a VSCode extension, the doc
generator (`nova-doc`), a REPL, and a debugger. `40-TOOLING.md` specifies
each of them.

**The user's decisions, 2026-10-06:**
- **Registry: a git-backed index.** A public GitHub repo the user owns holds
  the package index, and package tarballs are attached to it as release
  assets. There is no registry server; a hosted one can serve the same index
  later.
- **Install: git now, crates.io at the release.** During Phase 3, Nova
  installs with `cargo install --git https://github.com/Sakeerin/nova
  nova-cli`. At the `v0.3.0` release, all 21 workspace crates are published
  to crates.io under the user's account, so the gate's `cargo install
  nova-cli` works literally; every one of the 21 names returned 404 on
  crates.io on 2026-10-06. Release binaries are built throughout, and the
  runtime library ships inside `nova`.
- **Scope: the gate, `nova doc` and the full Phase 3 LSP.** The LSP covers
  every row `40-TOOLING.md` §3.1 marks Phase 3. The REPL and the debugger
  move to a recorded backlog (§3, decision 10).
- **LSP engine: re-check first, and measure.** The server re-runs the front
  end on each edit, behind an interface that `salsa` could replace, against
  a latency budget. `salsa` is adopted only if the budget is missed.
- **VSCode extension: a `.vsix` now, the stores at the release.** During
  Phase 3 the extension ships as a `.vsix` in GitHub releases. At `v0.3.0` it
  is published to the VS Code Marketplace and Open VSX under a publisher
  account the user owns.
- **Order: foundations first** (§4).

## 2. Reality check — what Phase 3 needs that the repo lacks

1. **An installed `nova` cannot build.** `find_runtime_lib`
   (`crates/nova-driver/src/link.rs`) looks for the runtime library at
   `NOVA_RUNTIME_LIB`, or else beside the `nova` executable or one or two
   directories above it. `cargo install` and the release archives ship only
   the executable, so `nova build` and `nova test` fail on an installed copy.
   `nova run` is not affected: its JIT resolves runtime calls against the
   runtime linked into `nova` itself (`nova-codegen-cranelift` registers
   `nova_runtime::symbols()`).
2. **There is no project model.** There is no `nova.toml`, and no `nova new`
   or `nova init`. The CLI has `parse`, `run`, `build`, `check` and `test`
   (`crates/nova-cli/src/main.rs`), and they already treat the current
   directory as the project: the file argument defaults to `src/main.nova`.
3. **Modules are files beside the entry file** (ADR 0003): `import m` globs
   module `m`'s public names, and `m` is `m.nova` next to the entry. ADR 0003
   defers `import … as`, qualified `m::name` paths, nested module
   directories and re-exports, all on ADR 0025's backlog. Packages need two
   things that list does not include: a module identity per package, so that
   two packages can each have a `utils.nova`, and an `import <pkg>` form.
4. **The lexer discards ordinary comments.** `skip_trivia`
   (`crates/nova-lexer/src/lib.rs`) drops `//` and `/* */` comments, so a
   formatter built on today's tokens would delete them. `///` doc comments
   are kept, as `Token::DocComment`, which `nova doc` can use.
5. **The type checker answers whole programs.** It returns a `CheckResult`:
   one typed `nova_hir::Module` and its diagnostics, partly filled when there
   are errors (`crates/nova-typeck/src/lib.rs`). An editor needs lookups by
   position on top of that: what is at this offset, where it is defined, and
   what type it has. It also needs each diagnostic's byte span converted to
   LSP's line and UTF-16 column.
6. **The tooling crates are stubs.** `nova-fmt`, `nova-lsp`, `nova-pm`,
   `nova-doc`, `nova-test`, `nova-bundler` and `nova-codegen-wasm` each have
   a one-line `lib.rs` and a manifest. None of `lsp-server`, `lsp-types`,
   `tower-lsp`, `tokio`, `ureq`, `reqwest`, `tar`, `flate2`, `semver` or
   `sha2` is in `Cargo.lock`; `toml` and `serde` are. Each new dependency
   gets the scrutiny `httparse` and `ring` got (ADR 0019 for `httparse`, and
   the note in `crates/nova-runtime/Cargo.toml` for `ring`).

## 3. Key decisions (recommended now, confirmed before building)

Each is confirmed in its sub-phase's spec, and recorded in an ADR where it
departs from `40-TOOLING.md`.

1. **Getting the runtime into `nova` (3.0).** Candidates:
   - embed the runtime library when `nova` is built, and unpack it to a
     per-version cache on first use;
   - download the matching library from the GitHub release on first use;
   - ship it beside the binary in the release archives.

   Recommendation: a spike first, because embedding must also work when
   `nova-cli` is installed from crates.io, and Cargo's artifact dependencies
   are unstable. Prefer embedding.
2. **`nova.toml`'s first scope (3.0).** Parse `[package]`, `[dependencies]`
   (registry versions and path dependencies) and `[dev-dependencies]`, and
   warn on any other key. Features, `[[bin]]`, `[lib]`, `[build]` and git
   dependencies (`40-TOOLING.md` §4.1) wait.
3. **Package modules (3.3).** `import <pkg>` resolves to the dependency's
   `src/lib.nova`. Files inside a package import each other by file name, as
   today. A module's identity becomes the pair (package, file). Qualified
   paths and nested directories stay deferred.
4. **The index format (3.3).** `40-TOOLING.md` §4.4's Cargo-style sparse
   index, kept in a GitHub repo the user creates (for example
   `Sakeerin/nova-index`): one file per package, one JSON line per version
   with its SHA-256. Tarballs (`.nova-pkg`, gzip tar, §4.6) are attached as
   release assets. `nova publish` packages the code, checks it compiles,
   uploads the tarball and commits the index line through GitHub's REST API,
   with the token `nova login` stored. The index is read over plain HTTPS, so
   any static host can mirror it.
5. **The resolver (3.3).** Semver caret ranges, and one version of each
   package per build. `40-TOOLING.md` §4.3 asks for Cargo's semantics, where
   several major versions can coexist; that waits, with an ADR.
6. **The formatter (3.1).** A Wadler-style pretty printer (`40-TOOLING.md`
   §2.2), with comments re-attached from a lexer mode that keeps them. Three
   checks run over every `.nova` file in the repo: formatting twice changes
   nothing, the output parses to the same AST, and every comment survives,
   in order.
7. **The LSP protocol crate (3.2).** `40-TOOLING.md` §3.2 names `tower-lsp`,
   whose last release (0.20.0) was on 2023-08-11 (crates.io, checked
   2026-10-06). Recommendation: `lsp-server`, rust-analyzer's synchronous
   crate (0.10.0, 2026-07-16). It suits a re-check engine on a worker thread
   and adds no Tokio. The alternative is `tower-lsp-server`, a maintained
   async fork (0.23.0, 2026-09-11). Either is an ADR, and the same ADR
   records the engine: re-checking the program on each edit (§1) rather
   than `salsa`, which `40-TOOLING.md` §3.1 and §3.2 specify. The latency
   budget is set in 3.2's spec, for example 200 ms from an edit to its
   diagnostics on the largest example.
8. **The VSCode extension (3.2).** TypeScript with `vscode-languageclient`,
   and a TextMate grammar for colouring until 3.4's semantic tokens.
9. **`nova doc` (3.5).** `///` comments as Markdown, rendered with
   `pulldown-cmark` into a static site under `target/doc/` (`40-TOOLING.md`
   §5). 3.5's spec decides whether `nova test --doc` is in.
10. **The recorded scope (3.0).** ADR 0026 records the registry decision,
    which replaces the registry server of master spec Phase 3 position 3
    with the git-backed index, and what Phase 3 leaves out:
    - the REPL (`40-TOOLING.md` §6, master spec Phase 3 position 7) and the
      debugger (position 8);
    - `nova lint` (§8) and `nova bench`;
    - the Zed and Neovim extensions (§3.3);
    - the LSP rows §3.1 marks Phase 4 (inlay hints, code lens, call
      hierarchy).

## 4. Sub-phases (each gated, reviewed, and merged on the user's word)

Ordered so that each tool builds on the ground it needs. Each runs the
established loop: spec, plan, implementation, a fresh fact-check and review,
a PR, and a merge on the user's word.

### 3.0 — Foundations
- ADR 0026, the recorded scope (decision 10).
- The runtime inside `nova` (decision 1): an installed `nova` can `build`
  and `test` with no checkout; `NOVA_RUNTIME_LIB` still overrides.
- `nova.toml` parsing (decision 2), in `nova-pm`.
- `nova new <name>` and `nova init`, writing `40-TOOLING.md` §1.2's
  template.
- `run`, `build`, `check` and `test` find the project by walking up from the
  current directory to the nearest `nova.toml`. With none, they keep today's
  `src/main.nova` default.
- `nova test` also runs the files under `tests/` (`40-TOOLING.md` §7).
- `nova version`.
- **Gate:** CI on all three operating systems runs `cargo install --path
  crates/nova-cli --root <temp dir>`, which installs `nova` with nothing
  beside it. Then `nova new demo`, and `nova run`, `nova build` and
  `nova test` inside `demo`, all succeed with no environment variable set.

### 3.1 — Formatter
- A lexer mode that keeps comments.
- `nova fmt` for the project, a single file, `--check` and `--stdin`, in
  `40-TOOLING.md` §2.1's fixed style. `.editorconfig` is read for line
  endings and the final newline only (§2.4).
- **Gate:** decision 6's three checks pass on every `.nova` file in the
  repo. `nova fmt --check` exits non-zero on an unformatted file and zero
  after formatting. CI then runs `nova fmt --check` on `std/` and
  `examples/`, once 3.1's spec has confirmed that no test depends on their
  line numbers.

### 3.2 — LSP core and the VSCode extension
- `nova lsp` (decision 7) keeps open documents in sync and publishes
  diagnostics with exact LSP positions.
- Completion offers names in scope, fields and methods after `.` (std's
  included) and keywords.
- Document formatting calls `nova fmt`. Workspace roots come from
  `nova.toml` (`40-TOOLING.md` §3.2).
- `tools/vscode-nova/` (decision 8): language registration, the grammar and
  the LSP client. CI builds the `.vsix`.
- **Gate:** a Rust test drives `nova lsp` over stdio and gets a diagnostic
  for a planted error, completions that include a std method and a record
  field, and a formatted document. The extension's smoke test passes in CI.
  The edit-to-diagnostics latency is recorded on the development host, and
  CI asserts a looser bound to catch regressions.

### 3.3 — Packages and publishing
- Package modules (decision 3), with path dependencies first.
- `nova add`, `nova remove`, `nova update`, and `nova.lock` with exact
  versions and SHA-256 checksums (`40-TOOLING.md` §4.2).
- The resolver (decision 5), and fetching from the index (decision 4) into a
  local cache.
- `nova package`, `nova publish` and `nova login`.
- **Gate, in two parts:**
  - **in CI:** a clean runner installs `nova`, publishes a library to a
    local file-based index, and a second project adds it, imports it, builds
    and runs;
  - **once, by hand:** the same publish goes to the user's real index, and
    the transcript is recorded.

  With this sub-phase, every step of the Phase 3 gate can be run, installing
  from git rather than crates.io.

### 3.4 — LSP completeness
- Hover, with types and `///` docs.
- Go to definition, across files and into std and dependencies.
- Find references, and rename across files. Rename refuses items from std
  or a dependency.
- Code actions: fix-its from diagnostics that carry a suggested edit, and
  "organize imports" in `40-TOOLING.md` §2.1's order.
- Semantic highlighting, where the editor supports semantic tokens.
- **Gate:** the scripted LSP test covers each capability on a multi-file
  project with a dependency. The extension's smoke test also checks hover
  and go to definition.

### 3.5 — `nova doc`
- `nova doc [--open]` builds a static site under `target/doc/`: a page per
  module, item signatures from the type checker, a search index, and a light
  and dark theme (`40-TOOLING.md` §5).
- `nova test --doc`, as decision 9 settles.
- **Gate:** for `std/`, and for an example that uses a dependency, every
  public item has an anchor and a link check finds no broken link. std's
  site is attached to releases.

### 3.6 — The `v0.3.0` release
- All 21 crates published to crates.io at 0.3.0, in dependency order, under
  the user's account, after the names are checked again.
- The extension published to the VS Code Marketplace and Open VSX.
- Install scripts (`40-TOOLING.md` §10) that fetch the release binaries.
- **Gate:** the Phase 3 gate on a clean machine, recorded: `cargo install
  nova-cli` from crates.io, `nova new`, autocomplete in VSCode with the
  published extension, `nova fmt`, and `nova publish` to the real index.
- The close-out, as Phase 2's was: an ADR for Phase 3's boundary, a
  CHANGELOG `[0.3.0]`, and the `v0.3.0` tag, pushed only on the user's
  word.

## 5. Cross-cutting

- **Testing:** every tool is tested end to end through the `nova` binary, as
  `crates/nova-cli/tests/run_tests.rs` does today. The LSP is driven over
  stdio, and the extension through `vscode-test` in CI.
- **Dependencies:** each new crate builds on the workspace's minimum Rust,
  1.78 (CI's MSRV job), or the minimum moves with an ADR. Each is recorded
  with why it was chosen and what it adds to `Cargo.lock`.
- **Records:** `40-TOOLING.md` gets a dated note as each sub-phase lands, and
  each departure from it gets an ADR.
- **Security:** the token `nova login` stores is readable only by its owner,
  never printed, and sent only to GitHub's API. A downloaded package is
  checked against the index's SHA-256 before it is used. The index is
  append-only: a published version is never overwritten.
- **Publishing:** every publish to crates.io, an extension store or the real
  index is a stop for the user's word. CI publishes only to a local index.

## 6. Top risks

1. **Embedding the runtime may not survive `cargo install` from crates.io**,
   because Cargo's artifact dependencies are unstable. The spike in §7 comes
   first; the fallback is downloading the library from the matching release.
2. **The LSP on broken code.** The type checker was built for finished
   programs. Lookups are built on the typed module it already returns, which
   is partly filled when there are errors, and the LSP tests feed it broken
   code on purpose.
3. **The formatter losing a comment or changing meaning.** Decision 6's
   three checks run on every file, and test fixtures are never reformatted
   automatically.
4. **Package modules change module identity, which every compile depends
   on.** Path dependencies come first, and the full suite runs on all three
   operating systems.
5. **Publishing is public and partly irreversible:** crates.io cannot delete
   a crate, and the index is append-only. Every real publish is a stop for
   the user's word.

## 7. Suggested first step

The spike for decision 1: can `nova-cli` carry the runtime library and still
build when it is packaged and installed the way crates.io installs it? Try
it with `cargo package` and a local install before 3.0's spec is written.
Its answer picks between embedding and downloading, and 3.0's spec and
ADR 0026 follow.
