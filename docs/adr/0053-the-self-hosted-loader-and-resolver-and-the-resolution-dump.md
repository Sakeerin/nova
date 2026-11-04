# ADR 0053 — The self-hosted loader and resolver, and the resolution dump

## Status

Accepted (2026-11-02). Branch `claude/loving-archimedes-frtf5q-52a`
(`docs/superpowers/specs/2026-11-02-phase-5-2-the-loader-and-the-resolver-design.md`
§3). §1 is the resolution dump, its grammar and S0's `nova check
--dump-resolution`; §2 departs from the spec on its 11 codes; §3 to §5
are N's loader, resolver and dump, and each Rust idiom's translation; §6
std on disk; §7 S0's insertion order; §8 the fuzzer; §9 the comparison
and its time; §10 the speed; §11 the rule that a change to resolution
changes both compilers. 5.2b (packages, the WASM target, test mode, the
live heap) writes ADR 0054.

## Decision

### 1. The resolution dump

`nova_resolver::dump::dump_program` writes a program's part,
`nova_resolver::dump::dump_std` std's part; `nova_driver::ResolutionDumps`
runs `check`'s pipeline up to the resolver for each root and dumps it,
with std's modules lexed and parsed once for all of them (§9), and
`nova_driver::dump_std_resolution` resolves a program of no module of its
own and dumps std's part. The hidden `nova check --dump-resolution FILE…`
prints, for each file, `== FILE` (the path as given) and its program's
part, or `IOERR` and the error's kind for a root it cannot read, then
exits 2 after the rest, as `parse --dump-ast` does (ADR 0052 §1). `nova
check --dump-resolution --std` prints std's part. Both take `--target`;
5.2a compares the native target. With neither a file nor `--std`, or with
both, it prints one error line to stderr, nothing to stdout, and exits 2.

**Hidden, and `check` as before.** The form is taken only when
`--dump-resolution` is the first argument after `check`: `main` looks
for it before clap parses the command line, and parses what follows with
a parser of its own (`DumpResolutionCmd`), while `check`'s `CheckCmd` is
the base's, field for field. So every other `nova check` command line,
under any `argv[0]` and in a terminal, gives what the base gives, byte for
byte on both streams and in the exit code: its errors, tips ("a similar
argument exists" names no hidden flag), usage lines, the order in which
clap reports a fault, and `--help` and `-h`, which `check`'s help never
lists the flags in. The first version declared the flags in `CheckCmd`
and re-parsed the command line without them to refuse; clap then
suggested them, parsed `--std=1` and `--std --std` as their own errors,
reported `--target bogus` before a second file, and printed help for
`check --std --help`, which the first fact-check found in 17 of 24 shapes.
After the flag, the form takes `--std`, `--target native|wasm` (or
`--target=…`) and files, each option once, `--` before a file that starts
with `-`, an empty file as one that is not there; it refuses anything
else, `--help`, `-h` and `--dump-resolution` again included, with one
clap error under the usage `nova check --dump-resolution …` and the exit
code 2. `--dump-resolution=…` is not the form, so the base's refusal of
it stands. 5.1's hidden `parse --dump-tokens` and `--dump-ast` are still
arguments of `parse` itself, so `parse --dump` is told "a similar argument
exists: '--dump-ast'": a known 5.1 leak (ADR 0052), in the base, left as
it is; this mechanism does not reach `parse`.

**What runs.** `check`'s pipeline for a loose program: the loader
(`load_program`, breadth first from the root, sibling files), then, if no
error so far, the `@test` functions stripped, then
`resolve_program_with_std` with the target's std, parsed once for all the
roots (§9). The type checker does not run. A program the
loader stops leaves the dump with its modules and the diagnostics so far
(`resolved=false`). A root directly in a package's `src/` or `tests/`
prints the one line `UNSUPPORTED package`: packages are 5.2b's.

**Two parts.** std's modules come after the user's, and their resolution
is the same in every program for a target: std's scopes hold no user
name. So std's part is printed once, and a program's part prints only
what is its own. A unit test resolves a program of two modules and checks
that std's part of its resolution is std's part of a program of none.

**Ids.** Within a part, modules and merged items are numbered from 0: a
program's part numbers the user's modules, std's part std's. A user
definition is `u<n>`, its `DefId`; a std definition is `s<n>`, its
`DefId` less the first std definition's. A program's part says where std
begins in its numbering, `(StdStart module= def= item=)`, so the global
`DefId` of `s<n>` is `def + n`, and a std definition's `item=` in std's
part plus the program's `item` is its merged item index.

**Labels.** A place, a span in a file, names the file by a label: a user
module's label (a loose file's stem), or a std module's name in the file
database, `<std/core>`. A file with no label prints `?`.

**The grammar.** Every line ends with `\n`, the last included. `I(d)` is
`d` times two spaces. As in ADR 0052, a line's `)` comes right after its
last child's.

```
output     = { "== " path "\n" part }            -- nova check --dump-resolution FILE…
           | std diagnostics                     -- nova check --dump-resolution --std
part       = "IOERR " kind "\n" | "UNSUPPORTED package\n" | program diagnostics

program    = "(Program target=" target " resolved=" bool
             { "\n" module(1) }
             [ "\n" stdstart { "\n" def } { "\n" scope } { "\n" exports }
               { "\n" items } { "\n" split } { "\n" test } ]  -- when resolved
             ")\n"
std        = "(Std target=" target " modules=" uint " defs=" uint " items=" uint
             { "\n" module(1) } { "\n" def } { "\n" scope } { "\n" exports }
             { "\n" items } { "\n" missing } ")\n"
target     = "native" | "wasm"

