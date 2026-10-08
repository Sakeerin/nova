# Phase 3.2, "LSP core and the VSCode extension": design

- **Date:** 2026-10-08.
- **Status:** the design was approved in conversation on 2026-10-08, in five
  sections. A fact-check followed, and this spec includes its corrections,
  with one more decision by the user (§13, decision 2). This written spec
  awaits the user's review.
- **Branch:** `phase-3-2-lsp-core`, cut from `main` at `d045d05`.
- **Inputs:**
  - `docs/phase-3-plan.md` §1, §2 items 6 and 7, §3 decisions 7 and 8, §4's
    3.2 entry, §5 and §6 risk 2;
  - `nova-spec/40-TOOLING.md` §3;
  - ADR 0025, ADR 0026 and ADR 0028;
  - the user's answers of 2026-10-08 (§13).

## 1. What 3.2 delivers

- **The front end keeps going past errors** for the language server.
  - `nova_driver::analyze` reads sources through a provider, so an editor's
    unsaved buffers are checked.
  - With `keep_going`, it runs resolution and type checking on a program
    with parse or resolution errors, and returns every diagnostic and the
    partial typed module.
  - `nova check`, `nova build`, `nova run` and `nova test` behave exactly as
    they do today (§13, decision 1).
- **What is at the cursor:**
  - the parser keeps an unfinished `foo.`;
  - the type checker takes a probe offset and records three things there:
    the receiver's type and members, and the locals in scope;
  - `names_in_scope` lists what a module sees.
- **Positions:** `LineIndex` converts between byte offsets and LSP's line and
  UTF-16 column.
- **`nova lsp`** speaks the Language Server Protocol over stdio, with
  `lsp-server` 0.7.8 and `lsp-types` 0.97 (§13, decision 2). It offers:
  - diagnostics for whole projects, re-checked on every edit and on every
    watched change on disk;
  - completion: members after `.`, and elsewhere names in scope, primitive
    types and keywords;
  - document formatting through `nova-fmt`.
- **`tools/vscode-nova/`:**
  - the language, its TextMate grammar and its language configuration;
  - a TypeScript client that runs the installed `nova lsp` (§13, decision 5);
  - a smoke test.
- **CI** builds and tests the extension, and `release.yml` attaches its
  `.vsix` to a tagged release.
- **Records:** ADR 0029, "The language server", and the updates of §11.

**The gate** (`docs/phase-3-plan.md` §4, made concrete):

1. A Rust test drives `nova lsp` over stdio and gets:
   - the diagnostic for a planted type error (E0010), with its exact UTF-16
     range;
   - completions in a file that also has a syntax error;
   - completions that include a std method (`push` on a `Vec`) and a record
     field;
   - a formatted document.
2. The extension's smoke test passes in CI.
3. On the development host, with a release build, `05-json-api` meets the
   200 ms budget of §6.8 for an edit to its diagnostics and for a completion
   request to its answer. The figures are recorded in ADR 0029 and in
   `docs/benchmarks/README.md`. CI asserts §6.8's looser bound with the
   debug binary.
4. If the budget is missed, adopting `salsa` becomes its own step, decided
   with the user.

## 2. The starting point (read on 2026-10-08)

### The driver (`crates/nova-driver/src/lib.rs`)

- **It stops at the first failing stage:**
  - after lexing and parsing every module (`:582-584`);
  - after resolution (`:635-637`);
  - after type checking (`:643-645`).
- **It prints diagnostics and returns only their count.** `render` writes to
  stderr (`:465-473`).
- **It reads source from disk in two places:** an existence probe on the
  entry (`:457`), and `std::fs::read_to_string` for every module (`:502`).
- **It strips every `@test` function before resolution** unless it is
  building tests (`:600-602`). `nova new`'s template puts a `@test` in
  `src/main.nova` (`crates/nova-cli/src/template.rs:14`).
- **Every public entry point takes a path and reads it from disk:**

  | Entry point | Line |
  |---|---|
  | `check_file(path)` | `:43` |
  | `compile_file(path)` | `:59` |
  | `build_file(path, output)` | `:93` |
  | `build_file_release(path, output)` | `:134` |
  | `build_test_binary(path)` | `:197` |
  | `run_file(path, args)` | `:437` |

- **`check_file` also lowers to MIR** (`:48`). MIR lowering starts from
  `main` (`crates/nova-mir/src/mono.rs`):
  - E0601 for a missing `main` has no label (`:20-23`), and lowering stops
    there (`:24`);
  - a generic `main` gets an E0601 with a label (`:28-29`);
  - MIR's other checks are E0075 (`:65`), E0011 (`:108`), E0013 (`:137`),
    E0078 (`:194`) and E0079 (`:210`).

