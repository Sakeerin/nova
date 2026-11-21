# ADR 0059 — The self-hosted driver, `build` and `check`

## Status

Accepted (2026-11-20). Branch `claude/loving-archimedes-frtf5q-55b`
(`docs/superpowers/specs/2026-11-19-phase-5-5b-the-driver-and-linking-design.md`),
written as 5.5b's prototype was built. CI's figures are §14's, from
the prototype's draft pull request Sakeerin/nova#146 (measurement only,
closed unmerged); no line was passed.

Part 1 built what §1 to §3 and §8 record, on the success path:
- §1, N's command lines;
- §2 and §3, `build`'s steps, `clang`, the runtime and `cc`;
- §8, the golden tables shared by two test files,
  `s1_builds_the_golden_suite`, and the CI steps.

Part 2 built what §4 to §7 and §9 to §11 record:
- §4, the Windows refusal;
- §5, every message and exit code;
- §6, `check`'s lines and the extraction of S0's stderr;
- §7, what N does not match;
- §9, both probe tests;
- §10, N's `@test`s;
- §11, the mutants.

Part 3 measured S1 building N with its own `build` (§13) and wrote the
records: §12 to §17 and the Consequences, with the figures of parts 1
and 2 (§12).

The plan's mutants and its fact-check added three things after part 3:
- §9, the clang test's case of a runtime two directories up, and its
  report counting the cases that ran;
- §8, a main-step test that holds CI's step to the ignored tests;
- §11, the plan's mutants, and the two that these additions kill.

## Decision

### 1. The command lines (the user's decision 5)

```
selfhost build FILE [-o OUT | --output OUT] [--release] --std DIR
selfhost check FILE --std DIR
```

N's `main.nova` dispatches `build` and `check` to a new
`selfhost/src/driver.nova` (803 lines, 596 of code; code lines exclude
blank lines and lines that start with `//` after their indentation).
`build_args` and `check_args` read the command line (spec §3.1):
- **Each option once, in any order:** `-o OUT`, `--output OUT`,
  `--output=OUT`, `--release`, `--std DIR`, `--std=DIR`, and one FILE.
  `--` ends the options, and `-` alone is a FILE.
- **Refused:** a second FILE or option, an unknown option,
  `--release=…`, no FILE, no `--std`, an option without its value, an
  empty value, and a separate value that starts with `-` but is not `-`
  itself (clap takes that for an option). `-oX` and `-o=X` are unknown
  options. `check` takes neither an output nor `--release`.
- **A refusal** prints one line on stderr, `usage: selfhost build FILE
  [-o OUT] [--release] --std DIR` or `usage: selfhost check FILE --std
  DIR`, nothing on stdout, and exits 2. clap's exit code is matched; its
  text is not (§7).
- **The default output** is FILE's stem, relative, as S0's
  `PathBuf::from(stem)`; `out` when FILE has no stem. The helper,
  `file_stem_or`, is new in `loader.nova`, and `module_name` is now
  `file_stem_or(path, "main")`. No `.exe`: N never links on Windows.

**`--std` is required, never searched for** (spec §3.1, not asked): S0
embeds std, so there is nothing to mirror; N's other commands take
`--std`, and every harness passes it. This narrows the phase plan's
2026-11-02 amendment to §5.5, "finding std next to the executable or in
`$NOVA_HOME`": N finds std only where `--std` says.

### 2. `build`, step by step

`build_command`, in S0's order (spec §3.2):
1. The command line (§1), then the Windows refusal (§4).
2. **std,** read from `--std`. Unreadable: N's existing line, `error:
   cannot read std's PATH: KIND`, exit 2, as `selfhost mir` gives it (§17).
3. **FILE, loaded** as `selfhost mir` loads it: a loose program, or a
   module directly in a package's `src/` or `tests/`, with its path
   dependencies, its `@test` functions stripped. Unreadable: `failed to
   open FILE`, caused by the `IoError`'s message, Rust's text. A manifest
   N's reader refuses: `Error: selfhost build cannot read FILE: WHY`, exit
   1 (differs: S0 syncs).
4. **Resolved, checked and lowered in full** (`compile_root`), std's
   bodies with it, as S0's `build` checks (`FrontendContext::check`), not
   `selfhost mir`'s shared-std path (§17). Diagnostics print as §6 says,
   warnings too; with errors, `could not compile due to N previous
   error(s)`, exit 1.
5. **Emitted** by N's `compile_ir` (5.5a). Its error: `internal LLVM IR
   generation error (this is a compiler bug)`, caused by its words; no
   `.ll` is written.
6. **The `.ll` written** to `intermediate(out, "ll")`, `<out's file
   name>.nova.ll` beside OUT, `out.nova.ll` for an OUT with no file name,
   as S0's `intermediate`; `-o out.ll` gives `out.ll.nova.ll`. Not
   written: `failed to write <ll>` and the message, exit 1.
7. **`clang`, the runtime and `cc`** (§3), on `<out's file name>.nova.o`.
8. **Clean-up:** the object is removed whatever happened, an error
   ignored. On success the `.ll` is removed too, `built OUT` is printed
   on stdout, and the exit code is 0. On any failure after step 6 the
   `.ll` is kept, under the release head `release build failed; generated
   LLVM IR left at <ll>`.

