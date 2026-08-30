# `Map` HashDoS: a keyed, seeded `str_hash`

**Status:** approved 2026-08-26. Design only.

**Goal.** Make a `Map<String, _>` resistant to adversarially chosen keys, so that Phase 2's
throughput gate on untrusted input becomes claimable for string-keyed maps — which is the exposure
`std/json` hands to whatever serves it.

**Base.** `main` == `origin/main` == `ba5ab77`, 566 commits, 0 merge commits, 1073 tests passing
(8 ignored) across 44 targets, clean tree, tagged `v0.2.0-alpha.1`.

---

## 1. Why this increment exists, and what ADR 0005 actually permits

The merged tree says of itself, in `nova-spec/20-STDLIB.md` section 7 and in ADR 0018, that **Phase
2's throughput gate is not claimable on untrusted input.** The reason is not `std/json`, which was
just hardened: it is that `stringify`'s `Object` arm performs one `Map` lookup per member and `parse`
one insert per key, `Map` selects buckets with `k.hash() & (cap - 1)` and probes linearly, and
`impl Hash for String` is `str_hash` — FNV-1a, which the runtime's own doc comment calls "*not*
collision-resistant and must not be used for anything security-sensitive". Adversarially chosen
object keys make both directions quadratic in the number of keys, regardless of the depth cap or the
linear buffer that increment added.

**ADR 0005 is the governing record and it contains a tension that had to be resolved before
designing.** It discloses the exposure and says a seeded hasher "is a `Hasher`-shaped question, i.e.
it is the migration below" — the accumulating `fn hash<H: Hasher>(self, h: H)` shape it records as a
breaking change with a deprecation cycle. But its closing paragraph says the opposite of what that
sentence implies:

> Cheaper changes that this decision does *not* foreclose, because none of them touch `Hash`'s
> signature: replacing FNV-1a inside `nova_rt_str_hash`, replacing `mix64`'s constants or rounds,
> and seeding either from a process-start value.

Both are true of different things. A **swappable seeded `Hasher` object** — per-map hasher choice —
needs the migration. **Replacing or seeding the one-shot function** does not, and the ADR says so
explicitly. This increment takes the second road, so `pub trait Hash { fn hash(self) -> Int }` is
untouched, there is no migration, no deprecation cycle, and no user-visible break.

ADR 0005 needs a dated amendment recording which of its two sentences governs, because the broader
one nearly produced the wrong increment.

---

## 2. What the measurement established, and why the obvious design is not enough

The first design considered was the cheap one the ADR names: seed FNV-1a's offset basis from a
process-start value, a one-line change. **It was measured and rejected.**

Method: FNV-1a and the splitmix64 finalizer were reimplemented exactly (prime `0x100000001b3`,
shipped basis `0xcbf29ce484222325`, finalizer constants `0xbf58476d1ce4e5b9` and
`0x94d049bb133111eb`) and run against key sets drawn from lowercase ASCII. Bucket figures use the
full 64-bit hash masked with `& (cap - 1)`, exactly as `Map` does. **A constructed attack** means N
keys selected to share a bucket under the shipped hash at the capacity a `Map` holding N keys
actually has, `Map` growing at 3/4 load with a power-of-two capacity.

| constructed attack | shipped FNV | seeded FNV only | `mix64(seeded FNV)` | ideal for a keyed hash |
|---|---|---|---|---|
| cap 64, N = 48 | 48 in one bucket | **30, 7, 6, 16** | **3, 4, 4, 3** | ~2 |
| cap 256, N = 192 | 192 in one bucket | **28, 7, 9, 21** | **3, 5, 4, 4** | ~2 |

Each of the four figures is a different fresh seed. Seeding alone left **30 of 48** keys colliding
under one seed in four — an improvement, but three to fifteen times worse than a keyed hash, and not
something on which the gate could honestly be claimed.

The cause is structural, not a matter of seed quality. `Map` masks the **low** bits, and FNV-1a's low
bits barely depend on the basis. Bit 0 of the output is `bit0(basis)` XOR the parity of the input
bytes' low bits, so changing the basis flips bit 0 **identically for both keys in a colliding pair**,
leaving that pair's collision intact. Measured on the diffusion this predicts:

| | seeded FNV only | `mix64(seeded FNV)` | ideal |
|---|---|---|---|
| keys changing bucket, cap 8 | 66.7% | **87.3%** | 87.5% |
| keys changing bucket, cap 16 | 66.7% | **93.5%** | 93.8% |

