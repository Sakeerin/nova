# ADR 0018 — `std/json`: position 11 before position 10, one intrinsic, and the codec's data-integrity rules

**Numbering:** confirmed against `docs/adr/`'s actual contents rather than
trusted from the plan — `0001` through `0017` all exist with no gap, so
`0018` is next. A previous increment guessed a number already in use; this
one listed the directory, as ADR 0017 did.

## Status

Accepted (2026-08-23). The `std/json` increment, branch `std-json`
(`docs/superpowers/specs/2026-08-22-std-json-design.md`).

**Amended 2026-08-25 (branch `std-json-hardening`,
`docs/superpowers/specs/2026-08-25-std-json-hardening-design.md`): §8 is
reversed.** Both costs §8 discloses as deliberate and unfixed are now bounded or
fixed — a declared depth cap in each direction, and one growable character
buffer behind every accumulator §8 said could not have one. The decision §8
recorded is left standing as written, with a dated amendment at the end of that
section answering it passage by passage, and the §8 heading carries a marker so
a reader arriving from a link meets the correction before the count. The
References entry pointing into §8 is corrected too, at the end of this file.
**No decision in this ADR changed**: the build-order decision, the intrinsic,
and the data-integrity rules all stand — §8's disclosure and the two pointers
into it are the whole of what moved. One further item lands in that amendment
and in `20-STDLIB.md` §7 without being an amendment of anything here:
adversarially chosen object keys are a quadratic exposure that neither cap
touches, whose governing decision was already taken in
`docs/adr/0005-mutable-receivers-and-one-shot-hash.md`.

**Amended again 2026-08-26 (same branch, final fix wave): statements of fact
inside that amendment are corrected or scoped** — the render direction's
boundaries are pinned in one shape only, the asymmetric counting rule belongs to
both directions rather than to `parse` alone, the guard's cost grows with the
cycle's width as well as with the bound, and that amendment's Go comparison is
flatly false rather than merely over-broad. **Again no decision changes**; the
corrections are at the end of §8, after the 2026-08-25 amendment they qualify.

**Amended 2026-09-01 (branch `std-http-parsing`, a separate later increment):
the Consequences bullet naming position 10 unstarted is now stale.** No
decision in this ADR moves — the correction is at the Consequences section
itself, below, where the stale sentence lives.

## Context

`00-MASTER-SPEC.md` §3 lists Phase 2's standard-library build order, and it
specifies positions 10 and 11 in different registers. Position 10 is
`std/http` — "server first, then client; use hyper internals at runtime
layer" (`nova-spec/00-MASTER-SPEC.md:240`). Position 11 is `std/json` —
"custom parser, type-safe codec via traits" (`:241`), and
`nova-spec/20-STDLIB.md` §7 then writes that codec entirely in Nova: a sum
type, three free functions, two traits, and a `@derive`.

The difference is not one of size. Position 10 names a **new Rust
dependency and a new runtime surface** that this workspace does not have:
there is no `hyper` anywhere in `Cargo.lock`, no `std/http/` directory, and
nothing above `net.rs`/`poll.rs` in the runtime crate — no HTTP module and
no `nova_rt_http_*` symbol in `symbols()`. Position 11 names Nova source
over the thin runtime seam this project already builds on. So json is
buildable in the style this codebase has established and http is not.

The dependency also runs in that direction. Phase 2's own gate is
`examples/05-json-api` serving 10k+ req/sec (`:245`) — an HTTP example
whose payload is JSON. Building json first supplies something http will
need; building http first would have supplied nothing json needs.

## Decision

### 1. Take position 11 ahead of position 10, and record it here

`docs/adr/0014-stdlib-build-order-deviations.md` established that a
fully-specified module whose dependencies already exist may be taken ahead
of an earlier, less-ready position, provided each occurrence gets an ADR
entry. This is that entry. ADR 0014's Consequences section asked for it by
name: it observed that positions 8 and 10 were unbuilt and that neither had
been "explicitly passed over by name in a design doc the way position 2 has
been, twice", then said — quoting it exactly — "Should a future increment
take a later position ahead of either, that increment's own design doc
should record it, and this ADR (or its successor) is where a reader should
expect to find the index of such records." This increment does exactly that
to position 10, in
`docs/superpowers/specs/2026-08-22-std-json-design.md` §1, and this is the
successor record.

**This is the third ADR in that index, and the count needs stating
carefully, because the number of ADRs and the number of deviations are not
the same number.** ADR 0014 records *two* events at once — Phase 2.1
deferring position 2 (`std/fmt`) behind async, and the `std/log` increment
taking position 6 ahead of it. ADR 0015 records position 2 finally closed,
itself out of sequence, since positions 3–5 and 7 had already shipped by
then. This ADR records one event: **position 11 built while position 10 is
untouched.** Distinguishing it from both predecessors:

- ADR 0014's two events and ADR 0015's one all concern **position 2**,
  whose blocker was a spec gap — `Formatter`'s body is elided in
  `20-STDLIB.md` §3 — and, in the first instance, a missing language
  feature. This one concerns **position 10**, whose specification is not
  thin at all; what it lacks is an implementation route in this project's
  established style.
- ADRs 0016 and 0017 reached position 8 in order and are **not** part of
  this count. ADR 0016 states plainly that "position 8 has not been
  skipped before now."

**Position 10 remains unstarted and unblocked, and nothing in this
increment makes it harder.** That is a claim about this diff, not a
prediction: no dependency was added to `Cargo.lock`, no module name or
`STD_MODULES` slot that `std/http` would want was taken, no
`RESERVED_TYPE_NAMES` entry was claimed (it stays at 7), and the only
runtime change is one leaf `extern "C"` function that parses a decimal
string. `std/http` will still need hyper at the runtime layer and will
still need to build that surface from nothing; it needs neither more nor
less of that than it did before this branch.

### 2. §7's type needed no deviation — the deviations are the order and one intrinsic, not the shape

`20-STDLIB.md` §7's declaration ships **verbatim**:

```nova
pub type JsonValue =
    | Null
    | Bool(Bool)
    | Number(Float)
    | String(String)
    | Array([JsonValue])
    | Object(Map<String, JsonValue>)
```

Measured, not assumed. Three properties were each probed before the shape
was committed to, because any one of them failing would have forced a
different representation:

- recursion through an array works (`| Node([Tree])` with a recursive sum
  over the elements);
- recursion through a generic record works (`| Obj(Map<String, Node>)`
  with recursive descent through `m.get(k)`);
- **variant names may shadow builtin type names** — `Bool(Bool)` and
  `String(String)` are accepted, and `match` arms bind them without
  qualification.

That last one is the load-bearing measurement, and it is the reason this
section exists: a reader who assumed the shadowing was illegal would
"fix" §7 into a deviation that was never needed.

`JsonError` is one addition §7 names in the traits' signatures without
declaring, so this increment declared it: `pub record JsonError { msg:
String, at: Int }`.

