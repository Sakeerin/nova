# Phase 4.4, "Component syntax": design

- **Date:** 2026-10-17.
- **Status:** approved by the user on 2026-10-17.
- **The user's decisions, 2026-10-17:**
  1. A signal is read with `count.value()`, as 4.3 built it; `count.value`
     stays a field access, which `Signal` has no field for.
  2. Inside `view { … }`, a named argument `name: value` is the attribute
     of that name, with no helper function needed: `_` becomes `-`,
     `value` is the property, and `on_kind` is the event `kind`. Keywords
     such as `type` and `for` can be names there.
  3. Inside `view { … }`, a text or attribute value that is not a literal
     becomes reactive by itself: it is wrapped in a closure, so it updates
     when a signal it reads changes. A literal stays fixed.
  4. `||` is a closure with no parameters everywhere, and `nova fmt` prints
     it in place of `| |`. The repository's `.nova` files are reformatted
     in the same change.
  5. `nova build --target wasm --release` runs `wasm-opt -Oz` on
     `app.wasm` when `wasm-opt` is on `PATH`, and says so in one line when
     it is not; it is never required. The gate's 50 KB is measured without
     it.
  6. `view { … }` has no `if` or `for` of its own: `show` and `for_each`,
     with a `|| view { … }` closure, do that.
  7. A project's own `index.html` sits at its root, beside `nova.toml`.
- **Branch:** `claude/loving-archimedes-frtf5q-44`, from `main` at
  `e6d94d7`, as 4.3 used `-43`.
- **Inputs:**
  - `docs/phase-4-plan.md` §3 (decisions 6, 7 and 8) and §4's 4.4 entry;
  - `nova-spec/30-FRONTEND.md` §3, §4 and §7, and `60-EXAMPLES.md` §6;
  - the 4.3 spec,
    `docs/superpowers/specs/2026-10-17-phase-4-3-std-ui-reactivity-and-views-design.md`;
  - ADR 0028 (the formatter), ADR 0042 (`std/ui`).

## 1. What 4.4 delivers

After 4.4, a program writes its views as `30-FRONTEND.md` §3 does:

```nova
import ui
import ui::html::{button, div}

component Counter() {
    let count = signal(0)

    view {
        div(class: "counter") {
            text("Count: ${count.value()}")
            button(on_click: || count.update(|n| n + 1)) { text("+") }
            button(on_click: || count.update(|n| n - 1)) { text("-") }
        }
    }
}

fn main() {
    mount("#app", Counter)
}
```

1. **RFC 0001** specifies the syntax (§3), and **ADR 0043** records it
   (§11):
   - `component Name(params) { … }`, a function that returns a `View`;
   - `view { … }`, a tree of elements, text and views;
   - named arguments and trailing blocks, inside `view` only;
   - `||`, a closure with no parameters, everywhere.
2. **The language** (§3 to §6): the lexer and parser, the type checker's
   desugaring, the formatter, the language server's semantic tokens, and
   the extension's grammar.
3. **The bundler's pieces 4.3 left** (§7): a project's own `index.html`,
   `assets/` copied from `public/`, `manifest.json`, and `wasm-opt` under
   `--release`.
4. **`examples/06-counter-spa`** (§8), as above, with its own `index.html`.

**The gate** (`docs/phase-4-plan.md` §4):
- `06-counter-spa` builds, and its counter responds to clicks in Chromium,
  in the `browser` job.
- Its `app.wasm` and `app.js` are under 50 KB gzipped, together.
- `nova fmt`'s three checks pass on every `.nova` file: formatting twice
  changes nothing, the output parses to the same tree, and every comment
  survives.

## 2. The starting point (read on 2026-10-17, at `e6d94d7`)

- **The lexer** (`crates/nova-lexer`) skips line breaks. `||` is one token,
  `PipePipe`; `| |` is two `Pipe`s. `KEYWORDS` lists 32 reserved words, and
  a test keeps it equal to the lexer's tokens. There are no contextual
  keywords. No `.nova` file uses `component` or `view` as a name.
