# ADR 0041 — `std/dom` and the boundary with JS

## Status

Accepted, 2026-10-16 (Phase 4.2, branch `claude/loving-archimedes-frtf5q`;
spec `docs/superpowers/specs/2026-10-16-phase-4-2-std-dom-and-the-shim-design.md`).
`extern "js"`, the handles' release by the collector, `fetch` as an
`async fn`, and `import dom` are the user's decisions of 2026-10-16.

## Context

After 4.1 a WASM build ran the whole language, but nothing in it could
touch a page, and JS could not call it.
- ADR 0038 said `std/dom` would reach the shim through compiler-known
  functions. Each such function so far had meant a `Builtin`, an
  `RtFunc`, a signature in the checker, and a shim entry.
- `nova-spec/30-FRONTEND.md` §9.3 passes JS objects by handle and
  functions by a "trampoline via stable function pointer", but says
  nothing of when a handle is released or what keeps a function JS holds
  alive.
- ADR 0040's collector runs only at the end of a turn, and its roots were
  the executor's tables and tasks. A closure only JS holds was in none of
  them.
- Every std module was in every program's prelude.

## Decision

1. **std declares JS functions as `extern "js"`.**
   - Only a std module of a WASM build may: a user's `extern` block is
     still E0090 on WASM, and `"js"` is still E0900 natively.
   - Parameters and results are `Int`, `Float`, `Bool` and `String`.
   - The backend imports each called one from `env` under its name; the
     shim holds each one's JS and adds only those the module imports.
   - A JS exception in one is a Nova panic that names the function.
   - User code still has no JS interop: `@js_import` and `@js_export`
     stay in ADR 0038's backlog.
2. **JS objects are handles, released by the collector.**
   - The shim keeps the objects in a table; Nova holds a slot's index in
     a 16-byte box, whose address a record keeps in an `Int` field. The
     collector is conservative, so that field keeps the box.
   - A weak registry of boxes, in `std/rt`, is walked after each sweep, as
     the task index is pruned: a freed box's slot is released.
   - A slot that holds nothing is a panic. A box is freed only once no
     value holds it, so no value names a released slot; a value built by
     hand reads whatever word its address holds, and reaches at most an
     object of the program's own table.
3. **Closures JS may call are a root, and each outermost call from JS is a
   turn.**
   - `std/rt` keeps a table of `fn(Int)` closures, whose entries the
     collector marks. `std/dom` wraps each handler in one.
   - `nova_call` calls an entry, then runs a turn: it wakes what is due,
     runs the queue, and collects if a collection is due. A call made
     while Nova code is on the stack (a handler that clicks another
     element) runs its handler only: the shim counts the calls in
     progress, and only the outermost runs the turn, so the collector
     never runs with a Nova frame on the stack.
   - A module that registers such a closure exports `nova_call`, and the
     turn and the switches with it, so a program with a synchronous `main`
     collects at the end of each call.
   - A listener's closure stays a root until the listener is removed.
   - A task that calls `block_on` panics as re-entrant wherever it runs,
     in a call's turn as in an `async fn main`'s.
4. **A task can wait for JS.** It parks on a token, a handle the shim
   settles: `fetch` settles one with its reply, then calls `nova_wake`,
   which queues the tasks parked on it and runs a turn. A turn that ends
   with no parked task's deadline, and a task waiting on a token, returns
   -2, and JS sets no timer. Inside `block_on`, which holds the thread,
   such a wait is a panic.
5. **`std/dom` is std's first imported module.** It is in WASM's std but in
   no prelude: a program writes `import dom`. Natively that import is
   E0092. The names `dom` and `ui` are std's: a project's module under
   either, which an `import` reaches, is E0093, and `nova-pm` refuses
   either as a package name.
   `std/rt`'s public names are seen by std's modules only.
6. **Tests use a fake DOM; CI uses Chromium.** `tests/dom/fake_dom.mjs` is
   plain JS that holds what a browser holds and no more, so a missing root
   shows under `NOVA_GC_STRESS`. A CI job, `browser`, clicks the gate's
   counter in Chromium through Playwright, which only that job installs.

## Consequences

- **Adding a DOM function** is an `extern "js"` declaration and a wrapper
  in `std/dom`, and an arm in `shim.rs`'s `js_function` (and, for one that
  calls Nova back or settles a token, its name in `shim()`'s list for
  `shim/dom_call.js` or `shim/dom_fetch.js`): no new `Builtin`, `RtFunc` or
  signature in the checker.
- **Handles and closures are the collector's,** so 4.3's reactive graph,
  whose effects JS calls back, needs no new mechanism.
- **Every shim carries the scheduler,** about 0.5 KB gzipped: hello world's
  `app.js` is 2.8 KB where it was 2.3 KB.
- **An `async fn main` imports `host_release`,** because the collector may
  release handles, so its shim carries the handle table too: about 0.7 KB
  more.
- **The fake DOM can differ from a browser.** The browser job is the
  check, and later sub-phases add their own browser tests.

## References

- `docs/phase-4-plan.md` §3, decisions 5, 6, 12 and 13, and §4's 4.2 entry.
- The spec named above.
- ADR 0038, the recorded scope; ADR 0039, the backend; ADR 0040, the
  collector and the executor.
- `nova-spec/30-FRONTEND.md` §5 and §9; `nova-spec/13-RUNTIME.md` §8;
  `nova-spec/14-CODEGEN.md` §8.
