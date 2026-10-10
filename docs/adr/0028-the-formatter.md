# ADR 0028 — The formatter

## Status

Accepted (2026-10-07). Branch `phase-3-1-formatter`
(`docs/superpowers/specs/2026-10-07-phase-3-1-formatter-design.md`;
`docs/phase-3-plan.md` §3 decision 6).

## Context

The master spec's Phase 3 list opens with "`crates/nova-fmt` — formatter (no
options, only `--check`)", and `nova-spec/40-TOOLING.md` §2 describes it:
gofmt's fixed style, built on Wadler's pretty printer. Before 3.1, `nova-fmt`
was a one-line stub, and the front end could not support it:

- the lexer dropped comments, so a formatter printing from the AST would
  have deleted every one;
- `///` lexed as a doc comment that no grammar rule accepted, so no file
  could hold one;
- the AST keeps literal values, not their spellings, and records no
  parentheses, blank lines or comments.

## Decision

1. **Modes.** `nova fmt [PATH]... [--check] [--stdin]`. With no path it
   formats the project's `src/`, found as 3.0's commands find the project;
   paths name files or directories, searched for `*.nova` without `target/`
   or dot-directories. `--check` writes nothing and exits 1 if a file would
   change; `--stdin` formats standard input to standard output. Errors exit
   2 and outrank a would-change. The master spec's "only `--check`" is read
   as "no style options": `--check` and `--stdin` choose what the command
   reads and writes, never how code looks.
2. **Canonical output.** The layout depends only on the code, its comments
   and its blank lines: lines of 100 columns, counted in Unicode scalar
   values, 4-space indentation, one blank line between top-level items, and
   at most one wherever the author left any.
3. **The author's parentheses are kept,** one pair per node, and none are
   added. No precedence model decides where parentheses go.
4. **Doc comments.** `///` documents the item, member, field or variant
   after it, and lands in that node's `docs`; anywhere else it is a parse
   error. `////` is a plain comment. An unterminated `/*` is
   `UnterminatedBlockComment`.
5. **Separators.** Statements print without `;`, and match arms without
   `,`, except where the parser, which ignores line breaks, would read two
   as one: before one that begins with `(`, `[`, `{`, `-`, `*`, `&` or `|`,
   and after a `return` or `break` with no value.
6. **Line endings.** `.editorconfig`'s `end_of_line` if it says `lf` or
   `crlf`, otherwise the file's own, otherwise LF; one final newline unless
   `insert_final_newline = false`. A multi-line string literal holds its
   file's line ending, so it keeps it, and changes with it if the file's
   changes.
7. **The self-check, on every call.** The output is lexed and parsed again,
   and must give the input's AST, ignoring spans and with import runs
   sorted, and the input's comments. Otherwise nothing is returned, and
   `nova fmt` leaves the file unchanged and says so.

## Alternatives

- **A lossless syntax tree.** A parser that keeps every token and comment,
  with the AST derived from it, is the sturdiest base, and 3.2's LSP could
  use it. It is a phase-sized rewrite of the parser that touches every AST
  consumer.
- **Re-indenting tokens by bracket depth.** Simple, and safe for comments,
  but it cannot break lines by width or sort imports, so it misses
  `40-TOOLING.md` §2.1 and §2.2.
- **Removing redundant parentheses.** It needs a precedence model, and the
  author's parentheses often say something.
- **Style options.** `40-TOOLING.md` §2.1 rules them out.

## Consequences

- `std/` and `examples/` are in `nova fmt`'s layout, and CI's Test job
  checks that they stay so on all three systems.
- A file `nova fmt` cannot handle is refused, never damaged. A refusal is a
  formatter bug, and the file is its reproduction.
- The self-check cannot see two things: impl member order, which the AST
  keeps in three lists, and where a comment sits among the tokens. Tests pin
  both.
- A call whose last argument is a closure with two or more statements breaks
  its whole argument list. Hugging the last argument, as rustfmt does, can
  come later; no corpus file needs it.
- `nova-fmt`'s `format_file` and `format_text` are the library entry points
  3.2's LSP will call.
- **Amended 2026-10-10 (branch `phase-3-4b-fixes-colour`):** a blank
  line between imports now ends a group, as in gofmt, and each group is
  sorted on its own. The self-check is unchanged: a group's sort is a
  sort within its run (ADR 0033).

## References

- `docs/superpowers/specs/2026-10-07-phase-3-1-formatter-design.md`, and
  its plan, `docs/superpowers/plans/2026-10-07-phase-3-1-formatter.md`.
- `nova-spec/40-TOOLING.md` §2; `docs/phase-3-plan.md` §3 decision 6 and
  §4's 3.1 entry; ADR 0026, Phase 3's scope.
