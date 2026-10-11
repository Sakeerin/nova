# Phase 3.5a, "The documentation site": design

> Status: **draft for review** (2026-10-11). The first half of Phase 3.5,
> "`nova doc`" (`docs/phase-3-plan.md` §4, the 3.5 entry;
> `nova-spec/40-TOOLING.md` §5). Phase 3.4 is complete: PRs #107 and #108,
> ADRs 0032 and 0033. On 2026-10-11 the user made four decisions (§20,
> decisions 1 to 4):
> - split 3.5 in two, as 3.3 and 3.4 were: 3.5a is the site, and 3.5b is
>   `nova test --doc`;
> - use `pulldown-cmark`;
> - write docs for every public std item;
> - fix PR #108's nine deferred minors first, on this branch.
>
> Branch `claude/loving-archimedes-frtf5q`.

`nova doc` writes a static site under `target/doc/`. It has a page per
module, and each documented item on it shows:
- its signature as written, with every name in it linked to where that
  name is documented;
- its `///` docs, rendered as Markdown.

Every page has a search box and a light and a dark theme. Every site
carries std's pages, and std gets docs for every public item. Before any
of that, this branch fixes the nine minors that PR #108 deferred.

## 1. What 3.5a delivers

- **The 3.4b follow-ups** (§3).
- **`nova doc [--open] [--std]`** (§4):
  - what a site documents (§5);
  - its files and anchors (§6) and its pages (§7);
  - signatures (§8) and docs (§9);
  - search and themes (§10);
  - the link check (§11).
- **std's docs** (§12): a summary for each module, and docs for each
  public item and member.
- **An example that uses a dependency**, `examples/05-geometry` (§13).
- **std's site on each GitHub release** (§14).
- **The phase plan's 3.5 gate, for the site** (§17).

## 2. The starting point (read on 2026-10-11)

### `nova-doc` and the command line

- `crates/nova-doc` is a stub. Its `lib.rs` is one line, and its manifest
  depends on `nova-diagnostics`, `nova-ast`, `nova-parser` and `anyhow`.
- The CLI has no `doc` command (`crates/nova-cli/src/main.rs`, `Command`).
- `pulldown-cmark` is not in `Cargo.lock`. `bitflags` 2.13.1 and `memchr`
  2.8.3, which it needs, are.

### Docs in the source

- `///` documents the thing it precedes: an item, a trait or impl member,
  a record field, a variant or an extern function. It lands in that
  node's `docs: Vec<Spanned<String>>`, one entry per line, each holding
  the text after `///` (`crates/nova-ast/src/item.rs`;
  `crates/nova-lexer/src/lib.rs:603`). Anywhere else it is a parse error.
- `//!` is an ordinary comment. `skip_trivia` treats exactly three
  slashes as a doc comment and anything else as trivia
  (`lib.rs:156-172`). `lex_with_comments` returns every comment with its
  span (`lib.rs:57`), and block comments do not nest.
- Docs on variants written one per line (`| Circle(Float)`), and on
  record fields, pass `nova fmt --check` and `nova check` as written
  (probed on 2026-10-11).
- Hover reads docs from the text: the `///` lines directly above a
  declaration (`crates/nova-lsp/src/hover.rs:182`). It builds its
  declaration line from the text too (`:39`).
- No `.nova` file in the repository has a `///` line, std included.
  - std's 17 modules hold 6,141 lines, about 3,500 of them `//` comment
    lines.
  - Those comments are written for maintainers: measurements, ADR
    references and runtime function names.
  - About 220 `pub` items and methods sit at the top level and in impls:
    136 `pub fn`, 27 `pub async fn`, 34 `pub record`, 11 `pub type` and
    12 `pub trait`. Their fields, variants and trait methods come on top.
    No `pub const` exists.

### What an analysis holds

- `nova_driver::analyze_program` returns an `Analysis`
  (`crates/nova-driver/src/analyze.rs:71`). It holds:
  - the `FileDb`;
  - the modules in load order, with their packages;
  - the graph, `Definitions` and the typed HIR module;
  - the index, with `Options::index`.

  It does not keep the merged AST the resolver built (`resolved.file`,
  `:172`), which holds the items' `docs`, their visibility and the impl
  blocks.
