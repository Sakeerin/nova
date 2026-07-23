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
- Match exhaustiveness and reachability via Maranget's usefulness algorithm:
  a `match` is non-exhaustive (`E0020`) when a wildcard row is still useful
  against the arms, and the diagnostic names witness patterns for the uncovered
  values (`Some(_)`, `false`, `_`); an arm is unreachable (`E0021`) when it is
  useless against the earlier arms. This fixes `match` on `Bool` being rejected
  when both `true` and `false` are covered, and detects redundant arms that the
  previous catch-all-only check missed
- Generic impl blocks: `impl<T> Box<T> { … }` (inherent) and
  `impl<T> Trait for Box<T> { … }` (trait), with the impl's type parameters
  usable in method signatures and bodies; a method is monomorphized per
  instance by recovering the impl's type arguments from the receiver type.
  Conditional impls `impl<T: Bound> Trait for Box<T>` are supported — the
  bound on the impl's parameter is verified at monomorphization (E0013),
  including transitively through nested generic impls (`where` clauses on
  impls remain unsupported)
- `nova build --release`: optimizing build through a new LLVM backend
  (`nova-codegen-llvm`) that emits textual LLVM IR from MIR and compiles it
  with a discovered LLVM toolchain (`clang`, or `llc`, at `-O2`; override with
  `NOVA_CLANG`/`NOVA_LLC`), then links via the same platform linker as the
  debug build. The IR mirrors the Cranelift backend's layouts and runtime ABI,
  so a program behaves identically across `nova run`, `nova build`, and
  `nova build --release`. Requires LLVM ≥ 15 (opaque pointers); with no
  toolchain found, the generated `.ll` is left in place with a clear message
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
- Impl selection is now structural at every site. `resolve_method_full` (trait
  dispatch) and the monomorphization bound check selected the *first* impl
  sharing a type head, so a program with two non-overlapping impls for the same
  head (`impl Foo for Pair<Int, Bool>` + `impl Foo for Pair<Int, String>`) was
  accepted or rejected depending on declaration order; both now scan for the
  impl that structurally fits (adversarial review)
- Impl methods are mangled by their full self type, not just its head, so two
  concrete impls sharing a head (`Pair<Int, Bool>` vs `Pair<Int, Int>`) no
  longer collide to one symbol and miscompile each other's calls (adversarial
  review)
- Overlapping implementations are rejected (`E0074`): two trait impls of the
  same trait whose self types share a ground instance, or two inherent impls
  that overlap and define the same method (Phase 1 has no specialization);
  previously dispatch silently depended on declaration order (adversarial
  review)
- An impl type parameter that never appears in the self type is rejected
  (`E0073`) instead of leaking an uninferrable variable that made every method
  on the impl uncallable (adversarial review)
- Exhaustiveness: an empty `match` on a value of generic type (`match x { }`
  where `x: T`) was silently accepted and trapped at runtime; a match with no
  arms is now reported non-exhaustive (`E0020`) for any inhabited scrutinee
  (adversarial review)
- A bare identifier pattern that names a nullary variant of a *different* sum
  type is now rejected (`E0001`) rather than silently treated as a catch-all
  binding, which had masked uncovered cases and produced a spurious
  unreachable-arm warning — matching the `Path`/`TupleStruct` pattern arms
  (adversarial review)
- LLVM backend: a `match` on a `Bool` emitted `switch i64` over the `i8`
  scrutinee, producing a type-mismatched module that LLVM rejects (so
  `--release` failed for any boolean match); the switch now uses the
  discriminant's own type (adversarial review)
- `nova build`: intermediate object/IR files are now named so they can never
  alias the `-o` output path — previously `-o out.ll` (or `-o out.obj`) made an
  intermediate the output file and deleted the built binary on success
  (adversarial review)

### Remaining for Phase 1 gate completion
- Real garbage collector (the runtime uses a leaking allocator — see
  `docs/adr/0002-phase1-leaking-allocator.md`)

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
