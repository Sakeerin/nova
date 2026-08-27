# `Map` HashDoS Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make `nova_rt_str_hash` keyed and seeded, so a `Map<String, _>` resists adversarially chosen keys.

**Architecture:** One Rust function changes. FNV-1a's offset basis is replaced by a per-process seed, and the splitmix64 finalizer is applied to the result. The seed is a `OnceLock<u64>` filled from `std`'s `RandomState`. **No new intrinsic, no trait change, no new dependency, and no `.nova` library edit** — so the 12-site checklist is not paid and `STD_ONLY`, `RtFunc` and `STD_MODULES` are untouched.

**Tech Stack:** Rust only (`crates/nova-runtime/src/lib.rs`), plus Nova fixtures under `tests/runtime/` and their registration in `crates/nova-cli/tests/run_tests.rs`.

**Spec:** `docs/superpowers/specs/2026-08-26-map-hashdos-design.md`

## Amendment - 2026-08-26, written after execution

This plan is kept as written; what it got wrong is part of the record, and a
plan rewritten to match what shipped would erase the only thing a superseded
plan is still good for. Statements in it that the finished increment falsified,
with the correction each time:

- **The runtime doc comment this plan specifies verbatim in Task 1** - the
  line reading "cannot read this value: no builtin exposes it". FALSE. `str_hash` is the builtin
  that exposes it: `("").hash()` returns splitmix64's finalizer applied to the
  raw seed, because FNV's loop body never runs on an empty string, and that
  finalizer is invertible. Measured on the finished branch - a seed recovered
  from one such call then predicted two further hashes from the same process
  exactly. The `collect_externs` half of the sentence is true; the closed world
  over routes is not. Corrected wording lives at `nova_rt_str_hash` in
  `crates/nova-runtime/src/lib.rs`.
- **A consequence nobody drew at planning time, and the more useful half.**
  The closing note says the resistance property "is not test-assertable:
  building a colliding set requires the seed the design is trying to keep
  unknown." That
  premise is gone. Since the seed is recoverable in one call, a fixture CAN
  recover it, construct a colliding key set, and assert the degradation
  directly. Whoever strengthens this next should start there rather than
  re-deriving the analysis.
- **The Architecture note above, "no `.nova` library edit"** - FALSE. The increment edits comments
  in `std/core`, `std/collections` and `std/json`. No behaviour changed, which
  is what the note meant and not what it said.
- **The closing note on the resistance property, "machine-specific in their
  absolutes though not in their shape"** - FALSE. The figures are deterministic integer arithmetic over given
  bytes; they vary with the seed, not with the machine.
- **The `largest under 60` bound in Task 2's fixture body and golden, and
  mutation 2's expectation written against it.** The bound shipped and was then
  changed to `< 64`. At 60 the assertion fails about one run in 6,100 - an
  undisclosed flake, while its sibling assertion's risk was derived. Measured
  over 300 seeds: the finalized largest bucket ranges 40 to 56 and the
  unfinalized ranges 66 to 76, so a usable bound sits inside that gap, and 60
  sat only four above the finalized maximum.
- **This plan's mutation-1 expectation.** It predicted the diffusion fixture
  would fail with the basis pinned to the old constant and the finalizer kept.
  It passes: the finalizer alone satisfies that assertion, which is why the
  cross-process test rather than the diffusion fixture is what catches a dead
  seed.

Task 3's instructions to narrow the Phase 2.2a disclosure and to correct the
resolver doc comment are NOT in this list. They quote stale wording in order to
instruct its replacement, which is what they were for and remains correct.

The spec carries its own dated amendment. Neither document was edited in place.

## Further note - 2026-08-27, from the `hashdos-resistance-test` increment

**This note corrects the CORRECTION of one item in the amendment above. The
item itself is not being corrected — it is being restored.** The item concerned
is the one beginning **"A consequence nobody drew at planning time"**, whose
claim is that a fixture "CAN recover it, construct a colliding key set, and
assert the degradation directly".

**Verdict stated before either quotation, so that no sentence below can be read
the wrong way round: that amendment item is CORRECT, mechanism included, and
the sentence that called its mechanism wrong is the false one.** The false
sentence is not in this plan. It is in the design record for the follow-up
increment, `docs/superpowers/specs/2026-08-27-hashdos-resistance-test-design.md`
section 2, and it reads: "**The amendment's conclusion is right and its
mechanism is wrong.** Recovering the seed does not let a Nova fixture predict a
hash, because computing FNV-1a over a key needs that key's bytes and `std/core`
records that '`String` has no length, indexing or iteration, so Nova cannot
walk its bytes'". **That verdict and the reason given for it are what is
retracted here.** That spec now carries its
own dated correction saying the same thing about itself.

