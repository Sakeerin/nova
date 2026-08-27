# HashDoS Resistance Test Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Pin HashDoS precomputation resistance with a test that executes it — a key set built to collide under one process's seed must spread near-ideally in a fresh process.

**Architecture:** One Rust test in `crates/nova-cli/tests/run_tests.rs` orchestrating two Nova programs in two processes. Phase 1 searches for 32 keys sharing a bucket and prints their indices; the test inlines those indices into a second Nova program and runs it in a fresh process, asserting the largest bucket holds at most 10. No golden `.stdout`, because both phases print seed-dependent output.

**Tech Stack:** Rust (`assert_cmd` via the existing `nova()` helper) plus two generated Nova programs. No new dependency, no new intrinsic, no trait change, no `std/*.nova` edit, no new ADR.

**Spec:** `docs/superpowers/specs/2026-08-27-hashdos-resistance-test-design.md`

## Amendment - 2026-08-27, written after execution

This plan is kept as written and amended here rather than corrected in place: a plan rewritten to match what shipped would erase the evidence that it was wrong, which is what a superseded plan is still good for. One item below is not a statement but an **instruction**, so flagging it is not enough — an instruction stays executable until something tells an executor not to execute it.

**STEP 3'S INSTRUCTION MUST NOT BE FOLLOWED AS WRITTEN.** Step 3, headed "Correct the merged plan's amendment mechanism", directs the executor to "Add a further dated note (2026-08-27) recording that recovering the seed does not let a Nova fixture predict a hash, because computing FNV-1a over a key needs that key's bytes and `std/core` records that `String` has no length, indexing or iteration". **Do not write that.** The justification is false, so the note that step asks for would retract something true. `std/bytes` exposes `bytes_from_string`, `byte_at` and `to_ints`, reachable from a user module with no import at all — the registered fixture `tests/runtime/bytes_basics.nova` calls `bytes_from_string("hi")` as its first statement and is checked against a golden — and `std/strings` gives `String` a `len`, `chars`, `char_at` and `slice`. Nova can walk a `String`'s bytes, so the merged plan's amendment mechanism was viable.

**What to write instead**, and what the executing increment did write: a note recording that the merged plan's amendment **stands**, and that the *correction* of it — in this plan's own spec, section 2 — is the thing that fails. That note is in `docs/superpowers/plans/2026-08-26-map-hashdos.md` under its 2026-08-27 heading, and the spec carries a matching dated correction as its own closing section. Step 3's other directions are unaffected and were followed: quote the retracted wording inside the retraction, keep each sentence unambiguous about which half is which, and anchor by content rather than by line number.

**The same false justification appears elsewhere in this plan as statements rather than instructions, and the paragraph above retracts each of them rather than repeating it:** the note to executors headed "Why two processes and not one", which states it flatly, with "at all"; the Phase 1 doc comment this plan dictates verbatim; and the Phase 2 body comment beside the second build. The shipped tree does not carry that wording — Task 1's prose was corrected on this branch, at the commit whose subject is "correct two false justifications in the hashdos test's prose" — so this plan and the tree it produced disagree about a comment this plan dictated. Read the tree.

**The two-process design still stands, on corrected grounds.** It is not forced by impossibility. Both phases exercise the runtime's real hash, whereas computing the hash in Nova would pin a second copy of the algorithm and fail for the wrong reason if the two ever diverged. `hashdos_precomputed_key_set_does_not_survive_a_new_process` in `crates/nova-cli/tests/run_tests.rs` states that framing at itself.

Passages here are anchored by content and not by line number, for the reason the merged plan's own note gives: inserting an amendment at the head of a file moves every line number the amendment cites, so a line citation falsifies itself in the act of being written.

## Global Constraints