- `Definitions` maps each merged item to its module (`module_of`,
  `crates/nova-resolver/src/lib.rs:1266`). The user modules come first,
  then std's in `STD_MODULES` order (`:1876`), then `std/test` with
  `Options::tests` (`STD_TEST_MODULE`, `:1895`). A std module's file is
  named `<std/core>`, `<std/collections>` and so on
  (`analyze.rs:148-159`).
- `Definitions::exported(module, name)` gives what a module exports
  under a name, in each namespace (`lib.rs:1422`). It is the resolver's
  own rule for what an import may name.
- A `Def` has a name, its name's span and a kind (`lib.rs:1149-1205`).
  Visibility lives only in the AST.
- The index records every name the front end resolved, with its target
  (`crates/nova-resolver/src/index.rs:22`).
  `crates/nova-driver/tests/index_complete.rs` checks std, `examples/` and
  `tests/runtime/`: every identifier lies inside an occurrence. So every
  name in a std signature has a target.
- A parameter always carries a type (`Param.ty` is not optional), and so
  does a `const`. Only a return type may be left out.
- std has inherent impls on primitive types:
  - `impl Int`, `impl Float` and `impl String` in `std/fmt`;
  - `impl String` in `std/strings`;
  - `impl Bytes` in `std/bytes`.

  Its trait impls for them are in `std/core` and `std/json`.
- Five builtin functions are in every module's scope: `println`, `print`,
  `eprint`, `eprintln` and `panic`. `nova_typeck::builtin_text` gives
  each a signature (`crates/nova-typeck/src/lib.rs:162`). The other
  builtins (`Builtin::STD_ONLY`) are std's own.

### Projects and examples

- `run`, `build`, `check` and `test` find the project by walking up to a
  `nova.toml`, and sync it first (`crates/nova-cli/src/project.rs`). A
  project with only path dependencies gets no `nova.lock`
  (`crates/nova-index/src/sync.rs:86`).
- `Program`'s roots are a public list
  (`crates/nova-driver/src/program.rs:46`). `Roots::Check` is
  `src/main.nova` then `src/lib.nova`.
- A package may be named `std`. `check_name` allows ASCII letters, digits,
  `-` and `_` (`crates/nova-pm/src/name.rs:15`).
- No example has a `nova.toml`, so none uses a dependency.
  - `nova-spec/60-EXAMPLES.md` reserves `06-` to `15-`.
  - `03-producer-consumer` already shares its number with
    `03-http-server`.
  - The index-completeness check analyses every
    `examples/*/src/main.nova` (`index_complete.rs:22`).
- CI runs `nova fmt --check std examples` on three systems
  (`.github/workflows/ci.yml:45`). Its `install` job runs gate scripts
  against an installed `nova` (`:198`).
- `release.yml`'s prepare job expects four archives and one `.vsix`
  (`:138-141`).

### The deferred minors' sites

1. Organize imports: `crates/nova-fmt/src/organize.rs:71`, and its
   `comments_above`.
2. The importable loop: `crates/nova-driver/src/program.rs:415-447`, one
   `loader.find` per module and name.
3. The code-action and semantic-token handlers,
   `crates/nova-lsp/src/lib.rs:380-421`, run outside `analysis::guarded`
   (`analysis.rs:104`).
4. `crates/nova-driver/tests/broken.rs`'s planted program carries only
   single-file fixes.
5. `lsp_navigation.rs:984` and `lsp_fixes.rs:484` both name a test
   directory `gate`.
6. `docs/phase-3-plan.md`'s 3.4b note. PR #108's CI passed every job,
   the VS Code smoke test included.
7. `40-TOOLING.md` §3.1's 3.4b note.
8. `suggest::closest` (`crates/nova-diagnostics/src/suggest.rs:42`).
9. `lines::line_after` and `lines::line_before_item`
   (`crates/nova-diagnostics/src/lines.rs:48`, `:61`), which know only
   `//` lines.

## 3. The 3.4b follow-ups

Each of PR #108's nine deferred minors is fixed test-first, in a commit
of its own, before `nova doc`:

| # | Fix | Test |
|---|---|---|
| 1 | Organize imports leaves the comment lines that start the file where they are, above the block. They are the file's header, not the first import's lead, so removing or moving that import no longer takes them. | an unused first import under a header comment; a first import that sorts below another |
| 2 | The loader remembers each `find` it answered, by the directory it searched from and the name. The importable loop then probes each missing sibling file once per directory, not once per module. | a counting `Sources`: the reads are the loaded files plus one probe per directory and name |
| 3 | Code actions and semantic tokens run inside `analysis::guarded`, as the other requests do. A panic is logged and answered with an empty result. | a handler made to panic is answered, and the server keeps serving |
| 4 | The cut-program sweep also plants a two-file program, carrying an import fix and a make-public fix, swept whole and at cuts. | the sweep itself, with its vacuity check |
| 5 | The two gate tests get their own directories: `navigation-gate` and `fixes-gate`. | (none needed) |
| 6 | The phase plan's 3.4b note says the gate was met in PR #108's CI, on three systems and in the smoke test. | (none needed) |
| 7 | 40-TOOLING §3.1's note reads "With 3.2 and 3.4a, every Phase 3 row of this table is delivered". | (none needed) |
| 8 | `suggest::closest` skips a candidate whose length, in characters, differs from the name's by more than the bound. Its distance is at least that difference, so it could not qualify. | the existing cases, and a long candidate skipped |
| 9 | The import fix's line rules step over block comments as they step over `//` lines. If the last import's line ends inside a `/* … */` that goes on, the new line goes after the comment's last line. A block comment directly above the first item is walked over. | both cases, on `\n` and on `\r\n` |

## 4. `nova doc`

### 4.1 The command

`nova doc [--open] [--std]`

- **In a project:** it finds the project as `nova check` does, syncs it,
  analyses it (§5.1), and writes the site to the project's `target/doc/`.
- **Outside any project,** it stops: "nova doc documents a project; run
  it inside one, or use `nova doc --std`". A loose file has no name to
  file its pages under.
- **`--std`** writes std's site alone, and analyses and syncs no
  project. It writes to the project's `target/doc/` inside a project,
  and otherwise to `target/doc/` under the current directory.
- **`--open`** then opens the site's start page: the root package's page,
  or std's with `--std`.
  - It runs `$BROWSER` with the page's path when that is set, and
    otherwise the system's opener: `xdg-open`, `open` or `cmd /C start`.
  - Failing to open is a warning that names the page. The exit status
    stays 0.
- On success it prints the start page's path. It exits 0, or 1 on an
  error.

### 4.2 What stops it

- **A program with errors is not documented.** `nova doc` prints the
  diagnostics as `nova check` does, and exits 1. Warnings are printed and
  do not stop it.
- **Two packages that would share a directory.** A documented package
  named `std`, or two whose names are equal ignoring ASCII case: it stops
  and says why.

### 4.3 Replacing `target/doc/`

- Every site holds a marker file, `target/doc/.nova-doc`. Before
  writing, `nova doc` removes a `target/doc/` that holds the marker, so
  no page of a removed module is left behind. [Amended 2026-10-11, final
  review: it removes everything in it but the marker, and writes the
  marker before any page, so a write cut short still leaves a directory
  the next `nova doc` replaces.]
- It does not touch a non-empty `target/doc/` without the marker, such
  as `cargo doc`'s in a Rust checkout. It stops and says to remove that
  directory.

## 5. What a site documents

### 5.1 The packages

- **The root package,** from one analysis of one root: `src/lib.nova`
  when the package has a library, otherwise `src/main.nova`. The modules
  documented are the package's modules that root reaches.
  - A library documents what each module exports: the items
    `Definitions::exported` says another module may import.
  - A package that is only a program documents every item, exported or
    not, since nothing imports a program. `cargo doc` treats a binary
    the same way.
- **Its dependencies:** every package that analysis loads, which means
  every dependency the root's modules import, directly or through
  another package. Each documents the exports of the modules its library
  reaches.
  - A declared dependency that nothing imports is not loaded, so it gets
    no pages.
  - Dev-dependencies are never loaded, since `tests/` is not a root.
- **std**, in every site: its 17 modules, `std/test` included, from an
  analysis of std alone (§5.3).

### 5.2 Items and members

- **Items:** functions, extern functions, records, sum types, traits and
  constants. The checker refuses type aliases today (ADR 0032), so none
  reaches a site.
- **Members:**
  - a record's `pub` fields;
  - a sum type's variants;
  - a trait's methods, required and provided, and its associated types;
  - an inherent impl's `pub` methods, or all its methods in a program;
  - for every impl, its header and its docs.
- **A trait impl shows its header and docs only.** Its methods are
  documented on the trait (decision 28).
- **Where an impl appears:**
  - with its type, when the type is documented in the same package;
  - on std's primitive pages, when it is std's impl on a primitive
    (§7.4);
  - otherwise on its own module's page, under "Implementations on other
    types". That covers an impl on another package's type, and a user's
    impl on a primitive.
