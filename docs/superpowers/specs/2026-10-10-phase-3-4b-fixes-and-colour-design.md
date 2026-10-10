# Phase 3.4b, "Fixes and colour": design

> Status: **draft for review** (2026-10-10). The second half of Phase 3.4,
> "LSP completeness" (`docs/phase-3-plan.md` §4, the 3.4 entry). 3.4a,
> "Navigation", merged as PR #107 on 2026-10-10 (ADR 0032). The design was
> approved in three sections on 2026-10-10. Three findings made while
> writing changed it; §13's decisions 7, 8 and 9 record each, and the user
> chose decision 7. A read-only fact-check against the code then found 21
> more; decisions 27 to 36 record the ones that changed the design.

Diagnostics learn to say how to fix themselves. Where the resolver or the
type checker raises an error whose fix it can decide, it attaches the
edit. `nova check` prints the fix as a `help:` line, and the language
server offers it as a quick fix. The server also organizes a file's
imports and colours every name by what it means, from the index 3.4a
built.

## 1. What 3.4b delivers

- **Fixes on `Diagnostic`** (§3): a title and text edits, which may reach
  another file. The command line prints each title as `= help: …`.
- **The fixes** (§4): make a local mutable, import a name, "did you mean",
  make an item public, and remove an unreachable arm.
- **Code actions** (§5): each fix as a quick fix, and organize imports as a
  source action.
- **Organize imports** (§6): one block of imports, dependencies then the
  project's own modules, merged, with unused imports removed from a file
  without errors. `nova fmt` keeps blank-line-separated groups of imports,
  as gofmt does (§6.6).
- **Semantic tokens** (§7): every name the index records, by what it means.
- **The VS Code extension** (§8): the `mutable` modifier declared, and a
  smoke test that also asks for tokens and a quick fix.
- **The rest of the 3.4 gate** (§10).

## 2. The starting point (read on 2026-10-10)

### Diagnostics

- `nova_diagnostics::Diagnostic` is `{ severity, code, message, labels,
  notes }` (`crates/nova-diagnostics/src/lib.rs:90`). Nothing carries an
  edit.
- Two renderers, `emit_all` and `render_to_string`
  (`crates/nova-diagnostics/src/render.rs:16`, `:72`), hand `notes` to
  codespan-reporting, which prints each as `= <note>`.
- The server publishes diagnostics through `nova-lsp`'s `convert.rs`
  (`diagnostics_for`). A published message is the diagnostic's message and
  then each note, one per line (`convert.rs:211-221`).
- The checker's `error()` labels its span "here" (`check.rs:7288-7291`).
- Two codes are warnings: E0021, an unreachable match arm
  (`crates/nova-typeck/src/check.rs:6815`), and M0006, an unknown manifest
  key. Every other code is an error.

### The errors a fix can follow

| Code | Message | Raised at |
|---|---|---|
| E0060 | cannot assign to immutable variable `x` | `check.rs:6430` (`x = v`, `x += v`) |
| E0060 | the place forms, through `require_mutable_place` | `check.rs:6528` (`arr[i] = v`, `rec.f = v`, a `mut self` method called on an immutable local) |
| E0001 | cannot find `x` in this scope | `check.rs:3660` (a value), `:6420` (an assignment's target) |
| E0001 | cannot find function `x` in this scope | `check.rs:3714` |
| E0001 | cannot find type `x` | `check.rs:2598` |
| E0001 | cannot find record `x` | `check.rs:4994` |
| E0001 | cannot find trait `x` | `check.rs:704`, `:1214`, `:2305`, `:2361` |
| E0900 | module-qualified paths are not supported yet | `check.rs:3591`, in `check_path`, when a two-segment path's qualifier is not a type in scope: `Point::origin`, `Shape::Empty`, and a call such as `Point::new()`, which falls through to it (`:3933`) |
| E0001 | `T::V` is not a variant of the matched type | `check.rs:6979` (a pattern) |
| E0001 | cannot resolve this pattern to a sum type variant | `check.rs:7006` (a pattern such as `Shape::Circle(r)`) |
| E0014 | record `R` has no field `f` | `check.rs:5019` (a record literal) |
| E0014 | no field `f` on record `R` | `check.rs:5451` (a read), `:6663` (a write), both through `no_field_message` |
| E0014 | no method `m` on type `T`, or on array type `T` | `check.rs:5819`, `:5779` |
| E0001 | `x` is not a public item of module `m` | `crates/nova-resolver/src/lib.rs:2471` (an `import m::{x}`) |
| E0082 | unknown attribute `@a`; known attributes are: test | `lib.rs:2168`; `KNOWN_ATTRIBUTES` is `["test"]` (`:2136`) |
| E0085 | unknown `@test` argument `a` | `lib.rs:2244`; `KNOWN_TEST_ARGS` is `["should_panic"]` (`:2139`) |
| E0021 | unreachable match arm (a warning) | `check.rs:6815`, labelled at the arm's pattern |

E0060's two sites add a note: "declare it as `let mut x` to allow
assignment" (or "mutation"), or for `self`, "declare the enclosing
method's receiver as `mut self` to allow mutation" (`check.rs:6441`,
`:6538`). Every binding not named `self` gets the `let mut` advice,
including match bindings and `for` variables, where `mut` does not parse.

"cannot find module `m`" (resolver `lib.rs:2360`, `program.rs:510`,
`:534`) names a module the loader could not find.

### Locals and `mut`

- `let mut x` parses, with the flag on the `let` (`parse_let_stmt`,
  `crates/nova-parser/src/grammar.rs:1422`). `let` takes only a name or `_`
  (E0022, `check.rs:3263`).
- Function and method parameters, and closure parameters, accept `mut`
  (`grammar.rs:594`, `:2252`).
