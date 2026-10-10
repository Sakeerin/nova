# Phase 3.4a, "Navigation", Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** `nova lsp` answers hover, go to definition, find references and
rename, from an index of every name the type checker resolves. Rename
checks itself by analysing the renamed program again.

**Architecture:**
- **`nova-resolver` owns the index's types** (`src/index.rs`): `Index`,
  `Occurrence`, `Role`, `Target` and their lookups. The resolver records
  the occurrences of import items into `ProgramResolution::imports`.
- **`nova-typeck` records everything else** when `CheckOptions::index` is
  on: declarations, uses, the types of locals, and which impl methods
  implement which trait methods. It writes the index and never reads it.
- **`nova-driver`** gains `Options::index` and `Analysis::index`, merging
  the resolver's import occurrences in.
- **`nova-lsp`** gains:
  - `analysis.rs`, which picks the analysis that answers a request
    (shared with completion);
  - `std_cache.rs`, std's sources on disk;
  - `hover.rs`, which reads declarations and docs from source;
  - `navigate.rs`, for definition and references;
  - `rename.rs`, for prepare rename, rename, and the check.
- **The VS Code extension's smoke test** gains hover and definition.

**Tech Stack:**
- Rust: MSRV 1.78, edition 2021.
- No new crate. `nova-lsp` gains the workspace's `crc32fast`, already
  locked.
- `lsp-server` 0.7.8 and `lsp-types` 0.97, already locked.
- Tests:
  - `nova-driver` integration tests over in-memory buffers;
  - `nova lsp` driven over stdio, through `crates/nova-cli/tests/lsp_client`;
  - the extension's `vscode-test` smoke test in CI.

**Spec:** `docs/superpowers/specs/2026-10-10-phase-3-4a-navigation-design.md`
(commits `894698d` and `58a7ee4`), approved by the user on 2026-10-10.
- Read it before Task 1: it is the authority this plan argues from.
- Its §12 lists 31 decisions, and its §2 the code as it stood.

## Global Constraints

- **The workspace's minimum Rust stays 1.78,** edition 2021. CI's MSRV job
  runs `cargo check --locked --workspace` with `RUSTFLAGS=-D warnings`.
- **No new crate in `Cargo.lock`.** `nova-lsp`'s new `crc32fast` edge is
  the only lockfile change.
- **The CLI is unchanged.** `nova check`, `build`, `run`, `test` and the
  server's diagnostics never turn the index on, and every existing test
  passes unchanged.
- **The checker never reads the index.** Recording is write-only, so a
  check with the index on gives the same diagnostics and HIR as one
  without.
- **Recording happens only at the AST's name sites**, each with the name's
  own span, deduplicated by (span, role, target) (spec decision 28). Never
  record inside `FnCtx::new_local`, `FnCtx::lookup`, `place_root` or an
  `emit_*` function.
- **Refusals** are `RequestFailed` (-32803) errors with the spec's messages
  (§5.4, §5.5). "No name here" is a `null` result.
- **Budgets** (spec §8.6): 200 ms for hover, definition and references;
  400 ms for rename; CI 2 s and 4 s. If the index pushes completion or
  diagnostics past ADR 0029's 200 ms, stop and ask the user.
- **Tests never touch the internet,** and every test that needs
  `$NOVA_HOME` sets it to a fresh directory.
- **Nothing is published, and no tag is pushed.** Merge by rebase, on the
  user's word only.
- **Downloading VS Code** to run the extension's test locally waits for the
  user's word. CI's run is otherwise the first (spec §8.7).

## Review Focus

The five inputs most likely to bite a user that the spec's tests leave
uncovered, most likely first. Each has its test in the task that owns the
code:

1. **A buffer with unsaved edits above the name.** Renaming from an open
   buffer that differs from the file on disk must edit the buffer's text
   at the buffer's offsets →
   `rename_edits_an_unsaved_buffer_at_its_own_offsets` (Task 10).
2. **A line with Thai and an emoji before the name.** Hover's range, the
   definition's range and rename's edits must be in UTF-16 columns →
   `ranges_count_utf16_after_thai_and_an_emoji` (Task 8).
3. **A CRLF file.** Definition and rename on a CRLF document must land on
   the right characters →
   `rename_in_a_crlf_document_edits_the_right_characters` (Task 10).
4. **A function used from `tests/`.** Renaming a library function must
   rename its uses in the project's `tests/` files, which are part of the
   owning project → `rename_reaches_the_projects_tests_files` (Task 10).
5. **A cursor between two names with no space,** as in `a+b`: just after
   `a` it must find `a`, and just before `b` it must find `b` →
   `a_cursor_between_two_names_finds_the_one_it_touches` (Task 1).

## Decisions: where this plan settles what the spec leaves open

1. **The interfaces.**
   - **`nova-resolver`:** `index::{Index, Occurrence, Role, Target}`, with
     `Index::{record, mark_shorthand, has, implement, extend, at, of,
     declaration, family}`; `ProgramResolution::imports`.
   - **`nova-typeck`:**
     - `CheckOptions::index`, `CheckResult::index`;
     - `display_ty_named` and `builtin_text`.
   - **`nova-diagnostics`:** `FileDb::id_of`.
   - **`nova-driver`:** `Options::index`, `Analysis::index`.
   - **`nova-lsp`:**
     - `analysis::{Scope, Answer, options, analyse, answering}`;
     - `std_cache::{dir, ensure, module_of, path_of, in_std_cache}`;
     - `hover::hover`;
     - `navigate::{definition, references}`;
     - `rename::{prepare, rename}`.
2. **The resolver always records import occurrences.** An import
   records a few occurrences, so `ProgramResolution::imports` is always
   filled. The driver merges it only when `Options::index` is on, so the
   CLI's output is unchanged.
3. **Deduplication uses a set.** `Index` keeps a private
   `HashSet<(Span, Role, Target)>`. std alone yields tens of thousands of
   occurrences, so a linear scan per record would be quadratic.
4. **Type parameters are found through the item being checked.**
   - `Checker::type_params` holds the current item's type parameters,
     names and declaration spans: the impl's first, then the method's.
   - Each item entry point sets it with `enter_type_params`, which also
     records their declarations.
   - A type-parameter use takes the declaration of the same name from that
     list. Names are unique within one item: duplicates are E0403, and a
     method's parameter may not shadow its impl's.
5. **`Index::types` holds locals only,** keyed by declaration span. A
   type parameter has no type: hover reads its bounds from source
   (Decision 6), which is the spec §3.1 comment's "type parameter's type".
6. **Hover reads declarations and docs from the source text,** not the
   AST. The analysis keeps no AST, and the source is the one place the
   spec's "as written" forms exist. Docs are the `///` lines directly
   above the declaration's line, skipping attribute lines (`@…`).
7. **`Index::at` takes `&Definitions`** to rank the occurrences that share
   a span: the value's, then the type's, then the trait's (spec decision
   16).
8. **Locations go through `nova_pm::real_path`,** so a dependency read as
   `app/../geom/src/lib.nova` is reported at its real path. An open
   document keeps its client URI.
9. **Completion shares `analysis::answering`.** `completion::analysis_at`
   moves there and gains the std-cache rule. Completion's behaviour is
   otherwise unchanged, and its tests guard it.
10. **The rename check compares spans by file name, not `FileId`.** The
    re-analysis numbers files again, so the two analyses' spans are
    compared by `(file name, start, end)`.
11. **The new tests' file** is `crates/nova-cli/tests/lsp_navigation.rs`.
    It copies the helpers it shares with `lsp.rs`: `open`, `project`,
    `MANIFEST`, `APP_MAIN`, `app_and_library`, and 3.3b's `registry_app`
    and `lock_and_cache`. `lsp_client::decode` becomes public for it. The
    latency measurement stays in `lsp.rs`.
12. **The completeness checks live in `crates/nova-driver/tests/index_complete.rs`.**
    Each program is analysed with `keep_going`, `tests`, `module_only`
    and `index` on.
13. **Std's cache test runs two servers at once** with one `NOVA_HOME`.
    That is two real processes, as spec §8.4 asks.
14. **The in-memory program for a request inside std's cache** is
    `fn main() {}` at `<temp>/nova-lsp-std/main.nova`, which exists only
    in the overlay (`Overlay::with`).
15. **A family member's ownership** is its declaration's: a file named
    `<std/…>` is std's; a module whose package is `Some(PackageId(k))`,
    `k > 0`, belongs to dependency `k`; anything else in the analysis's
    modules is the project's own.
16. **Results are sorted where they are produced.** `Index` keeps its
    occurrences in recording order. `references`, rename's edits and the
    rename check sort by (file name, offset), the spec's one order.

## File Structure

| File | Task | Responsibility |
|---|---|---|
| `crates/nova-resolver/src/index.rs` (new), `src/lib.rs` | 1 | The index's types and lookups |
| `crates/nova-resolver/src/lib.rs` | 2 | Import occurrences in `resolve_program` |
| `crates/nova-typeck/src/{lib,check}.rs`, `crates/nova-driver/src/analyze.rs`, `crates/nova-driver/tests/index.rs` (new) | 3 | Recording infrastructure, declarations, locals and their types; the driver's plumbing |
| `crates/nova-typeck/src/check.rs`, `crates/nova-driver/tests/index.rs` | 4 | Types, bounds, projections, impl headers |
| `crates/nova-typeck/src/check.rs`, `crates/nova-driver/tests/index.rs` | 5 | Values, calls, records, fields, methods, patterns, families |
| `crates/nova-driver/tests/index_complete.rs` (new), `tests/broken.rs`; `crates/nova-diagnostics/src/files.rs` | 6 | The completeness checks; broken programs with the index on; `FileDb::id_of` |
| `crates/nova-lsp/src/{lib,analysis,completion,hover,workspace}.rs`, `crates/nova-typeck/src/{lib,check}.rs`, `crates/nova-cli/tests/lsp_navigation.rs` (new) | 7 | Which analysis answers; hover |
| `crates/nova-lsp/src/{std_cache,navigate,checker,lib}.rs`, `Cargo.toml`, `Cargo.lock`, `crates/nova-cli/tests/lsp_client/mod.rs` | 8 | Std's cache; definition |
| `crates/nova-lsp/src/{navigate,lib}.rs` | 9 | References |
| `crates/nova-lsp/src/{rename,lib}.rs` | 10 | Prepare rename and rename |
| `crates/nova-lsp/src/rename.rs` | 11 | The rename check |
| `crates/nova-cli/tests/{lsp_navigation,lsp}.rs` | 12 | The gate test; latency |
| `tools/vscode-nova/test/{fixture/src,suite}/…`, `README.md` | 13 | The extension |
| `docs/adr/0032-navigation-in-the-language-server.md` (new), and the records Task 14 lists | 14 | ADR, notes, CHANGELOG, README, ARCHITECTURE, sweep |
| `$P/pr-body.md`, outside the repository | 15 | Final verification, mutants, the PR body |

## Conventions for every task

- **Working directory.** `D:\Projects\nona\nova`, which is Git Bash
  `/d/Projects/nona/nova`. The Bash tool resets its directory after each
  call, so write `cd /d/Projects/nona/nova && …` in one command.
- **Scratch directory.** `P=/c/Users/SAKEER~1/AppData/Local/Temp/gcm/p35`,
  outside the repository. Copy the helpers `count.py`, `extract.py`,
  `insert_after.py`, `replace_once.py` and `append.py` from `…/p34` before
  Task 1. Long output goes to a file there; read its tail.
- **Extract a task's code from its brief** with `extract.py BRIEF MARKER
  OUT`. It writes the first fenced block after the line holding MARKER,
  dedented. Never retype plan code.
- **Counting a full run:** `python -X utf8 $P/count.py <FILE>`. It prints
  `N result lines: P passed, F failed, I ignored`.
- **Line endings.** The working tree is mostly CRLF (`core.autocrlf=true`),
  and the index LF.
  - A Python edit keeps the file's own line ending.
  - New files may be LF.
  - Test strings that need CRLF write `\r\n` explicitly.
- **Write scripts and commit messages with the Write tool, never a Bash
  heredoc.** The Bash tool turns `\\` into `\`, and this plan's Rust has
  backslashes. Commit with `git commit -F $P/msg-<n>.txt`, then check
  `git log -1 --format=%s`.
- **Format before every commit:** run `cargo fmt --all`, then stage what it
  changed.
- **Chain a commit and what follows it with `&&`, never `;`.**
- **Port 3000 must be free for a full Windows suite.**
  - Check with `netstat -ano | grep -E "[:.]3000 .*LISTENING"`.
  - **Never stop a process that holds it: stop and ask the user.**
- **Linux runs** use `bash /c/Users/SAKEER~1/AppData/Local/Temp/gcm/linux/run.sh <cargo args>`.
  It exports tracked files, so `git add` new files first. Docker must be
  running; if it is not, the Linux run is CI's, with a ruling.
- **Mutants run only on committed work,** and are undone with `git checkout
  -- <file>`.
- **Expected outputs are exact.** If a step's output differs:
  - when the code does what this plan describes and the expected value is
    wrong, correct the value and ledger a ruling;
  - when the code does something else, fix the code.
- **A test written after its code, on purpose,** is marked *guard* in its
  comment, and is expected to pass on its first run.
- **Anchors.** This plan places each insertion by the code around it, as
  it stood at `58a7ee4`. If an anchor differs, follow the code's meaning
  and ledger the difference.

---

### Task 1: The index's types and lookups

**Files:**
- Create: `crates/nova-resolver/src/index.rs`
- Modify: `crates/nova-resolver/src/lib.rs` (the module list and re-exports at the top)

**Interfaces:**
- Consumes: `DefId`, `DefKind`, `Definitions`, `ModuleId`, `Builtin`
  (`nova-resolver`); `FileId`, `Span` (`nova-diagnostics`).
- Produces (re-exported from `nova_resolver`):
  - `pub enum Role { Declaration, Use }`, `Copy + Eq + Hash`;
  - `pub enum Target { Def(DefId), Variant(DefId, u32), Field(DefId, u32),
    TraitMethod(DefId, u32), Local(Span), TypeParam(Span), Module(ModuleId),
    Builtin(Builtin), BuiltinMethod(&'static str), Primitive(&'static str) }`,
    `Copy + Eq + Hash`;
  - `pub struct Occurrence { pub span: Span, pub role: Role, pub target:
    Target, pub shorthand: bool }`;
  - `pub struct Index { pub occurrences: Vec<Occurrence>, pub implements:
    Vec<(DefId, (DefId, u32))>, pub types: HashMap<Span, String>, .. }`,
    `Default + Clone + Debug`;
  - `Index::record(&mut self, span: Span, role: Role, target: Target)`;
  - `Index::mark_shorthand(&mut self, span: Span)`;
  - `Index::has(&self, span: Span, role: Role, target: Target) -> bool`;
  - `Index::implement(&mut self, method: DefId, trait_method: (DefId, u32))`;
  - `Index::extend(&mut self, occurrences: impl IntoIterator<Item = Occurrence>)`;
  - `Index::at(&self, defs: &Definitions, file: FileId, offset: u32) -> Option<&Occurrence>`;
  - `Index::of(&self, target: &Target) -> Vec<&Occurrence>`;
  - `Index::declaration(&self, target: &Target) -> Option<&Occurrence>`;
  - `Index::family(&self, target: &Target) -> Vec<Target>`.

- [ ] **Step 1: Write the failing tests**

Create `crates/nova-resolver/src/index.rs` holding only the tests, so they
fail to compile against the missing types:

```rust
//! The index of name occurrences the language server reads (spec
//! `docs/superpowers/specs/2026-10-10-phase-3-4a-navigation-design.md` §3).

#[cfg(test)]
mod tests {
    use super::*;
    use nova_diagnostics::FileDb;

    fn files() -> (FileId, FileId) {
        let mut db = FileDb::new();
        (db.add("a.nova", "a"), db.add("b.nova", "b"))
    }

    fn span(file: FileId, start: u32, end: u32) -> Span {
        Span::new(start, end, file)
    }

    #[test]
    fn a_repeated_record_is_kept_once() {
        let (a, _) = files();
        let mut index = Index::default();
        let s = span(a, 4, 9);
        index.record(s, Role::Use, Target::Local(span(a, 0, 1)));
        index.record(s, Role::Use, Target::Local(span(a, 0, 1)));
        index.record(s, Role::Declaration, Target::Local(span(a, 0, 1)));
        assert_eq!(index.occurrences.len(), 2);
        assert!(index.has(s, Role::Use, Target::Local(span(a, 0, 1))));
    }

    #[test]
    fn a_cursor_inside_or_at_the_end_of_a_name_finds_it() {
        let (a, b) = files();
        let defs = Definitions::default();
        let mut index = Index::default();
        let target = Target::Local(span(a, 0, 5));
        index.record(span(a, 10, 15), Role::Use, target);
        for offset in [10, 12, 14, 15] {
            let found = index.at(&defs, a, offset).map(|o| o.span);
            assert_eq!(found, Some(span(a, 10, 15)), "offset {offset}");
        }
        assert_eq!(index.at(&defs, a, 9).map(|o| o.span), None);
        assert_eq!(index.at(&defs, a, 16).map(|o| o.span), None);
        assert_eq!(index.at(&defs, b, 12).map(|o| o.span), None);
    }

    #[test]
    fn a_cursor_between_two_names_finds_the_one_it_touches() {
        // Review Focus 5: `a+b`. Just after `a` is `a`'s end; just before
        // `b` is inside `b`, which wins over an end.
        let (f, _) = files();
        let defs = Definitions::default();
        let mut index = Index::default();
        let a = Target::Local(span(f, 100, 101));
        let b = Target::Local(span(f, 200, 201));
        index.record(span(f, 0, 1), Role::Use, a);
        index.record(span(f, 2, 3), Role::Use, b);
        assert_eq!(index.at(&defs, f, 1).map(|o| o.target), Some(a));
        assert_eq!(index.at(&defs, f, 2).map(|o| o.target), Some(b));
        // `a.b`'s shape: `a` ends where the `.` starts.
        index.record(span(f, 10, 11), Role::Use, a);
        index.record(span(f, 12, 13), Role::Use, b);
        assert_eq!(index.at(&defs, f, 11).map(|o| o.target), Some(a));
        assert_eq!(index.at(&defs, f, 12).map(|o| o.target), Some(b));
    }

    #[test]
    fn at_a_shorthand_the_value_wins() {
        let (f, _) = files();
        let defs = Definitions::default();
        let mut index = Index::default();
        let x = span(f, 20, 21);
        let field = Target::Field(DefId(3), 0);
        let local = Target::Local(span(f, 2, 3));
        index.record(x, Role::Use, field);
        index.record(x, Role::Use, local);
        index.mark_shorthand(x);
        let found = index.at(&defs, f, 20).unwrap();
        assert_eq!(found.target, local);
        assert!(index.occurrences.iter().all(|o| o.shorthand));
    }

    #[test]
    fn of_and_declaration_find_a_targets_occurrences() {
        let (f, _) = files();
        let mut index = Index::default();
        let t = Target::Def(DefId(7));
        index.record(span(f, 0, 3), Role::Declaration, t);
        index.record(span(f, 10, 13), Role::Use, t);
        index.record(span(f, 20, 23), Role::Use, Target::Def(DefId(8)));
        assert_eq!(index.of(&t).len(), 2);
        assert_eq!(index.declaration(&t).map(|o| o.span), Some(span(f, 0, 3)));
        assert!(index.declaration(&Target::Def(DefId(8))).is_none());
    }

    #[test]
    fn a_family_holds_a_trait_method_and_its_impls() {
        let mut index = Index::default();
        index.implement(DefId(10), (DefId(2), 1));
        index.implement(DefId(11), (DefId(2), 1));
        index.implement(DefId(11), (DefId(2), 1));
        index.implement(DefId(12), (DefId(2), 0));
        let family = vec![
            Target::TraitMethod(DefId(2), 1),
            Target::Def(DefId(10)),
            Target::Def(DefId(11)),
        ];
        assert_eq!(index.family(&Target::TraitMethod(DefId(2), 1)), family);
        assert_eq!(index.family(&Target::Def(DefId(11))), family);
        assert_eq!(index.implements.len(), 3);
        assert_eq!(
            index.family(&Target::Def(DefId(99))),
            vec![Target::Def(DefId(99))]
        );
    }

    #[test]
    fn extend_records_through_the_set() {
        let (f, _) = files();
        let mut index = Index::default();
        let o = Occurrence {
            span: span(f, 0, 1),
            role: Role::Use,
            target: Target::Module(ModuleId(1)),
            shorthand: false,
        };
        index.extend([o.clone(), o]);
        assert_eq!(index.occurrences.len(), 1);
    }
}
```

In `crates/nova-resolver/src/lib.rs`, after the `use` lines at the top
(after `use rustc_hash::FxHashMap;`), add:

```rust
pub mod index;
pub use index::{Index, Occurrence, Role, Target};
```

- [ ] **Step 2: Run the tests to see them fail**

Run: `cd /d/Projects/nona/nova && cargo test -p nova-resolver --lib index:: 2>&1 | tail -20`
Expected: compile errors, among them `cannot find type `Index` in this scope`.

- [ ] **Step 3: Write the types and lookups**

In `crates/nova-resolver/src/index.rs`, between the module comment and
`#[cfg(test)]`, insert:

```rust
//!
//! The resolver records imports into it and the type checker everything
//! else. Neither reads it: it is written for the server alone.

use std::collections::{HashMap, HashSet};

use nova_diagnostics::{FileId, Span};

use crate::{Builtin, DefId, DefKind, Definitions, ModuleId};

/// Whether an occurrence declares its target or uses it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Role {
    Declaration,
    Use,
}

/// What a name means (spec §3.1).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Target {
    /// A function, extern function, const, record, sum type, trait, impl
    /// method or associated type.
    Def(DefId),
    Variant(DefId, u32),
    Field(DefId, u32),
    /// A trait's method, by its index in the checker's list of the trait's
    /// methods.
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

/// One name in the source.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Occurrence {
    /// The name's own token, not the expression around it.
    pub span: Span,
    pub role: Role,
    pub target: Target,
    /// A shorthand field `{ x }`, which is a field and a value at once.
    pub shorthand: bool,
}

/// Every name an analysis resolved (spec §3.1).
#[derive(Debug, Clone, Default)]
pub struct Index {
    pub occurrences: Vec<Occurrence>,
    /// Each impl method that implements a trait's method.
    pub implements: Vec<(DefId, (DefId, u32))>,
    /// Each local's type as hover shows it, by declaration span.
    pub types: HashMap<Span, String>,
    /// What `occurrences` holds, so a record is kept once (decision 28).
    seen: HashSet<(Span, Role, Target)>,
}

impl Index {
    /// Record an occurrence, unless the same one is already recorded.
    pub fn record(&mut self, span: Span, role: Role, target: Target) {
        if self.seen.insert((span, role, target)) {
            self.occurrences.push(Occurrence {
                span,
                role,
                target,
                shorthand: false,
            });
        }
    }

    /// Mark every occurrence at `span` as a shorthand field's.
    pub fn mark_shorthand(&mut self, span: Span) {
        for o in self.occurrences.iter_mut().filter(|o| o.span == span) {
            o.shorthand = true;
        }
    }

    pub fn has(&self, span: Span, role: Role, target: Target) -> bool {
        self.seen.contains(&(span, role, target))
    }

    /// Record that `method`, an impl's, implements `trait_method`.
    pub fn implement(&mut self, method: DefId, trait_method: (DefId, u32)) {
        if !self.implements.contains(&(method, trait_method)) {
            self.implements.push((method, trait_method));
        }
    }

    /// Record each of `occurrences`, as [`Index::record`] does.
    pub fn extend(&mut self, occurrences: impl IntoIterator<Item = Occurrence>) {
        for o in occurrences {
            self.record(o.span, o.role, o.target);
            if o.shorthand {
                self.mark_shorthand(o.span);
            }
        }
    }

    /// The occurrence at byte `offset` of `file`: one whose span holds it,
    /// or else one that ends there (spec §3.1). On a shared span the
    /// value's wins, then the type's, then the trait's, then a field's.
    pub fn at(&self, defs: &Definitions, file: FileId, offset: u32) -> Option<&Occurrence> {
        let holding: Vec<&Occurrence> = self
            .occurrences
            .iter()
            .filter(|o| o.span.file == file && o.span.start <= offset && offset < o.span.end)
            .collect();
        let found = if holding.is_empty() {
            self.occurrences
                .iter()
                .filter(|o| o.span.file == file && o.span.end == offset)
                .collect()
        } else {
            holding
        };
        found.into_iter().min_by_key(|o| rank(defs, &o.target))
    }

    /// Every occurrence of `target`, in recording order.
    pub fn of(&self, target: &Target) -> Vec<&Occurrence> {
        self.occurrences
            .iter()
            .filter(|o| &o.target == target)
            .collect()
    }

    pub fn declaration(&self, target: &Target) -> Option<&Occurrence> {
        self.occurrences
            .iter()
            .find(|o| &o.target == target && o.role == Role::Declaration)
    }

    /// `target`'s family (spec §3.1): a trait method and the impl methods
    /// that implement it, the trait's first; any other target alone.
    pub fn family(&self, target: &Target) -> Vec<Target> {
        let trait_method = match target {
            Target::TraitMethod(t, i) => Some((*t, *i)),
            Target::Def(m) => self
                .implements
                .iter()
                .find(|(d, _)| d == m)
                .map(|(_, tm)| *tm),
            _ => None,
        };
        let Some((t, i)) = trait_method else {
            return vec![*target];
        };
        let mut out = vec![Target::TraitMethod(t, i)];
        for (d, tm) in &self.implements {
            if *tm == (t, i) {
                out.push(Target::Def(*d));
            }
        }
        out
    }
}

/// Which of several occurrences on one span a request means: lower first.
fn rank(defs: &Definitions, target: &Target) -> u8 {
    match target {
        Target::Field(..) => 3,
        Target::Def(id) => match defs.defs().get(id.0 as usize).map(|d| &d.kind) {
            Some(DefKind::Record { .. } | DefKind::Sum { .. } | DefKind::AssocType { .. }) => 1,
            Some(DefKind::Trait { .. }) => 2,
            _ => 0,
        },
        Target::TypeParam(_) | Target::Primitive(_) => 1,
        _ => 0,
    }
}

```

(`Iterator::min_by_key` returns the first of equal minimums, so ties go
to the occurrence recorded first.)

- [ ] **Step 4: Run the tests to see them pass**

Run: `cd /d/Projects/nona/nova && cargo test -p nova-resolver --lib index:: 2>&1 | tail -15`
Expected: `test result: ok. 7 passed; 0 failed`.

- [ ] **Step 5: Commit**

Write `$P/msg-1.txt`:

```text
nova-resolver: the index of name occurrences

The types the language server's navigation reads (spec 3.4a §3.1):
occurrences with their role and target, which impl methods implement
which trait methods, and the types of locals. A record is kept once,
and `at` prefers a name that holds the cursor, then one that ends at
it, and on a shared span the value's.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
```

Run: `cd /d/Projects/nona/nova && cargo fmt --all && git add crates/nova-resolver/src/index.rs crates/nova-resolver/src/lib.rs && git commit -q -F $P/msg-1.txt && git log -1 --format=%s`
Expected: `nova-resolver: the index of name occurrences`

---

### Task 2: The resolver records imports

**Files:**
- Modify: `crates/nova-resolver/src/lib.rs`:
  - `ProgramResolution` (`:1495-1500`);
  - `resolve_program` (`:1551-1700`);
  - `resolve_import` (`:2317-2442`);
  - the tests module.

**Interfaces:**
- Consumes: Task 1's `Occurrence`, `Role`, `Target`.
- Produces: `ProgramResolution::imports: Vec<Occurrence>`, always filled:
  - a `Use` of `Target::Module(ModuleId(target))` at the import path's
    first segment, for every import whose module resolves;
  - for each name of an import list, a `Use` per namespace it binds:
    `Def(id)` for a value `Res::Def` or a type or trait, `Variant(sum, i)`
    for a variant, and `Builtin(b)` for a builtin.

- [ ] **Step 1: Write the failing tests**

At the end of the `mod tests` block of `crates/nova-resolver/src/lib.rs`
(before its closing `}`), add:

