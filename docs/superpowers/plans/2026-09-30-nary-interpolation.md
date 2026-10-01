# N-ary String Interpolation Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Lower every string interpolation of three or more parts to one heap array of the parts plus one `nova_rt_str_concat_n` call, instead of *n* − 1 pairwise concatenations.

**Architecture:** `K::StrConcat` in `crates/nova-mir/src/lower.rs` gains a branch for three or more parts. It lowers each part as today, then emits one `Stmt::MakeArray` and one `CallRuntime(RtFunc::StrConcatN)`. A new runtime function sums the parts' lengths, copies each part once and builds one string. Zero, one and two parts lower exactly as before. Neither code generator changes, because both emit runtime calls generically from `RtFunc::signature`.

**Tech Stack:** Rust (the `nova-mir`, `nova-runtime` and `nova-cli` crates) and Nova test fixtures.

**Spec:** `docs/superpowers/specs/2026-09-30-nary-interpolation-design.md`

## Global Constraints

- **Branch:** `nary-interpolation`, based on `0e00190`. Every commit ends with `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`. Merge only through a PR, by rebase, with no merge commits.
- **CI's gates, all of which must pass locally before the branch is offered:**
  - `cargo test --locked --workspace --all-features --no-fail-fast`
  - `cargo clippy --locked --all-targets --all-features -- -D warnings`
  - `cargo fmt --all -- --check`
  - MSRV 1.78
- **No cargo inside subagents.** The controller runs every build and test.
- **Output must be byte-identical** for every interpolation, before and after.
- **The collector frees memory only on Windows.** A `NOVA_GC_STRESS=1` test discriminates only there.
- **Measurement rules:**
  - record each binary's byte size;
  - delete `NAME` and `NAME.exe` before every `nova build -o NAME.exe`;
  - use one fresh server process per reading;
  - state ranges, and claim a difference only when they do not overlap;
  - write the per-request prediction into the record before running.
- **Scratch counters are never committed.** The patch is `C:\Users\SAKEER~1\AppData\Local\Temp\claude\D--Projects-nona\e50b9960-25c7-4b79-9a9f-e8700f4332fd\scratchpad\gc-alloc-counters.patch`, called **THE PATCH** below.
- **Records** are amended with dated notes and never silently rewritten.

## Review Focus

1. **Evaluation order of parts with side effects.** Parts must evaluate left to right, exactly once each. This is pinned by the `side(...)` line in Task 1's fixture.
2. **An `.await` inside one part of a three-or-more-part interpolation.** The other parts' temps must survive the suspension, and the result must be correct. This is pinned by `with_await` in Task 1's fixture, and by the existing MIR test `an .await inside a string interpolation suspends too`.
3. **Multi-byte UTF-8 parts.** Total capacity is in bytes, and the result must not be truncated or split. This is pinned by Task 2's runtime test (`日本語`, `🦀`) and by Task 1's `héllo`.
4. **Empty parts, and parts produced by a user `Display` impl** that itself interpolates (a nested n-ary call). This is pinned by Task 1's `e=` and `p=` line.
5. **Collections during the call** (`NOVA_GC_STRESS=1`). The parts and the array must not be freed early. This is pinned by Task 1's GC-stress test, on Windows.

---

### Task 1: An output guard for interpolation, before any change

This lands first and passes against the unchanged compiler. It pins the output the change must not alter.

**Files:**
- Create: `tests/runtime/interpolation_nary.nova`, `tests/runtime/interpolation_nary.stdout`
- Modify: `crates/nova-cli/tests/run_tests.rs`: add two tests after `gate_async_tasks_under_gc_stress`.

**Interfaces:**
- Produces: the fixture and two tests named `interpolation_nary_run` and `interpolation_nary_under_gc_stress`.

- [ ] **Step 1: Write the fixture**

`tests/runtime/interpolation_nary.nova`:

```
// String interpolations of three to eight parts. Their output must be
// byte-identical whether interpolation lowers to pairwise concatenations or
// to one n-ary call. Covers every interpolable part type, an empty string,
// non-ASCII text, a user `Display` type (whose own `fmt` interpolates),
// parts with side effects (evaluation order), an interpolation in a loop,
// and an `.await` inside a part.

trait Display { fn fmt(self) -> String }

record P { x: Int, y: Int }

impl Display for P {
    fn fmt(self) -> String { "(${self.x}, ${self.y})" }
}

fn side(tag: String, v: Int) -> Int {
    println("eval ${tag}")
    v
}

async fn later(n: Int) -> Int {
    yield_now().await
    n * 10
}

async fn with_await(a: Int) -> String {
    "a=${a} later=${later(a).await} done"
}

fn main() {
    let i = 7
    let f = 2.5
    let b = true
    let c = 'z'
    let s = "héllo"
    let e = ""
    let p = P { x: 1, y: 2 }
    println("three ${i} parts")
    println("${i}${f}${b}")
    println("i=${i} f=${f} b=${b} c=${c}")
    println("s=${s}|e=${e}|p=${p}|end")
    println("${side("first", 1)}-${side("second", 2)}-${side("third", 3)}")
    let mut k = 0
    while k < 3 {
        println("loop ${k} of ${3}")
        k = k + 1
    }
    println(block_on(with_await(4)))
}
```

- [ ] **Step 2: Check it compiles, run it on the unchanged compiler, and verify each line by hand**

Run: `cargo build --release --locked --workspace`, then `./target/release/nova check tests/runtime/interpolation_nary.nova`, then `./target/release/nova run tests/runtime/interpolation_nary.nova`.

Expected: `check` exits 0, and `run` prints exactly these twelve lines:

```
three 7 parts
72.5true
i=7 f=2.5 b=true c=z
s=héllo|e=|p=(1, 2)|end
eval first
eval second
eval third
1-2-3
loop 0 of 3
loop 1 of 3
loop 2 of 3
a=4 later=40 done
```

If `check` rejects the fixture's syntax, adjust the fixture, keeping every listed case, and record a ruling. If the output differs from the expected lines, do not copy it into `.stdout`. Find out which side is wrong first: the prediction (for example, the `Float` formatting) or the compiler. Record the answer as a ruling.

- [ ] **Step 3: Write the `.stdout`**

Write the twelve verified lines, each ending in `\n`, to `tests/runtime/interpolation_nary.stdout`.

- [ ] **Step 4: Add the tests**

In `crates/nova-cli/tests/run_tests.rs`, after `gate_async_tasks_under_gc_stress`:

```rust
/// Interpolations of three to eight parts, across every interpolable part
/// type, an empty string, non-ASCII text, a user `Display` type, parts with
/// side effects, a loop, and an `.await` inside a part. A guard: it must pass
/// before and after interpolation is lowered to one n-ary concatenation,
/// because that change must not alter any output by a byte.
#[test]
fn interpolation_nary_run() {
    let expected = std::fs::read_to_string(repo_root().join("tests/runtime/interpolation_nary.stdout"))
        .expect("expected-output fixture exists")
        .replace("\r\n", "\n");
    nova()
        .arg("run")
        .arg(repo_root().join("tests/runtime/interpolation_nary.nova"))
        .assert()
        .success()
        .stdout(expected);
}

/// The same fixture with `NOVA_GC_STRESS=1` (collect on every allocation), so
/// a part or the parts array freed before the concatenation reads it shows up
/// as wrong output or a crash. It discriminates only where the collector frees
/// memory, which is Windows.
#[test]
fn interpolation_nary_under_gc_stress() {
    let expected = std::fs::read_to_string(repo_root().join("tests/runtime/interpolation_nary.stdout"))
        .expect("expected-output fixture exists")
        .replace("\r\n", "\n");
    nova()
        .env("NOVA_GC_STRESS", "1")
        .arg("run")
        .arg(repo_root().join("tests/runtime/interpolation_nary.nova"))
        .assert()
        .success()
        .stdout(expected);
}
```

- [ ] **Step 5: Run them**

Run: `cargo test --locked -p nova-cli --test run_tests -- interpolation_nary`
Expected: 2 passed.

- [ ] **Step 6: Commit**