- **A pattern never accepts `mut`:** the parser builds every
  `Pattern::Ident` with `is_mut: false` (`grammar.rs:2636`), so
  `Some(mut y)` is a parse error.
- **Nor does a `for` loop:** `for mut i in 0..3` is P0001, "expected
  pattern (in statement), found 'mut'", as `for_loop_var`'s comment records
  (`check.rs:4503`).
- `let x` without an initializer is E0900 and binds nothing
  (`check.rs:3228`).
- The checker's locals are `hir::Local`s: a name, a type, `is_mut` and a
  span (`FnCtx::new_local`, `check.rs:353`). They do not record how they
  were bound. `hir::Local` is built at `check.rs:369` and `:4927`, the
  driver's `lib.rs:510`, and `nova-mir/src/mono.rs:472` and `:733`.
- User locals go through `bind_local` (`check.rs:7183`). `self` is bound
  by the same parameter loop as the other parameters (`check.rs:3011-3018`).
- `place_root`'s `PlaceRoot::ImmutableLocal` carries the root's name only
  (`check.rs:6496`), not the local.

### Imports and modules

- An import names a module by one segment: `import m` or
  `import m::{a, b}`. `import m as n` is refused (3.4a).
- **`import m` is a glob.** It binds every public name of `m`, in all three
  namespaces (`lib.rs:2384`). `import m::{a}` binds `a` in each namespace
  `m` exports it in (`lib.rs:2411`). So `import m` beside `import m::{a}`
  is E0002 today: the list's `a` is "already defined or imported in this
  module" (`lib.rs:2549-2563`).
- A module's scope binds the builtins first (`lib.rs:1639`), then its
  items and imports, and std's glob last (`lib.rs:1689-1700`). An import
  of a name std also has wins without a conflict; one the builtins have
  is E0002.
- What `m` names follows 3.3a §4.3: a file of the same package and
  directory, or a dependency's `src/lib.nova` by its import name, or, from
  `tests/`, the package itself. The driver decides it while loading and
  hands the resolver a table per module, `ModuleSource.imports`, keyed by
  the names the file actually imports (`lib.rs:1407`).
- **The loader parses only what the roots reach,** breadth first through
  imports (`crates/nova-driver/src/program.rs:311`). A module nothing
  imports is never read.
- Loose programs go through the same loader (`Place::Loose`,
  `program.rs:475`). `name_imports` serves only `resolver::resolve`
  (`lib.rs:1516`) and the resolver's tests. `ModuleSource` is built at the
  driver's `analyze.rs:163` and `lib.rs:730`, and the resolver's
  `lib.rs:1421` and `:1605`.
- `Definitions::names_in_scope` (`lib.rs:1362`) lists a module's names in
  all three namespaces, its imports, the builtins and std's names
  included. Completion uses it (`completion.rs:153`).
- Each loaded module knows its package (`Loaded.package`); the driver's
  `Analysis` carries them as `module_packages`.
- The resolver computes each module's exports, its public names by
  namespace, while resolving imports, and keeps them only for that.
- Std is glob-imported into every module, so a std name is never "cannot
  find", and std's sources contain no `import`.
- The `.nova` corpus has three `import` lines, two of them in
  `tests/runtime/modules/main.nova`, and no blank line between imports.
- **Method calls ignore imports.** `resolve_method_on` searches every impl
  in the program (`check.rs:5531-5568`), filled from all merged items
  (`:1090`). A trait's methods resolve whether or not the file imports the
  trait.
- **With `keep_going`, as the server runs,** `analyze.rs:208` drops every
  E0001 whose first backticked name is an item the parser dropped, in any
  loaded file (`program.rs:352`, `analyze.rs:231-250`). A parse error in
  one module can so hide another file's resolution errors.

### What the front end can see of the source

- The resolver gets each module's AST (`ModuleSource.file`), and the
  checker the merged AST (`check_with(file, defs, options)`,
  `check.rs:125`). `File` is its items (`nova-ast/src/lib.rs:30-32`).
  Neither sees source text.
- The loader lexes with `nova_lexer::lex`, which drops comments
  (`program.rs:344`); only `lex_with_comments` keeps them.
- An item's span starts at its `///` docs and attributes
  (`grammar.rs:415-416`, `:504`). `Attribute.span` covers only the `@`.
  `Def.span` is the item's name.
- A match arm has no span of its own (`nova-ast/src/expr.rs:144-148`), and
  the parser does not record where an arm's comma was (`grammar.rs:2236`).
  The checker has the arm's pattern span and its body's span
  (`check.rs:6716`, `:6735`).

### The formatter's imports

- The printer treats each run of consecutive top-level imports as one
  (`crates/nova-fmt/src/print/mod.rs:201`). It sorts the run by path in
  byte order and each `{…}` list by name, prints one import per line with
  **no blank line inside the run**, and keeps each import's comments with
  it. Comments that a blank line separates from the run's first import
  stay above the run.
- Its output check sorts the same runs (`fingerprint` and `import_runs`,
  `crates/nova-fmt/src/check.rs:56`, `:75`).
