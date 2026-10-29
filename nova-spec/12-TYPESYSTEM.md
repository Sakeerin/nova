# 12 — Type System Specification

> Crate: `nova-typeck`, `nova-resolver`
> Phase: 1
> Depends on: `nova-ast`, `nova-diagnostics`

---

## 1. Goals

- **Sound** static typing (no runtime type errors from compiled code, except `panic!`)
- **Inferred** locally; explicit at item boundaries
- **Generics** via monomorphization
- **Traits** for polymorphism (no inheritance)
- **Sum types** with exhaustive pattern matching

Algorithm: **Hindley-Milner** core + **trait constraints** (similar to Rust's HM-like inference, simplified).

---

## 2. Pipeline Position

```
AST → [Resolver] → resolved AST → [TypeCheck] → typed AST (HIR)
```

**Resolver** (`nova-resolver`):
- Resolves names: `foo` → `module::path::foo`
- Builds module graph
- Reports `unresolved name`, `duplicate definition`, `private item access`
- Outputs: `ResolvedAst` where every `Path` is fully qualified

**TypeChecker** (`nova-typeck`):
- Walks ResolvedAst
- Generates type variables and constraints
- Solves constraints via unification
- Resolves trait method calls
- Outputs: `TypedHir` where every expression has a `Type`

---

## 3. Core Type Representation

In `crates/nova-typeck/src/types.rs`:

```rust
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Ty {
    // Primitives
    Int,         // i64 default
    Int8, Int16, Int32, Int64,
    UInt8, UInt16, UInt32, UInt64,
    Float,       // f64 default
    Float32, Float64,
    Bool,
    Char,
    String,
    Unit,        // ()

    // Composite
    Tuple(Vec<Ty>),
    Array(Box<Ty>),
    Ref { is_mut: bool, inner: Box<Ty> },
    Fn { params: Vec<Ty>, ret: Box<Ty>, is_async: bool },

    // User-defined
    Record { def_id: DefId, args: Vec<Ty> },
    Sum { def_id: DefId, args: Vec<Ty> },

    // Generics
    Param(TypeParamId),       // T
    Var(TypeVarId),           // ?T (inference variable)

    // Special
    Never,                    // ! (return, panic, infinite loop)
    Error,                    // type error placeholder, suppresses cascading errors
}
```

**Amended 2026-10-27 (branch `claude/loving-archimedes-frtf5q-50b`):**
`Ty::Tuple(Vec<Ty>)` is built
(`docs/superpowers/specs/2026-10-26-phase-5-0-the-language-and-std-a-compiler-needs-design.md`
§4.1), as `nova-hir`'s `Ty::Tuple`, for one element or more: `()` stays
`Unit`, and `(T)` is `T`. **Tuples are anonymous records** (the user's
decision 3, 2026-10-26): two tuple types are the same when their element
types are, element by element and of the same arity; they unify, substitute,
normalize and match an impl's self type through their elements, and serve as
type arguments, in generic functions, closures, `Option`, `Vec` and across
`.await`. **A tuple has no impls:** it has no `TyHead`, so no `impl` can name
one (E0010, "impl blocks are only supported on named types"), no method
resolves on one (E0014), and `==`, interpolation and a `Hash`, `Eq` or
`Display` bound refuse it (E0013, with a note that a tuple has no impls); so
a tuple is not a `Map` key. `extern "C"` refuses it, as any type but `Int`,
`Float` and `Bool` (E0900). It displays as written: `(Int, String)`, and
`(Int,)` for a tuple of one. `t.N` reads element `N`, counted from 0, and
`t.N = e` and `t.N op= e` write it through a mutable binding, by a record
field's rule.

---

## 4. Inference Algorithm

### 4.1 Type Variables
- Allocated fresh from a counter: `TypeVarId(u32)`
- Stored in `InferContext` with possible substitution

### 4.2 Constraint Generation
Walk AST, generate:
- **Equality:** `T1 = T2` (e.g. binding to value type)
- **Subtype/Coerce:** rare — only for references and never-type
- **Trait bound:** `T: Trait`

### 4.3 Unification
Standard Robinson:
- `Var(v) = T` → substitute (with occurs check)
- `Record(d, args1) = Record(d, args2)` → unify args pairwise
- Otherwise → mismatch error

### 4.4 Generalization (limited)
- Functions are explicitly generic (no let-polymorphism for closures)
- Top-level fn signatures fully annotated
- Local `let` does NOT generalize (avoids ML-style let-polymorphism complications)

### 4.5 Defaulting
After unification, unresolved numeric variables default:
- Integer literal var → `Int`
- Float literal var → `Float`
- Otherwise unresolved → "cannot infer type" error with annotation suggestion

---

## 5. Traits & Resolution

### 5.1 Trait Definition
```nova
trait Display {
    fn fmt(self) -> String
}
```

Stored as `TraitDef { id, methods: Vec<MethodSig>, supers: Vec<TraitRef> }`.

### 5.2 Impl Lookup
- Build `ImplTable: Map<(TraitId, TyHead), Vec<ImplId>>` indexed by trait + type head
- For trait method call `x.fmt()`:
  1. Resolve `Display::fmt` to method
  2. Get receiver type `Ty(x)`
  3. Look up impl in table
  4. If multiple match → ambiguity error
  5. If none match → "no impl of `Display` for `Ty(x)`" error

### 5.3 Coherence (Orphan Rules)
- An `impl Trait for Type` is allowed only if either `Trait` or `Type` is defined in current package
- Prevents conflicting impls across packages

### 5.4 Trait Bounds in Generics
```nova
fn print_all<T: Display>(items: [T]) { ... }
```
- During monomorphization, check that concrete `T` has `Display` impl in scope
- Otherwise error at call site with span pointing to argument

---

## 6. Sum Types & Pattern Exhaustiveness

### 6.1 Representation
```nova
type Option<T> = | Some(T) | None
```
→ `Sum { def_id: Option, variants: [Some(T), None], args: [T] }`

### 6.2 Match Exhaustiveness
Algorithm: Maranget's "Compiling Pattern Matching to Good Decision Trees" (also used in Rust).

For `match expr { ... }`:
1. Build matrix of patterns
2. Check `usefulness` of each row (warn dead arms)
3. Check that an empty matrix after specialization → exhaustive
4. If not, generate witness pattern showing missing case

Examples:
```nova
match opt {
    Some(x) => x,
    // ERROR: missing `None`
    // suggestion: add arm `None => /* ... */,`
}
```

### 6.3 Refutability
- `let pattern = expr` requires irrefutable pattern (or `let else` form — v2)
- `match` arms can be refutable

---

## 7. Built-in Trait Hierarchy

Implement these in std/core during Phase 2:

```
Copy        — bitwise copyable types (Int, Bool, Float, etc.)
Clone       — explicit duplicate
Eq, Ord     — equality, ordering
Hash        — hashable
Display     — user-facing string
Debug       — debug string
Iterator    — iter protocol
IntoIter, FromIter
Default     — has default value
Drop        — destructor (custom cleanup, GC interaction)
Add, Sub, Mul, Div, Rem, Neg
BitAnd, BitOr, BitXor, Shl, Shr, Not
Index, IndexMut
Future      — async result
```

`Copy` and `Clone` are auto-derivable: `@derive(Copy, Clone)` attribute.

**Amended 2026-10-26 (branch `claude/loving-archimedes-frtf5q-50`):** the
operator traits above are not std's yet: `binary_result_ty` decides each
operator by its operands' types. 5.0a adds `String + String`, a new `String`
holding `a` then `b`, which MIR lowers to the runtime's concatenation; `+=`
follows on a local, a field or an element. `String + Char` and `String +
Int` stay E0010, with a note that suggests interpolation
(`docs/superpowers/specs/2026-10-26-phase-5-0-the-language-and-std-a-compiler-needs-design.md`
§3.6).

**Amended 2026-10-26 (branch `claude/loving-archimedes-frtf5q-50`):** `e?`
works on std/core's `Option` and `Result`, found by their `DefId`s, not by
the names in scope
(`docs/superpowers/specs/2026-10-26-phase-5-0-the-language-and-std-a-compiler-needs-design.md`
§3.3). On `Option<T>`, `Some(v)` gives `v` and `None` returns `None`; on
`Result<T, E>`, `Ok(v)` gives `v` and `Err(x)` returns `Err(x)`, re-wrapped.
**Nothing is converted:** the enclosing function, the nearest `fn` or
closure, must return the same sum with the same `E`, and `map_err` converts.
There is no `From` trait, since generic traits do not exist. The checker
desugars `e?` to a flat `match`, so no backend changes. Its errors are
E0098, E0099, and E0010 with a note pointing at `map_err` when the error
types differ.

---

## 8. Error Catalog (sample — full list grows)

| Code | Title |
|---|---|
| E0001 | Cannot find name in scope |
| E0002 | Duplicate definition |
| E0003 | Private item access |
| E0004 | An import names both a module of the package and a dependency (added 2026-10-08, Phase 3.3a, ADR 0030) |
| E0010 | Type mismatch |
| E0011 | Cannot infer type |
| E0012 | Generic argument count mismatch |
| E0013 | Trait bound not satisfied |
| E0014 | No method `m` on type `T` |
| E0015 | Ambiguous method call |
| E0020 | Non-exhaustive match |
| E0021 | Unreachable pattern |
| E0022 | Refutable pattern in `let`, or in `for` (`for` added 2026-10-27, Phase 5.0b) |
| E0030 | Recursive type without indirection |
| E0031 | Cycle in trait inheritance |
| E0040 | Cannot return reference to local |
| E0050 | Async/await mismatch |
| E0098 | `?` needs an `Option` or a `Result` (added 2026-10-26, Phase 5.0a) |
| E0099 | `?` cannot return from this function (added 2026-10-26, Phase 5.0a) |
| E0100 | The alternatives of an or-pattern bind different names, or a name at different types (added 2026-10-27, Phase 5.0b) |
| E0101 | This range pattern matches nothing (added 2026-10-27, Phase 5.0b) |
| E0102 | A variant hides one of std's; a warning (added 2026-10-26, Phase 5.0a) |
| E0103 | This value is not used, or a line break ended a statement; a warning (added 2026-10-26, Phase 5.0a) |
| E0104 | A pattern binds a name that starts with an upper-case letter, not a plain `let X` or `for X`; a warning (added 2026-10-28, Phase 5.0b) |

Each gets a fully detailed error in `nova-diagnostics::messages` with example, suggestion, and link.

**Amended 2026-10-26 (branch `claude/loving-archimedes-frtf5q-50`):** the
four codes 5.0a adds
(`docs/superpowers/specs/2026-10-26-phase-5-0-the-language-and-std-a-compiler-needs-design.md`
§6). E0098 is `?` on any other type, a program's own sum named `Option`
included; a note names the type. E0099 is `?` in a function whose return
type is neither sum, or the other of the two; a note suggests `.ok_or(err)?`
for an `Option` in a function that returns a `Result`, and another says that
`main` and `@test` functions return `()`, another that the function's
`Option` or `Result` is the program's own; `?` in a constant's initializer
is E0099 too, since a constant is no function. E0102 warns at a variant of the
package's own sum, or a path dependency's, whose name is also a variant std
brings into every module; std's own modules are never warned about. E0103
warns at a statement, not a block's tail, whose value can only be discarded:
a unary or binary operator other than an assignment, `&&` or `||`, a
literal, a name, or a parenthesised, tuple or array expression; and at each
place the newline rule (`11-PARSER.md` §2) ends a statement the parser
before it would have continued, unless a `;` is written there. E0100,
E0101 and E0104 are 5.0b's.

**Amended 2026-10-27 (branch `claude/loving-archimedes-frtf5q-50b`):** tuples
(`docs/superpowers/specs/2026-10-26-phase-5-0-the-language-and-std-a-compiler-needs-design.md`
§4.1, §4.2) take no new code. E0010 is a tuple of the wrong arity or
element type, and an `impl` on a tuple; E0011 an element whose type cannot
be inferred, and `t.N` on a value whose type is not yet known; E0013 `==`,
interpolation, or a `Hash`, `Eq`, `Display`, `Debug` or other bound on a
tuple, with the note that a tuple has no impls; E0014 an index past the
end ("no field `2` on `(Int, Int)`"), a name on a tuple, `.N` on a type
that is not a tuple, or a method on a tuple. P0001 is an index that is not
plain decimal digits (`11-PARSER.md` §2).

**Amended 2026-10-27 (branch `claude/loving-archimedes-frtf5q-50b`):** the
codes 5.0b's patterns add
(`docs/superpowers/specs/2026-10-26-phase-5-0-the-language-and-std-a-compiler-needs-design.md`
§4.3, §4.4, §6), and the codes they reuse. **E0100,** "the alternatives
bind different names": every alternative of an or-pattern binds the same
names, which are the same locals, at the same types. The label names the
binding: "`x` is missing from this alternative", on the alternative that
lacks it, or "`y` is not bound in the first alternative", on the name; a
name at two types is E0100 too, "the alternatives bind `x` at different
types", with both types in the label. **E0101,** "this range matches
nothing": `lo > hi`, or `lo >= hi` with `..`, in an `Int` or `Char` range.
**E0010** is a pattern of the wrong type or arity, a tuple pattern of the
wrong arity ("this pattern has 3 elements but the matched value is a tuple
of 2, `(Int, Int)`"), a range over a type that is not `Int` or `Char`, and
a guard that is not a `Bool`. **E0020** names nested witnesses
(`Some(Err(_))`, `(None, false)`), and a range in a `Char` column as
`Char`s, `'a'..='z'`; an empty `match` names every variant, as before 5.0b.
**E0021** is an unreachable arm, as before, and an unreachable alternative
of an or-pattern in a reachable arm, "unreachable alternative", on the
alternative alone: taken in the order the test chain tries them, every value
that matches the arm with it is matched by an earlier arm or takes an
earlier alternative; one inside an alternative already reported is not
reported again. In a guarded arm, an alternative that only a false guard
could lead to gets a note that the guard runs once. **E0022** is a refutable
pattern in a `let` or a `for`, decided by usefulness, with the
values it misses ("`None` is not matched"). Usefulness takes a tuple as a
type with one constructor, expands or-patterns, and takes each `Int`,
`Char` and `String` literal and each range as a constructor of its own:
ranges are not split, so a match on them needs `_` or a name, and an arm
that earlier ranges cover is not reported. **E0001** inside a pattern: a
bare name that resolves to a variant is that variant at any depth, as at the
top of a pattern before 5.0b, so `Some(None)` is `Some` of `None`; a name
that is a variant of another type is E0001, "`Eof` is not a variant of the
matched type", with the note "a name inside a pattern that names a variant
matches that variant; to bind a value, use a lower-case name", and a bare
variant that has fields, `Some(Leaf)`, is E0016. **E0104,** a warning: a
pattern binding whose name starts with an upper-case letter, "`Rde` binds a
value: no variant of that name is in scope", with a note that such a name
reads as a variant and that a lower-case name binds, or, for a constant's
name, that a pattern cannot match a constant and a guard can, and "did you
mean `Empty`?" with its fix for a name near a variant of a sum the matched
value's type unifies with; for a variant of the matched sum that is not in
scope, a note to write `Tok::Eof` or import it. A name that starts
with `_` is not warned about, nor a plain `let X = …` or `for X in …`,
which binds a name and not a pattern (the user's ruling of 2026-10-28). An
arm whose pattern was an error is not read for reachability or
exhaustiveness, so no second diagnostic follows the first.

**Amended 2026-10-27 (branch `claude/loving-archimedes-frtf5q-50b`):**
E0103 at a newline-rule split, narrowed by the user's ruling of 2026-10-27
for 5.0b onward: before a `(`, `[` or `-`, it warns only where the old
reading could have been well typed, by the type of the operand the old
reading would have extended (the previous statement's value, its rightmost
operand of a binary operator, read through a `return`, an assignment, a
compound assignment to an element or a field, and a closure, whose body the
old reading extended; a `let`'s value): a function before `(`, an array
before `[`, an `Int` or a `Float` before `-`, or a type not yet known. So
`g()` returning `Int`, then `(a, b)` on the next line, does not warn. A
split after a bare `return` or `break` warns as before.

**Amended 2026-10-29 (branch `claude/loving-archimedes-frtf5q-50b`), after
the review:** a guard on an or-pattern runs once, with the bindings of the
first alternative that matches, and when it is false, matching goes on with
the next arm
(`docs/superpowers/specs/2026-10-26-phase-5-0-the-language-and-std-a-compiler-needs-design.md`
§4.6). Rust instead tries the later alternatives when the guard is false,
so `match (1, 9) { (a, _) | (_, a) if a > 5 => "big", _ => "small" }` gives
`small` in Nova and `big` in Rust. E0021 marks the alternative a guard
cannot reach, `(_, a)` here, with that note.

---

## 9. Examples — Test Cases

### 9.1 Should pass
```nova
fn add<T: Add<T, Output = T>>(a: T, b: T) -> T { a + b }

let x = add(1, 2)        // x: Int
let y = add(1.0, 2.0)    // y: Float
```

### 9.2 Should fail with E0010
```nova
let x: Int = "hello"
// type mismatch: expected Int, found String
```

### 9.3 Should fail with E0020
```nova
match opt {
    Some(x) => x,
}
// non-exhaustive: pattern `None` not covered
```

### 9.4 Should fail with E0013
```nova
fn print_it<T: Display>(x: T) { println(x.fmt()) }

record Foo { a: Int }
print_it(Foo { a: 1 })
// trait bound `Foo: Display` not satisfied
// suggestion: implement `Display` for `Foo`
```

---

## 10. Implementation Order

1. `Ty` definition + `InferContext`
2. Unification (with occurs check)
3. Walk literals, locals, simple fn calls
4. Generic instantiation (fresh vars per call)
5. Records and field access
6. Sum types and pattern matching (start non-exhaustive)
7. Trait resolution + impl lookup
8. Exhaustiveness algorithm
9. Async/await typing (`async fn` returns `Future<T>`, `.await` unwraps)
10. Error type & error recovery (suppress cascading)

Each step ships with snapshot tests in `tests/typecheck/`.
