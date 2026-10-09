# Phase 3.3b, "The index and publishing": design

> Status: **draft for review** (2026-10-09). The second half of Phase 3.3
> (`docs/phase-3-plan.md` §4, the 3.3 entry as amended 2026-10-08). 3.3a,
> "Local packages", merged as PR #105
> (`docs/superpowers/specs/2026-10-08-phase-3-3a-local-packages-design.md`).
> The design was approved in six sections on 2026-10-09, then corrected
> after a read-only fact-check against the code; §14 lists every decision.

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
  It is read from an `https://` URL or a local directory.
- **The resolver and `nova.lock`** (§4): caret ranges, one version per
  name, a backtracking search that keeps locked versions.
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
  registry dependencies arrive with the package index", with the note "use
  `path = \"...\"` for a local package", and makes no edge
  (`crates/nova-pm/src/graph.rs:315-328`).
  - `nova add` without `--path` refuses with "registry dependencies arrive
    with the package index; use `--path <dir>` for a local package"
    (`crates/nova-cli/src/cmd/deps.rs:79`). Its help text says "Add a path
    dependency" (`deps.rs:18`, `main.rs:52`).
  - Tests pin both: `nova-pm/tests/graph.rs:119-130`,
    `nova-cli/tests/project.rs:226-234` (`http = "1.0"` under `nova run`),
    `nova-cli/tests/packages.rs:188`, `nova-cli/tests/deps.rs:197-200`.
    None sets `NOVA_HOME` or an index.
- **Manifests already parse version requirements.** `Dependency.version`
  is a `semver::VersionReq`. An entry with both `version` and `path` is
  M0003 (`manifest.rs:354-361`, the 3.0 spec §5.1), and stays so.
- **Who builds a graph.** `nova_pm::graph` is called by
  `Program::for_file` and `Program::for_package`
  (`crates/nova-driver/src/program.rs:83`, `:101`); `graph_from` by
  `nova add`'s pre-check (`deps.rs:130`), which refuses on any graph error
  but M0013 (`deps.rs:129-137`). The two constructors are reached from:
  - the CLI (`crates/nova-cli/src/project.rs:33-34`);
  - the driver's path-based entry points (`check_file`, `compile_file`,
    `build_file`, `build_file_release`, `build_test_binary`, `run_file`,
    `analyze`);
  - the language server (`checker.rs`, `completion.rs`);
  - about two dozen tests.
- **A graph edge to a path** is `declarer.dir.join(path)`
  (`graph.rs:330-332`), whoever the declarer is.
- **`$NOVA_HOME`.** `cache_root` (`crates/nova-driver/src/runtime_cache.rs:55-78`,
  `pub(crate)`, using `anyhow`) returns `$NOVA_HOME/runtime`. `NOVA_HOME`
  defaults to `.nova` in `USERPROFILE` on Windows and `HOME` elsewhere. Its
  error names `NOVA_RUNTIME_LIB`, and four tests (`:391-446`) pin it. The
  same file shows how two processes share a cache safely: unique temporary
  names from the process id and a counter (`:145-161`), and accepting the
  other process's copy after a failed rename (`:103-115`), with a
  two-process test (`:329-358`).
- **Warnings are printed, not counted.** `FrontendContext::render` counts
  only errors (`crates/nova-driver/src/lib.rs:586-594`), so `check_program`
  succeeds with warnings. The compiler raises two kinds: M0006 (an unknown
  manifest key, `manifest.rs:398`) and E0021 (an unreachable match arm,
  `check.rs:6672`). Every manifest's warnings are reported, a dependency's
  included (3.3a §3.3).
- **The language server's watcher** registers `**/*.nova` and
  `**/nova.toml` (`crates/nova-lsp/src/lib.rs:304`). A change to a
  `nova.toml` re-checks every open project (`lib.rs:184`). VS Code watches
  only its workspace folders (3.3a §6). A project is the package whose
  `src/` or `tests/` holds the file (`package_of`, `program.rs:177-188`).
- **M0012 is per manifest.** `names_ok` checks one package's own entries
  (`graph.rs:262-305`); M0011 compares names exactly (`graph.rs:417`).
- **The lockfile** holds 232 packages. Already there: `ring` 0.17.14,
  `flate2` 1.1.10, `crc32fast`, `serde_json` 1.0.151, `semver` 1.0.28. No
  HTTP client and no tar crate.
