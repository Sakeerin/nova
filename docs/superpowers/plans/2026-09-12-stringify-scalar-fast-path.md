# `stringify` Scalar Fast Path Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make `std/json`'s `stringify` return a top-level scalar's finished rendering directly, instead of copying it character by character into a second buffer and draining that.

**Architecture:** A `match` at the top of `stringify` returns early for `Null`, `Bool`, `Number` and `String`; an empty catch-all arm lets `Array` and `Object` fall through to the unchanged general path. No public API is added, no caller changes, and no Nova example is touched.

**Tech Stack:** Nova (`std/json/lib.nova`); the existing `tests/runtime/*.nova` + `.stdout` golden fixtures driven from `crates/nova-cli/tests/run_tests.rs`.

**Spec:** `docs/superpowers/specs/2026-09-12-stringify-scalar-fast-path-design.md`

## Global Constraints

- **Never push, merge or tag without explicit instruction from the human partner.**
- **A std edit is invisible until `cargo build --release` reruns.** Every `std/*/lib.nova` is `include_str!`'d into `crates/nova-resolver/src/lib.rs` at Rust build time. A probe that appears to show "no change" may be running the old embedded std. Rebuild, then verify the rebuild took.
- `cargo build --locked --workspace` BEFORE `cargo test --workspace --no-fail-fast`.
- Never pipe cargo output through `head`/`tail` before summing. Sum EVERY `test result:` line. Baseline: **1130 passed / 0 failed / 8 ignored across 45 targets**.
- No `reason = "..."` in any lint attribute (MSRV 1.78).
- CI's clippy gate is `cargo clippy --locked --all-targets --all-features -- -D warnings`, on ubuntu AND windows. `cargo fmt --all -- --check` must pass.
- The 8 ignored ADR-0010 GC tests stay ignored and untouched.
- The poll ABI is FROZEN and no panic may cross a generated poll boundary.
- Commit messages written to a UTF-8 file and applied with `git commit -F`, NEVER a heredoc. Every body ends exactly `Co-Authored-By: Claude Opus 5 <noreply@anthropic.com>`.
- Cite no SHA that is not already an ancestor of `main`. `b9dc13d` is. The spec commit is BRANCH-LOCAL and must not appear in any tracked file.
- Byte-scan every file written via `git show :<path>` (staged, not working tree — `core.autocrlf` smudges it): valid UTF-8, no byte below 0x20 outside tab/CR/LF, no 0x7f, ZERO backslash-`u`-four-hex in tracked markdown (write `U+XXXX`). Plant positives proving the scanner fires before trusting it.
- **Do NOT author backslashes through a heredoc or `python -c`.** A quoted bash heredoc has eaten a backslash level repeatedly on this project, including inside verification commands. Use the Write tool, and `grep -F` for literal checks. Verify authored escapes by reading BYTES (`od -c`), never by grepping for your own reconstruction.
- **Measurement discipline:** compiled release-runtime binary with its **byte size recorded** beside every figure, ONE FRESH PROCESS PER DATA POINT, and a **range or an explicit statement that it was one run**.
- **Do NOT predict a server throughput figure.** The isolated-to-whole-server amplification is 2.0x to 3.2x and unexplained.
- Sentence-shape discipline on every comment and record: prefer a roster with no count; never write that something is checked without naming what executes the check.

---

## File Structure

| File | Status | Responsibility |
|---|---|---|
| `std/json/lib.nova` | modify | The fast path, plus a note in the file's own cost commentary. |
| `tests/runtime/json_stringify.nova` | modify | Two variant-coverage cases it is currently missing. |
| `tests/runtime/json_stringify.stdout` | modify | Its golden gains the two matching lines. |
| `examples/05-json-api/BENCHMARK.md` | modify | The `users_json` figure, and a dated note that the server figures describe a superseded build. |
| `CHANGELOG.md` | modify | `[Unreleased]` entry. |

**No new test fixture, and no new Rust test — that is a finding, not an omission.** The spec's §5 proposed a new test on the basis that nothing compares the fast path against the general one. Reading the fixtures shows existing goldens already pin almost all of it:

- `tests/runtime/json_stringify.nova` renders **all four scalar arms at top level** (`Null`, both `Bool`s, a finite `Number`, a non-finite `Number` via `1.0 / 0.0`, and `String("hello")`) **and** an `Array` containing scalars plus a nested array, a single-key `Object`, and an empty `Object`. Its golden is `null`, `true`, `false`, `3`, `3.5`, `null`, `"hello"`, `[null,1,[2,3]]`, `{"only":[1,2]}`, `{}`. That is the scalar coverage AND the load-bearing container case the spec asks for.
- `tests/runtime/json_stringify_escapes.nova` pins a quote, a backslash, a newline, a tab, and the control character `0x01` rendering as a six-character `u`-escape.
- `tests/runtime/json_parse_strings.nova` pins backspace, form feed and carriage return in the encode direction, per the escapes fixture's own header.

So the fast path is **already gated** for every case except two, and those two are gaps that exist independently of this change: **the empty string**, and **an astral-plane character**. Task 2 adds exactly those to the variant fixture rather than duplicating coverage in a new one.

**How this meets the spec's success criterion 2**, which asks that "the new test covers every row of §5's table, including a container holding scalars": every row is covered, but by the fixtures named above plus Task 2's two additions — not by one new test. The criterion is met in substance and the mapping is stated here so a reviewer reads it as a finding rather than a gap. If a reviewer disagrees and wants a dedicated fixture, that is a ruling for whoever executes this, recorded either way.

---

### Task 1: Baseline, measured the way the "after" will be

**Files:**
- Create (scratch, NOT committed): a measurement harness and a results file, both outside the repository.
- No tracked file changes. **No commit in this task.**

**Interfaces:**
- Consumes: nothing.
- Produces: baseline figures in a scratch results file that Task 3 reads — `stringify` per call at the `String` arm, and `users_json` at 5, 10, 20 and 40 users, each with the harness binary's byte size.

- [ ] **Step 1: Confirm the tree is unmodified before measuring a baseline**

```bash
git status --porcelain=v1
git diff main --name-only
```

Expected: both empty. A baseline measured against an already-edited tree is not a baseline. If either prints anything, stop and report.

- [ ] **Step 2: Build the workspace so the harness compiles against current std**

```bash
cargo build --release --locked --workspace
```

- [ ] **Step 3: Write the harness**

Use the Write tool. It needs the example's own record shapes and `user_json`, because `users_json`'s cost is what the caller sees. Write it to **`/tmp/bench_stringify.nova`** — under Git Bash `/tmp` maps to the user's temp directory, which is outside the repository, and both tasks below name that exact path.

```nova
record User { id: Int, name: String, email: String }

record Store { users: Map<Int, User>, next_id: Int }

impl Store {
    fn create(mut self, name: String, email: String) -> User {
        let id = self.next_id
        self.next_id = id + 1
        let u = User { id: id, name: name, email: email }
        self.users.insert(id, u)
        u
    }
}

fn user_json(u: User) -> String {
    "{\"id\":${u.id},\"name\":${stringify(String(u.name))},\"email\":${stringify(String(u.email))}}"
}

fn users_json(s: Store) -> String {
    let mut out = "["
    let mut id = 1
    let mut first = true
    while id < s.next_id {
        match s.users.get(id) {
            Some(u) => {
                if !first { out = "${out}," }
                out = "${out}${user_json(u)}"
                first = false
            }
            None => {}
        }
        id = id + 1
    }
    "${out}]"
}

fn seed(n: Int) -> Store {
    let mut s = Store { users: Map::new(), next_id: 1 }
    let mut i = 0
    while i < n {
        let x = s.create("User ${i}", "user${i}@example.com")
        i = i + 1
    }
    s
}

fn bench_stringify(iters: Int) {
    let s = "User 1"
    let t = Instant::now()
    let mut acc = 0
    let mut i = 0
    while i < iters {
        acc = acc + stringify(String(s)).len()
        i = i + 1
    }
    let e = t.elapsed()
    println("stringify_string iters=${iters} ns_per_call=${e.nanos / iters} acc=${acc}")
}

fn bench_users_json(n: Int, iters: Int) {
    let s = seed(n)
    let probe = users_json(s)
    let t = Instant::now()
    let mut acc = 0
    let mut i = 0
    while i < iters {
        acc = acc + users_json(s).len()
        i = i + 1
    }
    let e = t.elapsed()
    println("users_json users=${n} bytes=${probe.len()} ns_per_call=${e.nanos / iters} acc=${acc}")
}

fn main() {
    // Warm before any clock is read.
    let w = users_json(seed(10))
    println("warm=${w.len()}")

    bench_stringify(5000)
    bench_users_json(5, 2000)
    bench_users_json(10, 2000)
    bench_users_json(20, 1000)
    bench_users_json(40, 500)
}
```

