# Phase 2 Close-out Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Close Phase 2 within a recorded boundary and build nothing. Write ADR 0025 and 20-STDLIB §18 and §19, add the dated notes and in-place corrections the spec lists, and leave `main` ready for the `v0.2.0` tag.

**Architecture:**
- **One read-only check script is the test.** Task 1 writes `closeout_check.py` and watches all three of its parts fail. Task 1 makes `stdlib` pass, Task 2 `adr`, and Task 3 `records`. Task 4 runs `all`.
- **Documentation only.** §18 and §19 are appended to 20-STDLIB, and ADR 0025 is a new file. Task 3 applies every other edit with one atomic script that keeps each file's own newline. The only `.rs` change is two rustdoc passages.
- **The release is not part of this branch.** It is the procedure at the end of this plan, run only after the user merges this branch's PR, with two stops for the user's word.

**Tech Stack:** Markdown; Python 3, run as `python -X utf8`, for the check, edit and sweep scripts; one scratch Nova program run with `nova run`; `cargo test`, `cargo fmt` and `cargo clippy` for the rustdoc edit; the GitHub CLI.

**Spec:** `docs/superpowers/specs/2026-10-06-phase-2-closeout-design.md` (commits `d240002` and `1302480`), approved by the user on 2026-10-06. Read it before Task 1. It is the authority this plan argues from.

## Global Constraints

- **Record only** (spec §1). Nothing is built:
  - no Nova or Rust behaviour changes;
  - the only `.rs` edit is two rustdoc passages in `crates/nova-parser/src/lib.rs`;
  - no manifest and no `Cargo.lock` change, checked with `git diff --exit-code main -- Cargo.lock Cargo.toml crates/*/Cargo.toml`;
  - no new test.
- **CI counts must equal PR #99's** (spec §4): windows 1222 / 0 / 8, ubuntu 1216 / 0 / 9, macOS 1217 / 0 / 8.
- **Dated notes say `2026-10-06` and name branch `phase-2-closeout`.**
  - `**Amended 2026-10-06 (branch `phase-2-closeout`):**` for corrections;
  - `**Recorded 2026-10-06 (branch `phase-2-closeout`):**` for the gate-family note;
  - `**AMENDED 2026-10-06 …**` in 20-STDLIB §1, matching that index's earlier notes.

  If you execute on a later day, change `2026-10-06` in every note you write, in ADR 0025's Status, in §18's and §19's "Added" notes and in `closeout_check.py`. Leave every other date as it is.
- **Existing note bodies stay as written** (spec §3.3). Notes are added. Text is corrected in place only where Task 3 lists it: the Phase 0 guides' cells and the parser's rustdoc.
- **Citations.** ADR 0025, §18, §19 and the new notes cite:
  - the master spec by section and list position;
  - 20-STDLIB by section;
  - `docs/phase-2-plan.md` by sub-phase.

  None cites a line number in a file this branch edits, because this branch's own notes shift those lines. The master spec's note at `00-MASTER-SPEC.md:208-241` asks for the same. The check script enforces it.
- **No growing total** in §18 or §19, such as `STD_ONLY`'s size. ADR 0025 may state dated figures it measured.
- **Line endings.** Each file's working-tree newline is kept:
  - CRLF: `00-MASTER-SPEC.md`, `13-RUNTIME.md`, `14-CODEGEN.md`, `20-STDLIB.md`, `docs/phase-2-plan.md`, `CHANGELOG.md` and `agent.md`;
  - LF: `10-LEXER.md`, `11-PARSER.md`, `50-TESTING.md`, `ARCHITECTURE.md`, `skill.md` and `crates/nova-parser/src/lib.rs`.

  The scripts convert their anchors to each file's own newline. A new file written with the Write tool is LF, which is fine.
- **Write scripts and commit messages with the Write tool, never a Bash heredoc,** and commit with `git commit -F <file>`.
- **The release** (the procedure after Task 4) is not part of this branch. Its tag is pushed only on the user's explicit word.

**Plan rulings.** Each is beyond the spec's lists, and each applies a rule the spec states:
- **R1. The Phase 0 guides.** Spec §3.3 corrects the guides in place "because they describe the code as it is now". That rule covers every cell that names a crate or method an existing crate does not use. Besides the chumsky and `inkwell` cells the spec lists, it corrects:
  - `agent.md`'s `nova-runtime` → `tokio` cell;
  - `skill.md`'s two Pratt rows, its `tokio` row, its `salsa` row ("Used for query-based compilation") and its "backed by `hyper`" row.

  These stay:
  - `agent.md`'s list of locked decisions ("async/await on Tokio"), the READMEs' and RFC 0000's feature lists, and master spec §1, because ADR 0009 records their deviation;
  - every row about a crate of a later phase.
- **R2. The gate-family note.** The 2026-10-05 note ending "This note does not assess whether Phase 2 is complete" sits in four files: the master spec §3, 13-RUNTIME, 20-STDLIB §7 and phase-2-plan 2.4. Every earlier increment added its note to all four, so the boundary note goes after it in all four. The spec names only the master spec's.
- **R3. Fuzz mentions.** The 10-LEXER and 11-PARSER notes also cover their "Tests Required" fuzz targets. The 50-TESTING note also covers §4.2's nightly job.
- **R4. The release's notes.** The "Phase 2 is complete" note goes in the same four files (spec §3.6 names the master spec).

## Review Focus

The five conditions most likely to mislead a reader, most likely first, each with what pins it:

1. **A §18 or §19 bullet that misstates the code.** Examples: `trim`'s whitespace set, `slice` panicking against clamping, `split` on an empty separator, `index_of` on an empty needle. A reader writes code against the doc. → The scratch behaviour program (Task 1, Steps 3–4) runs every bullet that a program can observe.
2. **An inventory row that misstates what `v0.2.0` ships.** → `closeout_check.py adr` greps `std/` for each "not built" item by name, `Cargo.lock` for the four unused crates, and git for `fuzz/` (Task 2). The fresh fact-check re-reads every row (Task 4).
3. **A note anchored in the wrong place, or twice.** → The edit script aborts before writing anything if an anchor matches other than once. `closeout_check.py records` counts each file's dated notes (Task 3).
4. **A citation that goes stale inside this branch.** → `closeout_check.py` rejects line-number cites into edited files in ADR 0025, §18, §19 and every added line (Tasks 1–3).
5. **The rustdoc edit breaks the doctest, rustfmt or clippy.** → `cargo test -p nova-parser`, doc tests included, with `cargo fmt` and `cargo clippy` (Task 3, Step 4).

## File Structure

| File | Task | Responsibility |
|---|---|---|
| `nova-spec/20-STDLIB.md` | 1, 3 | §18 and §19 (Task 1); the §1 note and §7's gate-family note (Task 3) |
| `docs/adr/0025-phase-2-boundary.md` (new) | 2 | The boundary: the inventory, the deviation sections, the backlog |
| `nova-spec/00-MASTER-SPEC.md` | 3 | Four notes: §3's gate family, Phase 0 position 6, §5.2, §6 |
| `nova-spec/13-RUNTIME.md` | 3 | The gate-family note |
| `nova-spec/10-LEXER.md`, `11-PARSER.md`, `14-CODEGEN.md`, `50-TESTING.md` | 3 | One note each |
| `docs/phase-2-plan.md` | 3 | Eleven notes |
| `ARCHITECTURE.md`, `agent.md`, `skill.md` | 3 | Cells corrected in place (spec §3.3 and R1) |
| `crates/nova-parser/src/lib.rs` | 3 | Two rustdoc passages |
| `CHANGELOG.md` | 3 | One "Added" bullet |
| Scratch, never committed | 1–4 | `closeout_check.py`, the behaviour program, `stdlib_18_19.md`, the edit scripts, the sweep |

## Conventions for every task

- **Working directory.** `D:\Projects\nona\nova`, which is Git Bash `/d/Projects/nona/nova`. The Bash tool resets its directory after each call, so write `cd /d/Projects/nona/nova && …` in one command.
- **Scratch.** Use two directories, created in Task 1, Step 1:
  - `C:/Users/SAKEER~1/AppData/Local/Temp/gcm/closeout/` holds read-only scripts and their inputs;
  - `C:/Users/SAKEER~1/AppData/Local/Temp/gcm/_once/` holds one-time edit scripts. Rename each to `*.applied` right after it runs, so that nothing that globs `*.py` re-runs it.
- **Chain a commit and what follows it with `&&`, never `;`.**
- **`nova`** is `target/debug/nova.exe`, built with `cargo build -p nova-runtime && cargo build -p nova-cli`.
- **Port 3000 must be free** for the full Windows suite (Task 4). Check its owner with `netstat -ano | grep ':3000 .*LISTENING'` and `tasklist //FI "PID eq <pid>"`. If it belongs to the user, never stop it: ask.
- **Counting a full run.** Replace `<FILE>`:
  ```bash
  sed -E 's/\x1b\[[0-9;]*m//g' <FILE> | grep -E "^test result:" | awk '{p+=$4; f+=$6; i+=$8; n++} END {print n" result lines: "p" passed, "f" failed, "i" ignored"}'
  ```

---

### Task 1: The check script, and 20-STDLIB §18 and §19

**Files:**
- Create (scratch): `closeout/closeout_check.py`, `closeout/closeout_behaviour.nova`, `closeout/closeout_behaviour.expected`, `closeout/stdlib_18_19.md`, `_once/t1_append.py`
- Modify: `nova-spec/20-STDLIB.md` (append §18 and §19 after §17)

**Interfaces:**
- Consumes: nothing.
- Produces:
  - `python -X utf8 C:/Users/SAKEER~1/AppData/Local/Temp/gcm/closeout/closeout_check.py stdlib|adr|records|all`. It prints each failed check and exits 1, or prints `<part>: OK (<n> checks)`.
  - The headings `## 18. \`std/strings\`` and `## 19. \`std/bytes\``, which ADR 0025 and the §1 note cite as §18 and §19.

- [ ] **Step 1: Write the check script**

Run `mkdir -p /c/Users/SAKEER~1/AppData/Local/Temp/gcm/closeout /c/Users/SAKEER~1/AppData/Local/Temp/gcm/_once`. Then write this with the Write tool to `C:/Users/SAKEER~1/AppData/Local/Temp/gcm/closeout/closeout_check.py`:

````python
# READ-ONLY check for the Phase 2 close-out
# (docs/superpowers/plans/2026-10-06-phase-2-closeout.md). It edits nothing.
# Usage: python -X utf8 closeout_check.py stdlib|adr|records|all
# Prints each failed check and exits 1 if any failed; otherwise prints
# "<part>: OK (<n> checks)" for each part it ran.
import re, subprocess, sys

ROOT = "D:/Projects/nona/nova/"

def read(path):
    try:
        return open(ROOT + path, encoding="utf-8").read().replace("\r\n", "\n")
    except FileNotFoundError:
        return None

def git(*args):
    return subprocess.run(["git", "-C", ROOT] + list(args), capture_output=True,
                          text=True, encoding="utf-8").stdout

FAILS, COUNT = [], [0]

def check(cond, msg):
    COUNT[0] += 1
    if not cond:
        FAILS.append(msg)

# A line-number cite into a file this branch edits goes stale inside the
# branch itself, because this branch's notes shift those files' lines.
EDITED = (r"(?:00-MASTER-SPEC|13-RUNTIME|20-STDLIB|10-LEXER|11-PARSER|14-CODEGEN"
          r"|50-TESTING)\.md|phase-2-plan\.md|ARCHITECTURE\.md|agent\.md|skill\.md"
          r"|CHANGELOG\.md|nova-parser/src/lib\.rs")
LINE_CITE = re.compile(r"(?:" + EDITED + r")[^\n]{0,12}?:\d{2,}")
BARE_CITE = re.compile(r"`:\d+")

def cites(text):
    m = LINE_CITE.search(text) or BARE_CITE.search(text)
    return m.group(0) if m else None

def signatures(src):
    out = []
    for line in src.split("\n"):
        m = re.match(r"\s*(pub fn \w+\([^)]*\)(?:\s*->\s*[^{]+?)?)\s*\{", line)
        if m:
            out.append(" ".join(m.group(1).split()))
    return out

