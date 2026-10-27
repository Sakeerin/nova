# ADR 0048 — Phase 5's scope and backend

## Status

Accepted (2026-10-26). Branch `claude/loving-archimedes-frtf5q-50`
(`docs/superpowers/specs/2026-10-26-phase-5-0-the-language-and-std-a-compiler-needs-design.md`
§3.1). The scope, the backend, the `clang` pin and the three parts of 5.0
are the user's decisions of 2026-10-26 (`docs/phase-5-plan.md` §1, and §3
decisions 1 and 2; the 5.0 spec's decisions 1 and 7).

## Context

`nova-spec/00-MASTER-SPEC.md` §3 makes Phase 5 the self-hosting phase: the
Nova compiler written in Nova. `docs/phase-5-plan.md` names the stages:
S0 is the Rust `nova`, N is the Nova compiler's source in `selfhost/`,
S1 = S0(N), S2 = S1(N) and S3 = S2(N). Its gate (5.6) is S1 == S2 byte
for byte, and S2 == S3, on Linux, with one pinned `clang`.

The master spec has N "bind LLVM via `unsafe` extern". Today's Nova has
no pointer FFI and no `unsafe`, and S0's own `--release` already writes
textual LLVM IR and runs `clang`, as `14-CODEGEN.md`'s note of 2026-10-06
recorded when it departed from `inkwell`.

Until 5.0, nothing in CI pinned a `clang` or ran the golden suite through
the LLVM backend: one test built hello world with `--release` where a
`clang` happened to be installed, and the browser job built the blog's
server with the runner's.

## Decision

1. **Scope.** N covers the pipeline from source to `.ll`: the lexer, the
   parser, the resolver, HIR, the type checker, MIR and the textual LLVM
   IR emitter, with the driver and the commands `build` and `check`.
   Diagnostics are compared with S0's by code and span, not by rendered
   text.
2. **What Phase 5 leaves out,** each to the backlog unless a sub-phase's
   spec pulls it back with a ruling that names this ADR:
   - pointer FFI, `unsafe` blocks, linking C libraries, and a binding of
     LLVM's C API;
   - porting `run`, `test`, fmt, the language server, doc, the package
     manager, and the Cranelift and WASM backends;
   - S1 as the shipped `nova`: `v0.5.0` ships the Rust `nova`;
   - the byte gate on macOS and Windows;
   - N built for WASM, which Phase 6's playground needs;
   - trait objects, mutable globals and nested module directories.
3. **The backend.** N writes textual `.ll` and runs `clang` and `cc`, as
   S0's `--release` does, through 5.0c's `std/process`. This departs from
   the master spec's "bind LLVM via `unsafe` extern", as `14-CODEGEN.md`'s
   note of 2026-10-06 departed from `inkwell`. Emitting S0's own text is
   what makes the gate testable.
4. **The `clang` CI pins.** Ubuntu's `clang-18`, 18.1.3, installed with
   `apt-get` on the `ubuntu-24.04` runner, which the `test` job's Linux
   leg now names in place of `ubuntu-latest`. A CI step asserts the
   version, not the test, so a local run with another `clang` still
   works. Under `NOVA_REQUIRE_CLANG=1`, which that leg sets with
   `NOVA_CLANG=clang-18`, `release_golden_suite` builds every golden
   program of `tests/runtime/` but the nine its exclusion table names,
   with `--release`, and runs it, and fails if that `clang` cannot run, or
   if it would skip. Elsewhere it says it did not run and passes.
   When `apt` ships another build, the version step fails; when GitHub
   retires the `ubuntu-24.04` image, the leg finds no runner and never
   starts, so no step runs. Either way a new ADR moves the pin.
5. **5.0's three parts,** which amend the phase plan's 5.0a and 5.0b
   (the 5.0 spec's decision 1):
   - **5.0a, the small language and CI:** this ADR, the golden suite
     through LLVM in CI, `?`, `+=` on fields and elements, the newline
     rule, `String + String`, and the warnings E0102 and E0103;
   - **5.0b, tuples and patterns,** with ADR 0049;
   - **5.0c, std,** with ADR 0050, including `std/process`.

## Consequences

- **The master spec's Phase 5 list is not amended;** this ADR records the
  difference, as ADR 0038 did for Phase 4.
- **N needs no new runtime dependency:** `clang` is a CI tool, and
  `cargo test` never needs it.
- **Every golden program but the nine excluded runs through the LLVM
  backend in CI** from 5.0a on, so each program 5.0b and 5.0c add runs
  there too, unless the table excludes it with its reason. GC stress stays
  out of that walk: two programs time out under it on both backends.
- **The pin is a maintenance cost:** another build of `clang-18` fails
  the version step, and a retired image leaves the Linux leg with no
  runner, so it never starts; both are loud, and moving the pin takes an
  ADR.
- **The backlog grows** by item 2's list, which Phase 5's closing ADR
  keeps.

## References

- `nova-spec/00-MASTER-SPEC.md` §3, Phase 5; `nova-spec/14-CODEGEN.md`'s
  note of 2026-10-06.
- `docs/phase-5-plan.md` §1, and §3 decisions 1 and 2.
- `docs/superpowers/specs/2026-10-26-phase-5-0-the-language-and-std-a-compiler-needs-design.md`
  §3.1, §3.2 and §12.
- `.github/workflows/ci.yml`, the `test` job;
  `crates/nova-cli/tests/run_tests.rs`, `release_golden_suite`.
- ADR 0038, Phase 4's scope; ADR 0047, Phase 4's boundary.