- `crates/nova-fmt/tests/layout.rs:153-155` pins that behaviour:
  `"import b\n\n\nimport a\n"` formats to `"import a\nimport b\n"` (the
  3.1 plan's decision 3). It is the only such test in `crates/`.

### The server and the index

- `nova lsp` advertises completion, formatting, hover, definition,
  references and rename (`crates/nova-lsp/src/lib.rs:111`). Code actions
  and semantic tokens are not advertised.
- `analysis::answering` picks the analysis that answers a request
  (3.4a §4). With the index on, `Analysis.index` holds every occurrence:
  span, role, target. The targets are `Def`, `Variant`, `Field`,
  `TraitMethod`, `Local`, `TypeParam`, `Module`, `Builtin`,
  `BuiltinMethod` and `Primitive` (`crates/nova-resolver/src/index.rs:22`).
  `Index::at` ranks a shared span's meanings: value, type, trait, then
  field.
- `DefKind` distinguishes `Fn`, `ExternFn`, `Method`, `Record`, `Sum`,
  `Trait`, `Const` and `AssocType`. The builtins in every module's scope
  are five functions: `println`, `print`, `eprint`, `eprintln` and `panic`.
- `self` is recorded as a local (3.4a decision 20).

### Tests

- `crates/nova-driver/tests/broken.rs` analyses 364 cut programs and checks
  every occurrence lies inside its file.
- `crates/nova-cli/tests/lsp_navigation.rs` drives the server over stdio.
  Its `project` helper (`:20`) and `app_and_library` (`:231`), which
  writes a fixed `APP_MAIN`, are private to that file; the shared module
  is `lsp_client`.
- `crates/nova-cli/tests/lsp.rs`'s `requests_stay_within_the_ci_bound`
  measures six requests.

## 3. Fixes on `Diagnostic`

### 3.1 The types

In `nova-diagnostics`:

```rust
/// A suggested change: a title, and the text edits that make it.
pub struct Fix {
    pub title: String,
    pub edits: Vec<Edit>,
}

/// Replace `span`'s text with `text`. An empty span inserts.
pub struct Edit {
    pub span: Span,
    pub text: String,
}
```

- `Diagnostic` gains `fixes: Vec<Fix>`, empty from `error` and `warning`,
  and a builder, `with_fix(fix)`.
- A span carries its file, so a fix may edit a file other than the one its
  diagnostic is in. Only "make it public" does.
- A fix's edits never overlap, and each lies inside its file, on character
  boundaries.
- A fix never edits std, another package, or a file inside nova's caches.
  The package's own library is the project's, so a fix raised in a
  `tests/` file may edit `src/lib.nova`.

### 3.2 What a fix promises

- **Applying a fix removes its diagnostic** and makes no error code more
  common.
- **"Did you mean" is the exception.** The intended name may have another
  type, so it can expose an error at the same place.
- The promise is held by the tests of §9.1 and §9.2. A fix is not
  re-checked when it is offered (decision 13).

### 3.3 The command line

- Both renderers print each fix's title after the notes, as
  `= help: <title>`:

  ```
  error[E0060]: cannot assign to immutable variable `x`
    ┌─ src/main.nova:3:5
    │
  3 │     x = 2
    │     ^^^^^ here
    │
    = help: make `x` mutable
  ```

- Where E0060 offers a fix, its note "declare it as `let mut x` …" is not
  added; the fix says it. Where no fix is offered (a pattern binding, a
  `for` variable, `self`), the note stays.
- The note is part of the published LSP message today (§2), so where the
  fix replaces it the published message loses that line. Two checker unit
  tests assert the note on a `let`-bound receiver (`check.rs:10049`,
  `:10275`); they assert the fix instead.
- Otherwise the published diagnostic is unchanged. Fixes reach the editor
  as code actions, not inside the diagnostic.
- Fixes are computed only for errors that occur. The tables §4.6 adds are
  built on every analysis, a few entries per module.

## 4. The fixes

### 4.1 Make it mutable (E0060)

- **Where:** both E0060 sites, when the place's root is a local the user
  bound with a `let` or as a parameter (of a function, a method or a
  closure).
- **The edit:** insert `mut ` at the start of the binding's name:
  `let x` becomes `let mut x`, and `fn f(x: Int)` becomes
  `fn f(mut x: Int)`.
- **Title:** "make `x` mutable".
- **Not offered** for a pattern binding or a `for` variable, where `mut`
  does not parse (§2), or for `self`, since changing a receiver changes
  every caller. Their notes stay. `self` is bound as a parameter (§2), so
  it is excluded by name, as the existing note already tells it apart.
- **What the checker gains:**
  - beside its locals, a checker-only table of how each was bound, by a
    `let`, as a parameter, or otherwise, keyed by the local's declaration
    span. `bind_local` takes the kind. `hir::Local` does not change, so
    its five construction sites (§2) do not either;
  - `PlaceRoot::ImmutableLocal` also carries the local's declaration span,
    so `require_mutable_place` can find the binding.

### 4.2 Import a name (E0001)

- **Where:** the "cannot find" sites of §2, except an assignment's target
  (`check.rs:6420`); and an unknown qualifier, where a type was used
  without being imported: the E0900 of a two-segment path whose qualifier
  is not a type in scope (`Point::new()`, `Shape::Empty`), and the two
  pattern errors of §2 (`Shape::Circle(r)`), when their qualifier is not a
  type in scope.
- **Each site needs one kind of item:**

  | Site | Needs |
  |---|---|
  | an unknown value (`check.rs:3660`) | any value: a function, a constant or a variant |
  | an unknown function in a call (`:3714`) | a function or extern function |
  | an unknown type (`:2598`) | a record or sum type |
  | an unknown record in a literal (`:4994`) | a record |
  | an unknown trait (four sites) | a trait |
  | an unknown qualifier in an expression | a record or sum type |
  | an unknown qualifier in a pattern | a sum type |

  A candidate of another kind would trade one error for another, such as
  a sum type for a record literal (E0010, `check.rs:4996-4999`).
- **Candidates:** every module the current module could import by a
  single-segment name, by the rules of 3.3a §4.3, whose exports hold an
  item named `x` of the kind the site needs (§4.6). Std is never a
  candidate.
- **Offered when exactly one module is a candidate,** and importing it
  would bind `x` in no namespace where the current module already has `x`
  from its own items, its imports or the builtins. A list import binds `x`
  in every namespace its module exports `x` in (§2). A std name does not
  block the fix, since an import of a name std has wins without a
  conflict (§2).
- **Title:** "import `x` from `m`", where `m` is the name the import uses.
- **The edit,** placed by §4.7's line rules:
  - when the file has an `import m::{…}`, add `, x` after the list's last
    name;
  - otherwise, when the file has imports, a new line `import m::{x}` at
    the end of the last import's line, so a comment ending that line stays
    on it;
  - otherwise, a new line `import m::{x}` and a blank line, before the
    first item. That is before the item's `///` docs, its attributes, and
    any comment lines directly above it with no blank line between. A
    comment a blank line separates from the first item stays above the
    import. A file with no items gets the import at its end.
- **What it cannot see:** a module nothing imports yet, which the loader
  never reads (§2).

### 4.3 Did you mean

- **The rule:** among the candidates, other than `x` itself, a candidate
  equal to `x` ignoring ASCII case comes first, and qualifies at any
  distance, as in rustc. Then the smallest edit distance, then byte order.
  Only one suggestion is made.
- **The distance:** the optimal string alignment distance, over Unicode
  scalar values: Levenshtein's insertions, deletions and substitutions,
  and a swap of two adjacent characters, each one edit. So `cuont` is one
  edit from `count`, where Levenshtein counts two. A candidate qualifies
  within `max(len(x), 3) / 3`, rustc's bound, with `len` in characters: 1
  up to 5 characters, 2 up to 8, and so on.
- **Title:** "change `x` to `y`". The edit replaces the name's own span,
  the path segment, with `y`. That can be narrower than the diagnostic's
  label: "cannot find type" labels the whole type with its arguments, and
  "cannot find record" the whole literal.
- **The candidates, by site,** each of the kind §4.2's table says the site
  needs:
  - an unknown value or function: the locals visible there, and the
    module's values from `Definitions::names_in_scope` (§2): its items,
    its imports, std's names and the builtins;
  - an unknown type, record or qualifier: the module's types from
    `names_in_scope`, the type parameters in scope, and, for a type, the
    primitive types;
  - an unknown trait: the module's traits from `names_in_scope`;
  - an unknown field: the record's fields, less those a literal already
    gives;
  - an unknown method: every method that resolution would accept on the
    receiver's type: its inherent methods, the methods of every trait
    implemented for it anywhere in the program (resolution ignores
    imports, §2), and the builtin methods of a builtin type;
  - E0082: the known attributes; E0085: the known `@test` arguments.
- The checker's own names (`__it` and its kind) are never candidates.
- Where an E0001 offers both an import and "did you mean", the import comes
  first.

### 4.4 Make it public

- **Where:** the resolver's "`x` is not a public item of module `m`"
  (`lib.rs:2471`), when `m` declares a private item named `x` and `m` is
  in the same package as the importing module. A loose program's files
  count as one package. A `tests/` file importing its own package's
  library by name is in the same package (§3.1).
- **The edit:** insert `pub ` before the item's keyword, in `m`'s file,
  found by §4.7 from the item's name: `fn`, `async fn`, `record`, `enum`,
  `trait` or `const`. When `m` has private items named `x` in more than one
  namespace, the fix makes each public.
- **Title:** "make `x` public in `m`".
- **Not offered** for a dependency's module, which this project does not
  own.

### 4.5 Remove an unreachable arm (E0021)

- **The edit:** delete the arm, from its pattern's start to its body's
  end, with a comma that follows it. When nothing else shares the arm's
  lines, the edit deletes those whole lines. Both are found by §4.7.
- **Title:** "remove the unreachable arm".
- The diagnostic's label stays on the pattern. The checker merges the
  pattern's span with the body's (§2) to give the fix the arm.

### 4.6 What the front end gains

- **`ModuleSource` gains two fields:**
  - `package: Option<u32>`, an opaque key that is equal for two modules of
    the same package, and `None` for a loose program;
  - `importable: Vec<(String, usize)>`, every loaded module this module
    could import, by the name it would write.

  The driver fills both for every program it loads, with a package or
  loose, by 3.3a §4.3's rules over the modules it loaded. `name_imports`
  fills `importable` by name for `resolver::resolve` and the resolver's
  own tests. The four `ModuleSource` construction sites of §2 change.
- **`Definitions` keeps** each module's exports (today local to the
  import pass), its importable table and its package key, with lookups by
  name and kind. §4.3's candidates come from the existing
  `names_in_scope`.