def part_stdlib():
    s = read("nova-spec/20-STDLIB.md")
    i17 = s.find("\n## 17. `std/process`\n")
    i18 = s.find("\n## 18. `std/strings`\n")
    i19 = s.find("\n## 19. `std/bytes`\n")
    check(0 < i17 < i18, "20-STDLIB: no §18 `std/strings` after §17")
    check(0 < i18 < i19, "20-STDLIB: no §19 `std/bytes` after §18")
    if not (0 < i17 < i18 < i19):
        return
    parts = {
        "§18": (s[i18:i19], read("std/strings/lib.nova"),
                ["str_len_chars", "str_chars", "str_from_chars", "str_index_of",
                 "str_join", "str_to_upper", "str_to_lower", "char_to_int"]),
        "§19": (s[i19:], read("std/bytes/lib.nova"),
                ["bytes_len", "bytes_is_utf8", "bytes_to_string_unchecked", "bytes_at",
                 "bytes_slice", "bytes_concat", "bytes_to_ints",
                 "bytes_from_string_intrinsic", "bytes_from_ints_intrinsic", "bytes_eq"]),
    }
    for name, (sec, src, builtins) in parts.items():
        check("**Added 2026-10-06 (branch `phase-2-closeout`), numbered out of the\n"
              "module-index order**" in sec,
              f"20-STDLIB {name}: no 'Added 2026-10-06 ... numbered out of the module-index order' note")
        m = re.search(r"```nova\n(.*?)\n```", sec, re.S)
        check(m is not None, f"20-STDLIB {name}: no nova block")
        if m is None:
            continue
        block = m.group(1)
        flat = " ".join(block.split())
        for sig in signatures(src):
            check(sig in flat, f"20-STDLIB {name}: signature not in the block: {sig}")
        lines = block.split("\n")
        for k, line in enumerate(lines):
            t = line.strip()
            if t.startswith("pub fn ") or (t.startswith("impl ")
                                           and t not in ("impl String {", "impl Bytes {")):
                above = lines[k - 1].strip() if k else ""
                check(above.startswith("//"), f"20-STDLIB {name}: no comment directly above: {t}")
        for b in builtins:
            check(f"`{b}`" in sec, f"20-STDLIB {name}: builtin `{b}` not named")
        c = cites(sec)
        check(c is None, f"20-STDLIB {name}: line-number cite: {c}")
        check(re.search(r"STD_ONLY`? (?:grows|has|holds|now)\b|\b\d+ `?STD_ONLY", sec) is None,
              f"20-STDLIB {name}: states a STD_ONLY total")
    s18, s19 = parts["§18"][0], parts["§19"][0]
    for needle in ("codepoint", "U+00A0", "U+2002", "U+2003", "U+3000", '"".split("")',
                   '",".join(parts)', '"SS"', "panic"):
        check(needle in s18, f"20-STDLIB §18: '{needle}' not found")
    for needle in ("impl Eq for Bytes", "clamp", "0..=255", "E0089", "immutable", "None"):
        check(needle in s19, f"20-STDLIB §19: '{needle}' not found")

ADR_HEADINGS = ("# ADR 0025 — Phase 2's boundary", "## Status", "## Context", "## Decision",
                "### The inventory", "### Deviations no earlier record decides",
                "### Rows whose record already explains them", "### The backlog",
                "### Found outside this boundary", "## Consequences", "## References")
ADR_SECTIONS = ("UDP and Unix sockets", "The HTTP client",
                "A router beyond `Server`'s exact paths and `GET`", "Atomics and `RwLock`",
                "`Queue`, `Deque` and `Vec::with_capacity`", "AEAD", "BLAKE3", "LLVM parity",
                "The collections benchmark", "The fixture migration", "`salsa`", "Fuzz targets",
                "chumsky: decided, the hand-written parser stays",
                "Per-sub-phase tags: decided, the four alpha tags stand in",
                "\"Benchmark hardware\": decided, this development host")
# Each "not built" row, as a pattern that would match an implementation.
ABSENT = (
    ("std/collections/lib.nova", r"^\s*pub record (Queue|Deque)\b|fn with_capacity\b",
     "Queue, Deque, with_capacity"),
    ("std/task/lib.nova", r"fn spawn_blocking\b|fn cancel\b", "spawn_blocking, JoinHandle::cancel"),
    ("std/sync/lib.nova", r"^\s*pub record (RwLock|Atomic\w*)\b|fn oneshot\b",
     "RwLock, atomics, a oneshot channel"),
    ("std/net/lib.nova", r"(?i)udp|unix", "UDP, Unix sockets"),
    ("std/http/lib.nova", r"^\s*pub (async )?fn (post|put|delete|route|use_middleware)\(",
     "the router beyond GET, the client"),
    ("std/crypto/lib.nova",
     r"^\s*pub (fn (blake3|encrypt|decrypt|aes_gcm_256|chacha20_poly1305)\b|record Aead\b)",
     "AEAD, BLAKE3"),
    ("std/process/lib.nova", r"^\s*pub fn (spawn|env)\b", "spawn, env"),
)

def part_adr():
    a = read("docs/adr/0025-phase-2-boundary.md")
    check(a is not None, "ADR 0025: docs/adr/0025-phase-2-boundary.md does not exist")
    if a is None:
        return
    for h in ADR_HEADINGS:
        check("\n" + h + "\n" in "\n" + a, f"ADR 0025: heading missing: {h}")
    for h in ADR_SECTIONS:
        check("\n#### " + h + "\n" in a, f"ADR 0025: deviation section missing: #### {h}")
    check("Accepted (2026-10-06)" in a, "ADR 0025: Status is not 'Accepted (2026-10-06)'")
    c = cites(a)
    check(c is None, f"ADR 0025: line-number cite: {c}")
    rows = [l for l in a.split("\n")
            if l.startswith("| ") and not l.startswith("| Promise") and not l.startswith("|---")]
    check(len(rows) == 42, f"ADR 0025: the inventory has {len(rows)} rows, not the plan's 42")
    for path, rx, what in ABSENT:
        t = read(path)
        check(t is not None, f"ADR 0025: {path} missing")
        if t is not None:
            m = re.search(rx, t, re.M)
            check(m is None, f"ADR 0025 says {what} is not built, but {path} matches: "
                             f"{m.group(0) if m else ''}")
    check(read("std/regex/lib.nova") is None, "ADR 0025 says std/regex is not built, but it exists")
    check(git("ls-files", "fuzz").strip() == "", "ADR 0025 says there is no fuzz/, but git tracks one")
    lock = read("Cargo.lock")
    for crate in ("chumsky", "salsa", "inkwell", "tokio"):
        check(f'\nname = "{crate}"\n' not in lock,
              f"ADR 0025 says no crate uses {crate}, but Cargo.lock has it")

NOTES = {  # file: dated notes this branch adds there
    "nova-spec/00-MASTER-SPEC.md": 4,
    "nova-spec/13-RUNTIME.md": 1,
    "nova-spec/20-STDLIB.md": 4,
    "nova-spec/10-LEXER.md": 1,
    "nova-spec/11-PARSER.md": 1,
    "nova-spec/14-CODEGEN.md": 1,
    "nova-spec/50-TESTING.md": 1,
    "docs/phase-2-plan.md": 11,
}
ALLOWED = set(NOTES) | {
    "ARCHITECTURE.md", "agent.md", "skill.md", "CHANGELOG.md", "crates/nova-parser/src/lib.rs",
    "docs/adr/0025-phase-2-boundary.md",
    "docs/superpowers/specs/2026-10-06-phase-2-closeout-design.md",
    "docs/superpowers/plans/2026-10-06-phase-2-closeout.md",
}

def part_records():
    for path, n in NOTES.items():
        t = read(path)
        got = t.count("2026-10-06 (branch `phase-2-closeout`)")
        check(got == n, f"{path}: {got} notes dated 2026-10-06 (branch `phase-2-closeout`), expected {n}")
        check("0025-phase-2-boundary.md" in t, f"{path}: never names docs/adr/0025-phase-2-boundary.md")
    diff = git("diff", "main", "-U0", "--", *NOTES.keys(), "CHANGELOG.md", "ARCHITECTURE.md",
               "agent.md", "skill.md", "crates/nova-parser/src/lib.rs")
    for line in diff.split("\n"):
        if line.startswith("+") and not line.startswith("+++"):
            c = cites(line)
            check(c is None, f"an added line cites a line number ({c}): {line[:90]}")
    arch, agent, skill = read("ARCHITECTURE.md"), read("agent.md"), read("skill.md")
    check("chumsky" not in arch.lower(), "ARCHITECTURE.md still names chumsky")
    for bad in ("| `chumsky` |", "| `inkwell` |", "| `tokio` |"):
        check(bad not in agent, f"agent.md still has the cell {bad}")
    check("async/await on Tokio" in agent, "agent.md's locked-decision line must stay (R1)")
    for bad in ("`chumsky`", "| chumsky |", "Pratt parsing", "`inkwell` crate",
                "`tokio` async runtime", "backed by `hyper`", "Used for query-based compilation"):
        check(bad not in skill, f"skill.md still says: {bad}")
    check("chumsky" not in read("crates/nova-parser/src/lib.rs").lower(),
          "crates/nova-parser/src/lib.rs still names chumsky")
    check("chumsky-style" in read("crates/nova-parser/src/grammar.rs"),
          "grammar.rs's 'chumsky-style' must stay (spec §3.3)")
    check("## [Unreleased]\n\n### Added\n- **Phase 2's boundary is recorded" in read("CHANGELOG.md"),
          "CHANGELOG.md: the close-out bullet does not head [Unreleased]'s Added")
    changed = set(git("diff", "main", "--name-only").split())
    changed |= set(git("ls-files", "--others", "--exclude-standard").split())
    stray = sorted(changed - ALLOWED)
    check(not stray, f"files changed outside the plan's list: {stray}")
    check('chumsky = "0.9"' in read("Cargo.toml"), "Cargo.toml changed; record only keeps it as it is")

PARTS = {"stdlib": part_stdlib, "adr": part_adr, "records": part_records}
names = list(PARTS) if sys.argv[1:] == ["all"] else sys.argv[1:]
if not names or any(n not in PARTS for n in names):
    sys.exit("usage: closeout_check.py stdlib|adr|records|all")
for n in names:
    before_f, before_c = len(FAILS), COUNT[0]
    PARTS[n]()
    if len(FAILS) == before_f:
        print(f"{n}: OK ({COUNT[0] - before_c} checks)")
for f in FAILS:
    print("FAIL:", f)
sys.exit(1 if FAILS else 0)
````

- [ ] **Step 2: Watch all three parts fail**

Run: `cd /d/Projects/nona/nova && python -X utf8 C:/Users/SAKEER~1/AppData/Local/Temp/gcm/closeout/closeout_check.py all; echo "exit=$?"`

Expected: `exit=1`, with failures from each part:
- `stdlib`: `FAIL: 20-STDLIB: no §18 \`std/strings\` after §17`, and the same for §19;
- `adr`: `FAIL: ADR 0025: docs/adr/0025-phase-2-boundary.md does not exist`;
- `records`: one failure per file in `NOTES`, plus the guides, the rustdoc and the CHANGELOG.

The `ABSENT` probes cannot fail yet, because `part_adr` returns before reaching them. A failure that is a Python error rather than a `FAIL:` line is a bug in the script: fix it first.

- [ ] **Step 3: Write the behaviour program and its expected output**

The program checks every §18 and §19 claim that a program can observe. It checks behaviour that already exists, so it is expected to pass at once. **A mismatch means the draft bullet in Step 5 is wrong: fix the bullet, never the expectation, and record a ruling.** Panics and aborts end the process, so those claims are checked against their tests in `crates/nova-cli/tests/run_tests.rs` instead (Step 5 names them).

Write with the Write tool to `C:/Users/SAKEER~1/AppData/Local/Temp/gcm/closeout/closeout_behaviour.nova`:

```nova
// Scratch check for 20-STDLIB §18 and §19 (phase-2-closeout); never committed.
// Each line prints one claim's observable value.
fn main() {
    // §18: counts are codepoints, not bytes.
    println("len café=${"café".len()} crab=${"🦀".len()}")
    // §18: char_at is None outside 0..len(), a negative index included.
    println("char_at 5=${"héllo".char_at(5).is_none()} neg=${"héllo".char_at(0 - 1).is_none()}")
    // §18: slice is half-open and counts codepoints; start == end gives "".
    println("slice=[${"héllo wörld".slice(6, 11)}] empty=[${"héllo".slice(2, 2)}]")
    // §18: index_of is a codepoint index; an empty needle is found at 0.
    println("index_of=${"héllo wörld".index_of("wörld").unwrap_or(0 - 1)} empty=${"abc".index_of("").unwrap_or(0 - 1)}")
    // §18: split keeps empty pieces; no separator found gives one piece.
    let a = "a,,b".split(",")
    let b = "abc".split(",")
    println("split adjacent=${a.len()} absent=${b.len()} [${b[0]}]")
    // §18: an empty separator splits into codepoints; "".split("") is [].
    println("split empty_sep=${"a→b".split("").len()} both_empty=${"".split("").len()}")
    // §18: join is called on the separator.
    println("join=[${"-".join(["a", "b", "c"])}]")
    // §18: trim's whitespace includes U+00A0 and U+3000 as well as ASCII.
    println("trim=[${"\u{00A0}\u{3000} x \t\n".trim()}]")
    // §18: full Unicode case mapping can change the codepoint count.
    println("upper=${"ß".to_upper()} lower_len=${"İ".to_lower().len()}")
    // §18: repeat(0) is "".
    println("repeat=[${"ab".repeat(0)}]")
    // §19: len counts bytes; to_string is None for invalid UTF-8.
    let raw = bytes_from_ints([104, 105, 255])
    println("bytes len=${raw.len()} utf8=${raw.to_string().is_some()} str_len=${bytes_from_string("é").len()}")
    // §19: slice clamps rather than panicking; crossed bounds give empty bytes.
    println("bytes clamp=${raw.slice(1, 100).len()} crossed=${raw.slice(2, 1).len()} neg=${raw.slice(0 - 5, 1).len()}")
    // §19: byte_at is None out of range, a negative index included.
    println("bytes byte_at=${raw.byte_at(2).unwrap_or(0 - 1)} past=${raw.byte_at(3).is_none()} neg=${raw.byte_at(0 - 1).is_none()}")
    // §19: index_of finds the first occurrence; an empty needle is found at 0.
    println("bytes index_of=${bytes_from_ints([97, 97, 97]).index_of(bytes_from_ints([97, 97])).unwrap_or(0 - 1)} empty=${raw.index_of(bytes_from_string("")).unwrap_or(0 - 1)}")
    // §19: Eq compares the bytes.
    println("bytes eq=${bytes_from_ints([1, 2]).eq(bytes_from_ints([1, 2]))} ne=${bytes_from_ints([1, 2]).eq(bytes_from_ints([1, 3]))}")
}
```

