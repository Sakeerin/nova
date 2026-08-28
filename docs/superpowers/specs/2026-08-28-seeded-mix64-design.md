# Seeding `mix64` so `Int`, `Bool` and `Char` keys resist a precomputing attacker

**Status:** design, approved 2026-08-28
**Base:** `main` at `0031620` (608 commits, no merge commits, 1076 passed / 0 failed / 8 ignored across 44 targets on Windows)
**Governs:** ADR 0005 (one-shot `Hash`), ADR 0018 sections 3 and 4, `nova-spec/20-STDLIB.md` section 7

## 1. The exclusion this closes, and the two it does not

`nova_rt_str_hash` is seeded per process and finalized with splitmix64, and a
test executes precomputation resistance for string-keyed maps. Throughout that
work `mix64` was left unseeded and the gap was disclosed rather than fixed. The
gate claim reads, in `nova-spec/20-STDLIB.md` section 7 and `docs/adr/0018`:

> the gate is claimable **for string-keyed maps against a precomputing
> attacker** [...] It is **not** claimed against an adversary who can observe
> timing and adapt, and it is **not** claimed for `Int`, `Bool` or `Char` keys
> at all, because `mix64` is unseeded and those keys' buckets are still a
> function of the key alone.

This increment closes the **third** exclusion and only that one. The
timing-adaptive adversary stays unclaimed, and nothing here makes the hash
cryptographic.

## 2. Why the obvious route is dead, measured rather than reasoned

`mix64` is written in Nova, in `std/core`, so a seed has to come from
somewhere Nova can reach. `str_hash` is reachable there — `std/core` already
calls it for `impl Hash for String` — and `str_hash("")` returns
`splitmix64_finalize(str_hash_seed)`, a value stable for the whole process. So
seeding without touching the compiler looked possible.

**It costs too much.** Measured on this tree, 500,000 iterations per figure,
median of five runs:

| operation | cost per call |
|---|---|
| `(i).hash()` today | 35–48 ns |
| `+ s.len()`, string hoisted out of the loop | +14 ns |
| `+ s.hash()`, string hoisted out of the loop | +16 ns |
| `+ ("").hash()`, literal allocated per call | +950 ns |
| `+ Instant::now().nanos`, runtime call plus a record | +855 ns |

The first two rows and the last two separate the cost cleanly: **a runtime call
costs about 15 ns and a GC allocation costs about 900 ns.** Seeding from
`str_hash("")` allocates a `String` per hash, so it would make `Int` hashing
roughly 26 times slower on `Map`'s hot path.

Worth recording because it nearly produced the wrong answer: the first two
measurements taken were the string route and `Instant::now()`, which **both
allocate**. They agreed at about 900 ns and together supported the conclusion
that every route was unattractive. The third measurement, with the allocation
hoisted out, reversed it.

So a seed must arrive without allocating, which means a new intrinsic.

## 3. The design

Three parts, and the first is the one that preserves the most.

### 3.1 `mix64` is not touched

`mix64` stays exactly as it is: unseeded, canonical splitmix64's finalizer,
written in Nova with its masked shifts and its complement-collision reasoning
intact. `tests/runtime/hash.nova`'s `splitmix64 canonical x1=true x3=true`
therefore keeps testing the same function it tests today, and ADR 0005's record
of those figures as *computed from the algorithm* stays true of the thing it
describes.

Seeding `mix64` in place would have destroyed that assertion, which is the
strongest one in the fixture.

### 3.2 The three `Hash` impls seed their input

```
impl Hash for Int    { fn hash(self) -> Int { mix64(self ^ int_hash_seed()) } }
impl Hash for Bool   { fn hash(self) -> Int { if self { mix64(1 ^ int_hash_seed()) } else { mix64(0 ^ int_hash_seed()) } } }
impl Hash for Char   { fn hash(self) -> Int { mix64(char_to_int(self) ^ int_hash_seed()) } }
```