- **A trait also lists its impls in the same package.**
- **`@test` functions are never documented.** The driver strips them
  when it is not testing.

### 5.3 std's pages

- One analysis of std alone, with `std/test` in scope, serves both
  `--std` and every project's site. So one `nova` writes the same std
  pages everywhere.
- A project's pages link into std by module and anchor (§6.3), so the two
  analyses need not share ids.

## 6. The site's files

### 6.1 Layout

```
target/doc/
├── .nova-doc                        the marker (§4.3)
├── index.html                       the front page: the site's packages
├── nova.css                         the themes
├── nova.js                          search and the theme toggle
├── search-index.js                  every documented item and member
├── <package>/index.html             the package's root module, lib or main
├── <package>/<module>/index.html    each other module
├── std/index.html                   modules, primitive types, builtins
├── std/<module>/index.html          core, collections, … test
└── std/primitive.<Type>.html        Int, Float, Bool, Char, String, Bytes, Future
```

- **Names need no escaping.** A module's directory is its file's stem,
  and a package's is its name. Both are ASCII: the lexer's identifiers,
  and `check_name`'s package names. A module directory's name has no
  `.`, so `primitive.<Type>.html` cannot meet one.
- **Relative URLs only.** Pages link to each other, to the stylesheet and
  to the scripts by relative URLs. The site works from disk (`file://`)
  and from any static host.
- **The same bytes everywhere.** Every file is written with `\n` line
  endings, and one program gives the same bytes on every system: no
  times, and every list in a fixed order.

### 6.2 Anchors

Each documented item and member has an `id` on its page:

| What | `id` |
|---|---|
| a function or extern function | `fn.<name>` |
| a record | `record.<name>` |
| a sum type | `type.<name>` |
| a trait | `trait.<name>` |
| a constant | `const.<name>` |
| a record's field | `record.<R>.field.<f>` |
| a variant | `type.<T>.variant.<V>` |
| a trait's method, or its associated type | `trait.<T>.method.<m>`, `trait.<T>.type.<A>` |
| an inherent method, with its type | `record.<T>.method.<m>` or `type.<T>.method.<m>` |
| a method on a primitive's page | `method.<m>` |
| a method under "Implementations on other types" | `impl.<T>.method.<m>` |

- `<T>` is the type's name without its arguments.
- Where two `id`s on one page would be equal, the later ones get `-2`,
  `-3` and so on, in source order. One example: a method on each of
  `Vec<Int>` and `Vec<String>` under "Implementations on other types".

### 6.3 Links between pages

- **A name in a signature (§8) links to its target's page and anchor,**
  when the target is documented in the site. Otherwise it is plain text:
  a private item, a local, a type parameter, a builtin.
- **A target in std, seen from a project's analysis,** is found by its
  module and its anchor. The module comes from its file's name
  (`<std/core>`), and the anchor follows §6.2. Neither depends on which
  analysis saw it.
- **A primitive type links to its page.**

## 7. Pages

Every page has the same header (the site's name, the search box and the
theme toggle), a sidebar, and a footer naming the `nova` that wrote it.

### 7.1 The front page

`target/doc/index.html` lists the root package, then its dependencies by
name, then std. Each shows its version and its manifest's `description`.

### 7.2 A package's page

- Its name, version, `description`, and its root module's docs (§9.2).
- Its other modules, each with the first paragraph of its docs.
- Then the root module's items, as a module page shows them (§7.3).

### 7.3 A module page

- **The module's name and its docs.**
- **Sections,** in this order, each holding its items in source order:
  Records, Sum types, Traits, Functions, Constants, Implementations on
  other types.
- **Each item** shows its signature (§8) and its docs (§9), then its
  members:
  - a record: its documented fields, each with its declaration and docs.
    If it has fields that are not documented, a line says so.
  - a sum type: every variant, with its payload and docs.
  - a trait: its associated types, then its methods, each with its
    signature and docs, then its impls in the same package.
  - a record or sum type: its impls in the same package. Each shows its
    header and docs, and an inherent impl also shows its documented
    methods.
- **The sidebar** lists the package's modules and the page's items.

### 7.4 std's front page and the primitive pages

- **std's front page** lists:
  - its modules, each with its summary;
  - the primitive types;
  - the five builtin functions, with their signatures from
    `builtin_text` and docs held in `nova-doc`.