**Without `--release`** (the user's decision 6) `build` runs the same
steps, `-O2` included, so it gives the same binary and the same
messages. S0 without `--release` uses Cranelift, another binary, and no
release head on its failures. Recorded (differs).

### 3. `clang`, the runtime and `cc`

`compile_and_link` (spec §3.3), as S0's `compile_ir_to_object` and
`link_executable` run them:
- **`clang`:** `$NOVA_CLANG` or `clang`, available when `run(clang,
  ["--version"])` is `Ok`, whatever its status. Then `clang -O2 -c LL -o
  OBJ`, with and without `--release`; what it prints (a
  `-Woverride-module` warning on every `.ll`) is dropped on success.
- **No `llc`** (not asked). `NOVA_LLC` is not read. Without `clang`,
  N gives S0's words verbatim, "no LLVM toolchain found for `--release`:
  install LLVM (so `clang` or `llc` is on PATH), or set NOVA_CLANG /
  NOVA_LLC to one". Recorded (differs): with `llc` and no `clang`, S0
  builds and N does not.
- **The runtime is looked for after `clang` has run,** as S0 looks for
  it, so a missing runtime gives S0's messages at the cost of one compile
  in that error alone (`find_runtime`):
  1. `$NOVA_RUNTIME_LIB`; when set it must exist, else `NOVA_RUNTIME_LIB
     points to X, which does not exist`;
  2. no embedded runtime (the user's decision 5);
  3. `current_exe()` (failing: `locating the nova executable` and the
     message), then `libnova_runtime.a` in its directory, its parent and
     its grandparent. S0's `build/` rule, for cargo's test binaries, is
     not ported. None: S0's words verbatim, "could not find the Nova
     runtime library (libnova_runtime.a) near EXE, and this nova does not
     carry one. …", with N's own path.
- **`cc`:** `$CC` or `cc`, run as `-o OUT OBJ RUNTIME -lpthread -ldl
  -lm`, S0's `link_cc` line, on macOS too.
- **A spawn failure** has two causes, `failed to spawn clang: "NAME"` (or
  `linker`) and the system's message, NAME quoted as Rust's `{:?}` quotes
  it for the names N meets: in `"`, with `"` and `\` escaped
  (`debug_quoted`).
- **A non-zero status** has one cause (`status_cause`): `clang failed
  (exit status: N):` (or `linking failed`), then the tool's stdout and
  its stderr, each as text and trimmed, on lines of their own, empty ones
  too.
- **A tool's output as text** (`tool_text`): `Bytes.to_string()`; when
  that is `None`, the ASCII bytes kept and U+FFFD for each other byte.

### 4. Windows (the user's decision 4)

N's std has no OS query. **N knows Windows by `current_exe()`'s shape**
(`windows_shape`, not asked): an ASCII letter and `:` (a drive), or
`\\` (a `\\?\` or UNC prefix), at its start; Unix's absolute paths start
with `/`. No inherited state is read.
- `build` checks it after the command line, before std or FILE is read,
  nothing written: `Error: selfhost build links on Linux and macOS only`,
  exit 1.
- If `current_exe()` fails, N is taken to be on Unix, and `build` goes on
  to §3's own error.
- `check` does not link, and runs on Windows.

**Unverified on Windows locally.** The main probe test (§9) checks the
refusal on CI's Windows leg, and an `@test` pins the shapes (`C:\x`,
`\\?\C:\x`, `/x`, `x`).

### 5. Messages and exit codes, against S0's

One helper, `fail(head, causes)`, prints anyhow's layout (`failure_text`):
`Error: HEAD`; when there are causes, a blank line, `Caused by:`, and
each cause's lines indented four spaces, empty ones too; the causes
numbered `0:`, `1:` when there are several, a numbered cause's later
lines indented seven. Then exit 1. Each row below is S0's, byte for
byte, on the probe set (§9), but where it says it differs:

| Case | stdout | stderr | exit |
|---|---|---|---|
| built | `built OUT` | warnings' lines | 0 |
| `check` clean | `ok: FILE` | warnings' lines | 0 |
| usage | | N's one line (differs: clap's text) | 2 |
| Windows, `build` | | `Error: selfhost build links on Linux and macOS only` (N's own) | 1 |
| FILE missing or a directory | | `failed to open FILE`, caused by Rust's text (`os error 2` or `21`) | 1 |
| compile errors | | §6's lines, then `could not compile due to N previous error(s)` (`build`) or `found N error(s)` (`check`) | 1 |
| emitter error (float `%`) | | `internal LLVM IR generation error (this is a compiler bug)` and its words; no `.ll` left | 1 |
| `.ll` not written (`-o nodir/x`) | | `failed to write nodir/x.nova.ll` and the message | 1 |
| no `clang` | | the release head and S0's cause | 1 |
| `clang` fails | | the release head and `clang failed (exit status: N):` with its output | 1 |
| runtime missing, a text file, or not found | | the release head and S0's cause; with a text file, the linker's failure and its output, the same on both sides | 1 |
| `cc` missing / fails | | the release head and `0: failed to spawn linker: "…"`, `1: …` / `linking failed (exit status: N):` and its output | 1 |
| manifest refused | | `Error: selfhost build cannot read FILE: WHY`, or `selfhost check` (differs: S0 syncs) | 1 |
| std unreadable | | `error: cannot read std's PATH: KIND` (N's own; S0 embeds std) | 2 |

### 6. `check`'s lines, and the extraction of S0's stderr (the user's decision 3)

`check_command`, as S0's file-mode `nova check`: the command line, std,
FILE loaded and checked as `build` does, and lowered only when the
program runs, as S0's `check_program_counted_for` decides it (a loose
program, or a package's `src/main.nova`); a package module that does
not run is not lowered. Float `%` gives `ok:`, as S0's.

**N's lines** (`diagnostic_lines`), per diagnostic, in N's order, on
stderr:
- `SEVERITY[CODE]: MESSAGE`, codespan's header, the severity lowercase,
  and no brackets when there is no code;
- then one `  ┌─ PATH:LINE:COL` line, with two spaces, per file the labels
  name, in the order the labels first name the files;
- no snippet, note, help or blank line.

Then `ok: FILE` on stdout, exit 0, or `Error: found N error(s)`,
counting errors and not warnings, exit 1.

**The locus rule** (`loci`; codespan-reporting 0.11.1, `term/views.rs`):
a file's locus is its earliest primary label, else its earliest
secondary; of two labels with one start and one style, the first.

**Line and column** (`line_and_column`), from the label's start, a byte
offset in its module's text:
- line = 1 + the `\n` bytes before it, so `\r\n` counts once;
- column = 1 + the bytes in `[line start, offset)` that are not UTF-8
  continuation bytes (`0x80` to `0xBF`): a tab is one column, and so is a
  Thai mark, a CJK character or an emoji.

**PATH** is the name S0's `FileDb` gives: FILE as given; for a sibling or
a dependency, the path N's loader joined and read; `<std/NAME>` for std.
Confirmed on the probes: `d/b.nova` under `check d/a.nova`, `./d/b.nova`
under `check ./d/a.nova`, `05-geometry/shapes/src/units.nova:27:26` for
the path dependency's error, and `<std/strings>:193:6`. std's text is
read from `--std` only when a label names it (`StdModule` keeps none),
and a locus in a std file N cannot read is left out (§17). A label whose
file is unknown is left out, as S0's renderer leaves it out.

**The extraction** (`extracted_stderr`, in `selfhost.rs`): S0 runs with
`NO_COLOR=1`, and its stderr is cut to
- the lines that begin `error[`, `warning[`, `note[` or `help[`;
- the lines whose first non-blank text is `┌─ `, re-indented to two
  spaces, a `\` in their path read as `/` on Windows, as `settle_paths`
  reads it for E0004;
- and every line from the first that begins `Error: ` to the end:
  anyhow's error and its causes, which span several lines (§17).

N's stderr must equal the extraction byte for byte; stdout and exit
codes must be equal. A message holding a line break would lose its
second line in the extraction and fail the comparison loudly; none
does today.

### 7. What N does not match (spec §3.6)

Recorded as (differs), and kept out of the probe set's comparison:
- clap's usage text; the exit code, 2, and the empty stdout are compared,
  and N's own line is pinned;
- no FILE: S0 finds a project or `src/main.nova`; N exits 2;
- project mode, the registry sync, `--target`, `--ssr`, `--ssg` and
  `llc` (spec §15);
- the no-runtime message's path, N's own executable's (the clang probe
  test checks N's against `beside_exe`'s words with N's path);
- Rust's `ExitStatus` text for a signal (`signal: 9 (SIGKILL)`), where
  N's std gives `128 + n` and N prints `exit status: 137`;
- a program name that `{:?}` escapes beyond `"` and `\`;
- tool output with invalid UTF-8 that also holds valid multi-byte
  characters, or with whitespace outside N's `trim` at its ends;
- Windows: `build` refused (§4), and a locus's `\` read as `/`;
- a Nova panic, an abort with 134, against Rust's 101;
- `check`'s snippets, notes, help and colour.

### 8. The harness: the shared tables, `s1_builds_the_golden_suite`, CI

**The shared tables** (spec §6.1): `crates/nova-cli/tests/golden/shared.rs`
(250 lines), included by `#[path = "golden/shared.rs"] mod golden;` in
`run_tests.rs` and `selfhost.rs`. It holds `RELEASE_GOLDEN_EXCLUDED`,
`_STDIN`, `_ENV`, `_STATUS`, `_LIMIT` and `_STDERR`,
`PROCESS_TEST_VALUE` and `PROCESS_TEST_EQ`, `quoted_stderr`,
`run_within_limit`, `run_release_binary`, `golden_programs()` (the walk,
its table checks and the floor of 131), and two helpers part 1 added so
the two walks share them: `golden_dir()` and `run_golden()`, one
program's run and judgement. `run_tests.rs` lost 213 lines and gained
12; `release_golden_suite` still gives 147 passed, 9 excluded.

**`s1_builds_the_golden_suite`** (`selfhost.rs`; spec §6.2; the user's
decisions 1, 7 and 8a), `#[ignore = "needs clang; the Selfhost job's
Linux step runs it"]`:
1. `clang` is `NOVA_CLANG` or `clang`, probed by `--version`; if it does
   not spawn, the test fails under `NOVA_REQUIRE_CLANG=1` and otherwise
   prints `s1 golden: not run (no clang)` and passes.
2. S1 is `selfhost()`, built by Cranelift as every test of the file
   builds it (`build_selfhost`).
3. One runtime for both sides (`golden_runtime`): `NOVA_RUNTIME_LIB` if
   set; else the only runtime `build_selfhost` unpacked under its fresh
   `NOVA_HOME`; else the one beside S0. Passed to both as
   `NOVA_RUNTIME_LIB` and named in the report.
4. Per program, 4 threads: `selfhost build SRC -o DIR/s1/NAME --release
   --std STD`, whose stdout must be exactly `built DIR/s1/NAME`; S0's
   `nova build --release SRC -o DIR/s0/NAME`, one basename; both with
   `NO_COLOR=1`, `RUST_BACKTRACE=0` and `RUST_LIB_BACKTRACE=0`, within
   `RELEASE_GOLDEN_LIMIT`. S1's binary runs as `release_golden_suite`
   runs S0's (`run_golden`): its stdout must equal its `.stdout`, and its
   status the table's.
5. The two binaries are read and compared byte for byte
   (`same_binaries`; the user's decision 8a, no hash crate); a
   difference names both sizes and the first differing offset.
6. Each program's two binaries, any `.ll` or `.o`, and its temporary
   directory are removed right after the comparison, and `DIR` at the
   end. The report: `s1 golden: 147 passed, 9 excluded, <clang
   version>, runtime <path>`.

It was seen red first: all 147 failed on N's usage line of that time,
`usage: selfhost tokens|ast FILE...`, exit 2.

**CI** (spec §7), two steps in the Selfhost job after `cargo test
--test selfhost`, on its Linux leg alone (`if: runner.os == 'Linux'`):
- `Install clang 18.1.3 (Linux)`, the Test job's step verbatim,
  asserting 18.1.3 (ADR 0048);
- `S1 builds the golden suite (Linux)`, with `NOVA_REQUIRE_CLANG=1` and
  `NOVA_CLANG=clang-18` and no `NOVA_RUNTIME_LIB`: `cargo test --locked
  --workspace --all-features --no-fail-fast --test selfhost -- --ignored
  --exact s1_builds_the_golden_suite
  n_build_answers_as_s0_does_with_clang`. Its S1 is rebuilt in the
  step's own process, and `golden_runtime` finds the release runtime its
  `build_selfhost` unpacked (`NOVA_EMBED_RUNTIME=1`). The step has no
  time line; its time is recorded (§14).

**`the_ci_step_names_every_clang_test`** (`selfhost.rs`), in the main
step on all three systems, holds that line to this file. With `--exact`,
a name that matches no test runs nothing and the step still passes. So
the test reads `ci.yml` from the repository and `selfhost.rs`'s own
source (`include_str!`), as text, CRLF line endings allowed. It finds
the step `S1 builds the golden suite (Linux)` and the names after
`--exact` on its `run:` line, and asserts:
- each name is a test of the file, marked `#[ignore]`;
- the names are exactly the tests marked `#[ignore = "needs clang;
  ..."]`, today `s1_builds_the_golden_suite` and
  `n_build_answers_as_s0_does_with_clang`.

It needs no crate, no network and no N, and takes under 10 ms.

### 9. The probe tests (the user's decision 8b)

S0 against N, each case in its own fixed directory under the test's,
the inputs written afresh into each, a fresh `NOVA_HOME`, `NO_COLOR=1`,
`RUST_BACKTRACE=0` and `RUST_LIB_BACKTRACE=0` on both sides; stdout, the
exit code, §6's extraction of stderr, and the files each side leaves, a
`.ll` among them, byte for byte. Each side's files are compared, then
removed before the other side runs.

**`n_build_and_check_answer_as_s0_does`,** in the main step on all three
systems, 4 threads, 39 cases:
- **19 `check` cases:** an unknown name with a help (E0001); a duplicate
  whose secondary label comes first (E0002); spans of several lines
  (E0010); after tabs; after CJK, an emoji and Thai; Thai combining
  marks; a warning alone (`ok:`); two errors and a 2-digit line; `\r\n`
  line endings; a sibling's error, from `d/` and from `./d/`; labels in
  two files (E0074, the primary in the second); a label in
  `<std/strings>`; a path dependency's error (a copy of
  `examples/05-geometry`, an error appended to its library's
  `src/units.nova`); a parse error; a clean program; float `%`; a missing
  FILE; a directory as FILE.
- **9 `build --release` cases:** compile errors; a missing FILE; a
  directory as FILE; `-o nodir/x`; no `clang` (`NOVA_CLANG` and
  `NOVA_LLC` naming a missing file) with `-o`, with the default name, and
  with a warning, each leaving both `.ll`s, equal; no `clang` and no
  runtime; float `%`, no `.ll`.
- **7 usage cases,** compared on exit 2 and empty stdout, N's line
  pinned: `build` with an unknown option, `-o` twice, a second FILE,
  `--release=yes`; `check` with an unknown option, a second FILE, `-o`.
- **4 cases on N alone,** which S0 has no form of: no `--std` and no FILE,
  for `build` and for `check`.

On Windows a case that builds is N's refusal alone (exit 1, nothing on
stdout, nothing left), and S0 is not run for it. The extraction has a
unit test of its own, `the_extraction_keeps_the_headers_the_loci_and_the_error`.

