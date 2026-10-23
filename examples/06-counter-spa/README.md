# 06-counter-spa

The smallest WASM frontend: a counter written as a component.

## What this demonstrates

- **`component` and `view { … }`** (RFC 0001): elements with attributes
  by name and children in a block, and `text(…)`.
- **Reactivity:** `text("Count: ${count.value()}")` reads a signal, so it
  is read again, alone, each time a button sets it.
- **`||`**, a closure with no parameters, for each button's handler.
- **The bundler:** the project's own `index.html` is copied into `dist/`,
  beside `app.js`, `app.wasm` and `manifest.json`.

## Run it

```bash
cd examples/06-counter-spa
nova dev
```

`nova dev` builds the page into `dist/`, prints
`nova dev: serving http://127.0.0.1:3000/`, and serves it there; open that
URL in a browser, or start it with `--open`. Each time `src/main.nova`
changes, it builds again and reloads the page; when the build fails, its
errors show over the page until a good build replaces them. `--port`
chooses another port, and Ctrl-C stops it.

To build once instead, `nova build --target wasm` writes the same `dist/`,
which any static server can serve, as long as it serves `app.wasm` as
`application/wasm`.

## Expected output

The page shows `Count: 0` and two buttons. `+` adds one and `-` takes one
away: three clicks of `+` and one of `-` leave `Count: 2`.

## Notes

- It differs from `nova-spec/60-EXAMPLES.md` §6 in three places, each the
  API 4.3 built: `import ui` and `import ui::html::{button, div}`,
  `count.value()`, and `mount("#app", Counter)`.
- `nova build --target wasm --release` runs binaryen's `wasm-opt -Oz
  --enable-bulk-memory` on `app.wasm` when `wasm-opt` is on `PATH`, which
  takes `app.wasm` and `app.js` from 20,997 bytes gzipped (by zlib at
  level 9) to 11,974 with binaryen 132; the gate's size, under 50 KiB, is
  measured without it.
- `cargo test` clicks it under a fake DOM, and CI's `browser` job in
  Chromium (`tests/browser/spa.mjs`); the 4.4 spec is
  `docs/superpowers/specs/2026-10-17-phase-4-4-component-syntax-design.md`.
- The same job runs it under `nova dev` (`tests/browser/dev.mjs`): it
  counts, an edit reloads it, and a type error shows the overlay until the
  fix; the 4.5 spec is
  `docs/superpowers/specs/2026-10-18-phase-4-5-router-and-nova-dev-design.md`.
- The same job runs Lighthouse on its page three times, mobile, and needs
  a median Performance score of 95 or more
  (`tests/browser/lighthouse.mjs`), and drives its `--release` build with
  binaryen's `wasm-opt`; the 4.8 spec is
  `docs/superpowers/specs/2026-10-22-phase-4-8-the-gate-and-v0.4.0-design.md`.