- **A primitive's page** holds std's impls on that type: inherent
  methods with their docs, and trait impls by header. A project's own
  impl on a primitive goes on its module's page (§5.2).

## 8. Signatures

The phase plan asks for "item signatures from the type checker". Nova's
signatures are written in full: every parameter and constant has a type.
So a signature is shown **as written**, and the checker supplies what
each name in it means, from the index (decision 5).

- **The text** runs from the declaration's first token after its docs
  and attributes, through its last token before:
  - a function's or method's body;
  - a record's `{`, a sum type's `=`, a trait's `{` or an impl's `{`.

  A required trait method or an extern function runs to its own end. A
  constant runs to its end, so its value is shown. A field shows
  `name: Type`, and a variant `Name(Payload)`.
- **The layout is kept.** Comments are removed, each line loses the
  indentation that all its lines share, and trailing spaces go. Code in
  `nova fmt`'s layout keeps that layout, a long signature broken over
  lines included.
- **Names are links** (§6.3). The item's own name is not.
- **Colour.** Keywords, literals and the item's name are marked for the
  theme, by the lexer the highlighter uses (§9.4).

## 9. Docs

### 9.1 Markdown

- **An item's docs** are its `///` lines, joined, each without one
  leading space. `pulldown-cmark` renders them, with tables and
  strikethrough on.
- **Raw HTML is shown as text,** escaped. A dependency's docs cannot put
  markup or a script into the site (decision 10).
- **Headings move down.** A doc's headings start one level below the
  heading of what it documents, and stop at `<h6>` (decision 11).
- **Links are kept as written.** The link check reports a relative link
  the site lacks (§11). `[Name]` links to items are not resolved (§21).
  [Amended 2026-10-11, final review: a link or an image whose URL has a
  scheme other than `http`, `https` or `mailto`, as `javascript:`, is
  dropped and its text kept as plain text, so decision 10 holds for
  links too.]
- **The summary** of an item or a module is its first paragraph, as
  plain text. Lists and search results show it.

### 9.2 Module docs

- A module's docs are its `//!` lines before the file's first token.
  Each line loses `//!` and one leading space, and they render as an
  item's docs do (§9.1).
- **`//!` stays a comment to the compiler and the formatter.** Only
  `nova doc` reads it, through `lex_with_comments` (decision 9). A `//!`
  line after the first token is an ordinary comment.

### 9.3 Code blocks

- These blocks are Nova, and are highlighted (§9.4):
  - a fenced block with no language, or with `nova`;
  - an indented block.

  A block in any other language is shown as written.
- 3.5a never runs a block. `nova test --doc` is 3.5b's.

### 9.4 Highlighting

- Nova code is coloured from `nova_lexer::lex_with_comments`:
  - keywords;
  - string, character and number literals;
  - comments and doc comments.
- Text the lexer rejects, such as an unterminated string, is shown
  plain.

## 10. Search and themes

### 10.1 Search

- **The index:** `search-index.js` sets one array. It holds an entry for
  every documented item and member, sorted by path. Each entry has:
  - its name and its kind;
  - its path, `package::module::Item::member`;
  - its URL from the site's root;
  - its summary.
- **The box:** each page's search box filters the index as the user
  types, case-insensitively.
  - Exact names come first, then names that start with the query, then
    names that contain it.
  - At most 50 results show, each linking to its anchor.
  - `/` focuses the box, and Escape clears it.
- **No library, no fetch.** The script is plain JavaScript. The index is
  loaded as a script, not fetched, so search works from `file://`.

### 10.2 Themes

- **Two palettes.** `nova.css` defines a light and a dark one.
- **The choice.** A page follows the system's `prefers-color-scheme`
  until the toggle picks one, which `localStorage` keeps. When storage is
  unavailable, the toggle still works for that page.
- **Nothing from the network.** No font, script or style is fetched.

## 11. The link check

- After writing, `nova doc` reads every page it wrote and checks each
  `href` and `src` that is not absolute (a scheme, or `//`):
  - the file must exist in the site;
  - a `#fragment` must be an `id` on that page.
- **A broken link is a warning,** naming the page and the link. It is
  either a `nova doc` bug or a relative link written in a doc. The exit
  status stays 0.
- **The check is public:** `nova_doc::check_links(dir)`. The gate tests
  (§16.4) call it directly, and CI's script (§17) reads `nova doc`'s
  warnings.

