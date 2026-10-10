# Phase 3.4b, "Fixes and colour", Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Diagnostics carry fixes, which `nova check` prints as `help:`
lines and `nova lsp` offers as quick fixes. The server also organizes
imports and colours every name by what it means.

**Architecture:**
- **`nova-diagnostics`** gains `Fix` and `Edit` on `Diagnostic`, the
  renderers' `help:` lines, `lines` (placing an edit by lines) and
  `suggest` ("did you mean").
- **`nova-resolver`** keeps what a fix needs: each module's exports, what
  it could import, its package and its text. It makes the "make public"
  and attribute fixes.
- **`nova-typeck`** makes the other fixes where it raises their errors, in
  a child module, `src/check/fixes.rs`. It reads source text through
  `CheckOptions::sources`.
- **`nova-driver`** computes each loaded module's importable table and
  hands both the text and the `FileDb` through.
- **`nova-fmt`** keeps blank-line-separated import groups, and gains
  `organize`, which builds the organized block.
- **`nova-lsp`** gains `code_action.rs`, `organize.rs` and `tokens.rs`.
- **The VS Code extension** declares the `mutable` modifier, and its smoke
  test asks for tokens and a quick fix.

**Tech Stack:**
- Rust: MSRV 1.78, edition 2021.
- No new crate. `lsp-server` 0.7.8 and `lsp-types` 0.97, already locked.
- Tests:
  - `nova-driver` integration tests over in-memory buffers and temporary
    projects;
  - `nova lsp` driven over stdio, through `crates/nova-cli/tests/lsp_client`;
  - the extension's `vscode-test` smoke test in CI.

**Spec:** `docs/superpowers/specs/2026-10-10-phase-3-4b-fixes-and-colour-design.md`,
approved by the user on 2026-10-10.
- Read it before Task 1: it is the authority this plan argues from.
- Its §13 lists 40 decisions, and its §2 the code as it stood.

## Global Constraints

- **The workspace's minimum Rust stays 1.78,** edition 2021. CI's MSRV job
  runs `cargo check --locked --workspace` with `RUSTFLAGS=-D warnings`.
  No `Option::is_none_or` (1.82): write `map_or(true, …)`.
- **No new crate, and `Cargo.lock` does not change.**
- **The published LSP diagnostics change in one way only:** E0060's note
  leaves the message where its fix replaces it (spec §3.3).
- **A fix never edits std, another package, or nova's caches** (spec
  §3.1), and its edits never overlap.
- **Fixes are computed only for errors that occur.** The resolver's and
  driver's new tables are built on every analysis. The index's new
  `locals` table is recorded only with the index on.
- **Code actions do not re-analyse per offered fix** (spec decision 13).
- **Budgets** (spec §9.8): 200 ms for code actions and semantic tokens;
  CI 2 s. If either misses, stop and ask the user.
- **Tests never touch the internet,** and every test that needs
  `$NOVA_HOME` sets it to a fresh directory.
- **Nothing is published, and no tag is pushed.** Merge by rebase, on the
  user's word only.
- **Downloading VS Code** to run the extension's test locally waits for the
  user's word. CI's run is otherwise the first (spec §9.9).

## Review Focus

The five inputs most likely to bite a user that the spec's tests leave
uncovered, most likely first. Each has its test in the task that owns the
code:

1. **An open buffer with unsaved edits above the error.** A quick fix
   must edit the buffer's text at the buffer's offsets →
   `a_quick_fix_edits_an_unsaved_buffer_at_its_own_offsets` (Task 10).
2. **Thai and an emoji before the name.** A quick fix's range and a
   token's column and length must be in UTF-16 →
   `a_quick_fixs_range_counts_utf16_after_thai_and_an_emoji` (Task 10) and
   `tokens_count_utf16_after_thai_and_an_emoji` (Task 12).
3. **Two errors with the same fix,** as `x = 1` then `x = 2` on one
   immutable `x`: the editor must offer "Make `x` mutable" once →
   `two_errors_with_one_fix_offer_it_once` (Task 10).
4. **A Thai name misspelled.** "Did you mean" must count characters, not
   bytes → `closest_counts_characters_not_bytes` (Task 2).
5. **A file whose every import is unused.** Organize imports must remove
   them all and the blank line after, not leave a blank first line →
   `organize_removes_every_import_and_the_blank_line_after` (Task 9).

## Decisions: where this plan settles what the spec leaves open

1. **The interfaces.**
   - **`nova-diagnostics`:** `Fix { title, edits }`, `Edit { span, text }`,
     `Edit::{insert, replace}`, `Fix::{new, apply}`,
     `Diagnostic::{fixes, with_fix}`; `lines::{ending, line_start,
     next_line_start, is_blank, is_comment_line, line_after,
     line_before_item, keyword_start, arm_removal}`;
     `suggest::{distance, closest, change_to}`.
   - **`nova-resolver`:** `ModuleSource::{text, package, importable}`;
     `Exported { value, ty, trait_def }`; `Definitions::{importable,
     exported, same_package, bound_outside_std, is_std_module}`;
     `LocalFlags { parameter, mutable }`, `Index::locals`; `index::rank`,
     now public.
   - **`nova-typeck`:** `CheckOptions::sources`.
   - **`nova-fmt`:** `organize::{organize, ImportView, Group, Verdict,
     TextEdit}`, re-exported at the crate root.
   - **`nova-lsp`:** `code_action::code_actions`, `organize::action`,
     `tokens::{legend, tokens}`.
2. **`nova-diagnostics` holds `lines` and `suggest`.** The resolver (make
   public, attributes) and the checker (the rest) both use them, and
   `nova-diagnostics` is the one crate both depend on that owns `Span` and
   `FileDb`.
3. **The checker's fixes live in `crates/nova-typeck/src/check/fixes.rs`,**
   a child module of `check.rs`, which can reach the `Checker`'s private
   state. `check.rs` is already 17,000 lines.
4. **Text reaches the resolver through `ModuleSource::text` and the
   checker through `CheckOptions::sources`.** Without it, as in
   `resolver::resolve` and both crates' unit tests, the three fixes that
   §4.7 places are not offered; the other fixes are.
5. **"Make mutable" uses a checker-only table, `Checker::bindings`,** keyed
   by the local's declaration span. `bind_local` takes the binding kind.
   `self` is excluded by name.
6. **The kinds of spec §4.2's table are made exact here:**
   - a call needs a function whose parameter count, or a variant whose
     field count, equals the call's arguments;
   - a value needs a function, a constant, or a variant without fields;
   - a qualifier `T::m` needs a type with variant `m` (no fields, or the
     call's argument count); a call's qualifier may also name an inherent
     associated function `m` taking that many;
   - a pattern's qualifier needs a sum type with that variant and arity,
     and the scrutinee's own type when it is known.

   The import fix does not check parameter types (spec decision 37).
7. **A call's qualifier is marked through `Checker::qualified_callee`.**
   `check_call` sets it, to its argument count, just before checking a
   two-segment callee path, and `check_path` takes it at entry. A path
   checked anywhere else sees `None`.
8. **`Definitions::bound_outside_std` tells std's names apart by their
   exports.** A scope entry equal to some std module's export of that name
   came through std's glob. Builtins are bound outside std.
9. **The driver builds `importable` with the loader's own `find`,** over
   each loaded sibling's stem and each visible dependency edge's import
   name, keeping the names that reach a loaded module. That is 3.3a §4.3
   by construction, E0004 clashes included.
10. **Organize imports formats its block with the formatter itself,**
    `format_named`, so its output is what format on save would print. It
    offers nothing when an import shares a line with other code, when
    there is an `import … as`, or when the text does not parse.
11. **An import is a dependency when the module it names is in another
    directory** (spec decision 26). The package's own library seen from
    `tests/`, and every dependency, are in another directory; siblings
    share one.
12. **The cut-program sweep re-analyses the first three fixes of each
    program** (spec decision 40), and checks every fix's edits.
13. **Remove-arm offers no fix when a comment sits between an arm's body
    and its comma.** `lines::arm_removal` returns `None` for anything but a
    comma, a line end, the text's end or `}` after the body.
14. **A code action's `diagnostics` entry** carries the published range,
    severity, code, source and message, without related information.
15. **The shared stdio helpers move into `lsp_client`:** `MANIFEST`,
    `APP_MAIN`, `INDEX`, `project`, `app_and_library`, `registry_app` and
    `lock_and_cache`, with their forms unchanged, and a new
    `app_with_main`, which takes the main file's text (the spec's
    "`app_and_library` takes the main file's text", without changing the
    callers 3.4a wrote). `lsp.rs` and `lsp_navigation.rs` drop their
    copies.
16. **`suggest::change_to` builds the "change `x` to `y`" fix,** shared by
    the resolver and the checker.
17. **The test file for fixes is `crates/nova-driver/tests/fixes.rs`,** one
    `#[test]` per spec §9.1 case, named after it.

## File Structure

| File | Task | Responsibility |
|---|---|---|
| `crates/nova-diagnostics/src/{lib,render}.rs` | 1 | `Fix`, `Edit`, `help:` lines |
| `crates/nova-diagnostics/src/{lines,suggest}.rs` (new) | 2 | Line rules; "did you mean" |
| `crates/nova-resolver/src/lib.rs`, `crates/nova-driver/src/{program,analyze,lib}.rs`, `crates/nova-driver/tests/fixes.rs` (new) | 3 | The tables a fix needs |
| `crates/nova-typeck/src/{lib,check}.rs`, `src/check/fixes.rs` (new), `crates/nova-resolver/src/index.rs`, `crates/nova-driver/src/{analyze,lib}.rs`, `crates/nova-driver/tests/{fixes,index}.rs` | 4 | Sources in the checker; make mutable; the index's locals |
| `crates/nova-typeck/src/check{,/fixes}.rs`, `crates/nova-driver/tests/fixes.rs` | 5 | Import a name; did you mean |
| `crates/nova-resolver/src/lib.rs`, `crates/nova-typeck/src/check{,/fixes}.rs`, `crates/nova-driver/tests/fixes.rs` | 6 | Make public; attributes; remove an arm |
| `crates/nova-driver/tests/broken.rs`, `crates/nova-cli/tests/run_tests.rs` | 7 | The sweep; the command line's help lines |
| `crates/nova-fmt/src/print/mod.rs`, `tests/layout.rs` | 8 | The formatter's import groups |
| `crates/nova-fmt/src/{lib,organize}.rs`, `tests/organize.rs` (new) | 9 | Organize imports' block |
| `crates/nova-lsp/src/{lib,code_action,convert,rename}.rs`, `crates/nova-cli/tests/{lsp_fixes,lsp,lsp_navigation}.rs`, `lsp_client/mod.rs` | 10 | Quick fixes |
| `crates/nova-lsp/src/{organize,code_action,lib}.rs`, `crates/nova-cli/tests/lsp_fixes.rs` | 11 | Organize imports in the server |
| `crates/nova-lsp/src/{tokens,lib}.rs`, `crates/nova-resolver/src/index.rs`, `crates/nova-cli/tests/lsp_fixes.rs` | 12 | Semantic tokens |
| `crates/nova-cli/tests/{lsp_fixes,lsp}.rs` | 13 | The gate test; latency |
| `tools/vscode-nova/{package.json,README.md,test/…}` | 14 | The extension |
| `docs/adr/0033-fixes-and-colour-in-the-language-server.md` (new), and the records Task 15 lists | 15 | ADR, notes, CHANGELOG, README, ARCHITECTURE, sweep |
| `$P/pr-body.md`, outside the repository | 16 | Final verification, mutants, the PR body |

## Conventions for every task

- **Working directory.** `D:\Projects\nona\nova`, which is Git Bash
  `/d/Projects/nona/nova`. The Bash tool resets its directory after each
  call, so write `cd /d/Projects/nona/nova && …` in one command.
- **Scratch directory.** `P=/c/Users/SAKEER~1/AppData/Local/Temp/gcm/p36`,
  outside the repository. Copy the helpers `count.py`, `extract.py`,
  `insert_after.py`, `replace_once.py`, `append.py` and `append_in_mod.py`
  from `…/p35` before Task 1. Long output goes to a file there; read its
  tail.
- **Extract a task's code from its brief** with `extract.py BRIEF MARKER
  OUT`. It writes the first fenced block after the line holding MARKER.
  Never retype plan code.
- **Placing code:** `insert_after.py TARGET ANCHOR_FILE BLOCK_FILE`,
  `replace_once.py TARGET OLD_FILE NEW_FILE`, `append.py TARGET BLOCK_FILE`
  and `append_in_mod.py TARGET BLOCK_FILE` (before a trailing
  `mod tests`'s closing brace). Each keeps the target's line endings.
- **Counting a full run:** `python -X utf8 $P/count.py <FILE>`. It prints
  `N result lines: P passed, F failed, I ignored`.
- **Line endings.** The working tree is mostly CRLF (`core.autocrlf=true`),
  and the index LF. New files may be LF. Test strings that need CRLF
  write `\r\n` explicitly.
- **Write scripts and commit messages with the Write tool, never a Bash
  heredoc.** The Bash tool turns `\\` into `\`, and this plan's Rust has
  backslashes.
- **Commit messages go to a new file each time:** `$P/msg-34b-<task>.txt`.
  The Write tool refuses to overwrite a file not read in this session, and
  a commit in the same batch would then take the old message. Write the
  message in one call, commit in the next, and check `git log -1
  --format=%s`.
- **Format before every commit:** run `cargo fmt --all`, then stage what it
  changed.
- **Chain a commit and what follows it with `&&`, never `;`.**
- **Port 3000 must be free for a full Windows suite.**
  - Check with `netstat -ano | grep -E "[:.]3000 .*LISTENING"`.
  - **Never stop a process that holds it: stop and ask the user.**
- **Linux runs** use `bash /c/Users/SAKEER~1/AppData/Local/Temp/gcm/linux/run.sh <cargo args>`.
  It exports tracked files, so `git add` new files first. Docker must be
  running; if it is not, the Linux run is CI's, with a ruling. Do not
  start Docker.
- **Mutants run only on committed work,** and are undone with `git checkout
  -- <file>`.
- **Expected outputs are exact.** If a step's output differs:
  - when the code does what this plan describes and the expected value is
    wrong, correct the value and ledger a ruling;
  - when the code does something else, fix the code.
- **A test written after its code, on purpose,** is marked *guard* in its
  comment, and is expected to pass on its first run.
- **Anchors.** This plan places each insertion by the code around it, as
  it stood at `1c7bae5`. `cargo fmt` may reflow an earlier task's code; if
  an anchor differs, follow the code's meaning and ledger the difference.

---

### Task 1: `Fix` and `Edit` on `Diagnostic`, and the `help:` lines

**Files:**
- Modify: `crates/nova-diagnostics/src/lib.rs` (after `Label`; `Diagnostic`
  and its builders; a new `mod tests` at the end)
- Modify: `crates/nova-diagnostics/src/render.rs` (both `.with_notes`
  calls)

**Interfaces:**
- Consumes: `Span`, `FileId`, `FileDb` (this crate).
- Produces:
  - `pub struct Fix { pub title: String, pub edits: Vec<Edit> }` and
    `pub struct Edit { pub span: Span, pub text: String }`, both
    `Debug + Clone + PartialEq + Eq`;
  - `Edit::insert(at: u32, file: FileId, text: impl Into<String>) -> Edit`,
    `Edit::replace(span: Span, text: impl Into<String>) -> Edit`;
  - `Fix::new(title: impl Into<String>, edits: Vec<Edit>) -> Fix`,
    `Fix::apply(&self, db: &FileDb) -> Option<Vec<(FileId, String)>>`;
  - `Diagnostic::fixes: Vec<Fix>`, `Diagnostic::with_fix(self, fix: Fix) -> Self`.

- [ ] **Step 1: Write the failing tests**

Append to the end of `crates/nova-diagnostics/src/lib.rs`, with
`append.py`, from this block (marker `T1-TESTS`):

```rust
#[cfg(test)]
mod tests {
    use super::*;

    fn two_files() -> (FileDb, FileId, FileId) {
        let mut db = FileDb::new();
        let a = db.add("a.nova", "let x = 1\n");
        let b = db.add("b.nova", "fn f() {}\n");
        (db, a, b)
    }

    #[test]
    fn a_fix_rides_on_its_diagnostic() {
        let (_, a, _) = two_files();
        let fix = Fix::new("make `x` mutable", vec![Edit::insert(4, a, "mut ")]);
        let d = Diagnostic::error("E0060", "cannot assign").with_fix(fix.clone());
        assert_eq!(d.fixes, vec![fix]);
        assert!(Diagnostic::warning("E0021", "unreachable").fixes.is_empty());
    }

    #[test]
    fn a_fix_applies_to_each_file_it_edits() {
        let (db, a, b) = two_files();
        let fix = Fix::new(
            "two files",
            vec![
                Edit::replace(Span::new(3, 4, b), "g"),
                Edit::insert(4, a, "mut "),
                Edit::insert(0, b, "pub "),
            ],
        );
        assert_eq!(
            fix.apply(&db),
            Some(vec![
                (b, "pub fn g() {}\n".to_string()),
                (a, "let mut x = 1\n".to_string()),
            ])
        );
    }

    #[test]
    fn a_fix_outside_its_file_splitting_a_character_or_overlapping_does_not_apply() {
        let mut db = FileDb::new();
        // "ก" is three bytes.
        let t = db.add("t.nova", "ก = 1\n");
        let outside = Fix::new("outside", vec![Edit::insert(99, t, "x")]);
        let split = Fix::new("split", vec![Edit::insert(1, t, "x")]);
        let overlap = Fix::new(
            "overlap",
            vec![
                Edit::replace(Span::new(0, 3, t), "a"),
                Edit::replace(Span::new(2, 5, t), "b"),
            ],
        );
        assert_eq!(outside.apply(&db), None);
        assert_eq!(split.apply(&db), None);
        assert_eq!(overlap.apply(&db), None);
    }

    #[test]
    fn the_renderer_prints_each_fix_as_a_help_line_after_the_notes() {
        let (db, a, _) = two_files();
        let d = Diagnostic::error("E0060", "cannot assign to immutable variable `x`")
            .with_primary_label(Span::new(4, 5, a), "here")
            .with_note("a note")
            .with_fix(Fix::new("make `x` mutable", vec![Edit::insert(4, a, "mut ")]));
        let out = render::render_to_string(&db, &[d]);
        let note = out.find("= a note").unwrap_or_else(|| panic!("{out}"));
        let help = out
            .find("= help: make `x` mutable")
            .unwrap_or_else(|| panic!("{out}"));
        assert!(note < help, "{out}");
    }
}
```

- [ ] **Step 2: Run the tests to see them fail**

Run: `cd /d/Projects/nona/nova && cargo test -p nova-diagnostics --lib 2>&1 | tail -5`
Expected: compile errors, `cannot find type 'Fix' in this scope` and
`cannot find type 'Edit'`.

- [ ] **Step 3: The types**

Insert after `Label`'s struct, the line `    pub primary: bool,\n}` (with
`insert_after.py`, anchor = that line and the `}` after it), this block
(marker `T1-TYPES`):

```rust

/// A suggested change: a title, and the text edits that make it (spec
/// `docs/superpowers/specs/2026-10-10-phase-3-4b-fixes-and-colour-design.md`
/// §3.1). A span carries its file, so one fix may edit several files.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Fix {
    pub title: String,
    pub edits: Vec<Edit>,
}

/// Replace `span`'s text with `text`. An empty span inserts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Edit {
    pub span: Span,
    pub text: String,
}

impl Edit {
    /// Insert `text` at byte `at` of `file`.
    pub fn insert(at: u32, file: FileId, text: impl Into<String>) -> Edit {
        Edit {
            span: Span::point(at, file),
            text: text.into(),
        }
    }

    /// Replace `span` with `text`.
    pub fn replace(span: Span, text: impl Into<String>) -> Edit {
        Edit {
            span,
            text: text.into(),
        }
    }
}

impl Fix {
    pub fn new(title: impl Into<String>, edits: Vec<Edit>) -> Fix {
        Fix {
            title: title.into(),
            edits,
        }
    }

    /// Each file the fix edits, in the order its edits first name them,
    /// with the text its edits make. `None` when an edit lies outside its
    /// file, splits a character, or overlaps another edit.
    pub fn apply(&self, db: &FileDb) -> Option<Vec<(FileId, String)>> {
        let mut files: Vec<FileId> = Vec::new();
        for e in &self.edits {
            if !files.contains(&e.span.file) {
                files.push(e.span.file);
            }
        }
        let mut out = Vec::new();
        for file in files {
            let text = db.get_source(file)?;
            let mut edits: Vec<&Edit> = self.edits.iter().filter(|e| e.span.file == file).collect();
            edits.sort_by_key(|e| (e.span.start, e.span.end));
            if edits.windows(2).any(|w| w[0].span.end > w[1].span.start) {
                return None;
            }
            let mut new = text.to_string();
            for e in edits.iter().rev() {
                let (start, end) = (e.span.start as usize, e.span.end as usize);
                if start > end
                    || end > text.len()
                    || !text.is_char_boundary(start)
                    || !text.is_char_boundary(end)
                {
                    return None;
                }
                new.replace_range(start..end, &e.text);
            }
            out.push((file, new));
        }
        Some(out)
    }
}
```

- [ ] **Step 4: `Diagnostic::fixes` and `with_fix`**

Make these edits in `crates/nova-diagnostics/src/lib.rs`, each with
`replace_once.py`:
1. In `pub struct Diagnostic`, replace `    pub notes: Vec<String>,\n}` with
   `    pub notes: Vec<String>,\n    /// Suggested changes, shown as \`help:\` lines and offered as quick\n    /// fixes (spec 3.4b §3).\n    pub fixes: Vec<Fix>,\n}`.
2. In both `error` and `warning`, replace `            notes: Vec::new(),\n        }`
   with `            notes: Vec::new(),\n            fixes: Vec::new(),\n        }`.
   The text occurs twice, so do it with one Python `str.replace` over the
   file and check the count is 2.
3. After `with_note`'s body, insert (marker `T1-WITH-FIX`):

```rust

    pub fn with_fix(mut self, fix: Fix) -> Self {
        self.fixes.push(fix);
        self
    }
```

- [ ] **Step 5: The renderers**

In `crates/nova-diagnostics/src/render.rs`, replace both
`            .with_notes(diag.notes.clone());` with
`            .with_notes(notes(diag));` (count 2), and append (marker
`T1-NOTES`):

```rust

/// The notes, then each fix's title as `help: …` (spec 3.4b §3.3).
fn notes(diag: &Diagnostic) -> Vec<String> {
    diag.notes
        .iter()
        .cloned()
        .chain(diag.fixes.iter().map(|f| format!("help: {}", f.title)))
        .collect()
}
```

- [ ] **Step 6: Run the tests**

Run: `cd /d/Projects/nona/nova && cargo test -p nova-diagnostics --lib 2>&1 | tail -5`
Expected: `test result: ok. 4 passed; 0 failed`.

Run: `cd /d/Projects/nona/nova && cargo build --workspace 2>&1 | tail -3`
Expected: `Finished`. Nothing outside this crate builds `Diagnostic` with
a struct literal (the fact-check confirmed), so nothing else changes.

- [ ] **Step 7: Commit**

Write `$P/msg-34b-1.txt`:

```text
nova-diagnostics: fixes on Diagnostic, and help lines

A Fix is a title and text edits, which may reach other files; Fix::apply
gives each edited file's new text, refusing edits outside their file,
splitting a character, or overlapping. Both renderers print each fix's
title as a help line after the notes (spec 3.4b §3.1, §3.3).

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
```

Run: `cd /d/Projects/nona/nova && cargo fmt --all && git add -u && git commit -q -F $P/msg-34b-1.txt && git log -1 --format=%s`
Expected: `nova-diagnostics: fixes on Diagnostic, and help lines`

---

### Task 2: Placing an edit by lines, and "did you mean"

**Files:**
- Create: `crates/nova-diagnostics/src/lines.rs`, `crates/nova-diagnostics/src/suggest.rs`
- Modify: `crates/nova-diagnostics/src/lib.rs` (the module list)

**Interfaces:**
- Consumes: `Fix`, `Edit`, `Spanned` (Task 1).
- Produces:
  - `lines::ending(text: &str) -> &'static str`;
  - `lines::line_start(text: &str, at: usize) -> usize`;
  - `lines::next_line_start(text: &str, at: usize) -> usize`;
  - `lines::is_blank(line: &str) -> bool`, `lines::is_comment_line(line: &str) -> bool`;
  - `lines::line_after(text: &str, at: usize, line: &str) -> (usize, String)`;
  - `lines::line_before_item(text: &str, item_start: usize, line: &str) -> (usize, String)`;
  - `lines::keyword_start(text: &str, name_start: usize) -> Option<usize>`;
  - `lines::arm_removal(text: &str, start: usize, end: usize) -> Option<(usize, usize)>`;
  - `suggest::distance(a: &str, b: &str) -> usize`;
  - `suggest::closest<'c>(name: &str, candidates: impl IntoIterator<Item = &'c str>) -> Option<&'c str>`;
  - `suggest::change_to<'c>(name: &Spanned<String>, candidates: impl IntoIterator<Item = &'c str>) -> Option<Fix>`.

- [ ] **Step 1: Write the failing tests**

Create `crates/nova-diagnostics/src/lines.rs` holding only its doc and
tests (marker `T2-LINES-TESTS`):

```rust
//! Placing an edit by the lines of a source text (spec
//! `docs/superpowers/specs/2026-10-10-phase-3-4b-fixes-and-colour-design.md`
//! §4.7). Offsets are bytes. The rules step over ASCII only, so every
//! offset they return is on a character boundary.

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_line_after_keeps_a_trailing_comment_on_its_line() {
        let text = "import a // why\nfn main() {}\n";
        assert_eq!(
            line_after(text, "import a".len(), "import b::{x}"),
            (16, "import b::{x}\n".to_string())
        );
        let crlf = "import a\r\nfn f() {}\r\n";
        assert_eq!(
            line_after(crlf, 8, "import b"),
            (10, "import b\r\n".to_string())
        );
    }

    #[test]
    fn a_line_after_the_last_line_without_an_ending_gets_one() {
        assert_eq!(line_after("import a", 8, "import b"), (8, "\nimport b".to_string()));
    }

    #[test]
    fn a_line_before_an_item_goes_above_its_comments_and_below_a_header() {
        let text = "// header\n\n// about f\n/// Doc.\nfn f() {}\n";
        let item = text.find("/// Doc.").unwrap();
        assert_eq!(
            line_before_item(text, item, "import m::{x}"),
            (11, "import m::{x}\n\n".to_string())
        );
    }

    #[test]
    fn the_keyword_before_a_name_takes_async_with_it() {
        let text = "async fn go() {}\nrecord P {}\ntype S =\n  | A\nfn f() {}\nlet x = 1\n";
        assert_eq!(keyword_start(text, text.find("go").unwrap()), Some(0));
        assert_eq!(keyword_start(text, text.find("P {").unwrap()), text.find("record"));
        assert_eq!(keyword_start(text, text.find("S =").unwrap()), text.find("type"));
        assert_eq!(keyword_start(text, text.find("f()").unwrap()), text.find("fn f"));
        assert_eq!(keyword_start(text, text.find("x =").unwrap()), None);
    }

    #[test]
    fn an_arm_alone_on_its_lines_goes_with_them() {
        let text = "match x {\n    1 => a,\n    _ => {\n        b\n    }\n}\n";
        let start = text.find("1 =>").unwrap();
        assert_eq!(arm_removal(text, start, start + "1 => a".len()), Some((10, 22)));
        let start = text.find("_ =>").unwrap();
        let end = text.rfind("    }").unwrap() + "    }".len();
        assert_eq!(
            arm_removal(text, start, end).map(|(s, e)| &text[s..e]),
            Some("    _ => {\n        b\n    }\n")
        );
    }

    #[test]
    fn an_arm_sharing_its_line_goes_alone_with_its_comma() {
        let text = "match x { 1 => a, 2 => b }";
        let start = text.find("1 =>").unwrap();
        assert_eq!(
            arm_removal(text, start, start + 6).map(|(s, e)| &text[s..e]),
            Some("1 => a, ")
        );
        let start = text.find("2 =>").unwrap();
        assert_eq!(
            arm_removal(text, start, start + 6).map(|(s, e)| &text[s..e]),
            Some("2 => b ")
        );
        // A comment between the body and its comma: no fix (plan decision 13).
        assert_eq!(arm_removal("1 => a /* c */,", 0, 6), None);
    }

    #[test]
    fn the_line_ending_is_the_texts_own() {
        assert_eq!(ending("a\r\nb\r\n"), "\r\n");
        assert_eq!(ending("a\nb"), "\n");
        assert!(is_blank("  \t\r\n") && !is_blank(" x\n"));
        assert!(is_comment_line("    // c\n") && !is_comment_line("x // c\n"));
    }
}
```

Create `crates/nova-diagnostics/src/suggest.rs` holding only its doc and
tests (marker `T2-SUGGEST-TESTS`):

```rust
//! "Did you mean" (spec
//! `docs/superpowers/specs/2026-10-10-phase-3-4b-fixes-and-colour-design.md`
//! §4.3).

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{FileId, Span};

    #[test]
    fn a_swap_is_one_edit() {
        assert_eq!(distance("cuont", "count"), 1);
        assert_eq!(distance("kitten", "sitting"), 3);
        assert_eq!(distance("", "ab"), 2);
    }

    #[test]
    fn the_nearest_within_the_bound_wins_then_byte_order() {
        assert_eq!(closest("cuont", ["mount", "count"]), Some("count"));
        assert_eq!(closest("ab", ["ac", "aa"]), Some("aa"));
        assert_eq!(closest("count", ["count"]), None);
    }

    #[test]
    fn the_bound_is_a_third_of_the_length() {
        // Five characters: one edit is offered, two are not.
        assert_eq!(closest("abcde", ["abxde"]), Some("abxde"));
        assert_eq!(closest("abcde", ["axxde"]), None);
        // Six characters: two edits are offered.
        assert_eq!(closest("abcdef", ["axxdef"]), Some("axxdef"));
    }

    #[test]
    fn a_name_equal_ignoring_case_is_offered_at_any_distance() {
        assert_eq!(
            closest("HTTPCLIENT", ["HTTPCLIENTS", "HttpClient"]),
            Some("HttpClient")
        );
    }

    #[test]
    fn closest_counts_characters_not_bytes() {
        // Review Focus 4: Thai is three bytes a character. Four characters
        // allow one edit; counted in bytes, twelve would allow four.
        assert_eq!(closest("ราคน", ["ราคา"]), Some("ราคา"));
        assert_eq!(closest("ราคน", ["รากก"]), None);
    }

    #[test]
    fn change_to_replaces_the_name() {
        let name = Spanned::new("cuont".to_string(), Span::new(4, 9, FileId::DUMMY));
        let fix = change_to(&name, ["count"]).unwrap();
        assert_eq!(fix.title, "change `cuont` to `count`");
        assert_eq!(fix.edits, vec![Edit::replace(name.span, "count")]);
    }
}
```

In `crates/nova-diagnostics/src/lib.rs`, replace `pub mod line_index;` with
`pub mod line_index;\npub mod lines;` and `pub mod render;` with
`pub mod render;\npub mod suggest;`.

- [ ] **Step 2: Run the tests to see them fail**

Run: `cd /d/Projects/nona/nova && cargo test -p nova-diagnostics --lib 2>&1 | tail -5`
Expected: compile errors, `cannot find function 'line_after'` and
`cannot find function 'closest'`, among others.

- [ ] **Step 3: The line rules**

Insert into `lines.rs` before `#[cfg(test)]` (marker `T2-LINES`):

```rust
/// The text's own line ending: `\r\n` when it holds one, else `\n`.
pub fn ending(text: &str) -> &'static str {
    if text.contains("\r\n") {
        "\r\n"
    } else {
        "\n"
    }
}

/// Where the line holding byte `at` starts.
pub fn line_start(text: &str, at: usize) -> usize {
    text[..at].rfind('\n').map_or(0, |i| i + 1)
}

/// Where the line after the one holding byte `at` starts, or the text's
/// end.
pub fn next_line_start(text: &str, at: usize) -> usize {
    text[at..].find('\n').map_or(text.len(), |i| at + i + 1)
}

/// Whether `line` holds only spaces, tabs and its line ending.
pub fn is_blank(line: &str) -> bool {
    line.chars().all(|c| matches!(c, ' ' | '\t' | '\r' | '\n'))
}

/// Whether `line`'s first characters after spaces and tabs are `//`.
pub fn is_comment_line(line: &str) -> bool {
    line.trim_start_matches([' ', '\t']).starts_with("//")
}

/// The line before the one starting at `start`: where it starts, and its
/// text with its ending.
fn line_before(text: &str, start: usize) -> Option<(usize, &str)> {
    if start == 0 {
        return None;
    }
    let begin = line_start(text, start - 1);
    Some((begin, &text[begin..start]))
}

