# `0.3.1`, the extension's rename: design

> Status: **approved by the user** (2026-10-13). The patch release that ADR
> 0037 maps out of Phase 3: the VS Code extension's `name` becomes
> `vscode-nova`, and `0.3.1` publishes it to the Marketplace and Open VSX
> (`docs/adr/0037-phase-3-boundary.md`, "The extension's stores"). On
> 2026-10-13 the user made four decisions (§9, decisions 1 to 4):
> - only the rename, with no backlog item;
> - the release's `.vsix` keeps its file name, `nova-vscode-<version>.vsix`;
> - after publishing, CI's "Release gate" runs on `0.3.1`, and the user
>   installs the extension from the Marketplace and checks completion;
> - two pull requests: the release, then a short one for the records.
>
> Branch `claude/loving-archimedes-frtf5q`.

`v0.3.0`'s extension reached neither store: the Marketplace refused its
name, `nova`, because another publisher's extension has it. The user
decided that the name becomes `vscode-nova`, released as `0.3.1`, because
the extension's version is always `nova-cli`'s. `0.3.1` changes nothing in
`nova` itself. It renames the extension, corrects the texts that name
`sakeerin.nova`, and publishes the whole release again at the new version,
as the runbook (`docs/releasing.md`) does for every release.

## 1. What `0.3.1` delivers

1. **The release pull request** (§3):
   - the rename, test-first;
   - the texts that name `sakeerin.nova`, and the store install back in
     the root README;
   - the version bump to `0.3.1`, and both CHANGELOGs.
2. **`v0.3.1`, published** (§4):
   - the tag and its GitHub release;
   - the 19 crates at `0.3.1`;
   - the extension in both stores, as `sakeerin.vscode-nova`.
3. **The check** (§5): "Release gate" on `0.3.1`, and the user's install
   of the extension from the Marketplace.
4. **The records pull request** (§6): dated notes that say the extension
   is in the stores.

## 2. The starting point (read on 2026-10-13)

- **`main` and the workspace.**
  - `main` is `8c72fe0`, the last commit of PR #113, which closed Phase 3.
  - Every member of the workspace is at `0.3.0`, with the 19 published
    crates pinned at `=0.3.0` in `[workspace.dependencies]`.
- **What `v0.3.0` published.** It is released:
  - the tag and its GitHub release;
  - the 19 crates on crates.io, owned by `Sakeerin`;
  - `shapes` 0.1.0 in `Sakeerin/nova-index`.
- **The extension's manifest** (`tools/vscode-nova/package.json`):
  - `name` is `nova`, `displayName` `Nova`, `publisher` `sakeerin`,
    `version` `0.3.0`;
  - `package-lock.json` has `"name": "nova"` twice: at its top, and in
    `packages[""]`.
- **The Marketplace's refusal.** The upload said "… already exists in the
  Marketplace. Please use a different 'name' in the package.json file."
  Nothing is in either store.
- **The texts that name `sakeerin.nova`:**
  - `crates/nova-cli/README.md`, line 54;
  - in the runbook:
    - the store listings' addresses (lines 339 and 340);
    - 6.2's install line (line 453);
    - the transcript template (line 545).
  - The runbook's version step names the lock's `"name": "nova"` (line 96,
    and its `perl` command on line 103).
  - The root README says the extension is not in the stores yet (lines 49
    to 55).
- **Dated notes saying `0.3.1` renames the extension.** They are records,
  and they stay as written; §6 adds what happened. They are in:
  - `40-TOOLING.md` §3.3;
  - `docs/phase-3-plan.md`;
  - ADRs 0036 and 0037;
  - the runbook's Accounts note and step 4.
- **The tests.**
  - `crates/nova-cli/tests/vscode_extension.rs` pins the extension's
    version, VS Code floor, `publisher` and language ID, but not its
    `name`.
  - `crates/nova-cli/tests/release_manifests.rs` holds `package.json`'s and
    the lock's versions to the workspace's.
