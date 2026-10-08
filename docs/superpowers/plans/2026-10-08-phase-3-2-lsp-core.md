# Phase 3.2, "LSP core and the VSCode extension", Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** `nova lsp` gives an editor diagnostics, completion and formatting
on code being typed, and `tools/vscode-nova/` connects VS Code to it.

**Architecture:**
- **The front end learns to keep going for the server only.**
  - `nova_driver::analyze` reads sources through a provider, so unsaved
    buffers are checked.
  - It runs every stage whatever the earlier ones found.
  - It returns diagnostics rather than printing them.
- **What is at the cursor:**
  - the parser keeps an unfinished `foo.`;
  - a type-checker probe records the receiver, its members and the locals at
    the cursor;
  - the resolver lists a module's names.
- **`nova-lsp`** runs the protocol on the main thread and checks projects on
  a second thread. A generation number keeps stale results from being
  published.
- **The extension** is a small TypeScript client that runs the installed
  `nova lsp`, plus a TextMate grammar.

**Tech Stack:**
- Rust: MSRV 1.78, edition 2021.
- New crates: `lsp-server =0.7.8` and `lsp-types 0.97`. `serde_json` moves
  into normal dependencies.
- Tests: `assert_cmd` end-to-end tests, and a stdio JSON-RPC test harness.
- The extension: TypeScript 5, `vscode-languageclient` 10, `@vscode/vsce`
  and `@vscode/test-electron`.
- Infrastructure: GitHub Actions, and Docker (`rust:1-slim`) for local Linux
  runs.

**Spec:** `docs/superpowers/specs/2026-10-08-phase-3-2-lsp-core-design.md`
(commits `4e3b4d2` and `0df74e0`), approved by the user on 2026-10-08.
- Read it before Task 1: it is the authority this plan argues from.
- Its §13 lists 16 decisions.
- Its §10 lists what needs the user's word during implementation.

## Global Constraints

- The workspace's minimum Rust stays **1.78**, edition 2021. CI's MSRV job
  runs `cargo check --locked --workspace` with `RUSTFLAGS=-D warnings`.
- **`lsp-server` is pinned `=0.7.8`** in `[workspace.dependencies]`. 0.7.9
  and later are edition 2024, which needs Rust 1.85 (spec §2, §13
  decision 2).
- **`lsp-types` is `0.97`** (0.97.0 is edition 2018).
- **No other new Rust crate.** In particular there is no `tokio`, no
  `notify` and no `url`.
- **`nova check`, `nova build`, `nova run` and `nova test` behave exactly as
  today** (spec §13, decision 1). Every existing test passes unchanged.
- **Only protocol messages go to stdout under `nova lsp`.** Logs go to
  stderr (spec §6.1).
- **Positions are UTF-16** (spec §5).
- **The budget is 200 ms** for both the median and the maximum of 20
  edits-to-diagnostics and of 20 completions, on `05-json-api` in a release
  build on the development host. The CI bound is 2 s in a debug build (spec
  §6.8).
- **The extension:**
  - `publisher` is `sakeerin`, `engines.vscode` is `^1.91.0`, and `version`
    equals `nova-cli`'s version;
  - its runtime dependency is `vscode-languageclient` ^10.1;
  - no bundler, and `package-lock.json` is committed.
- **`npm ci`, `npm test` and anything else that downloads npm packages or
  VS Code** runs on the development host only with the user's word (spec
  §10). CI runs them on every PR.
- **Nothing is published** to the VS Code Marketplace, Open VSX or
  crates.io. No tag is pushed.
- **The merge is by rebase, on the user's word only.**

## Review Focus

The five inputs most likely to bite a user that the spec's tests leave
uncovered, most likely first. Each has its test in the task that owns the
code:

1. **A CRLF file.** Most files on Windows are CRLF. Diagnostics must land on
   the right line and column, and formatting must keep CRLF →
   `a_crlf_document_gets_exact_ranges` (Task 10) and
   `format_buffer_keeps_a_buffers_crlf` (Task 9).
2. **A path with a space or non-ASCII text** (`C:\Users\a b\โปรเจกต์\main.nova`).
   The URI is percent-encoded, and must round-trip to the path the driver
   reads → `uris_with_spaces_and_thai_round_trip` (Task 10) and the gate
   test's directory name (Task 10).
3. **A new project file the entry does not import yet.** Someone writes a
   module before adding its `import`. Completion and diagnostics must still
   work, with no false E0601 →
   `completion_in_a_file_main_does_not_import_yet` (Task 12) and
   `an_unreached_project_file_is_checked_as_a_module` (Task 11).
4. **Closing a buffer whose unsaved text differs from the disk.** The
   diagnostics must go back to the file's contents →
   `closing_an_unsaved_buffer_rechecks_from_disk` (Task 11).
5. **An untitled buffer**, never saved, with no path. It must still get
   diagnostics → `an_untitled_buffer_gets_diagnostics` (Task 10).

## Decisions: where this plan settles what the spec leaves open