Independently: of 500 pairs colliding in the low 16 bits under the shipped basis, **9 still collided
under a different basis**, where chance alone predicts 0.01. That test reduces the output to its low
16 bits, which is legitimate here precisely because `Map` uses low bits — the reduction is the thing
under attack, not an artefact of it.

**So the finalizer is load-bearing, not decoration.** A constructed set differs in high bits and
agrees in low ones; the finalizer spreads high into low, which is exactly the axis the seed cannot
reach.

---

## 3. The change

`nova_rt_str_hash` becomes, in `crates/nova-runtime/src/lib.rs`:

1. FNV-1a over the string's bytes as today, but with the offset basis replaced by a **per-process
   seed**.
2. The splitmix64 finalizer applied to the result.

The seed is a `OnceLock<u64>` initialised on first use from `std::collections::hash_map::RandomState`
— `RandomState::new().build_hasher().finish()`. That draws OS entropy through `std`, needs **no new
dependency**, holds at MSRV 1.78, and is how Rust's own `HashMap` seeds itself. The runtime already
links it: `crates/nova-runtime/src/file.rs` builds a `std` `HashMap`.

`OnceLock` and not a thread-local: the seed must be **stable for the whole process**, because a `Map`
built under one value and probed under another finds nothing. Per-thread seeds would corrupt a `Map`
shared across threads, and per-call seeding would corrupt every `Map`.

**That is the entire change.** Specifically not in it:

- **No new intrinsic**, so the 12-site checklist is not paid and `STD_ONLY`, `RtFunc` and
  `STD_MODULES` are untouched.
- **No trait change.** `fn hash(self) -> Int` keeps its signature, and every `impl Hash` keeps its
  own — none is edited, whatever the set grows to.
- **No Nova-side diff at all.** `std/core`, `std/collections` and `std/json` are not edited.
- **No entropy exposed to Nova.** The seed is runtime-internal; no Nova program can read it. Note
  that Nova can already reach libc through `extern "C"` — `extern "C" { fn rand() -> Int }` compiles
  and runs — so this increment neither grants nor withholds that; it simply does not add a route.

### 3.1 Why no compile-time hazard exists

A hash computed at compile time and baked into output would break under a per-process seed, since a
`nova build` binary would carry the build process's seed and probe with the run process's. Verified
that this cannot happen, three ways: `Builtin::StrHash` lowers through
`Lowering::Runtime(RtFunc::StrHash)`, i.e. to a call and never to a value; **no const-eval or
const-fold pass exists anywhere under `crates/`**; and measured, the same literal hashes identically
across two separate processes today, which is the behaviour that changes.

---

## 4. What stays exposed, disclosed rather than implied

**`mix64` is untouched, so `Int`, `Bool` and `Char` keys remain attackable.** That is a decision, not
an oversight, and it rests on evidence rather than convenience: `tests/runtime/hash.stdout` pins
`mix64`'s bucket histograms, its complement-collision count, its 100-of-256 spread figure and its
canonical splitmix64 values — and ADR 0005 records that those histograms "were computed independently
from splitmix64's finalizer rather than recorded from a run". They are a specification of the low-bit
spreading that `Map`'s masking depends on. Seeding `mix64` would trade that specification for a
partial win on a path JSON does not take, since JSON object keys are strings.

**The result is not cryptographic.** Seeded, finalized FNV-1a is not SipHash. What it buys is
precomputation resistance — an attacker who cannot learn the seed cannot build a colliding set
offline — plus the diffusion measured in section 2. It does not defeat an adversary who can observe
timing and adapt. ADR 0005's sentence "FNV-1a is not collision-resistant" stays true after this
change, and no record should imply otherwise.

**So the gate claim narrows rather than disappears.** After this increment, Phase 2's throughput gate
on untrusted input is claimable **for string-keyed maps against a precomputing attacker**. Any record
saying more than that is wrong.

---

## 5. Testing

**What breaks by design.** `tests/runtime/json_object_forged_map.nova` hand-builds a `Map` at
capacity 4 assuming `"a"` hashes to slot 0 and `"b"` to slot 1, and asserts those preconditions
explicitly so that a hash change fails loudly rather than silently stops exercising the separator
placement it exists to pin. It must be rewritten to compute both slots at run time from `.hash()`
and place the entries accordingly — which is what its own header anticipated.

**What must survive untouched, and is expected to.** `hash.stdout`'s numeric assertions are all
`mix64` properties, so seeding `str_hash` cannot move them; its `String` lines assert only
`same`/`diff` booleans. `map_keys.stdout` pins counts and per-key lookups, not order.
`collections.stdout` pins capacity growth. `json_stringify.nova` deliberately uses single-keyed
objects because `Map::keys()` returns table order. Each of these is a prediction to **verify by
running**, not to assume.

