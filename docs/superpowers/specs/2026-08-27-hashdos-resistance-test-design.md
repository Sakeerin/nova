# Pinning HashDoS resistance with an executing test

**Status:** design, approved 2026-08-27
**Base:** `main` at `515145b` (584 commits, no merge commits, 1075 passed / 0 failed / 8 ignored across 44 targets on Windows; CI gate per leg: ubuntu 1067/0/1, macos 1068/0/0, windows 1075/0/8)
**Governs:** ADR 0005 (one-shot `Hash`), ADR 0018 section 4, `nova-spec/20-STDLIB.md` section 7

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
| **at most 10** | **8.269e-11, about 1 run in 12,093,966,516** | needs 11; a dead seed yields 32 |
| at most 12 | 5.684e-14 | needs 13 |

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

Written after the test shipped, at the review that closed the increment. **The
body above is left as written**, in the convention the previous increment used:
a design record keeps its original wording so the evidence of the error
survives. Read the sections named below only with this one beside them.

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

`std/core`'s quoted sentence is **not** the false one. It is true where it
stands, being scoped to `std/core`'s position: `$std.core` is the first
`STD_MODULES` entry and `$std.bytes` and `$std.strings` come after it, so *that
module* cannot walk bytes, which is why `str_hash` has to be a builtin for it.
Section 2 read a statement about one module's position as a statement about the
language. The same wording appears without that scoping at
`crates/nova-resolver/src/lib.rs`'s doc comments on `Builtin::StrHash` and
`Builtin::StrLenChars`; this increment changed no code, so that is flagged
rather than fixed. The merged plan carries the matching dated note.

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
