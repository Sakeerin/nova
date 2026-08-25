# `std/json` Hardening Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Give `parse` a declared depth cap, make `stringify` iterative with a cycle guard, and replace every interpolation accumulator in `std/json` with one growable `Vec<Char>` buffer.

**Architecture:** All production changes land in `std/json/lib.nova`. Two module-private helpers (`buf_push_str`, `vec_chars_to_string`) turn `Vec<Char>` plus `str_from_chars` into a growable string buffer; the four accumulators write into it. `parse` threads an `Int` depth parameter and rejects past `MAX_DEPTH`. `stringify` becomes a pop-loop over a `Vec<Work>` with a nesting-depth guard. **No new intrinsic**, so the 12-site checklist is not paid and `STD_ONLY`, `STD_MODULES` and `RtFunc` are untouched.

**Tech Stack:** Nova (`std/json/lib.nova`), `Vec<T>` from `std/collections` (glob-imported), the `str_from_chars` builtin, Rust only in `crates/nova-cli/tests/run_tests.rs` for fixture registration.

**Spec:** `docs/superpowers/specs/2026-08-25-std-json-hardening-design.md`

## Global Constraints

- `cargo build --locked --workspace` **before** `cargo test`. `--no-fail-fast`.
- **Sum every `test result:` line across all 44 targets. Never pipe cargo output through `head` or `tail`.** Baseline: **1066 passed / 0 failed / 8 ignored**.
- Clippy `--all-targets -- -D warnings` on ubuntu **and** windows; `cargo fmt --all -- --check`.
- **MSRV 1.78: no `reason = "..."` in any lint attribute.**
- The ignored ADR-0010 GC tests stay ignored and untouched. Census: **8 unconditional attributes plus one conditional**; runtime counts **Windows 8, macOS 0, Linux 1**. CI's advisory `--ignored` step is red on Linux **by design**.
- The poll ABI is frozen and **no panic may cross a generated poll boundary** — inert for this increment: `std/json` contains no `async`, no `.await` and no poll function. Say so rather than implying the constraint was satisfied by care.
- Every fixture path unique per process.
- Commit messages to a UTF-8 file applied with `git commit -F`, **never a heredoc**. Each body ends exactly `Co-Authored-By: Claude Opus 5 <noreply@anthropic.com>`.
- **Cite no SHA that is not already an ancestor of `main`.** `a73d5be` is. The spec commit is **branch-local** — do not cite it in any tracked file.
- **Byte-scan every file you write**: no byte below 0x20 outside tab/CR/LF, no `0x7f`, valid UTF-8, and **zero** occurrences of a backslash-`u` escape followed by four hex digits in tracked markdown. Write code points as `U+XXXX`.

## TWO HAZARDS, READ BEFORE TOUCHING ANYTHING

**1. `std/json/lib.nova` is `include_str!`'d into the compiler.** Editing it forces a full workspace rebuild, and **running a stale `nova` binary exercises the OLD std and reports a FALSE PASS** — the direction that does not announce itself. Every mutation measurement must rebuild first. This is not optional care; it is the difference between a measurement and a fiction.

**2. A quoted bash heredoc consumed a backslash during this increment's design.** `String("a\"b\\c")` reached the file as `a\"b\c` and failed `L0001: invalid escape sequence`. **Do not author Nova string escapes through a heredoc.** Use the Write tool, or a Python rewrite that asserts a match count.

## Sentence-shape discipline, binding on every comment and record this increment writes

- **Prefer a roster with no count.** "All four accumulators" is a census of a growable set, and it is *already* a corrected count once — which is the shape that reproduced this hazard on the previous branch.
- **A corrected number is usually the wrong fix.** Where a count is genuinely wanted, pair it with the durable predicate and say to re-measure.
- **No ordinals or closed worlds over `std`.** "The only / the first std module with a declared input limit" is **already false**: `std/time` declares `MAX_SECS`, `MAX_MILLIS`, `MAX_MICROS`, `MAX_NANOS` and clamps against them.
- **Quote the retracted wording inside the retraction** (house style), and remember the consequence: a correct retraction still *contains* the retracted text, so grep cannot distinguish a retraction from a survival. Read the context.

## File Structure

| File | Responsibility | Task |
|---|---|---|
| `std/json/lib.nova` | the buffer helpers, `quote`, `scan_str`, the depth cap, the iterative `stringify`, the cycle guard, and every comment describing them | 1, 2, 3 |
| `tests/runtime/json_depth_leaf.nova` + `.stdout` | the cap's leaf-costs-a-level boundary | 2 |
| `tests/runtime/json_depth_empty.nova` + `.stdout` | the cap's empty-innermost boundary | 2 |
| `tests/runtime/json_render_deep.nova` + `.stdout` | a depth no recursive `stringify` could render | 3 |
| `tests/runtime/json_object_forged_map.nova` + `.stdout` | the separator placement, behaviourally | 3 |
| `tests/runtime/json_stringify_cycle.nova` | the cycle guard's panic message (no golden; expected-failure test) | 3 |
| `crates/nova-cli/tests/run_tests.rs` | one `#[test]` per new fixture — **registration is NOT automatic** | 2, 3 |
| `docs/adr/0018-*.md`, `CHANGELOG.md`, `nova-spec/20-STDLIB.md`, `docs/superpowers/plans/2026-08-22-std-json.md`, `tests/runtime/json_parse_values.nova` | the records outside the module | 4 |

**Comment amendments live in the task that changes the code they describe**, not in Task 4. Leaving a false comment for a later task is exactly how a previous increment's records commit came to transcribe an instruction that was already wrong.

## Where the counting rule comes from, and why it is stated four times

Because `scan_array` and `scan_object` each return from an **empty-container fast path without calling `parse_value`**, no depth check runs at a level whose container is empty. A **leaf costs a depth level; an empty innermost container does not.** With the check at the top of `parse_value` and `parse` starting at `0`:

| input | max depth reached | result |
|---|---|---|
| 128 containers + a leaf | 128 | **Ok** |
| 129 containers + a leaf | 129 | **Err**, `at` = 129 |
| 129 containers, empty innermost | 128 | **Ok** |
| 130 containers, empty innermost | 129 | **Err**, `at` = 129 |

A fixture asserting "129 containers is rejected" is **wrong by one** — it is true of the leaf shape and false of the empty shape. Both shapes get their own fixture, and each fixture's header says which shape the number 128 counts.

---

## Task 1: The buffer, and the two per-literal accumulators

