# ADR 0054 — Packages, the WASM target and test mode in the resolution dump

## Status

Accepted (2026-11-04). Branch `claude/loving-archimedes-frtf5q-52b`
(`docs/superpowers/specs/2026-11-02-phase-5-2-the-loader-and-the-resolver-design.md`
§4). It extends ADR 0053, whose §1 grammar and examples stay as they were
for 5.2a's forms: §1 here is what 5.2b adds to S0's dump and to the hidden
form; §2 the cases; §3 the fuzzer; §4 the corpus (part A, S0's side);
§5 N's manifest reader and package graph; §6 what N ports and what it
does not; §7 the differential on both sides; §8 the decisions part A took
and the spec's correction; §9 CI's time; §10 the live heap (part B).

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
manifest reader (§5) does not give: no case, corpus file or fuzz input has a manifest the graph
warns about, so they are S0's alone, and the extraction drops such
programs (§2).

**Labels.** A module's label is the loader's (spec §4.1): its stem, or
`tests/<stem>` for the root package's `tests/`, or a dependency's import
name for its `lib.nova` and `<import name>/<stem>` for its other modules.
A manifest is labelled `nova.toml` for the root package's and `<import
name>/nova.toml` for a dependency's, so E0004's secondary label, in the
manifest, names a place without a path; without a graph (a root manifest
that does not parse) every file the graph's diagnostics name is the root's
`nova.toml`. A dependency's manifest that gives an M-code is not in the
graph, so its file has no label, and its M-code's labels print `?` in
place of one (`(Primary ?:82..83 ...)`); N refuses such a manifest (§5).

**The package key** is each module's `package=`, the graph's `PackageId`:
0 for the root package, then the dependencies in the graph's breadth-first
order; `None` for a loose module and for std's. The graph itself is not
printed: the resolver sees only the key (`owned`, `same_package`), and the
labels show which package a module came from.

**Paths in messages.** E0004's message names the sibling and the
manifest by their paths, and a package module's E0093 its file, each as
`Path::join` writes it from the root as given, with `\` on Windows, as
5.2a's E0093 does (ADR 0053 §5). The differential compares these two
messages with `\` read as `/` on Windows (§7).

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
(71 KB), and `selfhost/src/main.nova`, which reaches most of N, 11,202
lines (411 KB); the corpus's 67 package files dump to 152,084 lines (5.4
MB) in 2.7 s, S0 debug, one process.

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
`run_tests.rs` ran (a script, not committed): every
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
without `__`): E0004, E0004 inside a dependency (its secondary label
that dependency's manifest, which also has a dev-dependency the graph
never reads), E0004 for a `tests/` module named for the package (its
secondary label `[package]`'s span, whose last value is followed by a
comment; the first fact-check), a package module named `ui` (E0093), E0001's two
notes, a dependency's modules labelled by its import name with a dropped
fix and an E0102 in it, a dependency's tests stripped while a `tests/`
file names its own, a `tests/` file's dev-dependency and siblings, every
`@test` shape (E0083 to E0085), a loose sibling's kept test, E0090 with
E0092, `dom` and `ui` imported on WASM, and a WASM package whose dependency
has an `extern` block.

At the split: `resolve/` 145 programs (103 of 5.2a, and 28 packages with
42 roots), `resolve-wasm/` 30, `resolve-test/` 126 (37 packages with 71
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
note); seeds 321 to 323, 30 each, from the loose corpus. The package is
written four times, `v0/` to `v3/`, each with its manifests in one of four
forms N's reader takes and nova-pm reads with no M-code
(`fuzz::package_manifests`: the plain one; comments, a header with
spaces, `'...'`, a pre-release, more keys, the tables in another order
and a comment after `[package]`'s last value; the dependency tables as
inline tables before the header and the largest version; a byte-order
mark and `\r\n`; `dep`'s adding a version entry and a path to nothing
among its dev-dependencies), input `i` in form `i % 4`; the harness
asserts that E0004 names its entry on more than one line of a manifest
over them. The loose name
inputs are also resolved under test mode; the package's under `check`,
and seed 321's also under test mode and seed 322's for the WASM target,
each in the process of that mode's loose inputs, which costs no
measurable time (the first fact-check; §9). Manifests N refuses are not
among the fuzzer's: a refusal stops N's whole process, so each would
need two processes of its own (about 0.5 s of N's debug runtime each),
and the reader's table, the repository's manifests (§5) and a test of
the refusal pin them instead. The inputs are the same on every system,
which the Windows-checkout test checks for the two siblings too.

On the fixed seeds (S0's dump; part A): under `check` E0001 46, E0002 63,
E0082 14, E0083 8, E0089 10, E0092 13, E0102 50, E0900 18 (5.2a's); under
test mode E0084 8 and E0085 5 more, among the same; the WASM inputs
E0090 28 and E0092 10; the package's E0004 9, E0001's note on a package
with no library 5 and on a dev-dependency 6. With the manifest forms and
the two modes (the first fact-check), all the dumps together hold E0004
16 times and each note 9. The harness asserts each of
the 11 codes, E0004 and the two notes at least twice over all of them,
E0084 and E0085 only under test mode and E0090 only for WASM.

### 4. The corpus

Natively, every corpus file is now a root, 318: the 251 loose programs and
the 67 files directly in a package's `src/` or `tests/`, in 11 directories
(the spec's nine packages, `05-geometry`'s dependency `shapes`, and the VS
Code extension's fixture), which 5.2a skipped; for the WASM target, the 39
files under `tests/dom/` and `tests/wasm/`, six of them a package's; under
test mode, the 9 files that hold a `@test`. 5.2a's floor of 66 skipped
files is replaced by floors on these counts. The spec (§4.5) asks for
every corpus program under the WASM target; the harness runs these 39
(**differs**), since the rest are native programs and CI's Windows leg,
at about 60 s (§9), has no room for them; the second fact-check ran all
334 roots for the WASM target through both sides, and they agreed.

**Time (part A, S0 alone).** `selfhost.rs` ran in 36.7 and 38.0 s
locally, against 20.8 and 21.0 s for the 27 tests 5.1 had, interleaved:
1.76 and 1.81 times, where 5.2a measured 1.40 to 1.77. Part B adds N's
side of every 5.2b input (§9).

### 5. N's manifest reader and package graph

`selfhost/src/manifest.nova` reads what the loader needs of a
`nova.toml` and nothing more (decision 2 of §8): the package's name and
where its `[package]` table is, and each entry of `[dependencies]` and
`[dev-dependencies]` with its `path` and where its key is. That is more
than the spec's "name and path dependencies" (§4.1): the dev-dependencies
(a `tests/` module sees them, and `src/` gives E0001's note for one), the
spans E0004 prints (its secondary label is the entry's key; its message
names the entry's manifest and line), `[package]`'s span (the entry of the
package itself, which a `tests/` module sees by its import name, so
E0004 there labels the whole table, from its `[` to the end of its last
value, as toml_edit spans it), and whether each package's `src/lib.nova`
(and the root's `src/main.nova`) is a file. The M-codes stay S0's.

**It accepts** comments, blank lines, `\r\n`, a byte-order mark, and the
tables `[package]`, `[dependencies]` and `[dev-dependencies]`, each once,
headers with spaces inside the brackets, the two dependency tables also
as one inline table of inline tables before every header (`dependencies
= { geom = { path = "../geom" } }`, as `nova add` writes into one); bare keys; strings in `"..."` without an
escape or in `'...'`; in `[package]` the keys nova-pm reads (`name`,
`version` and `edition`, which must be there, `description`, `license`,
`repository` as strings, `authors`, `keywords`, `categories` as arrays of
strings on one line, a trailing comma allowed); in the dependency tables
`key = { path = "..." }` on one line; and in `[dev-dependencies]` a
version entry, `key = "REQ"` or `key = { version = "REQ" }`, whose
requirement is one to three numbers between dots after `^`, `~`, `=` or
nothing, each without a leading zero and at most `u64::MAX`, which
semver's `VersionReq` takes. nova-pm reads every manifest's
dev-dependencies but resolves only the root's, so a dependency's version
entries are read and left alone, as S0 leaves them, and the graph refuses
one in the root's manifest, which would need the registry. **It
refuses**, with a message, everything else: another table or key (which
S0 warns about, M0006), a dotted or quoted key, an array of tables, a
multi-line string, an escape, a number or a boolean, a version entry in
`[dependencies]` (which needs the registry), a `path` beside a `version`
(M0003), any other requirement (S0 takes more: `*`, `>=1`, a
pre-release, a list), a key written twice, a name nova-pm refuses
(`check_name`: a letter first, ASCII letters, digits, `-` and `_`, at
most 64, not a Windows device, not `dom` or `ui`), a version semver
refuses (three numbers, each without a leading zero and at most
`u64::MAX`, a pre-release's unbounded) or one with build metadata
(`+...`, M0003; a pre-release is read), an edition other than 2026, and
a control character other than a tab, a line feed and a `\r` before one
(DEL included), as TOML refuses it. N's test
`the_manifest_readers_accepted_and_refused_forms_are_pinned` pins this
table: 16 texts read and 51 refused, and three refusals' words. S0 reads
14 of the 16 with no M-code; the other two hold version entries among the
root's dev-dependencies, which the graph refuses and S0 gives an M-code
(the registry). S0 gives 48 of the 51 an M-code, and reads the other
three (a quoted key, an array over two lines, a multi-line string),
which no manifest of the repository holds. Among the 51 are a name of 65
characters, `1.0.0.0` and each number of a version one past `u64::MAX`
(the first fact-check found N reading those three, and DEL), and the
graph's test refuses a dependency's version past `u64::MAX`.

**Every manifest of the repository** (at the split, the 100 tracked `nova.toml` files
and every manifest the workspace's tests parse, recorded by a temporary
instrumentation of nova-pm while the whole suite ran: 211 distinct
texts), each in a package with `src/main.nova` and a library at each of
its relative paths, through both sides: 131 read as S0 reads them, the
same dump; 80 refused where S0 gives an M-code; none refused that S0
reads. Before the split four were: the inline dependency table `nova
add` writes (two), a byte-order mark, and a pre-release version; the
reader takes them now. After the first fact-check, at the 104 tracked
`nova.toml` files and every manifest the whole suite parses: 229 distinct texts, 146 read as S0
reads them, 83 refused where S0 gives an M-code, none refused that S0
reads.

**The graph** (`loader.nova`'s `read_graph`) is nova-pm's walk: the
root, then breadth first each package's entries, and the root's
dev-dependencies after its dependencies, a package read once by its
identity, its key its index. nova-pm's identity is `real_path` of the
package's directory, the absolute path with every link followed. N's
std and runtime have no way to name the current directory or follow a
link, so N's identity is the directory as the root's path and the
entries' paths spell it, with `.` and `x/..` folded (**differs**). The
two can differ on exactly two kinds of input: any path through a link,
whatever the root; and, unless the root is given by an absolute path,
any dependency path that is absolute or whose folded form climbs above
the current directory (starts with `..`), since N cannot compare it
with the other paths without the current directory's name. The two are
the same package for every input of the differential: no input holds a
link, every input that spells a dependency by its absolute path gives
its root by its absolute path too, and no folded path climbs above the
current directory. Where they differ: one directory spelled by an
absolute path and a relative one under a relative root (the first
fact-check's case (a)), or by a path that climbs above the current
directory and comes back down and one that does not (the second
fact-check's: in a directory `run`, `b` depends on `../../run/geom`), is
two packages of one name to N, which refuses the graph as M0011, saying
that the two may be one directory, where S0 reads one package; one
directory reached through a link and without it (case (b)) is refused
as M0011 too, with no such clause, since N cannot see a link; and
`x/..` after a link `x` (case (c)), which N folds and `real_path`
follows, makes N dump a package other than S0's, or S0 give an M-code
where N dumps. A Unix test pins the first three, each pair of paths
each way round, beside a root given absolutely, where both read one
package; on Windows N's paths read no drive, which 5.5
takes up with a root written with `\`.
Where nova-pm gives an error the graph refuses instead: a dependency's
directory with no manifest (M0007), a key that is not the package's name
(M0008), a dependency with no library (M0009), its own directory or a
cycle of `[dependencies]` (M0010), two packages of one name (M0011), an
import name that is a keyword, taken twice or the root's own (M0012),
a root with neither a library nor a program (M0013). A refusal stops
`selfhost resolve`: it prints `error: selfhost resolve cannot dump ROOT:
WHY` on stderr, the manifest's path in WHY, and exits 1 (not 2, which
is a root that cannot be read or a misuse, as in S0's form), having printed
nothing of that root's part, never a partial dump; the roots before it
were printed. A test pins it beside S0's M-code for the same root.

### 6. What N ports, and what it does not

N's loader (`load_program`, `Loader::find`, `has`, `visible_edges`,
`clash`, `module_dir`, the labels and `root_place`) is
`program.rs`'s, function for function, for the roots §1 takes: a loose
file, or a file directly in a package's `src/` or `tests/`
(`package_of`). A `Place` is a package (-1 for a loose file), whether it
is in `tests/`, and a stem; modules are found in the same order, labelled
as S0 labels them, keyed as S0 keys them (`package=`), and each import's
E0004, E0093 and E0001 with its two notes is S0's, word for word. The
driver's `strip_test_functions` takes `keep_root_tests`, and
`name_tests` names a `tests/` file's tests `<file stem>::<function>`.
`selfhost resolve` takes `--target native|wasm` (also as
`--target=VALUE`, as clap takes it for S0, since the first fact-check)
and `--test`, each once, as the hidden form does; like clap, it refuses
an empty target, a second `--target` in either spelling and a value
given to `--test` (since the second fact-check), and `--test --target
wasm` with one line on stderr, each with the exit code 2, as S0 does. For the WASM target it parses
`WASM_STD_MODULES` and also `STD_MODULES`, which `missing` reads, a file
both name parsed once; under test mode `std/test` follows std's modules,
labelled `<std/test>`. The resolver needed nothing new: 5.2a had ported
the WASM builtins, `IMPORTED_STD`, E0090, E0092, `missing`, the `@test`
checks (E0083 to E0085) and `TestFn`s, which no comparison had reached.

**Not ported**, all S0's alone: the M-codes and every manifest the reader
refuses (§5); registry packages, `nova.lock` and `project::sync`, which
the dump never runs; doc tests (`RootKind::Doc`) and `nova test`'s
whole-package program of several roots (decision 1); a registry
package's dropped warnings (no input has one); `real_path`'s identity
through a link or the current directory (§5).

### 7. The differential on both sides

Every input part A ran through S0 alone is now compared byte for byte
with N, exit codes included: std's native, WASM and test-mode parts; the
corpus natively, 251 loose programs and 67 files directly in a package's
`src/` or `tests/` (318, §4), its 39 WASM programs for the WASM target
and its 9 files with tests under test mode; the cases, `resolve/` 145
programs (42 a package's roots), `resolve-wasm/` 30 and `resolve-test/`
126, and all of them again with `\r\n`; and the name fuzzer's fixed
seeds: 150 loose inputs under `check` and under test mode, 60 WASM
inputs and 90 package inputs, 30 of them also under test mode and 30
for the WASM target. The floors part A set on these counts stay
(240 loose, 60 package files in 10 directories, 35 WASM, 8 with tests;
130, 25 and 115 cases), and so do the floors on codes, now over dumps
both sides print: every one of the 11 codes, E0004 and both notes twice
or more, E0084 and E0085 only under test mode, E0090 only for the WASM
target, every `KINDS` form somewhere and no `LATER_KINDS` form. 5.2a's
tests of refusals, roots and manifests compare N too: a package's root
in the header test, roots in a package named from inside and outside it,
`Nova.toml` on a case-folding disk, a linked, a dangling and an
unlistable manifest; and both sides refuse no root under each mode, and
`--test --target wasm` with a root or `--std`.

**What the comparison found:** nothing. The first build of N's package
loader and its WASM and test-mode switches agreed with S0 on every input
above, so neither N nor S0's dump was changed by it; N's own tests found
one wrong expectation in a test (the root package's library seen from
`tests/` is labelled `lib`, as S0 labels it).

**Paths on Windows** (decision 4). E0004's message names the sibling and
the manifest by path, and E0093's the module it refuses, each as
`Path::join` writes it, with `\` on Windows, which the dump escapes as
`\x5c`, where N writes `/`. The harness compares these two messages with
`\x5c` read as `/` on Windows (`compare::settle_paths`), and nothing else;
a test checks the rule on every system. So the package inputs are
compared on Windows too, not skipped. CI's Windows leg found one test
that compared S0's and N's dumps of E0004 without the rule (the
altered-std test of §9): the harness now settles every resolution dump,
S0's and N's, in the one function that runs either side, so no
comparison can miss it, and a test on every system feeds that function
S0's E0004 as Windows writes it and checks it equals N's.

**Edges probed before the split**, each through both sides:

- A dependency reached through `..`, an absolute one, and E0004 inside a
  dependency reached through `..`: the same dump; both print a path as it
  is joined, `..` kept.
- A dependency that is the package itself (M0012, its import name the
  package's), two entries with one path (M0008), and a dev-dependency
  that is also a dependency (M0012): S0's M-code, N's refusal, exit 1.
- A root in a subdirectory of `src/` (a loose program on both:
  `package_of` looks directly in `src/` and `tests/` only), an empty
  `tests/`, a package whose only root is `src/lib.nova`, a `tests/` file
  importing the library, a dependency and a dev-dependency, `--test` on
  a package of four dependencies whose tests (one named as the root's)
  are stripped, and colliding `@test`s (E0002 under test mode, stripped
  under `check`), each under `check` and test mode: the same dump.
- **One difference found:** N's reader read a manifest ending in a `\r`
  with no line feed, or with a control character in a comment, which
  toml_edit refuses (M0001). It now refuses every control character but
  a tab, a line feed and a `\r` before one, as TOML does.
- Case-insensitive disks: both sides ask for `nova.toml` by its path and
  compare the parent's name with `src` and `tests` exactly, so `Nova.toml`
  is a manifest on a case-folding disk on both (5.2a's test, now through
  N too) and `SRC/main.nova` is a loose program on both; a dependency's
  manifest is asked for by its path as well.
- Windows: `src\main.nova` typed as a root is a package's module to S0 and
  a loose program to N, whose paths split at `/` only (ADR 0053 §5); the
  harness writes every root with `/`, and 5.5 settles both. macOS (a
  temporary directory behind a link): a package is identified by
  `real_path` in S0 and by its folded path in N, which differ only on
  the inputs §5 lists, none of the differential's; a temporary directory
  behind a link is behind it in every spelling, and no printed path is
  canonical.
- CRLF: `cases/` has no `.gitattributes` rule, so CI's Windows checkout
  gives every case's `nova.toml` `\r\n`, and the CRLF copy reads them on
  every system.
- The harness's threads are scoped and joined in a fixed order, and each
  output is joined in order: no count of cores changes what is compared
  or printed.
- `--test` before or after a root, beside an empty root, twice, and with
  `--std --target wasm` in either order: the same stdout and exit code on
  both sides (a test). A `--` is still a root of that name to N, where
  S0's form skips it (ADR 0053 §5); the harness passes none.

### 8. The decisions part A took, and the spec's correction

1. **Roots are files:** a file directly in a package's `src/` or
   `tests/`, or a loose program; `nova test`'s whole-package program is
   not a root. If 5.5 needs it, the loader already takes one root per
   call and the queue would start from several.
2. **M-codes stay S0's.** N's reader is minimal, reads what §5 lists, and
   refuses the rest; no comparison input has an M-code.
3. **The spec's §4.1 note "a dependency without `src/lib.nova`"** is not
   what happens: the graph refuses a dependency with no library first
   (M0009). E0001's note "`<name>` has no src/lib.nova" comes only when a
   package with no library is imported, by its own import name, from its
   own `tests/`. The spec is left as approved; this is the correction.
4. **Windows paths** are compared with `\` read as `/` (§7), as ADR 0053
   settles paths, not skipped.
5. **Test names** are `name_tests`'s, `<file stem>::<function>` for a
   `tests/` file's.

### 9. CI's time

**Before the split** (the prototype as part B left it; 4 cores, Linux,
the test binaries prebuilt, three interleaved rounds): `selfhost.rs` 51.6,
49.5 and 51.9 s, 165 to 174 s of user CPU; 5.1's 27 tests alone 22.4,
24.1 and 21.1 s. Each test alone, N's build (3.7 s of CPU) in each: the
corpus's and the cases' resolution 67 s of CPU, the name fuzzer 36 s, the
cases again with `\r\n` 22 s, 5.1's mutated corpus 15 s, its corpus and
its CRLF corpus 9 s each, and 5.1's every-kind test 9 s, which ran both
sides over the corpus and the cases again. The processes are nearly all
the CPU (the harness 1 to 2 s): N, with the debug runtime, about two
thirds of it. Where N's time goes (callgrind): the debug runtime is about
five sixths of N's instructions; of N's own work on a package's root,
lexing and parsing are about half; on a small root, resolving std again
for each program (seeding std's scopes, collecting its items) is about
half, as it is S0's (61% of S0's per-root time). N read and parsed std
once per process, 0.40 s (0.57 s for the WASM target, which parses
`STD_MODULES` too), and 61 N processes took roots, 50 to a batch.

**The speed-up**, every input kept:

- One process a side for each set and mode (`agree_on_resolution_in`):
  the loose corpus, the package corpus, the WASM corpus, the files with
  tests, each mode's cases (loose and packages together, the case apart
  from its own directory), each of 5.2a's small tests' sets; the sets of
  a test still run side by side.
- The name fuzzer's three seeds in one process a mode, each seed's inputs
  in a directory named for it (`names/301/f00000.nova`), counted seed by
  seed from the headers (`seed_parts`); the package inputs, each seed
  from inside its own package, as before.
- 5.1's corpus and cases dumps compared once a run (`Compared`), and the
  every-kind test reads them, as it read both sides' equal output again.
- The CRLF copies compared only where the copy changes a file: on a
  Windows checkout, which already has each `\r\n`, the cases' and the
  corpus's copies change nothing, and the first comparison was of those
  bytes; on Linux and macOS every file with a line break changes.

**After:** 49.2, 47.3 and 48.7 s, 145 to 151 s of user CPU, 2.0 to 2.3
times 5.1's tests. CI then measured `selfhost.rs` at 35.59 s on Linux,
29.54 s on macOS and 76.59 s on Windows (all 16 jobs green), Windows about
2.1 times Linux in every test, 5.1's included: past the spec's minute on
Windows (risk 4). Of the options measured (an optimized debug runtime;
std resolved once and shared; part B's cuts; none), the user chose the
second, with no input cut.

**std resolved once, shared by every root of a process (differs).** The
spec has each program resolved as `check` resolves it, std included.
The hidden dump's path now resolves std once per process and mode and
shares it: S0's `ResolutionDumps` holds a `nova_resolver::SharedStd` (the
resolution of a program of no module of its own: std's exports, the
variants std globs, `missing`) and resolves each program with
`resolve_program_with_shared_std`, and N's `selfhost resolve` holds a
`SharedStd` (`share_std`) and resolves with `resolve_program_shared`. The
user's modules go through the same steps in the same order; where the
full resolution reads std's own state they read std's exports, rebased
past the user's definitions (`Exports::rebased`), and std's glob binds
the same names in each user module, std module by std module; std's own
modules are not resolved again. The result holds the user's definitions,
scopes, items and tests and every module's exports, all a program's part
prints, and is not for the type checker. A program reads the shared
state and never writes it: S0's is borrowed immutably, and N's is read
through copies. Should resolving std alone give a diagnostic beyond
parsing it, which would come between a program's own, or find a `@test`
function, which the full resolution lists with a program's tests and
the shared one would drop, every program is resolved in full on both
sides (no std file holds either; a std's diagnostic and its test are all
std could add to a program's part that the shared path leaves out, its
definitions, scopes and items being std's part's). nova-resolver's test
`std_with_a_diagnostic_or_a_test_is_resolved_for_each_program` builds std
from altered text (`core` defining a name twice, `ui` marking a function
`@test`) and checks the shared dump is the full one under `check` and
test mode; the harness runs N with `--std` pointed at such a copy and
checks each program keeps std's E0002 or lists std's test and is
otherwise S0's; with a copy whose `ui` ends with a `pub trait`, which
no std module exports, it checks each program importing `ui` lists the
trait with the id N's std part gives it and is otherwise S0's, so a
trait's rebasing is compared as a function's and a type's are. `nova check`, `nova build`, `nova test` and the language
server keep `resolve_program_with_std`; the resolver's only other change
is `variant_shadowing` split around the variants it reads
(`variant_shadowing_of`), and `FileDb` derives `Clone`, so that S0's dump
registers std's files once and copies them for each root. The driver's
test `std_resolved_once_dumps_what_std_resolved_for_each_program_dumps`
compares S0's shared and unshared dumps byte for byte over the corpus
and the cases of each mode (`check`, the WASM target, test mode; 463,
69 and 135 roots), in order and reversed; the differential compares N's
shared dumps with S0's. The harness also runs std's three parts side by
side and the corpus's packages in three processes a side, root `i` in
process `i % 3`.

Per root, S0 resolves the loose corpus about six times as fast (4.7 s of
CPU to 0.5 s), N about three times (9.6 s to 3.0 s); the packages, whose
roots are slow to parse, little faster. `selfhost.rs` locally, measured
on one machine state with the binaries of both trees built: before, 40.6,
40.3 and 40.8 s (126 to 128 s of user CPU); after, 28.5, 27.3 and 27.0 s
(83 to 87 s), 1.5 times 5.1's 27 tests alone (18.1, 18.2 and 18.0 s). (An
earlier, slower state of the machine gave 47.3 to 49.2 s before and 34.0
to 37.4 s after.) The rest is mostly N's debug runtime parsing: std's 21
modules once per process (0.36 s, 49 processes) and the package roots
that reach most of N. A projection from these local times by the ratios
of CI to the local time before (Linux 0.88, macOS 0.73, Windows 1.89)
put CI at 24 to 25 s on Linux, 20 to 21 s on macOS and 51 to 54 s on
Windows. It was wrong: CI measured more, below.

**After the first fact-check** (its fixes, two tests and a case, and the
fuzzer's manifest forms and package inputs under test mode and for the
WASM target), locally, on a slower machine state than the figures above:
`selfhost.rs` took 30.7, 31.8 and 30.8 s before and 31.8, 30.9 and
31.7 s after; with the fuzzer as it was, 32.1, 31.6 and 31.3 s, so the
fuzzer's new inputs cost no measurable time.

**CI, measured** (the figures that count; the local ones above are only
local). Run 41813500363, before std was shared: `selfhost.rs` 35.59 s
on Linux, 29.54 s on macOS, 76.59 s on Windows. Run 42006844810, on the
tree after the first fact-check's fixes: 37.90 s on Linux (42 tests),
35.77 s on macOS (40) and 60.48 s on Windows (36: one test failed early,
the comparison §7 now settles, so the full run is likely slightly
longer). Linux measured 37.9 s where the local projection said 24 to 25
s, and Windows about 1.6 times Linux, not the 1.89 times the local time
it was projected by. Sharing std took Windows from 76.59 s to about 60 s.

**The user's decision (differs from the spec's risk 4, which asks the
user when CI exceeds a minute on a system):** asked with these figures,
the user accepted about 60 s on Windows, with no coverage cut. The real
PR's CI is read for all three systems and compared with these figures;
should `selfhost.rs` exceed 70 s on Windows, the user is asked again.

**After the second fact-check** (its fixes: every resolution dump settled
where it is run, three tests, five argument shapes and one assertion),
locally, one machine state, three runs each: `selfhost.rs` 32.6, 33.4 and
34.7 s before, 33.0, 33.4 and 34.6 s after (100.6 to 106.0 s and 102.8
to 106.4 s of user CPU): no measurable time added.

### 10. The live heap (spec §4.4, phase plan decision 8)

N resolving its largest inputs, measured locally as the research did
(facts §6), with the debug runtime CI uses and a release runtime
(`libnova_runtime.a` built `--release` into an isolated target dir and
passed as `NOVA_RUNTIME_LIB`); peak RSS and wall time are the best of
three by `wait4`; the pauses are timed with gdb from the entry of
`nova_gc_collect_roots` to the return of `collect`, which ends with the
mark and `Pages::sweep` (inlined in the release runtime, so the pause
ends at the return of the allocation that triggered it); the live heap
is the largest `NOVA_GC_DEBUG` reports after a collection. The script is
in the 5.2b plan's last task.

| input | runtime | wall | peak RSS | collections | median pause | p95 | max | total GC | max live |
|---|---|---:|---:|---:|---:|---:|---:|---:|---:|
| `selfhost/src/main.nova`, a package root, all 23 of N's modules | debug | 1.08 s | 36.9 MB | 15 | 15.3 ms | 70.3 ms | 93.5 ms | 396 ms | 10.0 MB |
| std's native part (`--std-part`) | debug | 0.48 s | 16.8 MB | 12 | 10.7 ms | 32.1 ms | 47.1 ms | 185 ms | 6.7 MB |
| std's WASM part (`--target wasm --std-part`) | debug | 0.55 s | 19.0 MB | 12 | 9.7 ms | 30.1 ms | 44.3 ms | 170 ms | 5.5 MB |
| `selfhost/src/main.nova` | release | 0.26 s | 36.1 MB | 15 | 3.9 ms | 18.1 ms | 29.0 ms | 105 ms | 10.0 MB |
| std's native part | release | 0.11 s | 16.0 MB | 12 | 2.4 ms | 8.0 ms | 11.1 ms | 47 ms | 6.7 MB |
| std's WASM part | release | 0.13 s | 18.2 MB | 12 | 2.5 ms | 7.2 ms | 9.9 ms | 41 ms | 5.5 MB |

These are the split's figures, with std resolved once (§9). Before it,
part B measured the root at 32.8 MB of peak RSS and a 102.3 ms longest
pause with the debug runtime, 31.8 MB and 29.8 ms with the release one:
std's state, resolved alone and kept for every root, adds about 4 MB of
peak to a run of one root, and the pauses are the same within noise.

No threshold is set. The pauses grow with the live heap, about 9 ms a
live MB at the largest with the debug runtime and 3 ms with the release
one, as the research found (7 to 8 and 2 to 3), and the collector takes
31 to 41% of N's time on these inputs. For 5.3, which holds a type on
every node of std as well, a live heap five to ten times this one would
give pauses of about 0.5 to 1 s with the debug runtime and 0.15 to 0.3 s
with a release one: slower runs, not a failure, since N is a batch
compiler; the cost to watch is CI's time, not the pause.

## Consequences

- S0's hidden dump covers every program `nova check FILE` and `nova test`'s
  path take, packages, the WASM target and test mode; `nova check`, its
  help and every command line without `--dump-resolution` first are the
  base's.
- N loads and resolves the same roots as S0, packages, the WASM target and
  test mode included, byte for byte in the dump, on every input the
  differential holds; 5.2 is done, and 5.3 ports the type checker and HIR
  on top of it.
- N reads `nova.toml` through a reader of its own that refuses what it
  does not read; the package graph's diagnostics stay S0's until a driver
  needs them (5.5).
- The hidden dump resolves std once per process and mode, on both sides,
  a departure from the spec's per-program resolution that a test shows
  changes no byte (§9); with every input kept, CI measured `selfhost.rs`
  at 37.90 s on Linux, 35.77 s on macOS and about 60 s on Windows, which
  the user accepted; the real PR's CI is compared with these, and the
  user is asked again past 70 s on Windows (§9).
- The live heap is recorded (§10), with no threshold.