- **The parser** (`crates/nova-parser/src/grammar.rs`) is recursive
  descent:
  - a closure starts at `Pipe` (`parse_closure`); `PipePipe` is only the
    logical or, so `|| x` is "expected expression";
  - a call's arguments are expressions; `name: value` in a call is an
    error, and `Ident {` is a record literal;
  - because line breaks are skipped, a statement that begins with `|`
    continues the line before; the formatter puts `;` before one
    (`CONTINUES`, ADR 0028 item 5).
- **No pass rewrites the tree.** The type checker lowers the tree to HIR,
  and desugars string interpolation there. The formatter's self-check
  compares trees, so sugar must survive parsing as its own nodes.
- **The type checker** gives an unannotated closure parameter a fresh
  variable, which unification fills: `count.update(|n| n + 1)` checks
  today (`tests/runtime/iterator.nova` has `map(|n| n + 1)`). A parameter
  used only as a receiver, `|s| s.len()`, is E0011.
- **`std/ui`** (4.3): `View` is `Element | Text | DynText | Region`, and
  `Attr` is `Static | Dynamic | Property | DynProperty | On`.
  `std/ui/html`'s builders take `([Attr], [View])`, and the void ones
  (`input`, `img`, `br`, `hr`) take `([Attr])`. `mount(selector, f)`
  takes the selector first.
- **`nova build --target wasm`** writes `index.html` (a fixed page with no
  `#app`), `app.js`, `app.wasm` and `package.json` into `dist/`, and
  leaves the rest of the directory alone. It refuses `--release`. A
  project's `dist/` is at its root. `wasm-opt` is not installed here or in
  CI.
- **The examples** are `01` to `05`; only `05-geometry` has a `nova.toml`.
  `index_complete.rs` analyses every `examples/*/src/main.nova`.

## 3. The syntax (RFC 0001)

### 3.1 `component`

```
item       += [ "pub" ] "component" IDENT "(" params ")" block
```

`component Name(params) { body }` is the function `fn Name(params) -> View
{ body }`. It takes doc comments and attributes as a function does. It has
no type parameters, no `async` and no written return type. `component` is
a keyword only where an item starts and the next two tokens are a name and
`(`; anywhere else it is an identifier.

### 3.2 `view { … }`

```
atom       += "view" "{" child* "}"
child       = element | postfix_expr
element     = IDENT "{" child* "}"
            | IDENT "(" view_args ")" [ "{" child* "}" ]
view_args   = view_arg { "," view_arg } [ "," ]
view_arg    = NAME ":" expr | expr
NAME        = IDENT | keyword
```

- `view` is a keyword only where an expression starts and the next token
  is `{`, and not in an `if`, `while`, `for` or `match` head, where `{`
  opens the body.
- A **child** is an element or a postfix expression: a name, a call, a
  method call, a field, a literal, a parenthesised expression, a block, or
  another `view { … }`. An expression with an operator is written in
  parentheses. Children are separated by nothing but space, as statements
  are.
- An **element** is a name followed by a trailing block, or by an argument
  list that holds at least one named argument, or both. A call with only
  positional arguments and no trailing block is an ordinary expression:
  `show(open, || view { … })` and `Counter()` are children of this kind.
- In an element's argument list, `name: value` is a named argument and a
  plain expression is a positional one. `NAME` may be a keyword (`type`,
  `for`). Named arguments exist only in an element's argument list.

### 3.3 `||`

```
atom       += "||" [ "->" type ] expr
```

`||` where an expression starts is a closure with no parameters, the same
closure as `| |`. Elsewhere it stays the logical or. Like `|`, a statement
that begins with `||` continues the line before, so the formatter puts `;`
before one.

## 4. The desugaring

The parser keeps `component` and `view` as their own nodes; the type
checker desugars them into the calls 4.3's API takes, as it desugars
string interpolation.

### 4.1 `component`

The node becomes a `fn` item whose return type is `std/ui`'s `View`. A
component is called, passed and mounted as a function:
`mount("#app", Counter)`.

### 4.2 `view { … }`

- `view { c }` is the `View` that its one child desugars to. No child, or
  more than one, is **E0094**, "`view` holds one root, found N".
- A child that is a postfix expression must have type `View`.

### 4.3 Elements

An element `name(args) { children }` desugars to a call of the function
`name`:

| The function `name` takes | The element desugars to |
|---|---|
| `([Attr], [View])` | `name([attrs…], [children…])` |
| `([Attr])` | `name([attrs…])`; a trailing block is **E0095**, "`name` takes no children" |
| anything else | **E0096**, "`name` is not an element builder" |

