# ADR 0018 — `std/json`: position 11 before position 10, one intrinsic, and the codec's data-integrity rules

**Numbering:** confirmed against `docs/adr/`'s actual contents rather than
trusted from the plan — `0001` through `0017` all exist with no gap, so
`0018` is next. A previous increment guessed a number already in use; this
one listed the directory, as ADR 0017 did.

## Status

Accepted (2026-08-23). The `std/json` increment, branch `std-json`
(`docs/superpowers/specs/2026-08-22-std-json-design.md`).

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
   that as "accumulated decimal digits could never become a `Float`" and
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

### 8. Two unbounded costs, both disclosed here rather than only at the code

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
  `scan_str` (the accumulator numbers, all four sites)
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