/// The edit adding `line` on a line of its own after the line holding byte
/// `at`: where it goes, and what it inserts, in the text's own ending.
pub fn line_after(text: &str, at: usize, line: &str) -> (usize, String) {
    let nl = ending(text);
    let next = next_line_start(text, at);
    if next == text.len() && !text.ends_with('\n') {
        (next, format!("{nl}{line}"))
    } else {
        (next, format!("{line}{nl}"))
    }
}

/// The edit adding `line` and a blank line before the item starting at
/// byte `item_start`, above the comment lines directly over it. A comment
/// a blank line separates from the item stays above (spec §4.7).
pub fn line_before_item(text: &str, item_start: usize, line: &str) -> (usize, String) {
    let nl = ending(text);
    let mut at = line_start(text, item_start);
    while let Some((begin, prev)) = line_before(text, at) {
        if is_comment_line(prev) {
            at = begin;
        } else {
            break;
        }
    }
    (at, format!("{line}{nl}{nl}"))
}

/// Where the keyword before the item name at byte `name_start` starts:
/// `fn`, `record`, `type`, `trait` or `const`, with an `async` before
/// `fn` (spec §4.7).
pub fn keyword_start(text: &str, name_start: usize) -> Option<usize> {
    let bytes = text.as_bytes();
    let back_over_space = |mut i: usize| {
        while i > 0 && matches!(bytes[i - 1], b' ' | b'\t') {
            i -= 1;
        }
        i
    };
    let back_over_word = |mut i: usize| {
        while i > 0 && bytes[i - 1].is_ascii_alphabetic() {
            i -= 1;
        }
        i
    };
    let end = back_over_space(name_start);
    let start = back_over_word(end);
    let keyword = &text[start..end];
    if !matches!(keyword, "fn" | "record" | "type" | "trait" | "const") {
        return None;
    }
    if keyword == "fn" {
        let end = back_over_space(start);
        let word = back_over_word(end);
        if &text[word..end] == "async" {
            return Some(word);
        }
    }
    Some(start)
}

/// What removing a match arm, bytes `start..end`, takes (spec §4.7): a
/// comma after it and the spaces after that, and when nothing else shares
/// its lines, those whole lines. `None` when anything but a comma, a line
/// ending, the text's end or `}` follows the arm.
pub fn arm_removal(text: &str, start: usize, end: usize) -> Option<(usize, usize)> {
    let bytes = text.as_bytes();
    let skip = |mut i: usize| {
        while i < bytes.len() && matches!(bytes[i], b' ' | b'\t') {
            i += 1;
        }
        i
    };
    let mut after = skip(end);
    match bytes.get(after) {
        Some(b',') => after = skip(after + 1),
        None | Some(b'\r' | b'\n' | b'}') => {}
        Some(_) => return None,
    }
    let first = line_start(text, start);
    let alone_before = text[first..start].bytes().all(|b| matches!(b, b' ' | b'\t'));
    let alone_after = matches!(bytes.get(after), None | Some(b'\r' | b'\n'));
    if alone_before && alone_after {
        Some((first, next_line_start(text, after)))
    } else {
        Some((start, after))
    }
}

```

- [ ] **Step 4: "Did you mean"**

Insert into `suggest.rs` before `#[cfg(test)]` (marker `T2-SUGGEST`):

```rust
use crate::{Edit, Fix, Spanned};

/// The optimal string alignment distance between `a` and `b`, over Unicode
/// scalar values: an insertion, a deletion, a substitution, or a swap of two
/// adjacent characters each counts one.
// The indices are offsets into three arrays at once, which is what the
// lint's iterator form cannot express.
#[allow(clippy::needless_range_loop)]
pub fn distance(a: &str, b: &str) -> usize {
    let a: Vec<char> = a.chars().collect();
    let b: Vec<char> = b.chars().collect();
    let mut d = vec![vec![0usize; b.len() + 1]; a.len() + 1];
    for (i, row) in d.iter_mut().enumerate() {
        row[0] = i;
    }
    for (j, cell) in d[0].iter_mut().enumerate() {
        *cell = j;
    }
    for i in 1..=a.len() {
        for j in 1..=b.len() {
            let cost = usize::from(a[i - 1] != b[j - 1]);
            let mut best = (d[i - 1][j] + 1)
                .min(d[i][j - 1] + 1)
                .min(d[i - 1][j - 1] + cost);
            if i > 1 && j > 1 && a[i - 1] == b[j - 2] && a[i - 2] == b[j - 1] {
                best = best.min(d[i - 2][j - 2] + 1);
            }
            d[i][j] = best;
        }
    }
    d[a.len()][b.len()]
}

/// The candidate to offer for the unknown `name`, other than `name`
/// itself (spec §4.3): one equal to it ignoring ASCII case, at any
/// distance; else the nearest within `max(len, 3) / 3` edits, `len` in
/// characters; ties in byte order.
pub fn closest<'c>(name: &str, candidates: impl IntoIterator<Item = &'c str>) -> Option<&'c str> {
    let bound = name.chars().count().max(3) / 3;
    let mut best: Option<(bool, usize, &'c str)> = None;
    for c in candidates {
        if c == name {
            continue;
        }
        let key = if c.eq_ignore_ascii_case(name) {
            (false, 0, c)
        } else {
            let d = distance(name, c);
            if d > bound {
                continue;
            }
            (true, d, c)
        };
        if best.map_or(true, |b| key < b) {
            best = Some(key);
        }
    }
    best.map(|(_, _, c)| c)
}

/// "change `x` to `y`", `y` the closest of `candidates` (spec §4.3).
pub fn change_to<'c>(
    name: &Spanned<String>,
    candidates: impl IntoIterator<Item = &'c str>,
) -> Option<Fix> {
    let to = closest(&name.value, candidates)?;
    Some(Fix::new(
        format!("change `{}` to `{to}`", name.value),
        vec![Edit::replace(name.span, to)],
    ))
}

```

- [ ] **Step 5: Run the tests**

Run: `cd /d/Projects/nona/nova && cargo test -p nova-diagnostics --lib 2>&1 | tail -5`
Expected: `test result: ok. 17 passed; 0 failed` (Task 1's 4, `lines`' 7,
`suggest`'s 6).

Run: `cd /d/Projects/nona/nova && cargo clippy -p nova-diagnostics --all-targets -- -D warnings 2>&1 | tail -3`
Expected: `Finished`, no warning.

- [ ] **Step 6: Commit**

Write `$P/msg-34b-2.txt`:

```text
nova-diagnostics: placing an edit by lines; did you mean

lines holds spec 3.4b §4.7's rules: a line after an import, a line
before an item's comments, the keyword before a name, and what removing
a match arm takes. suggest holds §4.3's: the optimal string alignment
distance, rustc's bound with a case-insensitive match first, and the
"change x to y" fix.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
```

Run: `cd /d/Projects/nona/nova && cargo fmt --all && git add crates/nova-diagnostics/src/lines.rs crates/nova-diagnostics/src/suggest.rs && git add -u && git commit -q -F $P/msg-34b-2.txt && git log -1 --format=%s`
Expected: `nova-diagnostics: placing an edit by lines; did you mean`

---

### Task 3: The tables a fix is made from

**Files:**
- Modify: `crates/nova-resolver/src/lib.rs` (`ModuleSource`, `name_imports`,
  `Exports`, `Definitions`, `resolve_program`; tests at the end of
  `mod tests`)
- Modify: `crates/nova-driver/src/program.rs` (`Loaded`, `load_program`, a
  new `sibling_stem`)
- Modify: `crates/nova-driver/src/analyze.rs:161-168`, `crates/nova-driver/src/lib.rs:728-735`
  (the two `ModuleSource` literals)
- Create: `crates/nova-driver/tests/fixes.rs`

**Interfaces:**
- Consumes: nothing new.
- Produces:
  - `ModuleSource::{text: Option<&'a str>, package: Option<u32>, importable: Vec<(String, usize)>}`;
  - `pub struct Exported { pub value: Option<Res>, pub ty: Option<DefId>, pub trait_def: Option<DefId> }`,
    `Debug + Clone + Copy + Default + PartialEq + Eq`;
  - `Definitions::importable(&self, module: ModuleId) -> &[(String, ModuleId)]`;
  - `Definitions::exported(&self, module: ModuleId, name: &str) -> Exported`;
  - `Definitions::same_package(&self, a: ModuleId, b: ModuleId) -> bool`;
  - `Definitions::bound_outside_std(&self, module: ModuleId, name: &str) -> Exported`;
  - `Definitions::is_std_module(&self, module: ModuleId) -> bool`;
  - in `tests/fixes.rs`: `Buffers`, `options`, `MAIN`, `buffers`, `loose`,
    `fresh`, `write`, `manifest`, `project`, `app_and_geom`, `whole`,
    `messages`, `diagnostic`, `fix`, `apply`, `text`, `counts`,
    `assert_fixes`, which Tasks 4-6 use.

- [ ] **Step 1: Write the failing tests**

Append to the resolver's `mod tests`, with `append_in_mod.py`, this block
(marker `T3-RESOLVER-TESTS`):

```rust
    // === Phase 3.4b: what a fix needs to know (spec
    // docs/superpowers/specs/2026-10-10-phase-3-4b-fixes-and-colour-design.md §4.6) ===

    #[test]
    fn name_imports_makes_every_other_module_importable() {
        let p = resolve_two("fn main() {}\n", "pub fn area() -> Int { 1 }\n");
        let d = &p.definitions;
        assert_eq!(d.importable(ModuleId(0)), [("lib".to_string(), ModuleId(1))]);
        assert_eq!(d.importable(ModuleId(1)), [("main".to_string(), ModuleId(0))]);
        assert!(d.same_package(ModuleId(0), ModuleId(1)));
    }

    #[test]
    fn exported_names_each_namespace_a_public_name_has() {
        let p = resolve_two(
            "fn main() {}\n",
            "pub record Area { x: Int }\npub fn area() -> Int { 1 }\nfn hidden() -> Int { 0 }\n",
        );
        let d = &p.definitions;
        let area = d.exported(ModuleId(1), "area");
        assert!(matches!(area.value, Some(Res::Def(_))), "{area:?}");
        assert!(area.ty.is_none() && area.trait_def.is_none(), "{area:?}");
        assert!(d.exported(ModuleId(1), "Area").ty.is_some());
        assert_eq!(d.exported(ModuleId(1), "hidden"), Exported::default());
    }

    #[test]
    fn a_name_bound_through_std_is_not_bound_outside_it() {
        let p = resolve_two(
            "record Option { x: Int }\nfn main() {}\n",
            "pub fn area() -> Int { 1 }\n",
        );
        let d = &p.definitions;
        assert_eq!(d.bound_outside_std(ModuleId(0), "Some"), Exported::default());
        assert!(d.bound_outside_std(ModuleId(0), "Option").ty.is_some());
        assert!(matches!(
            d.bound_outside_std(ModuleId(0), "println").value,
            Some(Res::Builtin(_))
        ));
        assert!(d.is_std_module(ModuleId(2)) && !d.is_std_module(ModuleId(1)));
    }
```

Create `crates/nova-driver/tests/fixes.rs` (marker `T3-FIXES-RS`):

```rust
//! The fixes the front end attaches to its diagnostics (spec
//! `docs/superpowers/specs/2026-10-10-phase-3-4b-fixes-and-colour-design.md`
//! §4, §9.1), and the tables they are made from (§4.6). One test per case
//! of §9.1 (plan decision 17).

use std::collections::HashMap;
use std::io;
use std::path::{Path, PathBuf};

use nova_diagnostics::{Diagnostic, Fix};
use nova_driver::{analyze, analyze_program, Analysis, Options, Program, Roots, Sources};
use nova_resolver::ModuleId;

/// Buffers by path, then the disk: what the language server's overlay does.
#[derive(Clone)]
struct Buffers(Vec<(PathBuf, String)>);

impl Sources for Buffers {
    fn read(&self, path: &Path) -> io::Result<String> {
        match self.0.iter().find(|(p, _)| p == path) {
            Some((_, text)) => Ok(text.clone()),
            None => std::fs::read_to_string(path),
        }
    }
}

/// The language server's options: every stage, `@test` bodies, no MIR.
fn options() -> Options {
    Options {
        keep_going: true,
        tests: true,
        module_only: true,
        ..Options::default()
    }
}

const MAIN: &str = "mem/main.nova";

/// `files`, each `(path, text)`, as buffers.
fn buffers(files: &[(&str, &str)]) -> Buffers {
    Buffers(
        files
            .iter()
            .map(|(p, t)| (PathBuf::from(p), t.to_string()))
            .collect(),
    )
}

/// The loose program whose entry is `mem/main.nova`, analysed over `b`.
fn loose(b: &Buffers) -> Analysis {
    analyze(Path::new(MAIN), b, &options()).expect("the entry is readable")
}

/// A fresh directory with a fixed name, so each run replaces the last.
fn fresh(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("nova-driver-fixes-{name}"));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// Write each `(path, text)` under `dir`.
fn write(dir: &Path, files: &[(&str, &str)]) {
    for (path, text) in files {
        let path = dir.join(path);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, text).unwrap();
    }
}

/// A `nova.toml` for `name`, with `extra` appended.
fn manifest(name: &str, extra: &str) -> String {
    format!("[package]\nname = \"{name}\"\nversion = \"0.1.0\"\nedition = \"2026\"\n{extra}")
}

/// A project named `demo` in a fresh directory, holding `files`.
fn project(name: &str, files: &[(&str, &str)]) -> PathBuf {
    let dir = fresh(name);
    write(&dir, &[("nova.toml", manifest("demo", "").as_str())]);
    write(&dir, files);
    dir
}

/// `dir/app`, holding `app`, which depends by path on `dir/geom`, whose
/// `src/lib.nova` is `geom_lib`. Returns `dir/app`.
fn app_and_geom(name: &str, app: &[(&str, &str)], geom_lib: &str) -> PathBuf {
    let dir = fresh(name);
    write(
        &dir.join("geom"),
        &[
            ("nova.toml", manifest("geom", "").as_str()),
            ("src/lib.nova", geom_lib),
        ],
    );
    let deps = "\n[dependencies]\ngeom = { path = \"../geom\" }\n";
    let app_dir = dir.join("app");
    write(&app_dir, &[("nova.toml", manifest("app", deps).as_str())]);
    write(&app_dir, app);
    app_dir
}

/// `dir`'s whole project, its program, library and tests, over `b`.
fn whole(dir: &Path, b: &Buffers) -> Analysis {
    analyze_program(Program::for_package(dir, Roots::Test), b, &options())
        .expect("the project reads")
}

fn messages(a: &Analysis) -> String {
    a.diagnostics
        .iter()
        .map(|d| {
            let fixes: Vec<&str> = d.fixes.iter().map(|f| f.title.as_str()).collect();
            format!("{} {} {:?} {fixes:?}\n", d.code, d.message, d.notes)
        })
        .collect()
}

/// The first diagnostic with `code` whose message holds `part`.
#[track_caller]
fn diagnostic<'a>(a: &'a Analysis, code: &str, part: &str) -> &'a Diagnostic {
    a.diagnostics
        .iter()
        .find(|d| d.code == code && d.message.contains(part))
        .unwrap_or_else(|| panic!("no {code} holding {part:?}:\n{}", messages(a)))
}

/// `d`'s fix titled `title`.
#[track_caller]
fn fix<'a>(d: &'a Diagnostic, title: &str) -> &'a Fix {
    d.fixes
        .iter()
        .find(|f| f.title == title)
        .unwrap_or_else(|| panic!("no fix {title:?}: {:?}", d.fixes))
}

/// `base` with `fix` applied: each file it edits, read from a buffer of
/// its new text.
#[track_caller]
fn apply(a: &Analysis, fix: &Fix, base: &Buffers) -> Buffers {
    let edited = fix
        .apply(&a.db)
        .unwrap_or_else(|| panic!("the fix does not apply: {fix:?}"));
    let mut files = base.0.clone();
    for (file, text) in edited {
        let path = PathBuf::from(a.db.get_name(file).unwrap());
        files.retain(|(p, _)| *p != path);
        files.push((path, text));
    }
    Buffers(files)
}

/// The text of `b`'s buffer for the file whose path ends with `end`.
#[track_caller]
fn text<'b>(b: &'b Buffers, end: &str) -> &'b str {
    b.0.iter()
        .find(|(p, _)| p.ends_with(end))
        .map(|(_, t)| t.as_str())
        .unwrap_or_else(|| panic!("no buffer for {end}"))
}

/// How many diagnostics of each code `a` has.
fn counts(a: &Analysis) -> HashMap<String, usize> {
    let mut out = HashMap::new();
    for d in &a.diagnostics {
        *out.entry(d.code.clone()).or_insert(0) += 1;
    }
    out
}

/// Spec §3.2's promise: `code` is less common after the fix, and no code
/// but those in `exposes` is more common.
#[track_caller]
fn assert_fixes(before: &Analysis, after: &Analysis, code: &str, exposes: &[&str]) {
    let (b, a) = (counts(before), counts(after));
    assert!(
        a.get(code).copied().unwrap_or(0) < b.get(code).copied().unwrap_or(0),
        "{code} is not less common:\n{}",
        messages(after)
    );
    for (c, n) in &a {
        if !exposes.contains(&c.as_str()) {
            assert!(
                *n <= b.get(c).copied().unwrap_or(0),
                "{c} became more common:\n{}",
                messages(after)
            );
        }
    }
}

// === Task 3: the tables a fix is made from (spec §4.6) ===

#[test]
fn every_loaded_module_knows_what_it_could_import() {
    let app = app_and_geom(
        "importable",
        &[
            ("src/main.nova", "import geom\nimport shapes\n\nfn main() {}\n"),
            ("src/shapes.nova", "pub fn side() -> Int {\n    2\n}\n"),
            ("src/lib.nova", "pub fn name() -> String {\n    \"app\"\n}\n"),
            ("tests/check.nova", "import app\n\n@test\nfn t() {}\n"),
        ],
        "pub fn area() -> Int {\n    1\n}\n",
    );
    let a = whole(&app, &Buffers(Vec::new()));
    assert!(a.diagnostics.is_empty(), "{}", messages(&a));
    let d = a.definitions.as_ref().unwrap();
    let module = |end: &str| {
        let i = a
            .modules
            .iter()
            .position(|(_, p)| p.ends_with(end))
            .unwrap_or_else(|| panic!("no module {end}"));
        ModuleId(i as u32)
    };
    let main = module("app/src/main.nova");
    let shapes = module("app/src/shapes.nova");
    let lib = module("app/src/lib.nova");
    let check = module("app/tests/check.nova");
    let geom = module("geom/src/lib.nova");
    assert_eq!(
        d.importable(main).to_vec(),
        [
            ("geom".to_string(), geom),
            ("lib".to_string(), lib),
            ("shapes".to_string(), shapes),
        ]
    );
    assert_eq!(
        d.importable(check).to_vec(),
        [("app".to_string(), lib), ("geom".to_string(), geom)]
    );
    assert_eq!(d.importable(geom).to_vec(), Vec::<(String, ModuleId)>::new());
    assert!(d.same_package(main, check) && !d.same_package(main, geom));
}
```

- [ ] **Step 2: Run the tests to see them fail**

Run: `cd /d/Projects/nona/nova && cargo test -p nova-resolver --lib importable 2>&1 | tail -5`
Expected: compile errors, `no method named 'importable' found for struct
'Definitions'`, and `cannot find type 'Exported'`.

- [ ] **Step 3: `ModuleSource`'s fields**

In `crates/nova-resolver/src/lib.rs`:
1. In `pub struct ModuleSource`, after `    pub imports: std::collections::HashMap<String, ImportTarget>,`
   insert (marker `T3-MODULE-SOURCE`):

```rust
    /// The module's source text, for fixes placed by lines (spec
    /// `docs/superpowers/specs/2026-10-10-phase-3-4b-fixes-and-colour-design.md`
    /// §4.7). `None` in the resolver's own tests.
    pub text: Option<&'a str>,
    /// A key equal for two modules of one package; `None` for a loose
    /// program, whose files count as one package (spec 3.4b §4.6).
    pub package: Option<u32>,
    /// Every loaded module this one could import: the name an import of it
    /// writes, and its index in `modules` (spec 3.4b §4.6).
    pub importable: Vec<(String, usize)>,
```

2. In `ModuleSource::new`, replace `            imports: std::collections::HashMap::new(),\n        }`
   with `            imports: std::collections::HashMap::new(),\n            text: None,\n            package: None,\n            importable: Vec::new(),\n        }`.
3. Replace `name_imports` whole (from its doc comment's first line to its
   closing brace) with (marker `T3-NAME-IMPORTS`):

```rust
/// Fill each module's import table by module name, as a program without
/// packages has always resolved (ADR 0003): `import m` names the first
/// module called `m`. Every other module becomes importable by its name
/// (spec 3.4b §4.6).
pub fn name_imports(modules: &mut [ModuleSource]) {
    let names: Vec<String> = modules.iter().map(|m| m.name.clone()).collect();
    for (i, module) in modules.iter_mut().enumerate() {
        for item in &module.file.items {
            let Item::Import(import) = &item.value else {
                continue;
            };
            let Some(first) = import.path.value.segments.first() else {
                continue;
            };
            if let Some(index) = names.iter().position(|n| *n == first.value) {
                module
                    .imports
                    .insert(first.value.clone(), ImportTarget::Module(index));
            }
        }
        module.importable = names
            .iter()
            .enumerate()
            .filter(|(j, _)| *j != i)
            .map(|(j, n)| (n.clone(), j))
            .collect();
    }
}
```

4. In `resolve_program`'s `all`, replace
   `            imports: m.imports.clone(),\n        })` with
   `            imports: m.imports.clone(),\n            text: m.text,\n            package: m.package,\n            importable: m.importable.clone(),\n        })`.

- [ ] **Step 4: What `Definitions` keeps**

1. Replace `/// The public exports of one module (its \`pub\` items), used to resolve imports.\n#[derive(Default)]`
   with the same doc line and `#[derive(Debug, Default)]`.
2. In `pub struct Definitions`, after `    item_module: Vec<u32>,` insert
   (marker `T3-DEFINITIONS-FIELDS`):

```rust
    /// Each module's exports, std's included (spec 3.4b §4.6).
    exports: Vec<Exports>,
    /// What each module could import, by the name an import writes.
    importable: Vec<Vec<(String, ModuleId)>>,
    /// Each module's package key ([`ModuleSource::package`]).
    packages: Vec<Option<u32>>,
    /// The first std module's index: the user modules come before it.
    std_start: usize,
```

3. After the `impl Definitions` block that holds `names_in_scope` (its
   closing `}` follows `        out\n    }`), insert (marker `T3-EXPORTED`):

```rust

/// What a module exports under one name, in each namespace (spec 3.4b
/// §4.2).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Exported {
    pub value: Option<Res>,
    pub ty: Option<DefId>,
    pub trait_def: Option<DefId>,
}

impl Definitions {
    /// The modules `module` could import, by the name an import writes
    /// (spec 3.4b §4.6).
    pub fn importable(&self, module: ModuleId) -> &[(String, ModuleId)] {
        self.importable
            .get(module.0 as usize)
            .map_or(&[], Vec::as_slice)
    }

    /// What `module` exports under `name`.
    pub fn exported(&self, module: ModuleId, name: &str) -> Exported {
        let Some(e) = self.exports.get(module.0 as usize) else {
            return Exported::default();
        };
        Exported {
            value: e.values.get(name).copied(),
            ty: e.types.get(name).copied(),
            trait_def: e.traits.get(name).copied(),
        }
    }

    /// Whether `a` and `b` are modules of one package. A loose program's
    /// modules are.
    pub fn same_package(&self, a: ModuleId, b: ModuleId) -> bool {
        self.packages.get(a.0 as usize) == self.packages.get(b.0 as usize)
    }

    /// What `module` binds `name` to, in each namespace, other than through
    /// std's glob (plan decision 8): a binding equal to some std module's
    /// export of that name came through it. Builtins are bound outside std.
    pub fn bound_outside_std(&self, module: ModuleId, name: &str) -> Exported {
        let Some(scope) = self.modules.get(module.0 as usize) else {
            return Exported::default();
        };
        let std = &self.exports[self.std_start.min(self.exports.len())..];
        Exported {
            value: scope
                .values
                .get(name)
                .copied()
                .filter(|r| !std.iter().any(|e| e.values.get(name) == Some(r))),
            ty: scope
                .types
                .get(name)
                .copied()
                .filter(|d| !std.iter().any(|e| e.types.get(name) == Some(d))),
            trait_def: scope
                .traits
                .get(name)
                .copied()
                .filter(|d| !std.iter().any(|e| e.traits.get(name) == Some(d))),
        }
    }

    /// Whether `module` is one of the implicit std modules.
    pub fn is_std_module(&self, module: ModuleId) -> bool {
        let m = module.0 as usize;
        m >= self.std_start && m < self.modules.len()
    }
}
```

4. In `resolve_program`, after the std glob loop's closing `}` (the loop
   `for (mid, std_exports) in exports.iter().enumerate().skip(std_start) {`)
   and before `    ProgramResolution {`, insert (marker `T3-KEEP`):

```rust

    // Spec 3.4b §4.6: what a fix needs to know.
    definitions.importable = all
        .iter()
        .map(|m| {
            m.importable
                .iter()
                .map(|(n, i)| (n.clone(), ModuleId(*i as u32)))
                .collect()
        })
        .collect();
    definitions.packages = all.iter().map(|m| m.package).collect();
    definitions.std_start = std_start;
    definitions.exports = exports;
```

- [ ] **Step 5: Run the resolver's tests**

Run: `cd /d/Projects/nona/nova && cargo test -p nova-resolver --lib 2>&1 | tail -3`
Expected: `0 failed`, three more passed than before.

- [ ] **Step 6: The driver's importable tables, text and package**

In `crates/nova-driver/src/program.rs`:
1. In `pub(crate) struct Loaded`, after
   `    pub imports: HashMap<String, ImportTarget>,` insert:

```rust
    /// Every loaded module it could import, by the name an import of it
    /// writes (spec 3.4b §4.6).
    pub importable: Vec<(String, usize)>,
```

2. In `load_program`'s `load.modules.push(Loaded {`, replace
   `            imports: HashMap::new(),\n        });` with
   `            imports: HashMap::new(),\n            importable: Vec::new(),\n        });`.