Why the reason does not hold: `std/bytes` exposes `bytes_from_string`,
`byte_at` and `to_ints`, and they are reachable from a user module with no
import at all — the registered fixture `tests/runtime/bytes_basics.nova` calls
`bytes_from_string("hi")` as its first statement and is checked against a
golden. `std/strings` additionally gives `String` a `len`, `chars`, `char_at`
and `slice`. So a Nova fixture can obtain a key's bytes. The arithmetic such a
prediction needs also already runs in Nova: `std/core`'s `mix64` computes
splitmix64's finalizer in ordinary Nova `Int` shift, xor and multiply. What
fails is the impossibility argument, not the amendment item it was aimed at.

`std/core`'s own sentence — `String` "has no length, indexing or iteration, so
Nova cannot walk its bytes" — is **not** the false statement here. It is true
where it stands, because it is scoped to `std/core`'s position: `$std.core` is
the first `STD_MODULES` entry and `$std.bytes` and `$std.strings` come after
it, so *that module* cannot walk bytes, which is exactly why `str_hash` has to
be a builtin for it. The error was reading a statement about one module's
position as a statement about the language. The same wording appears without
that scoping in `crates/nova-resolver/src/lib.rs`, on the doc comments at
`Builtin::StrHash` and `Builtin::StrLenChars`, where it reads as a claim about
Nova; the `hashdos-resistance-test` increment changes no code, so that wording
is flagged here rather than edited.

**What shipped is a two-process test, and that is a preference rather than a
necessity.** `hashdos_precomputed_key_set_does_not_survive_a_new_process` in
`crates/nova-cli/tests/run_tests.rs` searches for a colliding set by calling
`.hash()` — the runtime's real hash — in one process, and re-hashes that set in
a second, separately launched process. It **searches** where the amendment item
expected it to **derive**, and both routes were open. The reason for preferring
the search is not that deriving was impossible: a reimplementation in Nova
would pin a second copy of the algorithm and fail for the wrong reason if the
two ever diverged. That test's own doc comment states the same framing at
itself.

Passages are anchored by content here, for the reason the note above this one
gives.

## Global Constraints

- `cargo build --locked --workspace` **before** `cargo test`. `--no-fail-fast`.
- **Sum every `test result:` line across all 44 targets. Never pipe cargo output through `head` or `tail`.** Baseline: **1073 passed / 0 failed / 8 ignored**.
- Clippy `--all-targets -- -D warnings` on ubuntu **and** windows; `cargo fmt --all -- --check`.
- **MSRV 1.78: no `reason = "..."` in any lint attribute.** `OnceLock` stabilised in 1.70, so it is available.
- The ignored ADR-0010 GC tests stay ignored and untouched.
- The poll ABI is frozen and no panic may cross a generated poll boundary — **inert here**: this increment touches no async path and no poll function. Say so rather than implying care satisfied it.
- Every fixture path unique per process.
- Commit messages to a UTF-8 file applied with `git commit -F`, **never a heredoc**. Each body ends exactly `Co-Authored-By: Claude Opus 5 <noreply@anthropic.com>`.
- **Cite no SHA that is not already an ancestor of `main`.** `ba5ab77` is. The spec commit is **branch-local** — it must not appear in any tracked file.
- **Byte-scan every file you write**: no byte below 0x20 outside tab/CR/LF, no `0x7f`, valid UTF-8, and zero occurrences of a backslash-`u` escape followed by four hex digits in tracked markdown. Write code points as `U+XXXX`.

## Two hazards, read before starting

**1. A known pre-existing flake.** Async/threading tests can fail with a Windows `0xc0000005` roughly one run in four and pass on re-run. **Its cause is NOT established.** If you hit it, re-run and say so. Attribute no cause and fix nothing — a previous branch shipped a wrong diagnosis of this and had to retract it.

**2. Do not author Nova string escapes through a bash heredoc.** A quoted heredoc consumed a backslash on the previous branch: `String("a\"b\\c")` reached the file as `a\"b\c` and failed `L0001`. Use the Write tool, or a Python rewrite that asserts a match count.

## Sentence-shape discipline, binding on every comment and record

