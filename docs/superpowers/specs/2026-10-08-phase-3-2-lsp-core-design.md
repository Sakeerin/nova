# Phase 3.2, "LSP core and the VSCode extension": design

- **Date:** 2026-10-08.
- **Status:** the design was approved in conversation on 2026-10-08, in five
  sections. This written spec awaits the user's review.
- **Branch:** `phase-3-2-lsp-core`, cut from `main` at `d045d05`.
- **Inputs:**
  - `docs/phase-3-plan.md`: §1, §2 items 6 and 7, §3 decisions 7 and 8, §4's
    3.2 entry, §5 and §6 risk 2;
  - `nova-spec/40-TOOLING.md` §3;
  - ADR 0025, ADR 0026 and ADR 0028;
  - the user's answers of 2026-10-08 (§13).

## 1. What 3.2 delivers

- **The front end keeps going past errors** for the language server.
  - `nova_driver::analyze` reads sources through a provider, so an editor's
    unsaved buffers are checked.
  - With `keep_going`, it runs resolution and type checking on a program with
    parse or resolution errors, and returns every diagnostic and the partial
    typed module.
  - `nova check`, `nova build`, `nova run` and `nova test` behave exactly as
    they do today (§13, decision 1).
- **What is at the cursor:**
  - the parser keeps an unfinished `foo.`;
  - the type checker takes a probe offset and records the receiver's type and
    the locals in scope there;
  - `members` lists a type's fields and methods;
  - `names_in_scope` lists what a module sees.
- **Positions:** `LineIndex` converts between byte offsets and LSP's line and
  UTF-16 column.
- **`nova lsp`** speaks the Language Server Protocol over stdio, with
  `lsp-server` and `lsp-types` (§13, decision 2). It offers:
  - diagnostics for whole projects, re-checked on every edit and on every
    watched change on disk;
  - completion: members after `.`; elsewhere names in scope and keywords;
  - document formatting through `nova-fmt`.
- **`tools/vscode-nova/`:**
  - the language, its TextMate grammar and its language configuration;
  - a TypeScript client that runs the installed `nova lsp` (§13, decision 5);
  - a smoke test.
- **CI** builds and tests the extension, and `release.yml` attaches its
  `.vsix` to a tagged release.
- **Records:** ADR 0029, "The language server", and the dated notes of §11.

**The gate** (`docs/phase-3-plan.md` §4, made concrete):

1. A Rust test drives `nova lsp` over stdio. It gets:
   - the diagnostic for a planted type error (E0010), with its exact UTF-16
     range;
   - completions in a file that also has a syntax error;
   - completions that include a std method (`push` on a `Vec`) and a record
     field;
   - a formatted document.
2. The extension's smoke test passes in CI.
3. On the development host, with a release build, `05-json-api` meets the
   200 ms budget (§13, decision 3) for both:
   - an edit to its diagnostics;
   - a completion request to its answer.

   The figures are recorded in ADR 0029 and `docs/benchmarks/README.md`. CI
   asserts at most 2 s for each, with the debug binary.
4. If the budget is missed, adopting `salsa` becomes its own step, decided
   with the user.

## 2. The starting point (read on 2026-10-08)

**The driver** (`crates/nova-driver/src/lib.rs`):
- **It stops at the first failing stage:** after lexing and parsing every
  module (`:582-584`), after resolution (`:635-637`), and after type
  checking (`:643-645`).
- **It prints diagnostics and returns only their count.** `render` writes to
  stderr (`:465-473`).
- **It reads source from disk in two places:** an existence probe on the
  entry (`:457`), and `std::fs::read_to_string` for every module (`:502`).
- **Its public entry points take only a path:** `check_file` (`:43`),
  `compile_file` (`:59`), `build_file` (`:93`), `run_file` (`:437`).
- **`check_file` also lowers to MIR** (`:48`). Some trait-bound checks, and
  E0601 "no `main` function found", live there (`crates/nova-mir/src/mono.rs:20-23`).
  E0601 carries no label.

