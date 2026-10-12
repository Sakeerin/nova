# ADR 0036 — Publishing nova

## Status

Accepted, 2026-10-12 (Phase 3.6a, branch `claude/loving-archimedes-frtf5q`;
spec `docs/superpowers/specs/2026-10-12-phase-3-6a-release-preparation-design.md`).

## Context

The phase plan's 3.6 entry is the `v0.3.0` release: the crates on
crates.io, in dependency order, under the user's account; the extension
on the VS Code Marketplace and Open VSX; install scripts that fetch the
GitHub release's binaries (`40-TOOLING.md` §10); and the Phase 3 gate on a
clean machine, recorded: `cargo install nova-cli` from crates.io, `nova
new`, completion in VS Code with the published extension, `nova fmt`, and
`nova publish` to the real index.

Before 3.6a:
- Two of the phase plan's crates.io blockers (§2 item 8) were left after
  3.0. The dependencies between workspace crates were paths with no
  version, which `cargo package` refuses, and `repository` was the master
  spec's placeholder, `https://github.com/novalang/nova`.
- ADR 0027 said the package route, by which a `nova` installed from
  crates.io builds and embeds its runtime, "first runs for real at 3.6's
  crates.io install".
- No install script existed. `40-TOOLING.md` §10 named
  `https://novalang.dev/install.sh`, on a domain that does not exist.
- `release.yml` had never run on a tag.

On 2026-10-12 the user made seven decisions:
1. 3.6 is two sub-phases. 3.6a prepares everything and publishes nothing.
   3.6b publishes, step by step on the user's word, runs the gate, writes
   the records and pushes the `v0.3.0` tag.
2. crates.io gets the 19 crates `cargo install nova-cli` needs. The other
   four are `publish = false`.
3. A workflow the user starts publishes the crates, with a token the user
   adds as a secret.
4. The extension is published by hand, from the `.vsix` the release
   attaches, under the publisher `sakeerin`.
5. The install scripts are release assets. They check the archive against
   `SHA256SUMS`. `install.ps1` sets the user's PATH, and `install.sh`
   prints the line to add.
6. The gate runs in CI on fresh Linux, macOS and Windows runners, from
   what was published, and once by the user, from a checklist.
7. The first package on the real index is `shapes`, from
   `examples/05-geometry`.

While planning, a prototype found that the project `nova new` makes
fails `nova fmt --check`, so the gate's format step would fail on every
runner. The spec was amended to fix the templates (its decision 24), and
to run `install.ps1`'s PATH code in `release.yml` (its decision 25).

## Decision

1. **The 19 crates.** crates.io gets every crate `nova-cli` reaches
   through normal and build dependencies, itself included, in this order:
   nova-diagnostics, nova-std, nova-ast, nova-lexer, nova-runtime,
   nova-parser, nova-pm, nova-fmt, nova-index, nova-resolver, nova-hir,
   nova-mir, nova-typeck, nova-codegen-cranelift, nova-codegen-llvm,
   nova-driver, nova-doc, nova-lsp, nova-cli. `nova-bench-http`,
   `nova-bundler`, `nova-codegen-wasm` and `nova-test` are `publish =
   false`: `cargo package` and `cargo publish` skip them, and nothing
   reserves their names. All 23 names were free on crates.io on
   2026-10-12.
2. **One version.** `[workspace.package]` holds `version`, and every
   member says `version.workspace = true`. It stays `0.2.0` in 3.6a;
   3.6b's release commit sets `0.3.0`.
3. **Exact pins.** `[workspace.dependencies]` pins each of the 19 at
   `=<version>`, with its path. Every member's normal and build dependency
   on one says `{ workspace = true }`, the four unpublished crates' too,
   for one rule.
   - The pin is exact because the crates are released together and their
     APIs between each other are not kept stable. `nova-cli` must never
     be built against another release's `nova-driver`, and an exact pin
     holds that even for a `cargo install` without `--locked`.
   - Dev-dependencies stay paths. `cargo package` strips them, so they
     need no version and add no publish order. The cost: a published
     crate's own tests do not build from its tarball.
   - `Cargo.lock` does not change in 3.6a. It records the members'
     versions and their dependencies' names, which stay as they were.
   - A bump changes the workspace version, the 19 pins, the extension's
     `package.json` and `package-lock.json`, and `Cargo.lock`.
     `release_manifests.rs` holds the pins and the extension to the
     workspace version.
