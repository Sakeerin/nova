# ADR 0039 — The WASM backend and its runtime

## Status

Accepted, 2026-10-14 (Phase 4.0, branch `claude/loving-archimedes-frtf5q`;
spec `docs/superpowers/specs/2026-10-14-phase-4-0-foundations-design.md`).
The route and the runtime's language are the user's decisions of
2026-10-14 (`docs/phase-4-plan.md` §3, decision 1).

## Context

`nova-spec/14-CODEGEN.md` §8 sketches a WASM backend on `wasm-encoder`,
with a bundler step that writes a JS shim. `nova-spec/13-RUNTIME.md` §8
says a WASM build uses the browser's event loop "via wasm-bindgen", starts
with reference counting, and reaches the DOM through bindings generated
from `web-sys`.

`nova-runtime` cannot run in a browser:
- its collector finds roots by scanning the native stack;
- its executor and IO wait by blocking;
- its net, fs, signal and process modules call the operating system.

MIR suits WASM: every heap value is a `Ptr`, objects are 8-byte slots,
generics are monomorphized, and async is already stackless.

A spike in scratch space compared two routes for the runtime, on one
program:
- **(a)** the backend writes the runtime into every module, and a JS shim
  does what the browser does better. It came to 1,847 bytes gzipped.
- **(b)** `nova-runtime`'s OS-free parts, compiled for
  `wasm32-unknown-unknown` ahead of time and committed. It came to 9,931
  bytes gzipped, and such a module ships every runtime function in every
  app: `str::parse::<f64>` alone added 14.6 KB. Its bytes depended on the
  toolchain: Rust 1.78 built 24,917 bytes where 1.97 built 18,707.

## Decision

1. **Route (a).** `nova-codegen-wasm` lowers MIR to one module with
   `wasm-encoder` 0.243.0, the last release that declares a minimum Rust
   under the workspace's 1.78. `wasmparser` 0.243.0 validates, in tests
   only. An installed `nova` needs no other toolchain to build for WASM.
2. **The runtime is written in Nova:** `std/rt`, compiled only for the
   WASM target.
   - It is written on nine memory builtins (`mem_load8`, `mem_store8`,
     `mem_load64`, `mem_store64`, `mem_copy`, `mem_fill`, `mem_pages`,
     `mem_grow`, `heap_base`). They exist only in a WASM build's std, and
     lower to instructions in line.
   - Each of its functions is the port of a `nova-runtime` function, with
     the same results, on `Int` addresses.
   - MIR calls `std/rt`'s function where a native build calls
     `nova-runtime`, so mono adds only the runtime functions a program
     reaches. The backend converts between a `Ptr` and an `Int` address at
     those calls.
   - In 4.0 its allocator only grows. 4.1 adds the collector.
3. **The shim does what JS does best:** printing, the panic, `Float` to
   text and back, case mapping, and the hash seed.
   - Each is held to `nova-runtime`'s result. ECMAScript breaks an exact
     tie between two shortest texts of a `Float` toward the even digit, and
     Rust upward; `toFixed` rounds ties up, and Rust to even. The shim
     decides both exactly, with `BigInt` on the double's bits.
   - Offline, against Rust: 2,005,144 doubles' text, 565,134 fixed-precision
     cases and 399,831 parses, with no difference. Differential programs in
     `tests/wasm/` keep it so. `str_to_float` is std's alone, so no program
     can hand it any text: a unit test runs the shim's parser under Node
     against Rust's `str::parse`, on every case of its grammar.
   - Case mapping rests on the browser's Unicode tables, which can lag
     Rust's.
4. **Values and memory.**
   - `I64` → `i64`, `F64` → `f64`, `I8` → `i32`, `Ptr` → `i32`.
   - One 32-bit memory, 16 MB at first and at most 1 GB.
   - Address 0 is never an object; `8..64` holds `std/rt`'s state words; the
     data segment starts at 64.
   - Slots stay 8 bytes, a `Ptr` zero-extended in its slot.
   - A string literal is a static header in the data segment, not an
     allocation per evaluation.
5. **Control flow** is structured by Ramsey's algorithm ("Beyond
   Relooper", 2022), for the reducible graphs MIR builds. An irreducible
   graph falls back to a dispatch loop. A `Switch` is a chain of
   comparisons.
6. **Function values** are indices into one `funcref` table, called with
   `call_indirect`.
7. **`nova build --target wasm`,** and its alias `nova bundle`, write
   `dist/`: `index.html`, `app.js` and `app.wasm`.
8. **Under Node,** a panic exits with 134 and a trap with 132, the native
   abort's and trap's statuses. Tests run WASM builds under Node, which CI
   installs on all three systems; with `NOVA_REQUIRE_NODE=1` a test fails
   without it.

## Consequences

- **`nova-codegen-wasm` is published** from the next release: 20 crates.
- **Two copies of each runtime function exist,** Rust's and Nova's. The
  differential programs, and the golden suite run both ways, are what keep
  them equal.
- **A WASM build's recursion is bounded by V8's stack,** about 1 MB under
  Node's default. A program that recurses that deep stops with `nova:
  stack overflow`.
- **The DOM, the event loop and the collector** follow in 4.1 and 4.2, on
  this runtime.

## References

- `docs/phase-4-plan.md` §3, decisions 1 to 5.
- The spec named above, §2, §4 to §6 and §13.
- `nova-spec/14-CODEGEN.md` §2.3 and §8; `nova-spec/13-RUNTIME.md` §8;
  `nova-spec/30-FRONTEND.md` §7 and §10.
- ADR 0038, Phase 4's scope.