**Two properties are assertable despite a per-process seed:**

- **Cross-process variation.** A Rust-side test builds one binary with `nova build` and runs it
  twice, asserting the two hash outputs differ. This is the test written *for* the property that the
  seed is live, and it catches a seed accidentally fixed to a constant. Whether anything else catches
  that too is a question for running the mutation, not for asserting here — designed-for is not
  exclusively-capable, and on this project every claim that one test was the only thing catching
  something has been measured false.
- **Within-process stability.** Already covered by `hash.nova`'s `same=` lines, which must keep
  passing.

**Diffusion is assertable as a seed-independent statistic.** A fixture hashes many keys into a known
number of buckets and asserts the largest bucket stays under a generous bound — a property that holds
for every seed, unlike any particular histogram. The bound must be loose enough that no seed fails it
and tight enough that the unfinalized hash would: section 2's numbers give the gap to aim at.

**What no test asserts, said plainly.** The resistance property itself is not test-assertable, because
building a colliding set requires the seed the test is trying to prove unknowable. It rests on section
2's analysis and its numbers, recorded with their method so a later reader can re-run them. This is
the same honesty the previous increment applied to asymptotics, and for the same reason.

**Mutations to run and report:** fix the seed to the old constant basis (the diffusion fixture and the
cross-process test must both fail); drop the finalizer, keeping the seed (the diffusion fixture must
fail — this is the mutation that distinguishes this design from the one section 2 rejected); reseed
per call rather than once (every `Map` fixture must fail).

---

## 6. Records to amend

- **ADR 0005** — a dated amendment recording which of its two sentences governs: seeding or replacing
  the one-shot function is permitted by its own closing paragraph, and only a swappable `Hasher`
  object needs the migration. Its Phase 2.2a disclosure that hashes are not randomized per process
  becomes false for `String` and stays true for `mix64`.
  [Forward marker, 2026-08-30: the last clause describes an amendment this
  increment went on to make correctly, so it is an accurate record rather than a
  false claim — but the half it left standing has since fallen. The 2026-08-28
  `seeded-mix64` increment made the disclosure false for `Int`, `Bool` and `Char`
  as well, those impls now computing `mix64(key ^ int_hash_seed())`; `mix64`
  itself remains unseeded and module-private, which is the distinction that
  sentence turned on and which is why "stays true for `mix64`" is still true of
  the function while no longer true of the key types. The earlier decision to
  leave those three attackable was right when it was written and this increment
  took the other side of it. **ADR 0005's 2026-08-28 amendment governs.** The
  wording here is left byte-identical and superseded by this marker rather than
  edited.]
- **`crates/nova-resolver/src/lib.rs`** — the doc comment on `Builtin::StrHash` describes `str_hash`
  as "FNV-1a over the string's bytes", which this change falsifies.
- **`crates/nova-runtime/src/lib.rs`** — `nova_rt_str_hash`'s own doc comment, including its
  "not collision-resistant" note, which stays true but needs the seed and finalizer described.
- **`nova-spec/20-STDLIB.md` section 7 and ADR 0018** — their "not claimable" statements need
  narrowing to what section 4 above permits, not deleting.
- **`std/collections/lib.nova`** — `Map`'s header, on iteration order now varying per process.
- **`CHANGELOG.md`** under `[Unreleased]`.

A new ADR is **not** needed: this is a change permitted by ADR 0005's own terms, recorded as an
amendment to it rather than as a decision overriding it.

---

## 7. Method notes for whoever implements this

- `std/*/lib.nova` is `include_str!`'d into the compiler, so a stale `nova` binary exercises the old
  std and reports a **false pass**. This increment edits no `.nova` library file, but it does edit the
  runtime, so the same rebuild-first discipline applies to every measurement.
- **grep is line-oriented and a miss is not evidence of absence.** Sweep prose with whitespace-
  tolerant patterns that also normalise `//` and `>` gutters. A line-oriented sweep shipped a false
  completion claim on the previous branch.
- Prefer a roster with no count. A corrected number is usually the wrong fix. No ordinals or closed
  worlds over `std`, the runtime or the workspace. Quote retracted wording inside the retraction, and
  remember that a correct retraction therefore still contains it.
- This spec and the plan that follows sit in **no per-commit review's scanned set**. Byte-scan them
  when written, and re-scan the whole branch-changed set as one population at final review.

---

## 8. Amendment, 2026-08-26 — statements above that what shipped falsified

