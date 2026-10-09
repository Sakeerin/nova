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
seven of them; the debugger appears only as the master spec's Phase 3
position 8 and in `14-CODEGEN.md` §9 (DWARF).

**The user's decisions, 2026-10-06:**
- **Registry: a git-backed index.** A public GitHub repo the user owns holds
  the package index, and package tarballs are attached to it as release
  assets. There is no registry server; a hosted one can serve the same index
  later.
- **Install: git now, crates.io at the release.** During Phase 3, Nova
  installs with `cargo install --git https://github.com/Sakeerin/nova
  nova-cli`; that repo is public. At the `v0.3.0` release, all 21 workspace
  crates are published to crates.io under the user's account, so the gate's
  `cargo install nova-cli` works literally; every one of the 21 names
  returned 404 on crates.io on 2026-10-06. Release binaries are offered
  throughout. Whichever way it is installed, `nova` must be able to build
  and test without a checkout (decision 1).

  **Amended 2026-10-07 (branch `phase-3-0-foundations`):** 3.0 adds a
  22nd crate, `nova-std` (`std/`). Its name returned 404 from crates.io's
  index on 2026-10-07. The README recommends `cargo install --locked
  --git …`, which builds with the tested lockfile.
- **Scope: the gate, `nova doc` and the full Phase 3 LSP.** The LSP covers
  every row `40-TOOLING.md` §3.1 marks Phase 3. The REPL and the debugger
  move to a recorded backlog (§3, decision 10).
- **LSP engine: re-check first, and measure.** The server re-runs the front
  end on each edit, behind an interface that `salsa` could replace, against
  a latency budget. `salsa` is adopted only if the budget is missed.
- **VSCode extension: a `.vsix` now, the stores at the release.** During
  Phase 3 the extension ships as a `.vsix` attached to GitHub releases. At
  `v0.3.0` it is published to the VS Code Marketplace and Open VSX under a
  publisher account the user owns.
- **Order: foundations first** (§4).

## 2. Reality check — what Phase 3 needs that the repo lacks

1. **An installed `nova` cannot build.** `find_runtime_lib`
   (`crates/nova-driver/src/link.rs`) looks for the runtime library at
   `NOVA_RUNTIME_LIB`, or else beside the `nova` executable or one or two
   directories above it. `cargo install` installs only the executable, so
   `nova build` and `nova test` fail on an installed copy. `nova run` is not
   affected: its JIT resolves runtime calls against the runtime linked into
   `nova` itself (`nova-codegen-cranelift` registers
   `nova_runtime::symbols()`), and std is compiled in.

   **Amended 2026-10-07 (branch `phase-3-0-foundations`):** resolved by
   3.0. A release-profile `nova` carries its runtime library and unpacks
   it to `$NOVA_HOME/runtime/` (ADR 0027).
2. **Nothing is released yet.** `.github/workflows/release.yml` builds
   `nova` for four targets on each `v*` tag and uploads the binaries as
   workflow artifacts; it creates no GitHub release, so there is nothing to
   download from a release page.

   **Amended 2026-10-07 (branch `phase-3-0-foundations`):** resolved in
   `release.yml` by 3.0: each `v*` tag now gets a GitHub release with
   per-target archives; the next tag is its first real run.
3. **There is no project model.** There is no `nova.toml`, and no `nova new`
   or `nova init`. The CLI has `parse`, `run`, `build`, `check` and `test`
   (`crates/nova-cli/src/main.rs`). `run`, `build` and `check` default their
   file to `src/main.nova` in the current directory, `test` always uses that
   file, and `parse` requires one.

   **Amended 2026-10-07 (branch `phase-3-0-foundations`):** 3.0 adds
   `nova.toml`, `nova new`, `nova init` and project discovery; dependencies,
   the lock file and `tests/` wait for 3.3.
4. **Modules are files beside the entry file** (ADR 0003): `import m` globs
   module `m`'s public names, and `m` is `m.nova` next to the entry. ADR 0003
   defers `import … as`, qualified `m::name` paths, nested module
   directories and re-exports, all on ADR 0025's backlog. Packages need two
   things that list does not include: a module identity per package, so that
   two packages can each have a `utils.nova`, and an `import <pkg>` form.
5. **Comments.** `skip_trivia` (`crates/nova-lexer/src/lib.rs`) drops `//`
   and `/* */` comments, so a formatter built on today's tokens would delete
   them. `///` doc comments are lexed as `Token::DocComment`, but the parser
   has no rule for them: a `///` before an item is a parse error today, and
   no `.nova` file in the repo contains one.
