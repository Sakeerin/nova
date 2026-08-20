# `std/fmt` — design

**Status:** approved 2026-08-19. Base: `main` == `origin/main` == `dc4177b`, 483 commits, 0 merge commits, 1021 tests (8 deliberately ignored), clean tree, seven CI checks green, tagged `v0.2.0-alpha.1`.

**Every `file:line` citation below is relative to that baseline.** This increment adds one builtin to `crates/nova-resolver/src/lib.rs`, which will move everything under it — as the `std/log` increment's own citations moved during its execution. Locate by content; treat a number here as a hint about where to grep.

**Goal.** Close Phase 2's position 2 by giving Nova the formatting it cannot express in-language: fixed-place decimal rendering of a `Float`, and zero- and space-padding to a width.

**Approach in one line.** A new `std/fmt` module carrying methods on `Int`, `String` and `Float`, backed by exactly one runtime intrinsic — the one thing Nova arithmetic cannot do at all.

---

## 1. Scope

### In

- A new embedded module `std/fmt`, `STD_MODULES` **10 → 11**.
- `Int::pad(width)`, `String::pad_left(width)`, `String::pad_right(width)`, `Float::fixed(places)`.
- One runtime intrinsic, `float_fixed`, `STD_ONLY` **64 → 65**.
- Collapsing `std/time`'s hand-rolled `pad2`/`pad3` into calls — the increment's own first caller.
- A dated amendment to `nova-spec/20-STDLIB.md` §3, and an ADR, because what §3 specifies is not what this ships.

### Out, deliberately

- **Format specifications in interpolation syntax** (`${x:>8.2}`). Measured: this does not parse today — `error[P0001]: expected '}' (in string interpolation), found ':'`. Adding it is a **lexer** change, not a stdlib one, and it carries a real ambiguity: interpolation holes are lexed **inline**, their tokens going straight into the stream (`crates/nova-lexer/src/lib.rs:7`), and a colon is **already legal inside a hole** — `"${P { x: 3 }}"` compiles and prints `P(3)`, measured. So `:` cannot simply mean "begin format spec"; the lexer would need a brace-depth rule. That is tractable and it is not justified yet: nothing in the tree wants aligned interpolation badly enough to pay for a grammar change.
- **`Formatter`.** §3 declares `pub record Formatter { ... }` with its body elided and describes it only as a "Format builder for Display impls". There is no specified behaviour to build. `Display` is `fn fmt(self) -> String` (`std/core/lib.nova:98`) and returns a whole string, so an incremental builder is not merely unspecified but would need a different trait shape and Nova has no `&mut`. Deferred until something needs it, and §3 amended to say so.
- **`format(parts: [FormatPart]) -> String`.** §3 names this as interpolation's desugar target over `FormatPart = | Lit(String) | Val(String)`. Note `Val(String)`: the parts arrive **already stringified**, so `format` is concatenation. The compiler already lowers interpolation to concatenation directly; routing it through a Nova function would allocate an array to do the same work, through a compiler change, with nothing visible to the user. **This is a pessimization, not a feature**, and §3's amendment records it as obsolete rather than pending.
- **Hex, binary and octal rendering**, and a `pad_with(char)` variant. Nothing has asked for them. `Int::hex` would be pure Nova and can be added in an afternoon when something wants it.

### Out permanently

- **Truncation on overflow.** A value wider than `width` is returned unpadded, never cut. A formatter that silently drops digits turns a display concern into a correctness one.

---

## 2. Why this, and not what §3 says

§3 specifies three things. The four print functions **already work** as compiler builtins (`Builtin::Print`, `Println`, `EPrint`, `EPrintln`). The other two are, respectively, a pessimization and a blank — argued above. So a literally faithful increment at position 2 would move working code behind a module, slow down interpolation, and invent a type from a one-line comment. That is churn wearing the costume of compliance.

**What is genuinely missing is measurable.** Two facts, both probed at this baseline:

- **No *Nova-visible* builtin exposes any float conversion.** The complete set of conversion builtins reachable from Nova is `char_to_int`, `bytes_to_ints`, `bytes_to_string_unchecked`. There is no `Float`→`Int`, no rounding, no truncation — and no implicit conversion either (`let i: Int = some_float` gives `error[E0010]: type mismatch: expected 'Int', found 'Float'`). **So fixed-place decimal rendering is not merely absent from the stdlib; it is inexpressible in Nova.**

  **Stated precisely, because the looser version is false:** the *runtime* does format floats. `RtFunc::FloatToStr` exists and is what interpolation lowers to (`crates/nova-mir/src/lib.rs:178`, `:424`, `:505`), implemented as `gc_str(&v.to_string())` at `crates/nova-runtime/src/lib.rs:365`. That `to_string()` is precisely what produces `0.30000000000000004`. So the gap is not that the runtime cannot render a float — it is that nothing lets a Nova program ask for a *different* rendering. This design's intrinsic is a sibling of an existing function three lines away, not a new capability in the runtime.
- **Float rendering today is unhelpful and unavoidable.** `${0.1 + 0.2}` prints `0.30000000000000004`; `${100.0 / 3.0}` prints `33.333333333333336`. A program that wants two decimal places has no route to it.

Against that, **padding is expressible** — the `std/log` increment proved it three commits ago by hand-rolling `pad2` and `pad3` in `std/time` out of `/`, `%` and interpolation, precisely because there was nowhere to get them. That asymmetry decides the shape: **the half that needs the intrinsic is the half users cannot work around, and the half that is pure Nova is consolidation of something already written by hand.** Both are in scope; only one is load-bearing.

---

## 3. Where it lives

`std/fmt` is a new `STD_MODULES` entry, placed immediately after `$std.strings`, giving `core, bytes, io, fs, collections, strings, fmt, task, net, time, log`.

**CORRECTED 2026-08-19, after Task 1's review disproved this section's original claim.** The first draft said the position was "forced from both sides" — after `$std.strings` because padding calls `String::repeat`, and before `$std.time` so `std/time` could call `Int::pad` — and that getting it wrong would fail to compile. **That is false, and it was measured false:** the reviewer moved the entry ahead of `$std.strings` in a throwaway worktree, rebuilt from scratch, and both fixtures still produced byte-correct output.

Two mechanisms make the order irrelevant here, and **both were documented in the tree before this spec was written**:

- **Method resolution is order-independent.** `collect_impls` (`crates/nova-typeck/src/check.rs:1015`) builds its table from a global `self.defs.methods()` filtered by owner — a single pass over the already-merged item list, not a per-module ordered walk. So an `impl Int` in any module is visible to every other module regardless of position.
- **The glob import is omnidirectional.** `import_std_module` binds one std module's names into *every other* module's scope, not only into later ones.

And `STD_MODULES`' **own doc comment**, twelve lines above the constant, says it outright: *"Order is significant only in that it fixes module indices; user modules always come first, then these in the order listed here — `std/core` stays first so its module index is unchanged from when it was the only embedded module."*

**The error is worth recording rather than quietly deleting, because of its shape.** The original claim cited a real comment — `resolve_program`'s *"Compile each implicit std module (in `std_entries` order)"* — and inferred a **visibility** constraint from a statement about **mechanism**. The comment that spoke to consequence was adjacent to the very line the plan directs an implementer to edit, and went unread. Citing a true sentence is not the same as citing the relevant one.

**So the only real ordering rule is: `$std.core` stays first**, to keep its module index stable. Everything after that is free, and this increment's placement after `$std.strings` is a readability choice — formatting sits near the string library — not a requirement. `00-MASTER-SPEC.md` §3's numbering is a schedule and this list is neither a dependency order nor a visibility order, which is why the two diverging (`std/fmt` at spec-position 2, list-position 7) means nothing at all.

**Methods on builtin types are legal from a module that does not define them, and additive.** Both measured: an inherent `impl Int { pub fn doubled(self) -> Int }` in an ordinary program compiles and runs; and a second inherent `impl String` alongside `std/strings`' existing one (`:113`) also works, with methods from both visible. `std/strings` is currently the **only** place in `std` with an inherent impl on a builtin — this adds the second, deliberately, because a formatting method belongs with formatting rather than with string manipulation.

---

## 4. Nova surface

`std/fmt/lib.nova`, complete:

```nova
impl Int {
    // Zero-pad to `width`, sign included in the count: `(7).pad(2)` is "07",
    // `(-5).pad(3)` is "-05". Wider than `width` returns unpadded.
    pub fn pad(self, width: Int) -> String
}

impl String {
    // Space-pad to `width`. `pad_left` right-aligns, `pad_right` left-aligns.
    pub fn pad_left(self, width: Int) -> String
    pub fn pad_right(self, width: Int) -> String
}

impl Float {
    // Fixed-place decimal: `(100.0 / 3.0).fixed(2)` is "33.33".
    pub fn fixed(self, places: Int) -> String
}
```

**Methods, not top-level `pub fn`s, and that is not a style preference.** Nova has no imports and no qualified paths: every std module's public names are glob-imported into every other module, and `import_std_module` resolves a collision **silently in the user's favour** (`crates/nova-resolver/src/lib.rs:1305-1311`). A top-level `pub fn pad` would take `pad` from every Nova program ever written, and a program with its own `pad` would silently lose access to this one with no diagnostic. `std/strings` already refused this trade for `join` and left the reason in the source (`:249-251`); `std/log` refused it for five level names last increment.

`RESERVED_TYPE_NAMES` stays at **7** — this adds no type.

---

## 5. The one intrinsic

```rust
/// Format a `Float` to a fixed number of decimal places.
#[no_mangle]
pub extern "C" fn nova_rt_float_fixed(v: f64, places: i64) -> *mut NovaStr
```

`places` is clamped to `0..=17`, then `gc_str(&format!("{:.*}", places as usize, v))`.

**The signature is copied from its sibling rather than invented.** `nova_rt_float_to_str(v: f64) -> *mut NovaStr` sits at `crates/nova-runtime/src/lib.rs:365`, is `gc_str(&v.to_string())`, and is declared `pub extern "C"` with no `unsafe` and no `-unwind` — correct for this family, because it takes no pointer and `format!` on an `f64` cannot panic. Do not reach for `extern "C-unwind"` here just because the async intrinsics use it; the two families differ for a reason, and an earlier draft of this spec got it wrong by pattern-matching on the wrong neighbour.

**This deliberately breaks the policy-in-Nova habit that made the last two increments cheap to test, and the reason is specific rather than convenient.** Both `std/time` and `std/log` put every arithmetic and formatting decision in Nova so a fixture could reach it, and that was right because the computations were exact integer arithmetic. Decimal rendering of a binary float is not that: it is a solved problem with sharp edges — shortest-round-trip representation, ties-to-even at the cut, the difference between `0.005` as written and `0.005` as stored — and Nova cannot even begin, having no `Float`→`Int`. Reimplementing it in Nova would be slower, longer, and wrong in ways only a fuzz test would find. **Where the Rust side is the correct implementation rather than the convenient one, use it and say so.**

The upper clamp of 17 is the point past which more digits carry no information for `f64`.

**The lower clamp of 0 is load-bearing in a stronger way than this section first stated.** The original wording said a negative `places` "must not reach `format!`'s precision argument", which is true and understates the consequence. Measured during Task 2 and again in its review: an unclamped negative precision panics `Formatting argument out of range`, and because this function is `extern "C"` with **no `-unwind`** that panic cannot unwind — it escalates to `panic in a function that cannot unwind` and **aborts the process** (`STATUS_STACK_BUFFER_OVERRUN`), on the direct Rust path *and* on the JIT-compiled path, where the backtrace truncates at the intrinsic frame because generated code carries no unwind tables.

**That escalation is the argument for `extern "C"` here, not against it.** `crates/nova-runtime/src/task.rs`'s module docs already establish the rule: anything reachable from a generated call site must abort rather than attempt to unwind. So the calling convention is not a judgement made for this function — it is the family's existing policy, and the clamp is what keeps this function inside it.

---

## 6. Edge cases, each with a stated answer

| Case | Answer |
|---|---|
| `width` or `places` negative | Clamped to 0. **`String::repeat` panics on a negative count** — measured: `nova: panic: String::repeat count must not be negative` — so an unclamped width would propagate a panic out of a formatting call. |
| Value wider than `width` | Returned unpadded. Never truncated (§1). |
| `(-5).pad(3)` | `-05`. The sign counts toward the width, matching Rust's `{:03}`. |
| `Float::fixed(0)` | No decimal point: `33`. |
| `places` above 17 | Clamped to 17; beyond that the digits are noise for `f64`. |
| `NaN`, `inf`, `-inf` | Rendered as Rust renders them — `NaN`, `inf`, `-inf`. Not special-cased, and not an error: a formatter that fails on a value the type permits is worse than one that shows it. |
| `Int::pad` of `0` | `(0).pad(2)` is `00`. |