3. In `load_program`, replace its last line `    Ok(load)\n}` (the
   function's end, after the loop that fills `module.imports`) with
   (marker `T3-IMPORTABLE`):

```rust
    // Spec 3.4b §4.6 (plan decision 9): what each loaded module could
    // import, by the name an import of it writes, through `find` itself.
    let mut place_of: Vec<Option<Place>> = vec![None; load.modules.len()];
    for (place, &i) in &places {
        place_of[i] = Some(place.clone());
    }
    let nowhere = Span::point(0, FileId::DUMMY);
    let mut importables: Vec<Vec<(String, usize)>> = Vec::new();
    for (i, from) in place_of.iter().enumerate() {
        let mut importable = Vec::new();
        if let Some(from) = from {
            let mut names: Vec<String> = place_of
                .iter()
                .flatten()
                .filter_map(|p| sibling_stem(from, p))
                .collect();
            if let (Place::Package { package, tests, .. }, Some(graph)) = (from, graph) {
                names.extend(
                    visible_edges(graph, *package, *tests)
                        .into_iter()
                        .map(|e| e.import_name),
                );
            }
            names.sort();
            names.dedup();
            for name in names {
                if let Found::Module(target, _) = loader.find(from, &name, nowhere, db) {
                    if let Some(&j) = places.get(&target) {
                        if j != i {
                            importable.push((name, j));
                        }
                    }
                }
            }
        }
        importables.push(importable);
    }
    for (module, importable) in load.modules.iter_mut().zip(importables) {
        module.importable = importable;
    }
    Ok(load)
}
```

4. After `fn module_dir`'s closing brace, insert (marker `T3-SIBLING`):

```rust

/// `p`'s stem when it is a module beside `from`: a loose file in the same
/// directory, or a module of the same package and directory (spec 3.4b
/// §4.6).
fn sibling_stem(from: &Place, p: &Place) -> Option<String> {
    match (from, p) {
        (Place::Loose { dir: a, .. }, Place::Loose { dir: b, stem }) if a == b => {
            Some(stem.clone())
        }
        (
            Place::Package {
                package: a,
                tests: s,
                ..
            },
            Place::Package {
                package: b,
                tests: t,
                stem,
            },
        ) if a == b && s == t => Some(stem.clone()),
        _ => None,
    }
}
```

5. In `crates/nova-driver/src/analyze.rs`, replace
   `            imports: m.imports.clone(),\n        })\n        .collect();\n    let resolved = nova_resolver::resolve_program(&module_sources`
   with
   `            imports: m.imports.clone(),\n            text: analysis.db.get_source(m.file),\n            package: m.package.map(|p| p.0),\n            importable: m.importable.clone(),\n        })\n        .collect();\n    let resolved = nova_resolver::resolve_program(&module_sources`.
6. In `crates/nova-driver/src/lib.rs`, replace
   `                imports: m.imports.clone(),\n            })\n            .collect();\n        let resolved = nova_resolver::resolve_program(&sources`
   with
   `                imports: m.imports.clone(),\n                text: self.db.get_source(m.file),\n                package: m.package.map(|p| p.0),\n                importable: m.importable.clone(),\n            })\n            .collect();\n        let resolved = nova_resolver::resolve_program(&sources`.

- [ ] **Step 7: Run the tests**

Run: `cd /d/Projects/nona/nova && cargo test -p nova-driver --test fixes 2>&1 | tail -3`
Expected: `test result: ok. 1 passed; 0 failed`.

Run: `cd /d/Projects/nona/nova && cargo test -p nova-resolver --lib && cargo test -p nova-driver 2>&1 | grep -E "^test result" | grep -v " 0 failed" ; echo done`
Expected: `done` alone: every result line has `0 failed`.

- [ ] **Step 8: Commit**

Write `$P/msg-34b-3.txt`:

```text
Resolver and driver: the tables a fix is made from

Each module now carries its text, its package key and what it could
import, and Definitions keeps each module's exports with lookups by
name and namespace (spec 3.4b §4.6). The driver builds the importable
tables with the loader's own find, so they follow 3.3a §4.3 exactly.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
```

Run: `cd /d/Projects/nona/nova && cargo fmt --all && git add crates/nova-driver/tests/fixes.rs && git add -u && git commit -q -F $P/msg-34b-3.txt && git log -1 --format=%s`
Expected: `Resolver and driver: the tables a fix is made from`

---

### Task 4: Sources in the checker; make it mutable; the index's locals

**Files:**
- Create: `crates/nova-typeck/src/check/fixes.rs`
- Modify: `crates/nova-typeck/src/lib.rs` (`CheckOptions`)
- Modify: `crates/nova-typeck/src/check.rs` (imports; `Checker`; `bind_local`
  and its seven callers; `note_params`; `place_root`, `PlaceRoot`,
  `require_mutable_place`; `check_assign`; two unit tests; the probe
  test's `CheckOptions` literal)
- Modify: `crates/nova-resolver/src/index.rs`, `src/lib.rs` (`LocalFlags`,
  `Index::locals`)
- Modify: `crates/nova-driver/src/analyze.rs:191-194`, `crates/nova-driver/src/lib.rs:745`
- Test: `crates/nova-driver/tests/fixes.rs`, `crates/nova-driver/tests/index.rs`

**Interfaces:**
- Consumes: `Fix`, `Edit` (Task 1); Task 3's test helpers.
- Produces:
  - `CheckOptions<'a>::sources: Option<&'a FileDb>`;
  - `pub(super) enum Binding { Let, Param, Other }` and
    `Checker::bindings: FxHashMap<Span, Binding>`;
  - `Checker::mutable_fix(&self, name: &str, decl: Span) -> Option<Fix>`;
  - `pub struct LocalFlags { pub parameter: bool, pub mutable: bool }`,
    `Index::locals: HashMap<Span, LocalFlags>`, which Task 12 reads.

- [ ] **Step 1: Write the failing tests**

Append to `crates/nova-driver/tests/fixes.rs` (marker `T4-TESTS`):

```rust

// === Task 4: make it mutable (spec §4.1; §9.1 cases 1-10) ===

/// The E0060 holding `part`, in the loose program `src`, has one fix,
/// "make `name` mutable", which leaves `decl` in the text; applied, the
/// error goes and nothing comes. The fix replaces the note (spec §3.3).
#[track_caller]
fn makes_mutable(src: &str, part: &str, name: &str, decl: &str) {
    let b = buffers(&[(MAIN, src)]);
    let a = loose(&b);
    let d = diagnostic(&a, "E0060", part);
    assert!(d.notes.is_empty(), "the fix replaces the note: {:?}", d.notes);
    assert_eq!(d.fixes.len(), 1, "{:?}", d.fixes);
    let edited = apply(&a, fix(d, &format!("make `{name}` mutable")), &b);
    let after = text(&edited, "main.nova");
    assert!(after.contains(decl), "{after}");
    assert_fixes(&a, &loose(&edited), "E0060", &[]);
}

/// The E0060 holding `part` has no fix, and keeps a note holding `note`.
#[track_caller]
fn no_mutable_fix(src: &str, part: &str, note: &str) {
    let a = loose(&buffers(&[(MAIN, src)]));
    let d = diagnostic(&a, "E0060", part);
    assert!(d.fixes.is_empty(), "{:?}", d.fixes);
    assert!(d.notes.iter().any(|n| n.contains(note)), "{:?}", d.notes);
}

#[test]
fn case_01_make_mutable_for_a_let() {
    makes_mutable(
        "fn main() {\n    let x = 0\n    x = 1\n    println(\"${x}\")\n}\n",
        "cannot assign to immutable variable `x`",
        "x",
        "let mut x = 0",
    );
}

#[test]
fn case_02_make_mutable_for_a_compound_assignment() {
    makes_mutable(
        "fn main() {\n    let x = 0\n    x += 1\n    println(\"${x}\")\n}\n",
        "cannot assign to immutable variable `x`",
        "x",
        "let mut x = 0",
    );
}

#[test]
fn case_03_make_mutable_for_an_element() {
    makes_mutable(
        "fn main() {\n    let a = [1, 2]\n    a[0] = 3\n    println(\"${a[0]}\")\n}\n",
        "an element of immutable `a`",
        "a",
        "let mut a = [1, 2]",
    );
}

#[test]
fn case_04_make_mutable_for_a_field() {
    makes_mutable(
        "record P { v: Int }\n\nfn main() {\n    let p = P { v: 1 }\n    p.v = 2\n    println(\"${p.v}\")\n}\n",
        "a field of immutable `p`",
        "p",
        "let mut p = P { v: 1 }",
    );
}

#[test]
fn case_05_make_mutable_for_a_mut_self_call() {
    makes_mutable(
        "record P { v: Int }\n\nimpl P {\n    fn bump(mut self) {\n        self.v = self.v + 1\n    }\n}\n\nfn main() {\n    let p = P { v: 1 }\n    p.bump()\n}\n",
        "mutates its receiver, but `p` is immutable",
        "p",
        "let mut p = P { v: 1 }",
    );
}

#[test]
fn case_06_make_mutable_for_a_parameter() {
    makes_mutable(
        "fn twice(n: Int) -> Int {\n    n = n * 2\n    n\n}\n\nfn main() {\n    println(\"${twice(2)}\")\n}\n",
        "cannot assign to immutable variable `n`",
        "n",
        "fn twice(mut n: Int)",
    );
}

#[test]
fn case_07_make_mutable_for_a_closure_parameter() {
    makes_mutable(
        "fn main() {\n    let f = |n: Int| {\n        n = n + 1\n        n\n    }\n    println(\"${f(1)}\")\n}\n",
        "cannot assign to immutable variable `n`",
        "n",
        "|mut n: Int|",
    );
}

#[test]
fn case_08_no_make_mutable_for_a_match_binding() {
    no_mutable_fix(
        "fn main() {\n    let o = Some(1)\n    match o {\n        Some(v) => {\n            v = 2\n        }\n        None => {}\n    }\n}\n",
        "cannot assign to immutable variable `v`",
        "let mut v",
    );
}

#[test]
fn case_09_no_make_mutable_for_a_for_variable() {
    no_mutable_fix(
        "fn main() {\n    for i in 0..3 {\n        i = i + 1\n    }\n}\n",
        "cannot assign to immutable variable `i`",
        "let mut i",
    );
}

#[test]
fn case_10_no_make_mutable_for_self() {
    no_mutable_fix(
        "record C { n: Int }\n\nimpl C {\n    fn reset(self) {\n        self.n = 0\n    }\n}\n\nfn main() {}\n",
        "a field of immutable `self`",
        "mut self",
    );
}
```

Append to `crates/nova-driver/tests/index.rs` (marker `T4-INDEX-TEST`):

```rust

#[test]
fn the_index_records_how_each_local_was_declared() {
    // Spec 3.4b §4.6, §7.2: parameters and `mut`, for semantic tokens.
    let src = "fn f(mut a: Int, b: Int) -> Int {\n    let mut c = a\n    let d = b\n    c = d\n    c\n}\n\nfn main() {}\n";
    let a = analyse(&[(MAIN, src)]);
    let flags = |word: &str| {
        let (file, start) = place(&a, MAIN, word, 0);
        let span = nova_diagnostics::Span::new(start, start + word.len() as u32, file);
        index(&a).locals.get(&span).copied()
    };
    let declared = |parameter, mutable| Some(nova_resolver::LocalFlags { parameter, mutable });
    assert_eq!(flags("a"), declared(true, true));
    assert_eq!(flags("b"), declared(true, false));
    assert_eq!(flags("c"), declared(false, true));
    assert_eq!(flags("d"), declared(false, false));
}
```

- [ ] **Step 2: Run the tests to see them fail**

Run: `cd /d/Projects/nona/nova && cargo test -p nova-driver --test fixes case_0 2>&1 | grep -E "^test |test result"`
Expected: cases 1-7 FAIL with "the fix replaces the note" (the note is
still added and there is no fix), and cases 8-10 pass.

Run: `cd /d/Projects/nona/nova && cargo test -p nova-driver --test index the_index_records_how 2>&1 | tail -3`
Expected: a compile error, `no field 'locals' on type '&Index'`.

- [ ] **Step 3: The index's locals**

In `crates/nova-resolver/src/index.rs`, after `    pub types: HashMap<Span, String>,`
in `pub struct Index` insert:

```rust
    /// Each local's declaration span: whether it is a parameter and whether
    /// it is `mut` (spec 3.4b §4.6, §7.2). Recorded with the index only.
    pub locals: HashMap<Span, LocalFlags>,
```

and before `/// One name in the source.` insert (marker `T4-LOCAL-FLAGS`):

```rust
/// How a local was declared, for semantic tokens (spec 3.4b §7.2).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LocalFlags {
    pub parameter: bool,
    pub mutable: bool,
}

```

In `crates/nova-resolver/src/lib.rs`, replace
`pub use index::{Index, Occurrence, Role, Target};` with
`pub use index::{Index, LocalFlags, Occurrence, Role, Target};`.

- [ ] **Step 4: `CheckOptions::sources`**

In `crates/nova-typeck/src/lib.rs`, replace the `CheckOptions` struct
(from `/// What [`check_with`] does beyond checking.` to its closing brace)
with (marker `T4-CHECK-OPTIONS`):

```rust
/// What [`check_with`] does beyond checking.
#[derive(Debug, Clone, Copy, Default)]
pub struct CheckOptions<'a> {
    pub probe: Option<ProbePoint>,
    /// Record the language server's index (spec
    /// `docs/superpowers/specs/2026-10-10-phase-3-4a-navigation-design.md`
    /// §3). Recording is write-only: the checker never reads it.
    pub index: bool,
    /// The program's sources, for the fixes placed by lines (spec
    /// `docs/superpowers/specs/2026-10-10-phase-3-4b-fixes-and-colour-design.md`
    /// §4.7). Without them, those fixes are not offered.
    pub sources: Option<&'a nova_diagnostics::FileDb>,
}
```

In `crates/nova-typeck/src/check.rs`'s probe test, replace
`            probe: Some(ProbePoint { file, offset }),\n            index: false,\n        };`
with
`            probe: Some(ProbePoint { file, offset }),\n            index: false,\n            sources: None,\n        };`.

- [ ] **Step 5: The checker's state**

In `crates/nova-typeck/src/check.rs`:
1. Replace `use nova_diagnostics::{Diagnostic, Span, Spanned};` with
   `use nova_diagnostics::{Diagnostic, FileDb, Span, Spanned};`, and
   `    Builtin, DefId, DefKind, Definitions, Index, MethodOwner, ModuleId, Res, Role, Target,`
   with
   `    Builtin, DefId, DefKind, Definitions, Index, LocalFlags, MethodOwner, ModuleId, Res, Role,\n    Target,`.
2. After the `use crate::{ … };` block (it ends `    ProbeResult,\n};`),
   insert:

```rust

mod fixes;

use fixes::Binding;
```

3. In `struct Checker`, after `    type_params: Vec<(String, Span)>,` insert
   (marker `T4-CHECKER-FIELDS`):

```rust
    /// The program's sources (spec 3.4b §4.7), when the caller gave them.
    sources: Option<&'a FileDb>,
    /// How each local the user wrote was bound, by its declaration span
    /// (spec 3.4b §4.1; plan decision 5).
    bindings: FxHashMap<Span, Binding>,
```

4. In `check_with`'s `Checker { … }`, after `        type_params: Vec::new(),`
   insert:

```rust
        sources: options.sources,
        bindings: FxHashMap::default(),
```

`sources` is read from Task 5 on. Until then the compiler warns that it
is never read; Task 5 removes the warning, and no clippy run happens
before it.

- [ ] **Step 6: The fixes module, with "make mutable"**

Create `crates/nova-typeck/src/check/fixes.rs` (marker `T4-FIXES-RS`):

```rust
//! The fixes the checker attaches to the errors it raises (spec
//! `docs/superpowers/specs/2026-10-10-phase-3-4b-fixes-and-colour-design.md`
//! §4; plan decision 3).

use nova_diagnostics::{Edit, Fix, Span};

use super::Checker;

/// How a local the user wrote was bound (spec §4.1).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Binding {
    /// By a `let`.
    Let,
    /// As a function's, a method's or a closure's parameter, `self`
    /// included.
    Param,
    /// By a pattern or a `for` loop, where `mut` does not parse.
    Other,
}

impl<'a> Checker<'a> {
    /// "make `x` mutable" (spec §4.1): `mut ` before the name of a local
    /// bound by a `let` or as a parameter. `decl` is that name's span.
    /// Never for `self`, since changing a receiver changes every caller.
    pub(super) fn mutable_fix(&self, name: &str, decl: Span) -> Option<Fix> {
        if name == "self" {
            return None;
        }
        match self.bindings.get(&decl) {
            Some(Binding::Let | Binding::Param) => Some(Fix::new(
                format!("make `{name}` mutable"),
                vec![Edit::insert(decl.start, decl.file, "mut ")],
            )),
            _ => None,
        }
    }
}
```

- [ ] **Step 7: `bind_local` takes the binding**

1. Replace `bind_local` and `note_params` (from
   `    /// \`fcx.new_local\` for a name the user wrote, declaring it (spec §3.2).`
   to the end of `note_params`) with (marker `T4-BIND-LOCAL`):

```rust
    /// `fcx.new_local` for a name the user wrote, declaring it (spec §3.2).
    /// `_` declares nothing. The checker's own locals call `new_local`
    /// directly and are never recorded. `binding` says how it was bound
    /// (spec 3.4b §4.1), which the index also keeps (3.4b §4.6).
    fn bind_local(
        &mut self,
        fcx: &mut FnCtx,
        name: String,
        ty: Ty,
        is_mut: bool,
        span: Span,
        binding: Binding,
    ) -> LocalId {
        if name != "_" {
            self.note_decl(span, Target::Local(span));
            if let Some(index) = self.index.as_mut() {
                index.locals.insert(
                    span,
                    LocalFlags {
                        parameter: binding == Binding::Param,
                        mutable: is_mut,
                    },
                );
            }
        }
        self.bindings.insert(span, binding);
        fcx.new_local(name, ty, is_mut, span)
    }

    /// Declare parameters that no body binds: a trait's required method's
    /// and an extern function's.
    fn note_params(&mut self, params: &[ast::Param]) {
        for p in params {
            self.note_decl(p.name.span, Target::Local(p.name.span));
            if let Some(index) = self.index.as_mut() {
                index.locals.insert(
                    p.name.span,
                    LocalFlags {
                        parameter: true,
                        mutable: p.is_mut,
                    },
                );
            }
        }
    }
```

2. Give each of the seven callers its binding. Each old text occurs once:

| Line (at `1c7bae5`) | Old | New ends with |
|---|---|---|
| 3012 | `                p.is_mut,\n                p.name.span,\n            );` | `p.name.span,\n                Binding::Param,\n            );` |
| 3271 | `self.bind_local(fcx, name, value.ty.clone(), *is_mut \|\| pat_mut, name_span);` | `…, name_span, Binding::Let);` |
| 4374 | `let i = self.bind_local(fcx, var_name, Ty::Int, false, var_span);` | `…, var_span, Binding::Other);` |
| 4649 | `let elem = self.bind_local(fcx, var_name, item_ty, false, var_span);` | `…, var_span, Binding::Other);` |
| 4785 | `self.bind_local(fcx, p.name.value.clone(), ty.clone(), p.is_mut, p.name.span);` | `…, p.name.span, Binding::Param);` |
| 6930 | `                    *is_mut,\n                    name.span,\n                );\n                hir::Pattern::Bind(local)` | `name.span,\n                    Binding::Other,\n                );\n                hir::Pattern::Bind(local)` |
| 7099 | `self.bind_local(fcx, name.value.clone(), bound_ty, *is_mut, name.span);` | `…, name.span, Binding::Other);` |

Then run `cargo fmt --all`, which reflows the lines that grew.

- [ ] **Step 8: The fix at both E0060 sites**

1. In `place_root`, replace
   `                    Some(_) => PlaceRoot::ImmutableLocal(p.segments[0].value.clone()),`
   with

```rust
                    Some(l) => PlaceRoot::ImmutableLocal(
                        p.segments[0].value.clone(),
                        fcx.locals[l.0 as usize].span,
                    ),
```

2. In `enum PlaceRoot`, replace
   `    /// Rooted at an immutable local of the given name.\n    ImmutableLocal(String),`
   with
   `    /// Rooted at an immutable local: its name, and its declaration's span\n    /// (spec 3.4b §4.1).\n    ImmutableLocal(String, Span),`.
3. In `require_mutable_place`, replace
   `            PlaceRoot::ImmutableLocal(name) => {\n                self.error("E0060", what.immutable_message(&name), span);`
   with (marker `T4-REQUIRE`):

```rust
            PlaceRoot::ImmutableLocal(name, decl) => {
                self.error("E0060", what.immutable_message(&name), span);
                // Spec 3.4b §3.3: where a fix is offered, it says what the
                // note would.
                if let Some(fix) = self.mutable_fix(&name, decl) {
                    self.push_fix(Some(fix));
                    return;
                }
```

4. In `check_assign`, replace (marker `T4-ASSIGN-OLD`)

```rust
        if !info.is_mut {
            self.error(
                "E0060",
                format!("cannot assign to immutable variable `{name}`"),
                span,
            );
            self.diagnostics
                .last_mut()
                .expect("just pushed")
                .notes
                .push(format!(
                    "declare it as `let mut {name}` to allow assignment"
                ));
        }
```

with (marker `T4-ASSIGN-NEW`)

```rust
        if !info.is_mut {
            self.error(
                "E0060",
                format!("cannot assign to immutable variable `{name}`"),
                span,
            );
            // Spec 3.4b §3.3: where a fix is offered, it says what the note
            // would.
            match self.mutable_fix(name, info.span) {
                Some(fix) => self.push_fix(Some(fix)),
                None => self
                    .diagnostics
                    .last_mut()
                    .expect("just pushed")
                    .notes
                    .push(format!("declare it as `let mut {name}` to allow assignment")),
            }
        }
```

5. Add `push_fix` to `fixes.rs`'s `impl` block, after `mutable_fix`
   (marker `T4-PUSH-FIX`):

```rust

    /// Add `fix`, if there is one, to the diagnostic just pushed.
    pub(super) fn push_fix(&mut self, fix: Option<Fix>) {
        if let (Some(fix), Some(d)) = (fix, self.diagnostics.last_mut()) {
            d.fixes.push(fix);
        }
    }
```

- [ ] **Step 9: The two unit tests that asserted the note**

In `crates/nova-typeck/src/check.rs`, in
`mut_self_method_on_immutable_receiver_suggests_let_mut`, replace

```rust
        assert!(
            d.notes.iter().any(|n| n.contains("let mut p")),
            "{:?}",
            d.notes
        );
```

with (marker `T4-UNIT-P`)

```rust
        // Spec 3.4b §3.3: the fix says what the note did.
        assert!(d.notes.is_empty(), "{:?}", d.notes);
        assert_eq!(
            d.fixes.iter().map(|f| f.title.as_str()).collect::<Vec<_>>(),
            ["make `p` mutable"]
        );
```

and in `immutable_self_root_is_advised_to_use_mut_self_not_let_mut_self`
the same block with `c` for `p` (old: `n.contains("let mut c")`; new
title: ``"make `c` mutable"``).

- [ ] **Step 10: The driver passes the sources**

1. In `crates/nova-driver/src/analyze.rs`, replace
   `            probe,\n            index: options.index,\n        },` with
   `            probe,\n            index: options.index,\n            sources: Some(&analysis.db),\n        },`.
2. In `crates/nova-driver/src/lib.rs`, replace
   `        let checked = nova_typeck::check(&resolved.file, &resolved.definitions);\n        self.render(&checked.diagnostics);`
   with

```rust
        let checked = nova_typeck::check_with(
            &resolved.file,
            &resolved.definitions,
            &nova_typeck::CheckOptions {
                sources: Some(&self.db),
                ..Default::default()
            },
        );
        self.render(&checked.diagnostics);
```

- [ ] **Step 11: Run the tests**

Run: `cd /d/Projects/nona/nova && cargo test -p nova-driver --test fixes 2>&1 | tail -3`
Expected: `test result: ok. 11 passed; 0 failed`.

Run: `cd /d/Projects/nona/nova && cargo test -p nova-driver --test index 2>&1 | tail -3`
Expected: `0 failed`, one more passed than before.

Run: `cd /d/Projects/nona/nova && cargo test -p nova-typeck 2>&1 | grep -E "^test result" | grep -v " 0 failed"; echo done`
Expected: `done` alone.

- [ ] **Step 12: Commit**

Write `$P/msg-34b-4.txt`:

```text
nova-typeck: sources in the checker, and make it mutable

CheckOptions carries the program's sources for the fixes placed by
lines. Each local records how it was bound, and E0060 offers "make x
mutable" for a let or a parameter, in place of its note; a pattern
binding, a for variable and self keep the note (spec 3.4b §4.1, §3.3).
The index records each local's parameter and mut flags, for semantic
tokens.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
```

Run: `cd /d/Projects/nona/nova && cargo fmt --all && git add crates/nova-typeck/src/check/fixes.rs && git add -u && git commit -q -F $P/msg-34b-4.txt && git log -1 --format=%s`
Expected: `nova-typeck: sources in the checker, and make it mutable`

---

### Task 5: Import a name, and "did you mean"

**Files:**
- Modify: `crates/nova-typeck/src/check/fixes.rs`
- Modify: `crates/nova-typeck/src/check.rs` (imports, `Checker`, and the
  sites the table in Step 4 lists)
- Test: `crates/nova-driver/tests/fixes.rs`

**Interfaces:**
- Consumes: `lines` and `suggest` (Task 2); `Definitions::{importable,
  exported, bound_outside_std, names_in_scope}` (Task 3); `push_fix`
  (Task 4).
- Produces, all `pub(super)` in `check/fixes.rs`:
  - `enum Need { Value, Call(usize), Type, Record, Trait, Qualifier { member: String, call: Option<usize> }, Pattern { variant: String, arity: usize, scrutinee: Option<DefId> } }`;
  - `fn visible_locals(fcx: &FnCtx) -> Vec<String>`;
  - `fn scrutinee_sum(fcx: &FnCtx, ty: &Ty) -> Option<DefId>`;
  - `Checker::name_fixes(&mut self, name: Spanned<String>, need: Need, nearby: Vec<String>)`;
  - `Checker::field_fix(&self, ty: &Ty, field: &Spanned<String>) -> Option<Fix>`;
  - `Checker::method_fix(&self, recv: &Ty, fcx: &FnCtx, method: &Spanned<String>) -> Option<Fix>`;
  - `Checker::qualified_callee: Option<usize>` (plan decision 7).

- [ ] **Step 1: Write the failing tests**

Append to `crates/nova-driver/tests/fixes.rs` (marker `T5-TESTS`):

```rust

// === Task 5: import a name, and did you mean (spec §4.2, §4.3; §9.1
// cases 11-23, 25 and 26) ===

const GEOMETRY: &str = "pub record Point { x: Int, y: Int }\n\npub fn origin() -> Point {\n    Point { x: 0, y: 0 }\n}\n\npub fn manhattan(p: Point) -> Int {\n    p.x + p.y\n}\n\npub trait Shape {\n    fn area(self) -> Int\n}\n\npub fn one() -> Int {\n    1\n}\n";

const KINDS: &str = "pub type Shape =\n  | Circle(Int)\n  | Empty\n\npub record P { v: Int }\n\nimpl P {\n    fn new() -> P {\n        P { v: 7 }\n    }\n}\n\npub fn make() -> Shape {\n    Empty\n}\n\npub fn one() -> Int {\n    1\n}\n";

/// In the loose program `files`, the diagnostic `code` holding `part`
/// offers `title`, which leaves `expect` in the file ending `file`;
/// applied, the diagnostic goes and nothing comes.
#[track_caller]
fn offers(files: &[(&str, &str)], code: &str, part: &str, title: &str, file: &str, expect: &str) {
    let b = buffers(files);
    let a = loose(&b);
    let d = diagnostic(&a, code, part);
    let edited = apply(&a, fix(d, title), &b);
    let after = text(&edited, file);
    assert!(after.contains(expect), "{after}");
    assert_fixes(&a, &loose(&edited), code, &[]);
}

/// The diagnostic `code` holding `part` offers no import.
#[track_caller]
fn no_import(files: &[(&str, &str)], code: &str, part: &str) {
    let a = loose(&buffers(files));
    let d = diagnostic(&a, code, part);
    assert!(
        d.fixes.iter().all(|f| !f.title.starts_with("import")),
        "{:?}",
        d.fixes
    );
}

#[test]
fn case_11_import_extends_an_existing_list() {
    offers(
        &[
            (
                MAIN,
                "import geometry::{origin}\n\nfn main() {\n    let d = manhattan(origin())\n    println(\"${d}\")\n}\n",
            ),
            ("mem/geometry.nova", GEOMETRY),
        ],
        "E0001",
        "cannot find function `manhattan`",
        "import `manhattan` from `geometry`",
        "main.nova",
        "import geometry::{origin, manhattan}\n",
    );
}

#[test]
fn case_12_import_adds_a_line_after_the_last_import() {
    for nl in ["\n", "\r\n"] {
        let main = "import shapes // the shapes\n\nfn main() {\n    println(\"${twice()} ${one()}\")\n}\n"
            .replace('\n', nl);
        let shapes = "import geometry\n\npub fn twice() -> Int {\n    one() * 2\n}\n".replace('\n', nl);
        offers(
            &[
                (MAIN, main.as_str()),
                ("mem/shapes.nova", shapes.as_str()),
                ("mem/geometry.nova", GEOMETRY),
            ],
            "E0001",
            "cannot find function `one`",
            "import `one` from `geometry`",
            "main.nova",
            &format!("import shapes // the shapes{nl}import geometry::{{one}}{nl}{nl}fn main()"),
        );
    }
}

#[test]
fn case_13_import_goes_above_the_first_item_below_a_header() {
    for (k, nl) in ["\n", "\r\n"].into_iter().enumerate() {
        let main = "// A header.\n\n// What main does.\nfn main() {\n    println(\"${twice()} ${one()}\")\n}\n"
            .replace('\n', nl);
        let lib = "import geometry\nimport shapes\n\npub fn name() -> String {\n    \"demo\"\n}\n"
            .replace('\n', nl);
        let geometry = GEOMETRY.replace('\n', nl);
        let shapes = "pub fn twice() -> Int {\n    2\n}\n".replace('\n', nl);
        let dir = project(
            &format!("above-the-first-item-{k}"),
            &[
                ("src/main.nova", main.as_str()),
                ("src/lib.nova", lib.as_str()),
                ("src/geometry.nova", geometry.as_str()),
                ("src/shapes.nova", shapes.as_str()),
            ],
        );
        let none = Buffers(Vec::new());
        let a = whole(&dir, &none);
        let d = diagnostic(&a, "E0001", "cannot find function `one`");
        let edited = apply(&a, fix(d, "import `one` from `geometry`"), &none);
        let after = text(&edited, "main.nova");
        let want = format!(
            "// A header.{nl}{nl}import geometry::{{one}}{nl}{nl}// What main does.{nl}fn main()"
        );
        assert!(after.starts_with(&want), "{after}");
        assert_fixes(&a, &whole(&dir, &edited), "E0001", &[]);
    }
}

#[test]
fn case_14_import_from_a_dependencys_library() {
    let app = app_and_geom(
        "from-a-dependency",
        &[
            (
                "src/main.nova",
                "import geom\nimport shapes\n\nfn main() {\n    println(\"${area()} ${twice()}\")\n}\n",
            ),
            ("src/shapes.nova", "pub fn twice() -> Int {\n    side() * 2\n}\n"),
        ],
        "pub fn area() -> Int {\n    1\n}\n\npub fn side() -> Int {\n    2\n}\n",
    );
    let none = Buffers(Vec::new());
    let a = whole(&app, &none);
    let d = diagnostic(&a, "E0001", "cannot find function `side`");
    let edited = apply(&a, fix(d, "import `side` from `geom`"), &none);
    let after = text(&edited, "shapes.nova");
    assert!(
        after.starts_with("import geom::{side}\n\npub fn twice()"),
        "{after}"
    );
    assert_fixes(&a, &whole(&app, &edited), "E0001", &[]);
}

#[test]
fn case_15_import_a_type_a_record_and_a_trait() {
    let geometry = ("mem/geometry.nova", GEOMETRY);
    offers(
        &[
            (
                MAIN,
                "import geometry::{origin}\n\nfn show(p: Point) -> Int {\n    p.x\n}\n\nfn main() {\n    println(\"${show(origin())}\")\n}\n",
            ),
            geometry,
        ],
        "E0001",
        "cannot find type `Point`",
        "import `Point` from `geometry`",
        "main.nova",
        "import geometry::{origin, Point}\n",
    );
    offers(
        &[
            (
                MAIN,
                "import geometry::{origin}\n\nfn main() {\n    let p = Point { x: 1, y: 2 }\n    println(\"${p.x}\")\n}\n",
            ),
            geometry,
        ],
        "E0001",
        "cannot find record `Point`",
        "import `Point` from `geometry`",
        "main.nova",
        "import geometry::{origin, Point}\n",
    );
    offers(
        &[
            (
                MAIN,
                "import geometry::{origin}\n\nrecord Sq { s: Int }\n\nimpl Shape for Sq {\n    fn area(self) -> Int {\n        self.s * self.s\n    }\n}\n\nfn main() {}\n",
            ),
            geometry,
        ],
        "E0001",
        "cannot find trait `Shape`",
        "import `Shape` from `geometry`",
        "main.nova",
        "import geometry::{origin, Shape}\n",
    );
}

#[test]
fn case_16_import_an_unknown_qualifier() {
    let kinds = ("mem/kinds.nova", KINDS);
    offers(
        &[
            (
                MAIN,
                "import kinds::{one}\n\nfn main() {\n    let p = P::new()\n    println(\"${p.v} ${one()}\")\n}\n",
            ),
            kinds,
        ],
        "E0900",
        "module-qualified paths",
        "import `P` from `kinds`",
        "main.nova",
        "import kinds::{one, P}\n",
    );
    offers(
        &[
            (MAIN, "import kinds::{one}\n\nfn main() {\n    let e = Shape::Empty\n}\n"),
            kinds,
        ],
        "E0900",
        "module-qualified paths",
        "import `Shape` from `kinds`",
        "main.nova",
        "import kinds::{one, Shape}\n",
    );
    offers(
        &[
            (
                MAIN,
                "import kinds::{make}\n\nfn main() {\n    match make() {\n        Shape::Circle(r) => println(\"${r}\")\n        Shape::Empty => println(\"empty\")\n    }\n}\n",
            ),
            kinds,
        ],
        "E0001",
        "cannot resolve this pattern",
        "import `Shape` from `kinds`",
        "main.nova",
        "import kinds::{make, Shape}\n",
    );
}

#[test]
fn case_17_no_import_when_two_modules_export_the_name() {
    no_import(
        &[
            (
                MAIN,
                "import a::{x}\nimport b::{y}\n\nfn main() {\n    let s = x() + y() + g()\n}\n",
            ),
            ("mem/a.nova", "pub fn x() -> Int {\n    1\n}\n\npub fn g() -> Int {\n    2\n}\n"),
            ("mem/b.nova", "pub fn y() -> Int {\n    1\n}\n\npub fn g() -> Int {\n    3\n}\n"),
        ],
        "E0001",
        "cannot find function `g`",
    );
}

#[test]
fn case_18_no_import_that_binds_a_name_the_module_has_but_std_does_not_block() {
    let geo = (
        "mem/geo.nova",
        "pub record Area { v: Int }\n\npub fn Area() -> Int {\n    2\n}\n\npub record Map { v: Int }\n\npub fn Map() -> Int {\n    3\n}\n\npub fn one() -> Int {\n    1\n}\n",
    );
    no_import(
        &[
            (
                MAIN,
                "import geo::{one}\n\nrecord Area { w: Int }\n\nfn main() {\n    let n = Area() + one()\n}\n",
            ),
            geo,
        ],
        "E0001",
        "cannot find function `Area`",
    );
    // `Map` is also std's type, which an import wins over (spec §4.2).
    offers(
        &[
            (MAIN, "import geo::{one}\n\nfn main() {\n    let n = Map() + one()\n}\n"),
            geo,
        ],
        "E0001",
        "cannot find function `Map`",
        "import `Map` from `geo`",
        "main.nova",
        "import geo::{one, Map}\n",
    );
}

#[test]
fn case_19_no_import_of_a_sum_type_for_a_record_literal() {
    no_import(
        &[
            (MAIN, "import geo::{one}\n\nfn main() {\n    let p = Point { x: 1 }\n}\n"),
            (
                "mem/geo.nova",
                "pub type Point =\n  | Origin\n\npub fn one() -> Int {\n    1\n}\n",
            ),
        ],
        "E0001",
        "cannot find record `Point`",
    );
}

#[test]
fn case_20_did_you_mean_a_local() {
    offers(
        &[(
            MAIN,
            "fn main() {\n    let count = 1\n    let total = cuont + 1\n    println(\"${total}\")\n}\n",
        )],
        "E0001",
        "cannot find `cuont`",
        "change `cuont` to `count`",
        "main.nova",
        "let total = count + 1",
    );
}

#[test]
fn case_21_did_you_mean_a_type_and_a_qualifier() {
    offers(
        &[(
            MAIN,
            "record Point { x: Int }\n\nfn show(p: Piont) -> Int {\n    p.x\n}\n\nfn main() {}\n",
        )],
        "E0001",
        "cannot find type `Piont`",
        "change `Piont` to `Point`",
        "main.nova",
        "fn show(p: Point)",
    );
    offers(
        &[(
            MAIN,
            "type Shape =\n  | Empty\n\nfn main() {\n    let s = Shpae::Empty\n}\n",
        )],
        "E0900",
        "module-qualified paths",
        "change `Shpae` to `Shape`",
        "main.nova",
        "let s = Shape::Empty",
    );
}

#[test]
fn case_22_did_you_mean_a_field_in_a_read_and_a_literal() {
    offers(
        &[(
            MAIN,
            "record P { width: Int }\n\nfn main() {\n    let p = P { widht: 1 }\n}\n",
        )],
        "E0014",
        "has no field `widht`",
        "change `widht` to `width`",
        "main.nova",
        "P { width: 1 }",
    );
    offers(
        &[(
            MAIN,
            "record P { width: Int }\n\nfn main() {\n    let p = P { width: 1 }\n    let w = p.widht + 1\n}\n",
        )],
        "E0014",
        "no field `widht` on record `P`",
        "change `widht` to `width`",
        "main.nova",
        "let w = p.width + 1",
    );
}

#[test]
fn case_23_did_you_mean_a_method_one_from_a_trait_not_imported() {
    offers(
        &[(
            MAIN,
            "record P { v: Int }\n\nimpl P {\n    fn double(self) -> Int {\n        self.v * 2\n    }\n}\n\nfn main() {\n    let p = P { v: 1 }\n    let d = p.doubel()\n}\n",
        )],
        "E0014",
        "no method `doubel`",
        "change `doubel` to `double`",
        "main.nova",
        "let d = p.double()",
    );
    // Method calls resolve through every impl, imported or not (spec §4.3).
    offers(
        &[
            (MAIN, "import loud::{make}\n\nfn main() {\n    let s = make().shuot()\n}\n"),
            (
                "mem/loud.nova",
                "pub trait Loud {\n    fn shout(self) -> String\n}\n\npub record T { s: String }\n\nimpl Loud for T {\n    fn shout(self) -> String {\n        self.s\n    }\n}\n\npub fn make() -> T {\n    T { s: \"hi\" }\n}\n",
            ),
        ],
        "E0014",
        "no method `shuot`",
        "change `shuot` to `shout`",
        "main.nova",
        "let s = make().shout()",
    );
}

#[test]
fn case_25_did_you_mean_a_name_equal_ignoring_case_beyond_the_distance() {
    offers(
        &[(
            MAIN,
            "fn main() {\n    let httpclient = 1\n    let x = HTTPCLIENT + 1\n}\n",
        )],
        "E0001",
        "cannot find `HTTPCLIENT`",
        "change `HTTPCLIENT` to `httpclient`",
        "main.nova",
        "let x = httpclient + 1",
    );
}

#[test]
fn case_26_no_did_you_mean_beyond_the_distance_and_its_edge() {
    let a = loose(&buffers(&[(
        MAIN,
        "fn main() {\n    let abcde = 1\n    let x = abxde + axxde\n}\n",
    )]));
    let near = diagnostic(&a, "E0001", "cannot find `abxde`");
    fix(near, "change `abxde` to `abcde`");
    let far = diagnostic(&a, "E0001", "cannot find `axxde`");
    assert!(far.fixes.is_empty(), "{:?}", far.fixes);
}
```

- [ ] **Step 2: Run the tests to see them fail**

Run: `cd /d/Projects/nona/nova && cargo test -p nova-driver --test fixes case_ 2>&1 | grep -E "^test .*(FAILED|ok)$" | sort | uniq -c | head -40`
Expected: cases 11-16, 18, 20-23 and 25-26 FAIL with "no fix …"; cases 17
and 19 pass (there is no fix of any kind yet), and so do cases 1-10.

- [ ] **Step 3: The import and "did you mean" fixes**

In `crates/nova-typeck/src/check/fixes.rs`:
1. Replace its `use` lines (`use nova_diagnostics::{Edit, Fix, Span};` and
   `use super::Checker;`) with:

```rust
use nova_ast as ast;
use nova_ast::item::ImportKind;
use nova_diagnostics::{lines, suggest, Edit, FileId, Fix, Span, Spanned};
use nova_hir::{Ty, TyHead};
use nova_resolver::{DefId, DefKind, Exported, ModuleId, Res, ScopeEntry};

use super::{Checker, FnCtx, MethodRes};
```

2. After `enum Binding`'s closing brace, insert (marker `T5-NEED`):

```rust

/// What an unknown name must be at its site (spec §4.2's table; plan
/// decision 6).
#[derive(Clone, Debug)]
pub(super) enum Need {
    /// A value: a function, a constant, or a variant without fields.
    Value,
    /// A call's callee, with the call's argument count.
    Call(usize),
    /// A type: a record or a sum type.
    Type,
    /// A record literal's record.
    Record,
    /// A trait.
    Trait,
    /// `T` in `T::member`; as a call's callee, `call` holds its argument
    /// count.
    Qualifier { member: String, call: Option<usize> },
    /// `T` in the pattern `T::variant` with `arity` fields; `scrutinee` is
    /// the matched value's sum type, when it is known.
    Pattern {
        variant: String,
        arity: usize,
        scrutinee: Option<DefId>,
    },
}

/// The primitive types' names, which "did you mean" offers for a type.
const PRIMITIVES: [&str; 7] = ["Bool", "Bytes", "Char", "Float", "Future", "Int", "String"];

/// The locals `fcx` can see; never the checker's own (`__it` and its kind).
pub(super) fn visible_locals(fcx: &FnCtx) -> Vec<String> {
    let mut names: Vec<String> = fcx
        .scopes
        .iter()
        .flat_map(|scope| scope.keys().cloned())
        .filter(|n| !n.starts_with("__"))
        .collect();
    names.sort();
    names.dedup();
    names
}

/// The sum type of a matched value, when inference knows it.
pub(super) fn scrutinee_sum(fcx: &FnCtx, ty: &Ty) -> Option<DefId> {
    match fcx.icx.apply(ty) {
        Ty::Sum { def_id, .. } => Some(def_id),
        _ => None,
    }
}
```

3. Append to the `impl<'a> Checker<'a>` block, before its closing brace
   (marker `T5-METHODS`):

```rust

    /// The import and "did you mean" fixes for the unknown `name`, added to
    /// the diagnostic just pushed (spec §4.2, §4.3). `nearby` holds other
    /// names the site could mean: the visible locals, or the type
    /// parameters in scope.
    pub(super) fn name_fixes(&mut self, name: Spanned<String>, need: Need, nearby: Vec<String>) {
        let import = self.import_fix(&name, &need);
        let change = self.did_you_mean(&name, &need, &nearby);
        self.push_fix(import);
        self.push_fix(change);
    }

    /// "import `x` from `m`" (spec §4.2): the one module this one could
    /// import that exports an `x` of the kind `need` names, when importing
    /// it binds `x` nowhere this module already has it.
    fn import_fix(&self, name: &Spanned<String>, need: &Need) -> Option<Fix> {
        let x = name.value.as_str();
        let here = self.cur_module;
        let found: Vec<(&str, ModuleId)> = self
            .defs
            .importable(here)
            .iter()
            .filter(|(_, m)| self.fits(&self.defs.exported(*m, x), need))
            .map(|(import, m)| (import.as_str(), *m))
            .collect();
        let [(import, module)] = found.as_slice() else {
            return None;
        };
        let exported = self.defs.exported(*module, x);
        let bound = self.defs.bound_outside_std(here, x);
        let clash = (exported.value.is_some() && bound.value.is_some())
            || (exported.ty.is_some() && bound.ty.is_some())
            || (exported.trait_def.is_some() && bound.trait_def.is_some());
        if clash {
            return None;
        }
        let edit = self.import_edit(name.span.file, import, x)?;
        Some(Fix::new(format!("import `{x}` from `{import}`"), vec![edit]))
    }

    /// Where `x` goes in `file`, this module's (spec §4.2, §4.7): into an
    /// `import m::{…}`, or a new line after the last import, or above the
    /// first item.
    fn import_edit(&self, file: FileId, import: &str, x: &str) -> Option<Edit> {
        let here = self.cur_module;
        let items: Vec<&ast::Item> = self
            .file
            .items
            .iter()
            .enumerate()
            .filter(|(i, _)| self.defs.module_of(*i) == here)
            .map(|(_, item)| &item.value)
            .collect();
        let spans: Vec<Span> = self
            .file
            .items
            .iter()
            .enumerate()
            .filter(|(i, _)| self.defs.module_of(*i) == here)
            .map(|(_, item)| item.span)
            .collect();
        let list = items.iter().rev().find_map(|item| match item {
            ast::Item::Import(imp) => match (&imp.kind, imp.path.value.segments.as_slice()) {
                (ImportKind::List(names), [seg]) if seg.value == import => names.last(),
                _ => None,
            },
            _ => None,
        });
        if let Some(last) = list {
            return Some(Edit::insert(last.span.end, file, format!(", {x}")));
        }
        let text = self.sources?.get_source(file)?;
        let line = format!("import {import}::{{{x}}}");
        let last_import = items
            .iter()
            .zip(&spans)
            .filter(|(item, _)| matches!(item, ast::Item::Import(_)))
            .map(|(_, span)| *span)
            .last();
        let (at, inserted) = match (last_import, spans.first()) {
            (Some(last), _) => lines::line_after(text, last.end as usize, &line),
            (None, Some(first)) => lines::line_before_item(text, first.start as usize, &line),
            (None, None) => (text.len(), format!("{line}{}", lines::ending(text))),
        };
        Some(Edit::insert(at as u32, file, inserted))
    }

    /// Whether `exported` holds an item of the kind `need` names (spec
    /// §4.2's table; plan decision 6).
    fn fits(&self, exported: &Exported, need: &Need) -> bool {
        let kind = |id: DefId| &self.defs.def(id).kind;
        match need {
            Need::Value => match exported.value {
                Some(Res::Def(id)) => matches!(kind(id), DefKind::Fn { .. } | DefKind::Const { .. }),
                Some(Res::Variant(sum, vi)) => self.variant_arity(sum, vi) == Some(0),
                _ => false,
            },
            Need::Call(n) => match exported.value {
                Some(Res::Def(id)) => self.arity(id) == Some(*n),
                Some(Res::Variant(sum, vi)) => self.variant_arity(sum, vi) == Some(*n),
                _ => false,
            },
            Need::Type => exported
                .ty
                .is_some_and(|id| matches!(kind(id), DefKind::Record { .. } | DefKind::Sum { .. })),
            Need::Record => exported
                .ty
                .is_some_and(|id| matches!(kind(id), DefKind::Record { .. })),
            Need::Trait => exported.trait_def.is_some(),
            Need::Qualifier { member, call } => exported
                .ty
                .is_some_and(|id| self.has_member(id, member, *call)),
            Need::Pattern {
                variant,
                arity,
                scrutinee,
            } => exported.ty.is_some_and(|id| {
                scrutinee.map_or(true, |s| s == id)
                    && self
                        .variant_named(id, variant)
                        .is_some_and(|(_, a)| a == *arity)
            }),
        }
    }

    /// A function's or an extern function's parameter count.
    fn arity(&self, id: DefId) -> Option<usize> {
        match self.defs.def(id).kind {
            DefKind::Fn { .. } => self.sigs.get(&id).map(|s| s.params.len()),
            DefKind::ExternFn { .. } => self
                .externs
                .iter()
                .find(|e| e.def_id == id)
                .map(|e| e.params.len()),
            _ => None,
        }
    }

    /// The field count of variant `vi` of sum type `sum`.
    fn variant_arity(&self, sum: DefId, vi: usize) -> Option<usize> {
        match &self.defs.def(sum).kind {
            DefKind::Sum { variants, .. } => variants.get(vi).map(|v| v.arity),
            _ => None,
        }
    }

    /// Sum type `sum`'s variant `name`: its index and field count.
    fn variant_named(&self, sum: DefId, name: &str) -> Option<(usize, usize)> {
        match &self.defs.def(sum).kind {
            DefKind::Sum { variants, .. } => variants
                .iter()
                .position(|v| v.name == name)
                .map(|i| (i, variants[i].arity)),
            _ => None,
        }
    }

    /// Whether type `id` has `member` for `T::member`: a variant without
    /// fields, or, as a call's callee with `call` arguments, a variant with
    /// that many or an inherent associated function taking them.
    fn has_member(&self, id: DefId, member: &str, call: Option<usize>) -> bool {
        let head = match self.defs.def(id).kind {
            DefKind::Record { .. } => TyHead::Record(id),
            DefKind::Sum { .. } => TyHead::Sum(id),
            _ => return false,
        };
        if let Some((_, arity)) = self.variant_named(id, member) {
            return arity == call.unwrap_or(0);
        }
        let Some(n) = call else {
            return false;
        };
        matches!(
            self.find_assoc_fns(head, member).as_slice(),
            [f] if self.sigs.get(f).is_some_and(|s| s.params.len() == n)
        )
    }

    /// "change `x` to `y`" (spec §4.3), `y` a name of the kind `need` says,
    /// in scope here or in `nearby`.
    fn did_you_mean(&self, name: &Spanned<String>, need: &Need, nearby: &[String]) -> Option<Fix> {
        let mut candidates: Vec<String> = nearby.to_vec();
        for (n, entry) in self.defs.names_in_scope(self.cur_module) {
            if self.offers(need, entry) {
                candidates.push(n);
            }
        }
        if matches!(need, Need::Type | Need::Qualifier { .. }) {
            candidates.extend(PRIMITIVES.iter().map(|p| p.to_string()));
        }
        suggest::change_to(name, candidates.iter().map(String::as_str))
    }

    /// Whether "did you mean" offers a name bound to `entry` where `need`
    /// holds (spec §4.3).
    fn offers(&self, need: &Need, entry: ScopeEntry) -> bool {
        let kind = |id: DefId| &self.defs.def(id).kind;
        match (need, entry) {
            (Need::Value, ScopeEntry::Value(Res::Def(id))) => {
                matches!(kind(id), DefKind::Fn { .. } | DefKind::Const { .. })
            }
            (Need::Value, ScopeEntry::Value(Res::Variant(sum, vi))) => {
                self.variant_arity(sum, vi) == Some(0)
            }
            (Need::Call(_), ScopeEntry::Value(Res::Def(id))) => {
                matches!(kind(id), DefKind::Fn { .. } | DefKind::ExternFn { .. })
            }
            (Need::Call(_), ScopeEntry::Value(Res::Variant(..) | Res::Builtin(_))) => true,
            (Need::Type | Need::Qualifier { .. }, ScopeEntry::Type(_)) => true,
            (Need::Record, ScopeEntry::Type(id)) => matches!(kind(id), DefKind::Record { .. }),
            (Need::Pattern { .. }, ScopeEntry::Type(id)) => matches!(kind(id), DefKind::Sum { .. }),
            (Need::Trait, ScopeEntry::Trait(_)) => true,
            _ => false,
        }
    }

    /// "change `f` to `g`" among the fields of `ty`, a record (spec §4.3).
    pub(super) fn field_fix(&self, ty: &Ty, field: &Spanned<String>) -> Option<Fix> {
        let Ty::Record { def_id, .. } = ty else {
            return None;
        };
        let record = self.records.iter().find(|r| r.def_id == *def_id)?;
        suggest::change_to(field, record.fields.iter().map(|f| f.name.as_str()))
    }

    /// "change `m` to `n`" among the methods a call on `recv` resolves:
    /// its inherent methods and those of every trait implemented for it,
    /// since resolution ignores imports (spec §4.3).
    pub(super) fn method_fix(&self, recv: &Ty, fcx: &FnCtx, method: &Spanned<String>) -> Option<Fix> {
        let trait_methods = |tid: DefId| {
            self.traits
                .iter()
                .filter(move |t| t.def_id == tid)
                .flat_map(|t| t.methods.iter().map(|m| m.name.clone()))
        };
        let mut names: Vec<String> = Vec::new();
        match recv {
            Ty::Param(k) => {
                for &tid in fcx.param_bounds.get(*k as usize).into_iter().flatten() {
                    names.extend(trait_methods(tid));
                }
            }
            _ => {
                if let Some(head) = recv.head() {
                    for imp in self.impls.iter().filter(|i| i.self_head == head) {
                        names.extend(imp.methods.iter().map(|(n, _)| n.clone()));
                        if let Some(tid) = imp.trait_id {
                            names.extend(trait_methods(tid));
                        }
                    }
                }
            }
        }
        names.sort();
        names.dedup();
        names.retain(|n| {
            matches!(
                self.resolve_method_on(recv, fcx, n),
                MethodRes::Inherent(_) | MethodRes::Trait(..)
            )
        });
        suggest::change_to(method, names.iter().map(String::as_str))
    }
```

- [ ] **Step 4: The checker's state, and the sites**

In `crates/nova-typeck/src/check.rs`:
1. Replace `use nova_diagnostics::{Diagnostic, FileDb, Span, Spanned};` with
   `use nova_diagnostics::{suggest, Diagnostic, FileDb, Span, Spanned};`, and
   `use fixes::Binding;` with `use fixes::{Binding, Need};`.
2. In `struct Checker`, after `    bindings: FxHashMap<Span, Binding>,`
   insert:

```rust
    /// Set by `check_call` to its argument count just before it checks a
    /// two-segment callee path; taken by `check_path` (plan decision 7).
    qualified_callee: Option<usize>,
```

   and in `check_with`, after `        bindings: FxHashMap::default(),`
   insert `        qualified_callee: None,`.
3. At each site, insert the new lines right after the old text, each old
   text occurring once. Indent the new lines as the old text's first line:

| Site | Old text (its last line) | Insert after it |
|---|---|---|
| trait, `collect_supertraits` | `self.error("E0001", format!("cannot find trait \`{name}\`"), path.span);` | `if let Some(seg) = path.value.segments.last() {` / `    self.name_fixes(seg.clone(), Need::Trait, Vec::new());` / `}` |
| trait, an impl's | `self.error("E0001", format!("cannot find trait \`{name}\`"), tr.span);` | the same, with `tr` for `path` |
| trait, a bound | `self.error("E0001", format!("cannot find trait \`{name}\`"), b.span);` | the same, with `b` |
| trait, a `where` bound | `None => self.error("E0001", format!("cannot find trait \`{name}\`"), b.span),` | replace the arm: `None => {` / `    self.error("E0001", format!("cannot find trait \`{name}\`"), b.span);` / `    if let Some(seg) = b.value.segments.last() {` / `        self.name_fixes(seg.clone(), Need::Trait, Vec::new());` / `    }` / `}` |
| type | `self.error("E0001", format!("cannot find type \`{name}\`"), ty.span);` | `self.name_fixes(path.segments[0].clone(), Need::Type, generics.keys().cloned().collect());` |
| value | `self.error("E0001", format!("cannot find \`{name}\` in this scope"), span);` | `let nearby = fixes::visible_locals(fcx);` / `self.name_fixes(path.segments[0].clone(), Need::Value, nearby);` |
| call | `                                format!("cannot find function \`{name}\` in this scope"),` / `                                callee.span,` / `                            );` | `let nearby = fixes::visible_locals(fcx);` / `self.name_fixes(path.segments[0].clone(), Need::Call(args.len()), nearby);` |
| record | `self.error("E0001", format!("cannot find record \`{name}\`"), span);` | `self.name_fixes(path.segments[0].clone(), Need::Record, Vec::new());` |
| assignment's target | `                format!("cannot find \`{name}\` in this scope"),` / `                lhs.span,` / `            );` | `let nearby = fixes::visible_locals(fcx);` / `self.push_fix(suggest::change_to(&path.segments[0], nearby.iter().map(String::as_str)));` |
| field, literal | `                    format!("record \`{name}\` has no field \`{fname}\`"),` / `                    init.name.span,` / `                );` | `let given: Vec<&str> = fields.iter().map(\|f\| f.name.value.as_str()).collect();` / `let fix = suggest::change_to(&init.name, record.fields.iter().map(\|f\| f.name.as_str()).filter(\|n\| !given.contains(n)));` / `self.push_fix(fix);` |
| field, read | `            self.no_field_message(fcx, &recv_ty, &field.value),` / `            field.span,` / `        );` / `        error_expr(span)` | before `error_expr(span)`: `let fix = self.field_fix(&recv_ty, field);` / `self.push_fix(fix);` |
| field, write | `                self.no_field_message(fcx, &recv_ty, &field.value),` / `                field.span,` / `            );` / `            self.check_expr(fcx, rhs);` | before `self.check_expr(fcx, rhs);`: `let fix = self.field_fix(&recv_ty, field);` / `self.push_fix(fix);` |
| method, array | `                    "no method \`{}\` on array type \`{}\`",` … `                method.span,` / `            );` | `self.push_fix(suggest::change_to(method, ["len"]));` |
| method | `                        "no method \`{}\` on type \`{}\`",` … `                    method.span,` / `                );` | `let fix = self.method_fix(&recv_ty, fcx, method);` / `self.push_fix(fix);` |
| pattern, a path | `format!("\`{ty_name}::{v_name}\` is not a variant of the matched type"),` / `                    pattern.span,` / `                );` | the block below, marker `T5-PATTERN-PATH` |
| pattern, a tuple struct | `"cannot resolve this pattern to a sum type variant",` / `                        pattern.span,` / `                    );` | the block below, marker `T5-PATTERN-TUPLE` |

The two pattern blocks:

```rust
                // Spec 3.4b §4.2: a qualifier this module never imported.
                if self.defs.resolve_type(self.cur_module, ty_name).is_none() {
                    let scrutinee = fixes::scrutinee_sum(fcx, scrut_ty);
                    self.name_fixes(
                        path.segments[0].clone(),
                        Need::Pattern {
                            variant: v_name.to_string(),
                            arity: 0,
                            scrutinee,
                        },
                        Vec::new(),
                    );
                }
```

(marker `T5-PATTERN-PATH`), and (marker `T5-PATTERN-TUPLE`):

```rust
                    // Spec 3.4b §4.2: a qualifier this module never imported.
                    if path.segments.len() == 2
                        && self
                            .defs
                            .resolve_type(self.cur_module, &path.segments[0].value)
                            .is_none()
                    {
                        let scrutinee = fixes::scrutinee_sum(fcx, scrut_ty);
                        self.name_fixes(
                            path.segments[0].clone(),
                            Need::Pattern {
                                variant: path.segments[1].value.clone(),
                                arity: fields.len(),
                                scrutinee,
                            },
                            Vec::new(),
                        );
                    }
```

4. The qualifier (plan decision 7):
   - In `check_path`, replace
     `    fn check_path(&mut self, fcx: &mut FnCtx, path: &ast::Path, span: Span) -> hir::Expr {\n        if path.segments.len() == 2 {`
     with the same two lines and, between them,
     `        let callee_args = self.qualified_callee.take();`.
   - Replace
     `            self.unsupported(span, "module-qualified paths");\n            return error_expr(span);\n        }\n        if path.segments.len() != 1 {`
     with (marker `T5-QUALIFIER`):

```rust
            self.unsupported(span, "module-qualified paths");
            // Spec 3.4b §4.2, §4.3: a qualifier this module never imported,
            // or misspelled.
            self.name_fixes(
                path.segments[0].clone(),
                Need::Qualifier {
                    member: v_name.to_string(),
                    call: callee_args,
                },
                fcx.generics.keys().cloned().collect(),
            );
            return error_expr(span);
        }
        if path.segments.len() != 1 {
```

   - In `check_call`, replace
     `        let callee_expr = self.check_expr(fcx, callee);\n        let checked: Vec<hir::Expr> = args.iter().map(|a| self.check_expr(fcx, a)).collect();\n        let ret = fcx.icx.fresh();`
     with (marker `T5-CALLEE`):

```rust
        // Plan decision 7: a two-segment callee's E0900 knows it is a
        // call's, and its argument count.
        if let ast::Expr::Path(path) = &callee.value {
            if path.segments.len() == 2 {
                self.qualified_callee = Some(args.len());
            }
        }
        let callee_expr = self.check_expr(fcx, callee);
        // Taken by `check_path`; cleared here too, so it reaches no other
        // path.
        self.qualified_callee = None;
        let checked: Vec<hir::Expr> = args.iter().map(|a| self.check_expr(fcx, a)).collect();
        let ret = fcx.icx.fresh();
```

Run `cargo fmt --all` after the edits.

- [ ] **Step 5: Run the tests**

Run: `cd /d/Projects/nona/nova && cargo test -p nova-driver --test fixes 2>&1 | tail -3`
Expected: `test result: ok. 26 passed; 0 failed`.

Run: `cd /d/Projects/nona/nova && cargo test -p nova-typeck 2>&1 | grep -E "^test result" | grep -v " 0 failed"; cargo test -p nova-driver 2>&1 | grep -E "^test result" | grep -v " 0 failed"; echo done`
Expected: `done` alone.

Run: `cd /d/Projects/nona/nova && cargo clippy -p nova-typeck -p nova-resolver -p nova-diagnostics --all-targets -- -D warnings 2>&1 | tail -3`
Expected: `Finished`, no warning.

- [ ] **Step 6: Commit**

Write `$P/msg-34b-5.txt`:

```text
nova-typeck: import a name, and did you mean

An unknown name, type, record, trait or qualifier offers "import x from
m" when exactly one importable module exports an x of the kind the site
needs and importing it binds nothing the module has; and "change x to
y" for the closest name in scope. Unknown fields and methods offer the
closest field or callable method (spec 3.4b §4.2, §4.3).

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
```

Run: `cd /d/Projects/nona/nova && cargo fmt --all && git add -u && git commit -q -F $P/msg-34b-5.txt && git log -1 --format=%s`
Expected: `nova-typeck: import a name, and did you mean`

---

### Task 6: Make public, the attributes, and an unreachable arm

**Files:**
- Modify: `crates/nova-resolver/src/lib.rs` (imports; `resolve_import`
  and its call; a new `make_public`; `validate_test_function`)
- Modify: `crates/nova-typeck/src/check.rs` (`check_match` and
  `check_match_usefulness`), `crates/nova-typeck/src/check/fixes.rs`
- Test: `crates/nova-driver/tests/fixes.rs`

**Interfaces:**
- Consumes: `lines::{keyword_start, arm_removal}`, `suggest::change_to`
  (Task 2); `ModuleSource::{text, package}` (Task 3); Task 5's `offers`
  test helper.
- Produces: `fn make_public(module: &ModuleSource, name: &str, import: &str, same_package: bool) -> Option<Fix>`
  in the resolver, and `Checker::remove_arm_fix(&self, arm: Span) -> Option<Fix>`.

- [ ] **Step 1: Write the failing tests**

Append to `crates/nova-driver/tests/fixes.rs` (marker `T6-TESTS`):

```rust

// === Task 6: make public, the attributes, an unreachable arm (spec §4.3,
// §4.4, §4.5; §9.1 cases 24 and 27-30) ===

#[test]
fn case_24_did_you_mean_an_attribute_and_a_test_argument() {
    offers(
        &[(MAIN, "@tset\nfn t() {}\n\nfn main() {}\n")],
        "E0082",
        "unknown attribute `@tset`",
        "change `tset` to `test`",
        "main.nova",
        "@test\nfn t()",
    );
    offers(
        &[(MAIN, "@test(should_panik)\nfn t() {\n    panic(\"x\")\n}\n\nfn main() {}\n")],
        "E0085",
        "unknown `@test` argument `should_panik`",
        "change `should_panik` to `should_panic`",
        "main.nova",
        "@test(should_panic)",
    );
}

#[test]
fn case_27_make_public_edits_the_sibling_module() {
    for nl in ["\n", "\r\n"] {
        let main = "import lib::{hidden}\n\nfn main() {\n    println(\"${hidden()}\")\n}\n"
            .replace('\n', nl);
        let lib = "/// The secret.\nfn hidden() -> Int {\n    1\n}\n".replace('\n', nl);
        offers(
            &[(MAIN, main.as_str()), ("mem/lib.nova", lib.as_str())],
            "E0001",
            "`hidden` is not a public item of module `lib`",
            "make `hidden` public in `lib`",
            "lib.nova",
            &format!("/// The secret.{nl}pub fn hidden()"),
        );
    }
}

#[test]
fn case_28_make_public_from_tests_edits_the_packages_library() {
    for (k, nl) in ["\n", "\r\n"].into_iter().enumerate() {
        let lib = "fn name() -> String {\n    \"app\"\n}\n".replace('\n', nl);
        let test = "import demo::{name}\n\n@test\nfn t() {\n    assert_eq(name(), \"app\")\n}\n"
            .replace('\n', nl);
        let dir = project(
            &format!("public-from-tests-{k}"),
            &[("src/lib.nova", lib.as_str()), ("tests/t.nova", test.as_str())],
        );
        let none = Buffers(Vec::new());
        let a = whole(&dir, &none);
        let d = diagnostic(&a, "E0001", "`name` is not a public item of module `demo`");
        let edited = apply(&a, fix(d, "make `name` public in `demo`"), &none);
        let after = text(&edited, "lib.nova");
        assert!(after.starts_with("pub fn name()"), "{after}");
        assert_fixes(&a, &whole(&dir, &edited), "E0001", &[]);
    }
}

#[test]
fn case_29_no_make_public_for_a_dependencys_item() {
    let app = app_and_geom(
        "public-dependency",
        &[("src/main.nova", "import geom::{secret}\n\nfn main() {}\n")],
        "fn secret() -> Int {\n    1\n}\n\npub fn area() -> Int {\n    1\n}\n",
    );
    let a = whole(&app, &Buffers(Vec::new()));
    let d = diagnostic(&a, "E0001", "`secret` is not a public item of module `geom`");
    assert!(d.fixes.is_empty(), "{:?}", d.fixes);
}

#[test]
fn case_30_remove_an_unreachable_arm() {
    for nl in ["\n", "\r\n"] {
        let after_any = format!("        _ => \"any\"{nl}    }}");
        // On its own line.
        let src = "fn main() {\n    let x = 1\n    let s = match x {\n        _ => \"any\"\n        1 => \"one\"\n    }\n    println(s)\n}\n"
            .replace('\n', nl);
        offers(
            &[(MAIN, src.as_str())],
            "E0021",
            "unreachable match arm",
            "remove the unreachable arm",
            "main.nova",
            &after_any,
        );
        // Over several lines.
        let src = "fn main() {\n    let x = 1\n    let s = match x {\n        _ => \"any\"\n        1 => {\n            \"one\"\n        }\n    }\n    println(s)\n}\n"
            .replace('\n', nl);
        offers(
            &[(MAIN, src.as_str())],
            "E0021",
            "unreachable match arm",
            "remove the unreachable arm",
            "main.nova",
            &after_any,
        );
    }
    // Sharing its line, with a trailing comma.
    offers(
        &[(
            MAIN,
            "fn main() {\n    let x = 1\n    let s = match x { _ => \"any\", 1 => \"one\", }\n    println(s)\n}\n",
        )],
        "E0021",
        "unreachable match arm",
        "remove the unreachable arm",
        "main.nova",
        "match x { _ => \"any\", }",
    );
}
```

- [ ] **Step 2: Run the tests to see them fail**

Run: `cd /d/Projects/nona/nova && cargo test -p nova-driver --test fixes 2>&1 | grep -E "^test case_(24|2[7-9]|30)" `
Expected: cases 24, 27, 28 and 30 FAILED ("no fix …"); case 29 ok.

- [ ] **Step 3: The resolver's fixes**

In `crates/nova-resolver/src/lib.rs`:
1. Replace `use nova_diagnostics::{Diagnostic, FileId, Span};` with
   `use nova_diagnostics::{suggest, Diagnostic, Edit, FileId, Fix, Span};`.
2. In `resolve_import`'s parameters, replace
   `    imports: &std::collections::HashMap<String, ImportTarget>,` with
   `    modules: &[ModuleSource],`, and in its body replace
   `    let target = match imports.get(target_name) {` with
   `    let target = match modules[mid].imports.get(target_name) {`.
   In `resolve_program`'s pass 2, replace `                    &m.imports,`
   with `                    &all,`.
3. Replace (marker `T6-NOT-PUB-OLD`)

```rust
                if !found {
                    diagnostics.push(
                        Diagnostic::error(
                            "E0001",
                            format!("`{name}` is not a public item of module `{target_name}`"),
                        )
                        .with_primary_label(n.span, "not found or not `pub`"),
                    );
                }
```

with (marker `T6-NOT-PUB-NEW`)

```rust
                if !found {
                    let mut diag = Diagnostic::error(
                        "E0001",
                        format!("`{name}` is not a public item of module `{target_name}`"),
                    )
                    .with_primary_label(n.span, "not found or not `pub`");
                    let same_package = modules[mid].package == modules[target].package;
                    if let Some(fix) = make_public(&modules[target], name, target_name, same_package) {
                        diag = diag.with_fix(fix);
                    }
                    diagnostics.push(diag);
                }
```

4. After `fn is_pub`'s closing brace, insert (marker `T6-MAKE-PUBLIC`):

```rust

/// "make `x` public in `m`" (spec 3.4b §4.4): `pub ` before the keyword of
/// each private item `module` declares named `name`, when `module` is the
/// importer's own package's. The keyword is found in the module's text.
fn make_public(module: &ModuleSource, name: &str, import: &str, same_package: bool) -> Option<Fix> {
    if !same_package {
        return None;
    }
    let text = module.text?;
    let mut edits = Vec::new();
    for item in &module.file.items {
        let (vis, item_name) = match &item.value {
            Item::Function(f) => (f.vis, &f.name),
            Item::Record(r) => (r.vis, &r.name),
            Item::Type(t) => (t.vis, &t.name),
            Item::Trait(t) => (t.vis, &t.name),
            Item::Const(c) => (c.vis, &c.name),
            _ => continue,
        };
        if item_name.value != name || is_pub(vis) {
            continue;
        }
        let at = nova_diagnostics::lines::keyword_start(text, item_name.span.start as usize)?;
        edits.push(Edit::insert(at as u32, item_name.span.file, "pub "));
    }
    (!edits.is_empty()).then(|| Fix::new(format!("make `{name}` public in `{import}`"), edits))
}
```

5. In `validate_test_function`, replace
   `        if attr.name.value != "test" {\n            diagnostics.push(unknown_attribute(attr));\n            continue;\n        }`
   with (marker `T6-ATTRIBUTE`):

```rust
        if attr.name.value != "test" {
            // Spec 3.4b §4.3: on a function only, since `@test` on anything
            // else is E0083 (decision 38).
            let mut d = unknown_attribute(attr);
            if let Some(fix) = suggest::change_to(&attr.name, KNOWN_ATTRIBUTES.iter().copied()) {
                d = d.with_fix(fix);
            }
            diagnostics.push(d);
            continue;
        }
```

6. In the same function, replace (marker `T6-ARGUMENT-OLD`)

```rust
                diagnostics.push(
                    Diagnostic::error(
                        "E0085",
                        format!(
                            "unknown `@test` argument `{}`; the accepted arguments are: {}",
                            arg.value,
                            KNOWN_TEST_ARGS.join(", "),
                        ),
                    )
                    .with_primary_label(arg.span, "unknown argument"),
                );
```

with (marker `T6-ARGUMENT-NEW`)

```rust
                let mut d = Diagnostic::error(
                    "E0085",
                    format!(
                        "unknown `@test` argument `{}`; the accepted arguments are: {}",
                        arg.value,
                        KNOWN_TEST_ARGS.join(", "),
                    ),
                )
                .with_primary_label(arg.span, "unknown argument");
                if let Some(fix) = suggest::change_to(arg, KNOWN_TEST_ARGS.iter().copied()) {
                    d = d.with_fix(fix);
                }
                diagnostics.push(d);
```

- [ ] **Step 4: Remove an unreachable arm**

In `crates/nova-typeck/src/check.rs`:
1. Replace
   `        // Normalized (pattern, has-guard, span) per arm, for the exhaustiveness\n        // and reachability analysis after all arms are checked.\n        let mut arm_pats: Vec<(usefulness::Pat, bool, Span)> = Vec::new();`
   with
   `        // Normalized (pattern, has-guard, its pattern's span, the whole arm's\n        // span) per arm, for the exhaustiveness and reachability analysis\n        // after all arms are checked.\n        let mut arm_pats: Vec<(usefulness::Pat, bool, Span, Span)> = Vec::new();`.
2. Replace `            arm_pats.push((upat, guarded, arm.pattern.span));` with
   `            arm_pats.push((upat, guarded, arm.pattern.span, arm.pattern.span.merge(arm.body.span)));`.
3. In `check_match_usefulness`, replace
   `        arm_pats: &[(usefulness::Pat, bool, Span)],` with
   `        arm_pats: &[(usefulness::Pat, bool, Span, Span)],` and
   `        for (pat, guarded, arm_span) in arm_pats {` with
   `        for (pat, guarded, arm_span, arm) in arm_pats {`.
4. Replace (marker `T6-E0021-OLD`)

```rust
                self.diagnostics.push(
                    Diagnostic::warning("E0021", "unreachable match arm")
                        .with_primary_label(*arm_span, "this arm is never reached")
                        .with_note(
                            "an earlier arm already matches every value this one would".to_string(),
                        ),
                );
```

with (marker `T6-E0021-NEW`)

```rust
                let mut d = Diagnostic::warning("E0021", "unreachable match arm")
                    .with_primary_label(*arm_span, "this arm is never reached")
                    .with_note(
                        "an earlier arm already matches every value this one would".to_string(),
                    );
                // Spec 3.4b §4.5.
                if let Some(fix) = self.remove_arm_fix(*arm) {
                    d = d.with_fix(fix);
                }
                self.diagnostics.push(d);
```

5. In `crates/nova-typeck/src/check/fixes.rs`, append to the `impl` block
   (marker `T6-REMOVE-ARM`):

```rust

    /// "remove the unreachable arm" (spec §4.5), `arm` being its pattern
    /// through its body, placed by §4.7's rules.
    pub(super) fn remove_arm_fix(&self, arm: Span) -> Option<Fix> {
        let text = self.sources?.get_source(arm.file)?;
        let (start, end) = lines::arm_removal(text, arm.start as usize, arm.end as usize)?;
        Some(Fix::new(
            "remove the unreachable arm",
            vec![Edit::replace(Span::new(start as u32, end as u32, arm.file), "")],
        ))
    }
```

- [ ] **Step 5: Run the tests**

Run: `cd /d/Projects/nona/nova && cargo test -p nova-driver --test fixes 2>&1 | tail -3`
Expected: `test result: ok. 31 passed; 0 failed`.

Run: `cd /d/Projects/nona/nova && for c in nova-resolver nova-typeck nova-driver; do cargo test -p $c 2>&1 | grep -E "^test result" | grep -v " 0 failed"; done; echo done`
Expected: `done` alone.

- [ ] **Step 6: Commit**

Write `$P/msg-34b-6.txt`:

```text
Make public, the attributes, and an unreachable arm

The resolver offers "make x public in m" when m is the importer's own
package's, placing pub before the item's keyword, and "did you mean"
for an unknown attribute on a function and an unknown @test argument.
The checker offers to remove an unreachable arm, with its comma, and
its lines when nothing else shares them (spec 3.4b §4.3-§4.5).

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
```

Run: `cd /d/Projects/nona/nova && cargo fmt --all && git add -u && git commit -q -F $P/msg-34b-6.txt && git log -1 --format=%s`
Expected: `Make public, the attributes, and an unreachable arm`

---

### Task 7: The sweep over broken programs, and the command line

**Files:**
- Modify: `crates/nova-driver/tests/broken.rs`
- Modify: `crates/nova-cli/tests/run_tests.rs` (after
  `import_of_private_item_is_rejected`)

**Interfaces:**
- Consumes: `Fix::apply` (Task 1); every fix of Tasks 4-6.
- Produces: nothing new.

- [ ] **Step 1: Write the command line's tests**

Insert into `crates/nova-cli/tests/run_tests.rs` after
`import_of_private_item_is_rejected`'s closing brace (marker `T7-CLI`):

```rust

/// Spec 3.4b §3.3: `nova check` prints a fix's title as a help line, in
/// place of the note it replaces.
#[test]
fn check_prints_a_fix_as_a_help_line() {
    let dir = std::env::temp_dir().join("nova-check-help-mutable");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("temp dir");
    let main = dir.join("main.nova");
    std::fs::write(
        &main,
        "fn main() {\n    let x = 1\n    x = 2\n    println(\"${x}\")\n}\n",
    )
    .expect("write");
    let assert = nova().arg("check").arg(&main).assert().failure();
    let stderr = String::from_utf8_lossy(&assert.get_output().stderr).to_string();
    assert!(stderr.contains("= help: make `x` mutable"), "stderr: {stderr}");
    assert!(!stderr.contains("declare it as"), "the fix replaces the note: {stderr}");
}

/// Spec 3.4b §4.4: the resolver's fix, placed in another file.
#[test]
fn check_prints_make_public_as_a_help_line() {
    let dir = std::env::temp_dir().join("nova-check-help-public");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("temp dir");
    std::fs::write(dir.join("lib.nova"), "fn hidden() -> Int {\n    1\n}\n").expect("write lib");
    let main = dir.join("app.nova");
    std::fs::write(
        &main,
        "import lib::{hidden}\n\nfn main() {\n    println(\"${hidden()}\")\n}\n",
    )
    .expect("write main");
    let assert = nova().arg("check").arg(&main).assert().failure();
    let stderr = String::from_utf8_lossy(&assert.get_output().stderr).to_string();
    assert!(
        stderr.contains("= help: make `hidden` public in `lib`"),
        "stderr: {stderr}"
    );
}
```

Run: `cd /d/Projects/nona/nova && cargo test -p nova-cli --test run_tests check_prints 2>&1 | tail -3`
Expected: `test result: ok. 2 passed; 0 failed`. These are *guards*:
Tasks 1 and 4-6 already print the lines, so they pass on their first run.
Confirm they can fail: in `crates/nova-diagnostics/src/render.rs`, change
`.with_notes(notes(diag));` in `emit_all` back to
`.with_notes(diag.notes.clone());`, rerun, see both FAIL, then
`git checkout -- crates/nova-diagnostics/src/render.rs`.

- [ ] **Step 2: The sweep**

In `crates/nova-driver/tests/broken.rs`:
1. Replace the module doc's last sentence,
   `//! index on, must return within 10 s without panicking, and every\n//! occurrence it records must lie inside its file.`
   with
   `//! index on, must return within 10 s without panicking, and every\n//! occurrence it records must lie inside its file. Every fix it makes must\n//! apply, and its first three fixes, applied one at a time, must analyse\n//! again without panicking (spec 3.4b §9.2; plan decision 12).`.
2. After `fn spans_inside`'s closing brace, insert (marker `T7-FIXES-HOLD`):

```rust

/// A cut program with one fix applied: the fixed files' texts, then the
/// cut, then the disk.
struct Fixed<'c> {
    cut: &'c Cut,
    files: Vec<(PathBuf, String)>,
}

impl Sources for Fixed<'_> {
    fn read(&self, path: &Path) -> std::io::Result<String> {
        match self.files.iter().find(|(p, _)| p == path) {
            Some((_, text)) => Ok(text.clone()),
            None => self.cut.read(path),
        }
    }
}

/// Spec 3.4b §9.2: every fix's edits lie inside their file, on character
/// boundaries, and do not overlap; and the first three, each applied
/// alone, analyse again without panicking. How many fixes it checked.
fn fixes_hold(
    a: &nova_driver::Analysis,
    path: &Path,
    cut: &Cut,
    options: &Options,
) -> Result<usize, String> {
    let fixes: Vec<&nova_diagnostics::Fix> = a.diagnostics.iter().flat_map(|d| &d.fixes).collect();
    for (k, fix) in fixes.iter().enumerate() {
        let Some(edited) = fix.apply(&a.db) else {
            return Err(format!("a fix does not apply: {fix:?}"));
        };
        if k >= 3 {
            continue;
        }
        let files = edited
            .into_iter()
            .map(|(file, text)| (PathBuf::from(a.db.get_name(file).unwrap_or("")), text))
            .collect();
        let fixed = Fixed { cut, files };
        let again = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            analyze(path, &fixed, options)
        }));
        if again.is_err() {
            return Err(format!("panicked after the fix {:?}", fix.title));
        }
    }
    Ok(fixes.len())
}
```

3. Replace the thread's analysis, from
   `            let started = Instant::now();\n            let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {\n                analyze(&path, &sources, &options).map(|a| spans_inside(&a))\n            }));`
   through
   `            if done\n                .send((path, text.len(), outcome.is_ok(), spans_ok, took))\n                .is_err()`
   with (marker `T7-THREAD`):

```rust
            let started = Instant::now();
            let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                analyze(&path, &sources, &options)
            }));
            let took = started.elapsed();
            let (spans_ok, fixes) = match &outcome {
                Ok(Ok(a)) => (spans_inside(a), fixes_hold(a, &path, &sources, &options)),
                _ => (true, Ok(0)),
            };
            current.lock().unwrap()[t] = None;
            if done
                .send((path, text.len(), outcome.is_ok(), spans_ok, fixes, took))
                .is_err()
```

4. In the receiving loop, before `for _ in 0..total {` insert
   `    let mut checked = 0usize;`; replace
   `            Ok((path, len, ok, spans_ok, took)) => {` with
   `            Ok((path, len, ok, spans_ok, fixes, took)) => {`; and after the
   `} else if !spans_ok { … }` branch insert (marker `T7-REPORT`):

```rust
                } else if let Err(why) = &fixes {
                    failures.push(format!("{why}: {} cut at byte {len}", path.display()));
```

   so the chain reads `if !ok … else if !spans_ok … else if let Err(why)
   = &fixes … else if took > …`. After the chain, still inside the `Ok`
   arm, add `checked += fixes.unwrap_or(0);`.
5. Before the final `assert!(failures.is_empty(), …)`, insert
   (marker `T7-CHECKED`):

```rust
    // The sweep is not vacuous: the cut programs carry fixes.
    eprintln!("{checked} fixes checked");
    assert!(checked > 0, "no cut program had a fix");
```

- [ ] **Step 3: Run the sweep**

Run: `cd /d/Projects/nona/nova && cargo test -p nova-driver --test broken -- --nocapture 2>&1 | grep -E "fixes checked|test result"`
Expected: `N fixes checked` with N above 0, and `test result: ok. 1
passed; 0 failed`, in under five minutes. Ledger N. If a fix fails to
apply or a re-analysis panics, the report names the cut: that is a bug
in the fix it names, fixed in the task that made it, with a ruling.

Confirm the sweep has teeth: in `crates/nova-diagnostics/src/suggest.rs`'s
`change_to`, change `vec![Edit::replace(name.span, to)]` to
`vec![Edit::replace(Span::new(name.span.start, name.span.end + 100_000, name.span.file), to)]`
(and add `Span` to that file's `use crate::{…}`), rerun, and expect FAILED
with "a fix does not apply". If it passes, no cut program carried a "did
you mean": ledger that, and make the same change to `import_fix`'s list
edit in `check/fixes.rs` instead. Then `git checkout -- <the file>`.

- [ ] **Step 4: Commit**

Write `$P/msg-34b-7.txt`:

```text
Tests: every fix on broken programs, and the help lines

The cut-program sweep checks that every fix applies, inside its file,
on character boundaries and without overlapping, and re-analyses each
program's first three fixes (spec 3.4b §9.2). Two command-line tests
read the help lines nova check prints.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
```

Run: `cd /d/Projects/nona/nova && cargo fmt --all && git add -u && git commit -q -F $P/msg-34b-7.txt && git log -1 --format=%s`
Expected: `Tests: every fix on broken programs, and the help lines`

---

### Task 8: The formatter keeps blank-line import groups

**Files:**
- Modify: `crates/nova-fmt/src/print/mod.rs` (`Printer::file`, the
  `import_run` doc)
- Test: `crates/nova-fmt/tests/layout.rs`

**Interfaces:**
- Consumes: nothing new.
- Produces: the formatter's groups, which Task 9's block relies on.

- [ ] **Step 1: Write the failing tests**

In `crates/nova-fmt/tests/layout.rs`, replace

```rust
    // Blank lines between imports do not end their run, and do not survive
    // inside it (the 3.1 plan's decision 3).
    assert_formats("import b\n\n\nimport a\n", "import a\nimport b\n");
```

with (marker `T8-LAYOUT-OLD-CASE`)

```rust
    // A blank line between imports ends a group, and one survives between
    // groups (spec 3.4b §6.6, which replaces the 3.1 plan's decision 3).
    assert_formats("import b\n\n\nimport a\n", "import b\n\nimport a\n");
```

and append (marker `T8-LAYOUT-TESTS`):

```rust

#[test]
fn a_blank_line_keeps_two_groups_of_imports_each_sorted() {
    // Spec 3.4b §6.6: as gofmt does.
    assert_formats(
        "import geom\nimport app\n\nimport utils\nimport shapes\n\nfn main() {}\n",
        "import app\nimport geom\n\nimport shapes\nimport utils\n\nfn main() {}\n",
    );
    assert_stable("import app\nimport geom\n\nimport shapes\nimport utils\n\nfn main() {}\n");
}

#[test]
fn a_header_comment_stays_above_the_second_group() {
    assert_formats(
        "import a\n\n// The project's own.\n\nimport c\nimport b\n",
        "import a\n\n// The project's own.\n\nimport b\nimport c\n",
    );
}
```

- [ ] **Step 2: Run the tests to see them fail**

Run: `cd /d/Projects/nona/nova && cargo test -p nova-fmt --test layout import 2>&1 | grep -E "^test "`
Expected: `imports_are_sorted_within_a_run`,
`a_blank_line_keeps_two_groups_of_imports_each_sorted` and
`a_header_comment_stays_above_the_second_group` FAILED; the rest ok.

- [ ] **Step 3: Groups in the printer**

In `crates/nova-fmt/src/print/mod.rs`'s `Printer::file`, replace

```rust
            if matches!(items[i].value, Item::Import(_)) {
                let start = i;
                while i < items.len() && matches!(items[i].value, Item::Import(_)) {
                    i += 1;
                }
                self.import_run(&mut v, &items[start..i]);
```

with (marker `T8-FILE`)

```rust
            if matches!(items[i].value, Item::Import(_)) {
                // Spec 3.4b §6.6: a blank line between two imports ends a
                // group, as in gofmt, and each group is sorted on its own.
                let start = i;
                i += 1;
                while i < items.len()
                    && matches!(items[i].value, Item::Import(_))
                    && !self
                        .src
                        .blank_line_in(items[i - 1].span.end, items[i].span.start)
                {
                    i += 1;
                }
                self.import_run(&mut v, &items[start..i]);
```

and replace `import_run`'s first doc line,
`    /// A run of imports, sorted by path and printed one per line, with no\n    /// blank line inside the run.`
with
`    /// A group of imports, a run with no blank line inside it (spec 3.4b\n    /// §6.6), sorted by path and printed one per line.`.

The output check (`check.rs`) does not change (spec decision 39): a
group's sort is a sort within the run of consecutive imports it is part
of.

- [ ] **Step 4: Run the tests**

Run: `cd /d/Projects/nona/nova && cargo test -p nova-fmt 2>&1 | grep -E "^test result" | grep -v " 0 failed"; echo done`
Expected: `done` alone, the corpus test included: no `.nova` file in the
repository has a blank line between imports.

Run: `cd /d/Projects/nona/nova && cargo run -q -p nova-cli -- fmt --check std examples; echo "exit $?"`
Expected: `exit 0`.

- [ ] **Step 5: Commit**

Write `$P/msg-34b-8.txt`:

```text
nova-fmt: a blank line between imports keeps two groups

As gofmt does, a blank line inside a run of imports now ends a group,
and each group is sorted on its own, one blank line apart (spec 3.4b
§6.6). The output check is unchanged: a group's sort is a sort within
its run. No formatted file in the repository changes.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
```

Run: `cd /d/Projects/nona/nova && cargo fmt --all && git add -u && git commit -q -F $P/msg-34b-8.txt && git log -1 --format=%s`
Expected: `nova-fmt: a blank line between imports keeps two groups`

---

### Task 9: Organize imports' block

**Files:**
- Create: `crates/nova-fmt/src/organize.rs`, `crates/nova-fmt/tests/organize.rs`
- Modify: `crates/nova-fmt/src/lib.rs` (module list, re-exports)

**Interfaces:**
- Consumes: `Source` (this crate); `lines` (Task 2); Task 8's groups.
- Produces (re-exported from `nova_fmt`):
  - `pub struct ImportView<'a> { pub path: String, pub path_start: u32, pub first: &'a str, pub glob: bool, pub names: Vec<(&'a str, u32)> }`;
  - `pub enum Group { Dependency, Module }`, ordered, dependencies first;
  - `pub struct Verdict { pub group: Group, pub unused_glob: bool, pub unused_names: Vec<String> }`;
  - `pub struct TextEdit { pub start: u32, pub end: u32, pub text: String }`;
  - `pub fn organize(text: &str, judge: &dyn Fn(&ImportView) -> Verdict) -> Option<Vec<TextEdit>>`.

- [ ] **Step 1: Write the failing tests**

Create `crates/nova-fmt/tests/organize.rs` (marker `T9-TESTS`):

```rust
//! Organize imports' block (spec
//! `docs/superpowers/specs/2026-10-10-phase-3-4b-fixes-and-colour-design.md`
//! §6, §9.4).

use nova_fmt::{format, organize, Group, ImportView, TextEdit, Verdict};

/// Every import the project's own, nothing unused.
fn keep(_: &ImportView) -> Verdict {
    Verdict {
        group: Group::Module,
        unused_glob: false,
        unused_names: Vec::new(),
    }
}

/// `text` with `edits` applied, from the last.
fn applied(text: &str, edits: &[TextEdit]) -> String {
    let mut edits = edits.to_vec();
    edits.sort_by_key(|e| e.start);
    let mut out = text.to_string();
    for e in edits.iter().rev() {
        out.replace_range(e.start as usize..e.end as usize, &e.text);
    }
    out
}

/// `text` organized by `judge`; the result is as `nova fmt` prints it, and
/// a second run offers nothing.
#[track_caller]
fn organized(text: &str, judge: &dyn Fn(&ImportView) -> Verdict) -> String {
    let edits = organize(text, judge).unwrap_or_else(|| panic!("nothing to do for {text:?}"));
    let out = applied(text, &edits);
    assert_eq!(organize(&out, judge), None, "a second run changes {out:?}");
    assert_eq!(format(&out).unwrap(), out.replace("\r\n", "\n"), "not as nova fmt prints it");
    out
}

#[test]
fn scattered_imports_gather_at_the_first() {
    assert_eq!(
        organized("import b\n\nfn f() {}\n\nimport a\n", &keep),
        "import a\nimport b\n\nfn f() {}\n"
    );
}

#[test]
fn imports_and_their_lists_are_sorted() {
    assert_eq!(
        organized("import zeta\nimport alpha::{c, a}\n\nfn main() {}\n", &keep),
        "import alpha::{a, c}\nimport zeta\n\nfn main() {}\n"
    );
}

#[test]
fn imports_of_one_module_merge() {
    assert_eq!(
        organized("import m::{b}\nimport m::{a}\nimport n\nimport n\n\nfn main() {}\n", &keep),
        "import m::{a, b}\nimport n\n\nfn main() {}\n"
    );
    // A list beside a glob of the same module goes (spec §6.3).
    assert_eq!(
        organized("import m\nimport m::{a}\n\nfn main() {}\n", &keep),
        "import m\n\nfn main() {}\n"
    );
}

#[test]
fn unused_imports_and_names_go() {
    let judge = |v: &ImportView| Verdict {
        group: Group::Module,
        unused_glob: v.first == "n",
        unused_names: match v.first {
            "m" => vec!["b".to_string()],
            "o" => vec!["c".to_string()],
            _ => Vec::new(),
        },
    };
    assert_eq!(
        organized("import m::{a, b}\nimport n\nimport o::{c}\n\nfn main() {}\n", &judge),
        "import m::{a}\n\nfn main() {}\n"
    );
}

#[test]
fn comments_travel_with_their_imports_and_a_header_stays() {
    let text = "// header\n\n// about m\nimport m // why m\n// about a\nimport a\n\nfn main() {}\n";
    assert_eq!(
        organized(text, &keep),
        "// header\n\n// about a\nimport a\n// about m\nimport m // why m\n\nfn main() {}\n"
    );
    // An unused import's comments go with it.
    let judge = |v: &ImportView| Verdict {
        group: Group::Module,
        unused_glob: v.first == "a",
        unused_names: Vec::new(),
    };
    assert_eq!(
        organized(text, &judge),
        "// header\n\n// about m\nimport m // why m\n\nfn main() {}\n"
    );
}

#[test]
fn a_doc_comment_travels_with_its_import() {
    assert_eq!(
        organized("/// The shapes.\nimport m\nimport a\n\nfn main() {}\n", &keep),
        "import a\n/// The shapes.\nimport m\n\nfn main() {}\n"
    );
}

#[test]
fn two_groups_dependencies_first() {
    let judge = |v: &ImportView| Verdict {
        group: if v.first == "geom" {
            Group::Dependency
        } else {
            Group::Module
        },
        unused_glob: false,
        unused_names: Vec::new(),
    };
    assert_eq!(
        organized("import shapes\nimport geom\nimport app_utils\n\nfn main() {}\n", &judge),
        "import geom\n\nimport app_utils\nimport shapes\n\nfn main() {}\n"
    );
}

#[test]
fn organize_removes_every_import_and_the_blank_line_after() {
    // Review Focus 5.
    let unused = |_: &ImportView| Verdict {
        group: Group::Module,
        unused_glob: true,
        unused_names: Vec::new(),
    };
    assert_eq!(
        organized("import m\nimport n\n\nfn main() {}\n", &unused),
        "fn main() {}\n"
    );
    // At the end of the text, the blank line before it goes.
    let b_unused = |v: &ImportView| Verdict {
        group: Group::Module,
        unused_glob: v.first == "b",
        unused_names: Vec::new(),
    };
    assert_eq!(
        organized("import a\n\nfn f() {}\n\nimport b\n", &b_unused),
        "import a\n\nfn f() {}\n"
    );
}

#[test]
fn a_crlf_file_keeps_its_line_endings() {
    assert_eq!(
        organized("import b\r\nimport a\r\n\r\nfn main() {}\r\n", &keep),
        "import a\r\nimport b\r\n\r\nfn main() {}\r\n"
    );
}

#[test]
fn nothing_to_do_or_nothing_parsed_offers_nothing() {
    assert_eq!(organize("import a\nimport b\n\nfn main() {}\n", &keep), None);
    assert_eq!(organize("fn main() {}\n", &keep), None);
    assert_eq!(organize("import a\nfn main( {\n", &keep), None);
}
```

- [ ] **Step 2: Run the tests to see them fail**

Run: `cd /d/Projects/nona/nova && cargo test -p nova-fmt --test organize 2>&1 | tail -3`
Expected: a compile error, `unresolved imports 'nova_fmt::organize'`.

- [ ] **Step 3: The block**

Create `crates/nova-fmt/src/organize.rs` (marker `T9-ORGANIZE`):

```rust
//! Organize imports' block (spec
//! `docs/superpowers/specs/2026-10-10-phase-3-4b-fixes-and-colour-design.md`
//! §6). The caller judges each import: its group, and what of it is
//! unused. The block is printed by the formatter itself (plan decision 10).

use nova_ast::item::ImportKind;
use nova_ast::Item;
use nova_diagnostics::lines;
use nova_lexer::Token;

use crate::source::Source;

/// One import, as the caller sees it to judge it.
pub struct ImportView<'a> {
    /// Its path, as `a::b`.
    pub path: String,
    /// Where the path's first segment starts.
    pub path_start: u32,
    /// The path's first segment.
    pub first: &'a str,
    /// Whether it is a glob, `import m`.
    pub glob: bool,
    /// A `{…}` list's names, each with where it starts.
    pub names: Vec<(&'a str, u32)>,
}

/// Which group an import goes in (spec §6.2): dependencies first.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Group {
    Dependency,
    Module,
}

/// The caller's judgement of one import (spec §6.2, §6.4).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Verdict {
    pub group: Group,
    /// A glob nothing in the file uses.
    pub unused_glob: bool,
    /// The names of a list nothing in the file uses.
    pub unused_names: Vec<String>,
}

/// Replace bytes `start..end` of the text with `text`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TextEdit {
    pub start: u32,
    pub end: u32,
    pub text: String,
}

/// One import, gathered for the block.
struct Entry {
    group: Group,
    path: String,
    /// `None` for a glob.
    names: Option<Vec<String>>,
    /// Its comment, doc and attribute lines, in order.
    lead: Vec<String>,
    /// A comment ending its line.
    trail: Option<String>,
}

/// The edit organizing `text`'s imports makes (spec §6): one, from the
/// first import to the last change. `None` when the text does not parse,
/// an import shares a line with other code or is an `import … as`, or
/// nothing would change (plan decision 10).
pub fn organize(text: &str, judge: &dyn Fn(&ImportView) -> Verdict) -> Option<Vec<TextEdit>> {
    let src = Source::parse(text, "<organize>").ok()?;
    let items = &src.file.items;
    let imports: Vec<usize> = (0..items.len())
        .filter(|&i| matches!(items[i].value, Item::Import(_)))
        .collect();
    if imports.is_empty() {
        return None;
    }
    // The bytes each import owns: its lead lines through its line's end.
    let mut regions: Vec<(usize, usize)> = Vec::new();
    let mut entries: Vec<Entry> = Vec::new();
    for (k, &i) in imports.iter().enumerate() {
        let Item::Import(imp) = &items[i].value else {
            continue;
        };
        if matches!(imp.kind, ImportKind::Alias(_)) {
            return None;
        }
        let (start, end) = (items[i].span.start as usize, items[i].span.end as usize);
        let first_line = lines::line_start(text, start);
        if !text[first_line..start].trim_matches([' ', '\t']).is_empty() {
            return None;
        }
        let line_end = lines::next_line_start(text, end);
        let rest = text[end..line_end]
            .trim_end_matches(['\r', '\n'])
            .trim_matches([' ', '\t']);
        let trail = match rest {
            "" => None,
            r if r.starts_with("//") => Some(r.to_string()),
            _ => return None,
        };
        // After another import, every line between them is this one's lead
        // (spec §6.5); after anything else, the comments directly above.
        let lead_start = if k > 0 && imports[k - 1] + 1 == i {
            regions[k - 1].1
        } else {
            comments_above(text, first_line)
        };
        let mut lead: Vec<String> = text[lead_start..first_line]
            .lines()
            .map(|l| l.trim().to_string())
            .filter(|l| !l.is_empty())
            .collect();
        // Its `///` docs and attributes: the item's text before `import`.
        let keyword = src.find_from(items[i].span.start, &Token::Import)? as usize;
        lead.extend(
            text[start..keyword]
                .lines()
                .map(|l| l.trim().to_string())
                .filter(|l| !l.is_empty()),
        );
        regions.push((lead_start, line_end));
        let segments = &imp.path.value.segments;
        let first = segments.first()?;
        let view = ImportView {
            path: segments
                .iter()
                .map(|s| s.value.as_str())
                .collect::<Vec<_>>()
                .join("::"),
            path_start: first.span.start,
            first: &first.value,
            glob: matches!(imp.kind, ImportKind::Simple),
            names: match &imp.kind {
                ImportKind::List(names) => names
                    .iter()
                    .map(|n| (n.value.as_str(), n.span.start))
                    .collect(),
                _ => Vec::new(),
            },
        };
        let verdict = judge(&view);
        let (names, removed) = match &imp.kind {
            ImportKind::List(listed) => {
                let kept: Vec<String> = listed
                    .iter()
                    .map(|n| n.value.clone())
                    .filter(|n| !verdict.unused_names.contains(n))
                    .collect();
                let removed = !listed.is_empty() && kept.is_empty();
                (Some(kept), removed)
            }
            _ => (None, verdict.unused_glob),
        };
        if !removed {
            entries.push(Entry {
                group: verdict.group,
                path: view.path,
                names,
                lead,
                trail,
            });
        }
    }
    let nl = lines::ending(text);
    let block = render(merge(entries))?.replace('\n', nl);
    let out = assemble(text, &regions, &block, nl);
    if out == text {
        return None;
    }
    // One edit, from the first import to the last byte that changes.
    let start = regions[0].0;
    let room = (text.len() - start).min(out.len() - start);
    let mut same = 0;
    while same < room && text.as_bytes()[text.len() - 1 - same] == out.as_bytes()[out.len() - 1 - same] {
        same += 1;
    }
    while !text.is_char_boundary(text.len() - same) {
        same -= 1;
    }
    Some(vec![TextEdit {
        start: start as u32,
        end: (text.len() - same) as u32,
        text: out[start..out.len() - same].to_string(),
    }])
}

/// Where the comment lines directly above the line starting at `line`
/// start, with no blank line between (spec §6.5).
fn comments_above(text: &str, line: usize) -> usize {
    let mut at = line;
    while at > 0 {
        let begin = lines::line_start(text, at - 1);
        if lines::is_comment_line(&text[begin..at]) {
            at = begin;
        } else {
            break;
        }
    }
    at
}

/// Imports of one path become one (spec §6.3): a glob absorbs a list
/// beside it, and lists merge. Sorted by group, then path.
fn merge(entries: Vec<Entry>) -> Vec<Entry> {
    let mut out: Vec<Entry> = Vec::new();
    for e in entries {
        match out.iter_mut().find(|m| m.path == e.path) {
            Some(m) => {
                m.lead.extend(e.lead);
                m.lead.extend(e.trail);
                let both_lists = m.names.is_some() && e.names.is_some();
                if both_lists {
                    if let (Some(a), Some(b)) = (m.names.as_mut(), e.names) {
                        a.extend(b);
                    }
                } else {
                    m.names = None;
                }
            }
            None => out.push(e),
        }
    }
    for e in &mut out {
        if let Some(names) = &mut e.names {
            names.sort();
            names.dedup();
        }
    }
    out.sort_by(|a, b| (a.group, &a.path).cmp(&(b.group, &b.path)));
    out
}

/// The block: each group's imports, a blank line between groups, printed
/// by the formatter (plan decision 10). Empty when no import is left.
fn render(entries: Vec<Entry>) -> Option<String> {
    if entries.is_empty() {
        return Some(String::new());
    }
    let mut block = String::new();
    for (k, e) in entries.iter().enumerate() {
        if k > 0 && entries[k - 1].group != e.group {
            block.push('\n');
        }
        for l in &e.lead {
            block.push_str(l);
            block.push('\n');
        }
        block.push_str("import ");
        block.push_str(&e.path);
        if let Some(names) = &e.names {
            block.push_str("::{");
            block.push_str(&names.join(", "));
            block.push('}');
        }
        if let Some(t) = &e.trail {
            block.push(' ');
            block.push_str(t);
        }
        block.push('\n');
    }
    crate::format_named(&block, "<organize>").ok()
}

/// `text` with the block at the first import's place and every other
/// import's region gone. A removed import's blank line goes: the one after
/// it, or, at the text's end, the one before it (spec §6.2).
fn assemble(text: &str, regions: &[(usize, usize)], block: &str, nl: &str) -> String {
    let blank = format!("{nl}{nl}");
    let mut out = String::from(&text[..regions[0].0]);
    for (k, &(_, end)) in regions.iter().enumerate() {
        let piece = if k == 0 { block } else { "" };
        out.push_str(piece);
        let gap_end = regions.get(k + 1).map_or(text.len(), |r| r.0);
        let mut gap = &text[end..gap_end];
        if piece.is_empty() && (out.is_empty() || out.ends_with(&blank)) {
            if let Some(rest) = gap.strip_prefix(nl) {
                gap = rest;
            } else if gap.is_empty() && gap_end == text.len() && !out.is_empty() {
                out.truncate(out.len() - nl.len());
            }
        }
        out.push_str(gap);
    }
    out
}
```

In `crates/nova-fmt/src/lib.rs`, replace `mod file;` with `mod file;\nmod organize;`
and `pub use file::{format_buffer, format_file, format_text, FileError, Formatted, LineEnding};`
with that line and
`pub use organize::{organize, Group, ImportView, TextEdit, Verdict};`.

- [ ] **Step 4: Run the tests**

Run: `cd /d/Projects/nona/nova && cargo test -p nova-fmt --test organize 2>&1 | tail -3`
Expected: `test result: ok. 10 passed; 0 failed`.

Run: `cd /d/Projects/nona/nova && cargo test -p nova-fmt 2>&1 | grep -E "^test result" | grep -v " 0 failed"; echo done`
Expected: `done` alone.

- [ ] **Step 5: Commit**

Write `$P/msg-34b-9.txt`:

```text
nova-fmt: organize imports' block

organize gathers a file's imports into one block at the first: merged,
with what the caller judges unused removed, each import's comments, docs
and attributes travelling with it, dependencies before the project's
own modules, printed by the formatter itself (spec 3.4b §6).

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
```

Run: `cd /d/Projects/nona/nova && cargo fmt --all && git add crates/nova-fmt/src/organize.rs crates/nova-fmt/tests/organize.rs && git add -u && git commit -q -F $P/msg-34b-9.txt && git log -1 --format=%s`
Expected: `nova-fmt: organize imports' block`

---

### Task 10: Quick fixes in the server

**Files:**
- Create: `crates/nova-lsp/src/code_action.rs`, `crates/nova-cli/tests/lsp_fixes.rs`
- Modify: `crates/nova-lsp/src/lib.rs` (module list, imports,
  `capabilities`, `Server::request`, a new `answer_of`)
- Modify: `crates/nova-lsp/src/convert.rs` (`severity` and `message`
  become `pub(crate)`), `crates/nova-lsp/src/rename.rs` (`Owner`, `owner`
  and `in_registry` become `pub(crate)`)
- Modify: `crates/nova-cli/tests/lsp_client/mod.rs` (`Client::initialized`;
  the shared helpers), `crates/nova-cli/tests/lsp.rs` and
  `crates/nova-cli/tests/lsp_navigation.rs` (their copies go)

**Interfaces:**
- Consumes: `Diagnostic::fixes` (Tasks 1-6); `analysis::{answering,
  options}`, `navigate::Locator` (3.4a).
- Produces:
  - `code_action::code_actions(answer: &Answer, path: &Path, start: u32, end: u32, only: Option<&[lsp::CodeActionKind]>, uri_of: &dyn Fn(&Path) -> String) -> Vec<lsp::CodeActionOrCommand>`;
  - `Server::answer_of(&self, uri: &lsp::Uri) -> Option<(Answer, PathBuf, String)>`;
  - in `lsp_client`: `Client::initialized: Value`, `MANIFEST`, `APP_MAIN`,
    `INDEX`, `project`, `app_and_library`, `app_with_main`,
    `registry_app`, `lock_and_cache` (plan decision 15);
  - in `lsp_fixes.rs`: `open`, `position`, `at`, `actions`, `titled`,
    `edits_in`, which Tasks 11-13 use.

- [ ] **Step 1: The shared helpers and the initialize result**

In `crates/nova-cli/tests/lsp_client/mod.rs`:
1. In `pub struct Client`, after `    unread: Vec<Value>,` insert
   `    /// The initialize response's result: the server's capabilities.\n    pub initialized: Value,`;
   in `start_with_env`'s `Client { … }` literal, after
   `            unread: Vec::new(),` insert `            initialized: Value::Null,`;
   and replace
   `        client.request(\n            "initialize",`
   with
   `        client.initialized = client.request(\n            "initialize",`
   and the `);` that closes that call with `)["result"]\n        .clone();`.
2. Append (marker `T10-SHARED`):

```rust

// === The project helpers the stdio tests share (3.4b plan decision 15) ===

/// A project's `nova.toml`, naming it `demo`.
pub const MANIFEST: &str = "[package]\nname = \"demo\"\nversion = \"0.1.0\"\nedition = \"2026\"\n";

/// 3.3a's app's main file, which calls `geom`'s `area`.
pub const APP_MAIN: &str = "import geom\n\nfn main() {\n    let a: Int = area()\n}\n";

/// The registry index the tests' locks name.
pub const INDEX: &str = "https://example.test/index/";

/// A project: `nova.toml`, and `src/` holding `files`.
pub fn project(name: &str, files: &[(&str, &str)]) -> PathBuf {
    let dir = fresh_dir(name);
    std::fs::write(dir.join("nova.toml"), MANIFEST).unwrap();
    std::fs::create_dir_all(dir.join("src")).unwrap();
    for (file, text) in files {
        std::fs::write(dir.join("src").join(file), text).unwrap();
    }
    dir
}

/// `app`, which depends on `geom` by path and runs `APP_MAIN`, in one
/// fresh directory. `geom`'s `src/lib.nova` is `lib`.
pub fn app_and_library(name: &str, lib: &str) -> (PathBuf, PathBuf) {
    app_with_main(name, lib, APP_MAIN)
}

/// [`app_and_library`], with `main` as the app's `src/main.nova`.
pub fn app_with_main(name: &str, lib: &str, main: &str) -> (PathBuf, PathBuf) {
    let dir = fresh_dir(name);
    let geom = dir.join("geom");
    std::fs::create_dir_all(geom.join("src")).unwrap();
    std::fs::write(geom.join("nova.toml"), MANIFEST.replace("demo", "geom")).unwrap();
    std::fs::write(geom.join("src").join("lib.nova"), lib).unwrap();
    let app = dir.join("app");
    std::fs::create_dir_all(app.join("src")).unwrap();
    let manifest = format!(
        "{}\n[dependencies]\ngeom = {{ path = \"../geom\" }}\n",
        MANIFEST.replace("demo", "app")
    );
    std::fs::write(app.join("nova.toml"), manifest).unwrap();
    std::fs::write(app.join("src").join("main.nova"), main).unwrap();
    (app, geom)
}

/// `dir/app`, which depends on `geom = "<req>"` and runs `APP_MAIN`, and
/// `dir/home`, the server's `NOVA_HOME`.
pub fn registry_app(name: &str, req: &str) -> (PathBuf, PathBuf) {
    let dir = fresh_dir(name);
    let app = dir.join("app");
    std::fs::create_dir_all(app.join("src")).unwrap();
    std::fs::write(
        app.join("nova.toml"),
        format!(
            "{}\n[dependencies]\ngeom = \"{req}\"\n",
            MANIFEST.replace("demo", "app")
        ),
    )
    .unwrap();
    std::fs::write(app.join("src").join("main.nova"), APP_MAIN).unwrap();
    (app, dir.join("home"))
}

/// `geom` 0.1.0, locked in `app`'s nova.lock and unpacked under `home`,
/// with `lib` as its lib.nova. Its directory.
pub fn lock_and_cache(app: &Path, home: &Path, lib: &str) -> PathBuf {
    let geom = home
        .join("registry")
        .join("src")
        .join(nova_pm::index_dir_name(INDEX))
        .join("geom-0.1.0");
    std::fs::create_dir_all(geom.join("src")).unwrap();
    std::fs::write(geom.join("nova.toml"), MANIFEST.replace("demo", "geom")).unwrap();
    std::fs::write(geom.join("src").join("lib.nova"), lib).unwrap();
    std::fs::write(
        app.join("nova.lock"),
        format!(
            "version = 1\nindex = \"{INDEX}\"\n\n[[package]]\nname = \"geom\"\n\
             version = \"0.1.0\"\nchecksum = \"00\"\ndependencies = []\n"
        ),
    )
    .unwrap();
    geom
}
```

3. In `crates/nova-cli/tests/lsp.rs`, delete its `MANIFEST`, `project`,
   `APP_MAIN` and `app_and_library` (lines 114-125 and 758-776 at
   `1c7bae5`). In `crates/nova-cli/tests/lsp_navigation.rs`, delete its
   `MANIFEST`, `project`, `APP_MAIN`, `app_and_library`, `INDEX`,
   `registry_app` and `lock_and_cache`. Add the names each file uses to
   its `use lsp_client::{…};`; the compiler names any missing one.

Run: `cd /d/Projects/nona/nova && cargo test -p nova-cli --test lsp --test lsp_navigation --no-run 2>&1 | grep -E "^(warning|error)" | sort | uniq -c; echo done`
Expected: `done` alone.

- [ ] **Step 2: Write the failing tests**

Create `crates/nova-cli/tests/lsp_fixes.rs` (marker `T10-TESTS`):

```rust
//! `nova lsp`'s code actions and semantic tokens, driven over stdio (spec
//! `docs/superpowers/specs/2026-10-10-phase-3-4b-fixes-and-colour-design.md`
//! §5-§7, §9.6, §10).

mod lsp_client;

use lsp_client::{file_uri, lock_and_cache, project, registry_app, same_uri, Client};
use serde_json::{json, Value};

fn open(client: &mut Client, uri: &str, text: &str) {
    client.notify(
        "textDocument/didOpen",
        json!({ "textDocument": { "uri": uri, "languageId": "nova", "version": 1, "text": text } }),
    );
}

/// The LSP position of byte `byte` of `text`.
fn position(text: &str, byte: usize) -> Value {
    let line = text[..byte].matches('\n').count();
    let line_start = text[..byte].rfind('\n').map_or(0, |i| i + 1);
    json!({ "line": line, "character": text[line_start..byte].encode_utf16().count() })
}

/// The position of the start of the `n`th `marker` (from 0) in `text`.
fn at(text: &str, marker: &str, n: usize) -> Value {
    let start = text
        .match_indices(marker)
        .nth(n)
        .unwrap_or_else(|| panic!("no `{marker}` number {n}"))
        .0;
    position(text, start)
}

/// The code actions for `start..end`, with `only` when given.
fn actions_in(client: &mut Client, uri: &str, start: Value, end: Value, only: Option<&[&str]>) -> Vec<Value> {
    let mut context = json!({ "diagnostics": [] });
    if let Some(only) = only {
        context["only"] = json!(only);
    }
    let response = client.request(
        "textDocument/codeAction",
        json!({ "textDocument": { "uri": uri }, "range": { "start": start, "end": end }, "context": context }),
    );
    response["result"]
        .as_array()
        .unwrap_or_else(|| panic!("{response}"))
        .clone()
}

/// The code actions at the empty range `at`, as VS Code sends a cursor.
fn actions(client: &mut Client, uri: &str, at: Value, only: Option<&[&str]>) -> Vec<Value> {
    actions_in(client, uri, at.clone(), at, only)
}

/// The action titled `title`.
#[track_caller]
fn titled<'a>(found: &'a [Value], title: &str) -> &'a Value {
    found
        .iter()
        .find(|a| a["title"] == title)
        .unwrap_or_else(|| panic!("no {title:?} in {found:?}"))
}

/// The edits an action makes in the document `uri`.
#[track_caller]
fn edits_in(action: &Value, uri: &str) -> Vec<Value> {
    let changes = action["edit"]["changes"]
        .as_object()
        .unwrap_or_else(|| panic!("{action}"));
    changes
        .iter()
        .find(|(u, _)| same_uri(u, uri))
        .map(|(_, e)| e.as_array().unwrap().clone())
        .unwrap_or_else(|| panic!("no edit for {uri} in {action}"))
}

// === Task 10: quick fixes (spec §5) ===

const MUTABLE: &str = "fn main() {\n    let x = 1\n    x = 2\n    println(\"${x}\")\n}\n";

#[test]
fn the_server_offers_quick_fixes_and_organize_imports() {
    let dir = project("capabilities", &[("main.nova", "fn main() {}\n")]);
    let client = Client::start(&dir, false);
    assert_eq!(
        client.initialized["capabilities"]["codeActionProvider"]["codeActionKinds"],
        json!(["quickfix", "source.organizeImports"]),
        "{}",
        client.initialized
    );
}

#[test]
fn make_mutable_at_a_cursor() {
    let dir = project("make-mutable", &[("main.nova", MUTABLE)]);
    let uri = file_uri(&dir.join("src").join("main.nova"));
    let mut client = Client::start(&dir, false);
    open(&mut client, &uri, MUTABLE);
    let found = actions(&mut client, &uri, at(MUTABLE, "x = 2", 0), None);
    let action = titled(&found, "Make `x` mutable");
    assert_eq!(action["kind"], "quickfix");
    assert_eq!(action["isPreferred"], true);
    assert_eq!(action["diagnostics"][0]["code"], "E0060");
    let point = json!({ "line": 1, "character": 8 });
    assert_eq!(
        edits_in(action, &uri),
        [json!({ "range": { "start": point, "end": point }, "newText": "mut " })]
    );
}

#[test]
fn make_public_edits_the_sibling_modules_uri() {
    let main = "import lib::{hidden}\n\nfn main() {\n    println(\"${hidden()}\")\n}\n";
    let dir = project(
        "make-public",
        &[("main.nova", main), ("lib.nova", "fn hidden() -> Int {\n    1\n}\n")],
    );
    let uri = file_uri(&dir.join("src").join("main.nova"));
    let lib = file_uri(&dir.join("src").join("lib.nova"));
    let mut client = Client::start(&dir, false);
    open(&mut client, &uri, main);
    let found = actions(&mut client, &uri, at(main, "hidden}", 0), None);
    let action = titled(&found, "Make `hidden` public in `lib`");
    let point = json!({ "line": 0, "character": 0 });
    assert_eq!(
        edits_in(action, &lib),
        [json!({ "range": { "start": point, "end": point }, "newText": "pub " })]
    );
}

#[test]
fn only_quickfix_or_source_filters_the_actions() {
    let dir = project("only", &[("main.nova", MUTABLE)]);
    let uri = file_uri(&dir.join("src").join("main.nova"));
    let mut client = Client::start(&dir, false);
    open(&mut client, &uri, MUTABLE);
    let fixes = actions(&mut client, &uri, at(MUTABLE, "x = 2", 0), Some(&["quickfix"]));
    titled(&fixes, "Make `x` mutable");
    let source = actions(&mut client, &uri, at(MUTABLE, "x = 2", 0), Some(&["source"]));
    assert!(source.iter().all(|a| a["kind"] != "quickfix"), "{source:?}");
}

#[test]
fn no_actions_inside_a_downloaded_package() {
    let (app, home) = registry_app("downloaded", "0.1");
    let lib = "pub fn area() -> Int {\n    let x = 1\n    x = 2\n    x\n}\n";
    let geom = lock_and_cache(&app, &home, lib);
    let uri = file_uri(&geom.join("src").join("lib.nova"));
    let mut client = Client::start_with_env(&app, false, &[("NOVA_HOME", &home)]);
    open(&mut client, &uri, lib);
    assert_eq!(
        actions(&mut client, &uri, at(lib, "x = 2", 0), None),
        Vec::<Value>::new()
    );
}

#[test]
fn a_quick_fix_edits_an_unsaved_buffer_at_its_own_offsets() {
    // Review Focus 1: the buffer holds two lines the disk does not.
    let dir = project("unsaved", &[("main.nova", MUTABLE)]);
    let uri = file_uri(&dir.join("src").join("main.nova"));
    let buffer = format!("// one\n// two\n{MUTABLE}");
    let mut client = Client::start(&dir, false);
    open(&mut client, &uri, &buffer);
    let found = actions(&mut client, &uri, at(&buffer, "x = 2", 0), None);
    let action = titled(&found, "Make `x` mutable");
    assert_eq!(
        edits_in(action, &uri)[0]["range"]["start"],
        json!({ "line": 3, "character": 8 })
    );
}

#[test]
fn a_quick_fixs_range_counts_utf16_after_thai_and_an_emoji() {
    // Review Focus 2: `x` follows three Thai characters and an emoji, one
    // UTF-16 unit each and two.
    let text = "fn main() {\n    let s = \"ไทย😀\"; let x = 1\n    x = 2\n    println(\"${s}${x}\")\n}\n";
    let dir = project("utf16-fix", &[("main.nova", text)]);
    let uri = file_uri(&dir.join("src").join("main.nova"));
    let mut client = Client::start(&dir, false);
    open(&mut client, &uri, text);
    let found = actions(&mut client, &uri, at(text, "x = 2", 0), None);
    let action = titled(&found, "Make `x` mutable");
    assert_eq!(
        edits_in(action, &uri)[0]["range"]["start"],
        json!({ "line": 1, "character": 25 })
    );
}

#[test]
fn two_errors_with_one_fix_offer_it_once() {
    // Review Focus 3.
    let text = "fn main() {\n    let x = 1\n    x = 2\n    x = 3\n    println(\"${x}\")\n}\n";
    let dir = project("one-fix-once", &[("main.nova", text)]);
    let uri = file_uri(&dir.join("src").join("main.nova"));
    let mut client = Client::start(&dir, false);
    open(&mut client, &uri, text);
    let found = actions_in(
        &mut client,
        &uri,
        at(text, "x = 2", 0),
        at(text, "    println", 0),
        None,
    );
    let mutable: Vec<&Value> = found
        .iter()
        .filter(|a| a["title"] == "Make `x` mutable")
        .collect();
    assert_eq!(mutable.len(), 1, "{found:?}");
    assert_eq!(mutable[0]["diagnostics"].as_array().unwrap().len(), 2, "{found:?}");
}
```

Run: `cd /d/Projects/nona/nova && cargo test -p nova-cli --test lsp_fixes 2>&1 | grep -E "^test |test result"`
Expected: every test FAILED: the first because `codeActionProvider` is
null, the rest with "nova lsp does not handle textDocument/codeAction"
in the panic.

- [ ] **Step 3: The server's quick fixes**

1. In `crates/nova-lsp/src/convert.rs`, replace `fn severity(s: Severity)`
   with `pub(crate) fn severity(s: Severity)`, and
   `fn message(d: &Diagnostic, suffix: Option<&str>) -> String {` with
   `pub(crate) fn message(d: &Diagnostic, suffix: Option<&str>) -> String {`.
2. In `crates/nova-lsp/src/rename.rs`, replace `fn in_registry(path: &Path) -> bool {`
   with `pub(crate) fn in_registry(path: &Path) -> bool {`, `enum Owner {`
   with `pub(crate) enum Owner {`, and `fn owner(a: &Analysis, file: FileId) -> Owner {`
   with `pub(crate) fn owner(a: &Analysis, file: FileId) -> Owner {`.
3. Create `crates/nova-lsp/src/code_action.rs` (marker `T10-CODE-ACTION`):

```rust
//! Code actions: each fix as a quick fix, and organize imports (spec
//! `docs/superpowers/specs/2026-10-10-phase-3-4b-fixes-and-colour-design.md`
//! §5).

use std::collections::HashMap;
use std::path::Path;

use lsp_types as lsp;
use nova_diagnostics::{Diagnostic, Fix};

use crate::analysis::Answer;
use crate::navigate::Locator;
use crate::rename::{in_registry, owner, Owner};
use crate::{convert, std_cache};

/// Whether an action of `kind` is wanted, given the request's `only`: its
/// kind is an entry, or starts with an entry and a `.` (spec §5).
fn wanted(kind: &lsp::CodeActionKind, only: Option<&[lsp::CodeActionKind]>) -> bool {
    only.map_or(true, |only| {
        only.iter().any(|o| {
            kind.as_str() == o.as_str() || kind.as_str().starts_with(&format!("{}.", o.as_str()))
        })
    })
}

/// The actions for bytes `start..end` of `path`'s document (spec §5).
/// None inside std's cache or a downloaded package, as rename refuses
/// there.
pub fn code_actions(
    answer: &Answer,
    path: &Path,
    start: u32,
    end: u32,
    only: Option<&[lsp::CodeActionKind]>,
    uri_of: &dyn Fn(&Path) -> String,
) -> Vec<lsp::CodeActionOrCommand> {
    if std_cache::in_std_cache(path) || in_registry(path) {
        return Vec::new();
    }
    let mut out = Vec::new();
    if wanted(&lsp::CodeActionKind::QUICKFIX, only) {
        out.extend(quick_fixes(answer, start, end, uri_of));
    }
    out
}

/// Each fix of each diagnostic whose primary label overlaps
/// `start..end`, once, in the diagnostics' order (spec §5). A fix with an
/// edit outside the owning project is dropped.
fn quick_fixes(
    answer: &Answer,
    start: u32,
    end: u32,
    uri_of: &dyn Fn(&Path) -> String,
) -> Vec<lsp::CodeActionOrCommand> {
    let a = &answer.analysis;
    let mut found: Vec<(u32, &Diagnostic, &Fix)> = Vec::new();
    for d in &a.diagnostics {
        let Some(label) = d.labels.iter().find(|l| l.primary) else {
            continue;
        };
        if label.span.file != answer.file || label.span.start > end || start > label.span.end {
            continue;
        }
        for fix in &d.fixes {
            if fix
                .edits
                .iter()
                .all(|e| matches!(owner(a, e.span.file), Owner::Own))
            {
                found.push((label.span.start, d, fix));
            }
        }
    }
    found.sort_by_key(|(at, _, _)| *at);
    let locator = Locator::new(a, uri_of);
    let mut actions: Vec<(&Fix, lsp::CodeAction)> = Vec::new();
    for (_, d, fix) in found {
        let diagnostic = lsp_diagnostic(&locator, d);
        if let Some((_, action)) = actions.iter_mut().find(|(f, _)| **f == *fix) {
            action
                .diagnostics
                .get_or_insert_with(Vec::new)
                .extend(diagnostic);
            continue;
        }
        let Some(edit) = workspace_edit(&locator, fix) else {
            continue;
        };
        actions.push((
            fix,
            lsp::CodeAction {
                title: capitalised(&fix.title),
                kind: Some(lsp::CodeActionKind::QUICKFIX),
                diagnostics: Some(diagnostic.into_iter().collect()),
                edit: Some(edit),
                is_preferred: Some(d.fixes.len() == 1),
                ..Default::default()
            },
        ));
    }
    actions
        .into_iter()
        .map(|(_, action)| lsp::CodeActionOrCommand::CodeAction(action))
        .collect()
}

/// `d` as the server publishes it, at its primary label, without related
/// information (plan decision 14).
fn lsp_diagnostic(locator: &Locator, d: &Diagnostic) -> Option<lsp::Diagnostic> {
    let label = d.labels.iter().find(|l| l.primary)?;
    Some(lsp::Diagnostic {
        range: locator.range(label.span)?,
        severity: Some(convert::severity(d.severity)),
        code: Some(lsp::NumberOrString::String(d.code.clone())),
        source: Some("nova".to_string()),
        message: convert::message(d, None),
        ..Default::default()
    })
}

/// `fix`'s edits as LSP's, each file under the URI the locator gives it;
/// `None` when one cannot be placed.
// `WorkspaceEdit::changes` is a `HashMap` keyed by `lsp::Uri`, whose
// parsed form caches into a `Cell`; the key is never changed here.
#[allow(clippy::mutable_key_type)]
fn workspace_edit(locator: &Locator, fix: &Fix) -> Option<lsp::WorkspaceEdit> {
    let mut changes: HashMap<lsp::Uri, Vec<lsp::TextEdit>> = HashMap::new();
    for e in &fix.edits {
        let uri = locator.uri(e.span.file)?;
        let range = locator.range(e.span)?;
        changes.entry(uri).or_default().push(lsp::TextEdit {
            range,
            new_text: e.text.clone(),
        });
    }
    Some(lsp::WorkspaceEdit {
        changes: Some(changes),
        ..Default::default()
    })
}

/// A fix's title as the editor shows it (spec decision 16).
fn capitalised(title: &str) -> String {
    let mut chars = title.chars();
    match chars.next() {
        Some(c) => c.to_uppercase().chain(chars).collect(),
        None => String::new(),
    }
}
```

4. In `crates/nova-lsp/src/lib.rs`:
   - replace `mod checker;` with `mod checker;\nmod code_action;`;
   - replace `    Completion, Formatting, GotoDefinition, HoverRequest, PrepareRenameRequest, References,`
     with `    CodeActionRequest, Completion, Formatting, GotoDefinition, HoverRequest,\n    PrepareRenameRequest, References,`;
   - in `capabilities`, after the `rename_provider: …` field's closing
     `})),` insert:

```rust
        code_action_provider: Some(lsp::CodeActionProviderCapability::Options(
            lsp::CodeActionOptions {
                code_action_kinds: Some(vec![
                    lsp::CodeActionKind::QUICKFIX,
                    lsp::CodeActionKind::SOURCE_ORGANIZE_IMPORTS,
                ]),
                resolve_provider: Some(false),
                work_done_progress_options: Default::default(),
            },
        )),
```

   - in `Server::request`, before the `_ => Response::new_err(` arm, insert
     (marker `T10-REQUEST`):

```rust
            CodeActionRequest::METHOD => {
                match serde_json::from_value::<lsp::CodeActionParams>(request.params) {
                    Ok(p) => {
                        let found = self
                            .answer_of(&p.text_document.uri)
                            .map(|(answer, path, text)| {
                                let lines = LineIndex::new(&text);
                                let start = lines.offset(p.range.start.line, p.range.start.character);
                                let end = lines.offset(p.range.end.line, p.range.end.character);
                                code_action::code_actions(
                                    &answer,
                                    &path,
                                    start,
                                    end,
                                    p.context.only.as_deref(),
                                    &|path| self.uri_of(path),
                                )
                            })
                            .unwrap_or_default();
                        Response::new_ok(request.id, found)
                    }
                    Err(e) => invalid(request.id, e),
                }
            }
```

   - after `answer_at`'s closing brace, insert (marker `T10-ANSWER-OF`):

```rust

    /// The analysis that answers a request about the open document `uri`,
    /// with the index on, its path and its text (3.4a spec §4). `None` for
    /// a document that is not open.
    fn answer_of(&self, uri: &lsp::Uri) -> Option<(Answer, PathBuf, String)> {
        let doc = self.workspace.get(&uri::text(uri))?;
        let options = analysis::options(None, true);
        let answer = analysis::answering(&doc.path, &self.workspace.overlay(), &options)?;
        Some((answer, doc.path.clone(), doc.text.clone()))
    }
```

- [ ] **Step 4: Run the tests**

Run: `cd /d/Projects/nona/nova && cargo test -p nova-cli --test lsp_fixes 2>&1 | tail -3`
Expected: `test result: ok. 8 passed; 0 failed`.

Run: `cd /d/Projects/nona/nova && cargo test -p nova-lsp 2>&1 | grep -E "^test result" | grep -v " 0 failed"; cargo test -p nova-cli --test lsp --test lsp_navigation 2>&1 | grep -E "^test result" | grep -v " 0 failed"; echo done`
Expected: `done` alone.

- [ ] **Step 5: Commit**

Write `$P/msg-34b-10.txt`:

```text
nova-lsp: quick fixes

The server advertises code actions. Each request analyses afresh and
offers every fix on a diagnostic whose label overlaps the range, once,
titled as the editor shows it, preferred when it is the diagnostic's
only one; none inside std's cache or a downloaded package (spec 3.4b
§5). The stdio tests' project helpers move into lsp_client.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
```

Run: `cd /d/Projects/nona/nova && cargo fmt --all && git add crates/nova-lsp/src/code_action.rs crates/nova-cli/tests/lsp_fixes.rs && git add -u && git commit -q -F $P/msg-34b-10.txt && git log -1 --format=%s`
Expected: `nova-lsp: quick fixes`

---

### Task 11: Organize imports in the server

**Files:**
- Create: `crates/nova-lsp/src/organize.rs`
- Modify: `crates/nova-lsp/src/code_action.rs` (`code_actions`),
  `crates/nova-lsp/src/lib.rs` (module list)
- Test: `crates/nova-lsp/src/organize.rs` (unit), `crates/nova-cli/tests/lsp_fixes.rs`

**Interfaces:**
- Consumes: `nova_fmt::{organize, Group, ImportView, TextEdit, Verdict}`
  (Task 9); the index (3.4a); Task 10's test helpers.
- Produces: `organize::edits(answer: &Answer) -> Option<Vec<nova_fmt::TextEdit>>`,
  `organize::action(answer: &Answer, uri_of: &dyn Fn(&Path) -> String) -> Option<lsp::CodeAction>`.

- [ ] **Step 1: Write the failing tests**

Create `crates/nova-lsp/src/organize.rs` holding its doc and tests only
(marker `T11-UNIT-TESTS`):

```rust
//! Organize imports in the server: each import's group, and what of it is
//! unused (spec
//! `docs/superpowers/specs/2026-10-10-phase-3-4b-fixes-and-colour-design.md`
//! §6.2, §6.4; plan decision 11).

#[cfg(test)]
mod tests {
    use super::*;
    use crate::workspace::Overlay;

    /// `files`' first, analysed as the server would and organized; `None`
    /// when organizing changes nothing.
    fn organized(name: &str, files: &[(&str, &str)]) -> Option<String> {
        let dir = std::env::temp_dir().join(format!("nova-lsp-organize-{name}"));
        let mut overlay = Overlay::default();
        for (file, text) in files {
            overlay = overlay.with(&dir.join(file), text.to_string());
        }
        let options = crate::analysis::options(None, true);
        let answer = crate::analysis::answering(&dir.join(files[0].0), &overlay, &options)
            .expect("an answer");
        let mut text = answer.analysis.db.get_source(answer.file).unwrap().to_string();
        for e in edits(&answer)?.iter().rev() {
            text.replace_range(e.start as usize..e.end as usize, &e.text);
        }
        Some(text)
    }

    const GEOMETRY: &str =
        "pub fn origin() -> Int {\n    1\n}\n\npub fn manhattan() -> Int {\n    2\n}\n";

    #[test]
    fn a_list_name_unused_goes_and_a_used_one_stays() {
        assert_eq!(
            organized(
                "list",
                &[
                    ("main.nova", "import geometry::{manhattan, origin}\n\nfn main() {\n    let o = origin()\n}\n"),
                    ("geometry.nova", GEOMETRY),
                ]
            )
            .as_deref(),
            Some("import geometry::{origin}\n\nfn main() {\n    let o = origin()\n}\n")
        );
    }

    #[test]
    fn a_glob_unused_goes_and_a_used_one_stays() {
        assert_eq!(
            organized(
                "glob",
                &[
                    ("main.nova", "import geometry\nimport shout\n\nfn main() {\n    let o = origin()\n}\n"),
                    ("geometry.nova", GEOMETRY),
                    ("shout.nova", "pub fn shout() -> String {\n    \"!\"\n}\n"),
                ]
            )
            .as_deref(),
            Some("import geometry\n\nfn main() {\n    let o = origin()\n}\n")
        );
    }

    #[test]
    fn a_trait_used_only_through_its_methods_stays() {
        let loud = "pub trait Loud {\n    fn shout(self) -> String\n}\n\npub record T { s: String }\n\nimpl Loud for T {\n    fn shout(self) -> String {\n        self.s\n    }\n}\n\npub fn make() -> T {\n    T { s: \"hi\" }\n}\n";
        assert_eq!(
            organized(
                "trait",
                &[
                    ("main.nova", "import loud::{Loud, make}\n\nfn main() {\n    let s = make().shout()\n}\n"),
                    ("loud.nova", loud),
                ]
            ),
            None
        );
    }

    #[test]
    fn a_sum_type_used_only_through_its_variant_stays() {
        assert_eq!(
            organized(
                "variant",
                &[
                    ("main.nova", "import kinds::{Empty, Shape}\n\nfn main() {\n    let e = Empty\n}\n"),
                    ("kinds.nova", "pub type Shape =\n  | Circle(Int)\n  | Empty\n"),
                ]
            ),
            None
        );
    }

    #[test]
    fn an_error_in_the_file_keeps_unused_imports() {
        assert_eq!(
            organized(
                "error",
                &[
                    ("main.nova", "import geometry::{manhattan, origin}\n\nfn main() {\n    let o = origin()\n    let n: Int = \"no\"\n}\n"),
                    ("geometry.nova", GEOMETRY),
                ]
            ),
            None
        );
    }

    #[test]
    fn a_parse_error_elsewhere_keeps_unused_imports() {
        // Spec §6.4: a dropped item's E0001s are filtered out, so the parse
        // error must block removal itself.
        let broken = "pub fn origin() -> Int {\n    1\n}\n\npub fn manhattan( -> Int {\n    2\n}\n";
        assert_eq!(
            organized(
                "parse-error",
                &[
                    ("main.nova", "import geometry::{manhattan, origin}\n\nfn main() {\n    let o = origin()\n    let m = manhattan()\n}\n"),
                    ("geometry.nova", broken),
                ]
            ),
            None
        );
    }
}
```

In `crates/nova-lsp/src/lib.rs`, replace `mod navigate;` with
`mod navigate;\nmod organize;`.

Append to `crates/nova-cli/tests/lsp_fixes.rs` (marker `T11-TESTS`):

```rust

// === Task 11: organize imports (spec §6) ===

/// `text` with the LSP `edits` applied.
fn applied(text: &str, edits: &[Value]) -> String {
    let offset = |p: &Value| {
        let line = p["line"].as_u64().unwrap() as usize;
        let character = p["character"].as_u64().unwrap() as usize;
        let start: usize = text.split_inclusive('\n').take(line).map(str::len).sum();
        let (mut units, mut at) = (0, start);
        for c in text[start..].chars() {
            if units >= character {
                break;
            }
            units += c.len_utf16();
            at += c.len_utf8();
        }
        at
    };
    let mut spans: Vec<(usize, usize, String)> = edits
        .iter()
        .map(|e| {
            (
                offset(&e["range"]["start"]),
                offset(&e["range"]["end"]),
                e["newText"].as_str().unwrap().to_string(),
            )
        })
        .collect();
    spans.sort_by_key(|s| s.0);
    let mut out = text.to_string();
    for (start, end, new) in spans.iter().rev() {
        out.replace_range(*start..*end, new);
    }
    out
}

const SHAPES: &str = "pub fn twice() -> Int {\n    2\n}\n";
const EXTRA: &str = "pub fn unused() -> Int {\n    0\n}\n";

#[test]
fn organize_imports_groups_and_drops_an_unused_import() {
    let main = "import shapes\nimport geom\nimport extra\n\nfn main() {\n    println(\"${area()} ${twice()}\")\n}\n";
    let (app, _geom) = lsp_client::app_with_main("organize", "pub fn area() -> Int {\n    1\n}\n", main);
    std::fs::write(app.join("src").join("shapes.nova"), SHAPES).unwrap();
    std::fs::write(app.join("src").join("extra.nova"), EXTRA).unwrap();
    let uri = file_uri(&app.join("src").join("main.nova"));
    let mut client = Client::start(&app, false);
    open(&mut client, &uri, main);
    let found = actions(&mut client, &uri, at(main, "fn main", 0), Some(&["source.organizeImports"]));
    assert_eq!(found.len(), 1, "{found:?}");
    let action = titled(&found, "Organize imports");
    assert_eq!(action["kind"], "source.organizeImports");
    assert_eq!(
        applied(main, &edits_in(action, &uri)),
        "import geom\n\nimport shapes\n\nfn main() {\n    println(\"${area()} ${twice()}\")\n}\n"
    );
}

#[test]
fn organize_imports_in_a_file_with_an_error_keeps_unused_imports() {
    let main = "import shapes\nimport extra\n\nfn main() {\n    let n: Int = \"no\"\n    println(\"${twice()}\")\n}\n";
    let dir = project(
        "organize-error",
        &[("main.nova", main), ("shapes.nova", SHAPES), ("extra.nova", EXTRA)],
    );
    let uri = file_uri(&dir.join("src").join("main.nova"));
    let mut client = Client::start(&dir, false);
    open(&mut client, &uri, main);
    let found = actions(&mut client, &uri, at(main, "fn main", 0), Some(&["source.organizeImports"]));
    let action = titled(&found, "Organize imports");
    assert_eq!(
        applied(main, &edits_in(action, &uri)),
        "import extra\nimport shapes\n\nfn main() {\n    let n: Int = \"no\"\n    println(\"${twice()}\")\n}\n"
    );
}

#[test]
fn only_source_returns_organize_imports_alone() {
    let main = "import shapes\nimport extra\n\nfn main() {\n    let x = 1\n    x = 2\n    println(\"${x} ${twice()} ${unused()}\")\n}\n";
    let dir = project(
        "organize-only",
        &[("main.nova", main), ("shapes.nova", SHAPES), ("extra.nova", EXTRA)],
    );
    let uri = file_uri(&dir.join("src").join("main.nova"));
    let mut client = Client::start(&dir, false);
    open(&mut client, &uri, main);
    let source = actions(&mut client, &uri, at(main, "x = 2", 0), Some(&["source"]));
    let kinds: Vec<&str> = source.iter().map(|a| a["kind"].as_str().unwrap()).collect();
    assert_eq!(kinds, ["source.organizeImports"]);
    let fixes = actions(&mut client, &uri, at(main, "x = 2", 0), Some(&["quickfix"]));
    assert!(
        !fixes.is_empty() && fixes.iter().all(|a| a["kind"] == "quickfix"),
        "{fixes:?}"
    );
}
```

- [ ] **Step 2: Run the tests to see them fail**

Run: `cd /d/Projects/nona/nova && cargo test -p nova-lsp organize 2>&1 | tail -3`
Expected: a compile error, `cannot find function 'edits' in this scope`.

- [ ] **Step 3: The judge, and the action**

Insert into `crates/nova-lsp/src/organize.rs` before `#[cfg(test)]`
(marker `T11-ORGANIZE`):

```rust
use std::collections::HashMap;
use std::path::Path;

use lsp_types as lsp;
use nova_diagnostics::{FileId, LineIndex, Severity, Span};
use nova_driver::Analysis;
use nova_fmt::{Group, ImportView, TextEdit, Verdict};
use nova_resolver::{Definitions, Index, ModuleId, Target};

use crate::analysis::Answer;
use crate::convert;
use crate::navigate::Locator;

/// The edits organizing `answer`'s file makes, or `None` when organizing
/// changes nothing.
pub fn edits(answer: &Answer) -> Option<Vec<TextEdit>> {
    let a = &answer.analysis;
    let text = a.db.get_source(answer.file)?;
    let removal = may_remove(a, answer.file);
    nova_fmt::organize(text, &|view| verdict(a, answer.file, view, removal))
}

/// The organize imports action for `answer`'s file (spec §5, §6).
// `WorkspaceEdit::changes` is a `HashMap` keyed by `lsp::Uri`, whose
// parsed form caches into a `Cell`; the key is never changed here.
#[allow(clippy::mutable_key_type)]
pub fn action(answer: &Answer, uri_of: &dyn Fn(&Path) -> String) -> Option<lsp::CodeAction> {
    let edits = edits(answer)?;
    let a = &answer.analysis;
    let text = a.db.get_source(answer.file)?;
    let uri = Locator::new(a, uri_of).uri(answer.file)?;
    let lines = LineIndex::new(text);
    let edits: Vec<lsp::TextEdit> = edits
        .into_iter()
        .map(|e| lsp::TextEdit {
            range: convert::range(&lines, e.start, e.end),
            new_text: e.text,
        })
        .collect();
    Some(lsp::CodeAction {
        title: "Organize imports".to_string(),
        kind: Some(lsp::CodeActionKind::SOURCE_ORGANIZE_IMPORTS),
        edit: Some(lsp::WorkspaceEdit {
            changes: Some(HashMap::from([(uri, edits)])),
            ..Default::default()
        }),
        ..Default::default()
    })
}

/// Whether unused imports may go (spec §6.4): no error in `file`, and no
/// lex or parse error in any file, since `keep_going` filters out the
/// E0001s a dropped item causes.
fn may_remove(a: &Analysis, file: FileId) -> bool {
    !a.diagnostics.iter().any(|d| {
        matches!(d.code.as_str(), "L0001" | "P0001")
            || (d.severity == Severity::Error && d.labels.iter().any(|l| l.span.file == file))
    })
}

/// One import's group and what of it is unused (spec §6.2, §6.4).
fn verdict(a: &Analysis, file: FileId, view: &ImportView, removal: bool) -> Verdict {
    let first = Span::new(view.path_start, view.path_start + view.first.len() as u32, file);
    let module = a.index.as_ref().and_then(|index| module_at(index, first));
    let group = match module {
        Some(m) if !beside(a, file, m) => Group::Dependency,
        _ => Group::Module,
    };
    let kept = Verdict {
        group,
        unused_glob: false,
        unused_names: Vec::new(),
    };
    let (Some(m), Some(index), Some(defs), true) =
        (module, a.index.as_ref(), a.definitions.as_ref(), removal)
    else {
        return kept;
    };
    if view.glob {
        let used = index.occurrences.iter().any(|o| {
            o.span.file == file && o.span != first && declared_in(a, defs, &o.target, m)
        });
        Verdict {
            unused_glob: !used,
            ..kept
        }
    } else {
        let unused_names = view
            .names
            .iter()
            .filter(|(name, start)| {
                !list_name_used(index, Span::new(*start, *start + name.len() as u32, file))
            })
            .map(|(name, _)| name.to_string())
            .collect();
        Verdict {
            unused_names,
            ..kept
        }
    }
}

/// The module the import whose first segment is at `first` names.
fn module_at(index: &Index, first: Span) -> Option<ModuleId> {
    index.occurrences.iter().find_map(|o| match o.target {
        Target::Module(m) if o.span == first => Some(m),
        _ => None,
    })
}

/// Whether module `m`'s file is in `file`'s directory (plan decision 11).
fn beside(a: &Analysis, file: FileId, m: ModuleId) -> bool {
    let dir = |f: FileId| {
        a.modules
            .iter()
            .find(|(id, _)| *id == f)
            .and_then(|(_, p)| p.parent().map(Path::to_path_buf))
    };
    let Some((target, _)) = a.modules.get(m.0 as usize) else {
        return false;
    };
    dir(*target).is_some() && dir(*target) == dir(file)
}

/// Whether the list name at `at` is used (spec §6.4): an occurrence in its
/// file, other than the import's own, targets what it binds. A call of a
/// trait's method uses the trait, and a variant its sum type.
fn list_name_used(index: &Index, at: Span) -> bool {
    let bound: Vec<Target> = index
        .occurrences
        .iter()
        .filter(|o| o.span == at)
        .map(|o| o.target)
        .collect();
    index
        .occurrences
        .iter()
        .filter(|o| o.span.file == at.file && o.span != at)
        .any(|o| {
            bound.iter().any(|b| match (*b, o.target) {
                (b, t) if b == t => true,
                (Target::Def(t), Target::TraitMethod(u, _)) => t == u,
                (Target::Def(s), Target::Variant(u, _)) => s == u,
                _ => false,
            })
        })
}

/// Whether `target` is declared in module `m`'s file: an item, a variant
/// or field of its types, or a method of its traits.
fn declared_in(a: &Analysis, defs: &Definitions, target: &Target, m: ModuleId) -> bool {
    let Some((file, _)) = a.modules.get(m.0 as usize) else {
        return false;
    };
    let id = match *target {
        Target::Def(id) | Target::Variant(id, _) | Target::Field(id, _) | Target::TraitMethod(id, _) => id,
        _ => return false,
    };
    defs.def(id).span.file == *file
}

```

In `crates/nova-lsp/src/code_action.rs`'s `code_actions`, replace
`        out.extend(quick_fixes(answer, start, end, uri_of));\n    }\n    out`
with (marker `T11-ACTIONS`):

```rust
        out.extend(quick_fixes(answer, start, end, uri_of));
    }
    if wanted(&lsp::CodeActionKind::SOURCE_ORGANIZE_IMPORTS, only) {
        if let Some(action) = crate::organize::action(answer, uri_of) {
            out.push(lsp::CodeActionOrCommand::CodeAction(action));
        }
    }
    out
```

- [ ] **Step 4: Run the tests**

Run: `cd /d/Projects/nona/nova && cargo test -p nova-lsp organize 2>&1 | tail -3`
Expected: `test result: ok. 6 passed; 0 failed`.

Run: `cd /d/Projects/nona/nova && cargo test -p nova-cli --test lsp_fixes 2>&1 | tail -3`
Expected: `test result: ok. 11 passed; 0 failed`.

- [ ] **Step 5: Commit**

Write `$P/msg-34b-11.txt`:

```text
nova-lsp: organize imports

The server judges each import for nova-fmt's organize: a dependency
when the module it names is in another directory, unused when nothing
in the file uses what it binds, a trait's method calls and a sum type's
variants counting as uses. Nothing is removed from a file with an
error, or while any file has a parse error (spec 3.4b §6.2, §6.4).

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
```

Run: `cd /d/Projects/nona/nova && cargo fmt --all && git add crates/nova-lsp/src/organize.rs && git add -u && git commit -q -F $P/msg-34b-11.txt && git log -1 --format=%s`
Expected: `nova-lsp: organize imports`

---

### Task 12: Semantic tokens

**Files:**
- Create: `crates/nova-lsp/src/tokens.rs`
- Modify: `crates/nova-lsp/src/lib.rs` (module list, imports,
  `capabilities`, `Server::request`)
- Modify: `crates/nova-resolver/src/index.rs` (`rank` becomes public)
- Test: `crates/nova-lsp/src/tokens.rs` (unit), `crates/nova-cli/tests/lsp_fixes.rs`

**Interfaces:**
- Consumes: `Index::{occurrences, locals}` (3.4a, Task 4); `index::rank`.
- Produces: `tokens::legend() -> lsp::SemanticTokensLegend`,
  `tokens::tokens(answer: &Answer) -> Vec<lsp::SemanticToken>`.

- [ ] **Step 1: Write the failing tests**

Create `crates/nova-lsp/src/tokens.rs` holding its doc and tests only
(marker `T12-UNIT-TESTS`):

```rust
//! Semantic tokens: every name the index records, by what it means (spec
//! `docs/superpowers/specs/2026-10-10-phase-3-4b-fixes-and-colour-design.md`
//! §7).

#[cfg(test)]
mod tests {
    use super::*;
    use crate::workspace::Overlay;

    fn answer(name: &str, files: &[(&str, &str)]) -> Answer {
        let dir = std::env::temp_dir().join(format!("nova-lsp-tokens-{name}"));
        let mut overlay = Overlay::default();
        for (file, text) in files {
            overlay = overlay.with(&dir.join(file), text.to_string());
        }
        let options = crate::analysis::options(None, true);
        crate::analysis::answering(&dir.join(files[0].0), &overlay, &options).expect("an answer")
    }

    /// Each token: its line, its UTF-16 column, its byte offset, its type's
    /// name and its modifiers' names.
    type Decoded = Vec<(u32, u32, usize, String, Vec<String>)>;

    fn decoded(text: &str, tokens: &[lsp::SemanticToken]) -> Decoded {
        let lines = LineIndex::new(text);
        let legend = legend();
        let (mut line, mut col) = (0, 0);
        tokens
            .iter()
            .map(|t| {
                line += t.delta_line;
                col = if t.delta_line == 0 { col + t.delta_start } else { t.delta_start };
                let ty = legend.token_types[t.token_type as usize].as_str().to_string();
                let mods = legend
                    .token_modifiers
                    .iter()
                    .enumerate()
                    .filter(|(i, _)| t.token_modifiers_bitset & (1 << i) != 0)
                    .map(|(_, m)| m.as_str().to_string())
                    .collect();
                (line, col, lines.offset(line, col) as usize, ty, mods)
            })
            .collect()
    }

    /// The token at the `n`th whole `word` of `text`: its type and modifiers.
    #[track_caller]
    fn token(found: &Decoded, text: &str, word: &str, n: usize) -> (String, Vec<String>) {
        let ident = |c: Option<char>| c.is_some_and(|c| c.is_alphanumeric() || c == '_');
        let start = text
            .match_indices(word)
            .map(|(i, _)| i)
            .filter(|&i| !ident(text[..i].chars().next_back()) && !ident(text[i + word.len()..].chars().next()))
            .nth(n)
            .unwrap_or_else(|| panic!("no `{word}` number {n}"));
        found
            .iter()
            .find(|t| t.2 == start)
            .map(|t| (t.3.clone(), t.4.clone()))
            .unwrap_or_else(|| panic!("no token at `{word}` number {n}: {found:?}"))
    }

    fn of(ty: &str, mods: &[&str]) -> (String, Vec<String>) {
        (ty.to_string(), mods.iter().map(|m| m.to_string()).collect())
    }

    const EVERY_ROW: &str = "import geometry::{origin}\n\nconst LIMIT: Int = 3\n\nrecord Point { x: Int }\n\ntype Shape =\n  | Empty\n\ntrait Show {\n    fn show(self) -> String\n}\n\nimpl Show for Point {\n    fn show(self) -> String {\n        \"p\"\n    }\n}\n\nfn first<T>(x: T) -> T {\n    x\n}\n\nfn main() {\n    let mut n = LIMIT\n    n = n + origin()\n    let p = Point { x: n }\n    let s = Shape::Empty\n    let o = Some(1)\n    println(p.show())\n    let k = [1].len()\n}\n";

    #[test]
    fn every_row_of_the_table() {
        let a = answer(
            "every-row",
            &[
                ("main.nova", EVERY_ROW),
                ("geometry.nova", "pub fn origin() -> Int {\n    1\n}\n"),
            ],
        );
        let text = EVERY_ROW;
        let found = decoded(text, &tokens(&a));
        let decl = "declaration";
        assert_eq!(token(&found, text, "geometry", 0), of("namespace", &[]));
        assert_eq!(token(&found, text, "origin", 0), of("function", &[]));
        assert_eq!(token(&found, text, "LIMIT", 0), of("variable", &[decl, "readonly"]));
        assert_eq!(token(&found, text, "Int", 0), of("type", &["defaultLibrary"]));
        assert_eq!(token(&found, text, "Point", 0), of("struct", &[decl]));
        assert_eq!(token(&found, text, "x", 0), of("property", &[decl]));
        assert_eq!(token(&found, text, "Shape", 0), of("enum", &[decl]));
        assert_eq!(token(&found, text, "Empty", 0), of("enumMember", &[decl]));
        assert_eq!(token(&found, text, "Show", 0), of("interface", &[decl]));
        assert_eq!(token(&found, text, "show", 0), of("method", &[decl]));
        assert_eq!(token(&found, text, "show", 1), of("method", &[decl]));
        assert_eq!(token(&found, text, "first", 0), of("function", &[decl]));
        assert_eq!(token(&found, text, "T", 0), of("typeParameter", &[decl]));
        assert_eq!(token(&found, text, "x", 1), of("parameter", &[decl]));
        assert_eq!(token(&found, text, "n", 0), of("variable", &[decl, "mutable"]));
        assert_eq!(token(&found, text, "n", 1), of("variable", &["mutable"]));
        assert_eq!(token(&found, text, "LIMIT", 1), of("variable", &["readonly"]));
        assert_eq!(token(&found, text, "Some", 0), of("enumMember", &["defaultLibrary"]));
        assert_eq!(token(&found, text, "println", 0), of("function", &["defaultLibrary"]));
        assert_eq!(token(&found, text, "show", 2), of("method", &[]));
        assert_eq!(token(&found, text, "len", 0), of("method", &["defaultLibrary"]));
        // `self` is the grammar's.
        let at_self = text.find("self").unwrap();
        assert!(found.iter().all(|t| t.2 != at_self), "{found:?}");
    }

    #[test]
    fn one_token_per_span_the_value_wins() {
        let text = "record P { x: Int }\n\nfn main() {\n    let x = 1\n    let p = P { x }\n}\n";
        let found = decoded(text, &tokens(&answer("shorthand", &[("main.nova", text)])));
        assert_eq!(token(&found, text, "x", 2), of("variable", &[]));
        let at = text.rfind("x }").unwrap();
        assert_eq!(found.iter().filter(|t| t.2 == at).count(), 1, "{found:?}");
    }

    #[test]
    fn tokens_count_utf16_after_thai_and_an_emoji() {
        // Review Focus 2: `n` follows three Thai characters, one UTF-16 unit
        // each, and an emoji, two.
        let text = "fn f(a: String, b: Int) -> Int {\n    b\n}\n\nfn main() {\n    let n = 1\n    let s = f(\"ไทย😀\", n)\n}\n";
        let found = decoded(text, &tokens(&answer("utf16", &[("main.nova", text)])));
        let at = text.rfind("n)").unwrap();
        let t = found.iter().find(|t| t.2 == at).unwrap_or_else(|| panic!("{found:?}"));
        assert_eq!((t.0, t.1, t.3.as_str()), (6, 23, "variable"));
    }

    #[test]
    fn encode_writes_relative_deltas() {
        let found = encode("ab cd\nef", &[(0, 2, 0, 0), (3, 5, 1, 1), (6, 8, 2, 0)]);
        let raw: Vec<(u32, u32, u32, u32, u32)> = found
            .iter()
            .map(|t| (t.delta_line, t.delta_start, t.length, t.token_type, t.token_modifiers_bitset))
            .collect();
        assert_eq!(raw, [(0, 0, 2, 0, 0), (0, 3, 2, 1, 1), (1, 0, 2, 2, 0)]);
    }
}
```

In `crates/nova-lsp/src/lib.rs`, replace `mod std_cache;` with
`mod std_cache;\nmod tokens;`.

Append to `crates/nova-cli/tests/lsp_fixes.rs` (marker `T12-TESTS`):

```rust

// === Task 12: semantic tokens (spec §7) ===

/// The tokens of `uri`, decoded: line, UTF-16 column, length, type index,
/// modifier bits.
fn tokens(client: &mut Client, uri: &str) -> Vec<(u64, u64, u64, u64, u64)> {
    let response = client.request(
        "textDocument/semanticTokens/full",
        json!({ "textDocument": { "uri": uri } }),
    );
    let data: Vec<u64> = response["result"]["data"]
        .as_array()
        .unwrap_or_else(|| panic!("{response}"))
        .iter()
        .map(|v| v.as_u64().unwrap())
        .collect();
    let (mut line, mut col) = (0, 0);
    data.chunks(5)
        .map(|t| {
            line += t[0];
            col = if t[0] == 0 { col + t[1] } else { t[1] };
            (line, col, t[2], t[3], t[4])
        })
        .collect()
}

/// The legend's indices (spec §7.1).
const FUNCTION: u64 = 10;
const VARIABLE: u64 = 7;
const DECLARATION: u64 = 1;
const DEFAULT_LIBRARY: u64 = 4;
const MUTABLE: u64 = 8;

#[test]
fn the_server_advertises_semantic_tokens() {
    let dir = project("tokens-capability", &[("main.nova", "fn main() {}\n")]);
    let client = Client::start(&dir, false);
    let provider = &client.initialized["capabilities"]["semanticTokensProvider"];
    assert_eq!(provider["full"], true, "{provider}");
    assert_eq!(provider["legend"]["tokenTypes"][0], "namespace", "{provider}");
    assert_eq!(provider["legend"]["tokenModifiers"][3], "mutable", "{provider}");
}

#[test]
fn semantic_tokens_name_by_name() {
    let main = "fn main() {\n    let mut x = 1\n    x = x + 1\n    println(\"${x}\")\n}\n";
    let dir = project("tokens", &[("main.nova", main)]);
    let uri = file_uri(&dir.join("src").join("main.nova"));
    let mut client = Client::start(&dir, false);
    open(&mut client, &uri, main);
    let found = tokens(&mut client, &uri);
    assert!(found.contains(&(0, 3, 4, FUNCTION, DECLARATION)), "{found:?}");
    assert!(found.contains(&(1, 12, 1, VARIABLE, DECLARATION | MUTABLE)), "{found:?}");
    assert!(found.contains(&(2, 4, 1, VARIABLE, MUTABLE)), "{found:?}");
    assert!(found.contains(&(3, 4, 7, FUNCTION, DEFAULT_LIBRARY)), "{found:?}");
}

#[test]
fn semantic_tokens_in_a_file_with_an_error() {
    let main = "fn main() {\n    let x = 1\n    let y = nope + x\n}\n";
    let dir = project("tokens-error", &[("main.nova", main)]);
    let uri = file_uri(&dir.join("src").join("main.nova"));
    let mut client = Client::start(&dir, false);
    open(&mut client, &uri, main);
    let found = tokens(&mut client, &uri);
    assert!(found.contains(&(2, 19, 1, VARIABLE, 0)), "{found:?}");
    assert!(!found.iter().any(|t| t.0 == 2 && t.1 == 12), "an unresolved name: {found:?}");
}
```

- [ ] **Step 2: Run the tests to see them fail**

Run: `cd /d/Projects/nona/nova && cargo test -p nova-lsp tokens 2>&1 | tail -3`
Expected: a compile error, `cannot find function 'tokens' in this scope`.

- [ ] **Step 3: The tokens**

In `crates/nova-resolver/src/index.rs`, replace
`fn rank(defs: &Definitions, target: &Target) -> u8 {` with
`pub fn rank(defs: &Definitions, target: &Target) -> u8 {`, adding
`/// How \`at\` ranks meanings that share a span: value, type, trait, field\n/// (3.4a spec §3.1). Semantic tokens use the same order (3.4b §7.3).`
above it if it has no doc comment.

Insert into `crates/nova-lsp/src/tokens.rs` before `#[cfg(test)]`
(marker `T12-TOKENS`):

```rust
use lsp_types as lsp;
use nova_diagnostics::LineIndex;
use nova_driver::Analysis;
use nova_resolver::{DefId, DefKind, Definitions, Index, Occurrence, Role, Target};

use crate::analysis::Answer;

/// The token types, in the legend's order (spec §7.1).
const TYPES: [lsp::SemanticTokenType; 12] = [
    lsp::SemanticTokenType::NAMESPACE,
    lsp::SemanticTokenType::TYPE,
    lsp::SemanticTokenType::STRUCT,
    lsp::SemanticTokenType::ENUM,
    lsp::SemanticTokenType::INTERFACE,
    lsp::SemanticTokenType::TYPE_PARAMETER,
    lsp::SemanticTokenType::PARAMETER,
    lsp::SemanticTokenType::VARIABLE,
    lsp::SemanticTokenType::PROPERTY,
    lsp::SemanticTokenType::ENUM_MEMBER,
    lsp::SemanticTokenType::FUNCTION,
    lsp::SemanticTokenType::METHOD,
];

const NAMESPACE: u32 = 0;
const TYPE: u32 = 1;
const STRUCT: u32 = 2;
const ENUM: u32 = 3;
const INTERFACE: u32 = 4;
const TYPE_PARAMETER: u32 = 5;
const PARAMETER: u32 = 6;
const VARIABLE: u32 = 7;
const PROPERTY: u32 = 8;
const ENUM_MEMBER: u32 = 9;
const FUNCTION: u32 = 10;
const METHOD: u32 = 11;

/// The modifiers' bits, in the legend's order (spec §7.1).
const DECLARATION: u32 = 1;
const READONLY: u32 = 2;
const DEFAULT_LIBRARY: u32 = 4;
const MUTABLE: u32 = 8;

/// The legend the server advertises (spec §7.1).
pub fn legend() -> lsp::SemanticTokensLegend {
    lsp::SemanticTokensLegend {
        token_types: TYPES.to_vec(),
        token_modifiers: vec![
            lsp::SemanticTokenModifier::DECLARATION,
            lsp::SemanticTokenModifier::READONLY,
            lsp::SemanticTokenModifier::DEFAULT_LIBRARY,
            lsp::SemanticTokenModifier::new("mutable"),
        ],
    }
}

/// The tokens of `answer`'s file (spec §7): one per span the index
/// records, the meaning 3.4a's ranking puts first; none for `self` or an
/// unresolved name.
pub fn tokens(answer: &Answer) -> Vec<lsp::SemanticToken> {
    let a = &answer.analysis;
    let (Some(index), Some(defs), Some(text)) = (
        a.index.as_ref(),
        a.definitions.as_ref(),
        a.db.get_source(answer.file),
    ) else {
        return Vec::new();
    };
    let mut found: Vec<&Occurrence> = index
        .occurrences
        .iter()
        .filter(|o| o.span.file == answer.file && o.span.start < o.span.end)
        .collect();
    found.sort_by_key(|o| (o.span.start, o.span.end, nova_resolver::index::rank(defs, &o.target)));
    found.dedup_by_key(|o| (o.span.start, o.span.end));
    let mut spans: Vec<(u32, u32, u32, u32)> = Vec::new();
    for o in found {
        let Some(name) = text.get(o.span.start as usize..o.span.end as usize) else {
            continue;
        };
        if name == "self" || name.contains('\n') {
            continue;
        }
        let (ty, mods) = classify(a, defs, index, o);
        spans.push((o.span.start, o.span.end, ty, mods));
    }
    encode(text, &spans)
}

/// A name's token type and modifiers (spec §7.2).
fn classify(a: &Analysis, defs: &Definitions, index: &Index, o: &Occurrence) -> (u32, u32) {
    let std = |id: DefId| {
        let file = defs.def(id).span.file;
        if a.db.get_name(file).is_some_and(|n| n.starts_with("<std/")) {
            DEFAULT_LIBRARY
        } else {
            0
        }
    };
    let (ty, mods) = match o.target {
        Target::Def(id) => match defs.def(id).kind {
            DefKind::Fn { .. } | DefKind::ExternFn { .. } => (FUNCTION, std(id)),
            DefKind::Method { .. } => (METHOD, std(id)),
            DefKind::Record { .. } => (STRUCT, std(id)),
            DefKind::Sum { .. } => (ENUM, std(id)),
            DefKind::Trait { .. } => (INTERFACE, std(id)),
            DefKind::Const { .. } => (VARIABLE, std(id) | READONLY),
            DefKind::AssocType { .. } => (TYPE, std(id)),
        },
        Target::Variant(sum, _) => (ENUM_MEMBER, std(sum)),
        Target::Field(record, _) => (PROPERTY, std(record)),
        Target::TraitMethod(tr, _) => (METHOD, std(tr)),
        Target::Local(decl) => {
            let flags = index.locals.get(&decl).copied();
            let ty = if flags.is_some_and(|f| f.parameter) {
                PARAMETER
            } else {
                VARIABLE
            };
            (ty, if flags.is_some_and(|f| f.mutable) { MUTABLE } else { 0 })
        }
        Target::TypeParam(_) => (TYPE_PARAMETER, 0),
        Target::Module(_) => (NAMESPACE, 0),
        Target::Builtin(_) => (FUNCTION, DEFAULT_LIBRARY),
        Target::BuiltinMethod(_) => (METHOD, DEFAULT_LIBRARY),
        Target::Primitive(_) => (TYPE, DEFAULT_LIBRARY),
    };
    let declaration = if o.role == Role::Declaration { DECLARATION } else { 0 };
    (ty, mods | declaration)
}

/// `spans` (start, end, type, modifiers), sorted by start, as the
/// protocol's relative tokens, columns and lengths in UTF-16 (spec §7.3).
fn encode(text: &str, spans: &[(u32, u32, u32, u32)]) -> Vec<lsp::SemanticToken> {
    let lines = LineIndex::new(text);
    let (mut prev_line, mut prev_col) = (0u32, 0u32);
    let mut out = Vec::new();
    for &(start, end, ty, mods) in spans {
        let (line, col) = lines.position(start);
        let length = text[start as usize..end as usize].encode_utf16().count() as u32;
        let delta_line = line - prev_line;
        let delta_start = if delta_line == 0 { col - prev_col } else { col };
        out.push(lsp::SemanticToken {
            delta_line,
            delta_start,
            length,
            token_type: ty,
            token_modifiers_bitset: mods,
        });
        prev_line = line;
        prev_col = col;
    }
    out
}

```

In `crates/nova-lsp/src/lib.rs`:
- replace `    CodeActionRequest, Completion, Formatting, GotoDefinition, HoverRequest,\n    PrepareRenameRequest, References,`
  with `    CodeActionRequest, Completion, Formatting, GotoDefinition, HoverRequest,\n    PrepareRenameRequest, References, SemanticTokensFullRequest,`;
- in `capabilities`, after `code_action_provider`'s closing `)),` insert:

```rust
        semantic_tokens_provider: Some(
            lsp::SemanticTokensServerCapabilities::SemanticTokensOptions(
                lsp::SemanticTokensOptions {
                    legend: tokens::legend(),
                    range: Some(false),
                    full: Some(lsp::SemanticTokensFullOptions::Bool(true)),
                    work_done_progress_options: Default::default(),
                },
            ),
        ),
```

- in `Server::request`, before the `_ => Response::new_err(` arm, insert
  (marker `T12-REQUEST`):

```rust
            SemanticTokensFullRequest::METHOD => {
                match serde_json::from_value::<lsp::SemanticTokensParams>(request.params) {
                    Ok(p) => {
                        let data = self
                            .answer_of(&p.text_document.uri)
                            .map(|(answer, _, _)| tokens::tokens(&answer))
                            .unwrap_or_default();
                        Response::new_ok(
                            request.id,
                            lsp::SemanticTokensResult::Tokens(lsp::SemanticTokens {
                                result_id: None,
                                data,
                            }),
                        )
                    }
                    Err(e) => invalid(request.id, e),
                }
            }
```

- [ ] **Step 4: Run the tests**

Run: `cd /d/Projects/nona/nova && cargo test -p nova-lsp tokens 2>&1 | tail -3`
Expected: `test result: ok. 4 passed; 0 failed`.

Run: `cd /d/Projects/nona/nova && cargo test -p nova-cli --test lsp_fixes 2>&1 | tail -3`
Expected: `test result: ok. 14 passed; 0 failed`.

A row of `every_row_of_the_table` that fails names the occurrence the
index recorded for it. If the index records the name with another target
than spec §7.2's row assumes (a trait method's call as `Def`, say), the
table follows the index and the ruling says so.