4. **Metadata.**
   - `repository` and `homepage` are `https://github.com/Sakeerin/nova`.
   - `crates/nova-cli/README.md` is nova-cli's crates.io page: what Nova
     is, how to install `nova`, and what it needs. The 18 libraries share
     `crates/README.md`, which says their API is internal and changes with
     every release.
   - Each library's description names Nova and says the crate is
     internal: "The Nova language's lexer (internal to the nova
     toolchain)". nova-cli alone has keywords and categories.
   - Each of the 19 crate directories holds copies of `LICENSE-MIT` and
     `LICENSE-APACHE`, byte for byte the root's, so every tarball carries
     both texts. Copies, not symlinks: a Windows checkout turns a symlink
     into a text file.
5. **The package gate.** `.github/scripts/package-gate.sh`, run by CI's
   "Packaged nova (os)" on Linux, macOS and Windows on every pull request:
   - `cargo fetch --locked`, then everything offline;
   - package and verify the 19 with `cargo package --locked`, crates.io's
     own check;
   - check that each packaged `Cargo.lock` names only what the workspace's
     names, at the same versions;
   - unpack the 19, and install `nova` from the unpacked nova-cli with a
     `[patch.crates-io]` entry for each of the 18 libraries: a release
     build, which embeds the runtime through the package route;
   - run `gate.sh --test` on that `nova`.

   The install has no `--locked`, because the patch paths would change the
   packaged lock; the lockfile check stands in for it. What the gate
   cannot prove:
   - crates.io serves the real dependencies, not patched copies;
   - the nested runtime build's offline resolution from
     `~/.cargo/registry` first runs for real at 3.6b's gate;
   - `cargo package`'s verification builds in the dev profile, which
     never embeds, so no `cargo publish` check exercises the embedding
     route.
6. **The publish workflow.** `.github/workflows/publish-crates.yml`,
   "Publish to crates.io", runs only when started by hand, with one input,
   `version`.
   - It refuses unless it runs on the tag `v<version>`, the workspace's
     version is `<version>`, and the secret is set. So crates.io receives
     exactly the tree the GitHub release was built from.
   - It runs the package gate first, then the publish script, with the
     repository's owner as `NOVA_PUBLISH_OWNER`: crates.io's logins are
     GitHub logins, and the crates belong to the account that owns the
     repository.
   - One job, in the environment `crates-io`, where the user can require
     their own approval; a limit of 345 minutes; never two runs at once.
   - The job holds the token, so it checks out with `persist-credentials:
     false`, restores no cache, and runs no third-party action:
     `rust-toolchain.toml` selects stable, which the runner's rustup
     has. Only the publish step's `env:` has the token, and nothing
     prints it.
   - The token is made for one release, scoped to `publish-new` and
     `publish-update` for `nova-*` crates, and set to expire in a day. The
     maintainer adds it as the environment's secret `CARGO_REGISTRY_TOKEN`
     before the run, then revokes it and deletes the secret
     (`docs/releasing.md` §3).