- `cargo build --locked --workspace` BEFORE `cargo test`. `include_str!` embeds every `std/*/lib.nova` in the compiler, so a stale binary measures old text and reports a FALSE PASS.
- `cargo test --workspace --no-fail-fast`. Sum every one of the 44 `test result:` lines; never pipe cargo output through `head` or `tail` before summing. Baseline 1075 passed / 0 failed / 8 ignored; expect 1076 / 0 / 8 with the increase landing entirely in the `nova-cli` `run_tests` target.
- `cargo clippy --locked --workspace --all-targets -- -D warnings` and `cargo fmt --all -- --check`.
- No `reason = "..."` in any lint attribute (MSRV 1.78).
- The ignored ADR-0010 GC tests stay ignored and untouched.
- The poll ABI is FROZEN and no panic may cross a generated poll boundary. **Inert here** — this touches no async path. Say so rather than implying care satisfied it.
- Every fixture path unique per process: the generated sources and their build directory carry `std::process::id()`.
- `git commit -F` from a UTF-8 file, NEVER a heredoc. Each body ends exactly `Co-Authored-By: Claude Opus 5 <noreply@anthropic.com>`.
- Cite no SHA that is not already an ancestor of `main`. `515145b` is. Every SHA created on this branch is BRANCH-LOCAL and must not appear in any tracked file.
- Byte-scan every file written: valid UTF-8, no byte below 0x20 outside tab/CR/LF, no 0x7f, and ZERO occurrences of a backslash followed by `u` and four hex digits in tracked markdown (write code points as U+XXXX).
- Do not push, merge or tag.

## Sentence-shape discipline, binding on every comment and record

- Prefer a roster with NO count. A corrected number is usually the wrong fix; deleting the number is right.
- No ordinals and no closed worlds over `std`, the runtime, the workspace or the record set. If you must quantify, name the population inside the sentence.
- Never claim a test is "the only" thing catching something. Measured false four times on this project, and the previous increment shipped exactly that shape and had to retract it.
- When you retract wording, quote it inside the retraction — and remember a correct retraction therefore still CONTAINS the false phrase, so make the sentence unambiguous about which half is which.
- grep is line-oriented, so a MISS IS NOT EVIDENCE OF ABSENCE. Sweep prose whitespace-flattened with `//`, `*` and `>` gutters stripped, and run the line-oriented and flattened sweeps separately.

---

## File Structure

| File | Responsibility | Task |
|---|---|---|
| `crates/nova-cli/tests/run_tests.rs` | the whole test: two Nova source templates, two builds, two runs, the assertions | 1 |
| `nova-spec/20-STDLIB.md` | section 7's gate claim gains a pointer to the executing test; scope unchanged | 2 |
| `docs/adr/0018-std-json-scope-and-build-order.md` | same pointer, same unchanged scope | 2 |
| `docs/superpowers/plans/2026-08-26-map-hashdos.md` | a further dated note correcting its amendment's mechanism | 2 |
| `CHANGELOG.md` | one entry under `[Unreleased]` | 2 |

Nothing else is touched. In particular no file under `std/` is expected to change; if you find yourself editing one, stop and say why.

---

## Task 1: The cross-process resistance test

**Files:**
- Modify: `crates/nova-cli/tests/run_tests.rs` — append after `str_hash_seed_varies_across_processes`, which ends at line 8667 (currently the last line of the file)

**Interfaces:**
- Consumes: the existing `nova()` helper (returns a `Command` for the built `nova` binary) and `Command` from `assert_cmd`, both already imported and used by `str_hash_seed_varies_across_processes` at `:8635`. Read that test first — this one mirrors its build-then-run shape.
- Produces: nothing other tasks consume. Task 2 references this test **by name** (`hashdos_precomputed_key_set_does_not_survive_a_new_process`), so keep that name.

### THE TWO TRAPS IN THIS TASK

**Trap 1 — `format!` cannot build the phase-2 source.** The Nova code contains `${i}` interpolation, and Rust's `format!` treats `{i}` as a named argument capture. Verified against `rustc --edition 2021`:

```
error[E0425]: cannot find value `i` in this scope
  |
2 |     let s = format!(r#"let k = "k${i}""#);
  |                                    ^ not found in this scope
```