- **Names.** On 2026-10-09:
  - `Sakeerin/nova-index` did not exist on GitHub (404).
  - `nova-registry` is taken on crates.io (200); `nova-index` is free
    (404). `nova-cli` will depend on the new crate, so v0.3.0 must publish
    it to crates.io whichever other crates 3.6 decides to publish
    (`docs/phase-3-plan.md` §4, 3.6).
- **Gates.** `.github/scripts/gate.sh` refuses to run with any `NOVA_`
  variable set (`:21-25`); 3.3a's `packages-gate.sh` runs after it in CI's
  `install` job (`ci.yml:221-228`).
- **Records that look ahead to 3.3.** ADR 0026 says "3.3 records the exact
  rules, yanking included" (`:65`), and that 3.3 measures GitHub's API
  limits and the index repository's size (`:67`). `docs/phase-3-plan.md`
  §5 says the stored token is "readable only by its owner" (`:393`).

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
  - A line that does not parse is skipped, and the command prints a note
    naming the file and line. The note is the index's, not the compiler's:
    it never stops a command.
- **Names are one name whatever their case.** `Geom` and `geom` share a
  file, and `nova publish` refuses a name whose case differs from one
  already in it. Resolution compares names exactly, as manifests do
  (3.3a's M0008).

### 3.2 `config.json`

At the index's root:

```json
{"dl": "https://github.com/Sakeerin/nova-index/releases/download/{name}-{version}/{name}-{version}.nova-pkg", "api": "Sakeerin/nova-index"}
```

- `dl` is a tarball location template with `{name}` and `{version}`:
  absolute (`https://…`) or relative to the index's root
  (`dl/{name}-{version}.nova-pkg`).
- `api`, when present, is the GitHub repository `nova publish` writes to
  (§7). An index without it is written directly, which only a local index
  allows.

### 3.3 Which index, and when it is read

- **`NOVA_INDEX`** names the index:
  - an `https://` URL;
  - `http://` only for a loopback IP literal, `127.0.0.1` or `[::1]`,
    which tests use;
  - a local directory: an absolute path, or a `file:` URL such as
    `file:///srv/index` or `file:///C:/index` (percent-escapes decoded, and
    on Windows the `/` before the drive letter dropped, as RFC 8089 says).
- **The default** is `https://raw.githubusercontent.com/Sakeerin/nova-index/main/`.
- **One canonical form.** Every index is compared and keyed by its
  canonical form:
  - an HTTP URL with the scheme and host in lower case, no default port,
    and exactly one `/` at the end;
  - a local directory's real path (3.3a's `real_path`), written as
    `file:///` and the path with `/` separators and an upper-case drive
    letter.

  So `file:///c:/x` and `C:\x` are one index.
- **Another URL is another index,** a mirror included: the lock (§4.6) and
  the cache (§5.1) are keyed by the canonical form. Transparent mirrors are
  not in 3.3b (§15).
- **Freshness.** `raw.githubusercontent.com` caches a file for up to five
  minutes, so another machine may not see a new version until then. Reads
  made while publishing, the verification's included, go through the
  GitHub API instead (§6.6, §7).
- **When it is read.**
  - Index files: while resolving (§4.4), and never otherwise. They are
    fetched afresh for each resolution and not kept.
  - `config.json`: when a tarball must be downloaded (`dl`), and by `nova
    login` and `nova publish` (`api`).
  - A complete lockfile with every tarball unpacked means no network at
    all.
- **Redirects** are followed only to `https://` URLs, or to `http://` on a
  loopback IP literal, and never carry the token (§8).

## 4. The resolver and `nova.lock`

### 4.1 What is resolved

- Every version entry of the root's `[dependencies]` and
  `[dev-dependencies]`, resolved together, so a name has one version
  across both.
- The version entries of each path dependency's `[dependencies]`.
- Each registry package's `deps` from its index line, transitively.

The verification of a package to publish resolves its `[dependencies]`
only (§6.6).

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
  every requirement gathered so far, and a chosen version adds its `deps`.
  A conflict backs up to the last choice with another candidate.
- **Locked versions are kept.** A package in `nova.lock` whose locked
  version still meets every requirement keeps it: it is the only
  candidate. Only a package with no locked version, or one whose locked
  version no longer fits, is searched. So editing one entry changes as few
  versions as possible.
- The search is exponential in the worst case. Nova's graphs are small, and
  PubGrub's better explanations are left for later (§15).

### 4.4 When resolution runs

- When a version entry has no locked package, or its locked version no
  longer meets its requirement, or a locked package's own `deps` are not
  all locked, or the lock's `index` is not the canonical form of the index
  in use (§3.3).