- **The front end reads the sources:** the resolver and the checker get
  the program's `FileDb`, read-only, for §4.7. Without one, as in
  `resolver::resolve` and the resolver's tests, the fixes that §4.7 places
  are not offered.
- **The checker's locals** record how they were bound (§4.1).
- **The index gains a table of locals:** for each local's declaration
  span, whether it is a parameter and whether it is `mut` (§7). The
  existing recording sites fill it, only when the index is on, as for the
  rest of the index.
- The other tables are built on every analysis, the command line's
  included. They hold a few entries per module, and the command line's
  help lines need them.

### 4.7 Placing edits by lines

Three fixes place their edits by the source's lines, which neither AST
holds (§2). The rules read the file's text from the `FileDb`:
- **A comment line** is a line whose first character after spaces and
  tabs is `//`. A **blank line** holds only spaces and tabs.
- **After the last import** (§4.2): the edit inserts at the end of that
  import's last line, after its line ending, so a comment that ends the
  line stays with it. When that line is the file's last and has no
  ending, the edit adds one first.
- **Before the first item** (§4.2): from the item's span, which starts at
  its docs and attributes (§2), move up over comment lines with no blank
  line between, and insert at the start of the topmost.
- **An item's keyword** (§4.4): from the item's name, move left over
  spaces and tabs to the keyword, and over `async` before `fn`. `pub `
  goes before the first of them.
- **An arm's comma and lines** (§4.5): after the body's end, spaces and
  tabs, then a `,` if there is one, belong to the arm. The arm owns its
  lines when the text before its start on its first line, and after its
  end on its last line, is only spaces, tabs and the line ending; the edit
  then deletes from its first line's start through its last line's
  ending.