```rust
    // === Phase 3.4a: import occurrences (spec
    // docs/superpowers/specs/2026-10-10-phase-3-4a-navigation-design.md §3.3) ===

    fn occurrence_at(
        prog: &ProgramResolution,
        src: &str,
        needle: &str,
    ) -> Vec<(Role, Target)> {
        let start = src.find(needle).unwrap() as u32;
        prog.imports
            .iter()
            .filter(|o| o.span.start == start && o.span.end == start + needle.len() as u32)
            .map(|o| (o.role, o.target))
            .collect()
    }

    #[test]
    fn an_imports_module_is_a_use_of_it() {
        let main = "import lib\nfn main() { let a = area() }\n";
        let prog = resolve_two(main, "pub fn area() -> Int { 1 }\n");
        assert_eq!(
            occurrence_at(&prog, main, "lib"),
            vec![(Role::Use, Target::Module(ModuleId(1)))]
        );
    }

    #[test]
    fn a_list_imports_module_is_its_first_segment() {
        let main = "import lib::{area}\nfn main() { let a = area() }\n";
        let prog = resolve_two(main, "pub fn area() -> Int { 1 }\n");
        // `lib` alone, not `lib::`.
        assert_eq!(
            occurrence_at(&prog, main, "lib"),
            vec![(Role::Use, Target::Module(ModuleId(1)))]
        );
    }

    #[test]
    fn a_listed_name_is_a_use_of_each_namespace_it_binds() {
        // User modules' defs come before std's, so `position` finds these.
        let main = "import lib::{area, Figure, Paint, Round}\nfn main() {}\n";
        let lib = "pub fn area() -> Int { 1 }\n\
                   pub type Figure = Round(Int) | Square(Int)\n\
                   pub trait Paint { fn paint(self) -> String }\n";
        let prog = resolve_two(main, lib);
        let id = |name: &str| {
            DefId(
                prog.definitions
                    .defs()
                    .iter()
                    .position(|d| d.name == name)
                    .unwrap() as u32,
            )
        };
        assert_eq!(
            occurrence_at(&prog, main, "area"),
            vec![(Role::Use, Target::Def(id("area")))]
        );
        assert_eq!(
            occurrence_at(&prog, main, "Figure"),
            vec![(Role::Use, Target::Def(id("Figure")))]
        );
        assert_eq!(
            occurrence_at(&prog, main, "Paint"),
            vec![(Role::Use, Target::Def(id("Paint")))]
        );
        assert_eq!(
            occurrence_at(&prog, main, "Round"),
            vec![(Role::Use, Target::Variant(id("Figure"), 0))]
        );
    }

    #[test]
    fn an_unresolved_import_records_nothing() {
        let main = "import nowhere\nimport lib::{missing}\nfn main() {}\n";
        let prog = resolve_two(main, "pub fn area() -> Int { 1 }\n");
        assert!(occurrence_at(&prog, main, "nowhere").is_empty());
        assert!(occurrence_at(&prog, main, "missing").is_empty());
        // The module of a list import still resolves.
        assert_eq!(occurrence_at(&prog, main, "lib").len(), 1);
    }
```

(`parse_file` gives every span `FileId::DUMMY`, so a span is matched by
its offsets alone.)

- [ ] **Step 2: Run the tests to see them fail**

Run: `cd /d/Projects/nona/nova && cargo test -p nova-resolver --lib import 2>&1 | tail -15`
Expected: a compile error, `no field `imports` on type `ProgramResolution``.

- [ ] **Step 3: Record the occurrences**

In `ProgramResolution`, after its `pub tests: Vec<TestFn>,` field, add:

```rust
    /// The occurrences of import items, for the language server's index
    /// (spec 3.4a §3.3). Always filled; the driver merges them only when
    /// asked.
    pub imports: Vec<Occurrence>,
```

In `resolve_program`, after `let mut tests = Vec::new();` (`:1559`), add:

```rust
    let mut imports: Vec<Occurrence> = Vec::new();
```

In its pass 2, change the `resolve_import(` call to pass `&mut imports`
after `&mut diagnostics`:

```rust
                resolve_import(
                    &mut definitions,
                    &exports,
                    &m.imports,
                    mid,
                    imp,
                    &mut diagnostics,
                    &mut imports,
                );
```

and in the `ProgramResolution { … }` it returns, after `tests,`, add
`imports,`.

In `fn resolve_import(`, add the parameter after `diagnostics: &mut
Vec<Diagnostic>,`:

```rust
    imports: &mut Vec<Occurrence>,
```

After the self-import check (the `if target == mid { … return; }` block),
before `match &imp.kind {`, add:

```rust
    // Spec 3.4a §3.3: the module, at its own segment. Only one-segment
    // paths reach here, and `path.span` would also cover a list's `::`.
    if let Some(first) = imp.path.value.segments.first() {
        imports.push(Occurrence {
            span: first.span,
            role: Role::Use,
            target: Target::Module(ModuleId(target as u32)),
            shorthand: false,
        });
    }
```

In the `ImportKind::List(names)` arm, record each binding. Change the three
`if let` blocks so each also pushes its occurrence:

```rust
                if let Some(r) = exports[target].values.get(name).copied() {
                    bind_value(
                        &mut definitions.modules[mid],
                        diagnostics,
                        name.clone(),
                        n.span,
                        r,
                    );
                    let target = match r {
                        Res::Def(id) => Target::Def(id),
                        Res::Variant(sum, i) => Target::Variant(sum, i as u32),
                        Res::Builtin(b) => Target::Builtin(b),
                    };
                    imports.push(Occurrence {
                        span: n.span,
                        role: Role::Use,
                        target,
                        shorthand: false,
                    });
                    found = true;
                }
                if let Some(id) = exports[target].types.get(name).copied() {
                    bind_type(
                        &mut definitions.modules[mid],
                        diagnostics,
                        name.clone(),
                        n.span,
                        id,
                    );
                    imports.push(Occurrence {
                        span: n.span,
                        role: Role::Use,
                        target: Target::Def(id),
                        shorthand: false,
                    });
                    found = true;
                }
                if let Some(id) = exports[target].traits.get(name).copied() {
                    bind_trait(
                        &mut definitions.modules[mid],
                        diagnostics,
                        name.clone(),
                        n.span,
                        id,
                    );
                    imports.push(Occurrence {
                        span: n.span,
                        role: Role::Use,
                        target: Target::Def(id),
                        shorthand: false,
                    });
                    found = true;
                }
```

The value arm shadows the outer `target: usize` with a `Target` inside its
block only. If clippy or a reader objects, rename it `named`, and ledger
the rename.

- [ ] **Step 4: Run the tests to see them pass, and the crate's suite**

Run: `cd /d/Projects/nona/nova && cargo test -p nova-resolver 2>&1 | tail -15`
Expected: every result line `ok`, the four new tests among them; no
warnings.

Then check that nothing else builds a `ProgramResolution` by hand:

Run: `cd /d/Projects/nona/nova && git grep -n "ProgramResolution {" -- crates`
Expected: one line, in `resolve_program`.

- [ ] **Step 5: Commit**

Write `$P/msg-2.txt`:

```text
nova-resolver: record the occurrences of import items

An import's module, at its first segment, and each listed name once per
namespace it binds (spec 3.4a §3.3). They go into
ProgramResolution::imports, always: a few per import, which the driver
merges into the language server's index only when asked.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
```

Run: `cd /d/Projects/nona/nova && cargo fmt --all && git add crates/nova-resolver/src/lib.rs && git commit -q -F $P/msg-2.txt && git log -1 --format=%s`
Expected: `nova-resolver: record the occurrences of import items`

---

### Task 3: The checker's recording, declarations, locals and their types; the driver's plumbing

**Files:**
- Modify: `crates/nova-typeck/src/lib.rs`:
  - `CheckOptions` (`:46-48`) and `CheckResult` (`:26-34`);
  - `display_ty_named`, a new function after `display_ty`.
- Modify: `crates/nova-typeck/src/check.rs`:
  - the `use` lines (`:9`, `:14`);
  - `check_with` (`:118-215`) and the `Checker` struct (`:217-293`);
  - `collect_records`, `collect_sums`, `collect_traits` and
    `collect_impls`;
  - `collect_signatures`, `collect_externs`, `check_function`,
    `check_method` and `check_fn_body`;
  - `check_const`, `check_block`, `check_path`, `check_for`,
    `check_for_iterator` and `check_closure`;
  - `check_assign`, `check_pattern` and `variant_pattern`;
  - a new "index" section before `fn variant_index`;
  - the test helper's `CheckOptions` literal (`:16608`).
- Modify: `crates/nova-driver/src/analyze.rs` (`Options`, `Analysis`, `analyze_program`)
- Create: `crates/nova-driver/tests/index.rs`

**Interfaces:**
- Consumes: Task 1's `Index`, `Role`, `Target`; Task 2's
  `ProgramResolution::imports`.
- Produces:
  - `nova_typeck::CheckOptions { probe: Option<ProbePoint>, index: bool }`;
  - `nova_typeck::CheckResult::index: Option<nova_resolver::Index>`;
  - `pub fn nova_typeck::display_ty_named(ty: &Ty, defs: &Definitions, names: &[String]) -> String`;
  - `nova_driver::Options::index: bool`;
  - `nova_driver::Analysis::index: Option<nova_resolver::Index>`, with the
    resolver's import occurrences merged in;
  - the `Checker` helpers that Tasks 4 and 5 use: `note_use(span,
    target)`, `note_decl(span, target)` and `type_param_target(name) ->
    Option<Target>`.

- [ ] **Step 1: Write the failing tests**

Create `crates/nova-driver/tests/index.rs`:

```rust
//! The index of name occurrences (spec
//! `docs/superpowers/specs/2026-10-10-phase-3-4a-navigation-design.md` §3,
//! §8.1).

use std::io;
use std::path::{Path, PathBuf};

use nova_diagnostics::FileId;
use nova_driver::{analyze, Analysis, Options, Sources};
use nova_resolver::{Index, Occurrence, Role, Target};

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

fn options() -> Options {
    Options {
        keep_going: true,
        tests: true,
        module_only: true,
        index: true,
        ..Options::default()
    }
}

const MAIN: &str = "mem/main.nova";

/// Analyse `files`, each `(path, text)`; the first is the entry.
fn analyse(files: &[(&str, &str)]) -> Analysis {
    let buffers = Buffers(
        files
            .iter()
            .map(|(p, t)| (PathBuf::from(p), t.to_string()))
            .collect(),
    );
    analyze(Path::new(files[0].0), &buffers, &options()).expect("the entry is readable")
}

fn index(a: &Analysis) -> &Index {
    a.index.as_ref().expect("the index was asked for")
}

fn word_starts<'t>(text: &'t str, word: &'t str) -> impl Iterator<Item = usize> + 't {
    let ident = |c: Option<char>| c.is_some_and(|c| c.is_ascii_alphanumeric() || c == '_');
    text.match_indices(word).map(|(i, _)| i).filter(move |&i| {
        !ident(text[..i].chars().next_back()) && !ident(text[i + word.len()..].chars().next())
    })
}

/// Where the `n`th whole word `word` (from 0) is in `path`'s text.
#[track_caller]
fn place(a: &Analysis, path: &str, word: &str, n: usize) -> (FileId, u32) {
    let (file, _) = a
        .modules
        .iter()
        .find(|(_, p)| p == Path::new(path))
        .unwrap_or_else(|| panic!("{path} is not a module"));
    let text = a.db.get_source(*file).unwrap();
    let start = word_starts(text, word)
        .nth(n)
        .unwrap_or_else(|| panic!("no `{word}` number {n} in {path}"));
    (*file, start as u32)
}

/// The occurrence a request at the `n`th `word` finds.
#[track_caller]
fn at<'a>(a: &'a Analysis, path: &str, word: &str, n: usize) -> &'a Occurrence {
    let (file, start) = place(a, path, word, n);
    index(a)
        .at(a.definitions.as_ref().unwrap(), file, start)
        .unwrap_or_else(|| panic!("nothing at `{word}` number {n}"))
}

/// The `n`th `word` is a use, and the `d`th `word` in `decl_path`
/// declares what it uses. Returns the target.
#[track_caller]
fn uses(a: &Analysis, path: &str, word: &str, n: usize, decl_path: &str, d: usize) -> Target {
    let o = at(a, path, word, n);
    assert_eq!(o.role, Role::Use, "`{word}` number {n}: {o:?}");
    let decl = index(a)
        .declaration(&o.target)
        .unwrap_or_else(|| panic!("no declaration of {o:?}"));
    assert_eq!(
        (decl.span.file, decl.span.start),
        place(a, decl_path, word, d),
        "`{word}` number {n} is {o:?}"
    );
    o.target
}

/// The `n`th `word` declares something: what.
#[track_caller]
fn declares(a: &Analysis, path: &str, word: &str, n: usize) -> Target {
    let o = at(a, path, word, n);
    assert_eq!(o.role, Role::Declaration, "`{word}` number {n}: {o:?}");
    o.target
}

/// Nothing is recorded over the `n`th `word`.
#[track_caller]
fn nothing_at(a: &Analysis, path: &str, word: &str, n: usize) {
    let (file, start) = place(a, path, word, n);
    let end = start + word.len() as u32;
    let found: Vec<&Occurrence> = index(a)
        .occurrences
        .iter()
        .filter(|o| o.span.file == file && o.span.start < end && start < o.span.end)
        .collect();
    assert!(found.is_empty(), "`{word}` number {n}: {found:?}");
}

/// The name of the file `target` is declared in.
#[track_caller]
fn declared_in(a: &Analysis, target: &Target) -> String {
    let decl = index(a)
        .declaration(target)
        .unwrap_or_else(|| panic!("no declaration of {target:?}"));
    a.db.get_name(decl.span.file).unwrap().to_string()
}

/// The type hover shows for the local declared at the `n`th `word`.
#[track_caller]
fn local_type(a: &Analysis, path: &str, word: &str, n: usize) -> Option<String> {
    let Target::Local(span) = declares(a, path, word, n) else {
        panic!("`{word}` number {n} is not a local");
    };
    index(a).types.get(&span).cloned()
}

// === Task 3: declarations, locals and their types ===

#[test]
fn a_locals_use_resolves_to_its_let() {
    let a = analyse(&[(MAIN, "fn main() {\n    let total = 1\n    let b = total + 2\n}\n")]);
    let t = uses(&a, MAIN, "total", 1, MAIN, 0);
    assert!(matches!(t, Target::Local(_)), "{t:?}");
}

#[test]
fn a_shadowing_let_is_a_new_local() {
    let a = analyse(&[(
        MAIN,
        "fn main() {\n    let x = 1\n    let x = x + 1\n    let y = x\n}\n",
    )]);
    // `x + 1` uses the first `x`; `let y = x` the second.
    uses(&a, MAIN, "x", 2, MAIN, 0);
    uses(&a, MAIN, "x", 3, MAIN, 1);
}

#[test]
fn parameters_self_and_an_assignments_target_are_locals() {
    let text = "record Counter { n: Int }\n\
impl Counter {\n    fn bump(mut self, by: Int) -> Int {\n        let mut step = by\n        step = step + 1\n        self.n + step\n    }\n}\n\
fn main() {}\n";
    let a = analyse(&[(MAIN, text)]);
    uses(&a, MAIN, "by", 1, MAIN, 0);
    uses(&a, MAIN, "step", 1, MAIN, 0);
    uses(&a, MAIN, "step", 2, MAIN, 0);
    uses(&a, MAIN, "step", 3, MAIN, 0);
    uses(&a, MAIN, "self", 1, MAIN, 0);
}

#[test]
fn closure_parameters_and_for_variables_are_locals() {
    let text = "fn main() {\n    let add = |k: Int| k + 1\n    for i in 0..3 {\n        let j = i\n    }\n}\n";
    let a = analyse(&[(MAIN, text)]);
    uses(&a, MAIN, "k", 1, MAIN, 0);
    uses(&a, MAIN, "i", 1, MAIN, 0);
}

#[test]
fn a_match_binding_and_a_variant_payload_are_locals() {
    let text = "fn main() {\n    let o: Option<Int> = Some(1)\n    let v = match o {\n        Some(n) => n,\n        other => 0,\n    }\n}\n";
    let a = analyse(&[(MAIN, text)]);
    uses(&a, MAIN, "n", 1, MAIN, 0);
    assert!(matches!(declares(&a, MAIN, "other", 0), Target::Local(_)));
}

#[test]
fn items_fields_variants_and_trait_methods_are_declared() {
    let text = "pub const LIMIT: Int = 3\n\
record Point { x: Int, y: Int }\n\
type Shape = Round(Int) | Flat\n\
trait Show {\n    fn show(self) -> String\n    fn twice(self) -> String { self.show() }\n}\n\
fn main() {}\n";
    let a = analyse(&[(MAIN, text)]);
    assert!(matches!(declares(&a, MAIN, "LIMIT", 0), Target::Def(_)));
    assert!(matches!(declares(&a, MAIN, "main", 0), Target::Def(_)));
    let Target::Def(point) = declares(&a, MAIN, "Point", 0) else {
        panic!("a record is a Def");
    };
    assert_eq!(declares(&a, MAIN, "y", 0), Target::Field(point, 1));
    let Target::Def(shape) = declares(&a, MAIN, "Shape", 0) else {
        panic!("a sum type is a Def");
    };
    assert_eq!(declares(&a, MAIN, "Flat", 0), Target::Variant(shape, 1));
    let Target::Def(show) = declares(&a, MAIN, "Show", 0) else {
        panic!("a trait is a Def");
    };
    assert_eq!(declares(&a, MAIN, "show", 0), Target::TraitMethod(show, 0));
    // A provided method is declared once, as the trait's method, never as
    // its default body's own `Def`.
    assert_eq!(declares(&a, MAIN, "twice", 0), Target::TraitMethod(show, 1));
    let (file, start) = place(&a, MAIN, "twice", 0);
    let here = index(&a)
        .occurrences
        .iter()
        .filter(|o| o.span.file == file && o.span.start == start)
        .count();
    assert_eq!(here, 1);
}

#[test]
fn parameters_without_a_body_are_declared() {
    let text = "trait Area {\n    fn scaled(self, factor: Int) -> Int\n}\n\
extern \"C\" {\n    fn abs(value: Int) -> Int\n}\n\
fn main() {}\n";
    let a = analyse(&[(MAIN, text)]);
    assert!(matches!(declares(&a, MAIN, "factor", 0), Target::Local(_)));
    assert!(matches!(declares(&a, MAIN, "value", 0), Target::Local(_)));
}

#[test]
fn type_parameters_are_declared() {
    let text = "record Pair<A, B> { a: A, b: B }\nfn swap<T>(t: T) -> T { t }\nfn main() {}\n";
    let a = analyse(&[(MAIN, text)]);
    assert!(matches!(declares(&a, MAIN, "A", 0), Target::TypeParam(_)));
    assert!(matches!(declares(&a, MAIN, "T", 0), Target::TypeParam(_)));
}

#[test]
fn a_locals_type_is_read_after_inference() {
    let text = "fn first<T>(xs: [T]) -> T {\n    let x = xs[0]\n    x\n}\n\
fn main() {\n    let mut v = Vec::new()\n    v.push(1)\n    let f = |k| k + 1\n    let w = Vec::new()\n}\n";
    let a = analyse(&[(MAIN, text)]);
    // A type parameter by its declared name, not `T0`.
    assert_eq!(local_type(&a, MAIN, "x", 0).as_deref(), Some("T"));
    assert_eq!(local_type(&a, MAIN, "v", 0).as_deref(), Some("Vec<Int>"));
    // A closure's local, from its enclosing function's inference.
    assert_eq!(local_type(&a, MAIN, "k", 0).as_deref(), Some("Int"));
    // An unsolved variable as `_`.
    assert_eq!(local_type(&a, MAIN, "w", 0).as_deref(), Some("Vec<_>"));
}

#[test]
fn nothing_is_recorded_for_an_unresolved_name_or_a_wildcard() {
    let a = analyse(&[(MAIN, "fn main() {\n    let _ = 1\n    let a = missing + 1\n}\n")]);
    nothing_at(&a, MAIN, "missing", 0);
    nothing_at(&a, MAIN, "_", 0);
}

#[test]
fn the_index_is_none_unless_asked_for() {
    let buffers = Buffers(vec![(PathBuf::from(MAIN), "fn main() {}\n".to_string())]);
    let options = Options {
        index: false,
        ..options()
    };
    let a = analyze(Path::new(MAIN), &buffers, &options).unwrap();
    assert!(a.index.is_none());
}
```

`declared_in` is used from Task 4 on. Until then it is dead code in a
test target, which only warns; add `#[allow(dead_code)]` above it, and
remove the attribute in Task 4.

- [ ] **Step 2: Run the tests to see them fail**

Run: `cd /d/Projects/nona/nova && cargo test -p nova-driver --test index 2>&1 | tail -20`
Expected: a compile error, `struct `Options` has no field named `index``
(and `no field `index` on type `Analysis``).

- [ ] **Step 3: The options and the results**

In `crates/nova-typeck/src/lib.rs`, add a field to `CheckOptions`, after
`pub probe: Option<ProbePoint>,`:

```rust
    /// Record the language server's index (spec
    /// `docs/superpowers/specs/2026-10-10-phase-3-4a-navigation-design.md`
    /// §3). Recording is write-only: the checker never reads it.
    pub index: bool,
```

and to `CheckResult`, after `pub probe: ProbeResult,`:

```rust
    /// What the index recorded: `None` unless [`CheckOptions::index`].
    pub index: Option<nova_resolver::Index>,
```

After `display_ty`'s closing `}`, add:

```rust
/// [`display_ty`] for hover (spec 3.4a §3.4): type parameter `i` prints as
/// `names[i]`, and an unsolved variable or an error as `_`.
pub fn display_ty_named(ty: &Ty, defs: &Definitions, names: &[String]) -> String {
    let show = |t: &Ty| display_ty_named(t, defs, names);
    let list = |ts: &[Ty]| ts.iter().map(show).collect::<Vec<_>>().join(", ");
    match ty {
        Ty::Fn { params, ret } => format!("fn({}) -> {}", list(params.as_slice()), show(ret)),
        Ty::Sum { def_id, args } | Ty::Record { def_id, args } => {
            let name = &defs.def(*def_id).name;
            if args.is_empty() {
                name.clone()
            } else {
                format!("{name}<{}>", list(args.as_slice()))
            }
        }
        Ty::Array(elem) => format!("[{}]", show(elem)),
        Ty::Future(out) => format!("Future<{}>", show(out)),
        Ty::Param(i) => names
            .get(*i as usize)
            .filter(|n| !n.is_empty())
            .cloned()
            .unwrap_or_else(|| format!("T{i}")),
        Ty::Assoc { on, assoc } => format!("{}::{}", show(on), defs.def(*assoc).name),
        Ty::Var(_) | Ty::Error => "_".to_string(),
        other => display_ty(other, defs),
    }
}
```

(If `list`'s `.map(show)` does not borrow-check, write `.map(|t|
display_ty_named(t, defs, names))`.)

In `crates/nova-typeck/src/check.rs`, at the test helper near `:16608`,
change `let options = CheckOptions { probe: Some(ProbePoint { file, offset
}), };` to add `index: false,` after the `probe` line.

In `crates/nova-driver/src/analyze.rs`, add to `Options`, after `pub probe:
Option<Probe>,`:

```rust
    /// Record the language server's index (spec
    /// `docs/superpowers/specs/2026-10-10-phase-3-4a-navigation-design.md`
    /// §3, §3.6). Off for every CLI command.
    pub index: bool,
```

to `Analysis`, after `pub probe: ProbeResult,`:

```rust
    /// Every name the front end resolved, the resolver's imports merged
    /// in: `None` unless [`Options::index`].
    pub index: Option<nova_resolver::Index>,
```

In `analyze_program`, add `index: None,` to the `Analysis { … }` literal
after `probe: ProbeResult::default(),`. After
`diagnostics.extend(resolved.diagnostics);` add:

```rust
    let imports = resolved.imports;
```

Change `&CheckOptions { probe },` to `&CheckOptions { probe, index: options.index },`.
After `analysis.probe = checked.probe;` add:

```rust
    if let Some(mut index) = checked.index {
        index.extend(imports);
        analysis.index = Some(index);
    }
```

The language server builds two `Options` literals with no `..`, so they
must name the new field. In `crates/nova-lsp/src/checker.rs` (`run`) and
`crates/nova-lsp/src/completion.rs` (`analysis_at`), add `index: false,`
to the `Options { … }` literal. The server's diagnostics never turn the
index on (spec §3.6).

- [ ] **Step 4: The checker's state and helpers**

In `check.rs`, change the `use nova_resolver::…` line (`:9`) to:

```rust
use nova_resolver::{
    Builtin, DefId, DefKind, Definitions, Index, MethodOwner, ModuleId, Res, Role, Target,
};
```

and the `use crate::{…}` line (`:14`) to add `display_ty_named`:

```rust
use crate::{
    display_ty, display_ty_named, CheckOptions, CheckResult, Member, MemberKind, ProbePoint,
    ProbeResult,
};
```

In `struct Checker`, after the `probe_locals_pending` field, add:

```rust
    /// The language server's index (spec 3.4a §3), when asked for. Written
    /// here and never read.
    index: Option<Index>,
    /// The type parameters of the item being checked, names and
    /// declaration spans, the impl's first (plan decision 4). Set by
    /// [`Checker::enter_type_params`] at each item entry point.
    type_params: Vec<(String, Span)>,
```

In `check_with`'s `Checker { … }` literal, after `probe_locals_pending:
None,` add:

```rust
        index: options.index.then(Index::default),
        type_params: Vec::new(),
```

Before `checker.reject_self_type_params();` add:

```rust
    checker.note_item_declarations();
