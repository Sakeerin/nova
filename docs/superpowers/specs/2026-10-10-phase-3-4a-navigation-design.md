# Phase 3.4a, "Navigation": design

> Status: **draft for review** (2026-10-10). The first half of Phase 3.4,
> "LSP completeness" (`docs/phase-3-plan.md` §4, the 3.4 entry). On
> 2026-10-10 the user split 3.4 in two: 3.4a, "Navigation" (this spec),
> and 3.4b, "Fixes and colour" (suggested edits, code actions, organize
> imports and semantic tokens), which gets its own spec. The design was
> approved in three sections on 2026-10-10, then corrected after a
> read-only fact-check against the code; §12 lists every decision, and
> decisions 4, 25 and 26 record where the approved design changed.

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
  a dependency, and two checks that the index is complete over std,
  `examples/` and `tests/runtime/`.

## 2. The starting point (read on 2026-10-10)

### The server

- **It answers two requests.** `nova lsp` answers completion and
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
  (`check.rs:3081-3093`), and a closure is finalized with its enclosing
  function's inference context (`:2965-2968`, `:3035-3038`).
- **A position index was set aside once.** ADR 0029's alternatives, and
  3.2's decision 4, rejected "a full position index" because it "makes
  every check pay for tables only the server reads"
  (`docs/adr/0029-the-language-server.md:92-94`).

### The checker

- **Nothing records where a name points.** The checker resolves names
  itself, per module, through `Definitions::resolve_value`,
  `resolve_type` and `resolve_trait`
  (`crates/nova-resolver/src/lib.rs:1273-1293`). Outside its tests,
  twelve functions of `crates/nova-typeck/src/check.rs` (16,739 lines)
  call those or `FnCtx::lookup` (`check.rs:330`):
  - `collect_supertraits`, `collect_impls`, `resolve_bounds`, `apply_where`
    and `convert_ty`;
  - `check_path`, `check_call` and `check_record_literal`;
  - `qualifier_self_ty`, `check_assign`, `place_root` and `check_pattern`.

  Six more make locals: `check_fn_body`, `check_block`, `check_for`,
  `check_for_iterator`, `check_closure` and `variant_pattern`.
- **Not every local is the user's.**
  - `new_local` (`check.rs:337`) also makes the checker's own locals:
    `__f_<field>` at a field initializer's name (`:4947`), `__base`
    (`:4980`), and `_` locals, which it keeps out of scope (`:339`).
  - `new_local_unscoped` (`:351`) makes temporaries such as `__it`, `__i`
    and `__callee`.
- **The checker makes calls of its own.**
  - A `for` loop's `next` goes through a made-up path at the iterable's
    span (`check.rs:4483-4499`).
  - Interpolation calls `fmt` (`:6040-6069`).
  - A trait's provided method has its signature converted twice
    (`:918-923`, `:1006-1013`).
  - `place_root` and `check_call` look up names that `check_path` also
    resolves (`:6349-6352`, `:3582`).
- **Members are resolved on the spot.** `check_field` (`check.rs:5285`),
  `check_field_set` (`:6485`) and `check_method_call` (`:5609`) resolve
  against the receiver's type where the access is checked.
  - A method call on a receiver whose type is still unknown is E0011,
    "cannot infer the receiver's type" (`:5623-5629`). A field read on one
    is E0014 (`:5334`).
  - `resolve_method_on` (`:5398`) answers `MethodRes::Inherent(DefId)`
    or `MethodRes::Trait(trait, index)` (`check.rs:31-39`).
  - An array's `len()` is built in: `ArrayLen`, with no declaration
    (`:5631-5640`). std calls it (`std/collections/lib.nova:11`, `:28`).
- **Trait methods.**
  - `MethodRes::Trait`'s index is into the checker's `TraitDef.methods`
    (`check.rs:5453`, `:5890`). That list skips associated types (`:846`)
    and duplicate names (`:930`), so it is not the index among the
    trait's items.
  - A call on a concrete receiver whose method comes from a trait impl
    also resolves as `MethodRes::Trait` (`:5420-5432`), so no call ever
    targets an impl's method.
  - A trait's provided method also has its own `Def`, `Trait::m$default`
    (`resolver lib.rs:1986-1996`). The checker matches impl methods to
    their trait's in `check_impl_method_signatures` (`check.rs:1770`).
- **`Self`.** Bare `Self` as a type inside an impl is E0001
  (`std/http/lib.nova:773-775`). `Self` resolves only as the base of a
  projection, `Self::Out` (`check.rs:2345`; `tests/runtime/assoc_types.nova:93-94`).
  A method's `self` is a local made by `new_local` (`check.rs:2929-2930`).
