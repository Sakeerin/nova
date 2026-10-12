# Phase 3.6a, "Release preparation": design

> Status: **draft for review** (2026-10-12). The first half of Phase 3.6,
> "The `v0.3.0` release" (`docs/phase-3-plan.md` §4, the 3.6 entry; §2
> item 8; §5; §6 risks 1 and 5). Phase 3.5 is complete: PRs #109 and
> #110, ADRs 0034 and 0035. On 2026-10-12 the user made seven decisions
> (§19, decisions 1 to 7):
> - 3.6 is two sub-phases. 3.6a prepares everything and publishes
>   nothing; 3.6b publishes, step by step on the user's word, runs the
>   gate, writes the records and pushes the `v0.3.0` tag.
> - crates.io gets the 19 crates `cargo install nova-cli` needs. The other
>   four get `publish = false`.
> - The crates are published by a workflow that runs only when the user
>   starts it, with a crates.io token the user adds as a secret.
> - The VS Code extension is published by hand, from the `.vsix` the
>   `v0.3.0` release attaches, under the publisher `sakeerin`.
> - The install scripts are release assets, fetched from
>   `github.com/Sakeerin/nova/releases/latest/download/`. They check the
>   archive against `SHA256SUMS`, and only the Windows script edits PATH.
> - The gate runs in CI on fresh Linux, macOS and Windows runners, and
>   once by the user, on a machine of their choosing, from a checklist.
> - The first package on the real index is `shapes`, from
>   `examples/05-geometry`.
>
> Branch `claude/loving-archimedes-frtf5q`.
>
> **Amended 2026-10-12, while planning:** a prototype of every part found
> that the project `nova new` makes fails `nova fmt --check`: its
> templates put one-expression bodies on three lines, which `nova fmt`
> joins into one. §8's format check would fail on every runner, and a
> user's first project would not be formatted. 3.6a fixes the templates
> (§7.1), the one change outside tests §12 now allows. And since no test
> runs `install.ps1`'s PATH code, `release.yml`'s Windows build now runs
> the script against its own archive with PATH on (§6.3). Decisions 24
> and 25.

After 3.6a, publishing `v0.3.0` is a sequence of steps someone only has
to start: a version bump, a tag, a workflow run, two store uploads and an
index repository. Every step that could fail has already run in CI
against the real crates, except the uploads themselves.

## 1. What 3.6a delivers

1. **Crates ready for crates.io.** Every crate `nova-cli` needs has a
   versioned dependency on each workspace crate it uses, the real
   repository URL, a README, and its licence texts. The other four crates
   are `publish = false`. The version is set once, in the workspace.
2. **The package route, proven before anything is uploaded.** A CI job,
   on Linux, macOS and Windows, packages the 19 crates, installs `nova`
   from the packages as crates.io would, and runs the install gate on it
   (§4). ADR 0027 said the package route would first run for real at
   3.6's crates.io install; with this job it runs on every pull request.
3. **A publish workflow nobody runs yet.** `Publish to crates.io` runs
   only from the GitHub UI, only on a `v*` tag that matches the version,
   repeats §4's check, then uploads the 19 crates in order and waits out
   crates.io's rate limit (§5).
4. **Install scripts.** `install.sh` and `install.ps1`, attached to every
   release, install the latest (or a chosen) `nova` from the release's
   archives after checking `SHA256SUMS` (§6).
5. **The extension, ready to list.** A CHANGELOG and the store fields the
   listing shows (§7).
6. **The gate, written.** A workflow that runs the Phase 3 gate on fresh
   runners from what was published, and the checklist for the user's own
   run (§8, §9).
7. **The runbook.** `docs/releasing.md`: every step of a release, in
   order, with the stops for the user's word (§9).

Nothing in 3.6a publishes, tags, or adds a secret. `Cargo.lock` does not
change (§3.2).

## 2. The starting point (read on 2026-10-12)

### The crates

- The workspace has 23 members (`Cargo.toml:1-3`, `members = ["crates/*",
  "std"]`). Each has `version = "0.2.0"` as a literal at its line 3; the
  workspace's `[workspace.package]` (`Cargo.toml:5-9`) has no `version`.
- `[workspace.package]` sets edition, `rust-version = "1.78"`, `license =
  "MIT OR Apache-2.0"`, and `repository =
  "https://github.com/novalang/nova"`, the master spec's placeholder
  (`nova-spec/00-MASTER-SPEC.md:17-19`).
- Every crate has a `description`, mostly one or two words ("Lexer",
  "Parser"). None sets `publish`, `readme`, `homepage`, `keywords` or
  `categories`. No crate directory has a README or a licence file; they
  are at the repository root only.
