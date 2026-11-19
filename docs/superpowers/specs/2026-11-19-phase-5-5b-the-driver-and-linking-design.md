# Phase 5.5b, "the driver and linking: N's `build` and `check`, and the golden suite built by S1": design

- **Date:** 2026-11-19.
- **Status:** approved by the user (2026-11-19).
- **The user's decisions, 2026-11-19:**
  1. **S1's golden suite runs in a new step of the Selfhost job's
     `ubuntu-24.04` leg,** after `selfhost.rs`, `clang-18` installed
     there; S1 built by Cranelift, as `selfhost.rs` builds it; its
     programs on the release runtime; outside the 80 s line.
  2. **The intermediate is named as S0 names it,** `<out>.nova.ll`; no
     emitter change; 5.6 uses one `-o` basename across stages.
  3. **`check` prints code and span,** `error[CODE]: message` and
     `path:line:col` (30 to 60 Nova lines), compared by code and span
     (ADR 0048 decision 1), not by rendered text.
  4. **Linux and macOS:** N's `build` refuses Windows with a message,
     recorded (differs); the `.ll` comparison stays on all three systems.
  5. **File mode and `--std`:** `selfhost build FILE [-o OUT] [--release]
     --std DIR`, `selfhost check FILE --std DIR`; the runtime from
     `NOVA_RUNTIME_LIB` or beside the executable; exit codes 0/1/2 and
     S0's `built` and `Error:` lines; no project mode, no std builtin
     (differs).
  6. **`build` without `--release` is `-O2`, as with it;** recorded that
     S0 without `--release` uses Cranelift.
  7. **The harness runs S1's real `build` per golden program,** with
     `run_tests.rs`'s stdin, environment and exclusion tables.
  8. **The gate, besides golden 147/147:** (a) each S1 binary equals S0
     `--release`'s for the same `-o` name; (b) `build`'s and `check`'s
     messages and exit codes equal S0's on a probe set, paths and Rust's
     signal text excepted. Not chosen: `check`'s code and span on every
     input; `check` is compared on the probe set alone.
- **Not asked; the recommendation taken, which the user can overturn:**
  `--std` is required, never searched for (§3.1); usage errors print N's
  own line, exit 2 (§3.1); the default output is the relative `<stem>`,
  as S0's (§3.2); the runtime is looked for after `clang`, as S0 does,
  and `llc` is not ported (§3.3); Windows is known from `current_exe`'s
  shape (§3.5); `check` prints codespan's own header and locus lines
  (§4); the harness is `#[ignore]`d tests in `selfhost.rs`, the golden
  tables moved to a module both test files include (§6.1); binaries
  compared byte for byte, no hash crate (§6.2); the probes without
  `clang` run in the main step on all three systems, those with it in
  the new step (§6.3); the prototype measured on CI first (§10); one
  pull request, split into tasks (§11).