**Files:**
- Modify: `std/json/lib.nova` — add two helpers immediately above `quote`; rewrite `quote`; rewrite `P::scan_str`; amend `scan_str`'s cost note, `quote`'s note and `vec_to_array`'s uniqueness claim
- Test: the existing `json_*` goldens, unchanged, plus a reported mutation

**Interfaces:**
- Produces: `fn buf_push_str(mut b: Vec<Char>, s: String)` and `fn vec_chars_to_string(b: Vec<Char>) -> String`, both module-private, both consumed by Task 3

- [ ] **Step 1: Confirm the goldens you must not change**

Run, and keep the output — these are the seven fixtures that must stay byte-identical through Tasks 1 and 3:

```bash
cargo build --locked --workspace && cargo test --workspace --no-fail-fast json
```

Expected: all seven `json_*_run` tests pass. Record the exact pass count. If any fails before you have changed anything, stop and report — the tree is not the baseline this plan assumes.

- [ ] **Step 2: Add the two buffer helpers**

Insert immediately above `fn quote(s: String) -> String` in `std/json/lib.nova`:

```nova
// A growable character buffer, appended to and drained once.
//
// `Vec<Char>` plus `str_from_chars` IS the growable string buffer this file's
// cost note used to say the language did not have. `Vec` grows by doubling
// (`std/collections`) so appending is amortised O(1), and `str_from_chars`
// encodes a `[Char]` in one pass, so N appends plus one drain is linear where
// `out = "${out}..."` is quadratic. Measured, this compiler, debug build, with
// a 129 ms compile baseline included: pushing n characters took 142, 135, 132
// and 142 ms at n = 8000, 16000, 32000 and 64000 -- flat across an eightfold
// range -- against 164, 249, 447 and 752 ms for interpolation.
//
// The parameter is `mut b` and not `b`. A plain `b: Vec<Char>` fails
// `E0060: Vec_T.push mutates its receiver, but b is immutable`. Records are
// heap objects passed by pointer, so the mutation reaches the caller --
// including `Vec::push`'s reassignment of its backing array when it grows,
// which is the case that would break a value-semantics assumption. Measured,
// not assumed.
fn buf_push_str(mut b: Vec<Char>, s: String) {
    let cs = s.chars()
    let mut i = 0
    while i < cs.len() {
        b.push(cs[i])
        i = i + 1
    }
}

// The buffer's contents as a `String`. The same manoeuvre `vec_to_array` runs
// for `JsonValue`: allocate an exact-length array, copy, hand it on.
//
// THE EXACT-LENGTH COPY IS LOAD-BEARING and is not redundant with the `Vec`'s
// own backing array: that array is the buffer's CAPACITY, not its length, so
// encoding it directly would append the filler character of every unused slot.
// A mutation that returns the backing array PASSES at any length that happens
// to equal capacity, and capacity is always a power of two here, so exercise
// the drain at a length that is not one.
//
// `unwrap_or(' ')` cannot take its default: `i` runs strictly below `b.len()`,
// which is exactly the range `Vec::get` answers with `Some`.
fn vec_chars_to_string(b: Vec<Char>) -> String {
    let n = b.len()
    let mut out = [' '; n]
    let mut i = 0
    while i < n {
        out[i] = b.get(i).unwrap_or(' ')
        i = i + 1
    }
    str_from_chars(out)
}
```

- [ ] **Step 3: Rewrite `quote` to use the buffer**

Replace the whole body of `quote`. Keep its signature exactly:

```nova
fn quote(s: String) -> String {
    let cs = s.chars()
    let mut out: Vec<Char> = Vec::new()
    out.push('"')
    let mut i = 0
    while i < cs.len() {
        let c = cs[i]
        let code = char_to_int(c)
        if c == '"' {
            out.push('\\')
            out.push('"')
        } else {
            if c == '\\' {
                out.push('\\')
                out.push('\\')
            } else {
                if code < 32 {
                    buf_push_str(out, escape_control(code))
                } else {
                    out.push(c)
                }
            }
        }
        i = i + 1
    }
    out.push('"')
    vec_chars_to_string(out)
}
```

Note the two-character escapes are now two `push` calls rather than one interpolation of a two-character literal. `escape_control` still returns a `String`, so it goes through `buf_push_str`.

- [ ] **Step 4: Rewrite `P::scan_str` to use the buffer**

Replace the body. The two `return ""` failure paths stay exactly as they are — on failure the return value is meaningless by contract, so they must not be changed to drain the buffer:

```nova
    fn scan_str(mut self) -> String {
        if self.err.is_some() { return "" }
        if self.peek() != '"' {
            self.fail("expected a string")
            return ""
        }
        self.bump()
        let mut out: Vec<Char> = Vec::new()
        while true {
            if self.at_end() {
                self.fail("unterminated string")
                return ""
            }
            let c = self.peek()
            if c == '"' {
                self.bump()
                return vec_chars_to_string(out)
            }
            if c == '\\' {
                self.bump()
                let piece = self.scan_escape()
                if self.err.is_some() { return "" }
                buf_push_str(out, piece)
            } else if char_to_int(c) < 32 {
                self.fail("a control character must be escaped")
                return ""
            } else {
                self.bump()
                out.push(c)
            }
        }
        vec_chars_to_string(out)
    }
```

- [ ] **Step 5: Rebuild and confirm every golden is byte-identical**

```bash
cargo build --locked --workspace && cargo test --workspace --no-fail-fast
```

Expected: **1066 passed / 0 failed / 8 ignored**, unchanged. Sum every `test result:` line. If any `json_*` golden differs, the buffer conversion changed behaviour and must be fixed, not the golden.

- [ ] **Step 6: Run the drain mutation and report which golden caught it**

Temporarily change `vec_chars_to_string`'s last line to `str_from_chars(b.data)` — wrong by capacity-versus-length. **Rebuild** (`include_str!`), then run the json tests.

Report: which golden line failed and at what content length. `stringify(String("hello"))` renders 7 characters into a buffer of capacity 8, so it should fail; if **nothing** fails, the mutation is unpinned and you must add a case at a non-power-of-two length before proceeding. Revert the mutation and rebuild.

- [ ] **Step 7: Amend the comments this task falsified**

Locate by content, never by line number. Three sites:

**`scan_str`'s cost note** — the paragraph beginning "FOUR ACCUMULATORS IN THIS FILE HAVE THAT SHAPE, NOT TWO" and running through the two measured bullets and the closing sentence. Replace the whole block with a roster and a retraction, no count:

```
    // THE ACCUMULATOR ROSTER IN THIS FILE, by name rather than by count, and
    // what changed. `quote` and this function each built one string literal by
    // interpolation; `stringify`'s `Array` and `Object` arms built the whole
    // rendered document that way. All of them now append into a `Vec<Char>`
    // drained once through `str_from_chars`, so none is quadratic in what it
    // emits. Regrep before citing this list: `grep -n 'out = "${out}'` over
    // this file should match nothing.
    //
    // RETRACTED, and quoted so the retraction is legible: this note used to
    // close "Neither is capped, and neither is fixable without a growable
    // string buffer the language does not have." That is true only of a
    // growable `String` TYPE, which Nova still lacks. It is false in the sense
    // the sentence was used: `Vec<Char>` plus `str_from_chars` composes into
    // exactly such a buffer, both already shipped, and both were already
    // called from this very file -- `Vec` at `vec_to_array`, `str_from_chars`
    // at `span`. The fix needed no language change.
    //
    // The measurements the old note carried described the interpolation form
    // and are superseded; `buf_push_str` above carries the buffer's numbers.
    // What did NOT change is `parse`'s cost, which was already effectively
    // flat: 279, 282 and 344 ms for an array of 4000, 8000 and 16000
    // one-character numbers.
```

**`quote`'s own note**, wherever it describes interpolation as its accumulator — point it at the roster above rather than restating it.

**`vec_to_array`'s header** — it says "this is the one-way conversion out". That asserts a uniqueness this task breaks, since `vec_chars_to_string` is a second drain. Reword to name what this one does without claiming to be the only one.

- [ ] **Step 8: Byte-scan and commit**

Byte-scan `std/json/lib.nova`: no byte below 0x20 outside tab/CR/LF, no `0x7f`, valid UTF-8.

Write the message to a UTF-8 file and apply it with `git commit -F`. **Never a heredoc.** Body ends exactly `Co-Authored-By: Claude Opus 5 <noreply@anthropic.com>`.

---

## Task 2: The depth cap on `parse`

**Files:**
- Modify: `std/json/lib.nova` — add `MAX_DEPTH`; thread a depth parameter through `parse_value`, `scan_array` and `scan_object`; amend `parse_value`'s no-cap paragraphs and `fail`'s count note
- Create: `tests/runtime/json_depth_leaf.nova` + `.stdout`, `tests/runtime/json_depth_empty.nova` + `.stdout`
- Modify: `crates/nova-cli/tests/run_tests.rs` — two `#[test]`s

**Interfaces:**
- Consumes: nothing from Task 1
- Produces: `const MAX_DEPTH: Int = 128`; `fn parse_value(mut self, d: Int) -> JsonValue`, `fn scan_array(mut self, d: Int) -> JsonValue`, `fn scan_object(mut self, d: Int) -> JsonValue`

- [ ] **Step 1: Write the two failing fixtures**

`tests/runtime/json_depth_leaf.nova`:

```nova
// The depth cap's boundary on the shape where a LEAF sits innermost.
//
// WHICH SHAPE 128 COUNTS HERE: containers plus a leaf. The leaf occupies a
// depth level of its own, because `parse_value` is what checks the depth and
// the leaf is a `parse_value` call. So 128 containers wrapping a leaf is the
// deepest accepted input of this shape, and 129 is the first rejected one.
//
// The sibling fixture `json_depth_empty.nova` pins the OTHER shape, where the
// innermost container is empty and one level cheaper, and it accepts 129. A
// single fixture asserting "129 is rejected" would be true here and false
// there, which is the off-by-one this pair exists to make impossible.
fn repeat(s: String, n: Int) -> String {
    let mut out = ""
    let mut i = 0
    while i < n {
        out = "${out}${s}"
        i = i + 1
    }
    out
}

fn nest(n: Int, mid: String) -> String {
    "${repeat("[", n)}${mid}${repeat("]", n)}"
}

fn report(label: String, src: String) {
    match parse(src) {
        Ok(_) => println("${label} -> Ok")
        Err(e) => println("${label} -> Err ${e.msg} at ${e.at}")
    }
}

fn main() {
    report("128 containers + leaf", nest(128, "1"))
    report("129 containers + leaf", nest(129, "1"))
}
```

`tests/runtime/json_depth_leaf.stdout`:

```
128 containers + leaf -> Ok
129 containers + leaf -> Err maximum nesting depth exceeded at 129
```

`tests/runtime/json_depth_empty.nova`:

```nova
// The depth cap's boundary on the shape where the innermost container is
// EMPTY.
//
// WHICH SHAPE 128 COUNTS HERE: it does not count containers. An empty
// container is one level cheaper than a container holding a leaf, because
// `scan_array` and `scan_object` each return from an empty-container fast path
// WITHOUT calling `parse_value`, and `parse_value` is what checks the depth. So
// no check runs at the innermost level and 129 containers is accepted, where
// the sibling fixture `json_depth_leaf.nova` rejects 129 of the leaf shape.
//
// Both rejections report the same `at`, because both fail on entry to the
// `parse_value` that would have been one level too deep, with the same number
// of opening brackets consumed.
fn repeat(s: String, n: Int) -> String {
    let mut out = ""
    let mut i = 0
    while i < n {
        out = "${out}${s}"
        i = i + 1
    }
    out
}

fn nest(n: Int, mid: String) -> String {
    "${repeat("[", n)}${mid}${repeat("]", n)}"
}

fn report(label: String, src: String) {
    match parse(src) {
        Ok(_) => println("${label} -> Ok")
        Err(e) => println("${label} -> Err ${e.msg} at ${e.at}")
    }
}

fn main() {
    report("129 containers, empty innermost", nest(129, ""))
    report("130 containers, empty innermost", nest(130, ""))
}
```

`tests/runtime/json_depth_empty.stdout`:

```
129 containers, empty innermost -> Ok
130 containers, empty innermost -> Err maximum nesting depth exceeded at 129
```

- [ ] **Step 2: Register both fixtures — registration is NOT automatic**

Add to `crates/nova-cli/tests/run_tests.rs`, beside the existing `json_*_run` tests, following their exact shape:

```rust
#[test]
fn json_depth_leaf_run() {
    let expected = std::fs::read_to_string(repo_root().join("tests/runtime/json_depth_leaf.stdout"))
        .expect("expected-output fixture exists")
        .replace("\r\n", "\n");
    nova()
        .arg("run")
        .arg(repo_root().join("tests/runtime/json_depth_leaf.nova"))
        .assert()
        .success()
        .stdout(expected);
}

#[test]
fn json_depth_empty_run() {
    let expected =
        std::fs::read_to_string(repo_root().join("tests/runtime/json_depth_empty.stdout"))
            .expect("expected-output fixture exists")
            .replace("\r\n", "\n");
    nova()
        .arg("run")
        .arg(repo_root().join("tests/runtime/json_depth_empty.nova"))
        .assert()
        .success()
        .stdout(expected);
}
```