**`n_build_answers_as_s0_does_with_clang`,** ignored, run in the CI step
of §8, 10 cases with the golden walk's runtime as `NOVA_RUNTIME_LIB`
where a case names none:
- success with `-o`, with the default name from a relative FILE, and
  with a warning; no `.ll` or `.o` left beside a binary built, and the
  binaries equal;
- `NOVA_CLANG=false`; `NOVA_CLANG=false` with `NOVA_RUNTIME_LIB` missing
  (`clang`'s failure is the one reported, since the runtime is looked for
  after `clang`);
- `CC=false`; `CC` naming a missing file;
- `cc`'s command line: `CC` a script that writes its arguments to
  `cc-args.txt` and runs `cc` (`CC_ARGS`, Unix only), the file compared;
- `NOVA_RUNTIME_LIB` naming a missing file, and naming a text file.

Then, on N alone:
- **no runtime:** N copied alone into a fixed directory, none in it or
  the two above, `NOVA_RUNTIME_LIB` unset, against `beside_exe`'s words
  as written there with N's path (CI's S0 carries a runtime);
- **without `--release`:** the same stdout and the same binary as with
  it (the user's decision 6);
- **a runtime two directories up:** a copy of N in `up/bin` under that
  fixed directory, the runtime beside `up`, `NOVA_RUNTIME_LIB` unset:
  N builds and prints `built up`, as S0's `beside_exe` looks in the
  executable's directory, its parent and its grandparent (spec §3.3);
- **the harness's own checks:** on the binary built, `run_golden` refuses
  a wrong stdout (`arrays`'s, and `fs_already_exists`'s `cleaned`, which
  a check looser than equality would take), and `same_binaries` refuses
  a binary one byte off, at that offset.

Each case counts once as it runs, and the report gives "N of M" out of
the cases that ran: 16 when all run, which is the 10 above, the three on
N alone, and the harness's three refusals. Those three run only once
`build` without `--release` has passed.

**Fixed temporary names,** each under the temporary directory and
removed by its test at the end: `nova-selfhost-golden`,
`nova-selfhost-probe`, `nova-selfhost-probe-clang`,
`nova-selfhost-alone`. Part 3's measurement used two more, outside any
test (§13).

### 10. N's `@test`s

`selfhost/src/driver_tests.nova` (214 lines, 163 of code), 11 `@test`s,
run under `the_selfhost_tests_pass` on both runtimes:
`the_intermediate_is_named_for_the_outputs_file_name_as_s0_names_it` (a
trailing `/`, an output with no name, `-o out.ll`);
`the_default_output_is_files_stem_as_path_file_stem_gives_it`
(`a.b.nova`, `.nova`, `x.`);
`windows_is_known_by_a_drive_or_a_double_backslash`;
`the_command_lines_take_what_the_spec_says_and_refuse_the_rest`;
`a_failure_is_printed_in_anyhows_layout` (no cause, one, one with empty
lines, two);
`line_and_column_count_line_feeds_and_characters_as_codespan_does`
(tab, `\r\n`, multi-byte);
`a_files_locus_is_its_earliest_primary_label_else_its_earliest_secondary`
(the earliest primary against an earlier secondary, two files);
`a_diagnostic_prints_its_header_and_one_locus_per_file`;
`a_tools_name_is_quoted_as_debug_quotes_it`;
`a_failed_tools_cause_is_its_status_line_and_its_output_trimmed`;
`a_tools_output_that_is_not_utf8_keeps_its_ascii_and_marks_each_other_byte`.
N's `nova test` passes 119 (108 before), on the debug runtime and on the
release runtime. `selfhost/src` is now 57 files and 32,356 lines.

### 11. The mutants

Each was applied in the working tree, the listed tests run, then
reverted; none was committed. **All 23 were killed:**

| Mutant | Edit | Killed by |
|---|---|---|
| M1 | the `.ll` named `NAME.ll`, not `NAME.nova.ll` | main probe (5 cases), an `@test`, clang probe (9) |
| M2 | `clang` without `-O2` | clang probe (3), golden (147 of 147 differ) |
| M3a | the runtime looked for before the `clang` probe | main probe (the 4 no-`clang` cases), clang probe (1) |
| M3b | the runtime looked for between the `clang` probe and its run | clang probe: "clang fails, and no runtime" |
| M4a | `cc` without `-lm` | clang probe: "cc's command line" |
| M4b | `cc` without `-lpthread` | clang probe: "cc's command line" |
| M4c | the runtime before the object on `cc`'s line | clang probe (5) |
| M5 | the default output FILE's name, not its stem | main probe (1), an `@test`, clang probe (1) |
| M6 | one line break before `Caused by:` | main probe (10), an `@test` |
| M7 | a failure exits 2 | main probe (25); N's `@test`s do not call `fail` |
| M8a | the drive's `:` test on `;` | an `@test` (on Linux and macOS; CI's Windows leg would also fail the main probe) |
| M8b | `/` taken for Windows' `\\` | main probe (9), an `@test` |
| M9 | the `.ll` kept on success | clang probe (4) |
| C1 | the column counts bytes | main probe (the wide and Thai cases), an `@test` |
| C2 | `\r` counted as a line break | main probe (CRLF), an `@test` |
| C3a | a file's first label is its locus | **an `@test` alone** |
| C3b | the later start wins | **an `@test` alone** |
| C3c | the files' loci reversed | main probe (two files, std), two `@test`s |
| C4 | `Error` for `error` in a header | main probe (15), an `@test` |
| H1 | the binaries never compared | the clang probe's harness check; `s1_builds_the_golden_suite` alone does not kill it, since S0's and S1's binaries are equal anyway |
| H2 | the stdout check loosened to `contains` | the clang probe's harness check on `fs_already_exists` |
| H3a | the extraction without `warning[` | main probe (2), the extraction test |
| H3b | loci not re-indented | main probe (5), the extraction test |