1. **How the dropped names take effect** (spec §3.2):
   - `nova_parser::parse_recovering(tokens, file) -> Parsed { file, errors, dropped }`
     sits beside the unchanged `parse`;
   - `analyze` with `keep_going` removes every E0001 whose first backticked
     name is a dropped name.

   Every E0001 site already types the use `Ty::Error` (the checker's
   `error_expr`, an unbound import name), so nothing cascades from it. The
   resolver and the type checker need no change for this, and a test pins
   each E0001 message the filter relies on (Task 7's `a_dropped_name_raises_no_e0001_at_any_site`). Names are matched
   across modules: a dropped `area` in one module also silences an
   unresolved `area` in another, which costs at most one missing E0001
   while code is mid-edit.
2. **`ScopeEntry`** is `Value(Res) | Type(DefId) | Trait(DefId)`. `Res`
   already distinguishes an item, a variant and a builtin. That covers the
   spec's three kinds (§4.3), and adds the type and trait namespaces that
   completion needs for records and traits.
3. **A member's detail.** The checker gives each member a `decl: Option<Span>`,
   because it has no source text. The server cuts the detail from the
   source (spec §4.2):
   - a field's type span is used as it is;
   - a method's span starts at its name, and the server takes the text up
     to the first `{` or `;` at paren depth 0, or the end of the line,
     collapses whitespace, and prefixes `fn `;
   - an array's `len` has no span, and its detail is `fn len(self) -> Int`.
4. **Where the probe records locals** (spec §4.1). The first of these to be
   reached wins:
   - a name expression whose span touches the offset;
   - the first statement of a block that starts at or after the offset,
     with the scope as it is just before that statement;
   - the end of a block whose span holds the offset.

   This gives locals on an empty line too.
5. **`Analysis.modules[i]` is `ModuleId(i)`:** the user's modules come first,
   in load order (`resolve_program`'s `all`, resolver `lib.rs:1515-1546`). A
   test pins it.
6. **Paths are compared as `PathKey`:**
   - the directory is canonicalised when it exists;
   - Windows' `\\?\` prefix is stripped;
   - the whole path is lowercased on Windows.

   `Overlay` implements `Sources` over a `PathKey` map.
7. **An untitled buffer** (`untitled:Untitled-1`) gets the path
   `<temp>/nova-untitled/Untitled-1.nova`, which exists only in the overlay.
   It is loose, and is checked with `module_only` unless it declares
   `fn main`.
8. **The checker thread:**
   - It receives `Job`s over `std::sync::mpsc`.
   - A shared `Mutex<HashMap<ProjectKey, u64>>` holds each project's newest
     generation; a finished job is published only if it is still the newest.
   - A project job also checks, with `module_only`, each open file of that
     project the entry did not reach (spec §6.2).
9. **Completion** analyses the file's project with the probe. If the file
   is not among the project's modules, it analyses again with the file as a
   module (spec §6.2's ownership rule, applied to the probe).
10. **The CLI's subscriber** writes to stderr only under `nova lsp`, chosen
    after argument parsing. Every other command keeps stdout (spec §6.1).
11. **Tests:**
    - **the gate test** is `crates/nova-cli/tests/lsp.rs`, with one framed
      JSON-RPC client in `crates/nova-cli/tests/lsp_client/mod.rs`;
    - **the latency test** is `#[ignore]`d in the same file;
    - **the extension's Rust-side checks** (grammar keywords, version) are in
      `crates/nova-cli/tests/vscode_extension.rs`, which needs `serde_json`
      as a dev-dependency of `nova-cli`.
12. **The smoke test** pins VS Code `1.91.0`, the extension's floor, so the
    `engines` claim is what CI proves.
13. **TypeScript** compiles with `tsc` to `out/` (`target: ES2022`,
    `module: commonjs`, `strict: true`). The test runner is mocha, started by
    `@vscode/test-electron`'s `runTests`.
14. **What the "inside a string or comment" check covers** (spec §6.4):
    - the spans of `StrPart`, `RawStr` and `Char` tokens, and of comments,
      from `lex_with_comments`;
    - not interpolations `${…}`, which are code.
15. **The probe's receiver rule is wider than the spec's** (spec §4.1 says
    "whose name touches the offset"). The probe fires when the offset lies
    between the receiver's end and the name's end. Newlines do not end a
    statement, so `v.` followed by `println("x")` on the next line parses as
    `v.println("x")`. That name never touches a cursor left after the `.`.
    In a chain the first match, the innermost, wins.
16. **Completion's analyses use `module_only`.** Completion needs no MIR,
    and skipping it saves time inside the budget.
17. **`RESERVED_TYPE_NAMES` is already public** (resolver `lib.rs:1141`), so
    spec §4.3's "becomes public" needs no change.
18. **`Analysis.probe` is a `ProbeResult`, not an `Option`** (spec §3.1).
    An empty result already says "nothing here".

## File Structure

| File | Task | Responsibility |
|---|---|---|
| `crates/nova-diagnostics/src/line_index.rs` (new), `src/lib.rs`, `tests/line_index.rs` (new) | 1 | Offsets and UTF-16 positions |
| `crates/nova-lexer/src/lib.rs`, `tests/keywords.rs` (new) | 2 | `KEYWORDS` |
| `crates/nova-resolver/src/lib.rs` | 4 | `ScopeEntry`, `names_in_scope`, and their tests in the inline `mod tests` |
| `crates/nova-parser/src/grammar.rs`, `src/lib.rs`, `tests/parser_tests.rs` | 3 | The unfinished member access, dropped names, `parse_recovering` |
| `crates/nova-typeck/src/check.rs` (with its inline `mod tests`), `src/lib.rs` | 5, 6 | `CheckOptions`, `check_with`, no E0014 for a missing name; the probe and members |
| `crates/nova-driver/src/lib.rs`, `src/analyze.rs` (new), `tests/analyze.rs` (new) | 7 | `Sources`, `Options`, `Analysis`, `analyze` |
| `crates/nova-driver/tests/broken.rs` (new) | 8 | Broken programs never panic or hang |
| `crates/nova-fmt/src/file.rs`, `src/lib.rs`, `tests/files.rs` | 9 | `format_buffer` |
| `Cargo.toml`, `Cargo.lock`, `crates/nova-lsp/Cargo.toml`, `crates/nova-lsp/src/{lib,uri,convert,workspace,checker,completion,formatting}.rs` | 10-13 | The server |
| `crates/nova-cli/src/main.rs`, `src/cmd/{mod,lsp}.rs`, `Cargo.toml` | 10 | `nova lsp`, stderr logging |
| `crates/nova-cli/tests/lsp.rs`, `tests/lsp_client/mod.rs` (new) | 10-14 | The gate test and the latency test |
| `tools/vscode-nova/**` (new), `.gitignore` | 15 | The extension |
| `crates/nova-cli/tests/vscode_extension.rs` (new) | 15 | The grammar's keywords; the extension's version |
| `.github/workflows/ci.yml`, `.github/workflows/release.yml` | 16 | The extension job; the `.vsix` in releases |
| `docs/adr/0029-the-language-server.md` (new), and the records Task 17 lists | 17 | ADR, notes, CHANGELOG, ARCHITECTURE, README, benchmarks, sweep |

## Conventions for every task

- **Working directory.** `D:\Projects\nona\nova`, which is Git Bash
  `/d/Projects/nona/nova`. The Bash tool resets its directory after each
  call, so write `cd /d/Projects/nona/nova && …` in one command.
- **Scratch directory.** `P=/c/Users/SAKEER~1/AppData/Local/Temp/gcm/p32`,
  outside the repository.
  - Long output goes to a file there; read its tail.
  - Before Task 1, copy the helpers from `p31`:
    `mkdir -p $P && cp /c/Users/SAKEER~1/AppData/Local/Temp/gcm/p31/{count,replace_once,insert_after,append,extract}.py $P/`.
- **Counting a full run:** `python -X utf8 $P/count.py <FILE>`. It prints
  `N result lines: P passed, F failed, I ignored`.
- **Line endings.** The working tree is mostly CRLF (`core.autocrlf=true`).
  The Edit tool is fine. Write new files with the Write tool.
- **Write scripts and commit messages with the Write tool, never a Bash
  heredoc.** The Bash tool turns `\\` into `\`. Commit with
  `git commit -F $P/msg-<n>.txt`, then check `git log -1 --format=%s`.
- **Format before every commit:** run `cargo fmt --all`, then stage what it
  changed. rustfmt may rewrap the plan's code; that is expected.
- **Chain a commit and what follows it with `&&`, never `;`.**
- **Grep:** Git Bash's `grep -i` with two or more `-e` patterns aborts
  silently. Use `-i -E "a|b"` or `git grep`.
- **Port 3000 must be free for a full Windows suite** (the
  `http_server_example_*` tests). Check with
  `netstat -ano | grep -E "[:.]3000 .*LISTENING"`, which prints nothing
  when the port is free. The user's own servers sometimes hold it. **Never
  stop such a process: stop and ask the user.**
- **Linux runs** use the existing harness:
  `bash /c/Users/SAKEER~1/AppData/Local/Temp/gcm/linux/run.sh <cargo args>`.
  - It exports the checkout's tracked files as an LF tarball, uncommitted
    edits included. **Untracked files are not exported:** `git add` new
    files first.
  - Docker Desktop must be running: `docker version` shows a `Server:`
    section. Start it with PowerShell
    `Start-Process "C:\Program Files\Docker\Docker\Docker Desktop.exe"`,
    and run it a second time if no `com.docker.backend` process appears.
- **Mutants run only on committed work,** and are undone with
  `git checkout -- <file>`. Never commit a mutant.
- **Unused-code warnings in `nova-lsp` during Tasks 10 to 12 are
  expected:** `PathKey::is_under` and `uri::from_path` are first used in
  Task 11, and `convert::range` by formatting in Task 13. From Task 13 on
  there are none. Task 18 runs clippy with `-D warnings` over the finished
  crate.
- **Expected outputs are exact.** If a test fails on the first run of its
  task, read the failure before touching anything. If the code does what
  this plan describes and the plan's expected value is wrong, correct the
  value and ledger it as a ruling. If the code does something else, fix the
  code.
- **lsp-types names.** The code below uses `lsp-types` 0.97's names. If a
  field or type is named differently in 0.97.0's source (`~/.cargo/registry`),
  follow the source and ledger the difference as a ruling.

---
### Task 1: `LineIndex`, byte offsets to UTF-16 positions and back

Spec §5 and §9.1. A new type; nothing calls it until Task 10.

**Files:**
- Create: `crates/nova-diagnostics/src/line_index.rs`
- Modify: `crates/nova-diagnostics/src/lib.rs:7-10` (declare and export it)
- Create: `crates/nova-diagnostics/tests/line_index.rs`

**Interfaces:**
- Consumes: nothing.
- Produces: `nova_diagnostics::LineIndex` with
  - `LineIndex::new(text: &str) -> LineIndex`;
  - `fn position(&self, offset: u32) -> (u32, u32)`, the zero-based line and
    UTF-16 column;
  - `fn offset(&self, line: u32, column: u32) -> u32`;
  - `fn len(&self) -> u32`, the text's length in bytes.

- [ ] **Step 1: Pre-flight**

```bash
cd /d/Projects/nona/nova && git branch --show-current && git status --short | wc -l && git log -1 --format=%h && P=/c/Users/SAKEER~1/AppData/Local/Temp/gcm/p32 && mkdir -p $P && cp /c/Users/SAKEER~1/AppData/Local/Temp/gcm/p31/{count,replace_once,insert_after,append,extract}.py $P/ && ls $P
```

Expected: `phase-3-2-lsp-core`, `0`, the plan's commit, and the five
scripts listed.

Then take the baseline. Check port 3000 first (Conventions), then:

```bash
cd /d/Projects/nona/nova && P=/c/Users/SAKEER~1/AppData/Local/Temp/gcm/p32 && cargo build --locked -p nova-runtime 2>&1 | tail -1 && cargo test --locked --workspace --all-features --no-fail-fast > $P/suite-baseline.txt 2>&1; echo "exit=$?"; python -X utf8 $P/count.py $P/suite-baseline.txt
```

Expected: `exit=0` and `62 result lines: 1428 passed, 0 failed, 8 ignored`
(CI's Windows figure at `d045d05`). Ledger it as the baseline.

- [ ] **Step 2: Write the failing tests**

`crates/nova-diagnostics/tests/line_index.rs`:

```rust
//! `LineIndex` (spec
//! `docs/superpowers/specs/2026-10-08-phase-3-2-lsp-core-design.md` §5).

use nova_diagnostics::LineIndex;

#[test]
fn ascii_lines_and_columns() {
    let ix = LineIndex::new("ab\ncd");
    assert_eq!(ix.position(0), (0, 0));
    assert_eq!(ix.position(2), (0, 2));
    assert_eq!(ix.position(3), (1, 0));
    assert_eq!(ix.position(4), (1, 1));
    assert_eq!(ix.offset(1, 1), 4);
    assert_eq!(ix.len(), 5);
}

#[test]
fn thai_counts_one_unit_per_character() {
    // Each Thai letter is three bytes of UTF-8 and one UTF-16 unit.
    let ix = LineIndex::new("กข\nx");
    assert_eq!(ix.position(3), (0, 1));
    assert_eq!(ix.position(6), (0, 2));
    assert_eq!(ix.position(7), (1, 0));
    assert_eq!(ix.offset(0, 2), 6);
}

#[test]
fn a_character_above_u_ffff_counts_two_units() {
    // `😀` is four bytes of UTF-8 and a surrogate pair in UTF-16.
    let ix = LineIndex::new("a😀b");
    assert_eq!(ix.position(1), (0, 1));
    assert_eq!(ix.position(5), (0, 3));
    assert_eq!(ix.offset(0, 3), 5);
    // A column inside the pair clamps to the character's start.
    assert_eq!(ix.offset(0, 2), 1);
}

#[test]
fn crlf_ends_a_line_and_its_cr_belongs_to_the_end() {
    let ix = LineIndex::new("a\r\nb\r\n");
    assert_eq!(ix.position(1), (0, 1));
    assert_eq!(ix.position(3), (1, 0));
    assert_eq!(ix.position(4), (1, 1));
    // A column past the line's end clamps to just before its `\r\n`.
    assert_eq!(ix.offset(0, 9), 1);
    assert_eq!(ix.offset(1, 9), 4);
}

#[test]
fn past_the_last_line_clamps_to_the_end_of_the_text() {
    let ix = LineIndex::new("ab\ncd");
    assert_eq!(ix.offset(7, 0), 5);
    assert_eq!(ix.position(99), (1, 2));
}

#[test]
fn every_character_boundary_round_trips() {
    let text = "fn main() {\r\n    let s = \"ก😀\" // é\n}\n";
    let ix = LineIndex::new(text);
    for (offset, c) in text.char_indices() {
        // The `\n` of `\r\n` has no place of its own: it is part of the
        // line's end, which an LSP position reaches only as the `\r`.
        if c == '\n' && text[..offset].ends_with('\r') {
            continue;
        }
        let (line, column) = ix.position(offset as u32);
        assert_eq!(ix.offset(line, column), offset as u32, "offset {offset}");
    }
}
```

- [ ] **Step 3: Run them to verify they fail**

```bash
cd /d/Projects/nona/nova && cargo test --locked -p nova-diagnostics --test line_index 2>&1 | grep -E "error\[|unresolved import" | head -3
```

Expected: `error[E0432]: unresolved import` naming `nova_diagnostics::LineIndex`.

- [ ] **Step 4: Write `LineIndex`**

`crates/nova-diagnostics/src/line_index.rs`:

```rust
//! Byte offsets to LSP positions and back (spec
//! `docs/superpowers/specs/2026-10-08-phase-3-2-lsp-core-design.md` §5).
//!
//! The compiler's spans are byte offsets. The Language Server Protocol
//! counts a position as a zero-based line and a column in UTF-16 code units,
//! so a character above U+FFFF is two columns, and a Thai letter, three bytes
//! of UTF-8, is one.

/// The start of every line in a text.
#[derive(Debug, Clone)]
pub struct LineIndex {
    text: String,
    /// The byte offset of each line's first character; `starts[0]` is 0.
    starts: Vec<u32>,
}

impl LineIndex {
    /// Index `text`. `\n` ends a line, and so does `\r\n`, whose `\r` is
    /// part of the line's end.
    pub fn new(text: &str) -> LineIndex {
        let mut starts = vec![0];
        for (i, b) in text.bytes().enumerate() {
            if b == b'\n' {
                starts.push(i as u32 + 1);
            }
        }
        LineIndex {
            text: text.to_owned(),
            starts,
        }
    }

    /// The text's length in bytes.
    pub fn len(&self) -> u32 {
        self.text.len() as u32
    }

    /// Whether the text is empty.
    pub fn is_empty(&self) -> bool {
        self.text.is_empty()
    }

    /// The zero-based line and UTF-16 column of byte `offset`. An offset
    /// past the end is the end of the text.
    pub fn position(&self, offset: u32) -> (u32, u32) {
        let offset = offset.min(self.len());
        let line = match self.starts.binary_search(&offset) {
            Ok(line) => line,
            Err(next) => next - 1,
        };
        let start = self.starts[line] as usize;
        let mut column = 0u32;
        for (i, c) in self.text[start..].char_indices() {
            if start + i >= offset as usize {
                break;
            }
            column += c.len_utf16() as u32;
        }
        (line as u32, column)
    }

    /// The byte offset of zero-based `line` and UTF-16 `column`.
    ///
    /// Out-of-range positions clamp:
    /// - a column past the line's end goes to the line's end, before its
    ///   `\r\n` or `\n`;
    /// - a line past the last goes to the end of the text;
    /// - a column inside a surrogate pair goes to its character's start.
    pub fn offset(&self, line: u32, column: u32) -> u32 {
        let Some(&start) = self.starts.get(line as usize) else {
            return self.len();
        };
        let end = self.line_end(line as usize);
        let mut units = 0u32;
        for (i, c) in self.text[start as usize..end].char_indices() {
            let next = units + c.len_utf16() as u32;
            if next > column {
                return start + i as u32;
            }
            units = next;
        }
        end as u32
    }

    /// The byte offset where `line`'s text ends, before its line break.
    fn line_end(&self, line: usize) -> usize {
        let start = self.starts[line] as usize;
        let mut end = self
            .starts
            .get(line + 1)
            .map_or(self.text.len(), |&s| s as usize);
        let bytes = self.text.as_bytes();
        if end > start && bytes[end - 1] == b'\n' {
            end -= 1;
        }
        if end > start && bytes[end - 1] == b'\r' {
            end -= 1;
        }
        end
    }
}
```

In `crates/nova-diagnostics/src/lib.rs`, replace

```rust
pub mod files;
pub mod render;

pub use files::{FileDb, FileId};
```

with

```rust
pub mod files;
pub mod line_index;
pub mod render;

pub use files::{FileDb, FileId};
pub use line_index::LineIndex;
```

- [ ] **Step 5: Run the tests to verify they pass**

```bash
cd /d/Projects/nona/nova && cargo test --locked -p nova-diagnostics 2>&1 | grep -E "^test result|FAILED|panicked" | head
```

Expected: every `test result:` line is `ok`, `line_index` with 6 passed.

- [ ] **Step 6: Commit**

Write `$P/msg-1.txt`:

```
nova-diagnostics: LineIndex, byte offsets to UTF-16 positions and back

The language server's positions are a zero-based line and a UTF-16
column (spec docs/superpowers/specs/2026-10-08-phase-3-2-lsp-core-design.md
§5):
- a character above U+FFFF counts two units;
- \r\n ends a line;
- out-of-range positions clamp to the line's end or the text's end;
- a column inside a surrogate pair clamps to the character's start.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
```

```bash
cd /d/Projects/nona/nova && P=/c/Users/SAKEER~1/AppData/Local/Temp/gcm/p32 && cargo fmt --all && git add crates/nova-diagnostics && git commit -q -F $P/msg-1.txt && git log -1 --format=%s
```

Expected: `nova-diagnostics: LineIndex, byte offsets to UTF-16 positions and back`.

The task's test command:
`cargo test --locked -p nova-diagnostics`.

---
### Task 2: `KEYWORDS`

Spec §4.3 and §7.4. Completion offers these words, and the extension's
grammar colours them. `nova_resolver::RESERVED_TYPE_NAMES`, the primitive
type names completion also offers, is already public (resolver
`lib.rs:1141`), so spec §4.3's "becomes public" needs no change.

**Files:**
- Modify: `crates/nova-lexer/src/lib.rs:28-29` (after the `pub use` lines)
- Create: `crates/nova-lexer/tests/keywords.rs`

**Interfaces:**
- Consumes: nothing.
- Produces: `nova_lexer::KEYWORDS: [&str; 32]`, the lexer's alphabetic
  tokens in declaration order.

- [ ] **Step 1: Write the failing tests**

`crates/nova-lexer/tests/keywords.rs`:

```rust
//! `KEYWORDS` (spec
//! `docs/superpowers/specs/2026-10-08-phase-3-2-lsp-core-design.md` §4.3).

use nova_diagnostics::FileId;
use nova_lexer::{lex, Token, KEYWORDS};

#[test]
fn every_keyword_lexes_as_itself_not_as_an_identifier() {
    for word in KEYWORDS {
        let (tokens, errors) = lex(word, FileId::DUMMY);
        assert!(errors.is_empty(), "{word}: {errors:?}");
        // The keyword, then `Eof`.
        assert_eq!(tokens.len(), 2, "{word}: {tokens:?}");
        assert!(
            !matches!(tokens[0].value, Token::Ident(_)),
            "`{word}` lexed as an identifier"
        );
    }
}

#[test]
fn keywords_lists_every_alphabetic_token_the_lexer_declares() {
    let source = include_str!("../src/lib.rs");
    let mut declared: Vec<String> = source
        .split("#[token(\"")
        .skip(1)
        .map(|rest| rest.chars().take_while(|c| *c != '"').collect::<String>())
        .filter(|w| !w.is_empty() && w.chars().all(|c| c.is_ascii_alphabetic() || c == '_'))
        .collect();
    let mut listed: Vec<String> = KEYWORDS.iter().map(|w| w.to_string()).collect();
    declared.sort();
    listed.sort();
    assert_eq!(declared, listed);
}
```

- [ ] **Step 2: Run them to verify they fail**

```bash
cd /d/Projects/nona/nova && cargo test --locked -p nova-lexer --test keywords 2>&1 | grep -E "error\[" | head -3
```

Expected: `error[E0432]: unresolved import` naming `nova_lexer::KEYWORDS`.

- [ ] **Step 3: Add `KEYWORDS`**

In `crates/nova-lexer/src/lib.rs`, after `pub use token::Token;`:

```rust

/// Every keyword, in the order the lexer declares its tokens: the words
/// completion offers, and the extension's grammar colours (spec
/// `docs/superpowers/specs/2026-10-08-phase-3-2-lsp-core-design.md` §4.3,
/// §7.4). `tests/keywords.rs` keeps it equal to the lexer's alphabetic
/// tokens.
pub const KEYWORDS: [&str; 32] = [
    "let", "mut", "const", "fn", "return", "if", "else", "while", "for", "in", "break",
    "continue", "match", "type", "record", "trait", "impl", "import", "module", "pub", "async",
    "await", "extern", "unsafe", "true", "false", "as", "is", "where", "with", "self", "Self",
];
```

- [ ] **Step 4: Run the tests to verify they pass**

```bash
cd /d/Projects/nona/nova && cargo test --locked -p nova-lexer 2>&1 | grep -E "^test result|FAILED|panicked" | head
```

Expected: every `test result:` line is `ok`, `keywords` with 2 passed.

- [ ] **Step 5: Commit**

Write `$P/msg-2.txt`:

```
nova-lexer: KEYWORDS, the words completion offers

The lexer's 32 alphabetic tokens, in declaration order, for the
language server's completion and the extension's grammar (spec
docs/superpowers/specs/2026-10-08-phase-3-2-lsp-core-design.md §4.3,
§7.4). A test reads the lexer's own source, so the list cannot fall
behind a new keyword.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
```

```bash
cd /d/Projects/nona/nova && P=/c/Users/SAKEER~1/AppData/Local/Temp/gcm/p32 && cargo fmt --all && git add crates/nova-lexer && git commit -q -F $P/msg-2.txt && git log -1 --format=%s
```

Expected: `nova-lexer: KEYWORDS, the words completion offers`.

The task's test command: `cargo test --locked -p nova-lexer`.

---
### Task 3: The parser keeps an unfinished `foo.`, and names what it drops

Spec §3.2, §3.4 and §9.1.

- **`parse` keeps its signature.** Its output changes only for an unfinished
  member access, so the CLI's parse errors read as before: the P0001
  message is the one `parse_ident` gives today.
- **Which later errors follow can change.** Parsing now continues after the
  `.` instead of discarding the statement. No existing test feeds such
  input; Step 4 runs them all to confirm.

**Files:**
- Modify: `crates/nova-parser/src/grammar.rs`:
  - the `Parser` struct, `:31-45`, and `Parser::new`, `:47-56`;
  - `parse_file`, the free function at `:281-288`;
  - the `parse_file` method's loop, `:294-330`;
  - the `Token::Dot` arm, `:1801-1821`.
- Modify: `crates/nova-parser/src/lib.rs` (`Parsed`, `parse_recovering`)
- Test: `crates/nova-parser/tests/parser_tests.rs` (append)

**Interfaces:**
- Consumes: nothing.
- Produces:
  - `nova_parser::Parsed { pub file: File, pub errors: Vec<ParseError>, pub dropped: Vec<Spanned<String>> }`;
  - `nova_parser::parse_recovering(tokens: &[Spanned<Token>], file: FileId) -> Parsed`;
  - in the AST, an unfinished `x.` is `Expr::Field { target: x, field }`,
    where `field.value` is `""` and `field.span` is the empty span at the
    `.`'s end.

- [ ] **Step 1: Write the failing tests**

Append to `crates/nova-parser/tests/parser_tests.rs`:

```rust

// === Phase 3.2: recovery for the language server (spec
// docs/superpowers/specs/2026-10-08-phase-3-2-lsp-core-design.md §3.2, §3.4) ===

#[test]
fn an_unfinished_member_access_keeps_its_receiver() {
    let source = "fn main() {\n    v.\n}\n";
    let (file, errors) = parse_str(source);
    assert!(
        matches!(&errors[..], [ParseError::Expected { expected, .. }]
            if expected == "identifier (in field access)"),
        "{errors:?}"
    );
    let Item::Function(f) = &file.items[0].value else {
        panic!("{:?}", file.items);
    };
    let trailing = f.body.value.trailing.as_ref().expect("the body keeps `v.`");
    let nova_ast::expr::Expr::Field { target, field } = &trailing.value else {
        panic!("{:?}", trailing.value);
    };
    assert!(
        matches!(&target.value, nova_ast::expr::Expr::Path(p) if p.segments[0].value == "v"),
        "{:?}",
        target.value
    );
    assert_eq!(field.value, "");
    // The missing name is the empty span right after the `.`.
    let dot_end = "fn main() {\n    v.".len() as u32;
    assert_eq!((field.span.start, field.span.end), (dot_end, dot_end));
}

#[test]
fn an_item_dropped_after_its_name_is_reported_by_name() {
    let source = "fn f(x: Int {\n}\nfn g() {}\n";
    let mut db = FileDb::new();
    let id = db.add("dropped", source);
    let (tokens, lex_errors) = lex(source, id);
    assert!(lex_errors.is_empty(), "{lex_errors:?}");
    let parsed = nova_parser::parse_recovering(&tokens, id);
    assert!(!parsed.errors.is_empty());
    let dropped: Vec<&str> = parsed.dropped.iter().map(|n| n.value.as_str()).collect();
    assert_eq!(dropped, ["f"]);
    // The item after it still parses.
    assert!(parsed
        .file
        .items
        .iter()
        .any(|i| matches!(&i.value, Item::Function(f) if f.name.value == "g")));
}

#[test]
fn parse_recovering_agrees_with_parse_on_a_clean_file() {
    let source = "fn main() {\n    let v = 1\n    println(\"${v}\")\n}\n";
    let mut db = FileDb::new();
    let id = db.add("clean", source);
    let (tokens, _) = lex(source, id);
    let parsed = nova_parser::parse_recovering(&tokens, id);
    let (file, errors) = parse(&tokens, id);
    assert!(parsed.errors.is_empty() && errors.is_empty());
    assert!(parsed.dropped.is_empty());
    assert_eq!(
        format!("{:?}", parsed.file),
        format!("{:?}", file.expect("Some"))
    );
}
```

- [ ] **Step 2: Run them to verify they fail**

```bash
cd /d/Projects/nona/nova && cargo test --locked -p nova-parser --test parser_tests 2>&1 | grep -E "error\[|^test .*(unfinished|dropped|recovering).*FAILED" | head -5
```

Expected: `error[E0425]` (or `E0433`) naming `parse_recovering`. The tests
cannot compile until Step 3.

- [ ] **Step 3: Implement**

In `crates/nova-parser/src/grammar.rs`, add a field to `Parser`, after
`pending_gt: usize,`:

```rust
    /// The names of top-level items that failed to parse after their names
    /// were read: `fn f(x: Int {` drops `f` (spec
    /// `docs/superpowers/specs/2026-10-08-phase-3-2-lsp-core-design.md`
    /// §3.2). The language server stops reporting their uses as unresolved.
    dropped: Vec<Spanned<String>>,
```

and initialise it in `Parser::new`, after `pending_gt: 0,`:

```rust
            dropped: Vec::new(),
```

Replace the free function

```rust
pub(crate) fn parse_file(
    tokens: &[Spanned<Token>],
    file: FileId,
) -> (Option<File>, Vec<ParseError>) {
    let mut p = Parser::new(tokens, file);
    let ast_file = p.parse_file();
    (Some(ast_file), p.errors)
}
```

with

```rust
pub(crate) fn parse_file(
    tokens: &[Spanned<Token>],
    file: FileId,
) -> (Option<File>, Vec<ParseError>) {
    let parsed = parse_recovering(tokens, file);
    (Some(parsed.file), parsed.errors)
}

pub(crate) fn parse_recovering(tokens: &[Spanned<Token>], file: FileId) -> crate::Parsed {
    let mut p = Parser::new(tokens, file);
    let ast_file = p.parse_file();
    crate::Parsed {
        file: ast_file,
        errors: p.errors,
        dropped: p.dropped,
    }
}
```

In the `parse_file` method, replace

```rust
        while !self.is_at_end() {
            match self.try_parse_item() {
                Some(item) => items.push(item),
                None => {
```

with

```rust
        while !self.is_at_end() {
            let start = self.pos;
            match self.try_parse_item() {
                Some(item) => items.push(item),
                None => {
                    self.note_dropped_item(start);
```

and add this method to the same `impl` block, after `parse_file`:

```rust
    /// The item that started at token `start` failed. If it got as far as
    /// its name, remember the name (spec
    /// `docs/superpowers/specs/2026-10-08-phase-3-2-lsp-core-design.md`
    /// §3.2). Only the tokens the failed attempt consumed are read.
    fn note_dropped_item(&mut self, start: usize) {
        let consumed = &self.tokens[start..self.pos.min(self.tokens.len())];
        let keyword = consumed.iter().position(|t| {
            matches!(
                t.value,
                Token::Fn | Token::Record | Token::Type | Token::Trait | Token::Const
            )
        });
        if let Some(k) = keyword {
            if let Some(Token::Ident(name)) = consumed.get(k + 1).map(|t| &t.value) {
                let span = consumed[k + 1].span;
                self.dropped.push(Spanned::new(name.clone(), span));
            }
        }
    }
```

In the `Token::Dot` arm, replace

```rust
                    } else {
                        let field = self.parse_ident("field access")?;
                        let end = field.span;
```

with

```rust
                    } else if !matches!(
                        self.peek(),
                        Token::Ident(_) | Token::SelfLower | Token::SelfUpper
                    ) {
                        // An unfinished `foo.` keeps its receiver, so an
                        // editor can complete after the `.` (spec
                        // docs/superpowers/specs/2026-10-08-phase-3-2-lsp-core-design.md
                        // §3.4). The error is the one `parse_ident` reports.
                        self.errors.push(ParseError::Expected {
                            expected: "identifier (in field access)".to_owned(),
                            found: self.peek().description().to_owned(),
                            span: self.peek_span(),
                        });
                        let dot_end = self.tokens[self.pos - 1].span.end;
                        let gap = Span::new(dot_end, dot_end, self.file);
                        expr = Spanned::new(
                            Expr::Field {
                                target: Box::new(expr),
                                field: Spanned::new(String::new(), gap),
                            },
                            start.merge(gap),
                        );
                        break;
                    } else {
                        let field = self.parse_ident("field access")?;
                        let end = field.span;
```

In `crates/nova-parser/src/lib.rs`, after the `parse` function:

```rust

/// What [`parse_recovering`] returns.
#[derive(Debug)]
pub struct Parsed {
    /// The file, with every item that parsed.
    pub file: File,
    pub errors: Vec<ParseError>,
    /// The names of the top-level items dropped after their names were
    /// read: `fn f(x: Int {` drops `f`.
    pub dropped: Vec<Spanned<String>>,
}

/// [`parse`], also naming the items it dropped (spec
/// `docs/superpowers/specs/2026-10-08-phase-3-2-lsp-core-design.md` §3.2).
/// The language server's analysis calls this; every other caller uses
/// [`parse`], whose result is the same file and errors.
pub fn parse_recovering(tokens: &[Spanned<Token>], file: FileId) -> Parsed {
    grammar::parse_recovering(tokens, file)
}
```

- [ ] **Step 4: Run the parser's tests to verify they pass**

```bash
cd /d/Projects/nona/nova && P=/c/Users/SAKEER~1/AppData/Local/Temp/gcm/p32 && cargo test --locked -p nova-parser > $P/t3.txt 2>&1; echo "exit=$?"; python -X utf8 $P/count.py $P/t3.txt; grep -E "FAILED|panicked" $P/t3.txt | head
```

Expected: `exit=0`, 0 failed. The three new tests pass, and every existing
parser test and snapshot passes unchanged.

- [ ] **Step 5: Commit**

Write `$P/msg-3.txt`:

```
nova-parser: keep an unfinished `foo.`, and name the items it drops

For the language server (spec
docs/superpowers/specs/2026-10-08-phase-3-2-lsp-core-design.md §3.2,
§3.4):
- A `.` followed by something other than a name still reports the same
  P0001. The receiver now survives as a field access whose name is
  empty, at the `.`'s end, so completion can find the receiver's type.
- `parse_recovering` returns the file and errors `parse` returns, plus
  the names of the top-level items dropped after their names were read.

`parse` keeps its signature. The CLI never type-checks a file with a
parse error, so the empty name reaches only the language server.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
```

```bash
cd /d/Projects/nona/nova && P=/c/Users/SAKEER~1/AppData/Local/Temp/gcm/p32 && cargo fmt --all && git add crates/nova-parser && git commit -q -F $P/msg-3.txt && git log -1 --format=%s
```

Expected: `nova-parser: keep an unfinished `foo.`, and name the items it drops`.

The task's test command: `cargo test --locked -p nova-parser`.

---
### Task 4: The resolver lists a module's names

Spec §4.3 and §9.1. A new method; resolution does not change.

**Files:**
- Modify: `crates/nova-resolver/src/lib.rs`:
  - `ScopeEntry` and `names_in_scope`, after `impl Definitions`;
  - the inline `mod tests`, which gains two tests at its end.

**Interfaces:**
- Consumes: nothing.
- Produces:
  - `pub enum ScopeEntry { Value(Res), Type(DefId), Trait(DefId) }`;
  - `Definitions::names_in_scope(&self, module: ModuleId) -> Vec<(String, ScopeEntry)>`,
    sorted by name, then values, types, traits.

- [ ] **Step 1: Write the failing tests**

In `crates/nova-resolver/src/lib.rs`, add at the end of `mod tests`,
before its closing `}`:

```rust

    // === Phase 3.2: names in scope (spec
    // docs/superpowers/specs/2026-10-08-phase-3-2-lsp-core-design.md §4.3) ===

    #[test]
    fn names_in_scope_lists_items_imports_builtins_variants_and_std() {
        let r = resolve_two(
            "import lib::{area}\nrecord Point { x: Int }\nfn main() {}\n",
            "pub fn area() -> Int { 1 }\n",
        );
        assert!(r.diagnostics.is_empty(), "{:?}", r.diagnostics);
        let names = r.definitions.names_in_scope(ModuleId(0));
        let has = |name: &str, kind: fn(&ScopeEntry) -> bool| {
            names.iter().any(|(n, e)| n == name && kind(e))
        };
        assert!(has("main", |e| matches!(e, ScopeEntry::Value(Res::Def(_)))));
        assert!(has("Point", |e| matches!(e, ScopeEntry::Type(_))));
        assert!(has("area", |e| matches!(e, ScopeEntry::Value(Res::Def(_)))));
        assert!(has("println", |e| matches!(e, ScopeEntry::Value(Res::Builtin(_)))));
        assert!(has("Some", |e| matches!(e, ScopeEntry::Value(Res::Variant(_, _)))));
        assert!(has("Vec", |e| matches!(e, ScopeEntry::Type(_))));
        assert!(has("Display", |e| matches!(e, ScopeEntry::Trait(_))));
        let mut sorted = names.clone();
        sorted.sort_by(|a, b| a.0.cmp(&b.0));
        assert_eq!(
            names.iter().map(|n| &n.0).collect::<Vec<_>>(),
            sorted.iter().map(|n| &n.0).collect::<Vec<_>>(),
            "sorted by name"
        );
    }

    #[test]
    fn a_std_name_the_module_shadows_is_listed_once_as_its_own() {
        let r = resolve_two("record Vec { n: Int }\nfn main() {}\n", "pub fn f() {}\n");
        assert!(r.diagnostics.is_empty(), "{:?}", r.diagnostics);
        let vecs: Vec<DefId> = r
            .definitions
            .names_in_scope(ModuleId(0))
            .into_iter()
            .filter_map(|(n, e)| match e {
                ScopeEntry::Type(id) if n == "Vec" => Some(id),
                _ => None,
            })
            .collect();
        assert_eq!(vecs.len(), 1, "{vecs:?}");
        let DefKind::Record { item_index } = r.definitions.def(vecs[0]).kind else {
            panic!("not a record");
        };
        assert_eq!(r.definitions.module_of(item_index), ModuleId(0));
    }

```

- [ ] **Step 2: Run them to verify they fail**

```bash
cd /d/Projects/nona/nova && cargo test --locked -p nova-resolver --lib 2>&1 | grep -E "error\[" | head -4
```

Expected: `error[E0412]` or `E0425` naming `ScopeEntry` and `names_in_scope`.

- [ ] **Step 3: Implement**

After the `impl Definitions { … }` block that ends with `extern_functions`,
add:

```rust

/// What a name in a module's scope is bound to: what completion offers
/// (spec `docs/superpowers/specs/2026-10-08-phase-3-2-lsp-core-design.md`
/// §4.3).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScopeEntry {
    /// A value: a definition, a variant or a builtin.
    Value(Res),
    /// A record or a sum type.
    Type(DefId),
    /// A trait.
    Trait(DefId),
}

impl Definitions {
    /// Every name `module` can see, in all three namespaces:
    /// - its own items;
    /// - its imports;
    /// - the builtins;
    /// - std's glob-imported names.
    ///
    /// A module binds each name once per namespace, so a std name the
    /// module shadows appears once, as the module's own. The result is
    /// sorted by name, then values, types and traits, for a stable order.
    pub fn names_in_scope(&self, module: ModuleId) -> Vec<(String, ScopeEntry)> {
        let Some(scope) = self.modules.get(module.0 as usize) else {
            return Vec::new();
        };
        let mut out: Vec<(String, ScopeEntry)> = Vec::new();
        out.extend(
            scope
                .values
                .iter()
                .map(|(n, r)| (n.clone(), ScopeEntry::Value(*r))),
        );
        out.extend(
            scope
                .types
                .iter()
                .map(|(n, d)| (n.clone(), ScopeEntry::Type(*d))),
        );
        out.extend(
            scope
                .traits
                .iter()
                .map(|(n, d)| (n.clone(), ScopeEntry::Trait(*d))),
        );
        let rank = |e: &ScopeEntry| match e {
            ScopeEntry::Value(_) => 0,
            ScopeEntry::Type(_) => 1,
            ScopeEntry::Trait(_) => 2,
        };
        out.sort_by(|a, b| a.0.cmp(&b.0).then(rank(&a.1).cmp(&rank(&b.1))));
        out
    }
}
```

- [ ] **Step 4: Run the resolver's tests to verify they pass**

```bash
cd /d/Projects/nona/nova && cargo test --locked -p nova-resolver 2>&1 | grep -E "^test result|FAILED|panicked" | head
```

Expected: every `test result:` line is `ok`, and the two new tests pass.

- [ ] **Step 5: Commit**

Write `$P/msg-4.txt`:

```
nova-resolver: names_in_scope, the names a module sees

For the language server's completion (spec
docs/superpowers/specs/2026-10-08-phase-3-2-lsp-core-design.md §4.3):
every name a module sees in each namespace (its items, its imports,
the builtins, std's glob-imported names), sorted, with a shadowed std
name listed once. Resolution itself does not change.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
```

```bash
cd /d/Projects/nona/nova && P=/c/Users/SAKEER~1/AppData/Local/Temp/gcm/p32 && cargo fmt --all && git add crates/nova-resolver && git commit -q -F $P/msg-4.txt && git log -1 --format=%s
```

Expected: `nova-resolver: names_in_scope, the names a module sees`.

The task's test command: `cargo test --locked -p nova-resolver`.

---
### Task 5: The type checker takes a probe, and records the receiver at the cursor

Spec §3.2, §4.1 and §9.1. `check` keeps its signature and its behaviour.
`check_with` adds a probe:
- with no probe, it records nothing;
- a member access with a missing name (Task 3) adds no E0014.

**Files:**
- Modify: `crates/nova-typeck/src/lib.rs`:
  - `pub use check::{check, check_with}`;
  - `CheckOptions`, `ProbePoint`, `ProbeResult`, `Member` and `MemberKind`;
  - `CheckResult.probe`.
- Modify: `crates/nova-typeck/src/check.rs`:
  - `check` and `check_with` (`:113-202`), and the `use crate::…` line
    (`:14`);
  - three `Checker` fields, after `diagnostics: Vec<Diagnostic>,` (`:271`);
  - `check_field` (`:5037-5069`) and `check_method_call` (`:5355-5368`);
  - `finalize_function` (`:3050`);
  - the inline `mod tests`, which gains a helper and four tests.

**Interfaces:**
- Consumes: Task 3's `nova_parser::parse_recovering`, in tests.
- Produces:
  - `nova_typeck::check_with(file: &ast::File, defs: &Definitions, options: &CheckOptions) -> CheckResult`;
  - `CheckOptions { pub probe: Option<ProbePoint> }` (`Default`, `Copy`);
  - `ProbePoint { pub file: FileId, pub offset: u32 }`;
  - `ProbeResult { pub receiver: Option<Ty>, pub members: Vec<Member>, pub locals: Vec<(String, Ty)> }`
    (`Default`). `members` and `locals` are filled by Task 6;
  - `Member { pub name: String, pub kind: MemberKind, pub decl: Option<Span> }`;
  - `MemberKind { Field, Method }`;
  - `CheckResult.probe: ProbeResult`.

- [ ] **Step 1: Write the failing tests**

In `crates/nova-typeck/src/check.rs`, add at the end of `mod tests`,
before its closing `}`:

```rust

    // === Phase 3.2: the probe (spec
    // docs/superpowers/specs/2026-10-08-phase-3-2-lsp-core-design.md §4.1) ===

    /// Check `marked` with the probe where `<|>` stands, after removing the
    /// marker. The module and each std module get their own `FileId`, so the
    /// probe's offset cannot also land in std's code. Parse errors are
    /// allowed, as the language server allows them.
    fn probe_src(marked: &str) -> (CheckResult, Definitions) {
        let offset = marked.find("<|>").expect("a `<|>` marks the probe") as u32;
        let src = marked.replacen("<|>", "", 1);
        let mut db = nova_diagnostics::FileDb::new();
        let file = db.add("probe.nova", src.as_str());
        let std_files: Vec<FileId> = nova_resolver::STD_MODULES
            .iter()
            .map(|&(name, source)| db.add(name, source))
            .collect();
        let (tokens, _) = lex(&src, file);
        let parsed = nova_parser::parse_recovering(&tokens, file);
        let module = nova_resolver::ModuleSource {
            name: "main".to_string(),
            file: &parsed.file,
        };
        let resolved = nova_resolver::resolve_program(&[module], &std_files, None);
        let options = CheckOptions {
            probe: Some(ProbePoint { file, offset }),
        };
        let checked = check_with(&resolved.file, &resolved.definitions, &options);
        (checked, resolved.definitions)
    }

    #[test]
    fn a_missing_member_name_adds_no_type_error() {
        let (r, _) = probe_src("fn main() {\n    let s = \"a\"\n    s.<|>\n}\n");
        assert!(error_codes(&r).is_empty(), "{:?}", r.diagnostics);
    }

    #[test]
    fn the_probe_records_the_receiver_after_a_dot() {
        let (r, _) = probe_src("fn main() {\n    let s = \"a\"\n    s.<|>\n}\n");
        assert_eq!(r.probe.receiver, Some(Ty::String));
    }

    #[test]
    fn the_probe_reaches_a_name_on_the_next_line() {
        // Newlines do not end a statement, so `v.` then `println("x")` is the
        // call `v.println("x")`. The cursor is still after the `.`.
        let (r, _) = probe_src("fn main() {\n    let v = 1\n    v.<|>\n    println(\"x\")\n}\n");
        assert_eq!(r.probe.receiver, Some(Ty::Int));
    }

    #[test]
    fn a_chain_records_the_innermost_receiver() {
        let (r, defs) = probe_src(
            "record Q { n: Int }\nrecord P { q: Q }\n\
             fn main() {\n    let p = P { q: Q { n: 1 } }\n    let m = p.q<|>.n\n}\n",
        );
        let receiver = r.probe.receiver.expect("a receiver");
        assert_eq!(display_ty(&receiver, &defs), "P");
    }

    #[test]
    fn without_a_probe_nothing_is_recorded() {
        let r = check_src("fn main() {\n    let s = \"a\"\n    let n = s.len()\n}\n");
        assert_eq!(r.probe, ProbeResult::default());
    }
```

- [ ] **Step 2: Run them to verify they fail**

```bash
cd /d/Projects/nona/nova && cargo test --locked -p nova-typeck --lib 2>&1 | grep -E "error\[" | head -4
```

Expected: `error[E0425]` or `E0412` naming `check_with`, `CheckOptions`,
`ProbePoint` and `ProbeResult`.

- [ ] **Step 3: Add the types**

In `crates/nova-typeck/src/lib.rs`, replace `pub use check::check;` with
`pub use check::{check, check_with};`. Add a `probe` field to `CheckResult`,
after `diagnostics`:

```rust
    /// What the probe found: empty unless [`check_with`] was given one.
    pub probe: ProbeResult,
```

and add, after `CheckResult`:

```rust

/// Where the language server's probe looks: a byte offset in one file (spec
/// `docs/superpowers/specs/2026-10-08-phase-3-2-lsp-core-design.md` §4.1).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProbePoint {
    pub file: nova_diagnostics::FileId,
    pub offset: u32,
}

/// What [`check_with`] does beyond checking.
#[derive(Debug, Clone, Copy, Default)]
pub struct CheckOptions {
    pub probe: Option<ProbePoint>,
}

/// What the probe found (spec §4.1, §4.2). Each part is empty when the
/// probe's place holds nothing of its kind.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ProbeResult {
    /// At a member access whose receiver ends at or before the offset and
    /// whose name ends at or after it: the receiver's type.
    pub receiver: Option<Ty>,
    /// The receiver's fields and methods.
    pub members: Vec<Member>,
    /// The locals in scope at the offset, innermost first, each name once.
    pub locals: Vec<(String, Ty)>,
}

/// A field or method completion can offer after `.`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Member {
    pub name: String,
    pub kind: MemberKind,
    /// Where it is declared: a field's type, or a method's name, from which
    /// the server reads the signature. `None` for a built-in method.
    pub decl: Option<nova_diagnostics::Span>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemberKind {
    Field,
    Method,
}
```

- [ ] **Step 4: Thread the probe through the checker**

In `check.rs`, replace `use crate::{display_ty, CheckResult};` with:

```rust
use crate::{display_ty, CheckOptions, CheckResult, ProbePoint, ProbeResult};
```

Replace

```rust
/// Type-check a parsed file against its resolved definitions.
pub fn check(file: &ast::File, defs: &Definitions) -> CheckResult {
    let mut checker = Checker {
```

with

```rust
/// Type-check a parsed file against its resolved definitions.
pub fn check(file: &ast::File, defs: &Definitions) -> CheckResult {
    check_with(file, defs, &CheckOptions::default())
}

/// [`check`], with the language server's probe (spec
/// `docs/superpowers/specs/2026-10-08-phase-3-2-lsp-core-design.md` §4.1).
pub fn check_with(file: &ast::File, defs: &Definitions, options: &CheckOptions) -> CheckResult {
    let mut checker = Checker {
```

In the same `Checker { … }` literal, after `diagnostics: Vec::new(),` add:

```rust
        probe: options.probe,
        probe_result: ProbeResult::default(),
        probe_pending: None,
```

In the `CheckResult { … }` literal at the end of `check_with`, after
`diagnostics: checker.diagnostics,` add:

```rust
        probe: checker.probe_result,
```

In `struct Checker`, after its last field, `diagnostics: Vec<Diagnostic>,`
(the one just before `/// Per-function checking state.`), add:

```rust
    /// The language server's probe (spec 3.2 §4.1), what it has found, and
    /// the receiver it met in the function being checked. That receiver's
    /// type is read when inference finishes the function
    /// (`finalize_function`).
    probe: Option<ProbePoint>,
    probe_result: ProbeResult,
    probe_pending: Option<ProbePending>,
```

and after the `struct Checker` block, add:

```rust

/// A receiver the probe met, before its function's inference is finished.
struct ProbePending {
    receiver: Ty,
}
```

Add this method to `impl<'a> Checker<'a>`, beside `check_field`:

```rust
    /// Spec 3.2 §4.1: if the probe's offset lies between a member access's
    /// receiver and the end of its name, remember the receiver. The name may
    /// be the empty one of an unfinished `x.`, or a name on a later line:
    /// `v.` then `println("x")` parses as `v.println("x")`. The first match
    /// wins, which in a chain `a.b.c` is the innermost.
    fn probe_receiver(&mut self, recv_ty: &Ty, receiver: Span, name: Span) {
        let Some(p) = self.probe else {
            return;
        };
        if self.probe_pending.is_some() || self.probe_result.receiver.is_some() {
            return;
        }
        if name.file == p.file && receiver.end <= p.offset && p.offset <= name.end {
            self.probe_pending = Some(ProbePending {
                receiver: recv_ty.clone(),
            });
        }
    }
```

In `check_field`, replace

```rust
        let recv = self.check_expr(fcx, target);
        let recv_ty = fcx.icx.apply(&recv.ty);
        if let Some((index, field_ty)) =
```

with

```rust
        let recv = self.check_expr(fcx, target);
        let recv_ty = fcx.icx.apply(&recv.ty);
        self.probe_receiver(&recv_ty, target.span, field.span);
        // An unfinished `x.` has an empty name, and the parser has already
        // reported it (spec 3.2 §3.4).
        if field.value.is_empty() {
            return error_expr(span);
        }
        if let Some((index, field_ty)) =
```

In `check_method_call`, replace

```rust
        let recv_ty = fcx.icx.apply(&receiver.ty);
        if matches!(recv_ty, Ty::Error) {
            return error_expr(span);
        }
```

with

```rust
        let recv_ty = fcx.icx.apply(&receiver.ty);
        self.probe_receiver(&recv_ty, receiver.span, method.span);
        if matches!(recv_ty, Ty::Error) {
            return error_expr(span);
        }
```

At the start of `finalize_function`'s body, before
`let mut residual: Vec<Span> = Vec::new();`, add:

```rust
        // Spec 3.2 §4.1: the receiver the probe met in this function, read
        // with the function's final substitution. The function is finalized
        // before the closures lifted from it, so it takes the receiver even
        // when the access was inside one of them.
        if let Some(pending) = self.probe_pending.take() {
            self.probe_result.receiver = Some(icx.apply(&pending.receiver));
        }
```

- [ ] **Step 5: Run the type checker's tests to verify they pass**

```bash
cd /d/Projects/nona/nova && P=/c/Users/SAKEER~1/AppData/Local/Temp/gcm/p32 && cargo test --locked -p nova-typeck > $P/t5.txt 2>&1; echo "exit=$?"; python -X utf8 $P/count.py $P/t5.txt; grep -E "FAILED|panicked" $P/t5.txt | head
```

Expected: `exit=0` and 0 failed. The five new tests pass, and every
existing type-checker test passes unchanged.

- [ ] **Step 6: Commit**

Write `$P/msg-5.txt`:

```
nova-typeck: check_with and a probe that records the receiver at the cursor

For the language server (spec
docs/superpowers/specs/2026-10-08-phase-3-2-lsp-core-design.md §3.2,
§4.1):
- `check_with` takes `CheckOptions` with an optional probe, a byte offset
  in one file.
- When the offset lies between a member access's receiver and the end
  of its name, the receiver's type is recorded, read once inference has
  finished the function. The name may be the empty one of an unfinished
  `x.`, or one on the next line, since `v.` then `println(..)` is the
  call `v.println(..)`. In a chain the innermost access wins.
- A member access whose name is missing adds no E0014: the parser has
  already reported it.

`check` calls `check_with` with no probe, so nothing else changes.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
```

```bash
cd /d/Projects/nona/nova && P=/c/Users/SAKEER~1/AppData/Local/Temp/gcm/p32 && cargo fmt --all && git add crates/nova-typeck && git commit -q -F $P/msg-5.txt && git log -1 --format=%s
```

Expected: `nova-typeck: check_with and a probe that records the receiver at the cursor`.

The task's test command: `cargo test --locked -p nova-typeck`.

---
### Task 6: The probe lists the receiver's members and the locals in scope

Spec §4.1, §4.2 and §9.1, and decisions 3 and 4. Still nothing changes
without a probe.

**Files:**
- Modify: `crates/nova-typeck/src/check.rs`:
  - `ProbePending` and `probe_receiver` (from Task 5);
  - a fourth `Checker` field, `probe_locals_pending`, plus its initialiser;
  - `finalize_function`;
  - `check_path` (`:3428-3456`) and `check_block` (`:3078` and its loop);
  - new methods `probe_members`, `record_members`, `inherent_member`,
    `trait_members`, `probe_locals_*` and `scope_snapshot`;
  - the inline `mod tests`.

**Interfaces:**
- Consumes: Task 5's `ProbeResult`, `Member`, `MemberKind`, `probe_src` (the
  test helper) and `probe_receiver`.
- Produces: `ProbeResult.members` and `ProbeResult.locals`, filled as spec
  §4.1 and §4.2 say:
  - members: fields, inherent methods taking `self`, trait methods, a
    bound's methods, an array's `len`. A non-`pub` field or inherent method
    appears only in its own module, and the first of a name wins;
  - locals: innermost scope first, each name once, and within one scope in
    name order.

- [ ] **Step 1: Write the failing tests**

Add at the end of `mod tests`, after Task 5's tests:

```rust

    fn member_names(r: &CheckResult) -> Vec<&str> {
        r.probe.members.iter().map(|m| m.name.as_str()).collect()
    }

    #[test]
    fn members_of_a_record_in_its_own_module_include_private_fields() {
        let (r, _) = probe_src(
            "record P { x: Int, pub y: Int }\n\
             fn main() {\n    let p = P { x: 1, y: 2 }\n    p.<|>\n}\n",
        );
        assert_eq!(member_names(&r), ["x", "y"]);
        assert!(r.probe.members.iter().all(|m| m.kind == MemberKind::Field));
        assert!(r.probe.members.iter().all(|m| m.decl.is_some()));
    }

    #[test]
    fn members_of_a_std_vec_hide_its_internals_and_its_associated_functions() {
        let (r, _) = probe_src("fn main() {\n    let v: Vec<Int> = Vec::new()\n    v.<|>\n}\n");
        let names = member_names(&r);
        for m in ["push", "len", "get"] {
            assert!(names.contains(&m), "{m} missing from {names:?}");
        }
        // `data` is a std field without `pub`; `new` takes no `self`.
        assert!(!names.contains(&"data"), "{names:?}");
        assert!(!names.contains(&"new"), "{names:?}");
        let push = r.probe.members.iter().find(|m| m.name == "push").unwrap();
        assert_eq!(push.kind, MemberKind::Method);
        assert!(push.decl.is_some());
    }

    #[test]
    fn members_of_a_string_and_of_an_array() {
        let (s, _) = probe_src("fn main() {\n    let n = \"a\".<|>\n}\n");
        assert!(member_names(&s).contains(&"len"), "{:?}", member_names(&s));
        let (a, _) = probe_src("fn main() {\n    let n = [1, 2].<|>\n}\n");
        assert_eq!(member_names(&a), ["len"]);
        assert_eq!(a.probe.members[0].decl, None);
    }

    #[test]
    fn members_of_a_bounded_type_parameter_are_its_bounds_methods() {
        let (r, _) = probe_src("fn f<T: Display>(t: T) -> String {\n    t.<|>\n}\nfn main() {}\n");
        assert_eq!(member_names(&r), ["fmt"]);
    }

    #[test]
    fn members_include_the_methods_of_traits_the_type_implements() {
        let (r, _) = probe_src(
            "record P { x: Int }\n\
             impl Display for P {\n    fn fmt(self) -> String { \"p\" }\n}\n\
             fn main() {\n    let p = P { x: 1 }\n    p.<|>\n}\n",
        );
        let names = member_names(&r);
        assert!(names.contains(&"x") && names.contains(&"fmt"), "{names:?}");
    }

    #[test]
    fn the_probe_records_locals_on_an_empty_line() {
        let (r, _) = probe_src("fn main() {\n    let a = 1\n    let b = \"s\"\n    <|>\n}\n");
        assert_eq!(
            r.probe.locals,
            vec![("a".to_string(), Ty::Int), ("b".to_string(), Ty::String)]
        );
    }

    #[test]
    fn the_probe_records_locals_at_a_name_innermost_first_each_once() {
        let (r, _) = probe_src(
            "fn main(n: Int) {\n    let a = 1\n    if true {\n        let a = \"s\"\n        a<|>\n    }\n}\n",
        );
        assert_eq!(
            r.probe.locals,
            vec![("a".to_string(), Ty::String), ("n".to_string(), Ty::Int)]
        );
    }
```

`main(n: Int)` is a parameter list the checker accepts here, because the
probe tests check types, not MIR's E0601. If the checker rejects it, use a
helper `fn g(n: Int)` with the same body, and ledger the change.

- [ ] **Step 2: Run them to verify they fail**

```bash
cd /d/Projects/nona/nova && cargo test --locked -p nova-typeck --lib -- members_ locals 2>&1 | grep -E "^test .*(FAILED|ok)$" | head -10
```

Expected: the seven new tests FAIL (`members` and `locals` are empty),
because Task 5 fills only `receiver`.

- [ ] **Step 3: Record bounds and module with the receiver, and compute members**

Replace `ProbePending` with:

```rust
/// What the probe met, before its function's inference is finished.
struct ProbePending {
    receiver: Ty,
    /// The function's bounds, for a type-parameter receiver.
    bounds: Vec<Vec<DefId>>,
    /// The module the access is in, for the visibility rule.
    module: ModuleId,
}
```

Add a field to `struct Checker`, after `probe_pending: Option<ProbePending>,`:

```rust
    /// The locals in scope at the probe, with their types as inference had
    /// them then; read with the function's final substitution.
    probe_locals_pending: Option<Vec<(String, Ty)>>,
```

and `probe_locals_pending: None,` after `probe_pending: None,` in
`check_with`'s literal.

Change `probe_receiver` to take the function's context, and record the
bounds and module:

```rust
    fn probe_receiver(&mut self, fcx: &FnCtx, recv_ty: &Ty, receiver: Span, name: Span) {
        let Some(p) = self.probe else {
            return;
        };
        if self.probe_pending.is_some() || self.probe_result.receiver.is_some() {
            return;
        }
        if name.file == p.file && receiver.end <= p.offset && p.offset <= name.end {
            self.probe_pending = Some(ProbePending {
                receiver: recv_ty.clone(),
                bounds: fcx.param_bounds.clone(),
                module: self.cur_module,
            });
        }
    }
```

Keep its doc comment. Update its two callers to pass `fcx` first:
- `self.probe_receiver(fcx, &recv_ty, target.span, field.span);` in
  `check_field`;
- `self.probe_receiver(fcx, &recv_ty, receiver.span, method.span);` in
  `check_method_call`.

Replace the block Task 5 added to `finalize_function` with:

```rust
        // Spec 3.2 §4.1: what the probe met in this function, read with the
        // function's final substitution. The function is finalized before
        // the closures lifted from it, so it takes what the probe met even
        // inside one of them.
        if let Some(pending) = self.probe_pending.take() {
            let receiver = icx.apply(&pending.receiver);
            self.probe_result.members =
                self.probe_members(&receiver, &pending.bounds, pending.module);
            self.probe_result.receiver = Some(receiver);
        }
        if let Some(locals) = self.probe_locals_pending.take() {
            self.probe_result.locals = locals
                .into_iter()
                .map(|(name, ty)| (name, icx.apply(&ty)))
                .collect();
        }
```

Add these methods to `impl<'a> Checker<'a>`, beside `probe_receiver`:

```rust
    /// The members of `recv_ty` the probe offers, seen from module `from`
    /// (spec 3.2 §4.2):
    /// - its fields;
    /// - its inherent methods that take `self`;
    /// - the methods of the traits it implements;
    /// - for a type parameter, its bounds' methods;
    /// - for an array, `len`.
    ///
    /// A field or inherent method declared without `pub` is listed only
    /// inside its own module. The first of a name wins, so an inherent method
    /// hides a trait method of the same name, as `resolve_method_on` does.
    fn probe_members(&self, recv_ty: &Ty, bounds: &[Vec<DefId>], from: ModuleId) -> Vec<Member> {
        fn push(out: &mut Vec<Member>, m: Member) {
            if !out.iter().any(|o| o.name == m.name) {
                out.push(m);
            }
        }
        let mut out = Vec::new();
        match recv_ty {
            Ty::Param(k) => {
                for &tid in bounds.get(*k as usize).into_iter().flatten() {
                    for m in self.trait_members(tid) {
                        push(&mut out, m);
                    }
                }
            }
            Ty::Array(_) => push(
                &mut out,
                Member {
                    name: "len".to_string(),
                    kind: MemberKind::Method,
                    decl: None,
                },
            ),
            _ => {
                if let Ty::Record { def_id, .. } = recv_ty {
                    for m in self.record_members(*def_id, from) {
                        push(&mut out, m);
                    }
                }
                if let Some(head) = recv_ty.head() {
                    let fits = |i: &&hir::ImplInfo| {
                        i.self_head == head && i.match_args(recv_ty).is_some()
                    };
                    let inherent: Vec<DefId> = self
                        .impls
                        .iter()
                        .filter(fits)
                        .filter(|i| i.trait_id.is_none())
                        .flat_map(|i| i.methods.iter().map(|(_, d)| *d))
                        .filter(|d| !self.selfless.contains(d))
                        .collect();
                    for d in inherent {
                        if let Some(m) = self.inherent_member(d, from) {
                            push(&mut out, m);
                        }
                    }
                    let traits: Vec<DefId> = self
                        .impls
                        .iter()
                        .filter(fits)
                        .filter_map(|i| i.trait_id)
                        .collect();
                    for tid in traits {
                        for m in self.trait_members(tid) {
                            push(&mut out, m);
                        }
                    }
                }
            }
        }
        out
    }

    /// A record's fields, as `from` may see them.
    fn record_members(&self, record: DefId, from: ModuleId) -> Vec<Member> {
        let DefKind::Record { item_index } = self.defs.def(record).kind else {
            return Vec::new();
        };
        let ast::Item::Record(decl) = &self.file.items[item_index].value else {
            return Vec::new();
        };
        let own = self.defs.module_of(item_index) == from;
        decl.fields
            .iter()
            .filter(|f| own || matches!(f.vis, ast::Visibility::Pub))
            .map(|f| Member {
                name: f.name.value.clone(),
                kind: MemberKind::Field,
                decl: Some(f.ty.span),
            })
            .collect()
    }

    /// An inherent method, if `from` may see it.
    fn inherent_member(&self, method: DefId, from: ModuleId) -> Option<Member> {
        let loc = self.method_locs.get(&method)?;
        let ast::Item::Impl(block) = &self.file.items[loc.item_index].value else {
            return None;
        };
        let f = block.functions.get(loc.method_index)?;
        let own = self.defs.module_of(loc.item_index) == from;
        (own || matches!(f.vis, ast::Visibility::Pub)).then(|| Member {
            name: f.name.value.clone(),
            kind: MemberKind::Method,
            decl: Some(f.name.span),
        })
    }

    /// A trait's methods that take `self`, each with its name's span in the
    /// trait's declaration.
    fn trait_members(&self, trait_id: DefId) -> Vec<Member> {
        let Some(t) = self.traits.iter().find(|t| t.def_id == trait_id) else {
            return Vec::new();
        };
        let mut spans: FxHashMap<&str, Span> = FxHashMap::default();
        if let DefKind::Trait { item_index } = self.defs.def(trait_id).kind {
            if let ast::Item::Trait(decl) = &self.file.items[item_index].value {
                for item in &decl.items {
                    match item {
                        TraitItem::Required(sig) => {
                            spans.insert(sig.name.value.as_str(), sig.name.span);
                        }
                        TraitItem::Provided(f) => {
                            spans.insert(f.name.value.as_str(), f.name.span);
                        }
                        _ => {}
                    }
                }
            }
        }
        t.methods
            .iter()
            .filter(|m| m.has_self)
            .map(|m| Member {
                name: m.name.clone(),
                kind: MemberKind::Method,
                decl: spans.get(m.name.as_str()).copied(),
            })
            .collect()
    }

    /// The locals in scope now, innermost first, each name once, and within
    /// one scope in name order, with their types as inference has them.
    fn scope_snapshot(fcx: &FnCtx) -> Vec<(String, Ty)> {
        let mut seen: FxHashSet<&str> = FxHashSet::default();
        let mut out = Vec::new();
        for scope in fcx.scopes.iter().rev() {
            let mut names: Vec<(&String, &LocalId)> = scope.iter().collect();
            names.sort_by(|a, b| a.0.cmp(b.0));
            for (name, id) in names {
                if seen.insert(name.as_str()) {
                    out.push((name.clone(), fcx.locals[id.0 as usize].ty.clone()));
                }
            }
        }
        out
    }

    /// Record the locals in scope if the probe's offset lies in `lo..=hi`
    /// of `file` (decision 4). The first record wins.
    fn probe_locals(&mut self, fcx: &FnCtx, file: nova_diagnostics::FileId, lo: u32, hi: u32) {
        let Some(p) = self.probe else {
            return;
        };
        if self.probe_locals_pending.is_some() || !self.probe_result.locals.is_empty() {
            return;
        }
        if file == p.file && lo <= p.offset && p.offset <= hi {
            self.probe_locals_pending = Some(Self::scope_snapshot(fcx));
        }
    }
```

- [ ] **Step 4: Record locals at a name, before a statement and at a block's end**

In `check_path`, after `let name = path.segments[0].value.as_str();`, add:

```rust
        // Spec 3.2 §4.1: a name at the probe sees these locals.
        self.probe_locals(fcx, span.file, span.start, span.end);
```

In `check_block`, after `fcx.scopes.push(FxHashMap::default());`, the loop
reads `for stmt in &block.stmts {`. Make its first line:

```rust
        for stmt in &block.stmts {
            // Decision 4: the probe, between the block's start and this
            // statement, sees the scope as it is before the statement.
            self.probe_locals(fcx, span.file, span.start, stmt.span.start);
```

Before the block's scope is popped, after the loop and the trailing
expression (find the `fcx.scopes.pop();` that closes `check_block`), add:

```rust
        // Decision 4: anywhere else in the block sees all of its locals.
        self.probe_locals(fcx, span.file, span.start, span.end);
```

If `check_block` returns from more than one place before popping, add the
call before each `fcx.scopes.pop()` in it, and ledger how many there were.

- [ ] **Step 5: Run the type checker's tests to verify they pass**

```bash
cd /d/Projects/nona/nova && P=/c/Users/SAKEER~1/AppData/Local/Temp/gcm/p32 && cargo test --locked -p nova-typeck > $P/t6.txt 2>&1; echo "exit=$?"; python -X utf8 $P/count.py $P/t6.txt; grep -E "FAILED|panicked" $P/t6.txt | head
```

Expected: `exit=0` and 0 failed, the seven new tests included.

- [ ] **Step 6: Commit**

Write `$P/msg-6.txt`:

```
nova-typeck: the probe lists members and locals

For completion (spec
docs/superpowers/specs/2026-10-08-phase-3-2-lsp-core-design.md §4.1,
§4.2):
- With the receiver, the probe lists its members:
  - fields;
  - inherent methods taking `self`;
  - the methods of the traits it implements;
  - a bound's methods, for a type parameter;
  - `len`, for an array.

  A field or inherent method without `pub` appears only in its own
  module, which hides std internals like `Vec`'s `data`. Each member
  carries its declaration's span, for the server to show.
- The locals in scope are recorded at a name that touches the offset,
  before the first statement after it, or at the end of the block that
  holds it, so an empty line gets them too.

Both are read with the function's final substitution.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
```

```bash
cd /d/Projects/nona/nova && P=/c/Users/SAKEER~1/AppData/Local/Temp/gcm/p32 && cargo fmt --all && git add crates/nova-typeck && git commit -q -F $P/msg-6.txt && git log -1 --format=%s
```

Expected: `nova-typeck: the probe lists members and locals`.

The task's test command: `cargo test --locked -p nova-typeck`.

---
### Task 7: `nova_driver::analyze`, the front end for the language server

Spec §3.1 to §3.3, §9.1, and decision 1.

- **One loader serves both paths:** `load_program(entry, &dyn Sources, &mut FileDb)`.
  - The CLI's `FrontendContext` now calls it with `DiskSources`, and renders
    what it returns: the same diagnostics, in the same order, with the same
    "failed to read" error.
  - `analyze` calls it with the server's buffers.
- **`analyze` has three switches:**
  - `keep_going` runs every stage, and removes the E0001s about dropped
    names;
  - `tests` keeps `@test` functions and adds `std/test`;
  - `module_only` skips MIR.

**Files:**
- Create: `crates/nova-driver/src/analyze.rs`
- Modify: `crates/nova-driver/src/lib.rs`:
  - the module and its re-exports, after `mod runtime_cache;`;
  - `FrontendContext::load_modules`, `:482-554`, whose body is replaced.
- Create: `crates/nova-driver/tests/analyze.rs`

**Interfaces:**
- Consumes:
  - Task 3's `nova_parser::parse_recovering` and `Parsed`;
  - Task 5's `nova_typeck::{check_with, CheckOptions, ProbePoint, ProbeResult}`.
- Produces:
  - `pub trait Sources { fn read(&self, path: &Path) -> std::io::Result<String>; fn same_file(&self, a: &Path, b: &Path) -> bool { a == b } }`;
  - `pub struct DiskSources;` (implements `Sources`);
  - `#[derive(Debug, Clone, Default)] pub struct Options { pub keep_going: bool, pub tests: bool, pub module_only: bool, pub probe: Option<Probe> }`;
  - `#[derive(Debug, Clone)] pub struct Probe { pub path: PathBuf, pub offset: u32 }`;
  - `pub struct Analysis { pub db: FileDb, pub diagnostics: Vec<Diagnostic>, pub modules: Vec<(FileId, PathBuf)>, pub definitions: Option<Definitions>, pub module: Option<nova_hir::Module>, pub probe: ProbeResult }`,
    where `modules[i]` is `ModuleId(i)`;
  - `pub fn analyze(entry: &Path, sources: &dyn Sources, options: &Options) -> std::io::Result<Analysis>`.
    It is `Err` only when the entry cannot be read.

- [ ] **Step 1: Write the failing tests**

`crates/nova-driver/tests/analyze.rs`:

```rust
//! `analyze` (spec
//! `docs/superpowers/specs/2026-10-08-phase-3-2-lsp-core-design.md` §3,
//! §9.1).

use std::io;
use std::path::{Path, PathBuf};

use nova_driver::{analyze, Analysis, Options, Outcome, Probe, Sources};

/// Buffers by path, then the disk: what the language server's overlay does.
struct Buffers(Vec<(PathBuf, String)>);

impl Sources for Buffers {
    fn read(&self, path: &Path) -> io::Result<String> {
        match self.0.iter().find(|(p, _)| p == path) {
            Some((_, text)) => Ok(text.clone()),
            None => std::fs::read_to_string(path),
        }
    }
}

fn mem(files: &[(&str, &str)]) -> Buffers {
    Buffers(
        files
            .iter()
            .map(|(p, t)| (PathBuf::from(p), t.to_string()))
            .collect(),
    )
}

fn keep_going() -> Options {
    Options {
        keep_going: true,
        tests: true,
        ..Options::default()
    }
}

fn codes(a: &Analysis) -> Vec<&str> {
    a.diagnostics.iter().map(|d| d.code.as_str()).collect()
}

/// A fresh directory with a fixed name, so each run replaces the last.
fn fresh_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("nova-analyze-{name}"));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

const BOTH: &str = "fn f() {\n    let x = (\n}\nfn g() -> Int { \"s\" }\nfn main() {}\n";

#[test]
fn keep_going_reports_a_syntax_error_and_a_type_error_together() {
    let a = analyze(Path::new("mem/main.nova"), &mem(&[("mem/main.nova", BOTH)]), &keep_going())
        .unwrap();
    let codes = codes(&a);
    assert!(codes.contains(&"P0001"), "{codes:?}");
    assert!(codes.contains(&"E0010"), "{codes:?}");
}

#[test]
fn check_file_still_stops_at_the_syntax_error() {
    let dir = fresh_dir("staged");
    let file = dir.join("main.nova");
    std::fs::write(&file, BOTH).unwrap();
    match nova_driver::check_file(&file).unwrap() {
        Outcome::Failed { errors } => assert_eq!(errors, 1, "only the syntax error"),
        Outcome::Ok(()) => panic!("check_file accepted a syntax error"),
    }
}

#[test]
fn a_buffer_beats_the_file_on_disk() {
    let dir = fresh_dir("buffer");
    let file = dir.join("main.nova");
    std::fs::write(&file, "fn main() { let x: Int = \"s\" }\n").unwrap();
    let clean = Buffers(vec![(file.clone(), "fn main() {}\n".to_string())]);
    let a = analyze(&file, &clean, &keep_going()).unwrap();
    assert!(a.diagnostics.is_empty(), "{:?}", codes(&a));
}

#[test]
fn a_never_saved_buffer_is_checked() {
    let a = analyze(
        Path::new("nowhere/never/saved.nova"),
        &mem(&[("nowhere/never/saved.nova", "fn main() { let x: Int = \"s\" }\n")]),
        &keep_going(),
    )
    .unwrap();
    assert_eq!(codes(&a), ["E0010"]);
}

#[test]
fn an_unreadable_entry_is_an_error() {
    assert!(analyze(Path::new("nowhere/missing.nova"), &mem(&[]), &keep_going()).is_err());
}

#[test]
fn mir_runs_on_a_clean_program_unless_module_only() {
    let src = "fn helper() -> Int { 1 }\n";
    let program = analyze(Path::new("m/lib.nova"), &mem(&[("m/lib.nova", src)]), &keep_going())
        .unwrap();
    assert_eq!(codes(&program), ["E0601"], "MIR's no-`main` check");
    let module = Options {
        module_only: true,
        ..keep_going()
    };
    let a = analyze(Path::new("m/lib.nova"), &mem(&[("m/lib.nova", src)]), &module).unwrap();
    assert!(a.diagnostics.is_empty(), "{:?}", codes(&a));
}

#[test]
fn with_tests_a_test_body_is_checked() {
    let src = "@test\nfn t() {\n    let x: Int = \"s\"\n}\nfn main() {}\n";
    let a = analyze(Path::new("t/main.nova"), &mem(&[("t/main.nova", src)]), &keep_going())
        .unwrap();
    assert_eq!(codes(&a), ["E0010"]);
    let no_tests = Options {
        tests: false,
        ..keep_going()
    };
    let a = analyze(Path::new("t/main.nova"), &mem(&[("t/main.nova", src)]), &no_tests).unwrap();
    assert!(a.diagnostics.is_empty(), "{:?}", codes(&a));
}

#[test]
fn a_dropped_name_raises_no_e0001_at_any_site() {
    // Each item breaks right after its name, so item-level recovery resumes
    // at the next item (a `record P {` left open would instead swallow
    // everything up to the next `}`). Each use is of a kind with its own
    // E0001 message (decision 1).
    let main = "import lib::{area}\n\
                fn f(x: Int {\n}\n\
                record P %\n\
                trait T %\n\
                fn main() {\n    f(1)\n    let g = f\n    let p: P = P { x: 1 }\n}\n\
                impl T for Int {}\n";
    let lib = "pub fn area(r: Int {\n}\n";
    let a = analyze(
        Path::new("d/main.nova"),
        &mem(&[("d/main.nova", main), ("d/lib.nova", lib)]),
        &keep_going(),
    )
    .unwrap();
    let codes = codes(&a);
    assert!(codes.contains(&"P0001"), "{codes:?}");
    assert!(!codes.contains(&"E0001"), "{:?}", a.diagnostics);
}

#[test]
fn a_name_never_declared_still_raises_e0001() {
    let src = "fn main() {\n    nothing(1)\n}\n";
    let a = analyze(Path::new("u/main.nova"), &mem(&[("u/main.nova", src)]), &keep_going())
        .unwrap();
    assert_eq!(codes(&a), ["E0001"]);
}

#[test]
fn modules_are_numbered_in_load_order() {
    let a = analyze(
        Path::new("o/main.nova"),
        &mem(&[
            ("o/main.nova", "import geometry\nfn main() {}\n"),
            ("o/geometry.nova", "pub fn area() -> Int { 1 }\n"),
        ]),
        &keep_going(),
    )
    .unwrap();
    assert!(a.diagnostics.is_empty(), "{:?}", codes(&a));
    assert_eq!(a.modules.len(), 2);
    assert_eq!(a.modules[0].1, PathBuf::from("o/main.nova"));
    assert!(a.modules[1].1.ends_with("geometry.nova"));
    let defs = a.definitions.as_ref().unwrap();
    assert!(defs
        .names_in_scope(nova_resolver::ModuleId(1))
        .iter()
        .any(|(n, _)| n == "area"));
}

#[test]
fn the_probe_reaches_an_imported_module() {
    let geometry = "pub fn area(s: String) -> Int {\n    s.\n}\n";
    let offset = geometry.find("s.").unwrap() as u32 + 2;
    let options = Options {
        probe: Some(Probe {
            path: PathBuf::from("q/geometry.nova"),
            offset,
        }),
        ..keep_going()
    };
    let a = analyze(
        Path::new("q/main.nova"),
        &mem(&[
            ("q/main.nova", "import geometry\nfn main() {}\n"),
            ("q/geometry.nova", geometry),
        ]),
        &options,
    )
    .unwrap();
    assert_eq!(a.probe.receiver, Some(nova_hir::Ty::String));
}
```

Two tests name more crates: `nova_resolver::ModuleId` and `nova_hir::Ty`.
Both are normal dependencies of `nova-driver`, so integration tests can use
them.

- [ ] **Step 2: Run them to verify they fail**

```bash
cd /d/Projects/nona/nova && cargo test --locked -p nova-driver --test analyze 2>&1 | grep -E "error\[" | head -4
```

Expected: `error[E0432]: unresolved imports` naming `analyze`, `Analysis`,
`Options`, `Probe` and `Sources`.

- [ ] **Step 3: Write `analyze.rs`**

`crates/nova-driver/src/analyze.rs`:

```rust
//! The front end for the language server (spec
//! `docs/superpowers/specs/2026-10-08-phase-3-2-lsp-core-design.md` §3).
//!
//! `analyze` runs what `check_file` runs, with three differences:
//! - its sources come through [`Sources`], so an editor's unsaved buffers
//!   are read;
//! - with `keep_going`, every stage runs whatever the earlier ones found;
//! - its diagnostics are returned rather than printed.
//!
//! The CLI's entry points share [`load_program`] with it, through
//! [`DiskSources`], and keep their own staged behaviour.

use std::collections::{HashSet, VecDeque};
use std::path::{Path, PathBuf};

use nova_diagnostics::{Diagnostic, FileDb, FileId, Severity};
use nova_resolver::{Definitions, ModuleSource};
use nova_typeck::{CheckOptions, ProbePoint, ProbeResult};

/// Where `analyze` reads a module's text.
pub trait Sources {
    /// The text of `path`: an editor's buffer if one is open, else the file.
    fn read(&self, path: &Path) -> std::io::Result<String>;

    /// Whether `a` and `b` name the same file. The default compares them as
    /// written; the language server compares normalised paths.
    fn same_file(&self, a: &Path, b: &Path) -> bool {
        a == b
    }
}

/// Reads every module from disk: the CLI's sources.
pub struct DiskSources;

impl Sources for DiskSources {
    fn read(&self, path: &Path) -> std::io::Result<String> {
        std::fs::read_to_string(path)
    }
}

/// What `analyze` does beyond checking.
#[derive(Debug, Clone, Default)]
pub struct Options {
    /// Run every stage whatever the earlier ones found (spec §3.2).
    pub keep_going: bool,
    /// Keep `@test` functions and put `std/test` in scope, as `nova test`
    /// does (spec §13, decision 14).
    pub tests: bool,
    /// Check the entry as a module, not a program: no MIR lowering (spec
    /// §3.3).
    pub module_only: bool,
    /// Record what is at this place (spec §4.1).
    pub probe: Option<Probe>,
}

/// A place in one of the program's files.
#[derive(Debug, Clone)]
pub struct Probe {
    pub path: PathBuf,
    pub offset: u32,
}

/// What `analyze` found.
pub struct Analysis {
    /// Every file read, std included.
    pub db: FileDb,
    pub diagnostics: Vec<Diagnostic>,
    /// The program's own modules, by the paths they were read from, in load
    /// order: `modules[i]` is `ModuleId(i)`.
    pub modules: Vec<(FileId, PathBuf)>,
    pub definitions: Option<Definitions>,
    /// The typed module, partial when errors were found.
    pub module: Option<nova_hir::Module>,
    pub probe: ProbeResult,
}

/// One module [`load_program`] read.
pub(crate) struct Loaded {
    pub name: String,
    pub path: PathBuf,
    pub file: FileId,
    pub ast: nova_ast::File,
}

/// Read, lex and parse `entry` and every module it transitively imports,
/// each `<name>.nova` beside the entry. A module that cannot be read is
/// skipped: the resolver reports its `import`.
///
/// Returns:
/// - the modules, in load order;
/// - their lex and parse diagnostics, in the order the CLI has always
///   printed them;
/// - the names of the items the parser dropped.
///
/// `Err` only when the entry itself cannot be read.
pub(crate) fn load_program(
    entry: &Path,
    sources: &dyn Sources,
    db: &mut FileDb,
) -> std::io::Result<(Vec<Loaded>, Vec<Diagnostic>, Vec<String>)> {
    let dir = entry.parent().map(Path::to_path_buf).unwrap_or_default();
    let mut out: Vec<Loaded> = Vec::new();
    let mut diagnostics = Vec::new();
    let mut dropped = Vec::new();
    let mut seen: HashSet<String> = HashSet::new();
    let mut queue: VecDeque<(String, PathBuf)> = VecDeque::new();
    queue.push_back((crate::FrontendContext::module_name(entry), entry.to_path_buf()));

    while let Some((name, path)) = queue.pop_front() {
        if !seen.insert(name.clone()) {
            continue;
        }
        let is_entry = out.is_empty() && seen.len() == 1;
        let source = match sources.read(&path) {
            Ok(s) => s,
            Err(e) if is_entry => return Err(e),
            // A missing imported module: skip; the resolver flags the import.
            Err(_) => continue,
        };
        let file = db.add(path.display().to_string(), source.as_str());

        let (tokens, lex_errors) = nova_lexer::lex(&source, file);
        diagnostics.extend(lex_errors.iter().map(|e| {
            Diagnostic::error("L0001", e.to_string()).with_primary_label(e.span(), "here")
        }));
        let parsed = nova_parser::parse_recovering(&tokens, file);
        diagnostics.extend(parsed.errors.iter().map(|e| {
            Diagnostic::error("P0001", e.to_string()).with_primary_label(e.span(), "here")
        }));
        dropped.extend(parsed.dropped.into_iter().map(|n| n.value));

        // Queue imported modules, resolved beside the entry. Only
        // single-segment imports name a module file; the resolver rejects
        // the others.
        for item in &parsed.file.items {
            if let nova_ast::Item::Import(imp) = &item.value {
                if let [seg] = imp.path.value.segments.as_slice() {
                    let mod_name = seg.value.clone();
                    if !seen.contains(&mod_name) {
                        let mod_path = dir.join(format!("{mod_name}.nova"));
                        queue.push_back((mod_name, mod_path));
                    }
                }
            }
        }
        out.push(Loaded {
            name,
            path,
            file,
            ast: parsed.file,
        });
    }
    Ok((out, diagnostics, dropped))
}

/// Run the front end on `entry` for the language server (spec §3).
pub fn analyze(entry: &Path, sources: &dyn Sources, options: &Options) -> std::io::Result<Analysis> {
    let mut db = FileDb::new();
    let (loaded, mut diagnostics, dropped) = load_program(entry, sources, &mut db)?;
    let mut analysis = Analysis {
        db,
        diagnostics: Vec::new(),
        modules: loaded.iter().map(|m| (m.file, m.path.clone())).collect(),
        definitions: None,
        module: None,
        probe: ProbeResult::default(),
    };
    let stop = |diags: &[Diagnostic]| !options.keep_going && has_error(diags);
    if stop(&diagnostics) {
        analysis.diagnostics = diagnostics;
        return Ok(analysis);
    }

    let mut files: Vec<(String, nova_ast::File)> =
        loaded.into_iter().map(|m| (m.name, m.ast)).collect();
    if !options.tests {
        crate::strip_test_functions(&mut files);
    }
    let std_files: Vec<FileId> = nova_resolver::STD_MODULES
        .iter()
        .map(|&(name, src)| {
            let short = name.strip_prefix("$std.").unwrap_or(name);
            analysis.db.add(format!("<std/{short}>"), src)
        })
        .collect();
    let extra_std = options.tests.then(|| {
        let (name, src) = nova_resolver::STD_TEST_MODULE;
        let short = name.strip_prefix("$std.").unwrap_or(name);
        let file = analysis.db.add(format!("<std/{short}>"), src);
        (nova_resolver::STD_TEST_MODULE, file)
    });
    let module_sources: Vec<ModuleSource> = files
        .iter()
        .map(|(name, file)| ModuleSource {
            name: name.clone(),
            file,
        })
        .collect();
    let resolved = nova_resolver::resolve_program(&module_sources, &std_files, extra_std);
    diagnostics.extend(resolved.diagnostics);
    if stop(&diagnostics) {
        analysis.diagnostics = diagnostics;
        analysis.definitions = Some(resolved.definitions);
        return Ok(analysis);
    }

    let probe = options.probe.as_ref().and_then(|p| {
        analysis
            .modules
            .iter()
            .find(|(_, path)| sources.same_file(path, &p.path))
            .map(|(file, _)| ProbePoint {
                file: *file,
                offset: p.offset,
            })
    });
    let checked =
        nova_typeck::check_with(&resolved.file, &resolved.definitions, &CheckOptions { probe });
    diagnostics.extend(checked.diagnostics);
    // MIR lowering assumes a well-formed program (spec §3.3).
    if !options.module_only && !has_error(&diagnostics) {
        if let Err(mir) = nova_mir::lower_module(&checked.module) {
            diagnostics.extend(mir);
        }
    }
    if options.keep_going {
        diagnostics.retain(|d| !about_a_dropped_name(d, &dropped));
    }
    analysis.diagnostics = diagnostics;
    analysis.definitions = Some(resolved.definitions);
    analysis.module = Some(checked.module);
    analysis.probe = checked.probe;
    Ok(analysis)
}

fn has_error(diagnostics: &[Diagnostic]) -> bool {
    diagnostics.iter().any(|d| d.severity == Severity::Error)
}

/// Whether `d` is an E0001 about an item the parser dropped mid-edit (spec
/// §3.2): its message's first backticked name is that item's.
///
/// Every E0001 message names what it cannot find first, in backticks:
/// - "cannot find `x` in this scope";
/// - "cannot find function `f` in this scope";
/// - "cannot find type `T`", "cannot find record `R`", "cannot find trait
///   `T`";
/// - "`x` is not a public item of module `m`".
///
/// `tests/analyze.rs` pins each of them.
fn about_a_dropped_name(d: &Diagnostic, dropped: &[String]) -> bool {
    if d.code != "E0001" {
        return false;
    }
    let mut parts = d.message.split('`');
    parts.next();
    parts
        .next()
        .is_some_and(|name| dropped.iter().any(|n| n == name))
}
```

- [ ] **Step 4: Route the CLI's loading through `load_program`**

In `crates/nova-driver/src/lib.rs`, after `mod runtime_cache;`:

```rust
mod analyze;

pub use analyze::{analyze, Analysis, DiskSources, Options, Probe, Sources};
```

Replace the whole of `FrontendContext::load_modules`, its doc comment
included, with:

```rust
    /// Load, lex, and parse the entry module plus every module it transitively
    /// `import`s (resolved to `<name>.nova` beside the entry), from disk. A
    /// module whose file is missing is skipped here; the resolver reports the
    /// dangling import against its `import` site. Lex and parse diagnostics
    /// are rendered. The loading itself is [`analyze::load_program`], which
    /// the language server shares.
    fn load_modules(&mut self) -> Result<Vec<(String, nova_ast::File)>> {
        let (loaded, diagnostics, _dropped) =
            analyze::load_program(&self.entry, &DiskSources, &mut self.db)
                .with_context(|| format!("failed to read {}", self.entry.display()))?;
        self.render(&diagnostics);
        Ok(loaded.into_iter().map(|m| (m.name, m.ast)).collect())
    }
```

`FrontendContext::module_name` and `strip_test_functions` stay where they
are. `analyze.rs` is a child module, so it may call them.

The diagnostics come out in the same order as before: each module's lex
diagnostics, then its parse diagnostics, then the next module's.

- [ ] **Step 5: Run the driver's tests to verify they pass**

```bash
cd /d/Projects/nona/nova && P=/c/Users/SAKEER~1/AppData/Local/Temp/gcm/p32 && cargo test --locked -p nova-driver > $P/t7.txt 2>&1; echo "exit=$?"; python -X utf8 $P/count.py $P/t7.txt; grep -E "FAILED|panicked" $P/t7.txt | head
```

Expected: `exit=0` and 0 failed, the eleven new tests included.

If `a_dropped_name_raises_no_e0001_at_any_site` fails on a remaining E0001,
read its message. If its first backticked word is not the dropped name,
the message breaks decision 1's rule: ledger it, and widen
`about_a_dropped_name` to cover that message. Other diagnostics from the
broken items, such as E0014 on `impl T for Int`, are allowed; the test
asserts only that no E0001 survives.

- [ ] **Step 6: Run the CLI's tests**

The CLI's output must not change.

```bash
cd /d/Projects/nona/nova && P=/c/Users/SAKEER~1/AppData/Local/Temp/gcm/p32 && cargo test --locked -p nova-cli > $P/t7-cli.txt 2>&1; echo "exit=$?"; python -X utf8 $P/count.py $P/t7-cli.txt; grep -E "FAILED|panicked" $P/t7-cli.txt | head
```

Expected: `exit=0` and 0 failed (port 3000 free first, per the
Conventions).

- [ ] **Step 7: Commit**

Write `$P/msg-7.txt`:

```
nova-driver: analyze, the front end for the language server

`analyze(entry, sources, options)` runs what `check_file` runs, for the
language server (spec
docs/superpowers/specs/2026-10-08-phase-3-2-lsp-core-design.md §3):
- Sources come through `Sources`, so an editor's unsaved buffers are
  read.
- `keep_going` runs every stage whatever the earlier ones found, and
  drops each E0001 about an item the parser dropped mid-edit.
- `tests` keeps `@test` functions and adds std/test.
- `module_only` skips MIR, for a file that is a module, not a program.
- An optional probe reaches the type checker.

Diagnostics are returned, with the files and the typed module.

The CLI's loading is now `load_program` with `DiskSources`, rendered as
before: the same diagnostics, the same order, the same errors. The CLI's
entry points still stop at the first failing stage.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
```

```bash
cd /d/Projects/nona/nova && P=/c/Users/SAKEER~1/AppData/Local/Temp/gcm/p32 && cargo fmt --all && git add crates/nova-driver && git commit -q -F $P/msg-7.txt && git log -1 --format=%s
```

Expected: `nova-driver: analyze, the front end for the language server`.

The task's test command:
`cargo test --locked -p nova-driver -p nova-cli` (port 3000 free).

---
### Task 8: Broken programs never panic or hang

Spec §9.2 and §12 risk 1. The front end was written for finished programs,
and Tasks 3 to 7 feed it unfinished ones. This test cuts real programs at
many points and analyses each.

**Files:**
- Create: `crates/nova-driver/tests/broken.rs`

**Interfaces:**
- Consumes: Task 7's `analyze`, `Options` and `Sources`.
- Produces: nothing; it is a test.

- [ ] **Step 1: Write the test**

`crates/nova-driver/tests/broken.rs`:

```rust
//! Broken programs never panic or hang (spec
//! `docs/superpowers/specs/2026-10-08-phase-3-2-lsp-core-design.md` §9.2).
//!
//! Each example, cut at 16 evenly spaced character boundaries, and each
//! `tests/runtime/*.nova`, cut at 2, is analysed with `keep_going`, as the
//! language server analyses a file being typed. Each analysis must return
//! within 10 s without panicking.

use std::path::{Path, PathBuf};
use std::sync::{mpsc, Arc, Mutex};
use std::time::{Duration, Instant};

use nova_driver::{analyze, Options, Sources};

/// One file's text in place of its contents; every other file from disk.
struct Cut {
    path: PathBuf,
    text: String,
}

impl Sources for Cut {
    fn read(&self, path: &Path) -> std::io::Result<String> {
        if path == self.path {
            Ok(self.text.clone())
        } else {
            std::fs::read_to_string(path)
        }
    }
}

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// `text` cut at `n` evenly spaced character boundaries, never at its end.
fn cuts(text: &str, n: usize) -> Vec<String> {
    let bounds: Vec<usize> = text.char_indices().map(|(i, _)| i).collect();
    if bounds.is_empty() {
        return Vec::new();
    }
    (1..=n)
        .map(|k| text[..bounds[(bounds.len() * k / (n + 1)).min(bounds.len() - 1)]].to_string())
        .collect()
}

#[test]
fn cut_programs_never_panic_or_hang() {
    let root = root();
    let mut programs: Vec<(PathBuf, usize)> = Vec::new();
    for entry in std::fs::read_dir(root.join("examples")).unwrap() {
        let main = entry.unwrap().path().join("src").join("main.nova");
        if main.is_file() {
            programs.push((main, 16));
        }
    }
    for entry in std::fs::read_dir(root.join("tests").join("runtime")).unwrap() {
        let path = entry.unwrap().path();
        if path.extension().is_some_and(|e| e == "nova") {
            programs.push((path, 2));
        }
    }
    programs.sort();
    let mut jobs: Vec<(PathBuf, String)> = Vec::new();
    for (path, n) in &programs {
        let text = std::fs::read_to_string(path).unwrap();
        for cut in cuts(&text, *n) {
            jobs.push((path.clone(), cut));
        }
    }
    // 6 examples x 16 + 134 runtime programs x 2 = 364, on 2026-10-08.
    assert!(jobs.len() >= 360, "only {} cut programs", jobs.len());
    let total = jobs.len();

    let threads = std::thread::available_parallelism().map_or(4, |n| n.get()).min(8);
    let queue = Arc::new(Mutex::new(jobs));
    // What each thread is analysing, for the report if one hangs.
    let current: Arc<Mutex<Vec<Option<(PathBuf, usize)>>>> =
        Arc::new(Mutex::new(vec![None; threads]));
    let (done, results) = mpsc::channel();
    for t in 0..threads {
        let queue = Arc::clone(&queue);
        let current = Arc::clone(&current);
        let done = done.clone();
        std::thread::spawn(move || loop {
            let Some((path, text)) = queue.lock().unwrap().pop() else {
                break;
            };
            current.lock().unwrap()[t] = Some((path.clone(), text.len()));
            let options = Options {
                keep_going: true,
                tests: true,
                ..Options::default()
            };
            let sources = Cut {
                path: path.clone(),
                text: text.clone(),
            };
            let started = Instant::now();
            let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                analyze(&path, &sources, &options).map(|a| a.diagnostics.len())
            }));
            let took = started.elapsed();
            current.lock().unwrap()[t] = None;
            if done.send((path, text.len(), outcome.is_ok(), took)).is_err() {
                break;
            }
        });
    }
    drop(done);

    let mut failures: Vec<String> = Vec::new();
    for _ in 0..total {
        match results.recv_timeout(Duration::from_secs(30)) {
            Ok((path, len, ok, took)) => {
                if !ok {
                    failures.push(format!("panicked: {} cut at byte {len}", path.display()));
                } else if took > Duration::from_secs(10) {
                    failures.push(format!("took {took:?}: {} cut at byte {len}", path.display()));
                }
            }
            Err(_) => {
                let stuck: Vec<String> = current
                    .lock()
                    .unwrap()
                    .iter()
                    .flatten()
                    .map(|(p, len)| format!("{} cut at byte {len}", p.display()))
                    .collect();
                panic!("no analysis finished in 30 s; in flight: {stuck:?}");
            }
        }
    }
    assert!(failures.is_empty(), "{} of {total}:\n{}", failures.len(), failures.join("\n"));
}
```

- [ ] **Step 2: Run it**

```bash
cd /d/Projects/nona/nova && P=/c/Users/SAKEER~1/AppData/Local/Temp/gcm/p32 && cargo test --locked -p nova-driver --test broken > $P/t8.txt 2>&1; echo "exit=$?"; grep -E "^test |panicked:|took |in flight" $P/t8.txt | head -20
```

Expected: `test cut_programs_never_panic_or_hang ... ok` and `exit=0`.

This test is written after the code it checks, on purpose: it hunts for
crashes that Tasks 3 to 7 may have left, and no implementation is "missing"
here. **If it reports a panic or a hang, that is a real defect.** For each
one:
1. Use superpowers:systematic-debugging to reproduce it with the printed
   path and byte.
2. Add that cut as a unit test in the crate that panics, and watch it fail.
3. Fix the code. Never catch the panic in the test.
4. Ledger each fix as
   `Task 8: fixed <panic> at <file:line> — <test name> RED→GREEN`.

If it passes on its first run, prove that it can fail. Add
`panic!("mutant")` at the top of `nova_driver::analyze`, run Step 2, and
expect it to report 364 panics. Then remove the mutant with
`git checkout -- crates/nova-driver/src/analyze.rs`.

- [ ] **Step 3: Commit**

Write `$P/msg-8.txt`:

```
nova-driver: broken programs never panic or hang

Every example, cut at 16 points, and every tests/runtime program, cut
at 2 (364 programs on 2026-10-08), is analysed with keep_going, as the
language server analyses a file being typed (spec
docs/superpowers/specs/2026-10-08-phase-3-2-lsp-core-design.md §9.2).
Each must return within 10 s without a panic.

The cuts fall on character boundaries, the analyses run across up to 8
threads, and a hang reports the cut in flight. A floor on the count
keeps the test from passing over an empty corpus.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
```

```bash
cd /d/Projects/nona/nova && P=/c/Users/SAKEER~1/AppData/Local/Temp/gcm/p32 && cargo fmt --all && git add crates/nova-driver && git commit -q -F $P/msg-8.txt && git log -1 --format=%s
```

Expected: `nova-driver: broken programs never panic or hang`.

The task's test command: `cargo test --locked -p nova-driver`.

---
### Task 9: `nova_fmt::format_buffer`, formatting an editor's buffer

Spec §6.5 and §9.1, and Review Focus 1. `format_file` keeps its behaviour:
after reading the file, it now calls `format_buffer`.

**Files:**
- Modify: `crates/nova-fmt/src/file.rs:67-89` (`format_file`, and the new
  `format_buffer`)
- Modify: `crates/nova-fmt/src/lib.rs:12` (export it)
- Test: `crates/nova-fmt/tests/files.rs` (append)

**Interfaces:**
- Consumes: nothing.
- Produces: `nova_fmt::format_buffer(path: &Path, text: &str) -> Result<String, FormatError>`.

- [ ] **Step 1: Write the failing tests**

Append to `crates/nova-fmt/tests/files.rs`:

```rust

// === Phase 3.2: an editor's buffer (spec
// docs/superpowers/specs/2026-10-08-phase-3-2-lsp-core-design.md §6.5) ===

#[test]
fn format_buffer_applies_the_editorconfig_that_governs_its_path() {
    let dir = fresh_dir(
        "buffer",
        "\n[*.nova]\nend_of_line = crlf\n\n[keep.nova]\ninsert_final_newline = false\n",
    );
    // Neither file exists: the buffer is the text, and the path only
    // chooses the `.editorconfig` sections.
    let crlf = nova_fmt::format_buffer(&dir.join("main.nova"), UNFORMATTED).unwrap();
    assert_eq!(crlf, FORMATTED.replace('\n', "\r\n"));
    let keep =
        nova_fmt::format_buffer(&dir.join("keep.nova"), "fn main() {\nprintln(\"hi\")\n}").unwrap();
    assert_eq!(keep, "fn main() { println(\"hi\") }");
}

#[test]
fn format_buffer_keeps_a_buffers_crlf() {
    let dir = fresh_dir("buffer-crlf", "");
    let out = nova_fmt::format_buffer(&dir.join("main.nova"), &UNFORMATTED.replace('\n', "\r\n"))
        .unwrap();
    assert_eq!(out, FORMATTED.replace('\n', "\r\n"));
}

#[test]
fn format_buffer_refuses_a_syntax_error() {
    let dir = fresh_dir("buffer-error", "");
    let err = nova_fmt::format_buffer(&dir.join("main.nova"), "fn main( {\n").unwrap_err();
    assert!(matches!(err, FormatError::Syntax { .. }), "{err:?}");
}
```

- [ ] **Step 2: Run them to verify they fail**

```bash
cd /d/Projects/nona/nova && cargo test --locked -p nova-fmt --test files 2>&1 | grep -E "error\[" | head -3
```

Expected: `error[E0425]: cannot find function `format_buffer` in crate `nova_fmt``.

- [ ] **Step 3: Implement**

In `crates/nova-fmt/src/file.rs`, replace `format_file` with:

```rust
/// Format the file at `path` (spec §5.1): [`crate::format`]'s output, with
/// the line ending and the final newline §7.3 gives it. Nothing is written.
pub fn format_file(path: &Path) -> Result<Formatted, FileError> {
    let bytes = std::fs::read(path).map_err(FileError::Io)?;
    let original = String::from_utf8(bytes).map_err(|_| FileError::NotUtf8)?;
    let formatted = format_buffer(path, &original).map_err(FileError::Format)?;
    Ok(Formatted {
        formatted,
        original,
    })
}

/// Format `text`, an editor's buffer for the file at `path`, as
/// [`format_file`] would format that file if it held `text`. The
/// `.editorconfig` that governs `path` sets the line ending and the final
/// newline, and otherwise the buffer keeps its own (spec
/// `docs/superpowers/specs/2026-10-08-phase-3-2-lsp-core-design.md` §6.5).
/// `path` need not exist.
pub fn format_buffer(path: &Path, text: &str) -> Result<String, FormatError> {
    let settings = editorconfig::settings_for(&absolute(path));
    let ending = match settings.crlf {
        Some(true) => LineEnding::Crlf,
        Some(false) => LineEnding::Lf,
        None => LineEnding::of(text),
    };
    let mut lf = crate::format_named(text, &path.display().to_string())?;
    // With `insert_final_newline = false`, the text ends in a newline only
    // if it did (the 3.1 plan's decision 13).
    if settings.insert_final_newline == Some(false) && !text.ends_with('\n') {
        lf.pop();
    }
    Ok(ending.apply(&lf))
}
```

In `crates/nova-fmt/src/lib.rs`, replace
`pub use file::{format_file, format_text, FileError, Formatted, LineEnding};`
with:

```rust
pub use file::{format_buffer, format_file, format_text, FileError, Formatted, LineEnding};
```

- [ ] **Step 4: Run nova-fmt's tests, and `nova fmt`'s**

```bash
cd /d/Projects/nona/nova && cargo test --locked -p nova-fmt 2>&1 | grep -E "^test result|FAILED|panicked" | head && cargo test --locked -p nova-cli --test fmt 2>&1 | grep -E "^test result|FAILED" | head -3
```

Expected:
- every `test result:` line is `ok`, the three new tests included;
- `nova fmt`'s CLI tests still pass, 12 on Windows (15 on Linux).

- [ ] **Step 5: Commit**

Write `$P/msg-9.txt`:

```
nova-fmt: format_buffer, for an editor's unsaved text

The language server formats a buffer, not a file (spec
docs/superpowers/specs/2026-10-08-phase-3-2-lsp-core-design.md §6.5).
`format_buffer(path, text)` formats `text` exactly as `format_file` would
format the file at `path` if it held `text`: the `.editorconfig` that
governs `path` sets the line ending and the final newline, and otherwise
the buffer keeps its own. `path` need not exist.

`format_file` now reads the file and calls it, so its behaviour is
unchanged.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
```

```bash
cd /d/Projects/nona/nova && P=/c/Users/SAKEER~1/AppData/Local/Temp/gcm/p32 && cargo fmt --all && git add crates/nova-fmt && git commit -q -F $P/msg-9.txt && git log -1 --format=%s
```

Expected: `nova-fmt: format_buffer, for an editor's unsaved text`.

The task's test command: `cargo test --locked -p nova-fmt`.

---
### Task 10: `nova lsp`: the protocol, documents, and diagnostics for loose files

Spec §5, §6.1, §6.3, §6.7, §6.9 and §9.3 items 1 and 7, and Review Focus
items 1, 2 and 5.

**What this task builds:**
- the server's protocol loop;
- the URI and position conversions;
- the diagnostic mapping;
- the open documents and their overlay;
- a checker thread.

Every document is checked as a loose file, its own entry. Task 11 adds
projects, ownership, watched files and stale-result dropping. Tasks 12 and
13 add completion and formatting.

**Files:**
- Modify: `Cargo.toml` (`[workspace.dependencies]`: `lsp-server`,
  `lsp-types`), and `Cargo.lock` follows
- Modify: `crates/nova-lsp/Cargo.toml`
- Create:
  - `crates/nova-lsp/src/uri.rs`;
  - `crates/nova-lsp/src/convert.rs`;
  - `crates/nova-lsp/src/workspace.rs`;
  - `crates/nova-lsp/src/checker.rs`.
- Replace: `crates/nova-lsp/src/lib.rs`
- Modify: `crates/nova-cli/Cargo.toml` (`nova-lsp`; dev-dependency
  `serde_json`), `crates/nova-cli/src/main.rs`, `crates/nova-cli/src/cmd/mod.rs`
- Create: `crates/nova-cli/src/cmd/lsp.rs`
- Create: `crates/nova-cli/tests/lsp_client/mod.rs`, `crates/nova-cli/tests/lsp.rs`

**Interfaces:**
- Consumes:
  - Task 1's `LineIndex`;
  - Task 3's `parse_recovering`;
  - Task 7's `analyze`, `Analysis`, `Options` and `Sources`.
- Produces, for Tasks 11 to 14:
  - `nova_lsp::run() -> anyhow::Result<i32>`;
  - `uri::{to_path, from_path, untitled_path, document_path, text, parse}`;
  - `convert::{range, diagnostics_for}`;
  - `workspace::{PathKey, Document, Workspace, Overlay, declares_main}`
    (`Workspace::{open, change, close, get, overlay}`);
  - `checker::{Checker, Job, Publish}`.
  - In the tests:
    - `lsp_client::{Client, file_uri, fresh_dir, same_uri, WAIT}`;
    - `Client::{start, notify, request, wait_for, diagnostics, shutdown_and_exit, exit_without_shutdown}`.
  - In `lsp.rs`: `open`, `codes`, `planted` and `planted_range`.

- [ ] **Step 1: Add the crates**

In the root `Cargo.toml`, at the end of `[workspace.dependencies]`:

```toml

# The language server (spec
# docs/superpowers/specs/2026-10-08-phase-3-2-lsp-core-design.md §6; ADR 0029).
# lsp-server is rust-analyzer's synchronous framing crate. It is pinned
# because 0.7.9 and every later release are edition 2024, which needs Rust
# 1.85, above the 1.78 MSRV; 0.7.8 is edition 2021 and brings
# crossbeam-channel and log. lsp-types 0.97.0 (edition 2018) holds the
# protocol's types and brings fluent-uri 0.1, bitflags 1 and serde_repr.
# Every crate these pull in declares Rust 1.71 or less, or nothing, so the
# MSRV job is their check.
lsp-server = "=0.7.8"
lsp-types = "0.97"
```

Replace the `[dependencies]` of `crates/nova-lsp/Cargo.toml` with:

```toml
[dependencies]
nova-diagnostics = { path = "../nova-diagnostics" }
nova-lexer = { path = "../nova-lexer" }
nova-ast = { path = "../nova-ast" }
nova-parser = { path = "../nova-parser" }
nova-resolver = { path = "../nova-resolver" }
nova-typeck = { path = "../nova-typeck" }
nova-hir = { path = "../nova-hir" }
nova-driver = { path = "../nova-driver" }
nova-pm = { path = "../nova-pm" }
nova-fmt = { path = "../nova-fmt" }
lsp-server = { workspace = true }
lsp-types = { workspace = true }
serde_json = { workspace = true }
anyhow = { workspace = true }
tracing = { workspace = true }
```

In `crates/nova-cli/Cargo.toml`, add `nova-lsp = { path = "../nova-lsp" }`
after `nova-fmt = { path = "../nova-fmt" }` in `[dependencies]`, and
`serde_json = { workspace = true }` to `[dev-dependencies]`.

```bash
cd /d/Projects/nona/nova && cargo update -p nova-lsp 2>&1 | tail -3; git diff --stat -- Cargo.lock; grep -n -E '^name = "(lsp-server|lsp-types|crossbeam-channel|fluent-uri|serde_repr|log|bitflags)"' -A1 Cargo.lock
```

Expected:
- `lsp-server` at `version = "0.7.8"` and `lsp-types` at `0.97.0`;
- `crossbeam-channel`, `fluent-uri`, `serde_repr`, `log` and `bitflags`
  (a 1.x entry) are present.

Ledger every package `Cargo.lock` gained, with its version. If
`cargo update -p nova-lsp` refuses, run `cargo check -p nova-lsp`, which
also writes the lock. Never run a bare `cargo update`.

- [ ] **Step 2: Write the failing tests**

`crates/nova-cli/tests/lsp_client/mod.rs`:

```rust
//! A Language Server Protocol client for the tests. It frames JSON-RPC over
//! a `nova lsp` child's stdin and stdout (spec
//! `docs/superpowers/specs/2026-10-08-phase-3-2-lsp-core-design.md` §9.3).

#![allow(dead_code)]

use std::io::{BufRead, BufReader, Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, Command, ExitStatus, Stdio};
use std::sync::mpsc::{self, Receiver};
use std::time::{Duration, Instant};

use serde_json::{json, Value};

/// How long any one wait lasts before the test fails.
pub const WAIT: Duration = Duration::from_secs(30);

pub struct Client {
    child: Child,
    stdin: Option<ChildStdin>,
    messages: Receiver<Value>,
    next_id: i64,
    /// Messages read while waiting for another, oldest first.
    unread: Vec<Value>,
}

impl Client {
    /// Start `nova lsp` and initialize it. With `watch`, the client offers
    /// dynamic registration of watched files, as VS Code does.
    pub fn start(root: &Path, watch: bool) -> Client {
        let mut child = Command::new(assert_cmd::cargo::cargo_bin("nova"))
            .arg("lsp")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()
            .expect("start nova lsp");
        let stdin = child.stdin.take();
        let stdout = child.stdout.take().expect("piped stdout");
        let (tx, rx) = mpsc::channel();
        std::thread::spawn(move || {
            let mut reader = BufReader::new(stdout);
            while let Some(message) = read_message(&mut reader) {
                if tx.send(message).is_err() {
                    break;
                }
            }
        });
        let mut client = Client {
            child,
            stdin,
            messages: rx,
            next_id: 1,
            unread: Vec::new(),
        };
        let capabilities = if watch {
            json!({ "workspace": { "didChangeWatchedFiles": { "dynamicRegistration": true } } })
        } else {
            json!({})
        };
        client.request(
            "initialize",
            json!({ "processId": null, "rootUri": file_uri(root), "capabilities": capabilities }),
        );
        client.notify("initialized", json!({}));
        client
    }

    pub fn notify(&mut self, method: &str, params: Value) {
        self.send(&json!({ "jsonrpc": "2.0", "method": method, "params": params }));
    }

    /// Send a request and return its response.
    pub fn request(&mut self, method: &str, params: Value) -> Value {
        let id = self.next_id;
        self.next_id += 1;
        self.send(&json!({ "jsonrpc": "2.0", "id": id, "method": method, "params": params }));
        self.wait_for(|m| m.get("method").is_none() && m.get("id") == Some(&json!(id)))
    }

    /// The first message, old or new, that `want` accepts. The others are
    /// kept for later waits. A request from the server, such as
    /// `client/registerCapability`, is answered with `null` as it arrives.
    pub fn wait_for(&mut self, want: impl Fn(&Value) -> bool) -> Value {
        if let Some(i) = self.unread.iter().position(&want) {
            return self.unread.remove(i);
        }
        let deadline = Instant::now() + WAIT;
        loop {
            let left = deadline.saturating_duration_since(Instant::now());
            let message = self.messages.recv_timeout(left).unwrap_or_else(|_| {
                panic!("no matching message within {WAIT:?}; read but unmatched: {:#?}", self.unread)
            });
            if message.get("method").is_some() && message.get("id").is_some() {
                let id = message["id"].clone();
                self.send(&json!({ "jsonrpc": "2.0", "id": id, "result": null }));
            }
            if want(&message) {
                return message;
            }
            self.unread.push(message);
        }
    }

    /// The params of the next `textDocument/publishDiagnostics` for `uri`
    /// that `want` accepts.
    pub fn diagnostics(&mut self, uri: &str, want: impl Fn(&Value) -> bool) -> Value {
        let message = self.wait_for(|m| {
            m["method"] == "textDocument/publishDiagnostics"
                && m["params"]["uri"].as_str().is_some_and(|u| same_uri(u, uri))
                && want(&m["params"])
        });
        message["params"].clone()
    }

    /// `shutdown`, then `exit`; returns the server's exit status.
    pub fn shutdown_and_exit(mut self) -> ExitStatus {
        self.request("shutdown", Value::Null);
        self.notify("exit", Value::Null);
        self.wait()
    }

    /// `exit` with no `shutdown` first.
    pub fn exit_without_shutdown(mut self) -> ExitStatus {
        self.notify("exit", Value::Null);
        self.wait()
    }

    fn wait(&mut self) -> ExitStatus {
        self.stdin.take();
        let deadline = Instant::now() + WAIT;
        loop {
            if let Some(status) = self.child.try_wait().expect("poll nova lsp") {
                return status;
            }
            assert!(Instant::now() < deadline, "nova lsp did not exit");
            std::thread::sleep(Duration::from_millis(20));
        }
    }

    fn send(&mut self, message: &Value) {
        let body = serde_json::to_vec(message).expect("serialize");
        let stdin = self.stdin.as_mut().expect("stdin is open");
        write!(stdin, "Content-Length: {}\r\n\r\n", body.len()).expect("write header");
        stdin.write_all(&body).expect("write body");
        stdin.flush().expect("flush");
    }
}

impl Drop for Client {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

fn read_message(reader: &mut impl BufRead) -> Option<Value> {
    let mut length = None;
    loop {
        let mut line = String::new();
        if reader.read_line(&mut line).ok()? == 0 {
            return None;
        }
        let line = line.trim_end();
        if line.is_empty() {
            break;
        }
        if let Some(n) = line.strip_prefix("Content-Length: ") {
            length = n.parse().ok();
        }
    }
    let mut body = vec![0; length?];
    reader.read_exact(&mut body).ok()?;
    serde_json::from_slice(&body).ok()
}

/// A `file:` URI for `path`, as VS Code writes one: the drive letter lower
/// case and its `:` as `%3A`, and every byte outside `A-Za-z0-9-._~/`
/// percent-encoded.
pub fn file_uri(path: &Path) -> String {
    let mut s = path.to_string_lossy().replace('\\', "/");
    if s.as_bytes().get(1) == Some(&b':') {
        s = format!("{}{}", s[..1].to_lowercase(), &s[1..]);
    }
    if !s.starts_with('/') {
        s.insert(0, '/');
    }
    let mut out = String::from("file://");
    for b in s.bytes() {
        if b.is_ascii_alphanumeric() || b"-._~/".contains(&b) {
            out.push(b as char);
        } else {
            out.push_str(&format!("%{b:02X}"));
        }
    }
    out
}

/// Whether two URIs name the same thing: equal after percent-decoding, and
/// on Windows ignoring case. The server publishes an unopened file under its
/// own spelling of the URI.
pub fn same_uri(a: &str, b: &str) -> bool {
    let norm = |s: &str| {
        let decoded = decode(s);
        if cfg!(windows) {
            decoded.to_lowercase()
        } else {
            decoded
        }
    };
    norm(a) == norm(b)
}

fn decode(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            if let Ok(b) = u8::from_str_radix(&String::from_utf8_lossy(&bytes[i + 1..i + 3]), 16) {
                out.push(b);
                i += 3;
                continue;
            }
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// A fresh directory with a space and Thai in its path, so every URI is
/// percent-encoded (Review Focus 2). Its name is fixed, so each run
/// replaces the last.
pub fn fresh_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir()
        .join("nova lsp tests")
        .join(format!("โปรเจกต์-{name}"));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("create the test directory");
    dir
}
```

`crates/nova-cli/tests/lsp.rs`:

```rust
//! `nova lsp`, driven over stdio (spec
//! `docs/superpowers/specs/2026-10-08-phase-3-2-lsp-core-design.md` §9.3).

mod lsp_client;

use lsp_client::{file_uri, fresh_dir, Client};
use serde_json::{json, Value};

/// Thai and an emoji before the planted error, so a byte column and a
/// UTF-16 column differ (gate item 1).
const PLANTED_LINE: &str = "    let s = \"ก😀\"; let x: Int = \"s\"";

fn planted() -> String {
    format!("fn main() {{\n{PLANTED_LINE}\n}}\n")
}

/// The UTF-16 range of the planted `"s"`, on line 1.
fn planted_range() -> Value {
    let at = PLANTED_LINE.rfind("\"s\"").unwrap();
    let start = PLANTED_LINE[..at].encode_utf16().count();
    json!({
        "start": { "line": 1, "character": start },
        "end": { "line": 1, "character": start + 3 },
    })
}

fn open(client: &mut Client, uri: &str, text: &str) {
    client.notify(
        "textDocument/didOpen",
        json!({ "textDocument": { "uri": uri, "languageId": "nova", "version": 1, "text": text } }),
    );
}

fn codes(params: &Value) -> Vec<String> {
    params["diagnostics"]
        .as_array()
        .unwrap()
        .iter()
        .map(|d| d["code"].as_str().unwrap_or("").to_string())
        .collect()
}

fn nonempty(params: &Value) -> bool {
    !params["diagnostics"].as_array().unwrap().is_empty()
}

#[test]
fn a_planted_error_gets_its_diagnostic_with_an_exact_utf16_range() {
    let dir = fresh_dir("planted");
    let file = dir.join("main.nova");
    std::fs::write(&file, planted()).unwrap();
    let uri = file_uri(&file);
    let mut client = Client::start(&dir, false);
    open(&mut client, &uri, &planted());
    let params = client.diagnostics(&uri, nonempty);
    assert_eq!(codes(&params), ["E0010"]);
    assert_eq!(params["diagnostics"][0]["range"], planted_range());
    assert_eq!(params["version"], 1);
    // An open document's diagnostics come back under the client's own URI.
    assert_eq!(params["uri"], uri);
    assert_eq!(client.shutdown_and_exit().code(), Some(0));
}

#[test]
fn a_crlf_document_gets_exact_ranges() {
    let dir = fresh_dir("crlf");
    let file = dir.join("main.nova");
    let text = planted().replace('\n', "\r\n");
    std::fs::write(&file, &text).unwrap();
    let uri = file_uri(&file);
    let mut client = Client::start(&dir, false);
    open(&mut client, &uri, &text);
    let params = client.diagnostics(&uri, nonempty);
    assert_eq!(params["diagnostics"][0]["range"], planted_range());
}

#[test]
fn an_untitled_buffer_gets_diagnostics() {
    let dir = fresh_dir("untitled");
    let mut client = Client::start(&dir, false);
    open(&mut client, "untitled:Untitled-1", &planted());
    let params = client.diagnostics("untitled:Untitled-1", nonempty);
    assert_eq!(codes(&params), ["E0010"]);
}

#[test]
fn shutdown_then_exit_is_code_0_and_exit_alone_is_code_1() {
    let dir = fresh_dir("exit");
    assert_eq!(Client::start(&dir, false).shutdown_and_exit().code(), Some(0));
    assert_eq!(Client::start(&dir, false).exit_without_shutdown().code(), Some(1));
}

#[test]
fn an_unknown_request_gets_method_not_found() {
    let dir = fresh_dir("unknown");
    let mut client = Client::start(&dir, false);
    let response = client.request(
        "textDocument/hover",
        json!({
            "textDocument": { "uri": file_uri(&dir.join("x.nova")) },
            "position": { "line": 0, "character": 0 },
        }),
    );
    assert_eq!(response["error"]["code"], -32601);
}
```

- [ ] **Step 3: Run them to verify they fail**

```bash
cd /d/Projects/nona/nova && cargo test --locked -p nova-cli --test lsp 2>&1 | grep -E "^test |panicked|error:" | head -12
```

Expected: the five tests FAIL. `nova` has no `lsp` subcommand yet, so clap
exits with an error, and the client's first wait panics with "no matching
message".

- [ ] **Step 4: Write `uri.rs`**

`crates/nova-lsp/src/uri.rs`:

```rust
//! `file:` URIs and paths (spec
//! `docs/superpowers/specs/2026-10-08-phase-3-2-lsp-core-design.md` §6.9).

use std::path::{Path, PathBuf};

/// The path a `file:` URI names, or `None` for any other URI. Percent
/// encoding is decoded, so `file:///d%3A/x/y.nova` is `d:\x\y.nova` on
/// Windows. A URI with an authority (`file://server/share`) is not a local
/// file.
pub fn to_path(uri: &str) -> Option<PathBuf> {
    let rest = uri.strip_prefix("file://")?;
    if !rest.starts_with('/') {
        return None;
    }
    let decoded = percent_decode(rest);
    if cfg!(windows) {
        let b = decoded.as_bytes();
        if b.len() >= 3 && b[1].is_ascii_alphabetic() && b[2] == b':' {
            return Some(PathBuf::from(decoded[1..].replace('/', "\\")));
        }
        return None;
    }
    Some(PathBuf::from(decoded))
}

/// A `file:` URI for the absolute `path`, with every byte outside
/// `A-Za-z0-9-._~/:` percent-encoded.
pub fn from_path(path: &Path) -> String {
    let mut s = path.to_string_lossy().replace('\\', "/");
    if !s.starts_with('/') {
        s.insert(0, '/');
    }
    let mut out = String::from("file://");
    for b in s.bytes() {
        if b.is_ascii_alphanumeric() || b"-._~/:".contains(&b) {
            out.push(b as char);
        } else {
            out.push_str(&format!("%{b:02X}"));
        }
    }
    out
}

/// The path an untitled buffer is checked under. It exists only in the
/// overlay (decision 7).
pub fn untitled_path(uri: &str) -> Option<PathBuf> {
    let name = uri.strip_prefix("untitled:")?;
    let safe: String = name
        .chars()
        .map(|c| if c.is_alphanumeric() || c == '-' { c } else { '_' })
        .collect();
    Some(
        std::env::temp_dir()
            .join("nova-untitled")
            .join(format!("{safe}.nova")),
    )
}

/// The path a document is checked under: its file's, or an untitled
/// buffer's.
pub fn document_path(uri: &str) -> Option<PathBuf> {
    to_path(uri).or_else(|| untitled_path(uri))
}

/// An `lsp_types::Uri` as text. This and [`parse`] are the only two places
/// that touch `lsp_types::Uri`'s own API.
pub fn text(uri: &lsp_types::Uri) -> String {
    uri.as_str().to_owned()
}

/// Text as an `lsp_types::Uri`, or `None` if it does not parse.
pub fn parse(s: &str) -> Option<lsp_types::Uri> {
    s.parse().ok()
}

fn percent_decode(s: &str) -> String {
    let b = s.as_bytes();
    let hex = |c: u8| (c as char).to_digit(16).map(|d| d as u8);
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'%' && i + 2 < b.len() {
            if let (Some(h), Some(l)) = (hex(b[i + 1]), hex(b[i + 2])) {
                out.push(h * 16 + l);
                i += 3;
                continue;
            }
        }
        out.push(b[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(windows)]
    #[test]
    fn a_windows_uri_with_an_encoded_drive_letter_is_its_path() {
        assert_eq!(
            to_path("file:///d%3A/x/y.nova"),
            Some(PathBuf::from(r"d:\x\y.nova"))
        );
        assert_eq!(
            to_path("file:///D:/x/y.nova"),
            Some(PathBuf::from(r"D:\x\y.nova"))
        );
    }

    #[test]
    fn uris_with_spaces_and_thai_round_trip() {
        let path = if cfg!(windows) {
            PathBuf::from(r"C:\Users\a b\โปรเจกต์\main.nova")
        } else {
            PathBuf::from("/home/a b/โปรเจกต์/main.nova")
        };
        let uri = from_path(&path);
        assert!(uri.starts_with("file:///"), "{uri}");
        assert!(!uri.contains(' '), "{uri}");
        assert_eq!(to_path(&uri), Some(path));
    }

    #[test]
    fn other_uris_are_not_files() {
        assert_eq!(to_path("untitled:Untitled-1"), None);
        assert_eq!(to_path("file://server/share/x.nova"), None);
        let untitled = untitled_path("untitled:Untitled-1").unwrap();
        assert!(untitled.ends_with("Untitled-1.nova"), "{untitled:?}");
        assert_eq!(document_path("untitled:Untitled-1"), Some(untitled));
    }
}
```

- [ ] **Step 5: Write `workspace.rs`**

`crates/nova-lsp/src/workspace.rs`:

```rust
//! The open documents, and the sources an analysis reads (spec
//! `docs/superpowers/specs/2026-10-08-phase-3-2-lsp-core-design.md` §6.2,
//! §6.9).

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use nova_driver::Sources;

use crate::uri;

/// A path as the server compares paths (decision 6):
/// - absolute, with its directory canonicalised when it exists;
/// - Windows' `\\?\` prefix stripped and `/` for `\`;
/// - on Windows, lower case.
///
/// A buffer and the driver's path for the same file always have the same
/// key.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PathKey(String);

impl PathKey {
    pub fn of(path: &Path) -> PathKey {
        let abs = if path.is_absolute() {
            path.to_path_buf()
        } else {
            std::env::current_dir()
                .map(|cwd| cwd.join(path))
                .unwrap_or_else(|_| path.to_path_buf())
        };
        let full = match (abs.parent(), abs.file_name()) {
            (Some(dir), Some(name)) => std::fs::canonicalize(dir)
                .unwrap_or_else(|_| dir.to_path_buf())
                .join(name),
            _ => abs,
        };
        let mut s = full.to_string_lossy().replace('\\', "/");
        if let Some(rest) = s.strip_prefix("//?/") {
            s = rest.to_string();
        }
        if cfg!(windows) {
            s = s.to_lowercase();
        }
        PathKey(s)
    }

    /// Whether this path is inside the directory `dir`'s key.
    pub fn is_under(&self, dir: &PathKey) -> bool {
        self.0.starts_with(&format!("{}/", dir.0.trim_end_matches('/')))
    }
}

/// An open document.
#[derive(Debug, Clone)]
pub struct Document {
    /// The client's URI for it, under which its diagnostics are published.
    pub uri: String,
    /// The path it is checked under.
    pub path: PathBuf,
    pub version: i32,
    pub text: String,
}

/// Every open document.
#[derive(Debug, Default)]
pub struct Workspace {
    docs: HashMap<PathKey, Document>,
}

impl Workspace {
    /// Open `uri`, and return its path, or `None` for a URI that names no
    /// file.
    pub fn open(&mut self, uri: &str, version: i32, text: String) -> Option<PathBuf> {
        let path = uri::document_path(uri)?;
        self.docs.insert(
            PathKey::of(&path),
            Document {
                uri: uri.to_string(),
                path: path.clone(),
                version,
                text,
            },
        );
        Some(path)
    }

    /// Replace an open document's text.
    pub fn change(&mut self, uri: &str, version: i32, text: String) -> Option<PathBuf> {
        let path = uri::document_path(uri)?;
        let doc = self.docs.get_mut(&PathKey::of(&path))?;
        doc.version = version;
        doc.text = text;
        Some(path)
    }

    /// Close a document, and return it.
    pub fn close(&mut self, uri: &str) -> Option<Document> {
        let path = uri::document_path(uri)?;
        self.docs.remove(&PathKey::of(&path))
    }

    pub fn get(&self, uri: &str) -> Option<&Document> {
        let path = uri::document_path(uri)?;
        self.docs.get(&PathKey::of(&path))
    }

    /// A copy of the open buffers, for an analysis on another thread.
    pub fn overlay(&self) -> Overlay {
        Overlay {
            buffers: self
                .docs
                .iter()
                .map(|(k, d)| (k.clone(), d.text.clone()))
                .collect(),
        }
    }
}

/// The open buffers, read in place of their files; every other file comes
/// from disk.
#[derive(Debug, Clone, Default)]
pub struct Overlay {
    buffers: HashMap<PathKey, String>,
}

impl Sources for Overlay {
    fn read(&self, path: &Path) -> std::io::Result<String> {
        match self.buffers.get(&PathKey::of(path)) {
            Some(text) => Ok(text.clone()),
            None => std::fs::read_to_string(path),
        }
    }

    fn same_file(&self, a: &Path, b: &Path) -> bool {
        PathKey::of(a) == PathKey::of(b)
    }
}

/// Whether `text` declares a top-level `fn main`. A loose file without one
/// is checked as a module (spec §6.2).
pub fn declares_main(text: &str) -> bool {
    let file = nova_diagnostics::FileId::DUMMY;
    let (tokens, _) = nova_lexer::lex(text, file);
    let parsed = nova_parser::parse_recovering(&tokens, file);
    parsed
        .file
        .items
        .iter()
        .any(|i| matches!(&i.value, nova_ast::Item::Function(f) if f.name.value == "main"))
}
```

- [ ] **Step 6: Write `convert.rs`**

`crates/nova-lsp/src/convert.rs`:

```rust
//! Nova's diagnostics as LSP's (spec
//! `docs/superpowers/specs/2026-10-08-phase-3-2-lsp-core-design.md` §6.3).

use std::collections::HashMap;

use lsp_types as lsp;
use nova_diagnostics::{Diagnostic, FileId, LineIndex, Severity};
use nova_driver::Analysis;

/// LSP's range for bytes `start..end` of the indexed text.
pub fn range(index: &LineIndex, start: u32, end: u32) -> lsp::Range {
    let (sl, sc) = index.position(start);
    let (el, ec) = index.position(end);
    lsp::Range {
        start: lsp::Position {
            line: sl,
            character: sc,
        },
        end: lsp::Position {
            line: el,
            character: ec,
        },
    }
}

/// The LSP diagnostics `analysis` has for `file`.
///
/// A diagnostic goes to the file of its primary label, or of its first label
/// in one of the program's own files. One with no label in the program's
/// files goes to `entry`'s first line, naming the place it has: the spec
/// §6.3 fallback, for E0601 and for labels in std.
pub fn diagnostics_for(analysis: &Analysis, file: FileId, entry: FileId) -> Vec<lsp::Diagnostic> {
    let own: Vec<FileId> = analysis.modules.iter().map(|(f, _)| *f).collect();
    let mut indexes: HashMap<FileId, LineIndex> = HashMap::new();
    let mut out = Vec::new();
    for d in &analysis.diagnostics {
        let label = d
            .labels
            .iter()
            .find(|l| l.primary && own.contains(&l.span.file))
            .or_else(|| d.labels.iter().find(|l| own.contains(&l.span.file)));
        let place = label.map_or(entry, |l| l.span.file);
        if place != file {
            continue;
        }
        let index = indexes
            .entry(place)
            .or_insert_with(|| LineIndex::new(analysis.db.get_source(place).unwrap_or("")));
        let at = match label {
            Some(l) => range(index, l.span.start, l.span.end),
            None => range(index, 0, 0),
        };
        out.push(lsp::Diagnostic {
            range: at,
            severity: Some(severity(d.severity)),
            code: Some(lsp::NumberOrString::String(d.code.clone())),
            source: Some("nova".to_string()),
            message: message(analysis, d, label.is_none()),
            ..Default::default()
        });
    }
    out
}

fn severity(s: Severity) -> lsp::DiagnosticSeverity {
    match s {
        Severity::Error => lsp::DiagnosticSeverity::ERROR,
        Severity::Warning => lsp::DiagnosticSeverity::WARNING,
        Severity::Note => lsp::DiagnosticSeverity::INFORMATION,
        Severity::Help => lsp::DiagnosticSeverity::HINT,
    }
}

/// The message, its notes one per line, and for a diagnostic placed by the
/// fallback, where it really points, such as `<std/core>:12:5`.
fn message(analysis: &Analysis, d: &Diagnostic, fallback: bool) -> String {
    let mut m = d.message.clone();
    for note in &d.notes {
        m.push('\n');
        m.push_str(note);
    }
    if fallback {
        if let Some(l) = d.labels.first() {
            let name = analysis.db.get_name(l.span.file);
            let at = analysis.db.location(l.span.file, l.span.start);
            if let (Some(name), Some((line, column))) = (name, at) {
                m.push_str(&format!("\n(at {name}:{line}:{column})"));
            }
        }
    }
    m
}
```

Secondary labels as related information (spec §6.3's table) are left out
here; Task 11 adds them, with the URIs they need.

- [ ] **Step 7: Write `checker.rs`**

`crates/nova-lsp/src/checker.rs`:

```rust
//! The checker thread: analyses run here, off the protocol's thread (spec
//! `docs/superpowers/specs/2026-10-08-phase-3-2-lsp-core-design.md` §6.6).

use std::path::PathBuf;
use std::sync::mpsc;

use nova_driver::{analyze, Options, Sources};

use crate::convert;
use crate::workspace::{declares_main, Overlay};

/// One analysis to run.
pub struct Job {
    /// The file to check, as its own entry.
    pub path: PathBuf,
    /// Where its diagnostics go.
    pub uri: String,
    pub version: Option<i32>,
    pub overlay: Overlay,
    /// Only clear the file's diagnostics: it was closed.
    pub clear: bool,
}

/// A `textDocument/publishDiagnostics` to send.
pub struct Publish {
    pub uri: String,
    pub version: Option<i32>,
    pub diagnostics: Vec<lsp_types::Diagnostic>,
}

/// The handle the protocol's thread keeps.
pub struct Checker {
    jobs: mpsc::Sender<Job>,
}

impl Checker {
    /// Start the thread. It runs jobs in order and hands each result to
    /// `publish`.
    pub fn spawn(publish: impl Fn(Publish) + Send + 'static) -> Checker {
        let (jobs, queue) = mpsc::channel::<Job>();
        std::thread::Builder::new()
            .name("nova-lsp-checker".to_string())
            .spawn(move || {
                for job in queue {
                    if let Some(p) = run(&job) {
                        publish(p);
                    }
                }
            })
            .expect("start the checker thread");
        Checker { jobs }
    }

    pub fn submit(&self, job: Job) {
        let _ = self.jobs.send(job);
    }
}

fn run(job: &Job) -> Option<Publish> {
    if job.clear {
        return Some(Publish {
            uri: job.uri.clone(),
            version: None,
            diagnostics: Vec::new(),
        });
    }
    let module_only = !job
        .overlay
        .read(&job.path)
        .map(|t| declares_main(&t))
        .unwrap_or(true);
    let options = Options {
        keep_going: true,
        tests: true,
        module_only,
        probe: None,
    };
    let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        analyze(&job.path, &job.overlay, &options)
    }));
    let analysis = match outcome {
        Ok(Ok(a)) => a,
        Ok(Err(e)) => {
            tracing::warn!("nova lsp: cannot read {}: {e}", job.path.display());
            return None;
        }
        Err(_) => {
            tracing::warn!("nova lsp: the front end panicked on {}", job.path.display());
            return None;
        }
    };
    let entry = analysis.modules.first()?.0;
    Some(Publish {
        uri: job.uri.clone(),
        version: job.version,
        diagnostics: convert::diagnostics_for(&analysis, entry, entry),
    })
}
```

- [ ] **Step 8: Write the server's loop**

Replace `crates/nova-lsp/src/lib.rs` with:

```rust
//! The Nova language server, `nova lsp` (spec
//! `docs/superpowers/specs/2026-10-08-phase-3-2-lsp-core-design.md` §6;
//! ADR 0029).
//!
//! The protocol runs on the calling thread through `lsp-server`, and
//! analyses run on the checker thread. Stdout carries only protocol
//! messages; logs go through `tracing`, which `nova lsp` sends to stderr.

mod checker;
mod convert;
mod uri;
mod workspace;

use anyhow::Result;
use lsp_server::{Connection, ErrorCode, Message, Notification, Request, Response};
use lsp_types as lsp;
use lsp_types::notification::{
    DidChangeTextDocument, DidCloseTextDocument, DidOpenTextDocument, DidSaveTextDocument, Exit,
    Notification as _, PublishDiagnostics,
};

use checker::{Checker, Job, Publish};
use workspace::Workspace;

/// Serve over stdin and stdout until the client says `exit`. Returns the
/// exit code: 0 after `shutdown` then `exit`, 1 for an `exit` without
/// `shutdown` or a closed stream.
pub fn run() -> Result<i32> {
    let (connection, io_threads) = Connection::stdio();
    let code = serve(&connection)?;
    drop(connection);
    io_threads.join()?;
    Ok(code)
}

fn serve(connection: &Connection) -> Result<i32> {
    let (id, params) = connection.initialize_start()?;
    let _params: lsp::InitializeParams = serde_json::from_value(params)?;
    let result = serde_json::json!({
        "capabilities": capabilities(),
        "serverInfo": { "name": "nova lsp", "version": env!("CARGO_PKG_VERSION") },
    });
    connection.initialize_finish(id, result)?;

    let sender = connection.sender.clone();
    let checker = Checker::spawn(move |p: Publish| {
        let Some(uri) = uri::parse(&p.uri) else {
            tracing::warn!("nova lsp: cannot publish for {}", p.uri);
            return;
        };
        let params = lsp::PublishDiagnosticsParams {
            uri,
            diagnostics: p.diagnostics,
            version: p.version,
        };
        let _ = sender.send(Message::Notification(Notification::new(
            PublishDiagnostics::METHOD.to_string(),
            params,
        )));
    });
    let mut server = Server {
        connection,
        workspace: Workspace::default(),
        checker,
    };
    for message in &connection.receiver {
        match message {
            Message::Request(request) => {
                if connection.handle_shutdown(&request)? {
                    return Ok(0);
                }
                server.request(request);
            }
            Message::Notification(n) if n.method == Exit::METHOD => return Ok(1),
            Message::Notification(n) => server.notification(n),
            Message::Response(_) => {}
        }
    }
    Ok(1)
}

fn capabilities() -> lsp::ServerCapabilities {
    lsp::ServerCapabilities {
        position_encoding: Some(lsp::PositionEncodingKind::UTF16),
        text_document_sync: Some(lsp::TextDocumentSyncCapability::Options(
            lsp::TextDocumentSyncOptions {
                open_close: Some(true),
                change: Some(lsp::TextDocumentSyncKind::FULL),
                save: Some(lsp::TextDocumentSyncSaveOptions::Supported(true)),
                ..Default::default()
            },
        )),
        ..Default::default()
    }
}

struct Server<'c> {
    connection: &'c Connection,
    workspace: Workspace,
    checker: Checker,
}