- **Branch:** `claude/loving-archimedes-frtf5q-55b`, from `main` at
  `e71446df` (5.5a's merge), for this spec; the plan and the work follow
  on it. Nothing is pushed without the user's word.
- **Inputs:** `docs/phase-5-plan.md` (§1 to §3, §5.5 and its amendments,
  §5.6, §5, §6); ADRs 0048, 0050, 0057 and 0058; the 5.5a spec; the
  research of 2026-11-19 (summarised in §2).

## 1. What 5.5b delivers

N gains a driver. **`selfhost build`** writes S0's `.ll`, runs `clang`
and `cc` on it and leaves a binary; **`selfhost check`** prints each
diagnostic's code and locus. A new CI step builds the 147 golden
programs with S1's own `build`, runs them, and holds each binary equal
to S0 `--release`'s. This is the second half of the plan's 5.5
(amended 2026-11-18): its gate's second item, "programs S1 builds pass
the golden suite on Ubuntu", with decision 8's additions (§16).

## 2. The starting point

Measured on `e71446df`, 2026-11-19 (the research); line numbers
re-checked, and a few probes made for this spec with the debug S0 in
`target/` (not rebuilt).

1. **S0's driver is unchanged by 5.5a** but for `--ll`: `build`
   (`nova-cli/src/cmd/run.rs:418`; in file mode the output is
   `PathBuf::from("<stem><EXE_SUFFIX>")`, relative), `check` (`:465`),
   `could_not_compile` (`:221`); `build_program_release`
   (`nova-driver/src/lib.rs:921`) and `intermediate` (`:898`).
2. **`link.rs`:** `compile_ir_to_object` (`:25`), `tool_available`
   (`:49`), `run_tool` (`:53`), `link_executable` (`:70`), which **finds
   the runtime only after `clang` has run**; `beside_exe` (`:119`),
   `link_cc` (`:195`), `run_linker` (`:206`); `locate`
   (`runtime_cache.rs:187`). §3 gives what N takes from each.
3. **S0's messages,** probed (research §1): anyhow's `Error: HEAD`, a
   blank line, `Caused by:`, each cause line indented 4 spaces (empty
   ones too), numbered `0:`, `1:` when there are several; exit 1. clap's
   errors exit 2; a panic 101.
4. **`check` renders with codespan** (`nova-diagnostics/src/render.rs`):
   `error[CODE]: message`, then `┌─ path:line:col` indented by the
   gutter (2 spaces for 1-digit lines, 3 for 2-digit). A file's locus is
   its earliest primary label, or its earliest secondary if it has no
   primary; labels in two files give two loci, in the order the labels
   name them (codespan-reporting 0.11.1, `term/views.rs:103-135`). The
   column counts chars (`files.rs:206`): a tab is 1. Probed for this
   spec: a sibling's locus is `d/b.nova` under `check d/a.nova`,
   `./d/b.nova` under `./d/a.nova`; a path dependency's is
   `05-geometry/shapes/src/units.nova:27:27`, gutter 3; std's files are
   `<std/NAME>` (`lib.rs:752`); `check` left no `nova.lock` in the
   copied package.
5. **N** (`selfhost/src/main.nova:103-133`): `tokens`, `ast`, `resolve`,
   `hir`, `mir [--ll]`; no `build`, no `check`; std only from `--std`
   (`load_std`, `:507`). `mir --ll`'s `.ll` equals S0's intermediate.
   N's HIR dump holds S0's diagnostics (codes, messages, labels, notes,
   fixes, order) as byte spans on module labels (`diagnostic.nova:9-12`),
   with no line, column or path. `module_name`, `path_parent`,
   `path_join` (`loader.nova:773`, `:807`, `:854`) mirror `Path`. 55
   files, 31,399 lines.
6. **S1 passes the golden suite today, linked by hand:** 147/147, 16.4
   to 16.5 s wall at 4 threads (three runs), on the release runtime.
   **Each S1 binary is byte-identical to S0 `--release`'s for the same
   output basename,** from the Cranelift-built and the clang-built S1
   alike; only the `.ll`'s basename varies a binary (`STT_FILE`). S0
   `--release` took 15.6 s on the same walk (S0 at opt-level 0). Per
   program: front end 0.148 s (an upper bound, with the MIR dump),
   `clang` 0.108 s, link 0.137 s; std's fixed cost about 0.13 s a
   process.
7. **CI:** only the Test job's Linux leg installs `clang-18`, asserting
   18.1.3 (`ci.yml:56-65`). The Selfhost job (`ci.yml:164-194`) sets
   `NOVA_EMBED_RUNTIME=1` and opt-level 1; `build_selfhost`
   (`selfhost.rs:460`) uses a fresh `NOVA_HOME`, so the release runtime
   is unpacked at `home/runtime/<ver>-<crc>/libnova_runtime.a`. The
   target is `test = false` (`nova-cli/Cargo.toml:27-30`). Lines 80, 100,
   110 s (ADR 0057 §19); 5.5a measured 51.39, 29.94, 93.86 s (ADR 0058
   §13).
8. **The golden tables** (`run_tests.rs:1785-1824`): 9 exclusions,
   stdin for `io_streams`, two variables for `process_env`
   (`:13343-13349`), 60 s a process; 156 `.stdout` programs, 147 walked,
   floor 131.

## 3. N's commands (decision 5)

### 3.1 Command lines

```
selfhost build FILE [-o OUT | --output OUT] [--release] --std DIR
selfhost check FILE --std DIR
```

- **Each option once, in any order:** `-o OUT`, `--output OUT`,
  `--output=OUT`, `--release`, `--std DIR`, `--std=DIR`, one FILE; `--`
  ends the options. Anything else (a second FILE or option, an unknown
  option, `--release=…`, no FILE, no `--std`, an option without its
  value): one line, `usage: selfhost build FILE [-o OUT] [--release]
  --std DIR` (or `usage: selfhost check FILE --std DIR`), nothing on
  stdout, **exit 2**. clap's text is not matched; its exit code is.
- **`--std` is required, never searched for** (not asked): S0 embeds
  std, so there is nothing to mirror; N's other commands take `--std`;
  every harness passes it. This narrows the plan's 2026-11-02 amendment
  ("finding std next to the executable or in `$NOVA_HOME`"), recorded.
- **Code:** a new `selfhost/src/driver.nova`, about 320 to 470 lines
  (research §9), under 5.4's 1,500 a file; `main.nova` dispatches to it.

### 3.2 `build`, step by step

In S0's order:
1. The command line; then the Windows refusal (§3.5).
2. **Load FILE** as `selfhost mir` does: a loose program, or a module
   directly in a package's `src/` or `tests/` with its path
   dependencies. Unreadable: `failed to open FILE`, caused by the
   `IoError`'s message (Rust's text). A manifest N's reader refuses:
   `Error: selfhost build cannot read P: WHY`, exit 1 (differs: S0
   syncs).