So build the phase-2 source with a **raw string template containing a placeholder token** and `str::replace`, never `format!`:

```rust
let src = PHASE2_TEMPLATE.replace("__INDICES__", &index_list);
```

**Trap 2 — do not author Nova through a shell heredoc.** A quoted heredoc has silently consumed a backslash twice on this project. Everything here is written from Rust, which avoids the shell entirely. If you make a scratch fixture by hand while debugging, write it with a file-writing tool.

- [ ] **Step 1: Read the test this one mirrors**

Read `crates/nova-cli/tests/run_tests.rs:8632-8667`. Note three things you will reuse: the per-process temp directory built with `std::process::id()`, the `nova().arg("build").arg(&file).arg("-o").arg(&exe)` shape, and the closure that runs the built exe and trims its stdout.

- [ ] **Step 2: Add the two Nova source templates**

Both compile and run against the real compiler as written — verified on `main` at `515145b`, not written hopefully. Do not "improve" them; the constructs were chosen because they are the ones this language actually has.

```rust
/// Phase 1: search candidate keys for a set that collides under THIS
/// process's seed. Nova cannot compute a hash itself — `std/core` records
/// that `String` has no length, indexing or iteration — so this searches
/// with `.hash()` rather than deriving anything from the seed.
///
/// Two passes rather than a bucket-of-lists, because Nova's array
/// construction has a repeat form and a list form and no nested growable
/// structure worth building here. The first pass stops at the 32nd key in
/// some bucket; the second collects that bucket's first 32 indices, which
/// are exactly the keys the first pass counted.
const PHASE1_SRC: &str = r#"const CAP: Int = 64
const WANT: Int = 32
const BUDGET: Int = 4000

fn main() {
    let mut counts = [0; CAP]
    let mut target = -1
    let mut i = 0
    while i < BUDGET {
        let k = "k${i}"
        let b = k.hash() & (CAP - 1)
        counts[b] = counts[b] + 1
        if counts[b] >= WANT { target = b; break }
        i = i + 1
    }
    if target < 0 {
        println("found false")
        return
    }
    let mut out = ""
    let mut n = 0
    let mut j = 0
    while j < BUDGET {
        let k = "k${j}"
        if (k.hash() & (CAP - 1)) == target {
            out = "${out}${j} "
            n = n + 1
            if n == WANT { break }
        }
        j = j + 1
    }
    println("found true")
    println("count ${n}")
    println("indices ${out}")
}
"#;

/// Phase 2: re-hash phase 1's set in a FRESH process, therefore under a
/// fresh seed. `__INDICES__` is replaced by the comma-separated indices
/// phase 1 printed. Keys are `"k" + index` by construction, so an index
/// transfers the key exactly.
const PHASE2_TEMPLATE: &str = r#"const CAP: Int = 64

fn main() {
    let idx = [__INDICES__]
    let mut counts = [0; CAP]
    let mut i = 0
    while i < idx.len() {
        let k = "k${idx[i]}"
        let b = k.hash() & (CAP - 1)
        counts[b] = counts[b] + 1
        i = i + 1
    }
    let mut largest = 0
    let mut j = 0
    while j < CAP {
        if counts[j] > largest { largest = counts[j] }
        j = j + 1
    }
    println("keys ${idx.len()} largest ${largest}")
}
"#;
```

- [ ] **Step 3: Write the test**