- Line endings may be `\n` or `\r\n`; an inserted line takes the file's
  own ending.

## 5. Code actions

- **The capability:** `codeActionProvider` with the kinds `quickfix` and
  `source.organizeImports`, and no resolve.
- **Each request runs a fresh analysis** of its file, with the index on,
  by 3.4a's rules for which analysis answers (§4 of that spec). The
  client's copy of the diagnostics may be stale, so the server does not
  read `context.diagnostics`.
- **Quick fixes:** for each diagnostic in the request's file whose primary
  label overlaps the request's range, each of its fixes becomes one
  action:
  - its title is the fix's, with its first letter capitalised;
  - its kind is `quickfix`;
  - its `diagnostics` holds the diagnostic, converted as published;
  - its edit is a `WorkspaceEdit` with `changes` keyed by URI, as rename's
    is;
  - it is preferred when the diagnostic has exactly one fix.

  A range touching a label's end counts as overlapping, since VS Code
  sends the cursor as an empty range. A diagnostic that labels a wide
  span, such as a whole record literal, offers its fixes anywhere in it.
- **Identical fixes,** with the same title and the same edits, from several
  diagnostics, are offered once, naming every diagnostic.
- **Order:** by the diagnostic's position, then the fix's order on it;
  organize imports last.
- **`context.only` is honoured:** an action is returned when its kind is an
  entry of `only` or starts with an entry followed by `.`.
- **Read-only places get none:** a request inside std's cache or a
  downloaded package returns no actions, as rename refuses there. A fix
  with an edit in a file outside the owning project is dropped, as a
  second guard.
- **Organize imports** (§6) is one action, of kind
  `source.organizeImports`, titled "Organize imports", offered only when
  it would change the file.

## 6. Organize imports

### 6.1 When it is offered

- When the request's file lexes and parses without error, and organizing
  would change it. So a second run offers nothing.
- The work lives in `nova-fmt`, beside the printer's import sorting, so
  the two orders cannot drift. The server supplies, for each import, its
  group, and which of its names, or whether its glob, is unused.

### 6.2 The block

- **Every top-level import in the file** goes into one block, at the first
  import's place. The block replaces the first import's lines. Each other
  import's lines are deleted, with the blank line after them, so items
  between scattered imports stay where they are, one blank line apart.
- **Two groups, a blank line between:**
  - **dependencies:** an import whose module is in another package, or is
    the package's own library imported by its name from `tests/`;
  - **the project's own modules:** every other import, including one whose
    module could not be found.

  A loose program has only the second group.
- **Within each group, the formatter's order:** by path in byte order, and
  each `{…}` list by name. §6.6 makes the formatter keep the groups.

### 6.3 Merging

- `import m::{a}` and `import m::{b}` become `import m::{a, b}`, with the
  names sorted and duplicates removed.
- Two `import m` become one.
- `import m::{a}` beside `import m` is removed, since the glob binds every
  name the list does, and the pair is E0002 today (§2). This is merging,
  so it happens in a file with errors too; its comments go above the
  glob.

### 6.4 Unused imports

- **Removed only when the analysis reports no error in the file, and no
  loaded file has a lex or parse error.** A broken line may be a use the
  checker could not record. A parse error elsewhere can hide this file's
  errors (§2: with `keep_going`, an E0001 naming an item the parser
  dropped is filtered out), so it blocks removal too. Warnings do not
  block removal.
- **A name in a list** is used when an occurrence in the file, other than
  the import's own, targets something the name binds. A use of an enum's
  variant counts as a use of the enum, and a call of a trait's method as a
  use of the trait. Method resolution does not need the trait imported
  (§2), but a reader of the file uses the trait through its methods, so
  organize imports keeps it. A list whose names are all unused is
  removed; otherwise only its unused names go.
- **A glob `import m`** is used when an occurrence in the file targets
  anything declared in `m`: an item, a variant or field of its types, or
  a method of its traits. This counts more uses than the glob may
  provide, which only ever keeps an import.

### 6.5 Comments, docs and attributes

- Comment lines directly above an import, with no blank line between, and
  a comment ending its line, travel with it, as do its `///` docs and
  attributes.
- A comment that a blank line separates from the first import stays where
  it is, as the formatter's header rule does.
- Any other comment between the first and last import attaches to the
  import after it.
- When lists merge, the second import's comments go above the merged one.
- An unused import's attached comments are removed with it.

### 6.6 The formatter keeps groups

- `nova fmt`, in the printer and in its output check, splits a run of
  consecutive imports into groups at a blank line, as gofmt does.
- Each group is sorted on its own, groups print one blank line apart, and
  the header rule applies to each group's first import.
- No `.nova` file in the repository has a blank line between imports, so
  no formatted file in the repository changes. A user's file with such a
  blank line now keeps it, and its groups are sorted separately.
- `layout.rs:153-155`, which pins the 3.1 plan's decision 3 (§2), changes:
  `"import b\n\n\nimport a\n"` now formats to `"import b\n\nimport a\n"`,
  two groups one blank line apart.
- ADR 0028 and `40-TOOLING.md` §2.1 get dated notes.

## 7. Semantic tokens

### 7.1 The legend

- `semanticTokensProvider` with `full: true`, no delta, no range.
- **Token types,** in this order: `namespace`, `type`, `struct`, `enum`,
  `interface`, `typeParameter`, `parameter`, `variable`, `property`,
  `enumMember`, `function`, `method`.
- **Modifiers,** in this order: `declaration`, `readonly`,
  `defaultLibrary`, and `mutable`, a custom one as rust-analyzer uses.

### 7.2 What each name is