```

and in the returned `CheckResult { … }`, after `probe: checker.probe_result,`
add `index: checker.index,`.

Before `    fn variant_index(&self, sum_id: DefId, name: &str) -> Option<usize> {`
(in the `// === Helpers ===` section) insert:

```rust
    // === The language server's index (spec 3.4a §3) ===

    fn note(&mut self, span: Span, role: Role, target: Target) {
        if let Some(index) = self.index.as_mut() {
            index.record(span, role, target);
        }
    }

    fn note_use(&mut self, span: Span, target: Target) {
        self.note(span, Role::Use, target);
    }

    fn note_decl(&mut self, span: Span, target: Target) {
        self.note(span, Role::Declaration, target);
    }

    /// Declare every resolver definition (spec §3.2): each item by its
    /// `Def`, and a sum's variants beside it. A trait's provided method is
    /// declared as its `TraitMethod` by `collect_traits`, never by its
    /// default body's `Def`.
    fn note_item_declarations(&mut self) {
        if self.index.is_none() {
            return;
        }
        let defs: &'a Definitions = self.defs;
        for (i, def) in defs.defs().iter().enumerate() {
            let id = DefId(i as u32);
            match &def.kind {
                DefKind::Method {
                    owner: MethodOwner::TraitDefault,
                    ..
                } => {}
                DefKind::Sum { variants, .. } => {
                    self.note_decl(def.span, Target::Def(id));
                    for (vi, v) in variants.iter().enumerate() {
                        self.note_decl(v.span, Target::Variant(id, vi as u32));
                    }
                }
                _ => self.note_decl(def.span, Target::Def(id)),
            }
        }
    }

    /// Enter an item's type parameters, the impl's first: declare them, and
    /// let the item's uses find them by name (plan decision 4).
    fn enter_type_params(&mut self, lists: &[&[ast::TypeParam]]) {
        self.type_params.clear();
        for list in lists {
            for g in list.iter() {
                self.type_params.push((g.name.value.clone(), g.name.span));
                self.note_decl(g.name.span, Target::TypeParam(g.name.span));
            }
        }
    }

    fn type_param_target(&self, name: &str) -> Option<Target> {
        self.type_params
            .iter()
            .rev()
            .find(|(n, _)| n == name)
            .map(|(_, span)| Target::TypeParam(*span))
    }

    /// `fcx.new_local` for a name the user wrote, declaring it (spec §3.2).
    /// `_` declares nothing. The checker's own locals call `new_local`
    /// directly and are never recorded.
    fn bind_local(
        &mut self,
        fcx: &mut FnCtx,
        name: String,
        ty: Ty,
        is_mut: bool,
        span: Span,
    ) -> LocalId {
        if name != "_" {
            self.note_decl(span, Target::Local(span));
        }
        fcx.new_local(name, ty, is_mut, span)
    }

    /// Declare parameters that no body binds: a trait's required method's
    /// and an extern function's.
    fn note_params(&mut self, params: &[ast::Param]) {
        for p in params {
            self.note_decl(p.name.span, Target::Local(p.name.span));
        }
    }

    /// Each declared local's type as hover shows it (spec §3.4), read after
    /// inference, with `names` for the type parameters. A wholly unknown
    /// type is not stored.
    fn note_local_types(&mut self, locals: &[hir::Local], names: &[String]) {
        let defs = self.defs;
        let Some(index) = self.index.as_mut() else {
            return;
        };
        for local in locals {
            if !index.has(local.span, Role::Declaration, Target::Local(local.span)) {
                continue;
            }
            if matches!(local.ty, Ty::Var(_) | Ty::Error) {
                continue;
            }
            let shown = display_ty_named(&local.ty, defs, names);
            index.types.insert(local.span, shown);
        }
    }

```

After the free function `fn generic_scope(` (`:7349`) and its body, add:

```rust
/// Type parameter names by index, for [`display_ty_named`].
fn param_names(generics: &FxHashMap<String, u32>) -> Vec<String> {
    let n = generics
        .values()
        .map(|&i| i as usize + 1)
        .max()
        .unwrap_or(0);
    let mut names = vec![String::new(); n];
    for (name, &i) in generics {
        names[i as usize] = name.clone();
    }
    names
}
```

- [ ] **Step 5: Declarations at the collection passes**

All in `check.rs`:

1. `collect_records`: after `let generics = generic_scope(&decl.generics);`
   add:

   ```rust
            self.enter_type_params(&[&decl.generics]);
            for (fi, field) in decl.fields.iter().enumerate() {
                self.note_decl(field.name.span, Target::Field(DefId(i as u32), fi as u32));
            }
   ```

2. `collect_sums`: after `let generics = generic_scope(&decl.generics);`
   add `self.enter_type_params(&[&decl.generics]);`.
3. `collect_traits`, in the first loop over `decl.items`, before
   `self.check_duplicate_generics(generics, "method");` add:

   ```rust
                self.enter_type_params(&[generics.as_slice()]);
   ```

   and before `methods.push(hir::TraitMethod {` add:

   ```rust
                // Spec 3.4a §3.2: a required and a provided method alike are
                // declared as the trait's method, at the index `methods`
                // gives it, which is `MethodRes::Trait`'s.
                self.note_decl(name.span, Target::TraitMethod(def_id, methods.len() as u32));
                if !is_default {
                    self.note_params(params);
                }
   ```

4. `collect_traits`, in the default-body loop, after the `if
   !f.where_clause.is_empty() { continue; }` block, add
   `self.enter_type_params(&[&f.generics]);`.
5. `collect_impls`: after `let impl_generics =
   generic_scope(&block.generics);` add
   `self.enter_type_params(&[&block.generics]);`. In the method loop, after
   `let Some(def_id) = impl_methods.get(&(item_index, mi)).copied() else {
   continue; };` add
   `self.enter_type_params(&[&block.generics, &f.generics]);`.
6. `collect_signatures`: after `let generics = generic_scope(&f.generics);`
   add `self.enter_type_params(&[&f.generics]);`.
7. `collect_externs`: before `let empty = FxHashMap::default();` add:

   ```rust
            self.enter_type_params(&[]);
            self.note_params(&sig.params);
   ```

8. `check_function`: after `let generics = generic_scope(&f.generics);` add
   `self.enter_type_params(&[&f.generics]);`.
9. `check_method`: after the `let generics = match loc.owner { … };`
   statement add:

   ```rust
        match (loc.owner, &file.items[loc.item_index].value) {
            (MethodOwner::Impl, ast::Item::Impl(block)) => {
                self.enter_type_params(&[&block.generics, &f.generics])
            }
            _ => self.enter_type_params(&[&f.generics]),
        }
   ```

10. `check_const`: after `self.impl_self = None;` add
    `self.enter_type_params(&[]);`.

- [ ] **Step 6: Locals, their uses and their types**

Replace the seven places a user's name becomes a local. Each `fcx.new_local(`
becomes `self.bind_local(fcx, ` with the same arguments (in
`check_fn_body`, where `fcx` is a local, `self.bind_local(&mut fcx, `):

| Function | The line, as it stands |
|---|---|
| `check_fn_body` | `fcx.new_local(p.name.value.clone(), ty.clone(), p.is_mut, p.name.span);` |
| `check_block` | `fcx.new_local(name, value.ty.clone(), *is_mut \|\| pat_mut, name_span);` |
| `check_for` | `let i = fcx.new_local(var_name, Ty::Int, false, var_span);` |
| `check_for_iterator` | `let elem = fcx.new_local(var_name, item_ty, false, var_span);` |
| `check_closure` | `let local = fcx.new_local(p.name.value.clone(), ty.clone(), p.is_mut, p.name.span);` |
| `check_pattern` (`Pattern::Ident`) | `let local = fcx.new_local(name.value.clone(), scrut_ty.clone(), *is_mut, name.span);` |
| `variant_pattern` | `let local = fcx.new_local(name.value.clone(), bound_ty, *is_mut, name.span);` |

Leave `__f_{fname}`, `__base` and every `new_local_unscoped` call alone.
Then check that only those two `new_local(` calls remain:

Run: `cd /d/Projects/nona/nova && git grep -n "fcx.new_local(" -- crates/nova-typeck/src/check.rs`
Expected: two lines, `__f_{fname}` and `__base`, both in `check_record_literal`.

Record the uses:
- `check_path`: after `if let Some(local) = fcx.lookup(name) {` add

  ```rust
            self.note_use(
                path.segments[0].span,
                Target::Local(fcx.locals[local.0 as usize].span),
            );
  ```

- `check_assign`: after the `let Some(local) = fcx.lookup(name) else { … };`
  statement add the same three-line call.

Record the types. In `check_fn_body`, before `let mut func = hir::Function {`
add:

```rust
        let names = param_names(&fcx.generics);
```

after `self.finalize_function(&mut func, &fcx.icx);` add
`self.note_local_types(&func.locals, &names);`, and change the closures loop
to:

```rust
        for c in &mut closures {
            self.finalize_function(c, &fcx.icx);
            self.note_local_types(&c.locals, &names);
        }
```

In `check_const`, after its `self.finalize_function(&mut func, &fcx.icx);`
add `self.note_local_types(&func.locals, &[]);`, and in its closures loop,
after `self.finalize_function(cl, &fcx.icx);`, add
`self.note_local_types(&cl.locals, &[]);`.

- [ ] **Step 7: Run the tests to see them pass**

Run: `cd /d/Projects/nona/nova && cargo test -p nova-driver --test index 2>&1 | tail -20`
Expected: `test result: ok. 11 passed; 0 failed`.

Then the crates' own suites, to confirm recording changes nothing, and
that the server still builds:

Run: `cd /d/Projects/nona/nova && cargo test -p nova-typeck -p nova-resolver -p nova-driver 2>&1 > $P/t3.txt; python -X utf8 $P/count.py $P/t3.txt`
Expected: `0 failed`.

Run: `cd /d/Projects/nona/nova && cargo check --workspace --all-targets 2>&1 | tail -3`
Expected: `Finished`, with no warning.

- [ ] **Step 8: Commit**

Write `$P/msg-3.txt`:

```text
nova-typeck: record declarations, locals and their types

With CheckOptions::index, the checker records into the language
server's index (spec 3.4a §3.2, §3.4):
- every resolver definition, a sum's variants, a record's fields, and
  a trait's methods once each, a provided one as its TraitMethod;
- parameters, lets, for variables, closure parameters and pattern
  bindings, at the AST's binding sites, and their uses;
- type parameters, at each item's entry point;
- each local's type after inference, with type parameters by name.

Recording is write-only, and the driver's Options::index turns it on,
merging the resolver's import occurrences in. The CLI leaves it off.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
```

Run: `cd /d/Projects/nona/nova && cargo fmt --all && git add crates/nova-typeck crates/nova-driver crates/nova-lsp && git commit -q -F $P/msg-3.txt && git log -1 --format=%s`
Expected: `nova-typeck: record declarations, locals and their types`

---

### Task 4: Types, bounds, projections and impl headers

**Files:**
- Modify: `crates/nova-typeck/src/check.rs`:
  - `convert_ty` (`:2315-2560`);
  - `resolve_bounds`, `apply_where` and `collect_supertraits`;
  - `collect_impls` (its trait header and its associated-type bindings);
  - a new free function `primitive`.
- Modify: `crates/nova-driver/tests/index.rs`

**Interfaces:**
- Consumes: Task 3's `note_use`, `type_param_target`.
- Produces:
  - `Use` occurrences for every type name: `Def(record or sum)`,
    `Primitive(name)`, `TypeParam(span)`;
  - the associated type of a projection: `Def(assoc)`;
  - each trait named by a bound, a `where` clause, a supertrait list or an
    impl header: `Def(trait)`;
  - each associated type an impl binds: `Def(assoc)`.

- [ ] **Step 1: Write the failing tests**

Remove the `#[allow(dead_code)]` above `declared_in` in
`crates/nova-driver/tests/index.rs`, and append:

```rust
// === Task 4: types, bounds, projections, impl headers ===

#[test]
fn a_type_annotation_uses_its_type_and_arguments() {
    let text = "record Point { x: Int }\n\
fn origin(p: Point) -> Option<Point> { None }\n\
fn main() {\n    let q: Point = Point { x: 1 }\n}\n";
    let a = analyse(&[(MAIN, text)]);
    uses(&a, MAIN, "Point", 1, MAIN, 0); // a parameter's type
    uses(&a, MAIN, "Point", 2, MAIN, 0); // a generic argument
    uses(&a, MAIN, "Point", 3, MAIN, 0); // a let's annotation
    assert_eq!(at(&a, MAIN, "Int", 0).target, Target::Primitive("Int"));
    let option = at(&a, MAIN, "Option", 0).target;
    assert!(declared_in(&a, &option).starts_with("<std/"), "{option:?}");
}

#[test]
fn a_type_parameters_uses_resolve_to_its_declaration() {
    let text = "record Pair<A> { a: A }\n\
impl<K> Pair<K> {\n    fn get<M>(self, m: M) -> K { self.a }\n}\n\
fn wrap<T>(t: T) -> Pair<T> {\n    let p: Pair<T> = Pair { a: t }\n    p\n}\n\
fn main() {}\n";
    let a = analyse(&[(MAIN, text)]);
    uses(&a, MAIN, "A", 1, MAIN, 0);
    uses(&a, MAIN, "K", 1, MAIN, 0); // the impl's self type
    uses(&a, MAIN, "K", 2, MAIN, 0); // a method's return type: the impl's
    uses(&a, MAIN, "M", 1, MAIN, 0);
    uses(&a, MAIN, "T", 1, MAIN, 0);
    uses(&a, MAIN, "T", 2, MAIN, 0);
    uses(&a, MAIN, "T", 3, MAIN, 0); // inside the body
}

#[test]
fn a_projection_uses_its_parameter_and_associated_type() {
    let text = "trait Source {\n    type Item\n    fn take(self) -> Self::Item\n}\n\
record Box { v: Int }\n\
impl Source for Box {\n    type Item = Int\n    fn take(self) -> Self::Item { self.v }\n}\n\
fn pull<S: Source>(s: S) -> S::Item { s.take() }\n\
fn main() {}\n";
    let a = analyse(&[(MAIN, text)]);
    let item = declares(&a, MAIN, "Item", 0);
    assert!(matches!(item, Target::Def(_)), "{item:?}");
    assert_eq!(at(&a, MAIN, "Item", 1).target, item); // the trait's Self::Item
    assert_eq!(at(&a, MAIN, "Item", 2).target, item); // the impl's binding
    assert_eq!(at(&a, MAIN, "Item", 3).target, item); // the impl's Self::Item
    assert_eq!(at(&a, MAIN, "Item", 4).target, item); // S::Item
    uses(&a, MAIN, "S", 2, MAIN, 0); // the projection's base
    // `Self` is not recorded (spec decision 20).
    nothing_at(&a, MAIN, "Self", 0);
    nothing_at(&a, MAIN, "Self", 1);
}

#[test]
fn bounds_where_clauses_supertraits_and_impl_headers_use_their_traits() {
    let text = "trait Named { fn name(self) -> String }\n\
trait Loud: Named { fn shout(self) -> String }\n\
record Dog { n: Int }\n\
impl Named for Dog { fn name(self) -> String { \"d\" } }\n\
fn greet<T: Named>(t: T) -> String { t.name() }\n\
fn call<U>(u: U) -> String where U: Named { u.name() }\n\
fn main() {}\n";
    let a = analyse(&[(MAIN, text)]);
    uses(&a, MAIN, "Named", 1, MAIN, 0); // a supertrait
    uses(&a, MAIN, "Named", 2, MAIN, 0); // an impl header
    uses(&a, MAIN, "Named", 3, MAIN, 0); // an inline bound
    uses(&a, MAIN, "Named", 4, MAIN, 0); // a where clause
    uses(&a, MAIN, "Dog", 1, MAIN, 0); // an impl's self type
    uses(&a, MAIN, "U", 2, MAIN, 0); // a where clause's parameter
}
```

- [ ] **Step 2: Run the tests to see them fail**

Run: `cd /d/Projects/nona/nova && cargo test -p nova-driver --test index 2>&1 | tail -20`
Expected: the four new tests fail with "nothing at `Point` number 1" and
the like; Task 3's eleven pass.

- [ ] **Step 3: Record the types**

In `convert_ty`:

1. In the two-segment branch, inside `if by_index.is_some() || (in_impl &&
   self.impl_self.is_some()) {`, after the `if !args.is_empty() { …E0012… }`
   block, add:

   ```rust
                        if by_index.is_some() {
                            if let Some(t) = self.type_param_target(base) {
                                self.note_use(path.segments[0].span, t);
                            }
                        }
   ```

   and replace its last line, `return self.resolve_projection(on, base,
   assoc_name, &candidates, ty.span);`, with:

   ```rust
                        let projected =
                            self.resolve_projection(on, base, assoc_name, &candidates, ty.span);
                        if let Ty::Assoc { assoc, .. } = &projected {
                            self.note_use(path.segments[1].span, Target::Def(*assoc));
                        }
                        return projected;
   ```

2. In the one-segment part, inside `if let Some(&idx) = generics.get(name) {`
   add first:

   ```rust
                    if let Some(t) = self.type_param_target(name) {
                        self.note_use(path.segments[0].span, t);
                    }
   ```

3. Inside `if name == "Future" {` add first:
   `self.note_use(path.segments[0].span, Target::Primitive("Future"));`.
4. Inside `if let Some(p) = prim {` add first:

   ```rust
                    if let Some(name) = primitive(name) {
                        self.note_use(path.segments[0].span, Target::Primitive(name));
                    }
   ```

5. Inside `if let Some(def_id) = self.defs.resolve_type(self.cur_module,
   name) {` add first: `self.note_use(path.segments[0].span,
   Target::Def(def_id));`.

After `fn param_names(` (Task 3), add:

```rust
/// A built-in type's name, as the index's `Primitive` holds it.
fn primitive(name: &str) -> Option<&'static str> {
    nova_resolver::RESERVED_TYPE_NAMES
        .iter()
        .copied()
        .find(|n| *n == name)
}
```

- [ ] **Step 4: Record the traits and the bindings**

1. `resolve_bounds`: in `Some(id) => {` add first:

   ```rust
                            if let Some(seg) = b.value.segments.last() {
                                self.note_use(seg.span, Target::Def(id));
                            }
   ```

2. `apply_where`: in its `Some(id) => {` add the same three lines.
3. `collect_supertraits`: in `Some(id) => {` add first:

   ```rust
                        if let Some(seg) = path.value.segments.last() {
                            self.note_use(seg.span, Target::Def(id));
                        }
   ```

4. `collect_impls`, the trait header: change `Some(id) => Some(id),` to:

   ```rust
                        Some(id) => {
                            if let Some(seg) = tr.value.segments.last() {
                                self.note_use(seg.span, Target::Def(id));
                            }
                            Some(id)
                        }
   ```

5. `collect_impls`, the associated-type bindings: after the `let Some(assoc)
   = resolved else { … continue; };` statement add
   `self.note_use(b.name.span, Target::Def(assoc));`.

- [ ] **Step 5: Run the tests to see them pass**

Run: `cd /d/Projects/nona/nova && cargo test -p nova-driver --test index 2>&1 | tail -20`
Expected: `test result: ok. 15 passed; 0 failed`.

Run: `cd /d/Projects/nona/nova && cargo test -p nova-typeck 2>&1 > $P/t4.txt; python -X utf8 $P/count.py $P/t4.txt`
Expected: `0 failed`.

- [ ] **Step 6: Commit**

Write `$P/msg-4.txt`:

```text
nova-typeck: record type names, bounds and projections

Every type name the checker converts (records, sums, primitives, type
parameters), a projection's base and associated type, and the traits
named by bounds, where clauses, supertraits and impl headers, plus each
associated type an impl binds (spec 3.4a §3.2). `Self` is not recorded.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
```

Run: `cd /d/Projects/nona/nova && cargo fmt --all && git add crates/nova-typeck crates/nova-driver && git commit -q -F $P/msg-4.txt && git log -1 --format=%s`
Expected: `nova-typeck: record type names, bounds and projections`

---

### Task 5: Values, calls, records, fields, methods, patterns and families

**Files:**
- Modify: `crates/nova-typeck/src/check.rs`:
  - `check_path` (`:3479-3559`) and `check_call` (`:3561-3870`);
  - `check_record_literal` (`:4856-4990`), `check_field` (`:5285`) and
    `check_field_set` (`:6485`);
  - `check_method_call` (`:5609-5690`), `check_pattern` (`:6740-6880`)
    and `check_impl_method_signatures` (`:1770`);
  - the index section (Task 3).
- Modify: `crates/nova-driver/tests/index.rs`

**Interfaces:**
- Consumes: Task 3's helpers, Task 4's `primitive`.
- Produces:
  - `Use` occurrences for:
    - functions, consts and variants as values or calls;
    - builtins;
    - associated functions, with their qualifiers;
    - record literals' names and fields;
    - field reads and writes;
    - method calls: `Def`, `TraitMethod` or `BuiltinMethod("len")`;
    - patterns' variants;
  - shorthand marks;
  - `Index::implements` links.

- [ ] **Step 1: Write the failing tests**

Append to `crates/nova-driver/tests/index.rs`:

```rust
// === Task 5: values, calls, records, fields, methods, patterns, families ===

#[test]
fn calls_and_values_use_their_definitions() {
    let text = "const LIMIT: Int = 3\n\
fn double(n: Int) -> Int { n * 2 }\n\
fn main() {\n    let a = double(LIMIT)\n    let f = double\n    println(\"x\")\n}\n";
    let a = analyse(&[(MAIN, text)]);
    uses(&a, MAIN, "double", 1, MAIN, 0); // a direct call
    uses(&a, MAIN, "double", 2, MAIN, 0); // a function as a value
    uses(&a, MAIN, "LIMIT", 1, MAIN, 0);
    assert!(matches!(at(&a, MAIN, "println", 0).target, Target::Builtin(_)));
}

#[test]
fn variants_use_their_sum_and_variant() {
    let text = "type Shape = Round(Int) | Flat\n\
fn main() {\n    let a = Round(1)\n    let b = Shape::Round(2)\n    let c = Flat\n    let d = Shape::Flat\n    let e: Option<Int> = None\n}\n";
    let a = analyse(&[(MAIN, text)]);
    uses(&a, MAIN, "Round", 1, MAIN, 0);
    uses(&a, MAIN, "Round", 2, MAIN, 0);
    uses(&a, MAIN, "Shape", 1, MAIN, 0);
    uses(&a, MAIN, "Flat", 1, MAIN, 0);
    uses(&a, MAIN, "Flat", 2, MAIN, 0);
    uses(&a, MAIN, "Shape", 2, MAIN, 0);
    let none = at(&a, MAIN, "None", 0).target;
    assert!(declared_in(&a, &none).starts_with("<std/"), "{none:?}");
}

#[test]
fn associated_functions_use_their_qualifier_and_function() {
    let text = "record Point { x: Int }\n\
impl Point {\n    fn origin() -> Point { Point { x: 0 } }\n}\n\
trait Zero { fn zero() -> Self }\n\
impl Zero for Int { fn zero() -> Int { 0 } }\n\
fn pick<Z: Zero>() -> Z { Z::zero() }\n\
fn main() {\n    let p = Point::origin()\n    let n = Int::zero()\n}\n";
    let a = analyse(&[(MAIN, text)]);
    uses(&a, MAIN, "origin", 1, MAIN, 0);
    uses(&a, MAIN, "Point", 4, MAIN, 0); // the qualifier of Point::origin
    let Target::Def(zero) = declares(&a, MAIN, "Zero", 0) else {
        panic!("a trait is a Def");
    };
    // Through a bound and through an impl: the trait's method.
    assert_eq!(at(&a, MAIN, "zero", 2).target, Target::TraitMethod(zero, 0));
    assert_eq!(at(&a, MAIN, "zero", 3).target, Target::TraitMethod(zero, 0));
    uses(&a, MAIN, "Z", 2, MAIN, 0);
    // `Int` number 3 is the qualifier of `Int::zero()`.
    assert_eq!(at(&a, MAIN, "Int", 3).target, Target::Primitive("Int"));
}

#[test]
fn record_literals_and_field_accesses_use_their_fields() {
    let text = "record Point { x: Int, y: Int }\n\
fn main() {\n    let x = 1\n    let mut p = Point { x, y: 2 }\n    p.y = p.x + p.y\n}\n";
    let a = analyse(&[(MAIN, text)]);
    let Target::Def(point) = declares(&a, MAIN, "Point", 0) else {
        panic!("a record is a Def");
    };
    uses(&a, MAIN, "Point", 1, MAIN, 0);
    // `{ x }`: a field use and a value use, both marked; `at` takes the value.
    let (file, start) = place(&a, MAIN, "x", 2);
    let here: Vec<&Occurrence> = index(&a)
        .occurrences
        .iter()
        .filter(|o| o.span.file == file && o.span.start == start)
        .collect();
    assert_eq!(here.len(), 2, "{here:?}");
    assert!(here.iter().all(|o| o.shorthand), "{here:?}");
    assert!(here.iter().any(|o| o.target == Target::Field(point, 0)), "{here:?}");
    uses(&a, MAIN, "x", 2, MAIN, 1);
    assert_eq!(at(&a, MAIN, "y", 1).target, Target::Field(point, 1)); // `y: 2`
    assert_eq!(at(&a, MAIN, "y", 2).target, Target::Field(point, 1)); // the write
    assert_eq!(at(&a, MAIN, "x", 3).target, Target::Field(point, 0)); // `p.x`
    assert_eq!(at(&a, MAIN, "y", 3).target, Target::Field(point, 1)); // `p.y`
}

#[test]
fn method_calls_use_their_method() {
    let text = "record Counter { n: Int }\n\
impl Counter {\n    fn get(self) -> Int { self.n }\n}\n\
trait Show { fn show(self) -> String }\n\
impl Show for Counter { fn show(self) -> String { \"c\" } }\n\
fn main() {\n    let c = Counter { n: 1 }\n    let a = c.get()\n    let b = c.show()\n    let xs = [1, 2]\n    let n = xs.len()\n}\n";
    let a = analyse(&[(MAIN, text)]);
    uses(&a, MAIN, "get", 1, MAIN, 0);
    let Target::Def(show) = declares(&a, MAIN, "Show", 0) else {
        panic!("a trait is a Def");
    };
    assert_eq!(at(&a, MAIN, "show", 2).target, Target::TraitMethod(show, 0));
    assert_eq!(at(&a, MAIN, "len", 0).target, Target::BuiltinMethod("len"));
}

#[test]
fn patterns_use_their_variants() {
    let text = "type Shape = Round(Int) | Flat\n\
fn size(s: Shape) -> Int {\n    match s {\n        Round(r) => r,\n        Shape::Flat => 0,\n    }\n}\n\
fn other(s: Shape) -> Int {\n    match s {\n        Shape::Round(r) => r,\n        Flat => 0,\n    }\n}\n\
fn main() {}\n";
    let a = analyse(&[(MAIN, text)]);
    uses(&a, MAIN, "Round", 1, MAIN, 0);
    uses(&a, MAIN, "Flat", 1, MAIN, 0);
    uses(&a, MAIN, "Round", 2, MAIN, 0);
    uses(&a, MAIN, "Flat", 2, MAIN, 0);
    uses(&a, MAIN, "Shape", 2, MAIN, 0);
    uses(&a, MAIN, "Shape", 4, MAIN, 0);
    uses(&a, MAIN, "r", 1, MAIN, 0);
}

#[test]
fn an_impl_method_is_in_its_trait_methods_family() {
    let text = "trait Show { fn show(self) -> String }\n\
record A { n: Int }\nrecord B { n: Int }\n\
impl Show for A { fn show(self) -> String { \"a\" } }\n\
impl Show for B { fn show(self) -> String { \"b\" } }\n\
fn main() {}\n";
    let a = analyse(&[(MAIN, text)]);
    let trait_method = declares(&a, MAIN, "show", 0);
    let a_show = declares(&a, MAIN, "show", 1);
    let b_show = declares(&a, MAIN, "show", 2);
    assert!(matches!(trait_method, Target::TraitMethod(..)), "{trait_method:?}");
    assert_eq!(index(&a).family(&a_show), vec![trait_method, a_show, b_show]);
}
```

- [ ] **Step 2: Run the tests to see them fail**

Run: `cd /d/Projects/nona/nova && cargo test -p nova-driver --test index 2>&1 | tail -20`
Expected: the seven new tests fail (`nothing at …`, or a wrong target);
Tasks 3 and 4's fifteen pass.

- [ ] **Step 3: Values and calls**

Add to the index section of `check.rs`:

```rust
    /// A two-segment path's qualifier, `Point` in `Point::origin`, by
    /// `qualifier_self_ty`'s order: a primitive, then a nominal type.
    fn note_qualifier(&mut self, seg: &Spanned<String>) {
        if let Some(name) = primitive(&seg.value) {
            self.note_use(seg.span, Target::Primitive(name));
        } else if let Some(def_id) = self.defs.resolve_type(self.cur_module, &seg.value) {
            self.note_use(seg.span, Target::Def(def_id));
        }
    }

    /// A field access's field (spec §3.2).
    fn note_field(&mut self, fcx: &FnCtx, recv_ty: &Ty, field: u32, span: Span) {
        if let Ty::Record { def_id, .. } = fcx.icx.apply(recv_ty) {
            self.note_use(span, Target::Field(def_id, field));
        }
    }

    fn mark_shorthand(&mut self, span: Span) {
        if let Some(index) = self.index.as_mut() {
            index.mark_shorthand(span);
        }
    }
```

`check_path`:
1. Two segments: inside `if let Some(def_id) =
   self.defs.resolve_type(self.cur_module, ty_name) {` add first
   `self.note_use(path.segments[0].span, Target::Def(def_id));`, and inside
   `if let Some(vi) = self.variant_index(def_id, v_name) {` add first
   `self.note_use(path.segments[1].span, Target::Variant(def_id, vi as u32));`.
2. In `DefKind::Fn { .. } => {` and in `DefKind::Const { .. } => {` add
   first `self.note_use(path.segments[0].span, Target::Def(def_id));`.
3. Change `Some(Res::Variant(sum_id, vi)) => self.make_variant(fcx, sum_id,
   vi, Vec::new(), span),` to:

   ```rust
            Some(Res::Variant(sum_id, vi)) => {
                self.note_use(path.segments[0].span, Target::Variant(sum_id, vi as u32));
                self.make_variant(fcx, sum_id, vi, Vec::new(), span)
            }
   ```

`check_call`, one segment, inside `if fcx.lookup(name).is_none() {`:
1. In `Some(Res::Def(def_id)) => {`, inside the `if matches!(…Fn…ExternFn…) {`
   block, add first `self.note_use(path.segments[0].span, Target::Def(def_id));`.
2. In `Some(Res::Variant(sum_id, vi)) => {` add first
   `self.note_use(path.segments[0].span, Target::Variant(sum_id, vi as u32));`.
3. In `Some(Res::Builtin(b)) => {` add first
   `self.note_use(path.segments[0].span, Target::Builtin(b));`.

`check_call`, two segments:
1. Inside `if let Some(&k) = fcx.generics.get(ty_name) {` add first:

   ```rust
                    if let Some(t) = self.type_param_target(ty_name) {
                        self.note_use(path.segments[0].span, t);
                    }
   ```

   and change its `[(tid, idx)] => self.emit_trait_call(` arm to a block that
   first records the function:

   ```rust
                        [(tid, idx)] => {
                            self.note_use(path.segments[1].span, Target::TraitMethod(*tid, *idx));
                            self.emit_trait_call(
                                fcx,
                                *tid,
                                *idx,
                                TraitCallSelf::Qualifier(Ty::Param(k)),
                                checked,
                                span,
                            )
                        }
   ```

2. `Type::Variant(args)`: inside `if let Some(vi) = self.variant_index(def_id,
   name) {` add first:

   ```rust
                        self.note_use(path.segments[0].span, Target::Def(def_id));
                        self.note_use(path.segments[1].span, Target::Variant(def_id, vi as u32));
   ```

3. The inherent associated function: in `[assoc_id] => {` add after `let
   assoc_id = *assoc_id;`:

   ```rust
                            self.note_qualifier(&path.segments[0]);
                            self.note_use(path.segments[1].span, Target::Def(assoc_id));
   ```

4. Through a trait impl: in `[(tid, idx)] => {` (the one calling
   `TraitCallSelf::Qualifier(self_ty)`) add after `let (tid, idx) = (*tid,
   *idx);`:

   ```rust
                            self.note_qualifier(&path.segments[0]);
                            self.note_use(path.segments[1].span, Target::TraitMethod(tid, idx));
   ```

- [ ] **Step 4: Records, fields and methods**

`check_record_literal`:
1. After the `let Some(record) = … else { …E0010… };` statement add
   `self.note_use(path.segments[0].span, Target::Def(def_id));`.
2. In the fields loop, after the `let Some(field) = record.fields.iter().find(|f|
   f.name == fname) else { … continue; };` statement add:

   ```rust
            if let Some(fi) = record.fields.iter().position(|f| f.name == fname) {
                self.note_use(init.name.span, Target::Field(def_id, fi as u32));
            }
   ```

3. After the `let value = match &init.value { … };` statement add:

   ```rust
            // `{ x }`: the field and the value share the span (spec §3.2).
            if init.value.is_none() {
                self.mark_shorthand(init.name.span);
            }
   ```

`check_field`: inside `if let Some((index, field_ty)) =
self.record_field_index_and_ty(fcx, &recv_ty, &field.value, span) {` add
first `self.note_field(fcx, &recv_ty, index, field.span);`.

`check_field_set`: after the `let Some((index, field_ty)) =
self.record_field_index_and_ty(fcx, &recv_ty, &field.value, span) else { …
};` statement add `self.note_field(fcx, &recv_ty, index, field.span);`.

`check_method_call`:
1. Inside `if method.value == "len" && args.is_empty() {` add first
   `self.note_use(method.span, Target::BuiltinMethod("len"));`.
2. In `MethodRes::Inherent(def_id) => {` add first
   `self.note_use(method.span, Target::Def(def_id));`.
3. Change `MethodRes::Trait(trait_id, method_idx) => self.emit_trait_call(…),`
   to a block:

   ```rust
            MethodRes::Trait(trait_id, method_idx) => {
                self.note_use(method.span, Target::TraitMethod(trait_id, method_idx));
                self.emit_trait_call(
                    fcx,
                    trait_id,
                    method_idx,
                    TraitCallSelf::Receiver(receiver, receiver_ast),
                    args,
                    span,
                )
            }
   ```

- [ ] **Step 5: Patterns and families**

`check_pattern`:
1. `Pattern::Ident`: inside `if self.variant_matches_scrutinee(fcx, sum_id,
   scrut_ty) {` add first `self.note_use(name.span, Target::Variant(sum_id,
   vi as u32));`.
2. `Pattern::Path` of one segment: inside its `if
   self.variant_matches_scrutinee(…) {` add first
   `self.note_use(path.segments[0].span, Target::Variant(sum_id, vi as u32));`.
3. `Pattern::Path` of two segments: inside its innermost `if
   self.variant_matches_scrutinee(…) {` add first:

   ```rust
                            self.note_use(path.segments[0].span, Target::Def(sum_id));
                            self.note_use(path.segments[1].span, Target::Variant(sum_id, vi as u32));
   ```

4. `Pattern::TupleStruct`: before its last line,
   `self.variant_pattern(fcx, sum_id, vi, fields, scrut_ty, pattern.span)`,
   add:

   ```rust
                if path.segments.len() == 2 {
                    self.note_use(path.segments[0].span, Target::Def(sum_id));
                }
                if let Some(last) = path.segments.last() {
                    self.note_use(last.span, Target::Variant(sum_id, vi as u32));
                }
   ```

`check_impl_method_signatures`: after the `let Some(trait_method) =
tr.methods.iter().find(|m| &m.name == name) else { … continue; };`
statement add:

```rust
                // Spec 3.4a §3.2: the family links, where the checker matches
                // an impl method to its trait's.
                if let Some(idx) = tr.methods.iter().position(|m| &m.name == name) {
                    if let Some(index) = self.index.as_mut() {
                        index.implement(*def_id, (trait_id, idx as u32));
                    }
                }
```

- [ ] **Step 6: Run the tests to see them pass**

Run: `cd /d/Projects/nona/nova && cargo test -p nova-driver --test index 2>&1 | tail -20`
Expected: `test result: ok. 22 passed; 0 failed`.

Run: `cd /d/Projects/nona/nova && cargo test -p nova-typeck -p nova-driver 2>&1 > $P/t5.txt; python -X utf8 $P/count.py $P/t5.txt`
Expected: `0 failed`.

- [ ] **Step 7: Commit**

Write `$P/msg-5.txt`:

```text
nova-typeck: record values, calls, fields, methods and patterns

Functions, consts, variants and builtins as values and calls;
associated functions with their qualifiers; record literals' names and
fields, a shorthand `{ x }` marked as a field and a value at once;
field reads and writes; method calls, a trait-resolved one as the
trait's method and an array's `len` as a builtin method; patterns'
variants. Where the checker matches an impl method to its trait's, the
index links the two into one family (spec 3.4a §3.1, §3.2).

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
```

Run: `cd /d/Projects/nona/nova && cargo fmt --all && git add crates/nova-typeck crates/nova-driver && git commit -q -F $P/msg-5.txt && git log -1 --format=%s`
Expected: `nova-typeck: record values, calls, fields, methods and patterns`

---

### Task 6: The index is complete over the corpora

**Files:**
- Create: `crates/nova-driver/tests/index_complete.rs`
- Modify: `crates/nova-diagnostics/src/files.rs` (`FileDb::id_of`)
- Modify: `crates/nova-driver/tests/broken.rs`
- Modify, if the checks find gaps: `crates/nova-typeck/src/check.rs` and
  `crates/nova-driver/tests/index.rs`

**Interfaces:**
- Consumes: Tasks 1-5.
- Produces: `pub fn FileDb::id_of(&self, name: &str) -> Option<FileId>`,
  used again by Task 7.

- [ ] **Step 1: `FileDb::id_of`, test first**

In `crates/nova-diagnostics/src/files.rs`, in its tests module (create
`#[cfg(test)] mod tests { use super::*; … }` at the end of the file if
there is none), add:

```rust
    #[test]
    fn a_file_is_found_by_its_name() {
        let mut db = FileDb::new();
        let a = db.add("<std/core>", "a");
        let b = db.add("b.nova", "b");
        assert_eq!(db.id_of("<std/core>"), Some(a));
        assert_eq!(db.id_of("b.nova"), Some(b));
        assert_eq!(db.id_of("c.nova"), None);
    }
```

Run: `cd /d/Projects/nona/nova && cargo test -p nova-diagnostics a_file_is_found 2>&1 | tail -8`
Expected: a compile error, `no method named `id_of``.

After `get_name`, add:

```rust
    /// The file added under `name`, the latest if several were.
    pub fn id_of(&self, name: &str) -> Option<FileId> {
        self.by_name.get(name).copied()
    }
```

Run the same command. Expected: `test result: ok. 1 passed`.

- [ ] **Step 2: Write the completeness checks**

Create `crates/nova-driver/tests/index_complete.rs`:

```rust
//! Every name in std, `examples/` and `tests/runtime/` is in the index
//! (spec `docs/superpowers/specs/2026-10-10-phase-3-4a-navigation-design.md`
//! §8.2):
//! 1. every use has exactly one declaration, unless it is a builtin, a
//!    builtin method, a primitive or a module;
//! 2. every identifier token lies inside an occurrence, two for a
//!    shorthand, apart from the names `skipped` lists.

use std::collections::HashMap;
use std::path::PathBuf;

use nova_diagnostics::{FileId, Severity, Span, Spanned};
use nova_driver::{analyze, Analysis, DiskSources, Options};
use nova_lexer::Token;
use nova_resolver::{Role, Target};

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// Every program the checks analyse, sorted.
fn programs() -> Vec<PathBuf> {
    let root = root();
    let mut out = Vec::new();
    for entry in std::fs::read_dir(root.join("examples")).unwrap() {
        let main = entry.unwrap().path().join("src").join("main.nova");
        if main.is_file() {
            out.push(main);
        }
    }
    for entry in std::fs::read_dir(root.join("tests").join("runtime")).unwrap() {
        let path = entry.unwrap().path();
        // Meant to fail `nova check` with E0089 (run_tests.rs).
        if path.file_name().is_some_and(|n| n == "bytes_reserved.nova") {
            continue;
        }
        if path.extension().is_some_and(|e| e == "nova") {
            out.push(path);
        }
    }
    // One program of three files, which holds the corpora's only imports.
    out.push(root.join("tests").join("runtime").join("modules").join("main.nova"));
    out.sort();
    out
}

fn options() -> Options {
    Options {
        keep_going: true,
        tests: true,
        module_only: true,
        index: true,
        ..Options::default()
    }
}

/// std's files in `a`, which can be checked only as std (spec §8.2).
fn std_files(a: &Analysis) -> Vec<FileId> {
    nova_resolver::STD_MODULES
        .iter()
        .chain(std::iter::once(&nova_resolver::STD_TEST_MODULE))
        .map(|(name, _)| {
            let short = name.strip_prefix("$std.").unwrap_or(name);
            a.db.id_of(&format!("<std/{short}>"))
                .unwrap_or_else(|| panic!("<std/{short}> is not in the analysis"))
        })
        .collect()
}

fn place(a: &Analysis, span: Span) -> String {
    let name = a.db.get_name(span.file).unwrap_or("?");
    let (line, column) = a.db.location(span.file, span.start).unwrap_or((0, 0));
    let text = a.db.get_source(span.file).unwrap_or("");
    let word = text
        .get(span.start as usize..span.end as usize)
        .unwrap_or("?");
    format!("{name}:{line}:{column} `{word}`")
}

/// Check 1, over the whole analysis.
fn every_use_has_its_declaration(a: &Analysis, failures: &mut Vec<String>) {
    let index = a.index.as_ref().unwrap();
    let mut declared: HashMap<Target, usize> = HashMap::new();
    for o in &index.occurrences {
        if o.role == Role::Declaration {
            *declared.entry(o.target).or_default() += 1;
        }
    }
    for o in &index.occurrences {
        if o.role != Role::Use {
            continue;
        }
        if matches!(
            o.target,
            Target::Builtin(_) | Target::BuiltinMethod(_) | Target::Primitive(_) | Target::Module(_)
        ) {
            continue;
        }
        let n = declared.get(&o.target).copied().unwrap_or(0);
        if n != 1 {
            failures.push(format!("{}: {:?} has {n} declarations", place(a, o.span), o.target));
        }
    }
}

/// The identifiers check 2 skips, each with its reason (spec §8.2).
fn skipped(tokens: &[Spanned<Token>], i: usize) -> bool {
    let Token::Ident(name) = &tokens[i].value else {
        return true;
    };
    // `_` declares nothing.
    if name == "_" {
        return true;
    }
    let prev = i.checked_sub(1).map(|j| &tokens[j].value);
    // An attribute's name, `@test`.
    if matches!(prev, Some(Token::At)) {
        return true;
    }
    // A `module m` declaration's name, which the resolver ignores.
    if matches!(prev, Some(Token::Module)) {
        return true;
    }
    // An attribute's arguments, `@test(should_panic)`.
    in_attribute_arguments(tokens, i)
}

fn in_attribute_arguments(tokens: &[Spanned<Token>], i: usize) -> bool {
    let mut j = i;
    while j > 0 {
        j -= 1;
        match tokens[j].value {
            Token::LParen => {
                return j >= 2
                    && matches!(tokens[j - 1].value, Token::Ident(_))
                    && matches!(tokens[j - 2].value, Token::At);
            }
            Token::Ident(_) | Token::Comma => continue,
            _ => return false,
        }
    }
    false
}

/// Check 2, over one file.
fn every_name_is_covered(a: &Analysis, file: FileId, failures: &mut Vec<String>) {
    let index = a.index.as_ref().unwrap();
    let mut at: HashMap<Span, (usize, bool)> = HashMap::new();
    for o in index.occurrences.iter().filter(|o| o.span.file == file) {
        let slot = at.entry(o.span).or_default();
        slot.0 += 1;
        slot.1 |= o.shorthand;
    }
    let source = a.db.get_source(file).unwrap();
    let (tokens, _) = nova_lexer::lex(source, file);
    for i in 0..tokens.len() {
        if skipped(&tokens, i) {
            continue;
        }
        let span = tokens[i].span;
        match at.get(&span) {
            None => failures.push(format!("{}: no occurrence", place(a, span))),
            Some(&(n, true)) if n != 2 => {
                failures.push(format!("{}: a shorthand with {n} occurrences", place(a, span)));
            }
            Some(_) => {}
        }
    }
}

#[test]
fn every_name_in_the_corpora_is_indexed() {
    let programs = programs();
    // 6 examples, 133 top-level runtime programs and the modules program,
    // on 2026-10-10.
    assert_eq!(programs.len(), 140, "{programs:#?}");
    let mut files = 0;
    let mut failures: Vec<String> = Vec::new();
    for (k, path) in programs.iter().enumerate() {
        let a = analyze(path, &DiskSources, &options()).unwrap();
        let errors: Vec<String> = a
            .diagnostics
            .iter()
            .filter(|d| d.severity == Severity::Error)
            .map(|d| format!("{} {}", d.code, d.message))
            .collect();
        assert!(errors.is_empty(), "{} does not check cleanly: {errors:?}", path.display());
        every_use_has_its_declaration(&a, &mut failures);
        let mut own: Vec<FileId> = a.modules.iter().map(|(f, _)| *f).collect();
        // std is the same in every analysis: check its files once.
        if k == 0 {
            own.extend(std_files(&a));
        }
        files += own.len();
        for file in own {
            every_name_is_covered(&a, file, &mut failures);
        }
    }
    // 6 + 133 + 3 files of user code, and std's 17.
    assert_eq!(files, 159);
    failures.sort();
    failures.dedup();
    assert!(
        failures.is_empty(),
        "{} problem(s):\n{}",
        failures.len(),
        failures.join("\n")
    );
}
```

- [ ] **Step 3: Run the checks, and read what they find**

Run: `cd /d/Projects/nona/nova && cargo test -p nova-driver --test index_complete 2>&1 > $P/t6.txt; tail -60 $P/t6.txt`
Expected: either `test result: ok. 1 passed`, or a list of problems, each
naming a file, a line and the name.

This check exists to find what Tasks 3-5 missed. For each kind of problem
(a construct, not each line):
1. Find where the checker resolves that name.
2. Add a test for it to `crates/nova-driver/tests/index.rs` that fails.
3. Add the recording, at the AST name site.
4. Run `cargo test -p nova-driver --test index`, then this check again.
5. Ledger a ruling naming the construct.

A name that the checker never resolves, by design, goes to `skipped` with
its reason and a ruling. That covers a construct the corpora hold but the
checker ignores. Never skip a name the checker does resolve.

Repeat until the check passes.

- [ ] **Step 4: Broken programs, with the index on**

In `crates/nova-driver/tests/broken.rs`:
1. Add `index: true,` to the `Options { … }` literal in the worker.
2. Change `analyze(&path, &sources, &options).map(|a| a.diagnostics.len())`
   to `analyze(&path, &sources, &options).map(|a| spans_inside(&a))`.
3. Change `done.send((path, text.len(), outcome.is_ok(), took))` to:

   ```rust
            let spans_ok = !matches!(outcome, Ok(Ok(false)));
            if done
                .send((path, text.len(), outcome.is_ok(), spans_ok, took))
                .is_err()
   ```

4. In the receiving loop, change `Ok((path, len, ok, took)) => {` to `Ok((path,
   len, ok, spans_ok, took)) => {`, and add a branch after the `if !ok {
   … }` one:

   ```rust
                } else if !spans_ok {
                    failures.push(format!(
                        "an occurrence outside its file: {} cut at byte {len}",
                        path.display()
                    ));
   ```

5. Add, after `fn cuts(`:

   ```rust
/// Whether every occurrence the index recorded lies inside its file's text
/// (spec 3.4a §8.3).
fn spans_inside(a: &nova_driver::Analysis) -> bool {
    a.index.as_ref().map_or(true, |index| {
        index.occurrences.iter().all(|o| {
            a.db.get_source(o.span.file)
                .is_some_and(|s| o.span.start <= o.span.end && o.span.end as usize <= s.len())
        })
    })
}
   ```

6. Update the module comment's last sentence to: "Each analysis, with the
   index on, must return within 10 s without panicking, and every
   occurrence it records must lie inside its file."

Run: `cd /d/Projects/nona/nova && cargo test -p nova-driver --test broken 2>&1 | tail -8`
Expected: `test result: ok. 1 passed`.

- [ ] **Step 5: Run the driver's and checker's suites**

Run: `cd /d/Projects/nona/nova && cargo test -p nova-diagnostics -p nova-resolver -p nova-typeck -p nova-driver 2>&1 > $P/t6b.txt; python -X utf8 $P/count.py $P/t6b.txt`
Expected: `0 failed`.

- [ ] **Step 6: Commit**

Write `$P/msg-6.txt`, adding a line per construct Step 3 found:

```text
nova-driver: check that the index is complete

Over std, examples/ and tests/runtime/ (159 files), every use has
exactly one declaration, and every identifier lies inside an
occurrence, apart from `_`, attribute names and arguments, and a
`module` declaration's name (spec 3.4a §8.2). bytes_reserved.nova,
which is meant to fail, is left out by name. The 364 cut programs are
analysed with the index on, and every occurrence must lie inside its
file. FileDb::id_of finds std's files by name.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
```

Run: `cd /d/Projects/nona/nova && cargo fmt --all && git add crates/nova-diagnostics crates/nova-driver crates/nova-typeck && git commit -q -F $P/msg-6.txt && git log -1 --format=%s`
Expected: `nova-driver: check that the index is complete`

---

### Task 7: Which analysis answers a request; hover

**Files:**
- Create: `crates/nova-lsp/src/analysis.rs`, `crates/nova-lsp/src/hover.rs`
- Modify: `crates/nova-lsp/src/lib.rs`:
  - the `mod` list;
  - `capabilities()`;
  - `Server::request`;
  - a new `Server::answer_at`.
- Modify: `crates/nova-lsp/src/completion.rs` (`analysis_at`, `guarded`, the `use` lines)
- Modify: `crates/nova-lsp/src/workspace.rs` (`Workspace::by_path`, `Overlay::with`)
- Modify: `crates/nova-typeck/src/lib.rs` (`builtin_text`); `crates/nova-typeck/src/check.rs` (`builtin_signature` becomes `pub(crate)`)
- Create: `crates/nova-cli/tests/lsp_navigation.rs`

**Interfaces:**
- Consumes: `Analysis::index` (Task 3), `Index::at` and `declaration`
  (Task 1).
- Produces:
  - `analysis::Scope` (`Project(PathBuf)`, `File(PathBuf)`; Task 8 adds
    `Std`) and `analysis::Answer { analysis, file, scope }`;
  - `analysis::options(probe: Option<Probe>, index: bool) -> Options`;
  - `analysis::analyse(scope: &Scope, overlay: &Overlay, options: &Options) -> Option<Analysis>`;
  - `analysis::answering(path: &Path, overlay: &Overlay, options: &Options) -> Option<Answer>`;
  - `analysis::guarded`;
  - `hover::hover(answer: &Answer, offset: u32) -> Option<lsp::Hover>`;
  - `hover::declaration(a: &Analysis, target: &Target) -> Option<String>`;
  - `Workspace::by_path(&self, path: &Path) -> Option<&Document>`;
  - `Overlay::with(&self, path: &Path, text: String) -> Overlay`;
  - `nova_typeck::builtin_text(b: Builtin, defs: &Definitions) -> String`;
  - in the tests file: `open`, `project`, `position`, `at`, `range`,
    `hover` and `markdown`, which Tasks 8-12 use.

- [ ] **Step 1: Write the failing tests**

Create `crates/nova-cli/tests/lsp_navigation.rs`:

```rust
//! `nova lsp`'s navigation, driven over stdio (spec
//! `docs/superpowers/specs/2026-10-10-phase-3-4a-navigation-design.md`
//! §8.4, §9).

mod lsp_client;

use lsp_client::{file_uri, fresh_dir, same_uri, Client};
use serde_json::{json, Value};

const MANIFEST: &str = "[package]\nname = \"demo\"\nversion = \"0.1.0\"\nedition = \"2026\"\n";

fn open(client: &mut Client, uri: &str, text: &str) {
    client.notify(
        "textDocument/didOpen",
        json!({ "textDocument": { "uri": uri, "languageId": "nova", "version": 1, "text": text } }),
    );
}

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

/// The range of `word` at the start of the `n`th `marker` in `text`.
fn range(text: &str, marker: &str, n: usize, word: &str) -> Value {
    let start = text.match_indices(marker).nth(n).unwrap().0;
    json!({ "start": position(text, start), "end": position(text, start + word.len()) })
}

fn hover(client: &mut Client, uri: &str, position: Value) -> Value {
    client.request(
        "textDocument/hover",
        json!({ "textDocument": { "uri": uri }, "position": position }),
    )
}

/// A hover response's Markdown.
#[track_caller]
fn markdown(response: &Value) -> String {
    response["result"]["contents"]["value"]
        .as_str()
        .unwrap_or_else(|| panic!("no hover: {response}"))
        .to_string()
}

/// A hover's code block: its Markdown's first line inside the fence.
#[track_caller]
fn code(response: &Value) -> String {
    let md = markdown(response);
    md.strip_prefix("```nova\n")
        .and_then(|rest| rest.split("\n```").next())
        .unwrap_or_else(|| panic!("no code block: {md}"))
        .to_string()
}

