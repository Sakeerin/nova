# Phase 3.1, "Formatter", Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** `nova fmt` prints Nova source in one fixed layout, keeps every
comment, and refuses any output that would change the program. `std/` and
`examples/` are formatted with it, and CI keeps them that way.

**Architecture:**
- **The lexer** gains `lex_with_comments`, which also returns the comments it
  skips. `///` keeps its text, `////` becomes a plain comment, and an
  unterminated `/*` is an error.
- **The parser** attaches `///` lines to the items, members, fields and
  variants they precede, as `docs`, and rejects them anywhere else.
- **`nova-fmt`:**
  - walks the AST into a Wadler document (our own, about 250 lines);
  - reads what the AST forgets from the source: literal spellings,
    parentheses, blank lines, impl member order;
  - places comments with a cursor that takes them in source order;
  - re-parses its own output on every call, and refuses it if the AST or
    the comments changed.
- **`nova fmt`** formats a project, files or directories, with `--check` and
  `--stdin`, and keeps each file's line endings unless `.editorconfig` says
  otherwise.

**Tech Stack:** Rust (MSRV 1.78, edition 2021), no new crates, `assert_cmd`
end-to-end tests, the existing `insta` snapshots of the lexer and parser,
GitHub Actions, and Docker (`rust:1-slim`) for local Linux runs.

**Spec:** `docs/superpowers/specs/2026-10-07-phase-3-1-formatter-design.md`
(commits `83a325a` and `73532d8`), approved by the user on 2026-10-07. Read
it before Task 1: it is the authority this plan argues from. Its §13 lists
the 22 decisions made while writing it.

## Global Constraints

- **MSRV 1.78, edition 2021.** Every cargo call in this plan passes
  `--locked`, except two that must update `Cargo.lock`: Task 4's, which adds
  `thiserror` to `nova-fmt`, and Task 9's, which adds `nova-fmt` to
  `nova-cli`. Neither adds a package; each lists one more dependency under a
  crate already in the lock.
- **No new dependencies** (spec §5.2, §7.3): no package joins `Cargo.lock`.
  - `thiserror` is already the workspace's, and the lexer's and the
    parser's.
  - The document printer and the `.editorconfig` reader are written here.
  - `nova-fmt`'s tests use only the crate itself, the crates its manifest
    already lists (`nova-ast`, `nova-diagnostics`, `nova-lexer`,
    `nova-parser`) and `std`.
- **Names and values, verbatim from the spec:**
  - **Width** 100 columns, counted in Unicode scalar values; **indentation**
    4 spaces (§5.2, §6).
  - **The misplaced-doc error** (§4): `a doc comment must come right before
    an item, a field or a variant; use `//` for a plain comment`.
  - **The separator rule** (§5.6):
    - a `;` or `,` is printed before a statement or arm that begins with
      `(`, `[`, `{`, `-`, `*`, `&` or `|`;
    - a `return` or `break` with no value keeps its `;` whenever another
      statement follows it.
  - **`--check`'s line** (§7.1): `would reformat: <path>`.
  - **Exit codes** (§7.2): 0, 1 when `--check` finds a file that would
    change, 2 on any error, with an error outranking a would-change.
  - **The corpus gate** (§9.4): every `.nova` file in the repository except
    `target/` and dot-directories. `crates/nova-parser/tests/fixtures/async.nova`
    is the only file expected not to parse, and at least 167 files must be
    checked.
- **Fixtures are never rewritten** (the plan's risk 3): `tests/runtime/`,
  `crates/nova-parser/tests/fixtures/` and `docs/benchmarks/`. Only `std/`
  and `examples/` are formatted on disk (Task 11).
- **Records** (spec §10):
  - ADR bodies stay unchanged, apart from the new ADR 0028.
  - Specs and documents get dated notes that open with
    `**Amended 2026-10-07 (branch `phase-3-1-formatter`):**`.
  - The dated plans and specs under `docs/superpowers/` stay as they are.

  If you execute on a later day, change that date in every note you write.
- **Stops, from the standing workflow:**
  - Pushing the branch and opening the PR follow the standing workflow.
  - A merge happens only on the user's word, by rebase, verified by tree
    identity.
  - Nothing is published, and no tag is pushed.

## Review Focus

The five inputs most likely to bite a user that the spec's own tests leave
uncovered, most likely first. Each has its test in the task that owns the
code:

1. **A file whose last line is a line comment with no newline after it,**
   such as `fn main() {} // done`. The comment must survive, and the file
   must end with exactly one newline →
   `a_comment_at_the_end_of_a_file_without_a_newline` (Task 6).
2. **A string holding `//` or `/*`,** such as a URL. It must not be taken for
   a comment → `a_comment_marker_inside_a_string_is_not_a_comment` (Task 1,
   lexer) and `a_comment_marker_inside_a_string_is_not_a_comment_either`
   (Task 6, formatter).
3. **Non-ASCII text near the width limit,** such as a Thai string. Width is
   counted in characters, not bytes, so a line of exactly 100 characters
   stays on one line → `width_counts_characters_not_bytes` (Task 5).
4. **A comment between a closing `}` and its `else`.** It must survive, and
   formatting must stay idempotent →
   `a_comment_between_a_closing_brace_and_else_survives` (Task 6).
5. **Tab-indented input.** Tabs become 4-space indentation →
   `tab_indentation_becomes_spaces` (Task 5).

## Decisions: where this plan settles what the spec leaves open

Each is open to the user's review, like the spec's §13.

1. **`FormatError::Syntax` holds `rendered`** as well as `diagnostics`. The
   rendered text is what `nova check` prints, so a caller needs no
   `FileDb`. The spec's `Syntax(Vec<Diagnostic>)` becomes
   `Syntax { diagnostics, rendered }`.
2. **More public functions:** `format_named(source, name)` beside
   `format(source)`, and `format_text` beside `format_file`, which `--stdin`
   uses. A private `format_with` takes the printer as an argument, so a unit
   test can hand it a broken printer and watch the self-check refuse the
   output (spec §9.6, the first mutant).
3. **An import run prints without blank lines between its imports.**
   "Exactly one blank line between top-level items" applies between the run
   and its neighbours. Only top-level runs are sorted; an `import` inside a
   block keeps its place and its list order. A comment travels with the
   import after it, except comments that a blank line separates from the
   run's first import, such as a file's header: those stay above the run.
4. **A broken `where` clause takes its trailing comma only before a `{`.**
   The parser change (Task 2) accepts a trailing comma before `{`, `;` or
   `}`. A bodyless signature (a required trait method, an extern function) is
   followed by a `fn`, which a fn type could also begin with, so its clause
   ends at its last bound.
5. **How comments are placed** (§5.4). A cursor takes them in source order as
   the printer reaches each node:
   - an own-line comment becomes leading;
   - an end-of-line comment after a list element becomes trailing, printed
     after any `,`;
   - an end-of-line comment anywhere else is printed where it was, followed
     by a line break;
   - a mid-line block comment is printed with a space after it;
   - a comment on a `{`'s own line stays there, which keeps formatting
     idempotent.

   Because the printer reaches nodes in source order, no comment can be
   reordered, except inside an import run.
6. **"The whole construct fits"** (§6, "Blocks"). A function shares one
   group with its body, as do `while`, `for`, a closure, a match arm, and an
   `if` chain. So a one-statement body stays on one line only when the whole
   construct does. When it breaks, the signature line is measured on its
   own, and its parameters break only if that line still does not fit.
   Because the parameters break first, a line comment inside a `where`
   clause or a return type breaks them too.
7. **Method chains** break only when they hold two or more method calls.
   Field accesses, `?`, `.await` and indexing stay with the call before
   them, and so does any field access before the first call
   (`self.items` in `self.items.iter().map(f)`).
8. **What never breaks:** closure parameter lists and attribute arguments.
   None is long in the corpus. An or-pattern breaks only inside its own
   group, before each `|`.
9. **Docs print from the AST,** as `///` plus their text, before the
   attributes. A comment between doc lines or attributes moves after them,
   to just before the item's keyword, still in order.
10. **Blank lines in comma lists.** The spec keeps the author's blank lines
    before "a statement, field, member, match arm or variant" (§5.3). Here
    a field is a record declaration's field. A kept blank line breaks its
    list, since one line cannot hold it. Every other comma list (parameters,
    arguments, record literals, patterns) drops blank lines.
11. **`& &x` cannot arise in an expression.** The spec's §2 and §5.3 say a
    reference to a reference is written `& &x`. But `&` takes a postfix
    expression (`grammar.rs:1556-1560`), so `& &x` does not parse. It is
    written `&(&x)`, and the printer keeps those parentheses. Only the type
    `& &T` needs the space.
12. **No last-argument hugging.** A closure argument whose body holds two or
    more statements breaks the call's whole argument list, one argument per
    line. rustfmt and prettier hug the last argument instead. The corpus has
    no such closure, and the document printer could add hugging later.
13. **`insert_final_newline = false`** (§7.3, "the file keeps whatever it
    had") keeps a final newline if the file had one, and adds none if it
    had none. Blank lines at the end are still removed.
14. **Exact strings, not `insta` snapshots.** The master spec's §5.2 asks
    for `insta` snapshots of formatter output. Each formatter test instead
    compares the whole output with an exact string written beside its
    input, which pins the same thing, and its helper also checks
    idempotence. Task 12 records this in a dated note in the master spec.

## File Structure

| File | Task | Responsibility |
|---|---|---|
| `crates/nova-lexer/src/lib.rs`, `tests/lexer_tests.rs`, `tests/snapshots/lexer_tests__doc_comment.snap` | 1 | Comments, doc text, `////`, unterminated `/*` |
| `crates/nova-ast/src/item.rs` | 2 | `docs` on 14 nodes |
| `crates/nova-parser/src/grammar.rs`, `tests/parser_tests.rs`, `tests/snapshots/*.snap` | 2 | Doc positions and errors, pattern spans, `where`'s trailing comma |
| `crates/nova-typeck/src/check.rs:779` | 2 | A pattern gains `..` |
| `crates/nova-fmt/src/doc.rs` (new) | 3 | The document and its renderer |
| `crates/nova-fmt/src/lib.rs`, `src/source.rs`, `src/check.rs` (new) | 4, 5, 7 | The API, the parsed input, the self-check |
| `crates/nova-fmt/Cargo.toml`, `Cargo.lock` | 4 | `thiserror` for the error types |
| `crates/nova-fmt/src/print/{mod,item,expr,pattern,ty}.rs` (new) | 5, 6 | The AST walked into a document |
| `crates/nova-fmt/src/comments.rs` (new) | 6 | The comment cursor |
| `crates/nova-fmt/tests/{common/mod,layout,source_rules}.rs` (new) | 5 | Layout and source-rule tests |
| `crates/nova-fmt/tests/comments.rs` (new) | 6 | Comment tests |
| `crates/nova-fmt/src/{editorconfig,file}.rs`, `tests/files.rs` (new) | 7 | `.editorconfig`, line endings, `format_file` |
| `crates/nova-fmt/tests/corpus.rs` (new) | 8 | The gate's three checks over the repository |
| `crates/nova-cli/src/cmd/fmt.rs` (new), `src/cmd/mod.rs`, `src/main.rs`, `Cargo.toml`, `tests/fmt.rs` (new), `Cargo.lock` | 9 | `nova fmt` |
| `std/**/*.nova`, `examples/**/*.nova`, `.github/workflows/ci.yml` | 11 | The reformat, and CI's check |
| `docs/adr/0028-the-formatter.md` (new), records Task 12 lists | 12 | ADR, notes, CHANGELOG, ARCHITECTURE, citations, sweep |

## Conventions for every task

- **Working directory.** `D:\Projects\nona\nova`, which is Git Bash
  `/d/Projects/nona/nova`. The Bash tool resets its directory after each
  call, so write `cd /d/Projects/nona/nova && …` in one command.
- **Scratch directory.** `P=/c/Users/SAKEER~1/AppData/Local/Temp/gcm/p31`,
  outside the repository. Long output goes to a file there; read its tail.
- **Line endings.** The working tree is mostly CRLF (`core.autocrlf=true`).
  The Edit tool is fine. Write new files with the Write tool.
- **Write scripts and commit messages with the Write tool, never a Bash
  heredoc.** The Bash tool turns `\\` into `\`. Commit with
  `git commit -F $P/msg-<n>.txt`, then check `git log -1 --format=%s`.
- **Format before every commit:** run `cargo fmt --all`, then stage what it
  changed. rustfmt may rewrap the plan's code; that is expected.
- **Chain a commit and what follows it with `&&`, never `;`.**
- **Unused-code warnings** from `nova-fmt` in Tasks 3 to 5 are expected:
  the printer that uses that code arrives in Task 5, and the comments that
  construct `LineSuffix` and `BreakParent` in Task 6. From Task 6 on there
  are none. Task 13 runs clippy with `-D warnings` over the finished crate.
- **Port 3000 must be free for a full Windows suite** (the
  `http_server_example_*` tests). Check with
  `netstat -ano | grep -E "[:.]3000 .*LISTENING"`, which prints nothing
  when the port is free. **The user's own servers sometimes hold it.** Never
  stop such a process: stop and ask the user.
- **Linux runs** use the existing harness:
  `bash /c/Users/SAKEER~1/AppData/Local/Temp/gcm/linux/run.sh <cargo args>`.
  - It exports the checkout's tracked files as an LF tarball, uncommitted
    edits included. **Untracked files are not exported:** commit or
    `git add` new files first.
  - Docker Desktop must be running: `docker version` shows a `Server:`
    section. Start it with PowerShell
    `Start-Process "C:\Program Files\Docker\Docker\Docker Desktop.exe"`.
- **Mutants run only on committed work,** and are undone with
  `git checkout -- <file>`. Never commit a mutant.
- **Counting a full run.** Every full-suite step below is followed by this,
  with `<FILE>` replaced:

  ```bash
  sed -E 's/\x1b\[[0-9;]*m//g' <FILE> | grep -E "^test result:" | awk '{p+=$4; f+=$6; i+=$8; n++} END {print n" result lines: "p" passed, "f" failed, "i" ignored"}'
  ```
- **Expected outputs are exact.** A formatter test compares the whole
  output. If one fails on the first run of its task, read the diff before
  touching anything. If the code does what this plan describes and the
  plan's expected string is wrong, correct the string and ledger it as a
  ruling. If the code does something else, fix the code.

---
### Task 1: The lexer keeps comments, doc text, and reports an unterminated `/*`

Spec §3 and §9.1. `lex` keeps its signature; its output changes only for doc
text, `////`, and unterminated block comments.

**Files:**
- Modify: `crates/nova-lexer/src/lib.rs` (the `Comment` types, the
  `comments` field, `lex_with_comments`, `skip_trivia`, `next_token`, and
  the `DocComment` mapping at `lib.rs:534`)
- Modify: `crates/nova-lexer/tests/lexer_tests.rs`
- Modify: `crates/nova-lexer/tests/snapshots/lexer_tests__doc_comment.snap`,
  through `INSTA_UPDATE=always`

**Interfaces:**
- Consumes: nothing from earlier tasks.
- Produces, in `nova_lexer`:
  - `#[derive(Clone, Copy, Debug, PartialEq, Eq)] pub struct Comment { pub kind: CommentKind, pub span: Span }`
  - `#[derive(Clone, Copy, Debug, PartialEq, Eq)] pub enum CommentKind { Line, Block }`
  - `pub fn lex_with_comments(source: &str, file: FileId) -> (Vec<Spanned<Token>>, Vec<Comment>, Vec<LexError>)`
  - `Token::DocComment(text)` where `text` is everything after `///`, with
    trailing whitespace removed and leading whitespace kept.

- [ ] **Step 1: Take the baseline**

```bash
cd /d/Projects/nona/nova && P=/c/Users/SAKEER~1/AppData/Local/Temp/gcm/p31 && mkdir -p $P && git status --short | wc -l && git log --oneline -1 && git branch --show-current
```

Expected: `0`, `73532d8 docs: correct the 3.1 spec after its fact-check`, and
`phase-3-1-formatter`.

Then the baseline full suite, after checking port 3000:

```bash
cd /d/Projects/nona/nova && P=/c/Users/SAKEER~1/AppData/Local/Temp/gcm/p31 && netstat -ano | grep -E "[:.]3000 .*LISTENING"; cargo build --locked -p nova-runtime 2>&1 | tail -1 && cargo test --locked --workspace --all-features --no-fail-fast > $P/suite-baseline.txt 2>&1; echo "exit=$?"
```

Count it with the conventions' counting line. Expected: 0 failed. CI's
Windows leg reported 1291 passed and 8 ignored at `2bf8006`. Ledger the
local figures as the baseline Task 13 compares against.

- [ ] **Step 2: Write the failing tests**

In `crates/nova-lexer/tests/lexer_tests.rs`, change the first two lines to:

```rust
use nova_diagnostics::FileDb;
use nova_lexer::{lex, lex_with_comments, CommentKind, Token};
use std::path::{Path, PathBuf};
```

Then add at the end of the file:

```rust
// === Comments, for the formatter (spec
// docs/superpowers/specs/2026-10-07-phase-3-1-formatter-design.md §3) ===

/// Each comment `lex_with_comments` reports, as its kind and source text.
fn comments(source: &str) -> Vec<(CommentKind, String)> {
    let mut db = FileDb::new();
    let file = db.add("<test>", source);
    let (_, comments, errors) = lex_with_comments(source, file);
    assert!(errors.is_empty(), "unexpected lex errors: {errors:?}");
    comments
        .iter()
        .map(|c| (c.kind, source[c.span.start as usize..c.span.end as usize].to_owned()))
        .collect()
}

#[test]
fn lex_with_comments_returns_each_comment_in_order() {
    // The third sits just before a string, and the last ends the file with
    // no newline after it.
    assert_eq!(
        comments("// one\nfn /* two */ main() { f(/* three */\"s\") } // four"),
        vec![
            (CommentKind::Line, "// one".to_owned()),
            (CommentKind::Block, "/* two */".to_owned()),
            (CommentKind::Block, "/* three */".to_owned()),
            (CommentKind::Line, "// four".to_owned()),
        ]
    );
}

#[test]
fn a_line_comments_span_stops_before_a_crlf() {
    assert_eq!(
        comments("// a\r\nfn main() {}\r\n"),
        vec![(CommentKind::Line, "// a".to_owned())]
    );
}

#[test]
fn a_comment_inside_an_interpolation_hole_is_captured() {
    assert_eq!(
        comments("let s = \"${x /* c */}\""),
        vec![(CommentKind::Block, "/* c */".to_owned())]
    );
}

#[test]
fn a_comment_marker_inside_a_string_is_not_a_comment() {
    assert_eq!(
        comments("let u = \"http://example.com/*x*/\" // real"),
        vec![(CommentKind::Line, "// real".to_owned())]
    );
}

#[test]
fn four_slashes_make_a_plain_comment() {
    let toks = tokens("//// ----\nfn f() {}");
    assert!(matches!(toks[0], Token::Fn), "toks: {toks:?}");
    assert_eq!(
        comments("//// ----\nfn f() {}"),
        vec![(CommentKind::Line, "//// ----".to_owned())]
    );
}

#[test]
fn doc_text_keeps_its_indentation_and_drops_trailing_whitespace() {
    let toks = tokens("///   indented  \r\nfn f() {}");
    assert_eq!(toks[0], Token::DocComment("   indented".to_owned()));
}

#[test]
fn an_unterminated_block_comment_is_an_error() {
    let (_, errors) = lex_all("fn main() {} /* never closed");
    assert_eq!(errors, vec!["unterminated block comment".to_owned()]);
}

/// Every `.nova` file under `dir`, skipping `target/` and dot-directories.
fn nova_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let mut entries: Vec<_> = std::fs::read_dir(dir)
        .expect("read a directory")
        .map(|e| e.expect("read an entry"))
        .collect();
    entries.sort_by_key(|e| e.file_name());
    for entry in entries {
        let name = entry.file_name().to_string_lossy().into_owned();
        let kind = entry.file_type().expect("read an entry's type");
        if kind.is_dir() {
            if name != "target" && !name.starts_with('.') {
                nova_files(&entry.path(), out);
            }
        } else if kind.is_file() && name.ends_with(".nova") {
            out.push(entry.path());
        }
    }
}

#[test]
fn lex_and_lex_with_comments_agree_on_every_nova_file() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let mut files = Vec::new();
    nova_files(&root, &mut files);
    assert!(files.len() >= 168, "found only {} .nova files", files.len());
    for path in files {
        let text = std::fs::read_to_string(&path).expect("read a .nova file");
        let mut db = FileDb::new();
        let file = db.add("<corpus>", text.as_str());
        let (plain, _) = lex(&text, file);
        let (with, _, _) = lex_with_comments(&text, file);
        let pairs = |toks: Vec<nova_diagnostics::Spanned<Token>>| {
            toks.into_iter().map(|t| (t.value, t.span)).collect::<Vec<_>>()
        };
        assert_eq!(pairs(plain), pairs(with), "{}", path.display());
    }
}
```

- [ ] **Step 3: Run them to watch them fail**

```bash
cd /d/Projects/nona/nova && cargo test --locked -p nova-lexer --test lexer_tests 2>&1 | grep -E "^error(\[E[0-9]+\])?:" | head -5
```

Expected: errors that `lex_with_comments` and `CommentKind` do not exist in
`nova_lexer`.

- [ ] **Step 4: Add the comment types and `lex_with_comments`**

In `crates/nova-lexer/src/lib.rs`, replace

```rust
/// Lex `source` and return all tokens and any errors encountered.
///
/// Errors are non-fatal: the lexer continues after a bad character and collects
/// all errors in the returned `Vec`.
pub fn lex(source: &str, file: FileId) -> (Vec<Spanned<Token>>, Vec<LexError>) {
    let mut lexer = Lexer::new(source, file);
    lexer.tokenize()
}
```

with

```rust
/// Lex `source` and return all tokens and any errors encountered.
///
/// Errors are non-fatal: the lexer continues after a bad character and collects
/// all errors in the returned `Vec`.
pub fn lex(source: &str, file: FileId) -> (Vec<Spanned<Token>>, Vec<LexError>) {
    let mut lexer = Lexer::new(source, file);
    lexer.tokenize()
}

/// [`lex`], plus every comment it skipped, in source order: what the
/// formatter needs to keep them (spec
/// `docs/superpowers/specs/2026-10-07-phase-3-1-formatter-design.md` §3.1).
/// The tokens and errors are exactly `lex`'s.
pub fn lex_with_comments(
    source: &str,
    file: FileId,
) -> (Vec<Spanned<Token>>, Vec<Comment>, Vec<LexError>) {
    let mut lexer = Lexer::new(source, file);
    let (tokens, errors) = lexer.tokenize();
    (tokens, lexer.comments, errors)
}

/// A comment the lexer skipped.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Comment {
    pub kind: CommentKind,
    /// From `//` to the end of its line, without the line break or a `\r`
    /// before it; or from `/*` through `*/`.
    pub span: Span,
}

/// Which kind of comment.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CommentKind {
    /// `// …`, including `//// …`.
    Line,
    /// `/* … */`, which does not nest.
    Block,
}
```

In `pub struct Lexer<'src>`, after the `in_string` field, add:

```rust
    // Every comment skipped so far, in source order.
    comments: Vec<Comment>,
```

and in `Lexer::new`, after `in_string: false,`, add `comments: Vec::new(),`.

- [ ] **Step 5: Record comments, treat `////` as plain, report an unterminated `/*`**

Replace the whole of `fn skip_trivia` (its doc comment included) with:

```rust
    /// Advance past whitespace, `//` line comments, and `/* */` block
    /// comments, recording each comment. A `///` doc comment, exactly three
    /// slashes, is *not* trivia: it is left for the tokenizer to emit as a
    /// `DocComment`. Four or more slashes make a plain comment, as in Rust. A
    /// `/*` with no `*/` before the end of the file is an error (10-LEXER.md
    /// §6).
    fn skip_trivia(&mut self) -> Result<(), LexError> {
        let b = self.source.as_bytes();
        loop {
            while self.pos < b.len() {
                match b[self.pos] {
                    b' ' | b'\t' | b'\n' | b'\r' => self.pos += 1,
                    _ => break,
                }
            }
            if self.pos + 1 < b.len() && b[self.pos] == b'/' && b[self.pos + 1] == b'/' {
                let doc = b.get(self.pos + 2) == Some(&b'/') && b.get(self.pos + 3) != Some(&b'/');
                if doc {
                    break;
                }
                let start = self.pos;
                while self.pos < b.len() && b[self.pos] != b'\n' {
                    self.pos += 1;
                }
                let end = if self.pos > start && b[self.pos - 1] == b'\r' {
                    self.pos - 1
                } else {
                    self.pos
                };
                self.comments.push(Comment {
                    kind: CommentKind::Line,
                    span: self.span(start, end),
                });
                continue;
            }
            if self.pos + 1 < b.len() && b[self.pos] == b'/' && b[self.pos + 1] == b'*' {
                let start = self.pos;
                self.pos += 2;
                while self.pos + 1 < b.len() && !(b[self.pos] == b'*' && b[self.pos + 1] == b'/') {
                    self.pos += 1;
                }
                if self.pos + 1 >= b.len() {
                    self.pos = b.len();
                    return Err(LexError::UnterminatedBlockComment(self.span(start, b.len())));
                }
                self.pos += 2;
                self.comments.push(Comment {
                    kind: CommentKind::Block,
                    span: self.span(start, self.pos),
                });
                continue;
            }
            break;
        }
        Ok(())
    }
```

In `fn next_token`, change `self.skip_trivia();` to `self.skip_trivia()?;`.

- [ ] **Step 6: Keep the doc text's indentation**

Change the `DocComment` arm of the token mapping (`lib.rs:534`) from

```rust
        RawToken::DocComment => Token::DocComment(slice[3..].trim().to_owned()),
```

to

```rust
        // Leading whitespace kept, so indentation inside a doc's Markdown
        // survives; trailing whitespace, `\r` included, dropped (spec §3.2).
        RawToken::DocComment => Token::DocComment(slice[3..].trim_end().to_owned()),
```

- [ ] **Step 7: Run the tests, and accept the one changed snapshot**

```bash
cd /d/Projects/nona/nova && INSTA_UPDATE=always cargo test --locked -p nova-lexer 2>&1 | grep -E "^test result|FAILED|panicked" | head -20 && git diff --stat -- crates/nova-lexer/tests/snapshots && git diff -U0 -- crates/nova-lexer/tests/snapshots | grep -E "^[-+] "
```

Expected:
- every `test result:` line is `ok`, with 0 failed;
- only `lexer_tests__doc_comment.snap` changed, one line:
  `-    "DocComment(\"This is a doc comment\")",` became
  `+    "DocComment(\" This is a doc comment\")",`.

The leading space is the change §3.2 asks for. Then run the lexer's tests
again without `INSTA_UPDATE`, and expect 0 failed.

- [ ] **Step 8: Commit**

Write `$P/msg-1.txt`:

```
nova-lexer: keep comments, and a doc comment's indentation

lex_with_comments returns what lex returns, plus every comment skip_trivia
passes over, in source order, so the formatter can keep them (spec
docs/superpowers/specs/2026-10-07-phase-3-1-formatter-design.md §3).

- A doc comment is exactly three slashes. Four or more make a plain
  comment, as in Rust, so a `//// ----` separator line is legal.
- A doc comment's text keeps its leading whitespace, so indentation
  inside its Markdown survives for nova doc, and loses trailing
  whitespace, `\r` included.
- An unterminated `/*` reports UnterminatedBlockComment, the error
  10-LEXER.md §6 lists. It used to comment out the rest of the file
  silently.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
```

```bash
cd /d/Projects/nona/nova && P=/c/Users/SAKEER~1/AppData/Local/Temp/gcm/p31 && cargo fmt --all && git add crates/nova-lexer && git commit -q -F $P/msg-1.txt && git log -1 --format=%s
```

Expected: `nova-lexer: keep comments, and a doc comment's indentation`.

The task's test command: `cargo test --locked -p nova-lexer`.

---
### Task 2: Doc comments attach to the AST, and three grammar fixes

Spec §4 and §9.2. The grammar fixes: misplaced `///` is an error, a
parenthesised pattern's span covers its parentheses, and a `where` clause
takes a trailing comma.

**Files:**
- Modify: `crates/nova-ast/src/item.rs` (14 nodes gain `docs`)
- Modify: `crates/nova-parser/src/grammar.rs`
- Modify: `crates/nova-typeck/src/check.rs:779`
- Modify: `crates/nova-parser/tests/parser_tests.rs`
- Modify: the six AST snapshots in `crates/nova-parser/tests/snapshots/`,
  through `INSTA_UPDATE=always`

**Interfaces:**
- Consumes: Task 1's `Token::DocComment(text)`, with the text after `///`.
- Produces: `pub docs: Vec<Spanned<String>>` on `Function`, `FunctionSig`,
  `Record`, `RecordField`, `TypeDecl`, `Variant`, `TraitDecl`,
  `TraitItem::AssocType`, `ImplBlock`, `AssocTypeBinding`, `ConstDecl`,
  `Import`, `Module` and `ExternBlock`. Each holds one entry per `///` line,
  in order, and each entry's span is its `///` token.
  - A documented item's `Spanned<Item>` span starts at its first doc line.
  - A parenthesised pattern's span, and the unit pattern's, covers the
    parentheses.

- [ ] **Step 1: Write the failing tests**

In `crates/nova-parser/tests/parser_tests.rs`, replace the first line,
`use nova_ast::{File, Item, Visibility};`, with:

```rust
use nova_ast::item::{ExternItem, TraitItem, TypeDef};
use nova_ast::{File, Item, Stmt, Visibility};
use nova_diagnostics::Spanned;
```

Then add at the end of the file:

```rust
// === Doc comments (spec
// docs/superpowers/specs/2026-10-07-phase-3-1-formatter-design.md §4) ===

const MISPLACED: &str = "a doc comment must come right before an item, a field or a variant; use `//` for a plain comment";

fn docs(d: &[Spanned<String>]) -> Vec<&str> {
    d.iter().map(|s| s.value.as_str()).collect()
}

#[test]
fn docs_attach_to_each_kind_of_item() {
    let src = "/// f\nfn f() {}\n/// r\nrecord R { x: Int }\n/// t\ntype T = Int\n\
               /// tr\ntrait Tr {}\n/// i\nimpl R {}\n/// c\nconst C: Int = 1\n\
               /// im\nimport m\n/// mo\nmodule n\n/// e\nextern \"C\" {}\n";
    let (file, errors) = parse_str(src);
    assert!(errors.is_empty(), "{errors:?}");
    let got: Vec<Vec<&str>> = file
        .items
        .iter()
        .map(|item| match &item.value {
            Item::Function(x) => docs(&x.docs),
            Item::Record(x) => docs(&x.docs),
            Item::Type(x) => docs(&x.docs),
            Item::Trait(x) => docs(&x.docs),
            Item::Impl(x) => docs(&x.docs),
            Item::Const(x) => docs(&x.docs),
            Item::Import(x) => docs(&x.docs),
            Item::Module(x) => docs(&x.docs),
            Item::Extern(x) => docs(&x.docs),
        })
        .collect();
    let want: Vec<Vec<&str>> = [" f", " r", " t", " tr", " i", " c", " im", " mo", " e"]
        .iter()
        .map(|d| vec![*d])
        .collect();
    assert_eq!(got, want);
    // A documented item's span starts at its first doc line.
    assert_eq!(file.items[0].span.start, 0);
}

#[test]
fn docs_attach_to_members_fields_and_variants() {
    let src = "trait Tr {\n    /// req\n    fn a(self)\n    /// prov\n    fn b(self) {}\n    \
               /// assoc\n    type Item\n}\n\
               impl Tr for R {\n    /// m\n    fn a(self) {}\n    /// k\n    const K: Int = 1\n    \
               /// bind\n    type Item = Int\n}\n\
               record R {\n    /// x\n    x: Int,\n}\n\
               type T =\n    /// a\n    | A\n    | B\n\
               extern \"C\" {\n    /// ext\n    fn ext(x: Int) -> Int\n}\n";
    let (file, errors) = parse_str(src);
    assert!(errors.is_empty(), "{errors:?}");
    let Item::Trait(tr) = &file.items[0].value else { panic!("{:?}", file.items[0]) };
    let trait_docs: Vec<Vec<&str>> = tr
        .items
        .iter()
        .map(|it| match it {
            TraitItem::Required(sig) => docs(&sig.docs),
            TraitItem::Provided(f) => docs(&f.docs),
            TraitItem::AssocType { docs: d, .. } => docs(d),
        })
        .collect();
    assert_eq!(trait_docs, vec![vec![" req"], vec![" prov"], vec![" assoc"]]);
    let Item::Impl(im) = &file.items[1].value else { panic!("{:?}", file.items[1]) };
    assert_eq!(docs(&im.functions[0].docs), vec![" m"]);
    assert_eq!(docs(&im.consts[0].docs), vec![" k"]);
    assert_eq!(docs(&im.assoc_types[0].docs), vec![" bind"]);
    let Item::Record(r) = &file.items[2].value else { panic!("{:?}", file.items[2]) };
    assert_eq!(docs(&r.fields[0].docs), vec![" x"]);
    let Item::Type(t) = &file.items[3].value else { panic!("{:?}", file.items[3]) };
    let TypeDef::Sum(variants) = &t.def else { panic!("{:?}", t.def) };
    assert_eq!(docs(&variants[0].docs), vec![" a"]);
    assert!(variants[1].docs.is_empty());
    let Item::Extern(e) = &file.items[4].value else { panic!("{:?}", file.items[4]) };
    let ExternItem::Fn(sig) = &e.items[0];
    assert_eq!(docs(&sig.docs), vec![" ext"]);
}

#[test]
fn docs_may_come_between_attributes_on_a_top_level_item() {
    let (file, errors) = parse_str("/// one\n@test\n/// two\nfn t() {}\n");
    assert!(errors.is_empty(), "{errors:?}");
    let Item::Function(f) = &file.items[0].value else { panic!("{:?}", file.items) };
    assert_eq!(docs(&f.docs), vec![" one", " two"]);
    assert_eq!(f.attrs.len(), 1);
}

#[test]
fn docs_attach_to_a_nested_item() {
    let (file, errors) = parse_str("fn main() {\n    /// inner\n    fn helper() {}\n}\n");
    assert!(errors.is_empty(), "{errors:?}");
    let Item::Function(main) = &file.items[0].value else { panic!("{:?}", file.items) };
    let Stmt::Item(item) = &main.body.value.stmts[0].value else {
        panic!("{:?}", main.body.value.stmts)
    };
    let Item::Function(helper) = &**item else { panic!("{item:?}") };
    assert_eq!(docs(&helper.docs), vec![" inner"]);
}

#[test]
fn a_misplaced_doc_comment_is_an_error() {
    for (place, src) in [
        ("before a let", "fn main() {\n    /// no\n    let x = 1\n}\n"),
        ("before an expression statement", "fn main() {\n    /// no\n    f()\n}\n"),
        ("before a match arm", "fn main() {\n    match x {\n        /// no\n        _ => 1\n    }\n}\n"),
        ("before a parameter", "fn f(\n    /// no\n    x: Int,\n) {}\n"),
        ("before a closing brace", "fn main() {\n    f()\n    /// no\n}\n"),
        ("before a body's closing brace", "trait T {\n    /// no\n}\n"),
        ("at the end of the file", "fn main() {}\n/// no\n"),
    ] {
        let (_, errors) = parse_str(src);
        let messages: Vec<String> = errors.iter().map(|e| e.to_string()).collect();
        assert_eq!(messages, vec![MISPLACED.to_owned()], "{place}");
    }
}

#[test]
fn a_variant_list_does_not_take_the_next_items_doc() {
    let (file, errors) = parse_str("type T = | A | B\n/// next\nfn f() {}\n");
    assert!(errors.is_empty(), "{errors:?}");
    let Item::Function(f) = &file.items[1].value else { panic!("{:?}", file.items) };
    assert_eq!(docs(&f.docs), vec![" next"]);
}