### The parser (`crates/nova-parser/src/grammar.rs`)

- **It recovers.** It resynchronises at item boundaries (`:143-162`) and
  statement boundaries (`:165-178`).
- **It always returns `Some(File)`, with its errors** (`:281-288`).
  `parse`'s signature is `(Option<File>, Vec<ParseError>)`
  (`crates/nova-parser/src/lib.rs:35`).
- **The AST has no error node.** A construct that fails is dropped whole. A
  function whose body fails is lost (`:488-499`). An unfinished `foo.` fails
  at `parse_ident("field access")?` (`:1812`), so its statement is lost,
  `foo` included. `Expr::Field` is `{ target, field: Spanned<String> }`
  (`crates/nova-ast/src/expr.rs:97-100`).

### The resolver (`crates/nova-resolver/src/lib.rs`)

- **It resolves item-level names only.** The type checker resolves local
  names.
- **Each module has a private `ModuleScope`** (`:1223-1228`). It maps a value
  name to a `Res` (`:1225`): a definition, a variant `(DefId, index)`, or a
  `Builtin` (`:1206-1213`). It can be queried only by exact name.
- **Builtins:** `println`, `print`, `eprint`, `eprintln` and `panic` are
  seeded into every scope as builtins (`:1002-1008`, `:1528-1530`).
- **Primitive types:** their names (`RESERVED_TYPE_NAMES`, `:1141`) are in no
  namespace.
- **Std is lexed and parsed on every call:** the loop at `:1478-1486` runs
  `std_module` (`:1644-1657`) on all 16 modules.
- **Std's public names are glob-imported into every module**, last, as the
  lowest-priority binding (`:1583-1593`).
- **A list import of a missing name is E0001** (`:2322-2327`).

### The type checker (`crates/nova-typeck/src/`)

- **It continues past errors.** It uses `Ty::Error`
  (`crates/nova-hir/src/lib.rs:110-111`) and checks every function
  (`check.rs:170-188`). The typed module it returns is partial when errors
  were found (`lib.rs:27-32`).
- **It keeps no position-to-type table.** Local scopes are pushed and popped
  as checking goes, and are not kept.
- **A failed field or method lookup discards the receiver's checked
  expression** (`check.rs:5060-5068`, `:5422-5432`).
- **Member lookups:**
  - They are private methods of `Checker`: `record_field_index_and_ty`
    (`:5111`), `resolve_method_on` (`:5144`), `trait_method_index`
    (`:5193`) and `find_inherent_method` (`:5215`).
  - A type parameter's bounds live in the function's context
    (`FnCtx::param_bounds`, `:282`, used at `:5147`).
  - An array's `.len()` is a special case (`:5377-5386`).
  - Whether an inherent method takes `self` is known only to the private
    `selfless` set (`:217`).
  - `hir::RecordField` holds a name and a type (`hir lib.rs:920-923`), and
    `ImplInfo.methods` holds `(String, DefId)` pairs (`:876`).
- **"cannot find" is E0001,** at every site: a value (`check.rs:3502`), a
  function call (`:3549`), a record (`:4817`), a type (`:2493`), and a trait
  (`:650`, `:1145`, `:2221`, `:2274`). E0010 is a type mismatch (`:6722-6725`).
- **`display_ty`** is public. It takes `&Definitions`, and prints type
  parameters as `T0` and up (`lib.rs:35`, `:67`).
- **Visibility:** record fields have a parsed visibility
  (`crates/nova-ast/src/item.rs:83`; `nova-spec/11-PARSER.md:61`) that
  nothing reads.

### Positions

- Spans are byte offsets (`crates/nova-diagnostics/src/lib.rs:15-20`).
- `FileDb` only grows, and its unused `location()` gives byte columns
  (`files.rs:65-75`).
- Nothing in the compiler converts to UTF-16.

### Std and speed

Std is 6,141 lines, or 6,116 without `std/test`. It is re-parsed and
re-checked on every compile, and nothing in the front end is cached. Each
program was timed seven times with `nova check`:

| Program | Build | Time |
|---|---|---|
| `01-hello-world` | release | 107-115 ms |
| `05-json-api` | release | 108-127 ms |
| `tests/runtime/iterator.nova` (361 lines, the largest test program) | release | 108-126 ms |
| `05-json-api` | debug | 144-162 ms |

- The release binary was 12,900,352 bytes, built at 09:28:36. The debug
  binary was 18,010,112 bytes, built at 09:00:56.
- Each set had at most one cold-start outlier (1,291 ms and 4,027 ms), left
  out above.
- Std dominates every check.

### Tooling