```rust
/// A key set built to collide under one process's seed must NOT collide in
/// a fresh process. This is precomputation resistance, executing rather
/// than argued: before this test the property rested on measurements taken
/// in a throwaway harness and recorded in prose.
///
/// Phase 1 is itself an adaptive attack succeeding — it concentrates 32 of
/// 4000 candidate keys into one bucket — so the limit the records disclose
/// is demonstrated here too.
///
/// The thresholds are derived, not chosen to look generous. Phase 1's
/// budget of 4000 is about 2.6 times the observed worst case: over 200
/// random seeds a bucket reached 32 keys after a median of 1319 candidates,
/// 1463 at the 95th percentile and 1508 at the worst, succeeding for every
/// one of those seeds. Phase 2's bound of 10 comes from the binomial right
/// tail for 32 keys in 64 buckets unioned over the buckets: a false failure
/// runs about 8.3e-11, roughly one run in twelve billion, against an
/// observed maximum of 5 and a 99th percentile of 4 over 3000 simulated
/// seed pairs. If the seeding were dead the largest bucket would be 32, so
/// the margin before a broken seed could pass is 22 keys — measured, by
/// running both phases in one process.
///
/// What this does NOT cover: the splitmix64 finalizer. Dropping it while
/// keeping the seed leaves this test passing, because a seed change alone
/// moves these keys. `hash_diffusion` is what fails for that.
///
/// This builds and runs Nova binaries, so it joins the population the
/// `0xc0000005` flake draws from. That cost is stated rather than hidden.
#[test]
fn hashdos_precomputed_key_set_does_not_survive_a_new_process() {
    let dir = std::env::temp_dir().join(format!("nova-hashdos-resist-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("temp dir");

    let build_and_capture = |name: &str, src: &str| -> String {
        let file = dir.join(format!("{name}.nova"));
        std::fs::write(&file, src).expect("write nova source");
        let exe = dir.join(format!("{name}{}", std::env::consts::EXE_SUFFIX));
        nova()
            .arg("build")
            .arg(&file)
            .arg("-o")
            .arg(&exe)
            .assert()
            .success();
        let out = Command::new(&exe).assert().success();
        String::from_utf8_lossy(&out.get_output().stdout)
            .replace("\r\n", "\n")
            .trim()
            .to_string()
    };

    // Phase 1, in its own process.
    let found = build_and_capture("phase1", PHASE1_SRC);
    assert!(
        found.contains("found true"),
        "phase 1 must find 32 keys in one bucket within its budget; it printed:\n{found}"
    );
    assert!(
        found.contains("count 32"),
        "phase 1 must report exactly the set size it was asked for; it printed:\n{found}"
    );
    let indices: Vec<&str> = found
        .lines()
        .find_map(|l| l.strip_prefix("indices "))
        .expect("phase 1 prints an `indices` line")
        .split_whitespace()
        .collect();
    assert_eq!(
        indices.len(),
        32,
        "phase 1's index line must carry the whole set; it printed:\n{found}"
    );

    // Phase 2, in a SEPARATE process, therefore under a different seed.
    // Nova cannot evaluate the hash for a seed other than its own, so a
    // second real process is what supplies the second seed.
    let src = PHASE2_TEMPLATE.replace("__INDICES__", &indices.join(", "));
    let spread = build_and_capture("phase2", &src);
    assert!(
        spread.starts_with("keys 32 largest "),
        "phase 2 must report the set size and the largest bucket; it printed:\n{spread}"
    );
    let largest: usize = spread
        .rsplit(' ')
        .next()
        .and_then(|n| n.parse().ok())
        .expect("phase 2's last field is the largest bucket count");
    assert!(
        largest <= 10,
        "a set precomputed under another seed must not still collide: largest bucket {largest} \
         of 32 keys, bound 10. A dead seed yields 32. Phase 1 printed:\n{found}\n\
         Phase 2 printed:\n{spread}"
    );
}
```

- [ ] **Step 4: Build, then run the new test alone**

```bash
cargo build --locked --workspace
```

```bash
cargo test -p nova-cli --test run_tests hashdos_precomputed_key_set_does_not_survive_a_new_process -- --nocapture
```

Expected: PASS. Record the `largest` value you observed. Values of 2 through 5 are ordinary; anything at or above 11 on an unmutated tree is a finding worth reporting, not retrying away.

- [ ] **Step 5: Run mutation 1 — pin the seed to a constant basis**

In `crates/nova-runtime/src/lib.rs`, change `nova_rt_str_hash`'s first line from `let mut h: u64 = str_hash_seed();` to the old constant `let mut h: u64 = 0xcbf2_9ce4_8422_2325;`. Rebuild, then run the test.

