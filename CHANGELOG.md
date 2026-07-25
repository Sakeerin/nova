# Changelog

All notable changes to Nova are documented here.

Format follows [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).
Nova uses [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

---

## [Unreleased]

### Added (Phase 2 — standard library core, in progress)
- Module system (Phase 2.0): multi-file programs with `import`. One file is one
  module (its file stem); `import m` brings all of module `m`'s `pub` items into
  scope, `import m::{a, b}` brings the named ones, and only `pub` items are
  importable — private items stay module-local (ADR 0003). The driver loads the
  entry plus every transitively imported `<name>.nova` beside it; the resolver
  builds per-module namespaces, enforces visibility, and merges all items into
  one program so whole-program monomorphization is unchanged. Cross-module
  records, functions, and traits (including generic bounds over an imported
  trait) work under both backends. Dangling imports and private-item imports
  report `E0001`. (`import as` aliases and qualified `m::name` paths are later
  increments.)
- Method-level generics (Phase 2.0): an inherent method may introduce its own
  generic parameters on top of the impl's — e.g. `impl<T> Box<T> { fn map<U>(self,
  f: fn(T) -> U) -> Box<U> { … } }`. The method's parameters are inferred from
  the call arguments (the impl's from the receiver), inline bounds (`<U: Show>`)
  are enforced at monomorphization, and each concrete instantiation is
  monomorphized separately. Implemented via a flat generic layout — impl
  parameters first, method parameters after — so substitution, bound checking,
  and monomorphization stay uniform. Generic parameters that shadow an impl
  parameter report `E0403`.
- Generic trait methods (Phase 2.0): a trait method may now declare its own
  generic parameters — `trait Mapper { fn remap<U>(self, f: fn(Int) -> U) -> U }`
  — in the trait declaration (required or default-bodied) and in its impls. The
  method's type arguments are inferred per call site (`Self` at `Param(0)`, the
  method's generics after) and threaded into monomorphization so each concrete
  instantiation is compiled separately; inline bounds are enforced at
  monomorphization (`E0013`). An impl method whose generic arity *or* bounds
  disagree with the trait's is rejected (`E0072`), so the trait method signature
  the caller programs against is the contract every impl honors — an impl may
  neither drop nor add a method-generic bound. Dispatch works both on concrete
  receivers and through a `T: Trait` bound in a generic function. `async` on a
  trait-method declaration is rejected (`E0900`) like every other method site.
- `where` clauses (Phase 2.0): an out-of-line spelling of generic bounds on
  functions, impl blocks, and inherent methods — `fn label<T>(x: T) -> String
  where T: Show { … }` is equivalent to the inline `<T: Show>`, and
  `impl<T> Box<T> where T: Show { … }` is the conditional impl `impl<T: Show>`.
  Bounds accumulate on top of any inline bounds and are enforced at
  monomorphization. A `where` clause may only constrain one of the item's own
  type parameters; constraints on concrete/compound types (`where Box<T>:
  Trait`) and on trait methods are rejected with `E0900`.
- Prelude (Phase 2.0): `Option<T> = | Some(T) | None` and `Result<T, E> =
  | Ok(T) | Err(E)` are now built in — available in every module with no import
  or definition. They are compiled as an implicit module and glob-imported into
  every user module, so they use the ordinary generic sum-type and
  monomorphization machinery (and cost nothing when unused). A program may still
  define its own `Option`/`Result`, which shadows the prelude.
- extern / FFI (Phase 2.0): `extern "C" { fn sqrt(x: Float) -> Float }` declares
  external C functions callable like ordinary functions. Symbols resolve against
  the C runtime with no extra configuration — the Cranelift JIT's dlsym fallback
  under `nova run`, and the system linker under `nova build`. The emitted symbol
  is the raw (unmangled) declared name, imported into both backends. Supported:
  the C ABI (`"C"` or omitted) and FFI-safe scalar types — `Int`↔`int64_t`,
  `Float`↔`double`, `Bool`↔`_Bool`, and a unit (`void`) return. Non-scalar types
  (String, records, arrays — GC heap values), other ABIs, and generic/async/
  `where` extern declarations are rejected with `E0900`; symbols that collide
  with the compiler's own (`nova_*`, `main`) are reserved. Note: because `Int`
  is 64-bit, C functions that use narrower integers (32-bit `int`, e.g. `abs`,
  `getchar`) cannot be declared correctly yet and will truncate — declare only
  `int64_t`/`long long`/`double` C functions for now. (Pointers, strings,
  variadics, `link_name`, and fixed-width C integers are later increments.)

### Fixed (Phase 2)
- Cross-module symbol collision: two modules each defining a same-named item
  (function, generic function, or same-named type's inherent method) no longer
  collapse to one symbol at monomorphization. Symbols are now mangled by their
  owning `DefId`, fixing silent wrong-dispatch and a memory-unsafe type
  confusion; qualified/nested import paths (`a::b`) are rejected with `E0900`.
- A generic sum type used as a record field or a sum-variant payload
  (e.g. `record Slot { tag: Option<Int> }`) no longer gets a spurious `E0012`
  "expects 0 type arguments": type arity is precomputed, so it no longer depends
  on collection order (this also fixes forward-referenced generic records).
- Importing a module that exports a name coinciding with a prelude name
  (`Option`/`Result`/`Some`/`None`/`Ok`/`Err`) no longer raises a spurious
  `E0002`: the prelude is glob-imported last, as the lowest-priority binding, so
  a local definition *or* an import of the same name shadows it.
- Nested generic type annotations whose closing brackets abut (`Option<Option<
  Int>>`, `Box<Box<Int>>`) now parse: a glued `>>`/`>>>` token is split when
  closing generic argument lists (the `>>` right-shift operator is unaffected).
- Calling an `extern` function whose C symbol cannot be resolved no longer
  crashes `nova run` with a Rust panic — the JIT's finalize-time panic is caught
  and reported as a clean `E0902`, mirroring the `nova build` linker error.
- Two modules declaring the same C symbol with conflicting signatures now report
  `E0075` instead of crashing codegen / emitting invalid LLVM IR.

## [0.1.0] - 2026-07-23

Phase 1 (MVP compiler) milestone. Gate verified: all gate programs compile and
run via `nova run` (Cranelift JIT) and `nova build` (native executables), the
workspace test suite is green, and `clippy -D warnings` + `cargo fmt --check`
pass.

### Added (Phase 1 — MVP compiler)
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
- Garbage collector: `nova-runtime` now reclaims heap memory with a
  conservative, non-moving mark-and-sweep collector (`gc.rs`), replacing the
  leaking allocator (supersedes ADR 0002). All heap values — records, sums,
  arrays, closures, and strings — route through `gc::alloc`; collection is
  triggered at allocation past a growth threshold. Roots are found by scanning
  the stack plus callee-saved registers (flushed via a small `setjmp` C shim),
  and marking is range-based so interior pointers keep their object alive. It
  needs no codegen support and no external GC library. `NOVA_GC_DEBUG` logs
  collections; `NOVA_GC_STRESS` collects on every allocation (used to validate
  root scanning — the whole e2e suite passes under it). Precise stack bounds
  are implemented on Windows; other platforms retain leak-until-exit for now
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

### Known limitations / follow-ups (not Phase 1 blockers)
- Precise GC stack bounds are implemented on Windows; other platforms skip
  collection (leak-until-exit) until their stack-bounds query is added
- `nova build --release` needs an LLVM toolchain (`clang`/`llc`, ≥ 15) on the
  machine to produce the final binary from the emitted IR
- Spec drift to reconcile in Phase 2: chumsky 0.9 (spec calls for 0.10),
  `salsa` not yet integrated, `fuzz/` targets not yet written

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