- **Prefer a roster with no count.** A bare count of a set that can gain a member is a defect here.
- **A corrected number is usually the wrong fix.** Pair a count with the durable predicate and say to re-measure.
- **No ordinals or closed worlds** over `std`, the runtime, the workspace or the record set.
- **Never write that a test is "the only" thing catching something.** That shape has been measured false four times on this project — designed-for is not exclusively-capable.
- **Quote retracted wording inside the retraction**, and remember the consequence: a correct retraction still *contains* the retracted text, so grep cannot tell a retraction from a survival. Read context.
- **grep is line-oriented, so a MISS is not evidence of absence.** Sweep prose with whitespace-tolerant patterns that also normalise `//` and `>` gutters.

## File Structure

| File | Responsibility | Task |
|---|---|---|
| `crates/nova-runtime/src/lib.rs` | the seed, the finalizer, `nova_rt_str_hash`, and their doc comments | 1 |
| `tests/runtime/json_object_forged_map.nova` + `.stdout` | rewritten to compute slots at run time | 1 |
| `tests/runtime/hash_diffusion.nova` + `.stdout` | the finalizer's effect, as an exact structural assertion | 2 |
| `crates/nova-cli/tests/run_tests.rs` | one `#[test]` for the diffusion fixture, one for cross-process variation — **registration is NOT automatic** | 2 |
| `docs/adr/0005-*.md`, `crates/nova-resolver/src/lib.rs`, `nova-spec/20-STDLIB.md`, `docs/adr/0018-*.md`, `std/collections/lib.nova`, `CHANGELOG.md` | the records | 3 |

**Why Task 1 is not split.** Changing the hash breaks `json_object_forged_map`, which assumes `"a"` hashes to slot 0 at capacity 4. If the runtime change landed alone the suite would be red between tasks, and a reviewer cannot approve a change that leaves a golden failing. The runtime edit and the fixture rewrite are one deliverable.

---

## Task 1: The seeded, finalized `str_hash`, and the fixture it breaks

**Files:**
- Modify: `crates/nova-runtime/src/lib.rs` — the doc comment and body of `nova_rt_str_hash` (currently at `:257`, locate by content), plus two new private helpers above it
- Modify: `tests/runtime/json_object_forged_map.nova` and its `.stdout`
- Test: the three existing Rust unit tests in the same file, plus every `json_*`, `hash`, `map_keys` and `collections` golden

**Interfaces:**
- Produces: `fn str_hash_seed() -> u64` and `fn splitmix64_finalize(z: u64) -> u64`, both private to `crates/nova-runtime/src/lib.rs`. Task 2 depends on their *behaviour*, not their names.

- [ ] **Step 1: Record the pre-change baseline, so the mutations later have something to compare against**

```bash
cargo build --locked --workspace && cargo test --workspace --no-fail-fast
```

Sum every `test result:` line across all 44 targets. Expect **1073 passed / 0 failed / 8 ignored**. If it differs, stop and report — the tree is not the baseline this plan assumes.

Then record what the hash does today, which the change is about to alter:

```bash
./target/debug/nova.exe run <a scratch file outside the repo containing: fn main() { println("${("alpha").hash()}") }>
```

On this build, pre-change, that prints `-8447022563750764501`, and prints it again on a second run because the hash is unseeded. Record both runs.

- [ ] **Step 2: Add the seed and the finalizer**

Insert immediately above `nova_rt_str_hash`. `OnceLock` is already imported in `crates/nova-runtime/src/gc.rs`; add `use std::sync::OnceLock;` to this file's imports if it is not already there.