3. **Check and lower** with `mir --ll`'s pipeline. Diagnostics print as
   in §4, warnings too; with errors, `could not compile due to N
   previous error(s)`, exit 1.
4. **Emit** with N's `compile_ir`; its error is `internal LLVM IR
   generation error (this is a compiler bug)`, caused by its words; no
   `.ll`.
5. **The output:** `-o` as given, else FILE's stem, relative (not
   asked). S0 builds `PathBuf::from(stem)` too, which resolves against
   the cwd, so N needs no `current_dir` and prints the same `built`
   line. The stem is `Path::file_stem`'s, `out` when there is none (S0's
   fallback; a helper shared with `module_name`, whose is `main`). No
   `.exe`: N never links on Windows.
6. **The intermediates** (decision 2): `<out's file name>.nova.ll` and
   `.nova.o` beside OUT, `out` for a name-less OUT, as `intermediate`.
   The `.ll` not written: `failed to write <ll>` + message, exit 1.
7. **`clang`, the runtime, `cc`** (§3.3); a failure has the release
   head and the step's cause or causes, exit 1.
8. **Clean up:** the object removed always, an error ignored; on
   success the `.ll` removed, `built <out>` on stdout, exit 0.

**Decision 6:** without `--release`, the same steps, so the same binary
and messages. S0 without `--release` uses Cranelift, another binary,
and no release head on its failures. Recorded.

### 3.3 `clang`, the runtime and `cc`

- **`clang`:** `env("NOVA_CLANG")` or `clang`; available when
  `run(clang, ["--version"])` is `Ok`, whatever its status. Run `-O2 -c
  <ll> -o <obj>`, with and without `--release`; its output (a
  `-Woverride-module` warning on every `.ll`) dropped on success.
- **No `llc`** (not asked); `NOVA_LLC` ignored. Without `clang`, S0's
  words verbatim, "no LLVM toolchain found for `--release`: install LLVM
  (so `clang` or `llc` is on PATH), or set NOVA_CLANG / NOVA_LLC to one",
  so the probe matches. (Differs: with `llc` and no `clang`, S0 builds.)