- `nova-lsp` is a one-line stub that nothing depends on.
- **`Cargo.lock`:**
  - it has no `lsp-server`, `lsp-types` or `crossbeam-channel`;
  - `serde` is a direct dependency of `nova-ast` and `nova-diagnostics`;
  - `serde_json` arrives only through `criterion`;
  - both are declared in `[workspace.dependencies]` (`Cargo.toml:44-45`).
- **Logs:** `nova-cli` installs a `tracing_subscriber` that writes to stdout
  and also forwards `log` records (`crates/nova-cli/src/main.rs:54-59`).
- **`nova-fmt`:**
  - `format_file` reads from disk (`crates/nova-fmt/src/file.rs:69-70`);
  - `format_text` ignores `.editorconfig` (`:91-96`);
  - a source with a syntax error is refused (`source.rs:37-50`).
- **No tracked file is a `nova.toml`.** The examples and `tests/runtime`
  are loose files.
- **CI** sets up no Node and no display.
- **`release.yml`:**
  - `prepare` downloads the artifacts named `archive-*` (`:99`);
  - it fails unless `release/dist` holds exactly 4 files (`:107-112`);
  - its pull-request `paths` filter lists the build and release paths
    (`:12-20`).
- **The development host** has Node 22.16.0, npm 11.6.1 and VS Code 1.140.0.

### On the registries, read 2026-10-08

- **`lsp-server`:**
  - 0.7.9 (2025-08-06) and every later release, 0.10.0 (2026-07-16)
    included, use `edition = "2024"`, which needs Rust 1.85;
  - 0.7.8 (2024-12-20) is edition 2021, and depends on `crossbeam-channel`,
    `log`, `serde`, `serde_derive` and `serde_json`.
- **`lsp-types` 0.97.0** (2024-06-04) is edition 2018, and depends on
  `bitflags` 1, `fluent-uri` 0.1, `serde`, `serde_json` and `serde_repr`.
- **Every crate these pull in** is edition 2021 or earlier, and declares a
  minimum Rust of at most 1.71, or none.
- **`vscode-languageclient` 10.1.2** (2026-09-25) requires VS Code
  `^1.91.0`.

## 3. The front end on broken code

### 3.1 `analyze`

```rust
/// Where `analyze` reads a module's text.
pub trait Sources {
    /// The text of `path`: an editor's buffer if one is open, else the file.
    fn read(&self, path: &Path) -> std::io::Result<String>;
}

pub struct Options {
    /// Run every stage whatever the earlier ones found (§3.2).
    pub keep_going: bool,
    /// Keep `@test` functions and put `std/test` in scope, as `nova test`
    /// does (§13, decision 14).
    pub tests: bool,
    /// Check `entry` as a module, not a program: no MIR lowering (§3.3).
    pub module_only: bool,
    /// Record what is at this place (§4.1).
    pub probe: Option<Probe>,
}

pub struct Analysis {
    pub db: FileDb,
    pub diagnostics: Vec<Diagnostic>,
    /// The program's own modules, by the paths `analyze` read them from.
    pub modules: Vec<(FileId, PathBuf)>,
    pub definitions: Option<Definitions>,
    /// The typed module, partial when errors were found.
    pub module: Option<nova_hir::Module>,
    pub probe: Option<ProbeResult>,
}

pub fn analyze(entry: &Path, sources: &dyn Sources, options: &Options) -> Analysis;
```

- Both of the driver's disk reads go through `Sources`. `DiskSources` reads
  the disk.
- **Every existing entry point is unchanged in behaviour:** `check_file`,
  `compile_file`, `build_file`, `build_file_release`, `build_test_binary`,
  `run_file`, and with them `nova test`. Each reads through `DiskSources`,
  stops at the first failing stage, and prints what it prints today. The
  existing CLI tests pin this, and one more pins it on a file that has both
  a syntax error and a type error (§9.1).
- **Paths:** names in `FileDb` stay the paths the driver builds (`lib.rs:510`,
  `:545`). A `Sources` implementation maps them to buffers (§6.9).

### 3.2 `keep_going`

- Every module is lexed and parsed, and every diagnostic is kept.
- Resolution runs even when parsing found errors.
- Type checking runs even when resolution found errors.
- The typed module is returned whatever its errors.

**Cascades:**
- **A member access with a missing name** (§3.4) gets no diagnostic beyond
  the parser's P0001.
- **A dropped item's name** (`fn f(x: Int {`): when the parser drops an item
  after reading its name, it reports that name. A use of a dropped name is
  typed `Ty::Error`, and gets no E0001 at any of §2's E0001 sites, the
  resolver's list imports included. A function being edited therefore does
  not turn its callers red. `parse`'s signature and the AST's shape do not
  change; the plan chooses how the names travel.