## 12. std's docs

### 12.1 What is written

- **Each std module gets `//!` docs:** a summary sentence, and a
  paragraph on what the module holds.
- **Each documented item and member of std gets `///` docs** (§5.2).
  - A summary sentence comes first.
  - Then, where it matters: what it returns on failure, when it panics,
    and how it treats edge cases such as saturation.
- **They are written for a user of the module.**
  - No ADRs, runtime function names or measurements.
  - Inline code goes in backticks.
  - No code blocks yet: examples arrive with 3.5b, which tests them
    (decision 24).
- **The maintainer `//` comments stay.** Where one sits above an item,
  it stays above the item's `///` lines. The `///` lines go directly
  above the item, where hover looks for them (§2).
- **Every std file stays in `nova fmt`'s layout,** which CI checks.

### 12.2 Keeping it complete

`crates/nova-doc/tests/std_docs.rs` parses each std file. It fails, and
names each case, when:
- a module has no `//!` docs;
- any of these has no `///` docs:
  - a `pub` item;
  - a `pub` method of an inherent impl;
  - a trait's method or associated type;
  - a variant of a `pub` sum type;
  - a `pub` field of a `pub` record.

So std cannot gain an undocumented public item.

## 13. The example: `examples/05-geometry`

- **An app, `geometry`, at the example's root.** It has a `nova.toml`, a
  `src/main.nova`, and a `README.md` in `60-EXAMPLES.md` §9's template.
  It depends on a library beside it: `shapes = { path = "shapes" }`.
- **The library, `shapes/`, has two modules:**
  - `src/lib.nova` holds the API: a record, a sum type, a trait
    implemented for both, functions and a constant. It carries `///`
    docs and `//!` module docs.
  - `src/units.nova` holds a type the API's signatures use, so the site
    links between two modules of one package.
- **The app prints a fixed output,** and a test in `run_tests.rs` runs
  it.
- **Its name.** `60-EXAMPLES.md` reserves `06-` to `15-`. `05-geometry`
  shares `05` with `05-json-api`, sorting just before it and before them,
  as `03-producer-consumer` shares `03` (decision 23).
- **It joins** `nova fmt --check examples` in CI, and the
  index-completeness check (§2).

## 14. std's site on each release

- **A new job in `release.yml`** builds `nova`, runs `nova doc --std` in
  an empty directory, and packs its `target/doc/` as
  `nova-std-docs-<version>.tar.gz`. The archive sits beside the binaries
  and the `.vsix`.
- **The prepare job expects it too,** and `SHA256SUMS` covers it.
- **On a pull request,** when the release workflow runs, the archive is
  built and not published, as the other assets are.

## 15. Crates and code

### 15.1 Dependencies

- **`pulldown-cmark` `=0.13.4`** (decision 1), with default features off
  and `html` on.
  - It declares Rust 1.71.1.
  - It brings `pulldown-cmark-escape` 0.11.0 and `unicase`. `bitflags` 2
    and `memchr` are already locked, so `Cargo.lock` gains three
    entries.
- **`unicase` is locked at the newest version that builds on 1.78.**
  2.10.0 was released on 2026-10-05 and declares no minimum Rust. The
  plan checks it with the local 1.78 toolchain, and falls back to 2.9.0.
- **The reasons are recorded** in the workspace `Cargo.toml`, in ADR
  0034, and in a dated note on the master spec's §6.
- **`nova-doc` depends on** `nova-driver`, `nova-resolver`, `nova-ast`,
  `nova-lexer`, `nova-diagnostics`, `nova-pm` and `pulldown-cmark`. It
  drops whatever of its current dependencies it does not use.
  `nova-cli` depends on `nova-doc`.

### 15.2 The driver keeps the merged AST

- `Analysis` gains `file: Option<nova_ast::File>`: the merged AST the
  resolver built and the checker read. It is set whenever `definitions`
  is.
- Its items carry the docs, the visibility and the impl blocks.
  `Definitions` maps each one to its module (§2).
- Keeping it is a move, not a copy, and the language server ignores it.

### 15.3 `nova-doc`'s parts

- **The model:** from the analyses to packages, modules and items, each
  with its signature's pieces and its docs.
- **The renderer:** HTML pages, from the model alone.
- **Markdown, highlighting, search and the link check.**
- **The assets,** `nova.css` and `nova.js`, embedded with
  `include_str!`.