```rust
/// The per-process seed for [`nova_rt_str_hash`].
///
/// ONE value for the whole process. Not per thread and not per call: a `Map`
/// built under one seed and probed under another finds nothing, so a
/// thread-local would corrupt a map shared across threads and reseeding per
/// call would corrupt every map. `OnceLock` gives exactly that, and `gc.rs`
/// already uses the same pattern.
///
/// `RandomState` is `std`'s own `HashMap` seed source, so this draws OS
/// entropy with no new dependency and nothing above MSRV 1.78. Nova code
/// cannot read this value: no builtin exposes it, and `collect_externs`
/// refuses every `nova_`-prefixed `extern` name.
fn str_hash_seed() -> u64 {
    static SEED: OnceLock<u64> = OnceLock::new();
    *SEED.get_or_init(|| {
        use std::hash::{BuildHasher, Hasher};
        let mut h = std::collections::hash_map::RandomState::new().build_hasher();
        h.write_u64(0);
        h.finish()
    })
}

/// splitmix64's finalizer — the same function `std/core`'s `mix64` computes.
///
/// `std/core` writes it in signed Nova with masked shifts, because Nova's `>>`
/// on a negative `Int` sign-extends; on `u64` here the plain form is the same
/// function. Its constants correspond: Nova's `-4658895280553007687` is
/// `0xbf58476d1ce4e5b9` and `-7723592293110705685` is `0x94d049bb133111eb`.
///
/// This is LOAD-BEARING, not decoration. `Map` selects buckets with
/// `hash & (cap - 1)` -- the low bits -- and FNV-1a's low bits barely depend on
/// its starting value: bit 0 of an FNV-1a result is bit 0 of the basis XOR the
/// parity of the input bytes' low bits, so changing the basis flips bit 0
/// identically for both keys of a colliding pair and leaves the collision
/// standing. Measured, seeding alone left 30 of a constructed 48-key attack in
/// one bucket and moved only 66.7% of keys at capacity 8; with this finalizer
/// the same attack falls to 3 and 87.3% of keys move, against an ideal of
/// 87.5%. The spec's section 2 carries the method and the full figures.
fn splitmix64_finalize(mut z: u64) -> u64 {
    z ^= z >> 30;
    z = z.wrapping_mul(0xbf58_476d_1ce4_e5b9);
    z ^= z >> 27;
    z = z.wrapping_mul(0x94d0_49bb_1331_11eb);
    z ^ (z >> 31)
}
```

- [ ] **Step 3: Change `nova_rt_str_hash`'s body**

The seed **replaces** the offset basis rather than being XORed into it — that is the form the spec's measurements were taken against. The FNV prime and the byte loop are unchanged.

```rust
pub unsafe extern "C" fn nova_rt_str_hash(s: *const NovaStr) -> i64 {
    let mut h: u64 = str_hash_seed();
    for b in as_str(s).as_bytes() {
        h ^= *b as u64;
        h = h.wrapping_mul(0x0000_0100_0000_01b3);
    }
    splitmix64_finalize(h) as i64
}
```

- [ ] **Step 4: Amend the function's doc comment**

Its current second paragraph reads: "FNV-1a rather than something stronger because it is small, well known, and adequate for a hash map's bucket selection; it is *not* collision-resistant and must not be used for anything security-sensitive." The first clause is now false and the last is still true. Replace that paragraph with:

```
/// Seeded FNV-1a over the bytes, then splitmix64's finalizer. The seed is
/// per-process (see `str_hash_seed`) and the finalizer is what makes the seed
/// reach the low bits `Map` selects buckets from; the reasoning and the
/// measurements are at `splitmix64_finalize`. This used to be unseeded FNV-1a
/// alone, described here as "adequate for a hash map's bucket selection" --
/// true of an honest workload and false of an adversarial one, which is why it
/// changed.
///
/// STILL NOT COLLISION-RESISTANT, and still not for anything
/// security-sensitive. What the seed and finalizer buy is resistance to a
/// PRECOMPUTED collision set: an attacker who cannot learn the seed cannot
/// build one offline. An adversary who can observe timing and adapt is out of
/// scope, and `mix64` -- the `Int`, `Bool` and `Char` impls -- is not seeded at
/// all.
```

Keep the existing `u64 -> i64` paragraph and the `# Safety` section as they are: both are still exactly true.

- [ ] **Step 5: Rebuild and find out what broke**

```bash
cargo build --locked --workspace && cargo test --workspace --no-fail-fast
```

Sum every `test result:` line. Expect **one** failing fixture, `json_object_forged_map`. Report the actual failure text.

**Verify, and report individually, that each of these still passes** — the spec predicts they survive, and a prediction is not a result:

- `str_hash_is_deterministic_and_distinguishes` and `str_hash_handles_empty` in `crates/nova-runtime/src/lib.rs` (they assert same/not-equal relationships and empty-string stability, all seed-independent)
- `hash_run` — every numeric assertion in `tests/runtime/hash.stdout` is a `mix64` property
- `map_keys_run` — pins counts and per-key lookups, not order
- `collections_run` — pins capacity growth
- every other `json_*` fixture

If anything other than `json_object_forged_map` fails, stop and report before changing a golden. A golden that moves unexpectedly is evidence about the change, not a chore.

- [ ] **Step 6: Rewrite `tests/runtime/json_object_forged_map.nova`**

