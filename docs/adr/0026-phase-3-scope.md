# ADR 0026 — Phase 3's scope: a git-backed package index, and what the phase leaves out

## Status

Accepted (2026-10-07). Branch `phase-3-0-foundations`. The decisions are the
user's, made on 2026-10-06 (`docs/phase-3-plan.md` §1, and §3 decision 10).

## Context

`nova-spec/00-MASTER-SPEC.md` §3 lists Phase 3, "Tooling", in eight
positions:

1. the formatter;
2. `nova-pm`, with `nova.toml` and a lock file;
3. a registry server, in a separate repository, on Rust, Postgres and S3;
4. the LSP server;
5. the VSCode extension;
6. `nova doc`;
7. a REPL;
8. a debugger.

Its gate: "External user can `cargo install nova-cli`, init a project, write
code with autocomplete, format, and publish a package."

`nova-spec/40-TOOLING.md` describes more than those positions need. It
includes §4.4's registry at `registry.novalang.dev`, §8's linter, §9's CI
template, and commands such as `nova install` and `nova self update`. No
registry server or domain exists, and running one is an operations
commitment the gate does not require.

## Decision

1. **The registry is a git-backed index,** in place of position 3's
   registry server.
   - A public GitHub repository the user owns holds `40-TOOLING.md` §4.4's
     Cargo-style sparse index: one file per package, and one JSON line per
     version, with its SHA-256.
   - Package tarballs (`.nova-pkg`, §4.6) are attached to that repository
     as release assets.
   - `nova publish` uploads the tarball, and commits the index line through
     GitHub's REST API.
   - The index is read over plain HTTPS, so any static host can mirror it,
     and a hosted registry can serve the same index later.
2. **Phase 3 leaves out:**
   - the REPL (`40-TOOLING.md` §6; position 7) and the debugger
     (position 8);
   - `nova lint` (§8) and `nova bench`;
   - `nova install`, `nova clean`, `nova owner`, and `nova self update` and
     `nova self uninstall` (§1.1, §4.5, §10.3);
   - §9's CI template and its `setup-nova` action;
   - §7's parallel test runs and TAP output, and §4.3's edition rules;
   - the Zed and Neovim extensions (§3.3);
   - the LSP rows §3.1 marks Phase 4 (inlay hints, code lens, call
     hierarchy), and the Phase 4 commands in §1.1 (`build --target wasm`,
     `bundle`, `dev`).
3. **A sub-phase's spec may pull one of these back in,** with a ruling
   that names this ADR.

## Consequences

- **Publishing needs a GitHub token** with write access to the index
  repository, which `nova login` stores. 3.3's spec decides how it is
  stored.
- **The index is append-only:** a published version's line never changes.
  3.3 records the exact rules, yanking included.
- **No server to run.** The costs move to GitHub's API limits and to the
  index repository's size, which 3.3 measures.
- **A REPL and a debugger wait for a later phase.** The master spec's
  Phase 3 list is not amended; this ADR records the difference.
- **Nothing removes old runtime caches** (ADR 0027), because `nova clean`
  and `nova self` are out of this phase.

## References

- `nova-spec/00-MASTER-SPEC.md` §3, Phase 3.
- `nova-spec/40-TOOLING.md` §1.1, §3, §4, and §6 to §10.
- `docs/phase-3-plan.md` §1, and §3 decisions 4 and 10.
- `docs/superpowers/specs/2026-10-07-phase-3-0-foundations-design.md` §10.