- **`Ty::Error`** already keeps type errors from cascading.

### 3.3 MIR lowering

- **Lowering runs only when no stage before it found an error**, because it
  assumes a well-formed program; `check_file` does the same. So for clean
  code the editor shows what `nova check` shows (with `tests`, what
  `nova test` would check), and for broken code it shows more.
- **With `module_only`, lowering does not run at all.** It starts from
  `main`, and a module has none (§2). The server sets `module_only` in two
  cases (§6.2):
  - a project file that the project's entry does not reach;
  - a loose file that declares no top-level `fn main`.

### 3.4 The unfinished member access

When a `.` is followed by something other than a name, the parser:
1. reports P0001, as today;
2. keeps the receiver as `Expr::Field` with an empty name spanning the gap at
   the `.`'s end.

The statement survives. Only `keep_going` ever type-checks such a node:
`nova check` stops at parse errors, and `nova fmt` refuses a source with a
syntax error (`crates/nova-fmt/src/source.rs:37-50`).

## 4. What is at the cursor

### 4.1 The probe

`Probe { path: PathBuf, offset: u32 }`. As the type checker checks the
function whose body holds the offset, it fills a `ProbeResult`:

- **`receiver: Option<Ty>`:** at a member access whose name touches the
  offset, the empty name of §3.4 included. The type is read after inference
  has finished that function.
- **`members: Vec<Member>`:** the receiver's members (§4.2), computed by the
  checker at that point, where the function's bounds and the program's
  declarations are in reach.
- **`locals: Vec<(String, Ty)>`:** at a name expression, or the empty name,
  touching the offset. These are the locals in scope there, innermost first,
  each name once.

If the offset is in no function body, all three are empty. A probe costs
nothing when absent.

### 4.2 Members

Each `Member` has a `name`, a `kind` (`Field` or `Method`), and a `detail`.
The detail is the declaration as written, on one line: a field's type, or a
method's signature from `fn` to its body. It is read from the source, std's
included, so parameter names and type parameter names appear as declared
(§13, decision 16). The members are:

- the record's fields;
- its inherent methods that take `self`;
- the methods of every trait the type implements;
- for a type parameter, the methods of its bounds;
- for an array, `len`.

They cover std's types, since std is part of every program. `Ty::Error`
gives none.

**Visibility:** a field or inherent method declared without `pub` is listed
only inside the module that declares it, read from its declaration. This
hides std's internals, such as `Vec`'s `data`, which the checker does not
stop anyone using (§2; §13, decision 7). Trait methods are always listed.

**Shared code:** the lookups are the checker's own, so completion and
checking cannot disagree.

### 4.3 Names in scope, primitive types and keywords

- **`Definitions::names_in_scope(module) -> Vec<(String, ScopeEntry)>`.** A
  `ScopeEntry` is an item (`DefId`), a variant (`DefId` and index), or a
  builtin. The list holds:
  - the module's items;
  - its imports;
  - the builtins;
  - std's glob-imported names.

  A std name the module shadows is listed once, as the module's own.
- **Primitive types:** `RESERVED_TYPE_NAMES` becomes public, so completion
  can offer `Int`, `String` and the rest.
- **Keywords:** `nova_lexer::KEYWORDS` holds every alphabetic keyword token
  in the lexer's `Token` enum. A test lexes each one and checks that it comes
  out as its token, not as an identifier, and that no alphabetic token is
  missing.

## 5. Positions

`nova_diagnostics::LineIndex::new(text)` converts in both directions:

- **To LSP:** `offset → (line, utf16_column)`.
  - A character above U+FFFF counts as two UTF-16 units.
  - `\r\n` and `\n` both end a line, and the `\r` belongs to the line's end.
- **From LSP:** `(line, utf16_column) → offset`.
  - A column past the line's end clamps to the end.
  - A line past the last clamps to the end of the text.
  - A column inside a surrogate pair clamps to the character's start.

## 6. `nova lsp`

### 6.1 Protocol

- `nova lsp` is a new `nova-cli` subcommand that calls `nova_lsp::run`.
- It speaks JSON-RPC over stdio through `lsp-server`, using `lsp-types`.
- **Logging:** under `nova lsp`, the `tracing` subscriber writes to stderr,
  and so do the `log` records it forwards, `lsp-server`'s included. Nothing
  but protocol messages reaches stdout. The other commands keep their
  subscriber as it is (§2).
- **It advertises:**
  - position encoding UTF-16;
  - text sync `Full` (open, change, save, close);
  - completion, with trigger character `.`;
  - document formatting.
- **File watching:** if the client supports dynamic registration of
  `workspace/didChangeWatchedFiles`, the server registers `**/*.nova` and
  `**/nova.toml`. VS Code supports it. The watcher is the client's (§13,
  decision 9).
