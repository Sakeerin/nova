# ADR 0049 — Tuples and patterns

## Status

Accepted (2026-10-27). Branch `claude/loving-archimedes-frtf5q-50b`
(`docs/superpowers/specs/2026-10-26-phase-5-0-the-language-and-std-a-compiler-needs-design.md`
§4 and §4.7). Tuples without impls and the pattern forms are the user's
decisions 3 and 4 of 2026-10-26; lowering by test chains is the spec's
recommendation, taken without asking, which the user can overturn (§12
item 17).

## Context

Phase 5's compiler, N, is written in Nova, and before 5.0b Nova had no
tuples and only flat patterns: a variant whose fields are names or `_`,
a name, `_`, and `Int`, `Bool` and `String` literals. Every other form was
E0900, and `let` took a name or `_`. The spike that ported the lexer to
Nova wrote an inner `match` for each nested pattern and a mutated position
for each returned pair (spec §2).

The spec files say more than 5.0b builds: `13-RUNTIME.md` §2.2 lays a
tuple out without a header when it is on the stack, `14-CODEGEN.md` §4
flattens a small one, and §6.2 compiles a `match` to a Maranget decision
tree. `11-PARSER.md` §2's `pattern` includes record, array and `@`
patterns.

## Decision

1. **Tuples are anonymous records** (decision 3). `(T1, …, Tn)`, for n of
   one or more, is `Ty::Tuple`; `()` stays unit and `(T)` is `T`. Two tuple
   types are the same when their elements are. A tuple has no `TyHead`, so
   no impl names one: no `==`, interpolation, `Eq`, `Hash` or `Display`,
   and no `Map` key (E0013). `t.N` reads element `N`.
2. **A tuple's layout is a record's:** MIR's `MakeRecord`, a heap object
   with a header and one 8-byte slot an element, passed by reference, on
   every backend, WASM included. This departs from `13-RUNTIME.md` §2.2 and
   `14-CODEGEN.md` §4 as records already do. No backend sees a tuple.
3. **The pattern forms in:** `_`, names, `Int`, `Bool`, `String` and `Char`
   literals, variants by bare or qualified name with fields that are
   patterns, tuples, or-patterns at any depth, `Int` ranges `lo..hi` and
   `lo..=hi`, `Char` ranges `lo..=hi`, and guards. `let` and `for` take an
   irrefutable pattern. **Out, still E0900:** record and array patterns,
   `x @ p`, and `Float` literals; there is no `is`, `if let` or `while let`
   (decision 4).
4. **Usefulness does not split ranges.** A literal and a range are each a
   constructor of their own, so a match on them needs `_` or a name, and an
   arm earlier ranges cover is not reported unreachable (§12 item 14).
5. **A `match` lowers by test chains, not a decision tree.** A match whose
   arms are all flat, with no guard, lowers exactly as before 5.0b: one
   `Switch`, or a chain of string compares, byte for byte. Any other match
   lowers wholly by chains: the scrutinee is evaluated once, and each arm in
   order tests its pattern left to right and depth first, on values
   projected once each, binds, runs its guard, and on a failed test or a
   false guard jumps to the next arm. An or-pattern's alternatives join
   where they succeed. There is no flat switch before the inner chains.
   No new MIR statement exists, so Cranelift, LLVM, WASM and the async
   transform are unchanged.