**Pre-XOR, not post-XOR, and this is load-bearing.** `Map` selects buckets with
`hash & (cap - 1)`. Post-XOR — `mix64(x) ^ seed` — leaves the low bits as
`(mix64(x) & mask) XOR (seed & mask)`, which *permutes* buckets and leaves every
colliding pair still colliding. It would look like seeding and buy nothing.
Pre-XOR changes which pairs collide, because `mix64` is a bijection and the seed
moves each key to a different point of its domain.

This is the same failure mode measured on the string path, where dropping the
finalizer left the six bucket-selecting bits a function of `seed & 63` alone —
64 layouts rather than 2^64. Low-bit masking makes a seed far weaker than its
width suggests, and the fix in both cases is to put the seed where a strong
mixer still runs after it.

### 3.3 One new intrinsic supplies the seed

`int_hash_seed() -> Int`, an `STD_ONLY` builtin over a runtime function that
returns a per-process value from a `OnceLock<u64>` filled from
`std::collections::hash_map::RandomState` — the same source `str_hash_seed`
uses. It allocates nothing, so by section 2's measurement it costs about 15 ns,
roughly a 30 per cent increase on a 35–48 ns `Int` hash.

**A separate draw from `str_hash_seed`, not the same value.** If the two shared
a seed then `(0).hash()` would equal `("").hash()` exactly, since both reduce to
`mix64(seed)` — a startling coincidence, and leaking either would leak both. Two
draws cost one extra `OnceLock` and remove that.

## 4. The seed is recoverable, stated up front

`(0).hash()` returns `mix64(0 ^ seed)`, which is `mix64(seed)`, and `mix64` is
an invertible bijection with a published inverse. So **one call from ordinary
Nova code recovers the `Int` seed exactly**, exactly as `("").hash()` already
recovers the string seed.

This is not a new weakness introduced here. It is the same shape the string path
already has, it follows from ADR 0005's one-shot `Hash` returning a plain `Int`,
and the gate claim already declines the adaptive attacker for whom it matters.
What the seed buys is precomputation resistance: an attacker cannot build a
colliding key set before the process starts.

Any record written by this increment must say this rather than imply the `Int`
path is stronger than the `String` path. It is exactly as strong, and no more.

## 5. The golden, split by what survives

`tests/runtime/hash.stdout` holds eleven assertion lines. Classified by
simulating the proposed seeded form over 1000 seeds, with the model validated
by the fact that **seed 0 reproduces today's golden exactly** — `mix64(x ^ 0)`
is `mix64(x)`, and the simulation returns today's 100-of-256 and 132-negatives
figures.

### 5.1 Survives untouched, invariant over every seed tested

- the `same=true diff=true` relations for `Int`, `Bool`, `Char`, `String`, and
  the empty-string pair — structural, not numeric;
- `bound true true true`, the masking rule;
- `char/int agree true true`;
- `h(0) != h(-1): true`;
- `splitmix64 canonical x1=true x3=true`, because `mix64` is not touched;
- `buckets reached 8 of 8 (identity hashing would reach 1)` — measured min 8 and
  max 8 across 1000 seeds. This is the line that carries the anti-identity
  content, which matters for 5.3.

### 5.2 Strengthens from a measurement to a theorem

`complement collisions over 0..63: 0` — measured 0 for every one of 1000 seeds,
and **provably 0 for all of them**: the pairs are `x` and `-1-x`, `mix64` is
injective, and `x ^ s == (-1-x) ^ s` would require `x == -1-x`, which no `Int`
satisfies. The line stays, and its comment should record that seeding turned it
from an observed count into a consequence.

Note what this does *not* weaken: `mix64`'s masks still exist for the reason
`std/core` gives, and `hash.nova` still pins that unmasked shifts would fold the
finalizer 2-to-1. Seeding does not rescue an unmasked `mix64`.