- **Shutdown:** `shutdown` and `exit` follow the protocol. `exit` without
  `shutdown` exits with code 1. An unknown request gets `MethodNotFound`.

### 6.2 Projects, and which analysis owns a file

- **A file's project** is found with `nova_pm::find_root`:
  - With a `nova.toml`, the entry is the root's `src/main.nova`, as
    `nova run` picks it.
  - With none, the file is loose, and is its own entry, as with
    `nova check FILE`.
- **When a project becomes active:** when one of its files is opened. It is
  checked at once, then again on each of these:
  - an edit to one of its buffers;
  - a save;
  - a close (the file is then read from disk);
  - a watched change to a `.nova` file or a `nova.toml` under its root.
- **When a project goes inactive:** when its last open file closes. Its
  diagnostics are cleared.
- **Ownership:** each file's diagnostics come from exactly one analysis.
  - **A project's analysis** owns every module its entry reaches.
  - **An open project file the entry does not reach** gets an analysis of its
    own, with `module_only`, which owns that file alone.
  - **A loose file's analysis** owns that file alone. It is checked with
    `module_only` when the file declares no top-level `fn main`.

  So `tests/runtime/modules/geometry.nova`, opened beside its `main.nova`,
  gets no false E0601. The two analyses never publish for the same file.
- **Every analysis** runs with `keep_going` and `tests` (§13, decision 14).

### 6.3 Diagnostics

- **What is published:** after each check, diagnostics for every file the
  analysis owns. An open file's diagnostics carry its buffer's version. A
  file that had diagnostics and now has none gets an empty list.
- **The mapping:**

  | From a Nova `Diagnostic` | To the LSP diagnostic |
  |---|---|
  | Severity: Error, Warning, Note, Help | Error, Warning, Information, Hint |
  | `code` | The diagnostic's code |
  | The primary label | The range |
  | Notes | Appended to the message, one per line |
  | Secondary labels | Related information |

- **The fallback:** a diagnostic with no label, or whose labels all lie in
  std, goes on the analysis's entry file, on its first line. Its message
  names the std location, for example `<std/core>:12:5`.

### 6.4 Completion

Completion runs on the main thread. It is one `analyze` of the file's owning
analysis (§6.2), over the current buffers, with the probe at the cursor.

- **Inside a string or a comment,** found by lexing the buffer, the answer is
  an empty list.
- **After `.`:** the probe's `members`. Fields get kind `Field`, and methods
  get kind `Method`, each with its `detail`.
- **Elsewhere:**
  - the probe's `locals`, kind `Variable`;
  - `names_in_scope` for the cursor's module: functions and builtins as
    `Function`, records as `Struct`, sum types as `Enum`, variants as
    `EnumMember`, traits as `Interface`, and constants as `Constant`;
  - primitive types as `Struct`;
  - keywords as `Keyword`.
- **Every candidate is returned** (`isIncomplete: false`), and the editor
  filters by what has been typed.

### 6.5 Formatting

`textDocument/formatting` formats the buffer with a new
`nova_fmt::format_buffer(path, text)`. It applies the `.editorconfig` that
governs `path`, as `format_file` does, to the given text.

- **The answer** is one edit replacing the whole document, or no edit if the
  buffer is already formatted.
- **If the buffer has a syntax error,** or the self-check refuses the output,
  the answer is no edit, and the reason goes to the log. Format-on-save never
  raises an error.

### 6.6 Threads and stale results

- **The main thread** holds the open buffers (URI → version and text), and
  answers completion and formatting.
- **The checker thread** checks snapshots. A snapshot holds:
  - the analysis to run;
  - a copy of its buffers;
  - a generation number that rises with every change.
- **Queued snapshots** of the same analysis collapse to the latest.
- **A finished check** whose generation is older than the newest snapshot of
  its analysis is not published.
- **There is no debounce timer** and no cancellation. An edit that arrives
  during a check waits for that check to finish, then for its own.

### 6.7 Robustness

- Each `analyze` runs under `catch_unwind`. After a panic in the front end:
  - the panic is logged;
  - the previous diagnostics stay;
  - a completion answers with an empty list;
  - the server keeps running.
- §9.2's test feeds broken programs to `keep_going` on purpose.

### 6.8 The budget

**The budget:** 200 ms (§13, decision 3). It holds for both the median and
the maximum of 20 of each of these:
- from a `didChange` sent while no check is running, to the
  `publishDiagnostics` for that version;
- from a completion request to its answer.

It is measured on `examples/05-json-api/src/main.nova`, with a release build,
on the development host (§9.5).

**CI's bound:** 2 s for each, with the debug binary (§9.3).