Write with the Write tool to `C:/Users/SAKEER~1/AppData/Local/Temp/gcm/closeout/closeout_behaviour.expected`:

```text
len café=4 crab=1
char_at 5=true neg=true
slice=[wörld] empty=[]
index_of=6 empty=0
split adjacent=3 absent=1 [abc]
split empty_sep=3 both_empty=0
join=[a-b-c]
trim=[x]
upper=SS lower_len=2
repeat=[]
bytes len=3 utf8=false str_len=2
bytes clamp=2 crossed=0 neg=1
bytes byte_at=255 past=true neg=true
bytes index_of=0 empty=0
bytes eq=true ne=false
```

- [ ] **Step 4: Run it**

Run: `cd /d/Projects/nona/nova && cargo build -p nova-runtime 2>&1 | tail -1 && cargo build -p nova-cli 2>&1 | tail -1 && C=/c/Users/SAKEER~1/AppData/Local/Temp/gcm/closeout && ./target/debug/nova.exe run $C/closeout_behaviour.nova > $C/behaviour.out 2>&1; echo "exit=$?"; diff <(tr -d '\r' < $C/behaviour.out) <(tr -d '\r' < $C/closeout_behaviour.expected) && echo BEHAVIOUR-MATCHES`

Expected: two `Finished` lines, `exit=0` and `BEHAVIOUR-MATCHES`.
- A compile error is a bug in the scratch program's Nova; fix the program.
- A differing line contradicts a draft bullet in Step 5. Correct that bullet before Step 5 writes it, and ledger `Task 1: Ruling: <bullet> corrected to <what the program printed>`.

- [ ] **Step 5: Write §18 and §19, and append them**

Write this with the Write tool to `C:/Users/SAKEER~1/AppData/Local/Temp/gcm/closeout/stdlib_18_19.md`. It starts with a blank line and ends with a newline:

````markdown

---

## 18. `std/strings`

**Added 2026-10-06 (branch `phase-2-closeout`), numbered out of the
module-index order** for the reason §16's opening note gives. `std/strings`
shipped in Phase 2.2b
(`docs/superpowers/specs/2026-07-27-phase-2-2b-strings-design.md`) with no
section here. Its whole surface is one `impl String` block in
`std/strings/lib.nova`, glob-imported into every module.

```nova
impl String {
    // Whether this is "". Cheaper than `len() == 0`, which decodes the
    // whole string.
    pub fn is_empty(self) -> Bool

    // The number of codepoints, not bytes. O(n).
    pub fn len(self) -> Int

    // The codepoints, in order. Index this array rather than calling
    // `char_at` in a loop.
    pub fn chars(self) -> [Char]

    // The codepoint at `i`, or `None` when `i` is outside `0..len()`,
    // negative indexes included.
    pub fn char_at(self, i: Int) -> Option<Char>

    // Codepoints `start..end`, `end` exclusive. Panics when `start` is
    // negative, `end` is past the end, or `start` is after `end`.
    pub fn slice(self, start: Int, end: Int) -> String

    // The codepoints in reverse order.
    pub fn reverse(self) -> String

    // Whether `self` begins with `prefix`, codepoint for codepoint. An
    // empty `prefix` always matches.
    pub fn starts_with(self, prefix: String) -> Bool

    // Whether `self` ends with `suffix`, codepoint for codepoint. An empty
    // `suffix` always matches.
    pub fn ends_with(self, suffix: String) -> Bool

    // The codepoint index of the first occurrence of `needle`, or `None`.
    // An empty `needle` is found at 0.
    pub fn index_of(self, needle: String) -> Option<Int>

    // Whether `needle` occurs anywhere in `self`. An empty `needle` always
    // does.
    pub fn contains(self, needle: String) -> Bool

    // The pieces between non-overlapping occurrences of `sep`, left to
    // right.
    pub fn split(self, sep: String) -> [String]

    // `parts`, with `self` between each two.
    pub fn join(self, parts: [String]) -> String

    // `self` without leading or trailing whitespace.
    pub fn trim(self) -> String

    // `self` without leading whitespace.
    pub fn trim_start(self) -> String

    // `self` without trailing whitespace.
    pub fn trim_end(self) -> String

    // `n` copies of `self`. Panics when `n` is negative.
    pub fn repeat(self, n: Int) -> String

    // Uppercase, by full Unicode case mapping.
    pub fn to_upper(self) -> String

    // Lowercase, by full Unicode case mapping.
    pub fn to_lower(self) -> String
}
```

- **Counting.** Every index and length is in codepoints, Unicode scalar
  values, never bytes: `"café".len()` is 4, though the string is 5 bytes.
  Nothing is grapheme-aware, so `reverse` separates a combining accent from
  its letter.
- **Cost.** Most methods decode the whole string into a `[Char]` first, so
  `char_at` is O(n), and calling it for each index is quadratic. Call
  `chars()` once and index the array. `index_of`, `contains` and `join` work
  on the bytes, in the runtime.
- **Out-of-range indexes.** `char_at` returns `None`, for a negative index
  too. `slice` panics instead, as `Vec::set` does, because an index that must
  be valid is the caller's bug. `start == end` gives "".
- **`split`.** When `sep` does not occur, the result is one piece, the whole
  string. Adjacent, leading and trailing separators each give an empty piece;
  nothing is collapsed or trimmed. An empty `sep` splits into single
  codepoints, and `"".split("")` is `[]`.
- **`join` is called on the separator:** `",".join(parts)`. A free `join`
  would be glob-imported into every module and take the name from user code.
- **Whitespace,** for the `trim` family, is an explicit list rather than
  Unicode's White_Space property: space, tab, line feed, carriage return,
  U+00A0, U+2002, U+2003 and U+3000. A string of only whitespace trims to "".
- **Case.** `to_upper` and `to_lower` use full Unicode case mapping, so the
  codepoint count can change: `"ß".to_upper()` is `"SS"`, and
  `"İ".to_lower()` is 2 codepoints.
- **Builtins.** Eight `STD_ONLY` builtins back the module: `str_len_chars`,
  `str_chars`, `str_from_chars`, `str_index_of`, `str_join`, `str_to_upper`,
  `str_to_lower` and `char_to_int`.
- **Tests.** `tests/runtime/strings.nova` runs under `nova run`,
  `nova build` and `NOVA_GC_STRESS=1` (`strings_run`,
  `strings_build_standalone`, `strings_under_gc_stress`). `slice`'s three
  panics and `repeat`'s each have their own test:
  `string_slice_negative_start_panics`, `string_slice_end_past_len_panics`,
  `string_slice_start_after_end_panics` and
  `string_repeat_negative_count_panics`.

---

## 19. `std/bytes`

**Added 2026-10-06 (branch `phase-2-closeout`), numbered out of the
module-index order** for the reason §16's opening note gives. `std/bytes`
shipped on 2026-08-12
(`docs/superpowers/specs/2026-08-12-byte-type-design.md`) with no section
here. It is the index's "immutable byte buffers": no method changes a
`Bytes`, and `slice` and `concat` return new ones.

```nova
impl Bytes {
    // The number of bytes. `Bytes` has no encoding, so this is never a
    // character count.
    pub fn len(self) -> Int

    // These bytes as a `String`, or `None` when they are not valid UTF-8.
    pub fn to_string(self) -> Option<String>

    // The byte at `i`, as an `Int` in `0..=255`, or `None` when `i` is
    // outside `0..len()`, negative indexes included.
    pub fn byte_at(self, i: Int) -> Option<Int>

    // The bytes `start..end`, `end` exclusive, with both bounds clamped to
    // `0..len()`. Bounds that cross give empty bytes. Never panics.
    pub fn slice(self, start: Int, end: Int) -> Bytes

    // These bytes followed by `other`'s.
    pub fn concat(self, other: Bytes) -> Bytes

    // Each byte as an `Int` in `0..=255`.
    pub fn to_ints(self) -> [Int]

    // The index of the first occurrence of `needle`, or `None`. An empty
    // `needle` is found at 0.
    pub fn index_of(self, needle: Bytes) -> Option<Int>

    // Whether `needle` occurs anywhere in these bytes.
    pub fn contains(self, needle: Bytes) -> Bool
}

// A `Bytes` holding `s`'s UTF-8 bytes.
pub fn bytes_from_string(s: String) -> Bytes

// A `Bytes` holding each element of `ints` as one byte. Aborts the process
// when an element is outside `0..=255`.
pub fn bytes_from_ints(ints: [Int]) -> Bytes

// Equal when both hold the same bytes in the same order.
impl Eq for Bytes {
    fn eq(self, other: Bytes) -> Bool
}
```

- **Representation.** A `Bytes` value has `String`'s representation, one GC
  object with its bytes inline, but carries no encoding guarantee. Nothing
  converts between the two implicitly: `to_string` and `bytes_from_string`
  are the ways across.
- **`slice` clamps where `String::slice` panics.** A `Bytes` length often
  comes from outside the program, such as a file's contents, so a bad bound
  gives a shorter result rather than ending the process. The byte-type design
  records the difference as deliberate.
- **Indexes.** `byte_at` returns `None` out of range, for a negative index
  too, as `String::char_at` does.
- **`bytes_from_ints`** aborts with `nova: panic: nova_rt_bytes_from_ints:
  element out of range 0..=255` on a value outside the range.
- **`Bytes` is a compiler-owned type name:** declaring `record Bytes` is
  `E0089`.
- **Builtins.** Ten `STD_ONLY` builtins back the module: `bytes_len`,
  `bytes_is_utf8`, `bytes_to_string_unchecked`, `bytes_at`, `bytes_slice`,
  `bytes_concat`, `bytes_to_ints`, `bytes_from_string_intrinsic`,
  `bytes_from_ints_intrinsic` and `bytes_eq`. The two free functions call the
  `_intrinsic` builtins because a builtin of their own name would collide
  with them (`E0002`).
- **Tests.** `tests/runtime/bytes_api.nova` runs under `nova run`,
  `nova build` and `NOVA_GC_STRESS=1` (`bytes_api_run`,
  `bytes_api_build_standalone`, `bytes_api_under_gc_stress`). The other tests
  are `bytes_basics_run`, `bytes_from_ints_rejects_a_value_above_the_range`,
  `bytes_from_ints_rejects_a_value_below_the_range` and
  `bytes_reserved_declaration_is_rejected`.
````

Then write this with the Write tool to `C:/Users/SAKEER~1/AppData/Local/Temp/gcm/_once/t1_append.py`:

```python
# ONE-TIME EDIT: append §18 and §19 after §17, the last section of 20-STDLIB.
import sys
ROOT = "D:/Projects/nona/nova/"
F = "nova-spec/20-STDLIB.md"
ANCHOR = "- See `docs/adr/0023-program-arguments.md`.\n"
SRC = "C:/Users/SAKEER~1/AppData/Local/Temp/gcm/closeout/stdlib_18_19.md"
text = open(SRC, encoding="utf-8").read().replace("\r\n", "\n")
raw = open(ROOT + F, encoding="utf-8", newline="").read()
nl = "\r\n" if "\r\n" in raw else "\n"
a = ANCHOR.replace("\n", nl)
if raw.count(a) != 1 or not raw.endswith(a):
    sys.exit("ABORT before any write: the anchor is not §17's last line, exactly once")
open(ROOT + F, "w", encoding="utf-8", newline="").write(raw + text.replace("\n", nl))
print("appended §18 and §19 to", F)
```