- Every dependency between workspace crates is `{ path = "..." }` with no
  version. `cargo package` refuses all but `nova-diagnostics`, `nova-std`
  and `nova-bench-http`: "all dependencies must have a version
  requirement specified when packaging" (plan §2 item 8, the two
  remaining blockers).
- **nova-cli's set.** Its normal and build dependencies reach 19
  workspace crates, itself included: nova-ast, nova-cli,
  nova-codegen-cranelift, nova-codegen-llvm (through
  `crates/nova-driver/Cargo.toml:27`), nova-diagnostics, nova-doc,
  nova-driver, nova-fmt, nova-hir, nova-index, nova-lexer, nova-lsp,
  nova-mir, nova-parser, nova-pm, nova-resolver, nova-runtime, nova-std
  (through `crates/nova-resolver/Cargo.toml:22`) and nova-typeck.
- **Outside it:** nova-bench-http (a load generator), and the one-line
  stubs nova-bundler, nova-codegen-wasm and nova-test. The set's
  dev-dependencies never reach them.
- **Order.** cargo 1.97's own `cargo publish --workspace --dry-run` goes:
  diagnostics, std, ast, lexer, runtime, parser, pm, fmt, index,
  resolver, hir, mir, typeck, codegen-cranelift, codegen-llvm, driver,
  doc, lsp, cli.
- **Files.** Every `include_str!`, `include_bytes!` and build-script path
  in the set's non-test code stays inside its crate (std's 17 modules,
  nova-doc's assets, nova-cli's `OUT_DIR` payload, nova-runtime's
  `gc_stack.c`).
- `.cargo/config.toml` sets `[resolver] incompatible-rust-versions =
  "fallback"`. It applies when the workspace packages a crate, so each
  packaged `Cargo.lock` is resolved as CI's is; it is not itself
  packaged.

### A trial on a scratch clone

With versions added to the normal path dependencies, and dev-dependencies
left as paths:
- all 19 packaged and verified offline in one `cargo package --workspace
  --exclude …`: 1 min 51 s, a 929 MB target directory, 1,372,310 bytes of
  tarballs (the largest, nova-runtime, 249 KB; crates.io allows 10 MB);
- cargo strips path-only dev-dependencies from the packaged manifests, so
  they need no version;
- `cargo package`'s verification builds in the dev profile, which does
  not embed the runtime. Forced to embed (`NOVA_EMBED_RUNTIME=1`), it
  fails: the nested runtime build cannot see cargo's in-process package
  overlay. So no `cargo publish` check exercises the embedding route;
- **an emulated crates.io install did.** The unpacked packages, a
  `.cargo/config.toml` above them with a `[patch.crates-io]` entry for
  each of the 18 libraries, and `cargo install --path <unpacked nova-cli>
  --offline`: 136 s, a 25.0 MB `nova` carrying a 7.5 MB payload, `nova
  version` saying `runtime: embedded`, and `nova new`, `build`, `run` and
  `test` passing. Linux only.

### Names, limits, versions

- On 2026-10-12, all 23 names were free on crates.io (404 from its API and
  its sparse index). `nova` is taken (bbqsrc, 0.5.4), and so is
  `nova-registry` (NovaProto, 0.1.0); neither is used.
- crates.io's rate limit on new crates: a burst of 5, then 1 every 10
  minutes. The 19 take about 2 h 20 min.
- After 72 hours a published crate can be deleted only with a single
  owner, no reverse dependencies and under 100 downloads a month (plan §6
  risk 5). For the 19, that makes a publish permanent.
- The version is read from nova-cli's manifest by `release.yml`
  (`:62,137,164`, its first `version = "..."` line), which also requires
  the tag to be `v<version>` (`:178-183`) and CHANGELOG to have a section
  for it (`.github/scripts/release-notes.sh:15-25`).
- `tools/vscode-nova/package.json:5` says `0.2.0`, and
  `crates/nova-cli/tests/vscode_extension.rs:43` requires it to equal
  nova-cli's version; `:45` pins the publisher `sakeerin`.

### Releases and installs

- `release.yml` (3.0, 3.2, 3.5a) builds four targets
  (x86_64-unknown-linux-gnu, x86_64-apple-darwin cross-built and never
  run, aarch64-apple-darwin, x86_64-pc-windows-msvc), packs each as
  `nova-<version>-<target>.tar.gz` (`.zip` on Windows) holding one
  directory with `nova`, `README.md` and both licences, smoke-tests three
  of them with `gate.sh`, adds the `.vsix` and std's site, writes one
  `SHA256SUMS`, and on a `v*` tag runs `gh release create --verify-tag`.
  No secret is used anywhere.
