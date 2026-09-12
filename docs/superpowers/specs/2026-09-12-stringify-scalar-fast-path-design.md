# A scalar fast path for `stringify` — design

**Date:** 2026-09-12. **Base:** `main` at `b9dc13d`, 707 commits, 0 merge
commits ever, 1130 passed / 0 failed / 8 ignored across 45 targets.

## 1. What this is, and what it is not

`std/json`'s `stringify` renders a top-level scalar by computing the complete
answer and then copying it, character by character, into a second buffer that
it drains again. This returns the answer directly instead.

**It adds no public API and changes no caller.** `examples/05-json-api` is not
touched; its `stringify(String(u.name))` simply becomes cheaper. Nothing about
Phase 2's gate changes — the absolute 10k criterion remains unreachable by
response-path work alone, for the reason
`docs/superpowers/specs/2026-09-11-bun-ratio-design.md` §2 records, and this
increment does not claim otherwise.

**It is not a rewrite of the example's accumulator.** A spike measured that
directly and the answer was no: the interpolation accumulator is about 4.5% of
`users_json` at ten users, and `",".join` — the only alternative user code can
reach — was not faster.

## 2. The waste, read from the code rather than inferred

`stringify` allocates an output `Vec<Char>` and a work stack, pushes the value,
pops it, checks depth, and dispatches. Its four scalar arms are:

| arm | current | what it returns into the buffer |
|---|---|---|
| `Null` | `buf_push_str(out, "null")` | a literal |
| `Bool(b)` | `buf_push_str(out, if b { "true" } else { "false" })` | a literal |
| `Number(n)` | `buf_push_str(out, number_to_json(n))` | a complete `String` |
| `String(s)` | `buf_push_str(out, quote(s))` | a complete `String` |

In every case the argument to `buf_push_str` **is already the finished
rendering**. `buf_push_str` then calls `.chars()` on it and pushes each
character into `out`, and `vec_chars_to_string(out)` drains `out` back into a
`String`. For a top-level scalar that is two full round trips through a
`Vec<Char>`, plus a per-character Nova loop, plus two `Vec` allocations and the
`Work`/`Pending` wrappers — for a value that needed none of it.

**Measured, one run each, compiled release-runtime binary of 514,048 bytes:**
`stringify(String("User 1"))` costs **4,610 ns** against **1,960 ns** for
`quote` alone. The difference is the second round trip and the machinery.

## 3. The design

A scalar fast path at the top of `stringify`, before anything is allocated:

```nova
pub fn stringify(v: JsonValue) -> String {
    match v {
        Null => return "null"
        Bool(b) => return if b { "true" } else { "false" }
        Number(n) => return number_to_json(n)
        String(s) => return quote(s)
        _ => {}
    }
    // The general path below, unchanged, now reached only by Array and Object.
}
```

Both constructs this relies on were checked on this host rather than assumed:
`return` works as a match arm body, and a `_ => {}` arm lets control fall
through to the code after the match.

**All four arms, not just the measured one.** Only `String` has a measurement
behind it. Fixing that one and leaving three textually identical wastes in
place is the shape this project has repeatedly been burned by — a fix applied
to the artifact in hand rather than to the population that shares the defect.
The equivalence argument in §4 is one argument covering all four, and each gets
its own test arm. **The record must say that only the `String` arm's win was
measured**, and must not imply a figure for the others.

**No new public name, and that is the point.** An earlier version of this plan
was going to expose `quote` as public API. `std/strings`' own `join` comment
explains why that is costly: a top-level `pub fn` is glob-imported into every
module and would take that name from all user code. `std/json` currently
claims exactly two global FUNCTION names, `stringify` and `parse`, and
`quote` is far more collision-prone than either -- `JsonValue`, `JsonError`,
`ToJson`, `FromJson` and the six variant constructors are already
glob-exported too, several more collision-prone than `quote` would have
been, but none of them is a `pub fn`. The fast path removes the need
entirely, and makes every existing caller that passes a top-level scalar
faster -- which is what `examples/05-json-api` does -- rather than only
callers rewritten to use a new function. A caller that passes a top-level
`Array` or `Object` pays one extra match arm for no saving: `stringify`'s
own scalar arms inside the general path are untouched, so a caller reaching
them through a container gets no speedup either.