An edit made during a check waits for it (§6.6), so a burst of typing can
take up to about two checks, roughly 216-254 ms by §2's figures. The burst
test checks correctness, not time.

### 6.9 URIs and paths

- **URIs:** only `file:` URIs map to paths. Percent-encoding is decoded, so
  `file:///d%3A/x/y.nova` becomes `d:\x\y.nova`.
- **Untitled buffers:** an untitled buffer is checked as a loose file.
- **Matching paths:** buffers and the driver's module paths are compared
  after normalisation:
  - the directory is canonicalised when it exists;
  - on Windows, the drive letter's case is ignored.

  So a buffer always shadows the file the driver would have read.

## 7. The VS Code extension

### 7.1 Files

`tools/vscode-nova/` holds:
- `package.json`, `package-lock.json`, `tsconfig.json` and `src/extension.ts`;
- `syntaxes/nova.tmLanguage.json` and `language-configuration.json`;
- `test/`;
- `README.md`, the two licences, and `.vscodeignore`.

### 7.2 `package.json`

- **The language:** id `nova` for `.nova` files, with the grammar
  `source.nova`.
- **The language configuration:** `//` and `/* */` comments, brackets,
  auto-closing pairs, and an indent after `{`.
- **Settings:**
  - `nova.server.path`: a string, empty by default, meaning `nova` from
    PATH;
  - `nova.trace.server`: `off`, `messages` or `verbose`.
- **One command:** "Nova: Restart Language Server".
- **`name`** is `nova`.
- **`engines.vscode`** is `^1.91.0`, the floor `vscode-languageclient` 10
  needs.
- **`version`** equals `nova-cli`'s version, and a test checks that.
- **`publisher`** is `sakeerin`, provisional until the user creates the
  Marketplace publisher at `v0.3.0` (§13, decision 6).

### 7.3 The client

- It starts `nova lsp` from `nova.server.path`, or else from PATH. It covers
  saved and untitled documents of language `nova`.
- If the server cannot be spawned, it shows an error that names the setting
  and the install command,
  `cargo install --locked --git https://github.com/Sakeerin/nova nova-cli`.

### 7.4 The grammar

It colours:
- line and block comments, with `///` scoped as documentation;
- strings, with escapes and `${…}` interpolation;
- characters and numbers;
- the keywords and `@attributes`;
- capitalised type names, and the name after `fn`.

A Rust test reads the grammar's JSON and checks that its keyword list equals
`nova_lexer::KEYWORDS`.

### 7.5 Build and dependencies

- **Build:** `npm ci`, then `tsc`, with no bundler.
- **Packaging:** `vsce package --out nova-vscode-<version>.vsix` packages
  `vscode-languageclient`'s production dependencies with it.
- **The dependencies:**
  - runtime: `vscode-languageclient` ^10.1;
  - development: `typescript`, `@types/vscode`, `@types/node`,
    `@vscode/vsce`, `@vscode/test-electron`, `mocha` and `@types/mocha`.
- `package-lock.json` is committed.
- `.vscode-test/`, `node_modules/` and `out/` are git-ignored.

### 7.6 The smoke test

It uses `@vscode/test-electron` with a pinned VS Code version. It opens a
fixture project with `nova.server.path` pointing at the freshly built
`target/debug/nova`, and checks that:
- a `.nova` file gets the language `nova`;
- the planted error's diagnostic arrives;
- completion after `.` offers a record field;
- Format Document gives the formatted text.

## 8. CI and release

**`ci.yml`** gains a job, "VS Code extension", on ubuntu-latest:
1. `cargo build --locked -p nova-cli`;
2. `actions/setup-node`, with Node 22;
3. `npm ci` and the compile;
4. `xvfb-run -a npm test`;
5. the `.vsix` package, uploaded as an artifact.

Linux suffices: the extension is platform-neutral, and the Rust gate test
(§9.3) runs the server on all three systems.

**`release.yml`:**
- A new job builds the `.vsix`, and uploads it as the artifact
  `archive-vsix`, so `prepare`'s `archive-*` download takes it.
- `prepare` requires exactly 4 `nova-*` archives and 1 `nova-vscode-*.vsix`
  in `release/dist`.
- `SHA256SUMS` covers the `.vsix`, and `publish` attaches it to the tagged
  release.
- The pull-request `paths` filter gains `tools/vscode-nova/**`.
- Nothing is published to an extension store in 3.2.

## 9. Testing

### 9.1 Unit tests

- **`LineIndex`:** ASCII, Thai, an emoji, CRLF, the three clamps, and round
  trips.
- **Parser:**
  - `foo.` keeps its receiver, with P0001 reported;
  - a dropped item's name is reported;
  - every existing parser test passes unchanged.