| The occurrence's target | Token type | Modifiers |
|---|---|---|
| `Def`, a function or extern function | `function` | |
| `Def`, a method | `method` | |
| `Def`, a record | `struct` | |
| `Def`, a sum type | `enum` | |
| `Def`, a trait | `interface` | |
| `Def`, a constant | `variable` | `readonly` |
| `Def`, an associated type | `type` | |
| `Variant` | `enumMember` | |
| `Field` | `property` | |
| `TraitMethod` | `method` | |
| `Local`, a parameter | `parameter` | `mutable` when `mut` |
| `Local`, any other | `variable` | `mutable` when `mut` |
| `TypeParam` | `typeParameter` | |
| `Module` | `namespace` | |
| `Builtin` | `function` | `defaultLibrary` |
| `BuiltinMethod` | `method` | `defaultLibrary` |
| `Primitive` | `type` | `defaultLibrary` |

- A `Def`, variant, field or trait method declared in std also gets
  `defaultLibrary`.
- A declaration also gets `declaration`.
- `self` gets no token; the TextMate grammar colours it as a keyword.
- An unresolved name gets no token.

### 7.3 One token per span

- Where several occurrences share a span, 3.4a's ranking picks one:
  value, type, trait, then field. That covers `x` in `Point { x }`, and an
  import name binding two namespaces.
- Tokens are sorted by position and encoded as the protocol's relative
  deltas, with columns and lengths in UTF-16 from each file's line index.
- A span that is empty or crosses a line gives no token.

### 7.4 Which analysis answers

- The request's file only, answered by 3.4a's rules: std's cache from the
  in-memory program, and a downloaded package from its own analysis.
- In a file with errors, every name the index records is coloured.

## 8. The VS Code extension

- `package.json` declares the `mutable` modifier under
  `contributes.semanticTokenModifiers`, with a description, so themes can
  style it. Its description lists the new features.
- `vscode-languageclient` asks for semantic tokens and code actions once
  the server advertises them; nothing else in the client changes.
- The smoke test also asks VS Code for:
  - the fixture's semantic tokens, through
    `vscode.provideDocumentSemanticTokens`, and finds a function token;
  - code actions at a planted E0060, in a fixture file of its own, through
    `vscode.executeCodeActionProvider`, and finds "Make `x` mutable".

## 9. Testing

### 9.1 Each fix's variants (`crates/nova-driver/tests/fixes.rs`)

Each positive case analyses its program, finds the diagnostic, applies the
fix's edits to the sources in memory, and analyses again. The diagnostic
must be gone, and no error code more common, except after "did you mean"
(§3.2). Each "none" case asserts that no fix is offered, and the note
where one stays.

| # | Fix | Case |
|---|---|---|
| 1 | mutable | `let x = 0` then `x = 1` |
| 2 | mutable | `x += 1` |
| 3 | mutable | `arr[i] = v` on an immutable `let` |
| 4 | mutable | `rec.f = v` on an immutable `let` |
| 5 | mutable | a `mut self` method called on an immutable `let` |
| 6 | mutable | a function parameter assigned |
| 7 | mutable | a closure parameter assigned |
| 8 | mutable, none | a match binding assigned |
| 9 | mutable, none | a `for` variable assigned |
| 10 | mutable, none | a field of `self` set in a method without `mut self` |
| 11 | import | extends an existing `import m::{…}` |
| 12 | import | a new line after the last import, which ends with a comment that stays on its line |
| 13 | import | a file with no imports: above the first item and the comment lines on it, below a header comment |
| 14 | import | from a dependency's library, which another module of the program imports, so it is loaded |
| 15 | import | a type, a record literal, and a trait |
| 16 | import | an unknown qualifier: `Point::new()`, `Shape::Empty`, and the pattern `Shape::Circle(r)` |
| 17 | import, none | two modules export the name |
| 18 | import, none | importing would bind a name the module already has; a std name of the same spelling does not block it |
| 19 | import, none | a record literal whose only candidate is a sum type |
| 20 | did you mean | a local |
| 21 | did you mean | a type, and a qualifier |
| 22 | did you mean | a field, in a read and in a literal |
| 23 | did you mean | a method, one of them from a trait the file does not import |
| 24 | did you mean | an attribute, and a `@test` argument |
| 25 | did you mean | a name equal ignoring case, beyond the distance |
| 26 | did you mean, none | nothing within the distance, and its edge: a five-character name at distance 1 is offered, at 2 is not |
| 27 | public | a sibling module's private item; the edit is in the other file |
| 28 | public | from a `tests/` file, an item of the package's own library |
| 29 | public, none | a dependency's private item |
| 30 | remove arm | an arm on its own line, an arm over several lines, an arm sharing its line, and an arm with a trailing comma |

The cases run on `\n` and on `\r\n` sources where §4.7's rules place the
edit (cases 12, 13, 27, 28, 30).

### 9.2 Every fix on broken programs

`broken.rs`'s 364 cut programs: every fix's edits lie inside their file, on
character boundaries, and do not overlap. Applying each program's fixes one
at a time and analysing again does not panic.

### 9.3 The command line

`nova check` on a project with an E0060 prints `= help: make \`x\`
mutable`, and not the note. The two checker unit tests of §3.3 assert the
fix in place of the note.

### 9.4 Organize imports and the formatter's groups

- **The block, in `nova-fmt`,** given each import's group and unused names
  as inputs:
  - scattered imports, with an item between them;
  - unsorted imports, and unsorted lists;
  - lists merged, duplicate globs, and a list beside a glob of the same
    module;
  - unused list names, a wholly unused list, and an unused glob, removed;
  - comments travelling, a header comment staying, comments of a removed
    import removed;
  - the two groups;
  - nothing to do after a run;
  - `nova fmt` changing nothing in the result.
- **The classification, in `nova-lsp`,** from a real analysis:
  - a list name used and unused, a glob used and unused;
  - a trait used only through its methods, and an enum only through its
    variants, counted as used;
  - nothing unused when the file has an error, or when another loaded
    file has a parse error;
  - a dependency's import, the package's own library from `tests/`, and a
    sibling module, each in its group.