---

## 7. What collapses, and why that matters

`std/time`'s private `pad2` and `pad3` — written three commits ago because there was nowhere to get them — become `Int::pad(2)` and `Int::pad(3)`.

That substitution is **the increment's own acceptance test for the API's shape**. If `Int::pad` cannot replace them cleanly, the signature is wrong and the increment should learn that from its first caller rather than from a user.

**The six ISO-8601 goldens must not move.** They are the regression test for the substitution: `1970-01-01T00:00:00.000Z`, `2000-02-29T00:00:00.000Z`, `2024-02-29T00:00:00.000Z`, `2100-03-01T00:00:00.000Z`, `2025-12-31T23:59:59.999Z`, `2025-08-31T00:01:03.007Z`. The last of those is the one that pins padding specifically — single-digit minute and second, sub-100 millisecond value — so if `Int::pad` disagrees with the hand-rolled `pad2`/`pad3` in any way, that row fails.

Note the direction of dependency this creates: `std/time` now depends on `std/fmt`. **That dependency is real but carries no ordering requirement** — §3's retracted claim said otherwise, and this sentence originally ended "…which is why §3 places `$std.fmt` before `$std.time`". Method resolution is order-independent, so the placement is a readability choice and this dependency would resolve at any position.

Recording why the sentence survived its own retraction: §3 was corrected in place and **this dependent clause in §7 was left standing**, which is the third partial fix on this project — a claim corrected at one site while a second site repeating it was missed. The pattern is consistent enough to state as a rule: **after retracting a claim, grep the document for the claim's *consequences*, not just its wording.** A retraction that leaves its own corollaries in place has not been made.

---

## 8. Testing

**Runtime unit tests** for the intrinsic: a positive value at 2 places, a clamp at negative `places`, a clamp above 17, and each of `NaN`/`inf`/`-inf`.

**Nova fixtures**, each registered with an explicit `#[test]` in `crates/nova-cli/tests/run_tests.rs` — **registration is not automatic**, and an unregistered fixture runs zero tests while looking green. This has bitten three increments.

| Fixture | Pins |
|---|---|
| `fmt_int_pad` | `(7).pad(2)`, `(0).pad(2)`, `(-5).pad(3)`, `(123).pad(2)` unpadded, `(7).pad(-1)` clamped |
| `fmt_string_pad` | `pad_left`/`pad_right` at a width above, equal to and below the length |
| `fmt_float_fixed` | `(100.0 / 3.0).fixed(2)`, `(0.1 + 0.2).fixed(2)`, `.fixed(0)`, a value needing a trailing zero such as `(1.5).fixed(3)` |
| `fmt_float_edge` | `NaN`, `inf`, `-inf`, and `places` clamped at both ends |

**Mutations to run and report**, each with the test that must fail:

| Mutation | Caught by |
|---|---|
| `pad`'s width comparison `<` → `<=` | `fmt_int_pad`'s equal-width row |
| `pad` counts digits without the sign | `fmt_int_pad`'s `(-5).pad(3)` row |
| the negative-`places` clamp removed | **the in-process unit test** `negative_places_clamps_to_zero`, which does not merely fail — it panics `Formatting argument out of range` and then **aborts the whole test binary**, because the intrinsic is `extern "C"` with no `-unwind`. **Measured, and the context matters:** at the *fixture* level (`fmt_float_edge`, a subprocess) the same mutation surfaces as an ordinary `FAILED`, because the abort happens in the child and the harness reports a non-zero exit. An earlier draft of this row named the fixture and predicted a panic, conflating the two contexts. |
| `pad_left` and `pad_right` swapped | `fmt_string_pad` |
| `Int::pad` returns the number unpadded always | `system_time_iso8601`'s single-digit row, via §7's substitution |

**A uniqueness claim is not writable here without counting.** Four times across the last two increments a claim that one test uniquely caught a mutation was measured false — the count was 7 where 18 was reported, 5 rows where 1 was predicted, all statement shapes where blocks were claimed, and all five fixtures where one was named. So the table above says which test *must* fail, not which is the only one that can. To assert uniqueness, run the mutation against the whole suite and count.