- Under `nova update`, which ignores the lock, and `nova update <name>`,
  which unlocks that name only: every other locked version stays, and if
  they make `<name>` impossible to resolve, the error says to run `nova
  update` alone.
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
dependencies = ["json-api"]
```

- `index` is the canonical form (§3.3) of the index the packages came from.
- One `[[package]]` per registry package, sorted by name. `dependencies`
  names its `deps`, so the lock alone says whether it is complete and what
  is still needed. A path package is in the tree, so it is not locked.
- Only the root's `nova.lock` is read; a path dependency's is ignored.
- **Written** by every command that syncs, with `\n` line endings, through
  a temporary file renamed into place:
  - when its contents change;
  - and whenever the sync unpacked a package, even with the same contents,
    so an editor's watcher sees it and re-checks (§5.5).
- **Pruned** as it is written: a package no longer reached from the
  manifests is dropped.
- **Created** the first time a project has a registry dependency. Once it
  has none, the lock is kept with no packages.
- `nova package` leaves it out of the tarball (40-TOOLING §4.6): only
  libraries are published (§6.5).

## 5. The cache, the sync step, and the graph

### 5.1 The registry directory

- `$NOVA_HOME/registry/`, where `$NOVA_HOME` follows `cache_root`'s rule.
  That rule moves into `nova-pm` as a plain function returning
  `$NOVA_HOME`, and each user keeps its own error message: the runtime
  cache's (unchanged, with its four tests) and the registry's ("cannot
  place the package cache: set NOVA_HOME to a writable directory").
- Inside it, each index has its own directory, `<idx>`: the canonical
  form's host, keeping only `a-z`, `0-9`, `.` and `-` (`local` for a local
  index), then `-` and the CRC-32 of the canonical form in 8 hex digits.
  For example `raw.githubusercontent.com-1a2b3c4d`. The rule lives in
  `nova-pm`, which gains `crc32fast` (already locked), because the offline
  graph needs it (§5.4).
- `cache/<idx>/<name>-<version>.nova-pkg` holds the tarballs.
- `src/<idx>/<name>-<version>/` holds the unpacked packages, never edited
  in place.

### 5.2 Downloads and unpacking

- **Size.** A download is cut off at 10 MiB, the packing limit (§6.5).
  Unpacking stops at 100 MiB of contents or 10,000 entries.
- **Checksum.** A tarball's SHA-256 is checked against the locked checksum,
  which came from the index line, before anything is unpacked. A mismatch
  is an error naming both digests, and the download is discarded.
- **Entries.** Every entry must be a regular file or a directory under
  `<name>-<version>/`, with a portable name (§6.5). An absolute path, a `..`
  component, a link, a device file, or two names that differ only in case
  refuse the whole tarball.
- **The manifest** must name the package and the version it was downloaded
  as. Its `[dependencies]` must be exactly the index line's `deps`, and none
  may be a `path` entry. Its `[dev-dependencies]` are never read (3.3a
  §3.3).
- **Two processes at once,** as the runtime cache does it:
  - downloads and unpacks go to temporary names made unique by the process
    id and a counter;
  - a finished one is renamed into place;
  - if the rename fails because the other process got there first, its copy
    is used;
  - a temporary left by a killed process is only ever overwritten, never
    read.

### 5.3 The sync step

`nova_index::sync`:
1. reads the root's manifest and its path packages' manifests. If any has
   an error, the errors are shown and the sync stops;
2. resolves (§4) when §4.4 says so, reading index files;
3. downloads and unpacks each locked package missing from the cache;
4. writes `nova.lock` last (§4.6), so a lock never names a package that is
   not unpacked.

It takes the manifest's text and returns the lock it would write, so `nova
add` can sync a manifest before writing it (§6.1).

**When it runs:**
- before `nova run`, `build`, `check` and `test` when they work on a
  package (a project, or a file argument directly in a package's `src/` or
  `tests/`), after the CLI's own refusals, such as `run` in a library
  (3.3a §5.1);
- as `nova fetch`;
- never on a loose file.

**Errors:**
- the diagnostics of §4.5;
- "cannot reach the index at <url>: <reason>";
- "cannot write the package cache at <dir>: <reason>; set NOVA_HOME to a
  writable directory".

### 5.4 The graph stays offline

- `nova_pm::graph` reads the root's `nova.lock` and finds each version
  entry's locked package in `src/<idx>/<name>-<version>/`, where `<idx>`
  comes from the lock's `index`. From there a registry package is a
  package in a directory: its own manifest, its `[dependencies]` found the
  same way (never by `path`, §5.2), its identity its canonical directory
  (3.3a §3.3). `GraphPackage` records whether it came from the registry.
- **The registry directory.** `nova_pm::graph` finds it through the
  `$NOVA_HOME` rule. A second form takes it explicitly, and `Program`
  gains a constructor that passes it, so tests never set `NOVA_HOME`. The
  existing constructors and the driver's path-based entry points keep
  their signatures. When `$NOVA_HOME` cannot be found, registry entries
  are M0005 with that reason.
- **M0005 changes meaning:** a registry entry the cache cannot satisfy.
  It replaces "registry dependencies arrive with the package index". Each
  case has its own message, on the entry:
  - no locked package: "dependency `json` is not downloaded yet; run `nova
    fetch`";
  - a locked version that no longer meets the entry: "dependency `json` is
    locked at 1.4.1, which does not meet `^2`; run `nova fetch`";
  - a locked package not unpacked: as the first.
- **An unreadable `nova.lock`** is M0016 on the lock, and every registry
  entry is then M0005.
- **A dependency's warnings are not shown.** A warning whose labels are all
  in a registry package's files or manifest is dropped, by the CLI and the
  language server alike, as Cargo caps a dependency's lints. Errors are
  always shown.
- **A root inside the registry directory** is refused by every command
  that syncs or builds: "`<dir>` is a downloaded package in nova's cache;
  it is read only".

### 5.5 The language server

- It never syncs. An entry the cache cannot satisfy is M0005 on the
  manifest entry (3.3a §6's placement).
- Its watcher also registers `**/nova.lock`. A change to a `nova.lock`
  re-checks every open project, as a `nova.toml` does. That is why the
  sync rewrites the lock after unpacking anything (§4.6).
- **A file under the registry directory belongs to no project.** The
  server checks nothing and publishes nothing for it, so opening one, as
  go-to-definition will in 3.4, shows it without diagnostics.
- A registry package is a dependency, so no project publishes diagnostics
  for its files (3.3a's one-owner rule), and its warnings are not shown
  (§5.4).

## 6. Commands

### 6.1 `nova add <name>[@<req>] [--dev]`

- **The argument** is split at `@` before the name is checked.
- **Without `--path`, a registry dependency.**
  - With `@<req>`, the requirement as given.
  - Without, it resolves as if the entry were `"*"`, so the version fits the
    rest of the graph. It then writes the version chosen as Cargo writes it:
    `json = "1.4.1"`, which means `^1.4.1`.
- **The pre-check.** It syncs the proposed manifest in memory (§5.3), then
  builds the graph from the proposed manifest and the proposed lock. Any
  error refuses, and `nova.toml` and `nova.lock` are written together only
  after both pass. That covers M0014, M0015, an unreachable index, and
  3.3a's graph checks (M0008, M0011, M0012).
- **`--path`** does not sync. Its pre-check ignores M0005 as well as M0013,
  since neither is the new entry's fault.
- **`--path` with `@<req>`** is refused: an entry has `version` or `path`,
  never both (M0003).
- 3.3a's other refusals stand: a broken manifest, an entry already in
  either table. The help text becomes "Add a dependency to nova.toml".

### 6.2 `nova remove`

Unchanged. The next sync prunes the lock (§4.6).

### 6.3 `nova update [<name>]`

Re-resolves (§4.4), downloads what is new, writes the lock, and prints each
change, `json 1.2.0 -> 1.4.1`, or "nothing to update".

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
- **Portable names only.** Every path component must be valid on Windows,
  macOS and Linux:
  - no `:`, `\`, `<`, `>`, `"`, `|`, `?`, `*` or control character;
  - no trailing `.` or space;
  - no Windows device name (`con`, `aux`, `nul`, `com1`…, as
    `check_name`'s list);
  - no two paths that differ only in case.
- **Refused, writing nothing:**
  - a manifest with errors;
  - a package without `src/lib.nova`: only a library can be a dependency;
  - each `path` entry in `[dependencies]`, as M0017, "a published package
    cannot have a path dependency; publish `geom` first and depend on its
    version", on the entry. `[dev-dependencies]` may hold path entries: a
    dependent never reads them (3.3a §3.3), and the verification ignores
    them;
  - a symbolic link, or a name that is not portable, anywhere in what
    would be packed;
  - a tarball over 10 MiB.
- It prints the path, the number of files, the size and the SHA-256.

### 6.6 The verification

After packing, `nova package` (and so `nova publish`):
1. unpacks the tarball into a temporary directory;
2. resolves its `[dependencies]` afresh, with no lock and none of its
   dev-dependencies. When publishing to a GitHub index, index files are
   read through the GitHub API (§7), so a dependency published a moment
   ago is seen;
3. checks it as `nova check` does: the library, and the program when
   there is one (3.3a §5.1).

**What stops it:** any error, and any warning in the package's own files or
own manifest. The driver's `check_program` gains a count of the warnings it
showed for that purpose; `nova-cli` runs the check, so `nova-index` does
not depend on the driver.

**What it catches:** a file the tarball leaves out, a dependency that is
not published, and a warning the author did not fix. Tests are not run.

### 6.7 `nova login`

- **Reads a GitHub token from standard input,** one line: `gh auth token |
  nova login`, or `nova login < token.txt`. A terminal on standard input
  is refused, because a typed token would be shown: "pipe the token in,
  e.g. `gh auth token | nova login`".
- **Checks it** with the GitHub API: the `api` repository of the index in
  use (the default unless `NOVA_INDEX` is set) must exist, and the token
  must be able to push to it.
- **Stores it** in `$NOVA_HOME/credentials.toml` as `[github] token = "…"`,
  one token for any GitHub index. The file is written through a temporary
  file renamed into place.
  - On Unix, the temporary file is created with mode `0600` before
    anything is written to it.
  - On Windows, the file takes the permissions of its directory: for the
    default `$NOVA_HOME`, the user's profile (the user, SYSTEM and
    Administrators), as Cargo's credentials are. `nova login` warns when
    `$NOVA_HOME` is outside the profile.
