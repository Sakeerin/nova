# Pinning HashDoS resistance with an executing test

**Status:** design, approved 2026-08-27
**Base:** `main` at `515145b` (584 commits, no merge commits, 1075 passed / 0 failed / 8 ignored across 44 targets on Windows; CI gate per leg: ubuntu 1067/0/1, macos 1068/0/0, windows 1075/0/8)
**Governs:** ADR 0005 (one-shot `Hash`), ADR 0018 section 4, `nova-spec/20-STDLIB.md` section 7

## Amendment - 2026-08-29 — item (f)'s eleven-figure union bound

Recorded rather than corrected in place, so both figures stay in this file: a
grep for either lands here, and the record that a figure labelled "the exact"
value was wrong survives, which this derivation's history makes worth keeping.
An in-place correction would leave no trace that the label was ever unreliable.
Item (f)'s account of the earlier reciprocal error, including the reconstructed
early-exit cause, is not in question and is not touched.

This document now carries two correction locations, covering different things.
`## 11. Correction, 2026-08-27` records what the design above it got wrong,
item by item, and item (f) is where the figure this section corrects still
stands. This section corrects one figure inside that correction; it is dated
later and is unnumbered because it postdates the numbering. Item (f) carries a
pointer forward to here, so a reader who goes to section 11 for what this spec
got wrong does not meet the wrong figure unflagged.

### The figure

Item (f) states the `at most 10` threshold's union bound twice. Its paragraph
on that row's reciprocal says "the exact value is 8.26868630e-11" — correct at
nine significant figures, and also what the other model below rounds to at
nine, so it does not distinguish them. Its next paragraph, reconstructing the
early-exit origin, says "rather than the exact 8.2686863018e-11" — that
eleven-figure string belongs to the other model.

Recomputed here rather than taken from anywhere: by two independent routes that
agree on one exact rational, rendered through `Decimal` and never through a
float. With `P` the single-bucket tail `P(Binomial(32, 1/64) >= 11)`:

| quantity | at eleven significant figures | reciprocal |
|---|---|---|
| `64 * P`, the sum this label names | 8.2686863021e-11 | 12,093,819,543.57 |
| `1 - (1-P)^64`, an independence model | 8.2686863018e-11 | 12,093,819,544.07 |

The two agree at ten significant figures and part at the eleventh. Eleven is
therefore the precision this argument uses, and no more is published here — the
same reason the shipped test comment gives for stating its own reciprocal in
words.

The shipped figure matches the independence model at every printed digit, and
an eleven-figure match is not coincidence: an exactly computed probability was
relabelled as a union bound upstream. The figure is right for the model that
was computed and wrong for the model the words describe.
`docs/superpowers/plans/2026-08-28-seeded-mix64.md` carries the same
eleven-figure string in its Task 2 Step 4, copied from here, and is governed by
that plan's own amendment.

### Why every check passed, which is the transferable part

Both reciprocals round to 12,093,819,544, so the mantissa and the reciprocal
printed beside it were mutually consistent — both correct, for the same
unlabelled model. Cross-checking one against the other could not have detected
this. What catches the class is recomputing the quantity THE LABEL NAMES.

The two rounds this passage now records are mirror images of each other. In the
round item (f) already describes, a truncated tail sum was invisible in the
mantissa and visible in the reciprocal derived from it, so the pair DISAGREEING
is what exposed it. In this round the pair AGREED, because both halves came
from one computation of a model the label did not name. So agreement between a
figure and a second figure derived from it is not a check on the label, and
disagreement is not the only failure signature to watch for. A doc comment on
`hashdos_precomputed_int_key_set_does_not_survive_a_new_process` in
`crates/nova-cli/tests/run_tests.rs` gave the cause as each half of one
derivation having been checked against the other rather than against a
recomputation; that account was retracted there, because both halves were in
fact correct.

### Neither figure is the probability of the failure event

`64 * P` is a Bonferroni sum over 64 per-bucket events that are not disjoint,
so it is an upper bound on the probability that the largest bucket exceeds 10
rather than that probability; the independence form assumes an independence the
events do not have. What is computed exactly is the BOUND, not the probability,
and the true probability lies below it. So this amendment does not swap one
figure labelled "the exact" value for another: the union bound is exactly
8.2686863021e-11 at eleven significant figures, and it bounds the failure
probability from above.