impl Server<'_> {
    fn notification(&mut self, n: Notification) {
        match n.method.as_str() {
            DidOpenTextDocument::METHOD => {
                if let Ok(p) = serde_json::from_value::<lsp::DidOpenTextDocumentParams>(n.params) {
                    let uri = uri::text(&p.text_document.uri);
                    self.workspace
                        .open(&uri, p.text_document.version, p.text_document.text);
                    self.check(&uri);
                }
            }
            DidChangeTextDocument::METHOD => {
                if let Ok(p) = serde_json::from_value::<lsp::DidChangeTextDocumentParams>(n.params)
                {
                    let uri = uri::text(&p.text_document.uri);
                    // Full sync: the last change holds the whole text.
                    if let Some(change) = p.content_changes.into_iter().last() {
                        self.workspace
                            .change(&uri, p.text_document.version, change.text);
                        self.check(&uri);
                    }
                }
            }
            DidSaveTextDocument::METHOD => {
                if let Ok(p) = serde_json::from_value::<lsp::DidSaveTextDocumentParams>(n.params) {
                    self.check(&uri::text(&p.text_document.uri));
                }
            }
            DidCloseTextDocument::METHOD => {
                if let Ok(p) = serde_json::from_value::<lsp::DidCloseTextDocumentParams>(n.params)
                {
                    if let Some(doc) = self.workspace.close(&uri::text(&p.text_document.uri)) {
                        self.checker.submit(Job {
                            path: doc.path,
                            uri: doc.uri,
                            version: None,
                            overlay: self.workspace.overlay(),
                            clear: true,
                        });
                    }
                }
            }
            _ => {}
        }
    }

    fn request(&mut self, request: Request) {
        let response = Response::new_err(
            request.id,
            ErrorCode::MethodNotFound as i32,
            format!("nova lsp does not handle {}", request.method),
        );
        self.respond(response);
    }

    fn respond(&self, response: Response) {
        let _ = self.connection.sender.send(Message::Response(response));
    }

    /// Queue a check of an open document.
    fn check(&self, uri: &str) {
        let Some(doc) = self.workspace.get(uri) else {
            return;
        };
        self.checker.submit(Job {
            path: doc.path.clone(),
            uri: doc.uri.clone(),
            version: Some(doc.version),
            overlay: self.workspace.overlay(),
            clear: false,
        });
    }
}
```

- [ ] **Step 9: Add `nova lsp` to the CLI**

`crates/nova-cli/src/cmd/lsp.rs`:

```rust
//! `nova lsp`: the language server, over stdin and stdout (spec
//! `docs/superpowers/specs/2026-10-08-phase-3-2-lsp-core-design.md` §6).