#[test]
fn a_trailing_comma_ends_a_where_clause() {
    let (with, errors) = parse_str("fn f<T>(x: T) -> T where T: A, { x }\n");
    assert!(errors.is_empty(), "{errors:?}");
    let (without, errors) = parse_str("fn f<T>(x: T) -> T where T: A { x }\n");
    assert!(errors.is_empty(), "{errors:?}");
    let bounds = |file: &File| match &file.items[0].value {
        Item::Function(f) => f.where_clause.len(),
        other => panic!("{other:?}"),
    };
    assert_eq!((bounds(&with), bounds(&without)), (1, 1));
}

#[test]
fn a_parenthesised_pattern_spans_its_parentheses() {
    let src = "fn main() {\n    let (x) = 1\n    let () = ()\n}\n";
    let (file, errors) = parse_str(src);
    assert!(errors.is_empty(), "{errors:?}");
    let Item::Function(f) = &file.items[0].value else { panic!("{:?}", file.items) };
    let spans: Vec<&str> = f
        .body
        .value
        .stmts
        .iter()
        .map(|s| match &s.value {
            Stmt::Let { pattern, .. } => &src[pattern.span.start as usize..pattern.span.end as usize],
            other => panic!("{other:?}"),
        })
        .collect();
    assert_eq!(spans, vec!["(x)", "()"]);
}
```

- [ ] **Step 2: Run them to watch them fail**

```bash
cd /d/Projects/nona/nova && cargo test --locked -p nova-parser --test parser_tests 2>&1 | grep -E "^error(\[E[0-9]+\])?:" | sort | uniq -c | head -8
```

Expected: `no field `docs`` errors (E0609 or E0026). The tests cannot
compile until the AST has the field.

- [ ] **Step 3: Give the 14 AST nodes their docs**

In `crates/nova-ast/src/item.rs`, add this field as the first field of
`Function`, `FunctionSig`, `Record`, `RecordField`, `TypeDecl`, `Variant`,
`TraitDecl`, `ImplBlock`, `AssocTypeBinding`, `ConstDecl`, `Import`,
`Module` and `ExternBlock`:

```rust
    /// The `///` lines before it, one entry per line, holding the text after
    /// the `///` (spec
    /// `docs/superpowers/specs/2026-10-07-phase-3-1-formatter-design.md` §4).
    pub docs: Vec<Spanned<String>>,
```

and change `TraitItem::AssocType` to:

```rust
    AssocType {
        /// The `///` lines before it, as on the items above.
        docs: Vec<Spanned<String>>,
        name: Spanned<String>,
        bounds: Vec<Spanned<Path>>,
    },
```

`item.rs` already imports `Spanned`.

- [ ] **Step 4: The parser's doc helpers**

In `crates/nova-parser/src/grammar.rs`, after the `use crate::ParseError;`
line, add:

```rust
/// The error for a `///` that documents nothing (spec
/// `docs/superpowers/specs/2026-10-07-phase-3-1-formatter-design.md` §4).
const MISPLACED_DOC: &str =
    "a doc comment must come right before an item, a field or a variant; use `//` for a plain comment";
```

At the end of the `impl<'a> Parser<'a>` block that holds `sync_to_stmt_boundary`
(just before its closing `}` at `grammar.rs:174`), add:

```rust

    // --- Doc comments (spec §4) ---

    /// Consume consecutive `///` lines and return them, in order.
    fn take_docs(&mut self) -> Vec<Spanned<String>> {
        let mut docs = Vec::new();
        while let Token::DocComment(text) = self.peek().clone() {
            let span = self.peek_span();
            self.advance();
            docs.push(Spanned::new(text, span));
        }
        docs
    }

    /// Report a `///` that documents nothing.
    fn misplaced_doc(&mut self, span: Span) {
        self.errors.push(ParseError::Custom {
            message: MISPLACED_DOC.into(),
            span,
        });
    }

    /// Report and skip the `///` lines here, which document nothing. Returns
    /// whether there were any.
    fn reject_docs(&mut self) -> bool {
        let docs = self.take_docs();
        for d in &docs {
            self.misplaced_doc(d.span);
        }
        !docs.is_empty()
    }

    /// The index of the first token at or after the cursor that is not a
    /// `///` line.
    fn docs_end(&self) -> usize {
        let mut i = self.pos;
        while matches!(self.tokens.get(i).map(|t| &t.value), Some(Token::DocComment(_))) {
            i += 1;
        }
        i
    }

    /// Whether the token after any `///` lines is `tok`.
    fn next_after_docs_is(&self, tok: &Token) -> bool {
        self.tokens
            .get(self.docs_end())
            .is_some_and(|t| std::mem::discriminant(&t.value) == std::mem::discriminant(tok))
    }

    /// Whether the token at `i` starts an item that may appear inside a block.
    fn is_item_start_at(&self, i: usize) -> bool {
        let at = |k: usize| self.tokens.get(k).map(|t| &t.value);
        matches!(
            at(i),
            Some(
                Token::Fn
                    | Token::Async
                    | Token::Record
                    | Token::Trait
                    | Token::Impl
                    | Token::Type
                    | Token::Const
                    | Token::Import
                    | Token::Module
                    | Token::Extern
            )
        ) || (matches!(at(i), Some(Token::Pub))
            && matches!(
                at(i + 1),
                Some(
                    Token::Fn
                        | Token::Record
                        | Token::Trait
                        | Token::Type
                        | Token::Const
                        | Token::Impl
                )
            ))
    }

    /// A top-level item's docs and `@attributes`, which may be interleaved.
    fn parse_item_prefix(&mut self) -> (Vec<Spanned<String>>, Vec<Attribute>) {
        let mut docs = Vec::new();
        let mut attrs = Vec::new();
        loop {
            docs.extend(self.take_docs());
            if self.peek() != &Token::At {
                break;
            }
            attrs.extend(self.parse_attributes());
        }
        (docs, attrs)
    }
```

- [ ] **Step 5: Items take their docs**

In `try_parse_item`, change `let attrs = self.parse_attributes();` to
`let (docs, attrs) = self.parse_item_prefix();`, and in each of its nine
item arms add the docs after the line that sets `attrs`. For example the
function arm becomes:

```rust
            Token::Fn | Token::Async => {
                let mut func = self.parse_function(vis)?;
                func.attrs = attrs;
                func.docs = docs;
                Item::Function(func)
            }