6. **The front end is built for finished programs.** The type checker
   returns a `CheckResult`: one typed `nova_hir::Module` and its
   diagnostics, partly filled when it reports type errors
   (`crates/nova-typeck/src/lib.rs`). But `FrontendContext::check`
   (`crates/nova-driver/src/lib.rs`) stops before resolution on any lex or
   parse error, and before type checking on any resolution error, so code in
   the middle of being typed yields no typed module at all. An editor also
   needs:
   - sources from unsaved buffers, where the front end reads every module
     from disk;
   - lookups by position: what is at this offset, where it is defined, what
     type it has;
   - diagnostic spans in LSP's line and UTF-16 column, and a fallback
     position for the diagnostics that carry no span (E0601, E0902);
   - a file to open for std, which exists only as strings compiled into
     `nova` (registered as `<std/core>` and so on).
7. **Diagnostics carry no suggested edits.** `Diagnostic`
   (`crates/nova-diagnostics/src/lib.rs`) has a severity, code, message,
   labels and notes, and no edit, though its doc comment mentions an
   optional suggestion.
8. **Packaging for crates.io is blocked in three more places.**
   - `nova-resolver` embeds std with `include_str!` paths that leave its
     crate directory (`../../../std/<module>/lib.nova`), which `cargo
     package` cannot carry.
   - Dependencies between workspace crates are paths with no `version`.
   - The workspace `repository` field is still the spec's placeholder,
     `https://github.com/novalang/nova`.

   **Amended 2026-10-07 (branch `phase-3-0-foundations`):** 3.0 removes the
   first blocker, because std is now the crate `nova-std`; the other two
   remain for 3.6.
9. **The tooling crates are stubs.** `nova-fmt`, `nova-lsp`, `nova-pm`,
   `nova-doc` and `nova-test`, like Phase 4's `nova-bundler` and
   `nova-codegen-wasm`, each have a one-line `lib.rs` and a manifest. None of
   `lsp-server`, `lsp-types`, `tower-lsp`, `tokio`, `ureq`, `reqwest`, `tar`,
   `flate2`, `semver` or `sha2` is in `Cargo.lock`; `toml` and `serde` are.
   Each new dependency gets the scrutiny `httparse` and `ring` got (ADR 0019
   for `httparse`, and the note in `crates/nova-runtime/Cargo.toml` for
   `ring`).

   **Amended 2026-10-07 (branch `phase-3-0-foundations`):** `nova-pm` now
   parses `nova.toml` (3.0), and `flate2` and `semver` are in `Cargo.lock`;
   `toml` no longer is.

## 3. Key decisions (recommended now, confirmed before building)

Each is confirmed in its sub-phase's spec, and recorded in an ADR where it
departs from the master spec or from `40-TOOLING.md`.

1. **Getting the runtime and std into a packaged `nova` (3.0).** The
   runtime library must reach an installed `nova`, and std's sources must
   survive `cargo package` (§2 items 1 and 8). Candidates for the runtime:
   - embed the library when `nova` is built, and unpack it to a per-version
     cache on first use;
   - download the matching library from a GitHub release on first use;
   - ship it beside the binary in release archives.

   Recommendation: a spike first, because Cargo's artifact dependencies are
   unstable, and embedding must work when `nova-cli` is installed from
   crates.io. Prefer embedding. If the spike shows embedding cannot work,
   3.0's spec picks another route and, with the user, adjusts 3.0's gate:
   a download needs a published release, which an unreleased commit lacks.

   **Spike result, 2026-10-06:** both work on stable Cargo, without
   artifact dependencies. The spike used small stand-ins for the runtime and
   the CLI, not the real crates.
   - **std:** `std/` itself becomes a crate, with a `Cargo.toml` and a
     `lib.rs` that embeds its own files. `cargo package` then keeps all 17
     `.nova` files, and its verification step builds the packaged copy on
     its own.
   - **The runtime:** the runtime crate declares `links`, and its build
     script reports its source directory. The CLI's build script copies that
     source, leaving out its packaged `Cargo.lock`, runs a nested offline
     `cargo build --release` of the copy into a target directory of its own,
     and embeds the library. An emulated crates.io install (packaged sources
     only, dependencies patched to their unpacked copies, then `cargo
     install`) produced a lone executable carrying a valid archive, on
     Windows and on Linux. Without the lockfile step, a pinned version that
     was not in the cache broke the nested build.
   - **The cost:** the release runtime library is 14.6 MB on Windows and
     30.2 MB on Linux, and 4.6 MB and 7.9 MB gzipped, against a 7.2 MB and
     an 8.2 MB `nova`. Embedding it compressed grows `nova` by about two
     thirds on Windows and doubles it on Linux.
   - **Untested:** macOS, the real runtime inside the nested build, the
     extra install time, and installs that cross-compile.

   So 3.0 embeds the runtime, compressed, and the download route stays a
   fallback.