- **The runtime, after `clang`** (not asked), so a missing runtime
  gives S0's messages, at the cost of one compile in that error alone:
  1. `env("NOVA_RUNTIME_LIB")`: when set it must `exists`, else
     `NOVA_RUNTIME_LIB points to X, which does not exist`;
  2. no embedded runtime (decision 5);
  3. `current_exe()` (failing: `locating the nova executable` +
     message), then `libnova_runtime.a` in its directory, the parent and
     the grandparent. S0's `build/` rule, for cargo's test binaries, is
     not ported. None: S0's words verbatim, with N's own path.
- **`cc`:** `env("CC")` or `cc`, `-o <out> <obj> <runtime> -lpthread
  -ldl -lm`, on macOS too.
- **A spawn failure:** two causes, `failed to spawn clang: "NAME"` (or
  `linker`) and the message; NAME quoted as `{:?}` quotes it, `"` and `\`
  escaped. **A non-zero status:** one cause, `clang failed (exit status:
  N):` (or `linking failed`), then the tool's stdout and stderr, each
  trimmed, on lines of their own.
- **Tool output as text:** `Bytes.to_string()`; when `None`, ASCII bytes
  kept and U+FFFD for each other byte. Rust's lossy text differs only
  where invalid UTF-8 also holds valid multi-byte characters.

### 3.4 Messages and exit codes

One helper, `fail(head, causes)`, prints anyhow's layout (§2 item 3);
in every case N has, numbered causes are one line each. Then exit 1.

| Case | stdout | stderr | exit |
|---|---|---|---|
| built | `built OUT` | warnings' lines | 0 |
| `check` clean | `ok: FILE` | warnings' lines | 0 |
| usage | | N's one line | 2 |
| Windows, `build` | | `Error: selfhost build links on Linux and macOS only` | 1 |
| FILE unreadable | | `failed to open FILE` + message | 1 |
| compile errors | | §4's lines, then `could not compile due to N previous error(s)` or `found N error(s)` | 1 |
| emitter error | | `internal LLVM IR generation error (this is a compiler bug)` + words | 1 |
| `.ll` not written | | `failed to write <ll>` + message | 1 |
| no `clang` / `clang` fails | | release head + S0's cause | 1 |
| runtime missing or not found | | release head + S0's cause | 1 |
| `cc` missing / fails | | release head + `0: failed to spawn linker: …`, `1: …` / `linking failed (exit status: N):` + output | 1 |

### 3.5 Windows (decision 4)

N's std has no OS query. **N knows Windows by `current_exe()`'s shape**
(not asked): an ASCII letter and `:`, or `\\`, at its start. Windows
gives a drive, or a `\\?\` or UNC prefix; Unix's absolute paths start
with `/`. It reads no inherited state, unlike `env("OS") ==
"Windows_NT"`, which a parent can clear or a Unix shell set.
**Unverified on Windows here;** the first probe test (§6.3) checks it on
CI's Windows leg.
- After the command line, before FILE is read, nothing written:
  `Error: selfhost build links on Linux and macOS only`, exit 1.
- `check` works on Windows. If `current_exe()` fails, N is taken for
  Unix and `build` reaches §3.3's own error.

### 3.6 What N does not match

Recorded as (differs) in ADR 0059, and kept out of the probe set's
comparison: clap's usage text (exit 2 matches); no FILE (S0: a project
or `src/main.nova`; N: exit 2); §15's project mode, sync, `--target`,
`--ssr`, `--ssg` and `llc`; the no-runtime message's path; Rust's
`ExitStatus` text for a signal (`signal: 9 (SIGKILL)`), where N's std
gives `128 + n` (`std/process/lib.nova:113`) and N prints `exit status:
137`; a name `{:?}` escapes beyond `"` and `\`; tool output with invalid
UTF-8, or with whitespace outside N's `trim` (`std/strings/lib.nova:397`)
at its ends; Windows; a Nova panic (abort, 134, against 101); `check`'s
snippets, notes, help and colour.

## 4. `check`'s output (decision 3)

### 4.1 The lines

```
error[E0010]: type mismatch: expected `Int`, found `String`
  ┌─ typeerr.nova:2:18
Error: found 1 error
```