- **Never prints the token.** To log out, delete the file.

### 6.8 `nova publish`

`nova package` with its verification, then publishing to the index in use
(§7). A local index needs no login; a GitHub index needs one.

## 7. Publishing to GitHub

Every request carries the stored token, and goes to `api.github.com` or
`uploads.github.com` only (§8).

1. **Read the package's index file** through the Contents API, which is
   authenticated and never stale. If the version is there, stop: the index
   is append-only, and a version is never overwritten. If a name differing
   only in case is there, stop (§3.1).
2. **Create the release** `<name>-<version>` in the index repository, or
   reuse it if an earlier attempt made it.
3. **Upload the tarball** as that release's asset. An asset already there,
   left by an earlier attempt, is deleted first. That is safe: step 1 found
   no index line, so nothing points at it.
4. **Append the line** to the package's index file with the Contents API,
   passing the file's current `sha`. If GitHub rejects the write because
   the file changed in between, `nova publish` re-reads it, repeats step
   1's checks, and tries once more.

- **The index line is the commit point.** Until it exists nothing points at
  the tarball, so a failure in steps 2 to 4 leaves nothing anyone can
  depend on, and running `nova publish` again finishes the job.
- **The cost:** about six API requests per publish, against GitHub's
  5,000 an hour for an authenticated user. Each version adds one line of
  about 200 bytes to the repository; tarballs are release assets, which do
  not count towards its size. ADR 0031 records these, with what the
  by-hand publish measured (§11.2).