2. **`nova.toml`'s first scope (3.0).** Parse `[package]`, `[dependencies]`
   (registry versions and path dependencies) and `[dev-dependencies]`, and
   warn on any other key. Features, `[[bin]]`, `[lib]`, `[build]` and git
   dependencies (`40-TOOLING.md` §4.1) wait. The template `nova new` writes
   contains only keys that are parsed.
3. **Package modules (3.3).** `import <pkg>` resolves to the dependency's
   `src/lib.nova`. Files inside a package import each other by file name, as
   today. A module's identity becomes the pair (package, file). Qualified
   paths and nested directories stay deferred.
4. **The index format (3.3).** `40-TOOLING.md` §4.4's Cargo-style sparse
   index, kept in a GitHub repo the user creates (for example
   `Sakeerin/nova-index`): one file per package, one JSON line per version
   with its SHA-256. Tarballs (`.nova-pkg`, gzip tar, §4.6) are attached as
   release assets. `nova publish` packages the code, checks it compiles with
   no warnings (§4.6), uploads the tarball and commits the index line through
   GitHub's REST API, with the token `nova login` stored. The index is read
   over plain HTTPS, so any static host can mirror it.
5. **The resolver (3.3).** Semver caret ranges, and one version of each
   package per build. `40-TOOLING.md` §4.3 asks for Cargo's semantics, where
   several major versions can coexist; that waits, with an ADR.
6. **The formatter (3.1).** A Wadler-style pretty printer (`40-TOOLING.md`
   §2.2), with comments re-attached from a lexer mode that keeps them, and
   `///` doc comments parsed and attached to the items they precede (the
   formatter must keep them, and `nova doc` and hover read them). Three
   checks run over every `.nova` file in the repo: formatting twice changes
   nothing, the output parses to the same AST, and every comment survives,
   in order. `nova fmt`'s single-file and `--stdin` modes follow
   `40-TOOLING.md` §2.3; the master spec's position 1 says "only `--check`",
   so 3.1 records that in an ADR.
7. **The LSP stack (3.2).** `40-TOOLING.md` §3.2 names `tower-lsp`, whose
   last release, 0.20.0, was on 2023-08-11. The candidates (crates.io,
   checked 2026-10-06):
   - `lsp-server`, rust-analyzer's synchronous crate: 0.10.0 on 2026-07-16,
     no declared minimum Rust, and no Tokio among its dependencies;
   - `tower-lsp-server`, a community fork of `tower-lsp`: 0.23.0 on
     2025-12-07 (a 0.24.0 release candidate on 2026-09-11), requiring Rust
     1.85, above the workspace's 1.78.

   Recommendation: `lsp-server`, which suits a re-check engine on a worker
   thread. The ADR that records the crate also records the engine:
   re-checking the program on each edit (§1) rather than the `salsa` that
   `40-TOOLING.md` §3.1 and §3.2 specify, and that ADR 0025 mapped to
   Phase 3. 3.2's spec sets the latency budget, for example 200 ms from an
   edit to its diagnostics on the largest example.

   **Amended 2026-10-08 (branch `phase-3-2-lsp-core`):** `lsp-server`
   0.10.0 declares no minimum Rust, but it is edition 2024, which needs
   Rust 1.85, and so is every release from 0.7.9 on. 3.2 pins 0.7.8 to
   keep the 1.78 minimum (ADR 0029). The budget is 200 ms.
8. **The VSCode extension (3.2).** TypeScript with `vscode-languageclient`,
   and a TextMate grammar for colouring until 3.4's semantic tokens.
9. **`nova doc` (3.5).** `///` comments as Markdown, rendered with
   `pulldown-cmark` into a static site under `target/doc/` (`40-TOOLING.md`
   §5). 3.5's spec decides whether `nova test --doc` is in. std has no `///`
   docs yet, so its site shows signatures only unless 3.5 writes them.