// === Task 7: hover ===

const AREA: &str = "/// The area of a square.\n///\n/// Sides are whole numbers.\n\
pub fn area(side: Int) -> Int { side * side }\n\n\
fn main() {\n    let total = area(3)\n    let mut n = 0\n    n = total\n}\n";

#[test]
fn hover_shows_a_functions_signature_and_its_docs() {
    let dir = project("hover-docs", &[("main.nova", AREA)]);
    let uri = file_uri(&dir.join("src").join("main.nova"));
    let mut client = Client::start(&dir, false);
    open(&mut client, &uri, AREA);
    let response = hover(&mut client, &uri, at(AREA, "area(3)", 0));
    assert_eq!(
        markdown(&response),
        "```nova\npub fn area(side: Int) -> Int\n```\n\nThe area of a square.\n\nSides are whole numbers."
    );
    assert_eq!(response["result"]["range"], range(AREA, "area(3)", 0, "area"));
}

#[test]
fn hover_shows_a_locals_inferred_type() {
    let dir = project("hover-local", &[("main.nova", AREA)]);
    let uri = file_uri(&dir.join("src").join("main.nova"));
    let mut client = Client::start(&dir, false);
    open(&mut client, &uri, AREA);
    assert_eq!(code(&hover(&mut client, &uri, at(AREA, "total", 0))), "let total: Int");
    assert_eq!(code(&hover(&mut client, &uri, at(AREA, "n = total", 0))), "let mut n: Int");
}

const KINDS: &str = "/// A point.\nrecord Point { x: Int }\n\
type Shape = Round(Int) | Flat\n\
trait Show {\n    type Out\n    fn show(self) -> String\n}\n\
impl Show for Point {\n    type Out = Int\n    fn show(self) -> String { \"p\" }\n}\n\
pub const LIMIT: Int = 3\n\
fn keep<T: Show>(t: T) -> T {\n    let kept = t\n    kept\n}\n\
fn main() {\n    let p = Point { x: 1 }\n    let s = p.show()\n    let r = Round(LIMIT)\n    let xs = [1]\n    let n = xs.len()\n    print(\"x\")\n    let mut v = Vec::new()\n    v.push(1)\n}\n";

#[test]
fn hover_on_each_kind_of_name() {
    let dir = project("hover-kinds", &[("main.nova", KINDS)]);
    let uri = file_uri(&dir.join("src").join("main.nova"));
    let mut client = Client::start(&dir, false);
    open(&mut client, &uri, KINDS);
    let cases: &[(&str, usize, &str)] = &[
        ("Point { x: 1", 0, "record Point"),
        ("x: 1", 0, "x: Int"),
        ("Shape", 0, "type Shape"),
        ("Round(LIMIT)", 0, "Round(Int)"),
        ("LIMIT)", 0, "pub const LIMIT: Int"),
        ("show()", 0, "fn show(self) -> String"),
        ("Show>", 0, "trait Show"),
        ("Out = Int", 0, "type Out"),
        ("T) -> T", 0, "T: Show"),
        ("kept = t", 0, "let kept: T"),
        ("len()", 0, "fn len(self) -> Int"),
        ("print(", 0, "fn print(String) -> ()"),
        ("Int }", 0, "type Int"),
    ];
    for (marker, n, expected) in cases {
        let response = hover(&mut client, &uri, at(KINDS, marker, *n));
        assert_eq!(code(&response), *expected, "at `{marker}`: {response}");
    }
    // A record's docs.
    let point = markdown(&hover(&mut client, &uri, at(KINDS, "Point { x: 1", 0)));
    assert!(point.ends_with("\n\nA point."), "{point}");
    // A std method: its signature as std writes it.
    let push = code(&hover(&mut client, &uri, at(KINDS, "push(1)", 0)));
    assert!(push.contains("fn push("), "{push}");
}

#[test]
fn hover_on_a_module_and_on_a_package() {
    let dir = project(
        "hover-module",
        &[
            ("main.nova", "import geometry\nfn main() {}\n"),
            ("geometry.nova", "pub fn area() -> Int { 1 }\n"),
        ],
    );
    let uri = file_uri(&dir.join("src").join("main.nova"));
    let mut client = Client::start(&dir, false);
    open(&mut client, &uri, "import geometry\nfn main() {}\n");
    let response = hover(&mut client, &uri, at("import geometry", "geometry", 0));
    assert_eq!(code(&response), "module geometry");

    let (app, _geom) = app_and_library("hover-package", "pub fn area() -> Int { 1 }\n");
    let main = file_uri(&app.join("src").join("main.nova"));
    let mut client = Client::start(&app, false);
    open(&mut client, &main, APP_MAIN);
    let response = hover(&mut client, &main, at(APP_MAIN, "geom", 0));
    assert_eq!(code(&response), "package geom 0.1.0");
}

#[test]
fn hover_works_in_a_file_with_a_syntax_error() {
    let text = "fn broken( {\n}\nfn main() {\n    let total = 1\n    let b = total\n}\n";
    let dir = project("hover-broken", &[("main.nova", text)]);
    let uri = file_uri(&dir.join("src").join("main.nova"));
    let mut client = Client::start(&dir, false);
    open(&mut client, &uri, text);
    assert_eq!(code(&hover(&mut client, &uri, at(text, "total", 1))), "let total: Int");
}

#[test]
fn hover_on_no_name_or_an_unopened_document_is_null() {
    let dir = project("hover-null", &[("main.nova", AREA)]);
    let uri = file_uri(&dir.join("src").join("main.nova"));
    let mut client = Client::start(&dir, false);
    // Not opened yet.
    assert_eq!(hover(&mut client, &uri, at(AREA, "area(3)", 0))["result"], Value::Null);
    open(&mut client, &uri, AREA);
    // A blank line.
    assert_eq!(hover(&mut client, &uri, json!({ "line": 4, "character": 0 }))["result"], Value::Null);
}

// === Phase 3.3a's app and library, as in lsp.rs ===

const APP_MAIN: &str = "import geom\n\nfn main() {\n    let a: Int = area()\n}\n";

/// `app`, which depends on `geom` by path, in one fresh directory. `geom`'s
/// `src/lib.nova` is `lib`.
fn app_and_library(name: &str, lib: &str) -> (std::path::PathBuf, std::path::PathBuf) {
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
    std::fs::write(app.join("src").join("main.nova"), APP_MAIN).unwrap();
    (app, geom)
}
```

`same_uri` is first used in Task 8. Until then add `#[allow(unused_imports)]`
above the `use lsp_client::…` line, and remove it in Task 8.

- [ ] **Step 2: Run the tests to see them fail**

Run: `cd /d/Projects/nona/nova && cargo test -p nova-cli --test lsp_navigation 2>&1 > $P/t7.txt; tail -30 $P/t7.txt`
Expected: the six tests fail. Hover is not handled, so each response
carries `MethodNotFound` and `markdown` panics with "no hover: …"; the
null test fails on its error response.

- [ ] **Step 3: The shared analysis**

Create `crates/nova-lsp/src/analysis.rs`:

```rust
//! Which analysis answers a request (spec
//! `docs/superpowers/specs/2026-10-10-phase-3-4a-navigation-design.md` §4).
//! Completion and navigation share it.

use std::path::{Path, PathBuf};

use nova_diagnostics::FileId;
use nova_driver::{analyze_program, Analysis, Options, Probe, Program, Roots};

use crate::workspace::{Overlay, PathKey, ProjectKey};

/// The program an answer comes from, so that rename can analyse it again.
#[derive(Debug, Clone)]
pub enum Scope {
    /// A project's program, library and `tests/`.
    Project(PathBuf),
    /// A file on its own: `Program::for_file` finds its package.
    File(PathBuf),
}

/// An analysis, the request's file in it, and where it came from.
pub struct Answer {
    pub analysis: Analysis,
    pub file: FileId,
    pub scope: Scope,
}

/// A request's options: every stage runs, `@test` bodies are checked, no
/// MIR, and the probe or the index as asked.
pub fn options(probe: Option<Probe>, index: bool) -> Options {
    Options {
        keep_going: true,
        tests: true,
        module_only: true,
        probe,
        index,
    }
}

/// Analyse `scope` over `overlay`, guarded against a panic.
pub fn analyse(scope: &Scope, overlay: &Overlay, options: &Options) -> Option<Analysis> {
    let program = match scope {
        Scope::Project(dir) => Program::for_package(dir, Roots::Test),
        Scope::File(path) => Program::for_file(path),
    };
    guarded(|| analyze_program(program, overlay, options).ok())
}

/// The analysis that answers a request in `path` (spec §4, rule 1): its
/// project's, if that reaches the file, or else the file's own.
pub fn answering(path: &Path, overlay: &Overlay, options: &Options) -> Option<Answer> {
    if let ProjectKey::Root(dir) = ProjectKey::of(path) {
        let scope = Scope::Project(dir);
        if let Some(analysis) = analyse(&scope, overlay, options) {
            if let Some(file) = file_of(&analysis, path) {
                return Some(Answer {
                    analysis,
                    file,
                    scope,
                });
            }
        }
    }
    let scope = Scope::File(path.to_path_buf());
    let analysis = analyse(&scope, overlay, options)?;
    let file = file_of(&analysis, path)?;
    Some(Answer {
        analysis,
        file,
        scope,
    })
}

/// `path`'s file among `a`'s modules.
fn file_of(a: &Analysis, path: &Path) -> Option<FileId> {
    a.modules
        .iter()
        .find(|(_, p)| PathKey::of(p) == PathKey::of(path))
        .map(|(file, _)| *file)
}

/// `f`, with a panic in the front end logged and turned into `None`.
pub fn guarded<T>(f: impl FnOnce() -> Option<T>) -> Option<T> {
    match std::panic::catch_unwind(std::panic::AssertUnwindSafe(f)) {
        Ok(v) => v,
        Err(_) => {
            tracing::warn!("nova lsp: the front end panicked answering a request");
            None
        }
    }
}
```

In `crates/nova-lsp/src/completion.rs`, replace `analysis_at` and
`guarded` with:

```rust
/// The analysis that owns `path`, with the probe at `offset` (decision 9;
/// 3.4a §4). Completion needs no MIR and no index.
fn analysis_at(path: &Path, offset: u32, overlay: &Overlay) -> Option<Analysis> {
    let probe = Probe {
        path: path.to_path_buf(),
        offset,
    };
    analysis::answering(path, overlay, &analysis::options(Some(probe), false))
        .map(|answer| answer.analysis)
}
```