- [ ] **Step 5: Commit**

Write `$P/msg-34b-12.txt`:

```text
nova-lsp: semantic tokens

The server answers whole-document semantic tokens: every name the index
records, by what it means, with declaration, readonly, defaultLibrary
and a custom mutable modifier; one token per span, by 3.4a's ranking;
none for self or an unresolved name (spec 3.4b §7).

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
```

Run: `cd /d/Projects/nona/nova && cargo fmt --all && git add crates/nova-lsp/src/tokens.rs && git add -u && git commit -q -F $P/msg-34b-12.txt && git log -1 --format=%s`
Expected: `nova-lsp: semantic tokens`

---

### Task 13: The gate test, and latency

**Files:**
- Modify: `crates/nova-cli/tests/lsp_fixes.rs`, `crates/nova-cli/tests/lsp.rs`
  (`Figures`, `measure`, `requests_stay_within_the_ci_bound`,
  `latency_on_05_json_api`)

**Interfaces:**
- Consumes: Tasks 10-12.
- Produces: the latency figures Task 15's ADR records.

- [ ] **Step 1: The gate test**

Append to `crates/nova-cli/tests/lsp_fixes.rs` (marker `T13-GATE`):

```rust

// === Task 13: the gate (spec §10) ===

#[test]
fn the_gate_on_a_project_with_a_dependency() {
    // A quick fix importing from the dependency, "make public" editing a
    // second file, organize imports with both groups, and semantic tokens.
    let main = "import shapes::{helper}\nimport tidy\n\nfn main() {\n    println(\"${total()} ${helper()}\")\n}\n";
    let shapes = "fn helper() -> Int {\n    2\n}\n\npub fn twice() -> Int {\n    side() * helper()\n}\n";
    let tidy = "import shapes\nimport geom\nimport extra\n\npub fn total() -> Int {\n    area() + twice()\n}\n";
    let (app, _geom) = lsp_client::app_with_main(
        "gate",
        "pub fn area() -> Int {\n    1\n}\n\npub fn side() -> Int {\n    2\n}\n",
        main,
    );
    let src = app.join("src");
    std::fs::write(src.join("shapes.nova"), shapes).unwrap();
    std::fs::write(src.join("tidy.nova"), tidy).unwrap();
    std::fs::write(src.join("extra.nova"), EXTRA).unwrap();
    let (main_uri, shapes_uri, tidy_uri) = (
        file_uri(&src.join("main.nova")),
        file_uri(&src.join("shapes.nova")),
        file_uri(&src.join("tidy.nova")),
    );
    let mut client = Client::start(&app, false);
    open(&mut client, &main_uri, main);
    open(&mut client, &shapes_uri, shapes);
    open(&mut client, &tidy_uri, tidy);
    let start = json!({ "line": 0, "character": 0 });

    let found = actions(&mut client, &shapes_uri, at(shapes, "side()", 0), None);
    let import = titled(&found, "Import `side` from `geom`");
    assert_eq!(
        edits_in(import, &shapes_uri),
        [json!({ "range": { "start": start, "end": start }, "newText": "import geom::{side}\n\n" })]
    );

    let found = actions(&mut client, &main_uri, at(main, "helper}", 0), None);
    let public = titled(&found, "Make `helper` public in `shapes`");
    assert_eq!(
        edits_in(public, &shapes_uri),
        [json!({ "range": { "start": start, "end": start }, "newText": "pub " })]
    );

    let found = actions(&mut client, &tidy_uri, at(tidy, "pub fn", 0), Some(&["source.organizeImports"]));
    let organize = titled(&found, "Organize imports");
    assert_eq!(
        applied(tidy, &edits_in(organize, &tidy_uri)),
        "import geom\n\nimport shapes\n\npub fn total() -> Int {\n    area() + twice()\n}\n"
    );

    let found = tokens(&mut client, &tidy_uri);
    assert!(found.contains(&(1, 7, 4, 0, 0)), "`geom`, a namespace: {found:?}");
    assert!(found.contains(&(4, 7, 5, FUNCTION, DECLARATION)), "`total`: {found:?}");
    assert!(found.contains(&(5, 4, 4, FUNCTION, 0)), "`area`: {found:?}");
}
```

