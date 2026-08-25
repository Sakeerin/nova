# `std/json` hardening: a depth cap, an iterative `stringify`, and one buffer

**Status:** approved 2026-08-25. Design only.

**Goal.** Remove the two costs ADR 0018 section 8 recorded as known and deliberate — `parse`
aborting the process on a deeply nested input, and the interpolation accumulators rebuilding their
output once per element — and disclose a third cost, found while designing this one, that neither
fix touches.

**Base.** `main` == `origin/main` == `a73d5be`, 553 commits, 0 merge commits, 1066 tests passing
(8 ignored) across 44 targets, clean tree, tagged `v0.2.0-alpha.1`.

---

## 1. Why this increment exists

`00-MASTER-SPEC.md` puts `std/http` at Phase 2 position 10 and gates it on 10k+ requests per second
against untrusted input. A JSON request body is untrusted input and a JSON response is a
`stringify`, so both costs sit directly on that gate's path.

Two of them were recorded as deliberate, not as oversights. `parse_value`'s header states that no
depth cap is imposed and gives the reason; `scan_str`'s header states that the accumulators are
quadratic and gives the reason. This increment reverses the first decision and retracts the second
reason's closing claim. Both reversals are argued below against the text they reverse, because a
future increment will read those paragraphs as precedent.

**Measured first-hand on 2026-08-25** against `target/debug/nova.exe` built the same day, not
inherited from the earlier increment's notes:

| behaviour | measurement |
|---|---|
| `parse` depth 5000 | `Ok` |
| `parse` depth 6000 | `thread 'main' has overflowed its stack`, exit 127, input 12001 chars |
| `stringify` depth 10000 | renders, 20001 chars |
| `stringify` depth 16000 | overflows the stack |
| `stringify` of 4000 / 8000 / 16000 one-character numbers | 231 / 1239 / 9482 ms net of a 129 ms compile baseline |

The 12001-char figure is the one that matters: **an input smaller than many HTTP request bodies
ends the process**, and it does so without producing a `JsonError`, because the frame that would
have returned one no longer exists.

The render figures reproduce the earlier increment's recorded 236 / 1309 / 11622 ms within machine
variance. Their per-doubling ratios (5.4x then 7.7x) are quoted here as absolute numbers rather
than as ratios on purpose: ratios measured this way understate the asymptotics, because memcpy
throughput rises with block size, so a genuinely quadratic accumulator can show a ratio below 4.

---

## 2. Scope

**In.** A declared depth cap on `parse`; an iterative `stringify` with a cycle guard; one growable
`Vec<Char>` buffer replacing the interpolation accumulators; fixtures for what those make pinnable
for the first time; and the record amendments in section 9.

**Out, deliberately, and disclosed rather than left silent.** `Map`'s hash exposure (section 7)
is not fixed, so the position 10 gate is **not** claimable at the end of this increment.
`stringify_pretty` stays unshipped, so section 7 of `20-STDLIB.md` stays open. `JsonError` gains no
field and no variant. No new intrinsic is added, so the 12-site checklist is not paid.

---

## 3. The depth cap on `parse`

### 3.1 The number, and why it is a contract rather than a measurement

`const MAX_DEPTH: Int = 128`.

`parse_value`'s existing header refuses a cap on the ground that "a stack-size artefact is not a
budget a cap can be derived from, and a number taken from one machine's stack would be wrong on the
next". That objection is correct and it is the reason 128 is **not** derived from the 5000/6000
threshold. It is a declared contract with a large margin — 39x below the depth measured safe on the
platform where it was measured — chosen to match an existing deployed implementation rather than
invented here.

`serde_json` starts a deserializer with a budget of 128: a bare `remaining_depth: 128` literal on a
`u8` field in `src/de.rs`, **not** a named constant, defeatable through `disable_recursion_limit`
behind its `unbounded_depth` feature. Read at `master` on 2026-08-25. Other implementations choose
differently, which is why 128 is cited as a defensible precedent and not as a standard: Jackson's
`StreamReadConstraints.maxNestingDepth()` defaults to 1000, CPython has no JSON-specific constant
and inherits the interpreter's recursion limit, and Go's `encoding/json` imposes no cap at all.