10. **The recorded scope (3.0).** ADR 0026 records the registry decision,
    which replaces the registry server of the master spec's Phase 3
    position 3 with the git-backed index, and what Phase 3 leaves out:
    - the REPL (`40-TOOLING.md` §6, master spec position 7) and the debugger
      (position 8);
    - `nova lint` (§8) and `nova bench`;
    - `nova install`, `nova clean`, `nova owner`, and `nova self update` and
      `uninstall` (§1.1, §4.5, §10.3);
    - §9's CI template and its `setup-nova` action;
    - §7's parallel test runs and TAP output, and §4.3's edition rules;
    - the Zed and Neovim extensions (§3.3);
    - the LSP rows §3.1 marks Phase 4 (inlay hints, code lens, call
      hierarchy), and the Phase 4 commands in §1.1 (`build --target wasm`,
      `bundle`, `dev`).

    A sub-phase's spec may pull one of these in, with a ruling.

## 4. Sub-phases (each gated, reviewed, and merged on the user's word)

Ordered so that each tool builds on the ground it needs. Each runs the
established loop: spec, plan, implementation, a fresh fact-check and review,
a PR, and a merge on the user's word.

### 3.0 — Foundations
- ADR 0026, the recorded scope (decision 10).
- The runtime and std reach an installed `nova` (decision 1): it can `build`
  and `test` with no checkout, and `NOVA_RUNTIME_LIB` still overrides.
- `nova.toml` parsing (decision 2), in `nova-pm`.
- `nova new <name>` and `nova init`, writing `40-TOOLING.md` §1.2's template
  with only the keys 3.0 parses, and with its test inside `src/main.nova`;
  `tests/` arrives with 3.3.
- `run`, `build`, `check` and `test` find the project by walking up from the
  current directory to the nearest `nova.toml`. With none, they keep today's
  `src/main.nova` default.
- `release.yml` creates a GitHub release for each `v*` tag, with its
  binaries attached.
- `nova version`, beside clap's `--version`.
- **Gate:** CI on all three operating systems runs `cargo install --path
  crates/nova-cli --root <temp dir>`, which installs `nova` with nothing
  beside it. Then `nova new demo`, and `nova run`, `nova build` and
  `nova test` inside `demo`, all succeed with no environment variable set.
- **Spec:** `docs/superpowers/specs/2026-10-07-phase-3-0-foundations-design.md`,
  built on the branch `phase-3-0-foundations`.

### 3.1 — Formatter
- A lexer mode that keeps comments, and `///` doc comments parsed and
  attached to items (decision 6).
- `nova fmt` for the project, a single file, `--check` and `--stdin`, in
  `40-TOOLING.md` §2.1's fixed style. `.editorconfig` is read for line
  endings and the final newline only (§2.4).
- **Gate:** decision 6's three checks pass on every `.nova` file in the
  repo, and a `///` before an item parses. `nova fmt --check` exits non-zero
  on an unformatted file and zero after formatting. CI then runs
  `nova fmt --check` on `std/` and `examples/`, once 3.1's spec has
  confirmed that no test depends on their line numbers.
- **Spec:** `docs/superpowers/specs/2026-10-07-phase-3-1-formatter-design.md`,
  built on the branch `phase-3-1-formatter`.

### 3.2 — LSP core and the VSCode extension
- The front end continues past errors: resolution and type checking run on
  a program with parse or resolution errors, and their partial results are
  kept.
- `nova lsp` (decision 7) works from in-memory sources, so unsaved edits are
  checked, re-checks when workspace files change on disk (§3.2's file
  watcher), and publishes diagnostics with LSP positions (§2 item 6).
- Completion offers names in scope, fields and methods after `.` (std's
  included) and keywords.
- Document formatting calls `nova fmt`. Workspace roots come from
  `nova.toml` (`40-TOOLING.md` §3.2).
- `tools/vscode-nova/` (decision 8): language registration, the grammar and
  the LSP client. CI builds the `.vsix`, and `release.yml` attaches it to
  the GitHub release.
- **Gate:** a Rust test drives `nova lsp` over stdio. It gets a diagnostic
  for a planted error, completions in a file that also has a syntax error,
  completions that include a std method and a record field, and a formatted
  document. The extension's smoke test passes in CI. The latency budget is
  met on the development host and recorded; CI asserts a looser bound to
  catch regressions. If 3.2 or 3.4 misses the budget, adopting `salsa`
  becomes its own step, decided with the user.
- **Spec:** `docs/superpowers/specs/2026-10-08-phase-3-2-lsp-core-design.md`,
  built on the branch `phase-3-2-lsp-core`.

### 3.3 — Packages and publishing
- Package modules (decision 3), with path dependencies first.
- `nova add`, `nova remove`, `nova update`, and `nova.lock` with exact
  versions and SHA-256 checksums (`40-TOOLING.md` §4.2).
- The resolver (decision 5), and fetching from the index (decision 4) into a
  local cache.
- `nova package`, `nova publish` and `nova login`.
- `nova test` also runs the files under `tests/` (§7), which import the
  package by name.
