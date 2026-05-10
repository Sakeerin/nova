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

---

## 8. Error Catalog (sample — full list grows)

| Code | Title |
|---|---|
| E0001 | Cannot find name in scope |
| E0002 | Duplicate definition |
| E0003 | Private item access |
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

Each gets a fully detailed error in `nova-diagnostics::messages` with example, suggestion, and link.

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