RFC 8259 section 9 permits this. Quoted: "An implementation may set limits on the maximum depth of
nesting." (RFC 8259, section 9.)

Nothing in the tree today nests deeper than 6 — measured across every `json_*` fixture and golden
with a bracket-depth count that deliberately overcounts by including Nova's own syntax — so 128
breaks nothing that exists.

### 3.2 Depth travels as a parameter

`parse_value`, `scan_array` and `scan_object` each take an ordinary `Int` depth parameter; every
function that can re-enter `parse_value` receives its caller's depth plus one. Quantified over that
property rather than over today's three-function membership, so adding a fourth re-entrant scanner
does not silently falsify this paragraph.

`Int` parameters are by value in Nova even when declared `mut` — measured, not assumed — so no
unwinding path has to restore anything.

**The justification circulating in this increment's early drafts was wrong and must not be
repeated.** That draft said a `depth` field on the `P` record "would need a decrement on every
early-return path". It would not: a field design that brackets the *call* rather than the scanner
puts the increment immediately before `parse_value`'s call site and the decrement immediately after,
which leaves every early return after the decrement point and needing nothing.

The real hazard with a field design is different and was found by measurement. `scan_array` and
`scan_object` each have an **empty-container fast path** that returns without ever calling
`parse_value`. A field design that brackets the call therefore fails to unwind for those, and
sibling empty containers accumulate: 130 sibling empty arrays, whose true depth is 2, were rejected
as "depth 129". A parameter cannot accumulate across siblings at all, which is why it is chosen.

### 3.3 What the number counts, and the fixture consequence

This is the part most likely to ship wrong, so it is stated as a rule and pinned by two fixtures
rather than one.

The empty-container fast paths never re-enter `parse_value`, so **no depth check runs at a level
whose container is empty**. A leaf costs a depth level and an empty innermost container does not.
Concretely, with the check at the top of `parse_value`:

- `[` x129 then `]` x129 — 129 containers, empty innermost — is **accepted**.
- `[` x129 then `1` then `]` x129 — the same 129 containers plus a leaf — is **rejected**.

So "maximum nesting depth 128" means 128 containers plus a leaf, or 129 containers ending empty.
A fixture asserting "129 containers is rejected" is wrong by one. Both shapes get a fixture, and
the fixture header says which of them the number 128 counts.

The check goes at the **top of `parse_value`, before the `[`/`{` dispatch**, so an over-deep level
allocates no `Vec` at all. This also bounds the number of concurrently live per-array `Vec`s to 128
where today that number is unbounded — a benign side effect worth recording, and not a size bound:
the cap bounds depth, never document size.

### 3.4 The error

`JsonError { msg: "maximum nesting depth exceeded", at: <cursor> }`, through the existing channel.
`JsonError`'s declared shape is unchanged, so `20-STDLIB.md`'s declaration of it stays accurate and
every existing caller keeps working.

ADR 0018 named two obstacles to a cap — which depth to count, and whether exceeding it is an
ordinary `JsonError` or a failure a caller must distinguish from bad syntax. Both are decided here:
depth counts open containers enclosing the value being parsed, per the rule in 3.3, and the failure
is an ordinary `JsonError`, deliberately indistinguishable in shape from a syntax error. A consumer
that must answer 400 against 413 can match the message; one that only rejects bad input needs no
change. Message-matching is a weak contract and is recorded as such. Obstacles that ADR did not
raise, and that this increment does not close, are in section 7.

`fail`'s first-failure-wins guard already gives the right behaviour, verified by tracing every
writer of `err`: the depth error is the first failure, the walk goes inert, the cursor stops
advancing, `parse`'s trailing-content check fires and the guard suppresses it. `at` is the cursor
position where the over-deep level was reached, which is stable and assertable.