- **Types print positionally.** `display_ty` shows a type parameter as
  `T0`, `T1`, … and an unsolved variable as `?n`
  (`crates/nova-typeck/src/lib.rs:112`, `:116`). An impl's and its
  method's type parameters share one numbering, the impl's first.
  `builtin_signature` (`check.rs:7418`) gives every builtin's types.

### The AST, the resolver and the forms refused today

- **The HIR cannot locate a name.** A HIR `Expr` carries the span of the
  whole expression (`crates/nova-hir/src/lib.rs:986-990`), and the HIR is
  desugared (`:1035-1036`). A method call's name has no span there.
- **The AST can.** Path segments are `Spanned<String>`
  (`crates/nova-ast/src/lib.rs:71-73`). A record literal's field may be
  shorthand, `{ x }` for `{ x: x }` (`crates/nova-ast/src/expr.rs:152-156`),
  and that `x` is checked as a path (`check.rs:4932-4933`), so it may
  name a local, a const, a function or a variant.
- **Docs are attached.** Functions, records, fields, type declarations,
  variants, traits, trait signatures, impls, associated types, consts,
  imports, modules and extern blocks carry `docs: Vec<Spanned<String>>`,
  one entry per `///` line (`crates/nova-ast/src/item.rs:50-278`; 3.1).
- **Definitions.**
  - `Def { name, span, kind }` (`crates/nova-resolver/src/lib.rs:1198-1202`).
    Every `Def`'s span is its name's: a function's at `:1831`.
  - A method's `name` is mangled (`:1990`, `:2024-2025`), so text shown to
    the user comes from the source.
  - `DefKind` (`:1146-1176`) has `Fn`, `Sum` (its variants have name
    spans, `:1871`), `Record`, `Const`, `Trait`, `Method` (in an impl, or
    a trait's default body), `ExternFn` and `AssocType`. A trait's
    required methods have no `DefId` (`:2014`).
  - `Res` is `Def`, `Variant` or `Builtin` (`:1206-1213`).
- **Imports.** `resolve_import` (`resolver lib.rs:2317`) binds a target
  module's public names: all of them for `import m`, the only glob form,
  or the listed ones for `import m::{a, b}`. One name may bind into the
  value, type and trait namespaces (`:2363-2430`). For a list import the
  module's own span is `path.segments[0].span`; `path.span` also covers
  `::`. `import json` names a package's library module (ADR 0030).
- **Refused today**, so 3.4a records nothing for them:
  - type aliases ("type aliases are not supported yet", `resolver lib.rs:1897-1902`);
  - `import … as` (`:2435-2440`) and qualified import paths
    (`:2329-2336`);
  - module-qualified paths (`check.rs:2430`, `:3495`);
  - record, tuple, array, or, range and `x @ pat` patterns, E0900
    (`check.rs:6866-6873`), and match guards (`:6569`);
  - `let` and `for` bind only a name or `_` (`:3170`, `:4397`), and a
    variant's payload only a name or `_` (`:6940`);
  - `module m` declarations, which are parsed and ignored
    (`resolver lib.rs:2077-2080`).
- **Crates.** `nova-typeck` depends on `nova-resolver`, so the resolver
  cannot use the checker's types.

### Std, the driver and the caches

- **Std.**
  - Sixteen modules (`STD_MODULES`, `resolver lib.rs:1712-1729`), and
    `std/test` under `nova test` (`:1736`), are registered in each
    analysis as `<std/core>` and so on
    (`crates/nova-driver/src/analyze.rs:144`, `:150`).
  - Std's 17 files hold 6,141 lines, and their bodies are checked on every
    analysis.
  - Std-only builtins are seeded only into std's scopes (`Builtin::STD_ONLY`,
    `resolver lib.rs:1024`), so std's files can be checked only as std.
  - No file of std exists on the user's disk.
- **The driver.** `Options` (`analyze.rs:46-57`) has `keep_going`, `tests`,
  `module_only` and `probe`. `Analysis` (`:67-86`) has:
  - the file database and the diagnostics;
  - the modules and their packages, and the graph;
  - the root's `manifest`;
  - `definitions`, the typed module, and the probe's result.

  `Program::loose`, `for_file` and `for_package` are at
  `crates/nova-driver/src/program.rs:62`, `:78`, `:99`.
- **Caches.**
  - The server publishes nothing for a project inside `$NOVA_HOME/registry`
    (`in_the_cache`, `checker.rs:257-267`, a path-prefix test).
  - The runtime cache unpacks to `$NOVA_HOME/runtime/<version>-<crc32>/`
    (`crates/nova-driver/src/runtime_cache.rs:74-112`). It verifies a
    copy by size and CRC-32 (`:114-132`), replaces a damaged one, writes
    through unique temporary names (`:139-156`), and accepts another
    process's verified copy after a failed rename (`:97-108`).
  - It marks nothing read-only. A two-process test covers it (`:323`).