The attributes are the arguments, in order. A positional argument is an
`Attr` expression, used as it is. A named argument becomes an `Attr`:

| Named argument | Value | Desugars to |
|---|---|---|
| `on_kind: h` | `h: fn()` | `On("kind", \|v: String\| h())` |
| `on_kind: h` | `h: fn(String)` | `On("kind", h)` |
| `value: v` | a string literal | `Property("value", v)` |
| `value: v` | any other `String` | `DynProperty("value", \|\| v)` |
| `name: v` | a string literal | `Static("name", v)` |
| `name: v` | any other `String` | `Dynamic("name", \|\| v)` |

- The attribute's name is `name` with each `_` replaced by `-`
  (`aria_label` is `aria-label`). After `on_` comes the event's name as
  the DOM spells it, unchanged: `on_keydown` is the event `keydown`.
- A value that is not a `String`, or a handler of another type, is
  **E0097**, naming the attribute and the type found.
- A string literal means one with no interpolation. `class: "a"` is fixed;
  `class: "a ${b}"` and `class: name` are reactive.

### 4.4 Text and reactivity

- `text(e)` as a child, with the bare name `text` and one positional
  argument, is a text node: `Text(e)` when `e` is a string literal, and
  `DynText(|| e)` otherwise. It is part of `view`, not a call of
  `std/ui/html`'s `text`, so it needs no import.
- A reactive value's closure runs inside the effect that `mount` makes
  for it (4.3), so a signal the value reads updates that text or
  attribute alone. Under `render_to_string` it is read once.
- The closures capture by value, as every Nova closure does; a `Signal` is
  a record, so the closure reads the same signal.

### 4.5 `||`

`||` is the closure `| |`: the parser builds the same node.

## 5. Formatting

`nova fmt` prints the new syntax in one fixed layout, under its three
checks (ADR 0028):

- **`||`** for a closure with no parameters. Every `| |` in the repository
  becomes `||`, by `nova fmt` itself, in the change that teaches it.
- **`component Name(params) {`**, laid out as a `fn` without a return
  type.
- **`view {`**, then each child on its own line, indented, then `}`.
- **An element** prints `name(args)` when it has arguments, then ` { `
  and its children: on one line when it has one child and the whole fits
  within the width, and otherwise one child per line. Named arguments print
  `name: value`, and break one per line when the list is too long, as call
  arguments do.
- **A statement that begins with `||`** gets `;` before it, as one that
  begins with `|` does.

## 6. Editor support

- **The extension's grammar** colours `component` as a keyword where a
  name and `(` follow it, and `view` where `{` follows it. `KEYWORDS`, and
  the test that keeps the grammar's keyword list equal to it, do not
  change: neither word is reserved.
- **The language server's semantic tokens** gain a `keyword` type, used
  for `component` and `view` where they are keywords, since the grammar
  cannot tell `view {` in an `if` head from a view. Other keywords stay
  with the grammar.
- **The index:** an element's name is an occurrence of its builder, so
  hover, go-to-definition and rename work on it. A named argument's name
  is no symbol, and neither is `component` or `view` where it is a
  keyword; `index_complete.rs` skips all three, as it skips an
  attribute's name (`@test`).

## 7. The bundler

`nova build --target wasm` (and `nova bundle`) writes into `dist/`:

| File | From |
|---|---|
| `index.html` | the project's own `index.html` beside `nova.toml`, copied as it is; otherwise the generated page, which gains `<div id="app"></div>` |
| `app.js`, `app.wasm` | the compiler, as in 4.0 to 4.3 |
| `package.json` | as in 4.0 |
| `assets/` | the project's `public/` directory, copied; replaced on each build |
| `manifest.json` | the build's metadata (below) |

- **A project's own page** must load `app.js` and call its `init()`, as
  the generated one does; `06-counter-spa`'s shows how. A program built
  from a single file, outside a project, keeps the generated page and has
  no `assets/`.
- **`public/`** is copied file by file, directories kept; a symbolic link
  to a directory is skipped, so a copy cannot loop or leave the project.
