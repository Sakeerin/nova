# ADR 0043 — Component syntax

## Status

Accepted, 2026-10-17 (Phase 4.4, branch
`claude/loving-archimedes-frtf5q-44`; spec
`docs/superpowers/specs/2026-10-17-phase-4-4-component-syntax-design.md`;
RFC 0001, `docs/rfcs/0001-component-syntax.md`). `count.value()`,
attributes by name, reactivity by literalness, `||` everywhere with the
repository reformatted, `wasm-opt` when it is on `PATH`, no `if` or `for`
in `view`, and a project's `index.html` at its root are the user's
decisions of 2026-10-17.

## Context

After 4.3 a program built its page with builder calls of two arrays:
`div([class("counter")], [dyn_text(| | "${count.value()}")])`.
`nova-spec/30-FRONTEND.md` §3 writes the same page as
`component Counter() { view { div(class: "counter") { text(…) } } }`. The
language 4.4 starts from has:
- a lexer that skips line breaks, so a statement that begins with `(` or
  `|` continues the one before;
- 32 reserved words, kept equal to the lexer's tokens and the extension's
  grammar by tests, and no contextual keyword;
- `||` as one token, the logical or, so `|| x` did not parse;
- no named arguments and no trailing blocks;
- a formatter whose self-check compares trees, so sugar must survive
  parsing as its own nodes;
- and no pass that rewrites the tree: the type checker lowers it to HIR,
  and desugars string interpolation on the way.

## Decision

1. **`component` and `view` are contextual keywords.** `component` is one
   where an item starts, a name and `(` follow it, and a block (or a `->`,
   which is refused) follows its parameters; `view` where an expression
   starts and `{` follows it, outside an `if`, `while`, `for` or `match`
   head. Elsewhere both are names: no `.nova` file in the repository uses
   either as one, and `KEYWORDS` keeps its 32 words. Since line breaks are
   skipped, two programs read otherwise: a record named `view` can no
   longer be written as a literal, and a statement that is a variable
   named `component`, followed by a call and then a bare block, reads as
   a component.
2. **The sugar is kept in the tree and desugared by the type checker.**
   - A component is a `Function` with `is_component` set and no written
     return type; the checker gives it `std/ui`'s `View`. Every other
     consumer, the resolver, the index and `nova doc` among them, reads it
     as the function it is.
   - `Expr::View` holds `ViewNode`s, elements and expressions. The checker
     turns an element into a call of its builder with two arrays, or one
     for a void element, after checking the builder takes `([Attr],
     [View])` or `([Attr])` and returns `View`; so `input` needs no special
     case, and a program's own builder works.
   - A view holds one root, since `View` is one node (E0094); a void
     element takes no block (E0095); an element names a builder (E0096).
3. **Attributes by name.** Inside an element's argument list, `name: v`
   is the attribute `name` with each `_` made `-`; `value` is the property;
   `on_kind` is the handler of the DOM event `kind`, spelled as the DOM
   spells it. A handler is a `fn(String)`, given to `On` as it is, or a
   `fn()`, given to `std/ui/html`'s `on`; either returns `()`. Any other
   type is E0097. Named arguments exist nowhere else.
4. **Reactivity by literalness.** A string literal with no interpolation
   is fixed: `Static`, `Property` or `Text`. Any other value is wrapped in
   a closure: `Dynamic`, `DynProperty` or `DynText`, which `mount` runs in
   an effect, so the text or attribute alone follows the signals it reads.
   `text(e)` in a view is this syntax, not a call, and needs no import; it
   is shown on hover as `std/ui/html`'s `text`.
5. **`||` is a closure with no parameters wherever an expression starts.**
   It is the same node as `| |`, which `nova fmt` now prints as `||`; the
   repository's 128 lines that used `| |` were reformatted by `nova fmt`
   itself, but for four comment lines, edited by hand. A statement that
   begins with `||`, like one that begins with `|`, gets `;` before it, and
   so does a view child that would continue the one before it; the parser
   takes an optional `;` between children.
6. **The bundler's last pieces.** `nova build --target wasm` copies a
   project's own `index.html` from its root, or writes the generated page,
   which now has `<div id="app"></div>`; copies `public/` into `assets/`
   afresh, a link only when it leads to a file inside `public/`, and not
   at all when `public/` itself leads outside the project; and writes
   `manifest.json`, which lists every file it wrote with its size and
   nothing that changes between two builds of the same sources. It
   refuses an output directory that holds the project, however spelled, or
   lies inside `public/`, and removes a link at any name it writes before
   it writes, so no write reaches a file elsewhere. `--release` runs
   `wasm-opt -Oz` on `app.wasm` when `wasm-opt` is on `PATH`, says so in
   one line when it is not, and fails when it fails; nothing installs
   `wasm-opt`.

## Consequences

- **`06-counter-spa` is written as `60-EXAMPLES.md` §6 writes it**, but for
  `import ui`, `value()` and `mount("#app", Counter)`; its module and shim
  are 21 KB by `gzip -9`, against the gate's 50 KB, without `wasm-opt`.
- **A value that never changes but is not a literal**, such as `class:
  name`, costs an effect that runs once; the output is the same.
- **The language server colours `component` and `view` as keywords** from
  the parse, by a `keyword` semantic token, since the extension's grammar
  cannot tell `view {` in an `if` head from a view; its two rules colour
  the common case before the server answers.
- **`if` and `for` in a view, fragments, boolean properties such as
  `checked`, and named arguments outside `view`** are not in 4.4. Each can
  come later without changing what a view means today.

## References

- `docs/phase-4-plan.md` §3, decisions 6, 7 and 8, and §4's 4.4 entry.
- The spec and RFC named above.
- ADR 0028, the formatter; ADR 0042, `std/ui`.
- `nova-spec/10-LEXER.md`, `11-PARSER.md`, `30-FRONTEND.md` §3, §4 and §7,
  and `60-EXAMPLES.md` §6.