The old fixture hard-coded slot 0 for `"a"` and slot 2 for `"b"` at capacity 4. It must now find a working layout at run time. Write the whole file with the Write tool (it contains `${...}` interpolation and quoted strings — do not use a heredoc):

```nova
// The `Object` arm's separator placement, pinned behaviourally rather than by
// reading the source.
//
// `stringify` appends the member separator BEFORE looking the value up, so a
// key that `keys()` returns but `get()` misses leaves a comma with no member
// after it. The natural work-stack rewrite moves that push inside the `Some`
// arm, and that version emits `{"a":1}` here instead -- valid JSON, which is
// exactly what makes the trap look like a fix rather than a behaviour change.
// This golden is what refuses it.
//
// Reaching the `None` arm needs a FORGED map: unreachable through `Map`'s own
// API, but `Map` exposes every field and Nova has no field privacy, so a live
// slot placed off its own probe chain makes `keys()` return a key `get()`
// misses.
//
// THE LAYOUT IS COMPUTED, NOT ASSUMED. This file used to hard-code slot 0 for
// the reachable key and slot 2 for the missed one, which held only while
// `str_hash` was an unseeded constant-basis FNV-1a. It is now seeded
// per-process, so the slots differ every run and the layout has to be searched
// for. The three precondition lines below are not decoration: if no layout is
// found, or if the reachable key stops being reachable, this fixture FAILS
// rather than silently stopping exercising the separator.
record Layout { hit: String, miss: String, hs: Int, ms: Int, ok: Bool }

// Find a layout at capacity `cap`: `hit` live at its own home slot so `find`
// reaches it, `miss` live at a LATER slot whose own home slot is empty so
// `find` stops there and answers `None`. Later matters: `keys()` walks slots
// ascending, so the missed member must not be at index 0.
fn find_layout(cap: Int) -> Layout {
    let cands = ["a", "b", "c", "d", "e", "f", "g", "h"]
    let mut hi = 0
    while hi < cands.len() {
        let hs = cands[hi].hash() & (cap - 1)
        let mut mi = 0
        while mi < cands.len() {
            if mi != hi {
                let home = cands[mi].hash() & (cap - 1)
                if home != hs {
                    let mut slot = hs + 1
                    while slot < cap {
                        if slot != home {
                            return Layout { hit: cands[hi], miss: cands[mi], hs: hs, ms: slot, ok: true }
                        }
                        slot = slot + 1
                    }
                }
            }
            mi = mi + 1
        }
        hi = hi + 1
    }
    Layout { hit: "", miss: "", hs: 0, ms: 0, ok: false }
}

fn main() {
    let cap = 8
    let l = find_layout(cap)
    println("layout found ${l.ok}")

    let mut st = [0; cap]
    st[l.hs] = 1
    st[l.ms] = 1
    let mut ks = [""; cap]
    ks[l.hs] = l.hit
    ks[l.ms] = l.miss
    let mut vs: [JsonValue] = [Null; cap]
    vs[l.hs] = Number(1.0)
    vs[l.ms] = Number(2.0)
    let forged: Map<String, JsonValue> = Map { len: 2, used: 2, keys: ks, vals: vs, state: st }

    println("keys ${forged.keys().len()}")
    match forged.get(l.hit) {
        Some(_) => println("hit reachable true")
        None => println("hit reachable false")
    }
    match forged.get(l.miss) {
        Some(_) => println("miss reachable true")
        None => println("miss reachable false")
    }
    println("render ${stringify(Object(forged))}")
}
```

`tests/runtime/json_object_forged_map.stdout`:

```
layout found true
keys 2
hit reachable true
miss reachable false
render {"a":1,}
```

**The last line is the one the fixture exists for**, and note what it does *not* depend on: `stringify` renders the reachable member's key through `quote`, and the search tries `"a"` first, so `"a"` is the hit whenever a layout starting from it exists. If the golden's last line comes out with a different key, the search fell through to a later candidate — report that rather than editing the golden, because it means capacity 8 was tighter than expected and the header's claim about the search needs revising.

- [ ] **Step 7: Rebuild, and confirm the suite is green again**

```bash
cargo build --locked --workspace && cargo test --workspace --no-fail-fast
```

Expect **1073 passed / 0 failed / 8 ignored** — the same total as the baseline, because this task adds no test. Sum every line; report the real number.

