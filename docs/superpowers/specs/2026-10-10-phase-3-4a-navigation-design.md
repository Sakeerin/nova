# Phase 3.4a, "Navigation": design

> Status: **draft for review** (2026-10-10). The first half of Phase 3.4,
> "LSP completeness" (`docs/phase-3-plan.md` §4, the 3.4 entry). On
> 2026-10-10 the user split 3.4 in two: 3.4a, "Navigation" (this spec),
> and 3.4b, "Fixes and colour" (suggested edits, code actions, organize
> imports and semantic tokens), which gets its own spec. The design was
> approved in three sections on 2026-10-10; §12 lists every decision.

The language server learns what every name in a program means. The type
checker, which already decides that, records each name it resolves in an
index. Hover, go to definition, find references and rename all read the
index. Rename checks its own work: it analyses the renamed program again
and refuses if any name would change meaning.

## 1. What 3.4a delivers

- **The index** (§3): every name's place, what it means, and whether it is
  a declaration or a use, recorded by the type checker and the resolver
  when the server asks.
- **Hover** (§5.1): a name's declaration on one line, its inferred type for
  a local, and its `///` docs as Markdown.
- **Go to definition** (§5.2): to the declaration, in the same file, another
  module, a dependency, or std's sources written to a cache (§6).
- **Find references** (§5.3): every use in the owning project's analysis.
- **Rename** (§5.4, §5.5): across files, within the project's own package,
  checked by re-analysis.
- **The VS Code extension** (§7): its smoke test also checks hover and go to
  definition.
- **The gate** (§9): a scripted test over stdio on a multi-file project with
  a dependency, and two checks that the index is complete over every
  `.nova` file in std, `examples/` and `tests/runtime/`.

## 2. The starting point (read on 2026-10-10)

- **The server answers two requests.** `nova lsp` answers completion and
  formatting; any other request gets `MethodNotFound`
  (`crates/nova-lsp/src/lib.rs:204-255`). Its capabilities are sync,
  completion on `.`, and formatting (`lib.rs:101-119`).
- **Completion re-runs the front end per request.** `analysis_at` analyses
  the file's project if that reaches the file, or else the file alone
  (`crates/nova-lsp/src/completion.rs:41-66`). Diagnostics run on the
  checker thread (`checker.rs`), completion on the protocol thread.
- **3.2's probe looks at one place.** `ProbePoint` and `ProbeResult`
  (`crates/nova-typeck/src/lib.rs:36-67`) give the receiver and members
  at a `.`, and the locals at the cursor. The hooks are `probe_receiver`
  (`check.rs:5097`) and `probe_locals` (`check.rs:5273`). What the probe
  meets is resolved once its function's inference has finished
  (`check.rs:3081-3093`), so a local's type is known, not a variable.
- **Nothing records where a name points.** The checker resolves names
  itself, per module, through `Definitions::resolve_value`,
  `resolve_type` and `resolve_trait`
  (`crates/nova-resolver/src/lib.rs:1273-1293`). Twenty-one functions of
  `crates/nova-typeck/src/check.rs` (16,739 lines) call them or look up a
  local, among them `check_path`, `check_call`, `check_record_literal`,
  `check_pattern`, `variant_pattern`, `convert_ty`, `resolve_bounds`,
  `apply_where`, `find_assoc_fns`, `qualifier_self_ty`, `check_for`,
  `check_closure`, `check_block` and `check_fn_body`. Locals are made by
  `new_local` (`check.rs:337`, `:351`) and found by `FnCtx::lookup`
  (`check.rs:330`).
- **Members are resolved on the spot.** `check_field` (`check.rs:5285`),
  `check_field_set` (`:6485`) and `check_method_call` (`:5609`) resolve
  against the receiver's type where the access is checked. A receiver
  whose type is still unknown is E0011, "cannot infer the receiver's type"
  (`:5623-5629`). `resolve_method_on` (`:5398`) answers
  `MethodRes::Inherent(DefId)` or `MethodRes::Trait(trait, index)`
  (`check.rs:31-39`).
- **The HIR cannot locate a name.** A HIR `Expr` carries the span of the
  whole expression (`crates/nova-hir/src/lib.rs:986-990`), and the HIR is
  desugared: no interpolation, compound assignment or `for`
  (`:1035-1036`). A method call's name has no span there.