**The parser** (`crates/nova-parser/src/grammar.rs`):
- **It recovers.** It resynchronises at item boundaries (`:143-162`) and
  statement boundaries (`:165-178`).
- **It always returns `Some(File)`, with its errors** (`:281-288`).
- **The AST has no error node.** A construct that fails is dropped whole. An
  unfinished `foo.` fails at `parse_ident("field access")?` (`:1812`), so the
  statement is lost, `foo` included.

**The resolver** (`crates/nova-resolver/src/lib.rs`):
- **It resolves item-level names only.** Local names are resolved by the type
  checker.
- **Its per-module scopes are private** (`:1223-1228`), and can be queried
  only by exact name.
- **It lexes and parses all 16 std modules on every call** (`:1644-1656`).
- **Std's public names are glob-imported into every module**, last, as the
  lowest-priority binding (`:1583-1593`).

**The type checker** (`crates/nova-typeck/src/`):
- **It continues past type errors**, using `Ty::Error`
  (`crates/nova-hir/src/lib.rs:110-111`), and checks every function
  (`check.rs:170-188`).
- **The typed module it returns is partial when errors were found** (`lib.rs:27-32`).
- **It keeps no position-to-type table.** Local scopes are pushed and popped
  as it goes, and are not kept.
- **A failed field or method lookup discards the receiver's checked
  expression** (`check.rs:5060-5068`, `:5422-5432`).
- **The member lookups are private methods of `Checker`:**
  `record_field_index_and_ty` (`:5111`), `resolve_method_on` (`:5144`),
  `trait_method_index` (`:5193`) and `find_inherent_method` (`:5215`).
  Whether an inherent method takes `self` is kept only in the private
  `selfless` set (`:217`).
- **Error codes:**
  - E0001 "cannot find `{name}` in this scope" (`:3502`), and the same code
    for types and traits;
  - E0010 for a type mismatch (`:6722-6725`).
- **Record fields have a parsed visibility** (`crates/nova-ast/src/item.rs:83`;
  `nova-spec/11-PARSER.md:61`) that nothing enforces.

**Positions:**
- **Spans are byte offsets** (`crates/nova-diagnostics/src/lib.rs:15-20`).
- **`FileDb` only grows**, and its unused `location()` gives byte columns
  (`files.rs:65-75`).
- **Nothing in the compiler converts to UTF-16.**

**Std:** 6,141 lines, or 6,116 without `std/test`. It is re-parsed and
re-checked on every compile; nothing in the front end is cached.

**Speed:** `nova check`, timed seven times on each program:

| Program | Build | Time |
|---|---|---|
| `01-hello-world` | release | 107-115 ms |
| `05-json-api` | release | 108-127 ms |
| `tests/runtime/iterator.nova` (361 lines, the largest test program) | release | 108-126 ms |
| `05-json-api` | debug | 144-162 ms |

The release binary was 12,900,352 bytes, built at 09:28:36. Each set had at
most one cold-start outlier (1,291 ms and 4,027 ms), left out above. Std
dominates every check.

**Tooling:**
- **`nova-lsp`** is a one-line stub that nothing depends on.
- **`Cargo.lock`** has no `lsp-server`, `lsp-types` or `crossbeam-channel`.
  `serde` and `serde_json` are present, through `criterion`, and are declared
  in `[workspace.dependencies]` (`Cargo.toml:44-45`).
- **`nova-fmt`:** `format_file` reads from disk (`crates/nova-fmt/src/file.rs:69-70`),
  and `format_text` ignores `.editorconfig` (`:91-96`).
- **The examples have no `nova.toml`.**
- **CI** sets up no Node and no display.
- **`release.yml`**'s `prepare` step fails unless exactly 4 archives are
  present (`:107-112`).
- **The development host** has Node 22.16.0, npm 11.6.1 and VS Code 1.140.0.

