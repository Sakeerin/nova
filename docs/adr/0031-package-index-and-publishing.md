# ADR 0031: The package index and publishing

## Status

Accepted, 2026-10-09 (Phase 3.3b, branch `phase-3-3b-index-publishing`;
spec `docs/superpowers/specs/2026-10-09-phase-3-3b-index-and-publishing-design.md`).

## Context

ADR 0026 made the registry a git-backed index: a public GitHub repository
holding `40-TOOLING.md` §4.4's sparse index, with tarballs as release
assets, written through GitHub's REST API and read over HTTPS. ADR 0030
gave Nova packages with path dependencies. Phase 3.3b adds what a version
entry needs: an index to resolve against, a lock, a cache, and the
commands that publish.

## Decision

1. **The index** is Cargo's sparse layout, in lower case: `ge/om/geom`
   holds `geom`'s versions, one JSON line each, `{"name", "vers", "deps",
   "cksum", "v"}`. `v` is the line format's version; a reader skips a
   line whose `v` it does not know, so the format can grow. A line that
   does not parse is skipped with a note. `config.json` at the root gives
   `dl`, where tarballs are, and `api`, the repository publishing writes.
   `NOVA_INDEX` names an index: `https://`, `http://` on a loopback IP for
   tests, a `file:` URL or an absolute path. The default is
   `Sakeerin/nova-index` on raw.githubusercontent.com.
2. **One canonical form per index.** URLs and paths are compared as one
   form, which keys `nova.lock` and the cache. A mirror is another index:
   transparent mirrors wait.
3. **One version of each name per build,** with Cargo's caret
   requirements, and pre-releases only when a requirement names one.
   Several major versions side by side would need each package's imports
   to name a version, which waits for its own ADR. The resolver is a
   backtracking search in a fixed order. It keeps every locked version
   that still fits, so editing one entry moves as few versions as
   possible.
4. **`nova.lock`** records the index and each registry package's version,
   SHA-256 and dependencies. It is written last by every command that
   syncs, so it never names a package that is not unpacked.
5. **The graph stays offline.** `nova run`, `build`, `check` and `test`
   sync first: they resolve when the lock does not answer the manifests,
   and download what the cache lacks. The driver and the language server
   never touch the network. They read the lock and
   `$NOVA_HOME/registry/src/<idx>/<name>-<version>/`. An entry the cache
   cannot satisfy is M0005; M0014 to M0017 are the index's errors. A
   dependency's warnings are not shown.
6. **A downloaded package is not trusted:**
   - its SHA-256 must match the lock's before it is unpacked;
   - its size and entry count are bounded;
   - every entry must be a file or a directory inside the package, with
     a name every system can hold;
   - its manifest must match its index line, with no path dependency.
7. **Publishing** packs the library reproducibly, verifies the tarball,
   and only then writes. The verification unpacks it, resolves it afresh,
   and checks it with no error and no warning of its own. To GitHub, it:
   - reads the index file through the Contents API;
   - creates the release `<name>-<version>`;
   - uploads the tarball, replacing an earlier attempt's asset;
   - appends the line with the file's `sha`, which is the commit point.

   The index is append-only. A name keeps the case it was first published
   with.
8. **The token** is stored by `nova login` from a pipe, owner-only on
   Unix. It is sent only to api.github.com and uploads.github.com, never
   on a redirect, and never printed. On Windows the file takes its
   directory's permissions: the user's profile by default, as Cargo's
   credentials do.
9. **The network crate is `ureq`, pinned at 3.2.1,** with rustls on the
   `ring` the runtime already uses. It needs nothing installed beside
   `nova`, and builds on Rust 1.78; ureq 3.3.0 and later need 1.85. The
   crate holding all of this is `nova-index`, because `nova-registry` is
   taken on crates.io.

## Alternatives

- **Resolving in the driver**, so every build could touch the network.
  Refused: the language server would hang on a slow index, and a build
  with no network should work from the lock.
- **Shelling out to `git` or `curl`.** Refused: `nova` would need them
  installed, and their versions and proxies would vary.
- **Several major versions of one package in a build.** Waits: imports
  would have to name a version.
- **Reading the raw files while publishing.** Refused: raw.githubusercontent.com
  caches for up to five minutes, so a dependency published a moment ago
  would be missing. Publishing reads through the API.

## Consequences

- **Freshness.** Another machine may not see a new version for up to five
  minutes; `nova publish` says so.
- **GitHub's limits.** A publish takes about six API requests, against
  5,000 an hour for an authenticated user. Each version adds about 200
  bytes to the index repository; tarballs are release assets, which do
  not count towards its size. The by-hand publish (spec §11.2) measures
  these, and its figures are added here when it runs.
- **Yanking** is not in 3.3; a published version stays.
- **A stale editor.** The language server sees a fetch only through the
  lock's change, which is why a sync rewrites the lock after unpacking.
- **The cache is never cleaned:** `nova clean` is out of Phase 3 (ADR
  0026).

## References

- `nova-spec/40-TOOLING.md` §1.1, §4 and §10.1, and their dated notes.
- `docs/adr/0026-phase-3-scope.md` and `docs/adr/0030-package-modules.md`.
- `docs/phase-3-plan.md` §3 decisions 4 and 5, and §5.