The same reading applies to the other derived probabilities here — the
threshold table's `at most 6` and `at most 12` rows, and item (f)'s
recomputations of that table — rather than to the medians and percentiles
beside them, which are measurements. Each of those probabilities is exact only
for a model in which the hashes involved are independent and uniform. They are
not independent: each is a deterministic function of a single per-process seed.
So read each of them as exact for the model, an estimate for the run, and an
upper bound wherever it sums non-disjoint events. What these figures are is
computed rather than sampled: finite exact summations over rationals, binomial
tails here, so no sampling error enters and "closed form" is not the right
phrase for them either.

### The same conflation elsewhere in this document, reported and left standing

Section 5 carries it in a milder form, and these sites are named rather than
corrected because each states its method beside the figure. Its **Phase 2
threshold** paragraph says "The false-failure rate is the binomial right tail
unioned over the buckets"; the threshold table's second column is headed "false
failure (flake)"; and **Why 10 rather than a tighter bound** says choosing 10
"buys a flake rate around 1 in 12 billion". In each the quantity is a union
over non-disjoint per-bucket events, so it bounds the rate rather than being
it.

The figures there are right. Recomputed the same way: `at most 6` is
3.4750078e-05 with reciprocal 28,776.91, and `at most 12` is 5.573419e-14 —
matching the threshold table's own `at most 6` and `at most 12` rows, and item
(f)'s corrected row, at the precision each of those prints.

### Why this was worth an amendment rather than a note

The eleven-figure form is the one that propagated: into the successor
increment's plan, into a briefing file, and from there into a tracked doc
comment, where a review caught it. The more precise-looking figure was the
wrong one, and its apparent precision is what made it worth copying. Leaving a
known-wrong figure labelled "the exact" value in the root record is how the
next increment copies it again — this derivation has now produced a wrong
published figure twice.

## 1. What is unpinned today

`nova_rt_str_hash` is seeded FNV-1a followed by splitmix64's finalizer. The guards
it has today:

- `tests/runtime/hash_diffusion.nova` asserts a structural diffusion property —
  every one of 64 buckets is reached, and the largest holds fewer than 64 keys,
  with both error rates derived and disclosed.
- `crates/nova-cli/tests/run_tests.rs`'s `str_hash_seed_varies_across_processes`
  asserts that one string hashes differently in two processes.

Neither executes the property the design exists to provide. Diffusion is a
statement about one hash function's output spread; a differing hash value is a
statement that the seed is live. **Neither shows that a key set built to collide
stops colliding when the seed changes**, which is what "resists a precomputing
attacker" means, and neither shows that colliding keys concentrate in a bucket
at all.

So the central claim rests on measurements taken outside the repository, in a
throwaway harness, recorded in prose. That is weaker than an assertion the suite
runs, and this project has repeatedly found prose claims that no test executed.

## 2. A correction this design must carry

The merged plan for the seeding increment says the resistance property "is not
test-assertable: building a colliding set requires the seed the design is trying
to keep unknown", and that plan's own amendment then says a fixture can "recover
the seed, construct a colliding key set, and assert the degradation".

**The amendment's conclusion is right and its mechanism is wrong.** Recovering
the seed does not let a Nova fixture predict a hash, because computing FNV-1a
over a key needs that key's bytes and `std/core` records that "`String` has no
length, indexing or iteration, so Nova cannot walk its bytes". The end-to-end
prediction that proved the seed recoverable was performed outside Nova.

The route that works needs no seed at all: **generate candidate keys, call
`.hash()` on each, mask with `& (cap - 1)`, and group.** The runtime does the
hashing; the fixture searches rather than derives. This spec supersedes the
amendment's mechanism and keeps its conclusion.

## 3. What this pins, and what it does not

Pinned by execution after this increment:

- A key set concentrating many keys in one bucket **can be found from ordinary
  Nova code** in one process. This is an adaptive attack succeeding, so the
  limit the records already disclose becomes demonstrated rather than asserted.
- That same set, re-hashed in a **fresh process**, spreads near-ideally. This is
  precomputation resistance, executing.

Not changed, and the records must keep saying so:

- The gate claim's **scope is unchanged**: claimable for string-keyed maps
  against a precomputing attacker, not against an adaptive one, and not for
  `Int`, `Bool` or `Char` keys because `mix64` is unseeded. What changes is the
  claim's **evidence**, from external measurement to an executing assertion.
  Nothing here licenses widening it.
  [Forward marker, 2026-08-29: the exclusion of `Int`, `Bool` and `Char` keys is
  now false. It is closed by the `seeded-mix64` increment, and
  `docs/adr/0005-mutable-receivers-and-one-shot-hash.md`'s 2026-08-28 amendment
  is the governing record. Those three impls compute
  `mix64(key ^ int_hash_seed())`, XORing a per-process seed into `mix64`'s
  input, so such a key's bucket is a function of the key and of the process's
  seed, and the gate claim covers those keys on the same precomputing-attacker
  terms as string keys. The exclusion of the adaptive adversary, in this same
  sentence, is unchanged, and so is the refusal of a cryptographic claim below.
  The wording here is left byte-identical and superseded by this marker rather
  than edited. The next bullet is **not** marked, because it is still true: the
  seeding sits in the impls that call `mix64`, not in `mix64`.]
- `mix64` stays unseeded and untouched.
- The hash stays non-cryptographic. "FNV-1a is not collision-resistant" stays
  true.

## 4. Design

A single Rust test in `crates/nova-cli/tests/run_tests.rs`, orchestrating two
Nova programs in two processes. **No golden `.stdout` file.**

The absent golden is forced, not stylistic: both phases print seed-dependent
output — which keys collide differs every run — so a golden could pin only a
boolean and would break on the varying lines. It also keeps this increment clear
of the standing hazard that no golden may pin `Map` or `Set` iteration order.

### Phase 1 — find a colliding set

A Nova program scans candidate keys `"k0"` through `"k3999"`, computes
`key.hash() & 63` for each, groups by bucket, and stops when some bucket holds
32 keys. It prints:

- the 32 candidate **indices**, space separated, on one line;
- a line stating whether the search succeeded.

Indices rather than the keys themselves, because the keys are
`"k" + index` by construction, so an index transfers the key exactly and keeps
the generated phase-2 source small.

If no bucket reaches 32 within the budget, the program says so and phase 2 is
not attempted; the Rust test fails with that output attached. A silent empty set
must be impossible.

### Phase 2 — show the set no longer collides

The Rust test parses phase 1's indices, writes a **second** Nova program with
those indices inlined as an array literal, and compiles and runs it as a
**separate process**, which therefore draws a fresh seed. That program
re-derives `"k${i}"` for each index, hashes them, buckets them at the same
capacity, and prints the largest bucket count.

Assertion: **the largest bucket holds at most 10 of the 32 keys.**

### Why one process cannot do both

Nova cannot evaluate the hash for a seed other than its own, for the same reason
it cannot compute the hash at all: no byte access to a `String`. There is no way
to ask "what would this key hash to under a different seed" from inside the
language. A second real process is what supplies the second seed.

## 5. Derivations

Every number here was measured or computed, with the method stated so it can be
re-derived rather than trusted. Simulations use the real seeded, finalized hash
over the real key strings.

**Phase 1 budget.** Over 200 random seeds, a bucket reached 32 keys after a
median of 1319 candidates, 1463 at the 95th percentile, and 1508 at the worst.
A budget of 4000 is about 2.6 times the observed worst case. The search
succeeded for every one of those 200 seeds.

**Phase 2 threshold.** With 32 keys falling into 64 buckets the mean occupancy
is 0.5. The false-failure rate is the binomial right tail unioned over the
buckets:

| threshold | false failure (flake) | margin against a dead seed |
|---|---|---|
| at most 6 | 3.475e-05, about 1 run in 28,777 | needs 7 in one bucket to pass wrongly |
| **at most 10** | **8.269e-11, about 1 run in 12,093,819,544** | needs 11; a dead seed yields 32 |
| at most 12 | 5.573e-14 | needs 13 |

Simulating 3000 independent seed pairs end to end, the phase-2 largest bucket
never exceeded 5 and sat at 4 or below in 99 per cent of pairs.

**Why 10 rather than a tighter bound.** Both error rates are simultaneously
negligible here, which is not the situation the diffusion fixture faced. If the
seeding is dead — both processes sharing a basis — phase 2 sees 32 keys in one
bucket, so every threshold below 32 fails that mutation. Choosing 10 buys a
flake rate around 1 in 12 billion while leaving a 22-key margin before a broken
seed could pass. Tightening toward the observed maximum of 5 would trade a large
amount of flake safety for margin that is already enormous.

## 6. Mutations that must fail

Each states the expected failure, and the plan must report the observed one.