### 5.3 Becomes seed-dependent, and what replaces each

| line | today | over 1000 seeds | replacement |
|---|---|---|---|
| `keys -64..63 reach 100 of 256 buckets` | 100 | 90–112 | assert **>= 76** |
| `Int hashes of 0..255 that are negative: 132` | 132 | 100–153 | assert **80..176** |
| `keys 0..255 &7: 34 28 30 30 30 38 28 38` | histogram | varies freely | **dropped**, see below |
| `multiples of 8 &7: 11 8 6 6 10 10 5 8` | histogram | varies freely | **dropped**, see below |

Both bounds are exact, not sampled:

- **Distinct buckets.** 128 keys into 256 buckets is an occupancy problem;
  computed exactly with `P(exactly j occupied) = C(256,j) * Surj(128,j) /
  256^128`, the expected count is 100.88 and `P(distinct < 76)` is
  **2.37e-11, about 1 run in 42 billion**. The distribution sums to 1.000000 as
  a check, and 20,000 simulated seeds gave a minimum of 85, nine above the
  bound.
- **Negative count.** Each of 256 keys hashes negative with probability 1/2, so
  the count is `Binomial(256, 1/2)` with mean 128; outside `80..176` has exact
  probability **8.41e-10, about 1 run in 1.19 billion**.

**The two histograms are dropped rather than replaced, and the rejected
alternative is recorded because it is a trap.** The obvious replacement is a
largest-bucket bound, as `hash_diffusion` uses. Computed exactly for 256 keys in
8 buckets — `Binomial(256, 1/8)`, mean 32, unioned over the buckets — even
`largest < 56` flakes at **1.62e-04, about 1 run in 6,169**. That is the same
order as the undisclosed flake the previous increment had to remove. Eight
buckets is too few for a tight bound at this key count, so no largest-bucket
assertion is added here. What the histograms were really guarding is that
hashing is not the identity, and `buckets reached 8 of 8` from 5.1 asserts that
directly and is invariant.

## 6. The cost, and the two sites that can be forgotten

One new intrinsic pays the 12-site checklist whose counting rule ADR 0018
section 3 states: 12 sites across five files, of which 7 are compiler-forced.
`str_to_float` is the worked anchor to read at every site.

Two facts from that record bear directly on this increment's verification, and
both were measured there rather than reasoned:

- **`cargo check --workspace` finds only 6 of the 7 forced sites and reports
  success.** One forced site is a table inside `nova-typeck`'s `#[cfg(test)]`
  module, so `--all-targets` is mandatory.
- **The 7 are not discoverable in one build.** `nova-resolver`'s name table
  fires alone on the first pass, because cargo cannot compile downstream crates
  until the resolver builds. Fixing the first error and stopping means having
  seen one seventh of the work.

Of the unforced sites, two can actually be forgotten and they fail differently:

- **`STD_ONLY`** — omitting the element *and* its length together compiles the
  whole workspace clean, but `std/core` is compiled on every `nova` invocation,
  so the omission yields `E0001: cannot find function` immediately and
  universally.
- **`symbols()`** in `crates/nova-runtime/src/lib.rs` — its omission survives
  every compiler in the pipeline, Rust's and Nova's, to link time inside the
  JIT. It is held by one guard test,
  `every_rt_func_symbol_is_registered_with_the_jit` in
  `crates/nova-codegen-cranelift`.

The runtime cost is the ~15 ns per `Int`/`Bool`/`Char` hash from section 2, paid
on every `Map` operation over those key types.

## 7. Mutations that must fail

Each states the expected outcome, and the implementer reports the observed one.

1. **Post-XOR instead of pre-XOR** — `mix64(self) ^ seed`. A cross-process test
   over `Int` keys must fail: the seed permutes buckets without breaking
   collisions, so a set built under one seed still collides under another.
2. **Constant seed** — return a fixed value from the seed function. The
   cross-process test must fail.