- **`nova-cli`'s `cmd/doc.rs`** finds the project, syncs it, prints the
  diagnostics, writes the site and opens it.

## 16. Testing

### 16.1 Unit tests, in `nova-doc`

- **Markdown:**
  - a paragraph, emphasis, a list and a table;
  - raw HTML escaped, inline and in a block;
  - headings moved down;
  - a Nova code block highlighted, and a `text` one not;
  - the summary: the first paragraph, as plain text.
- **Module docs:** `//!` lines before the first token count, and lines
  after it do not; a file without any.
- **Signatures:**
  - comments removed;
  - the layout kept, and dedented;
  - a function stops at its body, a record at `{`, a sum type at `=`;
  - a constant keeps its value;
  - a link for each documented target;
  - plain text for a private item, a type parameter and a local;
  - the item's own name not linked.
- **Anchors:** each row of §6.2, and a clash getting `-2`.
- **The link check:** a missing file, a missing fragment, an absolute URL
  skipped, a `#` link within a page, and a link into another package.
- **Search:** an entry per item and member, sorted, with its text
  escaped as JSON.

### 16.2 The driver

- `Analysis.file` holds the merged items, std's included, and
  `module_of` maps them.

### 16.3 The command line (`crates/nova-cli/tests/doc.rs`)

Each test copies its project to a fixed-name directory under the
system's temporary directory, and sets `NOVA_HOME` to a fresh one.
- **A program with a path library:**
  - the pages of §6.1 exist;
  - the program's private items are documented, and only the library's
    exports;
  - a link from the program into the library, and one into std,
    resolve.
- **A library** documents its exports only.
- **An unused dependency** has no pages.
- **An error** stops it, with the diagnostic and exit 1. A warning does
  not.
- **Outside a project,** it stops and names `--std`.
- **`--std`** writes std's site alone.
- **Replacing the site:**
  - a removed module's page is gone after a second run;
  - a `target/doc/` without the marker is refused and left as it was.
- **A package named `std`** is refused.
- **`--open`:**
  - it runs `$BROWSER` with the start page;
  - a `$BROWSER` that does not exist gives a warning naming the page,
    and exit 0.
- **Two runs write the same bytes.**

### 16.4 The gate tests

- **std:**
  - `nova doc --std`;
  - every documented item and member of std has its anchor on its page.
    They are enumerated by parsing std's files, not through `nova-doc`'s
    model;
  - `check_links` finds nothing broken;
  - every std module and primitive type has its page.
- **The example:**
  - `nova doc` on a copy of `examples/05-geometry`;
  - the same two checks, for the app's items, the library's exports and
    std.
- **std's docs** are complete (§12.2).

### 16.5 Mutants

Each must fail the named test:

| # | Mutant | Caught by |
|---|---|---|
| 1 | a link to a private item emitted | §16.1's signatures, and the gate's link check |
| 2 | variants get no anchors | §16.4's std anchors |
| 3 | raw HTML passed through | §16.1's Markdown |
| 4 | comments kept in signatures | §16.1's signatures |
| 5 | the marker check skipped | §16.3's refusal |
| 6 | the link check ignoring fragments | §16.1's link check |
| 7 | a program documenting its exports only | §16.3's program |
| 8 | one std `///` block removed | §12.2 |
| 9 | follow-up 1's header rule reverted | §3's test 1 |

## 17. The gate

The phase plan's 3.5 gate, as it applies to the site:
- **"For `std/`, and for an example that uses a dependency, every public
  item has an anchor and a link check finds no broken link":** §16.4, in
  CI's Test job on three systems.
- **"std's site is attached to the GitHub release":** §14. A pull request
  builds the archive, and the next tag's release attaches it.
- **The installed `nova`:** CI's `install` job runs a new
  `.github/scripts/doc-gate.sh` with the installed `nova`, on a copy of
  the example. `nova doc` must succeed with no warning, and the front
  page, both packages' pages and std's must exist.
- **`nova test --doc`** is 3.5b's gate.

## 18. Records

- **ADR 0034, "The documentation generator":**
  - what a site documents;
  - signatures as written, linked by the checker's index;
  - a page per module, and the anchors;
  - `//!` module docs, read from comments;
  - raw HTML escaped;
  - std's pages from one analysis;
  - the marker on `target/doc/`, and the link check;
  - `pulldown-cmark`;
  - 3.5's split.