module(1)  = I(1) "(Module " uint " " string " package=" ("None" | uint)
             { "\n" import } { "\n" importable } ")"
import     = I(2) "(Import " string " " (uint | "Reported") ")"   -- sorted by name
importable = I(2) "(Importable " string " " uint ")"              -- in the loader's order
stdstart   = I(1) "(StdStart module=" uint " def=" uint " item=" uint ")"

def        = I(1) "(Def." kind " " id " " string " " place fields { "\n" variant } ")"
kind       = "Fn" | "Sum" | "Record" | "Const" | "Trait"          -- fields: " item=" uint
           | "Method"      -- " item=" uint " method=" uint " owner=" ("Impl" | "TraitDefault")
           | "ExternFn"    -- " item=" uint " fn=" uint
           | "AssocType"   -- " trait=" id
variant    = I(2) "(Variant " string " " place " arity=" uint ")" -- a Sum's, in order

scope      = I(1) "(Scope " uint { "\n" binding } ")"   -- values, types, traits, each
                                                        -- sorted by name (bytes)
exports    = I(1) "(Exports " uint { "\n" binding } ")" -- values, types, traits, each
                                                        -- in the order exported
binding    = I(2) "(Value.Def " string " " id ")"
           | I(2) "(Value.Variant " string " " id " " uint ")"   -- the sum, the index
           | I(2) "(Value.Builtin " string " " word ")"          -- Builtin's variant
           | I(2) "(Type " string " " id ")"
           | I(2) "(Trait " string " " id ")"
items      = I(1) "(Items " uint ".." uint " " uint ")"  -- a run of merged items of
                                                        -- one module: start..end module
split      = I(1) "(Split " place " kind=" ("Call" | "Index" | "Minus" | "Value") ")"
test       = I(1) "(Test " id " " string " should_panic=" bool ")"
missing    = I(1) "(Missing " string " " string ")"      -- sorted by name

diagnostics = "(Diagnostics" { "\n" diagnostic } ")\n"   -- every one, in order
diagnostic = I(1) "(" severity " " string " " string     -- code, message
             { "\n" label } { "\n" note } { "\n" fix } ")"
severity   = "Error" | "Warning" | "NoteDiagnostic" | "Help"
label      = I(2) "(" ("Primary" | "Secondary") " " place " " string ")"  -- in order
note       = I(2) "(Note " string ")"
fix        = I(2) "(Fix " string { "\n" edit } ")"
edit       = I(3) "(Edit " place " " string ")"

id         = "u" uint | "s" uint
place      = (string | "?") ":" uint ".." uint            -- a file's label, a span
string     = ADR 0052's: in double quotes, each char below 0x20, 0x7F,
             '"' and '\' as "\x" hh
bool       = "true" | "false"
```

A program's part holds only its own: its modules' scopes and exports, the
definitions before `StdStart`'s `def`, the items before its `item`, the
user's newline splits (what the type checker's E0103 reads), and its
tests (none under `check`, which strips them). `missing` is the target's,
the same in every program, so std's part holds it. Every diagnostic is in
the program's part, the loader's first, then the resolver's in S0's order:
std's parse errors, pass 1 module by module, E0090, pass 2 (imports),
E0102.

**One example of every form**, each S0's dump of the sources after it,
the first file the root, reduced to the lines of those forms and their
parents, each line closed as if the others were not there:

```
;; Program, Module, StdStart, Def.Fn, Scope, Value.Def, Exports, Items, Diagnostics:
;; hello.nova: fn main() {}\n
(Program target=native resolved=true
  (Module 0 "hello" package=None)
  (StdStart module=1 def=1 item=1)
  (Def.Fn u0 "main" "hello":3..7 item=0)
  (Scope 0
    (Value.Def "main" u0))
  (Exports 0)
  (Items 0..1 0))
(Diagnostics)

;; Import, Importable, Def.Sum, Variant, Def.Trait, Value.Variant, Type, Trait:
;; main.nova: import geometry\nfn main() {}\n
;; geometry.nova: pub fn area() -> Int { 1 }\n
;;     pub type Shape = | Circle(Float) | Dot\npub trait Show {}\n
(Program target=native resolved=true
  (Module 0 "main" package=None
    (Import "geometry" 1)
    (Importable "geometry" 1))
  (Module 1 "geometry" package=None
    (Importable "main" 0))
  (Def.Sum u2 "Shape" "geometry":36..41 item=3
    (Variant "Circle" "geometry":46..52 arity=1)
    (Variant "Dot" "geometry":62..65 arity=0))
  (Def.Trait u3 "Show" "geometry":76..80 item=4)
  (Exports 1
    (Value.Def "area" u1)
    (Value.Variant "Circle" u2 0)
    (Value.Variant "Dot" u2 1)
    (Type "Shape" u2)
    (Trait "Show" u3))
  (Items 0..2 0)
  (Items 2..5 1))

;; Def.Record, Def.Const, Def.AssocType, Def.Method, Def.ExternFn:
;; defs.nova: type Shape = | Circle(Float) | Dot\nrecord P { x: Int }\n
;;     const K: Int = 1\ntrait Show {\n    type Item\n    fn twice(self) {}\n
;;     }\nimpl Show for P {\n    fn twice(self) {}\n}\nextern "C" {\n
;;         fn abs(x: Int) -> Int\n}\nfn main() {}\n
(Program target=native resolved=true
  (Def.Record u1 "P" "defs":42..43 item=1)
  (Def.Const u2 "K" "defs":61..62 item=2)
  (Def.AssocType u4 "Item" "defs":94..98 trait=u3)
  (Def.Method u5 "Show::twice$default" "defs":106..111 item=3 method=1 owner=TraitDefault)
  (Def.Method u6 "P.Show.twice" "defs":148..153 item=4 method=0 owner=Impl)
  (Def.ExternFn u7 "abs" "defs":185..188 item=5 fn=0))