Run: `cd /d/Projects/nona/nova && python -X utf8 C:/Users/SAKEER~1/AppData/Local/Temp/gcm/_once/t1_append.py && mv C:/Users/SAKEER~1/AppData/Local/Temp/gcm/_once/t1_append.py C:/Users/SAKEER~1/AppData/Local/Temp/gcm/_once/t1_append.py.applied`

Expected: `appended §18 and §19 to nova-spec/20-STDLIB.md`, with no `ABORT`.

- [ ] **Step 6: Watch `stdlib` pass**

Run: `cd /d/Projects/nona/nova && python -X utf8 C:/Users/SAKEER~1/AppData/Local/Temp/gcm/closeout/closeout_check.py stdlib && git diff --check && git ls-files --eol nova-spec/20-STDLIB.md`

Expected:
- `stdlib: OK (<n> checks)`;
- no whitespace errors;
- `i/lf    w/crlf`.

- [ ] **Step 7: Commit**

Write this with the Write tool to `C:/Users/SAKEER~1/AppData/Local/Temp/gcm/_once/t1_commit.txt`:

```text
docs(std): document std/strings and std/bytes in 20-STDLIB

Both modules shipped without a section; §18 and §19 now give every
public signature with its contract, the behaviour a caller needs
(codepoint counting, slice's panics against Bytes::slice's clamping,
split's empty-separator rule, trim's whitespace list, full case
mapping), the builtins behind each module, and the tests that pin them.
Appended after §17, numbered out of the module-index order for §16's
reason.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
```

Run: `cd /d/Projects/nona/nova && git add nova-spec/20-STDLIB.md && git commit -q -F C:/Users/SAKEER~1/AppData/Local/Temp/gcm/_once/t1_commit.txt && git log --oneline -1`

Expected: one new commit, `docs(std): document std/strings and std/bytes in 20-STDLIB`.

---

### Task 2: ADR 0025, Phase 2's boundary

**Files:**
- Create: `docs/adr/0025-phase-2-boundary.md`

**Interfaces:**
- Consumes: §18 and §19 (Task 1), which the inventory cites; `closeout_check.py adr`.
- Produces: the path `docs/adr/0025-phase-2-boundary.md`, which every Task 3 note cites, and these section names: "LLVM parity", "The backlog", and the three "decided" sections.

- [ ] **Step 1: Confirm `adr` still fails**

Run: `cd /d/Projects/nona/nova && ls docs/adr/ | tail -2 && python -X utf8 C:/Users/SAKEER~1/AppData/Local/Temp/gcm/closeout/closeout_check.py adr; echo "exit=$?"`

Expected: the last ADR listed is `0024-gc-stack-bounds-on-unix.md`, so 0025 is the next number, and then `FAIL: ADR 0025: ... does not exist` and `exit=1`.

- [ ] **Step 2: Write the ADR**

Write this with the Write tool to `D:/Projects/nona/nova/docs/adr/0025-phase-2-boundary.md`:

```markdown
# ADR 0025 — Phase 2's boundary

## Status

Accepted (2026-10-06). Branch `phase-2-closeout`
(`docs/superpowers/specs/2026-10-06-phase-2-closeout-design.md`).

## Context

`nova-spec/00-MASTER-SPEC.md` §7 calls a phase done when:
1. its crates compile and pass CI;
2. its gate criteria are met with reproducible commands;
3. its new public surface is documented;
4. `CHANGELOG.md` is updated;
5. an ADR records each decision that deviates from the spec;
6. a `v0.{phase}.0` milestone is tagged.

Phase 2, "Standard Library Core", makes its promises in two places:
- **the master spec's §3:** the Phase 2 list of thirteen numbered entries,
  cited below as "position N", and the Phase 2 gate;
- **`docs/phase-2-plan.md`:** sub-phases 2.0 to 2.5, each with a gate, its
  §4 on how a sub-phase closes, and its §5's cross-cutting items.

`nova-spec/20-STDLIB.md` §1 names more, but it is headed "Module Index
(v1.0)": it lists v1.0's standard library, of which Phase 2 builds a part.
The inventory below includes the index's items that neither of the other two
names, marked "index", so that what `v0.2.0` leaves out is explicit too.

Earlier ADRs already narrowed Phase 2:
- 0009: `std/task` runs on Nova's own single-threaded executor, not Tokio;
- 0014: the standard library's build order;
- 0015: `std/fmt`'s scope;
- 0016 and 0017: `std/sync`;
- 0018 and 0019: 2.4 split into three increments, and `std/http` built over
  `httparse` rather than hyper.

An assessment on 2026-10-05 found Definition of Done items 1, 2 and 4 met:
- item 3 lacked 20-STDLIB sections for `std/strings` and `std/bytes`;
- item 5 lacked records for the deviations below;
- item 6, the tag, was not done.

It also found that the collector had never run off Windows, which ADR 0024
fixed for glibc Linux and macOS.

On 2026-10-06 the user decided:
- **Record only.** Closing Phase 2 builds nothing. Each gap is covered by an
  existing record or recorded here, and `v0.2.0` ships what exists.
- **An unscheduled backlog.** The deferred standard-library pieces get no
  phase and no date; each is built when a program needs it. Tooling goes
  where it fits: `salsa` to Phase 3, which builds the language server, and
  fuzzing to Phase 6, which holds the security audit.
- **One ADR,** this one, built around an inventory table.

## Decision

### The inventory

"Position N" is the master spec's Phase 2 list, and "index" is 20-STDLIB §1.
"2.N" is a sub-phase of `docs/phase-2-plan.md`, and "plan §N" is a section
of it. A bare "§N" is a section of `nova-spec/20-STDLIB.md`.

| Promise | Source | Status | Record |
|---|---|---|---|
| `std/core` | position 1 | shipped | §2 |
| `std/fmt`, `std/io` | position 2 | shipped | §3, §4; ADR 0015 |
| `std/collections`: `Vec`, `Map`, `Set` | position 3 | shipped | §12 |
| `std/collections`: `Queue`, `Deque`, `Vec::with_capacity` | §12's code block; `Queue` also index and 2.2 | not built | §12; backlog |
| `std/strings` | position 4 | shipped | §18 |
| `std/fs` | position 5 | shipped | §5; ADR 0012 |
| `std/time`, `std/log` | position 6 | shipped | §9, §10 |
| `std/task` | position 7, "wrap Tokio" | shipped, on Nova's own single-threaded executor | ADR 0009 |
| `std/task`: `spawn_blocking`, `JoinHandle::cancel` | §13's code block | not built | §13; backlog |
| `std/sync`: `Mutex`, a bounded `channel` | position 8 | shipped | ADRs 0016, 0017 |
| `std/sync`: atomics, `RwLock` | position 8 (atomics); index; 2.3 | not built | §13; ADR 0016; backlog |
| `std/sync`: a oneshot channel | 2.3 | not built | backlog |
| `std/net`: TCP, client and server | position 9 | shipped | §16; ADR 0013 |
| `std/net`: UDP, Unix sockets | position 9 (UDP); index | not built | §16; backlog |
| `std/http`: the server, over `httparse` | position 10 | shipped | §6; ADR 0019 |
| `std/http`: the client | position 10; index | not built | §6; backlog |
| `std/http`: a router beyond exact paths and `GET` | §6's code block; 2.4's notes | not built | §6; backlog |
| `std/http`: HTTPS, HTTP/2, chunked transfer-encoding | §6 | not in v1, by §6's own statement | §6 |
| `std/json` | position 11 | shipped | §7; ADR 0018 |
| `std/crypto`: SHA-256, SHA-512, HMAC-SHA-256, randomness | position 12 | shipped | §8 |
| `std/crypto`: AEAD | index; §8's code block | not built | §8; ADR 0018; backlog |
| `std/crypto`: BLAKE3 | §8's code block | refused by the `ring` backing | §8; backlog, with another backing |
| `std/test` and `nova test` | position 13 | shipped | §11 |
| `std/bytes` | index | shipped | §19 |
| `std/process`: `args`, `exit` | index; 2.5, optional | shipped | §17; ADR 0023 |
| `std/process`: `spawn`, `env` | index; 2.5, optional | not built | §17; backlog |
| `std/regex` | index; 2.5, optional | not built | backlog |
| Gate: `examples/05-json-api` serves 10k+ req/sec on benchmark hardware, with the methodology in `docs/benchmarks/` | the master spec's Phase 2 gate | met, on this development host | ADR 0021; "benchmark hardware" decided here |
| 2.0's gate | 2.0 | met in parts, under Cranelift only | LLVM parity, below; backlog |
| 2.1's gate | 2.1 | met as the 2.1 design narrowed it, under Cranelift | the 2.1 design's §2 |
| 2.2's gate | 2.2 | GC stress met; no collections benchmark | backlog |
| 2.3's gate | 2.3 | met by `examples/03-producer-consumer` | — |
| 2.4's gate | 2.4 | met by `examples/05-json-api` | ADR 0021 |
| 2.5: `std/test` and `nova test` | 2.5 | shipped | §11 |
| 2.5: migrating the e2e fixtures to `nova test` | 2.5 | not done | backlog |
| 2.5: chumsky 0.10 | 2.5; the master spec's Phase 0 position 6, and §6 | not adopted | decided here |
| 2.5: `salsa` scaffolding | 2.5; the master spec's §6 | not built | Phase 3 |
| 2.5: `fuzz/` targets | 2.5; the master spec's §5.2; `nova-spec/50-TESTING.md` §1.7 | not built | Phase 6 |
| 2.5: GC stack bounds off Windows | 2.5 | done for glibc Linux and macOS | ADR 0024 |
| Every module's programs under both backends and `NOVA_GC_STRESS` | plan §5 | none under LLVM; per-module stress coverage not assessed | LLVM parity, below; backlog |
| Both backends kept in lockstep | plan §5 | parity unverified | LLVM parity, below; backlog |
| Sub-phases "independently gated, reviewed, and tagged" | plan §4 | gated and reviewed; tagged as four alphas | decided here |

**Status words:**
- *Shipped:* a `v0.2.0` program can rely on it, within the limits its record
  states.
- *Not built:* nothing implements it.
- *Backlog*, *Phase 3* and *Phase 6:* where a deferred item goes.
- *Decided here:* a section below makes the decision.

### Deviations no earlier record decides

Each section says what was promised and where, what exists, and why it waits
or what is decided. Where a passage already says the item is unbuilt, the
section cites it.

#### UDP and Unix sockets

Position 9 names UDP, and the index names "TCP/UDP/Unix sockets". `std/net`
ships TCP, both client and server (§16; ADR 0013). §16 records twice that
UDP and Unix sockets remain unbuilt. Backlog.

#### The HTTP client

Position 10 says "server first, then client", and the index says "HTTP
client + server". §6's code block sketches the client: `get`, `post`, and a
`Response::json` that decodes. The server ships (ADR 0019), with `Server`
built on it. §6 records two things:
- the client does not ship;
- `Response::json` became a constructor, and one type cannot carry both
  meanings of that name, so building the client means renaming one of them.

Backlog.

#### A router beyond `Server`'s exact paths and `GET`

§6's code block specifies more than ships:
- `post`, `put`, `delete`, `route` and `use_middleware`;
- path parameters;
- the `Handler` and `Middleware` aliases.

2.4's notes track the router. `Server` ships with `new`, `get`, `dispatch`
and `listen`: `GET` only, exact paths, and synchronous handlers. An
`async fn` type does not parse yet (`P0001`), so neither alias can be written
(§6's 2026-10-04 note). Backlog.

#### Atomics and `RwLock`

Position 8 names atomics, and the index and 2.3 name both. `std/sync` ships a
`Mutex` and a bounded `channel` (ADRs 0016 and 0017). §13 records that
neither atomics nor `RwLock` has "a signature, a semantic, or a section
anywhere in `nova-spec/`", so building either starts with a design. Backlog.

#### `Queue`, `Deque` and `Vec::with_capacity`

§12's code block declares all three, and the index and 2.2 name `Queue`. None
is implemented, as §12 records. `std/sync`'s channel carries its own ring
buffer because `std/collections` "has no `Queue` to borrow". Backlog.

#### AEAD

The index promises "hashing, AEAD, random", and §8's code block sketches an
`Aead` API. `std/crypto` ships SHA-256, SHA-512, HMAC-SHA-256 and
randomness. §8 records AEAD as unstarted, as ADR 0018 does. Backlog.

#### BLAKE3

§8's code block declares `blake3`. §8 records it as refused by the backing §8
itself names: `ring` does not implement BLAKE3. Backlog, with a backing other
than `ring`.

#### LLVM parity

Plan §5 says both backends "must stay in lockstep", and that every module's
programs run under both. 2.0's gate asks for a program that "compiles and
runs under both backends". What exists:
- **The LLVM backend,** `crates/nova-codegen-llvm`, emits LLVM IR as text.
  `nova-driver`'s `link.rs` passes it to `clang`, or to `llc` when `clang` is
  missing. The backend's own tests check the IR text.
- **One release test runs anything.**
  `release_builds_and_runs_when_clang_available` builds and runs hello world.
  When `clang` is not on `PATH`, it returns early and passes, having run
  nothing. The other two `--release` tests hide the toolchain on purpose and
  check the IR it leaves behind.
- **"Both backends" in the sub-phase records means two Cranelift paths.** The
  records name `nova run` and `nova build`, and both are Cranelift; only
  `nova build --release` reaches LLVM. The module-system commit `8c37c79` and
  the 2.1 design's gate both say so.
- **2.0's features are tested separately.** `tests/runtime/modules/` is
  multi-file, with `import` and a generic function. `method_generics.nova`,
  `where_clauses.nova` and `extern_ffi.nova` test the other three features,
  each in its own program. No test runs any of them under LLVM.

So whether the release backend compiles Phase 2's programs correctly is
unverified, and the one test that would show it can pass without running.
Backlog: parity, including a release test that cannot pass vacuously.

#### The collections benchmark

2.2's gate ends with "benchmark basic ops". `collections_under_gc_stress`
meets its first half; nothing benchmarks collection operations. Backlog.

#### The fixture migration

2.5 says to "migrate the compiler's e2e fixtures to `nova test` where
sensible". `nova test` ships (§11), and the fixtures still run from
`crates/nova-cli/tests/run_tests.rs`. Backlog.

#### `salsa`

2.5 asks for "`salsa` scaffolding", and the master spec's §6 lists the crate.
No crate depends on it. Phase 3: the language server is where incremental
queries pay off (`nova-spec/40-TOOLING.md`).

#### Fuzz targets

2.5 asks for `fuzz/` targets for the lexer and parser, and the master spec's
§5.2 and `nova-spec/50-TESTING.md` §1.7 list more. There is no `fuzz/`
directory. Phase 6, beside its security audit.

#### chumsky: decided, the hand-written parser stays

2.5 asks for "chumsky 0.10", and the master spec's Phase 0 position 6 says to
use it. The master spec's §6 and `nova-spec/11-PARSER.md` §1 name it too.

The parser has been hand-written recursive descent since Phase 0
(`crates/nova-parser/src/grammar.rs`). Operator precedence comes from
explicit layering, and after an error the parser skips to the next item or
statement boundary. No crate has ever depended on chumsky. The workspace
`Cargo.toml` still declares `chumsky = "0.9"`, unused, a different version
from the 0.10 the specs name.

**Decision:** the hand-written parser stays, and chumsky is not adopted.
Adopting it would rewrite a working parser with no defect to fix. The unused
declaration stays until a manifest change has its own reason to happen; this
close-out changes no manifest.

#### Per-sub-phase tags: decided, the four alpha tags stand in

Plan §4's heading says each sub-phase is "independently gated, reviewed, and
tagged". Each was gated and reviewed, and none was tagged. Four pre-release
tags mark Phase 2's progress instead, `v0.2.0-alpha.1` to `v0.2.0-alpha.4`.

**Decision:** those four stand in for per-sub-phase tags, and none is added
after the fact.

#### "Benchmark hardware": decided, this development host

The master spec's Phase 2 gate reads "`examples/05-json-api` serves 10k+
req/sec on benchmark hardware", and the spec never defines benchmark
hardware. The gate was met under ADR 0021's procedure on this development
host, which runs Windows with the load generator on the same machine. The
methodology is in `docs/benchmarks/README.md`, and the figures are in
`examples/05-json-api/BENCHMARK.md`.

**Decision:** for Phase 2, benchmark hardware means that host. A later phase
that needs a stronger claim defines its hardware before it measures.

### Rows whose record already explains them

These rows have no section above:
- **`spawn_blocking` and `JoinHandle::cancel`:** §13 gives the reasons.
- **HTTPS, HTTP/2 and chunked transfer-encoding:** §6 puts them "Not in v1".
- **`std/process`'s `spawn` and `env`, and `std/regex`:** 2.5 lists those
  modules as optional, "as the server example demands", and §17 records that
  `spawn` and `env` do not exist.
- **The oneshot channel:** 2.3 names it beside `mpsc`, and its row is its
  record.
- **2.1's gate.** The 2.1 design narrowed the sub-phase (its §2) to a program
  that round-trips `Option` and `Result` and prints a custom `Display`, under
  `nova run` and `nova build`. That gate is met. The plan's "rewrite the
  Phase-1 examples" is not part of it, and nothing records it as done. This
  ADR records the drop, and the narrowed gate stands.

### The backlog

The backlog is unscheduled. Each item is built when a program needs it, and
the list promises no date, no phase and no order.
- `std/net`: UDP; Unix sockets.
- `std/http`: the client; a router beyond exact paths and `GET`.
- `std/sync`: atomics; `RwLock`; a oneshot channel.
- `std/collections`: `Queue`; `Deque`; `Vec::with_capacity`.
- `std/task`: `spawn_blocking`; `JoinHandle::cancel`.
- `std/crypto`: AEAD; BLAKE3, with a backing other than `ring`.
- `std/process`: `spawn`; `env`.
- `std/regex`.
- LLVM parity with Cranelift, and a release test that cannot pass without
  running.
- A benchmark of collection operations.
- Migrating the compiler's e2e fixtures to `nova test`.

Mapped to later phases:
- Phase 3 (Tooling): `salsa`.
- Phase 6 (1.0 Release, with its security audit): fuzz targets.

### Found outside this boundary

Met while closing Phase 2; recorded, not decided:
- ADR 0001 and the master spec's §1.1 say there is no JIT, while `nova run`
  JIT-compiles with Cranelift.
- Phase 1's LLVM backend emits textual IR rather than using `inkwell`, which
  the master spec's Phase 1 position 9 and `nova-spec/14-CODEGEN.md` §2.2
  name.

## Consequences

- With 20-STDLIB §18 and §19 (Definition of Done item 3) and this ADR
  (item 5), every item of §7's Definition of Done is met but the tag.
  Tagging `v0.2.0` completes Phase 2 within this boundary.
- What a `v0.2.0` program can rely on is the "shipped" rows, within the
  limits their records state.
- The backlog promises nothing.
- Dated notes point here from:
  - the master spec's §3, Phase 0 position 6, §5.2 and §6;
  - `nova-spec/10-LEXER.md`, `11-PARSER.md`, `13-RUNTIME.md`,
    `14-CODEGEN.md` §2.2 and `50-TESTING.md` §1.7;
  - `nova-spec/20-STDLIB.md`'s §1 and its gate notes in §7;
  - `docs/phase-2-plan.md`.
- The Phase 0 guides (`ARCHITECTURE.md`, `agent.md` and `skill.md`) and the
  parser's rustdoc are corrected in place instead, because they describe the
  code as it is now.
- The workspace `Cargo.toml` still declares `chumsky = "0.9"` and
  `rustyline = "14"`, which no crate uses.

## References

- Spec: `docs/superpowers/specs/2026-10-06-phase-2-closeout-design.md`
- `nova-spec/00-MASTER-SPEC.md` §3 and §7; `docs/phase-2-plan.md`
- `nova-spec/20-STDLIB.md` §1, §6, §8, §12, §13, §16, §17, §18 and §19
- `docs/superpowers/specs/2026-07-25-phase-2-1-std-core-design.md`
- ADRs 0001, 0009, 0013, 0014, 0015, 0016, 0017, 0018, 0019, 0021, 0023 and
  0024
```