**Four survived until a case was added:**
- **M3b** survived until "clang fails, and no runtime" was added: no case
  of spec §6.3 reaches a runtime missing with a `clang` that fails.
- **M4a and M4b** survived until "cc's command line" was added: the
  binaries stay byte-equal without `-lm` or `-lpthread`, which glibc
  2.34's libc (and macOS's libSystem) already holds.
- **H2** survived until `fs_already_exists` joined the harness check,
  which had run only a binary whose output no `.stdout` held.

The locus rule's two mutants (C3a, C3b) are killed by the `@test` alone:
no diagnostic from source has a file whose first label is not its
locus (S0 puts the primary first in E0002 and E0074), so no probe input
reaches the rule (§17).

**The plan's mutants** (2026-11-20, all at the split's tip, the tree
of the prototype's head) are 64:
- part 2's 23 again;
- 20 of part 1's checks: the shared tables, the golden harness, the
  usage line, `clang`'s and `cc`'s lines, the runtime's search, the
  default stem, std's moved loading, and three of CI's step;
- 21 of the words spec §3.4 lists.

Two of them survived until a test was added:
- **R2, the plan's mutant 37:** the runtime looked for in N's own
  directory alone. It survived until the clang test's case of a runtime
  two directories up was added (§9): every other case names the runtime
  or has none near.