and change its `use` lines: `use nova_driver::{Analysis, Probe};`, `use
crate::analysis;`, `use crate::workspace::{Overlay, PathKey};`. Remove any
name the compiler then reports unused.

In `crates/nova-lsp/src/workspace.rs`, add to `impl Workspace`:

```rust
    /// The open document for `path`, if any.
    pub fn by_path(&self, path: &Path) -> Option<&Document> {
        self.docs.get(&PathKey::of(path))
    }
```

and after the `impl Sources for Overlay` block:

```rust
impl Overlay {
    /// This overlay, with `text` read for `path`.
    pub fn with(&self, path: &Path, text: String) -> Overlay {
        let mut out = self.clone();
        out.buffers.insert(PathKey::of(path), text);
        out
    }
}
```

- [ ] **Step 4: Hover**

In `crates/nova-typeck/src/check.rs`, change `fn builtin_signature(` to
`pub(crate) fn builtin_signature(`. In `crates/nova-typeck/src/lib.rs`,
after `display_ty_named`, add:

```rust
/// A builtin's signature for hover (spec 3.4a §5.1), its type parameters
/// as `T`, `U`, …: `fn print(String) -> ()`.
pub fn builtin_text(builtin: nova_resolver::Builtin, defs: &Definitions) -> String {
    let (params, ret) = check::builtin_signature(builtin);
    let names: Vec<String> = ["T", "U", "V", "W"].iter().map(|s| s.to_string()).collect();
    let params = params
        .iter()
        .map(|p| display_ty_named(p, defs, &names))
        .collect::<Vec<_>>()
        .join(", ");
    format!(
        "fn {}({params}) -> {}",
        builtin.name(),
        display_ty_named(&ret, defs, &names)
    )
}
```

Create `crates/nova-lsp/src/hover.rs`:

```rust
//! Hover (spec
//! `docs/superpowers/specs/2026-10-10-phase-3-4a-navigation-design.md`
//! §5.1). Declarations and docs are read from the source text (plan
//! decision 6).

use lsp_types as lsp;
use nova_diagnostics::LineIndex;
use nova_driver::Analysis;
use nova_resolver::{DefKind, Target};

use crate::analysis::Answer;
use crate::convert;

/// The hover at byte `offset` of the request's file.
pub fn hover(answer: &Answer, offset: u32) -> Option<lsp::Hover> {
    let a = &answer.analysis;
    let o = a
        .index
        .as_ref()?
        .at(a.definitions.as_ref()?, answer.file, offset)?;
    let mut value = format!("```nova\n{}\n```", declaration(a, &o.target)?);
    let docs = docs_of(a, &o.target);
    if !docs.is_empty() {
        value.push_str("\n\n");
        value.push_str(&docs);
    }
    let source = a.db.get_source(answer.file)?;
    let range = convert::range(&LineIndex::new(source), o.span.start, o.span.end);
    Some(lsp::Hover {
        contents: lsp::HoverContents::Markup(lsp::MarkupContent {
            kind: lsp::MarkupKind::Markdown,
            value,
        }),
        range: Some(range),
    })
}

/// The one line hover shows for `target` (spec §5.1's table).
pub fn declaration(a: &Analysis, target: &Target) -> Option<String> {
    let index = a.index.as_ref()?;
    let defs = a.definitions.as_ref()?;
    match target {
        Target::Builtin(b) => return Some(nova_typeck::builtin_text(*b, defs)),
        Target::BuiltinMethod(name) => return Some(format!("fn {name}(self) -> Int")),
        Target::Primitive(name) => return Some(format!("type {name}")),
        Target::Module(m) => return Some(module_line(a, m.0 as usize)),
        _ => {}
    }
    let decl = index.declaration(target)?.span;
    let text = a.db.get_source(decl.file)?;
    let at = decl.start as usize;
    let name = text.get(at..decl.end as usize)?;
    Some(match target {
        Target::Local(span) => local_line(text, at, name, index.types.get(span)),
        Target::TypeParam(_) => collapse(&text[at..end_of(text, at, &[',', '>'], true)]),
        Target::Field(..) => collapse(&text[at..end_of(text, at, &[',', '}', '\n'], false)]),
        Target::Variant(..) => collapse(&text[at..end_of(text, at, &['|', '\n'], false)]),
        Target::TraitMethod(..) => item_line(text, at, &['{', ';', '\n']),
        Target::Def(id) => match &defs.def(*id).kind {
            DefKind::Fn { .. } | DefKind::Method { .. } => item_line(text, at, &['{', ';']),
            DefKind::ExternFn { .. }
            | DefKind::AssocType { .. }
            | DefKind::Record { .. }
            | DefKind::Trait { .. } => item_line(text, at, &['{', ';', '\n']),
            DefKind::Sum { .. } | DefKind::Const { .. } => {
                item_line(text, at, &['=', '{', ';', '\n'])
            }
        },
        _ => name.to_string(),
    })
}

/// A declaration from the first token of its line to the first of `stops`
/// outside brackets, on one line. This keeps `pub`, `async` and a `where`
/// clause.
fn item_line(text: &str, at: usize, stops: &[char]) -> String {
    let line_start = text[..at].rfind('\n').map_or(0, |i| i + 1);
    let start = line_start + (text[line_start..at].len() - text[line_start..at].trim_start().len());
    collapse(&text[start..end_of(text, start, stops, false)])
}

/// The first of `stops` at bracket depth 0 from `from`, or the text's end.
/// With `angles`, `<` and `>` count as brackets too.
fn end_of(text: &str, from: usize, stops: &[char], angles: bool) -> usize {
    let mut depth = 0i32;
    for (i, c) in text[from..].char_indices() {
        if depth <= 0 && stops.contains(&c) {
            return from + i;
        }
        match c {
            '(' | '[' => depth += 1,
            ')' | ']' => depth -= 1,
            '<' if angles => depth += 1,
            '>' if angles => depth -= 1,
            _ => {}
        }
    }
    text.len()
}

fn collapse(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// `let x: T`, `let mut x: T`, or `let x` when the type is unknown. A
/// parameter no body binds shows its annotation as written.
fn local_line(text: &str, at: usize, name: &str, ty: Option<&String>) -> String {
    let before = text[..at].trim_end();
    let mutable = before.strip_suffix("mut").is_some_and(|rest| {
        !rest.ends_with(|c: char| c.is_ascii_alphanumeric() || c == '_')
    });
    let head = if mutable {
        format!("let mut {name}")
    } else {
        format!("let {name}")
    };
    if let Some(ty) = ty {
        return format!("{head}: {ty}");
    }
    let after_name = at + name.len();
    let rest = text[after_name..].trim_start();
    if let Some(annotation) = rest.strip_prefix(':') {
        let from = text.len() - annotation.len();
        let end = end_of(text, from, &[',', ')', '=', '{', ';', '\n'], true);
        return format!("{head}: {}", collapse(&text[from..end]));
    }
    head
}

/// `module m`, or `package <name> <version>` for a package's library.
fn module_line(a: &Analysis, m: usize) -> String {
    let Some((_, path)) = a.modules.get(m) else {
        return "module".to_string();
    };
    let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("?");
    if stem == "lib" {
        if let (Some(Some(pid)), Some(graph)) = (a.module_packages.get(m), a.graph.as_ref()) {
            let package = &graph.package(*pid).manifest.package;
            return format!("package {} {}", package.name, package.version);
        }
    }
    format!("module {stem}")
}

/// The `///` docs of `target`'s declaration (spec §5.1).
fn docs_of(a: &Analysis, target: &Target) -> String {
    if !matches!(
        target,
        Target::Def(_) | Target::Field(..) | Target::Variant(..) | Target::TraitMethod(..)
    ) {
        return String::new();
    }
    let Some(decl) = a.index.as_ref().and_then(|i| i.declaration(target)) else {
        return String::new();
    };
    let Some(text) = a.db.get_source(decl.span.file) else {
        return String::new();
    };
    let at = decl.span.start as usize;
    // A field or variant has docs only when it starts its line; one on its
    // record's or sum's own line would take the item's.
    if matches!(target, Target::Field(..) | Target::Variant(..)) {
        let line_start = text[..at].rfind('\n').map_or(0, |i| i + 1);
        if !text[line_start..at]
            .chars()
            .all(|c| c.is_whitespace() || c == '|')
        {
            return String::new();
        }
    }
    docs_above(text, at)
}

/// The `///` lines directly above the line holding byte `at`, attributes
/// (`@…`) skipped, each without `///` and one following space.
fn docs_above(text: &str, at: usize) -> String {
    let mut lines: Vec<&str> = Vec::new();
    let mut end = text[..at].rfind('\n').map_or(0, |i| i + 1);
    while end > 0 {
        let start = text[..end - 1].rfind('\n').map_or(0, |i| i + 1);
        let line = text[start..end - 1].trim_end_matches('\r').trim();
        match line.strip_prefix("///") {
            // `////` is an ordinary comment.
            Some(doc) if !doc.starts_with('/') => lines.push(doc.strip_prefix(' ').unwrap_or(doc)),
            Some(_) => break,
            None if line.starts_with('@') => {}
            None => break,
        }
        end = start;
    }
    lines.reverse();
    lines.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn docs_above_skip_attributes_and_stop_at_code() {
        let text = "fn a() {}\n/// One.\n///\n/// Two.\n@test\nfn b() {}\n";
        let at = text.find("fn b").unwrap();
        assert_eq!(docs_above(text, at), "One.\n\nTwo.");
        let at = text.find("fn a").unwrap();
        assert_eq!(docs_above(text, at), "");
        let crlf = "//// not a doc\r\n/// Doc.\r\nfn c() {}\r\n";
        assert_eq!(docs_above(crlf, crlf.find("fn c").unwrap()), "Doc.");
    }

    #[test]
    fn an_item_line_keeps_pub_async_and_where_and_stops_at_its_body() {
        let text = "    pub async fn f<T>(x: T) -> T\n    where T: Show\n    { x }\n";
        let at = text.find("f<").unwrap();
        assert_eq!(item_line(text, at, &['{', ';']), "pub async fn f<T>(x: T) -> T where T: Show");
    }

    #[test]
    fn a_type_parameters_bounds_stop_at_its_comma_or_angle() {
        let text = "fn f<T: Into<Vec<Int>>, U>() {}";
        let at = text.find("T:").unwrap();
        assert_eq!(collapse(&text[at..end_of(text, at, &[',', '>'], true)]), "T: Into<Vec<Int>>");
        let at = text.find("U>").unwrap();
        assert_eq!(collapse(&text[at..end_of(text, at, &[',', '>'], true)]), "U");
    }

    #[test]
    fn a_local_without_a_type_shows_its_annotation() {
        let text = "fn scaled(self, factor: Int) -> Int";
        let at = text.find("factor").unwrap();
        assert_eq!(local_line(text, at, "factor", None), "let factor: Int");
        let text = "fn f(mut x: [Int])";
        let at = text.find("x:").unwrap();
        assert_eq!(local_line(text, at, "x", None), "let mut x: [Int]");
        assert_eq!(local_line("let y = 1", 4, "y", None), "let y");
    }
}
```

- [ ] **Step 5: Serve hover**

In `crates/nova-lsp/src/lib.rs`:
- Add `mod analysis;` and `mod hover;` to the `mod` list.
- Add `HoverRequest` to the `use lsp_types::request::{…}` line.
- Add `use analysis::Answer;`.
- In `capabilities()`, add `hover_provider:
  Some(lsp::HoverProviderCapability::Simple(true)),`.
- In `Server::request`'s `match`, before the `_ =>` arm, add:

```rust
            HoverRequest::METHOD => {
                match serde_json::from_value::<lsp::HoverParams>(request.params) {
                    Ok(p) => {
                        let found = self
                            .answer_at(&p.text_document_position_params)
                            .and_then(|(answer, offset)| hover::hover(&answer, offset));
                        Response::new_ok(request.id, found)
                    }
                    Err(e) => invalid(request.id, e),
                }
            }
```

- Add to `impl Server<'_>`:

```rust
    /// The analysis that answers a request at `at`, with the index on, and
    /// the request's byte offset (spec 3.4a §4). `None` for a document
    /// that is not open.
    fn answer_at(&self, at: &lsp::TextDocumentPositionParams) -> Option<(Answer, u32)> {
        let doc = self.workspace.get(&uri::text(&at.text_document.uri))?;
        let offset = LineIndex::new(&doc.text).offset(at.position.line, at.position.character);
        let options = analysis::options(None, true);
        let answer = analysis::answering(&doc.path, &self.workspace.overlay(), &options)?;
        Some((answer, offset))
    }
```

- After `fn capabilities()`, add:

```rust
/// The response to a request whose parameters do not parse.
fn invalid(id: lsp_server::RequestId, e: serde_json::Error) -> Response {
    Response::new_err(id, ErrorCode::InvalidParams as i32, e.to_string())
}
```

- [ ] **Step 6: Run the tests to see them pass**

Run: `cd /d/Projects/nona/nova && cargo test -p nova-lsp 2>&1 | tail -8`
Expected: hover.rs's four unit tests pass with the crate's others.

Run: `cd /d/Projects/nona/nova && cargo test -p nova-cli --test lsp_navigation 2>&1 > $P/t7.txt; tail -12 $P/t7.txt`
Expected: `test result: ok. 6 passed; 0 failed`.

Completion's tests guard the moved `analysis_at`:

Run: `cd /d/Projects/nona/nova && cargo test -p nova-cli --test lsp 2>&1 > $P/t7b.txt; python -X utf8 $P/count.py $P/t7b.txt`
Expected: `0 failed`, and as many passed as before this task.

- [ ] **Step 7: Commit**

Write `$P/msg-7.txt`:

```text
nova-lsp: hover

Hover shows a name's declaration on one line, as the source writes it,
then its `///` docs as Markdown (spec 3.4a §5.1): a local with its
inferred type, a type parameter with its bounds, a module, a package
with its version, a builtin with its signature. Which analysis answers
a request moves to analysis.rs, shared with completion.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
```

Run: `cd /d/Projects/nona/nova && cargo fmt --all && git add crates/nova-lsp crates/nova-typeck crates/nova-cli/tests/lsp_navigation.rs && git commit -q -F $P/msg-7.txt && git log -1 --format=%s`
Expected: `nova-lsp: hover`

---

### Task 8: Std's cache, and go to definition

**Files:**
- Create: `crates/nova-lsp/src/std_cache.rs`, `crates/nova-lsp/src/navigate.rs`
- Modify: `crates/nova-lsp/src/analysis.rs` (`Scope::Std`), `src/checker.rs` (`in_the_cache`), `src/lib.rs`
- Modify: `crates/nova-lsp/Cargo.toml` (`crc32fast`), `Cargo.lock`
- Modify: `crates/nova-cli/tests/lsp_navigation.rs`

**Interfaces:**
- Consumes: Task 7's `Answer`, `answering`, `Overlay::with`,
  `Workspace::by_path`; `FileDb::id_of` (Task 6).
- Produces:
  - `std_cache::dir(home: &Path) -> PathBuf`;
  - `std_cache::ensure() -> Option<PathBuf>`;
  - `std_cache::path_of(short: &str) -> Option<PathBuf>`;
  - `std_cache::module_of(path: &Path) -> Option<String>`;
  - `std_cache::in_std_cache(path: &Path) -> bool`;
  - `analysis::Scope::Std { module: String }`;
  - `navigate::path_of(a: &Analysis, file: FileId) -> Option<PathBuf>`;
  - `navigate::location(a: &Analysis, span: Span, uri_of: &dyn Fn(&Path) -> String) -> Option<lsp::Location>`;
  - `navigate::definition(answer: &Answer, offset: u32, uri_of: &dyn Fn(&Path) -> String) -> Option<lsp::Location>`;
  - `Server::uri_of(&self, path: &Path) -> String`.

- [ ] **Step 1: Write the failing tests**

Remove the `#[allow(unused_imports)]` from the tests file's `use
lsp_client::…` line, and append:

```rust
// === Task 8: std's cache, and go to definition ===

fn definition(client: &mut Client, uri: &str, position: Value) -> Value {
    client.request(
        "textDocument/definition",
        json!({ "textDocument": { "uri": uri }, "position": position }),
    )
}

/// A location's path, from its `file:` URI.
fn path_of(location: &Value) -> std::path::PathBuf {
    let uri = location["uri"].as_str().unwrap_or_else(|| panic!("no location: {location}"));
    let decoded = lsp_client::decode(uri.strip_prefix("file://").unwrap());
    let trimmed = if decoded.as_bytes().get(2) == Some(&b':') {
        &decoded[1..]
    } else {
        &decoded[..]
    };
    std::path::PathBuf::from(trimmed)
}

/// The text a location's range covers, read from its file.
fn text_at(location: &Value) -> String {
    let text = std::fs::read_to_string(path_of(location)).unwrap();
    let range = &location["range"];
    let offset = |p: &Value| {
        let line = p["line"].as_u64().unwrap() as usize;
        let character = p["character"].as_u64().unwrap() as usize;
        let start = text.split_inclusive('\n').take(line).map(str::len).sum::<usize>();
        let rest = &text[start..];
        let mut units = 0;
        for (i, c) in rest.char_indices() {
            if units >= character {
                return start + i;
            }
            units += c.len_utf16();
        }
        text.len()
    };
    text[offset(&range["start"])..offset(&range["end"])].to_string()
}

const MAIN_IMPORTS_GEOMETRY: &str =
    "import geometry\n\nfn main() {\n    let total = area()\n    let again = total\n    print(\"x\")\n}\n";
const GEOMETRY: &str = "/// One.\npub fn area() -> Int { 1 }\n";

#[test]
fn definition_in_the_same_file_and_into_another_module() {
    let dir = project(
        "definition-modules",
        &[("main.nova", MAIN_IMPORTS_GEOMETRY), ("geometry.nova", GEOMETRY)],
    );
    let main = dir.join("src").join("main.nova");
    let uri = file_uri(&main);
    let mut client = Client::start(&dir, false);
    open(&mut client, &uri, MAIN_IMPORTS_GEOMETRY);
    let local = definition(&mut client, &uri, at(MAIN_IMPORTS_GEOMETRY, "total", 1))["result"].clone();
    assert!(same_uri(local["uri"].as_str().unwrap(), &uri), "{local}");
    assert_eq!(local["range"], range(MAIN_IMPORTS_GEOMETRY, "total", 0, "total"));
    let area = definition(&mut client, &uri, at(MAIN_IMPORTS_GEOMETRY, "area()", 0))["result"].clone();
    assert!(
        same_uri(area["uri"].as_str().unwrap(), &file_uri(&dir.join("src").join("geometry.nova"))),
        "{area}"
    );
    assert_eq!(text_at(&area), "area");
    // `import geometry` goes to its file's start.
    let module = definition(&mut client, &uri, at(MAIN_IMPORTS_GEOMETRY, "geometry", 0))["result"].clone();
    assert!(path_of(&module).ends_with("geometry.nova"), "{module}");
    assert_eq!(module["range"]["start"], json!({ "line": 0, "character": 0 }));
    // A builtin has no definition.
    assert_eq!(definition(&mut client, &uri, at(MAIN_IMPORTS_GEOMETRY, "print", 0))["result"], Value::Null);
}

#[test]
fn definition_into_a_path_dependency_is_at_its_real_path() {
    let (app, geom) = app_and_library("definition-dependency", GEOMETRY);
    let main = file_uri(&app.join("src").join("main.nova"));
    let mut client = Client::start(&app, false);
    open(&mut client, &main, APP_MAIN);
    let found = definition(&mut client, &main, at(APP_MAIN, "area()", 0))["result"].clone();
    // Not `app/../geom/...`.
    assert!(
        same_uri(found["uri"].as_str().unwrap(), &file_uri(&geom.join("src").join("lib.nova"))),
        "{found}"
    );
    assert_eq!(text_at(&found), "area");
}

/// A fresh `NOVA_HOME`.
fn home(name: &str) -> std::path::PathBuf {
    fresh_dir(&format!("{name}-home"))
}

const USES_STD: &str = "fn main() {\n    let mut v = Vec::new()\n    v.push(1)\n}\n";

#[test]
fn definition_into_std_opens_its_cache() {
    let home = home("definition-std");
    let dir = project("definition-std", &[("main.nova", USES_STD)]);
    let uri = file_uri(&dir.join("src").join("main.nova"));
    let mut client = Client::start_with_env(&dir, false, &[("NOVA_HOME", &home)]);
    open(&mut client, &uri, USES_STD);
    let found = definition(&mut client, &uri, at(USES_STD, "push", 0))["result"].clone();
    let file = path_of(&found);
    assert!(file.starts_with(home.join("std")), "{}", file.display());
    assert_eq!(file.file_name().unwrap(), "collections.nova");
    assert_eq!(text_at(&found), "push");
    assert!(std::fs::metadata(&file).unwrap().permissions().readonly());
    // Every module is there.
    let dir_of = file.parent().unwrap();
    for name in ["core", "json", "test"] {
        assert!(dir_of.join(format!("{name}.nova")).is_file(), "{name}");
    }
}

#[test]
fn std_cache_is_reused_and_a_damaged_file_is_replaced() {
    let home = home("std-cache-reuse");
    let dir = project("std-cache-reuse", &[("main.nova", USES_STD)]);
    let uri = file_uri(&dir.join("src").join("main.nova"));
    let mut client = Client::start_with_env(&dir, false, &[("NOVA_HOME", &home)]);
    open(&mut client, &uri, USES_STD);
    let file = path_of(&definition(&mut client, &uri, at(USES_STD, "push", 0))["result"]);
    let written = std::fs::metadata(&file).unwrap().modified().unwrap();
    let text = std::fs::read_to_string(&file).unwrap();
    definition(&mut client, &uri, at(USES_STD, "push", 0));
    assert_eq!(std::fs::metadata(&file).unwrap().modified().unwrap(), written);
    // Damage it: the next request replaces it.
    let mut perms = std::fs::metadata(&file).unwrap().permissions();
    #[allow(clippy::permissions_set_readonly_false)]
    perms.set_readonly(false);
    std::fs::set_permissions(&file, perms).unwrap();
    std::fs::write(&file, "damaged").unwrap();
    definition(&mut client, &uri, at(USES_STD, "push", 0));
    assert_eq!(std::fs::read_to_string(&file).unwrap(), text);
}

#[test]
fn two_servers_write_std_cache_at_once() {
    let home = home("std-cache-two");
    let dir = project("std-cache-two", &[("main.nova", USES_STD)]);
    let uri = file_uri(&dir.join("src").join("main.nova"));
    let mut a = Client::start_with_env(&dir, false, &[("NOVA_HOME", &home)]);
    let mut b = Client::start_with_env(&dir, false, &[("NOVA_HOME", &home)]);
    open(&mut a, &uri, USES_STD);
    open(&mut b, &uri, USES_STD);
    let (ra, rb) = std::thread::scope(|s| {
        let ta = s.spawn(|| definition(&mut a, &uri, at(USES_STD, "push", 0)));
        let tb = s.spawn(|| definition(&mut b, &uri, at(USES_STD, "push", 0)));
        (ta.join().unwrap(), tb.join().unwrap())
    });
    for found in [&ra["result"], &rb["result"]] {
        assert_eq!(text_at(found), "push", "{found}");
    }
}

#[test]
fn a_request_inside_std_cache_is_answered_and_nothing_is_published() {
    let home = home("std-cache-inside");
    let dir = project("std-cache-inside", &[("main.nova", USES_STD)]);
    let uri = file_uri(&dir.join("src").join("main.nova"));
    let mut client = Client::start_with_env(&dir, false, &[("NOVA_HOME", &home)]);
    open(&mut client, &uri, USES_STD);
    let file = path_of(&definition(&mut client, &uri, at(USES_STD, "push", 0))["result"]);
    let text = std::fs::read_to_string(&file).unwrap();
    let std_uri = file_uri(&file);
    client.clear_unread();
    open(&mut client, &std_uri, &text);
    // Hover on `Vec`'s name, in its declaration inside std's own file.
    let start = text.find("record Vec").unwrap() + "record ".len();
    let response = hover(&mut client, &std_uri, position(&text, start));
    assert!(code(&response).contains("record Vec"), "{response}");
    // Nothing is published for it.
    let sentinel_dir = fresh_dir("std-cache-inside-sentinel");
    let sentinel = file_uri(&sentinel_dir.join("main.nova"));
    std::fs::write(sentinel_dir.join("main.nova"), "fn main() {}\n").unwrap();
    open(&mut client, &sentinel, "fn main() {}\n");
    assert_eq!(client.last_diagnostics_before(&std_uri, &sentinel), None);
}

// Review Focus 2.
const WIDE: &str = "fn main() {\n    let s = \"ก😀\"; let total = 1\n    let b = \"ก😀\"; let c = total\n}\n";

#[test]
fn ranges_count_utf16_after_thai_and_an_emoji() {
    let dir = project("definition-utf16", &[("main.nova", WIDE)]);
    let uri = file_uri(&dir.join("src").join("main.nova"));
    let mut client = Client::start(&dir, false);
    open(&mut client, &uri, WIDE);
    let found = definition(&mut client, &uri, at(WIDE, "total", 1))["result"].clone();
    assert_eq!(found["range"], range(WIDE, "total", 0, "total"));
    let response = hover(&mut client, &uri, at(WIDE, "total", 1));
    assert_eq!(response["result"]["range"], range(WIDE, "total", 1, "total"));
}
```

`definition_into_a_downloaded_package` comes with Task 8's copy of 3.3b's
helpers. Append:

```rust
// === Phase 3.3b's registry helpers, as in lsp.rs ===

const INDEX: &str = "https://example.test/index/";