- [ ] **Step 3: Watch `adr` pass, and keep `stdlib` passing**

Run: `cd /d/Projects/nona/nova && python -X utf8 C:/Users/SAKEER~1/AppData/Local/Temp/gcm/closeout/closeout_check.py stdlib adr && git diff --check`

Expected: `stdlib: OK (...)` and `adr: OK (...)`, and no whitespace errors. An `ABSENT` failure means a row says "not built" about something `std/` implements. Read the code, correct the row, and ledger a ruling.

- [ ] **Step 4: Commit**

Write this with the Write tool to `C:/Users/SAKEER~1/AppData/Local/Temp/gcm/_once/t2_commit.txt`:

```text
docs(adr): 0025, Phase 2's boundary

An inventory of every promise the master spec's §3 and the Phase 2 plan
make, with each one's status and record, so v0.2.0's boundary is
explicit. A section per deviation no earlier record decided, three of
them decided here (the hand-written parser stays; the four alpha tags
stand in for per-sub-phase tags; "benchmark hardware" is this
development host), and an unscheduled backlog, with salsa mapped to
Phase 3 and fuzzing to Phase 6.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
```

Run: `cd /d/Projects/nona/nova && git add docs/adr/0025-phase-2-boundary.md && git commit -q -F C:/Users/SAKEER~1/AppData/Local/Temp/gcm/_once/t2_commit.txt && git log --oneline -1`

Expected: one new commit, `docs(adr): 0025, Phase 2's boundary`.

---

### Task 3: Records: notes, in-place corrections, the CHANGELOG, and the sweep

**Files:** (all modify)
- `nova-spec/00-MASTER-SPEC.md` (four notes), `nova-spec/13-RUNTIME.md` (one), `nova-spec/20-STDLIB.md` (two)
- `nova-spec/10-LEXER.md`, `11-PARSER.md`, `14-CODEGEN.md`, `50-TESTING.md` (one each)
- `docs/phase-2-plan.md` (eleven notes)
- `ARCHITECTURE.md` (one cell), `agent.md` (three), `skill.md` (nine)
- `crates/nova-parser/src/lib.rs` (two rustdoc passages)
- `CHANGELOG.md` (`[Unreleased]`: one "Added" bullet)

**Interfaces:**
- Consumes: the path `docs/adr/0025-phase-2-boundary.md` and its section names (Task 2); §18 and §19 (Task 1).
- Produces: no code.

- [ ] **Step 1: Confirm `records` still fails**

Run: `cd /d/Projects/nona/nova && python -X utf8 C:/Users/SAKEER~1/AppData/Local/Temp/gcm/closeout/closeout_check.py records; echo "exit=$?"`

Expected: `exit=1`. Every `NOTES` file reports its count lower than expected. 20-STDLIB already has 2 of its 4, from §18 and §19.

- [ ] **Step 2: Apply every edit with one script**

Write this with the Write tool to `C:/Users/SAKEER~1/AppData/Local/Temp/gcm/_once/t3_records.py`. Every anchor below was checked on 2026-10-06 to match exactly once, at `1302480`. The script writes nothing unless all of them still do. No string in it contains a backslash.

````python
# ONE-TIME EDIT for the Phase 2 close-out's records (plan Task 3).
import sys
ROOT = "D:/Projects/nona/nova/"

def run(edits):
    state = {}
    for f, anchor, text, mode in edits:
        if f not in state:
            raw = open(ROOT + f, encoding="utf-8", newline="").read()
            state[f] = [raw, "\r\n" if "\r\n" in raw else "\n"]
        raw, nl = state[f]
        a = anchor.replace("\n", nl)
        t = text.replace("\n", nl)
        n = raw.count(a)
        if n != 1:
            sys.exit("ABORT before any write: %s: anchor matched %d times: %r" % (f, n, anchor[:70]))
        if mode == "after":
            state[f][0] = raw.replace(a, a + t)
        elif mode == "before":
            state[f][0] = raw.replace(a, t + a)
        else:
            state[f][0] = raw.replace(a, t)
    for f, (raw, nl) in state.items():
        open(ROOT + f, "w", encoding="utf-8", newline="").write(raw)
        print("wrote", f)

ADR25 = "`docs/adr/0025-phase-2-boundary.md`"
EDITS = []

# --- R2: the gate-family note, after the 2026-10-05 one in all four files ---
FAMILY = ("Phase 2 gate (§3, §4 and §5) now exists and passes. This note does not assess\n"
          "whether Phase 2 is complete.\n")
BOUNDARY = """
**Recorded 2026-10-06 (branch `phase-2-closeout`): Phase 2's boundary is
`docs/adr/0025-phase-2-boundary.md`.** Its inventory table lists every
promise the master spec's §3 and `docs/phase-2-plan.md` make for Phase 2,
whether it shipped, and where it is recorded; what is not built goes to an
unscheduled backlog. With that ADR and `nova-spec/20-STDLIB.md` §18 and §19,
every item of the master spec's §7 Definition of Done is met but the last,
the `v0.2.0` tag.
"""
for f in ("nova-spec/00-MASTER-SPEC.md", "nova-spec/13-RUNTIME.md",
          "nova-spec/20-STDLIB.md", "docs/phase-2-plan.md"):
    EDITS.append((f, FAMILY, BOUNDARY, "after"))

