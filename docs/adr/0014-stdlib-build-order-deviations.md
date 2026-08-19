# ADR 0014 — Standard library build-order deviations: taking fully-specified modules ahead of `std/fmt`\r
\r
**Numbering:** confirmed against `docs/adr/`'s actual contents rather than\r
trusted from the plan — `0001` through `0013` all exist with no gap, so\r
`0014` is next.\r
\r
## Status\r
\r
Accepted (2026-08-19). The `std/log` and wall-clock increment, branch\r
`std-log-core` (`docs/superpowers/specs/2026-08-19-std-log-core-design.md`).\r
\r
## Context\r
\r
`00-MASTER-SPEC.md` §3 lists Phase 2's standard-library modules "in order"\r
and calls the build order strict:\r
\r
1. `std/core`\r
2. `std/fmt`, `std/io`\r
3. `std/collections`\r
4. `std/strings`\r
5. `std/fs`\r
6. `std/time`, `std/log`\r
7. `std/task`\r
8. `std/sync`\r
9. `std/net`\r
10. `std/http`\r
\r
By that order, the earliest incomplete entry is **position 2, `std/fmt`**.\r
`std/io`, its co-listed half, shipped; `std/fmt` did not, and still has no\r
module of its own — its four print functions are compiler builtins\r
(`Builtin::Println`, `Builtin::Print`, `Builtin::EPrint`,\r
`Builtin::EPrintln`, `crates/nova-resolver/src/lib.rs:586-589`) and\r
`nova-spec`'s own `pub record Formatter { ... }`\r
(`nova-spec/20-STDLIB.md:154`) has never been given a body.\r
\r
Position 2 has now been passed over twice:\r
\r
1. **Phase 2.1** deferred `std/fmt` together with `std/io` behind async,\r
   recorded in\r
   `docs/superpowers/specs/2026-07-25-phase-2-1-std-core-design.md`\r
   (table, "Deferred to after async (2.3)": "Every I/O signature in spec\r
   §4 is `async fn`, over `&mut [u8]` slices, returning `impl Read`.\r
   Async, references, and existential returns are all absent. Building a\r
   synchronous stand-in would be a spec deviation that gets rewritten in\r
   2.3.").\r
2. **This increment** took position 6's `std/log` — fully specified, with\r
   every dependency already shipped (`std/io`'s streams, `std/time`'s\r
   clock, `std/strings`'s `repeat`) — instead of position 2, recorded in\r
   `docs/superpowers/specs/2026-08-19-std-log-core-design.md` §2.\r
\r
A skip recorded once could be an oversight. A skip recorded twice, by\r
name, in two different increments' own design documents, is a pattern\r
this project has now chosen on purpose — and a pattern deserves a\r
decision, not a second footnote.\r
\r
## Decision\r
\r
**Take a fully-specified module whose dependencies already exist ahead of\r
an earlier, less-ready position in `00-MASTER-SPEC.md` §3's list, and\r
record an ADR entry each time it happens.** The order in §3 remains the\r
default and is not being abandoned; this ADR does not reorder it. What\r
changes is that skipping an earlier position is no longer left implicit\r
in a single increment's own design doc — it gets a line here, so the next\r
reader of §3 (or of the "ADR written for any decision deviating from this\r
spec" gate at `00-MASTER-SPEC.md` §7, item 5) finds the deviation indexed\r
in one place rather than scattered across design docs that a build-order\r
audit would otherwise have to discover independently.\r
\r
This does not relax the requirement that a skipped module eventually\r
ships. It only accepts that *readiness* — not position — decides which\r
fully-specified module gets built next, in the same register `std/bytes`\r
and `std/net` already joined `nova-spec/20-STDLIB.md` §1 as modules with\r
no dedicated numbered section yet: the codebase has already been growing\r
by readiness for a while, and this ADR is the standing acknowledgement of\r
that, not a new practice.\r
\r
## Consequences\r
\r
- **`std/fmt` still has to ship**, and what it actually needs is thinner\r
  than its position suggests and murkier than `std/log`'s §10 ever was.\r
  `Display` already exists (`std/core/lib.nova:98`, `pub trait Display {\r
  fn fmt(self) -> String }`), with impls for `Int`/`Float`/`Bool`/`Char`/\r
  `String`, and string interpolation already calls it through the\r
  typechecker's own bridge (`crates/nova-typeck/src/check.rs:5756`,\r
  `try_display`). So roughly two thirds of a `std/fmt` increment would be\r
  **replacing working mechanisms** rather than building new ones: moving\r
  four functioning builtins (`print`/`println`/`eprint`/`eprintln`) behind\r
  a module boundary, and re-pointing interpolation's existing desugaring\r
  at a Nova-level `format(parts: [FormatPart]) -> String`. The remaining\r
  third — `pub record Formatter { ... }` — has its body elided in the spec\r
  (`nova-spec/20-STDLIB.md:154`): there is no specified behaviour to build\r
  yet, so a `std/fmt` increment would begin by designing what §3 left\r
  blank, not by implementing something already decided. That is a real\r
  increment with its own scoping conversation ahead of it; it is not a\r
  thing to sweep in ahead of a module that is already fully specified.\r
- **Positions 8 (`std/sync`) and 10 (`std/http`) are also unbuilt**, and\r
  this ADR does not single them out the way it does `std/fmt`: neither\r
  has been explicitly passed over by name in a design doc the way\r
  position 2 has been, twice. Should a future increment take a later\r
  position ahead of either, that increment's own design doc should record\r
  it, and this ADR (or its successor) is where a reader should expect to\r
  find the index of such records.\r
- **The next module actually built should close position 2, or explain,\r
  again, why not** — this ADR does not pre-approve a third skip. A skip\r
  recorded twice is a decision; a skip recorded a third time with no new\r
  reasoning is the oversight this ADR exists to head off.\r
- **No other ADR is needed for this increment.** Nothing here changes the\r
  execution model, the resource model, or the GC — only the build order,\r
  which is what this document is for.\r
\r
## References\r
\r
- Design: `docs/superpowers/specs/2026-08-19-std-log-core-design.md` §2\r
  ("Why `std/log`, and the build-order deviation")\r
- `docs/superpowers/specs/2026-07-25-phase-2-1-std-core-design.md`: the\r
  first recorded skip of position 2, deferring `std/fmt` + `std/io`\r
  behind async\r
- `nova-spec/00-MASTER-SPEC.md` §3: the strict build order; §7, item 5:\r
  "ADR written for any decision deviating from this spec"\r
- `nova-spec/20-STDLIB.md` §1: `std/bytes` and `std/net` already joined\r
  the module index with no dedicated numbered section at the time, the\r
  same by-readiness pattern this ADR names\r
- `nova-spec/20-STDLIB.md` §3: `std/fmt`'s specification, including\r
  `Formatter`'s elided body (`:154`)\r
- `std/core/lib.nova:98`: `Display`, already implemented and already used\r
  by interpolation\r
- `crates/nova-typeck/src/check.rs:5756`: `try_display`, the\r
  interpolation-to-`Display` bridge\r
- `crates/nova-resolver/src/lib.rs:586-589`: the four `std/fmt` print\r
  functions as compiler builtins today\r