/// Serve until the client says `exit`, and return the exit code: 0 after
/// `shutdown` then `exit`, and 1 otherwise.
pub fn run() -> i32 {
    match nova_lsp::run() {
        Ok(code) => code,
        Err(e) => {
            eprintln!("error: nova lsp: {e:#}");
            1
        }
    }
}
```

In `crates/nova-cli/src/cmd/mod.rs`, add `pub mod lsp;` after `pub mod fmt;`.

In `crates/nova-cli/src/main.rs`:
- add `lsp` to the module comment's list: "parse, run, build, check, test,
  fmt, lsp, new, init and version", plus "and Phase 3.2 `nova lsp`" with its
  spec path;
- add the variant after `Fmt(cmd::fmt::FmtCmd),`:

  ```rust
      /// Run the language server over stdin and stdout (editors start it).
      Lsp,
  ```
- replace the body of `main` up to `let cli = Cli::parse();` with:

  ```rust
  fn main() -> Result<()> {
      let cli = Cli::parse();
      let filter = tracing_subscriber::EnvFilter::from_default_env()
          .add_directive(tracing::Level::WARN.into());
      // Under `nova lsp`, stdout carries only protocol messages, so logs (and
      // the `log` records tracing forwards, lsp-server's included) go to
      // stderr (spec docs/superpowers/specs/2026-10-08-phase-3-2-lsp-core-design.md
      // §6.1). Every other command keeps its subscriber as it was.
      if matches!(cli.command, Command::Lsp) {
          tracing_subscriber::fmt()
              .with_env_filter(filter)
              .with_writer(std::io::stderr)
              .init();
      } else {
          tracing_subscriber::fmt().with_env_filter(filter).init();
      }

      nova_driver::set_embedded_runtime(embedded::runtime());
  ```

  Remove the old `let cli = Cli::parse();` that followed
  `set_embedded_runtime`.
- add the dispatch arm after the `Fmt` arm:

  ```rust
          // Its exit code is the protocol's: 0 after `shutdown` then `exit`.
          Command::Lsp => std::process::exit(cmd::lsp::run()),
  ```

- [ ] **Step 10: Run the tests to verify they pass**

```bash
cd /d/Projects/nona/nova && P=/c/Users/SAKEER~1/AppData/Local/Temp/gcm/p32 && cargo test --locked -p nova-lsp -p nova-cli --test lsp > $P/t10.txt 2>&1; echo "exit=$?"; grep -E "^test |FAILED|panicked|error(\[|:)" $P/t10.txt | head -20
```

Expected: `exit=0`. The five `lsp` tests pass, and so do `nova-lsp`'s
three `uri` tests (two on Unix).

**If a name from `lsp-types` or `lsp-server` does not compile**, read
0.97.0's or 0.7.8's source under `~/.cargo/registry/src/` and use the name
it has. The likely places are `lsp_types::Uri` (only in `uri::text` and
`uri::parse`), `initialize_start` and `initialize_finish`, and
`handle_shutdown`. Ledger each change as a ruling.

- [ ] **Step 11: Check the Linux run**

```bash
cd /d/Projects/nona/nova && P=/c/Users/SAKEER~1/AppData/Local/Temp/gcm/p32 && git add -A crates Cargo.toml Cargo.lock && bash /c/Users/SAKEER~1/AppData/Local/Temp/gcm/linux/run.sh test --locked -p nova-lsp -p nova-cli --test lsp > $P/t10-linux.txt 2>&1; echo "exit=$?"; grep -E "linux: source|^test |FAILED|panicked" $P/t10-linux.txt | head -20
```

Expected: `exit=0`, with the five `lsp` tests and two `uri` tests passing.

- [ ] **Step 12: Commit**

Write `$P/msg-10.txt`:

```
nova-lsp, nova-cli: nova lsp, with diagnostics for open files

`nova lsp` speaks the Language Server Protocol over stdio, through
lsp-server =0.7.8 and lsp-types 0.97 (spec
docs/superpowers/specs/2026-10-08-phase-3-2-lsp-core-design.md §6;
pinned: lsp-server 0.7.9 and later need Rust 1.85).
- **Sync.** It advertises full-text sync and UTF-16 positions.
- **Buffers.** It keeps every open buffer, and a checker thread runs
  `nova_driver::analyze` over them, in keep-going and test mode.
- **Diagnostics.** They are published under the client's own URI, with
  the buffer's version. A diagnostic with no place in the program's
  files goes on the entry's first line, naming where it points. Closing
  a document clears its diagnostics.
- **Paths.** URIs are percent-decoded to paths, and an untitled buffer
  gets a path that exists only in the overlay. Paths compare after
  normalisation, so a buffer always shadows its file.
- **Exit.** `shutdown` then `exit` exits 0, and `exit` alone exits 1.
- **Logs.** Under `nova lsp` logs go to stderr, so stdout carries only
  protocol messages.

Each open file is checked as its own entry for now; projects come next.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
```

```bash
cd /d/Projects/nona/nova && P=/c/Users/SAKEER~1/AppData/Local/Temp/gcm/p32 && cargo fmt --all && git add Cargo.toml Cargo.lock crates/nova-lsp crates/nova-cli && git commit -q -F $P/msg-10.txt && git log -1 --format=%s
```

Expected: `nova-lsp, nova-cli: nova lsp, with diagnostics for open files`.

The task's test command: `cargo test --locked -p nova-lsp -p nova-cli --test lsp`.

---
### Task 11: Projects, ownership, watched files, and stale results dropped

Spec §6.2, §6.3's related information, §6.6, §9.3 items 5 and 6, and Review
Focus 4.
- A document's project is now found through `nova.toml`. A project's
  analysis owns every module its entry reaches, and publishes for them,
  open or not.
- An open project file the entry does not reach is checked as a module.
- The checker collapses queued jobs per project, and publishes a result
  only if no newer job for that project was submitted while it ran.
- Closing a file re-checks its project from disk, and clears the project
  once no file of it is open.
- Watched-file events re-check.

**Files:**
- Modify: `crates/nova-lsp/src/workspace.rs` (`ProjectKey`, `real_path`,
  `Workspace::projects`, `Workspace::open_in`)
- Replace: `crates/nova-lsp/src/checker.rs`
- Modify: `crates/nova-lsp/src/convert.rs` (`diagnostics_for` gains
  `uri_of`, and related information)
- Modify: `crates/nova-lsp/src/lib.rs` (the `Server`, the watcher
  registration, the notifications)
- Modify: `crates/nova-cli/tests/lsp_client/mod.rs` (`Client::clear_unread`)
- Test: `crates/nova-cli/tests/lsp.rs` (append)

**Interfaces:**
- Consumes: Task 10's modules and test helpers.
- Produces:
  - `workspace::ProjectKey { Root(PathBuf), Loose(PathBuf) }`, with
    `of(&Path)`, `entry()` and `holds(&Path)`, equal by `PathKey`;
  - `workspace::real_path(&Path) -> PathBuf`;
  - `Workspace::projects() -> Vec<ProjectKey>`;
  - `Workspace::open_in(&ProjectKey) -> Vec<Document>`;
  - `checker::Job { project, generation, overlay, open, clear }`;
  - `convert::diagnostics_for(analysis, file, entry, uri_of: &dyn Fn(&Path) -> String)`;
  - in the tests:
    - `Client::clear_unread()`;
    - `lsp.rs`'s `project(name, files)`, `change(...)` and `nonempty`.

- [ ] **Step 1: Write the failing tests**

In `crates/nova-cli/tests/lsp_client/mod.rs`, add to `impl Client`:

```rust
    /// Forget every message read but not yet matched.
    pub fn clear_unread(&mut self) {
        self.unread.clear();
    }
```

Append to `crates/nova-cli/tests/lsp.rs`:

```rust

// === Task 11: projects, ownership, watched files, stale results ===

const MANIFEST: &str = "[package]\nname = \"demo\"\nversion = \"0.1.0\"\nedition = \"2026\"\n";

/// A project: `nova.toml`, and `src/` holding `files`.
fn project(name: &str, files: &[(&str, &str)]) -> std::path::PathBuf {
    let dir = fresh_dir(name);
    std::fs::write(dir.join("nova.toml"), MANIFEST).unwrap();
    std::fs::create_dir_all(dir.join("src")).unwrap();
    for (file, text) in files {
        std::fs::write(dir.join("src").join(file), text).unwrap();
    }
    dir
}

fn change(client: &mut Client, uri: &str, version: i32, text: &str) {
    client.notify(
        "textDocument/didChange",
        json!({
            "textDocument": { "uri": uri, "version": version },
            "contentChanges": [{ "text": text }],
        }),
    );
}

const MAIN_IMPORTS_GEOMETRY: &str = "import geometry\nfn main() {}\n";
const GEOMETRY_BROKEN: &str = "pub fn area() -> Int { \"s\" }\n";
const GEOMETRY_FIXED: &str = "pub fn area() -> Int { 1 }\n";

#[test]
fn a_module_fixed_on_disk_clears_its_diagnostic() {
    let dir = project(
        "watched",
        &[("main.nova", MAIN_IMPORTS_GEOMETRY), ("geometry.nova", GEOMETRY_BROKEN)],
    );
    let main = dir.join("src").join("main.nova");
    let geometry = dir.join("src").join("geometry.nova");
    let mut client = Client::start(&dir, true);
    open(&mut client, &file_uri(&main), MAIN_IMPORTS_GEOMETRY);
    // The project's analysis owns geometry.nova, which is not open.
    let before = client.diagnostics(&file_uri(&geometry), nonempty);
    assert_eq!(codes(&before), ["E0010"]);
    // The server registered its watcher.
    client.wait_for(|m| m["method"] == "client/registerCapability");
    std::fs::write(&geometry, GEOMETRY_FIXED).unwrap();
    client.notify(
        "workspace/didChangeWatchedFiles",
        json!({ "changes": [{ "uri": file_uri(&geometry), "type": 2 }] }),
    );
    client.diagnostics(&file_uri(&geometry), |p| !nonempty(p));
}

#[test]
fn a_burst_of_edits_ends_with_the_last_edits_diagnostics() {
    let dir = fresh_dir("burst");
    let file = dir.join("main.nova");
    std::fs::write(&file, "fn main() {}\n").unwrap();
    let uri = file_uri(&file);
    let mut client = Client::start(&dir, false);
    open(&mut client, &uri, "fn main() {}\n");
    client.diagnostics(&uri, |p| p["version"] == 1);
    // Odd versions hold the planted error; the last, 20, does not.
    for version in 2..=20 {
        let text = if version % 2 == 1 {
            planted()
        } else {
            "fn main() {}\n".to_string()
        };
        change(&mut client, &uri, version, &text);
    }
    // Version 2's check starts at once and is stale before it ends, and
    // versions 3 to 19 collapse unchecked. So the next publish is version
    // 20's: a stale one would come first.
    let next = client.diagnostics(&uri, |_| true);
    assert_eq!(next["version"], 20, "{next}");
    assert!(!nonempty(&next), "{next}");
}

#[test]
fn closing_an_unsaved_buffer_rechecks_from_disk() {
    let dir = project(
        "close",
        &[("main.nova", MAIN_IMPORTS_GEOMETRY), ("geometry.nova", GEOMETRY_BROKEN)],
    );
    let main = dir.join("src").join("main.nova");
    let geometry = dir.join("src").join("geometry.nova");
    let geometry_uri = file_uri(&geometry);
    let mut client = Client::start(&dir, false);
    open(&mut client, &file_uri(&main), MAIN_IMPORTS_GEOMETRY);
    // An unsaved buffer that fixes what the disk still has wrong.
    open(&mut client, &geometry_uri, GEOMETRY_FIXED);
    client.diagnostics(&geometry_uri, |p| p["version"] == 1 && !nonempty(p));
    client.clear_unread();
    client.notify(
        "textDocument/didClose",
        json!({ "textDocument": { "uri": geometry_uri } }),
    );
    // main.nova is still open, so the project is re-checked, reading
    // geometry.nova from disk.
    let after = client.diagnostics(&geometry_uri, nonempty);
    assert_eq!(codes(&after), ["E0010"]);
}

#[test]
fn closing_the_last_file_clears_the_project() {
    let dir = project(
        "last",
        &[("main.nova", MAIN_IMPORTS_GEOMETRY), ("geometry.nova", GEOMETRY_BROKEN)],
    );
    let main_uri = file_uri(&dir.join("src").join("main.nova"));
    let geometry_uri = file_uri(&dir.join("src").join("geometry.nova"));
    let mut client = Client::start(&dir, false);
    open(&mut client, &main_uri, MAIN_IMPORTS_GEOMETRY);
    client.diagnostics(&geometry_uri, nonempty);
    client.clear_unread();
    client.notify(
        "textDocument/didClose",
        json!({ "textDocument": { "uri": main_uri } }),
    );
    client.diagnostics(&geometry_uri, |p| !nonempty(p));
}

#[test]
fn an_unreached_project_file_is_checked_as_a_module() {
    // A guard more than a new behaviour: Task 10 already checked a file
    // with no `main` as a module. Here the file is in a project whose entry
    // does not import it, which is the ownership rule's case (spec §6.2).
    let dir = project(
        "unreached",
        &[("main.nova", "fn main() {}\n"), ("extra.nova", "fn helper() -> Int { 1 }\n")],
    );
    let extra = dir.join("src").join("extra.nova");
    let mut client = Client::start(&dir, false);
    open(&mut client, &file_uri(&dir.join("src").join("main.nova")), "fn main() {}\n");
    open(&mut client, &file_uri(&extra), "fn helper() -> Int { 1 }\n");
    let params = client.diagnostics(&file_uri(&extra), |p| p["version"] == 1);
    assert!(!nonempty(&params), "no E0601 for a module: {params}");
}

#[test]
fn a_secondary_label_becomes_related_information() {
    let dir = fresh_dir("related");
    let file = dir.join("main.nova");
    let text = "fn a() {}\nfn a() {}\nfn main() {}\n";
    std::fs::write(&file, text).unwrap();
    let uri = file_uri(&file);
    let mut client = Client::start(&dir, false);
    open(&mut client, &uri, text);
    let params = client.diagnostics(&uri, nonempty);
    let d = &params["diagnostics"][0];
    assert_eq!(d["code"], "E0002", "{params}");
    let related = &d["relatedInformation"][0];
    assert_eq!(related["message"], "first defined here");
    assert_eq!(related["location"]["range"]["start"], json!({ "line": 0, "character": 3 }));
}

#[test]
fn a_diagnostic_with_no_place_goes_on_the_entrys_first_line() {
    // MIR's E0601 has no label (spec §6.3's fallback). A project's entry is a
    // program, so a `src/main.nova` with no `main` gets it.
    let dir = project("no-main", &[("main.nova", "fn helper() {}\n")]);
    let uri = file_uri(&dir.join("src").join("main.nova"));
    let mut client = Client::start(&dir, false);
    open(&mut client, &uri, "fn helper() {}\n");
    let params = client.diagnostics(&uri, nonempty);
    assert_eq!(codes(&params), ["E0601"], "{params}");
    assert_eq!(
        params["diagnostics"][0]["range"],
        json!({ "start": { "line": 0, "character": 0 }, "end": { "line": 0, "character": 0 } })
    );
}
```

- [ ] **Step 2: Run them to verify they fail**

```bash
cd /d/Projects/nona/nova && cargo test --locked -p nova-cli --test lsp 2>&1 | grep -E "^test .*(ok|FAILED)$" | head -16
```

Expected:
- FAIL: `a_module_fixed_on_disk_clears_its_diagnostic`,
  `a_burst_of_edits_ends_with_the_last_edits_diagnostics`,
  `closing_an_unsaved_buffer_rechecks_from_disk`,
  `closing_the_last_file_clears_the_project`,
  `a_secondary_label_becomes_related_information` and
  `a_diagnostic_with_no_place_goes_on_the_entrys_first_line`;
- pass: `an_unreached_project_file_is_checked_as_a_module`, the guard, and
  Task 10's five.

A failure here is a wait timing out after 30 s, so this run takes a few
minutes.

- [ ] **Step 3: Projects in `workspace.rs`**

Add to `crates/nova-lsp/src/workspace.rs`:

```rust

/// A project, or a loose file (spec §6.2). Two keys are equal when their
/// paths' `PathKey`s are.
#[derive(Debug, Clone)]
pub enum ProjectKey {
    /// A directory holding `nova.toml`, in its real spelling; its entry is
    /// `src/main.nova`.
    Root(PathBuf),
    /// A file in no project: its own entry.
    Loose(PathBuf),
}

impl ProjectKey {
    /// The project `path` belongs to: the nearest directory above it that
    /// holds `nova.toml`, as `nova run` finds it, or none.
    pub fn of(path: &Path) -> ProjectKey {
        let dir = path.parent().unwrap_or(path);
        match nova_pm::find_root(dir) {
            Some(root) => ProjectKey::Root(real_path(&root)),
            None => ProjectKey::Loose(path.to_path_buf()),
        }
    }

    /// The file the project's analysis starts from.
    pub fn entry(&self) -> PathBuf {
        match self {
            ProjectKey::Root(dir) => dir.join("src").join("main.nova"),
            ProjectKey::Loose(file) => file.clone(),
        }
    }

    /// Whether `path` belongs to this project.
    pub fn holds(&self, path: &Path) -> bool {
        match self {
            ProjectKey::Root(dir) => PathKey::of(path).is_under(&PathKey::of(dir)),
            ProjectKey::Loose(file) => PathKey::of(path) == PathKey::of(file),
        }
    }

    fn id(&self) -> (bool, PathKey) {
        match self {
            ProjectKey::Root(dir) => (true, PathKey::of(dir)),
            ProjectKey::Loose(file) => (false, PathKey::of(file)),
        }
    }
}

impl PartialEq for ProjectKey {
    fn eq(&self, other: &Self) -> bool {
        self.id() == other.id()
    }
}

impl Eq for ProjectKey {}

impl std::hash::Hash for ProjectKey {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.id().hash(state);
    }
}