7. **The publish script.** `.github/scripts/publish-crates.sh VERSION`
   publishes the crates in the order of its list. `release_manifests.rs`
   holds the list to the workspace: exactly the published crates, each
   after the crates it depends on.
   - It runs from the repository's root, and refuses unless VERSION is
     the workspace's version there, since `cargo publish` uploads the
     manifests' version whatever VERSION says.
   - Before the first upload it reads every crate's sparse index file.
     For each crate crates.io already has, at any version, the owners
     that crates.io's API gives must include `NOVA_PUBLISH_OWNER`,
     ignoring case; otherwise, or when the API answers anything but
     200, the run stops there with nothing published. Without this check,
     someone else's crate under one of the names, at the version, would
     be skipped as ours, and the next crate's `cargo publish` would
     build it, build script included, in the process that holds the
     token.
   - A crate whose index file has a line with `"vers":"<version>"`, one
     the check found ours, is skipped, a yanked version included, since
     crates.io never takes a version twice. Otherwise the script runs
     `cargo publish --locked -p <crate>`. The index is not asked again
     while publishing, so a name taken meanwhile fails its `cargo
     publish` rather than being skipped.
   - When cargo's output matches `status 429|too many requests|try again
     after`, the script waits 660 s and tries the same crate again, 30
     attempts in all. A bare `429` would match cargo's own `429.0KiB`. Any
     other failure stops the run.
   - It ends with one line per crate: published, skipped, failed or not
     tried, also when it stops. A stopped run is started again from the
     same tag, and goes on where it stopped.
8. **The install scripts.** `tools/install/install.sh` (POSIX `sh`, Linux
   and macOS) and `tools/install/install.ps1` (Windows PowerShell 5.1 and
   PowerShell 7):

   ```sh
   curl -sSfL https://github.com/Sakeerin/nova/releases/latest/download/install.sh | sh
   ```

   ```powershell
   irm https://github.com/Sakeerin/nova/releases/latest/download/install.ps1 | iex
   ```

   - They take `NOVA_VERSION`, or the latest release from where
     `<base>/latest` redirects, and download the archive and `SHA256SUMS`
     from `<base>/download/v<version>/`. `<base>` is `NOVA_INSTALL_BASE`,
     by default `https://github.com/Sakeerin/nova/releases`.
   - The archive's SHA-256 must match its line in `SHA256SUMS`; a
     mismatch or a missing line installs nothing.
   - `nova` goes into `$NOVA_HOME/bin`, by default `~/.nova/bin`
     (`%USERPROFILE%\.nova\bin`). It is written beside the old one, its
     `nova version` runs there, and only then is it renamed over the old
     one, so a failed install, or a `nova` that cannot start on the
     machine, leaves the old one; the error then says nothing was
     replaced and names `cargo install nova-cli --locked`. The output of
     that `nova version` is the install's confirmation.
   - `install.ps1` first renames an existing `nova.exe` to
     `nova.exe.old`, since Windows lets a running program, such as the
     language server VS Code started, be renamed but not replaced, then
     moves the new one in, and renames the old one back if that fails.
     It removes `nova.exe.old` at once if it can, else at the next run,
     which stops with nothing replaced while that old `nova` still runs.
   - `install.ps1` puts the directory first in the user's `Path` in the
     registry, read unexpanded and written back as `REG_EXPAND_SZ`, as
     rustup does; `NOVA_INSTALL_NO_PATH=1` skips it.
     `[Environment]::SetEnvironmentVariable` would have stored every
     `%USERPROFILE%` entry expanded. `install.sh` edits no file, and
     prints the line to add.
   - Both are plain ASCII: Windows PowerShell 5.1 reads a script without
     a byte-order mark in the system's code page.
   - The Linux archive is built on `ubuntu-22.04`, and `release.yml`
     fails the build if its `nova`'s dynamic symbols name a glibc
     version newer than 2.35, so it starts on Ubuntu 22.04, Debian 12 and
     later. Built on `ubuntu-latest` (24.04), it needed glibc 2.39.
   - `release.yml` attaches both, so `SHA256SUMS` covers them. Its
     `prepare` job runs `install.sh` on the release's own Linux archive.
     Its Windows build runs `install.ps1` in Windows PowerShell 5.1, with
     PATH on, on its own archive, twice: the directory must come first,
     every earlier entry must stay as it was, the value must stay
     `REG_EXPAND_SZ`, and after the second run the directory must be
     there once.