- That path has never run on a tag: `v0.2.0` and earlier were pushed
  before 3.0 rewrote the workflow, and `v0.2.0` has no GitHub release.
- Release binaries are release builds, which embed the runtime
  (`crates/nova-cli/build_support.rs:14-21`). A lone `nova` still needs
  a system linker for `nova build` and `nova test` (`cc`, or the Visual
  Studio Build Tools on Windows; `crates/nova-driver/src/link.rs:153-191`).
- No install script exists. `40-TOOLING.md` §10 names
  `https://novalang.dev/install.sh` and `install.ps1`, on a domain that
  does not exist, installing into `~/.nova/bin/` and adding it to PATH.
  `~/.nova` is also `NOVA_HOME`'s default (`crates/nova-pm/src/home.rs:12-24`).
- The extension (`tools/vscode-nova/`) has a README, `LICENSE`, both
  licence texts and `.vscodeignore`; no CHANGELOG and no icon. CI and
  `release.yml` package it with `@vscode/vsce` 4.0.0, which warns that it
  should be bundled (362 files, 585 KB).
- The real index, `Sakeerin/nova-index`, does not exist yet (3.3b spec
  §11.2's by-hand step is pending).
- The tests' loopback HTTP server, `crates/nova-index/tests/support/mod.rs`,
  binds 127.0.0.1, answers from a handler and records each request;
  nova-cli's registry tests include it by path.

## 3. The crates

### 3.1 Which are published

The 19 crates of §2's set are published. `nova-bench-http`,
`nova-bundler`, `nova-codegen-wasm` and `nova-test` get `publish = false`,
so `cargo package` and `cargo publish` skip them, and nothing reserves
their names (decision 2).

### 3.2 Versions

- `[workspace.package]` gains `version`, and every member says
  `version.workspace = true`. The value stays `0.2.0` in 3.6a; 3.6b's
  release commit sets `0.3.0`.
- `[workspace.dependencies]` gains an entry for each of the 19 published
  crates: `nova-ast = { path = "crates/nova-ast", version = "=0.2.0" }`.
  Each member's normal and build dependency on one says `{ workspace =
  true }`.
- **The requirement is exact (`=`).** The crates are released together
  and their APIs between each other are not kept stable, so `nova-cli
  0.3.0` must never be built against `nova-driver 0.3.1`. An exact pin
  holds that even for `cargo install` without `--locked`.
- **Dev-dependencies stay `{ path = … }`.** Cargo strips them from the
  packaged manifest, so they add no publish-order edges. The cost: a
  published crate's own tests do not build from its tarball, which
  neither crates.io nor any publish check needs.
- **`Cargo.lock` does not change.** It records each member's version and
  its dependencies' names, which 3.6a leaves as they are; each task's
  `git diff --stat` shows it untouched. The user's rule stands: 3.6b's
  bump, which does change the lock's member versions, asks first.
- A bump is then: the workspace version, the 19 pins (all in the root
  `Cargo.toml`), the extension's `package.json` and `package-lock.json`,
  and `Cargo.lock`. A test holds the 19 pins and the extension to the
  workspace version (§11.1).

### 3.3 Metadata

- `repository` becomes `https://github.com/Sakeerin/nova`, and the
  workspace gains `homepage` with the same URL.
- **READMEs.** `crates/nova-cli/README.md` is nova-cli's crates.io page:
  what Nova is, `cargo install nova-cli --locked`, the install scripts,
  what an installed `nova` needs (a linker for `nova build`), and links to
  the repository. The other 18 share `crates/README.md` through `readme =
  "../README.md"` (`"../crates/README.md"` for std): a few lines saying
  the crate is part of the `nova` toolchain, that its API is internal and
  changes with every release, and how to install `nova`. Cargo packages a
  README outside the crate's directory (checked with cargo 1.97 on
  2026-10-12).
- **Descriptions.** Each published library's description names Nova and
  says the crate is internal, for example "The Nova language's lexer
  (internal to the nova toolchain)". nova-cli's: "The Nova programming
  language: compiler, formatter, language server, package manager and
  documentation generator".
- **Keywords and categories,** on nova-cli only: keywords `nova`,
  `programming-language`, `compiler`, `lsp`, `formatter`; categories
  `compilers`, `command-line-utilities`, `development-tools`.
- **Licence texts.** Each of the 19 crate directories gets copies of
  `LICENSE-MIT` and `LICENSE-APACHE`, byte for byte the root's, so every
  tarball carries the texts both licences require. A test keeps them
  identical (§11.1). Copies, not symlinks: Windows checkouts turn a
  symlink into a text file.

