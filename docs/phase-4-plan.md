# Phase 4 Plan — Frontend / WASM

> Status: **approved** by the user (2026-10-14). Derived from
> `nova-spec/00-MASTER-SPEC.md` §3 (Phase 4), `nova-spec/30-FRONTEND.md`,
> `nova-spec/14-CODEGEN.md` §2.3 and §8, `nova-spec/13-RUNTIME.md` §8, and
> `nova-spec/60-EXAMPLES.md` §6 and §7. The user's decisions of 2026-10-14 are in §1. Each sub-phase
> below gets its own spec, plan and review before it is built.

## 1. Goal (from the spec)

Phase 4 = "Frontend / WASM". The master spec's goal is "Build SPA + SSR app
end-to-end", and its gate reads: "`examples/06-counter-spa` and
`examples/07-fullstack-blog` work in browser, pass Lighthouse 95+."

The master spec lists nine items for Phase 4:

1. `crates/nova-codegen-wasm/`, a WASM backend (`wasm-encoder` and
   `walrus`);
2. `crates/nova-bundler/`: bundle, tree-shake, code-split;
3. `std/dom/`, low-level DOM bindings ("auto-bindgen from web-sys
   descriptors");
4. `std/ui/`: signals, effects, memos, components;
5. `std/ui/html/`, element builders;
6. `std/ui/router/`, a client router;
7. `nova dev`, a dev server with HMR;
8. SSR: render to string in the native runtime, hydrate in WASM;
9. SSG: compile-time route enumeration.

`60-EXAMPLES.md` adds to the gate:
- **06:** "`nova dev` opens browser, counter responds to clicks. Bundle
  size < 50 KB gzipped."
- **07:** "All pages work, Lighthouse Performance > 95, full CRUD via admin
  UI, no XSS via untrusted input". It asks for SSR, hydration, a login,
  cookie sessions, CSRF protection and SQLite.

The master spec's §7 closes the phase with the tag `v0.4.0`.

**The user's decisions, 2026-10-14:**
- **Scope: the gate.** Phase 4 builds what `06-counter-spa` and
  `07-fullstack-blog` need, which is all nine items, each as far as the
  gate needs it. ADR 0038 moves the rest to a recorded backlog (§3,
  decision 13):
  - examples 08 to 10;
  - code splitting with `lazy()`;
  - JS interop through npm;
  - source maps;
  - the LSP rows `40-TOOLING.md` §3.1 marks Phase 4.
- **Component syntax: as `30-FRONTEND.md` §3 writes it.** That means
  `component`, `view { … }`, named arguments and trailing blocks, with an
  RFC and an ADR (decision 7).
- **New crates: `wasm-encoder` only**, with `wasmparser` as a
  dev-dependency that validates the modules tests build.
  - No `walrus`.
  - `nova dev` is written on `std::net`, without `axum` or `tokio`.
  - Each crate's effect on `Cargo.lock` is shown in its sub-phase's spec
    before it is added.
- **`nova dev`: live reload, not HMR.**
  - A change rebuilds the app and reloads the page through Server-Sent
    Events.
  - A compile error shows as an overlay in the browser.
  - The page's state starts again on each reload.
- **The WASM runtime: a spike first.** It tries two routes, and the user
  picks one from what it measures (decision 1).
- **GC on WASM: mark-sweep between event turns,** not the reference
  counting `13-RUNTIME.md` §8 names (decision 3).
- **`07-fullstack-blog`'s store: a JSON file,** written and read through
  `std/fs` and `std/json`, in place of SQLite.
- **The gate is measured in CI, and by the user.**
  - A CI job builds both examples and runs Lighthouse in headless Chrome.
    It checks the gzipped sizes and drives both examples with Playwright.
  - At the phase's close, the user opens both examples in their own
    browser.

## 2. Reality check — what Phase 4 needs that the repo lacks

1. **There is no WASM backend.**
   - `crates/nova-codegen-wasm` and `crates/nova-bundler` exist, each with
     a one-line `src/lib.rs` and `publish = false`. Neither `nova-driver`
     nor `nova-cli` depends on them.
   - `nova build` takes `-o` and `--release` only
     (`crates/nova-cli/src/cmd/run.rs`). There is no `--target`.
   - `run` uses Cranelift's JIT, `build` a Cranelift object, and
     `build --release` the LLVM backend's textual IR through `clang` or
     `llc` (`crates/nova-driver/src/lib.rs`).
2. **MIR suits a WASM backend.**
   - `MirTy` is `I64`, `F64`, `I8`, `Ptr` or `Unit`
     (`crates/nova-mir/src/lib.rs`). `Ptr` covers every heap value:
     strings, bytes, sums, records, arrays, closures and futures.
   - Heap objects are 8-byte slots at fixed offsets: a sum is
     `{tag, fields…}`, an array `{len, elems…}`, a closure `{code, env}`.
   - Generics are monomorphized from `main` (`mono.rs`), and trait calls
     are resolved statically; there are no trait objects. So the functions
     MIR emits are already only the reachable ones, which is the tree
     shaking `30-FRONTEND.md` §7.2 asks for.
   - On `wasm32` a pointer is 32 bits while a slot stays 8 bytes; 4.0's
     spec decides how a pointer sits in a slot.
3. **Async is already stackless.**
   - `async_lower.rs` turns each `async fn` into a `$poll` function over a
     heap state object, so no stack switching is needed, which WASM could
     not do.
   - The executor is single-threaded (`crates/nova-runtime/src/task.rs`).
     But `block_on` and the IO poller wait by blocking, in `poll::wait`
     and `thread::sleep`, which a browser's event loop forbids.
4. **The runtime cannot run in a browser.**
   - `nova-runtime` is Rust, linked as a static library.
   - Its collector is conservative mark-sweep (`gc.rs`). It finds roots by
     spilling registers with `setjmp` (`gc_stack.c`) and scanning the
     native stack up to its base, which it finds through
     `pthread_getattr_np` or the Windows and macOS equivalents. A WASM
     program cannot read its own value stack.
   - Its net, fs, signal, process and poll modules call the OS.
5. **The language lacks `30-FRONTEND.md` §3's syntax.**
   - There are no named arguments: call arguments are a list of
     expressions (`crates/nova-ast`).
   - There are no trailing block arguments, and no `component` or `view`
     keyword among the lexer's 32.
   - `||` lexes as one token, so `let f = || 1` fails with P0001,
     "expected expression"; checked 2026-10-14 with `nova` 0.3.2.
   - Attributes take identifiers only, and only `@test(should_panic)` is
     accepted (`KNOWN_ATTRIBUTES` in `crates/nova-resolver`). So §5's
     `@js_import("__nova_dom__")` does not parse.
   - `extern "C"` accepts only `Int`, `Float` and `Bool`.
6. **std modules cannot be imported by name.**
   - The 16 non-test std modules are glob-imported into every module.
   - An import path uses `::` (`import geometry::{Point, origin}`), so
     §3's `import nova/ui` does not parse.
   - A prelude `std/ui/html` would put `div`, `text` and `input` into
     every program's scope.
7. **There are no mutable globals,** only `const`. Signals need a "current
   effect" that `value()` reads (§2), so it must live in the runtime, as
   task payload slots did in Phase 2.
8. **Closures capture by value.** Assigning to a captured variable inside a
   closure leaves the outer one unchanged (`tests/runtime/closures.nova`).
   A signal is a heap cell, so copies of it share one value, as §2 needs.
9. **The same component code must run on two targets for SSR (item 8):**
   - natively, rendering to a string;
   - in WASM, building and then hydrating the DOM.

   std has no per-target modules today.
10. **`07`'s server has most of what it needs.**
    - `std/http` serves requests (`Server.listen`), and `std/crypto` has
      `sha256`, `hmac_sha256`, `hmac_sha256_verify` and `random_bytes`.
    - `std/http` has no cookie helpers.
    - There is no database driver, and `extern "C"` could not carry
      SQLite's strings and pointers.
11. **The tools on this machine:**
    - Node 22 and npm 10.
    - Chromium at `/opt/pw-browsers`.
    - `clang` 18 with the `wasm32` target, and `wasm-ld`.
    - No `wasm-opt` or `wasmtime`, and no Rust `wasm32-unknown-unknown`
      target.

    GitHub's hosted runners have Node on all three operating systems.
12. **The crates (crates.io's index, 2026-10-14):**
    - **`wasm-encoder`:**
      - 0.243.0 is the last version that declares a minimum Rust under
        1.78: 1.76. 0.244.0 declares 1.81, and the latest, 0.262.0,
        declares 1.88.
      - Its normal dependency is `leb128fmt`, plus an optional
        `wasmparser`.
    - **`wasmparser`** declares the same minimums at the same versions.
    - **`walrus`** declares no minimum. It depends on `rayon`, `gimli`,
      `id-arena`, `wasmparser` and `wasm-encoder`.
    - **The master spec's pair doesn't fit:** `walrus` 0.22 requires
      `wasm-encoder` `^0.212`, while the master spec names 0.215.
    - None of these is in `Cargo.lock` today. `wasm-bindgen`, `js-sys`
      and `web-sys` are in it only through `criterion`'s `plotters`, a
      dev-dependency.
13. **The spec files disagree on examples 08 to 10.** `30-FRONTEND.md` §12
    lists them for Phase 4; `60-EXAMPLES.md` §8 calls them "post-Phase 6".

## 3. Key decisions (recommended now, confirmed before building)

Each is confirmed in its sub-phase's spec, and recorded in an ADR where it
departs from the master spec or from `30-FRONTEND.md`.

1. **The WASM runtime's route (spike, then the user; before 4.0's spec).**
   A WASM program needs an allocator, the collector, string operations,
   and number-to-text conversions. The spike tries two routes:
   - **(a) the backend emits the runtime.**
     - `nova-codegen-wasm` writes the allocator, collector and string
       primitives into every module.
     - The JS shim does what the browser does better: printing, the time,
       and possibly number formatting.
     - An installed `nova` builds WASM with no other toolchain. But all of
       it is new code, kept equal to the native runtime's behaviour by
       tests.
   - **(b) the Rust runtime, compiled to WASM ahead of time.**
     - A WASM build of the OS-free parts of `nova-runtime` (strings,
       number formatting, hashing, the page heap) is built with the
       `wasm32-unknown-unknown` target.
     - It is committed to the repository, and CI checks that a rebuild
       reproduces it byte for byte.
     - The backend's module and the runtime's share one memory: either
       the shim instantiates them side by side, or the backend merges
       them.
     - Its behaviour equals the native runtime's, which suits SSR and
       hydration. But the repository carries a binary, and CI needs the
       target.

   A third route was considered and not taken: the LLVM backend's IR,
   compiled for `wasm32` by `clang` and linked by `wasm-ld`. Every user
   would then need both tools, and Apple's `clang` ships without
   `wasm-ld`.

   The spike builds, under Node, a program that prints a string literal,
   an interpolated `Int`, and a `Float`. For each route it measures:
   - the gzipped size, against 30 KB (`30-FRONTEND.md` §11);
   - the amount of code;
   - what a user's machine and CI need.

   The user picks the route from its results, and 4.0's spec builds it.

   **Spike result, 2026-10-14: route (a), by the user.** The spike ran in
   scratch space, and nothing from it is in the repository.
   - **The program:** a string literal, an `Int` interpolated through
     `fib(20)`, and `Float` arithmetic, including `1.0 / 3.0`, `1.0e21`
     and `1.0e-7`.
   - **The pipeline:** `nova` 0.3.2's front end made the MIR. A small
     backend lowered it with `wasm-encoder` 0.243.0, and `wasmparser`
     0.243.0 validated the module. Both routes ran under Node 22 and
     printed what `nova run` prints.
   - **The sizes, gzipped:** (a) 1,847 bytes for `app.wasm` and `app.js`;
     (b) 9,931 bytes, of which the runtime's module was 8,782.
   - **(b)'s runtime ships every function, in every app.**
     - Without `Float` formatting it was 749 bytes; `core::fmt`'s
       formatting is about 8 KB.
     - `str::parse::<f64>` adds 14.6 KB, and `char::to_uppercase` 1.5 KB.
     - A whole runtime would leave little of hello world's 30 KB.
   - **(b)'s module was byte-identical** when built in two directories by
     one toolchain. Rust 1.78 built a different one, 24,917 bytes against
     18,707, so a committed binary would need a pinned toolchain.
   - **JS's formatting of a `Float` differs from Rust's in exact ties.**
     261 of 2,005,144 doubles differed. For example, Rust prints
     1658206780088562.25 ending in `.3`, and JS in `.2`, because
     ECMAScript picks the even digit.
     - Route (a) must detect ties.
     - Its runtime functions are held to the native runtime's by
       differential tests like this one.

   So 4.0 builds route (a). Its spec decides:
   - whether the runtime is written as WASM instructions in the backend,
     or in Nova with memory builtins;
   - which functions JS provides.
2. **The backend.**
   - MIR is lowered straight to WASM with `wasm-encoder`, pinned to the
     last version that builds on Rust 1.78. `wasmparser` of the same
     version validates every module the tests build.
   - Values map as:
     - `I64` to `i64`;
     - `F64` to `f64`;
     - `I8` to `i32`;
     - `Ptr` to `i32`.
   - Heap objects keep their 8-byte slots, so MIR's offsets do not change
     (§2 item 2).
   - Memory is 16 MB at first and at most 1 GB (`30-FRONTEND.md` §10).
   - There is no `walrus`: MIR's output is already only what is reachable.
   - `wasm-opt` runs when it is on `PATH` (§7.2 step 4) and is never
     required.
3. **The collector on WASM: mark-sweep between turns.**
   - Nova code runs only when JS calls it: `main`, an event handler, a
     timer, or a resolved promise. When that call returns, the WASM stack
     holds nothing, so the collector runs then, with no stack to scan.
   - Its roots are:
     - the table of Nova values that JS holds (event handlers, for
       example);
     - the executor's tasks;
     - the reactive graph's roots.
   - Like the native collector, it does not move objects, and it scans
     their slots conservatively.
   - It collects cycles. The graph between signals and effects is full of
     them, and reference counting would leak them unless weak references
     were added.
   - A long computation that does not return keeps allocating until it
     does. 4.1's spec sets the threshold, and a stress mode that collects
     after every turn in the tests.
   - It departs from `13-RUNTIME.md` §8, so it gets an ADR.
4. **Async on the browser's event loop.**
   - On WASM, `main` returns once the app is mounted.
   - The executor is polled from JS: a timer's callback, or a promise's
     resolution, polls the task that waits on it.
   - Nothing blocks, so there is no `block_on` on WASM.
   - `sleep` and `timeout` use `setTimeout`.
5. **std per target.**
   - The driver compiles a set of std modules chosen by the target:
     - **native:** today's set;
     - **WASM:** the modules that need no OS (core, strings, fmt,
       collections, json, bytes, log and time), plus task, on the event
       loop, and `std/dom`.
   - Using a native-only module from a WASM build is a compile error that
     names the target.
   - `std/ui` is one source on both targets. Underneath it, one backend
     calls `std/dom` (WASM), and the other builds an HTML string (native,
     for SSR). 4.1's spec decides how a target selects a module, and
     4.3's how `std/ui` selects its backend.
6. **Importable std modules.**
   - `std/dom`, `std/ui`, `std/ui/html` and `std/ui/router` are not in
     the prelude. A program imports them by name, with today's `::`
     paths, which differs from `30-FRONTEND.md`'s `import nova/ui` and
     gets an ADR.
   - 4.3's spec names the root (for example `import ui::html::{div,
     button}`) and checks the name cannot clash with a package's.
7. **Component syntax (4.4), by the user's decision.**
   - An RFC, `docs/rfcs/0001-…`, specifies:
     - `component Name(params) { … }`, sugar for a function that returns
       a `View`;
     - `view { … }`, sugar for builder calls (`30-FRONTEND.md` §3.1);
     - named arguments;
     - trailing blocks;
     - `||`, a closure with no parameters, everywhere.
   - Recommendation: named arguments and trailing blocks only inside
     `view { … }`, where they map to attributes and children (§4). Outside
     it, calls stay as they are, and the type checker gains no general
     named-argument rules.
   - Recommendation: `component` and `view` are contextual keywords, so
     no existing identifier breaks. On 2026-10-14 no `.nova` file in the
     repository used either as one; the published packages are checked
     too.
   - Text that reads a signal becomes a closure, so it updates when the
     signal changes (§3.1).
   - §2 writes `count.value()` and §3 writes `count.value`; the RFC picks
     one.
   - `nova fmt` formats the new syntax, under its three checks (ADR 0028).
     The extension's grammar and the server's semantic tokens colour it.
8. **The bundler's output** follows `30-FRONTEND.md` §7.1, without
   `chunks/`, because code splitting is in the backlog:
   - `index.html`, the user's or a generated one;
   - `app.wasm`;
   - `app.js`, an ES module with `init()` (§9.2);
   - `assets/`, copied from `public/`;
   - `manifest.json`.

   `nova build --target wasm` writes it to `dist/`, and `nova bundle` is
   its alias.
9. **`nova dev`, by the user's decision.**
   - An HTTP/1.1 server written on `std::net`. It binds `127.0.0.1` and
     serves `dist/` only, refusing any path that leaves it.
   - A watcher that polls the sources' modification times.
   - A rebuild on change, then a reload through Server-Sent Events.
   - A compile error shown as an overlay.
   - It departs from §8's HMR and `axum`, so it gets an ADR.
10. **SSR, hydration and SSG (4.6).**
    - `nova build --ssr` builds the server natively, so it renders with
      `std/ui`'s string backend.
    - The client hydrates: it attaches to the nodes the server sent, in
      order, and replays the effects (§7.4).
    - `nova build --ssg` renders the router's static routes into files
      under `dist/` at build time.
    - 4.6's spec decides:
      - how hydration finds its nodes;
      - what happens on a mismatch;
      - whether `--ssr` is a flag of `build`, or simply a native build of
        a server that uses `std/ui`.
11. **`07-fullstack-blog` (4.7)** follows `60-EXAMPLES.md` §7 with a JSON
    file in place of SQLite, by the user's decision, recorded in ADR 0038.
    - Its server is a Nova program on `std/http`.
    - **Sessions** are cookies signed with `hmac_sha256`, `HttpOnly` and
      `SameSite`.
    - **Forms** carry a CSRF token.
    - **Text** from users is escaped by both renderers by construction.
      `std/ui` offers no raw-HTML insertion.
    - 4.7's spec decides how passwords are hashed, for example PBKDF2
      over `hmac_sha256`, written in Nova.
12. **The gate's measurement, by the user's decision.**
    - **A CI job, on Ubuntu:**
      - builds both examples and serves them locally;
      - runs Lighthouse, installed by the job, in headless Chrome, and
        fails below 95 for Performance;
      - fails when hello world is 30 KB gzipped or more, or 06 is 50 KB
        or more;
      - drives 06 and 07 with Playwright, as 4.5 and 4.7 specify.
    - **`cargo test` never touches the internet.**
      - It runs WASM under Node.
      - It runs DOM programs against a small fake DOM written in JS and
        kept in the repository.
      - In CI, a test that needs Node fails when Node is missing; it does
        not pass without running.
    - **The user** opens both examples in their own browser at the close.
13. **The recorded scope (4.0).** ADR 0038 records the decisions above
    that depart from the specs, and what Phase 4 leaves out:
    - examples 08 to 10 (§2 item 13);
    - code splitting with `lazy()` (§7.3);
    - npm resolution for `@js_import` (§9.1), and user-level
      `@js_import` and `@js_export` (§9.2). `std/dom` reaches the shim
      through compiler-known functions instead;
    - source maps (§8; `14-CODEGEN.md` §9.2);
    - HMR that keeps state, `axum`, and `walrus`;
    - bindings generated from `web-sys` descriptors (master spec item 3).
      `std/dom` is a written subset;
    - reference counting, and the WASM GC proposal (`13-RUNTIME.md` §8);
    - `wasm-bindgen-test` and visual regression snapshots
      (`30-FRONTEND.md` §13);
    - the LSP rows `40-TOOLING.md` §3.1 marks Phase 4 (inlay hints, code
      lens, call hierarchy);
    - SQLite, replaced by a JSON file.

    A sub-phase's spec may pull one of these back in, with a ruling that
    names ADR 0038.

## 4. Sub-phases (each gated, reviewed, and merged on the user's word)

Each sub-phase builds on the one before it. Each runs the established
loop: a spec, a plan, the implementation, one fresh review and one fix
pass, a PR, and a merge on the user's word.

### 4.0 — Foundations and the first WASM program
- **ADR 0038**, the recorded scope (decision 13), and the spike's chosen
  route (decision 1).
- **`wasm-encoder`** in the workspace, pinned, and **`wasmparser`** as a
  dev-dependency (decision 2). Their effect on `Cargo.lock` is in the
  spec.
- **`nova build --target wasm`**, with `nova bundle` as its alias, writes
  `dist/`'s `index.html`, `app.wasm` and `app.js` (decision 8).
  - It covers `Int`, `Float` and `Bool`, control flow, calls, strings and
    `print`, which writes to the console.
- **A test harness** runs `dist/app.js` under Node and compares its
  output with the native build's golden files.
- **Gate:**
  - `examples/01-hello-world` and `02-fibonacci`, built with
    `--target wasm`, print under Node what their native builds print, on
    all three operating systems.
  - Hello world is under 30 KB gzipped, checked by a test.

### 4.1 — The language on WASM
- **The rest of MIR:** records, sums, arrays, closures, generics, and
  async.
- **The collector between turns,** with its stress mode (decision 3).
- **std per target** (decision 5), and the executor on the event loop
  (decision 4).
- **Gate:**
  - Every program in `tests/runtime/` that uses only WASM's std prints,
    built for WASM and run under Node, what its native build prints.
  - The programs left out are listed, each with its reason.

### 4.2 — `std/dom` and the shim
- **A table of handles** for JS objects, and a table of Nova closures that
  JS can call. The second is one of the collector's roots.
- **A written subset of the DOM:**
  - create elements and text;
  - set attributes and properties;
  - insert and remove nodes;
  - add and remove event listeners;
  - query selectors;
  - `location` and `history`;
  - timers;
  - `fetch`.
- **The fake DOM** for `cargo test`, and the CI job's first Playwright
  test in Chromium.
- **Gate:** a WASM program that builds elements and handles clicks passes
  under the fake DOM, and in Chromium in the CI job.

### 4.3 — `std/ui`: reactivity and element builders
- **Reactivity:** `signal`, `effect`, `memo`, `on_mount` and `on_cleanup`
  (`30-FRONTEND.md` §2).
  - An owner tree disposes of effects.
  - The runtime holds the current effect (§2 item 7).
- **Views:** the `View` type, `std/ui/html`'s builders and attribute
  helpers, `show`, `for_each`, and `mount` (§4).
- **Importable std modules** (decision 6).
- **The string backend** for native builds, enough for unit tests; SSR's
  use of it is 4.6's.
- **Gate:**
  - A counter written with builder calls works under the fake DOM and in
    Chromium.
  - §13's unit tests pass on both targets: signal reactivity, effect
    cleanup, and memo memoization.

### 4.4 — Component syntax
- **RFC 0001 and its ADR** (decision 7).
- **The language:** the lexer, parser, desugaring, type checker, formatter
  and the server's semantic tokens, and the extension's grammar.
- **`examples/06-counter-spa`** as `60-EXAMPLES.md` §6 writes it, with
  decision 6's import line.
- **Gate:**
  - `06-counter-spa` builds, and its counter responds to clicks in
    Chromium.
  - It is under 50 KB gzipped.
  - `nova fmt`'s three checks pass on every `.nova` file.

### 4.5 — The router and `nova dev`
- **`std/ui/router`:**
  - `Router` and `Route`;
  - `navigate`, `use_params` and `use_query`;
  - hash routing by default, and the history API by a flag
    (`30-FRONTEND.md` §6).
- **`nova dev`** (decision 9).
- **Gate:** in the CI job, through Playwright:
  - `06-counter-spa` under `nova dev` responds to clicks;
  - editing its source reloads the page with the change;
  - a type error shows the overlay.

### 4.6 — SSR, hydration and SSG
- **SSR:** the string backend renders a whole app, escaping all text.
- **Hydration**, and `nova build --ssr` and `--ssg` (decision 10).
- **Gate:**
  - A server-rendered page hydrates in Chromium and responds to events
    without re-creating its nodes.
  - `--ssg` writes one file per static route.

### 4.7 — `examples/07-fullstack-blog`
- **The blog as `60-EXAMPLES.md` §7 describes it,** with a JSON file
  (decision 11):
  - the home page's list (SSR);
  - a post's page (SSR, with its comments hydrated);
  - the login, validated in the client and authenticated on the server;
  - the admin's create, edit and delete, behind a session.
- **Gate:** in the CI job, through Playwright:
  - every page works;
  - the admin's full CRUD works;
  - a script put into a title or a comment stays inert text;
  - a form without its CSRF token is refused.

### 4.8 — The gate and `v0.4.0`
- **The CI job's Lighthouse run** on both examples, and the size checks
  (decision 12).
- **The user opens both examples** in their own browser.
- **The records:**
  - documentation for the new commands and std modules;
  - `CHANGELOG.md`;
  - dated notes in the specs;
  - an ADR that closes the phase, as ADR 0037 closed Phase 3.
- **The release `v0.4.0`,** by the runbook (`docs/releasing.md`).
  - If `nova-cli` depends on `nova-codegen-wasm` and `nova-bundler`, they
    join the published crates, with their names checked on crates.io
    first.
  - Every publish waits for the user's word.

## 5. Cross-cutting

- **Testing.**
  - Every feature is tested end to end through the `nova` binary, as
    `crates/nova-cli/tests/run_tests.rs` does today.
  - Every module a test builds is validated by `wasmparser`.
  - One set of golden files serves both targets.
  - Browser tests and Lighthouse run in the CI job, never in `cargo test`
    (decision 12).
- **Dependencies.**
  - Each new crate builds on Rust 1.78, the workspace's minimum, which
    CI's MSRV job checks.
  - Each is added only with the user's word, after its spec shows what it
    adds to `Cargo.lock`.
  - npm packages (Lighthouse, Playwright) are installed by the CI job
    only, and never by a test or by `nova`.
- **Records.**
  - `30-FRONTEND.md`, `14-CODEGEN.md` and `13-RUNTIME.md` get dated notes
    as each sub-phase lands.
  - Each departure from them or from the master spec gets an ADR.
- **Security.**
  - Both renderers escape text and attribute values by construction.
  - `nova dev` binds `127.0.0.1` and serves nothing outside `dist/`.
  - The blog signs its cookies, checks CSRF tokens, and never stores a
    password in clear.
- **Publishing.** Every publish to crates.io or an extension store, and
  every tag, waits for the user's word.

## 6. Top risks

1. **The WASM runtime's route** decides much of 4.0 and 4.1, and of the
   bundle's size. The spike comes first (decision 1).
2. **A missed root frees a live object.** On WASM, a Nova value that only
   JS holds, or that waits across an `await`, must be in a root table.
   - The stress mode collects after every turn in the tests.
   - The fake DOM drops its references the way a browser would.
3. **The syntax change reaches every part of the front end, and the
   tools:**
   - the lexer, parser, type checker and formatter;
   - the language server and the extension.

   New keywords could break identifiers in published packages. The
   keywords are contextual (decision 7), and the formatter's three checks
   run on every `.nova` file.
4. **Lighthouse 95 depends on the bundle's size and its time to
   interactive.** The size checks start in 4.0, not at the gate.
5. **Hydration can disagree with the server.**
   - `Map` iterates in an order that depends on its seed, so a view that
     iterates a `Map` could render differently on the two sides.
   - 4.6's spec decides between ordered iteration in views and a seed
     shared with the client.
6. **Phase 4 is the largest phase so far:** the master spec gives it weeks
   57 to 80. The order above puts a working program in a browser at 4.2,
   so later sub-phases can be cut back with the user if needed.
7. **CI time grows** with WASM tests on three operating systems and a
   browser job. Browser tests run on Ubuntu only.

## 7. Suggested first step

The spike for decision 1, in scratch space with nothing pushed:
- build the same small program by routes (a) and (b);
- run it under Node;
- measure each route's gzipped size, amount of code, and needs.

The user picks the route from its results. Then 4.0's spec and ADR 0038
follow.

**Done 2026-10-14:** the user picked route (a); the spike's results are
under decision 1. The next step is 4.0's spec.