```

and likewise `record.docs = docs;`, `td.docs = docs;`, `tr.docs = docs;`,
`impl_.docs = docs;`, `c.docs = docs;`, `imp.docs = docs;`, `m.docs = docs;`
and `e.docs = docs;`. Replace the start of its `_` arm,

```rust
            _ => {
                let span = self.peek_span();
                self.errors.push(ParseError::Expected {
```

with

```rust
            _ => {
                let span = self.peek_span();
                if let Some(first) = docs.first() {
                    // Docs with no item after them (spec §4).
                    self.misplaced_doc(first.span);
                    return None;
                }
                self.errors.push(ParseError::Expected {
```

Then add `docs: Vec::new(),` as the first field of each struct literal the
parser builds:
- `Function` in `parse_function` (`grammar.rs:386`);
- `FunctionSig` in `parse_function_sig` (`:413`);
- `Record` in `parse_record` (`:556`);
- `TypeDecl` in `parse_type_decl` (`:602`);
- `TraitDecl` in `parse_trait_decl` (`:667`);
- `ImplBlock` in `parse_impl_block` (`:786`);
- `ConstDecl` in `parse_const_decl` (`:805`);
- `Import` in `parse_import` (`:844`);
- `Module` in `parse_module` (`:861`);
- `ExternBlock` in `parse_extern_block` (`:895`).

The four list members get their docs in Step 6.

- [ ] **Step 6: Members, fields and variants take their docs**

**Record fields.** In `parse_record`, change the loop's first line,
`let fvis = self.parse_visibility();`, to:

```rust
            let docs = self.take_docs();
            if self.check(&Token::RBrace) || self.is_at_end() {
                if let Some(first) = docs.first() {
                    self.misplaced_doc(first.span);
                }
                continue;
            }
            let fvis = self.parse_visibility();
```

and the push to:

```rust
            fields.push(RecordField {
                docs,
                vis: fvis,
                name: fname,
                ty: fty,
            });
```

**Variants.** In `parse_type_decl`, replace

```rust
        // Sum type: starts with `|`
        let def = if self.check(&Token::Pipe) {
            let mut variants = Vec::new();
            while self.eat(&Token::Pipe).is_some() {
```

with

```rust
        // Sum type: starts with `|`, after the first variant's docs if any.
        // A variant's docs come before its `|`; docs with no `|` after them
        // belong to whatever follows the type, so they are left for it.
        let def = if self.next_after_docs_is(&Token::Pipe) {
            let mut variants = Vec::new();
            while self.next_after_docs_is(&Token::Pipe) {
                let docs = self.take_docs();
                self.advance(); // the `|`
```

and its push to:

```rust
                variants.push(Variant {
                    docs,
                    name: vname,
                    fields,
                });
```

**Trait members.** In `parse_trait_decl`, the loop starts:

```rust
        while !self.check(&Token::RBrace) && !self.is_at_end() {
            // An associated type declaration (`type Item` or `type Item:
```

Insert after its `while` line:

```rust
            let docs = self.take_docs();
            if self.check(&Token::RBrace) || self.is_at_end() {
                if let Some(first) = docs.first() {
                    self.misplaced_doc(first.span);
                }
                continue;
            }
```

Then change `items.push(TraitItem::AssocType { name, bounds });` to
`items.push(TraitItem::AssocType { docs, name, bounds });`, and the
speculative block from `if let Some(sig) = self.parse_function_sig() {` to
its matching `} else {` to:

```rust
            if let Some(mut sig) = self.parse_function_sig() {
                if self.check(&Token::LBrace) {
                    // Provided method — we need to re-parse as Function.
                    // Roll back and parse as full function.
                    self.pos = saved_pos;
                    self.errors.truncate(saved_errors_len);
                    let func_vis = Visibility::Private;
                    if let Some(mut func) = self.parse_function(func_vis) {
                        func.docs = docs;
                        items.push(TraitItem::Provided(func));
                    }
                } else {
                    // Required method — expect semicolon.
                    self.eat(&Token::Semicolon);
                    sig.docs = docs;
                    items.push(TraitItem::Required(sig));
                }
            } else {
```

**Impl members.** In `parse_impl_block`, the loop starts:

```rust
        while !self.check(&Token::RBrace) && !self.is_at_end() {
            // Captured before `parse_visibility` consumes it, so the `pub`
```

Insert after its `while` line the same six lines as for trait members. Then:
- the `Fn | Async` arm becomes:

  ```rust
                Token::Fn | Token::Async => {
                    if let Some(mut f) = self.parse_function(vis) {
                        f.docs = docs;
                        functions.push(f);
                    } else {
                        self.sync_to_item_boundary();
                    }
                }
  ```
- `assoc_types.push(AssocTypeBinding { name, ty });` becomes
  `assoc_types.push(AssocTypeBinding { docs, name, ty });`;
- in the `Const` arm, `if let Some(c) = self.parse_const_decl(vis) {` and
  `consts.push(c);` become:

  ```rust
                    if let Some(mut c) = self.parse_const_decl(vis) {
                        c.docs = docs;
                        consts.push(c);
  ```

**Extern functions.** In `parse_extern_block`, replace

```rust
        while !self.check(&Token::RBrace) && !self.is_at_end() {
            if let Some(sig) = self.parse_function_sig() {
                self.eat(&Token::Semicolon);
                items.push(ExternItem::Fn(sig));
```

with

```rust
        while !self.check(&Token::RBrace) && !self.is_at_end() {
            let docs = self.take_docs();
            if self.check(&Token::RBrace) || self.is_at_end() {
                if let Some(first) = docs.first() {
                    self.misplaced_doc(first.span);
                }
                continue;
            }
            if let Some(mut sig) = self.parse_function_sig() {
                sig.docs = docs;
                self.eat(&Token::Semicolon);
                items.push(ExternItem::Fn(sig));
```

- [ ] **Step 7: Docs on nested items; misplaced docs everywhere else**

**Blocks.** In `parse_block`, insert at the top of the loop body, before the
comment `// Check if this could be the trailing expression.`:

```rust
            // A `///` documents a nested item. Anywhere else in a block it is
            // misplaced (spec §4).
            if matches!(self.peek(), Token::DocComment(_)) && !self.is_item_start_at(self.docs_end()) {
                self.reject_docs();
                continue;
            }
```

**Nested items.** In `try_parse_stmt`, replace the whole
`if matches!( self.peek(), Token::Fn | … ) || (self.check(&Token::Pub) && { … }) {`
condition with:

```rust
        // Nested items, documented or not (spec §4)
        if self.is_item_start_at(self.docs_end()) {
```

keeping its body: `let item = self.try_parse_item()?;` and the rest.

**Match arms.** In `parse_match_expr`, change the loop body's first line,
`if let Some(arm) = self.parse_match_arm(ctx) {`, to:

```rust
            if self.reject_docs() {
                continue;
            }
            if let Some(arm) = self.parse_match_arm(ctx) {
```

**Parameters.** In `parse_params`, insert at the top of the loop body:

```rust
            if self.reject_docs() {
                continue;
            }
```

and do the same in `parse_closure`'s parameter loop, which starts
`while !self.check(&Token::Pipe) && !self.is_at_end() {`.

- [ ] **Step 8: Pattern spans, and `where`'s trailing comma**

In `parse_pattern_atom`'s `Token::LParen` arm, replace

```rust
                if self.eat(&Token::RParen).is_some() {
                    return Some(Spanned::new(Pattern::Tuple(vec![]), start));
                }
                let first = self.parse_pattern(ctx)?;
                if self.eat(&Token::RParen).is_some() {
                    return Some(Spanned::new(first.value, start));
                }
```

with

```rust
                // Spans over the parentheses, as for expressions and types,
                // so the formatter can keep them (spec §4).
                if let Some(close) = self.eat(&Token::RParen) {
                    return Some(Spanned::new(Pattern::Tuple(vec![]), start.merge(close.span)));
                }
                let first = self.parse_pattern(ctx)?;
                if let Some(close) = self.eat(&Token::RParen) {
                    return Some(Spanned::new(first.value, start.merge(close.span)));
                }
```

In `parse_where_clause_opt`, replace

```rust
            if self.eat(&Token::Comma).is_none() {
                break;
            }
        }
        bounds
```

with

```rust
            if self.eat(&Token::Comma).is_none() {
                break;
            }
            // A trailing comma (spec §4): the clause ends at the body or the
            // end of a signature.
            if matches!(self.peek(), Token::LBrace | Token::Semicolon | Token::RBrace) {
                break;
            }
        }
        bounds
```

- [ ] **Step 9: The one consumer that names every field**

In `crates/nova-typeck/src/check.rs:779`, change
`TraitItem::AssocType { name, bounds } => {` to
`TraitItem::AssocType { name, bounds, .. } => {`.

```bash
cd /d/Projects/nona/nova && cargo build --locked --workspace 2>&1 | grep -E "^(error|warning: unused)" | head -10; echo "build exit=${PIPESTATUS[0]}"
```

Expected: `build exit=0`. If another crate fails to compile because it builds
or destructures one of the 14 nodes, add `docs: Vec::new()` or `..` there,
and ledger each place as a ruling.

- [ ] **Step 10: Run the parser's tests, and accept the six snapshots**

```bash
cd /d/Projects/nona/nova && INSTA_UPDATE=always cargo test --locked -p nova-parser 2>&1 | grep -E "^test result|FAILED|panicked" | head -20 && git diff --stat -- crates/nova-parser/tests/snapshots && git diff -U0 -- crates/nova-parser/tests/snapshots | grep -E "^[-+] " | grep -vcE "^\+ +docs: \[\],$"
```

Expected:
- every `test result:` line is `ok`, with 0 failed;
- exactly the six `.snap` files changed;
- the last count is `0`: every changed line is an added `docs: [],`.

Then run the parser's tests once more without `INSTA_UPDATE`, and expect 0
failed.

- [ ] **Step 11: The whole suite**

The pattern spans and the new error reach every crate's diagnostics, so run
everything, after checking port 3000:

```bash
cd /d/Projects/nona/nova && P=/c/Users/SAKEER~1/AppData/Local/Temp/gcm/p31 && netstat -ano | grep -E "[:.]3000 .*LISTENING"; cargo test --locked --workspace --all-features --no-fail-fast > $P/suite-2.txt 2>&1; echo "exit=$?"
```

Count it. Expected: 0 failed, and the passed count is Task 1's baseline plus
Task 1's 8 lexer tests and these 8 parser tests.

- [ ] **Step 12: Commit**

Write `$P/msg-2.txt`:

```
nova-parser: doc comments on items, members, fields and variants

`///` lines become `docs` on the 14 AST nodes they can precede (spec
docs/superpowers/specs/2026-10-07-phase-3-1-formatter-design.md §4):
items, top-level and nested; trait and impl members; record fields;
sum-type variants; and extern functions. A top-level item's docs may
sit before, between or after its attributes. A `///` anywhere else is
an error.

Two grammar fixes the formatter needs:
- A parenthesised pattern's span covers its parentheses, as an
  expression's and a type's already do.
- A `where` clause accepts a trailing comma before `{`, `;` or `}`.

The six AST snapshots gain `docs: []`.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
```

```bash
cd /d/Projects/nona/nova && P=/c/Users/SAKEER~1/AppData/Local/Temp/gcm/p31 && cargo fmt --all && git add crates/nova-ast crates/nova-parser crates/nova-typeck && git status --short && git commit -q -F $P/msg-2.txt && git log -1 --format=%s
```

Expected: only those three crates are staged, and the subject line is
`nova-parser: doc comments on items, members, fields and variants`.

The task's test command: `cargo test --locked --no-fail-fast -p nova-parser -p nova-typeck`.

---
### Task 3: The document and its renderer

Spec §5.2. Wadler's pretty printer, written here, with no dependency.

**Files:**
- Create: `crates/nova-fmt/src/doc.rs`
- Modify: `crates/nova-fmt/src/lib.rs` (replace the one-line stub)

**Interfaces:**
- Consumes: nothing.
- Produces, in `crate::doc`:
  - `pub(crate) enum Doc { Nil, Text(String), Verbatim(String), Line, SoftLine, HardLine, Nest(Box<Doc>), Group(Box<Doc>, bool), IfBreak(Box<Doc>, Box<Doc>), LineSuffix(String), BreakParent, Concat(Vec<Doc>) }`, deriving `Clone` and `Debug`
  - `pub(crate) fn text(s: impl Into<String>) -> Doc`, `concat(Vec<Doc>) -> Doc`, `group(Doc) -> Doc`, `nest(Doc) -> Doc`, `if_break(broken: Doc, flat: Doc) -> Doc`
  - `pub(crate) fn render(doc: Doc, width: usize) -> String`
  - `pub(crate) const INDENT: usize = 4`

- [ ] **Step 1: Write the renderer's tests, and a module that has only them**

Replace `crates/nova-fmt/src/lib.rs` with:

```rust
//! The Nova formatter (spec
//! `docs/superpowers/specs/2026-10-07-phase-3-1-formatter-design.md`).

mod doc;
```

Create `crates/nova-fmt/src/doc.rs` holding only the tests for now:

```rust
//! Wadler's pretty printer (spec
//! `docs/superpowers/specs/2026-10-07-phase-3-1-formatter-design.md` §5.2).

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_group_that_fits_is_flat() {
        let doc = group(concat(vec![text("a"), Doc::Line, text("b")]));
        assert_eq!(render(doc, 3), "a b");
    }

    #[test]
    fn a_group_that_does_not_fit_breaks() {
        let doc = group(concat(vec![text("a"), Doc::Line, text("b")]));
        assert_eq!(render(doc, 2), "a\nb");
    }

    #[test]
    fn nest_indents_after_each_break() {
        let doc = group(concat(vec![
            text("f("),
            nest(concat(vec![Doc::SoftLine, text("x")])),
            Doc::SoftLine,
            text(")"),
        ]));
        assert_eq!(render(doc.clone(), 4), "f(x)");
        assert_eq!(render(doc, 3), "f(\n    x\n)");
    }

    #[test]
    fn if_break_follows_its_group() {
        let doc = group(concat(vec![
            text("[a"),
            nest(concat(vec![Doc::SoftLine, text("b"), if_break(text(","), Doc::Nil)])),
            Doc::SoftLine,
            text("]"),
        ]));
        assert_eq!(render(doc.clone(), 4), "[ab]");
        assert_eq!(render(doc, 3), "[a\n    b,\n]");
    }

    #[test]
    fn a_hard_line_breaks_every_group_around_it() {
        let doc = group(concat(vec![
            text("a"),
            Doc::Line,
            group(concat(vec![text("b"), Doc::HardLine, text("c")])),
        ]));
        assert_eq!(render(doc, 100), "a\nb\nc");
    }

    #[test]
    fn a_line_suffix_waits_for_the_next_line_break() {
        let doc = concat(vec![
            text("a"),
            Doc::LineSuffix(" // c".into()),
            text(","),
            Doc::HardLine,
            text("b"),
        ]);
        assert_eq!(render(doc, 100), "a, // c\nb");
    }

    #[test]
    fn a_line_suffix_left_at_the_end_is_printed() {
        let doc = concat(vec![text("a"), Doc::LineSuffix(" // end".into())]);
        assert_eq!(render(doc, 100), "a // end");
    }

    #[test]
    fn no_line_ends_in_added_whitespace_and_blank_lines_stay_empty() {
        let doc = nest(concat(vec![text("a "), Doc::HardLine, Doc::HardLine, text("b")]));
        assert_eq!(render(doc, 100), "a\n\n    b");
    }

    #[test]
    fn verbatim_text_counts_only_its_first_line_and_keeps_its_own() {
        let doc = group(concat(vec![
            text("f("),
            nest(concat(vec![Doc::SoftLine, Doc::Verbatim("\"a\n  b\"".into())])),
            Doc::SoftLine,
            text(")"),
        ]));
        assert_eq!(render(doc, 5), "f(\"a\n  b\")");
    }

    #[test]
    fn columns_count_characters_not_bytes() {
        // Ten Thai letters are 30 bytes but 10 columns.
        let thai = "ก".repeat(10);
        let doc = group(concat(vec![text(thai.clone()), Doc::Line, text("x")]));
        assert_eq!(render(doc.clone(), 12), format!("{thai} x"));
        assert_eq!(render(doc, 11), format!("{thai}\nx"));
    }
}
```

- [ ] **Step 2: Run them to watch them fail**

```bash
cd /d/Projects/nona/nova && cargo test --locked -p nova-fmt --lib 2>&1 | grep -E "^error(\[E[0-9]+\])?:" | sort | uniq -c | head -5
```

Expected: errors that `group`, `concat`, `text`, `render` and `Doc` cannot be
found.

- [ ] **Step 3: Write the document and the renderer**

In `crates/nova-fmt/src/doc.rs`, insert between the module comment and the
tests:

```rust
//!
//! A [`Doc::Group`] prints flat, with every [`Doc::Line`] a space and every
//! [`Doc::SoftLine`] nothing, when it fits in the rest of the line, and
//! broken, each of them a line break, when it does not. A group holding a
//! [`Doc::HardLine`] or [`Doc::BreakParent`] is always broken.

/// Spaces per indentation level (spec §6).
pub(crate) const INDENT: usize = 4;

/// A document.
#[derive(Clone, Debug)]
pub(crate) enum Doc {
    Nil,
    /// Text with no line break.
    Text(String),
    /// Text that may hold line breaks, printed exactly as given: a multi-line
    /// string literal or block comment. Only its first line counts when
    /// fitting, and its inner lines are never re-indented.
    Verbatim(String),
    /// A space when its group is flat, a line break when broken.
    Line,
    /// Nothing when its group is flat, a line break when broken.
    SoftLine,
    /// Always a line break. It breaks every group around it.
    HardLine,
    /// One more level of indentation after each line break inside.
    Nest(Box<Doc>),
    /// Flat if it fits, otherwise broken. The flag says it must break;
    /// [`render`] works it out before printing.
    Group(Box<Doc>, bool),
    /// The first document when the enclosing group is broken, the second
    /// when it is flat.
    IfBreak(Box<Doc>, Box<Doc>),
    /// Text printed just before the next line break, so that a trailing line
    /// comment goes after the `,` printed after it.
    LineSuffix(String),
    /// Breaks every group around it.
    BreakParent,
    Concat(Vec<Doc>),
}

pub(crate) fn text(s: impl Into<String>) -> Doc {
    Doc::Text(s.into())
}

pub(crate) fn concat(parts: Vec<Doc>) -> Doc {
    Doc::Concat(parts)
}

pub(crate) fn group(doc: Doc) -> Doc {
    Doc::Group(Box::new(doc), false)
}

pub(crate) fn nest(doc: Doc) -> Doc {
    Doc::Nest(Box::new(doc))
}

pub(crate) fn if_break(broken: Doc, flat: Doc) -> Doc {
    Doc::IfBreak(Box::new(broken), Box::new(flat))
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Mode {
    Flat,
    Break,
}

/// Render `doc` with lines `width` columns wide, counted in Unicode scalar
/// values. No line ends in whitespace the renderer added.
pub(crate) fn render(mut doc: Doc, width: usize) -> String {
    propagate(&mut doc);
    let mut out = Out::default();
    let mut stack: Vec<(usize, Mode, &Doc)> = vec![(0, Mode::Break, &doc)];
    while let Some((indent, mode, d)) = stack.pop() {
        match d {
            Doc::Nil | Doc::BreakParent => {}
            Doc::Text(s) | Doc::Verbatim(s) => out.write(s),
            Doc::Concat(parts) => stack.extend(parts.iter().rev().map(|p| (indent, mode, p))),
            Doc::Nest(inner) => stack.push((indent + INDENT, mode, inner)),
            Doc::Group(inner, broken) => {
                let left = width as isize - out.col as isize;
                let flat = mode == Mode::Flat || (!*broken && fits(left, indent, inner, &stack));
                stack.push((indent, if flat { Mode::Flat } else { Mode::Break }, inner));
            }
            Doc::IfBreak(broken, flat) => {
                stack.push((indent, mode, if mode == Mode::Break { broken } else { flat }));
            }
            Doc::LineSuffix(s) => out.suffix.push(s.clone()),
            Doc::Line if mode == Mode::Flat => out.write(" "),
            Doc::SoftLine if mode == Mode::Flat => {}
            Doc::Line | Doc::SoftLine | Doc::HardLine => out.newline(indent),
        }
    }
    out.finish()
}

/// Whether `first`, flat, and then the rest of its line fit in `left`
/// columns. The rest is `rest`, read from its top, in the modes it was
/// pushed with, up to the first line break.
fn fits(mut left: isize, indent: usize, first: &Doc, rest: &[(usize, Mode, &Doc)]) -> bool {
    let mut work: Vec<(usize, Mode, &Doc)> = vec![(indent, Mode::Flat, first)];
    let mut rest_at = rest.len();
    loop {
        if left < 0 {
            return false;
        }
        let (indent, mode, d) = match work.pop() {
            Some(item) => item,
            None if rest_at == 0 => return true,
            None => {
                rest_at -= 1;
                rest[rest_at]
            }
        };
        match d {
            Doc::Nil | Doc::BreakParent | Doc::LineSuffix(_) => {}
            Doc::Text(s) => left -= s.chars().count() as isize,
            Doc::Verbatim(s) => match s.find('\n') {
                Some(i) => return left >= s[..i].chars().count() as isize,
                None => left -= s.chars().count() as isize,
            },
            Doc::Concat(parts) => work.extend(parts.iter().rev().map(|p| (indent, mode, p))),
            Doc::Nest(inner) => work.push((indent + INDENT, mode, inner)),
            Doc::Group(inner, broken) => {
                work.push((indent, if *broken { Mode::Break } else { mode }, inner));
            }
            Doc::IfBreak(broken, flat) => {
                work.push((indent, mode, if mode == Mode::Break { broken } else { flat }));
            }
            Doc::Line if mode == Mode::Flat => left -= 1,
            Doc::SoftLine if mode == Mode::Flat => {}
            Doc::Line | Doc::SoftLine | Doc::HardLine => return true,
        }
    }
}

/// Mark every group that holds a hard break as broken, and say whether
/// `doc` holds one.
fn propagate(doc: &mut Doc) -> bool {
    match doc {
        Doc::HardLine | Doc::BreakParent => true,
        Doc::Nest(inner) => propagate(inner),
        Doc::Group(inner, broken) => {
            let hard = propagate(inner);
            *broken |= hard;
            hard
        }
        Doc::IfBreak(broken, flat) => {
            let a = propagate(broken);
            let b = propagate(flat);
            a || b
        }
        Doc::Concat(parts) => {
            let mut hard = false;
            for p in parts {
                hard |= propagate(p);
            }
            hard
        }
        _ => false,
    }
}

/// The text being written.
#[derive(Default)]
struct Out {
    text: String,
    /// The column the next character lands in, pending indentation included.
    col: usize,
    /// Indentation still to be written before the next text on this line.
    pending: Option<usize>,
    /// Line suffixes waiting for the next line break.
    suffix: Vec<String>,
}

impl Out {
    fn write(&mut self, s: &str) {
        if s.is_empty() {
            return;
        }
        if let Some(n) = self.pending.take() {
            self.text.extend(std::iter::repeat(' ').take(n));
        }
        self.text.push_str(s);
        match s.rfind('\n') {
            Some(i) => self.col = s[i + 1..].chars().count(),
            None => self.col += s.chars().count(),
        }
    }

    fn newline(&mut self, indent: usize) {
        for s in std::mem::take(&mut self.suffix) {
            self.write(&s);
        }
        while self.text.ends_with(' ') {
            self.text.pop();
        }
        self.text.push('\n');
        self.col = indent;
        self.pending = Some(indent);
    }

    fn finish(mut self) -> String {
        for s in std::mem::take(&mut self.suffix) {
            self.write(&s);
        }
        self.text
    }
}
```

- [ ] **Step 4: Run them to watch them pass**

```bash
cd /d/Projects/nona/nova && cargo test --locked -p nova-fmt --lib 2>&1 | grep -E "^test result|FAILED|panicked"
```

Expected: `test result: ok. 10 passed; 0 failed`. Warnings that the module's
items are never used outside its tests are expected until Task 5.

- [ ] **Step 5: Commit**

Write `$P/msg-3.txt`:

```
nova-fmt: a Wadler document and its renderer

The formatter's document (spec
docs/superpowers/specs/2026-10-07-phase-3-1-formatter-design.md §5.2):
text, line breaks a group may take or leave, hard breaks, nesting, and
two additions:
- line suffixes, so that a trailing comment lands after the `,` printed
  after it;
- verbatim text, so that a multi-line string is measured by its first
  line and never re-indented.

Width is counted in Unicode scalar values, and no line ends in
whitespace the renderer added. It is our own, about 250 lines, so
nothing joins Cargo.lock.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
```

```bash
cd /d/Projects/nona/nova && P=/c/Users/SAKEER~1/AppData/Local/Temp/gcm/p31 && cargo fmt --all && git add crates/nova-fmt && git commit -q -F $P/msg-3.txt && git log -1 --format=%s
```

Expected: `nova-fmt: a Wadler document and its renderer`.

The task's test command: `cargo test --locked -p nova-fmt --lib`.

---
### Task 4: The parsed input and the self-check

Spec §5.1 and §5.5. `format_with` takes the printer as an argument, so the
self-check's tests can hand it broken printers. The real printer arrives in
Task 5.

**Files:**
- Create: `crates/nova-fmt/src/source.rs`, `crates/nova-fmt/src/check.rs`
- Modify: `crates/nova-fmt/src/lib.rs`
- Modify: `crates/nova-fmt/Cargo.toml` (`thiserror`), and `Cargo.lock`
  through one unlocked build

**Interfaces:**
- Consumes: Task 1's `nova_lexer::lex_with_comments`, `Comment` and
  `CommentKind`, and Task 2's AST.
- Produces:
  - `pub enum FormatError { Syntax { diagnostics: Vec<Diagnostic>, rendered: String }, Internal { first_difference: String } }`,
    deriving `Debug` and `thiserror::Error`, as the master spec's §5.1
    asks of every error type
  - `pub const WIDTH: usize = 100`
  - `fn format_with(source: &str, name: &str, print: fn(&source::Source) -> String) -> Result<String, FormatError>`,
    private to the crate
  - in `crate::source`: `pub(crate) struct Source<'t> { pub text: &'t str, pub file: nova_ast::File, pub tokens: Vec<Spanned<Token>>, pub comments: Vec<Comment> }`,
    with `Source::parse(text: &'t str, name: &str) -> Result<Source<'t>, FormatError>`
    and `Source::slice(&self, span: Span) -> &'t str`
  - in `crate::check`: `check(input: &Source, output: &str) -> Result<(), FormatError>`,
    `import_key(item: &Item) -> String`, `import_runs(file: &File) -> Vec<Range<usize>>`,
    `fingerprint(file: &File) -> String` and `strip_spans(debug: &str) -> String`

- [ ] **Step 1: Write the failing tests**

Replace `crates/nova-fmt/src/lib.rs` with:

```rust
//! The Nova formatter (spec
//! `docs/superpowers/specs/2026-10-07-phase-3-1-formatter-design.md`).

mod check;
mod doc;
mod source;

#[cfg(test)]
mod tests {
    use super::*;

    /// Prints the input unchanged: always passes the check.
    fn unchanged(src: &source::Source) -> String {
        src.text.to_owned()
    }

    /// Drops every line comment.
    fn without_comments(src: &source::Source) -> String {
        src.text
            .lines()
            .filter(|l| !l.trim_start().starts_with("//"))
            .map(|l| format!("{l}\n"))
            .collect()
    }

    /// Renames `main`: a different program.
    fn renamed(src: &source::Source) -> String {
        src.text.replace("main", "mane")
    }

    /// Prints something that does not parse.
    fn broken(_: &source::Source) -> String {
        "fn (".to_owned()
    }

    /// Prints imports sorted, as the printer does (spec §6).
    fn sorted(_: &source::Source) -> String {
        "import alpha::{a, b}\nimport zeta\n".to_owned()
    }

    /// Sorts two commented imports, moving each comment with its import.
    fn moved(_: &source::Source) -> String {
        "// a\nimport alpha\n// z\nimport zeta\n".to_owned()
    }

    /// Swaps two comments that are not in an import run.
    fn swapped(_: &source::Source) -> String {
        "// b\n// a\nfn main() {}\n".to_owned()
    }

    fn internal(result: Result<String, FormatError>) -> bool {
        matches!(result, Err(FormatError::Internal { .. }))
    }

    #[test]
    fn output_that_keeps_the_program_and_its_comments_passes() {
        let out = format_with("// c\nfn main() {}\n", "<t>", unchanged).unwrap();
        assert_eq!(out, "// c\nfn main() {}\n");
    }

    #[test]
    fn crlf_input_reaches_the_printer_as_lf() {
        let out = format_with("fn main() {}\r\n", "<t>", unchanged).unwrap();
        assert_eq!(out, "fn main() {}\n");
    }

    #[test]
    fn the_self_check_refuses_output_that_lost_a_comment() {
        assert!(internal(format_with("// keep me\nfn main() {}\n", "<t>", without_comments)));
    }

    #[test]
    fn the_self_check_refuses_output_that_changed_the_program() {
        assert!(internal(format_with("fn main() {}\n", "<t>", renamed)));
    }

    #[test]
    fn the_self_check_refuses_output_that_does_not_parse() {
        assert!(internal(format_with("fn main() {}\n", "<t>", broken)));
    }

    #[test]
    fn sorting_imports_is_not_a_change() {
        assert!(format_with("import zeta\nimport alpha::{b, a}\n", "<t>", sorted).is_ok());
    }

    #[test]
    fn comments_in_an_import_run_may_move_with_their_imports() {
        assert!(format_with("// z\nimport zeta\n// a\nimport alpha\n", "<t>", moved).is_ok());
    }

    #[test]
    fn a_comment_moved_out_of_order_elsewhere_is_refused() {
        assert!(internal(format_with("// a\n// b\nfn main() {}\n", "<t>", swapped)));
    }

    #[test]
    fn a_syntax_error_is_reported_as_nova_check_would() {
        match format_with("fn main( {\n", "bad.nova", unchanged) {
            Err(FormatError::Syntax { diagnostics, rendered }) => {
                assert!(!diagnostics.is_empty());
                assert!(rendered.contains("P0001"), "{rendered}");
                assert!(rendered.contains("bad.nova"), "{rendered}");
            }
            other => panic!("{other:?}"),
        }
    }
}
```

Create `crates/nova-fmt/src/check.rs` with one test of its own:

```rust
//! The self-check (spec
//! `docs/superpowers/specs/2026-10-07-phase-3-1-formatter-design.md` §5.5).

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strip_spans_removes_every_span_field() {
        let debug = "Spanned { value: Ident(\"x\"), span: Span { start: 0, end: 1, file: FileId(0) } }";
        assert_eq!(strip_spans(debug), "Spanned { value: Ident(\"x\") }");
    }
}
```

- [ ] **Step 2: Run them to watch them fail**

```bash
cd /d/Projects/nona/nova && cargo test --locked -p nova-fmt --lib 2>&1 | grep -E "^error(\[E[0-9]+\])?:" | sort | uniq -c | head -8
```

Expected: errors that the file for module `source` is not found (E0583), and
that `format_with`, `FormatError` and `strip_spans` cannot be found.

- [ ] **Step 3: The parsed input**

Create `crates/nova-fmt/src/source.rs`:

```rust
//! The input, lexed and parsed (spec
//! `docs/superpowers/specs/2026-10-07-phase-3-1-formatter-design.md` §5.1),
//! with the source positions the printer reads what the AST forgets from
//! (§5.3).

use nova_ast::File;
use nova_diagnostics::{render, Diagnostic, FileDb, Span, Spanned};
use nova_lexer::{Comment, Token};

use crate::FormatError;

/// An input, lexed and parsed: its text, its tokens, its comments and its
/// AST.
pub(crate) struct Source<'t> {
    pub text: &'t str,
    pub file: File,
    pub tokens: Vec<Spanned<Token>>,
    pub comments: Vec<Comment>,
}

impl<'t> Source<'t> {
    /// Lex and parse `text`, naming it `name` in any diagnostics. Any lex or
    /// parse error is [`FormatError::Syntax`], holding the diagnostics
    /// `nova check` would show.
    pub fn parse(text: &'t str, name: &str) -> Result<Self, FormatError> {
        let mut db = FileDb::new();
        let id = db.add(name, text);
        let (tokens, comments, lex_errors) = nova_lexer::lex_with_comments(text, id);
        let (file, parse_errors) = nova_parser::parse(&tokens, id);
        let mut diagnostics: Vec<Diagnostic> = lex_errors
            .iter()
            .map(|e| Diagnostic::error("L0001", e.to_string()).with_primary_label(e.span(), "here"))
            .collect();
        diagnostics.extend(parse_errors.iter().map(|e| {
            Diagnostic::error("P0001", e.to_string()).with_primary_label(e.span(), "here")
        }));
        match file {
            Some(file) if diagnostics.is_empty() => Ok(Self {
                text,
                file,
                tokens,
                comments,
            }),
            _ => {
                let rendered = render::render_to_string(&db, &diagnostics);
                Err(FormatError::Syntax {
                    diagnostics,
                    rendered,
                })
            }
        }
    }

    /// The source text of `span`.
    pub fn slice(&self, span: Span) -> &'t str {
        &self.text[span.start as usize..span.end as usize]
    }
}
```

The codes and labels are the ones `nova check` uses
(`crates/nova-driver/src/lib.rs:516` and `:525`).

- [ ] **Step 4: The self-check**

In `crates/nova-fmt/src/check.rs`, insert between the module comment and the
tests:

```rust

use std::ops::Range;

use nova_ast::item::{Import, ImportKind};
use nova_ast::{File, Item};
use nova_lexer::{Comment, CommentKind};

use crate::source::Source;
use crate::FormatError;

/// Check `output` against `input`: it must parse, to the same AST, with the
/// same comments. Any difference is [`FormatError::Internal`], naming the
/// first one.
pub(crate) fn check(input: &Source, output: &str) -> Result<(), FormatError> {
    let out = Source::parse(output, "<formatted>").map_err(|e| match e {
        FormatError::Syntax { rendered, .. } => {
            internal(format!("the output does not parse:\n{rendered}"))
        }
        other => other,
    })?;
    let (before, after) = (fingerprint(&input.file), fingerprint(&out.file));
    if before != after {
        return Err(internal(first_difference("the AST", &before, &after)));
    }
    let (before, after) = (comment_record(input), comment_record(&out));
    if before.ordered != after.ordered {
        let at = before
            .ordered
            .iter()
            .zip(&after.ordered)
            .position(|(a, b)| a != b)
            .unwrap_or(before.ordered.len().min(after.ordered.len()));
        return Err(internal(format!(
            "the comments changed at comment {at}: before {:?}, after {:?}",
            before.ordered.get(at),
            after.ordered.get(at)
        )));
    }
    if before.in_runs != after.in_runs {
        return Err(internal(
            "the comments inside a run of imports changed".to_owned(),
        ));
    }
    Ok(())
}

fn internal(first_difference: String) -> FormatError {
    FormatError::Internal { first_difference }
}

/// The AST without its spans, after the import sorting the printer does
/// (spec §6). Two inputs have the same fingerprint exactly when they are
/// the same program.
pub(crate) fn fingerprint(file: &File) -> String {
    let mut file = file.clone();
    for run in import_runs(&file) {
        let items = &mut file.items[run];
        items.sort_by(|a, b| import_key(&a.value).cmp(&import_key(&b.value)));
        for item in items {
            if let Item::Import(Import {
                kind: ImportKind::List(names),
                ..
            }) = &mut item.value
            {
                names.sort_by(|a, b| a.value.cmp(&b.value));
            }
        }
    }
    strip_spans(&format!("{file:?}"))
}

/// The index ranges of the runs of consecutive top-level imports.
pub(crate) fn import_runs(file: &File) -> Vec<Range<usize>> {
    let mut runs = Vec::new();
    let mut i = 0;
    while i < file.items.len() {
        if matches!(file.items[i].value, Item::Import(_)) {
            let start = i;
            while i < file.items.len() && matches!(file.items[i].value, Item::Import(_)) {
                i += 1;
            }
            runs.push(start..i);
        } else {
            i += 1;
        }
    }
    runs
}

/// What an import sorts by: its path, as `a::b`.
pub(crate) fn import_key(item: &Item) -> String {
    match item {
        Item::Import(import) => import
            .path
            .value
            .segments
            .iter()
            .map(|s| s.value.as_str())
            .collect::<Vec<_>>()
            .join("::"),
        _ => String::new(),
    }
}

/// `debug`, a `{:?}` rendering, without its `, span: Span { … }` fields.
pub(crate) fn strip_spans(debug: &str) -> String {
    const FIELD: &str = ", span: Span { ";
    let mut out = String::with_capacity(debug.len());
    let mut rest = debug;
    while let Some(at) = rest.find(FIELD) {
        out.push_str(&rest[..at]);
        let after = &rest[at + FIELD.len()..];
        rest = match after.find(" }") {
            Some(end) => &after[end + 2..],
            None => "",
        };
    }
    out.push_str(rest);
    out
}

/// A file's comments, each as its kind and its text without trailing
/// whitespace on any line. Those outside import runs are kept in order.
/// Those inside each run are sorted, because sorting the run moves comments
/// with their imports (spec §5.5).
struct CommentRecord {
    ordered: Vec<String>,
    in_runs: Vec<Vec<String>>,
}

fn comment_record(src: &Source) -> CommentRecord {
    let items = &src.file.items;
    // A run's region: from the end of the item before it to the start of
    // the item after it.
    let regions: Vec<(u32, u32)> = import_runs(&src.file)
        .into_iter()
        .map(|run| {
            let start = if run.start == 0 { 0 } else { items[run.start - 1].span.end };
            let end = items
                .get(run.end)
                .map_or(src.text.len() as u32, |next| next.span.start);
            (start, end)
        })
        .collect();
    let mut record = CommentRecord {
        ordered: Vec::new(),
        in_runs: vec![Vec::new(); regions.len()],
    };
    for c in &src.comments {
        let entry = comment_entry(src, c);
        match regions
            .iter()
            .position(|&(s, e)| c.span.start >= s && c.span.start < e)
        {
            Some(k) => record.in_runs[k].push(entry),
            None => record.ordered.push(entry),
        }
    }
    for run in &mut record.in_runs {
        run.sort();
    }
    record
}

fn comment_entry(src: &Source, c: &Comment) -> String {
    let kind = match c.kind {
        CommentKind::Line => "line",
        CommentKind::Block => "block",
    };
    let lines: Vec<&str> = src.slice(c.span).lines().map(str::trim_end).collect();
    format!("{kind}: {}", lines.join("\n"))
}

/// Where `before` and `after` first differ, with some text either side.
fn first_difference(what: &str, before: &str, after: &str) -> String {
    let at = before
        .bytes()
        .zip(after.bytes())
        .position(|(a, b)| a != b)
        .unwrap_or(before.len().min(after.len()));
    format!(
        "{what} changed: before …{}… after …{}…",
        around(before, at),
        around(after, at)
    )
}

/// The text of `s` within 60 bytes of `at`, cut at character boundaries.
fn around(s: &str, at: usize) -> &str {
    let mut lo = at.saturating_sub(60).min(s.len());
    while !s.is_char_boundary(lo) {
        lo -= 1;
    }
    let mut hi = (at + 60).min(s.len());
    while !s.is_char_boundary(hi) {
        hi += 1;
    }
    &s[lo..hi]
}
```

- [ ] **Step 5: The API**

The master spec's §5.1 asks every error type to implement
`std::error::Error` through `thiserror`, which the lexer and parser already
use. In `crates/nova-fmt/Cargo.toml`, add after the `nova-parser` line:

```toml
thiserror = { workspace = true }
```

Then one cargo call without `--locked`:

```bash
cd /d/Projects/nona/nova && cargo build -p nova-fmt 2>&1 | tail -1 && git diff --stat -- Cargo.lock && git diff -U0 -- Cargo.lock | grep -E "^[-+] "
```

Expected: the build finishes, `Cargo.lock | 1 +`, and the one changed line
is `+ "thiserror",`, in `nova-fmt`'s dependency list. `thiserror` is
already in the lock, so no package is added.

In `crates/nova-fmt/src/lib.rs`, insert after `mod source;`:

```rust

use nova_diagnostics::Diagnostic;

/// The width lines are fitted in, in Unicode scalar values (spec §6).
pub const WIDTH: usize = 100;

/// Why formatting produced nothing.
#[derive(Debug, thiserror::Error)]
pub enum FormatError {
    /// The input does not lex or parse. `rendered` holds the diagnostics as
    /// `nova check` prints them, without colour.
    #[error("the input does not parse:\n{rendered}")]
    Syntax {
        /// The lexer's and the parser's diagnostics, as `nova check` makes
        /// them.
        diagnostics: Vec<Diagnostic>,
        /// Those diagnostics, rendered to text.
        rendered: String,
    },
    /// The self-check failed (spec §5.5): the output would have changed the
    /// program or lost a comment. `first_difference` says where.
    #[error("formatting would change the program or its comments: {first_difference}")]
    Internal {
        /// Where the output first differs from the input.
        first_difference: String,
    },
}

/// Format `source` with `print`, then check the output (spec §5.1, §5.5).
/// The public functions pass the real printer; the tests pass broken ones,
/// to watch the check refuse their output.
fn format_with(
    source: &str,
    name: &str,
    print: fn(&source::Source) -> String,
) -> Result<String, FormatError> {
    let text = source.replace("\r\n", "\n");
    let input = source::Source::parse(&text, name)?;
    let output = print(&input);
    check::check(&input, &output)?;
    Ok(output)
}
```

- [ ] **Step 6: Run them to watch them pass**

```bash
cd /d/Projects/nona/nova && cargo test --locked -p nova-fmt --lib 2>&1 | grep -E "^test result|FAILED|panicked"
```

Expected: `test result: ok. 20 passed; 0 failed`: the 10 renderer tests,
these 9, and `strip_spans_removes_every_span_field`.

- [ ] **Step 7: Commit**

Write `$P/msg-4.txt`:

```
nova-fmt: parse the input, and check the output against it

The formatter's self-check (spec
docs/superpowers/specs/2026-10-07-phase-3-1-formatter-design.md §5.5).
On every call, before returning, the output is lexed and parsed again,
and refused unless:
- it parses;
- its AST, without spans, equals the input's, after the import sorting
  the printer does;
- its comments equal the input's, in order. Inside a run of imports
  they are compared as a set, since sorting moves them with their
  imports.

A syntax error in the input is reported as nova check reports it, L0001
or P0001, also rendered to text for the caller. format_with takes the
printer as an argument; the tests hand it broken printers to watch the
check refuse their output.

FormatError implements std::error::Error through thiserror, as the
master spec's §5.1 asks; thiserror was already in the lock.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
```

```bash
cd /d/Projects/nona/nova && P=/c/Users/SAKEER~1/AppData/Local/Temp/gcm/p31 && cargo fmt --all && git add Cargo.lock crates/nova-fmt && git commit -q -F $P/msg-4.txt && git log -1 --format=%s
```

Expected: `nova-fmt: parse the input, and check the output against it`.

The task's test command: `cargo test --locked -p nova-fmt --lib`.

---
### Task 5: The printer

Spec §5.3, §5.6 and §6. Every node is printed, in source order. Comments come
in Task 6: until then the comment hooks at the end of `print/mod.rs` place
none, so an input with a comment fails the self-check. This task's tests
hold no comments.

**Files:**
- Create: `crates/nova-fmt/src/print/mod.rs`, `print/item.rs`,
  `print/expr.rs`, `print/pattern.rs`, `print/ty.rs`
- Modify: `crates/nova-fmt/src/source.rs` (position helpers),
  `crates/nova-fmt/src/lib.rs` (`mod print`, `format`, `format_named`)
- Create: `crates/nova-fmt/tests/common/mod.rs`, `tests/layout.rs`,
  `tests/source_rules.rs`

**Interfaces:**
- Consumes: Task 3's `doc` module, and Task 4's `Source`, `check::import_key`
  and `format_with`.
- Produces:
  - `pub fn format(source: &str) -> Result<String, FormatError>`, naming the
    input `<stdin>`
  - `pub fn format_named(source: &str, name: &str) -> Result<String, FormatError>`
  - `pub(crate) fn print::print(src: &Source) -> String`
  - for Task 6, these comment hooks on `Printer`, each a stub here:
    `take_comments_before(&mut self, offset: u32) -> Vec<Comment>`,
    `has_comment_before(&self, offset: u32) -> bool`,
    `skip_comments_within(&mut self, span: Span)`,
    `leading(&mut self, offset: u32) -> Doc`,
    `trailing(&mut self, offset: u32) -> Doc`,
    `dangling(&mut self, close: u32) -> Option<Doc>`
  - on `Source`: `token_from`, `token_end`, `last_code_token`,
    `start_with`, `parens_from`, `find_from`, `is_parenthesized`, `unparen`
    and `blank_line_in`

- [ ] **Step 1: Write the failing tests**

Create `crates/nova-fmt/tests/common/mod.rs`:

```rust
//! Helpers shared by the formatter's tests.

/// Format `input`, expect exactly `expected`, and expect formatting that
/// again to change nothing.
#[allow(dead_code)]
pub fn assert_formats(input: &str, expected: &str) {
    let first = nova_fmt::format(input)
        .unwrap_or_else(|e| panic!("formatting failed: {e:?}\n--- input ---\n{input}"));
    assert_eq!(first, expected, "\n--- input ---\n{input}\n--- got ---\n{first}");
    let second = nova_fmt::format(&first)
        .unwrap_or_else(|e| panic!("formatting the output failed: {e:?}"));
    assert_eq!(second, first, "formatting is not idempotent");
}

/// Expect `source` to be formatted already.
#[allow(dead_code)]
pub fn assert_stable(source: &str) {
    assert_formats(source, source);
}
```

Create `crates/nova-fmt/tests/layout.rs`:

```rust
//! The layout rules (spec
//! `docs/superpowers/specs/2026-10-07-phase-3-1-formatter-design.md` §6).

mod common;
use common::{assert_formats, assert_stable};

#[test]
fn a_function_that_fits_stays_on_one_line() {
    assert_formats(
        "fn add(a: Int, b: Int) -> Int {\n    a + b\n}\n",
        "fn add(a: Int, b: Int) -> Int { a + b }\n",
    );
}

#[test]
fn a_block_with_two_statements_breaks() {
    assert_formats(
        "fn main() { let x = 1\n println(\"${x}\") }\n",
        "fn main() {\n    let x = 1\n    println(\"${x}\")\n}\n",
    );
}

#[test]
fn exactly_one_blank_line_between_top_level_items() {
    assert_formats(
        "fn a() {}\nfn b() {}\n\n\n\nfn c() {}\n",
        "fn a() {}\n\nfn b() {}\n\nfn c() {}\n",
    );
}

#[test]
fn blank_lines_in_a_block_are_kept_once_and_dropped_at_its_edges() {
    assert_formats(
        "fn main() {\n\n    let a = 1\n\n\n    let b = 2\n    println(\"${a}${b}\")\n\n}\n",
        "fn main() {\n    let a = 1\n\n    let b = 2\n    println(\"${a}${b}\")\n}\n",
    );
}

#[test]
fn an_if_else_that_fits_stays_on_one_line() {
    assert_formats(
        "fn f(b: Bool) -> String {\n    if b {\n        \"true\"\n    } else {\n        \"false\"\n    }\n}\n",
        "fn f(b: Bool) -> String { if b { \"true\" } else { \"false\" } }\n",
    );
}

#[test]
fn an_if_else_breaks_all_its_blocks_together() {
    assert_formats(
        "fn f(b: Bool) {\n    if b { println(\"a\") } else { println(\"b\")\n println(\"c\") }\n}\n",
        "fn f(b: Bool) {\n    if b {\n        println(\"a\")\n    } else {\n        println(\"b\")\n        println(\"c\")\n    }\n}\n",
    );
}

#[test]
fn else_if_stays_on_the_closing_braces_line() {
    assert_formats(
        "fn f(n: Int) -> Int {\n    if n < 0 { 0 }\n    else if n == 0 { 1 }\n    else { 2 }\n}\n",
        "fn f(n: Int) -> Int { if n < 0 { 0 } else if n == 0 { 1 } else { 2 } }\n",
    );
}

#[test]
fn a_long_signature_breaks_its_parameters_and_its_body() {
    assert_formats(
        "pub fn connect(host: String, port: Int, timeout: Duration, retries: Int, verbose: Bool) -> Result<Connection, NetError> {\n    open(host)\n}\n",
        "pub fn connect(\n    host: String,\n    port: Int,\n    timeout: Duration,\n    retries: Int,\n    verbose: Bool,\n) -> Result<Connection, NetError> {\n    open(host)\n}\n",
    );
}

#[test]
fn a_body_too_long_for_the_line_breaks_but_its_signature_stays() {
    // From std/collections: the function does not fit on one line, but its
    // signature does, and so does its body's one expression.
    assert_stable(
        "pub fn get(self, i: Int) -> Option<T> {\n    if i < 0 { None } else { if i >= self.len { None } else { Some(self.data[i]) } }\n}\n",
    );
}

#[test]
fn a_record_that_fits_goes_on_one_line() {
    assert_formats(
        "pub record CryptoError {\n    pub kind: CryptoErrorKind\n    pub message: String\n}\n",
        "pub record CryptoError { pub kind: CryptoErrorKind, pub message: String }\n",
    );
}

#[test]
fn a_record_that_does_not_fit_gets_one_field_per_line_with_commas() {
    assert_formats(
        "record Config { name: String, version: String, description: String, license: String, repository: String }\n",
        "record Config {\n    name: String,\n    version: String,\n    description: String,\n    license: String,\n    repository: String,\n}\n",
    );
}

#[test]
fn blank_lines_between_fields_and_between_variants_are_kept() {
    // One blank line survives however many there were, and it breaks the
    // list (spec §5.3).
    assert_formats(
        "record P {\n    x: Int,\n\n\n    y: Int,\n}\n\ntype T =\n    | A\n\n    | B\n",
        "record P {\n    x: Int,\n\n    y: Int,\n}\n\ntype T =\n    | A\n\n    | B\n",
    );
}

#[test]
fn a_sum_type_that_fits_goes_on_one_line() {
    assert_formats("pub type Method =\n    | Get\n    | Post\n", "pub type Method = | Get | Post\n");
    assert_stable("pub type Option<T> = | Some(T) | None\n");
}

#[test]
fn a_sum_type_that_does_not_fit_gets_one_variant_per_line() {
    assert_formats(
        "pub type CryptoErrorKind = | EntropyUnavailable | InvalidLength | RequestTooLarge | InvalidKey | VerificationFailed\n",
        "pub type CryptoErrorKind =\n    | EntropyUnavailable\n    | InvalidLength\n    | RequestTooLarge\n    | InvalidKey\n    | VerificationFailed\n",
    );
}

#[test]
fn a_method_chain_breaks_before_each_call() {
    assert_formats(
        "fn main() {\n    let app = Router::new().get(\"/\", |_| Response::text(200, \"Hello from Nova!\")).get(\"/health\", |_| Response::json(status_ok()))\n    serve(app)\n}\n",
        "fn main() {\n    let app = Router::new()\n        .get(\"/\", |_| Response::text(200, \"Hello from Nova!\"))\n        .get(\"/health\", |_| Response::json(status_ok()))\n    serve(app)\n}\n",
    );
}

#[test]
fn a_long_condition_breaks_before_each_operator() {
    assert_formats(
        "fn f() -> Bool {\n    first_condition_is_true(a) && second_condition_is_true(b) && third_condition_is_true(c) && fourth(d)\n}\n",
        "fn f() -> Bool {\n    first_condition_is_true(a)\n        && second_condition_is_true(b)\n        && third_condition_is_true(c)\n        && fourth(d)\n}\n",
    );
}

#[test]
fn match_arms_go_one_per_line_without_commas() {
    assert_formats(
        "fn f(x: Int) -> String {\n    match x { 0 => \"zero\", 1 => \"one\", _ => \"many\" }\n}\n",
        "fn f(x: Int) -> String {\n    match x {\n        0 => \"zero\"\n        1 => \"one\"\n        _ => \"many\"\n    }\n}\n",
    );
}

#[test]
fn imports_are_sorted_within_a_run() {
    assert_formats(
        "import zeta\nimport alpha::{c, a, b}\n\nfn main() {}\n",
        "import alpha::{a, b, c}\nimport zeta\n\nfn main() {}\n",
    );
    // Blank lines between imports do not end their run, and do not survive
    // inside it (the 3.1 plan's decision 3).
    assert_formats("import b\n\n\nimport a\n", "import a\nimport b\n");
}

#[test]
fn imports_are_sorted_only_within_their_run() {
    assert_formats("import b\nfn f() {}\nimport a\n", "import b\n\nfn f() {}\n\nimport a\n");
}

#[test]
fn a_call_exactly_100_columns_wide_stays_on_one_line() {
    assert_stable(
        "fn main() {\n    report(first_argument_value, second_argument_value, third_argument_value, fourth_argument_value)\n    done()\n}\n",
    );
}

#[test]
fn a_call_101_columns_wide_puts_one_argument_per_line() {
    assert_formats(
        "fn main() {\n    reports(first_argument_value, second_argument_value, third_argument_value, fourth_argument_value)\n    done()\n}\n",
        "fn main() {\n    reports(\n        first_argument_value,\n        second_argument_value,\n        third_argument_value,\n        fourth_argument_value,\n    )\n    done()\n}\n",
    );
}

#[test]
fn small_forms_keep_their_usual_spacing() {
    assert_stable("fn main() { let t = (1,) }\n");
    assert_stable("fn f(p: P) -> P { P { x: 1, ..p } }\n");
    assert_stable("fn main() { for i in 0..=3 { println(\"${i}\") } }\n");
    assert_stable("fn f(x: Int) -> Int { -x + (x as Int) * 2 }\n");
    assert_stable("fn main() { let xs = [0; 4] }\n");
}

#[test]
fn closures_print_their_parameters_bare_or_typed() {
    assert_formats(
        "fn main() { let f = |a: Int, b| a + b\n let g = | | 1\n println(\"${f(1, 2)}${g()}\") }\n",
        "fn main() {\n    let f = |a: Int, b| a + b\n    let g = | | 1\n    println(\"${f(1, 2)}${g()}\")\n}\n",
    );
}

#[test]
fn trait_impl_and_extern_bodies_list_one_member_per_line() {
    assert_formats(
        "trait Shape { fn area(self) -> Float\n fn name(self) -> String { \"shape\" } }\nimpl Shape for Square { fn area(self) -> Float { self.side * self.side } }\nextern \"C\" { fn abs(x: Int) -> Int }\n",
        "trait Shape {\n    fn area(self) -> Float\n    fn name(self) -> String { \"shape\" }\n}\n\nimpl Shape for Square {\n    fn area(self) -> Float { self.side * self.side }\n}\n\nextern \"C\" {\n    fn abs(x: Int) -> Int\n}\n",
    );
}

#[test]
fn consts_and_type_aliases() {
    assert_formats(
        "pub const MAX: Int = 10\ntype Id = Int\n",
        "pub const MAX: Int = 10\n\ntype Id = Int\n",
    );
}

#[test]
fn a_where_clause_that_fits_stays_on_the_signature() {
    assert_stable("fn show<T: Display + Clone>(x: T) -> String where T: Debug { x.fmt() }\n");
}

#[test]
fn a_long_where_clause_takes_lines_of_its_own() {
    assert_formats(
        "fn f<T, U>(x: T, y: U) -> T where T: FirstVeryLongTraitName + SecondVeryLongTraitName, U: ThirdVeryLongTraitName + Fourth { x }\n",
        "fn f<T, U>(\n    x: T,\n    y: U,\n) -> T\nwhere\n    T: FirstVeryLongTraitName + SecondVeryLongTraitName,\n    U: ThirdVeryLongTraitName + Fourth,\n{\n    x\n}\n",
    );
}

#[test]
fn an_impl_keeps_its_traits_type_arguments() {
    assert_formats(
        "impl Into<String> for Name { fn into(self) -> String { self.value } }\n",
        "impl Into<String> for Name {\n    fn into(self) -> String { self.value }\n}\n",
    );
}

#[test]
fn doc_comments_print_before_attributes_and_on_fields_and_variants() {
    assert_formats(
        "@test\n/// Adds one.\n///   Indented.\nfn inc(x: Int) -> Int { x + 1 }\n",
        "/// Adds one.\n///   Indented.\n@test\nfn inc(x: Int) -> Int { x + 1 }\n",
    );
    assert_stable(
        "record P {\n    /// The x.\n    x: Int,\n}\n\ntype T =\n    /// First.\n    | A\n    | B\n",
    );
}
```

Create `crates/nova-fmt/tests/source_rules.rs`:

```rust
//! What the printer takes from the source, and the separator rule (spec
//! `docs/superpowers/specs/2026-10-07-phase-3-1-formatter-design.md` §5.3,
//! §5.6).

mod common;
use common::{assert_formats, assert_stable};

#[test]
fn literals_keep_their_spelling() {
    assert_formats(
        "fn main() { let a = 0xFF\n let b = 1_000_000\n let c = 1.5e3\n let d = 'x'\n let e = r\"raw\" }\n",
        "fn main() {\n    let a = 0xFF\n    let b = 1_000_000\n    let c = 1.5e3\n    let d = 'x'\n    let e = r\"raw\"\n}\n",
    );
}

#[test]
fn strings_are_printed_as_written() {
    assert_stable("fn main() { println(\"a ${ x  +  1 } b\") }\n");
}

#[test]
fn a_multi_line_string_is_never_reindented() {
    assert_formats(
        "fn main() {\n        let s = \"line one\n   line two\"\n        println(s)\n}\n",
        "fn main() {\n    let s = \"line one\n   line two\"\n    println(s)\n}\n",
    );
}

#[test]
fn crlf_input_comes_out_lf() {
    assert_formats(
        "fn main() {\r\n    let s = \"a\r\nb\"\r\n    println(s)\r\n}\r\n",
        "fn main() {\n    let s = \"a\nb\"\n    println(s)\n}\n",
    );
}

#[test]
fn the_authors_parentheses_are_kept() {
    assert_formats(
        "fn main() { let a = (b * c) + d\n let e = (p).hash()\n let f = ((g)) }\n",
        "fn main() {\n    let a = (b * c) + d\n    let e = (p).hash()\n    let f = (g)\n}\n",
    );
}

#[test]
fn a_parenthesised_literal_keeps_one_pair() {
    // A literal prints from its source text, which must not include the
    // parentheses around it.
    assert_formats(
        "fn main() { let a = ((1))\n let b = (\"s\")\n match a { (1) => 0, _ => 1 } }\n",
        "fn main() {\n    let a = (1)\n    let b = (\"s\")\n    match a {\n        (1) => 0\n        _ => 1\n    }\n}\n",
    );
}

#[test]
fn parentheses_the_parser_needs_are_kept() {
    assert_stable("fn main() { while (if m > 3 { false } else { true }) { m = m + 1 } }\n");
}

#[test]
fn a_parenthesised_tuple_loses_its_extra_pair() {
    assert_formats("fn main() { let t = ((1, 2)) }\n", "fn main() { let t = (1, 2) }\n");
}

#[test]
fn a_parenthesised_pattern_keeps_its_parentheses() {
    assert_formats(
        "fn main() { let (x) = 1\n println(\"${x}\") }\n",
        "fn main() {\n    let (x) = 1\n    println(\"${x}\")\n}\n",
    );
}

#[test]
fn tokens_that_would_merge_keep_a_space() {
    // In an expression `&` takes a postfix operand, so a reference to a
    // reference is written `&(&x)` and keeps its parentheses. A type has no
    // such rule, so `& &T` keeps its space.
    assert_formats(
        "fn main() { let f = | | 0\n let r = &(&x) }\n",
        "fn main() {\n    let f = | | 0\n    let r = &(&x)\n}\n",
    );
    assert_stable("fn f(x: & &Int, y: &mut &Int) {}\n");
}

#[test]
fn impl_members_keep_their_source_order() {
    assert_stable(
        "impl Shape for Square {\n    fn area(self) -> Float { 1.0 }\n    type Unit = Float\n    const SIDES: Int = 4\n    fn name(self) -> String { \"square\" }\n}\n",
    );
}

#[test]
fn a_function_type_prints_no_unit_return() {
    assert_formats(
        "fn f(g: fn(Int) -> ()) {}\nfn h(g: fn(Int)) {}\n",
        "fn f(g: fn(Int)) {}\n\nfn h(g: fn(Int)) {}\n",
    );
}

#[test]
fn a_record_pattern_ending_in_rest_takes_no_trailing_comma() {
    assert_formats(
        "fn f(p: Point) -> Int { match p { Point { first_coordinate, second_coordinate, third_coordinate, fourth_coordinate, .. } => first_coordinate } }\n",
        "fn f(p: Point) -> Int {\n    match p {\n        Point {\n            first_coordinate,\n            second_coordinate,\n            third_coordinate,\n            fourth_coordinate,\n            ..\n        } => first_coordinate\n    }\n}\n",
    );
}

#[test]
fn a_statement_starting_with_a_parenthesis_keeps_its_semicolon() {
    assert_formats(
        "fn main() { f(); (a, b).show() }\n",
        "fn main() {\n    f();\n    (a, b).show()\n}\n",
    );
}

#[test]
fn each_token_that_could_continue_a_statement_keeps_its_semicolon() {
    for (input, expected) in [
        ("fn main() { f(); [1, 2].len() }\n", "fn main() {\n    f();\n    [1, 2].len()\n}\n"),
        ("fn main() { let x = y; { z } }\n", "fn main() {\n    let x = y;\n    { z }\n}\n"),
        ("fn main() { let x = y; -z }\n", "fn main() {\n    let x = y;\n    -z\n}\n"),
        ("fn main() { let x = y; *p = 1 }\n", "fn main() {\n    let x = y;\n    *p = 1\n}\n"),
        ("fn main() { f(); &x }\n", "fn main() {\n    f();\n    &x\n}\n"),
        ("fn main() { f(); |x| x }\n", "fn main() {\n    f();\n    |x| x\n}\n"),
    ] {
        assert_formats(input, expected);
    }
}

#[test]
fn a_match_arm_starting_with_a_parenthesis_keeps_its_comma() {
    assert_formats(
        "fn main() { match p { a => 1, (b, c) => 2 } }\n",
        "fn main() {\n    match p {\n        a => 1,\n        (b, c) => 2\n    }\n}\n",
    );
}

#[test]
fn a_value_less_return_keeps_its_semicolon() {
    assert_formats("fn f() { return; x = 1 }\n", "fn f() {\n    return;\n    x = 1\n}\n");
    assert_formats(
        "fn main() { while true { break; x = 1 } }\n",
        "fn main() {\n    while true {\n        break;\n        x = 1\n    }\n}\n",
    );
}

#[test]
fn width_counts_characters_not_bytes() {
    // 4 + `report("` + 86 Thai letters + `")` is exactly 100 characters, and
    // 272 bytes.
    let thai = "ก".repeat(86);
    assert_stable(&format!("fn main() {{\n    a()\n    report(\"{thai}\")\n}}\n"));
}

#[test]
fn tab_indentation_becomes_spaces() {
    assert_formats(
        "fn main() {\n\tprintln(\"a\")\n\tprintln(\"b\")\n}\n",
        "fn main() {\n    println(\"a\")\n    println(\"b\")\n}\n",
    );
}

#[test]
fn an_empty_or_blank_input_formats_to_nothing() {
    assert_eq!(nova_fmt::format("").unwrap(), "");
    assert_eq!(nova_fmt::format("\n  \n\t\n").unwrap(), "");
}

#[test]
fn a_syntax_error_is_reported() {
    match nova_fmt::format("fn main( {\n") {
        Err(nova_fmt::FormatError::Syntax { rendered, .. }) => {
            assert!(rendered.contains("P0001"), "{rendered}")
        }
        other => panic!("{other:?}"),
    }
}
```

- [ ] **Step 2: Run them to watch them fail**

```bash
cd /d/Projects/nona/nova && cargo test --locked -p nova-fmt --test layout --test source_rules 2>&1 | grep -E "^error(\[E[0-9]+\])?:" | sort | uniq -c | head -5
```

Expected: errors that `format` cannot be found in `nova_fmt`.

- [ ] **Step 3: Positions in the source**

In `crates/nova-fmt/src/source.rs`, add inside `impl<'t> Source<'t>`, after
`slice`:

```rust

    /// The index of the first token that starts at or after `offset`.
    fn index_from(&self, offset: u32) -> usize {
        self.tokens.partition_point(|t| t.span.start < offset)
    }

    /// The first token that starts at or after `offset`: `Eof` past the end.
    pub fn token_from(&self, offset: u32) -> &Token {
        self.tokens
            .get(self.index_from(offset))
            .map_or(&Token::Eof, |t| &t.value)
    }

    /// Where a token of `kind` ends, if one is the first token at or after
    /// `offset`: the `,` after a list element, the `;` after a statement.
    pub fn token_end(&self, offset: u32, kind: &Token) -> Option<u32> {
        let t = self.tokens.get(self.index_from(offset))?;
        (t.value == *kind).then_some(t.span.end)
    }

    /// The last token ending at or before `end` that is not a `;`.
    pub fn last_code_token(&self, end: u32) -> Option<&Token> {
        let i = self.tokens.partition_point(|t| t.span.end <= end);
        self.tokens[..i]
            .iter()
            .rev()
            .map(|t| &t.value)
            .find(|t| **t != Token::Semicolon)
    }

    /// Where the token at `offset` starts after walking back over any tokens
    /// of `kinds`: a member's `pub fn`, `const` or `type`, a field's `pub`,
    /// a variant's `|`.
    pub fn start_with(&self, offset: u32, kinds: &[Token]) -> u32 {
        let mut i = self.index_from(offset);
        while i > 0 && kinds.contains(&self.tokens[i - 1].value) {
            i -= 1;
        }
        self.tokens.get(i).map_or(offset, |t| t.span.start)
    }

    /// The first `(` at or after `offset`, and its matching `)`: where each
    /// starts.
    pub fn parens_from(&self, offset: u32) -> Option<(u32, u32)> {
        let from = self.index_from(offset);
        let open = from + self.tokens[from..].iter().position(|t| t.value == Token::LParen)?;
        let mut depth = 0usize;
        for t in &self.tokens[open..] {
            if t.value == Token::LParen {
                depth += 1;
            } else if t.value == Token::RParen {
                depth -= 1;
                if depth == 0 {
                    return Some((self.tokens[open].span.start, t.span.start));
                }
            }
        }
        None
    }

    /// Where the first token of `kind`'s variant at or after `offset` starts.
    pub fn find_from(&self, offset: u32, kind: &Token) -> Option<u32> {
        self.tokens[self.index_from(offset)..]
            .iter()
            .find(|t| std::mem::discriminant(&t.value) == std::mem::discriminant(kind))
            .map(|t| t.span.start)
    }

    /// Whether `span` is wrapped in parentheses: it begins with `(`, and
    /// that `(`'s matching `)` ends it (spec §5.3).
    pub fn is_parenthesized(&self, span: Span) -> bool {
        let i = self.index_from(span.start);
        match self.tokens.get(i) {
            Some(t) if t.value == Token::LParen && t.span.start == span.start => {}
            _ => return false,
        }
        let mut depth = 0usize;
        for t in &self.tokens[i..] {
            if t.value == Token::LParen {
                depth += 1;
            } else if t.value == Token::RParen {
                depth -= 1;
                if depth == 0 {
                    return t.span.end == span.end;
                }
            }
        }
        false
    }

    /// `span` without the parentheses wrapped around it, however many pairs:
    /// `((x))` gives `x` (spec §5.3). With `own`, the innermost pair is the
    /// node's own syntax and stays: a tuple's `(a, b)`, or `()`.
    pub fn unparen(&self, span: Span, own: bool) -> Span {
        let mut outer = span;
        let mut inner = span;
        while self.is_parenthesized(inner) {
            outer = inner;
            let first = self.index_from(inner.start) + 1;
            let last = self.tokens.partition_point(|t| t.span.end <= inner.end) - 2;
            if first > last {
                break;
            }
            inner = Span {
                start: self.tokens[first].span.start,
                end: self.tokens[last].span.end,
                ..inner
            };
        }
        if own {
            outer
        } else {
            inner
        }
    }

    /// Whether the text between `a` and `b` holds a blank line: two line
    /// breaks with only spaces or tabs between them.
    pub fn blank_line_in(&self, a: u32, b: u32) -> bool {
        if a >= b {
            return false;
        }
        let mut seen = false;
        for c in self.text[a as usize..b as usize].chars() {
            match c {
                '\n' if seen => return true,
                '\n' => seen = true,
                ' ' | '\t' | '\r' => {}
                _ => seen = false,
            }
        }
        false
    }
```

- [ ] **Step 4: The walker's core**

Create `crates/nova-fmt/src/print/mod.rs`:

```rust
//! The AST, walked into a document (spec
//! `docs/superpowers/specs/2026-10-07-phase-3-1-formatter-design.md` §5.3,
//! §5.4 and §6).
//!
//! The printer reaches every node in source order, so a comment taken when
//! the printer reaches it lands in order. The comment hooks are at the end
//! of this file.

mod expr;
mod item;
mod pattern;
mod ty;

use nova_ast::{Item, Path};
use nova_diagnostics::{Span, Spanned};
use nova_lexer::{Comment, CommentKind, Token};

use crate::doc::{concat, if_break, nest, text, Doc};
use crate::source::Source;

/// Tokens that would continue the statement or match arm before them, since
/// the parser ignores line breaks (spec §5.6).
const CONTINUES: [Token; 7] = [
    Token::LParen,
    Token::LBracket,
    Token::LBrace,
    Token::Minus,
    Token::Star,
    Token::Amp,
    Token::Pipe,
];

/// Print `src`'s AST in the canonical layout (spec §6): `\n` line endings
/// and exactly one final newline, or nothing for an input with nothing in it.
pub(crate) fn print(src: &Source) -> String {
    let mut printer = Printer { src };
    let doc = printer.file();
    let out = crate::doc::render(doc, crate::WIDTH);
    let body = out.trim_end();
    if body.is_empty() {
        String::new()
    } else {
        format!("{body}\n")
    }
}

/// Walks one input's AST into a document.
pub(crate) struct Printer<'s, 't> {
    src: &'s Source<'t>,
}

/// How a vertical list spaces its elements (spec §5.3).
#[derive(Clone, Copy, PartialEq, Eq)]
enum Blank {
    /// The author's blank lines, at most one.
    Keep,
    /// Exactly one: between top-level items.
    One,
}

/// Whether a comma list's last element takes a comma (spec §6).
#[derive(Clone, Copy, PartialEq, Eq)]
enum LastComma {
    /// Only when the list is broken.
    Broken,
    /// Always: a one-element tuple, `(a,)`.
    Always,
    /// Never: after a `..` rest or a `..base`.
    Never,
}

/// How a comma list is printed (spec §6, "Comma lists").
#[derive(Clone, Copy)]
struct List {
    open: &'static str,
    close: &'static str,
    /// A space inside the delimiters when flat: `{ a, b }`, not `(a, b)`.
    spaced: bool,
    last: LastComma,
    /// Whether the author's blank lines between elements are kept, as
    /// between a record's fields (spec §5.3). One breaks the list.
    blanks: bool,
}

impl List {
    const PARENS: List = List {
        open: "(",
        close: ")",
        spaced: false,
        last: LastComma::Broken,
        blanks: false,
    };
    const BRACKETS: List = List {
        open: "[",
        close: "]",
        spaced: false,
        last: LastComma::Broken,
        blanks: false,
    };
    const ANGLES: List = List {
        open: "<",
        close: ">",
        spaced: false,
        last: LastComma::Broken,
        blanks: false,
    };
    const BRACES: List = List {
        open: "{",
        close: "}",
        spaced: true,
        last: LastComma::Broken,
        blanks: false,
    };
    /// A record's fields.
    const FIELDS: List = List {
        open: "{",
        close: "}",
        spaced: true,
        last: LastComma::Broken,
        blanks: true,
    };
    /// An import's `{a, b}`, which the corpus writes without spaces.
    const IMPORTS: List = List {
        open: "{",
        close: "}",
        spaced: false,
        last: LastComma::Broken,
        blanks: false,
    };

    fn last(self, last: LastComma) -> List {
        List { last, ..self }
    }
}

/// A vertical list being built, one element per line (spec §5.3).
struct Vertical {
    parts: Vec<Doc>,
    /// Where the last element, or the opening delimiter, ends.
    prev_end: u32,
    first: bool,
    blank: Blank,
}

impl Vertical {
    fn new(open: u32, blank: Blank) -> Vertical {
        Vertical {
            parts: Vec::new(),
            prev_end: open,
            first: true,
            blank,
        }
    }
}

impl<'s, 't> Printer<'s, 't> {
    /// The file: top-level items exactly one blank line apart, and each run
    /// of imports sorted, one per line (spec §6).
    fn file(&mut self) -> Doc {
        let src = self.src;
        let items = &src.file.items;
        let mut v = Vertical::new(0, Blank::One);
        let mut i = 0;
        while i < items.len() {
            if matches!(items[i].value, Item::Import(_)) {
                let start = i;
                while i < items.len() && matches!(items[i].value, Item::Import(_)) {
                    i += 1;
                }
                self.import_run(&mut v, &items[start..i]);
            } else {
                let item = &items[i];
                self.vertical_item(&mut v, item.span.start, item.span.end, None, |p| {
                    p.item(&item.value, item.span, true)
                });
                i += 1;
            }
        }
        self.vertical_finish(v, src.text.len() as u32)
    }

    /// A run of imports, sorted by path and printed one per line, with no
    /// blank line inside the run. Each import takes its comments with it,
    /// except comments that a blank line separates from the run's first
    /// import: those belong to the file, and stay above the run (spec §6;
    /// the 3.1 plan's decision 3).
    fn import_run(&mut self, v: &mut Vertical, run: &[Spanned<Item>]) {
        struct Entry {
            key: String,
            lead: Vec<Comment>,
            doc: Doc,
            trail: Doc,
        }
        let mut header = Vec::new();
        let mut entries = Vec::new();
        for (n, item) in run.iter().enumerate() {
            let mut lead = self.take_comments_before(item.span.start);
            if n == 0 {
                let split = (0..lead.len())
                    .rev()
                    .find(|&k| {
                        let next = lead.get(k + 1).map_or(item.span.start, |c| c.span.start);
                        self.src.blank_line_in(lead[k].span.end, next)
                    })
                    .map_or(0, |k| k + 1);
                header = lead.drain(..split).collect();
            }
            let doc = self.item(&item.value, item.span, true);
            let trail = self.trailing(item.span.end);
            entries.push(Entry {
                key: crate::check::import_key(&item.value),
                lead,
                doc,
                trail,
            });
        }
        let first_pos = header
            .first()
            .or(entries[0].lead.first())
            .map_or(run[0].span.start, |c| c.span.start);
        if !v.first {
            v.parts.push(Doc::HardLine);
            if v.blank == Blank::One || self.src.blank_line_in(v.prev_end, first_pos) {
                v.parts.push(Doc::HardLine);
            }
        }
        for (k, c) in header.iter().enumerate() {
            v.parts.push(self.comment_doc(c));
            v.parts.push(Doc::HardLine);
            let next = header.get(k + 1).map_or(run[0].span.start, |n| n.span.start);
            if self.src.blank_line_in(c.span.end, next) {
                v.parts.push(Doc::HardLine);
            }
        }
        entries.sort_by(|a, b| a.key.cmp(&b.key));
        for (n, entry) in entries.into_iter().enumerate() {
            if n > 0 {
                v.parts.push(Doc::HardLine);
            }
            for c in &entry.lead {
                v.parts.push(self.comment_doc(c));
                v.parts.push(Doc::HardLine);
            }
            v.parts.push(entry.doc);
            v.parts.push(entry.trail);
        }
        v.prev_end = run[run.len() - 1].span.end;
        v.first = false;
    }

    /// Add one element to `v` (spec §5.3, §5.4): the blank line before it,
    /// its leading comments, its document, `sep` where a separator is needed
    /// (§5.6), and its trailing comments.
    fn vertical_item(
        &mut self,
        v: &mut Vertical,
        start: u32,
        end: u32,
        sep: Option<&'static str>,
        print: impl FnOnce(&mut Self) -> Doc,
    ) {
        let comments = self.take_comments_before(start);
        let first_pos = comments.first().map_or(start, |c| c.span.start);
        if !v.first {
            v.parts.push(Doc::HardLine);
            if v.blank == Blank::One || self.src.blank_line_in(v.prev_end, first_pos) {
                v.parts.push(Doc::HardLine);
            }
        }
        for (k, c) in comments.iter().enumerate() {
            let next = comments.get(k + 1).map_or(start, |n| n.span.start);
            v.parts.push(self.comment_doc(c));
            if self.breaks_after(c, next) {
                v.parts.push(Doc::HardLine);
                if self.src.blank_line_in(c.span.end, next) {
                    v.parts.push(Doc::HardLine);
                }
            } else {
                v.parts.push(text(" "));
            }
        }
        v.parts.push(print(self));
        if let Some(s) = sep {
            v.parts.push(text(s));
        }
        v.parts.push(self.trailing(end));
        v.prev_end = end;
        v.first = false;
    }

    /// Close `v` before `close`: the comments left there, one per line
    /// (spec §5.4).
    fn vertical_finish(&mut self, mut v: Vertical, close: u32) -> Doc {
        for c in self.take_comments_before(close) {
            if !v.first {
                v.parts.push(Doc::HardLine);
                if self.src.blank_line_in(v.prev_end, c.span.start) {
                    v.parts.push(Doc::HardLine);
                }
            }
            v.parts.push(self.comment_doc(&c));
            v.prev_end = c.span.end;
            v.first = false;
        }
        concat(v.parts)
    }

    /// A comma list (spec §6, "Comma lists"): flat as `(a, b)`, or as
    /// `{ a, b }` when `list.spaced`; broken with one element per line, each
    /// followed by a comma, as `list.last` says for the last. `extent` gives
    /// each element's start and end in the source; `close_at` is where the
    /// closing delimiter starts. Not grouped: callers group it, or share its
    /// line breaks with more of their construct.
    fn comma_list<T>(
        &mut self,
        list: List,
        items: &[T],
        extent: impl Fn(&Self, &T) -> (u32, u32),
        mut print: impl FnMut(&mut Self, &T) -> Doc,
        close_at: u32,
    ) -> Doc {
        if items.is_empty() {
            return match self.dangling(close_at) {
                None => text(format!("{}{}", list.open, list.close)),
                Some(d) => concat(vec![
                    text(list.open),
                    nest(concat(vec![Doc::HardLine, d])),
                    Doc::HardLine,
                    text(list.close),
                ]),
            };
        }
        let edge = if list.spaced { Doc::Line } else { Doc::SoftLine };
        let mut inner = vec![edge.clone()];
        let mut prev_end = 0;
        for (i, item) in items.iter().enumerate() {
            let (start, end) = extent(&*self, item);
            if i > 0 {
                if list.blanks && self.src.blank_line_in(prev_end, start) {
                    inner.push(Doc::HardLine);
                }
                inner.push(Doc::Line);
            }
            inner.push(self.leading(start));
            inner.push(print(self, item));
            inner.push(self.trailing(end));
            inner.push(if i + 1 < items.len() {
                text(",")
            } else {
                match list.last {
                    LastComma::Broken => if_break(text(","), Doc::Nil),
                    LastComma::Always => text(","),
                    LastComma::Never => Doc::Nil,
                }
            });
            let comma_end = self.src.token_end(end, &Token::Comma);
            if let Some(at) = comma_end {
                inner.push(self.trailing(at));
            }
            prev_end = comma_end.unwrap_or(end);
        }
        if let Some(d) = self.dangling(close_at) {
            inner.push(Doc::HardLine);
            inner.push(d);
        }
        concat(vec![text(list.open), nest(concat(inner)), edge, text(list.close)])
    }

    /// A name, after any comments before it.
    fn name(&mut self, n: &Spanned<String>) -> Doc {
        concat(vec![self.leading(n.span.start), text(n.value.clone())])
    }

    /// `a::b::c`, after any comments before it.
    fn path(&mut self, path: &Path) -> Doc {
        let lead = match path.segments.first() {
            Some(first) => self.leading(first.span.start),
            None => Doc::Nil,
        };
        let joined: Vec<&str> = path.segments.iter().map(|s| s.value.as_str()).collect();
        concat(vec![lead, text(joined.join("::"))])
    }

    /// `A + B`.
    fn bounds(&mut self, bounds: &[Spanned<Path>]) -> Doc {
        let mut parts = Vec::new();
        for (i, b) in bounds.iter().enumerate() {
            if i > 0 {
                parts.push(text(" + "));
            }
            parts.push(self.path(&b.value));
        }
        concat(parts)
    }

    /// Doc comments: each a `///` line, then a line break (spec §4).
    fn docs(&self, docs: &[Spanned<String>]) -> Doc {
        let mut parts = Vec::new();
        for d in docs {
            parts.push(text(format!("///{}", d.value)));
            parts.push(Doc::HardLine);
        }
        concat(parts)
    }

    /// Source text printed exactly as written: literals, strings, an impl's
    /// trait (spec §5.3). Comments inside it belong to it. Callers take the
    /// comments before it first.
    fn verbatim(&mut self, span: Span) -> Doc {
        self.skip_comments_within(span);
        Doc::Verbatim(self.src.slice(span).to_owned())
    }

    /// A comment's text, without trailing whitespace on any of its lines.
    fn comment_doc(&self, c: &Comment) -> Doc {
        let lines: Vec<&str> = self.src.slice(c.span).lines().map(str::trim_end).collect();
        let t = lines.join("\n");
        if t.contains('\n') {
            Doc::Verbatim(t)
        } else {
            Doc::Text(t)
        }
    }

    /// Whether a line break follows comment `c` in the source before `next`.
    fn breaks_after(&self, c: &Comment, next: u32) -> bool {
        let next = next.max(c.span.end);
        c.kind == CommentKind::Line
            || self.src.text[c.span.end as usize..next as usize].contains('\n')
    }

    // --- Comments (spec §5.4). Until Task 6 these place no comments, so an
    // input with a comment fails the self-check. ---

    /// Take every comment not yet printed that starts before `offset`.
    fn take_comments_before(&mut self, _offset: u32) -> Vec<Comment> {
        Vec::new()
    }

    /// Whether a comment not yet printed starts before `offset`.
    fn has_comment_before(&self, _offset: u32) -> bool {
        false
    }

    /// Drop the comments inside verbatim text, which prints them itself.
    fn skip_comments_within(&mut self, _span: Span) {}

    /// The comments before `offset`, for a place inside a line.
    fn leading(&mut self, _offset: u32) -> Doc {
        Doc::Nil
    }

    /// The comments that end `offset`'s line.
    fn trailing(&mut self, _offset: u32) -> Doc {
        Doc::Nil
    }

    /// The comments before `close`, each on its own line.
    fn dangling(&mut self, _close: u32) -> Option<Doc> {
        None
    }
}
```

- [ ] **Step 5: Items**

Create `crates/nova-fmt/src/print/item.rs`:

```rust
//! Items (spec
//! `docs/superpowers/specs/2026-10-07-phase-3-1-formatter-design.md` §6,
//! "Items").

use nova_ast::item::{
    AssocTypeBinding, Attribute, ConstDecl, ExternBlock, ExternItem, Function, FunctionSig,
    ImplBlock, Import, ImportKind, Param, Record, RecordField, TraitDecl, TraitItem, TypeDecl,
    TypeDef, Variant, Visibility, WhereBound,
};
use nova_ast::ty::{Type, TypeParam};
use nova_ast::Item;
use nova_diagnostics::{Span, Spanned};
use nova_lexer::Token;

use super::{Blank, List, Printer, Vertical};
use crate::doc::{concat, group, if_break, nest, text, Doc};

/// The tokens a function's keyword may start with.
const FN_KEYWORDS: [Token; 3] = [Token::Fn, Token::Async, Token::Pub];

/// `pub ` or nothing.
fn vis(v: Visibility) -> &'static str {
    if v == Visibility::Pub {
        "pub "
    } else {
        ""
    }
}

/// A member of an `impl`, `trait` or `extern` body.
#[derive(Clone, Copy)]
enum Member<'a> {
    Fn(&'a Function),
    Const(&'a ConstDecl),
    Binding(&'a AssocTypeBinding),
    Trait(&'a TraitItem),
    Extern(&'a ExternItem),
}

impl<'s, 't> Printer<'s, 't> {
    /// An item (spec §6). `top` marks a top-level item: only a top-level
    /// import's `{…}` list is sorted.
    pub(super) fn item(&mut self, item: &Item, span: Span, top: bool) -> Doc {
        match item {
            Item::Function(f) => self.function(f),
            Item::Record(r) => self.record(r, span),
            Item::Type(t) => self.type_decl(t),
            Item::Trait(t) => self.trait_decl(t, span),
            Item::Impl(i) => self.impl_block(i, span),
            Item::Const(c) => self.const_decl(c),
            Item::Import(i) => self.import(i, top),
            Item::Module(m) => {
                let keyword = self.src.start_with(m.path.span.start, &[Token::Module]);
                let head = self.item_head(&m.docs, &m.attrs, keyword);
                concat(vec![head, text("module "), self.path(&m.path.value)])
            }
            Item::Extern(e) => self.extern_block(e, span),
        }
    }

    /// Docs, then attributes one per line (spec §6, "Order"), then the
    /// comments between them and the keyword at `keyword`, which move after
    /// them (the 3.1 plan's decision 9).
    fn item_head(&mut self, docs: &[Spanned<String>], attrs: &[Attribute], keyword: u32) -> Doc {
        concat(vec![self.docs(docs), self.attrs(attrs), self.leading(keyword)])
    }

    /// Attributes, one per line.
    fn attrs(&self, attrs: &[Attribute]) -> Doc {
        let mut parts = Vec::new();
        for a in attrs {
            let args = if a.args.is_empty() {
                String::new()
            } else {
                let names: Vec<&str> = a.args.iter().map(|x| x.value.as_str()).collect();
                format!("({})", names.join(", "))
            };
            parts.push(text(format!("@{}{args}", a.name.value)));
            parts.push(Doc::HardLine);
        }
        concat(parts)
    }

    /// A function. Its body shares a group with its signature, so a
    /// one-statement body stays on one line only when the whole function
    /// fits (spec §6, "Blocks"; the 3.1 plan's decision 6).
    fn function(&mut self, f: &Function) -> Doc {
        let keyword = self.src.start_with(f.name.span.start, &FN_KEYWORDS);
        let head = self.item_head(&f.docs, &f.attrs, keyword);
        let words = format!("{}{}fn ", vis(f.vis), if f.is_async { "async " } else { "" });
        let sig = self.signature(
            &f.name,
            &f.generics,
            &f.params,
            f.return_ty.as_ref(),
            &f.where_clause,
            true,
        );
        let gap = if f.where_clause.is_empty() { text(" ") } else { Doc::Nil };
        let body = self.block_parts(&f.body.value, f.body.span);
        concat(vec![head, group(concat(vec![text(words), sig, gap, body]))])
    }

    /// `name<generics>(params) -> ret where …`. The parameters break first,
    /// then the `where` clause (spec §6, "Signatures"). With `body`, a broken
    /// clause ends with a trailing comma and a line break before the `{`.
    fn signature(
        &mut self,
        name: &Spanned<String>,
        generics: &[TypeParam],
        params: &[Param],
        ret: Option<&Spanned<Type>>,
        where_clause: &[WhereBound],
        body: bool,
    ) -> Doc {
        let mut parts = vec![self.name(name), self.generics(generics)];
        let close = self
            .src
            .parens_from(name.span.end)
            .map_or(name.span.end, |(_, close)| close);
        parts.push(self.comma_list(
            List::PARENS,
            params,
            |p, x| p.param_extent(x),
            |p, x| p.param(x),
            close,
        ));
        if let Some(r) = ret {
            parts.push(text(" -> "));
            parts.push(self.ty(r));
        }
        if !where_clause.is_empty() {
            parts.push(self.where_clause(where_clause, body));
        }
        group(concat(parts))
    }

    /// `<T: A + B, U>`, or nothing.
    fn generics(&mut self, generics: &[TypeParam]) -> Doc {
        let Some(last) = generics.last() else {
            return Doc::Nil;
        };
        let last_end = self.type_param_extent(last).1;
        let close = self.src.find_from(last_end, &Token::Gt).unwrap_or(last_end);
        group(self.comma_list(
            List::ANGLES,
            generics,
            |p, g| p.type_param_extent(g),
            |p, g| p.type_param(g),
            close,
        ))
    }

    fn type_param(&mut self, g: &TypeParam) -> Doc {
        if g.bounds.is_empty() {
            self.name(&g.name)
        } else {
            concat(vec![self.name(&g.name), text(": "), self.bounds(&g.bounds)])
        }
    }

    /// A bound's span is its first token's (`grammar.rs:482-484`), so a
    /// list of bounds ends where its last path's last segment ends.
    fn type_param_extent(&self, g: &TypeParam) -> (u32, u32) {
        let end = g
            .bounds
            .last()
            .and_then(|b| b.value.segments.last())
            .map_or(g.name.span.end, |s| s.span.end);
        (g.name.span.start, end)
    }

    /// A parameter. `self`, and a closure parameter written without a type,
    /// print bare.
    pub(super) fn param(&mut self, p: &Param) -> Doc {
        let mut parts = vec![text(if p.is_mut { "mut " } else { "" }), self.name(&p.name)];
        if !bare(p) {
            parts.push(text(": "));
            parts.push(self.ty(&p.ty));
        }
        concat(parts)
    }

    fn param_extent(&self, p: &Param) -> (u32, u32) {
        let start = if p.is_mut {
            self.src.start_with(p.name.span.start, &[Token::Mut])
        } else {
            p.name.span.start
        };
        let end = if bare(p) { p.name.span.end } else { self.ty_end(&p.ty) };
        (start, end)
    }

    /// ` where T: A, U: B`, in a group of its own (spec §6, "Signatures").
    fn where_clause(&mut self, bounds: &[WhereBound], body: bool) -> Doc {
        let mut inner = Vec::new();
        for (i, b) in bounds.iter().enumerate() {
            let (start, end) = self.where_bound_extent(b);
            inner.push(Doc::Line);
            inner.push(self.leading(start));
            inner.push(self.ty(&b.ty));
            inner.push(text(": "));
            inner.push(self.bounds(&b.bounds));
            inner.push(self.trailing(end));
            inner.push(if i + 1 < bounds.len() {
                text(",")
            } else if body {
                // The parser takes a trailing comma only before the body's
                // `{` (the 3.1 plan's decision 4).
                if_break(text(","), Doc::Nil)
            } else {
                Doc::Nil
            });
            if let Some(at) = self.src.token_end(end, &Token::Comma) {
                inner.push(self.trailing(at));
            }
        }
        let mut parts = vec![Doc::Line, text("where"), nest(concat(inner))];
        if body {
            parts.push(Doc::Line);
        }
        group(concat(parts))
    }

    fn where_bound_extent(&self, b: &WhereBound) -> (u32, u32) {
        let end = b
            .bounds
            .last()
            .and_then(|p| p.value.segments.last())
            .map_or(b.ty.span.end, |s| s.span.end);
        (b.ty.span.start, end)
    }

    fn record(&mut self, r: &Record, span: Span) -> Doc {
        let keyword = self.src.start_with(r.name.span.start, &[Token::Record, Token::Pub]);
        let head = concat(vec![
            self.item_head(&r.docs, &r.attrs, keyword),
            text(format!("{}record ", vis(r.vis))),
            self.name(&r.name),
            self.generics(&r.generics),
            text(" "),
        ]);
        let fields = self.comma_list(
            List::FIELDS,
            &r.fields,
            |p, f| p.field_extent(f),
            |p, f| p.record_field(f),
            span.end - 1,
        );
        concat(vec![head, group(fields)])
    }

    fn record_field(&mut self, f: &RecordField) -> Doc {
        let start = self.src.start_with(f.name.span.start, &[Token::Pub]);
        concat(vec![
            self.item_head(&f.docs, &[], start),
            text(vis(f.vis)),
            self.name(&f.name),
            text(": "),
            self.ty(&f.ty),
        ])
    }

    fn field_extent(&self, f: &RecordField) -> (u32, u32) {
        let start = match f.docs.first() {
            Some(d) => d.span.start,
            None => self.src.start_with(f.name.span.start, &[Token::Pub]),
        };
        (start, self.ty_end(&f.ty))
    }

    fn type_decl(&mut self, t: &TypeDecl) -> Doc {
        let keyword = self.src.start_with(t.name.span.start, &[Token::Type, Token::Pub]);
        let head = concat(vec![
            self.item_head(&t.docs, &t.attrs, keyword),
            text(format!("{}type ", vis(t.vis))),
            self.name(&t.name),
            self.generics(&t.generics),
            text(" ="),
        ]);
        let variants = match &t.def {
            TypeDef::Alias(ty) => return concat(vec![head, text(" "), self.ty(ty)]),
            TypeDef::Sum(variants) => variants,
        };
        let mut inner = Vec::new();
        let mut prev_end: Option<u32> = None;
        for v in variants {
            let (start, end) = self.variant_extent(v);
            // The author's blank lines between variants are kept, and one
            // breaks the list (spec §5.3).
            if prev_end.is_some_and(|p| self.src.blank_line_in(p, start)) {
                inner.push(Doc::HardLine);
            }
            inner.push(Doc::Line);
            inner.push(self.leading(start));
            let pipe = self.src.start_with(v.name.span.start, &[Token::Pipe]);
            inner.push(self.item_head(&v.docs, &[], pipe));
            inner.push(text("| "));
            inner.push(self.name(&v.name));
            if !v.fields.is_empty() {
                inner.push(group(self.comma_list(
                    List::PARENS,
                    &v.fields,
                    |p, x| (x.span.start, p.ty_end(x)),
                    |p, x| p.ty(x),
                    end - 1,
                )));
            }
            inner.push(self.trailing(end));
            prev_end = Some(end);
        }
        concat(vec![head, group(nest(concat(inner)))])
    }

    /// A variant runs from its docs or its `|` to its name, or to the `)`
    /// after its fields.
    fn variant_extent(&self, v: &Variant) -> (u32, u32) {
        let start = match v.docs.first() {
            Some(d) => d.span.start,
            None => self.src.start_with(v.name.span.start, &[Token::Pipe]),
        };
        let end = match v.fields.last() {
            None => v.name.span.end,
            Some(last) => {
                let after = self.ty_end(last);
                self.src
                    .find_from(after, &Token::RParen)
                    .map_or(after, |at| at + 1)
            }
        };
        (start, end)
    }

    fn trait_decl(&mut self, t: &TraitDecl, span: Span) -> Doc {
        let keyword = self.src.start_with(t.name.span.start, &[Token::Trait, Token::Pub]);
        let mut head = vec![
            self.item_head(&t.docs, &t.attrs, keyword),
            text(format!("{}trait ", vis(t.vis))),
            self.name(&t.name),
            self.generics(&t.generics),
        ];
        if !t.supertraits.is_empty() {
            head.push(text(": "));
            head.push(self.bounds(&t.supertraits));
        }
        head.push(if t.where_clause.is_empty() {
            text(" ")
        } else {
            self.where_clause(&t.where_clause, true)
        });
        let members: Vec<Member> = t.items.iter().map(Member::Trait).collect();
        let body = self.members(&members, span);
        concat(vec![concat(head), body])
    }

    fn impl_block(&mut self, i: &ImplBlock, span: Span) -> Doc {
        let keyword = self.src.find_from(span.start, &Token::Impl).unwrap_or(span.start);
        let mut head = vec![
            self.item_head(&i.docs, &i.attrs, keyword),
            text("impl"),
            self.generics(&i.generics),
            text(" "),
        ];
        if let Some(tr) = &i.trait_ {
            // As written: the parser keeps the trait's path but drops its
            // type arguments (`grammar.rs:2511-2515`), so `impl Into<String>
            // for X` printed from the AST would lose `<String>`, and the
            // self-check could not notice.
            head.push(self.leading(tr.span.start));
            head.push(self.verbatim(tr.span));
            head.push(text(" for "));
        }
        head.push(self.ty(&i.ty));
        head.push(if i.where_clause.is_empty() {
            text(" ")
        } else {
            self.where_clause(&i.where_clause, true)
        });
        let mut members: Vec<Member> = i
            .functions
            .iter()
            .map(Member::Fn)
            .chain(i.consts.iter().map(Member::Const))
            .chain(i.assoc_types.iter().map(Member::Binding))
            .collect();
        // The AST keeps the three kinds in separate lists: print them in
        // source order (spec §5.3).
        members.sort_by_key(|m| self.member_extent(*m).0);
        let body = self.members(&members, span);
        concat(vec![concat(head), body])
    }

    fn const_decl(&mut self, c: &ConstDecl) -> Doc {
        let keyword = self.src.start_with(c.name.span.start, &[Token::Const, Token::Pub]);
        concat(vec![
            self.item_head(&c.docs, &c.attrs, keyword),
            text(format!("{}const ", vis(c.vis))),
            self.name(&c.name),
            text(": "),
            self.ty(&c.ty),
            text(" = "),
            self.expr(&c.value),
        ])
    }

    /// An import. With `sort`, a `{…}` list is sorted by name (spec §6).
    fn import(&mut self, i: &Import, sort: bool) -> Doc {
        let keyword = self.src.start_with(i.path.span.start, &[Token::Import]);
        let mut parts = vec![
            self.item_head(&i.docs, &i.attrs, keyword),
            text("import "),
            self.path(&i.path.value),
        ];
        match &i.kind {
            ImportKind::Simple => {}
            ImportKind::Alias(a) => {
                parts.push(text(" as "));
                parts.push(self.name(a));
            }
            ImportKind::List(names) => {
                let mut names: Vec<&Spanned<String>> = names.iter().collect();
                if sort {
                    names.sort_by(|a, b| a.value.cmp(&b.value));
                }
                let after = names
                    .iter()
                    .map(|n| n.span.end)
                    .max()
                    .unwrap_or(i.path.span.end);
                let close = self.src.find_from(after, &Token::RBrace).unwrap_or(after);
                parts.push(text("::"));
                parts.push(group(self.comma_list(
                    List::IMPORTS,
                    &names,
                    |_, n| (n.span.start, n.span.end),
                    |p, n| p.name(n),
                    close,
                )));
            }
        }
        concat(parts)
    }

    fn extern_block(&mut self, e: &ExternBlock, span: Span) -> Doc {
        let keyword = self.src.find_from(span.start, &Token::Extern).unwrap_or(span.start);
        let head = self.item_head(&e.docs, &e.attrs, keyword);
        let abi = e.abi.as_ref().map_or(String::new(), |a| format!("\"{a}\" "));
        let members: Vec<Member> = e.items.iter().map(Member::Extern).collect();
        concat(vec![head, text(format!("extern {abi}")), self.members(&members, span)])
    }

    /// A `{ … }` body of members, one per line, with the author's blank
    /// lines (spec §6): `{}` when it holds nothing.
    fn members(&mut self, members: &[Member], span: Span) -> Doc {
        let open = self.src.find_from(span.start, &Token::LBrace).unwrap_or(span.start);
        let lead = self.leading(open);
        let close = span.end - 1;
        if members.is_empty() && !self.has_comment_before(close) {
            return concat(vec![lead, text("{}")]);
        }
        let after_open = self.trailing(open + 1);
        let mut v = Vertical::new(open + 1, Blank::Keep);
        for m in members {
            let (start, end) = self.member_extent(*m);
            let end = self.src.token_end(end, &Token::Semicolon).unwrap_or(end);
            self.vertical_item(&mut v, start, end, None, |p| p.member(*m));
        }
        let inner = self.vertical_finish(v, close);
        concat(vec![
            lead,
            text("{"),
            after_open,
            nest(concat(vec![Doc::HardLine, inner])),
            Doc::HardLine,
            text("}"),
        ])
    }

    fn member(&mut self, m: Member) -> Doc {
        match m {
            Member::Fn(f) | Member::Trait(TraitItem::Provided(f)) => self.function(f),
            Member::Const(c) => self.const_decl(c),
            Member::Binding(b) => {
                let keyword = self.src.start_with(b.name.span.start, &[Token::Type]);
                concat(vec![
                    self.item_head(&b.docs, &[], keyword),
                    text("type "),
                    self.name(&b.name),
                    text(" = "),
                    self.ty(&b.ty),
                ])
            }
            Member::Trait(TraitItem::Required(sig)) | Member::Extern(ExternItem::Fn(sig)) => {
                self.function_sig(sig)
            }
            Member::Trait(TraitItem::AssocType { docs, name, bounds }) => {
                let keyword = self.src.start_with(name.span.start, &[Token::Type]);
                let mut parts = vec![
                    self.item_head(docs, &[], keyword),
                    text("type "),
                    self.name(name),
                ];
                if !bounds.is_empty() {
                    parts.push(text(": "));
                    parts.push(self.bounds(bounds));
                }
                concat(parts)
            }
        }
    }

    /// A signature with no body: a required trait method or an extern
    /// function.
    fn function_sig(&mut self, s: &FunctionSig) -> Doc {
        let keyword = self.src.start_with(s.name.span.start, &FN_KEYWORDS);
        let words = if s.is_async { "async fn " } else { "fn " };
        concat(vec![
            self.item_head(&s.docs, &[], keyword),
            text(words),
            self.signature(
                &s.name,
                &s.generics,
                &s.params,
                s.return_ty.as_ref(),
                &s.where_clause,
                false,
            ),
        ])
    }

    /// Where a member starts, from its docs or its keywords, and ends.
    fn member_extent(&self, m: Member) -> (u32, u32) {
        match m {
            Member::Fn(f) | Member::Trait(TraitItem::Provided(f)) => (
                self.member_start(&f.docs, f.name.span.start, &FN_KEYWORDS),
                f.body.span.end,
            ),
            Member::Const(c) => (
                self.member_start(&c.docs, c.name.span.start, &[Token::Const, Token::Pub]),
                c.value.span.end,
            ),
            Member::Binding(b) => (
                self.member_start(&b.docs, b.name.span.start, &[Token::Type]),
                self.ty_end(&b.ty),
            ),
            Member::Trait(TraitItem::Required(s)) | Member::Extern(ExternItem::Fn(s)) => (
                self.member_start(&s.docs, s.name.span.start, &FN_KEYWORDS),
                self.sig_end(s),
            ),
            Member::Trait(TraitItem::AssocType { docs, name, bounds }) => {
                let end = bounds
                    .last()
                    .and_then(|b| b.value.segments.last())
                    .map_or(name.span.end, |s| s.span.end);
                (self.member_start(docs, name.span.start, &[Token::Type]), end)
            }
        }
    }

    fn member_start(&self, docs: &[Spanned<String>], name: u32, keywords: &[Token]) -> u32 {
        match docs.first() {
            Some(d) => d.span.start,
            None => self.src.start_with(name, keywords),
        }
    }

    /// Where a bodyless signature ends: its last bound, its return type, or
    /// its `)`.
    fn sig_end(&self, s: &FunctionSig) -> u32 {
        if let Some(last) = s.where_clause.last() {
            return self.where_bound_extent(last).1;
        }
        if let Some(r) = &s.return_ty {
            return self.ty_end(r);
        }
        self.src
            .parens_from(s.name.span.end)
            .map_or(s.name.span.end, |(_, close)| close + 1)
    }
}

/// Whether `p` was written without a type: `self`, or a closure parameter.
/// The parser gives it an inferred type spanning its name
/// (`grammar.rs:441-447`, `:2031-2034`).
fn bare(p: &Param) -> bool {
    matches!(p.ty.value, Type::Infer) && p.ty.span == p.name.span
}
```

- [ ] **Step 6: Blocks, statements and expressions**

Create `crates/nova-fmt/src/print/expr.rs`:

```rust
//! Blocks, statements and expressions (spec
//! `docs/superpowers/specs/2026-10-07-phase-3-1-formatter-design.md` §5.6,
//! §6).

use nova_ast::expr::{AssignOp, BinOp, Expr, FieldInit, MatchArm, UnOp};
use nova_ast::item::Param;
use nova_ast::ty::Type;
use nova_ast::{Block, Path, Stmt};
use nova_diagnostics::{Span, Spanned};
use nova_lexer::Token;

use super::{Blank, LastComma, List, Printer, Vertical, CONTINUES};
use crate::doc::{concat, group, nest, text, Doc};

/// A statement in a block, or the block's trailing expression.
#[derive(Clone, Copy)]
enum Elem<'a> {
    Stmt(&'a Spanned<Stmt>),
    Tail(&'a Spanned<Expr>),
}

impl Elem<'_> {
    fn span(&self) -> Span {
        match self {
            Elem::Stmt(s) => s.span,
            Elem::Tail(e) => e.span,
        }
    }
}

/// One step of a method chain (spec §6, "Method chains").
enum Seg<'a> {
    /// `.name(args)`, with the call's span, which ends at its `)`.
    Call(&'a Spanned<String>, &'a [Spanned<Expr>], Span),
    Field(&'a Spanned<String>),
    Try,
    Await,
    Index(&'a Spanned<Expr>),
}

impl<'s, 't> Printer<'s, 't> {
    /// An expression, after any comments before it, inside one pair of the
    /// author's parentheses if it had any (spec §5.3). A tuple's own
    /// parentheses are its syntax, so it loses any extra pair.
    pub(super) fn expr(&mut self, e: &Spanned<Expr>) -> Doc {
        let lead = self.leading(e.span.start);
        if matches!(e.value, Expr::Tuple(_)) {
            let own = self.src.unparen(e.span, true);
            return concat(vec![lead, self.expr_kind(e, own)]);
        }
        let inner = self.src.unparen(e.span, false);
        if inner == e.span {
            return concat(vec![lead, self.expr_kind(e, inner)]);
        }
        let inner_lead = self.leading(inner.start);
        let body = self.expr_kind(e, inner);
        concat(vec![lead, text("("), inner_lead, body, text(")")])
    }

    /// `e` without its parentheses. `span` is `e`'s span inside them.
    fn expr_kind(&mut self, e: &Spanned<Expr>, span: Span) -> Doc {
        match &e.value {
            Expr::Lit(_) | Expr::StringInterp(_) => self.verbatim(span),
            Expr::Path(path) => self.path(path),
            Expr::Tuple(items) => {
                let last = if items.len() == 1 { LastComma::Always } else { LastComma::Broken };
                group(self.comma_list(
                    List::PARENS.last(last),
                    items,
                    |_, x| (x.span.start, x.span.end),
                    |p, x| p.expr(x),
                    span.end - 1,
                ))
            }
            Expr::Array(items) => group(self.comma_list(
                List::BRACKETS,
                items,
                |_, x| (x.span.start, x.span.end),
                |p, x| p.expr(x),
                span.end - 1,
            )),
            Expr::ArrayRepeat { init, len } => concat(vec![
                text("["),
                self.expr(init),
                text("; "),
                self.expr(len),
                text("]"),
            ]),
            Expr::Block(b) => group(self.block_parts(b, span)),
            Expr::If { .. } => self.if_chain(e),
            Expr::Match { scrutinee, arms } => self.match_expr(scrutinee, arms, span),
            Expr::While { cond, body } => group(concat(vec![
                text("while "),
                self.expr(cond),
                text(" "),
                self.block_parts(&body.value, body.span),
            ])),
            Expr::For { pattern, iter, body } => group(concat(vec![
                text("for "),
                self.pattern(pattern),
                text(" in "),
                self.expr(iter),
                text(" "),
                self.block_parts(&body.value, body.span),
            ])),
            Expr::Range { lo, hi, inclusive } => concat(vec![
                self.expr(lo),
                text(if *inclusive { "..=" } else { ".." }),
                self.expr(hi),
            ]),
            Expr::Return(value) => self.keyword_value("return", value.as_deref()),
            Expr::Break(value) => self.keyword_value("break", value.as_deref()),
            Expr::Continue => text("continue"),
            Expr::Closure { params, ret, body } => self.closure(params, ret.as_ref(), body),
            Expr::Record { path, fields, base } => {
                self.record_literal(path, fields, base.as_deref(), span)
            }
            Expr::Binary { .. } => self.binary(e),
            // `&` takes a postfix expression (`grammar.rs:1556-1560`), so a
            // reference to a reference is written `&(&x)`, and its
            // parentheses are kept: no `&&` can form here.
            Expr::Unary { op, expr } => concat(vec![text(un_op(*op)), self.expr(expr)]),
            Expr::Call { .. }
            | Expr::Field { .. }
            | Expr::Try(_)
            | Expr::Await(_)
            | Expr::Index { .. } => match self.method_chain(e, span) {
                Some(chain) => chain,
                None => self.postfix(e, span),
            },
            Expr::Cast { expr, ty } => concat(vec![self.expr(expr), text(" as "), self.ty(ty)]),
            Expr::Assign { op, lhs, rhs } => concat(vec![
                self.expr(lhs),
                text(format!(" {} ", assign_op(*op))),
                self.expr(rhs),
            ]),
        }
    }

    /// A postfix expression that is not a method chain. `span` is `e`'s, in
    /// its parentheses.
    fn postfix(&mut self, e: &Spanned<Expr>, span: Span) -> Doc {
        match &e.value {
            Expr::Call { callee, args } => concat(vec![self.expr(callee), self.args(args, span)]),
            Expr::Field { target, field } => {
                concat(vec![self.expr(target), text("."), self.name(field)])
            }
            Expr::Try(x) => concat(vec![self.expr(x), text("?")]),
            Expr::Await(x) => concat(vec![self.expr(x), text(".await")]),
            Expr::Index { target, index } => concat(vec![
                self.expr(target),
                text("["),
                self.expr(index),
                text("]"),
            ]),
            _ => unreachable!("postfix is only called on a postfix expression"),
        }
    }

    /// A call's `(args)`. `call` is the call's span, which ends at its `)`.
    fn args(&mut self, args: &[Spanned<Expr>], call: Span) -> Doc {
        group(self.comma_list(
            List::PARENS,
            args,
            |_, x| (x.span.start, x.span.end),
            |p, x| p.expr(x),
            call.end - 1,
        ))
    }

    /// A chain of two or more method calls: flat when it fits, otherwise
    /// each `.name(…)` on its own line, indented once (spec §6, "Method
    /// chains"; the 3.1 plan's decision 7). `None` for anything else.
    fn method_chain(&mut self, e: &Spanned<Expr>, span: Span) -> Option<Doc> {
        let mut segs = Vec::new();
        let mut cur = e;
        loop {
            let top = std::ptr::eq(cur, e);
            if !top && self.src.is_parenthesized(cur.span) {
                break;
            }
            match &cur.value {
                Expr::Call { callee, args } => match &callee.value {
                    Expr::Field { target, field } if !self.src.is_parenthesized(callee.span) => {
                        segs.push(Seg::Call(field, args, if top { span } else { cur.span }));
                        cur = target.as_ref();
                    }
                    _ => break,
                },
                Expr::Field { target, field } => {
                    segs.push(Seg::Field(field));
                    cur = target.as_ref();
                }
                Expr::Try(x) => {
                    segs.push(Seg::Try);
                    cur = x.as_ref();
                }
                Expr::Await(x) => {
                    segs.push(Seg::Await);
                    cur = x.as_ref();
                }
                Expr::Index { target, index } => {
                    segs.push(Seg::Index(index));
                    cur = target.as_ref();
                }
                _ => break,
            }
        }
        if segs.iter().filter(|s| matches!(s, Seg::Call(..))).count() < 2 {
            return None;
        }
        segs.reverse();
        let mut head = vec![self.expr(cur)];
        let mut tail = Vec::new();
        let mut called = false;
        for s in &segs {
            if matches!(s, Seg::Call(..)) {
                tail.push(Doc::SoftLine);
                called = true;
            }
            let d = self.seg(s);
            if called {
                tail.push(d);
            } else {
                head.push(d);
            }
        }
        Some(group(concat(vec![concat(head), nest(concat(tail))])))
    }

    fn seg(&mut self, s: &Seg) -> Doc {
        match s {
            Seg::Call(name, args, span) => {
                concat(vec![text("."), self.name(name), self.args(args, *span)])
            }
            Seg::Field(name) => concat(vec![text("."), self.name(name)]),
            Seg::Try => text("?"),
            Seg::Await => text(".await"),
            Seg::Index(i) => concat(vec![text("["), self.expr(i), text("]")]),
        }
    }

    /// A block's `{ … }`, after any comments before its `{`, without a group
    /// of its own, so that the construct holding it can share one (spec §6,
    /// "Blocks"). One statement and no comment may stay on one line;
    /// anything else is one statement per line.
    pub(super) fn block_parts(&mut self, b: &Block, span: Span) -> Doc {
        let lead = self.leading(span.start);
        let close = span.end - 1;
        let mut elems: Vec<Elem> = b.stmts.iter().map(Elem::Stmt).collect();
        if let Some(t) = &b.trailing {
            elems.push(Elem::Tail(t));
        }
        let commented = self.has_comment_before(close);
        if elems.is_empty() && !commented {
            return concat(vec![lead, text("{}")]);
        }
        if elems.len() == 1 && !commented {
            let d = self.elem(elems[0]);
            return concat(vec![
                lead,
                text("{"),
                nest(concat(vec![Doc::Line, d])),
                Doc::Line,
                text("}"),
            ]);
        }
        let after_open = self.trailing(span.start + 1);
        let mut v = Vertical::new(span.start + 1, Blank::Keep);
        for (i, el) in elems.iter().enumerate() {
            let sep = elems.get(i + 1).and_then(|next| self.separator(*el, *next));
            let s = el.span();
            // A trailing expression's span stops before a `;` after it.
            let end = self.src.token_end(s.end, &Token::Semicolon).unwrap_or(s.end);
            self.vertical_item(&mut v, s.start, end, sep, |p| p.elem(*el));
        }
        let inner = self.vertical_finish(v, close);
        concat(vec![
            lead,
            text("{"),
            after_open,
            nest(concat(vec![Doc::HardLine, inner])),
            Doc::HardLine,
            text("}"),
        ])
    }

    /// `;` between two statements where the second would otherwise continue
    /// the first (spec §5.6). A `;` after an item would not parse, and no
    /// item can be continued.
    fn separator(&self, cur: Elem, next: Elem) -> Option<&'static str> {
        if let Elem::Stmt(s) = cur {
            if matches!(s.value, Stmt::Item(_)) {
                return None;
            }
        }
        // A `return` or `break` with no value would take the next statement
        // as its value.
        if matches!(
            self.src.last_code_token(cur.span().end),
            Some(Token::Return | Token::Break)
        ) {
            return Some(";");
        }
        CONTINUES
            .contains(self.src.token_from(next.span().start))
            .then_some(";")
    }

    fn elem(&mut self, el: Elem) -> Doc {
        match el {
            Elem::Tail(e) => self.expr(e),
            Elem::Stmt(s) => match &s.value {
                Stmt::Expr(e) => self.expr(e),
                Stmt::Item(item) => self.item(item, s.span, false),
                Stmt::Let {
                    is_mut,
                    pattern,
                    ty,
                    init,
                } => {
                    let mut parts = vec![
                        text(if *is_mut { "let mut " } else { "let " }),
                        self.pattern(pattern),
                    ];
                    if let Some(t) = ty {
                        parts.push(text(": "));
                        parts.push(self.ty(t));
                    }
                    if let Some(x) = init {
                        parts.push(text(" = "));
                        parts.push(self.expr(x));
                    }
                    concat(parts)
                }
            },
        }
    }

    /// `if … { … } else if … { … } else { … }`, one group for the whole
    /// chain, so its blocks are all on one line or all broken (spec §6,
    /// "Blocks").
    fn if_chain(&mut self, e: &Spanned<Expr>) -> Doc {
        let mut parts = Vec::new();
        let mut cur = e;
        loop {
            let Expr::If { cond, then, else_ } = &cur.value else {
                unreachable!("if_chain is only called on an if")
            };
            parts.push(text("if "));
            parts.push(self.expr(cond));
            parts.push(text(" "));
            parts.push(self.block_parts(&then.value, then.span));
            let Some(next) = else_.as_deref() else {
                break;
            };
            parts.push(self.trailing(then.span.end));
            parts.push(text(" else "));
            match &next.value {
                Expr::If { .. } => {
                    parts.push(self.leading(next.span.start));
                    cur = next;
                }
                Expr::Block(b) => {
                    parts.push(self.block_parts(b, next.span));
                    break;
                }
                _ => {
                    parts.push(self.expr(next));
                    break;
                }
            }
        }
        group(concat(parts))
    }

    /// `match x { … }`: one arm per line, without commas but for §5.6's.
    fn match_expr(&mut self, scrutinee: &Spanned<Expr>, arms: &[MatchArm], span: Span) -> Doc {
        let head = concat(vec![text("match "), self.expr(scrutinee), text(" ")]);
        let open = self
            .src
            .find_from(scrutinee.span.end, &Token::LBrace)
            .unwrap_or(scrutinee.span.end);
        let lead = self.leading(open);
        let close = span.end - 1;
        if arms.is_empty() && !self.has_comment_before(close) {
            return concat(vec![head, lead, text("{}")]);
        }
        let after_open = self.trailing(open + 1);
        let mut v = Vertical::new(open + 1, Blank::Keep);
        for (i, arm) in arms.iter().enumerate() {
            let start = arm.pattern.span.start;
            let end = self
                .src
                .token_end(arm.body.span.end, &Token::Comma)
                .unwrap_or(arm.body.span.end);
            let sep = arms
                .get(i + 1)
                .filter(|n| CONTINUES.contains(self.src.token_from(n.pattern.span.start)))
                .map(|_| ",");
            self.vertical_item(&mut v, start, end, sep, |p| p.arm(arm));
        }
        let inner = self.vertical_finish(v, close);
        concat(vec![
            head,
            lead,
            text("{"),
            after_open,
            nest(concat(vec![Doc::HardLine, inner])),
            Doc::HardLine,
            text("}"),
        ])
    }

    /// `pattern if guard => body`, in a group that a block body shares.
    fn arm(&mut self, arm: &MatchArm) -> Doc {
        let mut parts = vec![self.pattern(&arm.pattern)];
        if let Some(g) = &arm.guard {
            parts.push(text(" if "));
            parts.push(self.expr(g));
        }
        parts.push(text(" => "));
        parts.push(self.body(&arm.body));
        group(concat(parts))
    }

    /// A match arm's or a closure's body. A block body shares the group of
    /// the construct holding it (spec §6, "Blocks").
    fn body(&mut self, body: &Spanned<Expr>) -> Doc {
        match &body.value {
            Expr::Block(b) if !self.src.is_parenthesized(body.span) => {
                self.block_parts(b, body.span)
            }
            _ => self.expr(body),
        }
    }

    /// `|a, b| body`. Its parameters never break (the 3.1 plan's decision 8).
    fn closure(
        &mut self,
        params: &[Param],
        ret: Option<&Spanned<Type>>,
        body: &Spanned<Expr>,
    ) -> Doc {
        let mut parts = Vec::new();
        if params.is_empty() {
            // `||` is one token, which no closure starts with (spec §5.3).
            parts.push(text("| |"));
        } else {
            parts.push(text("|"));
            for (i, p) in params.iter().enumerate() {
                if i > 0 {
                    parts.push(text(", "));
                }
                parts.push(self.param(p));
            }
            parts.push(text("|"));
        }
        if let Some(r) = ret {
            parts.push(text(" -> "));
            parts.push(self.ty(r));
        }
        parts.push(text(" "));
        parts.push(self.body(body));
        group(concat(parts))
    }

    /// `Path { field: value, short, ..base }`: no comma after `..base`
    /// (spec §6).
    fn record_literal(
        &mut self,
        path: &Path,
        fields: &[FieldInit],
        base: Option<&Spanned<Expr>>,
        span: Span,
    ) -> Doc {
        enum Part<'a> {
            Field(&'a FieldInit),
            Base(&'a Spanned<Expr>),
        }
        let head = concat(vec![self.path(path), text(" ")]);
        let mut parts: Vec<Part> = fields.iter().map(Part::Field).collect();
        if let Some(b) = base {
            parts.push(Part::Base(b));
        }
        let last = if base.is_some() { LastComma::Never } else { LastComma::Broken };
        let body = self.comma_list(
            List::BRACES.last(last),
            &parts,
            |p, part| match part {
                Part::Field(f) => (
                    f.name.span.start,
                    f.value.as_ref().map_or(f.name.span.end, |v| v.span.end),
                ),
                Part::Base(b) => (p.src.start_with(b.span.start, &[Token::DotDot]), b.span.end),
            },
            |p, part| match part {
                Part::Field(f) => match &f.value {
                    None => p.name(&f.name),
                    Some(v) => concat(vec![p.name(&f.name), text(": "), p.expr(v)]),
                },
                Part::Base(b) => concat(vec![text(".."), p.expr(b)]),
            },
            span.end - 1,
        );
        concat(vec![head, group(body)])
    }

    /// A chain of one precedence level: flat when it fits, otherwise broken
    /// before each operator, indented once (spec §6, "Binary expressions").
    fn binary(&mut self, e: &Spanned<Expr>) -> Doc {
        let Expr::Binary { op, .. } = &e.value else {
            unreachable!("binary is only called on a binary expression")
        };
        let mut operands = Vec::new();
        self.flatten(e, precedence(*op), true, &mut operands);
        let first = self.expr(operands[0].1);
        let mut rest = Vec::new();
        for &(op, x) in &operands[1..] {
            let op = op.expect("every operand after the first has an operator");
            rest.push(Doc::Line);
            rest.push(text(format!("{} ", bin_op(op))));
            rest.push(self.expr(x));
        }
        group(concat(vec![first, nest(concat(rest))]))
    }

    /// `e`'s operands at precedence `level`, left to right, each with the
    /// operator before it. A parenthesised operand is one operand.
    fn flatten<'a>(
        &self,
        e: &'a Spanned<Expr>,
        level: u8,
        top: bool,
        out: &mut Vec<(Option<BinOp>, &'a Spanned<Expr>)>,
    ) {
        match &e.value {
            Expr::Binary { op, lhs, rhs }
                if precedence(*op) == level && (top || !self.src.is_parenthesized(e.span)) =>
            {
                self.flatten(lhs, level, false, out);
                out.push((Some(*op), rhs.as_ref()));
            }
            _ => out.push((None, e)),
        }
    }

    fn keyword_value(&mut self, keyword: &str, value: Option<&Spanned<Expr>>) -> Doc {
        match value {
            None => text(keyword),
            Some(v) => concat(vec![text(format!("{keyword} ")), self.expr(v)]),
        }
    }
}

/// Binding strength, weakest first, as the parser's layers have it
/// (`grammar.rs:1324-1527`).
fn precedence(op: BinOp) -> u8 {
    match op {
        BinOp::Or => 1,
        BinOp::And => 2,
        BinOp::Eq | BinOp::Ne | BinOp::Lt | BinOp::Le | BinOp::Gt | BinOp::Ge => 3,
        BinOp::BitOr => 4,
        BinOp::BitXor => 5,
        BinOp::BitAnd => 6,
        BinOp::Shl | BinOp::Shr => 7,
        BinOp::Add | BinOp::Sub => 8,
        BinOp::Mul | BinOp::Div | BinOp::Rem => 9,
    }
}

fn bin_op(op: BinOp) -> &'static str {
    match op {
        BinOp::Or => "||",
        BinOp::And => "&&",
        BinOp::Eq => "==",
        BinOp::Ne => "!=",
        BinOp::Lt => "<",
        BinOp::Le => "<=",
        BinOp::Gt => ">",
        BinOp::Ge => ">=",
        BinOp::BitOr => "|",
        BinOp::BitXor => "^",
        BinOp::BitAnd => "&",
        BinOp::Shl => "<<",
        BinOp::Shr => ">>",
        BinOp::Add => "+",
        BinOp::Sub => "-",
        BinOp::Mul => "*",
        BinOp::Div => "/",
        BinOp::Rem => "%",
    }
}

fn un_op(op: UnOp) -> &'static str {
    match op {
        UnOp::Neg => "-",
        UnOp::Not => "!",
        UnOp::BitNot => "~",
        UnOp::Ref => "&",
        UnOp::RefMut => "&mut ",
        UnOp::Deref => "*",
    }
}

fn assign_op(op: AssignOp) -> &'static str {
    match op {
        AssignOp::Assign => "=",
        AssignOp::AddAssign => "+=",
        AssignOp::SubAssign => "-=",
        AssignOp::MulAssign => "*=",
        AssignOp::DivAssign => "/=",
        AssignOp::RemAssign => "%=",
        AssignOp::BitOrAssign => "|=",
        AssignOp::BitAndAssign => "&=",
        AssignOp::BitXorAssign => "^=",
        AssignOp::ShlAssign => "<<=",
        AssignOp::ShrAssign => ">>=",
    }
}
```

- [ ] **Step 7: Patterns and types**

Create `crates/nova-fmt/src/print/pattern.rs`:

```rust
//! Patterns (spec
//! `docs/superpowers/specs/2026-10-07-phase-3-1-formatter-design.md` §6).

use nova_ast::pattern::{FieldPat, Pattern};
use nova_ast::Path;
use nova_diagnostics::{Span, Spanned};
use nova_lexer::Token;

use super::{LastComma, List, Printer};
use crate::doc::{concat, group, nest, text, Doc};

impl<'s, 't> Printer<'s, 't> {
    /// A pattern, after any comments before it, inside one pair of the
    /// author's parentheses if it had any (spec §5.3).
    pub(super) fn pattern(&mut self, p: &Spanned<Pattern>) -> Doc {
        let lead = self.leading(p.span.start);
        if matches!(p.value, Pattern::Tuple(_)) {
            let own = self.src.unparen(p.span, true);
            return concat(vec![lead, self.pattern_kind(p, own)]);
        }
        let inner = self.src.unparen(p.span, false);
        if inner == p.span {
            return concat(vec![lead, self.pattern_kind(p, inner)]);
        }
        let inner_lead = self.leading(inner.start);
        let body = self.pattern_kind(p, inner);
        concat(vec![lead, text("("), inner_lead, body, text(")")])
    }

    /// `p` without its parentheses. `span` is `p`'s span inside them.
    fn pattern_kind(&mut self, p: &Spanned<Pattern>, span: Span) -> Doc {
        let close = span.end.saturating_sub(1);
        match &p.value {
            Pattern::Wildcard => text("_"),
            Pattern::Lit(_) => self.verbatim(span),
            Pattern::Ident { is_mut, name } => {
                concat(vec![text(if *is_mut { "mut " } else { "" }), self.name(name)])
            }
            Pattern::Binding { name, inner } => {
                concat(vec![self.name(name), text(" @ "), self.pattern(inner)])
            }
            Pattern::TupleStruct { path, fields } => concat(vec![
                self.path(path),
                group(self.comma_list(
                    List::PARENS,
                    fields,
                    |_, x| (x.span.start, x.span.end),
                    |pr, x| pr.pattern(x),
                    close,
                )),
            ]),
            Pattern::Record { path, fields, rest } => {
                self.record_pattern(path, fields, *rest, close)
            }
            Pattern::Tuple(items) => {
                let last = if items.len() == 1 { LastComma::Always } else { LastComma::Broken };
                group(self.comma_list(
                    List::PARENS.last(last),
                    items,
                    |_, x| (x.span.start, x.span.end),
                    |pr, x| pr.pattern(x),
                    close,
                ))
            }
            Pattern::Array(items) => group(self.comma_list(
                List::BRACKETS,
                items,
                |_, x| (x.span.start, x.span.end),
                |pr, x| pr.pattern(x),
                close,
            )),
            Pattern::Or(alternatives) => {
                let first = self.pattern(&alternatives[0]);
                let mut rest = Vec::new();
                for a in &alternatives[1..] {
                    rest.push(Doc::Line);
                    rest.push(text("| "));
                    rest.push(self.pattern(a));
                }
                group(concat(vec![first, nest(concat(rest))]))
            }
            Pattern::Range { lo, hi, inclusive } => concat(vec![
                self.pattern(lo),
                text(if *inclusive { "..=" } else { ".." }),
                self.pattern(hi),
            ]),
            Pattern::Path(path) => self.path(path),
        }
    }

    /// `P { x, y: q, .. }`: no comma after `..`, which the parser rejects
    /// there (spec §2, §6).
    fn record_pattern(&mut self, path: &Path, fields: &[FieldPat], rest: bool, close: u32) -> Doc {
        enum Part<'a> {
            Field(&'a FieldPat),
            /// The `..`, starting here.
            Rest(u32),
        }
        let head = concat(vec![self.path(path), text(" ")]);
        let mut parts: Vec<Part> = fields.iter().map(Part::Field).collect();
        if rest {
            let after = match fields.last() {
                Some(f) => f.pattern.as_ref().map_or(f.name.span.end, |p| p.span.end),
                None => path.segments.last().map_or(0, |s| s.span.end),
            };
            parts.push(Part::Rest(self.src.find_from(after, &Token::DotDot).unwrap_or(after)));
        }
        let last = if rest { LastComma::Never } else { LastComma::Broken };
        let body = self.comma_list(
            List::BRACES.last(last),
            &parts,
            |_, part| match part {
                Part::Field(f) => (
                    f.name.span.start,
                    f.pattern.as_ref().map_or(f.name.span.end, |p| p.span.end),
                ),
                Part::Rest(at) => (*at, at + 2),
            },
            |pr, part| match part {
                Part::Field(f) => match &f.pattern {
                    None => pr.name(&f.name),
                    Some(p) => concat(vec![pr.name(&f.name), text(": "), pr.pattern(p)]),
                },
                Part::Rest(_) => text(".."),
            },
            close,
        );
        concat(vec![head, group(body)])
    }
}
```

Create `crates/nova-fmt/src/print/ty.rs`:

```rust
//! Types (spec
//! `docs/superpowers/specs/2026-10-07-phase-3-1-formatter-design.md` §6).

use nova_ast::ty::Type;
use nova_diagnostics::{Span, Spanned};
use nova_lexer::Token;

use super::{LastComma, List, Printer};
use crate::doc::{concat, group, text, Doc};

impl<'s, 't> Printer<'s, 't> {
    /// A type, after any comments before it, inside one pair of the author's
    /// parentheses if it had any (spec §5.3).
    pub(super) fn ty(&mut self, t: &Spanned<Type>) -> Doc {
        let lead = self.leading(t.span.start);
        if matches!(t.value, Type::Tuple(_)) {
            let own = self.src.unparen(t.span, true);
            return concat(vec![lead, self.ty_kind(t, own)]);
        }
        let inner = self.src.unparen(t.span, false);
        if inner == t.span {
            return concat(vec![lead, self.ty_kind(t, inner)]);
        }
        let inner_lead = self.leading(inner.start);
        let body = self.ty_kind(t, inner);
        concat(vec![lead, text("("), inner_lead, body, text(")")])
    }

    /// `t` without its parentheses. `span` is `t`'s span inside them.
    fn ty_kind(&mut self, t: &Spanned<Type>, span: Span) -> Doc {
        match &t.value {
            Type::Path { path, args } => {
                let head = self.path(path);
                let Some(last) = args.last() else {
                    return head;
                };
                // The `>` may be half of a `>>` token (`grammar.rs:1058-1078`),
                // so the list closes where its last argument ends.
                let close = self.ty_end(last);
                concat(vec![
                    head,
                    group(self.comma_list(
                        List::ANGLES,
                        args,
                        |p, x| (x.span.start, p.ty_end(x)),
                        |p, x| p.ty(x),
                        close,
                    )),
                ])
            }
            Type::Ref { is_mut, inner } => {
                // `&&` is one token, so a reference to a reference keeps its
                // space: `& &T` (spec §5.3).
                let glued = !*is_mut
                    && !self.src.is_parenthesized(inner.span)
                    && matches!(inner.value, Type::Ref { .. });
                let prefix = if *is_mut {
                    "&mut "
                } else if glued {
                    "& "
                } else {
                    "&"
                };
                concat(vec![text(prefix), self.ty(inner)])
            }
            Type::Ptr { is_mut, inner } => {
                concat(vec![text(if *is_mut { "*mut " } else { "*" }), self.ty(inner)])
            }
            Type::Array(inner) => concat(vec![text("["), self.ty(inner), text("]")]),
            Type::Tuple(items) => {
                let last = if items.len() == 1 { LastComma::Always } else { LastComma::Broken };
                group(self.comma_list(
                    List::PARENS.last(last),
                    items,
                    |p, x| (x.span.start, p.ty_end(x)),
                    |p, x| p.ty(x),
                    span.end - 1,
                ))
            }
            Type::Fn { params, ret } => {
                let close = self
                    .src
                    .parens_from(span.start)
                    .map_or(span.end, |(_, close)| close);
                let mut parts = vec![
                    text("fn"),
                    group(self.comma_list(
                        List::PARENS,
                        params,
                        |p, x| (x.span.start, p.ty_end(x)),
                        |p, x| p.ty(x),
                        close,
                    )),
                ];
                // `fn(T)` and `fn(T) -> ()` are the same type, so no unit
                // return is printed (spec §6).
                if !matches!(&ret.value, Type::Tuple(v) if v.is_empty()) {
                    parts.push(text(" -> "));
                    parts.push(self.ty(ret));
                }
                concat(parts)
            }
            Type::Optional(inner) => concat(vec![self.ty(inner), text("?")]),
            Type::Infer => text("_"),
        }
    }

    /// Where `t` ends. The parser gives a `fn(…)` type written without `->`
    /// a unit return spanning the token after it, and spans the type over
    /// that token too (spec §2), so such a type ends at its `)`. So does a
    /// reference or a pointer to one.
    pub(super) fn ty_end(&self, t: &Spanned<Type>) -> u32 {
        if self.src.is_parenthesized(t.span) {
            return t.span.end;
        }
        match &t.value {
            Type::Fn { ret, .. }
                if self.src.last_code_token(ret.span.start) != Some(&Token::Arrow) =>
            {
                self.src
                    .parens_from(t.span.start)
                    .map_or(t.span.end, |(_, close)| close + 1)
            }
            Type::Ref { inner, .. } | Type::Ptr { inner, .. } => self.ty_end(inner),
            _ => t.span.end,
        }
    }
}
```

- [ ] **Step 8: `format` and `format_named`**

In `crates/nova-fmt/src/lib.rs`, add `mod print;` after `mod doc;`, and after
the `FormatError` enum:

```rust

/// Format `source`, naming it `<stdin>` in any diagnostics (spec §5.1).
pub fn format(source: &str) -> Result<String, FormatError> {
    format_named(source, "<stdin>")
}

/// Format `source`, naming it `name` in any diagnostics (spec §5.1): the
/// canonical layout, with `\n` line endings and one final newline, checked
/// against the input before it is returned (§5.5).
pub fn format_named(source: &str, name: &str) -> Result<String, FormatError> {
    format_with(source, name, print::print)
}
```

- [ ] **Step 9: Run the tests to watch them pass**

```bash
cd /d/Projects/nona/nova && P=/c/Users/SAKEER~1/AppData/Local/Temp/gcm/p31 && cargo test --locked -p nova-fmt > $P/task5.txt 2>&1; echo "exit=$?"; grep -E "^test result|FAILED|panicked" $P/task5.txt | head -30; grep -E -B4 -- "--> crates.nova-fmt" $P/task5.txt | grep -E "^warning" | sort | uniq -c
```

Expected: `exit=0`, and four `test result: ok.` lines: the library's 20,
`layout`'s 29, `source_rules`'s 21, and the doc-tests' 0. The only warning
from `nova-fmt` says the variants `LineSuffix` and `BreakParent` are never
constructed: Task 6's comments construct them. Every helper in `source.rs`
and `print/` is used from here on. If a test fails, read its diff: the
conventions say how to tell a wrong expectation from wrong code.

- [ ] **Step 10: Commit**

Write `$P/msg-5.txt`:

```
nova-fmt: print every construct in the canonical layout

The printer walks the AST into a Wadler document, in source order (spec
docs/superpowers/specs/2026-10-07-phase-3-1-formatter-design.md §5.3,
§5.6, §6).

- It reads what the AST forgets from the source:
  - literal spellings and strings, verbatim;
  - the author's parentheses, one pair of them;
  - blank lines between statements, members, arms, fields and variants;
  - impl member order;
  - an impl's trait, whose type arguments the parser drops;
  - where a `fn(T)` type with no `->` really ends.
- Blocks follow one rule: one statement, no comment, and it fits, so it
  stays on one line. The construct holding the block shares its group.
- Comma lists are flat, or broken with a trailing comma. That covers
  parameters, arguments, fields, generics, patterns and `where`.
- Method chains and binary expressions break before `.` and before the
  operator.
- `;` and `,` are printed only where §5.6 needs them.

Comments come next; until then an input with one fails the self-check.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
```

```bash
cd /d/Projects/nona/nova && P=/c/Users/SAKEER~1/AppData/Local/Temp/gcm/p31 && cargo fmt --all && git add crates/nova-fmt && git commit -q -F $P/msg-5.txt && git log -1 --format=%s
```

Expected: `nova-fmt: print every construct in the canonical layout`.

The task's test command: `cargo test --locked -p nova-fmt`.

---
### Task 6: Comments

Spec §5.4 and the 3.1 plan's decision 5. The cursor takes each comment when the
printer reaches it, so comments come out in the order they went in. The
hooks Task 5 left as stubs become real.

**Files:**
- Create: `crates/nova-fmt/src/comments.rs`
- Modify: `crates/nova-fmt/src/lib.rs` (`mod comments;`)
- Modify: `crates/nova-fmt/src/print/mod.rs` (the `comments` field, `print`,
  and the six hooks)
- Create: `crates/nova-fmt/tests/comments.rs`

**Interfaces:**
- Consumes: Task 1's `Comment { kind, span }` and `CommentKind`; Task 4's
  `Source::comments`; Task 5's hooks on `Printer`, with the signatures listed
  in Task 5's Interfaces block.
- Produces, in `crate::comments`: `pub(crate) struct Comments<'c>`, with
  `new(all: &'c [Comment]) -> Comments<'c>`,
  `take_before(&mut self, offset: u32) -> &'c [Comment]`,
  `next_starts_before(&self, offset: u32) -> bool` and
  `take_trailing(&mut self, text: &str, offset: u32) -> &'c [Comment]`.

- [ ] **Step 1: Write the failing tests**

In `crates/nova-fmt/src/lib.rs`, add `mod comments;` after `mod check;`.

Create `crates/nova-fmt/src/comments.rs` holding only its tests for now:

```rust
//! The comment cursor (spec
//! `docs/superpowers/specs/2026-10-07-phase-3-1-formatter-design.md` §5.4).
//!
//! The printer reaches the AST's nodes in source order and takes each
//! comment as it reaches it, so no comment can come out of order (this
//! plan's decision 5).

#[cfg(test)]
mod tests {
    use super::*;
    use nova_diagnostics::FileDb;

    fn comments(text: &str) -> Vec<Comment> {
        let mut db = FileDb::new();
        let id = db.add("t.nova", text);
        nova_lexer::lex_with_comments(text, id).1
    }

    fn texts<'a>(text: &'a str, taken: &[Comment]) -> Vec<&'a str> {
        taken
            .iter()
            .map(|c| &text[c.span.start as usize..c.span.end as usize])
            .collect()
    }

    #[test]
    fn take_before_takes_each_comment_once_in_order() {
        let text = "a // one\nb /* two */ c\n// three\n";
        let all = comments(text);
        let mut cs = Comments::new(&all);
        let end = text.len() as u32;
        assert_eq!(texts(text, cs.take_before(9)), vec!["// one"]);
        assert!(cs.next_starts_before(end));
        assert_eq!(texts(text, cs.take_before(end)), vec!["/* two */", "// three"]);
        assert!(cs.take_before(end).is_empty());
        assert!(!cs.next_starts_before(end));
    }

    #[test]
    fn take_trailing_takes_only_what_ends_the_line() {
        let text = "a /* x */ // y\nb /* z */ c\nd\n// own\n";
        let all = comments(text);
        let mut cs = Comments::new(&all);
        let after = |ch: char| text.find(ch).unwrap() as u32 + 1;
        // After `a`, a block comment and a line comment end the line.
        assert_eq!(texts(text, cs.take_trailing(text, after('a'))), vec!["/* x */", "// y"]);
        // After `b`, code follows the block comment: it belongs to `c`.
        assert!(cs.take_trailing(text, after('b')).is_empty());
        assert_eq!(texts(text, cs.take_before(after('c') - 1)), vec!["/* z */"]);
        // After `d`, the next comment is on a line of its own.
        assert!(cs.take_trailing(text, after('d')).is_empty());
        assert!(cs.next_starts_before(text.len() as u32));
    }
}
```

Create `crates/nova-fmt/tests/comments.rs`:

```rust
//! Comments (spec
//! `docs/superpowers/specs/2026-10-07-phase-3-1-formatter-design.md` §5.4).

mod common;
use common::{assert_formats, assert_stable};

#[test]
fn file_level_comments_keep_their_places() {
    assert_formats(
        "// header\nfn a() {}\n// between\nfn b() {} // after b\n// end\n",
        "// header\nfn a() {}\n\n// between\nfn b() {} // after b\n// end\n",
    );
}

#[test]
fn a_file_holding_only_comments() {
    assert_stable("// one\n\n/* two */\n");
}

#[test]
fn a_comment_at_the_end_of_a_file_without_a_newline() {
    assert_formats("fn main() {} // done", "fn main() {} // done\n");
    assert_formats("fn main() {}\n// done", "fn main() {}\n// done\n");
}

#[test]
fn comments_in_a_block_keep_their_lines() {
    assert_stable(
        "fn main() {\n    // first\n    let a = 1 // one\n\n    /* block */ let b = 2\n    // last\n}\n",
    );
}

#[test]
fn a_comment_between_a_closing_brace_and_else_survives() {
    assert_formats(
        "fn f(x: Bool) {\n    if x {\n        a()\n    } // not y\n    else {\n        b()\n    }\n}\n",
        "fn f(x: Bool) {\n    if x {\n        a()\n    } else { // not y\n        b()\n    }\n}\n",
    );
}

#[test]
fn a_comment_marker_inside_a_string_is_not_a_comment_either() {
    assert_formats(
        "fn main() { let url = \"http://example.com/*x*/\" // real\n}\n",
        "fn main() {\n    let url = \"http://example.com/*x*/\" // real\n}\n",
    );
}

#[test]
fn a_trailing_comment_in_a_list_goes_after_its_comma() {
    assert_formats(
        "fn main() {\n    f(a // first\n    , b)\n}\n",
        "fn main() {\n    f(\n        a, // first\n        b,\n    )\n}\n",
    );
}

#[test]
fn own_line_and_dangling_comments_in_lists() {
    assert_stable(
        "fn main() {\n    f(\n        // why a\n        a,\n        b,\n        // nothing after b\n    )\n    g(\n        // no arguments yet\n    )\n}\n",
    );
}

#[test]
fn comments_survive_in_declaration_lists() {
    assert_stable(
        "record P {\n    x: Int, // the x\n    // the y\n    y: Int,\n}\n\n\
         type T =\n    | A // the a\n    // the b\n    | B(Int)\n\n\
         trait Tr {\n    // a method\n    fn m(self)\n}\n\n\
         import m::{\n    a, // the a\n    b,\n}\n\n\
         fn g<\n    T, // the t\n    U,\n>() {}\n",
    );
}

#[test]
fn comments_survive_in_expression_and_pattern_lists() {
    assert_stable(
        "fn f(\n    a: Int, // the a\n    b: Int,\n) -> [Int] {\n\
         \x20   let (\n        x, // the x\n        y,\n    ) = (a, b)\n\
         \x20   let q = Q {\n        x: 1, // one\n        ..p\n    }\n\
         \x20   match a {\n        // zero\n        0 => [b] // one\n        _ => [\n            a, // first\n            b,\n        ]\n    }\n}\n",
    );
}

#[test]
fn a_comment_in_a_where_clause_breaks_the_parameters_first() {
    // The parameters break before the `where` clause does (spec §6), so a
    // forced break in the clause breaks them too (the 3.1 plan's decision 6).
    assert_formats(
        "fn w<T>(x: T) where T: A, // the bound\n{}\n",
        "fn w<T>(\n    x: T,\n)\nwhere\n    T: A, // the bound\n{}\n",
    );
}

#[test]
fn an_end_of_line_comment_inside_an_expression_stays_where_it_was() {
    assert_stable("fn main() {\n    let x = a\n        + // why\n        b\n    f(x)\n}\n");
}

#[test]
fn comments_move_with_their_imports() {
    // The header is separated from the first import by a blank line, so it
    // stays at the top (the 3.1 plan's decision 3).
    assert_formats(
        "// header\n\nimport zeta // last\n// about alpha\nimport alpha\n\nfn main() {}\n",
        "// header\n\n// about alpha\nimport alpha\nimport zeta // last\n\nfn main() {}\n",
    );
}

#[test]
fn a_comment_after_an_opening_brace_stays_on_its_line() {
    assert_stable("impl Shape for Square { // squares only\n    fn area(self) -> Float { 1.0 }\n}\n");
    assert_stable("fn main() { // entry\n    run()\n}\n");
}

#[test]
fn a_comment_between_doc_lines_moves_after_them() {
    assert_formats(
        "/// One.\n// note\n/// Two.\nfn f() {}\n",
        "/// One.\n/// Two.\n// note\nfn f() {}\n",
    );
}

#[test]
fn a_comment_after_a_function_type_stays_with_it() {
    // A `fn(T)` type with no `->` is spanned over the token after it (spec
    // §2); the comment must still trail the variant it follows.
    assert_stable("type Event =\n    | Click(fn(Int)) // a handler\n    | Close(Int)\n");
    assert_stable("record Button {\n    on_click: fn(Int), // the handler\n    label: String,\n}\n");
}

#[test]
fn a_block_comment_in_mid_line_keeps_its_place() {
    // A comment in a block keeps the block on several lines (spec §6).
    assert_formats("fn main() { f(a, /* b */ c) }\n", "fn main() {\n    f(a, /* b */ c)\n}\n");
}

#[test]
fn a_multi_line_block_comment_keeps_its_inner_lines() {
    assert_formats(
        "fn main() {\n        /* one\n           two */\n        f()\n}\n",
        "fn main() {\n    /* one\n           two */\n    f()\n}\n",
    );
}
```

In `comments_survive_in_expression_and_pattern_lists`, `\x20` is a space: a
`\` at the end of a line in a Rust string drops the next line's leading
whitespace, so each such line restarts with `\x20` and three more spaces.

- [ ] **Step 2: Run them to watch them fail**

```bash
cd /d/Projects/nona/nova && cargo test --locked -p nova-fmt --test comments 2>&1 | grep -E "^test result" ; cargo test --locked -p nova-fmt --lib 2>&1 | grep -E "^error(\[E[0-9]+\])?:" | sort | uniq -c | head -5
```

Expected: `test result: FAILED. 0 passed; 18 failed`, since the stub hooks
place no comment and the self-check refuses every output; and errors that
`Comments` and `Comment` cannot be found, from the unit tests.

- [ ] **Step 3: The cursor**

In `crates/nova-fmt/src/comments.rs`, insert between the module comment and
the tests:

```rust

use nova_lexer::{Comment, CommentKind};

/// One input's comments, and how many the printer has taken.
pub(crate) struct Comments<'c> {
    all: &'c [Comment],
    next: usize,
}

impl<'c> Comments<'c> {
    pub fn new(all: &'c [Comment]) -> Comments<'c> {
        Comments { all, next: 0 }
    }

    /// Take every comment not yet taken that starts before `offset`.
    pub fn take_before(&mut self, offset: u32) -> &'c [Comment] {
        let start = self.next;
        while self.next_starts_before(offset) {
            self.next += 1;
        }
        &self.all[start..self.next]
    }

    /// Whether a comment not yet taken starts before `offset`.
    pub fn next_starts_before(&self, offset: u32) -> bool {
        self.all
            .get(self.next)
            .is_some_and(|c| c.span.start < offset)
    }

    /// Take the comments that end `offset`'s line in `text`: those after
    /// `offset` with only spaces, tabs and each other before them, up to a
    /// line comment, or a block comment the end of the line follows. A block
    /// comment with code after it on its line is left for that code (spec
    /// §5.4, "remaining").
    pub fn take_trailing(&mut self, text: &str, offset: u32) -> &'c [Comment] {
        let start = self.next;
        let mut end = start;
        let mut at = offset as usize;
        let mut k = start;
        while let Some(c) = self.all.get(k) {
            at += spaces(&text[at..]);
            if c.span.start as usize != at {
                break;
            }
            at = c.span.end as usize;
            k += 1;
            if c.kind == CommentKind::Line {
                end = k;
                break;
            }
            let rest = &text[at..];
            let after = &rest[spaces(rest)..];
            if after.is_empty() || after.starts_with('\n') {
                end = k;
            }
        }
        self.next = end;
        &self.all[start..end]
    }
}

/// How many spaces and tabs `s` starts with.
fn spaces(s: &str) -> usize {
    s.len() - s.trim_start_matches(|c| c == ' ' || c == '\t').len()
}
```

- [ ] **Step 4: The printer takes comments**

In `crates/nova-fmt/src/print/mod.rs`:

- after `use crate::doc::{concat, if_break, nest, text, Doc};` add
  `use crate::comments::Comments;`;
- in `print`, change `let mut printer = Printer { src };` to:

  ```rust
      let mut printer = Printer {
          src,
          comments: Comments::new(&src.comments),
      };
  ```
- give `Printer` its second field:

  ```rust
  pub(crate) struct Printer<'s, 't> {
      src: &'s Source<'t>,
      comments: Comments<'s>,
  }
  ```
- replace everything from the line
  `// --- Comments (spec §5.4). Until Task 6 these place no comments, so an`
  to the end of the `impl` block (the stubs) with:

  ```rust
      // --- Comments (spec §5.4; the 3.1 plan's decision 5). ---

      /// Take every comment not yet printed that starts before `offset`.
      fn take_comments_before(&mut self, offset: u32) -> Vec<Comment> {
          self.comments.take_before(offset).to_vec()
      }

      /// Whether a comment not yet printed starts before `offset`.
      fn has_comment_before(&self, offset: u32) -> bool {
          self.comments.next_starts_before(offset)
      }

      /// Drop the comments inside verbatim text, which prints them itself.
      fn skip_comments_within(&mut self, span: Span) {
          self.comments.take_before(span.end);
      }

      /// The comments before `offset`, for a place inside a line: each is
      /// followed by a line break if it is a line comment or one followed it
      /// in the source, and otherwise by a space.
      fn leading(&mut self, offset: u32) -> Doc {
          let taken = self.comments.take_before(offset);
          let mut parts = Vec::new();
          for (k, c) in taken.iter().enumerate() {
              let next = taken.get(k + 1).map_or(offset, |n| n.span.start);
              parts.push(self.comment_doc(c));
              parts.push(if self.breaks_after(c, next) {
                  Doc::HardLine
              } else {
                  text(" ")
              });
          }
          concat(parts)
      }

      /// The comments that end `offset`'s line. A line comment waits for the
      /// end of the line the printer is on, after any `,` printed there, and
      /// breaks every group around it. A block comment follows after a
      /// space.
      fn trailing(&mut self, offset: u32) -> Doc {
          let taken = self.comments.take_trailing(self.src.text, offset);
          let mut parts = Vec::new();
          for c in taken {
              match c.kind {
                  CommentKind::Line => {
                      let line = self.src.slice(c.span).trim_end();
                      parts.push(Doc::LineSuffix(format!(" {line}")));
                      parts.push(Doc::BreakParent);
                  }
                  CommentKind::Block => {
                      parts.push(text(" "));
                      parts.push(self.comment_doc(c));
                  }
              }
          }
          concat(parts)
      }

      /// The comments before `close`, each on its own line, or `None`.
      fn dangling(&mut self, close: u32) -> Option<Doc> {
          let taken = self.comments.take_before(close);
          if taken.is_empty() {
              return None;
          }
          let mut parts = Vec::new();
          for (k, c) in taken.iter().enumerate() {
              if k > 0 {
                  parts.push(Doc::HardLine);
              }
              parts.push(self.comment_doc(c));
          }
          Some(concat(parts))
      }
  }
  ```

  The closing `}` above is the `impl` block's own.

- [ ] **Step 5: Run the tests to watch them pass**

```bash
cd /d/Projects/nona/nova && P=/c/Users/SAKEER~1/AppData/Local/Temp/gcm/p31 && cargo test --locked -p nova-fmt > $P/task6.txt 2>&1; echo "exit=$?"; grep -E "^test result|FAILED|panicked" $P/task6.txt | head -30; grep -c -E -- "--> crates.nova-fmt" $P/task6.txt
```

Expected: `exit=0`, and five `test result: ok.` lines: the library's 22 (20
before, and the cursor's 2), `comments`'s 18, `layout`'s 29,
`source_rules`'s 21, and the doc-tests' 0. The last count is `0`: no
warning points into `nova-fmt`.

- [ ] **Step 6: Commit**

Write `$P/msg-6.txt`:

```
nova-fmt: keep every comment, in order

A cursor takes each comment as the printer reaches the node after it
(spec docs/superpowers/specs/2026-10-07-phase-3-1-formatter-design.md
§5.4). The printer walks in source order, so no comment can be
reordered, except inside an import run, which is sorted.

- An own-line comment leads the node after it.
- An end-of-line comment after a list element trails it, after its
  comma.
- One anywhere else stays where it was, and a line break follows it.
- A block comment in mid-line keeps its place.
- A comment on a `{`'s line stays there, which keeps formatting
  idempotent.
- A comment between a doc line and its item moves after the docs.
- A file's header comment stays above a sorted import run.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
```

```bash
cd /d/Projects/nona/nova && P=/c/Users/SAKEER~1/AppData/Local/Temp/gcm/p31 && cargo fmt --all && git add crates/nova-fmt && git commit -q -F $P/msg-6.txt && git log -1 --format=%s
```

Expected: `nova-fmt: keep every comment, in order`.

The task's test command: `cargo test --locked -p nova-fmt`.

---
### Task 7: Files, line endings and `.editorconfig`

Spec §5.1 and §7.3. `format_file` is the entry point `nova fmt` uses, and
3.2's LSP will. It formats one file in memory and writes nothing.

**Files:**
- Create: `crates/nova-fmt/src/editorconfig.rs`, `crates/nova-fmt/src/file.rs`
- Modify: `crates/nova-fmt/src/lib.rs` (`mod editorconfig;`, `mod file;`,
  and the `pub use`)
- Create: `crates/nova-fmt/tests/files.rs`

**Interfaces:**
- Consumes: Task 5's `format` and `format_named`, and Task 4's
  `FormatError`.
- Produces, re-exported at the crate root:
  - `pub fn format_file(path: &Path) -> Result<Formatted, FileError>`
  - `pub fn format_text(source: &str) -> Result<String, FormatError>`
  - `pub struct Formatted { pub original: String, pub formatted: String }`,
    with `pub fn changed(&self) -> bool`
  - `pub enum FileError { Io(std::io::Error), NotUtf8, Format(FormatError) }`,
    deriving `Debug` and `thiserror::Error`
  - `pub enum LineEnding { Lf, Crlf }`, with `pub fn of(text: &str) -> LineEnding`
- And, inside the crate: `editorconfig::settings_for(file: &Path) -> Settings`,
  with `Settings { crlf: Option<bool>, insert_final_newline: Option<bool> }`.

- [ ] **Step 1: Write the failing tests**

In `crates/nova-fmt/src/lib.rs`, add `mod editorconfig;` after
`mod doc;`.

Create `crates/nova-fmt/src/editorconfig.rs` holding only its tests for now:

```rust
//! The two `.editorconfig` settings `nova fmt` reads (spec
//! `docs/superpowers/specs/2026-10-07-phase-3-1-formatter-design.md` §7.3):
//! `end_of_line` and `insert_final_newline`. Every other key is ignored.

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn braces_expand_and_numeric_ranges_never_match() {
        assert_eq!(expand("*.{nova,toml}"), vec!["*.nova", "*.toml"]);
        assert_eq!(expand("{a,{b,c}}"), vec!["a", "b", "c"]);
        assert_eq!(expand("{word}.nova"), vec!["{word}.nova"]);
        assert!(expand("{1..3}.nova").is_empty());
    }

    #[test]
    fn globs_match_as_editorconfig_says() {
        for (glob, path, want) in [
            ("*", "src/main.nova", true),
            ("*.nova", "main.nova", true),
            ("*.nova", "src/deep/main.nova", true),
            ("*.toml", "main.nova", false),
            ("src/*.nova", "src/main.nova", true),
            ("src/*.nova", "src/deep/main.nova", false),
            ("src/**.nova", "src/deep/main.nova", true),
            ("/main.nova", "main.nova", true),
            ("main.nov?", "main.nova", true),
            ("[lm]ain.nova", "main.nova", true),
            ("[!m]ain.nova", "main.nova", false),
            ("[a-k]ain.nova", "main.nova", false),
            ("{lib,main}.nova", "main.nova", true),
            ("{1..3}.nova", "1.nova", false),
        ] {
            assert_eq!(section_matches(glob, path), want, "[{glob}] on {path}");
        }
    }

    /// A fresh directory for one test, under the system temp directory. Its
    /// name is fixed, so each run replaces the last run's.
    fn fresh_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("nova-fmt-editorconfig-{name}"));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("create the test directory");
        dir
    }

    #[test]
    fn nearer_files_and_later_sections_win() {
        let dir = fresh_dir("nearer");
        let sub = dir.join("sub");
        std::fs::create_dir_all(&sub).unwrap();
        std::fs::write(
            dir.join(".editorconfig"),
            "root = true\n[*]\nend_of_line = crlf\ninsert_final_newline = false\n",
        )
        .unwrap();
        std::fs::write(
            sub.join(".editorconfig"),
            "[*.nova]\nend_of_line = lf\n\n# a comment\n[x.nova]\nEND_OF_LINE = CRLF\n",
        )
        .unwrap();
        let both = |crlf| Settings {
            crlf: Some(crlf),
            insert_final_newline: Some(false),
        };
        assert_eq!(settings_for(&sub.join("a.nova")), both(false));
        assert_eq!(settings_for(&sub.join("x.nova")), both(true));
        assert_eq!(settings_for(&dir.join("b.nova")), both(true));
    }

    #[test]
    fn the_search_stops_at_root() {
        let dir = fresh_dir("root");
        let inner = dir.join("inner");
        std::fs::create_dir_all(&inner).unwrap();
        std::fs::write(dir.join(".editorconfig"), "root = true\n[*]\nend_of_line = crlf\n")
            .unwrap();
        std::fs::write(inner.join(".editorconfig"), "root = true\n").unwrap();
        assert_eq!(settings_for(&inner.join("a.nova")), Settings::default());
    }

    #[test]
    fn other_keys_and_other_values_are_ignored() {
        let dir = fresh_dir("ignored");
        std::fs::write(
            dir.join(".editorconfig"),
            "root = true\n[*]\nindent_style = tab\nindent_size = 2\nend_of_line = cr\n\
             insert_final_newline = maybe\n",
        )
        .unwrap();
        assert_eq!(settings_for(&dir.join("a.nova")), Settings::default());
    }
}
```

Create `crates/nova-fmt/tests/files.rs`:

```rust
//! Files: their line endings, their final newline and `.editorconfig`
//! (spec `docs/superpowers/specs/2026-10-07-phase-3-1-formatter-design.md`
//! §5.1, §7.3).

use std::path::{Path, PathBuf};

use nova_fmt::{format_file, format_text, FileError, FormatError, LineEnding};

const UNFORMATTED: &str = "fn main() {\nprintln(\"hi\")\n}\n";
const FORMATTED: &str = "fn main() { println(\"hi\") }\n";

/// A fresh directory for one test, under the system temp directory. Its
/// `.editorconfig` says `root = true` before `sections`, so that one above
/// the temp directory cannot reach the test.
fn fresh_dir(name: &str, sections: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("nova-fmt-file-{name}"));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("create the test directory");
    std::fs::write(dir.join(".editorconfig"), format!("root = true\n{sections}"))
        .expect("write .editorconfig");
    dir
}

fn write(dir: &Path, name: &str, text: impl AsRef<[u8]>) -> PathBuf {
    let path = dir.join(name);
    std::fs::write(&path, text).expect("write the test file");
    path
}

fn crlf(text: &str) -> String {
    text.replace('\n', "\r\n")
}

#[test]
fn line_endings_come_from_the_first_line_break() {
    assert_eq!(LineEnding::of("a\r\nb\n"), LineEnding::Crlf);
    assert_eq!(LineEnding::of("a\nb\r\n"), LineEnding::Lf);
    assert_eq!(LineEnding::of("no break"), LineEnding::Lf);
}

#[test]
fn a_crlf_file_stays_crlf_and_an_lf_file_lf() {
    let dir = fresh_dir("endings", "");
    let file = write(&dir, "crlf.nova", crlf(UNFORMATTED));
    assert_eq!(format_file(&file).unwrap().formatted, crlf(FORMATTED));
    let file = write(&dir, "lf.nova", UNFORMATTED);
    assert_eq!(format_file(&file).unwrap().formatted, FORMATTED);
}

#[test]
fn an_already_formatted_file_is_unchanged() {
    let dir = fresh_dir("unchanged", "");
    let file = write(&dir, "main.nova", FORMATTED);
    let formatted = format_file(&file).unwrap();
    assert!(!formatted.changed(), "{formatted:?}");
    let file = write(&dir, "main.nova", UNFORMATTED);
    assert!(format_file(&file).unwrap().changed());
}

#[test]
fn a_file_with_no_line_break_gets_lf_and_one_final_newline() {
    let dir = fresh_dir("no-break", "");
    let file = write(&dir, "main.nova", "fn main() {}");
    assert_eq!(format_file(&file).unwrap().formatted, "fn main() {}\n");
}

#[test]
fn editorconfig_end_of_line_overrides_the_files_own() {
    let dir = fresh_dir("end-of-line-lf", "[*.nova]\nend_of_line = lf\n");
    let file = write(&dir, "main.nova", crlf(UNFORMATTED));
    assert_eq!(format_file(&file).unwrap().formatted, FORMATTED);
    let dir = fresh_dir("end-of-line-crlf", "[*.nova]\nend_of_line = crlf\n");
    let file = write(&dir, "main.nova", UNFORMATTED);
    assert_eq!(format_file(&file).unwrap().formatted, crlf(FORMATTED));
}

#[test]
fn insert_final_newline_false_keeps_the_files_last_line_as_it_was() {
    let dir = fresh_dir("final-newline", "[*]\ninsert_final_newline = false\n");
    let file = write(&dir, "without.nova", "fn main() {\nprintln(\"hi\")\n}");
    assert_eq!(format_file(&file).unwrap().formatted, "fn main() { println(\"hi\") }");
    let file = write(&dir, "with.nova", UNFORMATTED);
    assert_eq!(format_file(&file).unwrap().formatted, FORMATTED);
}

#[test]
fn a_multi_line_string_keeps_its_files_line_ending() {
    let dir = fresh_dir("string", "");
    let text = crlf("fn main() {\n    let s = \"a\nb\"\n    println(s)\n}\n");
    let file = write(&dir, "main.nova", &text);
    assert_eq!(format_file(&file).unwrap().formatted, text);
}

#[test]
fn a_file_that_is_not_utf8_or_starts_with_a_byte_order_mark_is_refused() {
    let dir = fresh_dir("refused", "");
    let file = write(&dir, "bytes.nova", [0x66u8, 0x6e, 0xff]);
    assert!(matches!(format_file(&file), Err(FileError::NotUtf8)));
    let file = write(&dir, "bom.nova", "\u{feff}fn main() {}\n");
    assert!(matches!(
        format_file(&file),
        Err(FileError::Format(FormatError::Syntax { .. }))
    ));
}

#[test]
fn format_text_keeps_the_inputs_line_ending() {
    assert_eq!(format_text(&crlf(UNFORMATTED)).unwrap(), crlf(FORMATTED));
    assert_eq!(format_text(UNFORMATTED).unwrap(), FORMATTED);
}
```

- [ ] **Step 2: Run them to watch them fail**

```bash
cd /d/Projects/nona/nova && cargo test --locked -p nova-fmt --lib --test files 2>&1 | grep -E "^error(\[E[0-9]+\])?:" | sort | uniq -c | head -8
```

Expected: errors that `format_file`, `format_text`, `FileError` and
`LineEnding` are not in `nova_fmt`, and that `expand`, `section_matches`,
`settings_for` and `Settings` cannot be found.

- [ ] **Step 3: The `.editorconfig` reader**

In `crates/nova-fmt/src/editorconfig.rs`, insert between the module comment
and the tests:

```rust

use std::path::{Path, PathBuf};

/// What the `.editorconfig` files say about one file: `None` where nothing
/// sets a value.
#[derive(Debug, Default, PartialEq, Eq)]
pub(crate) struct Settings {
    /// `Some(true)` for `crlf`, `Some(false)` for `lf`.
    pub crlf: Option<bool>,
    pub insert_final_newline: Option<bool>,
}

/// The settings for `file`, an absolute path, from the `.editorconfig` in
/// its directory and those above it, up to one that says `root = true`.
/// Nearer files override farther ones, and later sections earlier ones.
pub(crate) fn settings_for(file: &Path) -> Settings {
    let mut found: Vec<(PathBuf, String)> = Vec::new();
    for dir in file.ancestors().skip(1) {
        if let Ok(text) = std::fs::read_to_string(dir.join(".editorconfig")) {
            let root = is_root(&text);
            found.push((dir.to_path_buf(), text));
            if root {
                break;
            }
        }
    }
    let mut settings = Settings::default();
    for (dir, text) in found.iter().rev() {
        if let Ok(rel) = file.strip_prefix(dir) {
            let rel = rel.to_string_lossy().replace('\\', "/");
            apply(&mut settings, text, &rel);
        }
    }
    settings
}

/// Whether `text`'s preamble, before its first section, says `root = true`.
fn is_root(text: &str) -> bool {
    for line in text.lines() {
        if line.trim_start().starts_with('[') {
            return false;
        }
        if let Some((key, value)) = pair(line) {
            if key == "root" {
                return value == "true";
            }
        }
    }
    false
}

/// A `key = value` line, both trimmed and lowercased: keys and values are
/// case-insensitive. `None` for a blank line, a comment or a section.
fn pair(line: &str) -> Option<(String, String)> {
    let line = line.trim();
    if line.is_empty() || line.starts_with('#') || line.starts_with(';') || line.starts_with('[')
    {
        return None;
    }
    let (key, value) = line.split_once('=')?;
    Some((key.trim().to_ascii_lowercase(), value.trim().to_ascii_lowercase()))
}

/// Apply, in order, the sections of `text` that cover `rel`: the file's
/// path from the `.editorconfig`'s directory, with `/` separators.
fn apply(settings: &mut Settings, text: &str, rel: &str) {
    let mut active = false;
    for line in text.lines() {
        let line = line.trim();
        if let Some(glob) = line.strip_prefix('[').and_then(|l| l.strip_suffix(']')) {
            active = section_matches(glob, rel);
            continue;
        }
        if !active {
            continue;
        }
        let Some((key, value)) = pair(line) else {
            continue;
        };
        match (key.as_str(), value.as_str()) {
            ("end_of_line", "lf") => settings.crlf = Some(false),
            ("end_of_line", "crlf") => settings.crlf = Some(true),
            ("end_of_line", "unset") => settings.crlf = None,
            ("insert_final_newline", "true") => settings.insert_final_newline = Some(true),
            ("insert_final_newline", "false") => settings.insert_final_newline = Some(false),
            ("insert_final_newline", "unset") => settings.insert_final_newline = None,
            _ => {}
        }
    }
}

/// Whether section `glob` covers `rel`. A glob with no `/` matches the
/// file's name at any depth; one with a `/` matches the whole path from the
/// `.editorconfig`'s directory.
fn section_matches(glob: &str, rel: &str) -> bool {
    let (glob, target) = if glob.contains('/') {
        (glob.strip_prefix('/').unwrap_or(glob), rel)
    } else {
        (glob, rel.rsplit('/').next().unwrap_or(rel))
    };
    let target: Vec<char> = target.chars().collect();
    expand(glob)
        .iter()
        .any(|g| matches(&g.chars().collect::<Vec<char>>(), &target))
}

/// `glob` with its `{a,b}` alternatives expanded. A `{1..3}` range is not
/// supported, so a glob holding one expands to nothing and never matches.
fn expand(glob: &str) -> Vec<String> {
    let chars: Vec<char> = glob.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        match chars[i] {
            '\\' => i += 2,
            '{' => {
                let Some((close, commas)) = brace_group(&chars, i) else {
                    // An unclosed `{` is a literal.
                    i += 1;
                    continue;
                };
                if commas.is_empty() {
                    let inner: String = chars[i + 1..close].iter().collect();
                    if inner.contains("..") {
                        return Vec::new();
                    }
                    // `{word}`, with no comma, is literal.
                    i = close + 1;
                    continue;
                }
                let prefix: String = chars[..i].iter().collect();
                let suffix: String = chars[close + 1..].iter().collect();
                let mut out = Vec::new();
                let mut from = i + 1;
                for &to in commas.iter().chain(std::iter::once(&close)) {
                    let alternative: String = chars[from..to].iter().collect();
                    out.extend(expand(&format!("{prefix}{alternative}{suffix}")));
                    from = to + 1;
                }
                return out;
            }
            _ => i += 1,
        }
    }
    vec![glob.to_owned()]
}

/// The `}` that closes the `{` at `open`, and the commas directly inside
/// it.
fn brace_group(chars: &[char], open: usize) -> Option<(usize, Vec<usize>)> {
    let mut depth = 0;
    let mut commas = Vec::new();
    let mut i = open;
    while i < chars.len() {
        match chars[i] {
            '\\' => i += 1,
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    return Some((i, commas));
                }
            }
            ',' if depth == 1 => commas.push(i),
            _ => {}
        }
        i += 1;
    }
    None
}

/// Whether `glob`, its braces expanded, matches all of `path`: `*` within
/// a path segment, `**` across them, `?` one character, `[…]` and `[!…]` a
/// class, and `\` an escape.
fn matches(glob: &[char], path: &[char]) -> bool {
    match glob {
        [] => path.is_empty(),
        ['*', '*', rest @ ..] => (0..=path.len()).any(|k| matches(rest, &path[k..])),
        ['*', rest @ ..] => (0..=path.len())
            .take_while(|&k| k == 0 || path[k - 1] != '/')
            .any(|k| matches(rest, &path[k..])),
        ['?', rest @ ..] => path.first().is_some_and(|&c| c != '/') && matches(rest, &path[1..]),
        ['[', ..] => class(glob, path),
        ['\\', c, rest @ ..] | [c, rest @ ..] => {
            path.first() == Some(c) && matches(rest, &path[1..])
        }
    }
}

/// A `[…]` or `[!…]` class at the start of `glob`, with `a-z` ranges, then
/// the rest of `glob`. A `[` with no `]` is a literal.
fn class(glob: &[char], path: &[char]) -> bool {
    let Some(end) = glob.iter().skip(1).position(|&c| c == ']').map(|p| p + 1) else {
        return path.first() == Some(&'[') && matches(&glob[1..], &path[1..]);
    };
    let (negated, set) = match &glob[1..end] {
        ['!', set @ ..] => (true, set),
        set => (false, set),
    };
    let Some(&c) = path.first() else {
        return false;
    };
    let mut hit = false;
    let mut k = 0;
    while k < set.len() {
        if k + 2 < set.len() && set[k + 1] == '-' {
            hit |= set[k] <= c && c <= set[k + 2];
            k += 3;
        } else {
            hit |= set[k] == c;
            k += 1;
        }
    }
    c != '/' && hit != negated && matches(&glob[end + 1..], &path[1..])
}
```

- [ ] **Step 4: `format_file` and `format_text`**

Create `crates/nova-fmt/src/file.rs`:

```rust
//! A file's line endings and final newline (spec
//! `docs/superpowers/specs/2026-10-07-phase-3-1-formatter-design.md` §5.1,
//! §7.3).

use std::path::{Path, PathBuf};

use crate::{editorconfig, FormatError};

/// A file's line ending.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LineEnding {
    /// `\n`.
    Lf,
    /// `\r\n`.
    Crlf,
}

impl LineEnding {
    /// The line ending of `text`'s first line break, or LF if it has none
    /// (spec §7.3).
    pub fn of(text: &str) -> LineEnding {
        match text.find('\n') {
            Some(i) if text[..i].ends_with('\r') => LineEnding::Crlf,
            _ => LineEnding::Lf,
        }
    }

    /// `lf`, a text with `\n` line endings, in this line ending.
    fn apply(self, lf: &str) -> String {
        match self {
            LineEnding::Lf => lf.to_owned(),
            LineEnding::Crlf => lf.replace('\n', "\r\n"),
        }
    }
}

/// A file, formatted: what it holds, and what it should hold.
#[derive(Debug)]
pub struct Formatted {
    /// The file as it is.
    pub original: String,
    /// The file as `nova fmt` would write it.
    pub formatted: String,
}

impl Formatted {
    /// Whether formatting would change the file.
    pub fn changed(&self) -> bool {
        self.original != self.formatted
    }
}

/// Why a file was not formatted (spec §7.2).
#[derive(Debug, thiserror::Error)]
pub enum FileError {
    /// It could not be read.
    #[error("{0}")]
    Io(std::io::Error),
    /// It is not UTF-8.
    #[error("the file is not UTF-8")]
    NotUtf8,
    /// It does not lex or parse, or the self-check refused the output.
    #[error(transparent)]
    Format(FormatError),
}

/// Format the file at `path` (spec §5.1): [`crate::format`]'s output, with
/// the line ending and the final newline §7.3 gives it. Nothing is written.
pub fn format_file(path: &Path) -> Result<Formatted, FileError> {
    let bytes = std::fs::read(path).map_err(FileError::Io)?;
    let original = String::from_utf8(bytes).map_err(|_| FileError::NotUtf8)?;
    let settings = editorconfig::settings_for(&absolute(path));
    let ending = match settings.crlf {
        Some(true) => LineEnding::Crlf,
        Some(false) => LineEnding::Lf,
        None => LineEnding::of(&original),
    };
    let mut lf = crate::format_named(&original, &path.display().to_string())
        .map_err(FileError::Format)?;
    // With `insert_final_newline = false`, the file ends in a newline only
    // if it did (the 3.1 plan's decision 13).
    if settings.insert_final_newline == Some(false) && !original.ends_with('\n') {
        lf.pop();
    }
    Ok(Formatted {
        formatted: ending.apply(&lf),
        original,
    })
}

/// Format `source` from standard input (spec §7.1): [`crate::format`]'s
/// output, in the input's own line ending.
pub fn format_text(source: &str) -> Result<String, FormatError> {
    let lf = crate::format(source)?;
    Ok(LineEnding::of(source).apply(&lf))
}

/// `path`, made absolute against the current directory, so that the
/// `.editorconfig` search can walk all the way up.
fn absolute(path: &Path) -> PathBuf {
    if path.is_absolute() {
        return path.to_path_buf();
    }
    match std::env::current_dir() {
        Ok(cwd) => cwd.join(path),
        Err(_) => path.to_path_buf(),
    }
}
```

In `crates/nova-fmt/src/lib.rs`, add `mod file;` after `mod editorconfig;`,
and after the `mod` lines:

```rust

pub use file::{format_file, format_text, FileError, Formatted, LineEnding};
```

`std::path::absolute` would do `absolute`'s work, but it is newer than the
MSRV (1.79).

- [ ] **Step 5: Run the tests to watch them pass**

```bash
cd /d/Projects/nona/nova && P=/c/Users/SAKEER~1/AppData/Local/Temp/gcm/p31 && cargo test --locked -p nova-fmt > $P/task7.txt 2>&1; echo "exit=$?"; grep -E "^test result|FAILED|panicked" $P/task7.txt | head -30; grep -c -E -- "--> crates.nova-fmt" $P/task7.txt
```

Expected: `exit=0`, and six `test result: ok.` lines: the library's 27 (22
before, and the reader's 5), `comments`'s 18, `files`'s 9, `layout`'s 29,
`source_rules`'s 21, and the doc-tests' 0. The last count is `0`.

- [ ] **Step 6: Commit**

Write `$P/msg-7.txt`:

```
nova-fmt: format a file in its own line endings

format_file is the entry point for nova fmt, and for 3.2's LSP (spec
docs/superpowers/specs/2026-10-07-phase-3-1-formatter-design.md §5.1,
§7.3). It formats in memory and writes nothing.

- Line endings come from .editorconfig's end_of_line if it says lf or
  crlf, else from the file's first line break, else LF. A multi-line
  string therefore keeps its file's line ending.
- The file ends in exactly one newline, unless .editorconfig says
  insert_final_newline = false. Then it ends in one only if it did.
- .editorconfig is read by a reader of our own: the walk up to
  root = true, EditorConfig's globs without numeric ranges, and only
  those two keys.
- A file that is not UTF-8 is refused. One that starts with a byte-order
  mark fails to lex, as it does for nova check.

format_text does the same for --stdin, in the input's line ending.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
```

```bash
cd /d/Projects/nona/nova && P=/c/Users/SAKEER~1/AppData/Local/Temp/gcm/p31 && cargo fmt --all && git add crates/nova-fmt && git commit -q -F $P/msg-7.txt && git log -1 --format=%s
```

Expected: `nova-fmt: format a file in its own line endings`.

The task's test command: `cargo test --locked -p nova-fmt`.

---
### Task 8: The corpus gate

Spec §9.4. Every `.nova` file in the repository that parses must format to
the same program, with the same comments, and formatting the output again
must change nothing. The test does its own AST and comment comparison, so a
broken self-check cannot pass it.

**Files:**
- Create: `crates/nova-fmt/tests/corpus.rs`

**Interfaces:**
- Consumes: Task 5's `format_named`; `nova_lexer::lex_with_comments`,
  `nova_parser::parse`, and the AST's `Item` and `ImportKind`.
- Produces: nothing later tasks call.

- [ ] **Step 1: Write the test**

Create `crates/nova-fmt/tests/corpus.rs`:

```rust
//! The corpus gate (spec
//! `docs/superpowers/specs/2026-10-07-phase-3-1-formatter-design.md` §9.4).
//!
//! Every `.nova` file in the repository that parses must format to the same
//! program and the same comments, and formatting the output again must
//! change nothing. These checks are written here, apart from the library's
//! own self-check (§5.5), so that a broken self-check cannot pass them.

use std::path::{Path, PathBuf};

use nova_ast::item::ImportKind;
use nova_ast::Item;
use nova_diagnostics::FileDb;
use nova_lexer::CommentKind;

/// The only file expected not to parse: it uses a `::<User>` turbofish.
const EXPECTED_SKIP: &str = "crates/nova-parser/tests/fixtures/async.nova";

/// The gate's floor: the 168 tracked files, less the one that does not
/// parse.
const AT_LEAST: usize = 167;

/// The repository, two levels above this crate.
fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join("..")
}

/// Every `.nova` file under `dir`, skipping `target/`, directories whose
/// names begin with `.`, and symbolic links.
fn nova_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let mut entries: Vec<_> = std::fs::read_dir(dir)
        .unwrap()
        .map(|e| e.unwrap())
        .collect();
    entries.sort_by_key(|e| e.file_name());
    for e in entries {
        let name = e.file_name().to_string_lossy().into_owned();
        let kind = e.file_type().unwrap();
        if kind.is_dir() {
            if name != "target" && !name.starts_with('.') {
                nova_files(&e.path(), out);
            }
        } else if kind.is_file() && name.ends_with(".nova") {
            out.push(e.path());
        }
    }
}

/// `debug`, a `{:?}` rendering, without its `, span: Span { … }` fields.
fn without_spans(debug: &str) -> String {
    const FIELD: &str = ", span: Span { ";
    let mut out = String::new();
    let mut rest = debug;
    while let Some(at) = rest.find(FIELD) {
        out.push_str(&rest[..at]);
        let after = &rest[at + FIELD.len()..];
        rest = after.find(" }").map_or("", |end| &after[end + 2..]);
    }
    out.push_str(rest);
    out
}

/// What must survive formatting, or `None` if `text` does not lex and
/// parse: the AST without spans, with each run of top-level imports sorted
/// as the formatter sorts it (spec §6), and the comments, each as its kind
/// and its text without trailing whitespace on any line. The comments
/// inside an import run are compared as a set, since sorting the run moves
/// them (§5.5).
fn summary(text: &str) -> Option<(String, Vec<String>)> {
    let mut db = FileDb::new();
    let id = db.add("corpus.nova", text);
    let (tokens, comments, lex_errors) = nova_lexer::lex_with_comments(text, id);
    let (file, parse_errors) = nova_parser::parse(&tokens, id);
    let mut file = file?;
    if !lex_errors.is_empty() || !parse_errors.is_empty() {
        return None;
    }
    let mut regions = Vec::new();
    let mut i = 0;
    while i < file.items.len() {
        if !matches!(file.items[i].value, Item::Import(_)) {
            i += 1;
            continue;
        }
        let start = i;
        while i < file.items.len() && matches!(file.items[i].value, Item::Import(_)) {
            i += 1;
        }
        let from = if start == 0 { 0 } else { file.items[start - 1].span.end };
        let to = file.items.get(i).map_or(text.len() as u32, |next| next.span.start);
        regions.push((from, to));
        let run = &mut file.items[start..i];
        run.sort_by_key(|item| match &item.value {
            Item::Import(import) => {
                let names: Vec<&str> =
                    import.path.value.segments.iter().map(|s| s.value.as_str()).collect();
                names.join("::")
            }
            _ => String::new(),
        });
        for item in run {
            if let Item::Import(import) = &mut item.value {
                if let ImportKind::List(names) = &mut import.kind {
                    names.sort_by(|a, b| a.value.cmp(&b.value));
                }
            }
        }
    }
    let mut ordered = Vec::new();
    let mut in_runs = vec![Vec::new(); regions.len()];
    for c in &comments {
        let kind = match c.kind {
            CommentKind::Line => "line",
            CommentKind::Block => "block",
        };
        let body: Vec<&str> = text[c.span.start as usize..c.span.end as usize]
            .lines()
            .map(str::trim_end)
            .collect();
        let entry = format!("{kind}: {}", body.join("\n"));
        match regions
            .iter()
            .position(|&(s, e)| c.span.start >= s && c.span.start < e)
        {
            Some(k) => in_runs[k].push(entry),
            None => ordered.push(entry),
        }
    }
    for mut run in in_runs {
        run.sort();
        ordered.push(format!("import run: {run:?}"));
    }
    Some((without_spans(&format!("{file:?}")), ordered))
}

#[test]
fn every_corpus_file_formats_to_the_same_program_and_comments() {
    let root = root();
    let mut files = Vec::new();
    nova_files(&root, &mut files);
    let mut checked = 0;
    let mut skipped = Vec::new();
    let mut failures = Vec::new();
    for path in &files {
        let rel = path
            .strip_prefix(&root)
            .unwrap()
            .to_string_lossy()
            .replace('\\', "/");
        let text = std::fs::read_to_string(path).unwrap().replace("\r\n", "\n");
        let Some(before) = summary(&text) else {
            skipped.push(rel);
            continue;
        };
        checked += 1;
        let out = match nova_fmt::format_named(&text, &rel) {
            Ok(out) => out,
            Err(e) => {
                failures.push(format!("{rel}: {e:?}"));
                continue;
            }
        };
        match summary(&out) {
            None => failures.push(format!("{rel}: the output does not parse")),
            Some(after) if after.0 != before.0 => {
                failures.push(format!("{rel}: the AST changed"))
            }
            Some(after) if after.1 != before.1 => {
                failures.push(format!("{rel}: the comments changed"))
            }
            Some(_) => {}
        }
        match nova_fmt::format_named(&out, &rel) {
            Ok(again) if again == out => {}
            Ok(_) => failures.push(format!("{rel}: formatting it again changes it")),
            Err(e) => failures.push(format!("{rel}: formatting it again fails: {e:?}")),
        }
    }
    eprintln!("checked {checked} files; skipped {skipped:?}");
    assert!(failures.is_empty(), "{} failures:\n{}", failures.len(), failures.join("\n"));
    assert_eq!(skipped, vec![EXPECTED_SKIP.to_owned()], "the files that do not parse");
    assert!(checked >= AT_LEAST, "only {checked} files were checked");
}
```

- [ ] **Step 2: Run it**

```bash
cd /d/Projects/nona/nova && cargo test --locked -p nova-fmt --test corpus -- --nocapture 2>&1 | grep -E "^checked|^test result|failures:|^[a-z].*\.nova:" | head -40
```

Expected: `checked 167 files; skipped ["crates/nova-parser/tests/fixtures/async.nova"]`
and `test result: ok. 1 passed`. This gate checks the formatter Tasks 5 and
6 built, so it may pass on its first run. If it lists failures, each names
a file and what changed. Reduce that file to the smallest input that still
fails, add that input as a test in the task file that owns the construct
(`layout.rs`, `source_rules.rs` or `comments.rs`), watch it fail, fix the
printer, and ledger the case.

- [ ] **Step 3: Watch the floor fail**

The floor is the one check that a correct formatter never trips. Change
`const AT_LEAST: usize = 167;` to `const AT_LEAST: usize = 1000;`, then:

```bash
cd /d/Projects/nona/nova && cargo test --locked -p nova-fmt --test corpus 2>&1 | grep -E "only [0-9]+ files|^test result"
```

Expected: `only 167 files were checked` and `test result: FAILED`. Change the
constant back to `167`, and check that `git status --short` prints only
`?? crates/nova-fmt/tests/corpus.rs`.

- [ ] **Step 4: Commit**

Write `$P/msg-8.txt`:

```
nova-fmt: hold every .nova file in the repository to the gate

The corpus test formats each of the 167 files that parse (spec
docs/superpowers/specs/2026-10-07-phase-3-1-formatter-design.md §9.4)
and checks, with its own code rather than the library's self-check, that:

1. the output's AST equals the input's, with import runs sorted;
2. its comments equal the input's, in order outside import runs;
3. formatting the output again changes nothing.

async.nova is the one file expected not to parse, and fewer than 167
checked files fails the test, so no file can drop out unnoticed.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
```

```bash
cd /d/Projects/nona/nova && P=/c/Users/SAKEER~1/AppData/Local/Temp/gcm/p31 && cargo fmt --all && git add crates/nova-fmt && git commit -q -F $P/msg-8.txt && git log -1 --format=%s
```

Expected: `nova-fmt: hold every .nova file in the repository to the gate`.

The task's test command: `cargo test --locked -p nova-fmt`.

---
### Task 9: `nova fmt`

Spec §7 and §9.5: the command, its modes, exit codes and errors, and its
end-to-end tests, two of which are half of the phase plan's gate.

**Files:**
- Create: `crates/nova-cli/src/cmd/fmt.rs`
- Modify: `crates/nova-cli/src/cmd/mod.rs`, `crates/nova-cli/src/main.rs`
- Modify: `crates/nova-cli/Cargo.toml`, and `Cargo.lock` through one
  unlocked build
- Create: `crates/nova-cli/tests/fmt.rs`

**Interfaces:**
- Consumes: Task 7's `format_file`, `format_text`, `Formatted::changed`,
  `FileError`, and Task 4's `FormatError`; 3.0's `nova_pm::find_root`.
- Produces: `nova fmt [PATH]... [--check] [--stdin]`, exiting 0, 1 or 2;
  `cmd::fmt::run(cmd: FmtCmd) -> i32`.

- [ ] **Step 1: Write the failing tests**

Create `crates/nova-cli/tests/fmt.rs`:

```rust
//! End-to-end tests of `nova fmt` (spec
//! `docs/superpowers/specs/2026-10-07-phase-3-1-formatter-design.md` §7 and
//! §9.5).
//!
//! Every test works in its own directory under the system temp directory.
//! Each holds an `.editorconfig` that says `root = true`, so that one above
//! the temp directory cannot change a test's line endings.

use assert_cmd::Command;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

const UNFORMATTED: &str = "fn main() {\nprintln(\"hi\")\n}\n";
const FORMATTED: &str = "fn main() { println(\"hi\") }\n";

fn nova() -> Command {
    Command::cargo_bin("nova").expect("nova binary builds")
}

/// A fresh directory for one test. Its name is fixed, so each run replaces
/// the last run's.
fn fresh_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("nova-fmt-{name}"));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("create the test directory");
    std::fs::write(dir.join(".editorconfig"), "root = true\n").expect("write .editorconfig");
    dir
}

/// Write `text` at `rel` under `dir`, creating its directories.
fn write(dir: &Path, rel: &str, text: impl AsRef<[u8]>) -> PathBuf {
    let path = dir.join(rel);
    std::fs::create_dir_all(path.parent().unwrap()).expect("create the parent");
    std::fs::write(&path, text).expect("write the file");
    path
}

fn read(path: &Path) -> String {
    std::fs::read_to_string(path).expect("read the file")
}

fn stdout(assert: &assert_cmd::assert::Assert) -> String {
    String::from_utf8_lossy(&assert.get_output().stdout).into_owned()
}

fn stderr(assert: &assert_cmd::assert::Assert) -> String {
    String::from_utf8_lossy(&assert.get_output().stderr).into_owned()
}

#[test]
fn check_fails_on_an_unformatted_file_and_passes_once_it_is_formatted() {
    let dir = fresh_dir("gate");
    let file = write(&dir, "main.nova", UNFORMATTED);
    let out = nova().arg("fmt").arg("--check").arg(&file).assert().code(1);
    assert_eq!(stdout(&out), format!("would reformat: {}\n", file.display()));
    assert_eq!(read(&file), UNFORMATTED, "--check writes nothing");
    nova().arg("fmt").arg(&file).assert().success().stdout("");
    assert_eq!(read(&file), FORMATTED);
    assert!(!dir.join(".main.nova.nova-fmt.tmp").exists(), "a temporary file was left");
    nova().arg("fmt").arg("--check").arg(&file).assert().success().stdout("");
}

#[test]
fn stdin_formats_to_stdout_and_check_sets_only_the_exit_code() {
    nova()
        .args(["fmt", "--stdin"])
        .write_stdin(UNFORMATTED)
        .assert()
        .success()
        .stdout(FORMATTED);
    nova()
        .args(["fmt", "--stdin", "--check"])
        .write_stdin(UNFORMATTED)
        .assert()
        .code(1)
        .stdout("");
    nova()
        .args(["fmt", "--stdin", "--check"])
        .write_stdin(FORMATTED)
        .assert()
        .success()
        .stdout("");
}

#[test]
fn a_directory_is_searched_skipping_target_and_dot_directories() {
    let dir = fresh_dir("directories");
    let formatted = [
        write(&dir, "a.nova", UNFORMATTED),
        write(&dir, "sub/b.nova", UNFORMATTED),
    ];
    let skipped = [
        write(&dir, "target/c.nova", UNFORMATTED),
        write(&dir, ".hidden/d.nova", UNFORMATTED),
        write(&dir, "notes.txt", UNFORMATTED),
    ];
    nova().arg("fmt").arg(&dir).assert().success();
    for file in &formatted {
        assert_eq!(read(file), FORMATTED, "{}", file.display());
    }
    for file in &skipped {
        assert_eq!(read(file), UNFORMATTED, "{}", file.display());
    }
}

#[test]
fn no_path_formats_the_projects_src_even_from_a_subdirectory() {
    let dir = fresh_dir("project");
    write(
        &dir,
        "nova.toml",
        "[package]\nname = \"demo\"\nversion = \"0.1.0\"\nedition = \"2026\"\n",
    );
    let main = write(&dir, "src/main.nova", UNFORMATTED);
    let more = write(&dir, "src/util/more.nova", UNFORMATTED);
    let outside = write(&dir, "scratch.nova", UNFORMATTED);
    nova()
        .arg("fmt")
        .current_dir(dir.join("src").join("util"))
        .assert()
        .success();
    assert_eq!(read(&main), FORMATTED);
    assert_eq!(read(&more), FORMATTED);
    assert_eq!(read(&outside), UNFORMATTED, "only src/ is the project's");
}

#[test]
fn no_path_outside_a_project_uses_src_or_asks_for_paths() {
    let dir = fresh_dir("no-project");
    let out = nova().arg("fmt").current_dir(&dir).assert().code(2);
    assert!(stderr(&out).contains("name the files or directories"), "{}", stderr(&out));
    let main = write(&dir, "src/main.nova", UNFORMATTED);
    nova().arg("fmt").current_dir(&dir).assert().success();
    assert_eq!(read(&main), FORMATTED);
}

#[test]
fn a_file_with_a_syntax_error_is_left_untouched_and_the_rest_are_formatted() {
    let dir = fresh_dir("syntax-error");
    let bad = "fn main( {\n";
    let broken = write(&dir, "a.nova", bad);
    let good = write(&dir, "b.nova", UNFORMATTED);
    let out = nova().arg("fmt").arg(&dir).assert().code(2);
    assert!(stderr(&out).contains("P0001"), "{}", stderr(&out));
    assert_eq!(read(&broken), bad);
    assert_eq!(read(&good), FORMATTED);
    // With `--check`, the error outranks a file that would change.
    write(&dir, "b.nova", UNFORMATTED);
    nova().arg("fmt").arg("--check").arg(&dir).assert().code(2);
}

#[test]
fn an_already_formatted_file_is_not_rewritten() {
    let dir = fresh_dir("unchanged");
    let file = write(&dir, "main.nova", FORMATTED);
    let old = SystemTime::UNIX_EPOCH + Duration::from_secs(1_000_000_000);
    std::fs::File::options()
        .write(true)
        .open(&file)
        .unwrap()
        .set_modified(old)
        .unwrap();
    nova().arg("fmt").arg(&file).assert().success();
    assert_eq!(std::fs::metadata(&file).unwrap().modified().unwrap(), old);
}

#[test]
fn crlf_in_gives_crlf_out() {
    let dir = fresh_dir("crlf");
    let file = write(&dir, "main.nova", UNFORMATTED.replace('\n', "\r\n"));
    nova().arg("fmt").arg(&file).assert().success();
    assert_eq!(read(&file), FORMATTED.replace('\n', "\r\n"));
}

#[test]
fn editorconfig_can_force_lf_and_keep_a_missing_final_newline() {
    let dir = fresh_dir("editorconfig");
    write(
        &dir,
        ".editorconfig",
        "root = true\n\n[*.nova]\nend_of_line = lf\n\n[keep.nova]\ninsert_final_newline = false\n",
    );
    let crlf = write(&dir, "crlf.nova", UNFORMATTED.replace('\n', "\r\n"));
    let keep = write(&dir, "keep.nova", "fn main() {\nprintln(\"hi\")\n}");
    nova().arg("fmt").arg(&dir).assert().success();
    assert_eq!(read(&crlf), FORMATTED);
    assert_eq!(read(&keep), "fn main() { println(\"hi\") }");
}

#[test]
fn nova_check_accepts_a_doc_comment_before_an_item() {
    let dir = fresh_dir("doc-comment");
    let file = write(
        &dir,
        "main.nova",
        "/// The entry point.\nfn main() {\n    println(\"hi\")\n}\n",
    );
    nova().arg("check").arg(&file).assert().success();
}

#[test]
fn a_missing_path_or_a_file_that_is_not_utf8_is_an_error() {
    let dir = fresh_dir("errors");
    let out = nova().arg("fmt").arg(dir.join("nope.nova")).assert().code(2);
    assert!(stderr(&out).contains("does not exist"), "{}", stderr(&out));
    let bytes = write(&dir, "bytes.nova", [0x66u8, 0x6e, 0xff]);
    let out = nova().arg("fmt").arg(&bytes).assert().code(2);
    assert!(stderr(&out).contains("not UTF-8"), "{}", stderr(&out));
    assert_eq!(std::fs::read(&bytes).unwrap(), [0x66u8, 0x6e, 0xff]);
}
```

- [ ] **Step 2: Run them to watch them fail**

```bash
cd /d/Projects/nona/nova && cargo test --locked -p nova-cli --test fmt 2>&1 | grep -E "^test |^test result"
```

Expected: `test result: FAILED. 1 passed; 10 failed`. clap rejects `fmt` as
an unknown subcommand. The one that passes is
`nova_check_accepts_a_doc_comment_before_an_item`: Task 2 made the parser
accept `///`, and the test is here as half of the phase plan's gate.

- [ ] **Step 3: Depend on `nova-fmt`**

In `crates/nova-cli/Cargo.toml`, add after the `nova-pm` line:

```toml
nova-fmt = { path = "../nova-fmt" }
```

Then the plan's one cargo call without `--locked`:

```bash
cd /d/Projects/nona/nova && cargo build -p nova-cli 2>&1 | tail -1 && git diff --stat -- Cargo.lock && git diff -U0 -- Cargo.lock | grep -E "^[-+] "
```

Expected: the build finishes, `Cargo.lock | 1 +`, and the one changed line
is `+ "nova-fmt",`, in `nova-cli`'s dependency list. `nova-fmt` is already a
workspace member, so no package is added.

- [ ] **Step 4: The command**

Create `crates/nova-cli/src/cmd/fmt.rs`:

```rust
//! `nova fmt`: format Nova source files (spec
//! `docs/superpowers/specs/2026-10-07-phase-3-1-formatter-design.md` §7).

use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use clap::Args;
use nova_fmt::{FileError, FormatError};

#[derive(Args)]
pub struct FmtCmd {
    /// Files or directories to format (default: the project's `src/`).
    paths: Vec<PathBuf>,

    /// Write nothing; print each file that would change, and exit 1 if any
    /// would.
    #[arg(long)]
    check: bool,

    /// Read source from standard input and write it, formatted, to standard
    /// output.
    #[arg(long, conflicts_with = "paths")]
    stdin: bool,
}

/// Run `nova fmt` and return its exit code (spec §7.2): 0 when everything
/// is formatted, 1 when `--check` finds a file that is not, and 2 on any
/// error, which outranks a file that would change.
pub fn run(cmd: FmtCmd) -> i32 {
    let code = if cmd.stdin {
        stdin(cmd.check)
    } else {
        files(&cmd.paths, cmd.check)
    };
    let _ = std::io::stdout().flush();
    code
}

fn stdin(check: bool) -> i32 {
    let mut source = String::new();
    if let Err(e) = std::io::stdin().read_to_string(&mut source) {
        eprintln!("error: reading standard input: {e}");
        return 2;
    }
    match nova_fmt::format_text(&source) {
        Ok(out) if check => i32::from(out != source),
        Ok(out) => {
            print!("{out}");
            0
        }
        Err(e) => {
            report("<stdin>", &e);
            2
        }
    }
}

fn files(paths: &[PathBuf], check: bool) -> i32 {
    let files = match collect(paths) {
        Ok(files) => files,
        Err(message) => {
            eprintln!("error: {message}");
            return 2;
        }
    };
    let mut code = 0;
    for file in &files {
        let shown = file.display().to_string();
        match nova_fmt::format_file(file) {
            Ok(f) if !f.changed() => {}
            Ok(_) if check => {
                println!("would reformat: {shown}");
                code = code.max(1);
            }
            Ok(f) => {
                if let Err(e) = write(file, &f.formatted) {
                    eprintln!("error: writing {shown}: {e}");
                    code = 2;
                }
            }
            Err(FileError::Io(e)) => {
                eprintln!("error: reading {shown}: {e}");
                code = 2;
            }
            Err(FileError::NotUtf8) => {
                eprintln!("error: {shown} is not UTF-8, so it was left unchanged");
                code = 2;
            }
            Err(FileError::Format(e)) => {
                report(&shown, &e);
                code = 2;
            }
        }
    }
    code
}

/// Print why `file` was left unchanged: its diagnostics as `nova check`
/// prints them, or what the self-check found (spec §7.2).
fn report(file: &str, e: &FormatError) {
    match e {
        FormatError::Syntax { rendered, .. } => eprint!("{rendered}"),
        FormatError::Internal { first_difference } => eprintln!(
            "error: nova fmt could not format {file} safely, so it was left unchanged: \
             {first_difference}"
        ),
    }
}

/// The files to format, sorted (spec §7.1): the paths given, each
/// directory searched for `*.nova`. With none, the project's `src/`, or
/// outside a project `src/` if `src/main.nova` exists.
fn collect(paths: &[PathBuf]) -> Result<Vec<PathBuf>, String> {
    let mut files = Vec::new();
    if paths.is_empty() {
        let cwd = std::env::current_dir()
            .map_err(|e| format!("reading the current directory: {e}"))?;
        let src = match nova_pm::find_root(&cwd) {
            Some(root) => root.join("src"),
            None if Path::new("src/main.nova").is_file() => PathBuf::from("src"),
            None => {
                return Err("no project here, and no src/main.nova: \
                            name the files or directories to format"
                    .to_owned())
            }
        };
        search(&src, &mut files).map_err(|e| format!("searching {}: {e}", src.display()))?;
    }
    for path in paths {
        if path.is_dir() {
            search(path, &mut files)
                .map_err(|e| format!("searching {}: {e}", path.display()))?;
        } else if path.is_file() {
            files.push(path.clone());
        } else {
            return Err(format!("{} does not exist", path.display()));
        }
    }
    files.sort();
    files.dedup();
    Ok(files)
}

/// Every `*.nova` file under `dir`, but not in `target/`, in a directory
/// whose name begins with `.`, or behind a symbolic link to a directory.
fn search(dir: &Path, files: &mut Vec<PathBuf>) -> std::io::Result<()> {
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let kind = entry.file_type()?;
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().into_owned();
        if kind.is_dir() {
            if name != "target" && !name.starts_with('.') {
                search(&path, files)?;
            }
        } else if name.ends_with(".nova") && (kind.is_file() || path.is_file()) {
            files.push(path);
        }
    }
    Ok(())
}

/// Write `text` over `file` by way of a temporary file in the same
/// directory, renamed over it, so that an interrupted run never leaves half
/// a file (spec §7.1).
fn write(file: &Path, text: &str) -> std::io::Result<()> {
    let name = file
        .file_name()
        .map_or_else(String::new, |n| n.to_string_lossy().into_owned());
    let tmp = file.with_file_name(format!(".{name}.nova-fmt.tmp"));
    std::fs::write(&tmp, text)?;
    std::fs::rename(&tmp, file).map_err(|e| {
        let _ = std::fs::remove_file(&tmp);
        e
    })
}
```

In `crates/nova-cli/src/cmd/mod.rs`, add `pub mod fmt;` before `pub mod new;`.

In `crates/nova-cli/src/main.rs`:

- the module comment becomes:

  ```rust
  //! The `nova` command-line tool.
  //!
  //! Dispatches to subcommands: parse, run, build, check, test, fmt, new,
  //! init and version. Phase 0 implemented `nova parse`, Phase 1 `nova run`
  //! (Cranelift JIT) and `nova check`, Phase 3.0 the project commands (spec
  //! `docs/superpowers/specs/2026-10-07-phase-3-0-foundations-design.md`),
  //! and Phase 3.1 `nova fmt` (spec
  //! `docs/superpowers/specs/2026-10-07-phase-3-1-formatter-design.md`).
  ```
- after the `Test(cmd::test::TestCmd),` variant, add:

  ```rust
      /// Format Nova source files: the project's `src/`, or the files and
      /// directories given.
      Fmt(cmd::fmt::FmtCmd),
  ```
- after the `Command::Test(cmd) => cmd::test::run(cmd),` arm, add:

  ```rust
          // Its exit code says more than success or failure (spec §7.2).
          Command::Fmt(cmd) => std::process::exit(cmd::fmt::run(cmd)),
  ```

- [ ] **Step 5: Run the tests to watch them pass**

```bash
cd /d/Projects/nona/nova && P=/c/Users/SAKEER~1/AppData/Local/Temp/gcm/p31 && cargo test --locked -p nova-cli --test fmt > $P/task9.txt 2>&1; echo "exit=$?"; grep -E "^test result|FAILED|panicked" $P/task9.txt
```

Expected: `exit=0` and `test result: ok. 11 passed; 0 failed`.

Then try it by hand once, on std, without writing anything:

```bash
cd /d/Projects/nona/nova && cargo run --locked -q -p nova-cli -- fmt --check std examples; echo "exit=$?"
```

Expected: `exit=1`, after a `would reformat:` line for each std and example
file that Task 11 will reformat. No `error:` line, and no diagnostics: every
one of the 23 files parses and passes the self-check (Task 8 checked them
in memory).

- [ ] **Step 6: Commit**

Write `$P/msg-9.txt`:

```
nova-cli: nova fmt

nova fmt [PATH]... [--check] [--stdin] (spec
docs/superpowers/specs/2026-10-07-phase-3-1-formatter-design.md §7):

- With no path, it formats the project's src/, found from the current
  directory as 3.0's commands find it, or src/ when src/main.nova exists.
- A directory is searched for *.nova, skipping target/, dot-directories
  and symbolic links to directories. Files go in sorted order.
- --check writes nothing and prints "would reformat: <path>" for each
  file that would change. --stdin formats standard input to standard
  output.
- A file is written only if it changes, through a temporary file renamed
  over it.
- It exits 0, 1 when --check finds a file that would change, or 2 on any
  error. A file that does not parse is left untouched, its diagnostics
  printed as nova check prints them, and the others are still formatted.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
```

```bash
cd /d/Projects/nona/nova && P=/c/Users/SAKEER~1/AppData/Local/Temp/gcm/p31 && cargo fmt --all && git add Cargo.lock crates/nova-cli && git commit -q -F $P/msg-9.txt && git log -1 --format=%s
```

Expected: `nova-cli: nova fmt`.

The task's test command: `cargo test --locked -p nova-cli --test fmt`.

---
### Task 10: The mutants

Spec §9.6. Each mutant, applied alone to the committed code, must make its
named test fail. A mutant is never committed.

**Files:**
- Modify, then restore: `crates/nova-fmt/src/lib.rs`, `src/source.rs`,
  `src/print/mod.rs`, `src/print/expr.rs`, `src/print/item.rs`

**Interfaces:**
- Consumes: the tests of Tasks 4 to 6, by name.
- Produces: seven ledger lines, one per mutant.

For each mutant below:

1. Check that `git status --short` prints nothing.
2. Make the one edit with the Edit tool. rustfmt may have rewrapped the
   code since this plan was written, so find the construct the mutant
   names rather than an exact line.
3. Run its command and read the named test's line. It must read
   `test <name> ... FAILED`. A compile error, or a run with no `test result`
   line, proves nothing: fix the edit and run again.
4. Restore with `git checkout -- <file>`, and check that `git status --short`
   prints nothing.
5. Ledger it: `Task 10: M<n> <what> -> <test> FAILED (restored)`.

| # | The mutant (spec §9.6) | The edit | The command and its named test |
|---|---|---|---|
| M1 | the self-check skipped | in `lib.rs`'s `format_with`, delete the statement `check::check(&input, &output)?;` | `cargo test --locked -p nova-fmt --lib the_self_check_refuses_output_that_lost_a_comment` |
| M2 | trailing comments dropped | in `print/mod.rs`'s `trailing`, replace `let taken = self.comments.take_trailing(self.src.text, offset);` with `let _ = offset; let taken: &[Comment] = &[];` | `cargo test --locked -p nova-fmt --test comments a_trailing_comment_in_a_list_goes_after_its_comma` |
| M3 | the author's parentheses dropped | in `source.rs`'s `is_parenthesized`, change `return t.span.end == span.end;` to `return t.span.end == span.end && false;` | `cargo test --locked -p nova-fmt --test source_rules the_authors_parentheses_are_kept` |
| M4 | §5.6's first rule never printing a separator | in `print/expr.rs`'s `separator`, append `.filter(\|_\| false)` to the final `.then_some(";")` | `cargo test --locked -p nova-fmt --test source_rules a_statement_starting_with_a_parenthesis_keeps_its_semicolon` |
| M5 | §5.6's second rule never keeping a `return` or `break`'s `;` | in `print/expr.rs`'s `separator`, change `Some(Token::Return \| Token::Break)` to `Some(Token::Eof)` | `cargo test --locked -p nova-fmt --test source_rules a_value_less_return_keeps_its_semicolon` |
| M6 | impl members printed grouped by kind | in `print/item.rs`'s `impl_block`, delete the statement `members.sort_by_key(\|m\| self.member_extent(*m).0);` | `cargo test --locked -p nova-fmt --test source_rules impl_members_keep_their_source_order` |
| M7 | import runs merged across other items | in `print/mod.rs`'s `file`, change the inner loop's condition `while i < items.len() && matches!(items[i].value, Item::Import(_))` to `while i < items.len()` | `cargo test --locked -p nova-fmt --test layout imports_are_sorted_only_within_their_run` |

In the table, `\|` stands for `|`: Markdown needs the backslash there.

Each command, with the output it needs, as one line:

```bash
cd /d/Projects/nona/nova && <the command> 2>&1 | grep -E "^test |^test result"
```

Expected, for each: the named test's line ends in `FAILED`, and
`test result: FAILED. 0 passed; 1 failed`. The edits for M1, M2 and M6 leave
an unused function, an unused helper or a needless `mut` behind, so their
builds warn; that is expected while the mutant stands.

Why each one fails:
- **M1.** `format_with` returns the broken printer's output instead of
  refusing it.
- **M2.** `// first` is left for the next element, which prints it on a line
  of its own, so the output differs from the expected string. The comments
  stay in order, so the self-check passes, which is the point of the test.
- **M3.** No pair of parentheses is kept. `(b * c) + d` comes out as
  `b * c + d`, which parses to the same AST, so only the test sees it.
- **M4.** `f()` and `(a, b).show()` print without the `;`, and re-parse as
  the single call `f()(a, b).show()`. The self-check refuses that.
- **M5.** `return` and `x = 1` print without the `;`, and re-parse as
  `return x = 1`. The self-check refuses that.
- **M6.** The methods print first, then the consts, then the bindings. The
  AST keeps them in three lists either way, so only the test sees it.
- **M7.** The first import's run swallows every item after it and sorts
  them, which moves `fn f` above the imports. The self-check refuses that.

- [ ] **Step 1: Run M1 to M7 as above**

- [ ] **Step 2: Check that everything is restored**

```bash
cd /d/Projects/nona/nova && git status --short && cargo test --locked -p nova-fmt 2>&1 | grep -E "^test result" | grep -vc " 0 failed"
```

Expected: no `git status` output, and `0`: every result line of the
restored crate reports 0 failed.

The task's test command: `cargo test --locked -p nova-fmt`.

---
### Task 11: Format `std/` and `examples/`, and keep them formatted in CI

Spec §8. One mechanical commit formats the 17 files in `std/` and the 6 in
`examples/`, and changes nothing else. The full suite then runs on Windows
and Linux, and CI gains the check. The fixtures are never rewritten.

**Files:**
- Modify: `std/**/*.nova` and `examples/**/*.nova`, through `nova fmt` only
- Modify: `.github/workflows/ci.yml`

**Interfaces:**
- Consumes: Task 9's `nova fmt`.
- Produces: `std/` and `examples/` in `nova fmt`'s layout. Task 12
  re-derives the line-number citations from them.

- [ ] **Step 1: Check that no `.editorconfig` can reach the repository**

```bash
cd /d/Projects/nona/nova && git ls-files | grep -c editorconfig; ls ../.editorconfig ../../.editorconfig ../../../.editorconfig 2>/dev/null; echo "end"
```

Expected: `0`, then `end` alone. An `.editorconfig` above the checkout would
change the line endings `nova fmt` writes here but not on CI's runners: if
one is listed, stop and ask the user.

- [ ] **Step 2: Format**

```bash
cd /d/Projects/nona/nova && cargo run --locked -q -p nova-cli -- fmt std examples; echo "exit=$?"; cargo run --locked -q -p nova-cli -- fmt --check std examples; echo "exit=$?"
```

Expected: `exit=0` twice, with nothing else printed.

- [ ] **Step 3: Read the diff**

```bash
cd /d/Projects/nona/nova && git status --short | grep -v -E "^ M (std|examples)/" ; git diff --stat | tail -1; git ls-files --eol std examples | grep -vc "i/lf"
```

Expected:
- the first command prints nothing: only files in `std/` and `examples/`
  changed;
- a summary line of at most 23 files;
- `0`: the index still holds every file with LF endings. The working tree
  keeps each file's own endings, and Git compares content, so the diff
  shows content alone.

Then read `git diff -- std examples` in full. The self-check guarantees that
the AST and the comments are unchanged, so what to look for is layout the
spec does not intend. What the spec's §2 measurements predict:
- the 12 multi-line std records gain commas, or go onto one line where
  they fit;
- top-level items gain or lose blank lines until exactly one separates each
  pair;
- `std/collections/lib.nova:388`, the one line holding two statements, is
  split in two;
- lines longer than 100 columns break where they can. A long string cannot
  break, so a line holding one may stay long.

Anything else surprising is a finding: write the smallest input showing it
as a test in the owning task's test file, watch it fail, fix the printer,
and format again.

- [ ] **Step 4: The full suite on Windows**

```bash
cd /d/Projects/nona/nova && P=/c/Users/SAKEER~1/AppData/Local/Temp/gcm/p31 && netstat -ano | grep -E "[:.]3000 .*LISTENING"; cargo build --locked -p nova-runtime 2>&1 | tail -1 && cargo test --locked --workspace --all-features --no-fail-fast > $P/suite-reformat.txt 2>&1; echo "exit=$?"
```

If `netstat` prints a line, port 3000 is taken: stop and ask the user, and
never stop that process. Count the run with the conventions' counting line.
Expected: `exit=0`, 0 failed, and 132 more passed than Task 1's ledgered
baseline, with the same ignored count. The 132 are:

| Task | Tests |
|---|---|
| 1 | 8 lexer |
| 2 | 8 parser |
| 3 | 10 renderer |
| 4 | 10 library |
| 5 | 29 layout, 21 source rules |
| 6 | 18 comments, 2 cursor |
| 7 | 9 files, 5 `.editorconfig` |
| 8 | 1 corpus |
| 9 | 11 `nova fmt` |

- [ ] **Step 5: Commit the reformat**

Write `$P/msg-11a.txt`:

```
std, examples: format with nova fmt

The output of `nova fmt std examples`, and nothing else (spec
docs/superpowers/specs/2026-10-07-phase-3-1-formatter-design.md §8).
nova fmt's self-check guarantees that every file's AST and comments are
unchanged.

The parser fixtures, the runtime fixtures and docs/benchmarks/ are not
rewritten; the corpus test checks them in memory.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
```

```bash
cd /d/Projects/nona/nova && P=/c/Users/SAKEER~1/AppData/Local/Temp/gcm/p31 && git add std examples && git commit -q -F $P/msg-11a.txt && git log -1 --format=%s && git show --stat HEAD | grep -v -E "^ (std|examples)/" | tail -3
```

Expected: `std, examples: format with nova fmt`, and the stat lists no file
outside `std/` and `examples/`.

- [ ] **Step 6: The full suite on Linux**

```bash
cd /d/Projects/nona/nova && P=/c/Users/SAKEER~1/AppData/Local/Temp/gcm/p31 && docker version 2>&1 | grep -c "^Server:"; bash /c/Users/SAKEER~1/AppData/Local/Temp/gcm/linux/run.sh test --locked --workspace --all-features --no-fail-fast > $P/suite-linux.txt 2>&1; echo "exit=$?"; bash /c/Users/SAKEER~1/AppData/Local/Temp/gcm/linux/run.sh run --locked -q -p nova-cli -- fmt --check std examples; echo "exit=$?"
```

The first count must be `1`; if it is `0`, start Docker Desktop as the
conventions say. Count the suite with the conventions' counting line.
Expected: both exits `0`, 0 failed, and 132 more passed than the Linux
figure CI reported at `2bf8006` (1285 passed, 9 ignored), with 9 ignored.
The second run checks the LF files the Linux harness exports.

- [ ] **Step 7: Check formatting in CI**

In `.github/workflows/ci.yml`, after the Test job's step

```yaml
      - name: cargo test
        run: cargo test --locked --workspace --all-features --no-fail-fast
```

insert:

```yaml

      # std/ and examples/ stay in nova fmt's layout (spec
      # docs/superpowers/specs/2026-10-07-phase-3-1-formatter-design.md §8).
      # On Windows the checkout may hold CRLF files, which nova fmt keeps.
      - name: nova fmt --check std examples
        run: cargo run --locked -q -p nova-cli -- fmt --check std examples
```

Write `$P/msg-11b.txt`:

```
ci: check that std and examples stay formatted

The Test job on all three systems runs `nova fmt --check std examples`
after the tests (spec
docs/superpowers/specs/2026-10-07-phase-3-1-formatter-design.md §8). On
Windows it checks the line endings the checkout produces, which nova fmt
keeps.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
```

```bash
cd /d/Projects/nona/nova && P=/c/Users/SAKEER~1/AppData/Local/Temp/gcm/p31 && git add .github/workflows/ci.yml && git commit -q -F $P/msg-11b.txt && git log -1 --format=%s
```

Expected: `ci: check that std and examples stay formatted`.

The task's test command: `cargo run --locked -q -p nova-cli -- fmt --check std examples`.

---
### Task 12: The records

Spec §10. ADR 0028, dated notes in the specs, the project documents, the 14
line-number citations re-derived from the reformatted std, and a
set-difference sweep for anything else the change made stale.

**Files:**
- Create: `docs/adr/0028-the-formatter.md`
- Modify: `nova-spec/10-LEXER.md`, `nova-spec/11-PARSER.md`,
  `nova-spec/40-TOOLING.md`, `nova-spec/00-MASTER-SPEC.md`
- Modify: `CHANGELOG.md`, `ARCHITECTURE.md`, `docs/phase-3-plan.md`
- Modify, at the cited lines only: `nova-spec/20-STDLIB.md`,
  `docs/benchmarks/README.md`, `crates/nova-cli/tests/run_tests.rs`,
  `std/sync/lib.nova`
- Modify: whatever the sweep in Step 6 finds

**Interfaces:**
- Consumes: the whole branch, and Task 11's reformatted `std/`.
- Produces: nothing code depends on.

Every note opens with
`**Amended 2026-10-07 (branch `phase-3-1-formatter`):**`. If you execute on
a later day, use that day's date. ADR bodies, dated specs and plans, and the
CHANGELOG's released sections stay as they are.

- [ ] **Step 1: ADR 0028**

Create `docs/adr/0028-the-formatter.md`:

```markdown
# ADR 0028 — The formatter

## Status

Accepted (2026-10-07). Branch `phase-3-1-formatter`
(`docs/superpowers/specs/2026-10-07-phase-3-1-formatter-design.md`;
`docs/phase-3-plan.md` §3 decision 6).

## Context

The master spec's Phase 3 list opens with "`crates/nova-fmt` — formatter (no
options, only `--check`)", and `nova-spec/40-TOOLING.md` §2 describes it:
gofmt's fixed style, built on Wadler's pretty printer. Before 3.1, `nova-fmt`
was a one-line stub, and the front end could not support it:

- the lexer dropped comments, so a formatter printing from the AST would
  have deleted every one;
- `///` lexed as a doc comment that no grammar rule accepted, so no file
  could hold one;
- the AST keeps literal values, not their spellings, and records no
  parentheses, blank lines or comments.

## Decision

1. **Modes.** `nova fmt [PATH]... [--check] [--stdin]`. With no path it
   formats the project's `src/`, found as 3.0's commands find the project;
   paths name files or directories, searched for `*.nova` without `target/`
   or dot-directories. `--check` writes nothing and exits 1 if a file would
   change; `--stdin` formats standard input to standard output. Errors exit
   2 and outrank a would-change. The master spec's "only `--check`" is read
   as "no style options": `--check` and `--stdin` choose what the command
   reads and writes, never how code looks.
2. **Canonical output.** The layout depends only on the code, its comments
   and its blank lines: lines of 100 columns, counted in Unicode scalar
   values, 4-space indentation, one blank line between top-level items, and
   at most one wherever the author left any.
3. **The author's parentheses are kept,** one pair per node, and none are
   added. No precedence model decides where parentheses go.
4. **Doc comments.** `///` documents the item, member, field or variant
   after it, and lands in that node's `docs`; anywhere else it is a parse
   error. `////` is a plain comment. An unterminated `/*` is
   `UnterminatedBlockComment`.
5. **Separators.** Statements print without `;`, and match arms without
   `,`, except where the parser, which ignores line breaks, would read two
   as one: before one that begins with `(`, `[`, `{`, `-`, `*`, `&` or `|`,
   and after a `return` or `break` with no value.
6. **Line endings.** `.editorconfig`'s `end_of_line` if it says `lf` or
   `crlf`, otherwise the file's own, otherwise LF; one final newline unless
   `insert_final_newline = false`. A multi-line string literal holds its
   file's line ending, so it keeps it, and changes with it if the file's
   changes.
7. **The self-check, on every call.** The output is lexed and parsed again,
   and must give the input's AST, ignoring spans and with import runs
   sorted, and the input's comments. Otherwise nothing is returned, and
   `nova fmt` leaves the file unchanged and says so.

## Alternatives

- **A lossless syntax tree.** A parser that keeps every token and comment,
  with the AST derived from it, is the sturdiest base, and 3.2's LSP could
  use it. It is a phase-sized rewrite of the parser that touches every AST
  consumer.
- **Re-indenting tokens by bracket depth.** Simple, and safe for comments,
  but it cannot break lines by width or sort imports, so it misses
  `40-TOOLING.md` §2.1 and §2.2.
- **Removing redundant parentheses.** It needs a precedence model, and the
  author's parentheses often say something.
- **Style options.** `40-TOOLING.md` §2.1 rules them out.

## Consequences

- `std/` and `examples/` are in `nova fmt`'s layout, and CI's Test job
  checks that they stay so on all three systems.
- A file `nova fmt` cannot handle is refused, never damaged. A refusal is a
  formatter bug, and the file is its reproduction.
- The self-check cannot see two things: impl member order, which the AST
  keeps in three lists, and where a comment sits among the tokens. Tests pin
  both.
- A call whose last argument is a closure with two or more statements breaks
  its whole argument list. Hugging the last argument, as rustfmt does, can
  come later; no corpus file needs it.
- `nova-fmt`'s `format_file` and `format_text` are the library entry points
  3.2's LSP will call.

## References

- `docs/superpowers/specs/2026-10-07-phase-3-1-formatter-design.md`, and
  its plan, `docs/superpowers/plans/2026-10-07-phase-3-1-formatter.md`.
- `nova-spec/40-TOOLING.md` §2; `docs/phase-3-plan.md` §3 decision 6 and
  §4's 3.1 entry; ADR 0026, Phase 3's scope.
```

- [ ] **Step 2: Notes in the specs**

`nova-spec/10-LEXER.md`, §2.4, after the line
`Comments (`//`, `/* */`) are skipped, NOT emitted.`:

```markdown

**Amended 2026-10-07 (branch `phase-3-1-formatter`):** a doc comment is a
line comment that starts with exactly three slashes; four or more make a
plain comment, as in Rust. Its token carries the text after `///`, with its
leading whitespace and without its trailing whitespace. `lex` still skips
plain comments. `lex_with_comments` returns the same tokens, and every plain
comment, line or block, with its span, for the formatter
(`docs/superpowers/specs/2026-10-07-phase-3-1-formatter-design.md` §3).
```

`nova-spec/10-LEXER.md`, §6, after the line `- `UnexpectedCharacter(char)``:

```markdown

**Amended 2026-10-07 (branch `phase-3-1-formatter`):**
`UnterminatedBlockComment` is reported, spanning from the `/*` to the end of
the file. Until 3.1 an unterminated `/*` silently commented out the rest of
the file (`docs/superpowers/specs/2026-10-07-phase-3-1-formatter-design.md`
§3.3).
```

`nova-spec/11-PARSER.md`, at the end of §2, after the closing ```` ``` ```` of
the grammar and before the `---` above `## 3. AST Node Definitions (Rust)`:

```markdown

**Amended 2026-10-07 (branch `phase-3-1-formatter`):** three changes to the
grammar above (`docs/superpowers/specs/2026-10-07-phase-3-1-formatter-design.md`
§4):
- **Doc comments.** One or more `///` lines may come before an item, at the
  top level or in a block; a trait member; an impl member; a record field; a
  sum-type variant; or a function in an `extern` block. On a top-level item
  they may come before, between or after its attributes. The node gains
  `docs`, one entry per line. Anywhere else a `///` is an error: "a doc
  comment must come right before an item, a field or a variant; use `//`
  for a plain comment".
- **`where`** takes a trailing comma before the `{`, `;` or `}` that ends
  the clause.
- **Pattern spans.** A parenthesised pattern, and the unit pattern `()`, are
  spanned over their parentheses, as parenthesised expressions and types
  already were.
```

`nova-spec/40-TOOLING.md`, §1.1, after the paragraph of the note that ends
`(`docs/superpowers/specs/2026-10-07-phase-3-0-foundations-design.md` §6).`:

```markdown

**Amended 2026-10-07 (branch `phase-3-1-formatter`):** `nova fmt` exists, as
`nova fmt [PATH]... [--check] [--stdin]`. With no path it formats the
project's `src/`, or `src/` outside a project when `src/main.nova` exists;
paths may name files or directories. It exits 0, 1 when `--check` finds a
file that would change, or 2 on any error. ADR 0028 reads the master spec's
"only `--check`" as "no style options"
(`docs/superpowers/specs/2026-10-07-phase-3-1-formatter-design.md` §7).
```

`nova-spec/40-TOOLING.md`, §2.4, after the line
`Respects `.editorconfig` for line endings and final newline only.`:

```markdown

**Amended 2026-10-07 (branch `phase-3-1-formatter`):** §2.1 to §2.4 as built
(`docs/superpowers/specs/2026-10-07-phase-3-1-formatter-design.md` §5 to §7;
ADR 0028):
- The 100 columns are counted in Unicode scalar values. A construct that
  does not fit breaks in its own way, and a line holding something that
  cannot break, such as a long string, may run past 100.
- Imports are sorted within each run of consecutive `import` items, by path,
  and each `{…}` list by name, in byte order. "std first" waits for packages
  (3.3).
- The author's parentheses are kept, one pair each. Statements print without
  `;`, and match arms without `,`, except where the parser would otherwise
  read two as one.
- Every comment is kept, and every run checks its output: the same AST, the
  same comments, or the file is left unchanged.
- §2.3: paths may also name directories, and there may be several.
- §2.4: `.editorconfig`'s `end_of_line` (`lf` or `crlf`) wins, then the
  file's own line ending, then LF; one final newline unless
  `insert_final_newline = false`. The files are found by walking up to one
  with `root = true`, and EditorConfig's globs are matched, except numeric
  ranges.
```

`nova-spec/00-MASTER-SPEC.md`, Phase 3's list: directly under the line
`1. `crates/nova-fmt` — formatter (no options, only `--check`)`, indented as
the other notes inside its lists are:

```markdown
   **Amended 2026-10-07 (branch `phase-3-1-formatter`):** built in Phase 3.1.
   `nova fmt` also takes paths and `--stdin`, as `40-TOOLING.md` §2.3 asks;
   `docs/adr/0028-the-formatter.md` reads "only `--check`" as "no style
   options".
```

`nova-spec/00-MASTER-SPEC.md`, §5.2: directly under the line
`- Snapshot tests via `insta` for parser, type errors, formatter output`:

```markdown
  **Amended 2026-10-07 (branch `phase-3-1-formatter`):** the formatter's
  tests compare each whole output with an exact string written beside its
  input, rather than an `insta` snapshot, and also check that formatting the
  output again changes nothing
  (`docs/superpowers/plans/2026-10-07-phase-3-1-formatter.md`, decision 14).
```

- [ ] **Step 3: The project documents**

`CHANGELOG.md`, `[Unreleased]`, at the end of `### Added`, after
`- **ADR 0026** records Phase 3's scope.`:

```markdown
- **`nova fmt`** prints Nova source in one fixed layout: the project's
  `src/`, or the files and directories given; `--check` lists each file that
  would change and exits 1; `--stdin` formats standard input.
  - It keeps every comment and the author's parentheses, and each file's
    line endings unless `.editorconfig` says otherwise.
  - It refuses, and leaves the file untouched, any output that would change
    the program or lose a comment: every run parses its own output again.
  - `nova-fmt`'s library offers the same through `format`, `format_file`
    and `format_text`, for 3.2's language server.

  ADR 0028.
- **Doc comments.** `///` before an item, a trait or impl member, a record
  field, a variant or an extern function documents it. Anywhere else it is
  an error.
- `nova_lexer::lex_with_comments` returns the tokens and every comment.
```

`CHANGELOG.md`, `[Unreleased]`, at the end of `### Changed`, after
`- `nova-pm` no longer depends on `serde`, `toml`, `anyhow` or `tracing`.`:

```markdown
- `////` is a plain comment. It used to lex as a doc comment that no file
  could contain.
- An unterminated `/*` is an error, `UnterminatedBlockComment`, instead of
  silently commenting out the rest of the file.
- A `where` clause takes a trailing comma.
- `std/` and `examples/` are formatted with `nova fmt`, and CI checks that
  they stay so.
```

`ARCHITECTURE.md`: replace the row
`| `nova-fmt` | Opinionated code formatter |` with:

```markdown
| `nova-fmt` | The formatter: prints the AST in one fixed layout, keeps every comment, and refuses output that would change the program (ADR 0028) |
```

`docs/phase-3-plan.md`, §4's 3.1 entry: after its gate bullet, which ends
`confirmed that no test depends on their line numbers.`, add:

```markdown
- **Spec:** `docs/superpowers/specs/2026-10-07-phase-3-1-formatter-design.md`,
  built on the branch `phase-3-1-formatter`.
```

- [ ] **Step 4: Commit the records**

Write `$P/msg-12a.txt`:

```
docs: record Phase 3.1, the formatter

- ADR 0028, "The formatter": nova fmt's modes, canonical output, the
  author's parentheses, doc comments, separators, line endings and the
  self-check, with the alternatives and what follows.
- Dated notes: 10-LEXER §2.4 and §6, 11-PARSER §2, 40-TOOLING §1.1 and
  §2.4, and the master spec's Phase 3 list and §5.2.
- CHANGELOG [Unreleased], ARCHITECTURE's nova-fmt row, and the phase
  plan's 3.1 spec line.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
```

```bash
cd /d/Projects/nona/nova && P=/c/Users/SAKEER~1/AppData/Local/Temp/gcm/p31 && git add docs/adr/0028-the-formatter.md nova-spec CHANGELOG.md ARCHITECTURE.md docs/phase-3-plan.md && git commit -q -F $P/msg-12a.txt && git log -1 --format=%s && git show --stat HEAD | tail -1
```

Expected: `docs: record Phase 3.1, the formatter`, and `8 files changed`.

- [ ] **Step 5: The 14 line-number citations**

Re-derive each from the reformatted std (spec §8), by what it describes, not
by mapping old numbers: eight were stale before the reformat. Print where
each target is now:

```bash
cd /d/Projects/nona/nova && for q in "core|pub trait Display" "core|pub fn unwrap(self) -> T" "core|n.unwrap()" "core|can be *negative*" "collections|pub fn get(self, i: Int)" "collections|pub fn set(mut self, i: Int, v: T)" "collections|Vec::set index out of range" "collections|A hash may be negative" "strings|would take the name" "strings|pub fn join" "strings|i = i + s.len()" "strings|j = j + s.len()" "fmt|pub fn pad(self, width: Int)" "fmt|if len >= width { return s }" "http|Both park with no deadline"; do f=${q%%|*}; p=${q#*|}; echo "== std/$f: $p"; grep -n -F -- "$p" std/$f/lib.nova | head -6; done; grep -n -A8 -E "^fn trim_(start|end)_index" std/strings/lib.nova
```

Then edit each citation:

| # | Citing line (before this branch) | Points at | Write |
|---|---|---|---|
| 1 | `nova-spec/20-STDLIB.md:200` | `Display`, `fn fmt(self) -> String` | the line of `pub trait Display` |
| 2 | `nova-spec/20-STDLIB.md:1798` | the comment giving `join`'s reason, and `join` | a range from the first line of the comment directly above `pub fn join` to `pub fn join` |
| 3 | `nova-spec/20-STDLIB.md:2158` | `Vec`'s `get`, and `set` | the lines of `pub fn get(self, i: Int)` and `pub fn set(mut self, i: Int, v: T)`, the first match of each |
| 4 | `nova-spec/20-STDLIB.md:2382` | `if len >= width { return s }`, and `pad`'s signature | those two lines |
| 5 | `docs/benchmarks/README.md:458` | std/http's note that reads have no deadline | the line holding `Both park with no deadline` |
| 6 | `crates/nova-cli/tests/run_tests.rs:2475` | `split`'s pass-1 step, and its pass-2 step | the lines of `i = i + s.len()` and `j = j + s.len()` |
| 7 | `run_tests.rs:2605` | `trim_start_index`'s `cs.len()` fallback | its last line, `cs.len()` |
| 8 | `run_tests.rs:2606` | `trim_end_index`'s `floor` fallback | its last line, `floor` |
| 9 | `run_tests.rs:2889` | `Vec::set`'s `i >= self.len` guard, and its `i < 0` guard ("line 54's") | those two lines |
| 10 | `std/sync/lib.nova:85` | `Option::unwrap` | the first `pub fn unwrap(self) -> T` |
| 11 | `std/sync/lib.nova:86` | the five lines where std/core calls `unwrap`, six calls in all | every line `n.unwrap()` matches; check there are still five, the last holding two calls, and fix the comment's counts if not |
| 12 | `std/sync/lib.nova:145` | as 3 | as 3 |
| 13 | `std/sync/lib.nova:183` | std/core's warning that `hash % cap` can be negative | the line holding `can be *negative*` |
| 14 | `std/sync/lib.nova:184` | std/collections' same warning | the line holding `A hash may be negative` |

The citing lines have moved where the reformat touched their own files;
find each by its text. Change only the numbers, unless a number's sentence
no longer holds. Then:

```bash
cd /d/Projects/nona/nova && cargo run --locked -q -p nova-cli -- fmt --check std examples; echo "exit=$?"; git diff --stat
```

Expected: `exit=0` (comment text is printed as written, so editing it
cannot unformat std), and at most the four files above.

Write `$P/msg-12b.txt`:

```
Point std's 14 line-number citations at the reformatted std

Each is re-derived by what it cites, not mapped from its old number:
eight of the 14 were already stale before the reformat (spec
docs/superpowers/specs/2026-10-07-phase-3-1-formatter-design.md §2, §8).
Dated records keep theirs.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
```

```bash
cd /d/Projects/nona/nova && P=/c/Users/SAKEER~1/AppData/Local/Temp/gcm/p31 && git add nova-spec/20-STDLIB.md docs/benchmarks/README.md crates/nova-cli/tests/run_tests.rs std/sync/lib.nova && git commit -q -F $P/msg-12b.txt && git log -1 --format=%s
```

Expected: `Point std's 14 line-number citations at the reformatted std`.

- [ ] **Step 6: The set-difference sweep**

A grep proves only what it was pointed at. List every living document that
names what this branch changed, minus the files the branch touched:

```bash
cd /d/Projects/nona/nova && P=/c/Users/SAKEER~1/AppData/Local/Temp/gcm/p31 && git diff --name-only 2bf8006 HEAD > $P/touched.txt && for t in "nova fmt" "nova-fmt" "fmt --check" "formatter" "doc comment" "DocComment" "DOC_COMMENT" "UnterminatedBlockComment" "editorconfig" "lex_with_comments" "////" "trailing comma"; do git grep -l -F -- "$t" -- '*.md' '*.nova' '*.yml' '*.toml' '*.sh' | grep -v -x -F -f $P/touched.txt | grep -v -E "^docs/superpowers/|^docs/adr/" | sed "s|^|$t: |"; done | sort | tee $P/sweep.txt | wc -l
```

Read every hit in `$P/sweep.txt`, in context. For each, decide whether it
now says something untrue or incomplete about the formatter, doc comments,
comments in the lexer, `where`, or `std`'s layout:
- if it does, add a dated note there (or, for a non-document such as a
  script, fix it), and add the file to the commit below;
- if not, nothing changes.

Ledger each file as `Task 12: sweep <file> -> <note added | unaffected:
why>`. A file listed for a word it uses in another sense (a `Formatter`
type, Rust's own `cargo fmt --check`) is unaffected.

Then the reformat's own reach: a living document that quotes a std or
example line the reformat changed now quotes a line that no longer exists.
The lines to look for are those the reformat removed and did not put back
anywhere, ignoring indentation:

```bash
cd /d/Projects/nona/nova && P=/c/Users/SAKEER~1/AppData/Local/Temp/gcm/p31 && R=$(git log --format=%h --grep="^std, examples: format with nova fmt" -1) && git show $R -- std examples | grep -E "^[-][^-]" | sed -E 's/^[-][[:space:]]*//' | sort -u > $P/minus.txt && git show $R -- std examples | grep -E "^[+][^+]" | sed -E 's/^[+][[:space:]]*//' | sort -u > $P/plus.txt && comm -23 $P/minus.txt $P/plus.txt | awk 'length($0) >= 24' > $P/removed.txt && wc -l < $P/removed.txt && git grep -n -F -f $P/removed.txt -- '*.md' | grep -v -E "^docs/superpowers/|^docs/adr/|^CHANGELOG.md" | head -40
```

Every hit is a document quoting a line as it was. Update each to the line as
it is now, or ledger why it stands (a quotation of history, say). Lines
shorter than 24 characters are skipped, since `}` and `fn main() {` would
match everywhere.

If the sweep changed anything, write `$P/msg-12c.txt`:

```
docs: notes the formatter sweep found

The set-difference sweep over the living documents (spec
docs/superpowers/specs/2026-10-07-phase-3-1-formatter-design.md §10):
<one line per file changed, saying what changed>

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
```

and commit the files it lists with
`git commit -q -F $P/msg-12c.txt`, then check `git log -1 --format=%s`.

The task's test command: `cargo run --locked -q -p nova-cli -- fmt --check std examples`.

---
### Task 13: Final verification

Everything CI runs, run here first, on the finished branch, plus the gate's
three items by name.

**Files:**
- Create: `$P/pr-body.md` (outside the repository)

**Interfaces:**
- Consumes: the whole branch.
- Produces: the figures for the PR body.

- [ ] **Step 1: The full suite on Windows**

```bash
cd /d/Projects/nona/nova && P=/c/Users/SAKEER~1/AppData/Local/Temp/gcm/p31 && git status --short | wc -l && netstat -ano | grep -E "[:.]3000 .*LISTENING"; cargo build --locked -p nova-runtime 2>&1 | tail -1 && cargo test --locked --workspace --all-features --no-fail-fast > $P/suite-final.txt 2>&1; echo "exit=$?"
```

Expected: `0` uncommitted files, nothing from `netstat` (otherwise stop and
ask the user, as the conventions say), and `exit=0`. Count it with the
conventions' counting line: 0 failed, Task 1's baseline plus 132 passed, the
same ignored count.

- [ ] **Step 2: What CI's other jobs run**

```bash
cd /d/Projects/nona/nova && P=/c/Users/SAKEER~1/AppData/Local/Temp/gcm/p31 && cargo fmt --all -- --check; echo "fmt exit=$?"; cargo clippy --locked --all-targets --all-features -- -D warnings > $P/clippy.txt 2>&1; echo "clippy exit=$?"; tail -3 $P/clippy.txt
```

Expected: `fmt exit=0` and `clippy exit=0`. A clippy finding is fixed in the
code, never allowed by attribute, and the fix is its own commit,
`nova-fmt: <what clippy found>`, after which Step 1 runs again.

```bash
cd /d/Projects/nona/nova && P=/c/Users/SAKEER~1/AppData/Local/Temp/gcm/p31 && RUSTUP_TOOLCHAIN=1.78.0 RUSTFLAGS="-D warnings" cargo check --locked --workspace --target-dir target/msrv > $P/msrv.txt 2>&1; echo "msrv exit=$?"; tail -3 $P/msrv.txt
```

Expected: `msrv exit=0`. A failure here names an API newer than 1.78 (the
plan avoids `std::path::absolute`, `repeat_n` and char-array patterns):
replace it, commit, and run Step 1 again.

- [ ] **Step 3: The full suite on Linux, and the check on LF files**

```bash
cd /d/Projects/nona/nova && P=/c/Users/SAKEER~1/AppData/Local/Temp/gcm/p31 && docker version 2>&1 | grep -c "^Server:"; bash /c/Users/SAKEER~1/AppData/Local/Temp/gcm/linux/run.sh test --locked --workspace --all-features --no-fail-fast > $P/suite-linux-final.txt 2>&1; echo "exit=$?"; bash /c/Users/SAKEER~1/AppData/Local/Temp/gcm/linux/run.sh run --locked -q -p nova-cli -- fmt --check std examples; echo "exit=$?"
```

Expected: `1`, both exits `0`, and with the counting line 0 failed, 1417
passed (1285 + 132) and 9 ignored.

- [ ] **Step 4: The gate, item by item**

`docs/phase-3-plan.md` §4's 3.1 gate, each with the test that shows it:

```bash
cd /d/Projects/nona/nova && P=/c/Users/SAKEER~1/AppData/Local/Temp/gcm/p31 && grep -E "every_corpus_file_formats_to_the_same_program_and_comments|nova_check_accepts_a_doc_comment_before_an_item|check_fails_on_an_unformatted_file_and_passes_once_it_is_formatted" $P/suite-final.txt; cargo run --locked -q -p nova-cli -- fmt --check std examples; echo "exit=$?"
```

Expected: three lines ending in `ok`, and `exit=0`. Ledger them:
- decision 6's three checks on every `.nova` file:
  `every_corpus_file_formats_to_the_same_program_and_comments`;
- a `///` before an item parses:
  `nova_check_accepts_a_doc_comment_before_an_item`;
- `nova fmt --check` exits non-zero on an unformatted file and zero after
  formatting: `check_fails_on_an_unformatted_file_and_passes_once_it_is_formatted`;
- CI runs `nova fmt --check` on `std/` and `examples/`: Task 11's step,
  and no test depends on their line numbers (spec §2).

- [ ] **Step 5: Draft the PR body**

Write `$P/pr-body.md` with the Write tool, filling the `<…>` from the ledger
and from Steps 1 and 3:

```markdown
## Phase 3.1, "Formatter"

`nova fmt` prints Nova source in one fixed layout, keeps every comment, and
refuses any output that would change the program. `std/` and `examples/`
are formatted with it, and CI keeps them that way.

- Spec: `docs/superpowers/specs/2026-10-07-phase-3-1-formatter-design.md`
- Plan: `docs/superpowers/plans/2026-10-07-phase-3-1-formatter.md`
- ADR 0028, "The formatter"

### What changed

- **Lexer:** `lex_with_comments` returns every comment; `///` keeps its
  text; `////` is a plain comment; an unterminated `/*` is
  `UnterminatedBlockComment`.
- **Parser:** `///` attaches to the item, member, field or variant after it
  as `docs`, and is an error anywhere else. A parenthesised pattern spans
  its parentheses, and `where` takes a trailing comma.
- **`nova-fmt`:**
  - a Wadler document printer of its own;
  - an AST printer that reads spellings, parentheses, blank lines and impl
    member order from the source;
  - a comment cursor that keeps comments in order;
  - a self-check on every call;
  - `.editorconfig` for line endings and the final newline.
- **`nova fmt`:** the project, files, directories, `--check` and `--stdin`;
  exit codes 0, 1 and 2.
- **`std/` and `examples/`**, reformatted in one mechanical commit; CI's
  Test job checks them on all three systems.
- **Records:** ADR 0028; dated notes in 10-LEXER, 11-PARSER, 40-TOOLING and
  the master spec; CHANGELOG, ARCHITECTURE and the phase plan; the 14 std
  line-number citations, re-derived.

### The gate (`docs/phase-3-plan.md` §4, 3.1)

- Decision 6's three checks pass on every `.nova` file in the repository:
  `every_corpus_file_formats_to_the_same_program_and_comments`, 167 files,
  with `async.nova` the one that does not parse.
- A `///` before an item parses:
  `nova_check_accepts_a_doc_comment_before_an_item`.
- `nova fmt --check` exits 1 on an unformatted file and 0 after formatting:
  `check_fails_on_an_unformatted_file_and_passes_once_it_is_formatted`.
- CI runs `nova fmt --check std examples`. No test depended on their line
  numbers.

### Tests

| | Windows (local) | Linux (container) |
|---|---|---|
| `2bf8006` | <baseline> | 1285 passed, 9 ignored |
| this branch | <Step 1's figures> | <Step 3's figures> |

132 new tests. Each of the spec's seven mutants fails its named test.
Clippy, rustfmt and the MSRV check pass.

### Decisions to review

The plan's "Where this plan settles what the spec leaves open", items 1 to
14, and the rulings made while executing it:

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

1. **The final review.** A fresh reviewer, on the most capable model,
   reads the whole branch (`git merge-base main HEAD` to `HEAD`), with the
   spec, this plan, its Review Focus section verbatim, and the ledger's
   `Ruling:` lines. It is read-only, and runs no cargo and no scripts.
2. **The fix pass.** Each Critical or Important finding gets a test that
   fails first, then the fix, then a green suite. Minor findings are
   deferred to the PR body.
3. **Push and open the PR** against `main`, with `$P/pr-body.md` as its
   body. Then read the PR's CI once when the user returns; never poll it.
4. **Merge only on the user's word,** by rebase, and verify that `main`'s
   tree is the branch's tree. Nothing is published, and no tag is pushed.
