# ADR 0030: Package modules

## Status

Accepted, 2026-10-08 (Phase 3.3a, branch `phase-3-3a-local-packages`;
spec `docs/superpowers/specs/2026-10-08-phase-3-3a-local-packages-design.md`).

## Context

ADR 0003 gave Nova modules: one file each, named by its stem, all in the
entry's directory, found through one program-wide map from name to module.
Phase 3.3 adds packages. A program depends on libraries by path, and each
library brings its own files. Two packages may both have a `utils.nova`,
and a package's `tests/` may have one too. One map from name to module
cannot hold them.

## Decision

1. **A module is (package, directory, file stem).** The directory is
   `src` or `tests`. A file in no package's `src/` or `tests/` is loose,
   and resolves as before.
2. **The loader decides what an import names,** by package, and gives the
   resolver a table per module. The resolver looks each import up in its
   module's table; a module's name is only a label. A loose program's
   tables are filled by name, which is the old lookup.
3. **What `import x` names**, in a module of package P:
   - P's own `x.nova`, in the same directory;
   - else the library of P's dependency whose import name is `x`. An
     import name is the package's name with each `-` replaced by `_`;
   - a test module also sees the root's dev-dependencies, and the package
     itself;
   - **both** a file and a dependency is E0004. Neither wins;
   - a dev-dependency imported from `src/` is E0001, with a note;
   - a file matches only in its exact case, on every system.
4. **A library's API is its `lib.nova`.** Re-exports and `geom::utils`
   paths stay deferred (ADR 0003, ADR 0025), so a dependent names only
   `lib.nova`'s `pub` items. Library authors put their public types there.
5. **A program's `main` is its entry's.** Every other module's `main` is
   renamed before MIR. An entry without one is E0601, even when a
   dependency declares a `main`.
6. **A library is checked as a module.** MIR runs only when the entry is
   `src/main.nova` or a loose file. Module mode skips MIR's checks: E0011,
   E0013, E0075, E0078 and E0079. So a library can pass `nova check` and
   still fail in a dependent's build.
7. **Coherence stays program-wide,** with no orphan rule. Two packages that
   implement one trait for one type conflict, with today's error, when one
   program uses both.
8. **The language server: one owner per file.**
   - A project's analysis publishes only its own package's modules and its
     own `nova.toml`.
   - A dependency's problem shows on the dependent's manifest entry, with
     where it really is in the message.
   - The checker shares each project's package directories with the
     protocol thread, so an edit in an open dependency re-checks its
     dependents.
   - The client watches files only in its workspace folders. A dependency
     outside them is re-checked on edits and saves in the editor, but not
     when another program changes its files.

## Alternatives

- **Package-qualified names in the resolver's one map**, `geom/utils`.
  Rejected: the resolver would learn packages, dependencies and
  dev-dependencies, which the loader already knows, and the clash rule
  would live in two places.
- **Compiling each package separately,** with an interface per library.
  Rejected for now: Nova has no interface format, and symbols are already
  mangled by `DefId`, so one program-wide compile has no clashes to avoid.
- **A file shadowing a dependency of the same name, or the reverse.**
  Rejected: whichever won, adding a file or a dependency would silently
  change what an existing import means. E0004 makes the user choose.

## Consequences

- Every compile goes through the loader. A loose program resolves exactly
  as before, except that imports match case on Windows and macOS too.
- `nova run src/main.nova` inside a project reads its manifest; any other
  file argument is still loose (the 3.0 spec's §6.1, amended).
- `nova test` runs the root's `src/` tests that the roots reach, and every
  top-level `tests/*.nova`. A dependency's tests never run in its
  dependent's binary.
- A dependency is compiled into each program that uses it; it never gets a
  `target/` of its own.

## References

- `docs/superpowers/specs/2026-10-08-phase-3-3a-local-packages-design.md`
- `docs/superpowers/plans/2026-10-08-phase-3-3a-local-packages.md`
- ADR 0003 (the module model), ADR 0025 (Phase 2's boundary and backlog),
  ADR 0029 (the language server)
- `docs/phase-3-plan.md` §3, decision 3, and §4's 3.3 entry