- **Per diagnostic,** in N's order (the HIR dump's, then MIR's for a
  program that runs, as `check_program_counted_for`, `lib.rs:105`):
  `SEVERITY[CODE]: MESSAGE`, the severity lowercase; then one locus line
  per file its labels name, by codespan's rule and order (§2 item 4),
  `  ┌─ PATH:LINE:COL` with two spaces. No snippet, note, help or blank
  line. Codespan's own header and locus (not asked) make S0's lines
  extractable by a filter (§4.3).
- **On stderr;** `ok: FILE` on stdout. With errors the last line is
  `Error: found N error(s)` (`check`) or `Error: could not compile due to
  N previous error(s)` (`build`), counting errors, not warnings.
- **PATH** is the name S0's `FileDb` gives: FILE as given; for a sibling
  or a dependency, the path N's loader joined and read (to confirm on
  §6.3's two cases); `<std/NAME>` for std.

### 4.2 Line and column

From the label's start, a byte offset in its module's source:
- **line** = 1 + the `\n` bytes before it (codespan's `line_starts`), so
  `\r\n` counts once;
- **column** = 1 + the bytes in `[line start, offset)` that are not
  UTF-8 continuation bytes (`0x80` to `0xBF`): chars, as codespan's
  `column_index`. A tab is 1, a Thai mark, CJK or emoji character 1. No
  display width.

### 4.3 The comparison with S0

S0 runs with `NO_COLOR=1`. Its stderr is **extracted**: the lines that
begin `error[`, `warning[`, `note[` or `help[`; the lines whose first
non-blank text is `┌─ `, re-indented to two spaces; and the last line,
`Error: …`. N's stderr must equal the extraction byte for byte; stdout
and exit codes must be equal. On Windows a locus path's `\` is read as
`/`, as `settle_paths` (`selfhost/compare.rs:368`) does for E0004. A
message holding a line break would defeat the filter; none does today,
and the test then fails loudly.

## 5. std: nothing new (decision 5)

| Use | Where |
|---|---|
| arguments, exit | `args`, `exit` (`std/process/lib.nova:75`, `:100`) |
| `clang` and `cc`, output captured | `run` (`:158`), `ProcessOutput` (`:113`) |
| `NOVA_CLANG`, `NOVA_RUNTIME_LIB`, `CC` | `env` (`:201`) |
| the runtime beside the executable; Windows | `current_exe` (`:216`) |
| FILE, the `.ll`, the runtime, clean-up | `read_to_string`, `write_string`, `exists`, `remove_file` (`std/fs/lib.nova:97`, `:113`, `:177`, `:224`) |
| tool output as text | `Bytes.to_string` (`std/bytes/lib.nova:55`) |
| stderr | `eprintln` (as `main.nova:117`) |

`status` (`:184`) and `temp_dir` (`fs:162`) are not needed. What std
lacks (an OS query, `current_dir`, lossy UTF-8, a signal status) has an
N-only answer above or is outside 5.5b. **No question for the user.**

## 6. The harness

### 6.1 The golden tables, shared

Moved from `run_tests.rs` to a new `crates/nova-cli/tests/golden/shared.rs`
(not asked), included by `#[path = "golden/shared.rs"] mod golden;` in
`run_tests.rs` and `selfhost.rs`, as `selfhost/compare.rs` is; a file in
a subdirectory is no test target. It holds the `RELEASE_GOLDEN_*`
tables and limits, `PROCESS_TEST_VALUE`/`EQ`, `quoted_stderr`,
`run_within_limit`, `run_release_binary`, and `golden_programs()`:
`release_golden_suite`'s walk (`:1956-1991`), its table checks and
floor. `release_golden_suite` is otherwise unchanged; what one includer
leaves unused is allowed dead there. One copy, so no agreement test.

### 6.2 `s1_builds_the_golden_suite` (decisions 1, 7, 8a)

In `selfhost.rs`, `#[ignore = "needs clang; the Selfhost job's Linux
step runs it"]` (not asked): the main step skips it, so the 80 s line is
untouched, and no manifest changes.

1. **`clang`:** `NOVA_CLANG` or `clang`, probed by `--version`; if it
   does not spawn, fail under `NOVA_REQUIRE_CLANG=1`, else print `s1
   golden: not run (no clang)` and pass.
2. **S1:** `selfhost()`, by Cranelift (decision 1); its own speed does
   not change its programs' bytes (§2 item 6).
3. **One runtime for both sides,** the one S1 links: `NOVA_RUNTIME_LIB`
   if set; else what `build_selfhost` unpacked,
   `home/runtime/*/libnova_runtime.a` (exactly one); else the one beside
   S0. Passed as `NOVA_RUNTIME_LIB` to S0 and S1, named in the report.
4. **Per program, 4 threads:** S0 `nova build --release SRC -o
   <dir>/s0/NAME` and S1 `selfhost build SRC -o <dir>/s1/NAME --release
   --std <repo>/std`, one basename, with `NO_COLOR=1`, `RUST_BACKTRACE=0`
   and `RUST_LIB_BACKTRACE=0`, within `RELEASE_GOLDEN_LIMIT`; `<dir>` is
   the fixed `nova-selfhost-golden` in the temp directory. S1's binary
   runs with its stdin, variables and temporary directory
   (`run_release_binary`) and must match `.stdout` and its status.
5. **The two binaries read and compared byte for byte** (decision 8a),
   no hash crate; a difference names the program, both sizes and the
   first differing offset. The report: `s1 golden: 147 passed, 9
   excluded, <clang version>, runtime <path>`, failures with stderr.

### 6.3 The probe set (decision 8b)

S0 against N, a fresh `NOVA_HOME` and `NO_COLOR=1`, `RUST_BACKTRACE=0`,
`RUST_LIB_BACKTRACE=0` on both sides, each case in a fixed directory;
stdout, exit code and §4.3's stderr compared, and any `.ll` left behind
byte for byte.

**`n_build_and_check_answer_as_s0_does`,** in the main step on all
three systems (not asked: it needs no `clang`, and it shows the Windows
refusal on CI's Windows leg):
- `check`: an unknown name with a help (E0001); a duplicate whose
  secondary label comes first (E0002); a multi-line span (E0010); tabs;
  a column after Thai, CJK and emoji (L0001); a warning alone (E0021,
  `ok:`); two errors and a 2-digit line; a `\r\n` file; a sibling's
  error; a path dependency's error (a copy of `examples/05-geometry`); a
  parse error; a clean program; a missing FILE; a directory as FILE.
- `build --release`, on Linux and macOS (on Windows N must refuse, exit
  1): a compile error; a missing FILE; `-o nodir/x`; no `clang`
  (`NOVA_CLANG` and `NOVA_LLC` at a missing file), both `.ll`s left and
  equal; the float `%` program, no `.ll`.
- Usage, exit 2 and empty stdout on both sides: an unknown option, `-o`
  twice, a second FILE; N alone without `--std` or FILE.

**`n_build_answers_as_s0_does_with_clang`,** `#[ignore]`d, in the new
step on Linux: success with `-o` (`built OUT`, no `.ll` or `.o` left,
equal binaries); the default name from a fixed directory and a relative
FILE (`built hello`); a warning and success; `NOVA_CLANG=false`;
`CC=false`; `CC` at a missing file; `NOVA_RUNTIME_LIB` at a missing file
and at a text file (ld's lines, the same on both sides); no runtime (S1
copied alone to a fixed directory, `NOVA_RUNTIME_LIB` unset, against
`beside_exe`'s words as written, since CI's S0 carries a runtime); N's
`build` without `--release` equal to its `--release` (decision 6).

**Locally,** `cargo test --test selfhost` runs the first probe test
alone; the ignored two run as §7's step runs them. Without
`NOVA_RUNTIME_LIB` a checkout's S0 carries no runtime, and the debug one
beside it serves: S1's walk then takes 44.1 s, not 16.4 s (research §5).

## 7. CI (decision 1)

Two steps in the Selfhost job, after `cargo test --test selfhost`,
before N's own tests on the debug runtime:

```yaml
      - name: Install clang 18.1.3 (Linux)
        if: runner.os == 'Linux'
        run: |   # the Test job's step, unchanged
      - name: S1 builds the golden suite (Linux)
        if: runner.os == 'Linux'
        env:
          NOVA_REQUIRE_CLANG: "1"
          NOVA_CLANG: clang-18
        run: cargo test --locked --workspace --all-features --no-fail-fast --test selfhost -- --ignored --exact s1_builds_the_golden_suite n_build_answers_as_s0_does_with_clang
```

- The install sits right before the step (not asked), so the main step
  runs as today; it asserts 18.1.3 (ADR 0048 decision 4). No
  `NOVA_RUNTIME_LIB`: `NOVA_EMBED_RUNTIME=1` gives S0 the release
  runtime, which `build_selfhost` unpacks and §6.2 finds. The step
  rebuilds S1 in its own process, about 8.4 s locally. It has no time
  line; its time is recorded, and the user may set one.

## 8. Errors and diagnostics

No new code; S0's driver, emitter and diagnostics do not change; S0's
float `%` bug stays (5.5a's decision 4). N's messages are S0's words but
for the usage line and the Windows refusal.

## 9. Testing

Rules: no internet; fixed temporary names; `$NOVA_HOME` fresh;
MSRV 1.78 (no `is_none_or`, `LazyLock`, `std::path::absolute`); no new
crate, no manifest change, `Cargo.lock` unchanged; `clang` installed by
CI, never by a test. Then §6's tests, and N's `@test`s in a new
`selfhost/src/driver_tests.nova`: the intermediate's name (`-o out.ll`
gives `out.ll.nova.ll`; a trailing `/`; a name-less OUT); the stem
(`a.b.nova`, `.nova`, `x.`); the Windows shapes (`C:\x`, `\\?\C:\x`,
`/x`, `x`); `fail`'s layout (no cause, one, one with empty lines, two);
line and column (tab, `\r\n`, multi-byte, the earliest primary against
an earlier secondary, two files); the `{:?}` quoting; the status line;
the U+FFFD fallback. They run under `the_selfhost_tests_pass` on both
runtimes.

## 10. CI time

- **Measured locally** (research §5, 4 threads): S1's walk 16.4 s wall,
  about 59 s CPU; S0 `--release`'s 15.6 s, 54.8 s CPU (opt-level 0; CI's
  S0 is at 1); S1's build 8.4 s.
- **Projected for the new step, not measured:** the two walks share 4
  threads, about 114 s of CPU, so about 30 to 33 s, plus S1's build and
  the `clang` probes: **about 40 s locally, 46 to 57 s on CI** (×1.2 to
  1.35), **plus the install,** unmeasured. The research's 20 to 22 s
  counts S1's walk alone; decision 8a adds S0's builds.
- **The main step** gains the first probe test, about 40 short processes
  a side: +1 to 3 s projected, against margins of 23.5, 13.5 and 15.5 s
  at 5.5a's pull request's run (44475745997) and 25.1, 32.5 and 43.2 s
  at its merge on `main` (44479648345).
- **Projections have undershot:** 5.4 projected +1 to 4 s and measured
  +15 to 18 s; the Selfhost job is not CI's longest (Linux 3:18, the
  Test job 12:24 on Windows; ADR 0057 §19).