/// `path`, canonicalised when it exists, without Windows' `\\?\` prefix, so
/// that it is spelled as the file system spells it. The URIs of unopened
/// files are made from paths built on it, and VS Code keeps an opened
/// file's real spelling.
pub fn real_path(path: &Path) -> PathBuf {
    let real = std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
    match real.to_str().and_then(|s| s.strip_prefix(r"\\?\")) {
        Some(rest) => PathBuf::from(rest),
        None => real,
    }
}
```

and to `impl Workspace`:

```rust
    /// The projects the open documents belong to, each once.
    pub fn projects(&self) -> Vec<ProjectKey> {
        let mut out: Vec<ProjectKey> = Vec::new();
        for doc in self.docs.values() {
            let p = ProjectKey::of(&doc.path);
            if !out.contains(&p) {
                out.push(p);
            }
        }
        out
    }

    /// The open documents that belong to `project`.
    pub fn open_in(&self, project: &ProjectKey) -> Vec<Document> {
        self.docs
            .values()
            .filter(|d| project.holds(&d.path))
            .cloned()
            .collect()
    }
```

- [ ] **Step 4: Related information in `convert.rs`**

Replace `diagnostics_for` with:

```rust
/// The LSP diagnostics `analysis` has for `file`.
///
/// A diagnostic goes to the file of its primary label, or of its first label
/// in one of the program's own files. One with no label in the program's
/// files goes to `entry`'s first line, naming the place it has: the spec
/// §6.3 fallback, for E0601 and for labels in std. Its other labels in the
/// program's files become related information, under the URIs `uri_of`
/// gives their paths.
pub fn diagnostics_for(
    analysis: &Analysis,
    file: FileId,
    entry: FileId,
    uri_of: &dyn Fn(&Path) -> String,
) -> Vec<lsp::Diagnostic> {
    let own: Vec<FileId> = analysis.modules.iter().map(|(f, _)| *f).collect();
    let mut indexes: HashMap<FileId, LineIndex> = HashMap::new();
    let mut out = Vec::new();
    for d in &analysis.diagnostics {
        let label = d
            .labels
            .iter()
            .find(|l| l.primary && own.contains(&l.span.file))
            .or_else(|| d.labels.iter().find(|l| own.contains(&l.span.file)));
        let place = label.map_or(entry, |l| l.span.file);
        if place != file {
            continue;
        }
        let at = match label {
            Some(l) => span_range(analysis, &mut indexes, l.span),
            None => span_range(analysis, &mut indexes, Span::point(0, entry)),
        };
        let mut related = Vec::new();
        for other in &d.labels {
            if label.is_some_and(|l| std::ptr::eq(l, other)) || !own.contains(&other.span.file) {
                continue;
            }
            let Some((_, path)) = analysis.modules.iter().find(|(f, _)| *f == other.span.file)
            else {
                continue;
            };
            let Some(uri) = crate::uri::parse(&uri_of(path)) else {
                continue;
            };
            related.push(lsp::DiagnosticRelatedInformation {
                location: lsp::Location {
                    uri,
                    range: span_range(analysis, &mut indexes, other.span),
                },
                message: other.message.clone(),
            });
        }
        out.push(lsp::Diagnostic {
            range: at,
            severity: Some(severity(d.severity)),
            code: Some(lsp::NumberOrString::String(d.code.clone())),
            source: Some("nova".to_string()),
            message: message(analysis, d, label.is_none()),
            related_information: (!related.is_empty()).then_some(related),
            ..Default::default()
        });
    }
    out
}

/// The LSP range of `span`, indexing its file's text once.
fn span_range(
    analysis: &Analysis,
    indexes: &mut HashMap<FileId, LineIndex>,
    span: Span,
) -> lsp::Range {
    let index = indexes
        .entry(span.file)
        .or_insert_with(|| LineIndex::new(analysis.db.get_source(span.file).unwrap_or("")));
    range(index, span.start, span.end)
}
```

Change the imports at the top of `convert.rs` to:

```rust
use std::collections::HashMap;
use std::path::Path;

use lsp_types as lsp;
use nova_diagnostics::{Diagnostic, FileId, LineIndex, Severity, Span};
use nova_driver::Analysis;
```

- [ ] **Step 5: Replace `checker.rs`**

`crates/nova-lsp/src/checker.rs`:

```rust
//! The checker thread: analyses run here, off the protocol's thread (spec
//! `docs/superpowers/specs/2026-10-08-phase-3-2-lsp-core-design.md` §6.2,
//! §6.6).
//!
//! It runs one job at a time. Jobs queued for the same project collapse to
//! the latest. A finished job is published only if no newer job for its
//! project was submitted while it ran, so a stale result is never shown.

use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::sync::{mpsc, Arc, Mutex};

use nova_driver::{analyze, Analysis, Options, Sources};

use crate::convert;
use crate::uri;
use crate::workspace::{declares_main, Document, Overlay, PathKey, ProjectKey};

/// One project's check.
pub struct Job {
    pub project: ProjectKey,
    /// Rises with every job the server submits.
    pub generation: u64,
    pub overlay: Overlay,
    /// The project's open documents.
    pub open: Vec<Document>,
    /// No document of the project is open: clear what it published.
    pub clear: bool,
}

/// A `textDocument/publishDiagnostics` to send.
pub struct Publish {
    pub uri: String,
    pub version: Option<i32>,
    pub diagnostics: Vec<lsp_types::Diagnostic>,
}

/// The handle the protocol's thread keeps.
pub struct Checker {
    jobs: mpsc::Sender<Job>,
    /// The newest generation submitted for each project.
    newest: Arc<Mutex<HashMap<ProjectKey, u64>>>,
}

impl Checker {
    /// Start the thread. It hands each result to `publish`.
    pub fn spawn(publish: impl Fn(Publish) + Send + 'static) -> Checker {
        let (jobs, queue) = mpsc::channel::<Job>();
        let newest: Arc<Mutex<HashMap<ProjectKey, u64>>> = Arc::default();
        let seen = Arc::clone(&newest);
        std::thread::Builder::new()
            .name("nova-lsp-checker".to_string())
            .spawn(move || serve(&queue, &seen, &publish))
            .expect("start the checker thread");
        Checker { jobs, newest }
    }

    pub fn submit(&self, job: Job) {
        self.newest
            .lock()
            .expect("the newest map")
            .insert(job.project.clone(), job.generation);
        let _ = self.jobs.send(job);
    }
}

fn serve(
    queue: &mpsc::Receiver<Job>,
    newest: &Mutex<HashMap<ProjectKey, u64>>,
    publish: &dyn Fn(Publish),
) {
    // The URIs each project last published for, to clear the ones it no
    // longer reaches.
    let mut published: HashMap<ProjectKey, HashSet<String>> = HashMap::new();
    while let Ok(first) = queue.recv() {
        let mut latest: Vec<Job> = vec![first];
        while let Ok(job) = queue.try_recv() {
            match latest.iter_mut().find(|j| j.project == job.project) {
                Some(slot) => *slot = job,
                None => latest.push(job),
            }
        }
        for job in latest {
            let results = if job.clear { Some(Vec::new()) } else { check(&job) };
            // A panic keeps the last good diagnostics (spec §6.7).
            let Some(results) = results else {
                continue;
            };
            let current = newest.lock().expect("the newest map").get(&job.project).copied();
            if current != Some(job.generation) {
                continue;
            }
            let before = published.remove(&job.project).unwrap_or_default();
            let mut now = HashSet::new();
            for p in results {
                now.insert(p.uri.clone());
                publish(p);
            }
            for uri in before.difference(&now) {
                publish(Publish {
                    uri: uri.clone(),
                    version: None,
                    diagnostics: Vec::new(),
                });
            }
            if !job.clear {
                published.insert(job.project.clone(), now);
            }
        }
    }
}

/// Why an analysis gave nothing.
enum Failed {
    /// The entry could not be read.
    Unreadable,
    /// The front end panicked.
    Panicked,
}

fn run(entry: &Path, overlay: &Overlay, module_only: bool) -> Result<Analysis, Failed> {
    let options = Options {
        keep_going: true,
        tests: true,
        module_only,
        probe: None,
    };
    match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        analyze(entry, overlay, &options)
    })) {
        Ok(Ok(a)) => Ok(a),
        Ok(Err(e)) => {
            tracing::warn!("nova lsp: cannot read {}: {e}", entry.display());
            Err(Failed::Unreadable)
        }
        Err(_) => {
            tracing::warn!("nova lsp: the front end panicked on {}", entry.display());
            Err(Failed::Panicked)
        }
    }
}