```bash
git add tests/runtime/interpolation_nary.nova tests/runtime/interpolation_nary.stdout crates/nova-cli/tests/run_tests.rs
git commit -m "test(cli): pin interpolation output across part counts and types

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 2: `nova_rt_str_concat_n` and `RtFunc::StrConcatN`

**Files:**
- Modify: `crates/nova-runtime/src/lib.rs`: the new function after `nova_rt_str_concat`, a `symbols()` entry after `nova_rt_str_concat`'s, and tests in the module's test block beside `concat_and_eq_work`.
- Modify: `crates/nova-mir/src/lib.rs`: a new variant after `StrConcat` in `rt_funcs!`, plus its `symbol()` and `signature()` arms.

**Interfaces:**
- Produces: `#[no_mangle] pub unsafe extern "C" fn nova_rt_str_concat_n(parts: *const u8) -> *mut NovaStr`. `parts` is a Nova array `{ len: i64, elems… }` whose elements are `*const NovaStr`.
- Produces: `RtFunc::StrConcatN`, with `symbol()` returning `"nova_rt_str_concat_n"` and `signature()` returning `(vec![MirTy::Ptr], MirTy::Ptr)`.

- [ ] **Step 1: Write the failing runtime test**

In the runtime's test module, beside `concat_and_eq_work`:

```rust
    /// Build a Nova `[String]` array `{ len, elems… }` from `parts`, for
    /// `nova_rt_str_concat_n`. The parts are allocated first so the array is
    /// the last allocation before the call.
    unsafe fn str_array(parts: &[&str]) -> *const u8 {
        let ptrs: Vec<*mut NovaStr> = parts.iter().map(|p| make_str(p)).collect();
        let block = gc::alloc(8 + 8 * ptrs.len(), true) as *mut i64;
        *block = ptrs.len() as i64;
        for (i, p) in ptrs.iter().enumerate() {
            *block.add(1 + i) = *p as i64;
        }
        block as *const u8
    }

    #[test]
    fn concat_n_joins_every_part_in_order() {
        unsafe {
            for parts in [
                &[][..],
                &["solo"][..],
                &["a", "b", "c"][..],
                &["", "x", ""][..],
                &["日本", "語", "🦀", "-", "é"][..],
            ] {
                let got = as_str(nova_rt_str_concat_n(str_array(parts))).to_string();
                assert_eq!(got, parts.concat(), "parts {parts:?}");
            }
        }
    }
```

- [ ] **Step 2: Run it and watch it fail**

Run: `cargo test --locked -p nova-runtime --lib -- concat_n_joins_every_part_in_order`
Expected: compile error, `cannot find function nova_rt_str_concat_n`.

- [ ] **Step 3: Add the runtime function and its registration**

After `nova_rt_str_concat` in `crates/nova-runtime/src/lib.rs`:

```rust
/// Concatenate every string in `parts` into one new string: the n-ary form
/// string interpolation of three or more parts lowers to.
///
/// One copy of each part and one result, two GC objects, where pairwise
/// concatenation made a new string per part, each copying everything built
/// so far.
///
/// GC safety: every part's bytes are copied into the Rust `String` before
/// this function's first GC allocation (in `gc_str`), so a collection
/// triggered while building the result cannot free anything still being
/// read. `nova_rt_str_concat` relies on the same ordering.
///
/// # Safety
/// `parts` must point to a Nova array `{ len: i64, elems… }` whose element
/// `i`, at byte offset `8 + 8*i`, is a valid `NovaStr` pointer. A negative
/// length is treated as zero, as `nova_rt_str_from_chars` does.
#[no_mangle]
pub unsafe extern "C" fn nova_rt_str_concat_n(parts: *const u8) -> *mut NovaStr {
    let words = parts as *const i64;
    let n = (*words).max(0) as usize;
    let mut total = 0usize;
    for i in 0..n {
        let p = *words.add(1 + i) as *const NovaStr;
        total += (*p).len as usize;
    }
    let mut s = String::with_capacity(total);
    for i in 0..n {
        let p = *words.add(1 + i) as *const NovaStr;
        s.push_str(as_str(p));
    }
    gc_str(&s)
}
```

In `symbols()`, after `("nova_rt_str_concat", nova_rt_str_concat as *const u8),`:

```rust
        ("nova_rt_str_concat_n", nova_rt_str_concat_n as *const u8),
```

- [ ] **Step 4: Add the MIR variant**

In `crates/nova-mir/src/lib.rs`'s `rt_funcs!` list, after `StrConcat,`:

```rust
    /// `([str]) -> str` — concatenate every element of a string array; the
    /// lowering of an interpolation of three or more parts.
    StrConcatN,
```

In `symbol()`, after the `StrConcat` arm: `RtFunc::StrConcatN => "nova_rt_str_concat_n",`

In `signature()`, after the `StrConcat` arm: `RtFunc::StrConcatN => (vec![MirTy::Ptr], MirTy::Ptr),`

- [ ] **Step 5: Run the runtime test, the symbol test and the MIR crate**

Run: `cargo test --locked -p nova-runtime --lib -- concat_n_joins_every_part_in_order`, then `cargo test --locked -p nova-codegen-cranelift -- every_rt_func_symbol_is_registered_with_the_jit`, then `cargo test --locked -p nova-mir`
Expected: all pass. Nothing emits `StrConcatN` yet.

- [ ] **Step 6: Commit**

```bash
git add crates/nova-runtime/src/lib.rs crates/nova-mir/src/lib.rs
git commit -m "feat(runtime): add nova_rt_str_concat_n, an n-ary string concatenation

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 3: Lower three or more parts to the n-ary call

**Files:**
- Modify: `crates/nova-mir/src/lower.rs`, the `K::StrConcat` arm.
- Test: `crates/nova-mir/tests/lower_tests.rs`.

**Interfaces:**
- Consumes: `RtFunc::StrConcatN` (Task 2).

- [ ] **Step 1: Write the failing and guard MIR tests**

Add to `crates/nova-mir/tests/lower_tests.rs`:

```rust
/// Every statement of the function named `name`.
fn stmts_of<'m>(mir: &'m nova_mir::Module, name: &str) -> Vec<&'m nova_mir::Stmt> {
    let f = mir
        .functions
        .iter()
        .find(|f| f.name == name)
        .unwrap_or_else(|| panic!("no function `{name}`: {:?}", function_names(mir)));
    f.blocks.iter().flat_map(|b| &b.stmts).collect()
}

fn count_rt(stmts: &[&nova_mir::Stmt], want: nova_mir::RtFunc) -> usize {
    stmts
        .iter()
        .filter(|s| matches!(s, nova_mir::Stmt::CallRuntime { func, .. } if *func == want))
        .count()
}

/// An interpolation of three or more parts lowers to one array of the parts
/// and one n-ary concatenation, not a chain of pairwise ones.
#[test]
fn an_interpolation_of_three_or_more_parts_lowers_to_one_nary_concat() {
    let mir = mir_for(
        "fn s(a: Int, x: String, y: String) -> String { \"a=${a} x=${x} y=${y}\" }\n\
         fn main() { println(s(1, \"p\", \"q\")) }",
    );
    let stmts = stmts_of(&mir, "s");
    assert_eq!(count_rt(&stmts, nova_mir::RtFunc::StrConcatN), 1, "{stmts:?}");
    assert_eq!(count_rt(&stmts, nova_mir::RtFunc::StrConcat), 0, "{stmts:?}");
    assert_eq!(
        stmts.iter().filter(|s| matches!(s, nova_mir::Stmt::MakeArray { .. })).count(),
        1,
        "{stmts:?}"
    );
}