- **Afterwards** it prints the version, its SHA-256, and that other
  machines may take up to five minutes to see it (§3.3).
- **A local index:** write the tarball where `dl` says, creating
  directories, through a temporary file renamed into place, then append
  the line. Step 1's checks apply.

## 8. Security

- **The token** is sent only to `api.github.com` and `uploads.github.com`.
  It is never sent on a redirect, to the index's read host, or with a
  tarball download, and never appears on a command line, in a log or in an
  error message.
- **Tests' API stand-in.** `NOVA_GITHUB_API` replaces both API hosts only
  with `http://127.0.0.1:<port>` or `http://[::1]:<port>`, so the token
  cannot be sent anywhere else.
- **Downloads** are used only after their SHA-256 matches, within the size
  limits (§5.2).
- **Unpacking** refuses anything that could write outside the package's
  directory, or that a Windows or macOS file system would merge (§5.2).
  **Packing** never follows a link (§6.5).
- **A downloaded package is not trusted to point elsewhere:** its
  `[dependencies]` must equal its index line, with no `path` entries
  (§5.2).
- **Plain `http://`** is refused except on a loopback IP literal, and
  redirects follow the same rule (§3.3).
- **The stored token** is owner-only on Unix and relies on the profile's
  permissions on Windows (§6.7). This narrows `docs/phase-3-plan.md` §5's
  "readable only by its owner" on Windows; a dated note says so (§12).
- **The real index** is never written by CI. CI publishes only to a local
  index; every write to `Sakeerin/nova-index` is the user's, or on the
  user's explicit word (`docs/phase-3-plan.md` §5).