/// `dir/app`, which depends on `geom = "<req>"` and runs `APP_MAIN`, and
/// `dir/home`, the server's `NOVA_HOME`.
fn registry_app(name: &str, req: &str) -> (std::path::PathBuf, std::path::PathBuf) {
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
fn lock_and_cache(app: &std::path::Path, home: &std::path::Path, lib: &str) -> std::path::PathBuf {
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

#[test]
fn definition_into_a_downloaded_package() {
    let (app, home) = registry_app("definition-registry", "0.1");
    let geom = lock_and_cache(&app, &home, GEOMETRY);
    let main = file_uri(&app.join("src").join("main.nova"));
    let mut client = Client::start_with_env(&app, false, &[("NOVA_HOME", &home)]);
    open(&mut client, &main, APP_MAIN);
    let found = definition(&mut client, &main, at(APP_MAIN, "area()", 0))["result"].clone();
    assert!(
        same_uri(found["uri"].as_str().unwrap(), &file_uri(&geom.join("src").join("lib.nova"))),
        "{found}"
    );
    assert_eq!(text_at(&found), "area");
}
```

In `crates/nova-cli/tests/lsp_client/mod.rs`, make `fn decode(` public
(`pub fn decode(`), for `path_of`.

- [ ] **Step 2: Run the tests to see them fail**

Run: `cd /d/Projects/nona/nova && cargo test -p nova-cli --test lsp_navigation 2>&1 > $P/t8.txt; tail -30 $P/t8.txt`
Expected: Task 7's six pass; the eight new tests fail, each on a
`MethodNotFound` response ("no location").

- [ ] **Step 3: Std's cache**

In `crates/nova-lsp/Cargo.toml`, add under `[dependencies]`:

```toml
# Std's cache's directory name (spec 3.4a §6). Already in the lockfile.
crc32fast = { workspace = true }
```

Create `crates/nova-lsp/src/std_cache.rs`:

```rust
//! Std's sources on disk, so that a definition can open them (spec
//! `docs/superpowers/specs/2026-10-10-phase-3-4a-navigation-design.md`
//! §6).
//!
//! `$NOVA_HOME/std/<version>-<crc32>/`, one read-only file per module. A
//! file is compared byte for byte with the embedded text and rewritten
//! through a unique temporary name when it differs.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

use crate::workspace::PathKey;

/// Each std module's short name, as in `<std/core>`, and its text, in
/// `STD_MODULES` order then `std/test`.
fn sources() -> Vec<(&'static str, &'static str)> {
    nova_resolver::STD_MODULES
        .iter()
        .chain(std::iter::once(&nova_resolver::STD_TEST_MODULE))
        .map(|&(name, text)| (name.strip_prefix("$std.").unwrap_or(name), text))
        .collect()
}

fn crc() -> u32 {
    let mut hasher = crc32fast::Hasher::new();
    for (_, text) in sources() {
        hasher.update(text.as_bytes());
    }
    hasher.finalize()
}

/// The cache's directory under `home`. The version is the server's, the
/// one `serverInfo` reports; the CRC-32 keeps two builds apart.
pub fn dir(home: &Path) -> PathBuf {
    home.join("std")
        .join(format!("{}-{:08x}", env!("CARGO_PKG_VERSION"), crc()))
}

/// The cache, written where it is not already right. `None`, after a
/// warning logged once per server, when there is no `$NOVA_HOME` or a
/// write fails.
pub fn ensure() -> Option<PathBuf> {
    let Some(home) = nova_pm::nova_home_from_env() else {
        warn_once("there is no NOVA_HOME and no home directory");
        return None;
    };
    let dir = dir(&home);
    match write_all(&dir) {
        Ok(()) => Some(dir),
        Err(e) => {
            warn_once(&format!("cannot write {}: {e}", dir.display()));
            None
        }
    }
}

/// The cached file of std module `short`, written if need be.
pub fn path_of(short: &str) -> Option<PathBuf> {
    Some(ensure()?.join(format!("{short}.nova")))
}

/// The std module a path in the cache holds, by its file name.
pub fn module_of(path: &Path) -> Option<String> {
    if !in_std_cache(path) {
        return None;
    }
    let stem = path.file_stem()?.to_str()?;
    sources()
        .iter()
        .any(|(short, _)| *short == stem)
        .then(|| stem.to_string())
}

/// Whether `path` is under `$NOVA_HOME/std/`.
pub fn in_std_cache(path: &Path) -> bool {
    nova_pm::nova_home_from_env()
        .is_some_and(|home| PathKey::of(path).is_under(&PathKey::of(&home.join("std"))))
}

fn write_all(dir: &Path) -> std::io::Result<()> {
    std::fs::create_dir_all(dir)?;
    for (short, text) in sources() {
        write_one(&dir.join(format!("{short}.nova")), text)?;
    }
    Ok(())
}

static TEMPS: AtomicU64 = AtomicU64::new(0);

/// Make `path` hold `text`, read-only.
fn write_one(path: &Path, text: &str) -> std::io::Result<()> {
    let right = |p: &Path| std::fs::read(p).is_ok_and(|bytes| bytes == text.as_bytes());
    if !right(path) {
        let temp = path.with_extension(format!(
            "nova.tmp-{}-{}",
            std::process::id(),
            TEMPS.fetch_add(1, Ordering::Relaxed)
        ));
        {
            let mut file = std::fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&temp)?;
            file.write_all(text.as_bytes())?;
        }
        // Windows refuses to rename over a read-only file.
        #[cfg(windows)]
        if path.exists() {
            let _ = set_read_only(path, false);
        }
        if let Err(e) = std::fs::rename(&temp, path) {
            let _ = std::fs::remove_file(&temp);
            // Another server may have written it first.
            if !right(path) {
                return Err(e);
            }
        }
    }
    set_read_only(path, true)
}

fn set_read_only(path: &Path, on: bool) -> std::io::Result<()> {
    let mut perms = std::fs::metadata(path)?.permissions();
    if perms.readonly() != on {
        perms.set_readonly(on);
        std::fs::set_permissions(path, perms)?;
    }
    Ok(())
}

static WARNED: AtomicBool = AtomicBool::new(false);

fn warn_once(why: &str) {
    if !WARNED.swap(true, Ordering::Relaxed) {
        tracing::warn!("nova lsp: std's sources are not on disk, so no location in std: {why}");
    }
}
```

On Unix, `set_readonly(false)` would make the file writable by everyone,
so it is called only on Windows. Clippy's
`permissions_set_readonly_false` lint fires only on a literal `false`.

In `crates/nova-lsp/src/checker.rs`, change `in_the_cache` to:

```rust
/// Whether `project` is inside one of nova's caches: downloaded packages
/// (spec 3.3b §5.5), or std's sources (3.4a §6).
fn in_the_cache(project: &ProjectKey) -> bool {
    let path = match project {
        ProjectKey::Root(dir) => dir.as_path(),
        ProjectKey::Loose(file) => file.as_path(),
    };
    if crate::std_cache::in_std_cache(path) {
        return true;
    }
    let Some(registry) = nova_pm::registry_dir() else {
        return false;
    };
    PathKey::of(path).is_under(&PathKey::of(&registry))
}
```

At the end of `checker.rs`, add a tests module (or add to the one there,
if there is one). This is the test spec §8.5's tenth mutant breaks:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_diagnostics_check_leaves_the_index_off() {
        // A guard for spec 3.4a §3.6: diagnostics never pay for the index.
        let path = std::env::temp_dir()
            .join("nova-lsp-checker-index")
            .join("main.nova");
        let overlay = Overlay::default().with(&path, "fn main() {}\n".to_string());
        let a = run(Program::loose(&path), &overlay, true)
            .ok()
            .expect("the buffer is analysed");
        assert!(a.index.is_none());
    }
}
```

Run: `cd /d/Projects/nona/nova && cargo test -p nova-lsp the_diagnostics_check 2>&1 | tail -5`
Expected: `1 passed`.

- [ ] **Step 4: Requests inside std's cache**

In `crates/nova-lsp/src/analysis.rs`, add the variant:

```rust
    /// A file in std's cache (spec §4, rule 2), answered from an in-memory
    /// program that includes std.
    Std { module: String },
```

In `analyse`, change the `match scope` so the `Std` arm builds the
in-memory program:

```rust
pub fn analyse(scope: &Scope, overlay: &Overlay, options: &Options) -> Option<Analysis> {
    let (program, overlay) = match scope {
        Scope::Project(dir) => (Program::for_package(dir, Roots::Test), overlay.clone()),
        Scope::File(path) => (Program::for_file(path), overlay.clone()),
        Scope::Std { .. } => {
            // Plan decision 14: `fn main() {}`, which exists only in the
            // overlay.
            let entry = std::env::temp_dir().join("nova-lsp-std").join("main.nova");
            let overlay = overlay.with(&entry, "fn main() {}\n".to_string());
            (Program::loose(&entry), overlay)
        }
    };
    guarded(|| analyze_program(program, &overlay, options).ok())
}
```

and at the start of `answering`:

```rust
    if let Some(module) = crate::std_cache::module_of(path) {
        let scope = Scope::Std {
            module: module.clone(),
        };
        let analysis = analyse(&scope, overlay, options)?;
        let file = analysis.db.id_of(&format!("<std/{module}>"))?;
        return Some(Answer {
            analysis,
            file,
            scope,
        });
    }
```

- [ ] **Step 5: Definition**

Create `crates/nova-lsp/src/navigate.rs`:

```rust
//! Go to definition and find references (spec
//! `docs/superpowers/specs/2026-10-10-phase-3-4a-navigation-design.md`
//! §5.2, §5.3).

use std::path::{Path, PathBuf};

use lsp_types as lsp;
use nova_diagnostics::{FileId, LineIndex, Span};
use nova_driver::Analysis;
use nova_resolver::Target;

use crate::analysis::Answer;
use crate::{convert, std_cache, uri};

/// Where a file of `a` is on disk: a std module's place in std's cache, or
/// the real path of anything else, so `app/../geom` reads as `geom` (plan
/// decision 8).
pub fn path_of(a: &Analysis, file: FileId) -> Option<PathBuf> {
    let name = a.db.get_name(file)?;
    match name.strip_prefix("<std/").and_then(|s| s.strip_suffix('>')) {
        Some(short) => std_cache::path_of(short),
        None => Some(nova_pm::real_path(Path::new(name))),
    }
}

/// `span`'s location, under the URI `uri_of` gives its file.
pub fn location(a: &Analysis, span: Span, uri_of: &dyn Fn(&Path) -> String) -> Option<lsp::Location> {
    let path = path_of(a, span.file)?;
    let source = a.db.get_source(span.file)?;
    let range = convert::range(&LineIndex::new(source), span.start, span.end);
    Some(lsp::Location {
        uri: uri::parse(&uri_of(&path))?,
        range,
    })
}

/// The definition of the name at byte `offset` (spec §5.2).
pub fn definition(
    answer: &Answer,
    offset: u32,
    uri_of: &dyn Fn(&Path) -> String,
) -> Option<lsp::Location> {
    let a = &answer.analysis;
    let index = a.index.as_ref()?;
    let o = index.at(a.definitions.as_ref()?, answer.file, offset)?;
    match o.target {
        Target::Builtin(_) | Target::BuiltinMethod(_) | Target::Primitive(_) => None,
        Target::Module(m) => {
            let (file, _) = a.modules.get(m.0 as usize)?;
            location(a, Span::new(0, 0, *file), uri_of)
        }
        target => location(a, index.declaration(&target)?.span, uri_of),
    }
}
```

In `crates/nova-lsp/src/lib.rs`:
- Add `mod navigate;` and `mod std_cache;` to the `mod` list.
- Add `GotoDefinition` to the request imports.
- In `capabilities()`, add `definition_provider: Some(lsp::OneOf::Left(true)),`.
- Add to `impl Server<'_>`:

```rust
    /// The URI for `path`: the open document's, so a Windows client gets
    /// its own spelling back, else one made from the path (3.2's rule).
    fn uri_of(&self, path: &Path) -> String {
        match self.workspace.by_path(path) {
            Some(doc) => doc.uri.clone(),
            None => uri::from_path(path),
        }
    }
```

- Before the `_ =>` arm of `Server::request`, add:

```rust
            GotoDefinition::METHOD => {
                match serde_json::from_value::<lsp::GotoDefinitionParams>(request.params) {
                    Ok(p) => {
                        let found = self
                            .answer_at(&p.text_document_position_params)
                            .and_then(|(answer, offset)| {
                                navigate::definition(&answer, offset, &|path| self.uri_of(path))
                            })
                            .map(lsp::GotoDefinitionResponse::Scalar);
                        Response::new_ok(request.id, found)
                    }
                    Err(e) => invalid(request.id, e),
                }
            }
```

- [ ] **Step 6: Run the tests to see them pass**

Run: `cd /d/Projects/nona/nova && cargo check -p nova-lsp 2>&1 | tail -3 && git diff --stat Cargo.lock`
Expected: `Finished`; `Cargo.lock` gains one line, `crc32fast` under
`nova-lsp`'s dependencies, and no new package.

Run: `cd /d/Projects/nona/nova && cargo test -p nova-cli --test lsp_navigation 2>&1 > $P/t8.txt; tail -12 $P/t8.txt`
Expected: `test result: ok. 14 passed; 0 failed`.

Run: `cd /d/Projects/nona/nova && cargo test -p nova-cli --test lsp 2>&1 > $P/t8b.txt; python -X utf8 $P/count.py $P/t8b.txt`
Expected: `0 failed`. `a_file_in_the_cache_gets_nothing_published` guards
`in_the_cache`.

- [ ] **Step 7: Commit**

Write `$P/msg-8.txt`:

```text
nova-lsp: go to definition, and std's sources on disk

A definition goes to the name's declaration in the same file, another
module, a path dependency at its real path, a downloaded package, or
std (spec 3.4a §5.2). std exists only inside nova, so its sources are
written once to $NOVA_HOME/std/<version>-<crc32>/, read-only, each file
compared with the embedded text and rewritten through a temporary name
when it differs (§6). A request inside that cache is answered from a
program held in memory, and nothing is published for its files.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
```

Run: `cd /d/Projects/nona/nova && cargo fmt --all && git add Cargo.lock crates/nova-lsp crates/nova-cli/tests && git commit -q -F $P/msg-8.txt && git log -1 --format=%s`
Expected: `nova-lsp: go to definition, and std's sources on disk`

---

### Task 9: Find references

**Files:**
- Modify: `crates/nova-lsp/src/navigate.rs` (`references`, `sort_by_place`), `crates/nova-lsp/src/lib.rs`
- Modify: `crates/nova-cli/tests/lsp_navigation.rs`

**Interfaces:**
- Consumes: Task 8's `location`, `Server::uri_of`; Task 1's `family`.
- Produces:
  - `navigate::references(answer: &Answer, offset: u32, declarations: bool, uri_of: &dyn Fn(&Path) -> String) -> Vec<lsp::Location>`;
  - `navigate::sort_by_place(a: &Analysis, found: &mut [&Occurrence])`,
    which Tasks 10 and 11 use.

- [ ] **Step 1: Write the failing tests**

Append to `crates/nova-cli/tests/lsp_navigation.rs`:

```rust
// === Task 9: references ===

fn references(client: &mut Client, uri: &str, position: Value, declarations: bool) -> Vec<Value> {
    let response = client.request(
        "textDocument/references",
        json!({
            "textDocument": { "uri": uri },
            "position": position,
            "context": { "includeDeclaration": declarations },
        }),
    );
    response["result"]
        .as_array()
        .unwrap_or_else(|| panic!("no references: {response}"))
        .clone()
}

/// Each location as `(file name, line, character)`.
fn places(locations: &[Value]) -> Vec<(String, u64, u64)> {
    locations
        .iter()
        .map(|l| {
            let name = path_of(l).file_name().unwrap().to_string_lossy().into_owned();
            let start = &l["range"]["start"];
            (name, start["line"].as_u64().unwrap(), start["character"].as_u64().unwrap())
        })
        .collect()
}

#[test]
fn references_with_and_without_the_declaration() {
    let dir = project("references-local", &[("main.nova", MAIN_IMPORTS_GEOMETRY), ("geometry.nova", GEOMETRY)]);
    let uri = file_uri(&dir.join("src").join("main.nova"));
    let mut client = Client::start(&dir, false);
    open(&mut client, &uri, MAIN_IMPORTS_GEOMETRY);
    let with = references(&mut client, &uri, at(MAIN_IMPORTS_GEOMETRY, "total", 1), true);
    assert_eq!(places(&with), [("main.nova".to_string(), 3, 8), ("main.nova".to_string(), 4, 16)]);
    let without = references(&mut client, &uri, at(MAIN_IMPORTS_GEOMETRY, "total", 0), false);
    assert_eq!(places(&without), [("main.nova".to_string(), 4, 16)]);
    // Across files, in file then offset order.
    let area = references(&mut client, &uri, at(MAIN_IMPORTS_GEOMETRY, "area()", 0), true);
    let names: Vec<String> = places(&area).into_iter().map(|p| p.0).collect();
    assert_eq!(names.len(), 2, "{area:?}");
    let mut sorted = area.clone();
    sorted.sort_by_key(|l| path_of(l));
    assert_eq!(places(&sorted), places(&area));
}

#[test]
fn references_from_the_app_reach_into_its_dependency() {
    let lib = "pub fn area() -> Int { 1 }\npub fn twice() -> Int { area() + area() }\n";
    let (app, _geom) = app_and_library("references-dependency", lib);
    let main = file_uri(&app.join("src").join("main.nova"));
    let mut client = Client::start(&app, false);
    open(&mut client, &main, APP_MAIN);
    let found = references(&mut client, &main, at(APP_MAIN, "area()", 0), true);
    let mut counts = std::collections::BTreeMap::new();
    for (name, _, _) in places(&found) {
        *counts.entry(name).or_insert(0) += 1;
    }
    // The declaration and two uses in lib.nova, the call in main.nova.
    assert_eq!(counts.get("lib.nova"), Some(&3), "{found:?}");
    assert_eq!(counts.get("main.nova"), Some(&1), "{found:?}");
}

const FAMILY: &str = "trait Show { fn show(self) -> String }\n\
record A { n: Int }\nrecord B { n: Int }\n\
impl Show for A { fn show(self) -> String { \"a\" } }\n\
impl Show for B { fn show(self) -> String { \"b\" } }\n\
fn main() {\n    let a = A { n: 1 }\n    let b = B { n: 2 }\n    let s = a.show()\n    let t = b.show()\n}\n";

#[test]
fn a_trait_methods_family_is_found_from_each_member() {
    let dir = project("references-family", &[("main.nova", FAMILY)]);
    let uri = file_uri(&dir.join("src").join("main.nova"));
    let mut client = Client::start(&dir, false);
    open(&mut client, &uri, FAMILY);
    // The trait's declaration, two impls' and two calls.
    let from_trait = places(&references(&mut client, &uri, at(FAMILY, "show", 0), true));
    assert_eq!(from_trait.len(), 5, "{from_trait:?}");
    let from_impl = places(&references(&mut client, &uri, at(FAMILY, "show", 1), true));
    let from_call = places(&references(&mut client, &uri, at(FAMILY, "show()", 1), true));
    assert_eq!(from_impl, from_trait);
    assert_eq!(from_call, from_trait);
}
```

- [ ] **Step 2: Run the tests to see them fail**

Run: `cd /d/Projects/nona/nova && cargo test -p nova-cli --test lsp_navigation references 2>&1 | tail -15`
Expected: the three fail with "no references: …" (a `MethodNotFound` error).

- [ ] **Step 3: References**

Append to `crates/nova-lsp/src/navigate.rs`:

```rust
/// The references to the name at byte `offset` (spec §5.3): every
/// occurrence of its target, or of its family, declarations only when
/// asked, in the spec's one order.
pub fn references(
    answer: &Answer,
    offset: u32,
    declarations: bool,
    uri_of: &dyn Fn(&Path) -> String,
) -> Vec<lsp::Location> {
    let a = &answer.analysis;
    let (Some(index), Some(defs)) = (a.index.as_ref(), a.definitions.as_ref()) else {
        return Vec::new();
    };
    let Some(o) = index.at(defs, answer.file, offset) else {
        return Vec::new();
    };
    let family = index.family(&o.target);
    let mut found: Vec<&Occurrence> = index
        .occurrences
        .iter()
        .filter(|x| family.contains(&x.target))
        .filter(|x| declarations || x.role == Role::Use)
        .collect();
    sort_by_place(a, &mut found);
    found.dedup_by_key(|x| x.span);
    found
        .iter()
        .filter_map(|x| location(a, x.span, uri_of))
        .collect()
}

/// Sort by file name in the database, then offset: the spec's one order
/// (plan decision 16).
pub fn sort_by_place(a: &Analysis, found: &mut [&Occurrence]) {
    found.sort_by(|x, y| {
        let name = |o: &Occurrence| a.db.get_name(o.span.file).unwrap_or("").to_string();
        (name(x), x.span.start).cmp(&(name(y), y.span.start))
    });
}
```

and change its `use nova_resolver::Target;` to `use nova_resolver::{Occurrence,
Role, Target};`.

In `crates/nova-lsp/src/lib.rs`, add `References` to the request imports,
`references_provider: Some(lsp::OneOf::Left(true)),` to `capabilities()`,
and before the `_ =>` arm:

```rust
            References::METHOD => {
                match serde_json::from_value::<lsp::ReferenceParams>(request.params) {
                    Ok(p) => {
                        let found = self
                            .answer_at(&p.text_document_position)
                            .map(|(answer, offset)| {
                                navigate::references(
                                    &answer,
                                    offset,
                                    p.context.include_declaration,
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

- [ ] **Step 4: Run the tests to see them pass**

Run: `cd /d/Projects/nona/nova && cargo test -p nova-cli --test lsp_navigation 2>&1 > $P/t9.txt; tail -8 $P/t9.txt`
Expected: `test result: ok. 17 passed; 0 failed`.

- [ ] **Step 5: Commit**

Write `$P/msg-9.txt`:

```text
nova-lsp: find references

Every occurrence of the name's target in the owning project's analysis,
declarations only when the client asks, by file name then offset (spec
3.4a §5.3). A trait method and the impl methods that implement it are
found together, from any of them. From an app, a dependency's own uses
are listed too.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
```

Run: `cd /d/Projects/nona/nova && cargo fmt --all && git add crates/nova-lsp crates/nova-cli/tests/lsp_navigation.rs && git commit -q -F $P/msg-9.txt && git log -1 --format=%s`
Expected: `nova-lsp: find references`

---

### Task 10: Prepare rename and rename

**Files:**
- Create: `crates/nova-lsp/src/rename.rs`
- Modify: `crates/nova-lsp/src/lib.rs`
- Modify: `crates/nova-cli/tests/lsp_navigation.rs`

**Interfaces:**
- Consumes: Tasks 8 and 9's `navigate::path_of`, `sort_by_place`;
  `std_cache::in_std_cache`; Task 1's `family`.
- Produces:
  - `rename::Refused(pub String)`;
  - `rename::prepare(answer: &Answer, path: &Path, offset: u32) -> Result<Option<lsp::PrepareRenameResponse>, Refused>`;
  - `rename::rename(answer: &Answer, path: &Path, offset: u32, new: &str, overlay: &Overlay, uri_of: &dyn Fn(&Path) -> String) -> Result<lsp::WorkspaceEdit, Refused>`;
  - `Edit` and `plan_edits`, which Task 11's check reads.

- [ ] **Step 1: Write the failing tests**

Append to `crates/nova-cli/tests/lsp_navigation.rs`:

```rust
// === Task 10: prepare rename and rename ===

fn prepare(client: &mut Client, uri: &str, position: Value) -> Value {
    client.request(
        "textDocument/prepareRename",
        json!({ "textDocument": { "uri": uri }, "position": position }),
    )
}

fn rename(client: &mut Client, uri: &str, position: Value, new: &str) -> Value {
    client.request(
        "textDocument/rename",
        json!({ "textDocument": { "uri": uri }, "position": position, "newName": new }),
    )
}

/// A refusal's message, after checking it is `RequestFailed`.
#[track_caller]
fn refused(response: &Value) -> String {
    assert_eq!(response["error"]["code"], -32803, "{response}");
    response["error"]["message"].as_str().unwrap().to_string()
}

/// A rename's edits as `(file name, line, character, new text)`, sorted.
#[track_caller]
fn edits(response: &Value) -> Vec<(String, u64, u64, String)> {
    let changes = response["result"]["changes"]
        .as_object()
        .unwrap_or_else(|| panic!("no edit: {response}"));
    let mut out = Vec::new();
    for (uri, list) in changes {
        let name = path_of(&json!({ "uri": uri })).file_name().unwrap().to_string_lossy().into_owned();
        for e in list.as_array().unwrap() {
            let start = &e["range"]["start"];
            out.push((
                name.clone(),
                start["line"].as_u64().unwrap(),
                start["character"].as_u64().unwrap(),
                e["newText"].as_str().unwrap().to_string(),
            ));
        }
    }
    out.sort();
    out
}

#[test]
fn prepare_rename_gives_the_range_and_spelling() {
    let dir = project("prepare-range", &[("main.nova", MAIN_IMPORTS_GEOMETRY), ("geometry.nova", GEOMETRY)]);
    let uri = file_uri(&dir.join("src").join("main.nova"));
    let mut client = Client::start(&dir, false);
    open(&mut client, &uri, MAIN_IMPORTS_GEOMETRY);
    let response = prepare(&mut client, &uri, at(MAIN_IMPORTS_GEOMETRY, "total", 1));
    assert_eq!(response["result"]["placeholder"], "total");
    assert_eq!(response["result"]["range"], range(MAIN_IMPORTS_GEOMETRY, "total", 1, "total"));
    // No name: null.
    assert_eq!(prepare(&mut client, &uri, json!({ "line": 1, "character": 0 }))["result"], Value::Null);
}

const REFUSALS: &str = "import geometry\n\
record P { n: Int }\n\
impl Display for P {\n    fn fmt(self) -> String { \"p\" }\n}\n\
fn main() {\n    let mut v = Vec::new()\n    v.push(1)\n    print(\"x\")\n    let n: Int = area()\n}\n";

#[test]
fn prepare_rename_refuses_what_is_not_the_projects() {
    let dir = project("prepare-refusals", &[("main.nova", REFUSALS), ("geometry.nova", GEOMETRY)]);
    let uri = file_uri(&dir.join("src").join("main.nova"));
    let home = home("prepare-refusals");
    let mut client = Client::start_with_env(&dir, false, &[("NOVA_HOME", &home)]);
    open(&mut client, &uri, REFUSALS);
    let cases: &[(&str, usize, &str)] = &[
        ("push", 0, "`push` is declared in std and cannot be renamed"),
        ("fmt", 0, "`fmt` is declared in std and cannot be renamed"),
        ("print", 0, "`print` is built in and cannot be renamed"),
        ("Int =", 0, "`Int` is built in and cannot be renamed"),
        ("self", 0, "`self` is a keyword and cannot be renamed"),
        ("geometry", 0, "a module or package is renamed by renaming its file or its `nova.toml`"),
    ];
    for (marker, n, message) in cases {
        let response = prepare(&mut client, &uri, at(REFUSALS, marker, *n));
        assert_eq!(refused(&response), *message, "at `{marker}`");
    }
    // A dependency's name.
    let (app, _geom) = app_and_library("prepare-dependency", GEOMETRY);
    let main = file_uri(&app.join("src").join("main.nova"));
    let mut client = Client::start(&app, false);
    open(&mut client, &main, APP_MAIN);
    assert_eq!(
        refused(&prepare(&mut client, &main, at(APP_MAIN, "area()", 0))),
        "`area` is declared in the dependency `geom` and cannot be renamed"
    );
}

#[test]
fn prepare_rename_refuses_inside_nova_s_caches() {
    let home = home("prepare-cache");
    let dir = project("prepare-cache", &[("main.nova", USES_STD)]);
    let uri = file_uri(&dir.join("src").join("main.nova"));
    let mut client = Client::start_with_env(&dir, false, &[("NOVA_HOME", &home)]);
    open(&mut client, &uri, USES_STD);
    let file = path_of(&definition(&mut client, &uri, at(USES_STD, "push", 0))["result"]);
    let text = std::fs::read_to_string(&file).unwrap();
    let std_uri = file_uri(&file);
    open(&mut client, &std_uri, &text);
    let start = text.find("fn push").unwrap() + "fn ".len();
    assert_eq!(
        refused(&prepare(&mut client, &std_uri, position(&text, start))),
        "`push` is in nova's cache and cannot be renamed"
    );
}

#[test]
fn rename_across_files() {
    let dir = project("rename-files", &[("main.nova", MAIN_IMPORTS_GEOMETRY), ("geometry.nova", GEOMETRY)]);
    let uri = file_uri(&dir.join("src").join("main.nova"));
    let mut client = Client::start(&dir, false);
    open(&mut client, &uri, MAIN_IMPORTS_GEOMETRY);
    let response = rename(&mut client, &uri, at(MAIN_IMPORTS_GEOMETRY, "area()", 0), "size");
    assert_eq!(
        edits(&response),
        [
            ("geometry.nova".to_string(), 1, 7, "size".to_string()),
            ("main.nova".to_string(), 3, 16, "size".to_string()),
        ]
    );
}

const SHORTHAND: &str = "record Point { x: Int }\n\
fn main() {\n    let x = 1\n    let p = Point { x }\n    let y = p.x\n}\n";

#[test]
fn rename_writes_out_a_shorthand() {
    let dir = project("rename-shorthand", &[("main.nova", SHORTHAND)]);
    let uri = file_uri(&dir.join("src").join("main.nova"));
    let mut client = Client::start(&dir, false);
    open(&mut client, &uri, SHORTHAND);
    // The field: its declaration, `p.x`, and `{ x }` written out.
    let field = rename(&mut client, &uri, at(SHORTHAND, "x: Int", 0), "w");
    assert_eq!(
        edits(&field),
        [
            ("main.nova".to_string(), 0, 15, "w".to_string()),
            ("main.nova".to_string(), 3, 20, "w: x".to_string()),
            ("main.nova".to_string(), 4, 14, "w".to_string()),
        ]
    );
    // The local: its `let`, and `{ x }` written out the other way.
    let local = rename(&mut client, &uri, at(SHORTHAND, "x = 1", 0), "z");
    assert_eq!(
        edits(&local),
        [
            ("main.nova".to_string(), 2, 8, "z".to_string()),
            ("main.nova".to_string(), 3, 20, "x: z".to_string()),
        ]
    );
}

#[test]
fn rename_a_trait_methods_family() {
    let dir = project("rename-family", &[("main.nova", FAMILY)]);
    let uri = file_uri(&dir.join("src").join("main.nova"));
    let mut client = Client::start(&dir, false);
    open(&mut client, &uri, FAMILY);
    let response = rename(&mut client, &uri, at(FAMILY, "show()", 0), "render");
    let found = edits(&response);
    assert_eq!(found.len(), 5, "{found:?}");
    assert!(found.iter().all(|e| e.3 == "render"), "{found:?}");
}

#[test]
fn rename_refuses_a_new_name_that_is_not_a_name() {
    let dir = project("rename-invalid", &[("main.nova", MAIN_IMPORTS_GEOMETRY), ("geometry.nova", GEOMETRY)]);
    let uri = file_uri(&dir.join("src").join("main.nova"));
    let mut client = Client::start(&dir, false);
    open(&mut client, &uri, MAIN_IMPORTS_GEOMETRY);
    let place = at(MAIN_IMPORTS_GEOMETRY, "total", 0);
    for new in ["1x", "let", "two words", "a-b"] {
        let message = refused(&rename(&mut client, &uri, place.clone(), new));
        assert!(message.contains("is not a name"), "{new}: {message}");
    }
    assert_eq!(refused(&rename(&mut client, &uri, place.clone(), "_")), "`_` cannot be a name");
    assert_eq!(
        refused(&rename(&mut client, &uri, place.clone(), "Int")),
        "`Int` is a built-in type's name"
    );
    // The same name: an empty edit, with no `changes`.
    let same = rename(&mut client, &uri, place, "total");
    assert!(same["error"].is_null(), "{same}");
    assert!(same["result"]["changes"].is_null(), "{same}");
}

// Review Focus 1.
#[test]
fn rename_edits_an_unsaved_buffer_at_its_own_offsets() {
    let dir = project("rename-unsaved", &[("main.nova", MAIN_IMPORTS_GEOMETRY), ("geometry.nova", GEOMETRY)]);
    let uri = file_uri(&dir.join("src").join("main.nova"));
    let mut client = Client::start(&dir, false);
    // The buffer has two lines the disk does not.
    let buffer = format!("// one\n// two\n{MAIN_IMPORTS_GEOMETRY}");
    open(&mut client, &uri, &buffer);
    let response = rename(&mut client, &uri, at(&buffer, "total", 0), "sum");
    assert_eq!(
        edits(&response),
        [
            ("main.nova".to_string(), 5, 8, "sum".to_string()),
            ("main.nova".to_string(), 6, 16, "sum".to_string()),
        ]
    );
}

// Review Focus 3.
#[test]
fn rename_in_a_crlf_document_edits_the_right_characters() {
    let crlf = MAIN_IMPORTS_GEOMETRY.replace('\n', "\r\n");
    let dir = project("rename-crlf", &[("main.nova", crlf.as_str()), ("geometry.nova", GEOMETRY)]);
    let uri = file_uri(&dir.join("src").join("main.nova"));
    let mut client = Client::start(&dir, false);
    open(&mut client, &uri, &crlf);
    let response = rename(&mut client, &uri, at(&crlf, "total", 1), "sum");
    assert_eq!(
        edits(&response),
        [
            ("main.nova".to_string(), 3, 8, "sum".to_string()),
            ("main.nova".to_string(), 4, 16, "sum".to_string()),
        ]
    );
}

// Review Focus 4.
#[test]
fn rename_reaches_the_projects_tests_files() {
    let dir = project("rename-tests", &[("lib.nova", "pub fn area() -> Int { 1 }\n")]);
    std::fs::create_dir_all(dir.join("tests")).unwrap();
    let test = "import demo\n\n@test\nfn area_is_one() {\n    let a: Int = area()\n}\n";
    std::fs::write(dir.join("tests").join("area.nova"), test).unwrap();
    let lib = file_uri(&dir.join("src").join("lib.nova"));
    let mut client = Client::start(&dir, false);
    open(&mut client, &lib, "pub fn area() -> Int { 1 }\n");
    let response = rename(&mut client, &lib, json!({ "line": 0, "character": 7 }), "size");
    assert_eq!(
        edits(&response),
        [
            ("area.nova".to_string(), 4, 17, "size".to_string()),
            ("lib.nova".to_string(), 0, 7, "size".to_string()),
        ]
    );
}
```

- [ ] **Step 2: Run the tests to see them fail**

Run: `cd /d/Projects/nona/nova && cargo test -p nova-cli --test lsp_navigation rename 2>&1 > $P/t10.txt; tail -20 $P/t10.txt`
Expected: the ten new tests fail. A request the server does not handle
gets `MethodNotFound` (-32601), so `refused` fails on `-32601` and `edits`
on "no edit".

- [ ] **Step 3: Prepare rename**

Create `crates/nova-lsp/src/rename.rs`:

```rust
//! Prepare rename and rename (spec
//! `docs/superpowers/specs/2026-10-10-phase-3-4a-navigation-design.md`
//! §5.4, §5.5).

use std::collections::HashMap;
use std::path::Path;

use lsp_types as lsp;
use nova_diagnostics::{FileId, LineIndex};
use nova_driver::Analysis;
use nova_lexer::Token;
use nova_pm::PackageId;
use nova_resolver::{Occurrence, Role, Target};

use crate::analysis::Answer;
use crate::workspace::{Overlay, PathKey};
use crate::{convert, navigate, std_cache, uri};

/// Why a rename is refused: a `RequestFailed` error's message.
#[derive(Debug)]
pub struct Refused(pub String);

/// A name that may be renamed: where it is, how it is spelt, and every
/// target the rename covers.
struct Found<'a> {
    at: &'a Occurrence,
    old: String,
    family: Vec<Target>,
}

/// Prepare rename at byte `offset` of `path` (spec §5.4): the name's range
/// and spelling, `None` for no name, or a refusal.
pub fn prepare(
    answer: &Answer,
    path: &Path,
    offset: u32,
) -> Result<Option<lsp::PrepareRenameResponse>, Refused> {
    let Some(found) = renameable(answer, path, offset)? else {
        return Ok(None);
    };
    let Some(source) = answer.analysis.db.get_source(answer.file) else {
        return Ok(None);
    };
    let range = convert::range(&LineIndex::new(source), found.at.span.start, found.at.span.end);
    Ok(Some(lsp::PrepareRenameResponse::RangeWithPlaceholder {
        range,
        placeholder: found.old,
    }))
}

fn renameable<'a>(answer: &'a Answer, path: &Path, offset: u32) -> Result<Option<Found<'a>>, Refused> {
    let a = &answer.analysis;
    let (Some(index), Some(defs)) = (a.index.as_ref(), a.definitions.as_ref()) else {
        return Ok(None);
    };
    let Some(at) = index.at(defs, answer.file, offset) else {
        return Ok(None);
    };
    let old = a
        .db
        .get_source(at.span.file)
        .and_then(|s| s.get(at.span.start as usize..at.span.end as usize))
        .unwrap_or("")
        .to_string();
    let refuse = |why: String| Err(Refused(why));
    if nova_lexer::KEYWORDS.contains(&old.as_str()) {
        return refuse(format!("`{old}` is a keyword and cannot be renamed"));
    }
    if std_cache::in_std_cache(path) || in_registry(path) {
        return refuse(format!("`{old}` is in nova's cache and cannot be renamed"));
    }
    match at.target {
        Target::Builtin(_) | Target::BuiltinMethod(_) | Target::Primitive(_) => {
            return refuse(format!("`{old}` is built in and cannot be renamed"));
        }
        Target::Module(_) => {
            return refuse(
                "a module or package is renamed by renaming its file or its `nova.toml`".to_string(),
            );
        }
        _ => {}
    }
    let family = index.family(&at.target);
    for target in &family {
        let Some(decl) = index.declaration(target) else {
            return refuse(format!("`{old}` has no declaration to rename"));
        };
        match owner(a, decl.span.file) {
            Owner::Own => {}
            Owner::Std => return refuse(format!("`{old}` is declared in std and cannot be renamed")),
            Owner::Dependency(name) => {
                return refuse(format!(
                    "`{old}` is declared in the dependency `{name}` and cannot be renamed"
                ));
            }
            Owner::Elsewhere => {
                return refuse(format!("`{old}` is not declared in this project and cannot be renamed"));
            }
        }
    }
    Ok(Some(Found { at, old, family }))
}