- [ ] **Step 4: Build the harness and record exactly one binary**

```bash
rm -f /tmp/sjbase /tmp/sjbase.exe
./target/release/nova build /tmp/bench_stringify.nova -o /tmp/sjbase.exe
ls -l /tmp/sjbase*
```

Expected: exactly ONE file. **Record its byte size** — every figure below is quoted with it. Build to the `.exe` name: spawning an extensionless path fails on this host, and two files sharing a stem with one stale is the arrangement that produced this project's withdrawn benchmark figure.

- [ ] **Step 5: Take the baseline, three runs, each a fresh process**

```bash
/tmp/sjbase.exe
/tmp/sjbase.exe
/tmp/sjbase.exe
```

Three separate invocations, so each figure comes from its own process. Record all three per cell and report each cell as a **range**, not a mean.

- [ ] **Step 6: Write the results to a scratch file**

Record, for each of the five cells (`stringify` per call, and `users_json` at 5/10/20/40): the three values, the range, the harness binary's byte size, and that `main` was at `b9dc13d` unmodified. **No commit.**

---

### Task 2: The fast path, its two missing test cases, and the mutation

**Files:**
- Modify: `std/json/lib.nova` — `stringify` at :229, plus the file's cost commentary
- Modify: `tests/runtime/json_stringify.nova`
- Modify: `tests/runtime/json_stringify.stdout`

**Interfaces:**
- Consumes: Task 1's baseline figures and its harness binary path.
- Produces: `stringify` with a scalar fast path; behaviour unchanged for every input.

- [ ] **Step 1: Add the two missing cases to the variant fixture FIRST, and watch them fail**

Use the Write tool for the fixture — the astral-plane and quote cases carry bytes a heredoc mangles.

Append to `tests/runtime/json_stringify.nova`'s `main`, after the empty-`Object` line:

```nova
    // Two variant-coverage cases this fixture was missing, independent of any
    // fast path: an empty string, and a character outside the basic multi-
    // lingual plane. The runtime's own
    // `str_from_chars_round_trips_and_substitutes_invalid_scalars` pins that
    // an astral-plane pair survives a `[Char]` round trip, and this pins that
    // `stringify` does not escape it.
    println(stringify(String("")))
    println(stringify(String("ok 🦀")))
```

Do NOT update the golden yet.

- [ ] **Step 2: Run the fixture's test and confirm it fails on the golden mismatch**

The Rust test driving this fixture is `json_stringify_run`. Name it exactly: the shorter filter `json_stringify` matches three tests, including the escapes fixture and the cycle-panic test, which are not what this step is about.

```bash
cargo test --locked -p nova-cli --test run_tests json_stringify_run -- --nocapture
```

Expected: FAIL, because stdout now has two more lines than `json_stringify.stdout`. This proves the golden is actually compared rather than assumed. Record what it said.

- [ ] **Step 3: Update the golden to the two new expected lines**

Append to `tests/runtime/json_stringify.stdout`, matching the fixture's `println` order: a line holding two double-quote characters, then a line holding `"ok 🦀"` with its quotes.

**Verify by bytes, not by grep:** `od -c tests/runtime/json_stringify.stdout | tail -5` and confirm the emoji is its real four-byte UTF-8 sequence and the empty-string line is exactly two quote characters. This file is CRLF-terminated like its siblings — check `cat -A` shows `^M$` on the new lines too.

- [ ] **Step 4: Re-run and confirm the fixture passes against the unmodified `stringify`**

```bash
cargo test --locked -p nova-cli --test run_tests json_stringify_run -- --nocapture
```

Expected: PASS. The two new cases now describe current behaviour, so any fast path that changes them will fail — which is the point of adding them before the code.

