# ADR 0038 — Phase 4's scope, and what the phase leaves out

## Status

Accepted, 2026-10-14 (Phase 4.0, branch `claude/loving-archimedes-frtf5q`;
spec `docs/superpowers/specs/2026-10-14-phase-4-0-foundations-design.md`).
The decisions are the user's, made on 2026-10-14 (`docs/phase-4-plan.md`
§1, and §3 decision 13).

## Context

`nova-spec/00-MASTER-SPEC.md` §3 lists Phase 4, "Frontend / WASM", in nine
positions:

1. a WASM backend, on `wasm-encoder` and `walrus`;
2. a bundler, with tree shaking and code splitting;
3. `std/dom`, bindings generated from `web-sys` descriptors;
4. `std/ui`: signals, effects, memos and components;
5. `std/ui/html`, element builders;
6. `std/ui/router`;
7. `nova dev`, a dev server with HMR;
8. SSR, rendering natively and hydrating in WASM;
9. SSG.

Its gate: "`examples/06-counter-spa` and `examples/07-fullstack-blog` work
in browser, pass Lighthouse 95+."

`nova-spec/30-FRONTEND.md` describes more than the gate needs: examples 08
to 10 (§12), code splitting with `lazy()` (§7.3), JS interop through npm
(§9), source maps (§8), and `axum` for the dev server (§8).
`nova-spec/60-EXAMPLES.md` §7 asks the blog for SQLite, and §8 calls
examples 08 to 10 "post-Phase 6", contradicting `30-FRONTEND.md` §12.

## Decision

1. **Phase 4 builds what the gate's two examples need:** all nine
   positions, each as far as the gate needs it.
2. **It departs from the specs where the user decided so:**
   - **Component syntax is built as `30-FRONTEND.md` §3 writes it:**
     `component`, `view { … }`, named arguments and trailing blocks, with
     an RFC.
   - **New crates: `wasm-encoder` only,** with `wasmparser` as a
     dev-dependency that validates what tests build. There is no `walrus`.
     `nova dev` is written on `std::net`, without `axum` or `tokio`.
   - **`nova dev` reloads the page** through Server-Sent Events, with an
     overlay for compile errors. It is not HMR that keeps state.
   - **The WASM runtime is the backend's own (route (a))**, written in Nova
     on memory builtins, with the browser's work done in a JS shim. ADR
     0039 records it.
   - **The WASM collector is mark-sweep between event turns,** not the
     reference counting `13-RUNTIME.md` §8 names. 4.1's ADR records it.
   - **The blog stores its posts in a JSON file,** through `std/fs` and
     `std/json`, not SQLite.
   - **The gate is measured** by a CI job (Lighthouse in headless Chrome,
     the gzipped sizes, Playwright) and by the user, who opens both
     examples in their own browser at the phase's close.
3. **Phase 4 leaves out:**
   - examples 08 to 10;
   - code splitting with `lazy()`;
   - npm resolution for `@js_import`, and user-level `@js_import` and
     `@js_export`. `std/dom` reaches the shim through compiler-known
     functions instead. **Amended 2026-10-16 (branch
     `claude/loving-archimedes-frtf5q`):** through `extern "js"`
     declarations, which only std may write (ADR 0041);
   - source maps (`30-FRONTEND.md` §8; `14-CODEGEN.md` §9.2);
   - HMR that keeps state, `axum`, and `walrus`;
   - bindings generated from `web-sys` descriptors: `std/dom` is a written
     subset;
   - reference counting, and the WASM GC proposal;
   - `wasm-bindgen-test`, and visual regression snapshots
     (`30-FRONTEND.md` §13);
   - the LSP rows `40-TOOLING.md` §3.1 marks Phase 4 (inlay hints, code
     lens, call hierarchy);
   - SQLite.
4. **A sub-phase's spec may pull one of these back in,** with a ruling that
   names this ADR.

## Consequences

- **The master spec's Phase 4 list is not amended;** this ADR records the
  difference, as ADR 0026 did for Phase 3.
- **No new runtime dependency reaches an installed `nova` but
  `wasm-encoder`** and its one dependency, `leb128fmt`.
- **A program's JS dependencies wait** for the npm interop this ADR
  leaves out.
- **The examples beyond the gate,** and the LSP rows, go to the backlog the
  phase's closing ADR keeps.

## References

- `nova-spec/00-MASTER-SPEC.md` §3, Phase 4.
- `nova-spec/30-FRONTEND.md`, `nova-spec/60-EXAMPLES.md` §6 to §8,
  `nova-spec/13-RUNTIME.md` §8, `nova-spec/40-TOOLING.md` §3.1.
- `docs/phase-4-plan.md` §1, and §3 decisions 1, 3, 9, 11, 12 and 13.
- ADR 0039, the WASM backend and its runtime.