fn in_registry(path: &Path) -> bool {
    nova_pm::registry_dir().is_some_and(|r| PathKey::of(path).is_under(&PathKey::of(&r)))
}

enum Owner {
    Own,
    Std,
    Dependency(String),
    Elsewhere,
}

/// Whose a declaration's file is (plan decision 15).
fn owner(a: &Analysis, file: FileId) -> Owner {
    if a.db.get_name(file).is_some_and(|n| n.starts_with("<std/")) {
        return Owner::Std;
    }
    let Some(i) = a.modules.iter().position(|(f, _)| *f == file) else {
        return Owner::Elsewhere;
    };
    match a.module_packages.get(i).copied().flatten() {
        None | Some(PackageId(0)) => Owner::Own,
        Some(pid) => Owner::Dependency(
            a.graph
                .as_ref()
                .map(|g| g.package(pid).name.clone())
                .unwrap_or_default(),
        ),
    }
}
```

- [ ] **Step 4: Rename**

Append to `crates/nova-lsp/src/rename.rs`:

```rust
/// One replacement: `start..end` of `file` becomes `text`, which holds the
/// new name at `name_at`.
pub struct Edit {
    pub file: FileId,
    pub start: u32,
    pub end: u32,
    pub text: String,
    pub name_at: u32,
    pub role: Role,
}

/// Rename the name at byte `offset` of `path` to `new` (spec §5.4).
pub fn rename(
    answer: &Answer,
    path: &Path,
    offset: u32,
    new: &str,
    overlay: &Overlay,
    uri_of: &dyn Fn(&Path) -> String,
) -> Result<lsp::WorkspaceEdit, Refused> {
    let Some(found) = renameable(answer, path, offset)? else {
        return Err(Refused("there is no name here to rename".to_string()));
    };
    check_new_name(new)?;
    if new == found.old {
        return Ok(lsp::WorkspaceEdit::default());
    }
    let edits = plan_edits(&answer.analysis, &found, new);
    let _ = overlay; // Task 11's check reads it.
    Ok(workspace_edit(&answer.analysis, &edits, uri_of))
}

/// Spec §5.4: one identifier, not `_`, and not a built-in type's name.
fn check_new_name(new: &str) -> Result<(), Refused> {
    let (tokens, errors) = nova_lexer::lex(new, FileId::DUMMY);
    let tokens: Vec<&Token> = tokens
        .iter()
        .map(|t| &t.value)
        .filter(|t| !matches!(t, Token::Eof))
        .collect();
    let one_name =
        errors.is_empty() && matches!(tokens.as_slice(), [Token::Ident(s)] if s == new);
    if !one_name {
        return Err(Refused(format!(
            "`{new}` is not a name: a new name is one identifier, and not a keyword"
        )));
    }
    if new == "_" {
        return Err(Refused("`_` cannot be a name".to_string()));
    }
    if nova_resolver::RESERVED_TYPE_NAMES.contains(&new) {
        return Err(Refused(format!("`{new}` is a built-in type's name")));
    }
    Ok(())
}

/// The edits renaming `found` to `new` makes, in the spec's one order. A
/// shorthand `{ x }` is written out (spec §5.4).
fn plan_edits(a: &Analysis, found: &Found, new: &str) -> Vec<Edit> {
    let Some(index) = a.index.as_ref() else {
        return Vec::new();
    };
    let mut renamed: Vec<&Occurrence> = index
        .occurrences
        .iter()
        .filter(|o| found.family.contains(&o.target))
        .collect();
    navigate::sort_by_place(a, &mut renamed);
    renamed.dedup_by_key(|o| o.span);
    renamed
        .into_iter()
        .map(|o| {
            let (text, name_at) = match (o.shorthand, o.target) {
                (true, Target::Field(..)) => (format!("{new}: {}", found.old), 0),
                (true, _) => (format!("{}: {new}", found.old), found.old.len() as u32 + 2),
                (false, _) => (new.to_string(), 0),
            };
            Edit {
                file: o.span.file,
                start: o.span.start,
                end: o.span.end,
                text,
                name_at,
                role: o.role,
            }
        })
        .collect()
}

/// The edits as LSP's, each file under the URI `uri_of` gives it.
fn workspace_edit(a: &Analysis, edits: &[Edit], uri_of: &dyn Fn(&Path) -> String) -> lsp::WorkspaceEdit {
    let mut indexes: HashMap<FileId, LineIndex> = HashMap::new();
    let mut changes: HashMap<lsp::Uri, Vec<lsp::TextEdit>> = HashMap::new();
    for e in edits {
        let (Some(path), Some(source)) = (navigate::path_of(a, e.file), a.db.get_source(e.file)) else {
            continue;
        };
        let Some(uri) = uri::parse(&uri_of(&path)) else {
            continue;
        };
        let lines = indexes.entry(e.file).or_insert_with(|| LineIndex::new(source));
        changes.entry(uri).or_default().push(lsp::TextEdit {
            range: convert::range(lines, e.start, e.end),
            new_text: e.text.clone(),
        });
    }
    lsp::WorkspaceEdit {
        changes: Some(changes),
        ..Default::default()
    }
}
```

In `crates/nova-lsp/src/lib.rs`:
- Add `mod rename;`.
- Add `PrepareRenameRequest` and `Rename` to the request imports.
- In `capabilities()`, add:

```rust
        rename_provider: Some(lsp::OneOf::Right(lsp::RenameOptions {
            prepare_provider: Some(true),
            work_done_progress_options: Default::default(),
        })),
```

- Before the `_ =>` arm of `Server::request`:

```rust
            PrepareRenameRequest::METHOD => {
                match serde_json::from_value::<lsp::TextDocumentPositionParams>(request.params) {
                    Ok(p) => match self.answer_at(&p) {
                        Some((answer, offset)) => {
                            let path = self.path_of(&p.text_document.uri);
                            match rename::prepare(&answer, &path, offset) {
                                Ok(found) => Response::new_ok(request.id, found),
                                Err(rename::Refused(why)) => refused(request.id, why),
                            }
                        }
                        None => Response::new_ok(request.id, Value::Null),
                    },
                    Err(e) => invalid(request.id, e),
                }
            }
            Rename::METHOD => {
                match serde_json::from_value::<lsp::RenameParams>(request.params) {
                    Ok(p) => match self.answer_at(&p.text_document_position) {
                        Some((answer, offset)) => {
                            let path = self.path_of(&p.text_document_position.text_document.uri);
                            let overlay = self.workspace.overlay();
                            match rename::rename(&answer, &path, offset, &p.new_name, &overlay, &|path| {
                                self.uri_of(path)
                            }) {
                                Ok(edit) => Response::new_ok(request.id, edit),
                                Err(rename::Refused(why)) => refused(request.id, why),
                            }
                        }
                        None => Response::new_ok(request.id, Value::Null),
                    },
                    Err(e) => invalid(request.id, e),
                }
            }
```

- Add to `impl Server<'_>`:

```rust
    /// The path a request's document is checked under.
    fn path_of(&self, uri: &lsp::Uri) -> PathBuf {
        let text = uri::text(uri);
        match self.workspace.get(&text) {
            Some(doc) => doc.path.clone(),
            None => uri::document_path(&text).unwrap_or_default(),
        }
    }
```

- After `fn invalid(`, add:

```rust
/// A refusal: `RequestFailed`, with the reason (spec 3.4a §5.4).
fn refused(id: lsp_server::RequestId, why: String) -> Response {
    Response::new_err(id, ErrorCode::RequestFailed as i32, why)
}
```

- Add `use serde_json::Value;` if it is not imported.

- [ ] **Step 5: Run the tests to see them pass**

Run: `cd /d/Projects/nona/nova && cargo test -p nova-cli --test lsp_navigation 2>&1 > $P/t10.txt; tail -8 $P/t10.txt`
Expected: `test result: ok. 27 passed; 0 failed`.

- [ ] **Step 6: Commit**

Write `$P/msg-10.txt`:

```text
nova-lsp: prepare rename and rename

Prepare rename gives a name's range and spelling, or refuses with the
reason: std's and a dependency's names, builtins, primitives,
keywords, modules and packages, and anything in nova's caches (spec
3.4a §5.4). A trait method's family is renamed whole, so one whose
trait is std's is refused. Rename checks the new name, edits every
occurrence in the owning project's analysis, `tests/` included, and
writes out a shorthand `{ x }` as `{ w: x }` or `{ x: z }`.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
```

Run: `cd /d/Projects/nona/nova && cargo fmt --all && git add crates/nova-lsp crates/nova-cli/tests/lsp_navigation.rs && git commit -q -F $P/msg-10.txt && git log -1 --format=%s`
Expected: `nova-lsp: prepare rename and rename`

---

### Task 11: The rename check

**Files:**
- Modify: `crates/nova-lsp/src/rename.rs`
- Modify: `crates/nova-cli/tests/lsp_navigation.rs`

**Interfaces:**
- Consumes: Task 10's `Edit`, `plan_edits`; Task 7's `analysis::analyse`,
  `Overlay::with`.
- Produces: `rename::rename` refuses as spec §5.5 says.

- [ ] **Step 1: Write the failing tests**

Append to `crates/nova-cli/tests/lsp_navigation.rs`:

```rust
// === Task 11: the rename check ===

#[test]
fn a_rename_that_captures_a_name_is_refused() {
    let text = "fn main() {\n    let y = 1\n    let x = 2\n    let z = x + y\n}\n";
    let dir = project("check-capture", &[("main.nova", text)]);
    let uri = file_uri(&dir.join("src").join("main.nova"));
    let mut client = Client::start(&dir, false);
    open(&mut client, &uri, text);
    // `let y = 2` would shadow the first `y`, so `x + y` would read it.
    assert_eq!(
        refused(&rename(&mut client, &uri, at(text, "x = 2", 0), "y")),
        "renaming `x` to `y` would make 1 other name refer to it"
    );
}

#[test]
fn a_rename_that_changes_what_a_name_means_is_refused() {
    let text = "fn main() {\n    let x = 1\n    let f = |y: Int| x + y\n}\n";
    let dir = project("check-change", &[("main.nova", text)]);
    let uri = file_uri(&dir.join("src").join("main.nova"));
    let mut client = Client::start(&dir, false);
    open(&mut client, &uri, text);
    // Inside the closure, `y + y` would read its own parameter twice.
    assert_eq!(
        refused(&rename(&mut client, &uri, at(text, "x = 1", 0), "y")),
        "renaming `x` to `y` would change what 1 name refers to"
    );
}

#[test]
fn a_rename_that_adds_an_error_is_refused() {
    let text = "fn helper() -> Int { 1 }\nfn other() -> Int { 2 }\nfn main() {\n    let a = helper()\n}\n";
    let dir = project("check-error", &[("main.nova", text)]);
    let uri = file_uri(&dir.join("src").join("main.nova"));
    let mut client = Client::start(&dir, false);
    open(&mut client, &uri, text);
    let message = refused(&rename(&mut client, &uri, at(text, "helper", 0), "other"));
    assert!(
        message.starts_with("renaming `helper` to `other` would add an error: E0002"),
        "{message}"
    );
}

#[test]
fn a_program_with_errors_can_still_be_renamed() {
    // A guard: the check counts error codes, so an error already there
    // does not stop a rename.
    let text = "fn main() {\n    let total: Int = \"s\"\n    let b = total\n}\n";
    let dir = project("check-broken", &[("main.nova", text)]);
    let uri = file_uri(&dir.join("src").join("main.nova"));
    let mut client = Client::start(&dir, false);
    open(&mut client, &uri, text);
    let response = rename(&mut client, &uri, at(text, "total", 0), "sum");
    assert_eq!(edits(&response).len(), 2, "{response}");
}
```

- [ ] **Step 2: Run the tests to see them fail**

Run: `cd /d/Projects/nona/nova && cargo test -p nova-cli --test lsp_navigation a_rename_that 2>&1 | tail -15`
Expected: the three refusal tests fail: each gets an edit, so `refused`
fails on a missing `error`.

Run: `cd /d/Projects/nona/nova && cargo test -p nova-cli --test lsp_navigation a_program_with_errors 2>&1 | tail -5`
Expected: `1 passed`. It is a guard, marked so in its comment: rename
already edits a broken program, and the check must keep it that way.

- [ ] **Step 3: The check**

In `rename`, replace the line `let _ = overlay; // Task 11's check reads it.`
with:

```rust
    check(answer, &found, new, &edits, overlay)?;
```

Append to `crates/nova-lsp/src/rename.rs`:

```rust
/// A span as both analyses can compare it: by file name, not `FileId`,
/// since the second analysis numbers files again (plan decision 10).
type Place = (String, u32, u32);

/// Spec §5.5: analyse the renamed program, and refuse unless every renamed
/// name still means what it meant, nothing else came to mean it, and no
/// error code became more common.
fn check(
    answer: &Answer,
    found: &Found,
    new: &str,
    edits: &[Edit],
    overlay: &Overlay,
) -> Result<(), Refused> {
    let a = &answer.analysis;
    let name_of = |f: FileId| a.db.get_name(f).unwrap_or("").to_string();

    // The renamed texts, over the overlay; and where each renamed name
    // lands in them.
    let mut renamed = overlay.clone();
    let mut expected: Vec<Place> = Vec::new();
    let mut declared: Vec<Place> = Vec::new();
    let mut files: Vec<FileId> = edits.iter().map(|e| e.file).collect();
    files.dedup();
    for file in files {
        let Some(source) = a.db.get_source(file) else {
            continue;
        };
        let mut text = String::with_capacity(source.len());
        let mut copied = 0usize;
        for e in edits.iter().filter(|e| e.file == file) {
            text.push_str(&source[copied..e.start as usize]);
            let start = text.len() as u32 + e.name_at;
            let place = (name_of(file), start, start + new.len() as u32);
            if e.role == Role::Declaration {
                declared.push(place.clone());
            }
            expected.push(place);
            text.push_str(&e.text);
            copied = e.end as usize;
        }
        text.push_str(&source[copied..]);
        renamed = renamed.with(Path::new(&name_of(file)), text);
    }

    let options = crate::analysis::options(None, true);
    let Some(b) = crate::analysis::analyse(&answer.scope, &renamed, &options) else {
        return Err(Refused("the renamed program could not be analysed".to_string()));
    };
    let Some(index) = b.index.as_ref() else {
        return Err(Refused("the renamed program could not be analysed".to_string()));
    };
    let place = |o: &Occurrence| -> Place {
        (
            b.db.get_name(o.span.file).unwrap_or("").to_string(),
            o.span.start,
            o.span.end,
        )
    };
    let targets: Vec<Target> = index
        .occurrences
        .iter()
        .filter(|o| o.role == Role::Declaration && declared.contains(&place(o)))
        .map(|o| o.target)
        .collect();
    let now: Vec<Place> = index
        .occurrences
        .iter()
        .filter(|o| targets.contains(&o.target))
        .map(|o| place(o))
        .collect();
    let old = &found.old;
    let captured = now.iter().filter(|p| !expected.contains(p)).count();
    if captured > 0 {
        return Err(Refused(format!(
            "renaming `{old}` to `{new}` would make {} refer to it",
            count(captured, "other name", "other names")
        )));
    }
    let changed = expected.iter().filter(|p| !now.contains(p)).count();
    if changed > 0 {
        let verb = if changed == 1 { "refers" } else { "refer" };
        return Err(Refused(format!(
            "renaming `{old}` to `{new}` would change what {} {verb} to",
            count(changed, "name", "names")
        )));
    }
    if let Some((code, message)) = new_error(a, &b) {
        return Err(Refused(format!(
            "renaming `{old}` to `{new}` would add an error: {code} {message}"
        )));
    }
    Ok(())
}

fn count(n: usize, one: &str, many: &str) -> String {
    format!("{n} {}", if n == 1 { one } else { many })
}

/// The first error of `after`, in the spec's one order, whose code is
/// more common than in `before`.
fn new_error(before: &Analysis, after: &Analysis) -> Option<(String, String)> {
    use nova_diagnostics::Severity;
    let codes = |a: &Analysis| {
        let mut n: HashMap<String, usize> = HashMap::new();
        for d in a.diagnostics.iter().filter(|d| d.severity == Severity::Error) {
            *n.entry(d.code.clone()).or_default() += 1;
        }
        n
    };
    let (was, now) = (codes(before), codes(after));
    let mut errors: Vec<&nova_diagnostics::Diagnostic> = after
        .diagnostics
        .iter()
        .filter(|d| d.severity == Severity::Error)
        .filter(|d| now.get(&d.code) > was.get(&d.code).or(Some(&0)))
        .collect();
    let key = |d: &nova_diagnostics::Diagnostic| {
        let label = d.labels.iter().find(|l| l.primary).or(d.labels.first());
        label.map_or((String::new(), 0), |l| {
            (after.db.get_name(l.span.file).unwrap_or("").to_string(), l.span.start)
        })
    };
    errors.sort_by_key(|d| key(d));
    errors.first().map(|d| (d.code.clone(), d.message.clone()))
}
```

(`now.get(&d.code) > was.get(&d.code).or(Some(&0))` compares
`Option<&usize>`: a code that was absent counts as `Some(&0)`.)

- [ ] **Step 4: Run the tests to see them pass**

Run: `cd /d/Projects/nona/nova && cargo test -p nova-cli --test lsp_navigation 2>&1 > $P/t11.txt; tail -8 $P/t11.txt`
Expected: `test result: ok. 31 passed; 0 failed`.

- [ ] **Step 5: Commit**

Write `$P/msg-11.txt`:

```text
nova-lsp: rename checks itself

Before answering, rename applies its edits in memory and analyses the
program again (spec 3.4a §5.5). It refuses, saying why, when a renamed
name would resolve elsewhere, another name would come to mean the
renamed one, or an error code would become more common. Shadowing,
captures, clashes and duplicates need no rules of their own. A
program that already had errors can still be renamed.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
```

Run: `cd /d/Projects/nona/nova && cargo fmt --all && git add crates/nova-lsp crates/nova-cli/tests/lsp_navigation.rs && git commit -q -F $P/msg-11.txt && git log -1 --format=%s`
Expected: `nova-lsp: rename checks itself`

---

### Task 12: The gate test, and latency

**Files:**
- Modify: `crates/nova-cli/tests/lsp_navigation.rs` (the gate)
- Modify: `crates/nova-cli/tests/lsp.rs` (`measure`, its two callers)

**Interfaces:**
- Consumes: Tasks 7-11's requests.
- Produces: the figures ADR 0032 records (Task 14), in the ledger.

- [ ] **Step 1: Write the gate test**

Append to `crates/nova-cli/tests/lsp_navigation.rs`:

```rust
// === Task 12: the gate (spec §9) ===

const GATE_LIB: &str = "/// A rectangle's area.\npub fn area(w: Int, h: Int) -> Int { w * h }\n";
const GATE_UTIL: &str =
    "pub record Size { w: Int, h: Int }\npub fn square(n: Int) -> Size { Size { w: n, h: n } }\n";
const GATE_MAIN: &str = "import geom\nimport util\n\n\
fn main() {\n    let side = 3\n    let s = square(side)\n    let w = s.w\n    let t = Size { w, h: s.h }\n    let a = area(t.w, t.h)\n    let mut v = Vec::new()\n    v.push(a)\n    let h = 1\n    let total = side + h\n}\n";

#[test]
fn the_navigation_gate() {
    let (app, geom) = app_and_library("gate", GATE_LIB);
    std::fs::write(app.join("src").join("main.nova"), GATE_MAIN).unwrap();
    std::fs::write(app.join("src").join("util.nova"), GATE_UTIL).unwrap();
    let home = home("gate");
    let main = file_uri(&app.join("src").join("main.nova"));
    let mut client = Client::start_with_env(&app, false, &[("NOVA_HOME", &home)]);
    open(&mut client, &main, GATE_MAIN);

    // Hover: a local, a function with docs, a field, a std method.
    assert_eq!(code(&hover(&mut client, &main, at(GATE_MAIN, "side", 0))), "let side: Int");
    assert_eq!(
        markdown(&hover(&mut client, &main, at(GATE_MAIN, "area(", 0))),
        "```nova\npub fn area(w: Int, h: Int) -> Int\n```\n\nA rectangle's area."
    );
    assert_eq!(code(&hover(&mut client, &main, at(GATE_MAIN, "w\n", 0))), "w: Int");
    assert!(code(&hover(&mut client, &main, at(GATE_MAIN, "push", 0))).contains("fn push("));

    // Definition: the same file, another module, the dependency, std.
    let local = definition(&mut client, &main, at(GATE_MAIN, "side)", 0))["result"].clone();
    assert_eq!(local["range"], range(GATE_MAIN, "side", 0, "side"));
    let square = definition(&mut client, &main, at(GATE_MAIN, "square", 0))["result"].clone();
    assert!(path_of(&square).ends_with("util.nova"), "{square}");
    let area = definition(&mut client, &main, at(GATE_MAIN, "area(", 0))["result"].clone();
    assert!(
        same_uri(area["uri"].as_str().unwrap(), &file_uri(&geom.join("src").join("lib.nova"))),
        "{area}"
    );
    let push = definition(&mut client, &main, at(GATE_MAIN, "push", 0))["result"].clone();
    assert!(path_of(&push).starts_with(home.join("std")), "{push}");

    // References, with and without the declaration.
    assert_eq!(references(&mut client, &main, at(GATE_MAIN, "side", 0), true).len(), 3);
    assert_eq!(references(&mut client, &main, at(GATE_MAIN, "side", 0), false).len(), 2);

    // Rename across files, a shorthand field among them.
    let renamed = edits(&rename(&mut client, &main, at(GATE_MAIN, "w\n", 0), "width"));
    assert_eq!(renamed.len(), 5, "{renamed:?}");
    assert!(
        renamed.iter().any(|e| e.0 == "main.nova" && e.3 == "width: w"),
        "{renamed:?}"
    );
    assert_eq!(renamed.iter().filter(|e| e.0 == "util.nova").count(), 2, "{renamed:?}");

    // Prepare rename refuses std's and the dependency's names.
    assert!(refused(&prepare(&mut client, &main, at(GATE_MAIN, "push", 0))).contains("in std"));
    assert!(refused(&prepare(&mut client, &main, at(GATE_MAIN, "area(", 0))).contains("dependency `geom`"));

    // The check refuses a rename that shadows another name.
    assert_eq!(
        refused(&rename(&mut client, &main, at(GATE_MAIN, "h = 1", 0), "side")),
        "renaming `h` to `side` would make 1 other name refer to it"
    );
}
```

`at(GATE_MAIN, "w\n", 0)` is the `w` of `s.w`, the only `w` followed by a
newline. `side)` is the `side` of `square(side)`.

Then the robustness case of spec §8.4, for the requests other than hover:

```rust
#[test]
fn each_request_works_in_a_broken_file_and_is_empty_for_an_unopened_one() {
    // A guard over Tasks 8-11, as Task 7's hover test is for hover.
    let text = "fn broken( {\n}\nfn main() {\n    let total = 1\n    let b = total\n}\n";
    let dir = project("requests-broken", &[("main.nova", text)]);
    let uri = file_uri(&dir.join("src").join("main.nova"));
    let other = file_uri(&dir.join("src").join("other.nova"));
    let mut client = Client::start(&dir, false);
    open(&mut client, &uri, text);
    let use_ = at(text, "total", 1);
    let found = definition(&mut client, &uri, use_.clone())["result"].clone();
    assert_eq!(found["range"], range(text, "total", 0, "total"));
    assert_eq!(references(&mut client, &uri, use_.clone(), true).len(), 2);
    assert_eq!(prepare(&mut client, &uri, use_.clone())["result"]["placeholder"], "total");
    assert_eq!(edits(&rename(&mut client, &uri, use_.clone(), "sum")).len(), 2);
    // A document that is not open: empty results, never an error.
    assert_eq!(definition(&mut client, &other, use_.clone())["result"], Value::Null);
    assert!(references(&mut client, &other, use_.clone(), true).is_empty());
    assert_eq!(prepare(&mut client, &other, use_.clone())["result"], Value::Null);
    assert_eq!(rename(&mut client, &other, use_, "sum")["result"], Value::Null);
}
```

