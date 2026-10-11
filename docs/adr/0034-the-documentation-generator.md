# ADR 0034 — The documentation generator

## Status

Accepted, 2026-10-11 (Phase 3.5a, branch `claude/loving-archimedes-frtf5q`;
spec `docs/superpowers/specs/2026-10-11-phase-3-5a-the-documentation-site-design.md`).

## Context

`nova-spec/40-TOOLING.md` §5 asks for `nova doc`: `///` docs as Markdown,
a static site under `target/doc/`, a search index, light and dark themes,
and `nova test --doc`. The phase plan's 3.5 entry adds "a page per module,
item signatures from the type checker", and a gate: for std and for an
example that uses a dependency, every public item has an anchor and a
link check finds no broken link, and std's site is attached to the GitHub
release. Before 3.5, `nova-doc` was a stub, std had no `///` docs, and no
example used a dependency.

On 2026-10-11 the user split 3.5 in two: 3.5a, the site, std's docs and
the gate; 3.5b, `nova test --doc`.

## Decision

1. **What a site documents.**
   - The root package, from one analysis of one root: `src/lib.nova` when
     there is a library, else `src/main.nova`.
   - A library documents what each module exports; a package that is only
     a program documents every item and member, as `cargo doc` documents a
     binary.
   - Every dependency the program loads, by its exports.
   - std, in every site, from an analysis of std alone, so one `nova`
     writes the same std pages everywhere.
2. **Signatures are shown as written,** comments removed and dedented,
   and each name in them links to where the site documents it, through the
   index the type checker records (ADR 0032). That is "signatures from
   the type checker" in a language whose signatures carry every type.
3. **A page per module,** each item and member with an `id` such as
   `record.Point.field.x`, a page for each of std's primitive types, and
   std's builtin functions on its front page.
4. **Module docs are `//!` lines before a file's first token,** read from
   the lexer's comments. The compiler and the formatter treat them as
   comments.
5. **Docs are Markdown, rendered by `pulldown-cmark`** 0.13.4, pinned,
   with raw HTML escaped, so a dependency cannot put markup or a script
   into a site. Nova code blocks are coloured by the lexer.
6. **The site works from disk:** every URL is relative, search reads a
   script rather than fetching, and nothing comes from the network.
7. **`nova doc` replaces only a `target/doc/` it wrote,** which holds its
   marker, `.nova-doc`.
8. **Every run checks its links,** and reports a broken one as a warning.
9. **std's site is a release asset,** `nova-std-docs-<version>.tar.gz`.

## Alternatives

- **Printing signatures from the checker's types.** It would lose the
  author's layout, bounds and `where` clauses, and say nothing the source
  does not.
- **A page per item, as rustdoc writes.** The phase plan asks for a page
  per module, and a module page with anchors is enough for links and
  search.
- **Markdown by hand.** No new crate, but CommonMark's edge cases would
  differ. The user chose `pulldown-cmark`.
- **std's pages from each project's analysis.** They would then depend on
  which std modules a project loads; one analysis of std keeps them the
  same.
- **Linking into std at a hosted URL.** No host exists, and a site that
  carries std's pages works offline.

## Consequences

- `pulldown-cmark`, `pulldown-cmark-escape` and `unicase` are in
  `Cargo.lock`. All three build on Rust 1.78, and so do `nova-doc`'s
  tests. `nova doc`'s command-line tests are not checked on 1.78: cargo
  1.78 cannot parse the manifest of `assert_cmd` 2.2.1, a test dependency
  locked before this phase, which is why CI's MSRV job checks the
  workspace without `--all-targets`.
- `Analysis` keeps the merged AST, which `nova doc` reads.
- std's public items all have `///` docs, and every module `//!` docs:
  318 items and members in 17 modules. `std_docs.rs` keeps them so.
- std's docs say what the code does, and were probed where they could be.
  Where a maintainer comment leaves a behaviour unspecified, such as how
  `NaN` orders, the doc says so and promises nothing.
- The docs moved std's lines. Each commit that moved them re-pointed the
  live citations of those lines: in tests' comments,
  `nova-spec/20-STDLIB.md`, std's own comments and
  `docs/benchmarks/README.md`. Dated records keep the lines of their time.
- Code blocks are rendered but not run until 3.5b.
- A package whose name is `std` in any letter case cannot be documented,
  since its pages would share std's directory; nor can two packages whose
  names differ only in case.

## References

- `docs/superpowers/specs/2026-10-11-phase-3-5a-the-documentation-site-design.md`
- `docs/superpowers/plans/2026-10-11-phase-3-5a-the-documentation-site.md`
- ADR 0026 (Phase 3's scope), ADR 0030 (package modules), ADR 0032
  (navigation, and the index)
- `docs/phase-3-plan.md` §3 decision 9, and §4's 3.5 entry