---

## 9. Records

- **CHANGELOG** `[Unreleased]`: Added for the module, the four methods and the intrinsic; **Changed** for `std/time`'s `pad2`/`pad3` becoming calls, which is an internal change with no surface effect and should say so.
- **`nova-spec/20-STDLIB.md` §3**: a dated amendment in the file's existing house style (`**AMENDED <date> (branch `<branch>`):**`, as at lines 31, 36, 169, 184, 199, 214) recording what shipped and, more importantly, **why the two unshipped items are not pending**: `format(parts)` is obsolete because the compiler lowers interpolation directly and routing through it would be slower, and `Formatter`'s body cannot be specified from its one-line comment. Also record that §3's `module std.fmt` header line is not implemented — **no std module has one**, in any of the thirteen sections, which the `std/log` increment measured and §10's amendment already notes.
- **A new ADR, 0015** (`0001`–`0014` are in use; verified, and an earlier increment got this wrong by guessing). `00-MASTER-SPEC.md` **§7 item 5** requires an ADR for any decision deviating from the spec, and shipping different content at position 2 is exactly that. It should record the deviation, the two reasons §3's items were not built, and that the position is now **closed** rather than skipped — which distinguishes it from ADR 0014, whose subject is skipping position 2 twice.

---

## 10. Measured facts this design rests on

Each checked against the tree at `dc4177b` rather than recalled:

- **No *Nova-visible* float conversion builtin exists** — the full set reachable from Nova is `char_to_int`, `bytes_to_ints`, `bytes_to_string_unchecked`. **But `RtFunc::FloatToStr` does exist** internally (`nova-mir/src/lib.rs:178`/`:424`/`:505` → `nova_rt_float_to_str` at `nova-runtime/src/lib.rs:365`, `gc_str(&v.to_string())`), which is what interpolation lowers to and what produces today's output. The first draft of this spec said "no float builtins at all", which was true of `Builtin` and false of the runtime — the distinction is the whole reason the new intrinsic is a three-line sibling rather than new machinery.
- **No `Float`→`Int` conversion**, implicit or explicit: `let i: Int = some_float` gives `E0010`.
- **Float rendering today**: `0.1 + 0.2` → `0.30000000000000004`, `100.0 / 3.0` → `33.333333333333336`, `1.0` → `1`.
- **A format spec does not parse**: `${x:>8}` gives `error[P0001]: expected '}' (in string interpolation), found ':'`.
- **Interpolation holes lex inline** (`crates/nova-lexer/src/lib.rs:7`), and **a colon is already legal inside one**: `"${P { x: 3 }}"` prints `P(3)`.
- **An inherent `impl` on a builtin works from any module**, and **a second one alongside `std/strings`' `impl String` also works**, with both sets of methods visible.
- **`std/strings` is the only place in `std` with an inherent impl on a builtin** (`:113`).
- **`String::repeat` panics on a negative count**: `nova: panic: String::repeat count must not be negative`.
- **`STD_MODULES` compiles in list order** (`crates/nova-resolver/src/lib.rs:1118`); current order `core, bytes, io, fs, collections, strings, task, net, time, log`.
- **The counts, as declared**: `STD_ONLY: [Builtin; 64]`, `STD_MODULES: [(&str, &str); 10]`, `RESERVED_TYPE_NAMES: [&str; 7]`. This increment takes them to **65**, **11** and **7**.
- **A runtime-backed builtin passes through twelve seams; eleven are compiler-forced** — nine exhaustive `match`es plus two `const` array lengths — and exactly one is not: `nova_runtime::symbols()`, where an omission compiles clean and panics inside `cranelift-jit`. It is guarded by `every_rt_func_symbol_is_registered_with_the_jit` (`crates/nova-codegen-cranelift/src/lib.rs:958`), and the plan must require proving that guard bites rather than trusting care.
- **Nova language facts relied on below the surface**: statements need no separator, and a postfix operator therefore continues across a newline onto whatever precedes it — a line beginning `[` or `(` after any expression statement is swallowed, and a `let`/`return` prefix breaks it. There is no `==` on sum types (`E0013`). `with` is a reserved keyword. A bare `return` works in a function returning nothing. `impl` works on sum types. Multi-line `match` arms need no commas.