- **Dated notes:**
  - `40-TOOLING.md` §5 and §1.1;
  - `docs/phase-3-plan.md` §3 decision 9, and §4's 3.5 entry, for the
    split;
  - the master spec's §6, for the new crates;
  - `agent.md`, for its crate table's `nova-doc` row;
  - `60-EXAMPLES.md`, for `05-geometry`.
- **Also:**
  - the CHANGELOG's Unreleased section, the README, and ARCHITECTURE's
    `nova-doc` row;
  - the notes the follow-ups touch (§3, items 6 and 7).
- **The sweep:** `git grep`, across the repository minus the files the
  branch touches, for every claim that `nova doc` is still to come or
  that std has no docs.

## 19. Risks

1. **std's docs can be wrong.** They are long, hand-written text. They
   are written from the code and its tests, one module at a time, and
   the final review reads them against the code.
2. **A link to nowhere.** The link check runs on every `nova doc`, and
   the gate tests run it on std and on the example, on three systems.
3. **Raw HTML from a dependency.** It is escaped (decision 10).
4. **Deleting a directory `nova doc` did not write.** The marker guards
   against it (§4.3).
5. **The branch's size:** nine follow-ups, a crate, std's docs and an
   example. Each follow-up is its own commit, and std's docs are one
   commit per module.
6. **The new crates' minimum Rust drifting.** Both are pinned or locked,
   and the MSRV job builds them.
7. **std's line numbers move with the docs.** No test names a std line
   (checked on 2026-10-11; one comment in `run_tests.rs` does). The plan
   checks again.

## 20. Decisions

1. **`pulldown-cmark`** (the user, 2026-10-11): pinned `=0.13.4`, default
   features off, `html` on. `unicase` is locked at the newest version
   that builds on 1.78.
2. **3.5 is two sub-phases** (the user, 2026-10-11).
   - 3.5a: the site, std's docs and the gate.
   - 3.5b: `nova test --doc`, and tested examples.
3. **Every public std item gets docs, and each module a summary** (the
   user, 2026-10-11).
4. **PR #108's nine deferred minors come first on this branch** (the
   user, 2026-10-11).
5. **Signatures are shown as written, with each name linked through the
   checker's index.** This is the phase plan's "from the type checker",
   in a language whose signatures carry every type. Printing the
   checker's types instead would lose the author's layout, bounds and
   `where` clauses, and would say nothing the source does not.
6. **A page per module,** as the phase plan says. Items are sections
   with anchors, not pages of their own.
7. **What is documented:** a library documents its exports, and a package
   that is only a program documents every item, as `cargo doc` documents
   a binary.
8. **Every dependency the program loads is documented, and std is in
   every site.** So every name in a signature can link within the site,
   offline.
9. **Module docs are `//!` lines before the first token,** read from the
   comments. The lexer, parser and formatter do not change.
10. **Raw HTML in docs is escaped.**
11. **A doc's headings move below its item's heading.**
12. **std's pages come from one analysis of std alone,** the same in
    every site.
13. **The link check runs on every `nova doc`, and warns.**
14. **`nova doc` replaces only a `target/doc/` it wrote,** as its marker
    shows.
15. **`--std` documents std alone,** and std's site is a release asset.
16. **The builtin functions are on std's front page,** with their docs
    held in `nova-doc`.
17. **Primitive types get pages,** holding std's impls on them.
18. **A package named `std`, or two names equal ignoring case, stop
    `nova doc`.**
19. **Errors stop `nova doc`,** as they stop `nova check`. Warnings do
    not.
20. **`nova doc` needs a project, or `--std`.**
21. **`--open` uses `$BROWSER`, then the system's opener,** and failing
    to open is a warning.
22. **One program gives the same bytes on every system.**
23. **The example is `examples/05-geometry`.**
24. **std's docs have no code blocks until 3.5b tests them.**
25. **A test keeps std's docs complete.**
26. **`Analysis` keeps the merged AST.**
27. **Dev-dependencies are not documented.**
28. **A trait impl's methods are documented on the trait.** An impl shows
    its header and docs.

## 21. Not in 3.5a

- **3.5b's:** `nova test --doc`, hidden lines, and tested examples in
  std's docs and the example's.
- `[Name]` links resolved to an item.
- `--document-private-items` and `--no-deps`.
- Hover showing a module's `//!` docs.
- A page per item, source pages, and `[src]` links.
- Search by type, and a hosted site.
- A package's README as its front page.
- Reserving `std` as a package name. Only `nova doc` refuses it.