---

## 4. The iterative `stringify`

### 4.1 Shape

`stringify` keeps its declared signature, `pub fn stringify(v: JsonValue) -> String`, and loses its
recursion. Pending work lives in a heap `Vec` of

```nova
type Work =
    | Chunk(String)
    | Value(Pending)

record Pending { v: JsonValue, d: Int }
```

Children are pushed in reverse, so LIFO emits them in order. Verified by probe against the prebuilt
compiler, so none of this shape is assumed: a nested `match` inside a `match` arm compiles, a second
sum type in the module compiles, a variant carrying a **record** payload compiles, a `record`
declared after the sum type that references it resolves, `Vec::new()` infers, `stack.pop()` matches,
and a three-item stack emits `[7]`.

`Pending` carries the depth because 4.4's guard needs one per work item; the earlier draft of this
section declared `Value(JsonValue)`, which cannot support the guard at all.

### 4.2 The separator must stay outside the `Some` arm

Mandatory, not stylistic. The `Object` arm's separator push goes where the current code puts its
separator append — **outside** the `m.get` match, guarded only by `i > 0`.

The tempting rewrite moves it inside the `Some` arm, because the value must be looked up before it
can be pushed. That version diverges: the current form emits a comma before member `i` iff `i > 0`,
while the inside-the-arm form emits one iff `i > 0` **and** member `i` answered `Some`. Measured
divergence on a forged map: `{"a":1,}` against `{"a":1}` for a trailing `None`, and
`{"a":1,,"c":1}` against `{"a":1,"c":1}` for a middle one.

The inside-the-arm output is the *valid* JSON of the two, which is what makes this trap dangerous:
it looks like a fix. It is a behaviour change, and this increment is not the place to make it. If
that fallback is to be repaired, it is its own change with its own record.

### 4.3 Byte-identical, stated so it is true

**For every `JsonValue` the recursive form can render, the iterative form renders the same bytes.**
Not "for every `JsonValue`" — above roughly depth 11000 the recursive form produces no bytes at all,
and the threshold is a stack artefact, so the diverging set is not even portably definable.

Verified empirically rather than by reading, before any compiler change: the work-stack traversal
was implemented in a user module beside the shipped recursive one and diffed across 20 values — the
scalars, `[]`, `{}`, `[1]`, `{"a":1}`, `[[]]`, `[{}]`, `{"a":{}}`, `{"a":[]}`, `[1,2,3]`, a mixed
array, a four-key object, `{"k":[1,{"a":1}]}` and depth 400. Zero divergences. The four-key case
rendered `{"two":2,"three":3,"four":4,"one":1}` — table order, not insertion order — identically
under both forms, which settles the `Map::keys()` ordering question: the work stack walks the same
single `keys()` array.

That probe accumulated with interpolation, because `str_from_chars` is std-only and unreachable
from a user module. So it proves **traversal order**, not the buffer drain of section 5. The drain
is verified by this increment's own tests, not by that probe.

### 4.4 The cycle guard

`const MAX_RENDER_DEPTH: Int = 100_000`, checked as children are pushed; exceeding it calls
`panic("stringify: nesting too deep or cyclic value")`.

This exists because the iterative rewrite would otherwise turn a bounded crash into an unbounded
hang. A cyclic `JsonValue` is constructible in ordinary Nova, since arrays are heap references:

```nova
let mut a: [JsonValue] = [Null]
let v = Array(a)
a[0] = v
```

Measured on all three behaviours. The recursive form dies in under a second: stack overflow, exit
127, no message. The unguarded work-stack form grew without a termination condition — 200000 pops,
100001 stack items, one net item and one character per pop — and would end at `gc::alloc`'s
`handle_alloc_error`, which aborts with neither a `nova: panic:` prefix nor a location. The guard
replaces that with an immediate, named, located failure, which is better than both.

