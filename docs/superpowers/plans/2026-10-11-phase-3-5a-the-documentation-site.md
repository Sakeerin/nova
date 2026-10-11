# Phase 3.5a, "The documentation site", Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** `nova doc` writes a static site under `target/doc/`: a page per
module, signatures as written with every name linked through the
checker's index, Markdown docs, search, and a light and a dark theme. std
gets docs for every public item, and an example that uses a dependency
carries the gate. PR #108's nine deferred minors come first.

**Architecture:**
- **The follow-ups** touch `nova-fmt` (organize imports' header),
  `nova-driver` (the loader's cache), `nova-lsp` (a guard),
  `nova-diagnostics` (`suggest`, `lines`), two test files and two notes.
- **`nova-driver`:** `Analysis` keeps the merged AST, `file`.
- **`nova-doc`** goes from a stub to the generator:
  - `model.rs`: analyses to a `Site`;
  - `signature.rs`: signatures as written, with links;
  - `markdown.rs` and `highlight.rs`: docs and Nova code;
  - `render.rs` and `search.rs`: pages and the search index;
  - `links.rs`: the link check;
  - `write.rs`: writing `target/doc/`, behind its marker;
  - `assets/nova.css` and `assets/nova.js`.
- **`nova-cli`:** `cmd/doc.rs`, `nova doc [--open] [--std]`.
- **std:** `//!` and `///` docs in all 17 modules.
- **`examples/05-geometry`:** an app and a path library.
- **CI:** `.github/scripts/doc-gate.sh` in the `install` job; a
  `std-docs` job in `release.yml`.

**Tech Stack:**
- Rust: MSRV 1.78, edition 2021.
- New crates: `pulldown-cmark` `=0.13.4` (default features off, `html`),
  which brings `pulldown-cmark-escape` 0.11.0 and `unicase` 2.10.0. The
  user approved `pulldown-cmark` on 2026-10-11 (spec decision 1). Nothing
  else is added; `thiserror` and `serde_json` are already locked.
- Tests: unit tests in `nova-doc`; library-level gate tests in
  `crates/nova-doc/tests/`; `nova` driven through `assert_cmd` in
  `crates/nova-cli/tests/doc.rs`.

**Spec:** `docs/superpowers/specs/2026-10-11-phase-3-5a-the-documentation-site-design.md`,
approved by the user on 2026-10-11.
- Read it before Task 1: it is the authority this plan argues from.
- Its §20 lists 28 decisions, and its §2 the code as it stood.

## Global Constraints

- **The workspace's minimum Rust stays 1.78,** edition 2021. CI's MSRV job
  runs `cargo check --locked --workspace` with `RUSTFLAGS=-D warnings`.
  No `Option::is_none_or` (1.82), no `std::path::absolute` (1.79): write
  `map_or(true, …)`, and join a relative path onto `current_dir()`.
- **`Cargo.lock` changes once, in Task 8,** by exactly three new packages
  (`pulldown-cmark`, `pulldown-cmark-escape`, `unicase`) and the new edges
  of `nova-doc` and `nova-cli`. Any other change to it stops the task:
  ask the user.
- **No `unwrap()` outside `#[cfg(test)]` and test files:** `expect("why")`
  or `?` (agent.md §4). Errors implement `std::error::Error` through
  `thiserror`.
- **Tests never touch the internet.** Every CLI test sets `NOVA_HOME` to a
  fresh directory. Every test directory has a fixed name under the
  system's temporary directory, removed and made again at the test's
  start.
- **`nova doc` writes only under the `target/doc/` it chose,** and removes
  only a `target/doc/` that holds its marker (spec §4.3).
- **The site is the same bytes on every system** (spec §6.1): no
  `HashMap` order reaches the output, and every file is written with
  `\n`.
- **Nothing is published, and no tag is pushed.** Merge by rebase, on the
  user's word only. Never enable auto-merge.
- **This machine is Linux.** Windows and macOS results come from CI, which
  runs all three systems.

## Review Focus

The five inputs most likely to bite a user that the spec's tests leave
uncovered, most likely first. Each has its test in the task that owns the
code:

1. **A signature broken over lines by `nova fmt`,** inside an impl, with
   a comment on one of its lines. The page must show it dedented, without
   the comment, and without a blank line →
   `a_broken_signature_keeps_its_lines_less_their_indentation` (Task 10).
2. **A doc with raw HTML, including `<script>`.** It must show as text →
   `raw_html_shows_as_text` (Task 9).
3. **A package page reached through `file://` on Windows.** Every URL
   must be relative and use `/` → `every_url_is_relative_with_slashes`
   (Task 12).
4. **A user's relative link to a page the site lacks.** `nova doc` must
   warn, naming the page and the link, and still exit 0 →
   `a_broken_doc_link_is_a_warning` (Task 14).
5. **A second `nova doc` after a module is deleted.** Its page must be
   gone → `a_second_run_replaces_the_site` (Task 14).

## Decisions: where this plan settles what the spec leaves open

1. **The interfaces.**
   - **`nova-driver`:** `Analysis::file: Option<nova_ast::File>`.
   - **`nova-doc`:** `document_package(root: &Path) -> Result<Documented,
     Failure>`, `document_std() -> Result<Documented, Failure>`,
     `write_site(site: &Site, dir: &Path) -> Result<PathBuf, WriteError>`,
     `check_links(dir: &Path) -> Vec<BrokenLink>`, `Documented { site, db,
     warnings }`, `Failure::{Diagnostics { db, diagnostics }, Other(String)}`,
     `Site` (opaque), `BrokenLink { page, link }`, `WriteError`, and
     `MARKER`.
   - **`nova-lsp`:** `analysis::answered`.
   - **`nova-diagnostics`:** `lines::line_after` and
     `lines::line_before_item` keep their signatures.
2. **Follow-up 1's rule:** comment lines that begin the file, at byte 0,
   are never an import's lead. Comments a blank line separates from the
   first import were already left alone; this extends that to comments
   directly above it when they open the file.
3. **Follow-up 2 caches `Loader::has`,** not `find`: the failed read is
   `has`'s fallback for a name the directory listing lacks. The cache is
   per load, keyed by directory and stem.
4. **Follow-up 3's guard is `analysis::answered(f, default)`,** which
   runs `f` under `catch_unwind`, logs a panic, and returns the default.
   Code actions and semantic tokens use it. The spec's "a handler made to
   panic" is tested as the guard given a closure that panics: no input is
   known to make either handler panic, and a fault switch in the binary
   would ship a test path to users. The handlers' use of the guard is
   checked by reading, and ledgered.
5. **Follow-up 9 finds comments by a forward scan.** Before the first
   item there can be only comments and blank space, and block comments do
   not nest, so a scan from byte 0 finds every comment exactly. After an
   import, the scan skips spaces and whole block comments to the line's
   end.
6. **`unicase` stays at 2.10.0,** the version Cargo picks. A probe crate
   on 2026-10-11 built and ran `pulldown-cmark` 0.13.4 with it on the
   local 1.78 toolchain (spec §15.1's rule).
7. **A program-only package documents its private members too:** its
   records' private fields and its impls' private methods. Spec §5.1 says
   it documents "every item"; a private field of a documented record
   would otherwise show as "not documented" in a program where nothing is
   hidden.
8. **std's analysis has a loose entry no file holds,**
   `<nova doc>/main.nova`, holding `fn main() {}`, served by a `Sources`
   of its own. It runs with `tests` on, so `std/test` is in it, and with
   `module_only`.
9. **The root package's analysis has one root.** `Program::for_package`
   with `Roots::Check` lists `src/main.nova` then `src/lib.nova`. The
   plan keeps `lib.nova` when it is listed, otherwise `main.nova`, and
   sets `runs = false`, with `module_only` on.
10. **A signature's end comes from the AST where it can.**
    - A function's or method's: the `)` closing its parameters, its
      return type, and its `where` bounds, whichever ends last.
    - A record's, trait's or impl's: the first `{` after the name. A sum
      type's: the first `=`.
    - A constant's: its value's end. A field's: its type's end. A
      variant's: its payload's `)`, or its name.
    - An associated type's: its last bound, or its name.
11. **The text between two tokens** is only spaces and comments. After
    the comments go, it shows as nothing, one space, or a line break
    followed by the next line's indentation less the starting line's
    (`signature::gap`). A block comment counts as a space, so `a/*c*/b`
    stays two words.
12. **Headings.** A page's title is `<h1>`, a section `<h2>`, an item
    `<h3>` holding its kind and name, and a member group `<h4>`. Module
    docs move down 1 level, item docs 3, member and impl docs 4.
13. **Every URL is relative to its page,** built from site-root paths by
    prefixing one `../` per `/` in the page's path.
14. **The search index** is `window.NOVA_SEARCH`, an array of
    `[name, kind, path, url, summary]`, one per line, written with
    `serde_json`, sorted by path, then kind, then URL. It holds packages,
    modules, items, members, impl methods, primitive types and builtins.
15. **The CLI prints warnings** through `render::emit_all` and errors the
    same way, then fails with "could not document due to N previous
    error(s)". On success it prints the start page's path alone on
    stdout, so a script can read it.
16. **`--open` spawns its program and does not wait,** with stdout and
    stderr discarded, giving it the page's absolute path:
    `current_dir().join(path)`.
17. **Where each test lives:**
    - the gate tests and std's completeness check, in `crates/nova-doc/tests/`,
      through `nova-doc`'s library, with the expected anchors enumerated
      by the parser alone (`tests/common/mod.rs`);
    - the command's behaviour, in `crates/nova-cli/tests/doc.rs`;
    - the installed binary, in `doc-gate.sh`.
18. **std's docs land module by module.** `std_docs.rs` checks the
    modules in its `DONE` list. Each std task adds its modules, watches
    the test fail, writes the docs, and watches it pass. The last task
    replaces the list with every module.
19. **The builtins' docs and the primitive types' descriptions** are text
    in `nova-doc` (`model.rs`, `BUILTINS` and `PRIMITIVES`). The plan
    gives the text, written from the runtime and std.
20. **Impl headers carry no `id`.** Only the methods under them do (spec
    §6.2 lists no impl `id`), so nothing links to a header.
21. **The release workflow's triggers do not change.** This branch
    changes `Cargo.lock` and `crates/nova-cli/Cargo.toml`, which already
    trigger it, so its pull request runs the new job.
22. **Where the plan refines the spec, the plan records it:** decisions
    3, 4, 7, 9 to 12, 14 and 20 make the spec exact where it is silent or
    loose, and the spec does not repeat them.

## File Structure

| File | Task | Responsibility |
|---|---|---|
| `crates/nova-fmt/src/organize.rs`, `tests/organize.rs` | 1 | Follow-up 1 |
| `crates/nova-driver/src/program.rs`, `tests/analyze.rs` | 2 | Follow-up 2 |
| `crates/nova-lsp/src/{analysis,lib}.rs` | 3 | Follow-up 3 |
| `crates/nova-driver/tests/broken.rs` | 4 | Follow-up 4 |
| `crates/nova-cli/tests/{lsp_navigation,lsp_fixes}.rs`, `docs/phase-3-plan.md`, `nova-spec/40-TOOLING.md` | 5 | Follow-ups 5, 6, 7 |
| `crates/nova-diagnostics/src/suggest.rs` | 6 | Follow-up 8 |
| `crates/nova-diagnostics/src/lines.rs` | 7 | Follow-up 9 |
| `Cargo.toml`, `Cargo.lock`, `crates/nova-doc/Cargo.toml`, `crates/nova-driver/src/analyze.rs`, `tests/analyze.rs` | 8 | The crates; `Analysis::file` |
| `crates/nova-doc/src/{lib,markdown,highlight}.rs` | 9 | Markdown and Nova code |
| `crates/nova-doc/src/signature.rs` | 10 | Signatures |
| `crates/nova-doc/src/model.rs`, `src/lib.rs` | 11 | The model |
| `crates/nova-doc/src/{render,search}.rs`, `assets/{nova.css,nova.js}` | 12 | Pages, search, themes |
| `crates/nova-doc/src/{links,write}.rs` | 13 | The link check; writing the site |
| `crates/nova-cli/{Cargo.toml,src/main.rs,src/cmd/{mod,doc}.rs}`, `tests/doc.rs` | 14 | `nova doc` |
| `crates/nova-doc/tests/{common/mod.rs,std_docs.rs}`, `std/{core}/lib.nova` | 15 | The completeness check; std/core |
| `std/{collections,strings,bytes,fmt}/lib.nova` | 16 | std's docs, part 2 |
| `std/{io,fs,net,process}/lib.nova` | 17 | std's docs, part 3 |
| `std/{task,sync,time,log}/lib.nova` | 18 | std's docs, part 4 |
| `std/{json,http,crypto,test}/lib.nova`, `tests/std_docs.rs` | 19 | std's docs, part 5 |
| `examples/05-geometry/**`, `crates/nova-cli/tests/run_tests.rs` | 20 | The example |
| `crates/nova-doc/tests/gate.rs`, `.github/scripts/doc-gate.sh`, `.github/workflows/ci.yml` | 21 | The gate |
| `.github/workflows/release.yml` | 22 | std's site on each release |
| `docs/adr/0034-the-documentation-generator.md` (new), and the records Task 23 lists | 23 | ADR, notes, CHANGELOG, README, ARCHITECTURE, sweep |
| `$P/pr-body.md`, outside the repository | 24 | Final verification, mutants, the PR body |

## Conventions for every task

- **Working directory:** `/home/user/nova`. The Bash tool resets its
  directory after each call, so write `cd /home/user/nova && …` in one
  command.
- **Scratch directory:** `P=/tmp/claude-0/-home-user-nova/1504a555-d3ea-5356-b003-28f2066a0d92/scratchpad`,
  outside the repository. Long output goes to a file there, and you read
  its tail.
- **The ledger:** `$P/ledger-35a.md`. Every ruling made while executing
  gets a line `Ruling: <what> (Task N). Cost if wrong: <cost>.`. Every
  measured figure the plan asks for goes there too.
- **Placing code:** copy each block from this plan exactly, with the Edit
  or Write tool. Never retype plan code from memory.
- **Commit messages:** write each with the Write tool to a new file,
  `$P/msg-35a-<task>.txt`, then `git commit -F` it in the next call, and
  check `git log -1 --format=%s`. Every message ends with the two
  attribution lines the session gives.
- **Format before every commit:** run `cargo fmt --all`, then stage what
  it changed. Chain a commit and what follows it with `&&`, never `;`.
- **Counting a full run:** `cargo test --workspace --locked 2>&1 | tee
  $P/full.txt`, then `grep -E "^test result:" $P/full.txt | awk '{p+=$4;
  f+=$6; i+=$8} END {print p" passed, "f" failed, "i" ignored"}'`.
- **The MSRV check:** `RUSTFLAGS="-D warnings" cargo +1.78 check --locked
  --workspace`. The 1.78 toolchain is installed here.
- **Mutants run only on committed work,** and are undone with
  `git checkout -- <file>`.
- **Expected outputs are exact.** If a step's output differs:
  - when the code does what this plan describes and the expected value is
    wrong, correct the value and ledger a ruling;
  - when the code does something else, fix the code.
- **A test written after its code, on purpose,** is marked *guard* in its
  comment, and is expected to pass on its first run.
- **Anchors.** This plan places each insertion by the code around it, as
  it stood at `14dc6e1`. If `cargo fmt` reflowed an earlier task's code
  and an anchor differs, follow the code's meaning and ledger the
  difference.
- **No local paths and no personal names** go into the repository, a
  commit message or the PR body.

---
### Task 0: The baseline

**Files:** none.

- [ ] **Step 1: Record `main`'s test counts**

Run: `cd /home/user/nova && git status --short && git log --oneline -1 && cargo test --workspace --locked 2>&1 | tee $P/full-base.txt | tail -3`
then the counting one-liner of the Conventions on `$P/full-base.txt`.
Expected: a clean tree on this branch, whose only commits beyond `main`
are the spec's and this plan's. Ledger the counts as "Baseline: P passed,
F failed, I ignored". The PR body's test table compares against them.

---

### Task 1: Follow-up 1, organize imports leaves a file's header

**Files:**
- Modify: `crates/nova-fmt/src/organize.rs` (the `lead_start` in
  `organize`)
- Test: `crates/nova-fmt/tests/organize.rs`

**Interfaces:** none change.

- [ ] **Step 1: Write the failing test**

Insert after the test `comments_travel_with_their_imports_and_a_header_stays`
in `crates/nova-fmt/tests/organize.rs`:

```rust
#[test]
fn comments_that_start_the_file_stay_at_its_top() {
    // Follow-up 1 (spec 3.5a §3; plan decision 2): they are the file's
    // header, not its first import's lead.
    let unused_m = |v: &ImportView| Verdict {
        group: Group::Module,
        unused_glob: v.first == "m",
        unused_names: Vec::new(),
    };
    let text = "// header\nimport m\nimport a\n\nfn main() {}\n";
    assert_eq!(
        organized(text, &unused_m),
        "// header\nimport a\n\nfn main() {}\n"
    );
    assert_eq!(
        organized(text, &keep),
        "// header\nimport a\nimport m\n\nfn main() {}\n"
    );
}
```

- [ ] **Step 2: Run it to see it fail**

Run: `cd /home/user/nova && cargo test -p nova-fmt --test organize comments_that_start 2>&1 | tail -15`
Expected: FAIL. The first assertion's left side is
`"import a\n\nfn main() {}\n"`: the header went with `m`.

- [ ] **Step 3: The rule**

In `crates/nova-fmt/src/organize.rs`, replace:

```rust
        // After another import, every line between them is this one's lead
        // (spec §6.5); after anything else, the comments directly above.
        let lead_start = if k > 0 && imports[k - 1] + 1 == i {
            regions[k - 1].1
        } else {
            comments_above(text, first_line)
        };
```

with:

```rust
        // After another import, every line between them is this one's lead
        // (spec §6.5); after anything else, the comments directly above,
        // unless they begin the file, which makes them its header (spec
        // 3.5a §3, follow-up 1).
        let lead_start = if k > 0 && imports[k - 1] + 1 == i {
            regions[k - 1].1
        } else {
            match comments_above(text, first_line) {
                0 => first_line,
                above => above,
            }
        };
```

- [ ] **Step 4: Run the crate's tests**

Run: `cd /home/user/nova && cargo test -p nova-fmt 2>&1 | grep -E "^test result|FAILED|panicked" | head -20`
Expected: every `test result:` line `ok`; `a_doc_comment_travels_with_its_import`
still passes (its `///` is part of the item, so its first line is byte 0
either way).

- [ ] **Step 5: Commit**

Message: `fix: organize imports keeps the comments that start a file` and a
body: the comment lines directly above the first import, when they begin
the file, are its header; removing or moving that import used to take
them (PR #108's deferred minor 1).

---

### Task 2: Follow-up 2, the loader probes a missing sibling once

**Files:**
- Modify: `crates/nova-driver/src/program.rs` (`Loader`, its
  construction in `load_program`, and `has`)
- Test: `crates/nova-driver/tests/packages.rs`

**Interfaces:** none change.

- [ ] **Step 1: Write the failing test**

Append to `crates/nova-driver/tests/packages.rs`:

```rust
/// A `Sources` that counts every path it is asked for.
struct Counting(std::sync::Mutex<Vec<PathBuf>>);

impl nova_driver::Sources for Counting {
    fn read(&self, path: &Path) -> std::io::Result<String> {
        self.0.lock().unwrap().push(path.to_path_buf());
        std::fs::read_to_string(path)
    }
}

/// Follow-up 2 (spec 3.5a §3; plan decision 3): a sibling file that does
/// not exist is probed once per directory and name, however many modules
/// that directory holds.
#[test]
fn a_missing_sibling_is_probed_once_per_directory() {
    let dir = fresh("probe-once");
    geom(&dir);
    write(
        &dir.join("app"),
        &[
            ("nova.toml", manifest("app", GEOM_DEPENDENCY).as_str()),
            (
                "src/main.nova",
                "import geom\nimport a\nimport b\n\nfn main() {}\n",
            ),
            ("src/a.nova", "pub fn x() -> Int { 1 }\n"),
            ("src/b.nova", "pub fn y() -> Int { 2 }\n"),
        ],
    );
    let sources = Counting(std::sync::Mutex::new(Vec::new()));
    let a = analyze_program(
        Program::for_package(&dir.join("app"), Roots::Check),
        &sources,
        &options(false),
    )
    .expect("the entry reads");
    assert_eq!(codes(&a), Vec::<&str>::new());
    let probes = sources
        .0
        .lock()
        .unwrap()
        .iter()
        .filter(|p| p.ends_with("geom.nova"))
        .count();
    assert_eq!(probes, 1);
}
```

- [ ] **Step 2: Run it to see it fail**

Run: `cd /home/user/nova && cargo test -p nova-driver --test packages a_missing_sibling 2>&1 | tail -15`
Expected: FAIL with `left: 4` and `right: 1`: one probe for the import, and
one for each of `main`, `a` and `b` in the importable loop.

- [ ] **Step 3: The cache**

In `crates/nova-driver/src/program.rs`, in `struct Loader`, after the
`listings` field:

```rust
    /// Each `has` answered, by directory and stem: a name the listing lacks
    /// costs one read per load, not one per module (spec 3.5a §3,
    /// follow-up 2).
    known: HashMap<(PathBuf, String), bool>,
```

In `load_program`, where the loader is built, after `listings: HashMap::new(),`:

```rust
        known: HashMap::new(),
```

Replace `has`'s doc comment and first line,

```rust
    /// Whether `dir` holds `<stem>.nova`, its name matching exactly (spec
    /// §4.3). A name the disk does not hold in any case may be an editor's
    /// unsaved buffer, read through `Sources`.
    ///
    /// An empty `dir`, a loose entry named without one (`nova run
    /// main.nova`), is the current directory: `read_dir("")` fails on every
    /// system, which would leave the case-insensitive read to decide.
    fn has(&mut self, dir: &Path, stem: &str) -> bool {
```

with:

```rust
    /// Whether `dir` holds `<stem>.nova` ([`Loader::look`]), answered once
    /// per load (spec 3.5a §3, follow-up 2).
    fn has(&mut self, dir: &Path, stem: &str) -> bool {
        let key = (dir.to_path_buf(), stem.to_string());
        if let Some(&known) = self.known.get(&key) {
            return known;
        }
        let found = self.look(dir, stem);
        self.known.insert(key, found);
        found
    }

    /// Whether `dir` holds `<stem>.nova`, its name matching exactly (spec
    /// §4.3). A name the disk does not hold in any case may be an editor's
    /// unsaved buffer, read through `Sources`.
    ///
    /// An empty `dir`, a loose entry named without one (`nova run
    /// main.nova`), is the current directory: `read_dir("")` fails on every
    /// system, which would leave the case-insensitive read to decide.
    fn look(&mut self, dir: &Path, stem: &str) -> bool {
```

The old body of `has` is now `look`'s, unchanged.

- [ ] **Step 4: Run the crate's tests**

Run: `cd /home/user/nova && cargo test -p nova-driver 2>&1 | grep -E "^test result|FAILED|panicked" | head -20`
Expected: every `test result:` line `ok`.

- [ ] **Step 5: Commit**

Message: `perf: the loader probes a missing sibling file once per load`,
body: the importable loop asked `has` for every dependency's import name
from every module, and each miss read the disk (PR #108's deferred minor
2).

---

### Task 3: Follow-up 3, code actions and semantic tokens are guarded

**Files:**
- Modify: `crates/nova-lsp/src/analysis.rs` (a new `answered`, and a new
  `mod tests` at the end)
- Modify: `crates/nova-lsp/src/lib.rs` (the `CodeActionRequest` and
  `SemanticTokensFullRequest` arms of `request`)

**Interfaces:**
- Produces: `pub fn answered<T>(what: &str, f: impl FnOnce() -> T,
  default: T) -> T` in `analysis`.

- [ ] **Step 1: Write the failing test**

Append to `crates/nova-lsp/src/analysis.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_panic_while_answering_gives_the_default() {
        // Follow-up 3 (spec 3.5a §3; plan decision 4).
        let got: Vec<u32> = answered("a test", || panic!("planted"), Vec::new());
        assert!(got.is_empty());
        assert_eq!(answered("a test", || vec![1u32], Vec::new()), vec![1]);
    }
}
```

- [ ] **Step 2: Run it to see it fail**

Run: `cd /home/user/nova && cargo test -p nova-lsp --lib a_panic_while 2>&1 | grep -E "^error" | head -5`
Expected: `error[E0425]: cannot find function \`answered\` in this scope`.

- [ ] **Step 3: The guard**

In `crates/nova-lsp/src/analysis.rs`, after `guarded`:

```rust
/// `f`'s answer, or `default` when it panics, logged: what a request does
/// after its analysis, guarded as the analysis is (spec 3.5a §3,
/// follow-up 3).
pub fn answered<T>(what: &str, f: impl FnOnce() -> T, default: T) -> T {
    match std::panic::catch_unwind(std::panic::AssertUnwindSafe(f)) {
        Ok(v) => v,
        Err(_) => {
            tracing::warn!("nova lsp: answering {what} panicked");
            default
        }
    }
}
```

In `crates/nova-lsp/src/lib.rs`, in the `CodeActionRequest::METHOD` arm,
replace

```rust
                                code_action::code_actions(
                                    &answer,
                                    &path,
                                    start,
                                    end,
                                    p.context.only.as_deref(),
                                    &|path| self.uri_of(path),
                                )
```

with

```rust
                                analysis::answered(
                                    "code actions",
                                    || {
                                        code_action::code_actions(
                                            &answer,
                                            &path,
                                            start,
                                            end,
                                            p.context.only.as_deref(),
                                            &|path| self.uri_of(path),
                                        )
                                    },
                                    Vec::new(),
                                )
```

and in the `SemanticTokensFullRequest::METHOD` arm, replace

```rust
                            .map(|(answer, _, _)| tokens::tokens(&answer))
```

with

```rust
                            .map(|(answer, _, _)| {
                                analysis::answered(
                                    "semantic tokens",
                                    || tokens::tokens(&answer),
                                    Vec::new(),
                                )
                            })
```

- [ ] **Step 4: Run the crate's tests and the stdio tests**

Run: `cd /home/user/nova && cargo test -p nova-lsp 2>&1 | grep -E "^test result|FAILED" && cargo test -p nova-cli --test lsp_fixes 2>&1 | grep -E "^test result|FAILED"`
Expected: every `test result:` line `ok`. Ledger: "Ruling: follow-up 3's
handlers are checked by reading; the guard is tested with a closure that
panics (Task 3; plan decision 4). Cost if wrong: a handler that drops the
guard again is caught only by review."

- [ ] **Step 5: Commit**

Message: `fix: a panic answering code actions or tokens no longer ends the
server`, body: both run on every cursor move and ran outside the guard the
analysis has (PR #108's deferred minor 3).

---

### Task 4: Follow-up 4, the sweep plants a two-file program

**Files:**
- Modify: `crates/nova-driver/tests/broken.rs`

**Interfaces:** none.

- [ ] **Step 1: Two in-memory files in `Cut`**

In `crates/nova-driver/tests/broken.rs`, replace `struct Cut` and its
`impl Sources`:

```rust
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
```

with:

```rust
/// One file's text in place of its contents, and files no disk holds;
/// every other file from disk.
struct Cut {
    path: PathBuf,
    text: String,
    /// A planted program's other files, served whole (spec 3.5a §3,
    /// follow-up 4).
    extra: Vec<(PathBuf, String)>,
}

impl Sources for Cut {
    fn read(&self, path: &Path) -> std::io::Result<String> {
        if path == self.path {
            Ok(self.text.clone())
        } else if let Some((_, text)) = self.extra.iter().find(|(p, _)| p == path) {
            Ok(text.clone())
        } else {
            std::fs::read_to_string(path)
        }
    }
}
```

- [ ] **Step 2: The planted program**

After `const FIX_BEARING`, add:

```rust
/// Follow-up 4 (spec 3.5a §3): the two fixes that need a second file,
/// planted in a loose program of two files no disk holds. `main` lists
/// `one` from `util` and uses `two` without importing it (an import fix),
/// and lists `secret`, which is private (a make-public fix in `util`).
const TWO_MAIN: &str = "import util::{one, secret}\n\nfn main() {\n    println(\"${one()} ${two()} ${secret()}\")\n}\n";
const TWO_UTIL: &str =
    "pub fn one() -> Int { 1 }\n\npub fn two() -> Int { 2 }\n\nfn secret() -> Int { 3 }\n";
```

- [ ] **Step 3: The jobs carry their extra files**

In `cut_programs_never_panic_or_hang`, make each job a triple. Replace

```rust
    let mut jobs: Vec<(PathBuf, String)> = Vec::new();
    for (path, n) in &programs {
        let text = std::fs::read_to_string(path).unwrap();
        for cut in cuts(&text, *n) {
            jobs.push((path.clone(), cut));
        }
    }
```

with

```rust
    let mut jobs: Vec<(PathBuf, String, Vec<(PathBuf, String)>)> = Vec::new();
    for (path, n) in &programs {
        let text = std::fs::read_to_string(path).unwrap();
        for cut in cuts(&text, *n) {
            jobs.push((path.clone(), cut, Vec::new()));
        }
    }
```

replace

```rust
    jobs.push((planted.clone(), FIX_BEARING.to_string()));
    for cut in cuts(FIX_BEARING, 16) {
        jobs.push((planted.clone(), cut));
    }
```

with

```rust
    jobs.push((planted.clone(), FIX_BEARING.to_string(), Vec::new()));
    for cut in cuts(FIX_BEARING, 16) {
        jobs.push((planted.clone(), cut, Vec::new()));
    }
    // The two-file fixes, whole and at 16 cuts of `main` (follow-up 4).
    let two = root.join("tests").join("broken-two");
    let extra = vec![(two.join("util.nova"), TWO_UTIL.to_string())];
    jobs.push((two.join("main.nova"), TWO_MAIN.to_string(), extra.clone()));
    for cut in cuts(TWO_MAIN, 16) {
        jobs.push((two.join("main.nova"), cut, extra.clone()));
    }
```

In the worker loop, replace

```rust
            let Some((path, text)) = queue.lock().unwrap().pop() else {
                break;
            };
```

with

```rust
            let Some((path, text, extra)) = queue.lock().unwrap().pop() else {
                break;
            };
```

and

```rust
            let sources = Cut {
                path: path.clone(),
                text: text.clone(),
            };
```

with

```rust
            let sources = Cut {
                path: path.clone(),
                text: text.clone(),
                extra,
            };
```

- [ ] **Step 4: Its own vacuity check**

In the results loop, replace `let mut checked = 0usize;` with:

```rust
    let mut checked = 0usize;
    // The two-file program's fixes: both, from the whole program at least.
    let mut two_checked = 0usize;
```

replace `checked += fixes.unwrap_or(0);` with:

```rust
                let n = fixes.unwrap_or(0);
                checked += n;
                if path.starts_with(&two) {
                    two_checked += n;
                }
```

and after the final `assert!(checked > 0, "no cut program had a fix");`:

```rust
    assert!(
        two_checked >= 2,
        "the two-file program carried {two_checked} fixes, not its import and make-public fixes"
    );
```

- [ ] **Step 5: Run the sweep**

Run: `cd /home/user/nova && cargo test -p nova-driver --test broken -- --nocapture 2>&1 | grep -E "fixes checked|test result|panicked" | head`
Expected: `test result: ok. 1 passed`, and a "fixes checked" count at
least 2 above PR #108's 44. Ledger the count.

- [ ] **Step 6: The mutant**

Break the make-public fix's planting: change `fn secret()` in `TWO_UTIL`
to `pub fn secret()`, run Step 5's command, and expect FAIL naming the
two-file program's fix count. Undo the change.

- [ ] **Step 7: Commit**

Message: `test: the cut-program sweep plants the two-file fixes`, body: an
import fix and a make-public fix, which need a second file, now go through
the sweep too (PR #108's deferred minor 4).

---

### Task 5: Follow-ups 5, 6 and 7: test directories and two notes

**Files:**
- Modify: `crates/nova-cli/tests/lsp_navigation.rs` (lines 984, 987)
- Modify: `crates/nova-cli/tests/lsp_fixes.rs` (line 484)
- Modify: `docs/phase-3-plan.md` (after the 3.4b note in §4's 3.4 entry)
- Modify: `nova-spec/40-TOOLING.md` (§3.1's 3.4b note)

- [ ] **Step 1: The directories**

In `lsp_navigation.rs`, `app_and_library("gate", GATE_LIB)` becomes
`app_and_library("navigation-gate", GATE_LIB)` and `home("gate")` becomes
`home("navigation-gate")`. In `lsp_fixes.rs`, the `"gate",` at line 484
becomes `"fixes-gate",`. `crates/nova-cli/tests/fmt.rs`'s
`fresh_dir("gate")` stays: its helper's prefix differs.

Run: `cd /home/user/nova && grep -n '"gate"' crates/nova-cli/tests/lsp_*.rs`
Expected: no output.

- [ ] **Step 2: The phase plan's note**

In `docs/phase-3-plan.md`, after the paragraph ending "test also checks
semantic tokens and a quick fix." in the 3.4 entry, add a blank line and:

```markdown
  **Amended 2026-10-11 (branch `claude/loving-archimedes-frtf5q`):** the
  gate above was met in PR #108's CI, on Linux, Windows and macOS and in
  the VS Code smoke test, after the note was written.
```

- [ ] **Step 3: 40-TOOLING's note**

In `nova-spec/40-TOOLING.md` §3.1's 3.4b note, replace the line
`With 3.4a, every Phase 3 row of this table is delivered.` with
`With 3.2 and 3.4a, every Phase 3 row of this table is delivered.`

- [ ] **Step 4: Run the two stdio test files**

Run: `cd /home/user/nova && cargo test -p nova-cli --test lsp_navigation --test lsp_fixes 2>&1 | grep -E "^test result|FAILED"`
Expected: both `ok`.

- [ ] **Step 5: Commit**

Message: `chore: the LSP gate tests' own directories, and two notes put
right`, body: PR #108's deferred minors 5, 6 and 7.

---

### Task 6: Follow-up 8, "did you mean" skips what cannot qualify

**Files:**
- Modify: `crates/nova-diagnostics/src/suggest.rs` (`closest`, and its
  `mod tests`)

- [ ] **Step 1: The guard test**

Append to `suggest.rs`'s `mod tests`:

```rust
    #[test]
    fn a_candidate_too_long_or_short_to_qualify_is_skipped() {
        // Guard (follow-up 8, spec 3.5a §3): the behaviour must not change
        // when such candidates are skipped before their distance is taken.
        assert_eq!(closest("count", ["countdown", "cnt", "coun"]), Some("coun"));
        assert_eq!(closest("count", ["countdowns", "c"]), None);
        assert_eq!(closest("count", ["COUNT"]), Some("COUNT"));
    }
```

Run: `cd /home/user/nova && cargo test -p nova-diagnostics suggest 2>&1 | grep -E "^test result|FAILED"`
Expected: `ok`, before the change too.

- [ ] **Step 2: The skip**

In `closest`, replace

```rust
    let bound = name.chars().count().max(3) / 3;
```

with

```rust
    let len = name.chars().count();
    let bound = len.max(3) / 3;
```

and replace

```rust
        } else {
            let d = distance(name, c);
```

with

```rust
        } else {
            // Its distance is at least the difference in length (spec
            // 3.5a §3, follow-up 8).
            if c.chars().count().abs_diff(len) > bound {
                continue;
            }
            let d = distance(name, c);
```

- [ ] **Step 3: Run the crate's tests**

Run: `cd /home/user/nova && cargo test -p nova-diagnostics 2>&1 | grep -E "^test result|FAILED"`
Expected: `ok`.

- [ ] **Step 4: Commit**

Message: `perf: "did you mean" skips a candidate whose length rules it out`,
body: PR #108's deferred minor 8.

---

### Task 7: Follow-up 9, the import fix's lines step over block comments

**Files:**
- Modify: `crates/nova-diagnostics/src/lines.rs` (`line_after`,
  `line_before_item`, a new private `comments_before`, and `mod tests`)

- [ ] **Step 1: Write the failing tests**

Append to `lines.rs`'s `mod tests`:

```rust
    #[test]
    fn a_line_after_an_import_goes_past_a_block_comment_that_goes_on() {
        // Follow-up 9 (spec 3.5a §3; plan decision 5).
        let text = "import a /* one\n   two */\nfn main() {}\n";
        let after = text.find("two */\n").unwrap() + "two */\n".len();
        assert_eq!(
            line_after(text, "import a".len(), "import b::{x}"),
            (after, "import b::{x}\n".to_string())
        );
        let crlf = text.replace('\n', "\r\n");
        let after = crlf.find("two */\r\n").unwrap() + "two */\r\n".len();
        assert_eq!(
            line_after(&crlf, "import a".len(), "import b::{x}"),
            (after, "import b::{x}\r\n".to_string())
        );
    }

    #[test]
    fn a_line_before_an_item_goes_above_a_block_comment_over_it() {
        // Follow-up 9: the comment's blank line is inside it, and the
        // header a blank line separates stays above.
        let text = "// header\n\n/* about f\n\n   more */\nfn f() {}\n";
        let item = text.find("fn f").unwrap();
        assert_eq!(
            line_before_item(text, item, "import m::{x}"),
            (text.find("/* about").unwrap(), "import m::{x}\n\n".to_string())
        );
        let crlf = text.replace('\n', "\r\n");
        let item = crlf.find("fn f").unwrap();
        assert_eq!(
            line_before_item(&crlf, item, "import m::{x}"),
            (crlf.find("/* about").unwrap(), "import m::{x}\r\n\r\n".to_string())
        );
    }
```

- [ ] **Step 2: Run them to see them fail**

Run: `cd /home/user/nova && cargo test -p nova-diagnostics lines 2>&1 | grep -E "FAILED|panicked|test result" | head`
Expected: both new tests FAIL. The first inserts at byte 16, inside the
comment. The second inserts at `fn f`'s line, below the comment.

- [ ] **Step 3: The rules**

In `lines.rs`, replace `line_after`'s body:

```rust
    let nl = ending(text);
    let next = next_line_start(text, at);
```

with:

```rust
    let nl = ending(text);
    // Past spaces and whole block comments, so a block comment that starts
    // on this line and goes on stays whole (spec 3.5a §3, follow-up 9).
    let bytes = text.as_bytes();
    let mut i = at;
    loop {
        while i < bytes.len() && matches!(bytes[i], b' ' | b'\t') {
            i += 1;
        }
        if !text[i..].starts_with("/*") {
            break;
        }
        i = text[i + 2..].find("*/").map_or(text.len(), |n| i + 2 + n + 2);
    }
    let next = next_line_start(text, i);
```

Replace `line_before_item`'s body:

```rust
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
```

with:

```rust
    let nl = ending(text);
    let mut at = line_start(text, item_start);
    // The comments directly above, each with no blank line after it (spec
    // 3.5a §3, follow-up 9).
    for &(start, end) in comments_before(text, at).iter().rev() {
        if text[end..at].matches('\n').count() > 1 {
            break;
        }
        at = line_start(text, start);
    }
    (at, format!("{line}{nl}{nl}"))
```

and add, after `line_before_item`:

```rust
/// The comments before byte `end` of `text`, which holds only comments
/// and blank space there, by a scan from its start (plan decision 5).
/// Block comments do not nest.
fn comments_before(text: &str, end: usize) -> Vec<(usize, usize)> {
    let bytes = text.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    while i < end {
        match bytes[i] {
            b' ' | b'\t' | b'\r' | b'\n' => i += 1,
            b'/' if bytes.get(i + 1) == Some(&b'/') => {
                let e = text[i..].find('\n').map_or(text.len(), |n| i + n);
                out.push((i, e));
                i = e;
            }
            b'/' if bytes.get(i + 1) == Some(&b'*') => {
                let e = text[i + 2..].find("*/").map_or(text.len(), |n| i + 2 + n + 2);
                out.push((i, e));
                i = e;
            }
            _ => break,
        }
    }
    out
}
```

Delete the private `line_before` and its doc comment: `line_before_item`
was its only caller. `is_comment_line` stays: it is `pub`, and
`nova-fmt`'s organize imports calls it.

- [ ] **Step 4: Run the crate's tests, and the fixes**

Run: `cd /home/user/nova && cargo test -p nova-diagnostics 2>&1 | grep -E "^test result|FAILED" && cargo test -p nova-driver --test fixes 2>&1 | grep -E "^test result|FAILED"`
Expected: all `ok`. `a_line_before_an_item_goes_above_its_comments_and_below_a_header`
still passes: its header is a blank line away.

- [ ] **Step 5: Clippy, and commit**

Run: `cd /home/user/nova && cargo clippy -p nova-diagnostics --all-targets --locked -- -D warnings 2>&1 | tail -3`
Expected: `Finished`.

Message: `fix: the import fix steps over block comments`, body: after an
import whose line opens a block comment, the new line went inside it, and
a block comment above the first item was not walked over (PR #108's
deferred minor 9).

---
### Task 8: The crates, and `Analysis::file`

**Files:**
- Modify: `Cargo.toml` (`[workspace.dependencies]`, after `lsp-types`)
- Modify: `crates/nova-doc/Cargo.toml` (`[dependencies]`, a new
  `[dev-dependencies]`)
- Modify: `crates/nova-cli/Cargo.toml` (`[dependencies]`)
- Modify: `Cargo.lock` (by Cargo)
- Modify: `crates/nova-driver/src/analyze.rs` (`Analysis`, and
  `analyze_program`)
- Test: `crates/nova-driver/tests/analyze.rs`

**Interfaces:**
- Produces: `Analysis::file: Option<nova_ast::File>`.

- [ ] **Step 1: The workspace's new dependency**

In `Cargo.toml`, after the `lsp-types = "0.97"` line, add:

```toml

# `nova doc` renders `///` docs as Markdown (spec
# docs/superpowers/specs/2026-10-11-phase-3-5a-the-documentation-site-design.md
# §9.1; ADR 0034). pulldown-cmark is the CommonMark crate rustdoc and mdBook
# use. 0.13.4 declares Rust 1.71.1, and it is pinned so that `cargo update`
# cannot take a release with a higher minimum. Default features are off:
# they bring getopts, for its command-line tool. `html` brings
# pulldown-cmark-escape 0.11.0 (Rust 1.70). Its other dependencies are
# unicase 2.10.0, which declares no minimum and builds on 1.78 (checked
# 2026-10-11), and bitflags 2 and memchr, already locked.
pulldown-cmark = { version = "=0.13.4", default-features = false, features = ["html"] }
```

- [ ] **Step 2: `nova-doc`'s and `nova-cli`'s manifests**

In `crates/nova-doc/Cargo.toml`, replace the `[dependencies]` table:

```toml
[dependencies]
nova-diagnostics = { path = "../nova-diagnostics" }
nova-ast = { path = "../nova-ast" }
nova-parser = { path = "../nova-parser" }
anyhow = { workspace = true }
```

with:

```toml
[dependencies]
nova-diagnostics = { path = "../nova-diagnostics" }
nova-ast = { path = "../nova-ast" }
nova-lexer = { path = "../nova-lexer" }
nova-resolver = { path = "../nova-resolver" }
nova-typeck = { path = "../nova-typeck" }
nova-driver = { path = "../nova-driver" }
nova-pm = { path = "../nova-pm" }
# `///` docs as Markdown (spec
# docs/superpowers/specs/2026-10-11-phase-3-5a-the-documentation-site-design.md
# §9.1).
pulldown-cmark = { workspace = true }
# The search index's strings, written as JSON (spec §10.1).
serde_json = { workspace = true }
thiserror = { workspace = true }

[dev-dependencies]
# The gate tests and std's completeness check read the sources by the
# parser alone (spec §16.4; plan decision 17).
nova-parser = { path = "../nova-parser" }
nova-std = { path = "../../std" }
```

In `crates/nova-cli/Cargo.toml`, after the `nova-lsp` line:

```toml
# `nova doc` (spec
# docs/superpowers/specs/2026-10-11-phase-3-5a-the-documentation-site-design.md
# §4).
nova-doc = { path = "../nova-doc" }
```

- [ ] **Step 3: Let Cargo lock them, and check what it wrote**

Run: `cd /home/user/nova && cargo build -p nova-doc -p nova-cli 2>&1 | tail -3 && git diff Cargo.lock | grep -E '^[+-]name' `
Expected: `Finished`, and exactly three added names:

```
+name = "pulldown-cmark"
+name = "pulldown-cmark-escape"
+name = "unicase"
```

Run: `cd /home/user/nova && git diff Cargo.lock | grep -E '^\+version' && git diff --stat Cargo.lock`
Expected: `+version = "0.13.4"`, `+version = "0.11.0"`, `+version = "2.10.0"`.
Anything else in the diff besides those packages and the `dependencies`
lists of `nova-doc` and `nova-cli` stops the task: ask the user.

- [ ] **Step 4: The MSRV check, and commit**

Run: `cd /home/user/nova && RUSTFLAGS="-D warnings" cargo +1.78 check --locked --workspace 2>&1 | tail -3`
Expected: `Finished`.

Message: `chore: pulldown-cmark, for nova doc`. The body says that the user
approved the crate on 2026-10-11, names the three locked packages and
their versions, and says that `nova-cli` now depends on `nova-doc`, which
is still a stub.

- [ ] **Step 5: Write the failing test**

Append to `crates/nova-driver/tests/analyze.rs`:

```rust
/// Spec 3.5a §15.2: an analysis keeps the merged AST, std's items
/// included, and `module_of` maps each item to its module.
#[test]
fn an_analysis_keeps_the_merged_ast() {
    let sources = mem(&[(
        "keeps/main.nova",
        "fn main() {}\n\npub fn helper() -> Int { 1 }\n",
    )]);
    let a = analyze(Path::new("keeps/main.nova"), &sources, &Options::default())
        .expect("the entry reads");
    let file = a.file.as_ref().expect("the merged AST");
    let defs = a.definitions.as_ref().expect("the definitions");
    let own: Vec<&str> = file
        .items
        .iter()
        .enumerate()
        .filter(|(i, _)| defs.module_of(*i).0 == 0)
        .filter_map(|(_, item)| match &item.value {
            nova_ast::Item::Function(f) => Some(f.name.value.as_str()),
            _ => None,
        })
        .collect();
    assert_eq!(own, ["main", "helper"]);
    assert!(file.items.len() > own.len(), "std's items are merged in");
}
```

- [ ] **Step 6: Run it to see it fail**

Run: `cd /home/user/nova && cargo test -p nova-driver --test analyze an_analysis_keeps 2>&1 | grep -E "^error" | head -3`
Expected: `error[E0609]: no field \`file\` on type \`Analysis\``.

- [ ] **Step 7: The field**

In `crates/nova-driver/src/analyze.rs`, in `pub struct Analysis`, after
`pub definitions: Option<Definitions>,`:

```rust
    /// The merged AST the resolver built and the checker read: every
    /// module's items, std's last, which [`Definitions::module_of`] maps to
    /// their modules. Set whenever `definitions` is (spec 3.5a §15.2).
    pub file: Option<nova_ast::File>,
```

In `analyze_program`'s `let mut analysis = Analysis { … }`, after
`definitions: None,`, add `file: None,`.

After resolution, replace

```rust
    if stop(&diagnostics) {
        analysis.diagnostics = shown(diagnostics, &registry);
        analysis.definitions = Some(resolved.definitions);
        return Ok(analysis);
    }
```

with

```rust
    if stop(&diagnostics) {
        analysis.diagnostics = shown(diagnostics, &registry);
        analysis.definitions = Some(resolved.definitions);
        analysis.file = Some(resolved.file);
        return Ok(analysis);
    }
```

and at the end, replace

```rust
    if let Some(mut index) = checked.index {
        index.extend(imports);
        analysis.index = Some(index);
    }
    Ok(analysis)
```

with

```rust
    if let Some(mut index) = checked.index {
        index.extend(imports);
        analysis.index = Some(index);
    }
    analysis.file = Some(resolved.file);
    Ok(analysis)
```

- [ ] **Step 8: Run the driver's and the server's tests**

Run: `cd /home/user/nova && cargo test -p nova-driver -p nova-lsp 2>&1 | grep -E "^test result|FAILED" | head -20`
Expected: every line `ok`.

- [ ] **Step 9: Commit**

Message: `feat: an analysis keeps the merged AST`, body: `nova doc` reads
the items' docs, visibility and impl blocks from it, and `Definitions`
maps each to its module (spec 3.5a §15.2).

---

### Task 9: `nova-doc`'s Markdown and Nova code

**Files:**
- Modify: `crates/nova-doc/src/lib.rs` (the stub's one line goes)
- Create: `crates/nova-doc/src/markdown.rs`, `crates/nova-doc/src/highlight.rs`

**Interfaces:**
- Produces (crate-private):
  - `escape(text: &str) -> String` and `span(out: &mut String, class:
    &str, text: &str)` in `lib.rs`;
  - `highlight::class(token: &Token) -> Option<&'static str>`,
    `highlight::highlight(code: &str) -> String`;
  - `markdown::Docs { html, summary }` (`Debug, Clone, Default,
    PartialEq, Eq`), `markdown::join(lines: &[Spanned<String>]) ->
    String`, `markdown::render(text: &str, level: u8) -> Docs`.

- [ ] **Step 1: The crate root**

Replace `crates/nova-doc/src/lib.rs` with:

```rust
//! `nova doc`: the documentation site (spec
//! `docs/superpowers/specs/2026-10-11-phase-3-5a-the-documentation-site-design.md`;
//! ADR 0034).

// Until Task 13 writes a site, most of the crate is reached only by its
// tests. Task 13 removes this line.
#![allow(dead_code)]

mod highlight;
mod markdown;

/// `text` with `&`, `<`, `>`, `"` and `'` written as HTML entities.
pub(crate) fn escape(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            c => out.push(c),
        }
    }
    out
}

/// `text`, escaped, in a `<span>` of `class`.
pub(crate) fn span(out: &mut String, class: &str, text: &str) {
    out.push_str("<span class=\"");
    out.push_str(class);
    out.push_str("\">");
    out.push_str(&escape(text));
    out.push_str("</span>");
}
```

- [ ] **Step 2: Write the failing tests**

Create `crates/nova-doc/src/highlight.rs` with only its tests:

```rust
//! Nova code, coloured by the lexer (spec §9.4).

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keywords_literals_and_comments_are_coloured() {
        assert_eq!(
            highlight("fn f() -> Int { 1 } // one"),
            "<span class=\"kw\">fn</span> f() -&gt; Int { <span class=\"lit\">1</span> } \
             <span class=\"cm\">// one</span>"
        );
    }

    #[test]
    fn a_string_and_a_doc_comment() {
        assert_eq!(
            highlight("/// Doc.\nlet s = \"a <b>\""),
            "<span class=\"cm\">/// Doc.</span>\n<span class=\"kw\">let</span> s = \
             <span class=\"lit\">&quot;</span><span class=\"lit\">a &lt;b&gt;</span>\
             <span class=\"lit\">&quot;</span>"
        );
    }

    #[test]
    fn code_the_lexer_rejects_is_plain() {
        assert_eq!(highlight("let s = \"open"), "let s = &quot;open");
    }
}
```

Create `crates/nova-doc/src/markdown.rs` with only its tests:

```rust
//! Docs as Markdown (spec §9.1, §9.3; plan decision 12).

#[cfg(test)]
mod tests {
    use super::*;
    use nova_diagnostics::{FileId, Span};

    #[test]
    fn a_paragraph_emphasis_a_list_and_a_table() {
        let d = render(
            "Some *text*.\n\n- one\n- two\n\n| a | b |\n|---|---|\n| 1 | 2 |",
            3,
        );
        assert_eq!(
            d.html,
            "<p>Some <em>text</em>.</p>\n<ul>\n<li>one</li>\n<li>two</li>\n</ul>\n\
             <table><thead><tr><th>a</th><th>b</th></tr></thead><tbody>\n\
             <tr><td>1</td><td>2</td></tr>\n</tbody></table>\n"
        );
        assert_eq!(d.summary, "Some text.");
    }

    #[test]
    fn raw_html_shows_as_text() {
        // Review Focus 2.
        let d = render("Click <b>here</b>.\n\n<script>alert(1)</script>\n", 3);
        assert!(!d.html.contains("<script>"), "{}", d.html);
        assert!(!d.html.contains("<b>"), "{}", d.html);
        assert!(
            d.html.contains("&lt;script&gt;alert(1)&lt;/script&gt;"),
            "{}",
            d.html
        );
        assert!(d.html.contains("Click &lt;b&gt;here&lt;/b&gt;."), "{}", d.html);
    }

    #[test]
    fn headings_move_down_below_their_items() {
        let d = render("# Errors\n\n###### Deep", 3);
        assert_eq!(d.html, "<h4>Errors</h4>\n<h6>Deep</h6>\n");
    }

    #[test]
    fn nova_code_is_highlighted_and_other_code_is_not() {
        let d = render(
            "```\nfn f() {}\n```\n\n```text\nfn f() {}\n```\n\n    let x = 1\n",
            3,
        );
        assert_eq!(
            d.html,
            "<pre class=\"code\"><code><span class=\"kw\">fn</span> f() {}\n</code></pre>\n\
             <pre class=\"code\"><code>fn f() {}\n</code></pre>\n\
             <pre class=\"code\"><code><span class=\"kw\">let</span> x = \
             <span class=\"lit\">1</span>\n</code></pre>\n"
        );
    }

    #[test]
    fn the_summary_is_the_first_paragraph_as_plain_text() {
        let d = render("Reads `n` bytes,\nor fewer.\n\nThen more.", 3);
        assert_eq!(d.summary, "Reads n bytes, or fewer.");
    }

    #[test]
    fn empty_docs_render_nothing() {
        assert_eq!(render("", 3), Docs::default());
        assert_eq!(render("  \n", 3), Docs::default());
    }

    #[test]
    fn lines_join_without_one_leading_space() {
        let line = |s: &str| Spanned::new(s.to_string(), Span::point(0, FileId::DUMMY));
        assert_eq!(
            join(&[line(" One."), line(""), line("  Two.")]),
            "One.\n\n Two."
        );
    }
}
```

Add `mod`-level `use` lines later, with the code. Run: `cd /home/user/nova && cargo test -p nova-doc 2>&1 | grep -E "^error" | sort | uniq -c | head`
Expected: E0425 errors naming `highlight`, `render` and `join`.

- [ ] **Step 3: The highlighter**

In `crates/nova-doc/src/highlight.rs`, between the module doc comment and
`#[cfg(test)]`, insert:

```rust

use nova_diagnostics::FileId;
use nova_lexer::Token;

use crate::{escape, span};

/// The class a token is coloured with: `kw` for a keyword, `lit` for a
/// literal, `cm` for a doc comment, or none.
pub(crate) fn class(token: &Token) -> Option<&'static str> {
    match token {
        Token::Let
        | Token::Mut
        | Token::Const
        | Token::Fn
        | Token::Return
        | Token::If
        | Token::Else
        | Token::While
        | Token::For
        | Token::In
        | Token::Break
        | Token::Continue
        | Token::Match
        | Token::Type
        | Token::Record
        | Token::Trait
        | Token::Impl
        | Token::Import
        | Token::Module
        | Token::Pub
        | Token::Async
        | Token::Await
        | Token::Extern
        | Token::Unsafe
        | Token::True
        | Token::False
        | Token::As
        | Token::Is
        | Token::Where
        | Token::With
        | Token::SelfLower
        | Token::SelfUpper => Some("kw"),
        Token::Int(_)
        | Token::Float(_)
        | Token::Char(_)
        | Token::StrStart
        | Token::StrPart(_)
        | Token::StrEnd
        | Token::InterpOpen
        | Token::InterpClose
        | Token::RawStr(_) => Some("lit"),
        Token::DocComment(_) => Some("cm"),
        _ => None,
    }
}

/// `code` as HTML, coloured (spec §9.4). Code the lexer rejects is shown
/// plain.
pub(crate) fn highlight(code: &str) -> String {
    let (tokens, comments, errors) = nova_lexer::lex_with_comments(code, FileId::DUMMY);
    if !errors.is_empty() {
        return escape(code);
    }
    let mut marks: Vec<(usize, usize, Option<&str>)> = tokens
        .iter()
        .filter(|t| t.value != Token::Eof)
        .map(|t| (t.span.start as usize, t.span.end as usize, class(&t.value)))
        .collect();
    marks.extend(
        comments
            .iter()
            .map(|c| (c.span.start as usize, c.span.end as usize, Some("cm"))),
    );
    marks.sort_by_key(|m| m.0);
    let mut out = String::new();
    let mut at = 0;
    for (start, end, class) in marks {
        if start < at || end > code.len() {
            continue;
        }
        out.push_str(&escape(&code[at..start]));
        match class {
            Some(c) => span(&mut out, c, &code[start..end]),
            None => out.push_str(&escape(&code[start..end])),
        }
        at = end;
    }
    out.push_str(&escape(&code[at..]));
    out
}
```

- [ ] **Step 4: Markdown**

In `crates/nova-doc/src/markdown.rs`, between the module doc comment and
`#[cfg(test)]`, insert:

```rust

use nova_diagnostics::Spanned;
use pulldown_cmark::{html, CodeBlockKind, Event, HeadingLevel, Options, Parser, Tag, TagEnd};

use crate::escape;
use crate::highlight::highlight;

/// Docs, rendered: their HTML, and their first paragraph as plain text,
/// which lists and search results show (spec §9.1).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct Docs {
    pub html: String,
    pub summary: String,
}

/// Doc lines as the AST keeps them, each the text after its `///`, as one
/// Markdown text: each line loses one leading space.
pub(crate) fn join(lines: &[Spanned<String>]) -> String {
    lines
        .iter()
        .map(|l| l.value.strip_prefix(' ').unwrap_or(&l.value))
        .collect::<Vec<_>>()
        .join("\n")
}

/// `text` rendered (spec §9.1): raw HTML as text, headings moved down
/// `level` levels, and Nova code blocks coloured (§9.3).
pub(crate) fn render(text: &str, level: u8) -> Docs {
    if text.trim().is_empty() {
        return Docs::default();
    }
    let mut options = Options::empty();
    options.insert(Options::ENABLE_TABLES);
    options.insert(Options::ENABLE_STRIKETHROUGH);
    let mut events: Vec<Event> = Vec::new();
    // A code block being read: whether it is Nova, and its text so far.
    let mut code: Option<(bool, String)> = None;
    for event in Parser::new_ext(text, options) {
        if code.is_some() {
            match event {
                Event::Text(t) => {
                    if let Some((_, buf)) = code.as_mut() {
                        buf.push_str(&t);
                    }
                }
                Event::End(TagEnd::CodeBlock) => {
                    if let Some((nova, buf)) = code.take() {
                        let body = if nova { highlight(&buf) } else { escape(&buf) };
                        events.push(Event::Html(
                            format!("<pre class=\"code\"><code>{body}</code></pre>\n").into(),
                        ));
                    }
                }
                _ => {}
            }
            continue;
        }
        match event {
            Event::Start(Tag::CodeBlock(kind)) => {
                let nova = match kind {
                    CodeBlockKind::Indented => true,
                    CodeBlockKind::Fenced(info) => {
                        let lang = info
                            .split(|c: char| c == ',' || c.is_whitespace())
                            .next()
                            .unwrap_or("");
                        lang.is_empty() || lang == "nova"
                    }
                };
                code = Some((nova, String::new()));
            }
            Event::Html(h) | Event::InlineHtml(h) => events.push(Event::Text(h)),
            Event::Start(Tag::Heading {
                level: l,
                id,
                classes,
                attrs,
            }) => events.push(Event::Start(Tag::Heading {
                level: shift(l, level),
                id,
                classes,
                attrs,
            })),
            Event::End(TagEnd::Heading(l)) => {
                events.push(Event::End(TagEnd::Heading(shift(l, level))))
            }
            other => events.push(other),
        }
    }
    let summary = summary(&events);
    let mut html = String::new();
    html::push_html(&mut html, events.into_iter());
    Docs { html, summary }
}

/// `level` moved down `by` levels, stopping at `<h6>` (plan decision 12).
fn shift(level: HeadingLevel, by: u8) -> HeadingLevel {
    let n = by
        + match level {
            HeadingLevel::H1 => 1,
            HeadingLevel::H2 => 2,
            HeadingLevel::H3 => 3,
            HeadingLevel::H4 => 4,
            HeadingLevel::H5 => 5,
            HeadingLevel::H6 => 6,
        };
    match n {
        1 => HeadingLevel::H1,
        2 => HeadingLevel::H2,
        3 => HeadingLevel::H3,
        4 => HeadingLevel::H4,
        5 => HeadingLevel::H5,
        _ => HeadingLevel::H6,
    }
}

/// The first paragraph's text, without its markup.
fn summary(events: &[Event]) -> String {
    let mut out = String::new();
    let mut inside = false;
    for e in events {
        match e {
            Event::Start(Tag::Paragraph) => inside = true,
            Event::End(TagEnd::Paragraph) if inside => break,
            Event::Text(t) | Event::Code(t) if inside => out.push_str(t),
            Event::SoftBreak | Event::HardBreak if inside => out.push(' '),
            _ => {}
        }
    }
    out
}
```

- [ ] **Step 5: Run the tests**

Run: `cd /home/user/nova && cargo test -p nova-doc 2>&1 | grep -E "^test result|FAILED|panicked"`
Expected: `test result: ok. 10 passed`. If `pulldown-cmark`'s exact
whitespace differs from an expected string (a newline after `</table>`,
say), the code is right and the value is wrong: correct the value and
ledger it.

- [ ] **Step 6: Clippy, and commit**

Run: `cd /home/user/nova && cargo clippy -p nova-doc --all-targets --locked -- -D warnings 2>&1 | tail -3`
Expected: `Finished`.

Message: `feat(nova-doc): Markdown docs and coloured Nova code`, body: raw
HTML shows as text, headings move below their items, and Nova code blocks
are coloured by the lexer (spec 3.5a §9).

---

### Task 10: Signatures as written

**Files:**
- Create: `crates/nova-doc/src/signature.rs`
- Modify: `crates/nova-doc/src/lib.rs` (`mod signature;`)

**Interfaces:**
- Consumes: `highlight::class`, `escape`, `span`;
  `nova_diagnostics::lines::{keyword_start, line_start}`.
- Produces (crate-private):
  - `FileText<'a> { text, tokens, comments }` with `new(text, file)`,
    `next(from, want) -> Option<usize>`, `group_end(from) ->
    Option<usize>`, `module_docs() -> String`;
  - `decl_start(text, name_start) -> usize`;
  - `fn_end(ft, name: Span, ret: Option<&Spanned<Type>>, wheres:
    &[WhereBound]) -> usize`;
  - `render(ft, start, end, own: Option<Span>, link: &dyn Fn(Span) ->
    Option<String>) -> String`;
  - `gap(between, base) -> String`.

- [ ] **Step 1: Write the failing tests**

Add `mod signature;` after `mod markdown;` in `lib.rs`, and create
`crates/nova-doc/src/signature.rs` with only its tests:

```rust
//! Signatures as written (spec §8; plan decisions 10 and 11).

#[cfg(test)]
mod tests {
    use super::*;
    use nova_ast::item::TypeDef;
    use nova_ast::Item;

    fn parse(text: &str) -> nova_ast::File {
        let (tokens, errors) = nova_lexer::lex(text, FileId::DUMMY);
        assert!(errors.is_empty(), "{errors:?}");
        let (file, errors) = nova_parser::parse(&tokens, FileId::DUMMY);
        assert!(errors.is_empty(), "{errors:?}");
        file.expect("the file parses")
    }

    #[test]
    fn the_gap_between_two_tokens() {
        assert_eq!(gap("", 0), "");
        assert_eq!(gap("   ", 0), " ");
        assert_eq!(gap(" /* c */ ", 0), " ");
        assert_eq!(gap("/*c*/", 0), " ");
        assert_eq!(gap(" // why\n        ", 4), "\n    ");
        assert_eq!(gap("\r\n\r\n    ", 4), "\n");
    }

    #[test]
    fn a_function_stops_before_its_body_and_keeps_pub_async_and_where() {
        let text = "/// Doc.\npub async fn go<T>(x: T) -> T where T: Show { x }\n";
        let file = parse(text);
        let Item::Function(f) = &file.items[0].value else {
            panic!("not a function")
        };
        let ft = FileText::new(text, FileId::DUMMY);
        let start = decl_start(text, f.name.span.start as usize);
        let end = fn_end(&ft, f.name.span, f.return_ty.as_ref(), &f.where_clause);
        assert_eq!(
            render(&ft, start, end, Some(f.name.span), &|_| None),
            "<span class=\"kw\">pub</span> <span class=\"kw\">async</span> \
             <span class=\"kw\">fn</span> <span class=\"name\">go</span>&lt;T&gt;(x: T) -&gt; T \
             <span class=\"kw\">where</span> T: Show"
        );
    }

    #[test]
    fn a_broken_signature_keeps_its_lines_less_their_indentation() {
        // Review Focus 1.
        let text = "impl P {\n    pub fn far(\n        self,\n        // the distance\n        to: Int,\n    ) -> Int {\n        to\n    }\n}\n";
        let file = parse(text);
        let Item::Impl(imp) = &file.items[0].value else {
            panic!("not an impl")
        };
        let f = &imp.functions[0];
        let ft = FileText::new(text, FileId::DUMMY);
        let start = decl_start(text, f.name.span.start as usize);
        let end = fn_end(&ft, f.name.span, f.return_ty.as_ref(), &f.where_clause);
        assert_eq!(
            render(&ft, start, end, Some(f.name.span), &|_| None),
            "<span class=\"kw\">pub</span> <span class=\"kw\">fn</span> \
             <span class=\"name\">far</span>(\n    <span class=\"kw\">self</span>,\n    \
             to: Int,\n) -&gt; Int"
        );
    }

    #[test]
    fn names_link_and_the_items_own_name_does_not() {
        let text = "pub fn area(r: Rect, n: Int) -> Float { 0.0 }\n";
        let ft = FileText::new(text, FileId::DUMMY);
        let at = text.find("Rect").unwrap() as u32;
        let rect = Span::new(at, at + 4, FileId::DUMMY);
        let own = Span::new(7, 11, FileId::DUMMY);
        let end = text.find(" {").unwrap();
        let link = |s: Span| (s == rect).then(|| "../g/index.html#record.Rect".to_string());
        assert_eq!(
            render(&ft, 0, end, Some(own), &link),
            "<span class=\"kw\">pub</span> <span class=\"kw\">fn</span> \
             <span class=\"name\">area</span>(r: <a href=\"../g/index.html#record.Rect\">Rect</a>, \
             n: Int) -&gt; Float"
        );
    }

    #[test]
    fn a_record_a_sum_type_a_variant_a_trait_and_a_constant_end_where_they_should() {
        let text = "pub record P<T> { x: T }\npub type S = | A | B(Int)\npub trait Show: Eq { fn show(self) -> String }\npub const N: Int = 1 + 2\n";
        let file = parse(text);
        let ft = FileText::new(text, FileId::DUMMY);
        let plain = |start: usize, end: usize| render(&ft, start, end, None, &|_| None);
        let brace = |from: u32| ft.next(from as usize, |t| *t == Token::LBrace).unwrap();
        let Item::Record(r) = &file.items[0].value else { panic!() };
        assert_eq!(
            plain(decl_start(text, r.name.span.start as usize), brace(r.name.span.end)),
            "<span class=\"kw\">pub</span> <span class=\"kw\">record</span> P&lt;T&gt;"
        );
        let Item::Type(t) = &file.items[1].value else { panic!() };
        let eq = ft.next(t.name.span.end as usize, |t| *t == Token::Eq).unwrap();
        assert_eq!(
            plain(decl_start(text, t.name.span.start as usize), eq),
            "<span class=\"kw\">pub</span> <span class=\"kw\">type</span> S"
        );
        let TypeDef::Sum(variants) = &t.def else { panic!() };
        let b = &variants[1];
        let end = ft.group_end(b.name.span.end as usize).unwrap();
        assert_eq!(plain(b.name.span.start as usize, end), "B(Int)");
        let Item::Trait(tr) = &file.items[2].value else { panic!() };
        assert_eq!(
            plain(decl_start(text, tr.name.span.start as usize), brace(tr.name.span.end)),
            "<span class=\"kw\">pub</span> <span class=\"kw\">trait</span> Show: Eq"
        );
        let Item::Const(c) = &file.items[3].value else { panic!() };
        assert_eq!(
            plain(decl_start(text, c.name.span.start as usize), c.value.span.end as usize),
            "<span class=\"kw\">pub</span> <span class=\"kw\">const</span> N: Int = \
             <span class=\"lit\">1</span> + <span class=\"lit\">2</span>"
        );
    }

    #[test]
    fn a_field_starts_at_its_pub() {
        let text = "record P {\n    pub x: Int,\n    y: Int,\n}\n";
        assert_eq!(
            decl_start(text, text.find("x:").unwrap()),
            text.find("pub x").unwrap()
        );
        assert_eq!(decl_start(text, text.find("y:").unwrap()), text.find("y:").unwrap());
    }

    #[test]
    fn module_docs_are_the_bang_lines_before_the_first_token() {
        let ft = FileText::new(
            "// A note.\n//! The core.\n//!\n//! More.\nfn f() {}\n//! Not docs.\n",
            FileId::DUMMY,
        );
        assert_eq!(ft.module_docs(), "The core.\n\nMore.");
        assert_eq!(FileText::new("fn f() {}\n", FileId::DUMMY).module_docs(), "");
        assert_eq!(
            FileText::new("//! Only docs.\n", FileId::DUMMY).module_docs(),
            "Only docs."
        );
    }
}
```

Run: `cd /home/user/nova && cargo test -p nova-doc 2>&1 | grep -E "^error" | sort | uniq -c | head`
Expected: E0425 and E0433 errors naming `gap`, `FileText`, `decl_start`,
`fn_end`, `render`, `Span` and `Token`.

- [ ] **Step 2: The code**

In `crates/nova-doc/src/signature.rs`, between the module doc comment and
`#[cfg(test)]`, insert:

```rust

use nova_ast::item::WhereBound;
use nova_ast::Type;
use nova_diagnostics::{lines, FileId, Span, Spanned};
use nova_lexer::{Comment, Token};

use crate::highlight::class;
use crate::{escape, span};

/// One file's text, with its tokens and its comments, lexed once.
pub(crate) struct FileText<'a> {
    pub text: &'a str,
    pub tokens: Vec<Spanned<Token>>,
    pub comments: Vec<Comment>,
}

impl<'a> FileText<'a> {
    pub fn new(text: &'a str, file: FileId) -> FileText<'a> {
        let (tokens, comments, _) = nova_lexer::lex_with_comments(text, file);
        FileText {
            text,
            tokens,
            comments,
        }
    }

    /// Where the first token `want` accepts starts, at or after byte
    /// `from`.
    pub fn next(&self, from: usize, want: fn(&Token) -> bool) -> Option<usize> {
        self.tokens
            .iter()
            .find(|t| t.span.start as usize >= from && want(&t.value))
            .map(|t| t.span.start as usize)
    }

    /// Where the `(…)` that opens at the first `(` at or after byte `from`
    /// ends.
    pub fn group_end(&self, from: usize) -> Option<usize> {
        let mut depth = 0usize;
        for t in self.tokens.iter().filter(|t| t.span.start as usize >= from) {
            match t.value {
                Token::LParen => depth += 1,
                Token::RParen if depth == 1 => return Some(t.span.end as usize),
                Token::RParen if depth > 1 => depth -= 1,
                _ => {}
            }
        }
        None
    }

    /// The module's docs (spec §9.2): its `//!` lines before its first
    /// token, each without `//!` and one space.
    pub fn module_docs(&self) -> String {
        let first = self
            .tokens
            .iter()
            .find(|t| t.value != Token::Eof)
            .map_or(self.text.len(), |t| t.span.start as usize);
        self.comments
            .iter()
            .filter(|c| c.span.end as usize <= first)
            .filter_map(|c| self.text.get(c.span.start as usize..c.span.end as usize))
            .filter_map(|c| c.strip_prefix("//!"))
            .map(|l| l.strip_prefix(' ').unwrap_or(l))
            .collect::<Vec<_>>()
            .join("\n")
    }
}

/// Where a declaration's text starts (spec §8): its keyword, with an
/// `async` and a `pub` before it; or, with no keyword, as for a field or a
/// variant, its name, with a `pub` before it.
pub(crate) fn decl_start(text: &str, name_start: usize) -> usize {
    let at = lines::keyword_start(text, name_start).unwrap_or(name_start);
    let before = text[..at].trim_end_matches([' ', '\t']);
    match before.strip_suffix("pub") {
        Some(rest) if !rest.ends_with(|c: char| c.is_ascii_alphanumeric() || c == '_') => {
            rest.len()
        }
        _ => at,
    }
}

/// Where a function's signature ends (plan decision 10): the `)` closing
/// its parameters, its return type, or its last `where` bound, whichever
/// ends last.
pub(crate) fn fn_end(
    ft: &FileText,
    name: Span,
    ret: Option<&Spanned<Type>>,
    wheres: &[WhereBound],
) -> usize {
    let mut end = ft
        .group_end(name.end as usize)
        .unwrap_or(name.end as usize);
    if let Some(r) = ret {
        end = end.max(r.span.end as usize);
    }
    for w in wheres {
        end = end.max(w.ty.span.end as usize);
        for b in &w.bounds {
            end = end.max(b.span.end as usize);
        }
    }
    end
}

/// The HTML of the declaration `ft.text[start..end]` (spec §8): comments
/// dropped, its lines less the indentation of the line it starts on,
/// keywords and literals coloured, `own`, its name, marked, and each other
/// name `link` gives a URL to made a link.
pub(crate) fn render(
    ft: &FileText,
    start: usize,
    end: usize,
    own: Option<Span>,
    link: &dyn Fn(Span) -> Option<String>,
) -> String {
    let text = ft.text;
    let line = lines::line_start(text, start);
    let base = text[line..].len() - text[line..].trim_start_matches([' ', '\t']).len();
    let mut out = String::new();
    let mut last: Option<usize> = None;
    for t in &ft.tokens {
        let (s, e) = (t.span.start as usize, t.span.end as usize);
        if s < start || e > end || matches!(t.value, Token::Eof | Token::DocComment(_)) {
            continue;
        }
        if let Some(prev) = last {
            out.push_str(&gap(&text[prev..s], base));
        }
        let word = &text[s..e];
        match &t.value {
            Token::Ident(_) if own == Some(t.span) => span(&mut out, "name", word),
            Token::Ident(_) => match link(t.span) {
                Some(url) => {
                    out.push_str("<a href=\"");
                    out.push_str(&escape(&url));
                    out.push_str("\">");
                    out.push_str(&escape(word));
                    out.push_str("</a>");
                }
                None => out.push_str(&escape(word)),
            },
            other => match class(other) {
                Some(c) => span(&mut out, c, word),
                None => out.push_str(&escape(word)),
            },
        }
        last = Some(e);
    }
    out
}

/// What lies between two tokens, which is only spaces and comments, as it
/// shows (plan decision 11): nothing, one space, or a line break and the
/// next line's indentation less `base` characters.
pub(crate) fn gap(between: &str, base: usize) -> String {
    let mut plain = String::new();
    let mut rest = between;
    while let Some(c) = rest.chars().next() {
        if let Some(after) = rest.strip_prefix("//") {
            rest = &after[after.find('\n').unwrap_or(after.len())..];
        } else if let Some(after) = rest.strip_prefix("/*") {
            rest = &after[after.find("*/").map_or(after.len(), |i| i + 2)..];
            plain.push(' ');
        } else {
            plain.push(c);
            rest = &rest[c.len_utf8()..];
        }
    }
    match plain.rfind('\n') {
        None if plain.is_empty() => String::new(),
        None => " ".to_string(),
        Some(i) => {
            let indent = &plain[i + 1..];
            let drop = indent
                .chars()
                .take(base)
                .take_while(|c| matches!(c, ' ' | '\t'))
                .count();
            format!("\n{}", &indent[drop..])
        }
    }
}
```

- [ ] **Step 3: Run the tests, clippy, and commit**

Run: `cd /home/user/nova && cargo test -p nova-doc 2>&1 | grep -E "^test result|FAILED|panicked" && cargo clippy -p nova-doc --all-targets --locked -- -D warnings 2>&1 | tail -3`
Expected: `test result: ok. 17 passed`, and `Finished`.

Message: `feat(nova-doc): signatures as written`, body: a declaration's own
text, comments dropped, dedented, keywords and literals coloured, and each
name its target documents linked (spec 3.5a §8).

---
### Task 11: The model, from analyses to a site

**Files:**
- Create: `crates/nova-doc/src/model.rs`
- Modify: `crates/nova-doc/src/lib.rs` (`mod model;` and the public
  re-exports)

**Interfaces:**
- Consumes: `Analysis::file` (Task 8); `markdown` (Task 9); `signature`
  (Task 10); `nova_driver::{analyze_program, Program, Roots, Options,
  DiskSources, Sources}`; `nova_resolver::{Definitions, DefKind, Target,
  Role, Builtin, index::rank}`; `nova_typeck::builtin_text`.
- Produces:
  - `pub fn document_package(root: &Path) -> Result<Documented, Failure>`;
  - `pub fn document_std() -> Result<Documented, Failure>`;
  - `pub struct Documented { pub site: Site, pub db: FileDb, pub
    warnings: Vec<Diagnostic> }`;
  - `pub enum Failure { Diagnostics { db: FileDb, diagnostics:
    Vec<Diagnostic> }, Other(String) }`;
  - `pub struct Site` with crate-private `packages` and `start`;
  - crate-private `Package`, `Module`, `Item`, `ItemKind`, `Member`,
    `MemberKind`, `Impl`, `Primitive`, `BuiltinDoc`, and, under
    `#[cfg(test)]`, `sample_site() -> Site` for Tasks 12 and 13.

- [ ] **Step 1: Write the failing tests**

Add `mod model;` after `mod markdown;` in `lib.rs`, and after the `mod`
lines:

```rust
pub use model::{document_package, document_std, Documented, Failure, Site};
```

Create `crates/nova-doc/src/model.rs` with its doc comment and tests:

```rust
//! From analyses to a site (spec §5 to §7; plan decisions 7 to 9, 13 and
//! 19).

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    /// A fresh, empty directory with a fixed name, so each run replaces the
    /// last.
    fn fresh(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("nova-doc-model-{name}"));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn write(dir: &Path, files: &[(&str, &str)]) {
        for (path, text) in files {
            let path = dir.join(path);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(path, text).unwrap();
        }
    }

    fn manifest(name: &str, extra: &str) -> String {
        format!("[package]\nname = \"{name}\"\nversion = \"0.1.0\"\nedition = \"2026\"\n{extra}")
    }

    fn documented(root: &Path) -> Documented {
        match document_package(root) {
            Ok(d) => d,
            Err(Failure::Diagnostics { db, diagnostics }) => panic!(
                "{}",
                nova_diagnostics::render::render_to_string(&db, &diagnostics)
            ),
            Err(Failure::Other(why)) => panic!("{why}"),
        }
    }

    fn module<'s>(site: &'s Site, page: &str) -> &'s Module {
        site.packages
            .iter()
            .flat_map(|p| &p.modules)
            .find(|m| m.page == page)
            .unwrap_or_else(|| panic!("no page {page}"))
    }

    fn item<'m>(m: &'m Module, anchor: &str) -> &'m Item {
        m.items
            .iter()
            .find(|i| i.anchor == anchor)
            .unwrap_or_else(|| panic!("no {anchor} on {}", m.page))
    }

    const GEOM_LIB: &str = "import utils\n\n/// A point.\npub record Point {\n    /// Across.\n    pub x: Int,\n    y: Int,\n}\n\n/// The area of a square of side `width()`.\npub fn area() -> Int { width() * width() }\n\nfn secret() -> Int { 1 }\n\nrecord Hidden {\n    n: Int,\n}\n\n/// Makes a thing of a private type.\npub fn make() -> Hidden { Hidden { n: 1 } }\n";
    const GEOM_UTILS: &str = "/// Three.\npub fn width() -> Int { 3 }\n";
    const APP_MAIN: &str = "import geom\n\n/// Shows `p`.\nfn show(p: Point) -> String { \"${p.x}\" }\n\n/// A record of the app's own.\nrecord Local {\n    hidden: Int,\n}\n\nfn main() {\n    println(show(Point { x: area(), y: 0 }))\n}\n";

    #[test]
    fn std_is_one_package_of_modules_primitives_and_builtins() {
        let Ok(d) = document_std() else {
            panic!("std is not documented")
        };
        assert_eq!(d.site.packages.len(), 1);
        assert_eq!(d.site.start, "std/index.html");
        let std = &d.site.packages[0];
        assert!(std.std);
        let names: Vec<&str> = std.modules.iter().map(|m| m.name.as_str()).collect();
        assert_eq!(
            names,
            [
                "bytes",
                "collections",
                "core",
                "crypto",
                "fmt",
                "fs",
                "http",
                "io",
                "json",
                "log",
                "net",
                "process",
                "strings",
                "sync",
                "task",
                "test",
                "time"
            ]
        );
        let core = module(&d.site, "std/core/index.html");
        let option = item(core, "type.Option");
        let variants: Vec<&str> = option.members.iter().map(|m| m.anchor.as_str()).collect();
        assert_eq!(variants, ["type.Option.variant.Some", "type.Option.variant.None"]);
        assert!(option
            .impls
            .iter()
            .any(|i| i.methods.iter().any(|m| m.anchor == "type.Option.method.unwrap")));
        let string = std
            .primitives
            .iter()
            .find(|p| p.name == "String")
            .expect("a String page");
        assert!(string.impls.iter().any(|i| !i.methods.is_empty()));
        let mut builtins: Vec<&str> = std.builtins.iter().map(|b| b.name).collect();
        builtins.sort_unstable();
        assert_eq!(builtins, ["eprint", "eprintln", "panic", "print", "println"]);
    }

    #[test]
    fn a_program_documents_every_item_and_its_library_only_its_exports() {
        let dir = fresh("program");
        write(
            &dir.join("geom"),
            &[
                ("nova.toml", &manifest("geom", "")),
                ("src/lib.nova", GEOM_LIB),
                ("src/utils.nova", GEOM_UTILS),
            ],
        );
        write(
            &dir.join("app"),
            &[
                (
                    "nova.toml",
                    &manifest("app", "\n[dependencies]\ngeom = { path = \"../geom\" }\n"),
                ),
                ("src/main.nova", APP_MAIN),
            ],
        );
        let d = documented(&dir.join("app"));
        let names: Vec<&str> = d.site.packages.iter().map(|p| p.name.as_str()).collect();
        assert_eq!(names, ["app", "geom", "std"]);
        assert_eq!(d.site.start, "app/index.html");
        let app = module(&d.site, "app/index.html");
        assert!(app.root);
        let anchors: Vec<&str> = app.items.iter().map(|i| i.anchor.as_str()).collect();
        assert_eq!(anchors, ["fn.show", "record.Local", "fn.main"]);
        assert!(!item(app, "record.Local").hidden_fields);
        let show = item(app, "fn.show");
        assert!(
            show.signature
                .contains("<a href=\"../geom/index.html#record.Point\">Point</a>"),
            "{}",
            show.signature
        );
        assert!(
            show.signature
                .contains("<a href=\"../std/primitive.String.html\">String</a>"),
            "{}",
            show.signature
        );
        let geom = module(&d.site, "geom/index.html");
        let anchors: Vec<&str> = geom.items.iter().map(|i| i.anchor.as_str()).collect();
        assert_eq!(anchors, ["record.Point", "fn.area", "fn.make"]);
        // A private type in a public signature is plain text (mutant 1).
        let make = item(geom, "fn.make");
        assert!(!make.signature.contains("<a "), "{}", make.signature);
        let point = item(geom, "record.Point");
        assert!(point.hidden_fields);
        let fields: Vec<&str> = point.members.iter().map(|m| m.anchor.as_str()).collect();
        assert_eq!(fields, ["record.Point.field.x"]);
        module(&d.site, "geom/utils/index.html");
    }

    const IMPLS: &str = "/// A rectangle.\npub record Rect {\n    pub w: Int,\n    pub h: Int,\n}\n\nimpl Rect {\n    /// Its area.\n    pub fn area(self) -> Int { self.w * self.h }\n\n    fn hidden(self) -> Int { 0 }\n}\n\n/// What can be measured.\npub trait Measure {\n    /// The size.\n    fn size(self) -> Int\n}\n\nimpl Measure for Rect {\n    fn size(self) -> Int { self.area() }\n}\n\nimpl Int {\n    /// Twice it.\n    pub fn doubled(self) -> Int { self * 2 }\n}\n";

    #[test]
    fn an_impl_goes_with_its_type_and_a_trait_lists_its_impls() {
        let dir = fresh("impls");
        write(
            &dir,
            &[("nova.toml", &manifest("shapes", "")), ("src/lib.nova", IMPLS)],
        );
        let d = documented(&dir);
        let lib = module(&d.site, "shapes/index.html");
        let rect = item(lib, "record.Rect");
        assert_eq!(rect.impls.len(), 2, "the inherent impl and the trait impl");
        let methods: Vec<&str> = rect
            .impls
            .iter()
            .flat_map(|i| &i.methods)
            .map(|m| m.anchor.as_str())
            .collect();
        assert_eq!(methods, ["record.Rect.method.area"]);
        let measure = item(lib, "trait.Measure");
        assert_eq!(measure.impls.len(), 1);
        assert!(
            measure.impls[0]
                .header
                .contains("<a href=\"../shapes/index.html#record.Rect\">Rect</a>"),
            "{}",
            measure.impls[0].header
        );
        assert_eq!(lib.other_impls.len(), 1, "the impl on Int");
        let doubled: Vec<&str> = lib.other_impls[0]
            .methods
            .iter()
            .map(|m| m.anchor.as_str())
            .collect();
        assert_eq!(doubled, ["impl.Int.method.doubled"]);
    }

    #[test]
    fn an_error_is_a_failure_that_carries_its_diagnostics() {
        let dir = fresh("error");
        write(
            &dir,
            &[
                ("nova.toml", &manifest("app", "")),
                ("src/main.nova", "fn main() {\n    let x: Int = \"s\"\n}\n"),
            ],
        );
        match document_package(&dir) {
            Err(Failure::Diagnostics { diagnostics, .. }) => {
                assert!(diagnostics.iter().any(|d| d.severity == Severity::Error))
            }
            _ => panic!("a program with an error was documented"),
        }
    }

    #[test]
    fn a_package_named_std_stops_it() {
        let dir = fresh("named-std");
        write(
            &dir,
            &[("nova.toml", &manifest("std", "")), ("src/main.nova", "fn main() {}\n")],
        );
        match document_package(&dir) {
            Err(Failure::Other(why)) => assert!(why.contains("`std`"), "{why}"),
            _ => panic!("a package named std was documented"),
        }
    }

    #[test]
    fn every_builtin_a_module_sees_has_docs() {
        for b in Builtin::ALL.iter().filter(|b| !Builtin::STD_ONLY.contains(*b)) {
            assert!(BUILTINS.iter().any(|(n, _)| *n == b.name()), "{}", b.name());
        }
    }
}
```

Run: `cd /home/user/nova && cargo test -p nova-doc 2>&1 | grep -E "^error" | sort | uniq -c | head`
Expected: E0412, E0422 and E0425 errors naming `Site`, `Documented`,
`Failure`, `document_package` and the rest.

- [ ] **Step 2: The types**

In `crates/nova-doc/src/model.rs`, between the module doc comment and
`#[cfg(test)]`, insert:

```rust

use std::collections::{HashMap, HashSet};
use std::path::Path;

use nova_ast::item::{
    ConstDecl, ExternItem, ImplBlock, Record, TraitDecl, TraitItem, TypeDecl, TypeDef,
    Visibility, WhereBound,
};
use nova_ast::{Item as Ast, Type};
use nova_diagnostics::{Diagnostic, FileDb, FileId, Severity, Span, Spanned};
use nova_driver::{analyze_program, Analysis, DiskSources, Options, Program, Roots, Sources};
use nova_lexer::Token;
use nova_pm::PackageId;
use nova_resolver::index::rank;
use nova_resolver::{Builtin, DefId, DefKind, Definitions, ModuleId, Role, Target};

use crate::highlight::highlight;
use crate::markdown::{self, Docs};
use crate::signature::{self, FileText};

/// A program documented: its site, and the warnings its analysis found,
/// with the files they point into.
pub struct Documented {
    pub site: Site,
    pub db: FileDb,
    pub warnings: Vec<Diagnostic>,
}

/// Why a program was not documented.
pub enum Failure {
    /// Its analysis found errors, for the caller to print.
    Diagnostics {
        db: FileDb,
        diagnostics: Vec<Diagnostic>,
    },
    /// Anything else, in a sentence.
    Other(String),
}

/// A site: its packages, std last (spec §6.1).
pub struct Site {
    pub(crate) packages: Vec<Package>,
    /// The page `--open` opens, from the site's root.
    pub(crate) start: String,
}

/// A documented package.
pub(crate) struct Package {
    pub name: String,
    pub version: String,
    pub description: Option<String>,
    pub std: bool,
    /// The root module first, then the rest by name; std's all by name.
    pub modules: Vec<Module>,
    /// std's alone.
    pub primitives: Vec<Primitive>,
    /// std's alone.
    pub builtins: Vec<BuiltinDoc>,
}

/// A documented module, one page (spec §7.2, §7.3).
pub(crate) struct Module {
    pub name: String,
    /// `package`, or `package::module`: its title and its search path.
    pub qual: String,
    /// Its page, from the site's root.
    pub page: String,
    /// Whether it is its package's root module, whose page is the
    /// package's.
    pub root: bool,
    pub docs: Docs,
    /// In source order.
    pub items: Vec<Item>,
    pub other_impls: Vec<Impl>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ItemKind {
    Record,
    Sum,
    Trait,
    Function,
    Constant,
}

impl ItemKind {
    /// The word its heading and the search index show.
    pub fn word(self) -> &'static str {
        match self {
            ItemKind::Record => "record",
            ItemKind::Sum => "type",
            ItemKind::Trait => "trait",
            ItemKind::Function => "fn",
            ItemKind::Constant => "const",
        }
    }

    /// The section a module page lists it under, and that section's `id`.
    pub fn section(self) -> (&'static str, &'static str) {
        match self {
            ItemKind::Record => ("Records", "records"),
            ItemKind::Sum => ("Sum types", "sum-types"),
            ItemKind::Trait => ("Traits", "traits"),
            ItemKind::Function => ("Functions", "functions"),
            ItemKind::Constant => ("Constants", "constants"),
        }
    }
}

/// A documented item (spec §7.3).
pub(crate) struct Item {
    pub kind: ItemKind,
    pub name: String,
    pub anchor: String,
    /// Its signature's HTML (spec §8).
    pub signature: String,
    pub docs: Docs,
    /// Fields, variants, associated types and methods, as §7.3 orders them.
    pub members: Vec<Member>,
    /// Whether it is a record with fields that are not documented.
    pub hidden_fields: bool,
    /// A record's or sum type's impls; a trait's, by their headers.
    pub impls: Vec<Impl>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum MemberKind {
    Field,
    Variant,
    AssocType,
    Method,
}

impl MemberKind {
    /// The word the search index shows.
    pub fn word(self) -> &'static str {
        match self {
            MemberKind::Field => "field",
            MemberKind::Variant => "variant",
            MemberKind::AssocType => "type",
            MemberKind::Method => "method",
        }
    }

    /// The heading a page groups them under.
    pub fn heading(self) -> &'static str {
        match self {
            MemberKind::Field => "Fields",
            MemberKind::Variant => "Variants",
            MemberKind::AssocType => "Associated types",
            MemberKind::Method => "Methods",
        }
    }
}

/// A documented member.
pub(crate) struct Member {
    pub kind: MemberKind,
    pub name: String,
    pub anchor: String,
    pub signature: String,
    pub docs: Docs,
}

/// An impl: its header, its docs and its documented methods (spec §5.2).
pub(crate) struct Impl {
    pub header: String,
    pub docs: Docs,
    pub methods: Vec<Member>,
    /// The type's name, for the search index's paths.
    pub on: String,
}

/// A primitive type's page (spec §7.4).
pub(crate) struct Primitive {
    pub name: &'static str,
    pub page: String,
    pub about: Docs,
    pub impls: Vec<Impl>,
}

/// A builtin function, on std's front page (spec §7.4).
pub(crate) struct BuiltinDoc {
    pub name: &'static str,
    pub signature: String,
    pub docs: Docs,
}

/// The primitive types, with what their pages say first (plan decision
/// 19).
const PRIMITIVES: [(&str, &str); 7] = [
    ("Int", "A 64-bit signed integer. Arithmetic on `Int` wraps on overflow."),
    ("Float", "A 64-bit floating-point number."),
    ("Bool", "`true` or `false`."),
    ("Char", "A Unicode scalar value."),
    ("String", "A string of UTF-8 text."),
    ("Bytes", "A sequence of bytes."),
    ("Future", "The work an `async fn` call stands for, which `.await` runs to its value."),
];

/// The builtin functions every module sees, and their docs (plan decision
/// 19).
const BUILTINS: [(&str, &str); 5] = [
    ("println", "Writes its argument and a line break to standard output."),
    ("print", "Writes its argument to standard output, with no line break."),
    ("eprintln", "Writes its argument and a line break to standard error."),
    ("eprint", "Writes its argument to standard error, with no line break."),
    (
        "panic",
        "Stops the program at once: writes `nova: panic: ` and its argument to standard \
         error, and aborts. Under `nova test`, a test marked `@test(should_panic)` expects it.",
    ),
];

/// What std's front page says first.
const STD_ABOUT: &str =
    "Nova's standard library, which every module sees without an import.";

/// The entry of std's own analysis, which no file holds (plan decision
/// 8).
const STD_ENTRY: &str = "<nova doc>/main.nova";
```

- [ ] **Step 3: Documenting a package, and std**

Append, before `#[cfg(test)]`:

```rust

/// Document the package whose `nova.toml` is in `root` (empty for the
/// current directory), its dependencies and std (spec §5.1).
pub fn document_package(root: &Path) -> Result<Documented, Failure> {
    let mut program = Program::for_package(root, Roots::Check);
    let stem = |p: &Path| p.file_stem().and_then(|s| s.to_str()).map(str::to_string);
    let library = program
        .roots
        .iter()
        .any(|r| stem(&r.path).as_deref() == Some("lib"));
    let keep = if library { "lib" } else { "main" };
    program
        .roots
        .retain(|r| stem(&r.path).as_deref() == Some(keep));
    program.runs = false;
    let options = Options {
        index: true,
        module_only: true,
        ..Options::default()
    };
    let analysis = analyze_program(program, &DiskSources, &options)
        .map_err(|e| Failure::Other(format!("cannot read the program: {e}")))?;
    if analysis
        .diagnostics
        .iter()
        .any(|d| d.severity == Severity::Error)
    {
        return Err(Failure::Diagnostics {
            db: analysis.db,
            diagnostics: analysis.diagnostics,
        });
    }
    let std = std_analysis()?;
    let mut packages = Ctx::new(&analysis, false, !library)?.packages();
    packages.push(Ctx::new(&std, true, false)?.std_package());
    clash(&packages)?;
    let start = format!("{}/index.html", packages[0].name);
    let warnings = analysis.diagnostics.clone();
    Ok(Documented {
        site: Site { packages, start },
        db: analysis.db,
        warnings,
    })
}

/// Document std alone (spec §4.1, `--std`).
pub fn document_std() -> Result<Documented, Failure> {
    let std = std_analysis()?;
    let package = Ctx::new(&std, true, false)?.std_package();
    Ok(Documented {
        site: Site {
            packages: vec![package],
            start: "std/index.html".to_string(),
        },
        db: FileDb::new(),
        warnings: Vec::new(),
    })
}

/// Serves std's analysis its entry, which no file holds.
struct StdEntry;

impl Sources for StdEntry {
    fn read(&self, path: &Path) -> std::io::Result<String> {
        if path == Path::new(STD_ENTRY) {
            Ok("fn main() {}\n".to_string())
        } else {
            Err(std::io::Error::from(std::io::ErrorKind::NotFound))
        }
    }
}

/// An analysis of std alone, `std/test` included (plan decision 8).
fn std_analysis() -> Result<Analysis, Failure> {
    let options = Options {
        tests: true,
        index: true,
        module_only: true,
        ..Options::default()
    };
    let a = analyze_program(Program::loose(Path::new(STD_ENTRY)), &StdEntry, &options)
        .map_err(|e| Failure::Other(format!("cannot analyse std: {e}")))?;
    if a.diagnostics.iter().any(|d| d.severity == Severity::Error) {
        return Err(Failure::Diagnostics {
            db: a.db,
            diagnostics: a.diagnostics,
        });
    }
    Ok(a)
}

/// Spec §4.2: two packages that would share a directory.
fn clash(packages: &[Package]) -> Result<(), Failure> {
    for (k, p) in packages.iter().enumerate() {
        if !p.std && p.name.eq_ignore_ascii_case("std") {
            return Err(Failure::Other(format!(
                "the package `{}` would share std's pages in target/doc/std/, so nova doc \
                 cannot document it",
                p.name
            )));
        }
        if let Some(q) = packages[..k]
            .iter()
            .find(|q| q.name.eq_ignore_ascii_case(&p.name))
        {
            return Err(Failure::Other(format!(
                "the packages `{}` and `{}` would share one directory in target/doc/, so nova \
                 doc cannot document them",
                q.name, p.name
            )));
        }
    }
    Ok(())
}
```

- [ ] **Step 4: Reading an analysis**

Append, before `#[cfg(test)]`:

```rust

/// Which namespace an item is exported in.
#[derive(Clone, Copy)]
enum Ns {
    Value,
    Type,
    Trait,
}

/// An impl met while building modules, placed once all are built.
struct Pending<'a> {
    module: ModuleId,
    span: Span,
    block: &'a ImplBlock,
}

/// Where an impl goes (spec §5.2).
enum Dest {
    /// With its type: the type's page and anchor.
    Item(String, String),
    /// On a primitive's page.
    Primitive(&'static str),
    /// On its own module's page, under "Implementations on other types".
    Own,
}

/// One analysis, read for its site.
struct Ctx<'a> {
    a: &'a Analysis,
    defs: &'a Definitions,
    file: &'a nova_ast::File,
    /// The program's own modules, which come before std's.
    users: usize,
    /// Whether this is std's own analysis, whose entry has no page.
    std: bool,
    /// Whether the root package documents every item and member (spec
    /// §5.1; plan decision 7).
    program: bool,
    /// Each name's target, by its span (spec §6.3).
    uses: HashMap<Span, Target>,
    /// Each rendered module's file, lexed once.
    texts: HashMap<FileId, FileText<'a>>,
    /// Each record's, sum type's and trait's id, by its merged item index.
    by_item: HashMap<usize, DefId>,
}

impl<'a> Ctx<'a> {
    fn new(a: &'a Analysis, std: bool, program: bool) -> Result<Ctx<'a>, Failure> {
        let incomplete =
            || Failure::Other("the analysis is incomplete; this is a nova doc bug".to_string());
        let defs = a.definitions.as_ref().ok_or_else(incomplete)?;
        let file = a.file.as_ref().ok_or_else(incomplete)?;
        let index = a.index.as_ref().ok_or_else(incomplete)?;
        let mut uses: HashMap<Span, Target> = HashMap::new();
        for o in index.occurrences.iter().filter(|o| o.role == Role::Use) {
            let keep = uses
                .get(&o.span)
                .map_or(true, |t| rank(defs, &o.target) < rank(defs, t));
            if keep {
                uses.insert(o.span, o.target);
            }
        }
        let by_item = defs
            .defs()
            .iter()
            .enumerate()
            .filter_map(|(i, d)| match d.kind {
                DefKind::Record { item_index }
                | DefKind::Sum { item_index, .. }
                | DefKind::Trait { item_index } => Some((item_index, DefId(i as u32))),
                _ => None,
            })
            .collect();
        let mut ctx = Ctx {
            a,
            defs,
            file,
            users: a.modules.len(),
            std,
            program,
            uses,
            texts: HashMap::new(),
            by_item,
        };
        for m in 0..defs.module_count() {
            let m = ModuleId(m as u32);
            if !ctx.renders(m) {
                continue;
            }
            if let Some(id) = ctx.module_file(m) {
                if let Some(text) = a.db.get_source(id) {
                    ctx.texts.insert(id, FileText::new(text, id));
                }
            }
        }
        Ok(ctx)
    }

    /// Whether this analysis renders module `m`'s page: std's analysis its
    /// std modules, a project's analysis its own.
    fn renders(&self, m: ModuleId) -> bool {
        ((m.0 as usize) >= self.users) == self.std
    }

    fn std_name(&self, m: ModuleId) -> Option<&str> {
        self.defs.module_name(m).strip_prefix("$std.")
    }

    fn module_file(&self, m: ModuleId) -> Option<FileId> {
        let i = m.0 as usize;
        if i < self.users {
            self.a.modules.get(i).map(|(f, _)| *f)
        } else {
            self.a.db.id_of(&format!("<std/{}>", self.std_name(m)?))
        }
    }

    fn package_id(&self, m: ModuleId) -> Option<PackageId> {
        self.a.module_packages.get(m.0 as usize).copied().flatten()
    }

    fn package_name(&self, m: ModuleId) -> Option<&str> {
        Some(&self.a.graph.as_ref()?.package(self.package_id(m)?).name)
    }

    /// The stem of the root module of `m`'s package (plan decision 9).
    fn root_stem(&self, m: ModuleId) -> &'static str {
        if self.program && self.package_id(m) == Some(PackageId(0)) {
            "main"
        } else {
            "lib"
        }
    }

    /// Module `m`'s page, from the site's root (spec §6.1). Every std
    /// module has one, whichever analysis asks.
    fn page(&self, m: ModuleId) -> Option<String> {
        let i = m.0 as usize;
        if i >= self.users {
            return Some(format!("std/{}/index.html", self.std_name(m)?));
        }
        if self.std {
            return None;
        }
        let stem = self.a.modules.get(i)?.1.file_stem()?.to_str()?;
        let package = self.package_name(m)?;
        Some(if stem == self.root_stem(m) {
            format!("{package}/index.html")
        } else {
            format!("{package}/{stem}/index.html")
        })
    }

    /// Whether module `m` documents every item and member: it is the root
    /// package's, and that package is only a program.
    fn all(&self, m: ModuleId) -> bool {
        self.program && (m.0 as usize) < self.users && self.package_id(m) == Some(PackageId(0))
    }

    fn same_package(&self, a: ModuleId, b: ModuleId) -> bool {
        let std = |m: ModuleId| m.0 as usize >= self.users;
        if std(a) || std(b) {
            std(a) && std(b)
        } else {
            self.package_id(a) == self.package_id(b)
        }
    }

    /// Whether module `m` documents its item `name` (spec §5.1).
    fn shown(&self, m: ModuleId, ns: Ns, name: &str) -> bool {
        if self.all(m) {
            return true;
        }
        let e = self.defs.exported(m, name);
        match ns {
            Ns::Value => e.value.is_some(),
            Ns::Type => e.ty.is_some(),
            Ns::Trait => e.trait_def.is_some(),
        }
    }

    fn member_shown(&self, m: ModuleId, vis: Visibility) -> bool {
        self.all(m) || vis == Visibility::Pub
    }

    /// `target`'s page and anchor from the site's root, when the site
    /// documents it (spec §6.3).
    fn url(&self, target: &Target) -> Option<String> {
        match *target {
            Target::Primitive(p) => Some(format!("std/primitive.{p}.html")),
            Target::Def(id) => {
                let def = self.defs.def(id);
                let item = |item_index: usize, ns: Ns, prefix: &str| {
                    let m = self.defs.module_of(item_index);
                    if !self.shown(m, ns, &def.name) {
                        return None;
                    }
                    Some(format!("{}#{prefix}.{}", self.page(m)?, def.name))
                };
                match &def.kind {
                    DefKind::Fn { item_index } => item(*item_index, Ns::Value, "fn"),
                    DefKind::Const { item_index } => item(*item_index, Ns::Value, "const"),
                    DefKind::Record { item_index } => item(*item_index, Ns::Type, "record"),
                    DefKind::Sum { item_index, .. } => item(*item_index, Ns::Type, "type"),
                    DefKind::Trait { item_index } => item(*item_index, Ns::Trait, "trait"),
                    DefKind::ExternFn { item_index, .. } => {
                        let m = self.defs.module_of(*item_index);
                        if !self.all(m) {
                            return None;
                        }
                        Some(format!("{}#fn.{}", self.page(m)?, def.name))
                    }
                    DefKind::AssocType { trait_def } => {
                        let t = self.url(&Target::Def(*trait_def))?;
                        Some(format!("{t}.type.{}", def.name))
                    }
                    DefKind::Method { .. } => None,
                }
            }
            Target::Variant(sum, vi) => {
                let base = self.url(&Target::Def(sum))?;
                let DefKind::Sum { variants, .. } = &self.defs.def(sum).kind else {
                    return None;
                };
                Some(format!("{base}.variant.{}", variants.get(vi as usize)?.name))
            }
            Target::Field(rec, fi) => {
                let base = self.url(&Target::Def(rec))?;
                let DefKind::Record { item_index } = self.defs.def(rec).kind else {
                    return None;
                };
                let Ast::Record(r) = &self.file.items.get(item_index)?.value else {
                    return None;
                };
                let field = r.fields.get(fi as usize)?;
                if !self.member_shown(self.defs.module_of(item_index), field.vis) {
                    return None;
                }
                Some(format!("{base}.field.{}", field.name.value))
            }
            _ => None,
        }
    }

    /// `text[start..end]`'s signature for a page at `page` (spec §8).
    fn sig(&self, ft: &FileText, page: &str, start: usize, end: usize, own: Option<Span>) -> String {
        signature::render(ft, start, end, own, &|span| {
            self.uses
                .get(&span)
                .and_then(|t| self.url(t))
                .map(|url| relative(page, &url))
        })
    }
}

/// `url`, from the site's root, as seen from `page` (plan decision 13).
pub(crate) fn relative(page: &str, url: &str) -> String {
    format!("{}{url}", "../".repeat(page.matches('/').count()))
}

/// Doc lines rendered, their headings moved down `level` levels.
fn docs_of(lines: &[Spanned<String>], level: u8) -> Docs {
    markdown::render(&markdown::join(lines), level)
}

/// The last segment of a path type: what an impl is on.
fn type_name(ty: &Type) -> Option<&Spanned<String>> {
    match ty {
        Type::Path { path, .. } => path.segments.last(),
        _ => None,
    }
}

/// `base`, or `base-2`, `base-3` and so on: the first not yet on its page
/// (spec §6.2).
fn unique(taken: &mut HashSet<String>, base: String) -> String {
    if taken.insert(base.clone()) {
        return base;
    }
    (2..)
        .map(|n| format!("{base}-{n}"))
        .find(|a| taken.insert(a.clone()))
        .expect("an unused id")
}
```

- [ ] **Step 5: Modules and items**

Append, before `#[cfg(test)]`:

```rust

impl<'a> Ctx<'a> {
    /// Module `m` and its items; its impls go to `pending` (spec §7.3).
    fn module(&self, m: ModuleId, pending: &mut Vec<Pending<'a>>) -> Option<Module> {
        let page = self.page(m)?;
        let ft = self.texts.get(&self.module_file(m)?)?;
        let (name, qual, root) = if m.0 as usize >= self.users {
            let n = self.std_name(m)?.to_string();
            let qual = format!("std::{n}");
            (n, qual, false)
        } else {
            let stem = self.a.modules.get(m.0 as usize)?.1.file_stem()?.to_str()?.to_string();
            let package = self.package_name(m)?.to_string();
            let root = stem == self.root_stem(m);
            let qual = if root {
                package
            } else {
                format!("{package}::{stem}")
            };
            (stem, qual, root)
        };
        let mut items = Vec::new();
        for (i, item) in self.file.items.iter().enumerate() {
            if self.defs.module_of(i) != m {
                continue;
            }
            match &item.value {
                Ast::Function(f) if self.shown(m, Ns::Value, &f.name.value) => {
                    items.push(self.function(ft, &page, &f.name, &f.docs, f.return_ty.as_ref(), &f.where_clause))
                }
                Ast::Extern(block) if self.all(m) => {
                    for e in &block.items {
                        let ExternItem::Fn(sig) = e;
                        items.push(self.function(ft, &page, &sig.name, &sig.docs, sig.return_ty.as_ref(), &sig.where_clause));
                    }
                }
                Ast::Record(r) if self.shown(m, Ns::Type, &r.name.value) => {
                    items.push(self.record(ft, &page, m, r))
                }
                Ast::Type(t)
                    if matches!(t.def, TypeDef::Sum(_)) && self.shown(m, Ns::Type, &t.name.value) =>
                {
                    items.push(self.sum(ft, &page, t))
                }
                Ast::Trait(t) if self.shown(m, Ns::Trait, &t.name.value) => {
                    items.push(self.trait_item(ft, &page, t))
                }
                Ast::Const(c) if self.shown(m, Ns::Value, &c.name.value) => {
                    items.push(self.constant(ft, &page, c))
                }
                Ast::Impl(block) => pending.push(Pending {
                    module: m,
                    span: item.span,
                    block,
                }),
                _ => {}
            }
        }
        Some(Module {
            name,
            qual,
            page,
            root,
            docs: markdown::render(&ft.module_docs(), 1),
            items,
            other_impls: Vec::new(),
        })
    }

    fn function(
        &self,
        ft: &FileText,
        page: &str,
        name: &Spanned<String>,
        docs: &[Spanned<String>],
        ret: Option<&Spanned<Type>>,
        wheres: &[WhereBound],
    ) -> Item {
        let start = signature::decl_start(ft.text, name.span.start as usize);
        let end = signature::fn_end(ft, name.span, ret, wheres);
        Item {
            kind: ItemKind::Function,
            name: name.value.clone(),
            anchor: format!("fn.{}", name.value),
            signature: self.sig(ft, page, start, end, Some(name.span)),
            docs: docs_of(docs, 3),
            members: Vec::new(),
            hidden_fields: false,
            impls: Vec::new(),
        }
    }

    fn record(&self, ft: &FileText, page: &str, m: ModuleId, r: &Record) -> Item {
        let start = signature::decl_start(ft.text, r.name.span.start as usize);
        let end = ft
            .next(r.name.span.end as usize, |t| *t == Token::LBrace)
            .unwrap_or(r.name.span.end as usize);
        let anchor = format!("record.{}", r.name.value);
        let mut members = Vec::new();
        let mut hidden = false;
        for f in &r.fields {
            if !self.member_shown(m, f.vis) {
                hidden = true;
                continue;
            }
            let start = signature::decl_start(ft.text, f.name.span.start as usize);
            members.push(Member {
                kind: MemberKind::Field,
                name: f.name.value.clone(),
                anchor: format!("{anchor}.field.{}", f.name.value),
                signature: self.sig(ft, page, start, f.ty.span.end as usize, Some(f.name.span)),
                docs: docs_of(&f.docs, 4),
            });
        }
        Item {
            kind: ItemKind::Record,
            name: r.name.value.clone(),
            signature: self.sig(ft, page, start, end, Some(r.name.span)),
            anchor,
            docs: docs_of(&r.docs, 3),
            members,
            hidden_fields: hidden,
            impls: Vec::new(),
        }
    }

    fn sum(&self, ft: &FileText, page: &str, t: &TypeDecl) -> Item {
        let start = signature::decl_start(ft.text, t.name.span.start as usize);
        let end = ft
            .next(t.name.span.end as usize, |t| *t == Token::Eq)
            .unwrap_or(t.name.span.end as usize);
        let anchor = format!("type.{}", t.name.value);
        let members = match &t.def {
            TypeDef::Sum(variants) => variants
                .iter()
                .map(|v| {
                    let end = if v.fields.is_empty() {
                        v.name.span.end as usize
                    } else {
                        ft.group_end(v.name.span.end as usize)
                            .unwrap_or(v.name.span.end as usize)
                    };
                    Member {
                        kind: MemberKind::Variant,
                        name: v.name.value.clone(),
                        anchor: format!("{anchor}.variant.{}", v.name.value),
                        signature: self.sig(ft, page, v.name.span.start as usize, end, Some(v.name.span)),
                        docs: docs_of(&v.docs, 4),
                    }
                })
                .collect(),
            TypeDef::Alias(_) => Vec::new(),
        };
        Item {
            kind: ItemKind::Sum,
            name: t.name.value.clone(),
            signature: self.sig(ft, page, start, end, Some(t.name.span)),
            anchor,
            docs: docs_of(&t.docs, 3),
            members,
            hidden_fields: false,
            impls: Vec::new(),
        }
    }

    fn trait_item(&self, ft: &FileText, page: &str, t: &TraitDecl) -> Item {
        let start = signature::decl_start(ft.text, t.name.span.start as usize);
        let end = ft
            .next(t.name.span.end as usize, |t| *t == Token::LBrace)
            .unwrap_or(t.name.span.end as usize);
        let anchor = format!("trait.{}", t.name.value);
        let mut types = Vec::new();
        let mut methods = Vec::new();
        for item in &t.items {
            match item {
                TraitItem::AssocType { docs, name, bounds } => {
                    let end = bounds.last().map_or(name.span.end, |b| b.span.end) as usize;
                    let start = signature::decl_start(ft.text, name.span.start as usize);
                    types.push(Member {
                        kind: MemberKind::AssocType,
                        name: name.value.clone(),
                        anchor: format!("{anchor}.type.{}", name.value),
                        signature: self.sig(ft, page, start, end, Some(name.span)),
                        docs: docs_of(docs, 4),
                    });
                }
                TraitItem::Required(s) => methods.push(self.method(
                    ft,
                    page,
                    &format!("{anchor}.method"),
                    &s.name,
                    &s.docs,
                    s.return_ty.as_ref(),
                    &s.where_clause,
                )),
                TraitItem::Provided(f) => methods.push(self.method(
                    ft,
                    page,
                    &format!("{anchor}.method"),
                    &f.name,
                    &f.docs,
                    f.return_ty.as_ref(),
                    &f.where_clause,
                )),
            }
        }
        types.extend(methods);
        Item {
            kind: ItemKind::Trait,
            name: t.name.value.clone(),
            signature: self.sig(ft, page, start, end, Some(t.name.span)),
            anchor,
            docs: docs_of(&t.docs, 3),
            members: types,
            hidden_fields: false,
            impls: Vec::new(),
        }
    }

    /// A method, its anchor `<prefix>.<name>`.
    #[allow(clippy::too_many_arguments)]
    fn method(
        &self,
        ft: &FileText,
        page: &str,
        prefix: &str,
        name: &Spanned<String>,
        docs: &[Spanned<String>],
        ret: Option<&Spanned<Type>>,
        wheres: &[WhereBound],
    ) -> Member {
        let start = signature::decl_start(ft.text, name.span.start as usize);
        let end = signature::fn_end(ft, name.span, ret, wheres);
        Member {
            kind: MemberKind::Method,
            name: name.value.clone(),
            anchor: format!("{prefix}.{}", name.value),
            signature: self.sig(ft, page, start, end, Some(name.span)),
            docs: docs_of(docs, 4),
        }
    }

    fn constant(&self, ft: &FileText, page: &str, c: &ConstDecl) -> Item {
        let start = signature::decl_start(ft.text, c.name.span.start as usize);
        Item {
            kind: ItemKind::Constant,
            name: c.name.value.clone(),
            anchor: format!("const.{}", c.name.value),
            signature: self.sig(ft, page, start, c.value.span.end as usize, Some(c.name.span)),
            docs: docs_of(&c.docs, 3),
            members: Vec::new(),
            hidden_fields: false,
            impls: Vec::new(),
        }
    }
}
```

- [ ] **Step 6: Impls, packages and std**

Append, before `#[cfg(test)]`:

```rust

impl<'a> Ctx<'a> {
    /// Each impl in `pending`, where spec §5.2 puts it.
    fn place(&self, modules: &mut [Module], primitives: &mut [Primitive], pending: Vec<Pending<'a>>) {
        let mut taken: HashMap<String, HashSet<String>> = HashMap::new();
        for m in modules.iter() {
            let set = taken.entry(m.page.clone()).or_default();
            for item in &m.items {
                set.insert(item.anchor.clone());
                for member in &item.members {
                    set.insert(member.anchor.clone());
                }
            }
        }
        for p in pending {
            let block = p.block;
            let Some(ft) = self.module_file(p.module).and_then(|f| self.texts.get(&f)) else {
                continue;
            };
            let Some(own_page) = self.page(p.module) else {
                continue;
            };
            let on = type_name(&block.ty.value);
            let on_name = on.map_or_else(|| "type".to_string(), |s| s.value.clone());
            let ty = on.and_then(|s| self.uses.get(&s.span)).copied();
            let dest = match ty {
                Some(Target::Def(id)) => {
                    let item_index = match self.defs.def(id).kind {
                        DefKind::Record { item_index } | DefKind::Sum { item_index, .. } => {
                            Some(item_index)
                        }
                        _ => None,
                    };
                    match item_index {
                        Some(ii) if self.same_package(self.defs.module_of(ii), p.module) => {
                            // A type of this package that is not documented
                            // takes its impls with it.
                            let Some(url) = self.url(&Target::Def(id)) else {
                                continue;
                            };
                            let (page, anchor) = url.split_once('#').unwrap_or((url.as_str(), ""));
                            Dest::Item(page.to_string(), anchor.to_string())
                        }
                        _ => Dest::Own,
                    }
                }
                Some(Target::Primitive(name)) if self.std => Dest::Primitive(name),
                _ => Dest::Own,
            };
            let page = match &dest {
                Dest::Item(page, _) => page.clone(),
                Dest::Primitive(name) => format!("std/primitive.{name}.html"),
                Dest::Own => own_page.clone(),
            };
            let header = self.impl_header(ft, &page, p.span, block);
            let mut methods = Vec::new();
            if block.trait_.is_none() {
                let prefix = match &dest {
                    Dest::Item(_, anchor) => format!("{anchor}.method"),
                    Dest::Primitive(_) => "method".to_string(),
                    Dest::Own => format!("impl.{on_name}.method"),
                };
                for f in block
                    .functions
                    .iter()
                    .filter(|f| self.member_shown(p.module, f.vis))
                {
                    let mut m = self.method(ft, &page, &prefix, &f.name, &f.docs, f.return_ty.as_ref(), &f.where_clause);
                    m.anchor = unique(taken.entry(page.clone()).or_default(), m.anchor);
                    methods.push(m);
                }
            }
            let built = Impl {
                header,
                docs: docs_of(&block.docs, 4),
                methods,
                on: on_name.clone(),
            };
            match &dest {
                Dest::Item(page, anchor) => {
                    if let Some(item) = find_item(modules, page, anchor) {
                        item.impls.push(built);
                    }
                }
                Dest::Primitive(name) => {
                    if let Some(pr) = primitives.iter_mut().find(|pr| pr.name == *name) {
                        pr.impls.push(built);
                    }
                }
                Dest::Own => {
                    if let Some(m) = modules.iter_mut().find(|m| m.page == own_page) {
                        m.other_impls.push(built);
                    }
                }
            }
            // A trait of this package also lists the impl, by its header.
            let tr = block
                .trait_
                .as_ref()
                .and_then(|t| t.value.segments.last())
                .and_then(|s| self.uses.get(&s.span))
                .copied();
            if let Some(Target::Def(t)) = tr {
                if let DefKind::Trait { item_index } = self.defs.def(t).kind {
                    if self.same_package(self.defs.module_of(item_index), p.module) {
                        if let Some(url) = self.url(&Target::Def(t)) {
                            let (page, anchor) = url.split_once('#').unwrap_or((url.as_str(), ""));
                            let header = self.impl_header(ft, page, p.span, block);
                            if let Some(item) = find_item(modules, page, anchor) {
                                item.impls.push(Impl {
                                    header,
                                    docs: Docs::default(),
                                    methods: Vec::new(),
                                    on: on_name,
                                });
                            }
                        }
                    }
                }
            }
        }
    }

    /// An impl's header, `impl … {` without its brace (plan decision 10).
    fn impl_header(&self, ft: &FileText, page: &str, span: Span, block: &ImplBlock) -> String {
        let start = ft
            .next(span.start as usize, |t| *t == Token::Impl)
            .unwrap_or(span.start as usize);
        let end = ft
            .next(start, |t| *t == Token::LBrace)
            .unwrap_or(block.ty.span.end as usize);
        self.sig(ft, page, start, end, None)
    }

    /// A project's packages: the root first, then the rest by name (spec
    /// §7.1).
    fn packages(&self) -> Vec<Package> {
        let Some(graph) = self.a.graph.as_ref() else {
            return Vec::new();
        };
        let mut pending = Vec::new();
        let mut modules = Vec::new();
        let mut owners = Vec::new();
        for i in 0..self.users {
            let m = ModuleId(i as u32);
            let Some(pid) = self.package_id(m) else {
                continue;
            };
            if let Some(module) = self.module(m, &mut pending) {
                modules.push(module);
                owners.push(pid);
            }
        }
        self.place(&mut modules, &mut [], pending);
        let mut packages: Vec<Package> = Vec::new();
        for (module, pid) in modules.into_iter().zip(owners) {
            let gp = graph.package(pid);
            let at = match packages.iter().position(|p| p.name == gp.name) {
                Some(at) => at,
                None => {
                    packages.push(Package {
                        name: gp.name.clone(),
                        version: gp.manifest.package.version.to_string(),
                        description: gp.manifest.package.description.clone(),
                        std: false,
                        modules: Vec::new(),
                        primitives: Vec::new(),
                        builtins: Vec::new(),
                    });
                    packages.len() - 1
                }
            };
            packages[at].modules.push(module);
        }
        for p in &mut packages {
            p.modules
                .sort_by(|a, b| (!a.root, &a.name).cmp(&(!b.root, &b.name)));
        }
        let root = graph.root().name.clone();
        packages.sort_by(|a, b| (a.name != root, &a.name).cmp(&(b.name != root, &b.name)));
        packages
    }

    /// std's package, from std's own analysis (spec §5.3, §7.4).
    fn std_package(&self) -> Package {
        let mut pending = Vec::new();
        let mut modules: Vec<Module> = (self.users..self.defs.module_count())
            .filter_map(|i| self.module(ModuleId(i as u32), &mut pending))
            .collect();
        let mut primitives: Vec<Primitive> = PRIMITIVES
            .iter()
            .map(|&(name, about)| Primitive {
                name,
                page: format!("std/primitive.{name}.html"),
                about: markdown::render(about, 1),
                impls: Vec::new(),
            })
            .collect();
        self.place(&mut modules, &mut primitives, pending);
        modules.sort_by(|a, b| a.name.cmp(&b.name));
        let builtins = Builtin::ALL
            .iter()
            .filter(|b| !Builtin::STD_ONLY.contains(*b))
            .map(|b| BuiltinDoc {
                name: b.name(),
                signature: highlight(&nova_typeck::builtin_text(*b, self.defs)),
                docs: markdown::render(
                    BUILTINS
                        .iter()
                        .find(|(n, _)| *n == b.name())
                        .map_or("", |(_, d)| *d),
                    3,
                ),
            })
            .collect();
        Package {
            name: "std".to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            description: Some(STD_ABOUT.to_string()),
            std: true,
            modules,
            primitives,
            builtins,
        }
    }
}

/// The item at `page`'s `anchor`.
fn find_item<'m>(modules: &'m mut [Module], page: &str, anchor: &str) -> Option<&'m mut Item> {
    modules
        .iter_mut()
        .find(|m| m.page == page)?
        .items
        .iter_mut()
        .find(|i| i.anchor == anchor)
}

/// A small site for the renderer's, the search index's and the writer's
/// tests.
#[cfg(test)]
pub(crate) fn sample_site() -> Site {
    let docs = |text: &str| markdown::render(text, 3);
    let point = Item {
        kind: ItemKind::Record,
        name: "Point".to_string(),
        anchor: "record.Point".to_string(),
        signature: "<span class=\"kw\">pub</span> <span class=\"kw\">record</span> \
                    <span class=\"name\">Point</span>"
            .to_string(),
        docs: docs("A point."),
        members: vec![Member {
            kind: MemberKind::Field,
            name: "x".to_string(),
            anchor: "record.Point.field.x".to_string(),
            signature: "<span class=\"kw\">pub</span> x: \
                        <a href=\"../std/primitive.Int.html\">Int</a>"
                .to_string(),
            docs: docs("Across \"x\"."),
        }],
        hidden_fields: true,
        impls: Vec::new(),
    };
    let module = |name: &str, qual: &str, page: &str, root: bool, items: Vec<Item>| Module {
        name: name.to_string(),
        qual: qual.to_string(),
        page: page.to_string(),
        root,
        docs: markdown::render("A module.", 1),
        items,
        other_impls: Vec::new(),
    };
    let app = Package {
        name: "app".to_string(),
        version: "0.1.0".to_string(),
        description: Some("An app.".to_string()),
        std: false,
        modules: vec![
            module("main", "app", "app/index.html", true, vec![point]),
            module("util", "app::util", "app/util/index.html", false, Vec::new()),
        ],
        primitives: Vec::new(),
        builtins: Vec::new(),
    };
    let std = Package {
        name: "std".to_string(),
        version: "0.2.0".to_string(),
        description: Some(STD_ABOUT.to_string()),
        std: true,
        modules: vec![module("core", "std::core", "std/core/index.html", false, Vec::new())],
        primitives: vec![Primitive {
            name: "Int",
            page: "std/primitive.Int.html".to_string(),
            about: markdown::render("An integer.", 1),
            impls: Vec::new(),
        }],
        builtins: vec![BuiltinDoc {
            name: "println",
            signature: "<span class=\"kw\">fn</span> println(String) -&gt; ()".to_string(),
            docs: markdown::render("Writes a line.", 3),
        }],
    };
    Site {
        packages: vec![app, std],
        start: "app/index.html".to_string(),
    }
}
```

- [ ] **Step 7: Run the tests**

Run: `cd /home/user/nova && cargo test -p nova-doc 2>&1 | grep -E "^test result|FAILED|panicked" | head -20`
Expected: `test result: ok. 23 passed`. Three cases may need a ruling
rather than a code change:
- if the checker refuses `impl Int` outside std, remove the `impl Int`
  part of `IMPLS` and its two assertions, and ledger that;
- if a `PRIMITIVES` sentence is wrong against std and the runtime (check
  each: `Int`'s wrapping, `String`'s UTF-8, `Bytes`, `Future`), correct
  the sentence and ledger it;
- if `std_is_one_package_…`'s variant order differs, the code follows the
  source order: correct the expected value only if `std/core` declares
  them in that other order.

- [ ] **Step 8: Clippy, and commit**

Run: `cd /home/user/nova && cargo clippy -p nova-doc --all-targets --locked -- -D warnings 2>&1 | tail -3`
Expected: `Finished`.

Message: `feat(nova-doc): the model, from analyses to a site`, body: what
a site documents (a library's exports, a program's every item, the
dependencies it loads, and std from an analysis of its own), and where
each impl goes (spec 3.5a §5 to §7).

---
### Task 12: Pages, search and themes

**Files:**
- Create: `crates/nova-doc/src/render.rs`, `crates/nova-doc/src/search.rs`
- Create: `crates/nova-doc/assets/nova.css`, `crates/nova-doc/assets/nova.js`
- Modify: `crates/nova-doc/src/lib.rs` (`mod render; mod search;`, and
  `CSS` and `JS`)

**Interfaces:**
- Consumes: the model (Task 11), `model::relative`, `escape`.
- Produces (crate-private): `render::pages(site: &Site) -> Vec<(String,
  String)>`, `render::up(page: &str) -> String`, `search::index(site:
  &Site) -> String`, `CSS` and `JS`.

- [ ] **Step 1: Write the failing tests**

Add `mod render;` and `mod search;` after `mod model;` in `lib.rs`.
Create `crates/nova-doc/src/render.rs` with its doc comment and tests:

```rust
//! Pages (spec §6, §7; plan decisions 12 and 13).

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::sample_site;

    /// The values of each `name="…"` attribute in `html`.
    fn attributes<'h>(html: &'h str, name: &str) -> Vec<&'h str> {
        let pattern = format!(" {name}=\"");
        html.match_indices(&pattern)
            .map(|(at, _)| {
                let value = &html[at + pattern.len()..];
                &value[..value.find('"').unwrap()]
            })
            .collect()
    }

    #[test]
    fn every_page_of_the_sample_is_written() {
        let pages: Vec<String> = pages(&sample_site()).into_iter().map(|(p, _)| p).collect();
        assert_eq!(
            pages,
            [
                "index.html",
                "app/index.html",
                "app/util/index.html",
                "std/index.html",
                "std/primitive.Int.html",
                "std/core/index.html",
            ]
        );
    }

    #[test]
    fn every_url_is_relative_with_slashes() {
        // Review Focus 3.
        for (page, html) in pages(&sample_site()) {
            for url in attributes(&html, "href").into_iter().chain(attributes(&html, "src")) {
                assert!(!url.starts_with('/'), "{page}: {url}");
                assert!(!url.contains('\\'), "{page}: {url}");
                assert!(!url.contains("://"), "{page}: {url}");
            }
        }
    }

    #[test]
    fn a_package_page_shows_its_item_its_members_and_its_modules() {
        let (_, html) = pages(&sample_site())
            .into_iter()
            .find(|(p, _)| p == "app/index.html")
            .unwrap();
        assert!(html.contains("<title>app</title>"), "{html}");
        assert!(html.contains("<body data-root=\"../\">"), "{html}");
        assert!(html.contains("<link rel=\"stylesheet\" href=\"../nova.css\">"), "{html}");
        assert!(html.contains("<h2 id=\"records\">Records</h2>"), "{html}");
        assert!(
            html.contains("<h3 id=\"record.Point\"><a class=\"anchor\" href=\"#record.Point\">record Point</a></h3>"),
            "{html}"
        );
        assert!(html.contains("<div class=\"member\" id=\"record.Point.field.x\">"), "{html}");
        assert!(html.contains("<p class=\"note\">"), "{html}");
        assert!(html.contains("<a href=\"../app/util/index.html\">util</a>"), "{html}");
    }

    #[test]
    fn std_front_page_lists_modules_primitives_and_builtins() {
        let (_, html) = pages(&sample_site())
            .into_iter()
            .find(|(p, _)| p == "std/index.html")
            .unwrap();
        assert!(html.contains("<a href=\"../std/core/index.html\">core</a>"), "{html}");
        assert!(html.contains("<a href=\"../std/primitive.Int.html\">Int</a>"), "{html}");
        assert!(html.contains("<h3 id=\"fn.println\">"), "{html}");
    }
}
```

Create `crates/nova-doc/src/search.rs` with its doc comment and test:

```rust
//! The search index (spec §10.1; plan decision 14).

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::sample_site;

    #[test]
    fn the_index_has_an_entry_per_item_and_member_sorted_and_escaped() {
        let index = index(&sample_site());
        assert!(index.starts_with("window.NOVA_SEARCH = [\n"), "{index}");
        assert!(index.ends_with("\n];\n"), "{index}");
        let lines: Vec<&str> = index.lines().collect();
        assert!(lines.contains(&"[\"Point\",\"record\",\"app::Point\",\"app/index.html#record.Point\",\"A point.\"],"), "{index}");
        assert!(lines.contains(&"[\"x\",\"field\",\"app::Point::x\",\"app/index.html#record.Point.field.x\",\"Across \\\"x\\\".\"],"), "{index}");
        assert!(lines.contains(&"[\"util\",\"module\",\"app::util\",\"app/util/index.html\",\"A module.\"],"), "{index}");
        assert!(lines.iter().any(|l| l.starts_with("[\"println\",\"fn\",\"std::println\",\"std/index.html#fn.println\"")), "{index}");
        let paths: Vec<&str> = lines[1..lines.len() - 1]
            .iter()
            .map(|l| l.split('"').nth(5).unwrap())
            .collect();
        let mut sorted = paths.clone();
        sorted.sort_unstable();
        assert_eq!(paths, sorted);
    }
}
```

Run: `cd /home/user/nova && cargo test -p nova-doc 2>&1 | grep -E "^error" | sort | uniq -c | head`
Expected: E0425 errors naming `pages` and `index`.

- [ ] **Step 2: The renderer**

In `crates/nova-doc/src/render.rs`, between the module doc comment and
`#[cfg(test)]`, insert:

```rust

use crate::escape;
use crate::markdown::Docs;
use crate::model::{
    relative, Impl, Item, ItemKind, Member, MemberKind, Module, Package, Primitive, Site,
};

/// The version every page's footer names.
const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Every page of `site`: its path from the site's root, and its HTML.
pub(crate) fn pages(site: &Site) -> Vec<(String, String)> {
    let mut out = vec![("index.html".to_string(), front(site))];
    for p in &site.packages {
        if p.std {
            out.push(("std/index.html".to_string(), std_front(p)));
            for pr in &p.primitives {
                out.push((pr.page.clone(), primitive(p, pr)));
            }
        }
        for m in &p.modules {
            out.push((m.page.clone(), module(p, m)));
        }
    }
    out
}

/// One `../` for each `/` in `page`: from it back to the site's root.
pub(crate) fn up(page: &str) -> String {
    "../".repeat(page.matches('/').count())
}

/// A whole page: the header, the sidebar, the main part and the footer.
fn page(path: &str, title: &str, sidebar: &str, main: &str) -> String {
    let root = up(path);
    format!(
        "<!DOCTYPE html>\n\
         <html lang=\"en\">\n\
         <head>\n\
         <meta charset=\"utf-8\">\n\
         <meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">\n\
         <title>{title}</title>\n\
         <link rel=\"stylesheet\" href=\"{root}nova.css\">\n\
         <script>try{{var t=localStorage.getItem(\"nova-doc-theme\");\
         if(t)document.documentElement.setAttribute(\"data-theme\",t)}}catch(e){{}}</script>\n\
         <script src=\"{root}search-index.js\" defer></script>\n\
         <script src=\"{root}nova.js\" defer></script>\n\
         </head>\n\
         <body data-root=\"{root}\">\n\
         <header class=\"top\"><a class=\"site\" href=\"{root}index.html\">nova doc</a>\
         <input id=\"search\" type=\"search\" placeholder=\"Search (press /)\" \
         aria-label=\"Search\" autocomplete=\"off\">\
         <button id=\"theme\" type=\"button\" \
         aria-label=\"Switch between the light and the dark theme\">Theme</button></header>\n\
         <div class=\"layout\">\n\
         <nav class=\"sidebar\">\n{sidebar}</nav>\n\
         <main id=\"main\">\n{main}</main>\n\
         <section id=\"results\" class=\"results\" hidden></section>\n\
         </div>\n\
         <footer>Written by nova {version}.</footer>\n\
         </body>\n\
         </html>\n",
        title = escape(title),
        version = VERSION,
    )
}

/// The front page: every package (spec §7.1).
fn front(site: &Site) -> String {
    let mut main = String::from("<h1 class=\"title\">Documentation</h1>\n<ul class=\"list\">\n");
    let mut side = String::from("<h2>Packages</h2>\n<ul>\n");
    for p in &site.packages {
        let description = p
            .description
            .as_deref()
            .map_or(String::new(), |d| format!(" {}", escape(d)));
        main.push_str(&format!(
            "<li><a href=\"{n}/index.html\">{n}</a> <span class=\"version\">{v}</span>{description}</li>\n",
            n = escape(&p.name),
            v = escape(&p.version),
        ));
        side.push_str(&format!(
            "<li><a href=\"{n}/index.html\">{n}</a></li>\n",
            n = escape(&p.name)
        ));
    }
    main.push_str("</ul>\n");
    side.push_str("</ul>\n");
    page("index.html", "Documentation", &side, &main)
}

/// The sidebar of a page at `path` in package `p`, with the page's own
/// anchors and their labels.
fn sidebar(path: &str, p: &Package, on_page: &[(String, String)]) -> String {
    let mut s = format!(
        "<p class=\"package\"><a href=\"{}\">{}</a> <span class=\"version\">{}</span></p>\n",
        relative(path, &format!("{}/index.html", p.name)),
        escape(&p.name),
        escape(&p.version)
    );
    let others: Vec<&Module> = p.modules.iter().filter(|m| !m.root).collect();
    if !others.is_empty() {
        s.push_str("<h2>Modules</h2>\n<ul>\n");
        for m in others {
            s.push_str(&format!(
                "<li><a href=\"{}\">{}</a></li>\n",
                relative(path, &m.page),
                escape(&m.name)
            ));
        }
        s.push_str("</ul>\n");
    }
    if !p.primitives.is_empty() {
        s.push_str("<h2>Primitive types</h2>\n<ul>\n");
        for pr in &p.primitives {
            s.push_str(&format!(
                "<li><a href=\"{}\">{}</a></li>\n",
                relative(path, &pr.page),
                pr.name
            ));
        }
        s.push_str("</ul>\n");
    }
    if !on_page.is_empty() {
        s.push_str("<h2>On this page</h2>\n<ul>\n");
        for (anchor, label) in on_page {
            s.push_str(&format!("<li><a href=\"#{anchor}\">{}</a></li>\n", escape(label)));
        }
        s.push_str("</ul>\n");
    }
    s
}

/// A module's page; a root module's is its package's (spec §7.2, §7.3).
fn module(p: &Package, m: &Module) -> String {
    let mut main = String::new();
    if m.root {
        main.push_str(&format!(
            "<h1 class=\"title\">Package <span class=\"name\">{}</span> \
             <span class=\"version\">{}</span></h1>\n",
            escape(&p.name),
            escape(&p.version)
        ));
        if let Some(d) = &p.description {
            main.push_str(&format!("<p class=\"description\">{}</p>\n", escape(d)));
        }
    } else {
        main.push_str(&format!(
            "<h1 class=\"title\">Module <span class=\"name\">{}</span></h1>\n",
            escape(&m.qual)
        ));
    }
    docs(&mut main, &m.docs);
    let others: Vec<&Module> = p.modules.iter().filter(|o| !o.root).collect();
    if m.root && !others.is_empty() {
        main.push_str("<h2 id=\"modules\">Modules</h2>\n<ul class=\"list\">\n");
        for o in others {
            main.push_str(&format!(
                "<li><a href=\"{}\">{}</a> {}</li>\n",
                relative(&m.page, &o.page),
                escape(&o.name),
                escape(&o.docs.summary)
            ));
        }
        main.push_str("</ul>\n");
    }
    let on_page = sections(&mut main, &m.items, &m.other_impls);
    let title = if m.root { &p.name } else { &m.qual };
    page(&m.page, title, &sidebar(&m.page, p, &on_page), &main)
}

/// A module's items, by section, and its impls on other types (spec
/// §7.3). Returns the page's anchors for the sidebar.
fn sections(out: &mut String, items: &[Item], other: &[Impl]) -> Vec<(String, String)> {
    let mut on_page = Vec::new();
    for kind in [
        ItemKind::Record,
        ItemKind::Sum,
        ItemKind::Trait,
        ItemKind::Function,
        ItemKind::Constant,
    ] {
        let of: Vec<&Item> = items.iter().filter(|i| i.kind == kind).collect();
        if of.is_empty() {
            continue;
        }
        let (title, id) = kind.section();
        out.push_str(&format!("<h2 id=\"{id}\">{title}</h2>\n"));
        for item in of {
            item_html(out, item);
            on_page.push((item.anchor.clone(), item.name.clone()));
        }
    }
    if !other.is_empty() {
        out.push_str("<h2 id=\"implementations\">Implementations on other types</h2>\n");
        for i in other {
            impl_html(out, i);
        }
    }
    on_page
}

fn item_html(out: &mut String, item: &Item) {
    out.push_str(&format!(
        "<section class=\"item\">\n<h3 id=\"{a}\"><a class=\"anchor\" href=\"#{a}\">{w} {n}</a></h3>\n\
         <pre class=\"signature\"><code>{s}</code></pre>\n",
        a = item.anchor,
        w = item.kind.word(),
        n = escape(&item.name),
        s = item.signature
    ));
    docs(out, &item.docs);
    for kind in [
        MemberKind::Field,
        MemberKind::Variant,
        MemberKind::AssocType,
        MemberKind::Method,
    ] {
        let of: Vec<&Member> = item.members.iter().filter(|m| m.kind == kind).collect();
        if of.is_empty() {
            continue;
        }
        out.push_str(&format!("<h4>{}</h4>\n", kind.heading()));
        for m in of {
            member_html(out, m);
        }
    }
    if item.hidden_fields {
        out.push_str("<p class=\"note\">It has private fields, which are not documented.</p>\n");
    }
    if !item.impls.is_empty() {
        out.push_str("<h4>Implementations</h4>\n");
        for i in &item.impls {
            impl_html(out, i);
        }
    }
    out.push_str("</section>\n");
}

fn member_html(out: &mut String, m: &Member) {
    out.push_str(&format!(
        "<div class=\"member\" id=\"{}\">\n<pre class=\"signature\"><code>{}</code></pre>\n",
        m.anchor, m.signature
    ));
    docs(out, &m.docs);
    out.push_str("</div>\n");
}

fn impl_html(out: &mut String, i: &Impl) {
    out.push_str(&format!(
        "<div class=\"impl\">\n<pre class=\"signature\"><code>{}</code></pre>\n",
        i.header
    ));
    docs(out, &i.docs);
    for m in &i.methods {
        member_html(out, m);
    }
    out.push_str("</div>\n");
}

fn docs(out: &mut String, d: &Docs) {
    if !d.html.is_empty() {
        out.push_str("<div class=\"docs\">\n");
        out.push_str(&d.html);
        out.push_str("</div>\n");
    }
}

/// std's front page: its modules, primitive types and builtins (spec
/// §7.4).
fn std_front(p: &Package) -> String {
    let path = "std/index.html";
    let mut main = format!(
        "<h1 class=\"title\">Package <span class=\"name\">std</span> \
         <span class=\"version\">{}</span></h1>\n",
        escape(&p.version)
    );
    if let Some(d) = &p.description {
        main.push_str(&format!("<p class=\"description\">{}</p>\n", escape(d)));
    }
    main.push_str("<h2 id=\"modules\">Modules</h2>\n<ul class=\"list\">\n");
    for m in &p.modules {
        main.push_str(&format!(
            "<li><a href=\"{}\">{}</a> {}</li>\n",
            relative(path, &m.page),
            escape(&m.name),
            escape(&m.docs.summary)
        ));
    }
    main.push_str("</ul>\n<h2 id=\"primitives\">Primitive types</h2>\n<ul class=\"list\">\n");
    for pr in &p.primitives {
        main.push_str(&format!(
            "<li><a href=\"{}\">{}</a> {}</li>\n",
            relative(path, &pr.page),
            pr.name,
            escape(&pr.about.summary)
        ));
    }
    main.push_str("</ul>\n");
    let mut on_page = Vec::new();
    if !p.builtins.is_empty() {
        main.push_str("<h2 id=\"builtins\">Built-in functions</h2>\n");
        for b in &p.builtins {
            main.push_str(&format!(
                "<section class=\"item\">\n<h3 id=\"fn.{n}\"><a class=\"anchor\" href=\"#fn.{n}\">fn {n}</a></h3>\n\
                 <pre class=\"signature\"><code>{s}</code></pre>\n",
                n = b.name,
                s = b.signature
            ));
            docs(&mut main, &b.docs);
            main.push_str("</section>\n");
            on_page.push((format!("fn.{}", b.name), b.name.to_string()));
        }
    }
    page(path, "std", &sidebar(path, p, &on_page), &main)
}

/// A primitive type's page: std's impls on it (spec §7.4).
fn primitive(p: &Package, pr: &Primitive) -> String {
    let mut main = format!(
        "<h1 class=\"title\">Primitive type <span class=\"name\">{}</span></h1>\n",
        pr.name
    );
    docs(&mut main, &pr.about);
    let mut on_page = Vec::new();
    if !pr.impls.is_empty() {
        main.push_str("<h2 id=\"implementations\">Implementations</h2>\n");
        for i in &pr.impls {
            impl_html(&mut main, i);
            for m in &i.methods {
                on_page.push((m.anchor.clone(), m.name.clone()));
            }
        }
    }
    page(&pr.page, pr.name, &sidebar(&pr.page, p, &on_page), &main)
}
```

- [ ] **Step 3: The search index**

In `crates/nova-doc/src/search.rs`, between the module doc comment and
`#[cfg(test)]`, insert:

```rust

use crate::model::{Impl, Site};

/// One entry: name, kind, path, URL from the site's root, and summary.
type Entry = [String; 5];

fn entry(name: &str, kind: &str, path: String, url: String, summary: &str) -> Entry {
    [name.to_string(), kind.to_string(), path, url, summary.to_string()]
}

/// An impl's documented methods, under `path`.
fn methods(entries: &mut Vec<Entry>, path: &str, page: &str, i: &Impl) {
    for m in &i.methods {
        entries.push(entry(
            &m.name,
            m.kind.word(),
            format!("{path}::{}", m.name),
            format!("{page}#{}", m.anchor),
            &m.docs.summary,
        ));
    }
}

/// `search-index.js`: `window.NOVA_SEARCH`, one entry per line (spec
/// §10.1; plan decision 14).
pub(crate) fn index(site: &Site) -> String {
    let mut entries: Vec<Entry> = Vec::new();
    for p in &site.packages {
        entries.push(entry(
            &p.name,
            "package",
            p.name.clone(),
            format!("{}/index.html", p.name),
            p.description.as_deref().unwrap_or(""),
        ));
        for m in &p.modules {
            if !m.root {
                entries.push(entry(&m.name, "module", m.qual.clone(), m.page.clone(), &m.docs.summary));
            }
            for it in &m.items {
                let path = format!("{}::{}", m.qual, it.name);
                entries.push(entry(
                    &it.name,
                    it.kind.word(),
                    path.clone(),
                    format!("{}#{}", m.page, it.anchor),
                    &it.docs.summary,
                ));
                for mem in &it.members {
                    entries.push(entry(
                        &mem.name,
                        mem.kind.word(),
                        format!("{path}::{}", mem.name),
                        format!("{}#{}", m.page, mem.anchor),
                        &mem.docs.summary,
                    ));
                }
                for i in &it.impls {
                    methods(&mut entries, &path, &m.page, i);
                }
            }
            for i in &m.other_impls {
                methods(&mut entries, &format!("{}::{}", m.qual, i.on), &m.page, i);
            }
        }
        for pr in &p.primitives {
            let path = format!("std::{}", pr.name);
            entries.push(entry(pr.name, "primitive", path.clone(), pr.page.clone(), &pr.about.summary));
            for i in &pr.impls {
                methods(&mut entries, &path, &pr.page, i);
            }
        }
        for b in &p.builtins {
            entries.push(entry(
                b.name,
                "fn",
                format!("std::{}", b.name),
                format!("std/index.html#fn.{}", b.name),
                &b.docs.summary,
            ));
        }
    }
    entries.sort_by(|a, b| (&a[2], &a[1], &a[3]).cmp(&(&b[2], &b[1], &b[3])));
    let lines: Vec<String> = entries
        .iter()
        .map(|e| serde_json::to_string(e).expect("strings serialize"))
        .collect();
    format!("window.NOVA_SEARCH = [\n{}\n];\n", lines.join(",\n"))
}
```

- [ ] **Step 4: The assets**

Create `crates/nova-doc/assets/nova.css`:

```css
/* nova doc's themes (spec docs/superpowers/specs/2026-10-11-phase-3-5a-the-documentation-site-design.md §10.2). */
:root {
  --bg: #ffffff;
  --fg: #1d1f21;
  --muted: #5c6370;
  --line: #e1e4e8;
  --panel: #f6f8fa;
  --link: #0b62c4;
  --kw: #8b2bb9;
  --lit: #0a7d38;
  --cm: #6a737d;
  --name: #b35900;
  color-scheme: light;
}
@media (prefers-color-scheme: dark) {
  :root:not([data-theme="light"]) {
    --bg: #16181d;
    --fg: #e3e5e8;
    --muted: #9aa2ad;
    --line: #2c313a;
    --panel: #1f232a;
    --link: #6cb6ff;
    --kw: #d2a8ff;
    --lit: #7ee787;
    --cm: #8b949e;
    --name: #ffa657;
    color-scheme: dark;
  }
}
:root[data-theme="dark"] {
  --bg: #16181d;
  --fg: #e3e5e8;
  --muted: #9aa2ad;
  --line: #2c313a;
  --panel: #1f232a;
  --link: #6cb6ff;
  --kw: #d2a8ff;
  --lit: #7ee787;
  --cm: #8b949e;
  --name: #ffa657;
  color-scheme: dark;
}
* { box-sizing: border-box; }
body {
  margin: 0;
  background: var(--bg);
  color: var(--fg);
  font: 16px/1.5 system-ui, -apple-system, "Segoe UI", Roboto, sans-serif;
}
a { color: var(--link); text-decoration: none; }
a:hover { text-decoration: underline; }
header.top {
  display: flex;
  gap: 12px;
  align-items: center;
  padding: 8px 16px;
  border-bottom: 1px solid var(--line);
  background: var(--panel);
  position: sticky;
  top: 0;
}
header.top .site { font-weight: 600; color: var(--fg); }
#search, #theme {
  padding: 6px 10px;
  border: 1px solid var(--line);
  border-radius: 6px;
  background: var(--bg);
  color: var(--fg);
  font: inherit;
}
#search { flex: 1; max-width: 480px; }
#theme { cursor: pointer; }
.layout { display: flex; max-width: 1200px; margin: 0 auto; }
.sidebar {
  width: 240px;
  flex: none;
  padding: 16px;
  border-right: 1px solid var(--line);
  font-size: 14px;
}
.sidebar h2 {
  font-size: 13px;
  text-transform: uppercase;
  letter-spacing: 0.04em;
  color: var(--muted);
  margin: 16px 0 4px;
  border: none;
}
.sidebar ul { list-style: none; margin: 0; padding: 0; }
main, .results { flex: 1; min-width: 0; padding: 16px 24px 48px; }
h1.title .name, h3 a.anchor, code, pre {
  font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
}
.version { color: var(--muted); font-size: 0.8em; }
h2 { border-bottom: 1px solid var(--line); padding-bottom: 4px; margin-top: 32px; }
section.item { margin: 24px 0; }
h3 a.anchor { color: var(--fg); }
pre {
  background: var(--panel);
  border: 1px solid var(--line);
  border-radius: 6px;
  padding: 8px 12px;
  overflow-x: auto;
  font-size: 14px;
  line-height: 1.45;
}
code { font-size: 0.92em; }
pre code { font-size: inherit; }
.kw { color: var(--kw); }
.lit { color: var(--lit); }
.cm { color: var(--cm); font-style: italic; }
.name { color: var(--name); font-weight: 600; }
.member, .impl { margin: 12px 0 12px 16px; }
.docs { margin: 8px 0; }
.note { color: var(--muted); font-style: italic; }
.list li { margin: 4px 0; }
table { border-collapse: collapse; }
th, td { border: 1px solid var(--line); padding: 4px 8px; }
footer {
  color: var(--muted);
  font-size: 13px;
  text-align: center;
  padding: 24px;
  border-top: 1px solid var(--line);
}
.results ul { list-style: none; padding: 0; }
.results li { margin: 6px 0; }
.results .kind { color: var(--muted); font-size: 13px; margin-right: 6px; }
.results .summary { color: var(--muted); }
@media (max-width: 720px) {
  .layout { display: block; }
  .sidebar { width: auto; border-right: none; border-bottom: 1px solid var(--line); }
}
```

Create `crates/nova-doc/assets/nova.js`:

```js
// nova doc: search and the theme toggle (spec
// docs/superpowers/specs/2026-10-11-phase-3-5a-the-documentation-site-design.md §10).
(function () {
  "use strict";
  var root = document.body.getAttribute("data-root") || "";
  var html = document.documentElement;

  function systemDark() {
    return window.matchMedia && window.matchMedia("(prefers-color-scheme: dark)").matches;
  }

  function currentTheme() {
    return html.getAttribute("data-theme") || (systemDark() ? "dark" : "light");
  }

  var toggle = document.getElementById("theme");
  if (toggle) {
    toggle.addEventListener("click", function () {
      var next = currentTheme() === "dark" ? "light" : "dark";
      html.setAttribute("data-theme", next);
      try {
        localStorage.setItem("nova-doc-theme", next);
      } catch (e) {
        // Storage may be off: the choice then lasts for this page.
      }
    });
  }

  var input = document.getElementById("search");
  var results = document.getElementById("results");
  var main = document.getElementById("main");
  if (!input || !results || !main) {
    return;
  }

  function rank(name, query) {
    var n = name.toLowerCase();
    if (n === query) {
      return 0;
    }
    if (n.indexOf(query) === 0) {
      return 1;
    }
    return n.indexOf(query) >= 0 ? 2 : -1;
  }

  function run() {
    var query = input.value.trim().toLowerCase();
    if (!query) {
      results.hidden = true;
      main.hidden = false;
      return;
    }
    var all = window.NOVA_SEARCH || [];
    var found = [];
    for (var i = 0; i < all.length; i++) {
      var r = rank(all[i][0], query);
      if (r >= 0) {
        found.push([r, i]);
      }
    }
    found.sort(function (a, b) {
      return a[0] - b[0] || a[1] - b[1];
    });
    results.textContent = "";
    var heading = document.createElement("h1");
    heading.textContent = found.length ? "Results" : "No results";
    results.appendChild(heading);
    var list = document.createElement("ul");
    for (var j = 0; j < found.length && j < 50; j++) {
      var e = all[found[j][1]];
      var li = document.createElement("li");
      var kind = document.createElement("span");
      kind.className = "kind";
      kind.textContent = e[1];
      var link = document.createElement("a");
      link.href = root + e[3];
      link.textContent = e[2];
      li.appendChild(kind);
      li.appendChild(link);
      if (e[4]) {
        var summary = document.createElement("div");
        summary.className = "summary";
        summary.textContent = e[4];
        li.appendChild(summary);
      }
      list.appendChild(li);
    }
    results.appendChild(list);
    main.hidden = true;
    results.hidden = false;
  }

  input.addEventListener("input", run);
  document.addEventListener("keydown", function (ev) {
    if (ev.key === "/" && document.activeElement !== input) {
      ev.preventDefault();
      input.focus();
    } else if (ev.key === "Escape" && document.activeElement === input) {
      input.value = "";
      run();
      input.blur();
    }
  });
})();
```

In `crates/nova-doc/src/lib.rs`, after the `pub use model::…` line, add:

```rust

/// The site's stylesheet: its light and dark themes (spec §10.2).
pub(crate) const CSS: &str = include_str!("../assets/nova.css");

/// The site's script: search and the theme toggle (spec §10).
pub(crate) const JS: &str = include_str!("../assets/nova.js");
```

- [ ] **Step 5: Run the tests, clippy, and commit**

Run: `cd /home/user/nova && cargo test -p nova-doc 2>&1 | grep -E "^test result|FAILED|panicked" && cargo clippy -p nova-doc --all-targets --locked -- -D warnings 2>&1 | tail -3`
Expected: `test result: ok. 28 passed`, and `Finished`.

Message: `feat(nova-doc): pages, the search index and two themes`, body:
spec 3.5a §6, §7 and §10. Every URL is relative, so the site works from
disk, and the search box needs no fetch.

---

### Task 13: The link check, and writing the site

**Files:**
- Create: `crates/nova-doc/src/links.rs`, `crates/nova-doc/src/write.rs`
- Modify: `crates/nova-doc/src/lib.rs` (`mod links; mod write;`, the
  re-exports, and the `#![allow(dead_code)]` line goes)

**Interfaces:**
- Produces: `pub fn check_links(dir: &Path) -> Vec<BrokenLink>`, `pub
  struct BrokenLink { pub page: String, pub link: String }`, `pub fn
  write_site(site: &Site, dir: &Path) -> Result<PathBuf, WriteError>`,
  `pub enum WriteError { NotOurs(PathBuf), Io(PathBuf, std::io::Error) }`,
  `pub const MARKER: &str = ".nova-doc"`.

- [ ] **Step 1: Write the failing tests**

Add `mod links;` and `mod write;` after `mod search;` in `lib.rs`, and
after the `pub use model::…` line:

```rust
pub use links::{check_links, BrokenLink};
pub use write::{write_site, WriteError, MARKER};
```

Create `crates/nova-doc/src/links.rs` with its doc comment and tests:

```rust
//! The link check (spec §11).

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    /// A fresh site of `files` in a directory with a fixed name.
    fn site(name: &str, files: &[(&str, &str)]) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("nova-doc-links-{name}"));
        let _ = std::fs::remove_dir_all(&dir);
        for (path, text) in files {
            let path = dir.join(path);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(path, text).unwrap();
        }
        dir
    }

    fn broken(page: &str, link: &str) -> BrokenLink {
        BrokenLink {
            page: page.to_string(),
            link: link.to_string(),
        }
    }

    #[test]
    fn a_missing_file_and_a_missing_fragment_are_broken() {
        let dir = site(
            "broken",
            &[
                (
                    "index.html",
                    "<a href=\"a/index.html\">a</a> <a href=\"a/index.html#here\">h</a> \
                     <a href=\"a/index.html#gone\">g</a> <a href=\"nope.html\">n</a> \
                     <script src=\"x.js\"></script>",
                ),
                (
                    "a/index.html",
                    "<h3 id=\"here\">Here</h3> <a href=\"#here\">s</a> <a href=\"#missing\">m</a> \
                     <a href=\"../index.html\">u</a> <a href=\"../../out.html\">o</a>",
                ),
                ("x.js", ""),
            ],
        );
        assert_eq!(
            check_links(&dir),
            vec![
                broken("a/index.html", "#missing"),
                broken("a/index.html", "../../out.html"),
                broken("index.html", "a/index.html#gone"),
                broken("index.html", "nope.html"),
            ]
        );
    }

    #[test]
    fn absolute_urls_are_not_checked() {
        let dir = site(
            "absolute",
            &[(
                "index.html",
                "<a href=\"https://example.com/x\">x</a> <a href=\"mailto:a@b.c\">m</a> \
                 <a href=\"//cdn.example/x.js\">c</a>",
            )],
        );
        assert_eq!(check_links(&dir), Vec::new());
    }

    #[test]
    fn an_escaped_or_encoded_link_is_read_as_its_target() {
        let dir = site(
            "encoded",
            &[
                ("index.html", "<a href=\"a%20b.html#x&amp;y\">e</a>"),
                ("a b.html", "<p id=\"x&amp;y\">p</p>"),
            ],
        );
        assert_eq!(check_links(&dir), Vec::new());
    }
}
```

Create `crates/nova-doc/src/write.rs` with its doc comment and tests:

```rust
//! Writing the site to `target/doc/` (spec §4.3, §6.1).

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::sample_site;

    fn fresh(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("nova-doc-write-{name}"));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn the_sample_site_is_written_whole_and_its_links_hold() {
        let dir = fresh("whole").join("doc");
        let start = write_site(&sample_site(), &dir).unwrap();
        assert_eq!(start, dir.join("app").join("index.html"));
        for file in [
            MARKER,
            "index.html",
            "nova.css",
            "nova.js",
            "search-index.js",
            "app/index.html",
            "app/util/index.html",
            "std/index.html",
            "std/core/index.html",
            "std/primitive.Int.html",
        ] {
            assert!(dir.join(file).is_file(), "{file}");
        }
        assert_eq!(crate::check_links(&dir), Vec::new());
    }

    #[test]
    fn a_site_it_wrote_is_replaced_and_another_directory_is_refused() {
        let base = fresh("replace");
        let dir = base.join("doc");
        write_site(&sample_site(), &dir).unwrap();
        std::fs::write(dir.join("stale.html"), "old").unwrap();
        write_site(&sample_site(), &dir).unwrap();
        assert!(!dir.join("stale.html").exists());
        let other = base.join("other");
        std::fs::create_dir_all(&other).unwrap();
        std::fs::write(other.join("keep.txt"), "keep").unwrap();
        assert!(matches!(
            write_site(&sample_site(), &other),
            Err(WriteError::NotOurs(_))
        ));
        assert!(other.join("keep.txt").is_file());
        let empty = base.join("empty");
        std::fs::create_dir_all(&empty).unwrap();
        write_site(&sample_site(), &empty).unwrap();
    }

    #[test]
    fn every_file_ends_its_lines_with_a_line_feed() {
        let dir = fresh("endings").join("doc");
        write_site(&sample_site(), &dir).unwrap();
        for file in ["index.html", "nova.css", "nova.js", "search-index.js", "app/index.html"] {
            let text = std::fs::read_to_string(dir.join(file)).unwrap();
            assert!(!text.contains('\r'), "{file}");
        }
    }
}
```

Run: `cd /home/user/nova && cargo test -p nova-doc 2>&1 | grep -E "^error" | sort | uniq -c | head`
Expected: E0425 and E0412 errors naming `check_links`, `BrokenLink`,
`write_site`, `WriteError` and `MARKER`.

- [ ] **Step 2: The link check**

In `crates/nova-doc/src/links.rs`, between the module doc comment and
`#[cfg(test)]`, insert:

```rust

use std::collections::{HashMap, HashSet};
use std::path::Path;

/// A link that leads nowhere: the page it is on, from the site's root,
/// and the link as written.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BrokenLink {
    pub page: String,
    pub link: String,
}

/// Every link in the site at `dir` that is not absolute and leads to no
/// file of the site, or to no `id` on its page (spec §11). Pages in order
/// of their paths, and each page's links in order.
pub fn check_links(dir: &Path) -> Vec<BrokenLink> {
    let mut pages = Vec::new();
    walk(dir, "", &mut pages);
    pages.sort();
    let texts: HashMap<&str, String> = pages
        .iter()
        .map(|p| (p.as_str(), std::fs::read_to_string(dir.join(p)).unwrap_or_default()))
        .collect();
    let ids: HashMap<&str, HashSet<String>> = texts
        .iter()
        .map(|(p, t)| (*p, attributes(t, "id").into_iter().collect()))
        .collect();
    let mut broken = Vec::new();
    for page in &pages {
        let text = &texts[page.as_str()];
        for link in attributes(text, "href").into_iter().chain(attributes(text, "src")) {
            if !resolves(dir, page, &link, &ids) {
                broken.push(BrokenLink {
                    page: page.clone(),
                    link,
                });
            }
        }
    }
    broken
}

/// Each `.html` file under `dir`, by its path from the site's root.
fn walk(dir: &Path, prefix: &str, out: &mut Vec<String>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().into_owned();
        let rel = if prefix.is_empty() {
            name.clone()
        } else {
            format!("{prefix}/{name}")
        };
        let path = entry.path();
        if path.is_dir() {
            walk(&path, &rel, out);
        } else if name.ends_with(".html") {
            out.push(rel);
        }
    }
}

/// The values of every `name="…"` attribute in `html`, unescaped.
fn attributes(html: &str, name: &str) -> Vec<String> {
    let pattern = format!(" {name}=\"");
    let mut out = Vec::new();
    let mut rest = html;
    while let Some(at) = rest.find(&pattern) {
        let value = &rest[at + pattern.len()..];
        let end = value.find('"').unwrap_or(value.len());
        out.push(unescape(&value[..end]));
        rest = &value[end..];
    }
    out
}

/// Whether `link`, on `page`, leads to a file of the site, and to an `id`
/// on it when it names one.
fn resolves(dir: &Path, page: &str, link: &str, ids: &HashMap<&str, HashSet<String>>) -> bool {
    if link.starts_with("//") || has_scheme(link) {
        return true;
    }
    let (path, fragment) = link.split_once('#').unwrap_or((link, ""));
    let path = path.split('?').next().unwrap_or("");
    let target = if path.is_empty() {
        page.to_string()
    } else {
        match join(page, &decode(path)) {
            Some(t) => t,
            None => return false,
        }
    };
    match ids.get(target.as_str()) {
        Some(on_page) => fragment.is_empty() || on_page.contains(&decode(fragment)),
        None => fragment.is_empty() && dir.join(&target).is_file(),
    }
}

/// `link`, relative to `page`'s directory, as a path from the site's root;
/// `None` when it leaves the site or starts at a root.
fn join(page: &str, link: &str) -> Option<String> {
    if link.starts_with('/') {
        return None;
    }
    let mut parts: Vec<&str> = page.split('/').collect();
    parts.pop();
    for part in link.split('/') {
        match part {
            "" | "." => {}
            ".." => {
                parts.pop()?;
            }
            p => parts.push(p),
        }
    }
    Some(parts.join("/"))
}

/// Whether `link` starts with a URL scheme, such as `https:` or `mailto:`.
fn has_scheme(link: &str) -> bool {
    let Some(colon) = link.find(':') else {
        return false;
    };
    let scheme = &link[..colon];
    scheme.starts_with(|c: char| c.is_ascii_alphabetic())
        && scheme
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '+' | '-' | '.'))
}

/// `text` with its `%XX` escapes decoded.
fn decode(text: &str) -> String {
    let bytes = text.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        let hex = bytes
            .get(i + 1..i + 3)
            .and_then(|h| std::str::from_utf8(h).ok())
            .and_then(|h| u8::from_str_radix(h, 16).ok());
        match (bytes[i], hex) {
            (b'%', Some(b)) => {
                out.push(b);
                i += 3;
            }
            (b, _) => {
                out.push(b);
                i += 1;
            }
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// An attribute's value with its entities decoded.
fn unescape(text: &str) -> String {
    text.replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&amp;", "&")
}
```

- [ ] **Step 3: Writing the site**

In `crates/nova-doc/src/write.rs`, between the module doc comment and
`#[cfg(test)]`, insert:

```rust

use std::path::{Path, PathBuf};

use crate::model::Site;
use crate::{render, search, CSS, JS};

/// The file every site holds, so that `nova doc` replaces only a
/// directory it wrote (spec §4.3).
pub const MARKER: &str = ".nova-doc";

const MARKER_TEXT: &str =
    "This directory was written by nova doc, which replaces it on its next run.\n";

/// Why a site was not written.
#[derive(Debug, thiserror::Error)]
pub enum WriteError {
    #[error("{} exists and was not written by nova doc; remove it, and nova doc will write its site there", .0.display())]
    NotOurs(PathBuf),
    #[error("cannot write {}: {}", .0.display(), .1)]
    Io(PathBuf, std::io::Error),
}

/// Write `site` to `dir`, replacing a site written there before (spec
/// §4.3). Returns the start page's path.
pub fn write_site(site: &Site, dir: &Path) -> Result<PathBuf, WriteError> {
    let io = |path: &Path| {
        let path = path.to_path_buf();
        move |e| WriteError::Io(path, e)
    };
    if dir.exists() {
        if dir.join(MARKER).is_file() {
            std::fs::remove_dir_all(dir).map_err(io(dir))?;
        } else if std::fs::read_dir(dir).map_err(io(dir))?.next().is_some() {
            return Err(WriteError::NotOurs(dir.to_path_buf()));
        }
    }
    let mut files = render::pages(site);
    files.push(("nova.css".to_string(), CSS.to_string()));
    files.push(("nova.js".to_string(), JS.to_string()));
    files.push(("search-index.js".to_string(), search::index(site)));
    files.push((MARKER.to_string(), MARKER_TEXT.to_string()));
    for (path, text) in &files {
        let full = under(dir, path);
        if let Some(parent) = full.parent() {
            std::fs::create_dir_all(parent).map_err(io(parent))?;
        }
        std::fs::write(&full, text.replace("\r\n", "\n")).map_err(io(&full))?;
    }
    Ok(under(dir, &site.start))
}

/// `relative`, a `/`-separated path, under `dir`.
fn under(dir: &Path, relative: &str) -> PathBuf {
    relative
        .split('/')
        .fold(dir.to_path_buf(), |path, part| path.join(part))
}
```

The `replace("\r\n", "\n")` keeps the site's line endings `\n` when a
checkout's assets hold `\r\n` (spec §6.1).

In `lib.rs`, delete the two comment lines and `#![allow(dead_code)]`
added in Task 9.

- [ ] **Step 4: Run the tests, clippy and the MSRV check**

Run: `cd /home/user/nova && cargo test -p nova-doc 2>&1 | grep -E "^test result|FAILED|panicked" && cargo clippy -p nova-doc --all-targets --locked -- -D warnings 2>&1 | tail -3 && RUSTFLAGS="-D warnings" cargo +1.78 check --locked -p nova-doc 2>&1 | tail -2`
Expected: `test result: ok. 34 passed`, `Finished` twice. A dead-code
error now means the model has a field nothing reads: delete the field,
or read it where the spec says it shows, and ledger which.

- [ ] **Step 5: Commit**

Message: `feat(nova-doc): the link check, and writing target/doc/`, body:
a site replaces only a directory that holds its marker, and every
relative link is checked against the files and ids written (spec 3.5a
§4.3, §11).

---
### Task 14: `nova doc`

**Files:**
- Create: `crates/nova-cli/src/cmd/doc.rs`
- Modify: `crates/nova-cli/src/cmd/mod.rs` (`pub mod doc;`)
- Modify: `crates/nova-cli/src/main.rs` (the module doc, `Command::Doc`,
  its dispatch)
- Test: `crates/nova-cli/tests/doc.rs` (new)

**Interfaces:**
- Consumes: `nova_doc::{document_package, document_std, write_site,
  check_links, Failure}`; `crate::project::{mode, sync, Mode}`.

- [ ] **Step 1: Write the failing tests**

Create `crates/nova-cli/tests/doc.rs`:

```rust
//! `nova doc` end to end (spec
//! `docs/superpowers/specs/2026-10-11-phase-3-5a-the-documentation-site-design.md`
//! §4, §16.3).

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use assert_cmd::assert::Assert;
use assert_cmd::Command;

fn nova() -> Command {
    Command::cargo_bin("nova").expect("nova binary builds")
}

/// A fresh, empty directory with a fixed name, so each run replaces the
/// last.
fn fresh(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("nova-doc-{name}"));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("create the test directory");
    dir
}

fn write(dir: &Path, files: &[(&str, &str)]) {
    for (path, text) in files {
        let path = dir.join(path);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, text).unwrap();
    }
}

fn manifest(name: &str, extra: &str) -> String {
    format!("[package]\nname = \"{name}\"\nversion = \"0.1.0\"\nedition = \"2026\"\n{extra}")
}

/// `nova doc ARGS` in `dir`, with a fresh `NOVA_HOME` beside it. Colour
/// is off: `nova` colours its diagnostics even into a pipe.
fn doc(dir: &Path, args: &[&str]) -> Assert {
    let home = dir.with_file_name(format!(
        "{}-home",
        dir.file_name().unwrap().to_string_lossy()
    ));
    let _ = std::fs::remove_dir_all(&home);
    nova()
        .current_dir(dir)
        .env("NOVA_HOME", &home)
        .env("NO_COLOR", "1")
        .arg("doc")
        .args(args)
        .assert()
}

fn stdout(a: &Assert) -> String {
    String::from_utf8_lossy(&a.get_output().stdout)
        .trim()
        .replace('\\', "/")
}

fn stderr(a: &Assert) -> String {
    String::from_utf8_lossy(&a.get_output().stderr).into_owned()
}

/// A page of the site under `dir/target/doc`.
fn page(dir: &Path, path: &str) -> String {
    std::fs::read_to_string(dir.join("target").join("doc").join(path))
        .unwrap_or_else(|e| panic!("{path}: {e}"))
}

const GEOM_LIB: &str = "import utils\n\n/// A point.\npub record Point {\n    /// Across.\n    pub x: Int,\n    y: Int,\n}\n\nimpl Point {\n    /// Its x.\n    pub fn across(self) -> Int { self.x }\n\n    fn down(self) -> Int { self.y }\n}\n\n/// The area of a square of side `width()`.\npub fn area() -> Int { width() * width() }\n\nfn secret() -> Int { 1 }\n";
const GEOM_UTILS: &str = "/// Three.\npub fn width() -> Int { 3 }\n";
const APP_MAIN: &str = "import geom\n\n/// Shows `p`.\nfn show(p: Point) -> String { \"${p.x}\" }\n\n/// A record of the app's own.\nrecord Local {\n    hidden: Int,\n}\n\nfn main() {\n    println(show(Point { x: area(), y: 0 }))\n}\n";

/// `dir/geom`, a library, and `dir/app`, a program that uses it, with
/// `extra` appended to the app's manifest. Returns the app's directory.
fn geom_and_app(dir: &Path, extra: &str) -> PathBuf {
    write(
        &dir.join("geom"),
        &[
            ("nova.toml", &manifest("geom", "")),
            ("src/lib.nova", GEOM_LIB),
            ("src/utils.nova", GEOM_UTILS),
        ],
    );
    let deps = format!("\n[dependencies]\ngeom = {{ path = \"../geom\" }}\n{extra}");
    write(
        &dir.join("app"),
        &[("nova.toml", &manifest("app", &deps)), ("src/main.nova", APP_MAIN)],
    );
    dir.join("app")
}

#[test]
fn a_program_and_its_path_library_are_documented() {
    let app = geom_and_app(&fresh("program"), "");
    let a = doc(&app, &[]).success();
    assert_eq!(stdout(&a), "target/doc/app/index.html");
    for file in [
        ".nova-doc",
        "index.html",
        "nova.css",
        "nova.js",
        "search-index.js",
        "app/index.html",
        "geom/index.html",
        "geom/utils/index.html",
        "std/index.html",
        "std/core/index.html",
        "std/primitive.String.html",
    ] {
        assert!(app.join("target/doc").join(file).is_file(), "{file}");
    }
    let main = page(&app, "app/index.html");
    for id in ["fn.show", "fn.main", "record.Local", "record.Local.field.hidden"] {
        assert!(main.contains(&format!(" id=\"{id}\"")), "{id}");
    }
    assert!(main.contains("href=\"../geom/index.html#record.Point\""), "{main}");
    assert!(main.contains("href=\"../std/primitive.String.html\""), "{main}");
}

#[test]
fn a_library_documents_its_exports_only() {
    let dir = fresh("library");
    geom_and_app(&dir, "");
    let geom = dir.join("geom");
    let a = doc(&geom, &[]).success();
    assert_eq!(stdout(&a), "target/doc/geom/index.html");
    let lib = page(&geom, "geom/index.html");
    for id in [
        "record.Point",
        "record.Point.field.x",
        "record.Point.method.across",
        "fn.area",
    ] {
        assert!(lib.contains(&format!(" id=\"{id}\"")), "{id}");
    }
    for id in ["record.Point.field.y", "record.Point.method.down", "fn.secret"] {
        assert!(!lib.contains(&format!(" id=\"{id}\"")), "{id}");
    }
    assert!(lib.contains("private fields"), "{lib}");
    assert!(page(&geom, "geom/utils/index.html").contains(" id=\"fn.width\""));
}

#[test]
fn an_unused_dependency_has_no_pages() {
    let dir = fresh("unused");
    write(
        &dir.join("extra"),
        &[
            ("nova.toml", &manifest("extra", "")),
            ("src/lib.nova", "pub fn one() -> Int { 1 }\n"),
        ],
    );
    let app = geom_and_app(&dir, "extra = { path = \"../extra\" }\n");
    doc(&app, &[]).success();
    assert!(app.join("target/doc/geom/index.html").is_file());
    assert!(!app.join("target/doc/extra").exists());
}

#[test]
fn an_error_stops_it_and_a_warning_does_not() {
    let dir = fresh("error");
    write(
        &dir,
        &[
            ("nova.toml", &manifest("app", "")),
            ("src/main.nova", "fn main() {\n    let x: Int = \"s\"\n}\n"),
        ],
    );
    let a = doc(&dir, &[]).failure();
    let err = stderr(&a);
    assert!(err.contains("error["), "{err}");
    assert!(err.contains("could not document due to 1 previous error"), "{err}");
    assert!(!dir.join("target").join("doc").exists());

    let dir = fresh("warning");
    write(
        &dir,
        &[
            ("nova.toml", &manifest("app", "")),
            (
                "src/main.nova",
                "fn main() {\n    let x = 1\n    match x {\n        _ => println(\"a\")\n        1 => println(\"b\")\n    }\n}\n",
            ),
        ],
    );
    let a = doc(&dir, &[]).success();
    assert!(stderr(&a).contains("warning[E0021]"), "{}", stderr(&a));
}

#[test]
fn outside_a_project_it_names_std() {
    let dir = fresh("outside");
    let a = doc(&dir, &[]).failure();
    assert!(stderr(&a).contains("`nova doc --std`"), "{}", stderr(&a));
}

#[test]
fn std_alone() {
    let dir = fresh("std");
    let a = doc(&dir, &["--std"]).success();
    assert_eq!(stdout(&a), "target/doc/std/index.html");
    let mut top: Vec<String> = std::fs::read_dir(dir.join("target/doc"))
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    top.sort();
    assert_eq!(
        top,
        [".nova-doc", "index.html", "nova.css", "nova.js", "search-index.js", "std"]
    );
    assert!(dir.join("target/doc/std/test/index.html").is_file());
}

#[test]
fn a_second_run_replaces_the_site() {
    // Review Focus 5.
    let dir = fresh("replace");
    write(
        &dir,
        &[
            ("nova.toml", &manifest("app", "")),
            ("src/main.nova", "import util\n\nfn main() {\n    println(\"${one()}\")\n}\n"),
            ("src/util.nova", "pub fn one() -> Int { 1 }\n"),
        ],
    );
    doc(&dir, &[]).success();
    assert!(dir.join("target/doc/app/util/index.html").is_file());
    std::fs::remove_file(dir.join("src/util.nova")).unwrap();
    std::fs::write(dir.join("src/main.nova"), "fn main() {}\n").unwrap();
    doc(&dir, &[]).success();
    assert!(!dir.join("target/doc/app/util").exists());
}

#[test]
fn a_target_doc_it_did_not_write_is_refused() {
    let dir = fresh("not-ours");
    write(
        &dir,
        &[
            ("nova.toml", &manifest("app", "")),
            ("src/main.nova", "fn main() {}\n"),
            ("target/doc/keep.txt", "keep"),
        ],
    );
    let a = doc(&dir, &[]).failure();
    assert!(stderr(&a).contains("was not written by nova doc"), "{}", stderr(&a));
    assert!(dir.join("target/doc/keep.txt").is_file());
}

#[test]
fn a_package_named_std_is_refused() {
    let dir = fresh("named-std");
    write(
        &dir,
        &[("nova.toml", &manifest("std", "")), ("src/main.nova", "fn main() {}\n")],
    );
    let a = doc(&dir, &[]).failure();
    assert!(stderr(&a).contains("`std`"), "{}", stderr(&a));
}

#[test]
fn a_broken_doc_link_is_a_warning() {
    // Review Focus 4.
    let dir = fresh("doc-link");
    write(
        &dir,
        &[
            ("nova.toml", &manifest("app", "")),
            ("src/main.nova", "/// See [the guide](guide.html).\nfn main() {}\n"),
        ],
    );
    let a = doc(&dir, &[]).success();
    assert!(
        stderr(&a).contains("warning: app/index.html: broken link `guide.html`"),
        "{}",
        stderr(&a)
    );
}

#[test]
fn open_with_a_missing_browser_warns() {
    let dir = fresh("open-missing");
    write(
        &dir,
        &[("nova.toml", &manifest("app", "")), ("src/main.nova", "fn main() {}\n")],
    );
    let a = nova()
        .current_dir(&dir)
        .env("NOVA_HOME", dir.with_file_name("nova-doc-open-missing-home"))
        .env("BROWSER", dir.join("no-such-browser"))
        .args(["doc", "--open"])
        .assert()
        .success();
    let err = stderr(&a);
    assert!(err.contains("could not open a browser"), "{err}");
    assert!(err.replace('\\', "/").contains("target/doc/app/index.html"), "{err}");
}

#[cfg(unix)]
#[test]
fn open_runs_the_browser_with_the_start_page() {
    use std::os::unix::fs::PermissionsExt;
    let dir = fresh("open-runs");
    write(
        &dir,
        &[("nova.toml", &manifest("app", "")), ("src/main.nova", "fn main() {}\n")],
    );
    let opened = dir.join("opened.txt");
    let browser = dir.join("browser.sh");
    std::fs::write(
        &browser,
        format!("#!/bin/sh\necho \"$1\" > \"{}\"\n", opened.display()),
    )
    .unwrap();
    std::fs::set_permissions(&browser, std::fs::Permissions::from_mode(0o755)).unwrap();
    nova()
        .current_dir(&dir)
        .env("NOVA_HOME", dir.with_file_name("nova-doc-open-runs-home"))
        .env("BROWSER", &browser)
        .args(["doc", "--open"])
        .assert()
        .success();
    // The browser runs on its own; give it up to ten seconds.
    let mut seen = String::new();
    for _ in 0..100 {
        seen = std::fs::read_to_string(&opened).unwrap_or_default();
        if !seen.is_empty() {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
    assert!(seen.trim().ends_with("target/doc/app/index.html"), "{seen:?}");
    assert!(Path::new(seen.trim()).is_absolute(), "{seen:?}");
}

/// Every file under `dir`, by its path, with its bytes.
fn files(dir: &Path, prefix: &str, out: &mut BTreeMap<String, Vec<u8>>) {
    for entry in std::fs::read_dir(dir).unwrap() {
        let entry = entry.unwrap();
        let name = format!("{prefix}{}", entry.file_name().to_string_lossy());
        if entry.path().is_dir() {
            files(&entry.path(), &format!("{name}/"), out);
        } else {
            out.insert(name, std::fs::read(entry.path()).unwrap());
        }
    }
}

#[test]
fn two_runs_write_the_same_bytes() {
    let app = geom_and_app(&fresh("same-bytes"), "");
    doc(&app, &[]).success();
    let mut first = BTreeMap::new();
    files(&app.join("target/doc"), "", &mut first);
    doc(&app, &[]).success();
    let mut second = BTreeMap::new();
    files(&app.join("target/doc"), "", &mut second);
    assert_eq!(first.keys().collect::<Vec<_>>(), second.keys().collect::<Vec<_>>());
    for (name, bytes) in &first {
        assert!(second[name] == *bytes, "{name} differs");
    }
}
```

Run: `cd /home/user/nova && cargo test -p nova-cli --test doc 2>&1 | grep -E "^test |test result" | head -20`
Expected: the build succeeds, and every test FAILS: `nova doc` is not a
subcommand yet, so clap exits 2 with "unrecognized subcommand".

- [ ] **Step 2: The command**

Create `crates/nova-cli/src/cmd/doc.rs`:

```rust
//! `nova doc [--open] [--std]`: the documentation site (spec
//! `docs/superpowers/specs/2026-10-11-phase-3-5a-the-documentation-site-design.md`
//! §4; ADR 0034).

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use anyhow::{anyhow, bail, Result};
use clap::Args;
use nova_diagnostics::{render, Severity};
use nova_doc::Failure;

use crate::project::{self, Mode};

#[derive(Args)]
pub struct DocCmd {
    /// Open the site's start page afterwards, with $BROWSER or the
    /// system's opener.
    #[arg(long)]
    open: bool,

    /// Document std alone, inside a project or not.
    #[arg(long)]
    std: bool,
}

pub fn run(cmd: DocCmd) -> Result<()> {
    let mode = project::mode(None)?;
    let documented = if cmd.std {
        nova_doc::document_std().map_err(failure)?
    } else {
        let Mode::Project { root, .. } = &mode else {
            bail!("nova doc documents a project; run it inside one, or use `nova doc --std`");
        };
        project::sync(&mode)?;
        nova_doc::document_package(root).map_err(failure)?
    };
    render::emit_all(&documented.db, &documented.warnings);
    let dir = match &mode {
        Mode::Project { target_dir, .. } => target_dir.join("doc"),
        Mode::File(_) => PathBuf::from("target").join("doc"),
    };
    let start = nova_doc::write_site(&documented.site, &dir)?;
    for broken in nova_doc::check_links(&dir) {
        eprintln!("warning: {}: broken link `{}`", broken.page, broken.link);
    }
    println!("{}", start.display());
    if cmd.open {
        open(&start);
    }
    Ok(())
}

/// A failure as the command's error, its diagnostics printed first (plan
/// decision 15).
fn failure(f: Failure) -> anyhow::Error {
    match f {
        Failure::Diagnostics { db, diagnostics } => {
            render::emit_all(&db, &diagnostics);
            let errors = diagnostics
                .iter()
                .filter(|d| d.severity == Severity::Error)
                .count();
            anyhow!(
                "could not document due to {errors} previous error{}",
                if errors == 1 { "" } else { "s" }
            )
        }
        Failure::Other(why) => anyhow!(why),
    }
}

/// Open `page` (spec §4.1; plan decision 16) with `$BROWSER` when it is
/// set, else with the system's opener. A failure is a warning.
fn open(page: &Path) {
    let page = if page.is_absolute() {
        page.to_path_buf()
    } else {
        std::env::current_dir().map_or_else(|_| page.to_path_buf(), |d| d.join(page))
    };
    let mut command = match std::env::var_os("BROWSER") {
        Some(browser) if !browser.is_empty() => Command::new(browser),
        _ => opener(),
    };
    command
        .arg(&page)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    match command.spawn() {
        // nova exits next and the browser outlives it; the thread only
        // reaps it if it closes first.
        Ok(mut child) => {
            std::thread::spawn(move || {
                let _ = child.wait();
            });
        }
        Err(e) => eprintln!(
            "warning: could not open a browser ({e}); the site is at {}",
            page.display()
        ),
    }
}

#[cfg(target_os = "macos")]
fn opener() -> Command {
    Command::new("open")
}

#[cfg(windows)]
fn opener() -> Command {
    let mut c = Command::new("cmd");
    c.args(["/C", "start", ""]);
    c
}

#[cfg(not(any(windows, target_os = "macos")))]
fn opener() -> Command {
    Command::new("xdg-open")
}
```

In `crates/nova-cli/src/cmd/mod.rs`, after `pub mod deps;`, add
`pub mod doc;`.

In `crates/nova-cli/src/main.rs`:
- in the module doc, the list "parse, run, build, check, test, fmt, lsp,
  new, init, fetch, update, package, publish, login and version" becomes
  "parse, run, build, check, test, fmt, doc, lsp, new, init, fetch,
  update, package, publish, login and version", and after "Phase 3.3b the
  package index (spec …)" the sentence continues ", and Phase 3.5a `nova
  doc` (spec
  `docs/superpowers/specs/2026-10-11-phase-3-5a-the-documentation-site-design.md`)."
  with the full stop moved to its end;
- in `enum Command`, after `Fmt(cmd::fmt::FmtCmd),`:

```rust
    /// Write the project's documentation site to target/doc/, with its
    /// dependencies' and std's.
    Doc(cmd::doc::DocCmd),
```

- in `main`'s `match`, after the `Command::Fmt` arm:

```rust
        Command::Doc(cmd) => cmd::doc::run(cmd),
```

- [ ] **Step 3: Run the tests**

Run: `cd /home/user/nova && cargo test -p nova-cli --test doc 2>&1 | grep -E "^test |test result|panicked" | head -20`
Expected: `test result: ok. 13 passed` (12 on Windows, which skips the
`#[cfg(unix)]` test). Ledger: "Ruling: the unix-only browser test stands
for all three systems (Task 14). Cost if wrong: a Windows `cmd /C start`
fault shows only by hand."

- [ ] **Step 4: Clippy, the full suite, and commit**

Run: `cd /home/user/nova && cargo clippy --all-targets --all-features --locked -- -D warnings 2>&1 | tail -3 && cargo test --workspace --locked 2>&1 | tee $P/full-14.txt | grep -E "FAILED|panicked" | head`
Expected: `Finished`, and no FAILED line.

Message: `feat: nova doc`, body: `nova doc [--open] [--std]` writes the
project's site, its dependencies' and std's, to target/doc/; errors stop
it, warnings and broken doc links are reported (spec 3.5a §4).

---
### The rules for std's docs (Tasks 15 to 19)

Every std task follows them (spec §12.1):
- **Each module gets `//!` docs** at the very top of its file, before its
  maintainer comments: a summary sentence, then a short paragraph on what
  the module holds and how its parts fit. The existing `//` header stays
  below them.
- **Each documented item and member gets `///` docs** (spec §5.2): every
  `pub` item; every `pub` method of an inherent impl on a public type or a
  primitive; every method and associated type of a `pub` trait; every
  variant of a `pub` sum type; every `pub` field of a `pub` record.
  - The first sentence is a summary in the present tense, third person:
    "Returns the number of elements.", "Creates an empty map.". It stands
    alone, since lists and search show it by itself.
  - Then, only where it matters: what it returns on failure (each `Err`
    variant it can produce), when it panics, what it does at an edge
    (empty input, saturation, a closed channel), and whether it is
    `async`.
  - Inline code in backticks. No code blocks: examples are 3.5b's, which
    tests them (spec decision 24).
  - Written for a user of the module, from the code and its tests: no
    ADR numbers, runtime function names, measurements or history. Where a
    maintainer comment holds a fact a user needs (a clamp, a wrap, a
    limit), say the fact plainly.
- **Placement:** the `///` lines go directly above the item, below any
  maintainer `//` comment, with no line between them and the item, as
  hover reads them (spec §2). A variant or field documented on a line of
  its own may need the declaration broken over lines: write it in the
  layout `nova fmt` keeps (one variant or field per line).
- **Each std task, for each of its modules,** adds the module's name to
  `DONE` in `crates/nova-doc/tests/std_docs.rs`, runs the test to see it
  fail with that module's list, writes the docs, and runs the checks of
  its last step.
- **Facts are checked, not assumed.** Where a sentence states behaviour
  (rounding, an error kind, a limit), the task finds the code or the test
  that shows it. Where nothing does, it writes less, or ledgers the
  question for the final review.

---

### Task 15: The completeness check, and std/core

**Files:**
- Create: `crates/nova-doc/tests/common/mod.rs`, `crates/nova-doc/tests/std_docs.rs`
- Modify: `std/core/lib.nova`

**Interfaces:**
- Produces (test-only, `tests/common/mod.rs`): `PRIMITIVES`, `parse`,
  `module_docs`, `public_types`, `needing_docs`, `Source { page, package,
  file, all }`, `std_sources`, `anchors`, `missing`. Task 21's gate tests
  use the last four.

- [ ] **Step 1: The shared reading code**

Create `crates/nova-doc/tests/common/mod.rs`:

```rust
//! What a site must hold, read from the sources by the parser alone, not
//! through nova-doc's model (spec
//! `docs/superpowers/specs/2026-10-11-phase-3-5a-the-documentation-site-design.md`
//! §12.2, §16.4; plan decision 17).

// Each test file uses a part of this module.
#![allow(dead_code)]

use std::collections::{BTreeSet, HashMap};
use std::path::Path;

use nova_ast::item::{ExternItem, TraitItem, TypeDef};
use nova_ast::{File, Item, Type, Visibility};
use nova_diagnostics::{FileId, Spanned};
use nova_lexer::Token;

/// The primitive types, each with a page.
pub const PRIMITIVES: [&str; 7] = ["Int", "Float", "Bool", "Char", "String", "Bytes", "Future"];

/// `text`, parsed; it must lex and parse cleanly.
pub fn parse(text: &str) -> File {
    let (tokens, errors) = nova_lexer::lex(text, FileId::DUMMY);
    assert!(errors.is_empty(), "{errors:?}");
    let (file, errors) = nova_parser::parse(&tokens, FileId::DUMMY);
    assert!(errors.is_empty(), "{errors:?}");
    file.expect("the file parses")
}

/// The `//!` lines before `text`'s first token, without their `//!`.
pub fn module_docs(text: &str) -> String {
    let (tokens, comments, _) = nova_lexer::lex_with_comments(text, FileId::DUMMY);
    let first = tokens
        .iter()
        .find(|t| t.value != Token::Eof)
        .map_or(text.len(), |t| t.span.start as usize);
    comments
        .iter()
        .filter(|c| c.span.end as usize <= first)
        .filter_map(|c| text[c.span.start as usize..c.span.end as usize].strip_prefix("//!"))
        .collect::<Vec<_>>()
        .join("\n")
}

/// The last segment of a path type.
fn type_name(ty: &Type) -> Option<&str> {
    match ty {
        Type::Path { path, .. } => path.segments.last().map(|s| s.value.as_str()),
        _ => None,
    }
}

/// The primitive types, and every `pub` record and sum type in `sources`.
pub fn public_types(sources: &[Source]) -> BTreeSet<String> {
    let mut out: BTreeSet<String> = PRIMITIVES.iter().map(|p| p.to_string()).collect();
    for s in sources {
        for item in &s.file.items {
            match &item.value {
                Item::Record(r) if r.vis == Visibility::Pub => {
                    out.insert(r.name.value.clone());
                }
                Item::Type(t) if t.vis == Visibility::Pub => {
                    out.insert(t.name.value.clone());
                }
                _ => {}
            }
        }
    }
    out
}

/// Each thing in `file` that must have `///` docs (spec §12.2): what it
/// is, and its doc lines. `public` holds the types whose inherent methods
/// count.
pub fn needing_docs<'f>(
    file: &'f File,
    public: &BTreeSet<String>,
) -> Vec<(String, &'f [Spanned<String>])> {
    let mut out: Vec<(String, &'f [Spanned<String>])> = Vec::new();
    for item in &file.items {
        match &item.value {
            Item::Function(f) if f.vis == Visibility::Pub => {
                out.push((format!("fn {}", f.name.value), &f.docs))
            }
            Item::Const(c) if c.vis == Visibility::Pub => {
                out.push((format!("const {}", c.name.value), &c.docs))
            }
            Item::Record(r) if r.vis == Visibility::Pub => {
                out.push((format!("record {}", r.name.value), &r.docs));
                for f in r.fields.iter().filter(|f| f.vis == Visibility::Pub) {
                    out.push((format!("field {}.{}", r.name.value, f.name.value), &f.docs));
                }
            }
            Item::Type(t) if t.vis == Visibility::Pub => {
                out.push((format!("type {}", t.name.value), &t.docs));
                if let TypeDef::Sum(variants) = &t.def {
                    for v in variants {
                        out.push((format!("variant {}::{}", t.name.value, v.name.value), &v.docs));
                    }
                }
            }
            Item::Trait(t) if t.vis == Visibility::Pub => {
                out.push((format!("trait {}", t.name.value), &t.docs));
                for ti in &t.items {
                    match ti {
                        TraitItem::Required(s) => out
                            .push((format!("method {}::{}", t.name.value, s.name.value), &s.docs)),
                        TraitItem::Provided(f) => out
                            .push((format!("method {}::{}", t.name.value, f.name.value), &f.docs)),
                        TraitItem::AssocType { docs, name, .. } => {
                            out.push((format!("type {}::{}", t.name.value, name.value), docs))
                        }
                    }
                }
            }
            Item::Impl(imp) if imp.trait_.is_none() => {
                let Some(on) = type_name(&imp.ty.value) else {
                    continue;
                };
                if !public.contains(on) {
                    continue;
                }
                for f in imp.functions.iter().filter(|f| f.vis == Visibility::Pub) {
                    out.push((format!("method {on}.{}", f.name.value), &f.docs));
                }
            }
            _ => {}
        }
    }
    out
}

/// One module of a site: its page, its package, its parsed file, and
/// whether it documents every item, as a program's does.
pub struct Source {
    pub page: String,
    pub package: String,
    pub file: File,
    pub all: bool,
}

/// std's 17 modules.
pub fn std_sources() -> Vec<Source> {
    nova_std::ALL
        .iter()
        .map(|(name, text)| Source {
            page: format!("std/{name}/index.html"),
            package: "std".to_string(),
            file: parse(text),
            all: false,
        })
        .collect()
}

/// Every (page, id) a site must hold for `sources`: each documented item
/// and member (spec §5.2, §6.2).
pub fn anchors(sources: &[Source]) -> BTreeSet<(String, String)> {
    let mut out = BTreeSet::new();
    // Where each documented record and sum type is: (package, name, page,
    // prefix).
    let mut types: Vec<(&str, &str, &str, &str)> = Vec::new();
    for s in sources {
        let shown = |v: Visibility| s.all || v == Visibility::Pub;
        for item in &s.file.items {
            match &item.value {
                Item::Function(f) if shown(f.vis) => {
                    out.insert((s.page.clone(), format!("fn.{}", f.name.value)));
                }
                Item::Const(c) if shown(c.vis) => {
                    out.insert((s.page.clone(), format!("const.{}", c.name.value)));
                }
                Item::Extern(block) if s.all => {
                    for e in &block.items {
                        let ExternItem::Fn(sig) = e;
                        out.insert((s.page.clone(), format!("fn.{}", sig.name.value)));
                    }
                }
                Item::Record(r) if shown(r.vis) => {
                    let a = format!("record.{}", r.name.value);
                    for f in r.fields.iter().filter(|f| shown(f.vis)) {
                        out.insert((s.page.clone(), format!("{a}.field.{}", f.name.value)));
                    }
                    out.insert((s.page.clone(), a));
                    types.push((s.package.as_str(), r.name.value.as_str(), s.page.as_str(), "record"));
                }
                Item::Type(t) if shown(t.vis) => {
                    if let TypeDef::Sum(variants) = &t.def {
                        let a = format!("type.{}", t.name.value);
                        for v in variants {
                            out.insert((s.page.clone(), format!("{a}.variant.{}", v.name.value)));
                        }
                        out.insert((s.page.clone(), a));
                        types.push((s.package.as_str(), t.name.value.as_str(), s.page.as_str(), "type"));
                    }
                }
                Item::Trait(t) if shown(t.vis) => {
                    let a = format!("trait.{}", t.name.value);
                    for ti in &t.items {
                        let id = match ti {
                            TraitItem::Required(sig) => format!("{a}.method.{}", sig.name.value),
                            TraitItem::Provided(f) => format!("{a}.method.{}", f.name.value),
                            TraitItem::AssocType { name, .. } => format!("{a}.type.{}", name.value),
                        };
                        out.insert((s.page.clone(), id));
                    }
                    out.insert((s.page.clone(), a));
                }
                _ => {}
            }
        }
    }
    for s in sources {
        for item in &s.file.items {
            let Item::Impl(imp) = &item.value else {
                continue;
            };
            if imp.trait_.is_some() {
                continue;
            }
            let Some(on) = type_name(&imp.ty.value) else {
                continue;
            };
            for f in imp.functions.iter().filter(|f| s.all || f.vis == Visibility::Pub) {
                if s.package == "std" && PRIMITIVES.contains(&on) {
                    out.insert((
                        format!("std/primitive.{on}.html"),
                        format!("method.{}", f.name.value),
                    ));
                } else if let Some((_, _, page, prefix)) = types
                    .iter()
                    .find(|(p, n, _, _)| *p == s.package && *n == on)
                {
                    out.insert((
                        page.to_string(),
                        format!("{prefix}.{on}.method.{}", f.name.value),
                    ));
                }
            }
        }
    }
    out
}

/// The entries of `expected` the site at `dir` lacks, as `page#id`.
pub fn missing(dir: &Path, expected: &BTreeSet<(String, String)>) -> Vec<String> {
    let mut pages: HashMap<&str, String> = HashMap::new();
    let mut out = Vec::new();
    for (page, id) in expected {
        let text = pages
            .entry(page.as_str())
            .or_insert_with(|| std::fs::read_to_string(dir.join(page)).unwrap_or_default());
        if !text.contains(&format!(" id=\"{id}\"")) {
            out.push(format!("{page}#{id}"));
        }
    }
    out
}
```

- [ ] **Step 2: The completeness check, failing for std/core**

Create `crates/nova-doc/tests/std_docs.rs`:

```rust
//! std's docs are complete (spec
//! `docs/superpowers/specs/2026-10-11-phase-3-5a-the-documentation-site-design.md`
//! §12.2; plan decision 18).

mod common;

/// The modules whose docs are written. Each std task adds its own; the
/// last checks every module.
const DONE: &[&str] = &["core"];

#[test]
fn std_is_documented() {
    let public = common::public_types(&common::std_sources());
    let mut missing = Vec::new();
    for (name, text) in nova_std::ALL {
        if !DONE.contains(&name) {
            continue;
        }
        if common::module_docs(text).trim().is_empty() {
            missing.push(format!("std/{name}: no `//!` docs"));
        }
        for (what, docs) in common::needing_docs(&common::parse(text), &public) {
            if docs.iter().all(|l| l.value.trim().is_empty()) {
                missing.push(format!("std/{name}: {what}"));
            }
        }
    }
    assert!(
        missing.is_empty(),
        "{} undocumented:\n{}",
        missing.len(),
        missing.join("\n")
    );
}
```

Run: `cd /home/user/nova && cargo test -p nova-doc --test std_docs 2>&1 | tail -60`
Expected: FAIL, listing std/core's module docs and every documented item
and member of `std/core` (about 50). Keep the list in `$P/core-todo.txt`.

- [ ] **Step 3: std/core's docs**

Write the docs the list names, by the rules above. `std/core` holds
`Option`, `Result`, `Ordering`, the traits `Display`, `Debug`, `Eq`,
`Ord`, `Clone`, `Default`, `Hash` and `Iterator`, their impls for the
primitives, and the iterator adaptors. Its inherent impls on `Option` and
`Result` document their `pub` methods.

- [ ] **Step 4: The checks, and commit**

Run: `cd /home/user/nova && cargo test -p nova-doc --test std_docs 2>&1 | tail -3 && cargo run -q -p nova-cli -- fmt --check std && cargo test -p nova-std 2>&1 | grep -E "^test result|FAILED" && cargo test -p nova-driver --test index_complete 2>&1 | grep -E "^test result|FAILED"`
Expected: `test result: ok. 1 passed`; nothing from `nova fmt --check`;
the other test results `ok`.

Message: `docs: std/core's docs, and the check that keeps std documented`,
body: spec 3.5a §12. The check covers the modules whose docs are written;
the last std commit makes it cover all 17.

---

### Task 16: std's docs: collections, strings, bytes, fmt

**Files:**
- Modify: `std/collections/lib.nova`, `std/strings/lib.nova`,
  `std/bytes/lib.nova`, `std/fmt/lib.nova`
- Modify: `crates/nova-doc/tests/std_docs.rs` (`DONE`)

The modules hold, as `pub` items and inherent methods: `collections` 4
items and 23 methods (`Vec`, `Map`, `Set` and their iterator);
`strings` 18 methods on `String`; `bytes` 2 items and 8 methods; `fmt` 4
methods on `Int`, `String` and `Float`. Methods on `String`, `Int`,
`Float` and `Bytes` show on their primitive pages.

- [ ] **Step 1:** `DONE` becomes `&["core", "collections", "strings",
  "bytes", "fmt"]`. Run Task 15 Step 2's command. Expected: FAIL, listing
  the four modules.
- [ ] **Step 2:** Write the docs, by the rules.
- [ ] **Step 3:** Run Task 15 Step 4's command. Expected: as there.
- [ ] **Step 4:** Commit: `docs: std's collections, strings, bytes and
  fmt`.

---

### Task 17: std's docs: io, fs, net, process

**Files:**
- Modify: `std/io/lib.nova`, `std/fs/lib.nova`, `std/net/lib.nova`,
  `std/process/lib.nova`
- Modify: `crates/nova-doc/tests/std_docs.rs` (`DONE`)

`io` has 12 items and the `Read` and `Write` traits' methods; `fs` 15
items and 4 methods; `net` 4 items and 5 methods; `process` 2 items.
Each `async` function says so, and each function that returns a
`Result` names the error it returns and when.

- [ ] **Step 1:** add `"io", "fs", "net", "process"` to `DONE`; run; expect
  FAIL listing them.
- [ ] **Step 2:** Write the docs.
- [ ] **Step 3:** Run Task 15 Step 4's command.
- [ ] **Step 4:** Commit: `docs: std's io, fs, net and process`.

---

### Task 18: std's docs: task, sync, time, log

**Files:**
- Modify: `std/task/lib.nova`, `std/sync/lib.nova`, `std/time/lib.nova`,
  `std/log/lib.nova`
- Modify: `crates/nova-doc/tests/std_docs.rs` (`DONE`)

`task` has 4 items and a method; `sync` 6 items and 13 methods; `time` 6
items and 10 methods, whose constructors saturate (say so, and at what);
`log` 5 items and 9 methods.

- [ ] **Step 1:** add `"task", "sync", "time", "log"` to `DONE`; run;
  expect FAIL listing them.
- [ ] **Step 2:** Write the docs.
- [ ] **Step 3:** Run Task 15 Step 4's command.
- [ ] **Step 4:** Commit: `docs: std's task, sync, time and log`.

---

### Task 19: std's docs: json, http, crypto, test; every module checked

**Files:**
- Modify: `std/json/lib.nova`, `std/http/lib.nova`, `std/crypto/lib.nova`,
  `std/test/lib.nova`
- Modify: `crates/nova-doc/tests/std_docs.rs`

`json` has 6 items and the `ToJson` and `FromJson` traits' methods;
`http` 12 items, 9 methods and 4 `pub` fields; `crypto` 8 items; `test`
3 functions, available only inside `@test` functions (its `//!` docs say
so).

- [ ] **Step 1:** add `"json", "http", "crypto", "test"` to `DONE`; run;
  expect FAIL listing them.
- [ ] **Step 2:** Write the docs.
- [ ] **Step 3: Every module checked.** In `std_docs.rs`, delete `DONE`
  and its doc comment, and the `if !DONE.contains(&name) { continue; }`
  lines. The test now checks every module.
- [ ] **Step 4:** Run Task 15 Step 4's command, then the full suite:
  `cd /home/user/nova && cargo test --workspace --locked 2>&1 | tee $P/full-19.txt | grep -E "FAILED|panicked" | head`.
  Expected: no output from the second. A hover or completion test that
  compared a std item's text exactly may now see its docs: if one fails,
  check that the docs are the only difference, update its expected text,
  and ledger it.
- [ ] **Step 5: The mutant.** Delete one `///` line above a `pub fn` in
  `std/crypto/lib.nova`, run `cargo test -p nova-doc --test std_docs`,
  expect FAIL naming it, and undo with `git checkout -- std/crypto/lib.nova`.
- [ ] **Step 6:** Commit: `docs: std's json, http, crypto and test; every
  module checked`.

---
### Task 20: The example, `examples/05-geometry`

**Files:**
- Create: `examples/05-geometry/{nova.toml,README.md,src/main.nova}`,
  `examples/05-geometry/shapes/{nova.toml,src/lib.nova,src/units.nova}`
- Test: `crates/nova-cli/tests/run_tests.rs`

The files below were run with `nova run` and checked with `nova check`
and `nova fmt --check` on 2026-10-11, while this plan was written.

- [ ] **Step 1: Write the failing test**

Append to `crates/nova-cli/tests/run_tests.rs`:

```rust
/// `examples/05-geometry` (spec
/// `docs/superpowers/specs/2026-10-11-phase-3-5a-the-documentation-site-design.md`
/// §13): a program and the path library beside it.
#[test]
fn example_05_geometry_runs() {
    let home = std::env::temp_dir().join("nova-run-05-geometry-home");
    let _ = std::fs::remove_dir_all(&home);
    nova()
        .current_dir(repo_root().join("examples/05-geometry"))
        .env("NOVA_HOME", &home)
        .arg("run")
        .assert()
        .success()
        .stdout(
            "area 3.141592653589793, perimeter 6.283185307179586\n\
             area 6, perimeter 10\n\
             total area 9.141592653589793\n",
        );
}
```

Run: `cd /home/user/nova && cargo test -p nova-cli --test run_tests example_05 2>&1 | tail -5`
Expected: FAIL: the directory does not exist.

- [ ] **Step 2: The files**

`examples/05-geometry/nova.toml`:

```toml
[package]
name = "geometry"
version = "0.1.0"
edition = "2026"
description = "Measures a few shapes with the shapes library beside it"

[dependencies]
shapes = { path = "shapes" }
```

`examples/05-geometry/src/main.nova`:

```nova
//! Measures a few shapes with the `shapes` library beside this program.

import shapes

/// The shapes this program measures.
fn sample() -> [Shape] { [circle(1.0), rectangle(2.0, 3.0)] }

/// One line about `shape`: its area and its perimeter.
fn describe(shape: Shape) -> String { "area ${shape.area()}, perimeter ${shape.perimeter().value}" }

fn main() {
    let shapes = sample()
    for i in 0..shapes.len() { println(describe(shapes[i])) }
    println("total area ${total_area(shapes)}")
}
```

`examples/05-geometry/shapes/nova.toml`:

```toml
[package]
name = "shapes"
version = "0.1.0"
edition = "2026"
description = "Shapes on a plane, and what can be measured about them"
```

`examples/05-geometry/shapes/src/lib.nova`:

```nova
//! Shapes on a plane, and what can be measured about them.
//!
//! Every size is a `Length` from this package's `units` module, so a
//! radius cannot be mixed up with a bare number.

import units

/// The ratio of a circle's circumference to its diameter.
pub const PI: Float = 3.141592653589793

/// A rectangle, by the lengths of its sides.
pub record Rect {
    /// Its extent along the horizontal axis.
    pub width: Length,
    /// Its extent along the vertical axis.
    pub height: Length,
}

/// A shape, by its kind and its size.
pub type Shape =
    /// A circle, by its radius.
    | Circle(Length)
    /// A rectangle.
    | Rectangle(Rect)

/// What can be measured about a shape.
pub trait Measure {
    /// The area it covers, in square units.
    fn area(self) -> Float

    /// The length of its outline.
    fn perimeter(self) -> Length
}

impl Measure for Rect {
    fn area(self) -> Float { self.width.value * self.height.value }

    fn perimeter(self) -> Length { self.width.plus(self.height).times(2.0) }
}

impl Measure for Shape {
    fn area(self) -> Float {
        match self {
            Circle(r) => PI * r.value * r.value
            Rectangle(rect) => rect.area()
        }
    }

    fn perimeter(self) -> Length {
        match self {
            Circle(r) => r.times(2.0 * PI)
            Rectangle(rect) => rect.perimeter()
        }
    }
}

/// A circle of the given radius.
pub fn circle(radius: Float) -> Shape { Circle(Length::of(radius)) }

/// A rectangle of the given width and height.
pub fn rectangle(width: Float, height: Float) -> Shape {
    Rectangle(Rect { width: Length::of(width), height: Length::of(height) })
}

/// The area `shapes` cover together, counting any overlap twice.
pub fn total_area(shapes: [Shape]) -> Float {
    let mut total = 0.0
    for i in 0..shapes.len() { total = total + shapes[i].area() }
    total
}
```

`examples/05-geometry/shapes/src/units.nova`:

```nova
//! Lengths, kept apart from bare numbers.

/// A length on the plane, in whatever unit the caller measures in.
pub record Length {
    /// How long it is.
    pub value: Float,
}

impl Length {
    /// A length of `value` units.
    pub fn of(value: Float) -> Length { Length { value: value } }

    /// This length and `other` laid end to end.
    pub fn plus(self, other: Length) -> Length { Length::of(self.value + other.value) }

    /// This length scaled by `factor`.
    pub fn times(self, factor: Float) -> Length { Length::of(self.value * factor) }
}
```

`examples/05-geometry/README.md`, in `60-EXAMPLES.md` §9's template:

~~~markdown
# 05-geometry

Measures a few shapes with a library beside it, and documents both with
`nova doc`.

## What this demonstrates

- **A path dependency:** `shapes = { path = "shapes" }` in `nova.toml`,
  and `import shapes` in `src/main.nova`.
- **A library of two modules:** `shapes/src/lib.nova` is its API, and it
  uses its own `units.nova`.
- **Records, a sum type, a trait implemented for both, functions and a
  constant.**
- **Docs:** `///` on every item and member and `//!` on every module,
  which `nova doc` renders.

## Run it

```bash
cd examples/05-geometry
nova run
nova doc --open
```

## Expected output

```
area 3.141592653589793, perimeter 6.283185307179586
area 6, perimeter 10
total area 9.141592653589793
```

`nova doc` prints `target/doc/geometry/index.html`, the program's page,
which links to the library's pages and to std's.

## Notes

- A dependent can name only the items of a library's `lib.nova`, so the
  program never names `Length`, though it reads a `Length`'s `value`
  (`docs/adr/0030-package-modules.md`).
- A `Float` with no fraction prints without one: `6`, not `6.0`.
- Phase 3.5a's gate documents this example
  (`docs/superpowers/specs/2026-10-11-phase-3-5a-the-documentation-site-design.md`
  §13, §17).
~~~

- [ ] **Step 3: Run the test, and the checks the example joins**

Run: `cd /home/user/nova && cargo test -p nova-cli --test run_tests example_05 2>&1 | tail -3 && cargo run -q -p nova-cli -- fmt --check examples && cargo test -p nova-driver --test index_complete --test broken 2>&1 | grep -E "^test result|FAILED"`
Expected: `ok`; nothing from `nova fmt --check`; both `ok`. Then `git status
--short examples/` shows only the new files: no `target/` or `nova.lock`
left behind by the run.

- [ ] **Step 4: Commit**

Message: `feat: examples/05-geometry, an app and a path library`, body:
spec 3.5a §13; the gate documents it. `60-EXAMPLES.md` reserves `06-` to
`15-`, so it takes `05-`, as `03-producer-consumer` shares `03`.

---

### Task 21: The gate

**Files:**
- Create: `crates/nova-doc/tests/gate.rs`, `.github/scripts/doc-gate.sh`
- Modify: `.github/workflows/ci.yml` (the `install` job)

- [ ] **Step 1: The gate tests**

Create `crates/nova-doc/tests/gate.rs`:

```rust
//! The phase plan's 3.5 gate, for the site (spec
//! `docs/superpowers/specs/2026-10-11-phase-3-5a-the-documentation-site-design.md`
//! §16.4, §17): for std, and for an example that uses a dependency, every
//! public item has an anchor and a link check finds no broken link.

mod common;

use std::path::{Path, PathBuf};

use common::Source;

/// A fresh, empty directory with a fixed name, so each run replaces the
/// last.
fn fresh(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("nova-doc-gate-{name}"));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn repo() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// `from` copied to `to`, without any `target/`.
fn copy(from: &Path, to: &Path) {
    std::fs::create_dir_all(to).unwrap();
    for entry in std::fs::read_dir(from).unwrap() {
        let entry = entry.unwrap();
        let name = entry.file_name();
        if name == "target" {
            continue;
        }
        let path = entry.path();
        if path.is_dir() {
            copy(&path, &to.join(&name));
        } else {
            std::fs::copy(&path, to.join(&name)).unwrap();
        }
    }
}

#[test]
fn std_has_every_anchor_and_no_broken_link() {
    let dir = fresh("std").join("doc");
    let Ok(documented) = nova_doc::document_std() else {
        panic!("std is not documented")
    };
    nova_doc::write_site(&documented.site, &dir).expect("the site is written");
    let missing = common::missing(&dir, &common::anchors(&common::std_sources()));
    assert!(
        missing.is_empty(),
        "{} missing:\n{}",
        missing.len(),
        missing.join("\n")
    );
    assert_eq!(nova_doc::check_links(&dir), Vec::new());
    for (name, _) in nova_std::ALL {
        assert!(dir.join("std").join(name).join("index.html").is_file(), "std/{name}");
    }
    for p in common::PRIMITIVES {
        assert!(dir.join("std").join(format!("primitive.{p}.html")).is_file(), "{p}");
    }
}

#[test]
fn the_example_has_every_anchor_and_no_broken_link() {
    let example = fresh("example").join("geometry");
    copy(&repo().join("examples").join("05-geometry"), &example);
    let documented = match nova_doc::document_package(&example) {
        Ok(d) => d,
        Err(nova_doc::Failure::Diagnostics { db, diagnostics }) => panic!(
            "{}",
            nova_diagnostics::render::render_to_string(&db, &diagnostics)
        ),
        Err(nova_doc::Failure::Other(why)) => panic!("{why}"),
    };
    assert!(documented.warnings.is_empty());
    let dir = example.join("target").join("doc");
    nova_doc::write_site(&documented.site, &dir).expect("the site is written");
    let text = |path: &str| std::fs::read_to_string(example.join(path)).unwrap();
    let mut sources = vec![
        Source {
            page: "geometry/index.html".to_string(),
            package: "geometry".to_string(),
            file: common::parse(&text("src/main.nova")),
            all: true,
        },
        Source {
            page: "shapes/index.html".to_string(),
            package: "shapes".to_string(),
            file: common::parse(&text("shapes/src/lib.nova")),
            all: false,
        },
        Source {
            page: "shapes/units/index.html".to_string(),
            package: "shapes".to_string(),
            file: common::parse(&text("shapes/src/units.nova")),
            all: false,
        },
    ];
    sources.extend(common::std_sources());
    let missing = common::missing(&dir, &common::anchors(&sources));
    assert!(
        missing.is_empty(),
        "{} missing:\n{}",
        missing.len(),
        missing.join("\n")
    );
    assert_eq!(nova_doc::check_links(&dir), Vec::new());
}
```

Run: `cd /home/user/nova && cargo test -p nova-doc --test gate 2>&1 | tail -20`
Expected: `test result: ok. 2 passed`. These tests are written after the
code on purpose (*guard*): they are the gate. A missing anchor or a broken
link is a model bug; fix the model, not the test, and ledger it.

- [ ] **Step 2: The mutants**

Each must fail the named test; undo each with `git checkout -- <file>`:
1. In `model.rs`'s `sum`, make every variant's anchor `format!("{anchor}.v")`
   → `std_has_every_anchor_and_no_broken_link`.
2. In `links.rs`'s `resolves`, return `true` whenever the target page
   exists, ignoring the fragment → `a_missing_file_and_a_missing_fragment_are_broken`.

- [ ] **Step 3: The installed nova's half**

Create `.github/scripts/doc-gate.sh`:

```bash
#!/usr/bin/env bash
# The Phase 3.5a gate's installed half (spec
# docs/superpowers/specs/2026-10-11-phase-3-5a-the-documentation-site-design.md
# §17): the installed nova documents a copy of examples/05-geometry, an app
# with a path library, with no warning, and writes the pages the gate needs.
# CI's `install` job runs it, from the repository's root.
#
# Usage: doc-gate.sh NOVA WORKDIR
#   NOVA     the nova executable to check
#   WORKDIR  where to copy the example; it must not exist yet
set -euo pipefail

if [ $# -ne 2 ]; then
  echo "usage: doc-gate.sh NOVA WORKDIR" >&2
  exit 2
fi
nova=$1
work=$2

mkdir "$work"
cp -R examples/05-geometry "$work/geometry"
cd "$work/geometry"

start=$("$nova" doc 2> doc.err)
if [ -s doc.err ]; then
  cat doc.err >&2
  echo "doc-gate: nova doc wrote to stderr" >&2
  exit 1
fi
start=${start//\\//}
if [ "$start" != "target/doc/geometry/index.html" ]; then
  echo "doc-gate: nova doc printed '$start'" >&2
  exit 1
fi
for page in index.html geometry/index.html shapes/index.html shapes/units/index.html \
    std/index.html std/core/index.html std/primitive.String.html; do
  if [ ! -f "target/doc/$page" ]; then
    echo "doc-gate: no target/doc/$page" >&2
    exit 1
  fi
done
echo "doc-gate: passed"
```

Run: `cd /home/user/nova && chmod +x .github/scripts/doc-gate.sh && cargo build -q -p nova-cli && rm -rf $P/doc-gate && bash .github/scripts/doc-gate.sh "$PWD/target/debug/nova" $P/doc-gate`
Expected: `doc-gate: passed`.

In `.github/workflows/ci.yml`'s `install` job, after the step "The
installed nova builds a program against a path library":

```yaml
      # The Phase 3.5a gate (spec
      # docs/superpowers/specs/2026-10-11-phase-3-5a-the-documentation-site-design.md
      # §17): its installed half; the anchors and links are the Test job's.
      - name: The installed nova documents an example that uses a dependency
        shell: bash
        run: bash .github/scripts/doc-gate.sh "$RUNNER_TEMP/nova/bin/nova" "$RUNNER_TEMP/doc"
```

- [ ] **Step 4: Commit**

Message: `test: the 3.5 gate, for std and examples/05-geometry`, body:
every public item has its anchor and no link is broken, checked in the
Test job on three systems; the installed nova documents the example in
the install job (spec 3.5a §17).

---

### Task 22: std's site on each release

**Files:**
- Modify: `.github/workflows/release.yml`

- [ ] **Step 1: The job**

In `release.yml`, change the header comment's "and publish a GitHub
release with the archives, the `.vsix` and SHA256SUMS" to "and publish a
GitHub release with the archives, the `.vsix`, std's documentation site
and SHA256SUMS".

After the `vsix` job, add:

```yaml
  # std's documentation site, for the release (spec
  # docs/superpowers/specs/2026-10-11-phase-3-5a-the-documentation-site-design.md
  # §14). Its artifact is named `archive-std-docs` so that `prepare`'s
  # `archive-*` download takes it.
  std-docs:
    name: Write std's documentation site
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - uses: Swatinem/rust-cache@v2
      - name: Build nova
        run: cargo build --locked -p nova-cli
      - name: Write and pack the site
        shell: bash
        run: |
          set -euo pipefail
          version=$(tr -d '\r' < crates/nova-cli/Cargo.toml | sed -n 's/^version = "\(.*\)"$/\1/p' | head -n 1)
          site="$RUNNER_TEMP/std-docs"
          mkdir "$site"
          (cd "$site" && "$GITHUB_WORKSPACE/target/debug/nova" doc --std)
          mkdir -p dist
          tar -C "$site/target" -czf "dist/nova-std-docs-$version.tar.gz" doc
      - uses: actions/upload-artifact@v4
        with:
          name: archive-std-docs
          path: dist/nova-std-docs-*.tar.gz
          if-no-files-found: error
```

In the `prepare` job, `needs: [build, vsix]` becomes
`needs: [build, vsix, std-docs]`, and in its script replace:

```bash
          # An archive is nova-<version>-<target>; the extension is
          # nova-vscode-<version>.vsix.
          archives=$(ls release/dist | grep -c -E '^nova-[0-9]' || true)
          vsix=$(ls release/dist | grep -c -E '^nova-vscode-.+\.vsix$' || true)
          if [ "$archives" -ne 4 ] || [ "$vsix" -ne 1 ]; then
            echo "::error::expected 4 archives and 1 .vsix, found $archives and $vsix"
```

with:

```bash
          # An archive is nova-<version>-<target>; the extension is
          # nova-vscode-<version>.vsix; std's site is
          # nova-std-docs-<version>.tar.gz.
          archives=$(ls release/dist | grep -c -E '^nova-[0-9]' || true)
          vsix=$(ls release/dist | grep -c -E '^nova-vscode-.+\.vsix$' || true)
          docs=$(ls release/dist | grep -c -E '^nova-std-docs-.+\.tar\.gz$' || true)
          if [ "$archives" -ne 4 ] || [ "$vsix" -ne 1 ] || [ "$docs" -ne 1 ]; then
            echo "::error::expected 4 archives, 1 .vsix and std's site, found $archives, $vsix and $docs"
```

- [ ] **Step 2: Check the step's script by hand**

Run: `cd /home/user/nova && rm -rf $P/std-docs && mkdir $P/std-docs && (cd $P/std-docs && /home/user/nova/target/debug/nova doc --std) && tar -C $P/std-docs/target -czf $P/nova-std-docs-test.tar.gz doc && tar -tzf $P/nova-std-docs-test.tar.gz | head -5`
Expected: entries under `doc/`, the marker among them.

The workflow's first run is this branch's pull request: it changes
`Cargo.lock` and `crates/nova-cli/Cargo.toml`, which trigger it (plan
decision 21). Ledger: "Ruling: the release workflow's first run with the
std-docs job is the PR's (Task 22). Cost if wrong: a fault shows in CI."

- [ ] **Step 3: Commit**

Message: `ci: each release attaches std's documentation site`, body: spec
3.5a §14.

---

### Task 23: The records

**Files:**
- Create: `docs/adr/0034-the-documentation-generator.md`
- Modify: `nova-spec/40-TOOLING.md` (§1.1, §5), `docs/phase-3-plan.md`
  (§3 decision 9, §4's 3.5 entry), `nova-spec/00-MASTER-SPEC.md` (§6),
  `agent.md` (§5), `nova-spec/60-EXAMPLES.md` (§9), `CHANGELOG.md`,
  `README.md`, `ARCHITECTURE.md`

- [ ] **Step 1: ADR 0034**

Create `docs/adr/0034-the-documentation-generator.md`:

```markdown
# ADR 0034 — The documentation generator

## Status

Accepted, 2026-10-11 (Phase 3.5a, branch `claude/loving-archimedes-frtf5q`;
spec `docs/superpowers/specs/2026-10-11-phase-3-5a-the-documentation-site-design.md`).

## Context

`nova-spec/40-TOOLING.md` §5 asks for `nova doc`: `///` docs as Markdown,
a static site under `target/doc/`, a search index, light and dark themes,
and `nova test --doc`. The phase plan's 3.5 entry adds "a page per module,
item signatures from the type checker", and a gate: for std and for an
example that uses a dependency, every public item has an anchor and a
link check finds no broken link, and std's site is attached to the GitHub
release. Before 3.5, `nova-doc` was a stub, std had no `///` docs, and no
example used a dependency.

On 2026-10-11 the user split 3.5 in two: 3.5a, the site, std's docs and
the gate; 3.5b, `nova test --doc`.

## Decision

1. **What a site documents.**
   - The root package, from one analysis of one root: `src/lib.nova` when
     there is a library, else `src/main.nova`.
   - A library documents what each module exports; a package that is only
     a program documents every item and member, as `cargo doc` documents a
     binary.
   - Every dependency the program loads, by its exports.
   - std, in every site, from an analysis of std alone, so one `nova`
     writes the same std pages everywhere.
2. **Signatures are shown as written,** comments removed and dedented,
   and each name in them links to where the site documents it, through the
   index the type checker records (ADR 0032). That is "signatures from
   the type checker" in a language whose signatures carry every type.
3. **A page per module,** each item and member with an `id` such as
   `record.Point.field.x`, and pages for std's primitive types and
   builtins.
4. **Module docs are `//!` lines before a file's first token,** read from
   the lexer's comments. The compiler and the formatter treat them as
   comments.
5. **Docs are Markdown, rendered by `pulldown-cmark`** 0.13.4, pinned,
   with raw HTML escaped, so a dependency cannot put markup or a script
   into a site. Nova code blocks are coloured by the lexer.
6. **The site works from disk:** every URL is relative, search reads a
   script rather than fetching, and nothing comes from the network.
7. **`nova doc` replaces only a `target/doc/` it wrote,** which holds its
   marker, `.nova-doc`.
8. **Every run checks its links,** and reports a broken one as a warning.
9. **std's site is a release asset,** `nova-std-docs-<version>.tar.gz`.

## Alternatives

- **Printing signatures from the checker's types.** It would lose the
  author's layout, bounds and `where` clauses, and say nothing the source
  does not.
- **A page per item, as rustdoc writes.** The phase plan asks for a page
  per module, and a module page with anchors is enough for links and
  search.
- **Markdown by hand.** No new crate, but CommonMark's edge cases would
  differ. The user chose `pulldown-cmark`.
- **std's pages from each project's analysis.** They would then depend on
  which std modules a project loads; one analysis of std keeps them the
  same.
- **Linking into std at a hosted URL.** No host exists, and a site that
  carries std's pages works offline.

## Consequences

- `pulldown-cmark`, `pulldown-cmark-escape` and `unicase` are in
  `Cargo.lock`. All three build on Rust 1.78.
- `Analysis` keeps the merged AST, which `nova doc` reads.
- std's public items all have `///` docs, and a test keeps them so.
- Code blocks are rendered but not run until 3.5b.
- A package named `std` cannot be documented, since its pages would share
  std's directory.

## References

- `docs/superpowers/specs/2026-10-11-phase-3-5a-the-documentation-site-design.md`
- `docs/superpowers/plans/2026-10-11-phase-3-5a-the-documentation-site.md`
- ADR 0026 (Phase 3's scope), ADR 0030 (package modules), ADR 0032
  (navigation, and the index)
- `docs/phase-3-plan.md` §3 decision 9, and §4's 3.5 entry
```

- [ ] **Step 2: Dated notes**

Each note begins `**Amended 2026-10-11 (branch `claude/loving-archimedes-frtf5q`):**`:
- `nova-spec/40-TOOLING.md`, after §1.1's last note: `nova doc [--open]
  [--std]` exists; it needs a project, or `--std`, which writes std's site
  alone; it prints the start page's path (spec §4; ADR 0034).
- `40-TOOLING.md`, at the end of §5: 3.5a delivers the site. `///` docs
  as Markdown through `pulldown-cmark`, with raw HTML escaped; `//!` lines
  before the first token as module docs; a page per module under
  `target/doc/<package>/`, with std's in every site; signatures as
  written, each name linked through the checker's index; a search index
  read as a script; light and dark themes. `nova test --doc` is 3.5b's
  (ADR 0034).
- `docs/phase-3-plan.md` §3 decision 9: 3.5 is two sub-phases, by the
  user on 2026-10-11. `nova test --doc` is 3.5b's, and 3.5a writes docs
  for every public std item.
- `docs/phase-3-plan.md` §4's 3.5 entry: 3.5a, "The documentation site",
  is built (the spec's path, this branch, ADR 0034). Its gate is met in
  CI's Test job on three systems, and in the install job; std's site is
  attached by the next tag's release. 3.5b, `nova test --doc`, has its own
  spec next.
- `nova-spec/00-MASTER-SPEC.md` §6, after its last note: `nova-doc`
  renders Markdown with `pulldown-cmark` 0.13.4, pinned, which brings
  `pulldown-cmark-escape` and `unicase` (ADR 0034).
- `agent.md` §5, after its last note: `nova-doc` is built on
  `nova-driver`'s `analyze_program` and `pulldown-cmark`, not on
  `nova-ast` alone (ADR 0034).
- `nova-spec/60-EXAMPLES.md` §9, after its last note: a fourth example
  follows the template, `examples/05-geometry/README.md`; it takes `05-`
  because §8 reserves `06-` to `15-`.

- [ ] **Step 3: CHANGELOG, README, ARCHITECTURE**

- `CHANGELOG.md`, Unreleased:
  - under `### Added`, after the "Fixes and colour in the editor" entry:

    ```markdown
    - **`nova doc`** writes a project's documentation site to
      `target/doc/`: a page per module, each item's signature as written
      with every name linked, its `///` docs as Markdown, a search box and
      light and dark themes. The site carries the dependencies the program
      uses and std. `--std` writes std's site alone, and `--open` opens
      it. Every run checks the site's links. ADR 0034.
    - **std is documented:** every public item has `///` docs, and every
      module `//!` docs, which hover shows too.
    - **`examples/05-geometry`,** a program and the path library beside
      it.
    - **Each release attaches std's documentation site.**
    ```

  - under `### Changed`:

    ```markdown
    - **Organize imports keeps the comments that start a file** above the
      imports, even when the first import is removed or moved.
    - **The import fix steps over block comments:** after an import whose
      line opens one, and above a first item one sits on.
    - **A panic answering code actions or semantic tokens** no longer ends
      the language server.
    ```

- `README.md`, after the "### Packages" paragraph:

  ```markdown
  ### Documentation

  `nova doc` writes a project's documentation to `target/doc/`, with its
  dependencies' and std's, from the `///` comments above each item;
  `nova doc --open` opens it. Each GitHub release also attaches std's
  site.
  ```

- `ARCHITECTURE.md`'s `nova-doc` row: "Documentation generator: `nova
  doc`'s site, from an analysis of the program and of std, with
  signatures linked through the checker's index and Markdown docs
  (ADR 0034)".

- [ ] **Step 4: The sweep**

Run: `cd /home/user/nova && git grep -n -i -E "nova doc|std has no \`///\`|signatures only|3\.5'?s" -- ':!docs/superpowers' ':!CHANGELOG.md' ':!docs/adr/0034*' | head -40`
Expected: each hit either agrees with what is built, or is a dated
record of its own time. Correct the rest with a dated note, never by
rewriting history, and ledger each.

- [ ] **Step 5: Commit**

Message: `docs: records for Phase 3.5a: ADR 0034, notes, CHANGELOG`.

---

### Task 24: Final verification, mutants, and the PR body

**Files:** none in the repository; `$P/pr-body.md`.

- [ ] **Step 1: Everything, on Linux**

Run, each to a file in `$P`, reading the tails:
- `cargo fmt --all -- --check`
- `cargo clippy --locked --all-targets --all-features -- -D warnings`
- `RUSTFLAGS="-D warnings" cargo +1.78 check --locked --workspace`
- `cargo run -q -p nova-cli -- fmt --check std examples`
- `cargo test --workspace --locked --all-features --no-fail-fast 2>&1 | tee $P/full-final.txt`,
  then the counting one-liner.

Expected: all clean, and no failure. Ledger the counts beside Task 0's.

- [ ] **Step 2: The spec's mutants**

Spec §16.5's nine, each on committed work, undone with `git checkout --
<file>`, each failing its named test. Mutants 2, 6 and 8 ran in Tasks 21
and 19; run the others:
1. `model.rs`'s `url`: in the `item` closure, drop the `shown` check →
   `a_program_documents_every_item_and_its_library_only_its_exports`.
3. `markdown.rs`: push `Event::Html(h)` unchanged instead of
   `Event::Text(h)` → `raw_html_shows_as_text`.
4. `signature.rs`'s `gap`: push a line comment's characters into `plain`
   → `a_broken_signature_keeps_its_lines_less_their_indentation`.
5. `write.rs`: delete the `NotOurs` branch →
   `a_site_it_wrote_is_replaced_and_another_directory_is_refused`, and
   `a_target_doc_it_did_not_write_is_refused`.
7. `model.rs`'s `all`: return `false` →
   `a_program_documents_every_item_and_its_library_only_its_exports`.
9. `organize.rs`: restore the plain `comments_above(text, first_line)` →
   `comments_that_start_the_file_stay_at_its_top`.

Ledger each with the test that failed.

- [ ] **Step 3: The PR body**

Write `$P/pr-body.md`, with no local path and no personal name.
Sections, in order:
1. `## Phase 3.5a, "The documentation site"`: a paragraph on what a user
   gets, then the spec, the plan and ADR 0034, and that 3.5b, `nova test
   --doc`, follows.
2. `### PR #108's deferred minors`: the nine, by number, each in a line
   with its commit's subject.
3. `### What changed`: the crates and files, as this plan's Architecture
   block lists them.
4. `### Changes in behaviour`: `nova doc`; std's docs in hover; organize
   imports' header; the import fix's block comments; the guard.
5. `### Tests`: a table of Linux (local) results, `main` at `14dc6e1`
   against this branch, from Task 0 and Step 1; Windows and macOS "from
   this PR's CI". Then the new tests by file, and "each of the spec's nine
   mutants fails its named test".
6. `### Dependencies`: the three new locked packages, their versions and
   declared minimum Rust, and that the user approved `pulldown-cmark` on
   2026-10-11.
7. `### Final review`: filled in after the review.
8. `### Decisions to review`: this plan's decisions 1-22 by title, the
   spec's decisions 5-28 by title, and every ledger `Ruling:` line with
   its cost if wrong.
9. `### Deferred minors`: from the final review.
10. The attribution line the session gives for pull requests.

Run: `grep -n -i -E "/home/|/tmp/|claude-0|sakeerin" $P/pr-body.md`
Expected: no output.

There is no commit in this task. The branch is pushed, and the PR opened,
only after the final review and its fix pass, as the workflow says.