- **The AST can.** Path segments are `Spanned<String>`
  (`crates/nova-ast/src/lib.rs:71-73`). A record literal's field may be
  shorthand, `{ x }` for `{ x: x }` (`crates/nova-ast/src/expr.rs:152-156`),
  and so may a record pattern's (`crates/nova-ast/src/pattern.rs:31-36`).
- **Docs are attached.** Functions, records, fields, type declarations,
  variants, traits, trait signatures, impls, consts, imports, modules and
  extern blocks carry `docs: Vec<Spanned<String>>`, one entry per `///`
  line (`crates/nova-ast/src/item.rs:50-278`; 3.1).
- **Definitions.** `Def { name, span, kind }`
  (`crates/nova-resolver/src/lib.rs:1198-1202`); a function's span is its
  name's (`:1830`). `DefKind` (`:1146-1176`) has `Fn`, `Sum` (with its
  variants), `Record`, `Const`, `Trait`, `Method` (in an impl, or a trait's
  default body), `ExternFn` and `AssocType`. A trait's required methods
  have no `DefId`. `Res` is `Def`, `Variant` or `Builtin` (`:1206-1213`).
  `builtin_signature` (`check.rs:7418`) gives each builtin's types.
- **Three forms are refused today:** type aliases ("type aliases are not
  supported yet", `resolver lib.rs:1897-1902`), `import … as`
  (`:2435-2440`), and qualified import paths (`:2329-2336`).
- **Imports.** `resolve_import` (`resolver lib.rs:2317`) binds a target
  module's public names: all of them for `import m`, a glob, or the listed
  ones for `import m::{a, b}`. One name may bind into the value, type and
  trait namespaces (`:2363-2430`). `import json` names a package's library
  module (ADR 0030).
- **Std.** Sixteen modules (`STD_MODULES`, `resolver lib.rs:1712-1729`),
  and `std/test` under `nova test` (`:1736`), are registered in each
  analysis as `<std/core>` and so on (`crates/nova-driver/src/analyze.rs:144`,
  `:150`). Std's 17 files hold 6,141 lines, and their bodies are checked
  on every analysis. No file of std exists on the user's disk.
- **The driver.** `Options` (`analyze.rs:44-57`) has `keep_going`, `tests`,
  `module_only` and `probe`; `Analysis` (`:67-86`) has the file database,
  diagnostics, modules and their packages, the graph, `definitions`, the
  typed module and the probe's result. `Program::loose`, `for_file` and
  `for_package` are at `crates/nova-driver/src/program.rs:62`, `:78`,
  `:99`.
- **Caches.** The server publishes nothing for a project inside
  `$NOVA_HOME/registry` (`in_the_cache`, `checker.rs:257-267`). The runtime
  cache unpacks to `$NOVA_HOME/runtime/<version>-<crc32>/`, verifies an
  existing copy and replaces a damaged one, and writes through temporary
  names (`crates/nova-driver/src/runtime_cache.rs:74-156`).
- **Names that are not identifiers.** The lexer's 32 keywords include
  `self` and `Self` (`crates/nova-lexer/src/lib.rs:36-40`).
  `RESERVED_TYPE_NAMES` holds `Int`, `Float`, `Bool`, `Char`, `String`,
  `Future` and `Bytes` (`resolver lib.rs:1141-1142`).
- **Tests.** `crates/nova-cli/tests/lsp.rs` (1,026 lines) drives `nova lsp`
  over stdio through `tests/lsp_client` (`Client::start_with_env`,
  `mod.rs:36-48`). It has an app-and-library helper (`lsp.rs:687`), a
  latency test for the development host that is ignored by default
  (`:559`) and a CI bound (`:546`). `crates/nova-driver/tests/broken.rs`
  analyses 364 cut programs (`:74`).
- **The budget.** ADR 0029 §3: 200 ms for the median and the maximum on
  `05-json-api` in a release build; edits measured 17-18 ms (median) and up
  to 24 ms, completions 13 ms and up to 21 ms. CI asserts 2 s with the
  debug binary.
- **The extension's smoke test** (`tools/vscode-nova/test/suite/smoke.test.ts`)
  checks the language, a diagnostic, a completion and formatting. Its
  fixture's three files under `src/` import nothing.
- **The protocol crates.** `lsp-server` 0.7.8 has
  `ErrorCode::RequestFailed` (-32803); `lsp-types` 0.97.0 has the
  prepare-rename types (`src/rename.rs`). 3.4a needs no new crate.

## 3. The index

### 3.1 Occurrences and targets

The index is a list of occurrences, owned by `nova-typeck` and returned by
the checker:

```rust
pub struct Index {
    pub occurrences: Vec<Occurrence>,
}

pub struct Occurrence {
    /// The name's own token, not the expression around it.
    pub span: Span,
    pub role: Role,
    pub target: Target,
    /// A shorthand field `{ x }`, which is a field and a local at once.
    pub shorthand: bool,
}

pub enum Role { Declaration, Use }

pub enum Target {
    /// A function, extern function, const, record, sum type, trait,
    /// method or associated type.
    Def(DefId),
    Variant(DefId, u32),
    Field(DefId, u32),
    /// A trait's method, by its index among the trait's items.
    TraitMethod(DefId, u32),
    /// A local, parameter or pattern binding, named by its declaration's
    /// span.
    Local(Span),
    /// A type parameter, named by its declaration's span.
    TypeParam(Span),
    Module(ModuleId),
    Builtin(Builtin),
    /// `Int`, `Float`, `Bool`, `Char`, `String`, `Bytes` or `Future`.
    Primitive(&'static str),
}
```

Each target has at most one `Declaration` occurrence. `Builtin` and
`Primitive` have none, and a `Module` has none in the index. Its
declaration is its file.

The index also keeps each local's and type parameter's type, keyed by its
declaration span, for hover (§3.4).

Lookups:

- `at(file, offset)`: the occurrence whose span holds the offset
  (`start <= offset < end`). If there is none, the one that ends at the
  offset, so a cursor just after a name finds it. At a shorthand the
  local's occurrence wins (decision 5).
- `of(target)`: every occurrence of a target, in file then offset order.
- `declaration(target)`: its `Declaration` occurrence.

### 3.2 Where the checker records

With `CheckOptions { index: true, .. }` the checker records at the places
it already decides what a name means. It never reads the index, so
checking behaves exactly as before.

- **Declarations:**
  - item names: functions, extern functions, consts, records, sum types,
    traits, and methods in impls and traits;
  - fields and variants;
  - parameters, extern and trait-signature parameters included;
  - `let` and pattern bindings, `x @ pat`, closure parameters, `for`
    variables;
  - type parameters;
  - a trait's associated types.
- **Uses:**
  - a local's lookup;
  - a value or type path, each segment: `Shape::Circle` records the type
    and the variant, `Vec::new` the type and the associated function;
  - type annotations, generic arguments, associated type bindings
    (`Item = Int`), trait bounds, `where` clauses, and `impl Trait for
    Type` headers;
  - field reads and writes, record literals' and patterns' field names;
  - method calls: `Def(method)` for an inherent method,
    `TraitMethod(trait, index)` for one resolved through a trait;
  - variant patterns;
  - calls to builtins, and primitive type names.
- **`self` and `Self`.** `self` is the receiver parameter, so it is a local:
  its declaration is the method's `self`. `Self` inside an impl is a use
  of the impl's type. `Self` inside a trait is not recorded, because it
  names no single type.
- **Shorthand.** `Point { x }` records two occurrences on the span of `x`:
  a field use, and a local use (in a literal) or a local declaration (in a
  pattern). Both are marked `shorthand`.

### 3.3 Imports

The resolver records the occurrences of import items, and the driver merges
them into the checker's index:

- the module or package of `import m` and `import json`: a use of
  `Module(target)`;
- each name of `import m::{a, b}`: a use of each target it binds, one
  occurrence per namespace (value, type, trait) the name binds into.

A glob import binds names at the import's span. That is a fact about
scope, not a use of each name, so it records nothing beyond the module.

### 3.4 Types of locals

A local's type is read after its function's inference has finished, with
the inference context's substitution applied, as the probe does today
(`check.rs:3081-3093`). A closure's locals take its enclosing function's
substitution. A type that is still unknown is recorded as unknown, and
hover then shows the declaration without a type.

### 3.5 What is not recorded

- A name that does not resolve: the checker has reported it, and no
  target exists.
- The placeholders error recovery makes (an unfinished `x.`, a missing
  name).
- Attribute names (`@test`), and `Self` inside a trait (§3.2).

### 3.6 The driver

- `nova_driver::Options` gains `index: bool`, off by default. `Analysis`
  gains `index: Option<Index>`, with the resolver's import occurrences
  merged in.
- The server's diagnostics checks and every CLI command leave it off, so
  `nova check`, `build`, `run` and `test`, and the server's diagnostics,
  are unchanged.

## 4. Which analysis answers a request

Every request re-runs the front end with the index on, as completion does.
Nothing is kept between requests (decision 13).

1. **A file in a project, or a loose file:** the project's analysis if it
   reaches the file, or else the file alone, as `analysis_at` decides
   today.
2. **A file in std's cache** (§6): a one-line program held in memory, with
   `tests` on so `std/test` is included. The request is looked up in that
   analysis's `<std/…>` file of the same module.
3. **A file in a downloaded package** (`$NOVA_HOME/registry`): its own
   package's analysis, by rule 1. Names it imports from its own registry
   dependencies resolve only if that analysis can resolve them, so a
   request on one of those may find nothing (§11, risk 4).

**The owning project** of a request is rule 1's analysis. References and
rename reach that analysis and no further (decision 2). A library's
dependents are other projects, even when they are open.

## 5. The requests

The server advertises `hoverProvider`, `definitionProvider`,
`referencesProvider`, and `renameProvider` with `prepareProvider: true`.
A request whose document is not open, or whose place holds no occurrence,
gets an empty result (`null`, or an empty list). It never gets an error.
Each request is answered on the protocol thread, guarded against a panic
as completion is. A panic gives an empty result and a logged warning.

### 5.1 Hover

The result is Markdown: a fenced `nova` code block holding the declaration
on one line, then the `///` docs, if any, after a blank line.

| Target | The code block |
|---|---|
| function, method, extern function | its signature as written, up to its body, on one line (completion's rule, `completion.rs:127-157`) |
| trait method | its signature in the trait |
| record, sum type, trait | `record Point`, `type Shape`, `trait Show`, with type parameters |
| field | `x: Int` |
| variant | `Circle(Float)`, as written |
| const | `const MAX: Int` |
| local or parameter | `let x: Int` (`let mut` for a mutable one), or `let x` when the type is unknown |
| type parameter | `T`, with its bounds |
| module | `module m` |
| package's library | `package json 1.2.0`, from the graph |
| builtin | `fn print(String) -> ()`, from `builtin_signature` |
| primitive | `type Int` |

Docs are joined with newlines, with the `///` and one following space
removed. Std has no `///` docs yet, so std's items show the code block
only (`docs/phase-3-plan.md` decision 9 leaves std's docs to 3.5).

### 5.2 Go to definition

The result is one `Location`, the declaration occurrence's name:

- a local, parameter or type parameter: its declaration;
- a function, record, field, variant, trait, const, method or associated
  type: its declaration, in whatever file;
- a method called through a trait: the trait's declaration of it
  (decision 6);
- `import m`: the start of `m.nova`; `import json`: the start of the
  package's `src/lib.nova`;
- a name in std: its place in std's cache (§6). If the cache cannot be
  written, the result is empty and a warning is logged;
- a builtin or a primitive: nothing.

A location's URI is the open document's when that file is open, else one
made from its path (3.2's rule, `checker.rs:283-286`), so a Windows client
that spells `c%3A` gets its own spelling back.

### 5.3 Find references

The result is every occurrence of the target in the owning project's
analysis, in path then offset order, declarations included only when the
request's `context.includeDeclaration` is true. An app's analysis includes
its dependencies' files, so a dependency's own uses of a name are listed
when the request starts in the app.

### 5.4 Prepare rename and rename

**`textDocument/prepareRename`** answers the name's range and its current
spelling as the placeholder, or refuses. A refusal is an error response
with `RequestFailed` (-32803) and a message saying why:

- "`len` is declared in std and cannot be renamed";
- "`parse` is declared in the dependency `json` and cannot be renamed";
- "`print` is built in and cannot be renamed", and the same for a
  primitive;
- "a module or package is renamed by renaming its file or its
  `nova.toml`";
- "`x` is in nova's cache and cannot be renamed", for a request in a file
  under `$NOVA_HOME/std/` or `$NOVA_HOME/registry/`;
- no name at the place: an empty (`null`) result, which VS Code shows as
  "the element can't be renamed".

A name may be renamed when its declaration is in the owning project's own
package: the root package of a project, or the loose program's modules.
From inside a path dependency's own files that dependency is the owning
project, so its names may be renamed there.

**`textDocument/rename`** repeats those checks, then validates the new
name. It must lex to exactly one identifier token equal to itself, so not
a keyword. It must not be one of `RESERVED_TYPE_NAMES`. A new name equal
to the old one gives an empty edit. A refusal is a `RequestFailed` error
with the reason, and nothing is edited.

The edit replaces every occurrence of the target in the owning project's
analysis. A shorthand is written out:

- renaming the field `x` to `w`: `Point { x }` becomes `Point { w: x }`, in
  a literal or a pattern;
- renaming the local `x` to `y`: `Point { x }` becomes `Point { x: y }`.

The result is a `WorkspaceEdit` whose `changes` map each file's URI (as in
§5.2) to its edits.

### 5.5 The rename check

Before answering, the server applies its edits in memory, as an overlay
over the open buffers, and analyses the same program again with the index
on. Spans in the new analysis are mapped back through the edits. The
rename is refused unless all three hold:

1. every renamed occurrence resolves to the renamed declaration;
2. no other occurrence now resolves to the renamed declaration, so nothing
   was captured;
3. no error code appears more often than before. Warnings are not
   counted.

Each refusal names what it found: "renaming `x` to `y` would make 2 other
names refer to it", "… would change what 1 name refers to", "… would add
an error: E0005 …" (the message of the first error, in file then offset
order, whose code now appears more often). The check needs
no rules about scopes. Shadowing, a clash with an import, and a duplicate
item all show up as one of the three. A program that already had errors
can be renamed, as long as the count does not rise.

## 6. Std's sources on disk

- **Where.** `$NOVA_HOME/std/<version>-<crc32>/`, where `<version>` is
  `nova`'s and the CRC-32 is over std's embedded sources in
  `STD_MODULES` order, then `std/test`. Each module is a file named after
  its `<std/…>` name: `core.nova`, `json.nova`, `test.nova`.
- **When.** On the first request that needs a location in std: a
  definition, or a reference list that includes one.
- **How.** As the runtime cache does it (`runtime_cache.rs:74-156`): a file
  whose bytes equal the embedded text is used as it is; a missing or
  different one is written to a temporary name and renamed into place.
  After a failed rename, a file that now holds the right bytes, written by
  another process, is accepted. Each file is then marked read-only, and a
  read-only file that must be replaced has the mark cleared first.
- **Back again.** A path under `$NOVA_HOME/std/` maps to the module of its
  file name, for requests inside it (§4, rule 2).
- **No diagnostics.** `in_the_cache` also covers `$NOVA_HOME/std/`, so an
  opened std file gets none.
- **Failure.** With no `NOVA_HOME` and no home directory, or a write
  error, a location in std is left out of the result, and a warning is
  logged once per server.

## 7. The VS Code extension

- The fixture gains `src/shapes.nova`, holding a `pub fn area` with a
  `///` doc, and `src/navigate.nova`, which imports `shapes` and calls
  `area`.
- The smoke test gains two cases, through `vscode.executeHoverProvider`
  and `vscode.executeDefinitionProvider` on the call in `navigate.nova`:
  the hover holds `area`'s signature and its doc, and the definition is
  `shapes.nova` at `area`'s name.
- The extension's `package.json` and client need no change, because
  `vscode-languageclient` maps the four capabilities itself. Its README
  lists the new features.

## 8. Testing

Every change is test first. The suites:

### 8.1 The index (`nova-typeck`, `nova-resolver`, `nova-driver`)

- One test per recording place of §3.2 and §3.3. Each one names marked
  places in a small program, and asserts each occurrence's role and
  target, and its target's declaration span.
- A local's type after inference, a closure's local, and an unknown type
  (§3.4).
- Nothing is recorded for an unresolved name or a placeholder (§3.5).
- `Analysis.index` is `None` unless asked for.
- Shorthand in a literal and in a pattern gives two occurrences, and
  `at` picks the local.

### 8.2 The index is complete

Two checks run over every `.nova` file in `std/`, `examples/` and
`tests/runtime/`: 17, 6 and 134 files on 2026-10-10, 157 in all.

1. **Every use has its declaration:** each `Use` occurrence's target has
   exactly one `Declaration` occurrence, unless it is a `Builtin`, a
   `Primitive` or a `Module`.
2. **Every name is covered:** each `Token::Ident` lies inside an
   occurrence, two for a shorthand. The exceptions are named in the test,
   each with its reason: attribute names, and any construct a ruling adds.

The test counts its files and asserts the count, so a corpus that shrinks
cannot pass by checking less.

### 8.3 Broken programs

`broken.rs`'s 364 cut programs are analysed with the index on as well. No
panic, no hang, and every recorded occurrence's span lies inside its
file's text.

### 8.4 The language server (`crates/nova-cli/tests/lsp.rs`)

Over stdio, one test per behaviour:

- hover on each target kind of §5.1's table, with docs and without;
- definition for each case of §5.2, into another module, a path dependency,
  a downloaded package (a local index, as 3.3b's tests use, with its own
  `NOVA_HOME`) and std;
- std's cache: the first use writes it, the second reuses it untouched, a
  damaged file is replaced, its files are read-only, a request inside it
  is answered, and it publishes no diagnostics;
- references, with and without the declaration, and from the app into a
  dependency;
- prepare rename: each refusal of §5.4, and the range and placeholder;
- rename across files, a shorthand field, a shorthand local, an invalid
  new name, a keyword, and each of §5.5's three refusals;
- each request in a file with a syntax error;
- a request for a document that is not open gets an empty result.

### 8.5 Mutants

Each of these breaks its named test:

1. method calls record nothing: §8.2's coverage check;
2. a trait-resolved method records `Def` of an impl's method: the trait
   definition test;
3. `at` ignores a cursor at a name's end: its test;
4. rename skips §5.5: the shadowing refusal test;
5. rename does not write out a shorthand: the shorthand field test;
6. references ignore `includeDeclaration`: its test;
7. prepare rename allows a std name: its refusal test;
8. std's cache keeps a damaged file: the damaged-file test;
9. the server's diagnostics check turns the index on: a test that
   `Analysis.index` is `None` for it.

### 8.6 Latency

`latency_on_05_json_api` gains hover, definition and references, and
rename, 20 of each, in a release build on the development host. Budgets
for the median and the maximum: 200 ms for hover, definition and
references; 400 ms for rename, which analyses twice. The CI bound with the
debug binary is 2 s and 4 s. The figures go into ADR 0032 with each
binary's byte size and build time. If the index pushes completion or
diagnostics past ADR 0029's budget, that is a stop for the user's word
(ADR 0029, "`salsa` now").

### 8.7 The extension

§7's two cases run in CI's `VS Code extension` job. Running them locally
downloads VS Code, which waits for the user's word. Otherwise CI's run is
their first.

## 9. The gate

- **Scripted, over stdio:** a multi-file project with a path dependency.
  The test covers:
  - hover on a local, a function with `///` docs, a field and a std
    method;
  - definition in the same file, into another module, into the dependency
    and into std;
  - references, with and without the declaration;
  - rename across files, including a shorthand field;
  - prepare rename's refusals for a std name and a dependency's name;
  - one refusal from §5.5's check, where the new name shadows another.
- **The index is complete:** §8.2's two checks pass.
- **The extension:** §7's smoke test passes in CI.
- **Latency:** §8.6's budgets are met on the development host and
  recorded.

## 10. Records

- **ADR 0032, "Navigation in the language server":** the index lives in
  the checker, rename checks itself by re-analysis, references and rename
  reach the owning project only, std's cache, and the measured latency.
- **Dated notes:**
  - `nova-spec/40-TOOLING.md` §3.1, for hover, definition, references and
    rename;
  - `docs/phase-3-plan.md` §4's 3.4 entry, for the split into 3.4a and
    3.4b;
  - `agent.md`.
- **Also:** the CHANGELOG's Unreleased section; the README's editor
  paragraph; `tools/vscode-nova/README.md`; ARCHITECTURE's rows for
  `nova-typeck` and `nova-lsp`.
- **The sweep:** every claim that the server offers only diagnostics,
  completion and formatting, or that hover, definition, references or
  rename are still to come, found by `git grep` across the repository
  minus the files the branch touches.

## 11. Risks

1. **A recording place is missed.** The checker has 21 functions that
   resolve names. §8.2's coverage check, over 157 files, fails on any name
   no occurrence covers, whatever construct holds it.
2. **Recording changes checking.** The index is written and never read by
   the checker, and the full suite runs on all three systems. The CLI's
   output tests pin that nothing it prints changes.
3. **Latency.** Requests analyse with the index on, and rename analyses
   twice. §8.6 measures both, and ADR 0029's budget is the stop.
4. **A downloaded package's own registry dependencies.** A request inside
   a downloaded package uses that package's own analysis (§4, rule 3),
   which may not resolve what it imports from the registry. Hover and
   definition on those names then give nothing, while everything else in
   the file works.
5. **URI spellings on Windows.** Locations and edits use 3.2's rule (§5.2),
   tested with a `c%3A` URI.
6. **Std's cache shared by two servers.** It is written as the runtime
   cache is, which is tested with two processes at once.

## 12. Decisions

1. **3.4 is split in two** (user, 2026-10-10): 3.4a "Navigation" and 3.4b
   "Fixes and colour" (suggested edits on `Diagnostic`, code actions,
   organize imports, semantic tokens).
2. **References and rename reach the owning project only** (user,
   2026-10-10): its program, library and `tests/`. A library's dependents
   are other projects, even when open, so the result never depends on what
   is open. A rename that breaks a dependent shows in the dependent's own
   diagnostics.
3. **The index is recorded in the checker** (user, 2026-10-10, approach A
   of three). Recording where the checker resolves keeps one source of
   truth for what a name means. A separate indexer would duplicate
   shadowing, match bindings, closures and method lookup through traits.
   Growing the probe would still need a full index for references.
4. **Type aliases are not covered.** The resolver refuses them today, so
   there is nothing to hover or rename. The approved design's lists named
   them; the fact that they are refused was found while writing this spec.
5. **At a shorthand, requests act on the local.** `x` in `Point { x }` is
   the value used or bound. The field is reached from its declaration or
   any other use.
6. **A trait-dispatched method goes to the trait's declaration.** The
   checker resolves it as `TraitMethod`, not to one impl. Going to an
   implementation is `textDocument/implementation`, which 3.4a does not
   add.
7. **Rename checks itself by re-analysis** (§5.5) rather than by rules
   about scopes. The same idea as the formatter checking its output: one
   mechanism catches shadowing, captures, clashes and duplicates.
8. **Refusals are `RequestFailed` errors with a reason**, from prepare
   rename and from rename. An empty prepare-rename result is kept for "no
   name here".
9. **A new name** must lex to one identifier and not be a reserved type
   name. Case conventions are not enforced.
10. **Std's cache is per version and per content**, like the runtime's,
    verified on use, and read-only.
11. **A request inside std's cache is answered from an in-memory
    program** that includes std, since std is part of every analysis.
12. **Rename refuses inside nova's caches.** Hover, definition and
    references work there.
13. **Requests re-run the front end** and keep nothing between requests,
    as completion does. A cache would need invalidation on every edit,
    for a cost ADR 0029 measured at 13-21 ms.
14. **Budgets:** 200 ms for hover, definition and references, 400 ms for
    rename; CI 2 s and 4 s.
15. **The completeness checks cover std, `examples/` and
    `tests/runtime/`,** the largest corpus of valid Nova in the
    repository, and assert its size.
16. **`at` prefers a name that holds the cursor**, then one that ends at
    it.
17. **Hover's form** is a fenced `nova` block on one line, then the docs.
18. **References are ordered** by path, then offset.
19. **Rename's edit uses `changes`**, keyed by the URIs locations use.
20. **`self` is a local; `Self` in an impl is its type**, and `Self` in a
    trait is not recorded.
21. **The index is off for diagnostics and for every CLI command.**
22. **Unresolved names and error-recovery placeholders are not
    recorded.**
23. **An import list's name records one occurrence per namespace it
    binds**; a glob records only its module.
24. **A package import hovers as `package <name> <version>`**, from the
    graph, and goes to its library's first line.

## 13. Not in 3.4a

- 3.4b: suggested edits on `Diagnostic`, code actions, organize imports and
  semantic tokens.
- Requests the phase plan does not list: `documentHighlight`,
  `typeDefinition`, `implementation`, `documentSymbol` and
  `workspace/symbol`. The index makes each one small later.
- Renaming files, modules or packages.
- Rename and references across a library's dependents (decision 2).
- Hover on expressions that are not names.
- Std's `///` docs (3.5 decides).
- `salsa`, unless the budget is missed (ADR 0029).
- Phase 4's rows of `40-TOOLING.md` §3.1: inlay hints, code lens, call
  hierarchy.