Run: `cd /d/Projects/nona/nova && cargo test -p nova-cli --test lsp_fixes the_gate 2>&1 | tail -3`
Expected: `test result: ok. 1 passed; 0 failed`. A *guard*: Tasks 10-12
made each part. Confirm it can fail: in `crates/nova-lsp/src/organize.rs`'s
`verdict`, change `Some(m) if !beside(a, file, m) => Group::Dependency,` to
`Some(_) => Group::Module,`, rerun, see it FAIL at the organize assert,
then `git checkout -- crates/nova-lsp/src/organize.rs`.

- [ ] **Step 2: Latency grows to eight requests**

In `crates/nova-cli/tests/lsp.rs`:
1. In `struct Figures`, after `    renames: Vec<u128>,` insert
   `    code_actions: Vec<u128>,\n    tokens: Vec<u128>,`.
2. In `measure`, replace
   `    let renames = timed(&mut client, n, "textDocument/rename", renaming);\n    Figures {`
   with (marker `T13-MEASURE`):

```rust
    let renames = timed(&mut client, n, "textDocument/rename", renaming);
    // 3.4b spec §9.8: code actions at a place without a diagnostic, and
    // the whole document's tokens.
    let acting = json!({
        "textDocument": { "uri": uri },
        "range": { "start": call, "end": call },
        "context": { "diagnostics": [] },
    });
    let code_actions = timed(&mut client, n, "textDocument/codeAction", acting);
    let tokens = timed(
        &mut client,
        n,
        "textDocument/semanticTokens/full",
        json!({ "textDocument": { "uri": uri } }),
    );
    Figures {
```

   and replace `        renames,\n    }` with `        renames,\n        code_actions,\n        tokens,\n    }`.
   `call` is the hover position `after(&text, "parts[n] = ")` computes;
   `json!` reads it by reference, so it is still there.