- **CI1, the plan's mutant 41:** a name misspelt in CI's step. With
  `--exact` the step ran one test and passed. It survived until
  `the_ci_step_names_every_clang_test` was added (§8), which also kills
  a name dropped from the step (63) and `#[ignore]` removed from the
  clang test (64).

Four are accepted:
- **T5** (an environment entry dropped) is equivalent: `env` refuses
  names that hold `=`.
- **H5** (S1's `built OUT` line not checked) is covered by the clang
  test, which compares that line.
- **A1** (`clang`'s arguments in another order) gives the same command.
- **W4** (the Windows refusal's words) is unreachable on Linux, and CI's
  Windows leg catches it.

### 12. The figures of parts 1 and 2

Measured locally on 4 cores, `clang-18` 18.1.3, `cc` gcc 13.3, GNU ld
2.42; S0 at opt-level 0 unless said; S1's golden runs on the release
runtime (`NOVA_RUNTIME_LIB`, `0.4.0-be7e8b48`) unless said. Scripts and
logs are outside the repository, not committed.

**`s1_builds_the_golden_suite`** (part 1): 147 passed, 9 excluded, every
binary byte-equal to S0 `--release`'s.

| What | Figure |
|---|---|
| the test, wall (its `finished in`) | 49.7 s (44.38 s), 43.2 s (42.99 s) |
| S1's 147 builds alone, 4 threads | 16.5 s wall, 60.5 s CPU |
| S0 `--release`'s 147 builds alone, 4 threads | 16.4 s wall, 59.0 s CPU |
| the chain by hand (`mir --ll`, `clang`, `cc`) | front end 26.0, `clang` 16.4, link 21.3 s CPU; 17.9 s wall |
| S1's own build (Cranelift; its runtime not recorded) | 8.85 s, 9.00 s |

**`selfhost.rs`'s main step** ("as CI runs it" is
`NOVA_EMBED_RUNTIME=1 CARGO_PROFILE_DEV_OPT_LEVEL=1`; `finished in`):

| Comparison | Style | Before | After |
|---|---|---|---|
| part 1 against the spec's base `17ecca86`, one session, not interleaved | plain | 216.78 s (67 passed) | 210.88 s (67 passed, 1 ignored) |
| part 1 | as CI runs it | 55.01 s | 55.13 s |
| part 2 against part 1's head `09f4257b`, interleaved | plain | 208.65, 211.08 s (67, 1 ignored) | 221.77, 227.85 s (69, 2 ignored) |
| part 2 | as CI runs it | 58.13, 66.10 s | 70.36, 71.67 s |
| part 2, the main probe test alone | as CI runs it | | 3.43, 3.37 s |

Part 1 adds nothing measurable to the main step: its test is ignored
there. Part 2 adds about 15 s plain and about 9 s as CI runs it, against
the spec's projected 1 to 3 s (§17); the runs as CI runs them are noisy,
and the probe test costs more under the other tests' load than alone.

**The CI step, locally, as CI runs it** (no `NOVA_RUNTIME_LIB`, the
runtime `build_selfhost` unpacked, the same checksum as the release
runtime above; wall): part 1, the golden test alone, 33.8 s and 32.4 s;
part 2, both ignored tests, 39.3 s and 39.1 s (`finished in` 39.04 and
38.89 s). The spec projected about 40 s locally.

**Verification** (part 2, at `77d247c7`): the full suite (`cargo test
--workspace`, Node required) 2,837 passed, 1 failed (the IPv6 test, as on
the base), 10 ignored, the base's figures, since `selfhost.rs` is `test =
false`; `SH` 69 passed, 2 ignored; `release_golden_suite` 147 passed, 9
excluded; `cargo fmt --check`, both clippy commands as CI runs them,
`nova fmt --check std examples selfhost` and Rust 1.78's `cargo check
--workspace` with `-D warnings` clean (its test targets cannot build on
1.78, as before 5.5b: `assert_cmd` needs edition 2024); `Cargo.lock` and
the manifests unchanged.

### 13. S1 builds N, and S2 = S1 (part 3; spec §11 item 4)

For 5.6, a measurement only; no test was added. Measured 2026-11-20 at
`ecfcd0f8`, part 3's head, whose `selfhost/` and S0 sources the later
commits leave unchanged; every build on the release runtime above, through
`NOVA_RUNTIME_LIB`, with `NOVA_CLANG=clang-18`. `/usr/bin/time` is not
installed on this machine, so a small wrapper measured each command:
wall time, peak RSS as `wait4`'s `ru_maxrss` (what GNU `time -v`
reports: the largest of the process and the children it waited for,
`clang` and `cc` among them), and the process's own `VmHWM`, polled
every 20 ms.

**S1.** S1 is normally built by S0's `nova build` in `selfhost/`, by
Cranelift, without `--release` (`build_selfhost`); on CI it links the
release runtime its S0 embeds, and locally the debug one beside S0. The
S1 that built N here was built that way, linked against the release
runtime: 7.85 s, 102 MB, 9,376,184 bytes.

**S1 builds N with its own `build`:** `selfhost build
selfhost/src/main.nova -o nova-selfhost-s2/selfhost --release --std std`
(the fixed directory `nova-selfhost-s2` in the temporary directory, kept;
its binary is S2) exits 0 and prints `built …/selfhost`:

| Run | Wall | Peak RSS (`ru_maxrss`) | S1's own `VmHWM` |
|---|---:|---:|---:|
| 1 | 13.29 s | 203.4 MB | 203.6 MB |
| 2 | 12.97 s | 203.5 MB | 203.6 MB |

Its split, one run each: the front end and the `.ll` written (`NOVA_CLANG`
naming a missing file, so N stops at the `clang` probe) 1.77 s;
`clang-18 -O2 -c` on the `.ll` 11.08 s, 200.3 MB; `cc` 0.15 s, 80 MB.
So `clang` is about 85% of the build, and S1's own front end is the
peak by about 3 MB. The `.ll` of N's root is 12,759,478 bytes, 406,232
lines; ADR 0058 §11 counted 12,252,987 bytes for N's root at 5.5a, its
`(Ll ok)` line included, before the driver.

**S2 = S1 with one basename.** S0's `nova build --release
selfhost/src/main.nova -o nova-selfhost-s1r/selfhost` (the sibling fixed
directory, kept; its binary is the `--release` S1) took 11.74 s and
12.36 s, S0's own peak 123.5 MB and `clang`'s 200.3 MB. **The two
binaries are equal, byte for byte:** 9,354,680 bytes each, one SHA-256.
Their `.ll`s, removed on success by both, were made again with
`NOVA_CLANG` and `NOVA_LLC` naming a missing file, the same `-o`
basename in two other directories: equal byte for byte. The Cranelift
S1 (9,376,184 bytes) is a different binary from the `--release` one, as
expected; what it builds is not: the research (spec §2 item 6) found
the Cranelift- and the `clang`-built S1 giving the same binaries, and
here the Cranelift S1's S2 is the `--release` S1.

**S2 runs as S1 does.** On 24 commands, S1 and S2 gave the same stdout,
stderr and exit code (each side's output directory's path normalised):
- `check` on `tests/runtime/generics.nova`, `examples/05-geometry`'s
  main, and N's own root (`ok:`);
- `check` on ten of the probe set's inputs and a missing FILE;
- `build --release` of float `%` and of compile errors; two usage
  errors; `mir --ll` of `generics`;
- `build --release` of `arrays`, `generics`, `async_tasks`,
  `string_builder` and `examples/05-geometry`'s main: each pair of
  binaries equal, and each golden one, run, printing its `.stdout`.

**S2 builds N too:** twice, 12.84 s and 12.76 s, its own `VmHWM` 203.7
MB, and the binary equal to S2, byte for byte, so S2 = S3 here; S2's own
front end, measured as S1's, took 1.70 s. S2 is built by `clang -O2`,
S1 by Cranelift, yet S2 builds N no faster: `clang` dominates.

### 14. CI's figures

From the prototype's draft pull request Sakeerin/nova#146 (measurement
only, closed unmerged), two runs:
- run 44672859855 on `77d247c7` (part 2's head);
- run 44674948875 on `fe0e5f10`, the same code with `CC_ARGS` gated.

**`selfhost.rs`'s main step** (`finished in`; passed, failed, ignored),
against the Selfhost job's lines (ADR 0057 §19). The margin is the line
minus the time:

| System | Line | Run 44672859855 | Run 44674948875 | Counts |
|---|---:|---:|---:|---|
| Linux | 80 s | 56.35 s (margin 23.65) | 56.75 s (margin 23.25) | `69 0 2` |
| macOS | 100 s | 71.15 s (margin 28.85) | 86.12 s (margin 13.88) | `67 0 2` |
| Windows | 110 s | 63.56 s (margin 46.44) | 70.30 s (margin 39.70) | `63 0 2` |

Against 5.5a, the margins were:
- pull request's run 44475745997: 23.5, 13.5 and 15.5 s;
- merge on `main`, run 44479648345: 25.1, 32.5 and 43.2 s.

The main probe test's +9 s measured locally (§12) does not show above
CI's run-to-run swing: macOS moved 15 s between two runs of the same
code. No line was passed.

**The new steps on Linux:**
- **Install clang 18.1.3:** 9.1 s and 10.7 s. It prints `Ubuntu clang
  version 18.1.3 (1ubuntu1)`; the runner already had it installed.
- **S1 builds the golden suite:** 30.4 s and 30.3 s wall (`finished in
  30.25s` and `30.21s`): `s1_builds_the_golden_suite` and
  `n_build_answers_as_s0_does_with_clang`, 2 passed. This is below the
  39 s measured locally (§12) and the 46 to 57 s the spec projected
  (spec §10).

**The whole Selfhost job:**

| System | Run 44672859855 | Run 44674948875 |
|---|---:|---:|
| Linux | 3:31 | 3:26 |
| macOS | 3:46 | 4:20 |
| Windows | 3:52 | 3:55 |

The longest job stays the Test job on Windows: 12:40 in both runs.

**The Clippy job's `windows-latest` leg failed in the first run,** on the
second clippy command (`cargo clippy --locked --all-features --test
selfhost -- -D warnings`): `constant CC_ARGS is never used`, because its
one use is under `#[cfg(unix)]`. The constant is now `#[cfg(unix)]` too,
and no other item of the probe code is used under one system alone
(§17). The second run passed all 19 jobs.

**Not measured:** the Test job's totals. Its logs could be read only
from their tail, and the full log's download was refused.

### 15. The intermediate's name, and what 5.5b leaves to 5.6

**The intermediate's name** (the user's decision 2) is S0's,
`<out's file name>.nova.ll` beside OUT, with no emitter change. The
`.ll`'s basename reaches the binary (`STT_FILE`, research §6), so two
stages give equal binaries only when built with one `-o` basename:
**5.6 builds every stage with the same `-o` basename** (§13 used
`selfhost`), in directories of its own. No `source_filename` line, no
constant name.

**Left to 5.6** (spec §15):
- S1 == S2 == S3 in CI, its job and its time; §13 is the first
  measurement;
- the boundary ADR and `v0.5.0`;
- and, outside Phase 5's plan unless a spec takes them up: project mode,
  the library refusal and the registry sync; `--target`, `--ssr`,
  `--ssg`; `llc`; WASM and test mode's code generation; Cranelift in N;
  an embedded runtime in N; Windows linking; rendered diagnostics;
  `check`'s code and span on every input; any std builtin;
  `source_filename` or any emitter change.

### 16. ADR 0058 §14's questions, answered

5.5b answers each of the six questions ADR 0058 §14 deferred:
1. **Where S1's golden suite runs:** a new step of the Selfhost job's
   `ubuntu-24.04` leg, `clang-18` installed there, S1 built by
   Cranelift, its programs on the release runtime (§8).
2. **The intermediate's name:** S0's `<out>.nova.ll`, one `-o` basename
   for every stage (§15).
3. **N's command line and exit codes:** §1 and §5; std only from
   `--std` (§1); the runtime from `NOVA_RUNTIME_LIB` or beside the
   executable (§3); no project mode, so no `current_dir`; stderr compared
   by §6's extraction.
4. **`check`'s output:** codes and spans, codespan's header and locus
   lines (§6), ADR 0048 decision 1; not codespan's layout.
5. **`build` without `--release`:** `-O2`, as with it (§2).
6. **Platforms:** Linux and macOS; Windows refused with a message (§4);
   no OS query in std.

ADR 0058 is not changed.

### 17. Departures from the spec, and decisions it does not settle

**From the spec,** each recorded where it falls:
- **§10's main-step projection, +1 to 3 s, measured +9 to +15 s
  locally** (+9 s as CI runs it, +15 s plain; the probe test about
  3.5 s alone), §12. Against 5.5a's pull request's margins of 23.5,
  13.5 and 15.5 s on Linux, macOS and Windows this is the first risk;
  CI's figures are §14's.
- **§6.3's clang list gained three cases:**
  - "clang fails, and no runtime" (the runtime's order) and "cc's
    command line" (`cc`'s arguments), for M3b, M4a and M4b, which no
    case of the list could kill (§11). The first is on every Unix; the
    second is Unix only (`CC_ARGS`).
  - On N alone, "a runtime two directories up" (spec §3.3's search),
    for R2 (§11).
- **§9's locus rule** ("the earliest primary against an earlier
  secondary") **has no source input that reaches it:** only the `@test`
  covers it (§11, C3a and C3b).
- **The research's `frem.nova` never hit float `%`:** it wrote `{a % b}`
  without `$`, so it built; its float `%` row was right only by 5.5a's
  dump. The probe's `frem.nova` uses `${a % b}`.
- **The harness's stdout check** first ran a binary whose output no
  `.stdout` held, so it did not show that it was held to equality (H2);
  `fs_already_exists` was added (§9).
- **Local clippy runs only on Linux:** `CC_ARGS`, used only under
  `#[cfg(unix)]`, was dead code on Windows, and the Clippy job's Windows
  leg failed (§14); the constant is now `#[cfg(unix)]`. The other two
  `#[cfg(unix)]` blocks of the probe code use only `std`.

**Decisions the spec does not settle,** each with its cost if wrong
(all cheap):
- **`build` checks in full,** as S0's `build` does, not `selfhost mir`'s
  shared-std path; all 147 binaries are equal either way. About 10
  lines to switch.
- **std's loading moved** (`load_std`, `read_std_modules`,
  `labels_and_texts`, `io_kind_name`) from `main.nova` to
  `driver.nova`, verbatim and public: Nova forbids import cycles, and
  `check` needs them too.
- **std unreadable** is N's existing `error: cannot read std's PATH:
  KIND`, exit 2, as `selfhost mir` gives it; spec §3.4 has no row.
- **The command line's edges** (§1): an empty value and a separate value
  that starts with `-` refused; `-` alone a FILE; `-oX` and `-o=X`
  refused. If wrong, a different exit code from clap's on unusual forms.
- **A refused manifest's head names FILE,** `Error: selfhost build
  cannot read FILE: WHY`, and `check`'s mirrors it, `Error: selfhost
  check cannot read FILE: WHY`, exit 1.
- **The CI line names both ignored tests,** as spec §7 writes it, and
  `the_ci_step_names_every_clang_test` holds it to the tests `selfhost.rs`
  ignores as needing `clang` (§8), since `--exact` runs nothing for a
  name that matches no test.
- **The extraction's tail is every line from the first `Error: ` line,**
  not the last line alone: anyhow's causes span several lines. One line
  to change.
- **std's loci are read from `--std` on demand,** since `StdModule` keeps
  no text; a std file N cannot read leaves its locus out.
- **Each probe has its own directory,** and the files a side leaves are
  compared, then removed before the other side runs.
- **Three probe cases beyond §6.3's list** (above).
- **`golden_dir()` and `run_golden()`** joined the shared file, so both
  walks run and judge a binary the same way.

## Consequences

- N builds programs: `selfhost build` writes S0's `.ll`, runs `clang` and
  `cc` as S0 does on Linux and macOS, and leaves the same binary, byte
  for byte, as S0's `nova build --release` for the same `-o` basename:
  on the 147 golden programs (§8), and on N itself (§13).
- `selfhost check` prints each diagnostic's code and locus, S0's header
  and locus lines; N's `build` and `check` give S0's stdout, exit codes
  and messages on 39 probe cases without `clang` and 10 with it, §7's
  exceptions listed.
- N's `build` refuses Windows with its own message.
- S1 builds N in about 13 s locally, at about 204 MB, and S2 builds N to
  itself (§13): 5.6's S1 == S2 == S3 holds here with one `-o` basename.
- `selfhost.rs`'s main step grew about 9 s locally as CI runs it (§12),
  more than the spec projected; the new step takes about 39 s locally,
  outside the 80 s line, and 30 s on CI; on CI `selfhost.rs` stays within
every line (§14).
- S0, its driver, the emitter and the runtime are unchanged, so ADR 0058
  §16's rule is not engaged; no std builtin was added.