### Names and tokens

- The lexer's 32 keywords include `self` and `Self`, which are keyword
  tokens (`crates/nova-lexer/src/lib.rs:36-40`).
- `_` is a `Token::Ident` (the identifier pattern, `lib.rs:804`; the
  parser's `_`, `crates/nova-parser/src/grammar.rs:2352`). A wildcard
  makes no local (`check.rs:6747`).
- Attribute arguments are identifiers too: `@test(should_panic)`
  (`tests/runtime/nova_test.nova:54`).
- `RESERVED_TYPE_NAMES` holds `Int`, `Float`, `Bool`, `Char`, `String`,
  `Future` and `Bytes` (`resolver lib.rs:1141-1142`).

### Tests, the budget and the extension

- **The LSP tests.** `crates/nova-cli/tests/lsp.rs` (1,026 lines) drives
  `nova lsp` over stdio through `tests/lsp_client` (`Client::start_with_env`,
  `mod.rs:36-48`). It has an app-and-library helper (`lsp.rs:687`), a
  latency test for the development host that is ignored by default
  (`:559`), and a CI bound (`:546`).
- **Broken programs.** `crates/nova-driver/tests/broken.rs` analyses 364
  cut programs (`:74`).
- **The corpora.**
  - std has 17 files (`std/*/lib.nova`), and `examples/` has 6 (one
    `src/main.nova` each).
  - `tests/runtime/` holds 134 `.nova` files at its top level, plus a
    three-file program in `tests/runtime/modules/`: `main.nova` imports
    `geometry` and `shout`. Those are the only imports in the three
    corpora.
  - `tests/runtime/bytes_reserved.nova` is meant to fail `nova check` with
    E0089 (`crates/nova-cli/tests/run_tests.rs:6632-6636`).
  - `tests/runtime/nova_test.nova` holds only `@test` functions.
- **The budget.** ADR 0029 §3: 200 ms for the median and the maximum on
  `05-json-api` in a release build. Edits measured 17-18 ms (median) and up
  to 24 ms; completions 13 ms and up to 21 ms. CI asserts 2 s with the
  debug binary.
- **The extension's smoke test** (`tools/vscode-nova/test/suite/smoke.test.ts`)
  checks the language, a diagnostic, a completion and formatting. Its
  fixture's three files under `src/` import nothing.
- **The protocol crates.** `lsp-server` 0.7.8 has
  `ErrorCode::RequestFailed` (-32803); `lsp-types` 0.97.0 has the
  prepare-rename types (`src/rename.rs`). 3.4a needs no new crate.

## 3. The index

### 3.1 Occurrences and targets

The index's types live in `nova-resolver` (decision 27), so the resolver
can record imports and the checker everything else:

```rust
pub struct Index {
    pub occurrences: Vec<Occurrence>,
    /// Each impl method that implements a trait's method.
    pub implements: Vec<(DefId, (DefId, u32))>,
    /// Each local's and type parameter's type, by declaration span.
    pub types: HashMap<Span, String>,
}

pub struct Occurrence {
    /// The name's own token, not the expression around it.
    pub span: Span,
    pub role: Role,
    pub target: Target,
    /// A shorthand field `{ x }`, which is a field and a value at once.
    pub shorthand: bool,
}

pub enum Role { Declaration, Use }

pub enum Target {
    /// A function, extern function, const, record, sum type, trait,
    /// impl method or associated type.
    Def(DefId),
    Variant(DefId, u32),
    Field(DefId, u32),
    /// A trait's method, by its index in the checker's list of the
    /// trait's methods.
    TraitMethod(DefId, u32),
    /// A local or parameter, named by its declaration's span.
    Local(Span),
    /// A type parameter, named by its declaration's span.
    TypeParam(Span),
    Module(ModuleId),
    Builtin(Builtin),
    /// An array's `len`.
    BuiltinMethod(&'static str),
    /// `Int`, `Float`, `Bool`, `Char`, `String`, `Bytes` or `Future`.
    Primitive(&'static str),
}
```

Rules:

- **One declaration each.** Each target has at most one `Declaration`
  occurrence. `Builtin`, `BuiltinMethod` and `Primitive` have none. A
  `Module`'s declaration is its file.
- **Provided methods.** A trait's provided method is declared once, as its
  `TraitMethod`. Its default body's own `Def` is never a target.
- **No duplicates.** An occurrence is recorded once per (span, role,
  target), however many times the checker passes the same name.
- **Families.** A trait method and the impl methods that implement it are
  one **family** (`implements`). References and rename treat a family as
  one name (§5.3, §5.4).
- **Order.** Occurrences are ordered by their file's name in the file
  database (its path, or `<std/…>`), then by offset. That one order is
  used everywhere.

Lookups:

- `at(file, offset)` finds the occurrence whose span holds the offset
  (`start <= offset < end`), or else the one that ends at the offset, so
  a cursor just after a name finds it. Several occurrences can share a
  span:
  - at a shorthand, the value's wins (decision 5);
  - at an import-list name bound into several namespaces, the value's,
    then the type's, then the trait's.
- `of(target)` lists every occurrence of a target.
- `declaration(target)` gives its `Declaration` occurrence.

### 3.2 Where it is recorded

With `CheckOptions { index: true, .. }`, the checker records only at the
AST's name sites, each with that name's own span (decision 28). It never
reads the index, so checking behaves exactly as before.

- **Declarations**, at the AST binding sites:
  - item names: functions, extern functions, consts, records, sum types,
    traits, trait methods (required and provided), and impl methods;
  - fields, variants and associated types;
  - parameters, including `self`, extern parameters and trait-signature
    parameters;
  - a `let`'s or a `for`'s name, a variant pattern's payload name, and a
    closure's parameters;
  - type parameters.

  `new_local` is not a recording place, because it also makes the
  checker's own locals.
- **Uses:**
  - a local's lookup in a path expression;
  - every segment of a value or type path: `Shape::Circle` records the
    type and the variant, `Vec::new` the type and the associated function;
  - type annotations, generic arguments, associated type bindings
    (`Item = Int`), trait bounds, supertraits, `where` clauses, and
    `impl Trait for Type` headers;
  - the associated type in a projection such as `Self::Out` (its `Self`
    is not recorded, decision 20);
  - field reads and writes, and the field names of record literals;
  - method calls:
    - `Def(method)` for an inherent method;
    - `TraitMethod(trait, index)` for one resolved through a trait;
    - `BuiltinMethod("len")` for an array's `len`;
  - variant patterns' variant names;
  - calls to builtins, and primitive type names.
- **Families.** Each impl method that implements a trait method is added
  to `implements` where the checker matches them
  (`check_impl_method_signatures`).
- **Shorthand.** `Point { x }`, in a record literal, records two uses on
  the span of `x`: the field's, and the value's (a local, const, function
  or variant). Both are marked `shorthand`.

### 3.3 Imports

The resolver records the occurrences of import items in
`ProgramResolution`, and the driver merges them into the checker's index:

- the module of `import m`, of `import json`, and of `import m::{a, b}`,
  at `path.segments[0].span`: a use of `Module(target)`;
- each name of `import m::{a, b}`: a use of each target it binds, one
  occurrence per namespace (value, type, trait) the name binds into.

A glob import binds names at the import's span. That is a fact about
scope, not a use of each name, so it records nothing beyond the module.

### 3.4 Types for hover

A local's type is read after its function's inference has finished, with
the substitution applied, as the probe does (`check.rs:3081-3093`). A
closure's locals take its enclosing function's. The type is stored as the
text hover shows:

- type parameters are printed by their declared names, the impl's first,
  not as `T0`;
- an unsolved variable is printed as `_`.

A type that is wholly unknown is not stored, and hover then shows the
declaration without one.

### 3.5 What is not recorded

- A name that does not resolve: the checker has reported it, and no
  target exists.
- The placeholders error recovery makes (an unfinished `x.`, a missing
  name).
- The checker's own names and calls: `__it`, `__f_<field>`, a `for`
  loop's `next`, interpolation's `fmt`.
- `_`, attribute names and arguments (`@test(should_panic)`), and the
  name in a `module m` declaration, which the resolver ignores.
- `Self` (decision 20).
- The forms refused today (§2), which the checker reports as errors.

### 3.6 The driver

- `nova_driver::Options` gains `index: bool`, off by default. `Analysis`
  gains `index: Option<Index>`, with the resolver's import occurrences
  merged in.
- The server's diagnostics checks and every CLI command leave it off, so
  `nova check`, `build`, `run` and `test`, and the server's diagnostics,
  pay nothing and are unchanged. That is the answer to ADR 0029's reason
  for setting a full index aside (decision 21).

## 4. Which analysis answers a request

Every request re-runs the front end with the index on, as completion does.
Nothing is kept between requests (decision 13).

1. **A file in a project, or a loose file:** the project's analysis if it
   reaches the file, or else the file alone, as `analysis_at` decides
   today. A project file that no root reaches is analysed alone, so from
   it, references see only what it imports. That matches the compiler,
   which does not build such a file (decision 31).
2. **A file in std's cache** (§6): a one-line program held in memory,
   with `tests` on so `std/test` is included. The request is looked up in
   that analysis's `<std/…>` file of the same module.
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

- A request whose document is not open, or whose place holds no
  occurrence, gets an empty result: `null`, or an empty list. It never
  gets an error.
- Each request is answered on the protocol thread, guarded against a
  panic as completion is. A panic gives an empty result and a logged
  warning.
- Text shown to the user (declarations, refusal messages) is read from
  the source, never from a `Def`'s mangled `name`.

### 5.1 Hover

The result is Markdown: a fenced `nova` code block holding the declaration
on one line, then the `///` docs, if any, after a blank line.

A declaration with a body is shown from its start to the body, collapsed
onto one line. Its start is the item's first token after its docs and
attributes, so `pub` and `async` are kept. Its end is the first `{` or `;`
outside brackets, so a `where` clause on a later line is kept too. That is
wider than completion's detail, which starts at the name.

| Target | The code block |
|---|---|
| function, method, extern function | its signature, by the rule above |
| trait method | its signature in the trait |
| record, sum type, trait | `record Point`, `type Shape`, `trait Show`, with type parameters |
| field | `x: Int` |
| variant | `Circle(Float)`, as written |
| associated type | `type Item`, in its trait |
| const | `const MAX: Int` |
| local or parameter | `let x: Int` (`let mut` for a mutable one), or `let x` when the type is unknown |
| type parameter | `T`, with its bounds |
| module | `module m` |
| package's library | `package json 1.2.0`, from the graph |
| builtin | `fn print(String) -> ()`, from `builtin_signature`, with type parameters printed as `T` |
| an array's `len` | `fn len(self) -> Int` |
| primitive | `type Int` |

Docs are joined with newlines, with the `///` and one following space
removed. Std has no `///` docs yet, so std's items show the code block
only (`docs/phase-3-plan.md` decision 9 leaves std's docs to 3.5).

### 5.2 Go to definition

The result is one `Location`, the declaration occurrence's name:

- a local, parameter or type parameter: its declaration;
- a function, record, field, variant, trait, const, associated type or
  impl method: its declaration, in whatever file;
- a method call resolved through a trait: the trait's declaration of it
  (decision 6);
- `import m`: the start of `m.nova`; `import json`: the start of the
  package's `src/lib.nova`;
- a name in std: its place in std's cache (§6). If the cache cannot be
  written, the result is empty and a warning is logged;
- a builtin, an array's `len`, or a primitive: nothing.

A location's URI is the open document's when that file is open, else one
made from its path (3.2's rule, `checker.rs:283-286`), so a Windows client
that spells `c%3A` gets its own spelling back.

### 5.3 Find references

The result is every occurrence of the target in the owning project's
analysis, in §3.1's order. Declarations are included only when the
request's `context.includeDeclaration` is true.

- **Families.** For a trait method or an impl method that implements one,
  the result covers the whole family: the trait's declaration, every
  impl's declaration of it, and every call.
- **Dependencies.** An app's analysis includes its dependencies' files,
  so a dependency's own uses of a name are listed when the request starts
  in the app.

### 5.4 Prepare rename and rename

**`textDocument/prepareRename`** answers the name's range and its current
spelling as the placeholder, or refuses. A refusal is an error response
with `RequestFailed` (-32803) and a message saying why:

- "`len` is declared in std and cannot be renamed";
- "`parse` is declared in the dependency `json` and cannot be renamed";
- "`print` is built in and cannot be renamed", and the same for an
  array's `len` and a primitive;
- "`self` is a keyword and cannot be renamed";
- "a module or package is renamed by renaming its file or its
  `nova.toml`";
- "`x` is in nova's cache and cannot be renamed", for a request in a file
  under `$NOVA_HOME/std/` or `$NOVA_HOME/registry/`.

No name at the place gives an empty (`null`) result, which VS Code shows
as "the element can't be renamed".

**What may be renamed.** A name may be renamed when its declaration is in
the owning project's own package: the root package of a project, or the
loose program's modules. From inside a path dependency's own files, that
dependency is the owning project, so its names may be renamed there.

**A family is renamed whole:** the trait's method, every impl's method of
it, and every call. So a family whose trait is std's or a dependency's
cannot be renamed. Renaming `fmt` in `impl Display for P` is refused,
because `Display` is std's.

**`textDocument/rename`** repeats those checks, then validates the new
name:

- it must lex to exactly one identifier token equal to itself, so it is
  not a keyword;
- it must not be `_`, or one of `RESERVED_TYPE_NAMES`;
- a new name equal to the old one gives an empty edit.

A refusal is a `RequestFailed` error with the reason, and nothing is
edited.

The edit replaces every occurrence of the target, or of its family, in the
owning project's analysis. A shorthand in a record literal is written out:

- renaming the field `x` to `w`: `Point { x }` becomes `Point { w: x }`;
- renaming the value `x` (a local, const, function or variant) to `y`:
  `Point { x }` becomes `Point { x: y }`.

The result is a `WorkspaceEdit` whose `changes` map each file's URI (as in
§5.2) to its edits.

### 5.5 The rename check

Before answering, the server applies its edits in memory, as an overlay
over the open buffers, and analyses the same program again with the index
on. Spans in the new analysis are mapped back through the edits. The
rename is refused unless all three hold:

1. every renamed occurrence resolves to the renamed declaration, or to a
   member of the renamed family;
2. no other occurrence now resolves to one of them, so nothing was
   captured;
3. no error code appears more often than before. Warnings are not
   counted.

Each refusal names what it found:

- "renaming `x` to `y` would make 2 other names refer to it";
- "… would change what 1 name refers to";
- "… would add an error: E0005 …": the message of the first error, in
  §3.1's order, whose code now appears more often.

The check needs no rules about scopes. Shadowing, a clash with an import,
and a duplicate item all show up as one of the three. A program that
already had errors can be renamed, as long as no error code becomes more
common.

## 6. Std's sources on disk

- **Where.** `$NOVA_HOME/std/<version>-<crc32>/`, where `<version>` is the
  server's own (the one its `serverInfo` reports). The CRC-32 is over
  std's embedded sources, in `STD_MODULES` order, then `std/test`. Each
  module is a file named after its `<std/…>` name: `core.nova`,
  `json.nova`, `test.nova`.
- **When.** On the first request that needs a location in std: a
  definition, or a reference list that includes one.
- **How.** Like the runtime cache, with two differences:
  - a file is compared byte for byte with the embedded text, which is in
    memory, rather than by size and CRC-32;
  - files are marked read-only, which is new.

  A file that already holds the right bytes is used as it is. A missing or
  different one is written to a unique temporary name and renamed into
  place; a read-only file that must be replaced has the mark cleared
  first. After a failed rename, a file that now holds the right bytes,
  written by another process, is accepted.
- **Back again.** A path under `$NOVA_HOME/std/` maps to the module of its
  file name, for requests inside it (§4, rule 2).
- **No diagnostics.** `in_the_cache` also covers `$NOVA_HOME/std/`, so an
  opened std file gets none.
- **Failure.** With no `NOVA_HOME` and no home directory, or a write
  error, a location in std is left out of the result, and a warning is
  logged once per server.

## 7. The VS Code extension

- **The fixture** gains `src/shapes.nova`, holding a `pub fn area` with a
  `///` doc, and `src/navigate.nova`, which imports `shapes` and calls
  `area`.
- **The smoke test** gains two cases, run on the call in `navigate.nova`
  through `vscode.executeHoverProvider` and
  `vscode.executeDefinitionProvider`:
  - the hover holds `area`'s signature and its doc;
  - the definition is `shapes.nova` at `area`'s name.
- **No other change.** `vscode-languageclient` maps the four capabilities
  itself, so `package.json` and the client stay as they are. The
  extension's README lists the new features.

## 8. Testing

Every change is test first. The suites:

### 8.1 The index (`nova-resolver`, `nova-typeck`, `nova-driver`)

- One test per recording place of §3.2 and §3.3. Each one marks places in
  a small program, and asserts each occurrence's role and target, and its
  target's declaration span.
- Families: an impl method is in its trait method's family, and a
  provided method is declared once.
- Hover's types: a local's type after inference, a closure's local, a
  type parameter printed by its name, an unsolved variable as `_`, and an
  unknown type (§3.4).
- Nothing is recorded for an unresolved name, a placeholder, the
  checker's own names, `_`, or `Self` (§3.5).
- The same name passed twice by the checker (a provided method's
  signature) is recorded once.
- `Analysis.index` is `None` unless asked for.
- A record literal's shorthand gives two occurrences, and `at` picks the
  value.

### 8.2 The index is complete

Two checks run over three corpora:

- **std's 17 files**, through the `<std/…>` files of one analysis, since
  std can be checked only as std;
- **`examples/`'s 6 programs**;
- **`tests/runtime/`:** its top-level programs, except
  `bytes_reserved.nova`, which is meant to fail (133), and the
  `modules/` program as one program, which brings the corpora's only
  imports.

That is 159 files on 2026-10-10. Every program is analysed with `tests`
on, so `@test` functions are checked.

1. **Every use has its declaration:** each `Use` occurrence's target has
   exactly one `Declaration` occurrence, unless it is a `Builtin`,
   `BuiltinMethod`, `Primitive` or `Module`.
2. **Every name is covered:** each `Token::Ident` lies inside an
   occurrence, two for a shorthand. The exceptions are named in the test,
   each with its reason:
   - `_`;
   - attribute names and arguments;
   - a `module m` declaration's name;
   - any construct a later ruling adds.

The test counts its files and asserts the count, so a corpus that shrinks
cannot pass by checking less. `bytes_reserved.nova`'s exclusion is named,
with its reason.

### 8.3 Broken programs

`broken.rs`'s 364 cut programs are analysed with the index on as well:

- no panic and no hang;
- every recorded occurrence's span lies inside its file's text.

### 8.4 The language server (`crates/nova-cli/tests/lsp.rs`)

Over stdio, one test per behaviour:

- **Hover:** each target kind of §5.1's table, with docs and without, and
  a generic function's local.
- **Definition:**
  - each case of §5.2;
  - into another module and into a path dependency;
  - into a downloaded package, using a local index as 3.3b's tests do,
    with its own `NOVA_HOME`;
  - into std.
- **Std's cache:**
  - the first use writes it, and the second reuses it untouched;
  - a damaged file is replaced;
  - its files are read-only;
  - two processes writing it at once both end with the right files;
  - a request inside it is answered;
  - it publishes no diagnostics.
- **References:** with and without the declaration, from the app into a
  dependency, and a trait method's family from each member.
- **Prepare rename:** the range and placeholder, and each refusal of §5.4,
  including `self` and an impl method of a std trait.
- **Rename:**
  - across files;
  - a shorthand field and a shorthand local;
  - a trait method's family;
  - an invalid new name, a keyword and `_`;
  - each of §5.5's three refusals.
- **Robustness:** each request in a file with a syntax error, and a
  request for a document that is not open, which gets an empty result.

### 8.5 Mutants

Each of these breaks its named test:

1. method calls record nothing: §8.2's coverage check;
2. a trait-resolved call records an impl method's `Def`: the trait
   definition test;
3. `implements` is left empty: the family rename test;
4. `at` ignores a cursor at a name's end: its test;
5. rename skips §5.5: the shadowing refusal test;
6. rename does not write out a shorthand: the shorthand field test;
7. references ignore `includeDeclaration`: its test;
8. prepare rename allows a std name: its refusal test;
9. std's cache keeps a damaged file: the damaged-file test;
10. the server's diagnostics check turns the index on: a test that
    `Analysis.index` is `None` for it.

### 8.6 Latency

`latency_on_05_json_api` gains hover, definition, references and rename,
20 of each, in a release build on the development host.

- **Budgets** for the median and the maximum: 200 ms for hover, definition
  and references; 400 ms for rename, which analyses twice.
- **CI's bound** with the debug binary: 2 s, and 4 s for rename.
- **The record:** the figures go into ADR 0032 with each binary's byte
  size and build time.
- **The stop:** if the index pushes completion or diagnostics past ADR
  0029's budget, that is a stop for the user's word (ADR 0029, "`salsa`
  now").

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

- **ADR 0032, "Navigation in the language server":**
  - the index lives in the resolver's types and is recorded by the checker,
    off unless the server asks, which revisits ADR 0029's reason for
    setting a full index aside;
  - rename checks itself by re-analysis;
  - references and rename reach the owning project only;
  - trait method families;
  - std's cache;
  - the measured latency.
- **Dated notes:**
  - `nova-spec/40-TOOLING.md` §3.1, for hover, definition, references and
    rename;
  - `docs/phase-3-plan.md` §4's 3.4 entry, for the split into 3.4a and
    3.4b;
  - `agent.md`.
- **Also:** the CHANGELOG's Unreleased section; the README's editor
  paragraph; `tools/vscode-nova/README.md`; ARCHITECTURE's rows for
  `nova-resolver`, `nova-typeck` and `nova-lsp`.
- **The sweep:** every claim that the server offers only diagnostics,
  completion and formatting, or that hover, definition, references or
  rename are still to come, found by `git grep` across the repository
  minus the files the branch touches.

## 11. Risks

1. **A recording place is missed.** Twelve functions of the checker
   resolve names, and six more make locals. §8.2's coverage check, over
   159 files, fails on any name no occurrence covers, whatever construct
   holds it.
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
6. **Std's cache shared by two servers.** It is new code, written as the
   runtime cache is, and §8.4 tests two processes at once.
7. **Families.** A family is only as complete as `implements`. A missing
   link makes rename fail §5.5 rather than break code, and §8.5's third
   mutant pins the link.

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
5. **At a shorthand, requests act on the value.** `x` in `Point { x }` is
   the value used. The field is reached from its declaration or any other
   use.
6. **A trait-dispatched call goes to the trait's declaration.** The
   checker resolves it as `TraitMethod`, not to one impl. Going to an
   implementation is `textDocument/implementation`, which 3.4a does not
   add.
7. **Rename checks itself by re-analysis** (§5.5), not by rules about
   scopes. The formatter checks its output the same way. One mechanism
   catches shadowing, captures, clashes and duplicates.
8. **Refusals are `RequestFailed` errors with a reason**, from prepare
   rename and from rename. An empty prepare-rename result is kept for "no
   name here".
9. **A new name** must lex to one identifier, and not be `_` or a reserved
   type name. Case conventions are not enforced.
10. **Std's cache is per version and per content**, like the runtime's,
    compared byte for byte on use, and read-only.
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
    repository, and assert its size. `bytes_reserved.nova`, which is meant
    to fail, is left out by name. The `modules/` program is in, for its
    imports.
16. **`at` prefers a name that holds the cursor**, then one that ends at
    it. On a shared span the value wins, then the type, then the trait.
17. **Hover's form** is a fenced `nova` block on one line, then the docs.
    The signature runs from the item's first token to its body, which
    keeps `pub`, `async` and `where`.
18. **One order everywhere:** the file's name in the database, then the
    offset.
19. **Rename's edit uses `changes`**, keyed by the URIs locations use.
20. **`self` is a local, and `Self` is not recorded.** `self` hovers with
    its type and goes to its parameter, and rename refuses it as a
    keyword. Bare `Self` in an impl is an error today, and recording the
    `Self` of `Self::Out` as the impl's type would let rename rewrite it
    into a module-qualified path, which is refused (fact-check).
21. **The index is off for diagnostics and for every CLI command.** So a
    check pays for the index only when the server asks, which answers ADR
    0029's reason for setting a full index aside.
22. **Unresolved names, error-recovery placeholders and the checker's own
    names are not recorded.**
23. **An import list's name records one occurrence per namespace it
    binds**; a glob records only its module, at its first segment's span.
24. **A package import hovers as `package <name> <version>`**, from the
    graph, and goes to its library's first line.
25. **Pattern shorthand and `x @ pat` are not covered.** The checker
    refuses record, tuple, array, or, range and `x @ pat` patterns today.
    The approved design's Section 1 said "Record patterns work the same
    way" and its Section 2 named `x @ pat`. The fact-check found both
    refused.
26. **A trait method and its impls' methods are one family** for
    references and rename. A call on a concrete receiver resolves through
    the trait, so without families an impl method would have no uses, and
    renaming a trait method would leave its impls unmatched, which §5.5
    would always refuse. The approved design did not cover this; the
    fact-check found it.
27. **The index's types live in `nova-resolver`.** `nova-typeck` depends
    on `nova-resolver`, so the resolver can record imports only into a
    type it owns.
28. **Recording happens only at the AST's name sites**, deduplicated by
    (span, role, target). The checker passes some names twice and makes
    names of its own (§2); recording inside `new_local`, `FnCtx::lookup`,
    `place_root` or an `emit_*` function would duplicate them or put them
    at made-up spans, and an LSP client rejects overlapping edits.
29. **Hover prints type parameters by their declared names** and an
    unsolved variable as `_`, not the checker's `T0` and `?n`.
30. **Std's cache is labelled with the server's version.** The CRC-32
    over std's text keeps two builds with the same version apart.
31. **A project file no root reaches is analysed alone**, so its
    references see only what it imports. The compiler does not build such
    a file either.

## 13. Not in 3.4a

- 3.4b: suggested edits on `Diagnostic`, code actions, organize imports and
  semantic tokens.
- Requests the phase plan does not list: `documentHighlight`,
  `typeDefinition`, `implementation`, `documentSymbol` and
  `workspace/symbol`. The index makes each one small later.
- Renaming files, modules or packages.
- Rename and references across a library's dependents (decision 2).
- Hover on expressions that are not names.
- The forms the checker refuses today (§2): type aliases, `import … as`,
  record and other compound patterns.
- Std's `///` docs (3.5 decides).
- `salsa`, unless the budget is missed (ADR 0029).
- Phase 4's rows of `40-TOOLING.md` §3.1: inlay hints, code lens, call
  hierarchy.