1. **Pin the seed to a constant basis** in `nova_rt_str_hash`. Phase 2 must fail:
   the set found in phase 1 still collides, so the largest bucket is 32, far
   above 10. This is the mutation the increment exists to catch.
2. **Break phase 1's grouping** so it reports a bucket that does not hold 32
   keys. Phase 1's own success line must go false and the test must fail rather
   than proceeding to a meaningless phase 2.
3. **Reduce phase 1's candidate budget** far below the measured requirement, for
   example to 100. The search must report not-found and fail loudly, proving the
   not-found path is real rather than decorative.
4. **Drop the finalizer, keeping the seed.** Phase 2 is expected to still pass,
   because a seed change alone moves these keys; this mutation is included to
   record that this test does **not** cover the finalizer, which is what
   `hash_diffusion` covers. Reporting an expected pass is the point.

Mutation 4 exists to prevent a false claim of coverage. Whoever runs it should
record that this test and `hash_diffusion` answer different questions.

## 7. Records to amend

- `nova-spec/20-STDLIB.md` section 7 and `docs/adr/0018-std-json-scope-and-build-order.md`:
  the gate claim keeps its scope and gains a pointer to the executing test, so a
  reader can see the evidence rather than only the argument. Do not widen it.
- The merged plan's amendment, whose stated mechanism this spec corrects. The
  plan is a dated record; it gets a further dated note, not an edit in place.
- `CHANGELOG.md` under `[Unreleased]`.
- ADR 0005: a pointer only if the amendment there asserts something this
  increment changes. Check rather than assume; do not add an amendment that
  merely announces a test exists.

## 8. Hazards

- **`include_str!` embeds every `std/*/lib.nova` in the compiler.** This
  increment is not expected to touch `std`, but if it does, a stale binary
  measures the old text and reports a false pass. Build before testing
  regardless.
- **Never author Nova string escapes through a quoted shell heredoc.** A
  backslash has been silently consumed twice on this project. Phase 2's
  generated source is written from Rust, which avoids the shell entirely; any
  scratch fixture written by hand must use a file-writing tool.
- **Two live flakes, neither to be touched here.** The `0xc0000005` family hits
  a test that builds and runs a Nova binary, a different test each run, and its
  cause is unproven with every shared-fixture-path hypothesis eliminated. And
  `net::tests::connecting_to_a_closed_port_is_connection_refused` fails when a
  concurrent binder takes the ephemeral port `dead_addr()` frees. If either
  fires, re-run, say so, attribute no cause, and fix nothing.
- **Every fixture path must be unique per process**, so the generated phase-2
  source and its build directory carry a process id.
- This test builds and runs Nova binaries twice, so it joins the population the
  `0xc0000005` flake draws from. That is a cost worth stating, not a reason to
  avoid the test.

## 9. Out of scope

- Seeding `mix64`, which would extend resistance to `Int`, `Bool` and `Char`
  keys. `tests/runtime/hash.stdout` pins `mix64`'s histograms, so that is a
  larger change with its own records.
- Removing the seed's reachability from Nova. The channel follows from ADR 0005's
  one-shot `Hash` returning a plain `Int`; narrowing it is an ADR-level question.
- Fixing either flake.
- Any timing or throughput assertion. This suite has a standing rule against
  timing-flaky assertions: assert orderings and counts a correct implementation
  forces, never durations.

## 10. Success criteria

- A test exists that fails when the hash basis is pinned to a constant, and
  passes otherwise, with the failure observed rather than predicted.
- Phase 1's not-found path is shown to fail loudly, by mutation.
- The suite total moves from 1075 to 1076 on Windows, the increase landing
  entirely in the `nova-cli` `run_tests` target, with every other target's
  count unchanged when each of the 44 `test result:` lines is summed. The
  equivalent gate counts hold on the ubuntu and macos legs.
- Both derived rates in section 5 appear in the test's own documentation, so the
  next reader sees the threshold's justification without leaving the file.
- The records in section 7 state the same scope they state today, with evidence
  added and nothing widened.

## 11. Correction, 2026-08-27 — statements above that are wrong

Written after the test shipped, at the review that closed the increment, and
extended by the fix round that closed it for good; items (c), (e) and (f) carry
that later material and each says where it begins. **The body above is left as
written**, in the convention the previous increment used: a design record keeps
its original wording so the evidence of the error survives. Read the sections
named below only with this one beside them.