### 3.4 What stays

`rust-version` stays 1.78; every manifest field 3.6a adds is older. The
crates keep their names. `nova-std` keeps its directory, `std/`.

## 4. The package route, in CI

`.github/scripts/package-gate.sh WORKDIR`, run by a new `ci.yml` job,
"Packaged nova (os)", on Linux, macOS and Windows:

1. **Package and verify the 19** with `cargo package --locked` and a `-p`
   for each, into `WORKDIR/target`. This is crates.io's own check, run
   here before any upload.
2. **Their lockfiles agree with the workspace's.** Each packaged crate's
   `Cargo.lock` names only packages the workspace's `Cargo.lock` has, at
   the same versions. So `cargo install nova-cli --locked` builds what CI
   built.
3. **Install as crates.io would.** Unpack the 19 tarballs, write a
   `.cargo/config.toml` above them with a `[patch.crates-io]` entry for
   each of the 18 libraries, and `cargo install --path <unpacked
   nova-cli> --offline --root WORKDIR/root` from there, a release build
   that embeds the runtime through the package route: the nested runtime
   build reads the same config.
4. **The installed `nova` passes `gate.sh --test`**, as CI's install job
   requires of a `cargo install --path` copy: `runtime: embedded`, `nova
   new`, `run`, `build`, `test`.

It prints each step's duration. The job's cost is about the install job's
plus the packaging, run in parallel with the rest of CI. The emulation's
limit, recorded in ADR 0036: crates.io serves the real dependencies, and
the nested build's offline resolution of the runtime's own dependencies
from `~/.cargo/registry` first runs for real at 3.6b's gate.

## 5. Publishing to crates.io

### 5.1 The workflow

`.github/workflows/publish-crates.yml`, "Publish to crates.io":
- `on: workflow_dispatch` only, with one input, `version`. It never runs
  on a push, a pull request or a tag.
- One job on `ubuntu-latest`, in the environment `crates-io` (the user
  can require their own approval there), `timeout-minutes: 345`,
  `concurrency` so two runs never overlap, `permissions: contents: read`.
- **It refuses** unless it runs on the tag `v<version>` (`github.ref` is
  `refs/tags/v<version>`) and the workspace's version is `<version>`. So
  crates.io receives exactly the tagged tree, the one the GitHub release
  was built from.
- It runs §4's `package-gate.sh` first, then
  `.github/scripts/publish-crates.sh <version>` with
  `CARGO_REGISTRY_TOKEN` from the secret of that name. The token is
  never printed.

### 5.2 The script

`publish-crates.sh VERSION`:
- **The order** is a list in the script, the order of §2. A test holds it
  to the workspace: it is exactly the crates that are not `publish =
  false`, and each comes after every workspace crate it depends on
  through a normal or build dependency (§11.1).
- **Each crate:** if crates.io already has it at `VERSION` (its sparse
  index file lists that version), it prints `skip <crate> <version>
  (already on crates.io)` and goes on. Otherwise it runs `cargo publish
  --locked -p <crate>`, which also waits until the index shows the crate
  before the next one needs it.
- **The rate limit.** When cargo's output says crates.io refused for too
  many new crates (HTTP 429, "try again after"), it prints `rate limited:
  waiting <n> s` and tries the same crate again, up to 30 times; any
  other failure stops the run with cargo's output.
- So a run that stops can be started again from the same tag: it skips
  what is up and continues.
- `CARGO` (the cargo to run), `NOVA_PUBLISH_INDEX` (the sparse index's
  base URL, default `https://index.crates.io`) and `NOVA_PUBLISH_WAIT`
  (seconds, default 660) can be set; the tests set all three.
- It ends with one line per crate: published, or skipped.

### 5.3 What the user does (3.6b)

The steps in `docs/releasing.md` (§9): make a crates.io token, scoped to
`publish-new` and `publish-update` for `nova-*` crates, and set to expire
in a day; add it as the `crates-io` environment's secret
`CARGO_REGISTRY_TOKEN`; start the workflow on the tag; when it is done,
revoke the token and delete the secret.

## 6. Install scripts

`tools/install/install.sh` (POSIX `sh`, for Linux and macOS) and
`tools/install/install.ps1` (Windows PowerShell 5.1 and PowerShell 7).
Each release attaches both, so these always fetch the latest release's
own script:

```sh
curl -sSfL https://github.com/Sakeerin/nova/releases/latest/download/install.sh | sh
```

```powershell
irm https://github.com/Sakeerin/nova/releases/latest/download/install.ps1 | iex
```