;; Value.Builtin, and std's definitions by their ids in a program's scope:
;; hello.nova: fn main() {}\n
(Program target=native resolved=true
  (Scope 0
    (Value.Variant "Some" s0 0)
    (Value.Builtin "println" Println)
    (Type "Option" s0)))

;; Split:
;; split.nova: fn f() {\n    g\n    (1)\n}\nfn main() {}\n
(Program target=native resolved=true
  (Split "split":19..20 kind=Call))

;; Error, Primary, Secondary:
;; dup.nova: fn dup() {}\nfn dup() {}\nfn main() {}\n
(Diagnostics
  (Error "E0002" "duplicate definition of `dup`"
    (Primary "dup":15..18 "redefined here")
    (Secondary "dup":3..6 "first defined here")))

;; Note:
;; main.nova: fn println() {}\nfn main() {}\n
(Diagnostics
  (Error "E0002" "duplicate definition of `println`"
    (Primary "main":3..10 "redefined here")
    (Note "`println` is a compiler builtin")))

;; Fix, Edit:
;; main.nova: @tset\nfn main() {}\n
(Diagnostics
  (Error "E0082" "unknown attribute `@tset`; known attributes are: test"
    (Primary "main":1..5 "unknown attribute")
    (Fix "change `tset` to `test`"
      (Edit "main":1..5 "test"))))

;; Warning:
;; main.nova: type Shade = | Red | Ok(Int)\nfn main() {}\n
(Diagnostics
  (Warning "E0102" "variant `Ok` hides std's `Result::Ok`"
    (Primary "main":21..23 "this variant")
    (Note "in this module the bare name `Ok` now means this variant; std's is `Result::Ok`")))