3. In `requests_stay_within_the_ci_bound`, replace
   `            && within(&f.renames, 4000),` with
   `            && within(&f.renames, 4000)\n            && within(&f.code_actions, 2000)\n            && within(&f.tokens, 2000),`,
   the format string `"{}; {}; {}; {}; {}; {}",` with
   `"{}; {}; {}; {}; {}; {}; {}; {}",`, and
   `        summary("renames", &f.renames)\n    );` with
   `        summary("renames", &f.renames),\n        summary("code actions", &f.code_actions),\n        summary("tokens", &f.tokens)\n    );`.
   Its comment becomes
   `    // Gate item 8, 3.4a's and 3.4b's: 2 s each with the debug binary, 4 s\n    // for a rename, which analyses twice.`
4. In `latency_on_05_json_api`, after
   `    println!("{}", summary("rename", &f.renames));` insert
   `    println!("{}", summary("code actions", &f.code_actions));\n    println!("{}", summary("semantic tokens", &f.tokens));`,
   and in its assert, replace `            &f.references\n        ]` with
   `            &f.references,\n            &f.code_actions,\n            &f.tokens\n        ]`.

Run: `cd /d/Projects/nona/nova && cargo test -p nova-cli --test lsp requests_stay_within 2>&1 | tail -3`
Expected: `test result: ok. 1 passed; 0 failed`.

