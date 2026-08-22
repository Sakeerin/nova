# `std/json` — design

**Status:** approved 2026-08-22. Branch `std-json`. BASE `8f57975`.

Builds Phase 2 position **11**, `std/json`, **before position 10**, `std/http`. The
ordering deviation is deliberate and argued in §1; it needs ADR **0018** (`0001`–`0017`
are in use — verify with `ls docs/adr/` before writing).

---

## 1. Why position 11 before position 10

`nova-spec/00-MASTER-SPEC.md:240-241` specifies the two positions differently, and the
difference is what decides the order:

- **10, `std/http`:** "server first, then client; **use hyper internals at runtime
  layer**." That is a new Rust dependency and a new runtime surface. The workspace
  currently depends on no hyper.
- **11, `std/json`:** "**custom parser**, type-safe codec via traits." `20-STDLIB.md` §7
  specifies it entirely in Nova.

So json is buildable in the style this project has established — Nova source over a thin
runtime seam — and http is not. The dependency also runs this way: http's own gate example
is a JSON API, and the Phase 2 gate needs `examples/05-json-api`, which needs json.

This is the third recorded out-of-order build (ADR 0014 skipped position 2 twice, ADR 0015
closed it). ADR 0018 records this one in the same form and should say plainly that
**position 10 remains unstarted and unblocked** — nothing here makes http harder.

## 2. The type — §7 compiles verbatim

**Measured at `8f57975`.** This exact declaration compiles and runs:

```nova
pub type JsonValue =
    | Null
    | Bool(Bool)
    | Number(Float)
    | String(String)
    | Array([JsonValue])
    | Object(Map<String, JsonValue>)
```

Three things were probed rather than assumed, because any of them would have forced a
different representation:

- **Recursion through an array works.** `type Tree = | Leaf(Int) | Node([Tree])` with a
  recursive sum over `kids[i]` printed the correct total.
- **Recursion through a generic record works.** `| Obj(Map<String, Node>)` with recursive
  descent through `m.get(k)` printed the correct total.
- **Variant names may shadow builtin type names.** `Bool(Bool)` and `String(String)` are
  accepted, and `match` arms bind them without qualification.

So **no deviation from §7's type**, which is worth stating in the ADR: the deviations on
this branch are the build order and one intrinsic, not the shape.

## 3. One intrinsic: `str_to_float`

`Number(Float)` requires decimal text → `f64`, and **nothing in the tree converts a string
to a number.** The builtin table goes outward only: `float_fixed` (added by the `std/fmt`
increment) formats a float, and there is no inverse. `char_to_int` exists but is
**`STD_ONLY`** — callable from a std module, `error[E0001]` from user code, which is fine
here and is a fact fixtures must respect.

Correct decimal→binary rounding is a research-grade problem: a `digits × 10^exp`
accumulation double-rounds, so `parse(stringify(v)) != v` for some inputs. Reimplementing
it in Nova is a bad use of effort when the Rust standard library is already correct.

**So: one new intrinsic**, `str_to_float(s: String) -> Float`, wrapping Rust's parser.

- `STD_ONLY` **65 → 66**. `STD_MODULES` **12 → 13** (`$std.json`). `RESERVED_TYPE_NAMES`
  stays **7** — `JsonValue` and friends are ordinary glob-imported records, not builtin
  types.
- The seam that is **not** compiler-forced is `symbols()` in
  `crates/nova-runtime/src/lib.rs`: an omission compiles clean and fails inside the JIT at
  link time. `every_rt_func_symbol_is_registered_with_the_jit`
  (`crates/nova-codegen-cranelift/src/lib.rs`) is the guard, and the plan must **prove it
  bites** by removing the entry and watching the test name the symbol. Count the forced
  seams when you touch them rather than quoting a number from here — that count has moved
  as the compiler grew.
- **What the intrinsic must do with input it cannot parse** is a design decision, not an
  implementation detail: return a sentinel and let Nova decide, or reject earlier. The
  scanner already validates the number's *lexical* shape before calling it, so the
  intrinsic sees only text matching JSON's grammar. It must still not panic — **no panic
  may cross a generated call site**, so an unparseable input aborts or returns a defined
  value rather than unwinding. Prefer: the scanner guarantees the shape, and the intrinsic
  returns `0.0` for anything else, documented as unreachable-by-construction.