**There is one exception to "left as written", and it is item (f).** Two
arithmetic figures in section 5's threshold table are corrected in the table
itself, because a reference number is used rather than read: a reader who
copies a wrong one carries the error forward instead of noticing it. Item (f)
records what those figures were, so the evidence survives there instead of in
the table. Nothing else above is edited.

### a. Section 2's verdict on the merged plan's amendment is retracted

**The statement being retracted belongs to this spec. The statement being
restored belongs to the merged plan.** Section 2 above says of that plan's
amendment: "**The amendment's conclusion is right and its mechanism is wrong.**
Recovering the seed does not let a Nova fixture predict a hash, because
computing FNV-1a over a key needs that key's bytes and `std/core` records that
'`String` has no length, indexing or iteration, so Nova cannot walk its
bytes'". **The "mechanism is wrong" verdict is retracted, and so is the reason
given for it. The amendment's mechanism was viable, and the amendment was right
on both halves rather than on one.**

`std/bytes` exposes `bytes_from_string`, `byte_at` and `to_ints`, reachable
from a user module with no import — the registered fixture
`tests/runtime/bytes_basics.nova` calls `bytes_from_string("hi")` as its first
statement and is checked against a golden — and `std/strings` gives `String` a
`len`, `chars`, `char_at` and `slice`. Nova can obtain a key's bytes. The
arithmetic a prediction needs also already runs in Nova: `std/core`'s `mix64`
computes splitmix64's finalizer in ordinary Nova `Int` shift, xor and multiply.

`std/core`'s quoted sentence sits in a middle position, and both halves of that
need stating so neither is read off the other. It is **not** the false one this
item retracts; the retracted statement is section 2's, quoted above. And it is
**not** accurate today either: it was true when written and has since gone
**stale**. `std/core` itself can reach a string's bytes now, by the same route
it already uses for a string's characters — `str_chars` and
`bytes_from_string_intrinsic` are both `Builtin::STD_ONLY`, so both are seeded
into every std module's scope, and `std/core` already calls `str_chars` at
`impl Debug for String` above a comment recording that it is "already visible
here with no import".

**An earlier draft of this correction defended that sentence as *scoped* to
`std/core`'s position in `STD_MODULES`. That defence is withdrawn.** The
positional facts were right; the inference from them was not.
`crates/nova-resolver/src/lib.rs` documents the array's order as significant
"only in that it fixes module indices", and `import_std_module` binds each std
module's public names into every *other* module's scope, so position does not
gate the capability; whether it ever did is a question for that resolver code
and its history, not for this sentence. Section 2's error was therefore reading
a stale sentence
as a current one, not misreading a scope — and this item's conclusion depends
on neither reading, resting only on the byte surface existing.

The same wording appears elsewhere in the tree — at
`crates/nova-resolver/src/lib.rs`'s doc comments on `Builtin::StrHash` and
`Builtin::StrLenChars` among them — stale there in the same way; this increment
changed no code, so that is flagged rather than fixed. The merged plan carries
the matching dated note.

### b. Section 4's "Why one process cannot do both" gives a false reason

That subsection reads "Nova cannot evaluate the hash for a seed other than its
own, for the same reason it cannot compute the hash at all: no byte access to a
`String`." **The reason is false for the same reason item (a) gives, and the
two-process design nevertheless stands.** It stands on a different ground: both
phases exercise the runtime's real hash, whereas computing the hash in Nova
would pin a second copy of the algorithm and fail for the wrong reason if the
two ever diverged. The shipped test's doc comment states it that way. So the
design was preferable, not forced.

### c. Section 3 overstates what phase 1 demonstrates

Section 3 says of the searching phase: "This is an adaptive attack succeeding,
so the limit the records already disclose becomes demonstrated rather than
asserted." **That is too strong and is narrowed here.** The limit those records
disclose is stated as "an adversary who can observe timing and adapt"
(`nova-spec/20-STDLIB.md` section 7, `docs/adr/0018-std-json-scope-and-build-order.md`
section 8). Phase 1 observes no timing: it calls `.hash()` directly, which
requires code running inside the target process and is a stronger capability
than observing timing from outside. **What phase 1 does pin is that given
direct access to the hash, a colliding key set is cheap to find. It does not
pin the timing adversary, and the records were amended to say so rather than to
claim otherwise.** The shipped test's doc comment carries the same overstated
framing — "so the limit the records disclose is demonstrated here too" — and
this increment changed no code, so correcting that comment is outstanding.