;; Program resolved=false: a program the loader stops:
;; broken.nova: fn main( {\n
(Program target=native resolved=false
  (Module 0 "broken" package=None))
(Diagnostics
  (Error "P0001" "expected identifier (in parameter name), found `{`"
    (Primary "broken":9..10 "here"))
  (Error "P0001" "expected `)` (in function parameters), found `{`"
    (Primary "broken":9..10 "here")))

;; Import Reported: an import the loader refused (E0093):
;; main.nova: import ui\nfn main() {}\n
;; ui.nova: pub fn f() {}\n
(Program target=native resolved=false
  (Module 0 "main" package=None
    (Import "ui" Reported)))
(Diagnostics
  (Error "E0093" "`ui` is the name of a std module, so no module may take it: rename ui.nova"
    (Primary "main":7..9 "std's module, not this one")))

;; Std, from nova check --dump-resolution --std:
(Std target=native modules=21 defs=713 items=548
  (Module 0 "$std.core" package=None)
  (Def.Sum s0 "Option" "<std/core>":1717..1723 item=0
    (Variant "Some" "<std/core>":1758..1762 arity=1)
    (Variant "None" "<std/core>":1796..1800 arity=0))
  (Def.Method s2 "Option_T.is_some" "<std/core>":2101..2108 item=2 method=0 owner=Impl)
  (Items 0..56 0))
(Diagnostics)

;; IOERR and UNSUPPORTED: a root that is not there, and a package's src/main.nova:
;; pkg/src/main.nova: fn main() {}\n
;; pkg/nova.toml: (empty)
== missing.nova
IOERR NotFound
== pkg/src/main.nova
UNSUPPORTED package
```

The forms a loose native program never prints are 5.2b's: a module's
package key comes with packages, `(Missing` with the WASM target, `(Test`
with test mode; `NoteDiagnostic` and `Help` are severities the resolver
never gives. An import the loader reported, `(Import "x" Reported)`, is a
loose program's too, when E0093 refuses a sibling named for a std module;
a package's E0004 and dev-dependency print it too. `nova_resolver::dump::KINDS`
lists the 43 forms 5.2a's programs print and `LATER_KINDS` the others; a
unit test checks that each kind of definition, of value and of diagnostic
has a form in one of them, and the differential checks that every form
of `KINDS`, and none of `LATER_KINDS`, is in S0's dump of std, a loose
corpus program or a resolver case, and that every form those dumps print
is in one of the lists (`listed`: a field whose value is a number or an
id stands for itself), so a form added to the dump must be listed.

**Sizes.** std's native part is 7,728 lines (320 KB), 713 definitions
and 548 items, and S0 debug prints it in 0.082–0.142 s (§10). A typical program's part
is about 180 lines (6 KB), almost all of it its module's bindings of
std's names and the builtins; the corpus's largest loose program,
`tests/runtime/modules/main.nova` with its two siblings, is 561 lines.
S0 debug dumped the 251 loose corpus programs in about 17.8 s in one
process when it parsed std again for each, about 0.065 s of each (at
part A's head); with std parsed once per process (§9) it takes 3.6–4.3 s
(§10).

### 2. Eight of the 11 codes occur in 5.2a (differs from the spec)

The spec (§3.5, §3.7) asks that all of the resolver's 11 codes occur among
5.2a's inputs. Only 8 can: 5.2a compares `check`'s pipeline on the native
target, and of the 11,

- E0084 (a `@test` function's shape) and E0085 (an unknown `@test`
  argument) are given only for a function that keeps its `@test`, and
  `check` strips every `@test` function before resolving
  (`strip_test_functions`), so they come with 5.2b's test mode;
- E0090 (a user `extern` block) is given only on the WASM target, so it
  comes with 5.2b's `--target wasm`.

The user's split (the spec's decision 4) put both the 11 codes in 5.2a
and test mode and the WASM target in 5.2b, which the prototype found
cannot both hold; asked, the user chose on 2026-11-03 to move the three
codes to 5.2b with test mode and the WASM target.

N ports all three paths (`validate_test_function`'s E0084 and E0085, and
`resolve_program_for`'s E0090), unreachable natively. The harness asserts
that the other 8 (E0001, E0002, E0082, E0083, E0089, E0092, E0102, E0900)
occur among the name fuzzer's inputs from its fixed seeds, each at least
twice (`NATIVE_RESOLVER_CODES`; §8 gives the counts), and that none of the three does
(`LATER_RESOLVER_CODES`) there or in the dumps of the corpus and the
cases, though cases hold the programs that would give them; 5.2b moves
them to the first list. The loader's
codes on the loose path are L0001, the P-codes and E0093; E0004 and the
two noted E0001s are a package's, 5.2b's too.

### 3. N's loader

`selfhost/src/loader.nova` ports the driver's `load_program` and `Loader`
on the loose path, `strip_test_functions` (with `keep_root_tests` false,
as `check` calls it) and `package_of`. From the root it reads, lexes and
parses each module breadth first, with 5.1's lexer and parser; each
single-segment import is a file `<name>.nova` in the root's directory,
found by one listing of the directory per load (`has`: the name exactly,
not when only a name differing in case is listed, otherwise a read of the
path), and refused with E0093 when the name is `dom` or `ui`
(`RESERVED_MODULE_NAMES`). The root's directory is `Path::parent`'s:
the root less its last component, less the `/`s and `.` components that
end what is left, but not a leading `/` or `./` (`sub/./main.nova` is in
`sub`, `./main.nova` in `.`), and a sibling's path is that directory
joined with its file name, which E0093 prints. A root with no directory
part has the directory `""`, which both sides list as `.`. A root is in
a package (`package_of`, N's `in_package`) when its directory is named
`src` or `tests` and the path `nova.toml` in the directory above is a
file or a link to one, asked of the path as `Path::is_file` asks, not
looked for in a listing: so on a case-insensitive disk (macOS's and
Windows' by default) a `Nova.toml` makes a package on both sides, and a
directory that can be searched but not listed still holds its manifest;
for `src/main.nova` the path is `nova.toml`. Nova's std has no query of
a path's kind, so N asks `exists` and whether the path lists as a
directory: as the listing's `DirEntry.is_file` did, it takes a pipe
named `nova.toml` for a file, and a `nova.toml` directory it may not
list too, where S0 takes neither; no test makes either. A
module that cannot be read is skipped, so its
import is the resolver's "cannot find module"; a root that cannot be read
is `IOERR`. `ImportTarget`, what an import names, lives here, beside
the loader that decides it (S0 keeps it in `nova-resolver`), so the
loader builds and is tested without the resolver. Each module's label is
its file's stem; each module's
importable list is every loaded module beside it, by stem, in byte order,
less itself and the reserved names, as `find` itself decides it. The
diagnostics are the lex errors (L0001, "here"), the parse errors with
their codes and notes, and E0093, in load order; any error stops the
program before the resolver, as in S0.

### 4. N's resolver

`selfhost/src/resolver.nova` ports `nova-resolver`'s
`resolve_program_for` function by function, with S0's names:
`std_module`, `resolve_program_for`, `collect_item` (definitions and
their ids by `Vec` length, `type_full_name`'s mangling, the trait
defaults' `Trait::m$default`, the associated types, the attribute checks,
the tests, the merged item list), `reject_reserved_type_name`,
`unknown_attribute`, `validate_attrs_reject_test`,
`validate_test_function`, `test_shape_violations`, E0090, `resolve_import`,
`make_public`, `would_receive`, `binds`, the fix-dropping step,
`import_std_module` (std's glob, last and lowest-priority; `IMPORTED_STD`
nowhere, `STD_INTERNAL` into std alone), `std_globbed_variants`,
`variant_shadowing` (E0102), `missing_names`, `insert_value`,
`insert_type`, `bind_value`, `bind_type`, `bind_trait`, `unsupported`,
`path_tail`. Every message, label, note and fix is S0's word for word.
`builtins.nova` holds the builtin table (the 118 variants' Rust names,
which the dump prints, and their names in a program; `GLOBAL`, `STD_ONLY`
and `WASM_ONLY` as lists of variant names) and the std tables
(`STD_MODULES` and `WASM_STD_MODULES` with each source's path under std's
directory, `IMPORTED_STD`, `RESERVED_MODULE_NAMES`, `STD_INTERNAL`,
`STD_TEST_MODULE`, `RESERVED_TYPE_NAMES`); `diagnostic.nova` the
`Diagnostic` record, `suggest.rs`'s `distance`, `closest` and `change_to`,
and `lines.rs`'s `keyword_start`. `resolver.nova` is 1,486 lines (1,224 code lines), under
the 1,500 past which the brief asked for a split, so it is one file.

**Sizes,** in code lines as 5.1 counted them (a line that is neither
blank nor a comment line, one starting with `//`, and nothing from a Rust
file's first `#[cfg(test)]` on): N's are
`diagnostic.nova` 189, `builtins.nova` 473, `loader.nova` 341,
`resolver.nova` 1,224 and `resolution_dump.nova` 369, 2,596 in all, and
about 100 more in `main.nova`'s command; S0's are `nova-resolver`'s
`lib.rs` 1,962 (1,921 before 5.2a; the builtin and std tables about 465 of
them), `dump.rs` 552, and about 250 of `program.rs`'s 646 for the loose
path (its `load_program`, `Loader` and their types, less the package and
doc-test arms). N's tests are 277 more.

**Each Rust idiom's translation**, one way everywhere (ADR 0052's table
holds; these are new):

| Rust | Nova |
|---|---|
| `DefId(u32)`, `ModuleId(u32)`, `Option<u32>` package keys | `Int`s; -1 for none |
| `Builtin`, a 118-variant enum, and `name()` | its index in `BUILTIN_VARIANTS`, the macro's declaration order; `BUILTIN_NAMES[i]` its name |
| `Res`, `DefKind`, `MethodOwner`, `ImportTarget` | sums with prefixed variants (`ResDef`, `DefKindMethod`, `ImportTargetReported`), named fields positional |
| a `Span`'s `FileId` | the module's file label, passed beside the tree's spans (`file`) and kept in each `Def`, `VariantDef`, `Label` and `Edit` |
| `FxHashMap<String, _>` namespaces, looked up | `Map<String, _>`, never iterated into output; the dump sorts its keys (`sort_strings`, a merge sort) |
| `Exports`' maps and S0's new name `Vec`s (§7) | the same: maps for lookups, `value_names`/`type_names`/`trait_names` for every loop |
| `IndexMap<String, Span>` first definitions | `Map<String, Span>`: only looked up |
| `&mut definitions.modules[mid]` | `definitions.modules.at(mid)` held in a `let mut`: a record is a reference |
| `Diagnostic::error(…).with_primary_label(…)` | `error_at(code, message, file, span, label)`, then `d.add_note(…)` on a variable: a builder's `mut self` call on a temporary is E0060 |
| `diagnostics[before..].iter_mut().for_each(\|d\| d.fixes.clear())` | `clear_fixes(diagnostics, before)` |
| a constant array read in a loop (`BUILTIN_VARIANTS[i]`) | read once into `BuiltinTable` (`builtin_table()`): a constant array is built again each time it is read, which made the first build 9× slower than this one |
| std parsed in every `resolve_program_for` | parsed once per process (`load_std`, `std_module`), handed in as `StdSet`; its diagnostics are given first, as S0 gives them |
| `Path::parent`, `Path::join`, `file_name`, `file_stem` | `path_parent`, `path_join`, `file_name`, `module_name` over `/`, trimming `.` components and repeated `/`s as `Components` does |
| `VecDeque` of places | a `Vec` and its head's index; a loose place is its stem, since every module of a loose program is in the root's directory |

Not ported: the language server's import occurrences (`imports`,
`Occurrence`), which no dump prints, and `ResolveResult`/`resolve`, the
one-file wrapper only S0's tests call.

### 5. N's dump and command

`selfhost/src/resolution_dump.nova` ports `dump.rs`'s `dump_program`,
`dump_std`, `first_std_def` and the `Dump` writer, and prints §1's grammar
byte for byte, its strings through 5.1's `push_escaped`. `selfhost resolve
--std DIR FILE…` prints `== FILE` and each root's part (`IOERR` and the
kind for a root it cannot read, then exit code 2 after the rest; `UNSUPPORTED
package` for a root directly in a package's `src/` or `tests/`), and
`selfhost resolve --std DIR --std-part` std's part; without `--std DIR`,
or with neither a root nor `--std-part`, or with both, it prints its
usage line to stderr, nothing to stdout, and exits 2, as S0's form does
with its own line. `main.nova`'s `dump_resolution` is the
driver's: load, stop on any error, strip the `@test` functions, resolve.
N's own tests (`loader_tests.nova`, `resolver_tests.nova`, in `src/` as
5.1's are): a loose program's breadth-first order, its imports, E0093 and
the importable lists; what `check` strips; the paths; a glob's clashes in
declaration order; the mangled method names; a builtin's name redefined;
`make_public`'s edits; the attribute fix and `closest`'s edges. `nova test`
in `selfhost/` runs 25. N takes no `--target`: 5.2a resolves natively, and
`resolver.nova`'s WASM paths (E0090, E0092's WASM arm, `missing`) wait for
5.2b's flag. Its command line is a test tool's: an empty FILE is a root
that is not there, `== ` and `IOERR NotFound`, exit code 2, as in S0's
form (§1), and the roots test compares it on both sides; a `--` is a
root named so (`IOERR NotFound`), where S0's form skips it, and the
harness passes none.

**Differences found while building.** None in output on the inputs:
N's first build equalled S0 on std's part, all 251 loose corpus programs
and the cases, and no input of the fixed seeds or of the longer searches
(`NOVA_SELFHOST_FUZZ=7:1500` and `11:5000`, 6,500 name-fuzzed programs
beside the soup and the mutations) has shown one. Before the split, a
probe of the root's spellings found one, and two more are settled or
recorded here:

- **A root with a `.` component** (`sub/./main.nova`): `Path::parent`
  gives `sub`, and N gave `sub/.`, so E0093 named `sub/./ui.nova` where
  S0 names `sub/ui.nova`. N's `path_parent` and `file_name` now trim as
  `Components` does (§3), and `a_root_spelled_any_way_names_its_siblings_alike`
  compares eleven spellings: bare, `./`, `.//`, `sub/`, `./sub/`,
  `sub/./`, `sub//`, `./sub/.//`, `sub/../sub/`, `../sub/` and an absolute
  path.
- **Windows paths.** S0's resolution output holds one path: E0093's
  message names the sibling it refuses as `Path::join` writes it, the
  root's directory and the file name, `dir\ui.nova` on Windows, where N
  writes `dir/ui.nova`. (The `== FILE` header is the root as given on both
  sides when the root is UTF-8, below, and modules are labelled by stem.)
  Settled in the harness, not in
  either compiler: a case directory holding a `dom.nova` or `ui.nova` runs
  from inside that directory, its root named `main.nova`, so the path has
  no directory part and is `ui.nova` on every system, and the spellings
  test compares only the bare root on Windows. The other ways were worse:
  S0 printing `/` would change `nova check`'s message on Windows against
  the base, which this phase must not; N cannot tell the system it runs
  on (Nova has no such query); and running every case from its own
  directory costs about 10 s of CPU for N's std per process. So
  `selfhost resolve` given a root with a directory part still prints
  `/` on Windows: 5.5's driver, which owns paths and the command line,
  settles it. This was reasoned from `std::path` (on Windows `Path::join`
  pushes `\` after a directory that does not end in a separator, and
  `Path::parent` splits at `/` and `\` alike); the pull request's Windows
  leg is the proof. CRLF is the other Windows difference: git gives
  `cases/` no `.gitattributes` rule, as for 5.1's cases, so a Windows
  checkout compares the cases with `\r\n`, which both sides read alike;
  `the_resolution_dumps_agree_on_the_cases_with_windows_line_endings`
  compares a CRLF copy of them on every system. The same checkout gives
  `\r\n` to any checked-in text a test compares byte for byte: a
  test that reads a fixture with `include_str!` or `read_to_string` and
  compares it with output fails there. `nova-resolver`'s
  `dump_test_program.txt` did (the first fact-check reproduced it with a
  CRLF copy); its test now reads each `\r\n` in the fixture as `\n`, and
  `the_expected_program_part_may_have_windows_line_endings` checks that
  with a CRLF copy on every system. No other fixture that 5.2a reads is
  compared so: the harness's other reads normalise (`names_of`) or feed
  both sides the same bytes.
- **A file name that is not UTF-8** beside a root: S0's listing drops it
  and N's `read_dir` keeps it with U+FFFD in place of its bad bytes, so an
  import written with U+FFFD would find a module on N's side alone. No
  identifier holds U+FFFD, so no input reaches it; the spellings test puts
  one beside its roots on Linux (macOS refuses such names) and the dumps
  agree. A root that is a directory is `IOERR Other` on both sides on
  Linux (macOS reads it the same way); on Windows opening a directory
  fails with `PermissionDenied` instead, for S0 at least, and the harness
  compares no such root. An import of a directory named `x.nova` is
  "cannot find module" on both sides
  (`an_import_of_a_directory_finds_no_module`).
- **A root whose path is not UTF-8** (`r\xff.nova`, `sub\xfe/main.nova`,
  on Linux): S0 opens it and dumps its program, under a header with
  U+FFFD in place of the bad bytes; N's arguments reach it as UTF-8 with
  U+FFFD, so it opens another name, prints `IOERR NotFound` and exits 2.
  ADR 0051 records the same for 5.1's dumps. Nova has no byte paths
  before 5.5's driver, so it is recorded, not settled; no input of the
  differential has such a root, and
  `a_root_whose_path_is_not_utf8_is_dumped_by_s0_and_not_found_by_n` pins
  today's behaviour on Linux. A `nova.toml` that is a directory makes no
  package on either side: the header test compares such a root.

### 6. std on disk

N reads std's sources from `--std DIR` (the spec's decision 2), at the
paths `builtins.nova`'s tables give (`core/lib.nova`,
`ui/backend/native.nova`, …), labels each module as the driver registers
it (`<std/core>`), and parses the target's modules once per process; the
harness passes the checkout's `std/`, which is what S0 embeds
(`std/lib.rs`'s `include_str!`s), CRLF and all on a Windows checkout. A
file it cannot read stops `selfhost resolve` with a message and exit
code 2. In 5.5 the driver finds std next to its executable or in
`$NOVA_HOME`, as the language server does.

### 7. S0's insertion order

A glob import's E0002s for the names it clashes with followed `FxHash`
order (`beta, delta, epsilon, alpha, gamma` for five functions), which N's
seeded `Map` cannot reproduce. `Exports` keeps each namespace's names in a
`Vec` beside its map, in the order exported, which is the order declared,
and `resolve_import`'s glob follows them (the spec's decision 3, §3.1); no
crate is added. No other output changed: `import_std_module` and
`missing_names` bind with `or_insert`, whose result does not depend on
order, and the namespaces are looked up, never iterated into output. The
test `a_glob_imports_clashes_are_reported_in_declaration_order` pins it,
and the CHANGELOG's Changed records it.

### 8. The name fuzzer

`fuzz::name_mutate` (`crates/nova-cli/tests/selfhost/fuzz.rs`) mutates a
loose corpus program that parses, one to three times, at the level of
names: a definition renamed to collide with another, a builtin, a std
name, a built-in type's name or a std-only builtin's; an item duplicated
or deleted; `pub` toggled; an import of a missing module, a sibling whole
or by a list, `dom`, `ui`, a nested path, an alias or itself; an unknown
attribute or a malformed `@test`; an item that is an alias, a sum hiding
std's variants, a trait twice, an `extern` block, a record named `Bytes`,
a `@test` impl block, impls for `()` and a function type. A mutation
that would not parse is skipped, so every input reaches the resolver,
which the harness asserts. Input `i` is written as `f{i:05}.nova` beside
four siblings (`NAME_SIBLINGS`): `shapes` and `helpers`, and, since the
first fact-check, `vault`, which keeps a private item of each kind a fix
can make public and one name twice, the second with its keyword on a
line of its own, and `keeper`, which imports `vault` whole and declares a
record named as one of `vault`'s private functions. Three imports reach
them: `vault::{hidden}`, whose fix is dropped since one item's keyword
cannot be found; `vault::{SEALED, Cell, Hue, Veiled}`, a fix per kind;
and `vault::{Kept}` with `keeper`, whose fix is dropped because it would
bind `Kept` in `keeper`'s types as well. The fact-check's mutants of N
that dropped one item instead of the whole fix, lost `make_public`'s
`const` arm, or made `binds` ignore the types namespace all passed: the
two siblings held one private function and nothing else, so no input
offered `make_public` another kind of item or a clash in another
namespace. Now the fuzzer alone catches those three, and the mutants that
reword E0083's "an impl block" or rename `type_full_name`'s `anon`
fallbacks; the resolver cases catch all of them too (§9). One kind it
cannot reach is a file system's shape (a `nova.toml` that is a
directory, or named in another case): the inputs are files beside fixed
siblings, so the header test and
`a_package_is_found_by_its_manifests_path_on_both_sides` hold those.
The fixed seeds 301–303 give 50 inputs each (150 in all; the codes
they give: E0001 46, E0002 63, E0082 14, E0083 8, E0089 10, E0092 13,
E0102 50, E0900 18), and the harness asserts each of the 8
at least twice, a floor the rarest, E0083, clears with 6 to spare. The inputs are the same on
every system: the files are picked from their sorted list, read with each
`\r\n` as `\n`, and the siblings are written from the harness's own
text; a test checks that a Windows checkout's corpus, listed in reverse,
gives the same inputs. `NOVA_SELFHOST_FUZZ=SEED:COUNT` adds COUNT
name-fuzzed programs to its longer search.

### 9. The comparison, and its time

`selfhost.rs` compares S0's and N's resolution dumps byte for byte, with
their exit codes, through the same `compare_batch` as 5.1's dumps:

- std's part, once (`std_resolution`);
- every loose corpus program, 251 at 5.2a: the corpus is every `.nova`
  file of the repository outside `selfhost/` and `cases/`, then N's own
  sources, and a loose program is every one of them but those directly in
  a package's `src/` or `tests/`, which are 5.2b's; in batches of 50 roots
  run side by side (`agree_on_resolution`), each root's part there;
- beside them, the 103 resolver cases in
  `crates/nova-cli/tests/selfhost/cases/resolve/` (`agree_on_resolve_cases`):
  88 the inline sources of `nova-resolver`'s and the type checker's tests
  (`<testfile>__<test>[_n]`), 15 written by hand (names without `__`) for
  the loader's edges and some of the resolver's: a sibling named for a std
  module, a sibling that fails to lex or parse, a parse error with a note,
  an import cycle, an import of a directory, an import whose case differs
  from its file's (which only a case-insensitive disk, macOS's or
  Windows', could find), a root that imports itself, a `@test` function
  that would collide, an empty file, and a `pub` fix dropped because it
  would bind a name twice, once through a glob import and once through a
  listed one; and, from the first fact-check, a fix dropped because one
  item's keyword is not on its name's line (a `const` beside a function),
  a fix dropped because it would bind a type twice, a `@test` impl block
  (E0083's "an impl block"), and impls for a function type and `()`
  (methods named `anon.…`). The note, the two imports and these four close
  gaps that breaking N's code on purpose found, where no input told the
  two sides apart. A
  file is a program, a directory one whose root is `main.nova`, and a
  directory holding a `dom.nova` or `ui.nova` runs from inside itself
  (§5); the same cases with `\r\n` line endings, on every system, the
  copy checked to hold them;
- the name fuzzer's 150 inputs, through `agree_on_inputs` with the third
  kind of dump, `Dumps::Resolution`, which writes them beside the
  siblings;
- the headers: a missing root, a package's root, a root that does not
  parse, a root beside a `nova.toml` that is a directory, and a root
  whose contents are not UTF-8, exit code 2; and no root at all, refused
  by both sides with nothing on stdout; S0's form given an empty root, `== ` and
  `IOERR NotFound`;
- roots whose directory is `""`: a root with no directory part beside a
  directory named `ui.nova` (E0093), which both sides list as `.`, a
  package's `src/main.nova` and `tests/main.nova` named from the
  package's directory, and an empty root;
- a package's manifest asked for by its path (§3): a `Nova.toml`, a
  package on a case-insensitive disk and on no other, the test asking the
  disk which it is; and, but on Windows, a `nova.toml` that is a link to
  a file, one that is a dangling link, and one in a directory that can be
  searched but not listed;
- eleven spellings of a root's path (§5), and, but on Windows, a root
  named `q"uo\te.nova`, whose label the dump escapes; on Linux, a root
  whose path is not UTF-8, which pins today's difference (§5).

The corpus files in a package's `src/` or `tests/` are skipped: the test
asserts a floor on their count (66 at 5.2a) and reports it with
`eprintln!`, which libtest shows only under `--nocapture`, so CI's log
does not.

Every form of `KINDS` must appear in the dumps of std, the corpus and the
cases, and no form of `LATER_KINDS`; the dumps are both sides', byte for
byte, so this holds for N's as for S0's.

**Time.** On a 4-core Linux machine, debug build, with the target and the
temporary directory on the same disk (not tmpfs), `selfhost.rs` ran in
about 34 s (124 s of CPU) at part A's head, where S0 alone dumped the
corpus, the cases and 450 fuzz inputs, parsing std again for each, and
in 47 s with N's side added at 450 inputs. Three changes bring it down.
The machine's speed varies from day to day, so 5.2a's tests are measured
against 5.1's, run alone in the same build, interleaved: after the
second fact-check's fixes, three runs of the tree's 36 tests then took
31.9, 32.3 and 30.9 s (95 to 100 s of CPU), against 20.9, 20.0 and
20.3 s (49 to 51 s of CPU) for the 27 tests 5.1 had, 1.48 to 1.61
times; after the first fact-check's fixes the same runs took 24.8 to
26.5 s against 16.7 to 17.6 s, 1.40 to 1.58 times, and the replay's
slowest run then, 29.5 s, was 1.77 times 5.1's fastest. (The first
fact-check measured 24.3 s, 72 s of CPU, and 24 to 26 s over about 20
runs, with the target on tmpfs; part B's 34.5 to 39.9 s against 25.4 s
for 5.1's tests came from a slower state of the same machine.)

- **S0 parses std once per `--dump-resolution` process** (differs from
  the spec, which leaves S0's resolver as it was but for §7).
  `nova_resolver::parse_std` lexes and parses a target's std modules and
  `resolve_program_with_std` resolves a program with them;
  `resolve_program_for` is the two in one, as before, for every other
  caller (`check`, `build`, `run`, `test`, the language server), whose
  output is unchanged: `nova check` over every file the base tracks
  prints what the base's does, and a unit test checks that
  `resolve_program_for` and the two give the same file, definitions,
  diagnostics, tests and import occurrences for two programs sharing one
  `ParsedStd`. `nova_driver::ResolutionDumps` parses std once and, for
  each root, registers std's modules first in the program's file
  database, so their ids, and the parsed trees' spans, are the same in
  every program. The dumps are unchanged, byte for byte; S0 dumps the
  corpus in 3.6–4.3 s where it took 17.8 s. The cost: two public
  functions and a struct in `nova-resolver`, and a second path into the
  resolver that only the dump takes.
- **The two sides of a batch run at once** (`compare_batch`), which 5.1's
  dumps share, and the corpus's batches beside the cases'.
- **The fixed seeds give 50 inputs each, not 150**: every one of the 8
  codes still occurs at least twice, E0083 the rarest with 8.

**CI.** On `main`'s CI at `1fa7cd3f` (5.1b's merge), `selfhost.rs` took
19.45 s on Windows and 22.50 s on macOS, and about 19.3 s on Linux on
5.1b's pull request: 5.1's tests. Here 5.2a's tests take 1.4 to 1.8
times 5.1's in the same build (1.40 to 1.77 in every measurement above),
so on CI they should take about 27 s (19.3 × 1.4) to 40 s (22.50 × 1.8)
on each system, under the spec's
minute (risk 4), and no count is cut; the pull request's three legs give
the figures. (An estimate from 5.1a's older ratio, 1.4 to 2.1 times
local on macOS and Windows, gave 46 to 75 s; `main`'s run replaces it.)

### 10. The speed

Measured locally, five runs, N built by `nova build` with the debug
runtime (what CI uses), S0's debug build:

| | N | S0 |
|---|---:|---:|
| std's part alone (`--std-part` / `--std`) | 0.46–0.52 s | 0.082–0.142 s |
| the 251 loose corpus programs, one process | 6.3–6.7 s | 3.6–4.3 s (17.8 s parsing std per program, at part A's head) |

N parses std once in about 0.45 s and then resolves and dumps a program
in about 0.024 s, against S0's 0.015 s with std parsed once: 1.5 to 2.0×
S0 debug over the corpus (1.53 to 1.81 in five runs here after the
second fact-check's fixes, 1.69 to 1.84 in three after the first; the
second fact-check measured 1.48, 2.01 and 1.86), and about 4 to 6× on
std's part (3.6 to 6.3 here), whose 21 scopes of about 450
names each N sorts with a merge sort over `String.cmp`. Not gated, and
well under the ~10× past which the user would be told. The first build was
9× slower (139 s over the corpus): it read the builtin table's constant
arrays in a loop, and Nova builds a constant array again each time it is
read (§4's idiom).

### 11. A change to resolution changes both compilers

From 5.2a the loader of loose programs and the item-level resolver exist
twice. A change to how a loose program loads, how names are bound, what
the builtin or std tables hold, how methods are mangled, or what the
resolver says changes `nova-resolver` (or the driver's loader) and
`selfhost/` in one pull request, with the dump's grammar here if it
changes; the differential in every pull request holds the two equal
(phase plan decision 6).

## Consequences

- N has its third and fourth stages for loose programs: it loads a root
  and its siblings and resolves them with all of std, as S0 does, byte
  for byte in the dump. 5.2b adds packages, the WASM target and test mode
  (and the three codes §2 defers), and the live heap's figures; 5.3 ports
  the type checker and HIR on top of `Definitions`.
- The dump pins `DefId` numbering and mangled method names, which reach
  symbols in 5.5.
- S0's resolver gained `parse_std` and `resolve_program_with_std`, and
  its glob import's E0002s follow declaration order; nothing else in S0's
  behaviour changed. `nova check` gained a form only when
  `--dump-resolution` comes first after `check`; every other `nova
  check` command line, `--help` included, gives what it gave (§1).
- `selfhost.rs` takes 1.4 to 1.8 times as long locally as 5.1's tests
  alone in the same build, and should take about 27 to 40 s on CI's
  three systems (§9), under a minute.
- `selfhost resolve` writes `/` in E0093's path on Windows, where S0
  writes `\`, for a root with a directory part; the harness keeps such
  paths out of what it compares, and 5.5's driver settles it (§5).
- Nova's constant arrays cost an allocation per read: N reads them once,
  and 5.3's tables should too.