## 4. Why the output is byte-identical, argued rather than asserted

For a single top-level leaf the general path reduces to: allocate an empty
`out`; push `Value(Pending { v, d: 0 })`; pop it; test `d > MAX_RENDER_DEPTH`;
run the arm, which is `buf_push_str(out, X)` for that arm's `X`; find the stack
empty; return `vec_chars_to_string(out)`.

- **The depth guard cannot fire.** `MAX_RENDER_DEPTH` is `100_000` and a
  top-level value carries `d = 0`, so `0 > 100_000` is false. An early return
  therefore cannot skip a panic that would otherwise happen.
- **`buf_push_str` into an empty buffer is a pure copy.** It appends `X`'s
  characters in order and nothing else, so `out` holds exactly `X`'s
  characters.
- **Draining them back is lossless for any `X` that is already a `String`.**
  `crates/nova-runtime/src/lib.rs`'s
  `str_from_chars_round_trips_and_substitutes_invalid_scalars` pins the round
  trip for the empty string, ASCII, `café`, `日本語` and the astral-plane
  `🦀🇹🇭`. Its substituting case — a lone surrogate becoming U+FFFD —
  is reachable only by hand-building a `[Char]` block holding a non-scalar,
  which no `String` returned by `quote` or `number_to_json` can be.

So `vec_chars_to_string(out)` equals `X`, and returning `X` directly is the
same value. The argument is per-arm only in the choice of `X`; the reasoning
above is identical for all four.

## 5. Testing

**Existing gates already cover this and are not re-derived here:** `std/json`'s
own suite, `examples/05-json-api`'s golden test over nine exchanges, and
`docs/benchmarks/bun-equivalence.js`, which compares response body bytes
between the example and the Bun server. Any change to rendered bytes fails at
least one of them.

**Added, because none of those compares the two paths against each other:** a
test asserting the fast path agrees with the general path per scalar arm. The
general path cannot be called directly once the fast path shadows it, so the
test compares against **literal expected strings**, which is what the existing
`std/json` tests do:

| case | expected |
|---|---|
| `Null` | `null` |
| `Bool(true)`, `Bool(false)` | `true`, `false` |
| `Number` finite | its `number_to_json` rendering |
| `Number` non-finite | `null`, which is deliberately lossy and already recorded |
| `String("")` | a pair of quotes |
| `String` with a quote and a backslash | both escaped |
| `String` with a control character | its escape |
| `String` astral-plane | unchanged, as the runtime round-trip test pins |
| `Array` and `Object` containing scalars | unchanged, proving the general path still renders leaves |

That last row is load-bearing: the scalar arms remain reachable from inside a
container, so a fast path that accidentally deleted them would still pass every
scalar case above.

**A mutation, run and reported rather than predicted.** Break the fast path's
`String` arm — return `s` unquoted — and confirm something fails. A fast path
whose escaping is wrong and whose tests pass anyway is the failure mode worth
disproving, and this project's own record has three cases of a comment
asserting coverage no test performed.

## 6. What is measured, and what is not

Measured before and after, under the discipline already in force: compiled
release-runtime binary with its **byte size recorded**, one fresh process per
data point, and a **range or an explicit statement that it was one run**.

- `stringify` per call, at the `String` arm.
- `users_json` at 5, 10, 20 and 40 users, which is where the caller's win shows.

**Not measured, and the record must say so:** the `Null`, `Bool` and `Number`
arms, and the server's request throughput. The server figure is deliberately
excluded from PREDICTION: the amplification between an isolated cost and the
whole-server marginal is 2.0x to 3.2x and **unexplained**, so a throughput
claim derived from the isolated win would be extrapolation.

### 6.1 The recorded server figures now describe a superseded build