**Closed, at the fix round, later than the paragraph above: that comment WAS
corrected on this branch, and the sentence ending the paragraph is now false in
each of its three clauses — the comment no longer carries the framing, this
increment did change that code, and nothing is outstanding.** It was corrected
at the commit whose subject is "stop the hashdos
test's comment claiming it demonstrates the timing limit", and the shipped
`hashdos_precomputed_key_set_does_not_survive_a_new_process` no longer carries
the overstated framing — read the test rather than the sentence above it. The
"this increment changed no code" premise stopped holding at that commit: the
increment changed no *product* code and still does not, but the fix round
lifted that premise for this one comment, so the conclusion drawn from it fell
with it. **The item is kept rather than deleted** because the overclaim's
history is the useful part: the framing reached section 3 above, a shipped doc
comment, and Task 2 Step 2 of this increment's plan, which still instructs an
executor to write it into `nova-spec/20-STDLIB.md` and
`docs/adr/0018-std-json-scope-and-build-order.md`. Those two records do not
carry it — the executor declined that instruction — and the plan's head note
now says not to follow it. Nothing in this item is outstanding work, and a
reader who arrives to fix that doc comment will find it already fixed.

### d. Section 6's mutation-4 expectation is wrong

Mutation 4 reads "Phase 2 is expected to still pass, because a seed change
alone moves these keys". **Dropping the finalizer is only partly invisible to
this test, not wholly.** Without the finalizer the six bucket-selecting bits
depend on `seed & 63` alone, so the layout space collapses from 2^64 to the
residue pairs; enumerated against the real hash over the real key strings, a
minority of those pairs fail the bound and a smaller minority print the same
largest-bucket value a dead seed produces. Figures and the enumeration method
are in the shipped test's doc comment and are not duplicated here. **The usable
statement is the one that mutation existed to produce, and it survives in
stronger form: coverage of the finalizer here is partial and unreliable,
`tests/runtime/hash_diffusion.nova` is the deterministic detector, and a
failure of this test must not be read as a dead seed without checking the
finalizer separately.**

A better articulation of why the finalizer is needed also falls out, and is
recorded here because the project has been making the weaker argument: the
standing justification is that bit 0 of an FNV result is bit 0 of the basis
XOR the input bytes' parity. The stronger true statement is that **all six
bucket-selecting bits at capacity 64 are a function of six seed bits.**

### e. Section 7's ADR 0005 item, ruled rather than left open

Section 7 asks for a pointer in `docs/adr/0005-mutable-receivers-and-one-shot-hash.md`
"only if the amendment there asserts something this increment changes."
**Ruled: no amendment. It was checked, not assumed.** That ADR's relevant
wording is its Phase 2.2a disclosure that hashes are not randomized per
process, plus the previous increment's amendment quoting that same sentence.
Neither asserts anything about test-assertability or about precomputation
resistance, so this increment falsifies nothing there, and a note that only
announced a test's existence would be noise in a decision record.

**Reversed, at the fix round, later than the ruling above: ADR 0005 now carries
a dated 2026-08-27 amendment.** Two things above are wrong, and only one of them
was wrong when it was written.

The check behind the ruling looked at four things — test-assertability,
per-process randomisation, precomputation resistance, and the readability of the
seed — and was correct on all four at the time. **What it could not have
examined is the sentence that makes an amendment necessary, because that sentence
was not yet false when the check ran.** ADR 0005's Decision justifies `str_hash`
"because Nova cannot walk a string's bytes: `String` has no length, indexing or
iteration, and is not FFI-safe, so no `extern` can reach it either", and item (a)
above is what falsified the first clause of that, later in this same increment.
Nobody returned to the ruling.

**The reason stated above is separately inaccurate about ADR 0005's contents, and
that half was wrong when written.** Its 2026-08-26 amendment does assert
something about precomputation resistance — that "an attacker who cannot learn
the seed cannot build one offline", closing "Precomputation resistance is the
half that survives" — and it documents the seed-readability channel as well.
The defensible statement is the narrower one: this increment falsifies neither,
and it moves the evidence for the first from prose to an executing assertion.
That corrects the reason, not the outcome; the outcome is reversed on the
separate ground above.

**Keep the sequence, not only the outcome.** A check that examined the right
things and was then overtaken by a finding in its own increment is a different
failure from a check never made, and only the first one is invisible to a reader
who sees the word "checked" and stops there. The amendment in ADR 0005 states
that sequence at itself.

