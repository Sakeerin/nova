# ADR 0054 — Packages, the WASM target and test mode in the resolution dump

## Status

Proposed (2026-11-04), part A of 5.2b's prototype, branch
`proto/5-2b-whole`
(`docs/superpowers/specs/2026-11-02-phase-5-2-the-loader-and-the-resolver-design.md`
§4). It extends ADR 0053, whose §1 grammar and examples stay as they were
for 5.2a's forms: §1 here is what 5.2b adds to S0's dump and to the hidden
form; §2 the cases; §3 the fuzzer; §4 the corpus. Part B (N's loader of
packages, its WASM target and test mode, the differential on both sides,
and the live heap's figures, spec §4.4) writes the sections after them.

**Why a new ADR, not ADR 0053 amended:** ADR 0053 records 5.2a's state,
accepted and merged, and promises this one for 5.2b (its §2 and its
Status); the spec's records (§8) name ADR 0054 for packages, WASM, test
mode and the live heap. A reader porting the dump reads ADR 0053 §1, then
§1 here, which only adds forms and changes one: `UNSUPPORTED package`,
which S0 no longer prints. ADR 0053 §1 gains a dated note that says so.

## Decision

### 1. The dump of packages, the WASM target and test mode

**Roots.** A root is what `nova check FILE` takes (`Program::for_file`): a
file directly in the `src/` or `tests/` of a directory holding a
`nova.toml` (`package_of`, ADR 0053 §3) is a module of that package, any
other a loose program. 5.2a printed `UNSUPPORTED package` for the first;
S0 now dumps its program: the package graph read from the manifests, as
`nova_pm::graph` reads it, then the loader from that one root, breadth
first, with `src/` and `tests/` siblings, path dependencies' `src/lib.nova`
and their modules, E0004, E0093 and E0001's two notes, then `check`'s
stop, strip and resolver. A whole package (`nova check` with no file in a
project, `nova test`'s `src/main.nova`, `src/lib.nova` and `tests/*` in
one program) is not a root: the hidden form takes files, as 5.2a's did.

**Not synced.** `nova check FILE` syncs a package first (`project::sync`,
which may write `nova.lock` and fetch registry packages); the dump syncs
nothing and writes nothing, and reads registry packages only from what is
cached, as the language server does. A registry package's warnings are
dropped, as `check` hides them; no input of the differential has one.

**The graph's diagnostics** come first in `(Diagnostics`; with an error
among them the program stops before the loader, `resolved=false` with no
module, as `check` stops. They are nova-pm's (M-codes), which N's minimal
manifest reader (spec §4.1: a package's name and its path dependencies)
does not give: no case, corpus file or fuzz input has a manifest the graph
warns about, so they are S0's alone, and the extraction drops such
programs (§2).

**Labels.** A module's label is the loader's (spec §4.1): its stem, or
`tests/<stem>` for the root package's `tests/`, or a dependency's import
name for its `lib.nova` and `<import name>/<stem>` for its other modules.
A manifest is labelled `nova.toml` for the root package's and `<import
name>/nova.toml` for a dependency's, so E0004's secondary label, in the
manifest, names a place without a path; without a graph (a root manifest
that does not parse) every file the graph's diagnostics name is the root's
`nova.toml`.

**The package key** is each module's `package=`, the graph's `PackageId`:
0 for the root package, then the dependencies in the graph's breadth-first
order; `None` for a loose module and for std's. The graph itself is not
printed: the resolver sees only the key (`owned`, `same_package`), and the
labels show which package a module came from.

**Paths in messages.** E0004's message names the sibling and the
manifest by their paths, and a package module's E0093 its file, each as
`Path::join` writes it from the root as given, with `\` on Windows, as
5.2a's E0093 does (ADR 0053 §5). Part A compares no message across
systems; part B settles it for N (a case that holds one, run from inside
its package or compared with `\` read as `/` there).

**The WASM target.** `--target wasm` parsed in 5.2a and resolved with the
WASM std, its 13 WASM-only builtins in std's scopes, `IMPORTED_STD`'s
modules, E0090 and E0092; it was complete, and is now in `KINDS`:
`target=wasm`, and std's part's `(Missing` lines.

**Test mode.** `--test` resolves as `nova test` does
(`FrontendContext::check(true)`, which `build_test_program` runs): the root
package's and a loose program's `@test` functions are kept and a
dependency's stripped (`strip_test_functions` with `keep_root_tests`),
`std/test` joins std after its other modules, registered as `<std/test>`
and globbed into every module, std's own included, and a `tests/` file's
tests are named `<file stem>::<function>` (`name_tests`). The tests are
listed as `(Test` lines, in the resolver's order; E0083 to E0085 apply. The
part's header says ` mode=test`, after its target, and says nothing under
`check`, so 5.2a's dumps are unchanged byte for byte. `--std --test`
prints std's test-mode part, which `std/test` changes: one more module,
its definitions, and its names in every std module's scope (22 modules,
716 definitions, 551 items, 8,098 lines). A driver test checks that the
dump's tests are `nova test`'s, by id, name and `should_panic`, for a
package's `tests/` file and a loose file.

**What the hidden form takes.** After `check --dump-resolution`: `--std`,
`--target native|wasm`, `--test` and files, each option once, `--` before a
file that starts with `-`. `--test` with `--target wasm` is refused with
one line, `error: --dump-resolution --test is native only, as nova test
is`, nothing on stdout and the exit code 2, as `--std` with a file is
(ADR 0053 §1): `nova test` has no `--target`. `--test=…`, `--test --test`,
`--tests` and the rest are clap's errors under the form's own usage line.
`check --test …` without `--dump-resolution` first is the base's error,
byte for byte.

**The grammar**, ADR 0053 §1's with these changes:

```
part       = "IOERR " kind "\n" | program diagnostics      -- "UNSUPPORTED package" is gone
program    = "(Program target=" target [" mode=test"] " resolved=" bool
             { "\n" module(1) } [ ... ] ")\n"              -- as in ADR 0053 §1
std        = "(Std target=" target [" mode=test"] " modules=" uint " defs=" uint
             " items=" uint ... ")\n"                       -- as in ADR 0053 §1
module(1)  = I(1) "(Module " uint " " string " package=" ("None" | uint) ...
             -- a package's module: its key in the graph, 0 for the root package
place      = (string | "?") ":" uint ".." uint
             -- a manifest's label is "nova.toml" or "<import name>/nova.toml"
```

`KINDS` gains `package=<n>`, `target=wasm`, `(Missing`, `mode=test`,
`(Test`, `(Test should_panic=true` and `(Test should_panic=false`;
`LATER_KINDS` keeps the two severities no program prints, `(Help` and
`(NoteDiagnostic`.

**One example of every new form**, each S0's dump of the sources after
it, from the directory that holds them, reduced as ADR 0053's are:

```
;; package=<n>: a package's modules, its dependency's labelled by its import name:
;; nova check --dump-resolution app/src/main.nova
;; app/nova.toml: [package]\nname = "app"\nversion = "0.1.0"\n
;;     edition = "2026"\n[dependencies]\nshapes = { path = "shapes" }\n
;; app/src/main.nova: import shapes\nfn main() {}\n
;; app/shapes/nova.toml: [package]\nname = "shapes"\nversion = "0.1.0"\n
;;     edition = "2026"\n
;; app/shapes/src/lib.nova: import units\npub fn area() -> Int { unit() }\n
;; app/shapes/src/units.nova: pub fn unit() -> Int { 1 }\n
(Program target=native resolved=true
  (Module 0 "main" package=0
    (Import "shapes" 1)
    (Importable "shapes" 1))
  (Module 1 "shapes" package=1
    (Import "units" 2)
    (Importable "units" 2))
  (Module 2 "shapes/units" package=1
    (Importable "lib" 1))
  (StdStart module=3 def=3 item=5))

;; A tests/ module, and E0001 with a note: the package itself, which has no library:
;; nova check --dump-resolution app/tests/api.nova
;; app/nova.toml: [package]\nname = "app"\nversion = "0.1.0"\n
;;     edition = "2026"\n
;; app/src/main.nova: fn main() {}\n
;; app/tests/api.nova: import app\n
(Program target=native resolved=false
  (Module 0 "tests/api" package=0
    (Import "app" Reported)))
(Diagnostics
  (Error "E0001" "cannot find module `app`"
    (Primary "tests/api":7..10 "no such module")
    (Note "`app` has no src/lib.nova")))

;; E0001 with a note: a dev-dependency imported from src/:
;; nova check --dump-resolution app/src/main.nova
;; app/nova.toml: [package]\nname = "app"\nversion = "0.1.0"\n
;;     edition = "2026"\n[dev-dependencies]\nhelper = { path = "helper" }\n
;; app/src/main.nova: import helper\nfn main() {}\n
;; app/helper/nova.toml: [package]\nname = "helper"\nversion = "0.1.0"\n
;;     edition = "2026"\n
;; app/helper/src/lib.nova: pub fn h() {}\n
(Diagnostics
  (Error "E0001" "cannot find module `helper`"
    (Primary "main":7..13 "no such module")
    (Note "`helper` is a dev-dependency, which only `tests/` files can import")))

;; E0004, its Secondary in the manifest, labelled nova.toml:
;; nova check --dump-resolution app/src/main.nova
;; app/nova.toml: [package]\nname = "app"\nversion = "0.1.0"\n
;;     edition = "2026"\n[dependencies]\nshapes = { path = "shapes" }\n
;; app/src/main.nova: import shapes\nfn main() {}\n
;; app/src/shapes.nova: pub fn area() -> Int { 1 }\n
;; app/shapes/nova.toml: [package]\nname = "shapes"\nversion = "0.1.0"\n
;;     edition = "2026"\n
;; app/shapes/src/lib.nova: pub fn area() -> Int { 2 }\n
(Diagnostics
  (Error "E0004" "`shapes` is both a module of this package and a dependency: app/src/shapes.nova, and the entry at app/nova.toml:6"
    (Primary "main":7..13 "this import is ambiguous")
    (Secondary "nova.toml":73..79 "the dependency")))

;; A package graph's diagnostics: the program stops before the loader:
;; nova check --dump-resolution app/src/main.nova
;; app/nova.toml: [package]\nname = "app"\n
;; app/src/main.nova: fn main() {}\n
(Program target=native resolved=false)
(Diagnostics
  (Error "M0002" "`[package]` has no `version`"
    (Primary "nova.toml":0..22 "add `version = \x22...\x22` to this table"))
  (Error "M0002" "`[package]` has no `edition`"
    (Primary "nova.toml":0..22 "add `edition = \x22...\x22` to this table")))

;; mode=test, Test should_panic=false and true: nova test's resolution of a tests/ file:
;; nova check --dump-resolution --test app/tests/api.nova
;; app/nova.toml: [package]\nname = "app"\nversion = "0.1.0"\n
;;     edition = "2026"\n
;; app/src/lib.nova: pub fn one() -> Int { 1 }\n@test\nfn in_lib() {}\n
;; app/tests/api.nova: import app\n@test\nfn works() {}\n@test(should_panic)\n
;;     fn panics() { panic("no") }\n
(Program target=native mode=test resolved=true
  (Module 0 "tests/api" package=0)
  (Module 1 "lib" package=0)
  (Test u0 "api::works" should_panic=false)
  (Test u1 "api::panics" should_panic=true)
  (Test u3 "in_lib" should_panic=false))
(Diagnostics)

;; E0084 and E0085, under test mode:
;; nova check --dump-resolution --test main.nova
;; main.nova: @test\nfn takes(n: Int) {}\n@test(nope)\nfn t() {}\n
;;     fn main() {}\n
(Program target=native mode=test resolved=true)
(Diagnostics
  (Error "E0084" "a `@test` function may not have parameters"
    (Primary "main":9..14 "invalid test signature")
    (Secondary "main":15..21 "parameters here"))
  (Error "E0085" "unknown `@test` argument `nope`; the accepted arguments are: should_panic"
    (Primary "main":32..36 "unknown argument")))

;; Std mode=test, from nova check --dump-resolution --std --test:
(Std target=native mode=test modules=22 defs=716 items=551
  (Module 21 "$std.test" package=None)
  (Scope 0
    (Value.Def "assert_eq" s714)))
(Diagnostics)

;; target=wasm, and E0090 and E0092:
;; nova check --dump-resolution --target wasm page.nova
;; page.nova: import ui::ssr\nextern "C" {\n    fn abs(x: Int) -> Int\n}\n
;;     fn main() {}\n
(Program target=wasm resolved=true)
(Diagnostics
  (Error "E0090" "extern functions are not available on target wasm"
    (Primary "page":15..55 "this `extern` block"))
  (Error "E0092" "`ui::ssr` is a std module, which target wasm does not have"
    (Primary "page":7..14 "not on target wasm")))

;; Std target=wasm, Missing, from nova check --dump-resolution --std --target wasm:
(Std target=wasm modules=16 defs=781 items=563
  (Module 10 "$std.dom" package=None)
  (Missing "args" "$std.process"))
(Diagnostics)
```

**Sizes.** std's WASM part is 5,260 lines (233 KB): 16 modules, 781
definitions, 563 items and 92 `missing` names; its test-mode part 8,098
lines (333 KB), against the native part's 7,728 (320 KB). A package's
program holds every module its root reaches: `examples/05-geometry`'s
`src/main.nova` is 587 lines (19 KB), `examples/07-fullstack-blog`'s 2,184
(71 KB), and `selfhost/src/main.nova`, which reaches most of N, 10,661
lines (391 KB); the corpus's 66 package files dump to 146,588 lines (5.2
MB) in 2.5 s, S0 debug, one process.

### 2. The cases

`crates/nova-cli/tests/selfhost/cases/` holds a directory per mode, which
the directory's name says, so a case needs no marker: `resolve/` under
`check`, `resolve-wasm/` for the WASM target and `resolve-test/` under test
mode. In each, a `.nova` file is a program; a directory holding a
`nova.toml` is a package, and each `.nova` file directly in its `src/`,
then its `tests/`, sorted, is a root, named from the mode's directory;
any other directory is a loose program rooted at its `main.nova` (one
holding `dom.nova` or `ui.nova` runs from inside itself, ADR 0053 §5). A
package's dependencies are inside it, at the `path`s its manifest gives.

The cases come from the programs the tests resolve, recorded by a
temporary instrumentation of the resolver and the loader while the
workspace's library tests, the driver's and the CLI's package, fix,
doc-test, analysis, index and project tests, `wasm_tests.rs` and
`run_tests.rs` ran (`$P/proto52b/instrument.py`, `extract.py`): every
program resolved as a package, for the WASM target or under test mode,
kept when it is a package's, gives a resolver diagnostic, holds a
`@test` (under test mode) or imports `dom` or `ui` (for the WASM target);
dropped when it is a copy of a tracked file, a repeat, a program of doc
tests, a registry package's, or one whose graph gives a diagnostic; each
text with `\r\n` read as `\n`. A package whose dependencies were outside
it has them moved under `pkgs/`, its manifests' `path`s rewritten. Named
`<file>__<test>[_n]` as 5.2a's; a test that ran `nova` is named by its
temporary directory. The 5.2a cases holding a `@test` are copied to
`resolve-test/`, since `check` strips what they test. Hand cases (names
without `__`): E0004, a package module named `ui` (E0093), E0001's two
notes, a dependency's modules labelled by its import name with a dropped
fix and an E0102 in it, a dependency's tests stripped while a `tests/`
file names its own, a `tests/` file's dev-dependency and siblings, every
`@test` shape (E0083 to E0085), a loose sibling's kept test, E0090 with
E0092, `dom` and `ui` imported on WASM, and a WASM package whose dependency
has an `extern` block.

At part A: `resolve/` 141 programs (103 of 5.2a, and 26 packages with 38
roots), `resolve-wasm/` 30, `resolve-test/` 126 (37 packages with 71
roots). 5.1's token and AST differentials read them too, `nova fmt`'s gate
skips `cases/`, and the CRLF test copies all three directories,
`nova.toml`s included.

### 3. The fuzzer

`fuzz::wasm_name_mutate` and `fuzz::package_name_mutate` are
`name_mutate`, whose fixed seeds 301 to 303 give the inputs they gave,
then one edit of their own. The WASM sibling adds an import of `ui::ssr`
(E0092 there), `ui::router` or `dom`, or an `extern "C"` or `extern "JS"`
block (E0090); its inputs are the loose WASM corpus programs, beside
`NAME_SIBLINGS`, seeds 311 to 313, 20 each. The package sibling puts its
input in `src/` or, one in three, `tests/` of `PACKAGE_FILES`' package
`app` (beside `NAME_SIBLINGS` in both, a `src/clash.nova`, a dependency
`dep` whose library imports its own `inner` and keeps a private item and a
`@test`, a dependency `clash`, a dev-dependency `devdep`, and no library
of its own), and adds an import of `dep`, of its items, of a private one,
of `devdep`, of `clash` (E0004) or of `app` (from `tests/`, E0001 with a
note); seeds 321 to 323, 30 each, from the loose corpus, named from inside
`app`. The loose name inputs are also resolved under test mode; the
package's under `check` only, to keep the fuzzer's time in proportion
(test mode on a package is the cases'). The inputs are the same on every
system, which the Windows-checkout test checks for the two siblings too.

On the fixed seeds (S0's dump; part A): under `check` E0001 46, E0002 63,
E0082 14, E0083 8, E0089 10, E0092 13, E0102 50, E0900 18 (5.2a's); under
test mode E0084 8 and E0085 5 more, among the same; the WASM inputs
E0090 28 and E0092 10; the package's E0004 9, E0001's note on a package
with no library 5 and on a dev-dependency 6. The harness asserts each of
the 11 codes, E0004 and the two notes at least twice over all of them,
E0084 and E0085 only under test mode and E0090 only for WASM.

### 4. The corpus

Natively, every corpus file is now a root: the 251 loose programs and the
66 files directly in a package's `src/` or `tests/`, in 11 directories
(the spec's nine packages, `05-geometry`'s dependency `shapes`, and the VS
Code extension's fixture), which 5.2a skipped; for the WASM target, the 39
files under `tests/dom/` and `tests/wasm/`, six of them a package's; under
test mode, the 9 files that hold a `@test`. 5.2a's floor of 66 skipped
files is replaced by floors on these counts.

**Time (part A, S0 alone).** `selfhost.rs` ran in 36.7 and 38.0 s
locally, against 20.8 and 21.0 s for the 27 tests 5.1 had, interleaved:
1.76 and 1.81 times, where 5.2a measured 1.40 to 1.77. Part B adds N's
side of every 5.2b input.

## Consequences

- S0's hidden dump covers every program `nova check FILE` and `nova test`'s
  path take, packages, the WASM target and test mode; `nova check`, its
  help and every command line without `--dump-resolution` first are the
  base's.
- The differential's new inputs run through S0 alone until part B; part B
  replaces each `s0_resolution` call with a comparison.