- [ ] **Step 3: Run both to verify they fail for the right reason**

```bash
cargo build --locked --workspace && cargo test --workspace --no-fail-fast json_depth
```

Expected: **both FAIL**, and `json_depth_leaf` must fail with the 129 line reading `-> Ok` rather than the expected `Err`, because no cap exists yet. If either fails by crashing the process instead, report it — at these depths neither should overflow anything.

- [ ] **Step 4: Add `MAX_DEPTH`**

Place it with the file's other top-level declarations:

```nova
// The deepest nesting `parse` accepts. RFC 8259 section 9 permits a limit --
// "An implementation may set limits on the maximum depth of nesting" -- so a
// cap conforms.
//
// 128 IS A DECLARED CONTRACT AND DELIBERATELY NOT DERIVED FROM THIS BUILD'S
// STACK. That distinction answers the objection this file used to raise against
// any cap: a stack-size threshold is not a portable budget, and a number taken
// from one machine's stack would be wrong on the next. 128 is instead the
// budget `serde_json` starts a deserializer with -- a bare `remaining_depth:
// 128` literal on a `u8` field in its `src/de.rs`, not a named constant, and
// defeatable there through `disable_recursion_limit`. Read at `master` on
// 2026-08-25 and cited as a precedent that makes 128 defensible, not as a
// standard: Jackson's `StreamReadConstraints.maxNestingDepth()` defaults to
// 1000, CPython has no JSON-specific constant and inherits the interpreter's
// recursion limit, and Go's `encoding/json` caps nothing.
//
// For scale rather than for derivation: this build was measured parsing depth
// 5000 successfully and overflowing its stack at 6000, on an input of 12001
// characters. 128 sits far below that on the platform where it was measured,
// which is what makes it portable.
const MAX_DEPTH: Int = 128
```

- [ ] **Step 5: Thread the depth parameter**

Four edits, all located by content.

In `parse`, the single call becomes:

```nova
    let v = p.parse_value(0)