**The most important consequence of this increment, and the first draft of this
spec missed it.** `examples/05-json-api/BENCHMARK.md` records 1875.2 to 3108.5
req/sec for the absolute criterion and 0.116 to 0.231 for the Bun ratio. Every
one of those was measured against the current `stringify`, through
`users_json`, which this change makes cheaper. After it lands they are figures
for a superseded build.

Leaving them unmarked would recreate exactly the defect this project spent an
increment withdrawing: **a published figure whose stated subject is not the
artifact that produced it.**

Two things keep the record honest, and neither is a throughput claim:

- **The identity check already in the file does its job here.** Those figures
  are recorded beside the binary's byte size, 690,176. This change alters the
  binary, so a reader who rebuilds and compares sizes sees immediately that the
  figure predates their build. That is the mechanism working as intended rather
  than a gap.
- **A dated note states it in words as well**, naming what changed and that the
  figures were taken before it. Prose plus a checkable fact, because the lesson
  of that increment was that a warning without a check is not enough -- and
  equally that a check nobody reads needs its prose.

**Re-measuring the server is explicitly NOT in scope**, and that is a scope
decision rather than an oversight: it is the full four-cell matrix with
replicates plus the equivalence check, it would double this increment, and the
isolated `users_json` measurement above is what establishes the change did what
it claims. Re-measurement goes to the debt queue, named, so the next person who
wants a current throughput figure knows it is owed rather than missing.

## 7. Records to amend

- `CHANGELOG.md` under `[Unreleased]`.
- `std/json/lib.nova`'s own cost notes, which discuss the accumulator's
  quadratic-to-flat migration and should name this as a separate, later
  saving on the same function.
- `examples/05-json-api/BENCHMARK.md` — two changes, not one. The `users_json`
  figure recorded there moves, and the file is the destination
  `nova-spec/60-EXAMPLES.md` §5 names. Separately, a dated note recording that
  its server throughput and Bun-ratio figures were measured before this change
  and describe a superseded build, per §6.1, with re-measurement named as debt
  rather than silently owed.

**No `nova-spec/` amendment.** No gate, criterion or specified behaviour
changes; `stringify`'s contract is the same function of its input.

## 8. What this does not cover

- **The `Array` and `Object` arms.** They legitimately need the work stack.
  A nested scalar still pays one round trip through the shared buffer, which is
  the design and not waste.
- **`buf_push_str`'s per-character loop.** Appending in bulk would need a
  runtime intrinsic that does not exist; out of scope.
- **The example's accumulator.** Measured at about 4.5% and left alone.
- **Phase 2's gate.** Unreachable by response-path work, and untouched.
- **Re-measuring the server's throughput and the Bun ratio.** Deferred with a
  reason, marked in the record, and named in the debt queue -- see section 6.1.
- **Any public API.** Nothing is added, renamed or exposed.

## 9. Success criteria

1. `stringify` returns early for all four scalar arms, and the general path is
   unchanged for `Array` and `Object`.
2. The new test covers every row of §5's table, including a container holding
   scalars.
3. The mutation was run and its actual outcome recorded — not predicted.
4. `stringify`'s per-call cost and `users_json` at four collection sizes are
   measured before and after, with the binary's byte size beside each figure
   and either a range or an explicit "one run".
5. The records in §7 are amended, and state plainly that only the `String`
   arm's win was measured and that no throughput figure is claimed.
6. `examples/05-json-api/BENCHMARK.md` carries a dated note that its server
   throughput and Bun-ratio figures predate this change and describe a
   superseded build, and the debt queue names the re-measurement as owed.
7. Suite at **1130 passed / 0 failed / 8 ignored** across 45 targets plus
   whatever the new test adds, `cargo fmt --all -- --check` clean, and
   `cargo clippy --locked --all-targets --all-features -- -D warnings` clean.
8. No public API added, and no file under `nova-spec/` or `examples/`
   modified except `examples/05-json-api/BENCHMARK.md`.