The guard is on **nesting depth, not work-list length**. A cycle grows nesting depth without bound
while any acyclic document's depth is finite, so depth separates them; a length bound would also
trip on a wide, shallow document, which is legitimate input. 100_000 is ~780x `MAX_DEPTH` and well
above the depth 30000 the work-stack form was measured rendering, so it does not constrain
legitimate values.

Verified end to end by probe, with the bound lowered to 3000 so it was reachable: an acyclic value
walked normally, and the cyclic value above produced
`nova: panic: stringify: nesting too deep or cyclic value`, exit 127 — a named, prefixed, immediate
failure where the recursive form gives a bare `thread 'main' has overflowed its stack` and the
unguarded work stack gives nothing at all.

**The guard's cost is proportional to its bound**, which constrains how high the bound may go: a
cycle performs one iteration per level until it fires. With the linear buffer of section 5 that is
milliseconds at 100_000, and the probe demonstrates why the bound cannot simply be raised without
thought — the same probe at 100_000, accumulating by interpolation rather than into a buffer, did
not finish in two minutes. A reader raising this constant should check the accumulator first.

A panic is a process abort, so this is a better failure, not the absence of one. `stringify`
returns `String` and has no error channel to report a cycle through, and giving it one would
contradict the signature `20-STDLIB.md` declares.

### 4.5 What "heap-bounded" does and does not mean

`stringify` gets no cap of its own beyond 4.4's guard: with the work list on the heap, nesting depth
costs memory rather than stack frames — a budget larger by two to three orders of magnitude. The
absence of a general cap here is a scoping decision, not a proof of safety, and three residuals are
recorded rather than left to inference:

1. **Heap exhaustion still aborts without a `JsonError`.** `gc::alloc` calls
   `handle_alloc_error(layout)` on a null return, and no alloc-error hook is installed anywhere in
   the tree.
2. **There is no collect-and-retry.** `maybe_collect(size)` runs *before* the allocation and only
   when a byte threshold is crossed; after a null return there is no second attempt. So an abort
   can fire while collectable garbage is live.
3. **Off Windows the collector is a no-op.** `stack_base()` returns `None` under
   `cfg(not(windows))` with the comment that collection is skipped there, and `collect` early-returns.
   So on Linux and macOS the work list, every growth array `Vec::push` discards, and every
   intermediate string are retained until the process exits. "Bounded by the heap" is weakest
   exactly where this project's CI runs most of its legs.

Peak footprint also changes shape: from O(depth) native frames plus O(n^2) transient string garbage,
to O(n) live heap, because popping a wide `Array` pushes its children at once. That is a real trade
in the right direction, and its magnitude is unmeasured — recorded as an unmeasured consequence, not
as a finding. A one-frame-per-container design giving O(depth) instead of O(n) was checked and
rejected: in-place frame mutation does work (records are heap objects and `Vec::get` shares them),
but the output buffer is already proportional to the document, so the alternative buys nothing.

---

## 5. The buffer

Every `out = "${out}..."` accumulator in this file writes into a `Vec<Char>` drained once through
`str_from_chars`: `quote`, `scan_str`, and `stringify`'s `Array` and `Object` arms. That is a roster,
not a census — regrep before citing it, and after the change the grep should match nothing.

**No new type and no new intrinsic.** `Vec<T>` already ships in `std/collections`, which is
glob-imported into every module, and `std/json` already uses both halves of the composition:
`Vec<JsonValue>` at `vec_to_array` and `str_from_chars` at `span`. The drain mirrors `vec_to_array`
exactly — allocate an exact-length array, copy, encode — so it is this file's existing manoeuvre
applied to `Char`.

Verified by probe: `mut v: Vec<Char>` as a parameter compiles, and the mutation propagates to the
caller **across a reallocation**, which is the case that matters because `Vec::push` reassigns its
backing field on growth.

Measured, this compiler, debug build, 129 ms compile baseline included: pushing `n` chars into a
`Vec<Char>` took 142, 135, 132 and 142 ms at n = 8000, 16000, 32000 and 64000 — flat across an 8x
range, so amortised O(1) is measured rather than argued. Building the same string by interpolation
took 164, 249, 447 and 752 ms over the same range.