/// The diagnostics `job` publishes (spec §6.2's ownership rule):
/// - a project's analysis owns every module its entry reaches;
/// - each open project file the entry does not reach is checked on its own,
///   as a module;
/// - a loose file is its own entry, and a module unless it declares
///   `fn main`.
///
/// `None` after a panic.
fn check(job: &Job) -> Option<Vec<Publish>> {
    let mut out = Vec::new();
    match &job.project {
        ProjectKey::Loose(file) => {
            let module_only = !job
                .overlay
                .read(file)
                .map(|t| declares_main(&t))
                .unwrap_or(true);
            match run(file, &job.overlay, module_only) {
                Ok(a) => publish_own(job, &a, false, &mut out),
                Err(Failed::Panicked) => return None,
                Err(Failed::Unreadable) => {}
            }
        }
        ProjectKey::Root(_) => {
            let mut reached: Vec<PathKey> = Vec::new();
            match run(&job.project.entry(), &job.overlay, false) {
                Ok(a) => {
                    reached = a.modules.iter().map(|(_, p)| PathKey::of(p)).collect();
                    publish_own(job, &a, true, &mut out);
                }
                Err(Failed::Panicked) => return None,
                // No `src/main.nova`: every open file is checked on its own.
                Err(Failed::Unreadable) => {}
            }
            for doc in &job.open {
                if reached.contains(&PathKey::of(&doc.path)) {
                    continue;
                }
                match run(&doc.path, &job.overlay, true) {
                    Ok(a) => publish_own(job, &a, false, &mut out),
                    Err(Failed::Panicked) => return None,
                    Err(Failed::Unreadable) => {}
                }
            }
        }
    }
    Some(out)
}

/// One publish per file `analysis` owns: every module when `all`, else its
/// entry alone.
fn publish_own(job: &Job, analysis: &Analysis, all: bool, out: &mut Vec<Publish>) {
    let Some(&(entry, _)) = analysis.modules.first() else {
        return;
    };
    let uri_of = |path: &Path| match job
        .open
        .iter()
        .find(|d| PathKey::of(&d.path) == PathKey::of(path))
    {
        Some(doc) => doc.uri.clone(),
        None => uri::from_path(path),
    };
    let owned = if all {
        &analysis.modules[..]
    } else {
        &analysis.modules[..1]
    };
    for (file, path) in owned {
        let version = job
            .open
            .iter()
            .find(|d| PathKey::of(&d.path) == PathKey::of(path))
            .map(|d| d.version);
        out.push(Publish {
            uri: uri_of(path),
            version,
            diagnostics: convert::diagnostics_for(analysis, *file, entry, &uri_of),
        });
    }
}
```

- [ ] **Step 6: The server's projects, watcher and notifications**

In `crates/nova-lsp/src/lib.rs`:
- add `use std::collections::HashSet;` and `use std::path::PathBuf;`;
- import `DidChangeWatchedFiles` from `lsp_types::notification`;
- import `RegisterCapability` and `Request as _` from `lsp_types::request`;
- change `use workspace::Workspace;` to `use workspace::{ProjectKey, Workspace};`.

In `serve`, keep the parsed parameters, and read whether the client can
register watchers. Replace `let _params: lsp::InitializeParams = serde_json::from_value(params)?;`
with:

```rust
    let params: lsp::InitializeParams = serde_json::from_value(params)?;
    let can_watch = params
        .capabilities
        .workspace
        .as_ref()
        .and_then(|w| w.did_change_watched_files.as_ref())
        .and_then(|d| d.dynamic_registration)
        .unwrap_or(false);
```

Replace the `let mut server = Server { … };` literal with:

```rust
    let mut server = Server {
        connection,
        workspace: Workspace::default(),
        checker,
        generation: 0,
        active: HashSet::new(),
    };
    if can_watch {
        server.register_watcher();
    }
```

Replace `struct Server` and the whole `impl Server<'_>` with:

```rust
struct Server<'c> {
    connection: &'c Connection,
    workspace: Workspace,
    checker: Checker,
    /// Rises with every job submitted.
    generation: u64,
    /// The projects of the open documents, as of the last refresh.
    active: HashSet<ProjectKey>,
}

impl Server<'_> {
    fn notification(&mut self, n: Notification) {
        match n.method.as_str() {
            DidOpenTextDocument::METHOD => {
                if let Ok(p) = serde_json::from_value::<lsp::DidOpenTextDocumentParams>(n.params) {
                    let uri = uri::text(&p.text_document.uri);
                    if let Some(path) =
                        self.workspace
                            .open(&uri, p.text_document.version, p.text_document.text)
                    {
                        self.refresh(vec![ProjectKey::of(&path)]);
                    }
                }
            }
            DidChangeTextDocument::METHOD => {
                if let Ok(p) = serde_json::from_value::<lsp::DidChangeTextDocumentParams>(n.params)
                {
                    let uri = uri::text(&p.text_document.uri);
                    // Full sync: the last change holds the whole text.
                    if let Some(change) = p.content_changes.into_iter().last() {
                        if let Some(path) =
                            self.workspace
                                .change(&uri, p.text_document.version, change.text)
                        {
                            self.refresh(vec![ProjectKey::of(&path)]);
                        }
                    }
                }
            }
            DidSaveTextDocument::METHOD => {
                if let Ok(p) = serde_json::from_value::<lsp::DidSaveTextDocumentParams>(n.params) {
                    if let Some(path) = uri::document_path(&uri::text(&p.text_document.uri)) {
                        self.refresh(vec![ProjectKey::of(&path)]);
                    }
                }
            }
            DidCloseTextDocument::METHOD => {
                if let Ok(p) = serde_json::from_value::<lsp::DidCloseTextDocumentParams>(n.params)
                {
                    if let Some(doc) = self.workspace.close(&uri::text(&p.text_document.uri)) {
                        // Re-checked from disk if the project is still
                        // open; cleared if not.
                        self.refresh(vec![ProjectKey::of(&doc.path)]);
                    }
                }
            }
            DidChangeWatchedFiles::METHOD => {
                if let Ok(p) = serde_json::from_value::<lsp::DidChangeWatchedFilesParams>(n.params)
                {
                    let paths: Vec<PathBuf> = p
                        .changes
                        .iter()
                        .filter_map(|c| uri::to_path(&uri::text(&c.uri)))
                        .collect();
                    let manifest = paths.iter().any(|p| p.ends_with("nova.toml"));
                    let touched: Vec<ProjectKey> = self
                        .workspace
                        .projects()
                        .into_iter()
                        .filter(|project| manifest || paths.iter().any(|p| project.holds(p)))
                        .collect();
                    self.refresh(touched);
                }
            }
            _ => {}
        }
    }

    fn request(&mut self, request: Request) {
        let response = Response::new_err(
            request.id,
            ErrorCode::MethodNotFound as i32,
            format!("nova lsp does not handle {}", request.method),
        );
        self.respond(response);
    }

    fn respond(&self, response: Response) {
        let _ = self.connection.sender.send(Message::Response(response));
    }

    /// Re-check `touched`, and clear each project that no open document
    /// belongs to any more (spec §6.2).
    fn refresh(&mut self, touched: Vec<ProjectKey>) {
        let now: HashSet<ProjectKey> = self.workspace.projects().into_iter().collect();
        let gone: Vec<ProjectKey> = self.active.difference(&now).cloned().collect();
        for project in gone {
            self.submit(project, true);
        }
        for project in touched {
            if now.contains(&project) {
                self.submit(project, false);
            }
        }
        self.active = now;
    }

    fn submit(&mut self, project: ProjectKey, clear: bool) {
        self.generation += 1;
        let open = self.workspace.open_in(&project);
        self.checker.submit(Job {
            project,
            generation: self.generation,
            overlay: self.workspace.overlay(),
            open,
            clear,
        });
    }

    /// Ask the client to watch `.nova` files and `nova.toml` (spec §6.1).
    fn register_watcher(&self) {
        let watchers = ["**/*.nova", "**/nova.toml"]
            .iter()
            .map(|glob| lsp::FileSystemWatcher {
                glob_pattern: lsp::GlobPattern::String(glob.to_string()),
                kind: None,
            })
            .collect();
        let options = lsp::DidChangeWatchedFilesRegistrationOptions { watchers };
        let params = lsp::RegistrationParams {
            registrations: vec![lsp::Registration {
                id: "nova-watch".to_string(),
                method: DidChangeWatchedFiles::METHOD.to_string(),
                register_options: serde_json::to_value(options).ok(),
            }],
        };
        let request = Request::new(
            lsp_server::RequestId::from("nova-watch".to_string()),
            RegisterCapability::METHOD.to_string(),
            params,
        );
        let _ = self.connection.sender.send(Message::Request(request));
    }
}
```

The client's answer to `client/registerCapability` arrives as a
`Message::Response`, which `serve` already ignores.

- [ ] **Step 7: Run the tests to verify they pass**

```bash
cd /d/Projects/nona/nova && P=/c/Users/SAKEER~1/AppData/Local/Temp/gcm/p32 && cargo test --locked -p nova-lsp -p nova-cli --test lsp > $P/t11.txt 2>&1; echo "exit=$?"; grep -E "^test |FAILED|panicked" $P/t11.txt | head -20
```

Expected: `exit=0`, with all twelve `lsp` tests and the `uri` tests passing.

Then on Linux:

```bash
cd /d/Projects/nona/nova && P=/c/Users/SAKEER~1/AppData/Local/Temp/gcm/p32 && git add -A crates && bash /c/Users/SAKEER~1/AppData/Local/Temp/gcm/linux/run.sh test --locked -p nova-lsp -p nova-cli --test lsp > $P/t11-linux.txt 2>&1; echo "exit=$?"; grep -E "^test |FAILED|panicked" $P/t11-linux.txt | head -20
```

Expected: `exit=0`, with the same tests passing.

- [ ] **Step 8: Commit**

Write `$P/msg-11.txt`:

```
nova-lsp: projects, ownership, watched files, and no stale results

Spec docs/superpowers/specs/2026-10-08-phase-3-2-lsp-core-design.md
§6.2, §6.3 and §6.6.

Projects and ownership:
- A document's project is the nearest directory with nova.toml. Its
  analysis starts at src/main.nova and owns every module it reaches,
  publishing for them whether or not they are open.
- An open file the entry does not reach is checked on its own, as a
  module. A loose file is its own entry, and a module unless it declares
  `fn main`.

Re-checks:
- Closing a file re-checks its project from disk. Closing a project's
  last file clears what it published.
- The server registers a watcher for *.nova and nova.toml when the
  client allows it, and a watched change re-checks the projects that
  hold the file, or every project when a nova.toml changed.

Stale results: the checker collapses queued jobs per project to the
latest, and publishes a result only if no newer job for that project was
submitted while it ran.

A secondary label becomes related information.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
```

```bash
cd /d/Projects/nona/nova && P=/c/Users/SAKEER~1/AppData/Local/Temp/gcm/p32 && cargo fmt --all && git add crates/nova-lsp crates/nova-cli && git commit -q -F $P/msg-11.txt && git log -1 --format=%s
```

Expected: `nova-lsp: projects, ownership, watched files, and no stale results`.

The task's test command: `cargo test --locked -p nova-lsp -p nova-cli --test lsp`.

---
### Task 12: Completion

Spec §6.4 and §9.3 items 2 and 3, decisions 3, 9 and 14, and Review Focus 3.

**How a request is answered:**
- one analysis of the file's owning analysis, with the probe at the cursor;
- after `.`, the probe's members;
- elsewhere, the locals, the module's names, the primitive types and the
  keywords;
- inside a string or comment, nothing.

Completion never needs MIR, so its analyses use `module_only`.

**Files:**
- Create: `crates/nova-lsp/src/completion.rs`
- Modify: `crates/nova-lsp/src/lib.rs` (the capability, `mod completion;`,
  and `request` answering `textDocument/completion`)
- Test: `crates/nova-cli/tests/lsp.rs` (append)

**Interfaces:**
- Consumes:
  - Task 4's `names_in_scope` and `ScopeEntry`;
  - Task 6's probe results;
  - Task 2's `KEYWORDS`;
  - Task 11's `ProjectKey` and `Overlay`.
- Produces: `completion::complete(path: &Path, text: &str, offset: u32, overlay: &Overlay) -> Vec<lsp_types::CompletionItem>`.

- [ ] **Step 1: Write the failing tests**

Append to `crates/nova-cli/tests/lsp.rs`:

```rust

// === Task 12: completion ===

/// The labels of a completion response.
fn labels(response: &Value) -> Vec<String> {
    response["result"]
        .as_array()
        .unwrap_or_else(|| panic!("no items: {response}"))
        .iter()
        .map(|i| i["label"].as_str().unwrap().to_string())
        .collect()
}

/// The LSP position just after the first `marker` in `text`.
fn after(text: &str, marker: &str) -> Value {
    let at = text.find(marker).unwrap() + marker.len();
    let line = text[..at].matches('\n').count();
    let line_start = text[..at].rfind('\n').map_or(0, |i| i + 1);
    json!({ "line": line, "character": text[line_start..at].encode_utf16().count() })
}

fn complete(client: &mut Client, uri: &str, position: Value) -> Value {
    client.request(
        "textDocument/completion",
        json!({ "textDocument": { "uri": uri }, "position": position }),
    )
}

/// Start a server on a loose file holding `text`, opened.
fn loose(name: &str, text: &str) -> (Client, String) {
    let dir = fresh_dir(name);
    let file = dir.join("main.nova");
    std::fs::write(&file, text).unwrap();
    let uri = file_uri(&file);
    let mut client = Client::start(&dir, false);
    open(&mut client, &uri, text);
    (client, uri)
}

const BROKEN_WITH_DOTS: &str = "record Point { x: Int, y: Int }\n\
fn broken( {\n}\n\
fn main() {\n    let p = Point { x: 1, y: 2 }\n    let v: Vec<Int> = Vec::new()\n    p.\n    let n = 1\n    v.\n}\n";

#[test]
fn completion_works_in_a_file_with_a_syntax_error() {
    let (mut client, uri) = loose("complete", BROKEN_WITH_DOTS);
    let diagnostics = client.diagnostics(&uri, nonempty);
    assert!(codes(&diagnostics).contains(&"P0001".to_string()), "{diagnostics}");
    // A record field, after `p.` (gate item 3)...
    let fields = complete(&mut client, &uri, after(BROKEN_WITH_DOTS, "    p."));
    let names = labels(&fields);
    assert!(names.contains(&"x".to_string()) && names.contains(&"y".to_string()), "{names:?}");
    let x = fields["result"].as_array().unwrap().iter().find(|i| i["label"] == "x").unwrap();
    assert_eq!(x["detail"], "Int");
    // ...and a std method, after `v.`, with its declaration as its detail.
    let methods = complete(&mut client, &uri, after(BROKEN_WITH_DOTS, "    v."));
    let names = labels(&methods);
    assert!(names.contains(&"push".to_string()), "{names:?}");
    assert!(!names.contains(&"data".to_string()), "std's private field: {names:?}");
    let push = methods["result"].as_array().unwrap().iter().find(|i| i["label"] == "push").unwrap();
    assert_eq!(push["detail"], "fn push(mut self, x: T)");
}

#[test]
fn completion_after_a_dot_followed_by_a_name_on_the_next_line() {
    let text = "fn main() {\n    let s = \"a\"\n    s.\n    println(\"x\")\n}\n";
    let (mut client, uri) = loose("next-line", text);
    let names = labels(&complete(&mut client, &uri, after(text, "    s.")));
    assert!(names.contains(&"len".to_string()), "{names:?}");
}

#[test]
fn completion_offers_locals_names_types_and_keywords() {
    let text = "fn helper() {}\nfn main() {\n    let count = 1\n    \n}\n";
    let (mut client, uri) = loose("names", text);
    let names = labels(&complete(&mut client, &uri, after(text, "let count = 1\n    ")));
    for want in ["count", "helper", "main", "println", "Vec", "Int", "let", "match"] {
        assert!(names.contains(&want.to_string()), "{want} missing from {names:?}");
    }
}

#[test]
fn no_completion_inside_a_string_or_a_comment() {
    let text = "fn main() {\n    let s = \"in a string\" // in a comment\n}\n";
    let (mut client, uri) = loose("literal", text);
    assert!(labels(&complete(&mut client, &uri, after(text, "in a"))).is_empty());
    assert!(labels(&complete(&mut client, &uri, after(text, "// in"))).is_empty());
}

#[test]
fn completion_in_a_file_main_does_not_import_yet() {
    // Review Focus 3: a new module, before `main.nova` imports it.
    let extra = "fn helper() {\n    let s = \"a\"\n    s.\n}\n";
    let dir = project("not-yet", &[("main.nova", "fn main() {}\n"), ("extra.nova", extra)]);
    let uri = file_uri(&dir.join("src").join("extra.nova"));
    let mut client = Client::start(&dir, false);
    open(&mut client, &uri, extra);
    let names = labels(&complete(&mut client, &uri, after(extra, "    s.")));
    assert!(names.contains(&"len".to_string()), "{names:?}");
}
```

- [ ] **Step 2: Run them to verify they fail**

```bash
cd /d/Projects/nona/nova && cargo test --locked -p nova-cli --test lsp -- completion 2>&1 | grep -E "^test .*(ok|FAILED)$" | head -8
```

Expected: the five tests FAIL. The server answers `textDocument/completion`
with `MethodNotFound`, so `labels` panics with "no items".

- [ ] **Step 3: Write `completion.rs`**

`crates/nova-lsp/src/completion.rs`:

