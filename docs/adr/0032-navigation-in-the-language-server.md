# ADR 0032 — Navigation in the language server

## Status

Accepted, 2026-10-10 (Phase 3.4a, branch `phase-3-4a-navigation`; spec
`docs/superpowers/specs/2026-10-10-phase-3-4a-navigation-design.md`).

## Context

3.2 gave `nova lsp` diagnostics, completion and formatting, and left
hover, go to definition, find references and rename to 3.4 (ADR 0029).
The server re-runs the front end on each request. Completion uses a probe
that looks at one place in the program. ADR 0029 set aside a full
position index, because it "makes every check pay for tables only the
server reads".

On 2026-10-10 the user split 3.4 in two: 3.4a, navigation, and 3.4b,
fixes and colour.

## Decision

1. **An index of name occurrences.**
   - The type checker records it where it already decides what a name
     means. The resolver records the import items' occurrences.
   - Each occurrence has its span, whether it declares or uses, and its
     target: a definition, variant, field, trait method, local, type
     parameter, module, builtin or primitive.
   - The types live in `nova-resolver` (`index.rs`), because
     `nova-typeck` depends on it.
   - The index is off unless the server asks for it, so `nova check`,
     `build`, `run`, `test` and the server's diagnostics pay nothing. That
     answers ADR 0029's reason for setting a full index aside.
2. **Recording happens only at the AST's name sites,** with duplicates
   removed. The checker passes some names twice, and makes names of its
   own: a `for` loop's `next`, interpolation's `fmt`, and temporaries.
   Two checks over std, `examples/` and `tests/runtime/` (159 files) keep
   the index complete:
   - every use has exactly one declaration;
   - every identifier is covered.
3. **A trait method and the impl methods that implement it are one
   family** for references and rename. A call resolves through the trait,
   so without families an impl method would have no uses.
4. **References and rename reach the owning project only** (the user,
   2026-10-10). That is its program, library and `tests/`.
5. **Rename checks itself.** It applies its edits in memory and analyses
   the program again. It refuses, saying why, when:
   - a renamed name would resolve elsewhere;
   - another name would come to mean the renamed one;
   - an error code would become more common.
6. **Std's sources go on disk** at `$NOVA_HOME/std/<version>-<crc32>/`,
   one read-only file per module. Each file is compared with the embedded
   text and rewritten through a temporary name when it differs. A request
   inside that cache is answered from a program held in memory.
7. **Requests re-run the front end** with the index on. Nothing is cached
   between requests.
8. **The budgets** are 200 ms for hover, definition and references, and
   400 ms for rename, which analyses twice, for the median and the
   maximum of 20 on `05-json-api` in a release build. CI's bounds are 2 s
   and 4 s with the debug binary. Measured on 2026-10-10:

   | Run | Edit to diagnostics, median / max | Completion | Hover | Definition | References | Rename | Binary |
   |---|---|---|---|---|---|---|---|
   | 1 | 19 / 20 ms | 17 / 19 ms | 15 / 19 ms | 17 / 19 ms | 16 / 17 ms | 28 / 41 ms | `target/release/nova.exe`, 17,680,896 bytes, built 2026-10-10 03:33 UTC |
   | 2 | 21 / 26 ms | 16 / 19 ms | 19 / 29 ms | 15 / 17 ms | 13 / 16 ms | 27 / 30 ms | the same binary |
   | 3 | 19 / 20 ms | 14 / 15 ms | 15 / 15 ms | 14 / 17 ms | 13 / 15 ms | 26 / 29 ms | the same binary |

   Edits and completions do not record the index. Their medians, 19-21
   ms and 14-17 ms, sit beside ADR 0029's 17-18 ms and 13 ms, measured
   with a different binary (14,534,144 bytes then, 17,680,896 now).

## Alternatives

- **A separate indexer in `nova-lsp`.** It would duplicate the rules for
  shadowing, match bindings, closures and method lookup through traits,
  and drift from the checker's.
- **Growing 3.2's probe.** It answers one place. References and rename
  need every occurrence, so it would still need this index.
- **Rules for rename conflicts.** Re-analysis catches shadowing,
  captures, clashes and duplicates with one mechanism, as the formatter
  checks its own output.
- **Reaching a library's open dependents.** The answer would change with
  which files are open. The user chose the owning project.
- **Caching the last analysis.** It would need invalidation on every
  edit, for a cost ADR 0029 measured at 13-21 ms.

## Consequences

- The checker has a write-only recording path at its name sites. A new
  construct that resolves a name needs a recording place, and the
  completeness checks fail until it has one.
- Forms the checker refuses today are not navigable:
  - type aliases;
  - `import … as`;
  - record, tuple, array, or, range and `x @ pat` patterns.
- A request inside a downloaded package is answered from that package's
  own analysis, which may not resolve its own registry dependencies.
- Std's cache gains a directory for each version and build. Nothing
  removes old ones, as with the runtime's cache (ADR 0027).

## References

- `docs/superpowers/specs/2026-10-10-phase-3-4a-navigation-design.md`
- `docs/superpowers/plans/2026-10-10-phase-3-4a-navigation.md`
- ADR 0027 (the runtime's cache), ADR 0029 (the language server)
- `docs/phase-3-plan.md` §4, the 3.4 entry