/// Exactly two parts keep the single pairwise call: an array there would
/// cost three objects where the pairwise call costs two. A guard, which
/// passes before and after the change; the mutation that applies the n-ary
/// path to two parts is what shows its power.
#[test]
fn an_interpolation_of_two_parts_keeps_the_pairwise_concat() {
    let mir = mir_for(
        "fn s(x: String, y: String) -> String { \"${x}${y}\" }\n\
         fn main() { println(s(\"p\", \"q\")) }",
    );
    let stmts = stmts_of(&mir, "s");
    assert_eq!(count_rt(&stmts, nova_mir::RtFunc::StrConcat), 1, "{stmts:?}");
    assert_eq!(count_rt(&stmts, nova_mir::RtFunc::StrConcatN), 0, "{stmts:?}");
}
```

If `RtFunc` does not derive `PartialEq`, the `*func == want` comparison fails to compile. It does derive it, per the `rt_funcs!` macro.

- [ ] **Step 2: Run them**

Run: `cargo test --locked -p nova-mir --test lower_tests -- interpolation_of`
Expected: the three-part test FAILS (`StrConcatN` count 0, `StrConcat` count 5), and the two-part test PASSES.

- [ ] **Step 3: Change the lowering**

In `crates/nova-mir/src/lower.rs`, in the `K::StrConcat(parts)` arm, directly after the `if parts.is_empty() { ... }` block, insert:

```rust
                // Three or more parts: one array of the parts and one n-ary
                // concatenation, which copies each part once and builds one
                // string. Pairwise folding made a new string per part, each
                // copying everything built so far. Parts are lowered in the
                // same order as before, so evaluation order, side effects and
                // any `.await` inside a part are unchanged. Two parts keep the
                // pairwise call below: an array there would cost three objects
                // against two.
                if parts.len() >= 3 {
                    let elems: Vec<(Temp, MirTy)> = parts
                        .iter()
                        .map(|part| (self.lower_expr(part), MirTy::Ptr))
                        .collect();
                    let arr = self.new_temp(MirTy::Ptr);
                    self.push(Stmt::MakeArray { dst: arr, elems });
                    let t = self.new_temp(MirTy::Ptr);
                    self.push(Stmt::CallRuntime {
                        dst: Some(t),
                        func: RtFunc::StrConcatN,
                        args: vec![arr],
                    });
                    return t;
                }
```

- [ ] **Step 4: Run the MIR tests, the fixture tests and the interpolation-bearing CLI tests**

Run: `cargo test --locked -p nova-mir`, then `cargo test --locked -p nova-cli --test run_tests -- interpolation_nary gate_async json fmt`
Expected: all pass, including the existing MIR test asserting that an `.await` inside an interpolation still suspends, and both `interpolation_nary` tests, with output unchanged.

- [ ] **Step 5: Mutations, each run, recorded and restored**

- **M1:** in `nova_rt_str_concat_n`, change both loops to `0..n.saturating_sub(1)`, dropping the last part. Run Task 2's runtime test and `interpolation_nary_run`. Expected: both FAIL.
- **M2:** change the second loop to `for i in (0..n).rev()`, reversing the order. Run the same two tests. Expected: both FAIL.
- **M3:** change `parts.len() >= 3` to `parts.len() >= 2`. Run `an_interpolation_of_two_parts_keeps_the_pairwise_concat`. Expected: FAIL.

Record each outcome. If one hangs, kill it and record that. Restore each mutation before the next.

- [ ] **Step 6: Lint, format, and run the full suite**

Run: `cargo fmt --all`, then `cargo clippy --locked --all-targets --all-features -- -D warnings`, then `cargo test --locked --workspace --all-features --no-fail-fast 2>&1 | tee <log>`
Expected: clippy clean. Summing every `test result:` line gives 0 failed, and passed equals the base count plus the 5 tests this branch adds (2 CLI, 1 runtime, 2 MIR). The base count comes from a full run on `0e00190`; if none is recorded, run one before the change. Investigate any failure. One that does not reproduce is recorded as such, never ignored.

- [ ] **Step 7: Commit**

```bash
git add crates/nova-mir/src/lower.rs crates/nova-mir/tests/lower_tests.rs
git commit -m "perf(mir): lower an interpolation of three or more parts to one n-ary concat

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 4: Measure, and record

**Files:**
- Modify: `examples/05-json-api/BENCHMARK.md`: a new dated amendment immediately before `## What was measured, and with what`, and a dated pointer in "(json-drain-no-option)"'s "What this does not settle" bullet on the remaining objects.
- Modify: `CHANGELOG.md`: a `### Changed` bullet under `[Unreleased]`.
- Modify: any file Step 6 finds.

- [ ] **Step 1: Write the per-request prediction before measuring**

