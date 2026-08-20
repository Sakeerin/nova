# ADR 0015 — `std/fmt` scope: closing position 2 with what Nova cannot express itself

**Numbering:** confirmed against `docs/adr/`'s actual contents rather than
trusted from the plan — `0001` through `0014` all exist with no gap, so
`0015` is next.

## Status

Accepted (2026-08-19). The `std/fmt` increment, branch `std-fmt`
(`docs/superpowers/specs/2026-08-19-std-fmt-design.md`).

## Context

`00-MASTER-SPEC.md` §3 lists Phase 2's standard-library build order, and
`nova-spec/20-STDLIB.md` §3 specifies `std/fmt` at that order's position 2:
four print functions (`print`/`println`/`eprint`/`eprintln`), a
`format(parts: [FormatPart]) -> String` that string interpolation is said to
desugar to, and a `Formatter` record whose body is elided in the spec,
described only as a "Format builder for Display impls".

Position 2 had already been skipped twice by the time this increment
started, both skips recorded in
`docs/adr/0014-stdlib-build-order-deviations.md`: once in Phase 2.1,
deferring `std/fmt` behind async; once when this project's previous
increment took position 6's `std/log` instead, fully specified with its
dependencies already shipped. ADR 0014 does not pre-approve a third skip —
its own Consequences section states that "the next module actually built
should close position 2, or explain, again, why not."

Of §3's three items, the four print functions already work, as compiler
builtins (`Builtin::Print`/`Println`/`EPrint`/`EPrintln`, per ADR 0014's own
Context section). Nothing about them needed this increment.

## Decision

**Close position 2 by shipping the formatting Nova cannot express in the
language at all, and do not ship §3's other two items.**

What shipped: `Int::pad(width)`, `String::pad_left(width)`,
`String::pad_right(width)`, and `Float::fixed(places)`, as a new `std/fmt`
module (`STD_MODULES` 10 → 11) over one new runtime intrinsic,
`float_fixed` (`Builtin::STD_ONLY` 64 → 65).

The reason to build exactly these two capabilities, and not others: **there
is no Nova-visible float conversion and no `Float`→`Int` cast anywhere in
the language** — the complete set of conversion builtins reachable from
Nova is `char_to_int`, `bytes_to_ints`, `bytes_to_string_unchecked`, and
there is no implicit conversion either (`let i: Int = some_float` is
`error[E0010]`). Fixed-place decimal rendering was therefore not merely
missing from the stdlib; it was inexpressible in Nova itself, and could only
be added from the Rust side. Padding is a different case: it was merely
*absent*, not inexpressible, and had already been proven expressible by
being hand-rolled once, as `std/time`'s private `pad2`/`pad3` helpers,
before this increment existed to give it a shared home.

What did not ship, and why neither is pending work: `format(parts:
[FormatPart]) -> String` takes parts that arrive already as `String`
(`FormatPart = | Lit(String) | Val(String)`), so it is concatenation — and
the compiler already lowers string interpolation to concatenation directly,
so routing it through this Nova function would allocate an array to do
identical work, via a compiler change, with nothing visible to any user. A
pessimization, not a feature. `Formatter`'s body is elided in §3, described
only as a "Format builder for Display impls" — there is no specified
behaviour there to implement. And because `Display::fmt` must return a
whole `String` in one call regardless (`std/core/lib.nova:98`), any builder
reachable from it is a longer, slower spelling of interpolation: a `mut
self` accumulator (ADR 0005 §1 — ten uses in shipped `std/collections`) is
buildable inside `fmt` without changing `Display`'s shape at all, but each
append copies the string accumulated so far, so it is quadratic where
interpolation is a single compiler lowering. Both are recorded in
`nova-spec/20-STDLIB.md` §3's own 2026-08-19 amendment as specifications
the compiler has overtaken, not gaps still to fill.

## Consequences

**Position 2 is closed, not skipped a third time — that is what
distinguishes this ADR from 0014, whose entire subject is the two skips.**
ADR 0014's open question ("the next module actually built should close
position 2, or explain, again, why not") is answered: closed, with the two
items that could not have shipped any other way, and the two items left
unshipped reclassified as obsolete rather than deferred, so nothing about
position 2 remains outstanding.

`STD_MODULES`'s own doc comment states that array's only real ordering
constraint: "`std/core` stays first so its module index is unchanged from
when it was the only embedded module."
Nothing else about an entry's position is enforced by anything in the
compiler — `collect_impls` (`crates/nova-typeck/src/check.rs:1015`)
resolves methods from a single global table filtered by owner, not a
per-module ordered walk, and `import_std_module` binds every std module's
names into every *other* module, not only into later ones. `std/fmt`'s
placement immediately after `$std.strings` in that list is therefore a
readability choice, not a requirement forced by either compilation order or
`00-MASTER-SPEC.md` §3's numbering — the two lists answer different
questions and were never required to agree. (This project's own design spec
for this increment claimed otherwise, twice, and both claims were corrected
by measurement before this ADR was written; this ADR states the corrected
version directly rather than repeat the error a third time.)

**One thing is still genuinely absent, and it is the one interesting gap
this increment leaves:** format specifications inside interpolation syntax,
`${x:>8.2}`, do not parse — measured, `error[P0001]: expected '}' (in
string interpolation), found ':'`. Adding them is a **lexer** change, not a
stdlib one: interpolation holes are lexed inline, their tokens going
straight into the token stream (`crates/nova-lexer/src/lib.rs:7`), and a
colon is **already legal inside a hole** — `"${P { x: 3 }}"` compiles and
prints `P(3)`, measured. So `:` cannot simply be redefined to mean "begin a
format spec" the moment it appears inside `${...}`; the lexer would need a
rule, applied at `:`, that consults the per-hole brace counter it already
has (`crates/nova-lexer/src/lib.rs:9-11`) to tell a record literal's field
colon — reached through a nested `{` — from a would-be format spec's colon
at the hole's own brace depth. The counter exists; only the rule that reads
it at `:` does not, which makes this a smaller gap than a from-scratch
brace-depth mechanism would be. Tractable, and not justified yet: nothing
in this tree wants aligned interpolation output badly enough to pay for a
grammar change, and no fixture or caller depends on it.

**No other ADR is needed for this increment.** Nothing here changes the
execution model, the resource model, or the GC — only the standard
library's surface at position 2.

## References

- Design: `docs/superpowers/specs/2026-08-19-std-fmt-design.md`
- `docs/adr/0014-stdlib-build-order-deviations.md`: the two prior skips of
  position 2, and the standing requirement this ADR discharges
- `nova-spec/00-MASTER-SPEC.md` §3: the strict build order; §7, item 5:
  "ADR written for any decision deviating from this spec"
- `nova-spec/20-STDLIB.md` §3: `std/fmt`'s specification and this
  increment's own 2026-08-19 amendment
- `std/fmt/lib.nova`: the shipped module
- `std/core/lib.nova:98`: `Display`, `fn fmt(self) -> String`
- `crates/nova-typeck/src/check.rs:1015`: `collect_impls`, the global
  method table that makes `STD_MODULES` order irrelevant to method
  resolution
- `crates/nova-resolver/src/lib.rs`: `STD_MODULES` and its doc comment (the
  one real ordering rule); `import_std_module` (the omnidirectional glob
  import)
- `crates/nova-lexer/src/lib.rs:7`: interpolation holes lexed inline, the
  mechanism behind the colon ambiguity