- **Driver:**
  - `keep_going` reports a syntax error in `f` and a type error in `g` in one
    analysis, while `check_file` on the same file reports only the syntax
    error;
  - a buffer beats the file on disk;
  - a never-saved buffer is checked;
  - `module_only` skips MIR lowering;
  - MIR's diagnostics appear when the program is otherwise clean;
  - with `tests`, a `@test` body's type error is reported;
  - a dropped name causes no E0001, from the checker or from a list import.
- **Type checker:** the probe gives
  - `receiver` and `members` for a record, `String`, a std `Vec`, an array,
    a bounded type parameter, and the result of a method chain;
  - no-`self` functions excluded;
  - the visibility rule (§4.2);
  - `detail` as declared;
  - `locals`, shadowing included.
- **Resolver:** `names_in_scope` lists the module's items, its imports, the
  builtins, the variants with their index, and std's names, with a shadowed
  std name listed once.
- **Lexer:** `KEYWORDS` (§4.3).
- **Formatter:** `format_buffer` applies `end_of_line = crlf` and
  `insert_final_newline = false` from an `.editorconfig` beside the path.
- **Server:**
  - URI to path and back, Windows and Unix, encoded drive letters included;
  - generation dropping;
  - the ownership rule (§6.2);
  - the diagnostic mapping and its fallback.

### 9.2 Broken programs never panic or hang

`analyze`, with `keep_going` and `tests`, runs on two sets of programs:
- each example, cut at 16 evenly spaced points;
- each of the 134 `tests/runtime/*.nova`, cut at 2.

Each analysis must finish within 10 s and must not panic.
- Cuts fall on character boundaries, since 20 of the runtime programs hold
  non-ASCII text.
- The 364 analyses are spread across threads.
- A floor on the number checked keeps the test from passing vacuously.

### 9.3 The gate test

`crates/nova-cli/tests/lsp.rs` drives `nova lsp`. Its client frames JSON-RPC
over the child's stdin and stdout, reads on a thread with deadlines, and
kills the child on drop. It checks:

1. the planted E0010, with its exact UTF-16 range, on a line that holds
   non-ASCII text before it;
2. completions in a file that also has a syntax error;
3. completions that hold `push`, a std method on a `Vec`, and a record
   field;
4. the formatted document;
5. a module fixed on disk clears its diagnostic once its watched-file event
   arrives;
6. a burst of edits ends with the last edit's diagnostics;
7. `shutdown` then `exit`, and `exit` alone giving code 1;
8. §6.8's CI bound of 2 s, with the debug binary, for an edit and for a
   completion.

### 9.4 Mutants

Each of these fails at least one named test:
- UTF-16 columns counted in bytes;
- `keep_going` stopping after parse errors;
- a probe that records nothing;
- the generation check removed;
- `format_buffer` ignoring `.editorconfig`;
- no-`self` functions offered after `.`;
- `Sources` bypassed for the disk;
- `tests` ignored, so `@test` bodies are stripped.

### 9.5 Latency on the development host

An ignored test times 20 edits, each sent while no check runs, and 20
completions on `05-json-api`. It prints the median and the maximum of each.

- **Its bound depends on the build.** In a release build it asserts §6.8's
  200 ms. In a debug build it asserts only the 2 s CI bound, because CI's
  advisory `--ignored` step runs it there on all three systems, adding one
  test name to that step's list (`.github/workflows/ci.yml:80-82`).
- **Its record:** the figures go into ADR 0029 and `docs/benchmarks/README.md`,
  with the binary's size and build time recorded beside them.

### 9.6 The extension

- §7.6's smoke test, in CI.
- §7.4's keyword test, in the Rust suite.

## 10. What 3.2 asks of the user during implementation

- **`npm ci` on the development host** downloads the npm packages of §7.5.
  The smoke test downloads a VS Code build of about 150 MB into the
  git-ignored `.vscode-test/`. Either one is run locally only with the
  user's word; CI runs both on every PR regardless.
- **The merge**, by rebase, on the user's word.

## 11. Records