Run the rewritten fixture several times in a row and confirm the output is identical each time. The slots differ per process but the *golden* must not — that is the whole point of computing the layout.

- [ ] **Step 8: Clippy, fmt, byte-scan, commit**

```bash
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all -- --check
```

Byte-scan `crates/nova-runtime/src/lib.rs`, the fixture and its golden. Then write the message to a UTF-8 file and apply it with `git commit -F` — never a heredoc — with the body ending exactly `Co-Authored-By: Claude Opus 5 <noreply@anthropic.com>`.

---

## Task 2: The two tests that pin what the change actually did

**Files:**
- Create: `tests/runtime/hash_diffusion.nova` + `.stdout`
- Modify: `crates/nova-cli/tests/run_tests.rs` — two `#[test]`s appended beside the existing fixture tests

**Interfaces:**
- Consumes: the seeded, finalized `nova_rt_str_hash` from Task 1. Nothing else.

- [ ] **Step 1: Write the diffusion fixture**

This is the test that distinguishes the shipped design from the one the spec rejected, and its assertion is **exact rather than statistical**. Bit 0 of an FNV-1a result is bit 0 of the basis XOR the parity of the input bytes' low bits. So keys built only from even-ASCII letters all share bit 0, and without a finalizer they can reach only **half** the buckets — measured over 60 seeds: exactly 32 of 64, every time. With the finalizer: 64 of 64, every time.

Write with the Write tool. Nova has no `Int` to `Char` conversion, so the keys are built by indexing a string array and interpolating.

```nova
// What the finalizer buys, as an exact structural property rather than a
// statistical one.
//
// `Map` selects buckets with `hash & (cap - 1)`, so only the LOW bits matter.
// Bit 0 of an FNV-1a result is bit 0 of its starting value XOR the parity of
// the input bytes' low bits. Every key below is built from letters with an
// even ASCII code, so that parity is 0 for all of them and bit 0 of an
// unfinalized hash is the same for every key -- which puts every key in a
// bucket whose index has that same bit 0, reaching HALF the buckets and no
// more. splitmix64's finalizer mixes high bits down into low ones, which is
// the axis a starting value cannot reach, and the halving disappears.
//
// Measured against 60 random seeds, with keys built exactly as below:
// unfinalized reached 32 of 64 buckets under every seed and its largest
// bucket ran 66 to 76; finalized reached 64 of 64 under every seed with a
// largest bucket of 40 to 53. So `reached` is a structural assertion, not a
// threshold, and the bound on the largest bucket has room on both sides.
//
// The residual flake risk on `reached` is a bucket left empty by chance:
// about 2 in 10^12 per run with these numbers. The bound of 60 on the largest
// bucket sits 7 above the measured finalized maximum and 6 below the measured
// unfinalized minimum.
fn key_at(i: Int) -> String {
    let alpha = ["b", "d", "f", "h", "j", "l", "n", "p", "r", "t", "v", "x"]
    let mut n = i
    let mut out = ""
    let mut d = 0
    while d < 6 {
        out = "${out}${alpha[n % 12]}"
        n = n / 12
        d = d + 1
    }
    out
}

fn main() {
    let cap = 64
    let n = 1999
    let mut counts = [0; cap]
    let mut i = 0
    while i < n {
        let slot = key_at(i).hash() & (cap - 1)
        counts[slot] = counts[slot] + 1
        i = i + 1
    }
    let mut reached = 0
    let mut largest = 0
    let mut j = 0
    while j < cap {
        if counts[j] > 0 { reached = reached + 1 }
        if counts[j] > largest { largest = counts[j] }
        j = j + 1
    }
    println("keys ${n} into ${cap} buckets")
    println("reached ${reached} of ${cap}")
    println("largest under 60 ${largest < 60}")
}
```

`tests/runtime/hash_diffusion.stdout`:

```
keys 1999 into 64 buckets
reached 64 of 64
largest under 60 true
```

- [ ] **Step 2: Write the cross-process test**

The diffusion fixture cannot tell a live seed from a fixed one — it would pass with any constant basis plus the finalizer. This one does that job. Append to `crates/nova-cli/tests/run_tests.rs`:

```rust
/// The seed is per-process, so the same binary run twice must hash the same
/// string differently. This is the test written for that property; it fails
/// deterministically if the seed is replaced by a constant. Two independent
/// seeds colliding would defeat it, at a probability around 2^-64.
#[test]
fn str_hash_seed_varies_across_processes() {
    let dir = std::env::temp_dir().join("nova-str-hash-seed-cross-process");
    std::fs::create_dir_all(&dir).expect("temp dir");
    let file = dir.join("seed.nova");
    std::fs::write(&file, "fn main() { println(\"${(\"alpha\").hash()}\") }\n").expect("write");
    let exe = dir.join(format!("seed{}", std::env::consts::EXE_SUFFIX));
    nova()
        .arg("build")
        .arg(&file)
        .arg("-o")
        .arg(&exe)
        .assert()
        .success();
    let run = || {
        let out = Command::new(&exe).assert().success();
        String::from_utf8_lossy(&out.get_output().stdout).trim().to_string()
    };
    let first = run();
    let second = run();
    assert_ne!(
        first, second,
        "str_hash must differ across processes; both runs printed {first}"
    );
}
```

- [ ] **Step 3: Register the diffusion fixture — registration is NOT automatic**

A `tests/runtime/*.nova` with no `#[test]` runs zero tests and looks like it passes. Append beside the existing fixture tests:

```rust
#[test]
fn hash_diffusion_run() {
    let expected = std::fs::read_to_string(repo_root().join("tests/runtime/hash_diffusion.stdout"))
        .expect("expected-output fixture exists")
        .replace("\r\n", "\n");
    nova()
        .arg("run")
        .arg(repo_root().join("tests/runtime/hash_diffusion.nova"))
        .assert()
        .success()
        .stdout(expected);
}
```

- [ ] **Step 4: Run both, several times**

```bash
cargo build --locked --workspace && cargo test --workspace --no-fail-fast
```

Expect **1075 passed / 0 failed / 8 ignored** — two more than Task 1. **That is a prediction, not a budget.** Report the real summed total and explain any gap; never adjust or omit a test to reach a number.

Then run just these two five times in a row and confirm both pass every time. They depend on a per-process seed, so a single green run proves less here than usual.

- [ ] **Step 5: Run the three mutations and report each**

Rebuild after every mutation and after every revert — the runtime is compiled into the binary these fixtures run against.

1. **Fix the seed to the old constant.** Change `str_hash_seed`'s body to `0xcbf2_9ce4_8422_2325`. Expect `str_hash_seed_varies_across_processes` to FAIL (both runs print the same value). Expect `hash_diffusion_run` to still PASS — the finalizer is still there, and this is exactly why the cross-process test exists.
2. **Drop the finalizer, keep the seed.** Return `h as i64` instead of `splitmix64_finalize(h) as i64`. Expect `hash_diffusion_run` to FAIL with `reached 32 of 64` and `largest under 60 false`. **This is the most important of the three**: it is the mutation that separates the shipped design from the one the spec's section 2 rejected. If it passes, the fixture is not doing its job and you must say so.
3. **Reseed per call.** Replace `str_hash_seed()`'s `OnceLock` with a fresh `RandomState` per call. Expect widespread failure across `Map` fixtures — `map_keys`, `collections`, the `json_*` object tests — because a map probed under a different seed than it was built with finds nothing. Report which failed rather than predicting a list.

Revert and rebuild after each. Confirm the suite is green before moving on.

- [ ] **Step 6: Clippy, fmt, byte-scan, commit**

```bash
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all -- --check
```

Byte-scan the new fixture, its golden and `run_tests.rs`. Commit with `git commit -F` from a UTF-8 file.

---

## Task 3: The records

**Files:**
- Modify: `docs/adr/0005-mutable-receivers-and-one-shot-hash.md`
- Modify: `crates/nova-resolver/src/lib.rs` — the doc comment on `Builtin::StrHash` (near `:89`, locate by content)
- Modify: `nova-spec/20-STDLIB.md` section 7
- Modify: `docs/adr/0018-std-json-scope-and-build-order.md`
- Modify: `std/collections/lib.nova` — `Map`'s header
- Modify: `CHANGELOG.md`

**Interfaces:**
- Consumes: the shipped behaviour from Tasks 1 and 2. Produces nothing.

- [ ] **Step 1: Amend ADR 0005 with a dated note, and say which of its two sentences governs**

This is the substantive record and the reason this increment needed no new ADR. ADR 0005 contains a tension. It says a seeded hasher "is a `Hasher`-shaped question, i.e. it is the migration below", and its closing paragraph says the migration is **not** required for "replacing FNV-1a inside `nova_rt_str_hash` ... and seeding either from a process-start value".