`str_hash` is not thereby unjustified and ADR 0005's decision does not move. It
remains a one-shot hash primitive whose algorithm lives in the runtime, and a
Nova reimplementation would put a second copy of that algorithm — and now of
the per-process seed — in the tree to keep in step with the first. Only the
reason for the builtin is corrected.

### f. Section 5's threshold table carried two wrong figures

**Written at the fix round, later than items (a) to (e), and the one item that
edits the body above.** Both are recomputed with Python's `fractions.Fraction`,
which keeps `64 * P(Bin(32, 1/64) >= k)` an exact rational until the final
rounding:

| figure | was | is | exact value |
|---|---|---|---|
| `at most 12`, the union bound | 5.684e-14 | 5.573e-14 | 5.573419e-14 |
| `at most 10`, the reciprocal | 1 run in 12,093,966,516 | 1 run in 12,093,819,544 | 12,093,819,543.57 |

The two are wrong for different reasons, and only one reason is known. The
`at most 12` union bound was accumulated as a floating-point right tail instead
of an exact rational, which put it about 2 per cent high. **The reciprocal on the
`at most 10` row has no such account, and is the more instructive of the two.**
The `8.269e-11` mantissa beside it is right and is unchanged — the exact value
is 8.26868630e-11 — and the published reciprocal is not the reciprocal of that
mantissa at any rounding of it: 1/8.26869e-11 is 12,093,814,135 and 1/8.269e-11
is 12,093,360,745, and 12,093,966,516 is neither.

**Its origin was subsequently reconstructed and measured, 2026-08-27.** The
derivation script accumulated the tail with an early-exit shortcut, breaking out
of the summation once a term's log-probability fell below a threshold and a few
terms had been taken. That dropped the remaining tail terms, yielding a union of
8.268585816e-11 rather than the exact 8.2686863018e-11 — and the integer part of
its reciprocal is 12,093,966,516, reproducing the published figure exactly. So
the mantissa survived the truncation at four significant figures while the
reciprocal did not, because inverting a small number amplifies a relative error
that rounding had hidden. The lesson generalises past this row: a convergence
shortcut in a tail sum is invisible in the quantity you print and visible in
anything you derive from it. The `at most 6` row is right on both of
its figures (exact 3.4750078e-05, reciprocal 28,776.91), so the defect is not
uniform across the table: it has to be found per figure rather than inferred
from one row to the next.

**Neither corrected figure is the bound the test uses.** The bound in use is
10, an integer literal in the test's own `largest <= 10` assertion, and its
mantissa was already correct. Both wrong values were grepped for across every
tracked file and appear nowhere but the row each occupied and this item's own
record of it, so nothing else carried them forward. The reason to record
this at all is that a derived number is this increment's whole subject: an
increment that replaces an argued security claim with a measured one cannot
leave its own measurements approximate and expect to be believed about the
ones that matter.

**Superseded in part, 2026-08-29.** The eleven-figure form in the paragraph
above — "the exact 8.2686863018e-11" — belongs to the independence model, not
to the union bound this row's label names; that union bound is
8.2686863021e-11. See **Amendment - 2026-08-29 — item (f)'s eleven-figure union
bound**, immediately after this document's header, which also records why the
two figures' reciprocals agree and what that costs a check of one figure
against another derived from it.

### g. Section 8's `dead_addr()` hazard names a refuted mechanism

Section 8 says `net::tests::connecting_to_a_closed_port_is_connection_refused`
"fails when a concurrent binder takes the ephemeral port `dead_addr()` frees",
and this document's hazard list presented that as the flake with a known cause,
in contrast to the `0xc0000005` family.

**Measured 2026-08-28, the in-process mechanism cannot fire.** Ephemeral ports
are issued sequentially from a system-wide cursor (300 binds, 300 distinct
ports, span 309) and a freed port is not reissued for about 13,759 binds, while
the window here is microseconds wide; a direct probe found 0 steals in 12,000
attempts at 0, 4 and 16 concurrent in-process binders.

Cross-process theft is untested rather than refuted -- that probe used a
one-second timeout against a 2.0-second refusal latency and returned zeros for
both outcomes, so it never ran. The flake is real; **its cause is unknown
again**, so the contrast this section drew with the `0xc0000005` family no
longer holds and both should be treated the same way: instrument the next
occurrence rather than fix ahead of a mechanism.