```rust
//! Completion (spec
//! `docs/superpowers/specs/2026-10-08-phase-3-2-lsp-core-design.md` §6.4).

use std::collections::HashSet;
use std::path::Path;

use lsp_types as lsp;
use lsp_types::CompletionItemKind as Kind;
use nova_diagnostics::FileId;
use nova_driver::{analyze, Analysis, Options, Probe};
use nova_lexer::Token;
use nova_resolver::{DefKind, ModuleId, Res, ScopeEntry};
use nova_typeck::{Member, MemberKind};

use crate::workspace::{Overlay, PathKey, ProjectKey};

/// The completion items at byte `offset` of `text`, the buffer for `path`.
pub fn complete(path: &Path, text: &str, offset: u32, overlay: &Overlay) -> Vec<lsp::CompletionItem> {
    if in_literal_or_comment(text, offset) {
        return Vec::new();
    }
    let Some(analysis) = analysis_at(path, offset, overlay) else {
        return Vec::new();
    };
    if analysis.probe.receiver.is_some() {
        members(&analysis)
    } else {
        names(&analysis, path)
    }
}

/// The analysis that owns `path`, with the probe at `offset` (decision 9):
/// its project's, if the entry reaches it, or else its own as a module.
/// Completion needs no MIR, so `module_only` is always on.
fn analysis_at(path: &Path, offset: u32, overlay: &Overlay) -> Option<Analysis> {
    let options = Options {
        keep_going: true,
        tests: true,
        module_only: true,
        probe: Some(Probe {
            path: path.to_path_buf(),
            offset,
        }),
    };
    let project = ProjectKey::of(path);
    if let ProjectKey::Root(_) = project {
        if let Some(a) = guarded(|| analyze(&project.entry(), overlay, &options).ok()) {
            if a.modules.iter().any(|(_, p)| PathKey::of(p) == PathKey::of(path)) {
                return Some(a);
            }
        }
    }
    // A loose file, or a project file its entry does not reach: either way
    // it is analysed as its own entry.
    guarded(|| analyze(path, overlay, &options).ok())
}

fn guarded<T>(f: impl FnOnce() -> Option<T>) -> Option<T> {
    match std::panic::catch_unwind(std::panic::AssertUnwindSafe(f)) {
        Ok(v) => v,
        Err(_) => {
            tracing::warn!("nova lsp: the front end panicked during completion");
            None
        }
    }
}

/// Whether `offset` is inside a string, a character literal or a comment
/// (decision 14). Interpolations, `${…}`, are code.
fn in_literal_or_comment(text: &str, offset: u32) -> bool {
    let (tokens, comments, _) = nova_lexer::lex_with_comments(text, FileId::DUMMY);
    if comments
        .iter()
        .any(|c| c.span.start < offset && offset <= c.span.end)
    {
        return true;
    }
    tokens.iter().any(|t| {
        let (start, end) = (t.span.start, t.span.end);
        match t.value {
            Token::StrPart(_) | Token::RawStr(_) | Token::Char(_) => start < offset && offset < end,
            // Just inside the opening or closing quote.
            Token::StrStart => end == offset,
            Token::StrEnd => start == offset,
            _ => false,
        }
    })
}

fn item(label: &str, kind: Kind, detail: Option<String>) -> lsp::CompletionItem {
    lsp::CompletionItem {
        label: label.to_string(),
        kind: Some(kind),
        detail,
        ..Default::default()
    }
}

/// After `.`: the receiver's members (spec §4.2).
fn members(analysis: &Analysis) -> Vec<lsp::CompletionItem> {
    analysis
        .probe
        .members
        .iter()
        .map(|m| {
            let kind = match m.kind {
                MemberKind::Field => Kind::FIELD,
                MemberKind::Method => Kind::METHOD,
            };
            item(&m.name, kind, Some(member_detail(analysis, m)))
        })
        .collect()
}

/// A member's declaration as written, on one line (decision 3): a field's
/// type, or `fn` and a method's signature.
fn member_detail(analysis: &Analysis, m: &Member) -> String {
    let Some(span) = m.decl else {
        return "fn len(self) -> Int".to_string();
    };
    let Some(source) = analysis.db.get_source(span.file) else {
        return String::new();
    };
    match m.kind {
        MemberKind::Field => collapse(&source[span.start as usize..span.end as usize]),
        MemberKind::Method => format!("fn {}", collapse(signature(&source[span.start as usize..]))),
    }
}

/// From a method's name to its body: up to the first `{` or `;`, or the end
/// of its line, outside parentheses and brackets.
fn signature(from_name: &str) -> &str {
    let mut depth = 0i32;
    for (i, c) in from_name.char_indices() {
        match c {
            '(' | '[' => depth += 1,
            ')' | ']' => depth -= 1,
            '{' | ';' | '\n' if depth <= 0 => return &from_name[..i],
            _ => {}
        }
    }
    from_name
}

fn collapse(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Anywhere else:
/// - the locals at the cursor;
/// - the names the cursor's module sees;
/// - the primitive types;
/// - the keywords.
///
/// A local hides a module name of the same spelling.
fn names(analysis: &Analysis, path: &Path) -> Vec<lsp::CompletionItem> {
    let mut items = Vec::new();
    let mut seen: HashSet<String> = HashSet::new();
    let defs = analysis.definitions.as_ref();
    for (name, ty) in &analysis.probe.locals {
        if seen.insert(name.clone()) {
            let detail = defs.map(|d| nova_typeck::display_ty(ty, d));
            items.push(item(name, Kind::VARIABLE, detail));
        }
    }
    if let Some(defs) = defs {
        let module = analysis
            .modules
            .iter()
            .position(|(_, p)| PathKey::of(p) == PathKey::of(path))
            .unwrap_or(0);
        for (name, entry) in defs.names_in_scope(ModuleId(module as u32)) {
            if seen.contains(&name) {
                continue;
            }
            let kind = match entry {
                ScopeEntry::Value(Res::Def(id)) => match defs.def(id).kind {
                    DefKind::Const { .. } => Kind::CONSTANT,
                    _ => Kind::FUNCTION,
                },
                ScopeEntry::Value(Res::Variant(..)) => Kind::ENUM_MEMBER,
                ScopeEntry::Value(Res::Builtin(_)) => Kind::FUNCTION,
                ScopeEntry::Type(id) => match defs.def(id).kind {
                    DefKind::Sum { .. } => Kind::ENUM,
                    _ => Kind::STRUCT,
                },
                ScopeEntry::Trait(_) => Kind::INTERFACE,
            };
            items.push(item(&name, kind, None));
        }
    }
    for ty in nova_resolver::RESERVED_TYPE_NAMES {
        if !seen.contains(ty) {
            items.push(item(ty, Kind::STRUCT, None));
        }
    }
    for keyword in nova_lexer::KEYWORDS {
        items.push(item(keyword, Kind::KEYWORD, None));
    }
    items
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_signature_stops_at_its_body_outside_parentheses() {
        assert_eq!(signature("push(mut self, x: T) {\n"), "push(mut self, x: T) ");
        assert_eq!(signature("fmt(self) -> String\n"), "fmt(self) -> String");
        assert_eq!(
            signature("map<U>(self, f: fn(T) -> U) -> Map<U> { x }"),
            "map<U>(self, f: fn(T) -> U) -> Map<U> "
        );
        assert_eq!(collapse("push(mut self,\n    x: T) "), "push(mut self, x: T)");
    }

    #[test]
    fn strings_chars_and_comments_are_literal_places() {
        let text = "let s = \"ab\" // c\nlet t = 'x'\n";
        let at = |s: &str| text.find(s).unwrap() as u32;
        assert!(in_literal_or_comment(text, at("ab") + 1));
        assert!(in_literal_or_comment(text, at("\"ab") + 1));
        assert!(in_literal_or_comment(text, at("// c") + 3));
        assert!(!in_literal_or_comment(text, at("let s")));
        assert!(!in_literal_or_comment(text, at(" //")));
    }
}
```

- [ ] **Step 4: Answer `textDocument/completion`**

In `crates/nova-lsp/src/lib.rs`:
- add `mod completion;`;
- add `use nova_diagnostics::LineIndex;`;
- import `Completion` from `lsp_types::request`.

In `capabilities()`, add before `..Default::default()`:

```rust
        completion_provider: Some(lsp::CompletionOptions {
            trigger_characters: Some(vec![".".to_string()]),
            ..Default::default()
        }),
```

Replace `fn request` with:

```rust
    fn request(&mut self, request: Request) {
        let response = match request.method.as_str() {
            Completion::METHOD => match serde_json::from_value::<lsp::CompletionParams>(request.params)
            {
                Ok(p) => {
                    let at = p.text_document_position;
                    let uri = uri::text(&at.text_document.uri);
                    let items = match self.workspace.get(&uri) {
                        Some(doc) => {
                            let offset = LineIndex::new(&doc.text)
                                .offset(at.position.line, at.position.character);
                            completion::complete(&doc.path, &doc.text, offset, &self.workspace.overlay())
                        }
                        None => Vec::new(),
                    };
                    Response::new_ok(request.id, lsp::CompletionResponse::Array(items))
                }
                Err(e) => Response::new_err(request.id, ErrorCode::InvalidParams as i32, e.to_string()),
            },
            _ => Response::new_err(
                request.id,
                ErrorCode::MethodNotFound as i32,
                format!("nova lsp does not handle {}", request.method),
            ),
        };
        self.respond(response);
    }
```

- [ ] **Step 5: Run the tests to verify they pass**

```bash
cd /d/Projects/nona/nova && P=/c/Users/SAKEER~1/AppData/Local/Temp/gcm/p32 && cargo test --locked -p nova-lsp -p nova-cli --test lsp > $P/t12.txt 2>&1; echo "exit=$?"; grep -E "^test |FAILED|panicked" $P/t12.txt | head -24
```

Expected: `exit=0`. All `lsp` tests pass, the five new ones included, and
`nova-lsp`'s unit tests pass, the two new ones included.

If `fn push(mut self, x: T)` comes out with different spacing, read
std/collections' declaration. Then correct the expected string only if it
matches what is written there, and ledger it.

- [ ] **Step 6: Commit**

Write `$P/msg-12.txt`:

```
nova-lsp: completion

`textDocument/completion` analyses the file's owning analysis with the
probe at the cursor (spec
docs/superpowers/specs/2026-10-08-phase-3-2-lsp-core-design.md §6.4):
its project's, or its own as a module when the entry does not reach it.
- After `.`: the receiver's members. Each one's detail is its
  declaration as written, such as `fn push(mut self, x: T)`.
- Elsewhere:
  - the locals;
  - the names the module sees (items, imports, builtins, std);
  - the primitive types;
  - the keywords.
- Inside a string, a character or a comment: nothing.

The trigger character is `.`. Completion skips MIR.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
```

```bash
cd /d/Projects/nona/nova && P=/c/Users/SAKEER~1/AppData/Local/Temp/gcm/p32 && cargo fmt --all && git add crates/nova-lsp crates/nova-cli && git commit -q -F $P/msg-12.txt && git log -1 --format=%s
```

Expected: `nova-lsp: completion`.

The task's test command: `cargo test --locked -p nova-lsp -p nova-cli --test lsp`.

---
### Task 13: Document formatting

Spec §6.5 and §9.3 item 4. `textDocument/formatting` formats the buffer
with Task 9's `format_buffer`. It answers in one of two ways:
- one edit replacing the whole document;
- no edit, if the buffer is already formatted, has a syntax error, or the
  self-check refuses the output.

**Files:**
- Create: `crates/nova-lsp/src/formatting.rs`
- Modify: `crates/nova-lsp/src/lib.rs` (the capability, `mod formatting;`,
  the `Formatting` arm of `request`)
- Test: `crates/nova-cli/tests/lsp.rs` (append)

**Interfaces:**
- Consumes:
  - Task 9's `nova_fmt::format_buffer`;
  - Task 10's `convert::range`;
  - Task 12's `request` match.
- Produces: `formatting::format_document(path: &Path, text: &str) -> Vec<lsp_types::TextEdit>`.

- [ ] **Step 1: Write the failing tests**

Append to `crates/nova-cli/tests/lsp.rs`:

```rust

// === Task 13: formatting ===

/// Format the open document `uri`, and return the response.
fn format(client: &mut Client, uri: &str) -> Value {
    client.request(
        "textDocument/formatting",
        json!({ "textDocument": { "uri": uri }, "options": { "tabSize": 4, "insertSpaces": true } }),
    )
}

/// A loose file holding `text`, opened, in a directory whose `.editorconfig`
/// says `root = true`, so none above it can change line endings.
fn formattable(name: &str, text: &str) -> (Client, String) {
    let dir = fresh_dir(name);
    std::fs::write(dir.join(".editorconfig"), "root = true\n").unwrap();
    let file = dir.join("main.nova");
    std::fs::write(&file, text).unwrap();
    let uri = file_uri(&file);
    let mut client = Client::start(&dir, false);
    open(&mut client, &uri, text);
    (client, uri)
}

#[test]
fn the_document_is_formatted() {
    let text = "fn main() {\nprintln(\"hi\")\n}\n";
    let (mut client, uri) = formattable("format", text);
    let response = format(&mut client, &uri);
    let edits = response["result"].as_array().unwrap_or_else(|| panic!("{response}"));
    assert_eq!(edits.len(), 1, "{response}");
    assert_eq!(edits[0]["newText"], "fn main() { println(\"hi\") }\n");
    assert_eq!(
        edits[0]["range"],
        json!({ "start": { "line": 0, "character": 0 }, "end": { "line": 3, "character": 0 } })
    );
}

#[test]
fn a_formatted_or_broken_buffer_gets_no_edit() {
    let (mut client, uri) = formattable("format-none", "fn main() { println(\"hi\") }\n");
    assert_eq!(format(&mut client, &uri)["result"], json!([]));
    let (mut client, uri) = formattable("format-broken", "fn main( {\n");
    assert_eq!(format(&mut client, &uri)["result"], json!([]));
}
```

- [ ] **Step 2: Run them to verify they fail**

```bash
cd /d/Projects/nona/nova && cargo test --locked -p nova-cli --test lsp -- format 2>&1 | grep -E "^test .*(ok|FAILED)$" | head -4
```

Expected: both FAIL. The server answers `MethodNotFound`, so `result` is
null.

- [ ] **Step 3: Write `formatting.rs` and answer the request**

`crates/nova-lsp/src/formatting.rs`:

```rust
//! Document formatting (spec
//! `docs/superpowers/specs/2026-10-08-phase-3-2-lsp-core-design.md` §6.5).

use std::path::Path;

use lsp_types as lsp;
use nova_diagnostics::LineIndex;

/// The edits that format `text`, the buffer for `path`:
/// - one replacing the whole document;
/// - none if the buffer is already formatted, has a syntax error, or the
///   self-check refuses the output.
///
/// A refusal goes to the log, so format-on-save never raises an error.
pub fn format_document(path: &Path, text: &str) -> Vec<lsp::TextEdit> {
    match nova_fmt::format_buffer(path, text) {
        Ok(formatted) if formatted == text => Vec::new(),
        Ok(formatted) => {
            let index = LineIndex::new(text);
            vec![lsp::TextEdit {
                range: crate::convert::range(&index, 0, index.len()),
                new_text: formatted,
            }]
        }
        Err(e) => {
            tracing::warn!("nova lsp: not formatting {}: {e}", path.display());
            Vec::new()
        }
    }
}
```

In `crates/nova-lsp/src/lib.rs`, add `mod formatting;` and import
`Formatting` from `lsp_types::request`. In `capabilities()`, add before
`..Default::default()`:

```rust
        document_formatting_provider: Some(lsp::OneOf::Left(true)),
```

In `request`'s `match`, add before the `_ =>` arm:

```rust
            Formatting::METHOD => {
                match serde_json::from_value::<lsp::DocumentFormattingParams>(request.params) {
                    Ok(p) => {
                        let uri = uri::text(&p.text_document.uri);
                        let edits = match self.workspace.get(&uri) {
                            Some(doc) => formatting::format_document(&doc.path, &doc.text),
                            None => Vec::new(),
                        };
                        Response::new_ok(request.id, edits)
                    }
                    Err(e) => {
                        Response::new_err(request.id, ErrorCode::InvalidParams as i32, e.to_string())
                    }
                }
            }
```

- [ ] **Step 4: Run the tests to verify they pass**

```bash
cd /d/Projects/nona/nova && P=/c/Users/SAKEER~1/AppData/Local/Temp/gcm/p32 && cargo test --locked -p nova-lsp -p nova-cli --test lsp > $P/t13.txt 2>&1; echo "exit=$?"; grep -E "^test |FAILED|panicked" $P/t13.txt | head -26
```

Expected: `exit=0`, with every `lsp` test passing, the two new ones
included.

- [ ] **Step 5: Commit**

Write `$P/msg-13.txt`:

```
nova-lsp: document formatting

`textDocument/formatting` formats the buffer with nova_fmt::format_buffer
(spec docs/superpowers/specs/2026-10-08-phase-3-2-lsp-core-design.md
§6.5). It applies the .editorconfig that governs the buffer's path, and
answers with one edit replacing the whole document. It answers with no
edit when the buffer is already formatted, has a syntax error, or the
self-check refuses the output; the reason goes to the log, so
format-on-save never raises an error.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
```

```bash
cd /d/Projects/nona/nova && P=/c/Users/SAKEER~1/AppData/Local/Temp/gcm/p32 && cargo fmt --all && git add crates/nova-lsp crates/nova-cli && git commit -q -F $P/msg-13.txt && git log -1 --format=%s
```

Expected: `nova-lsp: document formatting`.

The task's test command: `cargo test --locked -p nova-lsp -p nova-cli --test lsp`.

---
### Task 14: The latency budget, measured and bounded

Spec §6.8, §9.3 item 8, §9.5, and the gate's item 3. The gate test asserts
CI's 2 s bound on every system. An ignored test measures the 200 ms budget
on the development host in a release build.

**Files:**
- Test: `crates/nova-cli/tests/lsp.rs` (append)

**Interfaces:**
- Consumes: Tasks 10 to 13's server, and `lsp.rs`'s helpers.
- Produces: the figures Task 17 records.

- [ ] **Step 1: Write the tests**

Append to `crates/nova-cli/tests/lsp.rs`:

```rust

// === Task 14: the latency budget (spec §6.8, §9.5) ===

use std::time::Instant;

/// The largest example, which the budget is measured on.
fn json_api() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("examples")
        .join("05-json-api")
        .join("src")
        .join("main.nova")
}

/// Time `n` edits, each sent when no check is running, and `n` completions,
/// on `05-json-api`. Returns each list in milliseconds, sorted.
fn measure(n: usize) -> (Vec<u128>, Vec<u128>) {
    let path = json_api();
    let text = std::fs::read_to_string(&path).unwrap();
    let uri = file_uri(&path);
    let mut client = Client::start(path.parent().unwrap(), false);
    open(&mut client, &uri, &text);
    client.diagnostics(&uri, |p| p["version"] == 1);
    let mut edits = Vec::new();
    for k in 0..n {
        let version = 2 + k as i32;
        let edited = format!("{text}// edit {k}\n");
        let started = Instant::now();
        change(&mut client, &uri, version, &edited);
        client.diagnostics(&uri, |p| p["version"] == version);
        edits.push(started.elapsed().as_millis());
    }
    // `self.users.` holds a `Map`: completion after it lists std's members.
    let position = after(&text, "        self.users.");
    let mut completions = Vec::new();
    for _ in 0..n {
        let started = Instant::now();
        let response = complete(&mut client, &uri, position.clone());
        completions.push(started.elapsed().as_millis());
        assert!(labels(&response).contains(&"insert".to_string()), "{response}");
    }
    edits.sort_unstable();
    completions.sort_unstable();
    (edits, completions)
}

fn summary(name: &str, ms: &[u128]) -> String {
    format!(
        "{name}: median {} ms, max {} ms, of {}",
        ms[ms.len() / 2],
        ms[ms.len() - 1],
        ms.len()
    )
}

#[test]
fn edits_and_completions_stay_within_the_ci_bound() {
    // Gate item 8: 2 s each, with the debug binary.
    let (edits, completions) = measure(3);
    let bound = 2000;
    assert!(
        edits.iter().chain(&completions).all(|&ms| ms <= bound),
        "{}; {}; the CI bound is {bound} ms",
        summary("edits", &edits),
        summary("completions", &completions)
    );
}

#[test]
#[ignore = "the development host's figure (spec §9.5): cargo test --release -p nova-cli --test lsp -- --ignored --nocapture latency"]
fn latency_on_05_json_api() {
    let (edits, completions) = measure(20);
    // In a release build, the 200 ms budget. CI's advisory `--ignored` step
    // runs this in a debug build, where only the 2 s CI bound applies.
    let bound = if cfg!(debug_assertions) { 2000 } else { 200 };
    let binary = assert_cmd::cargo::cargo_bin("nova");
    let meta = std::fs::metadata(&binary).unwrap();
    println!("{}", summary("edit to diagnostics", &edits));
    println!("{}", summary("completion", &completions));
    println!(
        "binary {} ({} bytes, modified {:?})",
        binary.display(),
        meta.len(),
        meta.modified().ok()
    );
    assert!(
        edits.iter().chain(&completions).all(|&ms| ms <= bound),
        "the budget is {bound} ms for the median and the maximum"
    );
}
```

- [ ] **Step 2: Run the CI bound**

```bash
cd /d/Projects/nona/nova && cargo test --locked -p nova-cli --test lsp -- edits_and_completions 2>&1 | grep -E "^test |panicked|CI bound" | head -4
```

Expected: `test edits_and_completions_stay_within_the_ci_bound ... ok`.

This test is written after the code it measures, because the budget is a
property of the finished server. A mutant shows that it can fail: put
`std::thread::sleep(std::time::Duration::from_millis(2500));` at the top of
`completion::complete`, run Step 2, and expect it to fail. Then undo it with
`git checkout -- crates/nova-lsp/src/completion.rs`.

- [ ] **Step 3: Measure the budget on the development host**

Check that port 3000 is free (not needed here, but the Conventions' habit),
then build in release first so the measured binary is fresh:

```bash
cd /d/Projects/nona/nova && P=/c/Users/SAKEER~1/AppData/Local/Temp/gcm/p32 && cargo build --release --locked -p nova-cli 2>&1 | tail -1 && cargo test --release --locked -p nova-cli --test lsp -- --ignored --nocapture latency > $P/t14-latency.txt 2>&1; echo "exit=$?"; grep -E "median|binary|test latency" $P/t14-latency.txt
```

Expected:
- `exit=0`;
- the edit-to-diagnostics line and the completion line, each with a
  median and a maximum of at most 200 ms;
- the binary line with its size.

Run it three times, and ledger all three runs' lines; they are Task 17's
figures.

**If the maximum exceeds 200 ms in two of the three runs, stop and ask the
user** (gate item 4: adopting `salsa` becomes its own step). Do not tune the
test to pass.

- [ ] **Step 4: Commit**

Write `$P/msg-14.txt`:

```
nova-cli: the language server's latency, bounded and measured

On examples/05-json-api (spec
docs/superpowers/specs/2026-10-08-phase-3-2-lsp-core-design.md §6.8,
§9.5), from an edit to its diagnostics and from a completion request to
its answer:
- The gate test asserts CI's 2 s bound with the debug binary, on every
  system.
- An ignored test times 20 of each and prints the median, the maximum
  and the binary measured. It asserts the 200 ms budget in a release
  build, and only the 2 s bound in CI's debug --ignored step.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
```

```bash
cd /d/Projects/nona/nova && P=/c/Users/SAKEER~1/AppData/Local/Temp/gcm/p32 && cargo fmt --all && git add crates/nova-cli && git commit -q -F $P/msg-14.txt && git log -1 --format=%s
```

Expected: `nova-cli: the language server's latency, bounded and measured`.

The task's test command: `cargo test --locked -p nova-cli --test lsp`.

---
### Task 15: The VS Code extension

Spec §7, §9.6 and §10, and decisions 12 and 13. The Rust side checks two
things the compiler knows: the grammar's keywords, and the extension's
version. The smoke test and the `.vsix` need npm, and **npm's first install
needs the user's word** (spec §10).

**Files (all new unless noted):**
- `tools/vscode-nova/package.json`, `tsconfig.json`, `.vscodeignore`,
  `README.md`, `LICENSE`, `LICENSE-MIT`, `LICENSE-APACHE`
- `tools/vscode-nova/src/extension.ts`
- `tools/vscode-nova/syntaxes/nova.tmLanguage.json`
- `tools/vscode-nova/language-configuration.json`
- `tools/vscode-nova/test/run.ts`, `test/suite/index.ts`,
  `test/suite/smoke.test.ts`
- `tools/vscode-nova/test/fixture/nova.toml`, `.editorconfig`,
  `src/main.nova`, `src/complete.nova`, `src/unformatted.nova`
- `tools/vscode-nova/package-lock.json`, written by `npm install` in Step 5
- Modify: `.gitignore`
- Create: `crates/nova-cli/tests/vscode_extension.rs`

**Interfaces:**
- Consumes: Task 2's `nova_lexer::KEYWORDS`; `nova lsp` (Tasks 10 to 13).
- Produces, for Task 16's CI job:
  - `npm run compile` and `npm test`;
  - `npx vsce package --out nova-vscode-<version>.vsix`.

- [ ] **Step 1: Write the failing Rust tests**

`crates/nova-cli/tests/vscode_extension.rs`:

```rust
//! The VS Code extension's files agree with the compiler (spec
//! `docs/superpowers/specs/2026-10-08-phase-3-2-lsp-core-design.md` §7.2,
//! §7.4).

use std::path::PathBuf;

use serde_json::Value;

fn extension() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("tools")
        .join("vscode-nova")
}

fn json(file: &str) -> Value {
    let path = extension().join(file);
    let text = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    serde_json::from_str(&text).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

#[test]
fn the_grammar_colours_every_keyword_and_nothing_else() {
    let grammar = json("syntaxes/nova.tmLanguage.json");
    let pattern = grammar["repository"]["keywords"]["match"]
        .as_str()
        .expect("a keywords pattern");
    let inner = pattern
        .strip_prefix("\\b(")
        .and_then(|p| p.strip_suffix(")\\b"))
        .unwrap_or_else(|| panic!("not \\b(…)\\b: {pattern}"));
    let mut listed: Vec<&str> = inner.split('|').collect();
    let mut keywords: Vec<&str> = nova_lexer::KEYWORDS.to_vec();
    listed.sort_unstable();
    keywords.sort_unstable();
    assert_eq!(listed, keywords);
}

#[test]
fn the_extension_has_nova_clis_version_and_its_floor() {
    let package = json("package.json");
    assert_eq!(package["version"], env!("CARGO_PKG_VERSION"));
    assert_eq!(package["engines"]["vscode"], "^1.91.0");
    assert_eq!(package["publisher"], "sakeerin");
    assert_eq!(package["contributes"]["languages"][0]["id"], "nova");
}
```

```bash
cd /d/Projects/nona/nova && cargo test --locked -p nova-cli --test vscode_extension 2>&1 | grep -E "^test |panicked" | head -4
```

Expected: both FAIL, with `tools/vscode-nova/...: The system cannot find
the path specified` (or `No such file or directory`).

- [ ] **Step 2: The manifest and the configuration files**

`tools/vscode-nova/package.json`:

```json
{
  "name": "nova",
  "displayName": "Nova",
  "description": "The Nova language: colouring, and diagnostics, completion and formatting from nova lsp.",
  "version": "0.2.0",
  "publisher": "sakeerin",
  "license": "MIT OR Apache-2.0",
  "repository": {
    "type": "git",
    "url": "https://github.com/Sakeerin/nova"
  },
  "engines": {
    "vscode": "^1.91.0"
  },
  "categories": [
    "Programming Languages",
    "Formatters"
  ],
  "activationEvents": [
    "onLanguage:nova"
  ],
  "main": "./out/src/extension.js",
  "contributes": {
    "languages": [
      {
        "id": "nova",
        "aliases": ["Nova", "nova"],
        "extensions": [".nova"],
        "configuration": "./language-configuration.json"
      }
    ],
    "grammars": [
      {
        "language": "nova",
        "scopeName": "source.nova",
        "path": "./syntaxes/nova.tmLanguage.json"
      }
    ],
    "configuration": {
      "title": "Nova",
      "properties": {
        "nova.server.path": {
          "type": "string",
          "default": "",
          "description": "The nova executable to run as the language server. Empty means nova from PATH."
        },
        "nova.trace.server": {
          "type": "string",
          "enum": ["off", "messages", "verbose"],
          "default": "off",
          "description": "Trace the messages between VS Code and nova lsp."
        }
      }
    },
    "commands": [
      {
        "command": "nova.restartServer",
        "title": "Nova: Restart Language Server"
      }
    ]
  },
  "scripts": {
    "compile": "tsc -p ./",
    "test": "node ./out/test/run.js"
  },
  "dependencies": {
    "vscode-languageclient": "^10.1.2"
  },
  "devDependencies": {
    "@types/mocha": "^10.0.10",
    "@types/node": "^22.0.0",
    "@types/vscode": "~1.91.0",
    "@vscode/test-electron": "^3.1.0",
    "@vscode/vsce": "^4.0.0",
    "mocha": "^12.0.3",
    "typescript": "~5.9.3"
  }
}
```

`tools/vscode-nova/tsconfig.json`:

```json
{
  "compilerOptions": {
    "module": "commonjs",
    "target": "ES2022",
    "lib": ["ES2022"],
    "outDir": "out",
    "rootDir": ".",
    "strict": true,
    "esModuleInterop": true,
    "sourceMap": true,
    "skipLibCheck": true
  },
  "include": ["src", "test"]
}
```

`tools/vscode-nova/language-configuration.json`:

```json
{
  "comments": {
    "lineComment": "//",
    "blockComment": ["/*", "*/"]
  },
  "brackets": [["{", "}"], ["[", "]"], ["(", ")"]],
  "autoClosingPairs": [
    { "open": "{", "close": "}" },
    { "open": "[", "close": "]" },
    { "open": "(", "close": ")" },
    { "open": "\"", "close": "\"", "notIn": ["string", "comment"] },
    { "open": "'", "close": "'", "notIn": ["string", "comment"] }
  ],
  "surroundingPairs": [["{", "}"], ["[", "]"], ["(", ")"], ["\"", "\""], ["'", "'"]],
  "indentationRules": {
    "increaseIndentPattern": "^.*\\{[^}\"']*$",
    "decreaseIndentPattern": "^\\s*\\}"
  }
}
```

`tools/vscode-nova/.vscodeignore`:

```
.vscode-test/**
src/**
test/**
out/test/**
**/*.map
tsconfig.json
```

`tools/vscode-nova/LICENSE`:

```
The Nova extension for VS Code is dual-licensed under the MIT licence
(LICENSE-MIT) or the Apache License 2.0 (LICENSE-APACHE), at your option,
as Nova itself is.
```

Copy the repository's licences beside it:

```bash
cd /d/Projects/nona/nova && cp LICENSE-MIT LICENSE-APACHE tools/vscode-nova/
```

`tools/vscode-nova/README.md`:

```markdown
# Nova for VS Code

Colouring for `.nova` files, and diagnostics, completion and formatting
from `nova lsp`.

The extension runs the `nova` you have installed: the `nova.server.path`
setting, or else `nova` from your PATH. Install it with

    cargo install --locked --git https://github.com/Sakeerin/nova nova-cli

**Nova: Restart Language Server** restarts it, for example after installing
a newer `nova`.
```

Add to the root `.gitignore`:

```
/tools/vscode-nova/node_modules
/tools/vscode-nova/out
/tools/vscode-nova/.vscode-test
/tools/vscode-nova/*.vsix
/tools/vscode-nova/test/fixture/.vscode
```

- [ ] **Step 3: The grammar and the client**

`tools/vscode-nova/syntaxes/nova.tmLanguage.json`:

```json
{
  "$schema": "https://raw.githubusercontent.com/martinring/tmlanguage/master/tmlanguage.json",
  "name": "Nova",
  "scopeName": "source.nova",
  "fileTypes": ["nova"],
  "patterns": [
    { "include": "#comments" },
    { "include": "#strings" },
    { "include": "#chars" },
    { "include": "#numbers" },
    { "include": "#attributes" },
    { "include": "#function-names" },
    { "include": "#keywords" },
    { "include": "#types" }
  ],
  "repository": {
    "comments": {
      "patterns": [
        { "name": "comment.line.documentation.nova", "match": "///(?!/).*$" },
        { "name": "comment.line.double-slash.nova", "match": "//.*$" },
        { "name": "comment.block.nova", "begin": "/\\*", "end": "\\*/" }
      ]
    },
    "strings": {
      "patterns": [
        { "name": "string.quoted.raw.nova", "begin": "r(#*)\"", "end": "\"\\1" },
        {
          "name": "string.quoted.double.nova",
          "begin": "\"",
          "end": "\"",
          "patterns": [
            { "name": "constant.character.escape.nova", "match": "\\\\." },
            {
              "name": "meta.interpolation.nova",
              "begin": "\\$\\{",
              "end": "\\}",
              "beginCaptures": { "0": { "name": "punctuation.section.interpolation.begin.nova" } },
              "endCaptures": { "0": { "name": "punctuation.section.interpolation.end.nova" } },
              "patterns": [{ "include": "$self" }]
            }
          ]
        }
      ]
    },
    "chars": { "name": "string.quoted.single.nova", "match": "'(\\\\.|[^'\\\\])'" },
    "numbers": { "name": "constant.numeric.nova", "match": "\\b\\d[\\d_]*(\\.\\d[\\d_]*)?\\b" },
    "attributes": { "name": "entity.name.function.decorator.nova", "match": "@[A-Za-z_][A-Za-z0-9_]*" },
    "function-names": {
      "match": "\\b(fn)\\s+([A-Za-z_][A-Za-z0-9_]*)",
      "captures": {
        "1": { "name": "keyword.other.fn.nova" },
        "2": { "name": "entity.name.function.nova" }
      }
    },
    "keywords": {
      "name": "keyword.other.nova",
      "match": "\\b(let|mut|const|fn|return|if|else|while|for|in|break|continue|match|type|record|trait|impl|import|module|pub|async|await|extern|unsafe|true|false|as|is|where|with|self|Self)\\b"
    },
    "types": { "name": "entity.name.type.nova", "match": "\\b[A-Z][A-Za-z0-9_]*\\b" }
  }
}
```

`tools/vscode-nova/src/extension.ts`:

```ts
// The Nova extension: the language, its grammar, and a client for
// `nova lsp` (spec docs/superpowers/specs/2026-10-08-phase-3-2-lsp-core-design.md §7).

import * as vscode from "vscode";
import {
  LanguageClient,
  LanguageClientOptions,
  ServerOptions,
} from "vscode-languageclient/node";

let client: LanguageClient | undefined;

export async function activate(context: vscode.ExtensionContext): Promise<void> {
  context.subscriptions.push(
    vscode.commands.registerCommand("nova.restartServer", async () => {
      await stop();
      await start();
    }),
  );
  await start();
}

export async function deactivate(): Promise<void> {
  await stop();
}

/** The `nova` to run: the `nova.server.path` setting, or `nova` from PATH. */
function serverCommand(): string {
  const configured = vscode.workspace
    .getConfiguration("nova")
    .get<string>("server.path", "");
  return configured.trim() !== "" ? configured : "nova";
}

async function start(): Promise<void> {
  const command = serverCommand();
  const serverOptions: ServerOptions = { command, args: ["lsp"] };
  const clientOptions: LanguageClientOptions = {
    documentSelector: [
      { scheme: "file", language: "nova" },
      { scheme: "untitled", language: "nova" },
    ],
  };
  // The client id `nova` makes the `nova.trace.server` setting apply.
  client = new LanguageClient("nova", "Nova", serverOptions, clientOptions);
  try {
    await client.start();
  } catch (error) {
    client = undefined;
    void vscode.window.showErrorMessage(
      `Nova: could not start "${command} lsp" (${String(error)}). ` +
        "Set nova.server.path, or install nova: " +
        "cargo install --locked --git https://github.com/Sakeerin/nova nova-cli",
    );
  }
}

async function stop(): Promise<void> {
  if (client !== undefined) {
    const running = client;
    client = undefined;
    await running.stop();
  }
}
```

- [ ] **Step 4: The smoke test and its fixture**

`tools/vscode-nova/test/fixture/nova.toml`:

```toml
[package]
name = "fixture"
version = "0.1.0"
edition = "2026"
```

`tools/vscode-nova/test/fixture/.editorconfig`:

```
root = true
```

`tools/vscode-nova/test/fixture/src/main.nova`:

```
fn main() {
    let x: Int = "s"
}
```

`tools/vscode-nova/test/fixture/src/complete.nova`:

```
record P { x: Int }

fn f() {
    let p = P { x: 1 }
    p.
}
```

`tools/vscode-nova/test/fixture/src/unformatted.nova`:

```
fn main() {
println("hi")
}
```

The two non-entry files are modules `main.nova` does not import. The
server checks them on their own (decision 9), which the smoke test relies
on.

`tools/vscode-nova/test/run.ts`:

```ts
// Runs the smoke test in a downloaded VS Code, pinned to the extension's
// floor, 1.91.0 (decision 12), against the freshly built debug `nova`.

import * as fs from "fs";
import * as path from "path";
import { runTests } from "@vscode/test-electron";

async function main(): Promise<void> {
  // out/test -> out -> tools/vscode-nova -> tools -> the repository.
  const extensionDevelopmentPath = path.resolve(__dirname, "..", "..");
  const repo = path.resolve(extensionDevelopmentPath, "..", "..");
  const nova = path.join(
    repo,
    "target",
    "debug",
    process.platform === "win32" ? "nova.exe" : "nova",
  );
  if (!fs.existsSync(nova)) {
    throw new Error(`build nova first (cargo build -p nova-cli): ${nova} is missing`);
  }
  const workspace = path.join(extensionDevelopmentPath, "test", "fixture");
  // The fixture's settings point the extension at that `nova`; the file is
  // git-ignored.
  fs.mkdirSync(path.join(workspace, ".vscode"), { recursive: true });
  fs.writeFileSync(
    path.join(workspace, ".vscode", "settings.json"),
    JSON.stringify({ "nova.server.path": nova }, null, 2),
  );
  await runTests({
    version: "1.91.0",
    extensionDevelopmentPath,
    extensionTestsPath: path.resolve(__dirname, "suite", "index"),
    launchArgs: [workspace, "--disable-extensions"],
  });
}

main().catch((error) => {
  console.error(error);
  process.exit(1);
});
```

`tools/vscode-nova/test/suite/index.ts`:

```ts
import * as path from "path";
import Mocha from "mocha";

export function run(): Promise<void> {
  const mocha = new Mocha({ ui: "bdd", timeout: 90000 });
  mocha.addFile(path.resolve(__dirname, "smoke.test.js"));
  return new Promise((resolve, reject) => {
    mocha.run((failures) =>
      failures > 0 ? reject(new Error(`${failures} test(s) failed`)) : resolve(),
    );
  });
}
```

`tools/vscode-nova/test/suite/smoke.test.ts`:

```ts
// The extension's smoke test (spec §7.6): the language, a diagnostic, a
// completion and a formatted document, through the real `nova lsp`.

import * as assert from "assert";
import * as path from "path";
import * as vscode from "vscode";

// out/test/suite -> the extension -> test/fixture.
const fixture = path.resolve(__dirname, "..", "..", "..", "test", "fixture");

function sleep(ms: number): Promise<void> {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

/** Retry `attempt` until it returns a value, for at most 60 s. */
async function eventually<T>(what: string, attempt: () => Promise<T | undefined>): Promise<T> {
  const deadline = Date.now() + 60000;
  for (;;) {
    const value = await attempt();
    if (value !== undefined) {
      return value;
    }
    if (Date.now() > deadline) {
      throw new Error(`timed out waiting for ${what}`);
    }
    await sleep(500);
  }
}

describe("the Nova extension", () => {
  it("gives .nova files the language nova", async () => {
    const doc = await vscode.workspace.openTextDocument(path.join(fixture, "src", "main.nova"));
    assert.strictEqual(doc.languageId, "nova");
  });

  it("shows the planted error's diagnostic", async () => {
    const uri = vscode.Uri.file(path.join(fixture, "src", "main.nova"));
    await vscode.window.showTextDocument(await vscode.workspace.openTextDocument(uri));
    const diagnostics = await eventually("a diagnostic", async () => {
      const found = vscode.languages.getDiagnostics(uri);
      return found.length > 0 ? found : undefined;
    });
    assert.ok(
      diagnostics.some((d) => d.code === "E0010"),
      JSON.stringify(diagnostics),
    );
  });

  it("completes a record field after a dot", async () => {
    const uri = vscode.Uri.file(path.join(fixture, "src", "complete.nova"));
    const doc = await vscode.workspace.openTextDocument(uri);
    const at = doc.positionAt(doc.getText().indexOf("p.") + 2);
    const labels = await eventually("a completion of x", async () => {
      const list = await vscode.commands.executeCommand<vscode.CompletionList>(
        "vscode.executeCompletionItemProvider",
        uri,
        at,
        ".",
      );
      const names = list.items.map((i) => (typeof i.label === "string" ? i.label : i.label.label));
      return names.includes("x") ? names : undefined;
    });
    assert.ok(labels.includes("x"), labels.join(", "));
  });

  it("formats a document", async () => {
    const uri = vscode.Uri.file(path.join(fixture, "src", "unformatted.nova"));
    await vscode.workspace.openTextDocument(uri);
    const edits = await eventually("a formatting edit", async () => {
      const found = await vscode.commands.executeCommand<vscode.TextEdit[]>(
        "vscode.executeFormatDocumentProvider",
        uri,
        { tabSize: 4, insertSpaces: true },
      );
      return found !== undefined && found.length > 0 ? found : undefined;
    });
    assert.strictEqual(edits.length, 1);
    assert.strictEqual(edits[0].newText, 'fn main() { println("hi") }\n');
  });
});
```

- [ ] **Step 5: The Rust tests pass; then ask the user before npm**

```bash
cd /d/Projects/nona/nova && cargo test --locked -p nova-cli --test vscode_extension 2>&1 | grep -E "^test |panicked" | head -4
```

Expected: both pass.

**Stop here and ask the user** (spec §10).
- The next commands download the npm packages of `package.json` into the
  git-ignored `node_modules/`; `package-lock.json` comes only from that
  install.
- The smoke test then downloads VS Code 1.91.0, about 150 MB, into
  `.vscode-test/`.

Ask for both together, naming the sources: `registry.npmjs.org`, and
`update.code.visualstudio.com`. Proceed only with the user's word.

- If the user declines the install, stop the task and say so. Spec §7.5
  requires the committed lockfile, and only an install writes it.
- If the user allows the install but not VS Code's download, run Step 6
  without its `npm test` line, and let CI run the smoke test.

- [ ] **Step 6: Install, compile, test and package**

```bash
cd /d/Projects/nona/nova && P=/c/Users/SAKEER~1/AppData/Local/Temp/gcm/p32 && cargo build --locked -p nova-cli 2>&1 | tail -1 && cd tools/vscode-nova && npm install > $P/t15-npm.txt 2>&1; echo "install exit=$?"; npm run compile > $P/t15-tsc.txt 2>&1; echo "compile exit=$?"; tail -5 $P/t15-tsc.txt; npm test > $P/t15-smoke.txt 2>&1; echo "smoke exit=$?"; tail -15 $P/t15-smoke.txt; npx vsce package --out nova-vscode-0.2.0.vsix > $P/t15-vsce.txt 2>&1; echo "vsce exit=$?"; tail -5 $P/t15-vsce.txt; ls -la nova-vscode-0.2.0.vsix
```

Expected:
- every exit is 0;
- the smoke output ends with `4 passing`;
- `vsce` prints `Packaged: …nova-vscode-0.2.0.vsix`, and the file exists.

Ledger the `.vsix`'s size and `package-lock.json`'s top-level package
count (`grep -c '"node_modules/' package-lock.json`).

`vsce` may warn, for example about a missing `icon` or `bugs` field. Ledger
each warning. Fix one only if it is an error.

- [ ] **Step 7: Commit**

Write `$P/msg-15.txt`:

```
tools/vscode-nova: the VS Code extension

Spec docs/superpowers/specs/2026-10-08-phase-3-2-lsp-core-design.md §7.

The extension:
- registers the language `nova` for .nova files, with a TextMate
  grammar and a language configuration;
- starts `nova lsp` through vscode-languageclient 10, from the
  `nova.server.path` setting or else from PATH, and names the setting
  and the install command if it cannot;
- offers "Nova: Restart Language Server".

Its version is nova-cli's and its floor is VS Code 1.91. There is no
bundler, and package-lock.json is committed.

The smoke test runs VS Code 1.91.0 against the freshly built nova and
checks four things: the language, the planted error's diagnostic, a
record field's completion, and a formatted document. Two Rust tests keep
the grammar's keywords equal to the lexer's, and the extension's version
equal to nova-cli's.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
```

```bash
cd /d/Projects/nona/nova && P=/c/Users/SAKEER~1/AppData/Local/Temp/gcm/p32 && cargo fmt --all && git add .gitignore tools/vscode-nova crates/nova-cli/tests/vscode_extension.rs && git status --short | head -30 && git commit -q -F $P/msg-15.txt && git log -1 --format=%s
```

Expected:
- `git status` lists the extension's sources, fixture, licences and
  `package-lock.json`;
- it lists no `node_modules/`, `out/`, `.vscode-test/`, `.vsix` or
  `test/fixture/.vscode/`;
- the subject is `tools/vscode-nova: the VS Code extension`.

The task's test command: `cargo test --locked -p nova-cli --test vscode_extension`.

---
### Task 16: CI builds and smoke-tests the extension; releases attach the `.vsix`

Spec §8. The `release.yml` changes are proved by its pull-request run,
because this PR touches its paths.

**Files:**
- Modify: `.github/workflows/ci.yml` (a `vscode` job, after `install`)
- Modify: `.github/workflows/release.yml`:
  - the `paths` filter;
  - a `vsix` job;
  - `prepare`'s `needs` and its count;
  - the header comment.

**Interfaces:**
- Consumes: Task 15's `npm` scripts and `vsce` command.
- Produces: the CI job `VS Code extension`, and the `archive-vsix` release
  artifact.

- [ ] **Step 1: The CI job**

Append to `.github/workflows/ci.yml`, as the last job:

```yaml

  # The VS Code extension (spec
  # docs/superpowers/specs/2026-10-08-phase-3-2-lsp-core-design.md §8): it
  # is platform-neutral, and the Rust gate test drives `nova lsp` on all
  # three systems, so one Linux job suffices. The smoke test runs VS Code
  # 1.91.0, the extension's floor, under a virtual display.
  vscode:
    name: VS Code extension
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - uses: Swatinem/rust-cache@v2
      - name: cargo build -p nova-cli
        run: cargo build --locked -p nova-cli
      - uses: actions/setup-node@v4
        with:
          node-version: 22
          cache: npm
          cache-dependency-path: tools/vscode-nova/package-lock.json
      - name: npm ci
        working-directory: tools/vscode-nova
        run: npm ci
      - name: Compile
        working-directory: tools/vscode-nova
        run: npm run compile
      - name: Smoke test
        working-directory: tools/vscode-nova
        run: xvfb-run -a npm test
      - name: Package the .vsix
        working-directory: tools/vscode-nova
        shell: bash
        run: |
          set -euo pipefail
          version=$(node -p "require('./package.json').version")
          npx vsce package --out "nova-vscode-$version.vsix"
      - uses: actions/upload-artifact@v4
        with:
          name: vsix
          path: tools/vscode-nova/*.vsix
          if-no-files-found: error
```

- [ ] **Step 2: The release**

In `.github/workflows/release.yml`:

Add `tools/vscode-nova/**` to the pull-request `paths` list, after
`Cargo.lock`.

Change the header comment's first sentence to: "On a `v*` tag: build the
four targets, pack and smoke-test each archive, package the VS Code
extension, and publish a GitHub release with the archives, the `.vsix` and
SHA256SUMS."

Add a job after `build`, before `prepare`:

```yaml
  # The VS Code extension, packaged for the release (spec
  # docs/superpowers/specs/2026-10-08-phase-3-2-lsp-core-design.md §8). Its
  # artifact is named `archive-vsix` so that `prepare`'s `archive-*`
  # download takes it. CI's `VS Code extension` job smoke-tests it.
  vsix:
    name: Package the VS Code extension
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: actions/setup-node@v4
        with:
          node-version: 22
      - name: Package
        working-directory: tools/vscode-nova
        shell: bash
        run: |
          set -euo pipefail
          npm ci
          npm run compile
          version=$(node -p "require('./package.json').version")
          npx vsce package --out "nova-vscode-$version.vsix"
      - uses: actions/upload-artifact@v4
        with:
          name: archive-vsix
          path: tools/vscode-nova/*.vsix
          if-no-files-found: error
```

In `prepare`, change `needs: build` to `needs: [build, vsix]`, and replace

```bash
          count=$(ls release/dist | wc -l)
          if [ "$count" -ne 4 ]; then
            echo "::error::expected 4 archives, found $count"
            ls release/dist
            exit 1
          fi
```

with

```bash
          # An archive is nova-<version>-<target>; the extension is
          # nova-vscode-<version>.vsix.
          archives=$(ls release/dist | grep -c -E '^nova-[0-9]' || true)
          vsix=$(ls release/dist | grep -c -E '^nova-vscode-.+\.vsix$' || true)
          if [ "$archives" -ne 4 ] || [ "$vsix" -ne 1 ]; then
            echo "::error::expected 4 archives and 1 .vsix, found $archives and $vsix"
            ls release/dist
            exit 1
          fi
```

`SHA256SUMS` already covers every file in `release/dist`, and `publish`
already attaches `release/dist/*`.

- [ ] **Step 3: Check the YAML**

```bash
cd /d/Projects/nona/nova && python -X utf8 -c "import yaml,sys; [yaml.safe_load(open(f, encoding='utf-8')) for f in ('.github/workflows/ci.yml', '.github/workflows/release.yml')]; print('both parse')" 2>&1 | tail -1; git diff --stat -- .github
```

Expected: `both parse`, with two files in the stat.

If PyYAML is missing (`ModuleNotFoundError`), ledger it and rely on the PR's
CI run, which reads both files.

- [ ] **Step 4: Commit**

Write `$P/msg-16.txt`:

```
ci, release: build, smoke-test and attach the VS Code extension

Spec docs/superpowers/specs/2026-10-08-phase-3-2-lsp-core-design.md §8.

CI gains a "VS Code extension" job on ubuntu. It builds nova, runs npm
ci and the compile, runs the smoke test under xvfb in VS Code 1.91.0,
and packages nova-vscode-<version>.vsix as an artifact.

release.yml packages the .vsix as the artifact archive-vsix, so
prepare's download takes it, and prepare now requires 4 archives and 1
.vsix. The checksums and the release attach it like the archives. The
pull-request paths include tools/vscode-nova/**.

No extension store is published to.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
```

```bash
cd /d/Projects/nona/nova && P=/c/Users/SAKEER~1/AppData/Local/Temp/gcm/p32 && git add .github/workflows/ci.yml .github/workflows/release.yml && git commit -q -F $P/msg-16.txt && git log -1 --format=%s
```

Expected: `ci, release: build, smoke-test and attach the VS Code extension`.

The task's test command: the YAML check of Step 3. The jobs themselves run
on the PR.

---
### Task 17: The records

Spec §11. ADR 0029, dated notes in the specs, the project documents, the
latency figures, and the set-difference sweep. The figures come from Task
14 Step 3's ledger lines. Fill each table row from them, as measured; never
round a figure toward the budget.

**Files:**
- Create: `docs/adr/0029-the-language-server.md`
- Modify (dated notes):
  - `nova-spec/40-TOOLING.md` §3.1, §3.2 and §3.3;
  - `nova-spec/11-PARSER.md`;
  - `nova-spec/00-MASTER-SPEC.md`;
  - `docs/phase-3-plan.md`.
- Modify: `CHANGELOG.md`, `ARCHITECTURE.md`, `README.md`,
  `docs/benchmarks/README.md`
- Modify: whatever Step 5's sweep finds

**Interfaces:**
- Consumes: Task 14's figures, and every task's behaviour.
- Produces: nothing code depends on.

- [ ] **Step 1: ADR 0029**

`docs/adr/0029-the-language-server.md`:

```markdown
# ADR 0029: The language server

## Status

Accepted, 2026-10-08 (Phase 3.2, branch `phase-3-2-lsp-core`; spec
`docs/superpowers/specs/2026-10-08-phase-3-2-lsp-core-design.md`).

## Context

`nova-spec/40-TOOLING.md` §3.2 specifies the language server as
`tower-lsp` for the protocol and `salsa` for incremental queries, and ADR
0025 mapped `salsa` to Phase 3. When Phase 3 was planned on 2026-10-06:
- `tower-lsp`'s last release was 0.20.0, from 2023-08-11;
- its fork `tower-lsp-server` needs Rust 1.85, and the workspace's minimum
  is 1.78.

The phase plan therefore recommended `lsp-server` and a re-check engine
(`docs/phase-3-plan.md` §3, decision 7). It left the latency budget to
3.2's spec, and held `salsa` back unless the budget was missed.

The front end was built for finished programs:
- the driver stopped at the first stage that found an error;
- it printed its diagnostics;
- it read every module from disk.

## Decision

1. **The protocol is `lsp-server` 0.7.8, pinned `=0.7.8`, with `lsp-types`
   0.97.**
   - `lsp-server` is rust-analyzer's synchronous framing crate, which suits
     a checker thread.
   - Releases 0.7.9 and later are edition 2024, which needs Rust 1.85, so
     the user chose to keep the 1.78 minimum and pin. A caret requirement
     would let `cargo update` take 0.7.9.
   - Every crate the two bring is edition 2021 or earlier, and declares Rust
     1.71 or less, or nothing.
2. **The engine re-checks the whole program on every change**, on one
   checker thread, behind `nova_driver::analyze`, a seam `salsa` could
   replace. Before 3.2, `nova check` of `05-json-api` and its 6,116 lines
   of std took 108-127 ms in a release build. Std is over 95% of every
   check.
3. **The budget is 200 ms** for both the median and the maximum, of 20
   edits to their diagnostics and of 20 completions, on `05-json-api`, in a
   release build on the development host. CI asserts 2 s with the debug
   binary. Measured:

   | Run | Edit to diagnostics, median / max | Completion, median / max | Binary |
   |---|---|---|---|
   | 1 | from Task 14 | from Task 14 | size, build time |
   | 2 | | | |
   | 3 | | | |

   An edit that arrives during a check waits for it, so a burst of typing
   can take up to about two checks. Nothing cancels a check.
4. **The front end keeps going for the server only.** `analyze` with
   `keep_going` runs every stage whatever the earlier ones found, and
   returns its diagnostics. With `tests`, it checks `@test` bodies, as
   `nova test` does. `nova check`, `build`, `run` and `test` keep their
   staged output: they share `analyze`'s loader, reading through
   `DiskSources`, and print exactly what they printed before.
5. **What is at the cursor comes from recovery plus a probe.**
   - The parser keeps an unfinished `foo.`.
   - The type checker's probe, an offset in one file, records:
     - the receiver of a member access whose receiver ends before the
       offset and whose name ends after it, a name on the next line
       included;
     - its members;
     - the locals in scope.
   - Members follow declared visibility, which the checker does not
     enforce.
6. **Each file's diagnostics have one owning analysis.**
   - A project's analysis owns every module its `src/main.nova` reaches.
   - An open file the entry does not reach is checked as a module, and so
     is a loose file with no `fn main`.
   - An E0001 about an item the parser dropped mid-edit is removed.
7. **The extension runs the installed `nova`**, from `nova.server.path` or
   PATH, as one platform-neutral `.vsix`.
8. **Full text sync, and the client's file watcher**, registered
   dynamically, so no watching crate is added.

## Alternatives

- **`tower-lsp-server`.** It needs Rust 1.85, and its async runtime would
  bring Tokio into a compiler that has none.
- **Protocol types written by hand.** Nova would own a few hundred lines of
  structs, more with each 3.4 capability.
- **`salsa` now.** Re-checking meets the budget, so `salsa`'s cost buys
  nothing yet. If a later sub-phase misses the budget, adopting it is a
  step decided with the user.
- **Raising the MSRV to 1.85** for `lsp-server` 0.10. The user chose the
  pin, which keeps a policy change out of a feature.
- **A placeholder name written at the cursor**, or **a full position
  index**, instead of the probe. The first edits the user's text; the
  second makes every check pay for tables only the server reads.
- **A `nova` bundled in the extension.** It could disagree with the
  user's own `nova`.

## Consequences

- **The server's analyses can meet code the front end never saw before:**
  half-typed programs. `catch_unwind` keeps a panic from killing the
  server, and a test analyses 364 cut programs on every run.
- **The stack's crates are not current.** `lsp-types` has had no release
  since 2024-06, and `lsp-server` is pinned to 2024-12's. LSP 3.17 is
  stable, and both crates are small enough to vendor.
- **3.4 adds hover, definitions, references, rename, code actions and
  semantic tokens on the same engine.** The probe is where hover's "what
  is here" starts.

## References

- `docs/superpowers/specs/2026-10-08-phase-3-2-lsp-core-design.md`
- `docs/superpowers/plans/2026-10-08-phase-3-2-lsp-core.md`
- `docs/phase-3-plan.md` §3, decision 7, and §4's 3.2 entry
- ADR 0025 (`salsa` mapped to Phase 3), ADR 0026 (Phase 3's scope), ADR
  0028 (the formatter's library entry points)
```

Fill the "Measured" table from Task 14 Step 3's three ledgered runs, with
each run's binary size and build time. Then delete the instruction words
in its first row.

- [ ] **Step 2: Notes in the specs**

Each note is inserted as its own paragraph, after the line named, with a
blank line before it.

In `nova-spec/40-TOOLING.md`:
- after `| Call Hierarchy | 4 | |`:

  ```markdown
  **Amended 2026-10-08 (branch `phase-3-2-lsp-core`):** 3.2 delivers
  diagnostics, completion and format on save
  (`docs/superpowers/specs/2026-10-08-phase-3-2-lsp-core-design.md`).
  Diagnostics come from re-checking the whole program, not from `salsa`
  (ADR 0029). Completion offers members after `.`; elsewhere the locals,
  the module's names, the primitive types and the keywords. The other Phase
  3 rows are 3.4's.
  ```
- after `- Workspace-aware: scans `nova.toml` to find roots`:

  ```markdown
  **Amended 2026-10-08 (branch `phase-3-2-lsp-core`):** as built, the stack
  is `lsp-server` 0.7.8, pinned, and `lsp-types` 0.97, not `tower-lsp`.
  Analyses run on one checker thread, re-checking the whole program, not
  through `salsa`. The file watcher is the client's, registered
  dynamically. A file's project is the nearest directory holding
  `nova.toml`, found on demand rather than by a scan. ADR 0029 has the
  reasons and the measured budget.
  ```
- after `- All use the same `nova lsp` backend`:

  ```markdown
  **Amended 2026-10-08 (branch `phase-3-2-lsp-core`):** `tools/vscode-nova/`
  exists. It is TypeScript with `vscode-languageclient` 10, a TextMate
  grammar, and a smoke test in CI, and it runs the installed `nova`. The
  Zed and Neovim extensions are outside Phase 3 (ADR 0026).
  ```

In `nova-spec/11-PARSER.md`, after the line `  already were.` that ends the
3.1 note:

```markdown

**Amended 2026-10-08 (branch `phase-3-2-lsp-core`):** two recoveries for
the language server (`docs/superpowers/specs/2026-10-08-phase-3-2-lsp-core-design.md`
§3.2, §3.4):
- A `.` followed by something other than a name still reports P0001, and
  now keeps its receiver as a field access with an empty name.
- `parse_recovering` also names the top-level items dropped after their
  names were read.

`parse` is unchanged, and only the server's analysis type-checks the empty
name.
```

In `nova-spec/00-MASTER-SPEC.md`:
- after `4. `crates/nova-lsp` — LSP server (use `tower-lsp`)`:

  ```markdown
     **Amended 2026-10-08 (branch `phase-3-2-lsp-core`):** built with
     `lsp-server` 0.7.8 and `lsp-types` 0.97 instead (ADR 0029).
  ```
- after `  positions 4 and 7.`:

  ```markdown

  **Amended 2026-10-08 (branch `phase-3-2-lsp-core`):** the language server
  uses `lsp-server` 0.7.8, pinned, and `lsp-types` 0.97, not `tower-lsp`
  0.20, whose last release was 2023's (ADR 0029). The dependency list's
  `tower-lsp` line above is that plan.
  ```

In `docs/phase-3-plan.md`:
- after the decision 7 paragraph that ends
  `edit to its diagnostics on the largest example.`:

  ```markdown

     **Amended 2026-10-08 (branch `phase-3-2-lsp-core`):** `lsp-server`
     0.10.0 declares no minimum Rust, but it is edition 2024, which needs
     Rust 1.85, and so is every release from 0.7.9 on. 3.2 pins 0.7.8 to
     keep the 1.78 minimum (ADR 0029). The budget is 200 ms.
  ```
- after the 3.2 gate bullet that ends `becomes its own step, decided with the user.`:

  ```markdown
  - **Spec:** `docs/superpowers/specs/2026-10-08-phase-3-2-lsp-core-design.md`,
    built on the branch `phase-3-2-lsp-core`.
  ```

- [ ] **Step 3: The project documents**

`CHANGELOG.md`, `[Unreleased]`, at the end of `### Added`, after
`- `nova_lexer::lex_with_comments` returns the tokens and every comment.`:

```markdown
- **`nova lsp`**, a language server over stdio.
  - **Diagnostics.** Whole projects are re-checked on every edit and on
    every watched change on disk, including unsaved buffers.
  - **Completion.** After `.`, fields and methods (std's included).
    Elsewhere, the locals, the names in scope, the primitive types and the
    keywords. It works in a file that has syntax errors.
  - **Formatting.** It formats through `nova-fmt`, with `.editorconfig`.

  Edits and completions take at most 200 ms on `examples/05-json-api`
  (ADR 0029).
- **`tools/vscode-nova/`**, the VS Code extension: the language, a
  grammar, and a client for the installed `nova lsp`. Each release attaches
  its `.vsix`.
- **`nova_driver::analyze`** runs the front end over editor buffers and
  past errors, for the language server.
- **`nova-fmt`'s `format_buffer`** formats an editor's text as
  `format_file` would format the file.
```

`CHANGELOG.md`, `[Unreleased]`, at the end of `### Changed`, after
`  they stay so.`:

```markdown
- The parser keeps an unfinished `foo.` as a field access with an empty
  name, after the same P0001. Parsing continues past it, so a few broken
  programs report fewer follow-on errors.
```

`ARCHITECTURE.md`: replace the row
`| `nova-lsp` | Language Server Protocol implementation |` with:

```markdown
| `nova-lsp` | The language server, `nova lsp`: diagnostics, completion and formatting over `nova_driver::analyze` (ADR 0029) |
```

`README.md`: after the paragraph that ends
`ready-built archives for Linux, macOS and Windows.`, add:

```markdown

### Editor support

`nova lsp` is a language server: it gives an editor diagnostics,
completion and formatting. For VS Code, `tools/vscode-nova/` is the
extension. Each GitHub release attaches it as `nova-vscode-<version>.vsix`,
which installs with **Extensions: Install from VSIX…**. It runs the `nova`
on your PATH, or the one the `nova.server.path` setting names.
```

`docs/benchmarks/README.md`: append:

```markdown

## The language server's latency (Phase 3.2)

The budget is 200 ms for the median and the maximum of 20 edits, each sent
while no check runs, to their diagnostics, and of 20 completions after
`self.users.` (ADR 0029). It is measured on
`examples/05-json-api/src/main.nova` with
`cargo test --release -p nova-cli --test lsp -- --ignored --nocapture latency`.

| Date | Run | Edit to diagnostics, median / max | Completion, median / max | Binary |
|---|---|---|---|---|
| 2026-10-08 | 1 | from Task 14 | from Task 14 | size, build time |
| 2026-10-08 | 2 | | | |
| 2026-10-08 | 3 | | | |

Each figure includes the round trip through the protocol on one machine.
CI asserts 2 s for each, with the debug binary, on all three systems.
```

Fill this table from the same three runs as the ADR's.

- [ ] **Step 4: Commit the records**

Write `$P/msg-17a.txt`:

```
docs: record Phase 3.2, the language server

- ADR 0029, "The language server": lsp-server 0.7.8 pinned and lsp-types
  0.97, not tower-lsp; re-checking, not salsa, with the 200 ms budget and
  its measured figures; keep-going for the server only; the probe;
  ownership; the installed nova; full sync and the client's watcher.
- Dated notes: 40-TOOLING §3.1, §3.2 and §3.3, 11-PARSER, the master
  spec's tower-lsp lines, and the phase plan's decision 7 and 3.2 spec
  line.
- CHANGELOG [Unreleased], ARCHITECTURE's nova-lsp row, the README's
  editor support, and the latency figures in docs/benchmarks/README.md.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
```

```bash
cd /d/Projects/nona/nova && P=/c/Users/SAKEER~1/AppData/Local/Temp/gcm/p32 && git add docs/adr/0029-the-language-server.md nova-spec CHANGELOG.md ARCHITECTURE.md README.md docs/phase-3-plan.md docs/benchmarks/README.md && git commit -q -F $P/msg-17a.txt && git log -1 --format=%s && git show --stat HEAD | tail -1
```

Expected: `docs: record Phase 3.2, the language server`, with `10 files changed`.

- [ ] **Step 5: The set-difference sweep**

A grep proves only what it was pointed at. List every living document that
names what this branch changed, minus the files the branch touched:

```bash
cd /d/Projects/nona/nova && P=/c/Users/SAKEER~1/AppData/Local/Temp/gcm/p32 && git diff --name-only d045d05 HEAD > $P/touched.txt && for t in "nova lsp" "nova-lsp" "tower-lsp" "salsa" "lsp-server" "lsp-types" "vscode" "VS Code" "VSCode" "language server" "completion" ".vsix" "keep_going"; do git grep -l -F -- "$t" -- '*.md' '*.nova' '*.yml' '*.toml' '*.sh' '*.json' | grep -v -x -F -f $P/touched.txt | grep -v -E "^docs/superpowers/|^docs/adr/|^tools/vscode-nova/" | sed "s|^|$t: |"; done | sort | tee $P/sweep.txt | wc -l
```

Read every hit in `$P/sweep.txt`, in context. For each, decide whether it
now says something untrue or incomplete about the language server, the
extension, the front end's error handling, or the release's contents.
- If it does, add a dated note there, or fix a non-document such as a
  script, and add the file to the commit below.
- If not, nothing changes.

Ledger each file as `Task 17: sweep <file> -> <note added | unaffected:
why>`. A file listed for a word it uses in another sense is unaffected:
Rust's `rust-analyzer`, a "completion" that means finishing, the
`.vscode/` of an unrelated tool.

If the sweep changed anything, write `$P/msg-17b.txt`:

```
docs: notes the language-server sweep found

The set-difference sweep over the living documents (spec
docs/superpowers/specs/2026-10-08-phase-3-2-lsp-core-design.md §11):
<one line per file changed, saying what changed>

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
```

and commit the files it lists with `git commit -q -F $P/msg-17b.txt`, then
check `git log -1 --format=%s`.

The task's test command:
`cargo test --locked -p nova-cli --test vscode_extension`. The records
touch no code; this confirms that the extension's tracked files still
parse.

---
### Task 18: Final verification

Before the PR, run here everything CI runs, on the finished branch, plus
the gate's items by name.

**Files:**
- Create: `$P/pr-body.md` (outside the repository)

**Interfaces:**
- Consumes: the whole branch.
- Produces: the figures for the PR body.

- [ ] **Step 1: The full suite on Windows**

```bash
cd /d/Projects/nona/nova && P=/c/Users/SAKEER~1/AppData/Local/Temp/gcm/p32 && git status --short | wc -l && netstat -ano | grep -E "[:.]3000 .*LISTENING"; cargo build --locked -p nova-runtime 2>&1 | tail -1 && cargo test --locked --workspace --all-features --no-fail-fast > $P/suite-final.txt 2>&1; echo "exit=$?"; python -X utf8 $P/count.py $P/suite-final.txt
```

Expected:
- `0` uncommitted files;
- nothing from `netstat`. If it prints anything, stop and ask the user, as
  the Conventions say;
- `exit=0`, 0 failed;
- passed: Task 1's baseline plus the new tests, 67 on Windows;
- ignored: the baseline's plus 1, the latency test.

Recount the new tests by name from the ledger if the figure differs, and
ledger the difference.

- [ ] **Step 2: What CI's other jobs run**

```bash
cd /d/Projects/nona/nova && P=/c/Users/SAKEER~1/AppData/Local/Temp/gcm/p32 && cargo fmt --all -- --check; echo "fmt exit=$?"; cargo clippy --locked --all-targets --all-features -- -D warnings > $P/clippy.txt 2>&1; echo "clippy exit=$?"; tail -3 $P/clippy.txt
```

Expected: `fmt exit=0` and `clippy exit=0`.
- A clippy finding is fixed in the code, never allowed by attribute.
- The fix is its own commit, `<crate>: what clippy found`.
- Step 1 then runs again.

```bash
cd /d/Projects/nona/nova && P=/c/Users/SAKEER~1/AppData/Local/Temp/gcm/p32 && RUSTUP_TOOLCHAIN=1.78.0 RUSTFLAGS="-D warnings" cargo check --locked --workspace --target-dir target/msrv > $P/msrv.txt 2>&1; echo "msrv exit=$?"; tail -3 $P/msrv.txt; grep -E "Checking (lsp-server|lsp-types|nova-lsp)" $P/msrv.txt
```

Expected: `msrv exit=0`, and the check lists `lsp-server v0.7.8`,
`lsp-types` and `nova-lsp`. This is the pin's proof.
- **If 1.78 rejects a crate, stop.** Name the crate, its edition and its
  `rust_version`, and ask the user: the pin's premise is wrong.

- [ ] **Step 3: The full suite on Linux**

```bash
cd /d/Projects/nona/nova && P=/c/Users/SAKEER~1/AppData/Local/Temp/gcm/p32 && docker version 2>&1 | grep -c "^Server:"; bash /c/Users/SAKEER~1/AppData/Local/Temp/gcm/linux/run.sh test --locked --workspace --all-features --no-fail-fast > $P/suite-linux-final.txt 2>&1; echo "exit=$?"; python -X utf8 $P/count.py $P/suite-linux-final.txt; bash /c/Users/SAKEER~1/AppData/Local/Temp/gcm/linux/run.sh run --locked -q -p nova-cli -- fmt --check std examples; echo "exit=$?"
```

Expected:
- `1` (Docker's server is up), and both exits `0`;
- 0 failed;
- passed: 1425 plus 66 (the Windows-only URI test is not compiled on Linux);
- 10 ignored.

- [ ] **Step 4: The gate, item by item**

`docs/phase-3-plan.md` §4's 3.2 gate, each with the test that shows it:

```bash
cd /d/Projects/nona/nova && P=/c/Users/SAKEER~1/AppData/Local/Temp/gcm/p32 && grep -E "a_planted_error_gets_its_diagnostic_with_an_exact_utf16_range|completion_works_in_a_file_with_a_syntax_error|the_document_is_formatted|edits_and_completions_stay_within_the_ci_bound" $P/suite-final.txt
```

Expected: four lines ending in `ok`. Ledger them:
- **a diagnostic for a planted error:**
  `a_planted_error_gets_its_diagnostic_with_an_exact_utf16_range`;
- **completions in a file with a syntax error,** holding a std method and a
  record field: `completion_works_in_a_file_with_a_syntax_error`;
- **a formatted document:** `the_document_is_formatted`;
- **the latency:**
  - CI's bound: `edits_and_completions_stay_within_the_ci_bound`;
  - the development host's budget: Task 14 Step 3's figures, in ADR 0029;
- **the extension's smoke test:** `4 passing` locally if the user allowed
  it (Task 15 Step 6), and CI's `VS Code extension` job.

- [ ] **Step 5: The mutants**

Spec §9.4: each mutant must fail at least one named test. Run each one
only on committed work.
1. Make the edit.
2. Run its test command and read the named test's line: it must say
   `FAILED`. A mutant that aborts the build reads as nothing failed, so
   check the name.
3. Restore the file with `git checkout -- <file>`, and confirm
   `git status --short` is empty before the next one.

| # | File | Edit | Test that must fail |
|---|---|---|---|
| 1 | `crates/nova-diagnostics/src/line_index.rs` | in `position`, `c.len_utf16()` → `c.len_utf8()` | `cargo test --locked -p nova-diagnostics --test line_index` → `thai_counts_one_unit_per_character` |
| 2 | `crates/nova-driver/src/analyze.rs` | `!options.keep_going && has_error(diags)` → `has_error(diags)` | `cargo test --locked -p nova-driver --test analyze` → `keep_going_reports_a_syntax_error_and_a_type_error_together` |
| 3 | `crates/nova-typeck/src/check.rs` | in `check_with`, `probe: options.probe,` → `probe: None,` | `cargo test --locked -p nova-typeck --lib` → `the_probe_records_the_receiver_after_a_dot` |
| 4 | `crates/nova-lsp/src/checker.rs` | delete `if current != Some(job.generation) { continue; }` | `cargo test --locked -p nova-cli --test lsp` → `a_burst_of_edits_ends_with_the_last_edits_diagnostics` |
| 5 | `crates/nova-fmt/src/file.rs` | in `format_buffer`, the `let ending = match settings.crlf { … };` → `let ending = LineEnding::of(text);` | `cargo test --locked -p nova-fmt --test files` → `format_buffer_applies_the_editorconfig_that_governs_its_path` |
| 6 | `crates/nova-typeck/src/check.rs` | in `probe_members`, delete `.filter(|d| !self.selfless.contains(d))` | `cargo test --locked -p nova-typeck --lib` → `members_of_a_std_vec_hide_its_internals_and_its_associated_functions` |
| 7 | `crates/nova-driver/src/analyze.rs` | in `load_program`, `sources.read(&path)` → `std::fs::read_to_string(&path)` | `cargo test --locked -p nova-driver --test analyze` → `a_buffer_beats_the_file_on_disk` |
| 8 | `crates/nova-driver/src/analyze.rs` | in `analyze`, `if !options.tests {` → `if true {` | `cargo test --locked -p nova-driver --test analyze` → `with_tests_a_test_body_is_checked` |

Ledger each as `Task 18: mutant <n> -> <test> FAILED (restored)`.

- [ ] **Step 6: Draft the PR body**

Write `$P/pr-body.md` with the Write tool, filling the `<…>` from the
ledger and from Steps 1 to 3:

```markdown
## Phase 3.2, "LSP core and the VSCode extension"

`nova lsp` gives an editor diagnostics, completion and formatting on code
being typed, and `tools/vscode-nova/` connects VS Code to it.

- Spec: `docs/superpowers/specs/2026-10-08-phase-3-2-lsp-core-design.md`
- Plan: `docs/superpowers/plans/2026-10-08-phase-3-2-lsp-core.md`
- ADR 0029, "The language server"

### What changed

- **The front end, for the server only.**
  - `nova_driver::analyze` reads editor buffers through `Sources`.
  - With `keep_going` it runs every stage past errors, and returns its
    diagnostics.
  - `nova check`, `build`, `run` and `test` are unchanged. They share its
    loader through `DiskSources`.
- **What is at the cursor.**
  - The parser keeps an unfinished `foo.`.
  - The type checker's probe records the receiver, its members and the
    locals at the cursor.
  - The resolver lists a module's names.
- **`nova lsp`.**
  - `lsp-server` 0.7.8 (pinned: 0.7.9 and later need Rust 1.85) and
    `lsp-types` 0.97.
  - Diagnostics per project, published by their owning analysis.
  - Completion and formatting.
  - Watched files, with stale results never published.
- **`tools/vscode-nova/`.** The language, a grammar and a client for the
  installed `nova`.
  - CI builds and smoke-tests it in VS Code 1.91.0.
  - Releases attach the `.vsix`.
- **Records.** ADR 0029, and dated notes in 40-TOOLING, 11-PARSER, the
  master spec and the phase plan.

### The gate (`docs/phase-3-plan.md` §4, 3.2)

- A Rust test drives `nova lsp` over stdio:
  - `a_planted_error_gets_its_diagnostic_with_an_exact_utf16_range`;
  - `completion_works_in_a_file_with_a_syntax_error`, with `push` and a
    record field;
  - `the_document_is_formatted`.
- The extension's smoke test: CI's `VS Code extension` job (<local result>).
- The latency budget is 200 ms on `05-json-api`, release build, this
  machine. Edit to diagnostics is <median / max>; completion is
  <median / max>. CI asserts 2 s
  (`edits_and_completions_stay_within_the_ci_bound`).

### Tests

| | Windows (local) | Linux (container) |
|---|---|---|
| `d045d05` | 1428 passed, 8 ignored | 1425 passed, 9 ignored |
| this branch | <Step 1's figures> | <Step 3's figures> |

Each mutant of spec §9.4 fails its named test. Clippy, rustfmt and the MSRV
check (Rust 1.78, `lsp-server` 0.7.8 included) pass.

### Decisions to review

The plan's "Decisions: where this plan settles what the spec leaves open",
items 1 to 18, and the rulings made while executing it:

<every `Ruling:` line from the ledger, in order>

### Deferred minors

<every `minor (deferred)` line from the ledger, or "None.">

🤖 Generated with [Claude Code](https://claude.com/claude-code)
```

The task's test command:
`cargo test --locked --workspace --all-features --no-fail-fast`.

---

## After the last task

These follow the standing workflow, not this plan:

1. **The final review.** A fresh reviewer, on the most capable model, reads
   the whole branch, from `git merge-base main HEAD` to `HEAD`. It gets the
   spec, this plan, its Review Focus section verbatim, and the ledger's
   `Ruling:` lines. It is read-only, and runs no cargo and no scripts.
2. **The fix pass.** Each Critical or Important finding gets a test that
   fails first, then the fix, then a green suite. Minor findings are
   deferred to the PR body.
3. **Push and open the PR** against `main`, with `$P/pr-body.md` as its
   body. Then read the PR's CI once when the user returns; never poll it.
4. **Merge only on the user's word,** by rebase, and verify that `main`'s
   tree is the branch's tree. Nothing is published, and no tag is pushed.