6. **What spec §4 left open (its §13), settled:**
   - **A tuple index is plain decimal digits:** `t.01`, `t.1_0`, `t.0x1`
     and `t.00` are P0001, so each index has one spelling. A tuple has one
     element or more, with no upper limit.
   - **A tuple type mangles as `T<elements>E`,** which is prefix-free, so
     two distinct tuple types never share an instance's name.
   - **The HIR pattern is recursive** (`Variant { fields }`, `Tuple`, `Or`,
     `Range`, `Bind`, the literals, `LitChar` among them), and a chain
     projects each sub-value once, into a temporary, just before its own
     tests.
   - **Ranges are not split** (item 4), and **an unreachable alternative of
     an or-pattern is reported:** E0021, "unreachable alternative", on the
     alternative alone, when its arm is reachable, in the order the test
     chain tries the alternatives: one no value takes, every value that
     matches the arm with it being matched by an earlier arm or taking an
     earlier alternative of its or-pattern or of one it sits in. So in
     `(a, 1 | 2) | (_, a)` the `2` is reached, by `(5, 2)`, though `(_, a)`
     would match that value too. A guarded arm's alternatives are checked
     the same way (item 7's guard).
   - **There is no flat switch before the inner chains** (item 5): MIR's
     size did not call for one.
7. **Where the build departs from spec §4, or from 5.0a:**
   - **A flat `let` of a one-variant sum goes through the flat switch,**
     `let Wrap(a, b) = w` testing the one tag once, where spec §4.6 says
     `let` and `for` destructuring lower to projections alone. A
     destructuring `let` is a one-arm `match`, so it takes the path any
     match of its shape takes; a nested or tuple `let` lowers by chains,
     where a sum with one variant gets no tag test.
   - **A bare name inside a variant pattern that names a variant matches
     that variant, as in Rust** (the user's ruling of 2026-10-28).
     `Some(Red)` bound a local named `Red` before 5.0b, when a variant's
     fields could only be names or `_`, and matched every `Some`; it is now
     the variant, by the rule a bare name at the top of a pattern always
     followed. So a program can run differently: in `match c { Some(Red)
     => 1, _ => 2 }` a `Some(Green)` took the first arm and now takes the
     second. A name that is a variant of another type is E0001 there, with
     a note: a name inside a pattern that names a variant matches that
     variant; to bind a value, use a lower-case name; a bare variant that
     has fields, `Some(Leaf)`, is E0016. It is the one place 5.0b changes
     what an existing program means; no repository program wrote such a
     pattern.
   - **E0104, a new warning, marks a pattern binding whose name starts with
     an upper-case letter** (the same ruling): such a name is most often a
     variant misspelt or not in scope, `Some(Rde)`, and as a binding it
     matches every value. A constant's name gets its own note, to match it
     with a guard, and a name near a variant of a sum the matched value's
     type unifies with a "did you mean"; a variant of the matched sum that
     is not in scope gets a note to write `Tok::Eof` or import it. A name
     that starts with `_` is not warned about, a leading `_` being the way
     to say a name is meant, nor a plain `let X` or `for X`, which is not a
     pattern.
   - **A guard on an or-pattern runs once, with the bindings of the first
     alternative that matches, and a false guard goes on to the next arm,**
     as spec §4.6 says. Rust instead tries the later alternatives when the
     guard is false: `match (1, 9) { (a, _) | (_, a) if a > 5 => "big",
     _ => "small" }` gives `small` in Nova and `big` in Rust. E0021 marks
     such an alternative, `(_, a)` here, which no value reaches, with a
     note that the guard runs once.
   - **A name bound twice in one pattern, `(x, x)`, is unchanged from
     before 5.0b:** both bind, the second shadowing the first, with no
     error.
   - **E0103 warns at a newline-rule split only where the old reading
     could have been well typed** (the user's ruling of 2026-10-27, which
     narrows the 5.0a plan's decision 37 for 5.0b on): before `(` only
     when the operand the old reading would have extended is a function,
     before `[` only when it is an array, before `-` only when it is an
     `Int` or a `Float`, and wherever its type is not yet known. That
     operand is the rightmost one, read through a `return`'s value, an
     assignment's, a compound assignment to an element or a field, and a
     closure's body, which the old reading extended (`let f = |x: Int| x`
     then `- 1` read as `|x: Int| x - 1`); a `let`'s value is judged on its
     own. A split after a bare `return` or `break` warns as before. So
     `g()` then a tuple `(a, b)` on the next line does not warn unless `g`
     returns a function.
   - **No `-` in a pattern:** the parser reads none, as before 5.0b, so a
     negative literal or range end does not parse (P0001), at the start of
     an arm as anywhere.
   - **`let mut P` makes every binding of `P` mutable,** since `mut` does
     not parse inside a pattern.

## Consequences

- **Chains test more at run time than a tree would:** a tag is loaded and
  compared once per arm that names it, where a tree tests it once. They
  duplicate no arm body and no test, so MIR grows with the patterns' size:
  a 14-arm, three-level match over a token is 190 statements in 67 blocks.
  A flat switch before the chains, or a tree, can come later without a
  language change, when N's profile asks for it.
- **The chains port simply to N in 5.4,** and keep every flat match's MIR,
  so no golden output or IR test moved when they landed.
- **A tuple costs an allocation,** as a record does; N returns pairs from
  hot functions, which a later flattening could serve.
- **A full-range match needs `_`:** `i64::MIN..=i64::MAX` alone, or every
  `Char` range, is reported non-exhaustive; overlapping ranges are not
  reported.
- **A program with `Some(None)` as a nested pattern** checks, and can run,
  differently: it matched any `Some` before, and matches `Some(None)` alone
  now. Exhaustiveness reports the values left uncovered only when no
  catch-all follows; behind a `_` the other values silently take the later
  arm. E0104 catches the misspelt case, not this one, whose name is a real
  variant.
- **Usefulness is exponential in or-patterns:** they are expanded into
  their alternatives, so a tuple of sixteen `true | false` alternatives
  takes 14 s to check in a debug build (twelve 0.6 s, fourteen 2.7 s), the
  work doubling with each two-way or-pattern. No program of the repository
  comes near; a lazy expansion can come later.
- **`Map::entries` and `channel` returning a tuple** stay out of 5.0 (spec
  §11; ADR 0017).

## References

- `docs/superpowers/specs/2026-10-26-phase-5-0-the-language-and-std-a-compiler-needs-design.md`
  §4, §6, §10 risks 2 and 3, §11 and §12 items 11 to 17.
- `nova-spec/11-PARSER.md` §2, `12-TYPESYSTEM.md` §3 and §8,
  `13-RUNTIME.md` §2.2, `14-CODEGEN.md` §4 and §6.2, and their notes of
  2026-10-27.
- `crates/nova-typeck/src/usefulness.rs`, `crates/nova-typeck/src/check/patterns.rs`,
  `crates/nova-mir/src/lower.rs` (`lower_chain_arms`).
- ADR 0017, `channel` without tuples; ADR 0048, Phase 5's scope.