## 9. The crates and their dependencies

- **`nova-index`** (`crates/nova-index/`) holds the index client, the sync
  step, downloading, unpacking, packing and publishing. `nova-cli` depends
  on it. The driver and the `nova-lsp` crate do not.
  - It is named `nova-index` because `nova-registry` is taken on crates.io
    (§2).
- **`nova-pm`** gains:
  - the resolver and `nova.lock`, which are pure code;
  - the `$NOVA_HOME` rule, the canonical index form and the `<idx>` name
    (§3.3, §5.1), with `crc32fast`, already locked;
  - the offline graph's reading of the lock and the cache (§5.4).
- **`nova-driver`** gains the warning count (§6.6), the `Program`
  constructor with an explicit registry directory (§5.4), and the dropping
  of a dependency's warnings.
- **New dependencies of `nova-index`:**
  - `ureq = { version = "=3.2.1", default-features = false, features =
    ["rustls"] }`: rustls on the `ring` already locked, and Mozilla's root
    certificates from `webpki-roots`. ureq 3.3.0 and later need Rust 1.85.
  - `tar = "0.4"`, for its deterministic header mode and entry types.
  - `serde` and `serde_json`, already locked, for the index lines and
    `config.json`.
- **The minimum Rust stays 1.78.** Any crate whose newest release needs
  more is held at an older version in the lockfile. On 2026-10-09 that
  means `zeroize` at 1.8.x and `ureq-proto` at 0.5.x.
  - The plan lists every package the lockfile gains, with each version's
    `rust_version` and edition (3.2's lesson).
  - CI's MSRV job runs `cargo check` on Linux only, so the plan also lists
    the crates only Windows or macOS build, and checks those by hand.
- **Proxies.** ureq reads `HTTPS_PROXY`, `HTTP_PROXY` and `NO_PROXY`.

## 10. Testing

None of the tests reach the internet. Any test that syncs sets `NOVA_HOME`
to a fresh temporary directory, and `NOVA_INDEX` to a local index or a
loopback server.

### 10.1 The resolver and the lockfile (`nova-pm`, unit tests)

- The newest matching version; `^0.x`; a pre-release only when named.
- A locked version that still fits is kept; one that no longer fits is
  replaced, and nothing else moves.
- A conflict that backtracking solves, and one it cannot: M0015 naming each
  requirement and its requirer.
- `nova update <name>` changes only that name, and says to run `nova
  update` when the others block it.
- `nova.lock` round-trips; an unreadable one is M0016; pruning drops what
  is no longer reached.
- The canonical index form and `<idx>` for HTTPS URLs with and without a
  port and a trailing `/`, an IPv6 loopback, and Windows `file:` spellings.

### 10.2 The offline graph (`nova-pm`)

- A registry entry found through the lock and an explicit registry
  directory.
- Each M0005 case: not locked, locked but no longer fitting, locked but not
  unpacked; and M0016.
- A downloaded package's manifest with a `path` entry, or with
  `[dependencies]` unlike its index line, is refused.
- A dependency's warning is dropped and its error is kept.

### 10.3 `nova-index`

- Index paths for names of one to five characters, and in mixed case.
- `config.json` with absolute and relative `dl`; an unknown `v` skipped; a
  bad line skipped with its note.
- A local index read and written, given as a path and as `file:` URLs.
- A tarball whose checksum is wrong is refused and discarded; one over the
  size limits is cut off.
- Unpacking refuses `..`, an absolute path, a link, a device entry, a name
  that is not portable, and two names differing only in case.
- Two processes unpacking one package at once both succeed, and leave one
  copy.
- Packing the same source twice gives identical bytes; a symbolic link and
  a non-portable name are refused; a hidden file is left out.

### 10.4 The network, against loopback servers in the test

- An index and tarballs served over `http://127.0.0.1`; a redirect to a
  non-loopback `http://` URL refused.
- A GitHub API stand-in that records every request, for:
  - the four publish steps;
  - the retry on a stale `sha`;
  - the refusal of an existing version;
  - a resumed publish that deletes an earlier attempt's asset;
  - the verification reading index files through the API;
  - no request outside the API hosts carrying the token.

### 10.5 End to end through `nova`

With `NOVA_HOME` in a temporary directory and `NOVA_INDEX` a local index:
- publish `geom`; `nova add geom` in an app; run, build and test it;
- `nova update` after a newer `geom` is published; `nova update geom`
  leaving other names alone;
- `nova fetch`, then a build with the index gone: the lock and the cache
  are enough;
- a refused `nova add` leaves `nova.toml` and `nova.lock` untouched;
  `nova add x@1 --path ..` refused;
- M0017 for a path dependency, a path dev-dependency allowed, and the other
  `nova package` refusals;
- verification failing on a warning in the package, and on a dependency
  that is not published, and passing despite a warning in a dependency;
- a command run inside the registry directory refused.

The tests that pin today's M0005 (§2) change: the graph's to the new
messages, the CLI's to M0014 against an empty local index.

### 10.6 The language server

- M0005 on an entry not yet downloaded, and on one whose locked version no
  longer fits.
- A re-check when `nova.lock` changes.
- Nothing published for a file opened from the registry directory.
- A dependency's warning not shown.

### 10.7 Mutants

Each must fail a named test:
- the checksum check skipped;
- locked versions not kept;
- the append-only check removed;
- the path-dependency refusal removed, at packing and at unpacking;
- `..` allowed when unpacking;
- the token sent with a tarball download;
- the lock written before the downloads.

## 11. The gate

### 11.1 In CI

`.github/scripts/registry-gate.sh NOVA WORKDIR`, in the `install` job after
`packages-gate.sh`, on all three systems, through the installed `nova`. It
sets `NOVA_INDEX` and `NOVA_HOME` only in its own process.
1. Make a local index in `WORKDIR/index`, with a `config.json` whose `dl`
   is relative. Point `NOVA_INDEX` at its absolute path, and `NOVA_HOME`
   at `WORKDIR/home`.
2. `nova new --lib geom`, then `nova publish`.
3. `nova new app`, `nova add geom`, and a `main.nova` that imports it.
4. `nova run` prints from `geom`; `nova build` and the built program print
   the same; `nova test` passes in both packages.
5. With the index directory renamed away, a second `nova build` still
   works: the lock and the cache are enough.

### 11.2 By hand, on the user's word

1. The user creates `Sakeerin/nova-index`, public, with a `config.json`
   (§3.2), and a fine-grained token that can write to it.
2. `gh auth token | nova login` (or the fine-grained token).
3. `nova publish` of a small library.
4. In a second project: `nova add` it from the default index, then `nova
   run`.

The transcript, and the requests and repository size it measured (§7), go
in ADR 0031, with local paths and names removed. 3.3b's merge needs only
§11.1.

## 12. Records

- **ADR 0031, "The package index and publishing":**
  - the index layout, the line format and its `v`;
  - one version per name, and why several majors wait;
  - the cache, the sync step and the offline graph;
  - the security rules of §8;
  - the five-minute staleness, and why a mirror is another index;
  - GitHub's API limits and the repository's size (§7);
  - why `ureq` is pinned, and the crate's name.
- **Dated notes:**
  - `nova-spec/40-TOOLING.md`:
    - §1.1, the command list (`fetch`, `package`, `login`, `publish`);
    - §4.1, M0005's new meaning;
    - §4.2, the lockfile as built;
    - §4.3, one version per name;
    - §4.4, the git-backed index in place of `registry.novalang.dev`;
    - §4.5, the commands as built;
    - §4.6, what a `.nova-pkg` holds, and the verification;
  - `docs/phase-3-plan.md`, its 3.3 entry (3.3b done, the by-hand half's
    status) and §5 (the token on Windows, §8);
  - ADR 0026, the registry decision as built, and that yanking is not in
    3.3 (§15).
- **`CHANGELOG.md`**, and the README's packages section: `nova add` from
  the index, `nova fetch`, `nova login`, `nova publish`.

## 13. Risks

1. **The TLS stack's minimum Rust drifts.** A transitive crate's new
   release can need a newer Rust. Mitigation: exact pins, the committed
   lockfile, CI's MSRV job, and the by-hand check of the platform-only
   crates (§9).
2. **The CDN's staleness** confuses someone who publishes and immediately
   adds from another machine. Mitigation: `nova publish` says so, and reads
   while publishing go through the API.
3. **A half-done publish.** Mitigation: the index line is written last, and
   publishing again resumes (§7).
4. **The token.** Mitigation: §8, and tests that look at every request's
   host.
5. **A malicious tarball.** Mitigation: checksums from the index, size
   limits, unpacking that refuses anything outside the package or not
   portable, and a manifest that must match its index line. The index's
   own integrity rests on GitHub and HTTPS; signing is not in 3.3b.
6. **A stale editor.** The language server sees a fetch only through the
   lock's change, and VS Code watches only its workspace. Mitigation: the
   sync rewrites the lock after unpacking (§4.6). An edit made outside
   the editor to a project it does not have open is not seen until the
   next edit.

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
   unknown or broken lines are skipped with a note (§3.1).
7. **`NOVA_INDEX`** selects the index: `https://`, a local path or a
   `file:` URL; `http://` only on a loopback IP literal (§3.3).
8. **The default read host** is `raw.githubusercontent.com`, with its
   five-minute cache; publishing and its verification read through the
   API (§3.3, §6.6, §7).
9. **`nova.lock` records the canonical index,** and another index
   re-resolves (§4.4, §4.6).
10. **The cache** is keyed by the canonical index's host and CRC-32,
    under `$NOVA_HOME/registry` (§5.1).
11. **M0005 is reused** for a registry entry the cache cannot satisfy
    (§5.4); M0014–M0017 are new (§4.5, §6.5).
12. **`nova fetch`** is added, as the sync step on its own (§6.4).
13. **Only libraries are published,** so a tarball never holds
    `nova.lock` (§6.5).
14. **A release per version,** tagged `<name>-<version>` in the index
    repository (§7).
15. **The size limits:** 10 MiB packed, 100 MiB or 10,000 entries
    unpacked; links and non-portable names are refused (§5.2, §6.5).
16. **`nova login` reads piped standard input only,** and the token is
    stored in `$NOVA_HOME/credentials.toml` (§6.7).
17. **`NOVA_GITHUB_API`** exists for tests, and accepts only a loopback IP
    literal (§8).
18. **The resolver** is a hand-written backtracking search; PubGrub waits
    (§4.3).
19. **The by-hand gate** does not hold up the merge; its transcript goes in
    ADR 0031 when it is done (§11.2).

Taken in correcting the spec after its fact-check, for the user's review:
20. **The lock is written last,** after every download, and rewritten
    whenever something was unpacked, so an editor re-checks (§4.6, §5.3).
21. **Locked versions are kept, not just preferred,** and `nova update
    <name>` moves only that name, as Cargo does (§4.3, §4.4).
22. **The lock lists each package's dependencies,** is pruned as it is
    written, and is created at the first registry dependency (§4.6).
23. **Index files are not cached;** each resolution fetches them afresh
    (§3.3).
24. **The canonical index form, the `<idx>` name and the `$NOVA_HOME`
    rule live in `nova-pm`,** with `crc32fast`; a mirror is another index
    (§3.3, §5.1).
25. **The registry directory reaches the graph** through the `$NOVA_HOME`
    rule, with an explicit form and a `Program` constructor for tests
    (§5.4).
26. **A downloaded package is checked:** its manifest must match its index
    line, with no `path` entries, within the size limits, with portable
    names (§5.2).
27. **The offline graph's M0005** also covers a locked version that no
    longer fits, and an unreadable lock is M0016 (§5.4).
28. **A dependency's warnings are not shown,** and only the package's own
    warnings stop a publish; the driver counts them (§5.4, §6.6).
29. **Commands refuse a root in the registry directory,** and the language
    server gives its files no project (§5.4, §5.5).
30. **Two processes share the cache** as the runtime cache does (§5.2).
31. **`nova add`:**
    - it syncs the proposed manifest in memory;
    - without `@` it resolves `"*"`;
    - `--path` doesn't sync and its pre-check ignores M0005;
    - `--path` with `@` is refused (§6.1).
32. **Path dev-dependencies may be published:** a dependent never reads
    them, and the verification ignores them (§6.5, §6.6).
33. **An earlier attempt's release asset is replaced,** since nothing
    points at it (§7).
34. **The token on Windows** relies on the profile's permissions, as
    Cargo's does, with a warning outside the profile and a dated note on
    the phase plan (§6.7, §8).
35. **The verification reads through the API** when publishing to GitHub,
    and checks the program too when there is one (§6.6).

## 15. Not in 3.3b

- Yanking a version, `nova owner`, and deleting from the index. ADR 0026
  expected yanking in 3.3; a dated note moves it on.
- Several indexes at once, an index per dependency, and transparent
  mirrors.
- Several major versions of one package in one build (decision 5's ADR).
- PubGrub-style conflict explanations.
- Signing packages or index lines.
- A hosted registry server (ADR 0026).
- `nova install`, `nova search`, and `--offline` (a complete lock and cache
  already need no network).
- Reserving names that differ only by `-` and `_`. M0012 checks one
  manifest's own entries, so `json-api` reached from the root and
  `json_api` reached from a dependency coexist; neither manifest can import
  both.
