# Seeding `mix64` so `Int`, `Bool` and `Char` keys resist a precomputing attacker

**Status:** design, approved 2026-08-28
**Base:** `main` at `0031620` (608 commits, no merge commits, 1076 passed / 0 failed / 8 ignored across 44 targets on Windows)
**Governs:** ADR 0005 (one-shot `Hash`), ADR 0018 sections 3 and 4, `nova-spec/20-STDLIB.md` section 7

## Amendment - 2026-08-29, after the Task 1 and Task 2 audits

Recorded rather than corrected in place, as this project does with plans and
specs: what a design got wrong is part of its record. The sections below keep
their wording and this section governs wherever the two disagree. Every
quotation is verbatim from the source named beside it, allowing for the line
breaks this file's own wrapping introduces inside a quoted span. Most come from
this file's own body; entries D, E and F also quote `tests/runtime/hash.nova`
and its golden, entry B quotes the plan, and entry G quotes
`std/collections/lib.nova` from the revision Task 2 replaced, so that one is
findable in no current file. Sites in this file were located with a sweep that
strips `#`, `*`, `>` and `///` gutters and collapses newlines, run separately
from a line-oriented sweep. They are named by section and quotation rather than
by line number, because a document that cites its own lines invalidates itself
the moment anything above the citation shifts — and inserting this section
shifted every line below it. The plan this spec drives carries its own dated
section for the same audits, and where an entry below has a sibling site there
it says so.

**A. Section 3.3's ratio is quoted from the favourable end of its own range.**
It reads "it costs about 15 ns, roughly a 30 per cent increase on a 35–48 ns
`Int` hash". The derivation is stated, which is the good part, but 15/48 is 31
per cent and 15/35 is 43 per cent, so "roughly 30 per cent" holds only at the
slowest baseline in the range the same sentence quotes. Read it as roughly 30
to 45 per cent.

**B. Section 3.3's two draws do not "remove that".** The sentence reads "Two
draws cost one extra `OnceLock` and remove that", where "that" is leaking
either seed leaking both. Two draws do not remove it.

`RandomState::new()` caches one 128-bit OS draw per THREAD in a `thread_local!`
cell, hands out `(k0, k1)`, and stores back `(k0 + 1, k1)`, so two calls on one
thread share `k1` exactly and differ in `k0` by the number of calls between
them; `std`'s own comment says that increment exists to vary `HashMap`
iteration order rather than to make keys independent. Whichever thread first
reaches each `OnceLock` supplies its draw and neither initialiser pins a
thread, so the two seeds can also come from two separate OS draws; the
shared-`k1` case arises only when one thread initialises both.

What the second draw buys is that the two seeds DIFFER, effectively certainly —
which does remove the exact-equality of `(0).hash()` and `("").hash()` that the
paragraph opens with. Whether recovering one yields the other rests on the
keyed hash's behaviour under a RELATED KEY, and this increment does not verify
that. The shipped argument is `int_hash_seed`'s doc comment in
`crates/nova-runtime/src/lib.rs`; follow it rather than this summary. The plan
carried the same claim in the stronger form "two independent keys", at both of
its own sites, and its own amendment retracts it there and names them.

**C. Section 5's opening count is false; read the sentence with it deleted.**
Section 5 opens "`tests/runtime/hash.stdout` holds eleven assertion lines".
Read it as "holds the assertion lines classified below". Eleven is the number
of bullets and table rows in sections 5.1 through 5.3, not of lines in the
file, because one bullet collapses the `same=true diff=true` relations for
`Int`, `Bool`, `Char`, `String` and the empty-string pair into a single item.
The roster reaches every line; only the number is false. Deleted rather than
replaced, because a corrected count reintroduces the same failure the next time
a line is added or removed — and Task 2 removed the two `&7` histograms, so a
reader who subtracts them from eleven arrives at a smaller number than the
golden holds. The plan carries the same sentence in its Task 2 Step 3 and is
governed the same way there.