- [ ] **Step 5: Add the fast path**

In `std/json/lib.nova`, `stringify` currently begins:

```nova
pub fn stringify(v: JsonValue) -> String {
    let mut out: Vec<Char> = Vec::new()
    let mut stack: Vec<Work> = Vec::new()
    stack.push(Value(Pending { v: v, d: 0 }))
```

Insert the fast path ahead of those allocations:

```nova
pub fn stringify(v: JsonValue) -> String {
    // A TOP-LEVEL SCALAR IS ALREADY FINISHED, so return it rather than
    // copying it twice. The general path below hands `buf_push_str` exactly
    // these values, and `buf_push_str` appends them character by character
    // into `out`, which `vec_chars_to_string` then drains back into a
    // `String` -- two round trips through a `Vec<Char>`, plus two `Vec`
    // allocations and the `Work`/`Pending` wrappers, for a value that needs
    // none of them.
    //
    // Byte-identical, argued in this increment's spec rather than assumed:
    // the depth guard cannot fire here because it tests
    // `d > MAX_RENDER_DEPTH` and a top-level value carries `d = 0`;
    // `buf_push_str` into an empty buffer is a pure copy; and draining those
    // characters back is lossless for anything already a `String`, which
    // `crates/nova-runtime/src/lib.rs`'s
    // `str_from_chars_round_trips_and_substitutes_invalid_scalars` pins.
    //
    // `tests/runtime/json_stringify.nova` is what checks it: that fixture
    // renders each arm below at top level AND inside an `Array` and an
    // `Object`, so deleting an arm here fails it rather than passing
    // silently. Only the `String` arm's saving was measured.
    match v {
        Null => return "null"
        Bool(b) => return if b { "true" } else { "false" }
        Number(n) => return number_to_json(n)
        String(s) => return quote(s)
        _ => {}
    }
    let mut out: Vec<Char> = Vec::new()
    let mut stack: Vec<Work> = Vec::new()
    stack.push(Value(Pending { v: v, d: 0 }))
```

Leave the rest of the function, including the four scalar arms inside `match p.v`, exactly as they are: they stay reachable from inside a container.

- [ ] **Step 6: Rebuild, because a std edit is invisible until you do**

```bash
cargo build --release --locked --workspace
```

Then verify the rebuild actually took, rather than assuming:

```bash
./target/release/nova run tests/runtime/json_stringify.nova
```

Expected: the same twelve lines as the golden. If the output is unchanged from before the edit that is expected here — the point of this step is that the binary now contains the edited std, which the next step's mutation will prove.

- [ ] **Step 7: Run the whole suite**

```bash
cargo build --locked --workspace
cargo test --workspace --no-fail-fast
```

Sum EVERY `test result:` line. Expected: **1130 passed / 0 failed / 8 ignored across 45 targets**, unmoved — the fixture gained cases but its Rust test count did not.

- [ ] **Step 8: The mutation — run it and report what happened**

Change the fast path's `String` arm to return the string unquoted:

```nova
        String(s) => return s
```

Rebuild (`cargo build --release --locked --workspace`) and run the suite. **Report what actually failed and what it printed — do not predict it.** Then restore `return quote(s)`, rebuild, and confirm the suite is green again.

If nothing fails, that is a finding about the tests, not a licence to proceed: say so plainly.

- [ ] **Step 9: Measure the after, same harness, same discipline**

Re-run Task 1's harness binary — **rebuilt**, since it embeds std through the compiler:

```bash
rm -f /tmp/sjafter /tmp/sjafter.exe
./target/release/nova build /tmp/bench_stringify.nova -o /tmp/sjafter.exe
ls -l /tmp/sjafter*
/tmp/sjafter.exe
/tmp/sjafter.exe
/tmp/sjafter.exe
```

Record the byte size and three runs per cell, as ranges. Append to the same scratch results file beside the baseline.

- [ ] **Step 10: Byte-scan and commit**

Stage by name — never `git add -A` or `git add .`:

```bash
git add std/json/lib.nova tests/runtime/json_stringify.nova tests/runtime/json_stringify.stdout
```

