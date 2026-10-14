# ADR 0037 — Phase 3's boundary

## Status

Accepted (2026-10-13). Branch `claude/loving-archimedes-frtf5q`
(`docs/superpowers/specs/2026-10-12-phase-3-6b-the-release-design.md` §6).

## Context

`nova-spec/00-MASTER-SPEC.md` §7 calls a phase done when:
1. its crates compile and pass CI;
2. its gate criteria are met with reproducible commands;
3. its new public surface is documented;
4. `CHANGELOG.md` is updated;
5. an ADR records each decision that deviates from the spec;
6. a `v0.{phase}.0` milestone is tagged.

Phase 3, "Tooling", makes its promises in three places:
- **the master spec's §3:** the Phase 3 list of eight numbered entries,
  cited below as "position N", and the Phase 3 gate: "External user can
  `cargo install nova-cli`, init a project, write code with autocomplete,
  format, and publish a package";
- **`docs/phase-3-plan.md`:** sub-phases 3.0 to 3.6, each with its own
  gate;
- **`nova-spec/40-TOOLING.md`:** the tooling Phase 3 builds, and more. Its
  rows below are cited as "40-TOOLING §N".

Earlier ADRs already narrowed Phase 3:
- 0026: a git-backed package index in place of position 3's registry
  server, and the list of what Phase 3 leaves out, the REPL and the
  debugger among them;
- 0027: the runtime and std inside an installed `nova`;
- 0028 to 0035: the formatter, the language server, package modules, the
  index and publishing, navigation, fixes and colour, `nova doc`, and doc
  tests;
- 0036: publishing `nova` itself.

Phase 3 was built in eleven pull requests, #102 to #112, from 2026-10-07
to 2026-10-12. `v0.3.0` was released on 2026-10-13:
- the annotated tag `v0.3.0`, on the release commit `c0db6c0`;
- the GitHub release "Nova 0.3.0": four archives, the two install
  scripts, std's documentation site and the extension's `.vsix`, with
  `SHA256SUMS`;
- the 19 crates `cargo install nova-cli` needs, on crates.io at 0.3.0;
- the package index, `Sakeerin/nova-index`, with its first package.