Expected: **FAIL**, with `largest bucket 32 of 32 keys`. This is the mutation the increment exists to catch. Confirmed independently by running both phases in one process, which printed `same-seed largest 32 of 32` on three consecutive runs.

Revert the mutation and rebuild before continuing.

- [ ] **Step 6: Run mutation 2 — break phase 1's grouping**

In `PHASE1_SRC`, change `if counts[b] >= WANT` to `if counts[b] >= 1`. Rebuild, run the test.

Expected: **FAIL** at the `count 32` assertion — phase 1 reports a bucket that does not hold 32 keys, and the test stops before phase 2 rather than proceeding to a meaningless comparison.

Revert.

- [ ] **Step 7: Run mutation 3 — starve phase 1's budget**

In `PHASE1_SRC`, change `const BUDGET: Int = 4000` to `const BUDGET: Int = 100`. Rebuild, run the test.

Expected: **FAIL** at the `found true` assertion, with phase 1 having printed `found false`. This proves the not-found path is real rather than decorative.

Revert.

- [ ] **Step 8: Run mutation 4 — drop the finalizer, keep the seed. EXPECTED TO PASS.**

In `crates/nova-runtime/src/lib.rs`, change `nova_rt_str_hash`'s return from `splitmix64_finalize(h) as i64` to `h as i64`. Rebuild, run the test.

Expected: **PASS**. This is not a defect and must not be "fixed". A seed change alone moves these keys, so this test cannot see the finalizer; `tests/runtime/hash_diffusion.nova` is what fails for that. Report the expected pass explicitly — the point of running it is to record honestly that this test does not cover the finalizer, so nobody later reads it as covering both.

Note that `hash_diffusion` WILL fail under this mutation. Record that too: it shows this test and `hash_diffusion` answer different questions.

Revert, rebuild, and confirm the tree is clean with `git status --porcelain`.

- [ ] **Step 9: Full suite, clippy, fmt**

```bash
cargo build --locked --workspace
```

```bash
cargo test --workspace --no-fail-fast
```

Sum every one of the 44 `test result:` lines. Expected **1076 passed / 0 failed / 8 ignored** — one more than the 1075 baseline, the increase entirely in the `nova-cli` `run_tests` target.

```bash
cargo clippy --locked --workspace --all-targets -- -D warnings
```

```bash
cargo fmt --all -- --check
```

If either flake fires — the `0xc0000005` family, or `net::tests::connecting_to_a_closed_port_is_connection_refused` — re-run, say it happened, attribute no cause, and fix nothing. Do not cite the candidate mechanism in `run_tests.rs:1476-1490` as an explanation; every shared-fixture-path hypothesis for that family has been eliminated.

- [ ] **Step 10: Commit**

Write the message to a UTF-8 file and apply it with `git commit -F`, never a heredoc. Body ends exactly `Co-Authored-By: Claude Opus 5 <noreply@anthropic.com>`.

```bash
git add crates/nova-cli/tests/run_tests.rs
```

---

## Task 2: The records

**Files:**
- Modify: `nova-spec/20-STDLIB.md` — section 7's gate claim
- Modify: `docs/adr/0018-std-json-scope-and-build-order.md` — the matching gate claim
- Modify: `docs/superpowers/plans/2026-08-26-map-hashdos.md` — a further dated note
- Modify: `CHANGELOG.md` — one entry under `[Unreleased]`

**Interfaces:**
- Consumes: the test name `hashdos_precomputed_key_set_does_not_survive_a_new_process` from Task 1.
- Produces: nothing.

### The boundary that governs every edit here

**Do NOT widen the gate claim.** Its scope is unchanged: claimable for string-keyed maps against a precomputing attacker, not against an adaptive one, and not for `Int`, `Bool` or `Char` keys because `mix64` is unseeded. What changes is the claim's **evidence** — from external measurement to an executing assertion. A reader must not come away thinking the guarantee grew.

**Amend, never rewrite.** The merged plan is a dated record. It gets a further note, not an edit in place.