## 4. The parser: a poisoned state record, because there is no `?`

**Measured:** `let a = half(n)?` gives `error[E0900]: the `?` operator are not supported
yet`, with the note that the feature "arrives in a later milestone". (That diagnostic's
grammar is itself wrong — "operator **are**" — worth a separate one-line fix, out of scope
here.)

With no `?`, threading `Result` through a recursive descent costs a `match` at every call
site: four to six lines per site, and the logic disappears into the plumbing. So the
parser carries its failure instead:

```nova
record P { cs: [Char], pos: Int, err: Option<JsonError> }
```

- Methods take `mut self`, mutate `pos`, and return values directly.
- `fail(m)` records only if `self.err.is_none()`, so **the first error wins** and later
  steps cannot overwrite it with a downstream symptom.
- Every step that could read past the end checks `err` first and no-ops, returning a
  harmless value (`Null`). This is the standard poisoned-parser shape: after a failure the
  remaining walk is inert rather than wrong.
- `parse` inspects `err` once at the end and returns `Result<JsonValue, JsonError>`.

**Measured** that this shape works: a record field holding `Option<Err2>`, mutated through
`mut self` methods, with `is_none()` guarding, and `peek`/`eat` over `"{}"` — first error
recorded at the right position, and the guard held.

Nova has no `loop`, so every scan loop is a `while`. `match` arms do not bind `mut`, so an
arm that needs a mutable binding rebinds inside itself.

## 5. Strings and escapes — `\uXXXX` needs no `Char` constructor

This is the part most likely to be got wrong, and the route is not obvious.

**There is no Int → `Char` conversion anywhere.** `char_to_int` goes one way and has no
inverse; the whole builtin table was checked. So a `\uXXXX` escape cannot be turned into a
`Char` directly. But `\u` escapes are **mandatory** in JSON (RFC 8259 §7), so refusing
them would be a conformance gap, not a simplification.

The route, using only shipped primitives:

1. Read four hex digits → an `Int` code point. `char_to_int` gives digit values.
2. If it is a **high surrogate** (`0xD800`–`0xDBFF`), require a following `\u` low
   surrogate (`0xDC00`–`0xDFFF`) and combine them. An unpaired surrogate is a `JsonError`.
3. Encode the code point to UTF-8 bytes in Nova — four ranges, standard, about fifteen
   lines.
4. `bytes_from_ints([Int]) -> Bytes` (`std/bytes/lib.nova:113`), then
   `Bytes::to_string() -> Option<String>` (`:23`).

**Step 4's `Option` is the validation.** `to_string` checks UTF-8 and returns `None`
rather than producing garbage, so a malformed code point falls out as a `JsonError` for
free. That is why this route is preferable to a second intrinsic: it reuses a tested,
validating primitive instead of adding an unchecked one.

The result of a `\u` escape is a **`String`**, not a `Char` — so the parser builds string
values by concatenation and never needs a `Char` constructor at all.

The simple escapes (`\" \\ \/ \b \f \n \r \t`) are direct. A `\` followed by anything else
is a `JsonError`, and an unescaped control character below `0x20` is too — both required
by RFC 8259.

## 6. `stringify`, and the edge the spec does not mention

`stringify(v: JsonValue) -> String` is total: no `Result`, no error state.

**Non-finite floats are the sharp edge.** `Number(Float)` interpolated through Nova's
float formatting yields Rust's `f64` Display, which emits `NaN`, `inf` and `-inf` — **none
of which is valid JSON.** JSON has no representation for them at all. So `stringify` must
not emit them. Two honest options, and the spec should pick one rather than leave it:

**Pick: emit `null` for a non-finite number**, matching what most JSON libraries do
(JavaScript's `JSON.stringify`, Python's is configurable but defaults to the invalid
`NaN`, Go errors). Emitting `null` keeps `stringify` total, keeps its output always valid
JSON, and is the behaviour least likely to surprise. **Document it as lossy**: a round-trip
through a non-finite number does not preserve it, and that is deliberate rather than
overlooked.

Escaping on output is the mirror of §5: `"`, `\`, and control characters below `0x20`
must be escaped, the last as `\u00XX`. Everything else passes through, including
non-ASCII, since the output is UTF-8.