Byte-scan the STAGED content via `git show :<path>`, with planted positives proving the scanner fires first. Note that `.stdout` is a golden with CRLF line endings and a real four-byte emoji — both are expected and neither is a control byte outside tab/CR/LF.

Write the commit message to a UTF-8 file with the Write tool and apply it with `git commit -F`. The body must record the mutation's actual outcome and state that only the `String` arm's saving was measured.

---

### Task 3: The records

**Files:**
- Modify: `examples/05-json-api/BENCHMARK.md`
- Modify: `CHANGELOG.md`
- Modify: `std/json/lib.nova` — its cost commentary only

**Interfaces:**
- Consumes: Task 1's baseline and Task 2's after-figures from the scratch results file.
- Produces: the tracked record.

- [ ] **Step 1: `examples/05-json-api/BENCHMARK.md` — the moved figure**

Its response-side decomposition records `users_json` at 158,399 ns and `stringify` inside it. Amend that with the before/after ranges from the results file, each with its harness binary byte size and the note that three runs were taken per cell. State explicitly that only the `String` arm was measured.

- [ ] **Step 2: `examples/05-json-api/BENCHMARK.md` — the superseded-build note**

A dated note recording that this file's **server** figures — 1875.2 to 3108.5 req/sec for the absolute criterion, and 0.116 to 0.231 for the Bun ratio — were measured through the previous `stringify` and therefore describe a superseded build.

State the mechanism that makes this checkable rather than only asserted: those figures are recorded beside the example binary's byte size, 690,176, so a reader who rebuilds and compares sizes sees the mismatch. And state that **re-measuring the server is deferred with a reason** — it is the full four-cell matrix with replicates plus the equivalence check — **not silently owed**.

**The owed re-measurement must be named in this tracked file, not only in a queue outside the repository.** The project's debt queue lives in the controller's memory, which an implementer reading this plan cannot see and a future reader of the repository cannot either. A tracked record that says "deferred" without saying where the debt is recorded is how debt becomes invisible.

Do NOT state a new throughput figure or a predicted one.

- [ ] **Step 3: `std/json/lib.nova`'s cost commentary**

The file's existing notes describe migrating its accumulators from interpolation to a `Vec<Char>` drained once, and carry that migration's measurements. Add a short note naming this increment as a **separate, later** saving on the same function: the accumulator migration made the general path linear where it had been quadratic; this skips the general path entirely for a top-level scalar. Name what checks it — `tests/runtime/json_stringify.nova` — and that only the `String` arm's saving was measured.

- [ ] **Step 4: `CHANGELOG.md` under `[Unreleased]`**

An entry naming: the fast path and which arms it covers; that no public API was added and why an earlier design's `quote`-exposure was dropped; the before/after ranges for `stringify` per call and `users_json`; that only the `String` arm was measured; the mutation's actual outcome; and that the server figures in `BENCHMARK.md` now describe a superseded build with re-measurement deferred.

- [ ] **Step 5: Verify, byte-scan and commit**

```bash
cargo build --locked --workspace
cargo test --workspace --no-fail-fast
cargo fmt --all -- --check
cargo clippy --locked --all-targets --all-features -- -D warnings
```

Sum EVERY `test result:` line — expect **1130 passed / 0 failed / 8 ignored across 45 targets**.

Byte-scan the staged content via `git show :<path>` with planted positives first, checking for zero backslash-`u`-four-hex in the markdown — write `U+XXXX` if a code point needs naming. Stage each file by name; commit with `git commit -F` from a UTF-8 file.

---

## What this plan does not do

- **No new public API.** Nothing added, renamed or exposed; the earlier `quote`-exposure design was dropped for the reason `std/strings`' `join` comment gives.
- **No change to `examples/05-json-api/src/main.nova`.** Its accumulator was measured at about 4.5% of `users_json` and left alone.
- **No new test fixture.** Existing goldens already pin all four arms at top level and inside containers; Task 2 adds only the two cases genuinely missing.
- **No `nova-spec/` amendment.** No gate, criterion or specified behaviour changes.
- **No server re-measurement, and no predicted throughput figure.** Deferred with a reason and recorded as debt.
- **The `Array` and `Object` arms are untouched.** They need the work stack; a nested scalar still pays one round trip through the shared buffer, which is the design rather than waste.
