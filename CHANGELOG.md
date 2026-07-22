# Changelog

All notable changes to Nova are documented here.

Format follows [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).
Nova uses [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

---

## [Unreleased]

### Added (Phase 1 — MVP compiler, in progress)
- `nova run <file>`: compile and execute Nova programs natively via the
  Cranelift JIT; `nova check <file>`: type-check only
- `nova build <file> [-o out]`: compile to a standalone native executable —
  Cranelift object emission with an exported C `main` wrapper, linked
  against the `nova-runtime` static library via the platform linker
  (MSVC `link.exe` through cc-rs on Windows, `cc` elsewhere); gate
  programs produce ~130 KB executables
- `nova-resolver`: item-level name resolution (functions, sum types,
  variants), builtin prelude (`println`, `print`), E0002 duplicates
- `nova-typeck` + `nova-hir`: Hindley-Milner inference with occurs check,
  explicit generics at function boundaries, sum types with minimal
  exhaustiveness checking (E0020), typed & desugared HIR output
- `nova-mir`: monomorphization reachable from `main`, CFG lowering,
  match compilation to switches, short-circuit lowering
- `nova-runtime`: C-ABI strings, console output, sum allocation
  (leaking allocator pending GC — ADR 0002)
- `nova-codegen-cranelift`: MIR → native code via cranelift-jit
- Records: declarations, literals (explicit fields, shorthand, `..base`
  spread), field access, generic records; boxed as tagless heap structs
- Traits: inherent methods, trait declarations with required and default
  methods, trait impls, method-call resolution by receiver type
  (E0015 on ambiguity), generic trait bounds verified at monomorphization
  (E0013), impl conformance (E0070/E0071), static dispatch; string
  interpolation bridges to a user `Display` (`fmt(self) -> String`)
- For loops over integer ranges (`for i in a..b` / `a..=b`), desugared to
  a counter-driven `while`
- Closures (`|x| body`) with by-value capture, and bare functions used as
  values: both compile to fat pointers `{ code, env }` with an env-first
  ABI; lifted to standalone functions and monomorphized like generics
- `break` and `continue` in `while`/`for` loops (E0080 outside a loop);
  `continue` in a `for` still advances the counter
- Top-level `const NAME: T = value` (compiled as a zero-arg function,
  referenced by call); constants may reference other constants; a cyclic
  constant is reported as E0081
- Arrays `[T]`: literals `[a, b, c]`, indexing `arr[i]`, element assignment
  `arr[i] = v` (mutable base required), and `arr.len()`; out-of-bounds
  access aborts with a message (heap layout `{ len, elems… }`)
- Generic impl blocks: `impl<T> Box<T> { … }` (inherent) and
  `impl<T> Trait for Box<T> { … }` (trait), with the impl's type parameters
  usable in method signatures and bodies; a method is monomorphized per
  instance by recovering the impl's type arguments from the receiver type.
  Conditional impls `impl<T: Bound> Trait for Box<T>` are supported — the
  bound on the impl's parameter is verified at monomorphization (E0013),
  including transitively through nested generic impls (`where` clauses on
  impls remain unsupported)
- Gate programs verified end-to-end: hello-world, fibonacci,
  match-on-enum, generic functions, records, traits, for-loops, closures,
  break/continue, constants, arrays, generic impls (e2e stdout tests under
  both `nova run` and `nova build`)

### Fixed
- Lexer: leading whitespace in a string segment directly after `${expr}`
  was skipped when resuming string mode
- Lexer: position drifted into comment text because the logos wrapper
  advanced by the token length, ignoring skipped comment/whitespace
  trivia — any program with a comment failed to parse
- Lexer: a comment immediately before a string (spurious error cascade)
  or raw string (silent mis-lex to `Ident("r")` + plain string) — comments
  are now skipped before literal dispatch (adversarial review)
- Typeck: a trait impl whose method signature diverged from the trait
  declaration (arity, parameter, or return type) was accepted and
  miscompiled — a wrong parameter type was memory-unsafe; now `E0072`
  (adversarial review)
- `nova check` now runs monomorphization so it catches unsatisfied trait
  bounds (`E0013`) that previously only `nova run`/`nova build` rejected
  (adversarial review)
- Record literal field initializers now evaluate in source order (adversarial review)
- Typeck: closure return type is resolved through inference (closure bodies
  previously returned `()` and discarded their result)
- Typeck: closures now capture all referenced enclosing locals — assignment
  targets and called function values were missed, causing miscompiles or a
  compiler panic (adversarial review)
- Typeck: `for` loops use an independent hidden counter (assigning the loop
  variable is rejected, not silently corrupting the trip count), unscoped
  counter locals (no name capture), and an overflow-safe inclusive form
  (an inclusive range ending at `Int::MAX` no longer loops forever)
  (adversarial review)
- Typeck: an `if`/`match` with a diverging branch (`return`/`break`/
  `continue`) is typed by its non-diverging branch instead of `Never`, and
  the `while` lowering guards a diverging condition — a `Never`-typed
  condition (e.g. `while (if c { return x } else { b }) {}`) previously
  crashed codegen with an internal error (adversarial review)
- Typeck: `break`/`continue` in a loop's own condition now target that loop
  (were rejected as "outside a loop" or mis-scoped) (adversarial review)
- Typeck: a function-typed value that is not a local (e.g. a fn-typed
  constant `CONST(args)`, or a fn returned from a field/call) can now be
  called directly instead of erroring with E0900 (adversarial review)
- Typeck: element assignment `arr[i] = v` now checks the mutability of the
  place's root binding, walking through field and nested-index projections —
  `rec.data[0] = v` and `grid[0][1] = v` on an immutable binding, and
  `make()[0] = v` on a temporary, were silently accepted and mutated
  immutable heap storage; now `E0060` (adversarial review)
- Typeck: a restricted inherent method (`impl<T> Pair<T, T> { … }`) no longer
  shadows an applicable trait method for a receiver it does not fit (e.g.
  `Pair<Int, String>`) — method resolution now falls through to the trait impl
  instead of rejecting the call, and the string-interpolation `Display` bridge
  is likewise no longer blocked (adversarial review)
- Monomorphization: the trait-bound satisfaction check no longer has a
  recursion depth cap that could accept an unsatisfiable bound for a very
  deeply nested conditional-impl type; the recursion is well-founded on the
  finite structure of the type, so deep nests are checked exactly
  (adversarial review)

### Remaining for Phase 1 gate completion
- LLVM release backend (`nova build --release`), full Maranget
  exhaustiveness

## [0.0.0] - 2026-05-10

Phase 0 (Foundation) milestone. Gate verified: `examples/01-hello-world` and
`examples/02-fibonacci` parse to AST with zero errors.

### Added
- Initial workspace setup (Phase 0)
- `nova-diagnostics`: error reporting infrastructure with codespan-reporting
- `nova-lexer`: full token set for Nova source files (logos-based)
- `nova-ast`: AST node type definitions
- `nova-parser`: recursive descent + Pratt parser (chumsky-based)
- `nova-cli`: `nova parse <file>` command for parser testing
- CI workflow (cargo test, fmt, clippy)
- Snapshot testing harness via `insta`
- Example files: hello-world, fibonacci