**What §7 declares and this increment did not build**, so that the section
is not mistaken for closed: `stringify_pretty(v: JsonValue, indent: Int)`,
the `@derive(ToJson, FromJson)` compiler builtin, and any `impl` beyond
`Int`/`Float`/`Bool`/`String` — no container impl, no blanket impl. The
container impls were rejected rather than merely skipped: `JsonValue`
already carries `Array` and `Object` variants that every fixture
constructs directly, each impl is public API that would owe its own
round-trip and mismatch fixtures, and one container impl invites the next
until the "endless tail of instances" a codec always offers is shipped
untested. `@derive` is §7's own deferral, described there as "a compiler
builtin (Phase 2)", not a library `impl` this module could have written.

**That refusal reverses the approved design, which asked for them.** The
design doc's §7 reads "Ship impls for the primitives and for the obvious
containers, and **no more**"; the plan's step 3 narrowed it to primitives
and said "Stop there", and this paragraph is where the narrowing gets
recorded. Without it, a reader who follows the References below to the
design doc — cited there for five other sections — would find it asking
for container impls with nothing anywhere saying they were dropped on
purpose.

### 3. One new intrinsic, `str_to_float`, and why a Nova float parser was refused

`Number(Float)` requires decimal text to become an `f64`, and nothing in
the tree converted a string to a number: the builtin table went outward
only, `float_fixed` formatting a float with no inverse. So
`str_to_float(s: String) -> Float` was added as a `STD_ONLY` builtin
wrapping Rust's parser — the exact mirror of `float_fixed`, and a builtin
for the same reason.

Reimplementing it in Nova was rejected on two grounds, and the **first** is
the stronger. The second is a cost, not a wall:

1. **Correctly rounded decimal-to-binary conversion is research-grade.**
   The obvious `digits × 10^exp` accumulation double-rounds — once into
   the accumulated significand, once into the scaling — so
   `parse(stringify(v)) != v` for some inputs. "Our JSON codec does not
   round-trip numbers" is a bad property to choose deliberately, and it is
   the one property a codec is most often trusted for. **This ground alone
   carries the refusal.**
2. **The missing `Int`-to-`Float` conversion makes such a parser awkward,
   not impossible.** There is no such builtin anywhere in the language and
   `as` casts are unsupported (`E0900`), so digits cannot be accumulated
   into a `Float` the obvious way. An earlier version of this section read
   that as "accumulated decimal digits could therefore never become a
   `Float`" and
   "the alternative does not exist", and ranked it *above* ground 1.
   **That was false and is corrected here.** `std/json`'s own `hex_digit`
   demonstrates the shape — an if/else chain from `Int` to a literal — and
   the same construction gives `Int` to `Float` with no conversion builtin
   and no cast, one arm per digit value. So this ground raises the cost of
   the alternative; it does not remove it, and the decision rests on
   ground 1.

Counts: `Builtin::STD_ONLY` **65 → 66**, `STD_MODULES` **12 → 13**
(`$std.json`), `RESERVED_TYPE_NAMES` unchanged at **7** — `JsonValue`,
`JsonError` and the trait names are ordinary glob-imported `std/json`
items, shadowable, not builtin types, the same standing `Instant`,
`TcpStream` and `File` already have.

#### The seam count, and the trap that matters more than the count

Adding one intrinsic touches **12 sites** across five files. **7 of the 12
are compiler-forced** — measured, not read, by adding only the two enum
variants (`Builtin::StrToFloat`, `RtFunc::StrToFloat`) and then letting
`cargo check --locked --workspace --all-targets` name the rest. Every one
of the 7 came back `error[E0004]: non-exhaustive patterns`.

**The counting rule, because a seam count with no rule behind it is not
reproducible.** A site is one *declaration, `match` arm, array or function
body in `crates/` that must change* for the new builtin to exist and work.
Three things therefore do **not** count on their own: a variant's doc
comment (it belongs to the variant it sits on — both `Builtin::StrToFloat`
and `RtFunc::StrToFloat` have one, so counting them separately would give
14 and counting only one of them, as an earlier version of this section
did, gave an unreproducible 13); an array's length annotation (it belongs
to the array, which is why omitting `STD_ONLY`'s element and its length
*together* compiles clean); and the tests that exercise the result, which
are coverage rather than seam. Under that rule the count is mechanical:
`grep -rn StrToFloat crates/ --include=*.rs` returns exactly the **10**
lines that name either enum variant, and the remaining **2** are the
`extern "C" fn nova_rt_str_to_float` definition and its `symbols()` entry,
which name the C symbol instead. 10 + 2 = 12.

Two facts about that measurement are worth more to a future reader than
the number:

- **Reaching 7 requires `--all-targets`.** One of the forced sites is a
  description table inside `nova-typeck`'s `#[cfg(test)] mod tests`, so a
  plain `cargo check --workspace` finds **6 and reports success**. A
  contributor who trusts a green plain `check` has been told the wall is
  one site shorter than it is.
- **The 7 are not discoverable in one build.** `nova-resolver`'s name
  table fired *alone* on the first pass, because cargo cannot compile the
  downstream crates until the resolver builds; the other six appeared
  together on the second. Fixing the first error and stopping means having
  seen one seventh of the work.

Of the five unforced sites, two are the enum declarations themselves (each
carrying its own doc comment), one is the `extern "C"` definition, and the
two that can actually be *forgotten* behave differently — both measured:

- **`STD_ONLY`.** Omitting the array element *and* its length together
  compiles the whole workspace clean with `--all-targets`; only a
  length/element mismatch is checked, and no test asserts that
  `GLOBAL ∪ STD_ONLY` covers every variant. But its consequence is loud:
  `std/json` is compiled on every `nova` invocation, so the omission
  yields `error[E0001]: cannot find function 'str_to_float' in this scope`
  immediately and universally.
- **`symbols()`** in `crates/nova-runtime/src/lib.rs` is the seam whose
  omission **survives every compile** — Rust's, including all test
  targets, and Nova's — to link time inside the JIT. It is caught only by
  the dedicated guard test
  `every_rt_func_symbol_is_registered_with_the_jit`
  (`crates/nova-codegen-cranelift/src/lib.rs`), which was proved to bite
  by removing the entry and watching it name `nova_rt_str_to_float`.

So the precise claim, and the one to carry forward: `symbols()` is not the
only site the compiler leaves unenforced, but it is the only one whose
omission is invisible to *every* compiler in the pipeline and is held by a
test alone.

### 4. `Map::keys()` — a position-3 API change made from a position-11 increment