- **The formatter:**
  - two groups kept, each sorted;
  - a header comment above the second group;
  - the output check accepting a grouped file;
  - formatting twice changing nothing;
  - `layout.rs`'s blank-line case, as §6.6 changes it.

### 9.5 Semantic tokens (`nova-lsp`)

Unit tests of the classification (every row of §7.2), of one token per
span, and of the encoding, including a non-ASCII line.

### 9.6 The language server (`crates/nova-cli/tests/lsp_fixes.rs`)

Over stdio, on a project with two modules and a path dependency. The main
module imports the dependency, so its library is loaded, and the second
module uses one of its names without importing it. `project` and
`app_and_library` move into the shared `lsp_client` module, and
`app_and_library` takes the main file's text.
- the capabilities are advertised;
- a quick fix in the second module imports the name from the dependency;
- "make public" edits the sibling module's URI;
- "make mutable" at a cursor's empty range;
- `only` returns organize imports alone;
- organize imports, with both groups and an unused import;
- organize imports in a file with an error keeps the unused import;
- no actions inside a downloaded package;
- semantic tokens, decoded and checked name by name;
- tokens for a file with an error.

### 9.7 Mutants

Each must fail the named test:

| # | Mutant | Caught by |
|---|---|---|
| 1 | `mut ` inserted before the `let` keyword | §9.1 case 1 |
| 2 | the import fix offered with two candidates | case 17 |
| 3 | the distance bound `/ 2` instead of `/ 3` | case 26 |
| 4 | the import groups swapped | §9.4's groups |
| 5 | unused imports removed from a file with an error | §9.4's classification, and §9.6 |
| 6 | token positions absolute, not relative | §9.6's decode |
| 7 | `defaultLibrary` dropped | §9.5 |
| 8 | code actions answered inside a downloaded package | §9.6 |
| 9 | the formatter's groups merged again | §9.4's formatter |

### 9.8 Latency

- On `05-json-api`, in a release build: 20 code action requests at a place
  without a diagnostic, and 20 semantic token requests, three runs.
- The budget is 200 ms each, for the median and the maximum. CI's bound is
  2 s with the debug binary.
- `requests_stay_within_the_ci_bound` grows to eight requests.
- ADR 0033 records the figures, with the binary's size.

### 9.9 The extension

The smoke test of §8. Its first run is CI's, since running it downloads VS
Code.

## 10. The gate

The phase plan's 3.4 gate, with 3.4a's half:
- **The scripted LSP test covers each capability on a multi-file project
  with a dependency:** hover, definition, references and rename in
  `lsp_navigation.rs` (3.4a); quick fixes, organize imports and semantic
  tokens in `lsp_fixes.rs` (§9.6).
- **The extension's smoke test** checks hover and go to definition (3.4a),
  and now semantic tokens and a quick fix.
- The latency budget is met and recorded (§9.8).

With this gate met, 3.4 is complete.

## 11. Records

- **ADR 0033, "Fixes and colour in the language server":**
  - fixes made where the error is found, on `Diagnostic`, printed as
    `help:` lines;
  - no re-check of a fix when it is offered;
  - organize imports' groups, as the meaning of §2.1's "std first then
    third-party" in a Nova that never imports std;
  - the formatter's blank-line groups;
  - names-only tokens, whole documents;
  - the measured latency.
- **Dated notes:**
  - `nova-spec/40-TOOLING.md` §3.1, for code actions and semantic
    highlighting, and §2.1, for import groups;
  - `docs/phase-3-plan.md` §4's 3.4 entry: 3.4 is complete;
  - ADR 0028, for the formatter's groups;
  - ADR 0029, for code actions and semantic tokens;
  - `agent.md`.
- **Also:** the CHANGELOG's Unreleased section; the README's editor
  paragraph; `tools/vscode-nova/README.md` and the extension's
  description; ARCHITECTURE's rows for `nova-diagnostics`,
  `nova-resolver`, `nova-typeck`, `nova-fmt` and `nova-lsp`.
- **The sweep:** every claim that code actions or semantic highlighting
  are 3.4b's or still to come, and every claim that imports sort as one
  run, found by `git grep` across the repository minus the files the
  branch touches.

## 12. Risks

1. **A fix that breaks the program.** §9.1 applies every variant and
   analyses again; §9.2 applies every fix on 364 broken programs.
2. **An import fix that does not resolve,** from a wrong importable table.
   §9.1's cases 11 to 15 analyse after applying, including a dependency.
3. **Organize imports removing a used import.** Only in a file without
   errors, and only while no loaded file has a parse error, which could
   hide this file's errors (§6.4). Uses are counted generously, and
   §9.4's classification tests pin each rule.
4. **The formatter's change.** A user's file with a blank line between
   imports now keeps it. No file in the repository has one, and the
   change is recorded in ADR 0028.
5. **Latency.** VS Code asks for code actions on every cursor move, and
   each request analyses. §9.8 measures it, and ADR 0029's budget is the
   stop.
6. **"Did you mean" on many errors.** Each search scans a scope's names.
   §9.2's sweep runs it over 364 broken programs.
7. **Overlapping edits,** which clients reject. §9.2 checks every fix, and
   identical fixes are offered once.
8. **Scripts reading the command line's output.** The `help:` lines are
   added after the notes, and E0060's note gives way to its fix; nothing
   else in the output moves.
9. **Placing an edit by lines** (§4.7). The rules read text the ASTs do not
   hold, so a layout the tests miss could misplace an edit. §9.1 runs the
   placed cases on both line endings, and §9.2 checks every edit's bounds
   on 364 broken programs.

## 13. Decisions

1. **The fixes are the determined ones** (user, 2026-10-10): make mutable,
   import a name, did you mean, make public, remove an unreachable arm.
   Fixes that write code (trait stubs, match arms, record fields) are not
   in.