- **The prototype on CI before the plan** (recommended, as 5.5a): with
  the user's word for that push, asked at that step, to
  `claude/loving-archimedes-frtf5q-55b-ci` with a draft pull request,
  measurement only, closed unmerged: both steps, the install, and
  `selfhost.rs` on all three systems. Past a line, or a step the user
  finds too long, the user is asked; no coverage is cut without asking.

## 11. The cut, for the plan

One pull request, split into tasks, test first (not asked):
1. **The prototype:** the tables moved (§6.1), `release_golden_suite`
   green under `NOVA_REQUIRE_CLANG=1`; `s1_builds_the_golden_suite`
   first, red on N's usage line; N's `build` on its success path; the
   CI steps; measured locally, then on CI (§10).
2. **`build`'s refusals and errors:** both probe tests' `build` cases
   first; then §3's refusals, errors, `fail`, the Windows refusal, and
   their `@test`s.
3. **`check`:** its probe cases first; then §4 and its `@test`s.
4. **The measurements:** CI's figures for the final tree; S1 building
   N once by its own `build` (time, and S2 = S1 with one basename), for
   5.6, no test.
5. **The records** (§13).

## 12. Crates and files

`nova-cli` (`tests/selfhost.rs`, `tests/run_tests.rs`, the new
`tests/golden/shared.rs`); `selfhost/` (`src/main.nova`, the new
`src/driver.nova` and `src/driver_tests.nova`);
`.github/workflows/ci.yml`; §13's files. S0's sources, the emitter and
the runtime unchanged, so ADR 0058 §16's rule is not engaged. No new
crate, no manifest change.

