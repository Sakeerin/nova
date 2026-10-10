# ADR 0033 — Fixes and colour in the language server

## Status

Accepted, 2026-10-10 (Phase 3.4b, branch `phase-3-4b-fixes-colour`; spec
`docs/superpowers/specs/2026-10-10-phase-3-4b-fixes-and-colour-design.md`).

## Context

3.4a gave `nova lsp` hover, go to definition, find references and rename,
from an index the type checker records (ADR 0032). The phase plan's 3.4
entry also asks for suggested edits on `Diagnostic`, code actions that
offer them, organize imports in `40-TOOLING.md` §2.1's order, and semantic
highlighting. On 2026-10-10 the user put those in 3.4b.

## Decision

1. **Fixes are made where the error is found.**
   - `Diagnostic` gains `fixes`: a title and text edits, which may reach
     another file.
   - The resolver and the checker attach them as they raise the errors,
     since each site knows what its fix needs. The three fixes placed by
     lines read the source text from the `FileDb`.
   - The fixes are the determined ones (the user, 2026-10-10): make a
     `let` or a parameter mutable (E0060); import a name exactly one
     importable module exports, of the kind its site needs; "did you
     mean", within rustc's bound; make an item public in the importer's
     own package; remove an unreachable arm (E0021).
2. **The command line prints each fix as `= help:`,** and E0060's fix
   replaces its note.
3. **Code actions** offer each fix of a diagnostic the request overlaps,
   once, and organize imports. A request analyses afresh, but no offered
   fix is re-checked: tests apply every kind of fix and analyse again, and
   the broken-program sweep applies every fix it meets. A cut program
   loses only its last item, whole, so its fixes come from one planted
   program, swept whole and at 16 cuts among the 381.
4. **Organize imports** gathers a file's imports into one block: the
   dependencies, then the project's own modules, a blank line between.
   That is §2.1's "std first then third-party", in a Nova whose std needs
   no import. Imports of one module merge. Unused imports go only from a
   file without errors, while no file has a parse error.
5. **`nova fmt` keeps blank-line-separated import groups,** as gofmt does
   (the user, 2026-10-10), so format on save keeps organize imports'
   groups (ADR 0028's note).
6. **Semantic tokens cover names only,** for whole documents, by what the
   index says each name means.
7. **The budgets** are 200 ms for code actions and semantic tokens, for the
   median and the maximum of 20 on `05-json-api` in a release build. CI's
   bound is 2 s with the debug binary. Measured on 2026-10-10:

   | Run | Edit, median / max | Completion | Hover | Definition | References | Rename | Code actions | Tokens | Binary |
   |---|---|---|---|---|---|---|---|---|---|
   | 1 | 19 / 20 ms | 15 / 17 ms | 16 / 17 ms | 14 / 16 ms | 13 / 15 ms | 28 / 30 ms | 14 / 16 ms | 13 / 15 ms | `target/release/nova.exe`, 17,925,632 bytes, built 2026-10-10 13:46 UTC |
   | 2 | 19 / 20 ms | 15 / 16 ms | 15 / 16 ms | 15 / 18 ms | 13 / 16 ms | 28 / 32 ms | 13 / 16 ms | 13 / 15 ms | the same binary |
   | 3 | 19 / 23 ms | 15 / 16 ms | 16 / 18 ms | 14 / 17 ms | 13 / 16 ms | 27 / 32 ms | 14 / 16 ms | 13 / 14 ms | the same binary |

## Alternatives

- **A fixer pass after analysis.** It would recover context the checker
  had discarded, such as the names in scope at a point.
- **A structured suggestion, rendered later.** Each fix would be split
  across two crates, and `nova-diagnostics` would name the checker's
  concepts.
- **Re-checking each offered fix.** Code actions are asked for on every
  cursor move; the tests carry the promise instead.
- **The formatter grouping by kind.** `nova fmt --stdin` and loose files
  have no manifest to tell a package from a module.
- **Every token, not names only.** The TextMate grammar already colours
  keywords, literals and comments.

## Consequences

- A fix can expose an error at its own place: "did you mean" may name
  something of another type, and an imported function's parameter types
  are not checked against the call.
- The import fix sees only the modules the program loads. A module nothing
  imports yet is never offered.
- A user's file with a blank line between imports keeps it, and its
  groups are sorted separately.
- E0060's note for a match binding or a `for` variable still advises
  `let mut`, which does not parse there.
- Fix-all, `codeAction/resolve`, and delta and range token requests are
  not offered.
- `Diagnostic` is 128 bytes with its fixes, clippy's default threshold
  for `result_large_err`; the workspace's `clippy.toml` raises the
  threshold to 192 bytes, since `Result<_, Diagnostic>` is how the front
  end and the package manager report an error.

## References

- `docs/superpowers/specs/2026-10-10-phase-3-4b-fixes-and-colour-design.md`
- `docs/superpowers/plans/2026-10-10-phase-3-4b-fixes-and-colour.md`
- ADR 0028 (the formatter), ADR 0029 (the language server), ADR 0032
  (navigation)
- `docs/phase-3-plan.md` §4, the 3.4 entry