The amendment must record that both are true of different things — a swappable seeded `Hasher` object needs the migration; seeding or replacing the one-shot function does not — and that the closing paragraph governs this change. Quote both sentences. Say plainly that the broader one nearly produced the wrong increment, because a later increment will read these paragraphs as precedent.

It must also narrow the Phase 2.2a disclosure "**Hashes are not randomized per process**, so a `Map` is HashDoS-attackable by adversarial keys": now false for `String` keys and **still true for `mix64`**, i.e. for `Int`, `Bool` and `Char`. Do not delete it; the surviving half is load-bearing.

**Do not touch** the migration decision itself, its prerequisites section, or the backend-independence requirement. None of them changed.

- [ ] **Step 2: Fix the resolver's doc comment**

`crates/nova-resolver/src/lib.rs` documents `str_hash` as "FNV-1a over the string's bytes". That is now incomplete rather than merely stale — it is seeded FNV-1a plus a finalizer. Correct it and point at the runtime function for the reasoning rather than restating the measurements.

- [ ] **Step 3: Narrow the "not claimable" statements — do not delete them**

`nova-spec/20-STDLIB.md` section 7 and `docs/adr/0018-std-json-scope-and-build-order.md` both say Phase 2's throughput gate is not claimable on untrusted input. That is no longer flatly true and is not flatly false either. Replace with what the spec's section 4 permits:

- claimable for **string-keyed** maps against a **precomputing** attacker;
- not claimed against an adversary who can observe timing and adapt;
- not claimed for `Int`, `Bool` or `Char` keys at all, because `mix64` is unseeded.

Quote the retracted wording inside the retraction. Both files carry the claim in their own words, so read each in place — and sweep whitespace-tolerantly, because a line-oriented grep missed a wrapped occurrence of exactly this kind on the previous branch.

- [ ] **Step 4: Amend `Map`'s header in `std/collections/lib.nova`**

`Map::keys()` returns table order, which the header already says. What changes is that table order now varies **per process** for `String` keys, so two runs of the same program can print keys in different orders. Say so, and say that `Int`, `Bool` and `Char` keys are unaffected because `mix64` is not seeded.

Touch no code in this file.

- [ ] **Step 5: `CHANGELOG.md` under `[Unreleased]`**

**Changed:** `str_hash` is seeded per process and finalized. **Known limitation:** what section 4 of the spec says stays exposed. Quote the absolute figures from the spec rather than ratios, and label them machine-specific.

- [ ] **Step 6: Full verification**

```bash
cargo build --locked --workspace && cargo test --workspace --no-fail-fast
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all -- --check
```

Sum every `test result:` line across all 44 targets and report the real total.

- [ ] **Step 7: Whole-branch byte scan, as one population**

Not per commit. Per-commit scans structurally cannot cover this plan and the spec, which were authored before execution began — and that is where this project's only control-byte escape reached a commit, and where the previous branch's one false claim originated.

```bash
git diff --name-only main..HEAD
```

Scan every file in that list plus this plan and the spec: no byte below 0x20 outside tab/CR/LF, no `0x7f`, valid UTF-8, and zero occurrences of a backslash-`u` escape followed by four hex digits in tracked markdown. If you scan with Python, build the backslash with `chr(92)` — its `re` rejects a bare backslash-`u` in a pattern.

- [ ] **Step 8: Commit**

`git commit -F` from a UTF-8 file, body ending exactly `Co-Authored-By: Claude Opus 5 <noreply@anthropic.com>`.

---

## Mutation summary — every one must be run, rebuilt, and reported

| # | Mutation | Task | Must break |
|---|---|---|---|
| 1 | seed fixed to the old constant basis | 2 | `str_hash_seed_varies_across_processes`; `hash_diffusion_run` must still PASS |
| 2 | finalizer dropped, seed kept | 2 | `hash_diffusion_run`, at `reached 32 of 64` |
| 3 | reseed per call instead of once | 2 | `Map` fixtures broadly; report which |

**Rebuild between every mutation.** The runtime is compiled into the binary the fixtures run against, so a stale build measures the unmutated code and reports a false pass.

## What no test asserts, stated rather than implied

The resistance property itself is not test-assertable: building a colliding set requires the seed the design is trying to keep unknown. It rests on the spec's section 2 — its method and its figures — and on mutation 2, which shows the finalizer is doing the work the analysis credits it with. Anyone strengthening this later should re-run section 2's harness rather than trusting these numbers, which are machine-specific in their absolutes though not in their shape.