## 13. Records

- **ADR 0059, the self-hosted driver, `build` and `check`:** the command
  lines; the steps and their order; `--std` required, narrowing the
  2026-11-02 amendment; no `llc`; the Windows detection and refusal;
  `check`'s lines and the extraction; §3.6's list; the harness, the
  shared tables, the probe set; the figures; the intermediate's name
  for 5.6.
- **Dated notes:** `docs/phase-5-plan.md` §5.5 (5.5b's decisions and
  gate); `nova-spec/00-MASTER-SPEC.md`'s Phase 5; `nova-spec/14-CODEGEN.md`
  §2.2 and §11 (N runs `clang` and `cc` as S0, on Linux and macOS);
  `nova-spec/40-TOOLING.md` §1.1 (`nova build`/`check` unchanged).
- **`CHANGELOG.md`:** Added (N's `build` and `check`, S1's golden step).

## 14. Risks

Ranked.

1. **CI time** of the new step (46 to 57 s projected) and the install
   (unmeasured), where 5.4's projection undershot 4 to 5 times; the
   prototype's figures come first.
2. **A binary that differs for the environment** (another `cc`, `CC`,
   flags or runtime): both sides run in one environment, with one
   `NOVA_RUNTIME_LIB` passed explicitly; the report names the runtime.
