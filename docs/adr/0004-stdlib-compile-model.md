# ADR 0004 — Standard library compile model

## Status

Accepted (2026-07-25). First increment of Phase 2.1 (`std/core`, Nova's first
standard-library module).

## Context

`Option`/`Result` have existed since the Phase 1 prelude (see `22e7a64`) as a
two-line Rust string constant (`PRELUDE_SRC`) lexed and parsed with
`FileId::DUMMY` and a `debug_assert!` guarding lex/parse failure. That was
adequate for two fixed lines nobody but the compiler author could break.

Phase 2.1 grows `std/core` into real Nova source — methods and traits on
`Option`/`Result`, written by hand like any other Nova code (Tasks 8-9). Two
problems follow directly from that growth:

- A `debug_assert!` compiles to nothing in a release build. A syntax error
  introduced into a growing `std/core` would panic a debug compiler build but
  silently produce an *empty* implicit module in a release one — the exact
  "silently wrong" failure mode `12-TYPESYSTEM.md`-adjacent diagnostics work
  in this codebase is built to avoid everywhere else.
- `FileId::DUMMY` carries no source text or name, so any diagnostic inside
  `std/core` (once it's big enough to plausibly contain a bug) would point at
  nothing a developer could open.

ADR 0003 established the module system this builds on: one file is one
module, the driver merges modules into a single `File` for whole-program
compilation, and only the driver owns a `FileDb` (the resolver and typeck
crates work in terms of already-parsed `File`s and are `FileDb`-agnostic).
`std/core` is compiled the same way — as one more module — rather than
inventing a second mechanism for library code.

## Decision

- `std/core` is real Nova source at `std/core/lib.nova` in the repo, not a
  Rust string constant.
- It is embedded into the compiler binary at build time with
  `include_str!("../../../std/core/lib.nova")` (`nova-resolver`'s
  `STD_CORE_SRC`), so the compiler stays a single self-contained executable
  with no runtime filesystem dependency to locate its own standard library.
- It is compiled as an implicit module exactly like a user module: lexed and
  parsed, then appended **last** to the module list, so user module indices
  are unaffected (module 0 is always the first real user module). Its `pub`
  names are then glob-imported into every user module at the **lowest**
  priority, so a user definition of the same name silently shadows it — no
  `import` is needed to use `Option`/`Result`, and no conflict is reported if
  a user happens to define their own.
- The driver — the sole owner of a `FileDb` — registers `STD_CORE_SRC` under
  the synthetic name `<std/core>` and threads the resulting `FileId` into
  `resolve_program(&[ModuleSource], std_core_file: FileId)`. A lex/parse
  failure inside `std/core` is now a normal `Diagnostic` reported against
  that real file, not a `debug_assert!`. The single-module `resolve(&File)`
  wrapper used by tests has no `FileDb` to register into, so it passes
  `FileId::DUMMY` — the same sentinel used throughout the test suite.

## Alternatives considered

- **Disk search path.** Read `std/core/lib.nova` (and later other `std/*`
  modules) from disk at compile time via a search path, the way user
  `import`s are resolved. Rejected for now: it needs qualified/nested import
  paths (`std::core`, `std::collections::hashmap`) that ADR 0003 explicitly
  deferred, and it adds a real deployment failure mode — the compiler binary
  would need its stdlib sources present and locatable (relative to the
  executable, or via an install-time environment variable) on every machine
  it runs on, for no benefit at today's single-file `std/core`. Recorded here
  as the natural next step once the standard library is too large to embed.
- **Precompiled artifact.** Ship a serialized, already-resolved/typechecked
  form of `std/core` instead of source. Rejected: `nova-mir` monomorphizes
  **whole-program** from `main` (ADR 0003), so a generic `Option<T>`/
  `Result<T, E>` cannot be precompiled independently of the instantiations a
  downstream program needs. Doing this soundly needs a serialized HIR format
  plus incremental-compilation infrastructure (cache keys, invalidation) that
  doesn't exist yet — out of scope for Phase 2.1.

## Consequences

- `std/core`'s public names (`Option`, `Result`, and whatever methods/traits
  Tasks 8-9 add) are visible in every module without an `import`, by design —
  `std/core` is a prelude in the traditional sense. It occupies a
  soft-reserved namespace: a name it defines can always be shadowed by a user
  definition, never must be avoided.
- Shadowing a *type or free function* name is silent (lowest-priority glob
  import, no conflict). But a user `impl` adding a method with the same name
  as one `std/core` defines on `Option`/`Result` is a normal inherent-impl
  overlap and is reported as `E0074`, exactly as two colliding user impls
  would be — `std/core`'s own impls get no special immunity from that check.
- The compiler binary embeds the full text of `std/core/lib.nova`, so the
  binary grows with the standard library, and `std/core` must compile
  standalone (it cannot reference any user module). Acceptable at today's
  size; the trigger for revisiting the "disk search path" alternative above.
- Every lex/parse failure in `std/core` is necessarily a compiler bug (its
  source ships with, and is fixed at, compiler build time — no end user can
  edit it), but it is now a debuggable diagnostic against a real file in
  every build profile, not only a debug-only assertion.

## Migration path

The driver-supplied `FileId` is the single seam this decision depends on:
`resolve_program` takes it as a parameter instead of hardcoding
`FileId::DUMMY` or computing one itself, and only the driver can produce a
real one (it owns the only `FileDb`). Swapping the `include_str!` embed for
an on-disk read (the rejected "disk search path" alternative, if `std/core`
later outgrows embedding) changes only where the driver gets `std/core`'s
source text and what name it registers it under — `resolve_program`,
`std_core_module`, and the Nova source in `std/core/lib.nova` itself are
untouched.