3. **Seed the `Int` impl but not `Char`** — `char/int agree` must fail, catching
   the case where the impls drift apart.
4. **Remove the `symbols()` entry** — `every_rt_func_symbol_is_registered_with_the_jit`
   must name the new symbol. ADR 0018 records this as the site whose omission is
   invisible to every compiler in the pipeline and is held by a test rather than
   by the type system, so running the mutation proves that guard bites for this
   symbol rather than assuming it does. Whether some other test would also catch
   it is not asserted here.
5. **Unmask one shift in `mix64`** — `complement collisions over 0..63` must go
   non-zero. Seeding must not have rescued an unmasked finalizer, and this
   proves 5.2's theorem depends on `mix64` staying a bijection.

## 8. Records to amend

- `nova-spec/20-STDLIB.md` section 7 and `docs/adr/0018` — the gate claim's
  third exclusion is closed. The other two stay, with their wording unchanged.
  A version stating only the improvement is wrong.
- ADR 0005 — a dated amendment. Its Phase 2.2a disclosure that hashes are not
  randomized per process was already narrowed for `String`; it now closes for
  `Int`, `Bool` and `Char` too.
- `std/core/lib.nova` — `mix64`'s own note, saying it is deliberately unseeded
  and that the seeding lives in the impls above it, so a reader does not take
  the untouched function as an oversight.
- `tests/runtime/hash.nova` — the fixture's header, recording which lines are
  invariant, which became theorems, and which were replaced with derived bounds.
- `CHANGELOG.md` under `[Unreleased]`.
- `nova-spec/13-RUNTIME.md` — the intrinsic roster, and `STD_ONLY`'s count.

Enumerate mechanically rather than working from this list. Five separate times
in the preceding week an enumeration was correct and the file list derived from
it was short.

## 9. Hazards

- `std/*/lib.nova` is embedded in the compiler via `include_str!`, so a std-side
  change measured against a stale binary is a **false pass**. Build before test.
- Never author Nova string escapes through a quoted shell heredoc; one has
  silently eaten a backslash twice on this project.
- Two live flakes, neither to be fixed or diagnosed: the `0xc0000005` family,
  whose cause is unproven and which fired once during the preceding week's work,
  and `net::tests::connecting_to_a_closed_port_is_connection_refused`, whose
  bind-then-drop diagnosis was refuted by measurement and which is now
  instrumented rather than fixed. Do not re-derive the port-theft story.
- No new fixture may pin `Map` or `Set` iteration order.
- Backslash scans must be done in Python with the pattern built from `chr(92)`,
  after asserting it matches a planted positive: `grep -E` on a shell-quoted
  backslash-u matches the word "succeeds", so a clean result from it proves
  nothing.

## 10. Out of scope

- The timing-adaptive adversary. Unchanged and still unclaimed.
- Making any hash cryptographic.
- Removing the seed's reachability from Nova, which follows from ADR 0005's
  one-shot `Hash` returning a plain `Int` and is an ADR-level question.
- `Hash for Float`, which still needs a NaN decision.

## 11. Success criteria

- A cross-process test fails when the seed is made constant and when the XOR is
  moved after `mix64`, with both failures observed rather than predicted.
- `splitmix64 canonical x1/x3` passes unchanged, testing an unmodified `mix64`.
- `complement collisions over 0..63: 0` passes, with its comment recording that
  it is now a consequence of injectivity rather than a measurement.
- The two replacement bounds appear with their exact probabilities in the
  fixture's own header, so the next reader sees the derivation without leaving
  the file.
- `every_rt_func_symbol_is_registered_with_the_jit` is shown to bite for the new
  symbol by removing the entry and watching it name it.
- The suite total rises by the number of tests added and nothing else moves,
  summed from every one of the 44 `test result:` lines on each CI leg.
- The gate claim's two surviving exclusions read exactly as they do today.