### 6.1 What they do

1. **Pick the archive.** Linux x86_64 → `x86_64-unknown-linux-gnu`;
   macOS arm64 → `aarch64-apple-darwin`, x86_64 →
   `x86_64-apple-darwin`; Windows x64 → `x86_64-pc-windows-msvc`.
   Anything else stops with "no ready-built nova for <os> <arch>;
   install it with: cargo install nova-cli --locked".
2. **Pick the version:** `NOVA_VERSION` if set, else the latest release,
   read from where `<base>/latest` redirects (`…/releases/tag/v<version>`).
   GitHub's latest skips prereleases.
3. **Download** `nova-<version>-<target>.<ext>` and `SHA256SUMS` from
   `<base>/download/v<version>/` into a temporary directory, removed on
   exit.
4. **Check** the archive's SHA-256 against its line in `SHA256SUMS`
   (`sha256sum` or `shasum -a 256`; `Get-FileHash`). A missing line or a
   mismatch stops with an error naming the file, and nothing is
   installed.
5. **Install** `nova` (`nova.exe`) into `$NOVA_HOME/bin`, by default
   `~/.nova/bin` (`%USERPROFILE%\.nova\bin`): the directory 40-TOOLING
   §10.1 names, under the home `nova` already uses. It writes beside the
   old binary and renames over it, so a failed install leaves the old one.
6. **Confirm** by running the installed `nova version` and printing it.
7. **PATH** (decision 5):
   - `install.ps1` adds the directory to the user's PATH (the registry's
     user `Path`, as rustup does) unless it is there, says so, and sets it
     for the current session. `NOVA_INSTALL_NO_PATH=1` skips this.
   - `install.sh` edits no file. When the directory is not on PATH it
     prints the line to add: `export PATH="$HOME/.nova/bin:$PATH"`.
- `<base>` is `NOVA_INSTALL_BASE` if set (a mirror, or the tests'
  loopback server), else `https://github.com/Sakeerin/nova/releases`.
- Each prints `nova: ` lines to say what it is doing; errors go to
  stderr and exit non-zero.
- `install.ps1` turns on TLS 1.2 for PowerShell 5.1, and uses
  `-UseBasicParsing`.
- Neither needs more than `curl`, `tar` and a SHA-256 tool, or what
  Windows PowerShell has.

### 6.2 What they leave out

`nova self update` and `self uninstall` stay out (ADR 0026); running the
script again updates, and deleting `~/.nova` uninstalls. No signature
beyond `SHA256SUMS` over HTTPS. No linux-aarch64 or musl build, no
codesigned macOS binary (§17).

### 6.3 In the release

`release.yml`:
- `prepare` copies both scripts into the release's assets, and its count
  check expects them, so `SHA256SUMS` covers them;
- a new step in `prepare` runs `install.sh` against the release's own
  Linux archive, served from `release/dist` over loopback, with
  `NOVA_VERSION` set: the real archive's layout, checksum and binary;
- every place that reads the version reads the workspace's, since
  nova-cli's manifest no longer holds it;
- its pull-request trigger also covers `tools/install/**`;
- the Windows build job runs `install.ps1`, in Windows PowerShell 5.1,
  against its own archive served over loopback, with PATH on: the
  installed `nova version` must say `runtime: embedded`, and the user's
  PATH in the registry must then hold the install directory. The runner
  is thrown away after the job, so its PATH may change; this is the only
  place the PATH code runs before a user's machine.

## 7. The extension

- `tools/vscode-nova/CHANGELOG.md`, which the Marketplace shows: 0.3.0
  is the first published version, with what the extension does.
- `package.json` gains `homepage`, `bugs` and `keywords`. The publisher
  stays `sakeerin`.
- No icon (the listing shows a default; one can come later), and no
  bundler: bundling would add a new npm dependency, and the 585 KB
  package works as it is.
- **The texts that name how to install `nova`** (the extension's README
  and the error `src/extension.ts:46-55` shows) change in 3.6b's release
  commit, with the root README, when what they say becomes true.

The by-hand publish (3.6b, decision 4): the user creates the Marketplace
publisher `sakeerin` and the Open VSX namespace `sakeerin`, then runs
`npx @vscode/vsce publish --packagePath nova-vscode-0.3.0.vsix` and
`npx ovsx publish nova-vscode-0.3.0.vsix` with the release's file. No
store token goes into GitHub.

### 7.1 The templates