Read the code that a `GET /users` request at ten users executes:
- `examples/05-json-api/src/main.nova`: `serve`, `handle`, `users_json`, `user_json` and `json_response`;
- `std/http/lib.nova`: `read_request`, `parse_request_head` and `Response::to_bytes`, plus anything they call.

For every interpolation executed per request, note its part count *n* and how many times it runs. An interpolation of *n* ≥ 3 parts saves 2(*n* − 1) − 3 = 2*n* − 5 objects per execution. Write the table and the predicted total into the draft amendment first. Do not copy the expected answer from this plan; for example, `user_json`'s 7 parts × 10 users give 90, plus whatever else the reading finds.

- [ ] **Step 2: Build the before and after binaries**

- **Plain:** `main` at `0e00190` gives `before.exe`, and the branch head gives `after.exe`. Use a `git worktree add ../nova-before 0e00190` for the before build, and remove it at the end.
- **Instrumented:** THE PATCH applied to each, built, then reverted with `git checkout -- crates/nova-runtime/src/gc.rs` in each tree, giving `before-i.exe` and `after-i.exe`.
- **Probe programs:** `interp_user`, `user_json` and `users_json_10` from `/tmp/gcm/phase/` (the generator is `gen.py`, plus the `interp_*` sources), built under each instrumented runtime.
- Delete sibling names before every build. Record every byte size. Confirm with `cmp` that before and after differ.

- [ ] **Step 3: Measure**

- **Probes:** `NOVA_GC_DEBUG=1 NOVA_GC_THRESHOLD=65536 ./<probe>.exe`, then objects and bytes per call as before. Expected: `interp_user` falls from 18 to 9.
- **Server allocation:** `run_alloc_b.sh` with `BIN` set to each instrumented server, ten users, three readings each, alternated.
- **Server throughput:** `run_x.sh` with `BIN` set to each plain server, ten users, three readings each, alternated.

Every reading must have `errors=0` and a 604-byte body.

- [ ] **Step 4: Write the amendment**

It holds:
- the prediction table from Step 1, and the measured result against it;
- the probe table;
- per-request objects and bytes, before and after;
- throughput, before and after, with ranges, per-request microseconds and the gate shortfall;
- binary sizes and the procedure;
- the mutations from Task 3 Step 5 and the full-suite count;
- a "What this does not settle" list, including that literal headers and two-part concatenations are unchanged.

Add a dated pointer from "(json-drain-no-option)"'s remaining-objects bullet.

- [ ] **Step 5: CHANGELOG**

Under `[Unreleased]` → `### Changed`, add a bullet with the change, the probe and per-request figures, the throughput ranges, the statement that the gate is still not met, and a pointer to the amendment.

- [ ] **Step 6: Set-difference sweep**

```bash
comm -23 \
  <(git grep -l -i -E "StrConcat|nova_rt_str_concat|pairwise concat|interpolation is (lowered|desugared)" | sort) \
  <(git diff --name-only 0e00190..HEAD | sort)
```

Open every hit, reading around it because grep is line-based. Amend any that now describes the lowering or its cost wrongly, with a dated note, or record it as deliberately left. The HIR module doc's "desugared to `StrConcat`" stays true.

- [ ] **Step 7: Independent check, then commit**

An agent with no cargo checks every figure in the amendment and the CHANGELOG against the logs. Fix what it finds, then commit:

```bash
git add examples/05-json-api/BENCHMARK.md CHANGELOG.md
git commit -m "docs(json-api): measure n-ary interpolation before and after

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

Add any files Step 6 amended to that commit.

---

### Task 5: Whole-branch review, then PR

- [ ] **Step 1:** Run CI's three gates locally again on the final tree: fmt, clippy, and the full suite. Report the summed counts.
- [ ] **Step 2:** A fresh reviewer on the most capable model, with no cargo, reads the spec, this plan and `git diff 0e00190..HEAD`, and checks the Review Focus items deliberately. Fix Critical and Important findings, each verified by a check that fails before the fix and passes after.
- [ ] **Step 3:** Push the branch, open a PR, and merge by rebase once CI is green, as the user has approved for this line of work. Then verify by tree identity.