```

`parse_value` gains the parameter and the check. **The check goes above the `[`/`{` dispatch**, so an over-deep level allocates no `Vec`:

```nova
    fn parse_value(mut self, d: Int) -> JsonValue {
        if self.err.is_some() { return Null }
        if d > MAX_DEPTH {
            self.fail("maximum nesting depth exceeded")
            return Null
        }
        let c = self.peek()
        if c == 'n' { return self.keyword("null", Null) }
        if c == 't' { return self.keyword("true", Bool(true)) }
        if c == 'f' { return self.keyword("false", Bool(false)) }
        if c == '"' {
            let s = self.scan_str()
            return String(s)
        }
        if c == '[' { return self.scan_array(d) }
        if c == '{' { return self.scan_object(d) }
        if c == '-' || is_digit(c) { return self.scan_number() }
        self.fail("expected a value")
        Null
    }
```

`scan_array`'s signature becomes `fn scan_array(mut self, d: Int) -> JsonValue` and its one recursive call becomes `let v = self.parse_value(d + 1)`. Its empty-container fast path is **unchanged** — that is what makes the empty shape one level cheaper, and it is deliberate.

`scan_object`'s signature becomes `fn scan_object(mut self, d: Int) -> JsonValue` and its one recursive call becomes `let v = self.parse_value(d + 1)`. Its empty fast path is likewise unchanged.

- [ ] **Step 6: Rebuild and verify both fixtures pass and nothing else moved**

```bash
cargo build --locked --workspace && cargo test --workspace --no-fail-fast
```

Expected: **1068 passed / 0 failed / 8 ignored** — the previous 1066 plus the two new fixtures. Sum every `test result:` line; do not take this predicted number as a budget. If the real total differs, report the actual number and what accounts for the gap rather than adjusting a test to reach it.

- [ ] **Step 7: Run both depth mutations and report**

**Rebuild between every mutation** — `include_str!`.

1. **Delete the check** (both lines). Expected: `json_depth_leaf` and `json_depth_empty` both fail on their second line, which now reads `-> Ok`.
2. **`>=` for `>`** in `if d > MAX_DEPTH`. Expected: `json_depth_leaf`'s **first** line flips to `Err ... at 128`, and `json_depth_empty`'s first line flips too. This is the off-by-one at exactly 128 and it is the reason the accepted boundary is asserted and not only the rejected one.

Revert and rebuild after each. Report both outcomes with the actual golden diffs.

- [ ] **Step 8: Amend `parse_value`'s no-cap paragraphs**

Locate by content. The paragraph beginning "NESTING DEPTH IS UNBOUNDED, IN BOTH DIRECTIONS", the measured-threshold paragraph, the "No fixture pins the threshold and none can" sentence, and the whole "NONE IS IMPOSED, and that is a DECISION rather than an oversight" paragraph.

**Delete that last paragraph rather than narrowing it** — every clause in it is now false, and a narrowed version of a reversed decision reads as though the decision still partly stands. Replace the block with:

```
    // NESTING DEPTH IS CAPPED IN THIS DIRECTION. This recurses through
    // `scan_array` and `scan_object` back into itself, and the check above
    // bounds that recursion at `MAX_DEPTH`, reporting an ordinary `JsonError`
    // through the same channel as a syntax error. `stringify` is bounded
    // differently -- it does not recurse at all -- and says so at its own
    // header rather than here, so this file no longer states one property for
    // both directions.
    //
    // WHAT THE NUMBER COUNTS. A leaf costs a depth level and an empty
    // innermost container does not, because the empty-container fast paths in
    // `scan_array` and `scan_object` return without re-entering this function.
    // So 128 containers wrapping a leaf is accepted and 129 is not, while 129
    // containers ending empty is accepted and 130 is not. Both boundaries are
    // pinned: `json_depth_leaf.nova` and `json_depth_empty.nova`.
    //
    // RETRACTED, quoted so the retraction is legible. This paragraph used to
    // say a cap was declined deliberately because "a stack-size artefact is not
    // a budget a cap can be derived from", and that a cap "needs an API choice
    // this increment's scope did not include, namely which depth and whether
    // exceeding it is an ordinary `JsonError` or something a caller must
    // distinguish from bad syntax." The objection was right and is honoured
    // rather than overruled: `MAX_DEPTH` is a declared contract, not a number
    // read off this build's stack. Both deferred questions are answered -- the
    // counting rule is above, and the failure is an ordinary `JsonError`,
    // deliberately indistinguishable in shape from bad syntax, so a caller that
    // must tell them apart matches the message. That is a weak contract and is
    // named as one.
    //
    // The claim that no fixture could pin the threshold was true of a
    // STACK threshold -- a fixture that crashed the process would fail the
    // suite by construction -- and stops applying to a declared constant.
```

Also amend **`fail`'s** note carrying "dropping this guard rewrites 24 golden lines across the three parse fixtures". This increment adds parse fixtures, so both the count and the closed set of files are now wrong. Keep the durable predicate, drop the closed world:

```
    // Measured, so the reach is not assumed: dropping this guard rewrites every
    // golden line whose rejection stops the cursor short of the end of the
    // input AND whose value walk had already failed. The predicate is the
    // durable part; the count is not, and this file's parse fixtures have grown
    // since it was taken. Re-measure by deleting the guard rather than citing a
    // number from here.
```

- [ ] **Step 9: Byte-scan and commit**

Byte-scan every file written, including both `.nova` fixtures and both `.stdout` goldens. Commit with `git commit -F` from a UTF-8 file.

---

## Task 3: The iterative `stringify` and the cycle guard

**Files:**
- Modify: `std/json/lib.nova` — add `Work`, `Pending`, `MAX_RENDER_DEPTH`; rewrite `stringify`; amend its header and the `Object` arm's unreachability argument
- Create: `tests/runtime/json_render_deep.nova` + `.stdout`, `tests/runtime/json_object_forged_map.nova` + `.stdout`, `tests/runtime/json_stringify_cycle.nova`
- Modify: `crates/nova-cli/tests/run_tests.rs` — three `#[test]`s

**Interfaces:**
- Consumes: `buf_push_str(mut b: Vec<Char>, s: String)` and `vec_chars_to_string(b: Vec<Char>) -> String` from Task 1
- Produces: `type Work = | Chunk(String) | Value(Pending)`, `record Pending { v: JsonValue, d: Int }`, `const MAX_RENDER_DEPTH: Int = 100_000`. `pub fn stringify(v: JsonValue) -> String` keeps its signature exactly.

- [ ] **Step 1: Write the three failing fixtures**

`tests/runtime/json_render_deep.nova`:

```nova
// A depth no recursive `stringify` could render. Before this increment the
// `Array` arm spent one native stack frame per level, so this value ended the
// process instead of producing output -- measured, this build: depth 10000
// rendered and 16000 overflowed the stack. There was no fixture exercising
// deep nesting at all, because a fixture that crashed the process would fail
// the suite by construction.
//
// The value is built BY LOOP, not by `parse`, and it has to be: `parse` caps
// at `MAX_DEPTH`, so no parsed input can reach this depth. That asymmetry is
// deliberate and is recorded at `stringify`'s header.
fn main() {
    let mut v = Number(1.0)
    let mut i = 0
    while i < 20000 {
        v = Array([v])
        i = i + 1
    }
    let s = stringify(v)
    println("depth 20000 rendered ${s.len()} chars")
}
```

`tests/runtime/json_render_deep.stdout`:

```
depth 20000 rendered 40001 chars
```

`tests/runtime/json_object_forged_map.nova`:

```nova
// The `Object` arm's separator placement, pinned behaviourally rather than by
// reading the source.
//
// `stringify` appends the member separator BEFORE looking the value up, so a
// key that `keys()` returns but `get()` misses leaves a comma with no member
// after it. The natural work-stack rewrite moves that push inside the `Some`
// arm -- the value must be looked up before it can be pushed -- and that
// version emits `{"a":1}` here instead. Its output is the VALID JSON of the
// two, which is exactly what makes the trap look like a fix rather than a
// behaviour change. This golden is what refuses it.
//
// Reaching the `None` arm needs a FORGED map. It is unreachable through
// `Map`'s own API, but `Map` exposes every field and Nova has no field
// privacy, so a live slot placed off its own probe chain makes `keys()` return
// a key `get()` misses. Layout at capacity 4: slot 0 holds "a", which
// `find` reaches because "a" hashes to slot 0; slot 2 holds "b", which `find`
// cannot reach because it starts at "b"'s own slot 1, meets state 0 there and
// stops.
//
// THE THREE PRECONDITION LINES ARE NOT DECORATION. They depend on where "a"
// and "b" hash at capacity 4. If `str_hash` ever changes, they flip and this
// fixture FAILS -- which is the point: it must not silently stop exercising the
// separator while still passing.
fn main() {
    let cap = 4
    let mut st = [0; cap]
    st[0] = 1
    st[2] = 1
    let mut ks = [""; cap]
    ks[0] = "a"
    ks[2] = "b"
    let mut vs: [JsonValue] = [Null; cap]
    vs[0] = Number(1.0)
    vs[2] = Number(2.0)
    let forged: Map<String, JsonValue> = Map { len: 2, used: 2, keys: ks, vals: vs, state: st }

    println("keys ${forged.keys().len()}")
    match forged.get("a") {
        Some(_) => println("a reachable true")
        None => println("a reachable false")
    }
    match forged.get("b") {
        Some(_) => println("b reachable true")
        None => println("b reachable false")
    }
    println("render ${stringify(Object(forged))}")
}
```

`tests/runtime/json_object_forged_map.stdout`:

```
keys 2
a reachable true
b reachable false
render {"a":1,}
```

`tests/runtime/json_stringify_cycle.nova` — **no golden**; its test asserts a failure and a stderr message:

```nova
// A cyclic `JsonValue` and the guard that refuses it.
//
// Nova arrays are heap references, so three lines close a loop. Before this
// increment the recursive `stringify` met it with a stack overflow: fatal,
// under a second, and carrying no message. The iterative form has no such
// natural floor -- unguarded it grows the work list one net item per pop
// forever, ending in an allocator abort that prints neither a `nova: panic:`
// prefix nor a location. The guard turns that back into an immediate, named
// failure, which is better than either.
//
// The guard is on NESTING DEPTH and not on work-list length, deliberately: a
// cycle grows depth without bound while a wide, shallow document grows only
// length, and a length bound would refuse legitimate input.
fn main() {
    let mut a: [JsonValue] = [Null]
    let v = Array(a)
    a[0] = v
    println("about to render a cyclic value")
    println("${stringify(v).len()}")
}
```

- [ ] **Step 2: Register all three — registration is NOT automatic**

```rust
#[test]
fn json_render_deep_run() {
    let expected =
        std::fs::read_to_string(repo_root().join("tests/runtime/json_render_deep.stdout"))
            .expect("expected-output fixture exists")
            .replace("\r\n", "\n");
    nova()
        .arg("run")
        .arg(repo_root().join("tests/runtime/json_render_deep.nova"))
        .assert()
        .success()
        .stdout(expected);
}

#[test]
fn json_object_forged_map_run() {
    let expected =
        std::fs::read_to_string(repo_root().join("tests/runtime/json_object_forged_map.stdout"))
            .expect("expected-output fixture exists")
            .replace("\r\n", "\n");
    nova()
        .arg("run")
        .arg(repo_root().join("tests/runtime/json_object_forged_map.nova"))
        .assert()
        .success()
        .stdout(expected);
}

/// A cyclic value must fail by the guard's own named panic, not by a stack
/// overflow and not by running until the allocator gives up. Asserting the
/// message is the whole point: the failure this replaces carried none.
#[test]
fn json_stringify_cycle_panics_with_a_named_message() {
    let out = nova()
        .arg("run")
        .arg(repo_root().join("tests/runtime/json_stringify_cycle.nova"))
        .assert()
        .failure();
    let stderr = String::from_utf8_lossy(&out.get_output().stderr).to_string();
    assert!(
        stderr.contains("nova: panic: stringify: nesting too deep or cyclic value"),
        "stderr: {stderr}"
    );
    assert!(
        !stderr.contains("has overflowed its stack") && !stderr.contains("memory allocation of"),
        "expected the guard, not a stack overflow or an allocator abort, stderr: {stderr}"
    );
}
```

- [ ] **Step 3: Run all three to verify they fail for the right reasons**

```bash
cargo build --locked --workspace && cargo test --workspace --no-fail-fast json_render_deep json_object_forged_map json_stringify_cycle
```

Expected, against the still-recursive `stringify`:
- `json_render_deep_run` fails — the process overflows its stack, so there is no stdout to compare.
- `json_object_forged_map_run` **PASSES**. That is correct and important: the golden records today's behaviour, and Task 3 must preserve it. Note it as a regression guard rather than a red-to-green test.
- `json_stringify_cycle_panics_with_a_named_message` fails — it does abort, but on a stack overflow, so the second assertion catches the wrong failure mode.

- [ ] **Step 4: Add the work-list types and the guard constant**

```nova
// One pending item in `stringify`'s work list: either literal text to emit, or
// a value still to be rendered along with its nesting depth.
//
// The depth rides with the value because the guard needs one per item; a bare
// `Value(JsonValue)` cannot support it. `Pending` is a record rather than a
// second payload on the variant because Nova rejects tuples (E0900).
type Work =
    | Chunk(String)
    | Value(Pending)

record Pending { v: JsonValue, d: Int }

// The nesting depth past which `stringify` refuses to continue.
//
// This exists for CYCLES, not for depth. A cyclic `JsonValue` is constructible
// in ordinary Nova because arrays are heap references, and with the work list
// on the heap the pop-loop has no natural floor for one: it grows a net item
// per pop until the allocator aborts, printing neither a `nova: panic:` prefix
// nor a location. The recursive form this replaces died faster and just as
// fatally, with no message either. A named panic is better than both, and it is
// the best available, because `stringify` returns `String` and has no error
// channel to report a cycle through.
//
// GUARDED ON DEPTH, NOT ON WORK-LIST LENGTH. A cycle grows nesting depth
// without bound; a wide, shallow document grows only length, and a length bound
// would refuse legitimate input. 100_000 is far above any depth a caller is
// likely to build deliberately -- the work list was measured rendering depth
// 30000 -- and far above anything `parse` can hand over, since `parse` stops at
// `MAX_DEPTH`.
//
// The guard's cost is proportional to its bound: a cycle runs one iteration per
// level before firing. That is milliseconds with the buffer this file now uses,
// and was more than two minutes when the same walk accumulated by
// interpolation. Check the accumulator before raising this constant.
const MAX_RENDER_DEPTH: Int = 100_000
```

- [ ] **Step 5: Rewrite `stringify`**

Signature unchanged. **The separator push in the `Object` arm stays outside the `m.get` match** — see Step 1's fixture header for what moving it costs:

```nova
pub fn stringify(v: JsonValue) -> String {
    let mut out: Vec<Char> = Vec::new()
    let mut stack: Vec<Work> = Vec::new()
    stack.push(Value(Pending { v: v, d: 0 }))
    while stack.len() > 0 {
        match stack.pop() {
            None => out = out
            Some(w) => match w {
                Chunk(s) => buf_push_str(out, s)
                Value(p) => match p.v {
                    Null => buf_push_str(out, "null")
                    Bool(b) => buf_push_str(out, if b { "true" } else { "false" })
                    Number(n) => buf_push_str(out, number_to_json(n))
                    String(s) => buf_push_str(out, quote(s))
                    Array(xs) => {
                        if p.d + 1 > MAX_RENDER_DEPTH {
                            panic("stringify: nesting too deep or cyclic value")
                        }
                        stack.push(Chunk("]"))
                        let mut i = xs.len() - 1
                        while i >= 0 {
                            stack.push(Value(Pending { v: xs[i], d: p.d + 1 }))
                            if i > 0 { stack.push(Chunk(",")) }
                            i = i - 1
                        }
                        stack.push(Chunk("["))
                    }
                    Object(m) => {
                        if p.d + 1 > MAX_RENDER_DEPTH {
                            panic("stringify: nesting too deep or cyclic value")
                        }
                        let ks = m.keys()
                        stack.push(Chunk("}"))
                        let mut i = ks.len() - 1
                        while i >= 0 {
                            match m.get(ks[i]) {
                                Some(val) => {
                                    stack.push(Value(Pending { v: val, d: p.d + 1 }))
                                    stack.push(Chunk(":"))
                                    stack.push(Chunk(quote(ks[i])))
                                }
                                None => out = out
                            }
                            if i > 0 { stack.push(Chunk(",")) }
                            i = i - 1
                        }
                        stack.push(Chunk("{"))
                    }
                }
            }
        }
    }
    vec_chars_to_string(out)
}
```

Why reverse: the list is LIFO, so to emit `[`, e0, `,`, e1, `]` the pushes run `]`, e1, `,`, e0, `[`. The separator for member `i` is pushed after member `i`'s own items and therefore pops before them, which is what puts it between members rather than after the last one.

`match stack.pop()`'s `None` arm is unreachable — the loop condition is `stack.len() > 0` — and is written out because the match must be total. `out = out` is this file's existing idiom for exactly that.

- [ ] **Step 6: Rebuild and verify everything**

```bash
cargo build --locked --workspace && cargo test --workspace --no-fail-fast
```

Expected: **1071 passed / 0 failed / 8 ignored** — Task 2's 1068 plus three. Sum every `test result:` line and report the real number. **The seven pre-existing `json_*` goldens must be byte-identical**; if any moved, the traversal is not equivalent and the code is wrong, not the golden.

- [ ] **Step 7: Run the three `stringify` mutations and report**

**Rebuild between every one.**

1. **Reverse the reverse-push** in the `Array` arm — iterate `i` upward from 0 instead of downward. Expected: `json_stringify.nova`'s array goldens invert their element order.
2. **Move the `Object` separator inside the `Some` arm** — put `if i > 0 { stack.push(Chunk(",")) }` as the last statement of the `Some` branch. Expected: `json_object_forged_map.stdout`'s last line becomes `render {"a":1}`. This is the mutation that fixture exists for; if it passes, the fixture is not doing its job and you must say so.
3. **Delete the cycle guard** (both `if` blocks). Expected: `json_stringify_cycle_panics_with_a_named_message` fails on the *message* assertion. It must **not** be allowed to fail by timing out the suite — if the deleted-guard run does not terminate promptly, kill it, report that, and note the fixture discriminates by hang in that direction.

Revert and rebuild after each.

- [ ] **Step 8: Amend `stringify`'s header and the `Object` arm's argument**

**`stringify`'s header** — replace the "TOTAL OVER VALUES, NOT OVER DEPTH" block, its measured depth numbers and the two-accumulator-arms paragraph:

```
// Render `v` as JSON text.
//
// TOTAL OVER THE VALUES IT RETURNS FROM. For every acyclic `JsonValue` there
// is a rendering, and there is no error channel to report a failure with. Depth
// is no longer a stack cost: the arms below push pending work onto a heap list
// instead of recursing, so nesting is bounded by memory rather than by stack
// size -- a budget larger by orders of magnitude, measured rendering depth
// 30000 where the recursive form this replaces overflowed the stack between
// 10000 and 16000.
//
// HEAP-BOUNDED IS NOT BOUNDED, and the residuals are named rather than left to
// inference. A cyclic value never terminates on its own and is refused by
// `MAX_RENDER_DEPTH` with a panic. Heap exhaustion still aborts the process
// without a `JsonError`, because `gc::alloc` calls `handle_alloc_error` and no
// alloc-error hook is installed. There is no collect-and-retry on that path.
// And off Windows the collector is a no-op, so nothing is reclaimed until the
// process exits -- which is where "bounded by the heap" is weakest.
//
// `parse` is bounded differently, by a declared `MAX_DEPTH`, so a value this
// function can render may be deeper than any input `parse` will accept. That
// asymmetry is deliberate: `parse` consumes untrusted text, where a cap is a
// feature, and this function renders values the program itself built, where
// refusing to render what a caller constructed helps nobody.
//
// The accumulator cost these arms used to carry is gone; the roster of what
// changed is at `scan_str`.
```

**The `Object` arm's unreachability argument** — its conclusion is now false as written. Correct it, quoting what it said:

```
                // The `None` arm is unreachable THROUGH `Map`'s OWN API: `ks`
                // is `m.keys()`, which returns exactly the occupied slots, and
                // nothing mutates `m` between that call and this lookup.
                //
                // This used to read "The `None` arm is UNREACHABLE", flatly.
                // That is wrong: `Map` exposes every field and Nova has no
                // field privacy, so a forged map with a live slot off its own
                // probe chain makes `keys()` return a key `get()` misses.
                // Measured -- `json_object_forged_map.nova` builds one and this
                // function renders `{"a":1,}` for it. So the arm is a real path
                // and the separator's position is load-bearing, not cosmetic:
                // it is pushed OUTSIDE this match, so a missed member leaves a
                // comma with no member after it, exactly as before this
                // function stopped recursing.
                //
                // Moving the push inside the `Some` arm would emit valid JSON
                // instead, which is a behaviour change dressed as a fix. It is
                // refused by that fixture's golden rather than by this comment.
                // Repairing the fallback is a separate change with its own
                // record; the other fallbacks in this file that were written
                // for the same reason degrade safely instead -- `bump` refuses
                // to advance, `span` returns the empty string, `utf8_bytes`
                // returns 0xFF precisely to force a downstream rejection --
                // which is the pattern this one departs from.
```

- [ ] **Step 9: Byte-scan and commit**

Byte-scan every file written. Commit with `git commit -F` from a UTF-8 file.

---

## Task 4: The records outside the module, and the after-measurement

**Files:**
- Modify: `docs/adr/0018-std-json-scope-and-build-order.md`
- Modify: `CHANGELOG.md` — **two physically separate sites**
- Modify: `nova-spec/20-STDLIB.md` section 7
- Modify: `docs/superpowers/plans/2026-08-22-std-json.md`
- Modify: `tests/runtime/json_parse_values.nova` — one label

**Interfaces:**
- Consumes: the final behaviour from Tasks 1-3. Nothing produces anything.

- [ ] **Step 1: Measure the "after" numbers, so the records carry facts and not hopes**

With the workspace built from the final tree, write a scratch program outside the repo that renders an array of N one-character numbers for N = 4000, 8000 and 16000, and time each run. Record wall-clock and subtract a `nova check` compile baseline measured the same way.

The "before" numbers, measured on this build on 2026-08-25, were **231 / 1239 / 9482 ms net** of a 129 ms baseline. Report the after numbers as measured. Do **not** write a predicted number into any record.

Also record: `stringify` of a value at depth 20000 succeeds (the deep fixture asserts it), and `parse` of 129 containers wrapping a leaf returns `maximum nesting depth exceeded` at 129.

**When quoting the improvement, quote absolute numbers, not per-doubling ratios.** The interpolation form's ratios were 3.4x, 2.65x and 1.96x per doubling, which *look* sub-quadratic and are not: the work is proportional to the square of the length while memcpy throughput rises with block size, so the wall-clock ratio is damped. A record citing bare ratios invites a reader to conclude the accumulator was never quadratic.

- [ ] **Step 2: Amend ADR 0018**

Locate by content. A dated amendment, not a rewrite of history — this ADR recorded a decision that this increment reverses, and the reversal is the interesting part.

Cover: the unbounded-depth passage (now capped in the parse direction, not-recursing in the render direction); the RFC-conformance line (its subjunctive "would be conforming" becomes indicative); the passage naming the two obstacles to a cap, **quoted, with both answered**; the four-accumulator passage with its measured numbers, superseded by Step 1's; and the pointer near the end that says the depth reasoning is stated once for both directions, which is no longer true because the two directions are now bounded differently.

State plainly that the counting rule is asymmetric (a leaf costs a level) and that both boundaries are pinned by fixtures.

- [ ] **Step 3: Amend `CHANGELOG.md` at BOTH sites**

The obvious paragraph is the `std/json` known-costs block. **There is a second site roughly 75 lines above it making the same claim.** Find it by searching for the *mechanism* — unbounded depth, hard abort, quadratic accumulators, "not fixable without" — and not for any single phrasing.

This project has already shipped a fix that corrected the two outer members of a triad and left the middle one carrying the retracted claim. Before committing, grep for the mechanism again and read every hit in context, because a correct retraction still contains the text it retracts.

Add an `[Unreleased]` entry under **Added** (the cap, the guard) and **Changed** (`stringify` no longer recurses; the accumulators are linear), and a **Known limitation** for section 7's subject below.

- [ ] **Step 4: Amend `nova-spec/20-STDLIB.md` section 7**

Locate by content. Four things change and one is added.

1. Disclosure 1 (nesting depth unbounded, kills the process) — rewrite for the two now-different directions, with the counting rule.
2. Disclosure 2 (four quadratic accumulators) — rewrite as a roster with no count, carrying Step 1's measured numbers, and retract the "no growable string buffer, so neither is fixable without a language change" claim by quoting it.
3. The framing sentence describing `stringify` as total over values but not over nesting depth — the nesting-depth qualifier is what changed.
4. **Delete the sentence "A caller putting `std/json` on a socket must impose a depth limit above it."** It is now false: the module self-limits. Quote it in the retraction.
5. **Add the hash-collision disclosure**, which is new and is the increment's most important record:

> `stringify`'s `Object` arm performs one `Map` lookup per member and `parse`
> one insert per key. `Map` selects buckets from `k.hash()` and probes
> linearly, and `impl Hash for String` is `str_hash`, which the runtime
> documents as not collision-resistant and not for anything
> security-sensitive. So object keys chosen to collide make both directions
> quadratic in the number of keys, independently of `MAX_DEPTH` and of the
> accumulators. **Phase 2 position 10's throughput gate on untrusted input is
> therefore not claimable on the strength of this increment.** A seeded hash
> would fix it and needs per-process entropy; no randomness source exists
> anywhere in the runtime, so that work needs a new intrinsic first and
> belongs to `std/collections`.

Section 7 stays **open** regardless: `stringify_pretty` is still unshipped.

- [ ] **Step 5: Amend the superseded plan and the stale fixture label**

`docs/superpowers/plans/2026-08-22-std-json.md` carries two full code listings — the recursive `stringify` and `quote`'s interpolation accumulator — that a future increment could read as current. Add a dated note at the head of each listing saying it records the shipped-then implementation and pointing at this plan. Do not rewrite the listings; they are the record of what was built.

`tests/runtime/json_parse_values.nova` has a label calling five levels "deep". Next to `MAX_DEPTH` that is misleading. Rename it to describe the shape rather than the magnitude, and update the paired `.stdout` line if the label appears in it.

- [ ] **Step 6: Full verification**

```bash
cargo build --locked --workspace && cargo test --workspace --no-fail-fast
```

Sum every `test result:` line across all 44 targets and report the real total. Then:

```bash
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all -- --check
```

Confirm the accumulator roster is empty, which is the mechanical check the amended comment tells readers to run:

```bash
grep -n 'out = "${out}' std/json/lib.nova
```

Expected: no matches.

- [ ] **Step 7: Byte-scan every file this branch changed, as one population**

Not per commit. Per-commit scans structurally cannot cover the plan and the spec, which were authored before task execution began — and that is exactly where this project's only control-byte escape reached a commit.

```bash
git diff --name-only main..HEAD
```

Scan every file in that list plus this plan and the spec: no byte below 0x20 outside tab/CR/LF, no `0x7f`, valid UTF-8, and zero occurrences of a backslash-`u` escape followed by four hex digits in tracked markdown. Build the backslash with `chr(92)` rather than writing it in a pattern — Python's `re` rejects a bare backslash-`u` in a pattern.

- [ ] **Step 8: Commit**

`git commit -F` from a UTF-8 file. Body ends exactly `Co-Authored-By: Claude Opus 5 <noreply@anthropic.com>`.

---

## Mutation summary — every one must be run, rebuilt, and reported

| # | Mutation | Task | Must break |
|---|---|---|---|
| 1 | drain returns the backing array instead of the exact-length copy | 1 | a golden at a non-power-of-two length; report which |
| 2 | delete the depth check | 2 | both depth fixtures' second lines |
| 3 | `>=` for `>` in the depth check | 2 | both depth fixtures' **first** lines (the accepted boundary) |
| 4 | reverse the reverse-push in the `Array` arm | 3 | `json_stringify.stdout`'s array element order |
| 5 | move the `Object` separator inside the `Some` arm | 3 | `json_object_forged_map.stdout`'s last line |
| 6 | delete the cycle guard | 3 | the cycle test's message assertion, promptly — not by suite timeout |

**Rebuild between every mutation.** `std/json/lib.nova` is `include_str!`'d, so a stale binary tests the unmutated library and reports a false pass.

## What no test asserts, stated rather than implied

**Asymptotics are not test-asserted, and this plan deliberately adds no timing assertion.** A Rust-side bound was considered — 9482 ms before, expected well under 100 ms after, so a 5 s assertion would leave roughly a 50x margin — and rejected: this suite runs its fixtures in parallel on three platforms under variable load, it has already had an intra-invocation flake investigated at length, and a wall-clock bound on a debug build is the shape that flakes. The asymptotic claim rests on Step 1 of Task 4 — a measurement recorded with its method, re-runnable — plus `json_render_deep.nova`, which completes in milliseconds after the change and would take about nine seconds before it.

That is weaker than an assertion and is said so. It is not a fixture that discriminates only by hanging: it completes either way.

**The buffer drain had no pre-implementation probe.** `str_from_chars` is std-only, so the design's user-space equivalence probe could exercise traversal order but not the drain. Mutation 1 is the drain's only coverage; treat it as required, not optional.