# --- the master spec ---
M = "nova-spec/00-MASTER-SPEC.md"
EDITS.append((M,
"6. `crates/nova-parser/` — see [11-PARSER.md], use **chumsky** (Pratt-style for expressions)\n",
"""   **Amended 2026-10-06 (branch `phase-2-closeout`):** the parser is
   hand-written recursive descent, with no chumsky and no Pratt combinator;
   """ + ADR25 + """ decides it stays.
""", "after"))
EDITS.append((M,
"- Fuzz targets in `fuzz/` for parser, lexer, JSON, regex\n",
"""  **Amended 2026-10-06 (branch `phase-2-closeout`):** there is no `fuzz/`
  directory; """ + ADR25 + """ maps fuzz targets to Phase 6.
""", "after"))
EDITS.append((M,
"tracing-subscriber = \"0.3\"\n```\n",
"""
**Amended 2026-10-06 (branch `phase-2-closeout`): eight crates in this list
are not in `Cargo.lock`, and no crate depends on them.**
- `chumsky`: the parser is hand-written;
  `docs/adr/0025-phase-2-boundary.md` decides it stays.
- `salsa`: ADR 0025 maps it to Phase 3.
- `inkwell`: the LLVM backend emits textual IR and calls `clang` or `llc`.
- `tokio`: `std/task` runs on Nova's own single-threaded executor
  (`docs/adr/0009-async-execution-model.md`).
- `wasm-encoder` and `walrus`: for Phase 4's WASM backend, position 1.
- `tower-lsp` and `rustyline`: for Phase 3's language server and REPL,
  positions 4 and 7.

The workspace `Cargo.toml` also declares `chumsky = "0.9"` and
`rustyline = "14"`, which no crate uses.
""", "after"))

# --- 11-PARSER, 10-LEXER, 14-CODEGEN, 50-TESTING (R3 for the fuzz clauses) ---
EDITS.append(("nova-spec/11-PARSER.md",
"- **Output:** `Spanned<Expr>`, `Spanned<Stmt>`, etc., with full span coverage\n",
"""
**Amended 2026-10-06 (branch `phase-2-closeout`): the parser does not use
chumsky.** It is hand-written recursive descent
(`crates/nova-parser/src/grammar.rs`). Operator precedence comes from explicit
layering, assignment down to postfix, not from a Pratt combinator. After an
error the parser skips to the next item boundary (an item keyword such as
`fn`, `pub` or `record`, or `}`) or statement boundary (`;` or `}`), the first
two levels §5 lists, and `parse` currently always returns `Some`. So the
first two bullets above, the grammar's "(Pratt)" label, §4's "rare with
chumsky" and §5's `recover_with` describe a design that was not built. §7's
fuzz target does not exist either: there is no `fuzz/` directory.
`docs/adr/0025-phase-2-boundary.md` decides the hand-written parser stays, and
maps fuzzing to Phase 6.
""", "after"))
EDITS.append(("nova-spec/10-LEXER.md",
"**Decision:** Use `logos` for the lexer. Chumsky for parser only. `logos` is ~2-3x faster and has explicit token regex patterns.\n",
"""
**Amended 2026-10-06 (branch `phase-2-closeout`):** the lexer uses `logos`, as
decided. The parser does not use chumsky either: it is hand-written
(`11-PARSER.md` §1's note), and `docs/adr/0025-phase-2-boundary.md` decides
it stays. §7's fuzz target does not exist; there is no `fuzz/` directory, and
ADR 0025 maps fuzzing to Phase 6.
""", "after"))
EDITS.append(("nova-spec/14-CODEGEN.md",
"- Use case: production binaries\n",
"""
**Amended 2026-10-06 (branch `phase-2-closeout`): there is no `inkwell`.**
`crates/nova-codegen-llvm` emits LLVM IR as text, and `nova-driver`'s
`link.rs` passes it to `clang`, or to `llc` when `clang` is missing. That holds
for §7's sketch and §9.1's `inkwell::debug_info` too. End to end, only hello
world is built and run in release, by a test that passes without running
anything when `clang` is not on `PATH`. Parity with the Cranelift backend is
deferred: """ + ADR25 + """.
""", "after"))
EDITS.append(("nova-spec/50-TESTING.md",
"- Run via `cargo fuzz run <target>` continuously in CI\n",
"""
**Amended 2026-10-06 (branch `phase-2-closeout`):** none of these targets
exists, there is no `fuzz/` directory, and §4.2's nightly fuzz job is not in
`.github/workflows/`. `docs/adr/0025-phase-2-boundary.md` maps fuzz targets
to Phase 6, beside its security audit.
""", "after"))

# --- docs/phase-2-plan.md ---
P = "docs/phase-2-plan.md"
EDITS.append((P,
"> and `nova-spec/20-STDLIB.md` / `13-RUNTIME.md`. Supersedes nothing yet.\n",
""">
> **Amended 2026-10-06 (branch `phase-2-closeout`): closed.**
> """ + ADR25 + """ records what each sub-phase below
> shipped, what it deferred, and where each deferred item went.
""", "after"))
EDITS.append((P,
"established loop: implement → tests → clippy/fmt → commit → adversarial-review\nworkflow → fix findings.\n",
"""
**Amended 2026-10-06 (branch `phase-2-closeout`):** each sub-phase was gated
and reviewed, and none was tagged. Four pre-release tags, `v0.2.0-alpha.1` to
`v0.2.0-alpha.4`, mark Phase 2's progress instead, and
`docs/adr/0025-phase-2-boundary.md` decides they stand in for per-sub-phase
tags.
""", "after"))
EDITS.append((P,
"  bound, and an `extern` runtime call compiles and runs under both backends.\n",
"""  **Amended 2026-10-06 (branch `phase-2-closeout`):** met in parts. The
  module-system commit, `8c37c79`, ran its programs under `nova run` and
  `nova build`, and both are Cranelift; only `nova build --release` uses LLVM.
  The four features are tested in separate programs
  (`tests/runtime/modules/`, `method_generics.nova`, `where_clauses.nova`,
  `extern_ffi.nova`), not in one. `docs/adr/0025-phase-2-boundary.md` puts
  LLVM parity on its backlog.
""", "after"))
EDITS.append((P,
"  round-trips `Option`/`Result` and custom `Display`.\n",
"""  **Amended 2026-10-06 (branch `phase-2-closeout`):** met as the 2.1 design
  narrowed it (`docs/superpowers/specs/2026-07-25-phase-2-1-std-core-design.md`
  §2): the round-trip and a custom `Display`, under `nova run` and
  `nova build`, both Cranelift. The narrowed gate leaves out rewriting the
  Phase-1 examples, and """ + ADR25 + """ records the drop.
""", "after"))
EDITS.append((P,
"  (`NOVA_GC_STRESS`) with correct output; benchmark basic ops.\n",
"""  **Amended 2026-10-06 (branch `phase-2-closeout`):** the stress half is met
  (`collections_under_gc_stress`). No benchmark of collection operations
  exists, and `Queue` was not built; both are on
  `docs/adr/0025-phase-2-boundary.md`'s backlog.
""", "after"))
EDITS.append((P,
"- **Gate:** a concurrent producer/consumer example with channels and timers\n  produces deterministic output.\n",
"""  **Amended 2026-10-06 (branch `phase-2-closeout`):** met by
  `examples/03-producer-consumer`. Of this sub-phase's list, `RwLock`,
  atomics and a oneshot channel were not built, while a bounded `channel` was
  (`docs/adr/0017-std-sync-channel-shape.md`).
  `docs/adr/0025-phase-2-boundary.md` puts the three on its backlog.
""", "after"))
EDITS.append((P,
"still routes by hand, because `Server` matches exact paths and `GET` only.\n",
"""
**Amended 2026-10-06 (branch `phase-2-closeout`):** the gate is met
(`docs/adr/0021-gate-ratio-paired-rounds.md`). What this sub-phase did not
build, UDP, the HTTP client and a router beyond exact paths and `GET`, is on
""" + ADR25 + """'s backlog.
""", "after"))
EDITS.append((P,
"- Optional: `std/crypto` (ring), `std/fs`, `std/process`, `std/regex` as the\n  server example demands.\n",
"""
**Amended 2026-10-06 (branch `phase-2-closeout`):** `std/test` and `nova test`
shipped. The fixture migration was not done, and is on
`docs/adr/0025-phase-2-boundary.md`'s backlog. ADR 0025 maps `salsa` to
Phase 3 and `fuzz/` targets to Phase 6, and decides the hand-written parser
stays rather than adopting chumsky. Of the optional modules, `std/crypto`,
`std/fs` and `std/process` shipped, the last with `args` and `exit` only;
`std/regex` did not.
""", "after"))
EDITS.append((P,
"- **Testing:** every module gets Nova programs run under both backends and under\n  `NOVA_GC_STRESS`; adversarial-review workflow after each substantial feature\n  (it found real bugs in every Phase-1 feature except the GC).\n",
"""  **Amended 2026-10-06 (branch `phase-2-closeout`):** not met for LLVM: the
  only program any test builds with `--release` is hello world. Per-module
  `NOVA_GC_STRESS` coverage was not assessed. LLVM parity is on
  """ + ADR25 + """'s backlog.
""", "after"))
EDITS.append((P,
"- **Backends:** both Cranelift (debug) and LLVM-IR (release) must stay in lockstep;\n  new MIR constructs (async state machines, FFI calls) need both.\n",
"""  **Amended 2026-10-06 (branch `phase-2-closeout`):** parity is unverified.
  The LLVM backend's tests check its IR text, and end to end it builds and runs
  only hello world, in a test that passes without running anything when
  `clang` is missing. `docs/adr/0025-phase-2-boundary.md` puts parity on its
  backlog.
""", "after"))

# --- 20-STDLIB §1 ---
EDITS.append(("nova-spec/20-STDLIB.md",
"ships, with `exit`; `spawn` and `env` do not exist.\n",
"""
**AMENDED 2026-10-06 (branch `phase-2-closeout`): §18 and §19 now cover
`std/strings` and `std/bytes`,** appended after §17 for the same reason.
`std/regex` alone has no section, and is not built. How much of each module
in this index shipped by `v0.2.0` is recorded in
`docs/adr/0025-phase-2-boundary.md`'s inventory table.
""", "after"))

# --- the Phase 0 guides, corrected in place (spec §3.3 and R1) ---
EDITS.append(("ARCHITECTURE.md",
"| `nova-parser` | Tokens → AST (uses `chumsky`) |\n",
"| `nova-parser` | Tokens → AST (hand-written recursive descent) |\n", "replace"))
EDITS.append(("agent.md",
"| `nova-parser` | Tokens → AST | `chumsky` |\n",
"| `nova-parser` | Tokens → AST | — (hand-written recursive descent) |\n", "replace"))
EDITS.append(("agent.md",
"| `nova-codegen-llvm` | Release-mode object files | `inkwell` |\n",
"| `nova-codegen-llvm` | Release-mode object files | — (textual LLVM IR, compiled by `clang` or `llc`) |\n", "replace"))
EDITS.append(("agent.md",
"| `nova-runtime` | GC + async runtime (Rust, linked) | `tokio` |\n",
"| `nova-runtime` | GC + async runtime (Rust, linked) | — (its own single-threaded executor, ADR 0009) |\n", "replace"))
EDITS.append(("skill.md",
"| `chumsky` 0.10 | Parser combinator library — read its guide first |\n",
"| `nova-parser`'s own code | Hand-written, with no parser library — read `grammar.rs` first |\n", "replace"))
EDITS.append(("skill.md",
"| Pratt parsing | Expression precedence (used for binary ops) |\n",
"| Precedence layering | One parsing level per precedence, assignment down to postfix |\n", "replace"))
EDITS.append(("skill.md",
"| Error recovery | `chumsky` supports recovery combinators — use them |\n",
"| Error recovery | Skip to the next item or statement boundary and keep going |\n", "replace"))
EDITS.append(("skill.md",
"| `salsa` incremental computation | Used for query-based compilation |\n",
"| `salsa` incremental computation | Not used yet; planned for Phase 3's language server |\n", "replace"))
EDITS.append(("skill.md",
"| `inkwell` crate | Rust LLVM bindings |\n",
"| Textual LLVM IR | The backend writes `.ll` text, which `clang` or `llc` compiles |\n", "replace"))
EDITS.append(("skill.md",
"| `tokio` async runtime | Task scheduling, `async`/`await` |\n",
"| Nova's own executor | Single-threaded task scheduling, `async`/`await` (ADR 0009) |\n", "replace"))
EDITS.append(("skill.md",
"| HTTP / TCP / UDP | `std/http`, `std/net` — backed by `hyper` in runtime |\n",
"| HTTP / TCP | `std/http` over `httparse` (ADR 0019), `std/net` (TCP; no UDP yet) |\n", "replace"))
EDITS.append(("skill.md",
"| Pratt parsing | `nova-parser` (expression precedence) |\n",
"| Precedence layering | `nova-parser` (expression precedence) |\n", "replace"))
EDITS.append(("skill.md",
"| nova-parser | ✓✓ | chumsky | — | — | — | — |\n",
"| nova-parser | ✓✓ | hand-written | — | — | — | — |\n", "replace"))