- **Whether `vscode-nova` is free.** This session cannot reach
  marketplace.visualstudio.com (its proxy refuses it), so whether the name
  is free there is the user's check (§4, step 0). Open VSX names are per
  namespace, and the namespace `sakeerin` is the user's.

## 3. The release pull request

Three commits, in this order.

1. **The rename, test-first.**
   - **The tests first:**
     - `vscode_extension.rs` asserts that `name` is `vscode-nova` and
       `displayName` is `Nova`;
     - `release_manifests.rs`'s extension test asserts that the lock's
       `name` and `packages[""].name` equal `package.json`'s.
     - All three assertions fail before the change.
   - **The change:**
     - `"name": "vscode-nova"` in `package.json`, and the lock's two
       `"name"` lines, with no other line touched;
     - `npm ci` against the lock still passes.
   - **The runbook's version step** names `"name": "vscode-nova"`, in its
     text and its `perl` command, so the release commit runs it as
     written.
2. **The texts.**
   - **The root README's editor section** goes back to the store install
     that the release commit of `v0.3.0` (`c0db6c0`) had, with the new ID:
     "install **Nova** by `sakeerin` … from the Marketplace or Open VSX
     (`code --install-extension sakeerin.vscode-nova`). Each GitHub
     release also attaches it as `nova-vscode-<version>.vsix` …".
   - **`crates/nova-cli/README.md`:** `sakeerin.vscode-nova`. This is the
     page crates.io shows for `nova-cli` 0.3.1.
   - **The runbook:**
     - the store listings,
       `https://marketplace.visualstudio.com/items?itemName=sakeerin.vscode-nova`
       and `https://open-vsx.org/extension/sakeerin/vscode-nova`;
     - 6.2's install line;
     - the transcript template.
3. **The release commit**, the runbook's step 1:
   - **The version is `0.3.1`:**
     - 20 lines in `Cargo.toml`, 23 in `Cargo.lock`, 1 in `package.json`,
       2 in `package-lock.json`;
     - by the runbook's commands, with `old=0.3.0`.
   - **`CHANGELOG.md`** gets `## [0.3.1] - <the date>`, with a fresh
     `## [Unreleased]` above it:

     > A patch release for the VS Code extension: it is in the Marketplace
     > and Open VSX as `sakeerin.vscode-nova`, because the Marketplace
     > refused the name `nova` that `0.3.0`'s extension had
     > (`docs/adr/0037-phase-3-boundary.md`). `nova` itself is unchanged;
     > `nova --version` reports 0.3.1.
     >
     > ### Changed
     >
     > - **The VS Code extension's ID is `sakeerin.vscode-nova`.** Install
     >   it with `code --install-extension sakeerin.vscode-nova`, or find
     >   **Nova** by `sakeerin` in the Extensions view; its display name
     >   stays Nova. If you installed `0.3.0`'s `.vsix`, uninstall it first
     >   (`code --uninstall-extension sakeerin.nova`): VS Code counts the
     >   two IDs as two extensions that claim the same settings and
     >   command, so whichever starts second fails, and the one left
     >   running may be the old one.
     >
     > (Corrected 2026-10-13, from the release PR's review: the text first
     > said "each would start `nova lsp`". The extension registers its
     > command before it starts the server, and VS Code refuses a second
     > registration of a command, so the second extension stops there.)

   - **`tools/vscode-nova/CHANGELOG.md`**, which the stores show:
     - a `## [0.3.1]` section: "The first version in the Marketplace and
       Open VSX", with the same change and the same uninstall advice;
     - `[0.3.0]`'s line "The first published version." becomes "Released
       with Nova 0.3.0 as the GitHub release's `.vsix` only, as
       `sakeerin.nova`: the Marketplace refused its name." (decision 5).