**On crates.io, read 2026-10-08:**
- `lsp-server` 0.10.0 (2026-07-16) depends on `crossbeam-channel`, `log`,
  `serde`, `serde_derive` and `serde_json`.
- `lsp-types` 0.97.0 (2024-06-04) depends on `bitflags` 1, `fluent-uri` 0.1,
  `serde`, `serde_json` and `serde_repr`.
- Every one of these declares a minimum Rust of at most 1.71, or none at all.

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
    /// Check `entry` as a module, not a program: no E0601 (§3.3).
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
- **`check_file`, `build_file`, `run_file` and `nova test` are unchanged in
  behaviour.** They read through `DiskSources`, stop at the first failing
  stage, and print what they print today. The existing CLI tests pin this,
  and one more pins it on a file that has both a syntax error and a type
  error (§9.1).
- Names in `FileDb` stay the paths the driver builds (`lib.rs:510`, `:545`). A
  `Sources` implementation maps them to buffers (§6.9).

### 3.2 `keep_going`

- Every module is lexed and parsed, and every diagnostic is kept.
- Resolution runs even when parsing found errors.
- Type checking runs even when resolution found errors.
- The typed module is returned whatever its errors.
- **Cascades:**
  - A member access with a missing name (§3.4) gets no diagnostic beyond the
    parser's P0001.
  - When the parser drops an item after reading its name (`fn f(x: Int {`),
    it reports that name. A use of a dropped name is typed `Ty::Error` with
    no E0001, so a function being edited does not turn its callers red.
    `parse`'s signature and the AST's shape do not change. The plan chooses
    how the names travel.
  - `Ty::Error` already keeps type errors from cascading.

### 3.3 MIR lowering and E0601

MIR lowering assumes a well-formed program, so `analyze` runs it only when
no stage before it found an error, as `check_file` does. For clean code, the
editor shows exactly what `nova check` shows; for broken code it shows more.

With `module_only`, MIR's E0601 "no `main` function found" is dropped. The
server sets `module_only` for a project file that is not reachable from the
project's entry (§6.2), because such a file is a module, not a program.

### 3.4 The unfinished member access

When a `.` is followed by something other than a name, the parser:
1. reports P0001, as today;
2. keeps the receiver as `Expr::Field` with an empty name, spanning the gap
   at the `.`'s end.

The statement survives. Only `keep_going` ever type-checks such a node:
`nova check` stops at parse errors, and `nova fmt` refuses a file with a
syntax error (ADR 0028).

## 4. What is at the cursor

### 4.1 The probe

`Probe { path: PathBuf, offset: u32 }`. As the type checker checks the
function whose body holds the offset, it records two things in
`ProbeResult`:

- **`receiver: Option<Ty>`:** at a member access whose name touches the
  offset, the empty name of §3.4 included. The type is read after inference
  has finished that function, so it holds no unsolved variables where
  inference solved them.
- **`locals: Vec<(String, Ty)>`:** at a name expression, or the empty name,
  touching the offset. These are the locals in scope there, innermost first,
  each name once.

If the offset is in no function body, both are empty. A probe costs nothing
when absent.

### 4.2 Members

`nova_typeck::members(module: &hir::Module, ty: &Ty, from: ModuleId) -> Vec<Member>`.
Each `Member` has a `name`, a `kind` (`Field` or `Method`) and a `detail`:
the field's type, or the method's signature through `display_ty`. It lists:

- the record's fields;
- its inherent methods that take `self`;
- the methods of every trait the type implements;
- for a type parameter, the methods of its bounds.

Std's types are included, since std is part of every program. `Ty::Error`
gives nothing.

**Visibility:** a field or inherent method declared without `pub` is listed
only when `from` is the module that declares it. This hides std's internals,
such as `Vec`'s `data`, which the checker does not stop anyone using (§2;
§13, decision 7). Trait methods are always listed.