`crates/nova-cli/src/template.rs`'s program and library templates, and
the `tests/` file `nova new` writes beside a library, become exactly what
`nova fmt` writes: a one-expression body on one line, as in `fn
greeting() -> String { "Hello, Nova!" }`. The programs behave as before:
`gate.sh`, `packages-gate.sh` and `registry-gate.sh` check their output,
and it does not change. A test in `crates/nova-cli/tests/project.rs`
runs `nova new`, `nova new --lib`, `nova init` and `nova init --lib`, and
requires `nova fmt --check` to pass on each project; the constants there
that pin the templates follow. The 3.0 spec, which quotes the template,
gets a dated note.

## 8. The gate, in CI

`.github/workflows/release-gate.yml`, "Release gate", `on:
workflow_dispatch` with an input `version`. It installs only what has been
published, on fresh `ubuntu-latest`, `macos-latest` and `windows-latest`
runners:
- **from crates.io:** `cargo install nova-cli --version =<version>
  --locked`, then `gate.sh --test`, `packages-gate.sh` and
  `doc-gate.sh`, as CI's install job runs them, and `nova fmt --check` on
  the project `nova new` made;
- **from the install script:** the release's `install.sh` (or
  `install.ps1`) with `NOVA_VERSION=<version>`, then `gate.sh --test` on
  the installed `nova`.

3.6a writes it; it first runs in 3.6b, after the crates and the release
are up. It never publishes. Its run's log, with the user's own run (§9),
is the gate's record.

## 9. The runbook: `docs/releasing.md`

The steps of a release, for `v0.3.0` and after, each marked where it
stops for the user's word:
1. **The release commit:** the version bump (§3.2) and CHANGELOG's
   `[<version>]` section, in a pull request, merged by rebase.
2. **The tag** `v<version>`, annotated, on that commit, pushed by the
   user's word. `release.yml` creates the GitHub release; check its
   assets and `SHA256SUMS`.
3. **crates.io:** §5.3.
4. **The extension:** §7's by-hand steps.
5. **The index** (first time only): create `Sakeerin/nova-index`
   (public, `config.json` as 3.3b spec §3.2 gives it) and a token that can
   write to it; `gh auth token | nova login`.
6. **The gate:** start `Release gate`; then the user's run on a clean
   machine (a fresh VM or a new user account), recorded: `cargo install
   nova-cli --locked`, `nova new`, editing in VS Code with the published
   extension until completion offers a field, `nova fmt`, and `nova
   publish` of `shapes` to the real index, then `nova add shapes` in a
   second project and `nova run`. A transcript template, with local paths
   and names removed, says what to keep.
7. **If something fails:** a crate rejected mid-run (rerun from the tag
   once fixed in a new patch version; every crate moves to it, since the
   pins are exact), a bad archive (delete the GitHub release, fix, tag the
   next patch version), what can be yanked and when (72 hours).

## 10. Records

- **ADR 0036, "Publishing nova":** the 19 crates and the four left out;
  exact pins and path dev-dependencies; the workspace version; the
  package gate and what it cannot prove; the publish workflow, the tag
  check and the token's life; the install scripts, their URL, PATH and
  checksum rules; the extension by hand; the gate's two halves. It names
  this spec and ADRs 0026, 0027 and 0031.
- **Dated notes:** `docs/phase-3-plan.md` §4's 3.6 entry (two
  sub-phases; 3.6a built) and §2 item 8 (the blockers fixed);
  `nova-spec/40-TOOLING.md` §10 (the scripts' URLs, install directory,
  PATH and checksum rules; usable from `v0.3.0`);
  `nova-spec/50-TESTING.md` §4.3 (the targets built, no linux-aarch64;
  the scripts attached; no `novalang.dev`); ADR 0027 (the package route
  runs in CI).
- `CHANGELOG.md` `[Unreleased]`: Added (install scripts, the publish and
  gate workflows, the package check), Changed (crate metadata, `publish =
  false`, the workspace version).
- The root README's install section changes in 3.6b (§7).

## 11. Testing

### 11.1 The manifests (`crates/nova-cli/tests/release_manifests.rs`)

From `cargo metadata --no-deps --offline` and the files:
- exactly the four crates of §3.1 are `publish = false`;
- every other member's normal and build dependencies on workspace crates
  are exact pins equal to the workspace version, and no workspace
  dependency lacks a version;
- `publish-crates.sh`'s list is the published set, in an order where each
  crate follows its normal and build dependencies;
- every published crate has a README, `LICENSE-MIT` and `LICENSE-APACHE`
  identical to the root's, the real repository URL, and a description
  that names Nova;
- `tools/vscode-nova/package.json` and `package-lock.json` carry the
  workspace version.