### 5.1 The retraction this requires

`scan_str`'s header currently closes with: "Neither is capped, and neither is fixable without a
growable string buffer the language does not have." `20-STDLIB.md` and `CHANGELOG.md` carry the same
claim in their own words, the CHANGELOG in two physically separate places.

**Retracted.** It is true only of a growable `String` *type*, which Nova still lacks. It is false in
the sense the sentence is used, because `Vec<Char>` plus `str_from_chars` composes into exactly such
a buffer, both already ship, and both are already called from this very file. The fix needs no
language change at all.

### 5.2 What the buffer does not fix

The accumulators were not the only cost in `stringify`'s `Object` arm. That arm also performs one
`Map` lookup per member, which the buffer change does not touch. See section 7.

---

## 6. Testing

### 6.1 What becomes pinnable for the first time

`parse_value`'s header states that "No fixture pins the threshold and none can: a fixture that
crashed the process would fail the suite by construction." That was right about a stack threshold
and stops applying to a declared cap. `MAX_DEPTH` is pinned by ordinary golden lines, and it takes
more than one fixture because of 3.3's counting rule: pin `[` x129 `]` x129 accepted **and**
`[` x129 `1` `]` x129 rejected, and say in the header which shape the number counts.

The same holds on the render side. There is no fixture in the tree today that exercises deep nesting
at all, measured two independent ways. A value built deep by loop and rendered is now an ordinary
fixture, where before it would have killed the process.

### 6.2 Mutations to run and report

- delete the depth check — the depth fixtures' goldens change
- `>=` for `>` in the depth comparison — catches the off-by-one at exactly 128
- reverse the reverse-push order in the `Array` arm
- move the `Object` separator inside the `Some` arm — must be caught by a forged-map fixture, per 4.2
- return the buffer's backing array instead of the exact-length copy — this one passes for any
  buffer that happens to be exactly full, so the mutation must be run at a length that is not a
  power of two
- delete the cycle guard — must fail by the guard's own fixture, not by a timeout

### 6.3 What stays unassertable, stated rather than implied

No test asserts *asymptotics*. The plan chooses between a Rust-side timing assertion with a wide
margin — 9482 ms measured before, expected under 100 ms after, so a 5 s bound leaves ~50x — and
recording the measurement in the records with a correctness fixture at scale. Timing assertions are
flaky in principle; a 50x margin is defensible; the choice is the plan's to make and to state.

A fixture that discriminates only by hanging is weaker than one that names a line, and this
increment should not add one: that criticism was already recorded against the `std/net` listener
fixture, and the cycle guard exists partly so this file never needs it.

---

## 7. The third cost, disclosed rather than fixed

**`std/json` remains quadratic on adversarially chosen object keys, so Phase 2 position 10's
throughput gate on untrusted input is not claimable at the end of this increment.**

`stringify`'s `Object` arm calls `m.keys()` once and `m.get(ks[i])` per member; `parse` inserts once
per key. `Map` selects buckets with `k.hash() & (cap - 1)` and probes linearly, and
`impl Hash for String` is `str_hash`, which the runtime documents as "*not* collision-resistant and
must not be used for anything security-sensitive". So an attacker choosing keys that share the low
bits of the hash makes each lookup walk the table, and both directions become quadratic in the
number of keys — independently of the depth cap and of the buffer.

This is not fixed here for a reason worth recording, because it makes the work larger than it looks:
a seeded hash needs per-process entropy and **there is no randomness source anywhere in the
runtime**. So the fix needs a new intrinsic first, and it belongs to `std/collections` rather than
to this module. It is filed as an explicit prerequisite for position 10, not as a nice-to-have.

Stated plainly, so no later reader has to infer it: after this increment `std/json` is
depth-bounded and its accumulators are linear, and it is still not safe to put in front of a hostile
client.

---

## 8. What the increment does not change