- **Gate, in two parts:**
  - **in CI:** a clean runner installs `nova`, publishes a library to a
    local file-based index, and a second project adds it, imports it, builds
    and runs;
  - **once, by hand:** the same publish goes to the user's real index, and
    the transcript is recorded.

  With this sub-phase, every step of the Phase 3 gate can be run, installing
  from git rather than crates.io.

  **Amended 2026-10-08:** 3.3 is two sub-phases, each with its own spec.
  3.3a, "Local packages", has package modules, path dependencies,
  `tests/`, `nova add --path`, `nova remove` and `nova new --lib`
  (`docs/superpowers/specs/2026-10-08-phase-3-3a-local-packages-design.md`,
  branch `phase-3-3a-local-packages`). 3.3b, "The index and publishing",
  has the rest of this entry, and its two-part gate.

  **Amended 2026-10-09:** 3.3b, "The index and publishing", is built
  (`docs/superpowers/specs/2026-10-09-phase-3-3b-index-and-publishing-design.md`,
  branch `phase-3-3b-index-publishing`; ADR 0031). Its CI gate runs in
  the `install` job on three systems. The by-hand publish to the real
  index waits for the user's word.

### 3.4 — LSP completeness
- Hover, with types and `///` docs.
- Go to definition, across files and into dependencies, and into std
  through std's sources written to a per-version cache.
- Find references, and rename across files. Rename refuses items from std
  or a dependency.
- Code actions: `Diagnostic` gains a suggested edit, the diagnostics that
  can suggest a fix carry one, and code actions offer them; and "organize
  imports" in `40-TOOLING.md` §2.1's order.
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
  site is attached to the GitHub release.

### 3.6 — The `v0.3.0` release
- The crates.io blockers of §2 item 8 are fixed, and the names are checked
  again. Then the crates are published at 0.3.0, in dependency order, under
  the user's account. `cargo install nova-cli` needs only `nova-cli` and the
  crates it depends on, so 3.6's spec settles with the user whether crates
  outside that set, such as Phase 4's stubs and `nova-bench-http`, are
  published too.
- The extension is published to the VS Code Marketplace and Open VSX.
- Install scripts (`40-TOOLING.md` §10) fetch the GitHub release's binaries.
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
  each departure from it or from the master spec gets an ADR.
- **Security:** the token `nova login` stores is readable only by its owner,
  never printed, and sent only to GitHub's API. A downloaded package is
  checked against the index's SHA-256 before it is used. The index is
  append-only: a published version is never overwritten.

  **Amended 2026-10-09 (branch `phase-3-3b-index-publishing`):** on
  Windows the token's file takes its directory's permissions, the user's
  profile by default, so "readable only by its owner" holds there only as
  far as the profile's do. `nova login` warns when `NOVA_HOME` is outside
  the profile (ADR 0031).
- **Publishing:** every publish to crates.io, an extension store or the real
  index is a stop for the user's word. CI publishes only to a local index.

## 6. Top risks

1. **Packaging `nova` for crates.io may break the runtime library and std's
   embedding** (§2 items 1 and 8), because Cargo's artifact dependencies are
   unstable and `cargo package` carries only files inside each crate. The
   spike found a route on stable Cargo (decision 1); 3.0 confirms it on
   macOS and with the real runtime.
2. **The LSP on broken code.** Today the front end stops before type
   checking on any parse or resolution error. 3.2 makes it continue, and its
   tests feed broken code on purpose.
3. **The formatter losing a comment or changing meaning.** Decision 6's
   three checks run on every `.nova` file in the repo, and test fixtures are
   never reformatted automatically.
4. **Package modules change module identity, which every compile depends
   on.** Path dependencies come first, and the full suite runs on all three
   operating systems.
5. **Publishing is public and, after a short window, permanent.** Under RFC
   3660, crates.io lets an owner delete a crate within 72 hours of
   publishing, and later only if it has a single owner, no reverse
   dependencies, and fewer than 100 downloads a month. Every crate `nova-cli`
   depends on would have a reverse dependency, so after 72 hours that set is
   effectively permanent. The index is append-only. Every real publish is a
   stop for the user's word.

## 7. Suggested first step

The spike for decision 1: can `nova-cli` carry the runtime library and std,
and still build programs, when it is packaged and installed the way
crates.io does it? Try it with `cargo package` and a local install before
3.0's spec is written. Its answer picks the route for decision 1, and 3.0's
spec and ADR 0026 follow.

**Done 2026-10-06:** the spike's answer is under decision 1. The next step
is 3.0's spec.