9. **The templates.** `nova new` and `nova init` write their `.nova` files
   in `nova fmt`'s layout, a body holding one expression on one line, so a
   new project passes `nova fmt --check` (the spec's §7.1, decision 24).
   What the programs print is unchanged.
10. **The extension, by hand.** The user publishes the `.vsix` the release
    attaches, under the publisher `sakeerin`, to the Marketplace and Open
    VSX. No store token goes into GitHub. The extension gains a CHANGELOG,
    `homepage`, `bugs` and `keywords`; no icon, and no bundler.
11. **The gate's two halves.**
    - `.github/workflows/release-gate.yml`, "Release gate", started by
      hand with a published version, installs `nova` on fresh Linux,
      macOS and Windows runners. From crates.io, `cargo install nova-cli
      --version =<version> --locked`, with no cargo cache, then
      `gate.sh --test`, `packages-gate.sh`, `doc-gate.sh` and `nova fmt
      --check`. From the release's install script, then `gate.sh --test`.
      It uses no secret and publishes nothing.
    - The maintainer's own run on a clean machine, from
      `docs/releasing.md` §6.2, which ends with `shapes` on the real
      index.

    The workflow's log and that run's transcript are the gate's record.
12. **The runbook.** `docs/releasing.md` holds every step of a release, in
    order, each marked where it stops for the user's word. The tag comes
    before crates.io: a GitHub release can be deleted and made again, and
    a crate cannot.

ADR 0026 stands: there is no `nova self update` or `self uninstall`.
Running the script again updates `nova`, and deleting `~/.nova`
uninstalls it.

## Alternatives

- **Every workspace crate on crates.io,** as the phase plan's §1 first
  planned. It would reserve the names, but publish three one-line stubs
  and a load generator that `nova` never uses.
- **Caret requirements between the crates.** A `cargo install` without
  `--locked` could then build `nova-cli` against a later release's
  libraries.
- **Publishing on a tag push, or from the maintainer's machine.** The user
  chose a workflow only they start, on the tag's own tree.
- **Install scripts at a domain of their own.** None exists. As release
  assets, `releases/latest/download/` always serves the latest release's
  own script.
- **`install.sh` editing a shell profile.** The user chose to print the
  line instead.
- **A bundled extension.** Bundling would add an npm dependency, and the
  package works as it is.

## Consequences

- **The package gate's cost,** measured on Linux on this branch when the
  gate was added: 237 s. Fetching took 0 s (the registry cache was warm),
  packaging and verifying 104 s, comparing the lockfiles 0 s, the install
  132 s, and the gate 1 s. The 19 tarballs then held 368 files, nothing
  stray, in 1,436,550 bytes; nova-cli's has grown by two test files
  since. The largest was nova-runtime's, 252,628 bytes, against
  crates.io's limit of 10 MB. The installed `nova` was 24,997,280 bytes,
  carried a 7,525,625-byte runtime, and said `runtime: embedded`. The
  work directory reached 1.6 GB, 1.5 GB of it the target directory. CI
  runs the job beside the others.
- **The tests.** `release_manifests.rs` holds the manifests (7 tests).
  `publish_script.rs` runs the publish script against a fake `cargo`, and
  a loopback sparse index and owners API: 13 tests in about 1.3 s on
  Linux. `install_scripts.rs` runs the install scripts against a
  loopback release: 9 tests in about 22 s on Linux, one of them a release
  whose `nova` cannot run, installed over a working one. `project.rs`
  requires `nova fmt --check` to pass on each project `nova new` and
  `nova init` make.
- **The first upload is slow.** crates.io allows a burst of 5 new crates,
  then one every 10 minutes, so the 19 take about 2 h 20 min. The job's
  345 minutes cover it after the package gate.
- **A publish is hard to undo.** crates.io refuses to delete a crate that
  another crate depends on, at any age, so the 19 can be deleted only one
  at a time, in the reverse of the publish order, `nova-cli` first. After
  72 hours a crate can also be deleted only while it has a single owner
  and at most 1000 downloads for each month since its first publish (RFC
  3660 said 100). Anyone can register a deleted name 24 hours later.
  These are crates.io's rules as its source,
  `src/controllers/krate/delete.rs`, had them on 2026-10-12. Yanking is
  the normal way to withdraw a version; the release gate runs within the
  72 hours, while deleting is simplest.
- **What first runs in the pull request's CI:**
  - the package route on Windows and macOS, with Git Bash's paths and
    GNU tar on Windows;
  - `install.ps1` in Windows PowerShell 5.1, which `install_scripts.rs`
    starts without the `PSModulePath` CI's PowerShell 7 would pass on,
    so that its own cmdlets load, including its check of the new
    `nova.exe` before the swap and its `nova.exe.old` rename; on Linux
    both ran only in PowerShell 7, with stand-in binaries;
  - the Linux release build on `ubuntu-22.04`, and its glibc check;
  - `install.ps1`'s PATH code, in `release.yml`'s Windows build; on Linux
    it ran only in PowerShell 7, against a stand-in registry;
  - the publish script's test under macOS's bash 3.2;
  - `release.yml`'s `install.sh` step, against a real release archive.
- **What first runs for real at 3.6b:** the uploads, the crates.io
  install's nested runtime build, the release gate, and the store and
  token pages, which the runbook writes from their documentation.
- ADR 0031's figures for GitHub's limits come from 3.6b's first publish to
  the real index, and are added there.
- The texts that say how to install `nova` (the root README, the
  extension's README and its error message) change in 3.6b's release
  commit, when what they say becomes true (the spec's decision 21). The
  install scripts work from `v0.3.0`, the first release that carries
  them.
- 3.6a leaves out (the spec's §15):
  - everything 3.6b does: the bump to 0.3.0, CHANGELOG's `[0.3.0]`, the
    tag, every publish, the index repository, running the gate, the ADR
    for Phase 3's boundary, and the README's install section;
  - linux-aarch64 and musl builds, signed and notarized macOS binaries,
    and Homebrew or winget packages;
  - `nova self update` and `self uninstall` (ADR 0026);
  - an icon and a bundled extension.

## References

- `docs/superpowers/specs/2026-10-12-phase-3-6a-release-preparation-design.md`
- `docs/superpowers/plans/2026-10-12-phase-3-6a-release-preparation.md`
- `docs/releasing.md`
- ADR 0026 (Phase 3's scope, no `self update`), ADR 0027 (the runtime and
  std in an installed `nova`, the package route), ADR 0031 (the package
  index, whose figures 3.6b's publish fills)
- `docs/phase-3-plan.md` §2 item 8, §4's 3.6 entry, and §6 risks 1 and 5
- `nova-spec/40-TOOLING.md` §10
