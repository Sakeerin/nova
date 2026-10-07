# ADR 0027 — The runtime library and std inside an installed `nova`

## Status

Accepted (2026-10-07). Branch `phase-3-0-foundations`
(`docs/superpowers/specs/2026-10-07-phase-3-0-foundations-design.md` §3 and
§4; `docs/phase-3-plan.md` §3 decision 1, with its spike result).

## Context

Before Phase 3.0, `nova` worked only beside a checkout:

- **The runtime library.** `nova build` and `nova test` link the runtime's
  staticlib, which `find_runtime_lib` (`crates/nova-driver/src/link.rs`)
  looked for at `NOVA_RUNTIME_LIB`, or in the executable's directory and
  the two above it. `cargo install` installs only the executable.
- **std.** `nova-resolver` embedded std with
  `include_str!("../../../std/...")`. Those paths leave the crate, so a
  packaged `nova-resolver` cannot carry them.

Cargo's artifact dependencies, which would hand `nova-cli` the runtime's
staticlib directly, are unstable. A spike on 2026-10-06, with stand-in
crates, found a route on stable Cargo (the plan's decision 1).

## Decision

1. **std is the crate `nova-std`,** whose package directory is `std/`. Its
   `lib.rs` embeds each `std/*/lib.nova` through a path inside the package,
   so `cargo package` keeps all 17 files, and its verification step builds
   the packaged copy on its own. `nova-resolver` takes the sources from it.
2. **The runtime library is built inside `nova-cli`'s build.**
   - `nova-runtime` declares `links = "nova_runtime"`, and its build script
     reports its source directory. That reaches `nova-cli`'s build script
     as `DEP_NOVA_RUNTIME_MANIFEST_DIR`.
   - `nova-cli`'s build script runs a nested
     `cargo build --lib --release --target $TARGET` into its own `OUT_DIR`.
   - In a workspace (a checkout, `cargo install --path` or
     `cargo install --git`), it builds in place with `--locked`.
   - From a package, it builds a copy without `Cargo.lock`, with an empty
     `[workspace]` table, `--offline`.
   - The runtime manifest's workspace keys choose the route; a file such as
     `Cargo.toml.orig` could be a stray merge backup.
   - The library is gzip-compressed with flate2 and embedded with
     `include_bytes!`. Only release-profile builds embed, unless
     `NOVA_EMBED_RUNTIME` says `1` or `0`.
3. **`nova` finds a runtime library** at `NOVA_RUNTIME_LIB`, then in the
   runtime it carries, then in the executable's directory and the two above
   it.
   - It unpacks the runtime it carries to
     `$NOVA_HOME/runtime/<version>-<crc32>/`. `NOVA_HOME` defaults to
     `.nova` in the home directory.
   - It writes a new temporary file and renames it into place, and it
     checks the file's size and CRC-32 on every use.
   - A runtime it carries but cannot unpack is an error, not a fall-back.
4. **`nova version`** reports whether the runtime is embedded.

## Alternatives

- **Download the library from a GitHub release on first use.** This needs
  a published release, which an unreleased commit lacks, and network
  access at link time. It stays a fallback, if embedding ever fails.
- **Ship the library beside `nova` in release archives.** This covers
  archives, but not `cargo install`, which installs only the executable.
- **Cargo's artifact dependencies** (`-Z bindeps`): unstable.

## Consequences

- **Size.** Measured on this branch: `nova.exe` went from 7,233,536 to
  12,319,232 bytes on Windows, and the payload is 4,674,243 bytes on
  Windows and 7,962,350 bytes on Linux. Decision 1's estimate was a 4.6 MB
  payload on a 7.2 MB Windows `nova`, about two thirds more, and 7.9 MB on
  an 8.2 MB Linux one, about double.
- **Install time.** A release build of `nova-cli` builds the runtime a
  second time, in release mode. A cold `cargo install` took 91 s before
  and 88 s after, on the development host: the nested build runs alongside
  the rest of the build, so it showed no cost there. CI's smaller runners
  may show one.
- **A stale copy can hide in release builds.** A release `nova` links the
  runtime it carries before one beside it. After editing the runtime,
  rebuild `nova-cli` too, whose build script reruns on the runtime's
  sources, or set `NOVA_RUNTIME_LIB`. Rebuilding `nova-runtime` alone is
  not enough.
- **The cache only grows:** one directory per version and build. No Phase
  3 command removes old ones (ADR 0026).
- **The package route first runs for real at 3.6's crates.io install.**
  3.0 tests only its decisions and its copy.

## References

- `docs/phase-3-plan.md` §2 items 1 and 8, and §3 decision 1.
- `docs/superpowers/specs/2026-10-07-phase-3-0-foundations-design.md` §3,
  §4 and §11.
- `crates/nova-cli/build.rs`, `crates/nova-cli/build_support.rs`,
  `crates/nova-driver/src/runtime_cache.rs` and `std/lib.rs`.