`pub fn keys(self) -> [K]` was added to `std/collections`, position 3's
public API, by this position-11 increment. The immediate reason is
`stringify`: `Object(Map<String, JsonValue>)` cannot be rendered without
enumerating its keys, and `Map`'s public API was exactly `new`, `insert`,
`get`, `contains_key`, `remove`, `len` and `is_empty` — every operation
*except* the one that visits what is there. (That list is exhaustive as of
this branch's base: it is every `pub fn` in `impl Map` before this change.)

The wider reason is why this was taken as a `std/collections` change
rather than a private helper inside `std/json`: **a map that cannot be
enumerated is arguably incomplete regardless of json.** Serialisation is
merely the first caller to notice. Two properties are documented at the
method and are deliberately not guarantees: keys come back in **table
order, which is not insertion order**, and the `[fill; n]` seed the
implementation needs is forced rather than stylistic, since allocating a
`[K]` requires a `K` in hand and Nova has no null.

The table-order note is stronger than "a `grow` may reorder them", which is
how it read first and which understates it. **`keys()` order is not a
function of the key set alone.** Hash order alone reverses insertion order
with **no growth at all** — measured: inserting `"a"`, `"c"`, `"e"` into a
fresh map returns `"e"`, `"c"`, `"a"`, three entries in a cap-8 table that
never reaches the 3/4 threshold, and 2046 of the 15 600 ordered triples of
distinct lowercase letters come back exactly reversed the same way — and
linear probing makes the order depend on **arrival sequence**: when two
keys share a home slot, whichever arrives second takes the forward slot.
Naming only `grow` leaves a reader free to infer that a map which never
grows preserves insertion order, and that inference is false.

That has a measured consequence in this increment's own test material.
Re-inserting a two-key object's keys in emitted order flips their slot
order for **56 of the 3782** ordered pairs of single-character `[a-zA-Z0-9]`
keys — all 56 stable 2-cycles — which is why `json_round_trip.nova` keeps
every object single-keyed. That restriction protects its `first == second`
assertion, not merely its golden's determinism.

### 5. The `\uXXXX` route, and why it needed no second intrinsic

RFC 8259 §7 makes `\uXXXX` escapes mandatory, so refusing them would have
been a conformance gap rather than a simplification. **There is no
Int-to-`Char` conversion anywhere in Nova** — `char_to_int` goes one way
and has no inverse, and `as` casts are `E0900` — so a code point cannot be
turned into a `Char` at all. That absence was established by enumeration
rather than by searching for a phrase: no arm of `builtin_signature`
(`crates/nova-typeck/src/check.rs`) returns `Ty::Char`. The only builtins
mentioning `Char` at all are `char_to_int` (`Char` → `Int`), `str_chars`
(`String` → `[Char]`) and `str_from_chars` (`[Char]` → `String`); none of
them produces a `Char` from a number. The route used instead, over
already-shipped primitives:

1. four hex digits become an `Int` code point;
2. surrogates are paired per RFC 8259 §7, or rejected — and **the
   rejection is this parser's decision, not conformance**: §7 gives the
   pair as the form for a character above the BMP, but §8.2 notes that the
   RFC's own ABNF admits a lone unpaired surrogate and calls the behaviour
   of software that receives one unpredictable, so requiring the low
   surrogate is a choice this implementation makes. `20-STDLIB.md` §7 and
   the CHANGELOG both carry that qualification; it was missing here, in the
   document §7 sends the reader to;
3. the code point is encoded to UTF-8 bytes **in Nova** (the four length
   classes, written out);
4. `bytes_from_ints([Int]) -> Bytes`, then `Bytes::to_string() ->
   Option<String>`.

**Step 4's `Option` is the validation.** `to_string` checks UTF-8 and
returns `None` rather than producing garbage, so a code point that is not
a Unicode scalar value falls out as a `JsonError` for free.

A second intrinsic for Int-to-`Char` would have been **strictly worse, not
merely unnecessary.** The runtime already has the primitive it would have
been built from, and its body is
`char::from_u32(v as u32).unwrap_or(char::REPLACEMENT_CHARACTER)`
(`nova_rt_char_to_str`): it **silently substitutes U+FFFD** for exactly
the inputs that must be rejected. Reusing a validating primitive in place
of adding an unchecked one is the whole of the decision.

One consequence is documented at the call site rather than left to be
rediscovered: because the explicit surrogate checks make every code point
reaching the encoder a scalar value, the `None` arm is a second line of
defence that cannot fire as the code stands. It was shown to be real by
deleting the lone-surrogate rejection, after which a lone low surrogate
does reach it and is still rejected — only the message changes.

### 6. `stringify`'s non-finite rule is deliberately lossy, and the path is live

JSON has no `NaN` and no `Infinity`. `stringify` is total — no `Result`,
no error state — so a non-finite `Number` had to render as *something*
valid, and it renders as **`null`**, matching JavaScript's
`JSON.stringify`. This is recorded as a **deliberate lossy choice**: a
round trip through a non-finite number does not preserve it.

This is a live path, not a theoretical one. Nova can produce both
non-finite kinds — measured: `0.0 / 0.0` and `1.0 / 0.0` — and Nova's
float `Display` is Rust's, which would otherwise emit `NaN` / `inf` /
`-inf`, none of which is valid JSON. The finiteness test is `n - n !=
0.0`, exactly zero for any finite `n` and `NaN` for either infinity and
for `NaN`, which avoids needing an `is_nan` the standard library does not
have.

### 7. The codec's two rules, in opposite directions, both about data integrity

**Encode: `Int::to_json` silently rounds beyond ±2^53.** An `f64` mantissa
carries 53 bits, so every `Int` in `-2^53..=2^53` survives exactly; the
first that does not is `2^53 + 1 == 9007199254740993`, which rounds to
`9007199254740992` (round half to even). This is silent because
`to_json` returns `JsonValue`, not a `Result`: the trait §7 specifies has
**no error channel**, so refusing was not available even if it were
wanted. `json_traits.nova` pins the collision of that exact pair on one
rendered JSON text.

**Decode: `Int::from_json` rejects rather than truncating or wrapping.**
Two independent failures, checked in that order: a magnitude outside
`i64::MIN..=i64::MAX` is an `Err`, never a wrapped value; and a fraction
is an `Err`, never a silent truncation — `Number(3.0)` decodes to `3`,
`Number(3.5)` does not decode. Both directions of the same principle:
letting a fraction through in the decode direction would make a round trip
**fabricate** a value rather than report that it could not produce one,
which is a different and worse failure than the encode side's documented
rounding.

Two details make that range check correct rather than merely plausible,
and both are easy to get wrong:

- **`i64::MAX` is not exactly representable as an `f64`, and its nearest
  `f64` is 2^63** — `9223372036854775808.0`, which is precisely the bound
  the check rejects (`n >= 9223372036854775808.0`). The lower bound,
  `i64::MIN`, *is* exact, being a power of two. So the comparison never
  has to ask whether an inexact `Float` equals an exact `Int` edge.
- **The digits must not come from `Float`'s `Display`.** The decode path
  originally did exactly that, and `Display` is a **shortest-round-trip**
  rendering rather than an exact one: once a `Float`'s neighbours are more
  than 1 apart, the shortest text that round-trips can name a rounder
  nearby number. Measured, `i64::MIN` rendered as `-9223372036854776000` —
  **off by 192, and outside `Int`'s own range**, which is the very shape of
  bug this rule exists to refuse rather than launder into a wrapped `Int`.
  The path now takes its digits from `float_fixed(n, 0)`, which reaches
  Rust's `format!("{:.*}", places, v)` with `places` clamped to `0..=17`
  and is therefore an exact fixed-precision rendering rather than a
  shortest one. `Display` is still read, but *only* for
  the shape check that decides fraction-or-not — never for the value —
  because `float_fixed` **rounds** a fraction away
  (`float_fixed(3.5, 0)` is `"4"`) and checking that text for a `.` would
  turn the fraction rule into the truncation it rejects.

### 8. Two unbounded costs, both disclosed here rather than only at the code — AMENDED 2026-08-25: neither cost is unbounded now, and the caller obligation is discharged; read the amendment at the end of this section before citing anything in it

The Phase 2 **gate** is `examples/05-json-api`, which means `std/json` in
front of a socket, reading text somebody else chose. A reader deciding
whether that is acceptable reads this ADR and `20-STDLIB.md` §7 — not
`parse_value`'s body. Both properties below were previously recorded at
exactly one place in the tree, or at none, and both are recorded here now
for that reason. **Neither is a bug**; both are decisions, and both are
measured.

**Nesting depth is unbounded in both directions, and exceeding it kills the
process.** `parse_value` recurses through `scan_array`/`scan_object`, and
`stringify` recurses through its `Array`/`Object` arms, one native frame per
level in each. Exhausting the stack is a **hard abort, not a `JsonError`** —
`parse` has no way to return one from a frame that no longer exists, and
`stringify` has no error channel at all. Measured, debug build, Windows:

| direction | survives | dies |
|---|---|---|
| `parse` of `[` × N, `1`, `]` × N | N = 5000 | N = 6000 |
| `stringify` of a value N levels deep | N = 1024, 4096, 8192 | N = 20000 |

Roughly **12 KB of input text is enough to abort the process** — less than
most HTTP request bodies. No fixture pins either threshold and none can: a
fixture that crashed the process would fail the suite by construction. The
exact numbers are stack-size and frame-layout artefacts of this build; the
unboundedness is the portable part.

RFC 8259 §9 explicitly permits limits on nesting depth, so a cap would be
conforming. **None is imposed, and that is a decision.** A stack-size
artefact is not a budget a cap can be derived from, and a number taken from
one machine's stack would be wrong on the next; a cap also needs an API
choice this increment's scope did not include — which depth, and whether
exceeding it is an ordinary `JsonError` or a failure a caller must
distinguish from bad syntax. What is owed instead is this disclosure. **A
consumer of `std/json` on a socket needs a depth limit imposed above it,
by its caller, until one exists here.**

This also fixes a word. **`stringify` is total over *values*, not over
*depth*** — there is no `JsonValue` *shape* it cannot render and no error
channel to report one with, which is the sense §6 above intends when it
writes "total — no `Result`, no error state". The unqualified form stood at
four sites — `std/json/lib.nova`'s header comment on `stringify` and its
`number_to_json` note, `20-STDLIB.md` §7, and this increment's `CHANGELOG`
entry — and is qualified at all four now. The dated plan and design
documents under `docs/superpowers/` also use it and are left as written at
their own date, in this project's standing convention for dated artefacts.

**Four string accumulators are quadratic, not two, and the two that were
recorded are the smaller pair.** `quote` and `scan_str` rebuild `out` by
interpolation once per character, which is quadratic in **one string
literal**. `stringify`'s `Array` and `Object` arms do the same once per
element, which is quadratic in the **whole rendered document** — a strictly
larger exposure, since every document pays it. Measured with a flat ~180 ms
compile baseline subtracted, so the compiler is not the quadratic party;
absolutes are debug-build numbers, asymptotics are not build-dependent:

| input | `parse` | `stringify` |
|---|---|---|
| array of 4 000 one-char numbers | 279 ms | 236 ms |
| 8 000 | 282 ms | 1 309 ms |
| 16 000 | 344 ms | 11 622 ms |

`parse` is effectively flat; `stringify` grows **5.5x then 8.9x per
doubling**, so a ~32 KB document renders in about **twelve seconds**. One
string literal through `scan_str` costs 182 ms at 8 000 characters, 473 at
16 000, 2 256 at 32 000 and **15 624 at 64 000**. So `scan_str` becomes
user-visible at roughly a **30 KB single string literal** and only for long
*individual* strings, while **`stringify` fires on every document and is
visible from about 16 KB of output**. Neither is fixable without a growable
string buffer the language does not have (`String` has no `+`, `E0013`), so
neither is capped; this closes the "unmeasured" deferral the cost carried
before.

**AMENDED 2026-08-25 (branch `std-json-hardening`): this section recorded a
decision that a later increment reversed, and the reversal is the interesting
part.** Both disclosures above are superseded — one by a declared cap, one by a
buffer that was said not to exist. The section is left as written at its own
date and answered here passage by passage, because a reader who finds only the
correction learns what is true without learning that it was once decided
otherwise, on grounds worth reading.

**The unbounded-depth passage is retracted in both directions, and the two
directions are now bounded differently.** `parse` declares `MAX_DEPTH = 128` and
tests it at `parse_value`'s entry, so exceeding it is an ordinary `JsonError`
carrying `maximum nesting depth exceeded`, on the same channel as a syntax
error. `stringify` does not recurse at all: it walks a work list on the heap,
and its ceiling is its own declared `MAX_RENDER_DEPTH = 100_000`, tested once at
pop time. So the table above describes neither direction now, and "one native
frame per level in each" is false of both. What survives is the *shape* of the
render-direction refusal: it is a `panic`, with the message `stringify: nesting
too deep or cyclic value`, and not a `JsonError`, because `stringify` returns
`String` and still has no error channel. The two numbers are independent, chosen
for independent reasons — `parse` consumes text somebody else wrote, where a cap
is a feature in its own right, while the render cap exists because nesting depth
is the only signal available for a cycle — and they are therefore no longer one
piece of reasoning that can be stated once for both directions.

**The RFC line moves from the subjunctive to the indicative.** This section
wrote that RFC 8259 §9 permits limits on nesting depth "so a cap **would be**
conforming". A cap is imposed, so it **is** conforming: §9's permission is
exercised rather than merely noted.

**The two obstacles this section raised against a cap are quoted, and both are
answered rather than overruled.** They were: "A stack-size artefact is not a
budget a cap can be derived from, and a number taken from one machine's stack
would be wrong on the next; a cap also needs an API choice this increment's
scope did not include — which depth, and whether exceeding it is an ordinary
`JsonError` or a failure a caller must distinguish from bad syntax."

- The first is honoured, not contradicted. `MAX_DEPTH` is **not** derived from
  this build's stack. It is a declared contract, taken from the budget
  `serde_json` starts a deserializer with — a bare `remaining_depth: 128`
  literal on a `u8` field in its `src/de.rs`, not a named constant there, and
  defeatable there through `disable_recursion_limit` — read at `master` on
  2026-08-25 and cited as a precedent that makes 128 defensible, not as a
  standard. It is not one: Jackson's
  `StreamReadConstraints.maxNestingDepth()` defaults to 1000, CPython has no
  JSON-specific constant and inherits the interpreter's recursion limit, and
  Go's `encoding/json` caps nothing. The measured stack thresholds in the table
  above are still stack artefacts; they are no longer what the cap is made of.
- The second is decided. Exceeding the cap is an **ordinary `JsonError`**,
  deliberately indistinguishable in shape from bad syntax, so a caller that
  must tell the two apart has to match on the message. That is a weak contract
  and is named as one here and at the code, rather than left for a reader to
  discover.

**The obligation this section placed on the caller is discharged, and its
sentence is quoted so the retraction is legible.** It read: "**A consumer of
`std/json` on a socket needs a depth limit imposed above it, by its caller,
until one exists here.**" One exists here. In the parse direction the module
self-limits, at a declared number, reporting an ordinary error; a caller may
still want a *smaller* limit than 128 for its own reasons, which is a choice
rather than a repair. The clause "until one exists here" is what dates the
sentence, and it has expired.

**"No fixture pins either threshold and none can" was true of a stack threshold
and stops applying to a declared constant.** A fixture that crashed the process
would indeed fail the suite by construction; a fixture that reads a declared
constant's boundary just passes. Both directions' boundaries are pinned now, by
`json_depth_leaf.nova` and `json_depth_empty.nova` in the parse direction and by
`json_render_guard_under.nova` and `json_render_guard.nova` in the render
direction, with `json_render_deep.nova` and `json_stringify_cycle.nova` covering
a legitimate deep value and a cyclic one.

**The parse-direction counting rule is asymmetric, and that is why there are two
fixtures and not one.** A leaf costs a depth level, because `parse_value` is
what tests the depth and a leaf is a `parse_value` call; an empty innermost
container does not, because the empty-container fast paths in `scan_array` and
`scan_object` return without re-entering it. So **128 containers wrapping a leaf
is accepted and 129 is not, while 129 containers ending empty is accepted and
130 is not.** A single fixture asserting "129 is rejected" would be true of one
shape and false of the other, which is the off-by-one the pair exists to make
impossible.

**`stringify` is still not total over nesting depth, and the reason changed.**
The cap tests depth alone and cannot tell a cyclic value from a legitimate deep
one, so it refuses **legitimate acyclic values** past `MAX_RENDER_DEPTH` as well
as cyclic ones. A review of this increment found the opposite claim written at
the code and retracted it there; it is not to be reintroduced here. The
residuals are therefore: a cyclic value is refused by the cap; a legitimate
acyclic value past the cap is refused by the same cap and by the same mechanism;
and heap exhaustion still aborts the process without a `JsonError`, because
`gc::alloc` calls `handle_alloc_error` with no alloc-error hook installed, with
no collect-and-retry on that path and with the collector a no-op off Windows.

**The accumulator passage's closing claim is retracted, in the same terms the
code uses.** It closed: "Neither is fixable without a growable string buffer the
language does not have (`String` has no `+`, `E0013`), so neither is capped."
That is true only of a growable `String` **type**, which Nova still lacks. It is
false in the sense the sentence was used: `Vec<Char>` plus `str_from_chars`
composes into exactly such a buffer, both already shipped, and both already
called from `std/json/lib.nova` itself — `Vec` at `vec_to_array`,
`str_from_chars` at `span`. The fix needed no language change. Every accumulator
this section counted now appends into a `Vec<Char>` and drains once through
`vec_chars_to_string`: `quote`'s and `scan_str`'s, and `stringify`'s own output
buffer, which replaces what the `Array` and `Object` arms used to do. The
mechanical check, which the code's own comment tells a reader to run rather than
trust: `grep -n 'out = "${out}' std/json/lib.nova` must match nothing but text a
comment quotes for illustration.

**The measured numbers above are superseded. Read the absolutes, not the
per-doubling ratios.** Measured 2026-08-25 against this build, debug, Windows:
an array of N one-character numbers, built by one array-repeat so that almost
nothing but `stringify` is timed, rendered in **108 / 126 / 247 ms** net at
N = 4 000 / 8 000 / 16 000 — output of 8 001 / 16 001 / 32 001 characters. The
same sizes parsed in **112 / 121 / 164 ms** net. Method, because a single cold
invocation is not a measurement here: two invocations discarded as warm-up, then
the minimum of five `nova run` invocations, minus the minimum of five
`nova check` invocations of the same program, which was 51-62 ms across every
program timed.

So **the 32 KB document this section says renders in about twelve seconds now
renders in about a quarter of a second.** Rendering is within a small factor of
parsing rather than tens of times worse than it.

**The best before-evidence in the tree is not the table above.** It is
`docs/superpowers/specs/2026-08-25-std-json-hardening-design.md`, which records
the pre-change library measured **on this same build on 2026-08-25** at
**231 / 1 239 / 9 482 ms net of a 129 ms compile baseline**, for exactly the
three sizes timed after. Against that figure the render of 32 001 characters
went from **9 482 ms net to 247 ms net — a reduction of about 9.2 seconds**, and
that is the number to quote, because both halves come from one build. The
2026-08-23 table above (236 / 1 309 / 11 622 ms, ~180 ms baseline) is the older
**corroborating** measurement: same shape, same order of magnitude, a different
session and a different baseline. An earlier draft of this amendment leaned on
the 2026-08-23 figure and claimed a change of "over eleven seconds"; that
overstates what one build supports and is corrected here to ~9.2 s.

What still crosses sessions is only the *after* measurement's baseline, ~52 ms
against the design document's 129 ms, and that gap cannot account for a
multi-second change either way. A reader wanting a single controlled A/B in one
sitting can reinstate the pre-change `std/json/lib.nova` and rebuild — it is
`include_str!`'d, so a stale binary reports the new code's numbers — and re-time
both halves; that was not done in the session that produced the after-numbers.

**Do not restate any of this as a growth ratio per doubling.** The old numbers'
own "5.5x then 8.9x" and any ratio computed from the new ones both mislead, in
opposite directions: memcpy throughput rises with block size, which damps the
wall-clock ratio of a quadratic below what its asymptotics imply, while a fixed
per-run cost that the netting does not remove flattens the ratio of a linear one
at small N. What was measured about the retired idiom **in the same session as
the after-numbers** is this: the same text built in user space by rebuilding an
accumulator with interpolation once per element cost 26 / 62 / 155 ms net at the
same three sizes — 6.0x for 4x the work, super-linear, where the shipped path
cost 2.3x for the same 4x. That probe is a **floor** under what the old `Array`
arm paid rather than a reconstruction of it: that arm rebuilt the whole
accumulator **twice** per element, once for the separator and once for the
element, on top of the tree walk and the number formatting.

**Not an amendment of anything above, and not a new disclosure either:
adversarial object keys are a quadratic exposure that neither cap touches.** The
exposure itself is already on the record, in
`docs/adr/0005-mutable-receivers-and-one-shot-hash.md`, which states that
"**Hashes are not randomized per process**, so a `Map` is HashDoS-attackable by
adversarial keys", that neither FNV-1a nor `mix64` is collision-resistant, and
that this is "Acceptable for Phase 2.2a". What this increment adds is the
**`std/json`-specific consequence** and a ruling about the gate. See the
`20-STDLIB.md` §7 disclosure added the same day for the full statement. In
short: `stringify`'s `Object` arm performs one `Map` lookup per member and
`parse` one insert per key; `Map` selects buckets as `k.hash() & (cap - 1)` and
probes linearly (`std/collections/lib.nova`, `slot_of`); and `impl Hash for
String` is `str_hash`, whose runtime doc comment
(`crates/nova-runtime/src/lib.rs`, `nova_rt_str_hash`) says in terms that it "is
*not* collision-resistant and must not be used for anything
security-sensitive". Keys chosen to collide therefore make **both directions of
the codec** quadratic in the number of keys, independently of `MAX_DEPTH`, of
`MAX_RENDER_DEPTH` and of the buffer.

**So this section's own premise is not satisfied by this increment: Phase 2's
throughput gate is not claimable on untrusted input on the strength of it.**
NARROWED 2026-08-26 by a later increment — read the third amendment below before
citing this sentence or the remedy it names; the sentence is now too broad rather
than false, and the remedy named is the wrong one. The
gate needs positions 10 and 11 together — `examples/05-json-api` is this module
behind an HTTP server that does not exist — so it is Phase 2's gate rather than
either position's, and an earlier draft of this amendment that called it
"position 10's throughput gate" is corrected. The remedy is the one ADR 0005
names and not a new intrinsic: a `Hasher`-shaped question, per-map hasher choice
or HashDoS resistance via a seed, reached through the accumulating-`Hasher`
migration that ADR describes, which it records as a breaking change with a
deprecation cycle. It belongs to `std/collections`, since every
`Map<String, _>` in the language carries the exposure and `std/json` is only
where it meets untrusted input. The Consequences bullet below saying nothing
here is a step toward that throughput number stands, and now has a second
reason.

### Second amendment, 2026-08-26 (same branch, final fix wave before merge)

Each item below corrects or scopes a statement of fact in the 2026-08-25
amendment above; no decision moves. Most were stated wider than the evidence
rather than wrong outright; the Go comparison was flatly false, and is marked as
such where it appears. This paragraph carried a
count of its own items until 2026-08-26, and the count was already wrong when it
was written, the Go correction having joined the list after it; the roster is
what to read, which is the same lesson the wave that wrote it was applying
elsewhere.

**"Both directions' boundaries are pinned now" is true of the parse direction
and true of ONE SHAPE in the render direction.** The paragraph above pins the
parse boundary with `json_depth_leaf.nova` and `json_depth_empty.nova` and the
render boundary with `json_render_guard_under.nova` and
`json_render_guard.nova` — but the render pair builds only the leaf-terminated
shape, and the render cap turns out to have **the same asymmetric counting rule
the parse cap has**. The shipped guard is one **pop-time** check on the depth an
item carries, hoisted out of the two per-arm push-time checks the plan
prescribed, and an empty innermost container pushes no child at all, so nothing
on that shape ever carries the depth the guard would refuse. Measured against
the shipped binary, arrays one deep per level: a chain ending in a leaf renders
at 100000 containers (200001 characters, exit 0) and panics at 100001; a chain
ending in an empty array renders at 100001 (200002 characters, exit 0) and
panics at 100002. The empty-innermost boundary is pinned by nothing, so a change
that moved it by a level would pass the whole suite. It stays unpinned by
ruling — render fixtures cost seconds each and that trade has not changed — and
is disclosed at the guard in `std/json/lib.nova` instead.

**So "the parse-direction counting rule is asymmetric" scopes the rule too
narrowly.** Both caps count the same way — a leaf costs a level, an empty
innermost container does not — for the same underlying reason in each direction:
in `parse` the empty-container fast paths return without re-entering
`parse_value`, and in `stringify` an empty container pushes no `Value` onto the
work list. What is parse-specific is only that **both** of its shapes have a
fixture. The clause "and that is why there are two fixtures and not one" still
explains the parse pair; it should not be read as saying the render direction
has no such rule.

**The guard's cost is not a function of its bound alone, and "an immediate,
named, located failure" is a promise about narrow cycles.** The work list gains
about `2w` items per level for a container of width `w` — `w` children, `w - 1`
separators, two brackets, one item removed — so both its peak size and the time
to fire grow with the bound **times the width**. Measured against the shipped
binary, min of 3 after a warm-up, cyclic arrays that fire the guard: width 1,
1.78 s; width 3 with the cycle at the last member, 4.71 s; width 10 at the last
member, 14.24 s; width 10 at the first member, 7.14 s. The absolutes are
machine-specific — an earlier session measured the width-1 case at about 2 s —
and the growth with width is the claim. `json_stringify_cycle.nova` builds a
width-1 cycle and no fixture in the tree builds a wider one, which is a
predicate to re-check rather than a tally to trust: this paragraph said "both
cycle fixtures" until 2026-08-26, and only that one closes a cycle at all.
The consequence, which is reasoning past width 10 rather
than a measurement: that peak is the same allocator pressure the guard was
introduced to replace, so a wide enough cycle reaches `handle_alloc_error` —
neither a `nova: panic:` prefix nor a location — before it reaches the named
panic. Nothing measures where the crossover sits.

**And the Go comparison in the two-obstacles passage above is false.** It reads
"Go's `encoding/json` caps nothing". `encoding/json` declares `const
maxNestingDepth = 10000` in `src/encoding/json/scanner.go` and enforces it in
`pushParseState`, which reports `exceeded max depth` — read at `master` on
2026-08-26. It was the one comparison in that sentence carrying no file and no
read date, beside a `serde_json` claim carrying both. The neighbours check out as
read on 2026-08-26: Jackson's `StreamReadConstraints.DEFAULT_MAX_DEPTH` is
`1000` (`src/main/java/com/fasterxml/jackson/core/StreamReadConstraints.java`,
branch `2.x`), and CPython's C accelerator declares no JSON-specific constant and
calls `_Py_EnterRecursiveCall` in `scan_once_unicode` (`Modules/_json.c` at
`main`). The argument is unchanged: implementations that declare a limit pick
different numbers, so 128 is a defensible precedent rather than a standard.

### Third amendment, 2026-08-26 (branch `map-hashdos`, a separate later increment)

Not the same wave as the two amendments above, which were this ADR's own branch
finishing. This one comes from the increment that seeded `str_hash`, and what it
touches in section 8's HashDoS paragraphs is headed below. No decision in this
ADR moves, and nothing about `std/json` itself changes: the caps, the guard and
the buffer are as recorded.

**The gate claim narrows.** The sentence above reads "**So this section's own
premise is not satisfied by this increment: Phase 2's throughput gate is not
claimable on untrusted input on the strength of it.**" That remains an accurate
statement about *this* increment — the JSON hardening did not and could not make
it claimable. As a standing statement about the tree it is now too broad.
`nova_rt_str_hash` is seeded once per process and finalized with splitmix64, so
the gate is claimable **for string-keyed maps against a precomputing attacker**,
one who must build a colliding key set before it can observe the process that
will receive them. It is **not** claimed against an adversary who can observe
timing and adapt, and **not** claimed for `Int`, `Bool` or `Char` keys, whose
`mix64` path is unseeded and whose buckets are still a function of the key alone.
`std/json`'s own exposure is the string-keyed one, object keys being strings.

**The remedy this section names is not the remedy that shipped.** The paragraph
above says "The remedy is the one ADR 0005 names and not a new intrinsic: a
`Hasher`-shaped question, per-map hasher choice or HashDoS resistance via a seed,
reached through the accumulating-`Hasher` migration that ADR describes, which it
records as a breaking change with a deprecation cycle." The "not a new
intrinsic" half was right and no intrinsic was added. The migration half was
wrong, and it was wrong here in the same way it was wrong in
`nova-spec/20-STDLIB.md` §7: ADR 0005's closing Migration-path paragraph
permits "replacing FNV-1a inside `nova_rt_str_hash` ... and seeding either from a
process-start value" precisely because none of that touches `Hash`'s signature.
Only a swappable seeded `Hasher` **object** needs the migration. `fn hash(self)
-> Int` is unchanged, no `impl Hash` was edited, no Nova library source changed
**behaviour** — the Nova-side edits in that increment are comments, in
`std/core`, `std/collections` and `std/json` — and there was no deprecation
cycle. ADR 0005 now carries a dated amendment saying which of its conflicting
sentences governs.

**What stays true.** The quadratic shape this section describes is still the
right shape for an attacker who can adapt, and the `std/collections` framing
still holds — every `Map<String, _>` carried the exposure and `std/json` is
where it met untrusted input, which is why the fix landed in the runtime rather
than in this module. Seeded, finalized FNV-1a is not collision-resistant and is
not cryptographic; the runtime function says so at itself.

**Section 4's table-order measurements need the same scoping that increment
applied in `nova-spec/20-STDLIB.md` §12 and at `Map` in
`std/collections/lib.nova`.** The figures there were measured under the
unseeded `str_hash`: `"a"`, `"c"`, `"e"` inserted into a fresh map coming back
`"e"`, `"c"`, `"a"`; 2046 of the 15 600 ordered triples of distinct lowercase
letters coming back exactly reversed; and 56 of the 3782 ordered pairs of
single-character `[a-zA-Z0-9]` keys flipping slot order when a two-key object's
keys are re-inserted in emitted order. With `str_hash` seeded once per process
each of those is a per-process function of the seed, so none may be relied on
run to run — not that each must differ, since two seeds may agree on a layout.
What they were recorded to establish survives untouched, which is why they are
kept rather than deleted: `keys()` order is not a function of the key set alone,
the reordering needs no `grow`, and a two-key object can therefore flip across a
render and a reparse. `json_round_trip.nova`'s single-key restriction still
protects its `first == second` assertion, and protects it against whatever
layout a run builds rather than against one measured set of pairs. Read each
figure as an instance of its property under the hash of the day.

### Fourth amendment, 2026-08-27 (branch `hashdos-resistance-test`, a separate later increment)

From the increment that added a test for the resistance property, not from this
ADR's own branch and not from the seeding increment that wrote the amendment
above. **No decision in this ADR moves, and `std/json` itself is untouched** —
the caps, the guard and the buffer are as recorded, and no signature changes.

**The gate claim's SCOPE does not move; its EVIDENCE does.** The paragraph
headed "**The gate claim narrows**" above defines the precomputing attacker as
"one who must build a colliding key set before it can observe the process that
will receive them", and
`hashdos_precomputed_key_set_does_not_survive_a_new_process` in
`crates/nova-cli/tests/run_tests.rs` now executes that definition rather than
approximating it: one Nova process searches candidate keys for a set
concentrating 32 of them in one bucket under its own seed, and a second,
separately launched process — therefore a fresh seed — re-hashes that same set
and is asserted to spread it. Until this increment the precomputing half rested
on figures taken in a throwaway harness and written up in prose. The
thresholds, their derived error rates, and the coverage that test does not give
are in its own doc comment and are not restated here. Nothing here licenses a
stronger reading of that paragraph.

**Its exclusions stand unchanged, and neither of the things they exclude gains
evidence here.** `Int`, `Bool` and `Char` keys are untouched, `mix64` still
being unseeded. And the exclusion of "an adversary who can observe timing and
adapt" is **still unaccompanied by any assertion added in this increment**: the
searching phase calls `.hash()` directly, which requires code running inside
the target process and is a stronger capability than observing timing from
outside, so that phase is not an instance of the adversary the exclusion names.
Whether anything else in the test suite bears on that exclusion is a question
for the suite, not for this sentence.

### Fifth amendment, 2026-08-28 (branch `seeded-mix64`, a separate later increment)

From the increment that seeded `std/core`'s `Int`, `Bool` and `Char` hashing.
**No decision in this ADR moves, and `std/json` itself is untouched** — the
caps, the guard and the buffer are as recorded, and no signature changes.
`docs/adr/0005-mutable-receivers-and-one-shot-hash.md`'s 2026-08-28 amendment is
the governing record; this section restates it for the sentences here that it
falsifies, and that amendment wins wherever the two diverge.

**The gate claim's exclusion of `Int`, `Bool` and `Char` keys closes; its
exclusion of the timing-adaptive adversary and its refusal of a cryptographic
claim stand as they read.**
The paragraph headed "**The gate claim narrows**" above excludes `Int`, `Bool`
and `Char` keys, "whose `mix64` path is unseeded and whose buckets are still a
function of the key alone", and the fourth amendment above restates that
exclusion as "`Int`, `Bool` and `Char` keys are untouched, `mix64` still being
unseeded". Both are now false. `std/core`'s primitive impls for `Int`, `Bool`
and `Char` compute `mix64(key ^ int_hash_seed())` over a per-process seed drawn
in the runtime by a call separate from the string seed's — that separate
`RandomState::new()` call sits in the private `str_hash_seed`, which
`nova_rt_str_hash` calls — XORed into `mix64`'s **input** rather than its
output, because `Map` consults the low bits and a post-XOR would permute
buckets without separating any colliding pair. `mix64` itself is unchanged:
what was unseeded and stays so is a module-private mixer rather than a key
type. The narrowed claim is pinned by
`hashdos_precomputed_int_key_set_does_not_survive_a_new_process` in
`crates/nova-cli/tests/run_tests.rs`, which is the two-phase shape the fourth
amendment describes for `String` keys, run through the other seed.

**The timing-adaptive exclusion and the non-cryptographic one apply to that
path on the same terms.** The gate is still **not** claimed against "an
adversary who can observe timing and adapt", and the paragraph headed
"**What stays true**" above still holds as written — "Seeded, finalized FNV-1a
is not collision-resistant and is not cryptographic",
and neither was `mix64` ever collision-resistant. For `Int`, `Bool` and `Char`
the adaptive exclusion is now concrete rather than categorical in the same way it
already was for `String`: `(0).hash()` is `mix64(seed)`, `mix64` is a bijection,
and `tests/runtime/hash.nova` performs that recovery, so one call from ordinary
Nova code yields the running process's int seed exactly. Precomputation
resistance is the half that survives, and that path is exactly as strong as the
string path and no stronger.

**Section 4's table-order measurements need no further scoping.** Every figure
in that section is measured over `String` keys, `std/json`'s object keys being
strings: the order a fresh map RETURNS for the insertions `"a"`, `"c"`, `"e"`,
which is `"e"`, `"c"`, `"a"` — the output rather than the input, which is how
`nova-spec/20-STDLIB.md` §12 names it as well — the cap-8 table and the 3/4
threshold that ordering happens under, the 2046 of 15 600 ordered triples of
distinct lowercase letters, and the 56 of 3782 ordered pairs of
single-character `[a-zA-Z0-9]` keys, all 56 of them stable 2-cycles. So the
third amendment's scoping of them to the hash of the day is what it was. What
widens is the general `Map` statement rather than this module's instance of
it: a
`Map<Int, _>`, `Map<Bool, _>` or `Map<Char, _>` now has the run-to-run layout
freedom a `Map<String, _>` has had since 2026-08-26. `nova-spec/20-STDLIB.md` §12
carries that, and `Map::keys`' own note in `std/collections/lib.nova` carries it
at the method.

## Consequences

- **Phase 2 is not complete, and this increment does not close it.**
  Position **10, `std/http`** is unstarted (no `hyper` in `Cargo.lock`, no
  `std/http/`). Position **12, `std/crypto`** is unstarted (no `ring` in
  `Cargo.lock`, no `std/crypto/`). The Phase 2 **gate** at
  `00-MASTER-SPEC.md:245` needs `examples/05-json-api` — `examples/` holds
  `01-hello-world`, `02-fibonacci` and `03-producer-consumer` — and needs
  benchmark methodology documented in `docs/benchmarks/`, which does not
  exist. Nothing here is a step toward the throughput number that gate
  names.
  [Forward marker, 2026-09-01, branch `std-http-parsing`: position 10 is no
  longer unstarted. Its server half ships — request-head parsing over one
  intrinsic, response serialisation over none — see
  `docs/adr/0019-offset-table-intrinsic-boundary.md`. Position 12
  `std/crypto` is the one of the two named here that is still unstarted; a
  sentence naming both together as "the two missing module groups" is wrong
  now, and naming `std/crypto` alone is the correction, not a revised count.
  `examples/05-json-api` and `docs/benchmarks/` still do not exist, so Phase
  2's gate is still not reached. The wording here is left as written and
  superseded by this marker rather than edited, the convention this
  project's CHANGELOG already uses: a shipped record keeps what it claimed
  at the time.]
- **Position 8 stays partial**, unchanged by this increment and recorded
  in ADRs 0016 and 0017; and **ADR 0014's bullet describing positions 8 and
  10 as unbuilt and not yet passed over by name is now stale in both
  halves** — position 8 was reached by ADRs 0016/0017, and position 10 has
  now been passed over by name, here. That bullet is left as written at
  its own date, in the same convention this project already applies to
  dated amendments; this is its successor.
- **The `lib.nova` file count moves 13 → 14**, and the running count in
  `20-STDLIB.md` §13's 2026-08-20 (`std-sync-mutex`) amendment is now one
  behind. That amendment records "12 `STD_MODULES` entries plus
  `STD_TEST_MODULE` is **thirteen** `lib.nova` files on disk"; with
  `$std.json` it is 13 plus `STD_TEST_MODULE`, **fourteen**. Two amendments
  before it (§3's and §10's, both 2026-08-19) record twelve and eleven,
  each correct at its own date. The §7 amendment written this increment
  continues that chain rather than editing the earlier ones.
- **`std/collections`' public API grew from outside its own position**, and
  that is now a precedent as well as a fix. A later increment that finds
  another position-3 gap should expect to be pointed here.
- **`str_to_float` is a general-purpose intrinsic with two callers, not
  one.** It was introduced for the parser's number scanner and is now also
  the whole mechanism of `Int::to_json`. Any future change to its
  rounding, its sentinel, or its acceptance grammar reaches both
  directions of the codec.
- **`float_fixed` has two callers now as well, and nothing recorded that
  until this bullet.** It was introduced for `std/fmt`'s `Float::fixed`;
  `Int::from_json` is the second, taking an integral `Float`'s exact digits
  from `float_fixed(n, 0)` for the reason §7 above gives. So the *mirror
  pair* of intrinsics each gained a second caller in this increment, one in
  each direction of the codec, and a change to `float_fixed`'s rounding or
  its `0..=17` clamp now reaches the JSON decode path as well as `std/fmt`.
  Worth its own bullet because the `str_to_float` bullet above was written
  while **both** variants' doc comments in `crates/nova-resolver/src/lib.rs`
  still claimed a single caller — and `Int::from_json`'s own twelve-line
  comment reasons about precisely the blast radius that claim misdirected.
  Both comments are corrected.
- **No other ADR is needed for this increment.** Nothing here changes the
  execution model, the resource model, or the GC. The 8 `#[ignore]`d
  ADR-0010 GC tests are untouched.

## References

- Design: `docs/superpowers/specs/2026-08-22-std-json-design.md` (§1 the
  ordering, §3 the intrinsic, §5 the escape route, §6 the non-finite rule,
  §10 rejected alternatives)
- Plan: `docs/superpowers/plans/2026-08-22-std-json.md`
- `nova-spec/00-MASTER-SPEC.md:240-241`: positions 10 and 11; `:245`: the
  Phase 2 gate; §7 item 5: "ADR written for any decision deviating from
  this spec"
- `nova-spec/20-STDLIB.md` §7: `std/json`'s specification, and this
  increment's dated amendment recording what shipped against it
- `docs/adr/0014-stdlib-build-order-deviations.md`: the build-order index
  this ADR extends, and the bullet asking for exactly this record
- `docs/adr/0015-std-fmt-scope.md`: position 2 closed
- `docs/adr/0016-std-sync-partial-close.md`,
  `docs/adr/0017-std-sync-channel-shape.md`: position 8, reached in order
  and still partial
- `std/json/lib.nova`: `JsonValue`, `JsonError`, `stringify`, `parse`, the
  poisoned-parser record `P`, and the two codec rules with their reasoning
  at the impls; §8's two disclosures live at `stringify`'s header comment,
  `parse_value` (the depth reasoning, stated once for both directions) and
  `scan_str` (the accumulator numbers, all four sites).
  **AMENDED 2026-08-25 (branch `std-json-hardening`):** this pointer is stale
  in both parentheses, for the reasons §8's amendment gives. The depth
  reasoning is no longer stated once for both directions and cannot be: the
  two directions are bounded by two different declared constants for two
  different reasons, so `parse_value`'s comment covers `MAX_DEPTH` and the
  counting rule while `stringify`'s header covers `MAX_RENDER_DEPTH`, the
  cycle, and what the cap refuses that is not a cycle. And `scan_str` now
  carries the accumulator **roster by name rather than by count**, since a
  count of a set a later increment can add to is the shape that goes stale;
  read the roster and re-run its grep instead of trusting a number from
  here or from there.
- `std/collections/lib.nova`: `Map::keys`, with the table-order and
  `[fill; n]` notes
- `crates/nova-resolver/src/lib.rs`: `Builtin::StrToFloat` and its doc
  comment, the name table, `STD_ONLY` (66), `STD_MODULES` (13),
  `RESERVED_TYPE_NAMES` (7), `STD_TEST_MODULE`
- `crates/nova-runtime/src/lib.rs`: `nova_rt_str_to_float` and its
  no-panic contract, its `symbols()` entry, and `nova_rt_char_to_str`'s
  U+FFFD substitution
- `crates/nova-codegen-cranelift/src/lib.rs`:
  `every_rt_func_symbol_is_registered_with_the_jit`, the guard on the one
  unenforced seam