- [ ] **Step 1: Check whether ADR 0005 needs anything**

```bash
grep -n "not randomized per process\|test-assertable\|precomputing" docs/adr/0005-mutable-receivers-and-one-shot-hash.md
```

Read what you find. Add an amendment **only if** ADR 0005 asserts something this increment changes. Do not add one that merely announces a test exists — that is noise in a decision record. Report which way you ruled and why.

- [ ] **Step 2: Point the two gate claims at the executing test**

In `nova-spec/20-STDLIB.md` section 7 and `docs/adr/0018-std-json-scope-and-build-order.md`, both of which already carry a dated narrowing from the seeding increment, add that the precomputing half is now pinned by `hashdos_precomputed_key_set_does_not_survive_a_new_process` in `crates/nova-cli/tests/run_tests.rs`, and that the adaptive limit is demonstrated by that test's own first phase.

Keep it short and point rather than restate: the derivation lives in the test's doc comment and in the spec. State plainly that the scope is unchanged and only the evidence moved.

- [ ] **Step 3: Correct the merged plan's amendment mechanism**

`docs/superpowers/plans/2026-08-26-map-hashdos.md` carries an amendment saying a fixture can "recover the seed, construct a colliding key set, and assert the degradation". Its conclusion is right; its mechanism is wrong.

Add a further dated note (2026-08-27) recording that recovering the seed does not let a Nova fixture predict a hash, because computing FNV-1a over a key needs that key's bytes and `std/core` records that `String` has no length, indexing or iteration — the end-to-end prediction that proved the seed recoverable ran outside Nova. The route that works needs no seed: generate candidates, call `.hash()`, mask, and group, so the fixture **searches** where the amendment expected it to **derive**.

Quote the retracted phrase inside the note, and make the sentence unambiguous about which half is being corrected — a correct retraction still contains the wording it retracts.

That file's existing amendment anchors passages by content rather than line number, for the reason its own note gives. Follow that convention; do not reintroduce line citations.

- [ ] **Step 4: CHANGELOG entry under `[Unreleased]`**

One entry. Say what is now executed that was previously argued, name the test, state that the gate claim's scope is unchanged and only its evidence moved, and record that this test does not cover the finalizer. Do not restate the derivations; point at the test's doc comment.

Do not touch any entry for a shipped release.

- [ ] **Step 5: Sweep your own diff, then verify**

Run BOTH sweeps over the lines this task added, separately, because one sees what the other cannot:

```bash
git diff | grep '^+' | grep -v '^+++' | sed 's/^+//' > /tmp/added.txt
```

Then flatten whitespace and strip `//`, `*` and `>` gutters before matching, and look for bare counts over growable sets, closed worlds, and "the only" claims. Judge each hit — a plan or spec counting its own closed contents is legitimate; a count over the record set is not.

- [ ] **Step 6: Build, test, lint**

Markdown-only edits cannot change behaviour, but build before testing anyway: the reason that rule exists is that the failure mode is a false pass, which is invisible.

```bash
cargo build --locked --workspace
```

```bash
cargo test --workspace --no-fail-fast
```

Expected 1076 / 0 / 8, unchanged from Task 1. A moved count means something unrelated happened — report it.

- [ ] **Step 7: Commit**

`git commit -F` from a UTF-8 file. Byte-scan every file written first.

---

## Notes for whoever executes this

**Why two processes and not one.** Nova cannot evaluate the hash for a seed other than its own, because it cannot walk a `String`'s bytes at all. There is no way to ask "what would this key hash to under a different seed" from inside the language, so a second real process is what supplies the second seed. This is also why seed recovery — which the predecessor's amendment reached for — is irrelevant here.

**Why no golden file.** Both phases print seed-dependent output; which keys collide differs every run. A golden could pin only a boolean and would break on the varying lines. This also keeps the increment clear of the standing rule that no fixture may pin `Map` or `Set` iteration order.

**What the mutations are for.** Three must fail and one must pass. The one that passes is the honest part: it records a gap rather than hiding it.