- **`manifest.json`** names the build and lists its files, sorted, with
  their sizes in bytes, and nothing that changes between two builds of
  the same sources:

  ```json
  {
    "name": "counter-spa",
    "version": "0.1.0",
    "release": false,
    "wasm_opt": false,
    "files": {
      "app.js": 12345,
      "app.wasm": 67890,
      "index.html": 211
    }
  }
  ```

  `name` and `version` come from `nova.toml`, or are the file's stem and
  `null` outside a project. `files` lists `assets/…` too.
- **`--release`** is accepted with `--target wasm`. It changes nothing in
  the compiler; it runs `wasm-opt -Oz app.wasm -o app.wasm` when
  `wasm-opt` is on `PATH`, and prints `wasm-opt not found on PATH;
  app.wasm is not optimized` to stderr when it is not. A failing
  `wasm-opt` fails the build. Without `--release`, `wasm-opt` never runs.

## 8. `examples/06-counter-spa`

- `nova.toml` (`name = "counter-spa"`), `index.html` with `<div
  id="app"></div>` and a module script that imports `init` from
  `./app.js` and calls it, and `src/main.nova` as §1 writes it.
- It differs from `60-EXAMPLES.md` §6 in three places, each the API 4.3
  built: `import ui` and `import ui::html::{button, div}`, `value()`, and
  `mount("#app", Counter)`. A note in `60-EXAMPLES.md` says so.
- A README in the template of `60-EXAMPLES.md` §9: what it shows, how to
  build it and serve `dist/`, and what a click does.

## 9. Testing

### 9.1 Unchanged rules

- Tests never touch the internet. A test that needs `NOVA_HOME` sets it to
  a fresh directory, and temporary directories have fixed names.
- A test that needs Node passes without it, and fails under
  `NOVA_REQUIRE_NODE=1`, which CI sets.
- No new crate, and `Cargo.lock` does not change.

### 9.2 The language

- **Lexer and parser** unit tests: `component` and `view` as keywords and
  as names (`let view = 1`, `fn component()`, `if view { }`); `||` as a
  closure and as an or; named arguments and trailing blocks; a named
  argument outside `view` is still an error.
- **`tests/runtime/ui_view.nova`**, on both targets through
  `render_to_string`: a component with parameters; nested elements, a void
  element, `type:`, `for:` and `aria_label:`; fixed and reactive text and
  attributes (a reactive one reads a signal set before the render);
  `value:`; an `on_click` that the render leaves out; `show` and
  `for_each` with `|| view { … }`; and `||` outside `view`.
- **Errors**, each a program that fails with its code: E0094 to E0097,
  and a component with a return type or type parameters (a parse error).
- **The formatter**: each new form formats to itself; `| |` formats to
  `||`; a statement that begins with `||` gets its `;`; the corpus test
  covers every `.nova` file.
- **The language server**: `component` and `view` get the `keyword`
  token, and an element's name hovers as its builder.
- **`index_complete.rs`** counts `06-counter-spa`'s program, and skips
  named arguments' names and the two contextual keywords.

### 9.3 The bundler

In `wasm_tests.rs`:
- a project's own `index.html` is copied; without one, the generated page
  has `#app`;
- `public/` becomes `assets/`, nested files included, and a file removed
  from `public/` is gone from `assets/` after the next build;
- `manifest.json` lists every file with its size, and is the same after a
  second build;
- `--release` without `wasm-opt` on `PATH` builds and prints its line; with
  a stand-in `wasm-opt` first on `PATH`, built by the test as a native
  Nova program that copies its input to its output and records its
  arguments, `--release` runs it with `-Oz`, and `manifest.json` says
  `"wasm_opt": true`.

### 9.4 `06-counter-spa`

- **Under the fake DOM**, in `cargo test`: the example builds, and a
  harness clicks `+` three times and `-` once, and finds `Count: 2`, as
  is and under `NOVA_GC_STRESS`.
- **The size:** the test gzips `app.wasm` and `app.js` with Node's `zlib`
  at level 9 and checks the sum is under 51,200 bytes.
- **In Chromium**, in the `browser` job: the job builds the example and a
  script clicks it the same way. One script, `tests/browser/spa.mjs`, finds
  the buttons by their text.

### 9.5 Checks

`cargo fmt --all -- --check`, clippy with `-D warnings`, `nova fmt
--check std examples tests/wasm tests/dom` and the corpus test, Rust 1.78,
and CI on all three systems.