- [ ] **Step 3: Measure the budget**

Run: `cd /d/Projects/nona/nova && cargo build --release -p nova-cli 2>&1 | tail -1 && ls -l target/release/nova.exe`
Expected: `Finished`, and the binary's size: ledger it.

Run three times:
`cd /d/Projects/nona/nova && cargo test --release -p nova-cli --test lsp -- --ignored --nocapture latency 2>&1 | grep -E "median|binary|test result"`
Expected: each run prints eight medians and maxima, all at or under 200
ms (400 ms for rename), and `1 passed`. Ledger the three runs' figures.
If code actions or tokens miss 200 ms, stop and ask the user (Global
Constraints).

- [ ] **Step 4: Commit**

Write `$P/msg-34b-13.txt`:

```text
Tests: the 3.4 gate, and latency over eight requests

One stdio test drives a project with a dependency through a quick fix
importing from it, "make public" editing a second file, organize
imports with both groups, and semantic tokens (spec 3.4b §10). The CI
bound and the budget measurement add code actions and semantic tokens.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
```

Run: `cd /d/Projects/nona/nova && cargo fmt --all && git add -u && git commit -q -F $P/msg-34b-13.txt && git log -1 --format=%s`
Expected: `Tests: the 3.4 gate, and latency over eight requests`

