# Releasing Nova

The steps of a release, from `v0.3.0` on, in the order they run (spec
`docs/superpowers/specs/2026-10-12-phase-3-6a-release-preparation-design.md`
§9). A release publishes the same version everywhere at once: the GitHub
release with its archives and install scripts, the 19 crates on crates.io,
and the VS Code extension on the Marketplace and Open VSX. The package
index is set up once, at the first release, whose gate publishes `shapes`
to it.

**[stop]** marks a step that publishes something, touches an account or
cannot be undone. Whoever works through this runbook, a person or an agent
working for the maintainer, waits there for the maintainer's explicit word
before doing it. Everything between two stops is preparation that can be
redone.

Below, `0.3.0` stands for the version being released. The commands set it
once, in a shell variable:

```sh
version=0.3.0
```

The commands are for a POSIX shell (on Windows, Git Bash) on the machine
the maintainer releases from, with `git`, `gh` (logged in to GitHub as
the repository's owner), Rust, Node.js 22 with `npm`, and `perl`.

## Before the first release

Once, before `v0.3.0`; a later release needs none of this again, except
that tokens are made fresh each time. **[stop]** Every item here is the
maintainer's own, on their accounts.

### Accounts

- **crates.io.** Log in at <https://crates.io> with the GitHub account
  that will own the crates, and verify its e-mail address under Account
  Settings: crates.io refuses a publish from an account without one.
- **The VS Code Marketplace.** At
  <https://marketplace.visualstudio.com/manage>, signed in with a
  Microsoft account, create a publisher whose ID is `sakeerin`. It must
  equal `publisher` in `tools/vscode-nova/package.json`, which
  `crates/nova-cli/tests/vscode_extension.rs` pins. Publishing takes an
  Azure DevOps personal access token (step 4), made at
  <https://dev.azure.com> under User settings → Personal access tokens.
- **Open VSX.** Log in at <https://open-vsx.org> with GitHub, link an
  Eclipse Foundation account (made at <https://accounts.eclipse.org> with
  the same GitHub account) under Settings, and sign the Publisher
  Agreement there. Then make an access token under Settings → Access
  Tokens and create the namespace, once:

  ```sh
  npx ovsx create-namespace sakeerin -p <open-vsx-token>
  ```

  `npx` asks to download `ovsx` first; it is not one of the extension's
  dependencies.

### The GitHub environment `crates-io`

The publish workflow runs in this environment, and its token lives there
only for the length of a publish (step 3).

1. In the repository, Settings → Environments → New environment, named
   `crates-io`.
2. Deployment protection rules: tick **Required reviewers** and add the
   maintainer. Leave "Prevent self-review" off, or the maintainer cannot
   approve their own run.
3. Deployment branches and tags: **Selected branches and tags**, then add
   a rule of type Tag with the pattern `v*`. The workflow itself refuses
   anything but the tag `v<version>`; this makes GitHub refuse it too.
4. Add no secret yet.

## 1. The release commit

A pull request whose last commit is the release commit:

1. **The version.** It is set in the root `Cargo.toml` (the workspace's
   `version` and the 19 `=` pins in `[workspace.dependencies]`), in
   `tools/vscode-nova/package.json`, and in
   `tools/vscode-nova/package-lock.json` (the two `version` lines under
   `"name": "nova"`; the lock holds other packages' versions too). From a
   checkout of `main`, on a new branch:

   ```sh
   old=0.2.0   # the version main has now
   perl -pi -e "s/^version = \"\Q$old\E\"\$/version = \"$version\"/; s/version = \"=\Q$old\E\"/version = \"=$version\"/" Cargo.toml
   perl -pi -e "s/^  \"version\": \"\Q$old\E\",\$/  \"version\": \"$version\",/" tools/vscode-nova/package.json
   perl -0pi -e "s/(\"name\": \"nova\",\n\s*\"version\": )\"\Q$old\E\"/\$1\"$version\"/g" tools/vscode-nova/package-lock.json
   cargo update --workspace
   git diff --stat
   ```

   `cargo update --workspace` moves only the workspace's own 23 packages
   in `Cargo.lock` (add `--offline` without a network). The diff shows 20
   lines changed in `Cargo.toml`, 23 in `Cargo.lock`, 1 in `package.json`
   and 2 in `package-lock.json`. `npm version` is not used: it rewrites
   `package.json`'s layout. Then:

   ```sh
   cargo test --locked -p nova-cli --test release_manifests --test vscode_extension
   ```

   holds the pins and the extension to the workspace's version.
2. **`CHANGELOG.md`.** `## [Unreleased]` becomes
   `## [0.3.0] - <the date, YYYY-MM-DD>`, under a lead paragraph that says
   what the release is (for `v0.3.0`: Phase 3 is complete, within the
   boundary its ADR records), and a fresh, empty `## [Unreleased]` goes
   above it. Phase 2's `[0.2.0]` section, from the commit "docs: finalize
   CHANGELOG for the v0.2.0 release", is the model. `release.yml` takes the
   release notes from this section, and fails without it.
3. **`tools/vscode-nova/CHANGELOG.md`,** which the stores show:
   `## [Unreleased]` becomes `## [0.3.0] - <the date>` (for `v0.3.0`, with
   the line "The first published version." under it). If the extension
   did not change since the last release, add a section that says
   "Released with Nova 0.3.0; no change to the extension." The extension's
   version always moves with Nova's.
4. Anything else the release's own spec puts in the release commit (for
   `v0.3.0`: the texts that say how to install `nova`, spec §7).

The pull request runs CI, including "Packaged nova", which packages the
19 crates at the new version and installs `nova` from them, and
`release.yml` without its publish job, whose release notes come from
item 2's section. Wait until all of it is green.

**[stop]** Merge it with **Rebase and merge** (`gh pr merge <number>
--rebase`): the repository has no merge commits, and every release tag
points at its CHANGELOG commit. Then wait for CI on `main` to pass on the
merged commits.

## 2. The tag and the GitHub release

Rebasing gave the commits new hashes, so tag the one on `main`:

```sh
git fetch origin main
git log -1 --format='%H %s' origin/main   # the CHANGELOG commit
commit=$(git rev-parse origin/main)
git tag -a "v$version" -m "Nova v$version" "$commit"
```

For a phase's release the message says so, as `v0.2.0`'s did: for
example `-m "Nova v0.3.0 — Phase 3 (Tooling)" -m "Phase 3 is complete,
within the boundary docs/adr/<its ADR> records."`.

**[stop]** Push it:

```sh
git push origin "v$version"
```

The tag starts `release.yml`, which builds the four targets, smoke-tests
three of them, installs the Windows archive with `install.ps1` (PATH on)
and the Linux one with `install.sh`, and creates the GitHub release
`v0.3.0` (not a draft, and a prerelease only if the version has a `-`).
Follow it with `gh run watch`, picking the Release run. Then check the
release:

```sh
gh release view "v$version" --repo Sakeerin/nova --json isPrerelease,assets --jq '.isPrerelease, .assets[].name'
mkdir "release-$version" && cd "release-$version"
gh release download "v$version" --repo Sakeerin/nova
sha256sum --check SHA256SUMS   # macOS: shasum -a 256 --check SHA256SUMS
curl -sSI https://github.com/Sakeerin/nova/releases/latest | grep -i '^location'
```

There are eight assets and `SHA256SUMS`, and every line of the check says
`OK`:

```
nova-0.3.0-aarch64-apple-darwin.tar.gz
nova-0.3.0-x86_64-apple-darwin.tar.gz
nova-0.3.0-x86_64-pc-windows-msvc.zip
nova-0.3.0-x86_64-unknown-linux-gnu.tar.gz
nova-std-docs-0.3.0.tar.gz
nova-vscode-0.3.0.vsix
install.sh
install.ps1
SHA256SUMS
```

`releases/latest` redirects to `…/releases/tag/v0.3.0`; the install
scripts find the latest version through it. The release's notes are the
CHANGELOG section of step 1.

A GitHub release can be deleted and made again; a crate cannot. That is
why the tag comes before crates.io.

## 3. crates.io

The workflow `.github/workflows/publish-crates.yml`, "Publish to
crates.io", publishes the 19 crates in dependency order with
`.github/scripts/publish-crates.sh` (spec §5). It runs only when started
by hand, only on the tag `v<version>` whose workspace version is
`<version>`, and it first repeats CI's package check
(`.github/scripts/package-gate.sh`).

1. **[stop]** Make a token at crates.io, Account Settings → API Tokens →
   New Token:
   - name: for example `nova v0.3.0`;
   - expiration: a custom date, the next day;
   - scopes: **publish-new** and **publish-update** only;
   - crates: the pattern `nova-*`.
2. **[stop]** Add it as the environment's secret, in Settings →
   Environments → `crates-io` → Environment secrets → Add environment
   secret, named `CARGO_REGISTRY_TOKEN`; or, pasting the token at the
   prompt:

   ```sh
   gh secret set CARGO_REGISTRY_TOKEN --env crates-io --repo Sakeerin/nova
   ```

3. **[stop]** Start the workflow: Actions → "Publish to crates.io" → Run
   workflow, "Use workflow from" the tag `v0.3.0`, version `0.3.0`; or

   ```sh
   gh workflow run publish-crates.yml --repo Sakeerin/nova --ref "v$version" -f version="$version"
   ```

   The run waits for the environment's reviewer: approve it under "Review
   deployments".
4. Wait. The package check takes about as long as CI's install job. The
   upload of 19 new crates then takes about 2 h 20 min: crates.io allows a
   burst of 5 new crates and then one every 10 minutes, and the script
   prints `rate limited: waiting <n> s` and tries again. A later release
   only adds versions to existing crates, which crates.io allows 30 at a
   time and then one a minute, so its upload takes minutes. The job's
   limit is 345 minutes. It ends with one line per crate, published or
   skipped.
5. Check what crates.io serves:

   ```sh
   curl -sS https://index.crates.io/no/va/nova-cli | tail -n 1
   ```

   The last line names `"vers":"0.3.0"`.
6. **Afterwards, whatever the result,** revoke the token (crates.io,
   Account Settings → API Tokens → Revoke) and delete the secret:

   ```sh
   gh secret delete CARGO_REGISTRY_TOKEN --env crates-io --repo Sakeerin/nova
   ```

   A run that stopped can be started again from the same tag with a new
   token (section 7).

## 4. The extension

The extension is published by hand, from the release's own `.vsix`. No
store token goes into GitHub.

```sh
git clone --depth 1 --branch "v$version" https://github.com/Sakeerin/nova "nova-$version"
cd "nova-$version/tools/vscode-nova"
npm ci   # so that npx runs the locked @vscode/vsce
gh release download "v$version" --repo Sakeerin/nova --pattern "nova-vscode-$version.vsix" --pattern SHA256SUMS
grep " nova-vscode-$version.vsix\$" SHA256SUMS | sha256sum --check   # macOS: shasum -a 256 --check
```

1. **[stop]** The Marketplace. Make an Azure DevOps personal access token
   with Organization **All accessible organizations**, Scopes **Custom
   defined → Marketplace → Manage**, and a short expiry. Then:

   ```sh
   read -rs VSCE_PAT && export VSCE_PAT   # paste the token; it is not shown
   npx @vscode/vsce publish --packagePath "nova-vscode-$version.vsix"
   unset VSCE_PAT
   ```

   (`vsce` can also sign in with Microsoft Entra ID instead of a token,
   with `--azure-credential` after `az login`; its documentation says
   which Azure DevOps currently expects.)
2. **[stop]** Open VSX, with a token from open-vsx.org's Settings →
   Access Tokens:

   ```sh
   read -rs OVSX_PAT && export OVSX_PAT
   npx ovsx publish "nova-vscode-$version.vsix"
   unset OVSX_PAT
   ```

   (`-p <token>` in place of `OVSX_PAT` does the same, and leaves the
   token in the shell's history.)
3. Check the listings, which can take a few minutes to appear:
   <https://marketplace.visualstudio.com/items?itemName=sakeerin.nova>
   and <https://open-vsx.org/extension/sakeerin/nova>. Each shows version
   0.3.0, the README, the CHANGELOG, and the repository, homepage and
   issues links. Revoke the Azure DevOps token when done.

## 5. The index (first release only)

`nova` reads the package index at
`https://raw.githubusercontent.com/Sakeerin/nova-index/main/` unless
`NOVA_INDEX` says otherwise, and `nova publish` writes to the repository
its `config.json` names (ADR 0031; spec
`docs/superpowers/specs/2026-10-09-phase-3-3b-index-and-publishing-design.md`
§3.2, §7, §11.2).

1. **[stop]** Create the repository, public, with its `config.json` on
   `main`:

   ```sh
   gh repo create Sakeerin/nova-index --public --description "The Nova package index" --clone
   cd nova-index
   printf '%s\n' '{"dl": "https://github.com/Sakeerin/nova-index/releases/download/{name}-{version}/{name}-{version}.nova-pkg", "api": "Sakeerin/nova-index"}' > config.json
   git add config.json
   git commit -m "The index's config"
   git branch -M main
   git push -u origin main
   curl -sS https://raw.githubusercontent.com/Sakeerin/nova-index/main/config.json
   ```

   The last command prints the file back; give it a few minutes if it
   says 404 at first.
2. **[stop]** Make the token `nova publish` uses: github.com, Settings →
   Developer settings → Personal access tokens → Fine-grained tokens →
   Generate new token, with resource owner `Sakeerin`, repository access
   **Only select repositories** → `Sakeerin/nova-index`, repository
   permission **Contents: Read and write** (Metadata: read-only comes with
   it), and a short expiry. It goes into `nova login` on the machine that
   publishes, in step 6.2. `gh auth token | nova login` works too, with
   `gh`'s own, broader token.

The index is append-only: a published version stays, and a name keeps the
case it was first published with.

## 6. The gate

### 6.1 In CI

**[stop]** Once the crates and the GitHub release are up (steps 2 and 3),
start the workflow `.github/workflows/release-gate.yml`: Actions →
"Release gate" → Run workflow (from `main`), version `0.3.0`; or

```sh
gh workflow run release-gate.yml --repo Sakeerin/nova -f version="$version"
```

On fresh Linux, macOS and Windows runners it installs `nova` from
crates.io (`cargo install nova-cli --version =0.3.0 --locked`) and runs
the install gates on it, and installs it with the release's `install.sh`
or `install.ps1` and runs `gate.sh --test` on that. It publishes nothing.
All six jobs must pass. Keep the run's address for the record.

If the crates.io jobs fail, decide quickly: deleting the crates is
simplest within 72 hours of publishing them (section 7).

### 6.2 On a clean machine

The maintainer's own run, recorded, on a machine of their choosing: a
fresh virtual machine, or a new user account on an existing one (so
nothing of an earlier `nova`, `~/.nova` or `~/.cargo` is in reach). It
needs:
- Rust, from <https://rustup.rs> (in a new user account too: rustup
  installs per user);
- a C linker, which `nova build` and `nova test` use: `cc` on Linux (for
  example the `build-essential` package), the Xcode Command Line Tools on
  macOS (`xcode-select --install`), the Visual Studio Build Tools with
  "Desktop development with C++" on Windows;
- VS Code;
- `curl` and `tar`, which current Windows has too.

The commands, in order, each with what it should show:

1. Install from crates.io; time it.

   ```sh
   cargo install nova-cli --locked
   nova version
   ```

   `nova 0.3.0`, the target, and `runtime: embedded`.
2. A project:

   ```sh
   nova new hello
   cd hello
   nova run
   ```

   `Hello, Nova!`.
3. **Autocomplete.** Install the extension from the store: in VS Code's
   Extensions view, search for Nova and install the one by `sakeerin`, or
   `code --install-extension sakeerin.nova`. Open the folder (`code .`),
   open `src/main.nova`, and replace it with this, cramped on purpose for
   step 4:

   ```nova
   record Point{x:Int,y:Int}

   fn main() {
       let p = Point{x:3,y:4}
       println("${p.x + p.y}")
   }
   ```

   Then, inside `main`, type `p.` on a new line: completion offers `x` and
   `y`. Take a screenshot, then delete the line again and save.
4. Format:

   ```sh
   nova fmt --check
   nova fmt
   nova fmt --check
   nova run
   ```

   The first check prints `would reformat: …main.nova` and exits 1;
   `nova fmt` rewrites it as `record Point { x: Int, y: Int }` and
   `Point { x: 3, y: 4 }`; the second check prints nothing and exits 0;
   `nova run` prints `7`.
5. **[stop]** Publish `shapes`, the library of `examples/05-geometry`, to
   the real index. This is permanent.

   ```sh
   cd ..
   curl -sSfL "https://github.com/Sakeerin/nova/archive/refs/tags/v$version.tar.gz" | tar xz
   cp -R nova-*/examples/05-geometry/shapes shapes
   cd shapes
   read -rs TOKEN                                  # the token of step 5
   printf %s "$TOKEN" | nova login
   curl -sS -H "Authorization: Bearer $TOKEN" https://api.github.com/rate_limit | grep -A 4 '"core"'
   nova package
   nova publish
   curl -sS -H "Authorization: Bearer $TOKEN" https://api.github.com/rate_limit | grep -A 4 '"core"'
   unset TOKEN
   curl -sS https://api.github.com/repos/Sakeerin/nova-index | grep '"size"'
   ```

   `nova login` says it is logged in to `Sakeerin/nova-index`; `nova
   publish` prints `published shapes 0.1.0 to Sakeerin/nova-index; other
   machines may take up to five minutes to see it`. The difference between
   the two `"used"` counts is the API requests the publish made, and
   `"size"` the repository's size in KB; ADR 0031 waits for both figures.
   In PowerShell 7, `Read-Host -MaskInput | nova login` replaces the
   `read` and `printf` lines.
6. A second project that depends on it, from the index:

   ```sh
   cd ..
   nova new measure
   cd measure
   nova add shapes
   cp ../nova-*/examples/05-geometry/src/main.nova src/main.nova
   nova run
   ```

   `nova add` prints `added shapes = "0.1.0" to [dependencies]` (if it
   says the index has no `shapes`, wait five minutes and try again).
   `nova run` prints:

   ```
   area 3.141592653589793, perimeter 6.283185307179586
   area 6, perimeter 10
   total area 9.141592653589793
   ```

7. Log out: delete `~/.nova/credentials.toml`
   (`%USERPROFILE%\.nova\credentials.toml`), and revoke the token unless
   it is kept for later publishes.

### 6.3 The transcript

Keep the run as a transcript, from the terminal's own output, with the
screenshot of step 3. Before it goes into the records, remove the home
directory (write `~`), user and machine names, IP addresses, and anything
a token could be read from; never paste a token. 3.6b's records take it:
the gate's record in the ADR for Phase 3's boundary, and the index's
figures in ADR 0031.

```text
Release:        v0.3.0
Date:           YYYY-MM-DD
Machine:        <OS and version>, <architecture>; <fresh VM image | new user account>
Rust:           <rustc --version>
VS Code:        <version>; extension sakeerin.nova <version>, from <Marketplace | Open VSX>
Release gate:   <the Actions run's address>, <all six jobs passed?>

1. cargo install nova-cli --locked     <duration>
   nova version                        <its three lines>
2. nova new hello; nova run            <output>
3. completion after `p.`               <the items offered>   [screenshot]
4. nova fmt --check; nova fmt; nova fmt --check; nova run
                                       <exit codes, output>
5. nova login                          <output>
   nova publish                        <output, with its sha256>
   API requests used by the publish    <after - before>
   nova-index size                     <KB>
6. nova add shapes                     <output>
   nova run                            <output>
Anything that went differently:        <notes>
```

## 7. If something fails

**A Release run on the tag fails** (step 2). If the failure passes (a
runner, the network), re-run the run's failed jobs (`gh run rerun
<run-id> --failed`): the release is made once they pass.

**An archive is wrong, or the Release run needs a change to pass** (step
2). Do this before step 3, while nothing else is published. **[stop]**
Delete the GitHub release, if one was made, and the tag:

```sh
gh release delete "v$version" --repo Sakeerin/nova --yes
git push origin --delete "v$version"
git tag -d "v$version"
```

Fix it on `main` in a pull request, and release the next patch version
(`0.3.1`) from step 1. A tag that was pushed is not moved to another
commit.

**A crate is rejected in the middle of the publish** (step 3). The script
stops at the first failure with cargo's output; the crates before it are
on crates.io for good, at that version.
- A passing failure (the network, crates.io's limit outlasting the
  retries, the job's time): **[stop]** start the workflow again on the
  same tag, with a fresh token if the first expired. Crates already on
  crates.io at that version are skipped (`skip <crate> <version> (already
  on crates.io)`), and the run continues where it stopped.
- A failure that needs a change (a manifest, a name taken since, a check
  crates.io makes that the package gate does not): fix it on `main`, then
  release the next patch version from step 1. Every crate moves to it,
  even those already published, because the pins between them are exact;
  nothing builds a mix. The half-published version's crates stay, with
  no `nova-cli` of that version to use them, since it is published last;
  yank them if wanted (below).

**Yank or delete.** `cargo yank --version 0.3.0 <crate>` (a token with
the **yank** scope) hides a version from new resolution, keeps it for
lockfiles that already name it, and can be undone with `--undo`; it is
the normal way to withdraw a version. Deleting removes a whole crate,
every version of it: its owner does it at crates.io → the crate →
Settings, logged in (crates.io takes no token for it). Delete only when a
publish must be undone entirely, for example a file that should never
have been in a tarball. crates.io's rules (its
`src/controllers/krate/delete.rs`, read on 2026-10-12):
- It refuses to delete a crate that another crate on crates.io depends
  on, whatever its age. So the 19 are deleted one at a time in the
  reverse of `publish-crates.sh`'s list: `nova-cli` first, then
  `nova-lsp`, `nova-doc`, `nova-driver` and so on, `nova-diagnostics`
  last.
- Within 72 hours of a crate's first publish, that is the only rule.
  After them, a crate can also be deleted only while it has a single
  owner and at most 1000 downloads for each month since its first publish.
  (RFC 3660 said 100 a month, as `docs/phase-3-plan.md` §6 risk 5 does.)
- Anyone can register a deleted name 24 hours later.

**The extension** (step 4). A refused token: make a new one. A refused
name or listing: the `.vsix` is the release's, so a change to it is a new
patch version. A version once published to a store cannot be published
there again.

**The index** (step 5, 6.2). `nova publish` commits by appending the
index line last, so a publish that failed can be run again and finishes.
A published version stays: there is no yank in the index.

**The release gate or the clean-machine run fails** (step 6). Record what
failed. **[stop]** The maintainer decides: delete the crates, `nova-cli`
first (above), which is simplest within 72 hours of the crates.io
publish; or, instead or as well, fix it and release the next patch
version.