Before the release, the user decided (the 3.6b spec's §13, decision 1),
and approved with that spec (decisions 6 and 9):
- **two pull requests:** the release, then this close-out after the gate;
- **`v0.3.0` is Phase 3's release,** with its gate to follow, because the
  gate installs what the release published; this ADR then calls Phase 3
  complete;
- **a failure that needs a change to the repository is released as
  `0.3.1`;** nothing published is replaced.

On 2026-10-13, during the release:
- **the user decided the extension's name:** the VS Code Marketplace
  refused `nova`, so the extension becomes `vscode-nova`, in `0.3.1`,
  after the gate (below);
- **the user ran the gate's own run on their machine,** in a separate
  install, since no second machine was at hand; this ADR rules on it
  (below).

## Decision

### The inventory

"Position N" is the master spec's Phase 3 list. "3.N" is a sub-phase of
`docs/phase-3-plan.md`. "40-TOOLING §N" is a section of
`nova-spec/40-TOOLING.md`.

| Promise | Source | Status | Record |
|---|---|---|---|
| The formatter, `nova fmt` | position 1; 3.1; 40-TOOLING §2 | shipped; 3.1's gate met | ADR 0028 |
| `nova-pm`: `nova.toml`, the package graph, `nova.lock`, the resolver | position 2; 3.0; 3.3 | shipped, with the index in `nova-index` | ADRs 0030, 0031 |
| A registry server in its own repository, on Rust, Postgres and S3 | position 3; 40-TOOLING §4.4 | replaced: a git-backed index on GitHub, `Sakeerin/nova-index`, live | ADRs 0026, 0031 |
| The language server, on `tower-lsp` | position 4; 3.2; 3.4; 40-TOOLING §3 | shipped, on `lsp-server`; 3.2's and 3.4's gates met | ADRs 0029, 0032, 0033 |
| The VS Code extension | position 5; 3.2; 40-TOOLING §3.3 | shipped as each release's `.vsix`; in neither store | ADR 0036; decided here |
| `nova doc`, a static HTML site | position 6; 3.5; 40-TOOLING §5 | shipped, with doc tests; 3.5's gate met | ADRs 0034, 0035 |
| A REPL, `nova repl` | position 7; 40-TOOLING §6 | not built | ADR 0026 |
| A debugger: DWARF from both backends, DAP in VS Code | position 8 | not built: no debug information is emitted | ADR 0026 |
| The Phase 3 gate | the master spec's §3 | met on what `v0.3.0` published | "The gate's record", below |
| 3.0: an installed `nova` that needs nothing beside it; projects | 3.0 | shipped; 3.0's gate met | ADR 0027 |
| 3.3: publishing to a local index, in CI | 3.3 | shipped; this part of 3.3's gate met | ADR 0031 |
| 3.3: the same publish, by hand, to the real index | 3.3 | done, in the Phase 3 gate | ADR 0031's figures |
| 3.6: the crates on crates.io | 3.6 | shipped: 19 crates, not every workspace crate | ADR 0036 |
| 3.6: the extension in the Marketplace and Open VSX | 3.6 | not done | decided here |
| 3.6: install scripts fetching the release's binaries | 3.6; 40-TOOLING §10 | shipped, as release assets | ADR 0036 |
| 3.6's gate, "on a clean machine", "with the published extension" | 3.6 | met, on a separate install of the maintainer's machine and with the release's `.vsix` rather than a store's | decided here |
| `nova build --target wasm`, `nova bundle`, `nova dev`; the LSP rows marked Phase 4 | 40-TOOLING §1.1, §3.1 | not built | ADR 0026; Phase 4 |
| `nova lint`, `nova bench` | 40-TOOLING §1.1, §8 | not built | ADR 0026; backlog |
| `nova install`, `nova clean`, `nova owner`, `nova self update`, `nova self uninstall` | 40-TOOLING §1.1, §4.5, §10.3 | not built | ADR 0026; backlog |
| Multiple toolchains, `nova +stable build` | 40-TOOLING §10.3 | not built | Phase 6, as §10.3 says |
| The language server on `salsa` | 40-TOOLING §3.2; ADR 0025's mapping | replaced: a whole-program re-check, within its budgets | ADR 0029; decided here |
| The Zed and Neovim extensions | 40-TOOLING §3.3 | not built | ADR 0026; backlog |
| Manifest features, git dependencies, `[build]`, `[[bin]]`, `[lib]` | 40-TOOLING §4.1 | not built: they warn M0006 | the plan's decision 2; backlog |
| Several major versions of a package side by side; edition rules | 40-TOOLING §4.3 | not built: one version per name | ADRs 0026, 0031; backlog |
| Yanking | ADR 0026's consequences | not built: a published version stays | ADR 0031; backlog |
| Parallel test runs, TAP output, `--bench`, `tests/**` | 40-TOOLING §7 | not built: `nova test` runs `src/` and the top level of `tests/` | ADRs 0026, 0030; backlog |
| The CI template and its `setup-nova` action | 40-TOOLING §9 | not built | ADR 0026; backlog |
| The installer at `novalang.dev`, adding itself to PATH | 40-TOOLING §10 | shipped differently: release assets; `install.ps1` sets PATH, `install.sh` prints the line | ADR 0036 |
| Doc tests' `should_panic`, `no_run`, `compile_fail`, compared output | 3.5b's design | not built: hidden `# ` lines only | ADR 0035; backlog |
| linux-aarch64 and musl archives, signed macOS binaries, Homebrew and winget | 3.6a's design | not built | ADR 0036; backlog |
| `nova-test`, the test runner's crate | the plan's §2 item 9 | a one-line stub, `publish = false`: the runner is `nova-cli`'s `cmd/test.rs` | backlog |

**Status words:**
- *Shipped:* a `v0.3.0` user can rely on it, within the limits its record
  states; *shipped differently* when its record changed how.
- *Not built:* nothing implements it. *Done* and *not done* say the same
  of a step that is not a feature, such as a publish.
- *Replaced:* something else does its job, as its record says.
- *Met:* a gate's checks passed, as the row and its record state them.
- *Backlog*, *Phase 4*, *Phase 6:* where a deferred item goes.
- *Decided here:* a section below makes the decision.

### Deviations no earlier record decides

#### The extension's stores: decided, `vscode-nova` in `0.3.1`

The plan's 3.6 entry says the extension "is published to the VS Code
Marketplace and Open VSX", and its gate asks for "autocomplete in VSCode
with the published extension". ADR 0036 has it published by hand, from
the release's own `.vsix`. The runbook checked that no Marketplace
extension had the display name `Nova`, and none had.

The Marketplace then refused the upload: an extension's manifest `name`
must be unique across the whole store, not only within a publisher, and
another publisher's extension is named `nova`. Open VSX could have taken
`sakeerin.nova`, but one extension would then have had two IDs.

**Decision (the user, 2026-10-13):** the extension's `name` becomes
`vscode-nova`, so its ID is `sakeerin.vscode-nova` in both stores; its
display name stays `Nova`. The change is released as `0.3.1`, after the
gate, as the 3.6b spec's decision 9 asks, because the extension's version
is `nova-cli`'s. `v0.3.0`'s extension is published to neither store.

For the gate, the release's `.vsix` stands for "the published
extension": it is the file the release published, the one the stores
would have received, and it gives the same autocomplete. (This ruling is
made here, for the user's review with this ADR.)

`0.3.1` also corrects the texts that still name `sakeerin.nova`:
`crates/nova-cli/README.md`, whose copy in `nova-cli` 0.3.0 on crates.io
stays as published, and the runbook's store listings, its 6.2 install
line and its transcript template. This branch already took the store
install out of the root README, which gets it back in `0.3.1`, and the
runbook already checks the manifest `name` (its note of this date).

**Recorded 2026-10-14 (branch `claude/loving-archimedes-frtf5q`):** the
extension is in both stores, as **Nova Language Tools**
(`sakeerin.vscode-nova`), from `0.3.2`.
- **`0.3.1`** carried the new `name`:
  - the tag `v0.3.1`, its GitHub release, and the 19 crates (run
    38501806159);
  - "Release gate" passed all six jobs on it (run 38505596370).

  The Marketplace accepted the `name` but refused the display name:
  "This extension display name is taken." Two other extensions have the
  display names `nova` and `NOVA`, and the Marketplace compares display
  names without regard to case; the runbook's search for exactly `Nova`
  had missed them. `0.3.1`'s extension reached neither store: the
  Marketplace refused it, and Open VSX waited for `0.3.2`, by the user's
  decision.
- **`0.3.2`** changed the display name to `Nova Language Tools`, the
  user's decision (spec
  `docs/superpowers/specs/2026-10-13-release-0-3-1-design.md` §10).
  First a query of the Marketplace's gallery API found the display name
  free. Then, on 2026-10-14:
  - the tag `v0.3.2` and its release (run 38510244399);
  - the 19 crates (run 38511828598);
  - "Release gate", which passed all six jobs (run 38522461402).
- **The stores.**
  - The maintainer uploaded the release's `nova-vscode-0.3.2.vsix` on
    the Marketplace's manage page, which accepted it.
  - The maintainer published it to Open VSX with `ovsx`, creating the
    namespace `sakeerin`. Open VSX marks the namespace unverified until
    its owner claims it.
- **The listings.**
  - **The Marketplace's,** as VS Code shows it: Nova Language Tools by
    `sakeerin`, `sakeerin.vscode-nova` 0.3.2, the README, the categories
    Programming Languages and Formatters, and links to the repository,
    the issues and the licence.
  - **Open VSX's:** Nova Language Tools 0.3.2 by `sakeerin`, MIT OR
    Apache-2.0, and the README, under the unverified-namespace warning.
  - The CHANGELOG tabs were not opened.
- **The check.**
  - The maintainer opened a separate VS Code, with `nova` 0.3.2
    installed from crates.io. `nova version` said `nova 0.3.2`,
    `x86_64-pc-windows-msvc` and `runtime: embedded`.
  - There, the maintainer installed Nova Language Tools from the
    Marketplace (`sakeerin.vscode-nova@0.3.2`).
  - Completion after `p.` offered `x` and `y` as fields, `x` typed
    `Int`.
  - A first try showed only VS Code's word-based suggestions. That
    window had no `nova` on its PATH, so the server did not start until
    `nova.server.path` named it.

#### The gate's "clean machine": decided, a separate install on the maintainer's machine stands

The plan's 3.6 gate asks for a run "on a clean machine", and the runbook
allows a fresh virtual machine or a new user account. No second machine
was at hand, so the maintainer ran it on their own Windows machine and
account, kept apart in one directory as far as it went (the runbook's
6.2 note of this date): `nova` installed with `--root` there, its `bin`
first on `PATH`, a `NOVA_HOME` there from step 5 on, and VS Code started
with its own user data and extensions directories, holding only the Nova
extension. Steps 1 to 4 unpacked the runtime into the account's own
`~/.nova` (the gate's record, below), and Rust and `~/.cargo`, with its
registry cache, were the account's own.

**Decision (made here, for the user's review with this ADR):** that run,
with the "Release gate" workflow's six jobs, meets the gate. The jobs
cover a machine with nothing of Nova on it: each starts on a clean
runner, and installs and uses `nova` there on Linux, macOS or Windows.
The maintainer's run adds what a workflow cannot, the editor's
autocomplete and a publish to the real index, and nothing it shared with
the account changes those two. A later phase's gate runs on a fresh
machine or account, as the runbook says.

#### `salsa`: decided, not adopted

ADR 0025 mapped `salsa` to Phase 3, because the language server is where
incremental queries pay off, and the master spec's §6 lists it.
`40-TOOLING.md` §3.2 builds the server on it. ADR 0029 chose a
whole-program re-check instead, and said adopting `salsa` would be a step
decided with the user if a later sub-phase missed the 200 ms budget.
None did (ADRs 0029, 0032 and 0033 record the measurements), and no crate
depends on `salsa`.

**Decision:** ADR 0025's mapping closes with `salsa` not adopted. It comes
back only as ADR 0029 says: if a budget is missed.

### Rows whose record already explains them

These rows have no section above:
- **The REPL and the debugger:** ADR 0026 left both out, and chose not to
  amend the master spec's list; the master spec's note of this date points
  here.
- **The registry server:** ADR 0026 replaced it with the git-backed index,
  and ADR 0031 built it.
- **`nova fmt`'s paths and `--stdin`:** ADR 0028 reads position 1's
  "only `--check`" as "no style options".
- **`tower-lsp`:** ADR 0029 records why the server is on `lsp-server`.
- **The 19 crates:** ADR 0036 records why `cargo install nova-cli` needs
  19, and why the other four workspace crates are `publish = false`.
- **The installer:** ADR 0036 records the release assets in place of
  `novalang.dev`, and why `install.sh` prints the PATH line rather than
  edit a shell profile.
- **The archives and packages 3.6a left out** (linux-aarch64, musl,
  signed macOS binaries, Homebrew, winget): ADR 0036 lists them.
- **The by-hand publish of 3.3:** it ran in the Phase 3 gate, and ADR
  0031 has its figures.
- **The 40-TOOLING rows ADR 0026 left out,** and the limits ADRs 0030,
  0031 and 0035 record: each row names its record, and the backlog below
  lists them.
- **`nova-test`:** the plan's §2 item 9 listed it as a stub, and nothing
  in Phase 3 needed it; `nova test` runs in `nova-cli`.

### The gate's record

**The "Release gate" workflow,** run 38367944634 on 2026-10-13, from
08:14 to 08:19 UTC, against the published `v0.3.0`. All six jobs passed.

| Job | How `nova` was installed | Then |
|---|---|---|
| From crates.io, Linux | `cargo install nova-cli --version "=0.3.0" --locked`, 2 min 3 s | `nova 0.3.0`, `x86_64-unknown-linux-gnu`, `runtime: embedded`; nothing but `nova` installed; the gate, `nova fmt --check` on the new project, the packages gate and the doc gate passed |
| From crates.io, macOS | the same, 1 min 58 s | the same, on `aarch64-apple-darwin` |
| From crates.io, Windows | the same, 3 min 41 s | the same, on `x86_64-pc-windows-msvc` |
| From the install script, Linux | `install.sh` | the gate passed |
| From the install script, macOS | `install.sh` | the gate passed |
| From the install script, Windows | `install.ps1`, in PowerShell 7 and in Windows PowerShell 5.1 | the gate passed |

**The maintainer's run,** from the terminal's output, with paths
shortened to `~` and nothing a token could be read from:

```text
Release:        v0.3.0
Date:           2026-10-13
Machine:        Windows 10.0.26100.9448, x86_64; the maintainer's own
                machine and account, the run kept in ~/nova-gate (the
                runbook's 6.2 note of this date)
Rust:           rustc 1.95.0 (59807616e 2026-04-14)
VS Code:        1.141.0; extension sakeerin.nova 0.3.0, from the release's
                nova-vscode-0.3.0.vsix
Release gate:   https://github.com/Sakeerin/nova/actions/runs/38367944634,
                all six jobs passed

1. cargo install nova-cli --locked --root ~/nova-gate
                                       about 2 min
   nova version                        nova 0.3.0
                                       target: x86_64-pc-windows-msvc
                                       runtime: embedded
2. nova new hello; nova run            Hello, Nova!
3. completion after `p.`               x (Int), y          [screenshot]
4. nova fmt --check; nova fmt; nova fmt --check; nova run
                                       would reformat: ~\nova-gate\hello\src\main.nova
                                       (nothing)
                                       (nothing)
                                       7
5. nova login                          logged in to Sakeerin/nova-index; the token is
                                       in ~/nova-gate/nova-home\credentials.toml
   nova package                        packed shapes 0.1.0: ~\nova-gate\shapes\target\package\shapes-0.1.0.nova-pkg
                                         3 files, 1304 bytes
                                         sha256 6e9f694b6951ef2de0a6888b5ab1d44787b2e4fd045f614924b8149b05357085
   nova publish                        (the same three lines, then)
                                       published shapes 0.1.0 to Sakeerin/nova-index; other
                                       machines may take up to five minutes to see it
   API requests used by the publish    17 - 15 = 2, as GitHub counts them
   nova-index size                     0 KB
6. nova new measure                    created `measure`: nova.toml, .gitignore, README.md, src/main.nova
   nova add shapes                     downloaded shapes 0.1.0
                                       added shapes = "0.1.0" to [dependencies]
   nova run                            area 3.141592653589793, perimeter 6.283185307179586
                                       area 6, perimeter 10
                                       total area 9.141592653589793
```

**The screenshot of step 3,** in words: in the separate VS Code, whose
Extensions view lists only Nova by `sakeerin`, `src/main.nova` holds the
cramped program with `record Point{x:Int,y:Int}`. On a new line inside
`main`, after `p.`, the completion list offers `x`, with its type `Int`,
and `y`.

**What went otherwise than the runbook says:**
- the machine and the extension's source, as decided above;
- step 4 ran in VS Code's PowerShell terminal, which prints no exit
  codes; its output is the runbook's;
- `NOVA_HOME` appears not to have been set until step 5 (no
  `~/nova-gate/nova-home` existed after step 2), so steps 1 to 4 unpacked
  the runtime into the account's own `~/.nova`; it was checked before
  `nova login`, and the token, the package and the download stayed in
  `~/nova-gate`;
- a first try at step 5 put the token on the `read` line, which showed
  it; it was revoked, a new one made, and nothing was published by that
  try. Both tokens are revoked.

### The backlog

The backlog is unscheduled. Each item is built when a program or a user
needs it, and the list promises no date, no phase and no order. Where an
item came from a pull request's "Deferred minors", the group names the
pull request.

- **Left out of Phase 3 by ADR 0026:** the REPL; the debugger (DWARF from
  both backends, DAP in VS Code); `nova lint`; `nova bench`; `nova
  install`, `nova clean`, `nova owner`, `nova self update` and `nova self
  uninstall`; the CI template and `setup-nova`; parallel test runs and TAP
  output; edition rules; the Zed and Neovim extensions.
- **`nova test`'s other gaps** (ADRs 0003, 0030): `--bench`, and
  `tests/**` below the top level of `tests/`.
- **Packages and the index** (ADRs 0030 and 0031; #105, #106):
  - yanking; several major versions of a package side by side;
    transparent mirrors; re-exports and `geom::utils` paths; separate
    compilation; an orphan rule;
  - the three caches no command cleans: runtimes (ADR 0027), downloaded
    packages (ADR 0031) and std's sources (ADR 0032);
  - a dependency whose `nova.toml` fails to parse reports on its first
    line, twice in a diamond; E0004's text and its target in a package
    with no library; M0010 naming manifests; Windows path spellings in
    messages; `nova add` through a symlinked `--path`, and `real_path` on
    UNC paths; exact case for `src/lib.nova`, `src/main.nova`, `src` and
    `tests`; the watch limit for a dependency's `nova.toml`; tests of rule
    5's note, a CRLF manifest under `add` and `remove`, review focus 2
    through `nova run`'s output, and `nova build` of a hyphenated
    package;
  - `nova login` checking the token's shape before sending it; `parse_lock`
    checking names, checksums and order; M0005 pointing at `nova update`
    when `nova fetch` cannot repair a lock; an invalid `NOVA_INDEX` in a
    project with no registry dependency; retrying GitHub's 422 and
    telling 409s apart; `config.json`'s `api` refusing `.` and `..`;
    packing's case-only and NFC/NFD clashes; the transfer timeout on slow
    links; a cached directory without `nova.toml`; M0014 naming the
    index's spelling; tests of the 10 MiB refusals.
- **The language server** (ADRs 0029, 0032, 0033; #104, #107, #108):
  - guarding every job against panics, and a larger checker stack (#104's
    finding 5); keeping the analyses that succeeded (6); exiting 0 after
    a message between shutdown and exit (7); `file:` URIs with an
    authority on Windows (8); computing a project key once (9); the burst
    test's timing (10); `member_detail` for members without a declaration
    span (11); ADR 0029's decision 4 against the `foo.` recovery in
    `parse` (12); a loose `main.nova`'s imported sibling's errors (13);
    probing completion at each cut point (14);
  - a rename that cannot place an edit refusing as a whole; std's cache
    across versions, its temporary file and read-only mark; hover's
    signature edge cases; the cost of the index when it is off; per-request
    canonicalisation; tests of client URI spellings, UTF-16 columns and
    definition of `import <package>`;
  - navigation of type aliases, `import … as` and the remaining pattern
    forms;
  - document versions on code-action edits; a semantic-tokens refresh when
    another file changes; fix-all, `codeAction/resolve`, and delta and
    range token requests; `nova check` colouring a pipe.
- **The formatter** (ADR 0028; #103): hugging the last argument;
  `separator()` and nested items; a parenthesised node at a file's end
  without a newline; an own-line comment between `}` and `else`; the
  fingerprint and literal spans; the self-check's two blind spots;
  `.editorconfig`'s byte-order mark and `..`; the misplaced-doc message in
  more positions; extern ABI strings with escapes; a blank line in a
  record's field list; property tests that format and reparse random
  trees (`nova-spec/11-PARSER.md`).
- **Docs and doc tests** (ADRs 0034, 0035; #109, #110): `should_panic`,
  `no_run`, `compile_fail` and compared output; doc tests of a program's
  items and of dependencies; checking examples as you type; an example per
  public std function; `[Name]` links and a page per item; leaving `pub
  @test fn` out of the doc tests; module names that are Windows device
  names; `nova doc --open` with a `,` in the path on Windows; stale line
  citations in std's maintainer comments.
- **std behaviours found while documenting it** (#109): `Stdout` losing
  the bytes after the last newline under `nova build`; `timeout` with a
  future that yields and retries; `std/http`'s `Server` and `HEAD`,
  `Connection: close`, HTTP/1.0 and `100 Continue`; a `TcpListener` closed
  while a task waits in `accept`.
- **Foundations** (#102): `gate.sh`'s `env | grep` under `pipefail`; a
  `nova.toml` with a byte-order mark or in UTF-16; the runtime cache's
  failure messages; `docs/benchmarks/README.md`'s old help text;
  `prepare`'s `gh release create` command; `release.yml`'s pull-request
  paths; the unpack test's start barrier; a `*.tmp` left by a killed
  unpack; whether a `[profile.release]` change rebuilds the embedded
  runtime.
- **Releases and installing** (ADR 0036; #111, #112): linux-aarch64 and
  musl archives; signed and notarized macOS binaries; Homebrew and winget;
  an icon and a bundled extension; a CI job that runs the
  `x86_64-apple-darwin` archive, and one on a distribution older than
  glibc 2.35; the glibc floor in `nova-cli`'s README and `install.sh`'s
  messages; picking the arm64 archive under Rosetta; a libc check before
  downloading; TLS 1.2 on old Windows before `irm`; `curl --retry` and
  `--proto '=https'`; the emulated install with `--locked`; the install
  tests' temporary directories; `VCRUNTIME140.dll` and `+crt-static`;
  actions pinned to commits; a test of an update while `nova.exe` runs;
  CI's redundant `cargo package -p nova-std` step; a runbook line on `gh
  release edit` when a publish fails for good; `publish-crates.yml`'s
  comment that says one new crate every 15 minutes, where crates.io allows
  one every 10.
- **Housekeeping:** the workspace `Cargo.toml` declares `chumsky`,
  `rustyline` and `toml`, which no crate uses; the master spec's §0 and
  sample manifest still say `novalang/nova`; `nova-test` is a stub;
  `nova-spec/20-STDLIB.md` cites `Display`'s `fn fmt` at
  `std/core/lib.nova:220`, the trait's line, where the `fn` is two lines
  below (#103).

Mapped elsewhere:
- **`0.3.1`:** the extension as `vscode-nova` in both stores, the texts
  that name `sakeerin.nova`, and the runbook's check of the manifest
  `name` (decided above).

  **Recorded 2026-10-14 (branch `claude/loving-archimedes-frtf5q`):**
  the stores took it in `0.3.2`, as Nova Language Tools (the note under
  "The extension's stores").
- **Phase 4:** `nova build --target wasm`, `nova bundle`, `nova dev`, and
  the LSP rows `40-TOOLING.md` §3.1 marks Phase 4.

### Found outside this boundary

Met while closing Phase 3; recorded, not decided:
- Issue #3 is still open: under `nova run` on Linux, a call into libm such
  as `sqrt` fails with E0902.
- The master spec's §1.1 says "No interpreter beyond REPL eval", and
  ADR 0001 says "Phase 3 uses AOT-then-dlopen" for its REPL; no REPL
  exists.
- `nova-spec/14-CODEGEN.md` §9.1 says LLVM emits DWARF 4; neither backend
  emits debug information. (Its note that there is no `inkwell` was
  already there.)
- `docs/phase-3-plan.md`'s status line still says "draft", as Phase 2's
  plan's did after its close.

## Consequences

- Every item of the master spec's §7 Definition of Done is met:
  1. the crates compile and CI passes on Linux, macOS and Windows;
  2. the Phase 3 gate is met with reproducible commands: the "Release
     gate" workflow and the runbook's 6.2;
  3. the new public surface is documented: `40-TOOLING.md`'s dated notes,
     std's documentation site, the READMEs and `docs/releasing.md`;
  4. `CHANGELOG.md` has `[0.3.0]`;
  5. ADRs 0026 to 0036 and this one record the deviations;
  6. `v0.3.0` is tagged.

  **Phase 3 is complete as `v0.3.0`, within this boundary.**
- What a `v0.3.0` user can rely on is the "shipped" rows, within the
  limits their records state. The editor extension is the release's
  `.vsix` until `0.3.1` puts it in the stores.

  **Recorded 2026-10-14 (branch `claude/loving-archimedes-frtf5q`):**
  the stores took it in `0.3.2`, not `0.3.1`. The inventory's rows and
  these lines stay as the boundary's record of `v0.3.0`; the note under
  "The extension's stores" has what followed.
- The backlog promises nothing. `0.3.1` carries the extension's rename.
- Dated notes point here from:
  - the master spec's phase notes before Phase 0, and its §6 (`salsa`, and
    the unused dependencies);
  - `docs/phase-3-plan.md`'s status block, its 3.6 entry and its §7;
  - ADRs 0027, 0031 and 0036;
  - `nova-spec/40-TOOLING.md`, at its head and in §3.3;
  - `docs/releasing.md`, where `v0.3.0`'s steps went otherwise.
- The guides are corrected in place instead, because they describe the
  code as it is now: `skill.md`'s DWARF row, and the `nova-test` rows of
  `agent.md` and `ARCHITECTURE.md`. The root README says where the
  extension is until `0.3.1`.
- `CHANGELOG.md` gets no entry: the close-out changes no behaviour (the
  3.6b spec's decision 11).

## References

- Spec: `docs/superpowers/specs/2026-10-12-phase-3-6b-the-release-design.md`
  §5 and §6, and its plan,
  `docs/superpowers/plans/2026-10-12-phase-3-6b-the-release.md`
- `nova-spec/00-MASTER-SPEC.md` §3 and §7; `docs/phase-3-plan.md`;
  `nova-spec/40-TOOLING.md`
- `docs/releasing.md`
- ADRs 0001, 0025, and 0026 to 0036
- Pull requests #102 to #112, and their "Deferred minors"
- The workflow runs: Release 38340770071; Publish to crates.io 38340986187,
  38341652388, 38345355576 and 38346870861; Release gate 38367944634
