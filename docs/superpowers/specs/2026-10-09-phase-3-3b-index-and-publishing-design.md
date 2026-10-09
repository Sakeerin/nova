# Phase 3.3b, "The index and publishing": design

> Status: **draft for review** (2026-10-09). The second half of Phase 3.3
> (`docs/phase-3-plan.md` §4, the 3.3 entry as amended 2026-10-08). 3.3a,
> "Local packages", merged as PR #105
> (`docs/superpowers/specs/2026-10-08-phase-3-3a-local-packages-design.md`).
> The design was approved in six sections on 2026-10-09; §14 lists the
> decisions.

A Nova package can now depend on a package published to an index. The
index is a git repository the user owns, read over HTTPS; `nova publish`
writes to it through GitHub's API. Versions are resolved with caret ranges,
one version of each package per build, and pinned in `nova.lock`. Packages
are downloaded once, checked against their SHA-256, and unpacked into a
shared cache, after which the compiler treats them as packages in
directories, exactly as 3.3a does.

## 1. What 3.3b delivers

- **The index** (§3): Cargo's sparse layout, one JSON line per version,
  and a `config.json` naming where tarballs are and where publishing goes.
  It is read from an `https://` URL or a local `file:` directory.
- **The resolver and `nova.lock`** (§4): caret ranges, one version per
  name, a backtracking search that prefers locked versions.
- **The cache and the sync step** (§5): downloads, verified and unpacked
  once under `$NOVA_HOME/registry/`. The graph reads `nova.lock` and the
  cache, offline.
- **Commands** (§6): `nova add <name>[@<req>]`, `nova update`, `nova
  fetch`, `nova package`, `nova login`, `nova publish`.
- **Publishing to GitHub** (§7), with the security rules of §8.
- **A new crate, `nova-index`** (§9), which owns everything that touches
  the network.
- **The gate** (§11): CI publishes to a local index and builds against
  it; one real publish to the user's index, by hand, on the user's word.

## 2. The starting point (read on 2026-10-09)

- **Registry entries are refused today.** `nova_pm::graph` turns a
  version-only entry into M0005, "dependency `x` is a registry dependency;
  registry dependencies arrive with the package index", and makes no edge
  (`crates/nova-pm/src/graph.rs:318`).
  - `nova add` without `--path` refuses with the same sentence
    (`crates/nova-cli/src/cmd/deps.rs:79`).
- **Manifests already parse version requirements.** `Dependency.version`
  is a `semver::VersionReq`. An entry with both `version` and `path` is
  M0003 (3.0 spec §5.1), and stays so.
- **`$NOVA_HOME`** is resolved by `cache_root` in
  `crates/nova-driver/src/runtime_cache.rs:55`. It defaults to `.nova` in
  `USERPROFILE` on Windows and `HOME` elsewhere; the runtime library's
  cache is `$NOVA_HOME/runtime/`.
- **The language server's watcher** registers `**/*.nova` and
  `**/nova.toml` (`crates/nova-lsp/src/lib.rs:304`).
- **The CLI's commands:** `parse`, `run`, `build`, `check`, `test`, `fmt`,
  `lsp`, `new`, `init`, `add`, `remove`, `version`
  (`crates/nova-cli/src/main.rs`).
- **The lockfile** holds 232 packages. Already there: `ring` 0.17.14,
  `flate2` 1.1.10, `serde_json` 1.0.151, `semver` 1.0.28. No HTTP client
  and no tar crate.
- **Warnings.** Nova raises two kinds: M0006 (an unknown manifest key) and
  E0021 (an unreachable match arm).
- **Names.** On 2026-10-09:
  - `Sakeerin/nova-index` did not exist on GitHub (404).
  - `nova-registry` is taken on crates.io (200); `nova-index` is free
    (404). The v0.3.0 release publishes every workspace crate there.
- **Gates.** `.github/scripts/gate.sh` refuses to run with any `NOVA_`
  variable set; 3.3a's `packages-gate.sh` runs after it in CI's `install`
  job.

## 3. The index

### 3.1 Layout

- **One file per package**, at Cargo's sparse-index path for its name in
  lower case:
  - one character: `1/<name>`;
  - two characters: `2/<name>`;
  - three characters: `3/<first character>/<name>`;
  - longer: `<characters 1-2>/<characters 3-4>/<name>`.

  So `geom` is `ge/om/geom` and `json-api` is `js/on/json-api`.