- [ ] **Step 2: Run the gate and robustness tests**

Run: `cd /d/Projects/nona/nova && cargo test -p nova-cli --test lsp_navigation 2>&1 > $P/t12.txt; tail -8 $P/t12.txt`
Expected: `test result: ok. 33 passed; 0 failed`. Both new tests are guards
over Tasks 7-11, so they pass on their first run. If one fails, the
failing line names a behaviour an earlier task missed: fix it there, with
a test in that task's style, and ledger a ruling.

- [ ] **Step 3: Measure the new requests**

In `crates/nova-cli/tests/lsp.rs`, replace `fn measure` with:

```rust
/// Each request's times in milliseconds, sorted.
struct Figures {
    edits: Vec<u128>,
    completions: Vec<u128>,
    hovers: Vec<u128>,
    definitions: Vec<u128>,
    references: Vec<u128>,
    renames: Vec<u128>,
}

/// `n` requests of `method`, each timed to its response, sorted.
fn timed(client: &mut Client, n: usize, method: &str, params: Value) -> Vec<u128> {
    let mut ms: Vec<u128> = (0..n)
        .map(|_| {
            let started = Instant::now();
            let response = client.request(method, params.clone());
            assert!(
                response["error"].is_null() && !response["result"].is_null(),
                "{method}: {response}"
            );
            started.elapsed().as_millis()
        })
        .collect();
    ms.sort_unstable();
    ms
}

/// Time `n` edits, each sent when no check is running, and `n` of each
/// request, on `05-json-api` (ADR 0029; 3.4a spec §8.6).
fn measure(n: usize) -> Figures {
    let path = json_api();
    let text = std::fs::read_to_string(&path).unwrap();
    let uri = file_uri(&path);
    let home = fresh_dir("latency-home");
    let mut client = Client::start_with_env(path.parent().unwrap(), false, &[("NOVA_HOME", &home)]);
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
        assert!(
            labels(&response).contains(&"insert".to_string()),
            "{response}"
        );
    }
    edits.sort_unstable();
    completions.sort_unstable();
    // `user_json(u)` in `users_json`: a function main.nova declares. The
    // edits above only appended, so its position is unchanged.
    let call = after(&text, "parts[n] = ");
    let at = json!({ "textDocument": { "uri": uri }, "position": call });
    let hovers = timed(&mut client, n, "textDocument/hover", at.clone());
    let definitions = timed(&mut client, n, "textDocument/definition", at.clone());
    let mut with_context = at.clone();
    with_context["context"] = json!({ "includeDeclaration": true });
    let references = timed(&mut client, n, "textDocument/references", with_context);
    let mut renaming = at;
    renaming["newName"] = json!("user_to_json");
    let renames = timed(&mut client, n, "textDocument/rename", renaming);
    Figures {
        edits,
        completions,
        hovers,
        definitions,
        references,
        renames,
    }
}
```

Replace `edits_and_completions_stay_within_the_ci_bound` with:

```rust
#[test]
fn requests_stay_within_the_ci_bound() {
    // Gate item 8, and 3.4a's: 2 s each with the debug binary, 4 s for a
    // rename, which analyses twice.
    let f = measure(3);
    let within = |ms: &[u128], bound: u128| ms.iter().all(|&m| m <= bound);
    assert!(
        within(&f.edits, 2000)
            && within(&f.completions, 2000)
            && within(&f.hovers, 2000)
            && within(&f.definitions, 2000)
            && within(&f.references, 2000)
            && within(&f.renames, 4000),
        "{}; {}; {}; {}; {}; {}",
        summary("edits", &f.edits),
        summary("completions", &f.completions),
        summary("hovers", &f.hovers),
        summary("definitions", &f.definitions),
        summary("references", &f.references),
        summary("renames", &f.renames)
    );
}
```

and change `latency_on_05_json_api`'s body to:

```rust
    let f = measure(20);
    // In a release build, the budgets: 200 ms, and 400 ms for a rename.
    // CI's advisory `--ignored` step runs this in a debug build, where only
    // the CI bounds apply.
    let (bound, rename_bound) = if cfg!(debug_assertions) { (2000, 4000) } else { (200, 400) };
    let binary = assert_cmd::cargo::cargo_bin("nova");
    let meta = std::fs::metadata(&binary).unwrap();
    println!("{}", summary("edit to diagnostics", &f.edits));
    println!("{}", summary("completion", &f.completions));
    println!("{}", summary("hover", &f.hovers));
    println!("{}", summary("definition", &f.definitions));
    println!("{}", summary("references", &f.references));
    println!("{}", summary("rename", &f.renames));
    println!(
        "binary {} ({} bytes, modified {:?})",
        binary.display(),
        meta.len(),
        meta.modified().ok()
    );
    let within = |ms: &[u128], b: u128| ms.iter().all(|&m| m <= b);
    assert!(
        [&f.edits, &f.completions, &f.hovers, &f.definitions, &f.references]
            .iter()
            .all(|ms| within(ms, bound))
            && within(&f.renames, rename_bound),
        "the budget is {bound} ms, {rename_bound} ms for a rename, for the median and the maximum"
    );
```

Renaming the CI-bound test is a change on purpose: its old name described
two of its six measurements. Ledger the rename, so the CI result lists can
be compared by name.

- [ ] **Step 4: Run them**

Run: `cd /d/Projects/nona/nova && cargo test -p nova-cli --test lsp requests_stay_within 2>&1 | tail -6`
Expected: `1 passed`.

Then the development host's figures, in a release build:

Run: `cd /d/Projects/nona/nova && cargo test --release -p nova-cli --test lsp -- --ignored --nocapture latency 2>&1 > $P/latency-1.txt; grep -E "median|binary|test result" $P/latency-1.txt`
Expected: six `median … max …` lines, each median and maximum within its
budget, the binary's size and time, and `1 passed`.

Run it twice more (`latency-2.txt`, `latency-3.txt`), so that ADR 0032 can
give ranges. Ledger the three runs' lines verbatim. If any request misses
its budget, or completion or diagnostics now miss ADR 0029's 200 ms,
**stop and ask the user** (Global Constraints).

- [ ] **Step 5: Commit**

Write `$P/msg-12.txt`:

```text
nova-cli: the navigation gate, and its latency

One test over stdio covers the gate (spec 3.4a §9): hover on a local, a
documented function, a field and a std method; definition in the same
file, into another module, the dependency and std; references with and
without the declaration; a rename across files with a shorthand field;
prepare rename's refusals for std and the dependency; and the check's
refusal of a shadowing rename. The latency measurement on 05-json-api
gains hover, definition, references and rename (200 ms, 400 ms for a
rename; CI 2 s and 4 s), and its CI-bound test is renamed for what it
now measures.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
```

Run: `cd /d/Projects/nona/nova && cargo fmt --all && git add crates/nova-cli/tests && git commit -q -F $P/msg-12.txt && git log -1 --format=%s`
Expected: `nova-cli: the navigation gate, and its latency`

---

### Task 13: The VS Code extension

**Files:**
- Create: `tools/vscode-nova/test/fixture/src/shapes.nova`, `tools/vscode-nova/test/fixture/src/navigate.nova`
- Modify: `tools/vscode-nova/test/suite/smoke.test.ts`, `tools/vscode-nova/README.md`

**Interfaces:**
- Consumes: hover and definition through `nova lsp` (Tasks 7, 8).
- Produces: two smoke-test cases CI runs.

- [ ] **Step 1: The fixture**

Create `tools/vscode-nova/test/fixture/src/shapes.nova`:

```nova
/// The area of a square.
pub fn area(side: Int) -> Int { side * side }
```

and `tools/vscode-nova/test/fixture/src/navigate.nova`:

```nova
import shapes

fn main() {
    let a = area(2)
}
```

Check them with the CLI, which also proves the fixture is valid Nova:

Run: `cd /d/Projects/nona/nova && cargo run -q -p nova-cli -- check tools/vscode-nova/test/fixture/src/navigate.nova; echo "exit $?"`
Expected: `exit 0`.

Run: `cd /d/Projects/nona/nova && cargo run -q -p nova-cli -- fmt --check tools/vscode-nova/test/fixture/src/shapes.nova tools/vscode-nova/test/fixture/src/navigate.nova; echo "exit $?"`
Expected: `exit 0`. If the formatter differs, keep its form, and ledger it.

- [ ] **Step 2: The smoke test's two cases**

In `tools/vscode-nova/test/suite/smoke.test.ts`, change the file comment's
first line to "The extension's smoke test (spec §7.6; 3.4a §7): the
language, a diagnostic, a completion, a formatted document, a hover and a
definition, through the real `nova lsp`.", and add, before the closing
`});` of the `describe` block:

```ts
  it("hovers over a call with its signature and doc", async () => {
    const uri = vscode.Uri.file(path.join(fixture, "src", "navigate.nova"));
    const doc = await vscode.workspace.openTextDocument(uri);
    const at = doc.positionAt(doc.getText().indexOf("area(") + 1);
    const text = await eventually("a hover", async () => {
      const hovers = await vscode.commands.executeCommand<vscode.Hover[]>(
        "vscode.executeHoverProvider",
        uri,
        at,
      );
      const joined = (hovers ?? [])
        .flatMap((h) => h.contents.map((c) => (typeof c === "string" ? c : c.value)))
        .join("\n");
      return joined.includes("fn area") ? joined : undefined;
    });
    assert.ok(text.includes("pub fn area(side: Int) -> Int"), text);
    assert.ok(text.includes("The area of a square."), text);
  });

  it("goes to a definition in another module", async () => {
    const uri = vscode.Uri.file(path.join(fixture, "src", "navigate.nova"));
    const doc = await vscode.workspace.openTextDocument(uri);
    const at = doc.positionAt(doc.getText().indexOf("area(") + 1);
    const found = await eventually("a definition", async () => {
      const locations = await vscode.commands.executeCommand<
        (vscode.Location | vscode.LocationLink)[]
      >("vscode.executeDefinitionProvider", uri, at);
      return locations !== undefined && locations.length > 0 ? locations : undefined;
    });
    const first = found[0];
    const target = "targetUri" in first ? first.targetUri : first.uri;
    const range =
      "targetUri" in first ? (first.targetSelectionRange ?? first.targetRange) : first.range;
    assert.ok(target.fsPath.endsWith("shapes.nova"), target.fsPath);
    assert.strictEqual(range.start.line, 1);
    assert.strictEqual(range.start.character, 7);
  });
```

- [ ] **Step 3: Compile the extension's TypeScript**

The extension's `node_modules` is already installed (3.2), so compiling
needs no download:

Run: `cd /d/Projects/nona/nova/tools/vscode-nova && npx tsc -p . 2>&1 | tail -5; echo "exit $?"`
Expected: no errors, `exit 0`.

Running the smoke test locally downloads VS Code, which waits for the
user's word (Global Constraints). Without it, CI's `VS Code extension`
job is the first run: ledger a ruling.

- [ ] **Step 4: The extension's README**

In `tools/vscode-nova/README.md`, change the first paragraph to:

```text
Colouring for `.nova` files, and from `nova lsp`: diagnostics, completion,
formatting, hover, go to definition, find references and rename.
```

- [ ] **Step 5: Commit**

Write `$P/msg-13.txt`:

```text
vscode-nova: hover and go to definition in the smoke test

The fixture gains a documented `area` in shapes.nova and a call to it
in navigate.nova. The smoke test hovers over the call, which must show
the signature and the doc, and goes to its definition, which must be
`area`'s name in shapes.nova (spec 3.4a §7). The README lists the new
features.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
```

Run: `cd /d/Projects/nona/nova && git add tools/vscode-nova/test tools/vscode-nova/README.md && git status --short tools/vscode-nova && git commit -q -F $P/msg-13.txt && git log -1 --format=%s`
Expected: the four files staged, nothing from `out/` or `node_modules/`;
then `vscode-nova: hover and go to definition in the smoke test`.

---

### Task 14: Records

**Files:**
- Create: `docs/adr/0032-navigation-in-the-language-server.md`
- Modify:
  - `nova-spec/40-TOOLING.md` (§3.1's note);
  - `docs/phase-3-plan.md` (the 3.4 entry);
  - `agent.md` (after the crate table's notes);
  - `CHANGELOG.md` (`[Unreleased]`, Added);
  - `README.md` ("Editor support");
  - `ARCHITECTURE.md` (the crate rows).
- Modify: whatever the sweep finds.

**Interfaces:**
- Consumes: Task 12's latency figures, from the ledger.

- [ ] **Step 1: ADR 0032**

Create `docs/adr/0032-navigation-in-the-language-server.md`, with the
figures from Task 12's three ledgered runs in the table:

```markdown
# ADR 0032 — Navigation in the language server

## Status

Accepted, 2026-10-10 (Phase 3.4a, branch `phase-3-4a-navigation`; spec
`docs/superpowers/specs/2026-10-10-phase-3-4a-navigation-design.md`).

## Context

3.2 gave `nova lsp` diagnostics, completion and formatting, and left
hover, go to definition, find references and rename to 3.4 (ADR 0029).
The server re-runs the front end on each request. Completion uses a probe
that looks at one place in the program. ADR 0029 set aside a full
position index, because it "makes every check pay for tables only the
server reads".

On 2026-10-10 the user split 3.4 in two: 3.4a, navigation, and 3.4b,
fixes and colour.

## Decision

1. **An index of name occurrences.**
   - The type checker records it where it already decides what a name
     means. The resolver records the import items' occurrences.
   - Each occurrence has its span, whether it declares or uses, and its
     target: a definition, variant, field, trait method, local, type
     parameter, module, builtin or primitive.
   - The types live in `nova-resolver` (`index.rs`), because
     `nova-typeck` depends on it.
   - The index is off unless the server asks for it, so `nova check`,
     `build`, `run`, `test` and the server's diagnostics pay nothing. That
     answers ADR 0029's reason for setting a full index aside.
2. **Recording happens only at the AST's name sites,** with duplicates
   removed. The checker passes some names twice, and makes names of its
   own: a `for` loop's `next`, interpolation's `fmt`, and temporaries.
   Two checks over std, `examples/` and `tests/runtime/` (159 files) keep
   the index complete:
   - every use has exactly one declaration;
   - every identifier is covered.
3. **A trait method and the impl methods that implement it are one
   family** for references and rename. A call resolves through the trait,
   so without families an impl method would have no uses.
4. **References and rename reach the owning project only** (the user,
   2026-10-10). That is its program, library and `tests/`.
5. **Rename checks itself.** It applies its edits in memory and analyses
   the program again. It refuses, saying why, when:
   - a renamed name would resolve elsewhere;
   - another name would come to mean the renamed one;
   - an error code would become more common.
6. **Std's sources go on disk** at `$NOVA_HOME/std/<version>-<crc32>/`,
   one read-only file per module. Each file is compared with the embedded
   text and rewritten through a temporary name when it differs. A request
   inside that cache is answered from a program held in memory.
7. **Requests re-run the front end** with the index on. Nothing is cached
   between requests.
8. **The budgets** are 200 ms for hover, definition and references, and
   400 ms for rename, which analyses twice, for the median and the
   maximum of 20 on `05-json-api` in a release build. CI's bounds are 2 s
   and 4 s with the debug binary. Measured on 2026-10-10:

   | Run | Edit | Completion | Hover | Definition | References | Rename | Binary |
   |---|---|---|---|---|---|---|---|
   | 1 | … | … | … | … | … | … | … |
   | 2 | … | … | … | … | … | … | … |
   | 3 | … | … | … | … | … | … | … |

## Alternatives

- **A separate indexer in `nova-lsp`.** It would duplicate the rules for
  shadowing, match bindings, closures and method lookup through traits,
  and drift from the checker's.
- **Growing 3.2's probe.** It answers one place. References and rename
  need every occurrence, so it would still need this index.
- **Rules for rename conflicts.** Re-analysis catches shadowing,
  captures, clashes and duplicates with one mechanism, as the formatter
  checks its own output.
- **Reaching a library's open dependents.** The answer would change with
  which files are open. The user chose the owning project.
- **Caching the last analysis.** It would need invalidation on every
  edit, for a cost ADR 0029 measured at 13-21 ms.

## Consequences

- The checker has a write-only recording path at its name sites. A new
  construct that resolves a name needs a recording place, and the
  completeness checks fail until it has one.
- Forms the checker refuses today are not navigable:
  - type aliases;
  - `import … as`;
  - record, tuple, array, or, range and `x @ pat` patterns.
- A request inside a downloaded package is answered from that package's
  own analysis, which may not resolve its own registry dependencies.
- Std's cache gains a directory for each version and build. Nothing
  removes old ones, as with the runtime's cache (ADR 0027).

## References

- `docs/superpowers/specs/2026-10-10-phase-3-4a-navigation-design.md`
- `docs/superpowers/plans/2026-10-10-phase-3-4a-navigation.md`
- ADR 0027 (the runtime's cache), ADR 0029 (the language server)
- `docs/phase-3-plan.md` §4, the 3.4 entry
```

Fill each `…` cell with the ledgered median and maximum, such as `14 / 16
ms`, and the binary column as in ADR 0029: path, bytes, build time. A
cell left as `…` is a records failure; the Step 6 scan checks for it.

- [ ] **Step 2: The dated notes**

1. `nova-spec/40-TOOLING.md`, after §3.1's "Amended 2026-10-08" paragraph:

   ```text
   **Amended 2026-10-10 (branch `phase-3-4a-navigation`):** 3.4a delivers
   hover, go to definition, find references and rename
   (`docs/superpowers/specs/2026-10-10-phase-3-4a-navigation-design.md`;
   ADR 0032). They read an index the type checker records while it
   resolves names, not salsa's queries. References and rename reach the
   owning project, and rename checks itself by analysing the renamed
   program again. Code actions and semantic highlighting are 3.4b's.
   ```

2. `docs/phase-3-plan.md`, at the end of the "### 3.4 — LSP completeness"
   entry, after its **Gate** bullet:

   ```text
     **Amended 2026-10-10:** 3.4 is two sub-phases, each with its own spec.
     3.4a, "Navigation", has hover, go to definition, find references and
     rename, and the gate's navigation half
     (`docs/superpowers/specs/2026-10-10-phase-3-4a-navigation-design.md`,
     branch `phase-3-4a-navigation`; ADR 0032). 3.4b, "Fixes and colour",
     has `Diagnostic`'s suggested edits, code actions, organize imports and
     semantic tokens, and the rest of the gate.
   ```

   Indent it two spaces, as the 3.3 entry's notes are, so it stays in the
   entry's last bullet.

3. `agent.md`, after the "**Amended 2026-10-09 (branch
   `phase-3-3b-index-publishing`):** `nova-index` …" paragraph that follows
   the crate table:

   ```text
   **Amended 2026-10-10 (branch `phase-3-4a-navigation`):** `nova-lsp` also
   answers hover, go to definition, find references and rename, from an
   index `nova-typeck` records and `nova-resolver` holds the types of
   (`docs/adr/0032-navigation-in-the-language-server.md`).
   ```

- [ ] **Step 3: The CHANGELOG, the README and ARCHITECTURE**

1. `CHANGELOG.md`, at the end of `[Unreleased]`'s `### Added` list:

   ```text
   - **Navigation in the editor.** `nova lsp` answers:
     - hover: a name's declaration, a local's inferred type, and `///` docs;
     - go to definition: into other modules, dependencies, and std, whose
       sources are written to `$NOVA_HOME/std/`;
     - find references;
     - rename.

     Rename covers the owning project, a trait method with its impls, and
     shorthand fields. It refuses, saying why, a rename that would change
     what a name means. ADR 0032.
   ```

2. `README.md`, "Editor support": change "`nova lsp` is a language server:
   it gives an editor diagnostics, completion and formatting." to "`nova
   lsp` is a language server: it gives an editor diagnostics, completion,
   formatting, hover, go to definition, find references and rename."
3. `ARCHITECTURE.md`'s crate rows:
   - `nova-resolver`: "Name resolution, module graph; the types of the
     language server's index of names";
   - `nova-typeck`: "Type inference and checking (HM + extensions); records
     the language server's index of names when asked";
   - `nova-lsp`: "The language server, `nova lsp`: diagnostics,
     completion, formatting, hover, definition, references and rename over
     `nova_driver::analyze` (ADR 0029, ADR 0032)".

Check that each code span stays on one line:

Run: ``cd /d/Projects/nona/nova && git diff -U0 -- CHANGELOG.md README.md ARCHITECTURE.md agent.md nova-spec/40-TOOLING.md docs/phase-3-plan.md | grep -E "^\+[^+]" | awk -F'`' 'NF % 2 == 0'``
Expected: no output. A printed line has an odd number of backticks, so
it opens a code span it does not close. Rewrap it so the span does not
split.

- [ ] **Step 4: The sweep**

Claims this branch makes stale can sit in files it never touches. List
them by set difference:

Run: `cd /d/Projects/nona/nova && git grep -l -i -E "completion and formatting|diagnostics, completion|hover|go to definition|find references|rename" -- . ':!docs/superpowers' ':!Cargo.lock' ':!target' | sort > $P/sweep-all.txt; git diff --name-only main...HEAD | sort > $P/sweep-touched.txt; comm -23 $P/sweep-all.txt $P/sweep-touched.txt`
Expected: a list of files the branch has not touched.

Read each match in those files with `git grep -n -i -E "<the same
pattern>" -- <file>`. For each line, decide:
- it says the server offers only diagnostics, completion and formatting;
- or it says hover, definition, references or rename are still to come;
- or neither.

If either of the first two, add a dated note (or correct a list that is
plainly a feature list) and ledger it. If neither, leave it and ledger the
file as checked. `git grep` is line-oriented, so also search for each word
alone ("hover", then "rename"), and confirm each empty result with a
single-pattern `-c`.

- [ ] **Step 5: Commit**

Write `$P/msg-14.txt`:

```text
Records for Phase 3.4a: ADR 0032, notes, CHANGELOG

ADR 0032 records the index in the checker, families, the owning
project's reach, rename's self-check, std's cache and the measured
latency. Dated notes go in 40-TOOLING §3.1, the phase plan's 3.4 entry
(split into 3.4a and 3.4b) and agent.md; the CHANGELOG, the README and
ARCHITECTURE list the new requests. <the sweep's findings, one line
each, or "The sweep found no other stale claim.">

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
```

Replace the angle-bracketed sentence with the sweep's result before
committing.

Run: `cd /d/Projects/nona/nova && git add docs/adr/0032-navigation-in-the-language-server.md nova-spec/40-TOOLING.md docs/phase-3-plan.md agent.md CHANGELOG.md README.md ARCHITECTURE.md && git add -u && git commit -q -F $P/msg-14.txt && git log -1 --format=%s`
Expected: `Records for Phase 3.4a: ADR 0032, notes, CHANGELOG`

- [ ] **Step 6: Scan the records**

Run: `cd /d/Projects/nona/nova && grep -n "…" docs/adr/0032-navigation-in-the-language-server.md; grep -n -E "TBD|TODO|<the sweep" docs/adr/0032-navigation-in-the-language-server.md CHANGELOG.md; git log -1 --format=%B | grep -c "<the sweep"`
Expected: no output from the greps, and `0`.

---

### Task 15: Final verification, mutants, and the PR body

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

Run: `cd /d/Projects/nona/nova && cargo test --workspace --no-fail-fast 2>&1 > $P/full-windows.txt; python -X utf8 $P/count.py $P/full-windows.txt`
Expected: `0 failed`. `main` at `cf0cebf` had 1701 passed and 9 ignored.
The branch adds:
- 7 + 4 resolver unit tests (Tasks 1-2);
- 22 index tests (Tasks 3-5) and 1 completeness test (Task 6);
- `FileDb::id_of`'s test, hover.rs's 4 unit tests and checker.rs's guard;
- 33 navigation tests (Tasks 7-12).

The CI-bound test was renamed, not added. Ledger the exact count; a
difference from 1701 + 73 = 1774 is explained by name, or it is a finding.

- [ ] **Step 2: The full suite on Linux**

If Docker is running:

Run: `cd /d/Projects/nona/nova && bash /c/Users/SAKEER~1/AppData/Local/Temp/gcm/linux/run.sh test --workspace --no-fail-fast 2>&1 > $P/full-linux.txt; python -X utf8 $P/count.py $P/full-linux.txt`
Expected: `0 failed`.

If it is not, the Linux run is CI's: ledger a ruling. Do not start Docker.

- [ ] **Step 3: Clippy, rustfmt and the minimum Rust**

Run: `cd /d/Projects/nona/nova && cargo clippy --workspace --all-targets -- -D warnings 2>&1 | tail -3`
Expected: `Finished`, no warning.

Run: `cd /d/Projects/nona/nova && cargo fmt --all -- --check; echo "exit $?"`
Expected: `exit 0`.

Run: `cd /d/Projects/nona/nova && RUSTFLAGS="-D warnings" cargo +1.78 check --locked --workspace 2>&1 | tail -3`
Expected: `Finished`.

Run: `cd /d/Projects/nona/nova && git diff main...HEAD -- Cargo.lock`
Expected: one added line, `"crc32fast",` in `nova-lsp`'s dependencies.

- [ ] **Step 4: The mutants (spec §8.5)**

For each, edit the code as described, run the named test, see it fail,
then undo with `git checkout -- <file>` and ledger the outcome:

| # | Mutant | File | Run | Expected |
|---|---|---|---|---|
| 1 | delete the three `note_use` calls in `check_method_call` | `crates/nova-typeck/src/check.rs` | `cargo test -p nova-driver --test index_complete` | FAIL, listing method names with "no occurrence" |
| 2 | in `check_method_call`'s `MethodRes::Trait` arm, record `Target::Def(trait_id)` instead | same | `cargo test -p nova-driver --test index method_calls_use_their_method` | FAIL |
| 3 | delete the `index.implement(…)` call in `check_impl_method_signatures` | same | `cargo test -p nova-cli --test lsp_navigation rename_a_trait_methods_family` | FAIL |
| 4 | in `Index::at`, change the end-of-name filter to `.filter(\|_\| false)` | `crates/nova-resolver/src/index.rs` | `cargo test -p nova-resolver --lib a_cursor_inside` | FAIL |
| 5 | delete `check(answer, &found, new, &edits, overlay)?;` | `crates/nova-lsp/src/rename.rs` | `cargo test -p nova-cli --test lsp_navigation a_rename_that_captures` | FAIL |
| 6 | in `plan_edits`, make both shorthand arms `(new.to_string(), 0)` | same | `cargo test -p nova-cli --test lsp_navigation rename_writes_out_a_shorthand` | FAIL |
| 7 | in `references`, change `declarations \|\| x.role == Role::Use` to `true` | `crates/nova-lsp/src/navigate.rs` | `cargo test -p nova-cli --test lsp_navigation references_with_and_without` | FAIL |
| 8 | in `renameable`, make `Owner::Std => …` `Owner::Std => {}` | `crates/nova-lsp/src/rename.rs` | `cargo test -p nova-cli --test lsp_navigation prepare_rename_refuses_what` | FAIL |
| 9 | in `write_one`, change `if !right(path) {` to `if !path.exists() {` | `crates/nova-lsp/src/std_cache.rs` | `cargo test -p nova-cli --test lsp_navigation std_cache_is_reused` | FAIL |
| 10 | in `checker.rs`'s `run`, set `index: true` | `crates/nova-lsp/src/checker.rs` | `cargo test -p nova-lsp the_diagnostics_check` | FAIL |

A mutant that compiles to nothing, or whose test panics for another
reason, has not been tested: read the failure and confirm it is the
assertion the mutant should break. Ledger each with its failing
assertion's line.

After the ten, confirm the tree is clean:

Run: `cd /d/Projects/nona/nova && git status --short`
Expected: no output.

- [ ] **Step 5: Write the PR body**

Write `$P/pr-body.md` with the Write tool. Keep local paths, process ids
and the user's name out of it. Sections, in order:

1. `## Phase 3.4a, "Navigation"`: one paragraph on what a user gets, then
   the spec, the plan and ADR 0032.
2. `### What changed`: the crates, as this plan's Architecture block
   lists them.
3. `### Changes in behaviour`:
   - the server advertises four more capabilities;
   - std's sources appear under `$NOVA_HOME/std/` after the first
     definition into std;
   - the CI-bound latency test was renamed.
4. `### Tests`: a table of Windows (local) and Linux results, `main` at
   `cf0cebf` against this branch. Linux comes from Step 2, or "from this
   PR's CI". Then the completeness checks' 159 files, the 364 cut
   programs, and "each of the spec's ten mutants fails its named test".
5. `### Latency`: the three runs' figures, as in ADR 0032.
6. `### Final review`: filled in after the review (the executing-plans
   skill's fix pass).
7. `### Decisions to review`: this plan's Decisions 1-16 by title, and
   every ledger `Ruling:` line, each with its cost if wrong.
8. `### Deferred minors`: from the final review.
9. The attribution line: `🤖 Generated with [Claude Code](https://claude.com/claude-code)`.

Run: `grep -n -i -E "AppData|SAKEER|/c/Users|C:.Users" $P/pr-body.md`
Expected: no output. (`.` stands for either slash, so the pattern holds
no backslash for the Bash tool to collapse.)

There is no commit in this task: `pr-body.md` is outside the repository.
Push and open the PR only after the final review and its fix pass, as the
standing workflow says.