**D. The canonical-vector line is misclassified, and the premise is
load-bearing at each of the sites named here.** They are section 3.1, which
says the line "therefore keeps testing the same function it tests today"; that
section's closing sentence; section 5.1's roster entry, which files the line
under what survives untouched "because `mix64` is not touched"; and section
11's criterion for that same line. The plan carries the same premise at its own
sites, which its amendment names.

The fixture line does not call `mix64`. It called `.hash()`, and once the impls
are seeded that expression is `mix64(x1 ^ seed)`, which equals the pinned
constant only at seed 0. Leaving `mix64` untouched preserves the function and
not the assertion.

What shipped, in `tests/runtime/hash.nova`: the fixture recovers the seed and
cancels it. It carries its own `mix64_inv`, sets `let s =
mix64_inv((0).hash())` and asserts `((-7046029254386353131) ^ s).hash()`
against the pinned constant. Since `(x1 ^ s) ^ s` is `x1`, both halves print
`true` at every seed and the golden line survives byte-identical — for a reason
this spec does not state: not that `mix64` was left alone, but that `mix64` is
a bijection and `(0).hash()` is `mix64(seed)`, so ordinary Nova can recover the
seed and cancel it before asserting. The inverse is written out in the fixture
because ADR 0005 makes `mix64` module-private to `std/core` on purpose, so a
fixture cannot call it.

Section 3.1's closing sentence reads "Seeding `mix64` in place would have
destroyed that assertion, which is the strongest one in the fixture", and both
halves need amending. The assertion was destroyed by the route the design did
take as well, and rewritten rather than preserved; both routes required the
line to change, so this was never a reason to prefer one. And "the strongest
one in the fixture" is an unqualified superlative over a growable set. The
shipped fixture distributes that strength per mask instead: it records the
complement-pair lines as mask 1's strongest pin, ahead of the `keys -64..63`
bound, and records mask 2 as resting on the canonical vectors alone after the
golden split, which is why those vectors are still asserted rather than
dropped.

Section 11's criterion — "`splitmix64 canonical x1/x3` passes unchanged,
testing an unmodified `mix64`" — stands as written: the golden line is
byte-identical and `mix64` is unmodified. Its stated reason does not. Flagged
precisely because a reader who checks the criterion and finds it satisfied
would conclude the reasoning behind it was sound.

How the misclassification was reached is the reusable part. The design was
verified by compiling the three seeded impls with a literal `0` standing in for
the seed call and finding the golden byte-identical. At seed 0 the XOR is the
identity, so that probe reproduces the old output BY CONSTRUCTION: it cannot
distinguish "invariant over every seed" from "identical at seed 0". The lines
section 5.1 files as invariant that really are invariant are so for reasons
this one does not have: `bound` and `char/int agree` compare two hashes that
both carry the seed, so it cancels; the `Int`, `Bool` and `Char`
`same=`/`diff=` halves and `h(0) != h(-1)` follow from `mix64` being injective
at every seed; and the complement-collision count is section 5.2's theorem.
Neither reason covers this line, and neither covers `buckets reached 8 of 8`,
which entry E takes up. (The two `String` relations are a third case the
shipped fixture header sets out: `str_hash` carries its own seed, so their
`diff=` halves are seed-dependent at a magnitude around 2^-64 and are left
unbounded on purpose.) This one was classified from the probe instead of from
the route its expression takes. The shipped fixture header states the trap
under "A NOTE ON HOW NOT TO CHECK THIS FILE".

**E. `buckets reached 8 of 8` is a random variable, not an invariant.** Section
5.1 files it under what survives untouched on the evidence "measured min 8 and
max 8 across 1000 seeds", and section 5.3 leans on it: "`buckets reached 8 of
8` from 5.1 asserts that directly and is invariant".