## 7. The traits

Exactly as §7 writes them:

```nova
pub trait ToJson { fn to_json(self) -> JsonValue }
pub trait FromJson { fn from_json(v: JsonValue) -> Result<Self, JsonError> }
```

Both forms are proven by shipped code rather than assumed. `Clone { fn clone(self) -> Self }`
and `Default { fn default() -> Self }` establish `Self` in return position and a trait
function with **no receiver**. And `impl Default for Int { fn default() -> Int { 0 } }`
(`std/core/lib.nova:540`) shows a receiverless trait function actually implemented — note
it writes the **concrete type** in the impl's return position, not `Self`. The plan must
follow that form.

Ship impls for the primitives and for the obvious containers, and **no more**. Every impl
is public API that has to be tested; an impl nobody asked for is untested surface. YAGNI
applies hardest here, because a codec invites an endless tail of instances.

## 8. Testing — a parser is the easiest thing to shape-test badly

Every fixture must be pinned by a mutation that breaks it. A JSON parser will happily pass
a suite that asserts nothing about whether it is correct.

**Mutations to run, and the plan must run each against the whole suite and count:**

| Mutation | Should break |
|---|---|
| swap `[`/`]` handling for `{`/`}` | array vs object parsing |
| off-by-one in the four-hex-digit read | `\u` decoding |
| drop the surrogate-pair combine | astral-plane strings |
| accept an unpaired surrogate | the `to_string()` validation path |
| `stringify` emits `NaN` instead of `null` | the non-finite rule |
| skip the control-character check | RFC conformance on input |
| `fail` overwrites an existing error | first-error-wins |

**Do not claim any fixture is the only one catching its mutation** unless the whole suite
was run and counted. Four such claims were measured false across the last five increments,
and one shipped inside the correction of another.

**The round-trip fixture is the load-bearing one:** a value using **every** variant —
including a nested array inside an object, an escape, and a non-integral number — through
`stringify` then `parse`, compared for equality. It is the only test that exercises both
directions against each other rather than against a golden the author chose.

Note the harness: these are `nova run` golden-stdout fixtures, so **registration in
`crates/nova-cli/tests/run_tests.rs` is not automatic** — an unregistered fixture runs zero
tests and looks like it passed.

## 9. Records

- **ADR 0018** — the build-order deviation (§1), the one intrinsic and why reimplementing
  float parsing in Nova was refused (§3), and that **§7's type needed no deviation**.
  Verify the number with `ls docs/adr/` first; a previous increment guessed one already in
  use.
- **`nova-spec/20-STDLIB.md` §7** — a dated amendment in the file's house style,
  `**AMENDED <date> (branch \`<branch>\`):**`. Regrep the live marker set rather than
  copying a list forward, and say whether the list given is exhaustive.
- **`CHANGELOG.md`** `[Unreleased]` → Added.
- State plainly what is **not** closed: position 10 `std/http`, position 12 `std/crypto`,
  and the Phase 2 gate's `examples/05-json-api` plus `docs/benchmarks/`.

## 10. Rejected alternatives

- **`Number(String)`, keeping the lexeme.** Lossless and needs no float parser, but
  deviates from §7 on the *representation* — a larger deviation than one intrinsic — and
  pushes conversion onto every caller.
- **A naive float parser in Nova.** Keeps zero intrinsics but is knowingly lossy, and "our
  JSON codec does not round-trip numbers" is a bad property to ship deliberately.
- **A second intrinsic for Int → `Char`.** Unnecessary once the `bytes_from_ints` →
  `Bytes::to_string()` route was found, and strictly worse: it would be unchecked where
  `to_string()` validates.
- **`Result` threaded through every parser step.** Writable, but with no `?` it costs a
  `match` per call site and buries the grammar in plumbing.
- **Splitting numbers into a later increment.** Leaves §7 half-built, and the gate example
  needs real numbers.