Added after implementation, at the fix round that closed the increment. **The body above is left as
written**, because it is the authority the implementation argued from and editing it to match what
shipped would erase the evidence that it was wrong. Each item below names the section, quotes the
statement, and gives the correction.

1. **§3 — "No Nova-side diff at all. `std/core`, `std/collections` and `std/json` are not edited",
   and §7's "This increment edits no `.nova` library file".** Both false. The increment edits
   `std/collections/lib.nova`, `std/core/lib.nova` and `std/json/lib.nova`. Every one of those edits
   is a **comment** — the per-process seed recorded at `Map`, at `impl Hash for String`, and at
   `std/json`'s object paths — so the source-compatibility conclusion the sentence was serving
   stands, and the accurate form of it is "no Nova-side source changed *behaviour*". The
   rebuild-first discipline §7 derives from `include_str!` therefore applies with more force than
   §7 expected, not less: the increment does edit files that are compiled into the binary its own
   fixtures run against.

2. **§3 — the seed expression is incomplete.** It reads
   `RandomState::new().build_hasher().finish()`. The shipped `str_hash_seed` writes one `u64` in
   between: `build_hasher()`, then `h.write_u64(0)`, then `h.finish()`. Without that write,
   `finish()` would hash an empty byte stream and so would still return a function of
   `RandomState`'s random keys, i.e. still a per-process value — so this is a difference between
   what was specified and what shipped rather than a defect in either, but a reader reimplementing
   the seed from §3 alone would compute a different number.

3. **§5's mutation-1 prediction is wrong.** It reads "fix the seed to the old constant basis (the
   diffusion fixture and the cross-process test must both fail)". Only the cross-process test fails.
   With the basis pinned to the old constant and the finalizer kept, `hash_diffusion.nova` **passes**
   — the finalizer is what that fixture discriminates on, and it is still present. The plan written
   from this spec corrected the prediction; this text did not, so the two disagreed until now. The
   mutation that must fail is §5's second one, dropping the finalizer while keeping the seed.

4. **§3 — "No entropy exposed to Nova. The seed is runtime-internal; no Nova program can read it."**
   False, and this is the item worth reading twice. `("").hash()` returns splitmix64's finalizer
   applied to the raw seed, because FNV's loop body never runs on an empty string, and that
   finalizer is a bijection with a published inverse — so **one call from ordinary Nova code
   recovers the seed exactly**. Measured on the implementing tree: one `("").hash()` inverted to a
   seed, and that seed then predicted two further hashes from the same process exactly; four
   processes gave four different values, so the seed is live. What §4 claims still holds on both
   halves. **Precomputation resistance stands**: the seed is per process, so a colliding key set
   cannot be built offline against a process that has not started, and recovering the seed requires
   a hash *from the running process*. **The adaptive case is now concrete rather than a category**:
   an attacker who can obtain one `String` hash from the process can recover its seed and construct
   collisions for that process. §4's "does not defeat an adversary who can observe timing and adapt"
   is the statement this is the mechanism for. The channel is ADR 0005's one-shot
   `fn hash(self) -> Int` returning a whole 64-bit result in one call, which predates this
   increment: seeding did not open the channel, it made the value behind it worth recovering.
   Narrowing that signature is an ADR-level question and is not proposed here.

5. **§5's "What no test asserts, said plainly" — appended 2026-08-27 from the
   `hashdos-resistance-test` increment, later than the four items above and by a different hand.**
   **Verdict before the quotation: that sentence is false on both halves, and the increment just
   named is what falsifies it.** It reads: "The resistance property itself is not test-assertable,
   because building a colliding set requires the seed the test is trying to prove unknowable." The
   first half, "not test-assertable", is **false**:
   `hashdos_precomputed_key_set_does_not_survive_a_new_process` in
   `crates/nova-cli/tests/run_tests.rs` asserts exactly that property, building a colliding set in
   one process and showing it spread in a fresh one. The second half, the reason, is **false
   independently and was false when written**: building a colliding set needs no seed at all,
   because the runtime hashes on request, so a fixture **searches** with `.hash()` where this
   sentence assumed it would have to **derive**. The full argument — including that Nova *can* walk
   a `String`'s bytes through `std/bytes` and `std/strings`, so even the deriving route was open —
   is in `docs/superpowers/specs/2026-08-27-hashdos-resistance-test-design.md` section 11 and is
   not restated here. §5's neighbouring ruling that diffusion is assertable as a seed-independent
   statistic stands, as does its warning against claiming any one test is the only thing catching
   something.

None of these changes §4's gate ruling, which the implementation kept: the throughput gate is
claimable for string-keyed maps **against a precomputing attacker**, and not more.