`JsonValue`, `JsonError`, `ToJson`, `FromJson` and both `parse` and `stringify` signatures are
untouched, so `20-STDLIB.md`'s declared surface needs no edit. `stringify_pretty` stays unshipped.
No intrinsic is added. `RtFunc`, `STD_ONLY` and `STD_MODULES` are untouched.

This finishes work on a module at Phase 2 position 9 rather than deviating from build order, so
**no new ADR** is needed under ADR 0014's out-of-order index. The records that change are
amendments, listed next.

---

## 9. Records to amend

A sweep of every markdown file, every `.rs` comment and every fixture header in the tree returned
roughly seventy sentences that this change falsifies or makes misleading. The roster by file:

| file | what changes |
|---|---|
| `std/json/lib.nova` | the bulk of it: `stringify`'s header, `parse_value`'s no-cap paragraphs, `scan_str`'s cost note, `quote`'s note, the `Object` arm's unreachability argument, and several `recorded at X` cross-references into paragraphs that move |
| `docs/adr/0018-std-json-scope-and-build-order.md` | the unbounded-depth passage, the two-obstacles passage (now answered), the four-accumulator passage, and the index near the end that points at both |
| `CHANGELOG.md` | the depth and accumulator paragraphs — **and a second, physically separate site making the same claim about 75 lines above them** |
| `nova-spec/20-STDLIB.md` section 7 | disclosures 1 and 2, the framing sentence that introduces them, and the "a caller putting `std/json` on a socket must impose a depth limit above it" sentence, which this increment makes false by moving that responsibility into the module |
| `docs/superpowers/plans/2026-08-22-std-json.md` | two full code listings showing the recursive `stringify` and `quote`'s accumulator |
| `tests/runtime/json_parse_values.nova` | a label calling five levels "deep", which stops being meaningful next to `MAX_DEPTH` |

That CHANGELOG pair is the shape to watch: this project has already shipped a fix that corrected the
two outer members of a triad and left the middle one carrying the retracted claim. Amend by
searching for the *mechanism* described, not for any one phrasing of it.

The plan and this spec are themselves in no downstream scanned set — every byte scan and task review
on this project covers the files its own commit touched, and these two are authored before the
dispatch loop begins. Byte-scan them when written, and re-scan the whole branch-changed set as one
population at final review.

---

## 10. Sentence-shape discipline

A dedicated sweep over the prose this increment intends to write flagged sentences that are true
today and fragile anyway, which is the class that survives correctness review because every
correctness review asks "is this false?" and gets "no". The rules it produced, applied throughout
this document:

- **Prefer a roster with no count.** "All four accumulators" is a census of a growable set, and it
  is already a corrected count once, which is the shape that reproduced the hazard last time.
- **A corrected number is usually the wrong fix.** Where a count is genuinely wanted, pair it with
  the durable predicate and say to re-measure — as `fail`'s "24 golden lines" note should.
- **No ordinals or closed worlds over `std`.** Two sentences drafted for this spec — "the only std
  module with a declared input limit" and "the first std module to impose a limit on untrusted
  input" — were **already false when drafted**: `std/time` declares `MAX_SECS`, `MAX_MILLIS`,
  `MAX_MICROS` and `MAX_NANOS` and clamps against them. Both were cut rather than corrected.
- **Quote the retracted wording inside the retraction**, which is this project's house style, and
  remember the consequence: a correct retraction still contains the text it retracts, so grep cannot
  tell a retraction from a survival. Read the context.

One pre-existing false claim is corrected in passing. The `Object` arm's comment asserts "The
`None` arm is UNREACHABLE". It is unreachable through `Map`'s own API, but `Map` exposes every field
and Nova has no field privacy, so a forged map with a live slot off its own probe chain makes
`keys()` return a key `get()` misses — and the shipped `stringify` emits `{"a":1,}` for such a value
today, measured. The comment becomes "unreachable through `Map`'s own API", and the arm is treated
as a real path in section 4.2 rather than as a formality.