## 10. Crates and files

| Crate or path | Change |
|---|---|
| `nova-lexer` | none: `component` and `view` stay identifiers |
| `nova-parser` | `component` items, `view` blocks, elements, named arguments, `||` closures |
| `nova-ast` | `Item::Component`, `Expr::View`, the element and argument nodes |
| `nova-typeck` | the desugaring (§4); E0094 to E0097 |
| `nova-fmt` | §5; `CONTINUES` gains `||` |
| `nova-lsp` | the `keyword` semantic token |
| `nova-driver`, `nova-cli` | §7: the page, `assets/`, `manifest.json`, `--release` and `wasm-opt` |
| `nova-codegen-wasm` | the generated page gains `#app` |
| `nova-doc` | a component is documented as a function |
| `tools/vscode-nova` | the grammar's two rules |
| `examples/06-counter-spa` | new |
| `tests/` | §9 |
| `.github/workflows/ci.yml` | the `browser` job builds and clicks `06-counter-spa` |
| every `.nova` file with `\| \|` | reformatted to `\|\|` |

## 11. Records

- **RFC 0001**, `docs/rfcs/0001-component-syntax.md`: the syntax and its
  desugaring (§3, §4), the alternatives the user weighed, and what is left
  out (§13).
- **ADR 0043**, `docs/adr/0043-component-syntax.md`: contextual keywords,
  sugar kept in the tree and desugared by the type checker, attributes by
  name, reactivity by literalness, and `--release` with an optional
  `wasm-opt`.
- **Dated notes:** `nova-spec/10-LEXER.md` (contextual keywords),
  `11-PARSER.md` (the grammar), `30-FRONTEND.md` §3, §4 and §7,
  `60-EXAMPLES.md` §6, the phase plan's 4.4 entry, the CHANGELOG, and the
  formatter spec's sentence that no statement begins with `||`.

## 12. Risks

- **A child that runs into the next.** Line breaks are skipped, so a
  child followed by one that begins with `(` or `[` could read as a call
  or an index. Children are postfix expressions, and an element's trailing
  block is taken only right after its name or arguments; the parser tests
  cover `text("a")` followed by `(text("b"))`.
- **Reactivity by literalness** wraps values that never change, such as
  `class: name` for a fixed `name`, in an effect that runs once. It costs
  an effect per such attribute, and no wrong output.
- **Type-directed elements:** an element is checked against its builder's
  parameters, so a user's own builder with the same two parameters works,
  and one with others is E0096 rather than a confusing mismatch.
- **Reformatting `| |` to `||`** touches 134 places in 17 files. The
  formatter's tree check proves each change is the same closure.

## 13. Not in 4.4

- `if` and `for` inside `view` (the user's decision 6).
- Named arguments and trailing blocks outside `view`.
- Boolean properties such as `checked`: `std/ui`'s `Attr` holds strings
  only.
- A view with several roots, or fragments.
- `nova dev` (4.5), code splitting, and a hashed asset pipeline.
- Installing `wasm-opt` anywhere.

## 14. Decisions made while writing this spec

1. **Sugar is kept in the tree and desugared by the type checker,** as
   string interpolation is, so `nova fmt` can print it back and its tree
   check holds.
2. **An element is checked against its builder's parameters,** `([Attr],
   [View])` or `([Attr])`, so `input` needs no special case and a user's
   own builder works.
3. **`text(e)` in a view is syntax,** not a call of `std/ui/html`'s
   `text`, so it can become reactive and needs no import.
4. **`view` holds exactly one root,** since `View` is one node and `std/ui`
   has no fragment.
5. **Only `value` is a property.** The user's choice named `checked` too,
   but `std/ui` sets properties as strings, and the string `"false"` would
   check a box; `checked` waits for boolean properties (§13).
6. **The generated page gains `<div id="app"></div>`,** so a program that
   mounts `#app` works without a page of its own.
7. **`manifest.json` holds no time or hash,** so two builds of the same
   sources write the same file.
8. **`--release` is accepted for WASM** and only governs `wasm-opt`;
   without `wasm-opt` it builds what a plain build does.
9. **The branch is `claude/loving-archimedes-frtf5q-44`,** as 4.3's was
   `-43`, since the earlier name's history cannot be rewritten here.