The pull request's CI runs everything it runs for any pull request:
- "Packaged nova" packages the 19 crates at `0.3.1` and installs from
  them;
- `release.yml` runs without its publish job, and takes its notes from
  `[0.3.1]`;
- the extension's smoke test runs, and the extension is packaged as
  `nova-vscode-0.3.1.vsix`.

Before the pull request opens, one fresh read-only review of the branch
runs, followed by one fix pass, as for every sub-phase.

## 4. Publishing, in the runbook's order

Each **[stop]** waits for the user's word. Who does what is as in 3.6b:
- **the user:** tokens, the tag, deployment approvals, the store uploads;
- **the agent:** the merge, starting workflows, and checking what each
  step produced.

0. **[stop]** **Before the release commit,** the user checks that no
   Marketplace extension is named `vscode-nova`, from their own machine:

   ```sh
   q() { curl -sS -X POST https://marketplace.visualstudio.com/_apis/public/gallery/extensionquery \
     -H 'Accept: application/json;api-version=7.2-preview.1' -H 'Content-Type: application/json' \
     -d "{\"filters\":[{\"criteria\":[{\"filterType\":10,\"value\":\"$1\"},{\"filterType\":8,\"value\":\"Microsoft.VisualStudio.Code\"}],\"pageSize\":100}],\"flags\":0}" \
     | grep -o '"extensionName":"[^"]*"' | sort -u; }
   q nova          # should list "extensionName":"nova", the name that was refused
   q vscode-nova   # must not list "extensionName":"vscode-nova"
   ```

   - The first query shows that the check sees names. If it does not list
     `nova`, the check proves nothing; the user says so, and the search
     page is used instead.
   - If `vscode-nova` is listed, stop: the user picks another name, and
     the pull request uses it.
1. **[stop]** Merge the release pull request with **Rebase and merge**,
   then wait for CI on `main` to pass.
2. **[stop]** **The tag.**
   - The user makes the tag and pushes it, with the commands the agent
     gives in full (this session's credential cannot push a tag, as
     `v0.3.0` showed). The tag message:
     `-m "Nova v0.3.1" -m "The VS Code extension as sakeerin.vscode-nova, in the Marketplace and Open VSX."`
   - The agent then checks the release:
     - eight assets and `SHA256SUMS`;
     - every sum `OK`;
     - `releases/latest` naming `v0.3.1`.
3. **[stop]** **crates.io.**
   - The user makes a token and adds it as the environment's secret. The
     runbook's scopes stand, though `publish-update` alone would do now
     that all 19 crates exist.
   - The agent starts "Publish to crates.io" on the tag, and the user
     approves the deployment.
   - What to expect:
     - the owner check lists the 19 crates, each owned by `Sakeerin`;
     - the 19 new versions fit in crates.io's burst of 30, so the upload
       takes minutes.
   - Afterwards, the user revokes the token and deletes the secret.