The statistic counts distinct buckets among the 64 multiples of 8 masked into 8
buckets. Recomputed here from the exact occupancy distribution over rationals,
`P(some bucket empty)` is 1.554270e-03, about one run in 643 per process; both
`hash_run` and `hash_build_standalone` read that golden, in separate processes,
which is about one run in 322 across the suite. The evidence given, "measured
min 8 and max 8 across 1000 seeds", is not evidence of invariance: at that rate
1000 draws show no failure about 21 per cent of the time.

What makes this more than an arithmetic slip is that three sentences earlier
the same paragraph refuses a largest-bucket bound because "even `largest < 56`
flakes at **1.62e-04, about 1 run in 6,169**" — a figure this amendment
recomputes as right. So the flake section 5 retained is about nine and a half
times more frequent than the one it rejects by name, and it was retained with
no disclosure.

What shipped is a bound: `buckets reached >= 6: true (identity hashing would
reach 1)`, whose false-failure probability is 4.836243e-12, in the same band as
this section's other two derived bounds at 2.37e-11 and 8.41e-10. A bound of at
least 7 was rejected at 2.825296e-07, about 336 times the loosest bound already
there, which is two and a half orders. The shipped fixture header said three
and has since been corrected to match, so there is no longer a divergence to
reconcile. Nothing real is lost dropping from 8 to 6: what the line guards
is that hashing is not the identity, and `fn hash(self) -> Int { self }` puts
all 64 multiples of 8 in bucket 0, reaching 1, as does a constant hash. The
shipped fixture header derives all of it, and also records what the loosening
costs — nothing in that file constrains the 8-bucket distribution any more,
only its support, so `keys -64..63 reach >= 76 of 256` is what covers low bits
instead.

**F. Section 5.3's replacement text is governed by what shipped.** Its table
prescribes "assert **>= 76**" and "assert **80..176**", and the two histograms
"**dropped**". The shipped golden lines are `keys -64..63 reach >= 76 of 256
buckets: true (unmasked reached 58)` and `Int hashes of 0..255 that are
negative in 80..176: true (masking is mandatory)` — the bound moved into the
printed text and the line prints a boolean rather than the count, which is what
makes the parenthetical contrasts the table asks to keep still meaningful. Both
histograms are gone. Beyond that table, the shipped golden also carries
`buckets reached >= 6`, a further seed-dependent line that neither this section
nor the plan's Step 3 lists as one; entry E governs it.

Together with entry C this changes what a reader should expect the golden to
hold. Subtracting the two dropped histograms from the false count of eleven
gives fewer lines than the golden holds, because the count was of roster items
rather than of lines. The shipped file is the authority on its own contents.

**G. Section 8's list of records to amend did not reach the files named here.**
It names `nova-spec/20-STDLIB.md` section 7 with `docs/adr/0018`, ADR 0005,
`std/core/lib.nova`, `tests/runtime/hash.nova`, `CHANGELOG.md` and
`nova-spec/13-RUNTIME.md`. Task 2 had to change each of these, and none appears
in this list or in any file list in the plan:

- `tests/runtime/collections.nova` with its `.stdout`. The fixture printed how
  many of the 16 `Int` keys `-8..7` hash negative and the golden pinned it at
  9. Seeded, that count is `Binomial(16, 1/2)`, so `P(exactly 9)` is 715/4096,
  about 0.174561; `collections_run`, `collections_build_standalone` and
  `collections_under_gc_stress` each read that one golden in its own process,
  so all three printing 9 has probability about 0.005319 and the collections
  gate would pass about one run in 188. Sixteen keys is too few for any bound:
  even the widest, 1 through 15, misses at one run in 32,768 per process, and
  at one run in 10,923 over the three processes that read that golden. **The
  comparison that decides it is stated per process on both sides**, because the
  two fixtures are read by different numbers of processes and a mixed basis is
  how this entry first got the factor wrong. Per process, 1 in 32,768 is about
  35,100 times the whole flake budget `tests/runtime/hash.nova`'s header sums
  for itself. Per suite run — the three processes reading the collections
  golden against the two reading `hash.stdout` — it is about 53,000. Both
  factors are computed from the exact sum of that header's three bounds; the
  header publishes the budget as about one run in a billion per process, one
  significant figure being all its own argument needs, so recomputing from that
  published rounding gives about 30,000 instead. This entry first gave the
  comparison as "about 105,000 times the fixture's whole flake budget of one run
  in 1,150,179,830", which set the collections figure's three-process rate
  against a per-process budget; that is retracted for the mixed basis and not
  for its arithmetic, which was right for what it computed. The ruling does not
  turn on the basis: at 35,100 or at 53,000, sixteen keys cannot carry a bound.
  An earlier version of this entry,
  and the briefing it came from, argued instead that 1 through 15 was "worse
  than the largest-bucket bound section 5.3 rejects by name". That is false,
  and false in the direction that flatters the deleted field: recomputed, the
  rejected largest-bucket bound is 1.620911e-04, which fires about 5.31 times
  as often as 1 through 15 per process and about 1.77 times as often as its
  three-process figure, so 1 through 15 is the tighter of the two rather than
  the looser. The ruling to delete the field stands; only that argument for it
  falls. The field was deleted.
- `crates/nova-mir/tests/lower_tests.rs`.
  `hash_builtins_lower_to_a_runtime_call_and_a_move` asserted that `impl Hash
  for Char` reaches no runtime function. `int_hash_seed` lowers to a runtime
  call, so that assertion fails at every seed, deterministically. It was
  narrowed rather than deleted, to an equality against
  `vec![RtFunc::IntHashSeed]`, since the property its doc comment names — that
  `char_to_int` lowers to a register move — is untouched, and asserting the
  whole call vector is what keeps that half pinned.
- `std/collections/lib.nova`. Its record header exempted these keys from
  per-process variation — "`Int`, `Bool` and `Char` keys go through `mix64`,
  which is not seeded, so their layout is unchanged and stable across runs" —
  and its note on `keys()` order confined the stronger statement to `String`
  keys on the same ground. This increment falsifies both, and the shipped
  paragraphs now turn on whether a key type is seeded rather than on which one
  it is.

Section 8's closing instruction to enumerate mechanically rather than work from
its list was the right instruction, and this is what it would have caught. Why
these were missed is worth more than the corrections: every sweep in this
increment scoped the golden question to the fixture whose subject is hashing,
and these pin hash-derived values incidentally. The files missing from this
spec's list and the plan's were found by sweeping for prose that ASSERTS the
old behaviour, not by reasoning about which files the change touches — a file
list built from "what does this code change" cannot reach a file that only
DESCRIBES the behaviour, and on this increment that was the larger set.

**H. "Exact probability" names a model, not the run.** Section 5.3 says the
negative count "has exact probability **8.41e-10, about 1 run in 1.19
billion**", says `largest < 56` "flakes at **1.62e-04, about 1 run in 6,169**",
and section 11's criterion asks that the bounds appear "with their exact
probabilities" in the fixture's header. Each figure is exact for a model in
which the hashes involved are independent and uniform. They are not
independent: every one is a deterministic function of a single 64-bit seed, so
at most 2^64 outcomes stand where the model for the 128-key line alone has
256^128, and the model therefore cannot be the true joint distribution. Read
each figure as exact for the model and an estimate for the run.

Separately, 1.62e-04 is a sum over the 8 per-bucket tails, which are not
disjoint events, so it is a Bonferroni upper bound on the flake probability
rather than that probability. The shipped fixture labels it as one and carries
the model caveat in its header. What the shipped figures are is computed rather
than sampled — finite exact summations, inclusion-exclusion over an occupancy
distribution or a binomial tail — so no sampling error enters and "closed form"
would not be the right phrase for them either. The plan carries the same two
shapes at its Step 3 table header and its Step 4 derivation, and the root
record for the eleven-figure form of the cross-process bound is
`docs/superpowers/specs/2026-08-27-hashdos-resistance-test-design.md`, amended
today.

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