---

### Task 14: The VS Code extension

**Files:**
- Create: `tools/vscode-nova/test/fixture/src/fix.nova`
- Modify: `tools/vscode-nova/package.json` (description,
  `contributes.semanticTokenModifiers`), `tools/vscode-nova/README.md`,
  `tools/vscode-nova/test/suite/smoke.test.ts`

**Interfaces:**
- Consumes: Tasks 10 and 12, through the real `nova lsp`.
- Produces: nothing new.

- [ ] **Step 1: The fixture and the tests**

Create `tools/vscode-nova/test/fixture/src/fix.nova` (marker `T14-FIXTURE`):

```nova
fn bump() -> Int {
    let x = 1
    x = 2
    x
}
```

In `smoke.test.ts`, replace the first comment's lines
`// The extension's smoke test (spec §7.6; 3.4a §7): the language, a\n// diagnostic, a completion, a formatted document, a hover and a definition,\n// through the real \`nova lsp\`.`
with
`// The extension's smoke test (spec §7.6; 3.4a §7; 3.4b §8): the language, a\n// diagnostic, a completion, a formatted document, a hover, a definition,\n// semantic tokens and a quick fix, through the real \`nova lsp\`.`,
and before the final `});` insert (marker `T14-SMOKE`):

```ts

  it("colours a call as a function with a semantic token", async () => {
    const uri = vscode.Uri.file(path.join(fixture, "src", "navigate.nova"));
    await vscode.workspace.openTextDocument(uri);
    const legend = await eventually("a token legend", async () =>
      vscode.commands.executeCommand<vscode.SemanticTokensLegend>(
        "vscode.provideDocumentSemanticTokensLegend",
        uri,
      ),
    );
    const fn = legend.tokenTypes.indexOf("function");
    assert.ok(fn >= 0, legend.tokenTypes.join(", "));
    const data = await eventually("a function token", async () => {
      const found = await vscode.commands.executeCommand<vscode.SemanticTokens>(
        "vscode.provideDocumentSemanticTokens",
        uri,
      );
      if (found === undefined) {
        return undefined;
      }
      for (let i = 3; i < found.data.length; i += 5) {
        if (found.data[i] === fn) {
          return found.data;
        }
      }
      return undefined;
    });
    assert.ok(data.length >= 5);
  });

  it("offers a quick fix for a planted error", async () => {
    const uri = vscode.Uri.file(path.join(fixture, "src", "fix.nova"));
    const doc = await vscode.workspace.openTextDocument(uri);
    const at = doc.positionAt(doc.getText().indexOf("x = 2"));
    const titles = await eventually("a quick fix", async () => {
      const found = await vscode.commands.executeCommand<(vscode.CodeAction | vscode.Command)[]>(
        "vscode.executeCodeActionProvider",
        uri,
        new vscode.Range(at, at),
      );
      const names = (found ?? []).map((a) => a.title);
      return names.includes("Make `x` mutable") ? names : undefined;
    });
    assert.ok(titles.includes("Make `x` mutable"), titles.join(", "));
  });
```

- [ ] **Step 2: The manifest and the README**

In `tools/vscode-nova/package.json`:
- replace the description's
  `completion, formatting, hover, go to definition, find references and rename from nova lsp.`
  with
  `completion, formatting, hover, go to definition, find references, rename, quick fixes, organize imports and semantic highlighting from nova lsp.`;
- after the `"grammars": [ … ],` entry insert:

```json
    "semanticTokenModifiers": [
      {
        "id": "mutable",
        "description": "A local or parameter declared mut"
      }
    ],
```

In `tools/vscode-nova/README.md`, replace
`Colouring for \`.nova\` files, and from \`nova lsp\`: diagnostics, completion,\nformatting, hover, go to definition, find references and rename.`
with
`Colouring for \`.nova\` files, and from \`nova lsp\`: diagnostics, completion,\nformatting, hover, go to definition, find references, rename, quick\nfixes, organize imports and semantic highlighting. A local or parameter\ndeclared \`mut\` carries the \`mutable\` token modifier, which a theme can\nstyle.`.

- [ ] **Step 3: Check the TypeScript and the JSON**

Run: `cd /d/Projects/nona/nova/tools/vscode-nova && npx tsc --noEmit -p . && echo tsc-ok`
Expected: `tsc-ok`. (The packages are installed from 3.2; nothing is
downloaded.)

Run: `cd /d/Projects/nona/nova && python -X utf8 -c "import json; json.load(open('tools/vscode-nova/package.json', encoding='utf-8')); print('json-ok')"`
Expected: `json-ok`.

Running the smoke test downloads VS Code, which waits for the user's
word. CI runs it first (spec §9.9): ledger a ruling.

- [ ] **Step 4: Commit**

Write `$P/msg-34b-14.txt`:

```text
VS Code extension: semantic tokens and a quick fix

The smoke test also asks VS Code for a file's semantic tokens and finds
a function, and for the code actions at a planted E0060 and finds "Make
x mutable". The extension declares the mutable token modifier, and its
description and README list the new features (spec 3.4b §8).

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
```

Run: `cd /d/Projects/nona/nova && git add tools/vscode-nova/test/fixture/src/fix.nova && git add -u && git commit -q -F $P/msg-34b-14.txt && git log -1 --format=%s`
Expected: `VS Code extension: semantic tokens and a quick fix`

---

### Task 15: Records

**Files:**
- Create: `docs/adr/0033-fixes-and-colour-in-the-language-server.md`
- Modify:
  - `nova-spec/40-TOOLING.md` (§2.1's and §3.1's notes);
  - `docs/phase-3-plan.md` (the 3.4 entry);
  - `docs/adr/0028-the-formatter.md`, `docs/adr/0029-the-language-server.md`;
  - `agent.md` (after the 3.4a note that follows the crate table);
  - `CHANGELOG.md` (`[Unreleased]`: Added, Changed);
  - `README.md` ("Editor support");
  - `ARCHITECTURE.md` (the crate rows).
- Modify: whatever the sweep finds.

**Interfaces:**
- Consumes: Task 13's latency figures, from the ledger.

- [ ] **Step 1: ADR 0033**

Create `docs/adr/0033-fixes-and-colour-in-the-language-server.md`, with
the figures from Task 13's three ledgered runs in the table (marker
`T15-ADR`):

```markdown
# ADR 0033 — Fixes and colour in the language server

## Status

Accepted, 2026-10-10 (Phase 3.4b, branch `phase-3-4b-fixes-colour`; spec
`docs/superpowers/specs/2026-10-10-phase-3-4b-fixes-and-colour-design.md`).

## Context

3.4a gave `nova lsp` hover, go to definition, find references and rename,
from an index the type checker records (ADR 0032). The phase plan's 3.4
entry also asks for suggested edits on `Diagnostic`, code actions that
offer them, organize imports in `40-TOOLING.md` §2.1's order, and semantic
highlighting. On 2026-10-10 the user put those in 3.4b.

## Decision

1. **Fixes are made where the error is found.**
   - `Diagnostic` gains `fixes`: a title and text edits, which may reach
     another file.
   - The resolver and the checker attach them as they raise the errors,
     since each site knows what its fix needs. The three fixes placed by
     lines read the source text from the `FileDb`.
   - The fixes are the determined ones (the user, 2026-10-10): make a
     `let` or a parameter mutable (E0060); import a name exactly one
     importable module exports, of the kind its site needs; "did you
     mean", within rustc's bound; make an item public in the importer's
     own package; remove an unreachable arm (E0021).
2. **The command line prints each fix as `= help:`,** and E0060's fix
   replaces its note.
3. **Code actions** offer each fix of a diagnostic the request overlaps,
   once, and organize imports. A request analyses afresh, but no offered
   fix is re-checked: tests apply every kind of fix and analyse again, and
   a sweep applies fixes on 364 broken programs.
4. **Organize imports** gathers a file's imports into one block: the
   dependencies, then the project's own modules, a blank line between.
   That is §2.1's "std first then third-party", in a Nova whose std needs
   no import. Imports of one module merge. Unused imports go only from a
   file without errors, while no file has a parse error.
5. **`nova fmt` keeps blank-line-separated import groups,** as gofmt does
   (the user, 2026-10-10), so format on save keeps organize imports'
   groups (ADR 0028's note).
6. **Semantic tokens cover names only,** for whole documents, by what the
   index says each name means.
7. **The budgets** are 200 ms for code actions and semantic tokens, for the
   median and the maximum of 20 on `05-json-api` in a release build. CI's
   bound is 2 s with the debug binary. Measured on 2026-10-10:

   | Run | Edit, median / max | Completion | Hover | Definition | References | Rename | Code actions | Tokens | Binary |
   |---|---|---|---|---|---|---|---|---|---|
   | 1 | … | … | … | … | … | … | … | … | … |
   | 2 | … | … | … | … | … | … | … | … | … |
   | 3 | … | … | … | … | … | … | … | … | … |

## Alternatives

- **A fixer pass after analysis.** It would recover context the checker
  had discarded, such as the names in scope at a point.
- **A structured suggestion, rendered later.** Each fix would be split
  across two crates, and `nova-diagnostics` would name the checker's
  concepts.
- **Re-checking each offered fix.** Code actions are asked for on every
  cursor move; the tests carry the promise instead.
- **The formatter grouping by kind.** `nova fmt --stdin` and loose files
  have no manifest to tell a package from a module.
- **Every token, not names only.** The TextMate grammar already colours
  keywords, literals and comments.

## Consequences

- A fix can expose an error at its own place: "did you mean" may name
  something of another type, and an imported function's parameter types
  are not checked against the call.
- The import fix sees only the modules the program loads. A module nothing
  imports yet is never offered.
- A user's file with a blank line between imports keeps it, and its
  groups are sorted separately.
- E0060's note for a match binding or a `for` variable still advises
  `let mut`, which does not parse there.
- Fix-all, `codeAction/resolve`, and delta and range token requests are
  not offered.

## References

- `docs/superpowers/specs/2026-10-10-phase-3-4b-fixes-and-colour-design.md`
- `docs/superpowers/plans/2026-10-10-phase-3-4b-fixes-and-colour.md`
- ADR 0028 (the formatter), ADR 0029 (the language server), ADR 0032
  (navigation)
- `docs/phase-3-plan.md` §4, the 3.4 entry
```

Fill each `…` cell with the ledgered median and maximum, such as `14 / 16
ms`, and the binary column as ADR 0032 does: path, bytes, build time. A
cell left as `…` is a records failure; Step 6 checks for it.

- [ ] **Step 2: The dated notes**

1. `nova-spec/40-TOOLING.md`, after §3.1's "Amended 2026-10-10 (branch
   `phase-3-4a-navigation`)" paragraph:

   ```text
   **Amended 2026-10-10 (branch `phase-3-4b-fixes-colour`):** 3.4b delivers
   code actions and semantic highlighting
   (`docs/superpowers/specs/2026-10-10-phase-3-4b-fixes-and-colour-design.md`;
   ADR 0033). Diagnostics carry fixes, made where each error is found,
   which `nova check` prints as `help:` lines and code actions offer;
   organize imports is a source action. Semantic tokens cover names only.
   With 3.4a, every Phase 3 row of this table is delivered.
   ```

2. `nova-spec/40-TOOLING.md`, after §2.1's "Amended 2026-10-07" list (the
   bullet ending `ranges.`), before `---`:

   ```text
   **Amended 2026-10-10 (branch `phase-3-4b-fixes-colour`):** a blank line
   between imports ends a group, as in gofmt, and each group is sorted on
   its own (ADR 0028). Organize imports writes two groups, the
   dependencies and then the project's own modules: "std first then
   third-party", in a Nova whose std needs no import (ADR 0033).
   ```

3. `docs/phase-3-plan.md`, at the end of the 3.4 entry, after its 3.4a
   note, indented two spaces as that note is:

   ```text
     **Amended 2026-10-10:** 3.4b, "Fixes and colour", is built
     (`docs/superpowers/specs/2026-10-10-phase-3-4b-fixes-and-colour-design.md`,
     branch `phase-3-4b-fixes-colour`; ADR 0033). With it the 3.4 gate is
     met: the scripted LSP tests cover each capability on a multi-file
     project with a dependency, and the extension's smoke test also checks
     semantic tokens and a quick fix.
   ```

4. `docs/adr/0028-the-formatter.md`, at the end of `## Consequences`:

   ```text
   - **Amended 2026-10-10 (branch `phase-3-4b-fixes-colour`):** a blank
     line between imports now ends a group, as in gofmt, and each group is
     sorted on its own. The self-check is unchanged: a group's sort is a
     sort within its run (ADR 0033).
   ```

5. `docs/adr/0029-the-language-server.md`, after its 3.4a note:

   ```text
     **Amended 2026-10-10 (branch `phase-3-4b-fixes-colour`):** 3.4b adds
     code actions and semantic tokens on the same engine, each request
     analysing afresh with the index on (ADR 0033).
   ```

6. `agent.md`, after the "**Amended 2026-10-10 (branch
   `phase-3-4a-navigation`):** `nova-lsp` also …" paragraph:

   ```text
   **Amended 2026-10-10 (branch `phase-3-4b-fixes-colour`):** a
   `nova-diagnostics` `Diagnostic` carries fixes, which `nova-resolver`
   and `nova-typeck` attach and `nova-lsp` offers as code actions;
   `nova-fmt` builds organize imports' block; `nova-lsp` also answers
   semantic tokens (`docs/adr/0033-fixes-and-colour-in-the-language-server.md`).
   ```

- [ ] **Step 3: The CHANGELOG, the README and ARCHITECTURE**

1. `CHANGELOG.md`, at the end of `[Unreleased]`'s first `### Added` list,
   after the "Navigation in the editor" entry:

   ```text
   - **Fixes and colour in the editor.** Diagnostics suggest fixes: make a
     local mutable, import a name, "did you mean", make an item public,
     and remove an unreachable arm. `nova check` prints each as a `help:`
     line, and `nova lsp` offers each as a quick fix. `nova lsp` also
     organizes imports, the dependencies first and then the project's own
     modules, and colours every name by what it means. ADR 0033.
   ```

   and at the start of the `### Changed` list that follows it:

   ```text
   - **`nova fmt` keeps a blank line between imports,** as gofmt does, and
     sorts each group of imports on its own (ADR 0028).
   ```

2. `README.md`, "Editor support": replace "completion, formatting, hover,
   go to definition, find references and\nrename." with "completion,
   formatting, hover, go to definition, find references,\nrename, quick
   fixes, organize imports and semantic highlighting.", rewrapped to the
   paragraph's width.
3. `ARCHITECTURE.md`'s crate rows:
   - `nova-diagnostics`: "Shared error reporting infrastructure, and the
     fixes a diagnostic suggests";
   - `nova-typeck`: "Type inference and checking (HM + extensions);
     records the language server's index of names when asked, and attaches
     fixes to its errors";
   - `nova-fmt`: "The formatter: prints the AST in one fixed layout, keeps
     every comment, and refuses output that would change the program (ADR
     0028); organize imports' block";
   - `nova-lsp`: "The language server, `nova lsp`: diagnostics,
     completion, formatting, hover, definition, references, rename, code
     actions and semantic tokens over `nova_driver::analyze` (ADR 0029,
     ADR 0032, ADR 0033)".

Check that each code span stays on one line:

Run: ``cd /d/Projects/nona/nova && git diff -U0 -- CHANGELOG.md README.md ARCHITECTURE.md agent.md nova-spec/40-TOOLING.md docs/phase-3-plan.md docs/adr/0028-the-formatter.md docs/adr/0029-the-language-server.md | grep -E "^\+[^+]" | awk -F'`' 'NF % 2 == 0'``
Expected: no output. A printed line has an odd number of backticks, so it
opens a code span it does not close. Rewrap it so the span does not split.

- [ ] **Step 4: The sweep**

Claims this branch makes stale can sit in files it never touches. List
them by set difference:

Run: `cd /d/Projects/nona/nova && git grep -l -i -E "code action|semantic (token|highlight)|3\.4b|import run|within (each|a) run|blank lines? between imports" -- . ':!docs/superpowers' ':!Cargo.lock' ':!target' | sort > $P/sweep-all.txt; git diff --name-only main...HEAD | sort > $P/sweep-touched.txt; comm -23 $P/sweep-all.txt $P/sweep-touched.txt`
Expected: a list of files the branch has not touched.

Read each match in those files with `git grep -n -i -E "<the same
pattern>" -- <file>`. For each line, decide:
- it says code actions or semantic highlighting are still to come, or
  3.4b's;
- or it says imports sort as one run across blank lines;
- or neither.

If either of the first two, add a dated note (or correct a list that is
plainly a feature list) and ledger it. If neither, leave it and ledger the
file as checked. `git grep` is line-oriented, so also search for each
phrase alone ("code action", then "semantic"), and confirm each empty
result with a single-pattern `-c`.

- [ ] **Step 5: Commit**

Write `$P/msg-34b-15.txt`:

```text
Records for Phase 3.4b: ADR 0033, notes, CHANGELOG

ADR 0033 records fixes made where the error is found, the help lines,
code actions without a re-check, organize imports' groups, the
formatter's blank-line groups, names-only tokens and the measured
latency. Dated notes go in 40-TOOLING §2.1 and §3.1, the phase plan's
3.4 entry (3.4 is complete), ADRs 0028 and 0029, and agent.md; the
CHANGELOG, the README and ARCHITECTURE list the new features. <the
sweep's findings, one line each, or "The sweep found no other stale
claim.">

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
```

Replace the angle-bracketed sentence with the sweep's result before
committing, in the same Write call.

Run: `cd /d/Projects/nona/nova && git add docs/adr/0033-fixes-and-colour-in-the-language-server.md && git add -u && git commit -q -F $P/msg-34b-15.txt && git log -1 --format=%s`
Expected: `Records for Phase 3.4b: ADR 0033, notes, CHANGELOG`

- [ ] **Step 6: Scan the records**

Run: `cd /d/Projects/nona/nova && grep -n -F "| … |" docs/adr/0033-fixes-and-colour-in-the-language-server.md; grep -n -E "TBD|TODO|<the sweep" docs/adr/0033-fixes-and-colour-in-the-language-server.md CHANGELOG.md; git log -1 --format=%B | grep -c "<the sweep"`
Expected: no output from the greps, and `0`.

---

### Task 16: Final verification, mutants, and the PR body

**Files:**
- Create: `$P/pr-body.md`, outside the repository

**Interfaces:**
- Consumes: everything.
- Produces: the evidence the PR body states.

- [ ] **Step 1: The full suite on Windows**

Check that port 3000 is free:

Run: `netstat -ano | grep -E "[:.]3000 .*LISTENING"`
Expected: no output. If a process holds it, stop and ask the user. Never
stop it.

Run: `cd /d/Projects/nona/nova && cargo test --workspace --no-fail-fast > $P/full-windows.txt 2>&1; python -X utf8 $P/count.py $P/full-windows.txt`
Expected: `0 failed`. `main` at `1039f1e` had 1781 passed and 9 ignored.
The branch adds:
- 17 `nova-diagnostics` unit tests (Tasks 1-2);
- 3 resolver unit tests (Task 3);
- 31 fix tests in `fixes.rs` and 1 index test (Tasks 3-6);
- 2 command-line tests (Task 7);
- 2 formatter layout tests (Task 8) and 10 organize tests (Task 9);
- 6 organize and 4 token unit tests in `nova-lsp` (Tasks 11-12);
- 15 stdio tests in `lsp_fixes.rs` (Tasks 10-13).

That is 91, so 1872 passed. A difference is explained by name, or it is a
finding. Ledger the exact count.

- [ ] **Step 2: The full suite on Linux**

If Docker is running:

Run: `cd /d/Projects/nona/nova && bash /c/Users/SAKEER~1/AppData/Local/Temp/gcm/linux/run.sh test --workspace --no-fail-fast > $P/full-linux.txt 2>&1; python -X utf8 $P/count.py $P/full-linux.txt`
Expected: `0 failed`.

If it is not, the Linux run is CI's: ledger a ruling. Do not start Docker.

- [ ] **Step 3: Clippy, rustfmt and the minimum Rust**

Run: `cd /d/Projects/nona/nova && cargo clippy --workspace --all-targets -- -D warnings 2>&1 | tail -3`
Expected: `Finished`, no warning.

Run: `cd /d/Projects/nona/nova && cargo fmt --all -- --check; echo "exit $?"`
Expected: `exit 0`.

Run: `cd /d/Projects/nona/nova && RUSTFLAGS="-D warnings" cargo +1.78 check --locked --workspace 2>&1 | tail -3`
Expected: `Finished`.

Run: `cd /d/Projects/nona/nova && git diff main...HEAD -- Cargo.lock | wc -l`
Expected: `0`.

- [ ] **Step 4: The mutants (spec §9.7)**

For each, edit the code as described, run the named test, see it fail,
then undo with `git checkout -- <file>` and ledger the outcome:

| # | Mutant | File | Run | Expected |
|---|---|---|---|---|
| 1 | in `mutable_fix`, `Edit::insert(decl.start, …)` → `Edit::insert(decl.start.saturating_sub(4), …)`, which puts `mut ` before `let` | `crates/nova-typeck/src/check/fixes.rs` | `cargo test -p nova-driver --test fixes case_01` | FAIL: P0001 became more common |
| 2 | in `import_fix`, `let [(import, module)] = found.as_slice() else` → `let Some((import, module)) = found.first() else` | same | `cargo test -p nova-driver --test fixes case_17` | FAIL |
| 3 | in `closest`, `.max(3) / 3` → `.max(3) / 2` | `crates/nova-diagnostics/src/suggest.rs` | `cargo test -p nova-driver --test fixes case_26` | FAIL |
| 4 | in `merge`, sort by `(b.group, &a.path).cmp(&(a.group, &b.path))` | `crates/nova-fmt/src/organize.rs` | `cargo test -p nova-fmt --test organize two_groups` | FAIL |
| 5 | in `may_remove`, return `true` (keep the parameters used: `let _ = (a, file); true`) | `crates/nova-lsp/src/organize.rs` | `cargo test -p nova-lsp an_error_in_the_file_keeps` | FAIL |
| 6 | in `encode`, delete `prev_line = line;` and `prev_col = col;`, so positions are absolute | `crates/nova-lsp/src/tokens.rs` | `cargo test -p nova-cli --test lsp_fixes semantic_tokens_name_by_name` | FAIL |
| 7 | in `classify`, `Target::Builtin(_) => (FUNCTION, DEFAULT_LIBRARY),` → `(FUNCTION, 0)` | same | `cargo test -p nova-lsp every_row_of_the_table` | FAIL |
| 8 | in `code_actions`, `if std_cache::in_std_cache(path) \|\| in_registry(path) {` → `if std_cache::in_std_cache(path) {` | `crates/nova-lsp/src/code_action.rs` | `cargo test -p nova-cli --test lsp_fixes no_actions_inside` | FAIL |
| 9 | in `Printer::file`, delete the `&& !self.src.blank_line_in(…)` condition | `crates/nova-fmt/src/print/mod.rs` | `cargo test -p nova-fmt --test layout a_blank_line_keeps` | FAIL |

A mutant that compiles to nothing, or whose test panics for another
reason, has not been tested: read the failure and confirm it is the
assertion the mutant should break. Ledger each with its failing
assertion's line.

After the nine, confirm the tree is clean:

Run: `cd /d/Projects/nona/nova && git status --short`
Expected: no output.

- [ ] **Step 5: Write the PR body**

Write `$P/pr-body.md` with the Write tool. Keep local paths, process ids
and the user's name out of it. Sections, in order:

1. `## Phase 3.4b, "Fixes and colour"`: one paragraph on what a user gets,
   then the spec, the plan and ADR 0033, and that with it 3.4 is
   complete.
2. `### What changed`: the crates, as this plan's Architecture block
   lists them.
3. `### Changes in behaviour`:
   - the command line prints fixes as `= help:` lines, and E0060's note
     gives way to its fix, in its published LSP message too;
   - `nova fmt` keeps a blank line between imports, and sorts each group;
   - the server advertises code actions and semantic tokens;
   - `requests_stay_within_the_ci_bound` measures eight requests.
4. `### Tests`: a table of Windows (local) and Linux results, `main` at
   `1039f1e` against this branch. Linux comes from Step 2, or "from this
   PR's CI". Then the 31 fix cases, the sweep's fix count from Task 7,
   and "each of the spec's nine mutants fails its named test".
5. `### Latency`: the three runs' figures, as in ADR 0033.
6. `### Final review`: filled in after the review (the executing-plans
   skill's fix pass).
7. `### Decisions to review`: this plan's Decisions 1-17 by title, the
   spec's planning decisions 37-40, and every ledger `Ruling:` line, each
   with its cost if wrong.
8. `### Deferred minors`: from the final review.
9. The attribution line: `🤖 Generated with [Claude Code](https://claude.com/claude-code)`.

Run: `grep -n -i -E "AppData|SAKEER|/c/Users|C:.Users" $P/pr-body.md`
Expected: no output. (`.` stands for either slash, so the pattern holds
no backslash for the Bash tool to collapse.)

There is no commit in this task: `pr-body.md` is outside the repository.
Push and open the PR only after the final review and its fix pass, as the
standing workflow says.