4. **[stop]** **The extension.**
   - The user publishes the release's `nova-vscode-0.3.1.vsix` to the
     Marketplace, then to Open VSX (the runbook's step 4).
   - The user checks the two listings at their new addresses.
   - The runbook's step 5, the index, is for the first release only.

## 5. The check

- **[stop]** **"Release gate" on `0.3.1`.**
  - The agent starts it on the user's word.
  - It installs `nova` 0.3.1 from crates.io and with the install scripts,
    on Linux, macOS and Windows, and publishes nothing.
  - All six jobs must pass.
- **The user's store install,** in a VS Code kept apart, as for `v0.3.0`.
  It uses a new directory (`~/nova-031` below), because `~/nova-gate`'s
  extensions directory holds `sakeerin.nova` 0.3.0.

  ```sh
  cargo install nova-cli --locked --root ~/nova-031
  ~/nova-031/bin/nova version                     # nova 0.3.1, runtime: embedded
  code --user-data-dir ~/nova-031/data --extensions-dir ~/nova-031/ext
  ```

  In that window:
  1. Install **Nova** by `sakeerin` from the Extensions view (the
     Marketplace).
  2. Set `nova.server.path` to `~/nova-031/bin/nova` (spelled out in
     full), and open a project.
  3. Type `p.` as in the runbook's 6.2 step 3: completion offers `x` and
     `y`.
  4. Take a screenshot.

  Then:

  ```sh
  code --extensions-dir ~/nova-031/ext --list-extensions --show-versions   # sakeerin.vscode-nova@0.3.1
  ```

  The Open VSX listing is checked by its page, in step 4 above. VS Code
  itself installs from the Marketplace.
- The runbook's full 6.2 run is not repeated (decision 3). `shapes` is
  already in the index, so it is not published again.

## 6. The records pull request

After the check, from `main`. It changes no code, and CHANGELOG gets no
entry. It adds dated notes ("Recorded <date>") on what `0.3.1` did:
- **ADR 0037**, under "The extension's stores": the extension is published
  as `sakeerin.vscode-nova` in both stores, with:
  - the release;
  - the Publish and "Release gate" runs;
  - the user's store install.
- **ADR 0036:** beside its line saying `0.3.1` renames the extension.
- **The runbook:**
  - beside its Accounts note and step 4;
  - anything in `0.3.1`'s steps that went otherwise.
- **`40-TOOLING.md` §3.3** and **`docs/phase-3-plan.md`:** beside their
  `0.3.1` lines.

As in 3.6b's close-out, the PR body describes the screenshot in words.
The records name no user, machine or path.

## 7. If something fails

- **`vscode-nova` is taken** (step 0). Stop before the release commit;
  the user picks another name.
- **The Marketplace refuses `0.3.1`'s `.vsix` for another reason** (step
  4).
  - The `.vsix` is the release's, so a fix is `0.3.2`, from step 1 (the
    runbook's section 7).
  - The crates at `0.3.1` stay; they work, and only the extension waits.
- **The Release run fails, the publish stops, or the gate fails.** The
  runbook's section 7 applies as written.
  - A version of a crate that exists already cannot be withdrawn by
    deletion without deleting the crate, so a bad `0.3.1` is yanked
    (`cargo yank --version 0.3.1`), not deleted.
  - Yanking is the user's decision.

## 8. Not in `0.3.1`

- **No backlog item from ADR 0037,** housekeeping included (decision 1).
- **No change to `nova`'s code,** and no new ADR: ADR 0037 made the
  decision.
- **The `.vsix`'s file name does not change** (decision 2). The file name
  need not match the manifest's `name`, and `release.yml`, CI, the
  install scripts' test and the runbook all use the current one.
- **`0.3.0`'s crates are not yanked.** They work, and only their crates.io
  README names the old ID, which `0.3.1`'s corrects.
- **No full 6.2 run, and no second publish of `shapes`** (decision 3).

## 9. Decisions

**The user's (2026-10-13):**
1. Only the rename, with no backlog item.
2. The `.vsix` keeps the file name `nova-vscode-<version>.vsix`.
3. The check after publishing: "Release gate" on `0.3.1`, and the user's
   install of the extension from the Marketplace, with completion.
4. Two pull requests: the release, then the records.

**Made here, for review with this spec:**

5. **The extension's CHANGELOG corrects `[0.3.0]`'s line.** The stores
   show this file, and "The first published version" would be false
   there. The section's heading and date stay.
6. **The uninstall advice for `sakeerin.nova` goes in both CHANGELOGs.**
   It stays out of the READMEs: only a `0.3.0` `.vsix` install needs it.
7. **The tests pin the extension's `name` and `displayName`, and the
   lock's `name`.** A later bump or edit then cannot lose the rename.
8. **The tag is the user's to push** (this session's credential cannot).
   Each workflow is started by the agent on the user's word, as in 3.6b
   decision 3.