- **ADR 0029, "The language server":**
  - `lsp-server` 0.7.8, pinned as `=0.7.8`, and `lsp-types` 0.97, rather than
    `tower-lsp` (`40-TOOLING.md` §3.2; the master spec's dependency list),
    with why the pin keeps the 1.78 MSRV;
  - re-checking rather than `salsa`, with the budget and its measured
    figures;
  - `keep_going` and `tests` for the server only;
  - the probe;
  - ownership of a file's diagnostics;
  - the installed `nova` as the server;
  - full text sync;
  - the client's file watcher.
- **Dated notes:**
  - `nova-spec/40-TOOLING.md` §3.1, for the rows 3.2 delivers (diagnostics,
    completion, format on save);
  - `nova-spec/40-TOOLING.md` §3.2, for the stack;
  - `nova-spec/40-TOOLING.md` §3.3, for `tools/vscode-nova/`;
  - `nova-spec/11-PARSER.md`, for the unfinished member access and the
    dropped-item names;
  - `nova-spec/00-MASTER-SPEC.md`, at its `tower-lsp` lines (`:632`, `:804`,
    `:832`).
- **Other updates:**
  - `docs/benchmarks/README.md`, the latency figures (§9.5);
  - `docs/phase-3-plan.md`, a dated note at §3 decision 7, where the
    recommendation of `lsp-server` rested on 0.10.0, and its 3.2 spec line;
  - `CHANGELOG.md` `[Unreleased]`;
  - `ARCHITECTURE.md`'s `nova-lsp` row;
  - an "Editor support" section in `README.md`;
  - the set-difference sweep over the living documents.

## 12. Risks

1. **The front end may panic or hang on half-typed code,** because it was
   written for finished programs. Mitigated by `catch_unwind` (§6.7), and by
   the cut programs with their timeout (§9.2).
2. **Cascading diagnostics.** Mitigated by the dropped-name rule (§3.2) and
   by `Ty::Error`. The tests assert exact diagnostics, so a new cascade
   fails one.
3. **Windows URIs** (`%3A`, the drive letter's case) may not match the
   driver's paths, so a buffer is missed. Mitigated by normalisation (§6.9)
   and by tests with encoded URIs.
4. **A flaky VS Code test in CI.** Mitigated by a pinned VS Code version, and
   by running one job on one system.
5. **`lsp-server` is pinned at a release from 2024-12, and `lsp-types` has
   had none since 2024-06.**
   - LSP 3.17, which `lsp-types` covers, is stable.
   - Both crates are small enough to vendor.
   - The pin lifts when the MSRV next moves.
6. **The budget's headroom is 73-92 ms today.** Bursts of typing can exceed
   it (§6.8), and 3.4 adds work to the same engine. A later miss is its own
   step with the user (§1, gate 4).

## 13. Decisions

**Made by the user on 2026-10-08:**
1. **Keep going for the LSP only.** The CLI keeps today's staged output.
2. **`lsp-server` and `lsp-types`, with `lsp-server` pinned at `=0.7.8`.**
   The fact-check found that 0.7.9 and later need Rust 1.85 (§2), and the
   user chose to keep the 1.78 MSRV rather than raise it.
3. **The budget is 200 ms** for an edit to its diagnostics and for a
   completion request. CI asserts 2 s with the debug binary.
4. **Recovery plus a probe** (§3.4, §4.1), rather than a placeholder name
   written at the cursor, or a full position index.
5. **The extension runs the installed `nova`,** from `nova.server.path` or
   PATH, with one platform-neutral `.vsix`.
6. **`publisher` is `sakeerin` for now,** settled at `v0.3.0`.

**Made while writing this spec:**

7. **Completion follows declared field and method visibility** (§4.2), though
   the checker enforces none. That is what a reader of the declarations
   expects, and it hides std's internals.
8. **Std has no prelude** (§2). Completion offers its glob-imported names
   with the builtins.
9. **The client watches files,** through dynamic registration, so no
   file-watching crate is added.
10. **Full text sync.** Nova files are small, and a whole-document update
    keeps the server simple.
11. **No debounce and no cancellation.** Collapsing queued snapshots (§6.6)
    keeps the checker on the latest text without a timer.
12. **Formatting a broken buffer returns no edit** (§6.5), rather than an
    error that format-on-save would raise on every save.
13. **A file is checked as a module** when its project's entry does not reach
    it, or when it is a loose file with no `fn main` (§3.3, §6.2). Such a
    file gets diagnostics without a false E0601.
14. **The server checks as `nova test` does:**
    - `@test` bodies get diagnostics and completion;
    - `std/test`'s names are in scope everywhere, which `nova build` would
      reject outside a test.
15. **Each file's diagnostics have one owning analysis** (§6.2).
16. **A member's detail is its declaration as written** (§4.2), not
    `display_ty`'s `T0` form.

## 14. Not in 3.2

- Hover, go to definition, references, rename, code actions and semantic
  tokens (3.4).
- Incremental sync, range formatting, signature help, workspace symbols and
  inlay hints.
- The Zed and Neovim extensions (ADR 0026).
- `salsa`, unless the budget is missed (§1, gate 4).
- Publishing to the VS Code Marketplace or Open VSX (`v0.3.0`).
- A `nova` bundled inside the extension.
- Caching std between checks, and cancelling a check.