3. **The default output name without `current_dir`:** relative, as S0's;
   a probe holds it.
4. **The Windows detection:** a heuristic until CI's Windows leg runs
   the first probe test, which fails there if it is wrong.
5. **Messages that cannot match** (§3.6): listed, N's text pinned.
6. **N's fixed std cost,** about 0.13 s × 147, 19 s of CPU: in §10.
7. **Loci paths** of siblings and dependencies: two probe cases.
8. **macOS linking is not exercised in CI;** its `cc` line is S0's.

## 15. Not in 5.5b

- Project mode, the library refusal, the registry sync; `--target`,
  `--ssr`, `--ssg`; `llc`.
- WASM and test-mode code generation; Cranelift in N; an embedded
  runtime in N; Windows linking.
- Rendered diagnostics (codespan's snippets, notes, help, colour);
  `check`'s code and span on every input.
- Any std builtin; `source_filename` or any emitter change.
- 5.6: S1 == S2 == S3, its CI job, the boundary ADR, `v0.5.0`.

## 16. Gate

- On Ubuntu, in the Selfhost job's new step, S1's own `build` builds
  the 147 golden programs on the release runtime; each runs with its
  table's stdin and variables and matches its `.stdout` and status; and
  each binary equals S0 `--release`'s for the same `-o` basename, byte
  for byte.
- N's `build` and `check` give S0's stdout, exit codes and messages on
  §6.3's probe set by §4.3's extraction, §3.6's exceptions listed in ADR
  0059; the first probe test passes on Linux, macOS and Windows, the
  second on Linux.
- N's `check` prints each diagnostic's code and locus.
- N's `build` refuses Windows with its message, on CI's Windows leg.
- `selfhost.rs`'s main step is within 80, 100 and 110 s, and the new
  step's and the install's times are recorded; or the user has ruled.
- The full suite, fmt, clippy (with `--test selfhost`), the MSRV job,
  `nova fmt --check` over `selfhost/`, `release_golden_suite`, and `git
  diff` on `Cargo.lock` are clean.