# --- the parser's rustdoc (comment-only) ---
EDITS.append(("crates/nova-parser/src/lib.rs",
"//! Uses `chumsky` for parser combinators with automatic error recovery.\n",
"//! A hand-written recursive-descent parser: after an error it skips to the next\n"
"//! item or statement boundary and keeps going.\n", "replace"))
EDITS.append(("crates/nova-parser/src/lib.rs",
"/// Returns `Some(File)` even when errors are present (thanks to chumsky\n"
"/// error recovery). Returns `None` only on catastrophic internal failures\n"
"/// (in practice, extremely rare).\n",
"/// Returns `Some(File)` even when errors are present: the parser skips to the\n"
"/// next item or statement boundary after an error and keeps going. It\n"
"/// currently never returns `None`.\n", "replace"))

# --- the CHANGELOG ---
EDITS.append(("CHANGELOG.md",
"## [Unreleased]\n\n### Added\n",
"""- **Phase 2's boundary is recorded, and two standard-library modules are
  documented.** `docs/adr/0025-phase-2-boundary.md` lists every promise
  Phase 2 made, whether it shipped, and where it is recorded. What was not
  built goes to an unscheduled backlog, with `salsa` mapped to Phase 3 and
  fuzzing to Phase 6. The ADR decides three questions the specs left open:
  the hand-written parser stays, the four alpha tags stand in for
  per-sub-phase tags, and "benchmark hardware" means this development host.
  `nova-spec/20-STDLIB.md` gains §18 `std/strings` and §19 `std/bytes`.
  Dated notes point at the ADR from the master spec, `docs/phase-2-plan.md`
  and the specs for the lexer, parser, runtime, codegen, testing and standard
  library. The Phase 0 guides' tables and the parser's rustdoc no longer name
  crates the code does not use.
""", "after"))

run(EDITS)
````

Run: `cd /d/Projects/nona/nova && python -X utf8 C:/Users/SAKEER~1/AppData/Local/Temp/gcm/_once/t3_records.py && mv C:/Users/SAKEER~1/AppData/Local/Temp/gcm/_once/t3_records.py C:/Users/SAKEER~1/AppData/Local/Temp/gcm/_once/t3_records.py.applied`

Expected: 13 `wrote` lines, one for each of `00-MASTER-SPEC.md`, `13-RUNTIME.md`, `20-STDLIB.md`, `phase-2-plan.md`, `11-PARSER.md`, `10-LEXER.md`, `14-CODEGEN.md`, `50-TESTING.md`, `ARCHITECTURE.md`, `agent.md`, `skill.md`, `crates/nova-parser/src/lib.rs` and `CHANGELOG.md`, and no `ABORT`.

If the script aborts, it has written nothing. Read the region it names, fix that anchor only, never a note, and re-run. Ledger `Task 3: Ruling: anchor <file> changed to <text>`.

- [ ] **Step 3: Watch `records` pass, and the other two stay passing**

Run: `cd /d/Projects/nona/nova && python -X utf8 C:/Users/SAKEER~1/AppData/Local/Temp/gcm/closeout/closeout_check.py all && git diff --check && git ls-files --eol nova-spec/00-MASTER-SPEC.md nova-spec/11-PARSER.md agent.md skill.md crates/nova-parser/src/lib.rs CHANGELOG.md`

Expected:
- `stdlib: OK`, `adr: OK` and `records: OK`;
- no whitespace errors;
- the CRLF files still `w/crlf` and the LF files still `w/lf`, as Global Constraints lists them.

- [ ] **Step 4: The rustdoc edit, as CI checks it**

Run: `cd /d/Projects/nona/nova && cargo fmt --all -- --check && echo FMT-OK && cargo clippy --locked -p nova-parser --all-targets -- -D warnings 2>&1 | tail -2 && cargo test --locked -p nova-parser 2>&1 | grep -E "^test result:|Doc-tests"`

Expected:
- `FMT-OK`;
- clippy `Finished`, with no warning;
- every `test result:` line `ok`, the `Doc-tests nova_parser` line among them, with its doctest passing.

- [ ] **Step 5: Run the sweep, and take the set difference**

Write this with the Write tool to `C:/Users/SAKEER~1/AppData/Local/Temp/gcm/closeout/closeout_sweep.py`. It is read-only and is not an edit script.

```python
# READ-ONLY sweep for the Phase 2 close-out. Lists, for every tracked text file
# outside docs/superpowers/ (dated records) and outside CHANGELOG.md's released
# sections, each line that matches a phrase; marks the files this branch changed
# as TOUCHED; then reports phrases found only after joining wrapped lines.
import re, subprocess
REPO = "D:/Projects/nona/nova"
PHRASES = [("chumsky", re.I), ("inkwell", re.I), (r"\bsalsa\b", 0), (r"\bfuzz", re.I),
           ("both backends", 0), ("not yet built", 0), (r"\bunbuilt\b", 0),
           ("still not complete", 0), (r"Phase 2 is (still )?not", 0), (r"\bQueue\b", 0),
           ("RwLock", 0), ("oneshot", 0), (r"\bUDP\b", 0), (r"\bclient\b", 0), (r"\bAEAD\b", 0),
           ("BLAKE3|blake3", 0), ("std/regex", 0), (r"\btokio\b", re.I), (r"\bPratt\b", 0)]
WRAPPED = ["both backends", "not yet built", "still not complete", r"Phase 2 is (still )?not"]
TEXT = (".rs", ".md", ".nova", ".c", ".yml", ".toml", ".txt", ".stdout", ".json", ".py", ".sh")
def git(*a):
    return subprocess.run(["git", "-C", REPO] + list(a), capture_output=True, text=True,
                          encoding="utf-8").stdout
touched = set(git("diff", "main", "--name-only").split())
for f in [f for f in git("ls-files").split("\n") if f]:
    if not f.endswith(TEXT) or f.startswith("docs/superpowers/") or f == "Cargo.lock":
        continue
    try:
        lines = open(f"{REPO}/{f}", encoding="utf-8").read().replace("\r\n", "\n").split("\n")
    except (UnicodeDecodeError, FileNotFoundError):
        continue
    stop = len(lines)
    if f == "CHANGELOG.md":
        stop = next((i for i, l in enumerate(lines) if l.startswith("## [0.")), stop)
    hits = [(i + 1, l.strip()[:140]) for i, l in enumerate(lines[:stop])
            if any(re.search(p, l, fl) for p, fl in PHRASES)]
    flat = " ".join(" ".join(lines[:stop]).split())
    wrapped = [p for p in WRAPPED
               if len(re.findall(p, flat)) > sum(len(re.findall(p, l)) for l in lines[:stop])]
    if hits or wrapped:
        print(f"== {'TOUCHED ' if f in touched else ''}{f} ({len(hits)})")
        for n, l in hits:
            print(f"  {n}: {l}")
        for p in wrapped:
            print(f"  WRAPPED: /{p}/ also matches across a line break")
```

Run: `cd /d/Projects/nona/nova && C=/c/Users/SAKEER~1/AppData/Local/Temp/gcm/closeout && python -X utf8 $C/closeout_sweep.py > $C/sweep.txt && grep -c "^== " $C/sweep.txt && grep "^== " $C/sweep.txt | grep -v TOUCHED`

Expected: about 55 files in all. The second command lists the set difference, the matched files this branch did not touch. For each one, and for every match left in a touched file:
- Read the matching lines in context. A `WRAPPED` line means a phrase also matches across a line break; find it by reading the file.
- Decide whether it is now **false**. A record of its own date's state is not false: the CHANGELOG's released sections, dated notes, and the bodies of ADRs and specs.
- Fix every false one in its own style: a dated note in a record, a rewritten comment in source. Re-run this step afterwards, and ledger each fix.

**What the planning sweep classified,** on 2026-10-06 before any edit. These are the 41 files in the set difference. Re-check each one; do not take the list on trust:
- **Recorded, not changed (record only):** `Cargo.toml`'s `chumsky = "0.9"`. ADR 0025 and the master spec's §6 note record it.
- **Locked-decision and feature lists whose deviation ADR 0009 records (R1):** "Tokio" in `README.md`, `nova-spec/README.md` and `docs/rfcs/0000-language-overview.md`. The same class covers the master spec's §1 and position 7 (touched files), and `agent.md`'s locked-decision line.
- **True as written:**
  - `crates/nova-codegen-llvm/src/lib.rs`: it says the backend does *not* link `inkwell`;
  - `crates/nova-parser/src/grammar.rs`: "chumsky-style" names a style (spec §3.3);
  - `std/crypto/lib.nova`: AEAD and BLAKE3 are not provided;
  - `std/sync/lib.nova`: "has no `Queue` to borrow";
  - `std/task/lib.nova` and `crates/nova-runtime/src/task.rs`: comparisons with tokio's `block_on`;
  - `crates/nova-runtime/src/file.rs` and `std/io/lib.nova`: "unbuilt", still unbuilt.
- **"Both backends" meaning Cranelift and LLVM:** `crates/nova-mir/src/async_lower.rs`, `lib.rs` and `lower.rs`, `crates/nova-mir/tests/lower_tests.rs`, and `crates/nova-codegen-llvm/tests/ir_tests.rs`. These are true.
- **"Both backends" meaning `nova run` and `nova build`,** the reading ADR 0025 records: two doc comments in `crates/nova-cli/tests/run_tests.rs`, and `docs/adr/0005-mutable-receivers-and-one-shot-hash.md`. These are true in that sense.
- **The verb "queue":** `crates/nova-driver/src/lib.rs` and `crates/nova-runtime/src/task.rs`'s "Queue a … future".
- **Dated ADR bodies, true at their date and not made false by a branch that builds nothing:** ADRs 0009, 0012, 0013, 0014, 0016, 0017, 0018, 0019 and 0022.
  - Their "unbuilt" items stay unbuilt.
  - ADR 0018's and 0019's "Phase 2 is not complete" stays true until the tag, and the release covers it (R4).
- **Network clients, or the unbuilt HTTP client, still true:**
  - `crates/nova-bench-http/src/main.rs` and `crates/nova-runtime/src/net.rs` and `poll.rs`;
  - `tests/runtime/http_keepalive.*` and `net_listener_accept.*`;
  - `std/http/lib.nova` and `docs/benchmarks/README.md`;
  - `nova-spec/30-FRONTEND.md`, client-side rendering;
  - `nova-spec/60-EXAMPLES.md`.
- **Phase 3 plans:** `nova-spec/40-TOOLING.md`'s `salsa`, consistent with ADR 0025's mapping.

**Matches expected to stay inside touched files:**
- **20-STDLIB:** §7's dated "Phase 2 is still not complete" lines, now followed by the boundary note; §6's, §8's, §13's and §16's own statements that items are unbuilt; §1's index lines.
- **The master spec:** position 7's "wrap Tokio" and position 9's UDP; §3's dated notes; Phase 1 position 9's `inkwell`, which ADR 0025 records as outside its boundary.
- **13-RUNTIME:** its dated amendments, which say there is no Tokio and that AEAD and BLAKE3 do not ship.
- **14-CODEGEN:** §7's and §9.1's `inkwell`, covered by §2.2's note.
- **11-PARSER:** the grammar's "(Pratt)" label, and §4's and §5's chumsky text, all covered by §1's note.
- **50-TESTING:** §4.2's fuzz job, covered by §1.7's note.
- **phase-2-plan:**
  - the original bullets;
  - the earlier dated notes, including "`04-todo-cli` still does not exist, so Phase 2 is still not complete", which the 2026-10-05 note after it supersedes;
  - "not yet built" in 2.4.
- **skill.md:** the tooling rows for `salsa` and fuzzing, which are skills for later phases.
- **ADR 0025:** every phrase, by design.

List in the task report every file you judged still true, and why.

- [ ] **Step 6: Commit**

Write this with the Write tool to `C:/Users/SAKEER~1/AppData/Local/Temp/gcm/_once/t3_commit.txt`:

```text
docs: point Phase 2's records at its boundary

Dated notes send every record that states Phase 2's scope to ADR 0025:
the master spec (the gate-family note, Phase 0's parser entry, §5.2's
fuzz targets, §6's dependency list), 13-RUNTIME, 20-STDLIB (§1, and the
gate-family note), 10-LEXER, 11-PARSER, 14-CODEGEN §2.2, 50-TESTING
§1.7, and the Phase 2 plan (its status, §4, each sub-phase and §5). The
Phase 0 guides' tables stop naming crates the code does not use
(chumsky, inkwell, tokio, salsa, hyper), and nova-parser's rustdoc
stops claiming chumsky; that is the only .rs change, comment-only. The
CHANGELOG gets one Added bullet.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
```

Run: `cd /d/Projects/nona/nova && git add -A nova-spec docs/phase-2-plan.md ARCHITECTURE.md agent.md skill.md crates/nova-parser/src/lib.rs CHANGELOG.md && git status --short && git commit -q -F C:/Users/SAKEER~1/AppData/Local/Temp/gcm/_once/t3_commit.txt && git log --oneline -1`