### 11.2 The publish script (`crates/nova-cli/tests/publish_script.rs`, Unix)

`publish-crates.sh` against a fake `cargo` (a script that records its
arguments and answers as told) and a sparse index served by the loopback
server:
- publishes the 19 in the list's order;
- skips a crate the index already lists at the version, and publishes it
  when the index lists only others;
- on a rate-limit answer waits (`NOVA_PUBLISH_WAIT=0`) and retries the
  same crate, then goes on;
- stops at any other failure, with cargo's output, publishing nothing
  after it;
- never prints `CARGO_REGISTRY_TOKEN`'s value.

### 11.3 The install scripts (`crates/nova-cli/tests/install_scripts.rs`)

A fake release (the host's archive, built with the system's `tar` or
PowerShell's `Compress-Archive` around the test's own `nova` binary, and
its `SHA256SUMS`) served by the loopback server, `NOVA_HOME` a fresh
directory with a fixed name; `install.sh` on Linux and macOS,
`install.ps1` (with `NOVA_INSTALL_NO_PATH=1`) on Windows:
- installs the latest release, found through the `latest` redirect, into
  `$NOVA_HOME/bin`, and prints the installed `nova version`;
- with `NOVA_VERSION`, fetches that version and never asks for `latest`;
- a checksum mismatch, and an archive missing from `SHA256SUMS`, install
  nothing and exit non-zero, naming the file;
- running it again replaces the binary;
- `install.sh` prints the PATH line only when the directory is not on
  PATH;
- `install.sh` on an unknown system (a fake `uname` first on PATH) names
  `cargo install nova-cli --locked`;
- every request went to the loopback server.

### 11.4 In CI

- "Packaged nova (os)", §4, on three systems.
- `release.yml`'s pull-request run: eight assets and `SHA256SUMS`, and
  `install.sh` against the real Linux archive (§6.3).
- The full suite, clippy, the 1.78 check and fmt as today.

### 11.5 Mutants

Each must fail the named test:
1. A crate's pin set to `0.2.1` → §11.1.
2. `publish = false` dropped from `nova-test` → §11.1.
3. Two crates swapped in the publish list (lexer before diagnostics) →
   §11.1.
4. The publish script's "already published" check removed → §11.2.
5. The script retries every failure, not just the rate limit → §11.2.
6. `install.sh`'s checksum comparison removed → §11.3.
7. `install.ps1` ignoring `NOVA_VERSION` → §11.3 (Windows, in CI).
8. A licence copy edited in one crate → §11.1.
9. A template back on three lines → §7.1's test.

## 12. Crates and files touched

- Every `Cargo.toml` (versions, pins, metadata, `publish = false`); new
  `crates/README.md`, `crates/nova-cli/README.md`, 38 licence copies.
- New: `.github/scripts/package-gate.sh`, `.github/scripts/publish-crates.sh`,
  `.github/workflows/publish-crates.yml`, `.github/workflows/release-gate.yml`,
  `tools/install/install.sh`, `tools/install/install.ps1`,
  `docs/releasing.md`, `tools/vscode-nova/CHANGELOG.md`.
- Changed: `ci.yml` (the package job), `release.yml` (§6.3),
  `tools/vscode-nova/package.json`.
- Tests: three new files in `crates/nova-cli/tests/`.
- No Rust code outside tests changes but the templates (§7.1). No new
  crate, no new npm package, no `Cargo.lock` change.

## 13. The gate for 3.6a

- CI green on three systems, with "Packaged nova (os)": the 19 package
  and verify, their lockfiles agree with the workspace's, the emulated
  crates.io install embeds the runtime and passes `gate.sh --test`.
- §11's tests pass where they run (the publish script's on Linux and
  macOS, `install.sh`'s on Linux and macOS, `install.ps1`'s on Windows).
- `release.yml`'s pull-request run is green with the scripts attached and
  `install.sh` installing the real Linux archive.
- §11.5's mutants each fail their test.
- Nothing is published, no tag is pushed, no secret is added, and
  `Cargo.lock` is unchanged.

## 14. Risks

1. **The real crates.io install differs from the emulation.** The nested
   runtime build resolves offline from `~/.cargo/registry`, which only a
   real install fills. *Mitigation:* the package gate on three systems;
   the release gate's crates.io install on three systems right after
   publishing; the 72-hour window, while a broken set can still be
   deleted, is when the gate runs.
2. **A crate rejected mid-run** (a name taken since 2026-10-12, a crates.io
   check the package gate does not make). *Mitigation:* the script stops at
   the first failure and can resume; the runbook's patch-version path.
