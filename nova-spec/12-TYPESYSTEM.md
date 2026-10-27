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
| E0022 | Refutable pattern in `let` |
| E0030 | Recursive type without indirection |
| E0031 | Cycle in trait inheritance |
| E0040 | Cannot return reference to local |
| E0050 | Async/await mismatch |
| E0098 | `?` needs an `Option` or a `Result` (added 2026-10-26, Phase 5.0a) |
| E0099 | `?` cannot return from this function (added 2026-10-26, Phase 5.0a) |
| E0102 | A variant hides one of std's; a warning (added 2026-10-26, Phase 5.0a) |
| E0103 | This value is not used, or a line break ended a statement; a warning (added 2026-10-26, Phase 5.0a) |

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
before it would have continued, unless a `;` is written there. E0100 and
E0101 are 5.0b's.

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