2. **Organize imports sorts, groups and removes unused imports** (user,
   2026-10-10), removing only in a file without errors.
3. **Semantic tokens cover names only, for whole documents** (user,
   2026-10-10). Keywords, literals and comments stay with the grammar.
4. **The command line shows fixes as `help:` lines** (user, 2026-10-10).
5. **Fixes are made where the error is found** (user, 2026-10-10, approach
   A of three). The site that raises an error knows what its fix needs. A
   later pass would recover context the checker discarded; a structured
   suggestion rendered later would split each fix across two crates.
6. **Each fix variant has its own test** (user, 2026-10-10), with the sweep
   and mutants.
7. **The formatter keeps blank-line-separated import groups** (user,
   2026-10-10). The approved design said organize imports' groups would
   survive format on save; the formatter in fact merges a run across blank
   lines, which was found while writing this spec. Of three choices
   (gofmt's groups, no groups, the formatter grouping by kind), the user
   chose gofmt's.
8. **"Make mutable" covers `let` bindings and parameters only.** The
   approved design named pattern bindings too; the parser rejects `mut` in
   patterns and in `for` loops, which was found while writing this spec.
9. **`import m` is a glob** (§2), so organize imports judges it by the uses
   of what `m` declares. The approved design treated it as binding the
   name `m`; this was found while writing this spec.
10. **Constants get `readonly`,** so a constant does not look like a local.
    The approved section named it.
11. **An import fix needs exactly one candidate,** and must not bind a name
    the module already has.
12. **"Did you mean" follows rustc's bound,** with a case-insensitive match
    first, then distance, then byte order, and offers one name. The
    distance counts a swap of adjacent characters as one edit, since a
    swap is the commonest typo and plain Levenshtein would put `cuont`
    out of reach of `count`.
13. **A fix is not re-checked when offered.** Rename re-analyses because a
    rename is requested once; code actions are requested on every cursor
    move, so §9.1's tests carry the promise.
14. **Code actions read a fresh analysis,** not the client's diagnostics,
    which may be stale.
15. **Identical fixes are offered once.**
16. **Titles are lower case on the command line** ("help: make `x`
    mutable") and capitalised in the editor ("Make `x` mutable").
17. **`self` gets no fix and no token.**
18. **No actions inside std's cache or a downloaded package,** as rename
    refuses there, and fixes with an edit outside the project are dropped.
19. **Organize imports lives in `nova-fmt`,** sharing the printer's sort,
    and the server supplies groups and unused names.
20. **An unused import takes its attached comments with it.**
21. **The front end's new tables are built on every analysis.** They are a
    few entries per module, and the command line's help lines need them.
22. **E0060's note gives way to the fix** where a fix is offered, and stays
    where none is.
23. **Code actions analyse with the index on,** which organize imports
    needs.
24. **Budgets:** 200 ms for code actions and semantic tokens; CI 2 s.
25. **An import fix names the module as an import would,** by 3.3a §4.3.
26. **For grouping, a dependency is an import of another package's
    library,** or of the package's own library from `tests/`. Every other
    import, an unresolved one included, is the project's own. For editing
    (§3.1), the package's own library is the project's.
27. **Three fixes place their edits by lines of the source** (§4.7), which
    the resolver and checker read from the `FileDb`. The fact-check found
    that neither sees text or comments, so the layout rules of §4.2, §4.4
    and §4.5 had nothing to work from. Giving the front end read access
    to the sources keeps decision 5's approach.
28. **The import fix also covers an unknown qualifier** (`Point::new()`,
    `Shape::Empty`, `Shape::Circle(r)`), the commonest way to use a type
    without importing it. The fact-check found these are E0900 and two
    pattern errors, not "cannot find".
29. **A candidate must be the kind of item its site needs** (§4.2's
    table). The fact-check found that matching by namespace alone could
    trade one error for another.
30. **Unused imports wait for a program without parse errors** (§6.4). The
    fact-check found that `keep_going` filters out E0001s that name items
    the parser dropped, in any loaded file.
31. **A list import beside a glob of the same module is removed** (§6.3),
    in any file. The fact-check found that the pair is E0002 today, so the
    approved "both stay" would have kept an error.
32. **Method candidates are every method resolution accepts** (§4.3),
    since method resolution searches every impl in the program, not those
    in scope (fact-check). For the same reason, a trait is not needed for
    its methods; organize imports keeps it anyway (§6.4).
33. **E0060's published message loses the note where a fix replaces it**,
    and two checker unit tests change (§3.3). The approved design said the
    published diagnostic was unchanged; the fact-check found that notes
    are part of the published message.
34. **A name equal ignoring case qualifies at any distance,** as in rustc
    (§4.3).
35. **"Did you mean" for a module name is not in.** The modules an import
    could name are files the loader did not read, and the candidates would
    need its directory listings (§14).
36. **How a local was bound lives in a checker-only table,** so `hir::Local`
    and its five construction sites do not change (§4.1).

## 14. Not in 3.4b

- Fix-all, and a `nova fix` command.
- `codeAction/resolve`.
- Delta and range requests for semantic tokens.
- Fixes that write code: stubs for a trait's missing methods (E0070), arms
  for a non-exhaustive match (E0020), missing record fields (E0014).
- Importing from a module the program does not load yet.
- "Make mutable" for a `mut self` receiver.
- E0060's note for a match binding or a `for` variable, which advises
  `let mut`, though `mut` does not parse there: noted here, not changed.
- "Did you mean" for a module name in an import (decision 35).
- Requests the phase plan does not list: `documentHighlight`,
  `typeDefinition`, `implementation`, `documentSymbol`, `workspace/symbol`.
- `salsa`, unless the budget is missed (ADR 0029).
- Phase 4's rows of `40-TOOLING.md` §3.1: inlay hints, code lens, call
  hierarchy.