- **One JSON line per version,** appended in publishing order:

  ```json
  {"name":"geom","vers":"0.2.0","deps":[{"name":"json-api","req":"^1.0"}],"cksum":"<sha256, 64 lower-case hex digits>","v":1}
  ```

  - `deps` lists the package's `[dependencies]` only: a dependency's
    dev-dependencies are never read (3.3a §3.3).
  - `v` is the line format's version. A reader skips a line whose `v` it
    does not know, so the format can grow.
  - A line that does not parse is skipped, with a warning naming the file
    and line.
- **Names are one name whatever their case.** `Geom` and `geom` share a
  file, and `nova publish` refuses a name whose case differs from one
  already in it. Resolution compares names exactly, as manifests do
  (3.3a's M0008).

### 3.2 `config.json`

At the index's root:

```json
{"dl": "https://github.com/Sakeerin/nova-index/releases/download/{name}-{version}/{name}-{version}.nova-pkg", "api": "Sakeerin/nova-index"}
```

- `dl` is a tarball location template with `{name}` and `{version}`.
  - Absolute (`https://…`) or relative to the index's root
    (`dl/{name}-{version}.nova-pkg`).
- `api`, when present, is the GitHub repository `nova publish` writes to
  (§7). An index without it is written directly, which only a `file:`
  index allows.

### 3.3 Where it is read

- **`NOVA_INDEX`** names the index:
  - an `https://` URL, ending in `/` or not;
  - `http://` only for a loopback host (`127.0.0.1`, `[::1]`,
    `localhost`), which tests use;
  - a local directory as a `file:` URL, `file:///srv/index` or
    `file:///C:/index`. The path is decoded as RFC 8089 says:
    percent-escapes decoded, and on Windows the `/` before a drive letter
    dropped.
- **The default** is `https://raw.githubusercontent.com/Sakeerin/nova-index/main/`.
  Any static host that serves the same files is a mirror.
- **Freshness.** `raw.githubusercontent.com` caches a file for up to five
  minutes. Another machine may not see a version until then, and `nova
  publish` says so.
- **When it is read.** Only while resolving (§4.4): for a dependency
  missing from `nova.lock`, a requirement the locked version no longer
  meets, or `nova update`. A complete lockfile with every tarball cached
  means no network at all.

## 4. The resolver and `nova.lock`

### 4.1 What is resolved

- Every version entry of the root's `[dependencies]` and
  `[dev-dependencies]`, resolved together, so a name has one version
  across both.
- The version entries of each path dependency's `[dependencies]`.
- Each registry package's `deps` from its index line, transitively.

### 4.2 The rule

- **One version of each name per build** (`docs/phase-3-plan.md` §3,
  decision 5). Several major versions side by side wait for an ADR.
- **Requirements** are `semver::VersionReq` with Cargo's semantics:
  `"1.2"` means `^1.2`; `^0.2` means `>=0.2.0, <0.3.0`.
- **A pre-release version** is chosen only when a requirement names a
  pre-release of that version.

### 4.3 The search

- A depth-first, backtracking search in a fixed order. Packages are taken
  in the order they are first reached, ties broken by name, so the same
  inputs always give the same versions.
- For each package, candidates are tried newest first, among those meeting
  every requirement gathered so far. A version in `nova.lock` that still
  meets them is tried before all others, so an edit to the manifest
  changes as few versions as possible.
- A chosen version adds its `deps`. A conflict backs up to the last choice
  with another candidate.
- The search is exponential in the worst case. Nova's graphs are small, and
  PubGrub's better explanations are left for later (§15).

### 4.4 When resolution runs

- When a version entry has no locked package, or its locked version no
  longer meets its requirement, or the lock's index (§4.6) is not
  `NOVA_INDEX`'s.
- Under `nova update`, which ignores the lock, and `nova update <name>`,
  which unlocks that name only.
- Never in the driver or the language server (§5.4).

### 4.5 Diagnostics

Each points at the manifest entry that brought the package in. For a
package reached only through the index, that is the root's entry through
which it is reached, as 3.3a's `Graph::reached_through`.

| Code | Error | Where it points |
|---|---|---|
| M0014 | the index has no package of that name | the entry |
| M0015 | no version meets every requirement; the message lists each requirement and who made it, e.g. "`json`: `^1.2` from `app`, `^2.0` from `geom`" | the root's entry for the first requirer |
| M0016 | `nova.lock` cannot be read; delete it, or run `nova update` | `nova.lock` |

A registry package and a path package with one name are 3.3a's M0011.

### 4.6 `nova.lock`

```toml
# Written by nova. Commit it for a program.
version = 1
index = "https://raw.githubusercontent.com/Sakeerin/nova-index/main/"

[[package]]
name = "geom"
version = "0.2.0"
checksum = "<sha256>"
```

- Registry packages only, sorted by name. A path package is in the tree,
  so it is not locked.
- Only the root's `nova.lock` is read; a path dependency's is ignored.
- Every command that resolves writes it, with `\n` line endings, and only
  when its contents change.
- `nova package` leaves it out of the tarball (40-TOOLING §4.6): only
  libraries are published (§6.5).

## 5. The cache, the sync step, and the graph

### 5.1 The cache

Under `$NOVA_HOME/registry/`, each index in its own directory, named
`<host>-<crc32 of the index URL, 8 hex digits>` (for a `file:` index,
`local-<crc32>`):
- `index/<idx>/…`, the index files last fetched;
- `cache/<idx>/<name>-<version>.nova-pkg`, the tarballs;
- `src/<idx>/<name>-<version>/`, the unpacked packages, never edited in
  place.

`cache_root`'s rule for `$NOVA_HOME` moves from `nova-driver` into
`nova-pm`, so the runtime cache and the registry share one definition.

### 5.2 Downloads and unpacking

- A tarball's SHA-256 is checked against the locked checksum, which came
  from the index line, before anything is unpacked. A mismatch is an
  error naming both digests, and the bad download is deleted.
- Unpacking goes into a temporary directory beside the final one, which
  is renamed into place, so an interrupted unpack never leaves half a
  package.
- Every entry must be a regular file or a directory, under
  `<name>-<version>/`. An absolute path, a `..` component, a link or a
  device file refuses the whole tarball.
- The unpacked package's `nova.toml` must name the package and version it
  was downloaded as.

### 5.3 The sync step

`nova_index::sync(root)`:
1. reads the root's manifest and its path packages' manifests;
2. resolves (§4) when §4.4 says so, reading index files on demand;
3. writes `nova.lock`;
4. downloads and unpacks each locked package missing from the cache.

It runs before `nova run`, `build`, `check` and `test` when they work on a
package (a project, or a file argument directly in a package's `src/` or
`tests/`), and as `nova fetch`. A loose file never syncs.

Its errors are the diagnostics of §4.5, and a plain error for an index
that cannot be reached: "cannot reach the index at <url>: <reason>".

### 5.4 The graph stays offline

- `nova_pm::graph` reads the root's `nova.lock` and finds each version
  entry's locked package in `src/<idx>/<name>-<version>/`, where `<idx>`
  comes from the lock's `index`. From there a registry package is a
  package in a directory: its own manifest, its `[dependencies]` read the
  same way, its identity its canonical directory (3.3a §3.3).
- The graph's caller passes the registry directory, so tests need not set
  `NOVA_HOME`; the commands and the language server pass
  `$NOVA_HOME/registry`.
- **M0005 changes meaning.** A version entry with no locked package, or
  whose package is not unpacked, is M0005, "dependency `json` is not
  downloaded yet; run `nova fetch`", on the entry. That replaces "registry
  dependencies arrive with the package index".
- The driver, the loader and the language server are otherwise unchanged.

### 5.5 The language server

- It never syncs. An entry not yet downloaded is M0005 on the manifest
  entry (3.3a §6's placement).
- Its watcher also registers `**/nova.lock`, and a change to a
  `nova.lock` re-checks every open project, as a `nova.toml` does.
- A cached package is a dependency, so no project publishes diagnostics
  for its files (3.3a's one-owner rule).

## 6. Commands

### 6.1 `nova add <name>[@<req>] [--dev]`

- Without `--path`, a registry dependency.
- Without `@<req>`, the newest version that is not a pre-release, written
  as Cargo writes it: `json = "1.4.1"`, which means `^1.4.1`.
- It syncs as if the entry were written, and writes `nova.toml` and
  `nova.lock` only if that succeeds. M0014, M0015 or an unreachable index
  leaves both untouched, as 3.3a's pre-check does.
- 3.3a's refusals stand: a broken manifest, an entry already in either
  table.

### 6.2 `nova remove`

Unchanged. The next sync drops what the lock no longer needs.

### 6.3 `nova update [<name>]`

Re-resolves (§4.4), writes the lock, downloads what is new, and prints
each change, `json 1.2.0 -> 1.4.1`, or "nothing to update".

### 6.4 `nova fetch`

The sync step and nothing else. For CI caches, and what M0005 tells the
user to run.

### 6.5 `nova package`

Writes `target/package/<name>-<version>.nova-pkg`, a gzip tar whose
entries all start with `<name>-<version>/`.
- **Included:** `nova.toml`, `src/**`, `tests/**`, and top-level
  `README*` and `LICENSE*` files.
- **Left out:** `target/`, `nova.lock`, and any file or directory whose
  name starts with `.` (so `.git/` and editor files).
- **Reproducible:** entries sorted by path, with fixed timestamps, modes
  (`0644` and `0755`) and owners, so packing the same source twice gives
  the same bytes and checksum.
- **Refused, writing nothing:**
  - a manifest with errors;
  - a package without `src/lib.nova`: only a library can be a dependency;
  - each `path` dependency, as M0017, "a published package cannot have a
    path dependency; publish `geom` first and depend on its version", on
    the entry;
  - a symbolic link anywhere in what would be packed;
  - a tarball over 10 MiB.
- It prints the path, the number of files, the size and the SHA-256.

### 6.6 The verification

After packing, `nova package` (and so `nova publish`):
1. unpacks the tarball into a temporary directory;
2. resolves its dependencies afresh from the index, with no lock;
3. runs `nova check` on it, as a module.

Any error or warning (M0006, E0021) stops it, and nothing is published.
This catches a file the tarball leaves out, and a dependency that is not
published. Tests are not run.

### 6.7 `nova login`

- Reads a GitHub token from standard input, one line: `gh auth token |
  nova login`, or paste it.
- Checks it with the GitHub API: the `api` repository of `NOVA_INDEX`'s
  index (the default index unless it is set) must exist, and the token must
  be able to push to it.
- Stores it in `$NOVA_HOME/credentials.toml` as `[github] token = "…"`,
  one token for any GitHub index. On Unix the file is created with mode
  `0600`. On Windows it has the permissions of the directory it is in,
  which for the default `$NOVA_HOME` is the user's profile, readable by
  that user only.
- Never prints the token. To log out, delete the file.

### 6.8 `nova publish`

`nova package` with its verification, then publishing to `NOVA_INDEX`'s
index (§7). A `file:` index needs no login; a GitHub index needs one.

## 7. Publishing to GitHub

Every request carries the stored token, and goes to `api.github.com` or
`uploads.github.com` only.

1. **Read the package's index file** through the Contents API, which is
   authenticated and never stale. If the version is there, stop: the index
   is append-only, and a version is never overwritten. If a name differing
   only in case is there, stop (§3.1).
2. **Create the release** `<name>-<version>` in the index repository, or
   reuse it if an earlier attempt made it.
3. **Upload the tarball** as that release's asset. If the asset exists, its
   SHA-256 must equal this tarball's, or publishing stops.
4. **Append the line** to the package's index file with the Contents API,
   passing the file's current `sha`. If GitHub rejects the write because
   the file changed in between, `nova publish` re-reads it, repeats step
   1's checks, and tries once more.

- **The index line is the commit point.** Until it exists nothing points at
  the tarball, so a failure in steps 2 to 4 leaves nothing anyone can
  depend on, and running `nova publish` again finishes the job.
- **Afterwards** it prints the version, its SHA-256, and that other
  machines may take up to five minutes to see it (§3.3).
- **A `file:` index:** write the tarball where `dl` says, creating
  directories, then append the line. Steps 1 and 3's checks apply.

## 8. Security

- **The token** is sent only to `api.github.com` and `uploads.github.com`:
  never to the index's read host, never with a tarball download, never on a
  command line, never in a log or an error message.
- **Tests' API stand-in.** `NOVA_GITHUB_API` replaces
  `https://api.github.com` (and the upload host) only with an `http://`
  loopback address, so the token cannot be redirected anywhere else.
- **Downloads** are used only after their SHA-256 matches (§5.2).
- **Unpacking** refuses anything that could write outside the package's
  directory (§5.2); **packing** never follows a link (§6.5).
- **Plain `http://`** is refused except on loopback (§3.3).
- **The real index** is never written by CI. CI publishes only to a local
  index; every write to `Sakeerin/nova-index` is the user's, or on the
  user's explicit word (`docs/phase-3-plan.md` §5).

## 9. The crate and its dependencies

- **`nova-index`** (`crates/nova-index/`) holds the index client, the
  cache, the sync step, packing and publishing. `nova-cli` depends on it.
  `nova-pm` gains the resolver and the lockfile, which are pure code, and
  `$NOVA_HOME`'s rule (§5.1). The driver and the language server do not
  depend on `nova-index`.
  - It is named `nova-index` because `nova-registry` is taken on
    crates.io (§2), where v0.3.0 publishes every workspace crate.
- **New dependencies:**
  - `ureq = { version = "=3.2.1", default-features = false, features =
    ["rustls"] }`: rustls on the `ring` already locked, and Mozilla's root
    certificates from `webpki-roots`. ureq 3.3.0 and later need Rust 1.85.
  - `tar = "0.4"`, for its deterministic header mode and entry types.
  - `serde` and `serde_json`, already locked, for the index lines and
    `config.json`.
- **The minimum Rust stays 1.78.** Any crate whose newest release needs
  more is held at an older version in the lockfile; on 2026-10-09 that
  means `zeroize` at 1.8.x and `ureq-proto` at 0.5.x. The plan lists every
  package the lockfile gains, and CI's MSRV job is the proof (3.2's lesson:
  check each version's edition as well as its `rust_version`).
- **Proxies.** ureq reads `HTTPS_PROXY`, `HTTP_PROXY` and `NO_PROXY`.

## 10. Testing

None of the tests reach the internet.

### 10.1 The resolver and the lockfile (`nova-pm`, unit tests)

- The newest matching version; `^0.x`; a pre-release only when named.
- A locked version that still fits is kept; one that no longer fits is
  replaced.
- A conflict that backtracking solves, and one it cannot: M0015 naming each
  requirement and its requirer.
- `nova update <name>` changes only that name.
- `nova.lock` round-trips; an unreadable one is M0016.

### 10.2 `nova-index`

- Index paths for names of one to five characters, and in mixed case.
- `config.json` with absolute and relative `dl`; an unknown `v` skipped; a
  bad line skipped with its warning.
- A `file:` index read and written; `file:` URLs with a drive letter and
  with a percent-escaped space.
- A tarball whose checksum is wrong is refused and deleted.
- Unpacking refuses `..`, an absolute path, a link and a device entry; an
  interrupted unpack leaves no directory behind.
- Packing the same source twice gives identical bytes; a symbolic link is
  refused; a hidden file is left out.

### 10.3 The network, against loopback servers in the test

- An index and tarballs served over `http://127.0.0.1`.
- A GitHub API stand-in that records every request: the four publish steps,
  the retry on a stale `sha`, the refusal of an existing version, a
  resumed publish after a failed upload, and that no request outside the
  API hosts carries the token.

### 10.4 End to end through `nova`

With `NOVA_HOME` in a temporary directory and `NOVA_INDEX=file:…`:
- publish `geom`; `nova add geom` in an app; run, build and test it;
- `nova update` after a newer `geom` is published; `nova fetch`;
- M0005 before fetching, and the build works after;
- a refused `nova add` leaves `nova.toml` and `nova.lock` untouched;
- M0017 for a path dependency, and the other `nova package` refusals;
- verification failing on a warning, and on a dependency that is not
  published.

### 10.5 The language server

M0005 on an entry not yet downloaded, and a re-check when `nova.lock`
changes.

### 10.6 Mutants

Each must fail a named test:
- the checksum check skipped;
- the locked version not preferred;
- the append-only check removed;
- the path-dependency refusal removed;
- `..` allowed when unpacking;
- the token sent with a tarball download.

## 11. The gate

### 11.1 In CI

`.github/scripts/registry-gate.sh NOVA WORKDIR`, in the `install` job after
`packages-gate.sh`, on all three systems, through the installed `nova`:
1. make a local index in `WORKDIR/index` with a `config.json` whose `dl` is
   relative, and set `NOVA_INDEX` to it and `NOVA_HOME` to `WORKDIR/home`;
2. `nova new --lib geom`, then `nova publish`;
3. `nova new app`, `nova add geom`, and a `main.nova` that imports it;
4. `nova run` prints from `geom`; `nova build` and the built program print
   the same; `nova test` passes in both packages;
5. a second `nova build` with the index directory renamed away still works:
   the cache and the lock are enough.

### 11.2 By hand, on the user's word

1. The user creates `Sakeerin/nova-index`, public, with a `config.json`
   (§3.2), and a fine-grained token that can write to it.
2. `gh auth token | nova login` (or the fine-grained token).
3. `nova publish` of a small library.
4. In a second project: `nova add` it from the default index, then `nova
   run`.

The transcript is recorded in ADR 0031, with local paths and names
removed. Until then, 3.3b's merge needs only §11.1.

## 12. Records

- **ADR 0031, "The package index and publishing":**
  - the index layout, the line format and its `v`;
  - one version per name, and why several majors wait;
  - the cache, the sync step and the offline graph;
  - the security rules of §8;
  - the five-minute staleness;
  - why `ureq` is pinned, and the crate's name.
- **Dated notes:**
  - `nova-spec/40-TOOLING.md` §4.2 (the lockfile as built), §4.3 (one
    version per name), §4.4 (the git-backed index replaces the registry
    server's URL, `registry.novalang.dev`), §4.5 (the commands as built,
    `nova fetch` added), §4.6 (what a `.nova-pkg` holds, and the
    verification);
  - `docs/phase-3-plan.md`'s 3.3 entry (3.3b done, the by-hand half's
    status);
  - ADR 0026 (the registry decision, as built).
- **`CHANGELOG.md`**, and the README's packages section: `nova add`
  from the index, `nova login`, `nova publish`.

## 13. Risks

1. **The TLS stack's minimum Rust drifts.** A transitive crate's new
   release can need a newer Rust. Mitigation: exact pins, the committed
   lockfile, and CI's MSRV job.
2. **The CDN's staleness** confuses someone who publishes and immediately
   adds from another machine. Mitigation: `nova publish` says so; reads
   during publishing use the API.
3. **A half-done publish.** Mitigation: the index line is written last, and
   publishing again resumes (§7).
4. **The token.** Mitigation: §8, and tests that look at every request's
   host.
5. **A malicious tarball.** Mitigation: checksums from the index, and
   unpacking that refuses anything outside the package. The index's own
   integrity rests on GitHub and HTTPS; signing is not in 3.3b.

## 14. Decisions

The user's, on 2026-10-09:
1. **The network:** a pinned Rust HTTP crate, so `nova` needs nothing else
   installed (`ureq`, §9).
2. **A `path` dependency refuses publishing** (M0017); an entry keeps
   `version` or `path`, never both.
3. **Verification checks the packed tarball** (§6.6), not the source in
   place, and runs no tests.
4. **The structure:** fetch first, offline graph. The network is in one
   crate, the sync step runs before compiling, and the driver and the
   language server only read the cache.

From `docs/phase-3-plan.md` (2026-10-06): the git-backed index with
release-asset tarballs (decision 4), caret ranges with one version per
name (decision 5), and the token rules of §5.

Taken while writing, for the user's review:
5. **The crate is `nova-index`,** since `nova-registry` is taken on
   crates.io.
6. **The line format** carries `v`, lists regular dependencies only, and
   unknown or broken lines are skipped (§3.1).
7. **`NOVA_INDEX`** selects the index, `https://` or `file:`; `http://`
   only on loopback (§3.3).
8. **The default read host** is `raw.githubusercontent.com`, with its
   five-minute cache; publishing reads through the API (§3.3, §7).
9. **`nova.lock` records the index,** and a different `NOVA_INDEX`
   re-resolves (§4.6).
10. **The cache** is keyed by host and a CRC-32 of the URL, under
    `$NOVA_HOME/registry` (§5.1).
11. **M0005 is reused** for "not downloaded yet"; M0014–M0017 are new
    (§4.5, §6.5).
12. **`nova fetch`** is added, as the sync step on its own (§6.4).
13. **Only libraries are published,** so a tarball never holds
    `nova.lock` (§6.5).
14. **A release per version,** tagged `<name>-<version>` in the index
    repository (§7).
15. **The tarball limit** is 10 MiB, and symbolic links are refused
    (§6.5).
16. **`nova login` reads standard input,** and the token is stored in
    `$NOVA_HOME/credentials.toml` (§6.7).
17. **`NOVA_GITHUB_API`** exists for tests, and accepts only loopback
    (§8).
18. **The resolver** is a hand-written backtracking search; PubGrub waits
    (§4.3).
19. **The by-hand gate** does not hold up the merge; its transcript goes in
    ADR 0031 when it is done (§11.2).

## 15. Not in 3.3b

- Yanking a version, `nova owner`, and deleting from the index.
- Several indexes at once, and an index per dependency.
- Several major versions of one package in one build (decision 5's ADR).
- PubGrub-style conflict explanations.
- Signing packages or index lines.
- A hosted registry server (ADR 0026).
- `nova install`, `nova search`, and `--offline` (a complete lock and cache
  already need no network).
- Reserving names that differ only by `-` and `_`: two such packages in
  one build are 3.3a's M0012.