**Shared code:** `members` uses the same lookup code as checking, moved out
of `Checker`, so completion and checking cannot disagree. Whether an inherent
method takes `self` becomes part of `hir::Module` (§2's `selfless`).

### 4.3 Names in scope and keywords

- **`Definitions::names_in_scope(module) -> Vec<(String, DefId)>`:**
  - the module's items;
  - its imports;
  - std's glob-imported names.

  A std name the module shadows is listed once, as the module's own.
- **`nova_lexer::KEYWORDS`:** every alphabetic keyword token in the lexer's
  `Token` enum. A test lexes each one and checks that it comes out as its
  token, not as an identifier, and that no alphabetic token is missing.

## 5. Positions

`nova_diagnostics::LineIndex::new(text)` converts in both directions:

- **To LSP:** `offset → (line, utf16_column)`. A character above U+FFFF counts
  two UTF-16 units. `\r\n` and `\n` both end a line, and the `\r` belongs to
  the line's end.
- **From LSP:** `(line, utf16_column) → offset`. A column past the line's end
  clamps to the end. A line past the last clamps to the end of the text. A
  column inside a surrogate pair clamps to the character's start.

## 6. `nova lsp`

### 6.1 Protocol

- `nova lsp` is a new `nova-cli` subcommand that calls `nova_lsp::run`.
- It speaks JSON-RPC over stdio through `lsp-server`, using `lsp-types`.
- It writes logs only to stderr. Nothing but protocol messages goes to stdout.
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

### 6.2 Projects

- **A file's project** is found with `nova_pm::find_root`:
  - With a `nova.toml`, the entry is the root's `src/main.nova`, as
    `nova run` picks it.
  - With none, the file is its own entry, as with `nova check FILE`.
- **When a project becomes active:** when one of its files is opened. It is
  checked at once, then again on each of these:
  - an edit to one of its buffers;
  - a save;
  - a close (the file is then read from disk);
  - a watched change to a `.nova` file or a `nova.toml` under its root.
- **When a project goes inactive:** when its last open file closes. Its
  diagnostics are cleared.
- **An open file that the entry does not reach** through imports is checked
  on its own, with `module_only`.

### 6.3 Diagnostics

- After each check, the server publishes diagnostics for every module in the
  analysis. An open file's diagnostics carry its buffer's version. A file
  that had diagnostics and now has none gets an empty list.
- **The mapping:**

  | From a Nova `Diagnostic` | To the LSP diagnostic |
  |---|---|
  | Severity: Error, Warning, Note, Help | Error, Warning, Information, Hint |
  | `code` | The diagnostic's code |
  | The primary label | The range |
  | Notes | Appended to the message, one per line |
  | Secondary labels | Related information |

- **The fallback:** a diagnostic with no label, or whose labels all lie in
  std, goes on the entry file's first line. Its message then names the std
  location, such as `<std/core>:12:5`.

### 6.4 Completion

Completion runs on the main thread. It is one `analyze` of the request's
project, over the current buffers, with the probe at the cursor.

- **Inside a string or a comment,** found by lexing the buffer, the answer is
  an empty list.
- **After `.`:** the probe's `receiver` gives `members`, with `from` the
  cursor's module. Fields have kind `Field`, and methods have kind `Method`.
- **Elsewhere:**
  - the probe's `locals`;
  - `names_in_scope` for the cursor's module;
  - `KEYWORDS`.
- Every candidate is returned (`isIncomplete: false`), and the editor filters
  by what has been typed.

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
  - the project;
  - a copy of the project's buffers;
  - a generation number that rises with every change.
- **Queued snapshots** of the same project collapse to the latest.
- **A finished check** whose generation is older than the newest snapshot of
  its project is not published.
- **There is no debounce timer;** collapsing does the same job.

### 6.7 Robustness

- Each `analyze` runs under `catch_unwind`. After a panic in the front end:
  - the panic is logged;
  - the previous diagnostics stay;
  - a completion answers with an empty list;
  - the server keeps running.
- §9.2's test feeds broken programs to `keep_going` on purpose.

### 6.8 The budget

Both of these take at most 200 ms (§13, decision 3):
- from a `didChange` to the `publishDiagnostics` for that version;
- from a completion request to its answer.

They are measured on `examples/05-json-api/src/main.nova`, with a release
build, on the development host (§9.5).

### 6.9 URIs and paths

- Only `file:` URIs map to paths. Percent-encoding is decoded, so
  `file:///d%3A/x/y.nova` becomes `d:\x\y.nova`.
- An untitled buffer is checked as a file of its own, with no project.
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
- **`engines.vscode`:** `^1.85.0`, the floor `vscode-languageclient` 9 needs.
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
- **Packaging:** `@vscode/vsce` writes `nova-vscode-<version>.vsix`, carrying
  `vscode-languageclient`'s production dependencies.
- **The dependencies:**
  - runtime: `vscode-languageclient` ^9;
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
5. the `vsce` package, uploaded as an artifact.

Linux suffices: the extension is platform-neutral, and the Rust gate test
(§9.3) runs the server on all three systems.

**`release.yml`:**
- A new job builds the `.vsix`.
- `prepare` requires exactly 4 archives and 1 `.vsix`.
- `SHA256SUMS` covers the `.vsix`.
- `publish` attaches it to the tagged release.
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
  - E0601 is dropped under `module_only`;
  - MIR's diagnostics appear when the program is otherwise clean.
- **Type checker:**
  - the probe's `receiver` for a record, a std `Vec`, and the result of a
    method chain;
  - its `locals`, shadowing included;
  - `members` for a record, `String`, `Vec` and a bounded type parameter;
  - no-`self` functions are excluded;
  - the visibility rule (§4.2).
- **Resolver:** `names_in_scope` lists the module's items, its imports and
  std's names, with a shadowed std name listed once.
- **Lexer:** `KEYWORDS` (§4.3).
- **Server:**
  - URI to path and back, Windows and Unix, encoded drive letters included;
  - generation dropping;
  - the diagnostic mapping and its fallback.

### 9.2 Broken programs never panic

`analyze` with `keep_going` runs, without a panic, on:
- each example, cut at 16 evenly spaced points;
- each `tests/runtime/*.nova`, cut at 2.

That is about 390 analyses, so the test spreads them across threads. A
floor on the number of programs checked keeps the test from passing
vacuously.

### 9.3 The gate test

`crates/nova-cli/tests/lsp.rs` drives `nova lsp`. Its client frames JSON-RPC
over the child's stdin and stdout, reads on a thread with deadlines, and
kills the child on drop. It checks:

1. the planted E0010, with its exact UTF-16 range on a line that holds
   non-ASCII text before it;
2. completions in a file that also has a syntax error;
3. completions holding `push` (a std method on a `Vec`) and a record field;
4. the formatted document;
5. a module fixed on disk, followed by its watched-file event, clears its
   diagnostic;
6. a burst of edits ends with the last edit's diagnostics;
7. `shutdown` and `exit`, and `exit` alone giving code 1;
8. the CI bounds of §6.8, with the debug binary: at most 2 s for each.

### 9.4 Mutants

Each of these fails at least one named test:
- UTF-16 columns counted in bytes;
- `keep_going` stopping after parse errors;
- a probe that records nothing;
- the generation check removed;
- `format_buffer` ignoring `.editorconfig`;
- no-`self` functions offered after `.`;
- `Sources` bypassed for the disk.

### 9.5 Latency on the development host

An ignored test, run with `--release`, times 20 edits and 20 completions on
`05-json-api` and prints the median and maximum of each. The binary's size
and build time are recorded beside the figures (the "measured the wrong
artifact" rule). The figures go into ADR 0029 and `docs/benchmarks/README.md`.

### 9.6 The extension

§7.6's smoke test, in CI. §7.4's keyword test, in the Rust suite.

## 10. What 3.2 asks of the user during implementation

- **`npm ci` on the development host** downloads the npm packages of §7.5.
  The extension's smoke test downloads a VS Code build (about 150 MB) into the
  git-ignored `.vscode-test/`. Either one is run locally only with the user's
  word; CI runs both on every PR regardless.
- **The merge**, by rebase, on the user's word.

## 11. Records

- **ADR 0029, "The language server":**
  - `lsp-server` and `lsp-types` rather than `tower-lsp` (`40-TOOLING.md`
    §3.2; the master spec's dependency list);
  - re-checking rather than `salsa`, with the budget and its measured
    figures;
  - `keep_going` for the server only;
  - the probe;
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
  - `nova-spec/00-MASTER-SPEC.md`, for the `tower-lsp` lines.
- **Other updates:**
  - `CHANGELOG.md` `[Unreleased]`;
  - `ARCHITECTURE.md`'s `nova-lsp` row;
  - `docs/phase-3-plan.md`'s 3.2 spec line;
  - an "Editor support" section in `README.md`;
  - the set-difference sweep over the living documents.

## 12. Risks

1. **The front end may panic on half-typed code.** It was written for
   finished programs. Mitigated by `catch_unwind` (§6.7) and the cut
   programs (§9.2).
2. **Cascading diagnostics.** Mitigated by the dropped-name rule (§3.2) and
   `Ty::Error`. The tests assert exact diagnostics, so a new cascade fails
   one.
3. **Windows URIs** (`%3A`, the drive letter's case) may not match the
   driver's paths, so a buffer is missed. Mitigated by normalisation (§6.9)
   and tests with encoded URIs.
4. **A flaky VS Code test in CI.** Mitigated by a pinned VS Code version and
   one job on one system.
5. **`lsp-types` has had no release since 2024-06.** LSP 3.17, which it
   covers, is stable, and the crate is small enough to vendor.
6. **The budget has headroom of about 90 ms today.** 3.4 adds work to the
   same engine. If a later sub-phase misses the budget, that is its own step
   with the user (§1, gate 4).

## 13. Decisions

**Made by the user on 2026-10-08:**
1. **Keep going for the LSP only.** The CLI keeps today's staged output.
2. **`lsp-server` and `lsp-types`.**
3. **The budget is 200 ms** for an edit to its diagnostics and for a
   completion request, with CI asserting 2 s with the debug binary.
4. **Recovery plus a probe** (§3.4, §4.1), rather than a placeholder name
   written at the cursor, or a full position index.
5. **The extension runs the installed `nova`,** from `nova.server.path` or
   PATH, with one platform-neutral `.vsix`.
6. **`publisher` is `sakeerin` for now,** settled at `v0.3.0`.

**Made while writing this spec:**

7. **Completion follows declared field and method visibility** (§4.2), though
   the checker enforces none. That is what a reader of the declarations
   expects, and it hides std's internals.
8. **Std's glob-imported names are completion's "prelude".** Std has no
   other prelude (§2).
9. **The client watches files,** through dynamic registration, so no
   file-watching crate is added.
10. **Full text sync.** Nova files are small, and a whole-document update
    keeps the server simple.
11. **No debounce.** Collapsing queued snapshots (§6.6) keeps the checker on
    the latest text without a timer.
12. **Formatting a broken buffer returns no edit** (§6.5), rather than an
    error that format-on-save would raise on every save.
13. **An unreached project file is checked as a module** (§3.3, §6.2), so it
    gets diagnostics before it is imported, without E0601.

## 14. Not in 3.2

- Hover, go to definition, references, rename, code actions and semantic
  tokens (3.4).
- Incremental sync, range formatting, signature help, workspace symbols and
  inlay hints.
- The Zed and Neovim extensions (ADR 0026).
- `salsa`, unless the budget is missed (§1, gate 4).
- Publishing to the VS Code Marketplace or Open VSX (`v0.3.0`).
- A `nova` bundled inside the extension.
- Caching std between checks.