Expected:
- the status lists only the 13 files from Step 2, plus any that Step 5 fixed;
- one new commit, `docs: point Phase 2's records at its boundary`.

---

### Task 4: Final verification

**Files:** none change, unless a check fails. If one does, fix it in the task that owns the file, and re-run this task.

- [ ] **Step 1: The check, and nothing stray**

Run: `cd /d/Projects/nona/nova && python -X utf8 C:/Users/SAKEER~1/AppData/Local/Temp/gcm/closeout/closeout_check.py all && git diff --exit-code main -- Cargo.lock Cargo.toml 'crates/*/Cargo.toml' && echo MANIFESTS-UNCHANGED && git diff --check main && git status --short && git log --oneline main..HEAD`

Expected:
- `stdlib: OK`, `adr: OK` and `records: OK`;
- `MANIFESTS-UNCHANGED`;
- no whitespace errors and an empty status;
- six commits: the spec, its correction, this plan, and Tasks 1–3, plus any fix commits.

- [ ] **Step 2: The whole suite on Windows**

Check port 3000 (Conventions). Then run it in the background: `cd /d/Projects/nona/nova && cargo build -p nova-runtime 2>&1 | tail -1 && cargo build --locked --workspace 2>&1 | tail -1 && cargo test --locked --workspace --all-features --no-fail-fast > C:/Users/SAKEER~1/AppData/Local/Temp/gcm/closeout/t4_full.txt 2>&1`. Count it as in Conventions.

Expected: `1222 passed, 0 failed, 8 ignored`, PR #99's Windows count. The branch changes no behaviour, so a different count is a finding: read the failing tests before anything else. The known `0xC0000005` flake is in ADR 0008 §4. A rerun that passes is evidence about the flake, not about this branch: say so in the report.

- [ ] **Step 3: The fresh fact-check**

Dispatch a read-only agent on the most capable model (`model: "opus"`, `run_in_background: false`), with this prompt, verbatim:

```text
You are a read-only fact checker. Repository: D:\Projects\nona\nova (Git Bash:
/d/Projects/nona/nova), branch phase-2-closeout. Check every factual claim this
branch adds against the repository's current files, the code and git history:

1. docs/adr/0025-phase-2-boundary.md: every sentence and every table row.
2. nova-spec/20-STDLIB.md §18 and §19, its last two sections: every signature
   against std/strings/lib.nova and std/bytes/lib.nova, and every behaviour
   bullet against the code and against the tests it names in
   crates/nova-cli/tests/run_tests.rs and tests/runtime/.
3. Every line this branch adds elsewhere. Run
   git diff main -- nova-spec docs/phase-2-plan.md ARCHITECTURE.md agent.md skill.md crates/nova-parser/src/lib.rs CHANGELOG.md
   and read each added line in its file's context.

Rules:
- Read-only. Edit, create and delete nothing in the repository.
- Never run cargo, nova, a build or a test: a watchdog kills long commands.
- The only script you may run is
  python -X utf8 C:/Users/SAKEER~1/AppData/Local/Temp/gcm/closeout/closeout_check.py all
  Do not run any other script, in that directory or anywhere else.
- A claim you could not check is NOT CHECKED, never CONFIRMED.
- A claim that something does not exist is CONFIRMED only with the search you
  ran, quoted. A grep miss is not absence in prose: also search with the
  phrase's words allowed to wrap across lines.

Report:
- totals for CONFIRMED, WRONG, PARTLY and NOT CHECKED;
- for each WRONG and PARTLY claim: its file and line, the claim quoted, the
  evidence as file:line, and the correction;
- the NOT CHECKED claims, each with the reason.
```

Expected: a report with zero WRONG. For each WRONG or PARTLY claim:
- re-read the evidence yourself;
- correct the artifact in the task that owns it;
- re-run `closeout_check.py all`;
- commit as `docs: correct <what> after the fact-check`;
- ledger `Final: fixed <claim> — <evidence>`.

A NOT CHECKED claim you can settle, settle and ledger; one you cannot goes to the user in the final message.

- [ ] **Step 4: Hand off to the whole-branch review**

The execution skill's final review takes over from here, followed by `superpowers:finishing-a-development-branch`. Give the reviewer the fact-check's report beside the package. The user's standing practice is to push, open a PR and merge only on their word, by rebase, verifying tree identity.

The PR body:

```text
## Summary
- Records Phase 2's boundary in ADR 0025. Its inventory covers every promise
  the master spec's §3 and the Phase 2 plan make, whether it shipped, and
  where it is recorded. What was not built goes to an unscheduled backlog,
  with salsa mapped to Phase 3 and fuzzing to Phase 6. It decides three open
  questions: the hand-written parser stays, the four alpha tags stand in for
  per-sub-phase tags, and "benchmark hardware" means the development host the
  gate was met on.
- Documents `std/strings` (20-STDLIB §18) and `std/bytes` (§19).
- Dated notes point at ADR 0025 from the master spec, the Phase 2 plan and
  six other spec files. The Phase 0 guides' tables and nova-parser's rustdoc
  stop naming crates the code does not use.

Documentation only: the one `.rs` edit is two rustdoc passages in
`crates/nova-parser/src/lib.rs`. No manifest or `Cargo.lock` change, and no
new test.

## Test plan
- [ ] CI counts equal PR #99's: windows 1222/0/8, ubuntu 1216/0/9, macOS 1217/0/8
- [x] Windows suite locally: <count>
- [x] `closeout_check.py all`, the read-only structural check: OK
- [x] A fresh read-only agent checked every claim: <totals>
- [x] `cargo fmt --check`, `cargo clippy -D warnings` and `cargo test -p nova-parser`, doctest included

The `v0.2.0` release follows on its own branch after this merges; the tag is
pushed only on the maintainer's word.

🤖 Generated with [Claude Code](https://claude.com/claude-code)
```

**The branch is not done until CI passes on all three operating systems** with the counts above. Read CI once when the user returns, through `mcp__ccd_pr__get_status`; never poll.

---

## After the merge: the `v0.2.0` release

**This is not a task of this branch.** Start it only after the user has merged this branch's PR. It is spec §3.6, with ruling R4. It has two stops: the merge of its own PR, and the push of the tag. Each needs the user's explicit word, given for that step.

- [ ] **R1. Start from the merged `main`**

Run: `cd /d/Projects/nona/nova && git fetch origin && git checkout main && git merge --ff-only origin/main && git log --oneline -1 && python -X utf8 C:/Users/SAKEER~1/AppData/Local/Temp/gcm/closeout/closeout_check.py adr records && git checkout -b release-0.2.0`

Expected: `main` at the rebased close-out commits, both parts `OK`, and a new branch. If `closeout_check.py` is gone, skip it and say so.

- [ ] **R2. Bump the workspace crates from 0.1.0 to 0.2.0**

Write this with the Write tool to `C:/Users/SAKEER~1/AppData/Local/Temp/gcm/_once/r2_bump.py`:

```python
# ONE-TIME EDIT: every crate's package version, 0.1.0 -> 0.2.0, exactly once each.
import glob, sys
ROOT = "D:/Projects/nona/nova/"
files = sorted(glob.glob(ROOT + "crates/*/Cargo.toml"))
if len(files) != 21:
    sys.exit("ABORT before any write: expected 21 crate manifests, found %d" % len(files))
out = {}
for f in files:
    raw = open(f, encoding="utf-8", newline="").read()
    nl = "\r\n" if "\r\n" in raw else "\n"
    a = nl + 'version = "0.1.0"' + nl
    if raw.count(a) != 1:
        sys.exit("ABORT before any write: %s has %d package-version lines" % (f, raw.count(a)))
    out[f] = raw.replace(a, nl + 'version = "0.2.0"' + nl)
for f, raw in out.items():
    open(f, "w", encoding="utf-8", newline="").write(raw)
print("bumped", len(out), "manifests")
```

Run: `cd /d/Projects/nona/nova && python -X utf8 C:/Users/SAKEER~1/AppData/Local/Temp/gcm/_once/r2_bump.py && mv C:/Users/SAKEER~1/AppData/Local/Temp/gcm/_once/r2_bump.py C:/Users/SAKEER~1/AppData/Local/Temp/gcm/_once/r2_bump.py.applied && cargo update --workspace --offline 2>&1 | tail -3 && git diff Cargo.lock | grep -c '^+version = "0.2.0"' && git diff --stat | tail -1 && git grep -n -F '0.1.0' -- . ':!CHANGELOG.md' ':!docs/superpowers/*' ':!Cargo.lock'`

Expected:
- `bumped 21 manifests`;
- `21` lock entries now at `0.2.0`, with `22 files changed`;
- the only `0.1.0` left is `nova-spec/40-TOOLING.md`'s example `nova.toml`.

If `--offline` cannot resolve, re-run `cargo update --workspace` without it and say so.

- [ ] **R3. The "Phase 2 is complete" note, in the four gate-family files (R4)**

Write a one-time script, in the same shape as Task 3's `t3_records.py`. It inserts this text after the boundary note in each of `nova-spec/00-MASTER-SPEC.md`, `13-RUNTIME.md`, `20-STDLIB.md` and `docs/phase-2-plan.md`. The anchor is the boundary note's last line, `Definition of Done is met but the last, the \`v0.2.0\` tag.\n` (the fact-check rewrapped the note; see the ledger). Replace `<DATE>` with the day you run it.

```text

**Recorded <DATE> (branch `release-0.2.0`): Phase 2 is complete, as
`v0.2.0`.** Every item of the master spec's §7 Definition of Done is met,
within `docs/adr/0025-phase-2-boundary.md`'s boundary.
```

Expected: four `wrote` lines.

- [ ] **R4. Finalize the CHANGELOG, last**

Edit `CHANGELOG.md` with the Edit tool. Replace `## [Unreleased]\n\n### Added\n` with the following, `<DATE>` being the day of this commit:

```text
## [Unreleased]

## [0.2.0] - <DATE>

**Phase 2, "Standard Library Core", is complete**, within the boundary
`docs/adr/0025-phase-2-boundary.md` records. This is the `v0.{phase}.0`
milestone `nova-spec/00-MASTER-SPEC.md` §7 reserves for a completed phase:
the standard library the master spec's Phase 2 list names, a 10k+ req/sec
`examples/05-json-api` on the development host the gate was measured on,
and the gate examples 03, 04 and 05. What Phase 2 did not build is on that
ADR's backlog. `nova --version` now reports 0.2.0.

### Added
```

- [ ] **R5. The whole suite on Windows, and the version**

Check port 3000, then run the full suite as in Task 4 Step 2, into `.../closeout/r5_full.txt`, and `./target/debug/nova.exe --version`.

Expected: `1222 passed, 0 failed, 8 ignored`, and `nova 0.2.0`.

- [ ] **R6. Commit in three, the finalize commit last**

Write each message with the Write tool:
1. `chore: bump the workspace crates to 0.2.0`, covering the 21 manifests and `Cargo.lock`;
2. `docs: record that Phase 2 is complete`, covering the four notes;
3. `docs: finalize CHANGELOG for the v0.2.0 release`, covering `CHANGELOG.md` alone. The tag goes on this commit.

Each message ends with `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`. Then `git log --oneline main..HEAD` shows exactly these three.

- [ ] **R7. Push and open the PR. STOP for the user's word to merge.**

Push `release-0.2.0`, open a PR against `main` and report its URL. Read CI once, when the user returns, through `mcp__ccd_pr__get_status`; never poll. Merge only on the user's word, by rebase. Then verify by tree identity: `git rev-parse origin/main^{tree}` equals the PR head's tree.

- [ ] **R8. Make the annotated tag locally. STOP for the user's word to push it.**

Find `main`'s copy of the finalize commit: `git log --oneline -3 origin/main` shows `docs: finalize CHANGELOG for the v0.2.0 release` on top. Write the tag message with the Write tool to `C:/Users/SAKEER~1/AppData/Local/Temp/gcm/_once/r8_tag.txt`:

```text
Nova v0.2.0 — Phase 2 (Standard Library Core)

Phase 2 is complete, within the boundary docs/adr/0025-phase-2-boundary.md
records: the standard library the master spec's Phase 2 list names, its
gate met on the development host it was measured on, and the gate examples
03, 04 and 05. What it did not build is on that ADR's unscheduled backlog.
```

Run: `cd /d/Projects/nona/nova && git tag -a v0.2.0 <sha> -F C:/Users/SAKEER~1/AppData/Local/Temp/gcm/_once/r8_tag.txt && git cat-file -p v0.2.0 | head -6 && git show v0.2.0:crates/nova-cli/Cargo.toml | grep -m1 '^version'`

Expected: the tag points at the finalize commit, and the tagged `nova-cli` says `version = "0.2.0"`.

**Do not push the tag without the user's explicit word for that push.** Pushing it starts `release.yml`, which builds and uploads four artifacts publicly. On their word: `git push origin v0.2.0`. When the user returns, read the run once with `gh run list --workflow release.yml --limit 1` and confirm its four artifacts.