3. **The rate limit is not what the source says.** *Mitigation:* the
   script waits and retries on any 429; the job's 345 minutes cover 19
   crates at one every 15 minutes, after the package check.
4. **The install scripts on shells and PowerShell versions CI does not
   run.** *Mitigation:* POSIX `sh`, no bashisms (tested by running them
   with `sh`, which is `dash` on Ubuntu); `install.ps1` on CI's Windows
   PowerShell 5.1; the release gate runs both from the real release.
5. **The tarballs carry something they should not** (a stray file in a
   crate directory). *Mitigation:* the package gate prints each crate's
   file count and size; the plan reviews the lists once.

## 15. Not in 3.6a

- Anything 3.6b does: the bump to 0.3.0, CHANGELOG `[0.3.0]`, the tag,
  every publish, the index repository, running the gate, the close-out
  ADR for Phase 3's boundary, the README's install section.
- linux-aarch64 and musl builds, codesigning and notarizing macOS
  binaries, a Homebrew or winget package.
- `nova self update` and `uninstall` (ADR 0026).
- An icon and a bundled extension.

## 16. 3.6b, in outline

3.6b's spec settles each step; this is the order 3.6a prepares for, each
a stop for the user's word:
1. The release commit, in a pull request (it changes `Cargo.lock`'s
   member versions, so it asks first).
2. The tag `v0.3.0`; `release.yml` creates the release.
3. `Publish to crates.io` on the tag.
4. The extension, by hand.
5. The index repository and `nova login`.
6. `Release gate`, and the user's own run.
7. The records: an ADR for Phase 3's boundary with the gate's record,
   ADR 0031's measurements from the first real publish, dated notes, and
   the README.

The tag comes before crates.io because a GitHub release can be deleted
and made again; a crate cannot.

## 17. Ordering

1. The manifests (§3) with §11.1's test.
2. The package gate (§4) and its CI job.
3. The publish script (§5.2) with §11.2's test, then the workflow (§5.1).
4. The install scripts (§6) with §11.3's test, then `release.yml`
   (§6.3).
5. The extension (§7).
6. The release gate workflow (§8) and the runbook (§9).
7. The records (§10).

## 18. Open for the plan

- The exact `cargo package` and `cargo install` commands of §4, from the
  scratch trial.
- How `install.ps1` reads the `latest` redirect in both PowerShell 5.1 and
  7 (`HttpClient` with redirects off).
- The fake `cargo`'s protocol in §11.2.

## 19. Decisions

By the user, 2026-10-12:

1. 3.6 is two sub-phases: 3.6a prepares and publishes nothing; 3.6b
   publishes on the user's word, runs the gate, writes the records and
   tags `v0.3.0`.
2. crates.io gets the 19 crates `nova-cli` needs; the four others are
   `publish = false`.
3. A workflow the user starts publishes the crates, with a token the user
   adds as a secret.
4. The extension is published by hand from the release's `.vsix`, under
   `sakeerin`.
5. The install scripts are release assets; they check `SHA256SUMS`;
   `install.ps1` sets the user PATH, `install.sh` prints the line.
6. The gate runs in CI on three systems from what was published, and once
   by the user from a checklist.
7. The first package on the real index is `shapes`.

Made in this spec, for review:

8. The version moves into `[workspace.package]`.
9. Workspace dependencies are exact pins (`=`), declared once in
   `[workspace.dependencies]`.
10. Dev-dependencies stay paths, so cargo strips them.
11. `repository` and `homepage` are `https://github.com/Sakeerin/nova`.
12. nova-cli has its own README; the 18 libraries share one.
13. Every published crate carries both licence texts, as copies.
14. Descriptions name Nova; keywords and categories on nova-cli only.
15. The package route is checked on every pull request, on three systems,
    by an emulated crates.io install.
16. The publish workflow runs only on the tag matching the version, and
    repeats the package check first.
17. The publish script skips what crates.io has and waits out the rate
    limit, so a run can resume.
18. The install scripts install into `$NOVA_HOME/bin` (`~/.nova/bin`).
19. The install scripts' base URL can be overridden (`NOVA_INSTALL_BASE`).
20. The extension gets a CHANGELOG and store fields; no icon, no bundler.
21. Texts that name how to install `nova` change in 3.6b's release commit.
22. `docs/releasing.md` is the runbook for this and later releases.
23. The tag comes before the crates.io publish.

Amended while planning (2026-10-12):

24. `nova new`'s templates are written as `nova fmt` writes them, with a
    test that every new project passes `nova fmt --check` (§7.1).
25. `release.yml`'s Windows build runs `install.ps1` with PATH on, against
    its own archive (§6.3).
